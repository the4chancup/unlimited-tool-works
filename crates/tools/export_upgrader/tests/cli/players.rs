//! The face folders' migration into `Players/` (the `spec/export_upgrader.md` "Player folders"
//! scenarios): the folder naming order, portraits, `ingame_face` markers, the
//! empty roster slot and the doubled `Faces/Faces/` flattening.

use std::fs;

use pes_savefile::EditFile;
use pes_savefile::settings_toml::{IngameFaceSettings, PlayerSettings};
use studio_core::PipelineEvent;

use crate::carried::mtl;
use crate::common::Sandbox;

/// How many `UnmigratedContentFound` events a run emitted.
pub(crate) fn unmigrated_found(run: &crate::common::Run) -> usize {
    run.events
        .iter()
        .filter(|envelope| matches!(envelope.event, PipelineEvent::UnmigratedContentFound))
        .count()
}

/// `upgrade <input> --savefile <save>`, `flags` appended.
pub(crate) fn upgrade_with(
    sandbox: &Sandbox,
    input: &str,
    save: &str,
    flags: &[&str],
) -> crate::common::Run {
    let input = sandbox.arg(input);
    let save = sandbox.arg(save);
    let mut args = vec!["upgrade", &input, "--savefile", &save];
    args.extend_from_slice(flags);
    sandbox.run(&args)
}

/// `upgrade <input> --savefile <save>`.
pub(crate) fn upgrade(sandbox: &Sandbox, input: &str, save: &str) -> crate::common::Run {
    upgrade_with(sandbox, input, save, &[])
}

/// The four files of the real tracer Fox face folder
/// (`team_compiler/tests/fixtures/tracer/old/egg Tracer/Faces/XXX05 - …/`),
/// a few bytes each (a test of names, not formats).
const FACE_FILES: [(&str, &[u8]); 4] = [
    ("fcl_hair.fmdl", b"hair"),
    ("face_diff.bin", b"diff"),
    ("shirt.dds", b"shirt"),
    ("portrait.dds", b"portrait"),
];

/// An old export's face folder at `relative` with the four face files.
pub(crate) fn face(sandbox: &Sandbox, relative: &str) {
    for (name, bytes) in FACE_FILES {
        sandbox.write(&format!("{relative}/{name}"), bytes);
    }
}

/// The three files of the real tracer Fox boots folder, a few bytes each;
/// `shirt.dds` carries the face's bytes so only a scenario's own files decide
/// a merge's byte clash.
const BOOTS_FILES: [(&str, &[u8]); 3] = [
    ("boots.fmdl", b"boots"),
    ("boots.skl", b"skl"),
    ("shirt.dds", b"shirt"),
];

/// An old export's boots folder at `relative`.
pub(crate) fn boots(sandbox: &Sandbox, relative: &str) {
    for (name, bytes) in BOOTS_FILES {
        sandbox.write(&format!("{relative}/{name}"), bytes);
    }
}

/// A gloves folder at `relative` (`glove_l.fmdl`, `glove_r.fmdl`).
pub(crate) fn gloves(sandbox: &Sandbox, relative: &str) {
    sandbox.write(&format!("{relative}/glove_l.fmdl"), b"glove_l");
    sandbox.write(&format!("{relative}/glove_r.fmdl"), b"glove_r");
}

/// Sets `id`'s `boots_id` and `gloves_id` on its appearance record.
pub(crate) fn wears(file: &mut pes_savefile::EditFile, id: u32, boots_id: u32, gloves_id: u32) {
    let player = file.player_mut(id).unwrap();
    player.appearance.boots_id = boots_id;
    player.appearance.gloves_id = gloves_id;
}

/// The `Players/` entry for slot `nn` under `output` (`NN` or
/// `NN - name`), whatever name the run gave it.
fn player_folder(sandbox: &Sandbox, output: &str, nn: &str) -> String {
    sandbox
        .listing(&format!("{output}/Players"))
        .into_iter()
        .find(|entry| {
            !entry.contains('/') && (*entry == nn || entry.starts_with(&format!("{nn} - ")))
        })
        .unwrap_or_else(|| panic!("no player folder for slot {nn}"))
}

// EU-PLY-02
#[test]
fn a_wearer_without_a_face_folder_gets_one_with_ingame_face() {
    let sandbox = Sandbox::new("ply_stray_boots");
    boots(&sandbox, "co Spring/Boots/k0430 - Crocs");
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71405, 430, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    for (name, bytes) in BOOTS_FILES {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Spring/Players/05 - Crocs/{name}"))).unwrap(),
            bytes,
            "{name}"
        );
    }
    // `ingame_face.toml`, filled from his record.
    let face = IngameFaceSettings::parse(
        &fs::read_to_string(sandbox.path("co Full Spring/Players/05 - Crocs/ingame_face.toml"))
            .unwrap(),
    )
    .unwrap();
    let save = EditFile::load(&sandbox.path("EDIT")).unwrap();
    assert_eq!(
        face,
        PlayerSettings::from_player(save.player(71405).unwrap())
            .unwrap()
            .ingame_face()
    );
    assert!(!sandbox.path("co Full Spring/Boots").exists());
}

