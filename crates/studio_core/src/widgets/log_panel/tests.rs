use std::borrow::Cow;

use egui::accesskit::{self, Role, Toggled};

use super::*;
use crate::events::{Disposition, ExportId, Message, Scope};
use vtree::ScopePath;

/// A headless window: one egui context with AccessKit on, run a frame at a time
/// over an 800 by 600 screen (the shell harness's `frame`, for a bare `Ui`).
struct Headless {
    ctx: egui::Context,
    /// The last frame's painted shapes.
    shapes: Vec<egui::epaint::ClippedShape>,
}

impl Headless {
    fn new() -> Headless {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        // The window draws with these fonts and style; the test measures
        // what a member sees, whichever crates the build unifies.
        crate::theme::install(&ctx);
        Headless {
            ctx,
            shapes: Vec::new(),
        }
    }

    /// Runs one frame with `events` as its input; returns the frame's AccessKit
    /// nodes and keeps its shapes in `shapes`.
    fn frame(
        &mut self,
        events: Vec<egui::Event>,
        contents: impl FnMut(&mut egui::Ui),
    ) -> Vec<accesskit::Node> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = self.ctx.run_ui(input, contents);
        let update = output
            .platform_output
            .accesskit_update
            .take()
            .expect("AccessKit is enabled on the context");
        self.shapes = std::mem::take(&mut output.shapes);
        // Nothing renders here, so the frame's texture deltas are dropped on
        // purpose.
        output.drop_without_applying_deltas();
        update.nodes.into_iter().map(|(_, node)| node).collect()
    }

    /// One frame of `panel` over `lines`.
    fn show(
        &mut self,
        panel: &mut LogPanel,
        lines: &[LogLine],
        events: Vec<egui::Event>,
    ) -> Vec<accesskit::Node> {
        self.frame(events, |ui| {
            panel.show(ui, lines);
        })
    }

    /// `show` with no input.
    fn idle(&mut self, panel: &mut LogPanel, lines: &[LogLine]) -> Vec<accesskit::Node> {
        self.show(panel, lines, Vec::new())
    }
}

/// The widget labeled `label` (a button's text sits in `label`).
fn entry<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| node.label() == Some(label))
        .unwrap_or_else(|| panic!("no widget labeled {label:?}"))
}

/// Whether some node carries exactly `text` (a label node's text is its `value`).
fn shows(nodes: &[accesskit::Node], text: &str) -> bool {
    nodes.iter().any(|node| node.value() == Some(text))
}

/// The bounds of the node carrying exactly `text`.
fn bounds(nodes: &[accesskit::Node], text: &str) -> accesskit::Rect {
    nodes
        .iter()
        .find(|node| node.value() == Some(text))
        .unwrap_or_else(|| panic!("no label with text {text:?}"))
        .bounds()
        .expect("a drawn node has bounds")
}

/// The centre of `node`'s bounds, where a click lands.
fn centre(node: &accesskit::Node) -> egui::Pos2 {
    let bounds = node.bounds().expect("a widget has bounds");
    egui::pos2(
        ((bounds.x0 + bounds.x1) / 2.0) as f32,
        ((bounds.y0 + bounds.y1) / 2.0) as f32,
    )
}

/// The primary button pressed (or released) at `pos`, the pointer moved there
/// first.
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

/// A wheel of `delta` points at `pos` (positive scrolls up, negative down), the
/// pointer moved there first: egui's wheel lands on the scroll area under it.
fn wheel_at(pos: egui::Pos2, delta: f32) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, delta),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::default(),
            source: egui::MouseWheelSource::Wheel,
        },
    ]
}

/// Whether a `↓ N new` jump button is drawn, at any count.
fn has_jump(nodes: &[accesskit::Node]) -> bool {
    nodes.iter().any(|node| {
        node.label()
            .is_some_and(|label| label.starts_with("↓ ") && label.ends_with(" new"))
    })
}

/// A click on `node`: press frame, release frame, returning the release's nodes —
/// the frame `clicked()` lands in.
fn click(
    window: &mut Headless,
    panel: &mut LogPanel,
    lines: &[LogLine],
    node: &accesskit::Node,
) -> Vec<accesskit::Node> {
    let pos = centre(node);
    window.show(panel, lines, pointer_at(pos, true));
    window.show(panel, lines, pointer_at(pos, false))
}

