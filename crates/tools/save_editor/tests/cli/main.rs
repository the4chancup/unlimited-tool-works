//! The Save editor's command line run in-process (`spec/save_editor.md` "CLI"): the refusals,
//! the findings and the files written, observed as the exit code, the `Message` events the tool
//! emits (which the binary prints one line each) and the saves and documents on disk.

#[path = "../common/mod.rs"]
mod common;

mod apply_patch;
mod check_aatf;
mod diff;
mod export_teams_list;
mod export_toml;
mod import_toml;
mod strip;

use std::fs;
use std::path::{Path, PathBuf};

use common::{Run, Sandbox};
use pes_savefile::EditFile;
use pes_savefile::container::SaveContainer;
use pes_savefile::model::player::STRIPPED_APPEARANCE_ID;
use pes_version::PesVersion;
use studio_core::RunningPes;

/// The salt the tests' own writes use; a save's bytes depend on it, its payload does not.
const TEST_SALT: [u8; 320] = [7; 320];

/// The decrypted payload of the save at `path`: what a byte-level comparison of two saves
/// looks at, since every write re-encrypts under a fresh salt.
fn payload(path: &Path) -> Vec<u8> {
    SaveContainer::decrypt(&fs::read(path).unwrap())
        .unwrap()
        .payload
}

/// The payload the save at `path` has after `edit` is applied to its model through the lib:
/// what a command that makes the same edit must write.
fn payload_after(path: &Path, edit: impl FnOnce(&mut EditFile)) -> Vec<u8> {
    let mut file = EditFile::load(path).unwrap();
    edit(&mut file);
    SaveContainer::decrypt(&file.to_bytes(&TEST_SALT).unwrap())
        .unwrap()
        .payload
}

impl Sandbox {
    /// Writes the PES 21 fixture save with every player's appearance id at -1 (what
    /// `strip-and-seed` leaves) at `relative`.
    fn stripped_fixture(&self, relative: &str) -> PathBuf {
        let mut file =
            EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        for player in file.players_mut() {
            player.appearance_id = Some(STRIPPED_APPEARANCE_ID);
        }
        let path = self.path(relative);
        fs::write(&path, file.to_bytes(&TEST_SALT).unwrap()).unwrap();
        path
    }
}

/// The running-game probe of a test where the save's own game, PES 2021, runs.
fn own_game_runs() -> Vec<RunningPes> {
    vec![RunningPes {
        version: PesVersion::Pes21,
        exe: None,
    }]
}

/// The running-game probe of a test where only a PES of another version runs.
fn another_version_runs() -> Vec<RunningPes> {
    vec![RunningPes {
        version: PesVersion::Pes19,
        exe: None,
    }]
}

/// A command that writes a PES 21 save, as the running-game tests run it. `setup` writes
/// its inputs into a fresh sandbox and `args` are its arguments there. It writes its outputs
/// in place, so each exists before the run: `saves` are compared by payload (every write
/// re-encrypts under a fresh salt), `files` byte for byte.
struct WritingCommand {
    name: &'static str,
    setup: fn(&Sandbox),
    args: fn(&Sandbox) -> Vec<String>,
    saves: &'static [&'static str],
    files: &'static [&'static str],
}

impl WritingCommand {
    /// A fresh sandbox `<name>_<case>` holding the command's inputs.
    fn prepare(&self, case: &str) -> Sandbox {
        let sandbox = Sandbox::new(&format!("{}_{case}", self.name));
        (self.setup)(&sandbox);
        sandbox
    }

    /// Runs the command in `sandbox` with `running` as the running games,
    /// `--edit-mode-closed` added when `edit_mode_closed` is set.
    fn run_in(
        &self,
        sandbox: &Sandbox,
        running: fn() -> Vec<RunningPes>,
        edit_mode_closed: bool,
    ) -> Run {
        let mut args = (self.args)(sandbox);
        if edit_mode_closed {
            args.push("--edit-mode-closed".to_owned());
        }
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        sandbox.run_with_running(running, &args)
    }

    /// Every output's name, the saves first.
    fn outputs(&self) -> impl Iterator<Item = &'static str> {
        self.saves.iter().chain(self.files).copied()
    }

    /// What the outputs in `sandbox` hold: each save's payload, then each file's bytes.
    fn written(&self, sandbox: &Sandbox) -> Vec<Vec<u8>> {
        let saves = self.saves.iter().map(|save| payload(&sandbox.path(save)));
        let files = self
            .files
            .iter()
            .map(|file| fs::read(sandbox.path(file)).unwrap());
        saves.chain(files).collect()
    }

    /// Runs the command with no game running, in the sandbox of `case` (each test its own),
    /// and returns what it wrote, asserting it changed every output (a skipped write would
    /// otherwise pass the comparisons made against it).
    fn written_with_no_game(&self, case: &str) -> Vec<Vec<u8>> {
        let sandbox = self.prepare(&format!("{case}_no_game"));
        let before = self.written(&sandbox);
        let run = self.run_in(&sandbox, Vec::new, false);
        assert_eq!(run.exit_code(), 0, "{}: {:?}", self.name, run.messages());
        let written = self.written(&sandbox);
        for (output, (before, after)) in self.outputs().zip(before.iter().zip(&written)) {
            assert_ne!(before, after, "{}: the command changes {output}", self.name);
        }
        written
    }

    /// With the save's own game running and no flag, the command is refused with exit code 2
    /// naming the game and the flag, and every output is left as it was, with no `.bak`.
    fn assert_refused_while_its_game_runs(&self) {
        let sandbox = self.prepare("own_game");
        let read = |output| fs::read(sandbox.path(output)).unwrap();
        let before: Vec<Vec<u8>> = self.outputs().map(read).collect();
        self.run_in(&sandbox, own_game_runs, false)
            .assert_refused(2, &["PES 2021", "--edit-mode-closed"]);
        for (output, before) in self.outputs().zip(before) {
            assert_eq!(read(output), before, "{}: {output}", self.name);
            let backup = format!("{output}.bak");
            assert!(!sandbox.path(&backup).exists(), "{}: {backup}", self.name);
        }
    }

    /// With the save's own game running and `--edit-mode-closed`, the command writes what it
    /// writes with no game running.
    fn assert_written_given_edit_mode_closed(&self) {
        let expected = self.written_with_no_game("edit_mode_closed");
        let sandbox = self.prepare("edit_mode_closed");
        let run = self.run_in(&sandbox, own_game_runs, true);
        assert_eq!(run.exit_code(), 0, "{}: {:?}", self.name, run.messages());
        assert_eq!(self.written(&sandbox), expected, "{}", self.name);
    }

    /// With only a PES of another version running, the command writes without the flag what
    /// it writes with no game running.
    fn assert_written_while_another_version_runs(&self) {
        let expected = self.written_with_no_game("another_version");
        let sandbox = self.prepare("another_version");
        let run = self.run_in(&sandbox, another_version_runs, false);
        assert_eq!(run.exit_code(), 0, "{}: {:?}", self.name, run.messages());
        assert_eq!(self.written(&sandbox), expected, "{}", self.name);
    }
}
