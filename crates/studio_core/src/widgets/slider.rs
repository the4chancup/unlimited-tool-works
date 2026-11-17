//! The suite's slider (the drafts' `input[type=range]` with its value box
//! at the right), which the Tactics sliders and the Appearance screen's
//! Physique fields share.

use std::ops::RangeInclusive;

use egui::emath::Numeric;

use super::text_box::text_box_border;
use super::wheel::{number_direction, step_number, wheel_steps};

/// A slider on `value` inside `range`, its rail `rail_width` long and its
/// value box at the rail's right. The rail's fill and the handle take the
/// selection blue: egui fills the handle with the widget state's
/// `bg_fill`, the rail's grey, and has no field for the handle's fill
/// alone, so it is painted over. On the light theme the value box draws
/// as a number box: the text edit's fill and the text box's border. A
/// stored value outside `range` draws as stored; only an edit is
/// clamped. The wheel over the rail steps the
/// value by one a notch as the rail reads, wheel down one more and wheel
/// up one less, whatever `Invert number scrolling` holds; over the value
/// box it steps as every number box does, the switch included: the box
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
    // On the light theme the value box reads as a number box: the text
    // edit's fill on the live states and the text box's border at rest,
    // set on the fields the box's `DragValue`, a button, is drawn from.
    // `number_box`'s loop is not shared: it also fills `bg_fill`, which
    // the rail and the handle are drawn from.
    let widgets = (!ui.visuals().dark_mode).then(|| ui.visuals().widgets.clone());
    if widgets.is_some() {
        let (border, _) = text_box_border(ui.visuals());
        let fill = ui.visuals().text_edit_bg_color();
        let enabled = ui.is_enabled();
        let states = &mut ui.visuals_mut().widgets;
        for state in [
            &mut states.inactive,
            &mut states.hovered,
            &mut states.active,
        ] {
            state.weak_bg_fill = fill;
        }
        if enabled {
            states.inactive.bg_stroke = border;
        }
    }
    // egui's slider answers for the rail and the value box together: the
    // wheel over either is hovering it.
    let mut response = ui.add(
        egui::Slider::new(value, range.clone())
            .clamping(egui::SliderClamping::Edits)
            .trailing_fill(true),
    );
    ui.spacing_mut().slider_width = kept_width;
    if let Some(widgets) = widgets {
        ui.visuals_mut().widgets = widgets;
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::test_support::{context, frame};

    /// A resting `slider`'s painted frames in `ctx`: its value box's frame,
    /// the rect shape reaching the row's right edge, and its rail, the
    /// widest bar of the rail's thickness.
    fn resting_frames(ctx: &egui::Context) -> (egui::epaint::RectShape, egui::epaint::RectShape) {
        let mut value = 4u8;
        let mut draw =
            |ui: &mut egui::Ui| slider(ui, &mut value, 0..=10u8, 100.0, ui.scope_id().with("s"));
        // egui styles a widget from its state on the frame before.
        frame(ctx, Vec::new(), &mut draw);
        let (shapes, response) = frame(ctx, Vec::new(), &mut draw);
        let rail_height = ctx.global_style().spacing.slider_rail_height;
        let value_box = shapes
            .iter()
            .find_map(|shape| match shape {
                egui::epaint::Shape::Rect(shape) if shape.rect.max.x == response.rect.max.x => {
                    Some(shape.clone())
                }
                _ => None,
            })
            .expect("the value box's frame");
        let rail = shapes
            .iter()
            .filter_map(|shape| match shape {
                egui::epaint::Shape::Rect(shape) if shape.rect.height() == rail_height => {
                    Some(shape)
                }
                _ => None,
            })
            .max_by(|a, b| a.rect.width().total_cmp(&b.rect.width()))
            .cloned()
            .expect("the rail's bar");
        (value_box, rail)
    }

    // On the light theme the value box draws as a number box: the text
    // edit's fill and the text box's border, where a control's resting
    // fill is the widgets' grey. The rail keeps the resting fill.
    #[test]
    fn a_light_sliders_value_box_draws_as_a_number_box() {
        let ctx = context(egui::Theme::Light);
        let visuals = ctx.style_of(egui::Theme::Light).visuals.clone();
        let fill = egui::Color32::from_gray(253);
        let border = egui::Color32::from_gray(190);
        assert_eq!(
            visuals.text_edit_bg_color(),
            fill,
            "premise: the light text edit's fill"
        );
        assert_eq!(
            visuals.widgets.noninteractive.bg_stroke.color, border,
            "premise: the light theme's quiet line"
        );
        assert_ne!(
            visuals.widgets.inactive.weak_bg_fill, fill,
            "premise: a control's resting fill is not the text edit's"
        );
        assert_eq!(
            visuals.widgets.inactive.bg_fill,
            egui::Color32::from_gray(200),
            "premise: the light resting widgets' fill"
        );
        let (value_box, rail) = resting_frames(&ctx);
        assert_eq!(value_box.fill, fill, "the resting value box's fill");
        assert_eq!(
            value_box.stroke,
            egui::Stroke::new(1.0, border),
            "the resting value box's border"
        );
        assert_eq!(
            rail.fill,
            egui::Color32::from_gray(200),
            "the rail keeps the resting widgets' fill"
        );
    }

    // On the dark theme the value box is the control every other is: the
    // resting widgets' fill and edge, as an unwrapped `DragValue` draws.
    #[test]
    fn a_dark_sliders_value_box_is_the_controls_grey() {
        let ctx = context(egui::Theme::Dark);
        let visuals = ctx.style_of(egui::Theme::Dark).visuals.clone();
        assert_eq!(
            visuals.widgets.inactive.weak_bg_fill,
            egui::Color32::from_gray(60),
            "premise: the dark resting fill"
        );
        let (value_box, rail) = resting_frames(&ctx);
        assert_eq!(
            (value_box.fill, value_box.stroke),
            (
                visuals.widgets.inactive.weak_bg_fill,
                visuals.widgets.inactive.bg_stroke
            ),
            "the resting widgets' fill and edge"
        );
        assert_eq!(
            rail.fill, visuals.widgets.inactive.bg_fill,
            "the rail's fill"
        );
    }
}
