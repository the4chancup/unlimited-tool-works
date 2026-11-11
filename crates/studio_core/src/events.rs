//! Pipeline events: what a tool's pipeline reports while it runs, and the structured findings
//! (`Message`) every tool emits instead of printing text.
//!
//! The CLI prints events as console lines; the GUI turns them into grid cells and log lines. Every
//! event travels in a `PipelineEventEnvelope`, whose run and revision ids let the GUI drop results
//! from a run or a filesystem snapshot that is no longer current.
//!
//! `EventLines` is the console's line format. The `studio` binary's printer and a tool's plain
//! run log both use it, so a finding reads the same in the terminal and in the window.

use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

use vtree::ScopePath;

/// Identifies one pipeline run. Events from an older run are stale and are dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RunId(pub u64);

/// Stable identity of one export (a folder or an archive) within a source set, independent of its
/// display name: two exports may share a display stem, and a rename must not confuse the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExportId(pub u64);

/// Revision of an export's on-disk contents; the folder watcher bumps it on every change.
/// Events carrying an older revision describe a snapshot that no longer exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExportRevision(pub u64);

/// A pipeline event with the identity needed to detect a stale result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineEventEnvelope {
    /// The run this event belongs to.
    pub run_id: RunId,
    /// The export this event is about, if any (run-wide events carry `None`).
    pub export_id: Option<ExportId>,
    /// The export revision the event was computed from, if any.
    pub export_revision: Option<ExportRevision>,
    /// When the event was made, measured from the run's start: `Some` only on a `Message`
    /// of a run asked to time its findings (`team-compiler`'s `--timestamps`), `None` on
    /// every other event.
    pub made_at: Option<Duration>,
    /// The event itself.
    pub event: PipelineEvent,
}

/// What a pipeline reports while it runs. Tool-neutral: team names, team ids and other
/// tool-specific identity belong in tool state or in a `Message`'s context, not here.
///
/// `Progress` and `Complete` are placeholder shapes: the Team compiler plan extends them (run strip
/// metrics, structured compile/deployment outcomes) before the views that need them are built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineEvent {
    /// An export entered the pipeline; `display_name` is what the grid row shows.
    ExportStarted {
        /// The export.
        export_id: ExportId,
        /// Human-readable name for the row label.
        display_name: String,
    },
    /// A name for the findings of an export id that is not an export of its own in the
    /// run: the Export upgrader's output, the second half of one upgrade. It names the
    /// id for `EventLines` exactly as `ExportStarted` does, but no counter counts it.
    ExportNamed {
        /// The export the name is for.
        export_id: ExportId,
        /// Human-readable name for the findings' lines.
        display_name: String,
    },
    /// An export left the pipeline, whatever its outcome.
    ExportProcessed {
        /// The export.
        export_id: ExportId,
    },
    /// A folder (player, kit, shared) inside an export changed state.
    FolderStatus {
        /// The export the folder belongs to.
        export_id: ExportId,
        /// The folder's canonical path within the export.
        path: ScopePath,
        /// Its new state.
        status: FolderStatus,
    },
    /// A structured finding for the user (a warning, an error, an info line).
    Message(Message),
    /// A command's result text (a JSON report, a document): the console prints it to stdout
    /// as it is, no prefix or folding, and a tool's log shows it as one entry. A command
    /// whose result is such text reports nothing else on success, so stdout parses
    /// (`save-editor diff --json`).
    Output(String),
    // The run strip needs stage, elapsed time, files written, and memory in
    // use — more than `Progress` carries. Extend `Progress` with these fields
    // or add separate `StageChanged`/`MetricsUpdated` events before implementing
    // the run strip view (see "Toolbar" in the Team compiler plan).
    /// Coarse run progress: `processed` of `total` units done.
    Progress {
        /// Units finished so far.
        processed: usize,
        /// Units in the run.
        total: usize,
    },
    // The Team compiler needs separate compile and deployment outcomes (rollback,
    // "artifacts built but installation failed"). Replace the boolean `success`
    // with structured CompileOutcome/DeploymentOutcome/CancellationOutcome before
    // implementing the run-result semantics described in the Team compiler plan.
    // At that point the placeholder folder-only counters below become task/scoped-
    // outcome counts, so typed non-folder tasks are represented in the summary.
    /// The run ended.
    Complete {
        /// Whether the run as a whole succeeded.
        success: bool,
        /// Files written to the output.
        files_written: usize,
        /// The distinct exports a finding refused whole.
        skipped_exports: usize,
        /// Folders skipped because of their findings.
        skipped_folders: usize,
        /// Duplicate entries dropped.
        duplicates: usize,
        /// Folders that ended with error-level findings.
        folders_with_errors: usize,
    },
    /// Content the tool could not place anywhere (the Export upgrader's leftovers).
    UnmigratedContentFound,
}

