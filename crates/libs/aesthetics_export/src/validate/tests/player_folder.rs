use super::*;

#[test]
fn a_player_folder_builds_with_its_links_and_markers() {
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/hair.dds", 9),
            ("Players/03 - A/gloves/glove_l.fmdl", 10),
            ("Players/03 - A/common/skin.dds", 9),
            ("Players/03 - A/Crocs.boots.txt", 0),
            ("Players/03 - A/ingame_face.toml", 0),
            ("Players/03 - A/fpc_on", 0),
            ("Players/03 - A/portrait.dds", 9),
            ("Players/03 - A/settings.toml", 20),
            ("Boots/Crocs/boots.fmdl", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
    let folder = &report.validated.unwrap().players[0];
    assert_eq!(folder.player_name, "A");
    assert_eq!(
        folder.links,
        vec![SharedLink {
            kind: SharedKind::Boots,
            name: "Crocs".to_owned(),
        }]
    );
    assert!(folder.ingame_face.is_some());
    assert_eq!(folder.fpc, Some(FpcDirective::On));
    assert_eq!(
        folder.portrait.as_ref().map(|f| f.path.as_str()),
        Some("Players/03 - A/portrait.dds")
    );
    assert_eq!(
        folder.settings.as_ref().map(|f| f.path.as_str()),
        Some("Players/03 - A/settings.toml")
    );
    // Pipeline content: loose files and subfolder files alike.
    assert_eq!(
        folder
            .files
            .iter()
            .map(|f| f.path.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Players/03 - A/common/skin.dds",
            "Players/03 - A/gloves/glove_l.fmdl",
            "Players/03 - A/hair.dds",
        ]
    );
    assert_eq!(folder.files[1].kind, FileKind::Model(ModelFormat::Fmdl));
}

#[test]
fn only_the_portrait_stem_is_the_portrait() {
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/hair.dds", 9),
            ("Players/03 - A/portrait.dds", 9),
        ],
        &[],
        &[],
    );
    let folder = &report.validated.unwrap().players[0];
    assert_eq!(
        folder.portrait.as_ref().map(|file| file.path.as_str()),
        Some("Players/03 - A/portrait.dds")
    );
    assert_eq!(
        folder
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        vec!["Players/03 - A/hair.dds"]
    );
}

