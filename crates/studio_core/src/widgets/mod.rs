//! The widgets several tools' views share: `BottomDock`, the collapsible bottom
//! panel's frame, `LogPanel`,
//! the run log drawn in it, `well_frame`, the log's well a dock's rows sit in,
//! the settings rows (`settings_row`) the menu's
//! sections share, the checkbox and radio button every view draws
//! (`choice`), a tool's toolbar row with its group separator
//! (`toolbar`), the measure that places text by its ink (`ink`), the
//! wheel stepping a combo box or a drag value (`wheel`), the modal window
//! every dialog draws through (`modal`), and the row spacing every form
//! shares (`FORM_ROW_SPACING`).
//! Where they sit in a tool's view is that tool's choice; the widgets
//! themselves know no tool.

mod choice;
mod dock;
mod ink;
mod log_panel;
mod menu_button;
mod modal;
mod settings_row;
mod toolbar;
mod wheel;

pub use choice::{checkbox, checkbox_without_text, radio, radio_value};
pub use dock::BottomDock;
pub use ink::ink_span;
pub use log_panel::{LogLine, LogPanel, well_frame};
pub use menu_button::menu_button;
pub use modal::{modal, modal_buttons, modal_frame};
pub(crate) use settings_row::{PathCommit, path_row_commit, row_label};
pub use settings_row::{PathKind, path_row};
pub use toolbar::{toolbar, toolbar_separator};
pub use wheel::{step_choice, step_number, wheel_steps};

/// The row spacing of a form that draws a drafts' `table.grid` (label, control,
/// label, control): the drafts pad each cell 2 points above and below, which puts
/// rows of 18-point controls 22 apart, where egui's `item_spacing.y` of 3 would put
/// them 21. An `egui::Grid`'s row spacing, or the `item_spacing.y` of a section
/// whose rows stack in a vertical layout; the theme's 3 still spaces every other
/// layout.
pub const FORM_ROW_SPACING: f32 = 4.0;