/// The panel's resolved id: since egui 0.37 a `Panel` persists its `PanelState` under
/// `parent_scope.with_salt(IdSalt::new(id_salt))` and gives its child ui the same for a
/// scope — the salt chain resolves the stored `panel_ids()` against the root ui's top id.
fn panel_id(salt: egui::Id) -> egui::Id {
    egui::Id::unique((egui::ViewportId::ROOT, "__top_ui")).with(salt)
}

/// The log body's scroll offset: its `scroll_area::State` under the id the
/// `id_salt` chain builds — the root ui's top id, the open panel's resolved id as the
/// child scope's salt, the panel frame's `child` content salt, the body frame's, then
/// the panel's `salt`.
fn scroll_offset(ctx: &egui::Context, panel: &LogPanel) -> f32 {
    let (open, _) = panel.dock.panel_ids();
    let id = egui::Id::unique((egui::ViewportId::ROOT, "__top_ui"))
        .with(egui::IdSalt::new(panel_id(open)))
        .with(egui::IdSalt::new("child"))
        .with(egui::IdSalt::new("child"))
        .with(egui::IdSalt::new(panel.salt));
    egui::scroll_area::State::load(ctx, id)
        .map(|state| state.clamped_offset().y)
        .expect("the log body's scroll area")
}

/// A dock panel's drawn height (its `PanelState` under the resolved panel id).
fn panel_height(ctx: &egui::Context, salt: egui::Id) -> f32 {
    egui::PanelState::load(ctx, panel_id(salt))
        .expect("the dock's panel state")
        .outer_rect
        .height()
}

fn plain_lines(count: usize) -> Vec<LogLine> {
    (1..=count)
        .map(|n| LogLine::plain(Severity::Info, format!("log line {n}")))
        .collect()
}

// ST-LOG-01
#[test]
fn only_the_capped_tail_of_the_log_is_drawn() {
    let lines = plain_lines(350);
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    let nodes = window.idle(&mut panel, &lines);
    assert!(
        !shows(&nodes, "log line 1"),
        "the first line is over the cap"
    );
    assert!(
        !shows(&nodes, "log line 50"),
        "the 301st from the end is too"
    );
    assert!(shows(&nodes, "log line 350"), "the newest is drawn");
    assert!(shows(&nodes, "last 300 of 350"), "the count line");

    // Stuck to the bottom: the two last drawn lines stack a row apart at the
    // body's bottom edge — a line scrolled under the viewport would be
    // window-clipped into a degenerate rect at the screen's edge instead.
    let last = bounds(&nodes, "log line 350");
    let before = bounds(&nodes, "log line 349");
    assert!(
        last.y0 - before.y0 > 5.0,
        "the newest lines stack one row apart: {} vs {}",
        last.y0,
        before.y0
    );
    let (open, _) = panel.dock.panel_ids();
    let bottom = egui::PanelState::load(&window.ctx, panel_id(open))
        .expect("the open panel's state")
        .outer_rect
        .max
        .y;
    assert!(
        last.y1 > f64::from(bottom) - 30.0,
        "the newest line is at the bottom: {} vs {}",
        last.y1,
        bottom
    );
}

