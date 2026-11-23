//! The master list: the team selector, the cross-team filter box, one team's row
//! heading its players' rows, and the quick actions under the list. A player row
//! shows the name drawn in the colors its savefile codes set; a team row the
//! team's id, its colours 1-2 as swatches and its name.

use egui::AtomExt as _;
use egui::Widget as _;
use egui::class::HasClasses as _;
use egui::emath::GuiRounding as _;
use pes_savefile::EditFile;
use pes_savefile::interchange::team_toml::labels::POSITIONS;
use pes_savefile::model::PlayerEntry;
use pes_savefile::model::names::display_name;
use studio_core::widgets::{
    combo_item, combo_separator, ink_span, radio_value, step_choice, text_box, wheel_steps,
};
use team_widgets::names::{append_name, plain_format};

use crate::session::Session;
use crate::session::cup::cup_teams;
use crate::session::selection::{self, ListRow, Row, TeamCategory, TeamChoice};
use crate::view::clipboard::{ClickKind, pressed};
use crate::view::team::channel_to_show;
use crate::view::{
    Reveal, SaveView, Selection, group_on, live_marks, quick_actions, reveal_for, selected_team_id,
};

/// A swatch square's side (the draft's 10px).
const SWATCH: f32 = 10.0;

/// The gap added to the row's item spacing between a dirty row's `●`
/// and the Live AATF mark after it (the draft's `.plist .dot + .mark`
/// `margin-left: 4px` on top of its 5 px flex gap: 9 px dot to mark,
/// measured on the rendered draft).
pub(super) const DOT_MARK_GAP: f32 = 4.0;

/// A player row's drag handle `≡` stands this far from the row's left
/// edge, and his place this far after the handle, both in ink (the
/// draft's `.prow .handle`): the name gets the room.
const HANDLE_INSET: f32 = 4.0;

/// The gap between a player row's columns, the place, the position and
/// the name (the draft's `.prow` gap).
const COLUMN_GAP: f32 = 5.0;

/// The position's column width (the draft's `.pos`): the names start at
/// one x whatever the position's letters.
const POSITION_WIDTH: f32 = 28.0;

/// The `●` of a row with unsaved changes (the draft's `.plist .dot`: the
/// warning colour, pushed to the row's right end): its galley at the
/// body font. `None` on a clean row.
fn dirty_dot(ui: &egui::Ui, dirty: bool) -> Option<std::sync::Arc<egui::Galley>> {
    dirty.then(|| {
        ui.painter().layout_no_wrap(
            "\u{25cf}".to_owned(),
            egui::TextStyle::Body.resolve(ui.style()),
            ui.visuals().warn_fg_color,
        )
    })
}

/// The width the row's text leaves at its right end for the dot and the
/// mark that show (`None`: not shown) — one `item_spacing.x` after the
/// text, the dot's and the mark's widths, and `item_spacing.x` plus
/// `DOT_MARK_GAP` between them when both show.
fn end_room(ui: &egui::Ui, dot_width: Option<f32>, mark_width: Option<f32>) -> f32 {
    if dot_width.is_none() && mark_width.is_none() {
        return 0.0;
    }
    let spacing = ui.spacing().item_spacing.x;
    let between = if dot_width.is_some() && mark_width.is_some() {
        spacing + DOT_MARK_GAP
    } else {
        0.0
    };
    spacing + dot_width.unwrap_or(0.0) + mark_width.unwrap_or(0.0) + between
}

/// The dot's right edge: `right_end` (the row's right less its padding)
/// with no mark, else `item_spacing.x` plus `DOT_MARK_GAP` before the
/// mark's left edge.
fn dot_right(ui: &egui::Ui, right_end: f32, mark_width: Option<f32>) -> f32 {
    right_end
        - mark_width.map_or(0.0, |width| {
            width + ui.spacing().item_spacing.x + DOT_MARK_GAP
        })
}

/// A player row's right-end layout: the dirty dot's galley, `mark`'s
/// width and the room `end_room` leaves for them — the row and the held
/// row's float take it alike, so a name is cut at the same width either
/// way.
fn end_layout(
    ui: &egui::Ui,
    dirty: bool,
    mark: Option<&live_marks::RowMark>,
) -> (Option<std::sync::Arc<egui::Galley>>, Option<f32>, f32) {
    let mark_width = mark.as_ref().map(|mark| mark.width(ui));
    let dot = dirty_dot(ui, dirty);
    let room = end_room(ui, dot.as_ref().map(|galley| galley.size().x), mark_width);
    (dot, mark_width, room)
}

