use eframe::App;
use egui::ViewportCommand;

use super::*;
use crate::settings::{SETTINGS_FILE_NAME, Settings, WindowPlace};
use crate::shell::StudioApp;
use crate::shell::test_harness::*;

/// The borders and title bar `monitor_frame`'s outer rect adds over the
/// inner one: the scenario's 16 points wide and 39 tall.
const DECORATIONS: egui::Vec2 = egui::vec2(16.0, 39.0);

/// One harness frame whose root `ViewportInfo` reports `monitor` and an
/// outer rect `DECORATIONS` bigger than the inner one, the window created
/// at `DEFAULT_SIZE`.
fn monitor_frame(
    ctx: &egui::Context,
    app: &mut StudioApp,
    monitor: Option<egui::Vec2>,
) -> FrameOutput {
    let mut input = raw_input(Vec::new(), true, None, None);
    input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, DEFAULT_SIZE));
    let info = input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .expect("`RawInput::default` carries the root viewport");
    info.monitor_size = monitor;
    info.inner_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, DEFAULT_SIZE));
    info.outer_rect = Some(egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        DEFAULT_SIZE + DECORATIONS,
    ));
    run_frame(ctx, app, input)
}

/// The frame's commands to the window's place: size, position and maximize,
/// the title's `Title` not.
fn fitting(output: &FrameOutput) -> Vec<ViewportCommand> {
    output
        .commands
        .iter()
        .filter(|command| {
            matches!(
                command,
                ViewportCommand::InnerSize(_)
                    | ViewportCommand::OuterPosition(_)
                    | ViewportCommand::Maximized(_)
            )
        })
        .cloned()
        .collect()
}

/// One harness frame as egui-winit builds it for a window `size_px`
/// native pixels big on a monitor at `native_ppp`: `inner_rect` and
/// `screen_rect` are the pixels over the pixels-per-point of the
/// context's current zoom (`update_viewport_info`, `take_egui_input`),
/// `native_pixels_per_point` the monitor's raw scale factor.
fn window_frame(
    ctx: &egui::Context,
    app: &mut StudioApp,
    size_px: egui::Vec2,
    native_ppp: f32,
) -> FrameOutput {
    run_frame(ctx, app, window_input(ctx, size_px, native_ppp))
}

/// `window_frame`'s input, for a test that runs the passes itself.
fn window_input(ctx: &egui::Context, size_px: egui::Vec2, native_ppp: f32) -> egui::RawInput {
    let mut input = raw_input(Vec::new(), true, None, None);
    let size_pt = size_px / (ctx.zoom_factor() * native_ppp);
    input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size_pt));
    let info = input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .expect("`RawInput::default` carries the root viewport");
    info.native_pixels_per_point = Some(native_ppp);
    info.inner_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size_pt));
    input
}

/// One harness frame for a window whose outer top-left sits at `outer_min`
/// native pixels, `size_px` native pixels big on a monitor at `native`,
/// `maximized` reported or not: the rects are pixel rects divided by the
/// pixels per point of the context's current zoom, as egui-winit builds
/// them, and `DECORATIONS` and the (8, 31) inset are pixels here.
fn placed_frame(
    ctx: &egui::Context,
    app: &mut StudioApp,
    outer_min: egui::Pos2,
    size_px: egui::Vec2,
    native: f32,
    maximized: bool,
) -> FrameOutput {
    let mut input = raw_input(Vec::new(), true, None, None);
    let ppp = ctx.zoom_factor() * native;
    input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size_px / ppp));
    let info = input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .expect("`RawInput::default` carries the root viewport");
    info.native_pixels_per_point = Some(native);
    info.outer_rect = Some(egui::Rect::from_min_size(outer_min, size_px + DECORATIONS) / ppp);
    info.inner_rect =
        Some(egui::Rect::from_min_size(outer_min + egui::vec2(8.0, 31.0), size_px) / ppp);
    info.maximized = Some(maximized);
    run_frame(ctx, app, input)
}

/// One harness frame for a minimized window, as egui-winit reports one on
/// Windows: minimized, not maximized (`IsZoomed` is false while minimized),
/// and no rects.
fn minimized_frame(ctx: &egui::Context, app: &mut StudioApp) -> FrameOutput {
    let mut input = raw_input(Vec::new(), true, None, None);
    let info = input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .expect("`RawInput::default` carries the root viewport");
    info.native_pixels_per_point = Some(1.0);
    info.minimized = Some(true);
    info.maximized = Some(false);
    run_frame(ctx, app, input)
}

