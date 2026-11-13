//! `diff`: the comparator's report over two saves.

use std::fs;

use pes_savefile::EditFile;
use pes_version::PesVersion;

use crate::common::cup_save::{
    CUP_PLAYER, assert_copies_differ, assert_premises, outside_cup_file, outside_players,
    second_copy,
};
use crate::{Sandbox, TEST_SALT};

/// A player id no save holds, so renumbering a player to it leaves him in one save alone.
const NOWHERE: u32 = 9_999_999;

impl Sandbox {
    /// `diff <first> <second> <flags>`.
    fn diff(&self, first: &str, second: &str, flags: &[&str]) -> crate::common::Run {
        let (first, second) = (self.arg(first), self.arg(second));
        let mut args = vec!["diff", &first, &second];
        args.extend(flags);
        self.run(&args)
    }

    /// Two PES 21 saves: `A` is the fixture with player 70110 renumbered to `NOWHERE`
    /// with his place on 701's roster (so 70110 is in `B` alone and `NOWHERE` in `A`
    /// alone, each on a cup roster, which is what the report reads); `B` is the fixture
    /// with 70105 carrying two changes, his boots id and his speed, and a second changed
    /// player, 70106, whose boots id moved by one. The three changed values, old and new.
    fn diff_pair(&self) -> [(u32, u32); 3] {
        // The boots id is a `u32` field, the speed a `u8`; both land as `u32` for the lines.
        let mut a = EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        a.player_mut(70110).unwrap().id = NOWHERE;
        let place = &mut a.team_mut(701).unwrap().roster[9];
        assert_eq!(place.player_id, 70110, "premise: 70110's place");
        place.player_id = NOWHERE;
        fs::write(self.path("A"), a.to_bytes(&TEST_SALT).unwrap()).unwrap();
        let mut b = EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        let boots = &mut b.player_mut(70105).unwrap().appearance.boots_id;
        let boots_change = (*boots, *boots + 1);
        *boots += 1;
        let speed = &mut b.player_mut(70105).unwrap().stats.speed;
        let speed_change = (*speed, (*speed + 1) % 100);
        *speed = speed_change.1;
        let boots2 = &mut b.player_mut(70106).unwrap().appearance.boots_id;
        let boots2_change = (*boots2, *boots2 + 1);
        *boots2 += 1;
        fs::write(self.path("B"), b.to_bytes(&TEST_SALT).unwrap()).unwrap();
        [
            boots_change,
            (u32::from(speed_change.0), u32::from(speed_change.1)),
            boots2_change,
        ]
    }
}

// SE-DIF-01
#[test]
fn each_changed_field_is_a_row_and_the_scopes_keep_their_own() {
    let sandbox = Sandbox::new("diff_scopes");
    let [
        (boots_old, boots_new),
        (speed_old, speed_new),
        (boots2_old, boots2_new),
    ] = sandbox.diff_pair();
    let boots = format!(
        "Info player_diff (player=70105, field=Boots ID, old={boots_old}, new={boots_new})"
    );
    // Both rows are 70105's; the comparator lists a player's fields in schema order, the
    // stat before the appearance field, and the players in the first save's record order,
    // so 70105's rows come before 70106's.
    let speed =
        format!("Info player_diff (player=70105, field=Speed, old={speed_old}, new={speed_new})");
    let boots2 = format!(
        "Info player_diff (player=70106, field=Boots ID, old={boots2_old}, new={boots2_new})"
    );
    let only_in = [
        format!("Info player_only_in (player={NOWHERE}, save=first)"),
        "Info player_only_in (player=70110, save=second)".to_owned(),
    ];

    let run = sandbox.diff("A", "B", &[]);
    assert_eq!(run.exit_code(), 0);
    assert_eq!(
        run.messages(),
        vec![
            speed.clone(),
            boots.clone(),
            boots2.clone(),
            only_in[0].clone(),
            only_in[1].clone()
        ]
    );
    let run = sandbox.diff("A", "B", &["--aesthetics"]);
    assert_eq!(run.exit_code(), 0);
    assert_eq!(
        run.messages(),
        vec![boots, boots2, only_in[0].clone(), only_in[1].clone()]
    );
    let run = sandbox.diff("A", "B", &["--gameplay"]);
    assert_eq!(run.exit_code(), 0);
    assert_eq!(
        run.messages(),
        vec![speed, only_in[0].clone(), only_in[1].clone()]
    );
    sandbox
        .diff("A", "B", &["--aesthetics", "--gameplay"])
        .assert_refused(2, &["--gameplay"]);
}

