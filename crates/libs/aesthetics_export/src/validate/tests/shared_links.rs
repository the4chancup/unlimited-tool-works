use super::*;

// TC-STR-05
#[test]
fn a_shared_link_resolves_with_or_without_the_txt_tail() {
    for link_file in ["Crocs.boots", "Crocs.boots.txt"] {
        let path = format!("Players/03 - A/{link_file}");
        let report = report(
            "egg Midcup",
            &[
                ("Players/03 - A/hair.dds", 9),
                (path.as_str(), 0),
                ("Boots/Crocs/boots.fmdl", 10),
            ],
            &[],
            &[],
        );
        assert_eq!(issue_codes(&report), vec![], "{link_file}");
        let player = &report.validated.unwrap().players[0];
        assert_eq!(
            player.links,
            vec![SharedLink {
                kind: SharedKind::Boots,
                name: "Crocs".to_owned(),
            }],
            "{link_file}"
        );
    }
}

// TC-STR-06
#[test]
fn a_link_to_a_missing_shared_folder_drops_the_player() {
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/hair.dds", 9),
            ("Players/03 - A/Nowhere.boots", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("link_target_missing", Disposition::DropFolder)]
    );
    assert_eq!(report.issues[0].scope, folder("Players/03 - A"));
    assert_eq!(
        report.issues[0].context,
        vec![("link", "Nowhere.boots".to_owned())]
    );
    assert!(report.validated.unwrap().players.is_empty());
}

// TC-STR-07
#[test]
fn two_links_of_one_kind_drop_the_player() {
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/hair.dds", 9),
            ("Players/03 - A/Crocs.boots", 0),
            ("Players/03 - A/Mud.boots", 0),
        ],
        &[],
        &[],
    );
    // Every finding is reported: the duplicate, and each missing target.
    assert_eq!(
        issue_codes(&report),
        vec![
            ("shared_link_duplicate", Disposition::DropFolder),
            ("link_target_missing", Disposition::DropFolder),
            ("link_target_missing", Disposition::DropFolder),
        ]
    );
    assert_eq!(report.issues[0].context, vec![("kind", "boots".to_owned())]);
    assert!(report.validated.unwrap().players.is_empty());
}

// TC-STR-08
#[test]
fn a_shared_folder_no_surviving_player_links_is_orphaned() {
    let report = report(
        "egg Midcup",
        &[
            ("players.txt", 5),
            ("Boots/Solo/boots.fmdl", 10),
            ("Boots/Duo/boots.fmdl", 10),
            ("Players/Droppable/Duo.boots", 0),
        ],
        &["Players/A", "Players/Droppable"],
        &[("players.txt", Ok(b"03 A"))],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("player_unlisted", Disposition::DropFolder),
            ("shared_folder_orphaned", Disposition::DropFolder),
            ("shared_folder_orphaned", Disposition::DropFolder),
        ]
    );
    assert_eq!(report.issues[0].scope, folder("Players/Droppable"));
    assert_eq!(report.issues[1].scope, folder("Boots/Duo"));
    assert_eq!(report.issues[2].scope, folder("Boots/Solo"));
    let validated = report.validated.unwrap();
    assert_eq!(validated.players.len(), 1);
    assert!(validated.boots.is_empty());
}

