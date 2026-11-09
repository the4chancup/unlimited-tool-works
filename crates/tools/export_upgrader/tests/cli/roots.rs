//! Team identity and the root files (the `spec/export_upgrader.md` "Team identity and root files"
//! scenarios): the Note's `Team:` line first, the export name's first
//! word as the fallback.

use std::fs;

use pes_version::PesVersion;
use studio_core::PipelineEvent;

use crate::common::Sandbox;
use crate::players::{face, wears};

/// `upgrade <input> --savefile <save>`.
fn upgrade(sandbox: &Sandbox, input: &str, save: &str) -> crate::common::Run {
    let input = sandbox.arg(input);
    let save = sandbox.arg(save);
    sandbox.run(&["upgrade", &input, "--savefile", &save])
}

// EU-ROOT-01
#[test]
fn a_note_team_line_overrides_the_exports_name() {
    let sandbox = Sandbox::new("root_note_team");
    // "Numbers" holds the same export as the `co Spring` fixture: its Note's
    // Team: line names /co/. A face folder gives the output a player folder
    // for the settings.toml half of the scenario.
    sandbox.copy_fixture_dir("co Spring", "Numbers");
    sandbox.write("Numbers/Faces/XXX05 - PLACEHOLDER/fcl_hair.fmdl", b"hair");
    // Slot 05's record carries the boots ID, so his generated settings.toml
    // proves it came from /co/'s records. A stock model's ID: one in the
    // team's old block (426-450) names an export's own boots and is not
    // written.
    sandbox.save_edited("EDIT", PesVersion::Pes21, |file| wears(file, 71405, 42, 0));

    let run = upgrade(&sandbox, "Numbers", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(sandbox.path("co Full Numbers").is_dir(), "{}", run.stdout());
    // Every output player folder holds its settings.toml, generated from
    // /co/'s records in the save.
    let players = sandbox.listing("co Full Numbers/Players");
    assert!(!players.is_empty(), "the output holds a player folder");
    for folder in players.iter().filter(|path| !path.contains('/')) {
        assert!(
            sandbox
                .path(&format!("co Full Numbers/Players/{folder}/settings.toml"))
                .is_file(),
            "{folder}"
        );
    }
    let text =
        fs::read_to_string(sandbox.path("co Full Numbers/Players/05 - PLACEHOLDER/settings.toml"))
            .unwrap();
    assert!(
        text.contains("boots_id = 42"),
        "slot 05's record's ID: {text}"
    );
}

// EU-ROOT-01
#[test]
fn a_note_only_export_reports_no_unmigrated_content() {
    let sandbox = Sandbox::new("root_note_only");
    // A `Team:` Note alone makes an old export; there is nothing else to
    // migrate or leave behind.
    fs::create_dir_all(sandbox.path("Numbers")).unwrap();
    fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/co Spring/co Note.txt"),
        sandbox.path("Numbers/co Note.txt"),
    )
    .unwrap();
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "Numbers", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        !run.stdout().contains("unmigrated_content"),
        "{}",
        run.stdout()
    );
    assert!(
        run.events
            .iter()
            .all(|envelope| !matches!(envelope.event, PipelineEvent::UnmigratedContentFound)),
        "no UnmigratedContentFound when nothing is unmigrated"
    );
    assert!(sandbox.path("co Full Numbers").is_dir(), "{}", run.stdout());
}

// EU-ROOT-02
#[test]
fn without_a_note_the_first_word_resolves() {
    let sandbox = Sandbox::new("root_no_note");
    fs::create_dir_all(sandbox.path("co Spring 26/Other")).unwrap();
    fs::write(
        sandbox.path("co Spring 26/Other/readme.txt"),
        b"not migrated",
    )
    .unwrap();
    // A face folder, so the run is clean and the output is written at all.
    face(&sandbox, "co Spring 26/Faces/XXX05 - A");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring 26", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let output = sandbox.path("co Full Spring 26");
    assert!(output.is_dir(), "{}", run.stdout());
    // No colors.txt, notes.txt or Note txt: the export held no Note.
    for entry in sandbox.listing("co Full Spring 26") {
        let lowered = entry.to_ascii_lowercase();
        assert!(
            lowered != "colors.txt" && !(lowered.ends_with(".txt") && lowered.contains("note")),
            "{entry}"
        );
    }
}

// EU-ROOT-04
#[test]
fn the_notes_team_colors_and_other_notes_become_the_root_files() {
    let sandbox = Sandbox::new("root_files");
    sandbox.copy_fixture_dir("co Spring", "co Spring");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/colors.txt")).unwrap(),
        b"048 146 082\n221 000 000\n000 000 000\n000 000 000\n"
    );
    // The Note's two lines after `Other Notes:`, joined by `\n`, CRLF dropped,
    // a final `\n` added.
    let note = fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/co Spring/co Note.txt"),
    )
    .unwrap();
    let note_text = String::from_utf8_lossy(&note);
    let after = note_text.split("Other Notes:").nth(1).unwrap();
    let expected = format!("{}\n", after.trim().replace("\r\n", "\n"));
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/notes.txt")).unwrap(),
        expected.as_bytes()
    );
    // The output holds no `.txt` whose name holds `note` other than
    // `notes.txt`: the Note itself is not written.
    let note_txts: Vec<String> = sandbox
        .listing("co Full Spring")
        .into_iter()
        .filter(|path| {
            let lowered = path.to_ascii_lowercase();
            lowered.ends_with(".txt") && lowered.contains("note")
        })
        .collect();
    assert_eq!(note_txts, ["notes.txt"], "{note_txts:?}");
}