// SE-DIF-06
#[test]
fn the_text_names_a_field_as_its_view_does_and_json_keeps_the_schema_name() {
    let sandbox = Sandbox::new("diff_view_label");
    sandbox.fixture(PesVersion::Pes17, "A");
    let mut file = EditFile::load(&sandbox.path("A")).unwrap();
    let player = file.player_mut(73401).unwrap();
    assert_eq!(player.stats.heading, 77, "premise: 73401's heading");
    player.stats.heading = 80;
    fs::write(sandbox.path("B"), file.to_bytes(&TEST_SALT).unwrap()).unwrap();

    // PES 2017's view names the schema's `Heading` Header.
    let run = sandbox.diff("A", "B", &[]);
    assert_eq!(run.exit_code(), 0);
    assert_eq!(
        run.messages(),
        vec!["Info player_diff (player=73401, field=Header, old=77, new=80)"]
    );

    let run = sandbox.diff("A", "B", &["--json"]);
    assert_eq!(run.exit_code(), 0);
    let stdout = run.stdout();
    let report: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|error| panic!("stdout does not parse as JSON: {error}\n{stdout}"));
    assert_eq!(
        report["players"],
        serde_json::json!([{
            "id": 73401,
            "rows": [{ "field": "Heading", "old": "77", "new": "80" }]
        }])
    );
}

/// SE-GUI-77's save on PES 21 as `A` and its second copy as `B`: team 300's name, its first
/// player's Height, the unrostered player's Height and 70101's Height differ. The report,
/// text and JSON, holds 70101's Height alone.
// SE-DIF-07
#[test]
fn a_team_outside_the_cup_and_a_player_on_no_cup_roster_are_not_reported() {
    let sandbox = Sandbox::new("diff_outside_cup");
    let (file, unrostered) = outside_cup_file(PesVersion::Pes21);
    fs::write(sandbox.path("A"), file.to_bytes(&TEST_SALT).unwrap()).unwrap();
    let copy = second_copy(&file, unrostered);
    fs::write(sandbox.path("B"), copy.to_bytes(&TEST_SALT).unwrap()).unwrap();
    let (a, b) = (
        EditFile::load(&sandbox.path("A")).unwrap(),
        EditFile::load(&sandbox.path("B")).unwrap(),
    );
    assert_premises(&a, unrostered);
    assert_copies_differ(&a, &b, unrostered);
    let outside = outside_players(&a)[0];
    let height = a.player(CUP_PLAYER).unwrap().basic.height;

    let run = sandbox.diff("A", "B", &[]);
    assert_eq!(run.exit_code(), 0);
    assert_eq!(
        run.messages(),
        vec![format!(
            "Info player_diff (player={CUP_PLAYER}, field=Height, old={height}, new={})",
            height + 1
        )],
        "70101's Height alone, nothing of {outside} or {unrostered}"
    );

    let run = sandbox.diff("A", "B", &["--json"]);
    assert_eq!(run.exit_code(), 0);
    let stdout = run.stdout();
    let report: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|error| panic!("stdout does not parse as JSON: {error}\n{stdout}"));
    assert_eq!(
        report,
        serde_json::json!({
            "players": [{
                "id": CUP_PLAYER,
                "rows": [{
                    "field": "Height",
                    "old": height.to_string(),
                    "new": (height + 1).to_string()
                }]
            }],
            "only_in_first": [],
            "only_in_second": []
        })
    );
}

