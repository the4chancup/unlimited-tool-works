//! VTL10's fixtures. The sample team made legal under VTL10: the tiers
//! read off heights (medals at 194 told apart by swerve, the buffed at
//! 188, outfield regulars at 178 and their goalkeepers at 180): slots 9
//! and 10 gold, 2, 5 and 20 silver, 3, 4, 6, 7, 12, 15, 16 and 22 buffed
//! with 15 and 22 re-registered LB to RB so the 188 cm players'
//! registered positions are RB and CMF alone, which no 178 cm player
//! shares. Every player holds Malicia (required) and three non-free skill
//! cards (`min_cards`); the buffed at RB — rated A there — carry the
//! 2/2 weak foot their special debuffs them to.

use super::*;
use crate::ruleset::Card;

/// The VTL10 data block, a template ruleset with custom checks.
const VTL10: &str = include_str!("../../rulesets/vtl10.rhai");

/// VTL10's rules loaded.
fn rules() -> Rules {
    Rules::from_data_block(VTL10).expect("VTL10 loads")
}

/// `records` (the sample team, on either build's version) made legal
/// under VTL10:
///
/// - heights by hand, then each tier's targets through
///   `Rules::apply_tier` (which also writes the RB-registered buffed
///   players' 2/2 weak foot): slots 9 and 10 gold (194 cm, swerve 99),
///   2, 5 and 20 silver (194, swerve 92), 3, 4, 6, 7, 12, 15, 16 and 22
///   buffed (188), the rest regulars at 178 — the goalkeepers 0 and 11
///   at 180 (form 8 by the goalkeeper special);
/// - slots 15 and 22 re-registered LB to RB — the A moving with the
///   registration — so the 188 cm players' registered positions are RB
///   and CMF alone;
/// - Malicia (required) and three non-free skill cards — fighting_spirit,
///   interception, track_back — to every player: the held cards already
///   fit (the free skills, the pass skills free on non-medals, the COM
///   styles inside the free counts, Incisive Run, Long Ranger and the
///   captain's Captaincy free by name), and the additions put everyone
///   at or over `min_cards` (the golds at their card limit of six).
fn legalize(mut records: Records) -> Records {
    // Premises on the sample as written.
    assert_eq!(records.team.tactics.set_pieces.captain, 10);
    assert_eq!(records.player_mut(0).positions.registered, position("GK").0);
    assert_eq!(
        records.player_mut(11).positions.registered,
        position("GK").0
    );
    for slot in [15, 22] {
        assert_eq!(
            records.player_mut(slot).positions.registered,
            position("LB").0
        );
    }
    for slot in [15, 22] {
        let entry = records.player_mut(slot);
        entry.positions.registered = position("RB").0;
        entry.positions.playable[playable_at("LB")] = 0;
        entry.positions.playable[playable_at("RB")] = 2;
    }
    let rules = rules();
    for (slot, tier, height) in [
        (9, "gold", 194),
        (10, "gold", 194),
        (2, "silver", 194),
        (5, "silver", 194),
        (20, "silver", 194),
        (3, "buffed", 188),
        (4, "buffed", 188),
        (6, "buffed", 188),
        (7, "buffed", 188),
        (12, "buffed", 188),
        (15, "buffed", 188),
        (16, "buffed", 188),
        (22, "buffed", 188),
    ] {
        records.player_mut(slot).basic.height = height;
        let view = records.view();
        let player = view.player(slot).expect("a filled slot").clone();
        rules
            .apply_tier(
                &reading(&rules, &view),
                &player,
                tier,
                records.player_mut(slot),
            )
            .expect("the tier applies");
    }
    for (slot, height) in [
        (0, 180),
        (11, 180),
        (1, 178),
        (8, 178),
        (13, 178),
        (14, 178),
        (17, 178),
        (18, 178),
        (19, 178),
        (21, 178),
    ] {
        records.player_mut(slot).basic.height = height;
        let view = records.view();
        let player = view.player(slot).expect("a filled slot").clone();
        rules
            .apply_tier(
                &reading(&rules, &view),
                &player,
                "regular",
                records.player_mut(slot),
            )
            .expect("the tier applies");
    }
    for slot in 0..23u8 {
        let entry = records.player_mut(slot);
        for name in ["malicia", "fighting_spirit", "interception", "track_back"] {
            let Card::Skill(index) = card(name) else {
                unreachable!("a skill name")
            };
            entry.skills.skills[usize::from(index)] = true;
        }
    }
    records
}

