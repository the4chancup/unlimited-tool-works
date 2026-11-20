//! The Save editor's section of the settings file: its fourteen keys. `check-aatf` and the window's
//! AATF checks read `aatf_ruleset` (the toolbar's ruleset dropdown writes it); the window reads
//! `live_aatf` (the toolbar's Live AATF toggle), `live_suggestions` (its Show suggestions
//! checkbox), `live_window` (whether Live AATF's list stands in its own window or its dock),
//! `aesthetics_from_exports` ("Aesthetics from exports", the aesthetics lock's setting),
//! `scroll_names` and `portraits` (the Tactics formation bar's checkboxes),
//! `positions_layout` (the Playable positions grid's layout toggle),
//! `team_category` (the team selector's radio buttons), `formation_pictures_folder`,
//! `formation_pictures_flat` and `formation_pictures_settings` (the formation pictures'
//! folder, grain box and settings box), `last_folder` (the folder the file
//! picker starts in) and `recent_files` (the File menu's recent saves, each its
//! path and version). A key enters with the code that reads
//! it, so the file never carries a setting nothing honors.

use std::path::PathBuf;

use pes_version::PesVersion;
use serde::{Deserialize, Serialize};

/// `RecentSave.version`'s serde: the one settings entry that names PES
/// 2014 — a recent save is a file the editor opened, not a version a
/// member picks — so its 14 reads where `PesVersion`'s own serde refuses
/// it. Every other number behaves as the enum's does: the 15-to-21 spellings
/// read, an unknown one is an error.
mod recent_version {
    use pes_version::PesVersion;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn serialize<S: Serializer>(
        version: &PesVersion,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        version.serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<PesVersion, D::Error> {
        let number = u16::deserialize(deserializer)?;
        if number == 14 {
            return Ok(PesVersion::Pes14);
        }
        PesVersion::try_from(number).map_err(serde::de::Error::custom)
    }
}

/// A recent save's entry in `recent_files`: its path and the PES version the
/// save was opened or written as, stored so the File menu's `[PES 2021]`
/// label reads no save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RecentSave {
    /// The save's path.
    pub(crate) path: PathBuf,
    /// Its version, the file's own, read at the open or the write.
    #[serde(with = "recent_version")]
    pub(crate) version: PesVersion,
}

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
    /// Appearance screen's "Edit aesthetics anyway" switch; off, they edit as every other
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
    /// The Playable positions grid's layout (the toggle under the panel's
    /// title): `PositionsLayout`'s spellings (`pitch`, `left`, `list`);
    /// an unknown one reads as `pitch`, kept in the file until the member
    /// presses a segment. A view setting, as `scroll_names` is.
    #[serde(default = "default_positions_layout")]
    pub(crate) positions_layout: String,
    /// The team selector's category (the radio buttons under its teams): which of the cup's
    /// team ranges the dropdown and the list show. `TeamCategory`'s spellings
    /// (`all`, `4cc`, `vgl`, `invitational`); an unknown one reads as `all`, kept in the
    /// file until the member picks a category.
    #[serde(default = "default_team_category")]
    pub(crate) team_category: String,
    /// The folder the formation pictures are written to, as Save last wrote one there. A
    /// relative path starts from the program's folder, as the Compiler's folders do; the
    /// default is `formations` there.
    pub(crate) formation_pictures_folder: String,
    /// The formation pictures' "Without the pitch's grain (smaller file)" box, off by
    /// default: on, the pitch is painted in flat bands, which a PNG holds in a smaller file.
    pub(crate) formation_pictures_flat: bool,
    /// The formation pictures' "With each preset's settings beside its row" box, off by
    /// default: on, each preset's settings list beside its row and the team's
    /// Automatic settings on a line under the grid.
    pub(crate) formation_pictures_settings: bool,
    /// The folder the last save was opened from, by whichever of the opens: the file
    /// picker starts there while it exists, so a member who keeps his saves in a folder
    /// of his own is taken back to it. Written on every open, as `scroll_names` is.
    pub(crate) last_folder: Option<PathBuf>,
    /// The last 10 saves opened or saved, newest first: the File menu's and the
    /// welcome panel's Recent files. Written back when the list moves, as
    /// `last_folder` is; a path gone from disk stays in the list, skipped when
    /// shown.
    pub(crate) recent_files: Vec<RecentSave>,
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
            positions_layout: default_positions_layout(),
            team_category: default_team_category(),
            formation_pictures_folder: "formations".to_owned(),
            formation_pictures_flat: false,
            formation_pictures_settings: false,
            last_folder: None,
            recent_files: Vec::new(),
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

