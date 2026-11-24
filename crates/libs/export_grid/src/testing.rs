//! What the lib's tests share: the Compiler's columns, a headless window drawn a frame at a
//! time on a clock the test moves, and finders over what a frame drew.

use std::time::Duration;

use egui::accesskit;
use egui::epaint::Shape;

use crate::cells::Width;
use crate::rows::{ColumnGroup, Columns};

/// The Compiler's columns: 23 players, the portraits, 10 kits.
pub(crate) const GROUPS: [ColumnGroup; 3] = [
    ColumnGroup {
        title: "Players",
        columns: 23,
        width: Width::Square,
    },
    ColumnGroup {
        title: "Portraits",
        columns: 1,
        width: Width::Wide,
    },
    ColumnGroup {
        title: "Kits",
        columns: 10,
        width: Width::Square,
    },
];
pub(crate) const COLUMNS: Columns = Columns {
    id_title: Some("ID"),
    line_names: false,
    groups: &GROUPS,
    last_title: "Actions",
};

/// A frame's length at 60 frames a second.
const FRAME: f64 = 1.0 / 60.0;

/// A headless window: one egui context with the suite's fonts and style, AccessKit on and
/// animations off.
pub(crate) struct Headless {
    pub(crate) ctx: egui::Context,
    /// The screen each frame runs on: 1280 by 600 unless a test narrows it.
    pub(crate) screen: egui::Vec2,
    /// The time the next frame runs at, in seconds. Each frame moves it on by a 60th of a
    /// second, as a running window would (a scroll moves only while time passes); a test
    /// moves it further for a delay to pass.
    pub(crate) time: f64,
}

/// What one frame drew: its AccessKit nodes and its shapes, and what it asked of the
/// platform: its commands (the clipboard's text) and when to paint again.
pub(crate) struct Drawn {
    pub(crate) nodes: Vec<accesskit::Node>,
    pub(crate) shapes: Vec<Shape>,
    pub(crate) commands: Vec<egui::OutputCommand>,
    /// `Duration::MAX` when nothing asked for a repaint.
    pub(crate) repaint_delay: Duration,
}

impl Headless {
    pub(crate) fn new() -> Headless {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        studio_core::theme::install(&ctx);
        ctx.global_style_mut(|style| style.animation_time = 0.0);
        Headless {
            ctx,
            screen: egui::vec2(1280.0, 600.0),
            time: 0.0,
        }
    }

    /// One frame of `contents` on the window's `screen` at its `time`, with `events` as its
    /// input.
    pub(crate) fn frame<R>(
        &mut self,
        events: Vec<egui::Event>,
        contents: impl FnMut(&mut egui::Ui) -> R,
    ) -> Drawn {
        self.frame_with(events, contents).1
    }

    /// `frame`, with what `contents` returned.
    pub(crate) fn frame_with<R>(
        &mut self,
        events: Vec<egui::Event>,
        mut contents: impl FnMut(&mut egui::Ui) -> R,
    ) -> (R, Drawn) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.screen)),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let mut result = None;
        let mut output = self.ctx.run_ui(input, |ui| result = Some(contents(ui)));
        self.time += FRAME;
        let repaint_delay = output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .expect("every frame produces the root viewport")
            .repaint_delay;
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
        let commands = std::mem::take(&mut output.platform_output.commands);
        output.drop_without_applying_deltas();
        let drawn = Drawn {
            nodes: update.nodes.into_iter().map(|(_, node)| node).collect(),
            shapes,
            commands,
            repaint_delay,
        };
        (result.expect("run_ui calls the closure"), drawn)
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
    /// The nodes that read as `text`: a painted cell's or title's, or a widget's. A label's
    /// text run, a node of its own with the label's text, is not counted again.
    pub(crate) fn all(&self, text: &str) -> Vec<egui::Rect> {
        self.nodes
            .iter()
            .filter(|node| node.role() != accesskit::Role::TextRun)
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

/// `text`'s painted run: what the painter draws is no AccessKit node, so it is found
/// among the frame's shapes; panics when no run of `text` was painted.
pub(crate) fn text_shape<'a>(drawn: &'a Drawn, text: &str) -> &'a egui::epaint::TextShape {
    drawn
        .shapes
        .iter()
        .find_map(|shape| match shape {
            Shape::Text(text_shape) if text_shape.galley.text() == text => Some(text_shape),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no text run of {text:?}"))
}

/// The ends of every line segment painted in `stroke`: lines, like painted text, are no
/// AccessKit node.
pub(crate) fn lines(drawn: &Drawn, stroke: egui::Stroke) -> Vec<[egui::Pos2; 2]> {
    drawn
        .shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::LineSegment { points, stroke: s } if *s == stroke => Some(*points),
            _ => None,
        })
        .collect()
}

/// A node's bounds, in points.
pub(crate) fn bounds(node: &accesskit::Node) -> egui::Rect {
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

/// The pointer moved to `pos`, then the wheel turned by `delta`.
pub(crate) fn wheel(pos: egui::Pos2, delta: egui::Vec2) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::default(),
        },
    ]
}
