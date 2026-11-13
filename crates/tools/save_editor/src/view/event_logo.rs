//! The active ruleset's event logo at the view's top right (the draft's
//! `.event-logo`), the sidebar clover mirrored: drawn past the toolbar
//! band's end, moving nothing, and with an event page a link to it.

use team_widgets::event_logo::HEIGHT;

use super::SaveView;

/// The logo's right edge from the window's, in points: as far as the
/// clover's ink stands from the left. The toolbar's band ends as far left
/// of the logo, so the logo has the same room on both sides.
pub(super) const RIGHT: f32 = 14.0;

/// The logo's top below the view's, in points: as far as the clover's ink
/// stands below the window's top.
const TOP: f32 = 8.0;

/// Where the logo stands while a save is open and the active ruleset
/// carries one: 28 points high, its right edge `RIGHT` from the window's,
/// its top `TOP` below the view's. It depends on the window and the
/// picture alone, so the toolbar's band can end short of it before either
/// is drawn.
pub(super) fn rect(ui: &egui::Ui, view: &SaveView) -> Option<egui::Rect> {
    view.session.as_ref()?;
    let size = view.rules.as_ref()?.logo.as_ref()?.size();
    let right = ui.ctx().content_rect().max.x - RIGHT;
    // `ui` is the shell's central panel inside its margin: the view's top
    // is the panel's.
    let top = ui.max_rect().top()
        - f32::from(egui::Frame::central_panel(ui.style()).inner_margin.top)
        + TOP;
    Some(egui::Rect::from_min_max(
        egui::pos2(right - size.x, top),
        egui::pos2(right, top + HEIGHT),
    ))
}

/// Draws the logo where `rect` places it, if anywhere: the rect the
/// toolbar's band already ended short of, computed before the toolbar
/// ran, so a save the toolbar opened this frame shows it on the next.
pub(super) fn show(ui: &mut egui::Ui, view: &mut SaveView, rect: Option<egui::Rect>) {
    let Some(rect) = rect else {
        return;
    };
    let Some(active) = view.rules.as_mut() else {
        return;
    };
    let Some(logo) = active.logo.as_mut() else {
        return;
    };
    let ruleset = active.rules.ruleset();
    // A child placed by rect allocates nothing in `ui`, so no row moves.
    // Right to left, centred: a picture wider than 3:1 is less than 28
    // high and sits in the middle of the box (`object-position: right`).
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    let image = logo.image(ui.ctx()).alt_text(&ruleset.name);
    match &ruleset.page {
        Some(page) => {
            let response = child
                .add(image.sense(egui::Sense::click()))
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(format!(
                    "{}\n\nClick to open the event's page:\n{page}",
                    ruleset.name
                ));
            if response.clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(page));
            }
        }
        None => {
            child.add(image).on_hover_text(&ruleset.name);
        }
    }
}
