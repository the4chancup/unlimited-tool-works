//! The suite's checkbox and radio button: egui's own, laid out and sized as
//! egui lays them, with a checked box or a selected radio button filled in the
//! selection blue.
//!
//! The blue is painted over the box or circle egui has just drawn, in the
//! hover and press states egui drew it in. egui's own hook for a checkbox's
//! look (`CheckboxStyle`) is public only behind its `experimental` feature,
//! and its radio button has none, so painting over is the one way that
//! covers both.

/// The width of a checked box's tick in every state. egui draws it in the
/// state's text stroke, 1 point at rest, which is too thin to read on the blue.
const TICK_WIDTH: f32 = 2.0;

/// A checkbox labelled `text`: egui's `Checkbox`, a checked box filled in the
/// selection blue with its tick in the selection's text colour. egui draws a
/// checked box on the same grey as an unchecked one, with a thin tick in the
/// label's colour, so a choice made hardly stands out; the suite's must.
pub fn checkbox(
    ui: &mut egui::Ui,
    checked: &mut bool,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    #[expect(
        clippy::disallowed_methods,
        reason = "the suite's checkbox is egui's with the blue painted over it"
    )]
    let response = ui.add(egui::Checkbox::new(checked, text.into()));
    if *checked {
        paint_checked_box(ui, &response);
    }
    response
}

/// [`checkbox`] with no label of its own, for a row whose label is drawn
/// beside it (pair it with `Response::labelled_by`).
pub fn checkbox_without_text(ui: &mut egui::Ui, checked: &mut bool) -> egui::Response {
    #[expect(
        clippy::disallowed_methods,
        reason = "the suite's checkbox is egui's with the blue painted over it"
    )]
    let response = ui.add(egui::Checkbox::without_text(checked));
    if *checked {
        paint_checked_box(ui, &response);
    }
    response
}

/// A radio button labelled `text`, drawn `selected` or not: egui's
/// `RadioButton`, a selected one filled in the selection blue with its dot in
/// the selection's text colour, for the same reason as [`checkbox`]. A click
/// is the caller's to answer; [`radio_value`] answers it.
pub fn radio(
    ui: &mut egui::Ui,
    selected: bool,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    #[expect(
        clippy::disallowed_methods,
        reason = "the suite's radio button is egui's with the blue painted over it"
    )]
    let response = ui.add(egui::RadioButton::new(selected, text.into()));
    if selected {
        paint_selected_radio(ui, &response);
    }
    response
}

/// The rects egui draws a checkbox's or radio button's mark and box in, as
/// `Spacing::icon_rectangles` gives them: the box at the left of the
/// widget's content, centred on its row. egui aligns the content inside the
/// response's rect as the ui's layout aligns widgets, so in a justified cell
/// wider than the content (a centred one, say) the box is not at the rect's
/// left. The content is as wide as the widget's unwrapped size, or the rect
/// when its label wraps.
fn icon_rectangles(ui: &egui::Ui, response: &egui::Response) -> (egui::Rect, egui::Rect) {
    let rect = response.rect;
    let width = response
        .intrinsic_size()
        .map_or(rect.width(), |size| size.x.min(rect.width()));
    let x_range = ui
        .layout()
        .horizontal_align()
        .align_size_within_range(width, rect.x_range());
    ui.spacing()
        .icon_rectangles(egui::Rect::from_x_y_ranges(x_range, rect.y_range()))
}

/// Paints a checked box over the one egui drew for `response`: the
/// selection blue inside the state's border and corners, then the tick
/// across the box's central 8 points, where egui draws its own.
fn paint_checked_box(ui: &egui::Ui, response: &egui::Response) {
    let visuals = ui.style().interact(response);
    let selection = ui.visuals().selection;
    let (tick, frame) = icon_rectangles(ui, response);
    let painter = ui.painter();
    painter.rect(
        frame,
        visuals.corner_radius,
        selection.bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );
    painter.line(
        vec![tick.left_center(), tick.center_bottom(), tick.right_top()],
        egui::Stroke::new(TICK_WIDTH, selection.stroke.color),
    );
}

