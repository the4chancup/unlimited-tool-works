//! The suite's text box: egui's `TextEdit` with the drafts' resting border.

/// A text box's corner radius in every state (the drafts' `input[type=text]`
/// `border-radius`), where the theme rounds a resting widget by 2.
const CORNER_RADIUS: u8 = 3;

/// A resting text box's outline under `visuals`: the 1-point border in the
/// theme's quiet line colour (the noninteractive stroke, grey 60 on the dark
/// theme) and the corner radius of 3 a text box keeps in every state. A
/// widget that should read as a field (the colour field's swatch) outlines
/// itself with it, so it stays visible on a fill as dark as the panel.
pub fn text_box_border(visuals: &egui::Visuals) -> (egui::Stroke, egui::CornerRadius) {
    (
        visuals.widgets.noninteractive.bg_stroke,
        egui::CornerRadius::same(CORNER_RADIUS),
    )
}

/// `edit` drawn as the drafts' `input[type=text]`: at rest, a 1-point border
/// in the theme's quiet line colour (the noninteractive stroke, grey 60 on
/// the dark theme), its corners rounded by 3 (`text_box_border`); hovered
/// and focused as egui draws them, the focus outline in the selection's
/// stroke. The box's outer size is egui's: the border takes its width off
/// the inner margin.
/// egui draws a resting text box with the resting widgets' stroke, which the
/// buttons share and which is none, and its hook for a text box's look is
/// public only behind its `experimental` feature, so the border is set on
/// the box's own `Ui` for the one widget. Add it as any widget
/// (`ui.add(text_box(…))`, `ui.add_sized(…)`).
pub fn text_box<'t>(edit: egui::TextEdit<'t>) -> impl egui::Widget + 't {
    move |ui: &mut egui::Ui| {
        let widgets = ui.visuals().widgets.clone();
        let (border, corner_radius) = text_box_border(ui.visuals());
        let shaped = &mut ui.visuals_mut().widgets;
        shaped.inactive.bg_stroke = border;
        for state in [
            &mut shaped.inactive,
            &mut shaped.hovered,
            &mut shaped.active,
        ] {
            state.corner_radius = corner_radius;
        }
        let response = ui.add(edit);
        ui.visuals_mut().widgets = widgets;
        response
    }
}

#[cfg(test)]
mod tests {
    use egui::epaint::Shape;

    use super::*;

    /// One 400 by 200 frame of `contents` on the suite's `theme`, pointer
    /// far from it; the frame's shapes and what `contents` returned.
    fn frame<R>(
        ctx: &egui::Context,
        contents: &mut impl FnMut(&mut egui::Ui) -> R,
    ) -> (Vec<Shape>, R) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 200.0),
            )),
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
    // stroke, corner radius 3, at egui's own outer size, on both themes; a
    // button beside it keeps no border, and a focused box takes the
    // selection's stroke.
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
                (boxed, egui_own, button)
            };
            // egui styles a text box from its state on the frame before.
            frame(&ctx, &mut draw);
            let (shapes, (boxed, egui_own, button)) = frame(&ctx, &mut draw);
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
                painted.corner_radius,
                egui::CornerRadius::same(3),
                "{theme:?}: the corners"
            );
            assert_eq!(
                boxed.rect.size(),
                egui_own.rect.size(),
                "{theme:?}: the border keeps egui's outer size"
            );
            assert_eq!(boxed.rect.height(), 18.0, "{theme:?}: 18 tall");
            assert_eq!(
                frame_at(&shapes, button.rect).stroke.width,
                0.0,
                "{theme:?}: a button drawn after it keeps no border"
            );

            ctx.memory_mut(|memory| memory.request_focus(boxed.id));
            frame(&ctx, &mut draw);
            let (shapes, (boxed, _, _)) = frame(&ctx, &mut draw);
            assert!(boxed.has_focus(), "premise: the box holds the keyboard");
            assert_eq!(
                frame_at(&shapes, boxed.rect).stroke,
                visuals.selection.stroke,
                "{theme:?}: the focus outline"
            );
        }
    }
}
