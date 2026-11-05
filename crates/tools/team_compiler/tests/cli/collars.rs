//! `Collars/`: which files the export format admits there, the collar name's checks, which
//! `check` reports as `compile` does, and a team's collar compiled onto every kit of the team.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use kit_config::{KitConfig, KitSlot};
use pes_version::PesVersion;
use uniparam::UniformParameter;

use crate::common::Sandbox;
use crate::compile::{
    compiled_players, cpk_entries, pes_settings, pes21_settings, tracer_kit, tracer_player_file,
};
use crate::compile_exports::{UNIFORM_PARAMETER, bundled_uniform_parameter, emitted_config};
use crate::prefox_faces::card_model;
use crate::{
    CLEAN_PLAYER, TEAM_COLORS_MISSING, clean_model, findings_of, no_deploy_lines, patch_written,
};

/// The export the tests here write, with the coverage tag a `Midcup` export carries.
const EXPORT: &str = "exports/co Midcup Collars";

/// The CPK path of stock collar 12's model on Fox, which a team's `collar_12.fmdl` replaces.
const COLLAR_12: &str = "Asset/model/character/uniform/nocloth/#Win/collar_012.fmdl";

/// The CPK path of stock collar 12's model on PES 15-17, which a team's `collar_12.model`
/// replaces.
const PRE_FOX_COLLAR_12: &str =
    "common/character0/model/character/uniform/nocloth/collar_012.model";

/// The entries of the sandbox's compiled CPK.
fn compiled(sandbox: &Sandbox) -> BTreeMap<String, Vec<u8>> {
    cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"))
}

/// Team `team_id`'s kit configs in the `UniformParameter.bin` `bytes`, by entry name, decoded
/// for PES 21.
fn team_configs(bytes: &[u8], team_id: u16) -> BTreeMap<String, KitConfig> {
    let prefix = format!("{team_id:03}_");
    UniformParameter::read(bytes)
        .unwrap()
        .entries()
        .filter(|(name, _)| name.starts_with(&prefix))
        .map(|(name, bytes)| {
            let config = KitConfig::decode(bytes, PesVersion::Pes21).unwrap();
            (name.to_owned(), config)
        })
        .collect()
}

/// The shirt model, shorts model, collar and winter collar of `config`: the FPC values and
/// the collars.
fn fpc_fields(config: &KitConfig) -> (u8, u8, u8, u8) {
    (
        config.shirt.model,
        config.shorts.model,
        config.shirt.collar,
        config.shirt.winter_collar,
    )
}

/// The config names of the kits team 714's record in the bundled `UniColor.bin` lists: kit
/// numbers 0 to 6 and 0x10.
fn bundled_714_kits() -> Vec<String> {
    [
        KitSlot::P1,
        KitSlot::P2,
        KitSlot::P3,
        KitSlot::P4,
        KitSlot::P5,
        KitSlot::P6,
        KitSlot::P7,
        KitSlot::G1,
    ]
    .into_iter()
    .map(|slot| slot.config_name(714))
    .collect()
}

