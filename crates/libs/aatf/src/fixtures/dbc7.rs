//! DBC7's fixtures. The sample team on PES 17 made legal under DBC7:
//! every slot's height from a table per system of the page (Height, Manlet,
//! Universal, GigaHeight with its 50 cm pool), the tiers written through
//! `Rules::apply_tier` — the golds, silvers and regulars of one medal
//! system each. Roster slots name the sample team's players: 2 and 10
//! rated 99, 5, 9 and 20 rated 88, 0 and 11 its goalkeepers at 189, which
//! the GK allowance counts as 185 in every system.

use super::*;
use crate::TeamCheck;

/// The DBC7 data block, a test ruleset.
const DBC7: &str = include_str!("../../rulesets/dbc7.rhai");

/// DBC7's rules loaded.
fn rules() -> Rules {
    Rules::from_data_block(DBC7).expect("DBC7 loads")
}

/// The `NearestSystem` warning on the team: `system` and its brackets off
/// as (bracket, count, quota).
fn nearest(system: &str, off: &[(&str, u8, u8)]) -> Violation {
    Violation {
        slot: None,
        severity: Severity::Warning,
        kind: ViolationKind::NearestSystem {
            system: system.to_string(),
            off: off
                .iter()
                .map(|(bracket, count, quota)| (bracket.to_string(), *count, *quota))
                .collect(),
        },
    }
}

/// The height rows of `team` under `rules`: its players' `Height`, the
/// quotas, the pool and the nearest system, in the check's order.
fn height_rows(rules: &Rules, team: &TeamView) -> Vec<Violation> {
    problems(rules, team)
        .into_iter()
        .filter(|violation| {
            matches!(
                violation.kind,
                ViolationKind::Height { .. }
                    | ViolationKind::BracketQuota { .. }
                    | ViolationKind::ExtraHeight { .. }
                    | ViolationKind::NearestSystem { .. }
            )
        })
        .collect()
}

// The height tables, one per system of the page, each slot's height by
// roster slot. The goalkeepers stay 189 (counted as 185).

/// Height: 6x 195 (5, 8, 14, 17, 20, 21), 6x 185 (2, 9, 10, 16 and the
/// keepers), 5x 180, 6x 175.
const HEIGHT: [u8; 23] = [
    189, 180, 185, 175, 175, 195, 175, 180, 195, 185, 185, 189, 180, 180, 195, 175, 185, 195, 180,
    175, 195, 195, 175,
];

/// Manlet: the Height table with 5, 8, 14 and 17 to the talls (10x 185)
/// and 20 and 21 to the mids (7x 180), the same 6x 175.
const MANLET: [u8; 23] = [
    189, 180, 185, 175, 175, 185, 175, 180, 185, 185, 185, 189, 180, 180, 185, 175, 185, 185, 180,
    175, 180, 180, 175,
];

/// Universal: 23x 185 (the keepers counted).
const UNIVERSAL: [u8; 23] = [
    189, 185, 185, 185, 185, 185, 185, 185, 185, 185, 185, 189, 185, 185, 185, 185, 185, 185, 185,
    185, 185, 185, 185,
];

/// GigaHeight: 7x 185 (2, 5, 9, 10, 16 and the keepers), 10x 180, 6x 175.
const GIGA: [u8; 23] = [
    189, 180, 185, 175, 175, 185, 175, 180, 180, 185, 185, 189, 180, 180, 180, 175, 185, 180, 180,
    175, 180, 180, 175,
];

/// The page's two-210 variant: 2x 210 (2, 16), 5x 185 (5, 9, 10 and the
/// keepers), 10x 180, 6x 175 — the 210s take tall slots and draw 25 each,
/// exactly the 50 cm pool.
const GIGA_210: [u8; 23] = [
    189, 180, 210, 175, 175, 185, 175, 180, 180, 185, 185, 189, 180, 180, 180, 175, 210, 180, 180,
    175, 180, 180, 175,
];

/// The Manlet table with a silver manlet: slot 9 at 175, slot 20 back to
/// the talls and slot 4 to the mids.
const MANLET_BONUS: [u8; 23] = [
    189, 180, 185, 175, 180, 185, 175, 180, 185, 175, 185, 189, 180, 180, 185, 175, 185, 185, 180,
    175, 185, 180, 175,
];

// The medal systems' slot assignments: the natural golds 2 and 10, the
// natural silvers 5, 9 and 20, each system's extras named.

const GGG: &[(u8, &str)] = &[(2, "gold"), (5, "gold"), (10, "gold")];
const GGSS: &[(u8, &str)] = &[(2, "gold"), (10, "gold"), (5, "silver"), (9, "silver")];
const GSSSS: &[(u8, &str)] = &[
    (2, "gold"),
    (10, "silver"),
    (5, "silver"),
    (9, "silver"),
    (20, "silver"),
];
const SSSSSS: &[(u8, &str)] = &[
    (2, "silver"),
    (5, "silver"),
    (9, "silver"),
    (10, "silver"),
    (16, "silver"),
    (20, "silver"),
];

