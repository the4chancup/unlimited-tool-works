//! The widgets several tools' views share: `BottomDock`, the collapsible bottom
//! panel's frame, `LogPanel`,
//! the run log drawn in it, `well_frame`, the log's well a dock's rows sit in,
//! the disabled widget and scope every view draws (`add_enabled`,
//! `add_enabled_ui`),
//! the settings rows (`settings_row`) the menu's
//! sections share, the checkbox and radio button every view draws
//! (`choice`), the text and number boxes every view draws (`text_box`,
//! `number_box`) and the text box's resting
//! outline (`text_box_border`), the collapsing
//! header every view draws (`collapsing_header`), a tool's
//! toolbar row with its group separator, its framed scope and the Ctrl-only
//! key its shortcuts match (`toolbar`), the measure that
//! places text by its ink (`ink`), the button frame a clickable picture shows while in
//! use (`in_use_frame`), the slider with its value box (`slider`), the wheel stepping a combo box or a drag value (`wheel`), the modal window
//! every dialog draws through (`modal`), the spacing of stacked fields
//! and checkboxes (`FIELD_SPACING`, `CHECKBOX_SPACING`, `checkbox_row`) and of a
//! grid of text rows (`TEXT_ROW_SPACING`), and the resting edge a widget
//! painting its own state draws (`at_rest`, `resting_edge`) or a row
//! takes out (`row`).
//! Where they sit in a tool's view is that tool's choice; the widgets
//! themselves know no tool.

mod choice;
mod collapsing_header;
mod combo;
mod dock;
mod enabled;
mod in_use_frame;
mod ink;
mod log_panel;
mod menu_button;
mod modal;
mod settings_row;
mod slider;
mod text_box;
mod toolbar;
mod wheel;

pub use choice::{checkbox, checkbox_row, checkbox_without_text, radio, radio_value};
pub use collapsing_header::collapsing_header;
pub use combo::{combo_item, combo_label, combo_separator};
pub use dock::BottomDock;
pub use enabled::{add_enabled, add_enabled_ui};
pub use in_use_frame::in_use_frame;
pub use ink::ink_span;
pub use log_panel::{LogLine, LogPanel, well_frame};
pub use menu_button::{menu_button, menu_separator};
pub use modal::{
    dialog_form, dialog_page, modal, modal_buttons, modal_fit_contents, modal_frame,
    modal_sentence_gap,
};
pub(crate) use settings_row::{PathCommit, path_row, path_row_commit, row_label};
pub use slider::slider;
pub use text_box::{number_box, text_box, text_box_border};
pub use toolbar::{consume_ctrl, toolbar, toolbar_framed, toolbar_separator};
pub use wheel::{
    cycle_choice, number_arrows, number_wheel_steps, set_inverted, step_choice, step_number,
    wheel_steps,
};

/// The row spacing of a grid of text rows (the help's Messages table, the Save
/// editor's changes table): the drafts' 2 points above and below each row's text.
/// Rows a line of text high stand 19 apart, once the grid's `min_row_height` lets
/// them be shorter than egui's 18-point control.
pub const TEXT_ROW_SPACING: f32 = 4.0;

/// The space between a form's stacked 18-point fields, whose rows then stand 24
/// apart: an `egui::Grid`'s row spacing, or the `item_spacing.y` of rows stacked
/// in a vertical layout. Stacked controls are spaced by the gap between their
/// boxes, not by one row pitch, because a checkbox's square is 4 points shorter
/// than a field: see `CHECKBOX_SPACING` and `CHECKBOX_ROW_HEIGHT`.
pub const FIELD_SPACING: f32 = 6.0;

/// Whether `response` is a live widget at rest: enabled, and in the state
/// egui's `Widgets::style` answers with the `inactive` visuals (not hovered,
/// highlighted, pressed, focused or clicked). A control's edge at rest is
/// the theme's resting stroke; the painted marks' and rows' edges follow it,
/// so they read it here rather than each re-deriving the state.
pub fn at_rest(ui: &egui::Ui, response: &egui::Response) -> bool {
    response.enabled()
        && std::ptr::eq(
            ui.style().interact(response),
            &ui.visuals().widgets.inactive,
        )
}

/// Adds `widget` with the theme's resting stroke taken out for that one
/// widget (saved and put back around the `ui.add`), because a row is not a
/// control: a picked row's blue draws no edge, and the next widget in `ui`
/// keeps its own.
pub fn row(ui: &mut egui::Ui, widget: impl egui::Widget) -> egui::Response {
    let resting = ui.visuals().widgets.inactive.bg_stroke;
    ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;
    let response = ui.add(widget);
    ui.visuals_mut().widgets.inactive.bg_stroke = resting;
    response
}

/// The edge a widget painting its own resting state asks for: `color`
/// stroked 1 point inside `response.rect` with the theme's corner radius
/// while the widget is at rest (`at_rest`), nothing in another state — a
/// hovered, pressed or focused widget keeps the frame egui draws. A toggle's
/// held side takes its fill's edge (the theme's `edge_color`), its unheld
/// side the panel's (`unheld_edge`), each a colour the theme's resting
/// stroke on the widgets' grey cannot give.
pub fn resting_edge(ui: &egui::Ui, response: &egui::Response, color: egui::Color32) {
    if at_rest(ui, response) {
        ui.painter().rect_stroke(
            response.rect,
            ui.visuals().widgets.inactive.corner_radius,
            egui::Stroke::new(1.0, color),
            egui::StrokeKind::Inside,
        );
    }
}

