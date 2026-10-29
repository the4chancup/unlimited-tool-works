//! The toolbar's list of unsaved changes (SE-GUI-43, the draft's
//! `unsavedLabel` and `changesPopup`): the `• unsaved changes` label, and
//! the popup a click on it opens under it, holding the close prompt's
//! table of changes and none of its answers.

use crate::view::prompt::{self, ChangeRow};
use crate::view::{Refusal, SaveView};

/// The label's tooltip.
const TOOLTIP: &str = "Lists the changes since the save was opened or last saved";

/// The open list's state on `SaveView`: its rows and the session revision
/// they were computed at.
pub(super) struct ChangesList {
    /// The `Session::revision` the rows were computed at. An edit, an undo
    /// or a redo moves it, and the list closes then, since its rows would
    /// go stale under it.
    revision: u64,
    /// The table's rows, `prompt::rows`' own.
    rows: Vec<ChangeRow>,
}

/// The label while the open save has unsaved changes, in the link blue
/// (the weak grey hid the one piece saying the save differs from its file
/// among the toolbar's quieter text), and the list under it while open.
/// A click on the label opens or closes the list; a click outside it, Esc,
/// an edit, an undo or a redo closes it, and so does saving, which takes
/// the label away.
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
        .corner_radius(6)
        .inner_margin(8);
    // The draft's 460 to 640 points measure the whole box: the content gets
    // what the frame's margins and stroke leave of them.
    let chrome = frame.total_margin().sum().x;
    let shown = egui::Popup::from_response(&label)
        .anchor(egui::PopupAnchor::Position(anchor))
        .align_alternatives(&[])
        .gap(3.0)
        .frame(frame)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(460.0 - chrome);
            ui.set_max_width(640.0 - chrome);
            egui::ScrollArea::both().max_height(240.0).show(ui, |ui| {
                prompt::changes_table(ui, "save_editor_changes_list", &list.rows, false);
            });
        });
    if shown.is_some_and(|shown| shown.response.should_close()) {
        view.changes_list = None;
    }
}

/// The list at `revision`, its rows computed now and never per frame: the
/// comparison walks every player and team of the save, a cost to pay once
/// per opening, not once per frame the list stays up. A codec error,
/// impossible on a save read whole, shows as a refusal rather than an
/// empty list.
fn open(view: &mut SaveView, revision: u64) -> Option<ChangesList> {
    let session = view.session.as_ref()?;
    match prompt::rows(session) {
        Ok(rows) => Some(ChangesList { revision, rows }),
        Err(error) => {
            view.refusal = Some(Refusal {
                title: "Cannot compare the save",
                message: format!("{error:#}"),
            });
            None
        }
    }
}
