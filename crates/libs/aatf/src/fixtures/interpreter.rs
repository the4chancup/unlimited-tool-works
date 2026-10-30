//! The generic interpreter's behavior on rulesets built for the test:
//! the paths no shipped ruleset's data exercises (the per-ruleset
//! fixtures cover the shipped files). The teams are the sample team or
//! the fixture saves' team 701, edited the smallest amount.

use super::*;
use crate::ruleset::{Card, Foot};

/// `text` assembled with the current logic.
fn rules(text: &str) -> Rules {
    Rules::from_data_block(text).expect("the data block loads")
}

/// `team`'s findings on roster `slot` under `rules`, suggestions out.
fn findings_at(rules: &Rules, team: &TeamView, slot: u8) -> Vec<Violation> {
    problems(rules, team)
        .into_iter()
        .filter(|violation| violation.slot == Some(slot))
        .collect()
}

/// The sample team's view with `edit` applied to the player at `slot`.
fn view_edited(slot: u8, edit: impl FnOnce(&mut PlayerView)) -> TeamView {
    let mut team = TeamView::sample_team();
    edit(player_mut(&mut team, slot));
    team
}

/// The smallest ruleset one `forbid` special can probe a condition
/// through: every player is `regular`, so `when` alone decides whether
/// `label` is reported.
const PROBE: &str = "const RULESET = #{
    schema: 1, name: \"probe\", squad_size: 23,
    universal: #{ max_skill_cards: 10 },
    tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77 }],
    specials: [SPECIALS],
};";

/// `PROBE` with `specials` filled in.
fn probe(specials: &str) -> Rules {
    rules(&PROBE.replace("SPECIALS", specials))
}

/// A `forbid` probe: whether `when` holds for the player at `slot` shows
/// as a `Special` violation on him.
fn fires(rules: &Rules, team: &TeamView, slot: u8, label: &str) -> bool {
    problems(rules, team).iter().any(|v| {
        v.slot == Some(slot)
            && matches!(&v.kind, ViolationKind::Special { label: l, .. } if l == label)
    })
}

/// `NoTier`'s heights: a lone `height_min` runs to 255, a lone
/// `height_max` from 0, an `any` list's entries count one by one, a
/// `not` names no height the tier takes, and a range two tiers name is
/// listed once, in order of its low end. Slot 3 (175 cm) fits none.
#[test]
fn no_tier_heights_read_every_bound_once() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [
                #{ id: \"giant\", label: \"Giant\", count: 1, rate: 77,
                   when: #{ height_min: 194 } },
                #{ id: \"small\", label: \"Small\", count: 1, rate: 77,
                   when: #{ height_max: 170 } },
                #{ id: \"pair\", label: \"Pair\", rate: 77,
                   when: #{ any: [#{ height_min: 180, height_max: 180 }, #{ height_min: 194 }],
                            not: #{ height_min: 185, height_max: 185 } } },
            ],
        };",
    );
    let team = TeamView::sample_team();
    assert_eq!(player(&team, 3).height, 175);
    assert_eq!(
        findings_at(&rules, &team, 3),
        vec![error(
            Some(3),
            ViolationKind::NoTier {
                heights: vec![(0, 170), (180, 180), (194, 255)]
            }
        )]
    );
}

/// The weak-foot suggestion reads the special's limits, not the tier's:
/// a `weak_foot` replacement raising both halves to 4 reports the
/// player's 2 as unused, once for usage and once for accuracy.
#[test]
fn a_specials_weak_foot_limits_report_the_suggestion() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77,
                        weak_foot: #{ usage: 2, accuracy: 2 } }],
            specials: [#{ label: \"four-four\", then: #{ weak_foot: #{ usage: 4, accuracy: 4 } } }],
            suggestions: [\"weak_foot\"],
        };",
    );
    let team = view_edited(3, |p| {
        p.weak_foot_usage = 2;
        p.weak_foot_accuracy = 2;
    });
    let found = rules.check_team(&team).expect("the check runs");
    let suggested: Vec<&ViolationKind> = found
        .iter()
        .filter(|v| v.slot == Some(3) && v.severity == Severity::Suggestion)
        .map(|v| &v.kind)
        .collect();
    assert_eq!(
        suggested,
        [
            &unused("weak_foot_usage", 2, 4),
            &unused("weak_foot_accuracy", 2, 4)
        ],
        "{found:?}"
    );
}

/// `ranges`: each key's value within its inclusive range on the game's
/// scale — a stat through `player.stat`, the six `PlayerValue` names
/// through their accessors. A stat the version lacks fails its range.
#[test]
fn a_special_fires_through_ranges() {
    // A stat: slot 3's speed set to 90, in [90, 99], out at [91, 99].
    let team = view_edited(3, |p| p.abilities[usize::from(stat("speed").0)] = Some(90));
    let inside = probe(
        "#{ label: \"in\", when: #{ ranges: #{ speed: [90, 99] } }, then: #{ forbid: \"x\" } }",
    );
    assert!(fires(&inside, &team, 3, "in"));
    let below = probe(
        "#{ label: \"in\", when: #{ ranges: #{ speed: [91, 99] } }, then: #{ forbid: \"x\" } }",
    );
    assert!(!fires(&below, &team, 3, "in"));
    // `age` and `form` read the accessors, on the game's scale.
    let team = view_edited(3, |p| p.age = 20);
    let inside = probe(
        "#{ label: \"in\", when: #{ ranges: #{ age: [15, 20] } }, then: #{ forbid: \"x\" } }",
    );
    assert!(fires(&inside, &team, 3, "in"));
    let below = probe(
        "#{ label: \"in\", when: #{ ranges: #{ age: [15, 19] } }, then: #{ forbid: \"x\" } }",
    );
    assert!(!fires(&below, &team, 3, "in"));
    let team = view_edited(3, |p| p.form = 8);
    let inside =
        probe("#{ label: \"in\", when: #{ ranges: #{ form: [8, 8] } }, then: #{ forbid: \"x\" } }");
    assert!(fires(&inside, &team, 3, "in"));
    let below =
        probe("#{ label: \"in\", when: #{ ranges: #{ form: [1, 7] } }, then: #{ forbid: \"x\" } }");
    assert!(!fires(&below, &team, 3, "in"));
    // The four other `PlayerValue` names read their accessors, same
    // shape: `[v, v]` fires, `[v + 1, 99]` does not.
    for (name, value) in [
        ("weight", 72),
        ("injury_resistance", 2),
        ("weak_foot_usage", 3),
        ("weak_foot_accuracy", 3),
    ] {
        let team = view_edited(3, |p| match name {
            "weight" => p.weight = value,
            "injury_resistance" => p.injury_resistance = value,
            "weak_foot_usage" => p.weak_foot_usage = value,
            _ => p.weak_foot_accuracy = value,
        });
        let inside = probe(&format!(
            "#{{ label: \"in\", when: #{{ ranges: #{{ {name}: [{value}, {value}] }} }}, then: #{{ forbid: \"x\" }} }}"
        ));
        assert!(fires(&inside, &team, 3, "in"), "{name}");
        let above = probe(&format!(
            "#{{ label: \"in\", when: #{{ ranges: #{{ {name}: [{}, 99] }} }}, then: #{{ forbid: \"x\" }} }}",
            value + 1
        ));
        assert!(!fires(&above, &team, 3, "in"), "{name}");
    }
    // `physical_contact` is a stat the sample team's PES 15 lacks: `()`
    // fails the range for every player.
    let missing = probe(
        "#{ label: \"in\", when: #{ ranges: #{ physical_contact: [1, 99] } }, then: #{ forbid: \"x\" } }",
    );
    let found = problems(&missing, &TeamView::sample_team());
    assert!(
        !found
            .iter()
            .any(|v| matches!(v.kind, ViolationKind::Special { .. })),
        "{found:?}"
    );
}

