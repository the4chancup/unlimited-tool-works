//! The grass-drop position rule's tests: the real formations the
//! rule's edges were measured on with the dragged players' overalls by
//! the editor's formula on their records, and synthetic
//! overalls for the pick's own parts (the free-candidate skip, the
//! tie, the keep and the missing row).

use super::*;
use crate::pitch::drop_rule::{dropped_position, free_move_14, spot_choices};

/// `dropped_position` at each `(x, y)` of `drops` answered by its
/// expected position, one assertion per drop named `case (x, y)`.
fn check(
    formation: &Formation,
    slot: usize,
    overalls: &[u8; 13],
    case: &str,
    drops: &[(u8, u8, &str)],
) {
    for &(x, y, expected) in drops {
        assert_eq!(
            dropped_position(formation, slot, x, y, Some(overalls)),
            position_byte(expected),
            "{case} ({x}, {y}): {expected} expected",
        );
    }
}

// Team 701's preset 1 formation 1 (PES 2017): the dragged slot and his
// thirteen overalls (GK to CF; every one the game showed equals them),
// then each drop and its position, on either side of each edge the
// game's drag panel was seen to cross.
#[test]
fn the_drop_rule_on_701s_preset_1_formation_1() {
    let formation = formation_701_1();
    check(
        &formation,
        2,
        &[62, 83, 73, 73, 73, 69, 68, 68, 71, 68, 68, 70, 70],
        "uv_up",
        &[
            (40, 15, "CB"),
            (40, 16, "DMF"),
            (40, 25, "DMF"),
            (40, 26, "AMF"),
            (40, 34, "AMF"),
            (40, 35, "SS"),
        ],
    );
    check(
        &formation,
        9,
        &[80, 90, 94, 94, 93, 89, 87, 87, 91, 88, 88, 89, 101],
        "bl_down",
        &[
            (38, 40, "CF"),
            (38, 39, "SS"),
            (38, 35, "SS"),
            (38, 34, "AMF"),
        ],
    );
    check(
        &formation,
        3,
        &[62, 70, 73, 85, 73, 69, 68, 68, 71, 68, 68, 70, 70],
        "rb_left",
        &[(84, 10, "RB"), (83, 10, "CB")],
    );
    check(
        &formation,
        4,
        &[62, 70, 85, 73, 73, 69, 68, 68, 71, 68, 68, 70, 70],
        "lb_right",
        &[(20, 10, "LB"), (21, 10, "CB")],
    );
    check(
        &formation,
        9,
        &[80, 90, 94, 94, 93, 89, 87, 87, 91, 88, 88, 89, 101],
        "bl_left",
        &[(21, 42, "CF"), (20, 42, "SS")],
    );
    check(
        &formation,
        5,
        &[73, 82, 85, 85, 96, 81, 79, 79, 83, 80, 80, 82, 82],
        "dmf_down",
        &[(52, 16, "DMF"), (52, 15, "CB")],
    );
    check(
        &formation,
        8,
        &[80, 87, 92, 92, 92, 89, 87, 87, 101, 87, 87, 94, 88],
        "amf_left",
        &[(21, 34, "AMF"), (20, 34, "CMF")],
    );
    check(
        &formation,
        5,
        &[73, 82, 85, 85, 96, 81, 79, 79, 83, 80, 80, 82, 82],
        "gk_box",
        &[(54, 2, "DMF")],
    );
}

