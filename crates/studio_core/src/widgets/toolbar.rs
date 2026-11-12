//! A tool's toolbar: one row of menus in egui's `MenuBar`, drawn unframed
//! as a menu bar draws them, with the drafts' separator between its groups
//! and a scope for the few controls that keep their frames.

/// The room a toolbar separator takes beside its line, each side (the
/// drafts' `.tsep` margin).
const SEPARATOR_SIDE: f32 = 2.5;

/// A tool's toolbar row: `add_contents` drawn in egui's `MenuBar`, so with
/// one menu open the pointer entering another menu's button opens that one,
/// as a menu bar does. The bar draws with the menu bar's own style
/// (`egui::containers::menu::menu_style`): its buttons have no frame or
/// fill at rest, and show one under the pointer. A control that must stay
/// framed goes in `toolbar_framed`.
pub fn toolbar<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::MenuBar::new().ui(ui, add_contents)
}

/// `add_contents` drawn inside a toolbar with the window's own style rather
/// than the bar's, so its buttons keep their frames: a combo box or a toggle
/// drawn flat beside the menus reads as one more menu, or as a label.
pub fn toolbar_framed<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.scope(|ui| {
        ui.set_style(ui.ctx().global_style());
        add_contents(ui)
    })
}

/// The line between two groups of a toolbar (the drafts' `.tsep`): 1 point
/// wide in the panel's stroke colour, `SEPARATOR_SIDE` clear on each side,
/// running the toolbar's height less 1 at either end. The height is the
/// controls' (`interact_size`), which every toolbar button fills: egui's
/// own `Separator` runs the whole height left below the row instead.
pub fn toolbar_separator(ui: &mut egui::Ui) -> egui::Response {
    let size = egui::vec2(1.0 + 2.0 * SEPARATOR_SIDE, ui.spacing().interact_size.y);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().vline(
            rect.center().x,
            (rect.top() + 1.0)..=(rect.bottom() - 1.0),
            ui.visuals().widgets.noninteractive.bg_stroke,
        );
    }
    response
}

#[cfg(test)]
mod tests {
    use egui::accesskit::{self, Role};

    use super::*;
    use crate::widgets::menu_button;

