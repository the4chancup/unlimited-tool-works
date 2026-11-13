//! The Save editor's section of the settings file: its eight keys. `check-aatf` and the window's
//! AATF checks read `aatf_ruleset` (the toolbar's ruleset dropdown writes it); the window reads
//! `live_aatf` (the toolbar's Live AATF toggle), `live_suggestions` (its Show suggestions
//! checkbox), `live_window` (whether Live AATF's list stands in its own window or its dock),
//! `aesthetics_from_exports` ("Aesthetics from exports", the aesthetics lock's setting),
//! `scroll_names` and `portraits` (the Tactics formation bar's checkboxes) and `last_folder`
//! (the folder the file
//! picker starts in). A key enters with the code that reads
//! it, so the file never carries a setting nothing honors.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The Save editor's settings, the keys this version reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SaveEditorSettings {
    /// The AATF ruleset the window and `check-aatf` use: a built-in ruleset's file stem
    /// (`autumn26`, the default), or an imported copy's file name (`cup.4ccrules`) in the
    /// data folder's `rules/`. One that is gone falls back to the first built-in ruleset.
    pub(crate) aatf_ruleset: String,
    /// The toolbar's Live AATF toggle; on, the open team's violations list after every
    /// edit, in the dock or its window.
    pub(crate) live_aatf: bool,
    /// Show suggestions, the checkbox beside the Live AATF toggle; on, Live AATF also lists
    /// each player's unused allowances.
    pub(crate) live_suggestions: bool,
    /// Where Live AATF lists the violations: off, in the panel docked at the bottom of the
    /// view; on, in a window of its own over it (the dock's pop-out button turns it on, the
    /// window's dock button off). Kept while Live AATF is off, so turning it on again opens
    /// the list where it was.
    pub(crate) live_window: bool,
    /// "Aesthetics from exports", off by default because the cup the suite serves first
    /// keeps its looks in the save. On, the save's aesthetics write only through the
    /// Appearance section's "Edit aesthetics anyway" switch; off, they edit as every other
    /// field does.
    pub(crate) aesthetics_from_exports: bool,
    /// "Scroll" (the Tactics formation bar's checkbox), on by default: a view setting,
    /// not an edit — `Tool::view` reads it at start and writes it back when the checkbox
    /// changes it, as `live_aatf` is written.
    #[serde(default = "default_scroll_names")]
    pub(crate) scroll_names: bool,
    /// "Portraits" (the Tactics formation bar's checkbox beside Scroll), on by default:
    /// a view setting as `scroll_names` is.
    #[serde(default = "default_portraits")]
    pub(crate) portraits: bool,
    /// The folder the last save was opened from, by whichever of the opens: the file
    /// picker starts there while it exists, so a member who keeps his saves in a folder
    /// of his own is taken back to it. Written on every open, as `scroll_names` is.
    pub(crate) last_folder: Option<PathBuf>,
}

impl Default for SaveEditorSettings {
    fn default() -> SaveEditorSettings {
        SaveEditorSettings {
            aatf_ruleset: crate::session::rules::FIRST_BUILTIN.to_owned(),
            live_aatf: false,
            live_suggestions: false,
            live_window: false,
            aesthetics_from_exports: false,
            scroll_names: default_scroll_names(),
            portraits: default_portraits(),
            last_folder: None,
        }
    }
}

/// `scroll_names`'s default: names scroll, as the game's do.
fn default_scroll_names() -> bool {
    true
}

/// `portraits`'s default: the pitch draws the players' portraits.
fn default_portraits() -> bool {
    true
}

/// The defaults as a settings table, merged into the file's section for every key it lacks.
pub(crate) fn default_table() -> toml::Table {
    toml::Table::try_from(SaveEditorSettings::default()).expect("the defaults serialize")
}

/// Reads the tool's settings table. A missing key loads as its default; a key this version does
/// not know is ignored, so a newer version's settings file does not break an older binary; a key
/// of the wrong type is an error naming it.
pub(crate) fn from_table(table: &toml::Table) -> Result<SaveEditorSettings, toml::de::Error> {
    table.clone().try_into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default ruleset is the first built-in one; a key this version does not know, the
    /// old `aatf_rules` path among them, reads back as the defaults.
    #[test]
    fn the_defaults_pick_autumn26_and_an_unknown_key_reads_back_as_the_defaults() {
        assert_eq!(default_table()["aatf_ruleset"].as_str(), Some("autumn26"));
        assert!(!default_table().contains_key("aatf_rules"));
        for unknown in [
            "setting_from_a_newer_version = 3",
            "aatf_rules = \"rules/dbc7.4ccrules\"",
        ] {
            let table: toml::Table = toml::from_str(unknown).unwrap();
            assert_eq!(
                from_table(&table).unwrap(),
                SaveEditorSettings::default(),
                "{unknown}"
            );
        }
    }

    #[test]
    fn aatf_ruleset_names_a_ruleset() {
        let table: toml::Table = toml::from_str("aatf_ruleset = \"vgl27\"").unwrap();
        assert_eq!(from_table(&table).unwrap().aatf_ruleset, "vgl27");
    }

    // SE-GUI-35's GIVEN (the setting off is the default).
    #[test]
    fn aesthetics_from_exports_defaults_off_and_round_trips() {
        assert_eq!(
            default_table()
                .get("aesthetics_from_exports")
                .and_then(toml::Value::as_bool),
            Some(false),
            "off by default"
        );
        let table: toml::Table = toml::from_str("aesthetics_from_exports = true").unwrap();
        let written = toml::Table::try_from(from_table(&table).unwrap()).unwrap();
        assert_eq!(
            written
                .get("aesthetics_from_exports")
                .and_then(toml::Value::as_bool),
            Some(true),
            "read in and written back"
        );
    }

    #[test]
    fn live_window_defaults_off_and_round_trips() {
        assert_eq!(
            default_table()
                .get("live_window")
                .and_then(toml::Value::as_bool),
            Some(false),
            "off by default: the list starts in its dock"
        );
        let table: toml::Table = toml::from_str("live_window = true").unwrap();
        let read = from_table(&table).unwrap();
        assert!(read.live_window, "read in");
        let written = toml::Table::try_from(read).unwrap();
        assert_eq!(
            written.get("live_window").and_then(toml::Value::as_bool),
            Some(true),
            "written back"
        );
    }
}
