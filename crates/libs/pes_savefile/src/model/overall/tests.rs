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

/// A PES 2014 player registered CB, rated C at every playable position,
/// holding `abilities` in the order of the Save editor's ability column:
/// attacking prowess, ball control, dribbling, low pass, lofted pass,
/// finishing, place kicking, swerve, heading, defensive prowess, ball
/// winning, kicking power, speed, explosive power, body control, jump,
/// goalkeeping, catching, stamina, tenacity.
fn pes14_player_with(abilities: [u8; 20]) -> PlayerEntry {
    let [
        attacking_prowess,
        ball_control,
        dribbling,
        low_pass,
        lofted_pass,
        finishing,
        place_kicking,
        swerve,
        heading,
        defensive_prowess,
        ball_winning,
        kicking_power,
        speed,
        explosive_power,
        body_control,
        jump,
        goalkeeping,
        catching,
        stamina,
        tenacity,
    ] = abilities;
    let mut player = PlayerEntry {
        stats: PlayerStats {
            attacking_prowess,
            ball_control,
            dribbling,
            low_pass,
            lofted_pass,
            finishing,
            place_kicking,
            swerve,
            heading,
            defensive_prowess,
            ball_winning,
            kicking_power,
            speed,
            explosive_power,
            body_control,
            jump,
            goalkeeping,
            catching: Some(catching),
            stamina,
            tenacity: Some(tenacity),
            ..PlayerStats::default()
        },
        ..PlayerEntry::default()
    };
    player.positions.registered = 1;
    player
}

/// `pes14_player_with` `ability` in every stat.
fn pes14_player(ability: u8) -> PlayerEntry {
    pes14_player_with([ability; 20])
}

/// `player`'s playable slot for `position` (a position byte).
fn playable_slot(position: u8) -> usize {
    PLAYABLE_ORDER
        .iter()
        .position(|&p| p == position)
        .expect("every position rated")
}

// PES 2014's own formula for a player holding 77 in every ability: each
// term `(value - 25) * weight` summed, rounded a half up, the registered
// CB's 7 added. The game's Game Plan showed GK 67, CB 78, LB and RB 74,
// CMF 72 and CF 71 for such players (run tactics6's `/3/`); CB's 71.24 and
// CF's 71.24 floor to 71 where rounding the sum plus 0.5 to the nearest
// would give 72. The other seven are the formula's.
#[test]
fn pes14s_overall_is_its_own_formula() {
    let player = pes14_player(77);
    for (position, expected) in [
        (0, 67),
        (1, 78),
        (2, 74),
        (3, 74),
        (4, 75),
        (5, 72),
        (6, 71),
        (7, 71),
        (8, 71),
        (9, 72),
        (10, 72),
        (11, 72),
        (12, 71),
    ] {
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
    let cmf = playable_slot(5);
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

// The other end: 99 in every ability, registered LWF: LWF's weights sum
// to 1.38, so 74 x 1.38 = 102.12 floors to 102, plus 9 is 111, held to
// 109. And no overall past CF.
#[test]
fn pes14s_overall_is_held_to_109_and_none_past_cf() {
    let mut player = pes14_player(99);
    player.positions.registered = 9;
    assert_eq!(player.overall_at(PesVersion::Pes14, 9), Some(109));
    assert_eq!(player.overall_at(PesVersion::Pes14, 13), None);
}

// The all-77 player registered CF instead: CB rated B adds PES 2014's 2,
// not PES 2016's 3 (71.24 floors to 71, so 73); CF rated A still takes
// the registered 7, not A's 5 (71 + 7 = 78).
#[test]
fn pes14s_registered_bonus_wins_and_cbs_b_adds_2() {
    let mut player = pes14_player(77);
    player.positions.registered = 12;
    player.positions.playable[playable_slot(1)] = 1;
    player.positions.playable[playable_slot(12)] = 2;
    assert_eq!(player.overall_at(PesVersion::Pes14, 1), Some(73), "CB at B");
    assert_eq!(
        player.overall_at(PesVersion::Pes14, 12),
        Some(78),
        "CF registered"
    );
}

// Run gameplan2a's two `/3/` players, registered CB and rated C, with
// abilities that all differ: each overall the game showed, which pins the
// ability each weight row reads.
#[test]
fn pes14s_overall_matches_the_games_for_players_whose_abilities_differ() {
    let set_a = pes14_player_with([
        90, 70, 85, 55, 80, 95, 99, 75, 50, 45, 65, 72, 88, 66, 58, 48, 60, 42, 82, 41,
    ]);
    let set_b = pes14_player_with([
        44, 61, 52, 78, 57, 47, 40, 49, 89, 92, 83, 68, 63, 74, 81, 97, 86, 94, 54, 99,
    ]);
    for (player, shown) in [
        (
            &set_a,
            &[
                (1, 53),
                (3, 64),
                (5, 67),
                (6, 72),
                (8, 71),
                (9, 75),
                (10, 75),
                (12, 78),
            ][..],
        ),
        (&set_b, &[(1, 88), (3, 64), (4, 57), (5, 51), (12, 42)][..]),
    ] {
        for &(position, expected) in shown {
            assert_eq!(
                player.overall_at(PesVersion::Pes14, position),
                Some(expected),
                "position {position}"
            );
        }
    }
}
