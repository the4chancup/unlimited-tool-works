//! The settings framework: one TOML file holding the common settings under `[common]` and one
//! table per tool under its id (`[team-compiler]`). Loading fills missing common keys with their
//! defaults; each tool's `default_settings()` is merged into its table the same way, which is what
//! carries settings across versions without a migration step.
//!
//! Saving is the caller's policy: this module writes the file and reports failure; the shell
//! keeps the in-memory settings and raises the `data_dir_read_only` condition.

mod common;
#[cfg(not(target_arch = "wasm32"))]
mod location;

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) use common::pes_folder_pattern;
pub use common::{CommonSettings, Theme, WindowPlace};
#[cfg(not(target_arch = "wasm32"))]
pub use location::{SETTINGS_FILE_NAME, resolve_data_dir, settings_file_writable, user_config_dir};
use log::debug;
use toml::{Table, Value};
use toml_edit::{DocumentMut, Item as EditItem, Table as EditTable};

/// The key of the common section; no tool may use it as its id.
pub const COMMON_KEY: &str = "common";

/// Everything the settings file holds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    /// The `[common]` section.
    pub common: CommonSettings,
    /// One table per tool, keyed by tool id, exactly as read (unknown keys included).
    tools: BTreeMap<String, Table>,
}

/// Why the settings file could not be read.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// The file exists but could not be read.
    #[error("cannot read the settings file")]
    Io(#[from] io::Error),
    /// The file is not valid TOML, or a section has the wrong shape.
    #[error("cannot understand the settings file")]
    Parse(#[from] toml::de::Error),
    /// A top-level key other than `common` is not a table (`foo = 1` at the root).
    #[error("settings key `{0}` is not a section")]
    NotASection(String),
}

impl Settings {
    /// Reads the settings file. A missing file is not an error: it yields the defaults, and the
    /// first save creates it.
    pub fn load(path: &Path) -> Result<Settings, SettingsError> {
        match fs::read_to_string(path) {
            Ok(text) => Settings::parse(&text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(error) => Err(error.into()),
        }
    }

    /// Parses settings-file text.
    pub fn parse(text: &str) -> Result<Settings, SettingsError> {
        let mut root: Table = toml::from_str(text)?;
        let common = match root.remove(COMMON_KEY) {
            Some(value) => value.try_into()?,
            None => CommonSettings::default(),
        };
        let mut tools = BTreeMap::new();
        for (key, value) in root {
            match value {
                Value::Table(table) => {
                    tools.insert(key, table);
                }
                _ => return Err(SettingsError::NotASection(key)),
            }
        }
        Ok(Settings { common, tools })
    }

    /// Writes the settings file atomically: to a sibling `<path>.<pid>-<n>.tmp`, then renamed
    /// over `path`, so a failure mid-write never leaves a truncated file behind, and two saves
    /// at once (two Studio processes, the GUI open while `quick_compile.bat` starts another; or
    /// two threads of one) never share a temporary file. An existing file is edited in place:
    /// a save moves only the values that changed, so comments, blank lines and key order
    /// written by hand survive.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        static SAVE_COUNTER: AtomicU64 = AtomicU64::new(0);
        let desired = self.to_table().map_err(io::Error::other)?;
        let regenerated = toml::to_string_pretty(&desired).map_err(io::Error::other)?;
        // A file with nothing to keep is written fresh: none yet, one that is not text, or one
        // that is not TOML (broken by hand since the session loaded it). The session's values
        // win there as they win over any hand edit made while the GUI runs, and a folder that
        // cannot be written fails the write below, whatever the read said. So is a file whose
        // edit would not read back as the settings: a shape the in-place edit cannot carry,
        // such as a section written inline under a comment, which would come out as a table
        // header `load` refuses and the next start could not open.
        let text = match fs::read_to_string(path) {
            Ok(existing) => match (existing.parse::<DocumentMut>(), toml::from_str(&existing)) {
                (Ok(mut document), Ok(held)) => {
                    let written = regenerated
                        .parse::<DocumentMut>()
                        .expect("`toml`'s own output is valid TOML");
                    apply_desired(document.as_table_mut(), &held, written.as_table(), &desired);
                    let edited = document.to_string();
                    match toml::from_str::<Table>(&edited) {
                        Ok(read_back) if read_back == desired => edited,
                        _ => {
                            debug!(
                                "{}: written fresh, the edit did not read back",
                                path.display()
                            );
                            regenerated
                        }
                    }
                }
                _ => {
                    debug!("{}: written fresh, not TOML", path.display());
                    regenerated
                }
            },
            Err(error) => {
                debug!("{}: written fresh, not edited: {error}", path.display());
                regenerated
            }
        };
        let mut temp_name = path.file_name().unwrap_or_default().to_owned();
        let sequence = SAVE_COUNTER.fetch_add(1, Ordering::Relaxed);
        temp_name.push(format!(".{}-{sequence}.tmp", std::process::id()));
        let temp = path.with_file_name(temp_name);
        let write_and_swap = || -> io::Result<()> {
            fs::write(&temp, text)?;
            fs::rename(&temp, path)
        };
        if let Err(error) = write_and_swap() {
            // Best effort: the original error is the one to report.
            if let Err(cleanup_error) = fs::remove_file(&temp) {
                debug!(
                    "{}: the leftover temporary file was not removed: {cleanup_error}",
                    temp.display()
                );
            }
            return Err(error);
        }
        Ok(())
    }

    /// The file's values: `[common]` first, then one table per tool in id order.
    fn to_table(&self) -> Result<Table, toml::ser::Error> {
        let mut root = Table::new();
        root.insert(COMMON_KEY.to_owned(), Value::try_from(&self.common)?);
        for (id, table) in &self.tools {
            root.insert(id.clone(), Value::Table(table.clone()));
        }
        Ok(root)
    }

    /// A tool's section, if the file had one.
    pub fn tool(&self, tool_id: &str) -> Option<&Table> {
        self.tools.get(tool_id)
    }

    /// Replaces a tool's section.
    ///
    /// # Panics
    /// If `tool_id` is the reserved `common` key: tool ids are compile-time constants, so this is
    /// a programming error, not a runtime condition.
    pub fn set_tool(&mut self, tool_id: &str, table: Table) {
        assert_reserved_free(tool_id);
        self.tools.insert(tool_id.to_owned(), table);
    }

    /// Fills a tool's section with every default it lacks, recursing into nested tables. Values
    /// already present are kept, including ones no default names.
    ///
    /// # Panics
    /// If `tool_id` is the reserved `common` key (see [`Settings::set_tool`]).
    pub fn merge_defaults(&mut self, tool_id: &str, defaults: &Table) {
        assert_reserved_free(tool_id);
        let table = self.tools.entry(tool_id.to_owned()).or_default();
        merge_missing(table, defaults);
    }
}

/// Brings `document` (the file as written, whose values are `held`) to the settings' values
/// `desired`, whose freshly written form is `written`, touching only what differs. The values
/// are compared as `toml` values, not as `toml_edit` items, whose decor is part of them: a
/// table both sides hold is recursed into; a value `held` already equals is left alone, so its
/// line stays byte for byte; any other is copied from `written`; a key `desired` lacks is
/// removed, and one the file lacks lands at the end of its table.
fn apply_desired(document: &mut EditTable, held: &Table, written: &EditTable, desired: &Table) {
    for key in held.keys().filter(|key| !desired.contains_key(*key)) {
        document.remove(key);
    }
    for (key, wanted) in desired {
        let fresh = written
            .get(key)
            .expect("`written` is `desired` written out, so it holds every key");
        match (held.get(key), wanted, document.get_mut(key)) {
            (Some(Value::Table(held)), Value::Table(wanted), Some(EditItem::Table(current))) => {
                let fresh = fresh.as_table().expect("a table is written as a table");
                apply_desired(current, held, fresh, wanted);
            }
            (Some(held), _, Some(_)) if held == wanted => {}
            // Replaced through the slot, not `insert`: `insert` resets an existing key's
            // decor, which holds the comment above it. The old value's own decor (its trailing
            // comment) moves onto the new value.
            (_, _, Some(slot)) => {
                let decor = slot.as_value().map(|value| value.decor().clone());
                *slot = fresh.clone();
                if let (Some(decor), Some(value)) = (decor, slot.as_value_mut()) {
                    *value.decor_mut() = decor;
                }
            }
            (_, _, None) => {
                document.insert(key, fresh.clone());
            }
        }
    }
}

fn assert_reserved_free(tool_id: &str) {
    assert_ne!(
        tool_id, COMMON_KEY,
        "`{COMMON_KEY}` is the common section, not a tool id"
    );
}

fn merge_missing(into: &mut Table, defaults: &Table) {
    for (key, default) in defaults {
        match (into.get_mut(key), default) {
            (None, _) => {
                into.insert(key.clone(), default.clone());
            }
            (Some(Value::Table(existing)), Value::Table(nested)) => merge_missing(existing, nested),
            (Some(_), _) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pes_version::PesVersion;

    fn table(text: &str) -> Table {
        toml::from_str(text).unwrap()
    }

    /// A temp folder unique to this test process, as `location.rs`'s `sandbox` does: two
    /// test binaries running at once (the mutation run's workers) must not share fixtures.
    fn test_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("{name}_{}", std::process::id()))
    }

    #[test]
    fn missing_file_loads_as_defaults() {
        let dir = test_dir("studio_core_settings_missing");
        let loaded = Settings::load(&dir.join("does_not_exist.toml")).unwrap();
        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn a_settings_path_that_exists_but_is_not_a_file_is_an_error() {
        let dir = test_dir("studio_core_settings_dir_path");
        fs::create_dir_all(&dir).unwrap();
        // Only a missing file yields the defaults; a folder at the path is a read failure.
        assert!(matches!(
            Settings::load(&dir),
            Err(SettingsError::Io(error)) if error.kind() != io::ErrorKind::NotFound
        ));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn save_then_load_round_trips() {
        let mut settings = Settings::default();
        settings.common.pes_version = PesVersion::Pes21;
        settings.common.thread_count = 4;
        settings.common.last_tool = Some("save-editor".to_owned());
        settings.set_tool(
            "team-compiler",
            table("dt00_overwrite_allow = true\nlabel = 'x'"),
        );

        let dir = test_dir("studio_core_settings_roundtrip");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        settings.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert_eq!(loaded, settings);
        assert_eq!(
            fs::read_dir(&dir).unwrap().count(),
            1,
            "no temporary file left behind"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn concurrent_saves_leave_a_complete_file() {
        let dir = test_dir("studio_core_settings_concurrent");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        std::thread::scope(|scope| {
            for thread_count in 1..=8 {
                let path = &path;
                scope.spawn(move || {
                    let mut settings = Settings::default();
                    settings.common.thread_count = thread_count;
                    settings.save(path).unwrap();
                });
            }
        });
        let loaded = Settings::load(&path).unwrap();
        assert!((1..=8).contains(&loaded.common.thread_count));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_failed_save_leaves_no_tmp_sibling() {
        let dir = test_dir("studio_core_settings_save_fail");
        drop(fs::remove_dir_all(&dir));
        fs::create_dir_all(&dir).unwrap();
        // `path` is a directory: the rename over it fails after the .tmp was written.
        let path = dir.join("settings.toml");
        fs::create_dir(&path).unwrap();

        assert!(Settings::default().save(&path).is_err());
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "no .tmp sibling left behind");

        fs::remove_dir_all(&dir).unwrap();
    }

    // A save edits the file in place: hand-written comments, blank lines and key order survive
    // a `last_tool` save.
    #[test]
    fn save_edits_in_place_and_only_the_changed_line_moves() {
        let dir = test_dir("studio_core_settings_inplace");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        let original = "# the member's note about this file\n\
                        [common]\n\
                        # the version to target\n\
                        pes_version = 19\n\
                        thread_count = 4\n\
                        last_tool = \"team-compiler\"\n\
                        theme = \"dark\"\n\
                        pes_folder_path = '/games/Pro Evolution Soccer 20**'\n\
                        exports_folder_path = \"exports\"\n\
                        memory_cap_percent = 80.0\n\
                        check_for_updates = true\n\
                        check_interval_hours = 24\n\
                        \n\
                        [team-compiler]\n\
                        # its own note\n\
                        label = 'x'\n\
                        dt00_overwrite_allow = true\n";
        fs::write(&path, original).unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings.common.last_tool = Some("save-editor".to_owned());
        settings.save(&path).unwrap();

        let expected = original
            .replace(
                "last_tool = \"team-compiler\"",
                "last_tool = \"save-editor\"",
            )
            .replace(
                "check_interval_hours = 24\n",
                "check_interval_hours = 24\n\
                 fpc_hide_boots_id = 55\n\
                 fpc_hide_gloves_id = 11\n\
                 invert_value_scrolling = false\n",
            );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            expected,
            "only the changed line moved"
        );
        assert_eq!(Settings::load(&path).unwrap(), settings);

        // A key the file lacks is added where its section ends.
        let path = dir.join("settings_missing_key.toml");
        let original = "[common]\n\
                        check_for_updates = true\n\
                        check_interval_hours = 24\n\
                        exports_folder_path = \"exports\"\n\
                        memory_cap_percent = 80.0\n\
                        pes_folder_path = '/games/Pro Evolution Soccer 20**'\n\
                        pes_version = 19\n\
                        theme = \"dark\"\n\
                        thread_count = 4\n\
                        \n\
                        [team-compiler]\n\
                        label = 'x'\n";
        fs::write(&path, original).unwrap();
        let mut settings = Settings::load(&path).unwrap();
        settings.common.last_tool = Some("save-editor".to_owned());
        settings.save(&path).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            original.replacen(
                "thread_count = 4\n",
                "thread_count = 4\n\
                 fpc_hide_boots_id = 55\nfpc_hide_gloves_id = 11\n\
                 invert_value_scrolling = false\n\
                 last_tool = \"save-editor\"\n",
                1
            ),
            "the absent key is appended where its section ends"
        );
        assert_eq!(Settings::load(&path).unwrap(), settings);
        fs::remove_dir_all(&dir).unwrap();
    }

    // A section a member wrote inline, under a comment, is a shape the in-place edit cannot
    // carry (the comment would land inside the new table header): the save writes the file
    // fresh rather than one that `load` refuses, so the next start still opens.
    #[test]
    fn save_writes_fresh_when_the_edit_would_not_read_back() {
        let dir = test_dir("studio_core_settings_inline_section");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        fs::write(&path, "# my cup settings\ncommon = { pes_version = 21 }\n").unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings.common.last_tool = Some("save-editor".to_owned());
        settings.save(&path).unwrap();

        assert_eq!(
            Settings::load(&path).unwrap(),
            settings,
            "the saved file loads back: {}",
            fs::read_to_string(&path).unwrap()
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    // A file broken by hand since the session loaded it is written fresh from the session.
    #[test]
    fn save_writes_fresh_over_a_file_that_does_not_parse() {
        let dir = test_dir("studio_core_settings_broken_file");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        let mut settings = Settings::default();
        settings.common.last_tool = Some("save-editor".to_owned());
        fs::write(&path, "[common\npes_version = 21\n").unwrap();

        settings.save(&path).unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            toml::to_string_pretty(&settings.to_table().unwrap()).unwrap()
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    // A tool with no settings keeps an empty table, and a nested table is edited key by key:
    // the comment inside it survives a change of its sibling.
    #[test]
    fn save_writes_an_empty_tool_table_and_edits_a_nested_one_in_place() {
        let dir = test_dir("studio_core_settings_nested");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        let original = "[team-compiler.kits]\n\
                        # kept\n\
                        collar = 3\n\
                        sleeves = 1\n";
        fs::write(&path, original).unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings.merge_defaults("export-upgrader", &Table::new());
        let mut tool = settings.tool("team-compiler").unwrap().clone();
        tool["kits"]
            .as_table_mut()
            .unwrap()
            .insert("sleeves".to_owned(), Value::Integer(2));
        settings.set_tool("team-compiler", tool);
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("[team-compiler.kits]\n# kept\ncollar = 3\nsleeves = 2\n"),
            "the nested table changed in place: {text}"
        );
        assert!(
            text.contains("[export-upgrader]"),
            "the empty table is written: {text}"
        );
        assert_eq!(Settings::load(&path).unwrap(), settings);
        fs::remove_dir_all(&dir).unwrap();
    }

    // A changed value keeps the comment above its key and the one after it: a pick in the
    // version selector rewrites `pes_version`, the key a member most likely annotated.
    #[test]
    fn save_keeps_the_comments_around_a_changed_value() {
        let dir = test_dir("studio_core_settings_changed_comments");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        let original = "[common]\n\
                        # the version to target\n\
                        pes_version = 19 # set by hand\n\
                        thread_count = 4\n\
                        theme = \"dark\"\n\
                        pes_folder_path = '/games/Pro Evolution Soccer 20**'\n\
                        exports_folder_path = \"exports\"\n\
                        memory_cap_percent = 80.0\n\
                        check_for_updates = true\n\
                        check_interval_hours = 24\n";
        fs::write(&path, original).unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings.common.pes_version = PesVersion::Pes21;
        settings.save(&path).unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            original
                .replace("pes_version = 19 #", "pes_version = 21 #")
                .replace(
                    "check_interval_hours = 24\n",
                    "check_interval_hours = 24\n\
                     fpc_hide_boots_id = 55\nfpc_hide_gloves_id = 11\n\
                     invert_value_scrolling = false\n"
                ),
            "the comments around the changed value survive"
        );
        assert_eq!(Settings::load(&path).unwrap(), settings);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn save_removes_dropped_keys_and_appends_new_sections() {
        let dir = test_dir("studio_core_settings_inplace_keys");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        let original = "[common]\n\
                        pes_version = 19\n\
                        thread_count = 0\n\
                        memory_cap_percent = 80.0\n\
                        exports_folder_path = \"exports\"\n\
                        pes_folder_path = '/games/Pro Evolution Soccer 20**'\n\
                        theme = \"dark\"\n\
                        check_for_updates = true\n\
                        check_interval_hours = 24\n\
                        last_tool = \"team-compiler\"\n\
                        \n\
                        [team-compiler]\n\
                        label = 'x'\n\
                        stale = 1\n";
        fs::write(&path, original).unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings.set_tool("team-compiler", table("label = 'x'"));
        settings.set_tool("save-editor", table("speed = 1"));
        settings.save(&path).unwrap();

        let expected = "[common]\n\
                        pes_version = 19\n\
                        thread_count = 0\n\
                        memory_cap_percent = 80.0\n\
                        exports_folder_path = \"exports\"\n\
                        pes_folder_path = '/games/Pro Evolution Soccer 20**'\n\
                        theme = \"dark\"\n\
                        check_for_updates = true\n\
                        check_interval_hours = 24\n\
                        last_tool = \"team-compiler\"\n\
                        fpc_hide_boots_id = 55\n\
                        fpc_hide_gloves_id = 11\n\
                        invert_value_scrolling = false\n\
                        \n\
                        [team-compiler]\n\
                        label = 'x'\n\
                        \n\
                        [save-editor]\n\
                        speed = 1\n";
        assert_eq!(fs::read_to_string(&path).unwrap(), expected);
        assert_eq!(Settings::load(&path).unwrap(), settings);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_common_keys_take_defaults_and_unknown_tool_keys_survive() {
        let loaded =
            Settings::parse("[common]\npes_version = 16\n[refs-arranger]\nfuture = 1\n").unwrap();
        assert_eq!(loaded.common.pes_version, PesVersion::Pes16);
        assert_eq!(
            loaded.common.thread_count,
            CommonSettings::default().thread_count
        );
        assert_eq!(
            loaded.tool("refs-arranger").unwrap()["future"],
            Value::Integer(1)
        );
        assert!(loaded.tool("team-compiler").is_none());
    }

    #[test]
    fn merge_defaults_fills_missing_keys_only() {
        let mut settings = Settings::default();
        settings.set_tool("tool", table("kept = 1\n[nested]\nkept = 'a'"));
        settings.merge_defaults(
            "tool",
            &table("kept = 99\nadded = 2\n[nested]\nkept = 'z'\nadded = 'b'\n[deep]\nx = 1"),
        );
        let tool = settings.tool("tool").unwrap();
        assert_eq!(tool["kept"], Value::Integer(1));
        assert_eq!(tool["added"], Value::Integer(2));
        assert_eq!(tool["nested"]["kept"], Value::String("a".to_owned()));
        assert_eq!(tool["nested"]["added"], Value::String("b".to_owned()));
        assert_eq!(tool["deep"]["x"], Value::Integer(1));
    }

    #[test]
    fn merge_defaults_creates_the_section() {
        let mut settings = Settings::default();
        settings.merge_defaults("new-tool", &table("a = 1"));
        assert_eq!(settings.tool("new-tool").unwrap()["a"], Value::Integer(1));
    }

    #[test]
    #[should_panic(expected = "not a tool id")]
    fn the_common_key_is_not_a_tool_section() {
        Settings::default().set_tool(COMMON_KEY, Table::new());
    }

    #[test]
    fn root_scalar_is_an_error() {
        assert!(matches!(
            Settings::parse("stray = 1"),
            Err(SettingsError::NotASection(key)) if key == "stray"
        ));
    }
}
