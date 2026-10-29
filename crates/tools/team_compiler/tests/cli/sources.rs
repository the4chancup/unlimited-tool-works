//! Archive sources: `.zip` and `.7z` exports read beside folder ones, and the ones refused.

use std::fs;
use std::path::Path;

use kit_config::{KitConfig, KitSlot, TexturePresence, texture_names};
use pes_version::PesVersion;

use crate::common::Sandbox;
use crate::compile::{compiled_players, cpk_entries, kit_texture, pes21_settings};
use crate::compile_exports::{UNI_COLOR, bundled_uni_color, with_uni_record};
use crate::{CLEAN_PLAYER, clean_model, findings_of, snapshot, source_fixture};

/// The folder `exports/co Full Spring/`, the same export as the `co Midcup Spring` fixtures
/// written under the `Full` coverage name: git keeps no empty folder, so it is built here.
fn spring_folder(sandbox: &Sandbox) {
    sandbox.write("exports/co Full Spring/players.txt", b"01 Keeper\n");
    sandbox.write(
        "exports/co Full Spring/notes.txt",
        b"Spring kit placeholder.\n",
    );
    fs::create_dir_all(sandbox.root.join("exports/co Full Spring/Kits/p2")).unwrap();
}

/// What `check` reports about the `co Full Spring` export, in any of its three forms. The roster
/// line names a player folder the export does not hold, so its finding shows the roster was read.
const SPRING_FINDINGS: [&str; 3] = [
    "Error players_txt_target_missing [DropSlot] at players.txt line 1 slot Some(1) (folder=Keeper)",
    "Info notes_found [Keep] at notes.txt ()",
    "Info export_identified [Keep] (team=/co/, id=714)",
];

/// `SPRING_FINDINGS` of a `co Full Spring` source checked beside other exports of `/co/`, then
/// the finding that skips each of them, naming `exports`.
fn spring_findings_beside(exports: &str) -> Vec<String> {
    SPRING_FINDINGS
        .iter()
        .map(|line| (*line).to_owned())
        .chain([format!(
            "Error duplicate_aesthetics_export [DropExport] (id=714, exports={exports})"
        )])
        .collect()
}

// TC-SRC-01
#[test]
fn one_export_as_a_folder_a_zip_and_a_7z_reports_and_compiles_the_same() {
    let sandbox = Sandbox::new("same_export_three_ways");
    spring_folder(&sandbox);
    sandbox.write(
        "exports/co Full Spring.zip",
        &source_fixture("co Midcup Spring.zip"),
    );
    sandbox.write(
        "exports/co Full Spring.7z",
        &source_fixture("co Midcup Spring.7z"),
    );

    let run = sandbox.run("", &["check"]);

    let lines = run.messages();
    // Three exports of one team: each is skipped, naming the three.
    let expected = spring_findings_beside("co Full Spring, co Full Spring.7z, co Full Spring.zip");
    for source in ["co Full Spring", "co Full Spring.zip", "co Full Spring.7z"] {
        assert_eq!(findings_of(&lines, source), expected, "{lines:#?}");
    }
    assert_eq!(lines.len(), 3 * expected.len(), "{lines:#?}");
    assert_eq!(run.exit_code(), 1);

    // Each source compiled alone: the empty kit folder p2 is the placeholder kit, the
    // `Full` export's `g1/` an empty one beside it.
    let settings = pes21_settings(&sandbox);
    let mut archives = Vec::new();
    for source in ["co Full Spring", "co Full Spring.zip", "co Full Spring.7z"] {
        let run = sandbox.run(
            &settings,
            &[
                "compile",
                "--no-deploy",
                "--export",
                &sandbox.arg(&format!("exports/{source}")),
            ],
        );
        let lines = run.messages();
        let expected: Vec<&str> = SPRING_FINDINGS
            .iter()
            .copied()
            .chain([
                "Info team_colors_missing [Keep] ()",
                "Info kit_config_generated [Keep] at Kits/p2 ()",
                "Info kit_placeholder [Keep] at Kits/p2 ()",
                "Info kit_config_generated [Keep] at Kits/g1 ()",
                "Info kit_placeholder [Keep] at Kits/g1 ()",
                "Warning kit_colors_missing [Keep] at Kits/p2 ()",
                "Warning kit_colors_missing [Keep] at Kits/g1 ()",
            ])
            .collect();
        assert_eq!(findings_of(&lines, source), expected, "{lines:#?}");
        assert_eq!(run.exit_code(), 1, "{source}");
        archives.push(cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk")));
    }
    assert!(
        archives[0] == archives[1],
        "the folder's and the zip's CPKs differ"
    );
    assert!(
        archives[0] == archives[2],
        "the folder's and the 7z's CPKs differ"
    );

    let placeholder = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resources/kits/placeholder_kit.dds"),
    )
    .unwrap();
    let names = texture_names(
        714,
        KitSlot::P2,
        TexturePresence {
            kit: true,
            ..TexturePresence::default()
        },
    );
    let config = KitConfig::template().encode_with_names(PesVersion::Pes21, &names);
    let paths: Vec<&str> = archives[0].keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            kit_texture("u0714g1"),
            kit_texture("u0714p2"),
            "common/character0/model/character/uniform/team/714/714_DEF_2nd_realUni.bin".to_owned(),
            "common/character0/model/character/uniform/team/714/714_DEF_GK1st_realUni.bin"
                .to_owned(),
            "common/character0/model/character/uniform/team/UniColor.bin".to_owned(),
            "common/character0/model/character/uniform/team/UniformParameter.bin".to_owned(),
            "common/etc/TeamColor.bin".to_owned(),
            "unlimited-tool-works/shared_ids/714.toml".to_owned(),
        ]
    );
    assert!(
        archives[0][&kit_texture("u0714p2")]
            == ftex::dds_to_ftex(&placeholder, ftex::ColorSpace::Normal).unwrap(),
        "the p2 texture is not the placeholder converted"
    );
    assert!(
        archives[0][&kit_texture("u0714g1")]
            == ftex::dds_to_ftex(&placeholder, ftex::ColorSpace::Normal).unwrap(),
        "the g1 texture is the placeholder converted"
    );
    assert_eq!(
        archives[0]["common/character0/model/character/uniform/team/714/714_DEF_2nd_realUni.bin"],
        config
    );
}

