use super::*;
use crate::interchange::team_toml::labels::{PLAYABLE_POSITIONS, POSITIONS};

/// Overalls read in game, with each player's abilities and positions
/// (`tests/fixtures/README.md` "The overall formula's fixture").
const POINTS: &str = include_str!("../../../tests/fixtures/overall_points.tsv");

/// One fixture row: its version, the player, the position byte, the overall
/// the game showed and the one PES 2016's formula gives.
struct Point {
    version: PesVersion,
    player: PlayerEntry,
    position: u8,
    shown: u8,
    computed: u8,
}

fn position_byte(name: &str) -> u8 {
    let index = POSITIONS
        .iter()
        .position(|p| *p == name)
        .unwrap_or_else(|| panic!("no position {name}"));
    u8::try_from(index).expect("thirteen positions")
}

fn points() -> Vec<Point> {
    let mut lines = POINTS.lines();
    let header: Vec<&str> = lines.next().expect("a header").split('\t').collect();
    lines
        .map(|line| {
            let cells: Vec<&str> = line.split('\t').collect();
            assert_eq!(cells.len(), header.len(), "{line}");
            let cell = |name: &str| {
                let at = header
                    .iter()
                    .position(|h| *h == name)
                    .unwrap_or_else(|| panic!("no column {name}"));
                cells[at]
            };
            let number = |name: &str| -> u8 { cell(name).parse().expect("a number") };
            let mut player = PlayerEntry::default();
            let s = &mut player.stats;
            s.attacking_prowess = number("attacking_prowess");
            s.ball_control = number("ball_control");
            s.dribbling = number("dribbling");
            s.low_pass = number("low_pass");
            s.lofted_pass = number("lofted_pass");
            s.finishing = number("finishing");
            s.place_kicking = number("place_kicking");
            s.kicking_power = number("kicking_power");
            s.heading = number("heading");
            s.defensive_prowess = number("defensive_prowess");
            s.ball_winning = number("ball_winning");
            s.swerve = number("swerve");
            s.speed = number("speed");
            s.explosive_power = number("explosive_power");
            s.body_control = number("body_control");
            s.physical_contact = Some(number("physical_contact"));
            s.jump = number("jump");
            s.goalkeeping = number("goalkeeping");
            s.coverage = Some(number("coverage"));
            s.catching = Some(number("catching"));
            s.clearing = Some(number("clearing"));
            s.reflexes = Some(number("reflexes"));
            s.stamina = number("stamina");
            player.positions.registered = position_byte(cell("registered"));
            // The fixture's letters run in the position byte's order;
            // `playable` runs CF first.
            let letters: Vec<char> = cell("playable").chars().collect();
            for (i, position) in PLAYABLE_ORDER.iter().enumerate() {
                player.positions.playable[i] = match letters[usize::from(*position)] {
                    'C' => 0,
                    'B' => 1,
                    'A' => 2,
                    other => panic!("rating {other}"),
                };
            }
            Point {
                version: match cell("version") {
                    "pes17" => PesVersion::Pes17,
                    "pes21" => PesVersion::Pes21,
                    other => panic!("version {other}"),
                },
                player,
                position: position_byte(cell("position")),
                shown: number("shown"),
                computed: number("computed"),
            }
        })
        .collect()
}

// Every overall read in game: PES 2017 shows exactly what the formula gives,
// PES 2021 within 2 of it.
#[test]
fn overalls_read_in_game() {
    let points = points();
    let count = |version: PesVersion| points.iter().filter(|p| p.version == version).count();
    assert_eq!(
        (count(PesVersion::Pes17), count(PesVersion::Pes21)),
        (162, 201)
    );
    for p in &points {
        let got = p.player.overall_at(p.version, p.position);
        assert_eq!(
            got,
            Some(p.computed),
            "{} at {}: registered {}",
            p.version,
            POSITIONS[usize::from(p.position)],
            POSITIONS[usize::from(p.player.positions.registered)]
        );
        if p.version == PesVersion::Pes17 {
            assert_eq!(p.computed, p.shown, "PES 2017 shows the formula's number");
        } else {
            assert!(p.computed.abs_diff(p.shown) <= 2, "PES 2021 within 2");
        }
    }
}

#[test]
fn a_position_past_cf_has_no_overall() {
    let point = points().remove(0);
    assert!(point.player.overall_at(point.version, 12).is_some());
    assert_eq!(point.player.overall_at(point.version, 13), None);
}

#[test]
fn the_playable_order_is_the_labels() {
    let ours: Vec<usize> = PLAYABLE_ORDER.iter().map(|&p| usize::from(p)).collect();
    assert_eq!(ours, PLAYABLE_POSITIONS);
}

