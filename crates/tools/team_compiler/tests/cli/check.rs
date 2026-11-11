//! `check`: the structure pass's findings for folder exports, and the exit code they give.

use studio_core::{ExportId, PipelineEvent, RunId};

use crate::common::Sandbox;
use crate::compile::tracer_kit;
use crate::{CLEAN_PLAYER, clean_model, snapshot};

// TC-CLI-11
#[test]
fn check_brackets_each_export_with_its_start_and_end() {
    let sandbox = Sandbox::new("envelopes");
    sandbox.write(
        &format!("exports/co Midcup Spring/{CLEAN_PLAYER}"),
        &clean_model(),
    );
    let run = sandbox.run("", &["check"]);
    assert_eq!(run.exit_code(), 0);
    let export = Some(ExportId(0));
    let kinds: Vec<(Option<ExportId>, &str)> = run
        .events
        .iter()
        .map(|envelope| {
            assert_eq!(envelope.run_id, RunId(1));
            assert_eq!(envelope.export_revision, None);
            let kind = match &envelope.event {
                PipelineEvent::ExportStarted {
                    export_id,
                    display_name,
                } => {
                    assert_eq!(
                        (*export_id, display_name.as_str()),
                        (ExportId(0), "co Midcup Spring")
                    );
                    "started"
                }
                PipelineEvent::Message(_) => "message",
                PipelineEvent::ExportProcessed { export_id } => {
                    assert_eq!(*export_id, ExportId(0));
                    "processed"
                }
                PipelineEvent::ExportNamed { .. }
                | PipelineEvent::FolderStatus { .. }
                | PipelineEvent::Output(_)
                | PipelineEvent::Progress { .. }
                | PipelineEvent::Complete { .. }
                | PipelineEvent::UnmigratedContentFound => "other",
            };
            (envelope.export_id, kind)
        })
        .collect();
    assert_eq!(
        kinds,
        [
            (export, "started"),
            (export, "message"),
            (export, "processed")
        ]
    );
}

