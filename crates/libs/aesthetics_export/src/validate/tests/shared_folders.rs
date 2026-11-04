use super::*;

#[test]
fn shared_folders_and_common_files_build_unchecked() {
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/Longhair.face", 0),
            ("Players/03 - A/Crocs.boots", 0),
            ("Players/03 - A/Keeper gloves.gloves", 0),
            ("Players/03 - A/hair.dds", 9),
            ("Faces/Longhair/face_high.fmdl", 10),
            ("Boots/Crocs/boots.fmdl", 10),
            ("Gloves/Keeper gloves/glove_l.fmdl", 10),
            ("Common/hair.dds", 9),
            ("Collars/collar_101.fmdl", 10),
        ],
        &[],
        &[],
    );
    let validated = report.validated.unwrap();
    assert_eq!(validated.faces[0].folder_name, "Longhair");
    assert_eq!(
        validated.faces[0].files[0].path.as_str(),
        "Faces/Longhair/face_high.fmdl"
    );
    assert_eq!(validated.boots[0].folder_name, "Crocs");
    assert_eq!(validated.gloves[0].folder_name, "Keeper gloves");
    assert_eq!(validated.common[0].path.as_str(), "Common/hair.dds");
    assert_eq!(validated.common[0].kind, FileKind::Texture);
    assert_eq!(
        validated.collars[0].path.as_str(),
        "Collars/collar_101.fmdl"
    );
}

