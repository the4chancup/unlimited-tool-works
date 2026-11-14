//! `upgrade-dpfl` (`spec/team_compiler/pipeline.md` "DpFileList upgrade"): what it lists
//! without `--yes`, and what it renames, writes and replaces in the game's `download/` with it.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::bins::install_names;
use crate::common::{Run, Sandbox};
use crate::deploy::{official_names, templates_folder};
use crate::snapshot;

/// The PES 2017 install's `DpFileList.bin` (`examples/DpFileList.bin`): 39 entries, the
/// faces/uniform layout from before the `teams` runs.
pub(crate) fn pes17_list() -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/DpFileList.bin"))
        .unwrap()
}

/// The official `DpFileList.bin`.
fn official_list() -> Vec<u8> {
    fs::read(templates_folder().join("DpFileList.bin")).unwrap()
}

/// The 6,272-byte placeholder CPK.
fn placeholder() -> Vec<u8> {
    fs::read(templates_folder().join("placeholder.cpk")).unwrap()
}

/// A 1 KiB file of its own bytes: `name`'s bytes, repeated.
fn kib_of(name: &str) -> Vec<u8> {
    name.bytes().cycle().take(1024).collect()
}

/// TC-DEP-09's install: the PES 2017 list, and its faces CPK alone in `download/`.
fn install_pes17_with_faces(sandbox: &Sandbox) {
    sandbox.write("PES/download/DpFileList.bin", &pes17_list());
    sandbox.write("PES/download/4cc_40_faces.cpk", &kib_of("4cc_40_faces.cpk"));
}

/// `upgrade-dpfl` in `sandbox`, with `--yes` when `yes`.
fn upgrade(sandbox: &Sandbox, yes: bool) -> Run {
    let args: &[&str] = if yes {
        &["upgrade-dpfl", "--yes"]
    } else {
        &["upgrade-dpfl"]
    };
    sandbox.run("", args)
}

/// The files of `sandbox`'s `download/`, by name, with their bytes.
fn download(sandbox: &Sandbox) -> BTreeMap<PathBuf, Vec<u8>> {
    snapshot(&sandbox.root.join("PES/download"))
}

fn renamed(from: &str, to: &str) -> String {
    format!("Info dpfilelist_cpk_renamed [Keep] (from={from}, to={to})")
}

/// A placeholder line for each official entry but `taken`, in the official order.
fn placeholders_but(taken: &[&str]) -> Vec<String> {
    official_names()
        .into_iter()
        .filter(|name| !taken.contains(&name.as_str()))
        .map(|name| format!("Info dpfilelist_placeholder_written [Keep] (cpk={name})"))
        .collect()
}

/// The line of the list's replacement in `sandbox`, the old one kept as the backup.
fn replaced(sandbox: &Sandbox) -> String {
    format!(
        "Info dpfilelist_replaced [Keep] (path={}, backup={})",
        sandbox.display("PES/download/DpFileList.bin"),
        sandbox.display("PES/download/DpFileList.bin.bak")
    )
}

fn dropped(cpk: &str) -> String {
    format!("Warning dpfilelist_cpk_dropped [Keep] (cpk={cpk})")
}

const PLANNED: &str = concat!(
    "Info dpfilelist_upgrade_planned [Keep] (command=",
    studio_core::exe_name!(),
    " compiler upgrade-dpfl --yes)"
);

/// The PES 2017 list's entries the official list lacks, but `4cc_40_faces.cpk`, in its order:
/// what TC-DEP-09's install drops, none of them with a file.
const PES17_DROPPED: [&str; 17] = [
    "4cc_15_billboard.cpk",
    "4cc_20_swipe.cpk",
    "4cc_25_gametips.cpk",
    "4cc_30_stadiums0.cpk",
    "4cc_31_stadiums1.cpk",
    "4cc_32_stadiums2.cpk",
    "4cc_35_referees.cpk",
    "4cc_38_balls.cpk",
    "4cc_45_uniform.cpk",
    "4cc_50_other_faces.cpk",
    "4cc_55_other_uniform.cpk",
    "4cc_60_midcup.cpk",
    "4cc_76_midcup.cpk",
    "4cc_77_midcup.cpk",
    "4cc_78_midcup.cpk",
    "4cc_79_midcup.cpk",
    "4cc_90_test.cpk",
];

