//! `compile` of kits for PES 2017: each kit's textures as DDS under the pre-Fox kit texture
//! folder, its mask (its own, or the bundled template when it has none) and never its srm, a
//! kit drawn for the Fox layout re-laid with its mask, the kit configs as loose files with no
//! `UniformParameter.bin`, and the two color bins written as on every version.

use std::fs;
use std::path::Path;

use dds_convert::{SourceFormat, decode};

use crate::common::Sandbox;
use crate::compile::{cpk_entries, kit_texture, pes21_settings, tracer_export, tracer_kit};
use crate::compile_exports::{
    TEAM_COLOR, UNI_COLOR, bundled_team_color, bundled_uni_color, config_path,
};
use crate::findings_of;
use crate::kit_layout::{
    PRE_FOX_BANDS, PRE_FOX_ISLANDS, STRIPES_FINDINGS, STRIPES_PRE_FOX_FINDINGS,
    assert_moved_inside_bands_only, bc1_dds, compiled_entries, compiled_pre_fox_top_level,
    stripe_centre, stripes, stripes_dds, stripes_dds_at_fox, write_stripes_export,
};
use crate::prefox_faces::{
    CLEAN, card_materials, card_model, compile_pes17, materials_naming, pes17, small_dds,
};

/// The CPK path of the pre-Fox kit texture `name` (`u0714p1`, `u0714p1_mask`).
fn pre_fox_kit_texture(name: &str) -> String {
    format!("common/character0/model/character/uniform/texture/{name}.dds")
}

/// The bundled mask template, `resources/kits/kit_mask.dds`, which a PES 15-17 kit with no
/// mask of its own is given.
fn bundled_kit_mask() -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resources/kits/kit_mask.dds"))
        .unwrap()
}

/// The tracer bullet's `config.toml`, a kit config that parses.
fn tracer_config() -> Vec<u8> {
    fs::read(Path::new(&tracer_export()).join("Kits/g1/config.toml")).unwrap()
}

/// Writes `Kits/<slot>/kit.dds` (the tracer's) and `config.toml` (the tracer's) into the
/// export `export`.
pub(crate) fn write_kit(sandbox: &Sandbox, export: &str, slot: &str) {
    let folder = format!("exports/{export}/Kits/{slot}");
    sandbox.write(&format!("{folder}/kit.dds"), &tracer_kit());
    sandbox.write(&format!("{folder}/config.toml"), &tracer_config());
}

// TC-KIT-21
#[test]
fn a_pes_17_kit_without_a_mask_gets_the_bundled_template_as_it_is() {
    let sandbox = Sandbox::new("prefox_kit_mask_template");
    write_kit(&sandbox, "co Full Mask", "p1");

    // The export's identity and its missing root colors, the generated and placeholder g1
    // a `Full` export plans, and p1's colors taken from its texture (no `colors.txt`):
    // nothing about the mask.
    let findings = [
        CLEAN[0],
        CLEAN[1],
        "Info kit_config_generated [Keep] at Kits/g1 ()",
        "Info kit_placeholder [Keep] at Kits/g1 ()",
        "Info kit_colors_derived [Keep] at Kits/p1 ()",
        "Warning kit_colors_missing [Keep] at Kits/g1 ()",
    ];
    let entries = compile_pes17(&sandbox, "co Full Mask", &findings);

    assert!(entries.contains_key(&pre_fox_kit_texture("u0714p1")));
    assert!(
        entries[&pre_fox_kit_texture("u0714p1_mask")] == bundled_kit_mask(),
        "the mask is the template, byte for byte"
    );
    // Nothing at the Fox kit texture paths.
    assert!(!entries.contains_key(&kit_texture("u0714p1")));
    assert!(!entries.contains_key(&kit_texture("u0714p1_mask")));
}

