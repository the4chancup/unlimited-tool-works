//! The suite's slider (the drafts' `input[type=range]` with its value box
//! at the right), which the Tactics sliders and the Appearance screen's
//! Physique fields share.

use std::ops::RangeInclusive;

use egui::emath::Numeric;

use super::wheel::{number_direction, step_number, wheel_steps};

/// A slider on `value` inside `range`, its rail `rail_width` long and its
/// value box at the rail's right. The rail's fill and the handle take the
/// selection blue: egui fills the handle with the widget state's
/// `bg_fill`, the rail's grey, and has no field for the handle's fill
/// alone, so it is painted over. A stored value outside `range` draws as
/// stored; only an edit is clamped. The wheel over the rail steps the
/// value by one a notch as the rail reads, wheel down one more and wheel
/// up one less, whatever `Invert number scrolling` holds; over the value
/// box it steps as every number box does, the switch included — the box
/// disagrees with its rail or with every other box either way, and every
/// box alike is the rule a member can predict.
/// `wheel_id` names the slider for the wheel's delayed step, and must stay
/// put from frame to frame: egui answers with the value box's id while the
/// box has or just lost the keyboard, so the response's own id changes.
/// The response is marked changed when the value moved.
pub fn slider<N: Numeric>(
    ui: &mut egui::Ui,
    value: &mut N,
    range: RangeInclusive<N>,
    rail_width: f32,
    wheel_id: egui::Id,
) -> egui::Response {
    let kept_width = ui.spacing().slider_width;
    ui.spacing_mut().slider_width = rail_width;
    // egui's slider answers for the rail and the value box together: the
    // wheel over either is hovering it.
    let mut response = ui.add(
        egui::Slider::new(value, range.clone())
            .clamping(egui::SliderClamping::Edits)
            .trailing_fill(true),
    );
    ui.spacing_mut().slider_width = kept_width;
    // The handle's rect as egui computes it (its slider geometry is
    // private, so it is recomputed here): the rail widget is the first
    // `rail_width` of the row `response` shares with the value box, its
    // radius the row's height / 2.5, the centre travelling `radius * 0.75`
    // in from each end for the default rectangle handle. `interact` keeps
    // the widget state's stroke (inactive, hovered or active), `expansion`
    // the state's own.
    let rail = egui::Rect::from_min_size(
        response.rect.min,
        egui::vec2(rail_width, response.rect.height()),
    );
    if ui.is_rect_visible(response.rect) {
        let radius = rail.height() / 2.5;
        let (low, high) = (range.start().to_f64(), range.end().to_f64());
        let place = ((value.to_f64() - low) / (high - low)).clamp(0.0, 1.0) as f32;
        let centre = egui::pos2(
            egui::lerp(rail.x_range().shrink(radius * 0.75), place),
            rail.center().y,
        );
        let visuals = ui.style().interact(&response);
        ui.painter().rect(
            egui::Rect::from_center_size(centre, egui::vec2(radius * 1.5, radius * 2.0))
                .expand(visuals.expansion),
            visuals.corner_radius,
            ui.visuals().selection.bg_fill,
            visuals.fg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    let mut wheel_response = response.clone();
    wheel_response.id = wheel_id;
    let steps = wheel_steps(ui, &wheel_response);
    // `wheel_steps` counts once: the rail moves as a sideways control,
    // wheel down one more; the rest of the row is the value box's, a
    // number box whose steps the invert switch turns.
    let steps = if ui
        .ctx()
        .pointer_hover_pos()
        .is_some_and(|pos| rail.contains(pos))
    {
        -steps
    } else {
        number_direction(ui.ctx(), steps)
    };
    if step_number(value, steps, range) {
        response.mark_changed();
    }
    response
}
