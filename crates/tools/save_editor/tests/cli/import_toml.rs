//! `import-toml`: a Team TOML file applied onto the team it names, and the save written with
//! the `.bak` rule.

use std::fs;

use pes_savefile::EditFile;
use pes_savefile::interchange::team_toml::TeamToml;
use pes_version::PesVersion;

use crate::{Sandbox, WritingCommand, payload, payload_after};

/// Team `/co/`, 714 in the PES 21 fixture, whose slot 3 is player 71403.
const CO: u32 = 714;
const CO_SLOT_3: u32 = 71403;

impl Sandbox {
    /// `export-toml --team <id>` of the save at `save` into `relative`, asserting it worked.
    fn export(&self, save: &str, id: u32, relative: &str) {
        let run = self.run(&[
            "export-toml",
            &self.arg(save),
            "--team",
            &id.to_string(),
            "--out",
            &self.arg(relative),
        ]);
        assert_eq!(run.exit_code(), 0, "{:?}", run.messages());
    }

    /// `import-toml <file> --base <base> --out <out>`.
    fn import(&self, file: &str, base: &str, out: &str) -> crate::common::Run {
        self.run(&[
            "import-toml",
            &self.arg(file),
            "--base",
            &self.arg(base),
            "--out",
            &self.arg(out),
        ])
    }
}

// SE-CLI-01
#[test]
fn a_write_keeps_the_file_it_replaces_as_bak_also_when_out_is_base() {
    let sandbox = Sandbox::new("import_bak");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    sandbox.export("EDIT", CO, "co.toml");
    fs::write(sandbox.path("OUT"), b"an older file").unwrap();

    let run = sandbox.import("co.toml", "EDIT", "OUT");
    assert_eq!(run.exit_code(), 0, "{:?}", run.messages());
    assert_eq!(fs::read(sandbox.path("OUT.bak")).unwrap(), b"an older file");
    let written = fs::read(sandbox.path("OUT")).unwrap();
    assert_ne!(written, b"an older file");

    let run = sandbox.import("co.toml", "OUT", "OUT");
    assert_eq!(run.exit_code(), 0, "{:?}", run.messages());
    assert_eq!(
        fs::read(sandbox.path("OUT.bak")).unwrap(),
        written,
        "the .bak holds the file the second write replaced"
    );
    assert_ne!(
        fs::read(sandbox.path("OUT")).unwrap(),
        written,
        "a write re-encrypts under a fresh salt"
    );
}

// SE-TOML-01
#[test]
fn a_team_exported_and_imported_back_leaves_the_payload_equal() {
    let sandbox = Sandbox::new("import_round_trip");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    sandbox.export("EDIT", CO, "co.toml");
    let run = sandbox.import("co.toml", "EDIT", "OUT");
    assert_eq!(run.exit_code(), 0);
    assert!(run.messages().is_empty(), "{:?}", run.messages());
    assert_eq!(
        payload(&sandbox.path("OUT")),
        payload(&sandbox.path("EDIT"))
    );
}

// SE-TOML-04
#[test]
fn a_minimal_document_writes_its_keys_and_nothing_else() {
    let sandbox = Sandbox::new("import_minimal");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    let before = EditFile::load(&sandbox.path("EDIT")).unwrap();
    let player = before.player(CO_SLOT_3).unwrap();
    assert_eq!(before.team(CO).unwrap().roster[2].player_id, CO_SLOT_3);
    assert_ne!(player.name, "PATCH");
    // `neck_length = 2` in the file is stored as 2 + 7.
    assert_ne!(player.appearance.neck_length, 9);
    fs::write(
        sandbox.path("minimal.toml"),
        format!(
            "[team]\nid = {CO}\n\n[players.03]\nname = \"PATCH\"\n\n[players.03.appearance.physique]\nneck_length = 2\n"
        ),
    )
    .unwrap();

    let run = sandbox.import("minimal.toml", "EDIT", "OUT");
    assert_eq!(run.exit_code(), 0);
    assert!(run.messages().is_empty(), "{:?}", run.messages());
    let expected = payload_after(&sandbox.path("EDIT"), |file| {
        let player = file.player_mut(CO_SLOT_3).unwrap();
        player.name = "PATCH".to_owned();
        player.appearance.neck_length = 9;
    });
    assert_eq!(payload(&sandbox.path("OUT")), expected);
}

