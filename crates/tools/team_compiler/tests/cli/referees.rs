//! `compile` over a refs export on Fox: each referee folder prepared once and emitted under
//! every slot `players.txt` maps it to, as `face/real/referee0NN`, `k99NN` and `g99NN`, its
//! textures once in team 999's common subfolder of its name, and a link of his made a part of
//! his slots' own packages. In a normal compile all of it goes into the refs CPK,
//! `refs_cpk_name`, beside the team side.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use dds_convert::{SourceFormat, Target, TextureRole, decode};
use kit_config::KitConfig;
use pes_version::{Engine, PesVersion};
use uniparam::UniformParameter;

use crate::common::{Run, Sandbox};
use crate::common_links::texture_directories;
use crate::compile::{
    cpk_entries, kit_texture, pes_settings, pes21_settings, tracer_kit, tracer_player_file,
};
use crate::compile_exports::{TEAM_COLOR, UNI_COLOR, UNIFORM_PARAMETER, bundled_uniform_parameter};
use crate::deploy::{install_pes, templates_folder};
use crate::models::package_names;
use crate::prefox_faces::{
    card_materials, card_model, materials_naming, nested_entries, ordered_entries, sampler_paths,
    small_dds,
};
use crate::prefox_kits::write_kit;
use crate::sideload::slashed;
use crate::textures::{texture_fixture, tracer_model_renaming};
use crate::{clean_model, findings_of, snapshot};

/// The refs export's folder in the sandbox.
pub(crate) const REFS: &str = "exports/refs Cup";

/// Where `Ref A`'s textures go: team 999's common subfolder of his folder's name.
const REF_A_TEXTURES: &str = "Asset/model/character/common/999/Ref A/sourceimages/#windx11";

/// The CPK path of referee slot `slot`'s package `stem` (`face`, `boots`, `glove`) of `kind`
/// (`face/real/referee0`, `boots/k99`, `glove/g99`).
fn referee_package(kind: &str, slot: &str, stem: &str) -> String {
    format!("Asset/model/character/{kind}{slot}/#Win/{stem}.fpk")
}

/// Writes `Ref A` mapped to each of `slots`: a face model and a boots model, each naming
/// `skin.dds`, and `skin.dds` (TC-REF-01's referee).
pub(crate) fn write_ref_a(sandbox: &Sandbox, slots: &[&str]) {
    let roster: String = slots.iter().map(|slot| format!("{slot} Ref A\n")).collect();
    sandbox.write(&format!("{REFS}/players.txt"), roster.as_bytes());
    let folder = format!("{REFS}/Players/Ref A");
    sandbox.write(
        &format!("{folder}/face_high.fmdl"),
        &tracer_model_renaming("fcl_hair.fmdl", &[("shirt.dds", "skin.dds")]),
    );
    sandbox.write(
        &format!("{folder}/boots.fmdl"),
        &tracer_model_renaming("boots.fmdl", &[("shirt.dds", "skin.dds")]),
    );
    sandbox.write(
        &format!("{folder}/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );
}

/// The refs CPK's file name at the default `refs_cpk_name`.
pub(crate) const REFS_CPK: &str = "4cc_18_referees.cpk";

/// `compile --no-deploy` in `sandbox` for PES 21: the run and its refs CPK's entries.
fn compile(sandbox: &Sandbox) -> (Run, BTreeMap<String, Vec<u8>>) {
    compile_for(sandbox, PesVersion::Pes21)
}

/// `compile` for `version`.
fn compile_for(sandbox: &Sandbox, version: PesVersion) -> (Run, BTreeMap<String, Vec<u8>>) {
    let settings = pes_settings(sandbox, version.number().try_into().unwrap());
    let run = sandbox.run(&settings, &["compile", "--no-deploy"]);
    let entries = cpk_entries(&sandbox.root.join("output").join(REFS_CPK));
    (run, entries)
}

/// The line `deploy_skipped_by_flag` reports for `output/<name>`.
fn skipped(sandbox: &Sandbox, name: &str) -> String {
    format!(
        "Info deploy_skipped_by_flag [Keep] (path={})",
        sandbox.display(&format!("output/{name}"))
    )
}

/// The game path of the referee template tree's `RefereeAppearance.bin`.
const REFEREE_APPEARANCE: &str =
    "common/character0/model/character/appearance/RefereeAppearance.bin";

/// The folder of `engine`'s referee template tree, below `resources/templates/` and below a
/// data directory's `templates/`.
fn tree_folder(engine: Engine) -> &'static str {
    match engine {
        Engine::Fox => "referees_fox",
        Engine::PreFox => "referees_prefox",
    }
}

/// The referee template tree of `engine` as the repository holds it, `resources/templates/
/// referees_fox/` (31 files) or `referees_prefox/` (51): every file by its path below that
/// folder, spelled with `/`, with its bytes.
fn referee_tree(engine: Engine) -> BTreeMap<String, Vec<u8>> {
    let folder = tree_folder(engine);
    let count = match engine {
        Engine::Fox => 31,
        Engine::PreFox => 51,
    };
    let tree: BTreeMap<String, Vec<u8>> = snapshot(&templates_folder().join(folder))
        .into_iter()
        .map(|(path, bytes)| (slashed(&path), bytes))
        .collect();
    assert_eq!(tree.len(), count, "the {folder} tree's files");
    tree
}

/// Asserts that `entries` hold every file of `tree`, a referee template tree, with its bytes.
fn assert_tree_in(entries: &BTreeMap<String, Vec<u8>>, tree: &BTreeMap<String, Vec<u8>>) {
    for (path, bytes) in tree {
        assert!(entries.get(path) == Some(bytes), "{path}");
    }
}

/// The game path of the referees' marker model: stock collar 77's `nocloth` model.
const MARKER_COLLAR: &str = "Asset/model/character/uniform/nocloth/#Win/collar_077.fmdl";

/// The game path of the converted marker texture, in the referees' Common output.
const MARKER_TEXTURE: &str =
    "Asset/model/character/common/999/sourceimages/#windx11/ref_marker.ftex";

/// The game path of stock collar 77's pre-Fox referee model, never written: the pre-Fox marker
/// is no collar.
const PRE_FOX_MARKER_MODEL: &str =
    "common/character0/model/character/uniform/nocloth/referee_collar_077.model";

/// The game path of a `.mtl` beside stock collar 77's pre-Fox referee model, never written.
const PRE_FOX_MARKER_MTL: &str =
    "common/character0/model/character/uniform/nocloth/referee_collar_077.mtl";

/// The game path of stock collar 77's pre-Fox model, never written.
const PRE_FOX_EMPTY_COLLAR: &str =
    "common/character0/model/character/uniform/nocloth/collar_077.model";

/// The game path the marker texture would have in the pre-Fox referees' Common output, never
/// written: the marker goes out at `REFEREE_PROP_TEXTURE` alone.
const PRE_FOX_COMMON_MARKER_TEXTURE: &str =
    "common/character1/model/character/uniform/common/999/ref_marker.dds";

/// The game path of the pre-Fox template tree's prop texture, which the converted marker
/// replaces.
const REFEREE_PROP_TEXTURE: &str = "common/character1/model/character/parts/referee/incom_bsm.dds";

/// The folder of the referee kit configs, in the template tree and in the refs CPK.
pub(crate) const REFEREE_CONFIGS: &str = "common/character0/model/character/uniform/team/referee/";

/// The referee kit configs of `engine`'s template tree as the repository holds it, by game
/// path.
fn template_configs(engine: Engine) -> BTreeMap<String, Vec<u8>> {
    let configs: BTreeMap<String, Vec<u8>> = referee_tree(engine)
        .into_iter()
        .filter(|(path, _)| path.starts_with(REFEREE_CONFIGS))
        .collect();
    assert_eq!(configs.len(), 20, "the tree's kit configs");
    configs
}

/// Writes `Ref A` in slot 01 (`write_ref_a`) and, when given, `marker` as the refs export's
/// `ref_marker.dds`.
pub(crate) fn write_refs_with_marker(sandbox: &Sandbox, marker: Option<&[u8]>) {
    write_ref_a(sandbox, &["01"]);
    if let Some(marker) = marker {
        sandbox.write(&format!("{REFS}/ref_marker.dds"), marker);
    }
}

/// The names of the `.cpk` files directly in `folder`, sorted.
fn cpk_files(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".cpk"))
        .collect();
    names.sort();
    names
}

