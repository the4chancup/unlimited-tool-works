//! The window's log: the grid's check's findings, then the `compile` the Compile button starts,
//! on a thread of its own and through the command line's own parser, preflight and verdict
//! (`cli`), its events drained into the log each frame. The Upgrade DpFileList button's
//! upgrade runs the same way, its lines appended after the run's it answers.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, unbounded};
use export_grid::{CellKey, Row, cell_places};
use studio_core::{
    CliError, EventLines, LogLine, Message, PipelineEvent, PipelineEventEnvelope, RunningPes,
    Severity, ToolContext, message_line,
};

use crate::messages::{Code, TOOL_ID};
use crate::{cli, upgrade};

/// A compile or a DpFileList upgrade in flight: its events, and the thread that ends with its
/// verdict.
struct Run {
    events: Receiver<PipelineEventEnvelope>,
    thread: JoinHandle<Result<u8, CliError>>,
}

/// What the last check, or a run's messages since, found of the game's `DpFileList.bin`.
#[derive(Default, PartialEq)]
enum Dpfl {
    /// The official one (or nothing found yet): no Upgrade DpFileList button.
    #[default]
    Official,
    /// The cup's outdated one (`dpfilelist_outdated`): a compile's preflight would stop it,
    /// so Compile is disabled.
    Outdated,
    /// Neither (`dpfilelist_not_official` or `dpfilelist_cpk_missing`, both Warnings): the
    /// button shows, Compile stays enabled.
    Other,
}

/// The window's log: the grid's last check's findings, then the lines of the latest run, as
/// the console prints them, ending with the run's exit code.
#[derive(Default)]
pub(crate) struct RunLog {
    /// The check's lines, then the run's.
    lines: Vec<LogLine>,
    /// How many of `lines` are the check's: the first `cells.len()`, one per cell line, then
    /// one per run-scoped finding ([`crate::check::live::LiveCheck::list_findings`]).
    check_len: usize,
    /// The cell each of the check's lines acts for, one per cell line: the cell lines are the
    /// first `cells.len()` of `lines`, so a run-scoped line's `cell_of` is `None`.
    cells: Vec<CellKey>,
    formatter: EventLines,
    run: Option<Run>,
    /// The last check's verdict on the game's `DpFileList.bin`, or a run's latest word on it
    /// since: the toolbar's Upgrade DpFileList button shows unless `Official`, and Compile is
    /// disabled while `Outdated`.
    dpfl: Dpfl,
}

/// The findings of a check's `rows` as the log prints them, each with the grid cell its line
/// acts for: row by row, each row's cells in `cell_places` order, Infos included. A finding two
/// cells hold (a referee folder backing two slots) is one line, acting for the first.
pub(crate) fn check_lines(rows: &[Row]) -> (Vec<LogLine>, Vec<CellKey>) {
    let mut lines = Vec::new();
    let mut cells = Vec::new();
    for row in rows {
        let mut earlier_findings: Vec<&Message> = Vec::new();
        for (place, cell) in cell_places(row) {
            for message in &cell.messages {
                if earlier_findings.contains(&message) {
                    continue;
                }
                lines.push(LogLine {
                    severity: message.severity,
                    code: Some(message.code.clone()),
                    text: message_line(Some(&row.key), message),
                });
                cells.push(CellKey {
                    row: row.key.clone(),
                    place,
                });
            }
            earlier_findings.extend(&cell.messages);
        }
    }
    (lines, cells)
}

