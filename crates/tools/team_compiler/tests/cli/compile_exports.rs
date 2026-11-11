//! `compile` over each export's content: the roster, the kits, a nested root, the root
//! files, the source kinds, and the run across several exports.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use dds_convert::{BlockCodec, SourceFormat, decode};
use kit_config::{KitConfig, KitSlot, ShortSleeves};
use pes_version::PesVersion;
use studio_core::PipelineEvent;

use crate::bins::{
    install_cpk, install_names, installed_configs, shirt_144_config, uni_color_p1_p2_p3,
};
use crate::common::Sandbox;
use crate::compile::{
    compiled_kits, compiled_players, compiled_portraits, cpk_entries, kit_texture,
    pass_through_settings, pes_settings, pes21_settings, pixels_cut_dds, tracer_export, tracer_kit,
    tracer_player_file, tracer_portrait,
};
use crate::textures::{bc1_dds, texture_fixture};
use crate::{
    BUNDLED_BINS, CLEAN_PLAYER, TEAM_COLORS_MISSING, bundled_bins_then, clean_model, command_args,
    deploy_skipped, findings_of, no_deploy_lines, patch_written, source_fixture,
};

// TC-ROS-01
#[test]
fn without_players_txt_each_folder_is_the_player_its_number_names() {
    let sandbox = Sandbox::new("ros_numbered");
    sandbox.copy_tracer_face("exports/co Midcup Numbers/Players/03 - A");
    sandbox.copy_tracer_face("exports/co Midcup Numbers/Players/15 - B");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Numbers"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=glove_l.fmdl, count=2)",
            "Info fmdl_weights_not_normalized [Keep] at Players/15 - B (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/15 - B (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/15 - B (file=glove_l.fmdl, count=2)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(compiled_players(&sandbox), [71403, 71415]);
    assert_eq!(run.exit_code(), 0);
}