/// `legalize` on the sample team's own version (PES 15).
fn legal_records() -> Records {
    legalize(Records::sample())
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
    assert_eq!(rules.ruleset().name, "/vt/ League 10");
    assert_eq!(problems(&rules, &legal_team()), vec![]);
}

/// For every tier of the file, a player of that tier in the legal team —
/// stats, form, injury resistance and weak foot broken — is legal again
/// after `apply_tier` (the RB-registered buffed slot's 2/2 weak foot
/// written included).
#[test]
fn apply_tier_then_check_reports_nothing_for_every_tier() {
    let rules = rules();
    let tiers: Vec<&str> = rules
        .ruleset()
        .tiers
        .iter()
        .map(|tier| tier.id.as_str())
        .collect();
    assert_eq!(tiers, ["gold", "silver", "regular", "buffed"]);
    for (tier, slot) in [("gold", 9), ("silver", 5), ("regular", 14), ("buffed", 3)] {
        let mut records = legal_records();
        let entry = records.player_mut(slot);
        set_tier_record(entry, 55, 2, 3);
        entry.stats.weak_foot_usage = 0;
        entry.stats.weak_foot_accuracy = 0;
        let broken = records.view();
        assert!(!problems(&rules, &broken).is_empty(), "{tier} {slot}");
        let player = broken.player(slot).expect("filled").clone();
        rules
            .apply_tier(
                &reading(&rules, &broken),
                &player,
                tier,
                records.player_mut(slot),
            )
            .expect("the tier applies");
        assert_eq!(problems(&rules, &records.view()), vec![], "{tier} {slot}");
    }
}

/// The 194 cm medals are told apart by swerve alone: a silver (slot 5,
/// swerve 92) at 91 fits no tier's `when`, and at 99 reads as gold —
/// every stat the gold's targets ask is a violation (PES 15 skips the
/// stats it lacks), the counts moved.
#[test]
fn silver_and_gold_at_194_differ_only_in_swerve() {
    let rules = rules();
    let swerve = usize::from(stat("swerve").0);
    assert_eq!(player(&legal_team(), 5).abilities[swerve], Some(92));
    assert_eq!(player(&legal_team(), 9).abilities[swerve], Some(99));
    let team = edited(5, |p| p.abilities[swerve] = Some(91));
    assert_eq!(
        problems(&rules, &team),
        vec![
            // 194 is the medals' own height: the cause is the swerve.
            error(Some(5), ViolationKind::NoTier { heights: vec![] }),
            error(
                None,
                ViolationKind::TierCount {
                    tier: "silver".to_string(),
                    count: 2,
                    expected: 3
                }
            )
        ]
    );
    let found = problems(&rules, &edited(5, |p| p.abilities[swerve] = Some(99)));
    assert_eq!(found.len(), 20, "{found:?}");
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
        ]
    );
    assert!(
        found.contains(&error(
            None,
            ViolationKind::TierCount {
                tier: "gold".to_string(),
                count: 3,
                expected: 2
            }
        )),
        "{found:?}"
    );
    assert!(
        found.contains(&error(
            None,
            ViolationKind::TierCount {
                tier: "silver".to_string(),
                count: 2,
                expected: 3
            }
        )),
        "{found:?}"
    );
}

/// The buffed weak-foot debuff: a 188 cm player rated A at LB or RB is
/// 2/2, the rest 4/4. Slot 3, registered RB, holds 2/2 — at 4/4 both
/// halves miss; slot 7, registered CMF, holds 4/4 — at 2/2 both miss.
#[test]
fn a_buffed_wingback_is_2_2_weak_foot() {
    let rules = rules();
    let legal = legal_team();
    let weak_foot = |slot: u8| {
        let p = player(&legal, slot);
        (p.weak_foot_usage, p.weak_foot_accuracy)
    };
    assert_eq!(weak_foot(3), (2, 2));
    assert_eq!(weak_foot(7), (4, 4));
    let at = |slot: u8, value: u8| {
        edited(slot, |p| {
            p.weak_foot_usage = value;
            p.weak_foot_accuracy = value;
        })
    };
    assert_eq!(
        problems(&rules, &at(3, 4)),
        vec![
            error(Some(3), ViolationKind::WeakFootUsage { value: 4, max: 2 }),
            error(
                Some(3),
                ViolationKind::WeakFootAccuracy { value: 4, max: 2 }
            )
        ]
    );
    assert_eq!(
        problems(&rules, &at(7, 2)),
        vec![
            error(Some(7), ViolationKind::WeakFootUsage { value: 2, max: 4 }),
            error(
                Some(7),
                ViolationKind::WeakFootAccuracy { value: 2, max: 4 }
            )
        ]
    );
}

