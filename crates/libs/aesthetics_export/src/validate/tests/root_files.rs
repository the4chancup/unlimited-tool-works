use super::*;

// TC-ROOT-01
#[test]
fn unexpected_root_files_and_folders_are_dropped() {
    let base = report(
        "egg Midcup",
        &[
            ("extra.bin", 4),
            ("readme.TXT", 20),
            ("Players/players.txt", 10),
            ("Players/03 - A/hair.dds", 9),
        ],
        &["wrapper"],
        &[],
    );
    assert_eq!(
        issue_codes(&base),
        vec![
            ("root_file_unexpected", Disposition::DropFile),
            ("root_file_unexpected", Disposition::DropFolder),
            ("root_file_unexpected", Disposition::DropFile),
        ]
    );
    assert_eq!(base.issues[0].scope, file_scope("extra.bin"));
    assert_eq!(base.issues[1].scope, folder("wrapper"));
    assert_eq!(base.issues[2].scope, file_scope("Players/players.txt"));

    // A referee export's ref_lists.txt is admitted.
    let refs = report(
        "refs Cup",
        &[("players.txt", 10), ("ref_lists.txt", 10)],
        &["Players/Keeper"],
        &[
            ("players.txt", Ok(b"01 Keeper")),
            ("ref_lists.txt", Ok(&b"R1"[..])),
        ],
    );
    assert!(
        refs.issues
            .iter()
            .all(|issue| issue.code != "root_file_unexpected")
    );
}