/// The team CPK's file name at the default `cpk_name`.
const TEAM_CPK: &str = "4cc_99_test.cpk";

/// The entries of the team CPK a `compile --no-deploy` in `sandbox` left in `output/`.
fn team_entries(sandbox: &Sandbox) -> BTreeMap<String, Vec<u8>> {
    cpk_entries(&sandbox.root.join("output").join(TEAM_CPK))
}

/// The `UniformParameter.bin` of the team CPK a `compile --no-deploy` in `sandbox` wrote.
fn team_uniform_parameter(sandbox: &Sandbox) -> UniformParameter {
    UniformParameter::read(&team_entries(sandbox)[UNIFORM_PARAMETER]).unwrap()
}

/// The name of the `UniformParameter.bin` entry of the referee kit config at game path
/// `path`: its file name, `.bin` included.
fn entry_name(path: &str) -> &str {
    path.strip_prefix(REFEREE_CONFIGS).unwrap()
}

// TC-REF-01
#[test]
fn a_referee_folder_is_emitted_under_each_of_his_slots_with_his_textures_once() {
    let sandbox = Sandbox::new("ref_slots");
    write_ref_a(&sandbox, &["01", "20", "35"]);

    let (run, entries) = compile(&sandbox);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "refs Cup"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/Ref A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/Ref A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=referees)",
        ],
        "no team_colors_missing: a referee has no team record"
    );
    assert_eq!(lines.last(), Some(&skipped(&sandbox, REFS_CPK)));
    assert_eq!(run.exit_code(), 0);
    // The refs export alone commits: the team CPK holds the bins alone, for the referee kit
    // configs' entries in UniformParameter.bin, so no 999 path and no referee path.
    assert_eq!(
        cpk_files(&sandbox.root.join("output")),
        [REFS_CPK, TEAM_CPK]
    );
    let team: Vec<String> = team_entries(&sandbox).into_keys().collect();
    let mut bins = [TEAM_COLOR, UNI_COLOR, UNIFORM_PARAMETER];
    bins.sort_unstable();
    assert_eq!(team, bins);
    assert!(!entries.contains_key(TEAM_COLOR));
    for slot in ["01", "20", "35"] {
        for (kind, stem) in [("face/real/referee0", "face"), ("boots/k99", "boots")] {
            let package = referee_package(kind, slot, stem);
            assert!(entries.contains_key(&package), "{package}");
            assert!(entries.contains_key(&format!("{package}d")), "{package}d");
        }
    }
    let skins: Vec<&String> = entries
        .keys()
        .filter(|path| path.ends_with("/skin.ftex"))
        .collect();
    assert_eq!(skins, [&format!("{REF_A_TEXTURES}/skin.ftex")]);
    let face =
        fpk::FpkFile::read(&entries[&referee_package("face/real/referee0", "01", "face")]).unwrap();
    let directories = texture_directories(face.get("face_high.fmdl").unwrap(), "skin.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| directory
                == "/Assets/pes16/model/character/common/999/Ref A/sourceimages/"),
        "{directories:?}"
    );
    // No note is collected without one.
    assert!(!sandbox.root.join("output/teamnotes.txt").exists());
    // The referee kits and appearance the game needs come with them.
    assert_tree_in(&entries, &referee_tree(Engine::Fox));
}

// TC-REF-14
#[test]
fn a_refs_export_s_root_colors_txt_is_not_read_so_not_checked() {
    let sandbox = Sandbox::new("ref_root_colors");
    write_ref_a(&sandbox, &["01"]);
    sandbox.write(&format!("{REFS}/colors.txt"), b"not a color\n");

    let run = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "refs Cup"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/Ref A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/Ref A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=referees)",
        ],
        "no color_entry_invalid: {lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
}