// TC-KIT-23
#[test]
fn pes_17_kit_configs_are_loose_files_and_no_uniform_parameter_bin_is_written() {
    let sandbox = Sandbox::new("prefox_kit_configs");
    write_kit(&sandbox, "co Midcup Configs", "p1");
    write_kit(&sandbox, "co Midcup Configs", "g1");

    let entries = compile_pes17(&sandbox, "co Midcup Configs", &[CLEAN[0], CLEAN[1]]);

    for ordinal in ["1st", "GK1st"] {
        let config = &entries[&config_path(ordinal)];
        assert_eq!(config.len(), 120, "{ordinal}");
    }
    let bins: Vec<&String> = entries
        .keys()
        .filter(|path| path.ends_with("UniformParameter.bin"))
        .collect();
    assert_eq!(bins, Vec::<&String>::new());
}

// TC-KIT-22
#[test]
fn the_mask_is_emitted_on_pes_17_and_the_srm_on_pes_21_each_dropping_the_other() {
    let sandbox = Sandbox::new("prefox_kit_maps");
    let folder = "exports/co Midcup Maps/Kits/p1";
    sandbox.write(&format!("{folder}/kit.dds"), &tracer_kit());
    sandbox.write(&format!("{folder}/config.toml"), &tracer_config());
    let mask = bc1_dds(64, 64, |_, _| [255, 0, 0, 255]);
    let srm = bc1_dds(64, 64, |_, _| [0, 0, 255, 255]);
    sandbox.write(&format!("{folder}/kit_mask.dds"), &mask);
    sandbox.write(&format!("{folder}/kit_srm.dds"), &srm);
    let not_used =
        |file: &str| format!("Info kit_texture_not_used [DropFile] at Kits/p1 (file={file})");

    let pes17_entries = compile_pes17(
        &sandbox,
        "co Midcup Maps",
        &[CLEAN[0], &not_used("kit_srm.dds"), CLEAN[1]],
    );

    let emitted = decode(
        &pes17_entries[&pre_fox_kit_texture("u0714p1_mask")],
        SourceFormat::Dds,
    )
    .unwrap();
    assert!(
        emitted.mips[0] == decode(&mask, SourceFormat::Dds).unwrap().mips[0],
        "the mask as given, not re-laid"
    );
    let maps: Vec<&String> = pes17_entries
        .keys()
        .filter(|path| path.contains("_srm"))
        .collect();
    assert_eq!(maps, Vec::<&String>::new());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    assert_eq!(
        findings_of(&run.messages(), "co Midcup Maps"),
        [CLEAN[0], &not_used("kit_mask.dds"), CLEAN[1],]
    );
    assert_eq!(run.exit_code(), 0);
    let pes21_entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let ftex = &pes21_entries[&kit_texture("u0714p1_srm")];
    let emitted = decode(&ftex::ftex_to_dds(ftex).unwrap(), SourceFormat::Dds).unwrap();
    assert!(
        emitted.mips[0] == decode(&srm, SourceFormat::Dds).unwrap().mips[0],
        "the srm as given, not re-laid"
    );
    let maps: Vec<&String> = pes21_entries
        .keys()
        .filter(|path| path.contains("_mask"))
        .collect();
    assert_eq!(maps, Vec::<&String>::new());
}

