//! `check` and `compile` for PES 2017 looking up the textures a `.mtl` names: a texture of a
//! material a mesh uses must be in the player's folder, or, named in the team's
//! Common folder, in `Common/` or in an installed CPK listed before the one compiled.

use pes_model::format::mtl::{MaterialEntry, MaterialSet};

use crate::bins::{install_cpk, install_names};
use crate::common::Sandbox;
use crate::compile::cpk_entries;
use crate::findings_of;
use crate::prefox_faces::{
    card_materials, card_model, face_cpk, face_folder, materials_naming, nested_entries, pes17,
    pre_fox_fixture, sampler_paths, small_dds,
};

/// The line both commands report for each team 714 export of these tests: its identity.
const IDENTIFIED: &str = "Info export_identified [Keep] (team=/co/, id=714)";

/// Writes slot `slot` of the export `export`: `face_high.model` (the card head, binding
/// `card`), `skin.dds`, and `face_high.mtl` holding `mtl`.
fn write_face(sandbox: &Sandbox, export: &str, slot: &str, mtl: &[u8]) {
    let player = format!("exports/{export}/Players/{slot}");
    sandbox.write(&format!("{player}/face_high.model"), &card_model());
    sandbox.write(&format!("{player}/face_high.mtl"), mtl);
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
}

/// The card head's material set naming `skin.dds`, with a second material, `unused`, which
/// the card head's model does not bind, naming `./missing.dds`.
fn materials_with_an_unused_one() -> Vec<u8> {
    let mut set = MaterialSet::read(&card_materials()).unwrap();
    let mut unused = set.materials[0].clone();
    unused.name = "unused".to_owned();
    for entry in &mut unused.entries {
        if let MaterialEntry::Sampler(sampler) = entry {
            sampler.path = "./missing.dds".to_owned();
        }
    }
    set.materials.push(unused);
    set.write()
}

/// The card head's material set with its one texture path, `./texture.dds`, replaced by
/// `path`.
fn materials_at(path: &str) -> Vec<u8> {
    let text = String::from_utf8(pre_fox_fixture("cardhead_materials.mtl")).unwrap();
    assert!(text.contains("./texture.dds"), "{text}");
    text.replace("./texture.dds", path).into_bytes()
}

