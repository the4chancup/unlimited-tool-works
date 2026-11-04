use super::*;

// TC-STR-15
#[test]
fn a_common_link_resolves_or_drops_the_player() {
    let report = report(
        "egg Midcup",
        &[
            ("Common/torso.fmdl", 10),
            ("Players/01 - A/torso.fmdl.common", 0),
            ("Players/02 - B/torso.fmdl.common.txt", 0),
            ("Players/03 - C/missing.fmdl.common", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("common_link_missing", Disposition::DropFolder)]
    );
    assert_eq!(report.issues[0].scope, folder("Players/03 - C"));
    assert_eq!(
        report.issues[0].context,
        link_path("missing.fmdl.common", "Common/missing.fmdl")
    );
    let validated = report.validated.unwrap();
    assert_eq!(
        validated
            .players
            .iter()
            .map(|player| player.path.as_str())
            .collect::<Vec<_>>(),
        vec!["Players/01 - A", "Players/02 - B"]
    );
    // Resolved `.common` links stay as link-kind files.
    assert_eq!(validated.players[0].files[0].kind, FileKind::CommonLink);
}

/// The default context with the installed CPKs holding the team's Common textures `stems`,
/// folded as the consumer folds them.
fn installed(stems: &[&str]) -> ValidationContext {
    ValidationContext {
        installed_common_textures: stems.iter().map(|stem| vtree::fold_name(stem)).collect(),
        ..crate::testing::context()
    }
}

#[test]
fn a_texture_link_whose_texture_an_installed_cpk_holds_stays_with_no_finding() {
    // The set is folded by its producer and the link's stem by validation, so either spelling
    // matches.
    for (link, held) in [("hair.dds.common", "hair"), ("Hair.dds.common", "HAIR")] {
        let report = report_with(
            &installed(&[held]),
            "egg Midcup",
            &[
                ("Players/03 - A/face_high.fmdl", 10),
                (&format!("Players/03 - A/{link}"), 0),
            ],
            &[],
            &[],
        );
        assert_eq!(issue_codes(&report), vec![], "{link}");
        assert_eq!(
            player_file_names(&report, 0),
            vec!["face_high.fmdl".to_owned(), link.to_owned()],
            "{link}"
        );
    }
}

#[test]
fn a_model_link_is_common_link_missing_whatever_the_installed_cpks_hold() {
    let report = report_with(
        &installed(&["face_high"]),
        "egg Midcup",
        &[("Players/03 - A/face_high.fmdl.common", 0)],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("common_link_missing", Disposition::DropFolder)]
    );
    assert_eq!(report.issues[0].scope, folder("Players/03 - A"));
}

#[test]
fn with_no_installed_texture_a_texture_link_with_no_target_is_common_link_missing() {
    for context in [installed(&[]), installed(&["skin"])] {
        let report = report_with(
            &context,
            "egg Midcup",
            &[
                ("Players/03 - A/face_high.fmdl", 10),
                ("Players/03 - A/hair.dds.common", 0),
            ],
            &[],
            &[],
        );
        assert_eq!(
            issue_codes(&report),
            vec![("common_link_missing", Disposition::DropFolder)]
        );
        assert_eq!(
            report.issues[0].context,
            link_path("hair.dds.common", "Common/hair.dds")
        );
    }
}

#[test]
fn a_common_file_off_the_allowlist_drops_or_keeps() {
    let strict = report("egg Midcup", &[("Common/x.exe", 4)], &[], &[]);
    assert_eq!(
        issue_codes(&strict),
        vec![("common_file_disallowed", Disposition::DropFile)]
    );
    assert_eq!(
        strict.issues[0].scope,
        IssueScope::File(ScopePath::new("Common/x.exe").unwrap())
    );
    assert!(strict.validated.unwrap().common.is_empty());

    let lenient = report_with(
        &context_with(false, false),
        "egg Midcup",
        &[("Common/x.exe", 4)],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&lenient),
        vec![("common_file_disallowed", Disposition::Keep)]
    );
    assert_eq!(lenient.validated.unwrap().common.len(), 1);
}

