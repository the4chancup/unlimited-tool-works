//! VGL26's fixtures. The sample team made legal under VGL26: its tiers
//! fix one height each, so every player's height and stats are set to
//! his tier's. Roster slots name the sample team's players: 2 and 10
//! its golds (99), 5, 9 and 20 its silvers (88), 0 and 11 its
//! goalkeepers (189 cm, on VGL26 185 like every buffed and regular GK).
//! VGL26 is played on PES 21: `legal_records_pes21` builds the same
//! records on that version (a Team TOML document is version-neutral),
//! which the fluid-formation fixtures need.

use super::*;
use crate::ruleset::Card;

/// The VGL26 data block, a template ruleset.
const VGL26: &str = include_str!("../../rulesets/vgl26.rhai");

/// VGL26's rules loaded.
fn rules() -> Rules {
    Rules::from_data_block(VGL26).expect("VGL26 loads")
}

/// `records` (the sample team, on either build's version) made legal
/// under VGL26:
///
/// - heights by hand, then each tier's targets through
///   `Rules::apply_tier`: slot 2 gold (195 cm), 10 and 5 silver (195),
///   9 and 20 bronze (190), 1, 6, 8, 14, 15, 17, 18 and 21 buffed (185),
///   0 and 11 goalkeepers (185, counted as regulars), the rest regulars
///   (180);
/// - the sample's preset 3 is a 3-4-3, which VGL26 bans: each of its
///   three formations made a 3-5-2 by one position (the last player's
///   SS read as AMF);
/// - cards and A positions untouched: the sample's already fit (slot
///   10's Captaincy free as the captain's).
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
    assert_eq!(records.team.tactics.auto.substitution, Some(0));
    for formation in &records.team.tactics.presets[2].formations {
        assert_eq!(formation.players[10].position, position("SS").0);
    }
    let rules = rules();
    for (slot, tier, height) in [
        (2, "gold", 195),
        (10, "silver", 195),
        (5, "silver", 195),
        (9, "bronze", 190),
        (20, "bronze", 190),
        (1, "buffed", 185),
        (6, "buffed", 185),
        (8, "buffed", 185),
        (14, "buffed", 185),
        (15, "buffed", 185),
        (17, "buffed", 185),
        (18, "buffed", 185),
        (21, "buffed", 185),
        (0, "goalkeeper", 185),
        (11, "goalkeeper", 185),
        (3, "regular", 180),
        (4, "regular", 180),
        (7, "regular", 180),
        (12, "regular", 180),
        (13, "regular", 180),
        (16, "regular", 180),
        (19, "regular", 180),
        (22, "regular", 180),
    ] {
        records.player_mut(slot).basic.height = height;
        let view = records.view();
        let player = view.player(slot).expect("a filled slot").clone();
        rules
            .apply_tier(&view, &player, tier, records.player_mut(slot))
            .expect("the tier applies");
    }
    for formation in &mut records.team.tactics.presets[2].formations {
        formation.players[10].position = position("AMF").0;
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
    assert_eq!(rules.ruleset().name, "/vg/ League 26");
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
    assert_eq!(
        tiers,
        [
            "gold",
            "silver",
            "bronze",
            "buffed",
            "goalkeeper",
            "regular"
        ]
    );
    for (tier, slot) in [
        ("gold", 2),
        ("silver", 5),
        ("bronze", 9),
        ("buffed", 8),
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

/// A silver's finishing target is 99 (slot 5): met it is legal; at 95,
/// the base rate, it is the one stat violation.
#[test]
fn a_silvers_finishing_is_99() {
    let rules = rules();
    let finishing = usize::from(stat("finishing").0);
    assert_eq!(player(&legal_team(), 5).abilities[finishing], Some(99));
    let team = edited(5, |p| p.abilities[finishing] = Some(95));
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(5),
            ViolationKind::Stat {
                stat: stat("finishing"),
                value: 95,
                target: 99,
                at_most: false
            }
        )]
    );
}