// ST-LOG-02
#[test]
fn scrolling_up_pauses_the_follow_and_the_jump_button_returns_to_the_bottom() {
    // 200 lines: over the body's ~11-row view but under the cap, so the drawn
    // window cannot slide underneath the assertion.
    let mut lines = plain_lines(200);
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    window.idle(&mut panel, &lines);

    // Wheel up over the body; egui drains the delta over a few frames.
    let body = egui::pos2(400.0, 500.0);
    window.show(
        &mut panel,
        &lines,
        vec![
            egui::Event::PointerMoved(body),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 200.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::default(),
                source: egui::MouseWheelSource::Wheel,
            },
        ],
    );
    // egui drains the wheel delta over a few frames; let it settle before
    // recording where the view stopped.
    let mut nodes = window.idle(&mut panel, &lines);
    for _ in 0..15 {
        nodes = window.idle(&mut panel, &lines);
    }
    let offset = scroll_offset(&window.ctx, &panel);
    // The topmost visible line (real bounds, not a clipped-offscreen one): the
    // marker the "view does not move" assertion reads.
    let marker = (1..=200)
        .map(|n| format!("log line {n}"))
        .find(|text| {
            nodes
                .iter()
                .find(|node| node.value() == Some(text.as_str()))
                .and_then(|node| node.bounds())
                .is_some_and(|rect| rect.y0 > 405.0 && rect.y0 < 560.0)
        })
        .expect("a line in view")
        .clone();
    let marker_y = bounds(&nodes, &marker).y0;

    // Three lines arrive: the view does not move and `↓ 3 new` counts them.
    lines.extend((201..=203).map(|n| LogLine::plain(Severity::Info, format!("log line {n}"))));
    let nodes = window.idle(&mut panel, &lines);
    assert!(
        (scroll_offset(&window.ctx, &panel) - offset).abs() < 1.0,
        "arriving lines do not move the view: {} vs {offset}",
        scroll_offset(&window.ctx, &panel)
    );
    assert!(
        (bounds(&nodes, &marker).y0 - marker_y).abs() < 1.0,
        "the first visible line has not moved: {} vs {marker_y}",
        bounds(&nodes, &marker).y0
    );
    let jump = entry(&nodes, "↓ 3 new");
    // The button overlays the body's bottom-right: the body's bottom edge is
    // the button's own bottom edge plus its margin.
    let body_bottom = jump.bounds().expect("the jump button's bounds").y1 + JUMP_MARGIN as f64;

    click(&mut window, &mut panel, &lines, jump);
    // The click sets the snap: the next frame issues `scroll_to_cursor`,
    // which updates the offset after that frame's draw, so the snapped
    // layout shows on the frame after.
    window.idle(&mut panel, &lines);
    let nodes = window.idle(&mut panel, &lines);
    let last = bounds(&nodes, "log line 203");
    // The scroll reached its maximum: the newest line's bounds are wholly
    // inside the body — an offscreen line would still pass a floor check.
    assert!(
        last.y1 >= body_bottom - 2.0 && last.y1 <= body_bottom + 1.0,
        "the last line's bottom edge is at the body's: {} vs {}",
        last.y1,
        body_bottom
    );
    assert!(
        !nodes.iter().any(|node| node.label() == Some("↓ 3 new")),
        "the button is gone"
    );

    // One more arrives: the view follows it to the bottom again — the
    // stick lands after that frame's draw, so it shows one frame later.
    lines.push(LogLine::plain(Severity::Info, "log line 204"));
    window.idle(&mut panel, &lines);
    let nodes = window.idle(&mut panel, &lines);
    let last = bounds(&nodes, "log line 204");
    assert!(
        last.y1 >= body_bottom - 2.0 && last.y1 <= body_bottom + 1.0,
        "the view follows the newest line to the body's bottom: {} vs {}",
        last.y1,
        body_bottom
    );
}

// ST-LOG-02
#[test]
fn a_paused_log_past_the_cap_holds_the_lines_it_had() {
    let mut lines = plain_lines(310);
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    let nodes = window.idle(&mut panel, &lines);
    // Over the cap, the drawn window is the last 300: line 11 is its first.
    assert!(shows(&nodes, "log line 11"), "the capped tail is drawn");
    assert!(!shows(&nodes, "log line 10"), "lines over the cap are not");

    // Scroll up to pause the follow, letting the wheel delta drain.
    window.show(
        &mut panel,
        &lines,
        wheel_at(egui::pos2(400.0, 500.0), 400.0),
    );
    for _ in 0..15 {
        window.idle(&mut panel, &lines);
    }

    // Five lines arrive: the drawn window cannot slide — its topmost line
    // is the same, and the arrivals are counted by the button, not drawn.
    lines.extend((311..=315).map(|n| LogLine::plain(Severity::Info, format!("log line {n}"))));
    let nodes = window.idle(&mut panel, &lines);
    assert!(
        shows(&nodes, "log line 11"),
        "paused, the window keeps the first line it had"
    );
    assert!(
        !shows(&nodes, "log line 315"),
        "arrivals past the pause are counted, not drawn"
    );
    let jump = entry(&nodes, "↓ 5 new");
    let body_bottom = jump.bounds().expect("the jump button's bounds").y1 + JUMP_MARGIN as f64;

    // The jump lands at the tail: the newest line is drawn, and once the
    // snap's offset lands (a frame after the issuing one) its bottom edge
    // is at the body's — a presence check alone would pass on a lost scroll.
    click(&mut window, &mut panel, &lines, jump);
    window.idle(&mut panel, &lines);
    let nodes = window.idle(&mut panel, &lines);
    let last = bounds(&nodes, "log line 315");
    assert!(
        last.y1 >= body_bottom - 2.0 && last.y1 <= body_bottom + 1.0,
        "the last line's bottom edge is at the body's: {} vs {}",
        last.y1,
        body_bottom
    );
}

