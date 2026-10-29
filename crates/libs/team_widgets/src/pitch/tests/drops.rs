//! The drag-and-drop tests: marker and row drops, the exchanges and
//! swaps, and the grass, own-chip and off-pitch releases.

use pes_savefile::model::instruction::Instruction;
use pes_savefile::schema::instruction;
use pes_version::PesVersion;

use super::paint::badge_rects;
use super::*;
use crate::test_support::{Headless, fixture_rows, fixture_tactics, roster_rows};

// The draft's "Move on pitch": a marker released on the grass writes
// the snapped drop point to its slot's x and y, and only there. The
// dragged starter is off his own roster slot, so a write keyed by his
// roster slot moves another slot's spot.
#[test]
fn a_marker_dragged_to_grass_writes_the_snapped_point() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let index = 4;
    tactics.starting_eleven.swap(index, 9);
    assert_ne!(
        usize::from(tactics.starting_eleven[index]),
        index,
        "premise: the dragged starter is off his own roster slot"
    );
    let before = tactics.clone();
    let formation = tactics.presets[0].formations[0].clone();
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &formation.players);
    let from = marker_point(&nodes, &formation.players, index);
    // An off-grid drop, as every real one is: the release lands
    // `+vec2(1.6, -3.5)` past a free grid point — 0.57 and 0.52 of a
    // unit, so the written unit is the rounded (x + 1, y + 1), which a
    // `.floor()` snap would read (x, y) — plus the press's offset from
    // his badge centre, since he stands at the pointer less it. The
    // release point itself stays on the canvas and inside no other
    // marker (another's would claim the drop for a swap).
    let slot = formation.players[index];
    let offset = press_offset(canvas, slot, from);
    let (x, y, onto) = (0..=GRID_X)
        .flat_map(|x| (0..=GRID_Y).map(move |y| (x, y)))
        .map(|(x, y)| {
            (
                x,
                y,
                badge_centre(canvas.min, FormationSlot { position: 1, x, y })
                    + egui::vec2(1.6, -3.5)
                    + offset,
            )
        })
        .find(|(x, y, point)| {
            *x < GRID_X
                && *y < GRID_Y
                && canvas.contains(*point)
                && (*x + 1 != slot.x || *y + 1 != slot.y)
                && centres.iter().enumerate().all(|(other, centre)| {
                    other == index || !marker_rect(*centre, canvas).contains(*point)
                })
        })
        .map(|(x, y, point)| (x + 1, y + 1, point))
        .expect("an off-grid point free of other markers");

    let changed = window.drag_pitch(
        from,
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    let mut expect = before;
    expect.presets[0].formations[0].players[index].x = x;
    expect.presets[0].formations[0].players[index].y = y;
    assert_eq!(tactics, expect, "only the dragged slot's x and y moved");

    // A second drop below the half-mark on both axes: `+vec2(0.85,
    // -2.04)` past a free grid point is 0.30 of a unit each way, so the
    // written unit is that point's own (x, y), which a `.ceil()` snap
    // would read (x + 1, y + 1).
    let before = tactics.clone();
    let formation = tactics.presets[0].formations[0].clone();
    let centres = centres_of(canvas, &formation.players);
    let from = marker_point(&nodes, &formation.players, index);
    let slot = formation.players[index];
    let offset = press_offset(canvas, slot, from);
    let (x, y, onto) = (0..GRID_X)
        .flat_map(|x| (0..GRID_Y).map(move |y| (x, y)))
        .map(|(x, y)| {
            (
                x,
                y,
                badge_centre(canvas.min, FormationSlot { position: 1, x, y })
                    + egui::vec2(0.85, -2.04)
                    + offset,
            )
        })
        .find(|(x, y, point)| {
            canvas.contains(*point)
                && (*x != slot.x || *y != slot.y)
                && centres
                    .iter()
                    .all(|centre| !marker_rect(*centre, canvas).contains(*point))
        })
        .expect("an off-grid point free of markers");

    // The pointer rests on his new spot a frame first: egui hit-tests a
    // press against the last frame's widgets, and the drop's frame
    // registered his marker at the spot he left.
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        vec![egui::Event::PointerMoved(from)],
    );
    let changed = window.drag_pitch(
        from,
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the second drop changed the record");
    let mut expect = before;
    expect.presets[0].formations[0].players[index].x = x;
    expect.presets[0].formations[0].players[index].y = y;
    assert_eq!(tactics, expect, "the point below the half-mark rounds down");

    // A third drop on preset 2's In possession: the snapped point lands
    // in the shown formation of the shown preset, and Kick-off, where
    // the first two drops wrote, stays.
    view.preset = 1;
    view.formation = 1;
    // Fluid formation on, so the preset's In-possession can be shown.
    tactics.presets[1].style.fluid = Some(true);
    let before = tactics.clone();
    let formation = tactics.presets[1].formations[1].clone();
    let slot = formation.players[index];
    let kick_off = tactics.presets[0].formations[0].players[index];
    assert_ne!(
        (slot.x, slot.y),
        (kick_off.x, kick_off.y),
        "premise: the dragged slot's spot differs from Kick-off's"
    );
    let centres = centres_of(canvas, &formation.players);
    let from = marker_point(&nodes, &formation.players, index);
    let offset = press_offset(canvas, slot, from);
    let (x, y, onto) = (0..GRID_X)
        .flat_map(|x| (0..GRID_Y).map(move |y| (x, y)))
        .map(|(x, y)| {
            (
                x,
                y,
                badge_centre(canvas.min, FormationSlot { position: 1, x, y })
                    + egui::vec2(0.85, -2.04)
                    + offset,
            )
        })
        .find(|(x, y, point)| {
            canvas.contains(*point)
                && (*x != slot.x || *y != slot.y)
                && centres
                    .iter()
                    .all(|centre| !marker_rect(*centre, canvas).contains(*point))
        })
        .expect("an off-grid point free of markers");

    // The pointer rests on his marker in the shown formation a frame
    // first: egui hit-tests a press against the last frame's widgets,
    // which drew Kick-off.
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        vec![egui::Event::PointerMoved(from)],
    );
    let changed = window.drag_pitch(
        from,
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the third drop changed the record");
    let mut expect = before;
    expect.presets[1].formations[1].players[index].x = x;
    expect.presets[1].formations[1].players[index].y = y;
    assert_eq!(
        tactics, expect,
        "only preset 2's In possession slot moved: Kick-off and the eleven stay"
    );
}