// TC-XML-07
#[test]
fn a_texture_a_used_material_names_that_the_folder_lacks_is_a_warning_and_the_face_is_kept() {
    let sandbox = Sandbox::new("prefox_mtl_texture_missing");
    let export = "co Midcup Hair";
    // Slot 05 has no `hair.*`; slot 03 is clean.
    write_face(&sandbox, export, "05 - A", &materials_naming("hair"));
    write_face(&sandbox, export, "03 - B", &card_materials());
    let not_found = "Warning mtl_texture_not_found [Keep] at Players/05 - A (file=face_high.mtl, texture=./hair.dds, materials=card)";

    let check = sandbox.run(&pes17(&sandbox), &["check"]);
    let lines = check.messages();
    assert_eq!(
        findings_of(&lines, export),
        [not_found, IDENTIFIED],
        "{lines:#?}"
    );
    assert_eq!(check.exit_code(), 0);

    let compile = sandbox.run(&pes17(&sandbox), &["compile", "--no-deploy"]);
    let lines = compile.messages();
    assert_eq!(
        findings_of(&lines, export),
        [not_found, IDENTIFIED, "Info team_colors_missing [Keep] ()"],
        "{lines:#?}"
    );
    assert_eq!(compile.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(entries.contains_key(&face_cpk(3)), "{:?}", entries.keys());
    // Slot 05's face is written, its missing texture's path as the member wrote it.
    let face = nested_entries(&entries[&face_cpk(5)]);
    let mtl = &face[&format!("{}face_high.mtl", face_folder(5))];
    assert_eq!(sampler_paths(mtl), ["./hair.dds"]);
}

// TC-XML-07
#[test]
fn a_texture_only_an_unused_material_names_is_an_info_and_its_path_is_kept() {
    let sandbox = Sandbox::new("prefox_mtl_texture_unused");
    let export = "co Midcup Spare";
    write_face(&sandbox, export, "05 - A", &materials_with_an_unused_one());
    let unused = "Info mtl_texture_unused_missing [Keep] at Players/05 - A (file=face_high.mtl, texture=./missing.dds, materials=unused)";

    let check = sandbox.run(&pes17(&sandbox), &["check"]);
    let lines = check.messages();
    assert_eq!(
        findings_of(&lines, export),
        [unused, IDENTIFIED],
        "{lines:#?}"
    );
    assert_eq!(check.exit_code(), 0);

    let compile = sandbox.run(&pes17(&sandbox), &["compile", "--no-deploy"]);
    let lines = compile.messages();
    assert_eq!(
        findings_of(&lines, export),
        [unused, IDENTIFIED, "Info team_colors_missing [Keep] ()"],
        "{lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let face = nested_entries(&entries[&face_cpk(5)]);
    let mtl = &face[&format!("{}face_high.mtl", face_folder(5))];
    // The texture the folder holds is pointed at the player's texture home; the missing one
    // keeps its path.
    assert_eq!(
        sampler_paths(mtl),
        [
            "model/character/uniform/common/714/05 - A/skin.dds",
            "./missing.dds"
        ]
    );
}

/// TC-TEX-05's three runs for a `.mtl`: slot 05's `face_high.mtl` names `hair.dds` in the
/// team's uniform Common folder, the export has no `Common/`, and the settings compile
/// `4cc_62_midcup` with the sandbox's PES folder.
fn write_common_hair_mtl(sandbox: &Sandbox) -> String {
    write_face(
        sandbox,
        "co Midcup Hair",
        "05 - A",
        &materials_at("team_common_folder/hair.dds"),
    );
    format!(
        "{}[team-compiler]\ncpk_name = \"4cc_62_midcup\"\n",
        pes17(sandbox)
    )
}

/// Installs the list `4cc_61_midcup.cpk`, `4cc_62_midcup.cpk`, `4cc_63_midcup.cpk`, in that
/// order, and the CPK `holder` holding the team's pre-Fox Common `hair.dds`.
fn install_pre_fox_hair_in(sandbox: &Sandbox, holder: &str) {
    install_names(
        sandbox,
        &[
            "4cc_61_midcup.cpk",
            "4cc_62_midcup.cpk",
            "4cc_63_midcup.cpk",
        ],
    );
    install_cpk(
        sandbox,
        holder,
        &[(
            "common/character1/model/character/uniform/common/714/hair.dds",
            b"compiled on an earlier day",
        )],
    );
}

/// The `check` lines of the export of `write_common_hair_mtl` run with `settings`.
fn hair_check_lines(sandbox: &Sandbox, settings: &str) -> Vec<String> {
    let run = sandbox.run(settings, &["check"]);
    findings_of(&run.messages(), "co Midcup Hair")
        .into_iter()
        .map(str::to_owned)
        .collect()
}

// The runs of TC-TEX-05, for a `.mtl`.
// TC-XML-17
#[test]
fn a_common_texture_of_a_mtl_is_found_in_an_earlier_installed_cpk_only() {
    let earlier = Sandbox::new("prefox_mtl_installed_earlier");
    let settings = write_common_hair_mtl(&earlier);
    install_pre_fox_hair_in(&earlier, "4cc_61_midcup.cpk");
    assert_eq!(hair_check_lines(&earlier, &settings), [IDENTIFIED]);

    let later = Sandbox::new("prefox_mtl_installed_later");
    let settings = write_common_hair_mtl(&later);
    install_pre_fox_hair_in(&later, "4cc_63_midcup.cpk");
    let not_found = "Warning mtl_texture_not_found [Keep] at Players/05 - A (file=face_high.mtl, texture=team_common_folder/hair.dds, materials=card)";
    assert_eq!(hair_check_lines(&later, &settings), [not_found, IDENTIFIED]);

    // No PES folder: the CPKs cannot be looked in, which holds nothing, the same Warning.
    let unknown = Sandbox::new("prefox_mtl_installed_unknown");
    let settings = write_common_hair_mtl(&unknown);
    assert_eq!(
        hair_check_lines(&unknown, &settings),
        [not_found, IDENTIFIED]
    );
}

// not looked up
// TC-CMN-36
#[test]
fn a_game_path_and_a_kit_stand_in_are_not_looked_for_and_the_stand_in_is_written_the_game_s() {
    let sandbox = Sandbox::new("prefox_mtl_texture_game_path");
    let export = "co Midcup Lashes";
    let game_path = "model/character/face/common/face_eyelash.dds";
    let mut set = MaterialSet::read(&card_materials()).unwrap();
    let material = &mut set.materials[0];
    let MaterialEntry::Sampler(sampler) = material.entries[0].clone() else {
        panic!("the card material's first entry is its sampler");
    };
    material.entries[0] = MaterialEntry::Sampler(pes_model::format::mtl::Sampler {
        path: game_path.to_owned(),
        ..sampler.clone()
    });
    material.entries.insert(
        1,
        MaterialEntry::Sampler(pes_model::format::mtl::Sampler {
            path: "team_common_folder/team_kit.dds".to_owned(),
            ..sampler
        }),
    );
    write_face(&sandbox, export, "05 - A", &set.write());

    let check = sandbox.run(&pes17(&sandbox), &["check"]);
    let lines = check.messages();
    assert_eq!(findings_of(&lines, export), [IDENTIFIED], "{lines:#?}");
    assert_eq!(check.exit_code(), 0);

    let compile = sandbox.run(&pes17(&sandbox), &["compile", "--no-deploy"]);
    assert_eq!(compile.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let face = nested_entries(&entries[&face_cpk(5)]);
    let mtl = &face[&format!("{}face_high.mtl", face_folder(5))];
    // The stand-in is written in the game's spelling at the team's Common; the
    // game path is kept as written.
    assert_eq!(
        sampler_paths(mtl),
        [
            game_path,
            "model/character/uniform/common/714/dummy_kit.dds",
        ]
    );
}

/// Writes slot 05 of `co Midcup Hair`: `face_high.mtl` naming `hair.dds` in the `jessie/`
/// subdirectory of the team's uniform Common folder; the settings compile `4cc_62_midcup`
/// for PES 17 with the sandbox's PES folder.
fn write_subfolder_hair_mtl(sandbox: &Sandbox) -> String {
    write_face(
        sandbox,
        "co Midcup Hair",
        "05 - A",
        &materials_at("model/character/uniform/common/team_folder/jessie/hair.dds"),
    );
    format!(
        "{}[team-compiler]\ncpk_name = \"4cc_62_midcup\"\n",
        pes17(sandbox)
    )
}

// A path into a `Common/` subfolder; the runs of TC-TEX-05, for a `.mtl`.
// TC-XML-17
#[test]
fn a_common_subfolder_texture_of_a_mtl_is_found_in_common_or_an_earlier_installed_cpk() {
    let not_found = "Warning mtl_texture_not_found [Keep] at Players/05 - A (file=face_high.mtl, texture=model/character/uniform/common/team_folder/jessie/hair.dds, materials=card)";

    // Nobody holds it: the list names the CPKs but none is written, so the lookup is made.
    let missing = Sandbox::new("prefox_mtl_subfolder_missing");
    let settings = write_subfolder_hair_mtl(&missing);
    install_names(
        &missing,
        &[
            "4cc_61_midcup.cpk",
            "4cc_62_midcup.cpk",
            "4cc_63_midcup.cpk",
        ],
    );
    assert_eq!(
        hair_check_lines(&missing, &settings),
        [not_found, IDENTIFIED]
    );

    // The export's `Common/jessie/hair.dds` holds it at the path it names.
    let held = Sandbox::new("prefox_mtl_subfolder_held");
    let settings = write_subfolder_hair_mtl(&held);
    held.write(
        "exports/co Midcup Hair/Common/jessie/hair.dds",
        &small_dds(),
    );
    assert_eq!(hair_check_lines(&held, &settings), [IDENTIFIED]);

    // An earlier installed CPK holds it at the path the Common output packs it.
    let installed = Sandbox::new("prefox_mtl_subfolder_installed");
    let settings = write_subfolder_hair_mtl(&installed);
    install_names(
        &installed,
        &[
            "4cc_61_midcup.cpk",
            "4cc_62_midcup.cpk",
            "4cc_63_midcup.cpk",
        ],
    );
    install_cpk(
        &installed,
        "4cc_61_midcup.cpk",
        &[(
            "common/character1/model/character/uniform/common/714/jessie/hair.dds",
            b"compiled on an earlier day",
        )],
    );
    assert_eq!(hair_check_lines(&installed, &settings), [IDENTIFIED]);
}