// TC-STR-16
#[test]
fn a_dropped_link_target_drops_its_players_and_pass_through_keeps() {
    let files = &[
        ("Faces/Base/x.exe", 4u64),
        ("Players/03 - A/Base.face", 0),
        ("Players/03 - A/hair.dds", 9),
    ];
    let strict = report("egg Midcup", files, &[], &[]);
    assert_eq!(
        issue_codes(&strict),
        vec![
            ("file_type_disallowed", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    assert_eq!(strict.issues[0].scope, folder("Faces/Base"));
    assert_eq!(strict.issues[1].scope, folder("Players/03 - A"));
    assert_eq!(
        strict.issues[1].context,
        link_target("Base.face", "Faces/Base", "file_type_disallowed")
    );
    let validated = strict.validated.unwrap();
    assert!(validated.players.is_empty());
    assert!(validated.faces.is_empty());

    // With pass_through the target stays, no link_target_dropped.
    let kept = report_with(&context_with(true, true), "egg Midcup", files, &[], &[]);
    assert_eq!(
        issue_codes(&kept),
        vec![("file_type_disallowed", Disposition::Keep)]
    );
    assert!(kept.issues[0].passed_through);
    let validated = kept.validated.unwrap();
    assert_eq!(validated.players.len(), 1);
    assert_eq!(validated.faces.len(), 1);
    assert_eq!(
        validated.players[0].links,
        vec![SharedLink {
            kind: SharedKind::Face,
            name: "Base".to_owned(),
        }]
    );

    // A conflict is never pass-through-eligible: both drop.
    let conflicted = report_with(
        &context_with(true, true),
        "egg Midcup",
        &[
            ("Faces/Base/hair.dds", 9),
            ("Faces/Base/hair.png", 9),
            ("Players/03 - A/Base.face", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&conflicted),
        vec![
            ("texture_stem_conflict", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    assert!(conflicted.issues.iter().all(|issue| !issue.passed_through));
    let validated = conflicted.validated.unwrap();
    assert!(validated.players.is_empty());
    assert!(validated.faces.is_empty());
}

#[test]
fn pass_through_keeps_a_dangling_link_player_minus_the_link() {
    let report = report_with(
        &context_with(true, true),
        "egg Midcup",
        &[
            ("Players/03 - A/Nowhere.boots", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("link_target_missing", Disposition::Keep)]
    );
    assert!(report.issues[0].passed_through);
    let player = &report.validated.unwrap().players[0];
    // The player is kept; the missing reference is not.
    assert!(player.links.is_empty());
}

// TC-MOD-70
#[test]
fn a_subfolder_s_shared_link_is_the_root_s_and_one_folder_linked_twice_counts_once() {
    let files = |subfolder_link: &str| {
        [
            ("Boots/Crocs/boots.fmdl".to_owned(), 10),
            ("Boots/Tabi/boots.fmdl".to_owned(), 10),
            ("Players/03 - A/face_high.fmdl".to_owned(), 10),
            ("Players/03 - A/Crocs.boots".to_owned(), 0),
            (format!("Players/03 - A/jessie/{subfolder_link}"), 0),
        ]
    };
    fn borrowed(files: &[(String, u64)]) -> Vec<(&str, u64)> {
        files
            .iter()
            .map(|(path, size)| (path.as_str(), *size))
            .collect()
    }

    let once = files("Crocs.boots");
    let compiled = report("egg Midcup", &borrowed(&once), &[], &[]);
    // Tabi, which no one links, is orphaned; Crocs is linked, once.
    assert_eq!(
        issue_codes(&compiled),
        vec![("shared_folder_orphaned", Disposition::DropFolder)]
    );
    assert_eq!(compiled.issues[0].scope, folder("Boots/Tabi"));
    let validated = compiled.validated.unwrap();
    assert_eq!(
        validated.players[0].links,
        vec![SharedLink {
            kind: SharedKind::Boots,
            name: "Crocs".to_owned(),
        }]
    );

    let twice = files("Tabi.boots");
    let checked = report("egg Midcup", &borrowed(&twice), &[], &[]);
    assert_eq!(
        issue_codes(&checked),
        vec![
            ("shared_link_duplicate", Disposition::DropFolder),
            ("shared_folder_orphaned", Disposition::DropFolder),
            ("shared_folder_orphaned", Disposition::DropFolder),
        ]
    );
    assert_eq!(checked.issues[0].scope, folder("Players/03 - A"));
    assert_eq!(
        checked.issues[0].context,
        vec![("kind", "boots".to_owned())]
    );
    assert!(checked.validated.unwrap().players.is_empty());
}

#[test]
fn a_link_file_below_any_subfolder_is_a_link_as_in_the_root() {
    // Both links resolve: the shared folder is linked, not orphaned, and the `.common` link
    // names the `Common/` file at its own path.
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/extra/Crocs.boots", 0),
            ("Players/03 - A/face/torso.fmdl.common", 0),
            ("Boots/Crocs/boots.fmdl", 10),
            ("Common/face/torso.fmdl", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
    let player = &report.validated.unwrap().players[0];
    assert_eq!(
        player.links,
        vec![SharedLink {
            kind: SharedKind::Boots,
            name: "Crocs".to_owned(),
        }]
    );
    assert!(
        player
            .files
            .iter()
            .all(|file| file.path.as_str() != "Players/03 - A/extra/Crocs.boots"),
        "a shared link is no pipeline file"
    );
}

#[test]
fn a_dropped_target_names_its_own_first_finding() {
    let report = report(
        "egg Midcup",
        &[
            ("Players/01 - P/fpc_on", 0),
            ("Players/01 - P/fpc_off", 0),
            ("Players/01 - P/hair.dds", 9),
            ("Players/02 - Q/Base.face", 0),
            ("Players/02 - Q/hair.dds", 9),
            ("Faces/Base/notes.doc", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("fpc_conflict", Disposition::DropFolder),
            ("file_type_disallowed", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    assert_eq!(
        report.issues[2].context,
        link_target("Base.face", "Faces/Base", "file_type_disallowed")
    );
}

#[test]
fn a_player_dropped_by_its_own_finding_reports_no_dropped_target() {
    let report = report(
        "egg Midcup",
        &[
            ("players.txt", 5),
            ("Players/A/fpc_on", 0),
            ("Players/A/fpc_off", 0),
            ("Players/A/Base.face", 0),
            ("Players/A/hair.dds", 9),
            ("Faces/Base/x.exe", 4),
        ],
        &[],
        &[("players.txt", Ok(b"03 A"))],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("fpc_conflict", Disposition::DropFolder),
            ("file_type_disallowed", Disposition::DropFolder),
        ]
    );
}
