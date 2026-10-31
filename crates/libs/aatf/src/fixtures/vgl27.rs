//! VGL27's fixtures. The sample team made legal under VGL27: the purple
//! heights are six manlets (175), seven mids (180) and ten talls (185,
//! the 189 cm goalkeepers counted), so the five 194 cm players are moved
//! into them and the goalkeepers stay 189. Roster slots name the sample
//! team's players: 2 and 10 rated 99, 5, 9 and 20 rated 88, 0 and 11
//! its goalkeepers. VGL27 is played on PES 21: `legal_records_pes21`
//! builds the same records on that version (a Team TOML document is
//! version-neutral), which the fluid-formation fixture needs.

use super::*;
use crate::ruleset::Card;

/// The VGL27 data block, a template ruleset.
const VGL27: &str = include_str!("../../rulesets/vgl27.rhai");

/// VGL27's rules loaded.
fn rules() -> Rules {
    Rules::from_data_block(VGL27).expect("VGL27 loads")
}

/// `records` (the sample team, on either build's version) made legal
/// under VGL27:
///
/// - heights by hand, then each tier's targets through
///   `Rules::apply_tier`: slot 2 gold (185 cm), 5 and 10 silver (185),
///   9 and 20 bronze (185), 0 and 11 goalkeepers (189, counted as 185),
///   8 to the mids (180), 14 and 17 to the talls (185), the rest
///   regulars at their own heights (175 or 180);
/// - cards, A positions and formations untouched: the sample's already
///   fit (slot 10's Captaincy free as the captain's, and no formation
///   ban, so the preset-3 3-4-3 stands).
fn legalize(mut records: Records) -> Records {
    // Premises on the sample as written.
    let dribbling = |records: &mut Records, slot: u8| records.player_mut(slot).stats.dribbling;
    assert_eq!(dribbling(&mut records, 2), 99);
    assert_eq!(dribbling(&mut records, 10), 99);
    assert_eq!(dribbling(&mut records, 5), 88);
    assert_eq!(dribbling(&mut records, 9), 88);
    assert_eq!(dribbling(&mut records, 20), 88);
    assert_eq!(dribbling(&mut records, 0), 74);
    assert_eq!(dribbling(&mut records, 11), 74);
    assert_eq!(records.team.tactics.set_pieces.captain, 10);
    assert_eq!(records.team.tactics.auto.preset_change, Some(false));
    for slot in [5, 8, 14, 17, 20] {
        assert_eq!(records.player_mut(slot).basic.height, 194, "{slot}");
    }
    assert_eq!(records.player_mut(0).basic.height, 189);
    assert_eq!(records.player_mut(11).basic.height, 189);
    let rules = rules();
    for (slot, tier, height) in [
        (2, "gold", 185),
        (5, "silver", 185),
        (10, "silver", 185),
        (9, "bronze", 185),
        (20, "bronze", 185),
        (0, "goalkeeper", 189),
        (11, "goalkeeper", 189),
    ] {
        records.player_mut(slot).basic.height = height;
        let view = records.view();
        let player = view.player(slot).expect("a filled slot").clone();
        rules
            .apply_tier(&view, &player, tier, records.player_mut(slot))
            .expect("the tier applies");
    }
    for (slot, height) in [
        (1, 180),
        (3, 175),
        (4, 175),
        (6, 175),
        (7, 180),
        (8, 180),
        (12, 180),
        (13, 180),
        (14, 185),
        (15, 175),
        (16, 185),
        (17, 185),
        (18, 180),
        (19, 175),
        (21, 180),
        (22, 175),
    ] {
        records.player_mut(slot).basic.height = height;
        let view = records.view();
        let player = view.player(slot).expect("a filled slot").clone();
        rules
            .apply_tier(&view, &player, "regular", records.player_mut(slot))
            .expect("the tier applies");
    }
    records
}

/// `legalize` on the sample team's own version (PES 15).
fn legal_records() -> Records {
    legalize(Records::sample())
}

