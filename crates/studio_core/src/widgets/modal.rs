//! The modal window every dialog draws through, the drafts' `.window`: its
//! frame, its title over the body, and its row of buttons at the right.

use super::FIELD_SPACING;

/// The drafts' `.window` frame: egui's popup frame with 12 points of margin across and 8 down.
pub fn modal_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::popup(style).inner_margin(egui::Margin::symmetric(12, 8))
}

/// The drafts' `.window` minimum width, margins and stroke included.
const MIN_WIDTH: f32 = 360.0;

/// `egui::Modal` under `id` in `modal_frame`, at least 360 points wide with its margins and
/// stroke: `title` centred over the dialog (`theme::window_title_text`), 12 points, then
/// `body`.
pub fn modal<R>(
    ctx: &egui::Context,
    id: egui::Id,
    title: impl Into<egui::RichText>,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::ModalResponse<R> {
    modal_with_min_width(ctx, id, title, Some(MIN_WIDTH), body)
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
///
/// The title is centred over the modal's final width, which the body decides: a wide
/// sentence widens the modal past its minimum. So the title's row is reserved first, its
/// width counting towards the modal's, and the title is drawn in it once the body is laid
/// out. A centring layout around the title alone would centre it over all the room the
/// screen leaves, not over the modal.
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
            titled_body(ui, title, body)
        })
}

/// A dialog's look as a page (a tool's start page with nothing open):
/// the modal's frame and centred title in the middle of `ui`'s rect, at
/// least `modal`'s 360 points wide, no backdrop and nothing outside the
/// frame blocked. A real `modal` would block the whole window, the
/// sidebar's tool switch included.
pub fn dialog_page<R>(
    ui: &egui::Ui,
    id: egui::Id,
    title: impl Into<egui::RichText>,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let rect = ui.max_rect();
    egui::Area::new(id)
        .sense(egui::Sense::hover())
        .pivot(egui::Align2::CENTER_CENTER)
        .fixed_pos(rect.center())
        .constrain_to(rect)
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ui.ctx(), |ui| {
            // The extra scope keeps a click inside the frame from
            // passing through to whatever is under it, as a modal's.
            ui.scope_builder(
                egui::UiBuilder::new().sense(egui::Sense::CLICK | egui::Sense::DRAG),
                |ui| {
                    let frame = modal_frame(ui.style());
                    let margins = frame.total_margin().sum().x;
                    frame
                        .show(ui, |ui| {
                            ui.set_min_width(MIN_WIDTH - margins);
                            titled_body(ui, title, body)
                        })
                        .inner
                },
            )
            .inner
        })
}

/// `title` centred over `body` as the modal draws them: the title's row
/// is reserved first, so `body` widens the frame, and the title is
/// drawn once the body's width is known. The body's result.
fn titled_body<R>(
    ui: &mut egui::Ui,
    title: impl Into<egui::RichText>,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let title = egui::WidgetText::from(crate::theme::window_title_text(title)).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::FontSelection::Default,
    );
    let (_, title_row) = ui.allocate_space(title.size());
    ui.add_space(TITLE_SPACING - ui.spacing().item_spacing.y);
    let inner = body(ui);
    let title_row = egui::Rect::from_x_y_ranges(ui.min_rect().x_range(), title_row.y_range());
    ui.new_child(
        egui::UiBuilder::new()
            .max_rect(title_row)
            .layout(egui::Layout::top_down(egui::Align::Center)),
    )
    .label(title);
    inner
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

/// The space between a dialog's title and its body.
const TITLE_SPACING: f32 = 12.0;

/// The space between a dialog's sentence and the element above or under it, the drafts'
/// `.window p` margin.
const SENTENCE_SPACING: f32 = 8.0;

/// The space between a dialog's form and what follows it.
const DIALOG_FORM_GAP: f32 = 6.0;

