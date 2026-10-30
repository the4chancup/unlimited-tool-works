//! The master list: the team selector, the cross-team filter box, one team's row
//! heading its players' rows, and the quick actions under the list. A player row
//! shows the name drawn in the colors its savefile codes set; a team row the
//! team's id, its colours 1-2 as swatches and its name.

use pes_savefile::EditFile;
use pes_savefile::interchange::team_toml::labels::POSITIONS;
use pes_savefile::model::PlayerEntry;
use pes_savefile::model::names::display_name;
use studio_core::widgets::{ink_span, step_choice, text_box, wheel_steps};
use team_widgets::names::{append_name, plain_format};

use crate::session::Session;
use crate::session::selection::{self, ListRow, Row, TeamChoice};
use crate::view::team::channel_to_show;
use crate::view::{SaveView, Selection, live_marks, quick_actions};

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

/// The selector, the filter box, the rows and the quick actions, top to
/// bottom of the side panel: the list's scroll area takes the height left
/// above the actions. Returns the right edge of the rows' frame, where the
/// player view counts its first gap from.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView) -> f32 {
    // egui's solid scroll bar stands outside the rows' frame, in a strip
    // (its 4-point gap and 6-point bar) kept at the column's right whether
    // or not the list scrolls, so a filter leaving few rows keeps the
    // list's width. A floating bar lies over the rows, where a row's dot
    // and mark at its right end take their hover and click. The controls
    // above and below end at the frame's edge too, so the column reads as
    // one box.
    let scroll = egui::style::ScrollStyle::solid();
    let frame_width = ui.available_width() - scroll.allocated_width();
    let frame_right = ui.max_rect().min.x + frame_width;
    let Some(session) = view.session.as_ref() else {
        return frame_right;
    };
    let team_before = view.team;
    ui.scope(|ui| {
        ui.spacing_mut().combo_width = frame_width;
        team_selector(ui, &mut view.team, session.file(), "save_editor_team");
    });
    // Picking a team selects its team row, so the detail shows that team's
    // view, its tactics with it: a manager who picks a team usually wants
    // its tactics next. Picking ALL keeps the selection.
    if view.team != team_before
        && let TeamChoice::Team(id) = view.team
    {
        view.selected = Some(Selection::Team(id));
    }
    ui.add(text_box(
        egui::TextEdit::singleline(&mut view.filter)
            .hint_text("Filter every team (name, shirt name or id)")
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
    // list or under ALL where each team's rows run unbroken under its team
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
        live_marks::held_violations(view.live_aatf, &view.live.check, &active.key)
    });
    let session = view.session.as_mut().expect("checked above");
    let rows = selection::list(session.file(), &view.team, &view.filter);
    // A page key moved the selection: its row's index, scrolled to below.
    let reveal = std::mem::take(&mut view.reveal_selected)
        .then(|| {
            rows.iter()
                .position(|row| Some(row_selection(*row)) == view.selected)
        })
        .flatten();
    ui.spacing_mut().scroll = scroll;
    // The rows touch, as the draft's do: `show_rows` adds the item
    // spacing's 3 points under each row, which made them 21 apart.
    ui.spacing_mut().item_spacing.y = 0.0;
    // The frame's fill goes under the rows, painted once the scroll area
    // has its rect.
    let background = ui.painter().add(egui::Shape::Noop);
    // Not shrunk: the frame takes the panel's height whatever few rows a
    // filter leaves. The scope clears the extreme background, which a
    // solid bar's track paints in at full opacity: the draft's `.vbar` has
    // no track, the panel shows through. The rows' frame reads the colour
    // outside the scope.
    let output = ui
        .scope(|ui| {
            ui.visuals_mut().extreme_bg_color = egui::Color32::TRANSPARENT;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show_rows(ui, layout.row_height, rows.len(), |ui, range| {
                    if let Some(index) = reveal {
                        // `show_rows` draws only the rows in view, so the
                        // row's rect comes from its index, the first drawn
                        // row standing at the cursor; egui scrolls the
                        // least that shows it, at once, so a held key's
                        // repeats never outrun an animation.
                        let top = ui.cursor().top()
                            + (index as f32 - range.start as f32) * layout.row_height;
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(ui.cursor().left(), top),
                            egui::vec2(layout.width, layout.row_height),
                        );
                        ui.scroll_to_rect_animation(
                            rect,
                            None,
                            egui::style::ScrollAnimation::none(),
                        );
                    }
                    for row in &rows[range] {
                        match row {
                            ListRow::Team(team_id) => {
                                team_row(
                                    ui,
                                    &mut view.selected,
                                    session,
                                    *team_id,
                                    layout,
                                    held.filter(|(id, _)| *id == *team_id).map(
                                        |(_, violations)| live_marks::row_mark(violations, None),
                                    ),
                                );
                            }
                            ListRow::Player(row) => {
                                player_row(
                                    ui,
                                    &mut view.selected,
                                    session,
                                    row,
                                    layout,
                                    held.filter(|(id, _)| *id == row.team_id).map(
                                        |(_, violations)| {
                                            live_marks::row_mark(violations, Some(row.slot))
                                        },
                                    ),
                                );
                            }
                        }
                    }
                })
        })
        .inner;
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
    frame_right
}

