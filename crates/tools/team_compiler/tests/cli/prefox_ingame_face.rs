//! `compile` for PES 2017 of a player folder holding `ingame_face`, whose models other than
//! gloves become his own boots, merged with a boots folder his link combines, and whose gloves
//! become his own gloves folder, unmerged, with a gloves folder his link combines, each with
//! the Common models and `.mtl` files his `.common` links copy in, his `.fmdl` parts converted;
//! and of the
//! shared folders written in the same shapes: a `Boots/` folder holding several boots models,
//! a `Gloves/` folder holding a `.mtl` no model uses.

use pes_model::format::PreFoxModel;
use pes_model::model::Model;

use crate::common::Sandbox;
use crate::compile::{cpk_entries, tracer_player_file};
use crate::prefox_faces::{
    BOOTS_K0625, CLEAN, card_materials, card_model, compile_pes17, complete_pes17, entries_under,
    face_cpk, pes17, pre_fox_fixture, sampler_paths, small_dds, write_slot_05_face,
};
use crate::textures::tracer_model_renaming;
use crate::{clean_model, findings_of};

/// The folder slot 05's own gloves are written to in team 714's export: his exclusive id.
const GLOVES_G0625: &str = "common/character0/model/character/glove/g0625/";

/// The folder of every pre-Fox gloves output of the CPK, shared and exclusive.
const GLOVES: &str = "common/character0/model/character/glove/";

/// Slot 05's common folder in team 714's Common output, where his textures are and his `.mtl`
/// files name them.
const SLOT_05_HOME: &str = "model/character/uniform/common/714/05 - A/";

/// The card-head template's face model, a clean pre-Fox model with one material, `card`.
fn card() -> Model {
    Model::from_file(&PreFoxModel::read(&card_model()).unwrap()).unwrap()
}

/// `model` written as a `.model` file.
fn written(model: &Model) -> Vec<u8> {
    model.to_file().unwrap().write().unwrap()
}

/// The card-head model with its one material, `card`, renamed `material`.
fn card_naming(material: &str) -> Vec<u8> {
    let mut model = card();
    assert_eq!(model.materials, ["card"]);
    model.materials[0] = material.to_owned();
    written(&model)
}

/// The card-head template's material set, its one material `card` renamed `material` and its
/// one texture path `./texture.dds` renamed `./<stem>.dds`.
fn materials(material: &str, stem: &str) -> Vec<u8> {
    let text = String::from_utf8(pre_fox_fixture("cardhead_materials.mtl")).unwrap();
    assert!(text.contains("name=\"card\""), "{text}");
    assert!(text.contains("./texture.dds"), "{text}");
    text.replace("name=\"card\"", &format!("name=\"{material}\""))
        .replace("./texture.dds", &format!("./{stem}.dds"))
        .into_bytes()
}

/// The number of meshes of the `.model` `bytes`.
fn mesh_count(bytes: &[u8]) -> usize {
    Model::from_file(&PreFoxModel::read(bytes).unwrap())
        .unwrap()
        .meshes
        .len()
}

// TC-MOD-35
#[test]
fn under_ingame_face_a_boots_link_beside_a_boots_model_merges_into_the_player_s_own_boots() {
    let sandbox = Sandbox::new("prefox_ingame_link");
    let export = "co Midcup Linked";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    sandbox.write(&format!("{player}/kit_boots.model"), &card_model());
    sandbox.write(&format!("{player}/kit_boots.mtl"), &card_materials());
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    sandbox.write(&format!("{player}/Crocs.boots"), b"");
    let crocs = format!("exports/{export}/Boots/Crocs");
    let crocs_model = card_naming("crocs");
    sandbox.write(&format!("{crocs}/boots.model"), &crocs_model);
    sandbox.write(&format!("{crocs}/boots.mtl"), &materials("crocs", "crocs"));
    sandbox.write(&format!("{crocs}/crocs.dds"), &small_dds());

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            CLEAN[0],
            CLEAN[1],
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)",
            "Info model_merged [Keep] at Players/05 - A (model=boots.model)",
        ],
    );

    assert!(!entries.contains_key(&face_cpk(5)), "{:?}", entries.keys());
    // Slot 05's exclusive folder alone: Crocs, linked by no other player, takes no id.
    let boots = entries_under(&entries, "common/character0/model/character/boots/");
    let names: Vec<&str> = boots.keys().copied().collect();
    assert_eq!(names, ["k0625/boots.model", "k0625/boots.mtl"]);
    assert_eq!(
        mesh_count(boots["k0625/boots.model"]),
        mesh_count(&crocs_model) + mesh_count(&card_model()),
        "Crocs's meshes plus the local model's"
    );
    // Crocs's part first (`boots.model` before `kit_boots.model`), each material pointed at
    // its texture in the player's common folder, which the CPK holds.
    let home = "model/character/uniform/common/714/05 - A/";
    assert_eq!(
        sampler_paths(boots["k0625/boots.mtl"]),
        [format!("{home}crocs.dds"), format!("{home}skin.dds")]
    );
    for stem in ["crocs", "skin"] {
        let texture = format!("common/character1/{home}{stem}.dds");
        assert!(entries.contains_key(&texture), "{texture}");
    }
}