// EU-SET-17
#[test]
fn a_shared_item_folder_merges_into_every_wearer_in_legacy_mode() {
    let sandbox = Sandbox::new("ply_legacy_ids");
    // EU-SET-15's export: slots 06 and 07 both wear g0431.
    let export = |name: &str| {
        face(&sandbox, &format!("{name}/Faces/XXX03 - A"));
        boots(&sandbox, &format!("{name}/Boots/k0430 - Crocs"));
        gloves(&sandbox, &format!("{name}/Gloves/g0431"));
        sandbox.write(&format!("{name}/Boots/k0440 - Empty/boots.skl"), b"skl");
    };
    export("co Spring");
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71405, 430, 0);
        wears(file, 71406, 0, 431);
        wears(file, 71407, 0, 431);
        wears(file, 71408, 55, 0);
        wears(file, 71409, 440, 0);
    });

    let run = upgrade_with(&sandbox, "co Spring", "EDIT", &["--legacy-ids"]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(run.stdout().contains("migration_clean"), "{}", run.stdout());
    for nn in ["06", "07"] {
        let folder = player_folder(&sandbox, "co Full Spring", nn);
        for (name, bytes) in [("glove_l.fmdl", b"glove_l"), ("glove_r.fmdl", b"glove_r")] {
            assert_eq!(
                fs::read(sandbox.path(&format!("co Full Spring/Players/{folder}/{name}"))).unwrap(),
                bytes,
                "{folder}/{name}"
            );
        }
    }
    // No shared gloves folder and no link file anywhere in the output.
    for entry in sandbox.listing("co Full Spring") {
        assert!(!entry.starts_with("Gloves"), "{entry}");
        assert!(!entry.ends_with(".gloves"), "{entry}");
    }

    // The same export, slot 06's face folder also holding a
    // `glove_l.fmdl` with other bytes: `legacy_merge_clash` is an Error
    // and nothing is written.
    export("co Summer");
    face(&sandbox, "co Summer/Faces/XXX06 - B");
    sandbox.write("co Summer/Faces/XXX06 - B/glove_l.fmdl", b"other");
    let run = upgrade_with(&sandbox, "co Summer", "EDIT", &["--legacy-ids"]);
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    assert!(
        run.stdout().contains(
            "- co Summer: Error legacy_merge_clash at Gloves/g0431 (path=glove_l.fmdl)\n"
        ),
        "{}",
        run.stdout()
    );
    sandbox.assert_nothing_written("co Full Summer", &run.stdout());

    // The same export without the flag: g0431 stays shared, a link per
    // wearer.
    export("co Autumn");
    let run = upgrade(&sandbox, "co Autumn", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    for (name, bytes) in [("glove_l.fmdl", b"glove_l"), ("glove_r.fmdl", b"glove_r")] {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Autumn/Gloves/g0431/{name}"))).unwrap(),
            bytes,
            "{name}"
        );
    }
    for nn in ["06", "07"] {
        let folder = player_folder(&sandbox, "co Full Autumn", nn);
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Autumn/Players/{folder}/g0431.gloves")))
                .unwrap(),
            b"",
            "{folder}/g0431.gloves"
        );
    }
}

// EU-PLY-05
#[test]
fn folders_worn_by_two_share_the_folder_with_links() {
    let sandbox = Sandbox::new("ply_shared");
    for slot in ["XXX03 - A", "XXX04 - B", "XXX05 - C", "XXX06 - D"] {
        face(&sandbox, &format!("co Spring/Faces/{slot}"));
    }
    boots(&sandbox, "co Spring/Boots/k0430 - Crocs");
    boots(&sandbox, "co Spring/Boots/k0431");
    gloves(&sandbox, "co Spring/Gloves/g0432 - Mitts");
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71403, 430, 432);
        wears(file, 71404, 431, 432);
        wears(file, 71405, 430, 0);
        wears(file, 71406, 431, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    for (name, bytes) in BOOTS_FILES {
        for folder in ["Boots/Crocs", "Boots/k0431"] {
            assert_eq!(
                fs::read(sandbox.path(&format!("co Full Spring/{folder}/{name}"))).unwrap(),
                bytes,
                "{folder}/{name}"
            );
        }
    }
    for name in ["glove_l.fmdl", "glove_r.fmdl"] {
        assert!(
            sandbox
                .path(&format!("co Full Spring/Gloves/Mitts/{name}"))
                .exists()
        );
    }
    for (player, links) in [
        ("03 - A", vec!["Crocs.boots", "Mitts.gloves"]),
        ("05 - C", vec!["Crocs.boots"]),
        ("04 - B", vec!["k0431.boots", "Mitts.gloves"]),
        ("06 - D", vec!["k0431.boots"]),
    ] {
        for link in links {
            assert_eq!(
                fs::read(sandbox.path(&format!("co Full Spring/Players/{player}/{link}"))).unwrap(),
                b"",
                "{player}/{link}"
            );
        }
        // No player folder holds a copy of a model of those folders.
        for file in ["boots.fmdl", "boots.skl", "glove_l.fmdl", "glove_r.fmdl"] {
            assert!(
                !sandbox
                    .path(&format!("co Full Spring/Players/{player}/{file}"))
                    .exists(),
                "{player}/{file}"
            );
        }
    }
}

// EU-PLY-06
#[test]
fn a_folder_no_save_record_wears_is_leftovers() {
    let sandbox = Sandbox::new("ply_unworn");
    face(&sandbox, "co Spring/Faces/XXX03 - A");
    gloves(&sandbox, "co Spring/Gloves/g0432 - Mitts");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let stdout = run.stdout();
    for file in ["glove_l.fmdl", "glove_r.fmdl"] {
        assert!(
            stdout.contains(&format!(
                "- co Spring: Warning leftover_dropped at Gloves/g0432 - Mitts/{file}\n"
            )),
            "{file}: {stdout}"
        );
    }
    // The output holds no copy of them, under any name.
    for entry in sandbox.listing("co Full Spring") {
        assert!(
            !entry.ends_with("glove_l.fmdl") && !entry.ends_with("glove_r.fmdl"),
            "{entry}"
        );
    }
    assert!(!stdout.contains("unmigrated_content"), "{stdout}");
}

