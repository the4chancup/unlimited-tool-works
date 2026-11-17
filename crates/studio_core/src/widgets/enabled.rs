//! The suite's disabled widgets: egui's, faded to half opacity as egui fades
//! them, with their text dimmed further to the palette's `disabled_text` and
//! their edge gone.
//!
//! egui fades a disabled widget whole, so its text keeps the resting text's
//! grey at half opacity, which leaves a disabled button reading almost as
//! plainly as an enabled one. egui's one `disabled_alpha` cannot keep the box
//! and darken the text, so a disabled scope draws its widgets' resting text
//! in a darker colour, which the same fade then draws at the drafts' disabled
//! grey. It draws a disabled widget in the resting visuals, so the theme's
//! resting edge, a live control's, would reach it: the scope takes it out,
//! and a disabled control draws none, so it still reads as off. egui never
//! draws a disabled widget hovered or pressed, so the resting state is the
//! only one it shows.

/// Adds `widget`, disabled unless `enabled`: egui's `Ui::add_enabled`, the
/// widget's text dimmed to the palette's `disabled_text` and its resting
/// edge taken while disabled. Not egui's own, whose disabled text is barely
/// dimmer than an enabled one's.
pub fn add_enabled(ui: &mut egui::Ui, enabled: bool, widget: impl egui::Widget) -> egui::Response {
    if enabled {
        return ui.add(widget);
    }
    // egui disables the widget in `ui` itself, not in a child scope, so its id
    // and its place in a wrapping row are those an enabled widget gets. The
    // resting look is set on `ui` too, and its style put back afterwards.
    let style = ui.style().clone();
    dim_resting(ui);
    #[expect(
        clippy::disallowed_methods,
        reason = "the suite's disabled widget is egui's with the resting look set"
    )]
    let response = ui.add_enabled(false, widget);
    ui.set_style(style);
    response
}

/// Adds the widgets `add_contents` draws in a scope that is disabled unless
/// `enabled`: egui's `Ui::add_enabled_ui`, its widgets' text dimmed to the
/// palette's `disabled_text` and their resting edge taken while disabled.
/// Not egui's own, whose disabled text is barely dimmer than an enabled
/// one's.
pub fn add_enabled_ui<R>(
    ui: &mut egui::Ui,
    enabled: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the suite's disabled scope is egui's with the resting look set"
    )]
    let scope = ui.add_enabled_ui(enabled, |ui| {
        if !enabled {
            dim_resting(ui);
        }
        add_contents(ui)
    });
    scope
}