/// The state of one grid cell: a folder or slot during the check phase, then during compilation.
///
/// There is no "pending" compile state: every checked cell is implicitly pending once compilation
/// starts, and `Processing` keeps the check-phase color while bolding the label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FolderStatus {
    /// Not validated yet.
    Unchecked,
    /// Validation in progress.
    Checking,
    /// Shallow-checked, no issues found (an archive not yet extracted).
    PartialOk,
    /// Deep-checked, no issues.
    FullOk,
    /// Checked with warnings; does not block compilation.
    Warning,
    /// Shallow check found errors; the list may be incomplete.
    PartialError,
    /// Deep check found errors; the list is authoritative.
    Error,
    /// Being processed by the pipeline.
    Processing,
    /// Processed successfully.
    Done,
    /// Processed with warnings.
    DoneWithWarning,
    /// Written despite error-level findings (pass-through, or a dropped file whose folder was
    /// otherwise processed).
    DoneWithErrors,
}

/// Stable, cross-tool identity of a finding: the owning tool plus its code within that tool's
/// message catalog. The catalog in the tool's `messages.rs` maps the code to text, severity and
/// disposition; the help window's generated "Messages" topic lists every code, and a log line
/// links to it through the tool id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MessageCode {
    /// The tool whose catalog defines the code (`team-compiler`).
    pub tool_id: &'static str,
    /// The code within that catalog (`player_number_duplicate`); borrowed for catalog constants,
    /// owned when a code is built at run time.
    pub code: Cow<'static, str>,
}

/// A structured finding: stable code, how serious it is, what the pipeline did about it, what
/// it is about, and the context fields the display template fills in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Cross-tool stable identity.
    pub code: MessageCode,
    /// Presentation and filtering.
    pub severity: Severity,
    /// Processing consequence, independent of severity.
    pub disposition: Disposition,
    /// What the message is about; drives grid cell mapping.
    pub scope: Scope,
    /// Structured fields (file name, expected and found values, ...), in template order.
    pub context: Vec<(String, String)>,
}

/// How serious a finding is. Ordered: `Info < Warning < Error < Fatal`; the "errors" filter
/// includes `Fatal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    /// Informational.
    Info,
    /// Something worth knowing that did not stop processing.
    Warning,
    /// Something was dropped or written wrong.
    Error,
    /// The run cannot continue.
    Fatal,
}

impl Severity {
    /// The severity's name as the console line and the help's Messages topic spell it.
    pub fn name(self) -> &'static str {
        match self {
            Severity::Info => "Info",
            Severity::Warning => "Warning",
            Severity::Error => "Error",
            Severity::Fatal => "Fatal",
        }
    }
}

/// What the pipeline did in response to a finding. A consequence, not a severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Disposition {
    /// Content kept as is.
    Keep,
    /// One file discarded.
    DropFile,
    /// One normalized roster assignment (a `players.txt` line) discarded.
    DropSlot,
    /// One folder discarded.
    DropFolder,
    /// The whole export discarded.
    DropExport,
    /// The run aborted.
    AbortRun,
}

