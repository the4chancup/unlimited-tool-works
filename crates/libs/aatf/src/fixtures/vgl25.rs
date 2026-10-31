//! VGL25's fixtures. No cup save exists for VGL25, so the legal team is
//! built from the sample team by hand; `legal_records` lists the edits.
//! Roster slots name the sample team's players: 2 and 10 its golds
//! (99), 5, 9 and 20 its silvers (88), 0 and 11 its goalkeepers (189 cm,
//! counted as 185), 3, 4, 6, 15, 19 and 22 its manlets (175 cm).

use super::*;
use crate::ruleset::Card;

/// The VGL25 data block, a test ruleset.
const VGL25: &str = include_str!("../../rulesets/vgl25.rhai");

/// VGL25's rules loaded.
fn rules() -> Rules {
    Rules::from_data_block(VGL25).expect("VGL25 loads")
}

/// The sample team made legal under VGL25:
///
/// - tiers through `Rules::apply_tier` on the records: the golds (2, 10)
///   and silvers (5, 9, 20) re-targeted (VGL's medals carry injury
///   resistance 2, not 3), the goalkeepers (0, 11) made regulars at 77
///   (goalkeepers are regulars here);
/// - heights by hand: the silver giants (5, 20) down to 180 and 8, 14,
///   16, 17, 18 and 21 to 190, VGL's giant; Purple's quotas are 6
///   manlets (175), 6 mids (180), 5 tall (185, the 189 cm goalkeepers
///   counted), 6 giants and 0 colossal;
/// - cards and A positions untouched: the sample's already fit (slot
///   10's Captaincy free as the captain's).
fn legal_records() -> Records {
    let rules = rules();
    let mut records = Records::sample();
    // Premises on the sample as written.
    let dribbling = |records: &mut Records, slot: u8| records.player_mut(slot).stats.dribbling;
    assert_eq!(dribbling(&mut records, 2), 99);
    assert_eq!(dribbling(&mut records, 10), 99);
    assert_eq!(dribbling(&mut records, 5), 88);
    assert_eq!(dribbling(&mut records, 9), 88);
    assert_eq!(dribbling(&mut records, 20), 88);
    assert_eq!(dribbling(&mut records, 0), 74);
    assert_eq!(dribbling(&mut records, 11), 74);
    for (slot, height) in [
        (0, 189),
        (11, 189),
        (5, 194),
        (8, 194),
        (14, 194),
        (16, 185),
        (17, 194),
        (18, 180),
        (20, 194),
        (21, 180),
    ] {
        assert_eq!(records.player_mut(slot).basic.height, height, "{slot}");
    }
    for slot in [5, 20] {
        records.player_mut(slot).basic.height = 180;
    }
    for slot in [8, 14, 16, 17, 18, 21] {
        records.player_mut(slot).basic.height = 190;
    }
    for (slot, tier) in [
        (2, "gold"),
        (10, "gold"),
        (5, "silver"),
        (9, "silver"),
        (20, "silver"),
        (0, "regular"),
        (11, "regular"),
    ] {
        let view = records.view();
        let player = view.player(slot).expect("a filled slot").clone();
        rules
            .apply_tier(&view, &player, tier, records.player_mut(slot))
            .expect("the tier applies");
    }
    records
}

/// `legal_records` viewed.
fn legal_team() -> TeamView {
    legal_records().view()
}

/// `legal_team` with `edit` applied to the player at `slot`.
fn edited(slot: u8, edit: impl FnOnce(&mut PlayerView)) -> TeamView {
    let mut team = legal_team();
    edit(player_mut(&mut team, slot));
    team
}

#[test]
fn the_legal_team_reports_nothing() {
    let rules = rules();
    assert_eq!(rules.ruleset().name, "/vg/ League 25");
    assert_eq!(problems(&rules, &legal_team()), vec![]);
}

