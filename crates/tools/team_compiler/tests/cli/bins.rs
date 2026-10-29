//! `compile`'s working bins taken from the installed CPKs: the walk of the PES folder's
//! `download/DpFileList.bin` (`spec/team_compiler/pipeline.md` "Bins accumulation").

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use kit_config::{KitConfig, KitSlot, TexturePresence, texture_names};
use pes_savefile::interchange::aesthetics_patch::AestheticsPatch;
use pes_savefile::player_tables::{AppearanceTable, PlayerTableFile};
use pes_savefile::settings_toml::{IngameFaceSettings, PlayerSettings, SettingKey};
use pes_version::PesVersion;
use uniparam::UniformParameter;

use crate::collars::pre_fox_model;
use crate::common::Sandbox;
use crate::compile::{cpk_entries, pes_settings, pes21_settings, tracer_kit, tracer_player_file};
use crate::compile_exports::{
    TEAM_COLOR, UNI_COLOR, UNIFORM_PARAMETER, bundled_team_color, bundled_uni_color,
    bundled_uniform_parameter, config_path, record_of, uni_record, with_uni_record,
};
use crate::models::body_skl;
use crate::prefox_faces::write_slot_05_face;
use crate::{BUNDLED_BINS, clean_model, deploy_skipped, patch_written};

/// Team `/co/`, whose `UniColor.bin` record the tests set.
const CO: usize = 714;

/// An installed `UniColor.bin` entry for team 714's kit 0 (p1).
const A: [u8; 8] = [0x00, 0x03, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66];

/// Another installed entry for team 714's kit 0, in a CPK loaded above `A`'s.
const B: [u8; 8] = [0x00, 0x03, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];

/// The entry `p2_export`'s kit compiles to: kit 1, icon 3, the two colors of its `colors.txt`.
const P2: [u8; 8] = [0x01, 0x03, 0x0a, 0x1b, 0x2c, 0xd4, 0xe5, 0xf6];

/// The export every test compiles: a Midcup `/co/` export with only `p2/`, its kit texture and
/// a `colors.txt` giving two colors, and no root `colors.txt`.
fn p2_export(sandbox: &Sandbox) {
    sandbox.write("exports/co Midcup Kits/Kits/p2/kit.dds", &tracer_kit());
    sandbox.write(
        "exports/co Midcup Kits/Kits/p2/colors.txt",
        b"#0a1b2c\n#d4e5f6\n",
    );
}

/// What `p2_export` reports, after the walk's findings: no `kit_config_generated`, p2's
/// `Midcup` kit keeping the installed config where it holds no `config.toml` of its own.
const P2_FINDINGS: [&str; 2] = [
    "co Midcup Kits: Info export_identified [Keep] (team=/co/, id=714)",
    "co Midcup Kits: Info team_colors_missing [Keep] ()",
];

/// Writes `PES/download/DpFileList.bin` listing `4cc_08_bins.cpk`, `4cc_61_midcup.cpk` and
/// `4cc_99_test.cpk`, in that load order (the `walk.bin` fixture).
fn install_list(sandbox: &Sandbox) {
    let list = fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dpfl/walk.bin"))
        .unwrap();
    sandbox.write("PES/download/DpFileList.bin", &list);
}

/// Writes `PES/download/DpFileList.bin` listing `names`, in that load order.
pub(crate) fn install_names(sandbox: &Sandbox, names: &[&str]) {
    sandbox.write("PES/download/DpFileList.bin", &dpfl_bytes(names));
}

/// A `DpFileList.bin` listing `names`, in that load order: a 16-byte header holding the count,
/// then one 48-byte record per CPK, its name NUL-padded.
pub(crate) fn dpfl_bytes(names: &[&str]) -> Vec<u8> {
    let count = u32::try_from(names.len()).unwrap();
    let mut list = vec![0; 4];
    list.extend(count.to_le_bytes());
    list.extend([0; 8]);
    for name in names {
        let mut record = [0; 48];
        record[..name.len()].copy_from_slice(name.as_bytes());
        list.extend(record);
    }
    list
}

/// Writes `PES/download/<name>` as a CPK holding `entries` (CPK path, bytes).
pub(crate) fn install_cpk(sandbox: &Sandbox, name: &str, entries: &[(&str, &[u8])]) {
    let path = sandbox.root.join("PES/download").join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut cpk = cpk::CpkWriter::new(fs::File::create(path).unwrap(), "test").unwrap();
    for (entry, bytes) in entries {
        cpk.add(entry, bytes, None).unwrap();
    }
    cpk.finish().unwrap();
}

/// The bundled `UniColor.bin` with team 714's kit-0 entry (its first) replaced by `kit_0`.
fn uni_color_with_kit_0(kit_0: [u8; 8]) -> Vec<u8> {
    let base = bundled_uni_color();
    let mut record = uni_record(&base, CO).to_vec();
    assert_eq!(record[5], 0x00, "the base's first entry is kit 0");
    record[5..13].copy_from_slice(&kit_0);
    with_uni_record(&base, CO, &record)
}

/// `installed` with team 714's kit-1 entry (its second) set to `P2`: what a compile of
/// `p2_export` builds on it.
fn with_p2(installed: &[u8]) -> Vec<u8> {
    let mut record = uni_record(installed, CO).to_vec();
    assert_eq!(record[13], 0x01, "the record's second entry is kit 1");
    record[13..21].copy_from_slice(&P2);
    with_uni_record(installed, CO, &record)
}

/// The `bin_source` line naming `bin` and `cpk`.
fn source(bin: &str, cpk: &str) -> String {
    format!("Info bin_source [Keep] (bin={bin}, cpk={cpk})")
}

/// TC-BIN-05's install: `4cc_08_bins.cpk` holding the bundled `TeamColor.bin` and a
/// `UniColor.bin` whose team 714 kit 0 is `A`, `4cc_61_midcup.cpk` one where it is `B`.
fn install_two_cpks(sandbox: &Sandbox) {
    install_list(sandbox);
    install_cpk(
        sandbox,
        "4cc_08_bins.cpk",
        &[
            (TEAM_COLOR, &bundled_team_color()),
            (UNI_COLOR, &uni_color_with_kit_0(A)),
        ],
    );
    install_cpk(
        sandbox,
        "4cc_61_midcup.cpk",
        &[(UNI_COLOR, &uni_color_with_kit_0(B))],
    );
}

// TC-BIN-05
#[test]
fn each_bin_comes_from_the_nearest_cpk_listed_below_the_run_s_own() {
    let sandbox = Sandbox::new("bins_walk");
    install_two_cpks(&sandbox);
    p2_export(&sandbox);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let mut expected = vec![
        source("TeamColor.bin", "4cc_08_bins.cpk"),
        source("UniColor.bin", "4cc_61_midcup.cpk"),
        source("UniformParameter.bin", "bundled"),
    ];
    expected.extend(P2_FINDINGS.map(str::to_owned));
    expected.push(deploy_skipped(&sandbox));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        entries[UNI_COLOR] == with_p2(&uni_color_with_kit_0(B)),
        "UniColor.bin is 4cc_61_midcup's with p2's entry set"
    );
    assert!(entries[TEAM_COLOR] == bundled_team_color());

    // The midcup CPK is the run's own: it and everything above it are skipped.
    let settings = format!(
        "{}[team-compiler]\ncpk_name = \"4cc_61_midcup\"\n",
        pes21_settings(&sandbox)
    );
    let run = sandbox.run(&settings, &["compile", "--no-deploy"]);

    let mut expected = vec![
        source("TeamColor.bin", "4cc_08_bins.cpk"),
        source("UniColor.bin", "4cc_08_bins.cpk"),
        source("UniformParameter.bin", "bundled"),
    ];
    expected.extend(P2_FINDINGS.map(str::to_owned));
    expected.push(format!(
        "Info deploy_skipped_by_flag [Keep] (path={})",
        sandbox.display("output/4cc_61_midcup.cpk")
    ));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_61_midcup.cpk"));
    assert!(
        entries[UNI_COLOR] == with_p2(&uni_color_with_kit_0(A)),
        "UniColor.bin is 4cc_08_bins' with p2's entry set"
    );
}