// TC-KIT-19
#[test]
fn a_pes_17_kit_marked_pre_fox_is_as_unmarked_and_one_marked_fox_is_re_laid() {
    let unmarked = Sandbox::new("prefox_kit_layout_unmarked");
    write_stripes_export(&unmarked, None);
    let pre_fox = Sandbox::new("prefox_kit_layout_pre_fox");
    write_stripes_export(&pre_fox, Some("pre-fox"));
    let fox = Sandbox::new("prefox_kit_layout_fox");
    write_stripes_export(&fox, Some("fox"));

    for sandbox in [&unmarked, &pre_fox] {
        compile_pes17(sandbox, "co Midcup Layout", &STRIPES_PRE_FOX_FINDINGS);
    }
    let mut converted = STRIPES_PRE_FOX_FINDINGS.to_vec();
    converted.insert(
        2,
        "Info kit_layout_converted [Keep] at Kits/p1 (from=fox, to=pre-fox)",
    );
    compile_pes17(&fox, "co Midcup Layout", &converted);

    assert!(compiled_entries(&pre_fox) == compiled_entries(&unmarked));
    let (relaid, width, _) = compiled_pre_fox_top_level(&fox, "u0714p1");
    let (as_drawn, _, _) = compiled_pre_fox_top_level(&unmarked, "u0714p1");
    assert_eq!(width, 1024);
    assert_moved_inside_bands_only(&relaid, &as_drawn, &PRE_FOX_BANDS);

    // The stripes drawn where the games' Fox models read them land back on their pre-Fox
    // ranges.
    let drawn_for_fox = Sandbox::new("prefox_kit_layout_fox_stripes");
    drawn_for_fox.write(
        "exports/co Midcup Layout/Kits/p1/kit.dds",
        &stripes_dds_at_fox(),
    );
    drawn_for_fox.write("exports/co Midcup Layout/Kits/p1/fox", b"");
    compile_pes17(&drawn_for_fox, "co Midcup Layout", &converted);
    let (relaid, _, _) = compiled_pre_fox_top_level(&drawn_for_fox, "u0714p1");
    for stripe in stripes() {
        let Some(centre) = stripe_centre(&relaid, &stripe, &PRE_FOX_ISLANDS) else {
            panic!(
                "{} stripe {} {:?}: no texel of its color",
                stripe.island, stripe.pre_fox, stripe.color
            );
        };
        assert!(
            (centre - stripe.pre_fox_centre).abs() <= 6.0,
            "{} stripe {}: centre {centre:.1}",
            stripe.island,
            stripe.pre_fox
        );
    }
}

// TC-KIT-29
#[test]
fn a_pes_17_kit_marked_fox_has_its_mask_re_laid_with_it() {
    let write = |sandbox: &Sandbox, marker: Option<&str>| {
        write_stripes_export(sandbox, marker);
        sandbox.write(
            "exports/co Midcup Layout/Kits/p1/kit_mask.dds",
            &stripes_dds(),
        );
    };
    let unmarked = Sandbox::new("prefox_kit_mask_layout_unmarked");
    write(&unmarked, None);
    let fox = Sandbox::new("prefox_kit_mask_layout_fox");
    write(&fox, Some("fox"));

    compile_pes17(&unmarked, "co Midcup Layout", &STRIPES_PRE_FOX_FINDINGS);
    // Once for the kit, its mask re-laid with its main texture.
    let mut converted = STRIPES_PRE_FOX_FINDINGS.to_vec();
    converted.insert(
        2,
        "Info kit_layout_converted [Keep] at Kits/p1 (from=fox, to=pre-fox)",
    );
    compile_pes17(&fox, "co Midcup Layout", &converted);

    let (relaid, width, _) = compiled_pre_fox_top_level(&fox, "u0714p1_mask");
    let (as_drawn, _, _) = compiled_pre_fox_top_level(&unmarked, "u0714p1_mask");
    assert_eq!(width, 1024);
    assert_moved_inside_bands_only(&relaid, &as_drawn, &PRE_FOX_BANDS);
}