// The grass drop also writes the slot's position, by the game's own
// rule: the band's default or the first free candidate the snapped
// point falls in, whichever his per-position overall rates higher.
// Slot 2's CB released at stored (40, 20) — its default CMF, its
// candidate DMF — takes the higher-rated DMF; released at (40, 12),
// inside CB's band, he is CB again. A non-identity eleven: the
// overalls read are his roster slot's, and the row slot 2 would name
// by its index rates CMF higher, so reading it would give CMF.
#[test]
fn a_marker_released_on_the_grass_takes_the_rules_position() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let mut roster = roster_rows(&rows);
    tactics.presets[0].formations[0] = formation_701_1();
    tactics.starting_eleven.swap(2, 5);
    let his_row = usize::from(tactics.starting_eleven[2]);
    assert_ne!(his_row, 2, "premise: slot 2's player is not roster slot 2");
    roster[his_row].overalls = Some([62, 83, 73, 73, 73, 69, 68, 68, 71, 68, 68, 70, 70]);
    let mut decoy = [70; 13];
    decoy[usize::from(position_byte("CMF"))] = 90;
    roster[2].overalls = Some(decoy);
    let players = tactics.presets[0].formations[0].players;
    assert_eq!(
        (players[2].position, players[2].x, players[2].y),
        (position_byte("CB"), 40, 8),
        "premise: slot 2's CB at (40, 8)"
    );
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let from = marker_point(&nodes, &players, 2);
    let onto = badge_centre(
        canvas.min,
        FormationSlot {
            position: 1,
            x: 40,
            y: 20,
        },
    ) + press_offset(canvas, players[2], from);
    assert!(
        canvas.contains(onto)
            && centres.iter().enumerate().all(|(other, centre)| {
                other == 2 || !marker_rect(*centre, canvas).contains(onto)
            }),
        "premise: the release lands on free grass"
    );

    let changed = window.drag_pitch(
        from,
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    let moved = tactics.presets[0].formations[0].players[2];
    assert_eq!(
        (moved.x, moved.y),
        (40, 20),
        "his spot is the snapped point"
    );
    assert_eq!(
        moved.position,
        position_byte("DMF"),
        "his overall rated DMF over the default CMF"
    );

    // A frame first: a press hit-tests against the markers the last
    // frame registered, still at his old spot.
    let players = tactics.presets[0].formations[0].players;
    let centres = centres_of(canvas, &players);
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let from = marker_point(&nodes, &players, 2);
    let onto = badge_centre(
        canvas.min,
        FormationSlot {
            position: 1,
            x: 40,
            y: 12,
        },
    ) + press_offset(canvas, players[2], from);
    assert!(
        centres
            .iter()
            .enumerate()
            .all(|(other, centre)| { other == 2 || !marker_rect(*centre, canvas).contains(onto) }),
        "premise: the release lands on free grass"
    );

    let changed = window.drag_pitch(
        from,
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    let moved = tactics.presets[0].formations[0].players[2];
    assert_eq!(
        (moved.x, moved.y),
        (40, 12),
        "his spot is the snapped point"
    );
    assert_eq!(
        moved.position,
        position_byte("CB"),
        "the point is in CB's band: he is CB again"
    );
}

// The same drop with no overalls on the dragged starter's roster row
// keeps his position: no overall, no pick. His spot still writes.
#[test]
fn a_marker_released_on_the_grass_without_overalls_keeps_his_position() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    tactics.presets[0].formations[0] = formation_701_1();
    let players = tactics.presets[0].formations[0].players;
    assert_eq!(
        (players[2].position, players[2].x, players[2].y),
        (position_byte("CB"), 40, 8),
        "premise: slot 2's CB at (40, 8)"
    );
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let from = marker_point(&nodes, &players, 2);
    let onto = badge_centre(
        canvas.min,
        FormationSlot {
            position: 1,
            x: 40,
            y: 20,
        },
    ) + press_offset(canvas, players[2], from);
    assert!(
        centres
            .iter()
            .enumerate()
            .all(|(other, centre)| { other == 2 || !marker_rect(*centre, canvas).contains(onto) }),
        "premise: the release lands on free grass"
    );

    let changed = window.drag_pitch(
        from,
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "his spot moved: the record changed");
    let moved = tactics.presets[0].formations[0].players[2];
    assert_eq!(
        (moved.x, moved.y),
        (40, 20),
        "his spot is the snapped point"
    );
    assert_eq!(
        moved.position,
        position_byte("CB"),
        "no overalls: his CB is kept"
    );
}

// A pitch player dropped on another swaps the two slots in the shown
// formation, as the game's drag does in Team Sheet/Edit Position: the
// two slots of preset 1's Kick-off exchange position, x and y, each
// player staying in his slot; the starting eleven, the preset's other
// two formations and presets 2 and 3 stay.
#[test]
fn a_marker_dropped_on_another_marker_swaps_the_two_slots_in_the_shown_formation() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let before = tactics.clone();
    let formation = before.presets[0].formations[0].clone();
    let (from, onto) = (3, 6);
    assert_ne!(
        formation.players[from], formation.players[onto],
        "premise: two different slots"
    );
    for (preset, formation) in [(0, 1), (0, 2), (1, 0), (2, 0)] {
        let players = before.presets[preset].formations[formation].players;
        assert_ne!(
            players[from], players[onto],
            "premise: a swap there too would show (preset {preset}, formation {formation})"
        );
    }
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    // `over` is inside `onto`'s box, free of every marker drawn after it
    // (`marker_point`), so the topmost-marker drop resolves to `onto`.
    let at = marker_point(&nodes, &formation.players, from);
    let over = marker_point(&nodes, &formation.players, onto);

    let changed = window.drag_pitch(
        at,
        over,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    let mut expect = before.clone();
    expect.presets[0].formations[0].players.swap(from, onto);
    assert_eq!(
        tactics, expect,
        "only Kick-off's two slots exchanged: the eleven, the other formations and presets stay"
    );
}

