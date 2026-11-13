//! The illegal formation rule: what the game's own formation screens can
//! make of one stored formation set, and the per-position pitch areas
//! that rule and the screens' drop both read. A formation breaking it is
//! one only a tool outside the game could have written; the game itself
//! plays it as stored, refusing and repairing nothing.

use std::ops::{Range, RangeInclusive};

use crate::model::tactics::{Formation, FormationSlot};

/// A position's pitch area in stored units, bounds inclusive (x 0 to
/// 104 across, y 0 to 48 up).
#[derive(Debug, Clone, Copy)]
pub struct PositionArea {
    /// The area's lowest y.
    pub y_min: u8,
    /// The area's highest y.
    pub y_max: u8,
    /// The area's leftmost x.
    pub x_min: u8,
    /// The area's rightmost x.
    pub x_max: u8,
}

impl PositionArea {
    /// The point is inside the area, bounds included.
    pub fn contains(self, x: u8, y: u8) -> bool {
        self.y_min <= y && y <= self.y_max && self.x_min <= x && x <= self.x_max
    }
}

/// One table row, `(y_min, y_max, x_min, x_max)` as the tables store it.
const fn rect(y_min: u8, y_max: u8, x_min: u8, x_max: u8) -> PositionArea {
    PositionArea {
        y_min,
        y_max,
        x_min,
        x_max,
    }
}

/// The two area tables the game holds per position, indexed by the
/// position byte (GK 0 to CF 12), read from PES 2016's executable (the
/// last version left unpacked) and checked against PES 2017's and PES
/// 2021's formation screens; a position's area holds a point when
/// either row does. The second table widens the first's left and
/// central rows out to the pitch's edges and its top rows to y 48 —
/// the right-side rows it leaves at x 99 — and both are kept as the
/// game has them. The first table's rows are also the areas a held
/// starter lights.
pub const AREAS_A: [PositionArea; 13] = [
    rect(0, 3, 52, 56),   // GK
    rect(4, 15, 5, 99),   // CB
    rect(4, 15, 5, 20),   // LB
    rect(4, 15, 84, 99),  // RB
    rect(16, 25, 21, 83), // DMF
    rect(16, 34, 5, 99),  // CMF
    rect(16, 34, 5, 20),  // LMF
    rect(16, 34, 84, 99), // RMF
    rect(26, 34, 21, 83), // AMF
    rect(35, 47, 5, 20),  // LWF
    rect(35, 47, 84, 99), // RWF
    rect(35, 47, 5, 99),  // SS
    rect(40, 47, 21, 83), // CF
];
/// The second table; `AREAS_A`'s doc explains it.
pub const AREAS_B: [PositionArea; 13] = [
    rect(0, 3, 52, 56),   // GK
    rect(4, 15, 0, 104),  // CB
    rect(4, 15, 0, 20),   // LB
    rect(4, 15, 84, 99),  // RB
    rect(16, 25, 21, 83), // DMF
    rect(16, 34, 0, 104), // CMF
    rect(16, 34, 0, 20),  // LMF
    rect(16, 34, 84, 99), // RMF
    rect(26, 34, 21, 83), // AMF
    rect(35, 48, 0, 20),  // LWF
    rect(35, 48, 84, 99), // RWF
    rect(35, 48, 0, 104), // SS
    rect(40, 48, 21, 83), // CF
];

/// Two outfield starters stand on each other when they are no more
/// than this many stored x units apart across (and `CLOSE_UP_DOWN` up
/// or down). With `CLOSE_UP_DOWN` it is the editor pitch's swap box in
/// stored units, taken so that each starter stands inside the other's
/// marker box: half the box's width across, and the shorter of its two
/// reaches from the badge's centre up or down. The game's own screen
/// swaps from further away (11 across, 3 to 5 up or down), but the
/// game's own formations hold pairs inside that larger box (two
/// defensive midfielders 9 apart on one line), so it would flag what the
/// game ships; this box flags none of them.
pub const CLOSE_ACROSS: u8 = 8;
/// The reach up or down of the box `CLOSE_ACROSS` describes.
pub const CLOSE_UP_DOWN: u8 = 2;