// TC-MOD-35
#[test]
fn under_ingame_face_a_texture_only_the_combined_boots_folder_holds_supplies_his_own_mtl() {
    let sandbox = Sandbox::new("prefox_ingame_link_texture");
    let export = "co Midcup Linked";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    sandbox.write(&format!("{player}/boots.model"), &card_model());
    // His own `.mtl` names `studs.dds`, which only the boots folder his link combines holds.
    sandbox.write(&format!("{player}/boots.mtl"), &materials("card", "studs"));
    sandbox.write(&format!("{player}/Studs.boots"), b"");
    let studs = format!("exports/{export}/Boots/Studs");
    sandbox.write(&format!("{studs}/boots.model"), &card_naming("studs"));
    sandbox.write(&format!("{studs}/boots.mtl"), &materials("studs", "studs"));
    sandbox.write(&format!("{studs}/studs.dds"), &small_dds());

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            CLEAN[0],
            CLEAN[1],
            "Info link_combined [Keep] at Players/05 - A (link=Studs.boots)",
            "Info model_merged [Keep] at Players/05 - A (model=boots.model)",
        ],
    );

    // Both parts' paths name the one texture in his common folder, which the CPK holds.
    let boots = entries_under(&entries, BOOTS_K0625);
    let studs = format!("{SLOT_05_HOME}studs.dds");
    assert_eq!(sampler_paths(boots["boots.mtl"]), [studs.clone(), studs]);
    let texture = format!("common/character1/{SLOT_05_HOME}studs.dds");
    assert!(entries.contains_key(&texture), "{texture}");
}

// TC-MOD-35
#[test]
fn under_ingame_face_a_subfolder_part_s_texture_comes_before_the_combined_boots_folder_s() {
    // His `parts/boots.mtl` and Crocs's `boots.mtl` both name `skin.dds`: his part's resolves
    // in `parts/` first, Crocs's, whose folder is no folder of his tree, as his root's would,
    // among the combined folder's textures.
    let sandbox = Sandbox::new("prefox_ingame_subfolder_texture");
    let export = "co Midcup Linked";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    sandbox.write(&format!("{player}/parts/boots.model"), &card_model());
    sandbox.write(&format!("{player}/parts/boots.mtl"), &card_materials());
    sandbox.write(&format!("{player}/parts/skin.dds"), &small_dds());
    sandbox.write(&format!("{player}/Crocs.boots"), b"");
    let crocs = format!("exports/{export}/Boots/Crocs");
    sandbox.write(&format!("{crocs}/boots.model"), &card_naming("crocs"));
    sandbox.write(&format!("{crocs}/boots.mtl"), &materials("crocs", "skin"));
    sandbox.write(
        &format!("{crocs}/skin.png"),
        &crate::textures::texture_fixture("skin.png"),
    );

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            CLEAN[0],
            CLEAN[1],
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)",
            "Info model_merged [Keep] at Players/05 - A (model=boots.model)",
        ],
    );

    // The parts merge into one `boots.model` and `boots.mtl`, Crocs's part first.
    let boots = entries_under(&entries, BOOTS_K0625);
    assert_eq!(
        sampler_paths(boots["boots.mtl"]),
        [
            format!("{SLOT_05_HOME}skin.dds"),
            format!("{SLOT_05_HOME}parts/skin.dds")
        ]
    );
    for texture in ["skin", "parts/skin"] {
        let texture = format!("common/character1/{SLOT_05_HOME}{texture}.dds");
        assert!(entries.contains_key(&texture), "{texture}");
    }
}

// TC-CMN-22
#[test]
fn under_ingame_face_a_subfolder_s_texture_link_comes_before_the_root_s_texture_of_its_stem() {
    // His `jessie/boots.fmdl` names `skin`, which both `jessie/skin.dds.common` stands for in
    // `jessie/` and his root's `skin.dds` is: the link's Common texture wins, as `jessie/`'s
    // own texture would.
    let sandbox = Sandbox::new("prefox_ingame_nested_texture_link");
    let export = "co Midcup Linked";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    sandbox.write(
        &format!("{player}/jessie/boots.fmdl"),
        &tracer_model_renaming("boots.fmdl", &[("shirt.dds", "skin.dds")]),
    );
    sandbox.write(&format!("{player}/jessie/skin.dds.common"), b"");
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    sandbox.write(
        &format!("exports/{export}/Common/jessie/skin.dds"),
        &small_dds(),
    );

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=jessie/boots.fmdl, count=1662)",
            CLEAN[0],
            CLEAN[1],
            "Info native_field_dropped [Keep] at Players/05 - A (model=jessie/boots.fmdl, field=bone_matrices)",
            "Warning mesh_flags_dropped [Keep] at Players/05 - A (model=jessie/boots.fmdl, material=1, field=no_shadow_cast)",
        ],
    );

    let boots = entries_under(&entries, BOOTS_K0625);
    assert!(
        sampler_paths(boots["boots.mtl"])
            .contains(&"model/character/uniform/common/714/jessie/skin.dds".to_owned()),
        "{:?}",
        sampler_paths(boots["boots.mtl"])
    );
}

/// Writes slot 07 of the export `export`: `face_high.model`, its `.mtl` and `skin.dds`, a
/// clean player beside slot 05, so the CPK is written whatever slot 05 gives.
fn write_slot_07_face(sandbox: &Sandbox, export: &str) {
    let player = format!("exports/{export}/Players/07 - B");
    sandbox.write(&format!("{player}/face_high.model"), &card_model());
    sandbox.write(&format!("{player}/face_high.mtl"), &card_materials());
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
}