#[test]
fn only_model_files_directly_in_collars_are_kept() {
    let files = [
        ("Collars/collar_12.fmdl", 10),
        ("Collars/collar_13.model", 10),
        ("Collars/collar_14.glb", 10),
        ("Collars/collar_15.gltf", 10),
        ("Collars/collar_12.dds", 9),
        ("Collars/collar_12.mtl", 9),
        ("Collars/old/collar_16.fmdl", 10),
    ];
    let strict = report("co Midcup", &files, &[], &[]);
    let disallowed = |path: &str, file: &str| ValidationIssue {
        code: "file_type_disallowed",
        scope: IssueScope::File(ScopePath::new(path).unwrap()),
        context: vec![("file", file.to_owned())],
        disposition: Disposition::DropFile,
        passed_through: false,
    };
    assert_eq!(
        strict.issues,
        [
            disallowed("Collars/collar_12.dds", "collar_12.dds"),
            disallowed("Collars/collar_12.mtl", "collar_12.mtl"),
            disallowed("Collars/old/collar_16.fmdl", "old/collar_16.fmdl"),
        ]
    );
    let mut kept: Vec<String> = strict
        .validated
        .unwrap()
        .collars
        .iter()
        .map(|file| file.path.as_str().to_owned())
        .collect();
    kept.sort_unstable();
    assert_eq!(
        kept,
        [
            "Collars/collar_12.fmdl",
            "Collars/collar_13.model",
            "Collars/collar_14.glb",
            "Collars/collar_15.gltf",
        ]
    );

    // With the strict check off, the files are only noted and stay.
    let lenient = report_with(&context_with(false, false), "co Midcup", &files, &[], &[]);
    assert_eq!(
        issue_codes(&lenient),
        vec![("file_type_disallowed", Disposition::Keep); 3]
    );
    assert_eq!(lenient.validated.unwrap().collars.len(), files.len());
}

#[test]
fn pass_through_keeps_the_resolved_common_link_and_prunes_the_other() {
    let report = report_with(
        &context_with(true, true),
        "egg Midcup",
        &[
            ("players.txt", 5),
            ("Common/torso.fmdl", 10),
            ("Players/A/torso.fmdl.common", 0),
            ("Players/A/missing.fmdl.common", 0),
            ("Players/A/hair.dds", 9),
        ],
        &["Players/A"],
        &[("players.txt", Ok(b"03 A"))],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("common_link_missing", Disposition::Keep)]
    );
    assert!(report.issues[0].passed_through);
    assert_eq!(
        player_file_names(&report, 0),
        vec!["hair.dds", "torso.fmdl.common"]
    );
}