/// `records` (the sample team on PES 17) made legal under DBC7: every
/// slot's height from `heights`, then the tiers the sample does not
/// already hold written through `Rules::apply_tier` — `medals`' slots to
/// their tiers, `regular` to the sample's rated players no medal table
/// counts (every other slot already rates no more than 77, the at_most
/// ceiling).
fn legalize(mut records: Records, heights: &[u8; 23], medals: &[(u8, &str)]) -> Records {
    // Premises on the sample as written.
    let dribbling = |records: &mut Records, slot: u8| records.player_mut(slot).stats.dribbling;
    assert_eq!(dribbling(&mut records, 2), 99);
    assert_eq!(dribbling(&mut records, 10), 99);
    assert_eq!(dribbling(&mut records, 5), 88);
    assert_eq!(dribbling(&mut records, 9), 88);
    assert_eq!(dribbling(&mut records, 20), 88);
    assert_eq!(dribbling(&mut records, 0), 74);
    assert_eq!(dribbling(&mut records, 11), 74);
    assert_eq!(records.player_mut(0).basic.height, 189);
    assert_eq!(records.player_mut(11).basic.height, 189);
    {
        let view = records.view();
        assert_eq!(player(&view, 0).position, Some(position("GK")));
        assert_eq!(player(&view, 11).position, Some(position("GK")));
    }
    for (slot, height) in heights.iter().enumerate() {
        records
            .player_mut(u8::try_from(slot).expect("the table has 23"))
            .basic
            .height = *height;
    }
    let mut tiers: Vec<(u8, &str)> = medals.to_vec();
    for slot in [2u8, 5, 9, 10, 20] {
        if !tiers.iter().any(|(s, _)| *s == slot) {
            tiers.push((slot, "regular"));
        }
    }
    let rules = rules();
    for (slot, tier) in tiers {
        let view = records.view();
        let player = view.player(slot).expect("a filled slot").clone();
        rules
            .apply_tier(&view, &player, tier, records.player_mut(slot))
            .expect("the tier applies");
    }
    records
}

/// `legalize` on the sample team's PES 17 build.
fn legal_records(heights: &[u8; 23], medals: &[(u8, &str)]) -> Records {
    legalize(Records::sample_on(PesVersion::Pes17), heights, medals)
}

/// `legal_records` viewed.
fn legal_team(heights: &[u8; 23], medals: &[(u8, &str)]) -> TeamView {
    legal_records(heights, medals).view()
}

/// The legal team under `heights` and `medals`, `edit` applied at `slot`.
fn edited(
    heights: &[u8; 23],
    medals: &[(u8, &str)],
    slot: u8,
    edit: impl FnOnce(&mut PlayerView),
) -> TeamView {
    let mut team = legal_team(heights, medals);
    edit(player_mut(&mut team, slot));
    team
}

/// Every height system of the page and every medal system reports
/// nothing: Height, Manlet, Universal and GigaHeight's own variant (2x
/// 210, 5x 185, 10x 180, 6x 175, drawing exactly the 50) on the GGSS
/// medals, and each medal system on the Giga heights.
#[test]
fn the_legal_teams_report_nothing() {
    let rules = rules();
    assert_eq!(rules.ruleset().name, "Divegrass Before Christmas 7");
    for (name, heights) in [
        ("Height", &HEIGHT),
        ("Manlet", &MANLET),
        ("Universal", &UNIVERSAL),
        ("GigaHeight 2x210", &GIGA_210),
    ] {
        assert_eq!(
            problems(&rules, &legal_team(heights, GGSS)),
            vec![],
            "{name}"
        );
    }
    for (name, medals) in [
        ("GGG", GGG),
        ("GGSS", GGSS),
        ("GSSSS", GSSSS),
        ("SSSSSS", SSSSSS),
    ] {
        assert_eq!(
            problems(&rules, &legal_team(&GIGA, medals)),
            vec![],
            "{name}"
        );
    }
}

/// A registered goalkeeper at 189 counts as 185 in every system: under
/// Universal (23 talls) and under GigaHeight's two-210 variant, where the
/// rest of the team draws exactly the 50 and he draws nothing (his 4 cm
/// are the special's allowance, not pool).
#[test]
fn a_keeper_at_189_counts_as_185_in_every_system() {
    let rules = rules();
    for (name, heights) in [("Universal", &UNIVERSAL), ("GigaHeight 2x210", &GIGA_210)] {
        let team = legal_team(heights, GGSS);
        assert_eq!(player(&team, 0).position, Some(position("GK")), "{name}");
        assert_eq!(player(&team, 0).height, 189, "{name}");
        assert_eq!(player(&team, 11).position, Some(position("GK")), "{name}");
        assert_eq!(player(&team, 11).height, 189, "{name}");
        assert_eq!(problems(&rules, &team), vec![], "{name}");
    }
}

/// Under the Manlet system a 175 regular may carry 82 and a 175 silver 93,
/// or anything up to it: the bonus is a one-option choice, taken when a
/// stat needs it. The
/// team's counts (10x 185, 7x 180, 6x 175) also fill GigaHeight's quotas
/// (three 185s in mid slots, drawing 15), so the systems' order is what
/// reads it as Manlet, and only Manlet gives the bonus.
/// The same regular with one stat at 83 warns inside his band of 5, but his
/// 22 others sit at the target, so the sum passes the all-82 one by a point;
/// under the Height system (no bonus) the 82s warn at target 77 — PES 17's
/// twenty-three ability stats — and the sum is 115 over the all-77 one.
#[test]
fn the_manlet_bonus_raises_the_175_targets() {
    let rules = rules();
    let mut team = legal_team(&MANLET_BONUS, GGSS);
    assert_eq!(player(&team, 3).height, 175);
    assert_eq!(player(&team, 9).height, 175);
    // The bonus taken whole: the regular (slot 3) at 82, the silver (9) at 93.
    set_tier(player_mut(&mut team, 3), 82, 8, 3);
    set_tier(player_mut(&mut team, 9), 93, 8, 3);
    assert_eq!(problems(&rules, &team), vec![]);
    // The bonus taken in part: a silver at 92 still reads as a silver, since
    // recognition sees the unbuffed 88 (three Manlet teams of the
    // maintainer's PES 2017 save carry one; a bonus every 175 must take
    // read them as regulars over 82).
    set_tier(player_mut(&mut team, 9), 92, 8, 3);
    assert_eq!(problems(&rules, &team), vec![]);
    // One stat over: dribbling 83 against the bonused target 82, a warning in
    // the band and one point over the all-82 sum for the `StatTotal`.
    let dribbling = stat("dribbling");
    player_mut(&mut team, 3).abilities[usize::from(dribbling.0)] = Some(83);
    assert_eq!(
        problems(&rules, &team),
        vec![
            warning(
                3,
                ViolationKind::Stat {
                    stat: dribbling,
                    value: 83,
                    target: 82,
                    at_most: true,
                }
            ),
            error(
                Some(3),
                ViolationKind::StatTotal {
                    total: 1887,
                    limit: 1886
                }
            ),
        ]
    );
    // The same 82s under Height: no bonus, target 77 on each stat — 23 Stat
    // warnings inside the band of 7 — and the sum 15 over the all-77 one.
    let team = edited(&HEIGHT, GGSS, 3, |p| set_tier(p, 82, 8, 3));
    let found = problems(&rules, &team);
    assert_eq!(found.len(), 24, "{found:?}");
    assert!(
        found[..23].iter().all(|v| v.slot == Some(3)
            && v.severity == Severity::Warning
            && matches!(
                &v.kind,
                ViolationKind::Stat {
                    value: 82,
                    target: 77,
                    at_most: true,
                    ..
                }
            )),
        "{found:?}"
    );
    assert_eq!(
        found[23],
        error(
            Some(3),
            ViolationKind::StatTotal {
                total: 1886,
                limit: 1771
            }
        )
    );
}

