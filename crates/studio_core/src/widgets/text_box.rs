//! The suite's text box: egui's `TextEdit` with the drafts' resting border.

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
/// selection's stroke. The box's outer size is egui's: the border takes its
/// width off the inner margin.
/// egui draws a resting text box with the resting widgets' stroke, the
/// control's edge the buttons share, and its hook for a text box's look is
/// public only behind its `experimental` feature, so the quieter border is
/// set on the box's own `Ui` for the one widget. Add it as any widget
/// (`ui.add(text_box(…))`, `ui.add_sized(…)`).
pub fn text_box<'t>(edit: egui::TextEdit<'t>) -> impl egui::Widget + 't {
    move |ui: &mut egui::Ui| {
        let widgets = ui.visuals().widgets.clone();
        let (border, _) = text_box_border(ui.visuals());
        ui.visuals_mut().widgets.inactive.bg_stroke = border;
        let response = ui.add(edit);
        ui.visuals_mut().widgets = widgets;
        response
    }
}

/// `drag` drawn as a text box on the light theme: a light text box is
/// nearly white and a light number box reads as one, the text edit's
/// fill in every state and the text box's border at rest
/// (`text_box_border`), so the two kinds of field look alike. The dark
/// theme's is not touched: there the box is the grey every control is,
/// the look the drafts were reviewed in. The light look is set on the
/// box's own `Ui` for the one widget, as `text_box` sets its border.
/// Add it as any widget (`ui.add(number_box(…))`, `ui.add_sized(…)`);
/// every `egui::DragValue` the suite adds goes through it.
pub fn number_box<'a>(drag: egui::DragValue<'a>) -> impl egui::Widget + 'a {
    move |ui: &mut egui::Ui| {
        if ui.visuals().dark_mode {
            return ui.add(drag);
        }
        let widgets = ui.visuals().widgets.clone();
        let (border, _) = text_box_border(ui.visuals());
        let fill = ui.visuals().text_edit_bg_color();
        let states = &mut ui.visuals_mut().widgets;
        for state in [
            &mut states.inactive,
            &mut states.hovered,
            &mut states.active,
        ] {
            state.weak_bg_fill = fill;
            state.bg_fill = fill;
        }
        states.inactive.bg_stroke = border;
        let response = ui.add(drag);
        ui.visuals_mut().widgets = widgets;
        response
    }
}

#[cfg(test)]
mod tests {
    use egui::epaint::Shape;

    use super::*;

    /// One 400 by 200 frame of `contents` with `events` as its input; the
    /// frame's shapes and what `contents` returned.
    fn frame<R>(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        contents: &mut impl FnMut(&mut egui::Ui) -> R,
    ) -> (Vec<Shape>, R) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 200.0),
            )),
            events,
            ..Default::default()
        };
        let mut result = None;
        let mut output = ctx.run_ui(input, |ui| result = Some(contents(ui)));
        let shapes = std::mem::take(&mut output.shapes)
            .into_iter()
            .map(|clipped| clipped.shape)
            .collect();
        // Nothing renders here, so the frame's texture deltas are dropped on
        // purpose.
        output.drop_without_applying_deltas();
        (shapes, result.expect("run_ui calls the closure"))
    }

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