/// `positions_layout`'s default: the pitch's rows.
fn default_positions_layout() -> String {
    crate::view::PositionsLayout::Pitch.setting().to_owned()
}

/// `team_category`'s default: every cup team listed.
fn default_team_category() -> String {
    crate::session::selection::TeamCategory::All
        .setting()
        .to_owned()
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

    // A recent save's version is the opened file's own: a PES 2014 save's
    // 14 reads and writes back, the one settings entry naming it, while
    // an unknown number is still refused.
    #[test]
    fn a_pes14_recent_save_reads_and_writes_its_14() {
        let table: toml::Table = toml::from_str(
            "recent_files = [{ path = 'a', version = 14 }, { path = 'b', version = 21 }]",
        )
        .unwrap();
        let read = from_table(&table).unwrap();
        let versions: Vec<PesVersion> = read
            .recent_files
            .iter()
            .map(|recent| recent.version)
            .collect();
        assert_eq!(versions, [PesVersion::Pes14, PesVersion::Pes21]);
        let written = toml::Table::try_from(read).unwrap();
        let files = written["recent_files"].as_array().unwrap();
        assert_eq!(files[0]["version"].as_integer(), Some(14));
        assert_eq!(files[1]["version"].as_integer(), Some(21));

        let bad: toml::Table =
            toml::from_str("recent_files = [{ path = 'a', version = 13 }]").unwrap();
        assert!(from_table(&bad).is_err(), "an unknown version is refused");
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
    fn team_category_defaults_to_all_and_round_trips() {
        assert_eq!(default_table()["team_category"].as_str(), Some("all"));
        let table: toml::Table = toml::from_str("team_category = \"vgl\"").unwrap();
        assert_eq!(from_table(&table).unwrap().team_category, "vgl");
    }

    #[test]
    fn the_formation_pictures_start_in_the_program_s_folder_with_the_grain_and_round_trip() {
        let defaults = default_table();
        assert_eq!(
            defaults["formation_pictures_folder"].as_str(),
            Some("formations"),
            "the formations folder, read from the program's folder"
        );
        assert_eq!(
            defaults["formation_pictures_flat"].as_bool(),
            Some(false),
            "the grain drawn by default"
        );
        let without: toml::Table = toml::from_str("live_aatf = true").unwrap();
        let read = from_table(&without).unwrap();
        assert_eq!(
            (
                read.formation_pictures_folder.as_str(),
                read.formation_pictures_flat,
                read.formation_pictures_settings
            ),
            ("formations", false, false),
            "a file without the keys reads the defaults"
        );
        let table: toml::Table = toml::from_str(
            "formation_pictures_folder = 'D:\\cup\\pictures'\nformation_pictures_flat = true",
        )
        .unwrap();
        let written = toml::Table::try_from(from_table(&table).unwrap()).unwrap();
        assert_eq!(
            written["formation_pictures_folder"].as_str(),
            Some("D:\\cup\\pictures")
        );
        assert_eq!(written["formation_pictures_flat"].as_bool(), Some(true));
    }

    #[test]
    fn the_settings_box_defaults_off_and_round_trips() {
        assert_eq!(
            default_table()
                .get("formation_pictures_settings")
                .and_then(toml::Value::as_bool),
            Some(false),
            "off by default"
        );
        let table: toml::Table = toml::from_str("formation_pictures_settings = true").unwrap();
        let written = toml::Table::try_from(from_table(&table).unwrap()).unwrap();
        assert_eq!(
            written
                .get("formation_pictures_settings")
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