/// The regulars' two heights: an outfielder (slot 14, registered CB) at
/// 180 and a goalkeeper (slot 0) at 178 are each their special's
/// violation — under VTL11, where every regular is 180, only the first
/// is even possible.
#[test]
fn regulars_are_178_outfield_and_180_in_goal() {
    let rules = rules();
    assert_eq!(player(&legal_team(), 14).height, 178);
    assert_eq!(player(&legal_team(), 0).height, 180);
    assert_eq!(
        problems(&rules, &edited(14, |p| p.height = 180)),
        vec![error(
            Some(14),
            special(
                "Outfield regulars are 178cm",
                "Outfield regulars must be 178cm"
            )
        )]
    );
    assert_eq!(
        problems(&rules, &edited(0, |p| p.height = 178)),
        vec![error(
            Some(0),
            special(
                "Regular goalkeepers are 180cm",
                "Regular goalkeepers must be 180cm"
            )
        )]
    );
}

/// Malicia is required on everyone: a regular (slot 14) without it gets
/// the `RequiredCard` violation (the card is free, so his non-free count is
/// unchanged).
#[test]
fn malicia_is_required() {
    let rules = rules();
    assert!(player(&legal_team(), 14).cards.contains(&card("malicia")));
    assert_eq!(
        problems(
            &rules,
            &edited(14, |p| p.cards.retain(|c| *c != card("malicia")))
        ),
        vec![error(
            Some(14),
            ViolationKind::RequiredCard {
                card: card("malicia")
            }
        )]
    );
}

/// `min_cards` asks three non-free skill cards, each extra A position
/// counting as one: a regular (slot 1, two A positions of two allowed)
/// down to two non-free skills gets the violation, and is legal again with a
/// third A position (which also costs him a card slot).
#[test]
fn three_non_free_skill_cards_or_the_extra_a_positions() {
    let rules = rules();
    assert_eq!(
        player(&legal_team(), 1)
            .ratings
            .iter()
            .filter(|r| **r == 2)
            .count(),
        2
    );
    let team = edited(1, |p| p.cards.retain(|c| *c != card("track_back")));
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(1),
            ViolationKind::MinCards {
                non_free: 2,
                min: 3
            }
        )]
    );
    let team = edited(1, |p| {
        p.cards.retain(|c| *c != card("track_back"));
        p.ratings[usize::from(position("DMF").0)] = 2;
    });
    assert_eq!(problems(&rules, &team), vec![]);
}

/// VTL10 allows no B rating anywhere — VTL11 let a 188 cm player hold
/// one at CB: a buffed (slot 3) rated B there, and a regular (slot 14)
/// rated B anywhere, are each the `BRating` violation.
#[test]
fn no_b_rating_anywhere() {
    let rules = rules();
    let cb = position("CB");
    assert_eq!(
        problems(&rules, &edited(3, |p| p.ratings[usize::from(cb.0)] = 1)),
        vec![error(Some(3), ViolationKind::BRating { rated_b: vec![cb] })]
    );
    let dmf = position("DMF");
    assert_eq!(
        problems(&rules, &edited(14, |p| p.ratings[usize::from(dmf.0)] = 1)),
        vec![error(
            Some(14),
            ViolationKind::BRating { rated_b: vec![dmf] }
        )]
    );
}

/// A 188 cm player may not hold an A at CB or GK: a buffed (slot 3)
/// rated A at CB is the special's violation (as VTL11's twin fixture
/// shows, the card limit survives it).
#[test]
fn a_188_holds_no_a_at_cb() {
    let rules = rules();
    let cb = position("CB");
    assert_eq!(
        problems(&rules, &edited(3, |p| p.ratings[usize::from(cb.0)] = 2)),
        vec![error(
            Some(3),
            special(
                "188cm players have no A at CB or GK",
                "188cm players may not have an A-position at CB or GK"
            )
        )]
    );
}