/// `strong_foot` compares with `player.strong_foot`'s label; a player
/// whose byte is past 1 holds neither.
#[test]
fn a_special_fires_through_strong_foot() {
    let right =
        probe("#{ label: \"r\", when: #{ strong_foot: \"right\" }, then: #{ forbid: \"x\" } }");
    let left =
        probe("#{ label: \"l\", when: #{ strong_foot: \"left\" }, then: #{ forbid: \"x\" } }");
    let team = view_edited(3, |p| p.strong_foot = Some(Foot::Right));
    assert!(fires(&right, &team, 3, "r"));
    assert!(!fires(&left, &team, 3, "l"));
    let team = view_edited(3, |p| p.strong_foot = None);
    assert!(!fires(&right, &team, 3, "r"));
}

/// `any` holds when at least one entry does, and an additive effect of
/// its special applies once to a player who meets two of them.
#[test]
fn a_special_fires_through_any_and_applies_once() {
    let special = probe(
        "#{ label: \"any\", when: #{ any: [#{ captain: true }, #{ strong_foot: \"right\" }] },
           then: #{ forbid: \"x\" } }",
    );
    // Only the second entry holds: slot 3 is no captain (slot 10 is).
    let team = view_edited(3, |p| p.strong_foot = Some(Foot::Right));
    assert!(fires(&special, &team, 3, "any"));
    // Neither entry holds.
    let team = view_edited(3, |p| p.strong_foot = Some(Foot::Left));
    assert!(!fires(&special, &team, 3, "any"));
    // Both entries hold (a right-footed RB rated A there alone): the
    // `cards: 1` effect applies once — the limit rises by 1, not 2.
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77,
                        cards: 3, a_positions: 1 }],
            specials: [#{ label: \"any\",
                when: #{ any: [#{ strong_foot: \"right\" }, #{ positions: [\"RB\"] }] },
                then: #{ cards: 1 } }],
        };",
    );
    let team = view_edited(3, |p| {
        assert_eq!(p.position.expect("a position").label(), "RB");
        assert_eq!(p.ratings.iter().filter(|r| **r == 2).count(), 1);
        p.strong_foot = Some(Foot::Right);
    });
    let limits = rules
        .card_limits(&team, team.player(3).expect("slot 3"), "regular")
        .expect("limits");
    assert_eq!(limits.limit, 4);
}

/// `form` and `injury_resistance` add to the tier's targets wherever
/// they are read: `tier_values`, the check and the suggestion.
#[test]
fn form_and_injury_resistance_effects_move_the_targets() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77,
                        form: 4, injury_resistance: 1 }],
            specials: [#{ label: \"fit\", then: #{ form: 2, injury_resistance: 1 } }],
            suggestions: [\"injury_resistance\"],
        };",
    );
    // A regular at the tier's own form 4 and injury resistance 1 reads
    // targets of 6 and 2.
    let team = view_edited(3, |p| {
        p.form = 4;
        p.injury_resistance = 1;
    });
    let values = rules
        .tier_values(&team, team.player(3).expect("slot 3"), "regular")
        .expect("values");
    assert_eq!((values.form, values.injury_resistance), (6, 2));
    let found = problems(&rules, &team);
    assert!(
        found.iter().any(|v| v.slot == Some(3)
            && v.kind
                == ViolationKind::Form {
                    value: 4,
                    target: 6
                }),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|v| v.slot == Some(3)
                && v.kind == ViolationKind::InjuryResistance { value: 1, max: 2 }),
        "{found:?}"
    );
    // The suggestion reads the raised limit, not the tier's 1.
    assert!(
        rules
            .check_team(&team)
            .expect("the check runs")
            .iter()
            .any(|v| v.slot == Some(3) && v.kind == unused("injury_resistance", 1, 2)),
        "the suggestion sees the effect"
    );
}