// TC-BIN-02
#[test]
fn the_empty_kit_folder_of_tc_src_01_s_export_gets_magenta_black_and_icon_3() {
    let sandbox = Sandbox::new("same_export_kit_colors");
    spring_folder(&sandbox);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 1, "the roster line naming no folder");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // A `Full` export rebuilds the record from its kits: kit 1 (p2) and kit 0x10 (the
    // empty `g1/` it plans), each magenta, black and icon 3.
    let base = bundled_uni_color();
    let mut record = vec![0xca, 0x02, 0x00, 0x00, 2];
    record.extend([0x01, 0x03, 0xff, 0x00, 0xff, 0x00, 0x00, 0x00]);
    record.extend([0x10, 0x03, 0xff, 0x00, 0xff, 0x00, 0x00, 0x00]);
    for _ in 2..10 {
        record.extend([0xff, 0, 0, 0, 0, 0, 0, 0]);
    }
    assert!(
        entries[UNI_COLOR] == with_uni_record(&base, 714, &record),
        "UniColor.bin is the base with team 714's record rebuilt magenta, black and icon 3"
    );
}

// TC-SRC-02
#[test]
fn a_folder_and_an_archive_sharing_a_stem_are_two_exports() {
    let sandbox = Sandbox::new("folder_and_archive");
    spring_folder(&sandbox);
    sandbox.write(
        "exports/co Full Spring.zip",
        &source_fixture("co Midcup Spring.zip"),
    );

    let run = sandbox.run("", &["check"]);

    let lines = run.messages();
    let expected = spring_findings_beside("co Full Spring, co Full Spring.zip");
    assert_eq!(findings_of(&lines, "co Full Spring"), expected);
    assert_eq!(findings_of(&lines, "co Full Spring.zip"), expected);
    assert_eq!(lines.len(), 2 * expected.len(), "{lines:#?}");
}