/// The selector, the filter box, the rows and the quick actions, top to
/// bottom of the side panel: the list's scroll area takes the height left
/// above the actions.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    // egui's solid scroll bar stands outside the rows' frame, in a strip
    // (its 4-point gap and 6-point bar) kept at the column's right whether
    // or not the list scrolls, so a filter leaving few rows keeps the
    // list's width. A floating bar lies over the rows, where a row's dot
    // and mark at its right end take their hover and click. The controls
    // above and below end at the frame's edge too, so the column reads as
    // one box.
    let scroll = egui::style::ScrollStyle::solid();
    let frame_width = ui.available_width() - scroll.allocated_width();
    let Some(session) = view.session.as_ref() else {
        return;
    };
    let team_before = view.team;
    let category_before = view.team_category;
    let selected_before = view.selected;
    ui.scope(|ui| {
        ui.spacing_mut().combo_width = frame_width;
        team_selector(
            ui,
            &mut view.team,
            Some(&mut view.team_category),
            session.file(),
            "save_editor_team",
        );
    });
    // Picking a team selects its team row, so the detail shows that team's
    // view, its tactics with it: a manager who picks a team usually wants
    // its tactics next — but not when the selection is already on that
    // team, a selected player included: narrowing the list to his own
    // team is a way to see his teammates, not to leave him. Picking All
    // teams keeps the selection.
    if view.team != team_before
        && let TeamChoice::Team(id) = view.team
        && selected_team_id(view) != Some(id)
    {
        view.selected = Some(Selection::Team(id));
        view.group.clear();
    }
    // A category pick: a team the new category leaves out gives way to
    // All teams, and a selection it leaves out moves to the list's first
    // player row (kept when it lists none).
    if view.team_category != category_before {
        if let TeamChoice::Team(id) = view.team
            && !view.team_category.contains(id)
        {
            view.team = TeamChoice::All;
        }
        let selection_out = view.selected.is_some_and(|selection| {
            let id = match selection {
                Selection::Team(id) | Selection::Player(id, _) => id,
            };
            !view.team_category.contains(id)
        });
        if selection_out {
            view.selected =
                selection::list(session.file(), &view.team, &view.filter, view.team_category)
                    .into_iter()
                    .find_map(|row| match row {
                        ListRow::Player(row) => Some(Selection::Player(row.team_id, row.slot)),
                        ListRow::Team(_) => None,
                    })
                    .or(view.selected);
            view.group.clear();
        }
    }
    // A pick scrolls to the row still selected when it named a team, moved
    // the selection or relisted the rows around it. Naming a team counts on
    // its own: under a typed filter the team's row may already be selected
    // and its rows already listed, yet scrolled off screen. Any other pick
    // moves nothing.
    if view.team != team_before || view.team_category != category_before {
        let listed_before =
            selection::list(session.file(), &team_before, &view.filter, category_before);
        let listed_after =
            selection::list(session.file(), &view.team, &view.filter, view.team_category);
        let team_picked = view.team != team_before && matches!(view.team, TeamChoice::Team(_));
        if view.selected.is_some()
            && (team_picked || view.selected != selected_before || listed_before != listed_after)
        {
            view.reveal_selected = Some(reveal_for(view.selected));
        }
    }
    ui.add(text_box(
        egui::TextEdit::singleline(&mut view.filter)
            .hint_text("Team or player: name, shirt name or ID")
            .desired_width(frame_width),
    ));
    // The quick actions sit at the panel's bottom, under the list (the
    // draft's `qagrid`): a bottom panel takes its height off the scroll's.
    // The panel's frame keeps its fill, the draft's `.plist` gap of 3 over
    // the grid and no margin under it, so the grid's last row ends at the
    // pane's bottom; no horizontal margin either, and the grid's width
    // stops at the frame's, so the grid spans the list rows' width. No
    // separator line over the grid: the draft's rows' border is followed
    // by the gap alone.
    let margin = egui::Margin {
        top: 3,
        ..egui::Margin::ZERO
    };
    egui::Panel::bottom("save_editor_quick_actions")
        .show_separator_line(false)
        .frame(egui::Frame::side_top_panel(ui.style()).inner_margin(margin))
        .show(ui, |ui| {
            // A child of its own: the panel's `ui` already spans the
            // panel's width, which `set_max_width` cannot shrink.
            ui.vertical(|ui| {
                ui.set_max_width(frame_width);
                quick_actions::show(ui, view);
            });
        });
    // A drop works on a row of the dragged player's own team, in one team's
    // list or under All teams where each team's rows run unbroken under its team
    // row; drag is off while a filter is typed, where neighbouring rows are
    // not one team's neighbouring slots.
    let layout = RowLayout {
        // Only the visible rows are laid out: `show_rows` draws the range
        // the viewport shows, each row's click target at the text row's
        // height plus the button padding (`interact_size.y`, what a
        // selectable occupies).
        row_height: ui.spacing().interact_size.y,
        width: frame_width,
        draggable: view.filter.trim().is_empty(),
    };
    // Live AATF's held check: the rows of the team it is for carry its
    // marks, the other teams' none.
    let held = view.rules.as_ref().and_then(|active| {
        live_marks::held_violations(
            view.live_aatf,
            view.live_suggestions,
            &view.live.check,
            &active.key,
        )
    });
    let session = view.session.as_mut().expect("checked above");
    let rows = selection::list(session.file(), &view.team, &view.filter, view.team_category);
    // The selected row's index and the reveal `reveal_selected` asks for,
    // when one is set and the row is listed: scrolled to below.
    let reveal = std::mem::take(&mut view.reveal_selected).and_then(|reveal| {
        rows.iter()
            .position(|row| Some(row_selection(*row)) == view.selected)
            .map(|index| (index, reveal))
    });
    ui.spacing_mut().scroll = scroll;
    // The rows touch, as the draft's do: `show_rows` adds the item
    // spacing's 3 points under each row, which made them 21 apart.
    ui.spacing_mut().item_spacing.y = 0.0;
    // The frame's fill goes under the rows, painted once the scroll area
    // has its rect.
    let background = ui.painter().add(egui::Shape::Noop);
    // The team whose row was double-clicked, applied once the rows are
    // drawn: the rows borrow the selection and the session.
    let mut shown_team = None;
    // The drop's mark, painted once after the rows: the first row to
    // report it wins (a pointer on the edge two rows share hovers both,
    // and the release there goes to the upper one).
    let mut drop_mark = None;
    // The `Id` the list's scroll area keeps its `State` under, keyed by
    // the open save (`generation`) so another save opened starts the
    // list at its top rather than at the last save's offset:
    // `.id_salt` below wraps the value in `IdSalt::new` itself, so
    // `make_persistent_id` of the same `IdSalt` is the area's `Id`,
    // letting `State::load` read the current offset for `Nearest`.
    let scroll_salt = ("save_editor_player_list", view.generation);
    // Not shrunk: the frame takes the panel's height whatever few rows a
    // filter leaves. The scope clears the extreme background, which a
    // solid bar's track paints in at full opacity: the draft's `.vbar` has
    // no track, the panel shows through. The rows' frame reads the colour
    // outside the scope.
    let output = ui
        .scope(|ui| {
            ui.visuals_mut().extreme_bg_color = egui::Color32::TRANSPARENT;
            // The reveal's scroll given to the area up front, so its
            // rows are laid out at it this pass: a `scroll_to_rect`
            // request lands at the area's end, on the next pass's
            // offset, and the selected row would draw a frame ahead of
            // its scroll. `Nearest` reads the current offset from the
            // area's stored state; `None` only before the first draw,
            // where the offset is 0 anyway. `Top` clamps to the
            // scrollable range, so the last team's row stands as high
            // as the list's end lets it.
            let scroll_offset = reveal.map(|(index, reveal)| {
                let row_top = index as f32 * layout.row_height;
                let row_bottom = row_top + layout.row_height;
                let viewport = ui.available_rect_before_wrap().height();
                let offset = egui::scroll_area::State::load(
                    ui.ctx(),
                    ui.make_persistent_id(egui::IdSalt::new(scroll_salt)),
                )
                .map_or(0.0, |state| state.unclamped_offset().y);
                let target = match reveal {
                    Reveal::Top => row_top,
                    Reveal::Nearest if row_top < offset => row_top,
                    Reveal::Nearest if row_bottom > offset + viewport => row_bottom - viewport,
                    Reveal::Nearest => offset,
                };
                target.clamp(
                    0.0,
                    (rows.len() as f32 * layout.row_height - viewport).max(0.0),
                )
            });
            if scroll_offset.is_some() {
                // The reveal's offset can stand past the stored
                // `max_offset` of a list that grew this frame (a widened
                // category or a cleared filter): with rubber-banding on,
                // egui's spring reads it as an overscroll and pulls the
                // list back. `vertical_scroll_offset` lands this frame,
                // so the spring is off for it.
                ui.spacing_mut().scroll.kinetic.rubber_band = false;
            }
            let mut area = egui::ScrollArea::vertical()
                .id_salt(scroll_salt)
                .auto_shrink([false, false]);
            if let Some(offset) = scroll_offset {
                area = area.vertical_scroll_offset(offset);
            }
            area.show_rows(ui, layout.row_height, rows.len(), |ui, range| {
                for row in &rows[range] {
                    match row {
                        ListRow::Team(team_id) => {
                            let double_clicked = team_row(
                                ui,
                                &mut RowSelection {
                                    selected: &mut view.selected,
                                    group: &mut view.group,
                                    click_kind: &mut view.click_kind,
                                },
                                session,
                                *team_id,
                                layout,
                                held.as_ref()
                                    .filter(|(id, _)| *id == *team_id)
                                    .map(|(_, violations)| live_marks::row_mark(violations, None)),
                            );
                            if double_clicked {
                                shown_team = Some(*team_id);
                            }
                        }
                        ListRow::Player(row) => {
                            drop_mark = drop_mark.or(player_row(
                                ui,
                                &mut RowSelection {
                                    selected: &mut view.selected,
                                    group: &mut view.group,
                                    click_kind: &mut view.click_kind,
                                },
                                session,
                                row,
                                layout,
                                held.as_ref().filter(|(id, _)| *id == row.team_id).map(
                                    |(_, violations)| {
                                        live_marks::row_mark(violations, Some(row.slot))
                                    },
                                ),
                            ));
                        }
                    }
                }
                // Painted once, after the rows: painted with its row,
                // the next row's background would cover its lower
                // half. The mark's ends are the row's own edges, inside
                // the frame's border as the row's box is.
                if let Some(mark) = drop_mark {
                    let stroke = egui::Stroke::new(2.0, ui.visuals().selection.stroke.color);
                    let y = if mark.lower {
                        mark.rect.max.y
                    } else {
                        mark.rect.min.y
                    };
                    let (x0, x1) = (mark.rect.min.x, mark.rect.max.x);
                    ui.painter().hline(x0..=x1, y, stroke);
                    ui.painter().vline(x0 + 1.0, y - 3.5..=y + 3.5, stroke);
                    ui.painter().vline(x1 - 1.0, y - 3.5..=y + 3.5, stroke);
                }
                // The held row's float, painted once after the rows as
                // the drop mark is: at the pointer less the grab kept at
                // the press, so the point pressed stays under it — the
                // row itself scrolls with the list and its place is left
                // blank, the float does not. Painted on the tooltip
                // layer, the scroll area's clip would cut it at the
                // list's edge. The payload lives until egui's end of
                // pass, so the release frame still reads it: skip it —
                // the drag is over, and nothing would ask for the frame
                // that removes it.
                if let Some(dragged) = egui::DragAndDrop::payload::<DraggedRow>(ui.ctx())
                    && !ui.input(|i| i.pointer.any_released())
                    && let Some(at) = ui.input(|i| i.pointer.interact_pos())
                {
                    let rect = egui::Rect::from_min_size(at - dragged.grab, row_size(ui, layout));
                    let is_selected = row_is_selected(view.selected, &view.group, &dragged.row);
                    let color = if is_selected {
                        ui.visuals().selection.stroke.color
                    } else {
                        ui.visuals().widgets.active.fg_stroke.color
                    };
                    let mark = held
                        .as_ref()
                        .filter(|(id, _)| *id == dragged.row.team_id)
                        .map(|(_, violations)| {
                            live_marks::row_mark(violations, Some(dragged.row.slot))
                        });
                    let (_, _, room) = end_layout(
                        ui,
                        session.player_dirty(dragged.row.team_id, dragged.row.slot),
                        mark.as_ref(),
                    );
                    let texts = row_text(
                        session.file().player(dragged.row.player_id),
                        &dragged.row,
                        &egui::TextStyle::Body.resolve(ui.style()),
                    );
                    paint_row_text(
                        ui,
                        &ui.ctx().layer_painter(egui::LayerId::new(
                            egui::Order::Tooltip,
                            ui.scope_id().with("lifted_row"),
                        )),
                        rect,
                        texts,
                        color,
                        room,
                    );
                }
            })
        })
        .inner;
    // A double click on a team row shows that team alone: the selector
    // picks it, the filter empties and the list scrolls to its row.
    if let Some(id) = shown_team {
        view.team = TeamChoice::Team(id);
        view.filter.clear();
        view.selected = Some(Selection::Team(id));
        view.group.clear();
        view.reveal_selected = Some(Reveal::Nearest);
    }
    // The viewport spans the bar's strip while nothing scrolls: the frame
    // keeps its own width.
    let frame = egui::Rect::from_min_size(
        output.inner_rect.min,
        egui::vec2(frame_width, output.inner_rect.height()),
    );
    // Rounded as every framed panel is (the draft's `border-radius: 2px`).
    let radius = ui.visuals().widgets.noninteractive.corner_radius;
    ui.painter().set(
        background,
        egui::epaint::RectShape::filled(frame, radius, ui.visuals().extreme_bg_color),
    );
    // The stroke over the rows, so a selected or hovered row's fill
    // reaches the frame's edge under it.
    ui.painter().rect_stroke(
        frame,
        radius,
        ui.visuals().widgets.noninteractive.bg_stroke,
        egui::StrokeKind::Inside,
    );
}