#[test]
fn a_wheel_stopping_a_point_short_of_the_end_leaves_the_bottom_too() {
    let mut lines = plain_lines(200);
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    window.idle(&mut panel, &lines);
    // Stuck at the end, the offset is the clamped maximum.
    let max = scroll_offset(&window.ctx, &panel);
    // A wheel up by half a point: egui clears its end flag on any wheel scroll,
    // and the offset lands inside a point of the end but not on it.
    window.show(&mut panel, &lines, wheel_at(egui::pos2(400.0, 500.0), 0.5));
    for _ in 0..5 {
        window.idle(&mut panel, &lines);
    }
    assert!(
        scroll_offset(&window.ctx, &panel) < max,
        "the view left the bottom: {} vs {max}",
        scroll_offset(&window.ctx, &panel)
    );
    // Two lines arrive: the jump button counts them — a "still at the bottom"
    // reading would hide it.
    lines.extend((201..=202).map(|n| LogLine::plain(Severity::Info, format!("log line {n}"))));
    let nodes = window.idle(&mut panel, &lines);
    entry(&nodes, "↓ 2 new");
}

/// One frame with two panels drawn: `lower` claims the lowest slot, `upper` the
/// one above it (`Panel::bottom` claims the bottom of what is left).
fn draw_two(
    window: &mut Headless,
    lower: &mut LogPanel,
    upper: &mut LogPanel,
    lines: &[LogLine],
    events: Vec<egui::Event>,
) -> Vec<accesskit::Node> {
    window.frame(events, |ui| {
        lower.show(ui, lines);
        upper.show(ui, lines);
    })
}

#[test]
fn two_panels_have_separate_scroll_state() {
    let lines = plain_lines(200);
    let mut lower = LogPanel::new("lower-log");
    let mut upper = LogPanel::new("upper-log");
    let mut window = Headless::new();
    draw_two(&mut window, &mut lower, &mut upper, &lines, Vec::new());
    let bottom = scroll_offset(&window.ctx, &lower);
    // Wheel up over the lower panel's body: it scrolls, the one above does not.
    draw_two(
        &mut window,
        &mut lower,
        &mut upper,
        &lines,
        wheel_at(egui::pos2(400.0, 550.0), 400.0),
    );
    for _ in 0..15 {
        draw_two(&mut window, &mut lower, &mut upper, &lines, Vec::new());
    }
    assert!(
        scroll_offset(&window.ctx, &lower) < bottom,
        "the lower panel scrolled up: {} vs {bottom}",
        scroll_offset(&window.ctx, &lower)
    );
    assert_eq!(
        scroll_offset(&window.ctx, &upper),
        bottom,
        "the upper panel did not move"
    );
}

#[test]
fn a_new_runs_shorter_log_resets_the_jump_count() {
    // 300 lines scrolled well up, three lines arriving.
    let mut lines = plain_lines(300);
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    window.idle(&mut panel, &lines);
    window.show(
        &mut panel,
        &lines,
        wheel_at(egui::pos2(400.0, 500.0), 3000.0),
    );
    for _ in 0..20 {
        window.idle(&mut panel, &lines);
    }
    lines.extend((301..=303).map(|n| LogLine::plain(Severity::Info, format!("log line {n}"))));
    let nodes = window.idle(&mut panel, &lines);
    entry(&nodes, "↓ 3 new");

    // A rerun replaces the lines with a shorter list, still overflowing the
    // body far enough to leave the view off the bottom.
    lines = (1..=250)
        .map(|n| LogLine::plain(Severity::Info, format!("rerun line {n}")))
        .collect();
    window.idle(&mut panel, &lines);
    lines.extend((251..=252).map(|n| LogLine::plain(Severity::Info, format!("rerun line {n}"))));
    let nodes = window.idle(&mut panel, &lines);
    // The counter starts over from the rerun's lines, not the old count plus 2.
    entry(&nodes, "↓ 2 new");
    let nodes = window.idle(&mut panel, &lines);
    entry(&nodes, "↓ 2 new");
}

