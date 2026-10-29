//! `check-aatf`: an AATF ruleset run over the save's cup teams, the violations as finding
//! lines, and the ruleset's selection (`--rules`, the `aatf_ruleset` setting, the built-in
//! official ruleset) or refusal when a `--rules` file does not load.

use std::fs;
use std::path::Path;

use pes_savefile::EditFile;
use pes_version::PesVersion;

use crate::TEST_SALT;
use crate::common::Sandbox;

/// The `rulesets/<ruleset>.rhai` rules file, assembled the way `aatf`'s `file::assemble`
/// builds a data block that has no CUSTOM CHECKS section: the block trimmed, the current
/// logic trimmed, then the banner line and the empty `custom_checks` stub. `autumn26` is
/// the block `Rules::official` embeds.
fn rules_file(ruleset: &str) -> String {
    let aatf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libs/aatf");
    let data = fs::read_to_string(aatf.join(format!("rulesets/{ruleset}.rhai"))).unwrap();
    let logic = fs::read_to_string(aatf.join("logic/logic.rhai")).unwrap();
    format!(
        "{}\n\n{}\n\n{}\nfn custom_checks(team) {{ [] }}\n",
        data.trim_end(),
        logic.trim_end(),
        "// ==== CUSTOM CHECKS: optional, hand-written; the Ruleset editor keeps this section as it is ===="
    )
}

/// The VGL27 violation team 701 of the PES 21 fixture save carries and the official ruleset
/// cannot produce: VGL27's one height system is `purple`, the official's are `red` and
/// `green`. (The official file now has `fielded_at_a` too, so `FieldedNotA` no longer
/// tells the files apart.)
const PICKED_FILE_VIOLATION: &str = "Error aatf_violation (team=701, rule=Height quota, problem=0 players in the Tall bracket of the Purple system, where its quota is 10)";

/// The official ruleset's marker violation on team 701 of the same save: a `red`
/// quota the VGL27 file cannot produce.
const OFFICIAL_VIOLATION: &str = "Error aatf_violation (team=701, rule=Height quota, problem=0 players in the Tall bracket of the Red system, where its quota is 10)";

/// `check-aatf <save>` with the given extra arguments.
fn check(sandbox: &Sandbox, save: &str, extra: &[&str]) -> crate::common::Run {
    let save = sandbox.arg(save);
    let mut args = vec!["check-aatf", save.as_str()];
    args.extend_from_slice(extra);
    sandbox.run(&args)
}

// SE-AATF-01
#[test]
fn a_teams_violations_print_in_order_and_an_error_exits_1() {
    let sandbox = Sandbox::new("aatf_errors");
    sandbox.fixture(PesVersion::Pes15, "EDIT");

    let run = check(&sandbox, "EDIT", &["--teams", "718"]);
    assert_eq!(run.exit_code(), 1, "{:?}", run.messages());
    // D3/D4: the five Height lines and the tall quota are gone — the
    // five players (regulars registered at CB at the ordinary 185 cm,
    // once "in no bracket" under the allowed height) are tall now, so
    // the red quota holds at 10 of 10.
    assert_eq!(
        run.messages(),
        [
            "Error aatf_violation (team=718, rule=Tier count, problem=2 Gold players, where the rules want 1)",
            "Error aatf_violation (team=718, rule=Tier count, problem=0 Bronze players, where the rules want 2)",
            "Error aatf_violation (team=718, rule=Tier count, problem=19 Regular players, where the rules want 18)",
        ]
    );
}

// SE-AATF-01
#[test]
fn a_warning_only_team_exits_0_and_a_clean_team_prints_nothing() {
    let sandbox = Sandbox::new("aatf_warning_legal");
    sandbox.fixture(PesVersion::Pes15, "EDIT");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/aatf");
    // The embedded file's only warnings were its two 194 cm specials,
    // errors since the cup's 2026-10-22 answers: the warning leg runs a
    // rules file that is the official one with the severity restored.
    fs::write(
        sandbox.path("warning.4ccrules"),
        rules_file("autumn26").replacen(
            "#{ label: \"Goalkeepers below giant\",",
            "#{ label: \"Goalkeepers below giant\", severity: \"warning\",",
            1,
        ),
    )
    .unwrap();
    let warning_rules = sandbox.arg("warning.4ccrules");
    for (file, extra, expected) in [
        (
            "official_warning.toml",
            vec!["--rules", warning_rules.as_str()],
            vec![
                "Warning aatf_violation (team=714, player=71401, rule=Special rule, problem=Goalkeepers below giant: GK heights cannot be 194cm or more)",
            ],
        ),
        ("official_legal.toml", Vec::new(), Vec::new()),
    ] {
        let run = sandbox.run(&[
            "import-toml",
            fixtures.join(file).to_str().unwrap(),
            "--base",
            &sandbox.arg("EDIT"),
            "--out",
            &sandbox.arg("OUT"),
        ]);
        assert_eq!(run.exit_code(), 0, "{file}: {:?}", run.messages());
        let mut args = vec!["--teams", "714"];
        args.extend(extra);
        let run = check(&sandbox, "OUT", &args);
        assert_eq!(run.exit_code(), 0, "{file}");
        assert_eq!(run.messages(), expected, "{file}");
    }
}