/// Selects the player row after the selected row in the list as drawn
/// (the team selector and the filter applied), or the one before it when
/// `forward` is false. Team rows are skipped; at either end of the list,
/// or with the selection not in it, nothing moves. Answers whether the
/// selection moved.
pub(super) fn step_player(view: &mut SaveView, forward: bool) -> bool {
    let (Some(session), Some(selected)) = (view.session.as_ref(), view.selected) else {
        return false;
    };
    let rows: Vec<Selection> = selection::list(session.file(), &view.team, &view.filter)
        .into_iter()
        .map(row_selection)
        .collect();
    let Some(at) = rows.iter().position(|row| *row == selected) else {
        return false;
    };
    let player = |row: &&Selection| matches!(row, Selection::Player(..));
    let next = if forward {
        rows[at + 1..].iter().find(player)
    } else {
        rows[..at].iter().rev().find(player)
    };
    let Some(next) = next else {
        return false;
    };
    view.selected = Some(*next);
    true
}

/// The `Selection` that names `row`.
fn row_selection(row: ListRow) -> Selection {
    match row {
        ListRow::Team(id) => Selection::Team(id),
        ListRow::Player(row) => Selection::Player(row.team_id, row.slot),
    }
}

/// How the list draws one row, computed once in `show`: the row's height,
/// its width and whether player rows drag (no filter typed).
#[derive(Clone, Copy)]
struct RowLayout {
    /// The row's height (`interact_size.y`, what a selectable occupies).
    row_height: f32,
    /// The row's width, the frame's: the scroll area's own spans the bar's
    /// strip while nothing scrolls.
    width: f32,
    /// Player rows drag and take drops.
    draggable: bool,
}