// TC-REF-11
#[test]
fn a_referee_folder_holding_textures_alone_reports_them_not_used_and_packs_none() {
    let sandbox = Sandbox::new("ref_textures_alone");
    sandbox.write(&format!("{REFS}/players.txt"), b"01 Ref A\n");
    sandbox.write(
        &format!("{REFS}/Players/Ref A/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );

    // No model, so no package: nothing would name the texture.
    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    let lines = check.messages();
    assert_eq!(
        findings_of(&lines, "refs Cup"),
        [
            "Info export_identified [Keep] (team=referees)",
            "Warning file_not_used [Keep] at Players/Ref A (file=skin.dds)",
        ],
        "{lines:#?}"
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        findings_of(&lines, "refs Cup")
            .contains(&"Warning file_not_used [Keep] at Players/Ref A (file=skin.dds)"),
        "{lines:#?}"
    );
    // Nothing of the refs export compiles, so no refs CPK is written, as when validation
    // drops its only folder: no texture of his went anywhere.
    assert!(!sandbox.root.join("output").join(REFS_CPK).exists());
    for cpk in cpk_files(&sandbox.root.join("output")) {
        let entries = cpk_entries(&sandbox.root.join("output").join(&cpk));
        assert!(
            entries.keys().all(|path| !path.contains("Ref A")),
            "{cpk}: {:?}",
            entries.keys()
        );
    }
}

// TC-REF-11
#[test]
fn a_referee_folder_holding_a_texture_link_alone_reports_it_not_used() {
    let sandbox = Sandbox::new("ref_texture_link_alone");
    sandbox.write(&format!("{REFS}/players.txt"), b"01 Ref A\n");
    sandbox.write(&format!("{REFS}/Players/Ref A/skin.dds.common"), b"");
    sandbox.write(
        &format!("{REFS}/Common/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );

    // No model, so no package: nothing would name the texture the link stands for.
    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    let lines = check.messages();
    assert_eq!(
        findings_of(&lines, "refs Cup"),
        [
            "Info export_identified [Keep] (team=referees)",
            "Warning file_not_used [Keep] at Players/Ref A (file=skin.dds.common)",
        ],
        "{lines:#?}"
    );
    assert_eq!(check.exit_code(), 0, "{lines:#?}");
}

/// The folder of referee slot `slot`'s pre-Fox face CPK: its CPK is `<folder>.cpk`, and every
/// entry of it sits in `<folder>/`.
fn pre_fox_referee_face(slot: &str) -> String {
    format!("common/character0/model/character/face/real/referee0{slot}")
}

// TC-REF-09
#[test]
fn on_pes_17_a_referee_folder_is_a_face_cpk_per_slot_his_boots_in_its_face_xml() {
    let sandbox = Sandbox::new("ref_slots_pes17");
    write_ref_a(&sandbox, &["01", "20", "35"]);

    let (run, entries) = compile_for(&sandbox, PesVersion::Pes17);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    for slot in ["01", "20", "35"] {
        let folder = pre_fox_referee_face(slot);
        let face = nested_entries(&entries[&format!("{folder}.cpk")]);
        let names: Vec<&str> = face
            .keys()
            .map(|path| path.strip_prefix(&format!("{folder}/")).unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "boots.mtl",
                "face.xml",
                "face_high.mtl",
                "oral_boots_win32.model",
                "oral_face_high_win32.model",
            ],
            "{slot}"
        );
        // His boots ride in the face, typed `parts` as a team player's own boots are.
        let entry = |xml_type: &str, path: &str, material: &str| {
            (xml_type.to_owned(), path.to_owned(), material.to_owned())
        };
        assert_eq!(
            ordered_entries(&face[&format!("{folder}/face.xml")]),
            [
                entry("parts", "./oral_boots_*.model", "./boots.mtl"),
                entry("face_neck", "./oral_face_high_*.model", "./face_high.mtl"),
            ],
            "{slot}"
        );
    }
    // No boots folder of his: the only boots files are the template tree's (`k0062`, the
    // referees' stock boots).
    let boots: Vec<&str> = entries
        .keys()
        .filter_map(|path| path.strip_prefix("common/character0/model/character/boots/"))
        .collect();
    assert_eq!(boots, ["k0062/boots.model", "k0062/boots.mtl"]);
    let skins: Vec<&String> = entries
        .keys()
        .filter(|path| path.ends_with("/skin.dds"))
        .collect();
    assert_eq!(
        skins,
        ["common/character1/model/character/uniform/common/999/Ref A/skin.dds"]
    );
    assert_tree_in(&entries, &referee_tree(Engine::PreFox));
}

/// The game path of the referee template tree's refkit body files, in the referees' pre-Fox
/// Common output.
const REFKIT: &str = "common/character1/model/character/uniform/common/999/refkit/";

// TC-REF-15
#[test]
fn on_pes_17_a_refs_export_s_refkit_texture_replaces_the_template_s_beside_its_other_files() {
    let sandbox = Sandbox::new("ref_refkit_texture");
    write_ref_a(&sandbox, &["01"]);
    let texture = small_dds();
    let tree = referee_tree(Engine::PreFox);
    let texture_path = format!("{REFKIT}texture.dds");
    assert_ne!(
        tree[&texture_path], texture,
        "a texture other than the template's"
    );
    sandbox.write(&format!("{REFS}/Common/refkit/texture.dds"), &texture);

    let (run, entries) = compile_for(&sandbox, PesVersion::Pes17);

    let lines = run.messages();
    // His FMDLs' conversion findings alone.
    assert_eq!(
        findings_of(&lines, "refs Cup"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/Ref A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/Ref A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=referees)",
            "Info native_field_dropped [Keep] at Players/Ref A (model=boots.fmdl, field=bone_matrices)",
            "Warning mesh_flags_dropped [Keep] at Players/Ref A (model=boots.fmdl, material=1, field=no_shadow_cast)",
            "Info native_field_dropped [Keep] at Players/Ref A (model=face_high.fmdl, field=bone_matrices)",
            "Warning mesh_flags_dropped [Keep] at Players/Ref A (model=face_high.fmdl, material=1, field=no_shadow_cast)",
        ],
        "no duplicate_path, no common_file_disallowed: {lines:#?}"
    );
    assert!(
        lines.iter().all(|line| !line.contains("duplicate_path")),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let decoded = decode(&texture, SourceFormat::Dds).unwrap();
    let expected = dds_convert::convert(
        &decoded,
        Target {
            version: PesVersion::Pes17,
            role: TextureRole::Color,
            keep_uncompressed: true,
        },
    )
    .unwrap();
    assert_ne!(tree[&texture_path], expected);
    assert!(
        entries[&texture_path] == expected,
        "the export's texture, converted for PES 17"
    );
    let others: Vec<&String> = tree
        .keys()
        .filter(|path| path.starts_with(REFKIT) && **path != texture_path)
        .collect();
    assert_eq!(others.len(), 15, "{others:#?}");
    for path in others {
        assert!(entries.get(path) == tree.get(path), "{path}");
    }
}

// TC-CMN-18
#[test]
fn on_pes_17_a_referee_s_face_xml_names_the_template_s_refkit_files_the_export_lacks() {
    let sandbox = Sandbox::new("ref_xml_refkit");
    sandbox.write(&format!("{REFS}/players.txt"), b"01 Ref A\n");
    let folder = format!("{REFS}/Players/Ref A");
    // A `face.xml` names a `.model` alone, never an FMDL: the card head stands in for his face.
    sandbox.write(
        &format!("{folder}/oral_face_high_win32.model"),
        &card_model(),
    );
    sandbox.write(&format!("{folder}/face_high.mtl"), &card_materials());
    sandbox.write(&format!("{folder}/skin.dds"), &small_dds());
    let arm = "model/character/uniform/common/team_folder/refkit/oral_arm_*.model";
    let refkit_mtl = "team_common_folder/refkit/refkit.mtl";
    let xml = format!(
        "<config>\n   \
         <model level=\"0\" type=\"face_neck\" path=\"./oral_face_high_*.model\" material=\"./face_high.mtl\"/>\n   \
         <model level=\"0\" type=\"parts\" path=\"{arm}\" material=\"{refkit_mtl}\"/>\n\
         </config>\n"
    );
    sandbox.write(&format!("{folder}/face.xml"), xml.as_bytes());

    let (run, entries) = compile_for(&sandbox, PesVersion::Pes17);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "refs Cup"),
        ["Info export_identified [Keep] (team=referees)"],
        "no xml_model_not_found: {lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let face_folder = pre_fox_referee_face("01");
    let face = nested_entries(&entries[&format!("{face_folder}.cpk")]);
    let entry = |xml_type: &str, path: &str, material: &str| {
        (xml_type.to_owned(), path.to_owned(), material.to_owned())
    };
    assert_eq!(
        ordered_entries(&face[&format!("{face_folder}/face.xml")]),
        [
            entry("face_neck", "./oral_face_high_*.model", "./face_high.mtl"),
            entry(
                "parts",
                "model/character/uniform/common/999/refkit/oral_arm_*.model",
                "model/character/uniform/common/999/refkit/refkit.mtl"
            ),
        ]
    );
    // The face packs nothing for the template's file: the refs CPK carries it.
    assert!(
        face.keys().all(|path| !path.contains("oral_arm")),
        "{:#?}",
        face.keys()
    );
    assert_tree_in(&entries, &referee_tree(Engine::PreFox));
}

/// The referee template's body as a referee's `face.xml` lists it when his folder holds
/// `fpc_off`, in the order a cup referee's own xml listed them in game: each model's name below
/// `refkit/`, without `oral_` and `_*.model`, with its type.
const REFKIT_BODY: [(&str, &str); 10] = [
    ("arm", "parts"),
    ("thigh", "parts"),
    ("refshirt", "parts"),
    ("pants", "parts"),
    ("pants_sub", "parts"),
    ("sleeve", "parts"),
    ("socks", "parts"),
    ("hand_l", "gloveL"),
    ("hand_r", "gloveR"),
    ("boots", "boots"),
];

/// The (type, path, material) of `REFKIT_BODY`'s entries but those of `left_out`, in order.
fn refkit_entries(left_out: &[&str]) -> Vec<(String, String, String)> {
    REFKIT_BODY
        .iter()
        .filter(|(name, _)| !left_out.contains(name))
        .map(|(name, xml_type)| {
            (
                (*xml_type).to_owned(),
                format!("model/character/uniform/common/999/refkit/oral_{name}_*.model"),
                "model/character/uniform/common/999/refkit/refkit.mtl".to_owned(),
            )
        })
        .collect()
}

/// Writes `Ref A` in slot 01 holding the card head as `face_high.model` with its
/// `face_high.mtl` and `skin.dds`, and `fpc_off`.
fn write_ref_a_needing_his_body(sandbox: &Sandbox) -> String {
    sandbox.write(&format!("{REFS}/players.txt"), b"01 Ref A\n");
    let folder = format!("{REFS}/Players/Ref A");
    sandbox.write(&format!("{folder}/face_high.model"), &card_model());
    sandbox.write(&format!("{folder}/face_high.mtl"), &card_materials());
    sandbox.write(&format!("{folder}/skin.dds"), &small_dds());
    sandbox.write(&format!("{folder}/fpc_off"), b"");
    folder
}

/// `compile_for` PES 17 asserting the run's findings are `export_identified` alone: slot 01's
/// face CPK's files, each by its name in the CPK's folder.
fn compile_ref_a_face_pes17(sandbox: &Sandbox) -> BTreeMap<String, Vec<u8>> {
    let (run, entries) = compile_for(sandbox, PesVersion::Pes17);
    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "refs Cup"),
        ["Info export_identified [Keep] (team=referees)"],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let folder = pre_fox_referee_face("01");
    nested_entries(&entries[&format!("{folder}.cpk")])
        .into_iter()
        .map(|(path, bytes)| {
            let name = path.strip_prefix(&format!("{folder}/")).unwrap().to_owned();
            (name, bytes)
        })
        .collect()
}

// TC-REF-16
#[test]
fn on_pes_17_a_referee_folder_holding_fpc_off_lists_the_refkit_body_after_his_face() {
    let sandbox = Sandbox::new("ref_refkit_body");
    write_ref_a_needing_his_body(&sandbox);

    let face = compile_ref_a_face_pes17(&sandbox);

    let mut expected = vec![(
        "face_neck".to_owned(),
        "./oral_face_high_*.model".to_owned(),
        "./face_high.mtl".to_owned(),
    )];
    expected.extend(refkit_entries(&[]));
    assert_eq!(ordered_entries(&face["face.xml"]), expected);
    // The body is the template tree's, in the refs CPK: the face packs none of it.
    let names: Vec<&String> = face.keys().collect();
    assert_eq!(
        names,
        ["face.xml", "face_high.mtl", "oral_face_high_win32.model"]
    );
}

// TC-REF-17
#[test]
fn on_pes_17_a_referee_holding_fpc_off_and_his_own_boots_takes_the_refkit_body_but_its_boots() {
    let sandbox = Sandbox::new("ref_refkit_own_boots");
    let folder = write_ref_a_needing_his_body(&sandbox);
    sandbox.write(&format!("{folder}/boots.model"), &card_model());
    sandbox.write(&format!("{folder}/boots.mtl"), &card_materials());

    let face = compile_ref_a_face_pes17(&sandbox);

    let entry = |xml_type: &str, path: &str, material: &str| {
        (xml_type.to_owned(), path.to_owned(), material.to_owned())
    };
    let mut expected = vec![
        entry("parts", "./oral_boots_*.model", "./boots.mtl"),
        entry("face_neck", "./oral_face_high_*.model", "./face_high.mtl"),
    ];
    expected.extend(refkit_entries(&["boots"]));
    assert_eq!(ordered_entries(&face["face.xml"]), expected);
}

// TC-REF-18
#[test]
fn a_referee_folder_with_a_category_subfolder_is_player_layout_proto_and_a_team_s_is_plain() {
    let sandbox = Sandbox::new("ref_layout_proto");
    let team = "co Midcup Subfolders";
    sandbox.write(&format!("{REFS}/players.txt"), b"01 Ref A\n");
    for folder in [
        format!("{REFS}/Players/Ref A"),
        format!("exports/{team}/Players/05 - A"),
    ] {
        for model in ["face_high", "boots/boots"] {
            sandbox.write(&format!("{folder}/{model}.model"), &card_model());
            sandbox.write(&format!("{folder}/{model}.mtl"), &card_materials());
        }
        sandbox.write(&format!("{folder}/skin.dds"), &small_dds());
    }

    let check = sandbox.run(&pes_settings(&sandbox, 17), &["check"]);

    // The referee's folder is the prototype layout, refused alone; the team player's
    // `boots/` is a subfolder of his own, his `boots.model` the boots by its name.
    let lines = check.messages();
    assert_eq!(
        findings_of(&lines, "refs Cup"),
        [
            "Error player_layout_proto [DropFolder] at Players/Ref A (folder=boots)",
            "Info export_identified [Keep] (team=referees)",
        ],
        "{lines:#?}"
    );
    assert_eq!(
        findings_of(&lines, team),
        ["Info export_identified [Keep] (team=/co/, id=714)"],
        "{lines:#?}"
    );
    assert_eq!(check.exit_code(), 1, "{lines:#?}");
}

// TC-REF-16
#[test]
fn on_pes_17_a_referee_s_own_face_xml_with_fpc_off_takes_the_refkit_body_it_does_not_name() {
    let sandbox = Sandbox::new("ref_xml_refkit_body");
    sandbox.write(&format!("{REFS}/players.txt"), b"01 Ref A\n");
    let folder = format!("{REFS}/Players/Ref A");
    sandbox.write(
        &format!("{folder}/oral_face_high_win32.model"),
        &card_model(),
    );
    sandbox.write(&format!("{folder}/face_high.mtl"), &card_materials());
    sandbox.write(&format!("{folder}/skin.dds"), &small_dds());
    sandbox.write(&format!("{folder}/fpc_off"), b"");
    let arm = "model/character/uniform/common/team_folder/refkit/oral_arm_*.model";
    let refkit_mtl = "team_common_folder/refkit/refkit.mtl";
    let xml = format!(
        "<config>\n   \
         <model level=\"0\" type=\"face_neck\" path=\"./oral_face_high_*.model\" material=\"./face_high.mtl\"/>\n   \
         <model level=\"0\" type=\"parts\" path=\"{arm}\" material=\"{refkit_mtl}\"/>\n\
         </config>\n"
    );
    sandbox.write(&format!("{folder}/face.xml"), xml.as_bytes());

    let face = compile_ref_a_face_pes17(&sandbox);

    let entry = |xml_type: &str, path: &str, material: &str| {
        (xml_type.to_owned(), path.to_owned(), material.to_owned())
    };
    // His own two entries as he wrote them, then the body's others: his `oral_arm` is the
    // refkit's, listed once.
    let mut expected = vec![
        entry("face_neck", "./oral_face_high_*.model", "./face_high.mtl"),
        entry(
            "parts",
            "model/character/uniform/common/999/refkit/oral_arm_*.model",
            "model/character/uniform/common/999/refkit/refkit.mtl",
        ),
    ];
    expected.extend(refkit_entries(&["arm"]));
    assert_eq!(ordered_entries(&face["face.xml"]), expected);
}

// TC-REF-17
#[test]
fn on_pes_17_a_referee_s_own_boots_or_gloves_leave_out_the_refkit_s() {
    let card = |sandbox: &Sandbox, path: &str| {
        sandbox.write(&format!("{REFS}/{path}.model"), &card_model());
        sandbox.write(&format!("{REFS}/{path}.mtl"), &card_materials());
    };
    // Each case: the model (a `.model` with its `.mtl`) it adds to TC-REF-16's Ref A, the link
    // to its shared folder when it is in one, and the refkit entries it leaves out.
    let cases: [(&str, &str, Option<&str>, &[&str]); 5] = [
        // A model named for the boots in a subfolder of his is his boots, as in his folder.
        (
            "boots_subfolder",
            "Players/Ref A/parts/studs_boots",
            None,
            &["boots"],
        ),
        // A model named for the boots is his boots, packed under another name than the
        // refkit's.
        ("kit_boots", "Players/Ref A/kit_boots", None, &["boots"]),
        // A link is his slot's `k9901`, which the refkit's boots would cover.
        (
            "boots_link",
            "Boots/Studs/boots",
            Some("Studs.boots"),
            &["boots"],
        ),
        // One glove is gloves: both refkit hands are left out.
        (
            "gloves_subfolder",
            "Players/Ref A/parts/glove_l",
            None,
            &["hand_l", "hand_r"],
        ),
        (
            "gloves_link",
            "Gloves/Wool/glove_l",
            Some("Wool.gloves"),
            &["hand_l", "hand_r"],
        ),
    ];
    for (name, model, link, left_out) in cases {
        let sandbox = Sandbox::new(&format!("ref_refkit_{name}"));
        write_ref_a_needing_his_body(&sandbox);
        card(&sandbox, model);
        if let Some(link) = link {
            sandbox.write(&format!("{REFS}/Players/Ref A/{link}"), b"");
            // The shared folder's own texture, which its `.mtl` names.
            let (shared, _) = model.rsplit_once('/').unwrap();
            sandbox.write(&format!("{REFS}/{shared}/skin.dds"), &small_dds());
        }

        let face = compile_ref_a_face_pes17(&sandbox);

        assert_eq!(
            refkit_entries_of(&face["face.xml"]),
            refkit_entries(left_out),
            "{name}"
        );
    }
}

// TC-REF-16
#[test]
fn on_pes_17_a_refkit_hand_a_referee_s_face_xml_lists_leaves_the_other_hand_appended() {
    // The refkit's folder in any letter case, as every Common path is compared.
    for (sandbox_name, refkit) in [
        ("ref_refkit_hand_by_hand", "refkit"),
        ("ref_refkit_hand_by_hand_cased", "RefKit"),
    ] {
        let sandbox = Sandbox::new(sandbox_name);
        sandbox.write(&format!("{REFS}/players.txt"), b"01 Ref A\n");
        let folder = format!("{REFS}/Players/Ref A");
        // A `face.xml` names a packed name: the card head is written under it.
        sandbox.write(
            &format!("{folder}/oral_face_high_win32.model"),
            &card_model(),
        );
        sandbox.write(&format!("{folder}/face_high.mtl"), &card_materials());
        sandbox.write(&format!("{folder}/skin.dds"), &small_dds());
        sandbox.write(&format!("{folder}/fpc_off"), b"");
        let hand_l =
            format!("model/character/uniform/common/team_folder/{refkit}/oral_hand_l_*.model");
        let refkit_mtl = "team_common_folder/refkit/refkit.mtl";
        let xml = format!(
            "<config>\n   \
             <model level=\"0\" type=\"face_neck\" path=\"./oral_face_high_*.model\" material=\"./face_high.mtl\"/>\n   \
             <model level=\"0\" type=\"gloveL\" path=\"{hand_l}\" material=\"{refkit_mtl}\"/>\n\
             </config>\n"
        );
        sandbox.write(&format!("{folder}/face.xml"), xml.as_bytes());

        let face = compile_ref_a_face_pes17(&sandbox);

        // The refkit's hand he lists is no glove of his own: only that hand is not appended.
        let mut expected = vec![(
            "gloveL".to_owned(),
            format!("model/character/uniform/common/999/{refkit}/oral_hand_l_*.model"),
            "model/character/uniform/common/999/refkit/refkit.mtl".to_owned(),
        )];
        expected.extend(refkit_entries(&["hand_l"]));
        assert_eq!(refkit_entries_of(&face["face.xml"]), expected, "{refkit}");
    }
}

/// The (type, path, material) of the entries of the `face.xml` `bytes` naming a model of the
/// referee template's body, in order, its folder in any letter case.
fn refkit_entries_of(bytes: &[u8]) -> Vec<(String, String, String)> {
    ordered_entries(bytes)
        .into_iter()
        .filter(|(_, path, _)| {
            path.to_ascii_lowercase()
                .starts_with("model/character/uniform/common/999/refkit/")
        })
        .collect()
}

// TC-OUT-04
#[test]
fn a_refs_export_whose_only_folder_validation_drops_writes_no_refs_cpk_and_no_tree() {
    let sandbox = Sandbox::new("ref_folder_dropped");
    write_ref_a(&sandbox, &["01"]);
    sandbox.write(&format!("{REFS}/Players/Ref A/readme.doc"), b"notes");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert!(
        lines.contains(
            &"refs Cup: Error file_type_disallowed [DropFolder] at Players/Ref A (file=readme.doc)"
                .to_owned()
        ),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
    // Nothing of the refs export went in, so neither the refs CPK nor its tree is written,
    // and the installed referees would stay.
    assert!(!sandbox.root.join("output").join(REFS_CPK).exists());
    assert_eq!(
        cpk_files(&sandbox.root.join("output")),
        Vec::<String>::new()
    );
}

/// Compiles Ref A for `version` in the sandbox `name` with a data directory file at the path of
/// `version`'s template tree's `RefereeAppearance.bin`, and asserts that the file replaces that
/// file of the refs CPK, reported first, the rest of the tree being the built-in one.
fn assert_tree_override_replaces_its_file(name: &str, version: PesVersion) {
    let sandbox = Sandbox::new(name);
    write_ref_a(&sandbox, &["01"]);
    let appearance = b"the cup's RefereeAppearance.bin";
    let folder = tree_folder(version.engine());
    let override_path = format!("data/templates/{folder}/{REFEREE_APPEARANCE}");
    sandbox.write(&override_path, appearance);

    let (run, entries) = compile_for(&sandbox, version);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    assert_eq!(
        run.messages().first(),
        Some(&format!(
            "Info template_override_active [Keep] (path={})",
            sandbox.display(&override_path)
        ))
    );
    assert_eq!(entries[REFEREE_APPEARANCE], appearance);
    let mut tree = referee_tree(version.engine());
    tree.insert(REFEREE_APPEARANCE.to_owned(), appearance.to_vec());
    assert_tree_in(&entries, &tree);
}

// TC-BIN-07
#[test]
fn a_data_directory_file_at_a_tree_path_replaces_that_file_of_the_refs_cpk() {
    assert_tree_override_replaces_its_file("ref_tree_override", PesVersion::Pes21);
}

// TC-BIN-07
#[test]
fn on_pes_17_a_data_directory_file_at_a_pre_fox_tree_path_replaces_that_file() {
    assert_tree_override_replaces_its_file("ref_tree_override_pes17", PesVersion::Pes17);
}

// TC-BIN-07, TC-REF-08
#[test]
fn a_data_directory_referee_kit_config_is_also_its_uniform_parameter_bin_entry() {
    let sandbox = Sandbox::new("ref_tree_config_override");
    write_ref_a(&sandbox, &["01"]);
    let def_1 = format!("{REFEREE_CONFIGS}referee_DEF_1.bin");
    let template = &template_configs(Engine::Fox)[&def_1];
    let mut config = KitConfig::decode(template, PesVersion::Pes21).unwrap();
    config.shirt.collar = 26;
    config.shirt.winter_collar = 26;
    let replacement = config.encode(PesVersion::Pes21).to_vec();
    assert_ne!(&replacement, template);
    sandbox.write(
        &format!("data/templates/referees_fox/{def_1}"),
        &replacement,
    );

    let (run, entries) = compile(&sandbox);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    // No marker: the replacement goes in as it is, collar 26.
    assert!(
        entries[&def_1] == replacement,
        "the replacement in the refs CPK"
    );
    assert!(
        team_uniform_parameter(&sandbox).get("referee_DEF_1.bin") == Some(replacement.as_slice()),
        "the replacement as its entry"
    );
}

// TC-REF-06
#[test]
fn a_ref_marker_goes_into_the_refs_cpk_as_collar_77_which_every_referee_kit_wears() {
    let sandbox = Sandbox::new("ref_marker");
    install_pes(&sandbox);
    // A data CPK of the game's: the marker is shown without writing anything outside the
    // refs CPK.
    let data_cpk = fs::read(templates_folder().join("placeholder.cpk")).unwrap();
    sandbox.write("PES/Data/dt00_x64.cpk", &data_cpk);
    write_refs_with_marker(&sandbox, Some(&tracer_player_file("shirt.dds")));

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let entries = cpk_entries(&sandbox.root.join("PES/download").join(REFS_CPK));
    assert!(entries.contains_key(MARKER_TEXTURE), "the marker texture");
    let collar = &entries[MARKER_COLLAR];
    assert_eq!(
        texture_directories(collar, "ref_marker.dds"),
        ["/Assets/pes16/model/character/common/999/sourceimages/"]
    );
    assert_eq!(
        texture_directories(collar, "cup_logo.dds"),
        Vec::<String>::new()
    );
    for (path, template) in template_configs(Engine::Fox) {
        let config = KitConfig::decode(&entries[&path], PesVersion::Pes21).unwrap();
        assert_eq!(
            (config.shirt.collar, config.shirt.winter_collar),
            (77, 77),
            "{path}"
        );
        assert_ne!(entries[&path], template, "{path}");
    }
    assert!(
        fs::read(sandbox.root.join("PES/Data/dt00_x64.cpk")).unwrap() == data_cpk,
        "dt00_x64.cpk untouched"
    );
}

/// The entries of the `UniformParameter.bin` `bytes`, by name.
fn uniform_parameter_entries(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    UniformParameter::read(bytes)
        .unwrap()
        .entries()
        .map(|(name, bytes)| (name.to_owned(), bytes.to_vec()))
        .collect()
}

// TC-REF-12
#[test]
fn on_fox_every_referee_kit_config_is_also_its_entry_in_the_team_cpk_s_uniform_parameter_bin() {
    let sandbox = Sandbox::new("ref_config_entries");
    write_refs_with_marker(&sandbox, Some(&tracer_player_file("shirt.dds")));

    let (run, entries) = compile(&sandbox);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert_eq!(
        cpk_files(&sandbox.root.join("output")),
        [REFS_CPK, TEAM_CPK]
    );
    assert_eq!(
        lines[lines.len() - 2..],
        [skipped(&sandbox, TEAM_CPK), skipped(&sandbox, REFS_CPK)],
        "the team CPK's line, then the refs CPK's"
    );
    let team = team_entries(&sandbox);
    for path in team.keys() {
        assert!(
            !path.contains("/999/") && !path.starts_with(REFEREE_CONFIGS),
            "{path}"
        );
    }
    let mut written = uniform_parameter_entries(&team[UNIFORM_PARAMETER]);
    let mut base = uniform_parameter_entries(&bundled_uniform_parameter());
    for path in template_configs(Engine::Fox).keys() {
        let name = entry_name(path);
        let entry = written
            .remove(name)
            .unwrap_or_else(|| panic!("no entry {name}"));
        assert!(entry == entries[path], "{path}: the refs CPK's loose file");
        let config = KitConfig::decode(&entry, PesVersion::Pes21).unwrap();
        assert_eq!(
            (config.shirt.collar, config.shirt.winter_collar),
            (77, 77),
            "{path}"
        );
        base.remove(name);
    }
    // The base's 2210 entries hold 10 of the 20 names (`referee_ACL_*`, `referee_DEF_*`): the
    // other 2200, the teams', are kept as they are.
    assert_eq!(base.len(), 2200);
    assert!(written == base, "every other entry is the bundled base's");
}

/// Compiles Ref A without `ref_marker.dds` for `version` in the sandbox `name`, and asserts
/// that the refs CPK holds none of `collars`, the marker's model files, no marker texture, and
/// every file of `version`'s template tree as it is (the pre-Fox prop texture and the kit
/// configs among them). Returns the sandbox.
fn assert_compiles_without_marker(name: &str, version: PesVersion, collars: &[&str]) -> Sandbox {
    let sandbox = Sandbox::new(name);
    write_refs_with_marker(&sandbox, None);

    let (run, entries) = compile_for(&sandbox, version);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    for collar in collars {
        assert!(!entries.contains_key(*collar), "no {collar}");
    }
    for path in entries.keys() {
        assert!(!path.contains("ref_marker"), "{path}");
    }
    assert_tree_in(&entries, &referee_tree(version.engine()));
    for (path, template) in template_configs(version.engine()) {
        assert!(entries[&path] == template, "{path}");
    }
    sandbox
}

// TC-REF-08
#[test]
fn without_a_ref_marker_the_refs_cpk_holds_no_collar_and_the_template_kit_configs() {
    let sandbox =
        assert_compiles_without_marker("ref_no_marker", PesVersion::Pes21, &[MARKER_COLLAR]);

    // Each template config is also its entry, so the team CPK is written for the bin.
    let bin = team_uniform_parameter(&sandbox);
    for (path, template) in template_configs(Engine::Fox) {
        assert!(
            bin.get(entry_name(&path)) == Some(template.as_slice()),
            "{path}"
        );
    }
}

// TC-REF-08
#[test]
fn without_a_ref_marker_a_pes_17_refs_cpk_holds_no_collar_pair_and_the_template_kit_configs() {
    assert_compiles_without_marker(
        "ref_no_marker_pes17",
        PesVersion::Pes17,
        &[
            PRE_FOX_MARKER_MODEL,
            PRE_FOX_MARKER_MTL,
            PRE_FOX_EMPTY_COLLAR,
        ],
    );
}

// TC-REF-04
// TC-REF-12
#[test]
fn on_pes_17_a_ref_marker_replaces_the_template_prop_texture_and_nothing_else() {
    let sandbox = Sandbox::new("ref_marker_pes17");
    write_refs_with_marker(&sandbox, Some(&tracer_player_file("shirt.dds")));

    let (run, entries) = compile_for(&sandbox, PesVersion::Pes17);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    // The pre-Fox referee configs are loose files alone: no bin to change, no team CPK.
    assert_eq!(cpk_files(&sandbox.root.join("output")), [REFS_CPK]);
    let mut tree = referee_tree(Engine::PreFox);
    assert!(
        tree.remove(REFEREE_PROP_TEXTURE).is_some(),
        "the template holds the prop texture the marker replaces"
    );
    let marker = &entries[REFEREE_PROP_TEXTURE];
    let decoded = decode(&tracer_player_file("shirt.dds"), SourceFormat::Dds).unwrap();
    let expected = dds_convert::convert(
        &decoded,
        Target {
            version: PesVersion::Pes17,
            role: TextureRole::Color,
            keep_uncompressed: true,
        },
    )
    .unwrap();
    assert!(*marker == expected, "the marker converted for PES 17");
    for path in [
        PRE_FOX_MARKER_MODEL,
        PRE_FOX_MARKER_MTL,
        PRE_FOX_EMPTY_COLLAR,
        PRE_FOX_COMMON_MARKER_TEXTURE,
    ] {
        assert!(!entries.contains_key(path), "no {path}");
    }
    assert_tree_in(&entries, &tree);
    for (path, template) in template_configs(Engine::PreFox) {
        assert!(entries[&path] == template, "{path}");
    }
}

// TC-REF-08
#[test]
fn a_ref_marker_that_fails_conversion_is_reported_and_left_out_with_its_collar() {
    let sandbox = Sandbox::new("ref_marker_failed");
    // A codec the converter cannot decode: the deep pass does not look at the marker, so its
    // conversion is what fails.
    write_refs_with_marker(&sandbox, Some(&texture_fixture("bc6h.dds")));

    let (run, entries) = compile(&sandbox);

    let lines = run.messages();
    assert!(
        lines.contains(
            &"refs Cup: Error texture_codec_unsupported [DropFile] at ref_marker.dds \
              (file=ref_marker.dds)"
                .to_owned()
        ),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
    // Ref A still commits, so the refs CPK and its tree are written, without the marker.
    assert!(entries.contains_key(&referee_package("face/real/referee0", "01", "face")));
    assert!(!entries.contains_key(MARKER_COLLAR), "no collar");
    assert!(!entries.contains_key(MARKER_TEXTURE), "no marker texture");
    for (path, template) in template_configs(Engine::Fox) {
        assert!(entries[&path] == template, "{path}");
    }
}

// TC-REF-03
#[test]
fn a_shared_face_two_referees_link_is_merged_into_each_one_s_face() {
    let sandbox = Sandbox::new("ref_shared_face");
    sandbox.write(&format!("{REFS}/players.txt"), b"01 Ref A\n02 Ref B\n");
    sandbox.write(&format!("{REFS}/Faces/Base/face_high.fmdl"), &clean_model());
    sandbox.write(&format!("{REFS}/Players/Ref A/Base.face"), b"");
    sandbox.write(
        &format!("{REFS}/Players/Ref A/hair_high.fmdl"),
        &clean_model(),
    );
    sandbox.write(&format!("{REFS}/Players/Ref B/Base.face"), b"");

    let (run, entries) = compile(&sandbox);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    assert_eq!(
        package_names(&entries[&referee_package("face/real/referee0", "01", "face")]),
        ["face_diff.bin", "face_high.fmdl", "hair_high.fmdl"]
    );
    assert_eq!(
        package_names(&entries[&referee_package("face/real/referee0", "02", "face")]),
        ["face_diff.bin", "face_high.fmdl"]
    );
    for path in entries.keys() {
        assert!(!path.contains("Base"), "{path}");
    }
}

// TC-REF-05
#[test]
fn test_mode_writes_a_referee_folder_s_processed_files_once() {
    let sandbox = Sandbox::new("ref_test_mode");
    write_ref_a(&sandbox, &["01", "20"]);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--mode", "test"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let written: Vec<String> = snapshot(&sandbox.root.join("output/test_output"))
        .into_keys()
        .map(|path| slashed(&path))
        .filter(|path| !path.starts_with("_bins/"))
        .collect();
    assert_eq!(
        written,
        [
            "refs Cup/Players/Ref A/boots.fmdl",
            "refs Cup/Players/Ref A/boots.skl",
            "refs Cup/Players/Ref A/face_diff.bin",
            "refs Cup/Players/Ref A/face_high.fmdl",
            "refs Cup/Players/Ref A/skin.ftex",
        ]
    );
    let tree = referee_tree(Engine::Fox);
    for path in snapshot(&sandbox.root.join("output/test_output")).keys() {
        let path = slashed(path);
        assert!(!path.contains("referee020"), "{path}");
        // The template tree is no export's, so test mode writes none of it, anywhere.
        assert!(
            !tree
                .keys()
                .any(|tree_path| path.ends_with(tree_path.as_str())),
            "{path}"
        );
    }
}

/// Writes `Ref A` mapped to slots 01 and 20, holding only the link `Studs.boots` to the shared
/// folder `Boots/Studs`, which holds `boots.fmdl` and `shirt.dds` (TC-REF-10's referee).
fn write_ref_a_linking_studs(sandbox: &Sandbox) {
    sandbox.write(&format!("{REFS}/players.txt"), b"01 Ref A\n20 Ref A\n");
    sandbox.write(&format!("{REFS}/Players/Ref A/Studs.boots"), b"");
    sandbox.write(
        &format!("{REFS}/Boots/Studs/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(
        &format!("{REFS}/Boots/Studs/shirt.dds"),
        &tracer_player_file("shirt.dds"),
    );
}

// TC-REF-10
#[test]
fn a_referee_s_link_to_shared_boots_is_written_as_each_of_his_slots_boots() {
    let sandbox = Sandbox::new("ref_shared_boots");
    write_ref_a_linking_studs(&sandbox);

    let (run, entries) = compile(&sandbox);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    // No face folder: with no face model his slots keep the game's referee head.
    let faces: Vec<&String> = entries
        .keys()
        .filter(|path| path.starts_with("Asset/model/character/face/real/referee0"))
        .collect();
    assert!(faces.is_empty(), "{faces:?}");
    let boots: Vec<&str> = entries
        .keys()
        .filter_map(|path| path.strip_prefix("Asset/model/character/boots/"))
        .collect();
    assert_eq!(
        boots,
        [
            "k9901/#Win/boots.fpk",
            "k9901/#Win/boots.fpkd",
            "k9920/#Win/boots.fpk",
            "k9920/#Win/boots.fpkd",
        ]
    );
    for slot in ["01", "20"] {
        let names = package_names(&entries[&referee_package("boots/k99", slot, "boots")]);
        assert!(
            names.contains(&"boots.fmdl".to_owned()),
            "{slot}: {names:?}"
        );
    }
}

// TC-REF-13
#[test]
fn on_pes_17_a_referee_s_link_to_shared_boots_is_each_slot_s_boots_folder_and_no_face() {
    let sandbox = Sandbox::new("ref_shared_boots_pes17");
    write_ref_a_linking_studs(&sandbox);

    let (run, entries) = compile_for(&sandbox, PesVersion::Pes17);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    // The template tree's stock boots, and the shared boots converted as each slot's folder.
    let boots: Vec<&str> = entries
        .keys()
        .filter_map(|path| path.strip_prefix("common/character0/model/character/boots/"))
        .collect();
    assert_eq!(
        boots,
        [
            "k0062/boots.model",
            "k0062/boots.mtl",
            "k9901/boots.model",
            "k9901/boots.mtl",
            "k9920/boots.model",
            "k9920/boots.mtl",
        ]
    );
    let model = |slot: &str| {
        &entries[&format!("common/character0/model/character/boots/k99{slot}/boots.model")]
    };
    assert_eq!(model("01"), model("20"));
    pes_model::format::PreFoxModel::read(model("01")).unwrap();
    // Not copied into a face as well, and no face of his own: the game's referee head stays.
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("xml_face_neck_added")),
        "{lines:#?}"
    );
    let faces: Vec<&String> = entries
        .keys()
        .filter(|path| path.starts_with("common/character0/model/character/face/real/referee0"))
        .collect();
    assert!(faces.is_empty(), "{faces:?}");
    let shirts: Vec<&String> = entries
        .keys()
        .filter(|path| path.ends_with("/shirt.dds"))
        .collect();
    assert_eq!(
        shirts,
        ["common/character1/model/character/uniform/common/999/Ref A/shirt.dds"]
    );
}

// His own per-kit boots are parts of his face, which lists their set; the shared boots are an
// output of their own, so their variant of another number is no variant of his face's set.
// TC-MOD-56
#[test]
fn on_pes_17_a_referee_s_own_boots_variant_leaves_out_none_of_his_linked_boots() {
    let sandbox = Sandbox::new("ref_shared_boots_variants_pes17");
    sandbox.write(&format!("{REFS}/players.txt"), b"01 Ref A\n");
    let player = format!("{REFS}/Players/Ref A");
    let studs = format!("{REFS}/Boots/Studs");
    sandbox.write(&format!("{player}/Studs.boots"), b"");
    for folder in [&player, &studs] {
        sandbox.write(&format!("{folder}/boots.mtl"), &card_materials());
        sandbox.write(&format!("{folder}/skin.dds"), &small_dds());
    }
    sandbox.write(&format!("{player}/boots_kit1.model"), &card_model());
    sandbox.write(&format!("{studs}/boots_kit2.model"), &card_model());

    let (run, entries) = compile_for(&sandbox, PesVersion::Pes17);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "refs Cup"),
        [
            "Info export_identified [Keep] (team=referees)",
            "Info xml_face_neck_added [Keep] at Players/Ref A ()"
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let boots: Vec<&str> = entries
        .keys()
        .filter_map(|path| path.strip_prefix("common/character0/model/character/boots/k9901/"))
        .collect();
    assert_eq!(boots, ["boots.model", "boots.mtl"]);
}

// TC-MOD-77
#[test]
fn on_pes_17_a_referee_s_linked_boots_model_takes_its_own_mtl_and_not_a_common_one() {
    let sandbox = Sandbox::new("ref_shared_boots_mtl_link_pes17");
    sandbox.write(&format!("{REFS}/players.txt"), b"01 Ref A\n");
    sandbox.write(&format!("{REFS}/Players/Ref A/Studs.boots"), b"");
    let studs = format!("{REFS}/Boots/Studs");
    sandbox.write(&format!("{studs}/boots.model"), &card_model());
    sandbox.write(&format!("{studs}/materials.mtl"), &card_materials());
    sandbox.write(&format!("{studs}/skin.dds"), &small_dds());
    // Kept by the lenient check alone: a shared folder's link resolves nothing, so the
    // search, name-matching it first, must not reach `Common/boots.mtl` through it.
    sandbox.write(&format!("{studs}/boots.mtl.common"), b"");
    sandbox.write(
        &format!("{REFS}/Common/boots.mtl"),
        &materials_naming("cloth"),
    );
    sandbox.write(&format!("{REFS}/Common/cloth.dds"), &small_dds());
    let settings = format!(
        "{}[team-compiler]\nstrict_file_type_check = false\n",
        pes_settings(&sandbox, 17)
    );

    let run = sandbox.run(&settings, &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output").join(REFS_CPK));
    let boots = "common/character0/model/character/boots/k9901/";
    let names: Vec<&str> = entries
        .keys()
        .filter_map(|path| path.strip_prefix(boots))
        .collect();
    assert_eq!(names, ["boots.model", "boots.mtl"]);
    // `materials.mtl`'s one texture, `skin.dds`, not the Common set's `cloth.dds`.
    let paths = sampler_paths(&entries[&format!("{boots}boots.mtl")]);
    assert_eq!(paths.len(), 1, "{paths:?}");
    assert!(paths[0].ends_with("/skin.dds"), "{paths:?}");
}

// TC-MOD-56
#[test]
fn on_pes_17_a_referee_s_linked_boots_folder_leaves_out_its_own_set_s_higher_variant() {
    let sandbox = Sandbox::new("ref_shared_boots_own_set_pes17");
    sandbox.write(&format!("{REFS}/players.txt"), b"01 Ref A\n");
    sandbox.write(&format!("{REFS}/Players/Ref A/Studs.boots"), b"");
    let studs = format!("{REFS}/Boots/Studs");
    for variant in ["boots_kit1", "boots_kit2"] {
        sandbox.write(&format!("{studs}/{variant}.model"), &card_model());
    }
    sandbox.write(&format!("{studs}/boots.mtl"), &card_materials());
    sandbox.write(&format!("{studs}/skin.dds"), &small_dds());

    let (run, entries) = compile_for(&sandbox, PesVersion::Pes17);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "refs Cup"),
        [
            "Info export_identified [Keep] (team=referees)",
            "Warning kit_variant_model_left_out [Keep] at Boots/Studs (model=boots_kitN.model, used=boots_kit1.model)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let meshes = |bytes: &[u8]| {
        pes_model::model::Model::from_file(&pes_model::format::PreFoxModel::read(bytes).unwrap())
            .unwrap()
            .meshes
            .len()
    };
    assert_eq!(
        meshes(&entries["common/character0/model/character/boots/k9901/boots.model"]),
        meshes(&card_model()),
        "one variant's meshes, not two merged"
    );
}

// TC-REF-11
#[test]
fn a_refs_export_s_kit_is_file_not_used_and_the_referee_face_compiles() {
    let sandbox = Sandbox::new("ref_kit_not_used");
    write_ref_a(&sandbox, &["01"]);
    sandbox.write(&format!("{REFS}/Kits/p1/kit.dds"), &tracer_kit());
    // The map PES 2018 to 2021 does not read: the kit folder is reported whole, and nothing
    // about its files.
    sandbox.write(&format!("{REFS}/Kits/p1/kit_mask.dds"), &tracer_kit());

    let (run, entries) = compile(&sandbox);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "refs Cup"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/Ref A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/Ref A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=referees)",
            "Warning file_not_used [Keep] (file=Kits/p1)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        entries.contains_key(&referee_package("face/real/referee0", "01", "face")),
        "{:#?}",
        entries.keys()
    );
    // The referees wear the template tree's kits alone.
    let kit_textures: Vec<&str> = entries
        .keys()
        .filter_map(|path| path.strip_prefix("Asset/model/character/uniform/texture/#windx11/"))
        .collect();
    assert!(!kit_textures.is_empty());
    assert!(
        kit_textures.iter().all(|name| name.starts_with("referee_")),
        "no kit of the export's: {kit_textures:#?}"
    );
}