// Team 718's preset 1 formations 1 and 3 (PES 2021): the dragged slot
// and his thirteen overalls (GK to CF; within 1 of those the game
// showed), then each drop and its position. Near a teammate's marker
// the game showed that teammate's position instead (the swap the drop
// on a marker makes), so `cmf_left`'s (31, 26) read LMF there.
#[test]
fn the_drop_rule_on_718s_preset_1_formations() {
    let party = PARTY_HARD;
    let zone: [u8; 13] = [73, 82, 85, 85, 85, 93, 79, 79, 83, 80, 80, 82, 82];
    let formation = formation_718_1();
    check(
        &formation,
        8,
        &party,
        "lmf_up",
        &[(20, 34, "LMF"), (20, 35, "LWF")],
    );
    check(
        &formation,
        8,
        &party,
        "lmf_right",
        &[(20, 26, "LMF"), (21, 26, "AMF"), (40, 26, "AMF")],
    );
    check(
        &formation,
        2,
        &zone,
        "cmf_left",
        &[(31, 26, "CMF"), (20, 26, "CMF"), (5, 26, "CMF")],
    );
    let formation = formation_718_3();
    check(
        &formation,
        8,
        &party,
        "lmf3_right",
        &[(21, 26, "AMF"), (31, 26, "AMF")],
    );
}

// Slot 2's CB dropped at (40, 20): the point's default is CMF and its
// candidate DMF, and his CB is neither — with both rated 80 the
// default stands; at 81 the candidate wins.
#[test]
fn the_default_wins_a_tie() {
    let formation = formation_701_1();
    let mut overalls = [70; 13];
    overalls[usize::from(position_byte("CMF"))] = 80;
    overalls[usize::from(position_byte("DMF"))] = 80;
    assert_eq!(
        dropped_position(&formation, 2, 40, 20, Some(&overalls)),
        position_byte("CMF"),
        "a tie keeps the band's default"
    );
    overalls[usize::from(position_byte("DMF"))] = 81;
    assert_eq!(
        dropped_position(&formation, 2, 40, 20, Some(&overalls)),
        position_byte("DMF"),
        "a higher-rated candidate wins"
    );
}

// Slot 9's CF dropped at (90, 40), where SS's and RWF's areas overlap:
// the default is SS and the candidate the free RWF, tried before SS,
// so a higher-rated RWF wins. Every real player there rates SS higher,
// so only synthetic overalls tell the candidates' order apart.
#[test]
fn a_free_rwf_is_tried_before_ss() {
    let formation = formation_701_1();
    let mut overalls = [70; 13];
    overalls[usize::from(position_byte("RWF"))] = 90;
    overalls[usize::from(position_byte("SS"))] = 80;
    assert_eq!(
        dropped_position(&formation, 9, 90, 40, Some(&overalls)),
        position_byte("RWF"),
        "RWF is the candidate, rated above the default SS"
    );
}

// A point inside his own position's area keeps it only when the point
// offers it: with slots 4 and 7 both LMF, slot 4 dropped at (10, 20),
// inside LMF's area, is offered CMF alone (the other LMF holds the wide
// position) and takes it.
#[test]
fn a_second_lmf_inside_lmfs_area_is_offered_cmf_alone() {
    let mut formation = formation_701_1();
    formation.players[4].position = position_byte("LMF");
    formation.players[7].position = position_byte("LMF");
    let mut overalls = [70; 13];
    overalls[usize::from(position_byte("LMF"))] = 90;
    assert_eq!(
        dropped_position(&formation, 4, 10, 20, Some(&overalls)),
        position_byte("CMF"),
        "the held LMF is not offered, so CMF is taken"
    );
}

// Slot 3's DMF dropped at (90, 20): RMF's area holds the point but
// slot 7 is the formation's RMF, so the candidate is none and the
// default CMF is taken — with slot 7 turned CMF, RMF is free and wins.
#[test]
fn a_wide_candidate_a_teammate_holds_is_skipped() {
    let mut formation = formation_718_1();
    let mut overalls = [70; 13];
    overalls[usize::from(position_byte("RMF"))] = 90;
    overalls[usize::from(position_byte("CMF"))] = 80;
    assert_eq!(
        dropped_position(&formation, 3, 90, 20, Some(&overalls)),
        position_byte("CMF"),
        "the held RMF is skipped: the default stands"
    );
    formation.players[7].position = position_byte("CMF");
    assert_eq!(
        dropped_position(&formation, 3, 90, 20, Some(&overalls)),
        position_byte("RMF"),
        "the RMF slot freed, the higher-rated candidate wins"
    );
}