/// The step the page keys move by while the list holds at most one team
/// row (a team picked): seven rows (the draft's `PAGE_ROWS`).
const PAGE_ROWS: usize = 7;

/// Selects the row `steps` rows after the selected row in the list as
/// drawn (the team selector and the filter applied), or the one before
/// it when `forward` is false: the arrow keys step one row. Every row
/// counts, the team rows too, so the walk can land on a team's row and
/// select it as a click does; a step past the first or last row lands on
/// it, and with the selection not in the list nothing moves. Answers
/// whether the selection moved.
pub(super) fn step_selection(view: &mut SaveView, forward: bool, steps: usize) -> bool {
    let Some((drawn, at)) = list_and_position(view) else {
        return false;
    };
    let next = if forward {
        (at + steps).min(drawn.len() - 1)
    } else {
        at.saturating_sub(steps)
    };
    if next == at {
        return false;
    }
    view.selected = Some(drawn[next]);
    view.group.clear();
    true
}

/// Selects the row a page key moves to: with two team rows or more in
/// the list as drawn (the team selector and the filter applied, wherever
/// it is scrolled), the next team row after the selected one, or the one
/// before it when `forward` is false, a player going to his own team's
/// row first; with at most one team row, `PAGE_ROWS` rows at a time. The
/// first and last rows stop it, and so does the first or last team row:
/// no team row past the selection, nothing moves. Answers whether the
/// selection moved.
pub(super) fn page_selection(view: &mut SaveView, forward: bool) -> bool {
    let Some((drawn, at)) = list_and_position(view) else {
        return false;
    };
    let teams: Vec<usize> = drawn
        .iter()
        .enumerate()
        .filter_map(|(index, row)| matches!(row, Selection::Team(_)).then_some(index))
        .collect();
    let next = if teams.len() >= 2 {
        if forward {
            teams.iter().find(|index| **index > at)
        } else {
            teams.iter().rev().find(|index| **index < at)
        }
        .copied()
        .unwrap_or(at)
    } else if forward {
        (at + PAGE_ROWS).min(drawn.len() - 1)
    } else {
        at.saturating_sub(PAGE_ROWS)
    };
    if next == at {
        return false;
    }
    view.selected = Some(drawn[next]);
    view.group.clear();
    true
}

