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
#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// The `[common]` section.
    pub common: CommonSettings,
    /// One table per tool, keyed by tool id, exactly as read (unknown keys included).
    tools: BTreeMap<String, Table>,
    /// The session's values, `to_table()`'s form, as of the last `parse` or successful
    /// `save`: what a save tells the session's own changes from the file's. `None` for
    /// settings that came from no file (`Settings::default()`).
    baseline: Option<Table>,
    /// The text a fresh write starts from (`with_template`): the bundled settings file, so a
    /// written file carries its comments. `None` writes the bare values.
    template: Option<&'static str>,
}

impl PartialEq for Settings {
    // `baseline` is bookkeeping for `save`, not part of what the settings hold: a
    // settings loaded from a file equals one built to the same values.
    fn eq(&self, other: &Self) -> bool {
        self.common == other.common && self.tools == other.tools && self.template == other.template
    }
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
        let mut settings = Settings {
            common,
            tools,
            baseline: None,
            template: None,
        };
        // `to_table` fails only on a `common` value TOML cannot write, which `save`
        // reports before `baseline` is ever read; `None` just writes the whole state.
        settings.baseline = settings.to_table().ok();
        Ok(settings)
    }

    /// Sets the text a fresh write of `save` starts from: the bundled settings file, so the
    /// written file carries its comments, the session's values replacing the template's own.
    /// A file that exists and parses is edited in place, template or not.
    pub fn with_template(mut self, text: &'static str) -> Settings {
        self.template = Some(text);
        self
    }

    /// Writes the settings file atomically: to a sibling `<path>.<pid>-<n>.tmp`, then renamed
    /// over `path`, so a failure mid-write never leaves a truncated file behind, and two saves
    /// at once (two Studio processes, the GUI open while `quick_compile.bat` starts another; or
    /// two threads of one) never share a temporary file. An existing file is edited in place:
    /// comments, blank lines and key order written by hand survive.
    ///
    /// A save writes only what this session changed since it last read the file or last
    /// wrote it (`baseline`), not the session's whole state: two windows open at once each
    /// hold their own copy, and a save of the whole copy puts back every value the other
    /// window changed meanwhile. So a `desired` value that still equals `baseline`'s is no
    /// change of the session's, and the file's own value stands, its absence included; one
    /// that differs is the session's change and is written, its absence included. With no
    /// baseline, or a file missing or not TOML, the session's whole state is written.
    ///
    /// After a successful write `baseline` becomes the session's own values (`desired`), not
    /// the file's: a value another window changed still differs from the session's copy, and
    /// remembering the file's value would have the next save write the session's stale copy
    /// back over it. After a failed write `baseline` stays as it was, so the next save still
    /// writes the change.
    pub fn save(&mut self, path: &Path) -> io::Result<()> {
        static SAVE_COUNTER: AtomicU64 = AtomicU64::new(0);
        let desired = self.to_table().map_err(io::Error::other)?;
        let regenerated = toml::to_string_pretty(&desired).map_err(io::Error::other)?;
        let written = regenerated
            .parse::<DocumentMut>()
            .expect("`toml`'s own output is valid TOML");
        // A file with nothing to keep is written fresh: none yet, one that is not text, or one that
        // is not TOML (broken by hand since the session loaded it). The fresh text is the
        // `template`'s edited to the session's values when one is set and its own edit reads back —
        // the file then carries its comments — and the values alone otherwise. The session's values
        // win there as they win over any hand edit made while the GUI runs, and a folder that
        // cannot be written fails the write below, whatever the read said. So is a file whose edit
        // would not read back as the settings: a shape the in-place edit cannot carry, such as a
        // section written inline under a comment, which would come out as a table header `load`
        // refuses and the next start could not open.
        let fresh = |why: &str| match self
            .template
            .and_then(|template| fresh_from_template(template, &desired, written.as_table()))
        {
            Some(edited) => {
                debug!("{}: written fresh from the template, {why}", path.display());
                edited
            }
            None => {
                debug!("{}: written fresh, {why}", path.display());
                regenerated.clone()
            }
        };
        let text = match fs::read_to_string(path) {
            Ok(existing) => match (existing.parse::<DocumentMut>(), toml::from_str(&existing)) {
                (Ok(mut document), Ok(held)) => {
                    // What the save leaves behind: with a baseline, the file's own values
                    // with this session's changes laid over them; the session's whole
                    // state otherwise.
                    let target = self
                        .baseline
                        .as_ref()
                        .map(|baseline| merge_desired(&held, baseline, &desired))
                        .unwrap_or_else(|| desired.clone());
                    let regenerated = toml::to_string_pretty(&target).map_err(io::Error::other)?;
                    let written = regenerated
                        .parse::<DocumentMut>()
                        .expect("`toml`'s own output is valid TOML");
                    apply_desired(document.as_table_mut(), &held, written.as_table(), &target);
                    let edited = document.to_string();
                    match toml::from_str::<Table>(&edited) {
                        Ok(read_back) if read_back == target => edited,
                        _ => {
                            debug!(
                                "{}: written fresh, the edit did not read back",
                                path.display()
                            );
                            regenerated
                        }
                    }
                }
                _ => fresh("not TOML"),
            },
            Err(error) => fresh(&format!("not edited: {error}")),
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
        self.baseline = Some(desired);
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
    /// A default is no change of the session's (`save`), so `baseline` is filled the same
    /// way: were it not, every window would write its defaults for the keys the file
    /// lacks, over a value another window had just set for one of them.
    ///
    /// # Panics
    /// If `tool_id` is the reserved `common` key (see [`Settings::set_tool`]).
    pub fn merge_defaults(&mut self, tool_id: &str, defaults: &Table) {
        assert_reserved_free(tool_id);
        let table = self.tools.entry(tool_id.to_owned()).or_default();
        merge_missing(table, defaults);
        if let Some(baseline) = &mut self.baseline {
            let remembered = baseline
                .entry(tool_id.to_owned())
                .or_insert_with(|| Value::Table(Table::new()));
            if let Value::Table(remembered) = remembered {
                merge_missing(remembered, defaults);
            }
        }
    }
}

/// `template` edited toward `desired` through `apply_desired` (`written` is `desired` written
/// fresh): the text a fresh write starts from, or `None` when `template` is not TOML or its
/// edit would not read back.
fn fresh_from_template(template: &str, desired: &Table, written: &EditTable) -> Option<String> {
    let (Ok(mut document), Ok(held)) = (
        template.parse::<DocumentMut>(),
        toml::from_str::<Table>(template),
    ) else {
        return None;
    };
    apply_desired(document.as_table_mut(), &held, written, desired);
    let edited = document.to_string();
    match toml::from_str::<Table>(&edited) {
        Ok(read_back) if read_back == *desired => Some(edited),
        _ => None,
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

/// `held` (the file's values now) with the session's changes laid over them: the table a
/// save writes. `baseline` is what the session last read or wrote, `desired` its values
/// now. A `desired` value that still equals `baseline`'s is no change of the session's,
/// so the file's own value stands, its absence included; one that differs is the
/// session's and is written, its absence included. A key all three hold as a table
/// recurses, so a change inside one does not take over its other values.
fn merge_desired(held: &Table, baseline: &Table, desired: &Table) -> Table {
    let mut merged = Table::new();
    for (key, wanted) in desired {
        let value = match (wanted, baseline.get(key), held.get(key)) {
            (wanted, Some(base), file) if wanted == base => file.cloned(),
            (Value::Table(wanted), Some(Value::Table(base)), Some(Value::Table(file))) => {
                Some(Value::Table(merge_desired(file, base, wanted)))
            }
            (wanted, _, _) => Some(wanted.clone()),
        };
        if let Some(value) = value {
            merged.insert(key.clone(), value);
        }
    }
    // A key the session dropped leaves the file; one another writer added while the
    // session was away (`held` alone has it) stays.
    for (key, file_value) in held {
        if !desired.contains_key(key) && !baseline.contains_key(key) {
            merged.insert(key.clone(), file_value.clone());
        }
    }
    merged
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
                        check_interval_minutes = 120\n\
                        \n\
                        [team-compiler]\n\
                        # its own note\n\
                        label = 'x'\n\
                        dt00_overwrite_allow = true\n";
        fs::write(&path, original).unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings.common.last_tool = Some("save-editor".to_owned());
        settings.save(&path).unwrap();

        let expected = original.replace(
            "last_tool = \"team-compiler\"",
            "last_tool = \"save-editor\"",
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            expected,
            "only the changed line moved"
        );
        assert_eq!(Settings::load(&path).unwrap(), settings);

        // A key the session changed is added where its section ends; one the file
        // lacks and the session did not change stays absent.
        let path = dir.join("settings_missing_key.toml");
        let original = "[common]\n\
                        check_for_updates = true\n\
                        check_interval_minutes = 120\n\
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
                "thread_count = 4\nlast_tool = \"save-editor\"\n",
                1
            ),
            "the changed key is appended where its section ends"
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

    /// A small settings-file template: two leading comments, a `[common]` table whose one
    /// key sits under a comment, and a tool table.
    const SAVE_TEMPLATE: &str = "# the template's first line\n\
                                 # and its second\n\
                                 \n\
                                 [common]\n\
                                 # the version to target\n\
                                 pes_version = 17\n\
                                 \n\
                                 [team-compiler]\n\
                                 # the tool's note\n\
                                 label = 'x'\n";

    /// The settings `SAVE_TEMPLATE` serves: the defaults but PES 19 and the tool's `label`.
    fn templated_settings() -> Settings {
        let mut settings = Settings::default().with_template(SAVE_TEMPLATE);
        settings.common.pes_version = PesVersion::Pes19;
        settings.set_tool("team-compiler", table("label = 'x'"));
        settings
    }

    /// Asserts `path` is what `settings`'s save of `SAVE_TEMPLATE` writes: the template's
    /// comments kept, the session's values in their place.
    fn assert_templated_save(path: &Path, settings: &Settings) {
        let text = fs::read_to_string(path).unwrap();
        assert!(
            text.starts_with("# the template's first line\n# and its second\n"),
            "{text}"
        );
        assert!(
            text.contains("[common]\n# the version to target\npes_version = 19\n"),
            "the template's comments, the session's value: {text}"
        );
        assert!(
            text.contains("[team-compiler]\n# the tool's note\nlabel = 'x'\n"),
            "{text}"
        );
        let loaded = Settings::load(path).unwrap();
        assert_eq!(loaded.common, settings.common);
        assert_eq!(loaded.tool("team-compiler"), settings.tool("team-compiler"));
    }

    // A save with a template and no file yet writes the template's comments with the
    // session's values.
    #[test]
    fn a_fresh_save_starts_from_the_template() {
        let dir = test_dir("studio_core_settings_template_fresh");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");

        let mut settings = templated_settings();
        settings.save(&path).unwrap();

        assert_templated_save(&path, &settings);
        fs::remove_dir_all(&dir).unwrap();
    }

    // Over a file broken since the session loaded it, the template takes the same place the
    // bare values take without one.
    #[test]
    fn a_file_that_is_not_toml_is_replaced_from_the_template() {
        let dir = test_dir("studio_core_settings_template_broken");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        fs::write(&path, "[common\nnot toml").unwrap();

        let mut settings = templated_settings();
        settings.save(&path).unwrap();

        assert_templated_save(&path, &settings);
        fs::remove_dir_all(&dir).unwrap();
    }

    // A file that parses is edited in place, template or not: its own comments stay, and no
    // template comment is added.
    #[test]
    fn a_file_that_parses_is_edited_ignoring_the_template() {
        let dir = test_dir("studio_core_settings_template_kept");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "# the member's own note\n[common]\npes_version = 17\n",
        )
        .unwrap();

        let mut settings = Settings::load(&path).unwrap().with_template(SAVE_TEMPLATE);
        settings.common.pes_version = PesVersion::Pes19;
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# the member's own note\n"), "{text}");
        assert!(text.contains("pes_version = 19\n"), "{text}");
        assert!(
            !text.contains("the template's first line"),
            "the template never touches a file that parses: {text}"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    // A template that does not parse, or whose edit does not read back, falls back to the
    // bare values.
    #[test]
    fn a_template_that_does_not_parse_falls_back_to_the_values() {
        let dir = test_dir("studio_core_settings_template_fallback");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");

        let mut settings = Settings::default().with_template("[common\nnot toml");
        settings.save(&path).unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            toml::to_string_pretty(&settings.to_table().unwrap()).unwrap()
        );

        // A section written inline under a comment: the edit puts the comment inside the new
        // table header, which does not parse (as in
        // `save_writes_fresh_when_the_edit_would_not_read_back`).
        fs::remove_file(&path).unwrap();
        let mut settings =
            Settings::default().with_template("# my cup settings\ncommon = { pes_version = 21 }\n");
        settings.common.last_tool = Some("save-editor".to_owned());
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
        // Set, not filled in as a default: a default is no change and is not written.
        settings.set_tool("export-upgrader", Table::new());
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
                        check_interval_minutes = 120\n";
        fs::write(&path, original).unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings.common.pes_version = PesVersion::Pes21;
        settings.save(&path).unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            original.replace("pes_version = 19 #", "pes_version = 21 #"),
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
                        check_interval_minutes = 120\n\
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
                        check_interval_minutes = 120\n\
                        last_tool = \"team-compiler\"\n\
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

    // Two sessions that read the same file: each save writes only the session's own
    // changes, and a value the other session changed keeps its new value and its line.
    // ST-SET-05
    #[test]
    fn each_sessions_save_writes_only_its_own_changes() {
        let dir = test_dir("studio_core_settings_two_sessions");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "[common]\n\
             thread_count = 4\n\
             theme = \"dark\"\n\
             \n\
             [team-compiler]\n\
             label = 'x'\n\
             dt00_overwrite_allow = false\n",
        )
        .unwrap();

        let mut session_a = Settings::load(&path).unwrap();
        let mut session_b = Settings::load(&path).unwrap();
        session_a.common.thread_count = 8;
        let mut tool = session_a.tool("team-compiler").unwrap().clone();
        tool["dt00_overwrite_allow"] = Value::Boolean(true);
        session_a.set_tool("team-compiler", tool);
        session_a.save(&path).unwrap();
        let after_a = fs::read_to_string(&path).unwrap();

        session_b.common.theme = Theme::Light;
        session_b.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        let loaded = Settings::parse(&text).unwrap();
        assert_eq!(
            loaded.common.thread_count, 8,
            "session A's change kept: {text}"
        );
        assert_eq!(
            loaded.common.theme,
            Theme::Light,
            "session B's change written: {text}"
        );
        assert_eq!(
            loaded.tool("team-compiler").unwrap()["dt00_overwrite_allow"],
            Value::Boolean(true),
            "{text}"
        );
        // The lines A's save wrote are still there, byte for byte.
        let line = |text: &str, key: &str| {
            text.lines()
                .find(|line| line.starts_with(key))
                .unwrap_or_else(|| panic!("{key} in: {text}"))
                .to_owned()
        };
        assert_eq!(line(&text, "thread_count"), line(&after_a, "thread_count"));
        assert_eq!(
            line(&text, "dt00_overwrite_allow"),
            line(&after_a, "dt00_overwrite_allow")
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    // Two sessions changing the same value leave the later save's.
    // ST-SET-05
    #[test]
    fn the_later_save_of_the_same_value_wins() {
        let dir = test_dir("studio_core_settings_same_value");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        fs::write(&path, "[common]\nthread_count = 4\ntheme = \"dark\"\n").unwrap();

        let mut session_a = Settings::load(&path).unwrap();
        let mut session_b = Settings::load(&path).unwrap();
        session_a.common.thread_count = 8;
        session_a.save(&path).unwrap();
        session_b.common.thread_count = 2;
        session_b.save(&path).unwrap();

        assert_eq!(Settings::load(&path).unwrap().common.thread_count, 2);
        fs::remove_dir_all(&dir).unwrap();
    }

    // A key one session removed stays removed when the other saves.
    // ST-SET-05
    #[test]
    fn a_key_the_other_session_removed_stays_removed() {
        let dir = test_dir("studio_core_settings_removed_key");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "[common]\n\
             theme = \"dark\"\n\
             \n\
             [team-compiler]\n\
             label = 'x'\n\
             stale = 1\n",
        )
        .unwrap();

        let mut session_a = Settings::load(&path).unwrap();
        let mut session_b = Settings::load(&path).unwrap();
        session_a.set_tool("team-compiler", table("label = 'x'"));
        session_a.save(&path).unwrap();
        session_b.common.theme = Theme::Light;
        session_b.save(&path).unwrap();

        let loaded = Settings::load(&path).unwrap();
        assert_eq!(
            loaded.tool("team-compiler").unwrap(),
            &table("label = 'x'"),
            "`stale` stayed removed"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    // A key one session added survives the other's save of an unrelated change:
    // the second session never saw it, so it is no change of its own.
    #[test]
    fn a_key_the_other_session_added_survives() {
        let dir = test_dir("studio_core_settings_added_key");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "[common]\ntheme = \"dark\"\n\n[team-compiler]\nlabel = 'x'\n",
        )
        .unwrap();

        let mut session_a = Settings::load(&path).unwrap();
        let mut session_b = Settings::load(&path).unwrap();
        let mut tool = session_a.tool("team-compiler").unwrap().clone();
        tool.insert("added".to_owned(), Value::Integer(7));
        session_a.set_tool("team-compiler", tool);
        session_a.set_tool("save-editor", table("speed = 1"));
        session_a.save(&path).unwrap();
        session_b.common.theme = Theme::Light;
        session_b.save(&path).unwrap();

        let loaded = Settings::load(&path).unwrap();
        assert_eq!(
            loaded.tool("team-compiler").unwrap()["added"],
            Value::Integer(7),
            "session B's save kept a key it never had"
        );
        assert_eq!(
            loaded.tool("save-editor"),
            Some(&table("speed = 1")),
            "and a section it never had"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    // A save remembers the session's own values, not the file's: a value another
    // session changed stays different from this session's copy, so it is still
    // not this session's change at the save after next.
    #[test]
    fn a_save_remembers_the_sessions_values_not_the_files() {
        let dir = test_dir("studio_core_settings_remembered");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        fs::write(&path, "[common]\nthread_count = 4\ntheme = \"dark\"\n").unwrap();

        let mut session_a = Settings::load(&path).unwrap();
        let mut session_b = Settings::load(&path).unwrap();
        session_b.common.thread_count = 2;
        session_b.save(&path).unwrap();

        session_a.common.theme = Theme::Light;
        session_a.save(&path).unwrap();
        assert_eq!(
            Settings::load(&path).unwrap().common.thread_count,
            2,
            "A's save kept B's value"
        );
        session_a.common.check_interval_minutes = 60;
        session_a.save(&path).unwrap();
        assert_eq!(
            Settings::load(&path).unwrap().common.thread_count,
            2,
            "A's second save still keeps B's value"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    // A failed save leaves the remembered values as they were, so the change is
    // still the session's change at the next save that succeeds.
    #[test]
    fn a_failed_save_keeps_the_change_for_the_next_one() {
        let dir = test_dir("studio_core_settings_retry");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        fs::write(&path, "[common]\nthread_count = 4\n").unwrap();
        let mut settings = Settings::load(&path).unwrap();
        settings.common.thread_count = 8;

        // A directory at the path makes the rename fail, as
        // `a_failed_save_leaves_no_tmp_sibling` shows.
        let blocked = dir.join("blocked.toml");
        fs::create_dir(&blocked).unwrap();
        assert!(settings.save(&blocked).is_err());

        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap().common.thread_count, 8);
        fs::remove_dir_all(&dir).unwrap();
    }

    // Settings that came from no file hold nothing to compare against: saved
    // over an existing file they write their whole state, as before.
    #[test]
    fn a_settings_from_no_file_writes_its_whole_state() {
        let dir = test_dir("studio_core_settings_no_file");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "# hand note\n[team-compiler]\nlabel = 'x'\nstale = 1\n",
        )
        .unwrap();

        let mut settings = Settings::default();
        settings.common.thread_count = 8;
        settings.set_tool("team-compiler", table("label = 'x'"));
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# hand note\n"), "{text}");
        assert_eq!(Settings::load(&path).unwrap(), settings);
        fs::remove_dir_all(&dir).unwrap();
    }

    // A default a tool fills in at start is no change of the session's: a key the file
    // lacks, and a section it lacks, changed and saved by one session are not put back
    // to their defaults by the other's save.
    #[test]
    fn a_default_filled_in_at_start_is_not_the_sessions_change() {
        let dir = test_dir("studio_core_settings_defaults");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "[common]\ntheme = \"dark\"\n\n[team-compiler]\nlabel = 'x'\n",
        )
        .unwrap();
        let compiler = table("label = ''\ndt00_overwrite_allow = false");
        let editor = table("speed = 1");

        let mut session_a = Settings::load(&path).unwrap();
        let mut session_b = Settings::load(&path).unwrap();
        for session in [&mut session_a, &mut session_b] {
            session.merge_defaults("team-compiler", &compiler);
            session.merge_defaults("save-editor", &editor);
        }
        let mut tool = session_a.tool("team-compiler").unwrap().clone();
        tool["dt00_overwrite_allow"] = Value::Boolean(true);
        session_a.set_tool("team-compiler", tool);
        session_a.set_tool("save-editor", table("speed = 3"));
        session_a.save(&path).unwrap();
        session_b.common.theme = Theme::Light;
        session_b.save(&path).unwrap();

        let loaded = Settings::load(&path).unwrap();
        assert_eq!(
            loaded.tool("team-compiler").unwrap()["dt00_overwrite_allow"],
            Value::Boolean(true),
            "B's default did not undo A's change"
        );
        assert_eq!(loaded.tool("save-editor"), Some(&table("speed = 3")));
        assert_eq!(loaded.common.theme, Theme::Light);
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