#[test]
fn place_of_reads_pixels_points_and_scale_from_a_normal_frame() {
    let mut viewport = egui::ViewportInfo {
        outer_rect: Some(egui::Rect::from_min_size(
            egui::pos2(100.0, 50.0),
            egui::vec2(1116.0, 739.0),
        )),
        inner_rect: Some(egui::Rect::from_min_size(
            egui::pos2(108.0, 81.0),
            egui::vec2(1100.0, 700.0),
        )),
        native_pixels_per_point: Some(1.0),
        ..Default::default()
    };
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1100.0, 700.0));
    assert_eq!(
        <[f32; 6]>::from(place_of(&viewport, Some(screen), 1.0).unwrap()),
        [100.0, 50.0, 1100.0, 700.0, 1.0, 0.0],
    );

    // The screen's pixels are whole: a fractional position is rounded.
    viewport.outer_rect = Some(egui::Rect::from_min_size(
        egui::pos2(10.51, 5.34),
        egui::vec2(1116.0, 739.0),
    ));
    viewport.inner_rect = Some(egui::Rect::from_min_size(
        egui::pos2(18.51, 36.34),
        egui::vec2(1100.0, 700.0),
    ));
    assert_eq!(
        <[f32; 6]>::from(place_of(&viewport, Some(screen), 1.0).unwrap()),
        [11.0, 5.0, 1100.0, 700.0, 1.0, 0.0],
    );

    // The same window at zoom 0.5: the rects are in the zoomed points and
    // the record comes out identical.
    viewport.outer_rect = Some(egui::Rect::from_min_size(
        egui::pos2(200.0, 100.0),
        egui::vec2(2232.0, 1478.0),
    ));
    viewport.inner_rect = Some(egui::Rect::from_min_size(
        egui::pos2(216.0, 162.0),
        egui::vec2(2200.0, 1400.0),
    ));
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(2200.0, 1400.0));
    assert_eq!(
        <[f32; 6]>::from(place_of(&viewport, Some(screen), 0.5).unwrap()),
        [100.0, 50.0, 1100.0, 700.0, 1.0, 0.0],
    );

    // A 200% display: the points are half the pixels, the scale is kept.
    viewport.native_pixels_per_point = Some(2.0);
    viewport.outer_rect = Some(egui::Rect::from_min_size(
        egui::pos2(50.0, 25.0),
        egui::vec2(566.0, 389.0),
    ));
    viewport.inner_rect = Some(egui::Rect::from_min_size(
        egui::pos2(58.0, 56.0),
        egui::vec2(550.0, 350.0),
    ));
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(550.0, 350.0));
    assert_eq!(
        <[f32; 6]>::from(place_of(&viewport, Some(screen), 1.0).unwrap()),
        [100.0, 50.0, 550.0, 350.0, 2.0, 0.0],
    );

    // A 150% display, its rects built as egui-winit builds them, a pixel
    // rect divided by the pixels per point: the inner size comes out an ULP
    // away from the screen rect's, and the frame still records.
    viewport.native_pixels_per_point = Some(1.5);
    viewport.outer_rect =
        Some(egui::Rect::from_min_size(egui::pos2(2992.0, 50.0), egui::vec2(1116.0, 739.0)) / 1.5);
    let inner =
        egui::Rect::from_min_size(egui::pos2(3000.0, 81.0), egui::vec2(1100.0, 700.0)) / 1.5;
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1100.0, 700.0) / 1.5);
    viewport.inner_rect = Some(inner);
    assert_ne!(inner.size(), screen.size(), "the premise: an ULP apart");
    assert_eq!(
        place_of(&viewport, Some(screen), 1.0).map(<[f32; 6]>::from),
        Some([2992.0, 50.0, 1100.0 / 1.5, 700.0 / 1.5, 1.5, 0.0]),
    );

    // The frames that report no normal place: maximized or fullscreen, and
    // a rect, the scale or the screen rect missing.
    viewport.native_pixels_per_point = Some(1.0);
    viewport.outer_rect = Some(egui::Rect::from_min_size(
        egui::pos2(100.0, 50.0),
        egui::vec2(1116.0, 739.0),
    ));
    viewport.inner_rect = Some(egui::Rect::from_min_size(
        egui::pos2(108.0, 81.0),
        egui::vec2(1100.0, 700.0),
    ));
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1100.0, 700.0));
    viewport.maximized = Some(true);
    assert_eq!(place_of(&viewport, Some(screen), 1.0), None);
    viewport.maximized = None;
    viewport.fullscreen = Some(true);
    assert_eq!(place_of(&viewport, Some(screen), 1.0), None);
    viewport.fullscreen = None;
    viewport.outer_rect = None;
    assert_eq!(place_of(&viewport, Some(screen), 1.0), None);
    viewport.outer_rect = Some(egui::Rect::from_min_size(
        egui::pos2(100.0, 50.0),
        egui::vec2(1116.0, 739.0),
    ));
    viewport.inner_rect = None;
    assert_eq!(place_of(&viewport, Some(screen), 1.0), None);
    viewport.inner_rect = Some(egui::Rect::from_min_size(
        egui::pos2(108.0, 81.0),
        egui::vec2(1100.0, 700.0),
    ));
    viewport.native_pixels_per_point = None;
    assert_eq!(place_of(&viewport, Some(screen), 1.0), None);
    viewport.native_pixels_per_point = Some(1.0);
    assert_eq!(place_of(&viewport, None, 1.0), None);

    // The zoom-landing frame: egui rescales `screen_rect` alone at
    // `begin_pass`, so the two rects differ in size there.
    let landing = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1375.0, 875.0));
    assert_eq!(place_of(&viewport, Some(landing), 1.0), None);
}

#[test]
fn place_of_compares_the_rects_in_pixels_at_the_scale_times_the_zoom() {
    // A 1024-point-wide window on a 200% display, at the zoom 0.8 the width
    // rule gives it: 1.6 pixels a point, the rects in the zoomed points.
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 875.0));
    let viewport = |inner_width: f32| egui::ViewportInfo {
        outer_rect: Some(egui::Rect::from_min_size(
            egui::pos2(63.0, 31.4),
            egui::vec2(1290.0, 900.0),
        )),
        inner_rect: Some(egui::Rect::from_min_size(
            egui::pos2(68.0, 50.9),
            egui::vec2(inner_width, 875.0),
        )),
        native_pixels_per_point: Some(2.0),
        ..Default::default()
    };
    // 0.4 points wider is 0.64 pixels: the landing frame's difference.
    assert_eq!(place_of(&viewport(1280.4), Some(screen), 0.8), None);
    // 0.25 points wider is 0.4 pixels, a rounding error: the frame records.
    // x = 63 × 1.6 = 100.8, rounded to 101; y = 31.4 × 1.6 = 50.24, to 50;
    // the size at zoom 1 is 1280 × 0.8 = 1024 by 875 × 0.8 = 700.
    assert_eq!(
        place_of(&viewport(1280.25), Some(screen), 0.8).map(<[f32; 6]>::from),
        Some([101.0, 50.0, 1024.0, 700.0, 2.0, 0.0]),
    );
}