// EU-PLY-08
#[test]
fn pre_fox_never_merges_the_alone_worn_folder() {
    let sandbox = Sandbox::new("ply_prefox");
    face(&sandbox, "co Spring/Faces/XXX05 - Bob");
    sandbox.write("co Spring/Faces/XXX05 - Bob/face.xml", b"<config/>");
    sandbox.write(
        "co Spring/Faces/XXX05 - Bob/face_high_win32.model",
        b"face_high",
    );
    boots(&sandbox, "co Spring/Boots/k0430 - Crocs");
    sandbox.write(
        "co Spring/Boots/k0430 - Crocs/oral_boots_win32.model",
        b"oral",
    );
    sandbox.write("co Spring/Boots/k0430 - Crocs/boots.mtl", &mtl(&[]));
    sandbox.write("co Spring/Boots/k0430 - Crocs/k2012_c.dds", b"tex");
    sandbox.save_edited("EDIT16", pes_version::PesVersion::Pes16, |file| {
        wears(file, 71405, 430, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT16");
    let report = run.stdout();
    assert_eq!(run.exit_code(), 0, "{report}");
    // The boots model loses its `oral_`/`_win32` wrappers (step 7).
    for (input, output) in [
        ("oral_boots_win32.model", "boots.model"),
        ("boots.mtl", "boots.mtl"),
        ("k2012_c.dds", "k2012_c.dds"),
    ] {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Spring/Boots/Crocs/{output}"))).unwrap(),
            fs::read(sandbox.path(&format!("co Spring/Boots/k0430 - Crocs/{input}"))).unwrap(),
            "{output}"
        );
    }
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/05 - Bob/Crocs.boots")).unwrap(),
        b""
    );
    for name in ["boots.model", "boots.mtl", "k2012_c.dds"] {
        assert!(
            !sandbox
                .path(&format!("co Full Spring/Players/05 - Bob/{name}"))
                .exists(),
            "{name}"
        );
    }
}

// EU-PLY-09
#[test]
fn a_byte_clash_keeps_the_folder_shared_and_their_own_file() {
    let sandbox = Sandbox::new("ply_clash");
    face(&sandbox, "co Spring/Faces/XXX05 - A");
    sandbox.write("co Spring/Faces/XXX05 - A/skin.dds", b"mine");
    face(&sandbox, "co Spring/Faces/XXX06 - B");
    sandbox.write("co Spring/Faces/XXX06 - B/skin.dds", b"shared");
    boots(&sandbox, "co Spring/Boots/k0430 - Crocs");
    sandbox.write("co Spring/Boots/k0430 - Crocs/skin.dds", b"theirs");
    boots(&sandbox, "co Spring/Boots/k0431");
    sandbox.write("co Spring/Boots/k0431/skin.dds", b"shared");
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71405, 430, 0);
        wears(file, 71406, 431, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        run.stdout()
            .contains("- co Spring: Info merge_kept_shared at Boots/k0430 - Crocs (path=skin.dds)"),
        "{}",
        run.stdout()
    );
    // The shared folder holds its own files; 05 keeps his face's skin.dds.
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Boots/Crocs/skin.dds")).unwrap(),
        b"theirs"
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Boots/Crocs/boots.fmdl")).unwrap(),
        b"boots"
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/05 - A/skin.dds")).unwrap(),
        b"mine"
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/05 - A/Crocs.boots")).unwrap(),
        b""
    );
    // 06 merges: one skin.dds, the face's bytes being equal to the item's.
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/06 - B/boots.fmdl")).unwrap(),
        b"boots"
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/06 - B/skin.dds")).unwrap(),
        b"shared"
    );
    assert!(!sandbox.path("co Full Spring/Boots/k0431").exists());
}

// EU-PLY-10, EU-REP-05
#[test]
fn second_folders_of_a_slot_and_of_an_id_are_unmigrated() {
    let sandbox = Sandbox::new("ply_second");
    face(&sandbox, "co Spring/Faces/XXX23 - Bahsaka");
    face(&sandbox, "co Spring/Faces/XXX23 - Smug Redman");
    boots(&sandbox, "co Spring/Boots/k0430 - A");
    boots(&sandbox, "co Spring/Boots/k0430 - B");
    // A face folder whose slot reads nothing and a boots folder under a
    // name the item grammar refuses, each unmigrated as a whole — here
    // empty, which only the folder listing can still see.
    fs::create_dir_all(sandbox.path("co Spring/Faces/XXX24 - Ann")).unwrap();
    fs::create_dir_all(sandbox.path("co Spring/Boots/K0431 - Rex")).unwrap();
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71423, 430, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 1);
    let stdout = run.stdout();
    for (folder, context) in [
        (
            "Faces/XXX23 - Smug Redman",
            "(reason=slot_taken, other=Faces/XXX23 - Bahsaka)",
        ),
        (
            "Boots/k0430 - B",
            "(reason=id_taken, other=Boots/k0430 - A)",
        ),
        ("Faces/XXX24 - Ann", "(reason=slot_invalid)"),
        ("Boots/K0431 - Rex", "(reason=id_invalid)"),
    ] {
        assert!(
            stdout.contains(&format!("unmigrated_content at {folder} {context}")),
            "{folder}: {stdout}"
        );
    }
    sandbox.assert_nothing_written("co Full Spring", &stdout);
}

// EU-PLY-12
#[test]
fn identical_shared_folders_merge_as_one() {
    let sandbox = Sandbox::new("ply_identical_shared");
    for slot in [
        "XXX03 - A",
        "XXX04 - B",
        "XXX05 - C",
        "XXX06 - D",
        "XXX07 - E",
        "XXX08 - F",
    ] {
        face(&sandbox, &format!("co Spring/Faces/{slot}"));
    }
    boots(&sandbox, "co Spring/Boots/k0430 - Crocs");
    boots(&sandbox, "co Spring/Boots/k0431 - Crocs");
    boots(&sandbox, "co Spring/Boots/k0432 - crocs");
    // `k0432 - crocs` differs from the two identical ones in its model alone.
    sandbox.write("co Spring/Boots/k0432 - crocs/boots.fmdl", b"other model");
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71403, 430, 0);
        wears(file, 71404, 431, 0);
        wears(file, 71405, 430, 0);
        wears(file, 71406, 431, 0);
        wears(file, 71407, 432, 0);
        wears(file, 71408, 432, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    for (name, bytes) in BOOTS_FILES {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Spring/Boots/Crocs/{name}"))).unwrap(),
            bytes,
            "{name}"
        );
    }
    assert!(!sandbox.path("co Full Spring/Boots/k0431").exists());
    for (name, bytes) in [
        ("boots.fmdl", b"other model" as &[u8]),
        ("boots.skl", b"skl"),
        ("shirt.dds", b"shirt"),
    ] {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Spring/Boots/k0432/{name}"))).unwrap(),
            bytes,
            "{name}"
        );
    }
    for (player, link) in [
        ("03 - A", "Crocs.boots"),
        ("04 - B", "Crocs.boots"),
        ("05 - C", "Crocs.boots"),
        ("06 - D", "Crocs.boots"),
        ("07 - E", "k0432.boots"),
        ("08 - F", "k0432.boots"),
    ] {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Spring/Players/{player}/{link}"))).unwrap(),
            b"",
            "{player}/{link}"
        );
    }
}

