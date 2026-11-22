//! `compile --mode sideload` (`spec/team_compiler/pipeline.md` "5. Writer", step 5): what a normal
//! run puts in its CPK, written as loose files that replace the game folder's `livecpk/`.

use std::collections::BTreeMap;
use std::path::Path;

use crate::bins::install_names;
use crate::common::Sandbox;
use crate::compile::{cpk_entries, pes_settings, pes21_settings};
use crate::compile_exports::TEAM_COLOR;
use crate::deploy::tracer_copy;
use crate::referees::{REFS, REFS_CPK, write_ref_a};
use crate::{deploy_skipped, patch_written, savefile_missing, snapshot};

/// What the sandboxes' `overrides/` folder puts at `TEAM_COLOR`.
const TEAM_COLOR_OVERRIDE: &[u8] = b"the operator's TeamColor.bin";

/// What an earlier sideload run left in `livecpk/`.
const OLD: &[u8] = b"an earlier sideload";

/// Where `tracer_copy` puts the tracer in a sandbox.
const TRACER: &str = "exports/egg Midcup Tracer";

/// A sandbox holding the tracer's completed copy, whose PES folder holds a
/// `livecpk/old.txt`, with an override of `TEAM_COLOR` in the data folder.
fn with_livecpk(name: &str) -> Sandbox {
    let sandbox = Sandbox::new(name);
    tracer_copy(&sandbox);
    sandbox.write("PES/livecpk/old.txt", OLD);
    sandbox.write(&format!("data/overrides/{TEAM_COLOR}"), TEAM_COLOR_OVERRIDE);
    sandbox
}

/// Every file under the sandbox's `PES/livecpk/`, by its path there spelled with `/` as a CPK
/// entry's is, with its bytes.
fn livecpk(sandbox: &Sandbox) -> BTreeMap<String, Vec<u8>> {
    snapshot(&sandbox.root.join("PES/livecpk"))
        .into_iter()
        .map(|(path, bytes)| (slashed(&path), bytes))
        .collect()
}

/// `path` spelled with `/` between its parts, as a CPK entry's path is.
pub(crate) fn slashed(path: &Path) -> String {
    let parts: Vec<&str> = path
        .components()
        .map(|part| part.as_os_str().to_str().unwrap())
        .collect();
    parts.join("/")
}

/// `compile --mode sideload --export <the tracer>` in `sandbox` with `settings`, naming the
/// copy `tracer_copy` made.
fn sideload(sandbox: &Sandbox, settings: &str) -> crate::common::Run {
    sandbox.run(
        settings,
        &[
            "compile",
            "--mode",
            "sideload",
            "--export",
            &sandbox.arg(TRACER),
        ],
    )
}

// TC-OUT-09
#[test]
fn sideload_writes_what_the_cpk_holds_as_loose_files_replacing_livecpk() {
    let normal = with_livecpk("sideload_normal_twin");
    install_names(&normal, &["4cc_99_test.cpk"]);
    write_ref_a(&normal, &["01"]);
    normal.complete(REFS);
    let normal_run = normal.run(
        &pes21_settings(&normal),
        &[
            "compile",
            "--no-deploy",
            "--export",
            &normal.arg(TRACER),
            "--export",
            &normal.arg(REFS),
        ],
    );
    assert_eq!(normal_run.exit_code(), 0, "{:#?}", normal_run.messages());
    let mut expected = cpk_entries(&normal.root.join("output/4cc_99_test.cpk"));
    let refs = cpk_entries(&normal.root.join(format!("output/{REFS_CPK}")));
    let team_count = expected.len();
    expected.extend(refs.clone());
    assert_eq!(
        expected.len(),
        team_count + refs.len(),
        "no path in both CPKs"
    );

    let sandbox = with_livecpk("sideload_tracer");
    install_names(&sandbox, &["4cc_99_test.cpk"]);
    write_ref_a(&sandbox, &["01"]);
    sandbox.complete(REFS);
    let download = snapshot(&sandbox.root.join("PES/download"));

    let run = sandbox.run(
        &pes21_settings(&sandbox),
        &[
            "compile",
            "--mode",
            "sideload",
            "--export",
            &sandbox.arg(TRACER),
            "--export",
            &sandbox.arg(REFS),
        ],
    );

    assert_eq!(run.exit_code(), 0);
    let written = livecpk(&sandbox);
    // `old.txt` is not among the CPKs' entries, so it is gone.
    assert_eq!(
        written.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>()
    );
    let first_difference = expected
        .iter()
        .find(|(path, bytes)| written.get(*path) != Some(bytes))
        .map(|(path, _)| path);
    assert_eq!(first_difference, None, "the first entry whose bytes differ");
    assert_eq!(written[TEAM_COLOR], TEAM_COLOR_OVERRIDE);
    // The same findings as the normal run's, `overrides_active` and `duplicate_path` among them.
    let normal_root = normal.root.display().to_string();
    let root = sandbox.root.display().to_string();
    // The twin says last that it installed neither CPK (`--no-deploy`), the team CPK first,
    // then that its patch was written; a sideload run has no such CPK notes but writes the
    // patch alike.
    let mut normal_messages = normal_run.messages();
    let refs_skipped = format!(
        "Info deploy_skipped_by_flag [Keep] (path={})",
        normal.display(&format!("output/{REFS_CPK}"))
    );
    assert_eq!(normal_messages.pop(), Some(patch_written(&normal, "792")));
    assert_eq!(normal_messages.pop(), Some(refs_skipped));
    assert_eq!(normal_messages.pop(), Some(deploy_skipped(&normal)));
    let normal_lines: Vec<String> = normal_messages
        .iter()
        .map(|line| line.replace(&normal_root, &root))
        .collect();
    let mut sideload_messages = run.messages();
    // The sideload tree is what the game loads, so its savefile step runs, and finds no
    // save in the sandbox.
    assert_eq!(sideload_messages.pop(), Some(savefile_missing(&sandbox)));
    assert_eq!(
        sideload_messages.pop(),
        Some(patch_written(&sandbox, "792"))
    );
    assert_eq!(sideload_messages, normal_lines);
    let overrides = sandbox.display("data/overrides");
    for line in [
        format!("Info overrides_active [Keep] (folder={overrides}, files=1)"),
        format!("Warning duplicate_path [Keep] (path={TEAM_COLOR})"),
    ] {
        assert!(run.messages().contains(&line), "{line}");
    }
    assert_eq!(snapshot(&sandbox.root.join("PES/download")), download);
    assert!(!sandbox.root.join("output/4cc_99_test.cpk").exists());
    assert!(!sandbox.root.join(format!("output/{REFS_CPK}")).exists());
    assert!(!sandbox.root.join("output/.staging").exists());
}

