//! PES 2014's own overall at a position, read from its executable: each
//! ability the position weighs contributes `(value - 25) * weight` to a
//! sum rounded a half up, plus the position's bonus, held to 40-109.

use crate::model::player::{PlayerEntry, PlayerStats};

use super::PLAYABLE_ORDER;

/// Which ability a weight row reads.
type Ability = fn(&PlayerStats) -> u8;

/// The weight rows, each ability's weight per position in the position
/// byte's order (GK CB LB RB DMF CMF LMF RMF AMF LWF RWF SS CF), as the
/// game's table holds them. The executable reads its abilities unnamed:
/// each row was named by its weights across the thirteen positions,
/// which match PES 2016's rows almost exactly, and run `gameplan2a`
/// confirmed the names in game (two players whose twenty abilities all
/// differ, fifteen overalls, each this formula's). Place kicking and
/// tenacity weigh nothing anywhere, so they have no row.
const WEIGHTS: [(Ability, [f32; 13]); 18] = [
    (
        |s| s.attacking_prowess,
        [
            0.0, 0.0, 0.06, 0.06, 0.07, 0.05, 0.07, 0.07, 0.15, 0.18, 0.18, 0.16, 0.33,
        ],
    ),
    (
        |s| s.ball_control,
        [
            0.0, 0.0, 0.10, 0.10, 0.19, 0.25, 0.16, 0.16, 0.23, 0.22, 0.22, 0.20, 0.25,
        ],
    ),
    (
        |s| s.dribbling,
        [
            0.0, 0.0, 0.15, 0.15, 0.16, 0.25, 0.26, 0.26, 0.23, 0.22, 0.22, 0.20, 0.15,
        ],
    ),
    (
        |s| s.low_pass,
        [
            0.0, 0.0, 0.0, 0.0, 0.19, 0.25, 0.07, 0.07, 0.23, 0.05, 0.05, 0.10, 0.0,
        ],
    ),
    (
        |s| s.lofted_pass,
        [
            0.0, 0.0, 0.15, 0.15, 0.20, 0.22, 0.13, 0.13, 0.15, 0.10, 0.10, 0.10, 0.0,
        ],
    ),
    (
        |s| s.finishing,
        [
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.18, 0.12, 0.12, 0.15, 0.38,
        ],
    ),
    (
        |s| s.swerve,
        [
            0.0, 0.0, 0.0, 0.0, 0.13, 0.0, 0.04, 0.04, 0.0, 0.0, 0.0, 0.0, 0.0,
        ],
    ),
    (
        |s| s.heading,
        [
            0.0, 0.20, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.03,
        ],
    ),
    (
        |s| s.defensive_prowess,
        [
            0.0, 0.27, 0.15, 0.15, 0.07, 0.03, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ],
    ),
    (
        |s| s.ball_winning,
        [
            0.0, 0.27, 0.14, 0.14, 0.03, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ],
    ),
    (
        |s| s.kicking_power,
        [
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.05, 0.05, 0.06, 0.0,
        ],
    ),
    (
        |s| s.speed,
        [
            0.0, 0.11, 0.15, 0.15, 0.03, 0.04, 0.26, 0.26, 0.05, 0.16, 0.16, 0.10, 0.05,
        ],
    ),
    (
        |s| s.explosive_power,
        [
            0.0, 0.0, 0.15, 0.15, 0.03, 0.06, 0.23, 0.23, 0.07, 0.16, 0.16, 0.20, 0.05,
        ],
    ),
    (
        |s| s.body_control,
        [
            0.12, 0.21, 0.12, 0.12, 0.14, 0.05, 0.0, 0.0, 0.05, 0.06, 0.06, 0.07, 0.10,
        ],
    ),
    (
        |s| s.jump,
        [
            0.12, 0.21, 0.12, 0.12, 0.05, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.03,
        ],
    ),
    (
        |s| s.goalkeeping,
        [
            0.52, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ],
    ),
    (
        |s| s.catching.expect("a PES 2014 record stores catching"),
        [
            0.52, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ],
    ),
    (
        |s| s.stamina,
        [
            0.0, 0.10, 0.13, 0.13, 0.15, 0.18, 0.14, 0.14, 0.03, 0.06, 0.06, 0.04, 0.0,
        ],
    ),
];

/// The bonus at the registered position, in the position byte's order.
const REGISTERED: [f32; 13] = [
    7.0, 7.0, 8.0, 8.0, 8.0, 7.0, 7.0, 7.0, 7.0, 9.0, 9.0, 7.0, 7.0,
];
/// The bonus at a playable position rated A.
const RATED_A: [f32; 13] = [
    5.0, 5.0, 4.0, 4.0, 4.0, 5.0, 5.0, 5.0, 5.0, 4.0, 4.0, 5.0, 5.0,
];
/// The bonus at a playable position rated B (a C has none).
const RATED_B: [f32; 13] = [
    3.0, 2.0, 2.0, 2.0, 2.0, 3.0, 3.0, 3.0, 3.0, 2.0, 2.0, 3.0, 3.0,
];

/// `player`'s overall at `position` (a position byte, GK 0 to CF 12),
/// `None` for a byte past CF.
pub(super) fn overall_at(player: &PlayerEntry, position: u8) -> Option<u8> {
    let col = usize::from(position);
    if col >= REGISTERED.len() {
        return None;
    }
    let stats = &player.stats;
    // The code subtracts 25 only from a value over 24, which every PES
    // 2014 ability is (40 at the least).
    let sum: f32 = WEIGHTS
        .iter()
        .map(|(get, weights)| (f32::from(get(stats)) - 25.0) * weights[col])
        .sum();
    let bonus = if player.positions.registered == position {
        REGISTERED[col]
    } else {
        let slot = PLAYABLE_ORDER
            .iter()
            .position(|&p| p == position)
            .expect("PLAYABLE_ORDER holds every position byte below 13");
        // 3, a rating no save holds, reads as an A.
        match player.positions.playable[slot] {
            2.. => RATED_A[col],
            1 => RATED_B[col],
            _ => 0.0,
        }
    };
    // Rounded a half up, the bonus added, held to 40-109.
    let overall = (sum + 0.5).floor() + bonus;
    Some(overall.clamp(40.0, 109.0) as u8)
}