#[test]
fn the_title_bar_check_overlaps_a_monitor_with_a_positive_area() {
    let at_scale = |rect: egui::Rect, scale: f32| Monitor {
        rect,
        scale,
        primary: rect.min == egui::Pos2::ZERO,
    };
    let primary = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));
    let place = |x: f32, y: f32| WindowPlace {
        x,
        y,
        width: 1100.0,
        height: 700.0,
        scale: 1.0,
        maximized: false,
    };
    assert!(on_a_monitor(&place(100.0, 50.0), &[at_scale(primary, 1.0)]));
    assert!(!on_a_monitor(
        &place(2500.0, 50.0),
        &[at_scale(primary, 1.0)]
    ));
    let monitors = [
        at_scale(
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(3840.0, 2160.0)),
            1.0,
        ),
        at_scale(
            egui::Rect::from_min_size(egui::pos2(-1920.0, 328.0), egui::vec2(1920.0, 1080.0)),
            1.0,
        ),
    ];
    assert!(on_a_monitor(&place(-1800.0, 400.0), &monitors));
    // The bar's left 120 pixels over the bottom 10 still count.
    assert!(on_a_monitor(
        &place(1800.0, 1070.0),
        &[at_scale(primary, 1.0)]
    ));
    // A bar starting on the monitor's right edge does not.
    assert!(!on_a_monitor(
        &place(1920.0, 50.0),
        &[at_scale(primary, 1.0)]
    ));
    // The bar's lower 12 pixels reach into the monitor from above.
    assert!(on_a_monitor(
        &place(100.0, -20.0),
        &[at_scale(primary, 1.0)]
    ));
    // The bar ends 8 pixels above the monitor, although the window below it
    // reaches in.
    assert!(!on_a_monitor(
        &place(100.0, -40.0),
        &[at_scale(primary, 1.0)]
    ));
    // On a 200% monitor the 1100-point bar is 2200 pixels: it reaches
    // x = 100, 100 pixels past the monitor's left edge.
    let scaled = WindowPlace {
        x: -2100.0,
        y: 50.0,
        width: 1100.0,
        height: 700.0,
        scale: 2.0,
        maximized: false,
    };
    let four_k = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(3840.0, 2160.0));
    assert!(on_a_monitor(&scaled, &[at_scale(four_k, 2.0)]));
    // Each monitor's own scale sizes the bar tested against it: a place
    // recorded on a 200% monitor straddling the 100% primary stands, that
    // monitor unplugged, at its points over the primary's scale, so its
    // bar is -2000..-900 pixels, on no monitor.
    let straddler = WindowPlace {
        x: -2000.0,
        y: 50.0,
        width: 1100.0,
        height: 700.0,
        scale: 2.0,
        maximized: false,
    };
    let left_200 = egui::Rect::from_min_size(egui::pos2(-3840.0, 0.0), egui::vec2(3840.0, 2160.0));
    assert!(!on_a_monitor(&straddler, &[at_scale(primary, 1.0)]));
    // With the 200% monitor still up, the bar -2000..200 lies on it.
    assert!(on_a_monitor(
        &straddler,
        &[at_scale(primary, 1.0), at_scale(left_200, 2.0)]
    ));
    // The monitor's scale sizes the bar's height too: at 200% it is 64
    // pixels, so from y = -40 it reaches 24 pixels in (32 would end at
    // -8, off the monitor), and from y = -70 it still ends above it.
    assert!(on_a_monitor(
        &place(100.0, -40.0),
        &[at_scale(primary, 2.0)]
    ));
    assert!(!on_a_monitor(
        &place(100.0, -70.0),
        &[at_scale(primary, 2.0)]
    ));
    assert!(
        on_a_monitor(&place(2500.0, 50.0), &[]),
        "an empty list trusts the place"
    );

    assert_eq!(
        place_or_fit(place(100.0, 50.0), &[at_scale(primary, 1.0)]),
        Opening::Place(place(100.0, 50.0)),
    );
    assert_eq!(
        place_or_fit(place(2500.0, 50.0), &[at_scale(primary, 1.0)]),
        Opening::Fit
    );
    assert_eq!(
        place_or_fit(straddler, &[at_scale(primary, 1.0)]),
        Opening::Fit
    );
    assert_eq!(
        place_or_fit(
            straddler,
            &[at_scale(primary, 1.0), at_scale(left_200, 2.0)]
        ),
        Opening::Place(straddler),
    );
    assert_eq!(
        place_or_fit(place(2500.0, 50.0), &[]),
        Opening::Place(place(2500.0, 50.0)),
    );
}

#[test]
fn the_default_size_holds_a_monitor_that_fits_it_and_fits_each_side_on_a_smaller_one() {
    assert_eq!(
        fitted_size(egui::vec2(1920.0, 1080.0), DECORATIONS),
        DEFAULT_SIZE
    );
    assert_eq!(
        fitted_size(egui::vec2(2000.0, 2000.0), DECORATIONS),
        DEFAULT_SIZE
    );
    // A square monitor holds the default too. Height-fitted on a shorter
    // monitor: the width keeps 1280 wherever the decorated window fits.
    assert_eq!(
        fitted_size(egui::vec2(1500.0, 1500.0), DECORATIONS),
        DEFAULT_SIZE
    );
    assert_eq!(
        fitted_size(egui::vec2(1366.0, 768.0), DECORATIONS),
        egui::vec2(1280.0, 691.0)
    );
    // A 1080p monitor at 125% scaling: 864 points tall, so 780 is within
    // the monitor yet past 90% of it.
    assert_eq!(
        fitted_size(egui::vec2(1536.0, 864.0), DECORATIONS),
        egui::vec2(1280.0, 778.0)
    );
    assert_eq!(
        fitted_size(egui::vec2(1280.0, 720.0), DECORATIONS),
        egui::vec2(1264.0, 648.0)
    );
    // Width-limited: the window with its 16-point borders is wider than
    // the monitor, so the width is what is left over them.
    assert_eq!(
        fitted_size(egui::vec2(1200.0, 2000.0), DECORATIONS),
        egui::vec2(1184.0, 780.0)
    );
}

