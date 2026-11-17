//! The suite's text and number boxes: `text_box`, egui's `TextEdit` with
//! the drafts' resting border, `number_box`, the `DragValue` drawn as one
//! on the light theme, and `number_box_look`, that light look as a scope
//! `slider`'s value box shares.

/// A resting text box's outline under `visuals`: the 1-point border in the
/// theme's quiet line colour (the noninteractive stroke, grey 60 on the dark
/// theme) and the theme's corner radius. A widget that should read as a
/// field (the colour field's swatch) outlines itself with it, so it stays
/// visible on a fill as dark as the panel.
pub fn text_box_border(visuals: &egui::Visuals) -> (egui::Stroke, egui::CornerRadius) {
    (
        visuals.widgets.noninteractive.bg_stroke,
        visuals.widgets.noninteractive.corner_radius,
    )
}

/// `edit` drawn as the drafts' `input[type=text]`: at rest, a 1-point border
/// in the theme's quiet line colour (the noninteractive stroke, grey 60 on
/// the dark theme), its corners the theme's radius (`text_box_border`);
/// hovered and focused as egui draws them, the focus outline in the
/// selection's stroke. A disabled box draws none, as a disabled control
/// does. The box's outer size is egui's: the border takes its width off the
/// inner margin.
/// egui draws a resting text box with the resting widgets' stroke, the
/// control's edge the buttons share, and its hook for a text box's look is
/// public only behind its `experimental` feature, so the quieter border is
/// set on the box's own `Ui` for the one widget. Add it as any widget
/// (`ui.add(text_box(…))`, `ui.add_sized(…)`).
pub fn text_box<'t>(edit: egui::TextEdit<'t>) -> impl egui::Widget + 't {
    move |ui: &mut egui::Ui| {
        if !ui.is_enabled() {
            return ui.add(edit);
        }
        let widgets = ui.visuals().widgets.clone();
        let (border, _) = text_box_border(ui.visuals());
        ui.visuals_mut().widgets.inactive.bg_stroke = border;
        let response = ui.add(edit);
        ui.visuals_mut().widgets = widgets;
        response
    }
}

/// The light theme's number-box look around `add`: on the light theme a
/// number box reads as a text box (the text edit's fill in every state
/// and the text box's border at rest), so the two kinds of field look
/// alike. It sets the live states' `weak_bg_fill` to the text edit's
/// fill and, while `ui` is enabled, the resting `bg_stroke` to
/// `text_box_border`'s stroke, runs `add` and puts the widgets back. On
/// the dark theme it only runs `add`: there the box is the grey every
/// control is, the look the drafts were reviewed in. `bg_fill` stays as
/// the theme has it: a `DragValue` never reads it, drawing as a button
/// (`weak_bg_fill`, `bg_stroke`) or, while edited, a `TextEdit`
/// (`text_edit_bg_color`). A slider's rail and handle are drawn from
/// `bg_fill`, so `slider`'s value box can share this scope.
pub fn number_box_look<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    if ui.visuals().dark_mode {
        return add(ui);
    }
    let widgets = ui.visuals().widgets.clone();
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
    let result = add(ui);
    ui.visuals_mut().widgets = widgets;
    result
}

/// `drag` drawn as a text box on the light theme (`number_box_look`):
/// a light text box is nearly white and a light number box reads as one,
/// so the two kinds of field look alike.
/// Add it as any widget (`ui.add(number_box(…))`, `ui.add_sized(…)`);
/// every `egui::DragValue` the suite adds goes through it.
pub fn number_box<'a>(drag: egui::DragValue<'a>) -> impl egui::Widget + 'a {
    move |ui: &mut egui::Ui| number_box_look(ui, |ui| ui.add(drag))
}

#[cfg(test)]
mod tests {
    use egui::epaint::Shape;

    use super::*;
    use crate::widgets::test_support::frame;