// TC-BIN-05
#[test]
fn the_installed_refs_cpk_is_passed_over_when_the_run_holds_a_refs_export() {
    let sandbox = Sandbox::new("bins_walk_refs");
    install_names(
        &sandbox,
        &["4cc_08_bins.cpk", "4cc_18_referees.cpk", "4cc_99_test.cpk"],
    );
    install_cpk(
        &sandbox,
        "4cc_08_bins.cpk",
        &[(TEAM_COLOR, &bundled_team_color())],
    );
    // The bundled bin with team 702's (/a/'s) first two colors changed, which no other copy has.
    let mut refs_team_color = bundled_team_color();
    let record = (702 - 100) * 16;
    refs_team_color[record + 4..record + 10].copy_from_slice(&[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]);
    install_cpk(
        &sandbox,
        "4cc_18_referees.cpk",
        &[(TEAM_COLOR, &refs_team_color)],
    );
    p2_export(&sandbox);
    let bins_sources = |team_color_cpk: &str| {
        vec![
            source("TeamColor.bin", team_color_cpk),
            source("UniColor.bin", "bundled"),
            source("UniformParameter.bin", "bundled"),
        ]
    };
    let team_cpk = sandbox.root.join("output/4cc_99_test.cpk");

    // Without a refs export the installed refs CPK is one more CPK listed below the run's.
    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    assert_eq!(run.messages()[..3], bins_sources("4cc_18_referees.cpk"));
    assert!(cpk_entries(&team_cpk)[TEAM_COLOR] == refs_team_color);

    sandbox.write("exports/refs Cup/players.txt", b"01 Keeper\n");
    sandbox.write(
        "exports/refs Cup/Players/Keeper/face_high.fmdl",
        &clean_model(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    // The run replaces it, so the walk goes on to the bins CPK below it.
    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    assert_eq!(run.messages()[..3], bins_sources("4cc_08_bins.cpk"));
    assert!(cpk_entries(&team_cpk)[TEAM_COLOR] == bundled_team_color());
}

// TC-CLI-10
#[test]
fn the_walk_reads_the_pes_folder_with_its_version_placeholder_expanded() {
    let sandbox = Sandbox::new("bins_placeholder");
    install_list(&sandbox);
    install_cpk(
        &sandbox,
        "4cc_08_bins.cpk",
        &[(UNI_COLOR, &uni_color_with_kit_0(A))],
    );
    fs::rename(sandbox.root.join("PES"), sandbox.root.join("PES2021")).unwrap();
    p2_export(&sandbox);
    let settings = format!(
        "[common]\npes_version = 21\npes_folder_path = '{}'\n",
        sandbox.root.join("PES20**").display()
    );

    let run = sandbox.run(&settings, &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    assert_eq!(
        run.messages()[..3],
        [
            source("TeamColor.bin", "bundled"),
            source("UniColor.bin", "4cc_08_bins.cpk"),
            source("UniformParameter.bin", "bundled"),
        ]
    );
}

// TC-BIN-08
#[test]
fn a_wesys_wrapped_installed_bin_is_read_unwrapped_and_written_plain() {
    let sandbox = Sandbox::new("bins_wrapped");
    install_list(&sandbox);
    let wrapped = wezlib::compress(&uni_color_with_kit_0(A));
    install_cpk(&sandbox, "4cc_08_bins.cpk", &[(UNI_COLOR, &wrapped)]);
    p2_export(&sandbox);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    assert_eq!(
        run.messages()[..3],
        [
            source("TeamColor.bin", "bundled"),
            source("UniColor.bin", "4cc_08_bins.cpk"),
            source("UniformParameter.bin", "bundled"),
        ]
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(!wezlib::is_wrapped(&entries[UNI_COLOR]), "written plain");
    assert_eq!(uni_record(&entries[UNI_COLOR], CO)[5..13], A);
    assert!(entries[UNI_COLOR] == with_p2(&uni_color_with_kit_0(A)));
}

// TC-BIN-09
#[test]
fn with_no_dpfilelist_the_bins_are_bundled_and_the_list_s_absence_reported() {
    let alone = Sandbox::new("bins_no_pes");
    p2_export(&alone);
    let run = alone.run(&pes21_settings(&alone), &["compile", "--no-deploy"]);
    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let bundled = cpk_entries(&alone.root.join("output/4cc_99_test.cpk"));

    let sandbox = Sandbox::new("bins_no_list");
    fs::create_dir_all(sandbox.root.join("PES/download")).unwrap();
    // The run's exe, so the deploying run's only finding about the install is the list's.
    sandbox.write("PES/PES2021.exe", b"the game");
    p2_export(&sandbox);
    let list = sandbox.display("PES/download/DpFileList.bin");
    let bundled_sources = [
        source("TeamColor.bin", "bundled"),
        source("UniColor.bin", "bundled"),
        source("UniformParameter.bin", "bundled"),
    ];

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile"]);

    let mut expected = vec![format!("Error dpfilelist_missing [Keep] (path={list})")];
    expected.extend(bundled_sources.clone());
    expected.extend(P2_FINDINGS.map(str::to_owned));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 1);
    let cpk = sandbox.root.join("output/4cc_99_test.cpk");
    let entries = cpk_entries(&cpk);
    assert!(entries[UNI_COLOR] == bundled[UNI_COLOR]);
    assert!(entries[TEAM_COLOR] == bundled[TEAM_COLOR]);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let mut expected = vec![format!("Warning dpfilelist_missing [Keep] (path={list})")];
    expected.extend(bundled_sources);
    expected.extend(P2_FINDINGS.map(str::to_owned));
    expected.push(format!(
        "Info deploy_skipped_by_flag [Keep] (path={})",
        sandbox.display("output/4cc_99_test.cpk")
    ));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 0);
}

// TC-OUT-18
#[test]
fn test_and_sideload_modes_without_a_dpfilelist_warn_and_build_on_the_bundled_bins() {
    let sandbox = Sandbox::new("bins_no_list_modes");
    fs::create_dir_all(sandbox.root.join("PES/download")).unwrap();
    sandbox.write("PES/PES2021.exe", b"the game");
    p2_export(&sandbox);
    let list = sandbox.display("PES/download/DpFileList.bin");
    // Neither mode deploys a CPK, so the list's absence is a Warning in both, and neither
    // ends with a deployment note.
    let mut expected = vec![format!("Warning dpfilelist_missing [Keep] (path={list})")];
    expected.extend([
        source("TeamColor.bin", "bundled"),
        source("UniColor.bin", "bundled"),
        source("UniformParameter.bin", "bundled"),
    ]);
    expected.extend(P2_FINDINGS.map(str::to_owned));

    for mode in ["test", "sideload"] {
        let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--mode", mode]);

        assert_eq!(run.messages(), expected, "--mode {mode}");
        assert_eq!(run.exit_code(), 0, "--mode {mode}");
    }
}

// TC-BIN-13
#[test]
fn an_installed_team_color_record_whose_header_holds_colors_gets_its_header_back() {
    let sandbox = Sandbox::new("bins_header_repaired");
    install_list(&sandbox);
    let mut installed = bundled_team_color();
    let record = (799 - 100) * 16;
    installed[record..record + 4].copy_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]);
    install_cpk(&sandbox, "4cc_08_bins.cpk", &[(TEAM_COLOR, &installed)]);
    p2_export(&sandbox);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let lines = run.messages();
    assert_eq!(lines[0], source("TeamColor.bin", "4cc_08_bins.cpk"));
    let repaired: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains("bin_header_repaired"))
        .collect();
    assert_eq!(
        repaired,
        ["Warning bin_header_repaired [Keep] (bin=TeamColor.bin, teams=799)"]
    );
    let emitted = &cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"))[TEAM_COLOR];
    // Team ID 799 and count 4, little-endian, then the installed record's other bytes.
    assert_eq!(emitted[record..record + 4], [0x1f, 0x03, 0x04, 0x00]);
    assert_eq!(
        emitted[record + 4..record + 16],
        installed[record + 4..record + 16]
    );
    assert_eq!(emitted[..record], installed[..record]);
    assert_eq!(emitted[record + 16..], installed[record + 16..]);
}

// TC-BIN-21
#[test]
fn an_installed_cpk_that_cannot_be_read_stops_the_run_before_any_export_is_read() {
    let sandbox = Sandbox::new("bins_unreadable");
    install_two_cpks(&sandbox);
    sandbox.write("PES/download/4cc_61_midcup.cpk", b"not a cpk");
    p2_export(&sandbox);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    let [line] = lines.as_slice() else {
        panic!("one finding, on no export: {lines:#?}");
    };
    let prefix = format!(
        "Fatal installed_bin_unreadable [AbortRun] (path={}, error=",
        sandbox.display("PES/download/4cc_61_midcup.cpk")
    );
    assert!(line.starts_with(&prefix), "{line}");
    assert_eq!(run.exit_code(), 3);
    assert!(!sandbox.root.join("output/4cc_99_test.cpk").exists());
}

// TC-BIN-21
#[test]
fn an_installed_bin_that_does_not_parse_stops_the_run_before_any_export_is_read() {
    let sandbox = Sandbox::new("bins_unparsable");
    install_two_cpks(&sandbox);
    // One byte short of a whole 85-byte record.
    install_cpk(&sandbox, "4cc_61_midcup.cpk", &[(UNI_COLOR, &[0; 84])]);
    p2_export(&sandbox);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        run.messages(),
        [format!(
            "Fatal installed_bin_unreadable [AbortRun] (path={}, error=cannot read {UNI_COLOR}: \
             UniColor.bin is 84 bytes, not a whole number of 85-byte records)",
            sandbox.display("PES/download/4cc_61_midcup.cpk")
        )],
        "one finding, on no export"
    );
    assert_eq!(run.exit_code(), 3);
    assert!(!sandbox.root.join("output/4cc_99_test.cpk").exists());
}

/// Team `team_id`'s kit config of `slot` holding shirt model 144, which lacks the FPC values,
/// with `fpc` the FPC values set, encoded for `version` with the slot's main texture name.
pub(crate) fn shirt_144_config(
    version: PesVersion,
    team_id: u16,
    slot: KitSlot,
    fpc: bool,
) -> Vec<u8> {
    let mut config = KitConfig::template();
    config.shirt.model = 144;
    assert!(
        !kit_config::matches_fpc(&config),
        "shirt model 144 is no FPC shirt"
    );
    if fpc {
        kit_config::apply_fpc(&mut config);
    }
    let names = texture_names(
        team_id,
        slot,
        TexturePresence {
            kit: true,
            ..TexturePresence::default()
        },
    );
    config.encode_with_names(version, &names).to_vec()
}

/// The bundled `UniColor.bin` with team 714's record holding kits 0, 1 and 2: p1, p2 and p3.
pub(crate) fn uni_color_p1_p2_p3() -> Vec<u8> {
    let record = record_of(
        714,
        3,
        &[
            A,
            [0x01, 0x03, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
            [0x02, 0x03, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c],
        ],
    );
    with_uni_record(&bundled_uni_color(), CO, &record)
}

/// A `UniformParameter.bin` holding `configs` (team ID, slot), each `shirt_144_config`.
pub(crate) fn installed_configs(configs: &[(u16, KitSlot)]) -> Vec<u8> {
    let mut bin = UniformParameter::new();
    for (team_id, slot) in configs {
        bin.insert(
            slot.config_name(*team_id),
            shirt_144_config(PesVersion::Pes21, *team_id, *slot, false),
        )
        .unwrap();
    }
    bin.write()
}

/// The emitted `UniformParameter.bin`'s entries whose name starts with `prefix`, by name.
fn configs_of(entries: &BTreeMap<String, Vec<u8>>, prefix: &str) -> Vec<(String, Vec<u8>)> {
    UniformParameter::read(&entries[UNIFORM_PARAMETER])
        .unwrap()
        .entries()
        .filter(|(name, _)| name.starts_with(prefix))
        .map(|(name, bytes)| (name.to_owned(), bytes.to_vec()))
        .collect()
}

// TC-BIN-06
#[test]
fn a_midcup_fpc_on_export_patches_the_installed_configs_of_the_kits_it_does_not_hold() {
    let sandbox = Sandbox::new("bins_fpc_absent_slots");
    install_list(&sandbox);
    let uni_color = uni_color_p1_p2_p3();
    // `UniformParameter::new()` plus p1's config: team 714 has no p3 config.
    let installed = installed_configs(&[(714, KitSlot::P1)]);
    install_cpk(
        &sandbox,
        "4cc_08_bins.cpk",
        &[
            (TEAM_COLOR, &bundled_team_color()),
            (UNI_COLOR, &uni_color),
            (UNIFORM_PARAMETER, &installed),
        ],
    );
    p2_export(&sandbox);
    sandbox.write(
        "exports/co Midcup Kits/Players/05 - A/face_high.fmdl",
        &clean_model(),
    );
    sandbox.write("exports/co Midcup Kits/Players/05 - A/fpc_on", b"");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let mut expected = vec![
        source("TeamColor.bin", "4cc_08_bins.cpk"),
        source("UniColor.bin", "4cc_08_bins.cpk"),
        source("UniformParameter.bin", "4cc_08_bins.cpk"),
    ];
    expected.extend(P2_FINDINGS.map(str::to_owned));
    // p1 patched in place, p2 generated (the install holds no p2 config), p3 unpatched.
    expected.extend([
        "co Midcup Kits: Info kit_config_fpc_adjusted [Keep] (slot=p1)".to_owned(),
        "co Midcup Kits: Info kit_config_generated [Keep] at Kits/p2 ()".to_owned(),
        "co Midcup Kits: Warning kit_config_fpc_unpatched [Keep] (slot=p3)".to_owned(),
        // The fpc_on player's resolved boots, gloves and appearance rows: no installed table.
        "Warning player_table_missing [Keep] (table=BootsList.bin, rows=1)".to_owned(),
        "Warning player_table_missing [Keep] (table=GloveList.bin, rows=1)".to_owned(),
        "Warning player_table_missing [Keep] (table=PlayerAppearance.bin, rows=1)".to_owned(),
    ]);
    expected.push(deploy_skipped(&sandbox));
    expected.push(patch_written(&sandbox, "714"));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let configs = configs_of(&entries, "714_");
    let names: Vec<&str> = configs.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        ["714_DEF_1st_realUni.bin", "714_DEF_2nd_realUni.bin"],
        "no 714_DEF_3rd"
    );
    assert!(
        configs[0].1 == shirt_144_config(PesVersion::Pes21, 714, KitSlot::P1, true),
        "p1 is the installed config with the FPC values"
    );
    // p2 emits no config of its own and the install holds none: the template stands in.
    let names = texture_names(
        714,
        KitSlot::P2,
        TexturePresence {
            kit: true,
            ..TexturePresence::default()
        },
    );
    let generated = KitConfig::template().encode_with_names(PesVersion::Pes21, &names);
    assert!(configs[1].1 == generated, "p2 is the generated config");
    assert_eq!(entries[&config_path("2nd")], generated);
}

