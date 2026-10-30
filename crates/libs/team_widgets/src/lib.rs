//! `team_widgets`: the egui widgets the Save editor and the Team creator both
//! draw over `pes_savefile`'s model types. They live in a lib because a tool
//! crate never depends on another tool crate. A widget takes `&mut` model,
//! the version's schema for gating and the AATF rules where it shows limits;
//! it draws and it returns what changed, so the host decides what an edit
//! means. It never sees a session, an undo stack, a file, or a savefile
//! byte.

/// The card pickers: the skills grid and COM playing styles with a
/// remaining-count badge per ruleset tier, and the playing-style combo.
pub mod cards;
/// A ruleset's event logo, decoded once and drawn at the size both
/// editors give it.
pub mod event_logo;
/// How the editor and the widgets draw a savefile name, its colour
/// codes applied as text colours.
pub mod names;
/// The tactics editor's pitch side: the formation canvas with its markers
/// and slot popover, and the starting-eleven and bench lists with
/// swap-by-drag.
pub mod pitch;
/// The tactics editor's settings half: preset selector, sliders, style
/// pairs, advanced instructions, the role palette and automatic settings.
pub mod tactics;
/// The headless egui harness the tests share.
#[cfg(test)]
mod test_support;
/// The AATF results list: a team's violations grouped per player, a row's
/// click reporting the player to select.
pub mod violations;