/// `legalize` on the PES 21 build.
fn legal_records_pes21() -> Records {
    legalize(Records::sample_on(PesVersion::Pes21))
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
    assert_eq!(rules.ruleset().name, "/vg/ League 27");
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
    assert_eq!(tiers, ["gold", "silver", "bronze", "goalkeeper", "regular"]);
    for (tier, slot) in [
        ("gold", 2),
        ("silver", 5),
        ("bronze", 9),
        ("goalkeeper", 0),
        ("regular", 12),
    ] {
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

/// A rating between two tiers' targets is placed by expected rating:
/// a silver (slot 5) at 96, between silver's 94 and gold's 99, stays a
/// silver and every stat over 94 is a violation. On PES 15 that is the 19
/// abilities, the `at_most` prowesses included (96 > 94).
#[test]
fn a_rating_between_tiers_reads_as_the_lower() {
    let rules = rules();
    let team = edited(5, |p| set_tier(p, 96, 8, 2));
    let found = problems(&rules, &team);
    assert_eq!(found.len(), 19, "{found:?}");
    let mut names: Vec<&str> = found
        .iter()
        .filter_map(|v| match &v.kind {
            ViolationKind::Stat { stat, .. } => Some(stat.label()),
            _ => None,
        })
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "attacking_prowess",
            "ball_control",
            "ball_winning",
            "body_control",
            "catching",
            "defensive_prowess",
            "dribbling",
            "explosive_power",
            "finishing",
            "goalkeeping",
            "heading",
            "jump",
            "kicking_power",
            "lofted_pass",
            "low_pass",
            "place_kicking",
            "speed",
            "stamina",
            "swerve",
        ]
    );
    assert!(
        found.iter().all(|v| v.slot == Some(5)
            && matches!(
                &v.kind,
                ViolationKind::Stat {
                    value: 96,
                    target: 94,
                    ..
                }
            )),
        "{found:?}"
    );
}

/// A registered goalkeeper rated 99 (slot 0 given the gold targets)
/// reads as the goalkeeper tier — the medal tiers' `when` rules out GK —
/// so every stat over 77 is a violation of his own, form 8 sits over the
/// goalkeepers' 4, injury resistance 2 over their 1 and weak foot 4/4
/// over their 2/2: 23 violations.
#[test]
fn a_goalkeeper_rated_99_is_a_goalkeeper() {
    let rules = rules();
    let mut records = legal_records();
    let view = records.view();
    let keeper = view.player(0).expect("slot 0").clone();
    assert_eq!(keeper.position, Some(position("GK")));
    assert_eq!(keeper.height, 189);
    rules
        .apply_tier(&view, &keeper, "gold", records.player_mut(0))
        .expect("gold applies");
    let found = problems(&rules, &records.view());
    assert_eq!(found.len(), 23, "{found:?}");
    let mut names: Vec<&str> = found
        .iter()
        .filter_map(|v| match &v.kind {
            ViolationKind::Stat { stat, .. } => Some(stat.label()),
            _ => None,
        })
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "attacking_prowess",
            "ball_control",
            "ball_winning",
            "body_control",
            "catching",
            "defensive_prowess",
            "dribbling",
            "explosive_power",
            "finishing",
            "goalkeeping",
            "heading",
            "jump",
            "kicking_power",
            "lofted_pass",
            "low_pass",
            "place_kicking",
            "speed",
            "stamina",
            "swerve",
        ]
    );
    assert!(
        found.iter().all(|v| v.slot == Some(0)
            && matches!(
                &v.kind,
                ViolationKind::Stat {
                    value: 99,
                    target: 77,
                    ..
                } | ViolationKind::Form {
                    value: 8,
                    target: 4
                } | ViolationKind::InjuryResistance { value: 2, max: 1 }
                    | ViolationKind::WeakFootUsage { value: 4, max: 2 }
                    | ViolationKind::WeakFootAccuracy { value: 4, max: 2 }
            )),
        "{found:?}"
    );
}

/// A goalkeeper at 189 counts as 185 (the `height_allowance` special):
/// the legal team's two are. An outfielder at 189 is in no bracket —
/// the allowance is the goalkeepers' alone.
#[test]
fn a_goalkeeper_at_189_counts_as_tall() {
    let rules = rules();
    assert_eq!(player(&legal_team(), 0).height, 189);
    assert_eq!(problems(&rules, &edited(0, |p| p.height = 189)), vec![]);
    assert_eq!(
        problems(&rules, &edited(12, |p| p.height = 189)),
        vec![
            // Purple's mid quota (7) one short.
            error(Some(12), no_bracket(189, &[(180, 180)], true)),
            quota(
                "purple",
                "mid",
                6,
                7,
                &[
                    (1, 180),
                    (7, 180),
                    (8, 180),
                    (13, 180),
                    (18, 180),
                    (21, 180)
                ]
            ),
        ]
    );
}

