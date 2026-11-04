use super::*;

#[test]
fn a_pre_fox_shared_boots_model_without_its_suffix_is_model_name_invalid() {
    let unsuffixed = report_for(
        pes_version::PesVersion::Pes17,
        &[
            ("Boots/Mud/hat.model", 10),
            ("Players/03 - A/Mud.boots", 0),
            ("Players/03 - A/hair.dds", 9),
        ],
    );
    assert_eq!(
        issue_codes(&unsuffixed),
        vec![
            ("model_name_invalid", Disposition::DropFolder),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    assert_eq!(unsuffixed.issues[0].scope, folder("Boots/Mud"));
    assert_eq!(
        unsuffixed.issues[0].context,
        vec![("file", "hat.model".to_owned())]
    );
    assert_eq!(unsuffixed.issues[1].scope, folder("Players/03 - A"));
    assert!(unsuffixed.validated.unwrap().boots.is_empty());

    let suffixed = report_for(
        pes_version::PesVersion::Pes17,
        &[
            ("Boots/Mud/kit_boots.model", 10),
            ("Players/03 - A/Mud.boots", 0),
        ],
    );
    assert_eq!(issue_codes(&suffixed), vec![]);
    assert_eq!(suffixed.validated.unwrap().boots.len(), 1);
}

#[test]
fn an_edit_hair_file_drops_its_folder_on_pre_fox_only() {
    let files = [
        ("Players/05 - A/face_edithair.xml", 10),
        ("Players/05 - A/face/hair.xml", 10),
        ("Players/05 - A/face_high.model", 10),
        ("Players/07 - B/Longhair.face", 0),
        ("Players/07 - B/hair.dds", 9),
        ("Faces/Longhair/HAIR.XML", 10),
        ("Faces/Longhair/hair_high.model", 10),
    ];
    let pre_fox = report_for(pes_version::PesVersion::Pes17, &files);
    let edithair = |scope: &str, file: &str| ValidationIssue {
        code: "edithair_unsupported",
        scope: folder(scope),
        context: vec![("file", file.to_owned())],
        disposition: Disposition::DropFolder,
        passed_through: false,
    };
    assert_eq!(
        pre_fox.issues[..3],
        // In the folder's file order: `/` sorts before `_`.
        [
            edithair("Players/05 - A", "face/hair.xml"),
            edithair("Players/05 - A", "face_edithair.xml"),
            edithair("Faces/Longhair", "HAIR.XML"),
        ]
    );
    assert_eq!(
        issue_codes(&pre_fox)[3..],
        [("link_target_dropped", Disposition::DropFolder)]
    );
    assert_eq!(pre_fox.issues[3].scope, folder("Players/07 - B"));
    let validated = pre_fox.validated.unwrap();
    assert!(validated.players.is_empty());
    assert!(validated.faces.is_empty());

    let fox = report_for(pes_version::PesVersion::Pes21, &files);
    assert_eq!(issue_codes(&fox), vec![]);
    let validated = fox.validated.unwrap();
    assert_eq!(validated.players.len(), 2);
    assert_eq!(validated.faces.len(), 1);
}