/// The goalkeeper's position byte.
const GK: u8 = 0;
/// The keeper's spot, the one the first slot always holds: the screens
/// cannot pick the keeper up.
const KEEPER_SPOT: (u8, u8) = (52, 3);
/// The outfield slots: every formation slot but the keeper's.
const OUTFIELD: Range<usize> = 1..11;
/// The spots a starter can be dragged to, across and up: a drop past an
/// edge stores the edge.
const MOVABLE_X: RangeInclusive<u8> = 5..=99;
const MOVABLE_Y: RangeInclusive<u8> = 4..=47;
/// The wide positions (LB, RB, LMF, RMF, LWF, RWF), in byte order: a
/// drop offers one only while no other starter holds it; the rest are
/// always free, however many starters share them.
pub const WIDE: [u8; 6] = [2, 3, 6, 7, 9, 10];

/// One of the three outfield lines whose sizes the game holds a
/// formation to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormationLine {
    /// CB, LB and RB: two to five.
    Defenders,
    /// DMF, CMF, LMF, RMF and AMF: one to six.
    Midfielders,
    /// LWF, RWF, SS and CF: one to five.
    Forwards,
}

impl FormationLine {
    /// The three lines, in the order the rule reports them.
    const ALL: [FormationLine; 3] = [
        FormationLine::Defenders,
        FormationLine::Midfielders,
        FormationLine::Forwards,
    ];

    /// The position bytes of the line.
    fn positions(self) -> &'static [u8] {
        match self {
            FormationLine::Defenders => &[1, 2, 3],
            FormationLine::Midfielders => &[4, 5, 6, 7, 8],
            FormationLine::Forwards => &[9, 10, 11, 12],
        }
    }

    /// The number of starters the line may hold, bounds included.
    fn limits(self) -> RangeInclusive<usize> {
        match self {
            FormationLine::Defenders => 2..=5,
            FormationLine::Midfielders => 1..=6,
            FormationLine::Forwards => 1..=5,
        }
    }
}

/// One thing in a stored formation set the game's own screens could not
/// have made (`Formation::illegal_breaks`). A slot is a formation slot,
/// 0 the keeper's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormationBreak {
    /// The first slot is not a goalkeeper on the keeper's spot (x 52,
    /// y 3).
    KeeperOffSpot,
    /// An outfield starter stands outside the area a starter can be
    /// dragged to (x 5 to 99, y 4 to 47).
    OutsideMovableRange {
        /// The starter's slot.
        slot: usize,
    },
    /// An outfield starter holds a position his spot does not offer.
    PositionNotOffered {
        /// The starter's slot.
        slot: usize,
    },
    /// More than one starter holds one wide position.
    WidePositionTwice {
        /// The position byte (LB 2, RB 3, LMF 6, RMF 7, LWF 9, RWF 10).
        position: u8,
        /// Every slot holding it, ascending.
        slots: Vec<usize>,
    },
    /// A line holds more or fewer starters than the game allows.
    LineCount {
        /// The line.
        line: FormationLine,
        /// How many of the eleven slots hold one of its positions.
        count: usize,
    },
    /// Two outfield starters stand on each other: each within
    /// `CLOSE_ACROSS` across and `CLOSE_UP_DOWN` up or down of the other.
    TooClose {
        /// The two slots, the lower first.
        slots: [usize; 2],
    },
}