/// Runs `compile --no-deploy` for PES 17 over `sandbox`, asserts the export `name`'s findings
/// are `CLEAN` then `failure`, that the run failed, and that the CPK holds slot 07's face and
/// no `boots/k0625/` entry.
fn assert_boots_left_out(sandbox: &Sandbox, name: &str, failure: &str) {
    complete_pes17(sandbox, name);
    let run = sandbox.run(&pes17(sandbox), &["compile", "--no-deploy"]);
    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, name),
        [CLEAN[0], CLEAN[1], failure],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(entries.contains_key(&face_cpk(7)), "{:?}", entries.keys());
    assert!(
        !entries.keys().any(|path| path.starts_with(BOOTS_K0625)),
        "{:?}",
        entries.keys()
    );
}

// TC-MOD-41
#[test]
fn under_ingame_face_boots_parts_defining_one_material_differently_leave_the_boots_out() {
    let sandbox = Sandbox::new("prefox_ingame_material");
    let export = "co Midcup Skin";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    for part in ["shirt", "socks"] {
        sandbox.write(&format!("{player}/{part}.model"), &card_naming("skin"));
        sandbox.write(&format!("{player}/{part}.mtl"), &materials("skin", part));
        sandbox.write(&format!("{player}/{part}.dds"), &small_dds());
    }
    write_slot_07_face(&sandbox, export);

    assert_boots_left_out(
        &sandbox,
        export,
        "Error merge_material_conflict [DropFolder] at Players/05 - A (material=skin)",
    );
}

// TC-MOD-41
#[test]
fn under_ingame_face_boots_parts_naming_one_bone_with_different_transforms_leave_the_boots_out() {
    let sandbox = Sandbox::new("prefox_ingame_bone");
    let export = "co Midcup Bones";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    let mut moved = card();
    moved.bones[0].matrix[3] += 1.0;
    sandbox.write(&format!("{player}/shirt.model"), &card_model());
    sandbox.write(&format!("{player}/socks.model"), &written(&moved));
    // One material set both parts use.
    sandbox.write(&format!("{player}/materials.mtl"), &card_materials());
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    write_slot_07_face(&sandbox, export);

    assert_boots_left_out(
        &sandbox,
        export,
        &format!(
            "Error skl_merge_conflict [DropFolder] at Players/05 - A (bone={})",
            moved.bones[0].name
        ),
    );
}

// TC-MOD-22
#[test]
fn a_shared_boots_folder_holding_two_boots_models_is_written_as_one_merged_boots_model() {
    let sandbox = Sandbox::new("prefox_shared_boots_merge");
    let export = "co Midcup Twin";
    write_slot_05_face(&sandbox, export);
    sandbox.write(&format!("exports/{export}/Players/05 - A/Twin.boots"), b"");
    let twin = format!("exports/{export}/Boots/Twin");
    let studs_model = card_naming("studs");
    sandbox.write(&format!("{twin}/a_boots.model"), &card_model());
    sandbox.write(&format!("{twin}/a_boots.mtl"), &materials("card", "left"));
    sandbox.write(&format!("{twin}/left.dds"), &small_dds());
    sandbox.write(&format!("{twin}/b_boots.model"), &studs_model);
    sandbox.write(&format!("{twin}/b_boots.mtl"), &materials("studs", "right"));
    sandbox.write(&format!("{twin}/right.dds"), &small_dds());

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            CLEAN[0],
            CLEAN[1],
            "Info model_merged [Keep] at Boots/Twin (model=boots.model)",
        ],
    );

    let boots = entries_under(&entries, BOOTS_K0625);
    let names: Vec<&str> = boots.keys().copied().collect();
    assert_eq!(names, ["boots.model", "boots.mtl", "left.dds", "right.dds"]);
    assert_eq!(
        mesh_count(boots["boots.model"]),
        mesh_count(&card_model()) + mesh_count(&studs_model)
    );
    assert_eq!(
        sampler_paths(boots["boots.mtl"]),
        ["./left.dds", "./right.dds"]
    );
}

/// Writes slot 05 of the export `export` holding `ingame_face` and `kit_boots.model`, then
/// each of `gloves`, a glove model's stem with the stem of the texture its `.mtl` names: the
/// model, a `.mtl` of its stem and the texture.
fn write_marked_slot_05(sandbox: &Sandbox, export: &str, gloves: &[(&str, &str)]) {
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    let parts = [("kit_boots", "boots")]
        .into_iter()
        .chain(gloves.iter().copied());
    for (stem, texture) in parts {
        sandbox.write(&format!("{player}/{stem}.model"), &card_model());
        sandbox.write(&format!("{player}/{stem}.mtl"), &materials("card", texture));
        sandbox.write(&format!("{player}/{texture}.dds"), &small_dds());
    }
}

/// The `glove.xml` listing `entries` in their order, each a type, a model name and a `.mtl`
/// name.
fn glove_xml(entries: &[(&str, &str, &str)]) -> String {
    let mut text = String::from("<?xml version='1.0' encoding='UTF-8'?>\r\n<config>\r\n");
    for (xml_type, model, material) in entries {
        text.push_str(&format!(
            "   <model level=\"0\" type=\"{xml_type}\" path=\"./{model}\" material=\"./{material}\" />\r\n"
        ));
    }
    text + "</config>"
}

