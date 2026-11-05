//! The window's `compile`: the run the Compile button starts, on a thread of its own and through
//! the command line's own parser, preflight and verdict (`cli`), its events drained into the
//! window's log each frame.

use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, unbounded};
use studio_core::{
    CliError, EventLines, LogLine, PipelineEventEnvelope, RunningPes, Severity, ToolContext,
};

use crate::cli;
use crate::messages::TOOL_ID;

/// A compile in flight: its events, and the thread that ends with its verdict.
struct Run {
    events: Receiver<PipelineEventEnvelope>,
    thread: JoinHandle<Result<u8, CliError>>,
}

/// The window's run log: the lines of the latest `compile`, as the console prints them, ending
/// with the run's exit code.
#[derive(Default)]
pub(crate) struct RunLog {
    lines: Vec<LogLine>,
    formatter: EventLines,
    run: Option<Run>,
}

impl RunLog {
    /// Starts `compile` with no arguments on a thread of its own, through the command line's own
    /// parser, preflight and verdict, so the button cannot compile anything the command line
    /// would refuse. `running` is the tool's running-PES probe, handed to `cli::run`. The
    /// previous run's lines go. Called only while no run is in flight (the button is disabled
    /// while one runs).
    pub(crate) fn start_compile(&mut self, ctx: &ToolContext, running: fn() -> Vec<RunningPes>) {
        self.lines.clear();
        // A fresh formatter: export ids restart with each run.
        self.formatter = EventLines::new();
        let matches = cli::command()
            .try_get_matches_from([TOOL_ID, "compile"])
            .expect("`compile` with no arguments is a valid team-compiler command line");
        let (sender, events) = unbounded();
        let ctx = ctx.with_events(sender);
        let thread = std::thread::spawn(move || cli::run(&matches, &ctx, running));
        self.run = Some(Run { events, thread });
    }

    /// Moves what the run has reported into the lines: every event so far while it runs; once
    /// its thread has ended, the rest and then the exit-code line.
    pub(crate) fn poll(&mut self) {
        let Some(run) = self.run.take() else {
            return;
        };
        if !run.thread.is_finished() {
            self.append(&run.events);
            self.run = Some(run);
            return;
        }
        // Joined before the last drain: the thread sends its last events just before it ends,
        // and a drain that ran first could leave them for a frame after the exit-code line.
        let outcome = run.thread.join();
        self.append(&run.events);
        let (severity, last) = match outcome {
            Ok(Ok(code)) => (
                Severity::Info,
                format!("Run finished with exit code {code}"),
            ),
            Ok(Err(error)) => {
                // As the binary prints a refusal, then the code it exits with.
                self.lines
                    .push(LogLine::plain(Severity::Error, format!("error: {error}")));
                (
                    Severity::Info,
                    format!("Run ended with exit code {}", error.exit_code),
                )
            }
            Err(_) => (
                Severity::Error,
                "Run stopped because the compile crashed".to_owned(),
            ),
        };
        self.lines.push(LogLine::plain(severity, last));
    }

    /// Whether a compile is in flight.
    pub(crate) fn is_running(&self) -> bool {
        self.run.is_some()
    }

    /// The lines shown, oldest first.
    pub(crate) fn lines(&self) -> &[LogLine] {
        &self.lines
    }

    fn append(&mut self, events: &Receiver<PipelineEventEnvelope>) {
        for envelope in events.try_iter() {
            if let Some(line) = LogLine::from_event(&mut self.formatter, &envelope.event) {
                self.lines.push(line);
            }
        }
    }
}

#[cfg(test)]
impl RunLog {
    /// A log whose run is in flight until `until` delivers (exit code 0) or its sender is gone
    /// (exit code 1), emitting nothing: a run that stays running for as long as a test needs.
    pub(crate) fn in_flight(until: Receiver<()>) -> RunLog {
        let (_, events) = unbounded();
        let thread = std::thread::spawn(move || Ok(u8::from(until.recv().is_err())));
        RunLog {
            lines: Vec::new(),
            formatter: EventLines::new(),
            run: Some(Run { events, thread }),
        }
    }

    /// Appends `lines`, as a run's events append them: a view test's overflowed log.
    #[cfg(test)]
    pub(crate) fn append_test_lines(&mut self, lines: &[&str]) {
        self.lines.extend(
            lines
                .iter()
                .map(|line| LogLine::plain(Severity::Info, *line)),
        );
    }

