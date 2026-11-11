//! `compile`'s face folder for each player on PES 2021: none for a player folder holding
//! `ingame_face`, whose models the face would take go to the player's boots instead; a blank
//! one, the bundled face diff alone, for every other player folder with no face model; and a
//! face file in a folder with no face model, which nothing uses, reported by both commands.

use std::collections::BTreeMap;
use std::fs;

use crate::common::Sandbox;
use crate::compile::{cpk_entries, pes_settings, pes21_settings, tracer_player_file};
use crate::models::{
    body_skl, boots_mesh_count, compile_clean, package_names, template, tracer_boots_mesh_count,
};
use crate::prefox_faces::{expected_face_xml, face_cpk, face_folder, nested_entries, small_dds};
use crate::{TEAM_COLORS_MISSING, assert_row_says, clean_model, findings_of, patch_written};

const BOOTS_05: &str = "Asset/model/character/boots/k0625/#Win/boots.fpk";
const FACE_05: &str = "Asset/model/character/face/real/71405/#Win/face.fpk";
const FACE_07: &str = "Asset/model/character/face/real/71407/#Win/face.fpk";

/// The CPK paths of `entries`.
fn paths(entries: &BTreeMap<String, Vec<u8>>) -> Vec<&str> {
    entries.keys().map(String::as_str).collect()
}

/// Asserts the face package at `path` in `entries` is the blank one: the bundled
/// `face_diff.bin` alone, with its `.fpkd` beside it.
pub(crate) fn assert_blank_face(entries: &BTreeMap<String, Vec<u8>>, path: &str) {
    assert_eq!(package_names(&entries[path]), ["face_diff.bin"], "{path}");
    let package = fpk::FpkFile::read(&entries[path]).unwrap();
    assert_eq!(
        package.get("face_diff.bin").unwrap(),
        template("face_diff.bin"),
        "{path}"
    );
    let fpkd = path.replace(".fpk", ".fpkd");
    assert!(entries.contains_key(&fpkd), "{fpkd}");
}

// TC-STR-20
#[test]
fn the_marker_s_old_spellings_are_an_error_and_the_folder_compiles_as_without_it() {
    let sandbox = Sandbox::new("face_marker_renamed");
    let export = "exports/co Midcup Renamed";
    for (slot, marker) in [
        ("05 - A", "ingame_face.toml"),
        ("06 - B", "ingame_face"),
        ("07 - C", "ingame_face.txt"),
    ] {
        sandbox.write(&format!("{export}/Players/{slot}/{marker}"), b"");
    }

    // `check` and `compile` report alike: each old spelling on its file alone, no disallowed
    // file, nothing on slot 05's `ingame_face.toml`.
    for args in [&["check"][..], &["compile", "--no-deploy"][..]] {
        let run = sandbox.run(&pes21_settings(&sandbox), args);
        let lines = run.messages();
        assert_eq!(run.exit_code(), 1, "{args:?}: {lines:#?}");
        assert!(
            lines.iter().any(|line| line
                == "co Midcup Renamed: Error ingame_face_marker_renamed [DropFile] at \
                    Players/06 - B/ingame_face ()"),
            "{args:?}: {lines:#?}"
        );
        assert!(
            lines.iter().any(|line| line
                == "co Midcup Renamed: Error ingame_face_marker_renamed [DropFile] at \
                    Players/07 - C/ingame_face.txt ()"),
            "{args:?}: {lines:#?}"
        );
        for line in &lines {
            assert!(
                !line.contains("file_type_disallowed") && !line.contains("05 - A"),
                "{args:?}: {lines:#?}"
            );
        }
    }
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert!(
        paths.iter().all(|path| !path.contains("/face/real/71405/")),
        "{paths:#?}"
    );
    assert_blank_face(
        &entries,
        "Asset/model/character/face/real/71406/#Win/face.fpk",
    );
    assert_blank_face(
        &entries,
        "Asset/model/character/face/real/71407/#Win/face.fpk",
    );
}