#[test]
fn a_nested_common_model_file_is_admitted_but_no_link_target() {
    let report = report(
        "egg Midcup",
        &[
            ("Common/sub/torso.fmdl", 10),
            ("Common/sub/notes.doc", 4),
            ("Players/03 - A/torso.fmdl.common", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("common_link_missing", Disposition::DropFolder),
            ("common_file_disallowed", Disposition::DropFile),
        ]
    );
    assert_eq!(report.issues[0].scope, folder("Players/03 - A"));
    assert_eq!(
        report.issues[1].scope,
        IssueScope::File(ScopePath::new("Common/sub/notes.doc").unwrap())
    );
    let validated = report.validated.unwrap();
    let common: Vec<&str> = validated
        .common
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    assert_eq!(common, vec!["Common/sub/torso.fmdl"]);
}

/// The `Common/` paths the player's `.common` links resolve to, in the export `files` parses to.
fn link_targets(files: &[(&str, u64)]) -> Vec<Option<String>> {
    let draft = crate::testing::parsed("egg Midcup", files, &[], &[]).draft;
    links::player_links(&draft.players[0], &draft)
        .into_iter()
        .map(|link| link.target.map(|target| target.as_str().to_owned()))
        .collect()
}

#[test]
fn a_common_link_below_a_subfolder_stands_for_the_common_file_at_its_path() {
    let files = [
        ("Common/jessie/body.fmdl", 10),
        ("Players/03 - A/face_high.fmdl", 10),
        ("Players/03 - A/jessie/body.fmdl.common", 0),
    ];
    let report = report("egg Midcup", &files, &[], &[]);
    assert_eq!(issue_codes(&report), vec![]);
    let player = &report.validated.unwrap().players[0];
    assert!(
        player
            .files
            .iter()
            .any(|file| file.path.as_str() == "Players/03 - A/jessie/body.fmdl.common")
    );
    assert_eq!(
        link_targets(&files),
        [Some("Common/jessie/body.fmdl".to_owned())]
    );
}

#[test]
fn a_common_link_below_a_subfolder_names_no_direct_common_file() {
    let report = report(
        "egg Midcup",
        &[
            ("Common/body.fmdl", 10),
            ("Players/03 - A/face_high.fmdl", 10),
            ("Players/03 - A/jessie/body.fmdl.common", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("common_link_missing", Disposition::DropFolder)]
    );
    assert_eq!(report.issues[0].scope, folder("Players/03 - A"));
    assert_eq!(
        report.issues[0].context,
        link_path("body.fmdl.common", "Common/jessie/body.fmdl")
    );
    assert!(report.validated.unwrap().players.is_empty());
}

#[test]
fn a_texture_link_below_a_subfolder_stands_for_the_common_texture_at_its_path() {
    let files = [
        ("Common/jessie/hair.dds", 9),
        ("Players/03 - A/face_high.fmdl", 10),
        ("Players/03 - A/jessie/hair.dds.common", 0),
    ];
    let compiled = report("egg Midcup", &files, &[], &[]);
    assert_eq!(issue_codes(&compiled), vec![], "{:?}", compiled.issues);
    assert_eq!(
        link_targets(&files),
        [Some("Common/jessie/hair.dds".to_owned())]
    );

    // With only a direct `Common/hair.dds`, the link still looks at its own path.
    let missing = report(
        "egg Midcup",
        &[
            ("Common/hair.dds", 9),
            ("Players/03 - A/face_high.fmdl", 10),
            ("Players/03 - A/jessie/hair.dds.common", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&missing),
        vec![("common_link_missing", Disposition::DropFolder)]
    );
    assert_eq!(
        missing.issues[0].context,
        link_path("hair.dds.common", "Common/jessie/hair.dds")
    );
}

#[test]
fn a_common_link_naming_a_kit_set_resolves_to_its_lowest_variant() {
    for (files, expected) in [
        // `kitN/armor_bsm.dds.common` names the set `Common/`'s `kit1`…`kit9` folders hold of
        // `armor_bsm.dds`; the link resolves to its lowest variant.
        (
            &[
                ("Common/kit2/armor_bsm.dds", 9),
                ("Common/kit1/armor_bsm.dds", 9),
                ("Players/03 - A/face_high.fmdl", 10),
                ("Players/03 - A/kitN/armor_bsm.dds.common", 0),
            ][..],
            "Common/kit1/armor_bsm.dds",
        ),
        // A `kitN` stem's set: `pants_kitN.dds.common` names `Common/pants_kit2.dds`.
        (
            &[
                ("Common/pants_kit2.dds", 9),
                ("Players/03 - A/face_high.fmdl", 10),
                ("Players/03 - A/pants_kitN.dds.common", 0),
            ][..],
            "Common/pants_kit2.dds",
        ),
    ] {
        let report = report("egg Midcup", files, &[], &[]);
        assert_eq!(issue_codes(&report), vec![], "{:?}", report.issues);
        assert_eq!(link_targets(files), [Some(expected.to_owned())]);
    }
}

#[test]
fn common_link_files_orders_a_texture_set_s_members_representative_first() {
    let files = [
        ("Common/kitF/a.dds", 9),
        ("Common/kit2/a.dds", 9),
        ("Common/kit1/a.dds", 9),
        ("Common/kitF/b.dds", 9),
        ("Common/boots_kit1.fmdl", 10),
        ("Common/boots_kitF.skl", 10),
        ("Common/pants_kit1.dds", 9),
        ("Common/pants_kitF.png", 9),
        ("Common/pants_kit2.mtl", 10),
    ];
    let draft = crate::testing::parsed("egg Midcup", &files, &[], &[]).draft;
    let reached = |below: &str| -> Vec<&str> {
        common_link_files(&draft.common, below)
            .iter()
            .map(|file| file.path.as_str())
            .collect()
    };
    // The set's members, lowest variant first, the `kitF` member last.
    assert_eq!(
        reached("kitN/a.dds"),
        [
            "Common/kit1/a.dds",
            "Common/kit2/a.dds",
            "Common/kitF/a.dds"
        ]
    );
    // A set of the `kitF` file alone is one fallback for every number.
    assert_eq!(reached("kitN/b.dds"), ["Common/kitF/b.dds"]);
    // The members are a set by stem, whatever their extensions; a `.mtl` of the
    // set's stem is no texture and the link does not reach it.
    assert_eq!(
        reached("pants_kitN.dds"),
        ["Common/pants_kit1.dds", "Common/pants_kitF.png"]
    );
    // A path naming the fallback finds nothing.
    assert!(reached("kitF/a.dds").is_empty());
    // A link to a model or skeleton is a literal lookup: `boots_kitN.fmdl` matches
    // nothing the set names, and `boots_kitF.skl` is a plain name to it.
    assert!(reached("boots_kitN.fmdl").is_empty());
    assert_eq!(reached("boots_kitF.skl"), ["Common/boots_kitF.skl"]);
}

#[test]
fn common_link_files_sorts_the_members_it_is_given_whatever_their_order() {
    // The members' order is the function's contract, not its input's: a
    // `common` slice in any order gives the lowest variant first, `kitF` last.
    let descriptor = |path: &str| FileDescriptor {
        path: ScopePath::new(path).unwrap(),
        source: ScopePath::new(path).unwrap(),
        size: 9,
        kind: FileKind::Texture,
    };
    let common = [
        descriptor("Common/kitF/a.dds"),
        descriptor("Common/kit2/a.dds"),
        descriptor("Common/kit1/a.dds"),
    ];
    let reached: Vec<&str> = common_link_files(&common, "kitN/a.dds")
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    assert_eq!(
        reached,
        [
            "Common/kit1/a.dds",
            "Common/kit2/a.dds",
            "Common/kitF/a.dds"
        ]
    );
}

#[test]
fn a_common_link_naming_a_set_of_the_kitf_file_alone_resolves_to_it() {
    let files = [
        ("Common/kitF/b.dds", 9),
        ("Players/03 - A/face_high.fmdl", 10),
        ("Players/03 - A/kitN/b.dds.common", 0),
    ];
    let report = report("egg Midcup", &files, &[], &[]);
    assert_eq!(issue_codes(&report), vec![], "{:?}", report.issues);
    assert_eq!(link_targets(&files), [Some("Common/kitF/b.dds".to_owned())]);
}

#[test]
fn a_common_link_naming_the_kitf_fallback_or_an_empty_set_is_common_link_missing() {
    // `kitF` is never a file under its own name: `Common/kitF/armor_bsm.dds` sits there,
    // and a link naming it finds nothing.
    let fallback = report(
        "egg Midcup",
        &[
            ("Common/kitF/armor_bsm.dds", 9),
            ("Players/03 - A/face_high.fmdl", 10),
            ("Players/03 - A/kitF/armor_bsm.dds.common", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&fallback),
        vec![("common_link_missing", Disposition::DropFolder)]
    );
    assert_eq!(
        fallback.issues[0].context,
        link_path("armor_bsm.dds.common", "Common/kitF/armor_bsm.dds")
    );

    // Another file in a member folder is no member of `armor_bsm`'s set.
    let missing = report(
        "egg Midcup",
        &[
            ("Common/kit1/other.dds", 9),
            ("Players/03 - A/face_high.fmdl", 10),
            ("Players/03 - A/kitN/armor_bsm.dds.common", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&missing),
        vec![("common_link_missing", Disposition::DropFolder)]
    );
    assert_eq!(
        missing.issues[0].context,
        link_path("armor_bsm.dds.common", "Common/kitN/armor_bsm.dds")
    );
}

#[test]
fn a_texture_link_below_a_subfolder_an_installed_cpk_satisfies_stays_with_no_finding() {
    let files = [
        ("Players/03 - A/face_high.fmdl", 10),
        ("Players/03 - A/jessie/hair.dds.common", 0),
    ];
    let report = report_with(&installed(&["jessie/hair"]), "egg Midcup", &files, &[], &[]);
    assert_eq!(issue_codes(&report), vec![], "{:?}", report.issues);
    // `hair` alone installed is not `jessie/hair`.
    let missing = report_with(&installed(&["hair"]), "egg Midcup", &files, &[], &[]);
    assert_eq!(
        issue_codes(&missing),
        vec![("common_link_missing", Disposition::DropFolder)]
    );
}

#[test]
fn ingame_face_and_common_links_act_by_target_at_any_depth() {
    let report = report(
        "egg Midcup",
        &[
            ("Common/face_high.fmdl", 10),
            ("Common/torso.fmdl", 10),
            ("Common/face_high.dds", 9),
            ("Common/boots/face_high.fmdl", 10),
            ("Players/01 - A/ingame_face.toml", 0),
            ("Players/01 - A/face_high.fmdl.common", 0),
            ("Players/02 - B/ingame_face.toml", 0),
            ("Players/02 - B/torso.fmdl.common", 0),
            ("Players/03 - C/ingame_face.toml", 0),
            ("Players/03 - C/face_high.dds.common", 0),
            ("Players/04 - D/ingame_face.toml", 0),
            ("Players/04 - D/boots/face_high.fmdl.common", 0),
        ],
        &[],
        &[],
    );
    // A's and D's explicit-face-model links contradict the marker, D's below a subfolder as
    // one in the root would; B's torso and C's texture do not.
    assert_eq!(
        issue_codes(&report),
        vec![("ingame_face_explicit_face_model", Disposition::DropFolder); 2]
    );
    assert_eq!(report.issues[0].scope, folder("Players/01 - A"));
    assert_eq!(report.issues[1].scope, folder("Players/04 - D"));
    let validated = report.validated.unwrap();
    assert_eq!(
        validated
            .players
            .iter()
            .map(|player| player.path.as_str())
            .collect::<Vec<_>>(),
        vec!["Players/02 - B", "Players/03 - C"]
    );
}

#[test]
fn a_common_link_in_common_is_no_link_either() {
    // A link in the export's `Common/` is no model content there; a player's subfolder named
    // `common/` is a subfolder like any other, its link naming `Common/common/x.fmdl`.
    let report = report_with(
        &context_with(false, false),
        "egg Midcup",
        &[
            ("Common/x.fmdl", 10),
            ("Common/sub/x.fmdl.common", 0),
            ("Players/03 - A/face_high.fmdl", 10),
            ("Players/03 - A/common/x.fmdl.common", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("common_link_missing", Disposition::DropFolder),
            ("common_file_disallowed", Disposition::Keep),
        ]
    );
    assert_eq!(
        report.issues[0].context,
        link_path("x.fmdl.common", "Common/common/x.fmdl")
    );
}

#[test]
fn a_common_link_below_common_is_a_link_to_common_common() {
    // `common/` is a plain subfolder, so the link stands for `Common/common/hair.png`,
    // claiming `hair` in `common/`'s own namespace, not the root's.
    let missing = report(
        "egg Midcup",
        &[
            ("Common/hair.png", 9),
            ("Players/03 - A/hair.dds", 9),
            ("Players/03 - A/common/hair.png.common", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&missing),
        vec![("common_link_missing", Disposition::DropFolder)]
    );
    assert_eq!(
        missing.issues[0].context,
        link_path("hair.png.common", "Common/common/hair.png")
    );

    let clean = report(
        "egg Midcup",
        &[
            ("Common/common/hair.png", 9),
            ("Players/03 - A/hair.dds", 9),
            ("Players/03 - A/common/hair.png.common", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&clean), vec![], "{:?}", clean.issues);
}
