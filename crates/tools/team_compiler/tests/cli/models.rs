//! `compile` over boots and gloves models: a player folder's own under the player's planned
//! ID, the skeleton packed with the boots, the folder's textures emitted once for every
//! package; a shared `Boots/` or `Gloves/` folder players link, compiled once under one of
//! the team's shared IDs, with `check` refusing an export needing more of them than the team
//! has; the merges: several parts under one name into one model, a shared folder a link
//! combines with the player's own model into the player's package, a shared `Faces/` folder
//! into the player's face; and a texture two of the player's sources hold, packed once or
//! resolved by its bytes.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use fmdl::ops::paths::texture_paths;
use fmdl::{FmdlFile, Model};

use crate::bins::{
    BOOTS_LIST, GLOVE_LIST, SHARED_IDS_714, install_tables, item_list, pairs_of, record,
};
use crate::common::Sandbox;
use crate::common_links::texture_directories;
use crate::compile::{cpk_entries, pes21_settings, pixels_cut_dds, tracer_kit, tracer_player_file};
use crate::conversion::HOME_714_05;
use crate::face_folders::assert_blank_face;
use crate::prefox_faces::{
    BOOTS_K0625, CLEAN, card_materials, card_model, compile_pes17, entries_under, materials_naming,
    small_dds,
};
use crate::textures::{hair_model_naming, tracer_model_pointing, tracer_model_renaming};
use crate::{TEAM_COLORS_MISSING, clean_model, command_args, findings_of};

/// The entry names of the FPK `bytes`.
pub(crate) fn package_names(bytes: &[u8]) -> Vec<String> {
    fpk::FpkFile::read(bytes)
        .unwrap()
        .entries()
        .map(|(name, _)| name.to_owned())
        .collect()
}

/// One of the bundled game skeletons, `resources/skeletons/<version>/body.skl`.
pub(crate) fn body_skl(version: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../resources/skeletons")
            .join(version)
            .join("body.skl"),
    )
    .unwrap()
}

/// Writes the tracer's face files into `folder` under their own names, and its boots and
/// gloves models under `boots_name`, `glove_l.fmdl` and `glove_r.fmdl`.
fn write_player(sandbox: &Sandbox, folder: &str, boots_name: &str) {
    for name in [
        "face_diff.bin",
        "fcl_hair.fmdl",
        "fcl_hair.skl",
        "fcl_hair_sim.fclo",
        "shirt.dds",
        "glove_l.fmdl",
        "glove_r.fmdl",
    ] {
        sandbox.write(&format!("{folder}/{name}"), &tracer_player_file(name));
    }
    sandbox.write(
        &format!("{folder}/{boots_name}"),
        &tracer_player_file("boots.fmdl"),
    );
}

/// Compiles the sandbox for PES 21, asserting the export `name` reports exactly `findings`
/// and the run exits with 0, and returns the CPK's entries by path.
pub(crate) fn compile_clean(
    sandbox: &Sandbox,
    name: &str,
    findings: &[&str],
) -> BTreeMap<String, Vec<u8>> {
    let run = sandbox.run(&pes21_settings(sandbox), &["compile", "--no-deploy"]);
    assert_eq!(findings_of(&run.messages(), name), findings);
    assert_eq!(run.exit_code(), 0);
    cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"))
}

// TC-MOD-01
#[test]
fn a_player_s_own_boots_and_gloves_compile_under_its_exclusive_id_with_the_textures_once() {
    let sandbox = Sandbox::new("mod_boots_gloves");
    write_player(
        &sandbox,
        "exports/co Midcup Models/Players/05 - A",
        "kit_boots.fmdl",
    );

    let entries = compile_clean(
        &sandbox,
        "co Midcup Models",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=kit_boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "Asset/model/character/boots/k0625/#Win/boots.fpk",
            "Asset/model/character/boots/k0625/#Win/boots.fpkd",
            "Asset/model/character/common/714/05 - A/sourceimages/#windx11/shirt.ftex",
            "Asset/model/character/face/real/71405/#Win/face.fpk",
            "Asset/model/character/face/real/71405/#Win/face.fpkd",
            "Asset/model/character/glove/g0625/#Win/glove.fpk",
            "Asset/model/character/glove/g0625/#Win/glove.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ]
    );
    assert_eq!(
        package_names(&entries["Asset/model/character/boots/k0625/#Win/boots.fpk"]),
        ["boots.fmdl", "boots.skl"]
    );
    assert_eq!(
        package_names(&entries["Asset/model/character/glove/g0625/#Win/glove.fpk"]),
        ["glove_l.fmdl", "glove_r.fmdl"]
    );
}

// TC-MOD-02
#[test]
fn boots_skl_is_the_skeleton_named_after_the_boots_model_or_the_bundled_pes_21_body_skl() {
    let boots_fpk = "Asset/model/character/boots/k0625/#Win/boots.fpk";

    let paired = Sandbox::new("mod_boots_skl_paired");
    write_player(
        &paired,
        "exports/co Midcup Paired/Players/05 - A",
        "kit_boots.fmdl",
    );
    // A real skeleton that differs from the bundled PES 21 one.
    let custom = body_skl("pes19");
    assert_ne!(custom, body_skl("pes21"));
    paired.write(
        "exports/co Midcup Paired/Players/05 - A/kit_boots.skl",
        &custom,
    );
    let entries = compile_clean(
        &paired,
        "co Midcup Paired",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=kit_boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );
    let package = fpk::FpkFile::read(&entries[boots_fpk]).unwrap();
    assert_eq!(package.get("boots.skl").unwrap(), custom);

    let bare = Sandbox::new("mod_boots_skl_bundled");
    write_player(
        &bare,
        "exports/co Midcup Bare/Players/05 - A",
        "kit_boots.fmdl",
    );
    let entries = compile_clean(
        &bare,
        "co Midcup Bare",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=kit_boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );
    let package = fpk::FpkFile::read(&entries[boots_fpk]).unwrap();
    assert_eq!(package.get("boots.skl").unwrap(), body_skl("pes21"));
}