// TC-REF-11
#[test]
fn a_refs_export_s_kits_logo_portraits_and_collars_are_not_read() {
    let sandbox = Sandbox::new("ref_unread_files");
    write_ref_a(&sandbox, &["01"]);
    // Each of them broken: nothing reads them, so none is checked.
    sandbox.write(&format!("{REFS}/Kits/p1/config.toml"), b"not toml");
    sandbox.write(&format!("{REFS}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(&format!("{REFS}/logo.png"), b"junk");
    sandbox.write(&format!("{REFS}/Portraits/01.dds"), b"junk");
    sandbox.write(&format!("{REFS}/Players/Ref A/portrait.dds"), b"junk");
    sandbox.write(&format!("{REFS}/Collars/collar_12.fmdl"), b"junk");

    let (run, entries) = compile(&sandbox);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "refs Cup"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/Ref A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/Ref A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=referees)",
            "Warning file_not_used [Keep] (file=Kits/p1)",
            "Warning file_not_used [Keep] (file=logo.png)",
            "Warning file_not_used [Keep] (file=Players/Ref A/portrait.dds)",
            "Warning file_not_used [Keep] (file=Portraits/01.dds)",
            "Warning file_not_used [Keep] (file=Collars/collar_12.fmdl)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        entries.contains_key(&referee_package("face/real/referee0", "01", "face")),
        "{:#?}",
        entries.keys()
    );
}

