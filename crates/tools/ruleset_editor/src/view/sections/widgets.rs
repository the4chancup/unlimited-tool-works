//! The cells the section forms repeat (the draft's `drag`, `text` and
//! weak "—" helpers): a number with a range, a text field of a given
//! width and the dash a cell draws where the row has no value.

use studio_core::widgets::{number_arrows, number_wheel_steps, step_number, text_box};

/// A `DragValue` over `value` whose edits stay inside `range`, the
/// stored value drawn as it is (the draft's `drag`); every section's
/// number goes through it. `clamp_existing_to_range` stays off: an egui number control clamps
/// the value it draws unless told not to, and a stored value past the
/// range would come back changed on the frame that drew it (drawing
/// DBC7's 48 paid cards past a 0-20 range rewrote them to 20).
pub(crate) fn number(
    ui: &mut egui::Ui,
    value: &mut u8,
    range: std::ops::RangeInclusive<u8>,
) -> egui::Response {
    let mut response = number_arrows(ui, value, range.clone(), |ui, value| {
        ui.add(
            egui::DragValue::new(value)
                .range(range.clone())
                .clamp_existing_to_range(false),
        )
    });
    if step_number(value, number_wheel_steps(ui, &response), range) {
        response.mark_changed();
    }
    response
}

/// A single-line text field `width` points wide (the draft's `text`),
/// `add_sized` rather than `desired_width`: a `TextEdit` caps its box
/// at the width a grid cell offers, and an `egui::Grid` offers a column
/// only what a cell of it has drawn — `interact_size` until then, which
/// the cap would keep it at for good.
pub(crate) fn text(ui: &mut egui::Ui, value: &mut String, width: f32) -> egui::Response {
    ui.add_sized(
        egui::vec2(width, ui.spacing().interact_size.y),
        text_box(egui::TextEdit::singleline(value)),
    )
}

/// The weak "—" a cell draws where the row has no value.
pub(crate) fn dash(ui: &mut egui::Ui) {
    ui.weak("—");
}
