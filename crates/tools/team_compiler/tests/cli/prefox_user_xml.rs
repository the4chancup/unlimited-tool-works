//! A player folder's own `face.xml` (`spec/team_compiler/messages.md` "User-supplied `face.xml`"):
//! read and checked by `check` for PES 2015 to 2017, its errors dropping the folder, and
//! written back by `compile` with only the files it names packed; ignored for PES 2018 to 2021,
//! whose `compile` builds the folder as without it.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::common::Sandbox;
use crate::compile::{cpk_entries, pes21_settings, tracer_kit};
use crate::models::{face_diff_fixture, template};
use crate::prefox_faces::{
    CLEAN, card_materials, card_model, common_output, compile_pes17, entries_under, face_cpk,
    face_folder, materials_naming, nested_entries, ordered_entries, pes15, pes16, pes17,
    pre_fox_fixture, sampler_paths, small_dds,
};
use crate::{clean_model, findings_of};

/// `/co/`'s identity line.
const IDENTIFIED: &str = "Info export_identified [Keep] (team=/co/, id=714)";

/// Writes the player folder `folder` of the export `export` holding the card head's model as
/// `<stem>.model`, its `.mtl` naming `skin.dds`, that texture, and the `face.xml` `xml`.
fn write_folder(sandbox: &Sandbox, export: &str, folder: &str, stem: &str, xml: &str) {
    let player = format!("exports/{export}/Players/{folder}");
    sandbox.write(&format!("{player}/{stem}.model"), &card_model());
    sandbox.write(&format!("{player}/{stem}.mtl"), &card_materials());
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    sandbox.write(&format!("{player}/face.xml"), xml.as_bytes());
}

/// The `face.xml` listing `entries`, each a `<model>`'s attributes.
fn face_xml(entries: &[&str]) -> String {
    let models: String = entries
        .iter()
        .map(|attributes| format!("   <model level=\"0\" {attributes}/>\n"))
        .collect();
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<config>\n{models}</config>\n")
}

/// The `face_high` entry, which names its two files.
const FACE_HIGH: &str = r#"type="face_neck" path="./face_high.model" material="./face_high.mtl""#;