/// Paints a selected radio button over the one egui drew for `response`:
/// the selection blue inside the state's border, then the dot, a third of
/// the circle's central 8 points, as egui sizes its own.
fn paint_selected_radio(ui: &egui::Ui, response: &egui::Response) {
    let visuals = ui.style().interact(response);
    let selection = ui.visuals().selection;
    let (dot, frame) = icon_rectangles(ui, response);
    let painter = ui.painter();
    painter.circle(
        frame.center(),
        frame.width() / 2.0 + visuals.expansion,
        selection.bg_fill,
        visuals.bg_stroke,
    );
    painter.circle_filled(dot.center(), dot.width() / 3.0, selection.stroke.color);
}

/// A radio button that sets `current` to `alternative` when clicked, selected
/// while they are equal: egui's `Ui::radio_value` drawn as [`radio`].
pub fn radio_value<T: PartialEq>(
    ui: &mut egui::Ui,
    current: &mut T,
    alternative: T,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    let mut response = radio(ui, *current == alternative, text);
    if response.clicked() && *current != alternative {
        *current = alternative;
        response.mark_changed();
    }
    response
}

#[cfg(test)]
mod tests {
    use egui::epaint::{ColorMode, Shape};

    use super::*;

    /// The selection blue a checked box or selected radio button is filled in.
    const BLUE: egui::Color32 = egui::Color32::from_rgb(0x00, 0x5c, 0x80);
    /// The selection's text colour the tick and the dot are drawn in.
    const TICK: egui::Color32 = egui::Color32::from_rgb(0xc0, 0xde, 0xff);
    /// The 1-point border a hovered box gets inside its edge.
    const HOVER_BORDER: egui::Color32 = egui::Color32::from_rgb(0x96, 0x96, 0x96);

    /// A context with the suite's look installed, as the window has it.
    fn context() -> egui::Context {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        ctx
    }

