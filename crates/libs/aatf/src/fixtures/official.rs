//! The official ruleset's fixtures (Autumn 26). Roster slots name the
//! sample team's players: slot 2 and 10 its golds, 5, 9 and 20 its
//! silvers, 0 and 11 its goalkeepers (189 cm), 3, 4, 6, 15, 19 and 22 its
//! manlets (175 cm), 5, 8, 14, 17 and 20 its giants (194 cm).

use std::collections::BTreeMap;

use super::*;
use crate::ruleset::Card;
use crate::values::CardLimits;
use crate::violation::CheckError;

/// The sample team (legal under Spring 26: two golds, three silvers at 88,
/// goalkeepers at 74, Green quotas 5 giants, 6 tall, 6 mid, 6 manlets)
/// made legal under Autumn 26 by the smallest edits: one gold becomes a
/// silver and two silvers bronzes, the silver giant takes the Green
/// giants' -3, the goalkeepers rate 77, and one mid player becomes a sixth
/// giant (Autumn 26 quotas 6 giants, 6 tall, 5 mid, 6 manlets). The
/// Green system, both 189 cm goalkeepers counted as tall, the manlets'
/// weak foot and the silver giant's -3 are exercised by this team alone.
fn legal_records() -> Records {
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
    assert_eq!(records.player_mut(5).basic.height, 194);
    assert_eq!(records.player_mut(1).basic.height, 180);
    set_tier_record(records.player_mut(10), 92, 8, 3);
    set_tier_record(records.player_mut(5), 89, 8, 3);
    set_tier_record(records.player_mut(9), 86, 8, 3);
    set_tier_record(records.player_mut(20), 86, 8, 3);
    set_tier_record(records.player_mut(0), 77, 4, 1);
    set_tier_record(records.player_mut(11), 77, 4, 1);
    records.player_mut(1).basic.height = 194;
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
    assert_eq!(problems(&Rules::official(), &legal_team()), vec![]);
}

/// Slot 3 (a regular RB, A at RB alone) rated C at RB.
#[test]
fn registered_position_not_a() {
    let rules = Rules::official();
    let rb = position("RB");
    assert_eq!(player(&legal_team(), 3).ratings[usize::from(rb.0)], 2);
    let team = edited(3, |p| p.ratings[usize::from(rb.0)] = 0);
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(3),
            // RB was his one A position.
            ViolationKind::RegisteredNotA { rated_a: vec![] }
        )]
    );
}

/// Slot 3 given a B at CB.
#[test]
fn b_rating() {
    let cb = position("CB");
    let team = edited(3, |p| p.ratings[usize::from(cb.0)] = 1);
    assert_eq!(
        problems(&Rules::official(), &team),
        vec![error(
            Some(3),
            // The B just given at CB.
            ViolationKind::BRating { rated_b: vec![cb] }
        )]
    );
}

