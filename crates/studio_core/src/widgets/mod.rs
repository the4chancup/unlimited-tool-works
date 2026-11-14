//! The widgets several tools' views share: `BottomDock`, the collapsible bottom
//! panel's frame, `LogPanel`,
//! the run log drawn in it, `well_frame`, the log's well a dock's rows sit in,
//! the disabled widget and scope every view draws (`add_enabled`,
//! `add_enabled_ui`),
//! the settings rows (`settings_row`) the menu's
//! sections share, the checkbox and radio button every view draws
//! (`choice`), the text box every view draws (`text_box`) and its resting
//! outline (`text_box_border`), the collapsing
//! header every view draws (`collapsing_header`), a tool's
//! toolbar row with its group separator and its framed scope (`toolbar`), the measure that
//! places text by its ink (`ink`), the button frame a clickable picture shows while in
//! use (`in_use_frame`), the slider with its value box (`slider`), the wheel stepping a combo box or a drag value (`wheel`), the modal window
//! every dialog draws through (`modal`), and the spacing of stacked fields
//! and checkboxes (`FIELD_SPACING`, `CHECKBOX_SPACING`, `FORM_ROW_SPACING`).
//! Where they sit in a tool's view is that tool's choice; the widgets
//! themselves know no tool.

mod choice;
mod collapsing_header;
mod combo;
mod dock;
mod enabled;
mod in_use_frame;
mod ink;
mod log_panel;
mod menu_button;
mod modal;
mod settings_row;
mod slider;
mod text_box;
mod toolbar;
mod wheel;

pub use choice::{checkbox, checkbox_without_text, radio, radio_value};
pub use collapsing_header::collapsing_header;
pub use combo::{COMBO_SEPARATOR_SPACING, combo_item, combo_label, combo_separator};
pub use dock::BottomDock;
pub use enabled::{add_enabled, add_enabled_ui};
pub use in_use_frame::in_use_frame;
pub use ink::ink_span;
pub use log_panel::{LogLine, LogPanel, well_frame};
pub use menu_button::{menu_button, menu_separator};
pub use modal::{
    dialog_form, dialog_page, modal, modal_buttons, modal_fit_contents, modal_frame,
    modal_sentence_gap,
};
pub(crate) use settings_row::{PathCommit, path_row, path_row_commit, row_label};
pub use slider::slider;
pub use text_box::{text_box, text_box_border};
pub use toolbar::{toolbar, toolbar_framed, toolbar_separator};
pub use wheel::{cycle_choice, number_arrows, set_inverted, step_choice, step_number, wheel_steps};

/// The row spacing of the forms that draw a drafts' `table.grid` (label, control,
/// label, control) and have not moved to `FIELD_SPACING` yet: the team view's
/// fields, the settings menu and the tools' settings, the Appearance screen. The
/// drafts pad each cell 2 points above and below, which puts rows of 18-point
/// controls 22 apart, where egui's `item_spacing.y` of 3 would put them 21. An
/// `egui::Grid`'s row spacing, or the `item_spacing.y` of a section whose rows
/// stack in a vertical layout; the theme's 3 still spaces every other layout.
pub const FORM_ROW_SPACING: f32 = 4.0;

/// The space between stacked fields' 18-point boxes: the Save editor's Stats
/// screen and a dialog's form, whose rows stand 24 apart. An `egui::Grid`'s row
/// spacing, or the `item_spacing.y` of rows stacked in a vertical layout. Stacked
/// controls are spaced by the gap between their boxes, not by one row pitch,
/// because a checkbox's square is 4 points shorter than a field: see
/// `CHECKBOX_SPACING`.
pub const FIELD_SPACING: f32 = 6.0;

/// The space between checkboxes stacked one under another, as the
/// `item_spacing.y` of their vertical layout: their 18-point rows stand 22
/// apart, which leaves 8 points between their 14-point squares. A field's
/// `FIELD_SPACING` would leave 10 between squares, since the square is 4 points
/// shorter than a field's box.
pub const CHECKBOX_SPACING: f32 = 4.0;
