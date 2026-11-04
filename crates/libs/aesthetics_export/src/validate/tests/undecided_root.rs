use super::*;

#[test]
fn an_undecided_root_reports_no_logo_finding() {
    let report = report(
        "egg Midcup",
        &[
            ("logo.txt", 5),
            ("a/Players/03 - A/face_high.fmdl", 10),
            ("b/Players/03 - A/face_high.fmdl", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("nested_root_ambiguous", Disposition::DropExport)]
    );
    assert!(report.validated.is_none());
}

#[test]
fn an_undecided_root_reports_the_parse_s_findings_alone() {
    // The export's name is judged once its root is decided: a stem with no team token
    // reports no `team_name_unknown` beside `nested_root_ambiguous`.
    let report = report(
        "--- ",
        &[
            ("a/Players/03 - A/face_high.fmdl", 10),
            ("b/Players/03 - B/face_high.fmdl", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("nested_root_ambiguous", Disposition::DropExport)]
    );
    assert!(report.validated.is_none());
}

#[test]
fn an_empty_doubled_players_layer_is_dropped_with_its_layer() {
    // `Players/Players/` alone under `Players/` is the doubled layer's own shell:
    // gone with the layer, not a folder named `Players` claiming a slot.
    let report = report("egg Midcup", &[], &["Players", "Players/Players"], &[]);
    assert_eq!(
        issue_codes(&report),
        vec![
            ("nested_folders_fixed", Disposition::Keep),
            ("export_empty", Disposition::DropExport),
        ]
    );
    assert!(report.validated.is_none());
    assert!(
        report
            .parsed
            .draft
            .players
            .iter()
            .all(|folder| folder.path.name() != "Players"),
        "{:?}",
        report
            .parsed
            .draft
            .players
            .iter()
            .map(|folder| folder.path.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn an_empty_doubled_kits_layer_is_dropped_with_its_layer() {
    let report = report("egg Midcup", &[], &["Kits", "Kits/Kits"], &[]);
    assert_eq!(
        issue_codes(&report),
        vec![
            ("nested_folders_fixed", Disposition::Keep),
            ("export_empty", Disposition::DropExport),
        ]
    );
    assert!(report.validated.is_none());
    assert!(
        report
            .parsed
            .draft
            .kits
            .iter()
            .all(|folder| folder.path.name() != "Kits")
    );
}

#[test]
fn an_empty_doubled_folder_beside_siblings_is_an_invalid_folder() {
    // `Players/` holding `Players/` *and* a real player: `Players/Players/` is not
    // the doubled layer (it is not the only entry) — it is a folder named
    // `Players`, an invalid player name, like a `Kits/Kits/` beside `p1`.
    let players = report(
        "egg Midcup",
        &[("Players/03 - A/face_high.fmdl", 10)],
        &["Players/Players"],
        &[],
    );
    assert_eq!(
        issue_codes(&players),
        vec![("player_folder_number_invalid", Disposition::DropFolder)]
    );
    assert_eq!(
        players.validated.unwrap().players[0].path.as_str(),
        "Players/03 - A"
    );

    let kits = report("egg Midcup", &[("Kits/p1/kit.dds", 9)], &["Kits/Kits"], &[]);
    assert_eq!(
        issue_codes(&kits),
        vec![("kit_folder_invalid", Disposition::DropFolder)]
    );
    assert_eq!(
        kits.validated.unwrap().kits.kits.keys().collect::<Vec<_>>(),
        vec![&kit_config::KitSlot::P1]
    );
}

#[test]
fn an_undecided_root_reports_no_roster_finding() {
    // A referee export whose two usable children both hold their own roster and
    // players: the ambiguity leaves the root undecided, so no `players.txt`
    // finding stands beside it.
    let ambiguous = report(
        "refs Cup",
        &[
            ("a/refs.txt", 10),
            ("a/Players/Keeper/face_high.fmdl", 10),
            ("b/refs.txt", 10),
            ("b/Players/Other/face_high.fmdl", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&ambiguous),
        vec![("nested_root_ambiguous", Disposition::DropExport)]
    );

    // A loose `players.txt` colliding with the flattened one is a conflict; the
    // loose file's lines name no folder that exists, but no roster finding is
    // reported beside the conflict.
    let conflicted = report(
        "egg Midcup",
        &[
            ("players.txt", 6),
            ("wrapper/players.txt", 6),
            ("wrapper/Players/03 - A/face_high.fmdl", 10),
        ],
        &[],
        &[("players.txt", Ok(b"03 A"))],
    );
    assert_eq!(
        issue_codes(&conflicted),
        vec![("nested_root_conflict", Disposition::DropExport)]
    );
    // The roster file's lines are not read while the root is undecided.
    assert!(conflicted.parsed.raw_roster.is_none());

    // A `players.txt` at the undecided root that is not UTF-8: its own finding is
    // a roster finding, reported nowhere either.
    let invalid = report(
        "refs Cup",
        &[
            ("players.txt", 4),
            ("a/refs.txt", 10),
            ("a/Players/Keeper/face_high.fmdl", 10),
            ("b/refs.txt", 10),
            ("b/Players/Other/face_high.fmdl", 10),
        ],
        &[],
        &[("players.txt", Ok(&[0xff, 0xfe][..]))],
    );
    assert_eq!(
        issue_codes(&invalid),
        vec![("nested_root_ambiguous", Disposition::DropExport)]
    );
}

#[test]
fn a_flattened_wrapper_of_empty_folders_leaves_no_root_folder() {
    let report = report(
        "egg Midcup",
        &[],
        &["wrapper", "wrapper/Kits", "wrapper/Kits/p1"],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("nested_folders_fixed", Disposition::Keep)]
    );
    assert!(report.parsed.draft.root_folders.is_empty());
    assert_eq!(
        report
            .validated
            .unwrap()
            .kits
            .kits
            .keys()
            .collect::<Vec<_>>(),
        vec![&kit_config::KitSlot::P1]
    );
}