// TC-SRC-05
#[test]
fn two_refs_exports_are_both_skipped_and_a_disabled_one_is_left_out() {
    let sandbox = Sandbox::new("multiple_refs");
    for name in ["refs a", "refs b"] {
        sandbox.write(&format!("exports/{name}/players.txt"), b"01 Keeper\n");
        sandbox.write(
            &format!("exports/{name}/Players/Keeper/face_high.fmdl"),
            &clean_model(),
        );
        // Validated, this would be root_file_unexpected: its absence shows neither is.
        sandbox.write(&format!("exports/{name}/extra.bin"), b"");
    }
    sandbox.write("exports/refs c/NO_USE", b"");
    sandbox.write("exports/refs c/players.txt", b"01 Keeper\n");

    let run = sandbox.run("", &["check"]);

    assert_eq!(
        run.messages(),
        [
            "refs a: Error multiple_ref_exports [DropExport] ()",
            "refs b: Error multiple_ref_exports [DropExport] ()",
            "refs c: Info export_disabled [DropExport] ()",
        ]
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-STR-09
#[test]
fn a_disallowed_file_is_an_error_when_strict_and_an_info_when_lenient() {
    let sandbox = Sandbox::new("strict_file_type_check");
    // A texture the deep pass passes, so the lenient run's findings are the disallowed file's.
    sandbox.write(
        "exports/co Midcup Spring/Players/03 - A/hair.dds",
        &tracer_kit(),
    );
    sandbox.write("exports/co Midcup Spring/Players/03 - A/readme.doc", b"");

    let strict = sandbox.run(
        "[team-compiler]\nstrict_file_type_check = true\n",
        &["check"],
    );
    assert_eq!(
        strict.messages(),
        [
            "co Midcup Spring: Error file_type_disallowed [DropFolder] at Players/03 - A (file=readme.doc)",
            "co Midcup Spring: Info export_identified [Keep] (team=/co/, id=714)",
        ]
    );
    assert_eq!(strict.exit_code(), 1);

    let lenient = sandbox.run(
        "[team-compiler]\nstrict_file_type_check = false\n",
        &["check"],
    );
    assert_eq!(
        lenient.messages(),
        [
            "co Midcup Spring: Info file_type_disallowed [Keep] at Players/03 - A (file=readme.doc)",
            "co Midcup Spring: Info export_identified [Keep] (team=/co/, id=714)",
        ]
    );
    assert_eq!(lenient.exit_code(), 0);
}

// TC-STR-16
#[test]
fn pass_through_keeps_a_disallowed_file_at_its_error_severity() {
    let sandbox = Sandbox::new("pass_through");
    sandbox.write("exports/co Midcup Spring/Players/03 - A/readme.doc", b"");
    let run = sandbox.run("[team-compiler]\npass_through = true\n", &["check"]);
    assert_eq!(
        run.messages(),
        [
            "co Midcup Spring: Error file_type_disallowed [Keep] at Players/03 - A (file=readme.doc)",
            "co Midcup Spring: Info export_identified [Keep] (team=/co/, id=714)",
        ]
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-CLI-01
#[test]
fn check_writes_nothing() {
    let sandbox = Sandbox::new("writes_nothing");
    let settings = "[team-compiler]\ncpk_name = \"cup\"\n";
    sandbox.write("data/settings.toml", settings.as_bytes());
    sandbox.write(
        &format!("exports/co Midcup Spring/{CLEAN_PLAYER}"),
        &clean_model(),
    );
    sandbox.write("exports/co Midcup Spring/notes.txt", b"a note");
    sandbox.write(
        "exports/zz Midcup Autumn/Players/03 - B/readme.doc",
        b"text",
    );
    let before = snapshot(&sandbox.root);

    let run = sandbox.run(settings, &["check"]);

    assert_eq!(run.exit_code(), 1);
    assert!(!sandbox.root.join("output").exists());
    // The settings file, the teams list and every export file, listing and bytes.
    assert_eq!(snapshot(&sandbox.root), before);
}

// TC-CLI-02
#[test]
fn check_exits_one_only_for_an_error_finding() {
    let sandbox = Sandbox::new("exit_codes");
    sandbox.write(
        &format!("exports/co Midcup Notes/{CLEAN_PLAYER}"),
        &clean_model(),
    );
    sandbox.write("exports/co Midcup Notes/notes.txt", b"a note");
    sandbox.write("exports/co Midcup Notes/extra.bin", b"");
    sandbox.write("exports/co Midcup Error/Players/03 - A/readme.doc", b"");
    sandbox.write(
        &format!("exports/co Midcup Clean/{CLEAN_PLAYER}"),
        &clean_model(),
    );

    let run = sandbox.run(
        "",
        &["check", "--export", &sandbox.arg("exports/co Midcup Notes")],
    );
    assert_eq!(
        run.messages(),
        [
            "co Midcup Notes: Warning root_file_unexpected [DropFile] at extra.bin ()",
            "co Midcup Notes: Info notes_found [Keep] at notes.txt ()",
            "co Midcup Notes: Info export_identified [Keep] (team=/co/, id=714)",
        ]
    );
    assert_eq!(run.exit_code(), 0);

    let run = sandbox.run(
        "",
        &["check", "--export", &sandbox.arg("exports/co Midcup Error")],
    );
    assert_eq!(
        run.messages(),
        [
            "co Midcup Error: Error file_type_disallowed [DropFolder] at Players/03 - A (file=readme.doc)",
            "co Midcup Error: Info export_identified [Keep] (team=/co/, id=714)",
        ]
    );
    assert_eq!(run.exit_code(), 1);

    let run = sandbox.run(
        "",
        &["check", "--export", &sandbox.arg("exports/co Midcup Clean")],
    );
    assert_eq!(
        run.messages(),
        ["co Midcup Clean: Info export_identified [Keep] (team=/co/, id=714)"]
    );
    assert_eq!(run.exit_code(), 0);
}

// TC-SRC-03
#[test]
fn a_disabled_export_reports_only_that_whatever_the_marker_case() {
    let sandbox = Sandbox::new("disabled");
    for (name, marker) in [("co Midcup One", "NO_USE.txt"), ("co Midcup Two", "no_use")] {
        sandbox.write(&format!("exports/{name}/{marker}"), b"");
        sandbox.write(&format!("exports/{name}/extra.bin"), b"");
    }
    // A marker below the source's own root is the nested root's content, not a marker.
    sandbox.write("exports/co Midcup Three/wrapper/NO_USE", b"");
    sandbox.write(
        &format!("exports/co Midcup Three/wrapper/{CLEAN_PLAYER}"),
        &clean_model(),
    );

    let run = sandbox.run("", &["check"]);

    assert_eq!(
        run.messages(),
        [
            "co Midcup One: Info export_disabled [DropExport] ()",
            "co Midcup Three: Warning nested_folders_fixed [Keep] (folder=wrapper)",
            "co Midcup Three: Warning root_file_unexpected [DropFile] at NO_USE ()",
            "co Midcup Three: Info export_identified [Keep] (team=/co/, id=714)",
            "co Midcup Two: Info export_disabled [DropExport] ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
}

// TC-ID-03
#[test]
fn an_export_is_identified_by_its_first_word() {
    let sandbox = Sandbox::new("identified");
    sandbox.write(
        &format!("exports/co Midcup Spring 2026/{CLEAN_PLAYER}"),
        &clean_model(),
    );
    // A refs export needs no teams-list row.
    sandbox.write("exports/refs Cup/players.txt", b"01 Keeper\n");
    sandbox.write(
        "exports/refs Cup/Players/Keeper/face_high.fmdl",
        &clean_model(),
    );
    let run = sandbox.run("", &["check"]);
    assert_eq!(
        run.messages(),
        [
            "co Midcup Spring 2026: Info export_identified [Keep] (team=/co/, id=714)",
            "refs Cup: Info export_identified [Keep] (team=referees)",
        ]
    );
    assert_eq!(run.exit_code(), 0);
}

// TC-ID-04, TC-ID-02
#[test]
fn an_unknown_team_is_an_error_and_the_export_beside_it_is_still_checked() {
    let sandbox = Sandbox::new("unknown_team");
    sandbox.write(
        &format!("exports/zz Midcup Spring/{CLEAN_PLAYER}"),
        &clean_model(),
    );
    sandbox.write(&format!("exports/---/{CLEAN_PLAYER}"), &clean_model());
    sandbox.write(
        &format!("exports/co Midcup Spring/{CLEAN_PLAYER}"),
        &clean_model(),
    );
    let run = sandbox.run("", &["check"]);
    assert_eq!(
        run.messages(),
        [
            "---: Error team_name_unknown [DropExport] (team_name=)",
            "co Midcup Spring: Info export_identified [Keep] (team=/co/, id=714)",
            "zz Midcup Spring: Error team_name_unknown [DropExport] (team_name=/zz/)",
        ]
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-CLI-09
#[test]
fn check_creates_the_missing_default_exports_folder_and_reports_it_empty() {
    let sandbox = Sandbox::new("missing_root");
    let exports = sandbox.root.join("exports");
    let run = sandbox.run("", &["check"]);
    assert!(exports.is_dir(), "the folder was created");
    assert_eq!(
        run.messages(),
        [format!(
            "Warning no_exports_found [Keep] (folder={})",
            exports.display()
        )]
    );
    assert_eq!(run.exit_code(), 0);
}

// TC-SRC-08
#[test]
fn export_paths_restrict_the_run_to_the_named_sources() {
    let sandbox = Sandbox::new("named_sources");
    for name in ["co Midcup A", "co Midcup B", "co Midcup C"] {
        sandbox.write(&format!("exports/{name}/{CLEAN_PLAYER}"), &clean_model());
    }
    sandbox.write(
        &format!("elsewhere/co Midcup D/{CLEAN_PLAYER}"),
        &clean_model(),
    );
    let run = sandbox.run(
        "",
        &[
            "check",
            "--export",
            &sandbox.arg("elsewhere/co Midcup D"),
            "--export",
            &sandbox.arg("exports/co Midcup B"),
        ],
    );
    // The two named exports of one team conflict; `co Midcup A` and `co Midcup C`, not named, do not.
    let duplicate =
        "Error duplicate_aesthetics_export [DropExport] (id=714, exports=co Midcup B, co Midcup D)";
    assert_eq!(
        run.messages(),
        [
            "co Midcup B: Info export_identified [Keep] (team=/co/, id=714)".to_owned(),
            format!("co Midcup B: {duplicate}"),
            "co Midcup D: Info export_identified [Keep] (team=/co/, id=714)".to_owned(),
            format!("co Midcup D: {duplicate}"),
        ]
    );
    assert_eq!(run.exit_code(), 1);
}