#[test]
fn a_write_names_marker_at_the_root_is_a_team_exports_flag() {
    let team = report(
        "egg Midcup",
        &[("write_names", 0), ("Players/03 - A/hair.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&team), vec![]);
    assert!(team.validated.unwrap().root.write_names);

    // A referee export does not admit it.
    let refs = report(
        "refs Cup",
        &[("players.txt", 10), ("write_names", 0)],
        &["Players/Keeper"],
        &[("players.txt", Ok(b"01 Keeper"))],
    );
    assert_eq!(
        issue_codes(&refs),
        vec![("root_file_unexpected", Disposition::DropFile)]
    );
    assert_eq!(refs.issues[0].scope, file_scope("write_names"));
    assert!(!refs.validated.unwrap().root.write_names);

    // Absent, the flag is off.
    let without = report("egg Midcup", &[("Players/03 - A/hair.dds", 9)], &[], &[]);
    assert_eq!(issue_codes(&without), vec![]);
    assert!(!without.validated.unwrap().root.write_names);
}

// TC-ROOT-03
#[test]
fn logo_files_get_one_of_each_role() {
    let invalid = report("egg Midcup", &[("logo_zoom.png", 9)], &[], &[]);
    assert_eq!(
        issue_codes(&invalid),
        vec![("logo_file_invalid", Disposition::DropFile)]
    );
    assert_eq!(invalid.validated.unwrap().logo, None);

    let duplicate = report("egg Midcup", &[("logo.png", 9), ("logo.dds", 9)], &[], &[]);
    assert_eq!(
        issue_codes(&duplicate),
        vec![("logo_role_duplicate", Disposition::DropFile)]
    );
    assert_eq!(duplicate.issues[0].scope, file_scope("logo.png"));
    assert_eq!(
        duplicate.issues[0].context,
        vec![("other", "logo.dds".to_owned())]
    );
    assert_eq!(duplicate.validated.unwrap().logo, None);

    let small_only = report("egg Midcup", &[("logo_small.png", 9)], &[], &[]);
    assert_eq!(
        issue_codes(&small_only),
        vec![("logo_small_without_main", Disposition::DropFile)]
    );
    assert_eq!(small_only.validated.unwrap().logo, None);

    let pair = report(
        "egg Midcup",
        &[("logo.png", 9), ("logo_small.png", 9)],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&pair), vec![]);
    let logo = pair.validated.unwrap().logo.unwrap();
    assert_eq!(logo.main.file.path.as_str(), "logo.png");
    assert_eq!(logo.small.unwrap().file.path.as_str(), "logo_small.png");
}

/// The slots the validated export's portraits claim, in map order.
fn portrait_slots(report: &ValidationReport) -> Vec<u8> {
    report
        .validated
        .as_ref()
        .unwrap()
        .portraits
        .keys()
        .map(|slot| slot.get())
        .collect()
}

// TC-ROOT-04
#[test]
fn a_bad_portrait_name_drops_only_that_file() {
    let report = report(
        "egg Midcup",
        &[("Portraits/24.dds", 9), ("Portraits/03.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("portrait_name_invalid", Disposition::DropFile)]
    );
    assert_eq!(report.issues[0].scope, file_scope("Portraits/24.dds"));
    assert_eq!(portrait_slots(&report), vec![3]);
}

#[test]
fn notes_are_read_or_dropped() {
    // A non-empty note is kept and reported.
    let note = report(
        "egg Midcup",
        &[("notes.txt", 12), ("Players/03 - A/hair.dds", 9)],
        &[],
        &[("notes.txt", Ok(b"good cup"))],
    );
    assert_eq!(issue_codes(&note), vec![("notes_found", Disposition::Keep)]);
    assert_eq!(
        note.validated.unwrap().root.notes.unwrap().path.as_str(),
        "notes.txt"
    );

    // Not UTF-8: the note is dropped, the export still valid.
    let bad = report(
        "egg Midcup",
        &[("notes.txt", 4), ("Players/03 - A/hair.dds", 9)],
        &[],
        &[("notes.txt", Ok(&b"\xff\xfe"[..]))],
    );
    assert_eq!(
        issue_codes(&bad),
        vec![("notes_encoding_invalid", Disposition::DropFile)]
    );
    let validated = bad.validated.unwrap();
    assert_eq!(validated.root.notes, None);
    assert_eq!(validated.players.len(), 1);

    // Unreadable: source_read_failed, still validated.
    let denied = report(
        "egg Midcup",
        &[("notes.txt", 5), ("Players/03 - A/hair.dds", 9)],
        &[],
        &[("notes.txt", Err("denied"))],
    );
    assert_eq!(
        issue_codes(&denied),
        vec![("source_read_failed", Disposition::DropFile)]
    );
    assert_eq!(
        denied.issues[0].context,
        vec![("reason", "denied".to_owned())]
    );
    assert!(denied.validated.is_some());

    // Whitespace only: nothing.
    let empty = report(
        "egg Midcup",
        &[("notes.txt", 3), ("Players/03 - A/hair.dds", 9)],
        &[],
        &[("notes.txt", Ok(b"  \n"))],
    );
    assert_eq!(issue_codes(&empty), vec![]);
    assert_eq!(empty.validated.unwrap().root.notes, None);
}

#[test]
fn two_portraits_of_one_slot_conflict_each() {
    for (files, slot) in [
        (&["Portraits/03.dds", "Portraits/03.png"][..], "03"),
        (&["Portraits/05 - Bob.png", "Portraits/05.dds"][..], "05"),
    ] {
        let report = report(
            "egg Midcup",
            &files
                .iter()
                .map(|file| (*file, 9))
                .collect::<Vec<(&str, u64)>>(),
            &[],
            &[],
        );
        assert_eq!(
            issue_codes(&report),
            vec![
                ("texture_stem_conflict", Disposition::DropFile),
                ("texture_stem_conflict", Disposition::DropFile),
            ],
            "{files:?}"
        );
        assert_eq!(report.issues[0].scope, file_scope(files[0]), "{files:?}");
        assert_eq!(report.issues[1].scope, file_scope(files[1]), "{files:?}");
        for issue in &report.issues {
            assert_eq!(issue.context[0], ("stem", slot.to_owned()), "{files:?}");
        }
        assert!(report.validated.unwrap().portraits.is_empty());
    }
}

#[test]
fn a_portrait_is_named_by_its_slot_as_a_player_folder_is() {
    for name in [
        "Portraits/05.dds",
        "Portraits/05 - Bob.png",
        "Portraits/05-Bob.png",
    ] {
        let report = report("egg Midcup", &[(name, 9)], &[], &[]);
        assert_eq!(issue_codes(&report), vec![], "{name}");
        assert_eq!(portrait_slots(&report), vec![5], "{name}");
    }
}

#[test]
fn a_portrait_name_the_slot_rule_refuses_is_invalid() {
    for name in [
        "Portraits/player_05.dds",
        "Portraits/24.dds",
        "Portraits/081.dds",
        "Portraits/ab.dds",
        "Portraits/sub/05.dds",
    ] {
        let report = report("egg Midcup", &[(name, 9)], &[], &[]);
        assert_eq!(
            issue_codes(&report),
            vec![("portrait_name_invalid", Disposition::DropFile)],
            "{name}"
        );
        assert_eq!(report.issues[0].scope, file_scope(name), "{name}");
        assert!(report.validated.unwrap().portraits.is_empty(), "{name}");
    }
}

#[test]
fn a_logo_pair_reads_its_fit_tags() {
    let report = report(
        "egg Midcup",
        &[("logo_crop.png", 9), ("logo_small_fit.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
    let logo = report.validated.unwrap().logo.unwrap();
    assert_eq!(logo.main.fit, Some(LogoFit::Crop));
    assert_eq!(logo.small.unwrap().fit, Some(LogoFit::Fit));
}

#[test]
fn a_fit_tag_is_read_in_any_case() {
    // `_Crop`, `_STRETCH` and `_Fit` are the tags spelled another way.
    for (name, fit) in [
        ("logo_Crop.png", LogoFit::Crop),
        ("logo_STRETCH.png", LogoFit::Stretch),
        ("logo_Fit.png", LogoFit::Fit),
    ] {
        let report = report("egg Midcup", &[(name, 9)], &[], &[]);
        assert_eq!(issue_codes(&report), vec![], "{name}");
        assert_eq!(
            report.validated.unwrap().logo.unwrap().main.fit,
            Some(fit),
            "{name}"
        );
    }
}

#[test]
fn a_portrait_number_past_two_digits_reads_its_slot() {
    // The folder rule reads the head's whole number: `003` is slot 03 as a
    // player folder `003` is, `081` (no slot 81) is refused.
    let report = report("egg Midcup", &[("Portraits/003.dds", 9)], &[], &[]);
    assert_eq!(issue_codes(&report), vec![]);
    assert_eq!(portrait_slots(&report), vec![3]);
}

#[test]
fn a_logo_candidate_that_is_no_texture_is_invalid() {
    let report = report(
        "egg Midcup",
        &[("logo.txt", 9), ("Players/03 - A/hair.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("logo_file_invalid", Disposition::DropFile)]
    );
    assert_eq!(report.validated.unwrap().logo, None);
}

#[test]
fn a_root_colors_file_is_the_team_colors() {
    let report = report(
        "egg Midcup",
        &[("colors.txt", 10), ("Players/03 - A/hair.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
    assert_eq!(
        report
            .validated
            .unwrap()
            .root
            .team_colors
            .unwrap()
            .path
            .as_str(),
        "colors.txt"
    );
}

#[test]
fn referee_files_are_unexpected_on_a_team_export() {
    let report = report(
        "egg Midcup",
        &[
            ("refs.txt", 10),
            ("ref_lists.txt", 10),
            ("ref_marker.dds", 9),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("root_file_unexpected", Disposition::DropFile),
            ("root_file_unexpected", Disposition::DropFile),
            ("root_file_unexpected", Disposition::DropFile),
        ]
    );
    assert_eq!(report.issues[0].scope, file_scope("ref_lists.txt"));
    assert_eq!(report.issues[1].scope, file_scope("ref_marker.dds"));
    assert_eq!(report.issues[2].scope, file_scope("refs.txt"));
    assert_eq!(report.validated.unwrap().root.referee_marker, None);
}

#[test]
fn a_ref_marker_is_admitted_on_a_refs_export() {
    // The name matches case-insensitively.
    for name in ["ref_marker.dds", "Ref_Marker.dds"] {
        let report = report(
            "refs Cup",
            &[("players.txt", 10), (name, 9)],
            &["Players/Keeper"],
            &[("players.txt", Ok(b"01 Keeper"))],
        );
        assert!(
            report
                .issues
                .iter()
                .all(|issue| issue.code != "root_file_unexpected"),
            "{name}"
        );
        assert_eq!(
            report
                .validated
                .unwrap()
                .root
                .referee_marker
                .unwrap()
                .path
                .as_str(),
            name
        );
    }
}