/// `player_tier` under the Manlet system: a 175 silver taking part of
/// the bonus (92, under the bonused 93 but over the unbuffed 88 the
/// recognition sees) reads `silver`, the regular at the bonused 82
/// `regular` (`the_manlet_bonus_raises_the_175_targets`).
#[test]
fn a_manlet_silver_taking_part_of_the_bonus_is_read_as_a_silver() {
    let rules = rules();
    let mut team = legal_team(&MANLET_BONUS, GGSS);
    set_tier(player_mut(&mut team, 9), 92, 8, 3);
    set_tier(player_mut(&mut team, 3), 82, 8, 3);
    for (slot, expected) in [(9, "silver"), (3, "regular"), (2, "gold"), (10, "gold")] {
        assert_eq!(tier_of(&rules, &team, slot), Some(expected), "slot {slot}");
    }
}

/// One centimetre over the pool: the two-210 variant draws exactly 50; a
/// mid at 181 draws 51, one `ExtraHeight` naming the two 210s' 25 each and
/// his 1 (every bracket still fills). The team fits no quota system, and
/// its total, 4196 cm (the keepers at 185), is 1 cm over GigaHeight's span
/// (4145 to 4195), 34 from Height's 4230, 36 from Manlet's 4160 and 59
/// from Universal's 4255: read under GigaHeight, the only system within 15
/// cm, and the nearest quota system is Height (the 181 and the 210s in no
/// bracket there, so Mid holds 9, Tall 5 and Giant none).
#[test]
fn a_centimetre_over_the_pool_is_extra_height() {
    let rules = rules();
    let team = edited(&GIGA_210, GGSS, 8, |p| p.height = 181);
    assert_eq!(
        problems(&rules, &team),
        vec![
            error(
                None,
                ViolationKind::ExtraHeight {
                    used: 51,
                    total: 50,
                    draws: vec![
                        (2, "tall".to_string(), 25),
                        (8, "mid".to_string(), 1),
                        (16, "tall".to_string(), 25),
                    ],
                }
            ),
            nearest("height", &[("mid", 9, 5), ("tall", 5, 6), ("giant", 0, 6)]),
        ]
    );
}

/// A player over the pool's `max` takes no bracket: the `Height`
/// violation and the `BracketQuota` his tall slot leaves (the other 210
/// still draws his 25, inside the pool). The total, 4196 cm, is read under
/// GigaHeight as in `a_centimetre_over_the_pool_is_extra_height`, the
/// nearest quota system Height (34 cm; Manlet 36), where the 211 and the
/// 210 take no bracket.
#[test]
fn a_player_over_the_max_is_a_height_violation() {
    let rules = rules();
    let team = edited(&GIGA_210, GGSS, 2, |p| p.height = 211);
    assert_eq!(
        problems(&rules, &team),
        vec![
            // GigaHeight's one range: its lowest quota bracket (manlet, 175)
            // to the pool's `max` (210).
            error(Some(2), no_bracket(211, &[(175, 210)], false)),
            quota(
                "giga",
                "tall",
                6,
                7,
                &[
                    (0, 185),
                    (5, 185),
                    (9, 185),
                    (10, 185),
                    (11, 185),
                    (16, 210)
                ]
            ),
            nearest("height", &[("mid", 10, 5), ("tall", 5, 6), ("giant", 0, 6)]),
        ]
    );
}

/// A player below every quota bracket takes none: a manlet at 174 is a
/// `Height` violation and the manlet quota comes up one short. The total,
/// 4144 cm, is 1 cm under GigaHeight's span and 16 from Manlet's 4160, so
/// the team is read under GigaHeight alone; the nearest quota system is
/// Manlet (Height is 86 cm away).
#[test]
fn a_player_below_the_brackets_is_a_height_violation() {
    let rules = rules();
    let team = edited(&GIGA, GGSS, 3, |p| p.height = 174);
    assert_eq!(
        problems(&rules, &team),
        vec![
            error(Some(3), no_bracket(174, &[(175, 210)], false)),
            quota(
                "giga",
                "manlet",
                5,
                6,
                &[(4, 175), (6, 175), (15, 175), (19, 175), (22, 175)]
            ),
            nearest(
                "manlet",
                &[("manlet", 5, 6), ("mid", 10, 7), ("tall", 7, 10)]
            ),
        ]
    );
}

/// The pool's tallest-first placement: 8x 185, 9x 180, 6x 175 — the
/// eighth 185 (the keepers counted) takes a mid slot and draws 5, inside
/// the 50.
#[test]
fn a_tall_player_fills_a_mid_slot() {
    let rules = rules();
    let team = edited(&GIGA, GGSS, 8, |p| p.height = 185);
    assert_eq!(problems(&rules, &team), vec![]);
}