/// The page's tall bracket takes outfielders at 185 and goalkeepers at
/// 189: a goalkeeper at 185 (slot 0) would count as tall, and the
/// special forbids it. The other brackets name no position, so a
/// goalkeeper at 180 is a mid and only the quotas move.
#[test]
fn a_tall_goalkeeper_must_be_189() {
    let rules = rules();
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
    assert_eq!(
        problems(&rules, &edited(0, |p| p.height = 180)),
        vec![
            quota(
                "purple",
                "mid",
                8,
                7,
                &[
                    (0, 180),
                    (1, 180),
                    (7, 180),
                    (8, 180),
                    (12, 180),
                    (13, 180),
                    (18, 180),
                    (21, 180)
                ]
            ),
            quota(
                "purple",
                "tall",
                9,
                10,
                &[
                    (2, 185),
                    (5, 185),
                    (9, 185),
                    (10, 185),
                    (11, 185),
                    (14, 185),
                    (16, 185),
                    (17, 185),
                    (20, 185)
                ]
            )
        ]
    );
}

/// A height no bracket takes is the player's own violation, and his
/// bracket's quota comes up short: a mid (slot 12) at 183.
#[test]
fn a_height_no_bracket_takes_is_illegal() {
    let rules = rules();
    assert_eq!(player(&legal_team(), 12).height, 180);
    assert_eq!(
        problems(&rules, &edited(12, |p| p.height = 183)),
        vec![
            error(Some(12), no_bracket(183, &[(180, 180)], true)),
            quota(
                "purple",
                "mid",
                6,
                7,
                &[
                    (1, 180),
                    (7, 180),
                    (8, 180),
                    (13, 180),
                    (18, 180),
                    (21, 180)
                ]
            ),
        ]
    );
}

/// A mid moved to tall (slot 8 at 185) puts the talls one over their
/// quota and the mids one under.
#[test]
fn a_bracket_over_its_quota() {
    let rules = rules();
    assert_eq!(
        problems(&rules, &edited(8, |p| p.height = 185)),
        vec![
            quota(
                "purple",
                "mid",
                6,
                7,
                &[
                    (1, 180),
                    (7, 180),
                    (12, 180),
                    (13, 180),
                    (18, 180),
                    (21, 180)
                ]
            ),
            quota(
                "purple",
                "tall",
                11,
                10,
                &[
                    (0, 185),
                    (2, 185),
                    (5, 185),
                    (8, 185),
                    (9, 185),
                    (10, 185),
                    (11, 185),
                    (14, 185),
                    (16, 185),
                    (17, 185),
                    (20, 185)
                ]
            )
        ]
    );
}

/// The Manlets special: a manlet regular (slot 3) may hold one card and
/// one A position over the tier's — five paid and three A positions are
/// legal, where a mid regular's four and two would not be.
#[test]
fn a_manlet_gets_one_more_card_and_one_more_a_position() {
    let rules = rules();
    let base = legal_team();
    assert_eq!(player(&base, 3).height, 175);
    assert_eq!(player(&base, 3).cards.len(), 3);
    let team = edited(3, |p| {
        p.cards.push(card("fighting_spirit"));
        p.cards.push(card("dipping_shots"));
        p.ratings[usize::from(position("CB").0)] = 2;
        p.ratings[usize::from(position("DMF").0)] = 2;
    });
    assert_eq!(problems(&rules, &team), vec![]);
}

/// A starting-eleven player may be fielded only at his A positions in
/// preset 1: slot 9 (registered SS, his only A) moved to AMF in preset
/// 1's formation is the violation; the same edit in preset 2 is legal.
#[test]
fn a_starter_is_fielded_only_at_his_a_positions_in_preset_1() {
    let rules = rules();
    let (ss, amf) = (position("SS"), position("AMF"));
    let team = legal_team();
    // Premise: slot 9 is the starter at formation index 8, fielded at SS
    // in presets 1 and 2's kick-off formations.
    let index = team
        .tactics
        .starting_eleven
        .iter()
        .position(|slot| *slot == 9)
        .expect("slot 9 starts");
    assert_eq!(player(&team, 9).position, Some(ss));
    assert_eq!(
        team.tactics.presets[0].formations[0].players[index].position,
        ss.0
    );
    assert_eq!(
        team.tactics.presets[1].formations[0].players[index].position,
        ss.0
    );
    let mut team = team;
    team.tactics.presets[0].formations[0].players[index].position = amf.0;
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(9),
            ViolationKind::FieldedNotA {
                preset: 1,
                position: amf,
                rated_a: vec![ss]
            }
        )]
    );
    let mut team = legal_team();
    team.tactics.presets[1].formations[0].players[index].position = amf.0;
    assert_eq!(problems(&rules, &team), vec![]);
}

