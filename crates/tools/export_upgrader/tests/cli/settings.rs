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
use crate::players::{boots, face, gloves, wears};

/// `upgrade <input> --savefile <save>`.
fn upgrade(sandbox: &Sandbox, input: &str, save: &str) -> crate::common::Run {
    let input = sandbox.arg(input);
    let save = sandbox.arg(save);
    sandbox.run(&["upgrade", &input, "--savefile", &save])
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

// EU-SET-03
#[test]
fn a_missing_custom_model_is_reported() {
    let sandbox = Sandbox::new("set_missing");
    face(&sandbox, "co Spring/Faces/XXX05 - A");
    sandbox.save_edited("EDIT", PesVersion::Pes21, |file| wears(file, 71405, 433, 0));

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 1);
    assert!(
        run.stdout().contains(
            "- co Spring: Error custom_model_missing (slot=05, name=PLACEHOLDER, key=boots_id, id=433)"
        ),
        "{}",
        run.stdout()
    );
    sandbox.assert_nothing_written("co Full Spring", &run.stdout());
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
    // /an/ player 70504, the PES 18 fixture save's slot-4 "Paul Levesque":
    // found by walking the fixture's players for a `cheek_type` past the
    // assumed cap (his 5, inside the field's bits the unmeasured cap is),
    // 2026-10-18. No face folder and custom boots: his folder marks the
    // in-game face, so the key lands in `ingame_face.toml`.
    boots(&sandbox, "an Spring/Boots/k0430 - Crocs");
    sandbox.save_edited("EDIT18", PesVersion::Pes18, |file| {
        wears(file, 70504, 430, 0);
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