// The swap writes the shown formation of the shown preset, not a fixed
// one: on preset 2's In-possession the two slots exchange there alone,
// preset 1's Kick-off (where a swap would show too) and every other
// formation unchanged.
#[test]
fn a_marker_dropped_on_another_swaps_in_the_shown_preset_and_formation() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView {
        preset: 1,
        formation: 1,
        ..TacticsView::default()
    };
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    // Fluid formation on, so the preset's In-possession can be shown.
    tactics.presets[1].style.fluid = Some(true);
    let before = tactics.clone();
    let formation = before.presets[1].formations[1].clone();
    let (from, onto) = (3, 6);
    assert_ne!(
        formation.players[from], formation.players[onto],
        "premise: two different slots in the shown formation"
    );
    let kick_off = before.presets[0].formations[0].players;
    assert_ne!(
        kick_off[from], kick_off[onto],
        "premise: a swap on preset 1's Kick-off would show"
    );
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let at = marker_point(&nodes, &formation.players, from);
    let over = marker_point(&nodes, &formation.players, onto);

    let changed = window.drag_pitch(
        at,
        over,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    let mut expect = before;
    expect.presets[1].formations[1].players.swap(from, onto);
    assert_eq!(
        tactics, expect,
        "only preset 2's In-possession two slots exchanged: the eleven and every other formation stay"
    );
}

// The keeper is never a swap target, his slot's position and spot being
// fixed: a marker held over him draws no swap mark, and released there
// moves to the snapped point as on the grass, the starting eleven and
// his slot unchanged.
#[test]
fn a_marker_released_on_the_keeper_moves_to_the_snapped_point() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let before = tactics.clone();
    let players = before.presets[0].formations[0].players;
    let keeper = players
        .iter()
        .position(|slot| slot.position == 0)
        .expect("the fixture has a keeper slot");
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let over = marker_point(&nodes, &players, keeper);
    // The dragged starter: an outfield one whose own box is off the hold,
    // the hold in no outfield marker's box but the keeper's.
    let index = (0..11)
        .find(|&index| {
            let from = marker_point(&nodes, &players, index);
            players[index].position != 0
                && !marker_rect(centres[index], canvas).contains(over)
                && snap(
                    over - press_offset(canvas, players[index], from),
                    canvas.min,
                ) != (players[index].x, players[index].y)
        })
        .expect("an outfield starter off the keeper's marker");
    assert!(
        (0..11).all(|other| {
            other == keeper || !marker_rect(centres[other], canvas).contains(over)
        }),
        "premise: the hold is on the keeper's marker alone"
    );
    let from = marker_point(&nodes, &players, index);

    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(from, true),
    );
    let (_, shapes) = window.frame(vec![egui::Event::PointerMoved(over)], |ui| {
        pitch(
            ui,
            "pitch",
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
        );
    });
    assert!(
        !shapes.iter().any(|clipped| matches!(
            &clipped.shape,
            egui::Shape::LineSegment { stroke, .. } if stroke.color == Palette::DARK.selected_corners
        )),
        "no swap mark over the keeper"
    );
    let (_, changed) = window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(over, false),
    );
    assert!(changed, "the drop changed the record");
    let mut expect = before.clone();
    let (x, y) = snap(
        over - press_offset(canvas, players[index], from),
        canvas.min,
    );
    expect.presets[0].formations[0].players[index].x = x;
    expect.presets[0].formations[0].players[index].y = y;
    assert_eq!(
        tactics, expect,
        "he moved to the snapped point; the eleven and the keeper's slot stay"
    );
}

/// A marker `index` of preset 1's Kick-off whose box's left edge has
/// free grass `reach` points out: a press `inset` points inside that
/// edge, level with the box's centre, no marker drawn after his covering
/// it, and its release `reach` points left on no marker. `None` when no
/// outfield marker has one.
fn edge_press(
    canvas: egui::Rect,
    players: &[FormationSlot; 11],
    inset: f32,
    reach: f32,
) -> Option<(usize, egui::Pos2, egui::Pos2)> {
    let centres = centres_of(canvas, players);
    let order = draw_order(&centres);
    (0..11).find_map(|index| {
        let rect = marker_rect(centres[index], canvas);
        let press = egui::pos2(rect.left() + inset, rect.center().y);
        let release = press - egui::vec2(reach, 0.0);
        let after = draw_position(canvas, players, index);
        (players[index].position != 0
            && canvas.contains(release)
            && order[after + 1..]
                .iter()
                .all(|&other| !marker_rect(centres[other], canvas).contains(press))
            && centres
                .iter()
                .all(|centre| !marker_rect(*centre, canvas).contains(release)))
        .then_some((index, press, release))
    })
}

// A release egui reports as the pressed marker's click (the pointer
// moved no farther than its click distance, 6) is a click, never a drop.
// egui marks the marker dragged once the pointer is past its hit radius
// (5) outside him, so a press 0.25 inside his edge
// released 5.5 off on the grass, a 5.75 move, is both his click and a
// drop: it opens his popover and writes nothing.
#[test]
fn a_release_within_the_click_distance_is_a_click_not_a_drop() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let before = tactics.clone();
    let players = before.presets[0].formations[0].players;
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let (index, press, release) =
        edge_press(canvas, &players, 0.25, 5.75).expect("a marker with grass at its left edge");
    assert_ne!(
        snap(release, canvas.min),
        (players[index].x, players[index].y),
        "premise: a drop there would move him"
    );

    let changed = window.drag_pitch(
        press,
        release,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(!changed, "a click writes nothing");
    assert_eq!(tactics, before, "his slot's x and y stay");
    assert_eq!(
        view.popover.map(|open| open.slot),
        Some(index),
        "the release is his click: the popover opens"
    );
}

// The click guard covers the swap too. Kick-off's slot 1 sits a stored
// unit left of and 8 below slot 5, so his box, drawn after 5's, overlaps
// its lower part and reaches 2.8 points past its left edge: a press on 5
// just above 1's top and a release 2.8 points away inside 1's box
// (outside 5's, so egui marks 5 dragged) is 5's click, never a swap with
// 1.
#[test]
fn a_release_within_the_click_distance_over_a_neighbour_is_a_click() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let before = tactics.clone();
    let players = before.presets[0].formations[0].players;
    let (index, neighbour) = (5, 1);
    assert_ne!(
        players[index], players[neighbour],
        "premise: a swap of the two would show"
    );
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let own = marker_rect(centres[index], canvas);
    let other = marker_rect(centres[neighbour], canvas);
    let press = egui::pos2(own.left() + 1.0, other.top() - 1.0);
    let release = egui::pos2(own.left() - 1.0, other.top() + 1.0);
    let under = |point: egui::Pos2| -> Vec<usize> {
        (0..11)
            .filter(|&m| marker_rect(centres[m], canvas).contains(point))
            .collect()
    };
    assert_eq!(
        under(press),
        vec![index],
        "premise: the press is on his box alone"
    );
    assert_eq!(
        under(release),
        vec![neighbour],
        "premise: the release is on the neighbour's box alone"
    );
    assert!(
        (release - press).length() <= 4.0,
        "premise: the release is within the click distance"
    );

    let changed = window.drag_pitch(
        press,
        release,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(!changed, "a click writes nothing");
    assert_eq!(tactics, before, "no swap with the neighbour");
    assert_eq!(
        view.popover.map(|open| open.slot),
        Some(index),
        "the release is his click: the popover opens"
    );
}