/// Slot 12 (a regular CMF holding two skills and one COM style, so his
/// three paid cards sit at the limit) with one card dropped, which costs
/// nothing, then given an A at GK: the GK rating is his second A. Without
/// the dropped card the extra A would also cost a card.
#[test]
fn gk_second_a() {
    let rules = Rules::official();
    let gk = position("GK");
    let dropped = edited(12, |p| {
        assert_eq!(p.cards.len(), 3);
        p.cards.pop();
    });
    assert_eq!(problems(&rules, &dropped), vec![]);
    let mut team = dropped;
    player_mut(&mut team, 12).ratings[usize::from(gk.0)] = 2;
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

/// The deadline export's first preset fields every starter at one of his
/// A positions: slot 9 (rated A at SS alone) moved to AMF in preset 1's
/// kick-off formation is the violation; the same move in preset 2 is not
/// checked, and a substitute (slot 14, outside the starting eleven) has
/// no fielded position to check.
#[test]
fn a_starter_is_fielded_only_at_his_a_positions_in_preset_1() {
    let rules = Rules::official();
    let (ss, amf) = (position("SS"), position("AMF"));
    let team = legal_team();
    // Premise: slot 9 is the starter at formation index 8, fielded at SS
    // in presets 1 and 2's kick-off formations and rated C at AMF.
    let index = team
        .tactics
        .starting_eleven
        .iter()
        .position(|slot| *slot == 9)
        .expect("slot 9 starts");
    assert_eq!(player(&team, 9).position, Some(ss));
    assert_eq!(player(&team, 9).ratings[usize::from(amf.0)], 0);
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
    // A substitute has no fielded position to check.
    assert!(team.fielded_positions(14, 1).is_empty());
    // The same move in preset 2 is not checked.
    let mut team = legal_team();
    team.tactics.presets[1].formations[0].players[index].position = amf.0;
    assert_eq!(problems(&rules, &team), vec![]);
}

#[test]
fn age_outside_the_bounds() {
    let rules = Rules::official();
    for age in [14, 51] {
        let team = edited(3, |p| p.age = age);
        assert_eq!(
            problems(&rules, &team),
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
    for age in [15, 50] {
        assert_eq!(problems(&rules, &edited(3, |p| p.age = age)), vec![]);
    }
}

/// Slot 3 is 175 cm: `max(30, 175 - 129)..=175 - 81` is 46 to 94; at
/// 150 cm the height-derived bound (21 to 69) sits below the floor, so
/// `max(30, ..)..=69` is 30 to 69.
#[test]
fn weight_outside_the_rule() {
    let rules = Rules::official();
    for weight in [45, 95] {
        let team = edited(3, |p| p.weight = weight);
        assert_eq!(
            problems(&rules, &team),
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
    assert_eq!(problems(&rules, &edited(3, |p| p.weight = 46)), vec![]);
    // At 150 cm (still a manlet: the bracket's `up_to` is 175) the floor
    // is the lower bound: 29 is under it, 30 is legal.
    let under = edited(3, |p| {
        p.height = 150;
        p.weight = 29;
    });
    assert_eq!(
        problems(&rules, &under),
        vec![error(
            Some(3),
            ViolationKind::Weight {
                weight: 29,
                min: 30,
                max: 69
            }
        )]
    );
    let at_floor = edited(3, |p| {
        p.height = 150;
        p.weight = 30;
    });
    assert_eq!(problems(&rules, &at_floor), vec![]);
}

/// A registered position past the labels, and a playing style the
/// version's table lacks: each `()` in the script.
#[test]
fn position_and_playing_style_ranges() {
    let rules = Rules::official();
    assert_eq!(
        problems(&rules, &edited(3, |p| p.position = None)),
        vec![error(Some(3), ViolationKind::PositionRange)]
    );
    assert_eq!(
        problems(&rules, &edited(3, |p| p.playing_style = None)),
        vec![error(Some(3), ViolationKind::PlayingStyleRange)]
    );
}

/// A rating stat is met exactly: dribbling 78 on a regular is off target
/// 77 (and reads as a regular still, bronze's 86 being above 78).
#[test]
fn a_rating_stat_off_its_target() {
    let dribbling = stat("dribbling");
    let team = edited(3, |p| p.abilities[usize::from(dribbling.0)] = Some(78));
    assert_eq!(
        problems(&Rules::official(), &team),
        vec![error(
            Some(3),
            ViolationKind::Stat {
                stat: dribbling,
                value: 78,
                target: 77,
                at_most: false
            }
        )]
    );
}

/// An `at_most` stat may sit below its target: attacking prowess 70 on a
/// regular is legal, 78 is not.
#[test]
fn an_at_most_stat_may_sit_below() {
    let rules = Rules::official();
    let prowess = stat("attacking_prowess");
    let index = usize::from(prowess.0);
    assert_eq!(
        problems(&rules, &edited(3, |p| p.abilities[index] = Some(70))),
        vec![]
    );
    assert_eq!(
        problems(&rules, &edited(3, |p| p.abilities[index] = Some(78))),
        vec![error(
            Some(3),
            ViolationKind::Stat {
                stat: prowess,
                value: 78,
                target: 77,
                at_most: true
            }
        )]
    );
}

/// Form is met exactly: a regular at 5 or 3 is off target 4.
#[test]
fn form_off_its_target() {
    let rules = Rules::official();
    for form in [5, 3] {
        assert_eq!(
            problems(&rules, &edited(3, |p| p.form = form)),
            vec![error(
                Some(3),
                ViolationKind::Form {
                    value: form,
                    target: 4
                }
            )],
            "{form}"
        );
    }
}

/// Injury resistance is at most the target: a regular at 2 is over 1.
#[test]
fn injury_resistance_over_the_target() {
    assert_eq!(
        problems(&Rules::official(), &edited(3, |p| p.injury_resistance = 2)),
        vec![error(
            Some(3),
            ViolationKind::InjuryResistance { value: 2, max: 1 }
        )]
    );
}

/// Slot 12, a regular who is no manlet, is limited to weak foot 2/2.
#[test]
fn weak_foot_over_the_limits() {
    let rules = Rules::official();
    assert_eq!(player(&legal_team(), 12).height, 180);
    assert_eq!(
        problems(&rules, &edited(12, |p| p.weak_foot_usage = 3)),
        vec![error(
            Some(12),
            ViolationKind::WeakFootUsage { value: 3, max: 2 }
        )]
    );
    assert_eq!(
        problems(&rules, &edited(12, |p| p.weak_foot_accuracy = 3)),
        vec![error(
            Some(12),
            ViolationKind::WeakFootAccuracy { value: 3, max: 2 }
        )]
    );
}

/// Slot 12 pays three cards, a regular's limit: a fourth skill is over
/// it, a trick (two free for a regular) is not, and an extra A position
/// costs a card.
#[test]
fn paid_cards_over_the_limit() {
    let rules = Rules::official();
    assert_eq!(player(&legal_team(), 12).cards.len(), 3);
    assert_eq!(
        problems(
            &rules,
            &edited(12, |p| p.cards.push(card("long_range_drive")))
        ),
        vec![error(Some(12), ViolationKind::Cards { paid: 4, limit: 3 })]
    );
    assert_eq!(
        problems(
            &rules,
            &edited(12, |p| p.cards.push(card("scissors_feint")))
        ),
        vec![]
    );
    let cf = position("CF");
    assert_eq!(
        problems(&rules, &edited(12, |p| p.ratings[usize::from(cf.0)] = 2)),
        vec![error(Some(12), ViolationKind::Cards { paid: 3, limit: 2 })]
    );
}

/// Slot 12 with every card removed and rated A at four more outfield
/// positions: five A positions where a regular is allowed one, so the
/// extra four deduct the paid limit to 3 - 4 = -1 and paid 0 is still
/// over it.
#[test]
fn an_overspend_in_a_positions_is_over_the_limit() {
    let rules = Rules::official();
    let team = edited(12, |p| {
        assert_eq!(p.cards.len(), 3);
        assert_eq!(p.ratings.iter().filter(|rating| **rating == 2).count(), 1);
        p.cards.clear();
        for name in ["DMF", "LMF", "AMF", "CF"] {
            p.ratings[usize::from(position(name).0)] = 2;
        }
    });
    assert_eq!(
        problems(&rules, &team),
        vec![error(Some(12), ViolationKind::Cards { paid: 0, limit: -1 })]
    );
    let limits = rules
        .card_limits(&team, team.player(12).expect("slot 12"), "regular")
        .expect("limits");
    assert_eq!(limits.limit, -1);
}

/// Eleven skills on slot 12 (a regular who is no manlet, so no bonus
/// card), none of them a trick: the game's cap of ten, over the paid limit
/// as well (the cap is never reached under it in this file).
#[test]
fn skill_cards_over_the_games_cap() {
    let team = edited(12, |p| {
        p.cards = [7, 8, 9, 10, 11, 12, 13, 14, 15, 17, 18]
            .into_iter()
            .map(Card::Skill)
            .collect();
    });
    assert_eq!(
        problems(&Rules::official(), &team),
        vec![
            error(Some(12), ViolationKind::Cards { paid: 11, limit: 3 }),
            error(Some(12), ViolationKind::SkillCardCap { count: 11, max: 10 }),
        ]
    );
}

/// Slot 12 (180 cm) at 182 cm, in no bracket: the player's height, and
/// the mid quota he left. (A manlet grown instead would also lose his
/// weak-foot allowance: the test uses a mid player so one rule breaks.)
#[test]
fn a_height_in_no_bracket() {
    assert_eq!(
        problems(&Rules::official(), &edited(12, |p| p.height = 182)),
        vec![
            error(Some(12), no_bracket(182, &[(180, 180)], true)),
            error(
                None,
                ViolationKind::BracketQuota {
                    system: "green".to_string(),
                    bracket: "mid".to_string(),
                    count: 4,
                    quota: 5
                }
            ),
        ]
    );
}

/// The mid bracket takes 180 cm alone ("Five of exactly 180cm", "Seven
/// players of height exactly 180cm"): slot 12 at 178 stands in no
/// bracket under Green or under Red, the mid quota one short each.
#[test]
fn a_height_between_manlet_and_mid_is_in_no_bracket() {
    let rules = Rules::official();
    assert_eq!(player(&legal_team(), 12).height, 180);
    assert_eq!(
        problems(&rules, &edited(12, |p| p.height = 178)),
        vec![
            error(Some(12), no_bracket(178, &[(180, 180)], true)),
            quota("green", "mid", 4, 5),
        ]
    );
    assert_eq!(player(&red_legal_team(), 12).height, 180);
    assert_eq!(
        problems(&rules, &red_edited(12, |p| p.height = 178)),
        vec![
            error(Some(12), no_bracket(178, &[(180, 180)], true)),
            quota("red", "mid", 6, 7),
        ]
    );
}

/// The `Height` violations of `team` under `rules`, alone.
fn height_violations(rules: &Rules, team: &TeamView) -> Vec<Violation> {
    problems(rules, team)
        .into_iter()
        .filter(|violation| matches!(violation.kind, ViolationKind::Height { .. }))
        .collect()
}

/// A mid (slot 12, 180) and a tall gold (slot 2, 185) both at 183: the
/// hint names both brackets left with room, Green's mid (180, quota 5)
/// and tall (185, quota 6), on each of them.
#[test]
fn the_height_hint_names_every_bracket_with_room() {
    let rules = Rules::official();
    let mut team = legal_team();
    assert_eq!(player(&team, 2).height, 185);
    assert_eq!(player(&team, 12).height, 180);
    for slot in [2, 12] {
        player_mut(&mut team, slot).height = 183;
    }
    let fits = [(180, 180), (185, 185)];
    assert_eq!(
        problems(&rules, &team),
        vec![
            error(Some(2), no_bracket(183, &fits, true)),
            error(Some(12), no_bracket(183, &fits, true)),
            quota("green", "mid", 4, 5),
            quota("green", "tall", 5, 6),
        ]
    );
}

/// Every Green quota met (the legal team's 23) and a 24th player, a copy
/// of the mid in slot 12, at 183 in slot 23: no bracket has room, so the
/// hint names every bracket with a quota, in the ruleset's order: manlet
/// (up to 175), mid (180), tall (185) and giant (194). Giga's quota is 0:
/// no player may take it, so the hint leaves it out.
#[test]
fn with_every_quota_met_the_height_hint_names_every_quota_bracket() {
    let rules = Rules::official();
    let mut team = legal_team();
    let mut extra = player(&team, 12).clone();
    assert!(team.players.iter().all(|p| p.slot != 23));
    extra.slot = 23;
    extra.height = 183;
    team.players.push(extra);
    assert_eq!(
        height_violations(&rules, &team),
        vec![error(
            Some(23),
            no_bracket(183, &[(0, 175), (180, 180), (185, 185), (194, 194)], false)
        )]
    );
}

/// The hint adds the player's height allowance to each range but the
/// manlet bracket's bottom. Autumn 26's goalkeepers at 189 take a 4 cm
/// allowance into tall (185), so no allowance ever leaves a player in no
/// bracket: the goalkeepers' allowance is lowered to 3 here, so both
/// keepers (slots 0 and 11, 189) read as 186. A manlet (slot 3, 175) at
/// 176 opens the manlet bracket too. The keepers' tall reads 185 + 3 and
/// the manlet range runs from the bottom to 175 + 3; slot 3, without an
/// allowance, reads the brackets as written.
#[test]
fn the_height_hint_adds_the_allowance() {
    let rules = Rules::from_data_block(&crate::rules::AUTUMN26.replace(
        "when: #{ positions: [\"GK\"], height_min: 189, height_max: 189 },\n           then: #{ height_allowance: 4 }",
        "when: #{ positions: [\"GK\"], height_min: 189, height_max: 189 },\n           then: #{ height_allowance: 3 }",
    ))
    .expect("the edited Autumn 26 loads");
    let mut team = legal_team();
    assert_eq!(player(&team, 0).height, 189);
    assert_eq!(player(&team, 11).height, 189);
    assert_eq!(player(&team, 3).height, 175);
    player_mut(&mut team, 3).height = 176;
    let keeper = no_bracket(189, &[(0, 178), (188, 188)], true);
    assert_eq!(
        height_violations(&rules, &team),
        vec![
            error(Some(0), keeper.clone()),
            error(Some(3), no_bracket(176, &[(0, 175), (185, 185)], true)),
            error(Some(11), keeper),
        ]
    );
}

/// Slot 1 (registered SS, rated A at AMF and SS) registered at CF, which
/// he is rated C at: the violation names his two A positions.
#[test]
fn registered_position_not_a_names_the_a_positions() {
    let (amf, ss, cf) = (position("AMF"), position("SS"), position("CF"));
    let team = edited(1, |p| {
        assert_eq!(p.position, Some(ss));
        assert_eq!(p.ratings[usize::from(cf.0)], 0);
        p.position = Some(cf);
    });
    assert_eq!(
        problems(&Rules::official(), &team),
        vec![error(
            Some(1),
            ViolationKind::RegisteredNotA {
                rated_a: vec![amf, ss]
            }
        )]
    );
}

/// The gold made a silver: no gold, three silvers. Read as silver he
/// also overpays a card: nine cards, three tricks and one COM style free
/// (the silver's one trade going to the style), so paid 5, and his
/// second A position takes the limit to 5 - 1.
#[test]
fn tier_counts_off() {
    let team = edited(2, |p| set_tier(p, 92, 8, 3));
    assert_eq!(
        problems(&Rules::official(), &team),
        vec![
            error(Some(2), ViolationKind::Cards { paid: 5, limit: 4 }),
            error(
                None,
                ViolationKind::TierCount {
                    tier: "gold".to_string(),
                    count: 0,
                    expected: 1
                }
            ),
            error(
                None,
                ViolationKind::TierCount {
                    tier: "silver".to_string(),
                    count: 3,
                    expected: 2
                }
            ),
        ]
    );
}

/// Slot 12, a mid player, shrunk to a manlet: one manlet over, one mid
/// short, in the brackets' order. (His weak foot 2/2 and three paid cards
/// stay within a manlet's larger allowances.)
#[test]
fn bracket_quotas_off() {
    assert_eq!(
        problems(&Rules::official(), &edited(12, |p| p.height = 175)),
        vec![
            error(
                None,
                ViolationKind::BracketQuota {
                    system: "green".to_string(),
                    bracket: "manlet".to_string(),
                    count: 7,
                    quota: 6
                }
            ),
            error(
                None,
                ViolationKind::BracketQuota {
                    system: "green".to_string(),
                    bracket: "mid".to_string(),
                    count: 4,
                    quota: 5
                }
            ),
        ]
    );
}

/// No captain set. Slot 10's Captaincy card is then paid, and paid over
/// his limit: nine cards less the two free tricks and the one COM style
/// the trade covers is six against a silver's 5.
#[test]
fn no_captain() {
    let mut team = legal_team();
    assert_eq!(team.captain, Some(10));
    team.captain = None;
    assert_eq!(
        problems(&Rules::official(), &team),
        vec![
            error(Some(10), ViolationKind::Cards { paid: 6, limit: 5 }),
            error(None, ViolationKind::Captain)
        ]
    );
}

/// Both goalkeepers off the roster: no goalkeeper, two regulars short,
/// two tall players short.
#[test]
fn no_goalkeeper() {
    let mut team = legal_team();
    let gk = position("GK");
    team.players.retain(|p| p.position != Some(gk));
    assert_eq!(team.players.len(), 21);
    assert_eq!(
        problems(&Rules::official(), &team),
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
                    quota: 6
                }
            ),
            error(None, ViolationKind::Goalkeepers { count: 0, min: 1 }),
        ]
    );
}

/// A player below every tier's expected rating reads as the last tier
/// whose condition holds, the regular, and is reported against it: one
/// `Stat` per ability PES 15 has (19) but the two `at_most` prowess
/// stats, which may sit below.
#[test]
fn a_player_below_every_tier_reads_as_the_last_fitting_tier() {
    let team = edited(3, |p| set_tier(p, 70, 4, 1));
    let found = problems(&Rules::official(), &team);
    assert_eq!(player(&team, 3).abilities.iter().flatten().count(), 19);
    assert_eq!(found.len(), 17, "{found:?}");
    assert!(
        found.iter().all(|v| matches!(
            v.kind,
            ViolationKind::Stat {
                value: 70,
                target: 77,
                at_most: false,
                ..
            }
        )),
        "{found:?}"
    );
}

// ---- The specials, one fixture each

/// Slot 12 pays three cards, his limit: Malicia on top costs nothing, and
/// with a paid card on top of both he pays four, not five.
#[test]
fn malicia_is_a_free_card() {
    let rules = Rules::official();
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

/// Captaincy on slot 12 is a paid card, and free once he is the
/// captain. Slot 10, the captain before, then pays his own: with the
/// silver's one trade covering one of his two COM styles that is paid 6
/// over limit 5.
#[test]
fn the_captains_captaincy_card_is_free() {
    let rules = Rules::official();
    assert_eq!(
        problems(&rules, &edited(12, |p| p.cards.push(card("captaincy")))),
        vec![error(Some(12), ViolationKind::Cards { paid: 4, limit: 3 })]
    );
    let mut team = edited(12, |p| p.cards.push(card("captaincy")));
    assert!(player(&team, 10).cards.contains(&card("captaincy")));
    team.captain = Some(12);
    assert_eq!(
        problems(&rules, &team),
        vec![error(Some(10), ViolationKind::Cards { paid: 6, limit: 5 })]
    );
}

/// Slot 9, a bronze, registered GK with his A moved there, at a
/// goalkeeper's tall height (189, so that rule alone breaks). A starter
/// fielded at SS in preset 1, where he is no longer rated A, he is also
/// the first-preset violation.
#[test]
fn medals_cannot_be_goalkeepers() {
    let gk = position("GK");
    let ss = position("SS");
    let team = edited(9, |p| {
        assert_eq!(p.position, Some(ss));
        assert_eq!(p.height, 185);
        p.position = Some(gk);
        p.height = 189;
        p.ratings[usize::from(gk.0)] = 2;
        p.ratings[usize::from(ss.0)] = 0;
    });
    assert_eq!(
        problems(&Rules::official(), &team),
        vec![
            error(
                Some(9),
                ViolationKind::FieldedNotA {
                    preset: 1,
                    position: ss,
                    rated_a: vec![gk]
                }
            ),
            error(
                Some(9),
                special("Medals cannot be goalkeepers", "Medals cannot play as GK")
            ),
        ]
    );
}

/// Slot 0, a goalkeeper at 189 cm counted as tall by the legal team: at
/// 185 still tall, but in the range the special forbids; at 184 also in
/// no bracket, one tall player short.
#[test]
fn goalkeepers_at_189_count_as_tall_and_181_to_188_is_forbidden() {
    let rules = Rules::official();
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
    assert_eq!(
        problems(&rules, &edited(0, |p| p.height = 184)),
        vec![
            error(Some(0), no_bracket(184, &[(185, 185)], true)),
            error(
                Some(0),
                special(
                    "Tall goalkeepers are 189",
                    "GKs in this bracket must be 189cm"
                )
            ),
            quota("green", "tall", 5, 6),
        ]
    );
}

/// A goalkeeper at 194 is an error ("Gold Medal Players and
/// Goalkeepers cannot be this height"), and the quotas (a tall player
/// short, a giant over, in the brackets' order).
#[test]
fn goalkeepers_below_giant_is_an_error() {
    assert_eq!(
        problems(&Rules::official(), &edited(0, |p| p.height = 194)),
        vec![
            error(
                Some(0),
                special(
                    "Goalkeepers below giant",
                    "GK heights cannot be 194cm or more"
                )
            ),
            quota("green", "tall", 5, 6),
            quota("green", "giant", 7, 6),
        ]
    );
}

/// The gold (slot 2, 185 cm) at 194: an error, and the quotas.
#[test]
fn gold_below_giant_is_an_error() {
    assert_eq!(
        problems(&Rules::official(), &edited(2, |p| p.height = 194)),
        vec![
            error(
                Some(2),
                special("Gold below giant", "Gold heights cannot be 194cm or more")
            ),
            quota("green", "tall", 5, 6),
            quota("green", "giant", 7, 6),
        ]
    );
}

/// `legal_team` under the Red system: no giant (the six 194 cm players
/// become one mid and five tall, the silver giant then rating 92), and
/// the six regular manlets at the Red manlet bonus, 82 (Red quotas 10
/// tall, 7 mid, 6 manlets).
fn red_legal_records() -> Records {
    let mut records = legal_records();
    for slot in [1, 8] {
        records.player_mut(slot).basic.height = 180;
    }
    for slot in [5, 20] {
        records.player_mut(slot).basic.height = 185;
    }
    // The two regulars registered at CB count as tall at 189, the
    // registered position alone asking (slots 14 and 17).
    for slot in [14, 17] {
        records.player_mut(slot).basic.height = 189;
    }
    set_tier_record(records.player_mut(5), 92, 8, 3);
    for slot in [3, 4, 6, 15, 19, 22] {
        assert_eq!(records.player_mut(slot).basic.height, 175);
        set_tier_record(records.player_mut(slot), 82, 4, 1);
    }
    records
}

/// `red_legal_records` viewed.
fn red_legal_team() -> TeamView {
    red_legal_records().view()
}

/// `red_legal_team` with `edit` applied to the player at `slot`.
fn red_edited(slot: u8, edit: impl FnOnce(&mut PlayerView)) -> TeamView {
    let mut team = red_legal_team();
    edit(player_mut(&mut team, slot));
    team
}

#[test]
fn the_red_legal_team_reports_nothing() {
    assert_eq!(problems(&Rules::official(), &red_legal_team()), vec![]);
}

/// The Red manlet bonus is optional ("non-medal players may receive a
/// +5 bonus ... Bronze Medal players may receive a +3 bonus ... Silver
/// Medal players may receive a +2 bonus"): a regular manlet rates 77
/// or 82, one at 79 (the bonus taken but not met) is off every rating
/// stat's raised target, a bronze manlet rates 86 or 89 and a silver
/// manlet 92 or 94. An untaken bonus is named in the suggestions.
#[test]
fn red_manlet_bonus() {
    let rules = Rules::official();
    // A regular manlet at 77 takes no bonus (the suggestion names it);
    // the legal Red team's manlets at 82 take it.
    let at_base = red_edited(3, |p| set_tier(p, 77, 4, 1));
    assert_eq!(problems(&rules, &at_base), vec![]);
    assert!(
        suggestions_for(&rules, &at_base, 3).contains(&ViolationKind::UnusedChoice {
            special: "Red manlet bonus".to_string(),
        }),
        "{:?}",
        suggestions_for(&rules, &at_base, 3)
    );
    assert_eq!(problems(&rules, &red_legal_team()), vec![]);
    // At 79 the bonus is taken (79 is over the base 77) and not met:
    // every rating stat is off the raised target (17 of PES 15's 19
    // abilities; the prowess pair may sit below).
    let found = problems(&rules, &red_edited(3, |p| set_tier(p, 79, 4, 1)));
    assert_eq!(found.len(), 17, "{found:?}");
    assert!(
        found.iter().all(|v| matches!(
            v.kind,
            ViolationKind::Stat {
                value: 79,
                target: 82,
                at_most: false,
                ..
            }
        )),
        "{found:?}"
    );
    // A bronze manlet at 86 and 89: slot 3 made a bronze, slot 9 a
    // regular, so the tier counts keep.
    let bronze_manlet = |rate: u8| {
        let mut team = red_legal_team();
        set_tier(player_mut(&mut team, 3), rate, 8, 3);
        let regular = player_mut(&mut team, 9);
        set_tier(regular, 77, 4, 1);
        regular.weak_foot_usage = 2;
        regular.weak_foot_accuracy = 2;
        // A regular's three paid cards: the COM style was the medals'
        // trade's, one more skill is over his limit.
        assert_eq!(regular.cards.pop(), Some(card("speeding_bullet")));
        assert_eq!(regular.cards.pop(), Some(card("outside_curler")));
        team
    };
    let at_base = bronze_manlet(86);
    assert_eq!(problems(&rules, &at_base), vec![]);
    assert!(
        suggestions_for(&rules, &at_base, 3).contains(&ViolationKind::UnusedChoice {
            special: "Red manlet bonus".to_string(),
        }),
        "{:?}",
        suggestions_for(&rules, &at_base, 3)
    );
    assert_eq!(problems(&rules, &bronze_manlet(89)), vec![]);
    // A silver manlet at 92 and 94: slot 3 a silver, slot 5 a bronze,
    // slot 9 a regular, so the tier counts keep.
    let silver_manlet = |rate: u8| {
        let mut team = red_legal_team();
        set_tier(player_mut(&mut team, 3), rate, 8, 3);
        set_tier(player_mut(&mut team, 5), 86, 8, 3);
        let regular = player_mut(&mut team, 9);
        set_tier(regular, 77, 4, 1);
        regular.weak_foot_usage = 2;
        regular.weak_foot_accuracy = 2;
        assert_eq!(regular.cards.pop(), Some(card("speeding_bullet")));
        assert_eq!(regular.cards.pop(), Some(card("outside_curler")));
        team
    };
    assert_eq!(problems(&rules, &silver_manlet(92)), vec![]);
    assert_eq!(problems(&rules, &silver_manlet(94)), vec![]);
    // The bonus has no gold amount (the wiki names regulars, bronzes and
    // silvers only), so a gold at 175 under Red is offered no bonus to
    // take.
    let gold_manlet = red_edited(2, |p| p.height = 175);
    assert!(
        !suggestions_for(&rules, &gold_manlet, 2).contains(&ViolationKind::UnusedChoice {
            special: "Red manlet bonus".to_string(),
        }),
        "{:?}",
        suggestions_for(&rules, &gold_manlet, 2)
    );
}

/// Under Red a non-medal player registered at CB is tall at 189, as the
/// wiki's "Center Back in both Registered and Playable Position ... can
/// go up to 189cm" reads: slot 14 (a substitute, so the registered
/// position is all the rule asks) is tall at 189 and at 185 like
/// anyone, and in no bracket at 187, one tall player short. A regular
/// registered at CMF at 189 under Red is in no bracket, and a CB at 189
/// under Green too. Slot 17, a starter at 189, keeps the allowance
/// fielded at RB in preset 1 (the registered position counts); the
/// fielded position is then the first-preset violation instead.
#[test]
fn red_cbs_at_189_count_as_tall() {
    let rules = Rules::official();
    let (cb, cmf, rb) = (position("CB"), position("CMF"), position("RB"));
    let red = red_legal_team();
    assert_eq!(player(&red, 14).position, Some(cb));
    assert_eq!(player(&red, 14).height, 189);
    assert!(!red.tactics.starting_eleven.contains(&14));
    assert_eq!(problems(&rules, &red), vec![]);
    assert_eq!(
        problems(&rules, &red_edited(14, |p| p.height = 185)),
        vec![]
    );
    assert_eq!(
        problems(&rules, &red_edited(14, |p| p.height = 187)),
        vec![
            error(Some(14), no_bracket(187, &[(185, 185)], true)),
            quota("red", "tall", 9, 10),
        ]
    );
    // A regular registered at CMF at 189 under Red: no allowance, no
    // bracket, one mid player short.
    assert_eq!(player(&red, 12).position, Some(cmf));
    assert_eq!(
        problems(&rules, &red_edited(12, |p| p.height = 189)),
        vec![
            error(Some(12), no_bracket(189, &[(180, 180)], true)),
            quota("red", "mid", 6, 7),
        ]
    );
    // A medal registered at CB at 189 under Red: the allowance is the
    // non-medals' (D3), so he is in no bracket and the tall quota one short.
    assert_eq!(player(&red, 20).position, Some(cb));
    assert_eq!(player(&red, 20).height, 185);
    assert_eq!(
        problems(&rules, &red_edited(20, |p| p.height = 189)),
        vec![
            error(Some(20), no_bracket(189, &[(185, 185)], true)),
            quota("red", "tall", 9, 10),
        ]
    );
    // A CB under Green: 189 is in no bracket.
    let mut green = legal_team();
    assert_eq!(player(&green, 8).position, Some(cb));
    assert_eq!(player(&green, 8).height, 194);
    player_mut(&mut green, 8).height = 189;
    assert_eq!(
        problems(&rules, &green),
        vec![
            error(Some(8), no_bracket(189, &[(194, 194)], true)),
            quota("green", "giant", 5, 6),
        ]
    );
    // The starter: registered CB is what counts, so fielded at RB in
    // preset 1 he is still tall at 189, and the fielded position is the
    // violation instead.
    let mut team = red_legal_team();
    assert_eq!(player(&team, 17).height, 189);
    assert_eq!(problems(&rules, &team), vec![]);
    let index = team
        .tactics
        .starting_eleven
        .iter()
        .position(|slot| *slot == 17)
        .expect("slot 17 starts");
    assert_eq!(
        team.tactics.presets[0].formations[0].players[index].position,
        cb.0
    );
    team.tactics.presets[0].formations[0].players[index].position = rb.0;
    assert_eq!(
        problems(&rules, &team),
        vec![error(
            Some(17),
            ViolationKind::FieldedNotA {
                preset: 1,
                position: rb,
                rated_a: vec![cb]
            }
        )]
    );
}

/// A manlet (slot 3, paid 3) gets one more card and one more A position,
/// and weak foot 4/4, which the legal team's manlets carry.
#[test]
fn manlets_get_a_card_an_a_position_and_four_four_weak_foot() {
    let rules = Rules::official();
    let manlet = player(&legal_team(), 3).clone();
    assert_eq!((manlet.height, manlet.cards.len()), (175, 3));
    assert_eq!((manlet.weak_foot_usage, manlet.weak_foot_accuracy), (4, 4));
    assert_eq!(
        problems(
            &rules,
            &edited(3, |p| p.cards.push(card("long_range_drive")))
        ),
        vec![]
    );
    let team = edited(3, |p| {
        p.cards.push(card("long_range_drive"));
        p.cards.push(card("knuckle_shot"));
    });
    assert_eq!(
        problems(&rules, &team),
        vec![error(Some(3), ViolationKind::Cards { paid: 5, limit: 4 })]
    );
    let cb = position("CB");
    assert_eq!(
        problems(&rules, &edited(3, |p| p.ratings[usize::from(cb.0)] = 2)),
        vec![]
    );
}

/// The legal team's silver giant (slot 5) rates 89; at 92 every ability
/// is off its lowered target, the prowess pair included (above it).
#[test]
fn green_silver_giants_rate_three_below() {
    let team = edited(5, |p| set_tier(p, 92, 8, 3));
    let found = problems(&Rules::official(), &team);
    assert_eq!(found.len(), 19, "{found:?}");
    assert!(
        found.iter().all(|v| matches!(
            v.kind,
            ViolationKind::Stat {
                value: 92,
                target: 89,
                ..
            }
        )),
        "{found:?}"
    );
}

/// `player_tier` reads under the team's height system: the Green silver
/// giant (slot 5) rates 89 — under no system his target would be 92 and
/// he would read as bronze — and reads `silver`
/// (`green_silver_giants_rate_three_below`); the goalkeepers' tier is
/// theirs.
#[test]
fn the_player_tier_is_read_under_the_teams_height_system() {
    let rules = Rules::official();
    let team = legal_team();
    // Premise: slot 5's highest rating stat is 89.
    let giant = player(&team, 5);
    let best = rules
        .ruleset()
        .stats
        .rating
        .iter()
        .filter_map(|stat| giant.abilities[usize::from(stat.0)])
        .max();
    assert_eq!(best, Some(89));
    for (slot, expected) in [(5, "silver"), (2, "gold"), (9, "bronze")] {
        assert_eq!(tier_of(&rules, &team, slot), Some(expected), "slot {slot}");
    }
    // A registered goalkeeper reads `goalkeeper`.
    let keeper = player(&team, 0);
    assert_eq!(keeper.position, Some(position("GK")));
    assert_eq!(tier_of(&rules, &team, 0), Some("goalkeeper"));
}

/// The medals' trades: each free COM style a medal may hold is instead a
/// free extra A position. Slot 9 (a bronze: marseille_turn and
/// cut_behind_and_turn the two free tricks, four paid skills, the COM
/// style speeding_bullet the one trade keeps free; rated A at SS alone)
/// pays his 4. An extra A beside the kept style spends the trade on the
/// style, so the A costs a card (paid 4 over limit 4 - 1); the style
/// dropped for the A is the same paid 4 inside the limit.
///
/// Slot 2 (a gold: four tricks of which three free, three paid skills,
/// the COM styles trickster and mazing_run; rated A at CF and AMF) has
/// two trades: both styles dropped, the third A position is free too
/// (paid 4 of 6); one kept, the second trade covers the extra A (paid 4
/// of 6); both kept, the extra A costs a card, the limit 6 - 1 = 5 his
/// paid 4 still meets until two more skills make it 6.
#[test]
fn medals_trade_a_free_com_style_for_a_free_a_position() {
    let rules = Rules::official();
    let (cf, ss) = (position("CF"), position("SS"));
    let bronze = player(&legal_team(), 9).clone();
    assert_eq!(bronze.cards.len(), 7);
    assert_eq!(bronze.cards[6], card("speeding_bullet"));
    assert_eq!(problems(&rules, &legal_team()), vec![]);
    assert_eq!(
        problems(&rules, &edited(9, |p| p.ratings[usize::from(cf.0)] = 2)),
        vec![error(Some(9), ViolationKind::Cards { paid: 4, limit: 3 })]
    );
    let team = edited(9, |p| {
        assert_eq!(p.cards.pop(), Some(card("speeding_bullet")));
        p.ratings[usize::from(cf.0)] = 2;
    });
    assert_eq!(problems(&rules, &team), vec![]);

    let gold = player(&legal_team(), 2).clone();
    assert_eq!(gold.cards.len(), 9);
    assert_eq!(
        (gold.cards[7], gold.cards[8]),
        (card("trickster"), card("mazing_run"))
    );
    // Both styles dropped, a second extra A: both trades spent on A
    // positions, paid 4 of 6.
    let team = edited(2, |p| {
        assert_eq!(p.cards.pop(), Some(card("mazing_run")));
        assert_eq!(p.cards.pop(), Some(card("trickster")));
        p.ratings[usize::from(ss.0)] = 2;
    });
    assert_eq!(problems(&rules, &team), vec![]);
    // Paid 4 is legal at a limit of 5 too: the limit itself shows both
    // trades taken, three A positions free and no card spent on them.
    let limits = rules
        .card_limits(&team, team.player(2).expect("filled"), "gold")
        .expect("limits");
    assert_eq!((limits.a_positions, limits.limit), (3, 6));
    // One style kept, one trade on the extra A: paid 4 of 6.
    let team = edited(2, |p| {
        assert_eq!(p.cards.pop(), Some(card("mazing_run")));
    });
    assert_eq!(problems(&rules, &team), vec![]);
    // Both kept, the extra A costs a card: paid 4 of 5 is legal, two
    // more paid skills are over.
    let team = edited(2, |p| {
        p.cards.push(card("long_range_drive"));
        p.cards.push(card("knuckle_shot"));
    });
    assert_eq!(
        problems(&rules, &team),
        vec![error(Some(2), ViolationKind::Cards { paid: 6, limit: 5 })]
    );
}

/// A bronze's free tricks are two: slot 9 at his limit of four paid
/// cards pays for a third trick, paid 5 over 4 (the COM style still the
/// trade's free one).
#[test]
fn a_bronze_pays_for_a_third_trick() {
    let team = edited(9, |p| p.cards.push(card("sombrero")));
    assert_eq!(
        problems(&Rules::official(), &team),
        vec![error(Some(9), ViolationKind::Cards { paid: 5, limit: 4 })]
    );
}

// ---- Suggestions

/// Each listed allowance a player does not use up is one suggestion:
/// slot 12 holds no trick (two free); the gold (slot 2) pays four of
/// five (the extra A costs a card against the tier's 6) and has weak
/// foot usage 2 of 4, then, with a COM style dropped, four of six and
/// injury resistance one short of 3 (no error: the target is
/// `at_most`). The one COM style still held is the one trade's, so no
/// `com` suggestion.
#[test]
fn suggestions_name_each_unused_allowance() {
    let rules = Rules::official();
    let team = legal_team();
    assert_eq!(
        suggestions_for(&rules, &team, 12),
        vec![unused("tricks", 0, 2)]
    );
    assert_eq!(
        suggestions_for(&rules, &team, 2),
        vec![unused("cards", 4, 5), unused("weak_foot_usage", 2, 4)]
    );
    let team = edited(2, |p| {
        assert_eq!(p.cards.pop(), Some(card("mazing_run")));
        p.injury_resistance = 2;
    });
    assert_eq!(problems(&rules, &team), vec![]);
    assert_eq!(
        suggestions_for(&rules, &team, 2),
        vec![
            unused("cards", 4, 6),
            unused("weak_foot_usage", 2, 4),
            unused("injury_resistance", 2, 3),
        ]
    );
}

// ---- tier_values, card_limits, apply_tier

/// For every tier of the official file, a player of that tier in the
/// legal team, his stats, form, injury resistance and weak foot broken,
/// is legal again after `apply_tier`: the silver giant under Green (89)
/// and a regular manlet under Red (77, the optional bonus untaken)
/// included.
#[test]
fn apply_tier_then_check_reports_nothing_for_every_official_tier() {
    let rules = Rules::official();
    let tiers: Vec<&str> = rules
        .ruleset()
        .tiers
        .iter()
        .map(|tier| tier.id.as_str())
        .collect();
    assert_eq!(tiers, ["gold", "silver", "bronze", "goalkeeper", "regular"]);
    // (tier, slot, under the Red team rather than the Green one)
    for (tier, slot, red) in [
        ("gold", 2, false),
        ("silver", 10, false),
        ("silver", 5, false),
        ("bronze", 9, false),
        ("goalkeeper", 0, false),
        ("regular", 12, false),
        ("regular", 3, true),
    ] {
        let mut records = if red {
            red_legal_records()
        } else {
            legal_records()
        };
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

/// The targets carry the specials: the Green silver giant rates 89, a
/// manlet gets weak foot 4/4 and, under Red, the 82 of the bonus he
/// takes; a regular who is no manlet gets 2/2, form 4, injury
/// resistance 1. An id no tier has is refused before the script runs.
#[test]
fn tier_values_carry_the_specials() {
    let rules = Rules::official();
    let team = legal_team();
    let values = |team: &TeamView, slot: u8, tier: &str| {
        rules
            .tier_values(team, team.player(slot).expect("filled"), tier)
            .expect("values")
    };
    let giant = values(&team, 5, "silver");
    assert_eq!(giant.stats.len(), 25);
    assert!(giant.stats.values().all(|v| *v == 89), "{giant:?}");
    assert_eq!((giant.form, giant.injury_resistance), (8, 3));
    assert_eq!(
        giant.weak_foot,
        WeakFoot {
            usage: 4,
            accuracy: 4
        }
    );
    let manlet = values(&team, 3, "regular");
    assert!(manlet.stats.values().all(|v| *v == 77), "{manlet:?}");
    assert_eq!(
        manlet.weak_foot,
        WeakFoot {
            usage: 4,
            accuracy: 4
        }
    );
    let red = red_legal_team();
    let manlet = values(&red, 3, "regular");
    assert!(manlet.stats.values().all(|v| *v == 82), "{manlet:?}");
    let regular = values(&team, 12, "regular");
    assert_eq!(
        regular.weak_foot,
        WeakFoot {
            usage: 2,
            accuracy: 2
        }
    );
    assert_eq!((regular.form, regular.injury_resistance), (4, 1));
    assert_eq!(
        rules
            .tier_values(&team, team.player(3).expect("filled"), "platinum")
            .expect_err("no such tier"),
        CheckError::UnknownTier {
            tier: "platinum".to_string()
        }
    );
}

/// The card economy per player and tier: a regular's limit and free
/// tricks, a manlet's extra card and A position, the silver captain's
/// free Captaincy and the trade covering one of his two COM styles, and
/// the same player read as gold (both trades covering both).
#[test]
fn card_limits_name_the_economy() {
    let rules = Rules::official();
    let team = legal_team();
    let limits = |slot: u8, tier: &str| {
        rules
            .card_limits(&team, team.player(slot).expect("filled"), tier)
            .expect("limits")
    };
    assert_eq!(
        limits(12, "regular"),
        CardLimits {
            paid: 3,
            limit: 3,
            free: BTreeMap::from([("tricks".to_string(), 2)]),
            free_cards: vec![card("malicia")],
            required_cards: vec![],
            a_positions: 1,
            max_a_positions: None,
        }
    );
    let manlet = limits(3, "regular");
    assert_eq!((manlet.limit, manlet.a_positions), (4, 2));
    let captain = limits(10, "silver");
    assert_eq!(captain.limit, 5);
    assert_eq!(
        captain.free,
        BTreeMap::from([("com".to_string(), 1), ("tricks".to_string(), 3)])
    );
    assert_eq!(captain.free_cards, vec![card("malicia"), card("captaincy")]);
    let as_gold = limits(10, "gold");
    assert_eq!((as_gold.limit, as_gold.a_positions), (6, 1));
    assert_eq!(as_gold.free.get("com"), Some(&2));

    // The whole economy per tier: the paid limit, the A positions, the
    // free counts (the exact map, so a group named nowhere in it shows)
    // and `max_a_positions` unset, plus `tier_values`' weak foot.
    // Slot 0 for the goalkeeper; the medals' `com` frees come from the
    // trades their held COM styles take — slot 2's two trades cover both
    // of his (his second A position costs a card off the limit: 6 - 1),
    // slot 5's one trade covers his one (his second A costs the limit
    // 5 - 1), slot 10's second COM style is paid (the silver's one trade
    // spent), slot 9's one trade covers his one. Where a member's trade
    // masks the file's value, the probe names it.
    assert_economy(
        &rules,
        &team,
        &[
            (2, "gold", 5, 1, &[("com", 2), ("tricks", 3)], (4, 4)),
            (5, "silver", 4, 1, &[("com", 1), ("tricks", 3)], (4, 4)),
            (10, "silver", 5, 1, &[("com", 1), ("tricks", 3)], (4, 4)),
            (9, "bronze", 4, 1, &[("com", 1), ("tricks", 2)], (4, 4)),
            (0, "goalkeeper", 2, 1, &[], (2, 2)),
            (12, "regular", 3, 1, &[("tricks", 2)], (2, 2)),
        ],
    );
    // A gold's second trade on an A position: slot 5, read as gold,
    // takes it — one COM style held spends the first, his second A
    // position the second.
    assert_eq!(limits(5, "gold").a_positions, 2);
}

/// `paid` counts the held cards no free group or free card covers:
/// slot 2, a gold holding nine cards — four tricks, three other skills
/// and two COM styles — under the gold's free three tricks and the two
/// COM styles his trades cover, so the four the frees leave are his paid
/// ones. The badge a card picker shows under a tier is `limit - paid`.
#[test]
fn card_limits_reports_the_paid_count() {
    let rules = Rules::official();
    let team = legal_team();
    let gold = player(&team, 2);
    assert_eq!(gold.cards.len(), 9, "premise: the gold holds nine cards");
    assert_eq!(
        gold.cards
            .iter()
            .filter(|held| {
                [
                    card("scissors_feint"),
                    card("marseille_turn"),
                    card("cut_behind_and_turn"),
                    card("scotch_move"),
                ]
                .contains(held)
            })
            .count(),
        4,
        "premise: four of the gold's nine cards are tricks"
    );
    let limits = rules.card_limits(&team, gold, "gold").expect("limits");
    assert_eq!(limits.free.get("com"), Some(&2));
    assert_eq!(limits.free.get("tricks"), Some(&3));
    assert_eq!(limits.paid, 4);
}

/// An allowance the file does not list is never suggested: a manlet
/// (slot 3) with one A of two, and a bronze (slot 9) whose trade the
/// held COM style takes, get no `a_positions` suggestion.
#[test]
fn nothing_is_suggested_for_an_allowance_not_listed() {
    let rules = Rules::official();
    let team = legal_team();
    let official = rules.ruleset();
    assert!(!official.suggestions.iter().any(|s| s == "a_positions"));
    assert_eq!(
        player(&team, 3).ratings.iter().filter(|r| **r == 2).count(),
        1
    );
    for slot in [3, 9] {
        assert!(
            suggestions_for(&rules, &team, slot)
                .iter()
                .all(|kind| matches!(
                    kind,
                    ViolationKind::Unused { allowance, .. }
                        if [
                            "cards",
                            "tricks",
                            "com",
                            "weak_foot_usage",
                            "weak_foot_accuracy",
                            "injury_resistance"
                        ]
                        .contains(&allowance.as_str())
                )),
            "{slot}"
        );
    }
}

/// A medal's free COM style can be traded for a free extra A position.
/// Neither exists until taken, so one who holds neither a COM style nor an
/// extra A position would hear nothing; instead he is told of the
/// trade he leaves: the bronze (slot 9) without his COM style, and the
/// gold (slot 2) keeping one COM style of two and no extra A position,
/// for his second trade.
#[test]
fn an_untaken_medal_trade_is_suggested() {
    let rules = Rules::official();
    let trade = |special: &str| ViolationKind::UnusedChoice {
        special: special.to_string(),
    };
    let bronze = edited(9, |p| {
        assert_eq!(p.cards.pop(), Some(card("speeding_bullet")));
    });
    assert!(
        suggestions_for(&rules, &bronze, 9).contains(&trade(
            "Medals trade a free COM style for a free A position"
        )),
        "{:?}",
        suggestions_for(&rules, &bronze, 9)
    );
    let (cf, amf) = (position("CF"), position("AMF"));
    let gold = edited(2, |p| {
        assert_eq!(p.cards.pop(), Some(card("mazing_run")));
        let extra = if p.position == Some(cf) { amf } else { cf };
        assert_eq!(
            p.ratings[usize::from(extra.0)],
            2,
            "premise: A at CF and AMF"
        );
        p.ratings[usize::from(extra.0)] = 0;
    });
    let suggested = suggestions_for(&rules, &gold, 2);
    assert!(
        suggested.contains(&trade(
            "Gold trades a second free COM style for a free A position"
        )),
        "{suggested:?}"
    );
    assert!(
        !suggested.contains(&trade(
            "Medals trade a free COM style for a free A position"
        )),
        "the held COM style takes the first trade: {suggested:?}"
    );
}
