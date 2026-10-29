//! The common settings: what every tool shares.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pes_version::PesVersion;
use serde::{Deserialize, Serialize};

/// The default PES folder, with `**` standing for the two-digit version.
#[cfg(windows)]
const DEFAULT_PES_FOLDER: &str = r"C:\Program Files (x86)\Pro Evolution Soccer 20**";
/// No sensible default exists off Windows; the user sets it.
#[cfg(not(windows))]
const DEFAULT_PES_FOLDER: &str = "";

/// Settings shared by the shell and every tool, stored under `[common]` in the settings file.
/// Every field has a default, so a missing key loads as its default (this is how a new version's
/// settings appear in an old file).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CommonSettings {
    /// The PES version the suite targets; the sidebar selector.
    pub pes_version: PesVersion,
    /// The PES installation folder, with `**` standing for the two-digit version (`20**`).
    pub pes_folder_path: String,
    /// Per-version PES folders, `[common.pes_folders]` keyed by the version's number
    /// (`21 = '…'`): a filled entry is that version's folder taken as written, overriding
    /// `pes_folder_path`. An entry is never empty: the menu removes an emptied row, and
    /// loading drops a hand-written `19 = ''`, so "any entry" means a filled one
    /// wherever it is asked (the resolution, the menu's toggle, Fix PES path).
    #[serde(
        skip_serializing_if = "BTreeMap::is_empty",
        serialize_with = "folders_by_number",
        deserialize_with = "folders_from_number_keys"
    )]
    pub pes_folders: BTreeMap<PesVersion, String>,
    /// Where exports are picked up. A relative path resolves beside the executable in both
    /// data-location modes, so it sits next to `quick_compile.bat`.
    pub exports_folder_path: PathBuf,
    /// Worker threads for pipelines; `0` means automatic (logical cores minus one).
    pub thread_count: usize,
    /// Memory budget cap for pipelines, as a percentage of physical memory.
    pub memory_cap_percent: f32,
    /// GUI color theme.
    pub theme: Theme,
    /// Whether the shell looks for new releases.
    pub check_for_updates: bool,
    /// Hours between release checks.
    pub check_interval_hours: u32,
    /// When the last release check ran, in seconds since the Unix epoch.
    pub last_update_check: Option<u64>,
    /// A release the user chose to skip; the updater stays quiet about it.
    pub skipped_version: Option<String>,
    /// The tool active when the GUI last closed; `studio` with no arguments reopens it.
    pub last_tool: Option<String>,
    /// The main window class's last place, written when the window closes. `None` until the
    /// first close: the window then opens fitted and centered.
    pub window_main: Option<WindowPlace>,
    /// The boots ID the FPC hide preset writes (the `fpc` crate's `HideIds`), an ID the
    /// game has no boots model for.
    pub fpc_hide_boots_id: u16,
    /// The gloves ID it writes; gloves 1 to 10 are the goalkeepers' real gloves.
    pub fpc_hide_gloves_id: u16,
}

impl Default for CommonSettings {
    fn default() -> Self {
        CommonSettings {
            pes_version: PesVersion::Pes19,
            pes_folder_path: DEFAULT_PES_FOLDER.to_owned(),
            pes_folders: BTreeMap::new(),
            exports_folder_path: PathBuf::from("exports"),
            thread_count: 0,
            memory_cap_percent: 80.0,
            theme: Theme::Dark,
            check_for_updates: true,
            check_interval_hours: 24,
            last_update_check: None,
            skipped_version: None,
            last_tool: None,
            window_main: None,
            fpc_hide_boots_id: fpc::HideIds::CONVENTION.boots(),
            fpc_hide_gloves_id: fpc::HideIds::CONVENTION.gloves(),
        }
    }
}