// The same finding's other half: past the click distance the release is
// the drop, the snapped point written and no popover opened.
#[test]
fn a_release_past_the_click_distance_is_a_drop_not_a_click() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let before = tactics.clone();
    let players = before.presets[0].formations[0].players;
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let (index, press, release) =
        edge_press(canvas, &players, 1.0, 10.0).expect("a marker with grass at its left edge");
    let (x, y) = snap(
        release - press_offset(canvas, players[index], press),
        canvas.min,
    );
    assert_ne!(
        (x, y),
        (players[index].x, players[index].y),
        "premise: the drop moves him"
    );

    let changed = window.drag_pitch(
        press,
        release,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    let mut expect = before.clone();
    expect.presets[0].formations[0].players[index].x = x;
    expect.presets[0].formations[0].players[index].y = y;
    assert_eq!(tactics, expect, "the snapped point written");
    assert_eq!(view.popover, None, "no popover opened");
}

// The same decision's other half: a marker released off the pitch
// stays where he was.
#[test]
fn a_marker_released_off_the_pitch_changes_nothing() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let before = tactics.clone();
    let formation = before.presets[0].formations[0].clone();
    let index = 3;
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let from = marker_point(&nodes, &formation.players, index);
    // Right of the canvas but inside the window.
    let onto = egui::pos2(canvas.max.x + 20.0, canvas.center().y);
    assert!(!canvas.contains(onto), "premise: a release off the pitch");
    assert!(onto.x < 600.0, "premise: still inside the window");

    let changed = window.drag_pitch(
        from,
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(!changed, "a release off the pitch writes nothing");
    assert_eq!(tactics, before, "and nothing moved");
}

// egui reports a release to the closest widget within its 5-point
// search radius, so a marker released 3 points off the pitch
// beside an edge marker's box is still off the pitch: no swap with that
// marker, no move to the snapped edge point.
#[test]
fn a_marker_released_off_the_pitch_beside_an_edge_marker_stays() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    // `edge` moved to the grid's right end: his box, clamped inside the
    // canvas, meets its right edge.
    let (edge, index) = (6, 3);
    tactics.presets[0].formations[0].players[edge].x = GRID_X;
    let before = tactics.clone();
    let players = before.presets[0].formations[0].players;
    assert!(
        players[edge].position != 0 && players[index].position != 0,
        "premise: two outfield slots"
    );
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let edge_box = marker_rect(centres_of(canvas, &players)[edge], canvas);
    assert!(
        (edge_box.right() - canvas.right()).abs() < 0.01,
        "premise: his box meets the canvas' right edge"
    );
    let onto = egui::pos2(canvas.right() + 3.0, edge_box.center().y);
    assert!(!canvas.contains(onto), "premise: a release off the pitch");
    assert!(
        edge_box.distance_to_pos(onto) < 5.0,
        "premise: within egui's search radius of his box"
    );
    let from = marker_point(&nodes, &players, index);

    let changed = window.drag_pitch(
        from,
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(!changed, "a release off the pitch writes nothing");
    assert_eq!(tactics, before, "and nothing moved");
}

