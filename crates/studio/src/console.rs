//! The CLI's event and notice printers: each drains its channel on its own thread while a
//! command runs. Events' lines go to stdout, as `studio_core::EventLines` formats them.
//! Notices' texts go to stderr: the status bar's right slot, rendered as lines. On a terminal
//! stderr the event printer also draws the run's progress line and leaves its summary;
//! piped, it prints stdout alone.

use std::io::{self, IsTerminal, Write};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::Receiver;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressState, ProgressStyle};
use studio_core::{EventLines, PipelineEvent, PipelineEventEnvelope, ShellRequest};

/// Drains the channel on its own thread, printing each line to stdout, until every sender is
/// gone. Whether stderr is a terminal decides whether the progress line is drawn; it is read
/// here alone, so `print_events` takes the decision and its writers as arguments.
pub(crate) fn spawn_printer(events: Receiver<PipelineEventEnvelope>) -> JoinHandle<io::Result<()>> {
    let draw = io::stderr().is_terminal();
    std::thread::spawn(move || {
        // The unlocked handles lock per write, as the notice printer's do.
        print_events(events, draw, &mut io::stdout(), &mut io::stderr())
    })
}

/// The run's events as lines on `out`, as `EventLines` formats them. With `draw`, the run's
/// progress line (`[ 3/12 exports | 1,284/4,010 parts | 0:42 ]`) is drawn by indicatif on the
/// process's stderr, redrawn in place at its rate and cleared while each line goes to `out`
/// (`suspend`); on `Complete` it is replaced on `err` by the run's summary, and a stream that
/// ends without one (a `check`, an upgrade, a run stopped before it compiled) leaves no line.
/// Without `draw` the bar is hidden and nothing touches `err`, so piped stdout is
/// byte-for-byte what it is on a terminal.
fn print_events(
    events: Receiver<PipelineEventEnvelope>,
    draw: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> io::Result<()> {
    let mut lines = EventLines::new();
    let mut counts = RunCounts::new();
    let mut ended = false;
    // indicatif draws to stderr itself (the Windows console handled), capped at ten draws a
    // second once its target is set; the line is the counters message and the `{clock}` key,
    // no bar graphic.
    let bar = if draw {
        // Hidden until the first export: a bar redraws after every `suspend`, so it would
        // draw an empty frame while the findings before it print.
        let bar = ProgressBar::with_draw_target(None, ProgressDrawTarget::hidden());
        bar.set_style(
            ProgressStyle::with_template("[ {msg} | {clock} ]")
                .expect("a literal template parses")
                .with_key(
                    "clock",
                    |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                        // Rendered from the bar's state on every draw, so the clock moves
                        // between events too.
                        write!(w, "{}", elapsed_text(state.elapsed()))
                            .expect("writing into the draw buffer does not fail");
                    },
                ),
        );
        bar
    } else {
        ProgressBar::hidden()
    };
    for envelope in events {
        counts.observe(&envelope.event);
        // The line draws from the first export started, not the first finding; its message
        // is set before the bar has a target, so its first frame already holds the counters.
        if let Some(line) = counts.line() {
            bar.set_message(line);
        }
        let summary = match &envelope.event {
            PipelineEvent::ExportStarted { .. } => {
                if draw && counts.exports_total == 1 {
                    bar.set_draw_target(ProgressDrawTarget::stderr_with_hz(10));
                    // The steady tick makes the clock tick between findings, from its own
                    // thread: enabled once the message is up, so no empty brackets draw.
                    bar.enable_steady_tick(Duration::from_millis(500));
                }
                None
            }
            PipelineEvent::Complete {
                files_written,
                skipped_exports,
                skipped_folders,
                folders_with_errors,
                ..
            } => Some((
                *files_written,
                *skipped_exports,
                *skipped_folders,
                *folders_with_errors,
            )),
            PipelineEvent::ExportProcessed { .. }
            | PipelineEvent::ExportNamed { .. }
            | PipelineEvent::FolderStatus { .. }
            | PipelineEvent::Message(_)
            | PipelineEvent::Output(_)
            | PipelineEvent::Progress { .. }
            | PipelineEvent::UnmigratedContentFound => None,
        };
        match summary {
            Some((files, skipped_exports, skipped_folders, folders_with_errors)) => {
                ended = true;
                bar.finish_and_clear();
                if draw {
                    writeln!(
                        err,
                        "{}",
                        summary_line(
                            files,
                            skipped_exports,
                            skipped_folders,
                            folders_with_errors,
                            bar.elapsed()
                        )
                    )?;
                }
            }
            None => {
                if let Some(line) = lines.line(&envelope.event) {
                    let prefix = envelope.made_at.map(timestamp_prefix).unwrap_or_default();
                    bar.suspend(|| writeln!(out, "{prefix}{line}"))?;
                }
            }
        }
    }
    if !ended {
        bar.finish_and_clear();
    }
    Ok(())
}

