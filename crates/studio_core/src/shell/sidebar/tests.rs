use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use egui::accesskit::{self, Role, Toggled};
use pes_version::PesVersion;

use super::{
    ACTIVE_BAR_WIDTH, CLOVER_LEAVES, CLOVER_RING, ExeCheck, HELP_LABEL, ICON_SIZE,
    LABEL_END_PADDING, LABEL_GAP, RAIL_WIDTH, SETTINGS_LABEL, overlay_frame, overlay_id,
    selector_color, version_label,
};
use crate::help::{HelpSection, HelpTarget, HelpTopic};
use crate::pes_process::RunningPes;
use crate::settings::Settings;
use crate::shell::test_harness::*;
use crate::shell::{StudioApp, about, run_cli};
use crate::tool::{AppPaths, PlannedTool};

/// The overlay's inner width as the draft's row lays it out: the accent bar, the icon's
/// `ICON_SIZE` box centred in the rest of the icon cell `cell` wide, `LABEL_GAP`, the
/// widest body-text galley over `labels` (measured with the context's fonts) and
/// `LABEL_END_PADDING`.
fn overlay_inner_width(egui_ctx: &egui::Context, cell: f64, labels: &[&str]) -> f64 {
    let font = egui::TextStyle::Body.resolve(&egui_ctx.global_style());
    let widest = labels
        .iter()
        .map(|label| {
            egui_ctx
                .fonts_mut(|fonts| {
                    fonts.layout_no_wrap(label.to_string(), font.clone(), egui::Color32::WHITE)
                })
                .size()
                .x
        })
        .fold(0.0, f32::max);
    let bar = f64::from(ACTIVE_BAR_WIDTH);
    let icon_box_right = bar + (cell - bar) / 2.0 + f64::from(ICON_SIZE) / 2.0;
    icon_box_right + f64::from(LABEL_GAP) + f64::from(widest) + f64::from(LABEL_END_PADDING)
}

/// A key pressed event, as `hover`/`pointer` build pointer events.
fn key(key: egui::Key) -> Vec<egui::Event> {
    vec![egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]
}

/// Whether the frame paints a non-transparent `Shape::Rect` exactly over
/// `bounds` (within 2 points, for the visuals' expansion).
fn filled_at(out: &FrameOutput, bounds: accesskit::Rect) -> bool {
    out.shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Rect(shape)
            if shape.fill != egui::Color32::TRANSPARENT
                && (f64::from(shape.rect.min.x) - bounds.x0).abs() <= 2.0
                && (f64::from(shape.rect.min.y) - bounds.y0).abs() <= 2.0
                && (f64::from(shape.rect.max.x) - bounds.x1).abs() <= 2.0
                && (f64::from(shape.rect.max.y) - bounds.y1).abs() <= 2.0)
    })
}

/// The frame's `color` fill `ACTIVE_BAR_WIDTH` wide over the full height at
/// `bounds`' left edge: the active entry's accent bar.
fn accent_bar_at(
    out: &FrameOutput,
    bounds: accesskit::Rect,
    color: egui::Color32,
) -> Option<&egui::epaint::RectShape> {
    out.shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) => Some(shape),
            _ => None,
        })
        .find(|shape| {
            shape.fill == color
                && (f64::from(shape.rect.min.x) - bounds.x0).abs() <= 0.5
                && (f64::from(shape.rect.min.y) - bounds.y0).abs() <= 0.5
                && (f64::from(shape.rect.width()) - f64::from(ACTIVE_BAR_WIDTH)).abs() <= 0.5
                && (f64::from(shape.rect.height()) - (bounds.y1 - bounds.y0)).abs() <= 0.5
        })
}

/// How many of the frame's `Shape::Text` carry exactly `text`.
fn painted_text_count(out: &FrameOutput, text: &str) -> usize {
    out.shapes
        .iter()
        .filter(|clipped| {
            matches!(&clipped.shape, egui::Shape::Text(shape) if shape.galley.text() == text)
        })
        .count()
}

/// The `Shape::Text` carrying exactly `text` whose origin sits inside `rect`.
fn text_shape_in<'a>(
    out: &'a FrameOutput,
    text: &str,
    rect: egui::Rect,
) -> Option<&'a egui::epaint::TextShape> {
    out.shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) => Some(shape),
            _ => None,
        })
        .find(|shape| shape.galley.text() == text && rect.contains(shape.pos))
}

// TC-GUI-03
#[test]
fn the_sidebar_marks_the_active_tool_and_a_click_on_an_entry_switches_to_it() {
    let Shell {
        mut app,
        calls,
        _dir,
        ..
    } = shell();
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert_eq!(
        (calls[0].views(), calls[1].views()),
        (1, 0),
        "only the active view runs"
    );
    assert_eq!(entry(&nodes, "A").toggled(), Some(Toggled::True));
    assert_eq!(entry(&nodes, "B").toggled(), Some(Toggled::False));

    // egui hit-tests a click against the previous frame's widgets: the press comes a frame
    // after the layout, the release a frame after the press.
    let press = pointer(entry(&nodes, "B"), true);
    let release = pointer(entry(&nodes, "B"), false);
    frame(&egui_ctx, &mut app, press, true);
    frame(&egui_ctx, &mut app, release, true);
    assert_eq!(app.active, 1);
    assert_eq!(
        (calls[0].views(), calls[1].views()),
        (2, 1),
        "B's view ran on the click frame"
    );

    // The sidebar is drawn before the click is applied, so the entries show the switch on
    // the next frame.
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert_eq!(entry(&nodes, "A").toggled(), Some(Toggled::False));
    assert_eq!(entry(&nodes, "B").toggled(), Some(Toggled::True));
    assert_eq!((calls[0].views(), calls[1].views()), (2, 2));
}

// ST-SIDE-01
#[test]
fn the_gui_opens_on_the_tool_last_tool_names_and_lists_all_tools() {
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let (c, calls_c) = stub("c", "C", None);
    let dir = TestDir::new("studio_core_sidebar_last_named");
    let mut settings = test_settings(dir.path());
    settings.common.last_tool = Some("b".to_owned());
    let Shell { mut app, calls, .. } = shell_with(
        settings,
        vec![a, b, c],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a, calls_b, calls_c],
    );
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert_eq!(app.active, 1, "last_tool names the second tool");
    assert_eq!(
        (calls[0].views(), calls[1].views(), calls[2].views()),
        (0, 1, 0),
        "only the second tool's view is drawn"
    );
    // The full labels, in registration order (drawn top to bottom).
    let top = |label: &str| entry(&nodes, label).bounds().unwrap().y0;
    assert!(top("A") < top("B") && top("B") < top("C"), "registry order");
    assert_eq!(entry(&nodes, "B").toggled(), Some(Toggled::True));

    // The opened sidebar shows their full labels: the rail's entries are the
    // icon cell wide; resting the pointer on it opens the overlay, whose entries
    // span its inner width.
    let cell = entry(&nodes, "A").bounds().unwrap().width();
    let nodes = open_sidebar(&egui_ctx, &mut app, entry(&nodes, "A"), 1.0).nodes;
    let inner = overlay_inner_width(
        &egui_ctx,
        cell,
        &["A", "B", "C", HELP_LABEL, SETTINGS_LABEL],
    );
    for label in ["A", "B", "C"] {
        let width = entry(&nodes, label).bounds().unwrap().width();
        assert!(
            (width - inner).abs() < 0.5,
            "the opened sidebar's {label} spans its inner width ({width} vs {inner}, cell {cell})"
        );
    }
}

// ST-SIDE-02
#[test]
fn the_rail_opens_the_sidebar_over_the_view_on_hover_and_closes_on_leave() {
    let Shell {
        mut app,
        calls,
        _dir,
        ..
    } = shell();
    let egui_ctx = test_context();

    // Closed: each entry is the icon cell wide — the rail shows the icons alone.
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9)).nodes;
    let cell = entry(&nodes, "A").bounds().unwrap().width();
    assert!(
        cell < f64::from(RAIL_WIDTH),
        "the rail's entries are icon cells"
    );

    // The pointer at a rail entry at t = 1.00 and still there at 1.09: closed.
    let rail_pos = centre(entry(&nodes, "A"));
    frame_at(
        &egui_ctx,
        &mut app,
        vec![egui::Event::PointerMoved(rail_pos)],
        true,
        Some(1.00),
    );
    let closed = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(1.09)).nodes;
    let view_closed = calls[0].view_rect().unwrap();
    assert_eq!(
        entry(&closed, "A").bounds().unwrap().width(),
        cell,
        "a rest shorter than the delay keeps the rail"
    );

    // At 1.11, past the delay: open, every entry the overlay's inner width.
    let opened = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(1.11)).nodes;
    let inner = overlay_inner_width(&egui_ctx, cell, &["A", "B", HELP_LABEL, SETTINGS_LABEL]);
    for label in ["A", "B", HELP_LABEL, SETTINGS_LABEL] {
        let width = entry(&opened, label).bounds().unwrap().width();
        assert!(
            (width - inner).abs() < 0.5,
            "the open {label} spans the overlay's inner width ({width} vs {inner})"
        );
    }

    // The tool's view never moved: the same rect open and closed, its left edge
    // within the rail's width plus the central panel's margin.
    let view_open = calls[0].view_rect().unwrap();
    assert_eq!(view_closed, view_open, "the view kept its rect");
    assert!(
        view_open.min.x >= RAIL_WIDTH && view_open.min.x <= RAIL_WIDTH + 9.0,
        "the view starts at the rail's edge: {view_open:?}"
    );

    // Nothing moved when it opened: the logo's, each entry's, the help's and the
    // gear's top-left is the same open and closed (the widths differ).
    for label in ["4cc", "A", "B", HELP_LABEL, SETTINGS_LABEL] {
        let before = entry(&closed, label).bounds().unwrap();
        let after = entry(&opened, label).bounds().unwrap();
        assert!(
            (before.x0 - after.x0).abs() < 0.5 && (before.y0 - after.y0).abs() < 0.5,
            "{label} moved on open: {before:?} -> {after:?}"
        );
    }

    // The pointer moved to the view's middle: closed on the next frame.
    let middle = view_open.center();
    let nodes = frame_at(
        &egui_ctx,
        &mut app,
        vec![egui::Event::PointerMoved(middle)],
        true,
        Some(1.20),
    )
    .nodes;
    assert_eq!(
        entry(&nodes, "A").bounds().unwrap().width(),
        cell,
        "closed once the pointer left"
    );

    // A pointer that crosses the rail restarts the wait: on at 2.00, off at
    // 2.05, on again at 2.12 — still closed at 2.20, open at 2.28.
    for (pos, t) in [(rail_pos, 2.00), (middle, 2.05), (rail_pos, 2.12)] {
        frame_at(
            &egui_ctx,
            &mut app,
            vec![egui::Event::PointerMoved(pos)],
            true,
            Some(t),
        );
    }
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(2.20)).nodes;
    assert_eq!(
        entry(&nodes, "A").bounds().unwrap().width(),
        cell,
        "still closed at 0.08 s of rest"
    );
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(2.28)).nodes;
    assert!(
        entry(&nodes, "A").bounds().unwrap().width() > cell,
        "open at 0.16 s of rest"
    );

    // A press and release on rail entry B within 0.05 s switches at once, and
    // the overlay stays closed.
    frame_at(
        &egui_ctx,
        &mut app,
        vec![egui::Event::PointerMoved(middle)],
        true,
        Some(2.35),
    );
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(2.40)).nodes;
    frame_at(
        &egui_ctx,
        &mut app,
        pointer(entry(&nodes, "B"), true),
        true,
        Some(2.45),
    );
    frame_at(
        &egui_ctx,
        &mut app,
        pointer(entry(&nodes, "B"), false),
        true,
        Some(2.50),
    );
    assert_eq!(app.active, 1, "the click switched without the hover");
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(2.55)).nodes;
    assert_eq!(
        entry(&nodes, "A").bounds().unwrap().width(),
        cell,
        "the quick click did not open the overlay"
    );
}