// TC-CMN-01
#[test]
fn a_team_s_collar_is_compiled_and_every_kit_config_of_the_team_wears_it_after_fpc() {
    let sandbox = Sandbox::new("collar_compiled");
    sandbox.write(
        &format!("{EXPORT}/Players/05 - A/face_high.fmdl"),
        &clean_model(),
    );
    sandbox.write(&format!("{EXPORT}/Players/05 - A/fpc_on"), b"");
    sandbox.write(&format!("{EXPORT}/Collars/collar_12.fmdl"), &clean_model());
    sandbox.write(&format!("{EXPORT}/Kits/p1/kit.dds"), &tracer_kit());
    // Shirt model 144: a config lacking the FPC values.
    sandbox.write(
        &format!("{EXPORT}/Kits/p1/config.toml"),
        b"[shirt]\nmodel = 144\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Collars"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            TEAM_COLORS_MISSING,
            "Info kit_config_fpc_adjusted [Keep] at Kits/p1 ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = compiled(&sandbox);
    assert!(
        entries.get(COLLAR_12) == Some(&clean_model()),
        "the collar file as it is: {:#?}",
        entries.keys()
    );
    // The FPC values, the collar overriding the FPC collar, in the loose config and in every
    // config of the team in the bin: p1's and the absent slots' alike.
    let fpc_with_collar = (176, 16, 12, 12);
    assert_eq!(
        fpc_fields(&emitted_config(&entries, "1st", PesVersion::Pes21)),
        fpc_with_collar
    );
    // The bin also holds configs for 714's kits 8 and 9, which its record does not list: the
    // game does not offer them, so they are left as the base has them.
    let base = team_configs(&bundled_uniform_parameter(), 714);
    let configs = team_configs(&entries[UNIFORM_PARAMETER], 714);
    assert_eq!(
        configs.keys().collect::<Vec<_>>(),
        base.keys().collect::<Vec<_>>()
    );
    let offered = bundled_714_kits();
    for (name, config) in &configs {
        if offered.contains(name) {
            assert_eq!(fpc_fields(config), fpc_with_collar, "{name}");
        } else {
            assert_eq!(config, &base[name], "{name}");
        }
    }
}

// TC-CMN-03
#[test]
fn of_two_exports_replacing_one_collar_the_later_one_loses_it_and_keeps_its_configs() {
    let sandbox = Sandbox::new("collar_conflict");
    let first = "exports/a Midcup Collars";
    // Two different models, so the CPK's collar shows whose it is.
    let firsts = clean_model();
    let seconds = tracer_player_file("glove_l.fmdl");
    assert_ne!(firsts, seconds);
    sandbox.write(&format!("{first}/Collars/collar_12.fmdl"), &firsts);
    sandbox.write(&format!("{EXPORT}/Collars/collar_12.fmdl"), &seconds);
    sandbox.write(&format!("{EXPORT}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{EXPORT}/Kits/p1/config.toml"),
        b"[shirt]\ncollar = 30\nwinter_collar = 31\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Collars"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Collars/collar_12.fmdl (file=collar_12.fmdl, count=2)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            TEAM_COLORS_MISSING,
            "Error collar_id_conflict [DropFile] at Collars/collar_12.fmdl (file=collar_12.fmdl, claimant=a Midcup Collars)",
        ]
    );
    assert_eq!(run.exit_code(), 1);
    let entries = compiled(&sandbox);
    assert!(
        entries.get(COLLAR_12) == Some(&firsts),
        "/a/'s collar, not /co/'s"
    );
    // /a/ resends no kit: every config of its team's kits in the bin wears its collar. Team
    // 702's record in the bundled `UniColor.bin` lists kits 0 to 7 and 0x10.
    let a_configs = team_configs(&entries[UNIFORM_PARAMETER], 702);
    let a_base = team_configs(&bundled_uniform_parameter(), 702);
    let offered: Vec<String> = [
        KitSlot::P1,
        KitSlot::P2,
        KitSlot::P3,
        KitSlot::P4,
        KitSlot::P5,
        KitSlot::P6,
        KitSlot::P7,
        KitSlot::P8,
        KitSlot::G1,
    ]
    .into_iter()
    .map(|slot| slot.config_name(702))
    .collect();
    for name in &offered {
        let config = &a_configs[name];
        let collars = (config.shirt.collar, config.shirt.winter_collar);
        assert_eq!(collars, (12, 12), "{name}");
    }
    for (name, config) in &a_configs {
        if !offered.contains(name) {
            assert_eq!(config, &a_base[name], "{name}: not offered, the base's");
        }
    }
    // /co/'s p1 keeps its own collars, and its other configs are the base's.
    let p1 = emitted_config(&entries, "1st", PesVersion::Pes21);
    assert_eq!((p1.shirt.collar, p1.shirt.winter_collar), (30, 31));
    let base = team_configs(&bundled_uniform_parameter(), 714);
    for (name, config) in team_configs(&entries[UNIFORM_PARAMETER], 714) {
        if name != KitSlot::P1.config_name(714) {
            assert_eq!(config, base[&name], "{name}");
        }
    }
}