#[test]
fn the_filter_chips_show_their_selected_state() {
    let lines = vec![LogLine::plain(Severity::Info, "an info line")];
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    // A selectable_label reports its selected state as a toggle (as the help
    // tree's topics do).
    let toggled = |nodes: &[accesskit::Node], label: &str| {
        nodes
            .iter()
            .find(|node| node.role() == Role::Button && node.label() == Some(label))
            .map(|node| node.toggled())
    };
    let nodes = window.idle(&mut panel, &lines);
    assert_eq!(toggled(&nodes, "All"), Some(Some(Toggled::True)));
    assert_eq!(toggled(&nodes, "Warnings"), Some(Some(Toggled::False)));
    assert_eq!(toggled(&nodes, "Errors"), Some(Some(Toggled::False)));

    click(&mut window, &mut panel, &lines, entry(&nodes, "Warnings"));
    // Each chip's `selected` is read when it draws, so the new state shows on
    // the frame after the click.
    let nodes = window.idle(&mut panel, &lines);
    assert_eq!(toggled(&nodes, "All"), Some(Some(Toggled::False)));
    assert_eq!(toggled(&nodes, "Warnings"), Some(Some(Toggled::True)));
    assert_eq!(toggled(&nodes, "Errors"), Some(Some(Toggled::False)));
}

#[test]
fn at_the_bottom_arriving_lines_show_no_jump_button() {
    let mut lines = plain_lines(200);
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    let nodes = window.idle(&mut panel, &lines);
    assert!(!has_jump(&nodes), "stuck at the bottom, no jump button");

    lines.extend((201..=205).map(|n| LogLine::plain(Severity::Info, format!("log line {n}"))));
    let nodes = window.idle(&mut panel, &lines);
    assert!(
        !has_jump(&nodes),
        "the view follows the new lines, still no button"
    );
}

#[test]
fn a_log_shorter_than_the_view_shows_no_jump_button() {
    // A run's first lines fit the view: egui's offset stays 0 above a negative
    // end, and that is the bottom too.
    let mut lines = plain_lines(3);
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    window.idle(&mut panel, &lines);

    lines.extend((4..=5).map(|n| LogLine::plain(Severity::Info, format!("log line {n}"))));
    let nodes = window.idle(&mut panel, &lines);
    assert!(shows(&nodes, "log line 5"));
    assert!(!has_jump(&nodes), "five lines fit: nothing to jump to");
}

#[test]
fn scrolled_up_without_arrivals_shows_no_jump_button() {
    let lines = plain_lines(200);
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    window.idle(&mut panel, &lines);
    window.show(
        &mut panel,
        &lines,
        wheel_at(egui::pos2(400.0, 500.0), 200.0),
    );
    for _ in 0..15 {
        window.idle(&mut panel, &lines);
    }
    let nodes = window.idle(&mut panel, &lines);
    assert!(
        !has_jump(&nodes),
        "nothing arrived: no button, not even `↓ 0 new`"
    );
}

#[test]
fn scrolling_back_to_the_bottom_by_wheel_resumes_the_follow() {
    let mut lines = plain_lines(200);
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    window.idle(&mut panel, &lines);
    // Up by the wheel, then back down to the bottom the same way.
    window.show(
        &mut panel,
        &lines,
        wheel_at(egui::pos2(400.0, 500.0), 400.0),
    );
    for _ in 0..15 {
        window.idle(&mut panel, &lines);
    }
    let risen = scroll_offset(&window.ctx, &panel);
    window.show(
        &mut panel,
        &lines,
        wheel_at(egui::pos2(400.0, 500.0), -800.0),
    );
    for _ in 0..15 {
        window.idle(&mut panel, &lines);
    }
    let back = scroll_offset(&window.ctx, &panel);
    assert!(back > risen, "the wheel reached the bottom again");
    let nodes = window.idle(&mut panel, &lines);
    assert!(!has_jump(&nodes), "back at the bottom, no button");

    lines.push(LogLine::plain(Severity::Info, "log line 201"));
    let nodes = window.idle(&mut panel, &lines);
    let (open, _) = panel.dock.panel_ids();
    let bottom = egui::PanelState::load(&window.ctx, panel_id(open))
        .expect("the open panel's state")
        .outer_rect
        .max
        .y;
    assert!(
        bounds(&nodes, "log line 201").y1 > f64::from(bottom) - 30.0,
        "the view follows the newest line: {} vs {}",
        bounds(&nodes, "log line 201").y1,
        bottom
    );
    assert!(!has_jump(&nodes));
}