// TC-XML-03
#[test]
fn an_entry_without_path_and_one_naming_an_absent_model_each_drop_their_folder() {
    let sandbox = Sandbox::new("user_xml_errors");
    let export = "co Midcup Xml";
    write_folder(
        &sandbox,
        export,
        "05 - A",
        "face_high",
        &face_xml(&[r#"type="face_neck" material="./face_high.mtl""#]),
    );
    write_folder(
        &sandbox,
        export,
        "07 - B",
        "face_high",
        &face_xml(&[FACE_HIGH, r#"type="parts" path="./hat.model""#]),
    );

    let run = sandbox.run(&pes17(&sandbox), &["check"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Error xml_model_path_missing [DropFolder] at Players/05 - A (entry=1, type=face_neck)",
            "Warning xml_model_unlisted [Keep] at Players/05 - A (file=face_high.model)",
            "Error xml_model_not_found [DropFolder] at Players/07 - B (attribute=path, value=./hat.model)",
            IDENTIFIED,
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-XML-04
#[test]
fn a_dif_in_the_xml_beside_a_face_diff_xml_drops_the_folder() {
    let sandbox = Sandbox::new("user_xml_dif_conflict");
    let export = "co Midcup Xml";
    let dif = face_diff_fixture("dif.xml");
    let dif_text = String::from_utf8(dif.clone()).unwrap();
    // The fixture's `<dif>` text, the same face diff given twice.
    let start = dif_text.find("<dif>").unwrap() + "<dif>".len();
    let end = dif_text.find("</dif>").unwrap();
    let base64 = &dif_text[start..end];
    let xml =
        format!("<config>\n   <model level=\"0\" {FACE_HIGH}/>\n<dif>{base64}</dif>\n</config>\n");
    write_folder(&sandbox, export, "05 - A", "face_high", &xml);
    sandbox.write(
        &format!("exports/{export}/Players/05 - A/face_diff.xml"),
        &dif,
    );

    let run = sandbox.run(&pes17(&sandbox), &["check"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Error xml_dif_conflict [DropFolder] at Players/05 - A (file=face.xml)",
            IDENTIFIED,
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-XML-11
#[test]
fn a_dif_in_the_xml_beside_a_face_diff_bin_is_each_engine_s_own() {
    let sandbox = Sandbox::new("user_xml_dif_dual_engine");
    let export = "co Midcup Xml";
    let dif_text = String::from_utf8(face_diff_fixture("dif.xml")).unwrap();
    let start = dif_text.find("<dif>").unwrap() + "<dif>".len();
    let end = dif_text.find("</dif>").unwrap();
    let base64 = dif_text[start..end].trim();
    let xml =
        format!("<config>\n   <model level=\"0\" {FACE_HIGH}/>\n<dif>{base64}</dif>\n</config>\n");
    write_folder(&sandbox, export, "05 - A", "face_high", &xml);
    // A real bin other than the one the xml's `<dif>` decodes to, so each engine's pick shows.
    let bin = face_diff_fixture("plain.bin");
    assert_ne!(bin, face_diff_fixture("dif.bin"));
    sandbox.write(
        &format!("exports/{export}/Players/05 - A/face_diff.bin"),
        &bin,
    );

    let entries = compile_pes17(&sandbox, export, &CLEAN);

    let face = slot_05_face(&entries);
    assert_eq!(
        names(&face),
        ["face.xml", "face_high.model", "face_high.mtl"]
    );
    assert_eq!(decoded_dif(&face["face.xml"]), face_diff_fixture("dif.bin"));

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    let findings = findings_of(&lines, export);
    assert!(
        findings.contains(&"Info xml_ignored_fox [Keep] at Players/05 - A (file=face.xml)"),
        "{lines:#?}"
    );
    assert!(
        findings
            .iter()
            .all(|line| !line.contains("xml_dif_conflict") && !line.contains("face_diff.bin")),
        "{lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let face = fpk::FpkFile::read(&entries["Asset/model/character/face/real/71405/#Win/face.fpk"])
        .unwrap();
    assert_eq!(face.get("face_diff.bin").unwrap(), bin);
}

// TC-XML-13
#[test]
fn on_pes_17_a_face_diff_bin_the_xml_s_dif_replaces_is_not_checked() {
    let sandbox = Sandbox::new("user_xml_dif_bin_unread");
    let export = "co Midcup Xml";
    let dif_text = String::from_utf8(face_diff_fixture("dif.xml")).unwrap();
    let start = dif_text.find("<dif>").unwrap() + "<dif>".len();
    let end = dif_text.find("</dif>").unwrap();
    let base64 = dif_text[start..end].trim();
    let xml =
        format!("<config>\n   <model level=\"0\" {FACE_HIGH}/>\n<dif>{base64}</dif>\n</config>\n");
    write_folder(&sandbox, export, "05 - A", "face_high", &xml);
    // One byte shorter than its header gives: `face_diff_invalid` were it read.
    sandbox.write(
        &format!("exports/{export}/Players/05 - A/face_diff.bin"),
        &face_diff_fixture("dif.bin")[..943],
    );

    let entries = compile_pes17(&sandbox, export, &CLEAN);

    let face = slot_05_face(&entries);
    assert_eq!(
        names(&face),
        ["face.xml", "face_high.model", "face_high.mtl"]
    );
    assert_eq!(decoded_dif(&face["face.xml"]), face_diff_fixture("dif.bin"));
}

// TC-XML-12
#[test]
fn a_member_s_xml_pairing_a_common_model_with_a_mtl_lacking_its_materials_drops_the_folder() {
    let sandbox = Sandbox::new("user_xml_common_undefined");
    let export = "co Midcup Xml";
    let legs = r#"type="parts" path="team_common_folder/legs.model" material="./legs.mtl""#;
    write_folder(
        &sandbox,
        export,
        "05 - A",
        "face_high",
        &face_xml(&[FACE_HIGH, legs]),
    );
    // The card model binds `card`; this set defines only `other`.
    let materials = String::from_utf8(card_materials()).unwrap();
    assert!(materials.contains(r#"name="card""#), "{materials}");
    sandbox.write(
        &format!("exports/{export}/Players/05 - A/legs.mtl"),
        materials
            .replace(r#"name="card""#, r#"name="other""#)
            .as_bytes(),
    );
    sandbox.write(
        &format!("exports/{export}/Common/legs.model"),
        &card_model(),
    );

    let run = sandbox.run(&pes17(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Error model_material_undefined [DropFolder] at Players/05 - A (file=Common/legs.model, mtl=legs.mtl, materials=card)",
            IDENTIFIED,
            "Info team_colors_missing [Keep] ()",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(!entries.contains_key(&face_cpk(5)), "{:?}", entries.keys());
}

// TC-XML-06
#[test]
fn on_pes_21_the_xml_is_ignored_and_the_folder_compiles_as_without_it() {
    let compiled = |name: &str, with_xml: bool| {
        let sandbox = Sandbox::new(name);
        let player = "exports/co Midcup Xml/Players/05 - A";
        sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
        if with_xml {
            sandbox.write(
                &format!("{player}/face.xml"),
                face_xml(&[r#"type="face_neck" path="./face_high.model""#]).as_bytes(),
            );
        }
        let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
        let lines = run.messages();
        assert_eq!(run.exit_code(), 0, "{lines:#?}");
        let findings: Vec<String> = findings_of(&lines, "co Midcup Xml")
            .into_iter()
            .map(str::to_owned)
            .collect();
        (
            findings,
            cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk")),
        )
    };

    let (with_findings, with_entries) = compiled("user_xml_fox", true);
    let (without_findings, without_entries) = compiled("user_xml_fox_without", false);

    assert_eq!(
        with_findings,
        [
            IDENTIFIED,
            "Info xml_ignored_fox [Keep] at Players/05 - A (file=face.xml)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert_eq!(
        without_findings,
        [IDENTIFIED, "Info team_colors_missing [Keep] ()"]
    );
    assert!(
        with_entries
            .keys()
            .any(|path| path.contains("face/real/71405")),
        "{:?}",
        with_entries.keys()
    );
    assert_eq!(with_entries, without_entries);
}

// TC-XML-09
#[test]
fn on_pes_16_a_model_name_without_its_prefix_drops_the_folder_and_on_pes_17_it_does_not() {
    let sandbox = Sandbox::new("user_xml_oral_prefix");
    let export = "co Midcup Xml";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/body_uniform.model"), &card_model());
    sandbox.write(
        &format!("{player}/body_uniform.mtl"),
        &materials_naming("body"),
    );
    sandbox.write(&format!("{player}/body.dds"), &small_dds());
    sandbox.write(
        &format!("{player}/face.xml"),
        face_xml(&[r#"type="uniform" path="./body_uniform.model" material="./body_uniform.mtl""#])
            .as_bytes(),
    );

    let pes_16 = sandbox.run(&pes16(&sandbox), &["check"]);
    let lines = pes_16.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Error xml_oral_prefix_missing [DropFolder] at Players/05 - A (path=./body_uniform.model)",
            IDENTIFIED,
        ],
        "{lines:#?}"
    );
    assert_eq!(pes_16.exit_code(), 1);

    let pes_17 = sandbox.run(&pes17(&sandbox), &["check"]);
    let lines = pes_17.messages();
    assert_eq!(findings_of(&lines, export), [IDENTIFIED], "{lines:#?}");
    assert_eq!(pes_17.exit_code(), 0);
}

/// Slot 05's face CPK among the output CPK's `entries`, each file by its name in the face.
fn slot_05_face(entries: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
    slot_face(entries, 5)
}

/// Slot `slot`'s face CPK among the output CPK's `entries`, each file by its name in the face.
fn slot_face(entries: &BTreeMap<String, Vec<u8>>, slot: u8) -> BTreeMap<String, Vec<u8>> {
    let folder = face_folder(slot);
    nested_entries(&entries[&face_cpk(slot)])
        .into_iter()
        .map(|(path, bytes)| {
            (
                path.strip_prefix(folder.as_str()).unwrap().to_owned(),
                bytes,
            )
        })
        .collect()
}

/// The names of the files of `face`, in their order.
fn names(face: &BTreeMap<String, Vec<u8>>) -> Vec<&str> {
    face.keys().map(String::as_str).collect()
}

/// The first two lines every written `face.xml` starts with, as the game's own files spell
/// them.
const HEAD: &str = "<?xml version='1.0' encoding='UTF-8'?>\r\n<config>\r\n";

/// The `<dif>` lines and the root's end every written `face.xml` ends with, for the face diff
/// `dif`.
fn dif_tail(dif: &[u8]) -> String {
    format!("<dif>\r\n{}\r\n</dif>\r\n</config>", STANDARD.encode(dif))
}

/// The texture home of slot 05's textures in team 714's Common output, as a `.mtl` names it.
const SLOT_05_HOME: &str = "model/character/uniform/common/714/05 - A/";

/// Writes slot 05 of `export` holding `face_high.model` and `hat_parts.model` (the card head's
/// model), their `.mtl` files naming `skin.dds`, that texture, and `face.xml` when given.
fn write_face_and_hat(sandbox: &Sandbox, export: &str, xml: Option<&str>) {
    let player = format!("exports/{export}/Players/05 - A");
    for stem in ["face_high", "hat_parts"] {
        sandbox.write(&format!("{player}/{stem}.model"), &card_model());
        sandbox.write(&format!("{player}/{stem}.mtl"), &card_materials());
    }
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    if let Some(xml) = xml {
        sandbox.write(&format!("{player}/face.xml"), xml.as_bytes());
    }
}

// TC-XML-01
#[test]
fn a_member_s_xml_is_written_back_with_its_own_names_and_the_common_model_s_packed_one() {
    let sandbox = Sandbox::new("user_xml_compiled");
    let export = "co Midcup Xml";
    let xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<config>\n\
        \t<model level=\"0\" type=\"face_neck\" path=\"./face_high.model\" material=\"./face_high.mtl\"/>\n\
        \t<model level=\"1\" type=\"cape\" glow=\"1\" path=\"./hat_parts.model\" material=\"./hat_parts.mtl\"/>\n\
        \t<model level=\"0\" type=\"parts\" path=\"team_common_folder/legs.model\" material=\"team_common_folder/legs.mtl\"/>\n\
        </config>\n";
    write_face_and_hat(&sandbox, export, Some(xml));
    let common = format!("exports/{export}/Common");
    sandbox.write(&format!("{common}/legs.model"), &card_model());
    sandbox.write(&format!("{common}/legs.mtl"), &materials_naming("studs"));
    sandbox.write(&format!("{common}/studs.dds"), &small_dds());

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            "Warning xml_attribute_unknown [Keep] at Players/05 - A (entry=2, attribute=glow)",
            "Warning xml_type_unknown [Keep] at Players/05 - A (type=cape)",
            "Info xml_level_lod [Keep] at Players/05 - A (level=1)",
            IDENTIFIED,
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let face = slot_05_face(&entries);
    assert_eq!(
        names(&face),
        [
            "face.xml",
            "face_high.model",
            "face_high.mtl",
            "hat_parts.model",
            "hat_parts.mtl",
        ]
    );
    assert_eq!(face["face_high.model"], card_model());
    assert_eq!(face["hat_parts.model"], card_model());
    let expected = format!(
        "{HEAD}   \
         <model level=\"0\" type=\"face_neck\" path=\"./face_high.model\" material=\"./face_high.mtl\" />\r\n   \
         <model level=\"1\" type=\"cape\" glow=\"1\" path=\"./hat_parts.model\" material=\"./hat_parts.mtl\" />\r\n   \
         <model level=\"0\" type=\"parts\" path=\"model/character/uniform/common/714/oral_legs_*.model\" material=\"model/character/uniform/common/714/legs.mtl\" />\r\n\
         {}",
        dif_tail(&template("face_diff.bin"))
    );
    assert_eq!(String::from_utf8_lossy(&face["face.xml"]), expected);
    for mtl in ["face_high.mtl", "hat_parts.mtl"] {
        assert_eq!(
            sampler_paths(&face[mtl]),
            [format!("{SLOT_05_HOME}skin.dds")],
            "{mtl}"
        );
    }
    let common = common_output(&entries);
    let common_names: Vec<&str> = common.keys().copied().collect();
    assert_eq!(
        common_names,
        ["legs.mtl", "oral_legs_win32.model", "studs.dds"]
    );
}

// TC-CMN-18
#[test]
fn a_member_s_xml_names_a_common_subfolder_s_files_which_pack_at_their_own_path() {
    let sandbox = Sandbox::new("user_xml_common_subfolder");
    let export = "co Midcup Xml";
    let refkit = "model/character/uniform/common/team_folder/refkit/";
    let thigh =
        format!(r#"type="parts" path="{refkit}oral_thigh_*.model" material="{refkit}refkit.mtl""#);
    write_folder(
        &sandbox,
        export,
        "05 - A",
        "face_high",
        &face_xml(&[FACE_HIGH, &thigh]),
    );
    let common = format!("exports/{export}/Common/refkit");
    sandbox.write(&format!("{common}/oral_thigh_win32.model"), &card_model());
    sandbox.write(&format!("{common}/refkit.mtl"), &card_materials());
    sandbox.write(&format!("{common}/skin.dds"), &small_dds());

    let entries = compile_pes17(
        &sandbox,
        export,
        &[IDENTIFIED, "Info team_colors_missing [Keep] ()"],
    );

    let common_714 = "common/character1/model/character/uniform/common/714/";
    let common: Vec<&str> = entries_under(&entries, common_714).into_keys().collect();
    assert_eq!(
        common,
        [
            "05 - A/skin.dds",
            "refkit/oral_thigh_win32.model",
            "refkit/refkit.mtl",
            "refkit/skin.dds",
        ]
    );
    // The subfolder's `.mtl` names its own texture at the subfolder's path.
    let refkit_mtl = &entries[&format!("{common_714}refkit/refkit.mtl")];
    assert_eq!(
        sampler_paths(refkit_mtl),
        ["model/character/uniform/common/714/refkit/skin.dds"]
    );
    let face = slot_05_face(&entries);
    let expected = format!(
        "{HEAD}   \
         <model level=\"0\" type=\"face_neck\" path=\"./face_high.model\" material=\"./face_high.mtl\" />\r\n   \
         <model level=\"0\" type=\"parts\" path=\"model/character/uniform/common/714/refkit/oral_thigh_*.model\" material=\"model/character/uniform/common/714/refkit/refkit.mtl\" />\r\n\
         {}",
        dif_tail(&template("face_diff.bin"))
    );
    assert_eq!(String::from_utf8_lossy(&face["face.xml"]), expected);
}

/// The entry naming slot 05's subfolder model and its `.mtl` by their paths
/// (`write_subfolder_xml`).
const SUBFOLDER_X: &str =
    r#"type="parts" path="./jessie/body/oral_x_*.model" material="./jessie/body/x.mtl""#;

/// Writes slot 05 of `export` holding `face_high.model`, its `.mtl` and `skin.dds`, the card
/// head's model as `jessie/body/oral_x_win32.model` with `jessie/body/x.mtl` naming `skin.dds`,
/// and a `face.xml` listing both models by their paths (`FACE_HIGH`, `SUBFOLDER_X`).
fn write_subfolder_xml(sandbox: &Sandbox, export: &str) {
    write_folder(
        sandbox,
        export,
        "05 - A",
        "face_high",
        &face_xml(&[FACE_HIGH, SUBFOLDER_X]),
    );
    let body = format!("exports/{export}/Players/05 - A/jessie/body");
    sandbox.write(&format!("{body}/oral_x_win32.model"), &card_model());
    sandbox.write(&format!("{body}/x.mtl"), &card_materials());
}

// TC-XML-03
#[test]
fn a_local_reference_into_a_subfolder_is_checked_and_a_bare_name_names_the_folder_s_own_file() {
    let sandbox = Sandbox::new("user_xml_subfolder_check");
    let export = "co Midcup Xml";
    write_subfolder_xml(&sandbox, export);
    // `./x_*.model` names a file directly in the folder, which holds none: the subfolder's
    // `x_win32.model` is not it.
    write_folder(
        &sandbox,
        export,
        "07 - B",
        "face_high",
        &face_xml(&[FACE_HIGH, r#"type="parts" path="./x_*.model""#]),
    );
    let body = format!("exports/{export}/Players/07 - B/jessie/body");
    sandbox.write(&format!("{body}/x_win32.model"), &card_model());
    sandbox.write(&format!("{body}/x.mtl"), &card_materials());

    let run = sandbox.run(&pes17(&sandbox), &["check"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Error xml_model_not_found [DropFolder] at Players/07 - B (attribute=path, value=./x_*.model)",
            "Warning xml_model_unlisted [Keep] at Players/07 - B (file=jessie/body/x_win32.model)",
            IDENTIFIED,
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-XML-01
#[test]
fn a_member_s_xml_naming_a_subfolder_s_files_packs_them_at_their_paths() {
    let sandbox = Sandbox::new("user_xml_subfolder_compiled");
    let export = "co Midcup Xml";
    write_subfolder_xml(&sandbox, export);

    let entries = compile_pes17(
        &sandbox,
        export,
        &[IDENTIFIED, "Info team_colors_missing [Keep] ()"],
    );

    let face = slot_05_face(&entries);
    assert_eq!(
        names(&face),
        [
            "face.xml",
            "face_high.model",
            "face_high.mtl",
            "jessie/body/oral_x_win32.model",
            "jessie/body/x.mtl",
        ]
    );
    assert_eq!(face["jessie/body/oral_x_win32.model"], card_model());
    let expected = format!(
        "{HEAD}   \
         <model level=\"0\" type=\"face_neck\" path=\"./face_high.model\" material=\"./face_high.mtl\" />\r\n   \
         <model level=\"0\" type=\"parts\" path=\"./jessie/body/oral_x_*.model\" material=\"./jessie/body/x.mtl\" />\r\n\
         {}",
        dif_tail(&template("face_diff.bin"))
    );
    assert_eq!(String::from_utf8_lossy(&face["face.xml"]), expected);
    // Nearest first from `jessie/body/`, the root's `skin.dds`.
    assert_eq!(
        sampler_paths(&face["jessie/body/x.mtl"]),
        [format!("{SLOT_05_HOME}skin.dds")]
    );
}

// TC-XML-02
#[test]
fn without_its_xml_the_same_folder_s_models_are_typed_by_their_names() {
    let sandbox = Sandbox::new("user_xml_removed");
    let export = "co Midcup Xml";
    write_face_and_hat(&sandbox, export, None);

    let entries = compile_pes17(&sandbox, export, &CLEAN);

    let face = slot_05_face(&entries);
    assert_eq!(
        ordered_entries(&face["face.xml"]),
        [
            (
                "face_neck".to_owned(),
                "./oral_face_high_*.model".to_owned(),
                "./face_high.mtl".to_owned()
            ),
            (
                "parts".to_owned(),
                "./oral_hat_parts_*.model".to_owned(),
                "./hat_parts.mtl".to_owned()
            ),
        ]
    );
}

/// Writes `Faces/Round` of `export` holding the card head's model as `face_high.model`, its
/// `.mtl` naming `skin.dds`, that texture, and the `face.xml` `xml` when given, and a link to
/// it in each of the player folders `folders`.
fn write_round(sandbox: &Sandbox, export: &str, xml: Option<&str>, folders: &[&str]) {
    let round = format!("exports/{export}/Faces/Round");
    sandbox.write(&format!("{round}/face_high.model"), &card_model());
    sandbox.write(&format!("{round}/face_high.mtl"), &card_materials());
    sandbox.write(&format!("{round}/skin.dds"), &small_dds());
    if let Some(xml) = xml {
        sandbox.write(&format!("{round}/face.xml"), xml.as_bytes());
    }
    for folder in folders {
        sandbox.write(
            &format!("exports/{export}/Players/{folder}/Round.face"),
            b"",
        );
    }
}

/// The shared face's `face_high` entry, typed `parts`: with no `face_neck` entry, the dummy
/// shows where the appended entries go.
const ROUND_FACE_HIGH: &str = r#"type="parts" path="./face_high.model" material="./face_high.mtl""#;

// TC-XML-14
#[test]
fn a_shared_face_s_xml_is_its_linking_face_s_with_his_own_models_appended() {
    let sandbox = Sandbox::new("user_xml_shared_face");
    let export = "co Midcup Xml";
    let xml = face_xml(&[ROUND_FACE_HIGH]);
    write_round(&sandbox, export, Some(&xml), &["05 - A"]);
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/hair.model"), &card_model());
    sandbox.write(&format!("{player}/hair.mtl"), &card_materials());

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            IDENTIFIED,
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
            FACE_NECK_ADDED,
        ],
    );

    // The shared model under the name the xml gives it, his own under its generated one.
    let face = slot_05_face(&entries);
    assert_eq!(
        names(&face),
        [
            "dummy.mtl",
            "face.xml",
            "face_high.model",
            "face_high.mtl",
            "hair.mtl",
            "oral_dummy_win32.model",
            "oral_hair_win32.model",
        ]
    );
    assert_eq!(face["face_high.model"], card_model());
    assert_eq!(
        ordered_entries(&face["face.xml"]),
        owned_entries(&[
            ("parts", "./face_high.model", "./face_high.mtl"),
            ("parts", "./oral_hair_*.model", "./hair.mtl"),
            ("face_neck", "./oral_dummy_*.model", "./dummy.mtl"),
        ])
    );
    for mtl in ["face_high.mtl", "hair.mtl"] {
        assert_eq!(
            sampler_paths(&face[mtl]),
            [format!("{SLOT_05_HOME}skin.dds")],
            "{mtl}"
        );
    }
}

// TC-XML-14
#[test]
fn his_subfolder_s_model_of_a_name_the_shared_xml_names_is_appended_at_its_path() {
    let sandbox = Sandbox::new("user_xml_shared_face_subfolder");
    let export = "co Midcup Xml";
    let xml = face_xml(&[ROUND_FACE_HIGH]);
    write_round(&sandbox, export, Some(&xml), &["05 - A"]);
    // `./face_high.model` names a file directly in his folder, then the shared face's: his
    // `jessie/face_high.model` is neither, so it gets its generated entry.
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/jessie/face_high.model"), &card_model());
    sandbox.write(&format!("{player}/jessie/face_high.mtl"), &card_materials());

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            IDENTIFIED,
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
        ],
    );

    let face = slot_05_face(&entries);
    assert_eq!(
        names(&face),
        [
            "face.xml",
            "face_high.model",
            "face_high.mtl",
            "jessie/face_high.mtl",
            "jessie/oral_face_high_win32.model",
        ]
    );
    assert_eq!(
        ordered_entries(&face["face.xml"]),
        owned_entries(&[
            ("parts", "./face_high.model", "./face_high.mtl"),
            (
                "face_neck",
                "./jessie/oral_face_high_*.model",
                "./jessie/face_high.mtl"
            ),
        ])
    );
}

// TC-XML-14
#[test]
fn a_file_of_his_replacing_one_the_shared_xml_names_is_that_entry_s_file_not_appended() {
    let sandbox = Sandbox::new("user_xml_shared_face_replaced");
    let export = "co Midcup Xml";
    let xml = face_xml(&[FACE_HIGH]);
    write_round(&sandbox, export, Some(&xml), &["05 - A"]);
    // His own `face_high` pair, its `.mtl` naming his own texture.
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.model"), &card_model());
    sandbox.write(&format!("{player}/face_high.mtl"), &materials_naming("his"));
    sandbox.write(&format!("{player}/his.dds"), &small_dds());

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            IDENTIFIED,
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
        ],
    );

    let face = slot_05_face(&entries);
    assert_eq!(
        names(&face),
        ["face.xml", "face_high.model", "face_high.mtl"]
    );
    assert_eq!(
        ordered_entries(&face["face.xml"]),
        owned_entries(&[("face_neck", "./face_high.model", "./face_high.mtl")])
    );
    assert_eq!(
        sampler_paths(&face["face_high.mtl"]),
        [format!("{SLOT_05_HOME}his.dds")]
    );
}

// TC-XML-15
#[test]
fn a_player_s_own_xml_beside_a_face_link_drops_his_folder_even_under_pass_through() {
    let sandbox = Sandbox::new("user_xml_shared_face_conflict");
    let export = "co Midcup Xml";
    // The shared folder holds no xml: the player's own is the conflict all the same.
    write_round(&sandbox, export, None, &["05 - A"]);
    sandbox.write(
        &format!("exports/{export}/Players/05 - A/face.xml"),
        face_xml(&[r#"type="face_neck" path="./face_high.model""#]).as_bytes(),
    );
    // Another face, so the run writes a CPK the dropped one is missing from.
    let other = format!("exports/{export}/Players/07 - B");
    sandbox.write(&format!("{other}/face_high.model"), &card_model());
    sandbox.write(&format!("{other}/face_high.mtl"), &card_materials());
    sandbox.write(&format!("{other}/skin.dds"), &small_dds());
    let conflict = "Error xml_shared_face_conflict [DropFolder] at Players/05 - A (file=face.xml, link=Faces/Round)";
    // With its one linking player dropped, nothing compiles the shared folder.
    let orphaned = "Warning shared_folder_orphaned [DropFolder] at Faces/Round ()";

    let run = sandbox.run(&pes17(&sandbox), &["check"]);
    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [conflict, orphaned, IDENTIFIED],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");

    for settings in [
        pes17(&sandbox),
        format!("{}[team-compiler]\npass_through = true\n", pes17(&sandbox)),
    ] {
        let run = sandbox.run(&settings, &["compile", "--no-deploy"]);

        let lines = run.messages();
        assert_eq!(
            findings_of(&lines, export),
            [
                conflict,
                orphaned,
                IDENTIFIED,
                "Info team_colors_missing [Keep] ()",
            ],
            "{lines:#?}"
        );
        assert_eq!(run.exit_code(), 1, "{lines:#?}");
        let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
        assert!(!entries.contains_key(&face_cpk(5)), "{:?}", entries.keys());
        assert!(entries.contains_key(&face_cpk(7)), "{:?}", entries.keys());
    }
}

// TC-XML-03
#[test]
fn a_shared_face_s_xml_naming_an_absent_model_drops_the_folder_and_its_linking_player() {
    let sandbox = Sandbox::new("user_xml_shared_face_not_found");
    let export = "co Midcup Xml";
    let xml = face_xml(&[FACE_HIGH, r#"type="parts" path="./missing.model""#]);
    write_round(&sandbox, export, Some(&xml), &["05 - A"]);

    let run = sandbox.run(&pes17(&sandbox), &["check"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Error xml_model_not_found [DropFolder] at Faces/Round (attribute=path, value=./missing.model)",
            "Error link_target_dropped [DropFolder] at Players/05 - A (link=Round.face, target=Faces/Round, finding=xml_model_not_found)",
            IDENTIFIED,
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-XML-14
#[test]
fn his_own_face_diff_beside_a_face_link_wins_over_the_shared_xml_s_dif() {
    let sandbox = Sandbox::new("user_xml_shared_face_dif");
    let export = "co Midcup Xml";
    let dif_text = String::from_utf8(face_diff_fixture("dif.xml")).unwrap();
    let start = dif_text.find("<dif>").unwrap() + "<dif>".len();
    let end = dif_text.find("</dif>").unwrap();
    let base64 = dif_text[start..end].trim();
    let xml =
        format!("<config>\n   <model level=\"0\" {FACE_HIGH}/>\n<dif>{base64}</dif>\n</config>\n");
    write_round(&sandbox, export, Some(&xml), &["05 - A", "07 - B"]);
    // Slot 05's own face diff, another real one than the `<dif>`'s.
    let own = face_diff_fixture("plain.xml");
    sandbox.write(
        &format!("exports/{export}/Players/05 - A/face_diff.xml"),
        &own,
    );

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            IDENTIFIED,
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
            "Info link_combined [Keep] at Players/07 - B (link=Round.face)",
        ],
    );

    // Both faces are the shared xml's; his file is his face's diff, the `<dif>` the other's.
    let shared_entries = owned_entries(&[("face_neck", "./face_high.model", "./face_high.mtl")]);
    let slot_05 = slot_face(&entries, 5);
    assert_eq!(ordered_entries(&slot_05["face.xml"]), shared_entries);
    assert_eq!(
        decoded_dif(&slot_05["face.xml"]),
        face_diff_fixture("plain.bin")
    );
    let slot_07 = slot_face(&entries, 7);
    assert_eq!(ordered_entries(&slot_07["face.xml"]), shared_entries);
    assert_eq!(
        decoded_dif(&slot_07["face.xml"]),
        face_diff_fixture("dif.bin")
    );
}

/// The `face.xml` naming `./hat.model` typed `parts` with `./hat.mtl`, and no `face_neck`.
fn hat_xml() -> String {
    face_xml(&[r#"type="parts" path="./hat.model" material="./hat.mtl""#])
}

/// Each (type, path, material) as `ordered_entries` gives it.
fn owned_entries(entries: &[(&str, &str, &str)]) -> Vec<(String, String, String)> {
    entries
        .iter()
        .map(|(xml_type, path, material)| {
            (
                (*xml_type).to_owned(),
                (*path).to_owned(),
                (*material).to_owned(),
            )
        })
        .collect()
}

/// The finding of slot 05's face given the dummy.
const FACE_NECK_ADDED: &str = "Info xml_face_neck_added [Keep] at Players/05 - A ()";

// TC-XML-04
#[test]
fn a_member_s_xml_with_no_face_neck_entry_gets_the_dummy_after_its_entries() {
    let sandbox = Sandbox::new("user_xml_face_neck");
    let export = "co Midcup Xml";
    write_folder(&sandbox, export, "05 - A", "hat", &hat_xml());

    let entries = compile_pes17(&sandbox, export, &[&CLEAN[..], &[FACE_NECK_ADDED]].concat());

    let face = slot_05_face(&entries);
    assert_eq!(
        names(&face),
        [
            "dummy.mtl",
            "face.xml",
            "hat.model",
            "hat.mtl",
            "oral_dummy_win32.model"
        ]
    );
    assert_eq!(face["oral_dummy_win32.model"], template("dummy.model"));
    assert_eq!(
        ordered_entries(&face["face.xml"]),
        owned_entries(&[
            ("parts", "./hat.model", "./hat.mtl"),
            ("face_neck", "./oral_dummy_*.model", "./dummy.mtl"),
        ])
    );
}

// TC-XML-05
#[test]
fn a_model_the_member_s_xml_does_not_list_is_reported_and_not_packed() {
    let sandbox = Sandbox::new("user_xml_unlisted");
    let export = "co Midcup Xml";
    write_folder(&sandbox, export, "05 - A", "hat", &hat_xml());
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/hat2.model"), &card_model());
    sandbox.write(&format!("{player}/hat2.mtl"), &card_materials());

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            "Warning xml_model_unlisted [Keep] at Players/05 - A (file=hat2.model)",
            IDENTIFIED,
            "Info team_colors_missing [Keep] ()",
            FACE_NECK_ADDED,
        ],
    );

    let face = slot_05_face(&entries);
    assert!(
        face.keys().all(|name| !name.contains("hat2")),
        "{:?}",
        face.keys()
    );
    assert!(face.contains_key("hat.model"), "{:?}", face.keys());
}

// TC-XML-01
#[test]
fn an_entry_without_material_is_written_without_one_whatever_the_folder_s_mtl_files() {
    let sandbox = Sandbox::new("user_xml_no_material");
    let export = "co Midcup Xml";
    let player = format!("exports/{export}/Players/05 - A");
    // No `.mtl` anywhere: the name search would find none for `hat.model`.
    sandbox.write(&format!("{player}/hat.model"), &card_model());
    sandbox.write(
        &format!("{player}/face.xml"),
        face_xml(&[r#"type="face_neck" path="./hat.model""#]).as_bytes(),
    );

    let entries = compile_pes17(&sandbox, export, &CLEAN);

    let face = slot_05_face(&entries);
    assert_eq!(names(&face), ["face.xml", "hat.model"]);
    assert_eq!(
        String::from_utf8_lossy(&face["face.xml"]),
        format!(
            "{HEAD}   <model level=\"0\" type=\"face_neck\" path=\"./hat.model\" />\r\n{}",
            dif_tail(&template("face_diff.bin"))
        )
    );
}

// TC-XML-16
#[test]
fn a_kit_reference_packs_every_variant_of_its_model_and_mtl_sets_under_their_own_names() {
    let sandbox = Sandbox::new("user_xml_kit_set");
    let export = "co Midcup Xml";
    let player = format!("exports/{export}/Players/05 - A");
    write_folder(
        &sandbox,
        export,
        "05 - A",
        "face_high",
        &face_xml(&[
            FACE_HIGH,
            r#"type="parts" path="./pants_kitN.model" material="./pants_kitN.mtl""#,
        ]),
    );
    for kit in [1, 2] {
        sandbox.write(&format!("{player}/pants_kit{kit}.model"), &card_model());
        sandbox.write(&format!("{player}/pants_kit{kit}.mtl"), &card_materials());
    }

    let entries = compile_pes17(&sandbox, export, &CLEAN);

    let face = slot_05_face(&entries);
    assert_eq!(
        names(&face),
        [
            "face.xml",
            "face_high.model",
            "face_high.mtl",
            "pants_kit1.model",
            "pants_kit1.mtl",
            "pants_kit2.model",
            "pants_kit2.mtl",
        ]
    );
    assert_eq!(
        sampler_paths(&face["pants_kit2.mtl"]),
        [format!("{SLOT_05_HOME}skin.dds")]
    );
    // The game respells the entry for the kit picked.
    assert_eq!(
        ordered_entries(&face["face.xml"]),
        owned_entries(&[
            ("face_neck", "./face_high.model", "./face_high.mtl"),
            ("parts", "./pants_kitN.model", "./pants_kitN.mtl"),
        ])
    );
}

/// Compiles slot 05 holding `face_high` and the per-kit set `pants_kit1` and `pants_kit2`
/// (two different models, their `.mtl` files naming different textures), with the `face.xml`
/// naming `face_high` and the `<model>` attributes `pants`, against the kit folders `Kits/p1`
/// to `Kits/p4`; asserts the set is completed for kits 3 and 4, one finding each, its model
/// and `.mtl` copied from `pants_kit1`, and the xml written as the member wrote it.
fn assert_a_kit_reference_completes_its_model_and_mtl_sets(name: &str, pants: &str) {
    let sandbox = Sandbox::new(name);
    let export = "co Midcup Xml";
    let player = format!("exports/{export}/Players/05 - A");
    write_folder(
        &sandbox,
        export,
        "05 - A",
        "face_high",
        &face_xml(&[FACE_HIGH, pants]),
    );
    sandbox.write(&format!("{player}/pants_kit1.model"), &card_model());
    sandbox.write(
        &format!("{player}/pants_kit2.model"),
        &pre_fox_fixture("cardhead_doublesided_face_high.model"),
    );
    sandbox.write(&format!("{player}/pants_kit1.mtl"), &card_materials());
    sandbox.write(
        &format!("{player}/pants_kit2.mtl"),
        &materials_naming("his"),
    );
    sandbox.write(&format!("{player}/his.dds"), &small_dds());
    for kit in 1..=4 {
        sandbox.write(
            &format!("exports/{export}/Kits/p{kit}/kit.dds"),
            &tracer_kit(),
        );
    }

    let generated =
        (1..=4).map(|kit| format!("Info kit_config_generated [Keep] at Kits/p{kit} ()"));
    let missing = [3, 4].map(|kit| {
        format!(
            "Warning kit_variant_missing [Keep] at Players/05 - A (model=pants_kitN, kit={kit}, copied=pants_kit1)"
        )
    });
    let expected: Vec<String> = CLEAN
        .iter()
        .map(|line| (*line).to_owned())
        .chain(missing)
        .chain(generated)
        .collect();
    let entries = compile_pes17(
        &sandbox,
        export,
        &expected.iter().map(String::as_str).collect::<Vec<_>>(),
    );

    let face = slot_05_face(&entries);
    assert_eq!(
        names(&face),
        [
            "face.xml",
            "face_high.model",
            "face_high.mtl",
            "pants_kit1.model",
            "pants_kit1.mtl",
            "pants_kit2.model",
            "pants_kit2.mtl",
            "pants_kit3.model",
            "pants_kit3.mtl",
            "pants_kit4.model",
            "pants_kit4.mtl",
        ]
    );
    for kit in [3, 4] {
        assert_eq!(
            face[&format!("pants_kit{kit}.model")],
            card_model(),
            "{kit}"
        );
        assert_eq!(
            face[&format!("pants_kit{kit}.mtl")],
            face["pants_kit1.mtl"],
            "{kit}"
        );
    }
    assert_eq!(
        sampler_paths(&face["pants_kit3.mtl"]),
        [format!("{SLOT_05_HOME}skin.dds")]
    );
    assert_eq!(
        ordered_entries(&face["face.xml"]),
        owned_entries(&[
            ("face_neck", "./face_high.model", "./face_high.mtl"),
            ("parts", "./pants_kitN.model", "./pants_kitN.mtl"),
        ])
    );
}

// TC-XML-16
#[test]
fn a_kit_reference_into_a_subfolder_names_the_set_there_and_completes_it_at_its_path() {
    let sandbox = Sandbox::new("user_xml_kit_set_subfolder");
    let export = "co Midcup Xml";
    let player = format!("exports/{export}/Players/05 - A");
    write_folder(
        &sandbox,
        export,
        "05 - A",
        "face_high",
        &face_xml(&[
            FACE_HIGH,
            r#"type="parts" path="./jessie/pants_kitN.model" material="./jessie/pants_kitN.mtl""#,
        ]),
    );
    for kit in [1, 2] {
        sandbox.write(
            &format!("{player}/jessie/pants_kit{kit}.model"),
            &card_model(),
        );
        sandbox.write(
            &format!("{player}/jessie/pants_kit{kit}.mtl"),
            &card_materials(),
        );
    }
    // A variant of the root's set, which the reference into `jessie/` does not name.
    sandbox.write(&format!("{player}/pants_kit3.model"), &card_model());
    for kit in 1..=3 {
        sandbox.write(
            &format!("exports/{export}/Kits/p{kit}/kit.dds"),
            &tracer_kit(),
        );
    }

    let generated =
        (1..=3).map(|kit| format!("Info kit_config_generated [Keep] at Kits/p{kit} ()"));
    let expected: Vec<String> = [
        "Warning xml_model_unlisted [Keep] at Players/05 - A (file=pants_kit3.model)".to_owned(),
    ]
    .into_iter()
    .chain(CLEAN.iter().map(|line| (*line).to_owned()))
    .chain([
        "Warning kit_variant_missing [Keep] at Players/05 - A (model=jessie/pants_kitN, kit=3, copied=jessie/pants_kit1)".to_owned(),
    ])
    .chain(generated)
    .collect();
    let entries = compile_pes17(
        &sandbox,
        export,
        &expected.iter().map(String::as_str).collect::<Vec<_>>(),
    );

    let face = slot_05_face(&entries);
    assert_eq!(
        names(&face),
        [
            "face.xml",
            "face_high.model",
            "face_high.mtl",
            "jessie/pants_kit1.model",
            "jessie/pants_kit1.mtl",
            "jessie/pants_kit2.model",
            "jessie/pants_kit2.mtl",
            "jessie/pants_kit3.model",
            "jessie/pants_kit3.mtl",
        ]
    );
    assert_eq!(
        ordered_entries(&face["face.xml"]),
        owned_entries(&[
            ("face_neck", "./face_high.model", "./face_high.mtl"),
            (
                "parts",
                "./jessie/pants_kitN.model",
                "./jessie/pants_kitN.mtl"
            ),
        ])
    );
}

// TC-XML-16
#[test]
fn a_kit_reference_completes_its_model_and_mtl_sets_against_the_export_s_kits() {
    assert_a_kit_reference_completes_its_model_and_mtl_sets(
        "user_xml_kit_set_completed",
        r#"type="parts" path="./pants_kitN.model" material="./pants_kitN.mtl""#,
    );
}

// The `.mtl` set, completed first, reports nothing: the finding is the model set's.
// TC-XML-16
#[test]
fn a_kit_reference_naming_its_mtl_first_reports_each_number_once() {
    assert_a_kit_reference_completes_its_model_and_mtl_sets(
        "user_xml_kit_set_completed_mtl_first",
        r#"type="parts" material="./pants_kitN.mtl" path="./pants_kitN.model""#,
    );
}

// The model set alone reports its missing number when its entry names one shared `.mtl`,
// which serves every number as it is and is not copied.
// TC-XML-16
#[test]
fn a_kit_reference_naming_one_shared_mtl_reports_its_model_set_and_copies_no_mtl() {
    let sandbox = Sandbox::new("user_xml_kit_set_one_mtl");
    let export = "co Midcup Xml";
    let player = format!("exports/{export}/Players/05 - A");
    write_folder(
        &sandbox,
        export,
        "05 - A",
        "face_high",
        &face_xml(&[
            FACE_HIGH,
            r#"type="parts" path="./pants_kitN.model" material="./pants.mtl""#,
        ]),
    );
    sandbox.write(&format!("{player}/pants_kit1.model"), &card_model());
    sandbox.write(&format!("{player}/pants.mtl"), &card_materials());
    for kit in [1, 2] {
        sandbox.write(
            &format!("exports/{export}/Kits/p{kit}/kit.dds"),
            &tracer_kit(),
        );
    }

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            CLEAN[0],
            CLEAN[1],
            "Warning kit_variant_missing [Keep] at Players/05 - A (model=pants_kitN, kit=2, copied=pants_kit1)",
            "Info kit_config_generated [Keep] at Kits/p1 ()",
            "Info kit_config_generated [Keep] at Kits/p2 ()",
        ],
    );

    let face = slot_05_face(&entries);
    assert_eq!(
        names(&face),
        [
            "face.xml",
            "face_high.model",
            "face_high.mtl",
            "pants.mtl",
            "pants_kit1.model",
            "pants_kit2.model",
        ]
    );
    assert_eq!(
        ordered_entries(&face["face.xml"]),
        owned_entries(&[
            ("face_neck", "./face_high.model", "./face_high.mtl"),
            ("parts", "./pants_kitN.model", "./pants.mtl"),
        ])
    );
}

// The player's own variants of the set the shared xml names are packed by its entry, not
// appended under generated ones: his lowest, which the reference resolves to, and his
// higher one, which only the set names.
// TC-XML-16
#[test]
fn a_shared_xml_s_kit_reference_names_the_linking_player_s_own_variants() {
    let sandbox = Sandbox::new("user_xml_shared_kit_set");
    let export = "co Midcup Xml";
    let xml = face_xml(&[
        FACE_HIGH,
        r#"type="parts" path="./pants_kitN.model" material="./pants_kitN.mtl""#,
    ]);
    write_round(&sandbox, export, Some(&xml), &["05 - A"]);
    let round = format!("exports/{export}/Faces/Round");
    sandbox.write(&format!("{round}/pants_kit1.model"), &card_model());
    sandbox.write(&format!("{round}/pants_kit1.mtl"), &card_materials());
    let player = format!("exports/{export}/Players/05 - A");
    let his = pre_fox_fixture("cardhead_doublesided_face_high.model");
    for kit in [2, 3] {
        sandbox.write(&format!("{player}/pants_kit{kit}.model"), &his);
        sandbox.write(&format!("{player}/pants_kit{kit}.mtl"), &card_materials());
    }

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            IDENTIFIED,
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
        ],
    );

    let face = slot_05_face(&entries);
    assert_eq!(
        names(&face),
        [
            "face.xml",
            "face_high.model",
            "face_high.mtl",
            "pants_kit1.model",
            "pants_kit1.mtl",
            "pants_kit2.model",
            "pants_kit2.mtl",
            "pants_kit3.model",
            "pants_kit3.mtl",
        ]
    );
    assert_eq!(face["pants_kit2.model"], his);
    assert_eq!(face["pants_kit3.model"], his);
    assert_eq!(
        ordered_entries(&face["face.xml"]),
        owned_entries(&[
            ("face_neck", "./face_high.model", "./face_high.mtl"),
            ("parts", "./pants_kitN.model", "./pants_kitN.mtl"),
        ])
    );
}

// The player's own `kitF` files are the set's fallback a shared xml's `kitN` reference is
// completed from: packed under their own names as the face's files already hold them, the
// missing numbers' copies read from them, and nothing of the face keeps a `kitF` name.
// TC-XML-16
#[test]
fn a_shared_xml_s_kit_reference_is_completed_from_the_player_s_kitf_files() {
    let sandbox = Sandbox::new("user_xml_shared_kit_fallback");
    let export = "co Midcup Xml";
    let xml = face_xml(&[
        FACE_HIGH,
        r#"type="parts" path="./pants_kitN.model" material="./pants_kitN.mtl""#,
    ]);
    write_round(&sandbox, export, Some(&xml), &["05 - A"]);
    // The shared face's own member of the set, which its xml names so `check` keeps it;
    // the player's `kitF` files are the fallback the completions are read from.
    let round = format!("exports/{export}/Faces/Round");
    sandbox.write(&format!("{round}/pants_kit1.model"), &card_model());
    sandbox.write(&format!("{round}/pants_kit1.mtl"), &card_materials());
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/pants_kit1.model"), &card_model());
    sandbox.write(&format!("{player}/pants_kit1.mtl"), &card_materials());
    // His own `kitF` pair, its `.mtl` naming another texture so its copies show their source.
    let fallback = pre_fox_fixture("cardhead_doublesided_face_high.model");
    sandbox.write(&format!("{player}/pants_kitF.model"), &fallback);
    sandbox.write(
        &format!("{player}/pants_kitF.mtl"),
        &materials_naming("his"),
    );
    sandbox.write(&format!("{player}/his.dds"), &small_dds());
    for kit in 1..=3 {
        sandbox.write(
            &format!("exports/{export}/Kits/p{kit}/kit.dds"),
            &tracer_kit(),
        );
    }

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            IDENTIFIED,
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
            "Info kit_config_generated [Keep] at Kits/p1 ()",
            "Info kit_config_generated [Keep] at Kits/p2 ()",
            "Info kit_config_generated [Keep] at Kits/p3 ()",
        ],
    );

    let face = slot_05_face(&entries);
    assert_eq!(
        names(&face),
        [
            "face.xml",
            "face_high.model",
            "face_high.mtl",
            "pants_kit1.model",
            "pants_kit1.mtl",
            "pants_kit2.model",
            "pants_kit2.mtl",
            "pants_kit3.model",
            "pants_kit3.mtl",
        ]
    );
    // Kits 2 and 3's copies are the `kitF` files', under the numbers' spellings.
    assert_eq!(face["pants_kit2.model"], fallback);
    assert_eq!(face["pants_kit3.model"], fallback);
    assert_eq!(
        sampler_paths(&face["pants_kit2.mtl"]),
        [format!("{SLOT_05_HOME}his.dds")]
    );
    assert_eq!(
        sampler_paths(&face["pants_kit1.mtl"]),
        [format!("{SLOT_05_HOME}skin.dds")]
    );
    assert_eq!(
        ordered_entries(&face["face.xml"]),
        owned_entries(&[
            ("face_neck", "./face_high.model", "./face_high.mtl"),
            ("parts", "./pants_kitN.model", "./pants_kitN.mtl"),
        ])
    );
}

// TC-XML-18
#[test]
fn the_team_s_common_is_written_team_folder_in_either_prefix_and_any_letter_case() {
    let sandbox = Sandbox::new("user_xml_team_folder");
    let export = "co Midcup Xml";
    // Slot 05's xml names `Common/legs.model` by the short form and, in any letter case, by
    // the long one: two entries for one file, which packs once.
    write_folder(
        &sandbox,
        export,
        "05 - A",
        "face_high",
        &face_xml(&[
            FACE_HIGH,
            r#"type="parts" path="team_common_folder/legs.model""#,
            r#"type="parts" path="model/character/uniform/common/Team_Folder/legs.model""#,
        ]),
    );
    // Slot 06's `face_high.mtl` names `hair`, `jessie/skin` and `fur` in the team's Common:
    // the first two `Common/` holds, `fur` nothing does.
    let player = format!("exports/{export}/Players/06 - B");
    sandbox.write(&format!("{player}/face_high.model"), &card_model());
    let mtl = String::from_utf8(pre_fox_fixture("cardhead_materials.mtl"))
        .unwrap()
        .replace("./texture.dds", "team_common_folder/hair.dds")
        .replacen(
            "    <state",
            "    <sampler name=\"NormalMap\" path=\"team_common_folder/jessie/skin.dds\" />\n    \
             <sampler name=\"SpecularMap\" path=\"team_common_folder/fur.dds\" />\n    <state",
            1,
        );
    sandbox.write(&format!("{player}/face_high.mtl"), mtl.as_bytes());
    let common = format!("exports/{export}/Common");
    sandbox.write(&format!("{common}/legs.model"), &card_model());
    sandbox.write(&format!("{common}/hair.dds"), &small_dds());
    sandbox.write(&format!("{common}/jessie/skin.dds"), &small_dds());

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            "Warning mtl_texture_not_found [Keep] at Players/06 - B (file=face_high.mtl, texture=team_common_folder/fur.dds, materials=card)",
            IDENTIFIED,
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let face = slot_face(&entries, 5);
    // Both entries name the packed model alike, `team_folder` written as the team's ID.
    let expected = format!(
        "{HEAD}   \
         <model level=\"0\" type=\"face_neck\" path=\"./face_high.model\" material=\"./face_high.mtl\" />\r\n   \
         <model level=\"0\" type=\"parts\" path=\"model/character/uniform/common/714/oral_legs_*.model\" />\r\n   \
         <model level=\"0\" type=\"parts\" path=\"model/character/uniform/common/714/oral_legs_*.model\" />\r\n\
         {}",
        dif_tail(&template("face_diff.bin"))
    );
    assert_eq!(String::from_utf8_lossy(&face["face.xml"]), expected);
    let face = slot_face(&entries, 6);
    assert_eq!(
        sampler_paths(&face["face_high.mtl"]),
        [
            "model/character/uniform/common/714/hair.dds",
            "model/character/uniform/common/714/jessie/skin.dds",
            "model/character/uniform/common/714/fur.dds",
        ]
    );
    // `fur`, pointed at the team's Common output though nothing packs it.
    let common = common_output(&entries);
    let names: Vec<&str> = common.keys().copied().collect();
    assert_eq!(names, ["hair.dds", "oral_legs_win32.model"]);
    assert!(
        entries
            .contains_key("common/character1/model/character/uniform/common/714/jessie/skin.dds")
    );
}

// TC-XML-19
#[test]
fn a_folder_other_than_team_folder_after_common_is_refused_on_every_target() {
    let sandbox = Sandbox::new("user_xml_common_invalid");
    let export = "co Midcup Xml";
    // Slot 05's xml names a model by its legacy three-character folder.
    write_folder(
        &sandbox,
        export,
        "05 - A",
        "face_high",
        &face_xml(&[
            FACE_HIGH,
            r#"type="parts" path="model/character/uniform/common/XXX/legs.model""#,
        ]),
    );
    // Slot 06's `.mtl` names a texture by a team ID, slot 07's by another name: the same
    // rule a `face.xml` entry is judged by.
    for (slot, path) in [
        ("06 - B", "common_folder/797/hair.dds"),
        ("07 - C", "model/character/uniform/common/o/env.dds"),
    ] {
        let player = format!("exports/{export}/Players/{slot}");
        sandbox.write(&format!("{player}/face_high.model"), &card_model());
        let mtl = String::from_utf8(pre_fox_fixture("cardhead_materials.mtl"))
            .unwrap()
            .replace("./texture.dds", path);
        sandbox.write(&format!("{player}/face_high.mtl"), mtl.as_bytes());
    }
    let common = format!("exports/{export}/Common");
    sandbox.write(&format!("{common}/legs.model"), &card_model());
    sandbox.write(&format!("{common}/hair.dds"), &small_dds());
    sandbox.write(&format!("{common}/env.dds"), &small_dds());

    let mtls = [
        "Error mtl_common_path_invalid [DropFolder] at Players/06 - B (file=face_high.mtl, attribute=path, value=common_folder/797/hair.dds)",
        "Error mtl_common_path_invalid [DropFolder] at Players/07 - C (file=face_high.mtl, attribute=path, value=model/character/uniform/common/o/env.dds)",
    ];
    let expected_17: Vec<&str> = [
        "Error xml_common_path_invalid [DropFolder] at Players/05 - A (attribute=path, value=model/character/uniform/common/XXX/legs.model)",
    ]
    .into_iter()
    .chain(mtls)
    .chain([IDENTIFIED])
    .collect();
    // A `face.xml` is ignored on Fox (`xml_ignored_fox`); the `.mtl` rule applies on
    // every target all the same.
    let expected_21: Vec<&str> = mtls
        .into_iter()
        .chain([IDENTIFIED])
        .chain(["Info xml_ignored_fox [Keep] at Players/05 - A (file=face.xml)"])
        .collect();
    for (version, settings, expected) in [
        (17, pes17(&sandbox), expected_17),
        (21, pes21_settings(&sandbox), expected_21),
    ] {
        let run = sandbox.run(&settings, &["check"]);

        let lines = run.messages();
        assert_eq!(
            findings_of(&lines, export),
            expected,
            "PES {version}: {lines:#?}"
        );
        assert_eq!(run.exit_code(), 1, "PES {version}");
    }
}

// TC-XML-21
#[test]
fn a_file_directly_below_common_is_the_game_s_own_written_in_the_long_form() {
    let sandbox = Sandbox::new("user_xml_game_common");
    let export = "co Midcup Xml";
    // `face_high.mtl` names the game's own `uni_pattern_nrm00.dds`, and the xml a
    // `uniform_sub.model` below `common_folder/`, which is no entry of the export's
    // `Common/`.
    write_folder(
        &sandbox,
        export,
        "05 - A",
        "face_high",
        &face_xml(&[
            FACE_HIGH,
            r#"type="parts" path="common_folder/uniform_sub.model""#,
        ]),
    );
    let player = format!("exports/{export}/Players/05 - A");
    let mtl = String::from_utf8(pre_fox_fixture("cardhead_materials.mtl"))
        .unwrap()
        .replace("./texture.dds", "common_folder/uni_pattern_nrm00.dds");
    sandbox.write(&format!("{player}/face_high.mtl"), mtl.as_bytes());

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            "Warning xml_path_unchecked [Keep] at Players/05 - A (attribute=path, value=common_folder/uniform_sub.model)",
            IDENTIFIED,
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let face = slot_05_face(&entries);
    assert_eq!(
        sampler_paths(&face["face_high.mtl"]),
        ["model/character/uniform/common/uni_pattern_nrm00.dds"]
    );
    // Written with the game's own long form, the only one the game reads.
    let expected = format!(
        "{HEAD}   \
         <model level=\"0\" type=\"face_neck\" path=\"./face_high.model\" material=\"./face_high.mtl\" />\r\n   \
         <model level=\"0\" type=\"parts\" path=\"model/character/uniform/common/uniform_sub.model\" />\r\n\
         {}",
        dif_tail(&template("face_diff.bin"))
    );
    assert_eq!(String::from_utf8_lossy(&face["face.xml"]), expected);
}

// TC-XML-22
#[test]
fn a_reference_starting_common_is_refused_as_the_archive_s_root_folder() {
    let sandbox = Sandbox::new("user_xml_common_root");
    let export = "co Midcup Xml";
    // `common/` is a CPK's root folder, which no reference names: slot 05's `face.xml`
    // spells the team's Common with it, slot 06's `face_high.mtl` does the same, and
    // slot 07's `.mtl` names the game's own file with it; and a folder after
    // `common_folder/` is refused too, `team_folder` included (slot 08's `.mtl`).
    // `Common/` holds the first two references' files.
    write_folder(
        &sandbox,
        export,
        "05 - A",
        "face_high",
        &face_xml(&[
            FACE_HIGH,
            r#"type="parts" path="common/team_folder/legs.model""#,
        ]),
    );
    let player = format!("exports/{export}/Players/06 - B");
    sandbox.write(&format!("{player}/face_high.model"), &card_model());
    let mtl = String::from_utf8(pre_fox_fixture("cardhead_materials.mtl"))
        .unwrap()
        .replace("./texture.dds", "common/team_folder/hair.dds");
    sandbox.write(&format!("{player}/face_high.mtl"), mtl.as_bytes());
    // An `.fmdl` folder's `.mtl`, which PES 18 to 2021 do not read: the refusal shows on
    // PES 15 to 2017 alone.
    let player = format!("exports/{export}/Players/07 - C");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    let mtl = String::from_utf8(pre_fox_fixture("cardhead_materials.mtl"))
        .unwrap()
        .replace("./texture.dds", "common/uni_pattern_nrm00.dds");
    sandbox.write(&format!("{player}/face_high.mtl"), mtl.as_bytes());
    let player = format!("exports/{export}/Players/08 - D");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    let mtl = String::from_utf8(pre_fox_fixture("cardhead_materials.mtl"))
        .unwrap()
        .replace("./texture.dds", "common_folder/team_folder/hair.dds");
    sandbox.write(&format!("{player}/face_high.mtl"), mtl.as_bytes());
    let common = format!("exports/{export}/Common");
    sandbox.write(&format!("{common}/legs.model"), &card_model());
    sandbox.write(&format!("{common}/hair.dds"), &small_dds());

    let run = sandbox.run(&pes17(&sandbox), &["check"]);
    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Error xml_common_path_invalid [DropFolder] at Players/05 - A (attribute=path, value=common/team_folder/legs.model)",
            "Error mtl_common_path_invalid [DropFolder] at Players/06 - B (file=face_high.mtl, attribute=path, value=common/team_folder/hair.dds)",
            "Error mtl_common_path_invalid [DropFolder] at Players/07 - C (file=face_high.mtl, attribute=path, value=common/uni_pattern_nrm00.dds)",
            "Error mtl_common_path_invalid [DropFolder] at Players/08 - D (file=face_high.mtl, attribute=path, value=common_folder/team_folder/hair.dds)",
            IDENTIFIED,
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);

    // On PES 21 the `face.xml` is ignored and only slot 06's `.mtl`, his `.model`'s, is
    // read: the rule judges it all the same.
    let run = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Error mtl_common_path_invalid [DropFolder] at Players/06 - B (file=face_high.mtl, attribute=path, value=common/team_folder/hair.dds)",
            IDENTIFIED,
            "Info xml_ignored_fox [Keep] at Players/05 - A (file=face.xml)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
}

/// The `<dif>` text of the `face.xml` `bytes` decoded, its line ends and indent left out.
fn decoded_dif(bytes: &[u8]) -> Vec<u8> {
    let text = std::str::from_utf8(bytes).unwrap();
    let start = text.find("<dif>").unwrap() + "<dif>".len();
    let end = text.find("</dif>").unwrap();
    let base64: String = text[start..end]
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    STANDARD.decode(base64).unwrap()
}

// the pre-Fox tracer's member xml
// TC-XML-01
#[test]
fn the_fumos_xml_compiles_with_its_references_and_its_dif_as_written() {
    let sandbox = Sandbox::new("user_xml_fumos");
    let export = "co Midcup Fumos";
    let fumos = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tracer_prefox/old/jp Tracer/Faces/XXX20 - Fumos/face.xml"),
    )
    .unwrap();
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face.xml"), &fumos);
    let models = [
        "face_high_win32.model",
        "oral_boots_win32.model",
        "oral_glove_l_win32.model",
        "oral_glove_r_win32.model",
    ];
    let materials = ["boots.mtl", "face.mtl", "glove_l.mtl", "glove_r.mtl"];
    for model in models {
        sandbox.write(&format!("{player}/{model}"), &card_model());
    }
    for mtl in materials {
        sandbox.write(&format!("{player}/{mtl}"), &card_materials());
    }
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());

    let entries = compile_pes17(
        &sandbox,
        export,
        &[
            "Warning xml_type_unknown [Keep] at Players/05 - A (type=boots)",
            IDENTIFIED,
            "Info team_colors_missing [Keep] ()",
        ],
    );

    let face = slot_05_face(&entries);
    let mut expected: Vec<&str> = models.iter().chain(&materials).copied().collect();
    expected.push("face.xml");
    expected.sort_unstable();
    assert_eq!(names(&face), expected);
    for model in models {
        assert_eq!(face[model], card_model(), "{model}");
    }
    let xml = &face["face.xml"];
    assert_eq!(
        ordered_entries(xml),
        owned_entries(&[
            ("face_neck", "./face_high_*.model", "./face.mtl"),
            ("gloveL", "./oral_glove_l_*.model", "./glove_l.mtl"),
            ("gloveR", "./oral_glove_r_*.model", "./glove_r.mtl"),
            ("boots", "./oral_boots_*.model", "./boots.mtl"),
        ])
    );
    assert_eq!(decoded_dif(xml), decoded_dif(&fumos));
    assert!(xml.ends_with(b"\r\n</dif>\r\n</config>"));
}

// TC-MOD-23
#[test]
fn on_pes_15_a_member_s_uniform_entry_is_written_uniform_sub_and_named_by_its_path() {
    let sandbox = Sandbox::new("user_xml_uniform");
    let export = "co Midcup Xml";
    write_folder(
        &sandbox,
        export,
        "05 - A",
        "body",
        &face_xml(&[r#"type="uniform" path="./body.model" material="./body.mtl""#]),
    );

    let run = sandbox.run(&pes15(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            IDENTIFIED,
            "Info team_colors_missing [Keep] ()",
            "Info xml_uniform_pes15 [Keep] at Players/05 - A (path=./body.model)",
            FACE_NECK_ADDED,
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let face = slot_05_face(&entries);
    assert_eq!(
        ordered_entries(&face["face.xml"]),
        owned_entries(&[
            ("uniform_sub", "./body.model", "./body.mtl"),
            ("face_neck", "./oral_dummy_*.model", "./dummy.mtl"),
        ])
    );
}
