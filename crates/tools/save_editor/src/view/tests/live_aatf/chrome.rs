//! The dock's and the window's chrome: the header buttons' and the
//! resize handles' AccessKit reports (the widget infos are hand-set — a
//! glyph names nothing — and a disabled report would read a working
//! control as greyed out).

use super::*;

// The dock header's pop-out icon and close button report their names and
// enabled.
#[test]
fn the_docks_buttons_report_their_names_enabled() {
    let (mut view, mut window) = open_734_with("live_aatf_buttons_enabled", &[73401], 183);
    view.live_aatf = true;
    let nodes = draw(&mut window, &mut view);
    assert!(dock_drawn(&nodes), "premise: the dock is drawn");
    for name in [
        "Open Live AATF in a window of its own",
        "Turn Live AATF off",
    ] {
        assert_widget_named(node_labeled(&nodes, Role::Button, name), Role::Button, name);
    }
}

// The list's window resizes by its edges: it reports the bottom-right
// corner's `Splitter` handle, as the floating panels do.
#[test]
fn the_window_reports_its_resize_corner() {
    let (mut view, mut window) = open_734_with("live_aatf_window_resizable", &[73401], 183);
    view.live_aatf = true;
    view.live_window = true;
    // A new window's first frame is egui's sizing pass, drawn invisible.
    draw(&mut window, &mut view);
    let nodes = draw(&mut window, &mut view);
    assert_resizable_window(&nodes, "Live AATF");
}