/// The run's counters the progress line draws: the exports started (the total) and
/// processed, and the parts the `Progress` events reported. Counting lives here, away from
/// the bar's real stderr, so a test can read the line's text.
struct RunCounts {
    exports_done: usize,
    exports_total: usize,
    /// The last `Progress` pair (`None` until one arrives): the manifest's tasks done of all.
    parts: Option<(usize, usize)>,
}

impl RunCounts {
    fn new() -> RunCounts {
        RunCounts {
            exports_done: 0,
            exports_total: 0,
            parts: None,
        }
    }

    /// One event's effect on the counters.
    fn observe(&mut self, event: &PipelineEvent) {
        match event {
            PipelineEvent::ExportStarted { .. } => self.exports_total += 1,
            PipelineEvent::ExportProcessed { .. } => self.exports_done += 1,
            PipelineEvent::Progress { processed, total } => {
                self.parts = Some((*processed, *total));
            }
            PipelineEvent::ExportNamed { .. }
            | PipelineEvent::FolderStatus { .. }
            | PipelineEvent::Message(_)
            | PipelineEvent::Output(_)
            | PipelineEvent::Complete { .. }
            | PipelineEvent::UnmigratedContentFound => {}
        }
    }

    /// The progress line's counters text, `None` before the first `ExportStarted`.
    fn line(&self) -> Option<String> {
        (self.exports_total > 0)
            .then(|| progress_line(self.exports_done, self.exports_total, self.parts))
    }
}

/// The progress line's counters, which `set_message` puts inside the template's `[ ... ]`:
/// the exports done of all started, then the parts done once a `Progress` arrived.
fn progress_line(
    exports_done: usize,
    exports_total: usize,
    parts: Option<(usize, usize)>,
) -> String {
    let parts = match parts {
        Some((done, total)) => format!(" | {}/{} parts", grouped(done), grouped(total)),
        None => String::new(),
    };
    format!(
        "{}/{} exports{parts}",
        grouped(exports_done),
        grouped(exports_total)
    )
}

/// The summary line a `Complete` leaves: the files written, the exports skipped, the
/// folders skipped and the folders with errors, each a count of distinct exports or folders
/// with its plural right, and the run's elapsed `m:ss`.
fn summary_line(
    files: usize,
    skipped_exports: usize,
    skipped_folders: usize,
    folders_with_errors: usize,
    elapsed: Duration,
) -> String {
    format!(
        "Done: {} file{} written, {} export{} skipped, {} folder{} skipped, {} folder{} with errors, {}",
        grouped(files),
        plural_s(files),
        grouped(skipped_exports),
        plural_s(skipped_exports),
        grouped(skipped_folders),
        plural_s(skipped_folders),
        grouped(folders_with_errors),
        plural_s(folders_with_errors),
        elapsed_text(elapsed)
    )
}