// TC-SRC-06
#[test]
fn a_corrupt_archive_is_skipped_and_the_export_beside_it_is_still_checked() {
    let sandbox = Sandbox::new("corrupt_archive");
    // The scan compares the extension in any case.
    sandbox.write("exports/co Midcup Broken.ZIP", b"not a zip at all");
    sandbox.write(
        &format!("exports/co Midcup Spring/{CLEAN_PLAYER}"),
        &clean_model(),
    );

    let run = sandbox.run("", &["check"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Broken.ZIP"),
        [format!(
            "Error export_extract_failed [DropExport] (path={}, error=zip: invalid Zip archive: Could not find EOCD)",
            sandbox.display("exports/co Midcup Broken.ZIP")
        )]
    );
    assert_eq!(
        findings_of(&lines, "co Midcup Spring"),
        ["Info export_identified [Keep] (team=/co/, id=714)"]
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-SRC-07
#[test]
fn an_archive_whose_names_collide_or_escape_is_skipped_naming_the_path() {
    let sandbox = Sandbox::new("refused_listings");
    sandbox.copy_fixture("co Midcup Case.zip", "exports");
    sandbox.copy_fixture("co Midcup Escape.zip", "exports");

    let run = sandbox.run("", &["check"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Case.zip"),
        [
            "Error export_extract_failed [DropExport] (path=Players.txt, error=path collides with existing entry players.txt)"
        ]
    );
    assert_eq!(
        findings_of(&lines, "co Midcup Escape.zip"),
        [format!(
            "Error export_extract_failed [DropExport] (path={}, error=invalid entry name \"../x\")",
            sandbox.display("exports/co Midcup Escape.zip")
        )]
    );
    assert_eq!(lines.len(), 2, "{lines:#?}");
    assert_eq!(run.exit_code(), 1);
}

// TC-SRC-08
#[test]
fn export_paths_may_name_an_archive_outside_the_root() {
    let sandbox = Sandbox::new("named_archive");
    sandbox.write(
        &format!("exports/co Midcup A/{CLEAN_PLAYER}"),
        &clean_model(),
    );
    sandbox.write(
        &format!("exports/co Midcup C/{CLEAN_PLAYER}"),
        &clean_model(),
    );
    sandbox.copy_fixture("co Midcup Spring.zip", "elsewhere");

    let run = sandbox.run(
        "",
        &[
            "check",
            "--export",
            &sandbox.arg("elsewhere/co Midcup Spring.zip"),
            "--export",
            &sandbox.arg("exports/co Midcup A"),
        ],
    );

    let lines = run.messages();
    let duplicate = "Error duplicate_aesthetics_export [DropExport] (id=714, exports=co Midcup A, co Midcup Spring.zip)";
    assert_eq!(
        findings_of(&lines, "co Midcup A"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            duplicate
        ]
    );
    let spring = spring_findings_beside("co Midcup A, co Midcup Spring.zip");
    assert_eq!(findings_of(&lines, "co Midcup Spring.zip"), spring);
    // co Midcup C, in the root but not named, is not reported.
    assert_eq!(lines.len(), 2 + spring.len(), "{lines:#?}");
}

// TC-SRC-09
#[test]
fn check_and_compile_leave_every_source_as_it_was() {
    let sandbox = Sandbox::new("sources_untouched");
    sandbox.copy_tracer("egg Midcup Tracer");
    sandbox.write(
        "exports/co Midcup Zip.zip",
        &source_fixture("egg Midcup Tracer.zip"),
    );
    sandbox.write(
        "exports/dbg Midcup Seven.7z",
        &source_fixture("egg Midcup Tracer.7z"),
    );
    sandbox.copy_tracer_face("exports/esg Midcup Nested/wrapper/Players/03 - A");
    let exports = sandbox.root.join("exports");
    let before = snapshot(&exports);

    let settings = pes21_settings(&sandbox);
    assert_eq!(sandbox.run(&settings, &["check"]).exit_code(), 0);
    assert_eq!(snapshot(&exports), before, "check");
    assert_eq!(
        sandbox
            .run(&settings, &["compile", "--no-deploy"])
            .exit_code(),
        0
    );
    assert_eq!(snapshot(&exports), before, "compile");
    assert_eq!(compiled_players(&sandbox), [71405, 79005, 79205, 79303]);
}

// TC-SRC-11
#[test]
fn an_unfinished_upgrade_s_staging_folder_is_skipped_with_a_warning() {
    let sandbox = Sandbox::new("upgrading_skipped");
    // A staging folder an interrupted upgrade left, beside a finished export of another
    // team. Left in the run it would compile, or refuse the team's export as a duplicate.
    sandbox.copy_tracer("co Full Spring.upgrading");
    sandbox.write(
        &format!("exports/dbg Midcup A/{CLEAN_PLAYER}"),
        &clean_model(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        lines.contains(&format!(
            "Warning export_upgrade_unfinished [Keep] (path={})",
            sandbox.display("exports/co Full Spring.upgrading")
        )),
        "{lines:#?}"
    );
    assert!(
        findings_of(&lines, "co Full Spring.upgrading").is_empty(),
        "{lines:#?}"
    );
    assert_eq!(compiled_players(&sandbox), [79003]);
}

// TC-SRC-11
#[test]
fn an_unfinished_upgrade_s_staging_folder_is_skipped_in_any_letter_case() {
    let sandbox = Sandbox::new("upgrading_case");
    sandbox.copy_tracer("co Full Spring.UPGRADING");
    sandbox.write(
        &format!("exports/dbg Midcup A/{CLEAN_PLAYER}"),
        &clean_model(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        lines.contains(&format!(
            "Warning export_upgrade_unfinished [Keep] (path={})",
            sandbox.display("exports/co Full Spring.UPGRADING")
        )),
        "{lines:#?}"
    );
    assert_eq!(compiled_players(&sandbox), [79003]);
}

// TC-SRC-11
#[test]
fn a_named_export_path_ending_in_upgrading_is_compiled_the_member_named_it() {
    let sandbox = Sandbox::new("upgrading_named");
    sandbox.copy_tracer("co Midcup Spring.upgrading");

    let run = sandbox.run(
        &pes21_settings(&sandbox),
        &[
            "compile",
            "--no-deploy",
            "--export",
            &sandbox.arg("exports/co Midcup Spring.upgrading"),
        ],
    );

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        findings_of(&lines, "co Midcup Spring.upgrading")
            .contains(&"Info export_identified [Keep] (team=/co/, id=714)"),
        "{lines:#?}"
    );
    assert_eq!(compiled_players(&sandbox), [71405]);
}