/// Draws the resting widgets' text in `ui` in the palette's `disabled_text`
/// and takes their edge: a disabled control draws none. Only the resting
/// state is set, the one a disabled widget draws in.
fn dim_resting(ui: &mut egui::Ui) {
    let dimmed = crate::theme::palette(ui.visuals()).disabled_text;
    let widgets = &mut ui.visuals_mut().widgets;
    widgets.inactive.fg_stroke.color = dimmed;
    widgets.inactive.bg_stroke = egui::Stroke::NONE;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A context with the suite's look installed, as the window has it, in
    /// `theme`.
    fn context(theme: egui::Theme) -> egui::Context {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        ctx.set_theme(theme);
        ctx
    }

    /// One 800 by 600 frame drawing `contents` with `events` as its input;
    /// the frame's shapes.
    fn frame(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        contents: impl FnMut(&mut egui::Ui),
    ) -> Vec<egui::epaint::Shape> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            events,
            ..Default::default()
        };
        let mut contents = contents;
        let mut output = ctx.run_ui(input, |ui| contents(ui));
        let shapes = std::mem::take(&mut output.shapes)
            .into_iter()
            .map(|clipped| clipped.shape)
            .collect();
        // Nothing renders here, so the frame's texture deltas are dropped on
        // purpose.
        output.drop_without_applying_deltas();
        shapes
    }

    /// The rect shape painted exactly over `rect`.
    fn frame_at(shapes: &[egui::epaint::Shape], rect: egui::Rect) -> egui::epaint::RectShape {
        shapes
            .iter()
            .find_map(|shape| match shape {
                egui::Shape::Rect(shape) if shape.rect == rect => Some(shape.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no frame painted over {rect:?}"))
    }

    /// The resting widgets' text colour in `ui`, and whether `ui` is enabled.
    fn resting(ui: &egui::Ui) -> (egui::Color32, bool) {
        (
            ui.visuals().widgets.inactive.fg_stroke.color,
            ui.is_enabled(),
        )
    }

    /// The resting text colour and enabled state seen inside a disabled and
    /// an enabled `add_enabled_ui`, then inside a disabled and an enabled
    /// `add_enabled`, in `theme`.
    fn seen_inside(theme: egui::Theme) -> Vec<(egui::Color32, bool)> {
        let mut seen = Vec::new();
        frame(&context(theme), Vec::new(), |ui| {
            for enabled in [false, true] {
                add_enabled_ui(ui, enabled, |ui| seen.push(resting(ui)));
            }
            for enabled in [false, true] {
                add_enabled(ui, enabled, |ui: &mut egui::Ui| {
                    seen.push(resting(ui));
                    ui.label("Undo")
                });
            }
        });
        seen
    }

    #[test]
    fn a_disabled_scope_draws_the_dark_resting_text_in_the_disabled_grey() {
        let resting = egui::Color32::from_gray(180);
        let disabled = (egui::Color32::from_gray(134), false);
        assert_eq!(
            seen_inside(egui::Theme::Dark),
            vec![disabled, (resting, true), disabled, (resting, true)],
            "add_enabled_ui disabled and enabled, then add_enabled"
        );
    }

    #[test]
    fn a_disabled_scope_keeps_the_light_resting_text() {
        let resting = egui::Color32::from_gray(60);
        assert_eq!(
            seen_inside(egui::Theme::Light),
            vec![
                (resting, false),
                (resting, true),
                (resting, false),
                (resting, true)
            ],
            "add_enabled_ui disabled and enabled, then add_enabled"
        );
    }

    /// Whether an "Undo" button added through `add_enabled` with `enabled`
    /// reports itself enabled, and whether a click on it is reported.
    fn click_undo(enabled: bool) -> (bool, bool) {
        let ctx = context(egui::Theme::Dark);
        let rect = std::cell::Cell::new(egui::Rect::NOTHING);
        let seen = std::cell::Cell::new((false, false));
        let mut undo = |ui: &mut egui::Ui| {
            let response = add_enabled(ui, enabled, egui::Button::new("Undo"));
            rect.set(response.rect);
            seen.set((response.enabled(), seen.get().1 || response.clicked()));
        };
        frame(&ctx, Vec::new(), &mut undo);
        let pos = rect.get().center();
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        frame(
            &ctx,
            vec![egui::Event::PointerMoved(pos), button(true)],
            &mut undo,
        );
        frame(&ctx, vec![button(false)], &mut undo);
        frame(&ctx, Vec::new(), &mut undo);
        seen.get()
    }

    #[test]
    fn a_disabled_button_reports_no_click() {
        assert_eq!(click_undo(false), (false, false), "(enabled, clicked)");
        assert_eq!(click_undo(true), (true, true), "(enabled, clicked)");
    }

    #[test]
    fn the_disabled_grey_does_not_leak_into_the_parent() {
        let mut seen = Vec::new();
        frame(&context(egui::Theme::Dark), Vec::new(), |ui| {
            add_enabled_ui(ui, false, |ui| ui.label("Undo"));
            seen.push(resting(ui));
            add_enabled(ui, false, egui::Button::new("Undo"));
            seen.push(resting(ui));
        });
        let parent = (egui::Color32::from_gray(180), true);
        assert_eq!(
            seen,
            vec![parent, parent],
            "after add_enabled_ui, then after add_enabled"
        );
    }

    // The theme's resting stroke is a live control's edge: an enabled button
    // draws it, and the same button through `add_enabled` draws no stroke at
    // all, so it reads as off.
    #[test]
    fn an_enabled_button_draws_the_resting_edge_a_disabled_one_none() {
        let ctx = context(egui::Theme::Dark);
        let visuals = ctx.style_of(egui::Theme::Dark).visuals.clone();
        let edge = visuals.widgets.inactive.bg_stroke;
        assert_eq!(
            edge,
            egui::Stroke::new(1.0, egui::Color32::from_gray(90)),
            "premise: the dark theme's resting edge"
        );
        let mut rects = (egui::Rect::NOTHING, egui::Rect::NOTHING);
        let mut draw = |ui: &mut egui::Ui| {
            rects = (
                ui.button("Undo").rect,
                add_enabled(ui, false, egui::Button::new("Undo")).rect,
            );
        };
        // egui styles a widget from its state on the frame before.
        frame(&ctx, Vec::new(), &mut draw);
        let shapes = frame(&ctx, Vec::new(), &mut draw);
        assert_eq!(
            frame_at(&shapes, rects.0).stroke,
            edge,
            "the enabled button draws the resting edge"
        );
        assert_eq!(
            frame_at(&shapes, rects.1).stroke,
            egui::Stroke::NONE,
            "the disabled button draws no stroke"
        );
    }

    // A checked box in a disabled scope keeps its blue but draws no edge:
    // the scope's resting stroke is none.
    #[test]
    fn a_disabled_checked_checkbox_draws_its_blue_with_no_edge() {
        let ctx = context(egui::Theme::Dark);
        let visuals = ctx.style_of(egui::Theme::Dark).visuals.clone();
        let blue = visuals.selection.bg_fill;
        assert_eq!(
            blue,
            egui::Color32::from_rgb(0x00, 0x5c, 0x80),
            "premise: the selection blue"
        );
        let mut rect = egui::Rect::NOTHING;
        let mut draw = |ui: &mut egui::Ui| {
            add_enabled_ui(ui, false, |ui| {
                rect = crate::widgets::checkbox(ui, &mut true, "Undo").rect;
            });
        };
        frame(&ctx, Vec::new(), &mut draw);
        let shapes = frame(&ctx, Vec::new(), &mut draw);
        let square = egui::Rect::from_center_size(
            egui::pos2(rect.left() + 7.0, rect.center().y),
            egui::vec2(14.0, 14.0),
        );
        let boxes: Vec<_> = shapes
            .iter()
            .filter_map(|shape| match shape {
                egui::Shape::Rect(shape) if shape.rect == square => Some(shape),
                _ => None,
            })
            .collect();
        assert_eq!(boxes.len(), 2, "egui's box and the blue over it: {boxes:?}");
        let faded = blue.gamma_multiply(visuals.disabled_alpha());
        let blue_box = boxes
            .iter()
            .find(|shape| shape.fill == faded)
            .expect("the blue box, faded by the disabled scope");
        assert_eq!(
            blue_box.stroke,
            egui::Stroke::NONE,
            "the disabled checked box draws no edge"
        );
    }
}