/// A tier's `name_color`: its players' names must carry it, and a
/// player of a tier that sets none must carry none of the codes the
/// tiers set (the VTL11 rule). `apply_tier` writes or strips the
/// leading code so the checker accepts the result.
#[test]
fn a_tiers_name_color_is_checked_and_written() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [
                #{ id: \"gold\", label: \"Gold\", counts_as: \"regular\",
                   when: #{ positions: [\"GK\"] },
                   rate: 77, name_color: \"cc9900ff\" },
                #{ id: \"silver\", label: \"Silver\", counts_as: \"regular\",
                   when: #{ positions: [\"CB\"] },
                   rate: 77, name_color: \"c0c0c0ff\" },
                #{ id: \"regular\", label: \"Regular\", rate: 77 },
            ],
        };",
    );
    let name_color_of = |team: &TeamView, slot: u8| {
        problems(&rules, team)
            .into_iter()
            .filter(|v| v.slot == Some(slot))
            .find_map(|v| match v.kind {
                ViolationKind::NameColor { .. } => Some(v.kind),
                _ => None,
            })
    };
    let name_colors = |team: &TeamView| {
        problems(&rules, team)
            .into_iter()
            .filter(|v| matches!(v.kind, ViolationKind::NameColor { .. }))
            .count()
    };
    // A gold (slot 0, registered GK — the sample holds a second at 11)
    // without a code, a regular (slot 3) carrying silver's, and a
    // regular (slot 4) carrying an unrelated one, which reports
    // nothing; the sample's CBs (8, 14, 17, 20) are the silvers.
    let mut team = TeamView::sample_team();
    player_mut(&mut team, 3).name_color = Some("c0c0c0ff".to_string());
    player_mut(&mut team, 4).name_color = Some("12345678".to_string());
    assert_eq!(
        name_color_of(&team, 0),
        Some(ViolationKind::NameColor {
            color: None,
            expected: Some("cc9900ff".to_string())
        })
    );
    assert_eq!(
        name_color_of(&team, 3),
        Some(ViolationKind::NameColor {
            color: Some("c0c0c0ff".to_string()),
            expected: None
        })
    );
    assert_eq!(name_color_of(&team, 4), None);
    for (slot, expected) in [
        (8, "c0c0c0ff"),
        (11, "cc9900ff"),
        (14, "c0c0c0ff"),
        (17, "c0c0c0ff"),
        (20, "c0c0c0ff"),
    ] {
        assert_eq!(
            name_color_of(&team, slot),
            Some(ViolationKind::NameColor {
                color: None,
                expected: Some(expected.to_string())
            }),
            "{slot}"
        );
    }
    // `apply_tier` then `check_team`: each medal name gains its tier's
    // code, the regular loses silver's, the unrelated code is kept —
    // and the checker accepts them all.
    let mut records = Records::sample();
    assert!(
        pes_savefile::model::names::name_color(&records.player_mut(0).name).is_none(),
        "the gold's name carries no leading code yet"
    );
    records.player_mut(3).name = "\u{11}cc0c0c0ffNAME".to_string();
    records.player_mut(4).name = "\u{11}c12345678NAME".to_string();
    for (slot, tier) in [
        (0u8, "gold"),
        (8, "silver"),
        (11, "gold"),
        (14, "silver"),
        (17, "silver"),
        (20, "silver"),
        (3, "regular"),
        (4, "regular"),
    ] {
        let view = records.view();
        let player = view.player(slot).expect("filled").clone();
        rules
            .apply_tier(&view, &player, tier, records.player_mut(slot))
            .expect("the tier applies");
    }
    assert_eq!(name_colors(&records.view()), 0);
    assert!(
        records.player_mut(0).name.starts_with("\u{11}ccc9900ff"),
        "{:?}",
        records.player_mut(0).name
    );
    assert!(
        records.player_mut(8).name.starts_with("\u{11}cc0c0c0ff"),
        "{:?}",
        records.player_mut(8).name
    );
    assert_eq!(records.player_mut(3).name, "NAME");
    assert_eq!(records.player_mut(4).name, "\u{11}c12345678NAME");
}

/// A card free by name does not spend its group's free allowance:
/// holding only Long Ranger (a COM style `free_cards` covers) under a
/// tier freeing one `com` card, the `com` suggestion reports 0 of 1.
#[test]
fn a_card_free_by_name_does_not_spend_the_group_allowance() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77,
                        free: #{ com: 1 } }],
            specials: [#{ label: \"lr\", then: #{ free_cards: [\"long_ranger\"] } }],
            suggestions: [\"com\"],
        };",
    );
    let team = view_edited(3, |p| p.cards = vec![card("long_ranger")]);
    let found = suggestions_for(&rules, &team, 3);
    assert!(found.contains(&unused("com", 0, 1)), "{found:?}");
}

/// A `free` choice option is not "used" by a card already free by name:
/// holding Long Ranger, a second `com` card the tier's one free `com`
/// covers and a paid card over the zero base limit, the choice resolves
/// to its `cards` option and the check reports nothing on the slot.
#[test]
fn a_free_option_is_not_used_by_a_card_free_by_name() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                        injury_resistance: 1, weak_foot: #{ usage: 1, accuracy: 1 },
                        free: #{ com: 1 } }],
            specials: [
                #{ label: \"lr\", then: #{ free_cards: [\"long_ranger\"] } },
                #{ label: \"pick\",
                   then: #{ choice: [#{ free: #{ com: 1 } }, #{ cards: 1 }] } },
            ],
            suggestions: [\"com\", \"choices\"],
        };",
    );
    let team = view_edited(3, |p| {
        set_tier(p, 77, 4, 1);
        p.weak_foot_usage = 1;
        p.weak_foot_accuracy = 1;
        p.ratings = [0; 13];
        p.cards = vec![card("long_ranger"), card("trickster"), card("captaincy")];
    });
    let limits = rules
        .card_limits(&team, team.player(3).expect("slot 3"), "regular")
        .expect("limits");
    assert_eq!(limits.limit, 1, "the `cards` option, not the `free` one");
    assert_eq!(limits.free.get("com"), Some(&1));
    let found = rules.check_team(&team).expect("the check runs");
    assert!(
        !found.iter().any(|v| v.slot == Some(3)),
        "nothing on slot 3: {found:?}"
    );
}

