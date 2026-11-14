//! The `settings.toml` generation (the `spec/export_upgrader.md` "Player settings"
//! scenarios): the save record's values as keys, the folder-claimed and
//! coverage ID rules, the name rule, the missing custom model and the value
//! no key can hold.

use std::fs;

use pes_savefile::EditFile;
use pes_savefile::settings_toml::{IngameFaceSettings, NameSetting, PlayerSettings};
use pes_version::PesVersion;

use crate::carried::mtl;
use crate::common::Sandbox;
use crate::players::{boots, face, gloves, upgrade_with, wears};

/// `upgrade <input> --savefile <save>`.
fn upgrade(sandbox: &Sandbox, input: &str, save: &str) -> crate::common::Run {
    upgrade_with(sandbox, input, save, &[])
}

/// The `settings.toml` at `path` (an output-relative path ending in
/// `settings.toml`), parsed.
fn settings(sandbox: &Sandbox, path: &str) -> PlayerSettings {
    let text = fs::read_to_string(sandbox.path(path)).unwrap();
    PlayerSettings::parse(&text).unwrap_or_else(|error| panic!("{path}: {error}"))
}

// EU-SET-10
#[test]
fn a_face_folder_gets_a_settings_toml_of_his_record_but_the_face_keys() {
    let sandbox = Sandbox::new("set_record");
    // The /co/ fixture's slot 05 is named "PLACEHOLDER".
    face(&sandbox, "co Spring/Faces/XXX05 - PLACEHOLDER");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let path = "co Full Spring/Players/05 - PLACEHOLDER/settings.toml";
    let parsed = settings(&sandbox, path);
    let save = EditFile::load(&sandbox.path("EDIT")).unwrap();
    let record = save.player(71405).unwrap();
    let mut expected = PlayerSettings::from_player(record).expect("from_player");
    expected.set_ingame_face(IngameFaceSettings::default());
    assert_eq!(
        parsed.appearance, expected.appearance,
        "every appearance key his record's value but the face, hair and iris"
    );
    assert_eq!(parsed.shirt_name, Some(record.shirt_name.clone()));
    // He keeps his own face: no `ingame_face.toml`.
    assert!(
        !sandbox
            .path("co Full Spring/Players/05 - PLACEHOLDER/ingame_face.toml")
            .exists()
    );
    let text = fs::read_to_string(sandbox.path(path)).unwrap();
    for line in text.lines() {
        let line = line.trim_start_matches('#').trim_start();
        assert!(!line.starts_with("height ="), "{line}");
        assert!(!line.starts_with("weight ="), "{line}");
    }
}