    /// The line texts, for assertions about the text the member reads.
    #[cfg(test)]
    pub(crate) fn texts(&self) -> Vec<&str> {
        self.lines.iter().map(|line| line.text.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use studio_core::StudioTool;

    use std::path::Path;

    use super::*;
    use crate::Tool;
    use crate::testing::{install_pes, poll_until_done, sandbox, tool_context};

    /// The lines of a clean compile of the sandbox's tracer export, installed into its PES
    /// folder (`install_pes`), up to `patch_written`, whose path `tracer_lines` fills in.
    const TRACER_LINES: [&str; 7] = [
        "- Info bin_source (bin=TeamColor.bin, cpk=bundled)",
        "- Info bin_source (bin=UniColor.bin, cpk=bundled)",
        "- Info bin_source (bin=UniformParameter.bin, cpk=bundled)",
        "- egg Midcup Tracer: Info fmdl_weights_not_normalized at Players/05 - The Chad Stormworks Player (file=boots.fmdl, count=1662)",
        "- egg Midcup Tracer: Info fmdl_weights_not_normalized at Players/05 - The Chad Stormworks Player (file=fcl_hair.fmdl, count=1662)",
        "- egg Midcup Tracer: Info fmdl_weights_not_normalized at Players/05 - The Chad Stormworks Player (file=glove_l.fmdl, count=2)",
        "- egg Midcup Tracer: Info export_identified (team=/egg/, id=792)",
    ];

    /// `TRACER_LINES`, then the `patch_written` line naming `root`'s output patch, the
    /// `savefile_missing` warning (`tool_context` names a save under `root` that is not
    /// there), and the run's exit line.
    fn tracer_lines(root: &Path) -> Vec<String> {
        TRACER_LINES
            .iter()
            .map(|line| (*line).to_owned())
            .chain([
                format!(
                    "- Info patch_written (path={}, teams=792)",
                    root.join("output").join("aesthetics_patch.toml").display()
                ),
                format!(
                    "- Warning savefile_missing (patch={}, path={}, command={})",
                    root.join("output").join("aesthetics_patch.toml").display(),
                    root.join("save").join("EDIT00000000").display(),
                    crate::output::savefile::APPLY_COMMAND
                ),
                "Run finished with exit code 0".to_owned(),
            ])
            .collect()
    }

    /// Polls the log until its run has ended.
    fn finish(log: &mut RunLog) {
        poll_until_done(|| {
            log.poll();
            log.is_running()
        });
    }

    // TC-GUI-02
    #[test]
    fn a_compile_logs_the_consoles_lines_then_its_exit_code_and_a_rerun_replaces_them() {
        let temp = sandbox("gui_run_compile");
        let root = temp.path();
        install_pes(root);
        let ctx = tool_context(root, "");
        let mut log = RunLog::default();
        assert!(!log.is_running());

        log.start_compile(&ctx, Vec::new);
        assert!(log.is_running());
        finish(&mut log);
        assert_eq!(log.texts(), tracer_lines(root));
        assert!(root.join("PES/download/4cc_99_test.cpk").is_file());

        log.start_compile(&ctx, Vec::new);
        finish(&mut log);
        assert_eq!(log.texts(), tracer_lines(root));
    }

    #[test]
    fn polling_a_run_in_flight_returns_at_once_and_keeps_it_running() {
        let (release, until) = unbounded();
        let mut log = RunLog::in_flight(until);
        // `poll` runs on the GUI thread every frame: one that waited for the run to end would
        // freeze the window for the whole compile (here, forever: the run ends only on release).
        log.poll();
        assert!(log.is_running());
        release.send(()).unwrap();
        poll_until_done(|| {
            log.poll();
            log.is_running()
        });
        assert_eq!(log.texts(), ["Run finished with exit code 0"]);
    }

    #[test]
    fn the_tool_ticks_its_run_to_the_end() {
        let temp = sandbox("gui_run_tool_tick");
        let root = temp.path();
        install_pes(root);
        let ctx = tool_context(root, "");
        let mut tool = Tool::new();
        tool.run_log.start_compile(&ctx, Vec::new);
        poll_until_done(|| {
            tool.tick(&ctx);
            tool.run_log.is_running()
        });
        assert_eq!(tool.run_log.texts(), tracer_lines(root));
    }

    #[test]
    fn a_refused_compile_logs_the_error_line_then_its_exit_code() {
        let temp = sandbox("gui_run_refused");
        let root = temp.path();
        let ctx = tool_context(root, "[team-compiler]\ncpk_name = 'a/b'\n");
        let mut log = RunLog::default();
        log.start_compile(&ctx, Vec::new);
        finish(&mut log);
        let lines = log.lines();
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[0].text.starts_with("error: "), "{lines:?}");
        assert_eq!(
            lines[0].severity,
            Severity::Error,
            "the refusal is an error"
        );
        assert_eq!(lines[1].text, "Run ended with exit code 2");
        assert_eq!(lines[1].severity, Severity::Info, "the exit line is info");
    }
}