// The absent slots of TC-CMN-01, with no FPC: a `Midcup` export resending no kit.
// TC-CMN-01
#[test]
fn a_midcup_export_s_collar_goes_into_its_team_s_kits_it_does_not_resend() {
    let sandbox = Sandbox::new("collar_midcup");
    sandbox.write(&format!("{EXPORT}/{CLEAN_PLAYER}"), &clean_model());
    sandbox.write(&format!("{EXPORT}/Collars/collar_12.fmdl"), &clean_model());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let mut expected = no_deploy_lines(
        &sandbox,
        [
            "co Midcup Collars: Info export_identified [Keep] (team=/co/, id=714)",
            &format!("co Midcup Collars: {TEAM_COLORS_MISSING}"),
        ],
    );
    expected.push(patch_written(&sandbox, "714"));
    assert_eq!(run.messages(), expected);
    let entries = compiled(&sandbox);
    let configs = team_configs(&entries[UNIFORM_PARAMETER], 714);
    let base = team_configs(&bundled_uniform_parameter(), 714);
    assert_eq!(
        configs.keys().collect::<Vec<_>>(),
        base.keys().collect::<Vec<_>>()
    );
    // Each config of a kit the record lists is the base's with both collars set, nothing
    // else changed; kits 8 and 9, which it does not list, keep the base's.
    let offered = bundled_714_kits();
    let mut worn = Vec::new();
    for (name, config) in configs {
        let mut expected = base[&name].clone();
        if offered.contains(&name) {
            expected.shirt.collar = 12;
            expected.shirt.winter_collar = 12;
            worn.push(name.clone());
        }
        assert_eq!(config, expected, "{name}");
    }
    worn.sort();
    let mut offered = offered;
    offered.sort();
    assert_eq!(worn, offered, "every offered kit has a config");
}

/// Konami's pre-Fox shirt model `modD_shirt_tight_in_collar_052.model`, from `pes_model`'s
/// fixtures: a `.model` that reads.
pub(crate) fn pre_fox_model() -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../libs/pes_model/tests/fixtures/konami_collar_052.wesys.model"),
    )
    .unwrap()
}

/// What a PES 17 compile with no PES install reports last for an export of `/co/` holding a
/// claimed collar and only `Kits/p1/`: the run finds no installed loose config of the other
/// kits the bundled `UniColor.bin` lists for team 714 (`bundled_714_kits`) to wear it.
const NO_LOOSE_CONFIGS: [&str; 7] = [
    "Warning kit_config_collar_unpatched [Keep] (slot=p2)",
    "Warning kit_config_collar_unpatched [Keep] (slot=p3)",
    "Warning kit_config_collar_unpatched [Keep] (slot=p4)",
    "Warning kit_config_collar_unpatched [Keep] (slot=p5)",
    "Warning kit_config_collar_unpatched [Keep] (slot=p6)",
    "Warning kit_config_collar_unpatched [Keep] (slot=p7)",
    "Warning kit_config_collar_unpatched [Keep] (slot=g1)",
];

// TC-CMN-02
#[test]
fn a_collar_named_for_a_reserved_or_no_stock_collar_is_refused_and_dropped() {
    let sandbox = Sandbox::new("collar_ids");
    for name in ["collar_105.fmdl", "neck.fmdl", "collar_9999.fmdl"] {
        sandbox.write(&format!("{EXPORT}/Collars/{name}"), &clean_model());
    }

    let run = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Collars"),
        [
            "Error collar_id_conflict [DropFile] at Collars/collar_105.fmdl (file=collar_105.fmdl, claimant=FPC)",
            "Error collar_id_invalid [DropFile] at Collars/collar_9999.fmdl (file=collar_9999.fmdl)",
            "Error collar_id_invalid [DropFile] at Collars/neck.fmdl (file=neck.fmdl)",
            "Info export_identified [Keep] (team=/co/, id=714)",
        ]
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-CMN-10
#[test]
fn a_texture_in_collars_is_dropped_and_the_collar_model_kept() {
    let sandbox = Sandbox::new("collar_texture");
    sandbox.write(&format!("{EXPORT}/Collars/collar_12.fmdl"), &clean_model());
    sandbox.write(&format!("{EXPORT}/Collars/collar_12.dds"), b"");
    let disallowed =
        "Error file_type_disallowed [DropFile] at Collars/collar_12.dds (file=collar_12.dds)";

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    assert_eq!(
        findings_of(&check.messages(), "co Midcup Collars"),
        [
            disallowed,
            "Info export_identified [Keep] (team=/co/, id=714)"
        ]
    );
    assert_eq!(check.exit_code(), 1);

    // The model is kept and compiled.
    let compile = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&compile.messages(), "co Midcup Collars"),
        [
            disallowed,
            "Info export_identified [Keep] (team=/co/, id=714)",
            TEAM_COLORS_MISSING,
        ]
    );
    assert!(compiled(&sandbox).get(COLLAR_12) == Some(&clean_model()));
}

