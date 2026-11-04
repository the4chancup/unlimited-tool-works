use super::*;

#[test]
fn a_stem_ending_in_space_folds_without_panicking() {
    // `skin .png`'s stem is `skin ` — no valid path segment, still a name.
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/skin .png", 9),
            ("Players/03 - A/face_high.fmdl", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&report), vec![]);
    assert_eq!(report.validated.unwrap().players.len(), 1);
}

#[test]
fn a_link_name_ending_in_space_misses_its_target() {
    // `Crocs .boots` names `Crocs ` — `Boots/Crocs` exists but does not
    // match; the name folds rather than panicking.
    let report = report(
        "egg Midcup",
        &[
            ("Players/03 - A/Crocs .boots", 0),
            ("Boots/Crocs/boots.fmdl", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("link_target_missing", Disposition::DropFolder),
            ("shared_folder_orphaned", Disposition::DropFolder),
        ]
    );
}

#[test]
fn a_flattened_folder_spelling_collision_is_a_conflict() {
    // `Docs/b.txt` flattens onto `docs/`'s spelling — rejected, no panic.
    let report = report(
        "egg Midcup",
        &[
            ("docs/a.txt", 4),
            ("wrapper/Docs/b.txt", 4),
            ("wrapper/Players/03 - A/face_high.fmdl", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("nested_root_conflict", Disposition::DropExport)]
    );
    assert!(report.validated.is_none());
}

#[test]
fn a_texture_common_link_joins_the_stem_namespace() {
    let linked = report(
        "egg Midcup",
        &[
            ("Common/hair.png", 9),
            ("Players/03 - A/hair.dds", 9),
            ("Players/03 - A/hair.png.common", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&linked),
        vec![("texture_stem_conflict", Disposition::DropFolder)]
    );
    assert_eq!(linked.issues[0].scope, folder("Players/03 - A"));
    assert!(linked.validated.unwrap().players.is_empty());

    // A model link is not a texture: same stem, no conflict.
    let model_link = report(
        "egg Midcup",
        &[
            ("Common/torso.fmdl", 10),
            ("Players/03 - A/torso.dds", 9),
            ("Players/03 - A/torso.fmdl.common", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&model_link), vec![]);
}

#[test]
fn common_textures_conflict_each_other_and_drop_the_linker() {
    let report = report(
        "egg Midcup",
        &[
            ("Common/hair.dds", 9),
            ("Common/hair.png", 9),
            ("Players/03 - A/hair.png.common", 0),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("texture_stem_conflict", Disposition::DropFile),
            ("texture_stem_conflict", Disposition::DropFile),
            ("link_target_dropped", Disposition::DropFolder),
        ]
    );
    assert_eq!(report.issues[0].scope, file_scope("Common/hair.dds"));
    assert_eq!(report.issues[1].scope, file_scope("Common/hair.png"));
    assert_eq!(report.issues[2].scope, folder("Players/03 - A"));
    let validated = report.validated.unwrap();
    assert!(validated.common.is_empty());
    assert!(validated.players.is_empty());
}

#[test]
fn common_textures_of_one_stem_conflict_per_common_directory() {
    // `Common/`'s stem namespace is per directory, each packing at its own path: two
    // textures of one stem in `jessie/` collide at one CPK path, as two direct ones do.
    let same = report(
        "egg Midcup",
        &[("Common/jessie/hair.dds", 9), ("Common/jessie/hair.png", 9)],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&same),
        vec![
            ("texture_stem_conflict", Disposition::DropFile),
            ("texture_stem_conflict", Disposition::DropFile),
        ]
    );
    assert_eq!(same.issues[0].scope, file_scope("Common/jessie/hair.dds"));
    assert_eq!(same.issues[1].scope, file_scope("Common/jessie/hair.png"));
    assert!(same.validated.unwrap().common.is_empty());
    // In different directories of `Common/` the stem is no collision.
    let apart = report(
        "egg Midcup",
        &[("Common/jessie/hair.dds", 9), ("Common/other/hair.png", 9)],
        &[],
        &[],
    );
    assert_eq!(issue_codes(&apart), vec![]);
}

#[test]
fn two_collisions_on_one_loose_path_report_once() {
    // Both flattened names claim `docs/`'s spelling: one finding.
    let report = report(
        "egg Midcup",
        &[
            ("docs/a.txt", 4),
            ("wrapper/Docs/b.txt", 4),
            ("wrapper/Docs/c.txt", 4),
            ("wrapper/Players/03 - A/face_high.fmdl", 10),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![("nested_root_conflict", Disposition::DropExport)]
    );
    assert!(report.validated.is_none());
}