impl CommonSettings {
    /// The PES installation folder: `pes_version`'s `pes_folders` entry taken as written
    /// (no `**` expansion) when it has one; otherwise `pes_folder_path` with every `**`
    /// replaced by the two-digit number of `pes_version` (`21` for PES 2021).
    pub fn pes_folder(&self) -> PathBuf {
        if let Some(folder) = self.pes_folders.get(&self.pes_version) {
            return PathBuf::from(folder);
        }
        let version = self.pes_version.number().to_string();
        PathBuf::from(self.pes_folder_path.replace("**", &version))
    }

    /// Points the settings at the folder a running PES of `version` runs from (the
    /// running-PES notice's Fix PES path): `version`'s own entry, the folder as it is,
    /// whenever any version has one, since a rewritten main row would move every version
    /// that falls back to it; otherwise the main row, through `pes_folder_pattern`.
    pub(crate) fn fix_pes_folder(&mut self, folder: &Path, version: PesVersion) {
        if self.pes_folders.is_empty() {
            self.pes_folder_path = pes_folder_pattern(folder, version);
        } else {
            self.pes_folders
                .insert(version, folder.display().to_string());
        }
    }

    /// The FPC hide preset's boots and gloves IDs the settings give, or the
    /// `HideIdsError` naming the one outside its band: a value a member set by
    /// hand is not clamped (the settings menu's drag range bounds only what it
    /// enters), so an out-of-band pair is an error the consumers surface.
    pub fn fpc_hide_ids(&self) -> Result<fpc::HideIds, fpc::HideIdsError> {
        fpc::HideIds::new(self.fpc_hide_boots_id, self.fpc_hide_gloves_id)
    }
}

/// `pes_folders`' serializer. TOML keys are strings, and `toml::Value::try_from`, which
/// `Settings::save` builds the file from, refuses an integer key (`KeyNotString`).
fn folders_by_number<S: serde::Serializer>(
    folders: &BTreeMap<PesVersion, String>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_map(
        folders
            .iter()
            .map(|(version, folder)| (version.number().to_string(), folder)),
    )
}

/// `pes_folders`' deserializer, the reverse: `Settings::parse` reads `[common]` from a
/// `toml::Value`, whose table hands the key over as the string `"21"`, which
/// `PesVersion`'s `u16` deserializer refuses. The key is read as `pes_version`'s value
/// is, a number (`21` or `2021`), not through `PesVersion::from_str`, which also takes
/// `PES 2021`: one rule for both keys a member may write by hand. An empty folder is
/// dropped here, so no reader of the map has to tell an empty entry from none.
fn folders_from_number_keys<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<PesVersion, String>, D::Error> {
    BTreeMap::<String, String>::deserialize(deserializer)?
        .into_iter()
        .filter(|(_, folder)| !folder.is_empty())
        .map(|(key, folder)| {
            let number = key.parse::<u16>().map_err(serde::de::Error::custom)?;
            let version = PesVersion::try_from(number).map_err(serde::de::Error::custom)?;
            Ok((version, folder))
        })
        .collect()
}

/// The main row's text for a folder picked or found for `version`: the folder with
/// `version`'s year in its own name (the last path segment) written `20**`, so the
/// version selector keeps switching folders; any other folder unchanged.
pub(crate) fn pes_folder_pattern(folder: &Path, version: PesVersion) -> String {
    let year = version.year().to_string();
    match folder.file_name().and_then(|name| name.to_str()) {
        Some(name) => folder
            .with_file_name(name.replace(&year, "20**"))
            .display()
            .to_string(),
        None => folder.display().to_string(),
    }
}

