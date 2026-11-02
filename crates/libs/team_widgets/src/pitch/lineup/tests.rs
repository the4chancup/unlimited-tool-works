use egui::accesskit::Role;
use pes_savefile::model::instruction::Instruction;
use pes_version::PesVersion;

use super::*;
use crate::pitch::tests::{lineup_rows, marker_point, position_of_row, row_centre};
use crate::test_support::{
    Headless, centre, entry, fixture_rows, fixture_tactics, pointer_at, roster_rows, text_shape,
};

/// The lists' framed boxes in `shapes`, left to right (the eleven's
/// first): the stroked rects taller than a row.
fn list_frame_shapes(shapes: &[egui::epaint::ClippedShape]) -> Vec<&egui::epaint::RectShape> {
    let mut boxes: Vec<&egui::epaint::RectShape> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if !shape.stroke.is_empty() && shape.rect.height() > 100.0 => {
                Some(shape)
            }
            _ => None,
        })
        .collect();
    boxes.sort_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x));
    boxes
}

/// `list_frame_shapes`' rects.
fn list_frames(shapes: &[egui::epaint::ClippedShape]) -> Vec<egui::Rect> {
    list_frame_shapes(shapes)
        .into_iter()
        .map(|shape| shape.rect)
        .collect()
}

// A starter swapped with a bench player: every set-piece role, the
// joining of the attack and every preset's instruction target the
// starter held repoint to the incoming player. The captain stays.
#[test]
fn a_swap_hands_the_starters_roles_to_the_bench_player_but_not_the_captaincy() {
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let out = tactics.starting_eleven[4];
    let inn = tactics.bench_order[2];
    assert_ne!(out, inn, "premise: two different slots");
    let other = tactics.starting_eleven[6];
    assert!(other != out && other != inn, "premise: a third slot");
    let pieces = &mut tactics.set_pieces;
    pieces.free_kick_long = out;
    pieces.free_kick_short = out;
    pieces.free_kick_second = out;
    pieces.corner_left = out;
    pieces.corner_right = out;
    pieces.penalty = out;
    pieces.captain = out;
    tactics.players_to_join_attack = [other, out, u8::MAX];
    // The repoint is gated on the instruction taking one of ours:
    // the entries hold Counter Target.
    let counter_target = instruction::encode(PesVersion::Pes21, Instruction::CounterTarget)
        .expect("PES 21 stores Counter Target");
    tactics.presets[1]
        .attack_instructions
        .as_mut()
        .expect("PES 21 stores instructions")[0]
        .instruction = counter_target;
    tactics.presets[1]
        .attack_instructions
        .as_mut()
        .expect("PES 21 stores instructions")[0]
        .player = out;
    tactics.presets[2]
        .defence_instructions
        .as_mut()
        .expect("PES 21 stores instructions")[1]
        .instruction = counter_target;
    tactics.presets[2]
        .defence_instructions
        .as_mut()
        .expect("PES 21 stores instructions")[1]
        .player = out;
    tactics.presets[0]
        .defence_instructions
        .as_mut()
        .expect("PES 21 stores instructions")[0]
        .instruction = counter_target;
    tactics.presets[0]
        .defence_instructions
        .as_mut()
        .expect("PES 21 stores instructions")[0]
        .player = other;
    swap_in(PesVersion::Pes21, &mut tactics, 4, 2);
    assert_eq!(tactics.starting_eleven[4], inn, "the bench player starts");
    assert_eq!(tactics.bench_order[2], out, "the starter sits");
    let pieces = &tactics.set_pieces;
    for taker in [
        pieces.free_kick_long,
        pieces.free_kick_short,
        pieces.free_kick_second,
        pieces.corner_left,
        pieces.corner_right,
        pieces.penalty,
    ] {
        assert_eq!(taker, inn, "every taker follows the swap");
    }
    assert_eq!(pieces.captain, out, "the captain stays who he was");
    assert_eq!(
        tactics.players_to_join_attack,
        [other, inn, u8::MAX],
        "the join entry follows, the other starter's stays"
    );
    assert_eq!(
        tactics.presets[1]
            .attack_instructions
            .as_ref()
            .expect("instructions")[0]
            .player,
        inn,
        "preset 2's attack target follows"
    );
    assert_eq!(
        tactics.presets[2]
            .defence_instructions
            .as_ref()
            .expect("instructions")[1]
            .player,
        inn,
        "preset 3's defence target follows"
    );
    assert_eq!(
        tactics.presets[0]
            .defence_instructions
            .as_ref()
            .expect("instructions")[0]
            .player,
        other,
        "the other starter's target stays"
    );

    // A starter holding none of the six roles: each taker is another
    // starter and stays who he was. The eleven is shuffled so that a
    // taker's roster slot equals the outgoing starter's formation
    // index, which a repoint keyed by the index would move.
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    tactics.starting_eleven.swap(4, 9);
    assert_ne!(tactics.starting_eleven[4], 4, "premise: a shuffled eleven");
    let out = tactics.starting_eleven[4];
    let inn = tactics.bench_order[2];
    let holders = [5, 6, 7, 8, 9, 10].map(|index| tactics.starting_eleven[index]);
    assert!(
        holders.iter().all(|&holder| holder != out && holder != inn),
        "premise: the takers are neither of the two"
    );
    assert!(
        holders.contains(&4),
        "premise: a taker's roster slot is the outgoing starter's index"
    );
    let pieces = &mut tactics.set_pieces;
    pieces.free_kick_long = holders[0];
    pieces.free_kick_short = holders[1];
    pieces.free_kick_second = holders[2];
    pieces.corner_left = holders[3];
    pieces.corner_right = holders[4];
    pieces.penalty = holders[5];
    swap_in(PesVersion::Pes21, &mut tactics, 4, 2);
    assert_eq!(tactics.starting_eleven[4], inn, "the bench player starts");
    assert_eq!(tactics.bench_order[2], out, "the starter sits");
    let pieces = &tactics.set_pieces;
    assert_eq!(
        [
            pieces.free_kick_long,
            pieces.free_kick_short,
            pieces.free_kick_second,
            pieces.corner_left,
            pieces.corner_right,
            pieces.penalty,
        ],
        holders,
        "the other starters keep their roles"
    );
}

