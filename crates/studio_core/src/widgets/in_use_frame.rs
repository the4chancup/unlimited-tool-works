//! The button frame a clickable picture shows while in use, so it reads as a
//! button: the sidebar's 4cc logo and entries, a tool's event logo.

/// The frame a widget of `response` shows over `rect` while it is in use
/// (`active`, hovered, pressed or focused), in the button look egui gives a
/// selectable widget in that state: `None` while it is not in use, or while
/// `ui` is disabled. A row is not a control, so the frame takes no edge
/// while the row is only `active` at rest: the theme's resting stroke is a
/// control's. A caller painting a picture over the frame paints the
/// frame first (or into a slot reserved before the picture), since the
/// frame is filled.
pub fn in_use_frame(
    ui: &egui::Ui,
    rect: egui::Rect,
    response: &egui::Response,
    active: bool,
) -> Option<egui::Shape> {
    let in_use = active
        || response.hovered()
        || response.is_pointer_button_down_on()
        || response.has_focus();
    if !ui.is_enabled() || !in_use {
        return None;
    }
    let visuals = ui.style().interact_selectable(response, active);
    let stroke = if super::at_rest(ui, response) {
        egui::Stroke::NONE
    } else {
        visuals.bg_stroke
    };
    Some(
        egui::epaint::RectShape::new(
            rect.expand(visuals.expansion),
            visuals.corner_radius,
            visuals.weak_bg_fill,
            stroke,
            egui::StrokeKind::Inside,
        )
        .into(),
    )
}