// TC-BIN-23
#[test]
fn a_midcup_export_s_collar_reports_the_kit_it_does_not_hold_that_has_no_config_to_wear_it() {
    let sandbox = Sandbox::new("bins_collar_absent_slots");
    install_list(&sandbox);
    // Team 714 has p1's config and no p3 config.
    install_cpk(
        &sandbox,
        "4cc_08_bins.cpk",
        &[
            (TEAM_COLOR, &bundled_team_color()),
            (UNI_COLOR, &uni_color_p1_p2_p3()),
            (UNIFORM_PARAMETER, &installed_configs(&[(714, KitSlot::P1)])),
        ],
    );
    p2_export(&sandbox);
    sandbox.write(
        "exports/co Midcup Kits/Collars/collar_12.fmdl",
        &clean_model(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let mut expected = vec![
        source("TeamColor.bin", "4cc_08_bins.cpk"),
        source("UniColor.bin", "4cc_08_bins.cpk"),
        source("UniformParameter.bin", "4cc_08_bins.cpk"),
    ];
    expected.extend(P2_FINDINGS.map(str::to_owned));
    // p1 wears the collar with no finding; p2's template wears it (the install holds
    // no p2 config); p3 has none to wear it.
    expected.extend([
        "co Midcup Kits: Info kit_config_generated [Keep] at Kits/p2 ()".to_owned(),
        "co Midcup Kits: Warning kit_config_collar_unpatched [Keep] (slot=p3)".to_owned(),
    ]);
    expected.push(deploy_skipped(&sandbox));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let configs = configs_of(&entries, "714_");
    let names: Vec<&str> = configs.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        ["714_DEF_1st_realUni.bin", "714_DEF_2nd_realUni.bin"],
        "no 714_DEF_3rd"
    );
    for (name, config) in &configs {
        let config = KitConfig::decode(config, PesVersion::Pes21).unwrap();
        assert_eq!(
            (config.shirt.collar, config.shirt.winter_collar),
            (12, 12),
            "{name}"
        );
    }
}

// A `Midcup` kit without `config.toml` over a slot with no installed config gets the
// template, reported `kit_config_generated`, as a `Full` export's would.
// TC-KIT-34
#[test]
fn a_midcup_kit_without_config_or_an_installed_one_gets_the_template() {
    let sandbox = Sandbox::new("bins_midcup_generated");
    install_list(&sandbox);
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
    // p4: a `kit.dds` alone; the install holds no p4 config.
    sandbox.write("exports/co Midcup Kits/Kits/p4/kit.dds", &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let mut expected = vec![
        source("TeamColor.bin", "4cc_08_bins.cpk"),
        source("UniColor.bin", "4cc_08_bins.cpk"),
        source("UniformParameter.bin", "4cc_08_bins.cpk"),
        "co Midcup Kits: Info export_identified [Keep] (team=/co/, id=714)".to_owned(),
        "co Midcup Kits: Info team_colors_missing [Keep] ()".to_owned(),
        "co Midcup Kits: Info kit_config_generated [Keep] at Kits/p4 ()".to_owned(),
    ];
    expected.push(deploy_skipped(&sandbox));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let names = texture_names(
        714,
        KitSlot::P4,
        TexturePresence {
            kit: true,
            ..TexturePresence::default()
        },
    );
    let generated = KitConfig::template().encode_with_names(PesVersion::Pes21, &names);
    assert_eq!(
        entries[&config_path("4th")],
        generated,
        "the loose config is the template"
    );
    let bin = uniparam::UniformParameter::read(&entries[UNIFORM_PARAMETER]).unwrap();
    assert_eq!(
        bin.get("714_DEF_4th_realUni.bin"),
        Some(generated.as_slice()),
        "p4's entry is the template"
    );
}

/// TC-BIN-18's install: `4cc_08_bins.cpk` holding the bundled `TeamColor.bin`,
/// `uni_color_p1_p2_p3` and team 714's loose p1 config, shirt model 144 encoded for PES 17;
/// team 714 has no p3 config.
fn install_pre_fox_p1(sandbox: &Sandbox) {
    install_pre_fox(
        sandbox,
        &shirt_144_config(PesVersion::Pes17, 714, KitSlot::P1, false),
    );
}

/// `install_pre_fox_p1` with `p1` as team 714's loose p1 config.
fn install_pre_fox(sandbox: &Sandbox, p1: &[u8]) {
    install_list(sandbox);
    install_cpk(
        sandbox,
        "4cc_08_bins.cpk",
        &[
            (TEAM_COLOR, &bundled_team_color()),
            (UNI_COLOR, &uni_color_p1_p2_p3()),
            (&config_path("1st"), p1),
        ],
    );
}

/// Writes player slot 05 into the export `export`, a pre-Fox face with the marker `fpc_on`.
fn write_fpc_player(sandbox: &Sandbox, export: &str) {
    write_slot_05_face(sandbox, export);
    sandbox.write(&format!("exports/{export}/Players/05 - A/fpc_on"), b"");
}

/// The lines of a PES 17 `compile --no-deploy` of `p2_export` over `install_pre_fox_p1`:
/// both color bins from the installed CPK, the export's findings, then `fpc_findings`.
fn pre_fox_lines(sandbox: &Sandbox, fpc_findings: &[&str]) -> Vec<String> {
    let mut lines = vec![
        source("TeamColor.bin", "4cc_08_bins.cpk"),
        source("UniColor.bin", "4cc_08_bins.cpk"),
    ];
    lines.extend(P2_FINDINGS.map(str::to_owned));
    lines.extend(fpc_findings.iter().map(|line| (*line).to_owned()));
    lines.push(deploy_skipped(sandbox));
    lines
}

// TC-BIN-18
#[test]
fn a_midcup_fpc_on_pes_17_export_re_emits_the_loose_configs_of_the_kits_it_does_not_hold() {
    let sandbox = Sandbox::new("bins_fpc_loose_configs");
    install_pre_fox_p1(&sandbox);
    p2_export(&sandbox);
    write_fpc_player(&sandbox, "co Midcup Kits");

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);

    // p1 patched in place, p2 generated (the install holds no p2 config), p3 unpatched.
    let expected = pre_fox_lines(
        &sandbox,
        &[
            "co Midcup Kits: Info kit_config_fpc_adjusted [Keep] (slot=p1)",
            "co Midcup Kits: Info kit_config_generated [Keep] at Kits/p2 ()",
            "co Midcup Kits: Warning kit_config_fpc_unpatched [Keep] (slot=p3)",
        ],
    );
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        entries.get(&config_path("1st"))
            == Some(&shirt_144_config(PesVersion::Pes17, 714, KitSlot::P1, true)),
        "p1 is the installed config with the FPC values"
    );
    // The install holds no p2 config: the template stands in.
    let p2 = KitConfig::decode(&entries[&config_path("2nd")], PesVersion::Pes17).unwrap();
    assert!(kit_config::matches_fpc(&p2));
    assert!(!entries.contains_key(&config_path("3rd")));
    let bins: Vec<&String> = entries
        .keys()
        .filter(|path| path.ends_with("UniformParameter.bin"))
        .collect();
    assert_eq!(bins, Vec::<&String>::new());
}

/// `config` with its Name Y bits (0x1C bits 3-7, 0x1D bit 0) set to `y` by hand: `encode`
/// clamps Name Y to 39 at most, so it cannot write the value another tool may have.
fn with_name_y(mut config: Vec<u8>, y: u8) -> Vec<u8> {
    config[0x1C] = (config[0x1C] & 0x07) | ((y & 0x1F) << 3);
    config[0x1D] = (config[0x1D] & !0x01) | (y >> 5);
    assert_eq!(
        KitConfig::decode(&config, PesVersion::Pes17)
            .unwrap()
            .name
            .y,
        y
    );
    config
}

// TC-BIN-24
#[test]
fn an_installed_loose_config_re_encoded_for_the_fpc_values_reports_the_value_it_clamps() {
    let sandbox = Sandbox::new("bins_fpc_loose_clamped");
    let installed = with_name_y(
        shirt_144_config(PesVersion::Pes17, 714, KitSlot::P1, false),
        40,
    );
    install_pre_fox(&sandbox, &installed);
    p2_export(&sandbox);
    write_fpc_player(&sandbox, "co Midcup Kits");

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);

    let expected = pre_fox_lines(
        &sandbox,
        &[
            "co Midcup Kits: Info kit_config_fpc_adjusted [Keep] (slot=p1)",
            "co Midcup Kits: Warning kit_config_version_clamped [Keep] (slot=p1, field=name.y, value=40, max=33)",
            "co Midcup Kits: Info kit_config_generated [Keep] at Kits/p2 ()",
            "co Midcup Kits: Warning kit_config_fpc_unpatched [Keep] (slot=p3)",
        ],
    );
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let p1 = KitConfig::decode(&entries[&config_path("1st")], PesVersion::Pes17).unwrap();
    assert!(kit_config::matches_fpc(&p1), "p1 carries the FPC values");
    assert_eq!(p1.name.y, 33);
}

// TC-BIN-23
#[test]
fn a_midcup_pes_17_export_s_collar_reaches_the_loose_configs_of_the_kits_it_does_not_hold() {
    let sandbox = Sandbox::new("bins_collar_loose_configs");
    install_pre_fox_p1(&sandbox);
    p2_export(&sandbox);
    sandbox.write(
        "exports/co Midcup Kits/Collars/collar_12.model",
        &pre_fox_model(),
    );

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);

    // No FPC finding; p2's template wears the collar, p3 has no config to wear it.
    assert_eq!(
        run.messages(),
        pre_fox_lines(
            &sandbox,
            &[
                "co Midcup Kits: Info kit_config_generated [Keep] at Kits/p2 ()",
                "co Midcup Kits: Warning kit_config_collar_unpatched [Keep] (slot=p3)",
            ]
        )
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let installed = shirt_144_config(PesVersion::Pes17, 714, KitSlot::P1, false);
    let mut expected = KitConfig::decode(&installed, PesVersion::Pes17).unwrap();
    expected.shirt.collar = 12;
    expected.shirt.winter_collar = 12;
    assert!(
        entries.get(&config_path("1st")) == Some(&expected.encode(PesVersion::Pes17).to_vec()),
        "p1 is the installed config wearing collar 12"
    );
    assert!(!entries.contains_key(&config_path("3rd")));
}

// TC-BIN-18
#[test]
fn a_full_pes_17_export_re_emits_no_installed_loose_config() {
    let sandbox = Sandbox::new("bins_full_loose_configs");
    install_pre_fox_p1(&sandbox);
    sandbox.write("exports/co Full Kits/Kits/p2/kit.dds", &tracer_kit());
    write_fpc_player(&sandbox, "co Full Kits");

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        lines.iter().all(|line| !line.contains("kit_config_fpc_")),
        "no FPC finding for p1 or p3: {lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(!entries.contains_key(&config_path("1st")));
    assert!(!entries.contains_key(&config_path("3rd")));
}

// TC-BIN-18
#[test]
fn the_nearest_cpk_s_loose_config_is_the_one_an_absent_slot_s_edit_starts_from() {
    let sandbox = Sandbox::new("bins_nearest_loose_config");
    install_pre_fox_p1(&sandbox);
    // Loaded above `4cc_08_bins.cpk`'s shirt-144 copy: p1 with the FPC values already.
    install_cpk(
        &sandbox,
        "4cc_61_midcup.cpk",
        &[(
            &config_path("1st"),
            &shirt_144_config(PesVersion::Pes17, 714, KitSlot::P1, true),
        )],
    );
    p2_export(&sandbox);
    write_fpc_player(&sandbox, "co Midcup Kits");

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);

    // p2's is generated (no installed p2 config in any CPK); p3's is unpatched.
    let expected = pre_fox_lines(
        &sandbox,
        &[
            "co Midcup Kits: Info kit_config_generated [Keep] at Kits/p2 ()",
            "co Midcup Kits: Warning kit_config_fpc_unpatched [Keep] (slot=p3)",
        ],
    );
    assert_eq!(run.messages(), expected, "nothing for p1");
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        !entries.contains_key(&config_path("1st")),
        "the installed p1 stands"
    );
}

