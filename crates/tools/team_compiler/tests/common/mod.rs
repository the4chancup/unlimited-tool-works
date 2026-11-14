//! The sandbox the in-process tests run the Compiler's command line in: a fresh folder
//! standing in for the executable's, with its own settings text and teams list.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crossbeam_channel::unbounded;
use studio_core::{
    AppPaths, CliError, PipelineEventEnvelope, RunningPes, Settings, StudioTool, ToolContext,
};
use team_compiler::Tool;

/// The teams list every sandbox carries, so identities are predictable: `/co/` for the tests'
/// own exports, `/a/`, `/b/`, `/dbg/` and `/esg/` for more teams of their own, `/egg/` for the
/// tracer fixture, `/jp/` for the pre-Fox one. Each row is the bundled upstream list's own, so
/// an id a test asserts is the one a member sees.
const TEAMS_LIST: &str =
    "ID\tName\n702\t/a/\n707\t/b/\n714\t/co/\n731\t/jp/\n790\t/dbg/\n792\t/egg/\n793\t/esg/\n";

/// A fresh folder standing in for the executable's folder: `data/` holds the teams list, the
/// exports root defaults to `exports/` beside it and the output folder to `output/`.
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
            .join("team_compiler_in_process")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("data")).unwrap();
        fs::write(root.join("data/teams_list.txt"), TEAMS_LIST).unwrap();
        Sandbox { root }
    }

    /// `settings` with `pes_folder_path` set to the sandbox's `PES/` when the text names no
    /// game folder: the default is the real install's path, which a compile that deploys would
    /// write into; and `savefile_path` to the sandbox's `save/EDIT00000000` when the text names
    /// none: the `auto` default would find the developer's own save.
    fn sandboxed(&self, settings: &str) -> String {
        let mut root: toml::Table = toml::from_str(settings).unwrap();
        let common = root
            .entry("common")
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .unwrap();
        common
            .entry("pes_folder_path")
            .or_insert_with(|| toml::Value::String(self.root.join("PES").display().to_string()));
        let tool = root
            .entry("compiler")
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .unwrap();
        tool.entry("savefile_path").or_insert_with(|| {
            toml::Value::String(
                self.root
                    .join("save")
                    .join("EDIT00000000")
                    .display()
                    .to_string(),
            )
        });
        toml::to_string(&root).unwrap()
    }

    /// Runs `compiler <args>` with `settings` as the settings file's text, its game folder
    /// the sandbox's unless it names one (`sandboxed`), and a probe reporting no game: no test
    /// reads the machine's processes.
    pub fn run(&self, settings: &str, args: &[&str]) -> Run {
        self.run_with_running(settings, args, Vec::new)
    }

    /// `run` with `running` as the tool's running-PES probe in place of the no-game one
    /// (`Tool::with_running_probe`), for the tests of the savefile step's running-game cases.
    pub fn run_with_running(
        &self,
        settings: &str,
        args: &[&str],
        running: fn() -> Vec<RunningPes>,
    ) -> Run {
        let tool = Tool::with_running_probe(running);
        let mut parsed = Settings::parse(&self.sandboxed(settings)).unwrap();
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
            .try_get_matches_from(std::iter::once("compiler").chain(args.iter().copied()))
            .unwrap();
        let result = tool.cli_run(&matches, &ctx);
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
}
