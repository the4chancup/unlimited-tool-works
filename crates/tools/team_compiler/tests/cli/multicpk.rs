//! Multi-CPK mode (`spec/team_compiler/pipeline.md` "Multi-CPK mode: teams parts"): whole
//! teams filled first-fit into the official list's `teams` slots under
//! `cpk_part_max_size`, the placeholder in every slot left over, the overrides and the bins in
//! the bins CPK; every CPK installed into `download/` or none; and the single CPK's warning
//! past the same cap.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use uniparam::UniformParameter;

use crate::bins::dpfl_bytes;
use crate::common::{Run, Sandbox};
use crate::compile::{cpk_entries, kit_texture, pes21_settings, tracer_export, tracer_player_file};
use crate::compile_exports::{TEAM_COLOR, UNI_COLOR, UNIFORM_PARAMETER};
use crate::deploy::{install_pes, templates_folder};
use crate::patch_written;
use crate::referees::{REFEREE_CONFIGS, write_refs_with_marker};
use crate::snapshot;
use crate::upgrade::pes17_list;

/// The tracer's copies, one per team: /a/ 702, /b/ 707 and /co/ 714, in canonical order.
const EXPORTS: [&str; 3] = ["a Midcup Tracer", "b Midcup Tracer", "co Midcup Tracer"];

/// The refs CPK's file name at the default `refs_cpk_name`.
const REFS_CPK: &str = "4cc_18_referees.cpk";

/// The teams slots of the official list.
const TEAMS_SLOTS: [&str; 5] = [
    "4cc_41_teams.cpk",
    "4cc_42_teams.cpk",
    "4cc_43_teams.cpk",
    "4cc_44_teams.cpk",
    "4cc_45_teams.cpk",
];

/// A sandbox holding the tracer as each export of `exports`.
fn sandbox_with(name: &str, exports: &[&str]) -> Sandbox {
    let sandbox = Sandbox::new(name);
    for export in exports {
        sandbox.copy_tracer(export);
    }
    sandbox
}

/// PES 21 settings with `multicpk_mode` on and the `[team-compiler]` lines `extra`.
fn multicpk_settings(sandbox: &Sandbox, extra: &str) -> String {
    format!(
        "{}[team-compiler]\nmulticpk_mode = true\n{extra}",
        pes21_settings(sandbox)
    )
}

/// `compile --no-deploy` in `sandbox` with `settings`.
fn compile(sandbox: &Sandbox, settings: &str) -> Run {
    sandbox.run(settings, &["compile", "--no-deploy"])
}