/// A line names the rule in words and says what is wrong as the window's rows do, a stat by
/// the game's name: the legal 714 /co/ with its goalkeeper's Attacking Prowess raised from
/// the 77 the official ruleset allows to 82.
// SE-AATF-01
#[test]
fn a_stats_line_names_the_stat_as_the_game_does() {
    let sandbox = Sandbox::new("aatf_stat_line");
    sandbox.fixture(PesVersion::Pes15, "EDIT");
    let legal =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/aatf/official_legal.toml");
    let legal = fs::read_to_string(legal).unwrap();
    let raised = "[players.01.stats]\nattacking_prowess = 82\n";
    let toml = legal.replacen("[players.01.stats]\nattacking_prowess = 77\n", raised, 1);
    assert!(toml.contains(raised), "premise: slot 01's prowess was 77");
    fs::write(sandbox.path("raised.toml"), toml).unwrap();
    let run = sandbox.run(&[
        "import-toml",
        &sandbox.arg("raised.toml"),
        "--base",
        &sandbox.arg("EDIT"),
        "--out",
        &sandbox.arg("OUT"),
    ]);
    assert_eq!(run.exit_code(), 0, "{:?}", run.messages());

    let run = check(&sandbox, "OUT", &["--teams", "714"]);
    assert_eq!(run.exit_code(), 1, "{:?}", run.messages());
    assert_eq!(
        run.messages(),
        ["Error aatf_violation (team=714, player=71401, rule=Stat, \
          problem=Attacking Prowess 82, over the 77 the rules allow)"]
    );
}

// SE-AATF-02
#[test]
fn teams_limits_the_check_to_the_named_teams_in_id_order() {
    let sandbox = Sandbox::new("aatf_teams");
    sandbox.fixture(PesVersion::Pes15, "EDIT");

    let run = check(&sandbox, "EDIT", &["--teams", "718"]);
    assert_eq!(run.exit_code(), 1);
    let messages = run.messages();
    assert!(
        messages.iter().all(|line| line.contains("team=718")),
        "{messages:?}"
    );

    let run = check(&sandbox, "EDIT", &["--teams", "704,718"]);
    assert_eq!(run.exit_code(), 1);
    let messages = run.messages();
    // D6: 42 lines now, the 11 starters' FieldedNotA on top (a
    // placeholder team fielded at positions nobody is rated A at).
    assert_eq!(
        messages
            .iter()
            .filter(|line| line.contains("team=704"))
            .count(),
        42,
        "{messages:?}"
    );
    let last_704 = messages
        .iter()
        .rposition(|line| line.contains("team=704"))
        .unwrap();
    let first_718 = messages
        .iter()
        .position(|line| line.contains("team=718"))
        .unwrap();
    assert!(last_704 < first_718, "704's lines come first: {messages:?}");
    // SE-AATF-01's player field on an error line: 704's goalkeeper,
    // fielded in goal in preset 1 without an A there (rated A nowhere).
    assert!(
        messages.iter().any(|line| line
            == "Error aatf_violation (team=704, player=70401, rule=Fielded position, \
                problem=fielded at GK in preset 1 without an A rating (no position rated A))"),
        "{messages:?}"
    );
}

// SE-AATF-02
#[test]
fn a_teams_id_the_save_lacks_is_refused() {
    let sandbox = Sandbox::new("aatf_no_team");
    sandbox.fixture(PesVersion::Pes15, "EDIT");
    check(&sandbox, "EDIT", &["--teams", "9999"]).assert_refused(2, &["9999"]);
    // A known id beside the missing one: the refusal stands and emits nothing.
    check(&sandbox, "EDIT", &["--teams", "718,9999"]).assert_refused(2, &["9999"]);
}

