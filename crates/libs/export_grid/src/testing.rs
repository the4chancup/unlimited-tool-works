//! What the lib's tests share: a headless window drawn a frame at a time, and finders over
//! what a frame drew.

use egui::accesskit;
use egui::epaint::Shape;

/// A headless window: one egui context with the suite's fonts and style, AccessKit on and
/// animations off.
pub(crate) struct Headless {
    pub(crate) ctx: egui::Context,
}

/// What one frame drew: its AccessKit nodes and its shapes.
pub(crate) struct Drawn {
    pub(crate) nodes: Vec<accesskit::Node>,
    pub(crate) shapes: Vec<Shape>,
}

impl Headless {
    pub(crate) fn new() -> Headless {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        studio_core::theme::install(&ctx);
        ctx.global_style_mut(|style| style.animation_time = 0.0);
        Headless { ctx }
    }

    /// One frame of `contents` on a 1280 by 600 screen with `events` as its input.
    pub(crate) fn frame(
        &mut self,
        events: Vec<egui::Event>,
        mut contents: impl FnMut(&mut egui::Ui),
    ) -> Drawn {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 600.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = self.ctx.run_ui(input, |ui| contents(ui));
        let update = output
            .platform_output
            .accesskit_update
            .take()
            .expect("AccessKit is enabled on the context");
        let shapes = std::mem::take(&mut output.shapes)
            .into_iter()
            .map(|clipped| clipped.shape)
            .collect();
        // Nothing renders here, so the frame's texture deltas are dropped on purpose.
        output.drop_without_applying_deltas();
        Drawn {
            nodes: update.nodes.into_iter().map(|(_, node)| node).collect(),
            shapes,
        }
    }
}

/// The dark theme's visuals as the window installs them (its own amber and red, not egui's).
pub(crate) fn dark() -> egui::Visuals {
    Headless::new()
        .ctx
        .style_of(egui::Theme::Dark)
        .visuals
        .clone()
}

impl Drawn {
    /// The nodes that read as `text`: a painted cell's or title's, or a label's.
    pub(crate) fn all(&self, text: &str) -> Vec<egui::Rect> {
        self.nodes
            .iter()
            .filter(|node| node.value() == Some(text) || node.label() == Some(text))
            .map(bounds)
            .collect()
    }

    /// The one node that reads as `text`.
    pub(crate) fn one(&self, text: &str) -> egui::Rect {
        let found = self.all(text);
        assert_eq!(found.len(), 1, "one node reads {text:?}");
        found[0]
    }

    /// The fill of the filled rectangle at `rect`, the last one painted there.
    pub(crate) fn fill_at(&self, rect: egui::Rect) -> Option<egui::Color32> {
        self.shapes.iter().rev().find_map(|shape| match shape {
            Shape::Rect(painted) if painted.rect == rect && painted.fill.a() > 0 => {
                Some(painted.fill)
            }
            _ => None,
        })
    }
}

/// A node's bounds, in points.
fn bounds(node: &accesskit::Node) -> egui::Rect {
    let bounds = node.bounds().expect("a node has bounds");
    egui::Rect::from_min_max(
        egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
        egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
    )
}

/// The pointer moved to `pos` and the primary button went down there.
pub(crate) fn press(pos: egui::Pos2) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::default(),
        },
    ]
}

/// The primary button came up at `pos`: egui reports the click on this frame.
pub(crate) fn release(pos: egui::Pos2) -> Vec<egui::Event> {
    vec![egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    }]
}