// TC-MOD-86
#[test]
fn a_skeleton_named_like_its_model_but_the_case_pairs_too() {
    // `Kit_Boots.skl` names `kit_boots.fmdl`'s skeleton as the file system folds it.
    let sandbox = Sandbox::new("mod_boots_skl_case");
    write_player(
        &sandbox,
        "exports/co Midcup Case/Players/05 - A",
        "kit_boots.fmdl",
    );
    let custom = body_skl("pes19");
    assert_ne!(custom, body_skl("pes21"));
    sandbox.write(
        "exports/co Midcup Case/Players/05 - A/Kit_Boots.skl",
        &custom,
    );

    let entries = compile_clean(
        &sandbox,
        "co Midcup Case",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=kit_boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let package =
        fpk::FpkFile::read(&entries["Asset/model/character/boots/k0625/#Win/boots.fpk"]).unwrap();
    assert_eq!(package.get("boots.skl").unwrap(), custom);
}

// TC-MOD-86
#[test]
fn a_face_diff_bin_by_any_case_packs_as_face_diff_bin() {
    // `FACE_DIFF.BIN` is `face_diff.bin` as the file system folds it.
    let sandbox = Sandbox::new("mod_face_diff_case");
    let export = "exports/co Midcup Case";
    write_player(
        &sandbox,
        &format!("{export}/Players/05 - A"),
        "kit_boots.fmdl",
    );
    std::fs::remove_file(
        sandbox
            .root
            .join(format!("{export}/Players/05 - A/face_diff.bin")),
    )
    .unwrap();
    sandbox.write(
        &format!("{export}/Players/05 - A/Face_Diff.bin"),
        &tracer_player_file("face_diff.bin"),
    );

    let entries = compile_clean(
        &sandbox,
        "co Midcup Case",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=kit_boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let package =
        fpk::FpkFile::read(&entries["Asset/model/character/face/real/71405/#Win/face.fpk"])
            .unwrap();
    assert_eq!(
        package.get("face_diff.bin").unwrap(),
        tracer_player_file("face_diff.bin")
    );
}

// TC-MOD-86
#[test]
fn a_texture_stem_names_its_file_case_folded() {
    // `Shirt.dds` is the texture the model's `shirt` path names, folded alike.
    let sandbox = Sandbox::new("mod_stem_case");
    let export = "exports/co Midcup Case";
    write_player(
        &sandbox,
        &format!("{export}/Players/05 - A"),
        "kit_boots.fmdl",
    );
    std::fs::remove_file(
        sandbox
            .root
            .join(format!("{export}/Players/05 - A/shirt.dds")),
    )
    .unwrap();
    sandbox.write(
        &format!("{export}/Players/05 - A/Shirt.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let entries = compile_clean(
        &sandbox,
        "co Midcup Case",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=kit_boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let package =
        fpk::FpkFile::read(&entries["Asset/model/character/boots/k0625/#Win/boots.fpk"]).unwrap();
    let directories: Vec<String> =
        texture_paths(&FmdlFile::read(package.get("boots.fmdl").unwrap()).unwrap())
            .unwrap()
            .into_iter()
            .filter(|path| path.file_name == "shirt.dds")
            .map(|path| path.directory)
            .collect();
    assert!(!directories.is_empty());
    assert!(
        directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/05 - A/sourceimages/"),
        "{directories:?}"
    );
}

// TC-MOD-03
#[test]
fn a_folder_mapped_to_two_slots_emits_its_boots_under_both_ids_and_its_textures_once() {
    let sandbox = Sandbox::new("mod_two_slots");
    sandbox.write("exports/co Midcup Twice/players.txt", b"03 A\n07 A\n");
    for name in ["boots.fmdl", "shirt.dds"] {
        sandbox.write(
            &format!("exports/co Midcup Twice/Players/A/{name}"),
            &tracer_player_file(name),
        );
    }

    let entries = compile_clean(
        &sandbox,
        "co Midcup Twice",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/A (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "Asset/model/character/boots/k0623/#Win/boots.fpk",
            "Asset/model/character/boots/k0623/#Win/boots.fpkd",
            "Asset/model/character/boots/k0627/#Win/boots.fpk",
            "Asset/model/character/boots/k0627/#Win/boots.fpkd",
            "Asset/model/character/common/714/A/sourceimages/#windx11/shirt.ftex",
            "Asset/model/character/face/real/71403/#Win/face.fpk",
            "Asset/model/character/face/real/71403/#Win/face.fpkd",
            "Asset/model/character/face/real/71407/#Win/face.fpk",
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ]
    );
    assert_eq!(
        entries["Asset/model/character/boots/k0623/#Win/boots.fpk"],
        entries["Asset/model/character/boots/k0627/#Win/boots.fpk"],
        "one package under both ids"
    );
}

// TC-TEX-12
#[test]
fn a_texture_that_cannot_convert_drops_the_whole_folder_and_the_folder_beside_it_compiles() {
    let sandbox = Sandbox::new("mod_texture_failed");
    write_player(
        &sandbox,
        "exports/co Midcup Broken/Players/05 - A",
        "boots.fmdl",
    );
    // A header the deep pass reads, over pixel data cut short: the conversion fails.
    sandbox.write(
        "exports/co Midcup Broken/Players/05 - A/shirt.dds",
        &pixels_cut_dds(),
    );
    write_player(
        &sandbox,
        "exports/co Midcup Broken/Players/07 - B",
        "boots.fmdl",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    let findings = findings_of(&lines, "co Midcup Broken");
    assert_eq!(findings.len(), 9, "{findings:?}");
    assert_eq!(
        findings[..8],
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Info fmdl_weights_not_normalized [Keep] at Players/07 - B (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/07 - B (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/07 - B (file=glove_l.fmdl, count=2)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert!(
        findings[8].starts_with(
            "Error folder_pack_failed [DropFolder] at Players/05 - A (error=cannot convert shirt.dds"
        ),
        "{}",
        findings[8]
    );
    assert_eq!(run.exit_code(), 1);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "Asset/model/character/boots/k0627/#Win/boots.fpk",
            "Asset/model/character/boots/k0627/#Win/boots.fpkd",
            "Asset/model/character/common/714/07 - B/sourceimages/#windx11/shirt.ftex",
            "Asset/model/character/face/real/71407/#Win/face.fpk",
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
            "Asset/model/character/glove/g0627/#Win/glove.fpk",
            "Asset/model/character/glove/g0627/#Win/glove.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ],
        "nothing of slot 05 is in the CPK"
    );
}

/// Writes the shared boots folder `Boots/<name>` holding the tracer's boots model and texture,
/// with `skl` as its `boots.skl` when given, and a link to it in each of the player folders
/// `slots` (`Players/03 - P03/<name>.boots`), which hold nothing else.
fn write_shared_boots(
    sandbox: &Sandbox,
    export: &str,
    name: &str,
    skl: Option<&[u8]>,
    slots: &[u8],
) {
    let folder = format!("exports/{export}/Boots/{name}");
    sandbox.write(
        &format!("{folder}/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(
        &format!("{folder}/shirt.dds"),
        &tracer_player_file("shirt.dds"),
    );
    if let Some(skl) = skl {
        sandbox.write(&format!("{folder}/boots.skl"), skl);
    }
    for slot in slots {
        sandbox.write(
            &format!("exports/{export}/Players/{slot:02} - P{slot:02}/{name}.boots"),
            b"",
        );
    }
}

/// The `boots.skl` of the boots package at `path` in `entries`.
fn boots_skl(entries: &BTreeMap<String, Vec<u8>>, path: &str) -> Vec<u8> {
    fpk::FpkFile::read(&entries[path])
        .unwrap()
        .get("boots.skl")
        .unwrap()
        .to_vec()
}

/// The mesh count of the `boots.fmdl` of the boots package at `path` in `entries`.
pub(crate) fn boots_mesh_count(entries: &BTreeMap<String, Vec<u8>>, path: &str) -> usize {
    let package = fpk::FpkFile::read(&entries[path]).unwrap();
    let model = FmdlFile::read(package.get("boots.fmdl").unwrap()).unwrap();
    Model::from_file(&model).unwrap().meshes.len()
}

/// The mesh count of the tracer's boots model.
pub(crate) fn tracer_boots_mesh_count() -> usize {
    let model = FmdlFile::read(&tracer_player_file("boots.fmdl")).unwrap();
    Model::from_file(&model).unwrap().meshes.len()
}

// TC-MOD-07
#[test]
fn a_boots_link_beside_a_local_boots_model_combines_the_shared_folder_into_the_player_s_boots() {
    let sandbox = Sandbox::new("mod_link_combined");
    let export = "exports/co Midcup Combined";
    write_player(
        &sandbox,
        &format!("{export}/Players/05 - A"),
        "kit_boots.fmdl",
    );
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    sandbox.write(
        &format!("{export}/Boots/Crocs/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    // A texture of the shared folder's own, under a stem the player does not hold.
    sandbox.write(
        &format!("{export}/Boots/Crocs/sole.dds"),
        &tracer_player_file("shirt.dds"),
    );
    let k0625 = "Asset/model/character/boots/k0625/#Win/boots.fpk";
    let k0627 = "Asset/model/character/boots/k0627/#Win/boots.fpk";
    let sole = "Asset/model/character/common/714/05 - A/sourceimages/#windx11/sole.ftex";

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Combined"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=kit_boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
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
            "Asset/model/character/boots/k0625/#Win/boots.fpk",
            "Asset/model/character/boots/k0625/#Win/boots.fpkd",
            "Asset/model/character/common/714/05 - A/sourceimages/#windx11/shirt.ftex",
            sole,
            "Asset/model/character/face/real/71405/#Win/face.fpk",
            "Asset/model/character/face/real/71405/#Win/face.fpkd",
            "Asset/model/character/glove/g0625/#Win/glove.fpk",
            "Asset/model/character/glove/g0625/#Win/glove.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ],
        "no k0644: the shared folder is only a source of parts"
    );
    assert_eq!(
        boots_mesh_count(&entries, k0625),
        2 * tracer_boots_mesh_count(),
        "Crocs's meshes plus the local model's"
    );
    assert_eq!(boots_skl(&entries, k0625), body_skl("pes21"));

    // Slot 07 links Crocs plainly: Crocs compiles on its own too, as it is, textures beside
    // it, while slot 05's merge is unchanged. Slot 07 holds no face model: its face is blank.
    sandbox.write(&format!("{export}/Players/07 - B/Crocs.boots"), b"");
    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    assert_eq!(
        findings_of(&run.messages(), "co Midcup Combined"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=kit_boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=boots.fmdl)"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let added: Vec<&str> = entries
        .keys()
        .map(String::as_str)
        .filter(|path| !paths.contains(path))
        .collect();
    assert_eq!(
        added,
        [
            // Crocs's own output under slot 07's exclusive id, he being its one plain
            // wearer: no shared id, so no record and no `shared_ids_unrecorded`.
            k0627,
            "Asset/model/character/boots/k0627/#Win/boots.fpkd",
            "Asset/model/character/boots/k0627/#windx11/sole.ftex",
            "Asset/model/character/face/real/71407/#Win/face.fpk",
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
        ]
    );
    assert_eq!(boots_mesh_count(&entries, k0627), tracer_boots_mesh_count());
    assert_eq!(
        boots_mesh_count(&entries, k0625),
        2 * tracer_boots_mesh_count()
    );
    assert_eq!(
        entries[sole],
        entries["Asset/model/character/boots/k0627/#windx11/sole.ftex"]
    );
}

/// One of the bundled face templates, `resources/templates/<name>`.
pub(crate) fn template(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../resources/templates")
            .join(name),
    )
    .unwrap()
}

/// The face package of slot 05 in `entries`.
pub(crate) fn face_package(entries: &BTreeMap<String, Vec<u8>>) -> fpk::FpkFile {
    fpk::FpkFile::read(&entries["Asset/model/character/face/real/71405/#Win/face.fpk"]).unwrap()
}

// TC-MOD-12
#[test]
fn a_hair_model_alone_gets_the_bundled_face_diff_hair_simulation_and_body_skeleton() {
    let sandbox = Sandbox::new("mod_face_templates");
    sandbox.write(
        "exports/co Midcup Hair/Players/05 - A/fcl_hair.fmdl",
        &tracer_player_file("fcl_hair.fmdl"),
    );

    let entries = compile_clean(
        &sandbox,
        "co Midcup Hair",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let package = face_package(&entries);
    let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
    assert_eq!(
        names,
        [
            "face_diff.bin",
            "fcl_hair.fmdl",
            "fcl_hair_sim.fclo",
            "fcl_hair_sim.skl"
        ]
    );
    assert_eq!(
        package.get("face_diff.bin").unwrap(),
        template("face_diff.bin")
    );
    assert_eq!(
        package.get("fcl_hair_sim.fclo").unwrap(),
        template("fcl_hair_sim.fclo")
    );
    assert_eq!(package.get("fcl_hair_sim.skl").unwrap(), body_skl("pes21"));
}

// TC-MOD-12
#[test]
fn a_templates_face_diff_replaces_the_bundled_one_in_a_face_whose_folder_holds_none() {
    let sandbox = Sandbox::new("mod_face_template_override");
    // The tracer's own face diff, a real one other than the bundled template.
    let template_face_diff = tracer_player_file("face_diff.bin");
    assert_ne!(template_face_diff, template("face_diff.bin"));
    sandbox.write("data/templates/face_diff.bin", &template_face_diff);
    sandbox.write(
        "exports/co Midcup Hair/Players/05 - A/fcl_hair.fmdl",
        &tracer_player_file("fcl_hair.fmdl"),
    );

    let entries = compile_clean(
        &sandbox,
        "co Midcup Hair",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let package = face_package(&entries);
    assert!(
        package.get("face_diff.bin").unwrap() == template_face_diff,
        "the face diff is the templates folder's"
    );
    assert!(
        package.get("fcl_hair_sim.fclo").unwrap() == template("fcl_hair_sim.fclo"),
        "a resource with no file in the folder is the bundled one"
    );
}

// TC-MOD-13
#[test]
fn a_skeleton_paired_with_a_face_model_without_a_slot_is_reported_and_not_packed() {
    let sandbox = Sandbox::new("mod_skl_no_slot");
    sandbox.write(
        "exports/co Midcup Slot/Players/05 - A/face_high.fmdl",
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(
        "exports/co Midcup Slot/Players/05 - A/face_high.skl",
        &tracer_player_file("fcl_hair.skl"),
    );
    let findings = [
        "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
        "Info export_identified [Keep] (team=/co/, id=714)",
        "Warning skl_no_slot [Keep] at Players/05 - A (file=face_high.skl)",
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
    let package = face_package(&entries);
    let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
    assert_eq!(names, ["face_diff.bin", "face_high.fmdl"]);
}

// TC-MOD-53
#[test]
fn an_unsuffixed_model_is_reported_and_merged_into_the_hair_with_its_own_skeleton() {
    // `torso.fmdl` alone: reported by `check`, packed as `fcl_hair.fmdl`.
    let alone = Sandbox::new("mod_fallback_alone");
    alone.write(
        "exports/co Midcup Torso/Players/05 - A/torso.fmdl",
        &tracer_player_file("fcl_hair.fmdl"),
    );
    let check = alone.run(&pes21_settings(&alone), &["check"]);
    assert_eq!(
        findings_of(&check.messages(), "co Midcup Torso"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=torso.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=torso.fmdl)"
        ]
    );
    assert_eq!(check.exit_code(), 0);
    let run = alone.run(&pes21_settings(&alone), &["compile", "--no-deploy"]);
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&alone.root.join("output/4cc_99_test.cpk"));
    let package = face_package(&entries);
    let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
    assert_eq!(
        names,
        [
            "face_diff.bin",
            "fcl_hair.fmdl",
            "fcl_hair_sim.fclo",
            "fcl_hair_sim.skl"
        ]
    );

    // `torso.fmdl` beside `fcl_hair.fmdl`: one merged hair model.
    let merged = Sandbox::new("mod_fallback_merged");
    for name in ["torso.fmdl", "fcl_hair.fmdl"] {
        merged.write(
            &format!("exports/co Midcup Merged/Players/05 - A/{name}"),
            &tracer_player_file("fcl_hair.fmdl"),
        );
    }
    let run = merged.run(&pes21_settings(&merged), &["compile", "--no-deploy"]);
    assert_eq!(
        findings_of(&run.messages(), "co Midcup Merged"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=torso.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=torso.fmdl)",
            "Info team_colors_missing [Keep] ()",
            "Info fmdl_merged [Keep] at Players/05 - A (model=fcl_hair.fmdl)"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&merged.root.join("output/4cc_99_test.cpk"));
    let package = face_package(&entries);
    let model = FmdlFile::read(package.get("fcl_hair.fmdl").unwrap()).unwrap();
    let part = FmdlFile::read(&tracer_player_file("fcl_hair.fmdl")).unwrap();
    assert_eq!(
        Model::from_file(&model).unwrap().meshes.len(),
        2 * Model::from_file(&part).unwrap().meshes.len()
    );

    // `torso.fmdl` with `torso.skl`: that skeleton is the hair's.
    let paired = Sandbox::new("mod_fallback_skl");
    paired.write(
        "exports/co Midcup Paired/Players/05 - A/torso.fmdl",
        &tracer_player_file("fcl_hair.fmdl"),
    );
    let custom = body_skl("pes19");
    paired.write("exports/co Midcup Paired/Players/05 - A/torso.skl", &custom);
    let run = paired.run(&pes21_settings(&paired), &["compile", "--no-deploy"]);
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&paired.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        face_package(&entries).get("fcl_hair_sim.skl").unwrap(),
        custom
    );
}

// TC-MOD-14
#[test]
fn a_subfolder_s_model_is_the_part_its_name_gives_wherever_it_sits() {
    let sandbox = Sandbox::new("mod_subfolder_model");
    let player = "exports/co Midcup Subfolders/Players/05 - A";
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    sandbox.write(
        &format!("{player}/parts/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(
        &format!("{player}/parts/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );
    let boots_fpk = "Asset/model/character/boots/k0625/#Win/boots.fpk";

    let entries = compile_clean(
        &sandbox,
        "co Midcup Subfolders",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=parts/boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    // The model is the boots by its name, and under the marker the player has no face.
    assert_eq!(
        package_names(&entries[boots_fpk]),
        ["boots.fmdl", "boots.skl"]
    );
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert!(
        paths.iter().all(|path| !path.contains("/face/real/")),
        "{paths:#?}"
    );
}

/// `settings` with the strict file-type check off: a disallowed file is kept, as Info.
fn lenient(settings: &str) -> String {
    format!("{settings}[team-compiler]\nstrict_file_type_check = false\n")
}

// TC-MOD-67
#[test]
fn a_subfolder_s_marker_and_settings_are_file_type_disallowed_and_count_for_nothing() {
    let sandbox = Sandbox::new("mod_subfolder_root_files");
    let export = "co Midcup Subfolders";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    for name in ["ingame_face.toml", "settings.toml"] {
        sandbox.write(&format!("{player}/jessie/{name}"), b"");
    }
    let disallowed = |severity: &str, disposition: &str| -> Vec<String> {
        ["ingame_face.toml", "settings.toml"]
            .iter()
            .map(|name| {
                format!(
                    "{severity} file_type_disallowed [{disposition}] at Players/05 - A (file=jessie/{name})"
                )
            })
            .collect()
    };

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    // Each is out of place, and nothing more: no marker beside his face model.
    let lines = check.messages();
    let mut expected = disallowed("Error", "DropFolder");
    expected.push("Info export_identified [Keep] (team=/co/, id=714)".to_owned());
    assert_eq!(findings_of(&lines, export), expected, "{lines:#?}");

    // Kept, they count for nothing: the player keeps his face.
    let run = sandbox.run(
        &lenient(&pes21_settings(&sandbox)),
        &["compile", "--no-deploy"],
    );

    let lines = run.messages();
    let mut expected = disallowed("Info", "Keep");
    expected.extend(CLEAN.map(str::to_owned));
    assert_eq!(findings_of(&lines, export), expected, "{lines:#?}");
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        package_names(&entries["Asset/model/character/face/real/71405/#Win/face.fpk"]),
        ["face_diff.bin", "face_high.fmdl"]
    );
}

// TC-MOD-68
#[test]
fn a_shared_folder_s_subfolder_models_are_parts_of_its_package() {
    let sandbox = Sandbox::new("mod_shared_subfolder_models");
    let export = "co Midcup Subfolders";
    let root = format!("exports/{export}");
    sandbox.write(&format!("{root}/Players/05 - A/Crocs.boots"), b"");
    for model in ["boots.fmdl", "extra/x_boots.fmdl"] {
        sandbox.write(
            &format!("{root}/Boots/Crocs/{model}"),
            &tracer_player_file("boots.fmdl"),
        );
    }

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    // No `file_type_disallowed`: a shared folder's tree is a player folder's.
    let lines = check.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=extra/x_boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
        ],
        "{lines:#?}"
    );

    let entries = compile_clean(
        &sandbox,
        export,
        &[
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=extra/x_boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info fmdl_merged [Keep] at Boots/Crocs (model=boots.fmdl)",
        ],
    );

    // The shared boots hold both models' meshes, merged, under slot 05's exclusive id.
    assert_eq!(
        boots_mesh_count(&entries, "Asset/model/character/boots/k0625/#Win/boots.fpk"),
        2 * tracer_boots_mesh_count(),
        "boots.fmdl's and x_boots.fmdl's meshes"
    );
}

// TC-MOD-70
#[test]
fn a_subfolder_s_shared_link_is_the_root_s_and_names_one_folder_once() {
    let sandbox = Sandbox::new("mod_subfolder_shared_link");
    let export = "co Midcup Subfolders";
    let root = format!("exports/{export}");
    let player = format!("{root}/Players/05 - A");
    sandbox.write(
        &format!("{root}/Boots/Crocs/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    // Another model than Crocs', so his boots tell the two apart.
    sandbox.write(&format!("{root}/Boots/Tabi/boots.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/Crocs.boots"), b"");
    sandbox.write(&format!("{player}/jessie/Crocs.boots"), b"");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    // No `shared_link_duplicate`: both links name Crocs. Tabi, which no one links, is orphaned.
    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Warning shared_folder_orphaned [DropFolder] at Boots/Tabi ()",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let k0625 = "Asset/model/character/boots/k0625/#Win/boots.fpk";
    assert_eq!(
        boots_mesh_count(&entries, k0625),
        tracer_boots_mesh_count(),
        "Crocs' boots, once"
    );

    fs::remove_file(sandbox.root.join(format!("{player}/jessie/Crocs.boots"))).unwrap();
    sandbox.write(&format!("{player}/jessie/Tabi.boots"), b"");

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    // Two folders of one kind: the player is dropped, and neither folder is linked.
    let lines = check.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Error shared_link_duplicate [DropFolder] at Players/05 - A (kind=boots)",
            "Warning shared_folder_orphaned [DropFolder] at Boots/Crocs ()",
            "Warning shared_folder_orphaned [DropFolder] at Boots/Tabi ()",
            "Info export_identified [Keep] (team=/co/, id=714)",
        ],
        "{lines:#?}"
    );
}

// TC-MOD-70
#[test]
fn a_folder_linked_from_the_root_and_a_subfolder_beside_a_local_boots_model_combines_once() {
    let sandbox = Sandbox::new("mod_subfolder_link_combined");
    let export = "exports/co Midcup Combined";
    let player = format!("{export}/Players/05 - A");
    write_player(&sandbox, &player, "kit_boots.fmdl");
    sandbox.write(&format!("{player}/Crocs.boots"), b"");
    sandbox.write(&format!("{player}/jessie/Crocs.boots"), b"");
    sandbox.write(
        &format!("{export}/Boots/Crocs/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let combined: Vec<&str> = findings_of(&lines, "co Midcup Combined")
        .into_iter()
        .filter(|line| line.contains("link_combined"))
        .collect();
    assert_eq!(
        combined,
        ["Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)"]
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        boots_mesh_count(&entries, "Asset/model/character/boots/k0625/#Win/boots.fpk"),
        2 * tracer_boots_mesh_count(),
        "Crocs's meshes once, plus the local model's"
    );
}

/// The face diff fixture `name`, from `tests/fixtures/face_diff/`.
pub(crate) fn face_diff_fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/face_diff")
            .join(name),
    )
    .unwrap()
}

/// Copies the tracer's face into slot 05 of `export`, its `face_diff.bin` replaced by a
/// `face_diff.xml` holding `xml`.
fn write_xml_face(sandbox: &Sandbox, export: &str, xml: &[u8]) {
    let player = format!("{export}/Players/05 - A");
    sandbox.copy_tracer_face(&player);
    fs::remove_file(sandbox.root.join(&player).join("face_diff.bin")).unwrap();
    sandbox.write(&format!("{player}/face_diff.xml"), xml);
}

/// Copies the tracer's face into slot 07 of `export`, beside the slot 05 under test, so the
/// export still writes a CPK when slot 05's folder is dropped.
fn write_slot_07(sandbox: &Sandbox, export: &str) {
    sandbox.copy_tracer_face(&format!("{export}/Players/07 - B"));
}

/// Checks, then compiles, the sandbox for PES 21 (slot 05 under test, slot 07 written by
/// `write_slot_07`), asserting both commands report `finding` on slot 05 before the export
/// `name`'s identity (and `compile` its missing team colors after it), and that slot 05's
/// folder is not in the CPK while slot 07's is.
fn assert_face_dropped(sandbox: &Sandbox, name: &str, finding: &str) {
    for command in ["check", "compile"] {
        let run = sandbox.run(&pes21_settings(sandbox), &command_args(command));
        let validated = [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            finding,
            "Info fmdl_weights_not_normalized [Keep] at Players/07 - B (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/07 - B (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/07 - B (file=glove_l.fmdl, count=2)",
            "Info export_identified [Keep] (team=/co/, id=714)",
        ];
        // Planning's note, which only `compile` makes.
        let planned: &[&str] = if command == "compile" {
            &[TEAM_COLORS_MISSING]
        } else {
            &[]
        };
        assert_eq!(
            findings_of(&run.messages(), name),
            [&validated[..], planned].concat(),
            "{command}"
        );
        assert_eq!(run.exit_code(), 1, "{command}");
    }
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // Slot 05's face and portrait are player 71405's; `/co/`'s block starts at 621, so its
    // boots and gloves are k0625 and g0625, slot 07's 627.
    assert!(
        !entries
            .keys()
            .any(|path| path.contains("71405") || path.contains("0625/")),
        "{:?}",
        entries.keys()
    );
    assert!(entries.contains_key("Asset/model/character/face/real/71407/#Win/face.fpk"));
    assert!(entries.contains_key("Asset/model/character/boots/k0627/#Win/boots.fpk"));
}

// TC-MOD-15
#[test]
fn a_face_diff_xml_is_decoded_into_the_face_and_a_corrupt_one_drops_the_folder() {
    let sandbox = Sandbox::new("mod_face_diff_xml");
    write_xml_face(
        &sandbox,
        "exports/co Midcup Xml",
        &face_diff_fixture("dif.xml"),
    );
    let clean = [
        "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
        "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
        "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
        "Info export_identified [Keep] (team=/co/, id=714)",
    ];

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    assert_eq!(findings_of(&check.messages(), "co Midcup Xml"), clean);
    assert_eq!(check.exit_code(), 0);
    let entries = compile_clean(
        &sandbox,
        "co Midcup Xml",
        &[&clean[..], &["Info team_colors_missing [Keep] ()"]].concat(),
    );

    assert_eq!(
        face_package(&entries).get("face_diff.bin").unwrap(),
        face_diff_fixture("dif.bin")
    );

    // One payload character replaced by a character base64 does not use.
    let mut corrupt = face_diff_fixture("dif.xml");
    let payload = corrupt
        .windows(4)
        .position(|bytes| bytes == b"RkFD")
        .unwrap();
    assert!(corrupt[payload + 100].is_ascii_alphanumeric());
    corrupt[payload + 100] = b'*';
    let corrupt_sandbox = Sandbox::new("mod_face_diff_xml_corrupt");
    write_xml_face(&corrupt_sandbox, "exports/co Midcup Corrupt", &corrupt);
    write_slot_07(&corrupt_sandbox, "exports/co Midcup Corrupt");

    assert_face_dropped(
        &corrupt_sandbox,
        "co Midcup Corrupt",
        "Error face_diff_invalid [DropFolder] at Players/05 - A \
         (file=face_diff.xml, reason=the base64 text holds '*' where base64 cannot)",
    );
}

// TC-MOD-42
#[test]
fn a_face_diff_given_twice_or_shorter_than_its_header_drops_the_folder() {
    // Both forms in one folder: neither is read, the conflict is the finding.
    let twice = Sandbox::new("mod_face_diff_twice");
    twice.copy_tracer_face("exports/co Midcup Twice/Players/05 - A");
    twice.write(
        "exports/co Midcup Twice/Players/05 - A/face_diff.xml",
        &face_diff_fixture("dif.xml"),
    );
    write_slot_07(&twice, "exports/co Midcup Twice");
    assert_face_dropped(
        &twice,
        "co Midcup Twice",
        "Error xml_dif_conflict [DropFolder] at Players/05 - A (file=face_diff.xml)",
    );

    // A `face_diff.bin` one byte shorter than its header gives.
    let cut = Sandbox::new("mod_face_diff_cut");
    cut.copy_tracer_face("exports/co Midcup Cut/Players/05 - A");
    cut.write(
        "exports/co Midcup Cut/Players/05 - A/face_diff.bin",
        &face_diff_fixture("dif.bin")[..943],
    );
    write_slot_07(&cut, "exports/co Midcup Cut");
    assert_face_dropped(
        &cut,
        "co Midcup Cut",
        "Error face_diff_invalid [DropFolder] at Players/05 - A \
         (file=face_diff.bin, reason=the face diff is 943 bytes long, but its \
         header gives 944: the game would read past its end)",
    );
}

// TC-MOD-14
#[test]
fn a_subfolder_s_parts_combine_with_loose_root_files_of_their_category() {
    let sandbox = Sandbox::new("mod_subfolder_parts");
    let player = "exports/co Midcup Parts/Players/05 - A";
    for (name, tracer_name) in [
        ("fcl_hair.fmdl", "fcl_hair.fmdl"),
        ("face/torso.fmdl", "fcl_hair.fmdl"),
        ("glove_l.fmdl", "glove_l.fmdl"),
        ("gloves/glove_r.fmdl", "glove_r.fmdl"),
    ] {
        sandbox.write(
            &format!("{player}/{name}"),
            &tracer_player_file(tracer_name),
        );
    }
    let glove_fpk = "Asset/model/character/glove/g0625/#Win/glove.fpk";

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Parts"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face/torso.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=face/torso.fmdl)",
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
            "Asset/model/character/face/real/71405/#Win/face.fpk",
            "Asset/model/character/face/real/71405/#Win/face.fpkd",
            glove_fpk,
            "Asset/model/character/glove/g0625/#Win/glove.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ]
    );
    let package = face_package(&entries);
    let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
    assert_eq!(
        names,
        [
            "face_diff.bin",
            "fcl_hair.fmdl",
            "fcl_hair_sim.fclo",
            "fcl_hair_sim.skl"
        ]
    );
    let model = FmdlFile::read(package.get("fcl_hair.fmdl").unwrap()).unwrap();
    let part = FmdlFile::read(&tracer_player_file("fcl_hair.fmdl")).unwrap();
    assert_eq!(
        Model::from_file(&model).unwrap().meshes.len(),
        2 * Model::from_file(&part).unwrap().meshes.len()
    );
    assert_eq!(
        package_names(&entries[glove_fpk]),
        ["glove_l.fmdl", "glove_r.fmdl"]
    );
}

// TC-MOD-13
#[test]
fn a_pre_fox_target_reports_neither_the_fallback_nor_a_slotless_skeleton() {
    let sandbox = Sandbox::new("mod_pre_fox_names");
    for name in ["torso.fmdl", "face_high.fmdl"] {
        sandbox.write(
            &format!("exports/co Midcup Names/Players/05 - A/{name}"),
            &clean_model(),
        );
    }
    sandbox.write("exports/co Midcup Names/Players/05 - A/face_high.skl", b"");

    let run = sandbox.run("[common]\npes_version = 17\n", &["check"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Names"),
        ["Info export_identified [Keep] (team=/co/, id=714)"]
    );
    assert_eq!(run.exit_code(), 0);
}

/// Writes a face folder at `folder`: the tracer's hair model as `face_high.fmdl`, its
/// `face_diff.bin`, and its `shirt.dds` under `texture_name`.
fn write_face(sandbox: &Sandbox, folder: &str, texture_name: &str) {
    sandbox.write(
        &format!("{folder}/face_high.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(
        &format!("{folder}/face_diff.bin"),
        &tracer_player_file("face_diff.bin"),
    );
    sandbox.write(
        &format!("{folder}/{texture_name}"),
        &tracer_player_file("shirt.dds"),
    );
}

// TC-MOD-08
#[test]
fn a_face_link_combines_the_shared_face_folder_into_the_player_s_face() {
    let sandbox = Sandbox::new("mod_face_link");
    let export = "exports/co Midcup Faces";
    write_face(&sandbox, &format!("{export}/Players/05 - A"), "shirt.dds");
    sandbox.write(&format!("{export}/Players/05 - A/Longhair.face"), b"");
    // The tracer's boots model stands in for the shared hair model.
    sandbox.write(
        &format!("{export}/Faces/Longhair/hair_high.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Faces"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Faces/Longhair (file=hair_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Longhair.face)"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "Asset/model/character/common/714/05 - A/sourceimages/#windx11/shirt.ftex",
            "Asset/model/character/face/real/71405/#Win/face.fpk",
            "Asset/model/character/face/real/71405/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ],
        "nothing for Longhair on its own"
    );
    assert_eq!(
        package_names(&entries["Asset/model/character/face/real/71405/#Win/face.fpk"]),
        ["face_diff.bin", "face_high.fmdl", "hair_high.fmdl"]
    );
}

// TC-MOD-55
#[test]
fn a_shared_face_s_boots_model_is_a_part_of_the_linking_player_s_own_boots() {
    let sandbox = Sandbox::new("mod_face_link_boots");
    install_tables(&sandbox, &[(BOOTS_LIST, &item_list(&[(70201, 11)]))]);
    let export = "exports/co Midcup Round";
    sandbox.write(&format!("{export}/Players/05 - A/Round.face"), b"");
    for name in ["fcl_hair.fmdl", "boots.fmdl"] {
        sandbox.write(
            &format!("{export}/Faces/Round/{name}"),
            &tracer_player_file(name),
        );
    }

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Round"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Faces/Round (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Faces/Round (file=fcl_hair.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // Slot 05's exclusive id in team 714's block: 101 + (714 - 701) * 40 + 5 - 1.
    let boots = "Asset/model/character/boots/k0625/#Win/boots.fpk";
    assert_eq!(package_names(&entries[boots]), ["boots.fmdl", "boots.skl"]);
    assert_eq!(boots_mesh_count(&entries, boots), tracer_boots_mesh_count());
    assert_eq!(pairs_of(&entries, BOOTS_LIST), [(70201, 11), (71405, 625)]);
}

// TC-MOD-58
#[test]
fn a_shared_face_s_boots_model_converts_with_the_face_folder_s_mtl() {
    let sandbox = Sandbox::new("mod_face_link_boots_model");
    install_tables(&sandbox, &[(BOOTS_LIST, &item_list(&[(70201, 11)]))]);
    let export = "exports/co Midcup Round";
    sandbox.write(&format!("{export}/Players/05 - A/Round.face"), b"");
    sandbox.write(
        &format!("{export}/Faces/Round/fcl_hair.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(&format!("{export}/Faces/Round/boots.model"), &card_model());
    sandbox.write(
        &format!("{export}/Faces/Round/boots.mtl"),
        &card_materials(),
    );
    // The texture the card's `.mtl` names, as beside the pair in a player folder.
    sandbox.write(&format!("{export}/Faces/Round/skin.dds"), &small_dds());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Round"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Faces/Round (file=fcl_hair.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // Slot 05's exclusive id in team 714's block.
    let boots = "Asset/model/character/boots/k0625/#Win/boots.fpk";
    assert_eq!(package_names(&entries[boots]), ["boots.fmdl", "boots.skl"]);
    let package = fpk::FpkFile::read(&entries[boots]).unwrap();
    // The card's one bone is the game's own: the conversion writes no skeleton, so the boots
    // get the bundled one.
    assert_eq!(package.get("boots.skl").unwrap(), body_skl("pes21"));
    // The `.mtl`'s `./skin.dds`, pointed at the player's texture home, which the combined
    // face folder's textures join.
    let file = FmdlFile::read(package.get("boots.fmdl").unwrap()).unwrap();
    let skin: Vec<(String, String)> = texture_paths(&file)
        .unwrap()
        .into_iter()
        .filter(|path| path.file_name.starts_with("skin."))
        .map(|path| (path.directory, path.file_name))
        .collect();
    assert!(!skin.is_empty(), "the boots converted with Round's .mtl");
    for path in &skin {
        assert_eq!(path, &(HOME_714_05.to_owned(), "skin.dds".to_owned()));
    }
    assert_eq!(pairs_of(&entries, BOOTS_LIST), [(70201, 11), (71405, 625)]);
}

// TC-MOD-59
#[test]
fn a_boots_link_combines_with_the_boots_of_the_player_s_shared_face() {
    let sandbox = Sandbox::new("mod_face_link_boots_link");
    install_tables(&sandbox, &[(BOOTS_LIST, &item_list(&[(70201, 11)]))]);
    let export = "exports/co Midcup Round";
    sandbox.write(&format!("{export}/Players/05 - A/Round.face"), b"");
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    for name in ["fcl_hair.fmdl", "boots.fmdl"] {
        sandbox.write(
            &format!("{export}/Faces/Round/{name}"),
            &tracer_player_file(name),
        );
    }
    sandbox.write(
        &format!("{export}/Boots/Crocs/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Round"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Faces/Round (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Faces/Round (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=boots.fmdl)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // Slot 05's exclusive id in team 714's block.
    let boots = "Asset/model/character/boots/k0625/#Win/boots.fpk";
    assert_eq!(package_names(&entries[boots]), ["boots.fmdl", "boots.skl"]);
    assert_eq!(
        boots_mesh_count(&entries, boots),
        2 * tracer_boots_mesh_count(),
        "Round's meshes plus Crocs's"
    );
    assert_eq!(pairs_of(&entries, BOOTS_LIST), [(70201, 11), (71405, 625)]);
    // The block's first shared id, which Crocs would take if a player linked it plainly.
    assert!(
        !entries
            .keys()
            .any(|path| path.starts_with("Asset/model/character/boots/k0644/")),
        "{:?}",
        entries.keys()
    );
}

// TC-MOD-60
#[test]
fn a_gloves_link_combines_with_the_hands_split_from_the_player_s_face_model() {
    let sandbox = Sandbox::new("mod_hand_split_gloves_link");
    install_tables(&sandbox, &[(GLOVE_LIST, &item_list(&[]))]);
    // A full-body model with both hands on (`tests/fixtures/hand_split/README.md`): 40 faces,
    // of which each hand's split takes 8 and the body keeps 24.
    let body =
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hand_split/body.fmdl"))
            .unwrap();
    let export = "exports/co Midcup Hands";
    sandbox.write(&format!("{export}/Players/05 - A/body.fmdl"), &body);
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.gloves"), b"");
    // Crocs's gloves are the same strip under each glove's name, which is never split: their
    // bones are the split hands' own, so the two merge. An authored glove's hand bone has a
    // parent, which the strip's has not, and would not merge with them (the README).
    for name in ["glove_l.fmdl", "glove_r.fmdl"] {
        sandbox.write(&format!("{export}/Gloves/Crocs/{name}"), &body);
    }

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Hands"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=body.fmdl)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.gloves)",
            "Info model_hand_split [Keep] at Players/05 - A (model=body.fmdl, gloves=glove_l, glove_r)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=glove_l.fmdl)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=glove_r.fmdl)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // Slot 05's exclusive id in team 714's block.
    let gloves = "Asset/model/character/glove/g0625/#Win/glove.fpk";
    assert_eq!(
        package_names(&entries[gloves]),
        ["glove_l.fmdl", "glove_r.fmdl"]
    );
    // Each hand's 8 split faces and Crocs's glove of that name, the whole strip.
    assert_eq!(
        [
            face_count(&entries, gloves, "glove_l.fmdl"),
            face_count(&entries, gloves, "glove_r.fmdl"),
        ],
        [8 + 40, 8 + 40]
    );
    assert_eq!(pairs_of(&entries, GLOVE_LIST), [(71405, 625)]);
    // The block's first shared id, which Crocs would take if a player linked it plainly.
    assert!(
        !entries
            .keys()
            .any(|path| path.starts_with("Asset/model/character/glove/g0644/")),
        "{:?}",
        entries.keys()
    );
}

// TC-MOD-04
#[test]
fn a_texture_the_player_and_a_combined_boots_folder_hold_is_packed_once_or_overridden() {
    let export = "exports/co Midcup Skin";
    let skin = "Asset/model/character/common/714/05 - A/sourceimages/#windx11/skin.ftex";
    let k0625 = "Asset/model/character/boots/k0625/#Win/boots.fpk";
    let write = |sandbox: &Sandbox, shared_skin: &[u8]| {
        write_face(sandbox, &format!("{export}/Players/05 - A"), "skin.dds");
        sandbox.write(
            &format!("{export}/Players/05 - A/kit_boots.fmdl"),
            &tracer_player_file("boots.fmdl"),
        );
        sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
        sandbox.write(
            &format!("{export}/Boots/Crocs/boots.fmdl"),
            &tracer_player_file("boots.fmdl"),
        );
        sandbox.write(&format!("{export}/Boots/Crocs/skin.dds"), shared_skin);
        sandbox.write(
            &format!("{export}/Boots/Crocs/sole.dds"),
            &tracer_player_file("shirt.dds"),
        );
    };

    // The same bytes under one stem in both sources: packed once, no finding.
    let same = Sandbox::new("mod_skin_same");
    write(&same, &tracer_player_file("shirt.dds"));
    let run = same.run(&pes21_settings(&same), &["compile", "--no-deploy"]);
    assert_eq!(
        findings_of(&run.messages(), "co Midcup Skin"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=kit_boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=boots.fmdl)"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&same.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            k0625,
            "Asset/model/character/boots/k0625/#Win/boots.fpkd",
            skin,
            "Asset/model/character/common/714/05 - A/sourceimages/#windx11/sole.ftex",
            "Asset/model/character/face/real/71405/#Win/face.fpk",
            "Asset/model/character/face/real/71405/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ]
    );

    // Different bytes: the player's own `skin` is the one packed, the shared folder's copy
    // left out, the face, the boots and the texture only they brought all kept.
    let differing = Sandbox::new("mod_skin_differing");
    write(&differing, &tracer_kit());
    let run = differing.run(&pes21_settings(&differing), &["compile", "--no-deploy"]);
    assert_eq!(
        findings_of(&run.messages(), "co Midcup Skin"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=kit_boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=boots.fmdl)",
            "Info shared_texture_overridden [Keep] at Players/05 - A (texture=skin, folder=Boots/Crocs)"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&differing.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            k0625,
            "Asset/model/character/boots/k0625/#Win/boots.fpkd",
            skin,
            "Asset/model/character/common/714/05 - A/sourceimages/#windx11/sole.ftex",
            "Asset/model/character/face/real/71405/#Win/face.fpk",
            "Asset/model/character/face/real/71405/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ],
        "the same paths as the same-bytes run"
    );
    assert_eq!(
        entries[skin],
        ftex::dds_to_ftex(&tracer_player_file("shirt.dds"), ftex::ColorSpace::Normal).unwrap()
    );
}

// TC-MOD-04
#[test]
fn under_ingame_face_a_texture_the_player_and_a_combined_boots_folder_hold_differently_is_his() {
    let sandbox = Sandbox::new("mod_ingame_face_skin_differing");
    let export = "exports/co Midcup Skin";
    let player = format!("{export}/Players/05 - A");
    // His own folder feeds the boots, there being no face: two sources of one package, and
    // the local file still wins.
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    sandbox.write(
        &format!("{player}/kit_boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(
        &format!("{player}/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(&format!("{player}/Crocs.boots"), b"");
    sandbox.write(
        &format!("{export}/Boots/Crocs/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(&format!("{export}/Boots/Crocs/skin.dds"), &tracer_kit());
    sandbox.write(
        &format!("{export}/Boots/Crocs/sole.dds"),
        &tracer_player_file("shirt.dds"),
    );
    // Slot 07: boots that compile, so the CPK is written.
    sandbox.write(
        &format!("{export}/Players/07 - B/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Skin"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=kit_boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/07 - B (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=boots.fmdl)",
            "Info shared_texture_overridden [Keep] at Players/05 - A (texture=skin, folder=Boots/Crocs)"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "Asset/model/character/boots/k0625/#Win/boots.fpk",
            "Asset/model/character/boots/k0625/#Win/boots.fpkd",
            "Asset/model/character/boots/k0627/#Win/boots.fpk",
            "Asset/model/character/boots/k0627/#Win/boots.fpkd",
            "Asset/model/character/common/714/05 - A/sourceimages/#windx11/skin.ftex",
            "Asset/model/character/common/714/05 - A/sourceimages/#windx11/sole.ftex",
            "Asset/model/character/face/real/71407/#Win/face.fpk",
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ],
        "slot 05's boots and textures kept, no face under the marker"
    );
}

// TC-MOD-33
#[test]
fn a_texture_the_player_s_folder_and_a_combined_face_folder_hold_differently_is_the_player_s() {
    let sandbox = Sandbox::new("mod_face_texture_overridden");
    let export = "exports/co Midcup Conflict";
    write_face(&sandbox, &format!("{export}/Players/05 - A"), "skin.dds");
    sandbox.write(&format!("{export}/Players/05 - A/Round.face"), b"");
    sandbox.write(
        &format!("{export}/Faces/Round/hair_high.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(&format!("{export}/Faces/Round/skin.dds"), &tracer_kit());
    // Slot 07: boots that compile, so the CPK is written.
    sandbox.write(
        &format!("{export}/Players/07 - B/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Conflict"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/07 - B (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Faces/Round (file=hair_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
            "Info shared_texture_overridden [Keep] at Players/05 - A (texture=skin, folder=Faces/Round)"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "Asset/model/character/boots/k0627/#Win/boots.fpk",
            "Asset/model/character/boots/k0627/#Win/boots.fpkd",
            "Asset/model/character/common/714/05 - A/sourceimages/#windx11/skin.ftex",
            "Asset/model/character/face/real/71405/#Win/face.fpk",
            "Asset/model/character/face/real/71405/#Win/face.fpkd",
            "Asset/model/character/face/real/71407/#Win/face.fpk",
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ],
        "slot 05 kept, the shared folder's skin left out of his texture folder"
    );
    // The player's `skin` is the one packed; the merge of his face with Round's went ahead.
    assert_eq!(
        entries["Asset/model/character/common/714/05 - A/sourceimages/#windx11/skin.ftex"],
        ftex::dds_to_ftex(&tracer_player_file("shirt.dds"), ftex::ColorSpace::Normal).unwrap()
    );
    assert_eq!(
        package_names(&entries["Asset/model/character/face/real/71405/#Win/face.fpk"]),
        ["face_diff.bin", "face_high.fmdl", "hair_high.fmdl"]
    );
}

// TC-MOD-09
#[test]
fn parts_with_a_skeleton_mismatch_or_a_material_defined_twice_leave_their_boots_out() {
    let sandbox = Sandbox::new("mod_merge_conflicts");
    let export = "exports/co Midcup Conflicts";
    // Slot 05: two boots parts, one paired with a skeleton and one without.
    for name in ["boots.fmdl", "kit_boots.fmdl"] {
        sandbox.write(
            &format!("{export}/Players/05 - A/{name}"),
            &tracer_player_file("boots.fmdl"),
        );
    }
    sandbox.write(
        &format!("{export}/Players/05 - A/boots.skl"),
        &body_skl("pes21"),
    );
    sandbox.write(
        &format!("{export}/Players/05 - A/shirt.dds"),
        &tracer_player_file("shirt.dds"),
    );
    // Slot 07: the tracer's boots beside a copy whose `shirt` material another shader draws:
    // both define `shirt`, differently. (A copy differing only in its textures' directories
    // would not: every part's `shirt` is pointed at the player's own `shirt.dds` before the
    // merge.)
    sandbox.write(
        &format!("{export}/Players/07 - B/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    let mut other_shader =
        Model::from_file(&FmdlFile::read(&tracer_player_file("boots.fmdl")).unwrap()).unwrap();
    let shirt = other_shader
        .materials
        .iter_mut()
        .find(|material| material.name == "shirt")
        .unwrap();
    shirt.shader = "fox3ddf_blin".to_owned();
    shirt.technique = "fox3DDF_Blin".to_owned();
    sandbox.write(
        &format!("{export}/Players/07 - B/x_boots.fmdl"),
        &other_shader.to_file().unwrap().write(),
    );
    sandbox.write(
        &format!("{export}/Players/07 - B/shirt.dds"),
        &tracer_player_file("shirt.dds"),
    );
    // Slot 09: boots that compile, so the CPK is written.
    sandbox.write(
        &format!("{export}/Players/09 - C/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Conflicts"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=kit_boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/07 - B (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/07 - B (file=x_boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/09 - C (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Error skl_merge_conflict [DropFolder] at Players/05 - A (skeleton=differs)",
            "Error merge_material_conflict [DropFolder] at Players/07 - B (material=shirt)"
        ]
    );
    assert_eq!(run.exit_code(), 1);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "Asset/model/character/boots/k0629/#Win/boots.fpk",
            "Asset/model/character/boots/k0629/#Win/boots.fpkd",
            "Asset/model/character/common/714/05 - A/sourceimages/#windx11/shirt.ftex",
            "Asset/model/character/common/714/07 - B/sourceimages/#windx11/shirt.ftex",
            "Asset/model/character/face/real/71405/#Win/face.fpk",
            "Asset/model/character/face/real/71405/#Win/face.fpkd",
            "Asset/model/character/face/real/71407/#Win/face.fpk",
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
            "Asset/model/character/face/real/71409/#Win/face.fpk",
            "Asset/model/character/face/real/71409/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ],
        "no k0625 or k0627: the failed boots are left out; each folder's blank face commits \
         with the folder's textures"
    );
    // Each surviving face is the blank one: neither folder held a face model, slot 09
    // included.
    for player in [71405, 71407, 71409] {
        assert_blank_face(
            &entries,
            &format!("Asset/model/character/face/real/{player}/#Win/face.fpk"),
        );
    }
}

// TC-MOD-05
#[test]
fn shared_boots_folders_compile_once_each_under_the_shared_ids_in_name_order() {
    let sandbox = Sandbox::new("mod_shared_boots");
    // Crocs brings its own skeleton, Mud none, so each package says which folder it came from.
    let crocs_skl = body_skl("pes19");
    assert_ne!(crocs_skl, body_skl("pes21"));
    write_shared_boots(&sandbox, "co Midcup Shared", "Mud", None, &[11]);
    write_shared_boots(
        &sandbox,
        "co Midcup Shared",
        "Crocs",
        Some(&crocs_skl),
        &[3, 7],
    );

    let entries = compile_clean(
        &sandbox,
        "co Midcup Shared",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Mud (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Warning shared_ids_unrecorded [Keep] (team=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "Asset/model/character/boots/k0631/#Win/boots.fpk",
            "Asset/model/character/boots/k0631/#Win/boots.fpkd",
            "Asset/model/character/boots/k0631/#windx11/shirt.ftex",
            "Asset/model/character/boots/k0644/#Win/boots.fpk",
            "Asset/model/character/boots/k0644/#Win/boots.fpkd",
            "Asset/model/character/boots/k0644/#windx11/shirt.ftex",
            "Asset/model/character/face/real/71403/#Win/face.fpk",
            "Asset/model/character/face/real/71403/#Win/face.fpkd",
            "Asset/model/character/face/real/71407/#Win/face.fpk",
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
            "Asset/model/character/face/real/71411/#Win/face.fpk",
            "Asset/model/character/face/real/71411/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
            "unlimited-tool-works/shared_ids/714.toml",
        ],
        "no k0623 or k0627 for the linking slots; each one's face is blank"
    );
    let crocs = "Asset/model/character/boots/k0644/#Win/boots.fpk";
    let mud = "Asset/model/character/boots/k0631/#Win/boots.fpk";
    assert_eq!(boots_skl(&entries, crocs), crocs_skl);
    assert_eq!(boots_skl(&entries, mud), body_skl("pes21"));
    // Each model names its own folder's textures: the tracer's boots model lists `shirt.dds`
    // in two texture slots, both pointed at the output's folder.
    for (path, directory) in [
        (crocs, "/Assets/pes16/model/character/boots/k0644/"),
        (mud, "/Assets/pes16/model/character/boots/k0631/"),
    ] {
        let package = fpk::FpkFile::read(&entries[path]).unwrap();
        let model = FmdlFile::read(package.get("boots.fmdl").unwrap()).unwrap();
        let shirt: Vec<String> = texture_paths(&model)
            .unwrap()
            .into_iter()
            .filter(|texture| texture.file_name == "shirt.dds")
            .map(|texture| texture.directory)
            .collect();
        assert_eq!(shirt, [directory, directory], "{path}");
    }
}

// TC-MOD-06
#[test]
fn a_midcup_s_new_shared_folder_past_the_installed_record_exhausts_the_pool() {
    let sandbox = Sandbox::new("mod_boots_pool");
    // The installed CPK's /co/ record names every one of the block's shared boots ids.
    let names: Vec<(u16, String)> = (0..17u16)
        .map(|index| (644 + index, format!("R{index:02}")))
        .collect();
    let entries: Vec<(u16, &str)> = names
        .iter()
        .map(|(id, name)| (*id, name.as_str()))
        .collect();
    let installed = record(&entries);
    install_tables(&sandbox, &[(SHARED_IDS_714, installed.as_slice())]);
    // Birk, which the record does not name, is worn by two slots, so it takes a shared id,
    // the 18th needed; a one-wearer folder would take its wearer's exclusive id instead.
    write_shared_boots(&sandbox, "co Midcup Pool", "Birk", None, &[5, 6]);

    let run = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Pool"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Boots/Birk (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Error boots_id_pool_exhausted [DropExport] (count=18)"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-PLN-02
#[test]
fn shared_ids_follow_the_folder_names_so_a_new_folder_shifts_the_ones_after_it() {
    let sandbox = Sandbox::new("pln_shared_ids");
    let apple_skl = body_skl("pes19");
    let mango_skl = body_skl("pes18");
    assert_ne!(apple_skl, body_skl("pes21"));
    assert_ne!(mango_skl, body_skl("pes21"));
    assert_ne!(mango_skl, apple_skl);
    write_shared_boots(&sandbox, "co Midcup Named", "Zebra", None, &[3, 4]);
    write_shared_boots(
        &sandbox,
        "co Midcup Named",
        "Apple",
        Some(&apple_skl),
        &[7, 9],
    );
    let k0644 = "Asset/model/character/boots/k0644/#Win/boots.fpk";
    let k0645 = "Asset/model/character/boots/k0645/#Win/boots.fpk";
    let k0646 = "Asset/model/character/boots/k0646/#Win/boots.fpk";

    let first = compile_clean(
        &sandbox,
        "co Midcup Named",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Boots/Apple (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Zebra (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Warning shared_ids_unrecorded [Keep] (team=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );
    assert!(!first.contains_key(k0646), "{:?}", first.keys());
    assert_eq!(boots_skl(&first, k0644), apple_skl, "Apple");
    assert_eq!(boots_skl(&first, k0645), body_skl("pes21"), "Zebra");

    write_shared_boots(
        &sandbox,
        "co Midcup Named",
        "Mango",
        Some(&mango_skl),
        &[11, 13],
    );
    let second = compile_clean(
        &sandbox,
        "co Midcup Named",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Boots/Apple (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Mango (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Zebra (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Warning shared_ids_unrecorded [Keep] (team=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );
    assert_eq!(boots_skl(&second, k0644), apple_skl, "Apple");
    assert_eq!(boots_skl(&second, k0645), mango_skl, "Mango");
    assert_eq!(boots_skl(&second, k0646), body_skl("pes21"), "Zebra");
}

/// What `co Midcup Planned` reports at each compile.
const PLANNED_FINDINGS: &[&str] = &[
    "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
    "Info fmdl_weights_not_normalized [Keep] at Players/23 - B (file=glove_l.fmdl, count=2)",
    "Info export_identified [Keep] (team=/co/, id=714)",
    "Info team_colors_missing [Keep] ()",
];

// TC-PLN-01
#[test]
fn the_planned_ids_are_the_slot_s_and_two_compiles_write_the_same_bytes() {
    let sandbox = Sandbox::new("pln_planned_ids");
    sandbox.write(
        "exports/co Midcup Planned/Players/05 - A/boots.fmdl",
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(
        "exports/co Midcup Planned/Players/23 - B/glove_l.fmdl",
        &tracer_player_file("glove_l.fmdl"),
    );

    let first = compile_clean(&sandbox, "co Midcup Planned", PLANNED_FINDINGS);
    let first_cpk = fs::read(sandbox.root.join("output/4cc_99_test.cpk")).unwrap();
    let second = compile_clean(&sandbox, "co Midcup Planned", PLANNED_FINDINGS);
    let second_cpk = fs::read(sandbox.root.join("output/4cc_99_test.cpk")).unwrap();

    let paths: Vec<&str> = first.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "Asset/model/character/boots/k0625/#Win/boots.fpk",
            "Asset/model/character/boots/k0625/#Win/boots.fpkd",
            "Asset/model/character/face/real/71405/#Win/face.fpk",
            "Asset/model/character/face/real/71405/#Win/face.fpkd",
            "Asset/model/character/face/real/71423/#Win/face.fpk",
            "Asset/model/character/face/real/71423/#Win/face.fpkd",
            "Asset/model/character/glove/g0643/#Win/glove.fpk",
            "Asset/model/character/glove/g0643/#Win/glove.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ]
    );
    assert_eq!(
        package_names(&first["Asset/model/character/glove/g0643/#Win/glove.fpk"]),
        ["glove_l.fmdl"]
    );
    assert_eq!(first, second);
    assert!(first_cpk == second_cpk, "the two CPKs differ");
}

// TC-CMN-05
#[test]
fn per_kit_models_on_pes_21_compile_the_lowest_variant_alone_and_say_so() {
    let sandbox = Sandbox::new("mod_kit_variant_models");
    let player = "exports/co Midcup Variants/Players/05 - A";
    // Different models: merged, the two would make a hair of more meshes than `pants_kit1`'s.
    sandbox.write(
        &format!("{player}/pants_kit1.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(
        &format!("{player}/pants_kit2.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    // The deep pass does not read the left-out variant, which nothing reads; it is not a part
    // of the hair, so only `pants_kit1` is the hair's fallback.
    let checked = [
        "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=pants_kit1.fmdl, count=1662)",
        "Info export_identified [Keep] (team=/co/, id=714)",
        "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=pants_kit1.fmdl)",
    ];

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    assert_eq!(
        findings_of(&check.messages(), "co Midcup Variants"),
        checked
    );
    assert_eq!(check.exit_code(), 0);

    let both = compile_clean(
        &sandbox,
        "co Midcup Variants",
        &[
            &checked[..],
            &[
                "Info team_colors_missing [Keep] ()",
                "Warning kit_variant_model_left_out [Keep] at Players/05 - A (model=pants_kitN.fmdl, used=pants_kit1.fmdl)",
            ],
        ]
        .concat(),
    );
    fs::remove_file(sandbox.root.join(format!("{player}/pants_kit2.fmdl"))).unwrap();
    let alone = compile_clean(
        &sandbox,
        "co Midcup Variants",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=pants_kit1.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=pants_kit1.fmdl)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let hair = |entries: &BTreeMap<String, Vec<u8>>| {
        face_package(entries).get("fcl_hair.fmdl").unwrap().to_vec()
    };
    assert!(hair(&both) == hair(&alone), "the packed hair differs");
}

// TC-CMN-28
#[test]
fn a_kits_fallback_model_is_used_and_the_variant_left_out_on_pes_21() {
    let sandbox = Sandbox::new("mod_kit_fallback");
    let player = "exports/co Midcup Fallback/Players/05 - A";
    // Different models: the fallback is the hair, the variant the boots.
    sandbox.write(
        &format!("{player}/pants_kitF.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(
        &format!("{player}/pants_kit1.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    // The deep pass does not read the left-out variant, which nothing reads; it is not a
    // part of the hair, so only `pants_kitF` is the hair's fallback.
    let checked = [
        "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=pants_kitF.fmdl, count=1662)",
        "Info export_identified [Keep] (team=/co/, id=714)",
        "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=pants_kitF.fmdl)",
    ];

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    assert_eq!(
        findings_of(&check.messages(), "co Midcup Fallback"),
        checked
    );
    assert_eq!(check.exit_code(), 0);

    let entries = compile_clean(
        &sandbox,
        "co Midcup Fallback",
        &[
            &checked[..],
            &[
                "Info team_colors_missing [Keep] ()",
                "Warning kit_variant_model_left_out [Keep] at Players/05 - A (model=pants_kitN.fmdl, used=pants_kitF.fmdl)",
            ],
        ]
        .concat(),
    );

    let meshes = |bytes: &[u8]| {
        Model::from_file(&FmdlFile::read(bytes).unwrap())
            .unwrap()
            .meshes
            .len()
    };
    assert_eq!(
        meshes(face_package(&entries).get("fcl_hair.fmdl").unwrap()),
        meshes(&tracer_player_file("fcl_hair.fmdl")),
        "the face model is built from the `kitF` file alone"
    );
}

// TC-CMN-05
#[test]
fn a_per_kit_set_split_between_a_player_folder_and_its_face_subfolder_is_one_set_on_pes_21() {
    let sandbox = Sandbox::new("mod_kit_variant_face_subfolder");
    let player = "exports/co Midcup Variants/Players/05 - A";
    let kit_1 = tracer_player_file("fcl_hair.fmdl");
    sandbox.write(&format!("{player}/face/pants_kit1.fmdl"), &kit_1);
    sandbox.write(
        &format!("{player}/pants_kit2.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );

    let entries = compile_clean(
        &sandbox,
        "co Midcup Variants",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face/pants_kit1.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=face/pants_kit1.fmdl)",
            "Info team_colors_missing [Keep] ()",
            "Warning kit_variant_model_left_out [Keep] at Players/05 - A (model=pants_kitN.fmdl, used=pants_kit1.fmdl)",
        ],
    );

    let hair = face_package(&entries);
    let meshes = |bytes: &[u8]| {
        Model::from_file(&FmdlFile::read(bytes).unwrap())
            .unwrap()
            .meshes
            .len()
    };
    assert_eq!(
        meshes(hair.get("fcl_hair.fmdl").unwrap()),
        meshes(&kit_1),
        "kit 1's meshes alone"
    );
}

// TC-CMN-05
#[test]
fn a_left_out_kit_number_s_file_in_a_linked_face_is_left_out_too_on_pes_21() {
    let sandbox = Sandbox::new("mod_kit_variant_left_out_face");
    let export = "exports/co Midcup Variants";
    let player = format!("{export}/Players/05 - A");
    let kit_1 = tracer_player_file("fcl_hair.fmdl");
    sandbox.write(&format!("{player}/pants_kit1.fmdl"), &kit_1);
    sandbox.write(
        &format!("{player}/pants_kit2.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(&format!("{player}/Crocs.face"), b"");
    // A second part of kit 2, which no kit compiles: merged, it would add its mesh to the
    // hair of every kit.
    sandbox.write(
        &format!("{export}/Faces/Crocs/pants_kit2.fmdl"),
        &clean_model(),
    );

    // Crocs' own files are reported as Crocs' (`fmdl_fcl_hair_fallback`): another player
    // linking it may compile them.
    let entries = compile_clean(
        &sandbox,
        "co Midcup Variants",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=pants_kit1.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=pants_kit1.fmdl)",
            "Info fmdl_fcl_hair_fallback [Keep] at Faces/Crocs (file=pants_kit2.fmdl)",
            "Info team_colors_missing [Keep] ()",
            "Warning kit_variant_model_left_out [Keep] at Players/05 - A (model=pants_kitN.fmdl, used=pants_kit1.fmdl)",
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.face)",
        ],
    );

    let meshes = |bytes: &[u8]| {
        Model::from_file(&FmdlFile::read(bytes).unwrap())
            .unwrap()
            .meshes
            .len()
    };
    assert_eq!(
        meshes(face_package(&entries).get("fcl_hair.fmdl").unwrap()),
        meshes(&kit_1),
        "kit 1's meshes alone"
    );
}

// TC-CMN-05
#[test]
fn check_reads_a_kit_set_split_between_a_player_and_his_face_as_compile_does_on_pes_21() {
    let sandbox = Sandbox::new("mod_kit_variant_check_face");
    let export = "exports/co Midcup Variants";
    sandbox.write(
        &format!("{export}/Players/05 - A/pants_kit2.fmdl"),
        &clean_model(),
    );
    sandbox.write(&format!("{export}/Players/05 - A/X.face"), b"");
    sandbox.write(&format!("{export}/Faces/X/pants_kit1.fmdl"), &clean_model());
    // His `pants_kit2` is left out of his face, so it is no hair fallback of his.
    let checked = [
        "Info export_identified [Keep] (team=/co/, id=714)",
        "Info fmdl_fcl_hair_fallback [Keep] at Faces/X (file=pants_kit1.fmdl)",
    ];

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    assert_eq!(
        findings_of(&check.messages(), "co Midcup Variants"),
        checked
    );
    assert_eq!(check.exit_code(), 0);

    compile_clean(
        &sandbox,
        "co Midcup Variants",
        &[
            &checked[..],
            &[
                "Info team_colors_missing [Keep] ()",
                "Warning kit_variant_model_left_out [Keep] at Players/05 - A (model=pants_kitN.fmdl, used=pants_kit1.fmdl)",
                "Info link_combined [Keep] at Players/05 - A (link=X.face)",
            ],
        ]
        .concat(),
    );
}

// TC-CMN-05
#[test]
fn a_per_kit_set_split_between_a_player_and_a_boots_folder_he_combines_is_one_set_on_pes_21() {
    let sandbox = Sandbox::new("mod_kit_variant_combined");
    let export = "exports/co Midcup Variant Crocs";
    let boots = tracer_player_file("boots.fmdl");
    // Four bytes no FMDL reader accepts: the deep pass reads his roles over both sources, as
    // his boots package does, so it leaves `boots_kit2` unread.
    sandbox.write(&format!("{export}/Players/05 - A/boots_kit2.fmdl"), b"junk");
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    sandbox.write(&format!("{export}/Boots/Crocs/boots.fmdl"), &boots);
    sandbox.write(&format!("{export}/Boots/Crocs/boots_kit1.fmdl"), &boots);

    let entries = compile_clean(
        &sandbox,
        "co Midcup Variant Crocs",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots_kit1.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Warning kit_variant_model_left_out [Keep] at Players/05 - A (model=boots_kitN.fmdl, used=boots_kit1.fmdl)",
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=boots.fmdl)",
        ],
    );

    let meshes = Model::from_file(&FmdlFile::read(&boots).unwrap())
        .unwrap()
        .meshes
        .len();
    assert_eq!(
        boots_mesh_count(&entries, "Asset/model/character/boots/k0625/#Win/boots.fpk"),
        2 * meshes,
        "the shared `boots.fmdl` and kit 1's, not kit 2's"
    );
}

// TC-CMN-05
#[test]
fn per_kit_model_files_on_pes_21_are_a_set_and_only_the_lowest_is_converted() {
    let sandbox = Sandbox::new("mod_kit_variant_pre_fox_models");
    let player = "exports/co Midcup Variant Cards/Players/05 - A";
    for variant in ["pants_kit1", "pants_kit2"] {
        sandbox.write(&format!("{player}/{variant}.model"), &card_model());
        sandbox.write(&format!("{player}/{variant}.mtl"), &card_materials());
    }
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());

    let entries = compile_clean(
        &sandbox,
        "co Midcup Variant Cards",
        &[
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=pants_kit1.model)",
            "Info team_colors_missing [Keep] ()",
            "Warning kit_variant_model_left_out [Keep] at Players/05 - A (model=pants_kitN.model, used=pants_kit1.model)",
        ],
    );

    // The card's one mesh and the anti-blur mesh the FMDL export makes for it: one variant's,
    // `pants_kit2.model` being read by no task.
    let hair = face_package(&entries);
    let file = FmdlFile::read(hair.get("fcl_hair.fmdl").unwrap()).unwrap();
    assert_eq!(Model::from_file(&file).unwrap().meshes.len(), 2);
}

// TC-CMN-05
#[test]
fn per_kit_boots_on_pes_21_are_boots_read_without_their_kit_token() {
    let sandbox = Sandbox::new("mod_kit_variant_boots");
    let player = "exports/co Midcup Variant Boots/Players/05 - A";
    sandbox.write(&format!("{player}/boots_kit1.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/boots_kit2.fmdl"), &clean_model());

    let entries = compile_clean(
        &sandbox,
        "co Midcup Variant Boots",
        &[
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Warning kit_variant_model_left_out [Keep] at Players/05 - A (model=boots_kitN.fmdl, used=boots_kit1.fmdl)",
        ],
    );

    let boots = "Asset/model/character/boots/k0625/#Win/boots.fpk";
    assert!(entries.contains_key(boots), "no boots package");
    assert!(
        face_package(&entries).get("fcl_hair.fmdl").is_none(),
        "a variant was merged into the face's hair"
    );
}

// TC-MOD-62
#[test]
fn a_left_out_kit_variant_that_does_not_parse_is_not_read_on_pes_21() {
    let sandbox = Sandbox::new("mod_kit_variant_broken");
    let player = "exports/co Midcup Variant Boots/Players/05 - A";
    sandbox.write(&format!("{player}/face.fmdl"), &clean_model());
    let boots = tracer_player_file("boots.fmdl");
    sandbox.write(&format!("{player}/boots_kit1.fmdl"), &boots);
    // Four bytes no FMDL reader accepts.
    sandbox.write(&format!("{player}/boots_kit2.fmdl"), b"junk");

    let entries = compile_clean(
        &sandbox,
        "co Midcup Variant Boots",
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots_kit1.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=face.fmdl)",
            "Info team_colors_missing [Keep] ()",
            "Warning kit_variant_model_left_out [Keep] at Players/05 - A (model=boots_kitN.fmdl, used=boots_kit1.fmdl)",
        ],
    );

    let package =
        fpk::FpkFile::read(&entries["Asset/model/character/boots/k0625/#Win/boots.fpk"]).unwrap();
    let meshes = |bytes: &[u8]| {
        Model::from_file(&FmdlFile::read(bytes).unwrap())
            .unwrap()
            .meshes
            .len()
    };
    assert_eq!(meshes(package.get("boots.fmdl").unwrap()), meshes(&boots));
}

// TC-MOD-56
#[test]
fn per_kit_boots_in_a_shared_boots_folder_are_named_as_boots() {
    let sandbox = Sandbox::new("mod_kit_variant_shared_boots");
    let export = "exports/co Midcup Variant Crocs";
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    sandbox.write(
        &format!("{export}/Boots/Crocs/boots_kit1.fmdl"),
        &clean_model(),
    );
    sandbox.write(
        &format!("{export}/Boots/Crocs/boots_kit2.fmdl"),
        &clean_model(),
    );

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    assert_eq!(
        findings_of(&check.messages(), "co Midcup Variant Crocs"),
        ["Info export_identified [Keep] (team=/co/, id=714)"]
    );
    assert_eq!(check.exit_code(), 0);

    // Compiled, the shared folder's set collapses to its lowest variant, as a player's does.
    let entries = compile_clean(
        &sandbox,
        "co Midcup Variant Crocs",
        &[
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Warning kit_variant_model_left_out [Keep] at Boots/Crocs (model=boots_kitN.fmdl, used=boots_kit1.fmdl)",
        ],
    );
    let variant_meshes = Model::from_file(&FmdlFile::read(&clean_model()).unwrap())
        .unwrap()
        .meshes
        .len();
    assert_eq!(
        boots_mesh_count(&entries, "Asset/model/character/boots/k0625/#Win/boots.fpk"),
        variant_meshes,
        "kit 1's meshes alone"
    );
}

// TC-STR-17
#[test]
fn an_unsuffixed_model_below_a_shared_boots_folder_s_subfolder_is_named_invalid() {
    let sandbox = Sandbox::new("mod_shared_boots_nested");
    let export = "exports/co Midcup Nested Crocs";
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    sandbox.write(&format!("{export}/Boots/Crocs/boots.fmdl"), &clean_model());
    sandbox.write(
        &format!("{export}/Boots/Crocs/boots/extra.fmdl"),
        &clean_model(),
    );

    // The name rule applies at any depth and drops as it does a model directly in the
    // folder: `extra.fmdl` is no boots part, and no package reads it.
    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    let lines = check.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Nested Crocs"),
        [
            "Error fmdl_name_invalid [DropFolder] at Boots/Crocs (file=boots/extra.fmdl)",
            "Error link_target_dropped [DropFolder] at Players/05 - A (link=Crocs.boots, target=Boots/Crocs, finding=fmdl_name_invalid)",
            "Info export_identified [Keep] (team=/co/, id=714)",
        ],
        "{lines:#?}"
    );
}

// TC-MOD-56
#[test]
fn a_shared_boots_folder_s_per_kit_models_on_pes_17_collapse_to_the_lowest_variant() {
    let sandbox = Sandbox::new("mod_kit_variant_shared_boots_pes17");
    let export = "co Midcup Variant Studs";
    sandbox.write(&format!("exports/{export}/Players/05 - A/Studs.boots"), b"");
    let studs = format!("exports/{export}/Boots/Studs");
    for variant in ["boots_kit1", "boots_kit2"] {
        sandbox.write(&format!("{studs}/{variant}.model"), &card_model());
    }
    sandbox.write(&format!("{studs}/boots.mtl"), &materials_naming("studs"));
    sandbox.write(&format!("{studs}/studs.dds"), &small_dds());

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            CLEAN[0],
            CLEAN[1],
            "Warning kit_variant_model_left_out [Keep] at Boots/Studs (model=boots_kitN.model, used=boots_kit1.model)",
        ],
    );

    // The one `boots.model` every linking player wears is kit 1's alone, not the two merged.
    let boots = entries_under(&entries, BOOTS_K0625);
    let names: Vec<&str> = boots.keys().copied().collect();
    assert_eq!(names, ["boots.model", "boots.mtl", "studs.dds"]);
    let mesh_count = |bytes: &[u8]| {
        let file = pes_model::format::PreFoxModel::read(bytes).unwrap();
        pes_model::model::Model::from_file(&file)
            .unwrap()
            .meshes
            .len()
    };
    assert_eq!(
        mesh_count(boots["boots.model"]),
        mesh_count(&card_model()),
        "kit 1's meshes alone"
    );
    assert!(
        *boots["boots.model"] == card_model(),
        "kit 1's model as it is"
    );
}

/// The number of faces of the model `name` in the package at `path` in `entries`, read back
/// with `fmdl`.
fn face_count(entries: &BTreeMap<String, Vec<u8>>, path: &str, name: &str) -> usize {
    let package = fpk::FpkFile::read(&entries[path]).unwrap();
    let model = Model::from_file(&FmdlFile::read(package.get(name).unwrap()).unwrap()).unwrap();
    model.meshes.iter().map(|mesh| mesh.faces.len()).sum()
}

// TC-MOD-31
#[test]
fn a_face_model_weighted_to_the_hand_bones_gives_its_hands_to_the_player_s_gloves() {
    let sandbox = Sandbox::new("mod_hand_split");
    install_tables(
        &sandbox,
        &[(BOOTS_LIST, &item_list(&[])), (GLOVE_LIST, &item_list(&[]))],
    );
    // A full-body model with both hands on (`tests/fixtures/hand_split/README.md`): 40
    // faces, of which each hand's split takes 8 and the body keeps 24. Slot 05 holds it as
    // a face model, slot 06 the same bytes as boots, which are never split.
    let body =
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hand_split/body.fmdl"))
            .unwrap();
    let export = "exports/co Midcup Hands";
    sandbox.write(&format!("{export}/Players/05 - A/body.fmdl"), &body);
    sandbox.write(&format!("{export}/Players/06 - B/boots.fmdl"), &body);

    let entries = compile_clean(
        &sandbox,
        "co Midcup Hands",
        &[
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=body.fmdl)",
            "Info team_colors_missing [Keep] ()",
            "Info model_hand_split [Keep] at Players/05 - A (model=body.fmdl, gloves=glove_l, glove_r)",
        ],
    );

    let gloves = "Asset/model/character/glove/g0625/#Win/glove.fpk";
    assert_eq!(
        package_names(&entries[gloves]),
        ["glove_l.fmdl", "glove_r.fmdl"]
    );
    let face = "Asset/model/character/face/real/71405/#Win/face.fpk";
    let split = [
        face_count(&entries, gloves, "glove_l.fmdl"),
        face_count(&entries, gloves, "glove_r.fmdl"),
        face_count(&entries, face, "fcl_hair.fmdl"),
    ];
    assert_eq!(split, [8, 8, 24]);
    assert_eq!(
        split.iter().sum::<usize>(),
        40,
        "every face of the source, once"
    );
    assert_eq!(pairs_of(&entries, GLOVE_LIST), [(71405, 625)]);

    // Slot 06's boots keep the whole model, and give no gloves.
    let boots = "Asset/model/character/boots/k0626/#Win/boots.fpk";
    assert_eq!(face_count(&entries, boots, "boots.fmdl"), 40);
    assert!(
        !entries
            .keys()
            .any(|path| path.starts_with("Asset/model/character/glove/g0626/")),
        "no gloves for slot 06"
    );
    assert_eq!(
        pairs_of(&entries, BOOTS_LIST),
        [(71406, 626)],
        "slot 06's boots"
    );
}

/// The FTEX of slot 05's texture at `below`, its path below the folder without extension, in
/// team 714's PES 21 output: at its directory below the folder, then `sourceimages/` and the
/// platform folder.
fn slot_05_ftex(below: &str) -> String {
    let (directory, name) = below
        .rsplit_once('/')
        .map_or(("", below), |(directory, name)| (directory, name));
    let directory = if directory.is_empty() {
        String::new()
    } else {
        format!("{directory}/")
    };
    format!("Asset/model/character/common/714/05 - A/{directory}sourceimages/#windx11/{name}.ftex")
}

/// The directory an FMDL names slot 05's texture at `subdirectory` below the folder by (`""` or
/// ending in `/`), in team 714's PES 21 output.
fn slot_05_directory(subdirectory: &str) -> String {
    format!("/Assets/pes16/model/character/common/714/05 - A/{subdirectory}sourceimages/")
}

// TC-MOD-65
#[test]
fn a_subfolder_s_fmdl_names_the_texture_nearest_it_at_its_path_on_pes_21() {
    // The root's `shorts.dds` and `jessie/shorts.dds` are two textures, each at its own path;
    // `jessie/hair_high.fmdl` names the nearer one.
    let sandbox = Sandbox::new("mod_subfolder_textures_fox");
    let export = "co Midcup Subfolders";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(
        &format!("{player}/jessie/hair_high.fmdl"),
        &tracer_model_renaming("fcl_hair.fmdl", &[("shirt.dds", "shorts.dds")]),
    );
    sandbox.write(
        &format!("{player}/shorts.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(&format!("{player}/jessie/shorts.dds"), &small_dds());

    let entries = compile_clean(
        &sandbox,
        export,
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=jessie/hair_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let root = slot_05_ftex("shorts");
    let nearest = slot_05_ftex("jessie/shorts");
    assert!(entries.contains_key(&root), "{:#?}", entries.keys());
    assert!(entries.contains_key(&nearest), "{:#?}", entries.keys());
    assert_ne!(entries[&root], entries[&nearest]);
    // The subfolder's face part is a model of the face package beside the root's.
    let package = face_package(&entries);
    let models: Vec<&str> = package
        .entries()
        .map(|(name, _)| name)
        .filter(|name| name.ends_with(".fmdl"))
        .collect();
    assert_eq!(models, ["face_high.fmdl", "hair_high.fmdl"]);
    // Every entry of its texture table naming `shorts.dds` names the nearer one.
    let directories = texture_directories(package.get("hair_high.fmdl").unwrap(), "shorts.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| *directory == slot_05_directory("jessie/")),
        "{directories:?}"
    );
}

// TC-MOD-65
#[test]
fn a_subfolder_s_texture_comes_before_a_combined_shared_folder_s_of_its_stem_on_pes_21() {
    // `Boots/Crocs`, combined into slot 05's boots, holds a `skin.dds` of other bytes, which
    // sits at the root of his texture home; `jessie/hair_high.fmdl` names its own folder's.
    let sandbox = Sandbox::new("mod_subfolder_texture_before_combined");
    let export = "co Midcup Subfolders";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(
        &format!("{player}/jessie/hair_high.fmdl"),
        &tracer_model_renaming("fcl_hair.fmdl", &[("shirt.dds", "skin.dds")]),
    );
    sandbox.write(&format!("{player}/jessie/skin.dds"), &small_dds());
    // His own boots, which the link combines with, and the texture the tracer's boots name.
    sandbox.write(
        &format!("{player}/kit_boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(
        &format!("{player}/shirt.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(&format!("{player}/Crocs.boots"), b"");
    sandbox.write(
        &format!("exports/{export}/Boots/Crocs/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(
        &format!("exports/{export}/Boots/Crocs/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let entries = compile_clean(
        &sandbox,
        export,
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=jessie/hair_high.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=kit_boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=boots.fmdl)",
        ],
    );

    let subfolder = slot_05_ftex("jessie/skin");
    let combined = slot_05_ftex("skin");
    assert!(entries.contains_key(&subfolder), "{:#?}", entries.keys());
    assert!(entries.contains_key(&combined), "{:#?}", entries.keys());
    assert_ne!(entries[&subfolder], entries[&combined]);
    let package = face_package(&entries);
    let directories = texture_directories(package.get("hair_high.fmdl").unwrap(), "skin.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| *directory == slot_05_directory("jessie/")),
        "{directories:?}"
    );
}

// TC-MOD-71
#[test]
fn a_fmdl_texture_named_by_its_path_below_the_model_s_folder_resolves_there_on_pes_21() {
    let sandbox = Sandbox::new("mod_texture_path_below");
    let export = "co Midcup Textures";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &tracer_model_pointing("fcl_hair.fmdl", &[("shirt.dds", "skin.dds")], "./textures/"),
    );
    sandbox.write(&format!("{player}/textures/skin.dds"), &small_dds());

    // No finding names `skin`.
    let entries = compile_clean(
        &sandbox,
        export,
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let package = face_package(&entries);
    let directories = texture_directories(package.get("face_high.fmdl").unwrap(), "skin.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| *directory == slot_05_directory("textures/")),
        "{directories:?}"
    );
    let skin = slot_05_ftex("textures/skin");
    assert!(entries.contains_key(&skin), "{:#?}", entries.keys());
}

// TC-MOD-71
#[test]
fn a_fmdl_texture_named_by_its_path_below_the_model_s_folder_resolves_to_a_link_there_on_pes_21() {
    // `./jessie/` names `jessie/` below the folder, where the link stands for
    // `Common/jessie/hair.dds`.
    let sandbox = Sandbox::new("mod_texture_path_below_link");
    let export = "co Midcup Textures";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &tracer_model_pointing("fcl_hair.fmdl", &[("shirt.dds", "hair.dds")], "./jessie/"),
    );
    sandbox.write(&format!("{player}/jessie/hair.dds.common"), b"");
    sandbox.write(
        &format!("exports/{export}/Common/jessie/hair.dds"),
        &tracer_player_file("shirt.dds"),
    );

    // No finding names `hair`.
    let entries = compile_clean(
        &sandbox,
        export,
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let package = face_package(&entries);
    let directories = texture_directories(package.get("face_high.fmdl").unwrap(), "hair.dds");
    assert!(!directories.is_empty());
    assert!(
        directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/jessie/sourceimages/"),
        "{directories:?}"
    );
    let hair = "Asset/model/character/common/714/jessie/sourceimages/#windx11/hair.ftex";
    assert!(entries.contains_key(hair), "{:#?}", entries.keys());
}

// TC-MOD-73
#[test]
fn a_combined_folder_s_files_stand_at_their_paths_below_the_player_folder() {
    // `Faces/Round`'s `jessie/` files are at `jessie/` below slot 05's folder: the `skin`
    // `jessie/hair_high.fmdl` names resolves to `jessie/skin`, not the player's root `skin`.
    let sandbox = Sandbox::new("mod_combined_subfolder");
    let export = "co Midcup Subfolders";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(
        &format!("{player}/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(&format!("{player}/Round.face"), b"");
    let round = format!("exports/{export}/Faces/Round");
    sandbox.write(
        &format!("{round}/jessie/hair_high.fmdl"),
        &hair_model_naming("skin.dds"),
    );
    sandbox.write(&format!("{round}/jessie/skin.dds"), &small_dds());

    let entries = compile_clean(
        &sandbox,
        export,
        &[
            "Info fmdl_weights_not_normalized [Keep] at Faces/Round (file=jessie/hair_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
        ],
    );

    // The face package holds both models' meshes, `jessie/`'s part naming the texture at
    // `jessie/` in his texture home, which holds both `skin.ftex` at their paths.
    let package = face_package(&entries);
    let models: Vec<&str> = package
        .entries()
        .map(|(name, _)| name)
        .filter(|name| name.ends_with(".fmdl"))
        .collect();
    assert_eq!(models, ["face_high.fmdl", "hair_high.fmdl"]);
    let own = slot_05_ftex("skin");
    let combined = slot_05_ftex("jessie/skin");
    assert!(entries.contains_key(&own), "{:#?}", entries.keys());
    assert!(entries.contains_key(&combined), "{:#?}", entries.keys());
    assert_ne!(entries[&own], entries[&combined]);
    let directories = texture_directories(package.get("hair_high.fmdl").unwrap(), "skin.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| *directory == slot_05_directory("jessie/")),
        "{directories:?}"
    );
}

// TC-MOD-75
#[test]
fn a_shared_folder_s_texture_link_names_its_common_texture_on_pes_21() {
    // `jessie/armor_bsm.dds.common` stands for `Common/jessie/armor_bsm.dds` in `Boots/Crocs`
    // as it would in a player folder: `boots.fmdl`'s `./jessie/armor_bsm` resolves to it.
    let sandbox = Sandbox::new("mod_shared_texture_link");
    let export = "co Midcup Crocs";
    sandbox.write(&format!("exports/{export}/Players/05 - A/Crocs.boots"), b"");
    let crocs = format!("exports/{export}/Boots/Crocs");
    sandbox.write(
        &format!("{crocs}/boots.fmdl"),
        &tracer_model_pointing("boots.fmdl", &[("shirt.dds", "armor_bsm.dds")], "./jessie/"),
    );
    sandbox.write(&format!("{crocs}/jessie/armor_bsm.dds.common"), b"");
    sandbox.write(
        &format!("exports/{export}/Common/jessie/armor_bsm.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let entries = compile_clean(
        &sandbox,
        export,
        &[
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let package =
        fpk::FpkFile::read(&entries["Asset/model/character/boots/k0625/#Win/boots.fpk"]).unwrap();
    let directories = texture_directories(package.get("boots.fmdl").unwrap(), "armor_bsm.dds");
    assert!(!directories.is_empty());
    assert!(
        directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/jessie/sourceimages/"),
        "{directories:?}"
    );
    let armor = "Asset/model/character/common/714/jessie/sourceimages/#windx11/armor_bsm.ftex";
    assert!(entries.contains_key(armor), "{:#?}", entries.keys());
}

// TC-MOD-75
#[test]
fn a_shared_folder_s_texture_link_takes_the_common_file_s_spelling_on_pes_21() {
    // As a player's link does: the texture's path below `Common/` is spelled as `Common/`
    // spells it (`Common/JESSIE/Armor_Bsm.dds`), so the model's directory and the FTEX are
    // `JESSIE/`'s, not the link's `jessie/`.
    let sandbox = Sandbox::new("mod_shared_texture_spelling");
    let export = "co Midcup Crocs";
    sandbox.write(&format!("exports/{export}/Players/05 - A/Crocs.boots"), b"");
    let crocs = format!("exports/{export}/Boots/Crocs");
    sandbox.write(
        &format!("{crocs}/boots.fmdl"),
        &tracer_model_pointing("boots.fmdl", &[("shirt.dds", "armor_bsm.dds")], "./jessie/"),
    );
    sandbox.write(&format!("{crocs}/jessie/armor_bsm.dds.common"), b"");
    sandbox.write(
        &format!("exports/{export}/Common/JESSIE/Armor_Bsm.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let entries = compile_clean(
        &sandbox,
        export,
        &[
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let package =
        fpk::FpkFile::read(&entries["Asset/model/character/boots/k0625/#Win/boots.fpk"]).unwrap();
    let directories = texture_directories(package.get("boots.fmdl").unwrap(), "armor_bsm.dds");
    assert!(!directories.is_empty());
    assert!(
        directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/JESSIE/sourceimages/"),
        "{directories:?}"
    );
    let armor = "Asset/model/character/common/714/JESSIE/sourceimages/#windx11/Armor_Bsm.ftex";
    assert!(entries.contains_key(armor), "{:#?}", entries.keys());
}

// TC-MOD-71
#[test]
fn a_fmdl_texture_named_by_its_path_is_not_found_by_its_name_elsewhere_on_pes_21() {
    // The root's `skin.dds` is not at `./textures/`.
    let sandbox = Sandbox::new("mod_texture_path_below_missing");
    let export = "co Midcup Textures";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &tracer_model_pointing("fcl_hair.fmdl", &[("shirt.dds", "skin.dds")], "./textures/"),
    );
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    // Another player, so the run writes a CPK without slot 05's face.
    sandbox.write(
        &format!("exports/{export}/Players/03 - C/face_high.fmdl"),
        &clean_model(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert!(
        findings_of(&lines, export).contains(
            &"Error fmdl_texture_not_found [DropFolder] at Players/05 - A (model=face_high.fmdl, texture=./textures/skin.dds)"
        ),
        "{lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        !entries
            .keys()
            .any(|path| path.contains("/face/real/71405/")),
        "{:#?}",
        entries.keys()
    );
}

// TC-MOD-71
#[test]
fn a_converted_model_s_mtl_path_below_its_folder_names_the_texture_there_on_pes_21() {
    // `jessie/x_hair_high.mtl` names `./shorts/y.dds`: the converted model names the texture
    // at `jessie/shorts/`, not the root's `y` of other bytes.
    let sandbox = Sandbox::new("mod_mtl_path_below_fox");
    let export = "co Midcup Textures";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/jessie/x_hair_high.model"), &card_model());
    sandbox.write(
        &format!("{player}/jessie/x_hair_high.mtl"),
        &materials_naming("shorts/y"),
    );
    sandbox.write(&format!("{player}/jessie/shorts/y.dds"), &small_dds());
    sandbox.write(&format!("{player}/y.dds"), &tracer_player_file("shirt.dds"));

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        !findings_of(&lines, export)
            .iter()
            .any(|line| line.contains("texture_not_found")),
        "{lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));

    let package = face_package(&entries);
    let directories = texture_directories(package.get("hair_high.fmdl").unwrap(), "y.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| *directory == slot_05_directory("jessie/shorts/")),
        "{directories:?}"
    );
    let below = slot_05_ftex("jessie/shorts/y");
    let root = slot_05_ftex("y");
    assert!(entries.contains_key(&below), "{:#?}", entries.keys());
    assert!(entries.contains_key(&root), "{:#?}", entries.keys());
    assert_ne!(entries[&below], entries[&root]);
}

// TC-MOD-66
#[test]
fn a_converted_model_s_texture_names_resolve_in_its_mtl_s_folder_on_pes_21() {
    // `jessie/hair_high.model`'s `.mtl` is the root's `hair_high.mtl`, which names `skin`:
    // `skin` resolves in the `.mtl`'s own folder, the root, not in `jessie/`'s.
    let sandbox = Sandbox::new("mod_mtl_stem_resolution_fox");
    let export = "co Midcup Textures";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/jessie/hair_high.model"), &card_model());
    sandbox.write(&format!("{player}/hair_high.mtl"), &card_materials());
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    sandbox.write(
        &format!("{player}/jessie/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        !findings_of(&lines, export)
            .iter()
            .any(|line| line.contains("texture_not_found")),
        "{lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));

    let package = face_package(&entries);
    let directories = texture_directories(package.get("hair_high.fmdl").unwrap(), "skin.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| *directory == HOME_714_05),
        "{directories:?}"
    );
}

// TC-MOD-92
#[test]
fn a_txt_note_beside_a_face_model_is_a_file_no_role_takes() {
    let sandbox = Sandbox::new("txt_note");
    let export = "co Midcup Note";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/notes.txt"), b"a note");
    let settings = format!(
        "{}[team-compiler]\nstrict_file_type_check = true\n",
        pes21_settings(&sandbox)
    );

    let run = sandbox.run(&settings, &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Warning file_not_used [Keep] at Players/05 - A (file=notes.txt)",
            TEAM_COLORS_MISSING,
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0);
    // The folder compiles as if the note were absent.
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(face_package(&entries).get("face_high.fmdl").is_some());
    assert!(face_package(&entries).get("notes.txt").is_none());
    assert!(
        !entries.keys().any(|path| path.ends_with("notes.txt")),
        "{:#?}",
        entries.keys()
    );
}
