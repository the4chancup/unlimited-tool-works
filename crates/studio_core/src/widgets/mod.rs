//! The widgets several tools' views share: `BottomDock`, the collapsible bottom
//! panel's frame, `LogPanel`,
//! the run log drawn in it, the settings rows (`settings_row`) the menu's
//! sections share, the checkbox and radio button every view draws
//! (`choice`), a tool's toolbar row with its group separator
//! (`toolbar`), the measure that places text by its ink (`ink`), and the
//! wheel stepping a combo box or a drag value (`wheel`), and the modal
//! window every dialog draws through (`modal`).
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
pub use log_panel::{LogLine, LogPanel};
pub use menu_button::menu_button;
pub use modal::{modal, modal_buttons, modal_frame};
pub(crate) use settings_row::{PathCommit, path_row_commit, row_label};
pub use settings_row::{PathKind, path_row};
pub use toolbar::{toolbar, toolbar_separator};
pub use wheel::{step_choice, step_number, wheel_steps};
