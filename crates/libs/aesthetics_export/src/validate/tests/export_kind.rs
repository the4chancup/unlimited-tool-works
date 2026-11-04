use super::*;

// TC-STR-04
#[test]
fn an_export_with_no_usable_content_is_dropped_empty() {
    let report = report("egg Midcup", &[("readme.txt", 10)], &[], &[]);
    assert_eq!(
        issue_codes(&report),
        vec![("export_empty", Disposition::DropExport)]
    );
    assert_eq!(report.issues[0].scope, IssueScope::Export);
    assert!(report.validated.is_none());
}

#[test]
fn an_ambiguous_or_conflicting_root_reports_no_export_empty() {
    for files in [
        &[("a/Players/x/face_high.fmdl", 10), ("b/Kits/p1/kit.dds", 9)][..],
        &[
            ("notes.txt", 5),
            ("wrapper/notes.txt", 5),
            ("wrapper/Players/03 - A/face_high.fmdl", 10),
        ][..],
    ] {
        let report = report("egg Midcup", files, &[], &[]);
        assert!(
            report
                .issues
                .iter()
                .all(|issue| issue.code != "export_empty"),
            "{files:?}"
        );
    }
}

// TC-ID-04
#[test]
fn a_stem_with_no_team_token_is_dropped_unknown() {
    let report = report("--- ", &[("Players/03 - A/face_high.fmdl", 10)], &[], &[]);
    assert_eq!(
        issue_codes(&report),
        vec![("team_name_unknown", Disposition::DropExport)]
    );
    assert_eq!(report.issues[0].scope, IssueScope::Export);
    assert_eq!(report.issues[0].context, vec![("team_name", String::new())]);
    assert!(report.validated.is_none());
}

#[test]
fn a_team_export_without_its_coverage_tag_is_dropped() {
    let report = report(
        "co Spring 2026",
        &[("Players/03 - A/face_high.fmdl", 10)],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("export_tag_missing", Disposition::DropExport)]
    );
    assert_eq!(report.issues[0].scope, IssueScope::Export);
    assert_eq!(
        report.issues[0].context,
        vec![("name", "co Spring 2026".to_owned())]
    );
    assert!(report.validated.is_none());
}

#[test]
fn an_old_layout_export_is_export_layout_old_alone() {
    for kit_configs in ["Kit Configs", "kit configs"] {
        let report = report(
            "co Spring 2026",
            &[
                ("Faces/01 - A/face.model", 10),
                (&format!("{kit_configs}/co_DEF_1st_realUni.bin"), 10),
                ("Kit Textures/u0714p1.dds", 10),
                ("Portraits/01.dds", 10),
            ],
            &[],
            &[],
        );
        assert_eq!(
            issue_codes(&report),
            vec![("export_layout_old", Disposition::DropExport)],
            "{kit_configs}"
        );
        assert_eq!(report.issues[0].scope, IssueScope::Export);
        assert_eq!(
            report.issues[0].context,
            vec![("folder", kit_configs.to_owned())]
        );
        assert!(report.validated.is_none());
    }
}

#[test]
fn a_tagged_team_export_carries_its_coverage() {
    for (name, coverage) in [
        ("co Full Spring 2026", ExportCoverage::Full),
        ("co midcup day 5", ExportCoverage::Midcup),
    ] {
        let report = report(name, &[("Players/03 - A/face_high.fmdl", 10)], &[], &[]);
        assert_eq!(issue_codes(&report), vec![], "{name}");
        assert_eq!(report.validated.unwrap().coverage, coverage, "{name}");
    }
}

#[test]
fn a_referee_export_needs_no_tag_and_is_full() {
    let report = report(
        "refs Spring 2026",
        &[("players.txt", 10)],
        &["Players/Keeper"],
        &[("players.txt", Ok(b"01 Keeper"))],
    );
    assert_eq!(issue_codes(&report), vec![]);
    assert_eq!(report.validated.unwrap().coverage, ExportCoverage::Full);
}