// ST-LOG-03
#[test]
fn the_filter_chips_hide_lines_without_dropping_them() {
    let lines = vec![
        LogLine::plain(Severity::Info, "an info line"),
        LogLine::plain(Severity::Warning, "a warning line"),
        LogLine::plain(Severity::Error, "an error line"),
        LogLine::plain(Severity::Fatal, "a fatal line"),
    ];
    let all = [
        "an info line",
        "a warning line",
        "an error line",
        "a fatal line",
    ];
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    let nodes = window.idle(&mut panel, &lines);
    for text in all {
        assert!(shows(&nodes, text), "All draws {text}");
    }

    let nodes = click(&mut window, &mut panel, &lines, entry(&nodes, "Warnings"));
    assert!(
        !shows(&nodes, "an info line"),
        "Warnings hides the info line"
    );
    for text in &all[1..] {
        assert!(shows(&nodes, text), "Warnings keeps {text}");
    }
    assert!(
        shows(&nodes, "3 of 4 lines"),
        "the count tells the hidden ones"
    );

    let nodes = click(&mut window, &mut panel, &lines, entry(&nodes, "Errors"));
    for text in &all[..2] {
        assert!(!shows(&nodes, text), "Errors hides {text}");
    }
    for text in &all[2..] {
        assert!(shows(&nodes, text), "Errors keeps {text}");
    }
    assert!(shows(&nodes, "2 of 4 lines"));

    let nodes = click(&mut window, &mut panel, &lines, entry(&nodes, "All"));
    for text in all {
        assert!(shows(&nodes, text), "All draws {text} again");
    }
    assert!(shows(&nodes, "4 lines"));
}

/// One frame drawing `panel` with a central panel over the space left
/// above it; returns the frame's nodes and the central panel's height —
/// the "view above" whose growth the collapsed dock hands back.
fn with_central(
    window: &mut Headless,
    panel: &mut LogPanel,
    lines: &[LogLine],
) -> (Vec<accesskit::Node>, f32) {
    let mut height = 0.0;
    let nodes = window.frame(Vec::new(), |ui| {
        panel.show(ui, lines);
        height = egui::CentralPanel::default()
            .show(ui, |ui| {
                ui.label("tool view");
            })
            .response
            .rect
            .height();
    });
    (nodes, height)
}

/// The body's wells: the rects painted with `well_frame`'s fill and stroke.
fn wells(shapes: &[egui::epaint::ClippedShape], visuals: &egui::Visuals) -> Vec<egui::Rect> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.fill == visuals.extreme_bg_color
                    && shape.stroke == visuals.window_stroke() =>
            {
                Some(shape.rect)
            }
            _ => None,
        })
        .collect()
}

/// The log's well is 180 points high when the dock first opens, the drafts'
/// `logHeight` (the `.log-body`'s height with its border).
#[test]
fn the_log_opens_with_a_180_point_well() {
    let lines = plain_lines(5);
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    window.idle(&mut panel, &lines);
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let wells = wells(&window.shapes, &visuals);
    let [well] = wells[..] else {
        panic!("one well in the log: {wells:?}");
    };
    assert!(
        (well.height() - 180.0).abs() <= 1.0,
        "the well's height on the first frame: {well:?}"
    );
}

