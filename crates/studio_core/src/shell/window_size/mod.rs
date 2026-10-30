//! The window's opening size, its one-time fit to the monitor, the place it
//! reopens at and the zoom the views scale with under the design width. The
//! window is created at `DEFAULT_SIZE` or at the remembered place. The first
//! frame that reports the monitor's size fits and centers it once, and a
//! member's own resize or move holds after that. A window narrower than
//! `DEFAULT_SIZE.x` draws the whole UI scaled down with the width
//! (spec/core/gui.md "The window's size", "Scaling with the width").

use crate::settings::WindowPlace;

/// The size the window is created at and opens at on any monitor that holds it:
/// 1280 × 780 points, where "holds" means the height within 90% of the
/// monitor's logical height (room for the taskbar and the title bar) and the
/// width, decorations included, within the monitor's logical width.
pub(super) const DEFAULT_SIZE: egui::Vec2 = egui::vec2(1280.0, 780.0);

/// The smallest window a member's resize can reach: 960 × 600 points, under
/// the smallest default the fit produces on a monitor of at least 1280 × 720
/// logical points (1264 × 648), so on such a monitor the fit never meets it;
/// a smaller one (a 1366 × 768 laptop at 125% is 1093 × 614) can ask for less
/// than the minimum, which then holds. Under the design width the UI scales
/// down rather than narrowing (`width_zoom`), so the bound is legibility:
/// below 960 points the scaled text drops under 75% of its size.
#[cfg(not(target_arch = "wasm32"))]
pub(super) const MIN_SIZE: egui::Vec2 = egui::vec2(960.0, 600.0);

/// The icon, Archer on a transparent background in the tight crop on
/// the head at 64 pixels (`resources/icon/small.png`): the window's icon on
/// Windows, whose small size reads only as the tight crop.
#[cfg(not(target_arch = "wasm32"))]
const SMALL_ICON_PNG: &[u8] = include_bytes!("../../../../../resources/icon/small.png");

/// The icon, the whole Archer picture at 256 pixels
/// (`resources/icon/window.png`): the window's icon everywhere but Windows,
/// where one image serves every size.
#[cfg(not(target_arch = "wasm32"))]
const WINDOW_ICON_PNG: &[u8] = include_bytes!("../../../../../resources/icon/window.png");

/// The size the first frame fits the window to: the main class's design size
/// fitted to `monitor` (`fitted_size_of`).
pub(super) fn fitted_size(monitor: egui::Vec2, decorations: egui::Vec2) -> egui::Vec2 {
    fitted_size_of(DEFAULT_SIZE, monitor, decorations)
}

/// `design` fitted to `monitor`: the height is `design.y` while it is within
/// 90% of `monitor`'s height, else 90% of it rounded to whole points; the width
/// is `design.x` while the window with its `decorations` fits `monitor`'s
/// width, else what is left over them.
fn fitted_size_of(design: egui::Vec2, monitor: egui::Vec2, decorations: egui::Vec2) -> egui::Vec2 {
    let height = if design.y <= monitor.y * 0.9 {
        design.y
    } else {
        (monitor.y * 0.9).round()
    };
    let width = if design.x + decorations.x <= monitor.x {
        design.x
    } else {
        monitor.x - decorations.x
    };
    egui::vec2(width, height)
}

/// The zoom factor for a window `window_width` points wide at zoom 1: the
/// design width the views are laid out for is `DEFAULT_SIZE.x`, so narrower
/// scales the whole UI down with the width and at or over it the zoom is 1
/// and the views reflow.
pub(super) fn width_zoom(window_width: f32) -> f32 {
    (window_width / DEFAULT_SIZE.x).min(1.0)
}

/// How the window opens: fitted and centered on the primary monitor (no remembered place, or
/// one on no monitor), or created at the main class's remembered place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Opening {
    Fit,
    /// The remembered place is a native concept: no place is ever remembered
    /// or checked on the browser build.
    #[cfg(not(target_arch = "wasm32"))]
    Place(WindowPlace),
}

/// The height of a title bar at 100% scale, in pixels: what "the title bar lies on a
/// monitor" measures (32 on Windows 10 and 11).
#[cfg(not(target_arch = "wasm32"))]
const TITLE_BAR_PX: f32 = 32.0;