// EU-PLY-01
#[test]
fn the_face_folders_become_the_players_folders() {
    let sandbox = Sandbox::new("ply_folders");
    face(&sandbox, "co Spring/Faces/XXX15 - Snuffy");
    face(&sandbox, "co Spring/Faces/71416 Rex");
    face(&sandbox, "co Spring/Faces/XXX17- Ann");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    for player in [
        "Players/15 - Snuffy",
        "Players/16 - Rex",
        "Players/17 - Ann",
    ] {
        for (name, bytes) in FACE_FILES {
            assert_eq!(
                fs::read(sandbox.path(&format!("co Full Spring/{player}/{name}"))).unwrap(),
                bytes,
                "{player}/{name}"
            );
        }
    }
    assert!(
        !run.stdout().contains("unmigrated_content"),
        "{}",
        run.stdout()
    );
}

// EU-PLY-01
#[test]
fn a_doubled_faces_layer_is_flattened() {
    let sandbox = Sandbox::new("ply_nested_faces");
    face(&sandbox, "co Spring/Faces/Faces/XXX15 - Snuffy");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    for (name, _) in FACE_FILES {
        assert!(
            sandbox
                .path(&format!("co Full Spring/Players/15 - Snuffy/{name}"))
                .exists(),
            "{name}: {}",
            run.stdout()
        );
    }
}

// EU-PLY-01
#[test]
fn a_doubled_layer_in_another_letter_case_is_flattened_in_a_zip() {
    let sandbox = Sandbox::new("ply_nested_faces_zip");
    // A `.zip` holding `co Spring/Faces/faces/XXX15 - Snuffy/`, the doubled
    // layer spelled in another letter case.
    let options = zip::write::SimpleFileOptions::default();
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer
        .add_directory("co Spring/Faces/faces/", options)
        .unwrap();
    for (name, bytes) in FACE_FILES {
        writer
            .start_file(
                format!("co Spring/Faces/faces/XXX15 - Snuffy/{name}"),
                options,
            )
            .unwrap();
        std::io::Write::write_all(&mut writer, bytes).unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();
    sandbox.write("co Spring.zip", &bytes);
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring.zip", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    for (name, bytes) in FACE_FILES {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Spring/Players/15 - Snuffy/{name}"))).unwrap(),
            bytes,
            "{name}: {}",
            run.stdout()
        );
    }
}

// EU-PLY-07
#[test]
fn a_named_portrait_is_renamed_to_portrait_dds() {
    let sandbox = Sandbox::new("ply_named_portrait");
    sandbox.write(
        "co Spring/Faces/XXX15 - Snuffy/Snuffy_Portrait.dds",
        b"portrait1",
    );
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/15 - Snuffy/portrait.dds")).unwrap(),
        b"portrait1"
    );
}

// EU-PLY-14
#[test]
fn two_portrait_files_are_unmigrated_content() {
    let sandbox = Sandbox::new("ply_two_portraits");
    sandbox.write("co Spring/Faces/XXX16 - Rex/portrait.dds", b"portrait");
    sandbox.write("co Spring/Faces/XXX16 - Rex/rex_portrait.dds", b"portrait2");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 1);
    let stdout = run.stdout();
    assert!(
        stdout.contains(
            "unmigrated_content at Faces/XXX16 - Rex/portrait.dds (reason=portrait_taken, other=Faces/XXX16 - Rex/rex_portrait.dds)"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "unmigrated_content at Faces/XXX16 - Rex/rex_portrait.dds (reason=portrait_taken, other=Faces/XXX16 - Rex/portrait.dds)"
        ),
        "{stdout}"
    );
    assert_eq!(unmigrated_found(&run), 1);
    sandbox.assert_nothing_written("co Full Spring", &stdout);
}

// EU-PLY-03
#[test]
fn a_nameless_folder_takes_the_saves_display_name_cleaned() {
    let sandbox = Sandbox::new("ply_save_name");
    face(&sandbox, "co Spring/Faces/XXX09");
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        // The colour-coded name of fixture player 78205: the `\x11c` code
        // strips, the `>` is not a folder-name character.
        file.player_mut(71409).unwrap().name = "\u{11}c789922ff>skin".to_owned();
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        sandbox.path("co Full Spring/Players/09 - skin").is_dir(),
        "{}",
        run.stdout()
    );
}

// EU-PLY-22
#[test]
fn either_ingame_face_spelling_becomes_ingame_face_toml_filled() {
    let sandbox = Sandbox::new("ply_ingame");
    face(&sandbox, "co Spring/Faces/XXX03 - A");
    sandbox.write("co Spring/Faces/XXX03 - A/ingame_face", b"");
    face(&sandbox, "co Spring/Faces/XXX04 - B");
    sandbox.write("co Spring/Faces/XXX04 - B/ingame_face.txt", b"ignored");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let save = EditFile::load(&sandbox.path("EDIT")).unwrap();
    for (player, id) in [("Players/03 - A", 71403), ("Players/04 - B", 71404)] {
        // Each file holds its record's face, hair and iris; its
        // `settings.toml` holds none of them; and no old spelling is left.
        let face = IngameFaceSettings::parse(
            &fs::read_to_string(sandbox.path(&format!("co Full Spring/{player}/ingame_face.toml")))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            face,
            PlayerSettings::from_player(save.player(id).unwrap())
                .unwrap()
                .ingame_face(),
            "{player}"
        );
        let settings = PlayerSettings::parse(
            &fs::read_to_string(sandbox.path(&format!("co Full Spring/{player}/settings.toml")))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            settings.ingame_face(),
            IngameFaceSettings::default(),
            "{player}: settings.toml holds no face key"
        );
        for name in ["ingame_face", "ingame_face.txt"] {
            assert!(
                !sandbox
                    .path(&format!("co Full Spring/{player}/{name}"))
                    .exists(),
                "{player}/{name}"
            );
        }
    }
    // EU-PLY-22's referee half: `referees.rs`'
    // `either_ingame_face_spelling_in_a_referee_folder_becomes_ingame_face_toml`.
}

