//! The toolbar's list of unsaved changes (SE-GUI-43, the draft's
//! `unsavedLabel` and `changesPopup`): the `• unsaved changes` label, and
//! the popup a click on it opens under it, holding the close prompt's
//! table of changes and none of its answers, each row ended by an X that
//! puts that change back alone (SE-GUI-74, the draft's `changesTable`)
//! and its item a switch to what the row names.

use pes_savefile::EditFile;
use team_widgets::violations::{SWITCH_TOOLTIP, SelectedPlayer};

use crate::session::PutBack;
use crate::view::prompt::{self, UnsavedRow};
use crate::view::{Refusal, SaveView, aatf_check, comparator};

/// The label's tooltip.
const TOOLTIP: &str = "Lists the changes since the save was opened or last saved";

/// The tooltip of a row's X.
const X_TOOLTIP: &str = "Undoes this change alone";

/// The item's tooltip when a click switches to the row's team.
const TEAM_SWITCH_TOOLTIP: &str = "Click to switch to this team";

/// The open list's state on `SaveView`: its rows, the click targets of
/// their items and the session revision they were computed at.
pub(super) struct ChangesList {
    /// The `Session::revision` the rows were computed at. An edit, an undo
    /// or a redo moves it, and the list closes then, since its rows would
    /// go stale under it; a row's X computes them again instead.
    revision: u64,
    /// The table's rows, `prompt::rows`' own.
    rows: Vec<UnsavedRow>,
    /// Where a click on each row's item switches, aligned with `rows`:
    /// `None` when the row names nothing the list can switch to.
    switches: Vec<Option<Switch>>,
}

/// Where a click on a row's item switches the view (the draft's `to`):
/// the row's player, or its team for a team's own or a `Player order`
/// row.
#[derive(Clone, Copy)]
enum Switch {
    /// Selects the player on his roster slot (`aatf_check::select_player`).
    Player(SelectedPlayer),
    /// Selects the team's own row (`aatf_check::select_team`).
    Team(u32),
}

impl Switch {
    /// The item's hover text: where the click goes.
    fn tooltip(self) -> &'static str {
        match self {
            Switch::Player(_) => SWITCH_TOOLTIP,
            Switch::Team(_) => TEAM_SWITCH_TOOLTIP,
        }
    }
}

/// The label while the open save has unsaved changes, in the link blue
/// (the weak grey hid the one piece saying the save differs from its file
/// among the toolbar's quieter text), and the list under it while open.
/// A click on the label opens or closes the list; a click outside it, Esc,
/// an edit, an undo or a redo closes it, and so does saving, which takes
/// the label away. A row's X puts its change back and keeps the list open,
/// a row shorter, closing it with the last row; a row's item switches to
/// what the row names and keeps it open for the next row.
pub(super) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let Some(revision) = view
        .session
        .as_ref()
        .filter(|session| session.is_dirty())
        .map(|session| session.revision())
    else {
        view.changes_list = None;
        return;
    };
    if view
        .changes_list
        .as_ref()
        .is_some_and(|list| list.revision != revision)
    {
        view.changes_list = None;
    }
    let text = egui::RichText::new("\u{2022} unsaved changes").color(ui.visuals().hyperlink_color);
    let label = ui
        .add(egui::Label::new(text).sense(egui::Sense::click()))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(TOOLTIP);
    if label.clicked() {
        view.changes_list = match view.changes_list.take() {
            Some(_) => None,
            None => open(view, revision),
        };
    }
    let Some(list) = view.changes_list.as_ref() else {
        return;
    };
    // The draft's `.chpop`: the panel's fill and stroke, under the label
    // at its left edge. Only that one placement, so a list wider than the
    // room left of the window's edge slides left rather than flipping to
    // the label's right edge. The draft stops it 8 points inside the
    // window's edge, where egui's own constraint would stop it flush, so
    // the anchor slides by the overhang of the list's width as last drawn
    // (on the frame it opens, egui's sizing pass draws nothing, so the
    // label's left edge stands then).
    let popup_id = egui::Popup::default_response_id(&label);
    let width = ui
        .ctx()
        .memory(|memory| memory.area_rect(popup_id))
        .map_or(0.0, |rect| rect.width());
    let right_limit = ui.ctx().content_rect().right() - 8.0;
    let anchor = egui::pos2(
        label.rect.left().min(right_limit - width),
        label.rect.bottom(),
    );
    let frame = egui::Frame::popup(ui.style())
        .fill(ui.visuals().panel_fill)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(ui.visuals().menu_corner_radius)
        .inner_margin(8);
    // The draft's 460 to 640 points measure the whole box: the content gets
    // what the frame's margins and stroke leave of them.
    let chrome = frame.total_margin().sum().x;
    // Each row's item click target's hover text: the cells that offer a
    // switch.
    let tips: Vec<Option<&'static str>> = list
        .switches
        .iter()
        .map(|switch| switch.map(Switch::tooltip))
        .collect();
    let mut click = None;
    let shown = egui::Popup::from_response(&label)
        .anchor(egui::PopupAnchor::Position(anchor))
        .align_alternatives(&[])
        .gap(3.0)
        .frame(frame)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(460.0 - chrome);
            ui.set_max_width(640.0 - chrome);
            click = table_with_xs(ui, &list.rows, &tips, 460.0 - chrome);
        });
    if shown.is_some_and(|shown| shown.response.should_close()) {
        view.changes_list = None;
    }
    match click {
        Some(ListClick::PutBack(index)) => put_back_row(view, index),
        Some(ListClick::Switch(index)) => switch_to(view, index),
        None => {}
    }
}