// Away from his registered position the rating adds its bonus and nothing
// else: B the B table's, A the A table's, C none. Each position of a real
// player, registered aside.
#[test]
fn a_rating_adds_its_bonus() {
    let base = points()
        .into_iter()
        .find(|p| p.version == PesVersion::Pes17)
        .expect("a PES 2017 point")
        .player;
    for position in 0..13u8 {
        if position == base.positions.registered {
            continue;
        }
        let slot = PLAYABLE_ORDER
            .iter()
            .position(|&p| p == position)
            .expect("every position rated");
        let rated = |rating: u8| {
            let mut player = base.clone();
            player.positions.playable[slot] = rating;
            u32::from(
                player
                    .overall_at(PesVersion::Pes17, position)
                    .expect("a position"),
            )
        };
        let col = usize::from(position);
        assert_eq!(rated(1) - rated(0), RATED_B[col], "B at {position}");
        assert_eq!(rated(2) - rated(0), RATED_A[col], "A at {position}");
    }
}

// A keeper term is the mean of the abilities the version stores: PES 2015's
// record has no clearing, reflexes or coverage, and a keeper whose stored
// ones equal the missing ones rates the same.
#[test]
fn a_keeper_mean_counts_the_stored_abilities() {
    let keeper = points()
        .into_iter()
        .find(|p| p.version == PesVersion::Pes17 && p.position == 0)
        .expect("a keeper read in PES 2017");
    let mut pes15 = keeper.player.clone();
    pes15.stats.coverage = None;
    pes15.stats.clearing = None;
    pes15.stats.reflexes = None;
    pes15.stats.catching = Some(pes15.stats.goalkeeping);
    let mut full = pes15.clone();
    full.stats.coverage = Some(full.stats.goalkeeping);
    full.stats.clearing = Some(full.stats.goalkeeping);
    full.stats.reflexes = Some(full.stats.goalkeeping);
    assert_eq!(
        pes15.overall_at(PesVersion::Pes15, 0),
        full.overall_at(PesVersion::Pes15, 0)
    );
    // And not a zero standing in for the missing ones.
    let mut zeros = full.clone();
    zeros.stats.coverage = Some(0);
    assert_ne!(
        zeros.overall_at(PesVersion::Pes15, 0),
        full.overall_at(PesVersion::Pes15, 0)
    );
}

/// A PES 2014 player with `ability` in every stat, registered CB, rated C
/// at every playable position.
fn pes14_player(ability: u8) -> PlayerEntry {
    let mut player = PlayerEntry::default();
    let s = &mut player.stats;
    s.attacking_prowess = ability;
    s.ball_control = ability;
    s.dribbling = ability;
    s.low_pass = ability;
    s.lofted_pass = ability;
    s.finishing = ability;
    s.place_kicking = ability;
    s.swerve = ability;
    s.heading = ability;
    s.defensive_prowess = ability;
    s.ball_winning = ability;
    s.kicking_power = ability;
    s.speed = ability;
    s.explosive_power = ability;
    s.body_control = ability;
    s.jump = ability;
    s.goalkeeping = ability;
    s.catching = Some(ability);
    s.stamina = ability;
    s.tenacity = Some(ability);
    player.positions.registered = 1;
    player
}

// PES 2014's own formula, on the numbers the game's Game Plan shows for
// this player (run tactics6's `/3/`): each term `(value - 25) * weight`
// summed, rounded a half up, the registered CB's 7 added. CB's 71.24 and
// CF's 71.24 floor to 71 where rounding the sum plus 0.5 to the nearest
// would give 72.
#[test]
fn pes14s_overall_is_its_own_formula() {
    let player = pes14_player(77);
    for (position, expected) in [(0, 67), (1, 78), (2, 74), (3, 74), (5, 72), (12, 71)] {
        assert_eq!(
            player.overall_at(PesVersion::Pes14, position),
            Some(expected),
            "position {position}"
        );
    }
}

// The playable grade's bonus on the same player: A adds CMF's 5 to the
// weighted sum's 72, B its 3.
#[test]
fn pes14s_playable_grade_adds_its_bonus() {
    let mut player = pes14_player(77);
    let cmf = PLAYABLE_ORDER
        .iter()
        .position(|&p| p == 5)
        .expect("every position rated");
    player.positions.playable[cmf] = 2;
    assert_eq!(
        player.overall_at(PesVersion::Pes14, 5),
        Some(77),
        "A: 72 + 5"
    );
    player.positions.playable[cmf] = 1;
    assert_eq!(
        player.overall_at(PesVersion::Pes14, 5),
        Some(75),
        "B: 72 + 3"
    );
}

// 40 in every ability: the keeper's 15 x 1.28 = 19.2 rounds to 19, held
// to 40.
#[test]
fn pes14s_overall_is_held_to_40() {
    let player = pes14_player(40);
    assert_eq!(player.overall_at(PesVersion::Pes14, 0), Some(40));
}
