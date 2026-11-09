//! `compile` over `.common` links on Fox: a model link's Common model baked into the player's
//! package as a part, its skeleton with it, its textures left in the team's Common output, where
//! every texture directly in `Common/` goes once for the team; a texture link pointing the
//! player's own models at that output; and what a lenient file-type check keeps around them,
//! which nothing reads.

use std::collections::{BTreeMap, BTreeSet};

use fmdl::ops::paths::{rewrite_texture_paths, texture_paths};
use fmdl::{FmdlFile, Model};

use crate::bins::{install_cpk, install_names};
use crate::common::Sandbox;
use crate::compile::{
    compiled_players, cpk_entries, pes21_settings, pixels_cut_dds, tracer_kit, tracer_player_file,
};
use crate::conversion::HOME_714_05;
use crate::models::{body_skl, face_package, package_names};
use crate::patch::read_patch;
use crate::prefox_faces::{
    CLEAN, card_materials, card_model, compile_pes17, entries_under, face_cpk, face_folder,
    materials_naming, nested_entries, pes17, pre_fox_fixture, sampler_paths, small_dds,
    write_slot_05_face,
};
use crate::textures::texture_fixture;
use crate::{clean_model, findings_of};
use pes_model::format::mtl::MaterialSet;
use studio_core::StudioTool;

const FACE_05: &str = "Asset/model/character/face/real/71405/#Win/face.fpk";
const FACE_05_FPKD: &str = "Asset/model/character/face/real/71405/#Win/face.fpkd";
const BOOTS_05: &str = "Asset/model/character/boots/k0625/#Win/boots.fpk";
/// The team's Common output, the Common row of the game paths.
const COMMON_TEXTURES: &str = "Asset/model/character/common/714/sourceimages/#windx11";
const COMMON_DIRECTORY: &str = "/Assets/pes16/model/character/common/714/sourceimages/";

/// The tracer's hair model as a Common model of its own: its materials renamed with a
/// `_common` tail and its `shirt.dds` renamed `cloth.dds`, through `fmdl`'s model API, so it
/// merges with the hair itself with no material in common (two copies of one FMDL would define
/// every material twice, over textures in different places).
fn common_hair_model() -> Vec<u8> {
    let file = FmdlFile::read(&tracer_player_file("fcl_hair.fmdl")).unwrap();
    let mut model = Model::from_file(&file).unwrap();
    for material in &mut model.materials {
        material.name.push_str("_common");
        for (_, texture) in &mut material.textures {
            if texture.file_name == "shirt.dds" {
                texture.file_name = "cloth.dds".to_owned();
            }
        }
    }
    model.to_file().unwrap().write()
}

/// The mesh count of the FMDL `bytes`.
fn mesh_count(bytes: &[u8]) -> usize {
    Model::from_file(&FmdlFile::read(bytes).unwrap())
        .unwrap()
        .meshes
        .len()
}

/// The directory of every texture path of the FMDL `bytes` naming `file_name`.
pub(crate) fn texture_directories(bytes: &[u8], file_name: &str) -> Vec<String> {
    texture_paths(&FmdlFile::read(bytes).unwrap())
        .unwrap()
        .into_iter()
        .filter(|path| path.file_name == file_name)
        .map(|path| path.directory)
        .collect()
}

/// Asserts no entry of the CPK is a link file or a Common model's own path: the link is baked
/// away and the model travels inside a package.
fn assert_no_common_path(entries: &BTreeMap<String, Vec<u8>>) {
    for path in entries.keys() {
        assert!(
            !path.ends_with(".common") && !path.starts_with("Common"),
            "{path}"
        );
    }
}

// TC-MOD-10
// TC-MOD-11
#[test]
fn a_common_model_link_bakes_the_model_into_the_face_and_its_texture_stays_in_the_team_s_common() {
    let sandbox = Sandbox::new("cmn_face_link");
    let export = "exports/co Midcup Link";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/torso.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(
        &format!("{player}/shirt.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(&format!("{player}/legs.fmdl.common"), b"");
    sandbox.write(&format!("{export}/Common/legs.fmdl"), &common_hair_model());
    sandbox.write(
        &format!("{export}/Common/cloth.dds"),
        &tracer_player_file("shirt.dds"),
    );
    let cloth = format!("{COMMON_TEXTURES}/cloth.ftex");
    let shirt = "Asset/model/character/common/714/05 - A/sourceimages/#windx11/shirt.ftex";

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Link"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=torso.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Common/legs.fmdl (file=legs.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=legs.fmdl.common)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=torso.fmdl)",
            "Info team_colors_missing [Keep] ()",
            "Info fmdl_merged [Keep] at Players/05 - A (model=fcl_hair.fmdl)"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            shirt,
            cloth.as_str(),
            FACE_05,
            FACE_05_FPKD,
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin"
        ],
        "cloth once, in the team's Common output and not under the player's subfolder"
    );
    assert_no_common_path(&entries);
    assert_eq!(
        entries[&cloth],
        ftex::dds_to_ftex(&tracer_player_file("shirt.dds"), ftex::ColorSpace::Normal).unwrap()
    );
    let package = face_package(&entries);
    assert_eq!(
        package_names(&entries[FACE_05]),
        [
            "face_diff.bin",
            "fcl_hair.fmdl",
            "fcl_hair_sim.fclo",
            "fcl_hair_sim.skl"
        ]
    );
    let merged = package.get("fcl_hair.fmdl").unwrap();
    assert_eq!(
        mesh_count(merged),
        2 * mesh_count(&tracer_player_file("fcl_hair.fmdl")),
        "the local part's meshes plus the Common model's"
    );
    // The Common model's `cloth` points at the team's Common output, the player's own `shirt`
    // at the player's subfolder.
    let cloth_directories = texture_directories(merged, "cloth.dds");
    assert!(!cloth_directories.is_empty());
    assert!(
        cloth_directories
            .iter()
            .all(|directory| directory == COMMON_DIRECTORY),
        "{cloth_directories:?}"
    );
    let shirt_directories = texture_directories(merged, "shirt.dds");
    assert!(!shirt_directories.is_empty());
    assert!(
        shirt_directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/05 - A/sourceimages/"),
        "{shirt_directories:?}"
    );
}

// TC-MOD-02
#[test]
fn a_boots_link_packs_the_common_skeleton_and_two_players_linking_one_model_share_its_texture() {
    let sandbox = Sandbox::new("cmn_boots_link");
    let export = "exports/co Midcup Boots";
    // Named for the boots: a link is read directly in the player folder alone, and the linked
    // model's name gives its role.
    for slot in ["05 - A", "07 - B"] {
        sandbox.write(
            &format!("{export}/Players/{slot}/kit_boots.fmdl.common"),
            b"",
        );
    }
    sandbox.write(
        &format!("{export}/Common/kit_boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    let custom = body_skl("pes19");
    assert_ne!(custom, body_skl("pes21"));
    sandbox.write(&format!("{export}/Common/kit_boots.skl"), &custom);
    sandbox.write(
        &format!("{export}/Common/shirt.dds"),
        &tracer_player_file("shirt.dds"),
    );
    let shirt = format!("{COMMON_TEXTURES}/shirt.ftex");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Boots"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Common/kit_boots.fmdl (file=kit_boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            BOOTS_05,
            "Asset/model/character/boots/k0625/#Win/boots.fpkd",
            "Asset/model/character/boots/k0627/#Win/boots.fpk",
            "Asset/model/character/boots/k0627/#Win/boots.fpkd",
            shirt.as_str(),
            FACE_05,
            FACE_05_FPKD,
            "Asset/model/character/face/real/71407/#Win/face.fpk",
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ],
        "each player's package and blank face, the Common texture once"
    );
    assert_no_common_path(&entries);
    for path in [BOOTS_05, "Asset/model/character/boots/k0627/#Win/boots.fpk"] {
        let package = fpk::FpkFile::read(&entries[path]).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(names, ["boots.fmdl", "boots.skl"], "{path}");
        assert_eq!(package.get("boots.skl").unwrap(), custom, "{path}");
        let directories = texture_directories(package.get("boots.fmdl").unwrap(), "shirt.dds");
        assert!(!directories.is_empty());
        assert!(
            directories
                .iter()
                .all(|directory| directory == COMMON_DIRECTORY),
            "{path}: {directories:?}"
        );
    }
}