impl Formation {
    /// Everything in this formation set the game's own screens could not
    /// have made, clause by clause; within a clause by slot (clauses 2,
    /// 3 and 6), by position byte (4) or by line (5). Empty
    /// for a set the game's screens (the drag, the Select Position menu,
    /// a swap, the Change Formation list) could have produced. The game
    /// plays a broken set as stored; only a drop repairs the starter it
    /// moves.
    ///
    /// 1. The first slot is the keeper, at x 52, y 3: the screens cannot
    ///    pick him up, so nothing moves him or changes his position.
    /// 2. Every outfield starter stands in the movable range, x 5 to 99
    ///    and y 4 to 47: a drop past an edge stores the edge.
    /// 3. Every outfield starter holds a position his spot offers, its
    ///    row of `AREAS_A` holding the spot: a drop and the Select
    ///    Position menu write only a position the spot offers, and a swap
    ///    exchanges whole spot-and-position pairs. A starter outside the
    ///    movable range is left to clause 2 alone: the screens could not
    ///    have stored his spot at all, so his position there says nothing
    ///    more.
    /// 4. No wide position (LB, RB, LMF, RMF, LWF, RWF) is held twice:
    ///    a drop offers a wide position only while no other starter
    ///    holds it.
    /// 5. The eleven slots hold two to five defenders, one to six
    ///    midfielders and one to five forwards (`FormationLine`): the
    ///    screen refuses a drop that would leave a line outside them.
    /// 6. No two outfield starters stand on each other
    ///    (`CLOSE_ACROSS`, `CLOSE_UP_DOWN`): a drop inside a teammate's
    ///    marker swaps the two instead. The keeper's marker takes no
    ///    swap and is left out.
    ///
    /// Overalls play no part: the drag picks between a spot's two
    /// positions by rating, but the menu gives either.
    pub fn illegal_breaks(&self) -> Vec<FormationBreak> {
        let players = &self.players;
        let mut breaks = Vec::new();
        let keeper = players[0];
        if (keeper.position, (keeper.x, keeper.y)) != (GK, KEEPER_SPOT) {
            breaks.push(FormationBreak::KeeperOffSpot);
        }
        let movable = |slot: usize| {
            MOVABLE_X.contains(&players[slot].x) && MOVABLE_Y.contains(&players[slot].y)
        };
        breaks.extend(
            OUTFIELD
                .filter(|&slot| !movable(slot))
                .map(|slot| FormationBreak::OutsideMovableRange { slot }),
        );
        breaks.extend(
            OUTFIELD
                .filter(|&slot| movable(slot) && !offered(players[slot]))
                .map(|slot| FormationBreak::PositionNotOffered { slot }),
        );
        for position in WIDE {
            let slots: Vec<usize> = (0..players.len())
                .filter(|&slot| players[slot].position == position)
                .collect();
            if slots.len() > 1 {
                breaks.push(FormationBreak::WidePositionTwice { position, slots });
            }
        }
        for line in FormationLine::ALL {
            let count = players
                .iter()
                .filter(|player| line.positions().contains(&player.position))
                .count();
            if !line.limits().contains(&count) {
                breaks.push(FormationBreak::LineCount { line, count });
            }
        }
        for first in OUTFIELD {
            for second in first + 1..players.len() {
                let (a, b) = (players[first], players[second]);
                if a.x.abs_diff(b.x) <= CLOSE_ACROSS && a.y.abs_diff(b.y) <= CLOSE_UP_DOWN {
                    breaks.push(FormationBreak::TooClose {
                        slots: [first, second],
                    });
                }
            }
        }
        breaks
    }
}