/// Every line of `upgrade-dpfl` on TC-DEP-09's install in `sandbox`, `--yes` or not, but the
/// last line without it.
fn pes17_lines(sandbox: &Sandbox) -> Vec<String> {
    let mut lines = vec![renamed("4cc_40_faces.cpk", "4cc_41_teams.cpk")];
    lines.extend(placeholders_but(&["4cc_41_teams.cpk"]));
    lines.push(replaced(sandbox));
    lines.extend(PES17_DROPPED.map(dropped));
    lines
}

/// Asserts every official entry in `sandbox`'s `download/` but `taken` is the placeholder.
fn assert_placeholders(sandbox: &Sandbox, taken: &[&str]) {
    let placeholder = placeholder();
    for name in official_names() {
        if taken.contains(&name.as_str()) {
            continue;
        }
        let bytes = fs::read(sandbox.root.join("PES/download").join(&name)).unwrap();
        assert!(bytes == placeholder, "{name} is not the placeholder");
    }
}

// TC-DEP-09
#[test]
fn without_yes_the_rename_is_listed_and_nothing_is_written() {
    let sandbox = Sandbox::new("upgrade_planned");
    install_pes17_with_faces(&sandbox);
    let before = download(&sandbox);

    let run = upgrade(&sandbox, false);

    let mut lines = pes17_lines(&sandbox);
    lines.push(PLANNED.to_owned());
    assert_eq!(run.messages(), lines);
    assert_eq!(run.exit_code(), 0);
    assert!(download(&sandbox) == before, "nothing renamed or written");
}

// TC-DEP-10
#[test]
fn with_yes_the_cpk_is_renamed_the_slots_filled_and_the_list_replaced_with_a_backup() {
    let sandbox = Sandbox::new("upgrade_applied");
    install_pes17_with_faces(&sandbox);

    let run = upgrade(&sandbox, true);

    assert_eq!(run.messages(), pes17_lines(&sandbox));
    assert_eq!(run.exit_code(), 0);
    let files = download(&sandbox);
    assert!(files[Path::new("DpFileList.bin")] == official_list());
    assert!(files[Path::new("DpFileList.bin.bak")] == pes17_list());
    assert!(files[Path::new("4cc_41_teams.cpk")] == kib_of("4cc_40_faces.cpk"));
    assert!(!files.contains_key(Path::new("4cc_40_faces.cpk")));
    assert_placeholders(&sandbox, &["4cc_41_teams.cpk"]);
    // The official list's 53 CPKs, the list and its backup: nothing else.
    assert_eq!(files.len(), 55);
}

// TC-DEP-13
#[test]
fn an_old_dlc_s_cpks_take_their_stems_official_names_in_list_order() {
    let sandbox = Sandbox::new("upgrade_stems");
    let names = [
        "4cc_38_balls.cpk",
        "4cc_40_faces.cpk",
        "4cc_45_uniform.cpk",
        "4cc_60_midcup.cpk",
        "4cc_61_midcup.cpk",
        "4cc_86_mine.cpk",
    ];
    install_names(&sandbox, &names);
    for name in names {
        sandbox.write(&format!("PES/download/{name}"), &kib_of(name));
    }

    let run = upgrade(&sandbox, true);

    let taken = [
        "4cc_16_balls.cpk",
        "4cc_41_teams.cpk",
        "4cc_42_teams.cpk",
        "4cc_61_midcup.cpk",
        "4cc_62_midcup.cpk",
    ];
    let mut lines = vec![
        renamed("4cc_38_balls.cpk", "4cc_16_balls.cpk"),
        renamed("4cc_45_uniform.cpk", "4cc_42_teams.cpk"),
        renamed("4cc_40_faces.cpk", "4cc_41_teams.cpk"),
        renamed("4cc_61_midcup.cpk", "4cc_62_midcup.cpk"),
        renamed("4cc_60_midcup.cpk", "4cc_61_midcup.cpk"),
    ];
    let placeholders = placeholders_but(&taken);
    assert_eq!(placeholders.len(), 48);
    lines.extend(placeholders);
    lines.push(replaced(&sandbox));
    lines
        .push("Warning dpfilelist_cpk_dropped [Keep] (cpk=4cc_86_mine.cpk, size=1 KiB)".to_owned());
    assert_eq!(run.messages(), lines);
    assert_eq!(run.exit_code(), 0);
    let files = download(&sandbox);
    for (name, old) in [
        ("4cc_16_balls.cpk", "4cc_38_balls.cpk"),
        ("4cc_41_teams.cpk", "4cc_40_faces.cpk"),
        ("4cc_42_teams.cpk", "4cc_45_uniform.cpk"),
        ("4cc_61_midcup.cpk", "4cc_60_midcup.cpk"),
        ("4cc_62_midcup.cpk", "4cc_61_midcup.cpk"),
        ("4cc_86_mine.cpk", "4cc_86_mine.cpk"),
    ] {
        assert!(
            files[Path::new(name)] == kib_of(old),
            "{name} holds {old}'s bytes"
        );
    }
    for gone in [
        "4cc_38_balls.cpk",
        "4cc_40_faces.cpk",
        "4cc_45_uniform.cpk",
        "4cc_60_midcup.cpk",
    ] {
        assert!(!files.contains_key(Path::new(gone)), "{gone} is renamed");
    }
    assert_placeholders(&sandbox, &taken);
}