// ST-SIDE-02: the rail's widgets leave on the frame the overlay opens, so an
// overlay that faded in would show a dim, half-empty sidebar on every open.
#[test]
fn the_overlay_opens_opaque_on_every_open() {
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();
    let panel_fill = egui_ctx.global_style().visuals.panel_fill;
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9)).nodes;
    let rail_pos = centre(entry(&nodes, "A"));
    let middle = egui::pos2(400.0, 300.0);

    for (round, start) in [(1, 1.0), (2, 2.0)] {
        frame_at(
            &egui_ctx,
            &mut app,
            vec![egui::Event::PointerMoved(rail_pos)],
            true,
            Some(start),
        );
        let out = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(start + 0.16));
        // The overlay's frame is the rect at the rail's left edge wider than the rail
        // and narrower than the 800-point window; `egui::Frame` paints it inside a
        // `Shape::Vec` with its shadow.
        let mut fills = Vec::new();
        let mut shapes: Vec<&egui::Shape> = out.shapes.iter().map(|c| &c.shape).collect();
        while let Some(shape) = shapes.pop() {
            match shape {
                egui::Shape::Vec(children) => shapes.extend(children),
                egui::Shape::Rect(rect)
                    if rect.rect.min.x.abs() < 1.0
                        && rect.rect.width() > RAIL_WIDTH + 1.0
                        && rect.rect.width() < 400.0 =>
                {
                    fills.push(rect.fill);
                }
                _ => {}
            }
        }
        assert!(
            fills.contains(&panel_fill),
            "open {round}: the overlay's frame is painted in the panel fill on its opening frame, \
             wide fills {fills:?}"
        );
        frame_at(
            &egui_ctx,
            &mut app,
            vec![egui::Event::PointerMoved(middle)],
            true,
            Some(start + 0.3),
        );
    }
}

// The opened sidebar casts no shadow (egui paints a frame's shadow as a blurred rect);
// the rail's right line, drawn down the layer's whole height half a point inside its
// right edge, as the rail panel's sits, marks where it ends over the view.
#[test]
fn the_overlay_casts_no_shadow_and_ends_on_the_rails_line() {
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();
    let closed = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9)).nodes;
    let out = open_sidebar(&egui_ctx, &mut app, entry(&closed, "A"), 1.0);
    let outer = egui_ctx
        .memory(|m| m.area_rect(overlay_id()))
        .expect("the open overlay has a rect");
    let line = egui_ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    let edge = outer.right() - 0.5;

    let mut blurred = Vec::new();
    let mut edge_lines = Vec::new();
    let mut shapes: Vec<&egui::Shape> = out.shapes.iter().map(|c| &c.shape).collect();
    while let Some(shape) = shapes.pop() {
        match shape {
            egui::Shape::Vec(children) => shapes.extend(children),
            egui::Shape::Rect(rect) if rect.blur_width > 0.0 => blurred.push(rect.rect),
            egui::Shape::LineSegment { points, stroke }
                if (points[0].x - edge).abs() < 0.01 && (points[1].x - edge).abs() < 0.01 =>
            {
                edge_lines.push((points[0].y, points[1].y, *stroke));
            }
            _ => {}
        }
    }
    assert!(
        blurred.is_empty(),
        "the open overlay paints no shadow, blurred rects {blurred:?}"
    );
    assert_eq!(
        edge_lines,
        vec![(outer.top(), outer.bottom(), line)],
        "one line in the quiet stroke down the overlay's right edge at x {edge}"
    );
}

// ST-SIDE-02's popup rule: the overlay stays open while a popup it opened is
// open.
#[test]
fn the_open_version_popup_holds_the_overlay_open() {
    let Shell {
        mut app,
        calls,
        _dir,
        ..
    } = shell();
    let egui_ctx = test_context();

    // The version combo clicked on the rail: the press opens the overlay, and
    // the release opens the popup from the overlay's selector.
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9)).nodes;
    let cell = entry(&nodes, "A").bounds().unwrap().width();
    frame_at(
        &egui_ctx,
        &mut app,
        pointer(combo(&nodes), true),
        true,
        Some(1.00),
    );
    frame_at(
        &egui_ctx,
        &mut app,
        pointer(combo(&nodes), false),
        true,
        Some(1.05),
    );

    // The pointer resting at the selector, past the delay: the overlay is open
    // and so is the popup, with the full labels.
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(1.20)).nodes;
    entry(&nodes, "PES 2021");
    assert!(
        entry(&nodes, "A").bounds().unwrap().width() > f64::from(RAIL_WIDTH),
        "the overlay is open"
    );

    // With the popup open and the pointer moved to the view's middle, the
    // overlay stays open.
    let middle = calls[0].view_rect().unwrap().center();
    let nodes = frame_at(
        &egui_ctx,
        &mut app,
        vec![egui::Event::PointerMoved(middle)],
        true,
        Some(1.25),
    )
    .nodes;
    entry(&nodes, "PES 2021");
    assert!(
        entry(&nodes, "A").bounds().unwrap().width() > cell,
        "the open popup holds the overlay open"
    );

    // The popup closed (Escape): the overlay closes on the next frame.
    frame_at(
        &egui_ctx,
        &mut app,
        key(egui::Key::Escape),
        true,
        Some(1.30),
    );
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(1.35)).nodes;
    assert_eq!(
        entry(&nodes, "A").bounds().unwrap().width(),
        cell,
        "the overlay closed with the popup"
    );
    assert!(!has_label(&nodes, "PES 2021"), "the popup is closed");
}

// A press on the rail's selector opens the overlay without the hover delay, so the
// release lands on the overlay's selector and its list always shows the full names.
#[test]
fn a_press_on_the_rails_selector_opens_the_sidebar_at_once_with_the_full_names() {
    let Shell {
        mut app,
        calls,
        _dir,
        ..
    } = shell();
    let egui_ctx = test_context();

    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9)).nodes;
    let cell = entry(&nodes, "A").bounds().unwrap().width();
    frame_at(&egui_ctx, &mut app, hover(combo(&nodes)), true, Some(1.00));
    frame_at(
        &egui_ctx,
        &mut app,
        pointer(combo(&nodes), true),
        true,
        Some(1.01),
    );

    // Still pressed, 20 ms after the pointer arrived: the overlay is open.
    let held = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(1.02)).nodes;
    assert!(
        entry(&held, "A").bounds().unwrap().width() > f64::from(RAIL_WIDTH),
        "the press opened the overlay before the hover delay"
    );

    // The release opens the popup from the overlay's selector, drawn from the
    // next frame on: full names only.
    frame_at(
        &egui_ctx,
        &mut app,
        pointer(combo(&nodes), false),
        true,
        Some(1.03),
    );
    let out = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(1.04));
    for version in PesVersion::ALL {
        entry(&out.nodes, &version.to_string());
        let year = version.number().to_string();
        assert!(
            !out.nodes
                .iter()
                .any(|node| node.label() == Some(year.as_str())
                    || node.value() == Some(year.as_str())),
            "no node reads {year}"
        );
        assert_eq!(painted_text_count(&out, &year), 0, "no text paints {year}");
    }

    // Escape closes the popup; the pointer moved to the view closes the overlay.
    frame_at(
        &egui_ctx,
        &mut app,
        key(egui::Key::Escape),
        true,
        Some(1.05),
    );
    let middle = calls[0].view_rect().unwrap().center();
    let nodes = frame_at(
        &egui_ctx,
        &mut app,
        vec![egui::Event::PointerMoved(middle)],
        true,
        Some(1.06),
    )
    .nodes;
    assert!(!has_label(&nodes, "PES 2021"), "the popup is closed");
    assert_eq!(
        entry(&nodes, "A").bounds().unwrap().width(),
        cell,
        "the overlay closed"
    );
}

// ST-SIDE-02's keyboard rule: while a sidebar widget holds a focus the keyboard
// gave it, the overlay is open. A pointer click on an entry gives no focus and
// keeps nothing open.
#[test]
fn the_overlay_opens_on_keyboard_focus_and_a_click_gives_none() {
    let Shell {
        mut app,
        calls,
        _dir,
        ..
    } = shell();
    let egui_ctx = test_context();

    // Closed, so the icon cell is measured.
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9)).nodes;
    let cell = entry(&nodes, "A").bounds().unwrap().width();

    // Tab until a rail entry has focus.
    let mut t = 1.0;
    let mut tabbed = None;
    for _ in 0..20 {
        frame_at(&egui_ctx, &mut app, key(egui::Key::Tab), true, Some(t));
        if matches!(app.sidebar.focus_label, Some("A" | "B")) {
            tabbed = Some(egui_ctx.memory(|m| m.focused()));
            break;
        }
        t += 0.05;
    }
    let focused = tabbed
        .expect("a rail entry received focus within 20 tabs")
        .expect("the entry's widget is the focused one");

    // The next frame opens the overlay and the same entry still has the focus.
    t += 0.05;
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(t)).nodes;
    assert_eq!(
        egui_ctx.memory(|m| m.focused()),
        Some(focused),
        "the focus survived the move to the overlay"
    );
    assert!(
        entry(&nodes, "A").bounds().unwrap().width() > cell,
        "the overlay opened"
    );

    // Escape clears the keyboard focus: the overlay closes.
    t += 0.05;
    frame_at(&egui_ctx, &mut app, key(egui::Key::Escape), true, Some(t));
    t += 0.05;
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(t)).nodes;
    assert_eq!(
        entry(&nodes, "A").bounds().unwrap().width(),
        cell,
        "the overlay closed with the focus"
    );

    // A pointer click on an entry followed by the pointer leaving: the click
    // gives the entry no focus, so nothing keeps the overlay open.
    let middle = calls[0].view_rect().unwrap().center();
    t += 0.05;
    frame_at(
        &egui_ctx,
        &mut app,
        pointer(entry(&nodes, "B"), true),
        true,
        Some(t),
    );
    t += 0.05;
    frame_at(
        &egui_ctx,
        &mut app,
        pointer(entry(&nodes, "B"), false),
        true,
        Some(t),
    );
    assert_eq!(app.active, 1, "the click switched the tool");
    assert_eq!(
        egui_ctx.memory(|m| m.focused()),
        None,
        "a pointer click gives a widget no focus (egui focuses buttons on Tab only)"
    );
    t += 0.05;
    frame_at(
        &egui_ctx,
        &mut app,
        vec![egui::Event::PointerMoved(middle)],
        true,
        Some(t),
    );
    t += 0.05;
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(t)).nodes;
    assert_eq!(
        entry(&nodes, "A").bounds().unwrap().width(),
        cell,
        "closed within two frames of the pointer leaving"
    );
}

// The overlay is as wide as the longest label needs on one line.
#[test]
fn the_overlay_is_as_wide_as_the_longest_label_on_one_line() {
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "Player aesthetics editor", None);
    let dir = TestDir::new("studio_core_sidebar_width");
    let Shell { mut app, .. } = shell_with(
        test_settings(dir.path()),
        vec![a, b],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a, calls_b],
    );
    let egui_ctx = test_context();

    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9)).nodes;
    let cell = entry(&nodes, "A").bounds().unwrap().width();
    let nodes = open_sidebar(&egui_ctx, &mut app, entry(&nodes, "A"), 1.0).nodes;
    let width = entry(&nodes, "Player aesthetics editor")
        .bounds()
        .unwrap()
        .width();
    let inner = overlay_inner_width(
        &egui_ctx,
        cell,
        &["A", "Player aesthetics editor", HELP_LABEL, SETTINGS_LABEL],
    );
    assert!(
        (width - inner).abs() < 0.5,
        "the longest label's row spans the overlay's inner width ({width} vs {inner})"
    );
}

