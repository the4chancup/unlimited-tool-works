use super::*;

/// A `vertex_too_far_from_origin` content finding on `scope` (the consumer's code, not one of
/// `ISSUE_CODES`), never pass-through-eligible.
fn far_vertex(scope: IssueScope, disposition: Disposition) -> ContentFinding {
    ContentFinding {
        code: "vertex_too_far_from_origin",
        scope,
        context: vec![("file", "boots.fmdl".to_owned()), ("count", "1".to_owned())],
        disposition,
        pass_through_eligible: false,
    }
}

/// `far_vertex`'s issue, as validation reports it when nothing keeps the item.
fn far_vertex_issue(scope: IssueScope, disposition: Disposition) -> ValidationIssue {
    ValidationIssue {
        code: "vertex_too_far_from_origin",
        scope,
        context: vec![("file", "boots.fmdl".to_owned()), ("count", "1".to_owned())],
        disposition,
        passed_through: false,
    }
}

/// `report`'s export with `findings` added, with the default context.
fn with_findings(files: &[(&str, u64)], findings: Vec<ContentFinding>) -> ValidationReport {
    report("egg Midcup", files, &[], &[])
        .with_content_findings(findings, &crate::testing::context())
}

/// The paths of the validated players.
fn player_paths(validated: &ValidatedAestheticsExport) -> Vec<&str> {
    validated
        .players
        .iter()
        .map(|player| player.path.as_str())
        .collect()
}

/// The paths of `files`.
fn file_paths(files: &[FileDescriptor]) -> Vec<&str> {
    files.iter().map(|file| file.path.as_str()).collect()
}

/// The `link_target_dropped` issue on the linker in the folder at `path` when
/// `target` falls to `finding`.
fn dropped_link(path: &str, link: &str, target: &str, finding: &str) -> ValidationIssue {
    ValidationIssue {
        code: "link_target_dropped",
        scope: folder(path),
        context: link_target(link, target, finding),
        disposition: Disposition::DropFolder,
        passed_through: false,
    }
}

