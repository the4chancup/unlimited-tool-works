//! A player's overall rating at a position: PES 2016's own computation, read
//! from its executable, with three rows' abilities settled by in-game checks.
//! PES 2017 shows the number it gives for every player checked (162
//! overalls, nine positions); PES 2021's own differs from it by up to 2.
//! Every version but PES 2014 uses it, no other version's being read; PES
//! 2014's own is in `pes14`, read from its executable the same way.

use pes_version::PesVersion;

use super::player::{PlayerEntry, PlayerStats};

mod pes14;

/// Which abilities a weight row reads. The executable reads its abilities
/// unnamed; each row's ability is the one its weights' profile names (the
/// finishing row weighs CF 36, the heading row CB 23). PES 2017's overalls
/// settled the three rows it left open: the row weighing DMF 12 reads
/// swerve, not kicking power; the one weighing SS 6 reads kicking power; and
/// the one weighing DMF 13 reads the mean of body control and physical
/// contact, not body control alone.
#[derive(Clone, Copy)]
enum Term {
    /// One ability.
    Ability(fn(&PlayerStats) -> u8),
    /// The mean, rounded down, of the abilities the save's version stores:
    /// PES 2015 has no clearing, reflexes, coverage or physical contact, and
    /// a mean over the ones it has keeps the term on the same scale.
    Mean(&'static [fn(&PlayerStats) -> Option<u8>]),
    /// A 2-bit field, which counts 1 whatever it holds: every term enters as
    /// `max(1, value - 25)`.
    TwoBit,
}

impl Term {
    fn value(self, stats: &PlayerStats) -> u32 {
        match self {
            Term::Ability(get) => u32::from(get(stats)),
            Term::Mean(gets) => {
                let (sum, count) = gets
                    .iter()
                    .filter_map(|get| get(stats))
                    .fold((0, 0), |(sum, count), value| {
                        (sum + u32::from(value), count + 1)
                    });
                // Every version stores each mean's first ability
                // (goalkeeping, catching as PES 15's Saving, body control);
                // a bare record holding none counts 0 rather than dividing
                // by zero.
                sum / count.max(1)
            }
            Term::TwoBit => 1,
        }
    }
}

/// The twenty weight rows, each a weight per position in the position byte's
/// order (GK CB LB RB DMF CMF LMF RMF AMF LWF RWF SS CF). The executable's
/// columns put each right-side position before its left twin; the twins'
/// weights are equal, so this order reads the same numbers.
const WEIGHTS: [(Term, [u32; 13]); 20] = [
    (
        Term::Ability(|s| s.attacking_prowess),
        [0, 0, 6, 6, 7, 5, 7, 7, 14, 17, 17, 15, 31],
    ),
    (
        Term::Ability(|s| s.ball_control),
        [0, 0, 10, 10, 18, 24, 15, 15, 24, 19, 19, 19, 24],
    ),
    (
        Term::Ability(|s| s.dribbling),
        [0, 0, 14, 14, 15, 24, 25, 25, 24, 22, 22, 19, 14],
    ),
    (
        Term::Ability(|s| s.low_pass),
        [0, 0, 0, 0, 18, 24, 7, 7, 22, 5, 5, 9, 0],
    ),
    (
        Term::Ability(|s| s.lofted_pass),
        [0, 0, 14, 14, 19, 21, 12, 12, 14, 9, 9, 9, 0],
    ),
    (
        Term::Ability(|s| s.finishing),
        [0, 0, 0, 0, 0, 0, 0, 0, 17, 11, 11, 14, 36],
    ),
    (Term::Ability(|s| s.place_kicking), [0; 13]),
    (
        Term::Ability(|s| s.swerve),
        [0, 0, 0, 0, 12, 0, 4, 4, 0, 0, 0, 0, 0],
    ),
    (
        Term::Ability(|s| s.heading),
        [0, 23, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3],
    ),
    (
        Term::Ability(|s| s.defensive_prowess),
        [0, 26, 14, 14, 8, 3, 0, 0, 0, 0, 0, 0, 0],
    ),
    (
        Term::Ability(|s| s.ball_winning),
        [0, 26, 13, 13, 4, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    (
        Term::Ability(|s| s.kicking_power),
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 5, 6, 3],
    ),
    (
        Term::Ability(|s| s.speed),
        [0, 10, 16, 16, 3, 4, 24, 24, 5, 15, 15, 9, 5],
    ),
    (
        Term::Ability(|s| s.explosive_power),
        [0, 0, 14, 14, 3, 6, 22, 22, 7, 15, 15, 22, 5],
    ),
    (
        Term::Mean(&[|s| Some(s.body_control), |s| s.physical_contact]),
        [11, 20, 12, 12, 13, 5, 0, 0, 5, 6, 6, 7, 9],
    ),
    (
        Term::Ability(|s| s.jump),
        [11, 20, 12, 12, 5, 0, 0, 0, 0, 0, 0, 0, 3],
    ),
    (
        Term::Mean(&[|s| Some(s.goalkeeping), |s| s.coverage]),
        [49, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    (
        Term::Mean(&[|s| s.catching, |s| s.clearing, |s| s.reflexes]),
        [49, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    (
        Term::Ability(|s| s.stamina),
        [0, 9, 14, 14, 14, 17, 13, 13, 3, 6, 6, 4, 0],
    ),
    (
        Term::TwoBit,
        [0, 24, 58, 58, 24, 24, 56, 56, 38, 47, 47, 38, 38],
    ),
];

/// The bonus at the registered position, in the position byte's order.
const REGISTERED: [u32; 13] = [8, 8, 9, 9, 9, 8, 8, 8, 8, 10, 10, 9, 8];
/// The bonus at a playable position rated A.
const RATED_A: [u32; 13] = [5, 5, 4, 4, 4, 5, 5, 5, 5, 4, 4, 5, 5];
/// The bonus at a playable position rated B (a C has none).
const RATED_B: [u32; 13] = [3, 3, 2, 2, 2, 3, 3, 3, 3, 2, 2, 3, 3];

/// The registered position's blend term by the first nineteen terms' mean,
/// 0 to 99.
const BLEND: [u32; 100] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48,
    49, 50, 51, 52, 56, 59, 62, 65, 67, 70, 73, 75, 77, 79, 82, 83, 85, 87, 89, 90, 92, 93, 94, 95,
    96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113,
];

/// `PlayerPositions::playable`'s order as position bytes: entry `i` rates
/// the position `PLAYABLE_ORDER[i]` (CF first, GK last).
const PLAYABLE_ORDER: [u8; 13] = [12, 11, 9, 10, 8, 4, 5, 6, 7, 1, 2, 3, 0];

impl PlayerEntry {
    /// The player's overall at `position` (a position byte, GK 0 to CF 12),
    /// `version` picking the formula: PES 2014's own on a PES 2014 save,
    /// PES 2016's on every later version; `None` for a byte past CF. PES
    /// 2016's equals every overall PES 2017's screens were seen to show,
    /// and is within 2 of every one PES 2021's showed, whose own formula
    /// differs. The twenty weighted terms, each `max(1, value - 25)`,
    /// summed and rounded to hundredths' units; the position's bonus
    /// (registered, A, B); at his registered position, unless it is GK,
    /// 60% of that and 40% of the blend table at the first nineteen terms'
    /// mean; clamped to 40-109.
    pub fn overall_at(&self, version: PesVersion, position: u8) -> Option<u8> {
        if version == PesVersion::Pes14 {
            return pes14::overall_at(self, position);
        }
        self.later_overall_at(position)
    }

    /// PES 2016's formula, which every version after PES 2014 uses.
    fn later_overall_at(&self, position: u8) -> Option<u8> {
        let col = usize::from(position);
        if col >= REGISTERED.len() {
            return None;
        }
        let stats = &self.stats;
        let sum: u32 = WEIGHTS
            .iter()
            .map(|(term, weights)| weights[col] * term.value(stats).saturating_sub(25).max(1))
            .sum();
        let registered = self.positions.registered == position;
        let bonus = if registered {
            REGISTERED[col]
        } else {
            let slot = PLAYABLE_ORDER
                .iter()
                .position(|&p| p == position)
                .expect("PLAYABLE_ORDER holds every position byte below 13");
            let rating = self.positions.playable[slot];
            // 3, a rating no save holds, reads as an A, as the executable's
            // "at least A" test does.
            match rating {
                2.. => RATED_A[col],
                1 => RATED_B[col],
                _ => 0,
            }
        };
        let mut overall = (sum + 50) / 100 + bonus;
        if registered && position != 0 {
            let terms: u32 = WEIGHTS[..19]
                .iter()
                .map(|(term, _)| term.value(stats))
                .sum();
            // Abilities stop at 99 in game, so the mean does; a save holding
            // more reads the table's last entry rather than past its end.
            let mean =
                usize::try_from(((terms * 100 / 19 + 50) / 100).min(99)).expect("99 fits a usize");
            overall = (overall * 60 + 50 + BLEND[mean] * 40) / 100;
        }
        Some(u8::try_from(overall.clamp(40, 109)).expect("109 fits a u8"))
    }
}

#[cfg(test)]
mod tests;