// A row is a plain row until it is hovered, pressed, focused or active: an
// inactive, unhovered entry paints no button frame.
#[test]
fn an_inactive_row_paints_no_background_until_it_is_touched() {
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();

    let out = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9));
    let active = entry(&out.nodes, "A").bounds().unwrap();
    let inactive = entry(&out.nodes, "B").bounds().unwrap();
    assert!(
        filled_at(&out, active),
        "the active entry paints its selection background"
    );
    assert!(
        !filled_at(&out, inactive),
        "an inactive, unhovered entry paints no button frame"
    );

    // The pointer resting on the inactive entry, before the hover delay runs
    // out: the row is hovered, so it paints — while the sidebar is still a rail.
    let out = frame_at(
        &egui_ctx,
        &mut app,
        hover(entry(&out.nodes, "B")),
        true,
        Some(1.0),
    );
    assert!(
        filled_at(&out, entry(&out.nodes, "B").bounds().unwrap()),
        "the hovered row paints its background"
    );
}

// ST-SIDE-01's "the opened sidebar shows their full labels" and item 3's "the
// rail shows the icons alone": a label's text is painted in the overlay, never
// on the rail.
#[test]
fn the_labels_are_painted_in_the_overlay_and_never_on_the_rail() {
    const PLANNED: &[PlannedTool] = &[PlannedTool {
        id: "music-player",
        label: "Music player",
        icon: "m",
    }];
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let dir = TestDir::new("studio_core_sidebar_painted_labels");
    let Shell { mut app, .. } = shell_with(
        test_settings(dir.path()),
        vec![a, b],
        PLANNED,
        Some(dir.path().to_path_buf()),
        vec![calls_a, calls_b],
    );
    let egui_ctx = test_context();

    let rail = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9));
    assert_eq!(
        painted_text_count(&rail, "\u{F100}"),
        1,
        "the rail paints its logo's ring (the frame's shapes are collected)"
    );
    for label in ["A", "B", "Music player", HELP_LABEL, SETTINGS_LABEL] {
        assert_eq!(
            painted_text_count(&rail, label),
            0,
            "the rail paints no {label:?} label"
        );
    }

    let open = open_sidebar(&egui_ctx, &mut app, entry(&rail.nodes, "A"), 1.0);
    for label in ["A", "B", "Music player", HELP_LABEL, SETTINGS_LABEL] {
        assert_eq!(
            painted_text_count(&open, label),
            1,
            "the overlay paints one {label:?} label"
        );
    }
}

// An entry row: the icon centred in the icon cell (the row's first cell-width
// points less the accent bar's width at its left), the label starting the
// draft's gap past the right edge of the icon's 24-point box centred there,
// vertically centred on the row.
#[test]
fn a_row_paints_its_icon_centred_in_the_cell_and_its_label_after_it() {
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();

    let out = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9));
    let cell = entry(&out.nodes, "A").bounds().unwrap().width() as f32;
    let out = open_sidebar(&egui_ctx, &mut app, entry(&out.nodes, "A"), 1.0);
    let bounds = entry(&out.nodes, "A").bounds().unwrap();
    let row = egui::Rect::from_min_max(
        egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
        egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
    );
    let icon_cell = egui::Rect::from_min_max(
        egui::pos2(row.min.x + ACTIVE_BAR_WIDTH, row.min.y),
        egui::pos2(row.min.x + cell, row.max.y),
    );

    // The stub tools' icon is "i"; the same glyph sits on B's row, so the
    // cell's bounds pick A's.
    let icon = text_shape_in(&out, "i", icon_cell).expect("A's icon is painted");
    let icon_rect = egui::Rect::from_min_size(icon.pos, icon.galley.size());
    let centre = icon_cell.center();
    assert!(
        (icon_rect.center().x - centre.x).abs() <= 0.5
            && (icon_rect.center().y - centre.y).abs() <= 0.5,
        "the icon is centred in the icon cell ({icon_rect:?} in {icon_cell:?})"
    );

    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
    let label = text_shape_in(&out, "A", screen).expect("A's label is painted");
    let expected_x = centre.x + ICON_SIZE / 2.0 + LABEL_GAP;
    assert!(
        (label.pos.x - expected_x).abs() <= 0.5,
        "the label starts the gap past the icon box's right edge ({} vs {expected_x})",
        label.pos.x
    );
    let label_centre_y = label.pos.y + label.galley.size().y / 2.0;
    assert!(
        (label_centre_y - row.center().y).abs() <= 0.5,
        "the label is vertically centred on the row ({label_centre_y} vs {})",
        row.center().y
    );
}

// The active entry alone gets the accent-colored bar at its left edge, the
// draft's 3 px accent bar.
#[test]
fn the_accent_bar_marks_only_the_active_row() {
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();

    let out = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9));
    let color = egui_ctx.global_style().visuals.selection.stroke.color;
    assert!(
        accent_bar_at(&out, entry(&out.nodes, "A").bounds().unwrap(), color).is_some(),
        "the active row carries the accent bar"
    );
    for label in ["B", HELP_LABEL, SETTINGS_LABEL] {
        assert!(
            accent_bar_at(&out, entry(&out.nodes, label).bounds().unwrap(), color).is_none(),
            "the {label} row carries no accent bar"
        );
    }
}

// The accent bar's two left corners are rounded as the entry's, the draft's
// `.tool.on` border, and its right ones square against the fill.
#[test]
fn the_accent_bar_rounds_its_left_corners_only() {
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();

    let out = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9));
    let color = egui_ctx.global_style().visuals.selection.stroke.color;
    let bar = accent_bar_at(&out, entry(&out.nodes, "A").bounds().unwrap(), color)
        .expect("the active row carries the accent bar");
    assert_eq!(
        bar.corner_radius,
        egui::CornerRadius {
            nw: 2,
            ne: 0,
            sw: 2,
            se: 0
        }
    );
}

// ST-SIDE-02's "while the pointer is on it": the overlay's rect reaches past
// the drawn content by the frame's margins, and the overlay holds while the
// pointer is anywhere on it.
#[test]
fn the_overlay_holds_while_the_pointer_is_anywhere_on_it() {
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();

    let closed = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9)).nodes;
    let cell = entry(&closed, "A").bounds().unwrap().width();
    let out = open_sidebar(&egui_ctx, &mut app, entry(&closed, "A"), 1.0);
    let row_y = centre(entry(&out.nodes, "A")).y;

    // The overlay's right edge: its left edge plus the inner width (the icon box,
    // the gap, the widest label and the padding after it) plus the frame's
    // margins.
    let outer_left = egui_ctx
        .memory(|m| m.area_rect(overlay_id()))
        .expect("the open overlay has a rect")
        .min
        .x;
    let style = egui_ctx.global_style();
    let margins = overlay_frame(&style).total_margin().sum().x;
    let inner = overlay_inner_width(&egui_ctx, cell, &["A", "B", HELP_LABEL, SETTINGS_LABEL]);
    let right = f64::from(outer_left) + inner + f64::from(margins);

    // On the margin band, right of the drawn content but inside the outer
    // rect: the overlay holds.
    frame_at(
        &egui_ctx,
        &mut app,
        vec![egui::Event::PointerMoved(egui::pos2(
            (right - 2.0) as f32,
            row_y,
        ))],
        true,
        Some(1.3),
    );
    let out = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(1.35));
    assert!(
        entry(&out.nodes, "A").bounds().unwrap().width() > cell,
        "the overlay holds with the pointer on its margin band"
    );

    // Two points past the edge: closed.
    frame_at(
        &egui_ctx,
        &mut app,
        vec![egui::Event::PointerMoved(egui::pos2(
            (right + 2.0) as f32,
            row_y,
        ))],
        true,
        Some(1.4),
    );
    let out = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(1.45));
    assert_eq!(
        entry(&out.nodes, "A").bounds().unwrap().width(),
        cell,
        "the overlay closes once the pointer leaves it"
    );
}

// A row paints its button frame while it is in use, which a keyboard focus
// alone already makes it: the pointer needn't be anywhere near.
#[test]
fn a_focused_row_paints_its_background_untouched_by_the_pointer() {
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();

    // No pointer events at all: Tab until the inactive entry B holds the
    // keyboard focus; the rail stays open or closed on whatever it was.
    let mut out = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9));
    let mut t = 0.95;
    for _ in 0..20 {
        out = frame_at(&egui_ctx, &mut app, key(egui::Key::Tab), true, Some(t));
        if app.sidebar.focus_label == Some("B") {
            break;
        }
        t += 0.05;
    }
    assert_eq!(
        app.sidebar.focus_label,
        Some("B"),
        "B's row took the focus within 20 tabs"
    );
    assert!(
        filled_at(&out, entry(&out.nodes, "B").bounds().unwrap()),
        "the focused row paints its background"
    );
}

// ST-SIDE-03
#[test]
fn the_selector_is_colored_by_the_exe_found_in_the_pes_folder() {
    let egui_ctx = test_context();
    egui_ctx.all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);

    // A shell at PES 21 with `data_dir` in the given folder; `pes_folder_path` is the
    // given folder.
    let with_pes_folder = |folder: &PathBuf| {
        let (a, calls_a) = stub("a", "A", None);
        let mut settings = Settings::default();
        settings.common.pes_version = PesVersion::Pes21;
        settings.common.pes_folder_path = folder.display().to_string();
        shell_with(settings, vec![a], &[], Some(folder.clone()), vec![calls_a])
    };
    let hover_selector = |app: &mut StudioApp| {
        let nodes = frame(&egui_ctx, app, Vec::new(), true).nodes;
        frame(&egui_ctx, app, hover(combo(&nodes)), true);
        // `is_still` takes about 0.1 s of frames with no pointer movement.
        idle(&egui_ctx, app, 8).nodes
    };
    let visuals = egui::Visuals::dark();
    let color_of =
        |app: &StudioApp| selector_color(app.sidebar.exe_check.as_ref().unwrap().2, &visuals);

    // Another version's exe: red, the tooltip names what was found.
    let dir = test_dir("studio_core_sidebar_pes19");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("PES2019.exe"), b"").unwrap();
    let Shell { mut app, .. } = with_pes_folder(&dir);
    frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_eq!(
        app.sidebar.exe_check.as_ref().map(|(_, _, check)| *check),
        Some(ExeCheck::OtherVersion(PesVersion::Pes19))
    );
    assert_eq!(color_of(&app), Some(visuals.error_fg_color), "red");
    let nodes = hover_selector(&mut app);
    let expected = format!("{} holds PES2019.exe", dir.display());
    assert!(
        has_text(&nodes, &expected),
        "the tooltip names the exe found"
    );
    fs::remove_dir_all(&dir).unwrap();

    // No PES exe: orange, the tooltip says none was found.
    let dir = test_dir("studio_core_sidebar_noexe");
    fs::create_dir_all(&dir).unwrap();
    let Shell { mut app, .. } = with_pes_folder(&dir);
    frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_eq!(
        app.sidebar.exe_check.as_ref().map(|(_, _, check)| *check),
        Some(ExeCheck::NoExe)
    );
    assert_eq!(color_of(&app), Some(visuals.warn_fg_color), "orange");
    let nodes = hover_selector(&mut app);
    assert!(has_text(
        &nodes,
        &format!("There is no PES exe in {}", dir.display())
    ));
    fs::remove_dir_all(&dir).unwrap();

    // A name the games never ship is not an exe either: `PES20021.exe` is not PES 21's
    // (`PES2021.exe`), so the folder reads as no exe at all.
    let dir = test_dir("studio_core_sidebar_pes20021");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("PES20021.exe"), b"").unwrap();
    let Shell { mut app, .. } = with_pes_folder(&dir);
    frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_eq!(
        app.sidebar.exe_check.as_ref().map(|(_, _, check)| *check),
        Some(ExeCheck::NoExe)
    );
    assert_eq!(color_of(&app), Some(visuals.warn_fg_color), "orange");
    fs::remove_dir_all(&dir).unwrap();

    // A directory named like the exe is not an exe (the compiler's preflight
    // counts files only): orange like an empty folder.
    let dir = test_dir("studio_core_sidebar_dir_named_exe");
    fs::create_dir_all(dir.join("PES2021.exe")).unwrap();
    let Shell { mut app, .. } = with_pes_folder(&dir);
    frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_eq!(
        app.sidebar.exe_check.as_ref().map(|(_, _, check)| *check),
        Some(ExeCheck::NoExe)
    );
    assert_eq!(color_of(&app), Some(visuals.warn_fg_color), "orange");
    fs::remove_dir_all(&dir).unwrap();

    // The folder missing: orange too, the tooltip names it.
    let dir = test_dir("studio_core_sidebar_missing");
    let Shell { mut app, .. } = with_pes_folder(&dir);
    frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_eq!(
        app.sidebar.exe_check.as_ref().map(|(_, _, check)| *check),
        Some(ExeCheck::NoFolder)
    );
    assert_eq!(color_of(&app), Some(visuals.warn_fg_color), "orange");
    let nodes = hover_selector(&mut app);
    assert!(has_text(
        &nodes,
        &format!("{} does not exist", dir.display())
    ));

    // The selected version's exe: no colour.
    let dir = test_dir("studio_core_sidebar_pes21");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("PES2021.exe"), b"").unwrap();
    let Shell { mut app, .. } = with_pes_folder(&dir);
    frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_eq!(
        app.sidebar.exe_check.as_ref().map(|(_, _, check)| *check),
        Some(ExeCheck::Found)
    );
    assert_eq!(color_of(&app), None, "no colour");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_exe_check_relists_on_a_change_or_on_focus_never_per_frame() {
    let egui_ctx = test_context();
    let dir = test_dir("studio_core_sidebar_recheck");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("PES2021.exe"), b"").unwrap();

    let (a, calls_a) = stub("a", "A", None);
    let mut settings = Settings::default();
    settings.common.pes_version = PesVersion::Pes21;
    settings.common.pes_folder_path = dir.display().to_string();
    let Shell {
        mut app, settings, ..
    } = shell_with(settings, vec![a], &[], Some(dir.clone()), vec![calls_a]);

    frame(&egui_ctx, &mut app, Vec::new(), true);
    frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_eq!(app.sidebar.exe_listings, 1, "no re-list per frame");

    // The window's focus returning re-lists.
    frame(&egui_ctx, &mut app, Vec::new(), false);
    frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_eq!(app.sidebar.exe_listings, 2, "focus regained re-lists");

    // A version or path change re-lists.
    settings.lock().unwrap().common.pes_version = PesVersion::Pes19;
    frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_eq!(app.sidebar.exe_listings, 3, "the version changed");
    fs::remove_dir_all(&dir).unwrap();
}

