//! Spring 26's fixtures: the sample team is a real team built under this
//! ruleset (team 714 of the 2026 Summer Cup save), so it is the
//! known-legal team as it stands.

use super::*;
use crate::ruleset::Card;

/// The Spring 26 data block (the ruleset of the 2026 Spring and Summer
/// cups), a template.
const SPRING26: &str = include_str!("../../rulesets/spring26.rhai");

/// Spring 26's rules loaded.
fn rules() -> Rules {
    Rules::from_data_block(SPRING26).expect("Spring 26 loads")
}

/// `Records::sample()` viewed: the sample team is legal as written.
fn legal_team() -> TeamView {
    Records::sample().view()
}

/// `legal_team` with `edit` applied to the player at `slot`.
fn edited(slot: u8, edit: impl FnOnce(&mut PlayerView)) -> TeamView {
    let mut team = legal_team();
    edit(player_mut(&mut team, slot));
    team
}

/// The legal team under the Red system: no giant (the five 194 cm
/// players — slots 5, 8, 14, 17 and 20 — become four talls and one mid),
/// and the six manlets rating 82 under the Red manlet bonus (Red quotas
/// 10 tall, 7 mid, 6 manlets).
fn red_legal_records() -> Records {
    let mut records = Records::sample();
    for (slot, height) in [(5, 185), (8, 185), (14, 185), (17, 185), (20, 180)] {
        records.player_mut(slot).basic.height = height;
    }
    for slot in [3, 4, 6, 15, 19, 22] {
        assert_eq!(records.player_mut(slot).basic.height, 175);
        set_tier_record(records.player_mut(slot), 82, 4, 1);
    }
    records
}

/// The sample team as written reports nothing: two golds at 99, three
/// silvers at 88, goalkeepers at 74, Green with 5 giants.
#[test]
fn the_sample_team_is_legal_under_the_ruleset_it_was_built_for() {
    let rules = Rules::from_data_block(SPRING26).expect("Spring 26 loads");
    assert_eq!(rules.ruleset().name, "4cc Spring 26");
    let team = TeamView::sample_team();
    assert_eq!(player(&team, 0).abilities[2], Some(74));
    assert_eq!(problems(&rules, &team), vec![]);
}

/// The Spring 26 values the official file changed: a silver at 92 is off
/// every rating stat's target (88), and a medal's weak foot usage 3 is
/// over 2.
#[test]
fn a_silver_at_the_official_rate_and_a_medals_weak_foot_usage_are_off() {
    let rules = Rules::from_data_block(SPRING26).expect("Spring 26 loads");
    let mut team = TeamView::sample_team();
    set_tier(player_mut(&mut team, 5), 92, 8, 3);
    let found = problems(&rules, &team);
    // 17 rating stats of PES 15's 19 abilities, the prowess pair above 88 too: 19.
    assert_eq!(found.len(), 19, "{found:?}");
    assert!(
        found.iter().all(|v| matches!(
            v.kind,
            ViolationKind::Stat {
                value: 92,
                target: 88,
                ..
            }
        )),
        "{found:?}"
    );
    let mut team = TeamView::sample_team();
    assert_eq!(player(&team, 5).weak_foot_usage, 2);
    player_mut(&mut team, 5).weak_foot_usage = 3;
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(5),
            ViolationKind::WeakFootUsage { value: 3, max: 2 }
        )]
    );
}

/// For every tier of the file, a player of that tier in the legal team
/// — his stats, form, injury resistance and weak foot broken — is legal
/// again after `apply_tier` (the goalkeeper's 74 target included).
#[test]
fn apply_tier_then_check_reports_nothing_for_every_tier() {
    let rules = Rules::from_data_block(SPRING26).expect("Spring 26 loads");
    let tiers: Vec<&str> = rules
        .ruleset()
        .tiers
        .iter()
        .map(|tier| tier.id.as_str())
        .collect();
    assert_eq!(tiers, ["gold", "silver", "goalkeeper", "regular"]);
    for (tier, slot) in [
        ("gold", 2),
        ("silver", 5),
        ("goalkeeper", 0),
        ("regular", 12),
    ] {
        let mut records = Records::sample();
        let entry = records.player_mut(slot);
        set_tier_record(entry, 55, 2, 3);
        entry.stats.weak_foot_usage = 0;
        entry.stats.weak_foot_accuracy = 0;
        let broken = records.view();
        assert!(!problems(&rules, &broken).is_empty(), "{tier} {slot}");
        let player = broken.player(slot).expect("filled").clone();
        rules
            .apply_tier(&broken, &player, tier, records.player_mut(slot))
            .expect("the tier applies");
        assert_eq!(problems(&rules, &records.view()), vec![], "{tier} {slot}");
    }
}