// EU-SET-10
#[test]
fn an_upgraded_export_keeps_a_seven_star_teams_seven() {
    let sandbox = Sandbox::new("set_star");
    for slot in ["05", "06", "07"] {
        face(&sandbox, &format!("co Spring/Faces/XXX{slot} - A"));
    }
    sandbox.save_edited("EDIT", PesVersion::Pes21, |file| {
        for player in file.players_mut() {
            player.stats.star = Some(7);
        }
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let written: Vec<String> = sandbox
        .listing("co Full Spring/Players")
        .into_iter()
        .filter(|path| path.ends_with("settings.toml"))
        .collect();
    assert_eq!(written.len(), 3, "a settings.toml per folder: {written:?}");
    for path in written {
        let path = format!("co Full Spring/Players/{path}");
        assert_eq!(
            settings(&sandbox, &path).appearance.presentation.star,
            Some(7),
            "{path}: the save's star"
        );
    }
}

// EU-SET-02
#[test]
fn the_id_rules_stock_kept_zero_written_midcup_and_folder_claimed() {
    let sandbox = Sandbox::new("set_ids");
    for input in ["co Spring", "co Midcup Spring"] {
        face(&sandbox, &format!("{input}/Faces/XXX05 - A"));
        face(&sandbox, &format!("{input}/Faces/XXX06 - B"));
        boots(&sandbox, &format!("{input}/Boots/k0430 - Crocs"));
    }
    sandbox.save_edited("EDIT", PesVersion::Pes21, |file| {
        wears(file, 71405, 12, 0);
        wears(file, 71406, 430, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let first = settings(&sandbox, "co Full Spring/Players/05 - A/settings.toml");
    assert_eq!(first.boots_id, Some(12));
    assert_eq!(first.gloves_id, None);
    let second = settings(&sandbox, "co Full Spring/Players/06 - B/settings.toml");
    assert_eq!(second.boots_id, None, "the migrated folder decides");
    assert_eq!(second.gloves_id, None);

    // "co Midcup Spring" already carries the tag: the output is "…upgraded".
    let run = upgrade(&sandbox, "co Midcup Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let mut boots_ids = Vec::new();
    for folder in ["05 - A", "06 - B"] {
        let parsed = settings(
            &sandbox,
            &format!("co Midcup Spring upgraded/Players/{folder}/settings.toml"),
        );
        assert_eq!(
            parsed.gloves_id,
            Some(0),
            "{folder}: a Midcup unset key keeps the installed ID"
        );
        boots_ids.push(parsed.boots_id);
    }
    assert_eq!(
        boots_ids,
        [Some(12), None],
        "the stock ID kept, the migrated folder deciding the custom one, in Midcup too"
    );
}

// EU-SET-14
#[test]
fn an_unclaimed_custom_model_becomes_the_hide_id_with_a_folder() {
    let sandbox = Sandbox::new("set_hidden");
    face(&sandbox, "co Spring/Faces/XXX05 - A");
    // "co Summer" gives slot 05 no folder: the same worn IDs are missing there.
    face(&sandbox, "co Summer/Faces/XXX03 - A");
    sandbox.save_edited("EDIT", PesVersion::Pes21, |file| {
        wears(file, 71405, 433, 434)
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    for line in [
        "- co Spring: Warning custom_model_hidden (slot=05, name=PLACEHOLDER, key=boots_id, id=433, written=55)",
        "- co Spring: Warning custom_model_hidden (slot=05, name=PLACEHOLDER, key=gloves_id, id=434, written=11)",
    ] {
        assert!(run.stdout().contains(line), "{line}\n{}", run.stdout());
    }
    assert!(run.stdout().contains("migration_clean"), "{}", run.stdout());
    let parsed = settings(&sandbox, "co Full Spring/Players/05 - A/settings.toml");
    assert_eq!(parsed.boots_id, Some(55));
    assert_eq!(parsed.gloves_id, Some(11));

    // The same worn IDs without his face folder are missing content.
    let run = upgrade(&sandbox, "co Summer", "EDIT");
    assert_eq!(run.exit_code(), 1);
    for line in [
        "- co Summer: Error custom_model_missing (slot=05, name=PLACEHOLDER, key=boots_id, id=433)",
        "- co Summer: Error custom_model_missing (slot=05, name=PLACEHOLDER, key=gloves_id, id=434)",
    ] {
        assert!(run.stdout().contains(line), "{line}\n{}", run.stdout());
    }
    sandbox.assert_nothing_written("co Full Summer", &run.stdout());
}

// EU-SET-15
#[test]
fn the_worn_item_folder_ids_are_recorded_in_legacy_ids_toml() {
    let sandbox = Sandbox::new("set_legacy_ids");
    face(&sandbox, "co Spring/Faces/XXX03 - A");
    // Slot 05 has no face folder, as in EU-PLY-02: the boots folder's name
    // part names him.
    boots(&sandbox, "co Spring/Boots/k0430 - Crocs");
    gloves(&sandbox, "co Spring/Gloves/g0431");
    // Slot 09's worn boots folder holds no model: it claims the category
    // (fpc_on) but records no ID.
    sandbox.write("co Spring/Boots/k0440 - Empty/boots.skl", b"skl");
    sandbox.save_edited("EDIT", PesVersion::Pes21, |file| {
        wears(file, 71405, 430, 0);
        wears(file, 71406, 0, 431);
        wears(file, 71407, 0, 431);
        wears(file, 71408, 55, 0);
        wears(file, 71409, 440, 0);
    });

    let run = upgrade_with(&sandbox, "co Spring", "EDIT", &["--legacy-ids"]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(run.stdout().contains("migration_clean"), "{}", run.stdout());
    assert_eq!(
        fs::read_to_string(sandbox.path("co Full Spring/legacy_ids.toml")).unwrap(),
        concat!(
            "# The old export's boots and gloves IDs, which the Team compiler packs instead of its own (VGL27).\n",
            "[05]\nboots = 430\n",
            "[06]\ngloves = 431\n",
            "[07]\ngloves = 431\n",
        )
    );

    // The same export without the flag writes no file.
    let out = sandbox.arg("again");
    let run = upgrade_with(&sandbox, "co Spring", "EDIT", &["-o", out.as_str()]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        !sandbox
            .path("again/co Full Spring/legacy_ids.toml")
            .exists()
    );

    // An export whose players wear only stock IDs writes no file either.
    face(&sandbox, "co Summer/Faces/XXX03 - A");
    sandbox.save_edited("EDIT2", PesVersion::Pes21, |file| wears(file, 71403, 12, 3));
    let run = upgrade_with(&sandbox, "co Summer", "EDIT2", &["--legacy-ids"]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(!sandbox.path("co Full Summer/legacy_ids.toml").exists());
}

// EU-SET-04
#[test]
fn the_name_is_true_explicit_or_the_coded_save_name() {
    let sandbox = Sandbox::new("set_names");
    face(&sandbox, "co Spring/Faces/XXX05 - PLACEHOLDER");
    face(&sandbox, "co Spring/Faces/XXX06 - Other");
    face(&sandbox, "co Spring/Faces/XXX09");
    face(&sandbox, "co Spring/Faces/XXX10");
    sandbox.save_edited("EDIT", PesVersion::Pes21, |file| {
        // The colour-coded name of fixture player 78205, as EU-PLY-03 uses.
        file.player_mut(71409).unwrap().name = "\u{11}c789922ff>skin".to_owned();
        // A coded name whose display form equals the folder's label:
        // `name = true` would write "skin", stripping the codes.
        file.player_mut(71410).unwrap().name = "\u{11}c789922ffskin".to_owned();
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        settings(
            &sandbox,
            "co Full Spring/Players/05 - PLACEHOLDER/settings.toml"
        )
        .name,
        Some(NameSetting::FromFolder)
    );
    assert_eq!(
        settings(&sandbox, "co Full Spring/Players/06 - Other/settings.toml").name,
        Some(NameSetting::Explicit("PLACEHOLDER".to_owned()))
    );
    assert_eq!(
        settings(&sandbox, "co Full Spring/Players/09 - skin/settings.toml").name,
        Some(NameSetting::Explicit("\u{11}c789922ff>skin".to_owned())),
        "the coded save name byte for byte, in a folder named without it"
    );
    assert_eq!(
        settings(&sandbox, "co Full Spring/Players/10 - skin/settings.toml").name,
        Some(NameSetting::Explicit("\u{11}c789922ffskin".to_owned())),
        "the raw name decides, not the display name the folder derived"
    );
}

// EU-SET-11
#[test]
fn a_value_past_the_assumed_cap_is_written_as_stored() {
    let sandbox = Sandbox::new("set_past_top");
    // The PES 18 fixture save's /an/ is team 704, as the save itself numbers
    // it; its slot-04 player wears k0430 and holds a `cheek_type` past the
    // assumed cap (his 5, inside the field's bits the unmeasured cap is).
    // No face folder and custom boots: his folder marks the
    // in-game face, so the key lands in `ingame_face.toml`.
    boots(&sandbox, "an Spring/Boots/k0430 - Crocs");
    sandbox.save_edited("EDIT18", PesVersion::Pes18, |file| {
        // The rest of the roster's worn custom IDs, or
        // `custom_model_missing` is all the run reports.
        for id in 70401..=70423 {
            wears(file, id, 0, 0);
        }
        wears(file, 70404, 430, 0);
        let mut settings = PlayerSettings::default();
        settings.appearance.face.cheek_type = Some(5);
        settings
            .apply(file.player_mut(70404).unwrap(), PesVersion::Pes18)
            .unwrap();
    });

    let run = upgrade(&sandbox, "an Spring", "EDIT18");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let face = IngameFaceSettings::parse(
        &fs::read_to_string(sandbox.path("an Full Spring/Players/04 - Crocs/ingame_face.toml"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(face.face.cheek_type, Some(5));
    let named = run
        .stdout()
        .lines()
        .filter(|line| line.contains("settings_") && line.contains("Paul Levesque"))
        .count();
    assert_eq!(named, 0, "nothing is reported about him: {}", run.stdout());
    // The compile keeps what the upgrade wrote: PES 18's unmeasured cap is
    // the field's bits, so the narrowing does not reset his 5.
    let mut compiled = PlayerSettings::default();
    compiled.set_ingame_face(face);
    assert_eq!(compiled.narrow_to(PesVersion::Pes18), []);
    assert_eq!(compiled.appearance.face.cheek_type, Some(5));
}

// EU-SET-07
#[test]
fn an_fpc_strip_writes_no_marker_and_the_raw_values() {
    let sandbox = Sandbox::new("set_fpc");
    face(&sandbox, "co Spring/Faces/XXX05 - A");
    boots(&sandbox, "co Spring/Boots/k0126 - Blank");
    gloves(&sandbox, "co Spring/Gloves/g0127 - Blank");
    sandbox.save_edited("EDIT", PesVersion::Pes21, |file| {
        // The enable preset's strip (`fpc`'s `Preset::Hide`: long sleeves,
        // tucked, short socks), with his own blank-model IDs.
        let player = file.player_mut(71405).unwrap();
        player.appearance.sleeves = 2;
        player.appearance.untucked = false;
        player.appearance.socks = 2;
        wears(file, 71405, 126, 127);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let folder = "co Full Spring/Players/05 - A";
    assert!(!sandbox.path(&format!("{folder}/fpc_on")).exists());
    let parsed = settings(&sandbox, &format!("{folder}/settings.toml"));
    let save = EditFile::load(&sandbox.path("EDIT")).unwrap();
    let expected = PlayerSettings::from_player(save.player(71405).unwrap())
        .expect("from_player")
        .appearance
        .strip;
    assert_eq!(parsed.appearance.strip, expected, "the record's strip");
    assert_eq!(parsed.boots_id, None, "the boots folder decides");
    assert_eq!(parsed.gloves_id, None, "the gloves folder decides");
    // His own boots and gloves folders merged into his player folder.
    for name in ["boots.fmdl", "boots.skl", "glove_l.fmdl", "glove_r.fmdl"] {
        assert!(sandbox.path(&format!("{folder}/{name}")).exists(), "{name}");
    }
}

// EU-SET-08
#[test]
fn a_folderless_stock_wearer_gets_no_folder() {
    let sandbox = Sandbox::new("set_no_folder");
    face(&sandbox, "co Spring/Faces/XXX03 - A");
    sandbox.save_edited("EDIT", PesVersion::Pes21, |file| wears(file, 71410, 12, 3));

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let entries = sandbox.listing("co Full Spring/Players");
    assert!(
        entries.iter().all(|path| !path.starts_with("10")),
        "no player folder for slot 10: {entries:?}"
    );
}

// EU-SET-09
#[test]
fn an_unwritable_value_is_left_out_and_reported() {
    let sandbox = Sandbox::new("set_refused");
    // /jp/ player 73105, the PES 16 fixture save's slot-5 "SHITEYANYO" — the
    // same player `from_player_skipping`'s unit test finds by walking the
    // fixture for a stored 183 in `goal_celebration_2`; pinned 2026-10-18.
    // A pre-Fox face folder (a test of names, not formats).
    for (name, bytes) in [
        ("oral_face_neck_win32.model", b"model".as_slice()),
        ("face_neck.mtl", mtl(&[]).as_slice()),
        ("portrait.dds", b"portrait".as_slice()),
    ] {
        sandbox.write(&format!("jp Spring/Faces/XXX05 - SHITEYANYO/{name}"), bytes);
    }
    sandbox.save_edited("EDIT16", PesVersion::Pes16, |_| {});

    let run = upgrade(&sandbox, "jp Spring", "EDIT16");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        run.stdout().contains(
            "- jp Spring: Warning settings_value_refused (slot=05, name=SHITEYANYO, key=appearance.motion.goal_celebration_2, value=183)"
        ),
        "{}",
        run.stdout()
    );
    let parsed = settings(
        &sandbox,
        "jp Full Spring/Players/05 - SHITEYANYO/settings.toml",
    );
    assert_eq!(parsed.appearance.motion.goal_celebration_2, None);
    assert!(parsed.appearance.motion.goal_celebration_1.is_some());
}