// An eleven row released on the grass is no drop (the draft's canvas
// drop returns when it hits no marker): the grass move is
// `LineupDrag::Marker`'s alone.
#[test]
fn an_eleven_row_released_on_the_grass_changes_nothing() {
    let mut window = Headless::sized(600.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let before = tactics.clone();
    let formation = before.presets[0].formations[0].clone();
    let (nodes, _) =
        window.pitch_and_lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let onto = grass_point(canvas, &formation.players, egui::Rect::NOTHING);

    let changed = window.drag_pitch_and_lineup(
        row_centre(&nodes, 3),
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(!changed, "an eleven row on the grass is no drop");
    assert_eq!(tactics, before, "and nothing moved");
}

// A row released on the grass 3 points beside a marker's box, within
// egui's 5-point search radius, is still on the grass: a bench row swaps
// nobody in, an eleven row exchanges nobody.
#[test]
fn a_row_released_on_the_grass_beside_a_marker_does_nothing() {
    let mut window = Headless::sized(600.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let before = tactics.clone();
    let players = before.presets[0].formations[0].players;
    let (nodes, _) =
        window.pitch_and_lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    // 3 points out from the middle of one of a marker's sides.
    let (index, onto) = (0..11)
        .flat_map(|index| {
            let rect = marker_rect(centres[index], canvas);
            [
                egui::pos2(rect.left() - 3.0, rect.center().y),
                egui::pos2(rect.right() + 3.0, rect.center().y),
                egui::pos2(rect.center().x, rect.top() - 3.0),
                egui::pos2(rect.center().x, rect.bottom() + 3.0),
            ]
            .map(|point| (index, point))
        })
        .find(|(_, point)| {
            canvas.contains(*point)
                && centres
                    .iter()
                    .all(|centre| !marker_rect(*centre, canvas).contains(*point))
        })
        .expect("a marker with grass 3 points beside his box");
    assert!(
        (0..11).all(|other| !marker_rect(centres[other], canvas).contains(onto)),
        "premise: the release is in no marker's box"
    );
    assert!(
        marker_rect(centres[index], canvas).distance_to_pos(onto) < 5.0,
        "premise: within egui's search radius of his box"
    );

    let changed = window.drag_pitch_and_lineup(
        row_centre(&nodes, 11),
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(!changed, "a bench row on the grass is no drop");
    assert_eq!(tactics, before, "and nobody swapped in");

    // A starter whose own box is farther from the release than `index`'s:
    // a release resolved to the nearest marker lands on `index` and
    // exchanges the two, not on his own marker, where it changes nothing.
    let distance = |row: usize| marker_rect(centres[row], canvas).distance_to_pos(onto);
    let starter = (0..11)
        .find(|&row| distance(row) > distance(index))
        .expect("another eleven row farther from the release");
    assert!(
        distance(starter) > distance(index),
        "premise: his box is farther from the release than the one it is beside"
    );
    let changed = window.drag_pitch_and_lineup(
        row_centre(&nodes, starter),
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(!changed, "an eleven row on the grass is no drop");
    assert_eq!(tactics, before, "and nobody exchanged");
}

// A starter released on his own marker moves too (the arm's "or on his
// own marker"): the pointer moved 4 units right of the press still lands
// inside the marker box, where the drop is his own and the grass arm
// writes the spot 4 units right.
#[test]
fn a_marker_released_on_his_own_marker_moves() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let formation = tactics.presets[0].formations[0].clone();
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &formation.players);
    let order = draw_order(&centres);
    // An outfield slot whose x + 4 stays on the grid and whose drop
    // point covers no marker but his own (another's would claim the
    // release for a swap).
    let (index, from, onto) = (0..11)
        .filter(|&index| formation.players[index].position != 0)
        .filter_map(|index| {
            let slot = formation.players[index];
            let from = marker_point(&nodes, &formation.players, index);
            let onto = from + egui::vec2(4.0 * SPOT_XS, 0.0);
            (slot.x <= 100
                && order.iter().all(|&other| {
                    other == index || !marker_rect(centres[other], canvas).contains(onto)
                }))
            .then_some((index, from, onto))
        })
        .next()
        .expect("some outfield slot can move 4 units right");
    let slot = formation.players[index];
    assert!(
        marker_rect(centres[index], canvas).contains(onto),
        "premise: the release is still on his own marker"
    );

    let changed = window.drag_pitch(
        from,
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    let moved = tactics.presets[0].formations[0].players[index];
    assert_eq!(moved.x, slot.x + 4, "x grew by the 4 dragged units");
    assert_eq!(moved.y, slot.y, "y stayed");
}

// A press on the keeper where his marker covers another's is his, since
// a press hits the marker drawn on top, and his slot is fixed. He senses
// click-and-drag like the others so egui's hit test gives him both hits:
// a press-drag-release on him moves neither and writes nothing, and his
// release is his click.
#[test]
fn a_press_on_the_keeper_over_another_marker_moves_neither() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let keeper = tactics.presets[0].formations[0]
        .players
        .iter()
        .position(|slot| slot.position == 0)
        .expect("the fixture has a keeper slot");
    // The first outfield slot moved so its marker box covers the
    // keeper's (badge centre about (159.8, 323.0) to his (160, 358.5)).
    let under = tactics.presets[0].formations[0]
        .players
        .iter()
        .position(|slot| slot.position != 0)
        .expect("an outfield slot");
    tactics.presets[0].formations[0].players[under].x = 52;
    tactics.presets[0].formations[0].players[under].y = 6;
    let before = tactics.clone();
    let players = tactics.presets[0].formations[0].players;

    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &players);
    let point = canvas.min + egui::vec2(160.0, 330.0);
    assert!(
        marker_rect(centres[keeper], canvas).contains(point)
            && marker_rect(centres[under], canvas).contains(point),
        "premise: the point is inside both boxes"
    );
    assert!(
        draw_position(canvas, &players, keeper) > draw_position(canvas, &players, under),
        "premise: the keeper paints on top"
    );

    let changed = window.drag_pitch(
        point,
        grass_point(canvas, &players, egui::Rect::NOTHING),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(!changed, "neither marker moves");
    assert_eq!(tactics, before, "and nothing wrote");
    // His marker never moves, so the release is his click: the popover.
    assert_eq!(
        view.popover.map(|open| open.slot),
        Some(keeper),
        "the release is the keeper's click"
    );
}

// The goalkeeper's slot is fixed: his marker is no drag source, so a
// press-drag-release on it writes nothing — his release is
// his click, which opens the popover.
#[test]
fn the_keepers_marker_cannot_be_dragged() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let formation = tactics.presets[0].formations[0].clone();
    let keeper = formation
        .players
        .iter()
        .position(|slot| slot.position == 0)
        .expect("the fixture has a keeper slot");
    let before = tactics.clone();
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let from = marker_point(&nodes, &formation.players, keeper);
    let onto = grass_point(canvas, &formation.players, egui::Rect::NOTHING);

    let changed = window.drag_pitch(
        from,
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(!changed, "the keeper's spot is fixed");
    assert_eq!(tactics, before, "and nothing wrote");
    // His marker never moves, so the release is his click: the popover.
    assert_eq!(
        view.popover.map(|open| open.slot),
        Some(keeper),
        "the release is the keeper's click"
    );
}

// The widget's contract, to report what changed and nothing else: an
// idle frame and a hover over the canvas report no change and edit
// nothing.
#[test]
fn an_idle_frame_and_a_hover_change_nothing() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let before = tactics.clone();
    let (nodes, changed) =
        window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    assert!(!changed, "an idle frame reports no change");
    assert_eq!(tactics, before, "and edits nothing");

    let (_, changed) = window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        vec![egui::Event::PointerMoved(canvas_rect(&nodes).center())],
    );
    assert!(!changed, "a hover reports no change");
    assert_eq!(tactics, before, "and edits nothing");
}

// The draft's canvas drop: a bench row released on a marker swaps him
// in through `swap_in` — the taker he replaces follows to the bench.
#[test]
fn a_bench_row_dropped_on_a_marker_swaps_him_in() {
    let mut window = Headless::sized(600.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let index = 5;
    let out = tactics.starting_eleven[index];
    let inn = tactics.bench_order[0];
    assert_ne!(out, inn, "premise: two different slots");
    tactics.set_pieces.penalty = out;
    let (nodes, _) =
        window.pitch_and_lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let formation = tactics.presets[0].formations[0].clone();
    let onto = marker_point(&nodes, &formation.players, index);

    let changed = window.drag_pitch_and_lineup(
        row_centre(&nodes, 11),
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    assert_eq!(
        tactics.starting_eleven[index], inn,
        "the bench player starts"
    );
    assert_eq!(tactics.bench_order[0], out, "the starter sits");
    assert_eq!(
        tactics.set_pieces.penalty, inn,
        "the taker followed the swap"
    );
}

// An XI row released on a marker exchanges the two starters' slots —
// both stay on the pitch, so no role moves.
#[test]
fn a_starter_row_dropped_on_a_marker_exchanges_the_two() {
    let mut window = Headless::sized(600.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let index = 5;
    // A non-identity eleven: an arm writing indices would look the
    // same as an exchange on the fixture's [0, 1, 2, ...].
    tactics.starting_eleven.swap(2, 9);
    tactics.starting_eleven.swap(5, 8);
    let out = tactics.starting_eleven[2];
    let inn = tactics.starting_eleven[5];
    assert_ne!(out, inn, "premise: two different slots");
    let (nodes, _) =
        window.pitch_and_lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let formation = tactics.presets[0].formations[0].clone();
    let onto = marker_point(&nodes, &formation.players, index);

    let changed = window.drag_pitch_and_lineup(
        row_centre(&nodes, 2),
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    assert_eq!(tactics.starting_eleven[2], inn);
    assert_eq!(tactics.starting_eleven[5], out);
}

// The drop reads the swap mark's pick: a starter's marker moved onto the
// dragged one's own spot is the release's target — the dragged one's own
// spot is never it (the draft's `hitMarker` `skip`), so a release on the
// overlap swaps the two slots instead of moving the dragged one.
#[test]
fn a_release_on_a_marker_under_the_dragged_ones_spot_swaps() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    // `other`'s marker is moved onto `index`'s own spot; `index` paints
    // after him (ties keep the slot order), so the press hits `index`.
    let (index, other) = (4, 2);
    let players = &mut tactics.presets[0].formations[0].players;
    assert!(
        players[index].position != 0 && players[other].position != 0,
        "premise: two outfield slots"
    );
    players[other].x = players[index].x;
    players[other].y = players[index].y;
    assert_ne!(
        players[other].position, players[index].position,
        "premise: the two slots differ in position, so their swap shows"
    );
    let before = tactics.clone();

    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let players = tactics.presets[0].formations[0].players;
    let centres = centres_of(canvas, &players);
    let order = draw_order(&centres);
    assert!(
        draw_position(canvas, &players, index) > draw_position(canvas, &players, other),
        "premise: `index` paints after `other` at their shared spot"
    );
    // The release 10 points right of their shared centre: past the click
    // distance (a drop, not a click) and still inside both boxes.
    let onto = centres[index] + egui::vec2(10.0, 0.0);
    let covering: Vec<usize> = order
        .iter()
        .rev()
        .copied()
        .filter(|&m| m != index && marker_rect(centres[m], canvas).contains(onto))
        .collect();
    assert_eq!(
        covering.first(),
        Some(&other),
        "premise: `other` is the topmost other marker under the release"
    );
    let from = marker_point(&nodes, &players, index);

    let changed = window.drag_pitch(
        from,
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    let mut expect = before.clone();
    expect.presets[0].formations[0].players.swap(index, other);
    assert_eq!(
        tactics, expect,
        "the two slots exchanged, the starting eleven unchanged"
    );
}

// `swap_in`'s instruction gate: the outgoing starter's slot in a target
// byte repoints only where the version's decoded instruction takes one
// of ours (`takes_player`) — a byte under an instruction that takes none
// (as real saves hold) or a held Tight Marking (an opponent's) stays.
// Each version reads its own table: PES 17 stores Counter Target as a
// value PES 21 reads as another instruction.
#[test]
fn a_swap_repoints_only_targets_of_instructions_that_take_a_player() {
    assert_ne!(
        instruction::encode(PesVersion::Pes17, Instruction::CounterTarget),
        instruction::encode(PesVersion::Pes21, Instruction::CounterTarget),
        "premise: PES 17 and PES 21 store Counter Target differently"
    );
    for version in [PesVersion::Pes17, PesVersion::Pes18, PesVersion::Pes21] {
        let mut tactics = fixture_tactics(version);
        let out = tactics.starting_eleven[5];
        let inn = tactics.bench_order[0];
        assert_ne!(out, inn, "premise: two different slots ({version:?})");
        let encode = |inst| instruction::encode(version, inst).expect("the version stores it");
        let attack = tactics.presets[0]
            .attack_instructions
            .as_mut()
            .expect("the version has instructions");
        attack[0].instruction = encode(Instruction::CounterTarget);
        attack[0].player = out;
        attack[1].instruction = encode(Instruction::TikiTaka);
        attack[1].player = out;
        let defence = tactics.presets[0]
            .defence_instructions
            .as_mut()
            .expect("the version has instructions");
        defence[0].instruction = encode(Instruction::TightMarking);
        defence[0].player = out;

        swap_in(version, &mut tactics, 5, 0);

        let attack = &tactics.presets[0]
            .attack_instructions
            .as_ref()
            .expect("instructions");
        assert_eq!(
            attack[0].player, inn,
            "a Counter Target's target follows the swap ({version:?})"
        );
        assert_eq!(
            attack[1].player, out,
            "Tiki-Taka takes no player: the byte stays ({version:?})"
        );
        let defence = &tactics.presets[0]
            .defence_instructions
            .as_ref()
            .expect("instructions");
        assert_eq!(
            defence[0].player, out,
            "a held Tight Marking marks an opponent: it stays ({version:?})"
        );
    }
}

/// The badge centre of an outfield slot at stored `(x, y)` on `canvas`.
fn spot_centre(canvas: egui::Rect, x: u8, y: u8) -> egui::Pos2 {
    badge_centre(canvas.min, FormationSlot { position: 1, x, y })
}

/// `point` lies inside no marker box of `players` but `index`'s own.
fn under_no_other_marker(
    canvas: egui::Rect,
    players: &[FormationSlot; 11],
    index: usize,
    point: egui::Pos2,
) -> bool {
    centres_of(canvas, players)
        .iter()
        .enumerate()
        .all(|(other, centre)| other == index || !marker_rect(*centre, canvas).contains(point))
}

/// The texts painted on `badge`: the text shapes whose galley's centre
/// lies inside it, in paint order (his position, then his overall).
fn badge_texts(shapes: &[egui::epaint::ClippedShape], badge: egui::Rect) -> Vec<String> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if badge.contains(shape.pos + shape.galley.size() / 2.0) => {
                Some(shape.galley.job.text.clone())
            }
            _ => None,
        })
        .collect()
}

/// The frame's darkening fills: the rects filled black at alpha 153.
fn dim_fills(shapes: &[egui::epaint::ClippedShape]) -> Vec<egui::Rect> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if shape.fill == egui::Color32::from_black_alpha(153) => {
                Some(shape.rect)
            }
            _ => None,
        })
        .collect()
}

