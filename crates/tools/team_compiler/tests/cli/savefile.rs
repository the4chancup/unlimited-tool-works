//! The savefile update (`spec/team_compiler/pipeline.md` "6. Post-processing"): a deployed or
//! sideloaded run applies the `aesthetics_patch.toml` it just wrote to the local
//! savefile (`pes_savefile`'s `apply`), skipped while the target's PES runs unless the run
//! was given `--edit-mode-closed`, and never when the game loads nothing of the run.

use std::fs;
use std::path::PathBuf;

use pes_savefile::model::player::STRIPPED_APPEARANCE_ID;
use pes_savefile::model::team::RosterSlot;
use pes_savefile::{EditFile, fixture_save};
use pes_version::PesVersion;
use studio_core::RunningPes;

use crate::common::Sandbox;
use crate::compile::{pes_settings, pes21_settings};
use crate::deploy::install_pes;
use crate::patch::{pes17_crocs_export, read_patch, write_names_export};

/// The salt the tests' own writes use; a save's bytes depend on it, its payload does not.
const TEST_SALT: [u8; 320] = [7; 320];

/// Writes `version`'s fixture save with every player stripped at the sandbox's
/// `savefile_path` (`save/EDIT00000000`, which `sandboxed` names), and returns its path.
pub(crate) fn install_save(sandbox: &Sandbox, version: PesVersion) -> PathBuf {
    let mut file = EditFile::from_bytes(&fixture_save(version)).unwrap();
    for player in file.players_mut() {
        player.appearance_id = Some(STRIPPED_APPEARANCE_ID);
    }
    let path = sandbox.root.join("save").join("EDIT00000000");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, file.to_bytes(&TEST_SALT).unwrap()).unwrap();
    path
}

/// A `settings.toml` holding one motion key, for `/xx/ Full Name`'s slot 05.
fn motion_export(sandbox: &Sandbox, export: &str) {
    sandbox.write(
        &format!("exports/{export}/Players/05 - A/settings.toml"),
        b"[appearance.motion]\nhunching_dribbling = 2\n",
    );
}

// TC-SAV-01
// TC-SAV-18
#[test]
fn a_deployed_run_applies_its_patch_to_the_savefile_and_keeps_a_bak() {
    let sandbox = Sandbox::new("savefile_applied");
    install_pes(&sandbox);
    let save = install_save(&sandbox, PesVersion::Pes21);
    sandbox.write("PES/download/4cc_99_test.cpk", b"the old CPK");
    // Team 714's roster with 71405 in slot 06 and 71406 in slot 05: the patch's players
    // find the save's by id, whatever the roster order.
    let mut file = EditFile::load(&save).unwrap();
    let place = |roster: &mut [RosterSlot], id: u32, at: usize| {
        let from = roster
            .iter()
            .position(|slot| slot.player_id == id)
            .unwrap_or_else(|| panic!("{id} is in team 714's roster"));
        roster.swap(from, at);
    };
    {
        let roster = &mut file.team_mut(714).unwrap().roster;
        place(roster, 71406, 4);
        place(roster, 71405, 5);
        assert_eq!(roster[4].player_id, 71406);
        assert_eq!(roster[5].player_id, 71405);
    }
    assert_ne!(
        file.player(71405).unwrap().motion.hunching_dribbling,
        1,
        "the premise: the save does not already hold the export's value (2, stored 1)"
    );
    let before_bytes = file.to_bytes(&TEST_SALT).unwrap();
    fs::write(&save, &before_bytes).unwrap();
    motion_export(&sandbox, "co Full Motions");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let written = format!(
        "Info savefile_written [Keep] (path={}, teams=714)",
        save.display()
    );
    assert!(lines.contains(&written), "{lines:#?}");
    // The save is the pre-run one with the run's own patch applied through the lib: 71405's
    // look and motions are his patch entry's, the record made his, 71406 and every other
    // byte unchanged.
    let mut expected = EditFile::from_bytes(&before_bytes).unwrap();
    read_patch(&sandbox).apply(&mut expected).unwrap();
    let after = EditFile::load(&save).unwrap();
    assert_eq!(
        after.to_bytes(&TEST_SALT).unwrap(),
        expected.to_bytes(&TEST_SALT).unwrap()
    );
    let player = after.player(71405).unwrap();
    // A motion is stored one below its settings.toml value (1 to 5 stored 0 to 4).
    assert_eq!(
        player.motion.hunching_dribbling, 1,
        "the settings.toml's 2, stored"
    );
    // The stripped save's record is made his: appearance id and base copy his own id.
    assert_eq!(player.appearance_id, Some(71405));
    assert_eq!(player.appearance.base_copy_id, 71405);
    assert_eq!(player.name, file.player(71405).unwrap().name);
    assert_eq!(after.player(71406), file.player(71406), "71406 unchanged");
    assert_eq!(
        fs::read(sandbox.root.join("save").join("EDIT00000000.bak")).unwrap(),
        before_bytes,
        "the previous file is the .bak"
    );
    // The committed deployment: the new CPK in `download/`, the old one not kept aside.
    assert_ne!(
        fs::read(sandbox.root.join("PES/download/4cc_99_test.cpk")).unwrap(),
        b"the old CPK",
        "the run's CPK installed"
    );
    assert_eq!(
        leftover_files(&sandbox, "PES/download"),
        Vec::<String>::new(),
        "no .old left"
    );
}