impl RunLog {
    /// Starts `compile --mode <mode>` on a thread of its own, through the command line's own
    /// parser, preflight and verdict, so the button cannot compile anything the command line
    /// would refuse: a mode the command refuses (sideload where no runtime serves it) is
    /// refused the same way, its `error:` line in the log. `running` is the tool's
    /// running-PES probe, handed to `cli::run`. The previous run's lines go; the check's
    /// stay. Called only while no run is in flight (the button is disabled while one runs).
    pub(crate) fn start_compile(
        &mut self,
        ctx: &ToolContext,
        mode: cli::Mode,
        running: fn() -> Vec<RunningPes>,
    ) {
        self.lines.truncate(self.check_len);
        // The check's verdict on the game's list stays: the run's preflight reports it again
        // when the list still is not the official one, and the check after the run sets it.
        // A fresh formatter: export ids restart with each run.
        self.formatter = EventLines::new();
        let mode = cli::mode_name(mode);
        let matches = cli::command()
            .try_get_matches_from([TOOL_ID, "compile", "--mode", mode.as_str()])
            .expect("every `Mode` is a `--mode` value, a valid compiler command line");
        let (sender, events) = unbounded();
        let ctx = ctx.with_events(sender);
        let thread = std::thread::spawn(move || cli::run(&matches, &ctx, running));
        self.run = Some(Run { events, thread });
    }

    /// Starts the DpFileList upgrade on a thread of its own: `upgrade::run` applied in
    /// `download`, the folder its preview was made for (kept, never resolved again from
    /// the settings, which may have changed while the dialog was open), with `delete`,
    /// the dropped CPKs the dialog ticked for deletion, which the command line's `--yes`
    /// never deletes. The upgrade's lines are appended after the run's it answers: what
    /// the preflight said stays in the log. Called only while no run is in flight (the
    /// dialog opens only then).
    pub(crate) fn start_upgrade(
        &mut self,
        download: PathBuf,
        delete: BTreeSet<String>,
        ctx: &ToolContext,
    ) {
        // A fresh formatter: export ids restart with each run.
        self.formatter = EventLines::new();
        let (sender, events) = unbounded();
        let ctx = ctx.with_events(sender);
        let thread =
            std::thread::spawn(move || cli::verdict(upgrade::run(&download, true, &delete, &ctx)));
        self.run = Some(Run { events, thread });
    }

    /// Whether the game's `DpFileList.bin` is not the official one as the last check, or a
    /// run's messages since, found it (`dpfilelist_outdated`, `dpfilelist_not_official` or
    /// `dpfilelist_cpk_missing`): the Upgrade DpFileList button's condition beside "no run in
    /// flight".
    pub(crate) fn dpfl_found(&self) -> bool {
        self.dpfl != Dpfl::Official
    }

