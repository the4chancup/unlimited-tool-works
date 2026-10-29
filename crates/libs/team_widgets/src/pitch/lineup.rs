//! The "Starting eleven" and "Bench" lists, the draft's `lineupList` and
//! `benchList`. Each row is a drag handle, a position and a name; a drop
//! moves, exchanges or swaps a bench player in (`swap_in`).

use pes_savefile::model::tactics::{SetPieceTakers, TeamTactics};
use pes_savefile::schema::instruction;
use pes_version::PesVersion;

use studio_core::theme;
use studio_core::widgets::ink_span;

use super::{LineupDrag, lineup_colour, position_label};
use crate::names::append_name;
use crate::tactics::{RosterRow, TacticsView, raw_name_at};

/// The gap between the two lists (the draft's `#tac-lists` `gap: 6px`).
const LIST_GAP: f32 = 6.0;
/// A row's height (the draft's `.lrow`).
const ROW_HEIGHT: f32 = 18.0;
/// The lists' viewport height: twelve rows (the draft's `#tac-lists
/// .llist` — the eleven whole with a row to spare, which the bench uses
/// for its twelfth player).
const LIST_HEIGHT: f32 = 12.0 * ROW_HEIGHT;
/// The padding above and below a list's rows (`#tac-lists .llist`'s
/// `padding: 4px 0`).
const LIST_PADDING: f32 = 4.0;
/// A row's drag handle stands this far from the row's left edge, and the
/// position this far after the handle, both in ink (the draft's `.lrow
/// .handle`): the name gets the room.
const HANDLE_INSET: f32 = 4.0;
/// The position column's width (the draft's `.lrow .fpos`).
const POSITION_WIDTH: f32 = 30.0;

/// The draft's `#tac-lists` beside the pitch: the starting eleven and
/// the bench, each list half of the span it is given less `LIST_GAP`,
/// twelve 18-point rows tall plus `LIST_PADDING` top and bottom, framed
/// — no headings, so every panel of the grid's row lines up. Each row a
/// drag handle, a position and a name (the positions are
/// `view.formation`'s). Drops: a starter on a
/// starter exchanges the two, a bench row on a starter (or a starter on
/// a bench row) swaps them in `swap_in`'s sense, a bench row on a bench
/// row moves the dragged entry to the dropped row's place. `roster`
/// gives each roster slot's row, indexed by slot; a `bench_order` entry
/// whose roster slot holds no player draws no row, so the eleven never
/// gains an empty slot either (no row takes a starter).
/// `version` is the open save's (`swap_in`'s instruction decode).
/// Answers `true` only when `tactics` changed this frame
/// (`tactics_settings`' contract).
pub fn lineup(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    version: PesVersion,
    tactics: &mut TeamTactics,
    view: &mut TacticsView,
    roster: &[RosterRow<'_>],
) -> bool {
    ui.push_id(id_salt, |ui| {
        // The answer is the deep compare, as `tactics_settings`' is.
        let before = tactics.clone();
        let positions = tactics.presets[view.preset.min(2)].formations[view.formation.min(2)]
            .players
            .map(|slot| slot.position);
        // Each list its own child of `half` wide, so its frame fills the
        // column's share rather than hugging the rows.
        let top = ui.cursor().min;
        let right = ui.max_rect().right();
        let bottom = ui.max_rect().bottom();
        let half = (right - top.x - LIST_GAP) / 2.0;
        let cell = |ui: &mut egui::Ui, left: f32| {
            ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(egui::Rect::from_min_max(
                        egui::pos2(left, top.y),
                        egui::pos2(left + half, bottom),
                    ))
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            )
        };
        let mut eleven = cell(ui, top.x);
        list(&mut eleven, "starting eleven", |ui| {
            for (index, position) in positions.iter().enumerate() {
                starter_row(ui, version, tactics, roster, *position, index);
            }
        });
        let mut bench = cell(ui, top.x + half + LIST_GAP);
        list(&mut bench, "bench", |ui| {
            // The players only: a `bench_order` entry whose roster slot
            // holds no player draws no row (a 4cc team's 21 entries show
            // its 12 players).
            for index in 0..tactics.bench_order.len() {
                if bench_is_empty(tactics, roster, index) {
                    continue;
                }
                bench_row(ui, version, tactics, roster, index);
            }
        });
        let lists_bottom = eleven.min_rect().max.y.max(bench.min_rect().max.y);
        ui.advance_cursor_after_rect(egui::Rect::from_min_max(
            top,
            egui::pos2(right, lists_bottom),
        ));
        *tactics != before
    })
    .inner
}