// EU-PLY-11
#[test]
fn an_empty_roster_slot_is_a_warning_and_the_folder_still_forms() {
    let sandbox = Sandbox::new("ply_empty_slot");
    face(&sandbox, "co Spring/Faces/XXX23 - Pinto");
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        let team = file.team_mut(714).unwrap();
        let position = team
            .roster
            .iter()
            .position(|slot| slot.player_id == 71423)
            .unwrap();
        team.roster[position].player_id = 0;
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        run.stdout()
            .contains("- co Spring: Warning roster_slot_empty at Faces/XXX23 - Pinto (slot=23)"),
        "{}",
        run.stdout()
    );
    for (name, _) in FACE_FILES {
        assert!(
            sandbox
                .path(&format!("co Full Spring/Players/23 - Pinto/{name}"))
                .exists(),
            "{name}"
        );
    }
    assert!(
        !sandbox
            .path("co Full Spring/Players/23 - Pinto/settings.toml")
            .exists()
    );
    // `roster_slot_empty` is a warning, not unmigrated content: no event.
    assert_eq!(unmigrated_found(&run), 0);
}

// EU-PLY-13
#[test]
fn a_fox_hair_skeleton_is_renamed_to_pair_with_its_model() {
    let sandbox = Sandbox::new("ply_hair_skl");
    sandbox.write("co Spring/Faces/XXX05 - Bob/fcl_hair.fmdl", b"hair model");
    sandbox.write(
        "co Spring/Faces/XXX05 - Bob/fcl_hair_sim.skl",
        b"skeleton of Bob",
    );
    sandbox.write(
        "co Spring/Faces/XXX06 - Ann/fcl_hair_sim.skl",
        b"skeleton of Ann",
    );

    let input = sandbox.arg("co Spring");
    let run = sandbox.run(&["upgrade", &input, "--loose"]);
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/05 - Bob/fcl_hair.skl")).unwrap(),
        b"skeleton of Bob",
        "{}",
        run.stdout()
    );
    assert!(
        !sandbox
            .path("co Full Spring/Players/05 - Bob/fcl_hair_sim.skl")
            .exists()
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/06 - Ann/fcl_hair_sim.skl")).unwrap(),
        b"skeleton of Ann"
    );

    // The two face folders copied into a folder of their own, then through
    // `upgrade-models`.
    for folder in ["XXX05 - Bob", "XXX06 - Ann"] {
        fs::create_dir_all(sandbox.path(&format!("new models/{folder}"))).unwrap();
        for entry in fs::read_dir(sandbox.path(&format!("co Spring/Faces/{folder}"))).unwrap() {
            let name = entry.unwrap().file_name().to_str().unwrap().to_owned();
            fs::copy(
                sandbox.path(&format!("co Spring/Faces/{folder}/{name}")),
                sandbox.path(&format!("new models/{folder}/{name}")),
            )
            .unwrap();
        }
    }
    let input = sandbox.arg("new models");
    let run = sandbox.run(&["upgrade-models", &input]);
    assert_eq!(
        fs::read(sandbox.path("new models upgraded/05 - Bob/fcl_hair.skl")).unwrap(),
        b"skeleton of Bob",
        "{}",
        run.stdout()
    );
    assert!(
        !sandbox
            .path("new models upgraded/05 - Bob/fcl_hair_sim.skl")
            .exists()
    );
    assert!(
        sandbox
            .path("new models upgraded/06 - Ann/fcl_hair_sim.skl")
            .exists()
    );
}

// EU-PLY-15
#[test]
fn files_directly_in_the_faces_and_item_tops_are_leftovers() {
    let sandbox = Sandbox::new("ply_top_files");
    face(&sandbox, "co Spring/Faces/XXX05 - A");
    sandbox.write("co Spring/Faces/71406 - Civic cool.7z", b"archive");
    sandbox.write("co Spring/Boots/boots.fmdl", b"loose boots");
    sandbox.write("co Spring/Gloves/glove_l.fmdl", b"loose glove");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let stdout = run.stdout();
    for path in [
        "Faces/71406 - Civic cool.7z",
        "Boots/boots.fmdl",
        "Gloves/glove_l.fmdl",
    ] {
        assert!(
            stdout.contains(&format!(
                "- co Spring: Warning leftover_dropped at {path}\n"
            )),
            "{path}: {stdout}"
        );
    }
    // The output holds neither, under any name.
    for entry in sandbox.listing("co Full Spring") {
        for name in ["Civic cool.7z", "boots.fmdl", "glove_l.fmdl"] {
            assert!(!entry.ends_with(name), "{entry}");
        }
    }
    assert!(!stdout.contains("unmigrated_content"), "{stdout}");
}

// EU-PLY-16
#[test]
fn empty_face_folders_make_player_folders_with_no_ingame_face() {
    let sandbox = Sandbox::new("ply_empty_faces");
    fs::create_dir_all(sandbox.path("co Spring/Faces/XXX05 - Bob")).unwrap();
    fs::create_dir_all(sandbox.path("co Spring/Faces/XXX06 - Ann")).unwrap();
    boots(&sandbox, "co Spring/Boots/k0430 - FBM");
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71405, 430, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    for (name, bytes) in BOOTS_FILES {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Spring/Players/05 - Bob/{name}"))).unwrap(),
            bytes,
            "{name}"
        );
    }
    assert!(
        sandbox
            .path("co Full Spring/Players/05 - Bob/settings.toml")
            .is_file()
    );
    assert_eq!(
        sandbox.listing("co Full Spring/Players/06 - Ann"),
        ["settings.toml".to_owned()]
    );
    for player in ["05 - Bob", "06 - Ann"] {
        assert!(
            !sandbox
                .path(&format!("co Full Spring/Players/{player}/ingame_face.toml"))
                .exists(),
            "{player}"
        );
    }
    assert!(!sandbox.path("co Full Spring/Boots").exists());
}