// TC-MOD-45
#[test]
fn under_ingame_face_gloves_parts_are_written_unmerged_to_the_player_s_own_gloves_folder() {
    let sandbox = Sandbox::new("prefox_ingame_gloves");
    let export = "co Midcup Gloves";
    write_marked_slot_05(
        &sandbox,
        export,
        &[("x_gloveL", "left"), ("x_gloveR", "right")],
    );

    let entries = compile_pes17(&sandbox, export, &CLEAN);

    assert!(!entries.contains_key(&face_cpk(5)), "{:?}", entries.keys());
    // The boots alone: one part, written as it is, its `.mtl` the boots' one.
    let boots = entries_under(&entries, BOOTS_K0625);
    let names: Vec<&str> = boots.keys().copied().collect();
    assert_eq!(names, ["boots.model", "boots.mtl"]);
    assert_eq!(*boots["boots.model"], card_model());
    assert_eq!(
        sampler_paths(boots["boots.mtl"]),
        [format!("{SLOT_05_HOME}boots.dds")]
    );
    // His gloves, under his exclusive id: each model under its name lowercased with the `.mtl`
    // it uses, `kit_boots.mtl` not among them.
    let gloves = entries_under(&entries, GLOVES);
    let names: Vec<&str> = gloves.keys().copied().collect();
    assert_eq!(
        names,
        [
            "g0625/glove.xml",
            "g0625/x_glovel.model",
            "g0625/x_glovel.mtl",
            "g0625/x_glover.model",
            "g0625/x_glover.mtl",
        ]
    );
    let gloves = entries_under(&entries, GLOVES_G0625);
    assert_eq!(*gloves["x_glovel.model"], card_model());
    assert_eq!(
        String::from_utf8(gloves["glove.xml"].clone()).unwrap(),
        glove_xml(&[
            ("gloveL", "x_glovel.model", "x_glovel.mtl"),
            ("gloveR", "x_glover.model", "x_glover.mtl"),
        ])
    );
    for (name, texture) in [("x_glovel.mtl", "left"), ("x_glover.mtl", "right")] {
        assert_eq!(
            sampler_paths(gloves[name]),
            [format!("{SLOT_05_HOME}{texture}.dds")]
        );
    }
    for texture in ["boots", "left", "right"] {
        let path = format!("common/character1/{SLOT_05_HOME}{texture}.dds");
        assert!(entries.contains_key(&path), "{path}");
    }
}

// TC-MOD-45
#[test]
fn under_ingame_face_a_gloves_link_beside_a_gloves_part_combines_the_player_s_model_first() {
    let sandbox = Sandbox::new("prefox_ingame_gloves_link");
    let export = "co Midcup Keeper";
    write_marked_slot_05(
        &sandbox,
        export,
        &[("glove_l", "left"), ("x_gloveR", "right")],
    );
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/Keeper.gloves"), b"");
    // Keeper's `glove_l` packs under the name slot 05's own does; its `glove_r` under one of
    // its own.
    let keeper = format!("exports/{export}/Gloves/Keeper");
    let keeper_model = card_naming("keeper");
    for hand in ["glove_l", "glove_r"] {
        sandbox.write(&format!("{keeper}/{hand}.model"), &keeper_model);
        sandbox.write(
            &format!("{keeper}/{hand}.mtl"),
            &materials("keeper", "keeper"),
        );
    }
    sandbox.write(&format!("{keeper}/keeper.dds"), &small_dds());

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            CLEAN[0],
            CLEAN[1],
            "Info link_combined [Keep] at Players/05 - A (link=Keeper.gloves)",
        ],
    );

    // Keeper, linked by no other player, takes no id: its models are his gloves'.
    let gloves = entries_under(&entries, GLOVES);
    let names: Vec<&str> = gloves.keys().copied().collect();
    assert_eq!(
        names,
        [
            "g0625/glove.xml",
            "g0625/glove_l.model",
            "g0625/glove_l.mtl",
            "g0625/glove_r.model",
            "g0625/glove_r.mtl",
            "g0625/x_glover.model",
            "g0625/x_glover.mtl",
        ]
    );
    let gloves = entries_under(&entries, GLOVES_G0625);
    // His own `glove_l.model` and `.mtl`, not Keeper's.
    assert_eq!(*gloves["glove_l.model"], card_model());
    assert_eq!(
        sampler_paths(gloves["glove_l.mtl"]),
        [format!("{SLOT_05_HOME}left.dds")]
    );
    assert_eq!(*gloves["glove_r.model"], keeper_model);
    assert_eq!(
        sampler_paths(gloves["glove_r.mtl"]),
        [format!("{SLOT_05_HOME}keeper.dds")]
    );
    // By export path, case-folded: Keeper's `glove_r` first.
    assert_eq!(
        String::from_utf8(gloves["glove.xml"].clone()).unwrap(),
        glove_xml(&[
            ("gloveR", "glove_r.model", "glove_r.mtl"),
            ("gloveL", "glove_l.model", "glove_l.mtl"),
            ("gloveR", "x_glover.model", "x_glover.mtl"),
        ])
    );
    // His boots hold his boots part alone, no glove merged in.
    let boots = entries_under(&entries, BOOTS_K0625);
    let names: Vec<&str> = boots.keys().copied().collect();
    assert_eq!(names, ["boots.model", "boots.mtl"]);
    assert_eq!(*boots["boots.model"], card_model());
    assert_eq!(
        sampler_paths(boots["boots.mtl"]),
        [format!("{SLOT_05_HOME}boots.dds")]
    );
}