/// The team selector: `ALL`, then each team by name in the save's order.
/// `id_salt` is the combo's: two combos with one salt share their popup
/// state, so the comparator's filter passes its own (the player list's
/// stays `save_editor_team`).
pub(super) fn team_selector(
    ui: &mut egui::Ui,
    choice: &mut TeamChoice,
    file: &EditFile,
    id_salt: &str,
) {
    let mut chosen = *choice;
    let combo = egui::ComboBox::from_id_salt(id_salt)
        .selected_text(match chosen {
            TeamChoice::All => "ALL".to_owned(),
            TeamChoice::Team(id) => file
                .team(id)
                .map(|team| display_name(&team.name))
                .unwrap_or_else(|| id.to_string()),
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut chosen, TeamChoice::All, "ALL");
            for team in file.teams() {
                ui.selectable_value(
                    &mut chosen,
                    TeamChoice::Team(team.id),
                    display_name(&team.name),
                );
            }
        })
        .response;
    let teams = file.teams().iter().map(|team| TeamChoice::Team(team.id));
    let choices = std::iter::once(TeamChoice::All).chain(teams);
    if let Some(next) = step_choice(choices, &chosen, wheel_steps(ui, &combo)) {
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
        ui.painter().rect(
            rect.expand(visuals.expansion),
            visuals.corner_radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    visuals
}

/// A team's own row (the draft's `trow`): one click target the row's width
/// — the team's id in the weak colour, its colours 1 and 2 as small filled
/// squares (the shown colour, `channel_to_show`), the name with its colour
/// codes in the strong text colour, `[click for tactics]` after it in the
/// weak colour (the hint is elided before the name is), and a `●` at the
/// row's right end while the team's own entry has unsaved changes
/// (`Session::team_dirty`).
/// `mark`, while Live AATF holds this team's check, is the team's own
/// violations' glyph at the row's right end, after the dot. A click
/// selects the team
/// (the detail shows the team view); it is never a drag source nor a drop
/// target.
fn team_row(
    ui: &mut egui::Ui,
    selected: &mut Option<Selection>,
    session: &Session,
    team_id: u32,
    layout: RowLayout,
    mark: Option<live_marks::RowMark>,
) {
    let Some(team) = session.file().team(team_id) else {
        return;
    };
    let is_selected = *selected == Some(Selection::Team(team_id));
    let dirty = session.team_dirty(team_id);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(layout.width, layout.row_height),
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
        if !(is_selected || response.hovered() || response.has_focus()) {
            // The faint fill sets the row apart from a player row (the
            // draft's `.trow` backdrop).
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::ZERO, ui.visuals().faint_bg_color);
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
                    egui::CornerRadius::same(1),
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
        // `[click for tactics]` after the name, in the weak colour at the
        // normal weight (the draft's `.thint`): what a click on the row
        // opens. It takes only the room the name left, so it is elided
        // before the name is. An elided name ends a few points short of
        // its room (the cut falls between letters), so it is the elision,
        // not the room left, that drops the hint: a name filling the row
        // keeps it to itself.
        let hint_x = x + name_galley.size().x + ui.spacing().item_spacing.x;
        let hint_room = rect.max.x - padding - room - hint_x;
        if !name_galley.elided && hint_room > 0.0 {
            let mut hint = egui::text::LayoutJob::default();
            hint.append(
                "[click for tactics]",
                0.0,
                egui::TextFormat::simple(font, weak),
            );
            hint.wrap = egui::text::TextWrapping::truncate_at_width(hint_room);
            let hint_galley = ui.painter().layout_job(hint);
            ui.painter().galley(
                egui::pos2(hint_x, centre - hint_galley.size().y / 2.0),
                hint_galley,
                weak,
            );
        }
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
    let hover = display_name(&team.name);
    let response = if mark_hovered {
        response
    } else if dirty {
        response.on_hover_text(format!("{hover}\nUnsaved changes"))
    } else {
        response.on_hover_text(hover)
    };
    if response.clicked() {
        *selected = Some(Selection::Team(team_id));
    }
}

/// One list row: place, position, and the name with its colour codes drawn.
/// The whole row is one click target; a click selects it. `draggable` (no
/// filter typed) also makes it a drag source and a drop target: a drop on
/// a row of the dragged player's own team moves him to it
/// (`Session::move_player`, SE-GUI-08), a drop on another team's row does
/// nothing.
fn player_row(
    ui: &mut egui::Ui,
    selected: &mut Option<Selection>,
    session: &mut Session,
    row: &Row,
    layout: RowLayout,
    mark: Option<live_marks::RowMark>,
) {
    let file = session.file();
    let player = file.player(row.player_id);
    let dirty = session.player_dirty(row.team_id, row.slot);
    let is_selected = *selected == Some(Selection::Player(row.team_id, row.slot));
    let row_height = layout.row_height;
    let font = egui::TextStyle::Body.resolve(ui.style());
    let RowText {
        place,
        position,
        mut name,
    } = row_text(player, row, &font);
    // The unsaved dot paints at the row's right end, never inside the
    // text: a row the name overflows keeps it. The label and the hover
    // keep its text.
    let mut label = format!("{place} {position} {}", name.text);
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
    let (rect, response) = ui.allocate_exact_size(egui::vec2(layout.width, row_height), sense);
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::Role::Button,
            ui.is_enabled(),
            is_selected,
            label.clone(),
        )
    });
    let mark_response = if ui.is_rect_visible(rect) {
        // What `egui::Button::selectable` painted: the selectable's
        // visuals and the text row, drawn by hand so the mark can sit
        // at the right end with the text truncated before it.
        let visuals = paint_row_frame(ui, rect, &response, is_selected);
        let padding = ui.spacing().button_padding.x;
        let mark_width = mark.as_ref().map(|mark| mark.width(ui));
        // The end's room comes off the truncation width, so a long name
        // never runs under the dot or the mark (the dot paints at the
        // row's right end, as the team row's does).
        let dot = dirty_dot(ui, dirty);
        let room = end_room(ui, dot.as_ref().map(|galley| galley.size().x), mark_width);
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
        let place = ui.painter().layout_no_wrap(place, font.clone(), weak);
        let place_x = place_ink_x - ink_span(&place).min;
        let position_x = place_x + place.size().x + COLUMN_GAP;
        let position =
            ui.painter()
                .layout_no_wrap(position.to_owned(), font, visuals.fg_stroke.color);
        let name_x = position_x + POSITION_WIDTH + COLUMN_GAP;
        name.wrap = egui::text::TextWrapping::truncate_at_width(
            (rect.max.x - name_x - padding - room).max(0.0),
        );
        let name = ui.painter().layout_job(name);
        ui.painter().galley(
            egui::pos2(handle_x, rect.center().y - handle.size().y / 2.0),
            handle,
            visuals.fg_stroke.color,
        );
        for (x, galley) in [(place_x, place), (position_x, position), (name_x, name)] {
            ui.painter().galley(
                egui::pos2(x, rect.center().y - galley.size().y / 2.0),
                galley,
                visuals.fg_stroke.color,
            );
        }
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
        *selected = Some(Selection::Player(row.team_id, row.slot));
    }
    if layout.draggable {
        response.dnd_set_drag_payload(*row);
        if let Some(dropped) = response.dnd_release_payload::<Row>()
            && dropped.team_id == row.team_id
        {
            session.move_player(row.team_id, dropped.slot, row.slot);
            // The selection follows the dragged record to its new slot.
            *selected = Some(Selection::Player(row.team_id, row.slot));
        }
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