// TC-BIN-16
#[test]
fn a_full_export_removes_its_team_s_installed_configs_of_kits_it_does_not_hold() {
    let sandbox = Sandbox::new("bins_full_configs");
    install_list(&sandbox);
    let installed = installed_configs(&[
        (714, KitSlot::P1),
        (714, KitSlot::P2),
        (714, KitSlot::P3),
        (702, KitSlot::P1),
    ]);
    install_cpk(
        &sandbox,
        "4cc_08_bins.cpk",
        &[
            (TEAM_COLOR, &bundled_team_color()),
            (UNI_COLOR, &bundled_uni_color()),
            (UNIFORM_PARAMETER, &installed),
        ],
    );
    sandbox.write("exports/co Full Kits/Kits/p1/kit.dds", &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let configs = configs_of(&entries, "714_");
    let names: Vec<&str> = configs.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        ["714_DEF_1st_realUni.bin", "714_DEF_GK1st_realUni.bin"],
        "p2 and p3 removed, the placeholder g1 added"
    );
    assert!(configs[0].1 == entries[&config_path("1st")]);
    assert!(configs[1].1 == entries[&config_path("GK1st")]);
    assert_eq!(
        configs_of(&entries, "702_"),
        [(
            "702_DEF_1st_realUni.bin".to_owned(),
            shirt_144_config(PesVersion::Pes21, 702, KitSlot::P1, false)
        )],
        "team 702's config is the installed one"
    );
}

// TC-BIN-07
#[test]
fn a_templates_file_replaces_the_bundled_base_and_one_that_cannot_be_read_stops_the_run() {
    let sandbox = Sandbox::new("bins_templates");
    // The bundled base with one color byte of team 701's first kit entry changed, so a CPK
    // built on the template is told from one built on the base.
    let base = bundled_uni_color();
    let mut record = uni_record(&base, 701).to_vec();
    record[7] = record[7].wrapping_add(1);
    let template = with_uni_record(&base, 701, &record);
    sandbox.write("data/templates/UniColor.bin", &template);
    p2_export(&sandbox);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let mut expected = vec![format!(
        "Info template_override_active [Keep] (path={})",
        sandbox.display("data/templates/UniColor.bin")
    )];
    expected.extend(BUNDLED_BINS.map(str::to_owned));
    expected.extend(P2_FINDINGS.map(str::to_owned));
    expected.push(deploy_skipped(&sandbox));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 0);
    let cpk = sandbox.root.join("output/4cc_99_test.cpk");
    let emitted = &cpk_entries(&cpk)[UNI_COLOR];
    assert!(
        *emitted == with_p2(&template),
        "UniColor.bin is the template with p2's entry set"
    );
    assert_eq!(uni_record(emitted, 701), record, "team 701's changed byte");

    // A folder where a template's file goes cannot be read as one.
    fs::remove_file(&cpk).unwrap();
    fs::create_dir_all(sandbox.root.join("data/templates/TeamColor.bin")).unwrap();

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    let [line] = lines.as_slice() else {
        panic!("one finding, on no export: {lines:#?}");
    };
    let prefix = format!(
        "Fatal template_override_unreadable [AbortRun] (path={}, error=",
        sandbox.display("data/templates/TeamColor.bin")
    );
    assert!(line.starts_with(&prefix), "{line}");
    assert_eq!(run.exit_code(), 3);
    assert!(!cpk.exists());
}

/// The boots list's path in a CPK.
pub(crate) const BOOTS_LIST: &str = "common/character0/model/character/boots/BootsList.bin";

/// The gloves list's path in a CPK.
pub(crate) const GLOVE_LIST: &str = "common/character0/model/character/glove/GloveList.bin";

/// The player appearance table's path in a CPK.
pub(crate) const PLAYER_APPEARANCE: &str =
    "common/character0/model/character/appearance/PlayerAppearance.bin";

/// A `BootsList.bin` or `GloveList.bin` holding `pairs` (player id, item id) as they are
/// given: two little-endian `u32` each.
pub(crate) fn item_list(pairs: &[(u32, u32)]) -> Vec<u8> {
    pairs
        .iter()
        .flat_map(|(player_id, item_id)| {
            player_id
                .to_le_bytes()
                .into_iter()
                .chain(item_id.to_le_bytes())
        })
        .collect()
}

/// Ten installed boots rows, sorted by player id, for players of `/a/` (702) and `/b/` (707):
/// none for a team 714 player.
const INSTALLED_BOOTS: [(u32, u32); 10] = [
    (70201, 11),
    (70202, 12),
    (70203, 13),
    (70204, 14),
    (70205, 15),
    (70701, 21),
    (70702, 22),
    (70703, 23),
    (70704, 24),
    (70705, 25),
];

/// Installed gloves rows for players of `/a/` and `/b/`.
const INSTALLED_GLOVES: [(u32, u32); 2] = [(70201, 31), (70701, 32)];

/// A `PlayerAppearance.bin` of two 60-byte rows, players 70201 and 70701, each row's 56
/// appearance bytes told apart by their value.
fn player_appearance() -> Vec<u8> {
    let mut bytes = Vec::new();
    for (player_id, fill) in [(70201u32, 0x0a), (70701, 0x0b)] {
        bytes.extend(player_id.to_le_bytes());
        bytes.extend([fill; 56]);
    }
    bytes
}

/// Installs `4cc_08_bins.cpk`, listed below the run's CPK, holding the bundled color and kit
/// config bins and `tables` (CPK path, bytes).
pub(crate) fn install_tables(sandbox: &Sandbox, tables: &[(&str, &[u8])]) {
    install_list(sandbox);
    let team_color = bundled_team_color();
    let uni_color = bundled_uni_color();
    let uniform_parameter = bundled_uniform_parameter();
    let mut entries: Vec<(&str, &[u8])> = vec![
        (TEAM_COLOR, &team_color),
        (UNI_COLOR, &uni_color),
        (UNIFORM_PARAMETER, &uniform_parameter),
    ];
    entries.extend_from_slice(tables);
    install_cpk(sandbox, "4cc_08_bins.cpk", &entries);
}

/// The CPK's `BootsList.bin` or `GloveList.bin` at `path` as its (player id, item id) pairs, in
/// file order.
pub(crate) fn pairs_of(entries: &BTreeMap<String, Vec<u8>>, path: &str) -> Vec<(u32, u32)> {
    let (pairs, rest) = entries[path].as_chunks::<8>();
    assert!(rest.is_empty(), "{path} is whole pairs");
    pairs
        .iter()
        .map(|&[p0, p1, p2, p3, i0, i1, i2, i3]| {
            (
                u32::from_le_bytes([p0, p1, p2, p3]),
                u32::from_le_bytes([i0, i1, i2, i3]),
            )
        })
        .collect()
}

/// The `aesthetics_patch.toml` the run wrote to the sandbox's output folder, parsed back.
fn patch_of(sandbox: &Sandbox) -> AestheticsPatch {
    let text = fs::read_to_string(sandbox.root.join("output/aesthetics_patch.toml")).unwrap();
    AestheticsPatch::parse(&text).unwrap()
}