/// The window's place a frame reports, in the record's units, or `None` on a frame that
/// reports no normal place: maximized or fullscreen (a minimized window reports no rects at
/// all), a rect or the native scale missing, or the viewport rects built at another zoom than
/// the pass's: on the frame where a queued zoom lands, egui rescales `screen_rect` alone at
/// `begin_pass`, so `inner_rect` and `screen_rect` differ in size there by a pixel or more
/// and agree on every other frame to a rounding error. Pixels are the outer rect's points
/// times the native scale times the zoom; points at zoom 1 are the screen rect's size times
/// the zoom.
pub(super) fn place_of(
    viewport: &egui::ViewportInfo,
    screen_rect: Option<egui::Rect>,
    zoom: f32,
) -> Option<WindowPlace> {
    if viewport.maximized == Some(true) || viewport.fullscreen == Some(true) {
        return None;
    }
    let outer = viewport.outer_rect?;
    let inner = viewport.inner_rect?;
    let native = viewport.native_pixels_per_point?;
    let screen = screen_rect?;
    // The two sizes come from different divisions (egui-winit divides the
    // inner rect's corners, `screen_rect` its size), so on a consistent frame
    // they agree to a rounding error, while on the landing frame they differ
    // by the pixel or more whose change moved the zoom: compared in pixels,
    // against half of one.
    if ((inner.size() - screen.size()) * native * zoom)
        .abs()
        .max_elem()
        >= 0.5
    {
        return None;
    }
    Some(WindowPlace {
        x: (outer.min.x * native * zoom).round(),
        y: (outer.min.y * native * zoom).round(),
        width: screen.width() * zoom,
        height: screen.height() * zoom,
        scale: native,
        maximized: false,
    })
}

/// A monitor of the window's screen: its rectangle and its scale factor, the two
/// units the title-bar check needs one of each for.
#[cfg(not(target_arch = "wasm32"))]
pub(super) struct Monitor {
    /// The monitor's rectangle in the screen's physical pixels.
    rect: egui::Rect,
    /// The monitor's scale factor (native pixels per point), which sizes the
    /// title bar the check builds for it: the window's points stand at this
    /// scale's pixels while it sits on this monitor.
    scale: f32,
    /// Whether the OS names this monitor the primary one: winit converts a new window's
    /// position with its scale.
    primary: bool,
}

/// Whether `place`'s title bar (the window's width over the top `TITLE_BAR_PX` pixels at
/// 100%, sized by each monitor's own scale, the pixels the window stands at while it
/// sits there) overlaps that monitor's rectangle (physical pixels) with a positive
/// area; an empty list (a frame with no window to ask) trusts the place.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn on_a_monitor(place: &WindowPlace, monitors: &[Monitor]) -> bool {
    monitors.is_empty()
        || monitors.iter().any(|monitor| {
            let bar = egui::Rect::from_min_size(
                place.position(),
                egui::vec2(place.width, TITLE_BAR_PX) * monitor.scale,
            );
            monitor.rect.intersect(bar).is_positive()
        })
}

/// What the first frame does with the remembered `place` the window was created at:
/// resend its position when its title bar lies on one of `monitors` (the window's monitors,
/// each its rectangle and scale), else fit and center the window as a first start does. An
/// empty list, a frame with no window to ask, trusts the place.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn place_or_fit(place: WindowPlace, monitors: &[Monitor]) -> Opening {
    if on_a_monitor(&place, monitors) {
        Opening::Place(place)
    } else {
        Opening::Fit
    }
}

/// Whether the first frame sends `place`'s position again: always for a place not
/// maximized (a resend that moves nothing does not show there); for a maximized one only
/// when the creation can have landed off the record's pixels, its scale not the primary
/// monitor's (or no primary known) or not the frame's `native` pixels per point. Winit
/// moves a maximized window by restoring it first, so a resend that moves nothing shows
/// as a flicker: the window restores and maximizes again.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn resends_position(place: &WindowPlace, native: f32, monitors: &[Monitor]) -> bool {
    // Not `==`: a scale typed into the settings file to a few decimals (an X11 desk's
    // 1.0417 for winit's 1.0416666) still names the monitor's scale.
    let is_place_scale = |scale: f32| (scale - place.scale).abs() < 0.001;
    let primary_scale = monitors
        .iter()
        .find(|monitor| monitor.primary)
        .map(|monitor| monitor.scale);
    !place.maximized || !primary_scale.is_some_and(is_place_scale) || !is_place_scale(native)
}