/// The area `a` and `b` share, 0 when they only touch or are apart.
fn overlap(a: egui::Rect, b: egui::Rect) -> f32 {
    let width = (a.right().min(b.right()) - a.left().max(b.left())).max(0.0);
    let height = (a.bottom().min(b.bottom()) - a.top().max(b.top())).max(0.0);
    width * height
}

// PARTY HARD pressed on his portrait, 15 left of and 40 above his badge
// centre: the pointer moved 6 to the right from the press takes his
// marker 2 units right (6 / 2.827 rounds to 2), to (22, 26); 7 leaves him
// there (2.48 rounds to 2: a whole unit at a time); 8 takes him to (23,
// 26). The pressed point stays within half a unit of the pointer, and
// nothing is written while he is held.
// SE-GUI-42
#[test]
fn a_held_marker_moves_by_whole_units_under_the_pressed_point() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let mut roster = roster_rows(&rows);
    party_hard(&mut tactics, &mut roster);
    let players = tactics.presets[0].formations[0].players;
    assert_eq!(
        (players[8].position, players[8].x, players[8].y),
        (position_byte("LMF"), 20, 26),
        "premise: PARTY HARD, the LMF, at (20, 26)"
    );
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let centre = centres_of(canvas, &players)[8];
    let press = centre + egui::vec2(-15.0, -40.0);
    assert!(
        portrait_rect(badge_rect(centre)).contains(press)
            && under_no_other_marker(canvas, &players, 8, press),
        "premise: the press is on his portrait, under no other marker"
    );

    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(press, true),
    );
    // A first move past egui's click distance starts the drag: a
    // shorter one from the press could still be a click.
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        vec![egui::Event::PointerMoved(press + egui::vec2(0.0, 10.0))],
    );
    for (dx, x) in [(6.0, 22), (7.0, 22), (8.0, 23)] {
        let pointer = press + egui::vec2(dx, 0.0);
        let (shapes, changed) = window.pitch_shapes(
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
            vec![egui::Event::PointerMoved(pointer)],
        );
        assert!(!changed, "{dx} right: a held frame writes nothing");
        let badge = *badge_rects(&shapes)
            .last()
            .expect("the held marker's badge, painted last");
        near(badge.center(), spot_centre(canvas, x, 26), 0.5);
        let pressed = badge.center() + (press - centre);
        assert!(
            (pointer.x - pressed.x).abs() <= SPOT_XS / 2.0
                && (pointer.y - pressed.y).abs() <= SPOT_YS / 2.0,
            "{dx} right: the pressed point {pressed:?} within half a unit of the pointer {pointer:?}"
        );
    }
}