#[test]
fn the_portrait_stem_matches_case_insensitively() {
    let report = report(
        "egg Midcup",
        &[("Players/03 - A/Portrait.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
    let folder = &report.validated.unwrap().players[0];
    assert_eq!(
        folder.portrait.as_ref().map(|file| file.path.as_str()),
        Some("Players/03 - A/Portrait.dds")
    );
}

// TC-STR-13
#[test]
fn two_textures_sharing_a_stem_drop_the_folder() {
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/hair.dds", 9),
            ("Players/03 - A/hair.png", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("texture_stem_conflict", Disposition::DropFolder)]
    );
    assert_eq!(report.issues[0].scope, folder("Players/03 - A"));
    assert!(report.validated.unwrap().players.is_empty());
}

#[test]
fn a_subfolder_s_texture_shares_no_stem_with_the_root_s() {
    // A subfolder is a folder of its own: its `hair.dds` is looked up beside its models first,
    // so it is no second `hair` of the root's.
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/hair.dds", 9),
            ("Players/03 - A/common/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
    assert_eq!(report.validated.unwrap().players.len(), 1);
}

#[test]
fn each_folder_of_a_player_s_tree_is_a_stem_namespace_of_its_own() {
    // The root's `hair.dds` and `jessie/hair.dds` share no namespace; `jessie/`'s two do, and
    // so do `jessie/body/`'s, each folder's conflict reported once.
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/hair.dds", 9),
            ("Players/03 - A/jessie/hair.dds", 9),
            ("Players/03 - A/jessie/hair.png", 9),
            ("Players/03 - A/jessie/body/Skin.dds", 9),
            ("Players/03 - A/jessie/body/skin.tga", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("texture_stem_conflict", Disposition::DropFolder); 2]
    );
    assert_eq!(report.issues[0].scope, folder("Players/03 - A"));
    let files: Vec<&str> = report
        .issues
        .iter()
        .map(|issue| issue.context[1].1.as_str())
        .collect();
    assert_eq!(
        files,
        [
            "jessie/hair.dds,jessie/hair.png",
            "jessie/body/Skin.dds,jessie/body/skin.tga"
        ]
    );
    assert!(report.validated.unwrap().players.is_empty());
}

#[test]
fn a_texture_link_below_a_subfolder_claims_its_stem_in_its_own_folder() {
    // A link claims its linked stem in the folder holding it: beside `jessie/`'s own
    // `hair.dds` they are one stem twice; beside the root's they are not.
    let files = [
        ("Common/jessie/hair.png", 9),
        ("Players/03 - A/face_high.model", 10),
        ("Players/03 - A/jessie/hair.dds", 9),
        ("Players/03 - A/jessie/hair.png.common", 0),
    ];
    let conflict = report("egg Midcup", &files, &[], &[]);
    assert_eq!(
        issue_codes(&conflict),
        [("texture_stem_conflict", Disposition::DropFolder)],
        "{:?}",
        conflict.issues
    );
    assert!(conflict.validated.unwrap().players.is_empty());

    let clean = report(
        "egg Midcup",
        &[
            ("Common/jessie/hair.png", 9),
            ("Players/03 - A/face_high.model", 10),
            ("Players/03 - A/hair.dds", 9),
            ("Players/03 - A/jessie/hair.png.common", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&clean), vec![], "{:?}", clean.issues);
    assert_eq!(clean.validated.unwrap().players.len(), 1);
}

#[test]
fn a_per_player_singleton_below_a_subfolder_is_file_type_disallowed() {
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/face_high.model", 10),
            ("Players/03 - A/face.xml", 10),
            ("Players/03 - A/face_diff.bin", 10),
            ("Players/03 - A/jessie/Face.XML", 10),
            ("Players/03 - A/jessie/face_diff.bin", 10),
            ("Players/03 - A/jessie/body/face_diff.xml", 10),
            ("Players/03 - A/jessie/fcl_hair_sim.fclo", 10),
            // Another `.xml`, `.bin` or `.fclo` below a subfolder is model content.
            ("Players/03 - A/jessie/glove.xml", 10),
            ("Players/03 - A/jessie/x.bin", 10),
            ("Players/03 - A/jessie/x.fclo", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("file_type_disallowed", Disposition::DropFolder); 4]
    );
    assert_eq!(
        issue_files(&report),
        [
            "jessie/body/face_diff.xml",
            "jessie/Face.XML",
            "jessie/face_diff.bin",
            "jessie/fcl_hair_sim.fclo",
        ]
    );
    // As a subfolder's marker does under the strict check, it drops the player folder.
    assert!(report.validated.unwrap().players.is_empty());
}

#[test]
fn a_skl_mtl_or_materials_toml_below_a_subfolder_is_model_content() {
    // `.skl`, `.mtl` and `materials.toml` are model content like the subfolder's
    // models and textures; the name matches case-insensitively.
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/jessie/body.skl", 10),
            ("Players/03 - A/jessie/x.mtl", 9),
            ("Players/03 - A/jessie/materials.toml", 9),
            ("Players/04 - B/jessie/Materials.toml", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![], "{:?}", report.issues);
    assert_eq!(report.validated.unwrap().players.len(), 2);
}

#[test]
fn fpc_off_alone_is_an_off_directive() {
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/face_high.fmdl", 10),
            ("Players/03 - A/fpc_off", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(
        report.validated.unwrap().players[0].fpc,
        Some(FpcDirective::Off)
    );
}

#[test]
fn a_bare_fpc_marker_is_an_on_directive() {
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/face_high.fmdl", 10),
            ("Players/03 - A/fpc", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
    assert_eq!(
        report.validated.unwrap().players[0].fpc,
        Some(FpcDirective::On)
    );
}

#[test]
fn the_dotted_marker_spellings_are_disallowed_files() {
    for (files, folder_path, file) in [
        (
            &[
                ("Players/03 - A/face_high.fmdl", 10),
                ("Players/03 - A/fpc.on", 0),
            ][..],
            "Players/03 - A",
            "fpc.on",
        ),
        (
            &[("Kits/p1/kit.dds", 9), ("Kits/p1/icon.txt", 2)][..],
            "Kits/p1",
            "icon.txt",
        ),
    ] {
        let report = report("egg Midcup", files, &[], &[]);
        assert_eq!(
            issue_codes(&report),
            vec![("file_type_disallowed", Disposition::DropFolder)],
            "{file}"
        );
        assert_eq!(report.issues[0].scope, folder(folder_path));
        assert_eq!(report.issues[0].context, vec![("file", file.to_owned())]);
    }
}

// TC-STR-12
#[test]
fn both_fpc_markers_drop_the_folder() {
    // A roster-mapped player dropped by its own finding.
    let report = report(
        "egg Midcup",
        &[
            ("players.txt", 5),
            ("Players/A/hair.dds", 9),
            ("Players/A/fpc_on", 0),
            ("Players/A/fpc_off", 0),
        ],
        &[],
        &[("players.txt", Ok(b"03 A"))],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("fpc_conflict", Disposition::DropFolder)]
    );
    let validated = report.validated.unwrap();
    assert!(validated.players.is_empty());
    let ValidatedRoster::Team(map) = &validated.roster else {
        panic!("a team export");
    };
    assert!(map.is_empty());
}

#[test]
fn a_disallowed_file_drops_strict_and_keeps_lenient() {
    let files = &[
        ("Players/03 - A/hair.dds", 9u64),
        ("Players/03 - A/readme.doc", 20),
    ];
    let strict = report("egg Midcup", files, &[], &[]);
    assert_eq!(
        issue_codes(&strict),
        vec![("file_type_disallowed", Disposition::DropFolder)]
    );
    assert!(strict.validated.unwrap().players.is_empty());

    let lenient = report_with(&context_with(false, false), "egg Midcup", files, &[], &[]);
    assert_eq!(
        issue_codes(&lenient),
        vec![("file_type_disallowed", Disposition::Keep)]
    );
    assert_eq!(
        player_file_names(&lenient, 0),
        vec!["hair.dds", "readme.doc"]
    );
}

#[test]
fn a_txt_note_is_admitted_in_a_model_folder_but_not_in_a_kit_folder() {
    // Any `.txt` that is not a link's or marker's tolerated spelling is a note, a metadata
    // name (`notes.txt`, `readme.txt`) or an ordinary one alike: model content, at the
    // folder's root, below a subfolder and in a shared folder.
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/face_high.fmdl", 10),
            ("Players/03 - A/notes.txt", 4),
            ("Players/03 - A/readme.txt", 4),
            ("Players/03 - A/jessie/notes.txt", 4),
            ("Players/03 - A/jessie/guide.txt", 4),
            ("Players/03 - A/Crocs.boots", 0),
            // A `.txt` a known file hides under is no note: the name without it is a settings
            // file and a texture, so the misnamed files stay disallowed.
            ("Players/04 - B/face_high.fmdl", 10),
            ("Players/04 - B/settings.toml.txt", 4),
            ("Players/04 - B/skin.dds.txt", 4),
            ("Boots/Crocs/boots.fmdl", 10),
            ("Boots/Crocs/notes.txt", 4),
            ("Boots/Crocs/extra/guide.txt", 4),
            ("Kits/p1/kit.dds", 9),
            ("Kits/p1/notes.txt", 4),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("file_type_disallowed", Disposition::DropFolder); 3]
    );
    assert_eq!(report.issues[0].scope, folder("Players/04 - B"));
    assert_eq!(
        report.issues[0].context,
        vec![("file", "settings.toml.txt".to_owned())]
    );
    assert_eq!(
        report.issues[1].context,
        vec![("file", "skin.dds.txt".to_owned())]
    );
    assert_eq!(report.issues[2].scope, folder("Kits/p1"));
    assert_eq!(
        report.issues[2].context,
        vec![("file", "notes.txt".to_owned())]
    );
    let validated = report.validated.unwrap();
    assert_eq!(validated.players.len(), 1);
    assert_eq!(validated.boots.len(), 1);
}

#[test]
fn the_marker_s_old_spellings_are_reported_and_only_ingame_face_toml_marks() {
    let report = report(
        "egg Midcup",
        &[
            ("Players/05 - A/ingame_face.toml", 0),
            ("Players/05 - A/hair.dds", 9),
            ("Players/06 - B/ingame_face", 0),
            ("Players/06 - B/hair.dds", 9),
            ("Players/07 - C/ingame_face.txt", 0),
            ("Players/07 - C/hair.dds", 9),
        ],
        &[],
        &[],
    );
    // Each old spelling is reported on its file alone and marks nothing: the
    // folders are built as ones without it.
    assert_eq!(
        issue_codes(&report),
        vec![
            ("ingame_face_marker_renamed", Disposition::DropFile),
            ("ingame_face_marker_renamed", Disposition::DropFile),
        ]
    );
    assert_eq!(
        report
            .issues
            .iter()
            .map(|issue| issue.scope.clone())
            .collect::<Vec<_>>(),
        [
            IssueScope::File(ScopePath::new("Players/06 - B/ingame_face").unwrap()),
            IssueScope::File(ScopePath::new("Players/07 - C/ingame_face.txt").unwrap()),
        ]
    );
    let validated = report.validated.unwrap();
    let marked: Vec<bool> = validated
        .players
        .iter()
        .map(|player| player.ingame_face.is_some())
        .collect();
    assert_eq!(marked, [true, false, false]);
}

/// Off the player folder's direct position everything is today's finding:
/// an old-spelling marker or an `ingame_face.toml` inside a subfolder is
/// `file_type_disallowed` like `settings.toml`, and `ingame_face.toml.txt`
/// is not recognized, as `settings.toml.txt` is not.
#[test]
fn the_marker_files_are_disallowed_in_a_subfolder_or_with_a_txt_suffix() {
    let report = report_with(
        &context_with(false, false),
        "egg Midcup",
        &[
            ("Players/03 - A/face_high.fmdl", 10),
            ("Players/03 - A/jessie/ingame_face", 0),
            ("Players/03 - A/jessie/ingame_face.toml", 0),
            ("Players/03 - A/ingame_face.toml.txt", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("file_type_disallowed", Disposition::Keep),
            ("file_type_disallowed", Disposition::Keep),
            ("file_type_disallowed", Disposition::Keep),
        ]
    );
    assert_eq!(
        issue_files(&report),
        [
            "ingame_face.toml.txt",
            "jessie/ingame_face",
            "jessie/ingame_face.toml",
        ]
    );
    // None of them counts: the player keeps his face model and has no
    // in-game face.
    let player = &report.validated.unwrap().players[0];
    assert!(player.ingame_face.is_none());
}

// TC-STR-11
#[test]
fn ingame_face_with_explicit_face_content_drops_the_folder() {
    let local = report(
        "egg Midcup",
        &[
            ("Players/03 - A/ingame_face.toml", 0),
            ("Players/03 - A/face_high.fmdl", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&local),
        vec![("ingame_face_explicit_face_model", Disposition::DropFolder)]
    );
    assert!(local.validated.unwrap().players.is_empty());

    let linked = report(
        "egg Midcup",
        &[
            ("Players/03 - A/ingame_face.toml", 0),
            ("Players/03 - A/Long.face", 0),
            ("Faces/Long/face_high.fmdl", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&linked),
        vec![
            ("ingame_face_explicit_face_model", Disposition::DropFolder),
            ("shared_folder_orphaned", Disposition::DropFolder),
        ]
    );
    let validated = linked.validated.unwrap();
    assert!(validated.players.is_empty());
    assert!(validated.faces.is_empty());
}

#[test]
fn an_oral_model_is_explicit_face_content_too() {
    // `oral` has no skeleton slot either: beside `ingame_face` it drops the
    // folder as `face_high` and `hair_high` do.
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/ingame_face.toml", 0),
            ("Players/03 - A/oral.fmdl", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("ingame_face_explicit_face_model", Disposition::DropFolder)]
    );
    assert_eq!(
        report.issues[0].context,
        vec![("file", "oral.fmdl".to_owned())]
    );
    assert!(report.validated.unwrap().players.is_empty());
}

// TC-STR-14
#[test]
fn a_subfolder_s_files_are_typed_by_their_names_under_ingame_face_too() {
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/ingame_face.toml", 0),
            ("Players/03 - A/parts/boots.fmdl", 10),
            ("Players/03 - A/parts/glove_l.fmdl", 10),
            ("Players/03 - A/parts/skin.dds", 9),
            ("Players/04 - B/ingame_face.toml", 0),
            ("Players/04 - B/parts/hair_high.fmdl", 10),
            ("Players/05 - C/ingame_face.toml", 0),
            ("Players/05 - C/common/body.fmdl", 10),
            ("Players/05 - C/common/extra/readme.doc", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("ingame_face_explicit_face_model", Disposition::DropFolder),
            ("file_type_disallowed", Disposition::DropFolder),
        ]
    );
    // A face name below a subfolder is face content as it is in the root.
    assert_eq!(report.issues[0].scope, folder("Players/04 - B"));
    assert_eq!(
        report.issues[0].context,
        vec![("file", "hair_high.fmdl".to_owned())]
    );
    // `common/` is a plain subfolder: its model is admitted, a file of no known kind at any
    // depth not.
    assert_eq!(report.issues[1].scope, folder("Players/05 - C"));
    assert_eq!(
        report.issues[1].context,
        vec![("file", "common/extra/readme.doc".to_owned())]
    );
    // A's boots, glove and texture take their roles from their names: no finding.
    let validated = report.validated.unwrap();
    assert_eq!(
        validated
            .players
            .iter()
            .map(|player| player.path.as_str())
            .collect::<Vec<_>>(),
        vec!["Players/03 - A"]
    );
}

#[test]
fn boots_and_fcl_hair_models_are_no_explicit_face_content() {
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/ingame_face.toml", 0),
            ("Players/03 - A/kit_boots.fmdl", 10),
            ("Players/03 - A/x_fcl_hair.fmdl", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
    assert_eq!(report.validated.unwrap().players.len(), 1);
}

// TC-MOD-67
#[test]
fn a_subfolder_s_markers_and_settings_are_file_type_disallowed() {
    let report = report_with(
        &context_with(false, false),
        "egg Midcup",
        &[
            ("Players/03 - A/face_high.fmdl", 10),
            ("Players/03 - A/jessie/ingame_face.toml", 0),
            ("Players/03 - A/jessie/fpc_off", 0),
            ("Players/03 - A/jessie/settings.toml", 20),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("file_type_disallowed", Disposition::Keep); 3]
    );
    assert_eq!(
        issue_files(&report),
        [
            "jessie/fpc_off",
            "jessie/ingame_face.toml",
            "jessie/settings.toml",
        ]
    );
    // None of them counts: the player keeps his face, with no marker or settings.
    let player = &report.validated.unwrap().players[0];
    assert!(player.ingame_face.is_none());
    assert_eq!(player.fpc, None);
    assert_eq!(player.settings, None);
}

#[test]
fn a_referee_folder_with_category_subfolders_is_player_layout_proto_alone_a_team_s_valid() {
    let files = |folder: &str, subfolder: &str| {
        [
            (format!("Players/{folder}/face_high.model"), 10),
            (format!("Players/{folder}/{subfolder}/boots.model"), 10),
            // Out of place in either layout, but the one finding describes the folder.
            (format!("Players/{folder}/{subfolder}/notes.txt"), 10),
        ]
    };
    let refs_files: Vec<(String, u64)> = files("Ref A", "boots")
        .into_iter()
        .chain(files("Ref B", "Face"))
        .chain([("players.txt".to_owned(), 10)])
        .collect();
    let refs_files: Vec<(&str, u64)> = refs_files
        .iter()
        .map(|(path, size)| (path.as_str(), *size))
        .collect();
    let refs = report(
        "refs Cup",
        &refs_files,
        &[],
        &[("players.txt", Ok(b"01 Ref A\n02 Ref B\n"))],
    );
    assert_eq!(
        issue_codes(&refs),
        vec![("player_layout_proto", Disposition::DropFolder); 2]
    );
    assert_eq!(refs.issues[0].scope, folder("Players/Ref A"));
    assert_eq!(refs.issues[0].context, vec![("folder", "boots".to_owned())]);
    // Named as the export spells it.
    assert_eq!(refs.issues[1].scope, folder("Players/Ref B"));
    assert_eq!(refs.issues[1].context, vec![("folder", "Face".to_owned())]);
    assert!(refs.validated.unwrap().players.is_empty());

    // In a team export the names are plain subfolders: his `boots/boots.model` is his boots.
    let team_files: Vec<(String, u64)> = files("05 - A", "boots")[..2].to_vec();
    let team_files: Vec<(&str, u64)> = team_files
        .iter()
        .map(|(path, size)| (path.as_str(), *size))
        .collect();
    let team = report("egg Midcup", &team_files, &[], &[]);
    assert_eq!(issue_codes(&team), vec![]);
    assert_eq!(team.validated.unwrap().players.len(), 1);
}

#[test]
fn a_write_names_marker_in_a_player_folder_is_out_of_place() {
    // The finding a non-admitted marker gets, as `pre-fox` above.
    let dropped = report(
        "egg Midcup",
        &[
            ("Players/03 - A/write_names", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        dropped
            .issues
            .iter()
            .map(|issue| (issue.code, issue.disposition, issue.context.clone()))
            .collect::<Vec<_>>(),
        vec![(
            "file_type_disallowed",
            Disposition::DropFolder,
            vec![("file", "write_names".to_owned())]
        )]
    );

    // Under a non-strict check the folder keeps the file, which lands in
    // `files` as the other non-admitted markers do.
    let kept = report_with(
        &context_with(false, false),
        "egg Midcup",
        &[
            ("Players/03 - A/write_names", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    let files = &kept.validated.unwrap().players[0].files;
    assert!(files.iter().any(|file| file.path.name() == "write_names"));
}

#[test]
fn a_kept_disallowed_common_file_stays_in_the_players_files() {
    // A file below a subfolder the allowlist refuses keeps its finding (strict off), so it
    // stays in `files`.
    let report = report_with(
        &context_with(false, false),
        "egg Midcup",
        &[
            ("Common/hair.dds", 10),
            ("Players/03 - A/face_high.fmdl", 10),
            ("Players/03 - A/common/x.exe", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("file_type_disallowed", Disposition::Keep)]
    );
    let player = &report.validated.unwrap().players[0];
    assert!(
        player
            .files
            .iter()
            .any(|f| f.path.as_str() == "Players/03 - A/common/x.exe"),
        "{:?}",
        player
            .files
            .iter()
            .map(|f| f.path.as_str())
            .collect::<Vec<_>>()
    );
}