#[test]
fn the_zoom_is_the_width_over_the_design_width_and_one_at_or_over_it() {
    assert_eq!(width_zoom(960.0), 0.75);
    assert_eq!(width_zoom(1024.0), 0.8);
    assert_eq!(width_zoom(1280.0), 1.0);
    assert_eq!(width_zoom(1600.0), 1.0);
}

#[test]
fn the_window_is_created_at_the_default_size_and_centered() {
    let options = native_options(&Opening::Fit);
    assert_eq!(options.viewport.inner_size, Some(DEFAULT_SIZE));
    assert!(options.centered, "created centered on the monitor");
}

// ST-WIN-01 (its last clause): the options carry a minimum inner size,
// under the smallest default the fit produces so the fit never meets it.
#[test]
fn the_options_stop_a_members_resize_at_the_minimum() {
    let options = native_options(&Opening::Fit);
    assert_eq!(options.viewport.min_inner_size, Some(MIN_SIZE));
    let place = WindowPlace {
        x: 100.0,
        y: 50.0,
        width: 1100.0,
        height: 700.0,
        scale: 1.0,
        maximized: false,
    };
    let options = native_options(&Opening::Place(place));
    assert_eq!(options.viewport.min_inner_size, Some(MIN_SIZE));
    let smallest = fitted_size(egui::vec2(1280.0, 720.0), DECORATIONS);
    assert!(
        MIN_SIZE.x <= smallest.x && MIN_SIZE.y <= smallest.y,
        "the minimum {MIN_SIZE:?} fits inside the smallest default {smallest:?}"
    );
}

// The committed PNGs decode at the sizes they were stored at, on every
// platform `window_icon` compiles for: a member on either platform must
// never meet the decode failure its `expect` asserts against.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_icon_pngs_decode_at_their_stored_sizes() {
    let small = eframe::icon_data::from_png_bytes(SMALL_ICON_PNG).unwrap();
    assert_eq!(small.width, 64);
    assert_eq!(small.height, 64);
    assert_eq!(small.rgba.len(), 64 * 64 * 4);
    let window = eframe::icon_data::from_png_bytes(WINDOW_ICON_PNG).unwrap();
    assert_eq!(window.width, 256);
    assert_eq!(window.height, 256);
    assert_eq!(window.rgba.len(), 256 * 256 * 4);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn both_openings_carry_the_platform_icon() {
    let place = WindowPlace {
        x: 100.0,
        y: 50.0,
        width: 1100.0,
        height: 700.0,
        scale: 1.0,
        maximized: false,
    };
    let expected = eframe::icon_data::from_png_bytes(if cfg!(windows) {
        SMALL_ICON_PNG
    } else {
        WINDOW_ICON_PNG
    })
    .unwrap();
    for opening in [Opening::Fit, Opening::Place(place)] {
        let icon = native_options(&opening)
            .viewport
            .icon
            .expect("both openings carry the icon");
        assert_eq!(icon.width, expected.width);
        assert_eq!(icon.height, expected.height);
        assert_eq!(icon.rgba, expected.rgba);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn both_openings_carry_the_app_id() {
    let place = WindowPlace {
        x: 100.0,
        y: 50.0,
        width: 1100.0,
        height: 700.0,
        scale: 1.0,
        maximized: false,
    };
    for opening in [Opening::Fit, Opening::Place(place)] {
        assert_eq!(
            native_options(&opening).viewport.app_id.as_deref(),
            Some("unlimited-tool-works")
        );
    }
}

// ST-WIN-01
#[test]
fn the_first_frame_with_a_monitor_size_fits_and_centers_the_window_once() {
    let egui_ctx = test_context();

    for (monitor, size, pos) in [
        (
            egui::vec2(1920.0, 1080.0),
            egui::vec2(1280.0, 780.0),
            egui::pos2(312.0, 130.5),
        ),
        (
            egui::vec2(1366.0, 768.0),
            egui::vec2(1280.0, 691.0),
            egui::pos2(35.0, 19.0),
        ),
        (
            egui::vec2(1280.0, 720.0),
            egui::vec2(1264.0, 648.0),
            egui::pos2(0.0, 16.5),
        ),
    ] {
        let Shell { mut app, _dir, .. } = shell();
        let out = monitor_frame(&egui_ctx, &mut app, Some(monitor));
        assert_eq!(
            fitting(&out),
            [
                ViewportCommand::InnerSize(size),
                ViewportCommand::OuterPosition(pos),
            ],
            "monitor {monitor:?}: size then outer-rect centering"
        );
        // No later frame sends either command: a member's resize holds.
        let out = monitor_frame(&egui_ctx, &mut app, Some(monitor));
        assert!(
            fitting(&out).is_empty(),
            "monitor {monitor:?}: the fit is sent once"
        );
    }

    // A frame reporting no monitor size sends nothing and leaves the fit
    // pending; the first frame that reports one sends both.
    let Shell { mut app, _dir, .. } = shell();
    let out = monitor_frame(&egui_ctx, &mut app, None);
    assert!(fitting(&out).is_empty(), "no monitor size: no commands");
    let out = monitor_frame(&egui_ctx, &mut app, Some(egui::vec2(1920.0, 1080.0)));
    assert_eq!(
        fitting(&out),
        [
            ViewportCommand::InnerSize(egui::vec2(1280.0, 780.0)),
            ViewportCommand::OuterPosition(egui::pos2(312.0, 130.5)),
        ],
        "the fit waited for a frame reporting the monitor"
    );
}

// ST-WIN-04
#[test]
fn the_place_is_written_at_exit_and_the_next_start_opens_there() {
    let egui_ctx = test_context();
    let Shell { mut app, _dir, .. } = shell();
    let dir = _dir.as_ref().unwrap().path().to_path_buf();

    // Three frames at the place: the second is the zoom-landing one (the
    // window is narrower than the design width, so the first frame queues
    // a zoom), the third records.
    for _ in 0..3 {
        placed_frame(
            &egui_ctx,
            &mut app,
            egui::pos2(100.0, 50.0),
            egui::vec2(1100.0, 700.0),
            1.0,
            false,
        );
    }
    app.on_exit();

    let loaded = Settings::load(&dir.join(SETTINGS_FILE_NAME)).unwrap();
    assert_eq!(
        loaded.common.window_main,
        Some(WindowPlace {
            x: 100.0,
            y: 50.0,
            width: 1100.0,
            height: 700.0,
            scale: 1.0,
            maximized: false,
        }),
    );

    // The restart: a new shell over the loaded settings in the same folder,
    // on a new context as a new run's.
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let Shell { mut app, .. } = shell_with(
        loaded,
        vec![a, b],
        &[],
        Some(dir.clone()),
        vec![calls_a, calls_b],
    );
    let options = app.opening();
    assert_eq!(options.viewport.position, Some(egui::pos2(100.0, 50.0)));
    assert_eq!(options.viewport.inner_size, Some(egui::vec2(1100.0, 700.0)));
    assert!(!options.centered, "created where it closed, not centered");
    assert_ne!(options.viewport.maximized, Some(true), "and not maximized");
    app.monitors = |_| {
        vec![Monitor {
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0)),
            scale: 1.0,
            primary: true,
        }]
    };
    let restart_ctx = test_context();
    let out = placed_frame(
        &restart_ctx,
        &mut app,
        egui::pos2(100.0, 50.0),
        egui::vec2(1100.0, 700.0),
        1.0,
        false,
    );
    assert_eq!(
        fitting(&out),
        [ViewportCommand::OuterPosition(egui::pos2(100.0, 50.0))],
        "the position again in the frame's points, and no size"
    );
    let out = placed_frame(
        &restart_ctx,
        &mut app,
        egui::pos2(100.0, 50.0),
        egui::vec2(1100.0, 700.0),
        1.0,
        false,
    );
    assert!(
        fitting(&out).is_empty(),
        "the position is sent on the first frame only"
    );
}