// SE-AATF-03
#[test]
fn a_rules_file_that_fails_to_load_is_refused_with_its_position() {
    let sandbox = Sandbox::new("aatf_broken_rules");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    fs::write(
        sandbox.path("broken.4ccrules"),
        "const RULESET = #{ name: \"x\",\n",
    )
    .unwrap();

    let run = check(
        &sandbox,
        "EDIT",
        &["--teams", "701", "--rules", &sandbox.arg("broken.4ccrules")],
    );
    run.assert_refused(2, &["broken.4ccrules"]);
    let error = match &run.result {
        Err(error) => error.to_string(),
        Ok(code) => panic!("not refused: exit code {code}"),
    };
    // `LoadError::Parse` displays `{line}:{column}: <message>` after the file's name.
    let mut position = error
        .split("broken.4ccrules: ")
        .nth(1)
        .unwrap_or_else(|| panic!("no position after the file's name: {error}"))
        .split(':');
    let line = position.next().unwrap().trim().parse::<usize>();
    let column = position.next().unwrap().trim().parse::<usize>();
    assert!(line.is_ok() && column.is_ok(), "a line and column: {error}");
    assert!(run.events.is_empty(), "a refused command emits nothing");
}

// SE-AATF-03
#[test]
fn rules_and_the_aatf_ruleset_setting_pick_the_rules() {
    let sandbox = Sandbox::new("aatf_vgl27");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    let rules = sandbox.path("vgl27.4ccrules");
    fs::write(&rules, rules_file("vgl27")).unwrap();

    let arg = rules.to_str().unwrap().to_owned();
    let run = check(
        &sandbox,
        "EDIT",
        &["--teams", "701", "--rules", arg.as_str()],
    );
    assert_eq!(run.exit_code(), 1, "{:?}", run.messages());
    assert!(
        run.messages()
            .iter()
            .any(|line| line == PICKED_FILE_VIOLATION),
        "{:?}",
        run.messages()
    );

    // The setting naming the built-in VGL27.
    let run = sandbox.run_with_settings(
        "[save-editor]\naatf_ruleset = 'vgl27'\n",
        &["check-aatf", &sandbox.arg("EDIT"), "--teams", "701"],
    );
    assert_eq!(run.exit_code(), 1, "{:?}", run.messages());
    assert!(
        run.messages()
            .iter()
            .any(|line| line == PICKED_FILE_VIOLATION),
        "{:?}",
        run.messages()
    );

    // No ruleset picked: the default official ruleset runs, its own marker
    // the `red` quota VGL27 cannot emit.
    let run = check(&sandbox, "EDIT", &["--teams", "701"]);
    assert!(
        run.messages().iter().any(|line| line == OFFICIAL_VIOLATION),
        "the official file's own quota: {:?}",
        run.messages()
    );
}

// SE-AATF-05
#[test]
fn without_teams_every_cup_team_is_checked_in_id_order() {
    let sandbox = Sandbox::new("aatf_default_teams");
    // The fixture holds cup teams alone: team 703's id moved outside the cup range makes
    // the team outside 701 to 920, and it breaks the same rules (`--teams 925` flags it)
    // but the default selection leaves it out.
    let mut file = EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes15)).unwrap();
    // Five cup teams are enough to show the selection and its order, and checking all 220
    // took two minutes in a test build: the others move below the cup range (ids 100 and up,
    // free in this save), where the default selection leaves them out as it does 925.
    let kept = [701, 702, 703, 704, 718, 920];
    let moved: Vec<u32> = file
        .teams()
        .iter()
        .map(|team| team.id)
        .filter(|id| !kept.contains(id))
        .collect();
    for (offset, id) in (100..).zip(moved) {
        file.team_mut(id).unwrap().id = offset;
    }
    file.team_mut(703).unwrap().id = 925;
    // Swapping 704's and 718's ids (through the free id 921) makes record order disagree
    // with id order: the record at 704's place now carries 718, the one at 718's place 704.
    // Both still break the same rules, as the assertions below check.
    file.team_mut(704).unwrap().id = 921;
    file.team_mut(718).unwrap().id = 704;
    file.team_mut(921).unwrap().id = 718;
    fs::write(sandbox.path("EDIT"), file.to_bytes(&TEST_SALT).unwrap()).unwrap();

    let run = check(&sandbox, "EDIT", &["--teams", "925"]);
    assert!(
        run.messages().iter().any(|line| line.contains("team=925")),
        "925 breaks the same rules: {:?}",
        run.messages()
    );

    let run = check(&sandbox, "EDIT", &[]);
    assert_eq!(run.exit_code(), 1);
    let ids: Vec<u32> = run
        .messages()
        .iter()
        .filter_map(|line| {
            line.split("team=")
                .nth(1)?
                .split(|c: char| !c.is_ascii_digit())
                .next()?
                .parse()
                .ok()
        })
        .collect();
    assert!(
        ids.iter().all(|id| (701..=920).contains(id)),
        "an out-of-range team printed: {ids:?}"
    );
    assert!(
        ids.contains(&704) && ids.contains(&718),
        "SE-AATF-02's two teams print: {ids:?}"
    );
    let mut teams = ids.clone();
    teams.dedup();
    // Every cup team left breaks a rule (the fixture's placeholder players do), so each
    // prints, once, in id order: exactly these five, the record order notwithstanding.
    assert_eq!(teams, [701, 702, 704, 718, 920], "team id order: {teams:?}");
}