/// A window's remembered place: its outer position in the screen's physical pixels, its
/// inner size in points at zoom 1 (the pixels over the monitor's scale), the scale factor
/// of the monitor it was on, and whether it was maximized, written in the settings file as
/// `[x, y, width, height, scale, maximized]`. The position is in pixels because a point is a
/// different number of pixels on each monitor, and the OS places windows in pixels. The size
/// is in points at zoom 1, the size a window keeps when the OS moves it to a monitor of
/// another scale; zoomed points would reopen a window closed under the design width wider by
/// the zoom. The first five are the window's normal place even when it was maximized: the
/// place its restore button gives back.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
// Read as a list of any length so a record of five numbers, written before the sixth
// existed, still loads; a fixed `[f32; 6]` would refuse it, and an untagged enum of the
// two array lengths would replace the length error with serde's opaque "did not match".
#[serde(try_from = "Vec<f32>", into = "[f32; 6]")]
pub struct WindowPlace {
    /// The outer rectangle's left edge, in physical pixels.
    pub x: f32,
    /// The outer rectangle's top edge, in physical pixels.
    pub y: f32,
    /// The inner width in points at zoom 1.
    pub width: f32,
    /// The inner height in points at zoom 1.
    pub height: f32,
    /// The monitor's scale factor (native pixels per point) when the place was recorded.
    pub scale: f32,
    /// Whether the window was maximized on its last frame that was not minimized; written
    /// as 1 or 0, and only 1 reads as maximized.
    pub maximized: bool,
}

impl TryFrom<Vec<f32>> for WindowPlace {
    type Error = String;

    fn try_from(numbers: Vec<f32>) -> Result<WindowPlace, String> {
        let (x, y, width, height, scale, maximized) = match numbers[..] {
            [x, y, width, height, scale] => (x, y, width, height, scale, false),
            [x, y, width, height, scale, maximized] => {
                (x, y, width, height, scale, maximized == 1.0)
            }
            _ => {
                return Err(format!(
                    "a window place is 5 or 6 numbers, not {}",
                    numbers.len()
                ));
            }
        };
        Ok(WindowPlace {
            x,
            y,
            width,
            height,
            scale,
            maximized,
        })
    }
}

impl From<WindowPlace> for [f32; 6] {
    fn from(place: WindowPlace) -> [f32; 6] {
        [
            place.x,
            place.y,
            place.width,
            place.height,
            place.scale,
            if place.maximized { 1.0 } else { 0.0 },
        ]
    }
}

impl WindowPlace {
    /// The outer position, in physical pixels.
    pub fn position(&self) -> egui::Pos2 {
        egui::pos2(self.x, self.y)
    }

    /// The inner size, in points at zoom 1.
    pub fn size(&self) -> egui::Vec2 {
        egui::vec2(self.width, self.height)
    }
}