// The rule runs at every step from the position he holds: PARTY HARD
// held at (30, 26) is offered CMF (the band's default) and AMF (the
// first free candidate) and holds neither, so his higher overall picks
// AMF, 85 against 84, and his badge reads AMF 85. Released at (19, 20),
// he is offered CMF and LMF, holds neither (he holds AMF), and takes
// LMF, 95 against 84. The held frames write nothing; the release writes
// his spot and position, one change.
// SE-GUI-42
#[test]
fn a_held_marker_shows_the_rules_position_and_the_release_writes_it() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let mut roster = roster_rows(&rows);
    party_hard(&mut tactics, &mut roster);
    let before = tactics.clone();
    let players = tactics.presets[0].formations[0].players;
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let press = centres_of(canvas, &players)[8];
    let held = spot_centre(canvas, 30, 26);
    let release = spot_centre(canvas, 19, 20);
    for point in [press, held, release] {
        assert!(
            under_no_other_marker(canvas, &players, 8, point),
            "premise: {point:?} is under no other marker"
        );
    }

    let (_, changed) = window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(press, true),
    );
    assert!(!changed, "the press writes nothing");
    for frame in 0..2 {
        let (shapes, changed) = window.pitch_shapes(
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
            vec![egui::Event::PointerMoved(held)],
        );
        assert!(!changed, "held frame {frame} writes nothing");
        let badge = *badge_rects(&shapes)
            .last()
            .expect("the held marker's badge, painted last");
        near(badge.center(), held, 0.5);
        assert_eq!(
            badge_texts(&shapes, badge),
            ["AMF", "85"],
            "held frame {frame}: his badge reads the position he holds"
        );
        assert_eq!(tactics, before, "held frame {frame}: the record stays");
    }
    let (_, changed) = window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(release, false),
    );
    assert!(changed, "the release writes");
    let mut expect = before;
    expect.presets[0].formations[0].players[8] = FormationSlot {
        position: position_byte("LMF"),
        x: 19,
        y: 20,
    };
    assert_eq!(tactics, expect, "his slot holds (19, 20) and LMF, alone");
}