/// The monitors of `frame`'s window, each its rectangle in the screen's physical pixels,
/// its scale factor and whether it is the primary one (none is where winit knows no
/// primary); empty when the frame has no window (a test's frame).
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn monitors(frame: &eframe::Frame) -> Vec<Monitor> {
    let Some(window) = frame.winit_window() else {
        return Vec::new();
    };
    let primary = window.primary_monitor();
    window
        .available_monitors()
        .map(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            // Screen coordinates are far inside f32's exact integer range, and a
            // scale factor (an f64 like 1.0, 1.25, 2.0) fits in one too.
            Monitor {
                rect: egui::Rect::from_min_size(
                    egui::pos2(position.x as f32, position.y as f32),
                    egui::vec2(size.width as f32, size.height as f32),
                ),
                scale: monitor.scale_factor() as f32,
                primary: primary.as_ref() == Some(&monitor),
            }
        })
        .collect()
}

/// Turns off the icon Windows draws at the title bar's left; the taskbar and Alt+Tab keep
/// it, and the title stays where it stood, after the icon's slot, now blank. The two icons
/// come from the one image eframe sets, so no icon setting hides the title bar's alone, and
/// drawing a title bar of our own would lose the system's snapping and caption buttons.
/// Not `WTNCA_NOSYSMENU` beside it: that moves the title to the bar's edge but takes away
/// the window menu Alt+Space opens. A failure leaves the icon showing and is logged: the
/// window works the same.
#[cfg(windows)]
pub(super) fn hide_title_bar_icon(window: &impl raw_window_handle::HasWindowHandle) {
    use raw_window_handle::RawWindowHandle;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Controls::{
        SetWindowThemeAttribute, WTA_NONCLIENT, WTA_OPTIONS, WTNCA_NODRAWICON,
    };

    let handle = match window.window_handle().map(|handle| handle.as_raw()) {
        Ok(RawWindowHandle::Win32(handle)) => handle,
        other => {
            log::warn!("the title bar keeps its icon: no Win32 window handle ({other:?})");
            return;
        }
    };
    let options = WTA_OPTIONS {
        dwFlags: WTNCA_NODRAWICON,
        dwMask: WTNCA_NODRAWICON,
    };
    // SAFETY: `handle.hwnd` is the live window eframe just created, valid while the creation
    // context that lent it is borrowed here. `options` is a local `WTA_OPTIONS`, the type
    // `WTA_NONCLIENT` takes, and the size passed is its own; the call only reads it.
    let result = unsafe {
        SetWindowThemeAttribute(
            HWND(handle.hwnd.get() as *mut std::ffi::c_void),
            WTA_NONCLIENT,
            std::ptr::from_ref(&options).cast(),
            size_of::<WTA_OPTIONS>() as u32,
        )
    };
    if let Err(error) = result {
        log::warn!("the title bar keeps its icon: {error}");
    }
}

/// The window's icon. On Windows eframe makes both of the window's icons from
/// this one image: the small one (`ICON_SMALL`, which the title bar would draw
/// at 16 to 24 pixels, `hide_title_bar_icon`), where only the tight crop reads,
/// and the big one (`ICON_BIG`, the taskbar's). Elsewhere one image serves every
/// size, so the whole picture.
#[cfg(not(target_arch = "wasm32"))]
fn window_icon() -> egui::IconData {
    let png = if cfg!(windows) {
        SMALL_ICON_PNG
    } else {
        WINDOW_ICON_PNG
    };
    eframe::icon_data::from_png_bytes(png)
        .expect("the committed PNG decodes; the tests decode both at their stored sizes")
}

/// The native window's options for `opening`: `Fit` creates the window at `DEFAULT_SIZE` and
/// centered, the size the first frame's fit keeps on a monitor that holds it, so nothing
/// jumps there; `Place` creates it at the record's position (its pixels over its scale, the
/// points winit converts with the scale of the monitor it creates the window on) and inner
/// size, not centered (`NativeOptions`' default), and maximized when the record says so, the
/// place then being the one its restore button gives back. Both carry the `MIN_SIZE` minimum,
/// the icon and the app id.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn native_options(opening: &Opening) -> eframe::NativeOptions {
    let viewport = egui::ViewportBuilder::default()
        // A Linux desktop entry matches the window by this id, not by the title eframe uses.
        .with_app_id("unlimited-tool-works")
        .with_icon(window_icon())
        .with_min_inner_size(MIN_SIZE);
    match opening {
        Opening::Fit => eframe::NativeOptions {
            viewport: viewport.with_inner_size(DEFAULT_SIZE),
            centered: true,
            ..Default::default()
        },
        Opening::Place(place) => eframe::NativeOptions {
            viewport: viewport
                .with_position(place.position() / place.scale)
                .with_inner_size(place.size())
                .with_maximized(place.maximized),
            ..Default::default()
        },
    }
}