/// `stat_target` replaces the tier's target for the stat everywhere the
/// target is read: the check, and `tier_values` (`apply_tier`'s and the
/// badge's numbers). The tier's `cards` and `a_positions` are wide
/// enough that only the stat decides.
#[test]
fn a_stat_target_replaces_the_tier_target() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                        injury_resistance: 1, weak_foot: #{ usage: 2, accuracy: 2 },
                        cards: 40, a_positions: 13 }],
            specials: [#{ label: \"ninety\", then: #{ stat_target: #{ speed: 90 } } }],
        };",
    );
    let speed = stat("speed");
    let edited = |speed_value: u8| {
        view_edited(3, |p| {
            set_tier(p, 77, 4, 1);
            p.weak_foot_usage = 2;
            p.weak_foot_accuracy = 2;
            p.abilities[usize::from(speed.0)] = Some(speed_value);
        })
    };
    // Slot 3 at speed 90: the special's target met, nothing on him.
    assert_eq!(findings_at(&rules, &edited(90), 3), vec![]);
    // Speed 89: off the special's 90, not the tier's 77.
    let team = edited(89);
    assert_eq!(
        findings_at(&rules, &team, 3),
        vec![error(
            Some(3),
            ViolationKind::Stat {
                stat: speed,
                value: 89,
                target: 90,
                at_most: false
            }
        )]
    );
    let values = rules
        .tier_values(&team, team.player(3).expect("slot 3"), "regular")
        .expect("values");
    assert_eq!(values.stats.get(&speed), Some(&90));
}

/// `max_a_positions` caps the A ratings where the tier's `a_positions`
/// allowance does not: over the cap is an `APositions` violation, and
/// `card_limits` reports the cap. The tier's `cards` and `a_positions`
/// are wide enough that only the cap decides.
#[test]
fn max_a_positions_caps_the_a_positions() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                        injury_resistance: 1, weak_foot: #{ usage: 2, accuracy: 2 },
                        cards: 40, a_positions: 13 }],
            specials: [#{ label: \"cap\", then: #{ max_a_positions: 2 } }],
        };",
    );
    let cb = position("CB");
    let cmf = position("CMF");
    // Slot 3 rated A at two positions (RB's plus CB): at the cap,
    // nothing on him.
    let two = view_edited(3, |p| {
        set_tier(p, 77, 4, 1);
        p.weak_foot_usage = 2;
        p.weak_foot_accuracy = 2;
        assert_eq!(p.ratings.iter().filter(|rating| **rating == 2).count(), 1);
        p.ratings[usize::from(cb.0)] = 2;
    });
    assert_eq!(findings_at(&rules, &two, 3), vec![]);
    // A third A position: over the cap.
    let three = view_edited(3, |p| {
        set_tier(p, 77, 4, 1);
        p.weak_foot_usage = 2;
        p.weak_foot_accuracy = 2;
        p.ratings[usize::from(cb.0)] = 2;
        p.ratings[usize::from(cmf.0)] = 2;
    });
    assert_eq!(
        findings_at(&rules, &three, 3),
        vec![error(
            Some(3),
            ViolationKind::APositions {
                count: 3,
                max: 2,
                rated_a: vec![position("CB"), position("RB"), position("CMF")]
            }
        )]
    );
    let limits = rules
        .card_limits(&three, three.player(3).expect("slot 3"), "regular")
        .expect("limits");
    assert_eq!(limits.max_a_positions, Some(2));
}

/// A `free_cards` option is used when, without it, the paid cards are
/// over the limit and with it fewer are paid. The choice takes the first
/// option the player uses, so where freeing lowers nothing the `cards`
/// option raises the limit instead.
#[test]
fn a_free_cards_option_is_used_when_it_lowers_an_over_limit_paid_count() {
    let with_free = |free: &str| {
        rules(&format!(
            "const RULESET = #{{
                schema: 1, name: \"x\", squad_size: 23,
                universal: #{{ max_skill_cards: 10 }},
                tiers: [#{{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                            injury_resistance: 1, weak_foot: #{{ usage: 2, accuracy: 2 }},
                            cards: 3, a_positions: 1{free} }}],
                card_groups: #{{ tricks: [\"scissors_feint\"] }},
                specials: [#{{ label: \"pick\",
                    then: #{{ choice: [#{{ free_cards: [\"scissors_feint\"] }}, #{{ cards: 1 }}] }} }}],
            }};"
        ))
    };
    let holding = |held: &[&str]| {
        let held: Vec<Card> = held.iter().map(|name| card(name)).collect();
        view_edited(3, |p| {
            set_tier(p, 77, 4, 1);
            p.weak_foot_usage = 2;
            p.weak_foot_accuracy = 2;
            p.cards = held;
        })
    };
    // `tricks` covers Scissors Feint: freeing it by name pays for no
    // extra card, so the `free_cards` option is not used and the
    // `cards` one is, paid 4 against a raised limit of 4.
    let rules = with_free(", free: #{ tricks: 1 }");
    let team = holding(&[
        "scissors_feint",
        "malicia",
        "captaincy",
        "super_sub",
        "fighting_spirit",
    ]);
    assert_eq!(findings_at(&rules, &team, 3), vec![]);
    // No free allowance: Scissors Feint is paid and freeing it lowers
    // the paid count, so the option is used, paid 3 against 3.
    let rules = with_free("");
    let team = holding(&["scissors_feint", "malicia", "captaincy", "super_sub"]);
    assert_eq!(findings_at(&rules, &team, 3), vec![]);
    // A fourth outside skill: the first option is still the one taken
    // (the choice takes the first used option, not the best), paid 4
    // against 3.
    let team = holding(&[
        "scissors_feint",
        "malicia",
        "captaincy",
        "super_sub",
        "fighting_spirit",
    ]);
    assert_eq!(
        findings_at(&rules, &team, 3),
        vec![error(Some(3), ViolationKind::Cards { paid: 4, limit: 3 })]
    );
}

/// A `free_cards` option is not used at the limit: paid exactly at it
/// is not over it, so the choice passes to the `weak_foot` option the
/// player needs.
#[test]
fn a_free_cards_option_is_not_used_at_the_limit() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                        injury_resistance: 1, weak_foot: #{ usage: 2, accuracy: 2 },
                        cards: 3, a_positions: 1 }],
            specials: [#{ label: \"pick\",
                then: #{ choice: [#{ free_cards: [\"scissors_feint\"] },
                                  #{ weak_foot: #{ usage: 4, accuracy: 4 } }] } }],
        };",
    );
    // Paid 3 against 3: the `free_cards` option would take the choice
    // only if being over the limit were not required.
    let team = view_edited(3, |p| {
        set_tier(p, 77, 4, 1);
        p.weak_foot_usage = 4;
        p.weak_foot_accuracy = 4;
        p.cards = vec![card("scissors_feint"), card("malicia"), card("captaincy")];
    });
    assert_eq!(findings_at(&rules, &team, 3), vec![]);
}