/// The spot of `player`, inside the movable range, offers his position:
/// its `AREAS_A` row holds the spot. The screens offer the band's default
/// (CB up to y 15, CMF from 16 to 34, SS from 35) or the one other
/// position whose area holds the spot. Inside the movable range that is
/// the same test: each band default's row spans the range's whole width
/// over its band, no spot lies in two other positions' rows, and the
/// second table differs from the first only outside the range. Whether a
/// wide position is free is clause 4's question, not this one's. GK's row
/// lies below the range and a byte past CF has no row, so neither is ever
/// offered.
fn offered(player: FormationSlot) -> bool {
    AREAS_A
        .get(usize::from(player.position))
        .is_some_and(|area| area.contains(player.x, player.y))
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;

    use super::*;
    use crate::interchange::team_toml::labels::POSITIONS;
    use crate::test_support::open;

    use FormationBreak::*;
    use FormationLine::*;

    /// A slot holding `position` (named as `POSITIONS` names it) at
    /// `(x, y)`.
    fn at(position: &str, x: u8, y: u8) -> FormationSlot {
        let index = POSITIONS
            .iter()
            .position(|name| *name == position)
            .unwrap_or_else(|| panic!("{position} is a position name"));
        FormationSlot {
            position: u8::try_from(index).expect("13 positions"),
            x,
            y,
        }
    }

    /// The PES 2017 fixture's team 701, preset 1's Kick-off set: the
    /// game's default 4-2-3-1.
    fn base() -> Formation {
        let (file, _) = open(PesVersion::Pes17);
        file.team(701).expect("team 701").tactics.presets[0].formations[0].clone()
    }

    /// The breaks of `base` with each `(slot, value)` of `changes` set.
    fn breaks_with(changes: &[(usize, FormationSlot)]) -> Vec<FormationBreak> {
        let mut formation = base();
        for &(slot, value) in changes {
            formation.players[slot] = value;
        }
        formation.illegal_breaks()
    }

    /// One case: the `(slot, value)` changes to the base, and the breaks
    /// the changed set gives, exactly.
    type Case<'a> = (&'a [(usize, FormationSlot)], &'a [FormationBreak]);

    /// Every case of `cases` gives exactly its expected breaks.
    fn check(cases: &[Case<'_>]) {
        for (changes, expected) in cases {
            assert_eq!(breaks_with(changes), *expected, "the base with {changes:?}");
        }
    }

    /// The formations of a fixture file: one a line of eleven
    /// `POSITION x y` triples, `#` lines comments.
    fn read_formations(text: &str) -> Vec<Formation> {
        text.lines()
            .filter(|line| !line.starts_with('#'))
            .map(|line| {
                let slots: Vec<FormationSlot> = line
                    .split(", ")
                    .map(|triple| {
                        let fields: Vec<&str> = triple.split(' ').collect();
                        let [position, x, y] = fields[..] else {
                            panic!("a POSITION x y triple: {triple:?} in {line}");
                        };
                        at(position, x.parse().expect("x"), y.parse().expect("y"))
                    })
                    .collect();
                Formation {
                    players: slots.try_into().expect("eleven slots a line"),
                }
            })
            .collect()
    }

    #[test]
    fn every_formation_the_game_made_passes() {
        for (text, count) in [
            (
                include_str!("../../tests/fixtures/game_formations_pes17.txt"),
                938,
            ),
            (
                include_str!("../../tests/fixtures/game_formations_pes19.txt"),
                506,
            ),
        ] {
            let formations = read_formations(text);
            assert_eq!(formations.len(), count);
            for formation in formations {
                assert_eq!(formation.illegal_breaks(), [], "{formation:?}");
            }
        }
    }

    #[test]
    fn the_base_is_the_default_4_2_3_1_and_passes() {
        let formation = base();
        assert_eq!(
            formation.players,
            [
                at("GK", 52, 3),
                at("CB", 63, 10),
                at("CB", 41, 10),
                at("RB", 89, 12),
                at("LB", 15, 12),
                at("DMF", 64, 18),
                at("DMF", 40, 18),
                at("RMF", 86, 30),
                at("LMF", 18, 30),
                at("AMF", 52, 32),
                at("CF", 52, 43),
            ]
        );
        assert_eq!(formation.illegal_breaks(), []);
    }

    #[test]
    fn clause_1_the_keeper_off_his_spot() {
        check(&[
            (&[(0, at("GK", 52, 1))], &[KeeperOffSpot]),
            (&[(0, at("GK", 53, 3))], &[KeeperOffSpot]),
            // A fifth defender on the keeper's spot: the line allows him.
            (&[(0, at("CB", 52, 3))], &[KeeperOffSpot]),
        ]);
    }

    #[test]
    fn clause_2_a_starter_outside_the_movable_range() {
        // Every outfield position's area lies inside the movable range, so
        // a starter past it also stands where his position is not offered:
        // clause 2 alone reports him.
        check(&[
            (&[(4, at("LB", 2, 10))], &[OutsideMovableRange { slot: 4 }]),
            (&[(4, at("LB", 4, 12))], &[OutsideMovableRange { slot: 4 }]),
            (&[(4, at("LB", 5, 12))], &[]),
            (&[(3, at("RB", 99, 12))], &[]),
            (
                &[(3, at("RB", 100, 12))],
                &[OutsideMovableRange { slot: 3 }],
            ),
            // Outside the range he still holds his position and his spot
            // for the other clauses: a second LMF, and on a teammate.
            (
                &[(4, at("LMF", 2, 25))],
                &[
                    OutsideMovableRange { slot: 4 },
                    WidePositionTwice {
                        position: 6,
                        slots: vec![4, 8],
                    },
                ],
            ),
            (
                &[(4, at("LB", 2, 10)), (2, at("CB", 8, 10))],
                &[OutsideMovableRange { slot: 4 }, TooClose { slots: [2, 4] }],
            ),
            (&[(2, at("CB", 41, 3))], &[OutsideMovableRange { slot: 2 }]),
            (&[(2, at("CB", 41, 4))], &[]),
            (&[(10, at("CF", 52, 47))], &[]),
            (
                &[(10, at("CF", 52, 48))],
                &[OutsideMovableRange { slot: 10 }],
            ),
        ]);
    }

    #[test]
    fn clause_3_a_position_the_spot_does_not_offer() {
        let past_cf = FormationSlot {
            position: 13,
            x: 52,
            y: 32,
        };
        check(&[
            (
                &[(10, at("CF", 52, 10))],
                &[PositionNotOffered { slot: 10 }],
            ),
            (&[(9, at("GK", 52, 32))], &[PositionNotOffered { slot: 9 }]),
            (&[(9, past_cf)], &[PositionNotOffered { slot: 9 }]),
        ]);
    }

    #[test]
    fn clause_4_a_wide_position_twice() {
        check(&[(
            &[(9, at("LMF", 14, 25))],
            &[WidePositionTwice {
                position: 6,
                slots: vec![8, 9],
            }],
        )]);
    }

    #[test]
    fn clause_5_a_line_outside_its_limits() {
        let fewest_midfielders = [
            (6, at("SS", 40, 38)),
            (7, at("RWF", 86, 40)),
            (8, at("LWF", 18, 40)),
            (9, at("SS", 52, 36)),
        ];
        check(&[
            (
                &[(5, at("CB", 64, 14)), (6, at("CB", 40, 14))],
                &[LineCount {
                    line: Defenders,
                    count: 6,
                }],
            ),
            (&[(1, at("SS", 64, 44)), (3, at("RWF", 89, 40))], &[]),
            (
                &[
                    (1, at("SS", 64, 44)),
                    (3, at("RWF", 89, 40)),
                    (4, at("LWF", 15, 40)),
                ],
                &[LineCount {
                    line: Defenders,
                    count: 1,
                }],
            ),
            (
                &[(10, at("CMF", 52, 24))],
                &[LineCount {
                    line: Forwards,
                    count: 0,
                }],
            ),
            (
                &[(1, at("CMF", 63, 24)), (4, at("CMF", 15, 22))],
                &[LineCount {
                    line: Midfielders,
                    count: 7,
                }],
            ),
            (&fewest_midfielders, &[]),
            (
                &[fewest_midfielders.as_slice(), &[(5, at("CB", 64, 14))]].concat(),
                &[LineCount {
                    line: Midfielders,
                    count: 0,
                }],
            ),
            (
                &[fewest_midfielders.as_slice(), &[(1, at("SS", 64, 44))]].concat(),
                &[LineCount {
                    line: Forwards,
                    count: 6,
                }],
            ),
        ]);
    }

    #[test]
    fn clause_6_two_starters_on_each_other() {
        check(&[
            (
                &[(5, at("CMF", 40, 22)), (6, at("CMF", 48, 24))],
                &[TooClose { slots: [5, 6] }],
            ),
            (&[(5, at("CMF", 40, 22)), (6, at("CMF", 49, 24))], &[]),
            (&[(5, at("CMF", 40, 22)), (6, at("CMF", 48, 25))], &[]),
            // Just above the keeper: his marker is left out.
            (&[(2, at("CB", 52, 4))], &[]),
        ]);
    }

    #[test]
    fn a_set_breaking_every_clause_lists_them_in_order() {
        check(&[(
            &[
                (0, at("GK", 52, 1)),
                (4, at("LB", 2, 10)),
                (5, at("CB", 62, 12)),
                (6, at("CB", 42, 11)),
                (7, at("RB", 90, 5)),
                (9, at("LMF", 14, 25)),
                (10, at("CB", 52, 30)),
            ],
            &[
                KeeperOffSpot,
                OutsideMovableRange { slot: 4 },
                PositionNotOffered { slot: 10 },
                WidePositionTwice {
                    position: 3,
                    slots: vec![3, 7],
                },
                WidePositionTwice {
                    position: 6,
                    slots: vec![8, 9],
                },
                LineCount {
                    line: Defenders,
                    count: 8,
                },
                LineCount {
                    line: Forwards,
                    count: 0,
                },
                TooClose { slots: [1, 5] },
                TooClose { slots: [2, 6] },
            ],
        )]);
    }
}