// The savefile half of the write_names run (the patch half is `patch.rs`'s).
// TC-SAV-02
#[test]
fn a_write_names_export_s_names_reach_the_savefile() {
    let sandbox = Sandbox::new("savefile_write_names");
    install_pes(&sandbox);
    install_save(&sandbox, PesVersion::Pes21);
    write_names_export(
        &sandbox,
        "exports/co Full Auto",
        "name = true\nshirt_name = \"SNUFFY\"\n",
        "name = \"Shared\"\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let save = EditFile::load(&sandbox.root.join("save").join("EDIT00000000")).unwrap();
    let snuffy = save.player(71405).unwrap();
    assert_eq!(snuffy.name, "Snuffy", "name = true's");
    assert_eq!(snuffy.shirt_name, "SNUFFY");
    for id in [71406, 71408] {
        assert_eq!(save.player(id).unwrap().name, "Shared", "{id}");
    }
}

// The savefile half (the patch half is `patch.rs`'s): the step ran (`savefile_written`),
// but the name the field refused was never applied.
// TC-SAV-03
#[test]
fn a_name_the_field_refuses_keeps_the_save_s_name() {
    let sandbox = Sandbox::new("savefile_name_invalid");
    install_pes(&sandbox);
    let save = install_save(&sandbox, PesVersion::Pes21);
    let before = EditFile::load(&save).unwrap();
    let export = "exports/co Full Auto";
    sandbox.write(&format!("{export}/write_names"), b"");
    // Longer than the PES 21 name field (61 bytes).
    sandbox.write(
        &format!("{export}/Players/05 - A/settings.toml"),
        format!("name = \"{}\"\n", "a".repeat(70)).as_bytes(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    // The step ran and applied what it could (his motions): the name alone was refused.
    let written = format!(
        "Info savefile_written [Keep] (path={}, teams=714)",
        save.display()
    );
    assert!(lines.contains(&written), "{lines:#?}");
    let after = EditFile::load(&save).unwrap();
    assert_eq!(
        after.player(71405).unwrap().name,
        before.player(71405).unwrap().name,
        "the savefile keeps his name"
    );
}

// TC-SAV-04
#[test]
fn a_savefile_path_naming_a_missing_file_is_a_warning_and_the_patch_is_written() {
    let sandbox = Sandbox::new("savefile_missing");
    install_pes(&sandbox);
    motion_export(&sandbox, "co Full Motions");
    let missing = sandbox.root.join("save/MISSING");
    let settings = format!(
        "{}[team-compiler]\nsavefile_path = '{}'\n",
        pes21_settings(&sandbox),
        missing.display()
    );

    let run = sandbox.run(&settings, &["compile"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    for expected in [
        format!(
            "Info patch_written [Keep] (path={}, teams=714)",
            sandbox.display("output/aesthetics_patch.toml")
        ),
        format!(
            "Warning savefile_missing [Keep] (patch={}, path={}, command={} save-editor apply-patch)",
            sandbox.display("output/aesthetics_patch.toml"),
            missing.display(),
            studio_core::EXE_NAME
        ),
    ] {
        assert!(lines.contains(&expected), "{expected}: {lines:#?}");
    }
}

// The second half of the scenario: a team the save lacks is an error, the rest is applied.
// TC-SAV-05
#[test]
fn a_team_the_save_lacks_is_an_error_and_the_rest_is_written() {
    let sandbox = Sandbox::new("savefile_team_missing");
    install_pes(&sandbox);
    let save = install_save(&sandbox, PesVersion::Pes21);
    // The save holds 792 and not 714: team 714's entry renumbered to an id the save lacks.
    let mut file = EditFile::load(&save).unwrap();
    let free = (1..)
        .find(|id| file.team(*id).is_none())
        .expect("an id no team holds");
    file.team_mut(714).unwrap().id = free;
    let before_bytes = file.to_bytes(&TEST_SALT).unwrap();
    fs::write(&save, &before_bytes).unwrap();
    for export in ["co Full Co", "egg Full Egg"] {
        motion_export(&sandbox, export);
    }

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    let missing = "Error savefile_team_missing [Keep] (team=714)".to_owned();
    assert!(lines.contains(&missing), "{lines:#?}");
    // The save is the pre-run one with the patch applied through the lib: 792's entries
    // applied, 714's skipped, nothing else changed.
    let mut expected = EditFile::from_bytes(&before_bytes).unwrap();
    read_patch(&sandbox).apply(&mut expected).unwrap();
    assert_ne!(
        expected.to_bytes(&TEST_SALT).unwrap(),
        before_bytes,
        "792's entries applied"
    );
    let after = EditFile::load(&save).unwrap();
    assert_eq!(
        after.to_bytes(&TEST_SALT).unwrap(),
        expected.to_bytes(&TEST_SALT).unwrap()
    );
}

/// The files of `relative` (a folder of the sandbox) whose name ends in `.old` or
/// `.partial`: what a run's deployment transaction would leave behind.
fn leftover_files(sandbox: &Sandbox, relative: &str) -> Vec<String> {
    fs::read_dir(sandbox.root.join(relative))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".old") || name.ends_with(".partial"))
        .collect()
}

// The first half of the scenario: a wrong-version save aborts the run and rolls the
// deployment back.
// TC-SAV-05
#[test]
fn a_save_of_another_version_is_savefile_invalid_and_left_alone() {
    let sandbox = Sandbox::new("savefile_invalid_version");
    install_pes(&sandbox);
    let save = install_save(&sandbox, PesVersion::Pes19);
    let before = fs::read(&save).unwrap();
    sandbox.write("PES/download/4cc_99_test.cpk", b"the old CPK");
    motion_export(&sandbox, "co Full Motions");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 3, "{lines:#?}");
    let expected = format!(
        "Fatal savefile_invalid [AbortRun] (path={}, version=PES 2019, output={})",
        save.display(),
        sandbox.display("output")
    );
    assert!(lines.contains(&expected), "{lines:#?}");
    assert_eq!(fs::read(&save).unwrap(), before, "byte-identical");
    assert_eq!(
        fs::read(sandbox.root.join("PES/download/4cc_99_test.cpk")).unwrap(),
        b"the old CPK",
        "the deployment rolled back"
    );
    assert_eq!(
        leftover_files(&sandbox, "PES/download"),
        Vec::<String>::new(),
        "no .old or .partial left"
    );
    assert!(sandbox.root.join("output/4cc_99_test.cpk").is_file());
    assert!(sandbox.root.join("output/aesthetics_patch.toml").is_file());
}

// TC-SAV-07
#[test]
fn a_player_whose_appearance_id_is_set_is_reported_and_stays_set() {
    let sandbox = Sandbox::new("savefile_not_stripped");
    install_pes(&sandbox);
    let save = install_save(&sandbox, PesVersion::Pes21);
    let mut file = EditFile::load(&save).unwrap();
    file.player_mut(71405).unwrap().appearance_id = Some(71405);
    // 71406, whom the export does not compile, is not stripped either: only the patch's
    // players are checked, so he is not named.
    file.player_mut(71406).unwrap().appearance_id = Some(71406);
    fs::write(&save, file.to_bytes(&TEST_SALT).unwrap()).unwrap();
    motion_export(&sandbox, "co Full Motions");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let expected = "Warning savefile_player_not_stripped [Keep] (players=71405)".to_owned();
    assert!(lines.contains(&expected), "{lines:#?}");
    let after = EditFile::load(&save).unwrap();
    assert_eq!(after.player(71405).unwrap().appearance_id, Some(71405));
}

// TC-SAV-08
#[test]
fn a_savefile_that_cannot_be_replaced_rolls_the_deployment_back() {
    let sandbox = Sandbox::new("savefile_write_failed");
    install_pes(&sandbox);
    let save = install_save(&sandbox, PesVersion::Pes21);
    let before = fs::read(&save).unwrap();
    // `EditFile::save` copies the previous file to `<save>.bak` first: a folder of that
    // name makes the save impossible to replace.
    fs::create_dir_all(sandbox.root.join("save/EDIT00000000.bak")).unwrap();
    sandbox.write("PES/download/4cc_99_test.cpk", b"the old CPK");
    motion_export(&sandbox, "co Full Motions");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 3, "{lines:#?}");
    let prefix = format!(
        "Fatal savefile_write_failed [AbortRun] (path={}, error=",
        save.display()
    );
    let suffix = format!(", output={})", sandbox.display("output"));
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with(&prefix) && line.ends_with(&suffix)),
        "{lines:#?}"
    );
    assert_eq!(fs::read(&save).unwrap(), before, "byte-identical");
    assert_eq!(
        fs::read(sandbox.root.join("PES/download/4cc_99_test.cpk")).unwrap(),
        b"the old CPK",
        "the deployment rolled back"
    );
    assert_eq!(
        leftover_files(&sandbox, "PES/download"),
        Vec::<String>::new(),
        "no .old or .partial left"
    );
    assert!(sandbox.root.join("output/4cc_99_test.cpk").is_file());
    assert!(sandbox.root.join("output/aesthetics_patch.toml").is_file());
}