/// A click inside the list's table.
enum ListClick {
    /// A row's X: puts that change back alone.
    PutBack(usize),
    /// A row's item: switches to what the row names.
    Switch(usize),
}

/// The list's table with an X ending each row under an `Undo` head (the
/// draft's `changesTable` with `undoable`), scrolling up and down past
/// 240 points; the click inside it this frame. The X's and their head
/// stand in a column right of the sideways scroll area, inside the one
/// that scrolls up and down, each on its row's first line: a wide row
/// scrolls sideways under the column, so neither the head nor an X
/// leaves the view. A list narrower than `min_width` (the popup's floor)
/// keeps the column at its right edge, and each separator line is
/// carried on under the column, so it runs the whole row.
fn table_with_xs(
    ui: &mut egui::Ui,
    rows: &[UnsavedRow],
    tips: &[Option<&'static str>],
    min_width: f32,
) -> Option<ListClick> {
    let error = ui.visuals().error_fg_color;
    let separator = ui.visuals().widgets.noninteractive.bg_stroke;
    // The head, drawn as the table's other heads are (the strong text
    // colour); its width sets the column's.
    let galley =
        egui::WidgetText::from(egui::RichText::new("Undo").color(ui.visuals().strong_text_color()))
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Body,
            );
    let x_width = ui.spacing().interact_size.y.max(galley.size().x);
    egui::ScrollArea::vertical()
        .max_height(240.0)
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                let left = ui.cursor().min.x;
                let spacing = ui.spacing().item_spacing.x;
                let table_width = (ui.available_width() - x_width - spacing).max(0.0);
                let table = egui::ScrollArea::horizontal()
                    .id_salt("save_editor_changes_list_sideways")
                    .max_width(table_width)
                    .show(ui, |ui| {
                        prompt::changes_table(
                            ui,
                            "save_editor_changes_list",
                            rows.iter().map(|row| &row.cells),
                            Some(tips),
                        )
                    });
                let column = ui.cursor().min.x.max(left + min_width - x_width);
                // The head centred in the column, on the header row's
                // line (its galley centred in a rect of its own size).
                ui.put(
                    egui::Rect::from_center_size(
                        egui::pos2(column + 0.5 * x_width, table.inner.head.center().y),
                        galley.size(),
                    ),
                    egui::Label::new(egui::WidgetText::Galley(galley.clone())),
                );
                // Each separator line carried on under the column: drawn
                // from the row's left edge so the stretch under the table
                // (which the sideways scroll clips) meets the table's own.
                for y in &table.inner.lines {
                    ui.painter()
                        .hline(table.inner.head.left()..=column + x_width, *y, separator);
                }
                let mut click = table.inner.clicked.map(ListClick::Switch);
                for (index, item) in table.inner.items.iter().enumerate() {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(column, item.top()),
                        egui::vec2(x_width, item.height()),
                    );
                    let x =
                        egui::Button::new(egui::RichText::new("\u{d7}").color(error)).frame(false);
                    if ui.put(rect, x).on_hover_text(X_TOOLTIP).clicked() {
                        click = Some(ListClick::PutBack(index));
                    }
                }
                click
            })
            .inner
        })
        .inner
}