/// A medal count no system has: three golds and a silver match none —
/// one `MedalSystem` on the team, and the regulars' count left alone (19
/// is 23 less the four named-tier players), so no `TierCount`.
#[test]
fn a_medal_count_no_system_has_is_one_violation() {
    let rules = rules();
    let medals: &[(u8, &str)] = &[(2, "gold"), (5, "gold"), (10, "gold"), (9, "silver")];
    assert_eq!(
        problems(&rules, &legal_team(&GIGA, medals)),
        vec![error(
            None,
            ViolationKind::MedalSystem {
                systems: ["GGG", "GGSS", "GSSSS", "SSSSSS"]
                    .map(String::from)
                    .to_vec(),
            }
        )]
    );
}

/// The band: an `at_most` ability stat above its target by at most the tier's
/// `stat_warning` is a warning, past it an error — a regular's 84 warns (77 + 7)
/// and his 85 errs, a silver's 93 warns (88 + 5) and his 94 errs. The sums stay
/// under the all-77 and all-88 totals (the silver trades a stat down for them),
/// so no `StatTotal` joins in.
#[test]
fn a_stat_inside_the_band_warns_past_it_errs() {
    let rules = rules();
    let attacking = stat("attacking_prowess");
    let defensive = stat("defensive_prowess");
    // The regular at 84 and at 85.
    let mut team = edited(&GIGA, GGSS, 12, |p| {
        p.abilities[usize::from(attacking.0)] = Some(84)
    });
    assert_eq!(
        problems(&rules, &team),
        vec![warning(
            12,
            ViolationKind::Stat {
                stat: attacking,
                value: 84,
                target: 77,
                at_most: true,
            }
        )]
    );
    player_mut(&mut team, 12).abilities[usize::from(attacking.0)] = Some(85);
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(12),
            ViolationKind::Stat {
                stat: attacking,
                value: 85,
                target: 77,
                at_most: true,
            }
        )]
    );
    // The silver at 93 and at 94, defensive prowess traded down for the total.
    let mut team = edited(&GIGA, GGSS, 9, |p| {
        p.abilities[usize::from(attacking.0)] = Some(93);
        p.abilities[usize::from(defensive.0)] = Some(80);
    });
    assert_eq!(
        problems(&rules, &team),
        vec![warning(
            9,
            ViolationKind::Stat {
                stat: attacking,
                value: 93,
                target: 88,
                at_most: true,
            }
        )]
    );
    player_mut(&mut team, 9).abilities[usize::from(attacking.0)] = Some(94);
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(9),
            ViolationKind::Stat {
                stat: attacking,
                value: 94,
                target: 88,
                at_most: true,
            }
        )]
    );
}

/// A Manlet regular's band narrows to the silver's 5 (his bonused 82 plus the
/// regular's 7 would reach a silver's 88): at 87 a warning, at 88 an error, the
/// target 82 both ways — he is read as a regular, since recognition sees the
/// unbonused rate and never the band.
#[test]
fn a_manlet_regulars_band_is_five() {
    let rules = rules();
    let attacking = stat("attacking_prowess");
    let defensive = stat("defensive_prowess");
    let mut team = edited(&MANLET_BONUS, GGSS, 3, |p| {
        set_tier(p, 82, 8, 3);
        // Defensive prowess traded down keeps the sum under the all-82 total.
        p.abilities[usize::from(defensive.0)] = Some(74);
        p.abilities[usize::from(attacking.0)] = Some(87);
    });
    assert_eq!(player(&team, 3).height, 175);
    assert_eq!(
        problems(&rules, &team),
        vec![warning(
            3,
            ViolationKind::Stat {
                stat: attacking,
                value: 87,
                target: 82,
                at_most: true,
            }
        )]
    );
    player_mut(&mut team, 3).abilities[usize::from(attacking.0)] = Some(88);
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(3),
            ViolationKind::Stat {
                stat: attacking,
                value: 88,
                target: 82,
                at_most: true,
            }
        )]
    );
}

/// A regular's or a silver's ability stats add up to at most the sum of their
/// targets: an all-77 player sums to 1771, and one point over it is one
/// `StatTotal` error, the raised stat inside the band a warning beside it.
#[test]
fn a_stat_total_over_the_sum_is_an_error() {
    let rules = rules();
    let physical = stat("physical_contact");
    let team = edited(&GIGA, GGSS, 12, |p| {
        set_tier(p, 77, 8, 3);
        p.abilities[usize::from(physical.0)] = Some(78);
    });
    assert_eq!(
        problems(&rules, &team),
        vec![
            warning(
                12,
                ViolationKind::Stat {
                    stat: physical,
                    value: 78,
                    target: 77,
                    at_most: true,
                }
            ),
            error(
                Some(12),
                ViolationKind::StatTotal {
                    total: 1772,
                    limit: 1771
                }
            ),
        ]
    );
}

/// A silver's ability stats add up to at most the all-88 sum (2024): exactly it
/// is legal, one point over is one `StatTotal` error, the raised stat inside the
/// band a warning beside it. A Manlet silver at 175 who takes the bonus is held
/// to the bonused sum: all-93, so one over it is `StatTotal` at limit 2139, the
/// targets after every effect and the chosen option.
#[test]
fn a_silvers_stat_total_over_the_sum_is_an_error() {
    let rules = rules();
    let physical = stat("physical_contact");
    // Exactly the all-88 sum: legal.
    let team = edited(&GIGA, GGSS, 9, |p| set_tier(p, 88, 8, 3));
    assert_eq!(problems(&rules, &team), vec![]);
    // One point over it.
    let team = edited(&GIGA, GGSS, 9, |p| {
        set_tier(p, 88, 8, 3);
        p.abilities[usize::from(physical.0)] = Some(89);
    });
    assert_eq!(
        problems(&rules, &team),
        vec![
            warning(
                9,
                ViolationKind::Stat {
                    stat: physical,
                    value: 89,
                    target: 88,
                    at_most: true,
                }
            ),
            error(
                Some(9),
                ViolationKind::StatTotal {
                    total: 2025,
                    limit: 2024
                }
            ),
        ]
    );
    // A Manlet silver who takes the bonus, held to all-93.
    let team = edited(&MANLET_BONUS, GGSS, 9, |p| {
        set_tier(p, 93, 8, 3);
        p.abilities[usize::from(physical.0)] = Some(94);
    });
    assert_eq!(
        problems(&rules, &team),
        vec![
            warning(
                9,
                ViolationKind::Stat {
                    stat: physical,
                    value: 94,
                    target: 93,
                    at_most: true,
                }
            ),
            error(
                Some(9),
                ViolationKind::StatTotal {
                    total: 2140,
                    limit: 2139
                }
            ),
        ]
    );
}

