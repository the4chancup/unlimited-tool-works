//! The `aesthetics_patch.toml` `compile` writes to the output folder on a Fox target
//! (`spec/pes_savefile/operations.md` "Aesthetics patch"): every compiled player's resolved
//! motions, a `write_names` export's names, the `.bak` an earlier patch is moved to when the
//! run holds none, and `patch_write_failed` aborting the run.

use std::fs;

use pes_savefile::interchange::aesthetics_patch::AestheticsPatch;
use pes_savefile::settings_toml::PlayerSettings;
use pes_version::PesVersion;

use crate::clean_model;
use crate::common::Sandbox;
use crate::compile::{pes_settings, pes21_settings, tracer_kit};
use crate::prefox_faces::write_slot_05_face;

/// The patch in `sandbox`'s output folder, parsed back.
pub(crate) fn read_patch(sandbox: &Sandbox) -> AestheticsPatch {
    let text = fs::read_to_string(sandbox.root.join("output/aesthetics_patch.toml")).unwrap();
    AestheticsPatch::parse(&text).unwrap()
}

// TC-SAV-06
#[test]
fn a_run_holding_no_patch_moves_an_earlier_one_aside_to_bak() {
    let sandbox = Sandbox::new("patch_bak");
    let earlier = b"# an earlier run's patch\npes_version = 21\n";
    sandbox.write("output/aesthetics_patch.toml", earlier);
    // An older backup, which the move aside replaces.
    sandbox.write("output/aesthetics_patch.toml.bak", b"# an older backup\n");
    let export = "exports/co Full KitsOnly";
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{export}/Kits/p1/colors.txt"),
        b"#0a1b2c\n#d4e5f6\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        !lines.iter().any(|line| line.contains("patch_written")),
        "{lines:#?}"
    );
    let patch = sandbox.root.join("output/aesthetics_patch.toml");
    let bak = sandbox.root.join("output/aesthetics_patch.toml.bak");
    assert!(!patch.exists(), "no patch is written");
    assert_eq!(
        fs::read(&bak).unwrap().as_slice(),
        earlier,
        "the earlier one is now the .bak"
    );
}

/// A `write_names` export: `players.txt` mapping `Snuffy` to slot 05 and `Sharer` to 06 and
/// 08, `Snuffy` taking his name from the folder and `Sharer` setting its own.
pub(crate) fn write_names_export(sandbox: &Sandbox, export: &str, snuffy: &str, sharer: &str) {
    sandbox.write(&format!("{export}/write_names"), b"");
    sandbox.write(
        &format!("{export}/players.txt"),
        b"05 Snuffy\n06 Sharer\n08 Sharer\n",
    );
    sandbox.write(
        &format!("{export}/Players/Snuffy/settings.toml"),
        snuffy.as_bytes(),
    );
    sandbox.write(
        &format!("{export}/Players/Sharer/settings.toml"),
        sharer.as_bytes(),
    );
}

// The patch half of the write_names run (the savefile half is `savefile.rs`'s).
// TC-SAV-02
#[test]
fn a_write_names_export_s_names_reach_the_patch_shared_ones_warned() {
    let sandbox = Sandbox::new("patch_write_names");
    write_names_export(
        &sandbox,
        "exports/co Full Auto",
        "name = true\nshirt_name = \"SNUFFY\"\n",
        "name = \"Shared\"\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        lines.iter().any(|line| line
            == "co Full Auto: Warning settings_toml_name_shared [Keep] at Players/Sharer ()"),
        "{lines:#?}"
    );
    let patch = read_patch(&sandbox);
    let [team] = patch.teams.as_slice() else {
        panic!("one [[teams]] entry: {:?}", patch.teams)
    };
    assert_eq!(team.id, 714);
    let snuffy = &team.players[&5];
    assert_eq!(snuffy.name.as_deref(), Some("Snuffy"), "name = true's");
    assert_eq!(snuffy.shirt_name.as_deref(), Some("SNUFFY"));
    for slot in [6u8, 8] {
        assert_eq!(
            team.players[&slot].name.as_deref(),
            Some("Shared"),
            "slot {slot} carries the shared folder's name"
        );
    }
}

