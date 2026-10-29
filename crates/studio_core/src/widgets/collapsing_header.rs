//! The suite's collapsing header: egui's `CollapsingHeader` with its title in
//! the strong text colour.

/// A collapsing header titled `title`, drawn as the drafts' `details >
/// summary`: the title in the theme's strong text colour at rest and hovered
/// alike, where egui draws it in the widget state's text colour (the resting
/// grey, the hovered near-white). The arrow keeps egui's colours. Chain
/// egui's own builder calls (`id_salt`, `open`, `icon`) and `show` it as any
/// `CollapsingHeader`.
pub fn collapsing_header(title: impl Into<egui::WidgetText>) -> egui::CollapsingHeader {
    egui::CollapsingHeader::new(title.into().strong())
}

#[cfg(test)]
mod tests {
    use egui::epaint::ClippedShape;

    use super::*;

    /// The colour the frame painted the text `text` in: its first section's
    /// own colour, or the shape's fallback where the section leaves it to
    /// the painter.
    fn painted_colour(shapes: &[ClippedShape], text: &str) -> egui::Color32 {
        let shape = shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if shape.galley.text() == text => Some(shape),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no painted text {text:?}"));
        let colour = shape.galley.job.sections[0].format.color;
        if colour == egui::Color32::PLACEHOLDER {
            shape.fallback_color
        } else {
            colour
        }
    }

    /// One 400 by 200 frame of a header titled "Appearance", the pointer at
    /// `pointer`; the frame's shapes and the header's response.
    fn frame(
        ctx: &egui::Context,
        pointer: Option<egui::Pos2>,
    ) -> (Vec<ClippedShape>, egui::Response) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 200.0),
            )),
            events: pointer.map(egui::Event::PointerMoved).into_iter().collect(),
            ..Default::default()
        };
        let mut header = None;
        let mut output = ctx.run_ui(input, |ui| {
            header = Some(
                collapsing_header("Appearance")
                    .show(ui, |_| ())
                    .header_response,
            );
        });
        let shapes = std::mem::take(&mut output.shapes);
        // Nothing renders here, so the frame's texture deltas are dropped on
        // purpose.
        output.drop_without_applying_deltas();
        (shapes, header.expect("run_ui calls the closure"))
    }

    // The title is painted in the strong text colour at rest and under the
    // pointer, where egui would paint it in the resting grey and the hovered
    // near-white. The dark theme, the drafts' own: the light one's hovered
    // text is already its strong black.
    #[test]
    fn the_title_is_painted_in_the_strong_colour_at_rest_and_hovered() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        ctx.set_theme(egui::Theme::Dark);
        let visuals = ctx.style_of(egui::Theme::Dark).visuals.clone();
        let strong = visuals.strong_text_color();
        assert_ne!(
            visuals.widgets.inactive.text_color(),
            strong,
            "premise: the resting text is not the strong colour"
        );
        assert_ne!(
            visuals.widgets.hovered.text_color(),
            strong,
            "premise: the hovered text is not the strong colour"
        );

        let (resting, header) = frame(&ctx, None);
        assert_eq!(
            painted_colour(&resting, "Appearance"),
            strong,
            "the resting title"
        );

        // egui styles a widget from its hover on the frame before.
        frame(&ctx, Some(header.rect.center()));
        let (hovered, header) = frame(&ctx, Some(header.rect.center()));
        assert!(header.hovered(), "premise: the pointer is over the header");
        assert_eq!(
            painted_colour(&hovered, "Appearance"),
            strong,
            "the hovered title"
        );
    }
}