// TC-SAV-09
#[test]
fn a_patch_that_cannot_be_written_rolls_the_deployment_back() {
    let sandbox = Sandbox::new("patch_write_rolled_back");
    install_pes(&sandbox);
    let save = install_save(&sandbox, PesVersion::Pes21);
    let before = fs::read(&save).unwrap();
    // The patch's own path taken by a folder: the write cannot happen.
    fs::create_dir_all(sandbox.root.join("output/aesthetics_patch.toml")).unwrap();
    sandbox.write("PES/download/4cc_99_test.cpk", b"the old CPK");
    motion_export(&sandbox, "co Full Motions");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 3, "{lines:#?}");
    let prefix = format!(
        "Fatal patch_write_failed [AbortRun] (path={}, error=",
        sandbox.display("output/aesthetics_patch.toml")
    );
    assert!(
        lines.iter().any(|line| line.starts_with(&prefix)),
        "{lines:#?}"
    );
    assert_eq!(
        fs::read(sandbox.root.join("PES/download/4cc_99_test.cpk")).unwrap(),
        b"the old CPK",
        "the deployment rolled back"
    );
    assert_eq!(
        leftover_files(&sandbox, "PES/download"),
        Vec::<String>::new(),
        "no .old left"
    );
    assert_eq!(fs::read(&save).unwrap(), before, "byte-identical");
    assert_eq!(
        run.complete().files_written,
        0,
        "the rolled-back deployment put nothing in place"
    );
}

