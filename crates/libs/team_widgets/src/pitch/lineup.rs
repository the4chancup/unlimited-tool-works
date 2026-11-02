//! The "Starting eleven" and "Bench" lists, the draft's `lineupList` and
//! `benchList`. Each row is a drag handle, a position and a name; a drop
//! moves, exchanges or swaps a bench player in (`swap_in`).

use pes_savefile::model::names::display_name;
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
/// of `payload`, a drop target always, and a name cut at the row's end
/// hovers the full name on the row's own response. `position` is the
/// position byte the row shows, drawn in its group's lineup colour
/// (`lineup_colour`), which reads on the panel; `None` draws `--` in
/// the weak colour.
/// Its labels are not selectable: egui hands a press within its
/// interaction radius of a click-sensing widget inside a drag area to
/// that widget, so a press just beside a selectable label would start a
/// text selection instead of the row's drag.
/// While a drag holds over the row, its band is outlined in the
/// selection colour (the draft's `.dragover`), so a member sees which
/// row the drop swaps out; a row whose drop would write nothing gets
/// none (`drop_writes`).
fn lineup_row(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    payload: LineupDrag,
    position: Option<u8>,
    raw_name: &str,
) -> egui::Response {
    let id = ui.scope_id().with(id_salt);
    let drag = ui.dnd_drag_source(id, payload, |ui| {
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
            ui.add(egui::Label::new(handle).selectable(false));
            ui.add_space(HANDLE_INSET - after_handle - ink_span(&place).min);
            ui.allocate_ui_with_layout(
                egui::vec2(POSITION_WIDTH, ui.spacing().interact_size.y),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_width(POSITION_WIDTH);
                    ui.add(egui::Label::new(place).selectable(false));
                },
            );
            ui.add_space(name_gap);
            let mut job = egui::text::LayoutJob::default();
            append_name(&mut job, raw_name, &font);
            // Cut at the row's end, the full name hovering on the row:
            // the host gives the lists a fixed width, and a long name
            // would otherwise draw over the column at their right. The
            // hover goes on the row because the label senses nothing.
            job.wrap = egui::text::TextWrapping::truncate_at_width(ui.available_width());
            let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
            let elided = galley.elided;
            ui.add(egui::Label::new(galley).selectable(false));
            elided
        })
        .inner
    });
    // egui centres a dragged source on the pointer (`dnd_drag_source`'s
    // own translation of its layer); the draft keeps the point the row
    // was pressed at under it, so the press's offset from the centre goes
    // back on top.
    if ui.ctx().is_being_dragged(id)
        && let Some(press_origin) = ui.input(|i| i.pointer.press_origin())
    {
        ui.ctx().transform_layer_shapes(
            egui::LayerId::new(egui::Order::Tooltip, id),
            egui::emath::TSTransform::from_translation(drag.response.rect.center() - press_origin),
        );
    }
    let row = drag.response;
    // The outline sits outside the band, as a CSS outline sits outside
    // the box.
    if let Some(dropped) = row.dnd_hover_payload::<LineupDrag>()
        && drop_writes(*dropped, payload)
    {
        ui.painter().rect_stroke(
            row.rect,
            egui::CornerRadius::ZERO,
            egui::Stroke::new(1.0, ui.visuals().selection.stroke.color),
            egui::StrokeKind::Outside,
        );
    }
    if drag.inner {
        row.on_hover_text(display_name(raw_name))
    } else {
        row
    }
}

/// Whether a release of `dropped` on the row carrying `row` writes the
/// record, as `starter_row`'s and `bench_row`'s drop arms rule it: a
/// drop on the dragged row's own place swaps or moves a slot with
/// itself, which writes nothing (a marker held over his own row lands
/// there too); every other drop swaps two places or moves the dragged
/// entry.
fn drop_writes(dropped: LineupDrag, row: LineupDrag) -> bool {
    match dropped {
        LineupDrag::Starter(from) | LineupDrag::Marker(from) => match row {
            LineupDrag::Starter(index) => from != index,
            LineupDrag::Bench(_) | LineupDrag::Marker(_) => true,
        },
        LineupDrag::Bench(from) => match row {
            LineupDrag::Bench(index) => from != index,
            LineupDrag::Starter(_) | LineupDrag::Marker(_) => true,
        },
    }
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
mod tests;