/// `n`'s digits grouped by threes with commas (`1,284`).
fn grouped(n: usize) -> String {
    let digits = n.to_string();
    let mut text = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            text.push(',');
        }
        text.push(digit);
    }
    text
}

/// `elapsed` as `m:ss` (`0:42`, `62:07`).
fn elapsed_text(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// The `+s.mmm ` a timed finding's line starts with: `made_at`'s whole seconds, unpadded,
/// then its milliseconds truncated to three digits (`+0.042 `, `+125.500 `).
fn timestamp_prefix(made_at: Duration) -> String {
    format!("+{}.{:03} ", made_at.as_secs(), made_at.subsec_millis())
}

/// The plural `s`, absent for a count of one.
fn plural_s(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// Drains the shell-request channel, printing each notice's text to `out` on its own line,
/// until every sender is gone. Other requests are the shell's alone and are skipped; the
/// action a notice may carry cannot be clicked in a terminal.
fn print_notices(requests: Receiver<ShellRequest>, out: &mut impl Write) -> io::Result<()> {
    for request in requests {
        if let ShellRequest::Notify(notice) = request {
            writeln!(out, "{}", notice.text)?;
        }
    }
    Ok(())
}

/// Runs `print_notices` on stderr on its own thread; the join carries the writes' outcome.
pub(crate) fn spawn_notice_printer(requests: Receiver<ShellRequest>) -> JoinHandle<io::Result<()>> {
    std::thread::spawn(move || {
        // The unlocked handle locks per write: holding `stderr().lock()` for the command's
        // whole run would block every other thread's stderr write, the logger's included, and
        // hang the command at its first warning.
        print_notices(requests, &mut std::io::stderr())
    })
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::fs;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use crossbeam_channel::unbounded;
    use pes_version::PesVersion;
    use studio_core::{
        AppPaths, Disposition, ExportId, Message, MessageCode, Notice, NoticeKind, RunId, Scope,
        Settings, Severity, StudioTool, ToolContext,
    };

    use super::*;
    use crate::test_dir::TestDir;

    /// The `export_id`-less envelope every run's events share.
    fn envelope(event: PipelineEvent) -> PipelineEventEnvelope {
        PipelineEventEnvelope {
            run_id: RunId(1),
            export_id: None,
            export_revision: None,
            made_at: None,
            event,
        }
    }

    /// One run's stream: two exports started, a finding, three parts, both exports done,
    /// the run's end.
    fn run_events() -> Receiver<PipelineEventEnvelope> {
        let (tx, rx) = unbounded();
        let finding = PipelineEvent::Message(Message {
            code: MessageCode {
                tool_id: "team-compiler",
                code: Cow::Borrowed("bad_file"),
            },
            severity: Severity::Warning,
            disposition: Disposition::Keep,
            scope: Scope::Export {
                export_id: ExportId(1),
            },
            context: Vec::new(),
        });
        for event in [
            PipelineEvent::ExportStarted {
                export_id: ExportId(1),
                display_name: "co One".to_owned(),
            },
            PipelineEvent::ExportStarted {
                export_id: ExportId(2),
                display_name: "co Two".to_owned(),
            },
            finding,
            PipelineEvent::Progress {
                processed: 1,
                total: 3,
            },
            PipelineEvent::Progress {
                processed: 2,
                total: 3,
            },
            PipelineEvent::Progress {
                processed: 3,
                total: 3,
            },
            PipelineEvent::ExportProcessed {
                export_id: ExportId(1),
            },
            PipelineEvent::ExportProcessed {
                export_id: ExportId(2),
            },
            PipelineEvent::Complete {
                success: false,
                files_written: 12,
                skipped_exports: 1,
                skipped_folders: 2,
                duplicates: 0,
                folders_with_errors: 3,
            },
        ] {
            tx.send(envelope(event)).unwrap();
        }
        drop(tx);
        rx
    }

    /// The files below `folder`, counted; a folder itself is no file.
    fn file_count(folder: &Path) -> usize {
        fs::read_dir(folder)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                if path.is_dir() { file_count(&path) } else { 1 }
            })
            .sum()
    }

    /// A counters line's parts pair (`1/9 parts`), `None` when it has none.
    fn parts_of(line: &str) -> Option<(usize, usize)> {
        let pair = line.strip_suffix(" parts")?.rsplit(" | ").next()?;
        let (done, total) = pair.split_once('/')?;
        let count = |text: &str| text.replace(',', "").parse().ok();
        Some((count(done)?, count(total)?))
    }

    // EU-CLI-12
    #[test]
    fn an_upgrades_progress_line_counts_its_one_export_and_the_files_it_writes() {
        // EU-CLI-01's /co/ export and its save, run in-process through the
        // tool's command line as the binary's sandbox runs it.
        let dir = TestDir::new("console_upgrade");
        let export = dir.path().join("co - Spring");
        fs::create_dir_all(export.join("Other")).unwrap();
        fs::write(export.join("Other/readme.txt"), b"x").unwrap();
        fs::write(
            export.join("co Note.txt"),
            b"Team: /co/\r\n\r\nTeam Colours:\r\n- 1st: 048 146 082\r\n",
        )
        .unwrap();
        let face = export.join("Faces/XXX05 - A");
        fs::create_dir_all(&face).unwrap();
        for (name, bytes) in [
            ("fcl_hair.fmdl", b"hair" as &[u8]),
            ("face_diff.bin", b"diff"),
            ("shirt.dds", b"shirt"),
            ("portrait.dds", b"portrait"),
        ] {
            fs::write(face.join(name), bytes).unwrap();
        }
        let save = dir.path().join("EDIT");
        fs::write(&save, pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();

        let tool = export_upgrader::Tool::new();
        let mut settings = Settings::parse("").unwrap();
        settings.merge_defaults(tool.id(), &tool.default_settings());
        let (events_tx, events_rx) = unbounded();
        let (requests_tx, _requests_rx) = unbounded();
        let ctx = ToolContext::new(
            Arc::new(Mutex::new(settings)),
            AppPaths {
                exe_dir: dir.path().to_path_buf(),
                data_dir: Some(dir.path().join("data")),
            },
            events_tx,
            requests_tx,
        );
        let matches = tool
            .cli_command()
            .try_get_matches_from([
                "export-upgrader",
                "upgrade",
                export.to_str().unwrap(),
                "--savefile",
                save.to_str().unwrap(),
            ])
            .unwrap();
        assert_eq!(tool.cli_run(&matches, &ctx).unwrap(), 0);
        drop(ctx);
        let events: Vec<PipelineEventEnvelope> = events_rx.iter().collect();

        // The counters the line draws, after each event.
        let mut counts = RunCounts::new();
        let lines: Vec<String> = events
            .iter()
            .filter_map(|envelope| {
                counts.observe(&envelope.event);
                counts.line()
            })
            .collect();
        assert_eq!(lines.first().map(String::as_str), Some("0/1 exports"));
        assert!(
            lines.iter().all(|line| !line.contains("/2 exports")),
            "one upgrade is one export, never two: {lines:?}"
        );
        // The parts are the files the output holds: `colors.txt`, the face
        // folder's files and its `settings.toml` here.
        let files = file_count(&dir.path().join("co Full - Spring"));
        // A line keeps its parts pair until the next `Progress`, so
        // consecutive duplicates collapse to the events' own sequence.
        let mut parts: Vec<(usize, usize)> =
            lines.iter().filter_map(|line| parts_of(line)).collect();
        parts.dedup();
        assert!(
            !parts.is_empty(),
            "the parts counter appears once the files are planned"
        );
        assert!(
            parts.iter().all(|(_, total)| *total == files),
            "the parts total is the output's file count: {parts:?} for {files} files"
        );
        assert!(
            parts.windows(2).all(|pair| pair[0].0 + 1 == pair[1].0),
            "each write counts one part: {parts:?}"
        );
        assert_eq!(parts.last().unwrap().0, files);
        assert_eq!(
            lines.last().unwrap(),
            &format!("1/1 exports | {0}/{0} parts", grouped(files))
        );

        // The finding lines are the run's, the line drawn or not.
        let printed = |draw: bool| {
            let (tx, rx) = unbounded();
            for envelope in events.iter().cloned() {
                tx.send(envelope).unwrap();
            }
            drop(tx);
            let (mut out, mut err) = (Vec::new(), Vec::new());
            print_events(rx, draw, &mut out, &mut err).unwrap();
            (out, err)
        };
        let (out_draw, _) = printed(true);
        let (out_piped, err_piped) = printed(false);
        assert_eq!(out_draw, out_piped, "the line changes no stdout byte");
        assert_eq!(err_piped, b"", "nothing is drawn on a piped stderr");
    }

    #[test]
    fn the_lines_print_the_same_stdout_whether_the_progress_line_draws_or_not() {
        let mut out_no_draw = Vec::new();
        let mut err_no_draw = Vec::new();
        print_events(run_events(), false, &mut out_no_draw, &mut err_no_draw).unwrap();
        assert_eq!(out_no_draw, b"- co One: Warning bad_file\n");
        assert_eq!(err_no_draw, b"", "nothing is drawn on a piped stderr");

        let mut out_draw = Vec::new();
        let mut err_draw = Vec::new();
        print_events(run_events(), true, &mut out_draw, &mut err_draw).unwrap();
        assert_eq!(out_draw, out_no_draw, "the line changes no stdout byte");
        let err = String::from_utf8(err_draw).unwrap();
        assert!(
            err.starts_with(
                "Done: 12 files written, 1 export skipped, 2 folders skipped, 3 folders with errors, 0:"
            ),
            "the summary is all a drawn run leaves on stderr: {err}"
        );
    }

    #[test]
    fn the_progress_line_is_the_counters_alone_the_template_adding_brackets_and_clock() {
        assert_eq!(
            progress_line(3, 12, Some((1284, 4010))),
            "3/12 exports | 1,284/4,010 parts"
        );
        assert_eq!(progress_line(0, 1, None), "0/1 exports");
    }

    #[test]
    fn the_summary_line_counts_the_run_with_each_plural_right() {
        assert_eq!(
            summary_line(1284, 2, 2, 1, Duration::from_secs(42)),
            "Done: 1,284 files written, 2 exports skipped, 2 folders skipped, 1 folder with errors, 0:42"
        );
        assert_eq!(
            summary_line(1, 1, 1, 0, Duration::from_secs(5)),
            "Done: 1 file written, 1 export skipped, 1 folder skipped, 0 folders with errors, 0:05"
        );
        assert_eq!(
            summary_line(0, 0, 0, 3, Duration::from_secs(62 * 60 + 7)),
            "Done: 0 files written, 0 exports skipped, 0 folders skipped, 3 folders with errors, 62:07"
        );
    }

    #[test]
    fn the_progress_counters_start_empty_and_count_exports_and_parts() {
        let mut counts = RunCounts::new();
        assert_eq!(counts.line(), None, "no line before the first export");
        let finding = run_events()
            .iter()
            .map(|envelope| envelope.event)
            .find(|event| matches!(event, PipelineEvent::Message(_)))
            .unwrap();
        counts.observe(&finding);
        assert_eq!(counts.line(), None, "a finding is no export either");
        counts.observe(&PipelineEvent::ExportStarted {
            export_id: ExportId(1),
            display_name: "co One".to_owned(),
        });
        counts.observe(&PipelineEvent::ExportStarted {
            export_id: ExportId(2),
            display_name: "co Two".to_owned(),
        });
        assert_eq!(counts.line().as_deref(), Some("0/2 exports"));
        counts.observe(&PipelineEvent::ExportProcessed {
            export_id: ExportId(1),
        });
        counts.observe(&PipelineEvent::Progress {
            processed: 3,
            total: 6,
        });
        assert_eq!(counts.line().as_deref(), Some("1/2 exports | 3/6 parts"));
    }

    #[test]
    fn an_exports_name_alone_is_no_export_started() {
        // A name for an id that is no export of its own (the upgrader's
        // output) neither starts the drawing nor raises the total.
        let mut counts = RunCounts::new();
        counts.observe(&PipelineEvent::ExportStarted {
            export_id: ExportId(1),
            display_name: "co Spring".to_owned(),
        });
        counts.observe(&PipelineEvent::ExportNamed {
            export_id: ExportId(2),
            display_name: "co Full Spring".to_owned(),
        });
        assert_eq!(counts.line().as_deref(), Some("0/1 exports"));
    }

    #[test]
    fn the_progress_counters_never_count_more_exports_done_than_started() {
        let mut counts = RunCounts::new();
        for envelope in run_events() {
            counts.observe(&envelope.event);
        }
        assert!(
            counts.exports_done <= counts.exports_total,
            "{} of {}",
            counts.exports_done,
            counts.exports_total
        );
        assert_eq!(counts.line().as_deref(), Some("2/2 exports | 3/3 parts"));
    }

    #[test]
    fn grouped_groups_digits_by_threes() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(284), "284");
        assert_eq!(grouped(284_010), "284,010");
    }

    #[test]
    fn the_notice_printer_writes_each_notices_text_and_skips_other_requests() {
        let (tx, rx) = unbounded();
        tx.send(ShellRequest::Notify(Notice {
            text: "Switched to PES 2019".to_owned(),
            kind: NoticeKind::Info,
            action: None,
        }))
        .unwrap();
        tx.send(ShellRequest::SettingsChanged).unwrap();
        tx.send(ShellRequest::Close).unwrap();
        tx.send(ShellRequest::Notify(Notice {
            text: "Compiled 48 teams".to_owned(),
            kind: NoticeKind::Success,
            action: None,
        }))
        .unwrap();
        drop(tx);
        let mut out = Vec::new();
        print_notices(rx, &mut out).unwrap();
        assert_eq!(out, b"Switched to PES 2019\nCompiled 48 teams\n");
    }

    #[test]
    fn the_timestamp_prefix_is_whole_seconds_then_three_truncated_milliseconds() {
        assert_eq!(timestamp_prefix(Duration::from_millis(42)), "+0.042 ");
        assert_eq!(
            timestamp_prefix(Duration::from_micros(125_500_999)),
            "+125.500 "
        );
    }

    // TC-CLI-13
    #[test]
    fn a_timed_finding_prints_its_seconds_since_the_run_started_before_the_line() {
        let (tx, rx) = unbounded();
        tx.send(envelope(PipelineEvent::ExportStarted {
            export_id: ExportId(1),
            display_name: "co Spring.zip".to_owned(),
        }))
        .unwrap();
        let finding = || {
            PipelineEvent::Message(Message {
                code: MessageCode {
                    tool_id: "team-compiler",
                    code: Cow::Borrowed("bad_file"),
                },
                severity: Severity::Warning,
                disposition: Disposition::Keep,
                scope: Scope::Export {
                    export_id: ExportId(1),
                },
                context: Vec::new(),
            })
        };
        tx.send(PipelineEventEnvelope {
            made_at: Some(Duration::from_millis(12_345)),
            ..envelope(finding())
        })
        .unwrap();
        tx.send(envelope(finding())).unwrap();
        drop(tx);

        let mut out = Vec::new();
        print_events(rx, false, &mut out, &mut Vec::new()).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "+12.345 - co Spring.zip: Warning bad_file\n- co Spring.zip: Warning bad_file\n"
        );
    }
}
