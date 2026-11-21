//! `export-toml`: one team or every team of a save as Team TOML files.

use std::fs;

use pes_savefile::EditFile;
use pes_savefile::interchange::team_toml::TeamToml;
use pes_version::PesVersion;

use crate::Sandbox;
use crate::TEST_SALT;
use crate::common::cup_save::stock_file;

/// The ids of every team of the PES 21 fixture save, in record order.
fn team_ids(sandbox: &Sandbox) -> Vec<u32> {
    let save = EditFile::load(&sandbox.path("EDIT")).unwrap();
    save.teams().iter().map(|team| team.id).collect()
}

// SE-TOML-02
#[test]
fn all_teams_writes_one_file_per_team_replacing_an_older_one() {
    let sandbox = Sandbox::new("export_all_teams");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    let ids = team_ids(&sandbox);
    assert!(ids.contains(&714), "the fixture holds /co/");
    fs::create_dir_all(sandbox.path("teams")).unwrap();
    fs::write(sandbox.path("teams/714.toml"), "older = true\n").unwrap();

    let run = sandbox.run(&[
        "export-toml",
        &sandbox.arg("EDIT"),
        "--all-teams",
        "--out-dir",
        &sandbox.arg("teams"),
    ]);
    assert_eq!(run.exit_code(), 0);

    let mut written: Vec<String> = fs::read_dir(sandbox.path("teams"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    written.sort();
    let mut expected: Vec<String> = ids.iter().map(|id| format!("{id}.toml")).collect();
    expected.sort();
    assert_eq!(written, expected, "one <id>.toml per team, nothing else");
    // Each file is its own team's: its version line and its `[team]` id line, read as text.
    // Parsing all 220 files (5,143 lines each) took most of a minute in a test build; three
    // are parsed whole: the first, 714 (written over the older file) and the last.
    let value = |line: &str| line.split('#').next().unwrap().trim().to_owned();
    for id in &ids {
        let text = fs::read_to_string(sandbox.path(&format!("teams/{id}.toml"))).unwrap();
        // The file's first line says what it is; the version is the second.
        assert_eq!(
            value(text.lines().nth(1).unwrap()),
            "pes_version = 21",
            "{id}"
        );
        assert!(
            text.lines().any(|line| value(line) == format!("id = {id}")),
            "{id}: no `id = {id}` line"
        );
    }
    for id in [ids[0], 714, ids[ids.len() - 1]] {
        let text = fs::read_to_string(sandbox.path(&format!("teams/{id}.toml"))).unwrap();
        let document = TeamToml::parse(&text).unwrap_or_else(|error| panic!("{id}: {error}"));
        assert_eq!(document.team.id, Some(id));
        assert_eq!(document.pes_version, Some(PesVersion::Pes21));
    }
    assert_eq!(
        run.messages().len(),
        ids.len(),
        "one team_exported line per team: {:?}",
        run.messages()
    );
    assert_eq!(
        run.messages()[0],
        format!(
            "Info team_exported (team={}, path={})",
            ids[0],
            sandbox.path(&format!("teams/{}.toml", ids[0])).display()
        )
    );
}

// A save holding no cup team (one made from the stock game) is refused before
// the folder is created, so nothing is written.
// SE-TOML-11
#[test]
fn all_teams_on_a_save_with_no_cup_team_is_refused_and_writes_nothing() {
    let sandbox = Sandbox::new("export_no_cup_team");
    let file = stock_file(PesVersion::Pes21);
    fs::write(sandbox.path("EDIT"), file.to_bytes(&TEST_SALT).unwrap()).unwrap();

    let run = sandbox.run(&[
        "export-toml",
        &sandbox.arg("EDIT"),
        "--all-teams",
        "--out-dir",
        &sandbox.arg("teams"),
    ]);
    run.assert_refused(2, &["the save holds no cup team (ids 701 to 920)"]);
    assert!(!sandbox.path("teams").exists(), "no folder created");
}

// SE-TOML-03
#[test]
fn a_team_the_save_lacks_or_both_forms_is_refused_with_nothing_written() {
    let sandbox = Sandbox::new("export_refusals");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    assert!(!team_ids(&sandbox).contains(&999));

    let run = sandbox.run(&[
        "export-toml",
        &sandbox.arg("EDIT"),
        "--team",
        "999",
        "--out",
        &sandbox.arg("999.toml"),
    ]);
    run.assert_refused(2, &["999"]);
    assert!(!sandbox.path("999.toml").exists());

    let run = sandbox.run(&[
        "export-toml",
        &sandbox.arg("EDIT"),
        "--team",
        "714",
        "--out",
        &sandbox.arg("714.toml"),
        "--all-teams",
        "--out-dir",
        &sandbox.arg("teams"),
    ]);
    run.assert_refused(2, &["--all-teams"]);
    assert!(!sandbox.path("714.toml").exists());
    assert!(!sandbox.path("teams").exists());
}

// SE-TOML-09
#[test]
fn an_out_naming_the_save_itself_is_refused_and_the_save_unchanged() {
    let sandbox = Sandbox::new("export_out_is_the_save");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    let bytes = fs::read(sandbox.path("EDIT")).unwrap();

    let run = sandbox.run(&[
        "export-toml",
        &sandbox.arg("EDIT"),
        "--team",
        "714",
        "--out",
        &sandbox.arg("EDIT"),
    ]);
    run.assert_refused(2, &["--out"]);

    // The same file under another spelling is the save too: the compare is on the
    // canonical path, so a subfolder's `..` does not slip past it.
    fs::create_dir(sandbox.path("sub")).unwrap();
    let spelled = sandbox.arg("sub/../EDIT");
    let run = sandbox.run(&[
        "export-toml",
        &sandbox.arg("EDIT"),
        "--team",
        "714",
        "--out",
        &spelled,
    ]);
    run.assert_refused(2, &["--out"]);
    assert_eq!(
        fs::read(sandbox.path("EDIT")).unwrap(),
        bytes,
        "the save is unchanged"
    );
}