/// A 189 cm goalkeeper may hold at most one A position: rated A at CB beside GK
/// is one `APositions` error at the one-position cap the 189-only special sets;
/// rated A at CB alone passes the cap and takes the registered-position warning;
/// at 185 the same ratings are free (the cap does not apply).
#[test]
fn a_keeper_at_189_holds_no_second_a() {
    let rules = rules();
    let cb = usize::from(position("CB").0);
    let gk = usize::from(position("GK").0);
    let team = edited(&GIGA, GGSS, 0, |p| p.ratings[cb] = 2);
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(0),
            ViolationKind::APositions {
                count: 2,
                max: 1,
                rated_a: vec![position("GK"), position("CB")]
            }
        )]
    );
    // Rated A at CB alone: within the one-position cap, and the registered
    // position not rated A warns.
    let team = edited(&GIGA, GGSS, 0, |p| {
        p.ratings[gk] = 0;
        p.ratings[cb] = 2;
    });
    assert_eq!(
        problems(&rules, &team),
        vec![warning(
            0,
            special(
                "The registered position must be rated A",
                "The registered position is not rated A"
            )
        )]
    );
    let team = edited(&UNIVERSAL, GGSS, 0, |p| {
        p.height = 185;
        p.ratings[cb] = 2;
    });
    assert_eq!(problems(&rules, &team), vec![]);
}

/// The registered position not rated A is a warning, not an error: a player
/// moved to CB, where he is not rated, gets the special's `forbid` text once.
#[test]
fn a_registered_position_not_rated_a_is_a_warning() {
    let rules = rules();
    let team = edited(&GIGA, GGSS, 3, |p| p.position = Some(position("CB")));
    assert_eq!(
        problems(&rules, &team),
        vec![warning(
            3,
            special(
                "The registered position must be rated A",
                "The registered position is not rated A"
            )
        )]
    );
}

/// No card cap: a regular holding every card PES 2017 has, all 28 skills and
/// all 7 COM styles, reports nothing, the game not holding a player to the ten
/// of a regular match.
#[test]
fn a_player_holding_every_card_reports_nothing() {
    let rules = rules();
    let team = edited(&GIGA, GGSS, 12, |p| {
        p.cards = (0..28u8)
            .map(Card::Skill)
            .chain((0..7u8).map(Card::ComStyle))
            .collect()
    });
    assert_eq!(problems(&rules, &team), vec![]);
}

// Two real teams of the DBC6 save, each player's height and registered
// position by roster slot: the Team TOML's `[players.NN]` is slot NN - 1,
// which holds player `team id x 100 + NN`.

/// 893 /tsab/: 6x 195 (0, 2, 5, 6, 14, 22), 5x 185, 6x 180, 6x 175, its
/// goalkeeper at 195.
const TSAB: [(u8, &str); 23] = [
    (195, "GK"),
    (175, "RB"),
    (195, "CB"),
    (175, "LB"),
    (180, "DMF"),
    (195, "CB"),
    (195, "CB"),
    (185, "SS"),
    (185, "CMF"),
    (185, "CMF"),
    (185, "CF"),
    (175, "RB"),
    (180, "DMF"),
    (180, "DMF"),
    (195, "CB"),
    (175, "RB"),
    (175, "LB"),
    (180, "DMF"),
    (175, "LB"),
    (180, "DMF"),
    (180, "DMF"),
    (185, "SS"),
    (195, "CB"),
];

/// 897 /暁Records/: 6x 195, 4x 185, 4x 180, 7x 175, its two goalkeepers
/// (0 and 12) at 189, read as 185.
const AKATSUKI: [(u8, &str); 23] = [
    (189, "GK"),
    (185, "AMF"),
    (180, "DMF"),
    (180, "DMF"),
    (175, "CMF"),
    (195, "RWF"),
    (175, "CMF"),
    (175, "CMF"),
    (195, "LWF"),
    (195, "CB"),
    (195, "CF"),
    (180, "CMF"),
    (189, "GK"),
    (175, "CMF"),
    (175, "CMF"),
    (175, "CMF"),
    (195, "CB"),
    (180, "DMF"),
    (185, "CB"),
    (185, "CB"),
    (185, "DMF"),
    (175, "CMF"),
    (195, "CB"),
];

/// /cookie☆/ (857): 9x 185 with its goalkeeper at 189, 8x 180, 6x 175, a
/// total of 4155; Manlet wants 10 Talls and 7 Mids.
const COOKIE: [(u8, &str); 23] = [
    (189, "GK"),
    (185, "CB"),
    (185, "CB"),
    (185, "CB"),
    (175, "DMF"),
    (180, "CMF"),
    (175, "CMF"),
    (180, "CMF"),
    (185, "AMF"),
    (185, "SS"),
    (175, "CF"),
    (185, "CB"),
    (185, "CB"),
    (185, "CB"),
    (180, "CMF"),
    (180, "CMF"),
    (180, "RMF"),
    (180, "RMF"),
    (180, "LMF"),
    (180, "LMF"),
    (175, "DMF"),
    (175, "CF"),
    (175, "CF"),
];

/// The legal team with `medals`, every slot's height and registered
/// position from `players`.
fn dbc6_team(players: &[(u8, &str); 23], medals: &[(u8, &str)]) -> TeamView {
    let heights = players.map(|(height, _)| height);
    let mut team = legal_team(&heights, medals);
    for (slot, (_, registered)) in (0u8..).zip(players) {
        player_mut(&mut team, slot).position = Some(position(registered));
    }
    team
}