#[test]
fn no_content_findings_leave_the_report_as_validate_made_it() {
    // A parse issue (the nested root), structure issues and a cascade: none is doubled.
    let files = &[
        ("wrapper/Players/03 - A/boots.fmdl", 10),
        ("wrapper/Players/03 - A/readme.doc", 1),
        ("wrapper/Players/05 - B/Crocs.boots", 0),
        ("wrapper/Boots/Crocs/x.exe", 1),
        ("wrapper/Common/hair.dds", 9),
        ("wrapper/Kits/p1/kit.dds", 9),
    ];
    let first = report("egg Midcup", files, &[], &[]);
    assert_eq!(
        issue_codes(&first),
        vec![
            ("nested_folders_fixed", Disposition::Keep),
            ("file_type_disallowed", Disposition::DropFolder),
            ("file_type_disallowed", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    let again = first
        .clone()
        .with_content_findings(Vec::new(), &crate::testing::context());
    assert_eq!(again, first);
}

#[test]
fn a_content_finding_dropping_a_player_folder_takes_it_off_players_and_the_roster() {
    let report = with_findings(
        &[
            ("Players/03 - A/boots.fmdl", 10),
            ("Players/05 - B/boots.fmdl", 10),
        ],
        vec![far_vertex(
            folder("Players/03 - A"),
            Disposition::DropFolder,
        )],
    );
    assert_eq!(
        report.issues,
        vec![far_vertex_issue(
            folder("Players/03 - A"),
            Disposition::DropFolder
        )]
    );
    let validated = report.validated.unwrap();
    assert_eq!(player_paths(&validated), vec!["Players/05 - B"]);
    let ValidatedRoster::Team(map) = &validated.roster else {
        panic!("a team export");
    };
    assert_eq!(
        map.iter()
            .map(|(slot, index)| (slot.get(), index.0))
            .collect::<Vec<_>>(),
        vec![(5, 0)]
    );
}

#[test]
fn a_content_finding_dropping_a_shared_folder_cascades_to_its_linkers_and_orphans() {
    let report = with_findings(
        &[
            ("Boots/Crocs/boots.fmdl", 10),
            ("Gloves/Grip/glove_l.fmdl", 10),
            ("Players/03 - A/Crocs.boots", 0),
            ("Players/03 - A/Grip.gloves", 0),
            ("Players/05 - B/boots.fmdl", 10),
        ],
        vec![far_vertex(folder("Boots/Crocs"), Disposition::DropFolder)],
    );
    assert_eq!(
        report.issues,
        vec![
            far_vertex_issue(folder("Boots/Crocs"), Disposition::DropFolder),
            dropped_link(
                "Players/03 - A",
                "Crocs.boots",
                "Boots/Crocs",
                "vertex_too_far_from_origin"
            ),
            ValidationIssue {
                code: "shared_folder_orphaned",
                scope: folder("Gloves/Grip"),
                context: vec![],
                disposition: Disposition::DropFolder,
                passed_through: false,
            },
        ]
    );
    let validated = report.validated.unwrap();
    assert_eq!(player_paths(&validated), vec!["Players/05 - B"]);
    assert!(validated.boots.is_empty());
    assert!(validated.gloves.is_empty());
}

#[test]
fn a_content_finding_dropping_a_common_file_drops_the_player_linking_it() {
    let report = with_findings(
        &[
            ("Common/legs.fmdl", 10),
            ("Common/hair.dds", 9),
            ("Players/03 - A/legs.fmdl.common", 0),
            ("Players/05 - B/boots.fmdl", 10),
        ],
        vec![far_vertex(
            file_scope("Common/legs.fmdl"),
            Disposition::DropFile,
        )],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("vertex_too_far_from_origin", Disposition::DropFile),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    assert_eq!(report.issues[1].scope, folder("Players/03 - A"));
    assert_eq!(
        report.issues[1].context,
        vec![
            ("link", "legs.fmdl.common".to_owned()),
            ("target", "Common/legs.fmdl".to_owned()),
            ("finding", "vertex_too_far_from_origin".to_owned()),
        ]
    );
    let validated = report.validated.unwrap();
    assert_eq!(player_paths(&validated), vec!["Players/05 - B"]);
    assert_eq!(file_paths(&validated.common), vec!["Common/hair.dds"]);
}

#[test]
fn a_content_finding_dropping_a_common_subfolder_drops_every_file_below_it() {
    // The subfolder's two files leave `common`, the root's stays, and a player linking one
    // of them falls as one linking a dropped file does.
    let report = with_findings(
        &[
            ("Common/u0XXXp0/a.dds", 9),
            ("Common/u0XXXp0/b.dds", 9),
            ("Common/u0XXXp0x.dds", 9),
            ("Common/hair.dds", 9),
            ("Players/03 - A/u0XXXp0/a.dds.common", 0),
            ("Players/05 - B/boots.fmdl", 10),
        ],
        vec![far_vertex(
            folder("Common/u0XXXp0"),
            Disposition::DropFolder,
        )],
    );
    assert_eq!(
        report.issues,
        vec![
            far_vertex_issue(folder("Common/u0XXXp0"), Disposition::DropFolder),
            dropped_link(
                "Players/03 - A",
                "a.dds.common",
                "Common/u0XXXp0/a.dds",
                "vertex_too_far_from_origin"
            ),
        ]
    );
    let validated = report.validated.unwrap();
    assert_eq!(player_paths(&validated), vec!["Players/05 - B"]);
    // A sibling whose name merely starts with the folder's is no file below it.
    assert_eq!(
        file_paths(&validated.common),
        vec!["Common/hair.dds", "Common/u0XXXp0x.dds"]
    );
}

#[test]
fn a_common_kit_set_link_falls_only_with_its_last_member() {
    // A `.common` link naming a kit set reaches every member: a dropped member
    // is an absent one, a gap the set's completion fills, so one of several
    // dropped leaves the linker standing.
    let files = &[
        ("Common/kit1/armor_bsm.dds", 9),
        ("Common/kit2/armor_bsm.dds", 9),
        ("Players/03 - A/kitN/armor_bsm.dds.common", 0),
        ("Players/05 - B/boots.fmdl", 10),
    ];
    let report = with_findings(
        files,
        vec![far_vertex(
            file_scope("Common/kit1/armor_bsm.dds"),
            Disposition::DropFile,
        )],
    );
    assert_eq!(
        report.issues,
        vec![far_vertex_issue(
            file_scope("Common/kit1/armor_bsm.dds"),
            Disposition::DropFile
        )]
    );
    let validated = report.validated.unwrap();
    assert_eq!(
        player_paths(&validated),
        vec!["Players/03 - A", "Players/05 - B"]
    );
    assert_eq!(
        file_paths(&validated.common),
        vec!["Common/kit2/armor_bsm.dds"]
    );

    // Every member dropped: the link's folder falls, `link_target_dropped`
    // naming the set's representative (the lowest variant first).
    let report = with_findings(
        files,
        vec![
            far_vertex(
                file_scope("Common/kit1/armor_bsm.dds"),
                Disposition::DropFile,
            ),
            far_vertex(
                file_scope("Common/kit2/armor_bsm.dds"),
                Disposition::DropFile,
            ),
        ],
    );
    assert_eq!(
        report.issues,
        vec![
            far_vertex_issue(
                file_scope("Common/kit1/armor_bsm.dds"),
                Disposition::DropFile
            ),
            far_vertex_issue(
                file_scope("Common/kit2/armor_bsm.dds"),
                Disposition::DropFile
            ),
            dropped_link(
                "Players/03 - A",
                "armor_bsm.dds.common",
                "Common/kit1/armor_bsm.dds",
                "vertex_too_far_from_origin"
            ),
        ]
    );
    assert_eq!(
        player_paths(&report.validated.unwrap()),
        vec!["Players/05 - B"]
    );
}

#[test]
fn a_content_finding_dropping_a_common_file_cascades_through_a_shared_folder() {
    // The dropped `Common/` file takes the shared folder's `.common` link down with it,
    // and the players linking that folder follow in the same pass.
    let files = &[
        ("Common/skin.dds", 9),
        ("Boots/Crocs/boots.fmdl", 10),
        ("Boots/Crocs/skin.dds.common", 0),
        ("Players/03 - A/Crocs.boots", 0),
        ("Players/03 - A/hair.dds", 9),
    ];
    let dropped = with_findings(
        files,
        vec![far_vertex(
            file_scope("Common/skin.dds"),
            Disposition::DropFile,
        )],
    );
    assert_eq!(
        dropped.issues,
        vec![
            far_vertex_issue(file_scope("Common/skin.dds"), Disposition::DropFile),
            dropped_link(
                "Boots/Crocs",
                "skin.dds.common",
                "Common/skin.dds",
                "vertex_too_far_from_origin"
            ),
            dropped_link(
                "Players/03 - A",
                "Crocs.boots",
                "Boots/Crocs",
                "link_target_dropped"
            ),
        ]
    );
    let validated = dropped.validated.unwrap();
    assert!(player_paths(&validated).is_empty());
    assert!(validated.boots.is_empty());

    // An eligible finding kept under `pass_through` keeps its dependants too: neither
    // `link_target_dropped`, and the folder and its linker stand.
    let context = context_with(true, true);
    let kept = report("egg Midcup", files, &[], &[]).with_content_findings(
        vec![ContentFinding {
            pass_through_eligible: true,
            ..far_vertex(file_scope("Common/skin.dds"), Disposition::DropFile)
        }],
        &context,
    );
    assert_eq!(
        kept.issues,
        vec![ValidationIssue {
            passed_through: true,
            ..far_vertex_issue(file_scope("Common/skin.dds"), Disposition::Keep)
        }]
    );
    let validated = kept.validated.unwrap();
    assert_eq!(player_paths(&validated), vec!["Players/03 - A"]);
    assert_eq!(validated.boots.len(), 1);
}

#[test]
fn a_common_kit_set_link_leaves_a_shared_folder_standing_for_one_member_dropped() {
    let report = with_findings(
        &[
            ("Common/kit1/armor_bsm.dds", 9),
            ("Common/kit2/armor_bsm.dds", 9),
            ("Boots/Crocs/boots.fmdl", 10),
            ("Boots/Crocs/kitN/armor_bsm.dds.common", 0),
            ("Players/03 - A/Crocs.boots", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        vec![far_vertex(
            file_scope("Common/kit1/armor_bsm.dds"),
            Disposition::DropFile,
        )],
    );
    assert_eq!(
        report.issues,
        vec![far_vertex_issue(
            file_scope("Common/kit1/armor_bsm.dds"),
            Disposition::DropFile
        )]
    );
    let validated = report.validated.unwrap();
    assert_eq!(player_paths(&validated), vec!["Players/03 - A"]);
    assert_eq!(validated.boots.len(), 1);
    assert_eq!(
        file_paths(&validated.common),
        vec!["Common/kit2/armor_bsm.dds"]
    );
}

#[test]
fn a_content_finding_dropping_a_model_folder_s_file_takes_only_that_file_off() {
    let report = with_findings(
        &[
            ("Players/03 - A/oral.fmdl", 10),
            ("Players/03 - A/face_high.fmdl", 10),
            ("Players/03 - A/Crocs.boots", 0),
            ("Boots/Crocs/boots.fmdl", 10),
            ("Boots/Crocs/boots.dds", 9),
        ],
        vec![
            far_vertex(
                file_scope("Players/03 - A/oral.fmdl"),
                Disposition::DropFile,
            ),
            far_vertex(file_scope("Boots/Crocs/boots.fmdl"), Disposition::DropFile),
        ],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("vertex_too_far_from_origin", Disposition::DropFile),
            ("vertex_too_far_from_origin", Disposition::DropFile),
        ]
    );
    let validated = report.validated.unwrap();
    assert_eq!(player_paths(&validated), vec!["Players/03 - A"]);
    assert_eq!(
        file_paths(&validated.players[0].files),
        vec!["Players/03 - A/face_high.fmdl"]
    );
    assert_eq!(validated.boots.len(), 1);
    assert_eq!(
        file_paths(&validated.boots[0].files),
        vec!["Boots/Crocs/boots.dds"]
    );
}

#[test]
fn a_content_finding_dropping_a_common_file_and_the_link_naming_it_keeps_the_player() {
    let report = with_findings(
        &[
            ("Common/legs.fmdl", 10),
            ("Players/03 - A/legs.fmdl.common", 0),
            ("Players/03 - A/face_high.fmdl", 10),
        ],
        vec![
            far_vertex(
                file_scope("Players/03 - A/legs.fmdl.common"),
                Disposition::DropFile,
            ),
            far_vertex(file_scope("Common/legs.fmdl"), Disposition::DropFile),
        ],
    );
    // A dropped link is no link: no `link_target_dropped`.
    assert_eq!(
        issue_codes(&report),
        vec![
            ("vertex_too_far_from_origin", Disposition::DropFile),
            ("vertex_too_far_from_origin", Disposition::DropFile),
        ]
    );
    let validated = report.validated.unwrap();
    assert_eq!(player_paths(&validated), vec!["Players/03 - A"]);
    assert_eq!(
        file_paths(&validated.players[0].files),
        vec!["Players/03 - A/face_high.fmdl"]
    );
    assert!(validated.common.is_empty());
}

#[test]
fn a_content_finding_dropping_a_shared_link_leaves_no_link_and_orphans_its_folder() {
    let report = with_findings(
        &[
            ("Players/05 - A/Crocs.boots", 0),
            ("Players/05 - A/face_high.fmdl", 10),
            ("Boots/Crocs/boots.fmdl", 10),
        ],
        vec![far_vertex(
            file_scope("Players/05 - A/Crocs.boots"),
            Disposition::DropFile,
        )],
    );
    // A dropped link is no link: the folder it named is linked by no one.
    assert_eq!(
        issue_codes(&report),
        vec![
            ("vertex_too_far_from_origin", Disposition::DropFile),
            ("shared_folder_orphaned", Disposition::DropFolder),
        ]
    );
    let validated = report.validated.unwrap();
    assert_eq!(player_paths(&validated), vec!["Players/05 - A"]);
    assert!(validated.players[0].links.is_empty());
    assert_eq!(
        file_paths(&validated.players[0].files),
        vec!["Players/05 - A/face_high.fmdl"]
    );
    assert!(validated.boots.is_empty());
}

#[test]
fn pass_through_keeps_an_eligible_content_finding_s_folder_and_drops_a_not_eligible_one() {
    let context = context_with(true, true);
    let eligible = ContentFinding {
        pass_through_eligible: true,
        ..far_vertex(folder("Players/03 - A"), Disposition::DropFolder)
    };
    let report = report_with(
        &context,
        "egg Midcup",
        &[
            ("Players/03 - A/boots.fmdl", 10),
            ("Players/05 - B/boots.fmdl", 10),
        ],
        &[],
        &[],
    )
    .with_content_findings(
        vec![
            eligible,
            far_vertex(folder("Players/05 - B"), Disposition::DropFolder),
        ],
        &context,
    );
    assert_eq!(
        report.issues,
        vec![
            ValidationIssue {
                passed_through: true,
                ..far_vertex_issue(folder("Players/03 - A"), Disposition::Keep)
            },
            far_vertex_issue(folder("Players/05 - B"), Disposition::DropFolder),
        ]
    );
    assert_eq!(
        player_paths(&report.validated.unwrap()),
        vec!["Players/03 - A"]
    );
}

#[test]
fn a_content_finding_dropping_a_kit_folder_takes_the_kit_off() {
    let report = with_findings(
        &[("Kits/p1/kit.dds", 9), ("Kits/p2 - Away/kit.dds", 9)],
        vec![far_vertex(folder("Kits/p1"), Disposition::DropFolder)],
    );
    let validated = report.validated.unwrap();
    assert_eq!(
        validated
            .kits
            .kits
            .values()
            .map(|kit| kit.path.as_str())
            .collect::<Vec<_>>(),
        vec!["Kits/p2 - Away"]
    );
}

#[test]
fn a_content_finding_dropping_a_portrait_or_collar_file_takes_only_that_file_off() {
    let report = with_findings(
        &[
            ("Players/03 - A/boots.fmdl", 10),
            ("Portraits/03.dds", 9),
            ("Portraits/05.dds", 9),
            ("Collars/collar_101.fmdl", 10),
            ("Collars/collar_102.fmdl", 10),
        ],
        vec![
            far_vertex(file_scope("Portraits/03.dds"), Disposition::DropFile),
            far_vertex(file_scope("Collars/collar_101.fmdl"), Disposition::DropFile),
        ],
    );
    let validated = report.validated.unwrap();
    assert_eq!(
        validated
            .portraits
            .iter()
            .map(|(slot, file)| (slot.get(), file.path.as_str()))
            .collect::<Vec<_>>(),
        vec![(5, "Portraits/05.dds")]
    );
    assert_eq!(
        file_paths(&validated.collars),
        vec!["Collars/collar_102.fmdl"]
    );
}

#[test]
fn a_content_finding_dropping_a_player_s_settings_or_portrait_keeps_the_folder_without_it() {
    let files = &[
        ("Players/03 - A/boots.fmdl", 10),
        ("Players/03 - A/portrait.png", 9),
        ("Players/03 - A/settings.toml", 20),
    ];
    for (dropped, settings, portrait) in [
        ("settings.toml", None, Some("Players/03 - A/portrait.png")),
        ("portrait.png", Some("Players/03 - A/settings.toml"), None),
    ] {
        let without = with_findings(
            files,
            vec![far_vertex(
                file_scope(&format!("Players/03 - A/{dropped}")),
                Disposition::DropFile,
            )],
        );
        let player = &without.validated.unwrap().players[0];
        assert_eq!(
            player.settings.as_ref().map(|file| file.path.as_str()),
            settings,
            "{dropped}"
        );
        assert_eq!(
            player.portrait.as_ref().map(|file| file.path.as_str()),
            portrait,
            "{dropped}"
        );
        assert_eq!(
            file_paths(&player.files),
            vec!["Players/03 - A/boots.fmdl"],
            "{dropped}"
        );
    }
}

#[test]
fn a_content_finding_dropping_a_colors_file_takes_only_that_file_off() {
    let files = &[
        ("Players/03 - A/boots.fmdl", 10),
        ("Kits/p1/kit.dds", 9),
        ("Kits/p1/colors.txt", 16),
        ("colors.txt", 16),
    ];
    let without_kit_colors = with_findings(
        files,
        vec![far_vertex(
            file_scope("Kits/p1/colors.txt"),
            Disposition::DropFile,
        )],
    )
    .validated
    .unwrap();
    let kit = &without_kit_colors.kits.kits[&kit_config::KitSlot::P1];
    assert_eq!(kit.colors, None);
    assert_eq!(
        kit.textures
            .iter()
            .map(|texture| texture.file.path.as_str())
            .collect::<Vec<_>>(),
        vec!["Kits/p1/kit.dds"]
    );
    assert_eq!(
        without_kit_colors
            .root
            .team_colors
            .as_ref()
            .map(|file| file.path.as_str()),
        Some("colors.txt")
    );

    let without_team_colors = with_findings(
        files,
        vec![far_vertex(file_scope("colors.txt"), Disposition::DropFile)],
    )
    .validated
    .unwrap();
    assert_eq!(without_team_colors.root.team_colors, None);
    assert_eq!(
        without_team_colors.kits.kits[&kit_config::KitSlot::P1]
            .colors
            .as_ref()
            .map(|file| file.path.as_str()),
        Some("Kits/p1/colors.txt")
    );
}

#[test]
fn a_content_finding_dropping_either_logo_file_drops_the_logo() {
    let files = &[
        ("Players/03 - A/boots.fmdl", 10),
        ("logo.png", 9),
        ("logo_small.png", 9),
    ];
    assert!(
        report("egg Midcup", files, &[], &[])
            .validated
            .unwrap()
            .logo
            .is_some()
    );
    for name in ["logo.png", "logo_small.png"] {
        let report = with_findings(
            files,
            vec![far_vertex(file_scope(name), Disposition::DropFile)],
        );
        assert_eq!(report.validated.unwrap().logo, None, "{name}");
    }
}

#[test]
fn a_content_finding_dropping_the_export_leaves_no_sanitized_export() {
    let report = with_findings(
        &[("Players/03 - A/boots.fmdl", 10)],
        vec![far_vertex(IssueScope::Export, Disposition::DropExport)],
    );
    assert!(report.validated.is_none());
}

#[test]
fn content_findings_stand_after_the_folders_own_findings_and_before_the_cascade() {
    let report = with_findings(
        &[
            ("Boots/Crocs/boots.fmdl", 10),
            ("Players/03 - A/Crocs.boots", 0),
            ("Players/07 - C/readme.doc", 1),
            ("Kits/p1/kit.dds", 9),
            ("Kits/p1/icon_99", 0),
        ],
        vec![far_vertex(folder("Boots/Crocs"), Disposition::DropFolder)],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("file_type_disallowed", Disposition::DropFolder),
            ("vertex_too_far_from_origin", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
            ("kit_icon_invalid", Disposition::DropFile),
        ]
    );
}