    /// Whether the game's `DpFileList.bin` is the cup's outdated one (`dpfilelist_outdated`),
    /// whose Fatal finding a compile's preflight would make: the Compile button's disabled
    /// condition beside "a run in flight".
    pub(crate) fn dpfl_outdated(&self) -> bool {
        self.dpfl == Dpfl::Outdated
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
                self.append_refusal(&error.to_string());
                (
                    Severity::Info,
                    format!("Run ended with exit code {}", error.exit_code),
                )
            }
            Err(_) => (Severity::Error, "Run stopped because it crashed".to_owned()),
        };
        self.lines.push(LogLine::plain(severity, last));
    }

    /// Adds a refusal's line, `error: ` and then `error`, as the binary prints one: the
    /// Compile run's, or the grid's check's.
    pub(crate) fn append_refusal(&mut self, error: &str) {
        self.lines
            .push(LogLine::plain(Severity::Error, format!("error: {error}")));
    }

    /// Replaces the check's lines with `rows`' and its `list` of run-scoped findings, a
    /// new check's, ahead of the run's: the rows' lines each act for their
    /// cell, then one line per list finding, as the console prints a run-scoped finding,
    /// acting for none. `dpfl` is set from `list`.
    pub(crate) fn set_check(&mut self, rows: &[Row], list: &[Message]) {
        let (mut lines, cells) = check_lines(rows);
        lines.extend(list.iter().map(|message| LogLine {
            severity: message.severity,
            code: Some(message.code.clone()),
            text: message_line(None, message),
        }));
        let check_len = lines.len();
        self.lines.splice(..self.check_len, lines);
        self.check_len = check_len;
        self.cells = cells;
        self.dpfl = if list
            .iter()
            .any(|message| dpfl_flag(message.code.code.as_ref()) == Some(Dpfl::Outdated))
        {
            Dpfl::Outdated
        } else {
            list.iter()
                .find_map(|message| dpfl_flag(message.code.code.as_ref()))
                .unwrap_or(Dpfl::Official)
        };
    }

    /// Whether a run is in flight.
    pub(crate) fn is_running(&self) -> bool {
        self.run.is_some()
    }

    /// Every line's text, the check's then the run's, joined with `\n`: the whole log, so a
    /// report holds the findings too; the log panel's filter hides lines for drawing only.
    pub(crate) fn report(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The lines shown, oldest first.
    pub(crate) fn lines(&self) -> &[LogLine] {
        &self.lines
    }

    /// The grid cell the line at `line` in `lines` acts for: a check's cell line has one,
    /// a check's run-scoped list line and a run's line none.
    pub(crate) fn cell_of(&self, line: usize) -> Option<&CellKey> {
        self.cells.get(line)
    }

    fn append(&mut self, events: &Receiver<PipelineEventEnvelope>) {
        for envelope in events.try_iter() {
            if let PipelineEvent::Message(message) = &envelope.event
                && let Some(dpfl) = dpfl_flag(message.code.code.as_ref())
            {
                self.dpfl = dpfl;
            }
            if let Some(line) = LogLine::from_event(&mut self.formatter, &envelope.event) {
                self.lines.push(line);
            }
        }
    }
}