// Dropped at (40, 20), where the default is CMF and the candidate DMF:
// slot 5's DMF keeps his DMF although CMF rates higher, and slot 6's
// CMF keeps his CMF although DMF rates higher.
#[test]
fn the_current_position_is_kept_where_it_is_the_default_or_the_candidate() {
    let formation = formation_701_1();
    let mut overalls = [70; 13];
    overalls[usize::from(position_byte("CMF"))] = 80;
    assert_eq!(
        dropped_position(&formation, 5, 40, 20, Some(&overalls)),
        position_byte("DMF"),
        "his own DMF is the point's candidate"
    );
    let mut overalls = [70; 13];
    overalls[usize::from(position_byte("DMF"))] = 80;
    assert_eq!(
        dropped_position(&formation, 6, 40, 20, Some(&overalls)),
        position_byte("CMF"),
        "his own CMF is the point's default"
    );
}

// A slot whose row reads no overalls keeps his position: no overall,
// no pick.
#[test]
fn no_overalls_keep_the_position() {
    let formation = formation_701_1();
    assert_eq!(
        dropped_position(&formation, 2, 40, 20, None),
        position_byte("CB"),
        "no overalls: his CB is kept"
    );
}

// The illegal formation rule reads "a position his spot offers" as one
// row test, the position's `AREAS_A` row holding the spot. Inside the
// movable range that is what the drop offers a starter while no wide
// position is taken (a formation of eleven GK bytes), position by
// position at every spot: were the tables or the drop's order to change
// so that the two part, the rule would warn about what a drop makes.
#[test]
fn inside_the_movable_range_a_spot_offers_the_positions_whose_first_row_holds_it() {
    let empty = Formation::default();
    for y in 4..=47 {
        for x in 5..=99 {
            let choices = spot_choices(&empty, 1, x, y);
            for position in 1..13u8 {
                assert_eq!(
                    pes_savefile::model::formation_rule::AREAS_A[usize::from(position)]
                        .contains(x, y),
                    choices.contains(&position),
                    "position {position} at ({x}, {y})"
                );
            }
        }
    }
}

// Points only `AREAS_B` holds: a position's area holds the point when
// either row does. Slot 6's CMF dropped at (104, 9), past `AREAS_A`'s
// right edge where CB's second row alone reaches, is offered CB alone.
// Slot 2's CB at (2, 20) — inside CMF's and LMF's second rows only —
// is offered the band's default CMF against the free candidate LMF,
// the pick on his overalls.
#[test]
fn a_point_only_the_second_table_holds_still_offers_its_positions() {
    let formation = formation_701_1();
    assert_eq!(
        dropped_position(&formation, 6, 104, 9, Some(&[70; 13])),
        position_byte("CB"),
        "past the edge only CB's second row holds the point"
    );
    let mut overalls = [70; 13];
    overalls[usize::from(position_byte("CMF"))] = 80;
    assert_eq!(
        dropped_position(&formation, 2, 2, 20, Some(&overalls)),
        position_byte("CMF"),
        "the band's default CMF over the free LMF"
    );
    overalls[usize::from(position_byte("LMF"))] = 90;
    assert_eq!(
        dropped_position(&formation, 2, 2, 20, Some(&overalls)),
        position_byte("LMF"),
        "the free candidate LMF wins on a higher overall"
    );
}

/// `labels`' position bytes, for comparing with `spot_choices`.
fn bytes(labels: &[&str]) -> Vec<u8> {
    labels.iter().map(|label| position_byte(label)).collect()
}