/// /tsab/, one 180 where Height wants a 185, fits no quota system. Its
/// total, 4225 cm, is 5 from Height's 4230 and 30 from GigaHeight's span
/// (4145 to 4195), so Height is the one system within 15 cm and the team is
/// read under it: one 180 too many in Mid and one 185 short in Tall, not
/// GigaHeight's overdrawn pool.
#[test]
fn tsab_is_read_under_height() {
    let rules = rules();
    let team = dbc6_team(&TSAB, GGSS);
    assert_eq!(
        height_rows(&rules, &team),
        vec![
            quota(
                "height",
                "mid",
                6,
                5,
                &[
                    (4, 180),
                    (12, 180),
                    (13, 180),
                    (17, 180),
                    (19, 180),
                    (20, 180)
                ]
            ),
            quota(
                "height",
                "tall",
                5,
                6,
                &[(7, 185), (8, 185), (9, 185), (10, 185), (21, 185)]
            ),
        ]
    );
    let check = rules.check_team(&team).expect("the check runs");
    assert_eq!(check.height_system.as_deref(), Some("height"));
}

/// /暁Records/, one 175 where Height wants a 180, fits no quota system; the
/// goalkeepers' 189 read as 185. Its total, 4225 cm, is 5 from Height's
/// 4230 and 30 from GigaHeight's span, so the team is read under Height:
/// seven 175s for its six Manlet slots and Mid one short.
#[test]
fn akatsuki_is_read_under_height() {
    let rules = rules();
    let team = dbc6_team(&AKATSUKI, GGSS);
    assert_eq!(
        height_rows(&rules, &team),
        vec![
            quota(
                "height",
                "manlet",
                7,
                6,
                &[
                    (4, 175),
                    (6, 175),
                    (7, 175),
                    (13, 175),
                    (14, 175),
                    (15, 175),
                    (21, 175)
                ]
            ),
            quota(
                "height",
                "mid",
                4,
                5,
                &[(2, 180), (3, 180), (11, 180), (17, 180)]
            ),
        ]
    );
    let check = rules.check_team(&team).expect("the check runs");
    assert_eq!(check.height_system.as_deref(), Some("height"));
}

/// DBC7 with its Height system's line replaced by `line`.
fn dbc7_with_height_system(line: &str) -> Rules {
    let original = "#{ id: \"height\", label: \"Height\", by_quotas: true,";
    assert!(
        DBC7.contains(original),
        "premise: DBC7's Height system line"
    );
    Rules::from_data_block(&DBC7.replacen(original, line, 1)).expect("the edited DBC7 loads")
}

/// A team read under the first system gets no `NearestSystem`, whatever
/// its height checks find: with Height a system for any team holding a
/// 195 (no quota check), /tsab/ is read under it, its quotas off, and the
/// Manlet and Universal quota systems after it are no candidates.
#[test]
fn a_team_under_the_first_system_gets_no_nearest_system() {
    let rules = dbc7_with_height_system(
        "#{ id: \"height\", label: \"Height\", any_player: #{ height_min: 195 },",
    );
    assert_eq!(
        height_rows(&rules, &dbc6_team(&TSAB, GGSS)),
        vec![
            quota(
                "height",
                "mid",
                6,
                5,
                &[
                    (4, 180),
                    (12, 180),
                    (13, 180),
                    (17, 180),
                    (19, 180),
                    (20, 180)
                ]
            ),
            quota(
                "height",
                "tall",
                5,
                6,
                &[(7, 185), (8, 185), (9, 185), (10, 185), (21, 185)]
            ),
        ]
    );
}

/// Two quota systems as far from the team's total: the earlier in the file
/// is named. The two-210 variant with a 175 at 174 and a 180 at 181 still
/// totals 4195 cm (2x 210, 5x 185 with the keepers, 9x 180, 181, 5x 175,
/// 174): 35 from Height's 4230 and 35 from Manlet's 4160, inside
/// GigaHeight's span, so it is read under GigaHeight (the 174 in no
/// bracket, the 181 drawing 1 over the pool) and Height is named, where
/// the 174, the 181 and the 210s take no bracket.
#[test]
fn a_tie_in_centimetres_names_the_earlier_system() {
    let rules = rules();
    let mut team = legal_team(&GIGA_210, GGSS);
    assert_eq!(player(&team, 3).height, 175);
    assert_eq!(player(&team, 8).height, 180);
    player_mut(&mut team, 3).height = 174;
    player_mut(&mut team, 8).height = 181;
    let rows = height_rows(&rules, &team);
    assert_eq!(
        rows.last(),
        Some(&nearest(
            "height",
            &[
                ("manlet", 5, 6),
                ("mid", 9, 5),
                ("tall", 5, 6),
                ("giant", 0, 6)
            ]
        )),
        "{rows:?}"
    );
    let check = rules.check_team(&team).expect("the check runs");
    assert_eq!(check.height_system.as_deref(), Some("giga"));
}

/// Under GigaHeight a player no bracket has room for counts toward the
/// tallest bracket he reaches and draws nothing. Twenty-three 182s reach
/// Mid (drawing 2) and Manlet (drawing 7), not Tall: the ten last by slot
/// take Mid, the next six Manlet, and the first seven count in Mid, 17 of
/// 10, never in Manlet; the pool holds the sixteen placed draws alone, 62.
#[test]
fn an_unplaced_player_counts_in_the_tallest_bracket_he_reaches_and_draws_nothing() {
    let rules = rules();
    let team = legal_team(&[182; 23], GGSS);
    let rows: Vec<Violation> = height_rows(&rules, &team)
        .into_iter()
        .filter(|row| {
            matches!(
                row.kind,
                ViolationKind::BracketQuota { .. } | ViolationKind::ExtraHeight { .. }
            )
        })
        .collect();
    let mid: Vec<(u8, u8)> = (0..7).chain(13..23).map(|slot| (slot, 182)).collect();
    let draws = (7..13)
        .map(|slot| (slot, "manlet".to_string(), 7))
        .chain((13..23).map(|slot| (slot, "mid".to_string(), 2)))
        .collect();
    assert_eq!(
        rows,
        vec![
            quota("giga", "mid", 17, 10, &mid),
            quota("giga", "tall", 0, 7, &[]),
            error(
                None,
                ViolationKind::ExtraHeight {
                    used: 62,
                    total: 50,
                    draws,
                }
            ),
        ]
    );
}