/// The fit `viewport` calls for, once it reports the monitor's size: the inner size, and
/// the outer position that centers the *outer* rectangle (the decorations, outer minus
/// inner, counted; none when either rect is missing). `None` while the monitor is unknown.
/// `viewport`'s sizes are reported in the pass's points — the zoomed units `zoom` undoes —
/// and the size and position commands take the same units, so the fit runs at zoom 1 on
/// either side of them.
pub(super) fn fit(viewport: &egui::ViewportInfo, zoom: f32) -> Option<(egui::Vec2, egui::Pos2)> {
    let monitor = egui::Rect::from_min_size(egui::Pos2::ZERO, viewport.monitor_size?);
    Some(fit_on(viewport, zoom, monitor))
}

/// `fit` on `monitor`, a rectangle in the pass's points: the inner size, and the outer
/// position that centers the outer rectangle on `monitor`.
fn fit_on(
    viewport: &egui::ViewportInfo,
    zoom: f32,
    monitor: egui::Rect,
) -> (egui::Vec2, egui::Pos2) {
    let monitor = monitor * zoom;
    let decorations = match (viewport.outer_rect, viewport.inner_rect) {
        (Some(outer), Some(inner)) => outer.size() - inner.size(),
        _ => egui::Vec2::ZERO,
    } * zoom;
    let size = fitted_size(monitor.size(), decorations);
    let top_left = monitor.min + (monitor.size() - size - decorations) * 0.5;
    (size / zoom, top_left / zoom)
}

/// The monitor the window stands on, as its rectangle in the pass's points (its pixels
/// over the window's native scale times `zoom`, the units `OuterPosition` takes): the one
/// of `monitors` holding the center of the window's outer rectangle, else the primary.
/// `None` when the frame reports no outer rectangle or scale, or no monitor qualifies (an
/// empty list: a frame with no window to ask).
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn monitor_under(
    viewport: &egui::ViewportInfo,
    zoom: f32,
    monitors: &[Monitor],
) -> Option<egui::Rect> {
    let pixels_per_point = viewport.native_pixels_per_point? * zoom;
    let center = viewport.outer_rect?.center() * pixels_per_point;
    let monitor = monitors
        .iter()
        .find(|monitor| monitor.rect.contains(center))
        .or_else(|| monitors.iter().find(|monitor| monitor.primary))?;
    Some(monitor.rect / pixels_per_point)
}

/// Whether the window sits off `design` (the active class's design size), the
/// status bar's condition for drawing the Reset window button: maximized, or
/// its inner size at zoom 1 (`screen_rect`'s size times the pass's zoom, which
/// undoes the zoom the rect is expressed in) is not `design` fitted to its
/// monitor, both rounded to whole points. The position never counts, and a
/// frame reporting no monitor or no size leaves nothing to compare, so the
/// button is not drawn. `inner_rect`, the monitor and the decorations keep
/// the zoom the input was built with while egui rescales `screen_rect` alone
/// at `begin_pass` — the two differ only on the pass a queued zoom lands on,
/// and `screen / inner` carries the conversion (1 on every consistent frame).
pub(super) fn off_design_size(
    viewport: &egui::ViewportInfo,
    screen_rect: Option<egui::Rect>,
    zoom: f32,
    design: egui::Vec2,
) -> bool {
    if viewport.maximized == Some(true) {
        return true;
    }
    let (Some(monitor), Some(inner), Some(screen)) =
        (viewport.monitor_size, viewport.inner_rect, screen_rect)
    else {
        return false;
    };
    let decorations = viewport
        .outer_rect
        .map_or(egui::Vec2::ZERO, |outer| outer.size() - inner.size());
    let input_to_zoom_one = zoom * screen.size() / inner.size();
    (screen.size() * zoom).round()
        != fitted_size_of(
            design,
            monitor * input_to_zoom_one,
            decorations * input_to_zoom_one,
        )
        .round()
}

/// What the status bar's Reset window click sends: a maximized window is
/// restored first, then the `InnerSize` and `OuterPosition` of the fit on
/// `monitor`, the one it stands on (`monitor_under`). With none known (the
/// browser build, a frame with no window) it is the first frame's fit.
pub(super) fn reset(
    ctx: &egui::Context,
    viewport: &egui::ViewportInfo,
    monitor: Option<egui::Rect>,
) {
    if viewport.maximized == Some(true) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
    }
    let zoom = ctx.zoom_factor();
    let fitted = match monitor {
        Some(monitor) => Some(fit_on(viewport, zoom, monitor)),
        None => fit(viewport, zoom),
    };
    if let Some((size, top_left)) = fitted {
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(top_left));
    }
}

#[cfg(test)]
mod tests;