// ST-WIN-04
#[test]
fn a_place_on_no_monitor_opens_fitted_and_an_empty_list_trusts_it() {
    let egui_ctx = test_context();
    let dir = TestDir::new("studio_core_window_place_offscreen");
    let mut settings = test_settings(dir.path());
    settings.common.window_main = Some(WindowPlace {
        x: 2500.0,
        y: 50.0,
        width: 1100.0,
        height: 700.0,
        scale: 1.0,
        maximized: false,
    });
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let Shell { mut app, .. } = shell_with(
        settings.clone(),
        vec![a, b],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a, calls_b],
    );

    // The window is created at the place, unchecked: the check needs the
    // window's own monitor list, which exists only once the window does.
    let options = app.opening();
    assert_eq!(options.viewport.position, Some(egui::pos2(2500.0, 50.0)));
    assert_eq!(options.viewport.inner_size, Some(egui::vec2(1100.0, 700.0)));
    assert!(!options.centered);

    // (2500, 50) puts the title bar past the 1920 monitor's right edge:
    // the first frame drops the place for the fit of ST-WIN-01's first row.
    app.monitors = |_| {
        vec![Monitor {
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0)),
            scale: 1.0,
            primary: true,
        }]
    };
    let out = monitor_frame(&egui_ctx, &mut app, Some(egui::vec2(1920.0, 1080.0)));
    assert_eq!(
        fitting(&out),
        [
            ViewportCommand::InnerSize(egui::vec2(1280.0, 780.0)),
            ViewportCommand::OuterPosition(egui::pos2(312.0, 130.5)),
        ],
    );

    // No list at all, a frame with no window to ask: the place is trusted.
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let Shell { mut app, .. } = shell_with(
        settings,
        vec![a, b],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a, calls_b],
    );
    app.opening();
    let out = placed_frame(
        &egui_ctx,
        &mut app,
        egui::pos2(2500.0, 50.0),
        egui::vec2(1100.0, 700.0),
        1.0,
        false,
    );
    assert_eq!(
        fitting(&out),
        [ViewportCommand::OuterPosition(egui::pos2(2500.0, 50.0))],
    );
}

// ST-WIN-04
#[test]
fn a_maximized_place_on_no_monitor_is_unmaximized_before_the_fit() {
    let egui_ctx = test_context();
    let dir = TestDir::new("studio_core_window_place_offscreen_maximized");
    let mut settings = test_settings(dir.path());
    settings.common.window_main = Some(WindowPlace {
        x: 2500.0,
        y: 50.0,
        width: 1100.0,
        height: 700.0,
        scale: 1.0,
        maximized: true,
    });
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let Shell { mut app, .. } = shell_with(
        settings,
        vec![a, b],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a, calls_b],
    );
    let options = app.opening();
    assert_eq!(options.viewport.maximized, Some(true), "the premise");
    app.monitors = |_| {
        vec![Monitor {
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0)),
            scale: 1.0,
            primary: true,
        }]
    };
    let out = monitor_frame(&egui_ctx, &mut app, Some(egui::vec2(1920.0, 1080.0)));
    assert_eq!(
        fitting(&out),
        [
            ViewportCommand::Maximized(false),
            ViewportCommand::InnerSize(egui::vec2(1280.0, 780.0)),
            ViewportCommand::OuterPosition(egui::pos2(312.0, 130.5)),
        ],
        "the unmaximize first, then ST-WIN-01's fit"
    );
}