// EU-PLY-17
#[test]
fn modelless_item_folders_mark_their_wearers_fpc_on() {
    let sandbox = Sandbox::new("ply_empty_items");
    face(&sandbox, "co Spring/Faces/XXX03 - Ann");
    sandbox.write("co Spring/Boots/k0430 - EMPTY/boots.skl", b"skl");
    fs::create_dir_all(sandbox.path("co Spring/Gloves/g0432 - EMPTY")).unwrap();
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71403, 430, 432);
        wears(file, 71404, 430, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/03 - Ann/fpc_on")).unwrap(),
        b""
    );
    assert!(
        !sandbox
            .path("co Full Spring/Players/03 - Ann/ingame_face.toml")
            .exists()
    );
    // Slot 04 has no face folder: named from his save name, both markers.
    let players = sandbox.listing("co Full Spring/Players");
    let slot04 = players
        .iter()
        .find(|entry| entry.starts_with("04"))
        .expect("slot 04's folder")
        .clone();
    // `fpc_on` empty as ever; `ingame_face.toml` filled from his record.
    assert_eq!(
        fs::read(sandbox.path(&format!("co Full Spring/Players/{slot04}/fpc_on"))).unwrap(),
        b""
    );
    assert!(
        sandbox
            .path(&format!("co Full Spring/Players/{slot04}/ingame_face.toml"))
            .is_file()
    );
    // His gloves record stores 0 and no gloves folder decides the
    // category: under `fpc_on` an unset `gloves_id` would give the hide
    // preset's ID, so the 0 is written explicitly, a Full output included.
    let text =
        fs::read_to_string(sandbox.path(&format!("co Full Spring/Players/{slot04}/settings.toml")))
            .unwrap();
    assert!(text.contains("gloves_id = 0"), "{text}");
    let stdout = run.stdout();
    assert!(
        stdout.contains("- co Spring: Warning leftover_dropped at Boots/k0430 - EMPTY/boots.skl"),
        "{stdout}"
    );
    // No folder holds boots.skl, no link is written, no shared folder.
    for entry in sandbox.listing("co Full Spring") {
        assert!(
            !entry.ends_with("boots.skl")
                && !entry.ends_with(".boots")
                && !entry.ends_with(".gloves"),
            "{entry}"
        );
    }
    assert!(!sandbox.path("co Full Spring/Boots").exists());
    assert!(!sandbox.path("co Full Spring/Gloves").exists());
}