/// The `Dpfl` a `DpFileList.bin` finding of `code` sets — a run's message in `append`, a
/// check's list finding in `set_check`: `Outdated` for `dpfilelist_outdated`, `Other`
/// for the two Warnings the upgrade remedies, `Official` for the upgrade's own report that
/// the list is now the official one (a replacement, or nothing to do): an upgrade stopped
/// before the list is replaced emits neither, so its button stays for the retry. `None` for
/// every other code.
fn dpfl_flag(code: &str) -> Option<Dpfl> {
    if code == Code::DpfilelistOutdated.as_str() {
        return Some(Dpfl::Outdated);
    }
    if [
        Code::DpfilelistNotOfficial.as_str(),
        Code::DpfilelistCpkMissing.as_str(),
    ]
    .contains(&code)
    {
        return Some(Dpfl::Other);
    }
    [
        Code::DpfilelistReplaced.as_str(),
        Code::DpfilelistUpToDate.as_str(),
    ]
    .contains(&code)
    .then_some(Dpfl::Official)
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
            check_len: 0,
            cells: Vec::new(),
            formatter: EventLines::new(),
            run: Some(Run { events, thread }),
            dpfl: Dpfl::Official,
        }
    }

    /// `in_flight` whose last check found the cup's outdated list: the `dpfl` a drained
    /// `dpfilelist_outdated` message sets, which shows the Upgrade DpFileList button and
    /// disables Compile once the run ends.
    #[cfg(test)]
    pub(crate) fn in_flight_dpfl(until: Receiver<()>) -> RunLog {
        let mut log = RunLog::in_flight(until);
        log.dpfl = Dpfl::Outdated;
        log
    }

    /// A log whose `dpfl` is `Other`, as a drained `dpfilelist_not_official` or
    /// `dpfilelist_cpk_missing` sets it, with no run: the Upgrade DpFileList button shows
    /// and Compile is enabled.
    #[cfg(test)]
    pub(crate) fn dpfl_other() -> RunLog {
        RunLog {
            dpfl: Dpfl::Other,
            ..RunLog::default()
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

    /// Appends one line of `severity`: a view test's log with warnings and errors in it.
    #[cfg(test)]
    pub(crate) fn append_test_line(&mut self, severity: Severity, text: &str) {
        self.lines.push(LogLine::plain(severity, text));
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

    use std::borrow::Cow;
    use std::path::Path;

    use export_grid::{Cell, CellState, Line, Place, Position, RowBody};
    use studio_core::{Disposition, ExportId, MessageCode, Scope};
    use vtree::ScopePath;

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

        log.start_compile(&ctx, cli::Mode::Normal, Vec::new);
        assert!(log.is_running());
        finish(&mut log);
        assert_eq!(log.texts(), tracer_lines(root));
        assert!(root.join("PES/download/4cc_99_test.cpk").is_file());

        log.start_compile(&ctx, cli::Mode::Normal, Vec::new);
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
        tool.run_log
            .start_compile(&ctx, cli::Mode::Normal, Vec::new);
        poll_until_done(|| {
            tool.tick(&ctx);
            tool.run_log.is_running()
        });
        assert_eq!(tool.run_log.texts(), tracer_lines(root));
    }

    /// A finding of the Compiler's `code` at `severity` about `scope`, with `context`.
    fn finding(
        code: &'static str,
        severity: Severity,
        scope: Scope,
        context: &[(&str, &str)],
    ) -> Message {
        Message {
            code: MessageCode {
                tool_id: TOOL_ID,
                code: Cow::Borrowed(code),
            },
            severity,
            disposition: Disposition::Keep,
            scope,
            context: context
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect(),
        }
    }

    /// Feeds `log` `message` as one of a run's events, the way `poll` drains them.
    fn drain(log: &mut RunLog, message: Message) {
        let (sender, events) = unbounded();
        sender
            .send(PipelineEventEnvelope {
                run_id: studio_core::RunId(0),
                export_id: None,
                export_revision: None,
                made_at: None,
                event: PipelineEvent::Message(message),
            })
            .unwrap();
        log.append(&events);
    }

    /// The folder `path` of export 1.
    fn folder(path: &str) -> Scope {
        Scope::Folder {
            export_id: ExportId(1),
            path: ScopePath::new(path).unwrap(),
        }
    }

    /// A checked cell showing `text`, holding `messages`.
    fn cell(text: &str, messages: Vec<Message>) -> Position {
        Position::Cell(Box::new(Cell {
            messages,
            ..Cell::new(text, CellState::FullOk)
        }))
    }

    /// A row keyed `key` and labelled `label`, its label holding `label_messages`, its lines
    /// `lines` of player positions alone.
    fn row(key: &str, label: &str, label_messages: Vec<Message>, lines: Vec<Vec<Position>>) -> Row {
        Row {
            key: key.to_owned(),
            label: Cell {
                messages: label_messages,
                ..Cell::new(label, CellState::FullOk)
            },
            id: None,
            body: RowBody::Lines(
                lines
                    .into_iter()
                    .map(|players| Line {
                        name: None,
                        groups: vec![players],
                    })
                    .collect(),
            ),
            log_lines: Vec::new(),
        }
    }

    #[test]
    fn the_check_s_lines_are_each_row_s_findings_in_cell_order_one_per_finding() {
        let orphaned = finding(
            "shared_folder_orphaned",
            Severity::Warning,
            folder("Boots/Solo"),
            &[],
        );
        let identified = finding(
            "export_identified",
            Severity::Info,
            Scope::Export {
                export_id: ExportId(1),
            },
            &[("team", "/co/"), ("id", "714")],
        );
        let fpc = finding(
            "fpc_conflict",
            Severity::Error,
            folder("Players/01 - A"),
            &[],
        );
        let co = row(
            "co Midcup Map",
            "/co/",
            vec![orphaned, identified],
            vec![vec![cell("01", vec![fpc])]],
        );
        // A referee folder backing slots 01 and 35: its finding is on both cells.
        let disallowed = finding(
            "file_type_disallowed",
            Severity::Error,
            folder("Players/A"),
            &[("file", "notes.exe")],
        );
        let mut second_line = vec![Position::Empty; 11];
        second_line.push(cell("35", vec![disallowed.clone()]));
        let refs = row(
            "refs Cup",
            "/refs/",
            Vec::new(),
            vec![vec![cell("01", vec![disallowed])], second_line],
        );

        let (lines, cells) = check_lines(&[co, refs]);
        let texts: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "- co Midcup Map: Warning shared_folder_orphaned at Boots/Solo",
                "- co Midcup Map: Info export_identified (team=/co/, id=714)",
                "- co Midcup Map: Error fpc_conflict at Players/01 - A",
                "- refs Cup: Error file_type_disallowed at Players/A (file=notes.exe)",
            ]
        );
        let severities: Vec<Severity> = lines.iter().map(|line| line.severity).collect();
        assert_eq!(
            severities,
            [
                Severity::Warning,
                Severity::Info,
                Severity::Error,
                Severity::Error
            ]
        );
        assert_eq!(
            lines[0].code.as_ref().map(|code| code.code.as_ref()),
            Some("shared_folder_orphaned"),
            "the line keeps its help link"
        );
        let first_player = Place::Position {
            line: 0,
            group: 0,
            index: 0,
        };
        let key = |row: &str, place| CellKey {
            row: row.to_owned(),
            place,
        };
        assert_eq!(
            cells,
            [
                key("co Midcup Map", Place::Label),
                key("co Midcup Map", Place::Label),
                key("co Midcup Map", first_player),
                key("refs Cup", first_player),
            ]
        );
    }

    #[test]
    fn a_new_check_replaces_the_check_s_lines_and_keeps_the_run_s() {
        let identified = |key: &str, team: &str, id: &str| {
            let message = finding(
                "export_identified",
                Severity::Info,
                Scope::Export {
                    export_id: ExportId(1),
                },
                &[("team", team), ("id", id)],
            );
            row(key, team, vec![message], Vec::new())
        };
        let mut log = RunLog::default();
        log.set_check(&[identified("co Midcup Map", "/co/", "714")], &[]);
        log.append_test_line(Severity::Info, "Run finished with exit code 0");
        log.set_check(
            &[
                identified("a Midcup Grid", "/a/", "702"),
                identified("co Midcup Map", "/co/", "714"),
            ],
            &[],
        );
        assert_eq!(
            log.texts(),
            [
                "- a Midcup Grid: Info export_identified (team=/a/, id=702)",
                "- co Midcup Map: Info export_identified (team=/co/, id=714)",
                "Run finished with exit code 0",
            ]
        );
        assert_eq!(
            log.cell_of(1).map(|cell| cell.row.as_str()),
            Some("co Midcup Map")
        );
        assert_eq!(log.cell_of(2), None, "a run's line acts for no cell");
    }

    #[test]
    fn a_check_s_list_findings_are_lines_after_the_cell_lines_acting_for_none() {
        let identified = finding(
            "export_identified",
            Severity::Info,
            Scope::Export {
                export_id: ExportId(1),
            },
            &[("team", "/co/"), ("id", "714")],
        );
        let outdated = finding(
            "dpfilelist_outdated",
            Severity::Fatal,
            Scope::Run,
            &[
                ("cpk", "4cc_99_test.cpk"),
                ("command", crate::output::deploy::UPGRADE_COMMAND),
            ],
        );
        let mut log = RunLog::default();
        log.set_check(
            &[row("co Midcup Map", "/co/", vec![identified], Vec::new())],
            &[outdated],
        );
        assert_eq!(
            log.texts(),
            [
                "- co Midcup Map: Info export_identified (team=/co/, id=714)",
                &format!(
                    "- Fatal dpfilelist_outdated (cpk=4cc_99_test.cpk, command={})",
                    crate::output::deploy::UPGRADE_COMMAND
                ),
            ]
        );
        assert_eq!(log.cell_of(1), None, "a list line acts for no cell");
        assert!(log.dpfl_outdated(), "the outdated list disables Compile");
    }

    #[test]
    fn a_run_s_dpfilelist_messages_set_the_button_s_state() {
        for (code, start, found, outdated) in [
            ("dpfilelist_outdated", Dpfl::Other, true, true),
            ("dpfilelist_not_official", Dpfl::Outdated, true, false),
            ("dpfilelist_cpk_missing", Dpfl::Outdated, true, false),
            ("dpfilelist_replaced", Dpfl::Outdated, false, false),
            ("dpfilelist_up_to_date", Dpfl::Other, false, false),
            // Not about the game's list: each leaves the state as it was.
            ("cpk_name_unlisted", Dpfl::Outdated, true, true),
            ("dpfilelist_missing", Dpfl::Other, true, false),
        ] {
            let mut log = RunLog {
                dpfl: start,
                ..RunLog::default()
            };
            drain(&mut log, finding(code, Severity::Warning, Scope::Run, &[]));
            assert_eq!(log.dpfl_found(), found, "{code}");
            assert_eq!(log.dpfl_outdated(), outdated, "{code}");
        }
    }

    #[test]
    fn a_check_that_finds_nothing_clears_the_button_s_state() {
        let outdated = finding(
            "dpfilelist_outdated",
            Severity::Fatal,
            Scope::Run,
            &[("cpk", "4cc_99_test.cpk")],
        );
        let mut log = RunLog::default();
        log.set_check(
            &[row("co Midcup Map", "/co/", Vec::new(), Vec::new())],
            &[outdated],
        );
        assert!(log.dpfl_found() && log.dpfl_outdated(), "premise: set");

        log.set_check(&[row("co Midcup Map", "/co/", Vec::new(), Vec::new())], &[]);
        assert!(
            !log.dpfl_found() && !log.dpfl_outdated(),
            "a check with no list finding clears the state"
        );
    }

    #[test]
    fn a_check_s_list_line_is_replaced_by_the_next_check_s_and_the_run_s_kept() {
        let identified = |key: &str, team: &str, id: &str| {
            let message = finding(
                "export_identified",
                Severity::Info,
                Scope::Export {
                    export_id: ExportId(1),
                },
                &[("team", team), ("id", id)],
            );
            row(key, team, vec![message], Vec::new())
        };
        let cpk_missing = finding(
            "dpfilelist_cpk_missing",
            Severity::Warning,
            Scope::Run,
            &[("files", "4cc_16_balls.cpk")],
        );
        let outdated = finding(
            "dpfilelist_outdated",
            Severity::Fatal,
            Scope::Run,
            &[("cpk", "4cc_99_test.cpk")],
        );
        let mut log = RunLog::default();
        log.set_check(
            &[identified("co Midcup Map", "/co/", "714")],
            &[cpk_missing],
        );
        assert!(
            !log.dpfl_outdated(),
            "a Warning alone leaves Compile enabled"
        );
        assert!(log.dpfl_found());
        log.append_test_line(Severity::Info, "Run finished with exit code 0");
        log.set_check(&[identified("a Midcup Grid", "/a/", "702")], &[outdated]);
        assert_eq!(
            log.texts(),
            [
                "- a Midcup Grid: Info export_identified (team=/a/, id=702)",
                "- Fatal dpfilelist_outdated (cpk=4cc_99_test.cpk)",
                "Run finished with exit code 0",
            ]
        );
        assert!(log.dpfl_outdated());
    }

    #[test]
    fn a_refused_compile_logs_the_error_line_then_its_exit_code() {
        let temp = sandbox("gui_run_refused");
        let root = temp.path();
        let ctx = tool_context(root, "[compiler]\ncpk_name = 'a/b'\n");
        let mut log = RunLog::default();
        log.start_compile(&ctx, cli::Mode::Normal, Vec::new);
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