// TC-ROS-04
#[test]
fn players_txt_maps_a_folder_by_name_and_leaves_an_unlisted_one_out() {
    let sandbox = Sandbox::new("ros_named");
    sandbox.write("exports/co Midcup Roster/players.txt", b"03 snuffy\n");
    sandbox.copy_tracer_face("exports/co Midcup Roster/Players/Snuffy");
    sandbox.copy_tracer_face("exports/co Midcup Roster/Players/15 - B");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Roster"),
        [
            "Warning player_unlisted [DropFolder] at Players/15 - B ()",
            "Info fmdl_weights_not_normalized [Keep] at Players/Snuffy (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/Snuffy (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/Snuffy (file=glove_l.fmdl, count=2)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(compiled_players(&sandbox), [71403]);
    assert_eq!(run.exit_code(), 0);
}

// TC-ROS-05
#[test]
fn a_folder_listed_under_two_slots_is_compiled_for_both() {
    let sandbox = Sandbox::new("ros_two_slots");
    sandbox.write("exports/co Midcup Twice/players.txt", b"03 A\n07 A\n");
    sandbox.copy_tracer_face("exports/co Midcup Twice/Players/A");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Twice"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/A (file=glove_l.fmdl, count=2)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(compiled_players(&sandbox), [71403, 71407]);
    assert_eq!(run.exit_code(), 0);
}

// TC-ROS-06
#[test]
fn each_bad_roster_line_is_reported_and_only_the_good_one_compiled() {
    let sandbox = Sandbox::new("ros_bad_lines");
    sandbox.write(
        "exports/co Midcup Lines/players.txt",
        b"x3 A\n24 B\n05 Nobody\n07 C\n",
    );
    for folder in ["A", "B", "C"] {
        sandbox.copy_tracer_face(&format!("exports/co Midcup Lines/Players/{folder}"));
    }

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Lines"),
        [
            "Error players_txt_line_invalid [DropSlot] at players.txt line 1 slot None ()",
            "Error players_txt_slot_invalid [DropSlot] at players.txt line 2 slot Some(24) ()",
            "Error players_txt_target_missing [DropSlot] at players.txt line 3 slot Some(5) (folder=Nobody)",
            "Warning player_unlisted [DropFolder] at Players/A ()",
            "Info fmdl_weights_not_normalized [Keep] at Players/C (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/C (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/C (file=glove_l.fmdl, count=2)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(compiled_players(&sandbox), [71407]);
    assert_eq!(run.exit_code(), 1);
}

// TC-ROS-09
#[test]
fn an_empty_players_txt_compiles_the_kits_and_no_player() {
    let sandbox = Sandbox::new("ros_empty_roster");
    sandbox.write("exports/co Midcup Empty/players.txt", b"");
    sandbox.copy_tracer_face("exports/co Midcup Empty/Players/03 - A");
    sandbox.write("exports/co Midcup Empty/Kits/p1/kit.dds", &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Empty"),
        [
            "Warning player_unlisted [DropFolder] at Players/03 - A ()",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert!(compiled_players(&sandbox).is_empty());
    assert_eq!(compiled_kits(&sandbox), ["u0714p1"]);
    assert_eq!(run.exit_code(), 0);
}

// TC-KIT-01
#[test]
fn kit_folders_are_compiled_as_the_slot_their_name_starts_with() {
    let sandbox = Sandbox::new("kit_named");
    sandbox.write(
        "exports/co Midcup Kits/Kits/p1 - Lakers/kit.dds",
        &tracer_kit(),
    );
    sandbox.write("exports/co Midcup Kits/Kits/g1/kit.dds", &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Kits"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert_eq!(compiled_kits(&sandbox), ["u0714g1", "u0714p1"]);
    assert_eq!(run.exit_code(), 0);
}

// TC-KIT-11
#[test]
fn a_kit_holding_only_a_back_texture_gets_the_placeholder_main_texture() {
    let sandbox = Sandbox::new("kit_back_only");
    sandbox.write("exports/co Full Back/Kits/p4/kit_back.dds", &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Full Back"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info kit_config_generated [Keep] at Kits/p4 ()",
            "Info kit_placeholder [Keep] at Kits/p4 ()",
            "Info kit_config_generated [Keep] at Kits/g1 ()",
            "Info kit_placeholder [Keep] at Kits/g1 ()",
            "Warning kit_colors_missing [Keep] at Kits/p4 ()",
            "Warning kit_colors_missing [Keep] at Kits/g1 ()",
        ]
    );
    assert_eq!(
        compiled_kits(&sandbox),
        ["u0714g1", "u0714p4", "u0714p4_back"]
    );
    let names = kit_config::texture_names(
        714,
        kit_config::KitSlot::P4,
        kit_config::TexturePresence {
            kit: true,
            back: true,
            ..kit_config::TexturePresence::default()
        },
    );
    let config =
        kit_config::KitConfig::template().encode_with_names(pes_version::PesVersion::Pes21, &names);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        entries["common/character0/model/character/uniform/team/714/714_DEF_4th_realUni.bin"],
        config
    );
    assert_eq!(run.exit_code(), 0);
}

/// The CPK path of the portrait file `name` (`71405.dds`, `player_71405.dds`).
fn portrait_path(name: &str) -> String {
    format!("common/render/symbol/player/{name}")
}

// TC-PRT-01
#[test]
fn dds_portraits_from_both_sources_are_emitted_as_they_are_under_the_version_s_name() {
    let sandbox = Sandbox::new("portraits_dds");
    sandbox.copy_tracer_face("exports/co Midcup Portraits/Players/05 - A");
    sandbox.write(
        "exports/co Midcup Portraits/Portraits/07.dds",
        &tracer_kit(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Portraits"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    assert_eq!(compiled_portraits(&sandbox), ["71405.dds", "71407.dds"]);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(entries[&portrait_path("71405.dds")], tracer_portrait());
    assert_eq!(entries[&portrait_path("71407.dds")], tracer_kit());

    // PES 18 names the same files with the `player_` prefix.
    let run = sandbox.run("[common]\npes_version = 18\n", &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0);
    assert_eq!(
        compiled_portraits(&sandbox),
        ["player_71405.dds", "player_71407.dds"]
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        entries[&portrait_path("player_71405.dds")],
        tracer_portrait()
    );
    assert_eq!(entries[&portrait_path("player_71407.dds")], tracer_kit());
}

// TC-ROS-05
#[test]
fn a_folder_listed_under_two_slots_emits_its_portrait_for_both() {
    let sandbox = Sandbox::new("portraits_two_slots");
    sandbox.write("exports/co Midcup Twice/players.txt", b"03 A\n07 A\n");
    sandbox.copy_tracer_face("exports/co Midcup Twice/Players/A");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0);
    assert_eq!(compiled_portraits(&sandbox), ["71403.dds", "71407.dds"]);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    for name in ["71403.dds", "71407.dds"] {
        assert_eq!(entries[&portrait_path(name)], tracer_portrait(), "{name}");
    }
}

// TC-PRT-02
#[test]
fn a_slot_s_two_portraits_skip_the_export_when_they_differ_and_are_one_when_identical() {
    let sandbox = Sandbox::new("portraits_both_sources");
    sandbox.copy_tracer_face("exports/co Midcup Both/Players/05 - A");
    sandbox.write("exports/co Midcup Both/Portraits/05.dds", &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Both"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Error portrait_conflict [DropExport] (folder_portrait=Players/05 - A/portrait.dds, portraits_file=Portraits/05.dds)",
        ]
    );
    assert_eq!(run.exit_code(), 1);
    assert!(!sandbox.root.join("output/4cc_99_test.cpk").exists());

    // The same bytes in both places are one portrait.
    sandbox.write(
        "exports/co Midcup Both/Portraits/05.dds",
        &tracer_portrait(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Both"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    assert_eq!(compiled_portraits(&sandbox), ["71405.dds"]);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(entries[&portrait_path("71405.dds")], tracer_portrait());
}

/// Asserts `dds` is a BC3 DDS of 128x128 with the full 8-level chain: what a raster portrait
/// of the fixtures' size is encoded to.
fn assert_bc3_portrait(dds: &[u8], label: &str) {
    assert!(dds.starts_with(b"DDS "), "{label}: a DDS");
    let decoded = decode(dds, SourceFormat::Dds).unwrap();
    assert_eq!(
        decoded.blocks.map(|blocks| blocks.codec),
        Some(BlockCodec::Bc3),
        "{label}"
    );
    assert_eq!(
        (decoded.width, decoded.height, decoded.mips.len()),
        (128, 128, 8),
        "{label}"
    );
}

// TC-PRT-01
#[test]
fn a_dds_portrait_passes_through_and_a_png_one_is_encoded_to_bc3_under_the_version_s_name() {
    let sandbox = Sandbox::new("portraits_dds_and_png");
    sandbox.copy_tracer_face("exports/co Midcup Portraits/Players/05 - A");
    sandbox.write(
        "exports/co Midcup Portraits/Portraits/07.png",
        &texture_fixture("portrait.png"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Portraits"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    assert_eq!(compiled_portraits(&sandbox), ["71405.dds", "71407.dds"]);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(entries[&portrait_path("71405.dds")], tracer_portrait());
    assert_bc3_portrait(&entries[&portrait_path("71407.dds")], "PES 21");

    // PES 18 names the same files with the `player_` prefix.
    let run = sandbox.run("[common]\npes_version = 18\n", &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0);
    assert_eq!(
        compiled_portraits(&sandbox),
        ["player_71405.dds", "player_71407.dds"]
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        entries[&portrait_path("player_71405.dds")],
        tracer_portrait()
    );
    assert_bc3_portrait(&entries[&portrait_path("player_71407.dds")], "PES 18");
}

/// Writes `exports/co Midcup Odd`: slot 03 the tracer's player, its portrait included, and
/// `Portraits/05.dds`, a single-level DDS of 300x300, not a power of two on either side.
fn write_odd_portrait_export(sandbox: &Sandbox) {
    sandbox.copy_tracer_face("exports/co Midcup Odd/Players/03 - A");
    sandbox.write("exports/co Midcup Odd/Portraits/05.dds", &bc1_dds(300, 300));
}

/// The lines `exports/co Midcup Odd` gets, its portrait's finding with `disposition`.
fn odd_portrait_findings(disposition: &str) -> [String; 5] {
    [
        "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=boots.fmdl, count=1662)".to_owned(),
        "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=fcl_hair.fmdl, count=1662)".to_owned(),
        "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=glove_l.fmdl, count=2)".to_owned(),
        format!("Error texture_not_pow2 [{disposition}] at Portraits/05.dds (file=05.dds)"),
        "Info export_identified [Keep] (team=/co/, id=714)".to_owned(),
    ]
}

// TC-PRT-03
#[test]
fn a_portrait_whose_side_is_not_a_power_of_two_is_checked_and_left_out_alone() {
    let sandbox = Sandbox::new("portraits_odd");
    write_odd_portrait_export(&sandbox);

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    assert_eq!(
        findings_of(&check.messages(), "co Midcup Odd"),
        odd_portrait_findings("DropFile")
    );
    assert_eq!(check.exit_code(), 1);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Odd"),
        [
            &odd_portrait_findings("DropFile")[..],
            &[TEAM_COLORS_MISSING.to_owned()]
        ]
        .concat()
    );
    assert_eq!(run.exit_code(), 1);
    assert_eq!(compiled_players(&sandbox), [71403]);
    assert_eq!(compiled_portraits(&sandbox), ["71403.dds"]);
}

// TC-PRT-03
#[test]
fn pass_through_keeps_a_portrait_whose_side_is_not_a_power_of_two() {
    let sandbox = Sandbox::new("portraits_odd_pass_through");
    write_odd_portrait_export(&sandbox);

    let run = sandbox.run(
        &pass_through_settings(&sandbox),
        &["compile", "--no-deploy"],
    );

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Odd"),
        [
            &odd_portrait_findings("Keep")[..],
            &[TEAM_COLORS_MISSING.to_owned()]
        ]
        .concat()
    );
    assert_eq!(run.exit_code(), 1);
    assert_eq!(compiled_portraits(&sandbox), ["71403.dds", "71405.dds"]);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(entries[&portrait_path("71405.dds")], bc1_dds(300, 300));
}

// TC-KIT-10
#[test]
fn a_kit_mask_on_a_fox_target_is_reported_once_and_not_emitted() {
    let sandbox = Sandbox::new("kit_mask_fox");
    sandbox.write("exports/co Midcup Mask/Kits/p1/kit.dds", &tracer_kit());
    sandbox.write("exports/co Midcup Mask/Kits/p1/kit_mask.dds", &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Mask"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info kit_texture_not_used [DropFile] at Kits/p1 (file=kit_mask.dds)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    assert_eq!(compiled_kits(&sandbox), ["u0714p1"]);
}

// TC-KIT-30
#[test]
fn a_kit_texture_stem_the_compiler_does_not_build_is_not_used() {
    let sandbox = Sandbox::new("kit_spec_fox");
    sandbox.write("exports/co Midcup Spec/Kits/p1/kit.dds", &tracer_kit());
    sandbox.write("exports/co Midcup Spec/Kits/p1/kit_spec.dds", &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Spec"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info kit_texture_not_used [DropFile] at Kits/p1 (file=kit_spec.dds)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    assert_eq!(compiled_kits(&sandbox), ["u0714p1"]);
}

// TC-KIT-31
#[test]
fn a_kit_mask_a_fox_target_does_not_emit_is_not_checked() {
    let sandbox = Sandbox::new("kit_mask_fox_renamed");
    sandbox.write("exports/co Midcup Mask/Kits/p1/kit.dds", &tracer_kit());
    // A PNG renamed `.dds`: a kit texture the target emitted would be `texture_type_mismatch`.
    sandbox.write(
        "exports/co Midcup Mask/Kits/p1/kit_mask.dds",
        &texture_fixture("portrait.png"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Mask"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info kit_texture_not_used [DropFile] at Kits/p1 (file=kit_mask.dds)",
            "Info team_colors_missing [Keep] ()",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0);
    assert_eq!(compiled_kits(&sandbox), ["u0714p1"]);
}

// TC-STR-01
#[test]
fn content_nested_one_folder_down_compiles_as_if_at_the_root() {
    let sandbox = Sandbox::new("str_nested");
    sandbox.write("exports/co Midcup Wrapped/notes.txt", b"Notes.\n");
    sandbox.copy_tracer_face("exports/co Midcup Wrapped/wrapper/Players/03 - A");
    sandbox.copy_tracer_face("exports/dbg Midcup Doubled/Players/Players/03 - A");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Wrapped"),
        [
            "Warning nested_folders_fixed [Keep] (folder=wrapper)",
            "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=glove_l.fmdl, count=2)",
            "Info notes_found [Keep] at notes.txt ()",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(
        findings_of(&lines, "dbg Midcup Doubled"),
        [
            "Warning nested_folders_fixed [Keep] at Players (folder=Players/Players)",
            "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=glove_l.fmdl, count=2)",
            "Info export_identified [Keep] (team=/dbg/, id=790)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(compiled_players(&sandbox), [71403, 79003]);
    assert_eq!(run.exit_code(), 0);
}

// TC-ROOT-02
#[test]
fn pass_through_drops_notes_that_are_not_utf8_and_compiles_the_export() {
    let sandbox = Sandbox::new("root_notes_encoding");
    sandbox.copy_tracer("egg Midcup Tracer");
    sandbox.write("exports/egg Midcup Tracer/notes.txt", b"caf\xe9\n");

    let run = sandbox.run(
        &pass_through_settings(&sandbox),
        &["compile", "--no-deploy"],
    );

    assert_eq!(
        findings_of(&run.messages(), "egg Midcup Tracer"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - The Chad Stormworks Player (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - The Chad Stormworks Player (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - The Chad Stormworks Player (file=glove_l.fmdl, count=2)",
            "Error notes_encoding_invalid [DropFile] at notes.txt ()",
            "Info export_identified [Keep] (team=/egg/, id=792)"
        ]
    );
    assert_eq!(compiled_players(&sandbox), [79205]);
    assert_eq!(run.exit_code(), 1);
}

/// Where `compile` collects the notes, in `sandbox`'s output folder, spelled as a finding
/// names it.
fn teamnotes(sandbox: &Sandbox) -> std::path::PathBuf {
    sandbox.root.join("output").join("teamnotes.txt")
}

/// The export `exports/<name>`: one player that compiles with no finding, and a root
/// `notes.txt` of `notes` when given.
fn export_with_notes(sandbox: &Sandbox, name: &str, notes: Option<&[u8]>) {
    sandbox.write(&format!("exports/{name}/{CLEAN_PLAYER}"), &clean_model());
    if let Some(notes) = notes {
        sandbox.write(&format!("exports/{name}/notes.txt"), notes);
    }
}

/// The export `exports/b Midcup Skipped`, with a root `notes.txt`, which validation skips: its
/// `players.txt` maps slot 03 twice.
fn skipped_export_with_notes(sandbox: &Sandbox) {
    sandbox.write("exports/b Midcup Skipped/players.txt", b"03 A\n03 B\n");
    sandbox.write(
        "exports/b Midcup Skipped/Players/A/face_high.fmdl",
        &clean_model(),
    );
    sandbox.write(
        "exports/b Midcup Skipped/Players/B/face_high.fmdl",
        &clean_model(),
    );
    sandbox.write("exports/b Midcup Skipped/notes.txt", b"Not compiled.\n");
}

// TC-ROOT-09
#[test]
fn compile_collects_the_compiled_exports_notes_into_teamnotes_txt_and_removes_a_stale_one() {
    let sandbox = Sandbox::new("root_teamnotes");
    export_with_notes(&sandbox, "co Midcup Notes", Some(b"Hello"));
    export_with_notes(
        &sandbox,
        "a Midcup Notes",
        Some(b"First line\r\nsecond line\r\n"),
    );
    skipped_export_with_notes(&sandbox);
    let cpk = sandbox.root.join("output/4cc_99_test.cpk");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let mut expected = no_deploy_lines(
        &sandbox,
        [
            "a Midcup Notes: Info notes_found [Keep] at notes.txt ()",
            "a Midcup Notes: Info export_identified [Keep] (team=/a/, id=702)",
            "b Midcup Skipped: Error players_txt_slot_duplicate [DropExport] at players.txt line 2 slot Some(3) ()",
            "b Midcup Skipped: Info notes_found [Keep] at notes.txt ()",
            "co Midcup Notes: Info notes_found [Keep] at notes.txt ()",
            "co Midcup Notes: Info export_identified [Keep] (team=/co/, id=714)",
            "a Midcup Notes: Info team_colors_missing [Keep] ()",
            "co Midcup Notes: Info team_colors_missing [Keep] ()",
        ],
    );
    expected.push(patch_written(&sandbox, "702, 714"));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 1);
    assert_eq!(compiled_players(&sandbox), [70203, 71403]);
    assert_eq!(
        fs::read_to_string(teamnotes(&sandbox)).unwrap(),
        "--- /a/ ---\nFirst line\nsecond line\n\n--- /co/ ---\nHello\n"
    );

    // Compiled again without the notes: the file left over would show notes no export of
    // this run has. The CPK is removed first, so the run is seen writing its own.
    fs::remove_file(sandbox.root.join("exports/co Midcup Notes/notes.txt")).unwrap();
    fs::remove_file(sandbox.root.join("exports/a Midcup Notes/notes.txt")).unwrap();
    fs::remove_file(&cpk).unwrap();

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 1);
    assert!(cpk.is_file());
    assert!(!teamnotes(&sandbox).exists());

    let fresh = Sandbox::new("root_teamnotes_none");
    export_with_notes(&fresh, "co Midcup Plain", None);

    let run = fresh.run(&pes21_settings(&fresh), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0);
    assert!(fresh.root.join("output/4cc_99_test.cpk").is_file());
    assert!(!teamnotes(&fresh).exists());
}

// TC-ROOT-12
#[test]
fn a_compile_that_writes_no_cpk_leaves_the_previous_teamnotes_txt() {
    let sandbox = Sandbox::new("root_teamnotes_no_cpk");
    skipped_export_with_notes(&sandbox);
    sandbox.write("output/teamnotes.txt", b"--- /co/ ---\nPrevious.\n");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 1);
    assert!(!sandbox.root.join("output/4cc_99_test.cpk").exists());
    assert_eq!(
        fs::read(teamnotes(&sandbox)).unwrap(),
        b"--- /co/ ---\nPrevious.\n"
    );
}

// TC-ROOT-11
#[test]
fn a_teamnotes_txt_that_cannot_be_written_is_reported_and_the_cpk_stays() {
    let sandbox = Sandbox::new("root_teamnotes_write_failed");
    export_with_notes(&sandbox, "co Midcup Notes", Some(b"Hello"));
    // A folder in the file's place, with a file in it so no platform replaces it.
    sandbox.write("output/teamnotes.txt/kept", b"");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    let (last, first) = lines.split_last().unwrap();
    let mut before_last = no_deploy_lines(
        &sandbox,
        [
            "co Midcup Notes: Info notes_found [Keep] at notes.txt ()",
            "co Midcup Notes: Info export_identified [Keep] (team=/co/, id=714)",
            "co Midcup Notes: Info team_colors_missing [Keep] ()",
        ],
    );
    before_last.push(patch_written(&sandbox, "714"));
    assert_eq!(first, before_last);
    // The error ends with the platform's own text, so only its shape is fixed.
    let prefix = format!(
        "Error teamnotes_write_failed [Keep] (path={path}, error=cannot replace {path}: ",
        path = teamnotes(&sandbox).display()
    );
    assert!(last.starts_with(&prefix) && last.ends_with(')'), "{last}");
    assert_eq!(run.exit_code(), 1);
    assert_eq!(compiled_players(&sandbox), [71403]);
    assert!(teamnotes(&sandbox).join("kept").is_file());
}

/// The bundled `TeamColor.bin`, which a run with no installed bin builds on.
pub(crate) fn bundled_team_color() -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resources/bins/TeamColor.bin"))
        .unwrap()
}

/// Where the CPK keeps `TeamColor.bin`.
pub(crate) const TEAM_COLOR: &str = "common/etc/TeamColor.bin";

// TC-ROOT-10
#[test]
fn a_root_colors_txt_sets_the_team_s_colors_and_an_export_without_one_reports_it() {
    let sandbox = Sandbox::new("root_team_colors");
    sandbox.write("exports/co Midcup Colors/colors.txt", b"#c11200\n#414141\n");
    sandbox.write("exports/co Midcup Colors/Kits/p1/kit.dds", &tracer_kit());
    sandbox.write("elsewhere/dbg Midcup Plain/Kits/p1/kit.dds", &tracer_kit());
    let cpk = sandbox.root.join("output/4cc_99_test.cpk");
    let base = bundled_team_color();

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        run.messages(),
        no_deploy_lines(
            &sandbox,
            ["co Midcup Colors: Info export_identified [Keep] (team=/co/, id=714)",]
        )
    );
    assert_eq!(run.exit_code(), 0);
    // Team 714's record: its ID and count, then its first two colors; the rest is the base's.
    let record = (714 - 100) * 16;
    let mut expected = base.clone();
    expected[record + 4..record + 10].copy_from_slice(&[0xc1, 0x12, 0x00, 0x41, 0x41, 0x41]);
    assert!(
        cpk_entries(&cpk).get(TEAM_COLOR) == Some(&expected),
        "TeamColor.bin is the base with team 714's colors set"
    );

    let run = sandbox.run(
        &pes21_settings(&sandbox),
        &[
            "compile",
            "--no-deploy",
            "--export",
            &sandbox.arg("elsewhere/dbg Midcup Plain"),
        ],
    );

    assert_eq!(
        run.messages(),
        no_deploy_lines(
            &sandbox,
            [
                "dbg Midcup Plain: Info export_identified [Keep] (team=/dbg/, id=790)",
                "dbg Midcup Plain: Info team_colors_missing [Keep] ()",
            ]
        )
    );
    assert_eq!(run.exit_code(), 0);
    assert!(
        cpk_entries(&cpk).get(TEAM_COLOR) == Some(&base),
        "TeamColor.bin is the bundled base"
    );
}

/// The bundled `UniColor.bin`, which a run with no installed bin builds on.
pub(crate) fn bundled_uni_color() -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resources/bins/UniColor.bin"))
        .unwrap()
}

/// Where the CPK keeps `UniColor.bin`.
pub(crate) const UNI_COLOR: &str = "common/character0/model/character/uniform/team/UniColor.bin";

/// Team `team_id`'s 85-byte record in the `UniColor.bin` `bin`: the `u32` team ID, the kit
/// count, then ten 8-byte entries (kit number, icon, two colors).
pub(crate) fn uni_record(bin: &[u8], team_id: usize) -> &[u8] {
    let start = (team_id - 100) * 85;
    &bin[start..start + 85]
}

/// `bin` with team `team_id`'s `UniColor.bin` record replaced by `record`.
pub(crate) fn with_uni_record(bin: &[u8], team_id: usize, record: &[u8]) -> Vec<u8> {
    let start = (team_id - 100) * 85;
    let mut replaced = bin.to_vec();
    replaced[start..start + 85].copy_from_slice(record);
    replaced
}

/// The two menu colors `extract_kit_colors` gives for the tracer's `kit.dds`, decoded, as the
/// six bytes a `UniColor.bin` entry holds them in.
pub(crate) fn tracer_kit_colors() -> [u8; 6] {
    let decoded = decode(&tracer_kit(), SourceFormat::Dds).unwrap();
    let colors =
        color_tools::kit::extract_kit_colors(&decoded.mips[0], decoded.width, decoded.height)
            .expect("the tracer's kit texture gives colors");
    let [r1, g1, b1] = colors.color1;
    let [r2, g2, b2] = colors.color2;
    [r1, g1, b1, r2, g2, b2]
}

/// The bundled `UniformParameter.bin` base PES 19 to 21 build on.
pub(crate) fn bundled_uniform_parameter() -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../resources/bins/UniformParameter19.bin"),
    )
    .unwrap()
}

/// Where the CPK keeps `UniformParameter.bin`.
pub(crate) const UNIFORM_PARAMETER: &str =
    "common/character0/model/character/uniform/team/UniformParameter.bin";

/// The names of the entries in which the `UniformParameter.bin` `ours` differs from `base`: an
/// entry added, removed or with other bytes.
fn uniform_parameter_changes(ours: &[u8], base: &[u8]) -> Vec<String> {
    let ours = uniparam::UniformParameter::read(ours).unwrap();
    let base = uniparam::UniformParameter::read(base).unwrap();
    let mut names: Vec<String> = ours
        .entries()
        .filter(|(name, bytes)| base.get(name) != Some(*bytes))
        .map(|(name, _)| name.to_owned())
        .collect();
    names.extend(
        base.entries()
            .filter(|(name, _)| ours.get(name).is_none())
            .map(|(name, _)| name.to_owned()),
    );
    names
}

// TC-KIT-11
// TC-BIN-01
#[test]
fn each_kit_s_menu_colors_come_from_its_colors_txt_its_texture_or_the_missing_pair() {
    let sandbox = Sandbox::new("kit_colors");
    let export = "exports/co Full Colors";
    sandbox.write(&format!("{export}/colors.txt"), b"#c11200\n#414141\n");
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{export}/Kits/p1/colors.txt"),
        b"#0a1b2c\n#d4e5f6\n",
    );
    sandbox.write(&format!("{export}/Kits/p2/kit.dds"), &tracer_kit());
    fs::create_dir_all(sandbox.root.join(format!("{export}/Kits/p3"))).unwrap();

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Full Colors"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info kit_config_generated [Keep] at Kits/p1 ()",
            "Info kit_config_generated [Keep] at Kits/p2 ()",
            "Info kit_config_generated [Keep] at Kits/p3 ()",
            "Info kit_placeholder [Keep] at Kits/p3 ()",
            "Info kit_config_generated [Keep] at Kits/g1 ()",
            "Info kit_placeholder [Keep] at Kits/g1 ()",
            "Info kit_colors_derived [Keep] at Kits/p2 ()",
            "Warning kit_colors_missing [Keep] at Kits/p3 ()",
            "Warning kit_colors_missing [Keep] at Kits/g1 ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));

    // Team 714's record is rebuilt from the export's kits alone: kits 0 to 2 (p1 to p3)
    // and the goalkeeper kit a `Full` export always has (the empty g1 it plans), then
    // unused entries; every other record is the base's.
    let base = bundled_uni_color();
    let mut record = vec![0xca, 0x02, 0x00, 0x00, 4];
    record.extend([0x00, 0x03, 0x0a, 0x1b, 0x2c, 0xd4, 0xe5, 0xf6]);
    record.extend([0x01, 0x03]);
    record.extend(tracer_kit_colors());
    record.extend([0x02, 0x03, 0xff, 0x00, 0xff, 0x00, 0x00, 0x00]);
    record.extend([0x10, 0x03, 0xff, 0x00, 0xff, 0x00, 0x00, 0x00]);
    for _ in 4..10 {
        record.extend([0xff, 0, 0, 0, 0, 0, 0, 0]);
    }
    assert!(
        entries[UNI_COLOR] == with_uni_record(&base, 714, &record),
        "UniColor.bin is the base with team 714's record rebuilt from p1 to p3 and g1"
    );

    // The other two bins differ from their bases in team 714's entries alone.
    let mut team_color = bundled_team_color();
    let team = (714 - 100) * 16;
    team_color[team + 4..team + 10].copy_from_slice(&[0xc1, 0x12, 0x00, 0x41, 0x41, 0x41]);
    assert!(
        entries[TEAM_COLOR] == team_color,
        "TeamColor.bin is the base with team 714's colors set"
    );
    let changed =
        uniform_parameter_changes(&entries[UNIFORM_PARAMETER], &bundled_uniform_parameter());
    assert_eq!(
        changed,
        [
            "714_DEF_1st_realUni.bin",
            "714_DEF_2nd_realUni.bin",
            "714_DEF_3rd_realUni.bin",
            "714_DEF_GK1st_realUni.bin",
            "714_DEF_4th_realUni.bin",
            "714_DEF_5th_realUni.bin",
            "714_DEF_6th_realUni.bin",
            "714_DEF_7th_realUni.bin",
            "714_DEF_8th_realUni.bin",
            "714_DEF_9th_realUni.bin",
        ],
        "the four kits' generated configs written, p4 to p9 removed"
    );
}

// TC-KIT-25
#[test]
fn a_kit_without_a_texture_derives_its_colors_from_the_inherited_all_kit_dds() {
    let sandbox = Sandbox::new("kit_colors_inherited");
    let export = "exports/co Full Shared";
    sandbox.write(&format!("{export}/Kits/all/kit.dds"), &tracer_kit());
    fs::create_dir_all(sandbox.root.join(format!("{export}/Kits/p2"))).unwrap();

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Full Shared"),
        [
            "Info kit_textures_inherited [Keep] at Kits/p2 (stems=kit)",
            "Info kit_textures_inherited [Keep] at Kits/g1 (stems=kit)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info kit_config_generated [Keep] at Kits/p2 ()",
            "Info kit_config_generated [Keep] at Kits/g1 ()",
            "Info kit_colors_derived [Keep] at Kits/p2 ()",
            "Info kit_colors_derived [Keep] at Kits/g1 ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    // The inherited `all/kit.dds` is each kit's effective main texture, so neither is a
    // placeholder and each derives its colors.
    assert_eq!(compiled_kits(&sandbox), ["u0714g1", "u0714p2"]);
    // p2's entry: kit number 1, icon 3, the pair the extraction gives for all/kit.dds.
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let record = uni_record(&entries[UNI_COLOR], 714);
    let mut entry = vec![0x01, 0x03];
    entry.extend_from_slice(&tracer_kit_colors());
    assert_eq!(record[5..13], entry);
}

// TC-KIT-25
#[test]
fn an_all_kit_texture_that_does_not_decode_drops_each_kit_inheriting_it_and_not_the_others() {
    let sandbox = Sandbox::new("kit_inherited_undecodable");
    let export = "exports/co Midcup Shared";
    // A PNG under a `.dds` name: the deep pass reads its header as another type's.
    sandbox.write(
        &format!("{export}/Kits/all/kit.dds"),
        &texture_fixture("kit.png"),
    );
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    fs::create_dir_all(sandbox.root.join(format!("{export}/Kits/p2"))).unwrap();

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    // The finding is p2's, the kit the file reaches, and none is at `Kits/all`.
    assert_eq!(
        findings_of(&run.messages(), "co Midcup Shared"),
        [
            "Error texture_type_mismatch [DropFolder] at Kits/p2 (file=Kits/all/kit.dds)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert_eq!(run.exit_code(), 1);
    assert_eq!(compiled_kits(&sandbox), ["u0714p1"]);
}

// TC-KIT-12
#[test]
fn a_kit_colors_txt_with_one_valid_color_reports_its_bad_line_and_derives_both_colors() {
    let sandbox = Sandbox::new("kit_colors_one_valid");
    let export = "exports/co Full Colors";
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{export}/Kits/p1/colors.txt"),
        b"#0a1b2c\nnot a color\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Full Colors"),
        [
            "Warning color_entry_invalid [Keep] at Kits/p1/colors.txt (line=2, reason=not one color)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info kit_config_generated [Keep] at Kits/p1 ()",
            "Info kit_config_generated [Keep] at Kits/g1 ()",
            "Info kit_placeholder [Keep] at Kits/g1 ()",
            "Info kit_colors_derived [Keep] at Kits/p1 ()",
            "Warning kit_colors_missing [Keep] at Kits/g1 ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let record = uni_record(&entries[UNI_COLOR], 714);
    assert_eq!(record[5..7], [0x00, 0x03]);
    assert_eq!(record[7..13], tracer_kit_colors());
}

// TC-KIT-13
#[test]
fn a_kit_s_icon_marker_gives_its_menu_icon_and_a_kit_without_one_gets_icon_3() {
    // Team 790's base record is a placeholder whose entries carry icon 0, so icon 3 is
    // written, not kept.
    let sandbox = Sandbox::new("kit_icons");
    let export = "exports/dbg Full Icons";
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(&format!("{export}/Kits/p1/icon_7"), b"");
    sandbox.write(&format!("{export}/Kits/p2/kit.dds"), &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let base = bundled_uni_color();
    assert_eq!(
        uni_record(&base, 790)[5..7],
        [0x00, 0x00],
        "the base's icon 0"
    );
    // The export holds no `g1/`: a `Full` export plans an empty one, so the record is
    // rebuilt as p1, p2, g1 (icons 7, 3 and the placeholder's 3).
    let record = uni_record(&entries[UNI_COLOR], 790);
    assert_eq!(record[4], 3, "three kits");
    assert_eq!(record[5..7], [0x00, 0x07], "p1: icon 7");
    assert_eq!(record[13..15], [0x01, 0x03], "p2: icon 3");
    assert_eq!(record[21..23], [0x10, 0x03], "g1: icon 3");
}

// TC-KIT-14
#[test]
fn a_kit_whose_task_fails_keeps_its_base_entries_and_the_kit_beside_it_commits() {
    let sandbox = Sandbox::new("kit_colors_failed_task");
    let export = "exports/co Midcup Fails";
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{export}/Kits/p1/colors.txt"),
        b"#0a1b2c\n#d4e5f6\n",
    );
    // A config the bundled base does not hold: its entry is written.
    sandbox.write(&format!("{export}/Kits/p1/config.toml"), SHIRT_144);
    // A header the deep pass reads, over pixel data cut short: the kit's task fails.
    sandbox.write(&format!("{export}/Kits/p2/kit.dds"), &pixels_cut_dds());
    sandbox.write(
        &format!("{export}/Kits/p2/colors.txt"),
        b"#111111\n#222222\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    let findings = findings_of(&lines, "co Midcup Fails");
    // p1 compiles, p2's task failed: the only findings are the export's and the failure.
    let [identified, missing, failed] = findings.as_slice() else {
        panic!("{findings:#?}");
    };
    assert_eq!(
        [*identified, *missing],
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert!(
        failed.starts_with(
            "Error folder_pack_failed [DropFolder] at Kits/p2 (error=cannot convert kit.dds"
        ),
        "{failed}"
    );
    assert_eq!(run.exit_code(), 1);
    assert_eq!(compiled_kits(&sandbox), ["u0714p1"]);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));

    // p1's entry is written, p2's is the base's.
    let base = bundled_uni_color();
    let mut record = uni_record(&base, 714).to_vec();
    record[5..13].copy_from_slice(&[0x00, 0x03, 0x0a, 0x1b, 0x2c, 0xd4, 0xe5, 0xf6]);
    assert!(
        entries[UNI_COLOR] == with_uni_record(&base, 714, &record),
        "UniColor.bin is the base with team 714's p1 entry set"
    );
    // p1's config is the emitted one (it differs from the bundled base's), p2's the base's.
    let bin = uniparam::UniformParameter::read(&entries[UNIFORM_PARAMETER]).unwrap();
    let base_bin = uniparam::UniformParameter::read(&bundled_uniform_parameter()).unwrap();
    assert_ne!(
        entries[&config_path("1st")].as_slice(),
        base_bin.get("714_DEF_1st_realUni.bin").unwrap(),
        "p1's emitted config differs from the base's, so its entry is new"
    );
    assert_eq!(
        bin.get("714_DEF_1st_realUni.bin"),
        Some(entries[&config_path("1st")].as_slice()),
        "p1 is the compiled config"
    );
    assert_eq!(
        bin.get("714_DEF_2nd_realUni.bin"),
        base_bin.get("714_DEF_2nd_realUni.bin"),
        "p2 is the installed config"
    );
}

// TC-KIT-33
#[test]
fn a_midcup_kit_keeps_the_installed_config_textures_and_colors_it_does_not_resend() {
    // The installed game holds team 714's configs and textures for p1 to p3, p1's a
    // different image than the export's, so a copied file would show.
    let sandbox = Sandbox::new("kit_midcup_keeps_installed");
    install_names(
        &sandbox,
        &["4cc_08_bins.cpk", "4cc_61_midcup.cpk", "4cc_99_test.cpk"],
    );
    let ftex = ftex::dds_to_ftex(&tracer_kit(), ftex::ColorSpace::Normal).unwrap();
    let installed_p1 = ftex::dds_to_ftex(&bc1_dds(64, 64), ftex::ColorSpace::Normal).unwrap();
    assert_ne!(installed_p1, ftex);
    install_cpk(
        &sandbox,
        "4cc_08_bins.cpk",
        &[
            (TEAM_COLOR, &bundled_team_color()),
            (UNI_COLOR, &uni_color_p1_p2_p3()),
            (
                UNIFORM_PARAMETER,
                &installed_configs(&[(714, KitSlot::P1), (714, KitSlot::P2), (714, KitSlot::P3)]),
            ),
            (kit_texture("u0714p1").as_str(), installed_p1.as_slice()),
            (kit_texture("u0714p2").as_str(), ftex.as_slice()),
            (kit_texture("u0714p3").as_str(), ftex.as_slice()),
        ],
    );
    let export = "exports/co Midcup Keep";
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{export}/Kits/p1/colors.txt"),
        b"#0a1b2c\n#d4e5f6\n",
    );
    sandbox.write(
        &format!("{export}/Kits/p2/colors.txt"),
        b"#111111\n#222222\n",
    );
    sandbox.write(
        &format!("{export}/Kits/p3/config.toml"),
        &fs::read(Path::new(&tracer_export()).join("Kits/g1/config.toml")).unwrap(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Keep"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // p1's kit texture and p3's config are the only kit files: p1's config, p2's config
    // and p2's and p3's textures stay the installed ones.
    assert_eq!(compiled_kits(&sandbox), ["u0714p1"]);
    assert_eq!(
        entries[&kit_texture("u0714p1")],
        ftex,
        "p1's texture is the resent one's conversion, not the installed"
    );
    assert!(entries.contains_key(&config_path("3rd")));
    assert!(!entries.contains_key(&config_path("1st")));
    assert!(!entries.contains_key(&config_path("2nd")));

    // p1's and p2's entries are the installed configs', p3's is the emitted one.
    let bin = uniparam::UniformParameter::read(&entries[UNIFORM_PARAMETER]).unwrap();
    assert_eq!(
        bin.get("714_DEF_1st_realUni.bin"),
        Some(shirt_144_config(PesVersion::Pes21, 714, KitSlot::P1, false).as_slice()),
        "p1 is the installed config"
    );
    assert_eq!(
        bin.get("714_DEF_2nd_realUni.bin"),
        Some(shirt_144_config(PesVersion::Pes21, 714, KitSlot::P2, false).as_slice()),
        "p2 is the installed config"
    );
    assert_eq!(
        bin.get("714_DEF_3rd_realUni.bin"),
        Some(entries[&config_path("3rd")].as_slice()),
        "p3 is the compiled config"
    );
    // p3's emitted config is its `config.toml` encoded for the slot, its `kit` name
    // alone (the install holds none of its number textures).
    let expected = kit_config::KitConfig::from_toml(
        &fs::read_to_string(Path::new(&tracer_export()).join("Kits/g1/config.toml")).unwrap(),
    )
    .unwrap()
    .encode_with_names(
        PesVersion::Pes21,
        &kit_config::texture_names(
            714,
            KitSlot::P3,
            kit_config::TexturePresence {
                kit: true,
                ..kit_config::TexturePresence::default()
            },
        ),
    );
    assert_ne!(
        expected.to_vec(),
        shirt_144_config(PesVersion::Pes21, 714, KitSlot::P3, false),
        "the emitted config differs from the installed p3 config, so its entry is new"
    );
    assert_eq!(entries[&config_path("3rd")], expected.to_vec());

    // p1's and p2's colors are written, p3's the installed one.
    let record = uni_record(&entries[UNI_COLOR], 714);
    assert_eq!(
        record[5..13],
        [0x00, 0x03, 0x0a, 0x1b, 0x2c, 0xd4, 0xe5, 0xf6],
        "p1's colors.txt"
    );
    assert_eq!(
        record[13..21],
        [0x01, 0x03, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22],
        "p2's colors.txt"
    );
    assert_eq!(
        record[21..29],
        uni_record(&uni_color_p1_p2_p3(), 714)[21..29],
        "p3 is the installed entry"
    );
}

// TC-KIT-35
#[test]
fn a_midcup_kit_s_config_names_the_number_textures_the_install_holds() {
    // Team 714's installed CPK holds p1's textures `u0714p1`, `u0714p1_back` and
    // `u0714p1_name`, and p1's config.
    let sandbox = Sandbox::new("kit_midcup_config_names");
    install_names(
        &sandbox,
        &["4cc_08_bins.cpk", "4cc_61_midcup.cpk", "4cc_99_test.cpk"],
    );
    let ftex = ftex::dds_to_ftex(&tracer_kit(), ftex::ColorSpace::Normal).unwrap();
    install_cpk(
        &sandbox,
        "4cc_08_bins.cpk",
        &[
            (TEAM_COLOR, &bundled_team_color()),
            (UNI_COLOR, &uni_color_p1_p2_p3()),
            (UNIFORM_PARAMETER, &installed_configs(&[(714, KitSlot::P1)])),
            (kit_texture("u0714p1").as_str(), ftex.as_slice()),
            (kit_texture("u0714p1_back").as_str(), ftex.as_slice()),
            (kit_texture("u0714p1_name").as_str(), ftex.as_slice()),
        ],
    );
    let export = "exports/co Midcup Keep";
    let config_path_src = Path::new(&tracer_export()).join("Kits/g1/config.toml");
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{export}/Kits/p1/config.toml"),
        &fs::read(&config_path_src).unwrap(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // The emitted config names the installed `u0714p1_back` and `u0714p1_name` its folder
    // does not resend: the chest and leg fields empty.
    let names = kit_config::texture_names(
        714,
        KitSlot::P1,
        kit_config::TexturePresence {
            kit: true,
            back: true,
            name: true,
            ..kit_config::TexturePresence::default()
        },
    );
    let expected = kit_config::KitConfig::from_toml(&fs::read_to_string(&config_path_src).unwrap())
        .unwrap()
        .encode_with_names(pes_version::PesVersion::Pes21, &names);
    // The expected config names `u0714p1_back` and `u0714p1_name`, so it differs from the
    // same config encoded with the main texture's name alone.
    let main_only =
        kit_config::KitConfig::from_toml(&fs::read_to_string(&config_path_src).unwrap())
            .unwrap()
            .encode_with_names(
                pes_version::PesVersion::Pes21,
                &kit_config::texture_names(
                    714,
                    KitSlot::P1,
                    kit_config::TexturePresence {
                        kit: true,
                        ..kit_config::TexturePresence::default()
                    },
                ),
            );
    assert_ne!(expected.to_vec(), main_only.to_vec());
    assert_eq!(
        entries[&config_path("1st")],
        expected.to_vec(),
        "the config's five texture names: u0714p1, u0714p1_back, empty, empty, u0714p1_name"
    );
}

// TC-KIT-36
#[test]
fn a_midcup_kit_without_usable_colors_keeps_its_installed_menu_colors() {
    // The installed game holds team 714's configs, textures and `UniColor.bin` entries
    // for p1 to p3, with colors no extraction of the export's `kit.dds` gives.
    let sandbox = Sandbox::new("kit_midcup_keeps_colors");
    install_names(
        &sandbox,
        &["4cc_08_bins.cpk", "4cc_61_midcup.cpk", "4cc_99_test.cpk"],
    );
    let ftex = ftex::dds_to_ftex(&tracer_kit(), ftex::ColorSpace::Normal).unwrap();
    install_cpk(
        &sandbox,
        "4cc_08_bins.cpk",
        &[
            (TEAM_COLOR, &bundled_team_color()),
            (UNI_COLOR, &uni_color_p1_p2_p3()),
            (
                UNIFORM_PARAMETER,
                &installed_configs(&[(714, KitSlot::P1), (714, KitSlot::P2), (714, KitSlot::P3)]),
            ),
            (kit_texture("u0714p1").as_str(), ftex.as_slice()),
            (kit_texture("u0714p2").as_str(), ftex.as_slice()),
            (kit_texture("u0714p3").as_str(), ftex.as_slice()),
        ],
    );
    let export = "exports/co Midcup Keep";
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{export}/Kits/p1/config.toml"),
        &fs::read(Path::new(&tracer_export()).join("Kits/g1/config.toml")).unwrap(),
    );
    sandbox.write(&format!("{export}/Kits/p2/kit.dds"), &tracer_kit());
    sandbox.write(&format!("{export}/Kits/p2/colors.txt"), b"not a color\n");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    // The line is still refused, but no `kit_colors_derived` is reported: each kit's
    // installed `UniColor.bin` entry stands.
    assert_eq!(
        findings_of(&run.messages(), "co Midcup Keep"),
        [
            "Warning color_entry_invalid [Keep] at Kits/p2/colors.txt (line=1, reason=not one color)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        uni_record(&entries[UNI_COLOR], 714),
        uni_record(&uni_color_p1_p2_p3(), 714),
        "team 714's whole record is the installed one"
    );
}

// TC-BIN-42
#[test]
fn a_midcup_run_whose_only_work_is_a_root_colors_txt_still_writes_team_color_bin() {
    // Team 714's installed CPK as the kit test's: its `TeamColor.bin` record holds its
    // own bytes until a root `colors.txt` is compiled.
    let sandbox = Sandbox::new("bin_midcup_root_colors");
    install_names(
        &sandbox,
        &["4cc_08_bins.cpk", "4cc_61_midcup.cpk", "4cc_99_test.cpk"],
    );
    install_cpk(
        &sandbox,
        "4cc_08_bins.cpk",
        &[
            (TEAM_COLOR, &bundled_team_color()),
            (UNI_COLOR, &uni_color_p1_p2_p3()),
            (
                UNIFORM_PARAMETER,
                &installed_configs(&[(714, KitSlot::P1), (714, KitSlot::P2), (714, KitSlot::P3)]),
            ),
        ],
    );
    sandbox.write(
        "exports/co Midcup Colors/colors.txt",
        b"#c11200\n#414141\n#020304\n#050607\n",
    );
    // `export_empty` drops an export with no content folders, and a root `colors.txt`
    // alone is none: an empty kit folder is the smallest content it can compile.
    fs::create_dir_all(sandbox.root.join("exports/co Midcup Colors/Kits/p2")).unwrap();

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    // The root file is the run's whole work beside the bins, and they are written.
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(entries.contains_key(TEAM_COLOR), "the bins CPK exists");
    let record = (714 - 100) * 16;
    assert_eq!(
        entries[TEAM_COLOR][record..record + 16],
        [
            0xca, 0x02, 0x04, 0x00, 0xc1, 0x12, 0x00, 0x41, 0x41, 0x41, 0x02, 0x03, 0x04, 0x05,
            0x06, 0x07
        ],
        "team 714's record: its ID, count 4, then the four colors"
    );
}

// TC-BIN-42
#[test]
fn a_midcup_run_whose_only_work_is_a_colors_only_kit_still_writes_its_bins() {
    // The installed game holds team 714's configs and textures for p1 to p3, as
    // `a_midcup_kit_keeps_the_installed_config_textures_and_colors_it_does_not_resend`'s.
    let sandbox = Sandbox::new("kit_midcup_colors_only");
    install_names(
        &sandbox,
        &["4cc_08_bins.cpk", "4cc_61_midcup.cpk", "4cc_99_test.cpk"],
    );
    let ftex = ftex::dds_to_ftex(&tracer_kit(), ftex::ColorSpace::Normal).unwrap();
    install_cpk(
        &sandbox,
        "4cc_08_bins.cpk",
        &[
            (TEAM_COLOR, &bundled_team_color()),
            (UNI_COLOR, &uni_color_p1_p2_p3()),
            (
                UNIFORM_PARAMETER,
                &installed_configs(&[(714, KitSlot::P1), (714, KitSlot::P2), (714, KitSlot::P3)]),
            ),
            (kit_texture("u0714p1").as_str(), ftex.as_slice()),
            (kit_texture("u0714p2").as_str(), ftex.as_slice()),
            (kit_texture("u0714p3").as_str(), ftex.as_slice()),
        ],
    );
    sandbox.write(
        "exports/co Midcup Colors/Kits/p2/colors.txt",
        b"#111111\n#222222\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    // A `UniColor.bin` entry is all the colors-only kit committed, but the CPK exists:
    // the bins are written with it.
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(entries.contains_key(UNI_COLOR), "the bins CPK exists");
    let record = uni_record(&entries[UNI_COLOR], 714);
    assert_eq!(
        record[13..21],
        [0x01, 0x03, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22],
        "p2's colors.txt"
    );
    let installed = uni_color_p1_p2_p3();
    assert_eq!(
        record[5..13],
        uni_record(&installed, 714)[5..13],
        "p1 stays installed"
    );
    assert_eq!(
        record[21..29],
        uni_record(&installed, 714)[21..29],
        "p3 stays installed"
    );
}

/// The `UniColor.bin` entry of kit `kit` with icon 3 and the six color bytes `colors`.
fn kit_entry(kit: u8, colors: [u8; 6]) -> [u8; 8] {
    let mut entry = [kit, 0x03, 0, 0, 0, 0, 0, 0];
    entry[2..].copy_from_slice(&colors);
    entry
}

/// A placeholder kit's colors: magenta, then black.
const PLACEHOLDER_COLORS: [u8; 6] = [0xff, 0x00, 0xff, 0x00, 0x00, 0x00];

/// Team `team_id`'s 85-byte `UniColor.bin` record holding `entries` under the kit count
/// `count`, the rest of its ten entries unused.
pub(crate) fn record_of(team_id: u32, count: u8, entries: &[[u8; 8]]) -> Vec<u8> {
    let mut record = team_id.to_le_bytes().to_vec();
    record.push(count);
    for index in 0..10 {
        match entries.get(index) {
            Some(entry) => record.extend(entry),
            None => record.extend([0xff, 0, 0, 0, 0, 0, 0, 0]),
        }
    }
    record
}

/// The entry `index` of team `team_id`'s record in the `UniColor.bin` `bin`.
fn base_entry(bin: &[u8], team_id: usize, index: usize) -> [u8; 8] {
    uni_record(bin, team_id)[5 + index * 8..13 + index * 8]
        .try_into()
        .unwrap()
}

// TC-BIN-14
#[test]
fn a_full_export_rebuilds_its_team_s_record_from_its_kits_and_a_failed_kit_s_base_entry() {
    let sandbox = Sandbox::new("bin_full_rebuild");
    let export = "exports/co Full Kits";
    for slot in ["p1", "p2", "g1"] {
        sandbox.write(&format!("{export}/Kits/{slot}/kit.dds"), &tracer_kit());
    }
    // A header the deep pass reads, over pixel data cut short: the kit's task fails.
    sandbox.write(&format!("{export}/Kits/p3/kit.dds"), &pixels_cut_dds());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert!(
        findings_of(&lines, "co Full Kits")
            .iter()
            .any(|line| line.starts_with("Error folder_pack_failed [DropFolder] at Kits/p3")),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
    assert_eq!(compiled_kits(&sandbox), ["u0714g1", "u0714p1", "u0714p2"]);
    let base = bundled_uni_color();
    let kit_2 = base_entry(&base, 714, 2);
    assert_eq!(kit_2, [0x02, 0x03, 0x13, 0x2d, 0x3f, 0x8c, 0x09, 0x08]);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let colors = tracer_kit_colors();
    assert_eq!(
        uni_record(&entries[UNI_COLOR], 714),
        record_of(
            714,
            4,
            &[
                kit_entry(0x00, colors),
                kit_entry(0x01, colors),
                kit_2,
                kit_entry(0x10, colors),
            ]
        ),
        "p1, p2 and g1 new, p3 the base's, the base's kits 3 to 6 gone"
    );
}

// TC-BIN-15
#[test]
fn a_midcup_export_merges_its_kits_into_its_team_s_record() {
    let sandbox = Sandbox::new("bin_midcup_merge");
    let export = "exports/co Midcup Kits";
    for slot in ["p1", "p8"] {
        sandbox.write(&format!("{export}/Kits/{slot}/kit.dds"), &tracer_kit());
        // The menu colors the merge writes (the same pair the texture derives to):
        // without a `colors.txt` a `Midcup` kit's entry stays installed.
        sandbox.write(
            &format!("{export}/Kits/{slot}/colors.txt"),
            b"#ffffff\n#ffffff\n",
        );
    }

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let base = bundled_uni_color();
    assert_eq!(uni_record(&base, 714)[4], 8, "the base's eight kits");
    let colors = tracer_kit_colors();
    let mut expected = vec![kit_entry(0x00, colors)];
    expected.extend((1..7).map(|index| base_entry(&base, 714, index)));
    expected.push(kit_entry(0x07, colors));
    expected.push(base_entry(&base, 714, 7));
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        uni_record(&entries[UNI_COLOR], 714),
        record_of(714, 9, &expected),
        "p1 replaced, kits 1 to 6 and 0x10 the base's, p8 added"
    );
}

// TC-BIN-19
#[test]
fn a_full_export_without_kits_compiles_an_empty_p1_and_g1() {
    let sandbox = Sandbox::new("bin_full_no_kits");
    sandbox.write(
        &format!("exports/co Full Players/{CLEAN_PLAYER}"),
        &clean_model(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Full Players"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info kit_config_generated [Keep] at Kits/p1 ()",
            "Info kit_placeholder [Keep] at Kits/p1 ()",
            "Info kit_config_generated [Keep] at Kits/g1 ()",
            "Info kit_placeholder [Keep] at Kits/g1 ()",
            "Warning kit_colors_missing [Keep] at Kits/p1 ()",
            "Warning kit_colors_missing [Keep] at Kits/g1 ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    assert_eq!(compiled_players(&sandbox), [71403]);
    assert_eq!(compiled_kits(&sandbox), ["u0714g1", "u0714p1"]);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    for ordinal in ["1st", "GK1st"] {
        assert!(
            entries.contains_key(&config_path(ordinal)),
            "{ordinal}: {:#?}",
            entries.keys()
        );
    }
    assert_eq!(
        uni_record(&entries[UNI_COLOR], 714),
        record_of(
            714,
            2,
            &[
                kit_entry(0x00, PLACEHOLDER_COLORS),
                kit_entry(0x10, PLACEHOLDER_COLORS),
            ]
        )
    );
}

// TC-BIN-20
#[test]
fn a_full_export_lacking_g1_gets_an_empty_one_and_a_midcup_export_none() {
    let sandbox = Sandbox::new("bin_full_and_midcup");
    for slot in ["p1", "p2"] {
        sandbox.write(
            &format!("exports/co Full Two/Kits/{slot}/kit.dds"),
            &tracer_kit(),
        );
    }
    sandbox.write("exports/a Midcup One/Kits/p1/kit.dds", &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let placeholders: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains("kit_placeholder"))
        .collect();
    assert_eq!(
        placeholders,
        ["co Full Two: Info kit_placeholder [Keep] at Kits/g1 ()"]
    );
    let base = bundled_uni_color();
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let colors = tracer_kit_colors();
    assert_eq!(
        uni_record(&entries[UNI_COLOR], 714),
        record_of(
            714,
            3,
            &[
                kit_entry(0x00, colors),
                kit_entry(0x01, colors),
                kit_entry(0x10, PLACEHOLDER_COLORS),
            ]
        ),
        "/co/: p1 and p2 new, g1 the placeholder"
    );
    // Team 702's base record holds nine kits, 0 to 7 and 0x10: the `Midcup` export's p1
    // holds no `colors.txt`, so its entry is the base's kept, not replaced.
    let expected: Vec<[u8; 8]> = (0..9).map(|index| base_entry(&base, 702, index)).collect();
    assert_eq!(
        expected,
        [
            [0x00, 0x03, 0xff, 0x00, 0x99, 0x00, 0x00, 0x59],
            [0x01, 0x03, 0xfc, 0xa8, 0xc8, 0xff, 0xff, 0xff],
            [0x02, 0x03, 0xfc, 0xc6, 0x86, 0xe1, 0xe5, 0xdb],
            [0x03, 0x03, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff],
            [0x04, 0x03, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00],
            [0x05, 0x03, 0x29, 0x7f, 0xc8, 0xff, 0xff, 0xff],
            [0x06, 0x03, 0xff, 0x00, 0x99, 0x00, 0x00, 0x59],
            [0x07, 0x03, 0xf9, 0x47, 0x4a, 0xff, 0xff, 0xff],
            [0x10, 0x03, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff],
        ],
        "the base's record for team 702"
    );
    assert_eq!(uni_record(&base, 702)[4], 9);
    assert_eq!(
        uni_record(&entries[UNI_COLOR], 702),
        record_of(702, 9, &expected),
        "/a/: p1 kept, the rest the base's"
    );
}

/// The CPK path of team 714's kit config for the kit `ordinal` (`1st`, `2nd`).
pub(crate) fn config_path(ordinal: &str) -> String {
    format!("common/character0/model/character/uniform/team/714/714_DEF_{ordinal}_realUni.bin")
}

/// The 120 bytes team 714's kit config for the kit `ordinal` was emitted as in `entries`.
fn emitted_config_bytes<'a>(entries: &'a BTreeMap<String, Vec<u8>>, ordinal: &str) -> &'a [u8] {
    &entries[&config_path(ordinal)]
}

/// The kit config team 714's kit `ordinal` was emitted as in `entries`, decoded for `version`.
pub(crate) fn emitted_config(
    entries: &BTreeMap<String, Vec<u8>>,
    ordinal: &str,
    version: PesVersion,
) -> KitConfig {
    KitConfig::decode(emitted_config_bytes(entries, ordinal), version).unwrap()
}

/// A kit config carrying shirt model 144 and the template's values otherwise, the FPC shorts
/// and collars among them.
const SHIRT_144: &[u8] = b"[shirt]\nmodel = 144\n";

// TC-KIT-15
#[test]
fn an_fpc_on_player_writes_the_fpc_values_into_every_kit_config() {
    let sandbox = Sandbox::new("kit_fpc_on");
    let export = "exports/co Full Fpc";
    sandbox.write(
        &format!("{export}/Players/05 - A/face_high.fmdl"),
        &clean_model(),
    );
    sandbox.write(&format!("{export}/Players/05 - A/fpc_on"), b"");
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(&format!("{export}/Kits/p1/config.toml"), SHIRT_144);
    sandbox.write(&format!("{export}/Kits/p2/kit.dds"), &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Full Fpc"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info kit_config_generated [Keep] at Kits/p2 ()",
            "Info kit_config_generated [Keep] at Kits/g1 ()",
            "Info kit_placeholder [Keep] at Kits/g1 ()",
            "Info kit_config_fpc_adjusted [Keep] at Kits/p1 ()",
            "Info kit_colors_derived [Keep] at Kits/p1 ()",
            "Info kit_colors_derived [Keep] at Kits/p2 ()",
            "Warning kit_colors_missing [Keep] at Kits/g1 ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    assert_eq!(compiled_players(&sandbox), [71405]);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    for ordinal in ["1st", "2nd", "GK1st"] {
        let config = emitted_config(&entries, ordinal, PesVersion::Pes21);
        assert!(kit_config::matches_fpc(&config), "{ordinal}");
    }
}

// TC-KIT-16
#[test]
fn without_fpc_on_supplied_kit_configs_are_emitted_as_they_are() {
    let sandbox = Sandbox::new("kit_fpc_unknown");
    let export = "exports/co Midcup Plain";
    // `fpc_off` is a statement about its player alone.
    sandbox.write(&format!("{export}/{CLEAN_PLAYER}"), &clean_model());
    sandbox.write(&format!("{export}/Players/03 - A/fpc_off"), b"");
    let fpc: &[u8] =
        b"[shirt]\nmodel = 176\ncollar = 105\nwinter_collar = 105\n\n[shorts]\nmodel = 16\n";
    for (slot, config) in [("p1", fpc), ("p2", SHIRT_144)] {
        sandbox.write(&format!("{export}/Kits/{slot}/kit.dds"), &tracer_kit());
        sandbox.write(&format!("{export}/Kits/{slot}/config.toml"), config);
    }

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert!(
        lines
            .iter()
            .all(|line| !line.contains("kit_config_fpc_adjusted")),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    for (slot, ordinal, config) in [(KitSlot::P1, "1st", fpc), (KitSlot::P2, "2nd", SHIRT_144)] {
        let names = kit_config::texture_names(
            714,
            slot,
            kit_config::TexturePresence {
                kit: true,
                ..kit_config::TexturePresence::default()
            },
        );
        let supplied = KitConfig::from_toml(std::str::from_utf8(config).unwrap()).unwrap();
        assert_eq!(
            entries[&config_path(ordinal)],
            supplied.encode_with_names(PesVersion::Pes21, &names),
            "{ordinal}"
        );
    }
    assert!(kit_config::matches_fpc(&emitted_config(
        &entries,
        "1st",
        PesVersion::Pes21
    )));
    assert_eq!(
        emitted_config(&entries, "2nd", PesVersion::Pes21)
            .shirt
            .model,
        144
    );
}

// TC-REF-07
#[test]
fn a_kit_or_a_collar_file_taking_the_referees_collar_77_is_left_out() {
    let sandbox = Sandbox::new("kit_collar_reserved");
    let export = "exports/co Midcup Collars";
    sandbox.write(&format!("{export}/{CLEAN_PLAYER}"), &clean_model());
    sandbox.write(&format!("{export}/Collars/collar_77.fmdl"), &clean_model());
    let configs: [(&str, &[u8]); 2] = [
        ("p1", b"[shirt]\ncollar = 77\n"),
        ("p2", b"[shirt]\nwinter_collar = 77\n"),
    ];
    for (slot, config) in configs {
        sandbox.write(&format!("{export}/Kits/{slot}/kit.dds"), &tracer_kit());
        sandbox.write(&format!("{export}/Kits/{slot}/config.toml"), config);
    }

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    let findings = findings_of(&lines, "co Midcup Collars");
    for reserved in [
        "Error collar_id_conflict [DropFile] at Collars/collar_77.fmdl (file=collar_77.fmdl, claimant=referees)",
        "Error kit_collar_reserved [DropFolder] at Kits/p1 (field=collar)",
        "Error kit_collar_reserved [DropFolder] at Kits/p2 (field=winter_collar)",
    ] {
        assert!(findings.contains(&reserved), "{lines:#?}");
    }
    assert_eq!(run.exit_code(), 1);
    // The player commits, the dropped collar file leaving nothing for the export to be refused
    // over; neither kit's config nor its texture goes in.
    assert_eq!(compiled_players(&sandbox), [71403]);
    assert_eq!(compiled_kits(&sandbox), Vec::<String>::new());
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    for ordinal in ["1st", "2nd"] {
        assert!(!entries.contains_key(&config_path(ordinal)), "{ordinal}");
    }
}

// TC-KIT-17
#[test]
fn a_kit_config_value_the_version_cannot_hold_is_reported_and_clamped() {
    let sandbox = Sandbox::new("kit_config_clamped");
    let export = "exports/co Midcup Clamp";
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{export}/Kits/p1/config.toml"),
        b"[name]\ny = 36\n",
    );
    let clamped = "Warning kit_config_version_clamped [Keep] at Kits/p1/config.toml (field=name.y, value=36, max=33)";

    let run = sandbox.run(&pes_settings(&sandbox, 18), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert!(
        findings_of(&lines, "co Midcup Clamp").contains(&clamped),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        emitted_config(&entries, "1st", PesVersion::Pes18).name.y,
        33
    );

    let check = sandbox.run(&pes_settings(&sandbox, 18), &["check"]);
    let lines = check.messages();
    assert!(
        findings_of(&lines, "co Midcup Clamp").contains(&clamped),
        "{lines:#?}"
    );
}

// TC-KIT-32
#[test]
fn a_sleeve_or_fit_option_the_shirt_model_ignores_is_reported_and_the_config_kept() {
    let sandbox = Sandbox::new("kit_config_option_ignored");
    let export = "exports/co Midcup Ignored";
    let configs: [(&str, &[u8]); 4] = [
        ("p1", b"[shirt]\nmodel = 176\nshort_sleeves = \"cut-out\"\n"),
        ("p2", b"[shirt]\nmodel = 176\nlong_sleeves = \"undershirt-only\"\n"),
        ("p3", b"[shirt]\nmodel = 176\ntight = true\n"),
        (
            "p4",
            b"[shirt]\nmodel = 144\nshort_sleeves = \"cut-out\"\nlong_sleeves = \"undershirt-only\"\ntight = true\n",
        ),
    ];
    for (slot, config) in configs {
        sandbox.write(&format!("{export}/Kits/{slot}/kit.dds"), &tracer_kit());
        sandbox.write(&format!("{export}/Kits/{slot}/config.toml"), config);
    }
    let ignored = [
        "Warning kit_config_option_ignored [Keep] at Kits/p1/config.toml (option=shirt.short_sleeves, model=176)",
        "Warning kit_config_option_ignored [Keep] at Kits/p2/config.toml (option=shirt.long_sleeves, model=176)",
        "Warning kit_config_option_ignored [Keep] at Kits/p3/config.toml (option=shirt.tight, model=176)",
    ];
    let reported = |lines: &[String]| -> Vec<String> {
        findings_of(lines, "co Midcup Ignored")
            .into_iter()
            .filter(|line| line.contains("kit_config_option_ignored"))
            .map(str::to_owned)
            .collect()
    };

    let check = sandbox.run(&pes_settings(&sandbox, 21), &["check"]);
    let lines = check.messages();
    assert_eq!(reported(&lines), ignored, "{lines:#?}");
    assert_eq!(check.exit_code(), 0);

    let run = sandbox.run(&pes_settings(&sandbox, 21), &["compile", "--no-deploy"]);
    let lines = run.messages();
    assert_eq!(reported(&lines), ignored, "{lines:#?}");
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let p1 = emitted_config(&entries, "1st", PesVersion::Pes21);
    assert_eq!(p1.shirt.short_sleeves, ShortSleeves::CutOut);
    assert_eq!(p1.shirt.model, 176);
}

// TC-KIT-32
#[test]
fn with_fpc_on_a_model_144_config_s_options_are_reported_with_the_fpc_model_176() {
    let sandbox = Sandbox::new("kit_config_option_ignored_fpc_on");
    let export = "exports/co Midcup Ignored";
    sandbox.write(
        &format!("{export}/Players/05 - A/face_high.fmdl"),
        &clean_model(),
    );
    sandbox.write(&format!("{export}/Players/05 - A/fpc_on"), b"");
    sandbox.write(&format!("{export}/Kits/p4/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{export}/Kits/p4/config.toml"),
        b"[shirt]\nmodel = 144\nshort_sleeves = \"cut-out\"\nlong_sleeves = \"undershirt-only\"\ntight = true\n",
    );

    let check = sandbox.run(&pes_settings(&sandbox, 21), &["check"]);

    let lines = check.messages();
    let reported: Vec<&str> = findings_of(&lines, "co Midcup Ignored")
        .into_iter()
        .filter(|line| line.contains("kit_config_option_ignored"))
        .collect();
    assert_eq!(
        reported,
        [
            "Warning kit_config_option_ignored [Keep] at Kits/p4/config.toml (option=shirt.short_sleeves, model=176)",
            "Warning kit_config_option_ignored [Keep] at Kits/p4/config.toml (option=shirt.long_sleeves, model=176)",
            "Warning kit_config_option_ignored [Keep] at Kits/p4/config.toml (option=shirt.tight, model=176)",
        ],
        "{lines:#?}"
    );
}

// TC-KIT-24
#[test]
fn a_name_y_compiles_to_the_same_bytes_on_pes_18_and_pes_21() {
    let sandbox = Sandbox::new("kit_config_name_y");
    let export = "exports/co Midcup Name";
    sandbox.write(&format!("{export}/Kits/p1/kit.dds"), &tracer_kit());
    sandbox.write(
        &format!("{export}/Kits/p1/config.toml"),
        b"[name]\ny = 30\n",
    );

    let mut emitted = Vec::new();
    for (number, version) in [(18, PesVersion::Pes18), (21, PesVersion::Pes21)] {
        let run = sandbox.run(&pes_settings(&sandbox, number), &["compile", "--no-deploy"]);
        let lines = run.messages();
        assert!(
            !lines
                .iter()
                .any(|line| line.contains("kit_config_version_clamped")),
            "{version:?}: {lines:#?}"
        );
        assert_eq!(run.exit_code(), 0, "{version:?}");
        let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
        assert_eq!(
            emitted_config(&entries, "1st", version).name.y,
            30,
            "{version:?}"
        );
        emitted.push(emitted_config_bytes(&entries, "1st").to_vec());
    }

    assert_eq!(emitted[0][0x1C..0x1E], emitted[1][0x1C..0x1E]);
    // 30 is the template's own Name Y, so these are the template's bytes.
    assert_eq!(emitted[1][0x1C..0x1E], [0xF2, 0x0E]);
}

// TC-SRC-03
#[test]
fn a_disabled_export_is_reported_once_by_check_and_compile_and_not_compiled() {
    let sandbox = Sandbox::new("src_disabled");
    sandbox.write("exports/co Midcup One/NO_USE", b"");
    sandbox.copy_tracer_face("exports/co Midcup One/Players/03 - A");
    sandbox.write("exports/dbg Midcup Two/no_use.txt", b"");
    sandbox.copy_tracer_face("exports/dbg Midcup Two/Players/03 - A");

    // `compile` reports its working bins first; `check` builds none.
    for (command, bins) in [("check", &[][..]), ("compile", &BUNDLED_BINS[..])] {
        let run = sandbox.run(&pes21_settings(&sandbox), &command_args(command));
        let findings = [
            "co Midcup One: Info export_disabled [DropExport] ()",
            "dbg Midcup Two: Info export_disabled [DropExport] ()",
        ];
        assert_eq!(run.messages(), [bins, &findings[..]].concat(), "{command}");
        assert_eq!(run.exit_code(), 0, "{command}");
    }
    assert!(!sandbox.root.join("output/4cc_99_test.cpk").exists());
    assert!(!sandbox.root.join("output/.staging").exists());
}

// TC-SRC-04
#[test]
fn a_balls_export_is_skipped_by_check_and_compile() {
    let sandbox = Sandbox::new("src_balls");
    sandbox.copy_tracer_face("exports/balls Spring/Players/03 - A");
    sandbox.copy_tracer("egg Midcup Tracer");

    for command in ["check", "compile"] {
        let run = sandbox.run(&pes21_settings(&sandbox), &command_args(command));
        assert_eq!(
            findings_of(&run.messages(), "balls Spring"),
            ["Info export_balls_skipped [DropExport] ()"],
            "{command}"
        );
        assert_eq!(run.exit_code(), 0, "{command}");
    }
    assert_eq!(compiled_players(&sandbox), [79205]);
}

// TC-SRC-06
#[test]
fn a_corrupt_zip_is_skipped_and_the_export_beside_it_compiled() {
    let sandbox = Sandbox::new("src_corrupt_zip");
    sandbox.write("exports/co Midcup Broken.zip", b"not a zip at all");
    sandbox.copy_tracer("egg Midcup Tracer");

    for command in ["check", "compile"] {
        let run = sandbox.run(&pes21_settings(&sandbox), &command_args(command));
        assert_eq!(
            findings_of(&run.messages(), "co Midcup Broken.zip"),
            [format!(
                "Error export_extract_failed [DropExport] (path={}, error=zip: invalid Zip archive: Could not find EOCD)",
                sandbox.display("exports/co Midcup Broken.zip")
            )],
            "{command}"
        );
        assert_eq!(run.exit_code(), 1, "{command}");
    }
    assert_eq!(compiled_players(&sandbox), [79205]);
}

// TC-SRC-08
#[test]
fn export_paths_restrict_check_and_compile_to_the_named_exports() {
    let sandbox = Sandbox::new("src_named_exports");
    sandbox.copy_tracer_face("exports/co Midcup A/Players/03 - A");
    sandbox.copy_tracer("egg Midcup Tracer");
    sandbox.copy_tracer_face("elsewhere/dbg Midcup D/Players/03 - A");
    let named = [
        "--export",
        &sandbox.arg("elsewhere/dbg Midcup D"),
        "--export",
        &sandbox.arg("exports/co Midcup A"),
    ];

    for command in ["check", "compile"] {
        let args: Vec<&str> = command_args(command).into_iter().chain(named).collect();
        let run = sandbox.run(&pes21_settings(&sandbox), &args);
        let validated = [
            "co Midcup A: Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=boots.fmdl, count=1662)",
            "co Midcup A: Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=fcl_hair.fmdl, count=1662)",
            "co Midcup A: Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=glove_l.fmdl, count=2)",
            "co Midcup A: Info export_identified [Keep] (team=/co/, id=714)",
            "dbg Midcup D: Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=boots.fmdl, count=1662)",
            "dbg Midcup D: Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=fcl_hair.fmdl, count=1662)",
            "dbg Midcup D: Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=glove_l.fmdl, count=2)",
            "dbg Midcup D: Info export_identified [Keep] (team=/dbg/, id=790)",
        ];
        // The working bins and planning's notes, which only `compile` makes.
        let (bins, planned): (&[&str], &[&str]) = if command == "compile" {
            (
                &BUNDLED_BINS,
                &[
                    "co Midcup A: Info team_colors_missing [Keep] ()",
                    "dbg Midcup D: Info team_colors_missing [Keep] ()",
                ],
            )
        } else {
            (&[], &[])
        };
        let mut expected: Vec<String> = [bins, &validated[..], planned]
            .concat()
            .iter()
            .map(|line| (*line).to_owned())
            .collect();
        if command == "compile" {
            expected.push(deploy_skipped(&sandbox));
            expected.push(patch_written(&sandbox, "714, 790"));
        }
        assert_eq!(run.messages(), expected, "{command}");
        assert_eq!(run.exit_code(), 0, "{command}");
    }
    assert_eq!(compiled_players(&sandbox), [71403, 79003]);
    assert!(compiled_kits(&sandbox).is_empty());
}

// TC-ID-01
#[test]
fn a_zip_export_is_compiled_as_the_team_its_name_starts_with() {
    let sandbox = Sandbox::new("id_zip");
    sandbox.write(
        "exports/co Full Spring 2026.zip",
        &source_fixture("egg Midcup Tracer.zip"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let mut expected = no_deploy_lines(
        &sandbox,
        [
            "co Full Spring 2026.zip: Info fmdl_weights_not_normalized [Keep] at Players/05 - The Chad Stormworks Player (file=fcl_hair.fmdl, count=1662)",
            "co Full Spring 2026.zip: Info export_identified [Keep] (team=/co/, id=714)",
            "co Full Spring 2026.zip: Info team_colors_missing [Keep] ()",
            // A full export with no player kit compiles an empty `p1/`.
            "co Full Spring 2026.zip: Info kit_config_generated [Keep] at Kits/p1 ()",
            "co Full Spring 2026.zip: Info kit_placeholder [Keep] at Kits/p1 ()",
            "co Full Spring 2026.zip: Warning kit_colors_missing [Keep] at Kits/p1 ()",
            "co Full Spring 2026.zip: Info kit_colors_derived [Keep] at Kits/g1 ()",
        ],
    );
    expected.push(patch_written(&sandbox, "714"));
    assert_eq!(run.messages(), expected);
    assert_eq!(compiled_players(&sandbox), [71405]);
    assert_eq!(compiled_kits(&sandbox), ["u0714g1", "u0714p1"]);
    assert_eq!(run.exit_code(), 0);
}

// TC-ROOT-05
#[test]
fn a_root_notes_txt_that_cannot_be_read_is_dropped_and_the_rest_compiled() {
    let sandbox = Sandbox::new("root_notes_unreadable");
    sandbox.copy_fixture("egg Midcup Tracer bad notes.zip", "exports");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let mut expected = no_deploy_lines(
        &sandbox,
        [
            "egg Midcup Tracer bad notes.zip: Info fmdl_weights_not_normalized [Keep] at Players/05 - The Chad Stormworks Player (file=fcl_hair.fmdl, count=1662)",
            "egg Midcup Tracer bad notes.zip: Error source_read_failed [DropFile] at notes.txt (reason=Invalid checksum)",
            "egg Midcup Tracer bad notes.zip: Info export_identified [Keep] (team=/egg/, id=792)",
            "egg Midcup Tracer bad notes.zip: Info team_colors_missing [Keep] ()",
        ],
    );
    expected.push(patch_written(&sandbox, "792"));
    assert_eq!(run.messages(), expected);
    assert_eq!(compiled_players(&sandbox), [79205]);
    assert_eq!(run.exit_code(), 1);
}

// TC-PLN-08
#[test]
fn a_7z_over_the_memory_cap_compiles() {
    let sandbox = Sandbox::new("seven_z_over_cap");
    sandbox.copy_fixture("egg Midcup Tracer.7z", "exports");
    // A cap of a few hundred bytes against the archive's 151 KB decompressed: its permit is
    // oversized, so a task asking the budget for its own while it is held would wait forever.
    let settings = format!(
        "{}memory_cap_percent = 0.000001\n",
        pes21_settings(&sandbox)
    );

    let run = sandbox.run(&settings, &["compile", "--no-deploy"]);

    let mut expected = no_deploy_lines(
        &sandbox,
        [
            "egg Midcup Tracer.7z: Info fmdl_weights_not_normalized [Keep] at Players/05 - The Chad Stormworks Player (file=fcl_hair.fmdl, count=1662)",
            "egg Midcup Tracer.7z: Info export_identified [Keep] (team=/egg/, id=792)",
            "egg Midcup Tracer.7z: Info team_colors_missing [Keep] ()",
        ],
    );
    expected.push(patch_written(&sandbox, "792"));
    assert_eq!(run.messages(), expected);
    assert_eq!(compiled_players(&sandbox), [79205]);
    assert_eq!(compiled_kits(&sandbox), ["u0792g1"]);
    assert_eq!(run.exit_code(), 0);
}

// TC-PLN-05
#[test]
fn a_7z_kept_from_its_check_and_one_decompressed_again_compile_the_same_cpk() {
    // The default cap keeps the archive's 150 KiB decompressed for its tasks (any cap of at
    // least 1 GiB plus those 150 KiB keeps it). A cap of 0.01% of the available memory is
    // under 1 GiB on any machine with less than 10 TiB available, so no archive is kept and
    // the tasks decompress it again; it is above the archive's size wherever more than 1.5 GB
    // is available, so the archive's permit is no oversized one there.
    let mut outcomes = Vec::new();
    for (name, cap) in [
        ("seven_z_kept", ""),
        ("seven_z_not_kept", "memory_cap_percent = 0.01\n"),
    ] {
        let sandbox = Sandbox::new(name);
        sandbox.copy_fixture("egg Midcup Tracer.7z", "exports");
        let settings = format!("{}{cap}", pes21_settings(&sandbox));

        let run = sandbox.run(&settings, &["compile", "--no-deploy"]);

        assert_eq!(run.exit_code(), 0, "{name}");
        let cpk = fs::read(sandbox.root.join("output/4cc_99_test.cpk")).unwrap();
        // The notes name this sandbox's CPK and patch, so they are checked here and the rest
        // compared.
        let mut messages = run.messages();
        assert_eq!(
            messages.pop(),
            Some(patch_written(&sandbox, "792")),
            "{name}"
        );
        assert_eq!(messages.pop(), Some(deploy_skipped(&sandbox)), "{name}");
        outcomes.push((messages, cpk));
    }
    assert_eq!(outcomes[0].0, outcomes[1].0);
    assert!(outcomes[0].1 == outcomes[1].1, "the two CPKs differ");
}

// TC-CLI-11
#[test]
fn an_export_is_processed_after_its_last_task_or_after_planning_when_it_has_none() {
    let sandbox = Sandbox::new("processed_order");
    sandbox.write("exports/co Midcup Kits/Kits/p1/kit.dds", &tracer_kit());
    // Fails on the pool, so its finding is the writer's to report.
    sandbox.write("exports/co Midcup Kits/Kits/g1/kit.dds", &pixels_cut_dds());
    sandbox.write("exports/dbg Midcup Off/NO_USE", b"");
    sandbox.write(
        &format!("exports/dbg Midcup Off/{CLEAN_PLAYER}"),
        &clean_model(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let events: Vec<String> = run
        .events
        .iter()
        .map(|envelope| match &envelope.event {
            PipelineEvent::ExportStarted { export_id, .. } => format!("started {}", export_id.0),
            PipelineEvent::Message(message) => format!("message {}", message.code.code),
            PipelineEvent::ExportProcessed { export_id } => format!("processed {}", export_id.0),
            PipelineEvent::Progress { processed, total } => {
                format!("progress {processed}/{total}")
            }
            PipelineEvent::Complete { .. } => "complete".to_owned(),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        events,
        [
            "message bin_source",
            "message bin_source",
            "message bin_source",
            "started 0",
            "message export_identified",
            "started 1",
            "message export_disabled",
            "message team_colors_missing",
            "processed 1",
            "progress 1/2",
            "message folder_pack_failed",
            "processed 0",
            "progress 2/2",
            "message deploy_skipped_by_flag",
            "complete",
        ]
    );
    assert_eq!(run.exit_code(), 1);
    assert_eq!(
        run.complete().skipped_exports,
        1,
        "the disabled export counts as skipped"
    );
}

// TC-CLI-13
#[test]
fn timestamps_marks_each_finding_with_the_seconds_since_the_run_started() {
    // TC-CLI-11's root: a kit whose task fails on the pool, a disabled export.
    let sandbox = Sandbox::new("timestamps");
    sandbox.write("exports/co Midcup Kits/Kits/p1/kit.dds", &tracer_kit());
    sandbox.write("exports/co Midcup Kits/Kits/g1/kit.dds", &pixels_cut_dds());
    sandbox.write("exports/dbg Midcup Off/NO_USE", b"");
    sandbox.write(
        &format!("exports/dbg Midcup Off/{CLEAN_PLAYER}"),
        &clean_model(),
    );
    let settings = pes21_settings(&sandbox);

    let started = Instant::now();
    let timed = sandbox.run(&settings, &["compile", "--no-deploy", "--timestamps"]);
    let wall = started.elapsed();
    let untimed = sandbox.run(&settings, &["compile", "--no-deploy"]);

    assert_eq!(timed.messages(), untimed.messages());
    assert!(!timed.messages().is_empty(), "the run reports findings");
    for envelope in &timed.events {
        match &envelope.event {
            PipelineEvent::Message(_) => assert!(
                envelope.made_at.is_some_and(|made| made <= wall),
                "every Message carries a made_at under the run's {wall:?}: {envelope:?}"
            ),
            _ => assert_eq!(envelope.made_at, None, "{envelope:?}"),
        }
    }
    for envelope in &untimed.events {
        assert_eq!(envelope.made_at, None, "{envelope:?}");
    }

    let checked = sandbox.run(&settings, &["check", "--timestamps"]);
    let mut findings = 0;
    for envelope in &checked.events {
        if let PipelineEvent::Message(_) = &envelope.event {
            findings += 1;
            assert!(envelope.made_at.is_some(), "{envelope:?}");
        }
    }
    assert!(findings > 0, "the check reports findings");
}

// TC-PLN-05
#[test]
fn the_worker_count_changes_neither_the_findings_nor_the_cpk() {
    let mut outcomes = Vec::new();
    for threads in [1, 8] {
        let sandbox = Sandbox::new(&format!("worker_count_{threads}"));
        sandbox.copy_tracer("egg Midcup Tracer");
        sandbox.write(
            "exports/dbg Midcup Seven.7z",
            &source_fixture("egg Midcup Tracer.7z"),
        );
        sandbox.write("exports/co Midcup Kits/Kits/p1/kit.dds", &tracer_kit());
        sandbox.write("exports/co Midcup Kits/Kits/g1/kit.dds", &tracer_kit());
        let settings = format!("{}thread_count = {threads}\n", pes21_settings(&sandbox));

        let run = sandbox.run(&settings, &["compile", "--no-deploy"]);

        assert_eq!(run.exit_code(), 0, "{threads} threads");
        let cpk = fs::read(sandbox.root.join("output/4cc_99_test.cpk")).unwrap();
        // The notes name this sandbox's CPK and patch, so they are checked here and the rest
        // compared. The kits-only export adds no `[[teams]]` entry.
        let mut messages = run.messages();
        assert_eq!(
            messages.pop(),
            Some(patch_written(&sandbox, "790, 792")),
            "{threads}"
        );
        assert_eq!(messages.pop(), Some(deploy_skipped(&sandbox)), "{threads}");
        outcomes.push((messages, cpk));
    }
    assert_eq!(outcomes[0].0, outcomes[1].0);
    assert_eq!(
        outcomes[0].0,
        bundled_bins_then([
            "co Midcup Kits: Info export_identified [Keep] (team=/co/, id=714)",
            "dbg Midcup Seven.7z: Info fmdl_weights_not_normalized [Keep] at Players/05 - The Chad Stormworks Player (file=fcl_hair.fmdl, count=1662)",
            "dbg Midcup Seven.7z: Info export_identified [Keep] (team=/dbg/, id=790)",
            "egg Midcup Tracer: Info fmdl_weights_not_normalized [Keep] at Players/05 - The Chad Stormworks Player (file=boots.fmdl, count=1662)",
            "egg Midcup Tracer: Info fmdl_weights_not_normalized [Keep] at Players/05 - The Chad Stormworks Player (file=fcl_hair.fmdl, count=1662)",
            "egg Midcup Tracer: Info fmdl_weights_not_normalized [Keep] at Players/05 - The Chad Stormworks Player (file=glove_l.fmdl, count=2)",
            "egg Midcup Tracer: Info export_identified [Keep] (team=/egg/, id=792)",
            "co Midcup Kits: Info team_colors_missing [Keep] ()",
            "dbg Midcup Seven.7z: Info team_colors_missing [Keep] ()",
        ])
    );
    assert!(outcomes[0].1 == outcomes[1].1, "the CPKs differ");
}

// TC-PLN-05
#[test]
fn two_teams_with_players_and_kits_compile_to_the_same_cpk_on_one_worker_and_on_eight() {
    let mut cpks = Vec::new();
    for threads in [1, 8] {
        let sandbox = Sandbox::new(&format!("pln_worker_count_{threads}"));
        for export in ["a Midcup Home", "co Midcup Away"] {
            sandbox.copy_tracer_face(&format!("exports/{export}/Players/03 - A"));
            sandbox.copy_tracer_face(&format!("exports/{export}/Players/07 - B"));
            sandbox.write(&format!("exports/{export}/Kits/p1/kit.dds"), &tracer_kit());
        }
        let settings = format!("{}thread_count = {threads}\n", pes21_settings(&sandbox));

        let run = sandbox.run(&settings, &["compile", "--no-deploy"]);

        assert_eq!(
            run.exit_code(),
            0,
            "{threads} threads: {:#?}",
            run.messages()
        );
        assert_eq!(compiled_players(&sandbox), [70203, 70207, 71403, 71407]);
        assert_eq!(compiled_kits(&sandbox), ["u0702p1", "u0714p1"]);
        cpks.push(fs::read(sandbox.root.join("output/4cc_99_test.cpk")).unwrap());
    }
    let first_difference = cpks[0]
        .iter()
        .zip(&cpks[1])
        .position(|(one, eight)| one != eight);
    assert_eq!(
        (first_difference, cpks[0].len()),
        (None, cpks[1].len()),
        "the first differing offset, and the two sizes"
    );
}

/// Writes `exports/a Midcup Home`, a valid `/a/` export: one player and one kit.
fn write_a_home(sandbox: &Sandbox) {
    sandbox.write(
        &format!("exports/a Midcup Home/{CLEAN_PLAYER}"),
        &clean_model(),
    );
    sandbox.write("exports/a Midcup Home/Kits/p1/kit.dds", &tracer_kit());
}

/// Writes `a Midcup Home` beside two exports of `/co/`: the folder `co Full A`, holding a goalkeeper
/// kit, and `co Full B.zip`, the tracer's player and goalkeeper kit. Both compiled, the two kits
/// would write one CPK path twice.
fn write_two_exports_of_one_team(sandbox: &Sandbox) {
    write_a_home(sandbox);
    sandbox.write("exports/co Full A/Kits/g1/kit.dds", &tracer_kit());
    sandbox.write(
        "exports/co Full B.zip",
        &source_fixture("egg Midcup Tracer.zip"),
    );
}

/// The finding each of the two `/co/` exports of `write_two_exports_of_one_team` gets.
const CO_DUPLICATE: &str =
    "Error duplicate_aesthetics_export [DropExport] (id=714, exports=co Full A, co Full B.zip)";

// TC-PLN-03
#[test]
fn two_exports_of_one_team_are_both_skipped_naming_each_other_and_the_other_team_compiles() {
    let sandbox = Sandbox::new("pln_duplicate_team");
    write_two_exports_of_one_team(&sandbox);
    let validated = [
        "a Midcup Home: Info export_identified [Keep] (team=/a/, id=702)".to_owned(),
        "co Full A: Info export_identified [Keep] (team=/co/, id=714)".to_owned(),
        format!("co Full A: {CO_DUPLICATE}"),
        "co Full B.zip: Info fmdl_weights_not_normalized [Keep] at Players/05 - The Chad Stormworks Player (file=fcl_hair.fmdl, count=1662)".to_owned(),
        "co Full B.zip: Info export_identified [Keep] (team=/co/, id=714)".to_owned(),
        format!("co Full B.zip: {CO_DUPLICATE}"),
    ];

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    assert_eq!(check.messages(), validated);
    assert_eq!(check.exit_code(), 1);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    // Planning's notes are `a Midcup Home`'s alone: nothing of `/co/` is planned.
    let planned = ["a Midcup Home: Info team_colors_missing [Keep] ()".to_owned()];
    let mut expected = no_deploy_lines(&sandbox, [&validated[..], &planned[..]].concat());
    expected.push(patch_written(&sandbox, "702"));
    assert_eq!(lines, expected);
    assert_eq!(compiled_players(&sandbox), [70203]);
    assert_eq!(compiled_kits(&sandbox), ["u0702p1"]);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let of_714: Vec<&String> = entries.keys().filter(|path| path.contains("714")).collect();
    assert_eq!(of_714, Vec::<&String>::new());
    assert_eq!(
        run.complete().skipped_exports,
        2,
        "each export of /co/ was refused whole"
    );
}

// TC-PLN-07
#[test]
fn a_team_beside_two_skipped_exports_compiles_to_the_cpk_it_compiles_to_alone() {
    let beside = Sandbox::new("pln_skipped_beside");
    write_two_exports_of_one_team(&beside);
    let alone = Sandbox::new("pln_skipped_alone");
    write_a_home(&alone);

    let beside_run = beside.run(&pes21_settings(&beside), &["compile", "--no-deploy"]);
    let alone_run = alone.run(&pes21_settings(&alone), &["compile", "--no-deploy"]);

    assert_eq!(beside_run.exit_code(), 1, "the duplicate's errors");
    assert_eq!(alone_run.exit_code(), 0, "{:#?}", alone_run.messages());
    let beside_cpk = fs::read(beside.root.join("output/4cc_99_test.cpk")).unwrap();
    let alone_cpk = fs::read(alone.root.join("output/4cc_99_test.cpk")).unwrap();
    let first_difference = beside_cpk
        .iter()
        .zip(&alone_cpk)
        .position(|(beside, alone)| beside != alone);
    assert_eq!(
        (first_difference, beside_cpk.len()),
        (None, alone_cpk.len()),
        "the first differing offset, and the two sizes"
    );
}

/// Slot 05's boots, the player's own, as `/co/` compiles them.
const BOOTS_05: &str = "Asset/model/character/boots/k0625/#Win/boots.fpk";

/// What `write_overrides` puts at `BOOTS_05`.
const BOOTS_OVERRIDE: &[u8] = b"the operator's boots";

/// What `write_overrides` puts at `TEAM_COLOR`.
const TEAM_COLOR_OVERRIDE: &[u8] = b"the operator's TeamColor.bin";

/// Writes the sandbox's `data/overrides/` folder: a `TeamColor.bin` and slot 05's boots for
/// `/co/`, and returns the folder as `overrides_active` names it.
fn write_overrides(sandbox: &Sandbox) -> String {
    sandbox.write(&format!("data/overrides/{TEAM_COLOR}"), TEAM_COLOR_OVERRIDE);
    sandbox.write(&format!("data/overrides/{BOOTS_05}"), BOOTS_OVERRIDE);
    sandbox.display("data/overrides")
}

// TC-PLN-04
#[test]
fn the_overrides_replace_the_export_s_boots_and_the_team_color_bin_and_are_reported() {
    let sandbox = Sandbox::new("pln_overrides");
    let folder = write_overrides(&sandbox);
    sandbox.write(
        "exports/co Midcup Boots/Players/05 - A/boots.fmdl",
        &tracer_player_file("boots.fmdl"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let mut expected = no_deploy_lines(&sandbox, [
        "co Midcup Boots: Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)".to_owned(),
        "co Midcup Boots: Info export_identified [Keep] (team=/co/, id=714)".to_owned(),
        "co Midcup Boots: Info team_colors_missing [Keep] ()".to_owned(),
        format!("Info overrides_active [Keep] (folder={folder}, files=2)"),
        format!("Warning duplicate_path [Keep] (path={BOOTS_05})"),
        format!("Warning duplicate_path [Keep] (path={TEAM_COLOR})"),
    ]);
    expected.push(patch_written(&sandbox, "714"));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 0, "a Warning does not fail the run");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(entries[BOOTS_05], BOOTS_OVERRIDE);
    assert_eq!(entries[TEAM_COLOR], TEAM_COLOR_OVERRIDE);
    // The boots task was not dropped: its other entry is in the CPK.
    assert!(
        entries.contains_key("Asset/model/character/boots/k0625/#Win/boots.fpkd"),
        "{:#?}",
        entries.keys()
    );
    let complete = run.complete();
    assert_eq!(
        complete.files_written,
        entries.len(),
        "the overrides among the CPK's table of contents"
    );
    assert_eq!(complete.duplicates, 2, "the two entries replaced");
}

// TC-PLN-04
#[test]
fn the_overrides_are_written_with_the_bins_when_no_export_compiles() {
    let sandbox = Sandbox::new("pln_overrides_no_export");
    let folder = write_overrides(&sandbox);
    fs::create_dir_all(sandbox.root.join("exports")).unwrap();

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        run.messages(),
        no_deploy_lines(
            &sandbox,
            [
                format!(
                    "Warning no_exports_found [Keep] (folder={})",
                    sandbox.display("exports")
                ),
                format!("Info overrides_active [Keep] (folder={folder}, files=2)"),
                format!("Warning duplicate_path [Keep] (path={TEAM_COLOR})"),
            ]
        )
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            BOOTS_05,
            "common/character0/model/character/uniform/team/UniColor.bin",
            TEAM_COLOR,
        ]
    );
    assert_eq!(entries[BOOTS_05], BOOTS_OVERRIDE);
    assert_eq!(entries[TEAM_COLOR], TEAM_COLOR_OVERRIDE);
    assert_eq!(
        run.complete().duplicates,
        1,
        "the bin's entry alone was replaced"
    );
}
