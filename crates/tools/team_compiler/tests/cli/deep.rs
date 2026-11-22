//! The deep pass: what only a file's contents show, found by `check` and by `compile` alike,
//! on folder and archive exports, dropping and cascading as the structure pass's findings do.

use std::fs;
use std::path::Path;

use fmdl::{FmdlFile, Model};
use pes_version::PesVersion;

use crate::common::Sandbox;
use crate::compile::{
    compiled_kits, compiled_players, cpk_entries, pass_through_settings, pes_settings,
    pes21_settings, tracer_kit, tracer_player_file,
};
use crate::models::face_diff_fixture;
use crate::prefox_faces::{card_materials, card_model, face_cpk, small_dds};
use crate::{TEAM_COLORS_MISSING, command_args, findings_of};

/// The bytes of `tests/fixtures/deep/<name>`.
fn deep_fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/deep")
            .join(name),
    )
    .unwrap()
}

/// `vertex_too_far_from_origin` on slot 05's folder, as both commands report it.
const FAR_STRIKER: &str = "Error vertex_too_far_from_origin [DropFolder] at Players/05 - Striker (file=boots.fmdl, count=1)";

/// `/co/`'s identity line.
const IDENTIFIED: &str = "Info export_identified [Keep] (team=/co/, id=714)";