/// A `free_cards` option is also used when its card is covered by a
/// group's allowance: freeing it by name releases the allowance onto a
/// paid groupmate, so with the option fewer cards are paid. Without it
/// the paid count is over the limit here; with it, Scissors Feint is
/// free by name and `tricks`' one free card covers Flip Flap.
#[test]
fn a_covered_card_freed_by_name_frees_the_groups_allowance() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                        injury_resistance: 1, weak_foot: #{ usage: 2, accuracy: 2 },
                        cards: 3, free: #{ tricks: 1 }, a_positions: 1 }],
            card_groups: #{ tricks: [\"scissors_feint\", \"flip_flap\"] },
            specials: [#{ label: \"pick\",
                then: #{ choice: [#{ free_cards: [\"scissors_feint\"] }] } }],
        };",
    );
    // `held` order matters: Scissors Feint is listed first and takes the
    // allowance, so Flip Flap is the paid card: paid 4 against 3.
    let team = view_edited(3, |p| {
        set_tier(p, 77, 4, 1);
        p.weak_foot_usage = 2;
        p.weak_foot_accuracy = 2;
        p.cards = vec![
            card("scissors_feint"),
            card("flip_flap"),
            card("malicia"),
            card("captaincy"),
            card("super_sub"),
        ];
    });
    assert_eq!(findings_at(&rules, &team, 3), vec![]);
}

/// An option naming another tier is not used: `cards: #{ gold: 1 }`
/// raises nothing for a regular, so the choice passes to the
/// `free_cards` option that lowers his paid count.
#[test]
fn an_option_for_another_tier_is_not_used() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            stats: #{ rating: [\"speed\"] },
            universal: #{ max_skill_cards: 10 },
            tiers: [
                #{ id: \"gold\", label: \"Gold\", counts_as: \"regular\",
                   when: #{ positions: [\"GK\"] }, rate: 90, form: 8,
                   injury_resistance: 3, name_color: \"cc9900ff\",
                   weak_foot: #{ usage: 4, accuracy: 4 }, cards: 40, a_positions: 13 },
                #{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                   injury_resistance: 1, weak_foot: #{ usage: 2, accuracy: 2 },
                   cards: 3, a_positions: 13 },
            ],
            specials: [#{ label: \"pick\",
                then: #{ choice: [#{ cards: #{ gold: 1 } },
                                  #{ free_cards: [\"scissors_feint\"] }] } }],
        };",
    );
    // Paid 4 against 3: the per-tier `cards` option changes nothing for
    // a regular, so Scissors Feint's freeing applies, paid 3 against 3.
    let team = view_edited(3, |p| {
        set_tier(p, 77, 4, 1);
        p.weak_foot_usage = 2;
        p.weak_foot_accuracy = 2;
        p.cards = vec![
            card("scissors_feint"),
            card("malicia"),
            card("captaincy"),
            card("super_sub"),
        ];
    });
    assert_eq!(findings_at(&rules, &team, 3), vec![]);
}

/// A `stat_bonus` option is used per stat, as the `Stat` check reads
/// them: a VGL26 silver (the tier below verbatim, place kicking's 90
/// under the 95 rate) at 95 on every rating stat and place kicking 92
/// rates his expected 95 but is over place kicking's own target, so the
/// option raising it applies and he is legal; at 90 nothing is over and
/// the choice reports unused.
#[test]
fn a_stat_bonus_option_is_used_for_a_stat_over_its_own_target() {
    // VGL26's `stats` block, `tricks` group and silver tier verbatim
    // (rulesets/vgl26.rhai); the regulars are the rest of the squad.
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            stats: #{
                rating: [\"dribbling\", \"goalkeeping\", \"low_pass\", \"lofted_pass\", \"swerve\",
                         \"catching\", \"clearing\", \"reflexes\", \"coverage\", \"body_control\",
                         \"physical_contact\", \"kicking_power\", \"explosive_power\", \"ball_control\",
                         \"ball_winning\", \"jump\", \"place_kicking\", \"stamina\", \"speed\"],
                at_most: [\"attacking_prowess\", \"defensive_prowess\",
                          \"weak_foot_usage\", \"weak_foot_accuracy\"],
            },
            universal: #{ max_skill_cards: 41 },
            tiers: [
                #{ id: \"silver\", label: \"Silver\", count: 2,
                   when: #{ height_min: 195, height_max: 195 },
                   rate: 95, form: 8, injury_resistance: 2,
                   stats: #{ attacking_prowess: 99, finishing: 99, heading: 99,
                            place_kicking: 90, ball_winning: 90, defensive_prowess: 60 },
                   weak_foot: #{ usage: 4, accuracy: 4 },
                   cards: 8, free: #{ tricks: 12, com: 2 }, a_positions: 2 },
                #{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                   injury_resistance: 1, weak_foot: #{ usage: 2, accuracy: 2 },
                   cards: 6, a_positions: 2 },
            ],
            card_groups: #{
                tricks: [\"scissors_feint\", \"double_touch\", \"flip_flap\", \"marseille_turn\",
                         \"sombrero\", \"crossover_turn\", \"cut_behind_and_turn\", \"scotch_move\",
                         \"step_on_skill\", \"rabona\", \"no_look_pass\", \"malicia\"],
            },
            specials: [#{ label: \"pick\",
                then: #{ choice: [#{ stat_bonus: #{ place_kicking: 2 } },
                                  #{ cards: 1 }] } }],
            suggestions: [\"choices\"],
        };",
    );
    // A silver at every target but place kicking: the option raises
    // that one target.
    let silver = |place_kicking: u8| {
        view_edited(3, |p| {
            p.height = 195;
            set_tier(p, 95, 8, 2);
            p.weak_foot_usage = 4;
            p.weak_foot_accuracy = 4;
            for (name, value) in [
                ("attacking_prowess", 99),
                ("finishing", 99),
                ("heading", 99),
                ("ball_winning", 90),
                ("defensive_prowess", 60),
            ] {
                p.abilities[usize::from(stat(name).0)] = Some(value);
            }
            p.abilities[usize::from(stat("place_kicking").0)] = Some(place_kicking);
        })
    };
    // 92 over the 90 target: the option is used, the choice taken.
    let team = silver(92);
    assert_eq!(findings_at(&rules, &team, 3), vec![]);
    assert_eq!(suggestions_for(&rules, &team, 3), vec![]);
    // 90 meets it: no option is used and the choice reports unused.
    let team = silver(90);
    assert_eq!(findings_at(&rules, &team, 3), vec![]);
    assert_eq!(
        suggestions_for(&rules, &team, 3),
        vec![ViolationKind::UnusedChoice {
            special: "pick".to_string()
        }]
    );
}