// ST-LOG-04
#[test]
fn the_chevron_collapses_to_the_header_and_the_open_height_survives() {
    let lines = plain_lines(20);
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    let (open, collapsed) = panel.dock.panel_ids();
    let (nodes, central) = with_central(&mut window, &mut panel, &lines);
    let open_height = panel_height(&window.ctx, open);
    // The 180-point body under the frame's margins, the top line, the header
    // row and the spacing below it.
    assert!(
        (open_height - 206.0).abs() < 1.0,
        "the dock opens at its first height: {open_height}"
    );

    // The chevron collapses to the header row alone; the body is gone on the
    // frame after the click (the panel branch is chosen before the header draws)
    // and the view above takes the freed space.
    click(&mut window, &mut panel, &lines, entry(&nodes, "▾"));
    let (nodes, collapsed_central) = with_central(&mut window, &mut panel, &lines);
    assert!(!shows(&nodes, "log line 20"), "the body is not drawn");
    let header = panel_height(&window.ctx, collapsed);
    assert!(
        header < 60.0 && header > 10.0,
        "collapsed, the panel is its header row: {header}"
    );
    assert!(
        collapsed_central > central + 100.0,
        "the view above takes the freed space: {collapsed_central} vs {central}"
    );

    // Expanding restores the stored open height and the earlier view.
    click(&mut window, &mut panel, &lines, entry(&nodes, "▸"));
    let (nodes, restored_central) = with_central(&mut window, &mut panel, &lines);
    assert!(shows(&nodes, "log line 20"), "the body is drawn again");
    let back = panel_height(&window.ctx, open);
    assert!(
        (back - open_height).abs() < 1.0,
        "the open height survived the collapse: {back} vs {open_height}"
    );
    assert!(
        (restored_central - central).abs() < 1.0,
        "the view above returns to its earlier height: {restored_central} vs {central}"
    );

    // Dragging the divider up 80 points grows the panel by about 80, kept across
    // frames. The handle is the open panel's top edge.
    let top = egui::PanelState::load(&window.ctx, panel_id(open))
        .expect("the open panel's state")
        .outer_rect
        .min
        .y;
    let start = egui::pos2(400.0, top);
    let end = start - egui::vec2(0.0, 80.0);
    window.show(&mut panel, &lines, pointer_at(start, true));
    window.show(&mut panel, &lines, vec![egui::Event::PointerMoved(end)]);
    window.show(&mut panel, &lines, pointer_at(end, false));
    window.idle(&mut panel, &lines);
    let grown = panel_height(&window.ctx, open);
    assert!(
        (grown - (open_height + 80.0)).abs() < 2.0,
        "the divider drag resized the panel: {grown} vs {}",
        open_height + 80.0
    );
    window.idle(&mut panel, &lines);
    window.idle(&mut panel, &lines);
    let kept = panel_height(&window.ctx, open);
    assert_eq!(kept, grown, "the dragged height holds across frames");

    // Collapsing and expanding again comes back at the dragged height, not
    // the default: the open panel's stored height outlives the collapse.
    let (nodes, _) = with_central(&mut window, &mut panel, &lines);
    click(&mut window, &mut panel, &lines, entry(&nodes, "▾"));
    let (nodes, second_collapsed) = with_central(&mut window, &mut panel, &lines);
    assert!(
        (second_collapsed - collapsed_central).abs() < 2.0,
        "the header collapse frees the same space: {second_collapsed} vs {collapsed_central}"
    );
    click(&mut window, &mut panel, &lines, entry(&nodes, "▸"));
    let (_, second_central) = with_central(&mut window, &mut panel, &lines);
    let again = panel_height(&window.ctx, open);
    assert!(
        (again - grown).abs() < 1.0,
        "the dragged height survives the collapse: {again} vs {grown}"
    );
    assert!(
        (second_central - (central - (grown - open_height))).abs() < 2.0,
        "the view above sits under the dragged dock: {second_central}"
    );
}

#[test]
fn from_event_maps_a_message_to_its_severity_and_code() {
    let mut formatter = EventLines::new();
    let code = MessageCode {
        tool_id: "team-compiler",
        code: Cow::Borrowed("player_unlisted"),
    };
    let event = PipelineEvent::Message(Message {
        code: code.clone(),
        severity: Severity::Warning,
        disposition: Disposition::Keep,
        scope: Scope::Run,
        context: Vec::new(),
    });
    assert_eq!(
        LogLine::from_event(&mut formatter, &event),
        Some(LogLine {
            severity: Severity::Warning,
            code: Some(code),
            text: "- Warning player_unlisted".to_owned(),
        })
    );

    // An `Output` event is a codeless Info line of its text.
    let output = PipelineEvent::Output("the report".to_owned());
    assert_eq!(
        LogLine::from_event(&mut formatter, &output),
        Some(LogLine {
            severity: Severity::Info,
            code: None,
            text: "the report".to_owned(),
        })
    );

    // An event the console does not show gives no line.
    assert_eq!(
        LogLine::from_event(
            &mut formatter,
            &PipelineEvent::Progress {
                processed: 1,
                total: 2,
            }
        ),
        None
    );
}

#[test]
fn the_count_line_changes_at_the_cap() {
    let lines = plain_lines(LOG_CAP);
    assert_eq!(count_text(Filter::All, &lines), "300 lines");
    let lines = plain_lines(LOG_CAP + 1);
    assert_eq!(count_text(Filter::All, &lines), "last 300 of 301");
}