/// Team 714's Common output, where the Common textures are and a copied Common `.mtl` names
/// them.
const COMMON_714: &str = "model/character/uniform/common/714/";

// TC-MOD-44
#[test]
fn under_ingame_face_a_common_model_link_is_one_more_part_of_the_player_s_boots() {
    let sandbox = Sandbox::new("prefox_ingame_common_boots");
    let export = "co Midcup Studs";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    let socks_model = card_naming("socks");
    sandbox.write(&format!("{player}/socks.model"), &socks_model);
    sandbox.write(&format!("{player}/socks.mtl"), &materials("socks", "skin"));
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    sandbox.write(&format!("{player}/kit_boots.model.common"), b"");
    let common = format!("exports/{export}/Common");
    let studs_model = card_naming("studs");
    sandbox.write(&format!("{common}/kit_boots.model"), &studs_model);
    sandbox.write(
        &format!("{common}/kit_boots.mtl"),
        &materials("studs", "studs"),
    );
    sandbox.write(&format!("{common}/studs.dds"), &small_dds());

    // The two parts merge into his one `boots.model`; no link is combined, a `.common` link
    // not being a shared folder.
    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            CLEAN[0],
            CLEAN[1],
            "Info model_merged [Keep] at Players/05 - A (model=boots.model)",
        ],
    );

    assert!(!entries.contains_key(&face_cpk(5)), "{:?}", entries.keys());
    let boots = entries_under(&entries, BOOTS_K0625);
    let names: Vec<&str> = boots.keys().copied().collect();
    assert_eq!(names, ["boots.model", "boots.mtl"]);
    assert_eq!(
        mesh_count(boots["boots.model"]),
        mesh_count(&studs_model) + mesh_count(&socks_model),
        "the Common model's meshes plus socks's"
    );
    // The Common part first (`kit_boots.model` before `socks.model`): its texture stays in
    // the team's Common output, his own is in his common folder.
    assert_eq!(
        sampler_paths(boots["boots.mtl"]),
        [
            format!("{COMMON_714}studs.dds"),
            format!("{SLOT_05_HOME}skin.dds")
        ]
    );
    for texture in [
        format!("common/character1/{COMMON_714}studs.dds"),
        format!("common/character1/{SLOT_05_HOME}skin.dds"),
    ] {
        assert!(entries.contains_key(&texture), "{texture}");
    }
}

// TC-XML-17
#[test]
fn under_ingame_face_only_a_copied_common_mtl_names_a_common_texture_it_has_no_link_to() {
    let sandbox = Sandbox::new("prefox_ingame_common_unlinked");
    let export = "co Midcup Studs";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    sandbox.write(&format!("{player}/socks.model"), &card_naming("socks"));
    // His own `.mtl` names `studs.dds`, which only `Common/` holds, through no texture link.
    sandbox.write(&format!("{player}/socks.mtl"), &materials("socks", "studs"));
    sandbox.write(&format!("{player}/kit_boots.model.common"), b"");
    let common = format!("exports/{export}/Common");
    sandbox.write(&format!("{common}/kit_boots.model"), &card_naming("studs"));
    let studs = materials("studs", "studs");
    sandbox.write(&format!("{common}/kit_boots.mtl"), &studs);
    sandbox.write(&format!("{common}/studs.dds"), &small_dds());

    // So nothing supplies his own `studs.dds`, which is a Warning.
    let missing = "Warning mtl_texture_not_found [Keep] at Players/05 - A (file=socks.mtl, texture=./studs.dds, materials=socks)";
    let merged = "Info model_merged [Keep] at Players/05 - A (model=boots.model)";
    let entries = compile_pes17(&sandbox, export, &[missing, CLEAN[0], CLEAN[1], merged]);

    // The copied Common `.mtl`'s path names the Common output; his own stays as written.
    let boots = entries_under(&entries, BOOTS_K0625);
    assert_eq!(
        sampler_paths(boots["boots.mtl"]),
        [format!("{COMMON_714}studs.dds"), "./studs.dds".to_owned()]
    );
}

// TC-XML-17
#[test]
fn under_ingame_face_a_copied_common_mtl_names_no_subfolder_texture() {
    let sandbox = Sandbox::new("prefox_ingame_common_subfolder_texture");
    let export = "co Midcup Studs";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    sandbox.write(&format!("{player}/kit_boots.model.common"), b"");
    let common = format!("exports/{export}/Common");
    sandbox.write(&format!("{common}/kit_boots.model"), &card_naming("studs"));
    sandbox.write(
        &format!("{common}/kit_boots.mtl"),
        &materials("studs", "cloth"),
    );
    // Packed under `sub/` in the Common output, so not the `cloth.dds` the `.mtl` names.
    sandbox.write(&format!("{common}/sub/cloth.dds"), &small_dds());

    let missing = "Warning mtl_texture_not_found [Keep] at Common/kit_boots.mtl (file=kit_boots.mtl, texture=./cloth.dds, materials=studs)";
    let entries = compile_pes17(&sandbox, export, &[missing, CLEAN[0], CLEAN[1]]);

    // The copied Common `.mtl`'s path is left as written: the Common output holds no
    // `cloth.dds` of its own.
    let boots = entries_under(&entries, BOOTS_K0625);
    assert_eq!(sampler_paths(boots["boots.mtl"]), ["./cloth.dds"]);
}