/// For every tier of the file, a player of that tier in the legal team,
/// his stats, form, injury resistance and weak foot broken, is legal
/// again after `apply_tier`.
#[test]
fn apply_tier_then_check_reports_nothing_for_every_tier() {
    let rules = rules();
    let tiers: Vec<&str> = rules
        .ruleset()
        .tiers
        .iter()
        .map(|tier| tier.id.as_str())
        .collect();
    assert_eq!(tiers, ["gold", "silver", "regular"]);
    for (tier, slot) in [("gold", 2), ("silver", 5), ("regular", 12)] {
        let mut records = legal_records();
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

/// `card_limits` reports each tier's economy on a member no special
/// touches — the paid limit, the A positions, the free counts (the
/// exact map, so a group named nowhere in it shows) and
/// `max_a_positions` unset — and `tier_values` the weak foot. Slots:
/// 2 for the gold (two A positions inside the two allowed), 5 for the
/// silver (likewise), 12 for the regular, a mid — a manlet would read
/// the "Manlets" special's raises instead.
#[test]
fn card_limits_and_tier_values_report_the_tiers_values() {
    let rules = rules();
    let team = legal_team();
    assert_economy(
        &rules,
        &team,
        &[
            (2, "gold", 5, 2, &[("com", 1), ("tricks", 12)], (4, 4)),
            (5, "silver", 4, 2, &[("com", 1), ("tricks", 12)], (4, 4)),
            (12, "regular", 3, 2, &[("tricks", 12)], (2, 2)),
        ],
    );
}

/// A regular manlet (slot 3) rates 77, or 82 with the optional manlet
/// bonus taken; at 80 the bonus applies (his rating is over the tier's
/// 77 without it) and every rating stat is off the raised target 82:
/// 17 of PES 15's 19 abilities, the `at_most` prowess pair sitting below
/// it.
#[test]
fn a_regular_manlet_rates_77_or_82() {
    let rules = rules();
    assert_eq!(player(&legal_team(), 3).height, 175);
    for rate in [77, 82] {
        assert_eq!(
            problems(&rules, &edited(3, |p| set_tier(p, rate, 4, 1))),
            vec![],
            "{rate}"
        );
    }
    let found = problems(&rules, &edited(3, |p| set_tier(p, 80, 4, 1)));
    assert_eq!(found.len(), 17, "{found:?}");
    assert!(
        found.iter().all(|v| matches!(
            v.kind,
            ViolationKind::Stat {
                value: 80,
                target: 82,
                at_most: false,
                ..
            }
        )),
        "{found:?}"
    );
}

/// A manlet at 77 takes no option of the bonus choice: the `choices`
/// suggestion reports it (with the other allowances he does not use up).
#[test]
fn a_manlet_not_taking_the_bonus_gets_the_choice_suggestion() {
    let rules = rules();
    let team = legal_team();
    assert_eq!(
        suggestions_for(&rules, &team, 3),
        vec![
            unused("cards", 3, 4),
            unused("a_positions", 1, 3),
            unused("free_cards", 0, 1),
            ViolationKind::UnusedChoice {
                special: "Manlet bonus".to_string()
            },
        ]
    );
}

/// A manlet below the bracket's top is legal, and the `height`
/// suggestion reports the unused centimetres: slot 3 at 170 stays in the
/// manlet bracket (`up_to` 175) and reads 170 of the 175 it allows.
#[test]
fn a_manlet_below_the_brackets_top_gets_the_height_suggestion() {
    let rules = rules();
    assert_eq!(player(&legal_team(), 3).height, 175);
    let team = edited(3, |p| p.height = 170);
    assert_eq!(problems(&rules, &team), vec![]);
    assert_eq!(
        suggestions_for(&rules, &team, 3),
        vec![
            unused("cards", 3, 4),
            unused("a_positions", 1, 3),
            unused("height", 170, 175),
            unused("free_cards", 0, 1),
            ViolationKind::UnusedChoice {
                special: "Manlet bonus".to_string()
            },
        ]
    );
}

/// A medal at 190 (slot 9, a tall silver) or a goalkeeper at 190 (slot
/// 0) is forbidden; each is a giant, one tall player short and one giant
/// over.
#[test]
fn medals_and_goalkeepers_cannot_be_giants() {
    let rules = rules();
    assert_eq!(
        problems(&rules, &edited(9, |p| p.height = 190)),
        vec![
            error(
                Some(9),
                special("Medals cannot be giants", "Medals cannot be 190cm")
            ),
            quota(
                "purple",
                "tall",
                4,
                5,
                &[(0, 185), (2, 185), (10, 185), (11, 185)]
            ),
            quota(
                "purple",
                "giant",
                7,
                6,
                &[
                    (8, 190),
                    (9, 190),
                    (14, 190),
                    (16, 190),
                    (17, 190),
                    (18, 190),
                    (21, 190)
                ]
            ),
        ]
    );
    assert_eq!(
        problems(&rules, &edited(0, |p| p.height = 190)),
        vec![
            error(
                Some(0),
                special("Goalkeepers cannot be giants", "GKs cannot be 190cm")
            ),
            quota(
                "purple",
                "tall",
                4,
                5,
                &[(2, 185), (9, 185), (10, 185), (11, 185)]
            ),
            quota(
                "purple",
                "giant",
                7,
                6,
                &[
                    (0, 190),
                    (8, 190),
                    (14, 190),
                    (16, 190),
                    (17, 190),
                    (18, 190),
                    (21, 190)
                ]
            ),
        ]
    );
}

/// A goalkeeper at 189 counts as 185 (tall): the legal team's two are.
/// At 186 he is in no bracket, forbidden, and the talls are one short.
#[test]
fn goalkeepers_at_189_count_as_tall_and_186_is_forbidden() {
    let rules = rules();
    assert_eq!(player(&legal_team(), 0).height, 189);
    assert_eq!(problems(&rules, &edited(0, |p| p.height = 189)), vec![]);
    assert_eq!(
        problems(&rules, &edited(0, |p| p.height = 186)),
        vec![
            // Purple's tall quota (5) one short; at 186 he has no allowance.
            error(Some(0), no_bracket(186, &[(185, 185)], true)),
            error(
                Some(0),
                special(
                    "Tall goalkeepers are 189",
                    "GKs in this bracket must be 189cm"
                )
            ),
            quota(
                "purple",
                "tall",
                4,
                5,
                &[(2, 185), (9, 185), (10, 185), (11, 185)]
            ),
        ]
    );
}

/// Injury resistance is exact (not `at_most`): a gold at 1, below the
/// target 2, is off.
#[test]
fn a_golds_injury_resistance_is_exactly_two() {
    assert_eq!(
        problems(&rules(), &edited(2, |p| p.injury_resistance = 1)),
        vec![error(
            Some(2),
            ViolationKind::InjuryResistance { value: 1, max: 2 }
        )]
    );
}

/// The captain may hold one card over his tier's limit: slot 12, a
/// regular at his three paid, is over it with a fourth; as the captain
/// he is legal (slot 10's Captaincy turned paid with the armband gone,
/// and dropped).
#[test]
fn the_captain_gets_one_extra_card() {
    let rules = rules();
    let over = edited(12, |p| p.cards.push(card("long_range_drive")));
    assert_eq!(
        problems(&rules, &over),
        vec![error(Some(12), ViolationKind::Cards { paid: 4, limit: 3 })]
    );
    let mut team = over;
    team.captain = Some(12);
    let previous = player_mut(&mut team, 10);
    assert!(previous.cards.contains(&card("captaincy")));
    previous.cards.retain(|c| *c != card("captaincy"));
    assert_eq!(problems(&rules, &team), vec![]);
}

/// Acrobatic Finishing and Malicia are free: a regular at his limit
/// (slot 12, three paid) holds both on top of it.
#[test]
fn acrobatic_finishing_and_malicia_are_free() {
    let rules = rules();
    assert_eq!(player(&legal_team(), 12).cards.len(), 3);
    let team = edited(12, |p| {
        p.cards.push(card("acrobatic_finishing"));
        p.cards.push(card("malicia"));
    });
    assert_eq!(problems(&rules, &team), vec![]);
}

/// VGL's skill-card cap is past the game's ten (`max_skill_cards` 41):
/// a regular at his card limit (slot 12, three paid) holding eleven
/// skills — the tricks group covering the nine added — reports nothing.
#[test]
fn eleven_skills_are_legal() {
    let rules = rules();
    let skills = |p: &PlayerView| {
        p.cards
            .iter()
            .filter(|card| matches!(card, Card::Skill(_)))
            .count()
    };
    assert_eq!(skills(player(&legal_team(), 12)), 2);
    let team = edited(12, |p| {
        for name in [
            "scissors_feint",
            "flip_flap",
            "marseille_turn",
            "sombrero",
            "cut_behind_and_turn",
            "scotch_move",
            "rabona",
            "malicia",
            "double_touch",
        ] {
            p.cards.push(card(name));
        }
    });
    assert_eq!(skills(player(&team, 12)), 11);
    assert_eq!(problems(&rules, &team), vec![]);
}