/// The names of the files directly in the sandbox's `output/`.
fn output_files(sandbox: &Sandbox) -> BTreeSet<String> {
    fs::read_dir(sandbox.root.join("output"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect()
}

/// The shipped placeholder CPK's bytes.
fn placeholder() -> Vec<u8> {
    fs::read(templates_folder().join("placeholder.cpk")).unwrap()
}

/// The line `deploy_skipped_by_flag` reports for `output/<name>`.
fn skipped(sandbox: &Sandbox, name: &str) -> String {
    format!(
        "Info deploy_skipped_by_flag [Keep] (path={})",
        sandbox.display(&format!("output/{name}"))
    )
}

/// `cpk_part_max_size` as the findings show it: `N bytes` under 1 KiB, else in the largest of
/// KiB, MiB and GiB it reaches, to one decimal, rounded to the nearest, a trailing `.0` left
/// out.
fn size_text(bytes: u64) -> String {
    let units = [("GiB", 1u64 << 30), ("MiB", 1 << 20), ("KiB", 1 << 10)];
    let Some((unit, size)) = units.into_iter().find(|(_, size)| bytes >= *size) else {
        return format!("{bytes} bytes");
    };
    let tenths = (bytes * 10 + size / 2) / size;
    match tenths % 10 {
        0 => format!("{} {unit}", tenths / 10),
        digit => format!("{}.{digit} {unit}", tenths / 10),
    }
}

/// The length of the first teams part, and its entries, when /a/ and /b/ alone are compiled
/// at the default cap in the sandbox `name`: a cap at which they fill one part exactly.
fn first_part_of_a_and_b(name: &str) -> (u64, BTreeMap<String, Vec<u8>>) {
    let sandbox = sandbox_with(name, &EXPORTS[..2]);
    let run = compile(&sandbox, &multicpk_settings(&sandbox, ""));
    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let part = sandbox.root.join("output/4cc_41_teams.cpk");
    (fs::metadata(&part).unwrap().len(), cpk_entries(&part))
}

// TC-OUT-12
#[test]
fn teams_fill_the_slots_first_fit_and_the_bins_go_into_the_bins_cpk() {
    let (cap, a_and_b) = first_part_of_a_and_b("multicpk_parts_a_and_b");
    let sandbox = sandbox_with("multicpk_parts", &EXPORTS);
    let settings = multicpk_settings(&sandbox, &format!("cpk_part_max_size = {cap}\n"));

    let run = compile(&sandbox, &settings);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let output = sandbox.root.join("output");
    let mut expected_files: BTreeSet<String> = TEAMS_SLOTS.map(str::to_owned).into();
    expected_files.insert("4cc_08_bins.cpk".to_owned());
    expected_files.insert("aesthetics_patch.toml".to_owned());
    assert_eq!(
        output_files(&sandbox),
        expected_files,
        "no 99_test, no teams2"
    );
    assert!(
        cpk_entries(&output.join("4cc_41_teams.cpk")) == a_and_b,
        "the first part holds exactly /a/ and /b/"
    );
    let second = cpk_entries(&output.join("4cc_42_teams.cpk"));
    assert!(second.contains_key(&kit_texture("u0714g1")), "/co/'s kit");
    for team in ["u0702g1", "u0707g1"] {
        assert!(!second.contains_key(&kit_texture(team)), "{team}");
    }
    for slot in &TEAMS_SLOTS[2..] {
        assert!(
            fs::read(output.join(slot)).unwrap() == placeholder(),
            "{slot} is the placeholder"
        );
    }
    let bins: Vec<String> = cpk_entries(&output.join("4cc_08_bins.cpk"))
        .into_keys()
        .collect();
    let mut expected_bins = vec![
        TEAM_COLOR.to_owned(),
        UNI_COLOR.to_owned(),
        UNIFORM_PARAMETER.to_owned(),
    ];
    expected_bins.sort();
    assert_eq!(bins, expected_bins);
    let lines = run.messages();
    let mut expected_end = vec![skipped(&sandbox, "4cc_08_bins.cpk")];
    expected_end.extend(TEAMS_SLOTS.map(|slot| skipped(&sandbox, slot)));
    expected_end.push(patch_written(&sandbox, "702, 707, 714"));
    assert_eq!(lines[lines.len() - 7..], expected_end);
    // `Complete`'s files_written is every entry of every CPK the run wrote: the bins CPK,
    // the two parts it filled and the placeholders of the slots it left.
    let written: usize = ["4cc_08_bins.cpk", "4cc_41_teams.cpk", "4cc_42_teams.cpk"]
        .iter()
        .map(|name| cpk_entries(&output.join(name)).len())
        .sum::<usize>()
        + TEAMS_SLOTS[2..]
            .iter()
            .map(|slot| cpk_entries(&output.join(slot)).len())
            .sum::<usize>();
    assert_eq!(run.complete().files_written, written);
    assert!(!output.join(".staging").exists());
}

// TC-REF-12
#[test]
fn a_refs_export_alone_on_fox_writes_the_bins_cpk_for_its_configs_and_every_part_a_placeholder() {
    let sandbox = Sandbox::new("multicpk_refs_alone");
    write_refs_with_marker(&sandbox, Some(&tracer_player_file("shirt.dds")));

    let run = compile(&sandbox, &multicpk_settings(&sandbox, ""));

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let output = sandbox.root.join("output");
    let mut expected_files: BTreeSet<String> = run_cpks().into_iter().collect();
    expected_files.insert(REFS_CPK.to_owned());
    assert_eq!(output_files(&sandbox), expected_files);
    // The bins alone: no team committed anything.
    let bins = cpk_entries(&output.join("4cc_08_bins.cpk"));
    let mut expected_bins = vec![
        TEAM_COLOR.to_owned(),
        UNI_COLOR.to_owned(),
        UNIFORM_PARAMETER.to_owned(),
    ];
    expected_bins.sort();
    assert_eq!(bins.keys().cloned().collect::<Vec<_>>(), expected_bins);
    let uniform_parameter = UniformParameter::read(&bins[UNIFORM_PARAMETER]).unwrap();
    let refs = cpk_entries(&output.join(REFS_CPK));
    let mut configs = 0;
    for (path, bytes) in &refs {
        let Some(name) = path.strip_prefix(REFEREE_CONFIGS) else {
            continue;
        };
        configs += 1;
        assert!(
            uniform_parameter.get(name) == Some(bytes.as_slice()),
            "{path}: the refs CPK's loose file"
        );
    }
    assert_eq!(configs, 20);
    for slot in TEAMS_SLOTS {
        assert!(
            fs::read(output.join(slot)).unwrap() == placeholder(),
            "{slot} is the placeholder"
        );
    }
    let mut expected_end: Vec<String> = run_cpks()
        .iter()
        .map(|name| skipped(&sandbox, name))
        .collect();
    expected_end.push(skipped(&sandbox, REFS_CPK));
    assert_eq!(lines[lines.len() - 7..], expected_end);
}

/// Asserts `run` in `sandbox` was aborted with the Fatal `line` last, and wrote no CPK.
fn assert_aborted_with(sandbox: &Sandbox, run: &Run, line: impl Fn(&str) -> bool) {
    assert_eq!(run.exit_code(), 3);
    let lines = run.messages();
    let last = lines.last().unwrap();
    assert!(line(last), "{lines:#?}");
    let written: Vec<_> = snapshot(&sandbox.root.join("output"))
        .into_keys()
        .filter(|path| path.extension().is_some_and(|extension| extension == "cpk"))
        .collect();
    assert_eq!(
        written,
        Vec::<std::path::PathBuf>::new(),
        "no part is written"
    );
    assert!(!sandbox.root.join("output/.staging").exists());
}

// TC-OUT-13
#[test]
fn a_team_larger_than_the_cap_aborts_the_run_and_writes_no_part() {
    let sandbox = sandbox_with("multicpk_team_over_cap", &EXPORTS);
    let settings = multicpk_settings(&sandbox, "cpk_part_max_size = 4096\n");

    let run = compile(&sandbox, &settings);

    assert_aborted_with(&sandbox, &run, |line| {
        line.starts_with("Fatal cpk_team_exceeds_cap [AbortRun] (export=a Midcup Tracer, size=")
            && line.ends_with(", cap=4 KiB)")
    });
}

// TC-OUT-14
#[test]
fn content_past_the_last_slot_aborts_the_run_naming_the_shortfall() {
    let (cap, _) = first_part_of_a_and_b("multicpk_slots_exhausted_a_and_b");
    let sandbox = sandbox_with("multicpk_slots_exhausted", &EXPORTS);
    sandbox.write(
        "data/templates/DpFileList.bin",
        &dpfl_bytes(&["4cc_08_bins.cpk", "4cc_41_teams.cpk", "4cc_99_test.cpk"]),
    );
    let settings = multicpk_settings(&sandbox, &format!("cpk_part_max_size = {cap}\n"));

    let run = compile(&sandbox, &settings);

    let expected = format!(
        "Fatal cpk_slots_exhausted [AbortRun] (export=co Midcup Tracer, stem=teams, slots=1, \
         cap={})",
        size_text(cap)
    );
    assert_aborted_with(&sandbox, &run, |line| line == expected);
}

// TC-OUT-15
#[test]
fn a_single_cpk_past_the_cap_is_a_warning_and_is_written_whole() {
    let sandbox = Sandbox::new("single_cpk_over_cap");
    let settings = format!(
        "{}[team-compiler]\ncpk_part_max_size = 4096\n",
        pes21_settings(&sandbox)
    );

    let run = sandbox.run(
        &settings,
        &["compile", "--no-deploy", "--export", &tracer_export()],
    );

    assert_eq!(run.exit_code(), 0);
    let lines = run.messages();
    let warning = lines
        .iter()
        .find(|line| line.contains("cpk_size_over_limit"))
        .unwrap_or_else(|| panic!("{lines:#?}"));
    assert!(
        warning.starts_with("Warning cpk_size_over_limit [Keep] (cpk=4cc_99_test.cpk, size=")
            && warning.ends_with(", cap=4 KiB)"),
        "{warning}"
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(entries.contains_key(&kit_texture("u0792g1")), "the kit");
}

// TC-OUT-16
#[test]
fn a_second_slot_run_is_filled_by_its_own_stem_and_the_first_is_left_alone() {
    let sandbox = sandbox_with("multicpk_teams2", &EXPORTS);
    sandbox.write(
        "data/templates/DpFileList.bin",
        &dpfl_bytes(&["4cc_08_bins.cpk", "4cc_41_teams.cpk", "4cc_51_teams2.cpk"]),
    );
    let settings = multicpk_settings(&sandbox, "teams_cpk_name = \"teams2\"\n");

    let run = compile(&sandbox, &settings);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    assert_eq!(
        output_files(&sandbox),
        BTreeSet::from([
            "4cc_08_bins.cpk".to_owned(),
            "4cc_51_teams2.cpk".to_owned(),
            "aesthetics_patch.toml".to_owned(),
        ])
    );
    let part = cpk_entries(&sandbox.root.join("output/4cc_51_teams2.cpk"));
    for team in ["u0702g1", "u0707g1", "u0714g1"] {
        assert!(part.contains_key(&kit_texture(team)), "{team}");
    }
}

// TC-CLI-05
#[test]
fn multi_cpk_mode_needs_a_valid_bins_cpk_name() {
    let sandbox = sandbox_with("multicpk_refused", &EXPORTS[..1]);

    let settings = multicpk_settings(&sandbox, "bins_cpk_name = \"con\"\n");
    let run = compile(&sandbox, &settings);
    run.assert_refused(2, &["bins_cpk_name = \"con\""]);

    assert!(!sandbox.root.join("output").exists(), "nothing is written");
}

/// Every CPK a multi-CPK run writes at the default settings, in promotion order: the bins CPK,
/// then the teams slots.
fn run_cpks() -> Vec<String> {
    std::iter::once("4cc_08_bins.cpk")
        .chain(TEAMS_SLOTS)
        .map(str::to_owned)
        .collect()
}

/// A sandbox holding /a/ and /co/, `name`, whose game folder is `install`ed.
fn deploying_sandbox(name: &str, install: impl Fn(&Sandbox)) -> Sandbox {
    let sandbox = sandbox_with(name, &[EXPORTS[0], EXPORTS[2]]);
    install(&sandbox);
    sandbox
}

/// A deploying `compile` in `sandbox` with `multicpk_mode` on.
fn compile_deploying(sandbox: &Sandbox) -> Run {
    sandbox.run(&multicpk_settings(sandbox, ""), &["compile"])
}

/// The names of the files and folders in the sandbox's `download/` that a deployment makes
/// and must not leave: `.partial` and `.old`.
fn leftovers(sandbox: &Sandbox) -> Vec<String> {
    fs::read_dir(sandbox.root.join("PES/download"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".partial") || name.ends_with(".old"))
        .collect()
}

/// Asserts the bins CPK at `bins` holds the three bins, and the teams part at `part` the
/// kits of /a/ and /co/.
fn assert_run_s_cpks(bins: &Path, part: &Path) {
    let mut expected_bins = vec![
        TEAM_COLOR.to_owned(),
        UNI_COLOR.to_owned(),
        UNIFORM_PARAMETER.to_owned(),
    ];
    expected_bins.sort();
    assert_eq!(
        cpk_entries(bins).into_keys().collect::<Vec<_>>(),
        expected_bins
    );
    let part = cpk_entries(part);
    for team in ["u0702g1", "u0714g1"] {
        assert!(part.contains_key(&kit_texture(team)), "{team}");
    }
}

/// Asserts every CPK of the run is in the sandbox's output folder, the refs CPK holding the
/// referee of slot 01 last among them when the run held a refs export (`refs`), and nothing
/// else but the `aesthetics_patch.toml` and a `teamnotes.txt` when the run wrote one.
fn assert_every_cpk_promoted(sandbox: &Sandbox, refs: bool) {
    let mut files = output_files(sandbox);
    files.remove("teamnotes.txt");
    files.remove("aesthetics_patch.toml");
    let mut expected: BTreeSet<String> = run_cpks().into_iter().collect();
    if refs {
        expected.insert(REFS_CPK.to_owned());
    }
    assert_eq!(files, expected);
    let output = sandbox.root.join("output");
    assert_run_s_cpks(
        &output.join("4cc_08_bins.cpk"),
        &output.join("4cc_41_teams.cpk"),
    );
    if refs {
        let referee = "Asset/model/character/face/real/referee001/#Win/face.fpk";
        assert!(cpk_entries(&output.join(REFS_CPK)).contains_key(referee));
        let part = cpk_entries(&output.join("4cc_41_teams.cpk"));
        assert!(!part.contains_key(referee), "the referees are no team");
    }
}

// TC-DEP-01
#[test]
fn a_deploying_multi_cpk_compile_installs_every_cpk_and_leaves_nothing_in_the_output_folder() {
    let sandbox = deploying_sandbox("multicpk_deploys", install_pes);

    let run = compile_deploying(&sandbox);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let download = sandbox.root.join("PES/download");
    assert_run_s_cpks(
        &download.join("4cc_08_bins.cpk"),
        &download.join("4cc_41_teams.cpk"),
    );
    for slot in &TEAMS_SLOTS[1..] {
        assert!(
            fs::read(download.join(slot)).unwrap() == placeholder(),
            "{slot} is the placeholder"
        );
    }
    assert_eq!(leftovers(&sandbox), Vec::<String>::new());
    let cpks_in_output: Vec<String> = output_files(&sandbox)
        .into_iter()
        .filter(|name| name.ends_with(".cpk"))
        .collect();
    assert_eq!(cpks_in_output, Vec::<String>::new());
    assert!(!sandbox.root.join("output/.staging").exists());
}

// TC-DEP-08
#[test]
fn a_list_without_the_teams_slots_is_one_dpfilelist_outdated_and_every_cpk_is_promoted() {
    let sandbox = deploying_sandbox("multicpk_outdated", |sandbox| {
        sandbox.write("PES/PES2021.exe", b"the game");
        sandbox.write("PES/download/DpFileList.bin", &pes17_list());
    });
    let download = snapshot(&sandbox.root.join("PES/download"));

    let run = compile_deploying(&sandbox);

    // The PES 2017 list has the bins CPK, and none of the teams slots.
    let expected = format!(
        "Error dpfilelist_outdated [Keep] (cpk={}, path={}, output={}, command={} \
         team-compiler upgrade-dpfl)",
        TEAMS_SLOTS.join(", "),
        sandbox.display("PES/download/DpFileList.bin"),
        sandbox.display("output"),
        studio_core::EXE_NAME
    );
    let lines = run.messages();
    assert_eq!(lines[0], expected, "{lines:#?}");
    assert!(
        lines[1..].iter().all(|line| !line.starts_with("Error ")),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
    assert_every_cpk_promoted(&sandbox, false);
    assert_eq!(snapshot(&sandbox.root.join("PES/download")), download);
}

#[cfg(windows)]
// TC-DEP-11
#[test]
fn a_locked_teams_part_is_old_cpk_locked_and_no_cpk_of_the_run_is_installed() {
    // Windows only: Linux renames over a file another process holds open.
    use std::os::windows::fs::OpenOptionsExt;
    /// `FILE_SHARE_READ` alone, as PES holds its CPKs: read, not deleted or replaced.
    const FILE_SHARE_READ: u32 = 1;

    let sandbox = deploying_sandbox("multicpk_locked", install_pes);
    crate::referees::write_ref_a(&sandbox, &["01"]);
    let download = snapshot(&sandbox.root.join("PES/download"));
    let locked = sandbox.root.join("PES/download/4cc_42_teams.cpk");
    let held_open = fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&locked)
        .unwrap();

    let run = compile_deploying(&sandbox);

    drop(held_open);
    let lines = run.messages();
    // The error ends with the platform's own text, so only its shape is fixed.
    let prefix = format!(
        "Error old_cpk_locked [Keep] (path={}, error=",
        sandbox.display("PES/download/4cc_42_teams.cpk")
    );
    let suffix = format!(", output={})", sandbox.display("output"));
    let locked_lines: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains("old_cpk_locked"))
        .collect();
    assert_eq!(locked_lines.len(), 1, "{lines:#?}");
    assert!(
        locked_lines[0].starts_with(&prefix) && locked_lines[0].ends_with(&suffix),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
    assert_every_cpk_promoted(&sandbox, true);
    assert_eq!(snapshot(&sandbox.root.join("PES/download")), download);
    assert_eq!(leftovers(&sandbox), Vec::<String>::new());
}

// TC-DEP-11
#[test]
fn a_failing_copy_of_one_cpk_is_deploy_target_unwritable_and_installs_none() {
    let sandbox = deploying_sandbox("multicpk_copy_failed", install_pes);
    // A folder where the copy of the fourth CPK would go: the copy alone fails, the probe made
    // before any export is read passes, since `4cc_43_teams.cpk` itself is writable.
    let blocking = sandbox.root.join("PES/download/4cc_43_teams.cpk.partial");
    fs::create_dir_all(&blocking).unwrap();
    let download = snapshot(&sandbox.root.join("PES/download"));

    let run = compile_deploying(&sandbox);

    let lines = run.messages();
    let prefix = format!(
        "Error deploy_target_unwritable [Keep] (path={}, error=",
        sandbox.display("PES/download")
    );
    let suffix = format!(", output={})", sandbox.display("output"));
    let unwritable: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains("deploy_target_unwritable"))
        .collect();
    assert_eq!(unwritable.len(), 1, "{lines:#?}");
    assert!(
        unwritable[0].starts_with(&prefix) && unwritable[0].ends_with(&suffix),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
    assert_every_cpk_promoted(&sandbox, false);
    assert_eq!(snapshot(&sandbox.root.join("PES/download")), download);
    assert_eq!(leftovers(&sandbox), ["4cc_43_teams.cpk.partial"]);
    assert!(blocking.is_dir(), "the folder that was there stays");
}

// TC-OUT-07
#[test]
fn test_mode_writes_its_loose_files_as_without_multi_cpk_mode() {
    let mut trees = Vec::new();
    for (name, multicpk) in [
        ("multicpk_test_mode_on", true),
        ("multicpk_test_mode_off", false),
    ] {
        let sandbox = Sandbox::new(name);
        let settings = format!(
            "{}[team-compiler]\nmulticpk_mode = {multicpk}\n",
            pes21_settings(&sandbox)
        );
        let run = sandbox.run(
            &settings,
            &["compile", "--mode", "test", "--export", &tracer_export()],
        );
        assert_eq!(run.exit_code(), 0, "{name}");
        assert_eq!(
            output_files(&sandbox),
            BTreeSet::from(["test_output".to_owned(), "aesthetics_patch.toml".to_owned()]),
            "{name}"
        );
        trees.push(snapshot(&sandbox.root.join("output/test_output")));
    }
    assert!(trees[0] == trees[1], "the same loose files");
}