/// A quota system whose `any_player` holds for no player is no candidate:
/// with Height asking for a player of 200 or more, /tsab/ (4225 cm, 5 from
/// Height) is read under GigaHeight, 30 cm off its span with no system
/// nearer, and its nearest quota system is Universal (30 cm from its 4255),
/// not Manlet (65 from its 4160): Universal's every bracket is off, its six
/// 175s, six 180s and six 195s against quotas of 0, its five 185s against
/// 23.
#[test]
fn a_system_whose_any_player_fails_is_no_candidate() {
    let rules = dbc7_with_height_system(
        "#{ id: \"height\", label: \"Height\", by_quotas: true, any_player: #{ height_min: 200 },",
    );
    let team = dbc6_team(&TSAB, GGSS);
    let rows = height_rows(&rules, &team);
    assert_eq!(
        rows.last(),
        Some(&nearest(
            "universal",
            &[
                ("manlet", 6, 0),
                ("mid", 6, 0),
                ("tall", 5, 23),
                ("giant", 6, 0)
            ]
        )),
        "{rows:?}"
    );
    let check = rules.check_team(&team).expect("the check runs");
    assert_eq!(check.height_system.as_deref(), Some("giga"));
}

/// /cookie☆/ with its own medals (golds 8 and 9, silvers 4 and 6) and its
/// Manlets at +5 as the save holds them: the silvers at 93, the regulars
/// at 82.
fn cookie_team() -> TeamView {
    let medals: &[(u8, &str)] = &[(8, "gold"), (9, "gold"), (4, "silver"), (6, "silver")];
    let mut team = dbc6_team(&COOKIE, medals);
    for slot in [4, 6] {
        set_tier(player_mut(&mut team, slot), 93, 8, 3);
    }
    for slot in [10, 20, 21, 22] {
        set_tier(player_mut(&mut team, slot), 82, 8, 3);
    }
    team
}

/// /cookie☆/, one 180 where Manlet wants a 185, meets GigaHeight's quotas
/// and pool, where its Manlets' +5 would be six Stat total errors. Its
/// total, 4155 cm, is 5 from Manlet's 4160 and inside GigaHeight's span
/// (4145 to 4195): both are candidates, and Manlet's two bracket errors
/// beat GigaHeight's six, so it is read under Manlet: Tall 9 of 10, Mid 8
/// of 7, and no Stat total.
#[test]
fn cookie_is_read_under_manlet() {
    let rules = rules();
    let team = cookie_team();
    let check = rules.check_team(&team).expect("the check runs");
    let errors: Vec<&Violation> = check
        .violations
        .iter()
        .filter(|v| v.severity == Severity::Error)
        .collect();
    assert_eq!(
        errors,
        [
            &quota(
                "manlet",
                "mid",
                8,
                7,
                &[
                    (5, 180),
                    (7, 180),
                    (14, 180),
                    (15, 180),
                    (16, 180),
                    (17, 180),
                    (18, 180),
                    (19, 180)
                ]
            ),
            &quota(
                "manlet",
                "tall",
                9,
                10,
                &[
                    (0, 185),
                    (1, 185),
                    (2, 185),
                    (3, 185),
                    (8, 185),
                    (9, 185),
                    (11, 185),
                    (12, 185),
                    (13, 185)
                ]
            ),
        ]
    );
    assert_eq!(check.height_system.as_deref(), Some("manlet"));
    assert_eq!(check.medal_system.as_deref(), Some("GGSS"));
}

/// A tier helper reads the team under the system the check does:
/// /cookie☆/'s regular at 175 (slot 10) gets Manlet's +5, 82 on every
/// ability stat, where GigaHeight would give 77.
#[test]
fn a_tier_helper_reads_cookie_under_manlet() {
    let rules = rules();
    let team = cookie_team();
    let values = rules
        .tier_values(&team, player(&team, 10), "regular")
        .expect("the values");
    assert_eq!(values.stats.get(&stat("dribbling")), Some(&82));
    assert_eq!(values.stats.get(&stat("speed")), Some(&82));
}

/// The Manlet table with one 185 at 180 (slot 14), totalling 4155 cm: 5
/// from Manlet's 4160 and inside GigaHeight's span. Under Manlet that is
/// Tall 9 of 10 and Mid 8 of 7, two errors; under GigaHeight the counts
/// fill its quotas (two 185s in Mid slots, drawing 10), and each of
/// `bonused`, 175 regulars set to Manlet's 82, is one Stat total error.
fn manlet_one_off(bonused: &[u8]) -> TeamView {
    let mut team = legal_team(&MANLET, GGSS);
    assert_eq!(player(&team, 14).height, 185);
    player_mut(&mut team, 14).height = 180;
    for &slot in bonused {
        assert_eq!(player(&team, slot).height, 175);
        set_tier(player_mut(&mut team, slot), 82, 8, 3);
    }
    team
}

/// Between two candidates the one whose check reports fewer errors wins,
/// the nearer losing: with three bonused regulars, Manlet (5 cm away, two
/// bracket errors) beats GigaHeight (0 cm, three Stat totals).
#[test]
fn the_candidate_with_the_fewest_errors_is_read() {
    let rules = rules();
    let check = rules
        .check_team(&manlet_one_off(&[3, 4, 6]))
        .expect("the check runs");
    assert_eq!(check.height_system.as_deref(), Some("manlet"));
    let errors: Vec<&ViolationKind> = check
        .violations
        .iter()
        .filter(|v| v.severity == Severity::Error)
        .map(|v| &v.kind)
        .collect();
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors
            .iter()
            .all(|kind| matches!(kind, ViolationKind::BracketQuota { .. })),
        "{errors:?}"
    );
}

