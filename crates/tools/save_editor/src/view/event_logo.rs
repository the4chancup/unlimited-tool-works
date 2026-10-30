//! The active ruleset's event logo at the view's top right (the draft's
//! `.event-logo`), the sidebar clover mirrored: drawn over the toolbar
//! row's empty end, moving nothing, and with an event page a link to it.

use team_widgets::event_logo::HEIGHT;

use super::SaveView;

/// The logo's right edge from the window's, in points: as far as the
/// clover's ink stands from the left.
const RIGHT: f32 = 14.0;

/// The logo's top below the view's, in points: as far as the clover's ink
/// stands below the window's top.
const TOP: f32 = 8.0;

/// The room the toolbar's last widget keeps before the logo, in points;
/// nearer (a narrow window), the logo is not drawn rather than cover it.
const CLEARANCE: f32 = 8.0;

/// Draws the logo while the active ruleset carries one: 28 points high,
/// its right edge `RIGHT` from the window's, its top `TOP` below the
/// view's. `toolbar_end` is the x where the toolbar's last widget ends;
/// one reaching within `CLEARANCE` of the logo hides it.
pub(super) fn show(ui: &mut egui::Ui, view: &SaveView, toolbar_end: f32) {
    let Some(active) = view.rules.as_ref() else {
        return;
    };
    let Some(logo) = active.logo.as_ref() else {
        return;
    };
    let size = logo.size();
    let right = ui.ctx().content_rect().max.x - RIGHT;
    // `ui` is the shell's central panel inside its margin: the view's top
    // is the panel's.
    let top = ui.max_rect().top()
        - f32::from(egui::Frame::central_panel(ui.style()).inner_margin.top)
        + TOP;
    let rect = egui::Rect::from_min_max(
        egui::pos2(right - size.x, top),
        egui::pos2(right, top + HEIGHT),
    );
    if toolbar_end + CLEARANCE > rect.left() {
        return;
    }
    let ruleset = active.rules.ruleset();
    // A child placed by rect allocates nothing in `ui`, so no row moves.
    // Right to left, centred: a picture wider than 3:1 is less than 28
    // high and sits in the middle of the box (`object-position: right`).
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    let image = logo.image().alt_text(&ruleset.name);
    match &ruleset.page {
        Some(page) => {
            let response = child
                .add(image.sense(egui::Sense::click()))
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(format!(
                    "{}\n\nClick here to open the event's page:\n{page}",
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
