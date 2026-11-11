//! The sandbox the in-process tests run the Save editor's command line in: a fresh folder
//! standing in for the executable's, the fixture saves written into it on request, and the
//! tool run as the binary runs it (its settings merged, its events collected).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crossbeam_channel::unbounded;
use pes_savefile::fixture_save;
use pes_version::PesVersion;
use save_editor::Tool;
use studio_core::{
    AppPaths, CliError, EventLines, PipelineEvent, PipelineEventEnvelope, RunningPes, Settings,
    StudioTool, ToolContext,
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
            .join("save_editor_in_process")
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

    /// Writes `version`'s fixture save (a valid encrypted `EDIT00000000`) at `relative`.
    pub fn fixture(&self, version: PesVersion, relative: &str) -> PathBuf {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, fixture_save(version)).unwrap();
        path
    }

    /// Runs `save-editor <args>` with no settings file content (the defaults), the sandbox's
    /// `data/` as the data directory. A clap refusal is what the binary makes of it: its exit
    /// code (2 for a usage error) with its message, and no event.
    pub fn run(&self, args: &[&str]) -> Run {
        self.run_with_data_dir(Some(self.root.join("data")), args)
    }

    /// `run` with `settings` as the settings file's text (the tool sections under their ids).
    pub fn run_with_settings(&self, settings: &str, args: &[&str]) -> Run {
        self.run_full(Some(self.root.join("data")), settings, no_game, args)
    }

    /// `run` with the data directory given (`None`: no settings file exists yet).
    pub fn run_with_data_dir(&self, data_dir: Option<PathBuf>, args: &[&str]) -> Run {
        self.run_full(data_dir, "", no_game, args)
    }

    /// `run` with `running` as the PES games the command sees running.
    pub fn run_with_running(&self, running: fn() -> Vec<RunningPes>, args: &[&str]) -> Run {
        self.run_full(Some(self.root.join("data")), "", running, args)
    }

    /// The run's shared body: `data_dir` and `settings` are the settings file's location and
    /// text, `""` standing for no file; `running` is the running-game probe, so no command
    /// reads the machine's own process list.
    fn run_full(
        &self,
        data_dir: Option<PathBuf>,
        settings: &str,
        running: fn() -> Vec<RunningPes>,
        args: &[&str],
    ) -> Run {
        let tool = Tool::new().with_running_probe(running);
        let mut parsed = Settings::parse(settings).unwrap();
        // A default that finds a real folder is set to one under the sandbox
        // before the tool's defaults merge in (CONTRIBUTING.md "Testing and
        // verification"); a settings text naming `pes_folder_path` is kept.
        if parsed.common.pes_folder_path == Settings::default().common.pes_folder_path {
            parsed.common.pes_folder_path = self.root.join("pes").display().to_string();
        }
        parsed.merge_defaults(tool.id(), &tool.default_settings());
        let (events_tx, events_rx) = unbounded();
        let (requests_tx, _requests_rx) = unbounded();
        let ctx = ToolContext::new(
            Arc::new(Mutex::new(parsed)),
            AppPaths {
                exe_dir: self.root.clone(),
                data_dir,
            },
            events_tx,
            requests_tx,
        );
        let matches = tool
            .cli_command()
            .try_get_matches_from(std::iter::once("save-editor").chain(args.iter().copied()));
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

/// The running-game probe every run gets by default: no PES runs.
fn no_game() -> Vec<RunningPes> {
    Vec::new()
}

impl Run {
    pub fn exit_code(&self) -> u8 {
        match &self.result {
            Ok(code) => *code,
            Err(error) => panic!("refused with {}: {error}", error.exit_code),
        }
    }

    /// The refusal, asserting its exit code and that its message names every one of `names`.
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

    /// What the console printer writes: each event's `EventLines` line, newline-separated,
    /// so a command's `Output` text lands verbatim among the finding lines.
    pub fn stdout(&self) -> String {
        let mut lines = EventLines::new();
        let mut out = String::new();
        for envelope in &self.events {
            if let Some(line) = lines.line(&envelope.event) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out
    }

    /// Every `Message` event as one line: severity, code and context, so a test asserts all
    /// of them at once. Every finding is run-wide, so there is no source or location.
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
                | PipelineEvent::ExportProcessed { .. }
                | PipelineEvent::FolderStatus { .. }
                | PipelineEvent::Progress { .. }
                | PipelineEvent::Complete { .. }
                | PipelineEvent::UnmigratedContentFound => None,
            })
            .collect()
    }
}
