//! In a window too short for every entry, the tool entries scroll by the
//! wheel in the room between the head and the tail, which keep their places,
//! and the opened sidebar's entries stand where the rail's do (ST-SIDE-10).

use super::super::ENTRY_HEIGHT;
use super::*;

/// The scenario's fourteen stub tools, `(id, label)` pairs.
const TOOLS: [(&str, &str); 14] = [
    ("t01", "T01"),
    ("t02", "T02"),
    ("t03", "T03"),
    ("t04", "T04"),
    ("t05", "T05"),
    ("t06", "T06"),
    ("t07", "T07"),
    ("t08", "T08"),
    ("t09", "T09"),
    ("t10", "T10"),
    ("t11", "T11"),
    ("t12", "T12"),
    ("t13", "T13"),
    ("t14", "T14"),
];

/// A shell over the fourteen stub tools, the settings' own folder held by the
/// `Shell` so it lives as long as the test.
fn shell14() -> Shell {
    let dir = TestDir::new("studio_core_sidebar_short_window");
    let mut tools = Vec::new();
    let mut calls = Vec::new();
    for (id, label) in TOOLS {
        let (tool, tool_calls) = stub(id, label, None);
        tools.push(tool);
        calls.push(tool_calls);
    }
    let mut shell = shell_with(
        test_settings(dir.path()),
        tools,
        &[],
        Some(dir.path().to_path_buf()),
        calls,
    );
    shell._dir = Some(dir);
    shell
}

/// `frame_at` on a window `height` points tall (800 wide).
fn frame_at_height(
    egui_ctx: &egui::Context,
    app: &mut StudioApp,
    events: Vec<egui::Event>,
    time: Option<f64>,
    height: f32,
) -> FrameOutput {
    let mut input = raw_input(events, true, time, None);
    input.screen_rect = Some(egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(800.0, height),
    ));
    run_frame(egui_ctx, app, input)
}

/// `n` frames at `height` with no input, returning the last: `idle` at a
/// window height of the test's choosing.
fn idle_at_height(
    egui_ctx: &egui::Context,
    app: &mut StudioApp,
    n: usize,
    height: f32,
) -> FrameOutput {
    let mut out = frame_at_height(egui_ctx, app, Vec::new(), None, height);
    for _ in 1..n {
        out = frame_at_height(egui_ctx, app, Vec::new(), None, height);
    }
    out
}

/// The scroll room's top and bottom: from the head separator's bottom to the
/// tail separator's top — each separator a 9-point row flanked by the layout's
/// 3-point item spacing, which the entries never share with it.
fn scroll_band(nodes: &[accesskit::Node]) -> (f64, f64) {
    let head_end = combo(nodes).bounds().expect("the selector is drawn").y1;
    let help_top = entry(nodes, HELP_LABEL).bounds().unwrap().y0;
    (head_end + 15.0, help_top - 12.0)
}

/// The `i` icon glyph painted inside `bounds` (an entry row's): its text
/// shape and the clip it was painted under. A row scrolled past the scroll
/// area's clip still emits the shape with its origin outside the clip — it
/// paints nothing, so "in view" is the origin inside the clip.
fn icon_in(
    out: &FrameOutput,
    bounds: accesskit::Rect,
) -> Option<(&egui::epaint::TextShape, egui::Rect)> {
    out.shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape)
                if shape.galley.text() == "i"
                    && f64::from(shape.pos.x) >= bounds.x0
                    && f64::from(shape.pos.x) <= bounds.x1
                    && f64::from(shape.pos.y) >= bounds.y0
                    && f64::from(shape.pos.y) <= bounds.y1 =>
            {
                Some((shape, clipped.clip_rect))
            }
            _ => None,
        })
        .next()
}

/// The rail panel's bottom edge: the deepest clip bottom of the rail's
/// shapes — the panel clips its content to the window minus the status bar
/// (the frame's painted rect is no anchor: it grows to cover overflowing
/// contents).
fn rail_bottom(out: &FrameOutput) -> f32 {
    out.shapes
        .iter()
        .filter(|clipped| clipped.clip_rect.max.x <= RAIL_WIDTH)
        .map(|clipped| clipped.clip_rect.max.y)
        .fold(0.0, f32::max)
}

/// Whether the frame paints a scroll bar at `right` (the entries' right
/// edge): a thin rectangle — the bar's background or handle — taller than
/// one entry row. Entry fills are the full row width, so width rules them
/// out.
fn scroll_bar_at(out: &FrameOutput, right: f64) -> bool {
    out.shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Rect(shape)
            if shape.fill != egui::Color32::TRANSPARENT
                && (f64::from(shape.rect.max.x) - right).abs() <= 1.0
                && f64::from(shape.rect.width()) <= 10.0
                && f64::from(shape.rect.height()) > f64::from(ENTRY_HEIGHT) + 2.0)
    })
}