// TC-STR-15
#[test]
fn two_link_spellings_of_one_common_model_compile_it_once() {
    // `boots.fmdl.common` and the tolerated `boots.fmdl.common.txt` are one link to
    // one model: the boots task reads the model once, not twice.
    let sandbox = Sandbox::new("cmn_link_twice");
    let export = "exports/co Midcup Links";
    sandbox.write(&format!("{export}/Players/05 - A/boots.fmdl.common"), b"");
    sandbox.write(
        &format!("{export}/Players/05 - A/boots.fmdl.common.txt"),
        b"",
    );
    sandbox.write(
        &format!("{export}/Common/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let package = fpk::FpkFile::read(&entries[BOOTS_05]).unwrap();
    assert_eq!(
        package.entries().map(|(name, _)| name).collect::<Vec<_>>(),
        ["boots.fmdl", "boots.skl"]
    );
}

// TC-MOD-86
#[test]
fn a_common_skeleton_pairs_with_its_model_case_folded() {
    // `Common/Boots.skl` names `boots.fmdl`'s skeleton as the file system folds it.
    let sandbox = Sandbox::new("cmn_skl_case");
    let export = "exports/co Midcup Case";
    sandbox.write(&format!("{export}/Players/05 - A/boots.fmdl.common"), b"");
    sandbox.write(
        &format!("{export}/Common/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    let custom = body_skl("pes19");
    assert_ne!(custom, body_skl("pes21"));
    sandbox.write(&format!("{export}/Common/Boots.skl"), &custom);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let package = fpk::FpkFile::read(&entries[BOOTS_05]).unwrap();
    assert_eq!(package.get("boots.skl").unwrap(), custom);
}

// TC-MOD-07
#[test]
fn a_boots_link_beside_a_shared_boots_link_combines_the_shared_folder() {
    let sandbox = Sandbox::new("cmn_link_combined");
    let export = "exports/co Midcup Combined";
    sandbox.write(
        &format!("{export}/Players/05 - A/kit_boots.fmdl.common"),
        b"",
    );
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    sandbox.write(
        &format!("{export}/Common/kit_boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(
        &format!("{export}/Boots/Crocs/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Combined"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Common/kit_boots.fmdl (file=kit_boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=boots.fmdl)"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            BOOTS_05,
            "Asset/model/character/boots/k0625/#Win/boots.fpkd",
            FACE_05,
            FACE_05_FPKD,
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ],
        "no k0644: the shared folder is only a source of parts"
    );
    let package = fpk::FpkFile::read(&entries[BOOTS_05]).unwrap();
    assert_eq!(
        mesh_count(package.get("boots.fmdl").unwrap()),
        2 * mesh_count(&tracer_player_file("boots.fmdl"))
    );
}

// TC-MOD-13
#[test]
fn a_common_skeleton_of_a_slotless_face_model_is_reported_on_the_link_and_not_packed() {
    let sandbox = Sandbox::new("cmn_skl_no_slot");
    let export = "exports/co Midcup Slot";
    sandbox.write(
        &format!("{export}/Players/05 - A/face_high.fmdl.common"),
        b"",
    );
    sandbox.write(
        &format!("{export}/Common/face_high.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(
        &format!("{export}/Common/face_high.skl"),
        &tracer_player_file("fcl_hair.skl"),
    );
    let findings = [
        "Info fmdl_weights_not_normalized [Keep] at Common/face_high.fmdl (file=face_high.fmdl, count=1662)",
        "Info export_identified [Keep] (team=/co/, id=714)",
        "Warning skl_no_slot [Keep] at Players/05 - A (file=face_high.fmdl.common)",
    ];

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    assert_eq!(findings_of(&check.messages(), "co Midcup Slot"), findings);
    assert_eq!(check.exit_code(), 0);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    assert_eq!(
        findings_of(&run.messages(), "co Midcup Slot"),
        [&findings[..], &["Info team_colors_missing [Keep] ()"]].concat()
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        package_names(&entries[FACE_05]),
        ["face_diff.bin", "face_high.fmdl"]
    );
}

/// Writes the export `export`'s `Common/legs.model`, the card head, with the card's material
/// set naming `skin` as `Common/legs.mtl`, and `Common/skin.dds`.
fn write_common_card(sandbox: &Sandbox, export: &str) {
    sandbox.write(&format!("{export}/Common/legs.model"), &card_model());
    sandbox.write(&format!("{export}/Common/legs.mtl"), &card_materials());
    sandbox.write(&format!("{export}/Common/skin.dds"), &small_dds());
}

/// The meshes of the card head converted to an FMDL: its one mesh.
const CONVERTED_CARD_MESHES: usize = 1;

/// The card head as a model of the player's own: its one material renamed `torso`, in the
/// model and in its material set, which names `./face.dds`. It merges with the card itself
/// with no material in common, and its one bone is the card's, at the card's pose (a part of
/// another skeleton's pose would be `skl_merge_conflict`).
fn own_card() -> (Vec<u8>, Vec<u8>) {
    let file = pes_model::format::PreFoxModel::read(&card_model()).unwrap();
    let mut model = pes_model::model::Model::from_file(&file).unwrap();
    model.materials = vec!["torso".to_owned()];
    let mut materials = MaterialSet::read(&materials_naming("face")).unwrap();
    materials.materials[0].name = "torso".to_owned();
    (model.to_file().unwrap().write().unwrap(), materials.write())
}

// TC-MOD-10, TC-MOD-11
#[test]
fn a_common_model_link_converts_the_model_with_its_common_mtl_into_the_player_s_face() {
    let sandbox = Sandbox::new("cmn_model_link");
    let export = "exports/co Midcup Card";
    let player = format!("{export}/Players/05 - A");
    let (torso, torso_materials) = own_card();
    sandbox.write(&format!("{player}/torso.model"), &torso);
    sandbox.write(&format!("{player}/torso.mtl"), &torso_materials);
    sandbox.write(&format!("{player}/face.dds"), &small_dds());
    sandbox.write(&format!("{player}/legs.model.common"), b"");
    write_common_card(&sandbox, export);
    let skin = format!("{COMMON_TEXTURES}/skin.ftex");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Card"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=legs.model.common)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=torso.model)",
            "Info team_colors_missing [Keep] ()",
            "Info fmdl_merged [Keep] at Players/05 - A (model=fcl_hair.fmdl)"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "Asset/model/character/common/714/05 - A/sourceimages/#windx11/face.ftex",
            skin.as_str(),
            FACE_05,
            FACE_05_FPKD,
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin"
        ],
        "skin once, in the team's Common output"
    );
    assert_no_common_path(&entries);
    let package = face_package(&entries);
    let merged = package.get("fcl_hair.fmdl").unwrap();
    assert_eq!(
        mesh_count(merged),
        2 * CONVERTED_CARD_MESHES,
        "the local part's meshes plus the converted Common model's"
    );
    // The Common `.mtl`'s `./skin.dds`, pointed at the team's Common output, the player's own
    // `./face.dds` at his subfolder.
    let skin_directories = texture_directories(merged, "skin.dds");
    assert!(!skin_directories.is_empty());
    assert!(
        skin_directories
            .iter()
            .all(|directory| directory == COMMON_DIRECTORY),
        "{skin_directories:?}"
    );
    let face_directories = texture_directories(merged, "face.dds");
    assert!(!face_directories.is_empty());
    assert!(
        face_directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/05 - A/sourceimages/"),
        "{face_directories:?}"
    );
}

// TC-XML-19
#[test]
fn a_common_mtl_naming_a_team_id_drops_the_linking_player_s_folder_on_fox() {
    // Fox packs no `Common/` `.mtl` of its own: each linking player's model converts with it,
    // so its refusal drops his folder, where pre-Fox drops the file (TC-XML-08).
    let sandbox = Sandbox::new("cmn_mtl_team_id_fox");
    let export = "exports/co Midcup Card";
    sandbox.write(&format!("{export}/Players/05 - A/legs.model.common"), b"");
    write_common_card(&sandbox, export);
    let path = "common_folder/797/skin.dds";
    let mtl = String::from_utf8(card_materials())
        .unwrap()
        .replace("./skin.dds", path)
        .into_bytes();
    sandbox.write(&format!("{export}/Common/legs.mtl"), &mtl);

    let run = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    let lines = run.messages();
    let invalid = format!(
        "Error mtl_common_path_invalid [DropFolder] at Players/05 - A \
         (file=Common/legs.mtl, attribute=path, value={path})"
    );
    assert_eq!(
        findings_of(&lines, "co Midcup Card"),
        [
            invalid.as_str(),
            "Info export_identified [Keep] (team=/co/, id=714)"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-MOD-61
#[test]
fn a_common_model_converted_with_the_player_s_own_mtl_names_his_texture_of_its_stem() {
    let sandbox = Sandbox::new("cmn_model_local_mtl");
    let export = "exports/co Midcup Card";
    let player = format!("{export}/Players/05 - A");
    let (torso, torso_materials) = own_card();
    sandbox.write(&format!("{player}/torso.model"), &torso);
    sandbox.write(&format!("{player}/torso.mtl"), &torso_materials);
    sandbox.write(&format!("{player}/face.dds"), &small_dds());
    sandbox.write(&format!("{player}/legs.model.common"), b"");
    // His own `legs.mtl` beats `Common/`'s, and its `./skin.dds` is his own, other bytes than
    // Common's `skin.dds`.
    sandbox.write(&format!("{player}/legs.mtl"), &card_materials());
    sandbox.write(&format!("{player}/skin.dds"), &tracer_kit());
    write_common_card(&sandbox, export);
    let home = "Asset/model/character/common/714/05 - A/sourceimages/#windx11";
    let common_skin = format!("{COMMON_TEXTURES}/skin.ftex");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Card"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=legs.model.common)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=torso.model)",
            "Info team_colors_missing [Keep] ()",
            "Info fmdl_merged [Keep] at Players/05 - A (model=fcl_hair.fmdl)"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            format!("{home}/face.ftex").as_str(),
            format!("{home}/skin.ftex").as_str(),
            common_skin.as_str(),
            FACE_05,
            FACE_05_FPKD,
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin"
        ],
        "his skin at his home, Common's packed by the Common textures task all the same"
    );
    let package = face_package(&entries);
    let merged = package.get("fcl_hair.fmdl").unwrap();
    // His `legs.mtl` set `./skin.dds`, so it resolves in his folder, where his `skin.dds` is.
    for file_name in ["skin.dds", "face.dds"] {
        let directories = texture_directories(merged, file_name);
        assert!(!directories.is_empty(), "{file_name}");
        assert!(
            directories.iter().all(|directory| directory == HOME_714_05),
            "{file_name}: {directories:?}"
        );
    }
}

// TC-MOD-54
#[test]
fn a_player_s_model_converted_with_a_common_mtl_names_common_s_texture_of_its_stem() {
    let sandbox = Sandbox::new("cmn_own_model_common_mtl");
    let export = "exports/co Midcup Card";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{player}/legs.model"), &card_model());
    sandbox.write(&format!("{player}/legs.mtl.common"), b"");
    // His own `skin.dds`, other bytes than Common's: `Common/legs.mtl` set `./skin.dds`, so it
    // resolves in `Common/`, his own shadowing nothing.
    sandbox.write(&format!("{player}/skin.dds"), &tracer_kit());
    write_common_card(&sandbox, export);
    let home_skin = "Asset/model/character/common/714/05 - A/sourceimages/#windx11/skin.ftex";
    let common_skin = format!("{COMMON_TEXTURES}/skin.ftex");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Card"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=legs.model)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        entries[&common_skin] != entries[home_skin],
        "Common's skin, not his, in the team's Common output"
    );
    let package = face_package(&entries);
    let directories = texture_directories(package.get("fcl_hair.fmdl").unwrap(), "skin.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| directory == COMMON_DIRECTORY),
        "{directories:?}"
    );
}

// TC-MOD-11
#[test]
fn two_players_linking_one_common_model_each_convert_it_and_share_its_texture() {
    let sandbox = Sandbox::new("cmn_model_link_twice");
    let export = "exports/co Midcup Cards";
    for slot in ["05 - A", "07 - B"] {
        sandbox.write(&format!("{export}/Players/{slot}/legs.model.common"), b"");
    }
    write_common_card(&sandbox, export);
    let skin = format!("{COMMON_TEXTURES}/skin.ftex");
    let face_07 = "Asset/model/character/face/real/71407/#Win/face.fpk";

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Cards"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=legs.model.common)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/07 - B (file=legs.model.common)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            skin.as_str(),
            FACE_05,
            FACE_05_FPKD,
            face_07,
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin"
        ],
        "each player's face, the Common texture once"
    );
    for path in [FACE_05, face_07] {
        let package = fpk::FpkFile::read(&entries[path]).unwrap();
        let hair = package.get("fcl_hair.fmdl").unwrap();
        assert_eq!(mesh_count(hair), CONVERTED_CARD_MESHES, "{path}");
        let directories = texture_directories(hair, "skin.dds");
        assert!(!directories.is_empty());
        assert!(
            directories
                .iter()
                .all(|directory| directory == COMMON_DIRECTORY),
            "{path}: {directories:?}"
        );
    }
}

// TC-MOD-87
#[test]
fn a_material_a_local_and_a_common_part_define_over_textures_in_two_places_drops_the_folder() {
    let sandbox = Sandbox::new("cmn_material_conflict");
    let export = "exports/co Midcup Conflict";
    let player = format!("{export}/Players/05 - A");
    // The same hair model locally and in Common, each beside a `shirt.dds` of its own.
    sandbox.write(
        &format!("{player}/torso.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(&format!("{player}/torso2.fmdl.common"), b"");
    sandbox.write(
        &format!("{player}/shirt.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(
        &format!("{export}/Common/torso2.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(
        &format!("{export}/Common/shirt.dds"),
        &tracer_player_file("shirt.dds"),
    );
    // Slot 07: boots that compile, so the CPK is written.
    sandbox.write(
        &format!("{export}/Players/07 - B/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Conflict"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=torso.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/07 - B (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Common/torso2.fmdl (file=torso2.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=torso.fmdl)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=torso2.fmdl.common)",
            "Info team_colors_missing [Keep] ()",
            "Error merge_material_conflict [DropFolder] at Players/05 - A (material=shirt)"
        ]
    );
    assert_eq!(run.exit_code(), 1);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "Asset/model/character/boots/k0627/#Win/boots.fpk",
            "Asset/model/character/boots/k0627/#Win/boots.fpkd",
            &format!("{COMMON_TEXTURES}/shirt.ftex"),
            "Asset/model/character/face/real/71407/#Win/face.fpk",
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ],
        "nothing of slot 05, its own shirt included; the Common texture is the team's"
    );
}

// TC-TEX-12
#[test]
fn a_common_texture_that_cannot_convert_fails_the_common_task_and_the_linking_player_still_builds()
{
    let sandbox = Sandbox::new("cmn_texture_failed");
    let export = "exports/co Midcup Broken";
    sandbox.write(&format!("{export}/Players/05 - A/legs.fmdl.common"), b"");
    sandbox.write(
        &format!("{export}/Common/legs.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    // A header the deep pass reads, over pixel data cut short: the conversion fails.
    sandbox.write(&format!("{export}/Common/broken.dds"), &pixels_cut_dds());
    sandbox.write(
        &format!("{export}/Common/cloth.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    let findings = findings_of(&lines, "co Midcup Broken");
    assert_eq!(findings.len(), 5, "{findings:?}");
    assert_eq!(
        findings[..4],
        [
            "Info fmdl_weights_not_normalized [Keep] at Common/legs.fmdl (file=legs.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=legs.fmdl.common)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert!(
        findings[4].starts_with(
            "Error folder_pack_failed [DropFolder] at Common (error=cannot convert broken.dds"
        ),
        "{}",
        findings[4]
    );
    assert_eq!(run.exit_code(), 1);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            FACE_05,
            FACE_05_FPKD,
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin"
        ],
        "no Common texture, `cloth` included; the player's face is in"
    );
}

/// The tracer's hair model naming `hair.dds` in its `shirt` material and `skin.dds` in every
/// other material that named `shirt.dds`, through `fmdl`'s model API: one model with a path
/// to each texture.
fn model_naming_hair_and_skin() -> Vec<u8> {
    let file = FmdlFile::read(&tracer_player_file("fcl_hair.fmdl")).unwrap();
    let mut model = Model::from_file(&file).unwrap();
    for material in &mut model.materials {
        let renamed = if material.name == "shirt" {
            "hair.dds"
        } else {
            "skin.dds"
        };
        for (_, texture) in &mut material.textures {
            if texture.file_name == "shirt.dds" {
                texture.file_name = renamed.to_owned();
            }
        }
    }
    model.to_file().unwrap().write()
}

// TC-TEX-09
#[test]
fn a_texture_link_points_the_player_s_model_at_the_one_copy_in_the_team_s_common_output() {
    let sandbox = Sandbox::new("cmn_texture_link");
    let export = "exports/co Midcup Hair";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &model_naming_hair_and_skin(),
    );
    sandbox.write(
        &format!("{player}/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(&format!("{player}/hair.dds.common"), b"");
    sandbox.write(
        &format!("{export}/Common/hair.dds"),
        &tracer_player_file("shirt.dds"),
    );
    let hair = format!("{COMMON_TEXTURES}/hair.ftex");
    let skin = "Asset/model/character/common/714/05 - A/sourceimages/#windx11/skin.ftex";

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Hair"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            skin,
            hair.as_str(),
            FACE_05,
            FACE_05_FPKD,
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin"
        ],
        "hair once, in the team's Common output; the player's own folder holds skin alone"
    );
    assert_no_common_path(&entries);
    let package = face_package(&entries);
    let model = package.get("face_high.fmdl").unwrap();
    let hair_directories = texture_directories(model, "hair.dds");
    assert!(!hair_directories.is_empty());
    assert!(
        hair_directories
            .iter()
            .all(|directory| directory == COMMON_DIRECTORY),
        "{hair_directories:?}"
    );
    let skin_directories = texture_directories(model, "skin.dds");
    assert!(!skin_directories.is_empty());
    assert!(
        skin_directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/05 - A/sourceimages/"),
        "{skin_directories:?}"
    );
}

/// The texture link above with an FTEX for target: the link's stem is the name before
/// `.ftex.common`, as before `.dds.common`.
// TC-TEX-09
#[test]
fn a_link_to_an_ftex_points_the_player_s_model_at_the_one_copy_in_the_team_s_common_output() {
    let sandbox = Sandbox::new("cmn_texture_link_ftex");
    let export = "exports/co Midcup Hair";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &model_naming_hair_and_skin(),
    );
    sandbox.write(
        &format!("{player}/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(&format!("{player}/hair.ftex.common"), b"");
    let hair_ftex =
        ftex::dds_to_ftex(&tracer_player_file("shirt.dds"), ftex::ColorSpace::Normal).unwrap();
    sandbox.write(&format!("{export}/Common/hair.ftex"), &hair_ftex);
    let hair = format!("{COMMON_TEXTURES}/hair.ftex");
    let skin = "Asset/model/character/common/714/05 - A/sourceimages/#windx11/skin.ftex";

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Hair"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            skin,
            hair.as_str(),
            FACE_05,
            FACE_05_FPKD,
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin"
        ],
        "hair once, in the team's Common output; the player's own folder holds skin alone"
    );
    assert_no_common_path(&entries);
    let package = face_package(&entries);
    let model = package.get("face_high.fmdl").unwrap();
    let hair_directories = texture_directories(model, "hair.dds");
    assert!(!hair_directories.is_empty());
    assert!(
        hair_directories
            .iter()
            .all(|directory| directory == COMMON_DIRECTORY),
        "{hair_directories:?}"
    );
}

/// The tracer's hair model with its `shirt.dds` renamed `hair.dds` and pointed at the game's
/// own Common texture folder, which names no team.
fn model_naming_game_hair() -> Vec<u8> {
    model_naming_game("hair.dds")
}

/// The tracer's hair model with its `shirt.dds` renamed `file_name` and pointed at the game's
/// own Common texture folder, which names no team.
fn model_naming_game(file_name: &str) -> Vec<u8> {
    model_naming_in(file_name, GAME_COMMON_DIRECTORY)
}

/// The tracer's hair model with its `shirt.dds` renamed `file_name` and pointed at `directory`.
pub(crate) fn model_naming_in(file_name: &str, directory: &str) -> Vec<u8> {
    let mut file = FmdlFile::read(&tracer_player_file("fcl_hair.fmdl")).unwrap();
    rewrite_texture_paths(&mut file, |path| {
        if path.file_name == "shirt.dds" {
            path.file_name = file_name.to_owned();
            path.directory = directory.to_owned();
        }
    })
    .unwrap();
    file.write()
}

/// The game's own Common texture folder, which names no team.
const GAME_COMMON_DIRECTORY: &str = "/Assets/pes16/model/character/common/sourceimages/";
/// The team's Common texture folder, `team_folder` as a model spells it: a texture there
/// that the export does not hold is looked for, where one in the game's own
/// (`GAME_COMMON_DIRECTORY`) is not.
const TEAM_COMMON_DIRECTORY: &str =
    "/Assets/pes16/model/character/common/team_folder/sourceimages/";

// TC-TEX-11
#[test]
fn a_common_model_s_texture_a_link_of_the_player_s_stands_for_is_in_the_team_s_common_output() {
    // A midcup export: the player's `hair.dds.common` stands for a texture an earlier
    // installed CPK holds in the team's Common output, and `Common/` does not.
    let sandbox = Sandbox::new("cmn_model_linked_texture");
    let export = "exports/co Midcup Hair";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{player}/legs.fmdl.common"), b"");
    sandbox.write(&format!("{player}/hair.dds.common"), b"");
    sandbox.write(
        &format!("{export}/Common/legs.fmdl"),
        &model_naming_game_hair(),
    );
    install_names(&sandbox, &["4cc_61_midcup.cpk", "4cc_62_midcup.cpk"]);
    install_cpk(
        &sandbox,
        "4cc_61_midcup.cpk",
        &[(
            &format!("{COMMON_TEXTURES}/hair.ftex"),
            b"compiled on an earlier day",
        )],
    );
    let settings = format!(
        "{}[team-compiler]\ncpk_name = \"4cc_62_midcup\"\n",
        pes21_settings(&sandbox)
    );

    let run = sandbox.run(&settings, &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Hair"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Common/legs.fmdl (file=legs.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=legs.fmdl.common)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_62_midcup.cpk"));
    let package = face_package(&entries);
    let hair = texture_directories(package.get("fcl_hair.fmdl").unwrap(), "hair.dds");
    assert!(!hair.is_empty());
    assert!(
        hair.iter().all(|directory| directory == COMMON_DIRECTORY),
        "{hair:?}"
    );
}

// TC-CMN-26
#[test]
fn a_common_kit_fallback_serves_the_reference_and_its_own_name_finds_nothing() {
    let sandbox = Sandbox::new("cmn_kit_fallback");
    let export = "exports/co Midcup Common Kits";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{player}/legs.fmdl.common"), b"");
    sandbox.write(
        &format!("{export}/Common/legs.fmdl"),
        &model_naming_in("pants_kitN.dds", TEAM_COMMON_DIRECTORY),
    );
    sandbox.write(
        &format!("{export}/Common/pants_kit1.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(&format!("{export}/Common/pants_kitF.dds"), &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Common Kits"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Common/legs.fmdl (file=legs.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=legs.fmdl.common)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let texture = |name: &str| entries.get(&format!("{COMMON_TEXTURES}/{name}.ftex"));
    // The `kitF` file is the set's fallback, written once as `pants_kitN`: `pants_kit1`
    // beside it, no `pants_kitF` or `pants_kit2` file.
    let kitn = texture("pants_kitN").expect("pants_kitN.ftex");
    assert_eq!(
        kitn,
        &ftex::dds_to_ftex(&tracer_kit(), ftex::ColorSpace::Normal).unwrap(),
        "the `kitF` file's conversion under `kitN`"
    );
    assert!(texture("pants_kit1").is_some(), "pants_kit1.ftex");
    for name in ["pants_kit2", "pants_kit3", "pants_kitF", "pants_kitf"] {
        assert!(texture(name).is_none(), "{name} among {:?}", entries.keys());
    }
}

// TC-CMN-30
#[test]
fn a_model_naming_a_common_kit_fallback_file_itself_finds_nothing() {
    let sandbox = Sandbox::new("cmn_kit_fallback_missing");
    let export = "exports/co Midcup Common Kits";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{player}/legs.fmdl.common"), b"");
    sandbox.write(
        &format!("{export}/Common/legs.fmdl"),
        &model_naming_in("pants_kitF.dds", TEAM_COMMON_DIRECTORY),
    );
    sandbox.write(&format!("{export}/Common/pants_kitF.dds"), &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Common Kits"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Common/legs.fmdl (file=legs.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=legs.fmdl.common)",
            "Info team_colors_missing [Keep] ()",
            "Warning fmdl_texture_not_found [Keep] at Players/05 - A (model=Common/legs.fmdl, texture=/Assets/pes16/model/character/common/714/sourceimages/pants_kitF.dds)",
        ]
    );
    assert_eq!(run.exit_code(), 0);
}

/// The tracer's boots model with its `shirt.dds` renamed `armor_bsm.dds` and pointed below
/// its folder at `./kitN/` (`texture_lookup::path_below`).
fn boots_naming_kit_folder() -> Vec<u8> {
    let mut file = FmdlFile::read(&tracer_player_file("boots.fmdl")).unwrap();
    rewrite_texture_paths(&mut file, |path| {
        if path.file_name == "shirt.dds" {
            path.file_name = "armor_bsm.dds".to_owned();
            path.directory = "./kitN/".to_owned();
        }
    })
    .unwrap();
    file.write()
}

// TC-MOD-72
#[test]
fn a_shared_folder_s_kit_folder_of_links_finds_their_sets_in_common() {
    let sandbox = Sandbox::new("cmn_kit_folder_links");
    let export = "exports/co Midcup Kits";
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{export}/Common/kit1/armor_bsm.dds"),
        &tracer_player_file("shirt.dds"),
    );
    let crocs = format!("{export}/Boots/Crocs");
    sandbox.write(&format!("{crocs}/boots.fmdl"), &boots_naming_kit_folder());
    sandbox.write(&format!("{crocs}/kitN/armor_bsm.dds.common"), b"");
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Kits"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // `Common/kit1/armor_bsm.dds`, the member the `kitN/` link reaches, packed once.
    let armor = "Asset/model/character/common/714/kit1/sourceimages/#windx11/armor_bsm.ftex";
    assert!(entries.contains_key(armor), "{:#?}", entries.keys());
    // Crocs' own boots names `armor_bsm` in the `kitN/` directory its link spells.
    let shared = "Asset/model/character/boots/k0625/#Win/boots.fpk";
    let package = fpk::FpkFile::read(&entries[shared]).unwrap();
    let directories = texture_directories(package.get("boots.fmdl").unwrap(), "armor_bsm.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| directory
                == "/Assets/pes16/model/character/common/714/kitN/sourceimages/"),
        "{directories:?}"
    );
}

// TC-CMN-21
#[test]
fn a_common_kit_folder_set_completes_and_the_model_names_the_kitn_reference() {
    let sandbox = Sandbox::new("cmn_kit_folder_set");
    let export = "exports/co Midcup Kits";
    for slot in ["p1", "p2", "p3"] {
        sandbox.write(&format!("{export}/Kits/{slot}/kit.dds"), &tracer_kit());
    }
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &model_naming_in("armor_bsm.dds", "./kitN/"),
    );
    sandbox.write(&format!("{player}/kitN/armor_bsm.dds.common"), b"");
    // Different images: kit 1 the tracer's shirt, kit 2 its kit.
    sandbox.write(
        &format!("{export}/Common/kit1/armor_bsm.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(
        &format!("{export}/Common/kit2/armor_bsm.dds"),
        &tracer_kit(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Kits"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Warning kit_variant_missing [Keep] at Common (texture=kitN/armor_bsm, kit=3, copied=kit1/armor_bsm)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let armor = |folder: &str| {
        format!("Asset/model/character/common/714/{folder}/sourceimages/#windx11/armor_bsm.ftex")
    };
    let kit1 = entries.get(&armor("kit1")).expect("kit1/ armor_bsm.ftex");
    let kit2 = entries.get(&armor("kit2")).expect("kit2/ armor_bsm.ftex");
    let kitn = entries.get(&armor("kitN")).expect("kitN/ armor_bsm.ftex");
    assert_eq!(kitn, kit1, "the fallback, the lowest variant, under `kitN`");
    assert_ne!(kitn, kit2);
    for path in entries.keys() {
        assert!(!path.contains("/kit3/"), "kit 3 gets no copy: {path}");
        assert!(!path.contains("/kitF/"), "kitF is never emitted: {path}");
    }
    // The face's model names `armor_bsm` in the set's `kitN/` Common directory.
    let package = face_package(&entries);
    let directories = texture_directories(package.get("face_high.fmdl").unwrap(), "armor_bsm.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| directory
                == "/Assets/pes16/model/character/common/714/kitN/sourceimages/"),
        "{directories:?}"
    );
}

// TC-CMN-26
#[test]
fn a_texture_link_naming_a_kit_stem_finds_its_members_in_common() {
    let sandbox = Sandbox::new("cmn_kit_stem_link");
    let export = "exports/co Midcup Kit Stem";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &model_naming_in("pants_kitN.dds", TEAM_COMMON_DIRECTORY),
    );
    sandbox.write(&format!("{player}/pants_kitN.dds.common"), b"");
    sandbox.write(
        &format!("{export}/Common/pants_kit1.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(&format!("{export}/Common/pants_kitF.dds"), &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Kit Stem"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let texture = |name: &str| entries.get(&format!("{COMMON_TEXTURES}/{name}.ftex"));
    assert!(texture("pants_kit1").is_some(), "{:#?}", entries.keys());
    // The `kitF` file, the set's fallback, under the reference's spelling.
    assert!(texture("pants_kitN").is_some(), "{:#?}", entries.keys());
}

// TC-STR-15
#[test]
fn a_texture_link_whose_target_is_not_in_common_drops_its_folder() {
    let sandbox = Sandbox::new("cmn_texture_link_missing");
    let export = "exports/co Midcup Hair";
    sandbox.write(
        &format!("{export}/Players/05 - A/face_high.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(&format!("{export}/Players/05 - A/hair.dds.common"), b"");
    sandbox.write(
        &format!("{export}/Common/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Hair"),
        [
            "Error common_link_missing [DropFolder] at Players/05 - A (link=hair.dds.common, path=Common/hair.dds)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-MOD-76
#[test]
fn a_shared_folder_s_texture_link_whose_target_is_missing_drops_it_and_its_linker() {
    // A shared folder's `.common` link is checked as a player folder's is: the target
    // nothing satisfies drops the folder, and its linker follows.
    let sandbox = Sandbox::new("cmn_shared_link_missing");
    let export = "exports/co Midcup Round";
    let round = format!("{export}/Faces/Round");
    sandbox.write(
        &format!("{round}/fcl_hair.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(&format!("{round}/hair.dds.common"), b"");
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(&format!("{player}/Round.face"), b"");

    let run = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Round"),
        [
            "Error common_link_missing [DropFolder] at Faces/Round (link=hair.dds.common, path=Common/hair.dds)",
            "Error link_target_dropped [DropFolder] at Players/05 - A (link=Round.face, target=Faces/Round, finding=common_link_missing)",
            "Info export_identified [Keep] (team=/co/, id=714)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
}

// TC-MOD-54
#[test]
fn a_fox_model_whose_mtl_is_a_common_file_converts_with_it() {
    let sandbox = Sandbox::new("cmn_fox_mtl_link");
    let export = "exports/co Midcup Body";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{player}/body.model"), &card_model());
    sandbox.write(&format!("{player}/body.mtl.common"), b"");
    sandbox.write(&format!("{export}/Common/body.mtl"), &card_materials());
    sandbox.write(&format!("{export}/Common/skin.dds"), &small_dds());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Body"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=body.model)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    // The converted model's materials, each its name and shader, read back with `fmdl`, and
    // the directories its paths of the set's one texture, `./skin.dds`, name.
    let materials = |sandbox: &Sandbox| -> (Vec<(String, String)>, Vec<String>) {
        let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
        let face = face_package(&entries);
        let fmdl = face.get("fcl_hair.fmdl").unwrap();
        let directories = texture_directories(fmdl, "skin.dds");
        // The set's one sampler made it into the converted model.
        assert!(!directories.is_empty());
        let materials = Model::from_file(&FmdlFile::read(fmdl).unwrap())
            .unwrap()
            .materials
            .into_iter()
            .map(|material| (material.name, material.shader))
            .collect();
        (materials, directories)
    };
    let (linked, directories) = materials(&sandbox);
    let set = MaterialSet::read(&card_materials()).unwrap();
    assert!(!set.materials.is_empty());
    for material in &set.materials {
        assert!(
            linked.iter().any(|(name, _)| *name == material.name),
            "{linked:?}"
        );
    }
    // The set is Common's, so the texture it names is Common's: the team's Common output.
    assert!(
        directories
            .iter()
            .all(|directory| directory == COMMON_DIRECTORY),
        "{directories:?}"
    );
    // The same as the set gives the model as a local `.mtl` of its folder.
    let local = Sandbox::new("cmn_fox_mtl_local");
    local.write(&format!("{player}/body.model"), &card_model());
    local.write(&format!("{player}/body.mtl"), &card_materials());
    local.write(&format!("{player}/skin.dds"), &small_dds());
    let run = local.run(&pes21_settings(&local), &["compile", "--no-deploy"]);
    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let (local_materials, local_directories) = materials(&local);
    assert_eq!(linked, local_materials);
    // With `skin.dds` in the player's folder too, the set's stem still resolves where the set
    // is, in `Common/`: the team's Common output, where the local set's resolves at his home.
    let both = Sandbox::new("cmn_fox_mtl_link_own_texture");
    both.write(&format!("{player}/body.model"), &card_model());
    both.write(&format!("{player}/body.mtl.common"), b"");
    both.write(&format!("{player}/skin.dds"), &small_dds());
    both.write(&format!("{export}/Common/body.mtl"), &card_materials());
    both.write(&format!("{export}/Common/skin.dds"), &small_dds());
    let run = both.run(&pes21_settings(&both), &["compile", "--no-deploy"]);
    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let (_, own_directories) = materials(&both);
    assert!(
        local_directories
            .iter()
            .all(|directory| directory == HOME_714_05),
        "{local_directories:?}"
    );
    assert_eq!(own_directories, directories);
}

// TC-CMN-13
#[test]
fn a_texture_below_a_common_subfolder_is_not_used_on_fox() {
    let sandbox = Sandbox::new("cmn_nested");
    let export = "exports/co Midcup Nested";
    sandbox.write(
        &format!("{export}/Players/05 - A/face_high.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(
        &format!("{export}/Common/sub/x.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Nested"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Warning file_not_used [Keep] (file=Common/sub/x.dds)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        entries.keys().all(|path| !path.ends_with("/x.ftex")),
        "{:#?}",
        entries.keys()
    );
    assert!(entries.contains_key(FACE_05), "{:#?}", entries.keys());
}

// TC-CMN-13
#[test]
fn a_common_subfolder_s_unlinked_texture_is_not_used_beside_a_linked_one() {
    let sandbox = Sandbox::new("cmn_nested_unlinked");
    let export = "co Midcup Nested";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(
        &format!("{player}/jessie/hair_high.fmdl"),
        &model_naming_game_hair(),
    );
    sandbox.write(&format!("{player}/jessie/hair.dds.common"), b"");
    sandbox.write(
        &format!("exports/{export}/Common/jessie/hair.dds"),
        &tracer_player_file("shirt.dds"),
    );
    // Beside the linked one, which no link at any depth names.
    sandbox.write(
        &format!("exports/{export}/Common/jessie/other.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    let findings = findings_of(&lines, export);
    assert!(
        findings.contains(&"Warning file_not_used [Keep] (file=Common/jessie/other.dds)"),
        "{lines:#?}"
    );
    assert!(
        findings
            .iter()
            .all(|line| !line.contains("hair.dds") || !line.contains("file_not_used")),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let hair = "Asset/model/character/common/714/jessie/sourceimages/#windx11/hair.ftex";
    assert!(entries.contains_key(hair), "{:#?}", entries.keys());
    assert!(
        entries.keys().all(|path| !path.ends_with("other.ftex")),
        "{:#?}",
        entries.keys()
    );
}

// TC-CMN-20
#[test]
fn a_common_link_below_a_subfolder_stands_for_the_common_file_at_its_path() {
    let sandbox = Sandbox::new("cmn_nested_link");
    let export = "co Midcup Nested";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/jessie/body.fmdl.common"), b"");
    sandbox.write(
        &format!("exports/{export}/Common/jessie/body.fmdl"),
        &clean_model(),
    );

    let fox = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    // No `file_not_used` names `Common/jessie/body.fmdl`: the link reads it.
    let lines = fox.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=jessie/body.fmdl.common)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(fox.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let package = face_package(&entries);
    let names = package_names(&entries[FACE_05]);
    assert!(
        names.contains(&"face_high.fmdl".to_owned()) && names.contains(&"fcl_hair.fmdl".to_owned()),
        "{names:?}"
    );
    // `body.fmdl` is the face's `fcl_hair` part (its name's fallback), beside `face_high`.
    assert_eq!(
        mesh_count(package.get("fcl_hair.fmdl").unwrap()),
        mesh_count(&clean_model())
    );
    assert_no_common_path(&entries);

    let pre_fox = sandbox.run(&pes17(&sandbox), &["compile", "--no-deploy"]);

    // The conversions' losses aside, nothing is missing or unread.
    let lines = pre_fox.messages();
    assert!(
        findings_of(&lines, export)
            .iter()
            .all(|line| !line.contains("file_not_used") && !line.contains("common_link_missing")),
        "{lines:#?}"
    );
    assert_eq!(pre_fox.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let common = entries_under(
        &entries,
        "common/character1/model/character/uniform/common/714/jessie/",
    );
    // The Common FMDL converted once, at its path below the team's Common output.
    let names: Vec<&str> = common.keys().copied().collect();
    assert_eq!(names, ["body.mtl", "oral_body_win32.model"]);
    let face = nested_entries(&entries[&face_cpk(5)]);
    let xml = String::from_utf8(face[&format!("{}face.xml", face_folder(5))].clone()).unwrap();
    assert!(
        xml.contains(
            "path=\"model/character/uniform/common/714/jessie/oral_body_*.model\" material=\"model/character/uniform/common/714/jessie/body.mtl\""
        ),
        "{xml}"
    );
}

// TC-CMN-25
#[test]
fn a_linked_common_model_s_textures_travel_with_it_and_resolve_beside_it() {
    let sandbox = Sandbox::new("cmn_nested_model_textures");
    let export = "co Midcup Nested";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/jessie/body.fmdl.common"), b"");
    sandbox.write(
        &format!("exports/{export}/Common/jessie/body.fmdl"),
        &model_naming_game_hair(),
    );
    // The model's `hair`, beside it; `Common/`'s own `hair.dds` of other bytes, which the
    // name resolves past to the nearer one.
    sandbox.write(
        &format!("exports/{export}/Common/jessie/hair.dds"),
        &small_dds(),
    );
    sandbox.write(
        &format!("exports/{export}/Common/hair.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert!(
        findings_of(&lines, export)
            .iter()
            .all(|line| !line.contains("file_not_used")),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // Packed once, at `jessie/`'s path below the team's Common output, converted from
    // `Common/jessie/hair.dds`, not `Common/`'s.
    let hair = "Asset/model/character/common/714/jessie/sourceimages/#windx11/hair.ftex";
    assert_eq!(
        entries.get(hair),
        Some(&ftex::dds_to_ftex(&small_dds(), ftex::ColorSpace::Normal).unwrap()),
        "{:#?}",
        entries.keys()
    );
    let package = face_package(&entries);
    let directories = texture_directories(package.get("fcl_hair.fmdl").unwrap(), "hair.dds");
    assert!(!directories.is_empty());
    assert!(
        directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/jessie/sourceimages/"),
        "{directories:?}"
    );
}

// TC-CMN-25
#[test]
fn a_common_path_into_a_subfolder_resolves_at_that_path_on_pes_21() {
    let sandbox = Sandbox::new("cmn_nested_common_path");
    let export = "co Midcup Nested";
    let player = format!("exports/{export}/Players/05 - A");
    // `face_high.mtl` names `skin` by a pre-Fox Common path into `Common/jessie/`, as a
    // member's `.mtl` written before the move spells it.
    sandbox.write(&format!("{player}/face_high.model"), &card_model());
    let mtl = String::from_utf8(pre_fox_fixture("cardhead_materials.mtl"))
        .unwrap()
        .replace("./texture.dds", "team_common_folder/jessie/skin.dds")
        .into_bytes();
    sandbox.write(&format!("{player}/face_high.mtl"), &mtl);
    sandbox.write(&format!("{player}/jessie/skin.dds.common"), b"");
    sandbox.write(
        &format!("exports/{export}/Common/jessie/skin.dds"),
        &small_dds(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let package = face_package(&entries);
    let directories = texture_directories(package.get("face_high.fmdl").unwrap(), "skin.dds");
    assert!(!directories.is_empty());
    assert!(
        directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/jessie/sourceimages/"),
        "{directories:?}"
    );
}

// TC-CMN-25
#[test]
fn a_linked_common_model_brings_the_textures_above_it_and_resolves_nearest_first() {
    let sandbox = Sandbox::new("cmn_deep_model_textures");
    let export = "co Midcup Nested";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/jessie/parts/body.fmdl.common"), b"");
    sandbox.write(
        &format!("exports/{export}/Common/jessie/parts/body.fmdl"),
        &model_naming_game("skin.dds"),
    );
    // `skin` resolves nearest first from `parts/` up to `Common/`: `jessie/`'s own, which
    // the link brings, not `Common/`'s of other bytes.
    sandbox.write(
        &format!("exports/{export}/Common/jessie/skin.dds"),
        &small_dds(),
    );
    sandbox.write(
        &format!("exports/{export}/Common/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert!(
        findings_of(&lines, export)
            .iter()
            .all(|line| !(line.contains("file_not_used") && line.contains("jessie/skin.dds"))),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // Packed once, at `jessie/`'s path below the team's Common output, converted from
    // `Common/jessie/skin.dds`, not `Common/`'s.
    let skin = "Asset/model/character/common/714/jessie/sourceimages/#windx11/skin.ftex";
    assert_eq!(
        entries.get(skin),
        Some(&ftex::dds_to_ftex(&small_dds(), ftex::ColorSpace::Normal).unwrap()),
        "{:#?}",
        entries.keys()
    );
    let package = face_package(&entries);
    let directories = texture_directories(package.get("fcl_hair.fmdl").unwrap(), "skin.dds");
    assert!(!directories.is_empty());
    assert!(
        directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/jessie/sourceimages/"),
        "{directories:?}"
    );
}

// TC-CMN-25
#[test]
fn a_common_model_converted_with_the_player_s_own_mtl_resolves_beside_the_model() {
    let sandbox = Sandbox::new("cmn_model_own_mtl_nested");
    let export = "co Midcup Nested";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/jessie/body.model.common"), b"");
    // His own `jessie/body.mtl`, which the search prefers to `Common/`'s, names `skin`,
    // which he does not hold: the stem still finds the Common model's own texture, in
    // `jessie/`, not `Common/`'s root's of other bytes.
    sandbox.write(&format!("{player}/jessie/body.mtl"), &card_materials());
    sandbox.write(
        &format!("exports/{export}/Common/jessie/body.model"),
        &card_model(),
    );
    sandbox.write(
        &format!("exports/{export}/Common/jessie/body.mtl"),
        &card_materials(),
    );
    sandbox.write(
        &format!("exports/{export}/Common/jessie/skin.dds"),
        &small_dds(),
    );
    sandbox.write(
        &format!("exports/{export}/Common/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=jessie/body.model.common)",
            "Info team_colors_missing [Keep] ()",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let skin = "Asset/model/character/common/714/jessie/sourceimages/#windx11/skin.ftex";
    assert_eq!(
        entries.get(skin),
        Some(&ftex::dds_to_ftex(&small_dds(), ftex::ColorSpace::Normal).unwrap()),
        "{:#?}",
        entries.keys()
    );
    let package = face_package(&entries);
    let directories = texture_directories(package.get("fcl_hair.fmdl").unwrap(), "skin.dds");
    assert!(!directories.is_empty());
    assert!(
        directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/jessie/sourceimages/"),
        "{directories:?}"
    );
}

// TC-CMN-25
#[test]
fn a_player_s_mtl_paired_with_his_own_model_does_not_resolve_in_common() {
    // `face_high.model` is his own: the Common fallback is a linked Common `.model`'s
    // alone, so `skin` is missing though `Common/skin.dds` sits there.
    let sandbox = Sandbox::new("cmn_own_model_common_texture");
    let export = "co Midcup Nested";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.model"), &card_model());
    sandbox.write(&format!("{player}/face_high.mtl"), &card_materials());
    sandbox.write(
        &format!("exports/{export}/Common/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Warning mtl_texture_not_found [Keep] at Players/05 - A (file=face_high.mtl, texture=./skin.dds, materials=card)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
}

// TC-MOD-102
#[test]
fn a_common_model_s_team_folder_directory_names_the_team_s_common_output() {
    let sandbox = Sandbox::new("cmn_fmdl_team_folder");
    let export = "exports/co Midcup Nested";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/legs.fmdl.common"), b"");
    // `Common/legs.fmdl` names `hair` in `team_folder`, the Studio spelling of the team's
    // Common directory; the export holds no `hair`.
    sandbox.write(
        &format!("{export}/Common/legs.fmdl"),
        &model_naming_in(
            "hair.dds",
            "/Assets/pes16/model/character/common/team_folder/sourceimages/",
        ),
    );
    install_names(
        &sandbox,
        &[
            "4cc_61_midcup.cpk",
            "4cc_62_midcup.cpk",
            "4cc_63_midcup.cpk",
        ],
    );
    install_cpk(
        &sandbox,
        "4cc_61_midcup.cpk",
        &[(
            "Asset/model/character/common/714/sourceimages/#windx11/hair.ftex",
            b"compiled on an earlier day",
        )],
    );
    let settings = format!(
        "{}[team-compiler]\ncpk_name = \"4cc_62_midcup\"\n",
        pes21_settings(&sandbox)
    );

    let run = sandbox.run(&settings, &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert!(
        findings_of(&lines, "co Midcup Nested").iter().all(|line| {
            !line.contains("fmdl_texture_not_found") && !line.contains("mtl_texture_not_found")
        }),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_62_midcup.cpk"));
    let package = face_package(&entries);
    let directories = texture_directories(package.get("fcl_hair.fmdl").unwrap(), "hair.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| directory == "/Assets/pes16/model/character/common/714/sourceimages/"),
        "{directories:?}"
    );
}

/// `clean_model()` with the texture `from` renamed `name` and pointed at `directory`.
fn clean_model_naming_in(from: &str, name: &str, directory: &str) -> Vec<u8> {
    clean_model_naming_each(&[(from, name, directory)])
}

/// `clean_model()` with each `(from, name, directory)` of `entries` renamed and pointed
/// as given; a `from` the model does not name panics, so a stale argument fails the
/// test instead of passing it vacuously.
fn clean_model_naming_each(entries: &[(&str, &str, &str)]) -> Vec<u8> {
    let mut file = FmdlFile::read(&clean_model()).unwrap();
    let mut named = BTreeSet::new();
    rewrite_texture_paths(&mut file, |path| {
        if let Some((_, name, directory)) =
            entries.iter().find(|(from, _, _)| path.file_name == *from)
        {
            named.insert(path.file_name.clone());
            path.file_name = (*name).to_owned();
            path.directory = (*directory).to_owned();
        }
    })
    .unwrap();
    for (from, _, _) in entries {
        assert!(named.contains(*from), "{from} is no texture of the model");
    }
    file.write()
}

// TC-MOD-103
#[test]
fn an_fmdl_common_directory_naming_anything_but_team_folder_or_sourceimages_is_refused() {
    let sandbox = Sandbox::new("mod_fmdl_common_invalid");
    let export = "exports/co Midcup Nested";
    let dir = |folder: &str| format!("/Assets/pes16/model/character/common/{folder}/sourceimages/");
    // Slot 05's boots name `team_kit` in `common/000/` (the community's placeholder,
    // a stand-in's entry checked alike), slot 06's face `hair` in `common/biz/`
    // (a team's name), `Common/legs.fmdl` `skin` in `common/XXX/`, and slot 07's
    // `zero` in `sourceimages/` and `skin` in `Team_Folder` in any letter case: the
    // two admissible folders, which no finding names.
    sandbox.write(
        &format!("{export}/Players/05 - A/boots.fmdl"),
        &clean_model_naming_in("team_kit.dds", "team_kit.dds", &dir("000")),
    );
    sandbox.write(
        &format!("{export}/Players/06 - B/face_high.fmdl"),
        &clean_model_naming_in("dummy_nrm.dds", "hair.dds", &dir("biz")),
    );
    sandbox.write(
        &format!("{export}/Players/07 - C/face_high.fmdl"),
        &clean_model_naming_each(&[
            ("team_kit.dds", "zero.dds", GAME_COMMON_DIRECTORY),
            (
                "dummy_nrm.dds",
                "skin.dds",
                "/Assets/pes16/model/character/common/Team_Folder/sourceimages/",
            ),
        ]),
    );
    sandbox.write(
        &format!("{export}/Common/legs.fmdl"),
        &clean_model_naming_in("dummy_nrm.dds", "skin.dds", &dir("XXX")),
    );
    sandbox.write(&format!("{export}/Players/08 - D/legs.fmdl.common"), b"");

    let dir_000 = dir("000");
    let dir_biz = dir("biz");
    let dir_xxx = dir("XXX");
    for (settings, version) in [(pes21_settings(&sandbox), 21), (pes17(&sandbox), 17)] {
        let run = sandbox.run(&settings, &["check"]);

        let lines = run.messages();
        assert_eq!(
            findings_of(&lines, "co Midcup Nested"),
            [
                format!(
                    "Error fmdl_common_path_invalid [DropFolder] at Players/05 - A \
                     (model=boots.fmdl, texture={dir_000}team_kit.dds, directory={dir_000})"
                ),
                format!(
                    "Error fmdl_common_path_invalid [DropFolder] at Players/06 - B \
                     (model=face_high.fmdl, texture={dir_biz}hair.dds, directory={dir_biz})"
                ),
                format!(
                    "Error fmdl_common_path_invalid [DropFile] at Common/legs.fmdl \
                     (model=legs.fmdl, texture={dir_xxx}skin.dds, directory={dir_xxx})"
                ),
                "Error link_target_dropped [DropFolder] at Players/08 - D \
                 (link=legs.fmdl.common, target=Common/legs.fmdl, finding=fmdl_common_path_invalid)"
                    .to_owned(),
                "Info export_identified [Keep] (team=/co/, id=714)".to_owned(),
            ],
            "PES {version}: {lines:#?}"
        );
        assert_eq!(run.exit_code(), 1, "PES {version}");
    }
}

// TC-MOD-106
#[test]
fn a_model_naming_the_kit_stand_in_s_old_spelling_is_refused() {
    let sandbox = Sandbox::new("mod_dummy_kit_old");
    let export = "exports/co Midcup Nested";
    // Slot 05's face names `dummy_kit` in `team_common_folder/` and slot 06's
    // `Dummy_GK_Kit_Back` in `./`, the stand-in's old spelling, the game's, in any
    // letter case; slot 07's `.mtl` names `team_common_folder/dummy_kit_srm.dds`; and
    // slot 08's `dummy_nrm` and `dummy_kitten`, the game's own `dummy_` stems and no
    // stand-in, which no finding names.
    sandbox.write(
        &format!("{export}/Players/05 - A/face_high.fmdl"),
        &clean_model_naming_in("team_kit.dds", "dummy_kit.dds", "team_common_folder/"),
    );
    sandbox.write(
        &format!("{export}/Players/06 - B/face_high.fmdl"),
        &clean_model_naming_in("dummy_nrm.dds", "Dummy_GK_Kit_Back", "./"),
    );
    sandbox.write(
        &format!("{export}/Players/07 - C/face_high.model"),
        &card_model(),
    );
    let mtl = String::from_utf8(pre_fox_fixture("cardhead_materials.mtl"))
        .unwrap()
        .replace("./texture.dds", "team_common_folder/dummy_kit_srm.dds");
    sandbox.write(
        &format!("{export}/Players/07 - C/face_high.mtl"),
        mtl.as_bytes(),
    );
    sandbox.write(
        &format!("{export}/Players/08 - D/face_high.fmdl"),
        &clean_model_naming_each(&[
            ("team_kit.dds", "dummy_kitten.dds", "./"),
            ("dummy_nrm.dds", "dummy_nrm.dds", "./"),
        ]),
    );

    for (settings, version) in [(pes21_settings(&sandbox), 21), (pes17(&sandbox), 17)] {
        let run = sandbox.run(&settings, &["check"]);

        let lines = run.messages();
        assert_eq!(
            findings_of(&lines, "co Midcup Nested"),
            [
                "Error dummy_kit_name_old [DropFolder] at Players/05 - A \
                 (model=face_high.fmdl, texture=team_common_folder/dummy_kit.dds)",
                "Error dummy_kit_name_old [DropFolder] at Players/06 - B \
                 (model=face_high.fmdl, texture=./Dummy_GK_Kit_Back)",
                "Error dummy_kit_name_old [DropFolder] at Players/07 - C \
                 (file=face_high.mtl, texture=team_common_folder/dummy_kit_srm.dds)",
                "Info export_identified [Keep] (team=/co/, id=714)",
            ],
            "PES {version}: {lines:#?}"
        );
        assert_eq!(run.exit_code(), 1, "PES {version}");
    }
    // The finding's text says the stand-in is written `team_kit`, and that the
    // upgrader respells it.
    let help = team_compiler::Tool::new().help();
    let row = help
        .messages
        .iter()
        .find(|row| row.code == "dummy_kit_name_old")
        .unwrap();
    for word in ["`team_kit`", "export-upgrader upgrade", "upgrade-models"] {
        assert!(
            row.when.contains(word) || row.effect.contains(word),
            "{word}: {} / {}",
            row.when,
            row.effect
        );
    }
}

// TC-MOD-104
#[test]
fn an_fmdl_s_short_common_directories_compile_as_their_long_forms() {
    let sandbox = Sandbox::new("mod_fmdl_short_forms");
    let export = "exports/co Midcup Nested";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{export}/Common/hair.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(
        &format!("{export}/Common/jessie/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );
    // The model's three textures named in the `.mtl`'s two short forms,
    // `team_common_folder/` in any letter case and `common_folder/`, or in
    // their long forms: the twins of one model, compiled identically.
    let spellings = [
        [
            ("team_kit.dds", "hair.dds", "team_common_folder/"),
            ("dummy_nrm.dds", "skin.dds", "Team_Common_Folder/jessie/"),
            ("dummy_srm.dds", "zero.dds", "common_folder/"),
        ],
        [
            ("team_kit.dds", "hair.dds", TEAM_COMMON_DIRECTORY),
            (
                "dummy_nrm.dds",
                "skin.dds",
                "/Assets/pes16/model/character/common/team_folder/jessie/sourceimages/",
            ),
            ("dummy_srm.dds", "zero.dds", GAME_COMMON_DIRECTORY),
        ],
    ];
    for (settings, version) in [(pes21_settings(&sandbox), 21), (pes17(&sandbox), 17)] {
        let mut twins = Vec::new();
        for entries in &spellings {
            sandbox.write(
                &format!("{player}/face_high.fmdl"),
                &clean_model_naming_each(entries),
            );
            let run = sandbox.run(&settings, &["compile", "--no-deploy"]);
            let lines = run.messages();
            assert_eq!(run.exit_code(), 0, "PES {version}: {lines:#?}");
            let findings: Vec<String> = findings_of(&lines, "co Midcup Nested")
                .into_iter()
                .map(str::to_owned)
                .collect();
            let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
            twins.push((findings, entries));
        }
        let [(short_findings, short), (long_findings, long)] = twins.try_into().unwrap();
        assert_eq!(short_findings, long_findings, "PES {version}");
        assert_eq!(
            short.keys().collect::<Vec<_>>(),
            long.keys().collect::<Vec<_>>(),
            "PES {version} entry names"
        );
        // The texture paths every model names, `.mtl` files' on pre-Fox: the
        // same on both runs (not byte-equal files: the rewrite keeps a changed
        // path's old strings).
        let paths = |entries: &BTreeMap<String, Vec<u8>>| -> BTreeMap<String, Vec<String>> {
            let mut named = BTreeMap::new();
            if version == 17 {
                for (name, bytes) in nested_entries(&entries[&face_cpk(5)]) {
                    if name.ends_with(".mtl") {
                        named.insert(name, sampler_paths(&bytes));
                    }
                }
            } else {
                let package = face_package(entries);
                for (name, bytes) in package.entries() {
                    if name.ends_with(".fmdl") {
                        let file = FmdlFile::read(bytes).unwrap();
                        named.insert(
                            name.to_owned(),
                            texture_paths(&file)
                                .unwrap()
                                .into_iter()
                                .map(|path| format!("{}{}", path.directory, path.file_name))
                                .collect(),
                        );
                    }
                }
            }
            named
        };
        let short_paths = paths(&short);
        let long_paths = paths(&long);
        assert_eq!(short_paths, long_paths, "PES {version} texture paths");
        if version == 21 {
            assert_eq!(
                short_paths["face_high.fmdl"],
                [
                    "/Assets/pes16/model/character/common/714/sourceimages/hair.dds",
                    "/Assets/pes16/model/character/common/714/jessie/sourceimages/skin.dds",
                    "/Assets/pes16/model/character/common/sourceimages/zero.dds",
                ]
            );
        }
    }
}

// TC-MOD-105
#[test]
fn an_fmdl_directory_with_anything_after_common_folder_or_starting_common_is_refused() {
    let sandbox = Sandbox::new("mod_fmdl_short_invalid");
    let export = "exports/co Midcup Nested";
    // Slot 05's face names `hair` below `common_folder/`, which names the
    // game's own Common alone; slot 06's names it in `common/`, a CPK's
    // root folder.
    sandbox.write(
        &format!("{export}/Players/05 - A/face_high.fmdl"),
        &clean_model_naming_in("team_kit.dds", "hair.dds", "common_folder/jessie/"),
    );
    sandbox.write(
        &format!("{export}/Players/06 - B/face_high.fmdl"),
        &clean_model_naming_in(
            "team_kit.dds",
            "hair.dds",
            "common/team_folder/sourceimages/",
        ),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Nested"),
        [
            "Error fmdl_common_path_invalid [DropFolder] at Players/05 - A \
             (model=face_high.fmdl, texture=common_folder/jessie/hair.dds, \
             directory=common_folder/jessie/)",
            "Error fmdl_common_path_invalid [DropFolder] at Players/06 - B \
             (model=face_high.fmdl, texture=common/team_folder/sourceimages/hair.dds, \
             directory=common/team_folder/sourceimages/)",
            "Info export_identified [Keep] (team=/co/, id=714)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    // The finding's text says how the team's Common is written there, and
    // that the upgrader respells an `.fmdl` that names an old folder.
    let help = team_compiler::Tool::new().help();
    let row = help
        .messages
        .iter()
        .find(|row| row.code == "fmdl_common_path_invalid")
        .unwrap();
    for word in ["`team_common_folder/`", "upgrade-models"] {
        assert!(
            row.when.contains(word) || row.effect.contains(word),
            "{word}: {} / {}",
            row.when,
            row.effect
        );
    }
}

// TC-CMN-23
#[test]
fn a_common_mtl_names_a_texture_below_its_own_directory_on_pes_17() {
    let sandbox = Sandbox::new("cmn_nested_mtl_sub_texture");
    let export = "co Midcup Nested";
    write_slot_05_face(&sandbox, export);
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/jessie/hair_high.model.common"), b"");
    let common = format!("exports/{export}/Common");
    sandbox.write(&format!("{common}/jessie/hair_high.model"), &card_model());
    sandbox.write(
        &format!("{common}/jessie/hair_high.mtl"),
        &materials_naming("sub/hair"),
    );
    sandbox.write(&format!("{common}/jessie/sub/hair.dds"), &small_dds());
    // Of other bytes than `sub/hair.dds`, which `hair` must resolve to: the `.mtl` names a
    // path below its own directory, which a lookup among `jessie/`'s own would point wrong.
    sandbox.write(
        &format!("{common}/jessie/hair.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let entries = compile_pes17(&sandbox, export, &CLEAN);

    // The Common `.mtl`, packed at its own path, points `hair` at the texture below its
    // directory, which the Common output holds at that path.
    let common = entries_under(
        &entries,
        "common/character1/model/character/uniform/common/714/jessie/",
    );
    assert_eq!(
        sampler_paths(common["hair_high.mtl"]),
        ["model/character/uniform/common/714/jessie/sub/hair.dds"]
    );
    assert!(
        common.contains_key("sub/hair.dds") && common.contains_key("hair.dds"),
        "{common:?}"
    );
    let face = nested_entries(&entries[&face_cpk(5)]);
    let xml = String::from_utf8(face[&format!("{}face.xml", face_folder(5))].clone()).unwrap();
    assert!(
        xml.contains(
            "path=\"model/character/uniform/common/714/jessie/oral_hair_high_*.model\" material=\"model/character/uniform/common/714/jessie/hair_high.mtl\""
        ),
        "{xml}"
    );
}

// TC-CMN-22
#[test]
fn a_texture_link_below_a_subfolder_stands_for_the_common_texture_at_its_path() {
    let sandbox = Sandbox::new("cmn_nested_texture_link");
    let export = "co Midcup Nested";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    // `jessie/hair_high.fmdl` naming `hair`, and the link to `Common/jessie/hair.dds`.
    sandbox.write(
        &format!("{player}/jessie/hair_high.fmdl"),
        &model_naming_game_hair(),
    );
    sandbox.write(&format!("{player}/jessie/hair.dds.common"), b"");
    sandbox.write(
        &format!("exports/{export}/Common/jessie/hair.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let fox = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = fox.messages();
    let findings = findings_of(&lines, export);
    assert!(
        findings
            .iter()
            .all(|line| !line.contains("file_type_disallowed") && !line.contains("file_not_used")),
        "{lines:#?}"
    );
    assert_eq!(fox.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // The Common texture once, at `jessie/`'s own path below the team's Common output.
    let hair = "Asset/model/character/common/714/jessie/sourceimages/#windx11/hair.ftex";
    assert!(entries.contains_key(hair), "{:#?}", entries.keys());
    let package = face_package(&entries);
    let directories = texture_directories(package.get("hair_high.fmdl").unwrap(), "hair.dds");
    assert!(!directories.is_empty());
    assert!(
        directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/jessie/sourceimages/"),
        "{directories:?}"
    );

    let pre_fox = sandbox.run(&pes17(&sandbox), &["compile", "--no-deploy"]);

    let lines = pre_fox.messages();
    assert!(
        findings_of(&lines, export)
            .iter()
            .all(|line| !line.contains("file_not_used") && !line.contains("common_link_missing")),
        "{lines:#?}"
    );
    assert_eq!(pre_fox.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // `Common/jessie/hair.dds` at its own path below the team's Common output, which the
    // converted model's `.mtl` names.
    let hair = "common/character1/model/character/uniform/common/714/jessie/hair.dds";
    assert!(entries.contains_key(hair), "{:#?}", entries.keys());
    let face = nested_entries(&entries[&face_cpk(5)]);
    let mtl = &face[&format!("{}jessie/hair_high.mtl", face_folder(5))];
    assert!(
        sampler_paths(mtl)
            .contains(&"model/character/uniform/common/714/jessie/hair.dds".to_owned()),
        "{:?}",
        sampler_paths(mtl)
    );
}

// TC-CMN-22
#[test]
fn a_texture_link_s_path_below_common_is_spelled_as_common_spells_it() {
    let sandbox = Sandbox::new("cmn_nested_texture_case");
    let export = "co Midcup Nested";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(
        &format!("{player}/jessie/hair_high.fmdl"),
        &model_naming_game_hair(),
    );
    sandbox.write(&format!("{player}/jessie/hair.dds.common"), b"");
    // The link spells `jessie/hair`, `Common/` `Jessie/Hair`: one spelling, `Common/`'s.
    sandbox.write(
        &format!("exports/{export}/Common/Jessie/Hair.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let hair = "Asset/model/character/common/714/Jessie/sourceimages/#windx11/Hair.ftex";
    assert!(entries.contains_key(hair), "{:#?}", entries.keys());
    let package = face_package(&entries);
    let directories = texture_directories(package.get("hair_high.fmdl").unwrap(), "hair.dds");
    assert!(!directories.is_empty());
    assert!(
        directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/Jessie/sourceimages/"),
        "{directories:?}"
    );
}

// TC-CMN-22
#[test]
fn a_subfolder_s_texture_link_comes_before_the_root_s_texture_of_its_stem_on_pes_17() {
    let sandbox = Sandbox::new("cmn_nested_texture_link_nearer");
    let export = "co Midcup Nested";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(
        &format!("{player}/jessie/hair_high.fmdl"),
        &model_naming_game_hair(),
    );
    sandbox.write(&format!("{player}/jessie/hair.dds.common"), b"");
    // The root's own `hair.dds` is nearer nothing of `jessie/`'s: the link stands for it.
    sandbox.write(&format!("{player}/hair.dds"), &small_dds());
    sandbox.write(
        &format!("exports/{export}/Common/jessie/hair.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes17(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let face = nested_entries(&entries[&face_cpk(5)]);
    let mtl = &face[&format!("{}jessie/hair_high.mtl", face_folder(5))];
    assert!(
        sampler_paths(mtl)
            .contains(&"model/character/uniform/common/714/jessie/hair.dds".to_owned()),
        "{:?}",
        sampler_paths(mtl)
    );
}

// TC-STR-15
#[test]
fn a_common_link_below_a_subfolder_with_only_a_direct_common_file_is_common_link_missing() {
    let sandbox = Sandbox::new("cmn_nested_link_missing");
    let export = "co Midcup Nested";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/jessie/body.fmdl.common"), b"");
    sandbox.write(
        &format!("exports/{export}/Common/body.fmdl"),
        &clean_model(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    // The player is dropped, his face with him, and the run stops for the error, no panic.
    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Error common_link_missing [DropFolder] at Players/05 - A (link=body.fmdl.common, path=Common/jessie/body.fmdl)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    assert!(!sandbox.root.join("output/4cc_99_test.cpk").exists());
}

// TC-CMN-14
#[test]
fn a_common_model_an_fmdl_of_its_stem_beats_is_not_checked_and_the_link_loads_the_fmdl() {
    let sandbox = Sandbox::new("cmn_beaten_model");
    let export = "exports/co Midcup Beaten";
    sandbox.write(&format!("{export}/Players/05 - A/boots.model.common"), b"");
    let boots = tracer_player_file("boots.fmdl");
    sandbox.write(&format!("{export}/Common/boots.fmdl"), &boots);
    // Four bytes no `.model` reader accepts.
    sandbox.write(&format!("{export}/Common/boots.model"), b"junk");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Beaten"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Common/boots.fmdl (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        package_names(&entries[BOOTS_05]),
        ["boots.fmdl", "boots.skl"]
    );
    let package = fpk::FpkFile::read(&entries[BOOTS_05]).unwrap();
    assert_eq!(
        mesh_count(package.get("boots.fmdl").unwrap()),
        mesh_count(&boots)
    );
}

// TC-CMN-14
#[test]
fn a_common_model_an_fmdl_the_deep_pass_drops_beats_is_dropped_with_it() {
    let sandbox = Sandbox::new("cmn_beaten_model_dropped");
    let export = "exports/co Midcup Beaten";
    sandbox.write(&format!("{export}/Players/05 - A/boots.fmdl.common"), b"");
    sandbox.write(
        &format!("{export}/Players/07 - B/face_high.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    // A link naming the beaten `.model`, which loads the FMDL of its stem.
    sandbox.write(&format!("{export}/Players/09 - C/boots.model.common"), b"");
    // Four bytes no FMDL reader accepts, beside a `.model` it beats, which nothing reads.
    sandbox.write(&format!("{export}/Common/boots.fmdl"), b"junk");
    sandbox.write(&format!("{export}/Common/boots.model"), &card_model());
    sandbox.write(&format!("{export}/Common/boots.mtl"), &card_materials());
    sandbox.write(&format!("{export}/Common/skin.dds"), &small_dds());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Beaten"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/07 - B (file=face_high.fmdl, count=1662)",
            "Error model_broken [DropFile] at Common/boots.fmdl (file=boots.fmdl, error=fmdl is truncated)",
            "Info common_model_beaten_dropped [DropFile] at Common/boots.model (file=Common/boots.model, winner=Common/boots.fmdl)",
            "Error link_target_dropped [DropFolder] at Players/05 - A (link=boots.fmdl.common, target=Common/boots.fmdl, finding=model_broken)",
            "Error link_target_dropped [DropFolder] at Players/09 - C (link=boots.model.common, target=Common/boots.model, finding=common_model_beaten_dropped)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    // No boots converted from the `.model`, nor any other: slot 07's face alone.
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        entries
            .keys()
            .all(|path| !path.starts_with("Asset/model/character/boots/")),
        "{:#?}",
        entries.keys()
    );
    assert_eq!(compiled_players(&sandbox), [71407]);
}

// TC-CMN-14
#[test]
fn a_beaten_common_model_is_not_dropped_for_a_dropped_file_of_its_stem_that_is_no_model() {
    let sandbox = Sandbox::new("cmn_beaten_model_texture_dropped");
    let export = "exports/co Midcup Beaten";
    sandbox.write(&format!("{export}/Players/05 - A/boots.fmdl.common"), b"");
    sandbox.write(&format!("{export}/Common/boots.fmdl"), &clean_model());
    sandbox.write(&format!("{export}/Common/boots.model"), &card_model());
    // A texture of the stem the deep pass drops (a PNG under a `.dds` name): it beats no
    // model, so the `.model` the FMDL beats stays, unread, and the link loads the FMDL.
    sandbox.write(
        &format!("{export}/Common/boots.dds"),
        &texture_fixture("kit.png"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Beaten"),
        [
            "Error texture_type_mismatch [DropFile] at Common/boots.dds (file=boots.dds)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        package_names(&entries[BOOTS_05]),
        ["boots.fmdl", "boots.skl"]
    );
}

// TC-TEX-14
#[test]
fn a_common_texture_the_decoder_refuses_is_dropped_alone_and_the_others_are_packed() {
    let sandbox = Sandbox::new("cmn_texture_unreadable");
    let export = "exports/co Midcup Studs";
    sandbox.write(&format!("{export}/Players/05 - A/studs.dds.common"), b"");
    sandbox.write(
        &format!("{export}/Common/studs.dds"),
        &tracer_player_file("shirt.dds"),
    );
    // Four bytes that open with no accepted format's signature: no header to read.
    sandbox.write(&format!("{export}/Common/boots.dds"), b"junk");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Studs"),
        [
            "Error texture_unreadable [DropFile] at Common/boots.dds (file=boots.dds)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let common: Vec<&str> = entries
        .keys()
        .map(String::as_str)
        .filter(|path| path.starts_with(COMMON_TEXTURES))
        .collect();
    assert_eq!(common, [format!("{COMMON_TEXTURES}/studs.ftex")]);
}

/// Writes the export `export`: slot 05 holding `model_name` (`model`) and `boots.mtl.common`,
/// and `Common/boots.mtl` holding bytes no `.mtl` reader accepts.
fn write_broken_common_mtl(sandbox: &Sandbox, export: &str, model_name: &str, model: &[u8]) {
    let player = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{player}/{model_name}"), model);
    sandbox.write(&format!("{player}/boots.mtl.common"), b"");
    sandbox.write(&format!("{export}/Common/boots.mtl"), b"not a material set");
}

// TC-CMN-34
#[test]
fn a_common_mtl_no_search_reads_is_not_checked_on_pes_21() {
    let sandbox = Sandbox::new("cmn_fox_mtl_unread");
    let export = "exports/co Midcup Boots";
    // An FMDL carries its materials: no search reads the Common `.mtl` his link names.
    write_broken_common_mtl(&sandbox, export, "boots.fmdl", &clean_model());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Boots"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        package_names(&entries[BOOTS_05]),
        ["boots.fmdl", "boots.skl"]
    );

    // A `.model` in its place is converted with the `.mtl` its search finds through the link:
    // that one is read, and its Error drops the file, after which his search finds none.
    let model = Sandbox::new("cmn_fox_mtl_searched");
    write_broken_common_mtl(&model, export, "boots.model", &card_model());
    let run = model.run(&pes21_settings(&model), &["compile", "--no-deploy"]);
    let lines = run.messages();
    let error = pes_model::format::mtl::MaterialSet::read(b"not a material set")
        .unwrap_err()
        .to_string();
    assert_eq!(
        findings_of(&lines, "co Midcup Boots"),
        [
            "Error model_material_undefined [DropFolder] at Players/05 - A (file=boots.model)",
            format!(
                "Error mtl_broken [DropFile] at Common/boots.mtl (file=boots.mtl, error={error})"
            )
            .as_str(),
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
}

// TC-CMN-34
#[test]
fn a_common_mtl_no_link_names_is_not_checked_when_a_model_pairs_locally_on_pes_21() {
    let sandbox = Sandbox::new("cmn_fox_mtl_local_pair");
    let export = "exports/co Midcup Boots";
    let player = format!("{export}/Players/05 - A");
    // His `.model` pairs with his own `.mtl`: his search never reaches `Common/`.
    sandbox.write(&format!("{player}/boots.model"), &card_model());
    sandbox.write(&format!("{player}/boots.mtl"), &card_materials());
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    sandbox.write(&format!("{export}/Common/stray.mtl"), b"not a material set");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Boots"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        package_names(&entries[BOOTS_05]),
        ["boots.fmdl", "boots.skl"]
    );
}

// TC-CMN-34
#[test]
fn a_common_mtl_a_link_s_search_falls_back_to_is_checked_on_pes_21() {
    let sandbox = Sandbox::new("cmn_fox_mtl_fallback");
    let export = "exports/co Midcup Legs";
    sandbox.write(&format!("{export}/Players/05 - A/legs.model.common"), b"");
    sandbox.write(&format!("{export}/Common/legs.model"), &card_model());
    // The search's first candidate, dropped: the folders' pass then lands on `materials.mtl`.
    sandbox.write(&format!("{export}/Common/legs.mtl"), b"not a material set");
    // The card's material set with its one material, `card`, renamed: the fallback defines
    // no material the card binds.
    let text = String::from_utf8(card_materials()).unwrap();
    assert!(text.contains("name=\"card\""), "{text}");
    sandbox.write(
        &format!("{export}/Common/materials.mtl"),
        text.replace("name=\"card\"", "name=\"sleeve\"").as_bytes(),
    );
    sandbox.write(&format!("{export}/Common/skin.dds"), &small_dds());

    let run = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    let lines = run.messages();
    let error = MaterialSet::read(b"not a material set")
        .unwrap_err()
        .to_string();
    assert_eq!(
        findings_of(&lines, "co Midcup Legs"),
        [
            "Error model_material_undefined [DropFolder] at Players/05 - A (file=legs.model.common, mtl=Common/materials.mtl, materials=card)",
            format!(
                "Error mtl_broken [DropFile] at Common/legs.mtl (file=legs.mtl, error={error})"
            )
            .as_str(),
            "Info export_identified [Keep] (team=/co/, id=714)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
}

// TC-MOD-83
#[test]
fn a_shared_folder_s_model_link_of_a_non_boots_name_drops_the_folder() {
    // A model link in a shared boots folder is named as a model file there is:
    // `legs` is no boots name, so the link is `fmdl_name_invalid` and the folder
    // and its linker drop. The linked Common model is still read and reports its
    // own findings.
    let sandbox = Sandbox::new("cmn_shared_link");
    let export = "exports/co Midcup Crocs";
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    sandbox.write(
        &format!("{export}/Boots/Crocs/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(&format!("{export}/Boots/Crocs/legs.fmdl.common"), b"");
    sandbox.write(
        &format!("{export}/Common/legs.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Crocs"),
        [
            "Error fmdl_name_invalid [DropFolder] at Boots/Crocs (file=legs.fmdl.common)",
            "Info fmdl_weights_not_normalized [Keep] at Common/legs.fmdl (file=legs.fmdl, count=1662)",
            "Error link_target_dropped [DropFolder] at Players/05 - A (link=Crocs.boots, target=Boots/Crocs, finding=fmdl_name_invalid)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
}

/// The clean model as a Common model of its own: its materials renamed with a `_common` tail
/// so it merges with the clean model itself with no material in common (two copies of one
/// FMDL would define every material twice, over textures in different places), and its
/// `team_kit.dds` renamed `file_name`, pointed at the game's own Common texture folder,
/// which names no team.
fn clean_model_naming(file_name: &str) -> Vec<u8> {
    let file = FmdlFile::read(&clean_model()).unwrap();
    let mut model = Model::from_file(&file).unwrap();
    for material in &mut model.materials {
        material.name.push_str("_common");
        for (_, texture) in &mut material.textures {
            if texture.file_name == "team_kit.dds" {
                texture.file_name = file_name.to_owned();
                texture.directory = GAME_COMMON_DIRECTORY.to_owned();
            }
        }
    }
    model.to_file().unwrap().write()
}

// TC-MOD-99
#[test]
fn a_shared_folder_holding_only_a_model_link_takes_its_wearer_s_id() {
    let sandbox = Sandbox::new("cmn_shared_model_link");
    let export = "exports/co Midcup Crocs";
    sandbox.write(
        &format!("{export}/Players/05 - A/face_high.fmdl"),
        &clean_model(),
    );
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    sandbox.write(&format!("{export}/Boots/Crocs/boots.fmdl.common"), b"");
    sandbox.write(&format!("{export}/Common/boots.fmdl"), &clean_model());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Crocs"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // Slot 05's exclusive boots id, Crocs's one wearer: its package holds the Common
    // model's meshes.
    let shared = "Asset/model/character/boots/k0625/#Win/boots.fpk";
    let package = fpk::FpkFile::read(&entries[shared]).unwrap();
    assert_eq!(
        mesh_count(package.get("boots.fmdl").unwrap()),
        mesh_count(&clean_model())
    );
    // The patch half: the same exclusive ID in his entry.
    assert_eq!(
        read_patch(&sandbox).teams[0].players[&5].boots_id,
        Some(625)
    );
}

// TC-MOD-83
#[test]
fn a_shared_folder_s_model_link_of_another_package_s_name_is_named_invalid() {
    let sandbox = Sandbox::new("cmn_shared_bad_model_link");
    let export = "exports/co Midcup Bad Link";
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    let crocs = format!("{export}/Boots/Crocs");
    sandbox.write(&format!("{crocs}/boots.fmdl"), &clean_model());
    sandbox.write(&format!("{crocs}/glove_l.fmdl.common"), b"");
    sandbox.write(&format!("{export}/Common/glove_l.fmdl"), &clean_model());
    // A clean player beside it, so the CPK is written and slot 05's absence observable.
    sandbox.write(
        &format!("{export}/Players/07 - B/face_high.fmdl"),
        &clean_model(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Bad Link"),
        [
            "Error fmdl_name_invalid [DropFolder] at Boots/Crocs (file=glove_l.fmdl.common)",
            "Error link_target_dropped [DropFolder] at Players/05 - A (link=Crocs.boots, target=Boots/Crocs, finding=fmdl_name_invalid)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    assert_eq!(
        compiled_players(&sandbox),
        [71407],
        "slot 05 is left out of the output"
    );
}

// TC-MOD-80
#[test]
fn a_combined_shared_face_s_model_link_brings_the_common_model_into_the_face() {
    let sandbox = Sandbox::new("cmn_shared_face_model_link");
    let export = "exports/co Midcup Round";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/Round.face"), b"");
    let round = format!("{export}/Faces/Round");
    sandbox.write(&format!("{round}/fcl_hair.fmdl"), &clean_model());
    sandbox.write(&format!("{round}/jessie/body.fmdl.common"), b"");
    sandbox.write(
        &format!("{export}/Common/jessie/body.fmdl"),
        &clean_model_naming("hair.dds"),
    );
    sandbox.write(&format!("{export}/Common/jessie/hair.dds"), &small_dds());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Round"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Faces/Round (file=jessie/body.fmdl.common)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=fcl_hair.fmdl)"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // `jessie/hair.dds`, which the link brings, packed at `jessie/`'s path below the team's
    // Common output.
    let hair = "Asset/model/character/common/714/jessie/sourceimages/#windx11/hair.ftex";
    assert_eq!(
        entries.get(hair),
        Some(&ftex::dds_to_ftex(&small_dds(), ftex::ColorSpace::Normal).unwrap()),
        "{:#?}",
        entries.keys()
    );
    let package = face_package(&entries);
    // The three models' meshes: his face_high's, and Round's `fcl_hair` merged with the
    // Common `body.fmdl` its link brings in.
    let meshes = mesh_count(&clean_model());
    assert_eq!(mesh_count(package.get("face_high.fmdl").unwrap()), meshes);
    assert_eq!(
        mesh_count(package.get("fcl_hair.fmdl").unwrap()),
        2 * meshes
    );
    // The model built from `body.fmdl` names `hair` in `jessie/`'s Common output directory.
    let directories = texture_directories(package.get("fcl_hair.fmdl").unwrap(), "hair.dds");
    assert!(!directories.is_empty());
    assert!(
        directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/jessie/sourceimages/"),
        "{directories:?}"
    );
}

// TC-MOD-81
#[test]
fn a_shared_folder_s_mtl_link_gives_its_model_the_common_material_set() {
    let sandbox = Sandbox::new("cmn_shared_mtl_link");
    let export = "exports/co Midcup Crocs";
    sandbox.write(
        &format!("{export}/Players/05 - A/face_high.fmdl"),
        &clean_model(),
    );
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    sandbox.write(&format!("{export}/Boots/Crocs/boots.model"), &card_model());
    sandbox.write(&format!("{export}/Boots/Crocs/boots.mtl.common"), b"");
    sandbox.write(&format!("{export}/Common/boots.mtl"), &card_materials());
    sandbox.write(&format!("{export}/Common/skin.dds"), &small_dds());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Crocs"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // Crocs' own output: its `boots.model` converted with the Common `.mtl` the link
    // stands for, naming `skin` in the team's Common texture directory.
    let shared = "Asset/model/character/boots/k0625/#Win/boots.fpk";
    let package = fpk::FpkFile::read(&entries[shared]).unwrap();
    let boots = package.get("boots.fmdl").unwrap();
    assert_eq!(mesh_count(boots), CONVERTED_CARD_MESHES);
    let directories = texture_directories(boots, "skin.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| directory == COMMON_DIRECTORY),
        "{directories:?}"
    );
}

// TC-MOD-82
#[test]
fn a_combined_shared_folder_s_mtl_link_gives_its_model_the_common_material_set() {
    let sandbox = Sandbox::new("cmn_combined_mtl_link");
    let export = "exports/co Midcup Round";
    sandbox.write(&format!("{export}/Players/05 - A/Round.face"), b"");
    let round = format!("{export}/Faces/Round");
    sandbox.write(&format!("{round}/fcl_hair.fmdl"), &clean_model());
    sandbox.write(&format!("{round}/boots.model"), &card_model());
    // `materials.mtl`, which the name-matched `boots.mtl.common` beats: `skin` would
    // resolve in Round, where nothing holds it, not in `Common/` as the linked `.mtl`'s.
    sandbox.write(&format!("{round}/materials.mtl"), &card_materials());
    sandbox.write(&format!("{round}/boots.mtl.common"), b"");
    sandbox.write(&format!("{export}/Common/boots.mtl"), &card_materials());
    sandbox.write(&format!("{export}/Common/skin.dds"), &small_dds());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Round"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // The boots of his exclusive id: Round's `boots.model` converted with the Common `.mtl`
    // its link stands for, naming `skin` in the team's Common texture directory.
    let package = fpk::FpkFile::read(&entries[BOOTS_05]).unwrap();
    let boots = package.get("boots.fmdl").unwrap();
    assert_eq!(mesh_count(boots), CONVERTED_CARD_MESHES);
    let directories = texture_directories(boots, "skin.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| directory == COMMON_DIRECTORY),
        "{directories:?}"
    );
}

// TC-CMN-35
#[test]
fn an_unlinked_common_model_builds_nothing() {
    // A Common model no link names is accepted and builds nothing.
    let unlinked = Sandbox::new("cmn_unlinked");
    let export = "exports/co Midcup Spare";
    unlinked.write(
        &format!("{export}/Players/05 - A/face_high.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    unlinked.write(
        &format!("{export}/Common/spare.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    unlinked.write(
        &format!("{export}/Common/spare.skl"),
        &tracer_player_file("fcl_hair.skl"),
    );
    let run = unlinked.run(&pes21_settings(&unlinked), &["compile", "--no-deploy"]);
    assert_eq!(
        findings_of(&run.messages(), "co Midcup Spare"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Common/spare.fmdl (file=spare.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&unlinked.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            FACE_05,
            FACE_05_FPKD,
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin"
        ]
    );
}