/// On the PES 21 build a fluid preset fields all three of its
/// formations under the A-position rule: preset 1's out-of-possession
/// formation (the third) alone edited to put slot 9 (an SS, rated A
/// there alone) at AMF is the one violation.
#[test]
fn a_fluid_preset_fields_every_formation_under_the_a_rule() {
    let rules = rules();
    let (ss, amf) = (position("SS"), position("AMF"));
    let mut team = legal_records_pes21().view();
    assert_eq!(team.version, PesVersion::Pes21);
    // Premise: preset 1 is not fluid, so it fields its kick-off
    // formation alone; its third formation fields the starter at
    // index 8 (slot 9) at SS.
    assert!(
        team.formation(1, 2).is_none(),
        "preset 1 fields one formation"
    );
    assert_eq!(team.tactics.starting_eleven[8], 9);
    assert_eq!(
        team.tactics.presets[0].formations[2].players[8].position,
        ss.0
    );
    assert_eq!(player(&team, 9).position, Some(ss));
    assert_eq!(player(&team, 9).ratings[usize::from(amf.0)], 0);
    team.tactics.presets[0].style.fluid = Some(true);
    team.tactics.presets[0].formations[2].players[8].position = amf.0;
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(9),
            ViolationKind::FieldedNotA {
                preset: 1,
                position: amf,
                rated_a: vec![ss]
            }
        )]
    );
}

/// Injury resistance is exact (the rules page, stricter than the
/// checker's `>` test): a regular (slot 12) at 0, below the target 1,
/// is off.
#[test]
fn a_non_medals_injury_resistance_is_exactly_one() {
    assert_eq!(
        problems(&rules(), &edited(12, |p| p.injury_resistance = 0)),
        vec![error(
            Some(12),
            ViolationKind::InjuryResistance { value: 0, max: 1 }
        )]
    );
}

/// Heading is a paid card in VGL27: a regular at his card limit (slot
/// 12, three paid plus one more) holding it on top is over by one —
/// were it free, the paid count would stay at the limit.
#[test]
fn heading_is_a_paid_card() {
    let rules = rules();
    assert_eq!(player(&legal_team(), 12).cards.len(), 3);
    let team = edited(12, |p| p.cards.push(card("fighting_spirit")));
    assert_eq!(problems(&rules, &team), vec![]);
    let team = edited(12, |p| {
        p.cards.push(card("fighting_spirit"));
        p.cards.push(card("heading"));
    });
    assert_eq!(
        problems(&rules, &team),
        vec![error(Some(12), ViolationKind::Cards { paid: 5, limit: 4 })]
    );
}

/// `card_limits` reports each tier's economy on a member no special
/// touches — the paid limit, the A positions, the free counts (the
/// exact map, so a group named nowhere in it shows) and
/// `max_a_positions` unset — and `tier_values` the weak foot (4/4 for
/// the medals, 2/2 for the rest). Slots: 2 for the gold, 5 the silver,
/// 9 the bronze, 0 the goalkeeper (his one A fitting the one
/// allowance), 12 the regular, a mid — every one inside its A
/// allowance.
#[test]
fn card_limits_and_tier_values_report_the_tiers_values() {
    let rules = rules();
    let team = legal_team();
    assert_economy(
        &rules,
        &team,
        &[
            (2, "gold", 6, 2, &[("com", 1), ("tricks", 12)], (4, 4)),
            (5, "silver", 5, 2, &[("com", 1), ("tricks", 12)], (4, 4)),
            (9, "bronze", 5, 2, &[("com", 1), ("tricks", 12)], (4, 4)),
            (0, "goalkeeper", 4, 1, &[("tricks", 12)], (2, 2)),
            (12, "regular", 4, 2, &[("tricks", 12)], (2, 2)),
        ],
    );
}

/// The captain's choice: slot 10 (a silver) at one card over the limit
/// takes the free-card option and is legal; at two over, no single
/// option covers it and the +1 shows in the limit.
#[test]
fn the_captain_gets_one_extra_of_card_com_or_a_position() {
    let rules = rules();
    assert_eq!(legal_team().captain, Some(10));
    let team = edited(10, |p| p.cards.push(card("heading")));
    assert_eq!(problems(&rules, &team), vec![]);
    let team = edited(10, |p| {
        p.cards.push(card("heading"));
        p.cards.push(card("fighting_spirit"));
    });
    assert_eq!(
        problems(&rules, &team),
        vec![error(Some(10), ViolationKind::Cards { paid: 7, limit: 6 })]
    );
}