/// A registered goalkeeper at 185 rated 85 (slot 0 given the buffed
/// targets) reads as the goalkeeper tier (buffed's `when` rules out
/// GK), so every stat over or off 72 is a violation of his own, and
/// form 4 sits under the goalkeepers' 8.
#[test]
fn a_goalkeeper_at_185_rated_85_reads_as_goalkeeper() {
    let rules = rules();
    let mut records = legal_records();
    let view = records.view();
    let keeper = view.player(0).expect("slot 0").clone();
    assert_eq!(keeper.position, Some(position("GK")));
    assert_eq!(keeper.height, 185);
    rules
        .apply_tier(&view, &keeper, "buffed", records.player_mut(0))
        .expect("buffed applies");
    let found = problems(&rules, &records.view());
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
                ViolationKind::Stat { target: 72, .. }
                    | ViolationKind::Form {
                        value: 4,
                        target: 8
                    }
            )),
        "{found:?}"
    );
}

/// A registered goalkeeper must be 185 cm (the rules page): slot 0 at
/// 180 is still the goalkeeper tier, and the one violation is the height.
#[test]
fn a_goalkeeper_must_be_185() {
    let rules = rules();
    assert_eq!(player(&legal_team(), 0).height, 185);
    assert_eq!(
        problems(&rules, &edited(0, |p| p.height = 180)),
        vec![error(
            Some(0),
            ViolationKind::Special {
                label: "Goalkeepers are 185".to_string(),
                text: "GKs must be 185cm".to_string()
            }
        )]
    );
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

/// A 3-4-3 in one formation of a fluid preset 3 is the formation ban
/// (PES 21 build); the legal team's preset 3, all three formations now
/// 3-5-2, shows the violation names preset and formation.
#[test]
fn a_3_4_3_in_one_formation_of_a_fluid_preset_is_banned() {
    let rules = rules();
    let (amf, ss) = (position("AMF"), position("SS"));
    let mut team = legal_records_pes21().view();
    team.tactics.presets[2].style.fluid = Some(true);
    // Formation 2's last player back to SS: the shape is 3-4-3 again.
    team.tactics.presets[2].formations[1].players[10].position = ss.0;
    assert_eq!(
        team.tactics.presets[2].formations[0].players[10].position, amf.0,
        "the other formations stay 3-5-2"
    );
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            None,
            ViolationKind::FormationShape {
                preset: 3,
                formation: 2,
                shape: [3, 4, 3]
            }
        )]
    );
}

/// The sample's own preset 3 is a 3-4-3: restored in its kick-off
/// formation, the ban reports it (PES 15 has no fluid presets).
#[test]
fn the_samples_3_4_3_preset_3_is_banned() {
    let rules = rules();
    let ss = position("SS");
    let mut team = legal_team();
    team.tactics.presets[2].formations[0].players[10].position = ss.0;
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            None,
            ViolationKind::FormationShape {
                preset: 3,
                formation: 1,
                shape: [3, 4, 3]
            }
        )]
    );
}

/// The support settings and the man-marking ban are checked, each its
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

/// An entry that wants a setting on names it `wanted: true`: VGL26's man
/// marking entry turned to require man marking, which the legal team
/// (no preset marks anyone) breaks.
#[test]
fn a_team_setting_names_the_value_it_wants() {
    let rules = Rules::from_data_block(&VGL26.replace(
        "#{ label: \"No man marking\", man_marking: false }",
        "#{ label: \"No man marking\", man_marking: true }",
    ))
    .expect("the edited VGL26 loads");
    assert_eq!(
        problems(&rules, &legal_team()),
        vec![error(
            None,
            ViolationKind::TeamSetting {
                label: "No man marking".to_string(),
                setting: "man_marking".to_string(),
                wanted: true
            }
        )]
    );
}

/// A regular (slot 12, 180) at 183 fits no tier's `when`: the hint names
/// the heights the tiers take, each once and in order: the regulars' 180,
/// the buffed 185 (its `not` names no height it takes), the bronzes' 190
/// and the gold's and silvers' 195. The goalkeepers' tier names none.
#[test]
fn no_tier_names_the_heights_the_tiers_take() {
    let rules = rules();
    assert_eq!(player(&legal_team(), 12).height, 180);
    let team = edited(12, |p| p.height = 183);
    assert_eq!(
        problems(&rules, &team)
            .into_iter()
            .filter(|violation| violation.slot == Some(12))
            .collect::<Vec<_>>(),
        vec![error(
            Some(12),
            ViolationKind::NoTier {
                heights: vec![(180, 180), (185, 185), (190, 190), (195, 195)]
            }
        )]
    );
}