#[cfg(windows)]
// TC-SAV-11
#[test]
fn a_degraded_run_leaves_the_savefile_alone() {
    // Windows only: Linux renames over a file another process holds open (TC-DEP-02's setup).
    use std::os::windows::fs::OpenOptionsExt;
    /// `FILE_SHARE_READ` alone, as PES holds its CPKs: read, not deleted or replaced.
    const FILE_SHARE_READ: u32 = 1;

    let sandbox = Sandbox::new("savefile_degraded");
    install_pes(&sandbox);
    let save = install_save(&sandbox, PesVersion::Pes21);
    let before = fs::read(&save).unwrap();
    sandbox.write("PES/download/4cc_99_test.cpk", b"the old CPK");
    motion_export(&sandbox, "co Full Motions");
    let held_open = fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(sandbox.root.join("PES/download/4cc_99_test.cpk"))
        .unwrap();

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile"]);

    drop(held_open);
    let lines = run.messages();
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    // A leading space: the code field's position, which a sandbox path never matches.
    assert!(
        lines.iter().all(|line| !line.contains(" savefile_")),
        "{lines:#?}"
    );
    assert!(sandbox.root.join("output/4cc_99_test.cpk").is_file());
    assert!(sandbox.root.join("output/aesthetics_patch.toml").is_file());
    assert_eq!(fs::read(&save).unwrap(), before, "byte-identical");
}