// EU-PLY-18
#[test]
fn stray_files_in_face_and_item_folders_are_leftovers() {
    let sandbox = Sandbox::new("ply_strays");
    sandbox.write("co Spring/Faces/XXX05 - Bob/face.mtl", &mtl(&[]));
    sandbox.write("co Spring/Faces/XXX05 - Bob/face.mtl.backup", b"backup");
    sandbox.write("co Spring/Boots/k0430 - Crocs/boots.model", b"model");
    sandbox.write("co Spring/Boots/k0430 - Crocs/boots.mtl", &mtl(&[]));
    sandbox.write("co Spring/Boots/k0430 - Crocs/materials.mtl.bak", b"bak");
    sandbox.save_edited("EDIT16", pes_version::PesVersion::Pes16, |file| {
        wears(file, 71403, 430, 0);
        wears(file, 71404, 430, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT16");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        sandbox
            .path("co Full Spring/Players/05 - Bob/face.mtl")
            .is_file()
    );
    assert!(
        sandbox
            .path("co Full Spring/Boots/Crocs/boots.model")
            .is_file()
    );
    assert!(
        sandbox
            .path("co Full Spring/Boots/Crocs/boots.mtl")
            .is_file()
    );
    let stdout = run.stdout();
    for path in [
        "Faces/XXX05 - Bob/face.mtl.backup",
        "Boots/k0430 - Crocs/materials.mtl.bak",
    ] {
        assert!(
            stdout.contains(&format!(
                "- co Spring: Warning leftover_dropped at {path}\n"
            )),
            "{path}: {stdout}"
        );
    }
    for entry in sandbox.listing("co Full Spring") {
        assert!(
            !entry.ends_with(".backup") && !entry.ends_with(".bak"),
            "{entry}"
        );
    }
}

// EU-PLY-19
#[test]
fn copy_named_fmdls_in_item_folders_are_leftovers() {
    let sandbox = Sandbox::new("ply_fmdl_copies");
    face(&sandbox, "co Spring/Faces/XXX05 - A");
    sandbox.write("co Spring/Boots/k0430 - IKA-CHAN/boots.fmdl", b"boots");
    sandbox.write(
        "co Spring/Boots/k0430 - IKA-CHAN/boots - Copy.fmdl",
        b"copy",
    );
    sandbox.write("co Spring/Gloves/g0432/glove_l.fmdl", b"left");
    sandbox.write("co Spring/Gloves/g0432/glove_r.fmdl", b"right");
    sandbox.write("co Spring/Gloves/g0432/glove_l - Copy.fmdl", b"copy");
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71405, 430, 432);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    for name in ["boots.fmdl", "glove_l.fmdl", "glove_r.fmdl"] {
        assert!(
            sandbox
                .path(&format!("co Full Spring/Players/05 - A/{name}"))
                .is_file(),
            "{name}"
        );
    }
    let stdout = run.stdout();
    for path in [
        "Boots/k0430 - IKA-CHAN/boots - Copy.fmdl",
        "Gloves/g0432/glove_l - Copy.fmdl",
    ] {
        assert!(
            stdout.contains(&format!(
                "- co Spring: Warning leftover_dropped at {path}\n"
            )),
            "{path}: {stdout}"
        );
    }
    for entry in sandbox.listing("co Full Spring") {
        assert!(!entry.contains("Copy"), "{entry}");
    }
}

// EU-PLY-20
#[test]
fn a_game_path_face_folder_migrates_its_innermost_folder() {
    let sandbox = Sandbox::new("ply_game_path");
    for (name, bytes) in [
        ("face.xml", b"<config/>" as &[u8]),
        ("face_high_win32.model", b"face high"),
        ("face.mtl", &mtl(&[])),
    ] {
        sandbox.write(
            &format!(
                "co Spring/Faces/71401/common/character0/model/character/face/real/71401/{name}"
            ),
            bytes,
        );
    }
    fs::write(
        sandbox.path("EDIT16"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes16),
    )
    .unwrap();

    let run = upgrade(&sandbox, "co Spring", "EDIT16");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    // Slot 01, the folder named from his save record: the innermost
    // folder's files directly in it, no `common/` tree below.
    let players = sandbox.listing("co Full Spring/Players");
    let folder = players
        .iter()
        .find(|entry| entry.starts_with("01"))
        .expect("slot 01's folder")
        .clone();
    for name in ["face.xml", "face_high_win32.model", "face.mtl"] {
        assert!(
            sandbox
                .path(&format!("co Full Spring/Players/{folder}/{name}"))
                .is_file(),
            "{name}"
        );
    }
    assert!(
        !sandbox
            .path(&format!("co Full Spring/Players/{folder}/common"))
            .exists()
    );
}

// EU-PLY-21
#[test]
fn only_the_folderless_player_gets_ingame_face_toml_filled() {
    let sandbox = Sandbox::new("ply_folderless_face");
    face(&sandbox, "co Spring/Faces/XXX03 - A");
    boots(&sandbox, "co Spring/Boots/k0430 - Crocs");
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71405, 430, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let save = EditFile::load(&sandbox.path("EDIT")).unwrap();
    // Slot 03's own face: no `ingame_face.toml`, and his `settings.toml`
    // holds no face, hair or iris key.
    assert!(
        !sandbox
            .path("co Full Spring/Players/03 - A/ingame_face.toml")
            .exists()
    );
    let settings = PlayerSettings::parse(
        &fs::read_to_string(sandbox.path("co Full Spring/Players/03 - A/settings.toml")).unwrap(),
    )
    .unwrap();
    assert_eq!(settings.ingame_face(), IngameFaceSettings::default());
    // Slot 05, no face folder: `ingame_face.toml` with his record's face,
    // hair and iris.
    let face = IngameFaceSettings::parse(
        &fs::read_to_string(sandbox.path("co Full Spring/Players/05 - Crocs/ingame_face.toml"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        face,
        PlayerSettings::from_player(save.player(71405).unwrap())
            .unwrap()
            .ingame_face()
    );
}

// EU-PLY-23
#[test]
fn an_unworn_folder_named_for_the_wearer_pairs_his_custom_id() {
    let sandbox = Sandbox::new("ply_wearer_by_name");
    // VGL26 /omg/'s shape: the save numbers him 2552 while the folder made
    // for him is `k2952`.
    face(&sandbox, "co Spring/Faces/XXX05 - BANISHED");
    boots(&sandbox, "co Spring/Boots/k2952 - BANISHED");
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71405, 2552, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let stdout = run.stdout();
    assert!(
        stdout.contains(
            "- co Spring: Warning wearer_by_name at Boots/k2952 - BANISHED (slot=05, id=2552)"
        ),
        "{stdout}"
    );
    for (name, bytes) in BOOTS_FILES {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Spring/Players/05 - BANISHED/{name}")))
                .unwrap(),
            bytes,
            "{name}"
        );
    }
    // The folder claimed his category: no `boots_id`, no hide.
    let settings = PlayerSettings::parse(
        &fs::read_to_string(sandbox.path("co Full Spring/Players/05 - BANISHED/settings.toml"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(settings.boots_id, None);
    assert!(!stdout.contains("custom_model_hidden"), "{stdout}");
    assert!(!stdout.contains("Boots/k2952 - BANISHED/"), "{stdout}");
    assert!(!stdout.contains("unmigrated_content"), "{stdout}");
}

// EU-PLY-23
#[test]
fn two_unworn_folders_of_one_name_refuse_the_export() {
    let sandbox = Sandbox::new("ply_wearer_by_name_two");
    face(&sandbox, "co Spring/Faces/XXX05 - BANISHED");
    boots(&sandbox, "co Spring/Boots/k2952 - BANISHED");
    boots(&sandbox, "co Spring/Boots/k2953 - banished");
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71405, 2552, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    let stdout = run.stdout();
    // Two candidates for one player: neither pairs, and the missing ID
    // left beside unworn folders refuses the export (EU-PLY-26).
    assert!(
        stdout.contains("Error custom_model_unpaired (slot=05, key=boots_id, id=2552)"),
        "{stdout}"
    );
    assert!(!stdout.contains("wearer_by_name"), "{stdout}");
    sandbox.assert_nothing_written("co Full Spring", &stdout);
}

// EU-PLY-24
#[test]
fn an_id_numbered_in_the_other_kind_means_no_custom_item() {
    let sandbox = Sandbox::new("ply_not_worn");
    face(&sandbox, "co Spring/Faces/XXX05 - Hagoromo");
    boots(&sandbox, "co Spring/Boots/k2780 - Hagoromoslop");
    gloves(&sandbox, "co Spring/Gloves/g2799 - Momo");
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71405, 2780, 2780);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let stdout = run.stdout();
    // His gloves ID is a boots folder's number: no custom gloves, the hide
    // ID reported as such at Info.
    assert!(
        stdout.contains("- co Spring: Info custom_model_not_worn (slot=05"),
        "{stdout}"
    );
    for field in ["key=gloves_id", "id=2780", "written=11"] {
        assert!(stdout.contains(field), "{field}: {stdout}");
    }
    assert!(!stdout.contains("wearer_by_order"), "{stdout}");
    assert!(!stdout.contains("custom_model_hidden"), "{stdout}");
    for name in ["glove_l.fmdl", "glove_r.fmdl"] {
        assert!(
            stdout.contains(&format!("leftover_dropped at Gloves/g2799 - Momo/{name}")),
            "{name}: {stdout}"
        );
    }
    let settings = PlayerSettings::parse(
        &fs::read_to_string(sandbox.path("co Full Spring/Players/05 - Hagoromo/settings.toml"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(settings.gloves_id, Some(11));
    // The boots folder is his own worn ID's: it merges and claims the
    // category, so `boots_id` is left out.
    assert_eq!(settings.boots_id, None);
    for (name, bytes) in BOOTS_FILES {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Spring/Players/05 - Hagoromo/{name}")))
                .unwrap(),
            bytes,
            "{name}"
        );
    }
}

// EU-PLY-25
#[test]
fn order_then_number_pair_the_renumbered_saves_folders() {
    let sandbox = Sandbox::new("ply_order_number");
    for slot in [
        "XXX01 - CHALICE OF HEAVEN",
        "XXX05 - PARADISE",
        "XXX12 - ACKNOWLEDGED",
    ] {
        face(&sandbox, &format!("co Spring/Faces/{slot}"));
    }
    // Every model carries its folder's own text, so a wrong pairing shows
    // in the copied bytes, not only in the findings.
    for folder in [
        "k2451 - Acknowledged",
        "k2452 - DEFAULT MALE",
        "k2455 - DEFAULT FEMALE",
    ] {
        boots(&sandbox, &format!("co Spring/Boots/{folder}"));
        sandbox.write(
            &format!("co Spring/Boots/{folder}/boots.fmdl"),
            folder.as_bytes(),
        );
    }
    fs::create_dir_all(sandbox.path("co Spring/Gloves/g2451 - EMPTY")).unwrap();
    for folder in ["g2452 - DEFAULT MALE", "g2455 - DEFAULT FEMALE"] {
        gloves(&sandbox, &format!("co Spring/Gloves/{folder}"));
        for name in ["glove_l.fmdl", "glove_r.fmdl"] {
            sandbox.write(
                &format!("co Spring/Gloves/{folder}/{name}"),
                format!("{folder} {name}").as_bytes(),
            );
        }
    }
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71401, 2227, 2227);
        wears(file, 71405, 2229, 2229);
        wears(file, 71412, 2226, 2226);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 0, "{stdout}");
    for line in [
        "- co Spring: Warning wearer_by_name at Boots/k2451 - Acknowledged (slot=12, id=2226)",
        "- co Spring: Warning wearer_by_order at Boots/k2452 - DEFAULT MALE (slot=01, id=2227)",
        "- co Spring: Warning wearer_by_order at Boots/k2455 - DEFAULT FEMALE (slot=05, id=2229)",
        "- co Spring: Warning wearer_by_number at Gloves/g2452 - DEFAULT MALE (slot=01, id=2227, from=Boots/k2452 - DEFAULT MALE)",
        "- co Spring: Warning wearer_by_number at Gloves/g2455 - DEFAULT FEMALE (slot=05, id=2229, from=Boots/k2455 - DEFAULT FEMALE)",
        "- co Spring: Warning wearer_by_number at Gloves/g2451 - EMPTY (slot=12, id=2226, from=Boots/k2451 - Acknowledged)",
    ] {
        assert!(stdout.contains(&format!("{line}\n")), "{line}: {stdout}");
    }
    // Each delivered file is its paired folder's own, byte for byte.
    for (folder, files) in [
        (
            "01 - CHALICE OF HEAVEN",
            vec![
                ("boots.fmdl", "Boots/k2452 - DEFAULT MALE"),
                ("glove_l.fmdl", "Gloves/g2452 - DEFAULT MALE"),
                ("glove_r.fmdl", "Gloves/g2452 - DEFAULT MALE"),
            ],
        ),
        (
            "05 - PARADISE",
            vec![
                ("boots.fmdl", "Boots/k2455 - DEFAULT FEMALE"),
                ("glove_l.fmdl", "Gloves/g2455 - DEFAULT FEMALE"),
                ("glove_r.fmdl", "Gloves/g2455 - DEFAULT FEMALE"),
            ],
        ),
        (
            "12 - ACKNOWLEDGED",
            vec![("boots.fmdl", "Boots/k2451 - Acknowledged")],
        ),
    ] {
        for (name, source) in files {
            assert_eq!(
                fs::read(sandbox.path(&format!("co Full Spring/Players/{folder}/{name}"))).unwrap(),
                fs::read(sandbox.path(&format!("co Spring/{source}/{name}"))).unwrap(),
                "{folder}/{name}"
            );
        }
    }
    // The modelless gloves folder pairs all the same: slot 12 gets the
    // `fpc_on` marker, his `gloves_id` is left out, and no hide warning.
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/12 - ACKNOWLEDGED/fpc_on")).unwrap(),
        b""
    );
    let settings = PlayerSettings::parse(
        &fs::read_to_string(sandbox.path("co Full Spring/Players/12 - ACKNOWLEDGED/settings.toml"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(settings.gloves_id, None);
    assert_eq!(settings.boots_id, None);
    assert!(!stdout.contains("custom_model_hidden"), "{stdout}");
    assert!(!stdout.contains("custom_model_unpaired"), "{stdout}");
}

// EU-PLY-26
#[test]
fn a_missing_id_left_beside_unworn_folders_refuses_the_export() {
    let sandbox = Sandbox::new("ply_unpaired");
    face(&sandbox, "co Spring/Faces/XXX01 - Bard");
    boots(&sandbox, "co Spring/Boots/k3454 - KitM_Back");
    boots(&sandbox, "co Spring/Boots/k3456 - KitL_Back");
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        wears(file, 71401, 3460, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    assert!(
        run.stdout()
            .contains("Error custom_model_unpaired (slot=01, key=boots_id, id=3460)"),
        "{}",
        run.stdout()
    );
    sandbox.assert_nothing_written("co Full Spring", &run.stdout());
}