// SE-DIF-02
#[test]
fn json_flag_writes_the_same_report_as_one_document() {
    let sandbox = Sandbox::new("diff_json");
    let [
        (boots_old, boots_new),
        (speed_old, speed_new),
        (boots2_old, boots2_new),
    ] = sandbox.diff_pair();

    let run = sandbox.diff("A", "B", &["--json"]);
    assert_eq!(run.exit_code(), 0);
    let stdout = run.stdout();
    let report: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|error| panic!("stdout does not parse as JSON: {error}\n{stdout}"));
    assert_eq!(
        report["players"],
        serde_json::json!([
            {
                "id": 70105,
                "rows": [
                    { "field": "Speed", "old": speed_old.to_string(), "new": speed_new.to_string() },
                    { "field": "BootsId", "old": boots_old.to_string(), "new": boots_new.to_string() }
                ]
            },
            {
                "id": 70106,
                "rows": [
                    { "field": "BootsId", "old": boots2_old.to_string(), "new": boots2_new.to_string() }
                ]
            },
        ])
    );
    assert_eq!(
        report["only_in_first"],
        serde_json::json!([{ "id": NOWHERE }])
    );
    assert_eq!(
        report["only_in_second"],
        serde_json::json!([{ "id": 70110 }])
    );
    for line in stdout.lines() {
        assert!(
            !line.starts_with("- ") && !line.starts_with("Info "),
            "a finding line in the JSON output: {line}"
        );
    }
}

// SE-DIF-02
#[test]
fn json_report_keeps_the_scope() {
    let sandbox = Sandbox::new("diff_json_scope");
    let [
        (boots_old, boots_new),
        (speed_old, speed_new),
        (boots2_old, boots2_new),
    ] = sandbox.diff_pair();

    for (flag, players) in [
        (
            "--aesthetics",
            serde_json::json!([
                {
                    "id": 70105,
                    "rows": [{ "field": "BootsId", "old": boots_old.to_string(), "new": boots_new.to_string() }]
                },
                {
                    "id": 70106,
                    "rows": [{ "field": "BootsId", "old": boots2_old.to_string(), "new": boots2_new.to_string() }]
                }
            ]),
        ),
        (
            "--gameplay",
            serde_json::json!([{
                "id": 70105,
                "rows": [{ "field": "Speed", "old": speed_old.to_string(), "new": speed_new.to_string() }]
            }]),
        ),
    ] {
        let run = sandbox.diff("A", "B", &["--json", flag]);
        assert_eq!(run.exit_code(), 0);
        let stdout = run.stdout();
        let report: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|error| panic!("stdout does not parse as JSON: {error}\n{stdout}"));
        assert_eq!(report["players"], players);
        assert_eq!(
            report["only_in_first"],
            serde_json::json!([{ "id": NOWHERE }])
        );
        assert_eq!(
            report["only_in_second"],
            serde_json::json!([{ "id": 70110 }])
        );
    }
}

// SE-DIF-03
#[test]
fn two_versions_are_refused_naming_both() {
    let sandbox = Sandbox::new("diff_versions");
    sandbox.fixture(PesVersion::Pes21, "EDIT21");
    sandbox.fixture(PesVersion::Pes19, "EDIT19");
    sandbox
        .diff("EDIT21", "EDIT19", &[])
        .assert_refused(2, &["PES 2021", "PES 2019"]);
}

// SE-DIF-05
#[test]
fn the_appearance_id_is_an_ordinary_row() {
    let sandbox = Sandbox::new("diff_appearance_id");
    sandbox.fixture(PesVersion::Pes21, "OWN");
    let mut file = EditFile::load(&sandbox.path("OWN")).unwrap();
    let player = file.player_mut(70101).unwrap();
    assert_eq!(player.appearance_id, Some(70101));
    // The game's -1.
    player.appearance_id = Some(u32::MAX);
    fs::write(
        sandbox.path("MINUS_ONE"),
        file.to_bytes(&TEST_SALT).unwrap(),
    )
    .unwrap();

    let run = sandbox.diff("OWN", "MINUS_ONE", &[]);
    assert_eq!(run.exit_code(), 0);
    assert_eq!(
        run.messages(),
        vec!["Info player_diff (player=70101, field=Appearance ID, old=70101, new=4294967295)"]
    );

    let run = sandbox.diff("OWN", "MINUS_ONE", &["--json"]);
    assert_eq!(run.exit_code(), 0);
    let stdout = run.stdout();
    let report: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|error| panic!("stdout does not parse as JSON: {error}\n{stdout}"));
    assert_eq!(
        report["players"],
        serde_json::json!([{
            "id": 70101,
            "rows": [{ "field": "AppearanceId", "old": "70101", "new": "4294967295" }]
        }])
    );
}