/// The list's rows as drawn (the team selector and the filter applied)
/// as `Selection`s, and the selected row's index among them; `None` with
/// no session, no selection, or the selection not in the list.
fn list_and_position(view: &SaveView) -> Option<(Vec<Selection>, usize)> {
    let (Some(session), Some(selected)) = (view.session.as_ref(), view.selected) else {
        return None;
    };
    let drawn: Vec<Selection> =
        selection::list(session.file(), &view.team, &view.filter, view.team_category)
            .into_iter()
            .map(row_selection)
            .collect();
    drawn
        .iter()
        .position(|row| *row == selected)
        .map(|at| (drawn, at))
}

/// The `Selection` that names `row`.
fn row_selection(row: ListRow) -> Selection {
    match row {
        ListRow::Team(id) => Selection::Team(id),
        ListRow::Player(row) => Selection::Player(row.team_id, row.slot),
    }
}

/// How the list draws one row, computed once in `show`: the row's height,
/// its width, the list's own fill and whether player rows drag (no
/// filter typed).
#[derive(Clone, Copy)]
struct RowLayout {
    /// The row's height (`interact_size.y`, what a selectable occupies).
    row_height: f32,
    /// The span a row is laid out in, the rows' frame's width: the scroll
    /// area's viewport covers the scroll bar's strip while nothing scrolls,
    /// so the frame's width, not the viewport's, is a row's.
    width: f32,
    /// Player rows drag and take drops.
    draggable: bool,
}

/// The size `row_rect` lays a row out at: `layout`'s span less the list
/// frame's stroke on each side, its height `layout`'s. The held row's
/// float takes the same, so the float's name is cut where the row's is.
fn row_size(ui: &egui::Ui, layout: RowLayout) -> egui::Vec2 {
    let inset = ui.visuals().widgets.noninteractive.bg_stroke.width;
    egui::vec2(layout.width - 2.0 * inset, layout.row_height)
}

/// The box a list row occupies, the draft's `.prow` inside `.rows`'s
/// border: `layout`'s span inset by the list frame's stroke on each side.
/// A row's own edge, painted inside the box, then stands inside the frame's
/// border rather than under it — the frame's stroke is painted after the
/// rows, and a band sharing it would be covered. `id` is the row's own
/// (its team, and its slot for a player), not `allocate_rect`'s automatic
/// one: `show_rows` keeps an automatic id on its row only while every row
/// takes exactly one, so a second widget with an automatic id in a row
/// would hand a pressed or dragged row's state to its neighbour.
fn row_rect(
    ui: &mut egui::Ui,
    layout: RowLayout,
    id: egui::Id,
    sense: egui::Sense,
) -> (egui::Rect, egui::Response) {
    let inset = ui.visuals().widgets.noninteractive.bg_stroke.width;
    let avail = ui.available_rect_before_wrap();
    let rect = egui::Rect::from_min_size(avail.min + egui::vec2(inset, 0.0), row_size(ui, layout))
        .round_ui();
    ui.advance_cursor_after_rect(rect);
    let response = ui.interact(rect, id, sense);
    (rect, response)
}