// TC-DEP-15
#[test]
fn a_second_run_finds_the_list_up_to_date_and_writes_nothing() {
    let sandbox = Sandbox::new("upgrade_twice");
    install_pes17_with_faces(&sandbox);
    assert_eq!(upgrade(&sandbox, true).exit_code(), 0);
    let before = download(&sandbox);

    let run = upgrade(&sandbox, true);

    assert_eq!(
        run.messages(),
        [format!(
            "Info dpfilelist_up_to_date [Keep] (path={})",
            sandbox.display("PES/download/DpFileList.bin")
        )]
    );
    assert_eq!(run.exit_code(), 0);
    assert!(download(&sandbox) == before, "nothing written");
    assert!(before[Path::new("DpFileList.bin.bak")] == pes17_list());
}

// TC-DEP-10
#[test]
fn with_no_list_the_official_one_is_written_and_nothing_backed_up() {
    let sandbox = Sandbox::new("upgrade_no_list");
    fs::create_dir_all(sandbox.root.join("PES/download")).unwrap();

    let run = upgrade(&sandbox, true);

    let mut lines = placeholders_but(&[]);
    lines.push(format!(
        "Info dpfilelist_replaced [Keep] (path={})",
        sandbox.display("PES/download/DpFileList.bin")
    ));
    assert_eq!(run.messages(), lines);
    assert_eq!(run.exit_code(), 0);
    let files = download(&sandbox);
    assert!(files[Path::new("DpFileList.bin")] == official_list());
    assert!(!files.contains_key(Path::new("DpFileList.bin.bak")));
    assert_placeholders(&sandbox, &[]);
}

// TC-DEP-10
#[test]
fn a_list_that_does_not_read_is_replaced_and_backed_up_with_nothing_renamed() {
    let sandbox = Sandbox::new("upgrade_unreadable_list");
    sandbox.write("PES/download/DpFileList.bin", &[0; 15]);
    sandbox.write("PES/download/4cc_40_faces.cpk", &kib_of("4cc_40_faces.cpk"));

    let run = upgrade(&sandbox, true);

    let mut lines = placeholders_but(&[]);
    lines.push(format!(
        "Info dpfilelist_replaced [Keep] (path={}, backup={}, unreadable=the list is 15 bytes, \
         shorter than its 16-byte header)",
        sandbox.display("PES/download/DpFileList.bin"),
        sandbox.display("PES/download/DpFileList.bin.bak")
    ));
    assert_eq!(run.messages(), lines);
    assert_eq!(run.exit_code(), 0);
    let files = download(&sandbox);
    assert!(files[Path::new("DpFileList.bin")] == official_list());
    assert_eq!(files[Path::new("DpFileList.bin.bak")], [0; 15]);
    assert!(files[Path::new("4cc_40_faces.cpk")] == kib_of("4cc_40_faces.cpk"));
}

/// Writes the placeholder for each official entry but `except` in `sandbox`'s `download/`.
fn fill_official_slots(sandbox: &Sandbox, except: &str) {
    for name in official_names() {
        if name != except {
            sandbox.write(&format!("PES/download/{name}"), &placeholder());
        }
    }
}

