//! The `egui::Frame::group` frame the player screens share — the
//! Appearance screen's groups and the Stats screen's panels are drawn
//! in it — with the spacing constant and the stretch helper the two
//! screens lay them out by.

/// The margin over a `Frame::group`: between the Appearance screen's
/// groups and atop the Stats screen's columns (the draft's `.group`
/// `margin-top` and `.pcols > *` `margin-top`, the one 4), so a switch
/// between the screens moves no frame line.
pub(super) const GROUP_GAP: f32 = 4.0;

/// `prepared`'s `content_ui` grown to `size` before its frame paints:
/// `Frame::begin` reserves the frame's shape, so the `paint` after the
/// stretch draws the grown frame (the padding takes the extra). The
/// Appearance screen's `stretch`, in both axes.
pub(super) fn stretch_to(prepared: &mut egui::containers::frame::Prepared, size: egui::Vec2) {
    let min = prepared.content_ui.min_rect();
    prepared
        .content_ui
        .advance_cursor_after_rect(egui::Rect::from_min_max(min.min, min.min + size));
}