// EU-ROOT-06
#[test]
fn a_team_entry_whose_colors_do_not_read_is_unmigrated_content() {
    let sandbox = Sandbox::new("root_bad_entry");
    // The export holds the Note alone; the unreadable entry is reported and
    // nothing is written.
    fs::create_dir_all(sandbox.path("co Spring")).unwrap();
    fs::write(
        sandbox.path("co Spring/co Note.txt"),
        "Team: /co/\r\n\r\nTeam Colours:\r\n- 1st: 048 146 082\r\n- 2nd: blue\r\n- 3rd: 000 000 000\r\n",
    )
    .unwrap();
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 1);
    assert!(
        run.stdout()
            .contains("- co Spring: Error unmigrated_content at co Note.txt (reason=colors_unreadable, line=- 2nd: blue)"),
        "{}",
        run.stdout()
    );
    let found = run
        .events
        .iter()
        .filter(|envelope| matches!(envelope.event, PipelineEvent::UnmigratedContentFound))
        .count();
    assert_eq!(found, 1, "one UnmigratedContentFound");
    sandbox.assert_nothing_written("co Full Spring", &run.stdout());
}

// EU-ROOT-06
#[test]
fn a_team_entry_with_nothing_after_its_colon_is_unmigrated_content() {
    let sandbox = Sandbox::new("root_empty_entry");
    // `- 2nd:`, an AET template line left unfilled, reads as no color.
    fs::create_dir_all(sandbox.path("co Spring")).unwrap();
    fs::write(
        sandbox.path("co Spring/co Note.txt"),
        "Team: /co/\r\n\r\nTeam Colours:\r\n- 1st: 048 146 082\r\n- 2nd:\r\n",
    )
    .unwrap();
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 1);
    assert!(
        run.stdout()
            .contains("- co Spring: Error unmigrated_content at co Note.txt (reason=colors_unreadable, line=- 2nd:)"),
        "{}",
        run.stdout()
    );
    sandbox.assert_nothing_written("co Full Spring", &run.stdout());
}

// EU-ROOT-07
#[test]
fn a_team_entry_holding_two_colors_keeps_the_first() {
    let sandbox = Sandbox::new("root_two_colors");
    // `- 1st:`'s second color, a kit entry's spelling, is a leftover; the
    // run stays clean.
    fs::create_dir_all(sandbox.path("co Spring")).unwrap();
    fs::write(
        sandbox.path("co Spring/co Note.txt"),
        "Team: /co/\r\n\r\nTeam Colours:\r\n- 1st: 048 146 082 - 255 255 255\r\n- 2nd: #FFFFFF\r\n",
    )
    .unwrap();
    // A face folder, so the output holds content at all.
    face(&sandbox, "co Spring/Faces/XXX05 - A");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/colors.txt")).unwrap(),
        b"048 146 082\n#FFFFFF\n"
    );
    assert!(
        run.stdout().contains(
            "- co Spring: Warning leftover_dropped at co Note.txt (line=- 1st: 048 146 082 - 255 255 255)"
        ),
        "{}",
        run.stdout()
    );
}

// EU-ROOT-05
#[test]
fn a_shift_jis_notes_ascii_colors_read() {
    let sandbox = Sandbox::new("root_jp_note");
    sandbox.copy_fixture_dir("jp Summer 18", "jp Summer 18");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "jp Summer 18", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        fs::read(sandbox.path("jp Full Summer 18/colors.txt")).unwrap(),
        b"058 080 177\n255 255 255\n"
    );
    // The kit entries' colors, each written as the Note spells it.
    for (slot, colors) in [
        ("p1", "#212D62\n#DE344F\n"),
        ("p2", "#EEEEEE\n#70C8DE\n"),
        ("p3", "#050505\n#EFD0C3\n"),
        ("p4", "#D6384A\n#4466A7\n"),
        ("p5", "#000400\n#7B0000\n"),
        ("g1", "#FFFFFF\n#000000\n"),
    ] {
        assert_eq!(
            fs::read(sandbox.path(&format!("jp Full Summer 18/Kits/{slot}/colors.txt"))).unwrap(),
            colors.as_bytes(),
            "{slot}"
        );
    }
    // The "Other Notes:" section is empty: no notes.txt.
    assert!(!sandbox.path("jp Full Summer 18/notes.txt").exists());
    assert!(
        !run.stdout().contains("unmigrated_content"),
        "{}",
        run.stdout()
    );
}