// TC-REF-11
#[test]
fn a_refs_export_s_kits_define_no_kit_number_to_complete_a_referee_s_texture_set_against() {
    let sandbox = Sandbox::new("ref_kit_numbers_pes17");
    write_ref_a(&sandbox, &["01"]);
    for slot in ["p1", "p2"] {
        write_kit(&sandbox, "refs Cup", slot);
    }
    sandbox.write(
        &format!("{REFS}/Players/Ref A/pants_kit1.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let (run, entries) = compile_for(&sandbox, PesVersion::Pes17);

    let lines = run.messages();
    // No `kit_variant_missing`: kit 2 is no number of a refs export's.
    assert_eq!(
        findings_of(&lines, "refs Cup"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/Ref A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/Ref A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=referees)",
            "Warning file_not_used [Keep] (file=Kits/p1)",
            "Warning file_not_used [Keep] (file=Kits/p2)",
            "Info native_field_dropped [Keep] at Players/Ref A (model=boots.fmdl, field=bone_matrices)",
            "Warning mesh_flags_dropped [Keep] at Players/Ref A (model=boots.fmdl, material=1, field=no_shadow_cast)",
            "Info native_field_dropped [Keep] at Players/Ref A (model=face_high.fmdl, field=bone_matrices)",
            "Warning mesh_flags_dropped [Keep] at Players/Ref A (model=face_high.fmdl, material=1, field=no_shadow_cast)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let pants: Vec<&String> = entries
        .keys()
        .filter(|path| path.contains("/pants_kit"))
        .collect();
    assert_eq!(
        pants,
        ["common/character1/model/character/uniform/common/999/Ref A/pants_kit1.dds"]
    );
}

/// Writes TC-REF-01's referee in slot 01 beside the tracer as /co/ (714).
fn refs_beside_co(sandbox: &Sandbox) {
    write_ref_a(sandbox, &["01"]);
    sandbox.copy_tracer("co Midcup Tracer");
}

/// Asserts the team CPK at `team` holds /co/'s kit and no referee path, and the refs CPK at
/// `refs` Ref A's face and none of /co/'s paths.
fn assert_split(team: &Path, refs: &Path) {
    let team = cpk_entries(team);
    assert!(team.contains_key(&kit_texture("u0714g1")), "/co/'s kit");
    for path in team.keys() {
        assert!(
            !path.contains("/999/") && !path.contains("referee0") && !path.contains("k99"),
            "{path}"
        );
    }
    let refs = cpk_entries(refs);
    assert!(refs.contains_key(&referee_package("face/real/referee0", "01", "face")));
    for path in refs.keys() {
        assert!(!path.contains("714"), "{path}");
    }
}

// TC-REF-02
#[test]
fn a_refs_export_beside_a_team_goes_into_its_own_cpk_promoted_after_the_team_s() {
    let sandbox = Sandbox::new("ref_beside_team");
    refs_beside_co(&sandbox);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let output = sandbox.root.join("output");
    assert_split(&output.join("4cc_99_test.cpk"), &output.join(REFS_CPK));
    let lines = run.messages();
    let skips: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains("deploy_skipped_by_flag"))
        .collect();
    assert_eq!(
        skips,
        [
            &skipped(&sandbox, "4cc_99_test.cpk"),
            &skipped(&sandbox, REFS_CPK)
        ],
        "one each, the team CPK first"
    );
    let written = cpk_entries(&output.join("4cc_99_test.cpk")).len()
        + cpk_entries(&output.join(REFS_CPK)).len();
    assert_eq!(
        run.complete().files_written,
        written,
        "both CPKs' tables of contents, the referee template tree's entries among the refs CPK's"
    );
}

// TC-REF-02
#[test]
fn a_deploying_compile_installs_the_refs_cpk_with_the_team_s() {
    let sandbox = Sandbox::new("ref_beside_team_deploys");
    install_pes(&sandbox);
    refs_beside_co(&sandbox);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let download = sandbox.root.join("PES/download");
    assert_split(&download.join("4cc_99_test.cpk"), &download.join(REFS_CPK));
    assert_eq!(
        cpk_files(&sandbox.root.join("output")),
        Vec::<String>::new()
    );
}

// TC-REF-20
#[test]
fn a_referee_with_the_gloves_checkbox_and_no_gloves_is_reported() {
    let sandbox = Sandbox::new("ref_gloves_checkbox");
    sandbox.write(&format!("{REFS}/players.txt"), b"01 Ref A\n");
    let folder = format!("{REFS}/Players/Ref A");
    sandbox.write(&format!("{folder}/face_high.fmdl"), &clean_model());
    sandbox.write(
        &format!("{folder}/settings.toml"),
        b"[appearance.strip]\ngloves = true\n",
    );

    for args in [&["check"][..], &["compile", "--no-deploy"][..]] {
        let run = sandbox.run(&pes21_settings(&sandbox), args);
        let lines = run.messages();
        assert_eq!(run.exit_code(), 0, "{args:?}: {lines:#?}");
        assert!(
            lines.iter().any(|line| line
                == "refs Cup: Warning fpc_gloves_checkbox_winter_gloves [Keep] at \
                    Players/Ref A ()"),
            "{args:?}: {lines:#?}"
        );
    }
    compile(&sandbox);
    // A referee's settings reach no patch either.
    assert!(!sandbox.root.join("output/aesthetics_patch.toml").exists());
}

// TC-REF-20
#[test]
fn a_referee_s_linked_gloves_his_own_package_are_no_winter_gloves() {
    let sandbox = Sandbox::new("ref_gloves_link");
    sandbox.write(&format!("{REFS}/players.txt"), b"01 Ref A\n");
    let folder = format!("{REFS}/Players/Ref A");
    sandbox.write(&format!("{folder}/face_high.fmdl"), &clean_model());
    // His gloves are the linked folder's, built as his slot's `g99NN`: not the hands 0
    // the rule fires on.
    sandbox.write(&format!("{folder}/Grip.gloves"), b"");
    sandbox.write(
        &format!("{REFS}/Gloves/Grip/glove_l.fmdl"),
        &tracer_player_file("glove_l.fmdl"),
    );
    sandbox.write(
        &format!("{folder}/settings.toml"),
        b"[appearance.strip]\ngloves = true\n",
    );

    for args in [&["check"][..], &["compile", "--no-deploy"][..]] {
        let run = sandbox.run(&pes21_settings(&sandbox), args);
        let lines = run.messages();
        assert_eq!(run.exit_code(), 0, "{args:?}: {lines:#?}");
        assert!(
            !lines
                .iter()
                .any(|line| line.contains("fpc_gloves_checkbox_winter_gloves")),
            "{args:?}: {lines:#?}"
        );
    }
}