// ST-SIDE-04
#[test]
fn a_version_change_blocker_disables_the_selector_and_a_pick_saves_the_version() {
    let egui_ctx = test_context();
    egui_ctx.all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);

    // While a tool blocks, the selector is disabled and its tooltip is the reason.
    let (a, calls_a) = stub("a", "A", Some("a compile is running"));
    let (b, calls_b) = stub("b", "B", None);
    let dir = test_dir("studio_core_sidebar_blocked");
    fs::create_dir_all(&dir).unwrap();
    let Shell {
        mut app, settings, ..
    } = shell_with(
        test_settings(&dir),
        vec![a, b],
        &[],
        Some(dir.clone()),
        vec![calls_a, calls_b],
    );
    let version_before = settings.lock().unwrap().common.pes_version;
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert!(combo(&nodes).is_disabled(), "the selector is disabled");
    frame(&egui_ctx, &mut app, hover(combo(&nodes)), true);
    let nodes = idle(&egui_ctx, &mut app, 8).nodes;
    assert!(
        has_text(&nodes, "A: a compile is running"),
        "the tooltip is the blocking tool's reason"
    );
    assert_eq!(
        settings.lock().unwrap().common.pes_version,
        version_before,
        "the setting keeps its value"
    );

    // Unblocked: a pick writes the settings file and the tools read it next frame.
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let Shell {
        mut app,
        settings,
        calls,
        ..
    } = shell_with(
        test_settings(&dir),
        vec![a, b],
        &[],
        Some(dir.clone()),
        vec![calls_a, calls_b],
    );
    assert_ne!(
        Settings::default().common.pes_version,
        PesVersion::Pes21,
        "the test's pick really changes the setting"
    );
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, combo(&nodes));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "PES 2021"));
    assert_eq!(
        settings.lock().unwrap().common.pes_version,
        PesVersion::Pes21
    );
    // The save is applied at the next frame's logic.
    frame(&egui_ctx, &mut app, Vec::new(), true);
    let written = Settings::load(&dir.join(crate::settings::SETTINGS_FILE_NAME)).unwrap();
    assert_eq!(written.common.pes_version, PesVersion::Pes21);
    assert_eq!(calls[0].version_seen(), Some(PesVersion::Pes21));
    assert_eq!(calls[1].version_seen(), Some(PesVersion::Pes21));
    fs::remove_dir_all(&dir).unwrap();
}

// ST-SIDE-07
#[test]
fn the_planned_tools_follow_the_registered_ones_grayed_and_inert() {
    const PLANNED: &[PlannedTool] = &[
        PlannedTool {
            id: "a",
            label: "A",
            icon: "a",
        },
        PlannedTool {
            id: "planned-one",
            label: "Planned one",
            icon: "p",
        },
        PlannedTool {
            id: "planned-two",
            label: "Planned two",
            icon: "q",
        },
    ];
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let dir = TestDir::new("studio_core_sidebar_planned");
    let Shell { mut app, .. } = shell_with(
        test_settings(dir.path()),
        vec![a, b],
        PLANNED,
        Some(dir.path().to_path_buf()),
        vec![calls_a, calls_b],
    );
    let egui_ctx = test_context();
    egui_ctx.all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    // The planned entries are label-role nodes: the text sits in `value`.
    let planned = |label: &str| {
        nodes
            .iter()
            .find(|node| node.value() == Some(label))
            .unwrap_or_else(|| panic!("no text node {label:?}"))
    };
    let top = |node: &accesskit::Node| node.bounds().unwrap().y0;
    assert!(
        top(entry(&nodes, "B")) < top(planned("Planned one"))
            && top(planned("Planned one")) < top(planned("Planned two")),
        "the planned entries follow the registered ones, in registry order"
    );
    assert!(
        planned("Planned one").is_disabled(),
        "the planned entry is grayed"
    );

    // Not clickable: a click changes nothing.
    click(&egui_ctx, &mut app, planned("Planned one"));
    assert_eq!(app.active, 0);

    // Its tooltip says the tool is coming, and nothing more.
    idle(&egui_ctx, &mut app, 10);
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    let node = nodes
        .iter()
        .find(|node| node.value() == Some("Planned one"))
        .unwrap();
    frame(&egui_ctx, &mut app, hover(node), true);
    let nodes = idle(&egui_ctx, &mut app, 8).nodes;
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Coming soon™")),
        "the tooltip reads exactly \"Coming soon™\""
    );
}

// ST-SIDE-08
#[test]
fn a_cli_only_tool_is_a_planned_entry_never_active_and_still_registered() {
    const PLANNED: &[PlannedTool] = &[
        PlannedTool {
            id: "a",
            label: "A",
            icon: "a",
        },
        PlannedTool {
            id: "b",
            label: "B",
            icon: "b",
        },
        PlannedTool {
            id: "c",
            label: "C",
            icon: "c",
        },
        PlannedTool {
            id: "planned-one",
            label: "Planned one",
            icon: "p",
        },
    ];
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    // A chapter with a topic, so `showing` can name what it opened on.
    let (c, calls_c) = stub_help(
        "c",
        "C",
        None,
        HelpSection {
            title: "C",
            topics: vec![HelpTopic {
                slug: "c-topic",
                title: "C's topic",
                body: "C's help body.",
            }],
            messages: Vec::new(),
        },
    );
    let dir = TestDir::new("studio_core_sidebar_cli_only");
    let mut settings = test_settings(dir.path());
    settings.common.last_tool = Some("c".to_owned());
    // C registered first, as `studio` registers the Team compiler: the fallback must skip
    // it, not just take the first tool.
    let Shell {
        mut app,
        ctx,
        calls,
        ..
    } = shell_in(
        settings,
        vec![c, a, b],
        PLANNED,
        &["c"],
        AppPaths {
            exe_dir: PathBuf::from("exe"),
            data_dir: Some(dir.path().to_path_buf()),
        },
        None,
        vec![calls_c, calls_a, calls_b],
    );
    let egui_ctx = test_context();
    egui_ctx.all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);

    // `last_tool` names the CLI-only tool: the first window tool is active instead.
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert_eq!(
        app.active, 1,
        "the CLI-only tool is never active: the first window tool is"
    );
    assert_eq!(
        (calls[0].views(), calls[1].views(), calls[2].views()),
        (0, 1, 0),
        "A's view is the one drawn"
    );

    // C has no live entry: the tool entries are buttons labeled with the tool's name.
    let live = |label: &str| {
        nodes
            .iter()
            .any(|node| node.role() == Role::Button && node.label() == Some(label))
    };
    assert!(live("A"), "a window tool's entry is a button");
    assert!(!live("C"), "the CLI-only tool has no live entry");

    // C is drawn after the window tools, as a planned entry in registry order, grayed.
    let planned = |label: &str| {
        nodes
            .iter()
            .find(|node| node.value() == Some(label))
            .unwrap_or_else(|| panic!("no text node {label:?}"))
    };
    let top = |node: &accesskit::Node| node.bounds().unwrap().y0;
    assert!(
        top(entry(&nodes, "B")) < top(planned("C"))
            && top(planned("C")) < top(planned("Planned one")),
        "the CLI-only entry sits among the planned ones, in registry order"
    );
    assert!(planned("C").is_disabled(), "the CLI-only entry is grayed");

    // Not clickable: a click changes nothing.
    click(&egui_ctx, &mut app, planned("C"));
    assert_eq!(app.active, 1);
    assert_eq!(calls[0].views(), 0, "C's view never ran");

    // Its tooltip is a planned entry's: its window is what is coming.
    idle(&egui_ctx, &mut app, 10);
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    let node = nodes.iter().find(|node| node.value() == Some("C")).unwrap();
    frame(&egui_ctx, &mut app, hover(node), true);
    let nodes = idle(&egui_ctx, &mut app, 8).nodes;
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Coming soon™")),
        "the tooltip reads exactly \"Coming soon™\""
    );

    // A switch request naming it is ignored, as an unknown id is.
    ctx.switch_to_tool("c");
    frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_eq!(app.active, 1, "no switch makes the CLI-only tool active");

    // Its settings section is not in the settings menu, though its table is not
    // empty: the menu adds no node labeled "C" to what the sidebar's entry draws,
    // while A's header adds one.
    let labeled = |nodes: &[accesskit::Node], label: &str| {
        nodes
            .iter()
            .filter(|node| node.label() == Some(label))
            .count()
    };
    let (before_a, before_c) = (labeled(&nodes, "A"), labeled(&nodes, "C"));
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    assert_eq!(
        labeled(&nodes, "A"),
        before_a + 1,
        "the menu is open, A's section header drawn"
    );
    assert_eq!(
        labeled(&nodes, "C"),
        before_c,
        "the CLI-only tool has no section in the settings menu"
    );

    // Its help chapter is in the help window.
    app.open_help(&HelpTarget {
        tool: "c".to_owned(),
        topic: None,
        anchor: None,
    });
    assert_eq!(
        app.help_window.showing(),
        Some(("c", "c-topic")),
        "C's chapter is in the help window"
    );

    // And its CLI commands still run.
    let matches = app.tools[0].cli_command().get_matches_from(vec!["c"]);
    let code = run_cli(&app.tools, "c", &matches, &ctx);
    assert_eq!(code.unwrap(), 0, "the CLI-only tool's commands still run");
}