/// The team selector: `All teams`, or the category's name in `All`'s
/// place (`4cc teams`, `VGL teams`, `Invitational teams`), then each cup
/// team of `category` by name in the save's order. With `category`, the
/// popup ends, under a separator, in the four radio buttons of
/// `TeamCategory::CHOICES` (the draft's `.combo-foot`): the teams' scroll
/// area above them is as tall as any dropdown's list, the footer's height
/// added to the popup's rather than taken from it, so the radios stay in
/// view while the teams scroll. Picking one leaves the popup open, so the
/// shorter list shows at once (the popup's `CloseOnClickOutside` keeps it
/// open on a click inside, and an entry's own `ui.close()` closes it on a
/// pick). `id_salt` is the combo's: two combos with one salt share their
/// popup state, so the comparator's filter passes its own (the player
/// list's stays `save_editor_team`).
pub(super) fn team_selector(
    ui: &mut egui::Ui,
    choice: &mut TeamChoice,
    category: Option<&mut TeamCategory>,
    file: &EditFile,
    id_salt: &str,
) {
    let mut chosen = *choice;
    let shown = category.as_deref().copied().unwrap_or(TeamCategory::All);
    // The first entry names the shown category, so a shortened list is
    // not read as the whole save.
    let all_label = format!("{} teams", shown.label());
    let text = match chosen {
        TeamChoice::All => all_label.clone(),
        TeamChoice::Team(id) => file
            .team(id)
            .map(|team| display_name(&team.name))
            .unwrap_or_else(|| id.to_string()),
    };
    // A `ComboBox`'s popup is egui's own scroll area, which reports no
    // taller than the `Area` around it, and an `Area` measures inside
    // `spacing.default_area_size` (600 x 400): the list and the radios
    // could not grow past 400 points. `Popup::menu` gives the same
    // popup with the contents laid out plainly, so the teams' area
    // takes `combo_height` and the footer adds under it. The button is
    // the one `ComboBox` draws: the selected text and a down triangle
    // at the right edge.
    let combo_id = ui.make_persistent_id(id_salt);
    let popup_id = combo_id.with("popup");
    let icon = egui::Atom::paint(egui::Vec2::splat(ui.spacing().icon_width), |ui, args| {
        ui.painter().add(egui::Shape::rotated_triangle(
            egui::Rect::from_center_size(
                args.rect.center(),
                egui::vec2(args.rect.width() * 0.7, args.rect.height() * 0.45),
            ),
            0.0,
            args.fallback_text_color,
        ));
    });
    let open = egui::Popup::is_id_open(ui.ctx(), popup_id);
    let response = egui::Button::new((
        text.as_str(),
        icon.atom_grow(true).atom_align(egui::Align2::RIGHT_CENTER),
    ))
    .min_size(egui::vec2(ui.spacing().combo_width, 0.0))
    .with_class_if(egui::ComboBox::CLASS_OPEN, open)
    .ui(ui);
    response.widget_info(|| {
        let mut info = egui::WidgetInfo::new(egui::Role::ComboBox);
        info.enabled = ui.is_enabled();
        info.current_text_value = Some(text.clone());
        info
    });
    let popup = egui::Popup::menu(&response)
        .id(popup_id)
        .width(response.rect.width())
        .close_behavior(if category.is_some() {
            egui::PopupCloseBehavior::CloseOnClickOutside
        } else {
            egui::PopupCloseBehavior::CloseOnClick
        })
        .show(|ui| {
            ui.set_min_width(ui.available_width());
            egui::ScrollArea::vertical()
                .max_height(ui.spacing().combo_height)
                .show(ui, |ui| {
                    // `ComboBox` keeps the labels from wrapping, the
                    // narrow popup notwithstanding.
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    if combo_item(ui, &mut chosen, TeamChoice::All, all_label.as_str()).clicked() {
                        ui.close();
                    }
                    for team in cup_teams(file) {
                        if shown.contains(team.id)
                            && combo_item(
                                ui,
                                &mut chosen,
                                TeamChoice::Team(team.id),
                                display_name(&team.name),
                            )
                            .clicked()
                        {
                            ui.close();
                        }
                    }
                });
            if let Some(category) = category {
                combo_separator(ui);
                // The menu's own widget visuals take the resting edge off,
                // a menu row's look; the radios are controls, so the block
                // draws on the theme's own again.
                ui.scope(|ui| {
                    ui.set_style(ui.ctx().global_style());
                    for button in TeamCategory::CHOICES {
                        radio_value(ui, category, button, button.label());
                    }
                });
            }
        });
    if popup.is_some() {
        ui.ctx().accesskit_node_builder(popup_id, |node| {
            node.set_role(egui::Role::ListBox);
        });
    }
    let teams = cup_teams(file)
        .filter(|team| shown.contains(team.id))
        .map(|team| TeamChoice::Team(team.id));
    let choices = std::iter::once(TeamChoice::All).chain(teams);
    // `wheel_steps`'s open-list guard looks for the `ComboBox` popup's
    // id (`<combo id>.with("popup")`): under `combo_id`, which the popup
    // is named from, it sees the open list and steps nothing while it
    // is up.
    let mut for_wheel = response.clone();
    for_wheel.id = combo_id;
    if let Some(next) = step_choice(choices, &chosen, wheel_steps(ui, &for_wheel)) {
        chosen = next;
    }
    *choice = chosen;
}

/// The selectable's frame both rows paint: `interact_selectable`'s
/// visuals, the fill drawn while the row is selected, hovered or
/// focused; the visuals returned for the row's text colour.
fn paint_row_frame(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    response: &egui::Response,
    is_selected: bool,
) -> egui::style::WidgetVisuals {
    let visuals = ui.style().interact_selectable(response, is_selected);
    if is_selected || response.hovered() || response.has_focus() {
        // A row is not a control, but the selection blue never fills
        // without its edge: at rest the selected row strokes the fill's
        // `edge_color`; hovered or focused it keeps the state's stroke.
        let stroke = if is_selected && studio_core::widgets::at_rest(ui, response) {
            egui::Stroke::new(
                1.0,
                studio_core::theme::edge_color(ui.visuals(), visuals.bg_fill),
            )
        } else {
            visuals.bg_stroke
        };
        ui.painter().rect(
            rect.expand(visuals.expansion),
            visuals.corner_radius,
            visuals.bg_fill,
            stroke,
            egui::StrokeKind::Inside,
        );
    }
    visuals
}

/// The view state a list row reads and writes, bundled as the fields
/// `show` hands every row together: `selected`, the row the view
/// names; `group`, the player rows selected together (`group_on`); and
/// `click_kind`, the copy target a press on a row clears.
struct RowSelection<'a> {
    /// The selected row.
    selected: &'a mut Option<Selection>,
    /// The player rows selected together, the selected one first.
    group: &'a mut Vec<(u32, u8)>,
    /// The Tactics selector the last press named for Ctrl+C and Ctrl+V.
    click_kind: &'a mut Option<ClickKind>,
}