    /// One 800 by 600 frame drawing `contents` in a top-down panel, the pointer
    /// at `pointer` when given; the frame's shapes and the rects `contents`
    /// returned.
    fn frame(
        ctx: &egui::Context,
        pointer: Option<egui::Pos2>,
        contents: &mut impl FnMut(&mut egui::Ui) -> Vec<egui::Rect>,
    ) -> (Vec<Shape>, Vec<egui::Rect>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            events: pointer.into_iter().map(egui::Event::PointerMoved).collect(),
            ..Default::default()
        };
        let mut rects = Vec::new();
        let mut output = ctx.run_ui(input, |ui| rects = contents(ui));
        let shapes = std::mem::take(&mut output.shapes)
            .into_iter()
            .map(|clipped| clipped.shape)
            .collect();
        // Nothing renders here, so the frame's texture deltas are dropped on
        // purpose.
        output.drop_without_applying_deltas();
        (shapes, rects)
    }

    /// The shapes of the frame after `contents` has been drawn twice: egui
    /// styles a checkbox from its state on the frame before, so a hover shows
    /// from the second frame on.
    fn settled(
        pointer: Option<egui::Pos2>,
        mut contents: impl FnMut(&mut egui::Ui) -> Vec<egui::Rect>,
    ) -> (Vec<Shape>, Vec<egui::Rect>) {
        let ctx = context();
        frame(&ctx, pointer, &mut contents);
        frame(&ctx, pointer, &mut contents)
    }

    /// The filled rects painted inside `area`.
    fn rects_in(shapes: &[Shape], area: egui::Rect) -> Vec<&egui::epaint::RectShape> {
        shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::Rect(rect) if area.contains_rect(rect.rect) => Some(rect),
                _ => None,
            })
            .collect()
    }

    /// The circles painted inside `area`.
    fn circles_in(shapes: &[Shape], area: egui::Rect) -> Vec<&egui::epaint::CircleShape> {
        shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::Circle(circle) if area.contains(circle.center) => Some(circle),
                _ => None,
            })
            .collect()
    }

    /// The `(width, colour)` of every solid-coloured line painted inside `area`,
    /// with its points.
    fn lines_in(shapes: &[Shape], area: egui::Rect) -> Vec<(f32, egui::Color32, Vec<egui::Pos2>)> {
        shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::Path(path) if path.points.iter().all(|point| area.contains(*point)) => {
                    match path.stroke.color {
                        ColorMode::Solid(color) => {
                            Some((path.stroke.width, color, path.points.clone()))
                        }
                        ColorMode::UV(_) => None,
                    }
                }
                _ => None,
            })
            .collect()
    }

    /// A checked and an unchecked checkbox, one under the other; their rects.
    fn two_checkboxes(ui: &mut egui::Ui) -> Vec<egui::Rect> {
        vec![
            checkbox(ui, &mut true, "Show suggestions").rect,
            checkbox(ui, &mut false, "Scroll names").rect,
        ]
    }

    /// The tick's polyline in a 14-point box at `min`: from the box's
    /// (3, 7) down to (7, 11) and up to (11, 3).
    fn tick_points(min: egui::Pos2) -> Vec<egui::Pos2> {
        [(3.0, 7.0), (7.0, 11.0), (11.0, 3.0)]
            .map(|(x, y)| min + egui::vec2(x, y))
            .to_vec()
    }

    /// Asserts `area` holds a 14-point box filled blue at its left edge,
    /// stroked with `border` and rounded `corner`, and the 2-point tick across
    /// it in the selection's text colour.
    fn assert_checked_box(shapes: &[Shape], area: egui::Rect, border: egui::Stroke, corner: u8) {
        let blue: Vec<_> = rects_in(shapes, area)
            .into_iter()
            .filter(|rect| rect.fill == BLUE)
            .collect();
        let [box_shape] = blue[..] else {
            panic!("one blue box in {area:?}, not {blue:?}");
        };
        let expected = egui::Rect::from_center_size(
            egui::pos2(area.left() + 7.0, area.center().y),
            egui::vec2(14.0, 14.0),
        );
        assert_eq!(
            box_shape.rect, expected,
            "the box is 14 points at the row's left"
        );
        assert!(
            rects_in(shapes, area)
                .iter()
                .any(|rect| rect.fill != BLUE && rect.rect == expected),
            "the blue covers egui's own box exactly"
        );
        assert_eq!(box_shape.stroke, border, "the box's border");
        assert_eq!(box_shape.stroke_kind, egui::StrokeKind::Inside);
        assert_eq!(box_shape.corner_radius, egui::CornerRadius::same(corner));
        let ticks: Vec<_> = lines_in(shapes, area)
            .into_iter()
            .filter(|(_, color, _)| *color == TICK)
            .collect();
        assert_eq!(
            ticks,
            vec![(2.0, TICK, tick_points(expected.min))],
            "one 2-point tick in the selection's text colour"
        );
    }

    #[test]
    fn a_checked_checkbox_is_filled_blue_with_a_two_point_tick() {
        let (shapes, rects) = settled(None, two_checkboxes);
        assert_checked_box(&shapes, rects[0], egui::Stroke::NONE, 2);
        assert!(
            rects_in(&shapes, rects[1])
                .iter()
                .all(|rect| rect.fill != BLUE),
            "an unchecked box keeps egui's grey"
        );
    }

    #[test]
    fn a_hovered_checked_checkbox_keeps_its_blue_inside_the_hover_border() {
        let probe = settled(None, two_checkboxes).1;
        // The pointer over the label, right of the box: egui's response covers
        // the whole row.
        let pointer = probe[0].right_center() - egui::vec2(10.0, 0.0);
        let (shapes, rects) = settled(Some(pointer), two_checkboxes);
        assert_checked_box(&shapes, rects[0], egui::Stroke::new(1.0, HOVER_BORDER), 2);
    }

    #[test]
    fn a_selected_radio_button_is_a_blue_circle_with_a_light_dot() {
        let (shapes, rects) = settled(None, |ui| {
            let mut mode = 1;
            vec![
                radio_value(ui, &mut mode, 1, "Current team").rect,
                radio_value(ui, &mut mode, 2, "Select teams").rect,
            ]
        });
        let selected = circles_in(&shapes, rects[0]);
        let center = egui::pos2(rects[0].left() + 7.0, rects[0].center().y);
        assert!(
            selected.iter().any(|circle| circle.fill == BLUE
                && circle.center == center
                && circle.radius == 7.0),
            "a 14-point blue circle at the button: {selected:?}"
        );
        assert!(
            selected.iter().any(|circle| circle.fill == TICK
                && circle.center == center
                && (circle.radius - 2.667).abs() <= 0.01),
            "a dot of radius 2.667 in the selection's text colour: {selected:?}"
        );
        let unselected = circles_in(&shapes, rects[1]);
        assert!(
            unselected
                .iter()
                .all(|circle| circle.fill != BLUE && circle.fill != TICK),
            "an unselected button has neither: {unselected:?}"
        );
    }

    // The Save editor's captain radio: no label, centred in a justified cell
    // 54 points wide, where egui draws the circle in the cell's middle.
    #[test]
    fn in_a_centred_cell_the_blue_covers_egui_s_own_circle() {
        let (shapes, rects) = settled(None, |ui| {
            let cell = ui.allocate_ui(egui::vec2(54.0, 18.0), |ui| {
                ui.centered_and_justified(|ui| radio(ui, true, "")).inner
            });
            vec![cell.inner.rect]
        });
        let circles = circles_in(&shapes, rects[0]);
        let egui_s = circles
            .iter()
            .find(|circle| circle.radius == 7.0 && circle.fill != BLUE)
            .unwrap_or_else(|| panic!("egui's own circle: {circles:?}"));
        assert!(
            egui_s.center.x - (rects[0].left() + 7.0) > 1.0,
            "egui's circle is not at the cell's left (premise): {circles:?}"
        );
        assert!(
            circles
                .iter()
                .any(|circle| circle.fill == BLUE && circle.center == egui_s.center),
            "the blue circle sits on egui's: {circles:?}"
        );
        assert!(
            circles
                .iter()
                .any(|circle| circle.fill == TICK && circle.center == egui_s.center),
            "the dot sits on egui's circle: {circles:?}"
        );
    }

    /// The rects of `widget`'s response and of a label drawn under it, on a
    /// context of its own.
    fn layout(mut widget: impl FnMut(&mut egui::Ui) -> egui::Response) -> Vec<egui::Rect> {
        settled(None, |ui| {
            let mut rects = Vec::new();
            ui.horizontal(|ui| {
                rects.push(widget(ui).rect);
                rects.push(ui.label("beside").rect);
            });
            rects.push(widget(ui).rect);
            rects.push(ui.label("under").rect);
            rects
        })
        .1
    }

    #[test]
    #[expect(
        clippy::disallowed_methods,
        reason = "the yardstick is egui's own widgets' layout"
    )]
    fn the_checkbox_and_the_radio_button_take_egui_s_own_room() {
        for checked in [true, false] {
            let ours = layout(|ui| {
                let mut value = checked;
                checkbox(ui, &mut value, "Show suggestions")
            });
            let egui_s = layout(|ui| {
                let mut value = checked;
                ui.checkbox(&mut value, "Show suggestions")
            });
            assert_eq!(ours, egui_s, "a checkbox checked: {checked}");
            let ours = layout(|ui| {
                let mut value = checked;
                checkbox_without_text(ui, &mut value)
            });
            let egui_s = layout(|ui| {
                let mut value = checked;
                ui.add(egui::Checkbox::without_text(&mut value))
            });
            assert_eq!(ours, egui_s, "a checkbox without text checked: {checked}");
            let ours = layout(|ui| radio(ui, checked, "Current team"));
            let egui_s = layout(|ui| ui.radio(checked, "Current team"));
            assert_eq!(ours, egui_s, "a radio button selected: {checked}");
        }
    }
}