// The positions a spot offers on team 718's preset 1 formation 1, the
// band's default first: its CMF's spot (53, 26) offers CMF and the free
// candidate AMF; its middle CB's (52, 7) CB alone, the candidate CB
// being the default; its CF's (42, 41) SS and CF; a spot in the
// keeper's box (54, 1) nothing, GK being neither a default nor a
// candidate.
#[test]
fn a_spot_offers_the_bands_default_then_the_first_free_candidate() {
    let formation = formation_718_1();
    assert_eq!(
        spot_choices(&formation, 2, 53, 26),
        bytes(&["CMF", "AMF"]),
        "the CMF's spot"
    );
    assert_eq!(
        spot_choices(&formation, 6, 52, 7),
        bytes(&["CB"]),
        "the middle CB's spot"
    );
    assert_eq!(
        spot_choices(&formation, 10, 42, 41),
        bytes(&["SS", "CF"]),
        "the CF's spot"
    );
    assert_eq!(
        spot_choices(&formation, 6, 54, 1),
        bytes(&[]),
        "the keeper's box offers nothing"
    );
}

// A wide position a teammate holds is not offered: with slot 8 the
// formation's LMF, slot 2 made a second LMF at (10, 20), inside LMF's
// area, is offered CMF alone.
#[test]
fn a_second_lmf_inside_lmfs_area_is_offered_cmf_alone_by_the_spot() {
    let mut formation = formation_718_1();
    assert_eq!(
        formation.players[8].position,
        position_byte("LMF"),
        "premise: slot 8 is the LMF"
    );
    formation.players[2].position = position_byte("LMF");
    assert_eq!(
        spot_choices(&formation, 2, 10, 20),
        bytes(&["CMF"]),
        "the other LMF holds the wide position"
    );
}

/// `free_move_14` at each `(x, y)` of `drops` answered by its expected
/// position (`None` a refused drop), one assertion per drop named
/// `case (x, y)`.
fn check_14(
    formation: &Formation,
    slot: usize,
    overalls: Option<&[u8; 13]>,
    case: &str,
    drops: &[(u8, u8, Option<&str>)],
) {
    for &(x, y, expected) in drops {
        assert_eq!(
            free_move_14(formation, slot, x, y, overalls),
            expected.map(position_byte),
            "{case} ({x}, {y}): {expected:?} expected",
        );
    }
}

// PES 2014's free move on the game list's 4-4-2 with the all-77
// player's overalls: his own zone keeps him; elsewhere the first of
// CB, CMF, SS the point lies in, or the first free of the nine side
// and central positions when his overall there is at least the
// first's, the second on a tie. Slot 1 at (52, 45) is the "SS 72" run
// `tactics6` saw at the top edge.
#[test]
fn on_pes14_the_free_move_answers_the_role_at_the_point() {
    let formation = formation_442_14();
    check_14(
        &formation,
        5,
        Some(&ALL_77),
        "own zone",
        &[(50, 20, Some("CMF"))],
    );
    check_14(
        &formation,
        1,
        Some(&ALL_77),
        "second over",
        &[(52, 20, Some("DMF"))],
    );
    check_14(
        &formation,
        1,
        Some(&ALL_77),
        "first over",
        &[(52, 45, Some("SS"))],
    );
    check_14(
        &formation,
        9,
        Some(&ALL_77),
        "a tie",
        &[(12, 40, Some("LWF"))],
    );
}

// ...and refuses the drops the game refuses: a point outside every
// zone (width under 5, depth over 47, depth under 4 outside the
// keeper's widths), the keeper's zone, a dragged keeper, and a row
// that reads no overalls.
#[test]
fn on_pes14_the_free_move_refuses_what_the_game_refuses() {
    let formation = formation_442_14();
    check_14(
        &formation,
        1,
        Some(&ALL_77),
        "outside",
        &[(2, 20, None), (52, 48, None), (30, 2, None), (54, 1, None)],
    );
    check_14(&formation, 0, Some(&ALL_77), "keeper", &[(52, 20, None)]);
    check_14(&formation, 1, None, "no overalls", &[(52, 20, None)]);
}