// The logo's About block, its links and Copy; the Help link opens the help window.
// ST-SIDE-06
#[test]
fn the_logo_opens_about_and_copy_puts_the_version_block_on_the_clipboard() {
    let dir = test_dir("studio_core_sidebar_about");
    fs::create_dir_all(&dir).unwrap();
    let (a, calls_a) = stub("a", "A", None);
    let mut settings = test_settings(&dir);
    settings.common.pes_version = PesVersion::Pes21;
    let Shell { mut app, .. } =
        shell_with(settings, vec![a], &[], Some(dir.clone()), vec![calls_a]);
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "4cc"));

    let out = frame(&egui_ctx, &mut app, Vec::new(), true);
    let block = format!(
        "{} {}\nBuild: {}\nTarget: {}\nData location: {}\nPES version: PES 2021",
        crate::APP_NAME,
        crate::shell::version_label(),
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        env!("STUDIO_TARGET_TRIPLE"),
        dir.display(),
    );
    assert!(
        out.nodes
            .iter()
            .any(|node| node.value() == Some(block.as_str())),
        "the version block is shown"
    );
    entry(&out.nodes, "Release page");
    entry(&out.nodes, "Repository");

    // The links open their URLs.
    let out = click(&egui_ctx, &mut app, entry(&out.nodes, "Repository"));
    assert!(
        out.platform
            .commands
            .contains(&egui::OutputCommand::OpenUrl(egui::OpenUrl::same_tab(
                about::REPOSITORY_URL
            ))),
        "the repository link opens {}",
        env!("CARGO_PKG_REPOSITORY")
    );

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    let out = click(&egui_ctx, &mut app, entry(&nodes, "Release page"));
    assert!(
        out.platform
            .commands
            .contains(&egui::OutputCommand::OpenUrl(egui::OpenUrl::same_tab(
                about::RELEASE_PAGE_URL
            ))),
        "the release-page link opens the releases listing"
    );

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    let out = click(&egui_ctx, &mut app, entry(&nodes, "Copy"));
    assert!(
        out.platform
            .commands
            .contains(&egui::OutputCommand::CopyText(block)),
        "Copy puts exactly the version block on the clipboard"
    );

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "Close"));
    assert!(!app.sidebar.about_open, "Close dismisses the popup");

    // The popup's Help link closes it and opens the help window on Setting up.
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "4cc"));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    // egui reports a `ui.link` as a `Link` node carrying its text in `label`
    // (egui 0.37's `Role`).
    let link = nodes
        .iter()
        .find(|node| node.role() == Role::Link && node.label() == Some("Help"))
        .expect("the About popup's Help link");
    click(&egui_ctx, &mut app, link);
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    assert!(!app.sidebar.about_open, "the Help link closed the popup");
    assert_eq!(
        app.help_window.showing(),
        Some(("setting-up", "install")),
        "the help window opened on the Setting up chapter"
    );
    assert!(
        has_text(&nodes, "unpack it anywhere"),
        "the chapter's first topic is drawn"
    );
    fs::remove_dir_all(&dir).unwrap();
}

// The About popup's title draws in the window-title style (the drafts'
// `.window .title`): 14 points in the strong colour, not `Heading`'s 15,
// which a page's heading keeps.
#[test]
fn the_about_popups_title_is_14_points_and_strong() {
    let dir = test_dir("studio_core_about_title");
    fs::create_dir_all(&dir).unwrap();
    let (a, calls_a) = stub("a", "A", None);
    let Shell { mut app, .. } = shell_with(
        test_settings(&dir),
        vec![a],
        &[],
        Some(dir.clone()),
        vec![calls_a],
    );
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "4cc"));
    // The modal's first frame is a sizing pass; the settled frame's shapes
    // are painted.
    let out = idle(&egui_ctx, &mut app, 2);
    let title = format!("About {}", crate::APP_NAME);
    let shape = out
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == title => Some(shape),
            _ => None,
        })
        .next()
        .expect("the popup's title is painted");
    assert_eq!(
        shape.galley.job.sections[0].format.font_id.size, 14.0,
        "the drafts' .window .title"
    );
    let strong = egui_ctx
        .style_of(egui_ctx.theme())
        .visuals
        .strong_text_color();
    assert_eq!(
        shape.galley.job.sections[0].format.color, strong,
        "the title's strong colour"
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// The place of the drawn layer with `id` in egui's back-to-front layer order.
fn layer_rank(egui_ctx: &egui::Context, id: egui::Id) -> usize {
    egui_ctx.memory(|memory| {
        memory
            .layer_ids()
            .position(|layer| layer.id == id && memory.areas().is_visible(&layer))
            .unwrap_or_else(|| panic!("no drawn layer {id:?}"))
    })
}

/// The top modal's place in egui's back-to-front layer order.
fn top_modal_rank(egui_ctx: &egui::Context) -> usize {
    let modal = egui_ctx
        .memory(|memory| memory.top_modal_layer())
        .expect("a modal is open");
    layer_rank(egui_ctx, modal.id)
}

/// The Settings modal opened as a member opens it from the rail: a quick click on the
/// gear, then the pointer resting there until the overlay opens. Returns the last
/// frame's nodes.
fn settings_open_with_the_pointer_on_the_rail(
    egui_ctx: &egui::Context,
    app: &mut StudioApp,
) -> Vec<accesskit::Node> {
    let nodes = frame_at(egui_ctx, app, Vec::new(), true, Some(0.9)).nodes;
    let gear = entry(&nodes, SETTINGS_LABEL);
    frame_at(egui_ctx, app, pointer(gear, true), true, Some(1.0));
    frame_at(egui_ctx, app, pointer(gear, false), true, Some(1.02));
    let mut nodes = Vec::new();
    for time in [1.10, 1.20, 1.30, 1.40] {
        nodes = frame_at(egui_ctx, app, Vec::new(), true, Some(time)).nodes;
    }
    assert!(has_text(&nodes, "Threads"), "the settings menu is open");
    assert!(
        app.sidebar.open,
        "the overlay opened under the resting pointer"
    );
    nodes
}

// While a modal is open the opened sidebar is drawn under its dimmed backdrop, as
// the view is: drawn over it, the overlay's entries would act behind a dialog
// waiting for an answer.
#[test]
fn the_opened_sidebar_is_drawn_below_an_open_modal() {
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();
    settings_open_with_the_pointer_on_the_rail(&egui_ctx, &mut app);
    let overlay = layer_rank(&egui_ctx, overlay_id());
    let modal = top_modal_rank(&egui_ctx);
    assert!(
        overlay < modal,
        "the overlay ({overlay}) is ordered below the Settings modal ({modal})"
    );
}

#[test]
fn a_click_on_the_opened_sidebar_under_a_modal_switches_and_opens_nothing() {
    for label in ["B", HELP_LABEL] {
        let Shell { mut app, _dir, .. } = shell();
        let egui_ctx = test_context();
        let nodes = settings_open_with_the_pointer_on_the_rail(&egui_ctx, &mut app);
        let target = nodes
            .iter()
            .find(|node| node.role() == Role::Button && node.label() == Some(label))
            .unwrap_or_else(|| panic!("the overlay's {label} entry"));
        frame_at(&egui_ctx, &mut app, pointer(target, true), true, Some(1.50));
        frame_at(
            &egui_ctx,
            &mut app,
            pointer(target, false),
            true,
            Some(1.52),
        );
        frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(1.60));
        assert_eq!(app.active, 0, "a click on {label} switched no tool");
        assert_eq!(
            app.help_window.showing(),
            None,
            "a click on {label} opened no help"
        );
    }
}

// The About block is a modal the overlay opens itself: drawn above the overlay, which
// stays open under it, and its buttons take clicks.
#[test]
fn the_about_block_opened_from_the_overlay_is_above_it_and_takes_clicks() {
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9)).nodes;
    let nodes = open_sidebar(&egui_ctx, &mut app, entry(&nodes, "A"), 1.0).nodes;
    let logo = entry(&nodes, "4cc");
    frame_at(&egui_ctx, &mut app, pointer(logo, true), true, Some(1.30));
    frame_at(&egui_ctx, &mut app, pointer(logo, false), true, Some(1.32));
    let mut nodes = Vec::new();
    for time in [1.40, 1.50, 1.60] {
        nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(time)).nodes;
    }
    assert!(app.sidebar.about_open, "the logo opened the About block");
    assert!(app.sidebar.open, "the overlay stays open under it");
    let overlay = layer_rank(&egui_ctx, overlay_id());
    let modal = top_modal_rank(&egui_ctx);
    assert!(
        overlay < modal,
        "the About block ({modal}) is ordered above the overlay ({overlay})"
    );
    let copy = entry(&nodes, "Copy");
    frame_at(&egui_ctx, &mut app, pointer(copy, true), true, Some(1.70));
    let out = frame_at(&egui_ctx, &mut app, pointer(copy, false), true, Some(1.72));
    assert!(
        out.platform
            .commands
            .iter()
            .any(|command| matches!(command, egui::OutputCommand::CopyText(_))),
        "the About block's Copy takes the click"
    );
}

// The help and the gear read "(WIP)" until the help and the settings menu are
// complete: painted in the opened sidebar and as the entries' accessible names, on
// the rail as when open. The help window the entry opens carries the same name.
#[test]
fn the_help_and_the_gear_are_marked_wip_as_the_help_window_is() {
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();
    let rail = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9));
    let open = open_sidebar(&egui_ctx, &mut app, entry(&rail.nodes, "A"), 1.0);
    let button = |nodes: &[accesskit::Node], label: &str| {
        nodes
            .iter()
            .any(|node| node.role() == Role::Button && node.label() == Some(label))
    };
    for label in ["Help (WIP)", "Settings (WIP)"] {
        assert_eq!(
            painted_text_count(&open, label),
            1,
            "the opened sidebar paints {label:?}"
        );
        assert!(
            button(&rail.nodes, label) && button(&open.nodes, label),
            "{label:?} is the entry's accessible name, on the rail and open"
        );
    }

    let help = open
        .nodes
        .iter()
        .find(|node| node.role() == Role::Button && node.label() == Some("Help (WIP)"))
        .expect("the overlay's help entry");
    frame_at(&egui_ctx, &mut app, pointer(help, true), true, Some(1.30));
    frame_at(&egui_ctx, &mut app, pointer(help, false), true, Some(1.32));
    let mut nodes = Vec::new();
    for time in [1.40, 1.50, 1.60] {
        nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(time)).nodes;
    }
    assert!(
        nodes
            .iter()
            .any(|node| node.role() == Role::Window && node.label() == Some("Help (WIP)")),
        "the help window opened from the entry is titled \"Help (WIP)\""
    );
}

#[test]
fn the_opened_sidebar_stays_above_the_help_window() {
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.9)).nodes;
    let nodes = open_sidebar(&egui_ctx, &mut app, entry(&nodes, "A"), 1.0).nodes;
    let help = nodes
        .iter()
        .find(|node| node.role() == Role::Button && node.label() == Some(HELP_LABEL))
        .expect("the overlay's Help entry");
    frame_at(&egui_ctx, &mut app, pointer(help, true), true, Some(1.30));
    frame_at(&egui_ctx, &mut app, pointer(help, false), true, Some(1.32));
    for time in [1.40, 1.50, 1.60] {
        frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(time));
    }
    assert!(
        app.help_window.showing().is_some(),
        "the help window opened"
    );
    assert!(app.sidebar.open, "the pointer still rests on the overlay");
    let overlay = layer_rank(&egui_ctx, overlay_id());
    let windows: Vec<usize> = egui_ctx.memory(|memory| {
        memory
            .layer_ids()
            .enumerate()
            .filter(|(_, layer)| {
                layer.order == egui::Order::Middle && memory.areas().is_visible(layer)
            })
            .map(|(rank, _)| rank)
            .collect()
    });
    assert!(
        !windows.is_empty(),
        "the help window is a drawn Middle layer"
    );
    assert!(
        windows.iter().all(|&window| window < overlay),
        "the overlay ({overlay}) is ordered above the help window ({windows:?})"
    );
}