// TC-CMN-24
#[test]
fn under_ingame_face_a_copied_common_mtl_s_texture_resolves_in_its_own_directory() {
    let sandbox = Sandbox::new("prefox_ingame_common_mtl_nested");
    let export = "co Midcup Studs";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    // His own `studs.dds`, of other bytes than the Common one, shadows nothing: the copied
    // Common `.mtl` resolves where it is, never in his folder.
    sandbox.write(&format!("{player}/studs.dds"), &small_dds());
    sandbox.write(&format!("{player}/jessie/kit_boots.model.common"), b"");
    let common = format!("exports/{export}/Common");
    sandbox.write(
        &format!("{common}/jessie/kit_boots.model"),
        &card_naming("studs"),
    );
    sandbox.write(
        &format!("{common}/jessie/kit_boots.mtl"),
        &materials("studs", "studs"),
    );
    sandbox.write(
        &format!("{common}/jessie/studs.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let entries = compile_pes17(&sandbox, export, &CLEAN);

    // `./studs.dds` resolves in `Common/jessie/`, which holds it: the Common output's own
    // path below `Common/`, not his folder's.
    let boots = entries_under(&entries, BOOTS_K0625);
    assert_eq!(
        sampler_paths(boots["boots.mtl"]),
        [format!("{COMMON_714}jessie/studs.dds")]
    );
    let texture = format!("common/character1/{COMMON_714}jessie/studs.dds");
    assert!(entries.contains_key(&texture), "{texture}");
}

// TC-CMN-24
#[test]
fn under_ingame_face_a_copied_root_common_mtl_s_texture_resolves_in_common() {
    let sandbox = Sandbox::new("prefox_ingame_common_mtl_root");
    let export = "co Midcup Studs";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    // His own `studs.dds`, of other bytes than the Common one, shadows nothing.
    sandbox.write(&format!("{player}/studs.dds"), &small_dds());
    sandbox.write(&format!("{player}/kit_boots.model.common"), b"");
    let common = format!("exports/{export}/Common");
    sandbox.write(&format!("{common}/kit_boots.model"), &card_naming("studs"));
    sandbox.write(
        &format!("{common}/kit_boots.mtl"),
        &materials("studs", "studs"),
    );
    sandbox.write(
        &format!("{common}/studs.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let entries = compile_pes17(&sandbox, export, &CLEAN);

    let boots = entries_under(&entries, BOOTS_K0625);
    assert_eq!(
        sampler_paths(boots["boots.mtl"]),
        [format!("{COMMON_714}studs.dds")]
    );
    let texture = format!("common/character1/{COMMON_714}studs.dds");
    assert!(entries.contains_key(&texture), "{texture}");
}

// TC-MOD-45
#[test]
fn under_ingame_face_common_links_are_copied_into_the_player_s_own_gloves_folder() {
    let sandbox = Sandbox::new("prefox_ingame_common_gloves");
    let export = "co Midcup Hands";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    sandbox.write(&format!("{player}/glove_l.model.common"), b"");
    let right_model = card_naming("right");
    sandbox.write(&format!("{player}/glove_r.model"), &right_model);
    sandbox.write(&format!("{player}/glove_r.mtl.common"), b"");
    let common = format!("exports/{export}/Common");
    let left_model = card_naming("left");
    sandbox.write(&format!("{common}/glove_l.model"), &left_model);
    sandbox.write(&format!("{common}/glove_l.mtl"), &materials("left", "left"));
    sandbox.write(
        &format!("{common}/glove_r.mtl"),
        &materials("right", "right"),
    );
    for texture in ["left", "right"] {
        sandbox.write(&format!("{common}/{texture}.dds"), &small_dds());
    }

    let entries = compile_pes17(&sandbox, export, &CLEAN);

    assert!(!entries.contains_key(&face_cpk(5)), "{:?}", entries.keys());
    let gloves = entries_under(&entries, GLOVES);
    let names: Vec<&str> = gloves.keys().copied().collect();
    assert_eq!(
        names,
        [
            "g0625/glove.xml",
            "g0625/glove_l.model",
            "g0625/glove_l.mtl",
            "g0625/glove_r.model",
            "g0625/glove_r.mtl",
        ]
    );
    let gloves = entries_under(&entries, GLOVES_G0625);
    assert_eq!(*gloves["glove_l.model"], left_model);
    assert_eq!(*gloves["glove_r.model"], right_model);
    // By export path, case-folded: the Common model first.
    assert_eq!(
        String::from_utf8(gloves["glove.xml"].clone()).unwrap(),
        glove_xml(&[
            ("gloveL", "glove_l.model", "glove_l.mtl"),
            ("gloveR", "glove_r.model", "glove_r.mtl"),
        ])
    );
    for (name, texture) in [("glove_l.mtl", "left"), ("glove_r.mtl", "right")] {
        assert_eq!(
            sampler_paths(gloves[name]),
            [format!("{COMMON_714}{texture}.dds")]
        );
    }
    // No boots part, so no boots of his own.
    assert!(
        !entries.keys().any(|path| path.starts_with(BOOTS_K0625)),
        "{:?}",
        entries.keys()
    );
}

// TC-MOD-38
#[test]
fn a_shared_gloves_folder_s_mtl_no_model_uses_is_not_written() {
    let sandbox = Sandbox::new("prefox_shared_gloves_unused_mtl");
    let export = "co Midcup Spare";
    write_slot_05_face(&sandbox, export);
    sandbox.write(
        &format!("exports/{export}/Players/05 - A/Keeper.gloves"),
        b"",
    );
    let keeper = format!("exports/{export}/Gloves/Keeper");
    sandbox.write(&format!("{keeper}/glove_l.model"), &card_model());
    sandbox.write(&format!("{keeper}/glove_l.mtl"), &materials("card", "left"));
    // A second material set, which the search does not pick for `glove_l.model`.
    sandbox.write(
        &format!("{keeper}/materials.mtl"),
        &materials("card", "left"),
    );
    sandbox.write(&format!("{keeper}/left.dds"), &small_dds());

    let entries = compile_pes17(&sandbox, export, &[CLEAN[0], CLEAN[1]]);

    let gloves = entries_under(&entries, GLOVES);
    let names: Vec<&str> = gloves.keys().copied().collect();
    assert_eq!(
        names,
        [
            "g0625/glove.xml",
            "g0625/glove_l.model",
            "g0625/glove_l.mtl",
            "g0625/left.dds",
        ]
    );
}

// TC-MOD-85
#[test]
fn under_ingame_face_fmdl_parts_are_converted_into_the_player_s_own_boots_and_gloves() {
    let sandbox = Sandbox::new("prefox_ingame_fmdl");
    let export = "co Midcup Converted";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    // The tracer's hair as his boots, with its skeleton, the conversion's bind pose (the
    // tracer's boots have none), its right glove and the texture both name.
    sandbox.write(
        &format!("{player}/boots.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(
        &format!("{player}/boots.skl"),
        &tracer_player_file("fcl_hair.skl"),
    );
    sandbox.write(
        &format!("{player}/glove_r.fmdl"),
        &tracer_player_file("glove_r.fmdl"),
    );
    sandbox.write(
        &format!("{player}/shirt.dds"),
        &tracer_player_file("shirt.dds"),
    );
    complete_pes17(&sandbox, export);

    let run = sandbox.run(&pes17(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    // The `skl_parent` drops come from the skeleton, which the boots' conversion read.
    let folder = "at Players/05 - A";
    assert_eq!(
        findings_of(&lines, export),
        [
            format!(
                "Info fmdl_weights_not_normalized [Keep] {folder} (file=boots.fmdl, count=1662)"
            ),
            CLEAN[0].to_owned(),
            CLEAN[1].to_owned(),
            format!(
                "Info native_field_dropped [Keep] {folder} (model=boots.fmdl, field=bone_matrices)"
            ),
            format!(
                "Info native_field_dropped [Keep] {folder} (model=boots.fmdl, bone=8, field=skl_parent)"
            ),
            format!(
                "Info native_field_dropped [Keep] {folder} (model=boots.fmdl, bone=25, field=skl_parent)"
            ),
            format!(
                "Warning mesh_flags_dropped [Keep] {folder} (model=boots.fmdl, material=1, field=no_shadow_cast)"
            ),
            format!(
                "Info native_field_dropped [Keep] {folder} (model=glove_r.fmdl, field=bone_matrices)"
            ),
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(!entries.contains_key(&face_cpk(5)), "{:?}", entries.keys());
    let boots = entries_under(&entries, BOOTS_K0625);
    let names: Vec<&str> = boots.keys().copied().collect();
    assert_eq!(names, ["boots.model", "boots.mtl"]);
    // The hair's anti-blur mesh folded back.
    assert_eq!(mesh_count(boots["boots.model"]), 2);
    let gloves = entries_under(&entries, GLOVES_G0625);
    let names: Vec<&str> = gloves.keys().copied().collect();
    assert_eq!(names, ["glove.xml", "glove_r.model", "glove_r.mtl"]);
    assert_eq!(mesh_count(gloves["glove_r.model"]), 1);
    assert_eq!(
        String::from_utf8(gloves["glove.xml"].clone()).unwrap(),
        glove_xml(&[("gloveR", "glove_r.model", "glove_r.mtl")])
    );
    // Each converted set's texture paths pointed as his face's would be: the hair's texture at
    // his texture home, which the CPK holds, and the glove's reserved kit stem at the team's
    // Common directory.
    assert!(
        sampler_paths(boots["boots.mtl"]).contains(&format!("{SLOT_05_HOME}shirt.dds")),
        "{:?}",
        sampler_paths(boots["boots.mtl"])
    );
    assert_eq!(
        sampler_paths(gloves["glove_r.mtl"]),
        ["model/character/uniform/common/714/dummy_kit.dds"]
    );
    let texture = format!("common/character1/{SLOT_05_HOME}shirt.dds");
    assert!(entries.contains_key(&texture), "{texture}");
}

// TC-MOD-85
#[test]
fn under_ingame_face_a_common_fmdl_link_is_converted_into_the_player_s_own_boots() {
    let sandbox = Sandbox::new("prefox_ingame_common_fmdl");
    let export = "co Midcup Studs";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    // His own boots part, the tracer's right glove (`clean_model`), converted too: the card
    // head, bound to `sk_head` as the hair is but with another transform, would not merge with
    // the hair (`skl_merge_conflict`), nor would a second copy of the hair, whose `shirt`
    // material would name another texture place (`merge_material_conflict`).
    sandbox.write(&format!("{player}/boots.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/legs.fmdl.common"), b"");
    // The tracer's hair, its skeleton (the conversion's bind pose) and the texture it names.
    let common = format!("exports/{export}/Common");
    sandbox.write(
        &format!("{common}/legs.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(
        &format!("{common}/legs.skl"),
        &tracer_player_file("fcl_hair.skl"),
    );
    sandbox.write(
        &format!("{common}/shirt.dds"),
        &tracer_player_file("shirt.dds"),
    );
    complete_pes17(&sandbox, export);

    let run = sandbox.run(&pes17(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    // His boots convert the Common FMDL from its export path, the `skl_parent` drops coming
    // from the Common skeleton, which the conversion read.
    let folder = "at Players/05 - A";
    for finding in [
        format!(
            "Info native_field_dropped [Keep] {folder} (model=Common/legs.fmdl, bone=8, field=skl_parent)"
        ),
        format!("Info model_merged [Keep] {folder} (model=boots.model)"),
    ] {
        assert!(
            findings_of(&lines, export).contains(&finding.as_str()),
            "{finding}: {lines:#?}"
        );
    }
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(!entries.contains_key(&face_cpk(5)), "{:?}", entries.keys());
    let boots = entries_under(&entries, BOOTS_K0625);
    let names: Vec<&str> = boots.keys().copied().collect();
    assert_eq!(names, ["boots.model", "boots.mtl"]);
    // His glove's mesh, then the hair's two, its anti-blur mesh folded back.
    assert_eq!(mesh_count(boots["boots.model"]), 1 + 2);
    // The hair's texture pointed at the team's Common directory, where the Common textures
    // task emits it, as a Common part's places say (his folder holds no `shirt`).
    let paths = sampler_paths(boots["boots.mtl"]);
    let shirt = format!("{COMMON_714}shirt.dds");
    assert!(paths.contains(&shirt), "{paths:?}");
    assert!(
        entries.contains_key(&format!("common/character1/{shirt}")),
        "{:?}",
        entries.keys()
    );
}

// TC-MOD-62
#[test]
fn under_ingame_face_a_left_out_boots_variant_that_does_not_parse_is_not_read() {
    let sandbox = Sandbox::new("prefox_ingame_variant_broken");
    let export = "co Midcup Variant Boots";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    sandbox.write(&format!("{player}/boots_kit1.model"), &card_model());
    sandbox.write(&format!("{player}/boots.mtl"), &card_materials());
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    // Four bytes no `.model` reader accepts, in the variant his boots leave out.
    sandbox.write(&format!("{player}/boots_kit2.model"), b"junk");

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            CLEAN[0],
            CLEAN[1],
            "Warning kit_variant_model_left_out [Keep] at Players/05 - A (model=boots_kitN.model, used=boots_kit1.model)",
        ],
    );

    let boots = entries_under(&entries, BOOTS_K0625);
    let names: Vec<&str> = boots.keys().copied().collect();
    assert_eq!(names, ["boots.model", "boots.mtl"]);
    assert_eq!(*boots["boots.model"], card_model(), "kit 1's model alone");
}

/// Writes slot 05 of the export `export` holding `ingame_face`, `skin.dds` and a link to the
/// per-kit Common model `boots_kit1.model`, which `Common/` holds with no `.mtl`; with his own
/// `boots.model` and `boots.mtl` when `own_boots`.
fn write_per_kit_common_link(sandbox: &Sandbox, export: &str, own_boots: bool) {
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/ingame_face.toml"), b"");
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    if own_boots {
        sandbox.write(&format!("{player}/boots.model"), &card_model());
        sandbox.write(&format!("{player}/boots.mtl"), &card_materials());
    }
    sandbox.write(&format!("{player}/boots_kit1.model.common"), b"");
    sandbox.write(
        &format!("exports/{export}/Common/boots_kit1.model"),
        &card_model(),
    );
}

/// The finding on slot 05's link to a per-kit Common model, which no part of his takes.
const PER_KIT_LINK_NOT_USED: &str =
    "Warning file_not_used [Keep] at Players/05 - A (file=boots_kit1.model.common)";

// TC-MOD-77
#[test]
fn under_ingame_face_a_link_to_a_per_kit_common_model_is_not_used() {
    let sandbox = Sandbox::new("prefox_ingame_common_variant");
    let export = "co Midcup Studs";
    write_per_kit_common_link(&sandbox, export, true);

    let entries = compile_pes17(
        &sandbox,
        export,
        &[CLEAN[0], PER_KIT_LINK_NOT_USED, CLEAN[1]],
    );

    let boots = entries_under(&entries, BOOTS_K0625);
    let names: Vec<&str> = boots.keys().copied().collect();
    assert_eq!(names, ["boots.model", "boots.mtl"]);
    assert_eq!(*boots["boots.model"], card_model(), "his own model alone");
}

// TC-MOD-77
#[test]
fn under_ingame_face_a_link_to_a_per_kit_common_model_no_mtl_is_found_for_is_not_used() {
    let sandbox = Sandbox::new("prefox_ingame_common_variant_alone");
    let export = "co Midcup Studs";
    // No `.mtl` anywhere: as a part, the link would have none to be written with.
    write_per_kit_common_link(&sandbox, export, false);
    complete_pes17(&sandbox, export);

    let run = sandbox.run(&pes17(&sandbox), &["compile", "--no-deploy"]);

    // With no part, he plans no package, so no textures task: his `skin.dds` is not used
    // either.
    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            CLEAN[0],
            PER_KIT_LINK_NOT_USED,
            "Warning file_not_used [Keep] at Players/05 - A (file=skin.dds)",
            CLEAN[1]
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
}
