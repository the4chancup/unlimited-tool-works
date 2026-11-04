use super::*;

#[test]
fn kit_slots_and_labels_parse() {
    let report = report(
        "egg Midcup",
        &[("Kits/p1 - Lakers/kit.dds", 9), ("Kits/g1/kit.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
    let kits = &report.validated.unwrap().kits;
    let lakers = &kits.kits[&kit_config::KitSlot::P1];
    assert_eq!(lakers.path.as_str(), "Kits/p1 - Lakers");
    assert_eq!(lakers.label.as_deref(), Some("Lakers"));
    let goalie = &kits.kits[&kit_config::KitSlot::G1];
    assert_eq!(goalie.path.as_str(), "Kits/g1");
    assert_eq!(goalie.label, None);
}

#[test]
fn a_padded_kit_folder_name_still_claims_its_slot() {
    // `Kits/ p2` is a `p2` folder padded with a space: the head trims to `p2`.
    let report = report("egg Midcup", &[("Kits/ p2/kit.dds", 9)], &[], &[]);
    assert_eq!(issue_codes(&report), vec![], "{:?}", report.issues);
    assert!(
        report
            .validated
            .unwrap()
            .kits
            .kits
            .contains_key(&kit_config::KitSlot::P2)
    );
}

// TC-KIT-02
#[test]
fn two_folders_of_one_slot_are_both_dropped() {
    let report = report(
        "egg Midcup",
        &[("Kits/p1/kit.dds", 9), ("Kits/p1 - Lakers/kit.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("kit_slot_duplicate", Disposition::DropFolder),
            ("kit_slot_duplicate", Disposition::DropFolder),
        ]
    );
    assert_eq!(report.issues[0].scope, folder("Kits/p1"));
    assert_eq!(report.issues[1].scope, folder("Kits/p1 - Lakers"));
    assert!(report.validated.unwrap().kits.kits.is_empty());
}

// TC-KIT-03
#[test]
fn a_folder_name_with_no_kit_slot_is_invalid() {
    let report = report(
        "egg Midcup",
        &[],
        &["Kits/p10", "Kits/x1", "Kits/home"],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("kit_folder_invalid", Disposition::DropFolder),
            ("kit_folder_invalid", Disposition::DropFolder),
            ("kit_folder_invalid", Disposition::DropFolder),
        ]
    );
    assert_eq!(report.issues[0].scope, folder("Kits/home"));
    assert_eq!(report.issues[1].scope, folder("Kits/p10"));
    assert_eq!(report.issues[2].scope, folder("Kits/x1"));
    assert!(report.validated.unwrap().kits.kits.is_empty());
}

#[test]
fn an_invalid_kit_head_gets_no_own_findings() {
    let report = report("egg Midcup", &[("Kits/x1/back.dds", 9)], &[], &[]);
    assert_eq!(
        issue_codes(&report),
        vec![("kit_folder_invalid", Disposition::DropFolder)]
    );
}

// TC-KIT-04
#[test]
fn a_kit_inherits_only_the_stems_it_lacks() {
    let report = report(
        "egg Midcup",
        &[
            ("Kits/all/kit_back.dds", 9),
            ("Kits/all/kit_name.dds", 9),
            ("Kits/p2/kit_name.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("kit_textures_inherited", Disposition::Keep)]
    );
    assert_eq!(report.issues[0].scope, folder("Kits/p2"));
    assert_eq!(report.issues[0].context, vec![("stems", "back".to_owned())]);
    let kit = &report.validated.unwrap().kits.kits[&kit_config::KitSlot::P2];
    assert_eq!(
        kit.textures
            .iter()
            .map(|texture| (texture.stem.as_str(), texture.source))
            .collect::<Vec<_>>(),
        vec![
            ("kit_back", KitTextureSource::Shared),
            ("kit_name", KitTextureSource::Own),
        ]
    );
}

#[test]
fn the_inherited_stem_list_is_sorted() {
    // `all/`'s files arrive in path order, where `kit_a-1.dds` sorts before
    // `kit_a.dds` (`-` before `.`); the finding lists the stems sorted.
    let report = report(
        "egg Midcup",
        &[
            ("Kits/all/kit_a.dds", 9),
            ("Kits/all/kit_a-1.dds", 9),
            ("Kits/p2/kit.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("kit_textures_inherited", Disposition::Keep)]
    );
    assert_eq!(report.issues[0].scope, folder("Kits/p2"));
    assert_eq!(
        report.issues[0].context,
        vec![("stems", "a, a-1".to_owned())]
    );
}

// TC-KIT-05
#[test]
fn all_files_and_an_unused_all_folder() {
    let ignored = report("egg Midcup", &[("Kits/all/config.toml", 10)], &[], &[]);
    assert_eq!(
        issue_codes(&ignored),
        vec![
            ("kit_all_file_ignored", Disposition::DropFile),
            ("kit_all_unused", Disposition::Keep),
        ]
    );
    assert_eq!(ignored.issues[0].scope, file_scope("Kits/all/config.toml"));

    let unused = report("egg Midcup", &[("Kits/all/kit_back.dds", 9)], &[], &[]);
    assert_eq!(
        issue_codes(&unused),
        vec![("kit_all_unused", Disposition::Keep)]
    );
    assert_eq!(unused.issues[0].scope, folder("Kits/all"));
    assert_eq!(unused.validated.unwrap().kits.shared.len(), 1);
}

#[test]
fn an_all_head_in_any_case_is_the_shared_folder() {
    // `ALL` heads the shared-textures folder like `all`.
    let report = report("egg Midcup", &[("Kits/ALL/kit.dds", 9)], &[], &[]);
    assert_eq!(
        issue_codes(&report),
        vec![("kit_all_unused", Disposition::Keep)]
    );
    assert_eq!(
        report.issues[0].scope,
        folder("Kits/ALL"),
        "{:?}",
        report.issues
    );
    assert_eq!(report.validated.unwrap().kits.shared.len(), 1);
}

// TC-KIT-06
#[test]
fn both_layout_markers_drop_the_kit() {
    let report = report(
        "egg Midcup",
        &[
            ("Kits/p1/kit.dds", 9),
            ("Kits/p1/pre-fox", 0),
            ("Kits/p1/fox", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("kit_layout_conflict", Disposition::DropFolder)]
    );
    assert!(report.validated.unwrap().kits.kits.is_empty());
}

/// The kit an empty folder at `path` gives: nothing of its own, nothing inherited.
fn empty_kit(path: &str) -> KitFolder {
    KitFolder {
        path: ScopePath::new(path).unwrap(),
        label: None,
        config: None,
        colors: None,
        icon: None,
        layout: None,
        textures: Vec::new(),
    }
}

#[test]
fn a_full_export_without_kits_gets_an_empty_p1_and_g1() {
    let report = report(
        "co Full Spring",
        &[("Players/03 - A/face_high.fmdl", 10)],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
    let kits = report.validated.unwrap().kits.kits;
    assert_eq!(
        kits,
        BTreeMap::from([
            (kit_config::KitSlot::P1, empty_kit("Kits/p1")),
            (kit_config::KitSlot::G1, empty_kit("Kits/g1")),
        ])
    );
}

#[test]
fn a_full_export_s_empty_kits_inherit_all_s_textures() {
    let report = report(
        "co Full Spring",
        &[
            ("Players/03 - A/face_high.fmdl", 10),
            ("Kits/all/kit.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("kit_textures_inherited", Disposition::Keep),
            ("kit_textures_inherited", Disposition::Keep),
        ],
        "and no kit_all_unused: the empty kits use all/"
    );
    assert_eq!(report.issues[0].scope, folder("Kits/p1"));
    assert_eq!(report.issues[1].scope, folder("Kits/g1"));
    let kits = report.validated.unwrap().kits;
    for slot in [kit_config::KitSlot::P1, kit_config::KitSlot::G1] {
        let kit = &kits.kits[&slot];
        assert_eq!(kit.textures.len(), 1, "{slot:?}");
        assert_eq!(kit.textures[0].stem, "kit");
        assert_eq!(kit.textures[0].file, kits.shared[0]);
        assert_eq!(kit.textures[0].source, KitTextureSource::Shared);
    }
}

#[test]
fn a_full_export_with_a_player_kit_and_g1_gets_no_empty_kit() {
    let report = report(
        "co Full Spring",
        &[("Kits/p2/kit.dds", 9), ("Kits/g1/kit.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
    let kits = report.validated.unwrap().kits.kits;
    assert_eq!(
        kits.keys().copied().collect::<Vec<_>>(),
        [kit_config::KitSlot::P2, kit_config::KitSlot::G1]
    );
    assert_eq!(kits[&kit_config::KitSlot::P2].textures.len(), 1);
}

#[test]
fn a_full_export_whose_only_player_kit_is_dropped_gets_an_empty_p1() {
    let report = report(
        "co Full Spring",
        &[
            ("Kits/p1/kit.dds", 9),
            ("Kits/p1/pre-fox", 0),
            ("Kits/p1/fox", 0),
            ("Kits/g1/kit.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("kit_layout_conflict", Disposition::DropFolder)]
    );
    let kits = report.validated.unwrap().kits.kits;
    assert_eq!(kits[&kit_config::KitSlot::P1], empty_kit("Kits/p1"));
    assert_eq!(kits.len(), 2);
}

#[test]
fn a_midcup_or_referee_export_gets_no_empty_kit() {
    let midcup = report(
        "co Midcup Day 5",
        &[("Players/03 - A/face_high.fmdl", 10)],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&midcup), vec![]);
    assert!(midcup.validated.unwrap().kits.kits.is_empty());

    let referees = report(
        "refs Spring 2026",
        &[("players.txt", 10)],
        &["Players/Keeper"],
        &[("players.txt", Ok(b"01 Keeper"))],
    );
    assert_eq!(issue_codes(&referees), vec![]);
    assert!(referees.validated.unwrap().kits.kits.is_empty());
}

// TC-KIT-07
#[test]
fn a_texture_without_the_kit_prefix_drops_only_itself() {
    let report = report(
        "egg Midcup",
        &[("Kits/p1/back.dds", 9), ("Kits/p1/kit.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("kit_texture_name_invalid", Disposition::DropFile)]
    );
    assert_eq!(report.issues[0].scope, file_scope("Kits/p1/back.dds"));
    let kit = &report.validated.unwrap().kits.kits[&kit_config::KitSlot::P1];
    assert_eq!(kit.textures.len(), 1);
    assert_eq!(kit.textures[0].stem, "kit");
}

#[test]
fn kit_texture_names_match_the_prefix_in_any_case() {
    // `Kit` is `kit` and `KIT_back` is `kit_back`: the name rule folds case.
    let report = report(
        "egg Midcup",
        &[("Kits/p1/Kit.dds", 9), ("Kits/p1/KIT_back.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![], "{:?}", report.issues);
    let kit = &report.validated.unwrap().kits.kits[&kit_config::KitSlot::P1];
    assert_eq!(
        kit.textures
            .iter()
            .map(|texture| texture.stem.as_str())
            .collect::<Vec<_>>(),
        vec!["kit", "kit_back"]
    );
}

#[test]
fn an_icon_marker_names_the_kit_icon() {
    for (marker, icon) in [
        ("icon_7", 7),
        ("icon_07.txt", 7),
        ("icon_0", 0),
        ("icon_23", 23),
    ] {
        let path = format!("Kits/p1/{marker}");
        let report = report(
            "egg Midcup",
            &[(path.as_str(), 0), ("Kits/p1/kit.dds", 9)],
            &[],
            &[],
        );
        assert_eq!(issue_codes(&report), vec![], "{marker}");
        assert_eq!(
            report.validated.unwrap().kits.kits[&kit_config::KitSlot::P1].icon,
            Some(icon),
            "{marker}"
        );
    }
}

// TC-KIT-08
#[test]
fn an_out_of_range_icon_is_invalid_and_the_kit_kept() {
    for marker in ["icon_24", "icon_25", "icon_99999999999999999999"] {
        let path = format!("Kits/p1/{marker}");
        let report = report(
            "egg Midcup",
            &[(path.as_str(), 0), ("Kits/p1/kit.dds", 9)],
            &[],
            &[],
        );
        assert_eq!(
            issue_codes(&report),
            vec![("kit_icon_invalid", Disposition::DropFile)],
            "{marker}"
        );
        assert_eq!(report.issues[0].scope, file_scope(&path));
        let kit = &report.validated.unwrap().kits.kits[&kit_config::KitSlot::P1];
        assert_eq!(kit.icon, None, "{marker}");
    }
}

#[test]
fn two_icon_markers_are_each_invalid() {
    for (first, second) in [("icon_3", "icon_4"), ("icon_3", "icon_3.txt")] {
        let first = format!("Kits/p1/{first}");
        let second = format!("Kits/p1/{second}");
        let report = report(
            "egg Midcup",
            &[
                (first.as_str(), 0),
                (second.as_str(), 0),
                ("Kits/p1/kit.dds", 9),
            ],
            &[],
            &[],
        );
        assert_eq!(
            issue_codes(&report),
            vec![
                ("kit_icon_invalid", Disposition::DropFile),
                ("kit_icon_invalid", Disposition::DropFile),
            ],
            "{second}"
        );
        assert_eq!(report.issues[0].scope, file_scope(&first));
        assert_eq!(report.issues[1].scope, file_scope(&second));
        let kit = &report.validated.unwrap().kits.kits[&kit_config::KitSlot::P1];
        assert_eq!(kit.icon, None, "{second}");
    }
}

#[test]
fn an_icon_marker_outside_a_kit_folder_is_no_icon() {
    let in_all = report(
        "egg Midcup",
        &[("Kits/all/icon_5", 0), ("Kits/p1/kit.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&in_all),
        vec![("kit_all_file_ignored", Disposition::DropFile)]
    );
    assert_eq!(in_all.issues[0].scope, file_scope("Kits/all/icon_5"));

    // In a player folder an icon marker is a kit marker out of place, as `pre-fox` is.
    let in_player = |marker: &str| {
        let path = format!("Players/03 - A/{marker}");
        let report = report(
            "egg Midcup",
            &[(path.as_str(), 0), ("Players/03 - A/hair.dds", 9)],
            &[],
            &[],
        );
        report
            .issues
            .iter()
            .map(|issue| (issue.code, issue.disposition, issue.context.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        in_player("icon_5"),
        vec![(
            "file_type_disallowed",
            Disposition::DropFolder,
            vec![("file", "icon_5".to_owned())]
        )]
    );
    assert_eq!(
        in_player("pre-fox"),
        vec![(
            "file_type_disallowed",
            Disposition::DropFolder,
            vec![("file", "pre-fox".to_owned())]
        )]
    );
}

// TC-KIT-09
#[test]
fn stem_conflicts_drop_the_folder_they_are_in() {
    let own = report(
        "egg Midcup",
        &[("Kits/p1/kit.png", 9), ("Kits/p1/kit.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&own),
        vec![("texture_stem_conflict", Disposition::DropFolder)]
    );
    assert!(own.validated.unwrap().kits.kits.is_empty());

    let shared = report(
        "egg Midcup",
        &[
            ("Kits/all/kit_back.png", 9),
            ("Kits/all/kit_back.dds", 9),
            ("Kits/p2/kit.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&shared),
        vec![("texture_stem_conflict", Disposition::DropFolder)]
    );
    assert_eq!(shared.issues[0].scope, folder("Kits/all"));
    let kits = &shared.validated.unwrap().kits;
    assert!(kits.shared.is_empty());
    assert_eq!(kits.kits[&kit_config::KitSlot::P2].textures.len(), 1);

    let overridden = report(
        "egg Midcup",
        &[("Kits/all/kit_name.dds", 9), ("Kits/p3/kit_name.png", 9)],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&overridden), vec![]);
}

#[test]
fn an_all_folder_beside_only_invalid_kits_is_unused() {
    let report = report("egg Midcup", &[("Kits/all/kit.dds", 9)], &["Kits/x1"], &[]);
    assert_eq!(
        issue_codes(&report),
        vec![
            ("kit_folder_invalid", Disposition::DropFolder),
            ("kit_all_unused", Disposition::Keep),
        ]
    );
    assert_eq!(report.issues[1].scope, folder("Kits/all"));
    assert_eq!(report.validated.unwrap().kits.shared.len(), 1);
}

#[test]
fn a_non_texture_in_a_kit_drops_the_kit_strict() {
    let report = report(
        "egg Midcup",
        &[("Kits/p1/kit.dds", 9), ("Kits/p1/hair.fmdl", 10)],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("file_type_disallowed", Disposition::DropFolder)]
    );
    assert_eq!(report.issues[0].scope, folder("Kits/p1"));
    assert!(report.validated.unwrap().kits.kits.is_empty());
}

#[test]
fn a_kit_file_below_a_subfolder_offends() {
    let report = report(
        "egg Midcup",
        &[("Kits/p1/kit.dds", 9), ("Kits/p1/extra/kit_back.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("file_type_disallowed", Disposition::DropFolder)]
    );
    assert_eq!(
        report.issues[0].context,
        vec![("file", "extra/kit_back.dds".to_owned())]
    );
}

#[test]
fn a_kit_s_config_and_colors_below_a_subfolder_are_not_its_own() {
    // Lenient, so the folder stays and its kit can be read.
    let report = report_with(
        &context_with(false, false),
        "egg Midcup",
        &[
            ("Kits/p1/kit.dds", 9),
            ("Kits/p1/sub/config.toml", 9),
            ("Kits/p1/sub/colors.txt", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("file_type_disallowed", Disposition::Keep),
            ("file_type_disallowed", Disposition::Keep),
        ]
    );
    let kit = &report.validated.unwrap().kits.kits[&kit_config::KitSlot::P1];
    assert_eq!(kit.config, None);
    assert_eq!(kit.colors, None);
}

#[test]
fn layout_markers_pick_the_kit_layout() {
    let report = report(
        "egg Midcup",
        &[
            ("Kits/p1/kit.dds", 9),
            ("Kits/p1/fox", 0),
            ("Kits/p2/kit.dds", 9),
            ("Kits/p2/pre-fox", 0),
            ("Kits/p3/kit.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
    let kits = &report.validated.unwrap().kits;
    assert_eq!(
        kits.kits[&kit_config::KitSlot::P1].layout,
        Some(KitLayout::Fox)
    );
    assert_eq!(
        kits.kits[&kit_config::KitSlot::P2].layout,
        Some(KitLayout::PreFox)
    );
    assert_eq!(kits.kits[&kit_config::KitSlot::P3].layout, None);
}
