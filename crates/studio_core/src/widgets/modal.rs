//! The modal window every dialog draws through, the drafts' `.window`: its
//! frame, its title over the body, and its row of buttons at the right.

/// The drafts' `.window` frame: egui's popup frame with 12 points of margin across and 8 down.
pub fn modal_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::popup(style).inner_margin(egui::Margin::symmetric(12, 8))
}

/// `egui::Modal` under `id` in `modal_frame`, at least 360 points wide with its margins and
/// stroke: `title` (`theme::window_title`), 8 points, then `body`.
pub fn modal<R>(
    ctx: &egui::Context,
    id: egui::Id,
    title: impl Into<egui::RichText>,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::ModalResponse<R> {
    modal_with_min_width(ctx, id, title, Some(360.0), body)
}

/// `modal` without the 360-point minimum: the window is as wide as its title and body. For a
/// dialog whose draft sizes the window to its contents, where the minimum would leave room
/// unused right of them. Every other dialog takes `modal`, whose minimum keeps a short
/// prompt from shrinking to a narrow box around its sentence and buttons.
pub fn modal_fit_contents<R>(
    ctx: &egui::Context,
    id: egui::Id,
    title: impl Into<egui::RichText>,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::ModalResponse<R> {
    modal_with_min_width(ctx, id, title, None, body)
}

/// The modal both helpers draw, at least `min_width` wide with its margins and stroke when
/// one is given.
fn modal_with_min_width<R>(
    ctx: &egui::Context,
    id: egui::Id,
    title: impl Into<egui::RichText>,
    min_width: Option<f32>,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::ModalResponse<R> {
    egui::Modal::new(id)
        .frame(modal_frame(&ctx.global_style()))
        .show(ctx, |ui| {
            if let Some(min_width) = min_width {
                ui.set_min_width(min_width - modal_frame(ui.style()).total_margin().sum().x);
            }
            crate::theme::window_title(ui, title);
            ui.add_space(8.0 - ui.spacing().item_spacing.y);
            body(ui)
        })
}

/// The modal's button row, 10 points under what is above it, right-aligned at the right
/// edge of the modal's content; `buttons` adds them from the right, so the last in reading
/// order first.
pub fn modal_buttons<R>(ui: &mut egui::Ui, buttons: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.add_space(10.0 - ui.spacing().item_spacing.y);
    // A bare right-to-left layout would claim the whole available width, and the modal's
    // area would grow to it.
    let size = egui::vec2(ui.min_rect().width(), ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(
        size,
        egui::Layout::right_to_left(egui::Align::Center),
        buttons,
    )
    .inner
}

/// The space between a dialog's sentence and the element under it, the drafts' `.window p`
/// margin.
const SENTENCE_SPACING: f32 = 8.0;

/// The gap under a dialog's sentence when another sentence or a control follows it in the
/// body, 8 points where a label alone leaves egui's 3. A sentence that ends the body takes
/// none: the buttons' 10 under it stands for both margins.
pub fn modal_sentence_gap(ui: &mut egui::Ui) {
    ui.add_space(SENTENCE_SPACING - ui.spacing().item_spacing.y);
}

#[cfg(test)]
mod tests {
    use egui::accesskit;

    use super::*;

    /// The id the test modals and their areas go by.
    const ID: &str = "modal_test";

    /// The test context: the suite's theme installed, AccessKit on, so the
    /// title's label has a node.
    fn context() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        crate::theme::install(&ctx);
        ctx
    }

    /// Draws `title` over `body` through `modal` for a few frames on an
    /// 800 by 600 screen, so the area settles past its sizing pass. Returns
    /// the last frame's `body` result, the modal's outer rect and the
    /// title's rect.
    fn draw<R>(
        ctx: &egui::Context,
        title: &str,
        body: impl FnMut(&mut egui::Ui) -> R,
    ) -> (R, egui::Rect, egui::Rect) {
        draw_through(ctx, title, false, body)
    }

    /// `draw`, through `modal_fit_contents` when `fit_contents`.
    fn draw_through<R>(
        ctx: &egui::Context,
        title: &str,
        fit_contents: bool,
        mut body: impl FnMut(&mut egui::Ui) -> R,
    ) -> (R, egui::Rect, egui::Rect) {
        let mut last = None;
        let mut nodes = Vec::new();
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                let id = egui::Id::unique(ID);
                let response = if fit_contents {
                    modal_fit_contents(ui.ctx(), id, title, &mut body)
                } else {
                    modal(ui.ctx(), id, title, &mut body)
                };
                last = Some(response.inner);
            });
            nodes = output
                .platform_output
                .accesskit_update
                .take()
                .expect("AccessKit is enabled on the context")
                .nodes
                .into_iter()
                .map(|(_, node)| node)
                .collect::<Vec<accesskit::Node>>();
            // Nothing renders here, so the frame's texture deltas are dropped
            // on purpose.
            output.drop_without_applying_deltas();
        }
        let outer = ctx
            .memory(|memory| memory.area_rect(egui::Id::unique(ID)))
            .expect("the modal's area has a rect");
        let title = nodes
            .iter()
            .find(|node| node.value() == Some(title))
            .and_then(accesskit::Node::bounds)
            .expect("the title's label has bounds");
        let title = egui::Rect::from_min_max(
            egui::pos2(title.x0 as f32, title.y0 as f32),
            egui::pos2(title.x1 as f32, title.y1 as f32),
        );
        (last.expect("the modal was drawn"), outer, title)
    }

    /// Asserts `actual` is `expected` within half a point, naming the edge.
    fn assert_near(actual: f32, expected: f32, edge: &str) {
        assert!(
            (actual - expected).abs() <= 0.5,
            "{edge}: {actual}, expected {expected}"
        );
    }

    #[test]
    fn a_short_dialog_is_360_wide_with_its_button_at_the_right_under_the_body() {
        let ctx = context();
        let ((label, ok), outer, title) = draw(&ctx, "Kit slot assignments", |ui| {
            let label = ui.weak("(this version does not store kit slots)").rect;
            let ok = modal_buttons(ui, |ui| ui.button("OK").rect);
            (label, ok)
        });
        assert_near(outer.width(), 360.0, "the modal's width");
        assert_near(title.left(), outer.left() + 13.0, "the title's left");
        assert_near(title.top(), outer.top() + 9.0, "the title's top");
        assert_near(label.top(), title.bottom() + 8.0, "the body's top");
        assert_near(ok.right(), outer.right() - 13.0, "OK's right");
        assert_near(ok.top(), label.bottom() + 10.0, "OK's top");
        assert_near(outer.bottom(), ok.bottom() + 9.0, "the modal's bottom");
    }

    #[test]
    fn a_short_dialog_fitting_its_contents_is_as_wide_as_its_widest_line() {
        let ctx = context();
        let (label, outer, title) = draw_through(&ctx, "Kit slot assignments", true, |ui| {
            let label = ui.weak("(this version does not store kit slots)").rect;
            modal_buttons(ui, |ui| ui.button("OK"));
            label
        });
        assert!(outer.width() < 360.0, "the modal's width {}", outer.width());
        assert_near(
            outer.width(),
            label.width().max(title.width()) + 26.0,
            "the modal's width",
        );
    }

    #[test]
    fn a_sentence_stands_8_over_the_control_under_it_and_10_over_the_buttons_when_last() {
        let ctx = context();
        let ((sentence, field, last, ok), _, _) = draw(&ctx, "Set all stats", |ui| {
            let sentence = ui.label("Set all of the player's ability stats to").rect;
            modal_sentence_gap(ui);
            let field = ui.button("75").rect;
            let last = ui.label("Every ability takes the number.").rect;
            let ok = modal_buttons(ui, |ui| ui.button("OK").rect);
            (sentence, field, last, ok)
        });
        assert_near(field.top(), sentence.bottom() + 8.0, "the control's top");
        assert_near(ok.top(), last.bottom() + 10.0, "OK's top");
    }

    #[test]
    fn a_wide_dialogs_button_row_ends_at_its_body_and_does_not_widen_it() {
        let ctx = context();
        let ((close, copy), outer, _) = draw(&ctx, "Copy layout", |ui| {
            // A body exactly 500 points wide, whatever the font.
            ui.allocate_exact_size(egui::vec2(500.0, 40.0), egui::Sense::hover());
            modal_buttons(ui, |ui| {
                let close = ui.button("Close").rect;
                let copy = ui.button("Copy").rect;
                (close, copy)
            })
        });
        let spacing = ctx.global_style().spacing.item_spacing.x;
        assert_near(outer.width(), 500.0 + 26.0, "the modal's width");
        assert_near(close.right(), outer.right() - 13.0, "Close's right");
        assert_near(copy.right(), close.left() - spacing, "Copy's right");
    }
}