// ST-SIDE-05 is manual: a real PES must run. These cover the poll's mechanics with a
// stubbed probe, so they cite no scenario.

/// The folder `<dir>/running/Pro Evolution Soccer 2019` and its `PES2019.exe` path; the
/// names the running game really uses.
fn running_pes_2019(dir: &std::path::Path) -> (PathBuf, PathBuf) {
    let folder = dir.join("running").join("Pro Evolution Soccer 2019");
    let exe = folder.join("PES2019.exe");
    (folder, exe)
}

#[test]
fn the_poll_switches_to_the_running_version_once_and_offers_fix_pes_path() {
    let dir = test_dir("studio_core_autoswitch");
    fs::create_dir_all(dir.join("data")).unwrap();
    let (running_folder, running_exe) = running_pes_2019(&dir);
    // The action is offered only for a folder that exists on this system: the running
    // install's own, exe included.
    fs::create_dir_all(&running_folder).unwrap();
    fs::write(&running_exe, b"").unwrap();

    // The probe is a `fn`, so the exe path is rebuilt inside it (a closure could not be
    // one): `test_dir` gives the same path.
    fn probe() -> Vec<RunningPes> {
        vec![RunningPes {
            version: PesVersion::Pes19,
            exe: Some(running_pes_2019(&test_dir("studio_core_autoswitch")).1),
        }]
    }

    let (a, calls_a) = stub("a", "A", None);
    let mut settings = test_settings(&dir);
    settings.common.pes_version = PesVersion::Pes21;
    settings.common.pes_folder_path = dir
        .join("installed")
        .join("Pro Evolution Soccer 20**")
        .display()
        .to_string();
    let Shell { mut app, ctx, .. } = shell_with(
        settings,
        vec![a],
        &[],
        Some(dir.join("data")),
        vec![calls_a],
    );
    app.sidebar.probe = probe;
    let egui_ctx = test_context();
    assert_ne!(
        running_exe.parent(),
        Some(ctx.common().pes_folder().as_path()),
        "the running exe sits outside the resolved folder (the action's premise)"
    );

    // No poll before 5 s have passed.
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.0));
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(4.9));
    assert_eq!(
        ctx.common().pes_version,
        PesVersion::Pes21,
        "no poll before the interval"
    );

    // The switch's requests apply in the frame that switches: at t = 5 s the notice is
    // already drawn and the file already saved, no later frame needed.
    let out = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(5.0));
    assert_eq!(
        ctx.common().pes_version,
        PesVersion::Pes19,
        "the poll switched the selection"
    );
    assert!(app.sidebar.auto_switch_used, "the switch is spent");
    let file = dir.join("data").join(crate::settings::SETTINGS_FILE_NAME);
    assert!(
        file.is_file(),
        "the switching frame wrote the settings file"
    );
    let written = Settings::load(&file).unwrap();
    assert_eq!(
        written.common.pes_version,
        PesVersion::Pes19,
        "the switch saved in the switching frame"
    );
    assert!(
        has_text(&out.nodes, "Switched to PES 2019"),
        "the notice shows in the switching frame"
    );

    // The exe's folder differs from the resolved one, so the action is offered; clicking it
    // rewrites pes_folder_path from the running folder with the year back to `20**`.
    let fixed = running_folder.with_file_name("Pro Evolution Soccer 20**");
    click(&egui_ctx, &mut app, entry(&out.nodes, "Fix PES path"));
    assert_eq!(
        std::path::Path::new(&ctx.common().pes_folder_path),
        fixed.as_path(),
        "the ** placeholder is restored"
    );
    frame(&egui_ctx, &mut app, Vec::new(), true);
    let written =
        Settings::load(&dir.join("data").join(crate::settings::SETTINGS_FILE_NAME)).unwrap();
    assert_eq!(written.common.pes_folder_path, fixed.display().to_string());
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert!(
        !has_text(&nodes, "Switched to PES 2019"),
        "the click dismissed the notice"
    );

    // A manual pick back to PES 2021 is not fought at the next polls.
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, combo(&nodes));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "PES 2021"));
    assert_eq!(ctx.common().pes_version, PesVersion::Pes21);
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(15.0));
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(25.0));
    assert_eq!(
        ctx.common().pes_version,
        PesVersion::Pes21,
        "the spent switch never fires again"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_version_change_blocker_defers_the_auto_switch_without_spending_it() {
    fn probe() -> Vec<RunningPes> {
        vec![RunningPes {
            version: PesVersion::Pes19,
            exe: None,
        }]
    }
    let (a, calls_a) = stub("a", "A", Some("a compile is running"));
    let dir = TestDir::new("studio_core_autoswitch_blocked");
    let mut settings = test_settings(dir.path());
    settings.common.pes_version = PesVersion::Pes21;
    let Shell {
        mut app,
        ctx,
        calls,
        ..
    } = shell_with(
        settings,
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    app.sidebar.probe = probe;
    let egui_ctx = test_context();

    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.0));
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(5.0));
    assert_eq!(
        ctx.common().pes_version,
        PesVersion::Pes21,
        "the blocker deferred the switch"
    );
    assert!(
        !app.sidebar.auto_switch_used,
        "a deferred switch is not spent"
    );

    *calls[0].blocker.lock().unwrap() = None;
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(10.0));
    assert_eq!(
        ctx.common().pes_version,
        PesVersion::Pes19,
        "the switch happens once the blocker clears"
    );
}

#[test]
fn the_switch_notice_offers_fix_pes_path_only_when_the_exe_sits_elsewhere() {
    // `test_dir`, not `TestDir`: the probes are `fn` items that recompute the path by
    // name, which a `TestDir`'s counter suffix would break.
    let dir = test_dir("studio_core_autoswitch_notice");
    let egui_ctx = test_context();

    // The probe found no exe path: the notice has no action.
    fn no_exe() -> Vec<RunningPes> {
        vec![RunningPes {
            version: PesVersion::Pes19,
            exe: None,
        }]
    }
    let (a, calls_a) = stub("a", "A", None);
    let mut settings = test_settings(&dir);
    settings.common.pes_version = PesVersion::Pes21;
    let data = TestDir::new("studio_core_autoswitch_notice_data");
    let Shell { mut app, .. } = shell_with(
        settings,
        vec![a],
        &[],
        Some(data.path().to_path_buf()),
        vec![calls_a],
    );
    app.sidebar.probe = no_exe;
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.0));
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(5.0));
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(6.0)).nodes;
    assert!(has_text(&nodes, "Switched to PES 2019"));
    assert!(
        nodes
            .iter()
            .all(|node| node.label() != Some("Fix PES path")),
        "no action without an exe path"
    );

    // The exe's folder is already the resolved one: the notice has no action either.
    fn matching() -> Vec<RunningPes> {
        vec![RunningPes {
            version: PesVersion::Pes19,
            exe: Some(
                test_dir("studio_core_autoswitch_notice")
                    .join("installed")
                    .join("Pro Evolution Soccer 2019")
                    .join("PES2019.exe"),
            ),
        }]
    }
    let installed = dir.join("installed").join("Pro Evolution Soccer 2019");
    fs::create_dir_all(&installed).unwrap();
    fs::write(installed.join("PES2019.exe"), b"").unwrap();
    let (a, calls_a) = stub("a", "A", None);
    let mut settings = test_settings(&dir);
    settings.common.pes_version = PesVersion::Pes21;
    settings.common.pes_folder_path = dir
        .join("installed")
        .join("Pro Evolution Soccer 20**")
        .display()
        .to_string();
    let data = TestDir::new("studio_core_autoswitch_notice_data");
    let Shell { mut app, .. } = shell_with(
        settings,
        vec![a],
        &[],
        Some(data.path().to_path_buf()),
        vec![calls_a],
    );
    app.sidebar.probe = matching;
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.0));
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(5.0));
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(6.0)).nodes;
    assert!(has_text(&nodes, "Switched to PES 2019"));
    assert!(
        nodes
            .iter()
            .all(|node| node.label() != Some("Fix PES path")),
        "no action when the folder already matches"
    );

    // An exe whose folder does not exist on this system (a Windows path the probe read
    // under Wine or Proton): the notice carries no action either.
    fn missing_folder() -> Vec<RunningPes> {
        vec![RunningPes {
            version: PesVersion::Pes19,
            exe: Some(
                test_dir("studio_core_autoswitch_notice")
                    .join("running-missing")
                    .join("Pro Evolution Soccer 2019")
                    .join("PES2019.exe"),
            ),
        }]
    }
    let (a, calls_a) = stub("a", "A", None);
    let mut settings = test_settings(&dir);
    settings.common.pes_version = PesVersion::Pes21;
    let data = TestDir::new("studio_core_autoswitch_notice_data");
    let Shell { mut app, .. } = shell_with(
        settings,
        vec![a],
        &[],
        Some(data.path().to_path_buf()),
        vec![calls_a],
    );
    app.sidebar.probe = missing_folder;
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.0));
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(5.0));
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(6.0)).nodes;
    assert!(has_text(&nodes, "Switched to PES 2019"));
    assert!(
        nodes
            .iter()
            .all(|node| node.label() != Some("Fix PES path")),
        "no action for a folder that does not exist"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_fix_pes_path_click_rewrites_the_year_the_notice_was_raised_for() {
    let dir = test_dir("studio_core_autoswitch_fixpick");
    fs::create_dir_all(dir.join("data")).unwrap();
    let (running_folder, running_exe) = running_pes_2019(&dir);
    fs::create_dir_all(&running_folder).unwrap();
    fs::write(&running_exe, b"").unwrap();

    fn probe() -> Vec<RunningPes> {
        vec![RunningPes {
            version: PesVersion::Pes19,
            exe: Some(running_pes_2019(&test_dir("studio_core_autoswitch_fixpick")).1),
        }]
    }

    let (a, calls_a) = stub("a", "A", None);
    let mut settings = test_settings(&dir);
    settings.common.pes_version = PesVersion::Pes21;
    settings.common.pes_folder_path = dir
        .join("installed")
        .join("Pro Evolution Soccer 20**")
        .display()
        .to_string();
    let Shell { mut app, ctx, .. } = shell_with(
        settings,
        vec![a],
        &[],
        Some(dir.join("data")),
        vec![calls_a],
    );
    app.sidebar.probe = probe;
    let egui_ctx = test_context();

    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.0));
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(5.0));
    let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(6.0)).nodes;
    assert!(has_text(&nodes, "Switched to PES 2019"));
    entry(&nodes, "Fix PES path");

    // A manual pick back to PES 2021 leaves the notice up: the click must still rewrite the
    // running game's year, not the year the selection holds now.
    click(&egui_ctx, &mut app, combo(&nodes));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "PES 2021"));
    assert_eq!(ctx.common().pes_version, PesVersion::Pes21);

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "Fix PES path"));
    assert!(
        std::path::Path::new(&ctx.common().pes_folder_path).ends_with("Pro Evolution Soccer 20**"),
        "the running game's year is rewritten, not the selection's: {}",
        ctx.common().pes_folder_path
    );
    fs::remove_dir_all(&dir).unwrap();
}