// The savefile half (the patch half is `patch.rs`'s): a deployed PES 17 compile applies
// the patch it just wrote — his look, motions and the linked folder's boots ID, the
// record made his — and the `write_names` name with it.
// TC-SAV-19
#[test]
fn a_pre_fox_compile_writes_the_patch_and_applies_it_to_the_savefile() {
    let sandbox = Sandbox::new("savefile_prefox");
    install_pes(&sandbox);
    sandbox.write("PES/PES2017.exe", b"the game");
    let save = install_save(&sandbox, PesVersion::Pes17);
    let (export, toml) = pes17_crocs_export(&sandbox);
    // The premise: the save's team 714 holds 71405, whose boots are not the export's.
    let before = EditFile::load(&save).unwrap();
    assert!(before.team(714).is_some(), "the fixture holds team 714");
    let player05 = before.player(71405).expect("the fixture holds 71405");
    assert_ne!(player05.appearance.boots_id, 625);

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let written = format!(
        "Info savefile_written [Keep] (path={}, teams=714)",
        save.display()
    );
    assert!(lines.contains(&written), "{lines:#?}");
    // The save is the pre-run one with the run's own patch applied through the lib.
    let mut expected = EditFile::from_bytes(&before.to_bytes(&TEST_SALT).unwrap()).unwrap();
    read_patch(&sandbox).apply(&mut expected).unwrap();
    let after = EditFile::load(&save).unwrap();
    assert_eq!(
        after.to_bytes(&TEST_SALT).unwrap(),
        expected.to_bytes(&TEST_SALT).unwrap()
    );
    let player = after.player(71405).unwrap();
    assert_eq!(player.appearance.boots_id, 625, "the linked folder's ID");
    // A motion is stored one below its settings.toml value (1 to 5 stored 0 to 4).
    assert_eq!(
        player.motion.hunching_dribbling, 1,
        "the settings.toml's 2, stored"
    );
    // The record is made his: appearance id and base copy his own id,
    // the base-copy flag clear.
    assert_eq!(player.appearance_id, Some(71405));
    assert_eq!(player.appearance.base_copy_id, 71405);
    assert_eq!(player.edit_flags.base_copy, Some(false));
    assert_eq!(
        player.name, player05.name,
        "no marker, the save's name kept"
    );

    // The `write_names` half: the marker and `name = "Snuffy"` put his name in the
    // entry and the save alike.
    sandbox.write(&format!("{export}/write_names"), b"");
    sandbox.write(
        &format!("{export}/Players/05 - A/settings.toml"),
        format!("name = \"Snuffy\"\n{toml}").as_bytes(),
    );

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    assert_eq!(
        read_patch(&sandbox).teams[0].players[&5].name.as_deref(),
        Some("Snuffy")
    );
    assert_eq!(
        EditFile::load(&save).unwrap().player(71405).unwrap().name,
        "Snuffy"
    );
}