/// `card_limits` reports each tier's economy on a member nothing but
/// the captaincy free card touches — the paid limit, the A positions,
/// the free counts (the exact map, so a group named nowhere in it
/// shows) and `max_a_positions` unset — and `tier_values` the weak
/// foot. Slots: 10 for the gold (the captain — his free Captaincy
/// changes none of these numbers), 9 for the silver (the only
/// single-A medal), 0 for the goalkeeper (his one A fitting the one
/// allowance, the limit undeducted), 12 for the regular, a mid — a
/// manlet reads the "Manlets" special's raises instead: two A
/// positions and four paid, asserted last.
#[test]
fn card_limits_and_tier_values_report_the_tiers_values() {
    let rules = rules();
    let team = legal_team();
    assert_economy(
        &rules,
        &team,
        &[
            (10, "gold", 5, 1, &[("com", 2), ("tricks", 3)], (2, 4)),
            (9, "silver", 4, 1, &[("com", 1), ("tricks", 3)], (2, 4)),
            (0, "goalkeeper", 2, 1, &[], (2, 2)),
            (12, "regular", 3, 1, &[("tricks", 2)], (2, 2)),
        ],
    );
    // Slot 3 is a manlet (175 cm, the manlet bracket) and a regular
    // (the 77 rate): the "Manlets" special's +1 A position and +1 paid
    // card show in his limits — two and four, his one A inside them.
    let manlet = player(&team, 3);
    assert_eq!(manlet.height, 175);
    let limits = rules.card_limits(&team, manlet, "regular").expect("limits");
    assert_eq!((limits.a_positions, limits.limit), (2, 4));
}

/// Under the Red system the manlet bonus is for regulars and
/// goalkeepers alone: the built team's manlets rate 82. A regular
/// manlet at the unbonused 77 is off the raised 82; a goalkeeper at 175
/// (slot 11 to manlet, manlet 22 to tall, the quotas kept) is legal at
/// 79 and at 74 reads as a regular manlet, off the raised 82; a silver
/// manlet, whom the map does not name, keeps his 88.
#[test]
fn the_red_manlet_bonus_is_for_regulars_and_goalkeepers() {
    let rules = rules();
    // Premise: the red team's own build is legal.
    assert_eq!(problems(&rules, &red_legal_records().view()), vec![]);
    // A regular manlet at 77: the 17 rating stats off the raised 82.
    let mut records = red_legal_records();
    set_tier_record(records.player_mut(3), 77, 4, 1);
    let found = problems(&rules, &records.view());
    assert_eq!(found.len(), 17, "{found:?}");
    assert!(
        found.iter().all(|v| v.slot == Some(3)
            && matches!(
                v.kind,
                ViolationKind::Stat {
                    value: 77,
                    target: 82,
                    at_most: false,
                    ..
                }
            )),
        "{found:?}"
    );
    // A goalkeeper manlet (slot 11 to 175, a manlet to tall, quotas kept):
    // legal at the bonused 79; at 74 no tier's expected reaches him — the
    // bonus raises it too — and he reads as a regular manlet, off the 82.
    for rate in [79, 74] {
        let mut records = red_legal_records();
        records.player_mut(11).basic.height = 175;
        records.player_mut(22).basic.height = 185;
        let moved = records.player_mut(22);
        set_tier_record(moved, 77, 4, 1);
        moved.stats.weak_foot_usage = 1;
        moved.stats.weak_foot_accuracy = 1;
        set_tier_record(records.player_mut(11), rate, 4, 1);
        let found = problems(&rules, &records.view());
        if rate == 79 {
            assert_eq!(found, vec![], "{rate}: {found:?}");
        } else {
            assert!(!found.is_empty(), "{rate}: the unbonused rate is off");
            assert!(
                found.iter().all(|v| v.slot == Some(11)
                    && matches!(
                        v.kind,
                        ViolationKind::Stat {
                            value: 74,
                            target: 82,
                            at_most: false,
                            ..
                        }
                    )),
                "{found:?}"
            );
        }
    }
    // A silver manlet (slot 9 to 175, a manlet to tall): his 88 stands,
    // the bonus naming regular and goalkeeper alone.
    let mut records = red_legal_records();
    records.player_mut(9).basic.height = 175;
    records.player_mut(22).basic.height = 185;
    let moved = records.player_mut(22);
    set_tier_record(moved, 77, 4, 1);
    moved.stats.weak_foot_usage = 1;
    moved.stats.weak_foot_accuracy = 1;
    assert_eq!(problems(&rules, &records.view()), vec![]);
}