// ST-WIN-04
#[test]
fn a_window_on_a_200_percent_display_records_its_points_and_reopens_in_them() {
    let egui_ctx = test_context();
    let Shell { mut app, _dir, .. } = shell();
    let dir = _dir.as_ref().unwrap().path().to_path_buf();

    // A 2200 × 1400 pixel window at (100, 50) on a 200% display: the
    // record holds the points and the scale.
    for _ in 0..3 {
        placed_frame(
            &egui_ctx,
            &mut app,
            egui::pos2(100.0, 50.0),
            egui::vec2(2200.0, 1400.0),
            2.0,
            false,
        );
    }
    app.on_exit();

    let loaded = Settings::load(&dir.join(SETTINGS_FILE_NAME)).unwrap();
    assert_eq!(
        loaded.common.window_main,
        Some(WindowPlace {
            x: 100.0,
            y: 50.0,
            width: 1100.0,
            height: 700.0,
            scale: 2.0,
            maximized: false,
        }),
    );

    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let Shell { mut app, .. } = shell_with(
        loaded,
        vec![a, b],
        &[],
        Some(dir.clone()),
        vec![calls_a, calls_b],
    );
    let options = app.opening();
    assert_eq!(options.viewport.position, Some(egui::pos2(50.0, 25.0)));
    assert_eq!(options.viewport.inner_size, Some(egui::vec2(1100.0, 700.0)));
    assert!(!options.centered);
    let restart_ctx = test_context();
    let out = placed_frame(
        &restart_ctx,
        &mut app,
        egui::pos2(100.0, 50.0),
        egui::vec2(2200.0, 1400.0),
        2.0,
        false,
    );
    assert_eq!(
        fitting(&out),
        [ViewportCommand::OuterPosition(egui::pos2(50.0, 25.0))],
        "the position in the frame's own points"
    );
}

#[test]
fn the_placed_windows_position_is_sent_in_points_of_the_scale_times_the_zoom() {
    let egui_ctx = test_context();
    let dir = TestDir::new("studio_core_window_place_zoomed");
    let mut settings = test_settings(dir.path());
    settings.common.window_main = Some(WindowPlace {
        x: 100.0,
        y: 50.0,
        width: 1024.0,
        height: 700.0,
        scale: 2.0,
        maximized: false,
    });
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let Shell { mut app, .. } = shell_with(
        settings,
        vec![a, b],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a, calls_b],
    );
    app.opening();
    // A run starts at zoom 1, so the first frame meets another zoom only
    // when one is queued before it, as here: it lands at that frame's start.
    egui_ctx.set_zoom_factor(0.8);
    let out = placed_frame(
        &egui_ctx,
        &mut app,
        egui::pos2(100.0, 50.0),
        egui::vec2(2048.0, 1400.0),
        2.0,
        false,
    );
    // 2.0 × 0.8 = 1.6 pixels a point: 100 / 1.6 = 62.5, 50 / 1.6 = 31.25.
    assert_eq!(
        fitting(&out),
        [ViewportCommand::OuterPosition(egui::pos2(62.5, 31.25))],
    );
}

/// The frames of a window at (100, 50), 1100 × 700 on a 100% display, then
/// `ends` in order (`Some(true)` a maximized frame, `Some(false)` a normal
/// one at the place, `None` a minimized one), then the close: the
/// `window_main` the settings file holds, and the folder it is in.
fn window_main_after(ends: &[Option<bool>]) -> (Option<WindowPlace>, Shell) {
    let egui_ctx = test_context();
    let mut shell = shell();
    // Three frames at the place: the second is the zoom-landing one, the
    // third records.
    for _ in 0..3 {
        placed_frame(
            &egui_ctx,
            &mut shell.app,
            egui::pos2(100.0, 50.0),
            egui::vec2(1100.0, 700.0),
            1.0,
            false,
        );
    }
    for end in ends {
        match end {
            // A maximized window reports no normal place: the last one stands.
            Some(true) => placed_frame(
                &egui_ctx,
                &mut shell.app,
                egui::Pos2::ZERO,
                egui::vec2(1920.0, 1041.0),
                1.0,
                true,
            ),
            Some(false) => placed_frame(
                &egui_ctx,
                &mut shell.app,
                egui::pos2(100.0, 50.0),
                egui::vec2(1100.0, 700.0),
                1.0,
                false,
            ),
            None => minimized_frame(&egui_ctx, &mut shell.app),
        };
    }
    shell.app.on_exit();
    let dir = shell._dir.as_ref().unwrap().path();
    let loaded = Settings::load(&dir.join(SETTINGS_FILE_NAME)).unwrap();
    (loaded.common.window_main, shell)
}

// ST-WIN-04
#[test]
fn a_window_maximized_at_close_keeps_the_place_before_the_maximize_with_1_last() {
    let place = |maximized| WindowPlace {
        x: 100.0,
        y: 50.0,
        width: 1100.0,
        height: 700.0,
        scale: 1.0,
        maximized,
    };
    assert_eq!(
        window_main_after(&[Some(true), Some(true)]).0,
        Some(place(true)),
        "closed maximized"
    );
    assert_eq!(
        window_main_after(&[Some(true), Some(true), None]).0,
        Some(place(true)),
        "minimized from maximized, then closed"
    );
    assert_eq!(
        window_main_after(&[Some(true), Some(false)]).0,
        Some(place(false)),
        "maximized, then back to normal before the close"
    );
    assert_eq!(
        window_main_after(&[None]).0,
        Some(place(false)),
        "minimized from normal, then closed"
    );
}