/// A `free` option naming another tier is not used: `com` raised for
/// `gold` alone raises nothing for a regular, so the choice passes to
/// the option that frees a COM style for him.
#[test]
fn a_free_option_for_another_tier_is_not_used() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [
                #{ id: \"gold\", label: \"Gold\", counts_as: \"regular\",
                   when: #{ positions: [\"GK\"] }, rate: 90, form: 8,
                   injury_resistance: 3, name_color: \"cc9900ff\",
                   weak_foot: #{ usage: 4, accuracy: 4 }, cards: 40, a_positions: 13 },
                #{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                   injury_resistance: 1, weak_foot: #{ usage: 2, accuracy: 2 },
                   cards: 2, free: #{ com: 1 }, a_positions: 13 },
            ],
            specials: [#{ label: \"pick\",
                then: #{ choice: [#{ free: #{ com: #{ gold: 1 } } },
                                  #{ free: #{ com: 1 } }] } }],
            suggestions: [\"choices\"],
        };",
    );
    // Two COM styles over the one the tier frees: the second is paid,
    // three paid against two. The golds' option raises nothing for a
    // regular; the second frees his second COM style, paid 2 against 2.
    let team = view_edited(3, |p| {
        set_tier(p, 77, 4, 1);
        p.weak_foot_usage = 2;
        p.weak_foot_accuracy = 2;
        p.cards = vec![
            card("trickster"),
            card("mazing_run"),
            card("malicia"),
            card("captaincy"),
        ];
    });
    assert_eq!(findings_at(&rules, &team, 3), vec![]);
    let limits = rules
        .card_limits(&team, team.player(3).expect("slot 3"), "regular")
        .expect("limits");
    assert_eq!(limits.free.get("com"), Some(&2), "the second option's free");
}

/// An `a_positions` option naming another tier is not used: the golds'
/// +1 raises nothing for a regular over his allowance, so the choice
/// passes to the option that raises it for him.
#[test]
fn an_a_positions_option_for_another_tier_is_not_used() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [
                #{ id: \"gold\", label: \"Gold\", counts_as: \"regular\",
                   when: #{ positions: [\"GK\"] }, rate: 90, form: 8,
                   injury_resistance: 3, name_color: \"cc9900ff\",
                   weak_foot: #{ usage: 4, accuracy: 4 }, cards: 40, a_positions: 13 },
                #{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                   injury_resistance: 1, weak_foot: #{ usage: 2, accuracy: 2 },
                   cards: 3, a_positions: 1 },
            ],
            specials: [#{ label: \"pick\",
                then: #{ choice: [#{ a_positions: #{ gold: 1 } },
                                  #{ a_positions: 1 }] } }],
            suggestions: [\"choices\"],
        };",
    );
    // A second A position over the one allowed spends a card slot:
    // three paid against a limit of two. The golds' option raises
    // nothing for a regular; the second raises his allowance to two,
    // nothing extra spent, three paid against three.
    let team = view_edited(3, |p| {
        set_tier(p, 77, 4, 1);
        p.weak_foot_usage = 2;
        p.weak_foot_accuracy = 2;
        assert_eq!(p.ratings.iter().filter(|rating| **rating == 2).count(), 1);
        p.ratings[usize::from(position("CB").0)] = 2;
        p.cards = vec![card("malicia"), card("captaincy"), card("super_sub")];
    });
    assert_eq!(findings_at(&rules, &team, 3), vec![]);
    let limits = rules
        .card_limits(&team, team.player(3).expect("slot 3"), "regular")
        .expect("limits");
    assert_eq!(limits.a_positions, 2, "the second option's allowance");
}

/// A `stat_bonus` option naming another tier is not used: the golds'
/// bonus raises none of a regular's targets, so the choice passes to
/// the option that raises the target he is over.
#[test]
fn a_stat_bonus_option_for_another_tier_is_not_used() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [
                #{ id: \"gold\", label: \"Gold\", counts_as: \"regular\",
                   when: #{ positions: [\"GK\"] }, rate: 90, form: 8,
                   injury_resistance: 3, name_color: \"cc9900ff\",
                   weak_foot: #{ usage: 4, accuracy: 4 }, cards: 40, a_positions: 13 },
                #{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                   injury_resistance: 1, weak_foot: #{ usage: 2, accuracy: 2 },
                   cards: 40, a_positions: 13 },
            ],
            specials: [#{ label: \"pick\",
                then: #{ choice: [#{ stat_bonus: #{ all: #{ gold: 5 } } },
                                  #{ stat_bonus: #{ speed: 13 } }] } }],
            suggestions: [\"choices\"],
        };",
    );
    // Speed 90 over the regulars' 77: the golds' `all` raises nothing
    // for him; the second option raises speed's target to 90, met.
    let team = view_edited(3, |p| {
        set_tier(p, 77, 4, 1);
        p.weak_foot_usage = 2;
        p.weak_foot_accuracy = 2;
        p.abilities[usize::from(stat("speed").0)] = Some(90);
    });
    assert_eq!(findings_at(&rules, &team, 3), vec![]);
    let values = rules
        .tier_values(&team, team.player(3).expect("slot 3"), "regular")
        .expect("values");
    assert_eq!(
        values.stats.get(&stat("speed")),
        Some(&90),
        "the second option's bonus"
    );
}