/// What a finding is about. Identity travels as ids and canonical paths; human-readable names
/// stay in the message context.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Not tied to an export (output stage, settings).
    Run,
    /// A whole export.
    Export {
        /// The export.
        export_id: ExportId,
    },
    /// A player, shared or kit folder inside an export.
    Folder {
        /// The export.
        export_id: ExportId,
        /// The folder's canonical path within the export.
        path: ScopePath,
    },
    /// One file inside an export.
    File {
        /// The export.
        export_id: ExportId,
        /// The file's canonical path within the export.
        path: ScopePath,
    },
    /// One line of a roster file (`players.txt`).
    RosterEntry {
        /// The export.
        export_id: ExportId,
        /// The roster file.
        file: ScopePath,
        /// 1-based line number.
        line: usize,
        /// The slot the line assigns, when it could be read.
        slot: Option<u16>,
    },
}

impl Scope {
    /// The export this scope belongs to, if it is not run-wide.
    pub fn export_id(&self) -> Option<ExportId> {
        match self {
            Scope::Run => None,
            Scope::Export { export_id }
            | Scope::Folder { export_id, .. }
            | Scope::File { export_id, .. }
            | Scope::RosterEntry { export_id, .. } => Some(*export_id),
        }
    }
}

/// Turns pipeline events into the console's `-` lines: one self-contained line per `Message`
/// (source name, severity, code, location, context fields), so lines from exports processed in
/// parallel never need a header to be read. Remembers each export's source name from its
/// `ExportStarted` or `ExportNamed` event, so one value serves one run: export ids restart with
/// the next.
#[derive(Debug, Default)]
pub struct EventLines {
    names: HashMap<ExportId, String>,
}

impl EventLines {
    /// A formatter that has seen no export start yet.
    pub fn new() -> EventLines {
        EventLines::default()
    }

    /// The line for one event, or `None` for events the console does not show.
    pub fn line(&mut self, event: &PipelineEvent) -> Option<String> {
        match event {
            PipelineEvent::ExportStarted {
                export_id,
                display_name,
            }
            | PipelineEvent::ExportNamed {
                export_id,
                display_name,
            } => {
                self.names.insert(*export_id, display_name.clone());
                None
            }
            PipelineEvent::Message(message) => Some(self.message_line(message)),
            PipelineEvent::Output(text) => Some(text.clone()),
            PipelineEvent::ExportProcessed { .. }
            | PipelineEvent::FolderStatus { .. }
            | PipelineEvent::Progress { .. }
            | PipelineEvent::Complete { .. }
            | PipelineEvent::UnmigratedContentFound => None,
        }
    }

    fn message_line(&self, message: &Message) -> String {
        let source = match message.scope.export_id() {
            None => String::new(),
            Some(export_id) => match self.names.get(&export_id) {
                Some(name) => format!("{name}: "),
                // A tool that reports before `ExportStarted` still gets a readable line.
                None => format!("export {}: ", export_id.0),
            },
        };
        let location = match &message.scope {
            Scope::Run | Scope::Export { .. } => String::new(),
            Scope::Folder { path, .. } | Scope::File { path, .. } => {
                format!(" at {}", path.as_str())
            }
            Scope::RosterEntry { file, line, .. } => format!(" at {} line {line}", file.as_str()),
        };
        let context = if message.context.is_empty() {
            String::new()
        } else {
            let fields: Vec<String> = message
                .context
                .iter()
                .map(|(key, value)| format!("{key}={}", one_line(value)))
                .collect();
            format!(" ({})", fields.join(", "))
        };
        format!(
            "- {source}{} {}{location}{context}",
            message.severity.name(),
            message.code.code
        )
    }
}