// If several PES versions are running, the first detected wins: the probe's first game is
// the one that matters, not the first one that differs.
#[test]
fn the_poll_takes_the_first_detected_game_and_stays_put_when_it_is_the_selection() {
    fn probe() -> Vec<RunningPes> {
        vec![
            RunningPes {
                version: PesVersion::Pes19,
                exe: None,
            },
            RunningPes {
                version: PesVersion::Pes21,
                exe: None,
            },
        ]
    }
    let (a, calls_a) = stub("a", "A", None);
    let dir = TestDir::new("studio_core_autoswitch_first");
    let mut settings = test_settings(dir.path());
    settings.common.pes_version = PesVersion::Pes19;
    let Shell { mut app, ctx, .. } = shell_with(
        settings,
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    app.sidebar.probe = probe;
    let egui_ctx = test_context();

    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.0));
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(5.0));
    assert_eq!(
        ctx.common().pes_version,
        PesVersion::Pes19,
        "the first detected game is the selection: nothing switches"
    );
    assert!(
        !app.sidebar.auto_switch_used,
        "no switch happened: the switch stays unspent"
    );
}

#[test]
fn the_poll_calls_the_probe_every_five_seconds_and_schedules_its_repaint() {
    static PROBE_CALLS: AtomicUsize = AtomicUsize::new(0);
    // The probe reports the selected version itself (the default), so nothing switches and
    // the polling goes on.
    fn probe() -> Vec<RunningPes> {
        PROBE_CALLS.fetch_add(1, Ordering::SeqCst);
        vec![RunningPes {
            version: PesVersion::Pes19,
            exe: None,
        }]
    }

    let (a, calls_a) = stub("a", "A", None);
    let dir = TestDir::new("studio_core_autoswitch_poll");
    let Shell { mut app, .. } = shell_with(
        test_settings(dir.path()),
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    app.sidebar.probe = probe;
    let egui_ctx = test_context();

    // The t = 0 frame arms the first poll for t = 5 s.
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(0.0));
    // Between polls the frame asks for a repaint at the next one: three seconds left (egui
    // trims the request by the predicted frame time, so the bound is a hair under). The
    // t = 1 s frame is not read: the first frame's own request (the window's theme sync) is
    // still settling there.
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(1.0));
    let out = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(2.0));
    let delay = out.repaint_delay.as_secs_f64();
    assert!(
        (2.9..=3.0).contains(&delay),
        "the poll asks for a repaint at the next poll, got {delay} s"
    );

    for t in [5.0, 6.0, 9.9] {
        frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(t));
    }
    assert_eq!(
        PROBE_CALLS.load(Ordering::SeqCst),
        1,
        "the probe ran once, at t = 5"
    );
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(10.0));
    assert_eq!(
        PROBE_CALLS.load(Ordering::SeqCst),
        2,
        "the next poll ran at t = 10"
    );
}

// A fixed-width panel shows no resize edge (the draft's plain edge): the rail is
// `exact_size(RAIL_WIDTH)`, so resting the pointer on its right edge leaves the
// default cursor and registers no splitter.
#[test]
fn the_rails_fixed_width_edge_shows_no_resize_cursor() {
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();

    // The rail's right edge at x = RAIL_WIDTH (inside the grab band a resizable
    // side panel gives its edge), at a height no rail widget sits at. Three
    // frames stay well under the overlay's hover delay.
    let edge = egui::pos2(RAIL_WIDTH, 300.0);
    frame(
        &egui_ctx,
        &mut app,
        vec![egui::Event::PointerMoved(edge)],
        true,
    );
    let out = idle(&egui_ctx, &mut app, 3);
    assert_eq!(
        out.platform.cursor_icon,
        egui::CursorIcon::Default,
        "the fixed-width edge is not a resize handle"
    );
    assert!(
        !out.nodes
            .iter()
            .any(|node| node.role() == Role::Splitter && node.label() == Some("Resize panel")),
        "a fixed-width panel registers no splitter"
    );
}

/// The suite's registry in sidebar order, as the binary lists it.
const REGISTRY: &[PlannedTool] = &[
    PlannedTool {
        id: "team-compiler",
        label: "Team compiler",
        icon: crate::icons::TEAM_COMPILER,
    },
    PlannedTool {
        id: "save-editor",
        label: "Save editor",
        icon: crate::icons::SAVE_EDITOR,
    },
    PlannedTool {
        id: "stadium-compiler",
        label: "Stadium compiler",
        icon: crate::icons::STADIUM_COMPILER,
    },
    PlannedTool {
        id: "export-upgrader",
        label: "Export upgrader",
        icon: crate::icons::EXPORT_UPGRADER,
    },
    PlannedTool {
        id: "music-player",
        label: "Music player",
        icon: crate::icons::MUSIC_PLAYER,
    },
    PlannedTool {
        id: "music-export-editor",
        label: "Music export editor",
        icon: crate::icons::MUSIC_EXPORT_EDITOR,
    },
    PlannedTool {
        id: "match-tracker",
        label: "Match tracker",
        icon: crate::icons::MATCH_TRACKER,
    },
    PlannedTool {
        id: "kit-config-editor",
        label: "Kit config editor",
        icon: crate::icons::KIT_CONFIG_EDITOR,
    },
    PlannedTool {
        id: "refs-arranger",
        label: "Refs arranger",
        icon: crate::icons::REFS_ARRANGER,
    },
    PlannedTool {
        id: "balls-compiler",
        label: "Balls compiler",
        icon: crate::icons::BALLS_COMPILER,
    },
    PlannedTool {
        id: "player-aesthetics-editor",
        label: "Player aesthetics editor",
        icon: crate::icons::PLAYER_AESTHETICS_EDITOR,
    },
    PlannedTool {
        id: "team-creator",
        label: "Team creator",
        icon: crate::icons::TEAM_CREATOR,
    },
    PlannedTool {
        id: "ruleset-editor",
        label: "Ruleset editor",
        icon: crate::icons::RULESET_EDITOR,
    },
    PlannedTool {
        id: "db-generator",
        label: "DB generator",
        icon: crate::icons::DB_GENERATOR,
    },
];

/// A shell with the Save editor registered and the rest of the registry drawn as
/// planned entries: fourteen entries, as the build shows them.
fn draft_window_shell() -> Shell {
    let (save_editor, calls) = stub("save-editor", "Save editor", None);
    let dir = TestDir::new("studio_core_sidebar_draft_window");
    let mut shell = shell_with(
        test_settings(dir.path()),
        vec![save_editor],
        REGISTRY,
        Some(dir.path().to_path_buf()),
        vec![calls],
    );
    shell._dir = Some(dir);
    shell
}

/// One frame of `app` at the draft's 1280 by 780 window, with `events` as its input
/// and `time` as its clock.
fn draft_window_frame(
    egui_ctx: &egui::Context,
    app: &mut StudioApp,
    events: Vec<egui::Event>,
    time: Option<f64>,
) -> FrameOutput {
    let mut input = raw_input(events, true, time, None);
    input.screen_rect = Some(egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(1280.0, 780.0),
    ));
    run_frame(egui_ctx, app, input)
}

/// The rail of `draft_window_shell` at the draft's window, after three frames, so
/// every layout pass has settled.
fn rail_at_draft_window() -> FrameOutput {
    let Shell { mut app, _dir, .. } = draft_window_shell();
    let egui_ctx = test_context();
    let mut out = None;
    for _ in 0..3 {
        out = Some(draft_window_frame(&egui_ctx, &mut app, Vec::new(), None));
    }
    out.expect("three frames ran")
}

/// The rail and the opened sidebar of `draft_window_shell` at the draft's window, the
/// pointer resting on `node_text`'s widget: the rail's frame before the hover delay
/// runs out (the widget hovered), then the open one after it.
fn rail_and_open_at_draft_window(node_text: &str) -> (FrameOutput, FrameOutput) {
    let Shell { mut app, _dir, .. } = draft_window_shell();
    let egui_ctx = test_context();
    let settled = draft_window_frame(&egui_ctx, &mut app, Vec::new(), Some(0.9));
    let node = settled
        .nodes
        .iter()
        .find(|node| node.label() == Some(node_text))
        .unwrap_or_else(|| panic!("no widget labeled {node_text:?}"));
    let rail = draft_window_frame(&egui_ctx, &mut app, hover(node), Some(1.0));
    let open = draft_window_frame(&egui_ctx, &mut app, Vec::new(), Some(1.2));
    assert_eq!(
        (
            painted_text_count(&rail, HELP_LABEL),
            painted_text_count(&open, HELP_LABEL)
        ),
        (0, 1),
        "the first frame is the rail, the second the opened sidebar with its labels"
    );
    (rail, open)
}

/// The bounds of the rail entry showing `text`: a button's text is its label, a
/// planned entry's its value.
fn rail_bounds(nodes: &[accesskit::Node], text: &str) -> accesskit::Rect {
    nodes
        .iter()
        .find(|node| node.label() == Some(text) || node.value() == Some(text))
        .and_then(|node| node.bounds())
        .unwrap_or_else(|| panic!("no rail entry shows {text:?}"))
}

/// The horizontal lines drawn on the rail, top to bottom: the frame's line segments
/// that start and end left of the rail's edge stroke.
fn rail_lines(out: &FrameOutput) -> Vec<([egui::Pos2; 2], egui::Stroke)> {
    let mut lines: Vec<_> = out
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke }
                if points[0].y == points[1].y && points[1].x < RAIL_WIDTH - 1.0 =>
            {
                Some((*points, *stroke))
            }
            _ => None,
        })
        .collect();
    lines.sort_by(|a, b| a.0[0].y.total_cmp(&b.0[0].y));
    lines
}

/// Asserts `actual` is within one point of the draft's `expected`.
fn assert_at_draft(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= 1.0,
        "{what} at {actual}, the draft's at {expected}"
    );
}

// The rail's top block as the draft lays it out (`.rail` padding 6 and gap 3):
// the 32-point logo at 6, the selector under it at 41 spanning the rail's inner
// width, and the line under the selector on the draft's row 66.
#[test]
fn the_rails_logo_selector_and_line_sit_at_the_drafts_rows() {
    let out = rail_at_draft_window();
    let logo = rail_bounds(&out.nodes, "4cc");
    assert_at_draft(logo.y0, 6.0, "the logo's top");
    assert_at_draft(logo.y1, 38.0, "the logo's bottom");
    let selector = combo(&out.nodes).bounds().unwrap();
    assert_at_draft(selector.y0, 41.0, "the selector's top");
    assert_at_draft(selector.x0, 5.5, "the selector's left");
    assert_at_draft(selector.x1, 49.5, "the selector's right");
    let lines = rail_lines(&out);
    let ([start, end], _) = lines.first().expect("a line is drawn under the selector");
    assert_at_draft(f64::from(start.y), 66.5, "the line under the selector");
    assert_at_draft(f64::from(start.x), 6.0, "the line's left end");
    assert_at_draft(f64::from(end.x), 49.0, "the line's right end");
}

/// The selector's button: its node's bounds.
fn combo_rect(nodes: &[accesskit::Node]) -> egui::Rect {
    let bounds = combo(nodes).bounds().expect("the selector has bounds");
    egui::Rect::from_min_max(
        egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
        egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
    )
}

/// How many triangles the frame paints inside `rect`: egui's combo arrow is a
/// filled triangle, a closed three-point `Shape::Path`.
fn triangles_in(out: &FrameOutput, rect: egui::Rect) -> usize {
    out.shapes
        .iter()
        .filter(|clipped| {
            matches!(&clipped.shape, egui::Shape::Path(path)
                if path.closed
                    && path.points.len() == 3
                    && path.points.iter().all(|point| rect.contains(*point)))
        })
        .count()
}