// The pick between the two candidates, on the 3-2-3-2: at (10, 10) the
// first is CB (78) against the free LB (74), so CB; with LB rated 80
// the second wins. On the 4-4-2 the same point answers CB anyway: its
// LB is held by slot 4, so the nine give no second.
#[test]
fn on_pes14_the_second_candidate_wins_on_the_overall() {
    let formation = formation_3232_14();
    check_14(
        &formation,
        4,
        Some(&ALL_77),
        "first higher",
        &[(10, 10, Some("CB"))],
    );
    let mut overalls = ALL_77;
    overalls[usize::from(position_byte("LB"))] = 80;
    check_14(
        &formation,
        4,
        Some(&overalls),
        "second higher",
        &[(10, 10, Some("LB"))],
    );
    let formation = formation_442_14();
    check_14(
        &formation,
        5,
        Some(&overalls),
        "the LB held",
        &[(10, 10, Some("CB"))],
    );
}

// The line limits, the dragged player's own line not counted against
// its maximum: 4-2-3-1's only forward to midfield leaves no forward;
// 5-2-2-1's midfielder to defence makes six defenders where its RB to
// CB stays inside the defenders' line; 4-2-1-3 with its AMF made an SS
// leaves one midfielder — refused where the other versions' minimum
// of one would allow it.
#[test]
fn on_pes14_the_free_move_keeps_each_line_inside_its_limits() {
    let formation = formation_4231_14();
    check_14(
        &formation,
        10,
        Some(&ALL_77),
        "no forward left",
        &[(52, 25, None)],
    );
    let formation = formation_5221_14();
    check_14(
        &formation,
        6,
        Some(&ALL_77),
        "six defenders",
        &[(52, 10, None)],
    );
    check_14(
        &formation,
        4,
        Some(&ALL_77),
        "within his line",
        &[(52, 10, Some("CB"))],
    );
    let mut formation = formation_4213_14();
    formation.players[7] = FormationSlot {
        position: position_byte("SS"),
        x: 52,
        y: 40,
    };
    check_14(
        &formation,
        5,
        Some(&ALL_77),
        "one midfielder left",
        &[(52, 10, None)],
    );
}

// The free move as the game played it: the drops an in-game run made
// on the 4-4-2 and the 5-2-2-1 with players at 77 in every ability.
// Slot 6's CMF dropped past his zone's top edge is SS; slot 2's CB
// under a DMF candidate's higher overall; slot 9's CF inside CB's zone.
// On the 5-2-2-1: a move inside his own midfield line unchecked, a wide
// position a teammate holds skipped for the band's default, a tie won
// by the second candidate, and the two line-limit refusals (the lone
// forward, a sixth defender).
#[test]
fn on_pes14_the_free_move_gives_the_roles_the_game_gave() {
    let formation = formation_442_14();
    check_14(
        &formation,
        6,
        Some(&ALL_77),
        "past his zone's edge",
        &[(42, 35, Some("SS"))],
    );
    check_14(
        &formation,
        2,
        Some(&ALL_77),
        "the candidate wins",
        &[(49, 18, Some("DMF"))],
    );
    check_14(
        &formation,
        9,
        Some(&ALL_77),
        "the band's default",
        &[(37, 5, Some("CB"))],
    );
    let formation = formation_5221_14();
    check_14(
        &formation,
        6,
        Some(&ALL_77),
        "a held wide position",
        &[(14, 19, Some("CMF"))],
    );
    check_14(
        &formation,
        8,
        Some(&ALL_77),
        "a tie for the second",
        &[(99, 35, Some("RWF"))],
    );
    check_14(
        &formation,
        7,
        Some(&ALL_77),
        "no second in the band",
        &[(55, 38, Some("SS"))],
    );
    check_14(
        &formation,
        10,
        Some(&ALL_77),
        "within his line",
        &[(41, 38, Some("SS"))],
    );
    check_14(
        &formation,
        10,
        Some(&ALL_77),
        "the lone forward",
        &[(52, 25, None)],
    );
    check_14(
        &formation,
        6,
        Some(&ALL_77),
        "a sixth defender",
        &[(52, 5, None)],
    );
}