/// One of the two lists (the draft's `.llist`): the framed box filled in
/// the extreme background, its scroll area `LIST_HEIGHT` of rows with
/// `LIST_PADDING` above and below; the rows draw through `add`.
fn list(ui: &mut egui::Ui, id_salt: &'static str, add: impl FnOnce(&mut egui::Ui)) {
    // No margin left of the rows: a row runs from the frame, as the draft's
    // `.lrow` from its list's border, so its handle's inset is its own.
    // None above or below either: the draft's `.llist` pads its rows by
    // `LIST_PADDING` alone inside its border.
    let frame = egui::Frame::group(ui.style());
    let margin = frame.inner_margin;
    frame
        .fill(ui.visuals().extreme_bg_color)
        .inner_margin(egui::Margin {
            left: 0,
            top: 0,
            bottom: 0,
            ..margin
        })
        .show(ui, |ui| {
            // No gaps between the padding and the scroll either: the inner
            // height is exactly `LIST_HEIGHT` plus the two pads.
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add_space(LIST_PADDING);
            // Exactly `LIST_HEIGHT` whatever height the host leaves (the
            // draft's `min-height`): a scroll area takes the height
            // available up to its max, so a host shortened by a dock
            // would otherwise shrink the lists rather than scroll.
            egui::ScrollArea::vertical()
                .id_salt(id_salt)
                .auto_shrink([false, false])
                .min_scrolled_height(LIST_HEIGHT)
                .max_height(LIST_HEIGHT)
                .show(ui, |ui| {
                    // 18-point rows back to back (the draft's `.lrow`): no
                    // row gap, the handle row's own height the draft's 18.
                    ui.spacing_mut().interact_size.y = ROW_HEIGHT;
                    add(ui);
                });
            ui.add_space(LIST_PADDING);
        });
}