/// A dialog's own form, a label and its control on each row, as an `egui::Grid` of two
/// columns under `id_salt`. Its rows leave `FIELD_SPACING` between their boxes, so its
/// 18-point rows stand 24 apart, and what follows the form stands 6 under its last row,
/// where egui would leave 3. A form never ends a dialog's body: the buttons' own gap
/// would add to the 6.
pub fn dialog_form<R>(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let form = egui::Grid::new(id_salt)
        .num_columns(2)
        .spacing(egui::vec2(ui.spacing().item_spacing.x, FIELD_SPACING))
        .show(ui, add_contents);
    ui.add_space(DIALOG_FORM_GAP - ui.spacing().item_spacing.y);
    form
}

/// The gap between a dialog's sentence and a sentence or control under it, or between a
/// control and a sentence under it: 8 points, where a label alone leaves egui's 3. A
/// sentence that ends the body takes none: the buttons' 10 under it stands for both margins.
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

    /// Draws `contents` for a few frames on a `screen`-sized screen, so an
    /// area settles past its sizing pass. Returns the last frame's
    /// `contents` result and its AccessKit nodes.
    fn run<R>(
        ctx: &egui::Context,
        screen: egui::Vec2,
        mut contents: impl FnMut(&mut egui::Ui) -> R,
    ) -> (R, Vec<accesskit::Node>) {
        let mut last = None;
        let mut nodes = Vec::new();
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, screen)),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                last = Some(contents(ui));
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
        (last.expect("contents ran"), nodes)
    }

    /// The rect of `title`'s label node (a label's text rows have nodes of
    /// their own carrying its value, so the title's label is told apart by
    /// its role).
    fn title_rect(nodes: &[accesskit::Node], title: &str) -> egui::Rect {
        let titles = nodes
            .iter()
            .filter(|node| node.role() == accesskit::Role::Label && node.value() == Some(title))
            .collect::<Vec<_>>();
        assert_eq!(titles.len(), 1, "the title's label nodes");
        let title = titles[0].bounds().expect("the title's label has bounds");
        egui::Rect::from_min_max(
            egui::pos2(title.x0 as f32, title.y0 as f32),
            egui::pos2(title.x1 as f32, title.y1 as f32),
        )
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
        let (inner, nodes) = run(ctx, egui::vec2(800.0, 600.0), |ui| {
            let id = egui::Id::unique(ID);
            let response = if fit_contents {
                modal_fit_contents(ui.ctx(), id, title, &mut body)
            } else {
                modal(ui.ctx(), id, title, &mut body)
            };
            response.inner
        });
        let outer = ctx
            .memory(|memory| memory.area_rect(egui::Id::unique(ID)))
            .expect("the modal's area has a rect");
        (inner, outer, title_rect(&nodes, title))
    }

    /// The id the test `dialog_page`s go by.
    const PAGE_ID: &str = "dialog_page_test";

    /// `dialog_page` with `rect` as its page's rect (the child ui it is
    /// drawn in), for a few frames on a screen `rect` does not fill.
    /// Returns the last frame's `body` result, the page's outer rect and
    /// the title's rect.
    fn draw_page<R>(
        ctx: &egui::Context,
        rect: egui::Rect,
        title: &str,
        mut body: impl FnMut(&mut egui::Ui) -> R,
    ) -> (R, egui::Rect, egui::Rect) {
        let (inner, nodes) = run(ctx, egui::vec2(960.0, 720.0), |ui| {
            let page = ui.new_child(egui::UiBuilder::new().max_rect(rect));
            dialog_page(&page, egui::Id::unique(PAGE_ID), title, &mut body).inner
        });
        let outer = ctx
            .memory(|memory| memory.area_rect(egui::Id::unique(PAGE_ID)))
            .expect("the page's area has a rect");
        (inner, outer, title_rect(&nodes, title))
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
        assert_near(title.center().x, outer.center().x, "the title's centre");
        assert_near(title.top(), outer.top() + 9.0, "the title's top");
        assert_near(label.top(), title.bottom() + 12.0, "the body's top");
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
    fn a_body_wider_than_the_minimum_widens_the_modal_and_the_title_stays_centred_over_it() {
        let ctx = context();
        let (_, outer, title) = draw(&ctx, "Kit slot assignments", |ui| {
            // A body exactly 600 points wide, whatever the font.
            ui.allocate_exact_size(egui::vec2(600.0, 20.0), egui::Sense::hover());
            modal_buttons(ui, |ui| ui.button("OK"));
        });
        assert_near(outer.width(), 600.0 + 26.0, "the modal's width");
        assert_near(title.center().x, outer.center().x, "the title's centre");
        assert_near(title.top(), outer.top() + 9.0, "the title's top");
    }

    #[test]
    fn a_title_wider_than_its_body_sizes_a_modal_fitting_its_contents() {
        let ctx = context();
        let (_, outer, title) = draw_through(
            &ctx,
            "Kit slot assignments for every player of the team",
            true,
            |ui| {
                ui.label("Done.");
                modal_buttons(ui, |ui| ui.button("OK"));
            },
        );
        assert_near(outer.width(), title.width() + 26.0, "the modal's width");
        assert_near(title.left(), outer.left() + 13.0, "the title's left");
        assert_near(title.center().x, outer.center().x, "the title's centre");
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
    fn a_dialog_forms_rows_stand_24_apart_and_what_follows_it_6_under_its_last_row() {
        let ctx = context();
        // Each row's control is exactly 18 points high, whatever the font.
        let control = |ui: &mut egui::Ui| {
            ui.allocate_exact_size(egui::vec2(120.0, 18.0), egui::Sense::hover())
                .0
        };
        let ((first, second, after), _, _) = draw(&ctx, "Save pictures", |ui| {
            let rows = dialog_form(ui, "form", |ui| {
                ui.label("Folder");
                let first = control(ui);
                ui.end_row();
                ui.label("File name");
                let second = control(ui);
                ui.end_row();
                (first, second)
            })
            .inner;
            (rows.0, rows.1, control(ui))
        });
        assert_near(second.top(), first.top() + 24.0, "the second row's top");
        assert_near(
            after.top(),
            second.bottom() + 6.0,
            "the control after the form's top",
        );
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

    // The page form centres the modal's frame and title on the rect it is
    // drawn in — a rect offset from the screen's origin, so a slip to the
    // screen's centre shows — and registers no modal layer, where `modal`
    // itself does.
    #[test]
    fn a_dialog_page_centres_on_its_rect_and_registers_no_modal_layer() {
        let ctx = context();
        let rect = egui::Rect::from_min_max(egui::pos2(60.0, 0.0), egui::pos2(860.0, 600.0));
        let ((), outer, title) = draw_page(&ctx, rect, "Ruleset editor", |ui| {
            ui.label("the start page's entries");
        });
        assert!(
            (outer.center().x - rect.center().x).abs() <= 1.0
                && (outer.center().y - rect.center().y).abs() <= 1.0,
            "the frame's centre is the rect's: {outer:?} vs {rect:?}"
        );
        assert_near(title.center().x, outer.center().x, "the title's centre");
        assert!(
            ctx.memory(|memory| memory.top_modal_layer()).is_none(),
            "the page registers no modal layer"
        );

        // The premise: `modal` drawn in a frame registers one.
        let ((), _) = run(&ctx, egui::vec2(960.0, 720.0), |ui| {
            modal(ui.ctx(), egui::Id::unique("premise"), "premise", |ui| {
                ui.label("x");
            });
        });
        assert!(
            ctx.memory(|memory| memory.top_modal_layer()).is_some(),
            "premise: a modal registers a layer"
        );
    }

    // A short page is as wide as a short dialog: `modal`'s 360 points.
    #[test]
    fn a_short_dialog_page_is_360_wide() {
        let ctx = context();
        let rect = egui::Rect::from_min_max(egui::pos2(60.0, 0.0), egui::pos2(860.0, 600.0));
        let ((), outer, _) = draw_page(&ctx, rect, "Save editor", |ui| {
            ui.label("PES 2021");
        });
        assert_near(outer.width(), 360.0, "the page's width");
    }
}