// SE-AATF-06
#[test]
fn a_check_that_fails_on_one_team_does_not_stop_the_rest() {
    let sandbox = Sandbox::new("aatf_check_failed_continues");
    sandbox.fixture(PesVersion::Pes15, "EDIT");
    let file = EditFile::load(&sandbox.path("EDIT")).unwrap();
    // The fixture's one captain-19 team is 743; 744's is 22, so the custom check below
    // fails on 743 alone (the sample team's captain is 10: the file still loads).
    assert_eq!(file.team(743).unwrap().tactics.set_pieces.captain, 19);
    assert_eq!(file.team(744).unwrap().tactics.set_pieces.captain, 22);
    fs::write(
        sandbox.path("throw19.4ccrules"),
        rules_file("autumn26").replacen(
            "fn custom_checks(team) { [] }",
            "fn custom_checks(team) { if team.captain == 19 { throw \"boom\" } [] }",
            1,
        ),
    )
    .unwrap();

    let run = check(
        &sandbox,
        "EDIT",
        &[
            "--teams",
            "743,744",
            "--rules",
            &sandbox.arg("throw19.4ccrules"),
        ],
    );
    assert_eq!(run.exit_code(), 1, "{:?}", run.messages());
    let failed = run
        .messages()
        .iter()
        .position(|line| line.starts_with("Error aatf_check_failed (team=743,"))
        .unwrap_or_else(|| panic!("743's check failed line: {:?}", run.messages()));
    assert!(
        run.messages()[failed].contains("boom"),
        "{:?}",
        run.messages()
    );
    // After the failure line: teams print in id order, a failed one in its place.
    assert!(
        run.messages()[failed + 1..]
            .iter()
            .any(|line| line.contains("aatf_violation (team=744,")),
        "744's violations still print, after 743's failure: {:?}",
        run.messages()
    );
}

// SE-AATF-07
#[test]
fn rules_wins_over_the_aatf_ruleset_setting() {
    let sandbox = Sandbox::new("aatf_rules_flag_wins");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    fs::write(sandbox.path("official.4ccrules"), rules_file("autumn26")).unwrap();

    // The setting names VGL27; --rules names the official file and is the one checked.
    let run = sandbox.run_with_settings(
        "[save-editor]\naatf_ruleset = 'vgl27'\n",
        &[
            "check-aatf",
            &sandbox.arg("EDIT"),
            "--teams",
            "701",
            "--rules",
            &sandbox.arg("official.4ccrules"),
        ],
    );
    // A run that checked (0 or 1), not a refused one: a refusal prints no rule line either.
    // The official file's `red` quota proves it is the one checked (D6 gave it
    // `fielded_at_a` too, so FieldedNotA no longer tells the files apart).
    assert!(run.exit_code() < 2, "{:?}", run.messages());
    assert!(
        run.messages().iter().any(|line| line == OFFICIAL_VIOLATION),
        "the official file's own quota: {:?}",
        run.messages()
    );
}