// TC-SAV-17's no-game case: nothing running, the sideloaded run writes the save.
// TC-SAV-17
#[test]
fn a_sideload_run_writes_the_savefile_and_a_patch_naming_no_cpk() {
    let sandbox = Sandbox::new("savefile_sideload");
    sandbox.write("PES/PES2021.exe", b"the game");
    install_save(&sandbox, PesVersion::Pes21);
    motion_export(&sandbox, "co Full Motions");

    let run = sandbox.run(
        &pes21_settings(&sandbox),
        &["compile", "--mode", "sideload"],
    );

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let written = format!(
        "Info savefile_written [Keep] (path={}, teams=714)",
        sandbox.display("save/EDIT00000000")
    );
    assert!(lines.contains(&written), "{lines:#?}");
    assert_eq!(read_patch(&sandbox).cpks, Vec::<String>::new());
}

// The scenario's running-game cases, three runs in sequence on one sandbox: skipped while
// the target's PES runs, written with the flag, written when only another version runs.
// TC-SAV-17
#[test]
fn a_sideload_run_with_its_game_running_writes_the_savefile_only_given_edit_mode_closed() {
    fn pes21_running() -> Vec<RunningPes> {
        vec![RunningPes {
            version: PesVersion::Pes21,
            exe: None,
        }]
    }
    fn pes19_running() -> Vec<RunningPes> {
        vec![RunningPes {
            version: PesVersion::Pes19,
            exe: None,
        }]
    }

    let sandbox = Sandbox::new("savefile_sideload_running");
    sandbox.write("PES/PES2021.exe", b"the game");
    let save = install_save(&sandbox, PesVersion::Pes21);
    let before = fs::read(&save).unwrap();
    motion_export(&sandbox, "co Full Motions");
    let sideload = ["compile", "--mode", "sideload"];
    let written = format!(
        "Info savefile_written [Keep] (path={}, teams=714)",
        sandbox.display("save/EDIT00000000")
    );

    // Its own game running, no flag: the save is skipped, the patch written naming no CPK.
    let run = sandbox.run_with_running(&pes21_settings(&sandbox), &sideload, pes21_running);
    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let skipped = format!(
        "Warning savefile_skipped_pes_running [Keep] (path={}, version=PES 2021, flag=--edit-mode-closed)",
        sandbox.display("save/EDIT00000000")
    );
    assert!(lines.contains(&skipped), "{lines:#?}");
    assert!(
        lines.iter().all(|line| !line.contains("savefile_written")),
        "{lines:#?}"
    );
    assert_eq!(read_patch(&sandbox).cpks, Vec::<String>::new());
    assert_eq!(fs::read(&save).unwrap(), before, "byte-identical");

    // The flag is the member's word that Edit mode is closed: the save is written.
    let mut flagged = sideload.to_vec();
    flagged.push("--edit-mode-closed");
    let run = sandbox.run_with_running(&pes21_settings(&sandbox), &flagged, pes21_running);
    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(lines.contains(&written), "{lines:#?}");
    assert_ne!(fs::read(&save).unwrap(), before, "the save is written");

    // Another version's game never writes this save back: written without the flag.
    let run = sandbox.run_with_running(&pes21_settings(&sandbox), &sideload, pes19_running);
    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(lines.contains(&written), "{lines:#?}");
}

// The other modes the game loads nothing of (`spec/team_compiler/pipeline.md` "6.
// Post-processing"): --no-deploy and test mode.
// TC-SAV-16
#[test]
fn a_no_deploy_and_a_test_mode_run_leave_the_savefile_alone() {
    let sandbox = Sandbox::new("savefile_not_installed");
    sandbox.write("PES/PES2021.exe", b"the game");
    let save = install_save(&sandbox, PesVersion::Pes21);
    let before = fs::read(&save).unwrap();
    motion_export(&sandbox, "co Full Motions");

    for args in [
        vec!["compile", "--no-deploy"],
        vec!["compile", "--mode", "test"],
    ] {
        let run = sandbox.run(&pes21_settings(&sandbox), &args);
        let lines = run.messages();
        assert_eq!(run.exit_code(), 0, "{args:?}: {lines:#?}");
        assert!(
            lines.iter().all(|line| !line.contains(" savefile_")),
            "{args:?}: {lines:#?}"
        );
        assert_eq!(fs::read(&save).unwrap(), before, "{args:?}");
    }
}