/// A team's own row (the draft's `trow`): one click target the row's width
/// — the team's id in the weak colour, its colours 1 and 2 as small filled
/// squares (the shown colour, `channel_to_show`), the name with its colour
/// codes in the strong text colour, and a `●` at the
/// row's right end while the team's own entry has unsaved changes
/// (`Session::team_dirty`).
/// `mark`, while Live AATF holds this team's check, is the team's own
/// violations' glyph at the row's right end, after the dot. A click
/// selects the team
/// (the detail shows the team view); it is never a drag source nor a drop
/// target. Returns whether the row was double-clicked, which `show` turns
/// into the team picked in the selector with the filter cleared.
fn team_row(
    ui: &mut egui::Ui,
    selection: &mut RowSelection,
    session: &Session,
    team_id: u32,
    layout: RowLayout,
    mark: Option<live_marks::RowMark>,
) -> bool {
    let Some(team) = session.file().team(team_id) else {
        return false;
    };
    let is_selected = *selection.selected == Some(Selection::Team(team_id));
    let dirty = session.team_dirty(team_id);
    let (rect, response) = row_rect(
        ui,
        layout,
        ui.scope_id().with(("team_row", team_id)),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::Role::Button,
            ui.is_enabled(),
            is_selected,
            format!("{team_id} {}", display_name(&team.name)),
        )
    });
    let mark_response = if ui.is_rect_visible(rect) {
        let visuals = paint_row_frame(ui, rect, &response, is_selected);
        // A resting row (not selected, hovered or focused) paints its own
        // fill, which sets it apart from a player row (the draft's
        // `.trow` backdrop).
        let resting = !(is_selected || response.hovered() || response.has_focus());
        let row_fill = studio_core::theme::palette(ui.visuals()).team_row_fill;
        if resting {
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::ZERO, row_fill);
        }
        let padding = ui.spacing().button_padding.x;
        let centre = rect.center().y;
        let mut x = rect.min.x + padding;
        let weak = ui.visuals().weak_text_color();
        let font = egui::TextStyle::Body.resolve(ui.style());
        let id_galley = ui
            .painter()
            .layout_no_wrap(team_id.to_string(), font.clone(), weak);
        ui.painter().galley(
            egui::pos2(x, centre - id_galley.size().y / 2.0),
            id_galley.clone(),
            weak,
        );
        x += id_galley.size().x + ui.spacing().item_spacing.x;
        if let Some(colors) = &team.colors {
            for color in colors {
                let rgb = [
                    channel_to_show(color.red),
                    channel_to_show(color.green),
                    channel_to_show(color.blue),
                ];
                let swatch = egui::Rect::from_center_size(
                    egui::pos2(x + SWATCH / 2.0, centre),
                    egui::vec2(SWATCH, SWATCH),
                );
                ui.painter().rect_filled(
                    swatch,
                    ui.visuals().widgets.noninteractive.corner_radius,
                    egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                );
                x += SWATCH + 3.0;
            }
        }
        x += ui.spacing().item_spacing.x;
        let mut job = egui::text::LayoutJob::default();
        append_name(&mut job, &team.name, &font);
        // The end's room (the gap, the dot, the mark and what sits
        // between them) comes off the truncation width, so a long name
        // never runs under either.
        let mark_width = mark.as_ref().map(|mark| mark.width(ui));
        let dot = dirty_dot(ui, dirty);
        let room = end_room(ui, dot.as_ref().map(|galley| galley.size().x), mark_width);
        let text = if is_selected {
            visuals.fg_stroke.color
        } else {
            ui.visuals().strong_text_color()
        };
        job.wrap =
            egui::text::TextWrapping::truncate_at_width((rect.max.x - x - padding - room).max(0.0));
        let name_galley = ui.painter().layout_job(job);
        // The name's plain runs take the strong text colour, or the
        // selection's while the row is selected.
        ui.painter().galley(
            egui::pos2(x, centre - name_galley.size().y / 2.0),
            name_galley.clone(),
            text,
        );
        // The dot at the row's right end in the warning colour, one item
        // spacing plus `DOT_MARK_GAP` before the mark when the row has
        // one.
        if let Some(dot) = dot {
            let right = dot_right(ui, rect.max.x - padding, mark_width);
            ui.painter().galley(
                egui::pos2(right - dot.size().x, centre - dot.size().y / 2.0),
                dot,
                ui.visuals().warn_fg_color,
            );
        }
        mark.as_ref().map(|mark| {
            live_marks::paint_row_mark(ui, rect.shrink2(egui::vec2(padding, 0.0)), mark, team_id)
        })
    } else {
        None
    };
    let mark_hovered = mark_response.is_some_and(|mark| mark.hovered());
    let mut hover = display_name(&team.name);
    if dirty {
        hover.push_str("\nUnsaved changes");
    }
    hover.push_str("\nDouble-click to filter to this team");
    let response = if mark_hovered {
        response
    } else {
        response.on_hover_text(hover)
    };
    if response.clicked() {
        *selection.selected = Some(Selection::Team(team_id));
        selection.group.clear();
    }
    if pressed(ui, &response) {
        *selection.click_kind = None;
    }
    response.double_clicked()
}

/// The mark a drop on a player row draws: the row's rect, and the edge
/// the line stands on (`lower`: he comes from above). `player_row`
/// reports it; `show` paints it once after the rows.
struct DropMark {
    rect: egui::Rect,
    lower: bool,
}

/// The drag payload a held player row leaves: the dragged `Row` and the
/// pressed point's offset in it (`grab`), kept at the press — the row's
/// rect moves with the scroll after, so the offset cannot be read off it
/// later. `show` paints the float at the pointer less `grab`, wherever
/// the row has scrolled to; Esc drops the payload, which ends the drag
/// for the float, the mark and the release alike.
struct DraggedRow {
    row: Row,
    grab: egui::Vec2,
}

/// Whether `row`'s selection paint is on: an on group's every entry,
/// else the one selected row. `player_row` and the held row's float both
/// ask, so the float paints as its row does.
fn row_is_selected(selected: Option<Selection>, group: &[(u32, u8)], row: &Row) -> bool {
    if group_on(selected, group) {
        group.contains(&(row.team_id, row.slot))
    } else {
        selected == Some(Selection::Player(row.team_id, row.slot))
    }
}

