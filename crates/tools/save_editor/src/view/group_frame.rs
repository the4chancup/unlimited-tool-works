//! The `egui::Frame::group` frame the player screens share — the
//! Appearance screen's groups and the Stats screen's panels are drawn
//! in it — with the spacing constant and the margin and stretch helpers
//! the two screens lay them out by.

/// The margin over a `Frame::group`: between the Appearance screen's
/// groups and atop the Stats screen's columns (the draft's `.group`
/// `margin-top` and `.pcols > *` `margin-top`, the one 4), so a switch
/// between the screens moves no frame line.
pub(super) const GROUP_GAP: f32 = 4.0;

/// A group frame's margin on each axis: inner margin and stroke, which
/// `Prepared::outer_rect` adds to the content's `min_rect` (its outer
/// margin is 0 for `Frame::group`).
pub(super) fn frame_margin(frame: &egui::Frame) -> egui::Vec2 {
    egui::vec2(
        frame.inner_margin.sum().x + frame.stroke.width * 2.0,
        frame.inner_margin.sum().y + frame.stroke.width * 2.0,
    )
}

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