/// The pointer moved to `pos` and one notch of the wheel, as `wheel_notch`
/// builds for a node — but at a fixed spot, since the entries move.
fn wheel_notch_at(pos: egui::Pos2, up: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, if up { 1.0 } else { -1.0 }),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: egui::Modifiers::default(),
        },
    ]
}

/// `node`'s bounds, unwrapped.
fn bounds(node: &accesskit::Node) -> accesskit::Rect {
    node.bounds().expect("a widget has bounds")
}

// ST-SIDE-10
#[test]
fn in_a_short_window_the_entries_scroll_between_the_head_and_the_tail() {
    let Shell { mut app, .. } = shell14();
    let egui_ctx = test_context();
    let labels = TOOLS.map(|(_, label)| label);

    // The rail drawn in the 420-point window.
    let out = frame_at_height(&egui_ctx, &mut app, Vec::new(), Some(1.0), 420.0);
    let (band_top, band_bottom) = scroll_band(&out.nodes);
    let rail_bottom = rail_bottom(&out);
    let head = [bounds(entry(&out.nodes, "4cc")), bounds(combo(&out.nodes))];
    let tail = [
        bounds(entry(&out.nodes, HELP_LABEL)),
        bounds(entry(&out.nodes, SETTINGS_LABEL)),
    ];

    // No tool entry in view meets the tail: every entry's paint is clipped at
    // or above the tail's separator — where the tail stands unpinned, an entry
    // would paint under it.
    for label in labels {
        let (_, clip) = icon_in(&out, bounds(entry(&out.nodes, label)))
            .unwrap_or_else(|| panic!("{label}'s icon is painted"));
        assert!(
            f64::from(clip.max.y) <= band_bottom + 0.5,
            "{label}'s clip reaches the tail's separator: {clip:?} past {band_bottom}"
        );
    }
    let last = bounds(entry(&out.nodes, "T14"));
    assert!(
        last.y0 >= band_bottom,
        "the last tool entry is not in view: {last:?} vs band {band_top:.1}..{band_bottom:.1}"
    );
    assert!(
        tail[1].y1 <= f64::from(rail_bottom),
        "Settings ends inside the sidebar: {:?} vs rail bottom {rail_bottom}",
        tail[1]
    );

    // The wheel over the tool entries until they move no further: between
    // notches the pointer leaves the rail, so the rest that opens the sidebar
    // never completes and the entries scroll in the rail.
    let spot = egui::pos2(28.0, ((band_top + band_bottom) / 2.0) as f32);
    let off = egui::pos2(400.0, 300.0);
    let mut previous = f64::MAX;
    for notch in 0..60 {
        let out = frame_at_height(
            &egui_ctx,
            &mut app,
            wheel_notch_at(spot, false),
            None,
            420.0,
        );
        let y0 = bounds(entry(&out.nodes, "T14")).y0;
        if (y0 - previous).abs() < 0.01 {
            break;
        }
        assert!(notch < 59, "the entries still move after 60 notches");
        previous = y0;
        frame_at_height(
            &egui_ctx,
            &mut app,
            vec![egui::Event::PointerMoved(off)],
            None,
            420.0,
        );
    }
    let out = idle_at_height(&egui_ctx, &mut app, 3, 420.0);
    let last = bounds(entry(&out.nodes, "T14"));
    assert!(
        last.y1 <= band_bottom + 0.5 && last.y0 >= band_top - 0.5,
        "the last tool entry is in view above the tail's separator: {last:?} vs band {band_top:.1}..{band_bottom:.1}"
    );
    let (icon, clip) = icon_in(&out, last).expect("T14's icon is painted");
    assert!(
        clip.contains(icon.pos),
        "the last tool entry is in view: icon at {:?} clipped to {clip:?}",
        icon.pos
    );
    let first = bounds(entry(&out.nodes, "T01"));
    assert!(
        first.y1 <= band_top + 0.5,
        "the first tool entry is not in view: {first:?} vs band top {band_top}"
    );
    for (name, before) in ["4cc", "the selector", HELP_LABEL, SETTINGS_LABEL]
        .into_iter()
        .zip(head.into_iter().chain(tail))
    {
        let after = if name == "the selector" {
            bounds(combo(&out.nodes))
        } else {
            bounds(entry(&out.nodes, name))
        };
        assert_eq!(before, after, "{name} moved while the entries scrolled");
    }
    assert!(
        !scroll_bar_at(&out, bounds(entry(&out.nodes, "T01")).x1),
        "no scroll bar is drawn in the rail"
    );

    // The sidebar opened by a hover: every tool entry stands at the height it
    // has in the rail, and head and tail where they were.
    let rail_y: Vec<f64> = labels
        .iter()
        .map(|label| bounds(entry(&out.nodes, label)).y0)
        .collect();
    let cell = bounds(entry(&out.nodes, "T01")).x1 - bounds(entry(&out.nodes, "T01")).x0;
    let rail_pos = centre(entry(&out.nodes, "T14"));
    frame_at_height(
        &egui_ctx,
        &mut app,
        vec![egui::Event::PointerMoved(rail_pos)],
        Some(2.0),
        420.0,
    );
    let opened = frame_at_height(&egui_ctx, &mut app, Vec::new(), Some(2.3), 420.0);
    assert!(
        bounds(entry(&opened.nodes, "T01")).x1 - bounds(entry(&opened.nodes, "T01")).x0 > cell,
        "the sidebar opened on the hover"
    );
    for (label, before) in labels.iter().zip(rail_y) {
        let after = bounds(entry(&opened.nodes, label)).y0;
        assert!(
            (before - after).abs() < 0.5,
            "{label} moved on open: y {before:.1} in the rail, {after:.1} in the sidebar"
        );
    }
    for (name, before) in ["4cc", "the selector", HELP_LABEL, SETTINGS_LABEL]
        .into_iter()
        .zip(head.into_iter().chain(tail))
    {
        let after = if name == "the selector" {
            bounds(combo(&opened.nodes))
        } else {
            bounds(entry(&opened.nodes, name))
        };
        assert!(
            (before.x0 - after.x0).abs() < 0.5 && (before.y0 - after.y0).abs() < 0.5,
            "{name} moved on open: {before:?} -> {after:?}"
        );
    }
    assert!(
        !scroll_bar_at(&opened, bounds(entry(&opened.nodes, "T01")).x1),
        "no scroll bar is drawn in the opened sidebar"
    );

    // The same build in the window 780 points high: every entry in view, and a
    // wheel turn moves none.
    frame_at_height(
        &egui_ctx,
        &mut app,
        vec![egui::Event::PointerMoved(egui::pos2(400.0, 300.0))],
        Some(2.6),
        780.0,
    );
    let out = frame_at_height(&egui_ctx, &mut app, Vec::new(), Some(2.7), 780.0);
    let (band_top, band_bottom) = scroll_band(&out.nodes);
    for label in labels {
        let entry_bounds = bounds(entry(&out.nodes, label));
        assert!(
            entry_bounds.y0 >= band_top - 0.5 && entry_bounds.y1 <= band_bottom + 0.5,
            "{label} is in view in the tall window: {entry_bounds:?} vs band {band_top:.1}..{band_bottom:.1}"
        );
    }
    for (name, before) in ["4cc", "the selector"].into_iter().zip(head) {
        let after = if name == "the selector" {
            bounds(combo(&out.nodes))
        } else {
            bounds(entry(&out.nodes, name))
        };
        assert_eq!(
            before, after,
            "{name} does not stand where it does in a 780-point window"
        );
    }
    let before: Vec<f64> = labels
        .iter()
        .map(|label| bounds(entry(&out.nodes, label)).y0)
        .collect();
    let out = frame_at_height(
        &egui_ctx,
        &mut app,
        wheel_notch_at(centre(entry(&out.nodes, "T07")), false),
        Some(2.8),
        780.0,
    );
    for (label, before) in labels.iter().zip(before) {
        let after = bounds(entry(&out.nodes, label)).y0;
        assert_eq!(before, after, "{label} moved on a wheel turn at 780 points");
    }
}

