//! The sandbox the in-process tests run the Ruleset editor's command line in:
//! a fresh folder standing in for the executable's, and the tool run as the
//! binary runs it (its settings merged, its events collected).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crossbeam_channel::unbounded;
use ruleset_editor::Tool;
use studio_core::{
    AppPaths, CliError, EventLines, PipelineEvent, PipelineEventEnvelope, Settings, StudioTool,
    ToolContext,
};

/// A fresh folder standing in for the executable's folder; `data/` is the data directory.
pub struct Sandbox {
    pub root: PathBuf,
}

/// What one command returned, and the events it emitted.
pub struct Run {
    pub result: Result<u8, CliError>,
    pub events: Vec<PipelineEventEnvelope>,
}

impl Sandbox {
    pub fn new(name: &str) -> Sandbox {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("ruleset_editor_in_process")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("data")).unwrap();
        Sandbox { root }
    }

    /// An absolute path inside the sandbox.
    pub fn path(&self, relative: &str) -> PathBuf {
        relative
            .split('/')
            .fold(self.root.clone(), |path, segment| path.join(segment))
    }

    /// An absolute path inside the sandbox, as a command-line argument.
    pub fn arg(&self, relative: &str) -> String {
        self.path(relative).to_str().unwrap().to_owned()
    }

    /// Runs `ruleset-editor <args>` with no settings file content (the
    /// defaults), the sandbox's `data/` as the data directory. A clap refusal
    /// is what the binary makes of it: its exit code (2 for a usage error)
    /// with its message, and no event.
    pub fn run(&self, args: &[&str]) -> Run {
        let tool = Tool::new();
        let mut parsed = Settings::parse("").unwrap();
        parsed.merge_defaults(tool.id(), &tool.default_settings());
        let (events_tx, events_rx) = unbounded();
        let (requests_tx, _requests_rx) = unbounded();
        let ctx = ToolContext::new(
            Arc::new(Mutex::new(parsed)),
            AppPaths {
                exe_dir: self.root.clone(),
                data_dir: Some(self.root.join("data")),
            },
            events_tx,
            requests_tx,
        );
        let matches = tool
            .cli_command()
            .try_get_matches_from(std::iter::once("ruleset-editor").chain(args.iter().copied()));
        let result = match matches {
            Ok(matches) => tool.cli_run(&matches, &ctx),
            Err(error) => Err(CliError::new(
                u8::try_from(error.exit_code()).unwrap(),
                error,
            )),
        };
        drop(ctx);
        Run {
            result,
            events: events_rx.iter().collect(),
        }
    }
}

impl Run {
    pub fn exit_code(&self) -> u8 {
        match &self.result {
            Ok(code) => *code,
            Err(error) => panic!("refused with {}: {error}", error.exit_code),
        }
    }

    /// The refusal, asserting its exit code and that its message names every
    /// one of `names`.
    pub fn assert_refused(&self, exit_code: u8, names: &[&str]) {
        let error = match &self.result {
            Ok(code) => panic!("not refused: exit code {code}"),
            Err(error) => error,
        };
        let text = error.to_string();
        assert_eq!(error.exit_code, exit_code, "{text}");
        for name in names {
            assert!(
                text.contains(name),
                "the refusal does not name {name}: {text}"
            );
        }
        assert!(self.events.is_empty(), "a refused command emits nothing");
    }

    /// The run's stdout as the console prints it: every event's line as
    /// `EventLines` formats it, each ended as `writeln!` does.
    pub fn stdout(&self) -> String {
        let mut lines = EventLines::new();
        let mut stdout = String::new();
        for envelope in &self.events {
            if let Some(line) = lines.line(&envelope.event) {
                stdout.push_str(&line);
                stdout.push('\n');
            }
        }
        stdout
    }

    /// Every `Message` event as one line: severity, code and context, so a
    /// test asserts all of them at once. Every finding is run-wide, so there
    /// is no source or location.
    pub fn messages(&self) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|envelope| match &envelope.event {
                PipelineEvent::Message(message) => {
                    let context: Vec<String> = message
                        .context
                        .iter()
                        .map(|(key, value)| format!("{key}={value}"))
                        .collect();
                    Some(format!(
                        "{:?} {} ({})",
                        message.severity,
                        message.code.code,
                        context.join(", ")
                    ))
                }
                PipelineEvent::ExportStarted { .. }
                | PipelineEvent::ExportNamed { .. }
                | PipelineEvent::Output(_)
                | PipelineEvent::ExportsExpected { .. }
                | PipelineEvent::ExportProcessed { .. }
                | PipelineEvent::FolderStatus { .. }
                | PipelineEvent::Progress { .. }
                | PipelineEvent::Complete { .. }
                | PipelineEvent::UnmigratedContentFound => None,
            })
            .collect()
    }
}
