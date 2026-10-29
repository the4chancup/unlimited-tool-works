//! The 0.0.1 bundle's two launchers run against the real binary named as the bundle names
//! it, in a sandbox shaped like the extracted bundle: `exports/` and `old_exports/` beside
//! `UTW.exe`, `data/settings.toml` making the data location portable.
#![cfg(windows)] // the bats are the Windows bundle's; 0.0.1 is Windows only

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use pes_version::PesVersion;

/// A fresh folder standing in for the extracted bundle: the binary as
/// `UTW.exe`, the two launchers, `data/` making the data location portable,
/// and `exports/`/`old_exports/` beside them.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Sandbox {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("launchers")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("data")).unwrap();
        fs::create_dir_all(root.join("exports")).unwrap();
        fs::create_dir_all(root.join("old_exports")).unwrap();
        fs::copy(
            env!("CARGO_BIN_EXE_utw"),
            root.join(format!("{}.exe", studio_core::EXE_NAME)),
        )
        .unwrap();
        let bundle = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../resources/bundle");
        for bat in ["quick_compile.bat", "quick_upgrade.bat"] {
            fs::copy(bundle.join(bat), root.join(bat)).unwrap();
        }
        // Every path a run can touch stays inside the sandbox: the game folder is the
        // sandbox's `PES/`, `savefile_path` names a path inside it too, never `auto`,
        // which reads the real Documents folder.
        fs::write(
            root.join("data/settings.toml"),
            format!(
                "[common]\npes_version = 21\npes_folder_path = '{}'\n\
                 [team-compiler]\nsavefile_path = '{}'\n",
                root.join("PES").display(),
                root.join("save").join("EDIT00000000").display()
            ),
        )
        .unwrap();
        fs::write(root.join("data/teams_list.txt"), "ID\tName\n714\t/co/\n").unwrap();
        Sandbox { root }
    }

    /// Writes the PES 21 fixture save (a real encrypted `EDIT00000000`) at `relative` and
    /// returns its path.
    fn save(&self, relative: &str) -> PathBuf {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        path
    }

    /// Writes `bytes` at `relative`, parent folders created.
    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    /// Runs `cmd /c <bat> [args]` with the bundle as the current directory and stdin null:
    /// the bat's `pause` reads end of input and returns at once.
    fn run_bat(&self, bat: &str, args: &[&Path]) -> Output {
        Command::new("cmd")
            .arg("/c")
            .arg(self.root.join(bat))
            .args(args)
            .current_dir(&self.root)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }
}

/// The Export upgrader's fixture old export `name` (`co Spring`, `co Spring.zip`).
fn upgrader_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tools/export_upgrader/tests/fixtures")
        .join(name)
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

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// With no file dropped the launcher prints how to drop the savefile, writes nothing and
/// exits 2.
#[test]
fn upgrade_without_a_dropped_file_says_how_to_drop_and_writes_nothing() {
    let sandbox = Sandbox::new("upgrade_no_drop");

    let output = sandbox.run_bat("quick_upgrade.bat", &[]);

    let stdout = stdout(&output);
    for line in [
        "To upgrade the exports in old_exports, drag your PES savefile onto quick_upgrade.bat.",
        "The savefile is the file named EDIT00000000, in the game's save folder under Documents\\KONAMI.",
        "Nothing was upgraded.",
    ] {
        assert!(stdout.contains(line), "{line}: {stdout}");
    }
    assert_eq!(output.status.code(), Some(2), "{stdout}");
    assert_eq!(
        fs::read_dir(sandbox.root.join("exports")).unwrap().count(),
        0,
        "exports/ stays empty: {stdout}"
    );
}

/// Every folder and archive in `old_exports/` is upgraded into `exports/` under the name
/// the upgrader gives it; the tally line counts both. One input's name holds spaces and
/// parentheses, as a real `co - Spring (VGL26)` does, so the launcher's quoting is tested.
#[test]
fn upgrade_upgrades_every_old_export_and_tallies() {
    let sandbox = Sandbox::new("upgrade_two");
    copy_dir(
        &upgrader_fixture("co Spring"),
        &sandbox.root.join("old_exports/co - Spring (VGL26)"),
    );
    fs::copy(
        upgrader_fixture("co Spring.zip"),
        sandbox.root.join("old_exports/co Spring.zip"),
    )
    .unwrap();
    let save = sandbox.save("save/EDIT00000000");

    let output = sandbox.run_bat("quick_upgrade.bat", &[save.as_path()]);

    let stdout = stdout(&output);
    for name in ["co - Spring (VGL26)", "co Spring.zip"] {
        assert!(stdout.contains(&format!("Upgrading {name}")), "{stdout}");
    }
    assert!(
        sandbox
            .root
            .join("exports/co Full - Spring (VGL26)")
            .is_dir(),
        "{stdout}"
    );
    assert!(
        sandbox.root.join("exports/co Full Spring").is_dir(),
        "{stdout}"
    );
    assert!(stdout.contains("Upgraded: 2. Not upgraded: 0."), "{stdout}");
    assert_eq!(output.status.code(), Some(0), "{stdout}");
}