    /// One frame of a toolbar holding the "Player" and "Team" menus, a
    /// separator between them, then a "Framed" button in the framed scope,
    /// and under the toolbar the "Outside" menu, with `events` as its
    /// input; the frame's AccessKit nodes, its shapes and the separator's
    /// rect.
    fn frame(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> (
        Vec<accesskit::Node>,
        Vec<egui::epaint::ClippedShape>,
        egui::Rect,
    ) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            events,
            ..Default::default()
        };
        let mut separator = egui::Rect::NOTHING;
        let mut output = ctx.run_ui(input, |ui| {
            toolbar(ui, |ui| {
                menu_button(ui, "Player", |ui| ui.button("Set all stats"));
                separator = toolbar_separator(ui).rect;
                menu_button(ui, "Team", |ui| ui.button("Set edited flag"));
                toolbar_framed(ui, |ui| ui.button("Framed"));
            });
            menu_button(ui, "Outside", |ui| ui.button("Set all stats"));
        });
        let update = output
            .platform_output
            .accesskit_update
            .take()
            .expect("AccessKit is enabled on the context");
        let shapes = std::mem::take(&mut output.shapes);
        // Nothing renders here, so the frame's texture deltas are dropped on
        // purpose.
        output.drop_without_applying_deltas();
        (
            update.nodes.into_iter().map(|(_, node)| node).collect(),
            shapes,
            separator,
        )
    }

    fn context() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        crate::theme::install(&ctx);
        ctx
    }

    fn button<'a>(nodes: &'a [accesskit::Node], label: &str) -> Option<&'a accesskit::Node> {
        nodes
            .iter()
            .find(|node| node.role() == Role::Button && node.label() == Some(label))
    }

    fn centre(node: &accesskit::Node) -> egui::Pos2 {
        let bounds = node.bounds().expect("a drawn button has bounds");
        egui::pos2(
            ((bounds.x0 + bounds.x1) / 2.0) as f32,
            ((bounds.y0 + bounds.y1) / 2.0) as f32,
        )
    }

    /// The primary button pressed (or released) at `pos`, the pointer moved
    /// there first.
    fn pointer_at(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::default(),
            },
        ]
    }

    /// Whether a filled or stroked rectangle the size of `node`'s bounds
    /// (within the frame's stroke) is painted over it: the button's frame.
    fn paints_frame(shapes: &[egui::epaint::ClippedShape], node: &accesskit::Node) -> bool {
        let bounds = node.bounds().expect("a drawn button has bounds");
        shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) => {
                let same = (f64::from(rect.rect.left()) - bounds.x0).abs() <= 1.0
                    && (f64::from(rect.rect.right()) - bounds.x1).abs() <= 1.0
                    && (f64::from(rect.rect.top()) - bounds.y0).abs() <= 1.0
                    && (f64::from(rect.rect.bottom()) - bounds.y1).abs() <= 1.0;
                let visible = rect.fill != egui::Color32::TRANSPARENT
                    || (rect.stroke.width > 0.0 && rect.stroke.color != egui::Color32::TRANSPARENT);
                same && visible
            }
            _ => false,
        })
    }

    #[test]
    fn a_menu_button_in_the_bar_paints_no_frame_at_rest_and_the_framed_scope_does() {
        let ctx = context();
        let (nodes, shapes, _) = frame(&ctx, Vec::new());
        let outside = button(&nodes, "Outside \u{23f7}").expect("the menu outside the toolbar");
        assert!(
            paints_frame(&shapes, outside),
            "premise: a menu button outside a toolbar paints its frame"
        );
        let player = button(&nodes, "Player \u{23f7}").expect("the Player menu button");
        assert!(
            !paints_frame(&shapes, player),
            "the bar's menu button paints no frame"
        );
        let framed = button(&nodes, "Framed").expect("the framed scope's button");
        assert!(
            paints_frame(&shapes, framed),
            "the framed scope's button paints its frame"
        );
    }

    #[test]
    fn with_one_menu_open_hovering_another_menus_button_opens_that_one() {
        let ctx = context();
        let (nodes, _, _) = frame(&ctx, Vec::new());
        let player = centre(button(&nodes, "Player \u{23f7}").expect("the Player menu button"));
        let team = centre(button(&nodes, "Team \u{23f7}").expect("the Team menu button"));
        frame(&ctx, pointer_at(player, true));
        frame(&ctx, pointer_at(player, false));
        let (nodes, _, _) = frame(&ctx, Vec::new());
        assert!(
            button(&nodes, "Set all stats").is_some(),
            "premise: the click opened the Player menu"
        );

        // The pointer only moves onto Team: no click.
        frame(&ctx, vec![egui::Event::PointerMoved(team)]);
        let (nodes, _, _) = frame(&ctx, Vec::new());
        assert!(
            button(&nodes, "Set edited flag").is_some(),
            "hovering Team opened the Team menu"
        );
        assert!(
            button(&nodes, "Set all stats").is_none(),
            "the Player menu closed"
        );
    }

    #[test]
    fn hovering_a_menus_button_with_none_open_opens_nothing() {
        let ctx = context();
        let (nodes, _, _) = frame(&ctx, Vec::new());
        let team = centre(button(&nodes, "Team \u{23f7}").expect("the Team menu button"));
        frame(&ctx, vec![egui::Event::PointerMoved(team)]);
        let (nodes, _, _) = frame(&ctx, Vec::new());
        assert!(button(&nodes, "Set edited flag").is_none());
    }

    #[test]
    fn a_separator_is_a_one_point_line_with_room_each_side_and_the_bars_height_less_one() {
        let ctx = context();
        let (nodes, shapes, separator) = frame(&ctx, Vec::new());
        let player = button(&nodes, "Player \u{23f7}")
            .and_then(accesskit::Node::bounds)
            .expect("the Player menu button");
        let team = button(&nodes, "Team \u{23f7}")
            .and_then(accesskit::Node::bounds)
            .expect("the Team menu button");
        let height = ctx.global_style().spacing.interact_size.y;
        assert_eq!(separator.width(), 1.0 + 2.0 * SEPARATOR_SIDE);
        assert_eq!(separator.height(), height);
        // The bar's buttons stand an item spacing either side of it.
        let spacing = f64::from(ctx.global_style().spacing.item_spacing.x);
        assert!((f64::from(separator.left()) - (player.x1 + spacing)).abs() < 0.01);
        assert!((team.x0 - (f64::from(separator.right()) + spacing)).abs() < 0.01);

        let stroke = ctx.global_style().visuals.widgets.noninteractive.bg_stroke;
        let line = shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::LineSegment { points, stroke: s }
                    if *s == stroke && points[0].x == points[1].x =>
                {
                    Some(*points)
                }
                _ => None,
            })
            .expect("the separator's vertical line in the panel's stroke");
        assert_eq!(line[0].x, separator.center().x);
        assert_eq!(line[0].y.min(line[1].y), separator.top() + 1.0);
        assert_eq!(line[0].y.max(line[1].y), separator.bottom() - 1.0);
    }
}