// SE-AATF-01
#[test]
fn a_roster_order_break_reports_the_held_and_expected_ids() {
    let sandbox = Sandbox::new("aatf_roster_order");
    // Team 701's roster slots 0 and 1 with their player ids swapped.
    let mut file = EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
    let roster = &mut file.team_mut(701).unwrap().roster;
    assert_eq!(
        roster[0].player_id, 70101,
        "premise: slot 0 holds its player"
    );
    assert_eq!(
        roster[1].player_id, 70102,
        "premise: slot 1 holds its player"
    );
    (roster[0].player_id, roster[1].player_id) = (70102, 70101);
    fs::write(sandbox.path("EDIT"), file.to_bytes(&TEST_SALT).unwrap()).unwrap();

    let run = check(&sandbox, "EDIT", &["--teams", "701"]);
    assert_eq!(run.exit_code(), 1, "{:?}", run.messages());
    let messages = run.messages();
    // Each slot's break leads its own rows: slot 0's is the first line,
    // slot 1's follows slot 0's script violations (`check_team` lists the
    // players by slot, `spec/aatf_rules.md` "The host").
    let breaks: Vec<&str> = messages
        .iter()
        .filter(|line| line.contains("rule=Roster order"))
        .map(String::as_str)
        .collect();
    assert_eq!(
        breaks,
        [
            "Error aatf_violation (team=701, player=70102, rule=Roster order, \
             problem=the slot holds player 70102, not 70101)",
            "Error aatf_violation (team=701, player=70101, rule=Roster order, \
             problem=the slot holds player 70101, not 70102)",
        ],
        "{messages:?}"
    );
    assert_eq!(
        messages[0], breaks[0],
        "slot 0's break is the first line: {messages:?}"
    );
    assert_ne!(
        messages[1], breaks[1],
        "slot 0's own rows come before slot 1's break: {messages:?}"
    );
}

// SE-AATF-04
#[test]
fn an_imported_aatf_ruleset_is_read_from_the_data_directorys_rules_folder() {
    let sandbox = Sandbox::new("aatf_rules_relative");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    fs::create_dir_all(sandbox.path("data/rules")).unwrap();
    fs::write(sandbox.path("data/rules/cup.4ccrules"), rules_file("vgl27")).unwrap();

    // The sandbox runs the command in-process, so its current directory is the test
    // process's, which holds no cup.4ccrules: the copy is found only in the data
    // directory's `rules` folder.
    let run = sandbox.run_with_settings(
        "[save-editor]\naatf_ruleset = 'cup.4ccrules'\n",
        &["check-aatf", &sandbox.arg("EDIT"), "--teams", "701"],
    );
    assert_eq!(run.exit_code(), 1, "{:?}", run.messages());
    assert!(
        run.messages()
            .iter()
            .any(|line| line == PICKED_FILE_VIOLATION),
        "{:?}",
        run.messages()
    );
}

/// Without `--rules`, a settings file naming the built-in `vtl11` checks against VTL11:
/// the run prints VTL11's own violation, which the official ruleset does not.
// SE-GUI-32
#[test]
fn the_aatf_ruleset_setting_naming_vtl11_checks_against_vtl11() {
    /// VTL11's captain rule broken by team 701's captain, a Regular: a special the
    /// official ruleset does not have.
    const VTL11_VIOLATION: &str = "Error aatf_violation (team=701, player=70101, rule=Special rule, problem=Regulars cannot be captain: Regulars can't be captain)";

    let sandbox = Sandbox::new("aatf_ruleset_vtl11");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    let run = sandbox.run_with_settings(
        "[save-editor]\naatf_ruleset = 'vtl11'\n",
        &["check-aatf", &sandbox.arg("EDIT"), "--teams", "701"],
    );
    assert_eq!(run.exit_code(), 1, "{:?}", run.messages());
    assert!(
        run.messages().iter().any(|line| line == VTL11_VIOLATION),
        "VTL11's captain rule: {:?}",
        run.messages()
    );

    let official = check(&sandbox, "EDIT", &["--teams", "701"]);
    assert!(
        !official
            .messages()
            .iter()
            .any(|line| line == VTL11_VIOLATION),
        "premise: the official ruleset does not print it"
    );
}

/// A setting naming an imported copy that is gone: the check runs under the official
/// ruleset and the report carries the warning line saying so.
// SE-GUI-32, SE-AATF-04
#[test]
fn a_gone_aatf_ruleset_warns_and_checks_against_the_official_ruleset() {
    let sandbox = Sandbox::new("aatf_ruleset_gone");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    let run = sandbox.run_with_settings(
        "[save-editor]\naatf_ruleset = 'gone.4ccrules'\n",
        &["check-aatf", &sandbox.arg("EDIT"), "--teams", "701"],
    );
    assert_eq!(run.exit_code(), 1, "{:?}", run.messages());
    assert!(
        run.messages()
            .iter()
            .any(|line| line
                == "Warning aatf_ruleset_gone (ruleset=gone.4ccrules, used=4cc Autumn 26)"),
        "the warning line: {:?}",
        run.messages()
    );
    assert!(
        run.messages().iter().any(|line| line == OFFICIAL_VIOLATION),
        "the official ruleset's own quota: {:?}",
        run.messages()
    );
}