// ST-WIN-04
#[test]
fn a_window_closed_maximized_reopens_maximized_at_its_place() {
    let (window_main, Shell { _dir, .. }) = window_main_after(&[Some(true)]);
    let dir = _dir.as_ref().unwrap().path().to_path_buf();
    let loaded = Settings::load(&dir.join(SETTINGS_FILE_NAME)).unwrap();
    assert_eq!(loaded.common.window_main, window_main);
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let Shell { mut app, .. } = shell_with(
        loaded,
        vec![a, b],
        &[],
        Some(dir.clone()),
        vec![calls_a, calls_b],
    );
    let options = app.opening();
    assert_eq!(options.viewport.maximized, Some(true), "created maximized");
    assert_eq!(options.viewport.position, Some(egui::pos2(100.0, 50.0)));
    assert_eq!(options.viewport.inner_size, Some(egui::vec2(1100.0, 700.0)));
    assert!(!options.centered);

    // The first frame, the window shown maximized on the primary monitor, whose
    // scale and the frame's are the record's: the creation landed at the record's
    // pixels, so a resend would only restore the window and maximize it again.
    app.monitors = |_| {
        vec![Monitor {
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0)),
            scale: 1.0,
            primary: true,
        }]
    };
    let restart_ctx = test_context();
    let out = placed_frame(
        &restart_ctx,
        &mut app,
        egui::Pos2::ZERO,
        egui::vec2(1920.0, 1041.0),
        1.0,
        true,
    );
    assert_eq!(fitting(&out), [ViewportCommand::Maximized(true)]);
}

/// The maintainer's desk: an FHD primary at the origin at 100%, and a 4K to
/// its right at 200%, 327 pixels higher.
fn mixed_desk(_: &eframe::Frame) -> Vec<Monitor> {
    vec![
        Monitor {
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0)),
            scale: 1.0,
            primary: true,
        },
        Monitor {
            rect: egui::Rect::from_min_size(egui::pos2(1920.0, -327.0), egui::vec2(3840.0, 2160.0)),
            scale: 2.0,
            primary: false,
        },
    ]
}

/// The place commands of the first frame after a start at the remembered
/// `place`, the window's monitors `monitors`, the frame reporting the window
/// maximized at `native` pixels per point.
fn first_frame_commands(
    place: WindowPlace,
    monitors: fn(&eframe::Frame) -> Vec<Monitor>,
    native: f32,
) -> Vec<ViewportCommand> {
    let dir = TestDir::new("studio_core_window_place_maximized");
    let mut settings = test_settings(dir.path());
    settings.common.window_main = Some(place);
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let Shell { mut app, .. } = shell_with(
        settings,
        vec![a, b],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a, calls_b],
    );
    let options = app.opening();
    assert_eq!(options.viewport.maximized, Some(true), "the premise");
    app.monitors = monitors;
    let out = placed_frame(
        &test_context(),
        &mut app,
        egui::Pos2::ZERO,
        egui::vec2(1920.0, 1041.0),
        native,
        true,
    );
    fitting(&out)
}

// ST-WIN-04
#[test]
fn a_maximized_place_on_a_monitor_of_another_scale_than_the_primary_resends_its_position() {
    // [2400, 100, 1100, 700, 2, 1]: on the 4K at 200%, which winit's creation
    // converts at the primary's 100%.
    let place = WindowPlace {
        x: 2400.0,
        y: 100.0,
        width: 1100.0,
        height: 700.0,
        scale: 2.0,
        maximized: true,
    };
    assert_eq!(
        first_frame_commands(place, mixed_desk, 2.0),
        [
            ViewportCommand::OuterPosition(egui::pos2(1200.0, 50.0)),
            ViewportCommand::Maximized(true),
        ],
        "the position in the frame's points, then the maximize the move undoes"
    );
}

// ST-WIN-04
#[test]
fn a_maximized_place_at_the_primarys_scale_on_a_frame_of_another_resends_its_position() {
    // The record's scale is the primary's, but the window landed on the 4K.
    let place = WindowPlace {
        x: 100.0,
        y: 50.0,
        width: 1100.0,
        height: 700.0,
        scale: 1.0,
        maximized: true,
    };
    assert_eq!(
        first_frame_commands(place, mixed_desk, 2.0),
        [
            ViewportCommand::OuterPosition(egui::pos2(50.0, 25.0)),
            ViewportCommand::Maximized(true),
        ],
    );
}

#[test]
fn a_maximized_place_with_no_monitor_list_resends_its_position() {
    // A frame with no window to ask: no primary is known.
    let place = WindowPlace {
        x: 100.0,
        y: 50.0,
        width: 1100.0,
        height: 700.0,
        scale: 1.0,
        maximized: true,
    };
    assert_eq!(
        first_frame_commands(place, |_| Vec::new(), 1.0),
        [
            ViewportCommand::OuterPosition(egui::pos2(100.0, 50.0)),
            ViewportCommand::Maximized(true),
        ],
    );
}