/// A chosen option's replacing effect (`weak_foot`) gives way to one
/// a *later* special sets: choices resolve after every special, but
/// the specials' order decides which replacement stands.
#[test]
fn a_later_specials_weak_foot_wins_over_an_earlier_choice() {
    let with_specials = |specials: &str| {
        rules(&format!(
            "const RULESET = #{{
                schema: 1, name: \"x\", squad_size: 23,
                universal: #{{ max_skill_cards: 10 }},
                tiers: [#{{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                            injury_resistance: 1, weak_foot: #{{ usage: 2, accuracy: 2 }},
                            cards: 40, a_positions: 13 }}],
                specials: [{specials}],
            }};"
        ))
    };
    let choice =
        "#{ label: \"four\", then: #{ choice: [#{ weak_foot: #{ usage: 4, accuracy: 4 } }] } }";
    let replace = "#{ label: \"three\", then: #{ weak_foot: #{ usage: 3, accuracy: 3 } } }";
    let team = view_edited(3, |p| {
        set_tier(p, 77, 4, 1);
        p.weak_foot_usage = 4;
        p.weak_foot_accuracy = 4;
    });
    // The choice's special is earlier: the later special's 3/3 stands.
    let rules = with_specials(&format!("{choice}, {replace}"));
    assert_eq!(
        findings_at(&rules, &team, 3),
        vec![
            error(Some(3), ViolationKind::WeakFootUsage { value: 4, max: 3 }),
            error(
                Some(3),
                ViolationKind::WeakFootAccuracy { value: 4, max: 3 }
            ),
        ]
    );
    // Listed before the choice's special, the option applies: 4/4 met.
    let rules = with_specials(&format!("{replace}, {choice}"));
    assert_eq!(findings_at(&rules, &team, 3), vec![]);
}

/// An option whose replacing effect a later special overrides is not
/// used: applied, it raises nothing, so the choice passes to the
/// `cards` option that raises the limit the player exceeds.
#[test]
fn an_overridden_replacement_is_not_used() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                        injury_resistance: 1, weak_foot: #{ usage: 2, accuracy: 2 },
                        cards: 3, a_positions: 13 }],
            specials: [
                #{ label: \"four\",
                   then: #{ choice: [#{ weak_foot: #{ usage: 4, accuracy: 4 } },
                                     #{ cards: 1 }] } },
                #{ label: \"three\", then: #{ weak_foot: #{ usage: 3, accuracy: 3 } } },
            ],
        };",
    );
    // Paid 4 against 3: the `weak_foot` option's 4/4 cannot stand over
    // the later special's 3/3, so the `cards` option takes the choice.
    let team = view_edited(3, |p| {
        set_tier(p, 77, 4, 1);
        p.weak_foot_usage = 4;
        p.weak_foot_accuracy = 4;
        p.cards = vec![
            card("scissors_feint"),
            card("malicia"),
            card("captaincy"),
            card("super_sub"),
        ];
    });
    assert_eq!(
        findings_at(&rules, &team, 3),
        vec![
            error(Some(3), ViolationKind::WeakFootUsage { value: 4, max: 3 }),
            error(
                Some(3),
                ViolationKind::WeakFootAccuracy { value: 4, max: 3 }
            ),
        ]
    );
}

/// The same ordering holds for a chosen option's `max_a_positions`: a
/// later special's cap wins over an earlier choice's option.
#[test]
fn a_later_specials_max_a_positions_wins_over_an_earlier_choice() {
    let with_specials = |specials: &str| {
        rules(&format!(
            "const RULESET = #{{
                schema: 1, name: \"x\", squad_size: 23,
                universal: #{{ max_skill_cards: 10 }},
                tiers: [#{{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                            injury_resistance: 1, weak_foot: #{{ usage: 2, accuracy: 2 }},
                            cards: 40, a_positions: 13 }}],
                specials: [{specials}],
            }};"
        ))
    };
    let choice = "#{ label: \"cap3\", then: #{ choice: [#{ max_a_positions: 3 }] } }";
    let replace = "#{ label: \"cap2\", then: #{ max_a_positions: 2 } }";
    let cb = position("CB");
    let cmf = position("CMF");
    // Slot 3 rated A at three positions (the registered RB, plus CB and
    // CMF).
    let team = view_edited(3, |p| {
        set_tier(p, 77, 4, 1);
        p.weak_foot_usage = 2;
        p.weak_foot_accuracy = 2;
        assert_eq!(p.ratings.iter().filter(|rating| **rating == 2).count(), 1);
        p.ratings[usize::from(cb.0)] = 2;
        p.ratings[usize::from(cmf.0)] = 2;
    });
    // The choice's special is earlier: the later special's cap stands.
    let rules = with_specials(&format!("{choice}, {replace}"));
    assert_eq!(
        findings_at(&rules, &team, 3),
        vec![error(
            Some(3),
            ViolationKind::APositions {
                count: 3,
                max: 2,
                rated_a: vec![position("CB"), position("RB"), position("CMF")]
            }
        )]
    );
    // Listed after it, the option's cap applies: three A positions met.
    let rules = with_specials(&format!("{replace}, {choice}"));
    assert_eq!(findings_at(&rules, &team, 3), vec![]);
}

/// A `max_a_positions` option a later special overrides is not used:
/// applied, it raises nothing (the `from` gate drops it), so with
/// nothing else he needs the choice goes unused while the later
/// special's cap stands.
#[test]
fn an_overridden_max_a_positions_is_not_used() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                        injury_resistance: 1, weak_foot: #{ usage: 2, accuracy: 2 },
                        cards: 40, a_positions: 13 }],
            specials: [
                #{ label: \"cap3\",
                   then: #{ choice: [#{ max_a_positions: 3 }, #{ cards: 1 }] } },
                #{ label: \"cap2\", then: #{ max_a_positions: 2 } },
            ],
            suggestions: [\"choices\"],
        };",
    );
    // Three A positions over the standing cap of two: the earlier
    // choice's cap of three is overridden and its `cards` option is
    // nothing he needs, so no option is used.
    let team = view_edited(3, |p| {
        set_tier(p, 77, 4, 1);
        p.weak_foot_usage = 2;
        p.weak_foot_accuracy = 2;
        assert_eq!(p.ratings.iter().filter(|rating| **rating == 2).count(), 1);
        p.ratings[usize::from(position("CB").0)] = 2;
        p.ratings[usize::from(position("CMF").0)] = 2;
    });
    assert_eq!(
        findings_at(&rules, &team, 3),
        vec![error(
            Some(3),
            ViolationKind::APositions {
                count: 3,
                max: 2,
                rated_a: vec![position("CB"), position("RB"), position("CMF")]
            }
        )]
    );
    assert_eq!(
        suggestions_for(&rules, &team, 3),
        vec![ViolationKind::UnusedChoice {
            special: "cap3".to_string()
        }]
    );
}