/// The space between checkboxes stacked one under another, as the
/// `item_spacing.y` of their vertical layout or the row spacing of a grid of
/// checkboxes alone: their 18-point rows stand 22 apart, which leaves 8 points
/// between their 14-point squares. A field's `FIELD_SPACING` would leave 10
/// between squares, since the square is 4 points shorter than a field's box.
pub const CHECKBOX_SPACING: f32 = 4.0;

/// The height of a row of checkboxes alone in a grid of fields, which has one row
/// spacing, `FIELD_SPACING`. The 14-point square is centred in the row, 1 point
/// from its top and bottom, so it stands 7 under a field's box, 8 under the square
/// of a checkbox row above it and 7 above the next field's box. A checkbox's own
/// 18-point row would leave 10 between two squares. `checkbox_row` draws one.
pub const CHECKBOX_ROW_HEIGHT: f32 = 16.0;

#[cfg(test)]
mod tests {
    use super::*;

    /// A context with the suite's look installed, as the window has it.
    fn context() -> egui::Context {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        ctx
    }

    /// One 800 by 600 frame drawing `contents` with `events` as its
    /// input; the frame's shapes.
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
        // Nothing renders here, so the frame's texture deltas are dropped
        // on purpose.
        output.drop_without_applying_deltas();
        shapes
    }

    /// The rect shapes painted exactly over `rect`.
    fn frames_at(
        shapes: &[egui::epaint::Shape],
        rect: egui::Rect,
    ) -> Vec<&egui::epaint::RectShape> {
        shapes
            .iter()
            .filter_map(|shape| match shape {
                egui::Shape::Rect(shape) if shape.rect == rect => Some(shape),
                _ => None,
            })
            .collect()
    }

    /// The strokes of the rect shapes painted exactly over `rect`, the
    /// empty ones left out (an invisible frame strokes nothing).
    fn strokes_at(shapes: &[egui::epaint::Shape], rect: egui::Rect) -> Vec<egui::Stroke> {
        frames_at(shapes, rect)
            .into_iter()
            .map(|frame| frame.stroke)
            .filter(|stroke| !stroke.is_empty())
            .collect()
    }

    // A row is not a control: the picked row fills in the selection blue
    // and draws no edge, while the next widget keeps the theme's resting
    // stroke (`row` takes it out for the one widget alone).
    #[test]
    fn a_row_draws_its_blue_with_no_edge_and_the_next_widget_keeps_its_own() {
        let ctx = context();
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
                row(ui, egui::Button::selectable(true, "Pick")).rect,
                ui.button("Plain").rect,
            );
        };
        // egui styles a widget from its state on the frame before.
        frame(&ctx, Vec::new(), &mut draw);
        let shapes = frame(&ctx, Vec::new(), &mut draw);
        let [picked] = frames_at(&shapes, rects.0)[..] else {
            panic!("one frame over the picked row {:?}", rects.0);
        };
        assert_eq!(
            picked.fill, visuals.selection.bg_fill,
            "the picked row's blue"
        );
        assert_eq!(
            picked.stroke,
            egui::Stroke::NONE,
            "a row draws no resting edge"
        );
        let [plain] = frames_at(&shapes, rects.1)[..] else {
            panic!("one frame over the plain button {:?}", rects.1);
        };
        assert_eq!(plain.stroke, edge, "the next widget keeps the edge");
    }

    // The self-painted edge shows at rest alone: hovered, the widget's
    // own frame carries the state's stroke and `resting_edge` paints
    // nothing over it.
    #[test]
    fn resting_edge_paints_its_color_only_at_rest() {
        let ctx = context();
        let visuals = ctx.style_of(egui::Theme::Dark).visuals.clone();
        let color = crate::theme::edge_color(&visuals, visuals.selection.bg_fill);
        let rect_cell = std::cell::Cell::new(egui::Rect::NOTHING);
        let mut draw = |ui: &mut egui::Ui| {
            let response = ui.add(egui::Button::selectable(false, "Side"));
            resting_edge(ui, &response, color);
            rect_cell.set(response.rect);
        };
        frame(&ctx, Vec::new(), &mut draw);
        let shapes = frame(&ctx, Vec::new(), &mut draw);
        let rect = rect_cell.get();
        assert_eq!(
            strokes_at(&shapes, rect),
            [egui::Stroke::new(1.0, color)],
            "at rest the side's edge is the painted one"
        );

        // Hovered: egui's own frame alone, nothing painted over it.
        let at = rect.center();
        frame(&ctx, vec![egui::Event::PointerMoved(at)], &mut draw);
        let shapes = frame(&ctx, vec![egui::Event::PointerMoved(at)], &mut draw);
        assert_eq!(
            strokes_at(&shapes, rect),
            [visuals.widgets.hovered.bg_stroke],
            "hovered keeps egui's own frame"
        );
    }
}