/// VTL10 bans no card — VTL11's GK Penalty Specialist ban is one of its
/// own: a regular (slot 14) holding it is legal (a non-free card, his non-free
/// count rising to five inside the limit).
#[test]
fn the_gk_penalty_specialist_is_legal() {
    let rules = rules();
    let team = edited(14, |p| p.cards.push(card("gk_penalty_specialist")));
    assert_eq!(problems(&rules, &team), vec![]);
}

/// The custom checks: the 188 cm players' registered positions number at
/// most two — a third (slot 3 moved RB to AMF, his weak foot back at the
/// wingback-free 4/4) is the team's violation; a 178 cm player sharing
/// one (slot 19 moved to RB) is his own; and a third goalkeeper (the
/// same slot made a GK at 180, form 8 as the special asks) is the
/// team's again.
#[test]
fn the_custom_checks() {
    let rules = rules();
    let amf = position("AMF");
    let team = edited(3, |p| {
        p.position = Some(amf);
        p.ratings[usize::from(position("RB").0)] = 0;
        p.ratings[usize::from(amf.0)] = 2;
        p.weak_foot_usage = 4;
        p.weak_foot_accuracy = 4;
    });
    assert_eq!(
        problems(&rules, &team),
        vec![custom(
            None,
            "Too many registered positions with 188cm players. Maximum is 2"
        )]
    );
    let rb = position("RB");
    let team = edited(19, |p| {
        p.position = Some(rb);
        p.ratings[usize::from(position("LB").0)] = 0;
        p.ratings[usize::from(rb.0)] = 2;
    });
    assert_eq!(
        problems(&rules, &team),
        vec![custom(
            Some(19),
            "A 178cm player shares a 188cm player's registered position"
        )]
    );
    let gk = position("GK");
    let team = edited(19, |p| {
        p.position = Some(gk);
        p.ratings[usize::from(position("LB").0)] = 0;
        p.ratings[usize::from(gk.0)] = 2;
        p.height = 180;
        p.form = 8;
    });
    assert_eq!(
        problems(&rules, &team),
        vec![custom(None, "More than two goalkeepers")]
    );
}

/// Regulars — the buffed included — cannot be captain: the armband on a
/// regular (slot 14) is the special's violation (the former captain's
/// Captaincy card is non-free once it is no longer his, so it goes too).
#[test]
fn a_regular_cannot_be_captain() {
    let rules = rules();
    let mut team = legal_team();
    team.captain = Some(14);
    player_mut(&mut team, 10)
        .cards
        .retain(|c| *c != card("captaincy"));
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(14),
            special("Regulars cannot be captain", "Regulars can't be captain")
        )]
    );
}

/// Medals cannot play as GK: the gold (slot 9) made a goalkeeper — the A
/// moved with him — is both the special's violation and the custom
/// checks' third goalkeeper.
#[test]
fn a_medal_cannot_play_as_goalkeeper() {
    let rules = rules();
    let (gk, ss) = (position("GK"), position("SS"));
    let team = edited(9, |p| {
        p.position = Some(gk);
        p.ratings[usize::from(ss.0)] = 0;
        p.ratings[usize::from(gk.0)] = 2;
    });
    assert_eq!(
        problems(&rules, &team),
        vec![
            error(
                Some(9),
                special("Medals cannot be goalkeepers", "Medals cannot play as GK")
            ),
            custom(None, "More than two goalkeepers")
        ]
    );
}

/// Two goalkeepers are the floor too, not only the cap the custom check
/// reads: one made an outfielder (slot 11, registered CB and form back
/// at the regulars' 4) is both the outfield height special's violation
/// and the team's `Goalkeepers`.
#[test]
fn one_goalkeeper_is_under_the_floor() {
    let rules = rules();
    let (gk, cb) = (position("GK"), position("CB"));
    let team = edited(11, |p| {
        p.position = Some(cb);
        p.ratings[usize::from(gk.0)] = 0;
        p.ratings[usize::from(cb.0)] = 2;
        p.form = 4;
    });
    assert_eq!(
        problems(&rules, &team),
        vec![
            error(
                Some(11),
                special(
                    "Outfield regulars are 178cm",
                    "Outfield regulars must be 178cm"
                )
            ),
            error(None, ViolationKind::Goalkeepers { count: 1, min: 2 })
        ]
    );
}

