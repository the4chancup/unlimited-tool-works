//! VTL11's fixtures. The sample team made legal under VTL11: the tiers
//! read off exact heights (medals at 195 told apart by swerve, the
//! buffed at 188, the rest — goalkeepers included — regulars at 180):
//! slots 9 and 10 gold, 2, 5 and 20 silver, 3, 4, 6, 7, 12, 15, 16 and
//! 22 buffed with 15 and 22 re-registered LB to RB so the 188 cm
//! players' registered positions are RB and CMF alone, which no 180 cm
//! player shares. Every player holds Malicia (required) and three paid
//! skill cards (`min_cards`); the medals' names carry their colour
//! codes, which `apply_tier` writes.

use super::*;
use crate::ruleset::{Card, PlayStyle};
use pes_savefile::model::playstyle::PlayStyle as SavePlayStyle;
use pes_savefile::schema::playstyle;

/// The VTL11 data block, a template ruleset with custom checks.
const VTL11: &str = include_str!("../../rulesets/vtl11.rhai");

/// VTL11's rules loaded.
fn rules() -> Rules {
    Rules::from_data_block(VTL11).expect("VTL11 loads")
}

/// The playing style `name` names.
fn play_style(name: &str) -> PlayStyle {
    PlayStyle::try_from(name.to_string()).expect("a playing style name")
}

/// `records` (the sample team, on either build's version) made legal
/// under VTL11:
///
/// - heights by hand, then each tier's targets through
///   `Rules::apply_tier` (which also writes the medals' name colours):
///   slots 9 and 10 gold (195 cm, swerve 99), 2, 5 and 20 silver (195,
///   swerve 89), 3, 4, 6, 7, 12, 15, 16 and 22 buffed (188), the rest
///   regulars at 180, the goalkeepers 0 and 11 among them (form 8 by
///   the goalkeeper special);
/// - slots 15 and 22 re-registered LB to RB — the A moving with the
///   registration — so the 188 cm players' registered positions are RB
///   and CMF alone;
/// - slot 10, the captain, made a prolific winger: the sample's goal
///   poacher rated A at CF draws VTL11's warning, and the legal team
///   reports nothing (no gold special keys on prolific_winger);
/// - Malicia (required) and two paid skill cards — fighting_spirit,
///   interception — to every player: the held cards already fit (the
///   pass skills free on non-medals, the COM styles inside the free
///   counts, Long Ranger and the captain's Captaincy free by name), and
///   the additions put everyone at or over `min_cards` (the captain at
///   the paid limit).
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
    let goal_poacher = playstyle::encode(records.version, SavePlayStyle::GoalPoacher)
        .expect("the sample's versions have Goal Poacher");
    let prolific_winger = playstyle::encode(records.version, SavePlayStyle::ProlificWinger)
        .expect("the sample's versions have Prolific Winger");
    let entry = records.player_mut(10);
    assert_eq!(entry.positions.playing_style, goal_poacher);
    entry.positions.playing_style = prolific_winger;
    let rules = rules();
    for (slot, tier, height) in [
        (9, "gold", 195),
        (10, "gold", 195),
        (2, "silver", 195),
        (5, "silver", 195),
        (20, "silver", 195),
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
            .apply_tier(&view, &player, tier, records.player_mut(slot))
            .expect("the tier applies");
    }
    for slot in [0, 1, 8, 11, 13, 14, 17, 18, 19, 21] {
        records.player_mut(slot).basic.height = 180;
        let view = records.view();
        let player = view.player(slot).expect("a filled slot").clone();
        rules
            .apply_tier(&view, &player, "regular", records.player_mut(slot))
            .expect("the tier applies");
    }
    for slot in 0..23u8 {
        let entry = records.player_mut(slot);
        for name in ["malicia", "fighting_spirit", "interception"] {
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
    assert_eq!(rules.ruleset().name, "/vt/ League 11");
    assert_eq!(problems(&rules, &legal_team()), vec![]);
}

/// For every tier of the file, a player of that tier in the legal team —
/// stats, form, injury resistance, weak foot and name broken — is legal
/// again after `apply_tier` (the gold's name colour written included).
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
        entry.name = "NO COLOUR".to_string();
        let broken = records.view();
        assert!(!problems(&rules, &broken).is_empty(), "{tier} {slot}");
        let player = broken.player(slot).expect("filled").clone();
        rules
            .apply_tier(&broken, &player, tier, records.player_mut(slot))
            .expect("the tier applies");
        assert_eq!(problems(&rules, &records.view()), vec![], "{tier} {slot}");
    }
}