    /// The rect shape painted exactly over `rect`.
    fn frame_at(shapes: &[Shape], rect: egui::Rect) -> egui::epaint::RectShape {
        shapes
            .iter()
            .find_map(|shape| match shape {
                Shape::Rect(shape) if shape.rect == rect => Some(shape.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no frame painted over {rect:?}"))
    }

    // A resting text box: a 1-point border in the theme's noninteractive
    // stroke, the theme's corner radius, at egui's own outer size, on both
    // themes; a button beside it draws the resting edge, not the box's
    // border, a frameless one none, and a focused box takes the selection's
    // stroke.
    #[test]
    fn a_resting_text_box_draws_the_drafts_border() {
        for (theme, border) in [
            (egui::Theme::Dark, egui::Color32::from_gray(60)),
            (egui::Theme::Light, egui::Color32::from_gray(190)),
        ] {
            let ctx = egui::Context::default();
            crate::theme::install(&ctx);
            ctx.set_theme(theme);
            let mut text = "QUADS ONLY".to_owned();
            let mut plain = "QUADS ONLY".to_owned();
            let mut draw = |ui: &mut egui::Ui| {
                let boxed = ui.add(text_box(egui::TextEdit::singleline(&mut text)));
                let egui_own = ui.add(egui::TextEdit::singleline(&mut plain));
                let button = ui.button("Save");
                let frameless = ui.add(egui::Button::new("x").frame(false));
                (boxed, egui_own, button, frameless)
            };
            // egui styles a text box from its state on the frame before.
            frame(&ctx, Vec::new(), &mut draw);
            let (shapes, (boxed, egui_own, button, frameless)) = frame(&ctx, Vec::new(), &mut draw);
            let visuals = ctx.style_of(ctx.theme()).visuals.clone();
            assert_eq!(
                visuals.widgets.noninteractive.bg_stroke.color, border,
                "premise: {theme:?}'s noninteractive stroke"
            );

            let painted = frame_at(&shapes, boxed.rect);
            assert_eq!(
                painted.stroke,
                egui::Stroke::new(1.0, border),
                "{theme:?}: the resting border"
            );
            assert_eq!(
                painted.corner_radius, visuals.widgets.inactive.corner_radius,
                "{theme:?}: the corners"
            );
            assert_eq!(
                boxed.rect.size(),
                egui_own.rect.size(),
                "{theme:?}: the border keeps egui's outer size"
            );
            assert_eq!(boxed.rect.height(), 18.0, "{theme:?}: 18 tall");
            assert_eq!(
                frame_at(&shapes, button.rect).stroke,
                visuals.widgets.inactive.bg_stroke,
                "{theme:?}: a button drawn after it keeps the resting edge, \
                 not the box's border"
            );
            let frameless_frames: Vec<_> = shapes
                .iter()
                .filter(|shape| {
                    matches!(shape, egui::Shape::Rect(rect)
                        if rect.rect == frameless.rect
                            && (rect.fill != egui::Color32::TRANSPARENT
                                || !rect.stroke.is_empty()))
                })
                .collect();
            assert!(
                frameless_frames.is_empty(),
                "{theme:?}: a frameless button paints no frame: {frameless_frames:?}"
            );

            ctx.memory_mut(|memory| memory.request_focus(boxed.id));
            frame(&ctx, Vec::new(), &mut draw);
            let (shapes, (boxed, _, _, _)) = frame(&ctx, Vec::new(), &mut draw);
            assert!(boxed.has_focus(), "premise: the box holds the keyboard");
            assert_eq!(
                frame_at(&shapes, boxed.rect).stroke,
                visuals.selection.stroke,
                "{theme:?}: the focus outline"
            );
        }
    }

    // A disabled text box draws no border, as a disabled control draws
    // none: the disabled scope's resting stroke stays out.
    #[test]
    fn a_disabled_text_box_draws_no_border() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        ctx.set_theme(egui::Theme::Dark);
        let mut text = "QUADS ONLY".to_owned();
        let mut draw = |ui: &mut egui::Ui| {
            crate::widgets::add_enabled_ui(ui, false, |ui| {
                ui.add(text_box(egui::TextEdit::singleline(&mut text)))
            })
            .inner
        };
        // egui styles a text box from its state on the frame before.
        frame(&ctx, Vec::new(), &mut draw);
        let (shapes, boxed) = frame(&ctx, Vec::new(), &mut draw);
        assert_eq!(
            frame_at(&shapes, boxed.rect).stroke,
            egui::Stroke::NONE,
            "a disabled text box draws no border"
        );
    }

    // On the light theme a number box draws as a text box: the text edit's
    // fill at rest and hovered, the text box's border at rest.
    #[test]
    fn a_light_number_box_draws_as_a_text_box() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        ctx.set_theme(egui::Theme::Light);
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
        let mut value = 4u8;
        let mut text = "4".to_owned();
        let mut draw = |ui: &mut egui::Ui| {
            (
                ui.add(number_box(egui::DragValue::new(&mut value))),
                ui.add(text_box(egui::TextEdit::singleline(&mut text))),
            )
        };
        frame(&ctx, Vec::new(), &mut draw);
        let (shapes, (boxed, edit)) = frame(&ctx, Vec::new(), &mut draw);
        let painted = frame_at(&shapes, boxed.rect);
        assert_eq!(painted.fill, fill, "the resting number box's fill");
        assert_eq!(
            painted.stroke,
            egui::Stroke::new(1.0, border),
            "the resting number box's border"
        );
        assert_eq!(
            frame_at(&shapes, edit.rect).fill,
            fill,
            "the resting text box's fill"
        );

        // Hovered: the same fill, where the widgets' own hovered fill is
        // darker.
        assert_ne!(
            visuals.widgets.hovered.weak_bg_fill, fill,
            "premise: a control's hovered fill is not the text edit's"
        );
        let at = boxed.rect.center();
        frame(&ctx, vec![egui::Event::PointerMoved(at)], &mut draw);
        let (shapes, (boxed, _)) = frame(&ctx, vec![egui::Event::PointerMoved(at)], &mut draw);
        assert!(boxed.hovered(), "premise: the pointer is over the box");
        assert_eq!(
            frame_at(&shapes, boxed.rect).fill,
            fill,
            "the hovered number box's fill"
        );

        // Pressed: the same fill, where the widgets' own pressed fill is
        // darker still.
        assert_ne!(
            visuals.widgets.active.weak_bg_fill, fill,
            "premise: a control's pressed fill is not the text edit's"
        );
        let press = vec![
            egui::Event::PointerMoved(at),
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        frame(&ctx, press.clone(), &mut draw);
        let (shapes, (boxed, _)) = frame(&ctx, press, &mut draw);
        assert!(
            boxed.is_pointer_button_down_on(),
            "premise: the pointer holds the box"
        );
        assert_eq!(
            frame_at(&shapes, boxed.rect).fill,
            fill,
            "the pressed number box's fill"
        );
    }

    // A disabled light number box draws no border, as a disabled control
    // draws none: the scope's resting stroke stays out and the look does
    // not put the text box's in.
    #[test]
    fn a_disabled_light_number_box_draws_no_border() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        ctx.set_theme(egui::Theme::Light);
        let mut value = 4u8;
        let mut draw = |ui: &mut egui::Ui| {
            crate::widgets::add_enabled_ui(ui, false, |ui| {
                ui.add(number_box(egui::DragValue::new(&mut value)))
            })
            .inner
        };
        // egui styles a widget from its state on the frame before.
        frame(&ctx, Vec::new(), &mut draw);
        let (shapes, boxed) = frame(&ctx, Vec::new(), &mut draw);
        assert_eq!(
            frame_at(&shapes, boxed.rect).stroke,
            egui::Stroke::NONE,
            "a disabled number box draws no border"
        );
    }

    // On the dark theme a number box is the control every other is: the
    // resting widgets' fill and edge, as an unwrapped `DragValue` draws.
    #[test]
    fn a_dark_number_box_is_an_unwrapped_drag_value() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        ctx.set_theme(egui::Theme::Dark);
        let visuals = ctx.style_of(egui::Theme::Dark).visuals.clone();
        assert_eq!(
            visuals.widgets.inactive.weak_bg_fill,
            egui::Color32::from_gray(60),
            "premise: the dark resting fill"
        );
        let mut boxed = 4u8;
        let mut plain = 5u8;
        let mut draw = |ui: &mut egui::Ui| {
            (
                ui.add(number_box(egui::DragValue::new(&mut boxed))),
                ui.add(egui::DragValue::new(&mut plain)),
            )
        };
        frame(&ctx, Vec::new(), &mut draw);
        let (shapes, (boxed, plain)) = frame(&ctx, Vec::new(), &mut draw);
        let painted = frame_at(&shapes, boxed.rect);
        let unwrapped = frame_at(&shapes, plain.rect);
        assert_eq!(
            (painted.fill, painted.stroke),
            (unwrapped.fill, unwrapped.stroke),
            "the wrap is an unwrapped `DragValue` on the dark theme"
        );
        assert_eq!(
            (painted.fill, painted.stroke),
            (
                visuals.widgets.inactive.weak_bg_fill,
                visuals.widgets.inactive.bg_stroke,
            ),
            "the resting widgets' fill and edge"
        );
    }
}