/// The `free: #{ com: 1 }` option is the one the captain uses where a
/// second COM style is held: his second style is past the one the tier
/// frees, and the option's freeing shows in `card_limits` while the
/// paid limit stays the tier's (the `cards` option untouched).
#[test]
fn the_captains_com_option_frees_his_second_com() {
    let rules = rules();
    let team = legal_team();
    let limits = rules
        .card_limits(&team, team.player(10).expect("slot 10"), "silver")
        .expect("limits");
    assert_eq!(
        limits.free.get("com"),
        Some(&2),
        "the option's +1 free COM style"
    );
    assert_eq!(limits.limit, 5, "the tier's limit, not `cards`");
    assert_eq!(problems(&rules, &team), vec![]);
}

/// The `a_positions` option decides only once the earlier options do
/// not: a captain with one COM style (the `free: com` option inert) and
/// a third A position — over the silver allowance — has the option
/// raise it to three, the paid limit again the tier's.
#[test]
fn the_captains_a_positions_option_raises_his_allowance() {
    let rules = rules();
    let (cb, dmf) = (position("CB"), position("DMF"));
    let team = edited(10, |p| {
        assert_eq!(
            p.cards
                .iter()
                .filter(|c| matches!(c, Card::ComStyle(_)))
                .count(),
            2
        );
        assert_eq!(p.ratings.iter().filter(|r| **r == 2).count(), 1);
        p.cards.retain(|c| *c != card("long_ranger"));
        p.ratings[usize::from(cb.0)] = 2;
        p.ratings[usize::from(dmf.0)] = 2;
    });
    let limits = rules
        .card_limits(&team, team.player(10).expect("slot 10"), "silver")
        .expect("limits");
    assert_eq!(limits.a_positions, 3, "the option's +1 over the silver 2");
    assert_eq!(limits.limit, 5, "the tier's limit, not `cards`");
    assert_eq!(problems(&rules, &team), vec![]);
}

/// The man-marking ban and the support settings are checked, each its
/// own violation (the settings are stored on every version; the PES 15
/// build shows them).
#[test]
fn man_marking_and_the_support_settings_are_checked() {
    let rules = rules();
    let mut team = legal_team();
    team.tactics.presets[0].man_marking[4] = 3;
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            None,
            ViolationKind::TeamSetting {
                label: "No man marking".to_string(),
                setting: "man_marking".to_string(),
                wanted: false
            }
        )]
    );
    let mut team = legal_team();
    team.tactics.auto.substitution = Some(2);
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            None,
            ViolationKind::TeamSetting {
                label: "Support settings".to_string(),
                setting: "auto_substitution".to_string(),
                wanted: false
            }
        )]
    );
    let mut team = legal_team();
    team.tactics.auto.preset_change = Some(true);
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            None,
            ViolationKind::TeamSetting {
                label: "Support settings".to_string(),
                setting: "auto_preset_change".to_string(),
                wanted: false
            }
        )]
    );
    let mut team = legal_team();
    team.tactics.auto.offside_trap = Some(true);
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            None,
            ViolationKind::TeamSetting {
                label: "Support settings".to_string(),
                setting: "auto_offside_trap".to_string(),
                wanted: false
            }
        )]
    );
}

/// VGL27 has no formation ban: the sample's preset 3 is a 3-4-3 and
/// stands (VGL26 forbade it; its fixtures moved it to a 3-5-2).
#[test]
fn a_3_4_3_formation_is_legal() {
    let rules = rules();
    let team = legal_team();
    assert!(
        team.formation_shapes()
            .iter()
            .any(|f| f.preset == 3 && f.formation == 1 && f.shape == [3, 4, 3]),
        "preset 3 fields a 3-4-3"
    );
    assert_eq!(problems(&rules, &team), vec![]);
}

/// VGL's skill-card cap is past the game's ten (`max_skill_cards` 41):
/// a regular holding eleven skills — the tricks group covering the
/// nine added over his paid cards — reports nothing.
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
            "crossover_turn",
            "cut_behind_and_turn",
            "scotch_move",
            "step_on_skill",
            "rabona",
        ] {
            p.cards.push(card(name));
        }
    });
    assert_eq!(skills(player(&team, 12)), 11);
    assert_eq!(problems(&rules, &team), vec![]);
}
