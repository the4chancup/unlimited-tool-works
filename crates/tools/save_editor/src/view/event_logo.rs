//! The active ruleset's event logo at the view's top right (the draft's
//! `.event-logo`), the sidebar clover mirrored: drawn past the toolbar
//! band's end, moving nothing, and with an event page a link to it, framed
//! as a button while in use as the clover is.

use team_widgets::event_logo::HEIGHT;

use super::SaveView;

/// The logo's right edge from the view's, in points: as far as the
/// clover's ink stands from the left. The toolbar's band ends as far left
/// of the logo, so the logo has the same room on both sides.
pub(super) const RIGHT: f32 = 14.0;

/// The logo's top below the view's, in points: as far as the clover's ink
/// stands below the window's top.
const TOP: f32 = 8.0;

/// How far the frame a linked logo shows while in use stands out from the
/// picture, in points: the room the clover has in the sidebar logo's
/// square (28 inside 32), so the two logos' hovers look alike.
const FRAME_ROOM: f32 = 2.0;

/// Where the logo stands while a save is open and the active ruleset
/// carries one: 28 points high, its right edge `RIGHT` inside the view's
/// outer edge (`view_rect`'s right edge plus the central panel's margin —
/// the window's edge at the default window, the cap's edge in a larger
/// one), its top `TOP` below the view's. It depends on the view's rect
/// and the picture alone, so the toolbar's band can end short of it
/// before either is drawn.
pub(super) fn rect(ui: &egui::Ui, view: &SaveView, view_rect: egui::Rect) -> Option<egui::Rect> {
    view.session.as_ref()?;
    let size = view.rules.as_ref()?.logo.as_ref()?.size();
    let margin = egui::Frame::central_panel(ui.style()).inner_margin;
    let right = view_rect.right() + f32::from(margin.right) - RIGHT;
    let top = view_rect.top() - f32::from(margin.top) + TOP;
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
            // Reserved before the picture, so the frame lands under it.
            let frame_slot = child.painter().add(egui::Shape::Noop);
            let response = child
                .add(image.sense(egui::Sense::click()))
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(format!(
                    "{}\n\nClick to open the event's page:\n{page}",
                    ruleset.name
                ));
            let frame_rect = response.rect.expand(FRAME_ROOM);
            if let Some(frame) =
                studio_core::widgets::in_use_frame(&child, frame_rect, &response, false)
            {
                child.painter().set(frame_slot, frame);
            }
            if response.clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(page));
            }
        }
        None => {
            child.add(image).on_hover_text(&ruleset.name);
        }
    }
}