// TC-STR-17
#[test]
fn a_wrongly_named_boots_model_drops_folder_and_linker() {
    let report = report(
        "egg Midcup",
        &[
            ("Boots/Crocs/torso.fmdl", 10),
            ("Boots/Mud/kit_boots.fmdl", 10),
            ("Players/01 - A/Crocs.boots", 0),
            ("Players/01 - A/hair.dds", 9),
            ("Players/02 - B/Mud.boots", 0),
            ("Players/02 - B/hair.dds", 9),
            ("Players/03 - C/torso.fmdl", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("fmdl_name_invalid", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    assert_eq!(report.issues[0].scope, folder("Boots/Crocs"));
    assert_eq!(
        report.issues[0].context,
        vec![("file", "torso.fmdl".to_owned())]
    );
    assert_eq!(report.issues[1].scope, folder("Players/01 - A"));
    let validated = report.validated.unwrap();
    assert_eq!(
        validated
            .players
            .iter()
            .map(|player| player.path.as_str())
            .collect::<Vec<_>>(),
        vec!["Players/02 - B", "Players/03 - C"]
    );
    assert_eq!(
        validated
            .boots
            .iter()
            .map(|folder| folder.folder_name.as_str())
            .collect::<Vec<_>>(),
        vec!["Mud"]
    );
}

#[test]
fn wrong_suffixed_models_drop_their_shared_folders_and_linkers() {
    let report = report(
        "egg Midcup",
        &[
            ("Boots/X/glove_l.fmdl", 10),
            ("Gloves/Y/kit_boots.fmdl", 10),
            ("Players/01 - A/X.boots", 0),
            ("Players/01 - A/hair.dds", 9),
            ("Players/02 - B/Y.gloves", 0),
            ("Players/02 - B/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("fmdl_name_invalid", Disposition::DropFolder),
            ("fmdl_name_invalid", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    assert_eq!(report.issues[0].scope, folder("Boots/X"));
    assert_eq!(report.issues[1].scope, folder("Gloves/Y"));
    let validated = report.validated.unwrap();
    assert!(validated.players.is_empty());
    assert!(validated.boots.is_empty());
    assert!(validated.gloves.is_empty());
}

#[test]
fn a_gloves_folder_admits_every_gloves_suffix() {
    // `glove_l`, `glove_r` and the hand-skeleton `handL`/`handR` are all
    // gloves models.
    let report = report(
        "egg Midcup",
        &[
            ("Gloves/Keeper gloves/glove_l.fmdl", 10),
            ("Gloves/Keeper gloves/glove_r.fmdl", 10),
            ("Gloves/Keeper gloves/handL.fmdl", 10),
            ("Gloves/Keeper gloves/handR.fmdl", 10),
            ("Players/03 - A/Keeper gloves.gloves", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![], "{:?}", report.issues);
    let gloves = &report.validated.unwrap().gloves[0];
    assert_eq!(gloves.files.len(), 4);
}

#[test]
fn a_shared_folder_with_different_kinds_of_one_stem_is_fine() {
    let report = report(
        "egg Midcup",
        &[
            ("Faces/Base/hair.dds", 9),
            ("Faces/Base/hair.xml", 10),
            ("Players/03 - A/Base.face", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
    let validated = report.validated.unwrap();
    assert_eq!(validated.players.len(), 1);
    assert_eq!(validated.faces.len(), 1);
}

#[test]
fn a_shared_folder_s_subfolders_follow_a_player_folder_s_rules() {
    // A boots model says so by suffix at any depth: `extra/x_boots.fmdl` is content.
    let admitted = report(
        "egg Midcup",
        &[
            ("Boots/Crocs/boots.fmdl", 10),
            ("Boots/Crocs/extra/x_boots.fmdl", 10),
            ("Players/03 - A/Crocs.boots", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&admitted), vec![], "{:?}", admitted.issues);
    let validated = admitted.validated.unwrap();
    let files: Vec<&str> = validated.boots[0]
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    assert_eq!(
        files,
        ["Boots/Crocs/boots.fmdl", "Boots/Crocs/extra/x_boots.fmdl"]
    );

    // What a subfolder may not hold stays `file_type_disallowed`: a per-player singleton
    // (`face.xml`), a marker, a shared link — while a `.common` link is admitted and
    // checked, `common_link_missing` here, its `Common/` target absent.
    let disallowed = report(
        "egg Midcup",
        &[
            ("Faces/Round/face_high.fmdl", 10),
            ("Faces/Round/jessie/face.xml", 10),
            ("Faces/Round/jessie/ingame_face.toml", 0),
            ("Faces/Round/jessie/Crocs.boots", 0),
            ("Faces/Round/jessie/skin.dds.common", 0),
            ("Players/03 - A/Round.face", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&disallowed),
        vec![
            ("file_type_disallowed", Disposition::DropFolder),
            ("file_type_disallowed", Disposition::DropFolder),
            ("file_type_disallowed", Disposition::DropFolder),
            ("common_link_missing", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    let named: Vec<&str> = disallowed
        .issues
        .iter()
        .take(3)
        .map(|issue| issue.context[0].1.as_str())
        .collect();
    assert_eq!(
        named,
        [
            "jessie/Crocs.boots",
            "jessie/face.xml",
            "jessie/ingame_face.toml",
        ]
    );
    assert_eq!(
        disallowed.issues[3].context,
        vec![
            ("link", "skin.dds.common".to_owned()),
            ("path", "Common/jessie/skin.dds".to_owned()),
        ]
    );
    assert_eq!(disallowed.issues[0].scope, folder("Faces/Round"));
}

#[test]
fn a_shared_folder_s_common_links_are_admitted_at_any_depth() {
    // As a player folder's, a `.common` link stands for the `Common/` file at its own path
    // below the folder, at any depth.
    let report = report(
        "egg Midcup",
        &[
            ("Common/skin.dds", 9),
            ("Common/jessie/hair.dds", 9),
            ("Faces/Round/face_high.fmdl", 10),
            ("Faces/Round/skin.dds.common", 0),
            ("Faces/Round/jessie/hair.dds.common", 0),
            ("Players/03 - A/Round.face", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![], "{:?}", report.issues);
    let faces = &report.validated.unwrap().faces;
    assert_eq!(faces.len(), 1);
    assert_eq!(
        faces[0]
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        [
            "Faces/Round/face_high.fmdl",
            "Faces/Round/jessie/hair.dds.common",
            "Faces/Round/skin.dds.common",
        ]
    );
}

#[test]
fn a_shared_folder_s_shared_link_stays_file_type_disallowed() {
    // A shared folder holds no shared link: a `.face` file there is disallowed, as a marker
    // or a settings file is.
    let report = report(
        "egg Midcup",
        &[
            ("Boots/Crocs/boots.fmdl", 10),
            ("Boots/Crocs/Round.face", 0),
            ("Players/03 - A/Crocs.boots", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("file_type_disallowed", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    assert_eq!(report.issues[0].scope, folder("Boots/Crocs"));
    assert_eq!(
        report.issues[0].context,
        vec![("file", "Round.face".to_owned())]
    );
}

#[test]
fn a_shared_folder_s_common_link_whose_target_is_missing_drops_it_and_its_linker() {
    // `common_link_missing` on the shared folder, which the players linking it follow in
    // the same pass (`link_target_dropped`).
    let files = [
        ("Faces/Round/face_high.fmdl", 10),
        ("Faces/Round/hair.dds.common", 0),
        ("Players/03 - A/Round.face", 0),
        ("Players/03 - A/hair.dds", 9),
    ];
    let report = report("egg Midcup", &files, &[], &[]);
    assert_eq!(
        issue_codes(&report),
        vec![
            ("common_link_missing", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    assert_eq!(report.issues[0].scope, folder("Faces/Round"));
    assert_eq!(
        report.issues[0].context,
        vec![
            ("link", "hair.dds.common".to_owned()),
            ("path", "Common/hair.dds".to_owned()),
        ]
    );
    assert_eq!(report.issues[1].scope, folder("Players/03 - A"));
    assert_eq!(
        report.issues[1].context,
        vec![
            ("link", "Round.face".to_owned()),
            ("target", "Faces/Round".to_owned()),
            ("finding", "common_link_missing".to_owned()),
        ]
    );

    // On pre-Fox a shared folder's links stay unread, so the file just sits there: no
    // `common_link_missing`, the folder and its linker kept.
    let prefox = report_with(
        &context_for(pes_version::PesVersion::Pes17),
        "egg Midcup",
        &files,
        &[],
        &[],
    );
    assert_eq!(issue_codes(&prefox), vec![], "{:?}", prefox.issues);
    let faces = &prefox.validated.unwrap().faces;
    assert_eq!(faces.len(), 1);
}

#[test]
fn a_shared_folder_s_texture_link_claims_its_stem_on_fox_not_on_pre_fox() {
    // A `skin.dds` beside a `skin.dds.common`: on PES 18-21 the link claims `skin` as a
    // player's does and they collide; on PES 15-17 it stays unread and claims nothing.
    let files = [
        ("Common/skin.dds", 9),
        ("Boots/Crocs/boots.model", 10),
        ("Boots/Crocs/skin.dds", 9),
        ("Boots/Crocs/skin.dds.common", 0),
        ("Players/03 - A/Crocs.boots", 0),
        ("Players/03 - A/hair.dds", 9),
    ];
    let prefox = report_with(
        &context_for(pes_version::PesVersion::Pes17),
        "egg Midcup",
        &files,
        &[],
        &[],
    );
    assert_eq!(issue_codes(&prefox), vec![], "{:?}", prefox.issues);
    assert_eq!(prefox.validated.unwrap().boots.len(), 1);
    // Its textures still claim their stems there: a `skin.png` beside the `skin.dds` collides.
    let with_png: Vec<(&str, u64)> = files
        .iter()
        .copied()
        .chain([("Boots/Crocs/skin.png", 9)])
        .collect();
    let prefox = report_with(
        &context_for(pes_version::PesVersion::Pes17),
        "egg Midcup",
        &with_png,
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&prefox),
        vec![
            ("texture_stem_conflict", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ],
        "{:?}",
        prefox.issues
    );
    assert_eq!(
        prefox.issues[0].context,
        vec![
            ("stem", "skin".to_owned()),
            ("files", "skin.dds,skin.png".to_owned()),
        ]
    );

    let fox = report("egg Midcup", &files, &[], &[]);
    assert_eq!(
        issue_codes(&fox),
        vec![
            ("texture_stem_conflict", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    assert_eq!(fox.issues[0].scope, folder("Boots/Crocs"));
    assert_eq!(
        fox.issues[0].context,
        vec![
            ("stem", "skin".to_owned()),
            ("files", "skin.dds,skin.dds.common".to_owned()),
        ]
    );
}

#[test]
fn a_kept_shared_folder_s_link_whose_target_is_absent_comes_off_its_files() {
    // Under `pass_through` the folder keeps its other files, but a link naming nothing
    // comes off them, as a kept player's does.
    let report = report_with(
        &context_with(true, true),
        "egg Midcup",
        &[
            ("Boots/Crocs/boots.fmdl", 10),
            ("Boots/Crocs/spare_boots.fmdl.common", 0),
            ("Players/03 - A/Crocs.boots", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("common_link_missing", Disposition::Keep)]
    );
    let boots = &report.validated.unwrap().boots;
    assert_eq!(boots.len(), 1);
    assert_eq!(
        boots[0]
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        ["Boots/Crocs/boots.fmdl"]
    );
}

#[test]
fn a_shared_folder_s_texture_stems_collide_per_directory_of_its_tree() {
    // The lookup namespace of `texture_stem_conflict` is one directory, as a player's is:
    // a `skin.dds` in each of `a/` and `b/` is no conflict.
    let apart = report(
        "egg Midcup",
        &[
            ("Faces/Round/a/skin.dds", 9),
            ("Faces/Round/b/skin.dds", 9),
            ("Players/03 - A/Round.face", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&apart), vec![], "{:?}", apart.issues);

    // Two of one stem in one directory collide.
    let same = report(
        "egg Midcup",
        &[
            ("Faces/Round/a/skin.dds", 9),
            ("Faces/Round/a/skin.png", 9),
            ("Players/03 - A/Round.face", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&same),
        vec![
            ("texture_stem_conflict", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    assert_eq!(same.issues[0].scope, folder("Faces/Round"));
    assert_eq!(
        same.issues[0].context,
        vec![
            ("stem", "skin".to_owned()),
            ("files", "a/skin.dds,a/skin.png".to_owned()),
        ]
    );
}

#[test]
fn a_shared_folder_s_name_rule_applies_at_any_depth_naming_the_path_below() {
    // As a model directly in the folder: `fmdl_name_invalid` on a Fox target, the file named
    // below the folder.
    let report = report_with(
        &context_for(pes_version::PesVersion::Pes21),
        "egg Midcup",
        &[
            ("Boots/Crocs/extra/face_high.fmdl", 10),
            ("Boots/Crocs/extra/x.fmdl", 10),
            ("Players/03 - A/Crocs.boots", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("fmdl_name_invalid", Disposition::DropFolder),
            ("fmdl_name_invalid", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    let named: Vec<&str> = report
        .issues
        .iter()
        .take(2)
        .map(|issue| issue.context[0].1.as_str())
        .collect();
    assert_eq!(named, ["extra/face_high.fmdl", "extra/x.fmdl"]);
}

#[test]
fn a_shared_boots_folder_s_model_link_is_named_by_the_same_suffix_rule() {
    // A `.common` link stands for the file at its name, so where the links count
    // (PES 18-21) a link to a model is named as the model is: `glove_l` is no
    // boots name, and the finding names the link below the folder.
    let report = report_with(
        &context_for(pes_version::PesVersion::Pes21),
        "egg Midcup",
        &[
            ("Boots/Crocs/boots.fmdl", 10),
            ("Boots/Crocs/glove_l.fmdl.common", 0),
            ("Common/glove_l.fmdl", 10),
            ("Players/03 - A/Crocs.boots", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("fmdl_name_invalid", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    assert_eq!(report.issues[0].scope, folder("Boots/Crocs"));
    assert_eq!(
        report.issues[0].context,
        vec![("file", "glove_l.fmdl.common".to_owned())]
    );
    assert!(report.validated.unwrap().boots.is_empty());

    // A link to a boots model and a texture link pass the rule.
    let report = report_with(
        &context_for(pes_version::PesVersion::Pes21),
        "egg Midcup",
        &[
            ("Boots/Crocs/boots.fmdl.common", 0),
            ("Boots/Crocs/skin.dds.common", 0),
            ("Common/boots.fmdl", 10),
            ("Common/skin.dds", 9),
            ("Players/03 - A/Crocs.boots", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);

    // On PES 15-17 a shared folder's links stay unread (`file_not_used` is the
    // consumer's): the same link is no `model_name_invalid`.
    let report = report_with(
        &context_for(pes_version::PesVersion::Pes17),
        "egg Midcup",
        &[
            ("Boots/Crocs/glove_l.fmdl.common", 0),
            ("Common/glove_l.fmdl", 10),
            ("Players/03 - A/Crocs.boots", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
}