#[test]
fn only_a_maximized_place_whose_scales_all_match_skips_the_resend() {
    let place = |maximized| WindowPlace {
        x: 100.0,
        y: 50.0,
        width: 1100.0,
        height: 700.0,
        scale: 1.0,
        maximized,
    };
    let monitor = |scale, primary| Monitor {
        rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0)),
        scale,
        primary,
    };
    assert!(
        resends_position(&place(false), 1.0, &[monitor(1.0, true)]),
        "a place not maximized, every scale matching"
    );
    assert!(
        !resends_position(&place(true), 1.0, &[monitor(1.0, true)]),
        "maximized, every scale matching"
    );
    assert!(
        resends_position(&place(true), 1.0, &[monitor(1.0, false)]),
        "no monitor marked primary"
    );
    assert!(
        resends_position(&place(true), 1.0, &[monitor(2.0, true)]),
        "the primary at another scale"
    );
    assert!(
        resends_position(&place(true), 2.0, &[monitor(1.0, true)]),
        "the frame at another scale"
    );
    assert!(
        !resends_position(&place(true), 1.0004, &[monitor(1.0004, true)]),
        "scales 1.0004 against the record's 1.0 match"
    );
}

// ST-WIN-04
#[test]
fn a_five_number_record_opens_not_maximized() {
    let dir = TestDir::new("studio_core_window_place_five");
    let mut settings = test_settings(dir.path());
    settings.common.window_main =
        Settings::parse("[common]\nwindow_main = [100, 50, 1100, 700, 1]\n")
            .unwrap()
            .common
            .window_main;
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let Shell { mut app, .. } = shell_with(
        settings,
        vec![a, b],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a, calls_b],
    );
    let options = app.opening();
    assert_ne!(options.viewport.maximized, Some(true), "not maximized");
    assert_eq!(options.viewport.position, Some(egui::pos2(100.0, 50.0)));
    assert_eq!(options.viewport.inner_size, Some(egui::vec2(1100.0, 700.0)));
}

// ST-WIN-02
#[test]
fn a_window_narrower_than_the_design_width_scales_the_ui_with_the_width() {
    // A 960 × 700 window at 100%: 960 of the 1280-point design width.
    let egui_ctx = test_context();
    let Shell { mut app, _dir, .. } = shell();
    window_frame(&egui_ctx, &mut app, egui::vec2(960.0, 700.0), 1.0);
    window_frame(&egui_ctx, &mut app, egui::vec2(960.0, 700.0), 1.0);
    assert_eq!(egui_ctx.zoom_factor(), 0.75);
    assert_eq!(egui_ctx.content_rect().width(), 1280.0);
    // The zoom holds over later frames, and a resize of the height alone
    // changes nothing.
    for size_px in [
        egui::vec2(960.0, 700.0),
        egui::vec2(960.0, 600.0),
        egui::vec2(960.0, 600.0),
    ] {
        window_frame(&egui_ctx, &mut app, size_px, 1.0);
        assert_eq!(egui_ctx.zoom_factor(), 0.75);
        assert_eq!(egui_ctx.content_rect().width(), 1280.0);
    }

    // At and over the design width the zoom is 1 and the view reflows.
    for width_px in [1280.0, 1600.0] {
        let egui_ctx = test_context();
        let Shell { mut app, _dir, .. } = shell();
        window_frame(&egui_ctx, &mut app, egui::vec2(width_px, 700.0), 1.0);
        window_frame(&egui_ctx, &mut app, egui::vec2(width_px, 700.0), 1.0);
        assert_eq!(egui_ctx.zoom_factor(), 1.0);
        assert_eq!(egui_ctx.content_rect().width(), width_px);
    }

    // At 150% display scaling 1440 native pixels are the same 960 points.
    let egui_ctx = test_context();
    let Shell { mut app, _dir, .. } = shell();
    window_frame(&egui_ctx, &mut app, egui::vec2(1440.0, 1050.0), 1.5);
    window_frame(&egui_ctx, &mut app, egui::vec2(1440.0, 1050.0), 1.5);
    assert_eq!(egui_ctx.zoom_factor(), 0.75);
    assert_eq!(egui_ctx.content_rect().width(), 1280.0);
}

// ST-WIN-02
#[test]
fn the_frame_after_a_jump_under_the_design_width_lays_out_at_the_design_width() {
    // A snap from a 1920-point window to 960: the frame that meets 960 at zoom
    // 1 must not reach the screen laid out 960 points wide.
    let egui_ctx = test_context();
    let Shell { mut app, _dir, .. } = shell();
    window_frame(&egui_ctx, &mut app, egui::vec2(1920.0, 1000.0), 1.0);
    window_frame(&egui_ctx, &mut app, egui::vec2(1920.0, 1000.0), 1.0);
    assert_eq!(egui_ctx.zoom_factor(), 1.0);

    // Each pass of the jump frame: its layout width and whether egui discards it.
    let mut passes = Vec::new();
    let input = window_input(&egui_ctx, egui::vec2(960.0, 1000.0), 1.0);
    let output = egui_ctx.run_ui(input, |ui| {
        app.logic(ui.ctx(), &mut eframe::Frame::_new_kittest());
        app.ui(ui, &mut eframe::Frame::_new_kittest());
        passes.push((ui.ctx().content_rect().width(), ui.ctx().will_discard()));
    });
    output.drop_without_applying_deltas();

    let (last_width, _) = *passes.last().expect("a frame runs at least one pass");
    assert_eq!(last_width, 1280.0, "the kept pass, of {passes:?}");
    for (width, discarded) in &passes {
        assert!(
            *discarded || *width == 1280.0,
            "a pass reaching the screen, of {passes:?}"
        );
    }
}

// The zoom factor is the shell's alone: egui's keyboard zoom is off, so Ctrl
// with plus changes nothing.
#[test]
fn a_ctrl_plus_leaves_the_zoom_alone() {
    let egui_ctx = test_context();
    let Shell { mut app, _dir, .. } = shell();
    let ctrl_plus = egui::Event::Key {
        key: egui::Key::Plus,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    };
    frame(&egui_ctx, &mut app, vec![ctrl_plus], true);
    // A keyboard zoom is queued for the next pass, which is where the
    // factor would move.
    frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_eq!(egui_ctx.zoom_factor(), 1.0);
}