/// An export whose output folder is already in `exports/` is refused and counted as not
/// upgraded; the rest still upgrade and the existing folder keeps its contents.
#[test]
fn upgrade_counts_a_refused_export_and_leaves_the_existing_folder() {
    let sandbox = Sandbox::new("upgrade_refused");
    copy_dir(
        &upgrader_fixture("co Spring"),
        &sandbox.root.join("old_exports/co Spring"),
    );
    fs::copy(
        upgrader_fixture("co Spring.zip"),
        sandbox.root.join("old_exports/day5.zip"),
    )
    .unwrap();
    let existing = sandbox.root.join("exports/co Full Spring");
    fs::create_dir_all(&existing).unwrap();
    fs::write(existing.join("mine.txt"), b"edited since").unwrap();
    let save = sandbox.save("save/EDIT00000000");

    let output = sandbox.run_bat("quick_upgrade.bat", &[save.as_path()]);

    let stdout = stdout(&output);
    assert!(stdout.contains("Upgraded: 1. Not upgraded: 1."), "{stdout}");
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(
        sandbox.root.join("exports/co Full day5").is_dir(),
        "{stdout}"
    );
    assert_eq!(
        fs::read(existing.join("mine.txt")).unwrap(),
        b"edited since"
    );
}

/// With nothing in `old_exports/` the launcher says there is no export and exits 1.
#[test]
fn upgrade_over_an_empty_old_exports_says_there_is_no_export() {
    let sandbox = Sandbox::new("upgrade_empty");
    let save = sandbox.save("save/EDIT00000000");

    let output = sandbox.run_bat("quick_upgrade.bat", &[save.as_path()]);

    let stdout = stdout(&output);
    assert!(
        stdout.contains(
            "There is no export in old_exports. Put the old exports there (folders, .zip \
             or .7z files), then drag the savefile onto quick_upgrade.bat again."
        ),
        "{stdout}"
    );
    assert_eq!(output.status.code(), Some(1), "{stdout}");
}

/// `quick_compile.bat` compiles what `exports/` holds and deploys into the PES folder the
/// settings name: the CPK lands in the fake install's `download/`.
#[test]
fn compile_compiles_the_exports_and_deploys_into_the_pes_folder() {
    let sandbox = Sandbox::new("compile");
    // The fake PES 21 install: the exe and an `download/` as `upgrade-dpfl` leaves it, the
    // official list with a placeholder CPK for every name but the run's own.
    let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../resources/templates");
    sandbox.write("PES/PES2021.exe", b"the game");
    sandbox.write(
        "PES/download/DpFileList.bin",
        &fs::read(templates.join("DpFileList.bin")).unwrap(),
    );
    let placeholder = fs::read(templates.join("placeholder.cpk")).unwrap();
    for name in fs::read_to_string(templates.join("DpFileList.txt"))
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter(|name| *name != "4cc_99_test.cpk")
    {
        sandbox.write(&format!("PES/download/{name}"), &placeholder);
    }
    // A minimal team export, as `team_compiler_cli.rs`'s tests build one: one kit of the
    // tracer bullet's export under a name resolving to /co/ (714).
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tools/team_compiler/tests/fixtures/tracer/studio/egg Midcup Tracer/Kits/g1"),
        &sandbox.root.join("exports/co Midcup Kit/Kits/p1"),
    );

    let output = sandbox.run_bat("quick_compile.bat", &[]);

    let stdout = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    let installed = sandbox.root.join("PES/download/4cc_99_test.cpk");
    assert!(installed.is_file(), "{stdout}");
    assert_ne!(
        fs::read(&installed).unwrap(),
        placeholder,
        "the installed CPK is the run's own, not a placeholder"
    );
}