// TC-STR-22
#[test]
fn a_name_holding_the_legacy_kit_token_is_refused_once_on_its_outermost_name() {
    let sandbox = Sandbox::new("str_kit_token_old");
    let export = "exports/co Midcup Tokens";
    // Slot 05 holds a texture named with a legacy token; `Common/` a folder so named with
    // two textures in it; slot 06 two names that are no token: capitals, and a `g` with no
    // digit after it.
    let files = [
        "Players/05 - A/kit1_u0714p1.dds",
        "Common/u0XXXp0/a.dds",
        "Common/u0XXXp0/b.dds",
        "Players/06 - B/U0714P1.dds",
        "Players/06 - B/u0XXXg_back.dds",
    ];
    for file in files {
        sandbox.write(&format!("{export}/{file}"), &small_dds());
    }
    for slot in ["05 - A", "06 - B"] {
        sandbox.write(
            &format!("{export}/Players/{slot}/face_high.fmdl"),
            &clean_model(),
        );
    }

    let run = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    // The whole list: the `Common/` folder is dropped alone, so no player falls with it and
    // no file below it is checked again (no `file_not_used`); slot 06 gets nothing.
    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Tokens"),
        [
            "Error kit_token_old [DropFolder] at Players/05 - A \
             (path=Players/05 - A/kit1_u0714p1.dds)",
            "Error kit_token_old [DropFolder] at Common/u0XXXp0 (path=Common/u0XXXp0)",
            "Info export_identified [Keep] (team=/co/, id=714)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    // The finding's text names the folders it checks, the letter case it matches, and the
    // upgrader that respells the token.
    assert_row_says(
        "kit_token_old",
        &[
            "inside a player, Boots or Gloves folder or in `Common/`",
            "`u0`, `p` and `g` in lowercase as shown",
            "export-upgrader upgrade",
        ],
    );
}

// TC-MOD-16
#[test]
fn under_ingame_face_no_face_is_built_and_a_model_the_face_would_take_is_the_boots() {
    let sandbox = Sandbox::new("face_ingame_reroute");
    let player = "exports/co Midcup Ingame/Players/05 - A";
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    sandbox.write(
        &format!("{player}/torso.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    // A real skeleton that differs from the bundled PES 21 one.
    let custom = body_skl("pes19");
    assert_ne!(custom, body_skl("pes21"));
    sandbox.write(&format!("{player}/torso.skl"), &custom);
    sandbox.write(
        &format!("{player}/glove_l.fmdl"),
        &tracer_player_file("glove_l.fmdl"),
    );
    let glove_fpk = "Asset/model/character/glove/g0625/#Win/glove.fpk";

    // No `fmdl_fcl_hair_fallback` for `torso.fmdl`, and no `file_not_used` for `torso.skl`.
    let entries = compile_clean(
        &sandbox,
        "co Midcup Ingame",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=torso.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            TEAM_COLORS_MISSING,
        ],
    );

    assert_eq!(
        paths(&entries),
        [
            BOOTS_05,
            "Asset/model/character/boots/k0625/#Win/boots.fpkd",
            glove_fpk,
            "Asset/model/character/glove/g0625/#Win/glove.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ],
        "no face/real/71405/ path"
    );
    assert_eq!(
        package_names(&entries[BOOTS_05]),
        ["boots.fmdl", "boots.skl"]
    );
    let boots = fpk::FpkFile::read(&entries[BOOTS_05]).unwrap();
    assert_eq!(boots.get("boots.skl").unwrap(), custom);
    assert_eq!(package_names(&entries[glove_fpk]), ["glove_l.fmdl"]);
}

// TC-MOD-17
#[test]
fn under_ingame_face_a_boots_link_beside_a_boots_model_combines_and_a_plain_one_loads_the_shared_folder()
 {
    let sandbox = Sandbox::new("face_ingame_link");
    let export = "exports/co Midcup Linked";
    sandbox.write(&format!("{export}/Players/05 - A/ingame_face.toml"), b"");
    sandbox.write(
        &format!("{export}/Players/05 - A/kit_boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    sandbox.write(&format!("{export}/Players/07 - B/Crocs.boots"), b"");
    sandbox.write(
        &format!("{export}/Boots/Crocs/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    let k0627 = "Asset/model/character/boots/k0627/#Win/boots.fpk";

    let entries = compile_clean(
        &sandbox,
        "co Midcup Linked",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=kit_boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            TEAM_COLORS_MISSING,
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=boots.fmdl)",
        ],
    );

    // Slot 07 has no marker: it gets the blank face; slot 05 none. Crocs keeps its own
    // output for slot 07, who alone links it plainly, under his own k0627: no shared id,
    // so no record.
    assert_eq!(
        paths(&entries),
        [
            BOOTS_05,
            "Asset/model/character/boots/k0625/#Win/boots.fpkd",
            k0627,
            "Asset/model/character/boots/k0627/#Win/boots.fpkd",
            "Asset/model/character/face/real/71407/#Win/face.fpk",
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ],
        "no face/real/71405/ path"
    );
    assert_eq!(
        boots_mesh_count(&entries, BOOTS_05),
        2 * tracer_boots_mesh_count(),
        "Crocs's meshes plus the local model's"
    );
    assert_eq!(boots_mesh_count(&entries, k0627), tracer_boots_mesh_count());
    assert_blank_face(&entries, FACE_07);
}

// TC-MOD-18
#[test]
fn under_ingame_face_an_empty_face_subfolder_is_ignored() {
    let sandbox = Sandbox::new("face_ingame_empty_face");
    let player = "exports/co Midcup Empty/Players/05 - A";
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    sandbox.write(
        &format!("{player}/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    // Slot 07 of TC-MOD-19's export, an empty `face/` alone, shows such a folder is seen.
    fs::create_dir_all(sandbox.root.join(format!("{player}/face"))).unwrap();

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Empty"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            TEAM_COLORS_MISSING,
        ]
    );
    // The last lines name the patch's and the CPK's paths, which hold the sandbox's own name.
    let (patch, findings) = lines.split_last().unwrap();
    assert_eq!(*patch, patch_written(&sandbox, "714"));
    let (last, findings) = findings.split_last().unwrap();
    assert!(last.starts_with("Info deploy_skipped_by_flag"), "{last}");
    assert!(
        findings.iter().all(|line| !line.contains("face/")),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        paths(&entries),
        [
            BOOTS_05,
            "Asset/model/character/boots/k0625/#Win/boots.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ],
        "no face/real/71405/ path"
    );
    assert_eq!(
        package_names(&entries[BOOTS_05]),
        ["boots.fmdl", "boots.skl"]
    );
}

// TC-MOD-19
#[test]
fn a_player_folder_with_no_face_model_gets_the_blank_face() {
    let sandbox = Sandbox::new("face_blank");
    let export = "exports/co Midcup Blank";
    sandbox.write(
        &format!("{export}/Players/05 - A/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    // Slot 07 is an empty `face/` alone: without that folder the export holds nothing for
    // slot 07, so its face shows the empty folder is seen.
    fs::create_dir_all(sandbox.root.join(format!("{export}/Players/07 - B/face"))).unwrap();

    let entries = compile_clean(
        &sandbox,
        "co Midcup Blank",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            TEAM_COLORS_MISSING,
        ],
    );

    assert_eq!(
        paths(&entries),
        [
            BOOTS_05,
            "Asset/model/character/boots/k0625/#Win/boots.fpkd",
            FACE_05,
            "Asset/model/character/face/real/71405/#Win/face.fpkd",
            FACE_07,
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ]
    );
    assert_blank_face(&entries, FACE_05);
    assert_blank_face(&entries, FACE_07);
    assert_eq!(
        package_names(&entries[BOOTS_05]),
        ["boots.fmdl", "boots.skl"]
    );
}

// TC-MOD-32
#[test]
fn a_face_file_in_a_folder_with_no_face_model_is_not_used_and_reported() {
    let sandbox = Sandbox::new("face_file_not_used");
    let export = "exports/co Midcup Unused";
    // The tracer's `face_diff.bin`: a face diff the game reads, other than the bundled one.
    let own = tracer_player_file("face_diff.bin");
    assert_ne!(own, template("face_diff.bin"));
    for player in ["Players/05 - A", "Players/07 - B"] {
        sandbox.write(
            &format!("{export}/{player}/boots.fmdl"),
            &tracer_player_file("boots.fmdl"),
        );
        sandbox.write(&format!("{export}/{player}/face_diff.bin"), &own);
    }
    sandbox.write(&format!("{export}/Players/07 - B/ingame_face.toml"), b"");
    let validated = [
        "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
        "Info fmdl_weights_not_normalized [Keep] at Players/07 - B (file=boots.fmdl, count=1662)",
        "Info export_identified [Keep] (team=/co/, id=714)",
        "Info face_file_not_used [Keep] at Players/05 - A (file=face_diff.bin)",
        "Info face_file_not_used [Keep] at Players/07 - B (file=face_diff.bin)",
    ];

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    assert_eq!(
        findings_of(&check.messages(), "co Midcup Unused"),
        validated
    );
    assert_eq!(check.exit_code(), 0);
    let entries = compile_clean(
        &sandbox,
        "co Midcup Unused",
        &[&validated[..], &[TEAM_COLORS_MISSING]].concat(),
    );

    assert_eq!(
        paths(&entries),
        [
            BOOTS_05,
            "Asset/model/character/boots/k0625/#Win/boots.fpkd",
            "Asset/model/character/boots/k0627/#Win/boots.fpk",
            "Asset/model/character/boots/k0627/#Win/boots.fpkd",
            FACE_05,
            "Asset/model/character/face/real/71405/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ],
        "no face/real/71407/ path"
    );
    assert_blank_face(&entries, FACE_05);
}

// TC-MOD-32, TC-MOD-39
#[test]
fn a_link_to_a_face_folder_holding_textures_alone_gives_no_face() {
    let sandbox = Sandbox::new("face_link_textures_only");
    let export = "exports/co Midcup Unused";
    let player = format!("{export}/Players/05 - A");
    let own = tracer_player_file("face_diff.bin");
    assert_ne!(own, template("face_diff.bin"));
    sandbox.write(
        &format!("{player}/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(&format!("{player}/face_diff.bin"), &own);
    sandbox.write(&format!("{player}/Round.face"), b"");
    // A texture source, not a face: the player's own diff has no face model to go with.
    sandbox.write(&format!("{export}/Faces/Round/skin.dds"), &small_dds());
    let validated = [
        "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
        "Info export_identified [Keep] (team=/co/, id=714)",
        "Info face_file_not_used [Keep] at Players/05 - A (file=face_diff.bin)",
    ];

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    assert_eq!(
        findings_of(&check.messages(), "co Midcup Unused"),
        validated
    );
    assert_eq!(check.exit_code(), 0);
    // The link is still reported as combining: the folder's textures are a source of his.
    let link_combined = "Info link_combined [Keep] at Players/05 - A (link=Round.face)";
    let entries = compile_clean(
        &sandbox,
        "co Midcup Unused",
        &[&validated[..], &[TEAM_COLORS_MISSING, link_combined]].concat(),
    );
    assert_blank_face(&entries, FACE_05);

    // On PES 17 his boots model would be a part of his face, so he holds the diff and the
    // link alone there.
    fs::remove_file(sandbox.root.join(format!("{player}/boots.fmdl"))).unwrap();
    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);
    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Unused"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info face_file_not_used [Keep] at Players/05 - A (file=face_diff.bin)",
            TEAM_COLORS_MISSING,
            link_combined,
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let face = nested_entries(&entries[&face_cpk(5)]);
    let folder = face_folder(5);
    let names: Vec<&str> = face
        .keys()
        .map(|path| path.strip_prefix(folder.as_str()).unwrap())
        .collect();
    assert_eq!(names, ["dummy.mtl", "face.xml", "oral_dummy_win32.model"]);
    assert_eq!(
        face[&format!("{folder}face.xml")],
        expected_face_xml(
            &[("face_neck", "./oral_dummy_*.model", "./dummy.mtl", None)],
            &template("face_diff.bin")
        )
    );
}
