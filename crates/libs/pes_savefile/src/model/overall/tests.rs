use super::*;
use crate::interchange::team_toml::labels::{PLAYABLE_POSITIONS, POSITIONS};

/// Overalls read in game, with each player's abilities and positions
/// (`tests/fixtures/README.md` "The overall formula's fixture").
const POINTS: &str = include_str!("../../../tests/fixtures/overall_points.tsv");

/// One fixture row: its version, the player, the position byte, the overall
/// the game showed and the one PES 2016's formula gives.
struct Point {
    version: String,
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
                version: cell("version").to_owned(),
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
    let count = |version: &str| points.iter().filter(|p| p.version == version).count();
    assert_eq!((count("pes17"), count("pes21")), (162, 201));
    for p in &points {
        let got = p.player.overall_at(p.position);
        assert_eq!(
            got,
            Some(p.computed),
            "{} at {}: registered {}",
            p.version,
            POSITIONS[usize::from(p.position)],
            POSITIONS[usize::from(p.player.positions.registered)]
        );
        if p.version == "pes17" {
            assert_eq!(p.computed, p.shown, "PES 2017 shows the formula's number");
        } else {
            assert!(p.computed.abs_diff(p.shown) <= 2, "PES 2021 within 2");
        }
    }
}

#[test]
fn a_position_past_cf_has_no_overall() {
    let player = points().remove(0).player;
    assert!(player.overall_at(12).is_some());
    assert_eq!(player.overall_at(13), None);
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
        .find(|p| p.version == "pes17")
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
            u32::from(player.overall_at(position).expect("a position"))
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
        .find(|p| p.version == "pes17" && p.position == 0)
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
    assert_eq!(pes15.overall_at(0), full.overall_at(0));
    // And not a zero standing in for the missing ones.
    let mut zeros = full.clone();
    zeros.stats.coverage = Some(0);
    assert_ne!(zeros.overall_at(0), full.overall_at(0));
}
