//! The position a starter takes at a spot of the pitch: the rule the
//! game's own formation editor applies at every step of a drag and in
//! its Select Position menu, read from PES 2016's executable (the last
//! version left unpacked) and checked against PES 2017's and PES 2021's
//! editors. It is no fixed map of pitch areas to positions: a point
//! offers up to two positions and the dragged player's own per-position
//! overalls pick between them, so the same spot gives different players
//! different positions. PES 2014's Game Plan plays by a rule of its
//! own, `free_move_14`, read from its executable: a drop it refuses
//! answers `None`, where the editor's rule would keep the position.

use std::ops::RangeInclusive;

use pes_savefile::model::formation_rule::{AREAS_A, AREAS_B, FormationLine, WIDE};
use pes_savefile::model::tactics::Formation;

/// The full-width bands' defaults, in the order the game tries them:
/// CB, CMF, SS.
const BAND_DEFAULTS: [u8; 3] = [1, 5, 11];
/// The positions tried for a candidate, in the game's order: LB, RB,
/// DMF, LMF, RMF, AMF, LWF, RWF, CF, then CB and SS.
const CANDIDATES: [u8; 11] = [2, 3, 4, 6, 7, 8, 9, 10, 12, 1, 11];
/// The positions PES 2014 tries for its second candidate, in the game's
/// order: the nine that are no band default.
const CANDIDATES_14: [u8; 9] = [2, 3, 4, 6, 7, 8, 9, 10, 12];
/// The positions the snapped point `(x, y)` in stored units offers
/// `slot` of `formation`, as the game's drag and its Select Position
/// menu offer them: the band's default (the first of `BAND_DEFAULTS`
/// whose area holds the point), then the first candidate of
/// `CANDIDATES` whose area holds it and which is free, each only when
/// found and the candidate left out when it is the default. At most
/// two; none in the keeper's box, GK being neither.
pub(super) fn spot_choices(formation: &Formation, slot: usize, x: u8, y: u8) -> Vec<u8> {
    let default = BAND_DEFAULTS
        .into_iter()
        .find(|&position| holds(position, x, y));
    let candidate = CANDIDATES
        .into_iter()
        .find(|&position| holds(position, x, y) && free(formation, slot, position))
        .filter(|&candidate| Some(candidate) != default);
    default.into_iter().chain(candidate).collect()
}

/// The position `slot` of `formation` takes at the snapped point `(x,
/// y)` in stored units: his stored position where the point offers it or
/// offers nothing (`spot_choices`); else the one position offered, or of
/// two the one `overalls` (his thirteen per-position ratings, indexed by
/// the position byte) rates higher, the band's default on a tie.
/// `overalls` `None` (an empty slot or a host that reads none) keeps his
/// position: no overall, no pick.
///
/// A drag rules every step from the stored position, which nothing
/// writes until the release, never from the position shown a step
/// earlier, as the game does: a CF dragged down to AMF and back up is a
/// CF again, where ruling step to step would leave him the SS he passed
/// through on the way back.
pub(super) fn dropped_position(
    formation: &Formation,
    slot: usize,
    x: u8,
    y: u8,
    overalls: Option<&[u8; 13]>,
) -> u8 {
    let current = formation.players[slot].position;
    let choices = spot_choices(formation, slot, x, y);
    if choices.contains(&current) {
        return current;
    }
    let Some(overalls) = overalls else {
        return current;
    };
    match *choices.as_slice() {
        [] => current,
        [only] => only,
        [default, candidate, ..] => {
            if overalls[usize::from(candidate)] > overalls[usize::from(default)] {
                candidate
            } else {
                default
            }
        }
    }
}

/// `position`'s area holds the point: either of its two table rows
/// does (bounds inclusive).
fn holds(position: u8, x: u8, y: u8) -> bool {
    let index = usize::from(position);
    AREAS_A[index].contains(x, y) || AREAS_B[index].contains(x, y)
}

/// `position` may be taken at the drop: a central one always, however
/// many slots share it; a wide one only when no slot of `formation`
/// but `slot` himself holds it.
fn free(formation: &Formation, slot: usize, position: u8) -> bool {
    !WIDE.contains(&position)
        || formation
            .players
            .iter()
            .enumerate()
            .all(|(index, other)| index == slot || other.position != position)
}

/// PES 2014's free move (the draft's `freeMove14`): the position `slot`
/// of `formation` takes when dropped on open grass at the snapped point
/// `(x, y)` in stored units, `None` where the game refuses the drop and
/// he goes back to his spot. Read from PES 2014's executable, it differs
/// from `dropped_position` in three ways: a point outside every zone is
/// refused rather than keeping his position, its one zone table is
/// `AREAS_A` (the game holds no second), and of the two candidates the
/// higher overall wins, the second on a tie.
///
/// `None` too for a dragged keeper, for a point in the keeper's zone
/// (what the game writes for the two is not read, so they are refused
/// rather than guessed), for a point that would take a line outside its
/// limits, and for `overalls` `None` (an empty slot or a host that reads
/// none): no overall, no pick.
pub(super) fn free_move_14(
    formation: &Formation,
    slot: usize,
    x: u8,
    y: u8,
    overalls: Option<&[u8; 13]>,
) -> Option<u8> {
    let current = formation.players[slot].position;
    if current == 0 || AREAS_A[0].contains(x, y) {
        return None;
    }
    let overalls = overalls?;
    if AREAS_A
        .get(usize::from(current))
        .is_some_and(|area| area.contains(x, y))
    {
        return Some(current);
    }
    // Every outfield zone lies inside one of the three band defaults'
    // rows, so a point none of them holds is outside every zone.
    let first = BAND_DEFAULTS
        .into_iter()
        .find(|&position| AREAS_A[usize::from(position)].contains(x, y))?;
    // The line check reads the first candidate, as the game does: the
    // move may not take his line below its minimum nor the candidate's
    // above its maximum. A line he already stands in is not counted
    // against its maximum, so the check only runs across lines.
    let to = FormationLine::of(first).expect("a band default is on a line");
    let from = FormationLine::of(current);
    if from != Some(to) {
        let count = |line: FormationLine| {
            formation
                .players
                .iter()
                .filter(|player| FormationLine::of(player.position) == Some(line))
                .count()
        };
        if from.is_some_and(|line| count(line) - 1 < *line_limits_14(line).start())
            || count(to) + 1 > *line_limits_14(to).end()
        {
            return None;
        }
    }
    let second = CANDIDATES_14.into_iter().find(|&position| {
        AREAS_A[usize::from(position)].contains(x, y) && free(formation, slot, position)
    });
    match second {
        Some(second) if overalls[usize::from(second)] >= overalls[usize::from(first)] => {
            Some(second)
        }
        _ => Some(first),
    }
}

/// The starters a line may hold on PES 2014, bounds included: its
/// midfield minimum is one above the other versions', which
/// `FormationLine::limits` holds.
fn line_limits_14(line: FormationLine) -> RangeInclusive<usize> {
    match line {
        FormationLine::Defenders => 2..=5,
        FormationLine::Midfielders => 2..=6,
        FormationLine::Forwards => 1..=5,
    }
}