// EU-ROOT-08
#[test]
fn a_team_no_list_holds_upgrades_and_the_save_s_own_id_decides() {
    let sandbox = Sandbox::new("root_no_list_team");
    // A teams list holding /co/ at 714 and no /hist/ at all: it is not
    // read — /hist/ upgrades all the same, and /co/'s roster is read from
    // the save's own numbering (750), not the list's.
    sandbox.write("data/teams_list.txt", b"ID\tName\n714\t/co/\n");
    fs::create_dir_all(sandbox.path("hist Spring 26/Faces/XXX05")).unwrap();
    fs::write(
        sandbox.path("hist Spring 26/hist Note.txt"),
        b"Team: /hist/\r\n",
    )
    .unwrap();
    fs::create_dir_all(sandbox.path("co Spring 26/Faces/XXX05")).unwrap();
    sandbox.save_edited("EDIT", PesVersion::Pes21, |file| {
        // The fixture's /co/ sits at 714: renumber it away so the save
        // holds /co/ at 750 alone, /hist/ at 760 alone.
        file.team_mut(714).unwrap().name = "/old/".to_owned();
        file.team_mut(750).unwrap().name = "/co/".to_owned();
        file.team_mut(760).unwrap().name = "/hist/".to_owned();
        file.player_mut(76005).unwrap().name = "Ann".to_owned();
        file.player_mut(75005).unwrap().name = "Bob".to_owned();
        // The two teams' worn custom IDs, or `custom_model_missing` is
        // all the run reports.
        for id in 75001..=75023 {
            crate::players::wears(file, id, 0, 0);
        }
        for id in 76001..=76023 {
            crate::players::wears(file, id, 0, 0);
        }
    });

    for (input, output, player) in [
        ("hist Spring 26", "hist Full Spring 26", "05 - Ann"),
        ("co Spring 26", "co Full Spring 26", "05 - Bob"),
    ] {
        let run = upgrade(&sandbox, input, "EDIT");
        assert_eq!(run.exit_code(), 0, "{input}: {}", run.stdout());
        assert!(
            sandbox.path(&format!("{output}/Players/{player}")).is_dir(),
            "{input}: {}",
            run.stdout()
        );
    }
}

// EU-ROOT-09
#[test]
fn a_value_or_name_that_is_no_team_name_is_reported_nothing_written() {
    let sandbox = Sandbox::new("root_no_team_name");
    // "--- .zip" holds an old export (`Other/`), no Note; `---` folds to
    // no team name.
    let options = zip::write::SimpleFileOptions::default();
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer.start_file("---/Other/readme.txt", options).unwrap();
    std::io::Write::write_all(&mut writer, b"x").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    sandbox.write("--- .zip", &bytes);
    // "co Spring 26"'s Note gives `co op`, no team name either.
    fs::create_dir_all(sandbox.path("co Spring 26")).unwrap();
    fs::write(
        sandbox.path("co Spring 26/co Note.txt"),
        b"Team: co op\r\n\r\nTeam Colours:\r\n- 1st: 048 146 082",
    )
    .unwrap();
    sandbox.save("EDIT");
    let before = sandbox.listing("");

    let run = upgrade(&sandbox, "--- .zip", "EDIT");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    // The archive's stem is `--- `, its trailing space kept in the export's
    // label: it folds to no team name.
    assert!(
        run.stdout()
            .contains("- --- : Error team_name_unknown (name=---)\n"),
        "{}",
        run.stdout()
    );

    let run = upgrade(&sandbox, "co Spring 26", "EDIT");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    assert!(
        run.stdout()
            .contains("- co Spring 26: Error team_name_unknown at co Note.txt (name=co op)"),
        "{}",
        run.stdout()
    );

    assert_eq!(sandbox.listing(""), before, "nothing is written");
}

// EU-ROOT-10
#[test]
fn the_first_save_team_of_the_name_is_read_the_later_reported() {
    let sandbox = Sandbox::new("root_repeated_team");
    fs::create_dir_all(sandbox.path("co Spring 26/Faces/XXX05")).unwrap();
    sandbox.save_edited("EDIT", PesVersion::Pes21, |file| {
        // /co/ twice: 750 first in record order (its entry swapped before
        // 714's, which the fixture keeps at index 13 against 49), 714
        // later.
        let (first, later) = (
            file.teams().iter().position(|team| team.id == 750).unwrap(),
            file.teams().iter().position(|team| team.id == 714).unwrap(),
        );
        file.teams_mut().swap(first, later);
        file.team_mut(750).unwrap().name = "/co/".to_owned();
        file.player_mut(75005).unwrap().name = "Bob".to_owned();
        file.player_mut(71405).unwrap().name = "Cy".to_owned();
        for id in 75001..=75023 {
            crate::players::wears(file, id, 0, 0);
        }
    });

    let run = upgrade(&sandbox, "co Spring 26", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        run.stdout()
            .contains("- co Spring 26: Warning save_team_name_repeated (team=714, kept=750)"),
        "{}",
        run.stdout()
    );
    // The roster is read from the first of the name: 75005, not 71405.
    assert!(
        sandbox.path("co Full Spring 26/Players/05 - Bob").is_dir(),
        "{}",
        run.stdout()
    );
    assert!(
        !sandbox.path("co Full Spring 26/Players/05 - Cy").exists(),
        "{}",
        run.stdout()
    );
}