// TC-OUT-10
#[test]
fn sideload_is_refused_for_pes_2015_and_2016_and_writes_livecpk_for_pes_2017() {
    for version in [15, 16] {
        let sandbox = with_livecpk(&format!("sideload_pes{version}"));
        let before = snapshot(&sandbox.root);

        let run = sideload(&sandbox, &pes_settings(&sandbox, version));

        run.assert_refused(2, &["--mode sideload", &format!("PES 20{version}")]);
        assert_eq!(snapshot(&sandbox.root), before, "PES {version}");
    }

    let sandbox = with_livecpk("sideload_pes17");
    let run = sideload(&sandbox, &pes_settings(&sandbox, 17));

    // The tracer's Fox models are converted for PES 2017's face: the export compiles, and
    // its player's face CPK is written with the rest.
    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let written = livecpk(&sandbox);
    assert_eq!(written[TEAM_COLOR], TEAM_COLOR_OVERRIDE);
    assert!(!written.contains_key("old.txt"));
    assert!(
        written.contains_key("common/character0/model/character/face/real/79205.cpk"),
        "{:?}",
        written.keys()
    );
}

// TC-OUT-10
#[test]
fn sideload_without_a_pes_folder_is_refused_naming_it() {
    let sandbox = Sandbox::new("sideload_no_pes_folder");
    tracer_copy(&sandbox);
    // `**` is the version's two digits: the refusal names the folder looked for.
    let settings = format!(
        "[common]\npes_version = 21\npes_folder_path = '{}'\n",
        sandbox.root.join("PES**").display()
    );
    let before = snapshot(&sandbox.root);

    let run = sideload(&sandbox, &settings);

    run.assert_refused(2, &["--mode sideload", &sandbox.display("PES21")]);
    assert_eq!(snapshot(&sandbox.root), before);
}

// TC-OUT-03
#[test]
fn a_sideload_run_that_fails_while_writing_leaves_livecpk_as_it_was() {
    let sandbox = with_livecpk("sideload_write_failed");
    install_names(&sandbox, &["4cc_99_test.cpk"]);
    // A file where the run's staging folder goes: the loose tree cannot be written in it.
    sandbox.write("output/.staging", b"in the way");
    let before = snapshot(&sandbox.root.join("PES"));

    let run = sideload(&sandbox, &pes21_settings(&sandbox));

    let lines = run.messages();
    let last = lines.last().unwrap();
    let prefix = format!(
        "Fatal cpk_write_failed [AbortRun] (path={}, error=",
        sandbox.display("PES/livecpk")
    );
    assert!(last.starts_with(&prefix), "{last}");
    assert_eq!(run.exit_code(), 3);
    assert_eq!(snapshot(&sandbox.root.join("PES")), before);
}

// TC-OUT-18
#[test]
fn a_sideload_run_without_a_dpfilelist_only_warns_that_it_is_missing() {
    let sandbox = Sandbox::new("sideload_no_dpfilelist");
    tracer_copy(&sandbox);
    sandbox.write("PES/PES2021.exe", b"the game");

    let run = sideload(&sandbox, &pes21_settings(&sandbox));

    let missing = format!(
        "Warning dpfilelist_missing [Keep] (path={})",
        sandbox.display("PES/download/DpFileList.bin")
    );
    assert_eq!(run.messages().first(), Some(&missing));
    assert_eq!(run.exit_code(), 0);
    assert!(
        livecpk(&sandbox).contains_key(TEAM_COLOR),
        "the tree is written"
    );
}