/// GUI color theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Dark background (egui's default).
    Dark,
    /// Light background.
    Light,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(version: PesVersion, path: &str) -> CommonSettings {
        CommonSettings {
            pes_version: version,
            pes_folder_path: path.to_owned(),
            ..CommonSettings::default()
        }
    }

    #[cfg(windows)]
    #[test]
    fn the_default_pes_folder_names_the_default_version() {
        assert_eq!(
            CommonSettings::default().pes_folder(),
            PathBuf::from(r"C:\Program Files (x86)\Pro Evolution Soccer 2019")
        );
    }

    #[test]
    fn the_placeholder_is_replaced_by_the_two_digit_version() {
        assert_eq!(
            settings(PesVersion::Pes21, "/games/Pro Evolution Soccer 20**").pes_folder(),
            PathBuf::from("/games/Pro Evolution Soccer 2021")
        );
    }

    #[test]
    fn a_path_with_no_placeholder_is_unchanged() {
        assert_eq!(
            settings(PesVersion::Pes21, "/games/PES").pes_folder(),
            PathBuf::from("/games/PES")
        );
    }

    // A filled per-version row is that version's folder taken as written: no `**`
    // expansion, and a version without one falls back to the main row's resolution.
    #[test]
    fn a_per_version_folder_overrides_the_main_row_as_written() {
        let mut at_21 = settings(PesVersion::Pes21, "/games/Pro Evolution Soccer 20**");
        at_21
            .pes_folders
            .insert(PesVersion::Pes21, "/a/b/PES2021".to_owned());
        assert_eq!(at_21.pes_folder(), PathBuf::from("/a/b/PES2021"));

        let mut at_19 = at_21.clone();
        at_19.pes_version = PesVersion::Pes19;
        assert_eq!(
            at_19.pes_folder(),
            PathBuf::from("/games/Pro Evolution Soccer 2019"),
            "no row of its own: the main row's resolution"
        );

        at_21
            .pes_folders
            .insert(PesVersion::Pes21, "/a/b/PES20**".to_owned());
        assert_eq!(
            at_21.pes_folder(),
            PathBuf::from("/a/b/PES20**"),
            "an override holding ** is not expanded"
        );
    }

    // The release bundle's settings file opens the window on PES 2017, DBC's game: a key
    // under the wrong table would be ignored and the window would start on PES 2019.
    #[test]
    fn the_bundled_settings_start_on_pes_2017() {
        let settings = crate::settings::Settings::parse(include_str!(
            "../../../../resources/bundle/data/settings.toml"
        ))
        .expect("parses")
        .common;
        assert_eq!(settings.pes_version, PesVersion::Pes17);
    }

    // A hand-written empty entry (`19 = ''`) is dropped on load, so it is no override
    // for the resolution, the menu's toggle or Fix PES path alike.
    #[test]
    fn an_empty_per_version_folder_falls_back_to_the_main_row() {
        let settings = crate::settings::Settings::parse(
            "[common]
pes_version = 19
pes_folder_path = '/games/Pro Evolution Soccer 20**'
\n             [common.pes_folders]
19 = ''
21 = '/games/PES 21'
",
        )
        .expect("parses")
        .common;
        assert_eq!(
            settings.pes_folders.keys().copied().collect::<Vec<_>>(),
            vec![PesVersion::Pes21],
            "the empty entry is dropped, the filled one kept"
        );
        assert_eq!(
            settings.pes_folder(),
            PathBuf::from("/games/Pro Evolution Soccer 2019")
        );
    }

    // The pick's rule: the year anywhere in the folder's own name (the last path
    // segment) is written `20**`; nothing else moves a folder.
    #[cfg(windows)]
    #[test]
    fn pes_folder_pattern_rewrites_only_the_folders_own_year() {
        assert_eq!(
            pes_folder_pattern(
                Path::new(r"D:\Games\Pro Evolution Soccer 2019"),
                PesVersion::Pes19,
            ),
            r"D:\Games\Pro Evolution Soccer 20**"
        );
        assert_eq!(
            pes_folder_pattern(Path::new(r"D:\Games\PES2019 copy"), PesVersion::Pes19),
            r"D:\Games\PES20** copy",
            "the year in the middle of the name"
        );
        assert_eq!(
            pes_folder_pattern(Path::new(r"E:\PES"), PesVersion::Pes19),
            r"E:\PES",
            "a folder without the year is unchanged"
        );
        assert_eq!(
            pes_folder_pattern(Path::new(r"D:\2019\PES"), PesVersion::Pes19),
            r"D:\2019\PES",
            "the year only in a parent segment is unchanged"
        );
        assert_eq!(
            pes_folder_pattern(
                Path::new(r"D:\Games\Pro Evolution Soccer 2021"),
                PesVersion::Pes19,
            ),
            r"D:\Games\Pro Evolution Soccer 2021",
            "another version's year is unchanged"
        );
    }

    // The `/`-separated twin of the Windows case above.
    #[cfg(not(windows))]
    #[test]
    fn pes_folder_pattern_rewrites_only_the_folders_own_year() {
        assert_eq!(
            pes_folder_pattern(
                Path::new("/games/Pro Evolution Soccer 2019"),
                PesVersion::Pes19,
            ),
            "/games/Pro Evolution Soccer 20**"
        );
        assert_eq!(
            pes_folder_pattern(Path::new("/games/PES2019 copy"), PesVersion::Pes19),
            "/games/PES20** copy",
            "the year in the middle of the name"
        );
        assert_eq!(
            pes_folder_pattern(Path::new("/e/PES"), PesVersion::Pes19),
            "/e/PES",
            "a folder without the year is unchanged"
        );
        assert_eq!(
            pes_folder_pattern(Path::new("/2019/PES"), PesVersion::Pes19),
            "/2019/PES",
            "the year only in a parent segment is unchanged"
        );
        assert_eq!(
            pes_folder_pattern(
                Path::new("/games/Pro Evolution Soccer 2021"),
                PesVersion::Pes19,
            ),
            "/games/Pro Evolution Soccer 2021",
            "another version's year is unchanged"
        );
    }

    // Fix PES path: with no per-version rows the main row takes the pattern; once any
    // version has one the running version's own row takes the folder as it is, since a
    // rewritten main row would move every version that falls back to it.
    #[test]
    fn fix_pes_folder_writes_the_main_row_until_an_override_exists() {
        let mut bare = settings(PesVersion::Pes19, "/member/set/folder");
        bare.fix_pes_folder(
            Path::new("/running/Pro Evolution Soccer 2019"),
            PesVersion::Pes19,
        );
        assert_eq!(
            PathBuf::from(&bare.pes_folder_path),
            Path::new("/running/Pro Evolution Soccer 2019")
                .with_file_name("Pro Evolution Soccer 20**"),
            "no entries: the main row takes the pattern"
        );
        assert!(bare.pes_folders.is_empty(), "no entry appeared");

        let mut with_override = settings(PesVersion::Pes19, "/member/set/folder");
        with_override
            .pes_folders
            .insert(PesVersion::Pes21, "/elsewhere/PES2021".to_owned());
        let folder = Path::new("/running/Pro Evolution Soccer 2019");
        assert_ne!(
            with_override.pes_folder_path,
            pes_folder_pattern(folder, PesVersion::Pes19),
            "a wrong branch writes the pattern to the main row and still passes"
        );
        with_override.fix_pes_folder(folder, PesVersion::Pes19);
        assert_eq!(
            with_override.pes_folders.get(&PesVersion::Pes19),
            Some(&folder.display().to_string()),
            "the folder as given, its year kept"
        );
        assert_eq!(
            with_override.pes_folders.get(&PesVersion::Pes21),
            Some(&"/elsewhere/PES2021".to_owned()),
            "the other entry kept"
        );
        assert_eq!(
            with_override.pes_folder_path, "/member/set/folder",
            "the main row unchanged"
        );
    }

    // A settings file without `pes_folders` saves byte for byte: the
    // `skip_serializing_if` keeps an empty map from appearing as a table.
    #[test]
    fn a_file_without_per_version_folders_saves_unchanged() {
        let dir = std::env::temp_dir().join(format!(
            "studio_core_common_no_folders_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        crate::settings::Settings::default().save(&path).unwrap();
        let original = std::fs::read_to_string(&path).unwrap();
        assert!(
            !original.contains("pes_folders"),
            "no empty table appears: {original}"
        );

        let loaded = crate::settings::Settings::load(&path).unwrap();
        assert!(loaded.common.pes_folders.is_empty());
        loaded.save(&path).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            original,
            "nothing changed: the file is byte-identical"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // A filled row's table is keyed by the version's number (`21 = '…'`), loads back
    // as the same map, and is gone from the file once emptied.
    #[test]
    fn per_version_folders_round_trip_and_disappear_when_emptied() {
        let dir =
            std::env::temp_dir().join(format!("studio_core_common_folders_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        let mut settings = crate::settings::Settings::default();
        settings
            .common
            .pes_folders
            .insert(PesVersion::Pes21, "/games/PES 21".to_owned());
        settings.save(&path).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("[common.pes_folders]"), "the table: {text}");
        let file: toml::Table = toml::from_str(&text).unwrap();
        let folders = file["common"]["pes_folders"].as_table().unwrap();
        assert_eq!(folders.len(), 1, "keyed by the version's number");
        assert_eq!(folders["21"].as_str(), Some("/games/PES 21"));
        let loaded = crate::settings::Settings::load(&path).unwrap();
        assert_eq!(
            loaded.common.pes_folders, settings.common.pes_folders,
            "load returns the same map"
        );

        let mut emptied = loaded;
        emptied.common.pes_folders.clear();
        emptied.save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            !text.contains("pes_folders"),
            "the emptied table leaves the file: {text}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // A per-version key reads as `pes_version`'s value does: the number or the year
    // (`21`, `2021`), never another word (`PES 2021`), and a version the suite does
    // not know refuses the file as `pes_version = 22` does.
    #[test]
    fn a_per_version_key_reads_as_pes_version_does() {
        for key in ["21", "2021"] {
            let settings = crate::settings::Settings::parse(&format!(
                "[common.pes_folders]\n{key} = '/games/PES 21'\n"
            ))
            .expect("parses");
            assert_eq!(
                settings.common.pes_folders.get(&PesVersion::Pes21),
                Some(&"/games/PES 21".to_owned()),
                "key {key}"
            );
        }
        assert!(
            crate::settings::Settings::parse("[common.pes_folders]\n'PES 2021' = '/games'\n")
                .is_err(),
            "a word is not a key, as for pes_version"
        );
        assert!(
            crate::settings::Settings::parse("[common]\npes_version = 'PES 2021'\n").is_err(),
            "pes_version refuses the word too"
        );
        assert!(
            crate::settings::Settings::parse("[common.pes_folders]\n22 = '/games'\n").is_err(),
            "an unknown version refuses the file"
        );
    }

    // The defaults the spec pins: `spec/team_compiler/settings.md`'s "Common
    // settings (studio_core)" table (version 19, the `20**` folder, `exports/`,
    // 0 auto threads, 80.0, the update check) and `spec/core/distribution.md`'s
    // "Update state" table (`check_for_updates` true, `check_interval_hours` 24,
    // `last_update_check` and `skipped_version` empty). `theme` is the dark
    // preset, the default the GUI phase plans, `last_tool` empty on a first
    // run, `window_main` empty until the first close. The hide IDs are the FPC
    // convention's 55 and 11 (`fpc::HideIds::CONVENTION`).
    #[test]
    fn the_defaults_match_the_settings_tables() {
        // `pes_folder_path` is the shipped pattern on Windows and empty
        // elsewhere: no sensible default exists off Windows.
        #[cfg(windows)]
        let pes_folder_path = r"C:\Program Files (x86)\Pro Evolution Soccer 20**";
        #[cfg(not(windows))]
        let pes_folder_path = "";
        assert_eq!(
            CommonSettings::default(),
            CommonSettings {
                pes_version: PesVersion::Pes19,
                pes_folder_path: pes_folder_path.to_owned(),
                pes_folders: BTreeMap::new(),
                exports_folder_path: PathBuf::from("exports"),
                thread_count: 0,
                memory_cap_percent: 80.0,
                theme: Theme::Dark,
                check_for_updates: true,
                check_interval_hours: 24,
                last_update_check: None,
                skipped_version: None,
                last_tool: None,
                window_main: None,
                fpc_hide_boots_id: 55,
                fpc_hide_gloves_id: 11,
            }
        );
    }

    // A settings file whose `[common]` lacks both keys loads the
    // convention's 55 and 11 (the struct's `serde(default)`); a key set by
    // hand loads as written, clamped nowhere, and `fpc_hide_ids` reports
    // the band the value left.
    #[test]
    fn the_hide_ids_default_to_the_convention_and_load_as_written() {
        let settings = crate::settings::Settings::parse("[common]\npes_version = 21\n")
            .expect("parses")
            .common;
        assert_eq!(settings.fpc_hide_boots_id, 55);
        assert_eq!(settings.fpc_hide_gloves_id, 11);
        assert_eq!(
            settings.fpc_hide_ids(),
            Ok(fpc::HideIds::CONVENTION),
            "the defaults resolve to the convention"
        );

        let settings = crate::settings::Settings::parse(
            "[common]\nfpc_hide_boots_id = 627\nfpc_hide_gloves_id = 12\n",
        )
        .expect("parses")
        .common;
        assert_eq!(settings.fpc_hide_boots_id, 627, "as written, no clamp");
        assert_eq!(settings.fpc_hide_gloves_id, 12);
        assert_eq!(settings.fpc_hide_ids(), Err(fpc::HideIdsError::Boots(627)));
    }

    /// The `window_main` value of `text`'s `[common]` section, as `toml` reads it back.
    fn window_main_value(text: &str) -> toml::Value {
        let root: toml::Table = toml::from_str(text).unwrap();
        root["common"]["window_main"].clone()
    }

    // The window's place reads from the five numbers a record written before
    // the sixth existed holds, as not maximized, and writes back six, its key
    // inside `[common]`.
    #[test]
    fn a_window_place_loads_from_five_numbers_and_saves_as_six() {
        let settings =
            crate::settings::Settings::parse("[common]\nwindow_main = [100, 50, 1100, 700, 1]\n")
                .expect("parses");
        let place = WindowPlace {
            x: 100.0,
            y: 50.0,
            width: 1100.0,
            height: 700.0,
            scale: 1.0,
            maximized: false,
        };
        assert_eq!(settings.common.window_main, Some(place));

        let dir = std::env::temp_dir().join(format!(
            "studio_core_common_window_place_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        settings.save(&path).unwrap();
        let loaded = crate::settings::Settings::load(&path).unwrap();
        assert_eq!(loaded.common.window_main, Some(place));

        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            window_main_value(&text),
            toml::Value::try_from([100.0, 50.0, 1100.0, 700.0, 1.0, 0.0]).unwrap(),
            "six numbers, the sixth 0"
        );
        let common_at = text.find("[common]").expect("a [common] section");
        let key_at = text.find("window_main").expect("the place is written");
        assert!(key_at > common_at, "the key sits inside [common]");
        if let Some(next) = text[common_at..].find("\n[").map(|i| common_at + i) {
            assert!(key_at < next, "the key sits before the next section");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // A record of six numbers round-trips its sixth: 1 as maximized, 0 as not.
    #[test]
    fn a_window_place_of_six_numbers_round_trips_the_maximized_flag() {
        let dir = std::env::temp_dir().join(format!(
            "studio_core_common_window_place_six_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        for (flag, maximized) in [(1, true), (0, false)] {
            let text = format!("[common]\nwindow_main = [100, 50, 1100, 700, 1, {flag}]\n");
            let settings = crate::settings::Settings::parse(&text).expect("parses");
            assert_eq!(
                settings.common.window_main,
                Some(WindowPlace {
                    x: 100.0,
                    y: 50.0,
                    width: 1100.0,
                    height: 700.0,
                    scale: 1.0,
                    maximized,
                }),
                "a sixth number of {flag}"
            );
            settings.save(&path).unwrap();
            let written = std::fs::read_to_string(&path).unwrap();
            assert_eq!(
                window_main_value(&written),
                toml::Value::try_from([100.0, 50.0, 1100.0, 700.0, 1.0, f64::from(flag)]).unwrap(),
                "a sixth number of {flag} is written back"
            );
            std::fs::remove_file(&path).unwrap();
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // A record of any other length is refused, as a malformed value of any
    // other key is: the settings file does not parse.
    #[test]
    fn a_window_place_of_four_or_seven_numbers_is_refused() {
        for numbers in ["[100, 50, 1100, 700]", "[100, 50, 1100, 700, 1, 0, 0]"] {
            let text = format!("[common]\nwindow_main = {numbers}\n");
            let error = crate::settings::Settings::parse(&text)
                .expect_err("a place of the wrong length is refused");
            let crate::settings::SettingsError::Parse(error) = error else {
                panic!("{numbers}: a parse error, not {error:?}");
            };
            assert!(
                error
                    .to_string()
                    .contains("a window place is 5 or 6 numbers"),
                "{numbers}: {error}"
            );
        }
    }
}