/// One list row: place, position, and the name with its colour codes drawn.
/// The whole row is one click target; a click selects it, a Ctrl+click
/// (Cmd on a Mac) adds it to or takes it from the group of players
/// selected together (`toggle_in_group`). `draggable` (no
/// filter typed) also makes it a drag source and a drop target: a drop on
/// a row of the dragged player's own team moves him to it
/// (`Session::move_player`, SE-GUI-08), a drop on another team's row does
/// nothing. While a drag holds over the row and the drop would move a
/// record (`Session::moves_player`), the place it moves him to is marked
/// by a line 2 points thick in the selection colour, a serif at each end
/// (the draft's `.dropabove` and `.dropbelow`): on the row's lower edge
/// when he comes from above (the rows down to it move up and he lands
/// under them), on its upper edge when he comes from below. A line, not
/// an outline round the row, because the drop inserts him at that place
/// and shifts the rows between, where an outline would promise a swap; a
/// row the drop writes nothing on gets none. The row only reports the
/// mark: the list paints it once, after the rows, so the next row's
/// background never covers its lower half, and one mark at most shows.
/// A pointer on the edge two rows share hovers both, and the release
/// there goes to the upper row, the first to report. While held dragged,
/// the row's handle, place, position and name float above the list with
/// the point pressed under the pointer, painted by the list from the drag
/// payload (`DraggedRow`) so the float stays under the pointer while the
/// row scrolls out of view; its own place is left blank. The row shows
/// the pointing hand wherever the pointer is on it (egui leaves the
/// arrow over a row that can be dragged and clicked both, so the row
/// sets the pointing hand itself).
fn player_row(
    ui: &mut egui::Ui,
    selection: &mut RowSelection,
    session: &mut Session,
    row: &Row,
    layout: RowLayout,
    mark: Option<live_marks::RowMark>,
) -> Option<DropMark> {
    let file = session.file();
    let player = file.player(row.player_id);
    let dirty = session.player_dirty(row.team_id, row.slot);
    // A group's rows are filled and report selected as the selected row
    // does (the draft's `.on`): every entry of the on group, else the
    // one selected row.
    let is_selected = row_is_selected(*selection.selected, selection.group, row);
    let font = egui::TextStyle::Body.resolve(ui.style());
    let texts = row_text(player, row, &font);
    // The unsaved dot paints at the row's right end, never inside the
    // text: a row the name overflows keeps it. The label and the hover
    // keep its text.
    let mut label = format!("{} {} {}", texts.place, texts.position, texts.name.text);
    if dirty {
        label.push_str(" \u{25cf}");
    }
    // One line, truncated when the name is too long for the panel: the full
    // display name rides the hover, a dirty row's with the dot's text.
    // `mark`, while Live AATF holds this team's check, is his violations'
    // glyph at the row's right end; a dirty row's dot sits before it.
    let hover = player.map_or_else(|| "?".to_owned(), |player| display_name(&player.name));
    let sense = if layout.draggable {
        egui::Sense::click_and_drag()
    } else {
        egui::Sense::click()
    };
    let (rect, response) = row_rect(
        ui,
        layout,
        ui.scope_id().with(("player_row", row.team_id, row.slot)),
        sense,
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::Role::Button,
            ui.is_enabled(),
            is_selected,
            label.clone(),
        )
    });
    if layout.draggable {
        // The pressed point's offset in the row, kept at the press for
        // the drag payload: the row's rect moves with the scroll after,
        // so the offset cannot be read off it on a later frame.
        if response.is_pointer_button_down_on()
            && ui.input(|i| i.pointer.primary_pressed())
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            ui.ctx()
                .data_mut(|data| data.insert_temp(response.id.with("grab"), origin - rect.min));
        }
        let grab = ui
            .ctx()
            .data(|data| data.get_temp::<egui::Vec2>(response.id.with("grab")))
            .unwrap_or_default();
        // Set before `lifted` reads it below, so the payload names this
        // row on the drag's first frame: it is what the list paints the
        // float from.
        response.dnd_set_drag_payload(DraggedRow { row: *row, grab });
    }
    // While the row is held dragged the list paints its texts as the
    // float under the pointer; its place in the list is left blank, as
    // egui leaves a dragged lineup row's. The payload survives to the
    // end of the release frame's pass: the row paints at its place then,
    // the float skipped.
    let lifted = layout.draggable
        && egui::DragAndDrop::payload::<DraggedRow>(ui.ctx()).is_some_and(|dragged| {
            dragged.row.team_id == row.team_id && dragged.row.slot == row.slot
        })
        && !ui.input(|i| i.pointer.any_released());
    let mark_response = if !lifted && ui.is_rect_visible(rect) {
        // What `egui::Button::selectable` painted: the selectable's
        // visuals and the text row, drawn by hand so the mark can sit
        // at the right end with the text truncated before it.
        let visuals = paint_row_frame(ui, rect, &response, is_selected);
        let padding = ui.spacing().button_padding.x;
        // The end's room comes off the truncation width, so a long name
        // never runs under the dot or the mark (the dot paints at the
        // row's right end, as the team row's does).
        let (dot, mark_width, room) = end_layout(ui, dirty, mark.as_ref());
        paint_row_text(ui, ui.painter(), rect, texts, visuals.fg_stroke.color, room);
        // The dot at the row's right end in the warning colour, one item
        // spacing plus `DOT_MARK_GAP` before the mark when the row has
        // one.
        if let Some(dot) = dot {
            let right = dot_right(ui, rect.max.x - padding, mark_width);
            ui.painter().galley(
                egui::pos2(right - dot.size().x, rect.center().y - dot.size().y / 2.0),
                dot,
                ui.visuals().warn_fg_color,
            );
        }
        mark.as_ref().map(|mark| {
            live_marks::paint_row_mark(
                ui,
                rect.shrink2(egui::vec2(padding, 0.0)),
                mark,
                (row.team_id, row.slot),
            )
        })
    } else {
        None
    };
    let mark_hovered = mark_response.is_some_and(|mark| mark.hovered());
    let response = if mark_hovered {
        response
    } else if dirty {
        response.on_hover_text(format!("{hover}\nUnsaved changes"))
    } else {
        response.on_hover_text(hover)
    };
    if response.clicked() {
        // `command` is Ctrl on Windows and Linux, Cmd on a Mac (the
        // draft's `ctrlKey || metaKey`).
        if ui.input(|input| input.modifiers.command) {
            toggle_in_group(selection.selected, selection.group, row.team_id, row.slot);
        } else {
            *selection.selected = Some(Selection::Player(row.team_id, row.slot));
            selection.group.clear();
        }
    }
    if pressed(ui, &response) {
        *selection.click_kind = None;
    }
    let mut drop_mark = None;
    if layout.draggable {
        // `dnd_set_drag_payload` asks for the grab hand only on a drag
        // source that senses no click; the row takes clicks, so it asks
        // for the pointing hand itself.
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        // A place between two rows, not the row's border; only a row
        // whose release would move a record reports one, and only it may
        // take the payload: `dnd_release_payload` takes it from any row
        // under the pointer, and on the edge two rows share the row
        // above the marked one is drawn first.
        let accepts = response
            .dnd_hover_payload::<DraggedRow>()
            .filter(|dropped| {
                dropped.row.team_id == row.team_id
                    && session.moves_player(row.team_id, dropped.row.slot, row.slot)
            });
        if let Some(dropped) = &accepts {
            drop_mark = Some(DropMark {
                rect,
                lower: dropped.row.slot < row.slot,
            });
        }
        if accepts.is_some()
            && let Some(dropped) = response.dnd_release_payload::<DraggedRow>()
        {
            // The rows drawn before this one already painted the old
            // order, and no `request_repaint` is needed: the release is
            // input, after which egui draws the next frame anyway.
            session.move_player(row.team_id, dropped.row.slot, row.slot);
            // The selection follows the dragged record to its new slot.
            *selection.selected = Some(Selection::Player(row.team_id, row.slot));
            selection.group.clear();
        }
    }
    drop_mark
}