// The draft's `#sidebar select`: on the rail the selector shows its year alone,
// centred on the button, with no arrow. Opened, its full name starts at the
// button's left, past the button's padding, and the arrow follows it.
#[test]
fn the_rails_selector_centres_its_year_without_an_arrow_and_the_opened_one_keeps_both() {
    let (rail, open) = rail_and_open_at_draft_window("Save editor");
    let version = Settings::default().common.pes_version;

    let button = combo_rect(&rail.nodes);
    assert_eq!(
        triangles_in(&rail, button),
        0,
        "the rail's selector paints no arrow"
    );
    let year = text_shape_in(&rail, &version_label(version, true), button)
        .expect("the rail's selector paints its year");
    let year_centre = year.pos.x + year.galley.size().x / 2.0;
    assert!(
        (year_centre - button.center().x).abs() <= 0.5,
        "the year is centred on the button ({year_centre} vs {})",
        button.center().x
    );

    let button = combo_rect(&open.nodes);
    assert_eq!(
        triangles_in(&open, button),
        1,
        "the opened selector paints its arrow"
    );
    let name = text_shape_in(&open, &version_label(version, false), button)
        .expect("the opened selector paints its full name");
    let padding = test_context().global_style().spacing.button_padding.x;
    assert!(
        (name.pos.x - (button.min.x + padding)).abs() <= 0.5,
        "the full name starts at the button's left, past its padding ({} vs {})",
        name.pos.x,
        button.min.x + padding
    );
}

// The entries are the draft's 30-point rows 3 apart (`.rail` gap 3, egui's item
// spacing): the first at 74, each next one 3 under the one above it, the fourteenth
// thirteen pitches of 33 below the first.
#[test]
fn the_rails_entries_are_3_points_apart() {
    let out = rail_at_draft_window();
    assert_at_draft(
        rail_bounds(&out.nodes, "Save editor").y0,
        74.0,
        "the first entry's top",
    );
    let entries: Vec<&str> = std::iter::once("Save editor")
        .chain(
            REGISTRY
                .iter()
                .filter(|entry| entry.id != "save-editor")
                .map(|entry| entry.label),
        )
        .collect();
    assert_eq!(entries.len(), 14, "premise: the draft's fourteen entries");
    for pair in entries.windows(2) {
        let (above, below) = (
            rail_bounds(&out.nodes, pair[0]),
            rail_bounds(&out.nodes, pair[1]),
        );
        assert!(
            (below.y0 - above.y1 - 3.0).abs() <= 0.5,
            "{} stands 3 under {} ({above:?} vs {below:?})",
            pair[1],
            pair[0]
        );
    }
    assert_at_draft(
        rail_bounds(&out.nodes, "DB generator").y0,
        503.0,
        "the last entry's top",
    );
}

// The draft separates the help and the gear from the entries with the line it
// draws under the selector: the same stroke over the same span, on row 680.
#[test]
fn a_line_above_the_help_matches_the_line_under_the_selector() {
    let out = rail_at_draft_window();
    let lines = rail_lines(&out);
    assert_eq!(lines.len(), 2, "two lines on the rail: {lines:?}");
    let (under_selector, above_help) = (lines[0], lines[1]);
    assert_at_draft(
        f64::from(above_help.0[0].y),
        680.5,
        "the line above the help",
    );
    assert_eq!(
        (above_help.0[0].x, above_help.0[1].x, above_help.1),
        (
            under_selector.0[0].x,
            under_selector.0[1].x,
            under_selector.1
        ),
        "the line above the help has the span and stroke of the one under the selector"
    );
}

// The help and the gear rows sit at the rail's foot as the draft's tail: the gear
// 6 above the rail's bottom at 756 (`.rail` padding 6), the help one gap above it.
#[test]
fn the_help_and_the_gear_sit_at_the_drafts_rows() {
    let out = rail_at_draft_window();
    assert_at_draft(
        rail_bounds(&out.nodes, HELP_LABEL).y0,
        688.0,
        "the help's top",
    );
    assert_at_draft(
        rail_bounds(&out.nodes, SETTINGS_LABEL).y0,
        720.0,
        "the gear's top",
    );
}

// The draft's entries carry a 3-point left border, transparent but on the active
// one, so a glyph centres in the rest of the row: at x 29 on the rail, and at the
// same x in the opened sidebar, where it starts 3 + 8 past the row's left.
#[test]
fn every_rail_icon_centres_right_of_the_accent_bar_on_the_rail_and_open() {
    let mut glyphs: Vec<&str> = REGISTRY
        .iter()
        .filter(|entry| entry.id != "save-editor")
        .map(|entry| entry.icon)
        .collect();
    // The registered stub's icon, the help's and the gear's.
    glyphs.extend(["i", crate::icons::QUESTION, crate::icons::GEAR]);
    let (rail, open) = rail_and_open_at_draft_window("Save editor");
    let left_strip = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(RAIL_WIDTH, 780.0));
    for (state, out) in [("rail", &rail), ("open", &open)] {
        for glyph in &glyphs {
            let shape = text_shape_in(out, glyph, left_strip)
                .unwrap_or_else(|| panic!("{glyph:?} is painted ({state})"));
            let centre = shape.pos.x + shape.galley.size().x / 2.0;
            assert_at_draft(
                f64::from(centre),
                29.0,
                &format!("the {glyph:?} glyph's centre ({state})"),
            );
        }
    }
}

// The opened sidebar's rows as the draft's `#sb-open .tool` lays them out: every
// label starts at x 47 (the row's left at 6, its 3-point border, its padding 8,
// the 24-point icon and the gap 6), and the layer is the draft's 190.6 wide: its
// widest label, "Player aesthetics editor", with 6 of the row after it, the
// frame's padding 6 on both sides and the 1-point line.
#[test]
fn the_opened_sidebars_labels_and_width_match_the_draft() {
    let Shell { mut app, _dir, .. } = draft_window_shell();
    let egui_ctx = test_context();
    let settled = draft_window_frame(&egui_ctx, &mut app, Vec::new(), Some(0.9));
    let hovered = hover(entry(&settled.nodes, "Save editor"));
    draft_window_frame(&egui_ctx, &mut app, hovered, Some(1.0));
    let open = draft_window_frame(&egui_ctx, &mut app, Vec::new(), Some(1.2));
    assert_eq!(
        painted_text_count(&open, HELP_LABEL),
        1,
        "premise: the sidebar is open"
    );

    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 780.0));
    let labels = REGISTRY
        .iter()
        .map(|entry| entry.label)
        .chain([HELP_LABEL, SETTINGS_LABEL]);
    for label in labels {
        let shape = text_shape_in(&open, label, screen)
            .unwrap_or_else(|| panic!("the opened sidebar paints {label:?}"));
        assert_at_draft(
            f64::from(shape.pos.x),
            47.0,
            &format!("the {label:?} label's left"),
        );
    }

    let layer = egui_ctx
        .memory(|m| m.area_rect(overlay_id()))
        .expect("the open overlay has a rect");
    assert_at_draft(f64::from(layer.width()), 190.63, "the opened layer's width");
}

// The logo is the draft's `.logo` 32-point square: it alone senses and fills on
// hover, centred in the icon cell on the rail and at the same place open (the
// draft's 11.5 in both), so the hover that opens the sidebar moves nothing.
#[test]
fn the_logo_is_a_32_point_square_on_the_rail_and_open() {
    let (rail, open) = rail_and_open_at_draft_window("4cc");
    assert_eq!(
        rail_bounds(&open.nodes, "4cc").x0,
        rail_bounds(&rail.nodes, "4cc").x0,
        "the logo keeps its place when the sidebar opens"
    );
    for (state, out, draft_left) in [("rail", &rail, 11.5), ("open", &open, 11.5)] {
        let logo = rail_bounds(&out.nodes, "4cc");
        assert_at_draft(logo.x0, draft_left, &format!("the logo's left ({state})"));
        assert_at_draft(
            logo.x1 - logo.x0,
            32.0,
            &format!("the logo's width ({state})"),
        );
        assert!(
            filled_at(out, logo),
            "the hovered logo fills its square ({state})"
        );
    }
}

/// The ink of `shape`'s painted glyphs: each glyph's bitmap where the
/// tessellator places it (the galley's row, the glyph's baseline position and
/// its bitmap's offset), glyphs that paint nothing skipped.
fn ink(shape: &egui::epaint::TextShape) -> egui::Rect {
    let mut rect = egui::Rect::NOTHING;
    for row in &shape.galley.rows {
        for glyph in &row.row.glyphs {
            if glyph.uv_rect.is_nothing() {
                continue;
            }
            let min = shape.pos + row.pos.to_vec2() + glyph.pos.to_vec2() + glyph.uv_rect.offset;
            rect = rect.union(egui::Rect::from_min_size(min, glyph.uv_rect.size));
        }
    }
    rect
}

/// The logo's square and the frame's `Shape::Text` drawing `glyph`, the one
/// the logo paints: the screen holds no other.
fn logo_glyph<'a>(
    out: &'a FrameOutput,
    glyph: &str,
    state: &str,
) -> (egui::Rect, &'a egui::epaint::TextShape) {
    let logo = rail_bounds(&out.nodes, "4cc");
    let square = egui::Rect::from_min_max(
        egui::pos2(logo.x0 as f32, logo.y0 as f32),
        egui::pos2(logo.x1 as f32, logo.y1 as f32),
    );
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 780.0));
    let shape = text_shape_in(out, glyph, screen)
        .unwrap_or_else(|| panic!("the logo paints {glyph:?} ({state})"));
    (square, shape)
}

// The logo is the 4cc clover: the symbols font's ring (U+F100) and leaves
// (U+F101) at 28 points, the clover's diameter, each in its own green, both
// galleys centred on the logo's square, on the rail and open.
#[test]
fn the_logo_draws_the_clover_in_its_two_greens_centred_on_its_square() {
    let (rail, open) = rail_and_open_at_draft_window("4cc");
    for (state, out) in [("rail", &rail), ("open", &open)] {
        for (glyph, color) in [
            ("\u{F100}", egui::Color32::from_rgb(0x12, 0x96, 0x48)),
            ("\u{F101}", egui::Color32::from_rgb(0x8c, 0xc6, 0x42)),
        ] {
            let (square, shape) = logo_glyph(out, glyph, state);
            let format = &shape.galley.job.sections[0].format;
            assert_eq!(
                (&format.font_id, format.color),
                (&egui::FontId::proportional(28.0), color),
                "{glyph:?}'s font and colour ({state})"
            );
            let galley = egui::Rect::from_min_size(shape.pos, shape.galley.size());
            assert!(
                (galley.center() - square.center()).length() <= 0.5,
                "{glyph:?}'s galley {galley:?} is centred on the logo's square {square:?} \
                 ({state})"
            );
        }
    }
}

// The clover's glyphs are the symbols font's, not the replacement box a font
// draws for a codepoint it lacks: the ring's ink is the clover's 28-point
// diameter, the leaves' the 16.9 points they span inside it, which their bitmap
// rounds out to 18 whole pixels, and both inks are centred on the logo's square.
// The glyphs are the ones the logo draws, so a codepoint the font lacks fails
// here; every glyph's mismatch is listed, so one cannot hide another's.
#[test]
fn the_logos_clover_inks_28_points_across_centred_on_its_square() {
    let (rail, open) = rail_and_open_at_draft_window("4cc");
    let mut mismatches = Vec::new();
    for (state, out) in [("rail", &rail), ("open", &open)] {
        for (glyph, across) in [(CLOVER_RING, 28.0), (CLOVER_LEAVES, 18.0)] {
            let (square, shape) = logo_glyph(out, glyph, state);
            let ink = ink(shape);
            if (ink.width() - across).abs() > 1.0 || (ink.height() - across).abs() > 1.0 {
                mismatches.push(format!(
                    "{glyph:?}'s ink {ink:?} is not {across} points across ({state})"
                ));
            }
            if (ink.center() - square.center()).length() > 0.5 {
                mismatches.push(format!(
                    "{glyph:?}'s ink {ink:?} is not centred on the logo's square {square:?} \
                     ({state})"
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