// A killed run's stale `.old` is replaced when the old CPK moves aside, and the commit
// removes it: nothing is left.
// TC-DEP-01
#[test]
fn a_stale_old_cpk_from_a_killed_run_leaves_none_after_a_deployed_run() {
    let sandbox = Sandbox::new("savefile_stale_old");
    install_pes(&sandbox);
    install_save(&sandbox, PesVersion::Pes21);
    sandbox.write("PES/download/4cc_99_test.cpk", b"the old CPK");
    sandbox.write("PES/download/4cc_99_test.cpk.old", b"a killed run's");
    motion_export(&sandbox, "co Full Motions");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        sandbox.root.join("PES/download/4cc_99_test.cpk").is_file(),
        "deployed"
    );
    assert_eq!(
        leftover_files(&sandbox, "PES/download"),
        Vec::<String>::new(),
        "no .old left"
    );
}

// The game's DPFL lists a CPK that is not in `download/` (`dpfilelist_cpk_missing`): the
// deployment commits, but the game rejects the whole folder and runs vanilla, so the
// savefile step does not run, silently, the warning having said why.
// TC-DEP-14
#[test]
fn a_deployed_run_the_game_runs_vanilla_leaves_the_savefile_alone() {
    let sandbox = Sandbox::new("savefile_vanilla");
    install_pes(&sandbox);
    let save = install_save(&sandbox, PesVersion::Pes21);
    let before = fs::read(&save).unwrap();
    fs::remove_file(sandbox.root.join("PES/download/4cc_62_midcup.cpk")).unwrap();
    motion_export(&sandbox, "co Full Motions");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let expected = format!(
        "Warning dpfilelist_cpk_missing [Keep] (path={}, files=4cc_62_midcup.cpk, command={} team-compiler upgrade-dpfl)",
        sandbox.display("PES/download"),
        studio_core::EXE_NAME
    );
    assert!(lines.contains(&expected), "{lines:#?}");
    // A leading space: the code field's position, which a sandbox path never matches.
    assert!(
        lines.iter().all(|line| !line.contains(" savefile_")),
        "{lines:#?}"
    );
    assert!(
        sandbox.root.join("PES/download/4cc_99_test.cpk").is_file(),
        "deployed"
    );
    assert_eq!(fs::read(&save).unwrap(), before, "byte-identical");
}

// The scenario's sideload form: no deployment to roll back, so the failure stays an Error
// and the new `livecpk/` tree stays in place.
// TC-SAV-05
#[test]
fn a_sideload_run_with_a_save_of_another_version_is_an_error_only() {
    let sandbox = Sandbox::new("savefile_sideload_invalid");
    sandbox.write("PES/PES2021.exe", b"the game");
    let save = install_save(&sandbox, PesVersion::Pes19);
    let before = fs::read(&save).unwrap();
    motion_export(&sandbox, "co Full Motions");

    let run = sandbox.run(
        &pes21_settings(&sandbox),
        &["compile", "--mode", "sideload"],
    );

    let lines = run.messages();
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    let expected = format!(
        "Error savefile_invalid [Keep] (path={}, version=PES 2019)",
        save.display()
    );
    assert!(lines.contains(&expected), "{lines:#?}");
    assert_eq!(fs::read(&save).unwrap(), before, "byte-identical");
    let livecpk = sandbox.root.join("PES/livecpk");
    assert!(livecpk.is_dir(), "the sideload tree in place");
    assert!(
        fs::read_dir(&livecpk).unwrap().next().is_some(),
        "not empty"
    );
}