/// A Ctrl+click on a player row (the draft's `toggleInGroup`): the row
/// joins the players selected together — `selected`'s own first — or
/// leaves them. Leaving a group of two selects the remaining one alone,
/// a plain selection again; the row that is the only selected one does
/// nothing (one stays selected). With a team row selected the click
/// selects the player alone, as a plain click does.
fn toggle_in_group(
    selected: &mut Option<Selection>,
    group: &mut Vec<(u32, u8)>,
    team_id: u32,
    slot: u8,
) {
    // What the click toggles the row in or out of: the on group, else
    // the one selected player (a team row counts as none).
    let mut next = if group_on(*selected, group) {
        group.clone()
    } else {
        match *selected {
            Some(Selection::Player(team_id, slot)) => vec![(team_id, slot)],
            _ => Vec::new(),
        }
    };
    match next
        .iter()
        .position(|&(team, at)| team == team_id && at == slot)
    {
        Some(at) => {
            next.remove(at);
        }
        None => next.push((team_id, slot)),
    }
    let Some(&(team, slot)) = next.first() else {
        return;
    };
    *group = if next.len() > 1 { next } else { Vec::new() };
    *selected = Some(Selection::Player(team, slot));
}

/// `texts` painted inside `rect` on `painter`, as a player row and the
/// held row's float draw them alike: the `≡` handle `HANDLE_INSET` in
/// from the left edge in ink, the place the same past it (both in the
/// weak colour), then the position in `color` `COLUMN_GAP` later and
/// the name `POSITION_WIDTH` plus `COLUMN_GAP` later (its coded colours
/// kept), truncated to leave `room` and the row's padding at the right
/// end.
fn paint_row_text(
    ui: &egui::Ui,
    painter: &egui::Painter,
    rect: egui::Rect,
    texts: RowText,
    color: egui::Color32,
    room: f32,
) {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let padding = ui.spacing().button_padding.x;
    // The handle, then the place, placed by their ink, which starts a
    // glyph's side bearing in from its box: boxes alone left the place
    // a point off on the rows from 10 to 19.
    let weak = ui.visuals().weak_text_color();
    let handle = ui
        .painter()
        .layout_no_wrap("\u{2261}".to_owned(), font.clone(), weak);
    let handle_ink = ink_span(&handle);
    let handle_x = rect.min.x + HANDLE_INSET - handle_ink.min;
    let place_ink_x = handle_x + handle_ink.max + HANDLE_INSET;
    // The place in the weak colour, then the position and the name
    // in the row's text colour, each a column `COLUMN_GAP` after the
    // last (the draft's `.num`, `.pos` and `.pname`).
    let place = ui.painter().layout_no_wrap(texts.place, font.clone(), weak);
    let place_x = place_ink_x - ink_span(&place).min;
    let position_x = place_x + place.size().x + COLUMN_GAP;
    let position = ui
        .painter()
        .layout_no_wrap(texts.position.to_owned(), font, color);
    let name_x = position_x + POSITION_WIDTH + COLUMN_GAP;
    let mut name = texts.name;
    name.wrap = egui::text::TextWrapping::truncate_at_width(
        (rect.max.x - name_x - padding - room).max(0.0),
    );
    let name = ui.painter().layout_job(name);
    painter.galley(
        egui::pos2(handle_x, rect.center().y - handle.size().y / 2.0),
        handle,
        color,
    );
    for (x, galley) in [(place_x, place), (position_x, position), (name_x, name)] {
        painter.galley(
            egui::pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
    }
}

/// A player row's three columns' text.
struct RowText {
    /// His place in the roster, the slot's `01` to `23`: the `NN` of his
    /// id and of his export's `NN - Name` folder, not his shirt number
    /// (General info shows that). Two digits on every row, so the places
    /// line up with no column kept to right-align them.
    place: String,
    /// His registered position's label, `?` when the slot holds no
    /// player.
    position: &'static str,
    /// The name's runs, each colored as `name_spans` gives it (`None`
    /// runs the normal text color, `append_name`); `?` when the slot
    /// holds no player.
    name: egui::text::LayoutJob,
}

/// The row's place, position and name for `row`'s player, the name's
/// runs at `font`.
fn row_text(player: Option<&PlayerEntry>, row: &Row, font: &egui::FontId) -> RowText {
    let position = player
        .and_then(|player| POSITIONS.get(usize::from(player.positions.registered)))
        .copied()
        .unwrap_or("?");
    let mut name = egui::text::LayoutJob::default();
    match player {
        Some(player) => append_name(&mut name, &player.name, font),
        None => name.append("?", 0.0, plain_format(font.clone())),
    }
    RowText {
        place: format!("{:02}", u32::from(row.slot) + 1),
        position,
        name,
    }
}