// TC-BIN-25
#[test]
fn a_compiled_player_s_three_rows_join_the_installed_tables_which_keep_theirs() {
    let sandbox = Sandbox::new("bins_player_tables");
    let gloves = item_list(&INSTALLED_GLOVES);
    let appearance = player_appearance();
    install_tables(
        &sandbox,
        &[
            (BOOTS_LIST, &item_list(&INSTALLED_BOOTS)),
            (GLOVE_LIST, &gloves),
            (PLAYER_APPEARANCE, &appearance),
        ],
    );
    let folder = "exports/co Full Boots/Players/05 - A";
    sandbox.write(
        &format!("{folder}/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    let toml = "[appearance]\nskin_color = 3\n";
    sandbox.write(&format!("{folder}/settings.toml"), toml.as_bytes());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let lines = run.messages();
    assert_eq!(
        lines[..6],
        [
            source("TeamColor.bin", "4cc_08_bins.cpk"),
            source("UniColor.bin", "4cc_08_bins.cpk"),
            source("UniformParameter.bin", "4cc_08_bins.cpk"),
            source("BootsList.bin", "4cc_08_bins.cpk"),
            source("GloveList.bin", "4cc_08_bins.cpk"),
            source("PlayerAppearance.bin", "4cc_08_bins.cpk"),
        ]
    );
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("player_table_missing")),
        "{lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(entries.contains_key("Asset/model/character/boots/k0625/#Win/boots.fpk"));
    let mut expected = INSTALLED_BOOTS.to_vec();
    expected.push((71405, 625));
    assert_eq!(
        pairs_of(&entries, BOOTS_LIST),
        expected,
        "eleven pairs sorted by player id"
    );
    // No gloves and no marker: the Full export's default ID.
    let mut expected_gloves = INSTALLED_GLOVES.to_vec();
    expected_gloves.push((71405, 0));
    assert_eq!(
        pairs_of(&entries, GLOVE_LIST),
        expected_gloves,
        "the compiled player's gloves row gains the default ID"
    );
    // His settings.toml's row joins the two installed ones, sorted after them.
    let settings = PlayerSettings::parse(toml).unwrap();
    let row = settings.appearance_row(71405, PesVersion::Pes21).unwrap();
    assert_ne!(
        row,
        PlayerSettings::default()
            .appearance_row(71405, PesVersion::Pes21)
            .unwrap(),
        "the authored key changes the row"
    );
    let mut expected_appearance = appearance.clone();
    expected_appearance.extend(row);
    assert_eq!(entries[PLAYER_APPEARANCE], expected_appearance);
}

// TC-BIN-11
#[test]
fn a_player_whose_boots_task_fails_keeps_his_installed_boots_row() {
    let sandbox = Sandbox::new("bins_boots_failed");
    let installed = item_list(&[(70201, 11), (71405, 7)]);
    install_tables(&sandbox, &[(BOOTS_LIST, &installed)]);
    let folder = "exports/co Midcup Boots/Players/05 - A";
    // Two boots parts, one paired with a skeleton and one without: `skl_merge_conflict`
    // fails the boots task, and the folder's blank face still commits.
    for name in ["boots.fmdl", "kit_boots.fmdl"] {
        sandbox.write(
            &format!("{folder}/{name}"),
            &tracer_player_file("boots.fmdl"),
        );
    }
    sandbox.write(&format!("{folder}/boots.skl"), &body_skl("pes21"));
    sandbox.write(
        &format!("{folder}/shirt.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert!(
        lines.iter().any(|line| line
            == "co Midcup Boots: Error skl_merge_conflict [DropFolder] at Players/05 - A (skeleton=differs)"),
        "{lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(!entries.contains_key("Asset/model/character/boots/k0625/#Win/boots.fpk"));
    assert!(
        entries[BOOTS_LIST] == installed,
        "BootsList.bin keeps (71405, 7)"
    );
}

// TC-BIN-12
#[test]
fn a_player_s_own_gloves_and_a_linked_shared_folder_s_set_their_gloves_rows() {
    let sandbox = Sandbox::new("bins_gloves_rows");
    install_tables(&sandbox, &[(GLOVE_LIST, &item_list(&INSTALLED_GLOVES))]);
    let export = "exports/co Midcup Gloves";
    for name in ["glove_l.fmdl", "glove_r.fmdl"] {
        sandbox.write(
            &format!("{export}/Players/05 - A/{name}"),
            &tracer_player_file(name),
        );
        sandbox.write(
            &format!("{export}/Gloves/Keeper/{name}"),
            &tracer_player_file(name),
        );
    }
    sandbox.write(&format!("{export}/Players/07 - B/Keeper.gloves"), b"");
    sandbox.write(&format!("{export}/Players/08 - C/Keeper.gloves"), b"");
    // Grip, which slot 09 alone wears, takes his exclusive id.
    sandbox.write(&format!("{export}/Players/09 - D/Grip.gloves"), b"");
    sandbox.write(
        &format!("{export}/Gloves/Grip/glove_l.fmdl"),
        &tracer_player_file("glove_l.fmdl"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(entries.contains_key("Asset/model/character/glove/g0625/#Win/glove.fpk"));
    assert!(entries.contains_key("Asset/model/character/glove/g0644/#Win/glove.fpk"));
    assert!(entries.contains_key("Asset/model/character/glove/g0629/#Win/glove.fpk"));
    assert_eq!(
        pairs_of(&entries, GLOVE_LIST),
        [
            (70201, 31),
            (70701, 32),
            (71405, 625),
            (71407, 644),
            (71408, 644),
            (71409, 629)
        ]
    );
}

// TC-BIN-26
#[test]
fn a_full_export_zeroes_a_markerless_player_s_boots_row_and_a_midcup_keeps_it() {
    for (coverage, expected) in [
        ("Full", vec![(71405, 625), (71406, 0), (71407, 0)]),
        ("Midcup", vec![(71405, 625), (71406, 9), (71407, 0)]),
    ] {
        let sandbox = Sandbox::new(&format!("bins_boots_rows_{coverage}"));
        install_tables(
            &sandbox,
            &[(
                BOOTS_LIST,
                &item_list(&[(71405, 7), (71406, 9), (71407, 55)]),
            )],
        );
        let export = format!("exports/co {coverage} Boots");
        sandbox.write(
            &format!("{export}/Players/05 - A/boots.fmdl"),
            &tracer_player_file("boots.fmdl"),
        );
        sandbox.write(
            &format!("{export}/Players/06 - B/face_high.fmdl"),
            &clean_model(),
        );
        sandbox.write(
            &format!("{export}/Players/07 - C/face_high.fmdl"),
            &clean_model(),
        );
        sandbox.write(&format!("{export}/Players/07 - C/fpc_off"), b"");

        let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

        assert_eq!(run.exit_code(), 0, "{coverage}: {:#?}", run.messages());
        let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
        assert_eq!(pairs_of(&entries, BOOTS_LIST), expected, "{coverage}");
    }
}

// TC-BIN-27
#[test]
fn with_no_installed_tables_the_run_s_rows_are_left_out_and_each_table_reported() {
    let sandbox = Sandbox::new("bins_no_tables");
    sandbox.write(
        "exports/co Full Boots/Players/05 - A/boots.fmdl",
        &tracer_player_file("boots.fmdl"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let lines = run.messages();
    let missing: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains("player_table_missing"))
        .collect();
    assert_eq!(
        missing,
        [
            "Warning player_table_missing [Keep] (table=BootsList.bin, rows=1)",
            "Warning player_table_missing [Keep] (table=GloveList.bin, rows=1)",
            "Warning player_table_missing [Keep] (table=PlayerAppearance.bin, rows=1)",
        ]
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(entries.contains_key("Asset/model/character/boots/k0625/#Win/boots.fpk"));
    for table in [BOOTS_LIST, GLOVE_LIST, PLAYER_APPEARANCE] {
        assert!(!entries.contains_key(table), "no {table}");
    }
}

// TC-BIN-28
#[test]
fn a_compiled_player_s_appearance_row_is_his_settings_toml_s_not_the_installed_one() {
    let sandbox = Sandbox::new("bins_appearance_rows");
    // The installed rows the compile replaces: a row of set bytes for each of 71405 and
    // 71406, nothing like what a settings.toml builds.
    let mut installed = Vec::new();
    for player_id in [71405u32, 71406] {
        installed.extend(player_id.to_le_bytes());
        installed.extend([0xaa; 56]);
    }
    install_tables(&sandbox, &[(PLAYER_APPEARANCE, &installed)]);
    let export = "exports/co Full Appearance";
    // Slot 05's folder, with one appearance key authored; folder B, a face and no
    // settings.toml, mapped to 06 and 08.
    sandbox.write(
        "exports/co Full Appearance/players.txt",
        b"05 A\n06 B\n08 B\n",
    );
    let toml = "[appearance]\nskin_color = 3\n";
    sandbox.write(
        &format!("{export}/Players/A/settings.toml"),
        toml.as_bytes(),
    );
    sandbox.write(
        &format!("{export}/Players/B/face_high.fmdl"),
        &clean_model(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let table = AppearanceTable::read(
        PlayerTableFile::PlayerAppearance,
        &entries[PLAYER_APPEARANCE],
    )
    .unwrap();
    let default = PlayerSettings::default();
    let authored = PlayerSettings::parse(toml).unwrap();
    let with_key = authored.appearance_row(71405, PesVersion::Pes21).unwrap();
    let template = default.appearance_row(71406, PesVersion::Pes21).unwrap();
    assert_ne!(with_key[4..], template[4..], "the authored key sets a byte");
    assert_eq!(
        table.get(71405).copied(),
        Some(with_key[4..].try_into().unwrap()),
        "71405's row is the template's defaults with the key's value"
    );
    assert_eq!(
        table.get(71406).copied(),
        Some(template[4..].try_into().unwrap()),
        "71406's row is the template's defaults"
    );
    assert_eq!(
        table.get(71408).copied(),
        Some(template[4..].try_into().unwrap()),
        "71408's row is the template's defaults"
    );
    assert!(
        !entries[PLAYER_APPEARANCE]
            .windows(56)
            .any(|row| row.iter().all(|&byte| byte == 0xaa)),
        "nothing of either installed row remains"
    );
}

// TC-BIN-29
#[test]
fn the_authored_stock_id_the_marker_defaults_and_the_assigned_id_resolve_in_order() {
    let sandbox = Sandbox::new("bins_id_resolution");
    install_tables(
        &sandbox,
        &[
            (BOOTS_LIST, &item_list(&[(70201, 11)])),
            (GLOVE_LIST, &item_list(&[(70201, 31)])),
        ],
    );
    let export = "exports/co Full Ids";
    // Slot 05: fpc_on and an authored stock boots ID.
    sandbox.write(&format!("{export}/Players/05 - A/fpc_on"), b"");
    sandbox.write(
        &format!("{export}/Players/05 - A/settings.toml"),
        b"boots_id = 12\n",
    );
    // Slot 06: fpc_on, nothing else.
    sandbox.write(&format!("{export}/Players/06 - B/fpc_on"), b"");
    sandbox.write(
        &format!("{export}/Players/06 - B/face_high.fmdl"),
        &clean_model(),
    );
    // Slot 07: its own boots and an authored stock boots ID, which loses.
    sandbox.write(
        &format!("{export}/Players/07 - C/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(
        &format!("{export}/Players/07 - C/settings.toml"),
        b"boots_id = 12\n",
    );

    // A `settings.toml` finding: `check` and `compile` report it alike. Slot 07's own boots
    // take the assigned ID, so its key conflicts; slot 05's is used (the folder has no boots
    // model), and slot 06 sets none.
    for args in [&["check"][..], &["compile", "--no-deploy"][..]] {
        let run = sandbox.run(&pes21_settings(&sandbox), args);
        let lines = run.messages();
        assert_eq!(run.exit_code(), 0, "{args:?}: {lines:#?}");
        let conflicts: Vec<&str> = lines
            .iter()
            .filter(|line| line.contains("settings_model_id_conflict"))
            .map(String::as_str)
            .collect();
        assert_eq!(
            conflicts,
            [
                "co Full Ids: Warning settings_model_id_conflict [Keep] at Players/07 - C \
                 (key=boots_id)"
            ],
            "{args:?}"
        );
    }
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        pairs_of(&entries, BOOTS_LIST),
        [(70201, 11), (71405, 12), (71406, 55), (71407, 627)]
    );
    assert_eq!(
        pairs_of(&entries, GLOVE_LIST),
        [(70201, 31), (71405, 11), (71406, 11), (71407, 0)]
    );
}

// The hide preset's boots and gloves IDs are the common settings
// `fpc_hide_boots_id` and `fpc_hide_gloves_id` (55 and 11 by default): an
// `fpc_on` player with no models or authored IDs resolves to them, and a
// value outside the stock band (1 to 100) refuses the run before any
// output, the message naming the setting and its range.
// TC-BIN-43
#[test]
fn the_fpc_marker_defaults_are_the_common_settings_hide_ids() {
    let sandbox = Sandbox::new("bins_hide_ids_setting");
    install_tables(
        &sandbox,
        &[
            (BOOTS_LIST, &item_list(&[(70201, 11)])),
            (GLOVE_LIST, &item_list(&[(70201, 31)])),
        ],
    );
    let export = "exports/co Full Hide Ids";
    // Slot 05: fpc_on, no boots or gloves models and no authored IDs.
    sandbox.write(&format!("{export}/Players/05 - A/fpc_on"), b"");
    sandbox.write(
        &format!("{export}/Players/05 - A/face_high.fmdl"),
        &clean_model(),
    );

    let settings = format!(
        "{}fpc_hide_boots_id = 60\nfpc_hide_gloves_id = 12\n",
        pes21_settings(&sandbox)
    );
    let run = sandbox.run(&settings, &["compile", "--no-deploy"]);
    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        pairs_of(&entries, BOOTS_LIST).contains(&(71405, 60)),
        "BootsList.bin holds (71405, 60)"
    );
    assert!(
        pairs_of(&entries, GLOVE_LIST).contains(&(71405, 12)),
        "GloveList.bin holds (71405, 12)"
    );

    // fpc_hide_boots_id = 627 is outside the stock band: the run is
    // refused as an invalid configuration before any output.
    let sandbox = Sandbox::new("bins_hide_ids_refused");
    sandbox.write(&format!("{export}/Players/05 - A/fpc_on"), b"");
    let settings = format!("{}fpc_hide_boots_id = 627\n", pes21_settings(&sandbox));
    let run = sandbox.run(&settings, &["compile", "--no-deploy"]);
    run.assert_refused(2, &["fpc_hide_boots_id", "1 to 100"]);
    assert!(
        !sandbox.root.join("output").exists(),
        "the refusal wrote no output"
    );
}

// TC-BIN-41
#[test]
fn a_linked_shared_folder_s_gloves_id_wins_over_an_authored_gloves_id() {
    let sandbox = Sandbox::new("bins_gloves_link_conflict");
    install_tables(&sandbox, &[(GLOVE_LIST, &item_list(&[(70201, 31)]))]);
    let export = "exports/co Midcup Gloves Id";
    let folder = format!("{export}/Players/05 - A");
    // A plain link, the folder holding no gloves model: its assigned ID, his own as its one
    // wearer, is his row, and his settings.toml's stock `gloves_id` conflicts.
    sandbox.write(&format!("{folder}/Crocs.gloves"), b"");
    sandbox.write(&format!("{folder}/settings.toml"), b"gloves_id = 3\n");
    sandbox.write(
        &format!("{export}/Gloves/Crocs/glove_l.fmdl"),
        &tracer_player_file("glove_l.fmdl"),
    );

    for args in [&["check"][..], &["compile", "--no-deploy"][..]] {
        let run = sandbox.run(&pes21_settings(&sandbox), args);
        let lines = run.messages();
        assert_eq!(run.exit_code(), 0, "{args:?}: {lines:#?}");
        assert!(
            lines.iter().any(|line| line
                == "co Midcup Gloves Id: Warning settings_model_id_conflict [Keep] at \
                    Players/05 - A (key=gloves_id)"),
            "{args:?}: {lines:#?}"
        );
    }
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        pairs_of(&entries, GLOVE_LIST),
        [(70201, 31), (71405, 625)],
        "71405's row is his exclusive ID, Crocs's one wearer's, not the authored 3"
    );
}

// TC-BIN-41
#[test]
fn a_combined_face_folder_s_boots_part_assigns_the_id() {
    let sandbox = Sandbox::new("bins_face_link_boots_part");
    install_tables(&sandbox, &[(BOOTS_LIST, &item_list(&[(70201, 11)]))]);
    let export = "exports/co Midcup Face Boots";
    let folder = format!("{export}/Players/05 - A");
    // A face link always combines, and a `Faces/` folder may hold any model name: the
    // boots.fmdl Round holds is a part of his boots, which take his exclusive ID, and his
    // authored boots_id conflicts.
    sandbox.write(&format!("{folder}/Round.face"), b"");
    sandbox.write(&format!("{folder}/settings.toml"), b"boots_id = 3\n");
    sandbox.write(
        &format!("{export}/Faces/Round/face_high.fmdl"),
        &clean_model(),
    );
    sandbox.write(
        &format!("{export}/Faces/Round/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );

    for args in [&["check"][..], &["compile", "--no-deploy"][..]] {
        let run = sandbox.run(&pes21_settings(&sandbox), args);
        let lines = run.messages();
        assert_eq!(run.exit_code(), 0, "{args:?}: {lines:#?}");
        assert!(
            lines.iter().any(|line| line
                == "co Midcup Face Boots: Warning settings_model_id_conflict [Keep] at \
                    Players/05 - A (key=boots_id)"),
            "{args:?}: {lines:#?}"
        );
    }
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        pairs_of(&entries, BOOTS_LIST),
        [(70201, 11), (71405, 625)],
        "71405's row is his exclusive ID, not the authored 3"
    );
}

// TC-BIN-41
#[test]
fn a_plainly_linked_folder_holding_no_model_leaves_the_authored_id_its_row() {
    let sandbox = Sandbox::new("bins_plain_link_no_model");
    install_tables(&sandbox, &[(BOOTS_LIST, &item_list(&[(70201, 11)]))]);
    let export = "exports/co Midcup Plain";
    let folder = format!("{export}/Players/05 - A");
    // Crocs holds textures alone, no boots model: the link stays plain, the folder takes no
    // ID, and the authored boots_id is his row with no conflict.
    sandbox.write(&format!("{folder}/Crocs.boots"), b"");
    sandbox.write(&format!("{folder}/settings.toml"), b"boots_id = 3\n");
    sandbox.write(
        &format!("{export}/Boots/Crocs/crocs.dds"),
        &tracer_player_file("shirt.dds"),
    );

    for args in [&["check"][..], &["compile", "--no-deploy"][..]] {
        let run = sandbox.run(&pes21_settings(&sandbox), args);
        let lines = run.messages();
        assert_eq!(run.exit_code(), 0, "{args:?}: {lines:#?}");
        assert!(
            lines.iter().any(|line| line
                == "co Midcup Plain: Warning shared_folder_no_model [Keep] at Boots/Crocs \
                    (player=Players/05 - A)"),
            "{args:?}: {lines:#?}"
        );
        assert!(
            !lines
                .iter()
                .any(|line| line.contains("settings_model_id_conflict")),
            "{args:?}: {lines:#?}"
        );
    }
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        pairs_of(&entries, BOOTS_LIST),
        [(70201, 11), (71405, 3)],
        "the authored stock ID is his row, Crocs having nothing to assign"
    );
}

// TC-BIN-41
#[test]
fn a_hand_split_face_part_assigns_the_gloves_id_over_an_authored_one() {
    let sandbox = Sandbox::new("bins_hand_split_gloves_id");
    install_tables(&sandbox, &[(GLOVE_LIST, &item_list(&[(70201, 31)]))]);
    let export = "exports/co Midcup Hands Id";
    // A full-body model with both hands on (`tests/fixtures/hand_split/README.md`): the
    // hands split off it give him a gloves package, which takes his exclusive ID.
    let body =
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hand_split/body.fmdl"))
            .unwrap();
    sandbox.write(&format!("{export}/Players/05 - A/body.fmdl"), &body);
    sandbox.write(
        &format!("{export}/Players/05 - A/settings.toml"),
        b"gloves_id = 3\n",
    );

    for args in [&["check"][..], &["compile", "--no-deploy"][..]] {
        let run = sandbox.run(&pes21_settings(&sandbox), args);
        let lines = run.messages();
        assert_eq!(run.exit_code(), 0, "{args:?}: {lines:#?}");
        assert!(
            lines.iter().any(|line| line
                == "co Midcup Hands Id: Warning settings_model_id_conflict [Keep] at \
                    Players/05 - A (key=gloves_id)"),
            "{args:?}: {lines:#?}"
        );
    }
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        pairs_of(&entries, GLOVE_LIST),
        [(70201, 31), (71405, 625)],
        "71405's row is his exclusive ID, not the authored 3"
    );
}

// TC-BIN-48
#[test]
fn a_midcup_export_holding_only_settings_files_writes_the_tables() {
    let sandbox = Sandbox::new("bins_settings_only");
    install_tables(
        &sandbox,
        &[
            (BOOTS_LIST, &item_list(&[(70201, 11)])),
            (GLOVE_LIST, &item_list(&[(70201, 31)])),
            (PLAYER_APPEARANCE, &player_appearance()),
        ],
    );
    // `ingame_face.toml` keeps the folder's only work to its table rows: with no face model
    // it plans no package, so no task commits and only the rows are the run's.
    let face_toml = "[face]\nfacial_hair_type = 3\n";
    sandbox.write(
        "exports/co Midcup Settings/Players/05 - A/ingame_face.toml",
        face_toml.as_bytes(),
    );
    let toml = "boots_id = 12\n";
    sandbox.write(
        "exports/co Midcup Settings/Players/05 - A/settings.toml",
        toml.as_bytes(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    // The planned rows are the run's whole work, and still write the CPK.
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(pairs_of(&entries, BOOTS_LIST), [(70201, 11), (71405, 12)]);
    assert_eq!(pairs_of(&entries, GLOVE_LIST), [(70201, 31)]);
    let table = AppearanceTable::read(
        PlayerTableFile::PlayerAppearance,
        &entries[PLAYER_APPEARANCE],
    )
    .unwrap();
    let mut settings = PlayerSettings::parse(toml).unwrap();
    settings.set_ingame_face(IngameFaceSettings::parse(face_toml).unwrap());
    let row = settings.appearance_row(71405, PesVersion::Pes21).unwrap();
    assert_ne!(
        row[4..],
        PlayerSettings::default()
            .appearance_row(71405, PesVersion::Pes21)
            .unwrap()[4..],
        "the authored facial hair changes the row"
    );
    assert_eq!(
        table.get(71405).copied(),
        Some(row[4..].try_into().unwrap()),
        "71405's row is his two files'"
    );
}

// TC-BIN-49
#[test]
fn a_settings_value_past_the_target_s_range_writes_the_key_s_default() {
    let toml = "[appearance.motion]\nhunching_dribbling = 5\ndribbling = 2\n";
    let face_toml = "[face]\nfacial_hair_type = 20\n[hair]\nstyle = 4\ncolor_red = 40\n";
    for (number, version) in [(18u8, PesVersion::Pes18), (21, PesVersion::Pes21)] {
        let sandbox = Sandbox::new(&format!("bins_ranges_{number}"));
        install_tables(
            &sandbox,
            &[
                (BOOTS_LIST, &item_list(&INSTALLED_BOOTS)),
                (GLOVE_LIST, &item_list(&INSTALLED_GLOVES)),
                (PLAYER_APPEARANCE, &player_appearance()),
            ],
        );
        let export = format!("co Full Ranges{number}");
        sandbox.write(
            &format!("exports/{export}/Players/05 - A/settings.toml"),
            toml.as_bytes(),
        );
        sandbox.write(
            &format!("exports/{export}/Players/05 - A/ingame_face.toml"),
            face_toml.as_bytes(),
        );

        let run = sandbox.run(&pes_settings(&sandbox, number), &["compile", "--no-deploy"]);

        let lines = run.messages();
        assert_eq!(run.exit_code(), 0, "{number}: {lines:#?}");
        if version == PesVersion::Pes18 {
            let expected = format!(
                "{export}: Warning settings_value_out_of_range [Keep] at Players/05 - A \
                 (key=appearance.motion.hunching_dribbling, value=5, range=1 to 3)"
            );
            assert!(lines.contains(&expected), "{lines:#?}");
            // `dribbling` is a PES 20 and 21 key: dropped on PES 18, no finding names it.
            // PES 18's facial-hair cap is unmeasured, the field's bits:
            // the authored 20 is inside it, so no finding names it either.
            for key in ["appearance.motion.dribbling", "face.facial_hair_type"] {
                assert!(
                    !lines
                        .iter()
                        .any(|line| line.contains(&format!("key={key}"))),
                    "{key} in {lines:#?}"
                );
            }
        } else {
            // PES 21's measured facial-hair cap is 19: the authored 20 is
            // the only out-of-range value.
            let expected = format!(
                "{export}: Warning settings_value_out_of_range [Keep] at Players/05 - A \
                 (key=face.facial_hair_type, value=20, range=0 to 19)"
            );
            assert!(lines.contains(&expected), "{lines:#?}");
        }
        let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
        let table = AppearanceTable::read(
            PlayerTableFile::PlayerAppearance,
            &entries[PLAYER_APPEARANCE],
        )
        .unwrap();
        // The row is the export's two files narrowed to the target: on PES 18
        // the hunching is back at the template's 1 and the facial hair kept
        // at its authored 20 (the unmeasured cap is the field's bits); on
        // PES 21's measured cap the facial hair narrows to its 0 default.
        let mut expected = PlayerSettings::parse(toml).unwrap();
        expected.set_ingame_face(IngameFaceSettings::parse(face_toml).unwrap());
        expected.narrow_to(version);
        assert_eq!(
            expected.get(SettingKey::FacialHairType),
            Some(if version == PesVersion::Pes18 { 20 } else { 0 }),
            "{number}: the facial hair type"
        );
        assert_eq!(
            table.get(71405).copied(),
            Some(
                expected.appearance_row(71405, version).unwrap()[4..]
                    .try_into()
                    .unwrap()
            ),
            "{number}: 71405's row"
        );
        // Both runs' rows carry the authored in-game hair.
        let mut no_hair = expected.clone();
        no_hair.appearance.hair = Default::default();
        assert_ne!(
            expected.appearance_row(71405, version).unwrap()[4..],
            no_hair.appearance_row(71405, version).unwrap()[4..],
            "{number}: the in-game hair changes the row"
        );
        // The patch half: the narrowed hunching, `dribbling` only where the version holds it.
        let patch = fs::read_to_string(sandbox.root.join("output/aesthetics_patch.toml")).unwrap();
        if version == PesVersion::Pes18 {
            assert!(
                patch.contains("hunching_dribbling = 1"),
                "{number}: {patch}"
            );
            assert!(
                !patch.lines().any(|line| line.starts_with("dribbling")),
                "{number}: {patch}"
            );
        } else {
            assert!(
                patch.contains("hunching_dribbling = 5"),
                "{number}: {patch}"
            );
            assert!(patch.contains("dribbling = 2"), "{number}: {patch}");
        }
    }
}

// TC-STR-18
#[test]
fn an_invalid_ingame_face_toml_is_reported_and_the_face_is_the_default() {
    let sandbox = Sandbox::new("str_face_invalid");
    install_tables(&sandbox, &[(PLAYER_APPEARANCE, &player_appearance())]);
    let export = "exports/co Midcup Faces";
    let toml = "[appearance]\nskin_color = 4\n";
    // Slot 05's `ingame_face.toml` is not TOML at all; slot 06's holds a key
    // the file does not know.
    for (slot, face_toml) in [("05 - A", "this is not toml"), ("06 - B", "nonsense = 1\n")] {
        let player = format!("{export}/Players/{slot}");
        sandbox.write(&format!("{player}/settings.toml"), toml.as_bytes());
        sandbox.write(&format!("{player}/ingame_face.toml"), face_toml.as_bytes());
    }
    // `check` and `compile` report alike: each file's error on its own line.
    for args in [&["check"][..], &["compile", "--no-deploy"][..]] {
        let run = sandbox.run(&pes21_settings(&sandbox), args);
        let lines = run.messages();
        assert_eq!(run.exit_code(), 1, "{args:?}: {lines:#?}");
        for (slot, face_toml) in [("05 - A", "this is not toml"), ("06 - B", "nonsense = 1\n")] {
            let error = IngameFaceSettings::parse(face_toml)
                .unwrap_err()
                .to_string();
            assert!(
                lines.iter().any(|line| line
                    == &format!(
                        "co Midcup Faces: Error ingame_face_toml_invalid [Keep] at \
                         Players/{slot}/ingame_face.toml (file=ingame_face.toml, error={error})"
                    )),
                "{slot} {args:?}: {lines:#?}"
            );
        }
    }
    // Both players keep the in-game face: no face folder is built, and the
    // refused file's keys are ignored — the row is his `settings.toml`'s
    // with the game's default face.
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let paths: Vec<&str> = entries.keys().map(String::as_str).collect();
    assert!(
        paths.iter().all(|path| !path.contains("/face/real/7140")),
        "{paths:#?}"
    );
    let table = AppearanceTable::read(
        PlayerTableFile::PlayerAppearance,
        &entries[PLAYER_APPEARANCE],
    )
    .unwrap();
    let row = PlayerSettings::parse(toml)
        .unwrap()
        .appearance_row(71405, PesVersion::Pes21)
        .unwrap();
    assert_eq!(
        table.get(71405).copied(),
        Some(row[4..].try_into().unwrap()),
        "71405: skin colour 4, the template's face"
    );
    assert_eq!(
        table.get(71406).copied(),
        Some(row[4..].try_into().unwrap()),
        "71406: skin colour 4, the template's face"
    );
}

// TC-STR-19
#[test]
fn a_settings_toml_with_a_moved_key_is_invalid_and_the_face_file_still_applies() {
    let sandbox = Sandbox::new("str_face_keys_moved");
    install_tables(&sandbox, &[(PLAYER_APPEARANCE, &player_appearance())]);
    let export = "exports/co Midcup Moved";
    let player = format!("{export}/Players/05 - A");
    // `[appearance.hair]` is an unknown key in `settings.toml` now: the whole
    // file is refused, the in-game face's `iris_color` still applies.
    let toml = "[appearance]\nskin_color = 4\n[appearance.hair]\nlength = 5\n";
    sandbox.write(&format!("{player}/settings.toml"), toml.as_bytes());
    sandbox.write(&format!("{player}/ingame_face.toml"), b"iris_color = 7\n");

    for args in [&["check"][..], &["compile", "--no-deploy"][..]] {
        let run = sandbox.run(&pes21_settings(&sandbox), args);
        let lines = run.messages();
        assert_eq!(run.exit_code(), 1, "{args:?}: {lines:#?}");
        assert!(
            lines.iter().any(|line| line
                == "co Midcup Moved: Error settings_toml_invalid [DropFile] at \
                    Players/05 - A/settings.toml (file=settings.toml, \
                    error=appearance.hair: unknown key)"),
            "{args:?}: {lines:#?}"
        );
    }
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let table = AppearanceTable::read(
        PlayerTableFile::PlayerAppearance,
        &entries[PLAYER_APPEARANCE],
    )
    .unwrap();
    let mut settings = PlayerSettings::default();
    settings.set_ingame_face(IngameFaceSettings::parse("iris_color = 7\n").unwrap());
    assert_eq!(
        table.get(71405).copied(),
        Some(
            settings.appearance_row(71405, PesVersion::Pes21).unwrap()[4..]
                .try_into()
                .unwrap()
        ),
        "71405: iris 7, the template's skin colour 1"
    );
}

// TC-BIN-47
#[test]
fn an_ingame_face_toml_s_keys_reach_the_row_beside_a_plain_one_s_defaults() {
    let sandbox = Sandbox::new("bins_face_keys");
    install_tables(&sandbox, &[(PLAYER_APPEARANCE, &player_appearance())]);
    let export = "exports/egg Midcup Keys";
    let folder = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{folder}/face_high.fmdl"), &clean_model());
    let toml = "[appearance]\nskin_color = 3\n";
    sandbox.write(&format!("{folder}/settings.toml"), toml.as_bytes());
    let face_toml = "iris_color = 7\n[hair]\nlength = 5\n";
    sandbox.write(
        &format!("{export}/Players/06 - B/ingame_face.toml"),
        face_toml.as_bytes(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let table = AppearanceTable::read(
        PlayerTableFile::PlayerAppearance,
        &entries[PLAYER_APPEARANCE],
    )
    .unwrap();
    // 79205's row: his `settings.toml`'s, the template's defaults in every
    // face, hair and iris field.
    let skin = PlayerSettings::parse(toml)
        .unwrap()
        .appearance_row(79205, PesVersion::Pes21)
        .unwrap();
    assert_eq!(
        table.get(79205).copied(),
        Some(skin[4..].try_into().unwrap()),
        "79205's row"
    );
    // 79206's: iris 7 and hair length 5, every other face and hair field the
    // defaults.
    let mut settings = PlayerSettings::default();
    settings.set_ingame_face(IngameFaceSettings::parse(face_toml).unwrap());
    assert_eq!(
        table.get(79206).copied(),
        Some(
            settings.appearance_row(79206, PesVersion::Pes21).unwrap()[4..]
                .try_into()
                .unwrap()
        ),
        "79206's row"
    );
}

// TC-BIN-32
#[test]
fn a_failed_boots_task_keeps_the_installed_pair_and_the_appearance_row_is_still_set() {
    let sandbox = Sandbox::new("bins_boots_failed_row");
    install_tables(
        &sandbox,
        &[
            (BOOTS_LIST, &item_list(&[(71405, 7)])),
            (PLAYER_APPEARANCE, &player_appearance()),
        ],
    );
    let export = "exports/co Full Boots";
    let folder = format!("{export}/Players/05 - A");
    // Two boots parts, one paired with a skeleton and one without: `skl_merge_conflict`
    // fails the boots task, as in TC-BIN-11's test, and the folder's blank face still
    // commits.
    for name in ["boots.fmdl", "kit_boots.fmdl"] {
        sandbox.write(
            &format!("{folder}/{name}"),
            &tracer_player_file("boots.fmdl"),
        );
    }
    sandbox.write(&format!("{folder}/boots.skl"), &body_skl("pes21"));
    sandbox.write(
        &format!("{folder}/shirt.dds"),
        &tracer_player_file("shirt.dds"),
    );
    let toml = "[appearance]\nskin_color = 3\n[appearance.motion]\nhunching_dribbling = 5\n";
    sandbox.write(&format!("{folder}/settings.toml"), toml.as_bytes());
    // Slot 06's face compiles.
    sandbox.write(
        &format!("{export}/Players/06 - B/face_high.fmdl"),
        &clean_model(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert!(
        lines.iter().any(|line| line
            == "co Full Boots: Error skl_merge_conflict [DropFolder] at Players/05 - A (skeleton=differs)"),
        "{lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        pairs_of(&entries, BOOTS_LIST),
        [(71405, 7), (71406, 0)],
        "the failed task's row keeps the installed pair"
    );
    let table = AppearanceTable::read(
        PlayerTableFile::PlayerAppearance,
        &entries[PLAYER_APPEARANCE],
    )
    .unwrap();
    let authored = PlayerSettings::parse(toml).unwrap();
    let expected = authored.appearance_row(71405, PesVersion::Pes21).unwrap();
    assert_eq!(
        table.get(71405).copied(),
        Some(expected[4..].try_into().unwrap()),
        "71405's appearance row carries the key whatever the task's outcome"
    );
    // The patch half: his motion keys, whatever the failed task's outcome.
    let patch = patch_of(&sandbox);
    let motion = &patch.teams[0].players[&5].motion;
    assert_eq!(
        *motion,
        authored.appearance.motion.resolved(PesVersion::Pes21)
    );
}

// TC-BIN-33
#[test]
fn an_invalid_settings_toml_is_reported_and_the_row_is_the_template_s_defaults() {
    let sandbox = Sandbox::new("bins_settings_invalid_row");
    // The installed row for 71405 with no value at its default: every byte flipped.
    let template = PlayerSettings::default()
        .appearance_row(71405, PesVersion::Pes21)
        .unwrap();
    let mut installed = 71405u32.to_le_bytes().to_vec();
    installed.extend(template[4..].iter().map(|byte| byte ^ 0xff));
    install_tables(&sandbox, &[(PLAYER_APPEARANCE, &installed)]);
    let export = "exports/co Full Settings";
    sandbox.write(
        &format!("{export}/Players/05 - A/face_high.fmdl"),
        &clean_model(),
    );
    // `name` takes `true` or a string.
    sandbox.write(
        &format!("{export}/Players/05 - A/settings.toml"),
        b"name = 5\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert!(
        lines.iter().any(|line| line
            == "co Full Settings: Error settings_toml_invalid [DropFile] at \
                Players/05 - A/settings.toml (file=settings.toml, error=name: expected true or a string)"),
        "{lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let table = AppearanceTable::read(
        PlayerTableFile::PlayerAppearance,
        &entries[PLAYER_APPEARANCE],
    )
    .unwrap();
    assert_eq!(
        table.get(71405).copied(),
        Some(template[4..].try_into().unwrap()),
        "the dropped file leaves the template's row"
    );
    // The patch half: his motions at their defaults, the dropped file giving none.
    let patch = patch_of(&sandbox);
    assert_eq!(
        patch.teams[0].players[&5].motion,
        PlayerSettings::default()
            .appearance
            .motion
            .resolved(PesVersion::Pes21)
    );
}

// TC-BIN-30
#[test]
fn an_fpc_on_player_gets_the_hide_preset_with_the_interference_reported() {
    let sandbox = Sandbox::new("bins_fpc_strip");
    install_tables(&sandbox, &[(PLAYER_APPEARANCE, &player_appearance())]);
    let export = "exports/co Full Fpc";
    // Slot 05: `fpc_on`; his `sleeves` is the one key the preset sets that he set otherwise.
    let toml = "[appearance.strip]\nsleeves = \"short\"\ninners = \"normal\"\n\
        undershorts = \"short\"\nankle_taping = true\n";
    sandbox.write(&format!("{export}/Players/05 - A/fpc_on"), b"");
    sandbox.write(
        &format!("{export}/Players/05 - A/settings.toml"),
        toml.as_bytes(),
    );
    // Slot 06: no marker, the gloves checkbox on and no gloves model.
    sandbox.write(
        &format!("{export}/Players/06 - B/face_high.fmdl"),
        &clean_model(),
    );
    sandbox.write(
        &format!("{export}/Players/06 - B/settings.toml"),
        b"[appearance.strip]\ngloves = true\n",
    );

    // A `settings.toml` finding: `check` and `compile` report it alike.
    for args in [&["check"][..], &["compile", "--no-deploy"][..]] {
        let run = sandbox.run(&pes21_settings(&sandbox), args);
        let lines = run.messages();
        assert_eq!(run.exit_code(), 0, "{args:?}: {lines:#?}");
        let fpc: Vec<&str> = lines
            .iter()
            .filter(|line| {
                [
                    "fpc_strip_conflict",
                    "fpc_inners_break_hiding",
                    "fpc_undershorts_break_hiding",
                    "fpc_taping_shows_pieces",
                    "fpc_gloves_checkbox_winter_gloves",
                ]
                .iter()
                .any(|code| line.contains(code))
            })
            .map(String::as_str)
            .collect();
        assert_eq!(
            fpc,
            [
                "co Full Fpc: Warning fpc_strip_conflict [Keep] at Players/05 - A \
                 (key=appearance.strip.sleeves)",
                "co Full Fpc: Warning fpc_inners_break_hiding [Keep] at Players/05 - A ()",
                "co Full Fpc: Warning fpc_undershorts_break_hiding [Keep] at Players/05 - A ()",
                "co Full Fpc: Info fpc_taping_shows_pieces [Keep] at Players/05 - A ()",
                "co Full Fpc: Warning fpc_gloves_checkbox_winter_gloves [Keep] at \
                 Players/06 - B ()",
            ],
            "{args:?}"
        );
    }
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let table = AppearanceTable::read(
        PlayerTableFile::PlayerAppearance,
        &entries[PLAYER_APPEARANCE],
    )
    .unwrap();
    // The preset's sleeves win; the inners, undershorts and taping are as authored.
    let mut expected = PlayerSettings::parse(toml).unwrap();
    expected.apply_fpc_preset(&fpc::preset(fpc::Preset::Hide, fpc::HideIds::CONVENTION));
    assert_eq!(
        expected.get(SettingKey::Sleeves),
        Some(2),
        "the preset's long"
    );
    assert_eq!(
        table.get(71405).copied(),
        Some(
            expected.appearance_row(71405, PesVersion::Pes21).unwrap()[4..]
                .try_into()
                .unwrap()
        ),
        "71405's row"
    );
}

// TC-BIN-30
#[test]
fn a_midcup_markerless_player_is_not_checked_for_the_gloves_rule() {
    let sandbox = Sandbox::new("bins_fpc_midcup_gloves");
    // No marker, `gloves = true` and no gloves model: under a `Midcup` export his
    // installed row stays, which `check` cannot read, so no gloves finding.
    let export = "exports/co Midcup Gloves";
    sandbox.write(
        &format!("{export}/Players/05 - A/face_high.fmdl"),
        &clean_model(),
    );
    sandbox.write(
        &format!("{export}/Players/05 - A/settings.toml"),
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

/// Team 714's record of which shared folder took which of its block's shared IDs, at a path
/// no game reads.
pub(crate) const SHARED_IDS_714: &str = "unlimited-tool-works/shared_ids/714.toml";

/// The record file's text for `boots` (shared ID, folder name); the `[gloves]` table is
/// always written, here empty.
pub(crate) fn record(boots: &[(u16, &str)]) -> Vec<u8> {
    let mut text = "# Written by the Team compiler; do not edit.\n[boots]\n".to_owned();
    for (id, name) in boots {
        text.push_str(&format!("{id} = \"{name}\"\n"));
    }
    (text + "[gloves]\n").into_bytes()
}

// TC-BIN-34
#[test]
fn a_full_export_s_shared_ids_are_numbered_alphabetically_and_recorded() {
    let sandbox = Sandbox::new("bins_shared_ids_full");
    install_tables(&sandbox, &[(BOOTS_LIST, &item_list(&[(70201, 11)]))]);
    let export = "exports/co Full Shared";
    for folder in ["adidas", "Birk", "Crocs"] {
        sandbox.write(
            &format!("{export}/Boots/{folder}/boots.fmdl"),
            &clean_model(),
        );
    }
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    sandbox.write(&format!("{export}/Players/07 - C/Crocs.boots"), b"");
    sandbox.write(&format!("{export}/Players/06 - B/adidas.boots"), b"");
    sandbox.write(&format!("{export}/Players/08 - D/adidas.boots"), b"");
    // Birk, which slot 09 alone links, takes his exclusive id and is not recorded.
    sandbox.write(&format!("{export}/Players/09 - E/Birk.boots"), b"");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        pairs_of(&entries, BOOTS_LIST),
        [
            (70201, 11),
            (71405, 645),
            (71406, 644),
            (71407, 645),
            (71408, 644),
            (71409, 629)
        ]
    );
    assert_eq!(
        entries[SHARED_IDS_714],
        record(&[(644, "adidas"), (645, "Crocs")]),
        "the CPK records which folder took which shared ID"
    );
}

// TC-BIN-35
#[test]
fn a_midcup_export_keeps_the_installed_shared_ids_for_the_folders_it_resends() {
    let sandbox = Sandbox::new("bins_shared_ids_midcup");
    let installed = record(&[(644, "adidas"), (645, "Crocs")]);
    install_tables(
        &sandbox,
        &[
            (BOOTS_LIST, &item_list(&[(70201, 11)])),
            (SHARED_IDS_714, installed.as_slice()),
        ],
    );
    let export = "exports/co Midcup Shared";
    for folder in ["Birk", "crocs"] {
        sandbox.write(
            &format!("{export}/Boots/{folder}/boots.fmdl"),
            &clean_model(),
        );
    }
    sandbox.write(&format!("{export}/Players/05 - A/Birk.boots"), b"");
    sandbox.write(&format!("{export}/Players/07 - C/Birk.boots"), b"");
    sandbox.write(&format!("{export}/Players/06 - B/crocs.boots"), b"");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("shared_ids_unrecorded")),
        "{lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        pairs_of(&entries, BOOTS_LIST),
        [(70201, 11), (71405, 646), (71406, 645), (71407, 646)],
        "Birk takes the lowest free ID, crocs keeps the recorded one though one slot wears it"
    );
    assert_eq!(
        entries[SHARED_IDS_714],
        record(&[(644, "adidas"), (645, "crocs"), (646, "Birk")]),
        "the installed entries kept, the Midcup's folders under their own spellings"
    );
}

// TC-BIN-36
#[test]
fn a_midcup_export_with_no_installed_record_is_warned_and_records_its_own() {
    // The sandbox's own name must not hold the code it greps for.
    let sandbox = Sandbox::new("bins_no_installed_record");
    let appearance = player_appearance();
    install_tables(
        &sandbox,
        &[
            (BOOTS_LIST, &item_list(&[(70201, 11)])),
            (PLAYER_APPEARANCE, &appearance),
        ],
    );
    let export = "exports/co Midcup Shared";
    sandbox.write(&format!("{export}/Boots/Birk/boots.fmdl"), &clean_model());
    sandbox.write(&format!("{export}/Players/05 - A/Birk.boots"), b"");
    sandbox.write(&format!("{export}/Players/06 - B/Birk.boots"), b"");

    // `check` reports the missing record as `compile` does.
    for args in [&["check"][..], &["compile", "--no-deploy"][..]] {
        let run = sandbox.run(&pes21_settings(&sandbox), args);
        let lines = run.messages();
        assert!(
            lines
                .iter()
                .any(|line| line
                    == "co Midcup Shared: Warning shared_ids_unrecorded [Keep] (team=714)"),
            "{args:?}: {lines:#?}"
        );
    }
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(
        pairs_of(&entries, BOOTS_LIST),
        [(70201, 11), (71405, 644), (71406, 644)]
    );
    assert_eq!(entries[SHARED_IDS_714], record(&[(644, "Birk")]));

    // Slot 06's link removed, Birk's one wearer is slot 05: it takes his exclusive id, and
    // no folder taking a shared id means no record and no warning.
    fs::remove_file(
        sandbox
            .root
            .join(format!("{export}/Players/06 - B/Birk.boots")),
    )
    .unwrap();
    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("shared_ids_unrecorded")),
        "{lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(pairs_of(&entries, BOOTS_LIST), [(70201, 11), (71405, 625)]);
    assert!(
        !entries.contains_key(SHARED_IDS_714),
        "{:?}",
        entries.keys()
    );
}

// TC-BIN-39
#[test]
fn a_midcup_export_with_nothing_to_record_writes_no_shared_ids_record() {
    let sandbox = Sandbox::new("bins_shared_ids_none");
    install_tables(&sandbox, &[(BOOTS_LIST, &item_list(&[(70201, 11)]))]);
    // A settings-only player folder: no shared folder of either kind takes an ID, so there
    // is no record to write and none to warn about.
    let export = "exports/co Midcup Settings";
    sandbox.write(&format!("{export}/Players/05 - A/ingame_face.toml"), b"");
    sandbox.write(
        &format!("{export}/Players/05 - A/settings.toml"),
        b"boots_id = 12\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("shared_ids_unrecorded")),
        "{lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        !entries.contains_key(SHARED_IDS_714),
        "an empty record would read to the next Midcup as \"no shared folders\""
    );
}

// TC-BIN-40
#[test]
fn a_midcup_export_with_nothing_to_record_rewrites_the_installed_one() {
    let sandbox = Sandbox::new("bins_shared_ids_empty");
    let installed = record(&[]);
    install_tables(
        &sandbox,
        &[
            (BOOTS_LIST, &item_list(&[(70201, 11)])),
            (SHARED_IDS_714, installed.as_slice()),
        ],
    );
    // The same settings-only export as the no-record case: it carries the record it
    // inherited, even an empty one (a `Full` export with no shared folder writes one).
    let export = "exports/co Midcup Settings";
    sandbox.write(&format!("{export}/Players/05 - A/ingame_face.toml"), b"");
    sandbox.write(
        &format!("{export}/Players/05 - A/settings.toml"),
        b"boots_id = 12\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("shared_ids_unrecorded")),
        "{lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(entries[SHARED_IDS_714], installed);
}

// TC-BIN-40
#[test]
fn a_full_export_with_nothing_to_record_writes_an_empty_one() {
    let sandbox = Sandbox::new("bins_shared_ids_full_empty");
    install_tables(&sandbox, &[(BOOTS_LIST, &item_list(&[(70201, 11)]))]);
    // A `Full` export's record means "no shared folders": it writes it even empty.
    let export = "exports/co Full Settings";
    sandbox.write(&format!("{export}/Players/05 - A/ingame_face.toml"), b"");
    sandbox.write(
        &format!("{export}/Players/05 - A/settings.toml"),
        b"boots_id = 12\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("shared_ids_unrecorded")),
        "{lines:#?}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert_eq!(entries[SHARED_IDS_714], record(&[]));
}

// TC-BIN-30
#[test]
fn a_face_link_feeding_the_own_package_is_no_gloves_id_for_the_checkbox_rule() {
    let sandbox = Sandbox::new("bins_face_link_gloves_rule");
    let export = "exports/co Full Face Link";
    // A shared face feeds his own package always, but it is no gloves ID: the gloves
    // checkbox with no gloves warns as it does without a link.
    sandbox.write(&format!("{export}/Players/05 - A/Round.face"), b"");
    sandbox.write(
        &format!("{export}/Players/05 - A/settings.toml"),
        b"[appearance.strip]\ngloves = true\n",
    );
    sandbox.write(
        &format!("{export}/Faces/Round/face_high.fmdl"),
        &clean_model(),
    );

    for args in [&["check"][..], &["compile", "--no-deploy"][..]] {
        let run = sandbox.run(&pes21_settings(&sandbox), args);
        let lines = run.messages();
        assert_eq!(run.exit_code(), 0, "{args:?}: {lines:#?}");
        assert!(
            lines.iter().any(|line| line
                == "co Full Face Link: Warning fpc_gloves_checkbox_winter_gloves [Keep] at \
                    Players/05 - A ()"),
            "{args:?}: {lines:#?}"
        );
    }
}