/// VTL10 checks no name colour — VTL11's medal colour codes are its own:
/// a regular (slot 14) wearing the gold code is legal.
#[test]
fn the_name_colours_are_not_checked() {
    let rules = rules();
    let team = edited(14, |p| p.name_color = Some("cc9900ff".to_string()));
    assert_eq!(problems(&rules, &team), vec![]);
}

/// Incisive Run and Long Ranger are free by name beyond the tier's COM
/// allowance: a regular at the card limit (slot 14, seven non-free skills)
/// holding both and one more COM style — the allowance spent — is still
/// legal; a sixth COM style is the `max_com_styles` violation alone (his
/// non-free count is seven either way).
#[test]
fn incisive_run_and_long_ranger_are_free_and_coms_cap_at_five() {
    let rules = rules();
    let team = edited(14, |p| {
        for name in ["long_throw", "acrobatic_clear", "penalty_specialist"] {
            p.cards.push(card(name));
        }
        for name in ["incisive_run", "long_ranger", "trickster"] {
            p.cards.push(card(name));
        }
    });
    assert_eq!(problems(&rules, &team), vec![]);
    let team = edited(14, |p| {
        for name in [
            "incisive_run",
            "long_ranger",
            "trickster",
            "mazing_run",
            "speeding_bullet",
            "long_ball_expert",
        ] {
            p.cards.push(card(name));
        }
    });
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(14),
            ViolationKind::ComStyleCap { count: 6, max: 5 }
        )]
    );
}

/// The skill-card cap is twelve — VTL11's is eleven: a silver (slot 2,
/// ten skill cards) up to twelve is legal; a thirteenth is the
/// `SkillCardCap` violation.
#[test]
fn the_skill_card_cap_is_twelve() {
    let rules = rules();
    let skills = |slot: u8| {
        player(&legal_team(), slot)
            .cards
            .iter()
            .filter(|c| matches!(c, Card::Skill(_)))
            .count()
    };
    assert_eq!(skills(2), 10);
    let team = edited(2, |p| {
        p.cards.push(card("knuckle_shot"));
        p.cards.push(card("low_lofted_pass"));
    });
    assert_eq!(problems(&rules, &team), vec![]);
    let team = edited(2, |p| {
        p.cards.push(card("knuckle_shot"));
        p.cards.push(card("low_lofted_pass"));
        p.cards.push(card("outside_curler"));
    });
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(2),
            ViolationKind::SkillCardCap { count: 13, max: 12 }
        )]
    );
}

/// The gold's card economy is tighter than VTL11's: six non-free cards and
/// two free COM styles (seven and three there). Slot 9, at his non-free
/// limit, taking a seventh is the `Cards` violation; a third non-free
/// COM style over the allowance is non-free, and at the limit is the same
/// violation — under VTL11 both were inside it.
#[test]
fn a_golds_allowance_is_six_non_free_and_two_com() {
    let rules = rules();
    let team = edited(9, |p| p.cards.push(card("knuckle_shot")));
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(9),
            ViolationKind::Cards {
                non_free: 7,
                limit: 6
            }
        )]
    );
    let team = edited(9, |p| {
        p.cards.push(card("trickster"));
        p.cards.push(card("mazing_run"));
    });
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(9),
            ViolationKind::Cards {
                non_free: 7,
                limit: 6
            }
        )]
    );
}

/// The team tactics settings apply on PES 16 alone: the same team built
/// on that version is legal until each setting is turned on — man
/// marking, auto substitution, offside trap, preset change — each its
/// own violation, and man marking on PES 15 is no violation at all.
#[test]
fn the_pes16_team_settings() {
    let rules = rules();
    let team = legalize(Records::sample_on(PesVersion::Pes16)).view();
    assert_eq!(team.version, PesVersion::Pes16);
    assert_eq!(problems(&rules, &team), vec![]);
    let setting = |label: &str, name: &str| {
        error(
            None,
            ViolationKind::TeamSetting {
                label: label.to_string(),
                setting: name.to_string(),
                wanted: false,
            },
        )
    };
    let mut t = team.clone();
    t.tactics.presets[0].man_marking[4] = 3;
    assert_eq!(
        problems(&rules, &t),
        vec![setting("No man marking on PES 16", "man_marking")]
    );
    let mut t = team.clone();
    t.tactics.auto.substitution = Some(2);
    assert_eq!(
        problems(&rules, &t),
        vec![setting("PES 16 support settings", "auto_substitution")]
    );
    let mut t = team.clone();
    t.tactics.auto.offside_trap = Some(true);
    assert_eq!(
        problems(&rules, &t),
        vec![setting("PES 16 support settings", "auto_offside_trap")]
    );
    let mut t = team.clone();
    t.tactics.auto.preset_change = Some(true);
    assert_eq!(
        problems(&rules, &t),
        vec![setting("PES 16 support settings", "auto_preset_change")]
    );
    let mut t = legal_team();
    t.tactics.presets[0].man_marking[4] = 3;
    assert_eq!(problems(&rules, &t), vec![], "not on PES 15");
}