// TC-KIT-20
#[test]
fn a_pes_17_placeholder_kit_marked_fox_has_only_its_own_mask_re_laid() {
    // No `kit.dds`, so the placeholder stands in; a row number atlas, which its shape alone
    // re-arranges; and the kit's own mask, the one texture the marker re-lays: it re-lays the
    // kit's own or inherited textures only.
    let row = bc1_dds(2048, 256, |x, _| {
        if (x / 64) % 2 == 0 {
            [255, 0, 0, 255]
        } else {
            [0, 0, 255, 255]
        }
    });
    let write = |sandbox: &Sandbox, marker: Option<&str>| {
        let folder = "exports/co Full Layout/Kits/p1";
        sandbox.write(&format!("{folder}/kit_back.dds"), &row);
        sandbox.write(&format!("{folder}/kit_mask.dds"), &stripes_dds());
        if let Some(marker) = marker {
            sandbox.write(&format!("{folder}/{marker}"), b"");
        }
    };
    let unmarked = Sandbox::new("prefox_placeholder_mask_unmarked");
    write(&unmarked, None);
    let fox = Sandbox::new("prefox_placeholder_mask_fox");
    write(&fox, Some("fox"));

    // A `Full` export holding no `g1/` compiles an empty one beside the placeholder p1.
    let findings = [
        STRIPES_FINDINGS[0],
        STRIPES_FINDINGS[1],
        "Info kit_config_generated [Keep] at Kits/p1 ()",
        "Info kit_placeholder [Keep] at Kits/p1 ()",
        "Info kit_config_generated [Keep] at Kits/g1 ()",
        "Info kit_placeholder [Keep] at Kits/g1 ()",
        "Warning kit_colors_missing [Keep] at Kits/p1 ()",
        "Warning kit_colors_missing [Keep] at Kits/g1 ()",
    ];
    let as_drawn = compile_pes17(&unmarked, "co Full Layout", &findings);
    // Once for the kit: its own mask is re-laid, the placeholder never.
    let mut converted = findings.to_vec();
    converted.insert(
        6,
        "Info kit_layout_converted [Keep] at Kits/p1 (from=fox, to=pre-fox)",
    );
    let relaid = compile_pes17(&fox, "co Full Layout", &converted);

    // The placeholder and the number atlas are as without the marker: a row becomes a column
    // by its shape, marker or not.
    for name in ["u0714p1", "u0714p1_back"] {
        let path = pre_fox_kit_texture(name);
        assert!(
            relaid[&path] == as_drawn[&path],
            "{name} is as without the marker"
        );
    }
    let (_, width, height) = compiled_pre_fox_top_level(&fox, "u0714p1_back");
    assert_eq!((width, height), (128, 2048));
    let (relaid_mask, width, _) = compiled_pre_fox_top_level(&fox, "u0714p1_mask");
    let (mask, _, _) = compiled_pre_fox_top_level(&unmarked, "u0714p1_mask");
    assert_eq!(width, 1024);
    assert_moved_inside_bands_only(&relaid_mask, &mask, &PRE_FOX_BANDS);
}

// TC-BIN-04
#[test]
fn a_pes_17_compile_writes_both_color_bins_and_no_fox_bin() {
    let sandbox = Sandbox::new("prefox_bins");
    let export = "co Midcup Bins";
    sandbox.write(
        &format!("exports/{export}/colors.txt"),
        b"#c11200\n#414141\n",
    );
    write_kit(&sandbox, export, "p1");
    // A player linking a shared boots folder, which on Fox would add a boots ID to the patch.
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.model"), &card_model());
    sandbox.write(&format!("{player}/face_high.mtl"), &card_materials());
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    sandbox.write(&format!("{player}/Crocs.boots"), b"");
    let crocs = format!("exports/{export}/Boots/Crocs");
    sandbox.write(&format!("{crocs}/boots.model"), &card_model());
    sandbox.write(&format!("{crocs}/boots.mtl"), &materials_naming("crocs"));
    sandbox.write(&format!("{crocs}/crocs.dds"), &small_dds());

    let run = sandbox.run(&pes17(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        lines[..2],
        [
            "Info bin_source [Keep] (bin=TeamColor.bin, cpk=bundled)",
            "Info bin_source [Keep] (bin=UniColor.bin, cpk=bundled)",
        ],
        "{lines:#?}"
    );
    assert_eq!(findings_of(&lines, export), [CLEAN[0]], "{lines:#?}");
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));

    let mut team_color = bundled_team_color();
    let team = (714 - 100) * 16;
    team_color[team + 4..team + 10].copy_from_slice(&[0xc1, 0x12, 0x00, 0x41, 0x41, 0x41]);
    assert!(
        entries[TEAM_COLOR] == team_color,
        "TeamColor.bin is the base with team 714's colors set"
    );
    // Team 714's record: the kit holds no `colors.txt`, so a `Midcup` kit's entry is the
    // base's kept and the bin is written as it came.
    let base = bundled_uni_color();
    assert!(
        entries[UNI_COLOR] == base,
        "UniColor.bin is the base, the kit's entry kept"
    );
    assert!(
        !entries
            .keys()
            .any(|path| path.ends_with("UniformParameter.bin")),
        "UniformParameter.bin"
    );
}
