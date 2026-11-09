//! The sandbox the in-process tests run the Export upgrader's command line in:
//! a fresh folder under `CARGO_TARGET_TMPDIR` holding the data directory, the
//! inputs and the outputs; the tool run as the binary runs it (its settings
//! merged, its events collected).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crossbeam_channel::unbounded;
use export_upgrader::Tool;
use pes_version::PesVersion;
use studio_core::{
    AppPaths, CliError, EventLines, PipelineEvent, PipelineEventEnvelope, Settings, StudioTool,
    ToolContext,
};

/// A fresh folder standing in for the executable's folder; `data/` is the data
/// directory.
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
            .join("export_upgrader_in_process")
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

    /// Writes the PES 21 fixture save (a valid encrypted `EDIT00000000`, which
    /// holds `/co/`, 714) at `relative`, and returns its path.
    pub fn save(&self, relative: &str) -> PathBuf {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        path
    }

    /// Writes `bytes` at `relative`, parent folders created.
    pub fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
    }

    /// Writes `version`'s fixture save at `relative` with `edit` applied to
    /// its model first (a real encrypted `EDIT00000000` still).
    pub fn save_edited(
        &self,
        relative: &str,
        version: PesVersion,
        edit: impl FnOnce(&mut pes_savefile::EditFile),
    ) {
        let mut file =
            pes_savefile::EditFile::from_bytes(&pes_savefile::fixture_save(version)).unwrap();
        edit(&mut file);
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, file.to_bytes(&[7; 320]).unwrap()).unwrap();
    }

    /// Copies the fixture file `name` (`tests/fixtures/<name>`) to `relative`
    /// in the sandbox.
    pub fn copy_fixture(&self, name: &str, relative: &str) {
        let target = self.path(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(self.fixture(name), &target).unwrap();
    }

    /// Copies the fixture folder `name` (`tests/fixtures/<name>`) to
    /// `relative` in the sandbox.
    pub fn copy_fixture_dir(&self, name: &str, relative: &str) {
        copy_dir(&self.fixture(name), &self.path(relative));
    }

    /// The fixture path inside the crate's `tests/fixtures/`.
    fn fixture(&self, name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    /// Every file and folder below `relative`, as `/`-joined relative paths,
    /// sorted: the before/after snapshot for "nothing is written".
    pub fn listing(&self, relative: &str) -> Vec<String> {
        let mut out = Vec::new();
        let root = self.path(relative);
        if root.exists() {
            list(&root, "", &mut out);
        }
        out.sort();
        out
    }

    /// Runs `export-upgrader <args>` with no settings file content (the
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
            .try_get_matches_from(std::iter::once("export-upgrader").chain(args.iter().copied()));
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

    /// Asserts the unclean run whose `stdout` is given wrote nothing: neither the output
    /// folder `output` nor its staging folder (`spec/export_upgrader.md` "CLI").
    pub fn assert_nothing_written(&self, output: &str, stdout: &str) {
        assert!(
            !self.path(output).exists() && !self.path(&format!("{output}.upgrading")).exists(),
            "nothing is written: {stdout}"
        );
    }
}

/// Every entry below the sandbox's root, its path and its bytes (a folder
/// with none): the before/after snapshot for "nothing is written".
pub fn snapshot(sandbox: &Sandbox) -> Vec<(String, Vec<u8>)> {
    sandbox
        .listing("")
        .iter()
        .map(|path| {
            let full = sandbox.path(path);
            let bytes = if full.is_file() {
                fs::read(&full).unwrap()
            } else {
                Vec::new()
            };
            (path.clone(), bytes)
        })
        .collect()
}

/// The listing of `folder` (at `prefix`) and everything below it into `out`.
fn list(folder: &Path, prefix: &str, out: &mut Vec<String>) {
    for entry in fs::read_dir(folder).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_str().unwrap().to_owned();
        let relative = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        out.push(relative.clone());
        if entry.path().is_dir() {
            list(&entry.path(), &relative, out);
        }
    }
}

/// Copies the folder `from` to `to`, recursively.
fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
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
        // A refusal reports no finding; an `ExportStarted` prints no line.
        assert!(
            self.events
                .iter()
                .all(|envelope| !matches!(envelope.event, PipelineEvent::Message(_))),
            "a refused command reports no finding"
        );
    }

    /// What the console printer writes: each event's `EventLines` line,
    /// newline-separated.
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
}