// Tabbing to a tool entry scrolled out of view scrolls it back in: egui does
// not scroll a focused widget into view on its own.
#[test]
fn a_focused_entry_is_scrolled_into_view() {
    let Shell { mut app, .. } = shell14();
    let egui_ctx = test_context();

    frame_at_height(&egui_ctx, &mut app, Vec::new(), Some(1.0), 420.0);
    let mut t = 1.0;
    for _ in 0..30 {
        if app.sidebar.focus_label == Some("T14") {
            break;
        }
        t += 0.05;
        frame_at_height(&egui_ctx, &mut app, key(egui::Key::Tab), Some(t), 420.0);
    }
    assert_eq!(
        app.sidebar.focus_label,
        Some("T14"),
        "the last tool entry received focus within 30 tabs"
    );
    // Frames for the scroll to settle: it animates.
    let out = idle_at_height(&egui_ctx, &mut app, 20, 420.0);
    let (band_top, band_bottom) = scroll_band(&out.nodes);

    let last = bounds(entry(&out.nodes, "T14"));
    assert!(
        last.y0 >= band_top - 0.5 && last.y1 <= band_bottom + 0.5,
        "the focused entry is in view: {last:?} vs band {band_top:.1}..{band_bottom:.1}"
    );
    let (icon, clip) = icon_in(&out, last).expect("T14's icon is painted");
    assert!(
        clip.contains(icon.pos),
        "the focused entry is in view: icon at {:?} clipped to {clip:?}",
        icon.pos
    );
    let first = bounds(entry(&out.nodes, "T01"));
    assert!(
        first.y1 <= band_top + 0.5,
        "the first tool entry scrolled out: {first:?} vs band top {band_top}"
    );
}