// SE-TOML-05
#[test]
fn a_pes21_document_imports_into_pes18_converted_with_a_note_per_value() {
    let sandbox = Sandbox::new("import_cross_version");
    sandbox.fixture(PesVersion::Pes21, "EDIT21");
    sandbox.fixture(PesVersion::Pes18, "EDIT18");
    // `/a/` is team 702 in both fixtures.
    let team = 702;
    assert_eq!(
        EditFile::load(&sandbox.path("EDIT18"))
            .unwrap()
            .team(team)
            .unwrap()
            .name,
        "/a/"
    );
    sandbox.export("EDIT21", team, "a.toml");
    let mut document =
        TeamToml::parse(&fs::read_to_string(sandbox.path("a.toml")).unwrap()).unwrap();
    assert_eq!(document.team.name.as_deref(), Some("/a/"));
    // PES 18's face caps are unmeasured — the fields' bit maxima — so no
    // document value exceeds one; the capped note uses a motion, whose PES
    // 18 top (127) is measured.
    document
        .players
        .get_mut(&1)
        .unwrap()
        .appearance
        .motion
        .goal_celebration_1 = Some(162);
    fs::write(sandbox.path("a.toml"), document.to_toml().unwrap()).unwrap();

    let run = sandbox.import("a.toml", "EDIT18", "OUT18");
    assert_eq!(run.exit_code(), 0);
    let messages = run.messages();
    assert!(
        messages.contains(
            &"Warning import_value_capped (path=players.01.appearance.motion.goal_celebration_1, from=162, to=127)".to_owned()
        ),
        "{messages:?}"
    );
    assert!(
        messages.contains(
            &"Warning import_key_not_in_version (path=players.01.stats.tight_possession)"
                .to_owned()
        ),
        "{messages:?}"
    );
    let expected = payload_after(&sandbox.path("EDIT18"), |file| {
        let mut target = file.team(team).unwrap().clone();
        let notes = document
            .apply(PesVersion::Pes18, &mut target, file.players_mut())
            .unwrap();
        assert_eq!(notes.len(), messages.len(), "one line per note");
        *file.team_mut(team).unwrap() = target;
    });
    assert_eq!(payload(&sandbox.path("OUT18")), expected);
    let written = EditFile::load(&sandbox.path("OUT18")).unwrap();
    let slot_1 = written.team(team).unwrap().roster[0].player_id;
    assert_eq!(
        written.player(slot_1).unwrap().motion.goal_celebration_1,
        127,
        "the motion above the target's top is written at it"
    );
}

// SE-TOML-06
#[test]
fn a_name_longer_than_the_field_crops_to_it_with_a_note() {
    let sandbox = Sandbox::new("import_name_too_long");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    fs::write(
        sandbox.path("long.toml"),
        // PES 21's name field holds 61 bytes.
        format!(
            "[team]\nid = {CO}\n\n[players.03]\nname = \"{}\"\n",
            "A".repeat(70)
        ),
    )
    .unwrap();
    let run = sandbox.import("long.toml", "EDIT", "OUT");
    assert_eq!(run.exit_code(), 0, "{:?}", run.messages());
    assert!(
        run.messages().contains(&format!(
            "Warning import_text_cropped (path=players.03.name, cropped={})",
            "A".repeat(61)
        )),
        "{:?}",
        run.messages()
    );
    let written = EditFile::load(&sandbox.path("OUT")).unwrap();
    assert_eq!(
        written.player(CO_SLOT_3).unwrap().name,
        "A".repeat(61),
        "the name is written at the field's whole length"
    );
}

// SE-TOML-08
#[test]
fn a_team_the_save_lacks_or_a_document_with_no_id_is_refused() {
    let sandbox = Sandbox::new("import_no_team");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    fs::write(
        sandbox.path("999.toml"),
        "[team]\nid = 999\nname = \"/zz/\"\n",
    )
    .unwrap();
    sandbox
        .import("999.toml", "EDIT", "OUT")
        .assert_refused(2, &["999"]);
    fs::write(sandbox.path("noid.toml"), "[players.03]\nname = \"X\"\n").unwrap();
    sandbox
        .import("noid.toml", "EDIT", "OUT")
        .assert_refused(2, &["[team] id"]);
    assert!(!sandbox.path("OUT").exists());
}

/// `import-toml` writing in place a document that renames `/co/`'s slot 3.
const IMPORT_TOML: WritingCommand = WritingCommand {
    name: "import_running",
    setup: |sandbox| {
        sandbox.fixture(PesVersion::Pes21, "EDIT");
        let text = format!("[team]\nid = {CO}\n\n[players.03]\nname = \"PATCH\"\n");
        fs::write(sandbox.path("co.toml"), text).unwrap();
    },
    args: |sandbox| {
        let edit = sandbox.arg("EDIT");
        let file = sandbox.arg("co.toml");
        vec![
            "import-toml".to_owned(),
            file,
            "--base".to_owned(),
            edit.clone(),
            "--out".to_owned(),
            edit,
        ]
    },
    saves: &["EDIT"],
    files: &[],
};

// SE-CLI-03
#[test]
fn a_write_is_refused_while_the_saves_own_pes_runs() {
    IMPORT_TOML.assert_refused_while_its_game_runs();
}

// SE-CLI-03
#[test]
fn a_write_goes_ahead_while_its_pes_runs_given_edit_mode_closed() {
    IMPORT_TOML.assert_written_given_edit_mode_closed();
}

// SE-CLI-03
#[test]
fn a_write_goes_ahead_while_only_another_versions_pes_runs() {
    IMPORT_TOML.assert_written_while_another_version_runs();
}