// The patch half (the savefile half is `savefile.rs`'s).
// TC-SAV-03
#[test]
fn a_name_the_field_refuses_is_dropped_and_warned() {
    let sandbox = Sandbox::new("patch_name_invalid");
    let export = "exports/co Full Auto";
    sandbox.write(&format!("{export}/write_names"), b"");
    // Longer than the PES 21 name field (61 bytes).
    sandbox.write(
        &format!("{export}/Players/05 - A/settings.toml"),
        format!("name = \"{}\"\n", "a".repeat(70)).as_bytes(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        lines.iter().any(|line| line
            == "co Full Auto: Warning settings_name_invalid [Keep] at \
                Players/05 - A (key=name)"),
        "{lines:#?}"
    );
    let patch = read_patch(&sandbox);
    let player = &patch.teams[0].players[&5];
    assert_eq!(player.name, None, "the refused name is not written");
}

// `check` reports the two name findings `compile` does: the names rule is the one planning
// builds the patch with.
// TC-SAV-13
#[test]
fn check_reports_the_name_findings_compile_does() {
    let sandbox = Sandbox::new("patch_names_check");
    write_names_export(
        &sandbox,
        "exports/co Full Auto",
        &format!("name = \"{}\"\n", "a".repeat(70)),
        "name = \"Shared\"\n",
    );
    let name_findings = |lines: &[String]| -> Vec<String> {
        lines
            .iter()
            .filter(|line| {
                line.contains("settings_toml_name_shared") || line.contains("settings_name_invalid")
            })
            .cloned()
            .collect()
    };

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    let compile = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let findings = name_findings(&check.messages());
    assert_eq!(findings, name_findings(&compile.messages()));
    for expected in [
        "co Full Auto: Warning settings_toml_name_shared [Keep] at Players/Sharer ()",
        "co Full Auto: Warning settings_name_invalid [Keep] at Players/Snuffy (key=name)",
    ] {
        assert!(
            findings.iter().any(|line| line == expected),
            "{findings:#?}"
        );
    }
}

// The patch half (the savefile half is `savefile.rs`'s).
// TC-SAV-12
#[test]
fn a_pre_fox_compile_writes_no_patch() {
    let sandbox = Sandbox::new("patch_prefox");
    write_slot_05_face(&sandbox, "co Full Old");
    sandbox.write(
        "exports/co Full Old/Players/05 - A/settings.toml",
        b"[appearance.motion]\nhunching_dribbling = 2\n",
    );

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let output = sandbox.root.join("output");
    assert!(!output.join("aesthetics_patch.toml").exists());
    assert!(!output.join("aesthetics_patch.toml.bak").exists());
}

// The patch half with no `write_names` marker (the savefile half is `savefile.rs`'s): no
// name is written, the whole look is (his settings.toml's resolved, every other key the
// template's), the header names the CPK and the version.
// TC-SAV-01
// TC-SAV-18
#[test]
fn a_plain_export_s_patch_holds_his_look_and_no_name() {
    let sandbox = Sandbox::new("patch_plain");
    let export = "exports/co Full Plain";
    // A name longer than PES 21's field: unused without the marker, so not checked either. No
    // finding reports the names of an export without it.
    let toml = format!(
        "name = \"{}\"\nshirt_name = \"SOME\"\n[appearance.motion]\nhunching_dribbling = 2\n",
        "a".repeat(70)
    );
    sandbox.write(
        &format!("{export}/Players/05 - A/settings.toml"),
        toml.as_bytes(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        lines
            .iter()
            .all(|line| !line.contains(" settings_name_invalid")
                && !line.contains(" settings_toml_name_shared")),
        "no name finding without the marker: {lines:#?}"
    );
    let written = format!(
        "Info patch_written [Keep] (path={}, teams=714)",
        sandbox.display("output/aesthetics_patch.toml")
    );
    assert!(lines.contains(&written), "{lines:#?}");
    let patch = read_patch(&sandbox);
    assert_eq!(patch.pes_version, PesVersion::Pes21);
    assert_eq!(patch.cpks, ["4cc_99_test.cpk"], "the run's CPK");
    let [team] = patch.teams.as_slice() else {
        panic!("one [[teams]] entry: {:?}", patch.teams)
    };
    assert_eq!(team.id, 714);
    assert_eq!(team.name, "/co/");
    let player = &team.players[&5];
    assert_eq!(player.name, None, "no marker, no name");
    assert_eq!(player.shirt_name, None);
    let expected = PlayerSettings::parse(&toml)
        .unwrap()
        .appearance
        .resolved(PesVersion::Pes21);
    assert_eq!(
        player.appearance.as_ref(),
        Some(&expected),
        "every key of the version's schema, the settings.toml's or the template's"
    );
}

// A referee export compiled beside a team export adds no `[[teams]]` entry to the patch.
// TC-SAV-14
#[test]
fn a_referee_export_gets_no_patch_entry() {
    let sandbox = Sandbox::new("patch_refs");
    sandbox.write(
        "exports/co Full Team/Players/05 - A/face_high.fmdl",
        &clean_model(),
    );
    sandbox.write("exports/refs Cup/players.txt", b"01 Keeper\n");
    sandbox.write(
        "exports/refs Cup/Players/Keeper/face_high.fmdl",
        &clean_model(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let patch = read_patch(&sandbox);
    assert_eq!(patch.teams.len(), 1, "{:?}", patch.teams);
    assert_eq!(patch.teams[0].id, 714, "only the team export's");
}

// `patch_write_failed` aborts the run when the patch's path is taken (exit 3), and nothing
// more is published: the export's note writes no `teamnotes.txt`.
// TC-SAV-15
#[test]
fn a_patch_path_taken_by_a_folder_aborts_the_run() {
    let sandbox = Sandbox::new("patch_write_failed");
    fs::create_dir_all(sandbox.root.join("output/aesthetics_patch.toml")).unwrap();
    sandbox.write(
        "exports/co Full Blocked/Players/05 - A/face_high.fmdl",
        &clean_model(),
    );
    sandbox.write("exports/co Full Blocked/notes.txt", b"A note\n");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    let path = sandbox.display("output/aesthetics_patch.toml");
    assert!(
        lines.iter().any(|line| line.starts_with(&format!(
            "Fatal patch_write_failed [AbortRun] (path={path}, error="
        ))),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 3);
    assert!(!sandbox.root.join("output/teamnotes.txt").exists());
}