/// A tie on errors goes to the nearer system: with two bonused regulars,
/// GigaHeight (0 cm, two Stat totals) beats Manlet (5 cm, two bracket
/// errors).
#[test]
fn a_tie_on_errors_goes_to_the_nearer_system() {
    let rules = rules();
    let check = rules
        .check_team(&manlet_one_off(&[3, 4]))
        .expect("the check runs");
    assert_eq!(check.height_system.as_deref(), Some("giga"));
    let errors: Vec<&Violation> = check
        .violations
        .iter()
        .filter(|v| v.severity == Severity::Error)
        .collect();
    let total = ViolationKind::StatTotal {
        total: 1886,
        limit: 1771,
    };
    assert_eq!(
        errors,
        [&error(Some(3), total.clone()), &error(Some(4), total)]
    );
}

/// 15 cm from a baseline is within reach, 16 is not. The Manlet table with
/// three 185s at 180 (slots 8, 14, 16) totals 4145 cm, 15 from Manlet's
/// 4160 and on GigaHeight's span: with three regulars at Manlet's 82,
/// Manlet's two bracket errors beat GigaHeight's three Stat totals. A 175
/// at 174 (slot 22) makes it 4144, 16 from Manlet, and the team is read
/// under GigaHeight, though Manlet would report fewer errors (Tall, Mid
/// and Manlet off and the 174 in no bracket, 4, against the 174, the
/// Manlet quota and the three Stat totals, 5).
#[test]
fn a_team_15_cm_from_a_baseline_is_a_candidate_and_16_is_not() {
    let rules = rules();
    let mut team = legal_team(&MANLET, GGSS);
    for slot in [8, 14, 16] {
        assert_eq!(player(&team, slot).height, 185);
        player_mut(&mut team, slot).height = 180;
    }
    for slot in [3, 4, 6] {
        set_tier(player_mut(&mut team, slot), 82, 8, 3);
    }
    let check = rules.check_team(&team).expect("the check runs");
    assert_eq!(check.height_system.as_deref(), Some("manlet"), "4145 cm");
    assert_eq!(player(&team, 22).height, 175);
    player_mut(&mut team, 22).height = 174;
    let check = rules.check_team(&team).expect("the check runs");
    assert_eq!(check.height_system.as_deref(), Some("giga"), "4144 cm");
}

/// With no system within 15 cm the team is read under the last one. The
/// Height table with a 195 at 179 (slot 5) totals 4214 cm, 16 from
/// Height's 4230 and 19 above GigaHeight's span: read under GigaHeight,
/// though Height would report fewer errors (Giant one short and the 179 in
/// no bracket, 2, against GigaHeight's Mid and Manlet off and its
/// overdrawn pool, 3).
#[test]
fn a_team_no_system_is_within_15_cm_of_is_read_under_the_last() {
    let rules = rules();
    let team = edited(&HEIGHT, GGSS, 5, |p| {
        assert_eq!(p.height, 195);
        p.height = 179;
    });
    let check = rules.check_team(&team).expect("the check runs");
    assert_eq!(check.height_system.as_deref(), Some("giga"));
}

/// A legal team's check names the systems it was read under and nothing
/// else: each height system with the GGSS medals, each medal system on
/// the GigaHeight table; a medal count no system has names no medal
/// system.
#[test]
fn a_legal_teams_check_names_its_medal_and_height_systems() {
    let rules = rules();
    let named = |medal: &str, height: &str| TeamCheck {
        violations: vec![],
        medal_system: Some(medal.to_string()),
        height_system: Some(height.to_string()),
    };
    for (heights, system) in [
        (&HEIGHT, "height"),
        (&MANLET, "manlet"),
        (&UNIVERSAL, "universal"),
        (&GIGA, "giga"),
    ] {
        assert_eq!(
            rules
                .check_team(&legal_team(heights, GGSS))
                .expect("the check runs"),
            named("GGSS", system),
            "{system}"
        );
    }
    for (medals, label) in [(GGG, "GGG"), (GSSSS, "GSSSS"), (SSSSSS, "SSSSSS")] {
        assert_eq!(
            rules
                .check_team(&legal_team(&GIGA, medals))
                .expect("the check runs"),
            named(label, "giga"),
            "{label}"
        );
    }
    let medals: &[(u8, &str)] = &[(2, "gold"), (5, "gold"), (10, "gold"), (9, "silver")];
    let check = rules
        .check_team(&legal_team(&GIGA, medals))
        .expect("the check runs");
    assert_eq!(check.medal_system, None);
    assert_eq!(check.height_system.as_deref(), Some("giga"));
}

/// A team that passes under the last system keeps it while a quota system
/// is a candidate. The GigaHeight table (4145 cm: 7x 185 with the keepers'
/// 189 counted as 185, 10x 180, 6x 175) with the 175 of slot 3 at 176
/// totals 4146 cm: 14 from Manlet's 4160, within the 15, and inside
/// GigaHeight's span (4145 to 4195). The 176 takes a Manlet slot under
/// GigaHeight and draws 1 cm of the pool, so the team has no error there,
/// where under Manlet he takes no bracket and its quotas are off.
#[test]
fn a_green_giga_team_14_cm_from_manlet_stays_giga() {
    let rules = rules();
    let team = edited(&GIGA, GGSS, 3, |p| {
        assert_eq!(p.height, 175);
        p.height = 176;
    });
    // The keepers at 189 count as 185, the special's allowance of 4.
    let total: u32 = team
        .players
        .iter()
        .map(|p| {
            let allowance = if p.height == 189 && p.position == Some(position("GK")) {
                4
            } else {
                0
            };
            u32::from(p.height) - allowance
        })
        .sum();
    assert_eq!(total, 4146);
    assert_eq!(4160 - total, 14, "Manlet is within the 15 cm reach");
    assert_eq!(
        rules.check_team(&team).expect("the check runs"),
        TeamCheck {
            violations: vec![],
            medal_system: Some("GGSS".to_string()),
            height_system: Some("giga".to_string()),
        }
    );
}