/// `apply_tier` resolves the specials against the values it writes,
/// not the ones the player held: a special conditioned on `form`
/// sees the form the tier sets, so the written player passes his own
/// check.
#[test]
fn apply_tier_resolves_specials_against_the_values_it_writes() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                        injury_resistance: 1, weak_foot: #{ usage: 2, accuracy: 2 },
                        cards: 40, a_positions: 13 }],
            specials: [#{ label: \"fit\", when: #{ ranges: #{ form: [4, 4] } },
                          then: #{ stat_bonus: #{ speed: 1 } } }],
        };",
    );
    let mut records = Records::sample();
    records.player_mut(3).stats.form = 1;
    let view = records.view();
    let player = view.player(3).expect("slot 3").clone();
    assert_eq!(player.form, 2, "form 2 on the game's scale");
    rules
        .apply_tier(&view, &player, "regular", records.player_mut(3))
        .expect("the tier applies");
    let entry = records.player_mut(3);
    assert_eq!(entry.stats.form, 3, "form 4 stored");
    assert_eq!(
        entry.stats.abilities()[usize::from(stat("speed").0)],
        Some(78),
        "the form-gated bonus applied"
    );
    assert_eq!(findings_at(&rules, &records.view(), 3), vec![]);
}

/// Values that never settle — a special switching its own condition
/// off — are an `Unsettled` error, the player left as it was.
#[test]
fn apply_tier_refuses_values_that_never_settle() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                        injury_resistance: 1, weak_foot: #{ usage: 2, accuracy: 2 },
                        cards: 40, a_positions: 13 }],
            specials: [#{ label: \"swap\", when: #{ ranges: #{ form: [4, 4] } },
                          then: #{ form: 1 } }],
        };",
    );
    let mut records = Records::sample();
    records.player_mut(3).stats.form = 1;
    let before = records.player_mut(3).clone();
    let view = records.view();
    let player = view.player(3).expect("slot 3").clone();
    let err = rules
        .apply_tier(&view, &player, "regular", records.player_mut(3))
        .expect_err("values that never settle are refused");
    assert_eq!(
        err,
        crate::CheckError::Unsettled {
            label: "Regular".to_string()
        }
    );
    assert_eq!(
        err.to_string(),
        "the tier Regular writes values that never settle"
    );
    assert_eq!(records.player_mut(3), &before, "the player is untouched");
}

/// The name colour is a value the tier writes too: `apply_tier` writes
/// it onto the copy before the values settle, so a special conditioned
/// on `name_colors` resolves against the written colour, both ways.
/// `gold` gains the code and the `+1` it gates; `regular` strips the
/// code and the bonus with it.
#[test]
fn apply_tier_resolves_specials_against_the_written_name_color() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            stats: #{ rating: [\"speed\"] },
            universal: #{ max_skill_cards: 10 },
            tiers: [
                #{ id: \"gold\", label: \"Gold\", counts_as: \"regular\", rate: 90,
                   form: 8, injury_resistance: 3, name_color: \"cc9900ff\",
                   weak_foot: #{ usage: 4, accuracy: 4 }, cards: 40, a_positions: 13 },
                #{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                   injury_resistance: 1, weak_foot: #{ usage: 2, accuracy: 2 },
                   cards: 40, a_positions: 13 },
            ],
            specials: [#{ label: \"gilded\", when: #{ name_colors: [\"cc9900ff\"] },
                          then: #{ stat_bonus: #{ all: 1 } } }],
        };",
    );
    let applied = |name: &str, tier: &str| {
        let mut records = Records::sample();
        records.player_mut(3).name = name.to_string();
        let view = records.view();
        let player = view.player(3).expect("slot 3").clone();
        rules
            .apply_tier(&view, &player, tier, records.player_mut(3))
            .expect("the tier applies");
        records
    };
    // `gold` onto a colourless name: abilities at the gold rate plus 1,
    // and the checker reports nothing.
    let mut records = applied("NAME", "gold");
    for ability in records.player_mut(3).stats.abilities().iter().flatten() {
        assert_eq!(*ability, 91, "the gold rate plus the gated bonus");
    }
    assert_eq!(findings_at(&rules, &records.view(), 3), vec![]);
    // `regular` onto a name carrying gold's code: the code is stripped,
    // so the bonus is gone: abilities at the regular rate.
    let mut records = applied("\u{11}ccc9900ffNAME", "regular");
    for ability in records.player_mut(3).stats.abilities().iter().flatten() {
        assert_eq!(*ability, 77, "the plain regular rate");
    }
    assert_eq!(findings_at(&rules, &records.view(), 3), vec![]);
}

/// `team_settings`' `auto_attack_defence_levels` is compared like the
/// other auto flags: the sample team on PES 21 (the field does not
/// exist before PES 17, so `sample_on` builds it there) holds the flag
/// off, and a probe demanding `false` reports no `TeamSetting`; the flag
/// set reports the entry's.
#[test]
fn auto_attack_defence_levels_is_compared() {
    let rules = rules(
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77 }],
            team_settings: [#{ label: \"a/d levels off\",
                               auto_attack_defence_levels: false }],
        };",
    );
    let team_settings = |team: &TeamView| {
        problems(&rules, team)
            .into_iter()
            .filter(|v| matches!(v.kind, ViolationKind::TeamSetting { .. }))
            .collect::<Vec<_>>()
    };
    let mut team = Records::sample_on(PesVersion::Pes21).view();
    assert_eq!(
        team.tactics.auto.attack_defence_levels,
        Some(false),
        "the PES 21 sample holds the flag off"
    );
    assert_eq!(team_settings(&team), vec![]);
    team.tactics.auto.attack_defence_levels = Some(true);
    assert_eq!(
        team_settings(&team),
        vec![error(
            None,
            ViolationKind::TeamSetting {
                label: "a/d levels off".to_string(),
                setting: "auto_attack_defence_levels".to_string(),
                wanted: false
            }
        )]
    );
}
