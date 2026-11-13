//! The grass-drop position rule's tests: the real formations the
//! rule's edges were measured on with the dragged players' overalls by
//! the editor's formula on their records, and synthetic
//! overalls for the pick's own parts (the free-candidate skip, the
//! tie, the keep and the missing row).

use super::*;
use crate::pitch::drop_rule::{dropped_position, spot_choices};

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