// TC-CHK-01
#[test]
fn a_far_vertex_drops_its_folder_at_check_and_at_compile_even_with_pass_through() {
    let sandbox = Sandbox::new("deep_far_vertex");
    sandbox.write(
        "exports/co Midcup Far/Players/05 - Striker/boots.fmdl",
        &deep_fixture("boots_far.fmdl"),
    );
    sandbox.write(
        "exports/co Midcup Far/Players/07 - Winger/boots.fmdl",
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.complete("exports/co Midcup Far");

    let check = sandbox.run("", &["check"]);
    assert_eq!(
        findings_of(&check.messages(), "co Midcup Far"),
        [
            FAR_STRIKER,
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - Striker (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/07 - Winger (file=boots.fmdl, count=1662)",
            IDENTIFIED
        ]
    );
    assert_eq!(check.exit_code(), 1);

    let compile = sandbox.run(
        &pass_through_settings(&sandbox),
        &["compile", "--no-deploy"],
    );
    assert_eq!(
        findings_of(&compile.messages(), "co Midcup Far"),
        [
            FAR_STRIKER,
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - Striker (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/07 - Winger (file=boots.fmdl, count=1662)",
            IDENTIFIED,
            TEAM_COLORS_MISSING
        ]
    );
    assert_eq!(compile.exit_code(), 1);
    // `/co/`'s block starts at 621: slot 07's boots are 627, slot 05's would be 625.
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "Asset/model/character/boots/k0627/#Win/boots.fpk",
            "Asset/model/character/boots/k0627/#Win/boots.fpkd",
            "Asset/model/character/face/real/71407/#Win/face.fpk",
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ]
    );
}

// TC-CHK-02
#[test]
fn a_far_vertex_in_a_solid_7z_is_found_by_check() {
    let sandbox = Sandbox::new("deep_far_vertex_7z");
    sandbox.write(
        "exports/co Midcup Far.7z",
        &deep_fixture("co Midcup Far.7z"),
    );

    let run = sandbox.run("", &["check"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Far.7z"),
        [
            FAR_STRIKER,
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - Striker (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - Striker (file=glove_l.fmdl, count=2)",
            IDENTIFIED
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-PLN-05
#[test]
fn one_worker_thread_finds_what_the_default_finds_in_a_folder_and_in_a_solid_7z() {
    let sandbox = Sandbox::new("deep_one_worker");
    let export = "exports/co Midcup Workers";
    sandbox.write(
        &format!("{export}/Players/05 - Striker/boots.fmdl"),
        &deep_fixture("boots_far.fmdl"),
    );
    sandbox.write(
        &format!("{export}/Players/07 - Winger/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(
        &format!("{export}/Common/glove_l.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(
        &format!("{export}/Players/07 - Winger/glove_l.fmdl.common"),
        b"",
    );
    sandbox.complete(export);
    let one_worker = format!("{}thread_count = 1\n", pes21_settings(&sandbox));

    let default = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    let single = sandbox.run(&one_worker, &["check"]);

    assert_eq!(
        findings_of(&default.messages(), "co Midcup Workers"),
        [
            FAR_STRIKER,
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - Striker (file=boots.fmdl, count=1662)",
            WINGER_WEIGHTS,
            "Info fmdl_weights_not_normalized [Keep] at Common/glove_l.fmdl (file=glove_l.fmdl, count=1662)",
            IDENTIFIED
        ]
    );
    assert_eq!(single.messages(), default.messages());
    assert_eq!(single.exit_code(), 1);

    // A solid `.7z` on one worker: every read goes through the archive's lock on that thread.
    let sandbox = Sandbox::new("deep_one_worker_7z");
    sandbox.write(
        "exports/co Midcup Far.7z",
        &deep_fixture("co Midcup Far.7z"),
    );
    let run = sandbox.run(&one_worker, &["check"]);
    assert_eq!(
        findings_of(&run.messages(), "co Midcup Far.7z"),
        [
            FAR_STRIKER,
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - Striker (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - Striker (file=glove_l.fmdl, count=2)",
            IDENTIFIED
        ]
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-SRC-10
#[test]
fn exports_are_reported_in_discovery_order_with_a_7z_among_folders() {
    let sandbox = Sandbox::new("deep_discovery_order");
    // Discovery sorts by name: the `.7z`, team `/dbg/`, falls between the folders.
    sandbox.write(
        "exports/dbg Midcup Two.7z",
        &deep_fixture("co Midcup Far.7z"),
    );
    for export in ["co Midcup One", "egg Midcup Three", "esg Midcup Four"] {
        sandbox.write(
            &format!("exports/{export}/Players/07 - Winger/boots.fmdl"),
            &tracer_player_file("boots.fmdl"),
        );
        sandbox.complete(&format!("exports/{export}"));
    }

    let run = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    let winger = |export: &str| format!("{export}: {WINGER_WEIGHTS}");
    let identified = |export: &str, team: &str, id: u32| {
        format!("{export}: Info export_identified [Keep] (team={team}, id={id})")
    };
    assert_eq!(
        run.messages(),
        [
            winger("co Midcup One"),
            identified("co Midcup One", "/co/", 714),
            format!("dbg Midcup Two.7z: {FAR_STRIKER}"),
            "dbg Midcup Two.7z: Info fmdl_weights_not_normalized [Keep] at Players/05 - Striker (file=boots.fmdl, count=1662)".to_owned(),
            "dbg Midcup Two.7z: Info fmdl_weights_not_normalized [Keep] at Players/05 - Striker (file=glove_l.fmdl, count=2)".to_owned(),
            identified("dbg Midcup Two.7z", "/dbg/", 790),
            winger("egg Midcup Three"),
            identified("egg Midcup Three", "/egg/", 792),
            winger("esg Midcup Four"),
            identified("esg Midcup Four", "/esg/", 793),
        ]
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-CHK-01
#[test]
fn a_far_vertex_in_a_shared_folder_drops_the_player_linking_it() {
    let sandbox = Sandbox::new("deep_far_vertex_shared");
    sandbox.write(
        "exports/co Midcup Far/Boots/Crocs/boots.fmdl",
        &deep_fixture("boots_far.fmdl"),
    );
    sandbox.write(
        "exports/co Midcup Far/Players/05 - Striker/Crocs.boots",
        b"",
    );
    sandbox.write(
        "exports/co Midcup Far/Players/05 - Striker/glove_l.fmdl",
        &tracer_player_file("glove_l.fmdl"),
    );
    sandbox.complete("exports/co Midcup Far");

    let run = sandbox.run("", &["check"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Far"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - Striker (file=glove_l.fmdl, count=2)",
            "Error vertex_too_far_from_origin [DropFolder] at Boots/Crocs (file=boots.fmdl, count=1)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Error link_target_dropped [DropFolder] at Players/05 - Striker (link=Crocs.boots, target=Boots/Crocs, finding=vertex_too_far_from_origin)",
            IDENTIFIED
        ]
    );
    assert_eq!(run.exit_code(), 1);
}

/// The tracer's boots model with its first mesh holding 21846 faces (its first, repeated), one
/// over the hard limit: `fmdl_mesh_over_face_limit`, an Error a member can still pass through.
fn boots_over_the_face_limit() -> Vec<u8> {
    let file = FmdlFile::read(&tracer_player_file("boots.fmdl")).unwrap();
    let mut model = Model::from_file(&file).unwrap();
    let mesh = &mut model.meshes[0];
    mesh.faces = vec![mesh.faces[0]; 21_846];
    model.to_file().unwrap().write()
}

/// Slot 07's boots, the tracer's, as both commands report them.
const WINGER_WEIGHTS: &str =
    "Info fmdl_weights_not_normalized [Keep] at Players/07 - Winger (file=boots.fmdl, count=1662)";

// TC-CHK-06
#[test]
fn a_format_error_drops_its_folder_unless_pass_through_keeps_it() {
    let sandbox = Sandbox::new("deep_format_error");
    sandbox.write(
        "exports/co Midcup Faces/Players/05 - Striker/boots.fmdl",
        &boots_over_the_face_limit(),
    );
    sandbox.write(
        "exports/co Midcup Faces/Players/07 - Winger/boots.fmdl",
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.complete("exports/co Midcup Faces");
    let striker_weights = "Info fmdl_weights_not_normalized [Keep] at Players/05 - Striker (file=boots.fmdl, count=1662)";
    let boots = |slot: u32| {
        [
            format!("Asset/model/character/boots/k{slot:04}/#Win/boots.fpk"),
            format!("Asset/model/character/boots/k{slot:04}/#Win/boots.fpkd"),
        ]
    };
    // Each player holds no face model: its face is the blank one.
    let face = |player: u32| {
        [
            format!("Asset/model/character/face/real/{player}/#Win/face.fpk"),
            format!("Asset/model/character/face/real/{player}/#Win/face.fpkd"),
        ]
    };

    for command in ["check", "compile"] {
        let run = sandbox.run(&pes21_settings(&sandbox), &command_args(command));
        let validated = [
            "Error fmdl_mesh_over_face_limit [DropFolder] at Players/05 - Striker (file=boots.fmdl, count=21846)",
            striker_weights,
            WINGER_WEIGHTS,
            IDENTIFIED,
        ];
        // Planning's note, which only `compile` makes.
        let planned: &[&str] = if command == "compile" {
            &[TEAM_COLORS_MISSING]
        } else {
            &[]
        };
        assert_eq!(
            findings_of(&run.messages(), "co Midcup Faces"),
            [&validated[..], planned].concat(),
            "{command}"
        );
        assert_eq!(run.exit_code(), 1, "{command}");
    }
    // `/co/`'s block starts at 621: slot 07's boots are 627, slot 05's 625.
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    let color_bins = [
        "common/character0/model/character/uniform/team/UniColor.bin".to_owned(),
        "common/etc/TeamColor.bin".to_owned(),
    ];
    assert_eq!(paths, [&boots(627)[..], &face(71407), &color_bins].concat());

    for command in ["check", "compile"] {
        let run = sandbox.run(&pass_through_settings(&sandbox), &command_args(command));
        let validated = [
            "Error fmdl_mesh_over_face_limit [Keep] at Players/05 - Striker (file=boots.fmdl, count=21846)",
            striker_weights,
            WINGER_WEIGHTS,
            IDENTIFIED,
        ];
        let planned: &[&str] = if command == "compile" {
            &[TEAM_COLORS_MISSING]
        } else {
            &[]
        };
        assert_eq!(
            findings_of(&run.messages(), "co Midcup Faces"),
            [&validated[..], planned].concat(),
            "{command}"
        );
        assert_eq!(run.exit_code(), 1, "{command}");
    }
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            &boots(625)[..],
            &boots(627),
            &face(71405),
            &face(71407),
            &color_bins
        ]
        .concat()
    );
}

/// The card head's face model with its one mesh holding 65536 vertices (its last, repeated),
/// one more than a `.model` mesh can index: `model_mesh_over_vertex_limit`, an Error.
fn card_over_the_vertex_limit() -> Vec<u8> {
    let file = pes_model::format::PreFoxModel::read(&card_model()).unwrap();
    let mut model = pes_model::model::Model::from_file(&file).unwrap();
    let vertices = &mut model.meshes[0].vertices;
    let last = vertices.positions.len() - 1;
    while vertices.positions.len() <= 65_535 {
        vertices.positions.push(vertices.positions[last]);
        if let Some(normals) = &mut vertices.normals {
            normals.push(normals[last]);
        }
        if let Some(tangents) = &mut vertices.tangents {
            tangents.push(tangents[last]);
        }
        if let Some(bitangents) = &mut vertices.bitangents {
            bitangents.push(bitangents[last]);
        }
        if let Some(colors) = &mut vertices.colors {
            colors.push(colors[last]);
        }
        for uvs in &mut vertices.uvs {
            uvs.push(uvs[last]);
        }
        if let Some(indices) = &mut vertices.bone_indices {
            indices.push(indices[last]);
        }
        if let Some(weights) = &mut vertices.bone_weights {
            weights.push(weights[last]);
        }
    }
    let bytes = model.to_file().unwrap().write().unwrap();
    // Read back, so a writer that dropped the extra vertices cannot pass this test silently.
    let written = pes_model::format::PreFoxModel::read(&bytes).unwrap();
    let written = pes_model::model::Model::from_file(&written).unwrap();
    assert_eq!(written.meshes[0].vertices.positions.len(), 65_536);
    bytes
}

// TC-CHK-08
#[test]
fn a_model_mesh_over_the_vertex_limit_drops_its_folder_at_check_and_compile() {
    let sandbox = Sandbox::new("deep_vertex_limit");
    // Slot 07 is the clean card head, so the CPK is written and slot 05's absence observable.
    for (slot, model) in [("05", card_over_the_vertex_limit()), ("07", card_model())] {
        let player = format!("exports/co Midcup Dense/Players/{slot} - A");
        sandbox.write(&format!("{player}/face_high.model"), &model);
        sandbox.write(&format!("{player}/face_high.mtl"), &card_materials());
        sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    }
    sandbox.complete_for("exports/co Midcup Dense", PesVersion::Pes17, &[]);
    let over = "Error model_mesh_over_vertex_limit [DropFolder] at Players/05 - A (file=face_high.model, count=65536)";
    let settings = pes_settings(&sandbox, 17);

    let check = sandbox.run(&settings, &["check"]);
    let lines = check.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Dense"),
        [over, IDENTIFIED],
        "{lines:#?}"
    );
    assert_eq!(check.exit_code(), 1);

    let compile = sandbox.run(&settings, &["compile", "--no-deploy"]);
    let lines = compile.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Dense"),
        [over, IDENTIFIED, TEAM_COLORS_MISSING],
        "{lines:#?}"
    );
    assert_eq!(compile.exit_code(), 1);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(entries.contains_key(&face_cpk(7)), "{:?}", entries.keys());
    assert!(!entries.contains_key(&face_cpk(5)), "{:?}", entries.keys());
}

// TC-CHK-09
#[test]
fn pass_through_keeps_a_common_mtl_s_eligible_error_and_the_player_whose_link_pairs_with_it() {
    let sandbox = Sandbox::new("deep_common_mtl_pass_through");
    let export = "exports/co Midcup Legs";
    sandbox.write(&format!("{export}/Players/05 - A/legs.model.common"), b"");
    sandbox.write(&format!("{export}/Common/legs.model"), &card_model());
    let materials = String::from_utf8(card_materials()).unwrap();
    let ztest = r#"<state name="ztest" value="1" />"#;
    assert!(materials.contains(ztest), "{materials}");
    let ztest_zero = materials.replace(ztest, r#"<state name="ztest" value="0" />"#);
    sandbox.write(&format!("{export}/Common/legs.mtl"), ztest_zero.as_bytes());
    sandbox.write(&format!("{export}/Common/skin.dds"), &small_dds());
    sandbox.complete_for(export, PesVersion::Pes17, &[]);
    let settings = format!(
        "{}[compiler]\npass_through = true\n",
        pes_settings(&sandbox, 17)
    );

    let run = sandbox.run(&settings, &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Legs"),
        [
            "Error mtl_state_invalid [Keep] at Common/legs.mtl (file=legs.mtl, count=1)",
            IDENTIFIED,
            TEAM_COLORS_MISSING,
            "Info xml_face_neck_added [Keep] at Players/05 - A ()",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(entries.contains_key(&face_cpk(5)), "{:?}", entries.keys());
}

// TC-CHK-07
#[test]
fn a_model_that_does_not_parse_is_model_broken_and_drops_its_folder_even_with_pass_through() {
    let sandbox = Sandbox::new("deep_model_broken");
    sandbox.write(
        "exports/co Midcup Broken/Players/05 - Striker/boots.fmdl",
        b"not a model",
    );
    sandbox.write(
        "exports/co Midcup Broken/Players/07 - Winger/boots.fmdl",
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.complete("exports/co Midcup Broken");
    let findings = [
        "Error model_broken [DropFolder] at Players/05 - Striker (file=boots.fmdl, error=fmdl is truncated)",
        WINGER_WEIGHTS,
        IDENTIFIED,
    ];

    let check = sandbox.run("", &["check"]);
    assert_eq!(findings_of(&check.messages(), "co Midcup Broken"), findings);
    assert_eq!(check.exit_code(), 1);

    let compile = sandbox.run(
        &pass_through_settings(&sandbox),
        &["compile", "--no-deploy"],
    );
    assert_eq!(
        findings_of(&compile.messages(), "co Midcup Broken"),
        [&findings[..], &[TEAM_COLORS_MISSING]].concat()
    );
    assert_eq!(compile.exit_code(), 1);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "Asset/model/character/boots/k0627/#Win/boots.fpk",
            "Asset/model/character/boots/k0627/#Win/boots.fpkd",
            "Asset/model/character/face/real/71407/#Win/face.fpk",
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ]
    );
}

// `compile` emits a logo it can decode (`logo.rs`), so the CPK holding no `flag/` entry below
// proves the undecodable one was dropped.
// TC-CHK-05
#[test]
fn a_logo_that_does_not_decode_is_logo_file_invalid_and_the_export_is_otherwise_kept() {
    let sandbox = Sandbox::new("deep_logo_invalid");
    sandbox.write("exports/co Midcup Logo/logo.png", b"not an image");
    sandbox.write(
        "exports/co Midcup Logo/Players/07 - Winger/boots.fmdl",
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.complete("exports/co Midcup Logo");
    let findings = [
        WINGER_WEIGHTS,
        "Error logo_file_invalid [DropFile] at logo.png (file=logo.png, error=image decode failed: Format error decoding Png: Invalid PNG signature.)",
        IDENTIFIED,
    ];

    let check = sandbox.run("", &["check"]);
    assert_eq!(findings_of(&check.messages(), "co Midcup Logo"), findings);
    assert_eq!(check.exit_code(), 1);

    // Pass-through does not keep it: there is nothing the game's logo sizes can be made from.
    let compile = sandbox.run(
        &pass_through_settings(&sandbox),
        &["compile", "--no-deploy"],
    );
    assert_eq!(
        findings_of(&compile.messages(), "co Midcup Logo"),
        [&findings[..], &[TEAM_COLORS_MISSING]].concat()
    );
    assert_eq!(compile.exit_code(), 1);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert!(
        paths
            .iter()
            .all(|path| !path.starts_with("common/render/symbol/flag/")),
        "no logo is emitted: {paths:?}"
    );
    assert_eq!(
        paths,
        [
            "Asset/model/character/boots/k0627/#Win/boots.fpk",
            "Asset/model/character/boots/k0627/#Win/boots.fpkd",
            "Asset/model/character/face/real/71407/#Win/face.fpk",
            "Asset/model/character/face/real/71407/#Win/face.fpkd",
            "common/character0/model/character/uniform/team/UniColor.bin",
            "common/etc/TeamColor.bin",
        ]
    );
}

/// The tracer's face models in the player folder `player`, as both commands report them.
fn tracer_face_weights(player: &str) -> [String; 3] {
    [
        ("boots.fmdl", 1662),
        ("fcl_hair.fmdl", 1662),
        ("glove_l.fmdl", 2),
    ]
    .map(|(file, count)| {
        format!(
            "Info fmdl_weights_not_normalized [Keep] at Players/{player} (file={file}, count={count})"
        )
    })
}

/// `settings_toml_invalid` on slot `player`'s `settings.toml` holding `name = 5`.
fn settings_invalid(player: &str) -> String {
    format!(
        "Error settings_toml_invalid [DropFile] at Players/{player}/settings.toml \
         (file=settings.toml, error=name: expected true or a string)"
    )
}

// TC-CHK-03
#[test]
fn a_settings_toml_that_does_not_parse_is_ignored_and_the_folder_s_models_compile() {
    let sandbox = Sandbox::new("deep_settings_invalid");
    sandbox.copy_tracer_face("exports/co Midcup Settings/Players/05 - A");
    // `name` takes `true` or a string.
    sandbox.write(
        "exports/co Midcup Settings/Players/05 - A/settings.toml",
        b"name = 5\n",
    );
    sandbox.complete("exports/co Midcup Settings");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let mut expected = tracer_face_weights("05 - A").to_vec();
    expected.push(settings_invalid("05 - A"));
    expected.push(IDENTIFIED.to_owned());
    expected.push(TEAM_COLORS_MISSING.to_owned());
    assert_eq!(findings_of(&run.messages(), "co Midcup Settings"), expected);
    assert_eq!(run.exit_code(), 1);
    assert_eq!(compiled_players(&sandbox), [71405]);
    // Every model package the folder holds commits: the face, the boots and the gloves.
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    for path in [
        "Asset/model/character/boots/k0625/#Win/boots.fpk",
        "Asset/model/character/boots/k0625/#Win/boots.fpkd",
        "Asset/model/character/face/real/71405/#Win/face.fpk",
        "Asset/model/character/face/real/71405/#Win/face.fpkd",
        "Asset/model/character/glove/g0625/#Win/glove.fpk",
        "Asset/model/character/glove/g0625/#Win/glove.fpkd",
    ] {
        assert!(entries.contains_key(path), "{path}");
    }
}

/// `kit_config_invalid` on `Kits/p1`, whose `config.toml` holds `shirt = 144`.
const KIT_CONFIG_INVALID: &str = "Error kit_config_invalid [DropFolder] at Kits/p1 \
     (file=config.toml, error=invalid value for shirt: 144)";

/// Writes a kit `p1` whose `config.toml` is the wrong-typed `shirt = 144` and a kit `p2`
/// with no config into `export`.
fn write_kits(sandbox: &Sandbox, export: &str) {
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(&format!("{export}/Kits/p1/config.toml"), b"shirt = 144\n");
    sandbox.write(&format!("{export}/Kits/p2/kit.dds"), &tracer_kit());
}

// TC-KIT-27
#[test]
fn a_kit_config_that_does_not_parse_leaves_its_kit_out_and_the_kit_beside_it_compiles() {
    let sandbox = Sandbox::new("deep_kit_config_invalid");
    write_kits(&sandbox, "exports/co Midcup Kits");
    sandbox.complete("exports/co Midcup Kits");

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    assert_eq!(
        findings_of(&check.messages(), "co Midcup Kits"),
        [KIT_CONFIG_INVALID, IDENTIFIED]
    );
    assert_eq!(check.exit_code(), 1);

    let compile = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    assert_eq!(
        findings_of(&compile.messages(), "co Midcup Kits"),
        [KIT_CONFIG_INVALID, IDENTIFIED, TEAM_COLORS_MISSING,]
    );
    assert_eq!(compile.exit_code(), 1);
    assert_eq!(compiled_kits(&sandbox), ["u0714p2"]);
}

// TC-KIT-27
#[test]
fn a_toml_syntax_error_is_one_console_line_per_finding() {
    let sandbox = Sandbox::new("deep_toml_snippet");
    let export = "exports/co Midcup Toml";
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    // An unclosed array: the parse error carries its source snippet into the finding's context.
    sandbox.write(&format!("{export}/Kits/p1/config.toml"), b"shirt = [\n");

    let run = sandbox.run("", &["check"]);

    // The console's line format is `studio_core::EventLines`'s: every physical line of the
    // output is a finding line, never a bare TOML snippet.
    let mut lines = studio_core::EventLines::new();
    let console: Vec<String> = run
        .events
        .iter()
        .filter_map(|envelope| lines.line(&envelope.event))
        .collect();
    for line in &console {
        for physical in line.lines() {
            assert!(
                physical.starts_with("- "),
                "a bare snippet line: {physical:?}\n{console:#?}"
            );
        }
    }
    let finding = console
        .iter()
        .find(|line| line.contains("kit_config_invalid"))
        .expect("the kit's invalid config is reported");
    assert!(
        finding.contains("co Midcup Toml") && finding.contains("Error kit_config_invalid"),
        "{finding:?}"
    );
}

// TC-KIT-12
#[test]
fn a_kit_colors_txt_line_that_gives_no_color_is_a_warning_and_the_export_is_still_identified() {
    let sandbox = Sandbox::new("deep_color_entry_invalid");
    let export = "exports/co Midcup Colors";
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    // The old Team Note kit entry, two colors on one line, after one valid color.
    sandbox.write(
        &format!("{export}/Kits/p1/colors.txt"),
        b"211 74 79\n211 74 79 - 162 62 77\n",
    );
    sandbox.complete(export);

    let run = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Colors"),
        [
            "Warning color_entry_invalid [Keep] at Kits/p1/colors.txt (line=2, reason=not one color)",
            IDENTIFIED
        ]
    );
    // A Warning alone leaves the run clean.
    assert_eq!(run.exit_code(), 0);
}

// TC-DSP-03
#[test]
fn pass_through_keeps_no_face_diff_kit_config_or_settings_toml_that_cannot_be_read() {
    let sandbox = Sandbox::new("deep_documents_pass_through");
    let export = "exports/co Midcup Unreadable";
    // Slot 03: a `face_diff.bin` one byte shorter than its header gives.
    sandbox.copy_tracer_face(&format!("{export}/Players/03 - A"));
    sandbox.write(
        &format!("{export}/Players/03 - A/face_diff.bin"),
        &face_diff_fixture("dif.bin")[..943],
    );
    // Slot 05: a `face_diff.xml` beside the tracer's `face_diff.bin`.
    sandbox.copy_tracer_face(&format!("{export}/Players/05 - B"));
    sandbox.write(
        &format!("{export}/Players/05 - B/face_diff.xml"),
        &face_diff_fixture("dif.xml"),
    );
    // Slot 07: a `settings.toml` whose `name` is a number.
    sandbox.copy_tracer_face(&format!("{export}/Players/07 - C"));
    sandbox.write(
        &format!("{export}/Players/07 - C/settings.toml"),
        b"name = 5\n",
    );
    write_kits(&sandbox, export);
    sandbox.complete(export);

    let run = sandbox.run(
        &pass_through_settings(&sandbox),
        &["compile", "--no-deploy"],
    );

    let mut expected = tracer_face_weights("03 - A").to_vec();
    expected.push(
        "Error face_diff_invalid [DropFolder] at Players/03 - A (file=face_diff.bin, \
         reason=the face diff is 943 bytes long, but its header gives 944: the game would \
         read past its end)"
            .to_owned(),
    );
    expected.extend(tracer_face_weights("05 - B"));
    expected.push(
        "Error xml_dif_conflict [DropFolder] at Players/05 - B (file=face_diff.xml)".to_owned(),
    );
    expected.extend(tracer_face_weights("07 - C"));
    expected.push(settings_invalid("07 - C"));
    expected.extend([KIT_CONFIG_INVALID, IDENTIFIED, TEAM_COLORS_MISSING].map(str::to_owned));
    assert_eq!(
        findings_of(&run.messages(), "co Midcup Unreadable"),
        expected
    );
    assert_eq!(run.exit_code(), 1);
    assert_eq!(compiled_players(&sandbox), [71407]);
    assert_eq!(compiled_kits(&sandbox), ["u0714p2"]);
}