// The same swap on a version with no instruction table moves the
// takers and the join entries only.
#[test]
fn a_swap_on_pes16_moves_the_takers_and_join_entries() {
    let mut tactics = fixture_tactics(PesVersion::Pes16);
    assert!(
        tactics.presets.iter().all(|preset| {
            preset.attack_instructions.is_none() && preset.defence_instructions.is_none()
        }),
        "premise: PES 16 stores no instructions"
    );
    let out = tactics.starting_eleven[0];
    let inn = tactics.bench_order[0];
    assert_ne!(out, inn, "premise: two different slots");
    let pieces = &mut tactics.set_pieces;
    pieces.penalty = out;
    pieces.captain = out;
    tactics.players_to_join_attack = [out, u8::MAX, u8::MAX];
    swap_in(PesVersion::Pes16, &mut tactics, 0, 0);
    assert_eq!(tactics.starting_eleven[0], inn);
    assert_eq!(tactics.bench_order[0], out);
    assert_eq!(tactics.set_pieces.penalty, inn, "the taker follows");
    assert_eq!(tactics.set_pieces.captain, out, "the captain stays");
    assert_eq!(
        tactics.players_to_join_attack,
        [inn, u8::MAX, u8::MAX],
        "the join entry follows"
    );
}

// The draft's `rowDrop` over `lineupList`: a bench row dropped on a
// starter runs the swap through the widget.
#[test]
fn a_bench_row_dropped_on_a_starter_swaps_them_and_moves_the_roles() {
    let mut window = Headless::sized(900.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let out = tactics.starting_eleven[3];
    let inn = tactics.bench_order[0];
    assert_ne!(out, inn, "premise: two different slots");
    tactics.set_pieces.penalty = out;
    let (nodes, _) = window.lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let changed = window.drag_lineup(
        row_centre(&nodes, 11),
        row_centre(&nodes, 3),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    assert_eq!(tactics.starting_eleven[3], inn, "the bench player starts");
    assert_eq!(tactics.bench_order[0], out, "the starter sits");
    assert_eq!(
        tactics.set_pieces.penalty, inn,
        "the taker followed the swap"
    );
}

// A row spans the list's width, as the draft's `.lrow` does: a release
// on a row's band right of its name, inside the list, is a drop on that
// row, as a release on the name is.
#[test]
fn a_bench_row_released_on_a_starters_band_past_the_name_swaps_them() {
    let mut window = Headless::sized(900.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let out = tactics.starting_eleven[3];
    let inn = tactics.bench_order[0];
    assert_ne!(out, inn, "premise: two different slots");
    tactics.set_pieces.penalty = out;
    let (nodes, shapes) = window.lineup_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let eleven = list_frames(&shapes)[0];
    let name = lineup_rows(&nodes)[3]
        .last()
        .copied()
        .expect("row 3's name")
        .bounds()
        .expect("name bounds");
    let onto = egui::pos2(eleven.right() - 10.0, row_centre(&nodes, 3).y);
    assert!(
        f64::from(onto.x) > name.x1 && eleven.contains(onto),
        "premise: past the name ({name:?}), inside the eleven's frame ({eleven:?}) at {onto:?}"
    );

    let changed = window.drag_lineup(
        row_centre(&nodes, 11),
        onto,
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    assert_eq!(tactics.starting_eleven[3], inn, "the bench player starts");
    assert_eq!(tactics.bench_order[0], out, "the starter sits");
    assert_eq!(
        tactics.set_pieces.penalty, inn,
        "the taker followed the swap"
    );
}

// A couple of points above, beside and between the labels are inside
// the row's band yet outside every label, within egui's interaction
// radius of one of them: the pointer there is still the row's grab
// cursor, never a selectable label's text cursor.
#[test]
fn just_outside_a_rows_labels_the_pointer_is_still_the_grab_cursor() {
    let mut window = Headless::sized(900.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let (nodes, shapes) = window.lineup_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let lineup = lineup_rows(&nodes);
    let lists = list_frames(&shapes);
    let rect = |bounds: egui::accesskit::Rect| {
        egui::Rect::from_min_max(
            egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
            egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
        )
    };
    // A starter's row and a bench row: a point just over the name,
    // one before it, one after the position, and one beside the
    // handle in whichever of its two gaps is two points wide.
    for index in [3, 11 + 3] {
        let row: [&egui::accesskit::Node; 3] = lineup[index]
            .as_slice()
            .try_into()
            .expect("three labels on a row");
        let [handle, position, name] =
            row.map(|node| rect(node.bounds().expect("a label's bounds")));
        let frame = lists[usize::from(index >= 11)].shrink(1.0);
        let centre = name.center();
        let (left, right) = [(frame.min.x, handle.min.x), (handle.max.x, position.min.x)]
            .into_iter()
            .find(|&(left, right)| right - left >= 2.0)
            .expect("a gap beside the handle is two points wide");
        for (point, side, near) in [
            (
                egui::pos2(centre.x, name.min.y - 1.5),
                "above the name",
                name,
            ),
            (
                egui::pos2(name.min.x - 2.0, centre.y),
                "left of the name",
                name,
            ),
            (
                egui::pos2(position.max.x + 2.0, centre.y),
                "right of the position",
                position,
            ),
            (
                egui::pos2((left + right) / 2.0, centre.y),
                "beside the handle",
                handle,
            ),
        ] {
            assert!(
                frame.contains(point),
                "premise: {side} {point:?} is inside the list's frame ({frame:?})"
            );
            for (bounds, column) in [(handle, "handle"), (position, "position"), (name, "name")] {
                assert!(
                    !bounds.contains(point),
                    "premise: {side} {point:?} is outside the {column} label ({bounds:?})"
                );
            }
            assert!(
                near.distance_sq_to_pos(point) <= 25.0,
                "premise: {side} {point:?} is within five points of its label"
            );
            window.lineup(
                PesVersion::Pes21,
                &mut tactics,
                &mut view,
                &roster,
                vec![egui::Event::PointerMoved(point)],
            );
            assert_eq!(
                window.cursor_icon(),
                egui::CursorIcon::Grab,
                "row {index}, {side}"
            );
        }
    }
}

// A press a couple of points off a row's labels drags the row
// rather than starting a text selection — a
// starter lands on a starter as an exchange, a bench row on a bench
// row as a move.
#[test]
fn a_drag_started_just_beside_a_rows_labels_moves_the_player() {
    let mut window = Headless::sized(900.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    assert_ne!(
        tactics.starting_eleven[1], tactics.starting_eleven[4],
        "premise: two different slots"
    );
    let before = tactics.clone();
    let (nodes, _) = window.lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let lineup = lineup_rows(&nodes);
    let position = lineup[1][1].bounds().expect("row 1's position bounds");
    let name = lineup[1][2].bounds().expect("row 1's name bounds");
    let from = egui::pos2(name.x0 as f32 - 2.0, row_centre(&nodes, 1).y);
    assert!(
        f64::from(from.x) > position.x1 && f64::from(from.x) < name.x0,
        "premise: the press lands in the gap between position and name ({from:?})"
    );

    let changed = window.drag_lineup(
        from,
        row_centre(&nodes, 4),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    let mut expect = before;
    expect.starting_eleven.swap(1, 4);
    assert_eq!(tactics, expect, "the two starters exchanged");

    // The bench row's side of the gesture: a press two points right
    // of the position, released on another bench row.
    tactics = fixture_tactics(PesVersion::Pes21);
    let before = tactics.bench_order;
    assert_ne!(before[1], before[4], "premise: two different slots");
    let (nodes, _) = window.lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let lineup = lineup_rows(&nodes);
    let position = lineup[11 + 1][1]
        .bounds()
        .expect("bench row 1's position bounds");
    let name = lineup[11 + 1][2]
        .bounds()
        .expect("bench row 1's name bounds");
    let from = egui::pos2(position.x1 as f32 + 2.0, row_centre(&nodes, 11 + 1).y);
    assert!(
        f64::from(from.x) < name.x0,
        "premise: the press lands before the name ({from:?})"
    );

    let changed = window.drag_lineup(
        from,
        row_centre(&nodes, 11 + 4),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    let mut expect = before;
    expect[1] = before[2];
    expect[2] = before[3];
    expect[3] = before[4];
    expect[4] = before[1];
    assert_eq!(tactics.bench_order, expect, "row 1 landed at 4");
}

// A name longer than the row draws cut, and the row's hover is the
// whole plain name — the labels sense nothing, so the hover is the
// row's own (the draft's `.lname` `title`). A name that fits hovers
// nothing.
#[test]
fn a_cut_name_shows_the_whole_plain_name_on_the_rows_hover() {
    let mut window = Headless::sized(900.0, 1100.0);
    window
        .ctx
        .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let mut rows = fixture_rows(PesVersion::Pes21);
    let slot = usize::from(tactics.starting_eleven[3]);
    rows[slot].1 = "THE \u{11}cb7bec5ffEXTRAORDINARILY LONG-LEGGED FORWARD\u{11}d OF THE ENTIRE TWENTIETH CENTURY AND BEYOND".to_owned();
    rows[usize::from(tactics.starting_eleven[4])].1 = "TINY FIT".to_owned();
    let roster = roster_rows(&rows);
    let (nodes, shapes) = window.lineup_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert!(
        text_shape(&shapes, "THE ").galley.elided,
        "premise: the name is cut at the row's end"
    );

    // The pointer rests on the cut name: past the delay the hover's
    // text is the whole plain name — a second node carrying it,
    // beside the label's own.
    let full = "THE EXTRAORDINARILY LONG-LEGGED FORWARD OF THE ENTIRE TWENTIETH CENTURY AND BEYOND";
    let at = row_centre(&nodes, 3);
    let mut carrying = 0;
    window.lineup(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        vec![egui::Event::PointerMoved(at)],
    );
    for _ in 0..30 {
        let (nodes, _) = window.lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        carrying = nodes
            .iter()
            .filter(|node| node.value() == Some(full))
            .count();
        if carrying == 2 {
            break;
        }
    }
    assert_eq!(carrying, 2, "the row's hover shows the whole plain name");

    // A name that fits shows no hover: its text stays the row
    // label's alone.
    let at = row_centre(&nodes, 4);
    window.lineup(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        vec![egui::Event::PointerMoved(at)],
    );
    for _ in 0..10 {
        let (nodes, _) = window.lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        carrying = nodes
            .iter()
            .filter(|node| node.value() == Some("TINY FIT"))
            .count();
    }
    assert_eq!(carrying, 1, "a fitting name hovers nothing");
}

// The draft's `swapStarters`: a starter on a starter exchanges the two
// slots' players; both stay on the pitch so no role moves. Row 1 on
// row 4, not on row 2: on adjacent rows a move (the bench rows'
// shift) gives the same eleven as the exchange.
#[test]
fn a_starter_dropped_on_a_starter_exchanges_them_and_moves_no_role() {
    let mut window = Headless::sized(900.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    assert_ne!(
        tactics.starting_eleven[1], tactics.starting_eleven[4],
        "premise: two different slots"
    );
    tactics.set_pieces.penalty = tactics.starting_eleven[1];
    let before = tactics.clone();
    let (nodes, _) = window.lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let changed = window.drag_lineup(
        row_centre(&nodes, 1),
        row_centre(&nodes, 4),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    let mut expect = before;
    expect.starting_eleven.swap(1, 4);
    assert_eq!(
        tactics, expect,
        "the two exchanged, the rows between and the role stay"
    );
}

// The draft's `moveBench`: the dragged entry lands on the dropped
// row's place, the rows between shifting by one toward it.
#[test]
fn a_bench_row_dropped_on_a_bench_row_lands_in_its_place() {
    let mut window = Headless::sized(900.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let before = tactics.bench_order;
    assert_ne!(before[4], before[1], "premise: two different slots");

    // Row 4 dropped on row 1: it takes 1's place, the rows between
    // shifting down.
    let (nodes, _) = window.lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let changed = window.drag_lineup(
        row_centre(&nodes, 11 + 4),
        row_centre(&nodes, 11 + 1),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    let mut expect = before;
    expect[1] = before[4];
    expect[2] = before[1];
    expect[3] = before[2];
    expect[4] = before[3];
    assert_eq!(tactics.bench_order, expect, "row 4 landed at 1");

    // Row 1 dropped on row 4: it takes 4's place, the rows between
    // shifting up.
    tactics = fixture_tactics(PesVersion::Pes21);
    let before = tactics.bench_order;
    let (nodes, _) = window.lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let changed = window.drag_lineup(
        row_centre(&nodes, 11 + 1),
        row_centre(&nodes, 11 + 4),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    let mut expect = before;
    expect[1] = before[2];
    expect[2] = before[3];
    expect[3] = before[4];
    expect[4] = before[1];
    assert_eq!(tactics.bench_order, expect, "row 1 landed at 4");
}

// The draft's `#tac-lists .llist`: no "Starting eleven" or "Bench"
// headings; each list half of the span less the 6-point gap, its
// inner height twelve 18-point rows plus 4 of padding top and
// bottom.
#[test]
fn the_lists_are_untitled_halves_of_twelve_18_point_rows() {
    let mut window = Headless::sized(900.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    // The border alone adds to the padded rows: the draft's `.llist`
    // keeps no margin above or below them.
    let mut margin = 0.0;
    window.frame(vec![], |ui| {
        margin = 2.0 * f64::from(egui::Frame::group(ui.style()).stroke.width);
    });
    let (nodes, shapes) = window.lineup_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);

    for heading in ["Starting eleven", "Bench"] {
        assert!(
            !nodes
                .iter()
                .any(|node| node.role() == Role::Label && node.value() == Some(heading)),
            "no {heading:?} heading"
        );
    }
    // Each framed box a half of the 900-wide span less the gap; its
    // inner height 12 * 18 + 8.
    let boxes = list_frames(&shapes);
    assert_eq!(boxes.len(), 2, "the two framed lists ({boxes:?})");
    let half = (900.0 - LIST_GAP) / 2.0;
    for (index, list) in boxes.iter().enumerate() {
        assert!(
            (list.width() - half).abs() <= 1.0,
            "list {index} a half less the gap ({list:?})"
        );
        assert!(
            (f64::from(list.height()) - (12.0 * 18.0 + 8.0) - margin).abs() <= 1.0,
            "list {index}'s inner height twelve rows plus the padding ({list:?})"
        );
    }
    assert!(
        (boxes[1].min.x - (boxes[0].min.x + half + LIST_GAP)).abs() <= 1.0,
        "the second list a half and a gap later ({boxes:?})"
    );
    // Rows 18 points back to back: the first two rows' tops.
    let row_ys: Vec<f64> = lineup_rows(&nodes)[..2]
        .iter()
        .map(|row| row[0].bounds().expect("row bounds").y0)
        .collect();
    assert!(
        (row_ys[1] - row_ys[0] - 18.0).abs() <= 0.5,
        "18-point rows back to back ({row_ys:?})"
    );
}

// The draft's `.llist` is `background: var(--extreme)`: both lists'
// frames are filled in the extreme background, not left to the
// pane's grey.
#[test]
fn the_lists_are_filled_in_the_extreme_background() {
    let mut window = Headless::sized(900.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let mut extreme = egui::Color32::TRANSPARENT;
    window.frame(vec![], |ui| extreme = ui.visuals().extreme_bg_color);
    assert_ne!(
        extreme,
        egui::Color32::TRANSPARENT,
        "premise: the theme's extreme background is a colour"
    );
    let (_, shapes) = window.lineup_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let frames = list_frame_shapes(&shapes);
    assert_eq!(frames.len(), 2, "the two framed lists ({frames:?})");
    for (index, frame) in frames.iter().enumerate() {
        assert_eq!(frame.fill, extreme, "list {index}'s fill");
    }
}

// The bench list shows the players only: a `bench_order` entry whose roster
// slot holds no player draws no row (the fixture team's 21 entries
// show its 12 bench players: rows 12 to 20 hold roster slots 23 to 31,
// past team 701's 23 players).
#[test]
fn the_bench_list_draws_the_filled_entries_only() {
    let mut window = Headless::sized(900.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let filled = tactics
        .bench_order
        .iter()
        .filter(|slot| {
            roster
                .get(usize::from(**slot))
                .is_some_and(|row| row.registered_position.is_some())
        })
        .count();
    assert_eq!(filled, 12, "premise: team 701's bench holds 12 players");

    let (nodes, _) = window.lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let rows = lineup_rows(&nodes);
    assert_eq!(
        rows.len(),
        11 + filled,
        "the bench list's rows are the filled entries alone"
    );
    assert!(
        rows.iter()
            .all(|row| row.get(2).and_then(|node| node.value()) != Some("(empty)")),
        "no (empty) row is drawn"
    );
}

// The XI rows' position labels read the preset's formation on screen:
// the selector (the pitch's header) writes `view.formation`, never the
// record.
#[test]
fn the_position_labels_follow_the_formation_on_screen() {
    let mut window = Headless::sized(900.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    tactics.presets[0].formations[0].players[5].position = 4; // DMF
    tactics.presets[0].formations[1].players[5].position = 8; // AMF
    // Fluid formation on, so the selector offers In-possession.
    tactics.presets[0].style.fluid = Some(true);
    let before = tactics.clone();
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let (nodes, _) =
        window.pitch_and_lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    assert_eq!(position_of_row(&nodes, 5), "DMF", "kick-off's slot 5");

    let at = centre(entry(&nodes, Role::Button, "In-possession"));
    window.pitch_and_lineup(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(at, true),
    );
    let (_, changed) = window.pitch_and_lineup(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(at, false),
    );
    assert!(!changed, "the selector writes the view, not the record");
    assert_eq!(view.formation, 1, "in-possession is on screen");
    assert_eq!(tactics, before, "and the record is untouched");

    let (nodes, _) =
        window.pitch_and_lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    assert_eq!(position_of_row(&nodes, 5), "AMF", "in-possession's slot 5");
}

// And the reverse direction: a marker dragged onto a bench row swaps
// him out through `swap_in`, as a marker's `LineupDrag::Marker` is
// the row's payload.
#[test]
fn a_marker_dropped_on_a_bench_row_swaps_him_out() {
    let mut window = Headless::sized(600.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let index = 4;
    let out = tactics.starting_eleven[index];
    let inn = tactics.bench_order[0];
    assert_ne!(out, inn, "premise: two different slots");
    tactics.set_pieces.penalty = out;
    let (nodes, _) =
        window.pitch_and_lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let formation = tactics.presets[0].formations[0].clone();
    let from = marker_point(&nodes, &formation.players, index);

    let changed = window.drag_pitch_and_lineup(
        from,
        row_centre(&nodes, 11),
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

// The draft's `swapStarters` on `.lrow`: a marker dragged onto a
// starting-eleven row exchanges the two players in the eleven, as a row
// dropped on a row does. Every formation stays.
#[test]
fn a_marker_dropped_on_an_eleven_row_exchanges_the_two() {
    let mut window = Headless::sized(600.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let (index, row) = (3, 7);
    // A non-identity eleven: an arm writing indices would look the
    // same as an exchange on the fixture's [0, 1, 2, ...].
    tactics.starting_eleven.swap(3, 9);
    assert_ne!(
        tactics.starting_eleven[index], tactics.starting_eleven[row],
        "premise: two different slots"
    );
    let before = tactics.clone();
    let (nodes, _) =
        window.pitch_and_lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let formation = tactics.presets[0].formations[0].clone();
    let from = marker_point(&nodes, &formation.players, index);

    let changed = window.drag_pitch_and_lineup(
        from,
        row_centre(&nodes, row),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the drop changed the record");
    let mut expect = before;
    expect.starting_eleven.swap(index, row);
    assert_eq!(
        tactics, expect,
        "the two exchanged in the eleven, every formation unchanged"
    );
}

// The draft's `.dragover`: the row a drop on it would swap out is
// outlined in the selection colour while a drag is held over it — a
// starter's and a bench row's band alike — and a marker held over his
// own row draws none, that drop exchanging his slot with itself.
// Released off the lists, no row is outlined.
// SE-GUI-59
#[test]
fn the_row_under_a_held_drag_is_outlined_in_the_selection_colour() {
    /// The bands `shapes` outline in `selection`: the unfilled rects
    /// stroked in it (the lists' frame strokes are another colour).
    fn outlined(
        shapes: &[egui::epaint::ClippedShape],
        selection: egui::Color32,
    ) -> Vec<egui::Rect> {
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if shape.fill == egui::Color32::TRANSPARENT
                        && shape.stroke.color == selection =>
                {
                    Some(shape.rect)
                }
                _ => None,
            })
            .collect()
    }

    let mut window = Headless::sized(900.0, 1100.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let before = tactics.clone();
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let mut selection = egui::Color32::TRANSPARENT;
    window.frame(vec![], |ui| {
        selection = ui.visuals().selection.stroke.color;
    });

    // Row 1's drag held over row 4: row 4's band alone is outlined.
    let (nodes, _) = window.lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let over = row_centre(&nodes, 4);
    window.lineup(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(row_centre(&nodes, 1), true),
    );
    let (_, shapes) = window.frame(vec![egui::Event::PointerMoved(over)], |ui| {
        lineup(
            ui,
            "lineup",
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
        );
    });
    let outlines = outlined(&shapes, selection);
    let [outline] = outlines.as_slice() else {
        panic!("row 4's band alone is outlined, got {outlines:?}")
    };
    assert!(
        outline.contains(over),
        "the outline is the band under the pointer ({outline:?}, centre {over:?})"
    );
    assert!(
        outline.height() < ROW_HEIGHT * 2.0,
        "the outline is one row, not a run of rows ({outline:?})"
    );

    // A bench row's band outlines the same way.
    let over = row_centre(&nodes, 11 + 2);
    let (_, shapes) = window.frame(vec![egui::Event::PointerMoved(over)], |ui| {
        lineup(
            ui,
            "lineup",
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
        );
    });
    let outlines = outlined(&shapes, selection);
    let [outline] = outlines.as_slice() else {
        panic!("bench row 2's band alone is outlined, got {outlines:?}")
    };
    assert!(
        outline.contains(over),
        "the bench row's band is outlined ({outline:?}, centre {over:?})"
    );

    // Held off the lists, then released there: no row is outlined and
    // nothing drops.
    let off = egui::pos2(450.0, 900.0);
    let (_, shapes) = window.frame(vec![egui::Event::PointerMoved(off)], |ui| {
        lineup(
            ui,
            "lineup",
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
        );
    });
    assert!(
        outlined(&shapes, selection).is_empty(),
        "held off the lists outlines no row"
    );
    let (_, changed) = window.lineup(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(off, false),
    );
    assert!(!changed, "the release off the lists dropped nothing");
    assert_eq!(tactics, before, "the record is untouched");
    let (_, shapes) = window.lineup_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert!(
        outlined(&shapes, selection).is_empty(),
        "the drag ended: no row is outlined"
    );

    // A marker held over his own row: the drop exchanges his slot with
    // itself, which writes nothing, so the row shows no outline. Over
    // another starter's row the drop swaps, so that row shows one.
    let (nodes, _) =
        window.pitch_and_lineup(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let formation = tactics.presets[0].formations[0].clone();
    let index = 4;
    let paint_view = |window: &mut Headless,
                      events: Vec<egui::Event>,
                      tactics: &mut TeamTactics,
                      view: &mut TacticsView,
                      roster: &[RosterRow<'_>]| {
        window.frame(events, |ui| {
            crate::pitch::formation_selector(ui, tactics, view);
            crate::pitch::pitch(ui, "pitch", PesVersion::Pes21, tactics, view, roster, false);
            lineup(ui, "lineup", PesVersion::Pes21, tactics, view, roster);
        })
    };
    window.pitch_and_lineup(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(marker_point(&nodes, &formation.players, index), true),
    );
    let (_, shapes) = paint_view(
        &mut window,
        vec![egui::Event::PointerMoved(row_centre(&nodes, index))],
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(
        outlined(&shapes, selection).is_empty(),
        "a marker over his own row: the drop writes nothing, no outline"
    );
    let (_, shapes) = paint_view(
        &mut window,
        vec![egui::Event::PointerMoved(row_centre(&nodes, 7))],
        &mut tactics,
        &mut view,
        &roster,
    );
    let outlines = outlined(&shapes, selection);
    let [outline] = outlines.as_slice() else {
        panic!("row 7's band alone is outlined under the marker, got {outlines:?}")
    };
    assert!(
        outline.contains(row_centre(&nodes, 7)),
        "the outline is row 7's band ({outline:?})"
    );
    // Released on his own row the drop writes nothing, as the missing
    // outline said.
    let (_, changed) = window.pitch_and_lineup(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(row_centre(&nodes, index), false),
    );
    assert!(!changed, "the drop on his own row wrote nothing");
    assert_eq!(tactics, before, "the record is untouched");
}