/// The item of the list's row `index` clicked: a switch to what the row
/// names — its player, or its team for a team's own or a `Player order`
/// row — the list staying open, so a member can go through the changes
/// one by one.
fn switch_to(view: &mut SaveView, index: usize) {
    let Some(session) = view.session.as_ref() else {
        return;
    };
    let Some(switch) = view
        .changes_list
        .as_ref()
        .and_then(|list| list.switches.get(index))
        .copied()
        .flatten()
    else {
        return;
    };
    match switch {
        Switch::Player(player) => {
            // A row drawn while another save opened can name a team that
            // save lacks: its click does nothing.
            let Some(entry) = session.file().team(player.team_id) else {
                return;
            };
            aatf_check::select_player(
                player,
                &mut view.selected,
                &mut view.team,
                &mut view.filter,
                &entry.tactics,
                &mut view.tactics,
            );
        }
        Switch::Team(team_id) => aatf_check::select_team(
            team_id,
            &mut view.selected,
            &mut view.team,
            &mut view.filter,
        ),
    }
}

/// The X of the list's row `index` clicked: its change put back as one
/// undo step, then the list's rows computed again so it stays open a row
/// shorter, or closed once the save reads clean. A row the session cannot
/// put back (stale under it, which an open list never is) shows as a
/// refusal.
fn put_back_row(view: &mut SaveView, index: usize) {
    let (Some(session), Some(list)) = (view.session.as_mut(), view.changes_list.as_ref()) else {
        return;
    };
    if let Err(error) = session.put_back(&list.rows[index].put_back) {
        view.refusal = Some(Refusal {
            title: "Cannot undo this change",
            message: format!("{error:#}"),
        });
        view.changes_list = None;
        return;
    }
    let (dirty, revision) = (session.is_dirty(), session.revision());
    view.changes_list = if dirty { open(view, revision) } else { None };
}

/// The list at `revision`, its rows computed now and never per frame: the
/// comparison walks every player and team of the save, a cost to pay once
/// per opening, not once per frame the list stays up. A codec error,
/// impossible on a save read whole, shows as a refusal rather than an
/// empty list.
fn open(view: &mut SaveView, revision: u64) -> Option<ChangesList> {
    let session = view.session.as_ref()?;
    match prompt::rows(session) {
        Ok(rows) => Some(ChangesList {
            revision,
            switches: switches(session.file(), &rows),
            rows,
        }),
        Err(error) => {
            view.refusal = Some(Refusal {
                title: "Cannot compare the save",
                message: format!("{error:#}"),
            });
            None
        }
    }
}

/// Each row's click target, computed with the rows (`open`), never per
/// frame: a player row selects him where a roster holds him (`None`: no
/// roster does, so the item is no click target), a team's own row selects
/// it, and a `Player order` row selects the team rostering the group's
/// first player.
fn switches(file: &EditFile, rows: &[UnsavedRow]) -> Vec<Option<Switch>> {
    let roster = comparator::roster_of(file);
    rows.iter()
        .map(|row| match &row.put_back {
            PutBack::Player { id, .. } => {
                roster.get(id).map(|place| Switch::Player(place.select()))
            }
            PutBack::Team { id, .. } => Some(Switch::Team(*id)),
            PutBack::Order(places) => places
                .first()
                .and_then(|first| roster.get(&first.id))
                .map(|place| Switch::Team(place.select().team_id)),
        })
        .collect()
}