// TC-DEP-15
#[test]
fn an_official_list_with_a_slot_empty_gets_its_placeholder_and_is_not_rewritten() {
    let sandbox = Sandbox::new("upgrade_official_slot_empty");
    sandbox.write("PES/download/DpFileList.bin", &official_list());
    fill_official_slots(&sandbox, "4cc_62_midcup.cpk");

    let run = upgrade(&sandbox, true);

    assert_eq!(
        run.messages(),
        ["Info dpfilelist_placeholder_written [Keep] (cpk=4cc_62_midcup.cpk)"]
    );
    assert_eq!(run.exit_code(), 0);
    let files = download(&sandbox);
    assert!(files[Path::new("4cc_62_midcup.cpk")] == placeholder());
    assert!(!files.contains_key(Path::new("DpFileList.bin.bak")));
}

// TC-DEP-15
#[test]
fn a_list_with_an_entry_of_its_own_and_every_slot_filled_is_still_replaced() {
    let sandbox = Sandbox::new("upgrade_own_entry");
    let mut names = official_names();
    names.push("4cc_80_mine.cpk".to_owned());
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    install_names(&sandbox, &names);
    let old_list = fs::read(sandbox.root.join("PES/download/DpFileList.bin")).unwrap();
    fill_official_slots(&sandbox, "");
    sandbox.write("PES/download/4cc_80_mine.cpk", &kib_of("4cc_80_mine.cpk"));

    let run = upgrade(&sandbox, true);

    assert_eq!(
        run.messages(),
        [
            replaced(&sandbox),
            "Warning dpfilelist_cpk_dropped [Keep] (cpk=4cc_80_mine.cpk, size=1 KiB)".to_owned()
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let files = download(&sandbox);
    assert!(files[Path::new("DpFileList.bin")] == official_list());
    assert!(files[Path::new("DpFileList.bin.bak")] == old_list);
    assert!(files[Path::new("4cc_80_mine.cpk")] == kib_of("4cc_80_mine.cpk"));
}

// TC-DEP-16
#[test]
fn a_list_that_cannot_be_read_stops_the_upgrade_naming_it() {
    let sandbox = Sandbox::new("upgrade_list_not_a_file");
    // Reading a folder fails on every system, and not as a missing file.
    fs::create_dir_all(sandbox.root.join("PES/download/DpFileList.bin")).unwrap();

    let run = upgrade(&sandbox, true);

    run.assert_refused(3, &[&sandbox.display("PES/download/DpFileList.bin")]);
    assert_eq!(
        download(&sandbox).len(),
        0,
        "nothing written, the list's folder aside"
    );
}

// TC-DEP-16
#[test]
fn a_missing_pes_folder_or_download_folder_is_refused_naming_it() {
    let sandbox = Sandbox::new("upgrade_no_folder");
    let run = upgrade(&sandbox, true);
    run.assert_refused(2, &[&sandbox.display("PES"), "pes_folder_path"]);

    fs::create_dir_all(sandbox.root.join("PES")).unwrap();
    let run = upgrade(&sandbox, true);
    run.assert_refused(2, &[&sandbox.display("PES/download"), "pes_folder_path"]);
    assert!(!sandbox.root.join("PES/download").exists(), "not created");
}

#[cfg(windows)]
// TC-DEP-16
#[test]
fn a_cpk_held_open_stops_the_upgrade_naming_it_and_the_list_stays() {
    // Windows only: Linux renames a file another process holds open.
    use std::os::windows::fs::OpenOptionsExt;
    /// `FILE_SHARE_READ` alone, as PES holds its CPKs: read, not deleted or renamed.
    const FILE_SHARE_READ: u32 = 1;

    let sandbox = Sandbox::new("upgrade_held_open");
    install_pes17_with_faces(&sandbox);
    let before = download(&sandbox);
    let held_open = fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(sandbox.root.join("PES/download/4cc_40_faces.cpk"))
        .unwrap();

    let run = upgrade(&sandbox, true);

    drop(held_open);
    run.assert_refused(3, &[&sandbox.display("PES/download/4cc_40_faces.cpk")]);
    assert!(
        download(&sandbox) == before,
        "nothing renamed, written or replaced"
    );
}