/// Slot `index` of preset 1's Kick-off pressed on his badge centre (no
/// offset, so he stands at the pointer's spot), held at each stored spot
/// of `path` for a frame, and released at the last: the position each
/// held frame's badge shows, and whether the release changed `tactics`.
/// The pointer rests on him a frame first: a press hit-tests against the
/// markers the last frame drew.
fn hold_along(
    window: &mut Headless,
    tactics: &mut TeamTactics,
    view: &mut TacticsView,
    roster: &[RosterRow<'_>],
    index: usize,
    path: &[(u8, u8)],
) -> (Vec<String>, bool) {
    let version = PesVersion::Pes17;
    let players = tactics.presets[0].formations[0].players;
    let (nodes, _) = window.pitch(version, tactics, view, roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let press = centres_of(canvas, &players)[index];
    assert!(
        under_no_other_marker(canvas, &players, index, press),
        "premise: his badge centre is under no other marker"
    );
    window.pitch(
        version,
        tactics,
        view,
        roster,
        vec![egui::Event::PointerMoved(press)],
    );
    window.pitch(version, tactics, view, roster, pointer_at(press, true));
    let mut shown = Vec::new();
    for &(x, y) in path {
        let (shapes, changed) = window.pitch_shapes(
            version,
            tactics,
            view,
            roster,
            vec![egui::Event::PointerMoved(spot_centre(canvas, x, y))],
        );
        assert!(!changed, "held at ({x}, {y}): nothing written");
        let badge = *badge_rects(&shapes)
            .last()
            .expect("the held marker's badge, painted last");
        shown.push(badge_texts(&shapes, badge).swap_remove(0));
    }
    let &(x, y) = path.last().expect("a path of one spot or more");
    let (_, changed) = window.pitch(
        version,
        tactics,
        view,
        roster,
        pointer_at(spot_centre(canvas, x, y), false),
    );
    (shown, changed)
}

// SE-GUI-42's PES 17 cases: a held starter is ruled from the position he
// had when picked up, never from the one shown a step earlier. Team
// 702's CF COWBOY BEBOP, at (40, 45), is rated SS A and CF A; his
// overalls are CMF 99, AMF 100, SS 104 and CF 109. Held at (42, 30) he
// is offered CMF and AMF and has neither, so he shows AMF; at (42, 37),
// in SS's rows alone, SS; back at (40, 45), offered SS and his own CF,
// CF again, and a release there leaves him as he was. Released at (42,
// 30) he is an AMF, and dragged from there to (40, 45), offered SS and
// CF, he takes CF, rated higher. Released at (42, 30), then at (42, 37)
// he is an SS, and dragged to (40, 45), where SS is offered, he stays
// an SS.
// SE-GUI-42
#[test]
fn a_held_starter_is_ruled_from_the_position_he_had_when_picked_up() {
    let mut window = Headless::sized(600.0, 450.0);
    let (mut tactics, overalls) = team_702();
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes17);
    let mut roster = roster_rows(&rows);
    let cf = 10;
    roster[usize::from(tactics.starting_eleven[cf])].overalls = Some(overalls);
    let start = tactics.clone();
    let players = start.presets[0].formations[0].players;
    assert_eq!(
        (players[cf].position, players[cf].x, players[cf].y),
        (position_byte("CF"), 40, 45),
        "premise: COWBOY BEBOP, the CF, at (40, 45)"
    );
    assert_eq!(
        [overalls[5], overalls[8], overalls[11], overalls[12]],
        [99, 100, 104, 109],
        "premise: his overalls at CMF, AMF, SS and CF"
    );
    let (nodes, _) = window.pitch(PesVersion::Pes17, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    for (x, y) in [(42, 30), (42, 37)] {
        assert!(
            under_no_other_marker(canvas, &players, cf, spot_centre(canvas, x, y)),
            "premise: ({x}, {y}) is clear of his teammates' markers"
        );
    }
    let slot = |tactics: &TeamTactics| tactics.presets[0].formations[0].players[cf];
    let at = |position: &str, x: u8, y: u8| FormationSlot {
        position: position_byte(position),
        x,
        y,
    };

    let (shown, changed) = hold_along(
        &mut window,
        &mut tactics,
        &mut view,
        &roster,
        cf,
        &[(42, 30), (42, 37), (40, 45)],
    );
    assert_eq!(shown, ["AMF", "SS", "CF"], "held down, on and back up");
    assert!(!changed, "released where he was picked up: no change");
    assert_eq!(tactics, start, "he is the CF at (40, 45) still");

    let (shown, _) = hold_along(
        &mut window,
        &mut tactics,
        &mut view,
        &roster,
        cf,
        &[(42, 30)],
    );
    assert_eq!(shown, ["AMF"], "held at (42, 30)");
    assert_eq!(slot(&tactics), at("AMF", 42, 30), "released an AMF");
    let (shown, _) = hold_along(
        &mut window,
        &mut tactics,
        &mut view,
        &roster,
        cf,
        &[(40, 45)],
    );
    assert_eq!(shown, ["CF"], "the AMF held at (40, 45)");
    assert_eq!(slot(&tactics), at("CF", 40, 45), "released a CF");

    tactics = start.clone();
    for (x, y, position) in [(42, 30, "AMF"), (42, 37, "SS"), (40, 45, "SS")] {
        let (shown, _) = hold_along(&mut window, &mut tactics, &mut view, &roster, cf, &[(x, y)]);
        assert_eq!(shown, [position], "held at ({x}, {y})");
        assert_eq!(slot(&tactics), at(position, x, y), "released at ({x}, {y})");
    }
}

// PARTY HARD, rated A at LMF and LWF alone, held: the pitch outside
// those two areas is darkened by fills that cover the canvas less the
// rectangle from its left and top edges to x 20.5 and y 15.5 through the
// spot mapping (LMF's x 5 to 20 and y 16 to 34, LWF's above it to the
// top), no two of them overlapping, since two alpha fills over one place
// would read darker. Nothing is darkened before he is held, nor while
// the keeper, who is never dragged, is pressed and moved.
// SE-GUI-42
#[test]
fn a_held_starter_lights_his_rated_areas_and_darkens_the_rest() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let mut roster = roster_rows(&rows);
    party_hard(&mut tactics, &mut roster);
    let players = tactics.presets[0].formations[0].players;
    let (shapes, _) =
        window.pitch_shapes(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    assert_eq!(dim_fills(&shapes), [], "nothing held: nothing darkened");
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let press = centres_of(canvas, &players)[8];
    let held = spot_centre(canvas, 30, 26);

    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(press, true),
    );
    let (shapes, _) = window.pitch_shapes(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        vec![egui::Event::PointerMoved(held)],
    );
    let lit = egui::Rect::from_min_max(
        canvas.min,
        canvas.min + egui::vec2(SPOT_X0 + SPOT_XS * 20.5, SPOT_Y0 - SPOT_YS * 15.5),
    );
    let dims = dim_fills(&shapes);
    assert!(!dims.is_empty(), "held: the rest is darkened");
    for (i, dim) in dims.iter().enumerate() {
        assert!(
            canvas.expand(0.5).contains_rect(*dim),
            "fill {dim:?} inside the canvas"
        );
        assert!(overlap(*dim, lit) <= 0.5, "fill {dim:?} off the lit area");
        for other in &dims[i + 1..] {
            assert!(
                overlap(*dim, *other) <= 0.5,
                "fills {dim:?} and {other:?} do not overlap"
            );
        }
    }
    let darkened: f32 = dims.iter().map(|dim| dim.area()).sum();
    assert!(
        (darkened - (canvas.area() - lit.area())).abs() <= 1.0,
        "the fills cover the canvas less the lit area: {darkened} against {}",
        canvas.area() - lit.area()
    );
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(held, false),
    );

    // The keeper pressed and moved: no drag, nothing darkened.
    let players = tactics.presets[0].formations[0].players;
    let keeper = players
        .iter()
        .position(|slot| slot.position == 0)
        .expect("the formation has a keeper slot");
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let point = marker_point(&nodes, &players, keeper);
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(point, true),
    );
    let (shapes, _) = window.pitch_shapes(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        vec![egui::Event::PointerMoved(held)],
    );
    assert_eq!(
        dim_fills(&shapes),
        [],
        "the keeper pressed: nothing darkened"
    );
}