/// One starting-eleven row (the draft's XI `lrow`): the formation slot's
/// position and the player's name, always a drag source and a drop
/// target — the goalkeeper a starter like the others. Drops: a starter
/// exchanges the two slots' players (both stay on the pitch, so no role
/// moves); a bench player swaps in (`swap_in`).
fn starter_row(
    ui: &mut egui::Ui,
    version: PesVersion,
    tactics: &mut TeamTactics,
    roster: &[RosterRow<'_>],
    position: u8,
    index: usize,
) {
    let slot = tactics.starting_eleven[index];
    let response = lineup_row(
        ui,
        ("starting eleven", index),
        LineupDrag::Starter(index),
        Some(position),
        raw_name_at(roster, slot),
    );
    if let Some(dropped) = response.dnd_release_payload::<LineupDrag>() {
        match *dropped {
            LineupDrag::Starter(from) | LineupDrag::Marker(from) => {
                tactics.starting_eleven.swap(from, index);
            }
            LineupDrag::Bench(from) => swap_in(version, tactics, index, from),
        }
    }
}

/// Whether `bench_order[index]`'s roster slot holds no player: past the
/// roster or its `registered_position` unset (the host's empty-slot row,
/// `("", "", None)`).
fn bench_is_empty(tactics: &TeamTactics, roster: &[RosterRow<'_>], index: usize) -> bool {
    roster
        .get(usize::from(tactics.bench_order[index]))
        .is_none_or(|row| row.registered_position.is_none())
}

/// One bench row (the draft's bench `lrow`): the player's registered
/// position and name, always a drag source and a drop target. A starter
/// dropped on it swaps in (`swap_in`); a bench row dropped on it moves the
/// dragged entry to the dropped row's `bench_order` place, the entries
/// between shifting by one (the draft's `moveBench`, a move, not an
/// exchange).
fn bench_row(
    ui: &mut egui::Ui,
    version: PesVersion,
    tactics: &mut TeamTactics,
    roster: &[RosterRow<'_>],
    index: usize,
) {
    let slot = tactics.bench_order[index];
    let position = roster
        .get(usize::from(slot))
        .and_then(|row| row.registered_position);
    let response = lineup_row(
        ui,
        ("bench", index),
        LineupDrag::Bench(index),
        position,
        raw_name_at(roster, slot),
    );
    if let Some(dropped) = response.dnd_release_payload::<LineupDrag>() {
        match *dropped {
            LineupDrag::Starter(starter) | LineupDrag::Marker(starter) => {
                swap_in(version, tactics, starter, index);
            }
            LineupDrag::Bench(from) => {
                if from < index {
                    tactics.bench_order[from..=index].rotate_left(1);
                } else if from > index {
                    tactics.bench_order[index..=from].rotate_right(1);
                }
            }
        }
    }
}

/// One lineup row, the draft's `.lrow`: the ≡ handle, the position
/// column and the raw name drawn with its colour codes; a drag source
/// of `payload`, a drop target always. `position` is the position byte
/// the row shows, drawn in its group's lineup colour (`lineup_colour`),
/// which reads on the panel; `None` draws `--` in the weak colour.
fn lineup_row(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    payload: LineupDrag,
    position: Option<u8>,
    raw_name: &str,
) -> egui::Response {
    ui.dnd_drag_source(ui.scope_id().with(id_salt), payload, |ui| {
        ui.horizontal(|ui| {
            // The whole band is the drop target (the draft's `.lrow` spans
            // the list), not only the handle, position and name.
            ui.set_min_width(ui.available_width());
            let name_gap = ui.spacing().item_spacing.x;
            ui.spacing_mut().item_spacing.x = 0.0;
            let font = egui::TextStyle::Body.resolve(ui.style());
            let weak = ui.visuals().weak_text_color();
            let handle = ui
                .painter()
                .layout_no_wrap("≡".to_owned(), font.clone(), weak);
            let place = ui.painter().layout_no_wrap(
                position.map_or("--", position_label).to_owned(),
                font.clone(),
                position.map_or(weak, |position| {
                    lineup_colour(theme::palette(ui.visuals()), position)
                }),
            );
            // The handle and the position placed by their ink, which
            // starts a glyph's side bearing in from its box: boxes alone
            // left them a point off on some positions' letters.
            let handle_ink = ink_span(&handle);
            ui.add_space(HANDLE_INSET - handle_ink.min);
            let after_handle = handle.size().x - handle_ink.max;
            ui.add(egui::Label::new(handle));
            ui.add_space(HANDLE_INSET - after_handle - ink_span(&place).min);
            ui.allocate_ui_with_layout(
                egui::vec2(POSITION_WIDTH, ui.spacing().interact_size.y),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_width(POSITION_WIDTH);
                    ui.add(egui::Label::new(place));
                },
            );
            ui.add_space(name_gap);
            let mut job = egui::text::LayoutJob::default();
            append_name(&mut job, raw_name, &font);
            // Cut at the row's end (the full name on hover): the
            // host gives the lists a fixed width, and a long name
            // would otherwise draw over the column at their right.
            ui.add(egui::Label::new(job).truncate());
        });
    })
    .response
}

/// A bench player swapped onto the pitch: `starting_eleven[starter]` and
/// `bench_order[bench]` exchange, then every set-piece taker, every
/// join-attack entry and every preset's instruction target — where the
/// version's decoded instruction takes one of ours — that held the
/// outgoing starter's roster slot now holds the incoming one's, so no
/// role points at a player who left the pitch. The captain stays who he
/// was, benched or not.
pub(super) fn swap_in(
    version: PesVersion,
    tactics: &mut TeamTactics,
    starter: usize,
    bench: usize,
) {
    let out = tactics.starting_eleven[starter];
    let inn = tactics.bench_order[bench];
    tactics.starting_eleven[starter] = inn;
    tactics.bench_order[bench] = out;
    let SetPieceTakers {
        free_kick_long,
        free_kick_short,
        free_kick_second,
        corner_left,
        corner_right,
        penalty,
        // Named, not `..`: a taker field added later must join the list
        // above or fail to compile here, never be skipped like the captain.
        captain: _,
    } = &mut tactics.set_pieces;
    for taker in [
        free_kick_long,
        free_kick_short,
        free_kick_second,
        corner_left,
        corner_right,
        penalty,
    ] {
        if *taker == out {
            *taker = inn;
        }
    }
    for join in &mut tactics.players_to_join_attack {
        if *join == out {
            *join = inn;
        }
    }
    for preset in &mut tactics.presets {
        for instructions in [
            &mut preset.attack_instructions,
            &mut preset.defence_instructions,
        ]
        .into_iter()
        .flatten()
        {
            for entry in instructions {
                // The target byte is one of ours only where the
                // instruction takes one (`takes_player`): a held Tight
                // Marking's is an opponent's, and a target-less
                // instruction's is never read — both stay.
                if entry.player == out
                    && let Ok(Some(inst)) = instruction::decode(version, entry.instruction)
                    && inst.takes_player()
                {
                    entry.player = inn;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use egui::accesskit::Role;
    use pes_savefile::model::instruction::Instruction;
    use pes_version::PesVersion;

    use super::*;
    use crate::pitch::tests::{lineup_rows, marker_point, position_of_row, row_centre};
    use crate::test_support::{
        Headless, centre, entry, fixture_rows, fixture_tactics, pointer_at, roster_rows,
    };

    /// The lists' framed boxes in `shapes`, left to right (the eleven's
    /// first): the stroked rects taller than a row.
    fn list_frame_shapes(shapes: &[egui::epaint::ClippedShape]) -> Vec<&egui::epaint::RectShape> {
        let mut boxes: Vec<&egui::epaint::RectShape> = shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if !shape.stroke.is_empty() && shape.rect.height() > 100.0 =>
                {
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
        let (nodes, shapes) =
            window.lineup_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);
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
        let (nodes, shapes) =
            window.lineup_paint(PesVersion::Pes21, &mut tactics, &mut view, &roster);

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
}