/// `card_limits` per tier: the thirteen free skills, Incisive Run and
/// Long Ranger for everyone, the three pass skills on non-medals alone,
/// the captain's Captaincy, the `com` counts and the A-position
/// allowances.
#[test]
fn card_limits_name_the_economy() {
    use std::collections::BTreeMap;

    let rules = rules();
    let team = legal_team();
    let team_reading = reading(&rules, &team);
    let limits = |slot: u8, tier: &str| {
        rules
            .card_limits(&team_reading, team.player(slot).expect("filled"), tier)
            .expect("limits")
    };
    let free_skills = [
        "scissors_feint",
        "flip_flap",
        "marseille_turn",
        "sombrero",
        "cut_behind_and_turn",
        "scotch_move",
        "acrobatic_finishing",
        "heel_trick",
        "first_time_shot",
        "malicia",
        "double_touch",
        "crossover_turn",
        "step_on_skill",
    ]
    .map(card);
    let regular = limits(14, "regular");
    assert_eq!(
        regular.free_cards,
        [
            free_skills.as_slice(),
            &[
                card("one_touch_pass"),
                card("weighted_pass"),
                card("pinpoint_crossing"),
                card("incisive_run"),
                card("long_ranger"),
            ]
        ]
        .concat()
    );
    assert_eq!(regular.free, BTreeMap::from([("com".to_string(), 1)]));
    assert_eq!(regular.required_cards, vec![card("malicia")]);
    assert_eq!((regular.limit, regular.a_positions), (7, 2));
    let gold = limits(9, "gold");
    assert_eq!(
        gold.free_cards,
        [
            free_skills.as_slice(),
            &[card("incisive_run"), card("long_ranger")]
        ]
        .concat()
    );
    assert_eq!(gold.free, BTreeMap::from([("com".to_string(), 2)]));
    assert_eq!((gold.limit, gold.a_positions), (6, 2));
    let captain = limits(10, "gold");
    assert!(captain.free_cards.contains(&card("captaincy")));
    assert_eq!(
        (
            limits(5, "silver").free["com"],
            limits(5, "silver").a_positions
        ),
        (2, 3)
    );
    assert_eq!(limits(3, "buffed").a_positions, 1);

    // The whole economy per tier — the card limit, the A positions,
    // the free counts (the exact map, so a group named nowhere in it
    // shows) and `max_a_positions` unset — plus `tier_values`' weak
    // foot. Slots: 9 for the gold, 5 the silver, 14 the regular; for
    // the buffed, slot 7 — registered CMF, so no wingback A and no
    // debuff. Only the gold's card figures are written down
    // (`spec/aatf_rules.md` "VTL10"); the other numbers are the ones the
    // ruleset file holds.
    let weak_foot = |slot: u8, tier: &str| {
        rules
            .tier_values(&team_reading, team.player(slot).expect("filled"), tier)
            .expect("values")
            .weak_foot
    };
    assert_economy(
        &rules,
        &team,
        &[
            (9, "gold", 6, 2, &[("com", 2)], (4, 4)),
            (5, "silver", 7, 3, &[("com", 2)], (4, 4)),
            (14, "regular", 7, 2, &[("com", 1)], (4, 4)),
            (7, "buffed", 7, 1, &[("com", 1)], (4, 4)),
        ],
    );
    // Slot 3, the buffed wingback (A at RB), reads the special's debuff
    // instead: weak foot 2/2.
    let debuffed = weak_foot(3, "buffed");
    assert_eq!((debuffed.usage, debuffed.accuracy), (2, 2));
}