// TC-CMN-08
#[test]
fn a_pes_17_model_collar_is_compiled_unchanged_and_the_team_s_loose_configs_wear_it() {
    let sandbox = Sandbox::new("collar_pre_fox_compiled");
    sandbox.write(
        &format!("{EXPORT}/Collars/collar_12.model"),
        &pre_fox_model(),
    );
    sandbox.write(&format!("{EXPORT}/Kits/p1/kit.dds"), &tracer_kit());

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);

    let lines = run.messages();
    // No installed loose config for p1: the template is generated at bins time,
    // reported after the task's findings.
    let mut expected = vec![
        "Info export_identified [Keep] (team=/co/, id=714)",
        TEAM_COLORS_MISSING,
        "Info kit_config_generated [Keep] at Kits/p1 ()",
    ];
    expected.extend(NO_LOOSE_CONFIGS);
    assert_eq!(
        findings_of(&lines, "co Midcup Collars"),
        expected,
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = compiled(&sandbox);
    assert!(
        entries.get(PRE_FOX_COLLAR_12) == Some(&pre_fox_model()),
        "the collar file as it is: {:#?}",
        entries.keys()
    );
    assert!(
        entries.keys().all(|path| !path.starts_with("Asset/")),
        "nothing at a Fox path: {:#?}",
        entries.keys()
    );
    let p1 = emitted_config(&entries, "1st", PesVersion::Pes17);
    assert_eq!((p1.shirt.collar, p1.shirt.winter_collar), (12, 12));
}

// TC-CMN-09
#[test]
fn a_pes_17_fmdl_collar_is_converted_with_the_stock_collars_material_names() {
    let sandbox = Sandbox::new("collar_pre_fox_fmdl");
    // The tracer's boots: materials `kit`, `shirt` and `shirt antiblur`, one mesh each. The
    // antiblur mesh and the material only it uses fold into the `shirt` mesh, as every
    // conversion reads an FMDL, leaving two materials and their two meshes.
    let source = tracer_player_file("boots.fmdl");
    let mut source_model = fmdl::Model::from_file(&fmdl::FmdlFile::read(&source).unwrap()).unwrap();
    let bindings_of = |model: &fmdl::Model| -> Vec<usize> {
        model.meshes.iter().map(|mesh| mesh.material).collect()
    };
    assert_eq!(
        (source_model.materials.len(), bindings_of(&source_model)),
        (3, vec![0, 1, 2])
    );
    fmdl::ops::antiblur::decode(&mut source_model).unwrap();
    let source_bindings = bindings_of(&source_model);
    assert_eq!(
        (source_model.materials.len(), source_bindings.as_slice()),
        (2, [0, 1].as_slice())
    );
    sandbox.write(&format!("{EXPORT}/Collars/collar_12.fmdl"), &source);
    sandbox.write(&format!("{EXPORT}/Kits/p1/kit.dds"), &tracer_kit());

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);

    let lines = run.messages();
    // The deep pass reads the FMDL as it is; the conversion reports what the `.model` has no
    // place for, on the collar file, but not its losses about a material (the boots' shadow
    // flag): no `.mtl` is written for a collar.
    let mut expected = vec![
        "Info fmdl_weights_not_normalized [Keep] at Collars/collar_12.fmdl (file=collar_12.fmdl, count=1662)",
        "Info export_identified [Keep] (team=/co/, id=714)",
        TEAM_COLORS_MISSING,
        "Info native_field_dropped [Keep] at Collars/collar_12.fmdl (model=collar_12.fmdl, field=bone_matrices)",
        "Info kit_config_generated [Keep] at Kits/p1 ()",
    ];
    expected.extend(NO_LOOSE_CONFIGS);
    assert_eq!(
        findings_of(&lines, "co Midcup Collars"),
        expected,
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = compiled(&sandbox);
    assert!(
        entries
            .keys()
            .all(|path| !path.ends_with(".fmdl") && !path.ends_with(".mtl")),
        "no FMDL and no material set: {:#?}",
        entries.keys()
    );
    let Some(collar) = entries.get(PRE_FOX_COLLAR_12) else {
        panic!("no converted collar: {:#?}", entries.keys());
    };
    let model =
        pes_model::model::Model::from_file(&pes_model::format::PreFoxModel::read(collar).unwrap())
            .unwrap();
    assert_eq!(model.materials, ["uni_collar", "uni_shirts"]);
    // Each mesh is the source's, in order: the first material's stays on `uni_collar`, every
    // other material's goes to `uni_shirts`.
    let bindings: Vec<usize> = model.meshes.iter().map(|mesh| mesh.material).collect();
    let expected: Vec<usize> = source_bindings
        .iter()
        .map(|material| usize::from(*material != 0))
        .collect();
    assert_eq!(bindings, expected);
    let p1 = emitted_config(&entries, "1st", PesVersion::Pes17);
    assert_eq!((p1.shirt.collar, p1.shirt.winter_collar), (12, 12));
}