/// A medal made a goalkeeper — the A moved with him and his height to
/// the goalkeepers' 189 — is the special's violation alone (his tier's
/// count stands: the tier is read by rating, not position).
#[test]
fn a_medal_cannot_play_as_goalkeeper() {
    let rules = rules();
    let (gk, cf) = (position("GK"), position("CF"));
    let team = edited(2, |p| {
        assert_eq!(p.position, Some(cf));
        p.position = Some(gk);
        p.height = 189;
        p.ratings = [0; 13];
        p.ratings[usize::from(gk.0)] = 2;
    });
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(2),
            special("Medals cannot be goalkeepers", "Medals cannot play as GK")
        )]
    );
}

/// A goalkeeper at 185 still counts as tall — and that bracket is what
/// the tall-goalkeeper rule forbids: the special's violation alone (his
/// quota holds).
#[test]
fn a_tall_goalkeeper_must_be_189() {
    let rules = rules();
    assert_eq!(player(&legal_team(), 0).height, 189);
    assert_eq!(
        problems(&rules, &edited(0, |p| p.height = 185)),
        vec![error(
            Some(0),
            special(
                "Tall goalkeepers are 189",
                "GKs in this bracket must be 189cm"
            )
        )]
    );
}

/// A goalkeeper at 194 is a warning, and the tall quota he left with
/// the giant he joined; a gold (slot 2, 185 cm) at 194 the same.
#[test]
fn goalkeepers_and_golds_below_giant_are_warnings() {
    let rules = rules();
    assert_eq!(
        problems(&rules, &edited(0, |p| p.height = 194)),
        vec![
            warning(
                0,
                special(
                    "Goalkeepers below giant",
                    "GK heights cannot be 194cm or more"
                )
            ),
            quota(
                "green",
                "tall",
                5,
                6,
                &[(2, 185), (9, 185), (10, 185), (11, 185), (16, 185)]
            ),
            quota(
                "green",
                "giant",
                6,
                5,
                &[
                    (0, 194),
                    (5, 194),
                    (8, 194),
                    (14, 194),
                    (17, 194),
                    (20, 194)
                ]
            ),
        ]
    );
    assert_eq!(
        problems(&rules, &edited(2, |p| p.height = 194)),
        vec![
            warning(
                2,
                special("Gold below giant", "Gold heights cannot be 194cm or more")
            ),
            quota(
                "green",
                "tall",
                5,
                6,
                &[(0, 185), (9, 185), (10, 185), (11, 185), (16, 185)]
            ),
            quota(
                "green",
                "giant",
                6,
                5,
                &[
                    (2, 194),
                    (5, 194),
                    (8, 194),
                    (14, 194),
                    (17, 194),
                    (20, 194)
                ]
            ),
        ]
    );
}

/// Malicia is free on top of the paid limit: a regular at his three
/// (slot 12) holding it reports nothing, and a paid card on top of both
/// is over.
#[test]
fn malicia_is_a_free_card() {
    let rules = rules();
    assert_eq!(player(&legal_team(), 12).cards.len(), 3);
    assert_eq!(
        problems(&rules, &edited(12, |p| p.cards.push(card("malicia")))),
        vec![]
    );
    let team = edited(12, |p| {
        p.cards.push(card("malicia"));
        p.cards.push(card("long_range_drive"));
    });
    assert_eq!(
        problems(&rules, &team),
        vec![error(Some(12), ViolationKind::Cards { paid: 4, limit: 3 })]
    );
}

/// Captaincy is paid for everyone but the captain: on slot 12 it is
/// over the limit, and free once the armband moves to him — the former
/// captain's (slot 10) own turning paid, his gold limit still covering
/// it.
#[test]
fn the_captains_captaincy_card_is_free() {
    let rules = rules();
    let mut team = edited(12, |p| p.cards.push(card("captaincy")));
    assert_eq!(
        problems(&rules, &team),
        vec![error(Some(12), ViolationKind::Cards { paid: 4, limit: 3 })]
    );
    assert!(player(&team, 10).cards.contains(&card("captaincy")));
    team.captain = Some(12);
    assert_eq!(problems(&rules, &team), vec![]);
}

