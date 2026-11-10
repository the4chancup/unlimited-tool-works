//! Loose mode (the `spec/export_upgrader.md` "Loose mode" scenarios): no savefile, so
//! each item folder's wearer is the player at its ID's position in the team's
//! old block of 25 IDs, the block its kind's lowest ID falls in, and each
//! assumed pairing is reported.

use std::fs;

use pes_savefile::settings_toml::{NameSetting, PlayerSettings};

use crate::common::Sandbox;
use crate::players::{boots, face};

/// `upgrade <input> --loose`.
fn upgrade_loose(sandbox: &Sandbox, input: &str) -> crate::common::Run {
    let input = sandbox.arg(input);
    sandbox.run(&["upgrade", &input, "--loose"])
}

// EU-LOOSE-01
#[test]
fn the_ids_position_in_the_block_pairs_the_wearer() {
    let sandbox = Sandbox::new("loose_ids");
    // /a/'s old block is 126 to 150, so 0128 is slot 03 and 0130 slot 05.
    sandbox.write("a Spring/a Note.txt", b"Team: /a/\r\n");
    face(&sandbox, "a Spring/Faces/XXX03 - Ann");
    boots(&sandbox, "a Spring/Boots/k0128 - Rex");
    boots(&sandbox, "a Spring/Boots/k0130");

    let run = upgrade_loose(&sandbox, "a Spring");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let stdout = run.stdout();
    for line in [
        "- a Spring: Info wearer_assumed at Boots/k0128 - Rex (id=0128, slot=03)\n",
        "- a Spring: Info wearer_assumed at Boots/k0130 (id=0130, slot=05)\n",
    ] {
        assert!(stdout.contains(line), "{line:?} not in:\n{stdout}");
    }
    let out = "a Full Spring";
    assert_eq!(
        fs::read(sandbox.path(&format!("{out}/Players/03 - Ann/boots.fmdl"))).unwrap(),
        b"boots"
    );
    assert_eq!(
        fs::read(sandbox.path(&format!("{out}/Players/05/boots.fmdl"))).unwrap(),
        b"boots"
    );
    // Loose mode has no record to fill it with: the marker's file is empty.
    assert_eq!(
        fs::read(sandbox.path(&format!("{out}/Players/05/ingame_face.toml"))).unwrap(),
        b""
    );
    assert!(
        !sandbox
            .path(&format!("{out}/Players/05/settings.toml"))
            .exists()
    );
    let text =
        fs::read_to_string(sandbox.path(&format!("{out}/Players/03 - Ann/settings.toml"))).unwrap();
    let parsed = PlayerSettings::parse(&text).expect("the settings.toml parses");
    assert_eq!(
        parsed,
        PlayerSettings {
            name: Some(NameSetting::FromFolder),
            ..PlayerSettings::default()
        }
    );
    let mut entries: Vec<String> = fs::read_dir(sandbox.path(&format!("{out}/Players")))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_str().unwrap().to_owned())
        .collect();
    entries.sort();
    assert_eq!(entries, ["03 - Ann", "05"]);
    assert!(!sandbox.path(&format!("{out}/Boots")).exists());
}

// EU-SET-16
#[test]
fn loose_mode_records_the_inferred_ids() {
    let sandbox = Sandbox::new("loose_legacy_ids");
    // EU-LOOSE-01's export: /a/'s old block is 126 to 150, so 0128 is slot
    // 03 and 0130 slot 05.
    sandbox.write("a Spring/a Note.txt", b"Team: /a/\r\n");
    face(&sandbox, "a Spring/Faces/XXX03 - Ann");
    boots(&sandbox, "a Spring/Boots/k0128 - Rex");
    boots(&sandbox, "a Spring/Boots/k0130");

    let run = upgrade_loose(&sandbox, "a Spring");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        fs::read_to_string(sandbox.path("a Full Spring/legacy_ids.toml")).unwrap(),
        concat!(
            "# The old export's boots and gloves IDs, read only by the Team compiler's legacy_boots_ids setting (VGL27).\n",
            "[03]\nboots = 128\n",
            "[05]\nboots = 130\n",
        )
    );
}

// EU-LOOSE-02
#[test]
fn the_ids_naming_no_player_are_unmigrated_content() {
    let sandbox = Sandbox::new("loose_ids_past");
    // EU-LOOSE-01's export, plus a boots folder at a spare position and one
    // past the block.
    sandbox.write("a Spring/a Note.txt", b"Team: /a/\r\n");
    face(&sandbox, "a Spring/Faces/XXX03 - Ann");
    boots(&sandbox, "a Spring/Boots/k0128 - Rex");
    boots(&sandbox, "a Spring/Boots/k0130");
    sandbox.write("a Spring/Boots/k0149/boots.fmdl", b"spare");
    sandbox.write("a Spring/Boots/k0155/boots.fmdl", b"past");

    let run = upgrade_loose(&sandbox, "a Spring");
    assert_eq!(run.exit_code(), 1);
    let stdout = run.stdout();
    for line in [
        "- a Spring: Error unmigrated_content at Boots/k0149 (reason=loose_unplaceable)\n",
        "- a Spring: Error unmigrated_content at Boots/k0155 (reason=loose_unplaceable)\n",
    ] {
        assert!(stdout.contains(line), "{line:?} not in:\n{stdout}");
    }
    sandbox.assert_nothing_written("a Full Spring", &stdout);
}

// EU-LOOSE-03
#[test]
fn an_empty_face_folder_makes_its_player_folder() {
    // An empty face folder is still a player folder (step 3): the
    // directory is created even with nothing to write into it (loose mode
    // writes no settings.toml for a number-only name), for the compiler to
    // fill the blank face.
    let sandbox = Sandbox::new("loose_empty_face");
    sandbox.write("a Spring/a Note.txt", b"Team: /a/\r\n");
    fs::create_dir_all(sandbox.path("a Spring/Faces/71405")).unwrap();

    let run = upgrade_loose(&sandbox, "a Spring");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let folder = sandbox.path("a Full Spring/Players/05");
    assert!(folder.is_dir(), "{}", run.stdout());
    assert!(sandbox.listing("a Full Spring/Players/05").is_empty());
    assert!(
        !sandbox
            .path("a Full Spring/Players/05/ingame_face.toml")
            .exists()
    );
}