// TC-CMN-19
#[test]
fn a_pes_16_fmdl_collar_is_converted_with_every_material_named_uni_shirts() {
    let sandbox = Sandbox::new("collar_pes_16_fmdl");
    // The tracer's boots, two materials and two meshes once their antiblur mesh folds in
    // (TC-CMN-09's source).
    sandbox.write(
        &format!("{EXPORT}/Collars/collar_12.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(&format!("{EXPORT}/Kits/p1/kit.dds"), &tracer_kit());

    let run = sandbox.run(&pes_settings(&sandbox, 16), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = compiled(&sandbox);
    let Some(collar) = entries.get(PRE_FOX_COLLAR_12) else {
        panic!("no converted collar: {:#?}", entries.keys());
    };
    let model =
        pes_model::model::Model::from_file(&pes_model::format::PreFoxModel::read(collar).unwrap())
            .unwrap();
    assert_eq!(model.materials, ["uni_shirts"]);
    let bindings: Vec<usize> = model.meshes.iter().map(|mesh| mesh.material).collect();
    assert_eq!(bindings, [0, 0]);
}

// TC-CMN-12
#[test]
fn a_gltf_collar_is_dropped_and_its_export_compiled_without_it() {
    for (number, version) in [(21, PesVersion::Pes21), (17, PesVersion::Pes17)] {
        let sandbox = Sandbox::new(&format!("collar_gltf_{number}"));
        sandbox.write(&format!("{EXPORT}/Collars/collar_12.glb"), b"glTF");
        sandbox.write(&format!("{EXPORT}/Kits/p1/kit.dds"), &tracer_kit());
        sandbox.write(
            &format!("{EXPORT}/Kits/p1/config.toml"),
            b"[shirt]\ncollar = 30\nwinter_collar = 31\n",
        );

        let run = sandbox.run(&pes_settings(&sandbox, number), &["compile", "--no-deploy"]);

        let lines = run.messages();
        assert_eq!(
            findings_of(&lines, "co Midcup Collars"),
            [
                "Info export_identified [Keep] (team=/co/, id=714)",
                TEAM_COLORS_MISSING,
                "Error model_gltf_unsupported [DropFile] at Collars/collar_12.glb (file=collar_12.glb)",
            ],
            "{version}: {lines:#?}"
        );
        assert_eq!(run.exit_code(), 1, "{version}: {lines:#?}");
        assert_collar_left_out(&sandbox, version, (30, 31));
    }
}

// TC-CMN-33
#[test]
fn a_pes_17_fmdl_collar_whose_conversion_fails_is_dropped_and_its_export_compiled() {
    let sandbox = Sandbox::new("collar_pre_fox_fmdl_failed");
    // A material name holding a control character: `fmdl` reads and checks it, and the
    // conversion refuses it, a `.mtl` being XML.
    let mut model = fmdl::Model::from_file(&fmdl::FmdlFile::read(&clean_model()).unwrap()).unwrap();
    "kit\u{1}".clone_into(&mut model.materials[0].name);
    sandbox.write(
        &format!("{EXPORT}/Collars/collar_12.fmdl"),
        &model.to_file().unwrap().write(),
    );
    sandbox.write(&format!("{EXPORT}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{EXPORT}/Kits/p1/config.toml"),
        b"[shirt]\ncollar = 30\nwinter_collar = 31\n",
    );

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);

    let lines = run.messages();
    let mut expected = vec![
        "Info export_identified [Keep] (team=/co/, id=714)",
        TEAM_COLORS_MISSING,
        "Error model_conversion_failed [DropFile] at Collars/collar_12.fmdl (model=collar_12.fmdl, error=a name or path for the `.mtl` holds a character XML 1.0 cannot represent: \"kit\\u{1}\")",
    ];
    expected.extend(NO_LOOSE_CONFIGS);
    assert_eq!(
        findings_of(&lines, "co Midcup Collars"),
        expected,
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    // Planning gave the team's kits the collar the export claimed before the task converting
    // it failed, so p1 wears stock collar 12, not its own 30 and 31.
    assert_collar_left_out(&sandbox, PesVersion::Pes17, (12, 12));
}

/// Asserts the sandbox's CPK, compiled for `version` with a collar that was dropped, holds
/// no collar, and the export's p1 config wears `collars` (its collar and winter collar).
fn assert_collar_left_out(sandbox: &Sandbox, version: PesVersion, collars: (u8, u8)) {
    let entries = compiled(sandbox);
    assert!(
        entries.keys().all(|path| !path.contains("/nocloth/")),
        "{version}: no collar: {:#?}",
        entries.keys()
    );
    let p1 = emitted_config(&entries, "1st", version);
    assert_eq!(
        (p1.shirt.collar, p1.shirt.winter_collar),
        collars,
        "{version}"
    );
}

/// PES 17's stock collar 1, `common/character0/model/character/uniform/nocloth/collar_001.model`
/// in its `dt35_win.cpk`, from `pes_model`'s fixtures, WESYS-wrapped as the game ships it: a
/// `.model` collar whose materials, `uni_shirts` and `uni_collar`, the templates' `uniform.mtl`
/// defines, posed off PES 15's skeleton.
fn stock_collar_model() -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../libs/pes_model/tests/fixtures/konami_collar_001.wesys.model"),
    )
    .unwrap()
}

// TC-CMN-11
#[test]
fn a_pes_21_model_collar_is_converted_with_the_templates_uniform_mtl() {
    let sandbox = Sandbox::new("collar_fox_model");
    sandbox.write(
        &format!("{EXPORT}/Collars/collar_12.model"),
        &stock_collar_model(),
    );
    sandbox.write(&format!("{EXPORT}/{CLEAN_PLAYER}"), &clean_model());
    sandbox.write(&format!("{EXPORT}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{EXPORT}/Kits/p1/config.toml"),
        b"[shirt]\ncollar = 30\nwinter_collar = 31\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    // Each mesh's bitangents, which the FMDL has no place for; no bone moved, every bone of
    // the collar posed as PES 21's; no loss about a material (the dummy specular map the
    // converter adds), the collar writing none.
    assert_eq!(
        findings_of(&lines, "co Midcup Collars"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            TEAM_COLORS_MISSING,
            "Info vertex_bitangents_dropped [Keep] at Collars/collar_12.model (model=collar_12.model, mesh=0)",
            "Info vertex_bitangents_dropped [Keep] at Collars/collar_12.model (model=collar_12.model, mesh=1)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = compiled(&sandbox);
    let Some(collar) = entries.get(COLLAR_12) else {
        panic!("no converted collar: {:#?}", entries.keys());
    };
    let model = fmdl::Model::from_file(&fmdl::FmdlFile::read(collar).unwrap()).unwrap();
    // The `.model`'s material names in its order, each the stock PES 21 collar's material of
    // that name (`collar_107.fmdl`): its shader and technique, the game's pattern texture as
    // its one sampler, and its parameters.
    let pattern = vec![(
        "Pattern_Tex_LIN".to_owned(),
        fmdl::Texture {
            file_name: "uni_pattern.dds".to_owned(),
            directory: "/Assets/pes16/model/character/common/sourceimages/".to_owned(),
        },
    )];
    let parameters: Vec<(String, [f32; 4])> = [
        ("MatParamIndex_0", [40.0, 0.0, 0.0, 0.0]),
        ("BlendNormalXParam", [0.0, 0.0, 0.0, 0.0]),
        ("BlendNormalYParam", [0.666, 0.0, 0.0, 0.0]),
        ("RepetitionParam", [80.0, 0.0, 0.0, 0.0]),
        ("BlendCoeffParam", [0.0, 0.0, 0.0, 0.0]),
        ("BlendBoostParam", [0.0, 0.0, 0.0, 0.0]),
        ("PatchAnisoRoughnessParam", [0.4, 0.0, 0.0, 0.0]),
        ("PatternIndexParam", [1.0, 0.0, 0.0, 0.0]),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value))
    .collect();
    let stock = |name: &str, shader: &str, technique: &str| fmdl::MaterialInstance {
        name: name.to_owned(),
        shader: shader.to_owned(),
        technique: technique.to_owned(),
        textures: pattern.clone(),
        parameters: parameters.clone(),
    };
    assert_eq!(
        model.materials,
        [
            stock("uni_shirts", "pes_3ddf_shirt_nb", "pes3DDF_Shirt_NB_NC"),
            stock("uni_collar", "pes_3ddf_collar", "pes3DDF_Collar_NC"),
        ]
    );
    let mesh_flags: Vec<(u8, u8)> = model
        .meshes
        .iter()
        .map(|mesh| (mesh.alpha_flags, mesh.shadow_flags))
        .collect();
    assert_eq!(
        mesh_flags,
        [(32, 0), (32, 0)],
        "as the stock collar's meshes"
    );
    assert!(
        entries
            .keys()
            .all(|path| !path.contains("/nocloth/") || path == COLLAR_12),
        "the collar alone, no skeleton beside it: {:#?}",
        entries.keys()
    );
    // p1 and every other kit of the team the bundled `UniColor.bin` offers wear the collar.
    let p1 = emitted_config(&entries, "1st", PesVersion::Pes21);
    assert_eq!((p1.shirt.collar, p1.shirt.winter_collar), (12, 12));
    let configs = team_configs(&entries[UNIFORM_PARAMETER], 714);
    for name in bundled_714_kits() {
        let config = &configs[&name];
        let collars = (config.shirt.collar, config.shirt.winter_collar);
        assert_eq!(collars, (12, 12), "{name}");
    }
    assert_eq!(compiled_players(&sandbox), [71403]);
}

// The same-engine pre-check on a `.model` collar.
// TC-MOD-50
#[test]
fn a_model_collar_posed_off_the_version_s_skeleton_is_moved_onto_it_and_kept_wrapped_otherwise() {
    let source = stock_collar_model();
    // Flagged on PES 15: its trapezius bones, which PES 15's skeleton lacks, folded onto the
    // shoulders, its scapula and pectoralis bones moved onto PES 15's pose, written plain as
    // every converted model is, its material names kept.
    let sandbox = Sandbox::new("collar_pre_fox_moved");
    sandbox.write(&format!("{EXPORT}/Collars/collar_12.model"), &source);
    sandbox.write(&format!("{EXPORT}/Kits/p1/kit.dds"), &tracer_kit());

    let run = sandbox.run(&pes_settings(&sandbox, 15), &["compile", "--no-deploy"]);

    let lines = run.messages();
    let mut expected = vec![
        "Info export_identified [Keep] (team=/co/, id=714)",
        TEAM_COLORS_MISSING,
        "Info bone_folded_for_version [Keep] at Collars/collar_12.model (model=collar_12.model, bone=dsk_trapezius_l -> sk_shoulder_l)",
        "Info bone_folded_for_version [Keep] at Collars/collar_12.model (model=collar_12.model, bone=dsk_trapezius_r -> sk_shoulder_r)",
        "Info skeleton_retargeted [Keep] at Collars/collar_12.model (model=collar_12.model, bones=4)",
        "Info kit_config_generated [Keep] at Kits/p1 ()",
    ];
    expected.extend(NO_LOOSE_CONFIGS);
    assert_eq!(
        findings_of(&lines, "co Midcup Collars"),
        expected,
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = compiled(&sandbox);
    let Some(collar) = entries.get(PRE_FOX_COLLAR_12) else {
        panic!("no collar: {:#?}", entries.keys());
    };
    assert_ne!(collar, &source, "moved onto PES 15's skeleton");
    assert!(!wezlib::is_wrapped(collar), "written plain");
    let model =
        pes_model::model::Model::from_file(&pes_model::format::PreFoxModel::read(collar).unwrap())
            .unwrap();
    assert_eq!(model.materials, ["uni_shirts", "uni_collar"]);

    // Not flagged on PES 17: packed byte for byte, still wrapped.
    let sandbox = Sandbox::new("collar_pre_fox_kept");
    sandbox.write(&format!("{EXPORT}/Collars/collar_12.model"), &source);

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    assert!(compiled(&sandbox).get(PRE_FOX_COLLAR_12) == Some(&source));
}

// TC-CMN-03
#[test]
fn of_two_pes_17_exports_replacing_one_collar_the_later_one_loses_it() {
    let sandbox = Sandbox::new("collar_pre_fox_conflict");
    let first = "exports/a Midcup Collars";
    // Two different models, so the CPK's collar shows whose it is.
    let firsts = pre_fox_model();
    let seconds = card_model();
    assert_ne!(firsts, seconds);
    sandbox.write(&format!("{first}/Collars/collar_12.model"), &firsts);
    sandbox.write(&format!("{EXPORT}/Collars/collar_12.model"), &seconds);

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Collars"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            TEAM_COLORS_MISSING,
            "Error collar_id_conflict [DropFile] at Collars/collar_12.model (file=collar_12.model, claimant=a Midcup Collars)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    let entries = compiled(&sandbox);
    assert!(
        entries.get(PRE_FOX_COLLAR_12) == Some(&firsts),
        "/a/'s collar, not /co/'s: {:#?}",
        entries.keys()
    );
}

// The refusal half of TC-CMN-08.
// TC-CMN-08
#[test]
fn a_pre_fox_collar_past_the_version_s_stock_set_is_refused() {
    let sandbox = Sandbox::new("collar_pre_fox");
    sandbox.write(
        &format!("{EXPORT}/Collars/collar_117.model"),
        &pre_fox_model(),
    );
    sandbox.write(&format!("{EXPORT}/Kits/p1/kit.dds"), &tracer_kit());

    let past = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);

    let lines = past.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Collars"),
        [
            "Error collar_id_invalid [DropFile] at Collars/collar_117.model (file=collar_117.model)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            TEAM_COLORS_MISSING,
            "Info kit_config_generated [Keep] at Kits/p1 ()",
        ],
        "{lines:#?}"
    );
    assert_eq!(past.exit_code(), 1, "{lines:#?}");
    let entries = compiled(&sandbox);
    assert!(
        entries
            .keys()
            .all(|path| !path.ends_with("collar_117.model") && !path.contains("/nocloth/")),
        "no collar: {:#?}",
        entries.keys()
    );

    let sandbox = Sandbox::new("collar_pre_fox_last");
    sandbox.write(
        &format!("{EXPORT}/Collars/collar_116.model"),
        &pre_fox_model(),
    );

    let last = sandbox.run(&pes_settings(&sandbox, 17), &["check"]);

    assert_eq!(
        findings_of(&last.messages(), "co Midcup Collars"),
        ["Info export_identified [Keep] (team=/co/, id=714)"]
    );
    assert_eq!(last.exit_code(), 0);
}

// TC-CMN-16
#[test]
fn a_collar_below_a_collars_subfolder_is_kept_by_the_lenient_check_and_not_a_collar() {
    let sandbox = Sandbox::new("collar_nested");
    sandbox.write(
        &format!("{EXPORT}/Collars/sub/collar_12.model"),
        &pre_fox_model(),
    );
    sandbox.write(&format!("{EXPORT}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{EXPORT}/Kits/p1/config.toml"),
        b"[shirt]\ncollar = 30\nwinter_collar = 31\n",
    );
    let settings = format!(
        "{}[team-compiler]\nstrict_file_type_check = false\n",
        pes_settings(&sandbox, 17)
    );

    let run = sandbox.run(&settings, &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Collars"),
        [
            "Info file_type_disallowed [Keep] at Collars/sub/collar_12.model (file=sub/collar_12.model)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            TEAM_COLORS_MISSING,
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert_collar_left_out(&sandbox, PesVersion::Pes17, (30, 31));
}

// TC-CMN-16
#[test]
fn a_file_of_no_model_kind_in_collars_kept_by_the_lenient_check_gets_no_collar_finding() {
    let sandbox = Sandbox::new("collar_readme");
    sandbox.write(&format!("{EXPORT}/Collars/README.txt"), b"collar notes");
    sandbox.write(&format!("{EXPORT}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{EXPORT}/Kits/p1/config.toml"),
        b"[shirt]\ncollar = 30\nwinter_collar = 31\n",
    );
    let settings = format!(
        "{}[team-compiler]\nstrict_file_type_check = false\n",
        pes_settings(&sandbox, 17)
    );

    let run = sandbox.run(&settings, &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Collars"),
        [
            "Info file_type_disallowed [Keep] at Collars/README.txt (file=README.txt)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            TEAM_COLORS_MISSING,
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert_collar_left_out(&sandbox, PesVersion::Pes17, (30, 31));
}

// TC-CMN-02
#[test]
fn an_export_whose_only_collar_is_dropped_compiles_without_it() {
    let sandbox = Sandbox::new("collar_dropped");
    sandbox.write(&format!("{EXPORT}/{CLEAN_PLAYER}"), &clean_model());
    sandbox.write(
        &format!("{EXPORT}/Collars/collar_9999.fmdl"),
        &clean_model(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let mut expected = no_deploy_lines(
        &sandbox,
        [
            "co Midcup Collars: Error collar_id_invalid [DropFile] at Collars/collar_9999.fmdl (file=collar_9999.fmdl)",
            "co Midcup Collars: Info export_identified [Keep] (team=/co/, id=714)",
            &format!("co Midcup Collars: {TEAM_COLORS_MISSING}"),
        ],
    );
    expected.push(patch_written(&sandbox, "714"));
    assert_eq!(run.messages(), expected);
    assert_eq!(compiled_players(&sandbox), [71403]);
    assert_eq!(run.exit_code(), 1);
}