/// `captain_required` and `min_goalkeepers`: the armband gone is the
/// `Captain` violation; both goalkeepers gone is `Goalkeepers` with
/// the regular count and the tall quota they left.
#[test]
fn the_captain_and_the_goalkeepers_are_required() {
    let rules = rules();
    let mut team = legal_team();
    assert_eq!(team.captain, Some(10));
    team.captain = None;
    assert_eq!(
        problems(&rules, &team),
        vec![error(None, ViolationKind::Captain)]
    );
    let mut team = legal_team();
    team.players.retain(|p| p.position != Some(position("GK")));
    assert_eq!(team.players.len(), 21);
    assert_eq!(
        problems(&rules, &team),
        vec![
            error(
                None,
                ViolationKind::TierCount {
                    tier: "regular".to_string(),
                    count: 16,
                    expected: 18
                }
            ),
            error(
                None,
                ViolationKind::BracketQuota {
                    system: "green".to_string(),
                    bracket: "tall".to_string(),
                    count: 4,
                    quota: 6,
                    players: vec![(2, 185), (9, 185), (10, 185), (16, 185)],
                }
            ),
            error(None, ViolationKind::Goalkeepers { count: 0, min: 1 }),
        ]
    );
}

/// `no_b_ratings`, `registered_position_a` and `gk_not_second_a`: a B
/// at CB is the `BRating`; slot 3 (registered RB, A there alone) rated
/// C is the `RegisteredNotA`; slot 12's second A at GK is the
/// `GkSecondA` — a card dropped so only the flag fires.
#[test]
fn the_rating_rules_fire() {
    let rules = rules();
    let (cb, rb, gk) = (position("CB"), position("RB"), position("GK"));
    assert_eq!(
        problems(&rules, &edited(3, |p| p.ratings[usize::from(cb.0)] = 1)),
        vec![error(
            Some(3),
            // The B just given at CB.
            ViolationKind::BRating { rated_b: vec![cb] }
        )]
    );
    assert_eq!(player(&legal_team(), 3).position, Some(rb));
    assert_eq!(
        problems(&rules, &edited(3, |p| p.ratings[usize::from(rb.0)] = 0)),
        vec![error(
            Some(3),
            // RB was his one A position.
            ViolationKind::RegisteredNotA { rated_a: vec![] }
        )]
    );
    let team = edited(12, |p| {
        assert_eq!(p.cards.len(), 3);
        p.cards.pop();
        p.ratings[usize::from(gk.0)] = 2;
    });
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(12),
            // Slot 12 is rated A at CMF alone in the sample; GK is left out.
            ViolationKind::GkSecondA {
                rated_a: vec![position("CMF")]
            }
        )]
    );
}

/// `age`, `weight` and the skill-card cap: outside each bound the
/// finding names its numbers.
#[test]
fn age_weight_and_the_skill_cap_fire() {
    let rules = rules();
    for age in [14, 51] {
        assert_eq!(
            problems(&rules, &edited(3, |p| p.age = age)),
            vec![error(
                Some(3),
                ViolationKind::Age {
                    age,
                    min: 15,
                    max: 50
                }
            )],
            "{age}"
        );
    }
    // Slot 3 is 175 cm: `max(30, 175 - 129)..=175 - 81` is 46 to 94.
    for weight in [45, 95] {
        assert_eq!(
            problems(&rules, &edited(3, |p| p.weight = weight)),
            vec![error(
                Some(3),
                ViolationKind::Weight {
                    weight,
                    min: 46,
                    max: 94
                }
            )],
            "{weight}"
        );
    }
    // Eleven skills on slot 12, none of them a trick: over the game's
    // cap of ten, and over the paid limit too.
    let team = edited(12, |p| {
        p.cards = [7, 8, 9, 10, 11, 12, 13, 14, 15, 17, 18]
            .into_iter()
            .map(Card::Skill)
            .collect();
    });
    assert_eq!(
        problems(&rules, &team),
        vec![
            error(Some(12), ViolationKind::Cards { paid: 11, limit: 3 }),
            error(Some(12), ViolationKind::SkillCardCap { count: 11, max: 10 }),
        ]
    );
}