/// The 195 cm medals are told apart by swerve alone: a silver (slot 5,
/// swerve 89) at 90 fits no tier's `when`, and at 99 reads as gold —
/// every stat the gold's targets ask is a violation (PES 15 skips the
/// stats it lacks), the name colour the gold's, the counts moved.
#[test]
fn silver_and_gold_at_195_differ_only_in_swerve() {
    let rules = rules();
    let swerve = usize::from(stat("swerve").0);
    assert_eq!(player(&legal_team(), 5).abilities[swerve], Some(89));
    assert_eq!(player(&legal_team(), 9).abilities[swerve], Some(99));
    let team = edited(5, |p| p.abilities[swerve] = Some(90));
    assert_eq!(
        problems(&rules, &team),
        vec![
            // 195 is the medals' own height: the cause is the swerve.
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
    assert_eq!(found.len(), 21, "{found:?}");
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
            Some(5),
            ViolationKind::NameColor {
                color: Some("51bbc4ff".to_string()),
                expected: Some("cc9900ff".to_string())
            }
        )),
        "{found:?}"
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

/// The 195 cm silver (slot 5) at swerve 90 fits no tier's `when` (where
/// `check_team` reports `NoTier`,
/// `silver_and_gold_at_195_differ_only_in_swerve`) and has no tier; at
/// 99 he reads `gold`.
#[test]
fn a_player_no_tier_takes_has_no_tier() {
    let rules = rules();
    let swerve = usize::from(stat("swerve").0);
    let team = edited(5, |p| p.abilities[swerve] = Some(90));
    assert_eq!(tier_of(&rules, &team, 5), None);
    let team = edited(5, |p| p.abilities[swerve] = Some(99));
    assert_eq!(tier_of(&rules, &team, 5), Some("gold"));
}

/// The gold's playing-style bonuses: slot 9's creative_playmaker moves
/// his stamina target to 99 and defensive prowess to 60, what
/// `apply_tier` wrote — met is legal, short of it the `Stat` violation,
/// and losing the style puts both targets back. The other two fire the
/// same way on the PES 21 build (whose `physical_contact` PES 15
/// lacks): a Fox in the Box's ball winning and a Target Man's physical
/// contact, each to 99 with defensive prowess at 60.
#[test]
fn a_golds_playing_style_moves_his_targets() {
    let rules = rules();
    let gold = player(&legal_team(), 9).clone();
    assert_eq!(gold.playing_style, Some(play_style("creative_playmaker")));
    assert_eq!(gold.abilities[usize::from(stat("stamina").0)], Some(99));
    assert_eq!(
        gold.abilities[usize::from(stat("defensive_prowess").0)],
        Some(60)
    );
    assert_eq!(problems(&rules, &legal_team()), vec![]);
    assert_eq!(
        problems(
            &rules,
            &edited(9, |p| p.abilities[usize::from(stat("stamina").0)] =
                Some(94))
        ),
        vec![error(
            Some(9),
            ViolationKind::Stat {
                stat: stat("stamina"),
                value: 94,
                target: 99,
                at_most: false
            }
        )]
    );
    let found = problems(
        &rules,
        &edited(9, |p| p.playing_style = Some(play_style("none"))),
    );
    assert_eq!(
        found,
        vec![
            error(
                Some(9),
                ViolationKind::Stat {
                    stat: stat("defensive_prowess"),
                    value: 60,
                    target: 65,
                    at_most: false
                }
            ),
            error(
                Some(9),
                ViolationKind::Stat {
                    stat: stat("stamina"),
                    value: 99,
                    target: 94,
                    at_most: false
                }
            )
        ]
    );

    // On PES 21: Fox in the Box (+5 ball winning) and Target Man (+5
    // physical contact), each with the -5 defensive prowess. The
    // creative_playmaker bonus no longer applies, so stamina goes back
    // to the tier's 94 either way.
    let (bw, pc, st, dp) = (
        stat("ball_winning"),
        stat("physical_contact"),
        stat("stamina"),
        stat("defensive_prowess"),
    );
    let team = legalize(Records::sample_on(PesVersion::Pes21)).view();
    assert_eq!(team.version, PesVersion::Pes21);
    let gold = player(&team, 9).clone();
    assert_eq!(gold.playing_style, Some(play_style("creative_playmaker")));
    assert_eq!(gold.abilities[usize::from(st.0)], Some(99));
    assert_eq!(gold.abilities[usize::from(bw.0)], Some(94));
    assert_eq!(gold.abilities[usize::from(pc.0)], Some(94));
    assert_eq!(gold.abilities[usize::from(dp.0)], Some(60));
    // `team` with slot 9's style changed and stamina back at 94.
    let styled = |team: &TeamView, style: PlayStyle| {
        let mut team = team.clone();
        let player = player_mut(&mut team, 9);
        player.playing_style = Some(style);
        player.abilities[usize::from(st.0)] = Some(94);
        team
    };
    let fib = play_style("fox_in_the_box");
    let target_man = play_style("target_man");
    // The bonus met: legal; the tier's unadjusted values (ball winning
    // or physical_contact 94, defensive_prowess 65): off the moved
    // targets.
    let mut legal = styled(&team, fib);
    player_mut(&mut legal, 9).abilities[usize::from(bw.0)] = Some(99);
    assert_eq!(problems(&rules, &legal), vec![]);
    let mut unadjusted = styled(&team, fib);
    player_mut(&mut unadjusted, 9).abilities[usize::from(dp.0)] = Some(65);
    assert_eq!(
        problems(&rules, &unadjusted),
        vec![
            error(
                Some(9),
                ViolationKind::Stat {
                    stat: dp,
                    value: 65,
                    target: 60,
                    at_most: false
                }
            ),
            error(
                Some(9),
                ViolationKind::Stat {
                    stat: bw,
                    value: 94,
                    target: 99,
                    at_most: false
                }
            )
        ]
    );
    let mut legal = styled(&team, target_man);
    player_mut(&mut legal, 9).abilities[usize::from(pc.0)] = Some(99);
    assert_eq!(problems(&rules, &legal), vec![]);
    let mut unadjusted = styled(&team, target_man);
    player_mut(&mut unadjusted, 9).abilities[usize::from(dp.0)] = Some(65);
    assert_eq!(
        problems(&rules, &unadjusted),
        vec![
            error(
                Some(9),
                ViolationKind::Stat {
                    stat: dp,
                    value: 65,
                    target: 60,
                    at_most: false
                }
            ),
            error(
                Some(9),
                ViolationKind::Stat {
                    stat: pc,
                    value: 94,
                    target: 99,
                    at_most: false
                }
            )
        ]
    );
}

/// The exe's two playing-style warnings at CF, outside the error count:
/// a Goal Poacher does not activate there, a Creative Playmaker does.
/// Slots 10 and 2 are rated A at CF already (registered there, so no
/// allowance is spent); slot 9 is a creative playmaker whose only A is
/// SS. A gold Creative Playmaker's targets move, so the CF case for
/// that style is slot 2, a silver.
#[test]
fn the_playing_style_warnings_at_cf() {
    let rules = rules();
    assert_eq!(
        problems(
            &rules,
            &edited(10, |p| p.playing_style = Some(play_style("goal_poacher")))
        ),
        vec![warning(
            10,
            special(
                "Goal Poacher at CF",
                "Goal Poacher does not activate at CF with the league's exe"
            )
        )]
    );
    assert_eq!(
        problems(
            &rules,
            &edited(2, |p| p.playing_style =
                Some(play_style("creative_playmaker")))
        ),
        vec![warning(
            2,
            special(
                "Creative Playmaker at CF",
                "Creative Playmaker activates at CF with the league's exe"
            )
        )]
    );
    assert_eq!(
        problems(
            &rules,
            &edited(9, |p| p.playing_style =
                Some(play_style("creative_playmaker")))
        ),
        vec![],
        "a creative playmaker not rated A at CF: no warning"
    );
    // Losing the creative_playmaker bonus is the two `Stat` violations
    // `a_golds_playing_style_moves_his_targets` lists; no warning joins
    // them, slot 9 not being rated A at CF.
    assert_eq!(
        problems(
            &rules,
            &edited(9, |p| p.playing_style = Some(play_style("goal_poacher")))
        ),
        vec![
            error(
                Some(9),
                ViolationKind::Stat {
                    stat: stat("defensive_prowess"),
                    value: 60,
                    target: 65,
                    at_most: false
                }
            ),
            error(
                Some(9),
                ViolationKind::Stat {
                    stat: stat("stamina"),
                    value: 99,
                    target: 94,
                    at_most: false
                }
            )
        ]
    );
}

/// Malicia is required on everyone: a regular (slot 14) without it gets
/// the `RequiredCard` violation (the card is free, so his paid count is
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

/// The GK Penalty Specialist card is banned on everyone: a regular
/// (slot 14) holding it gets the special's violation — it is a paid
/// card, so his paid count rises to four, still inside the limit.
#[test]
fn the_gk_penalty_specialist_is_banned() {
    let rules = rules();
    assert_eq!(
        problems(
            &rules,
            &edited(14, |p| p.cards.push(card("gk_penalty_specialist")))
        ),
        vec![error(
            Some(14),
            special(
                "The GK Penalty Specialist card is banned",
                "The GK Penalty Specialist card is banned"
            )
        )]
    );
}

/// `min_cards` asks three paid skill cards, each extra A position
/// counting as one: a regular (slot 1, two A positions of two allowed)
/// down to two paid skills gets the violation, and is legal again with a
/// third A position (which also costs him a card slot).
#[test]
fn three_paid_skill_cards_or_the_extra_a_positions() {
    let rules = rules();
    assert_eq!(
        player(&legal_team(), 1)
            .ratings
            .iter()
            .filter(|r| **r == 2)
            .count(),
        2
    );
    let team = edited(1, |p| p.cards.retain(|c| *c != card("interception")));
    assert_eq!(
        problems(&rules, &team),
        vec![error(Some(1), ViolationKind::MinCards { paid: 2, min: 3 })]
    );
    let team = edited(1, |p| {
        p.cards.retain(|c| *c != card("interception"));
        p.ratings[usize::from(position("DMF").0)] = 2;
    });
    assert_eq!(problems(&rules, &team), vec![]);
}

/// The medals' names carry their tier's colour code, and a non-medal
/// carries neither: the gold (slot 9) without his, and a regular (slot
/// 14) wearing the gold code, each get `NameColor`.
#[test]
fn the_name_colours_are_checked() {
    let rules = rules();
    assert_eq!(
        problems(&rules, &edited(9, |p| p.name_color = None)),
        vec![error(
            Some(9),
            ViolationKind::NameColor {
                color: None,
                expected: Some("cc9900ff".to_string())
            }
        )]
    );
    assert_eq!(
        problems(
            &rules,
            &edited(14, |p| p.name_color = Some("cc9900ff".to_string()))
        ),
        vec![error(
            Some(14),
            ViolationKind::NameColor {
                color: Some("cc9900ff".to_string()),
                expected: None
            }
        )]
    );
}

/// The 188 cm players may hold a B at CB alone — it still counts toward
/// their single A-position allowance (paid three, one extra position,
/// inside the limit): legal; a B anywhere else, or an A at CB, is the
/// special's violation each.
#[test]
fn a_188s_b_rating_only_at_cb_and_never_an_a_there() {
    let rules = rules();
    let (cb, lb) = (position("CB"), position("LB"));
    let team = edited(3, |p| p.ratings[usize::from(cb.0)] = 1);
    assert_eq!(problems(&rules, &team), vec![]);
    assert_eq!(
        problems(&rules, &edited(3, |p| p.ratings[usize::from(lb.0)] = 1)),
        vec![error(
            Some(3),
            special(
                "B positions only for 188cm players at CB",
                "Illegal B positions, only allowed on 188cm players at CB"
            )
        )]
    );
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

/// The permitted B at CB still spends the one-A allowance
/// (`b_uses_a_allowance`): a buffed player at his paid limit (slot 3,
/// three paid in the legal team, seven after the added skills) gains it
/// and is over — the B counts toward the allowance, an extra position
/// costing a card; `card_limits` reports the limit one lower.
#[test]
fn the_permitted_b_at_cb_spends_the_a_allowance() {
    let rules = rules();
    let cb = position("CB");
    let at_limit = |p: &mut PlayerView| {
        for name in [
            "long_throw",
            "acrobatic_clear",
            "track_back",
            "penalty_specialist",
        ] {
            p.cards.push(card(name));
        }
    };
    assert_eq!(player(&legal_team(), 3).cards.len(), 6);
    let team = edited(3, at_limit);
    assert_eq!(problems(&rules, &team), vec![]);
    let team = edited(3, |p| {
        at_limit(p);
        p.ratings[usize::from(cb.0)] = 1;
    });
    assert_eq!(
        problems(&rules, &team),
        vec![error(Some(3), ViolationKind::Cards { paid: 7, limit: 6 })]
    );
    let limits = rules
        .card_limits(&team, team.player(3).expect("slot 3"), "buffed")
        .expect("limits");
    assert_eq!(limits.limit, 6, "the allowance spent costs a card");
}

/// The custom checks: the 188 cm players' registered positions number at
/// most two — a third (slot 3 moved RB to AMF) is the team's violation;
/// a 180 cm player sharing one (slot 19 moved to RB) is his own; and a
/// third goalkeeper (the same slot made a GK, form 8 as the special
/// asks) is the team's again.
#[test]
fn the_custom_checks() {
    let rules = rules();
    let amf = position("AMF");
    let team = edited(3, |p| {
        p.position = Some(amf);
        p.ratings[usize::from(position("RB").0)] = 0;
        p.ratings[usize::from(amf.0)] = 2;
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
            "A 180cm player shares a 188cm player's registered position"
        )]
    );
    let gk = position("GK");
    let team = edited(19, |p| {
        p.position = Some(gk);
        p.ratings[usize::from(position("LB").0)] = 0;
        p.ratings[usize::from(gk.0)] = 2;
        p.form = 8;
    });
    assert_eq!(
        problems(&rules, &team),
        vec![custom(None, "More than two goalkeepers")]
    );
}

/// Regulars — the buffed included — cannot be captain: the armband on a
/// regular (slot 14) is the special's violation (the former captain's
/// Captaincy card is paid once it is no longer his, so it goes too).
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
/// at the regulars' 4) is the team's `Goalkeepers` violation alone.
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
        vec![error(None, ViolationKind::Goalkeepers { count: 1, min: 2 })]
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

/// `card_limits` per tier: the thirteen free skills and Long Ranger for
/// everyone, the three pass skills on non-medals alone, the captain's
/// Captaincy, the `com` counts and the A-position allowances.
#[test]
fn card_limits_name_the_economy() {
    use std::collections::BTreeMap;

    let rules = rules();
    let team = legal_team();
    let limits = |slot: u8, tier: &str| {
        rules
            .card_limits(&team, team.player(slot).expect("filled"), tier)
            .expect("limits")
    };
    let free_skills = [
        "no_look_pass",
        "double_touch",
        "rabona",
        "acrobatic_finishing",
        "first_time_shot",
        "dipping_shots",
        "rising_shots",
        "heading",
        "malicia",
        "flip_flap",
        "marseille_turn",
        "scotch_move",
        "crossover_turn",
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
        [free_skills.as_slice(), &[card("long_ranger")]].concat()
    );
    assert_eq!(gold.free, BTreeMap::from([("com".to_string(), 3)]));
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

    // The whole economy per tier — the paid limit, the A positions,
    // the free counts (the exact map, so a group named nowhere in it
    // shows) and `max_a_positions` unset — plus `tier_values`' weak
    // foot. Slots: 9 for the gold (the playing-style bonus is a stat
    // bonus and changes none of these numbers), 5 the silver, 14 the
    // regular, 3 the buffed. The values are the ones the ruleset file
    // holds: `spec/aatf_rules.md` "Background" names no number for these
    // fields.
    assert_economy(
        &rules,
        &team,
        &[
            (9, "gold", 7, 2, &[("com", 3)], (4, 4)),
            (5, "silver", 7, 3, &[("com", 2)], (4, 4)),
            (14, "regular", 7, 2, &[("com", 1)], (4, 4)),
            (3, "buffed", 7, 1, &[("com", 1)], (4, 4)),
        ],
    );
}

/// Long Ranger is free by name beyond the tier's COM allowance: a
/// regular at the paid limit (slot 14, seven paid skills) holding it and
/// one more COM style — the allowance spent — is still legal; a sixth
/// COM style is the `max_com_styles` violation alone (his paid count is
/// seven either way).
#[test]
fn long_ranger_is_free_and_coms_cap_at_five() {
    let rules = rules();
    let paid = |p: &mut PlayerView| {
        for name in [
            "long_throw",
            "acrobatic_clear",
            "track_back",
            "penalty_specialist",
        ] {
            p.cards.push(card(name));
        }
    };
    let team = edited(14, |p| {
        paid(p);
        p.cards.push(card("long_ranger"));
        p.cards.push(card("trickster"));
    });
    assert_eq!(problems(&rules, &team), vec![]);
    let team = edited(14, |p| {
        for name in [
            "long_ranger",
            "trickster",
            "mazing_run",
            "speeding_bullet",
            "incisive_run",
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

/// Weak foot is exact (`at_most` is empty): under the 4/4 every tier
/// sets, a regular at 3/4 and one at 4/3 each report the lower value.
#[test]
fn weak_foot_below_the_exact_target() {
    let rules = rules();
    let regular = legal_team();
    let regular = player(&regular, 14);
    assert_eq!(
        (regular.weak_foot_usage, regular.weak_foot_accuracy),
        (4, 4)
    );
    assert_eq!(
        problems(&rules, &edited(14, |p| p.weak_foot_usage = 3)),
        vec![error(
            Some(14),
            ViolationKind::WeakFootUsage { value: 3, max: 4 }
        )]
    );
    assert_eq!(
        problems(&rules, &edited(14, |p| p.weak_foot_accuracy = 3)),
        vec![error(
            Some(14),
            ViolationKind::WeakFootAccuracy { value: 3, max: 4 }
        )]
    );
}

/// The skill-card cap is eleven, over the game's ten (the cup's saves
/// load them): eleven skills on a regular (slot 14 — eight free, three
/// paid, inside the budget) report nothing; a twelfth, free as well, is
/// the cap alone.
#[test]
fn eleven_skills_are_the_cap() {
    let rules = rules();
    let eleven = |p: &mut PlayerView| {
        p.cards = [
            "malicia",
            "no_look_pass",
            "double_touch",
            "rabona",
            "acrobatic_finishing",
            "first_time_shot",
            "dipping_shots",
            "rising_shots",
            "fighting_spirit",
            "interception",
            "track_back",
        ]
        .into_iter()
        .map(card)
        .collect();
    };
    assert_eq!(problems(&rules, &edited(14, eleven)), vec![]);
    let team = edited(14, |p| {
        eleven(p);
        p.cards.push(card("heading"));
    });
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(14),
            ViolationKind::SkillCardCap { count: 12, max: 11 }
        )]
    );
}

/// The fixture teams the renamed-file comparison runs on: the legal
/// team, a Silver player's ability stat over its target, a Silver
/// player without his name colour, and a regular made to read as a
/// fourth silver (his counts off by one each way). A tier's label
/// reaches no violation — its id does — so a handful of teams proves
/// the rule as well as every one the file builds.
fn fixture_teams() -> Vec<(&'static str, TeamView)> {
    vec![
        ("legal", legal_team()),
        (
            "a silver over his target",
            edited(5, |p| {
                p.abilities[usize::from(stat("ball_control").0)] = Some(95)
            }),
        ),
        (
            "a silver without his colour",
            edited(5, |p| p.name_color = None),
        ),
        (
            "one more silver",
            edited(14, |p| {
                p.height = 195;
                p.abilities[usize::from(stat("swerve").0)] = Some(89);
            }),
        ),
    ]
}

/// VTL11 with the Silver tier's label changed, assembled with the
/// current logic and the file's own custom checks: every fixture team
/// reports the same violations as under the original — a tier's label
/// reaches no violation (its id does).
// RE-FILE-04
#[test]
fn the_same_teams_pass_and_fail_with_silvers_label_changed() {
    let mut renamed = rules().ruleset().clone();
    renamed
        .tiers
        .iter_mut()
        .find(|tier| tier.id == "silver")
        .expect("the silver tier")
        .label = "Argent".to_string();
    let checks = crate::file::file_sections(VTL11)
        .custom_checks
        .expect("VTL11 has checks");
    let text = crate::file::assemble_file(&renamed, None, Some(&checks));
    let renamed = Rules::load(&text).expect("the renamed file loads");
    let rules = rules();
    assert!(
        fixture_teams().iter().any(|(_, team)| {
            rules
                .check_team(team)
                .expect("the original checks")
                .violations
                .iter()
                .any(|violation| {
                    matches!(
                        &violation.kind,
                        ViolationKind::TierCount { tier, .. } if tier == "silver"
                    )
                })
        }),
        "premise: a violation under silver's id is compared"
    );
    for (name, team) in fixture_teams() {
        assert_eq!(
            rules.check_team(&team).expect("the original checks"),
            renamed.check_team(&team).expect("the renamed checks"),
            "{name}"
        );
    }
}