/// The captain's Captaincy card and one more are free only through the
/// special: the captain (slot 10, the sample's Captaincy holder) pushed
/// to his card limit is legal — `card_limits` shows the +1 (9 over the
/// silver 8) and Captaincy among the free cards.
#[test]
fn the_captains_special_covers_the_limit_and_the_card() {
    let rules = rules();
    assert_eq!(legal_team().captain, Some(10));
    let team = edited(10, |p| {
        for name in [
            "knuckle_shot",
            "heel_trick",
            "dipping_shots",
            "rising_shots",
            "man_marking",
        ] {
            p.cards.push(card(name));
        }
    });
    assert_eq!(problems(&rules, &team), vec![]);
    let limits = rules
        .card_limits(&team, team.player(10).expect("slot 10"), "silver")
        .expect("limits");
    assert_eq!(limits.limit, 9, "the special's +1 over the silver 8");
    assert!(
        limits.free_cards.contains(&card("captaincy")),
        "Captaincy is free to the captain"
    );
}

/// Heading is a free card for everyone, outside the tricks group: a
/// regular at his card limit (slot 4, three non-free plus three more)
/// holds it on top, where a seventh non-free card is over.
#[test]
fn heading_is_free_at_the_card_limit() {
    let rules = rules();
    assert_eq!(player(&legal_team(), 4).cards.len(), 3);
    let team = edited(4, |p| {
        for name in ["long_range_drive", "knuckle_shot", "heel_trick"] {
            p.cards.push(card(name));
        }
        p.cards.push(card("heading"));
    });
    assert_eq!(problems(&rules, &team), vec![]);
    let team = edited(4, |p| {
        for name in [
            "long_range_drive",
            "knuckle_shot",
            "heel_trick",
            "dipping_shots",
        ] {
            p.cards.push(card(name));
        }
    });
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(4),
            ViolationKind::Cards {
                non_free: 7,
                limit: 6
            }
        )]
    );
}

/// `card_limits` reports each tier's economy on a member no special
/// touches — the card limit, the A positions, the free counts (the
/// exact map, so a group named nowhere in it shows) and
/// `max_a_positions` unset — and `tier_values` the weak foot, the
/// file's 4/4 on every tier. Slots: 2 for the gold, 5 the silver, 9 the
/// bronze, 8 the buffed, 0 the goalkeeper (his one A fitting the one
/// allowance), 12 the regular — every one inside its A allowance.
#[test]
fn card_limits_and_tier_values_report_the_tiers_values() {
    let rules = rules();
    let team = legal_team();
    assert_economy(
        &rules,
        &team,
        &[
            (2, "gold", 9, 2, &[("com", 2), ("tricks", 12)], (4, 4)),
            (5, "silver", 8, 2, &[("com", 2), ("tricks", 12)], (4, 4)),
            (9, "bronze", 8, 2, &[("com", 2), ("tricks", 12)], (4, 4)),
            (8, "buffed", 7, 2, &[("com", 1), ("tricks", 12)], (4, 4)),
            (0, "goalkeeper", 5, 1, &[("com", 1), ("tricks", 12)], (4, 4)),
            (12, "regular", 6, 2, &[("com", 1), ("tricks", 12)], (4, 4)),
        ],
    );
}

/// A medal's injury resistance is exact (the rules page): a silver at 1,
/// under the target 2, is the one violation.
#[test]
fn a_medals_injury_resistance_is_exactly_two() {
    let rules = rules();
    assert_eq!(player(&legal_team(), 5).injury_resistance, 2);
    assert_eq!(
        problems(&rules, &edited(5, |p| p.injury_resistance = 1)),
        vec![error(
            Some(5),
            ViolationKind::InjuryResistance { value: 1, max: 2 }
        )]
    );
}

/// VGL's skill-card cap is past the game's ten (`max_skill_cards` 41):
/// a regular holding eleven skills — the tricks group covering the
/// nine added over his non-free cards — reports nothing.
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