/// `value` as one console line: its lines trimmed, the empty ones dropped, the rest joined
/// with a space — a multi-line error (a TOML parse error's source snippet) stays inside its
/// finding's line.
fn one_line(value: &str) -> String {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_orders_info_below_fatal() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Fatal);
    }

    #[test]
    fn same_code_in_two_tools_is_two_identities() {
        let compiler = MessageCode {
            tool_id: "team-compiler",
            code: Cow::Borrowed("bad_file"),
        };
        let upgrader = MessageCode {
            tool_id: "export-upgrader",
            code: Cow::Borrowed("bad_file"),
        };
        assert_ne!(compiler, upgrader);
        assert_eq!(
            compiler,
            MessageCode {
                tool_id: "team-compiler",
                code: Cow::Owned("bad_file".to_owned())
            }
        );
    }

    #[test]
    fn scope_reports_its_export() {
        let path = ScopePath::new("Kits/p1").unwrap();
        assert_eq!(Scope::Run.export_id(), None);
        assert_eq!(
            Scope::Folder {
                export_id: ExportId(7),
                path
            }
            .export_id(),
            Some(ExportId(7))
        );
    }

    fn message(severity: Severity, code: &'static str, scope: Scope) -> PipelineEvent {
        PipelineEvent::Message(Message {
            code: MessageCode {
                tool_id: "team-compiler",
                code: Cow::Borrowed(code),
            },
            severity,
            disposition: Disposition::Keep,
            scope,
            context: Vec::new(),
        })
    }

    fn started(id: u64, name: &str) -> PipelineEvent {
        PipelineEvent::ExportStarted {
            export_id: ExportId(id),
            display_name: name.to_owned(),
        }
    }

    fn path(text: &str) -> ScopePath {
        ScopePath::new(text).unwrap()
    }

    /// A formatter that has seen export 1 start as `co Midcup Spring.zip`.
    fn printer() -> EventLines {
        let mut printer = EventLines::new();
        assert_eq!(printer.line(&started(1, "co Midcup Spring.zip")), None);
        printer
    }

    #[test]
    fn a_run_scoped_message_has_no_source_and_no_location() {
        let event = message(Severity::Error, "multiple_ref_exports", Scope::Run);
        assert_eq!(
            printer().line(&event).unwrap(),
            "- Error multiple_ref_exports"
        );
    }

    #[test]
    fn an_export_scoped_message_names_its_source() {
        let event = message(
            Severity::Info,
            "export_identified",
            Scope::Export {
                export_id: ExportId(1),
            },
        );
        assert_eq!(
            printer().line(&event).unwrap(),
            "- co Midcup Spring.zip: Info export_identified"
        );
    }

    #[test]
    fn a_named_exports_findings_carry_its_name() {
        // The name of an id that is no export of its own (the upgrader's
        // output) serves its findings' lines exactly as a started one's.
        let mut printer = printer();
        assert_eq!(
            printer.line(&PipelineEvent::ExportNamed {
                export_id: ExportId(2),
                display_name: "co Full Spring".to_owned(),
            }),
            None
        );
        let event = message(
            Severity::Error,
            "file_type_disallowed",
            Scope::Export {
                export_id: ExportId(2),
            },
        );
        assert_eq!(
            printer.line(&event).unwrap(),
            "- co Full Spring: Error file_type_disallowed"
        );
    }

    #[test]
    fn a_folder_scoped_message_shows_the_folder_path() {
        let event = message(
            Severity::Warning,
            "player_unlisted",
            Scope::Folder {
                export_id: ExportId(1),
                path: path("Players/15 - B"),
            },
        );
        assert_eq!(
            printer().line(&event).unwrap(),
            "- co Midcup Spring.zip: Warning player_unlisted at Players/15 - B"
        );
    }

    #[test]
    fn a_file_scoped_message_shows_the_file_path() {
        let event = message(
            Severity::Fatal,
            "file_unreadable",
            Scope::File {
                export_id: ExportId(1),
                path: path("Players/15 - B/face.dds"),
            },
        );
        assert_eq!(
            printer().line(&event).unwrap(),
            "- co Midcup Spring.zip: Fatal file_unreadable at Players/15 - B/face.dds"
        );
    }

    #[test]
    fn a_roster_entry_message_shows_file_and_line() {
        let event = message(
            Severity::Error,
            "players_txt_line_invalid",
            Scope::RosterEntry {
                export_id: ExportId(1),
                file: path("players.txt"),
                line: 1,
                slot: None,
            },
        );
        assert_eq!(
            printer().line(&event).unwrap(),
            "- co Midcup Spring.zip: Error players_txt_line_invalid at players.txt line 1"
        );
    }

    #[test]
    fn a_context_value_with_line_breaks_prints_on_one_line() {
        let PipelineEvent::Message(mut message) =
            message(Severity::Error, "kit_config_invalid", Scope::Run)
        else {
            unreachable!("the helper builds a Message");
        };
        message.context = vec![(
            "error".to_owned(),
            "TOML parse error at line 1, column 10\n  |\n1 | shirt = [\n  |          ^\nexpected a value"
                .to_owned(),
        )];
        assert_eq!(
            printer().line(&PipelineEvent::Message(message)).unwrap(),
            "- Error kit_config_invalid (error=TOML parse error at line 1, column 10 | 1 | shirt = [ |          ^ expected a value)"
        );
    }

    #[test]
    fn context_fields_follow_in_parentheses_in_the_given_order() {
        let PipelineEvent::Message(mut message) =
            message(Severity::Error, "multiple_ref_exports", Scope::Run)
        else {
            unreachable!("the helper builds a Message");
        };
        message.context = vec![
            ("exports".to_owned(), "refs a, refs b".to_owned()),
            ("count".to_owned(), "2".to_owned()),
        ];
        assert_eq!(
            printer().line(&PipelineEvent::Message(message)).unwrap(),
            "- Error multiple_ref_exports (exports=refs a, refs b, count=2)"
        );
    }

    #[test]
    fn an_export_that_never_started_is_named_by_its_number() {
        let event = message(
            Severity::Warning,
            "player_unlisted",
            Scope::Export {
                export_id: ExportId(7),
            },
        );
        assert_eq!(
            printer().line(&event).unwrap(),
            "- export 7: Warning player_unlisted"
        );
    }

    #[test]
    fn exports_with_the_same_stem_stay_distinguishable() {
        let mut printer = EventLines::new();
        printer.line(&started(1, "co Midcup Spring"));
        printer.line(&started(2, "co Midcup Spring.zip"));
        let line_for = |printer: &mut EventLines, id: u64| {
            printer
                .line(&message(
                    Severity::Error,
                    "players_txt_missing",
                    Scope::Export {
                        export_id: ExportId(id),
                    },
                ))
                .unwrap()
        };
        assert_eq!(
            line_for(&mut printer, 1),
            "- co Midcup Spring: Error players_txt_missing"
        );
        assert_eq!(
            line_for(&mut printer, 2),
            "- co Midcup Spring.zip: Error players_txt_missing"
        );
    }

    #[test]
    fn an_output_is_its_text_unchanged() {
        let text = "{\n  \"players\": [\n    { \"id\": 70105 }\n  ]\n}";
        assert_eq!(
            EventLines::new().line(&PipelineEvent::Output(text.to_owned())),
            Some(text.to_owned())
        );
    }

    #[test]
    fn events_other_than_messages_print_nothing() {
        let mut printer = EventLines::new();
        let silent = [
            PipelineEvent::ExportProcessed {
                export_id: ExportId(1),
            },
            PipelineEvent::FolderStatus {
                export_id: ExportId(1),
                path: path("Players/15 - B"),
                status: FolderStatus::Done,
            },
            PipelineEvent::Progress {
                processed: 1,
                total: 2,
            },
            PipelineEvent::Complete {
                success: true,
                files_written: 0,
                skipped_exports: 0,
                skipped_folders: 0,
                duplicates: 0,
                folders_with_errors: 0,
            },
            PipelineEvent::UnmigratedContentFound,
        ];
        for event in &silent {
            assert_eq!(printer.line(event), None);
        }
    }
}