#[test]
fn the_line_color_matches_the_severity() {
    let visuals = egui::Visuals::dark();
    assert_eq!(line_color(Severity::Info, &visuals), None);
    assert_eq!(
        line_color(Severity::Warning, &visuals),
        Some(visuals.warn_fg_color)
    );
    assert_eq!(
        line_color(Severity::Error, &visuals),
        Some(visuals.error_fg_color)
    );
    assert_eq!(
        line_color(Severity::Fatal, &visuals),
        Some(visuals.error_fg_color)
    );
}

#[test]
fn the_text_after_the_code_is_drawn() {
    let mut formatter = EventLines::new();
    let event = PipelineEvent::Message(Message {
        code: MessageCode {
            tool_id: "team-compiler",
            code: Cow::Borrowed("player_unlisted"),
        },
        severity: Severity::Warning,
        disposition: Disposition::Keep,
        scope: Scope::Folder {
            export_id: ExportId(1),
            path: ScopePath::new("Players/01 - T").unwrap(),
        },
        context: vec![("file".to_owned(), "boots.fmdl".to_owned())],
    });
    let line = LogLine::from_event(&mut formatter, &event).expect("a Message event gives a line");
    assert_eq!(
        line.text,
        "- export 1: Warning player_unlisted at Players/01 - T (file=boots.fmdl)"
    );
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    let nodes = window.idle(&mut panel, &[line]);
    assert!(
        shows(&nodes, " at Players/01 - T (file=boots.fmdl)"),
        "the line's text after the code is drawn"
    );
}

#[test]
fn a_codeless_line_stays_one_row_and_scrolls_sideways() {
    // A long line without a code must not wrap (the body's contract).
    // AccessKit bounds are the widget's own rect, unclipped: 400 monospace
    // characters run far past the 800-point window; a wrapped label would
    // be several rows tall instead of one.
    let long = "a".repeat(400);
    let lines = vec![
        LogLine::plain(Severity::Info, long.clone()),
        LogLine::plain(Severity::Info, "a tail row"),
    ];
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    let nodes = window.idle(&mut panel, &lines);
    let wide = bounds(&nodes, &long);
    let row = bounds(&nodes, "a tail row");
    assert!(
        wide.x1 > 800.0,
        "the long line is drawn wider than the window: {}",
        wide.x1 - wide.x0
    );
    assert!(
        (wide.y1 - wide.y0 - (row.y1 - row.y0)).abs() < 1.0,
        "the long line stays one row tall, not wrapped: {} vs {}",
        wide.y1 - wide.y0,
        row.y1 - row.y0
    );
}

/// The chevron's tooltips name the dock's noun (`BottomDock::new`'s third
/// argument): `the log` on the log panel, and a title whose own lowercase
/// would garble ("the live aatf") carries its own noun phrase instead. The
/// tooltip's text lands on the widget's accesskit description.
#[test]
fn the_chevrons_tooltip_names_the_docks_noun() {
    let lines = plain_lines(5);
    let mut panel = LogPanel::new("test-log");
    let mut window = Headless::new();
    let nodes = window.idle(&mut panel, &lines);
    assert_eq!(entry(&nodes, "▾").description(), Some("Collapse the log"));
    let nodes = click(&mut window, &mut panel, &lines, entry(&nodes, "▾"));
    let nodes = if nodes.iter().any(|node| node.label() == Some("▸")) {
        nodes
    } else {
        // The collapsed chevron draws on the frame after the click's.
        window.idle(&mut panel, &lines)
    };
    assert_eq!(entry(&nodes, "▸").description(), Some("Expand the log"));

    // A dock whose title is not its noun.
    let mut dock = BottomDock::new(egui::Id::unique("live"), "Live AATF", "Live AATF", 150.0);
    let mut draw = |events: Vec<egui::Event>| {
        window.frame(events, |ui| {
            dock.show(ui, |_| {}, |_| {});
        })
    };
    let nodes = draw(Vec::new());
    assert_eq!(entry(&nodes, "▾").description(), Some("Collapse Live AATF"));
    let at = centre(entry(&nodes, "▾"));
    draw(pointer_at(at, true));
    let nodes = draw(pointer_at(at, false));
    let nodes = if nodes.iter().any(|node| node.label() == Some("▸")) {
        nodes
    } else {
        draw(Vec::new())
    };
    assert_eq!(entry(&nodes, "▸").description(), Some("Expand Live AATF"));
}
