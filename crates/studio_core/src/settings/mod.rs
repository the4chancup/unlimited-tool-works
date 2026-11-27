//! The settings framework: one TOML file holding the common settings under `[common]` and one
//! table per tool under its id (`[compiler]`). Loading fills missing common keys with their
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
        // A file with nothing to keep is written fresh: none yet, one that is not text, or one that
        // is not TOML (broken by hand since the session loaded it). The fresh text is the
        // `template`'s edited to the session's values when one is set and its own edit reads back —
        // the file then carries its comments — and the values alone otherwise. The session's values
        // win there as they win over any hand edit made while the GUI runs, and a folder that
        // cannot be written fails the write below, whatever the read said. So is a file whose edit
        // would not read back as the settings: a shape the in-place edit cannot carry would come
        // out as text `load` refuses and the next start could not open.
        let fresh = |why: &str| -> io::Result<String> {
            let regenerated = toml::to_string_pretty(&desired).map_err(io::Error::other)?;
            let written = regenerated
                .parse::<DocumentMut>()
                .expect("`toml`'s own output is valid TOML");
            Ok(
                match self.template.and_then(|template| {
                    fresh_from_template(template, &desired, written.as_table())
                }) {
                    Some(edited) => {
                        debug!("{}: written fresh from the template, {why}", path.display());
                        edited
                    }
                    None => {
                        debug!("{}: written fresh, {why}", path.display());
                        regenerated
                    }
                },
            )
        };
        let text = match fs::read_to_string(path) {
            Ok(existing) => match (existing.parse::<DocumentMut>(), toml::from_str(&existing)) {
                (Ok(mut document), Ok(held)) => {
                    // What the save leaves behind: with a baseline, the file's own values
                    // with this session's changes laid over them; the session's whole
                    // state otherwise.
                    let target = match self.baseline.as_ref() {
                        Some(baseline) => {
                            let mut target = merge_desired(&held, baseline, &desired);
                            drop_stale_folder_keys(&mut target, baseline, &desired);
                            target
                        }
                        None => desired.clone(),
                    };
                    let regenerated = toml::to_string_pretty(&target).map_err(io::Error::other)?;
                    let written = regenerated
                        .parse::<DocumentMut>()
                        .expect("`toml`'s own output is valid TOML");
                    apply_desired(document.as_table_mut(), &held, written.as_table(), &target);
                    reads_back(document.to_string(), &target).unwrap_or_else(|| {
                        debug!(
                            "{}: written fresh, the edit did not read back",
                            path.display()
                        );
                        regenerated
                    })
                }
                _ => fresh("not TOML")?,
            },
            Err(error) => fresh(&format!("not edited: {error}"))?,
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
    pub(crate) fn to_table(&self) -> Result<Table, toml::ser::Error> {
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
    reads_back(document.to_string(), desired)
}

/// `edited` when it reads back as `values`, what its edit meant to leave; `None` for text
/// that is not TOML or that holds other values, which the caller then writes another way.
/// The in-place edit is trusted only as far as this check: a file that came out wrong
/// costs the next start, a file written fresh costs its comments.
fn reads_back(edited: String, values: &Table) -> Option<String> {
    match toml::from_str::<Table>(&edited) {
        Ok(read_back) if read_back == *values => Some(edited),
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
            // comment) moves onto the new value, and the comment above the key follows to
            // where the new kind writes it (`keep_leading_decor`).
            (_, _, Some(slot)) => {
                let header = match slot {
                    EditItem::Table(table) => Some(table.decor().clone()),
                    EditItem::ArrayOfTables(tables) => {
                        tables.iter().next().map(|table| table.decor().clone())
                    }
                    _ => None,
                };
                let decor = slot.as_value().map(|value| value.decor().clone());
                *slot = fresh.clone();
                if let (Some(decor), Some(value)) = (decor, slot.as_value_mut()) {
                    *value.decor_mut() = decor;
                }
                keep_leading_decor(document, key, header);
            }
            (_, _, None) => {
                let mut fresh = fresh.clone();
                place_tables(&mut fresh, header_position(document));
                document.insert(key, fresh);
            }
        }
    }
}

/// The position a header written under `document` takes: the enclosing table's own
/// position, so a `[key]`/`[[key]]` header renders inside its section; at the root,
/// which has none, one past the largest position any of its tables carries, so a new
/// section renders last. The tables cloned out of `written` carry `written`'s own
/// ordering, which is not the file's.
fn header_position(document: &EditTable) -> isize {
    match document.position() {
        Some(position) => position,
        None => {
            document
                .iter()
                .filter_map(|(_, item)| deepest_position(item))
                .max()
                .unwrap_or_default()
                + 1
        }
    }
}

/// The largest position `item`'s tables carry, `None` when it has none.
fn deepest_position(item: &EditItem) -> Option<isize> {
    match item {
        EditItem::Table(table) => deepest_in(table),
        EditItem::ArrayOfTables(tables) => tables.iter().filter_map(deepest_in).max(),
        _ => None,
    }
}

/// The largest position `table` and the tables under it carry, `None` when it has none.
fn deepest_in(table: &EditTable) -> Option<isize> {
    table
        .position()
        .into_iter()
        .chain(table.iter().filter_map(|(_, item)| deepest_position(item)))
        .max()
}

/// Gives every table `item` holds the `position` that orders where its header renders:
/// `item`'s own and each table nested under it, at any depth (a no-op for a value), so
/// a section inserted whole keeps its `[[key]]` tables beside its own header.
fn place_tables(item: &mut EditItem, position: isize) {
    match item {
        EditItem::Table(table) => place_in(table, position),
        EditItem::ArrayOfTables(tables) => {
            for table in tables.iter_mut() {
                place_in(table, position);
            }
        }
        _ => {}
    }
}

/// Sets `position` on `table` and every table nested under it.
fn place_in(table: &mut EditTable, position: isize) {
    table.set_position(position);
    for (_, item) in table.iter_mut() {
        place_tables(item, position);
    }
}

/// The comment above a key sits on the key itself for a `key = value` line and on the
/// (first) table's decor for a `[key]`/`[[key]]` header. A slot whose item changed kind
/// moves it over: left on the key it would land inside the header's brackets
/// (`[[# comment\nkey]]`), which is not TOML a save may write. `header` is the decor the
/// replaced item's own header carried, when it was a table kind. The new item's tables
/// take `header_position(document)` too, so a section's headers stay inside it and a root
/// inline section renders last, not where `written` put them.
fn keep_leading_decor(document: &mut EditTable, key: &str, header: Option<toml_edit::Decor>) {
    let position = header_position(document);
    let Some((mut leaf, slot)) = document.iter_mut().find(|(leaf, _)| leaf.get() == key) else {
        return;
    };
    place_tables(slot, position);
    let header_slot = match slot {
        EditItem::Table(table) => Some(table.decor_mut()),
        EditItem::ArrayOfTables(tables) => tables.get_mut(0).map(|table| table.decor_mut()),
        _ => None,
    };
    match (header_slot, header) {
        // A `key = value` became a header: the key's decor moves above the brackets and
        // is cleared off the key, which a header renders inside them.
        (Some(decor), None) => {
            if let Some(prefix) = leaf.leaf_decor().prefix().cloned() {
                decor.set_prefix(prefix);
            }
            leaf.leaf_decor_mut().clear();
        }
        // A header replaced by a fresh header keeps its own decor.
        (Some(decor), Some(header)) => *decor = header,
        // A header became a `key = value`: its decor's comment moves back above the key,
        // and the key gets the ` = ` spacing a `key = value` line carries.
        (None, Some(header)) => {
            if let Some(prefix) = header.prefix() {
                leaf.leaf_decor_mut().set_prefix(prefix.clone());
            }
            leaf.leaf_decor_mut().set_suffix(" ");
        }
        (None, None) => {}
    }
}

/// `held` (the file's values now) with the session's changes laid over them: the table a
/// save writes. `baseline` is what the session last read or wrote, `desired` its values
/// now. A `desired` value that still equals `baseline`'s is no change of the session's,
/// so the file's own value stands, its absence included; one that differs is the
/// session's and is written, its absence included. A changed key that is a table recurses,
/// so a change inside one does not take over its other values. A side that lacks the table
/// counts as an empty one there: `[common.pes_folders]` is left out of the file while it
/// is empty, and two windows that each fill a row, or one that empties its only row, must
/// still be merged row by row, not table by table.
fn merge_desired(held: &Table, baseline: &Table, desired: &Table) -> Table {
    let empty = Table::new();
    let mut merged = Table::new();
    let held_alone = held.keys().filter(|key| !desired.contains_key(*key));
    for key in desired.keys().chain(held_alone) {
        let (wanted, base, file) = (desired.get(key), baseline.get(key), held.get(key));
        let value = if wanted == base {
            file.cloned()
        } else if let (Some(wanted_table), Some(base), Some(file)) = (
            table_or_empty(wanted, &empty),
            table_or_empty(base, &empty),
            table_or_empty(file, &empty),
        ) {
            let inner = merge_desired(file, base, wanted_table);
            // A table the session no longer holds is gone once nothing else is in it.
            (wanted.is_some() || !inner.is_empty()).then_some(Value::Table(inner))
        } else {
            wanted.cloned()
        };
        if let Some(value) = value {
            merged.insert(key.clone(), value);
        }
    }
    merged
}

/// `value` as a table for `merge_desired`: an absent one stands for an empty table, and a
/// value that is no table is `None`.
fn table_or_empty<'a>(value: Option<&'a Value>, empty: &'a Table) -> Option<&'a Table> {
    match value {
        None => Some(empty),
        Some(Value::Table(table)) => Some(table),
        Some(_) => None,
    }
}

/// Removes the keys `target`'s `[common.pes_folders]` holds in a spelling the program
/// never writes. A member may write a row under the four-digit year (`2019`) the
/// reader also takes, but the program writes two digits (`19`), and a changed row kept
/// under both spellings reads back with the old one winning: the reader keeps a
/// version's later key in key order. A key drops only when the session changed that
/// version's row, the row under the written key differing between `baseline` and
/// `desired`, its absence in one included; a row the session did not touch stays as
/// the member wrote it.
fn drop_stale_folder_keys(target: &mut Table, baseline: &Table, desired: &Table) {
    fn folders_of(root: &Table) -> Option<&Table> {
        root.get(COMMON_KEY)
            .and_then(Value::as_table)
            .and_then(|section| section.get("pes_folders"))
            .and_then(Value::as_table)
    }
    let empty = Table::new();
    let (before, after) = (
        folders_of(baseline).unwrap_or(&empty),
        folders_of(desired).unwrap_or(&empty),
    );
    let Some(merged) = target
        .get_mut(COMMON_KEY)
        .and_then(Value::as_table_mut)
        .and_then(|section| section.get_mut("pes_folders"))
        .and_then(Value::as_table_mut)
    else {
        return;
    };
    merged.retain(|key, _| match common::written_folder_key(key) {
        Some(written) if written != key => {
            before.get(written.as_str()) == after.get(written.as_str())
        }
        _ => true,
    });
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
    use crate::shell::test_harness::test_dir;
    use pes_version::PesVersion;

    fn table(text: &str) -> Table {
        toml::from_str(text).unwrap()
    }

    #[test]
    fn missing_file_loads_as_defaults() {
        let dir = test_dir("settings_missing");
        let loaded = Settings::load(&dir.join("does_not_exist.toml")).unwrap();
        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn a_settings_path_that_exists_but_is_not_a_file_is_an_error() {
        let dir = test_dir("settings_dir_path");
        // Only a missing file yields the defaults; a folder at the path is a read failure.
        assert!(matches!(
            Settings::load(&dir),
            Err(SettingsError::Io(error)) if error.kind() != io::ErrorKind::NotFound
        ));
    }

    #[test]
    fn save_then_load_round_trips() {
        let mut settings = Settings::default();
        settings.common.pes_version = PesVersion::Pes21;
        settings.common.thread_count = 4;
        settings.common.last_tool = Some("save-editor".to_owned());
        settings.set_tool(
            "compiler",
            table("dt00_overwrite_allow = true\nlabel = 'x'"),
        );

        let dir = test_dir("settings_roundtrip");
        let path = dir.join("settings.toml");
        settings.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert_eq!(loaded, settings);
        assert_eq!(
            fs::read_dir(&dir).unwrap().count(),
            1,
            "no temporary file left behind"
        );
    }

    #[test]
    fn concurrent_saves_leave_a_complete_file() {
        let dir = test_dir("settings_concurrent");
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
    }

    #[test]
    fn a_failed_save_leaves_no_tmp_sibling() {
        let dir = test_dir("settings_save_fail");
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
    }

    // A save edits the file in place: hand-written comments, blank lines and key order survive
    // a `last_tool` save.
    #[test]
    fn save_edits_in_place_and_only_the_changed_line_moves() {
        let dir = test_dir("settings_inplace");
        let path = dir.join("settings.toml");
        let original = "# the member's note about this file\n\
                        [common]\n\
                        # the version to target\n\
                        pes_version = 19\n\
                        thread_count = 4\n\
                        last_tool = \"compiler\"\n\
                        theme = \"dark\"\n\
                        pes_folder_path = '/games/Pro Evolution Soccer 20**'\n\
                        exports_folder_path = \"exports\"\n\
                        memory_cap_percent = 80.0\n\
                        check_for_updates = true\n\
                        \n\
                        [compiler]\n\
                        # its own note\n\
                        label = 'x'\n\
                        dt00_overwrite_allow = true\n";
        fs::write(&path, original).unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings.common.last_tool = Some("save-editor".to_owned());
        settings.save(&path).unwrap();

        let expected = original.replace("last_tool = \"compiler\"", "last_tool = \"save-editor\"");
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
                        exports_folder_path = \"exports\"\n\
                        memory_cap_percent = 80.0\n\
                        pes_folder_path = '/games/Pro Evolution Soccer 20**'\n\
                        pes_version = 19\n\
                        theme = \"dark\"\n\
                        thread_count = 4\n\
                        \n\
                        [compiler]\n\
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
    }

    // A section a member wrote inline, under a comment, becomes a `[section]` table on
    // save with the comment kept above it: the in-place edit carries the change, so the
    // file stays the member's.
    #[test]
    fn an_inline_section_is_written_as_a_table_keeping_its_comment() {
        let dir = test_dir("settings_inline_section");
        let path = dir.join("settings.toml");
        fs::write(&path, "# my cup settings\ncommon = { pes_version = 21 }\n").unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings.common.last_tool = Some("save-editor".to_owned());
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.starts_with("# my cup settings\n[common]\n"),
            "the comment sits above the table the inline section became: {text}"
        );
        assert_eq!(Settings::load(&path).unwrap(), settings);
    }

    // A file broken by hand since the session loaded it is written fresh from the session.
    #[test]
    fn save_writes_fresh_over_a_file_that_does_not_parse() {
        let dir = test_dir("settings_broken_file");
        let path = dir.join("settings.toml");
        let mut settings = Settings::default();
        settings.common.last_tool = Some("save-editor".to_owned());
        fs::write(&path, "[common\npes_version = 21\n").unwrap();

        settings.save(&path).unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            toml::to_string_pretty(&settings.to_table().unwrap()).unwrap()
        );
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
                                 [compiler]\n\
                                 # the tool's note\n\
                                 label = 'x'\n";

    /// The settings `SAVE_TEMPLATE` serves: the defaults but PES 19 and the tool's `label`.
    fn templated_settings() -> Settings {
        let mut settings = Settings::default().with_template(SAVE_TEMPLATE);
        settings.common.pes_version = PesVersion::Pes19;
        settings.set_tool("compiler", table("label = 'x'"));
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
            text.contains("[compiler]\n# the tool's note\nlabel = 'x'\n"),
            "{text}"
        );
        let loaded = Settings::load(path).unwrap();
        assert_eq!(loaded.common, settings.common);
        assert_eq!(loaded.tool("compiler"), settings.tool("compiler"));
    }

    // A save with a template and no file yet writes the template's comments with the
    // session's values.
    #[test]
    fn a_fresh_save_starts_from_the_template() {
        let dir = test_dir("settings_template_fresh");
        let path = dir.join("settings.toml");

        let mut settings = templated_settings();
        settings.save(&path).unwrap();

        assert_templated_save(&path, &settings);
    }

    // Over a file broken since the session loaded it, the template takes the same place the
    // bare values take without one.
    #[test]
    fn a_file_that_is_not_toml_is_replaced_from_the_template() {
        let dir = test_dir("settings_template_broken");
        let path = dir.join("settings.toml");
        fs::write(&path, "[common\nnot toml").unwrap();

        let mut settings = templated_settings();
        settings.save(&path).unwrap();

        assert_templated_save(&path, &settings);
    }

    // A file that parses is edited in place, template or not: its own comments stay, and no
    // template comment is added.
    #[test]
    fn a_file_that_parses_is_edited_ignoring_the_template() {
        let dir = test_dir("settings_template_kept");
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
    }

    // A `key = value` the session turns into a list of tables keeps the comment above
    // its key: left where the `key =` line held it, the comment would land inside the
    // `[[key]]` header, the edit would not read back and the file would be written
    // fresh, without its comments.
    #[test]
    fn a_key_becoming_a_list_of_tables_keeps_the_comment_above_it() {
        let dir = test_dir("settings_tables_comment");
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "[save-editor]\n\
             # the saves opened last, newest first\n\
             recent_files = []\n\
             # its own note\n\
             label = 'x'\n",
        )
        .unwrap();

        let mut settings = Settings::load(&path).unwrap();
        let mut tool = settings.tool("save-editor").unwrap().clone();
        tool["recent_files"] = table(
            "recent_files = [{ path = 'a', name = 'A' }, { path = 'b', name = 'B' }]",
        )["recent_files"]
            .clone();
        settings.set_tool("save-editor", tool);
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("# the saves opened last, newest first\n"),
            "the comment above the key is kept: {text}"
        );
        assert!(
            text.contains("# its own note\nlabel = 'x'\n"),
            "the other key and its comment are byte for byte: {text}"
        );
        assert_eq!(Settings::load(&path).unwrap(), settings);
    }

    // The way back: a list of tables emptied to `[]` keeps the comment that sat above
    // its `[[key]]` header, moved back above the `key =` line.
    #[test]
    fn an_emptied_list_of_tables_keeps_the_comment_above_its_key() {
        let dir = test_dir("settings_emptied_comment");
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "[save-editor]\n\
             # its own note\n\
             label = 'x'\n\
             # the saves opened last, newest first\n\
             [[save-editor.recent_files]]\n\
             name = \"A\"\n\
             path = 'a'\n\
             \n\
             [[save-editor.recent_files]]\n\
             name = \"B\"\n\
             path = 'b'\n",
        )
        .unwrap();

        let mut settings = Settings::load(&path).unwrap();
        let mut tool = settings.tool("save-editor").unwrap().clone();
        tool["recent_files"] = Value::Array(Vec::new());
        settings.set_tool("save-editor", tool);
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("# the saves opened last, newest first\nrecent_files = []\n"),
            "the comment moved back above the emptied list: {text}"
        );
        assert!(
            text.contains("# its own note\nlabel = 'x'\n"),
            "the other key and its comment are byte for byte: {text}"
        );
        assert_eq!(Settings::load(&path).unwrap(), settings);
    }

    // The shipped settings file itself: a save that fills `save-editor`'s
    // `recent_files` keeps every comment line the file carries.
    #[test]
    fn the_bundled_settings_keep_every_comment_when_a_recent_save_lands() {
        let dir = test_dir("settings_bundled_comment");
        let path = dir.join("settings.toml");
        const BUNDLED: &str = include_str!("../../../../resources/bundle/data/settings.toml");
        fs::write(&path, BUNDLED).unwrap();

        let mut settings = Settings::load(&path).unwrap();
        let mut tool = settings.tool("save-editor").unwrap().clone();
        tool["recent_files"] = table(
            "recent_files = [{ path = 'a', name = 'A' }, { path = 'b', name = 'B' }]",
        )["recent_files"]
            .clone();
        settings.set_tool("save-editor", tool);
        settings.save(&path).unwrap();

        fn comments_of(text: &str) -> Vec<&str> {
            text.lines()
                .filter(|line| line.trim_start().starts_with('#'))
                .collect()
        }
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(
            comments_of(&text),
            comments_of(BUNDLED),
            "every comment line of the bundled file: {text}"
        );
        let before_compiler = |text: &str| {
            let save_editor = text.find("[save-editor]").unwrap();
            let compiler = text.find("[compiler]").unwrap();
            text.match_indices("[[save-editor.recent_files]]")
                .all(|(index, _)| save_editor < index && index < compiler)
        };
        assert!(
            before_compiler(&text),
            "the recent saves' headers stand inside [save-editor]: {text}"
        );
        assert_eq!(Settings::load(&path).unwrap(), settings);

        // A second save replaces the non-empty list with another, the old first
        // save last: the comment above the first header survives, and every new
        // table stands inside the section.
        let mut settings = Settings::load(&path).unwrap();
        let mut tool = settings.tool("save-editor").unwrap().clone();
        tool["recent_files"] = table(
            "recent_files = [{ path = 'b', name = 'B' }, { path = 'c', name = 'C' }, \
             { path = 'a', name = 'A' }]",
        )["recent_files"]
            .clone();
        settings.set_tool("save-editor", tool);
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(
            comments_of(&text),
            comments_of(BUNDLED),
            "every comment line of the bundled file: {text}"
        );
        assert!(before_compiler(&text), "{text}");
        assert_eq!(Settings::load(&path).unwrap(), settings);
    }

    // A key the file lacks lands at the end of its section's keys, but its tables take
    // the section's own position: `written` orders the sections alphabetically, so a
    // `[[key]]` inserted under a section that is not the last would render after the
    // later sections it sorts behind. At the root a new section renders last.
    #[test]
    fn an_inserted_table_key_stays_inside_its_section() {
        let dir = test_dir("settings_insert_position");
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "[common]\n\
             pes_version = 17\n\
             \n\
             [save-editor]\n\
             aatf_ruleset = \"dbc7\"\n\
             \n\
             [compiler]\n\
             multicpk_mode = false\n",
        )
        .unwrap();

        let mut settings = Settings::load(&path).unwrap();
        let mut tool = settings.tool("save-editor").unwrap().clone();
        tool.insert(
            "recent_files".to_owned(),
            table("recent_files = [{ path = 'a', name = 'A' }, { path = 'b', name = 'B' }]")
                ["recent_files"]
                .clone(),
        );
        settings.set_tool("save-editor", tool);
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        let save_editor = text.find("[save-editor]").unwrap();
        let compiler = text.find("[compiler]").unwrap();
        let headers: Vec<_> = text.match_indices("[[save-editor.recent_files]]").collect();
        assert!(
            headers.len() == 2
                && headers
                    .iter()
                    .all(|(index, _)| save_editor < *index && *index < compiler),
            "both headers inside [save-editor]'s section: {text}"
        );
        assert_eq!(Settings::load(&path).unwrap(), settings);

        // A section the file lacks lands last: `export-upgrader` sorts before
        // `save-editor`, where `written` would put it.
        let mut settings = Settings::load(&path).unwrap();
        settings.set_tool("export-upgrader", Table::new());
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.trim_end().ends_with("[export-upgrader]"),
            "the new section renders last: {text}"
        );
        assert_eq!(Settings::load(&path).unwrap(), settings);
    }

    // A section the file lacks is inserted whole: its own tables take the new position
    // with it, at any depth, or `written`'s ordering splits it across the file: a
    // `[[save-editor.recent_files]]` above the `[save-editor]` header it belongs to.
    // The file needs headers sorting after `save-editor` (`written` is alphabetical)
    // for the split to show.
    #[test]
    fn an_inserted_section_renders_as_one_block() {
        let dir = test_dir("settings_insert_block");
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "[common]\n\
             pes_version = 17\n\
             \n\
             [export-upgrader]\n\
             \n\
             [ruleset-editor]\n\
             last_folder = \"\"\n\
             \n\
             [compiler]\n\
             multicpk_mode = false\n\
             \n\
             [team-compiler]\n\
             \n\
             [z-section]\n\
             x = 1\n",
        )
        .unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings.set_tool(
            "save-editor",
            table(
                "aatf_ruleset = 'dbc7'\n\
                   recent_files = [{ path = 'a', name = 'A' }, { path = 'b', name = 'B' }]",
            ),
        );
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        let (section, last_existing) = (
            text.find("[save-editor]").unwrap(),
            text.find("[z-section]").unwrap(),
        );
        let headers: Vec<_> = text.match_indices("[[save-editor.recent_files]]").collect();
        assert_eq!(headers.len(), 2, "{text}");
        assert!(
            headers.iter().all(|(index, _)| section < *index),
            "[save-editor] before both of its tables' headers: {text}"
        );
        assert!(
            last_existing < section,
            "the inserted section and its tables render last: {text}"
        );
        assert_eq!(Settings::load(&path).unwrap(), settings);
    }

    // The new section's position is one past the file's deepest table, not its last
    // top-level one: `[compiler]` inserted into a file whose last section ends in
    // `[[save-editor.recent_files]]` tables and a `[save-editor.sub]` table renders
    // after all of them.
    #[test]
    fn an_inserted_section_renders_after_the_files_deepest_table() {
        let dir = test_dir("settings_deepest");
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "[common]\n\
             pes_version = 17\n\
             \n\
             [save-editor]\n\
             aatf_ruleset = \"dbc7\"\n\
             \n\
             [save-editor.sub]\n\
             x = 1\n\
             \n\
             [[save-editor.recent_files]]\n\
             name = \"A\"\n\
             path = 'a'\n\
             \n\
             [[save-editor.recent_files]]\n\
             name = \"B\"\n\
             path = 'b'\n",
        )
        .unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings.set_tool("compiler", table("multicpk_mode = false"));
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        let last_header = text.lines().rfind(|line| line.starts_with('[')).unwrap();
        assert_eq!(last_header, "[compiler]", "{text}");
        assert_eq!(Settings::load(&path).unwrap(), settings);
    }

    // A root `key = {}` line ahead of the sections, turned into `[key]` on save, lands
    // after them all: one past the file's last position, not the same position the
    // last section already holds (the equal position would sort it before that one,
    // the key being earlier in the file).
    #[test]
    fn an_inline_root_section_becomes_a_header_after_the_last_one() {
        let dir = test_dir("settings_root_inline");
        let path = dir.join("settings.toml");
        fs::write(&path, "x = { a = 1 }\n\n[z-one]\nk = 1\n\n[z-two]\nk = 2\n").unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings.set_tool("x", table("a = 2"));
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        let last_header = text.lines().rfind(|line| line.starts_with('[')).unwrap();
        assert_eq!(last_header, "[x]", "{text}");
        assert_eq!(Settings::load(&path).unwrap(), settings);
    }

    // A template that does not parse falls back to the bare values. One that parses but
    // holds a shape the file would not (a section written inline, under a comment) is
    // edited like any other file: the comment moves above the new table header.
    #[test]
    fn a_template_that_does_not_parse_falls_back_to_the_values() {
        let dir = test_dir("settings_template_fallback");
        let path = dir.join("settings.toml");

        let mut settings = Settings::default().with_template("[common\nnot toml");
        settings.save(&path).unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            toml::to_string_pretty(&settings.to_table().unwrap()).unwrap()
        );

        // The inline section becomes a `[section]` table under its comment, as in
        // `an_inline_section_is_written_as_a_table_keeping_its_comment`.
        fs::remove_file(&path).unwrap();
        let mut settings =
            Settings::default().with_template("# my cup settings\ncommon = { pes_version = 21 }\n");
        settings.common.last_tool = Some("save-editor".to_owned());
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# my cup settings\n[common]\n"), "{text}");
        assert_eq!(Settings::load(&path).unwrap().common, settings.common);
    }

    // A tool with no settings keeps an empty table, and a nested table is edited key by key:
    // the comment inside it survives a change of its sibling.
    #[test]
    fn save_writes_an_empty_tool_table_and_edits_a_nested_one_in_place() {
        let dir = test_dir("settings_nested");
        let path = dir.join("settings.toml");
        let original = "[compiler.kits]\n\
                        # kept\n\
                        collar = 3\n\
                        sleeves = 1\n";
        fs::write(&path, original).unwrap();

        let mut settings = Settings::load(&path).unwrap();
        // Set, not filled in as a default: a default is no change and is not written.
        settings.set_tool("export-upgrader", Table::new());
        let mut tool = settings.tool("compiler").unwrap().clone();
        tool["kits"]
            .as_table_mut()
            .unwrap()
            .insert("sleeves".to_owned(), Value::Integer(2));
        settings.set_tool("compiler", tool);
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("[compiler.kits]\n# kept\ncollar = 3\nsleeves = 2\n"),
            "the nested table changed in place: {text}"
        );
        assert!(
            text.contains("[export-upgrader]"),
            "the empty table is written: {text}"
        );
        assert_eq!(Settings::load(&path).unwrap(), settings);
    }

    // A changed value keeps the comment above its key and the one after it: a pick in the
    // version selector rewrites `pes_version`, the key a member most likely annotated.
    #[test]
    fn save_keeps_the_comments_around_a_changed_value() {
        let dir = test_dir("settings_changed_comments");
        let path = dir.join("settings.toml");
        let original = "[common]\n\
                        # the version to target\n\
                        pes_version = 19 # set by hand\n\
                        thread_count = 4\n\
                        theme = \"dark\"\n\
                        pes_folder_path = '/games/Pro Evolution Soccer 20**'\n\
                        exports_folder_path = \"exports\"\n\
                        memory_cap_percent = 80.0\n\
                        check_for_updates = true\n";
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
    }

    #[test]
    fn save_removes_dropped_keys_and_appends_new_sections() {
        let dir = test_dir("settings_inplace_keys");
        let path = dir.join("settings.toml");
        let original = "[common]\n\
                        pes_version = 19\n\
                        thread_count = 0\n\
                        memory_cap_percent = 80.0\n\
                        exports_folder_path = \"exports\"\n\
                        pes_folder_path = '/games/Pro Evolution Soccer 20**'\n\
                        theme = \"dark\"\n\
                        check_for_updates = true\n\
                        last_tool = \"compiler\"\n\
                        \n\
                        [compiler]\n\
                        label = 'x'\n\
                        stale = 1\n";
        fs::write(&path, original).unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings.set_tool("compiler", table("label = 'x'"));
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
                        last_tool = \"compiler\"\n\
                        \n\
                        [compiler]\n\
                        label = 'x'\n\
                        \n\
                        [save-editor]\n\
                        speed = 1\n";
        assert_eq!(fs::read_to_string(&path).unwrap(), expected);
        assert_eq!(Settings::load(&path).unwrap(), settings);
    }

    // Two sessions that read the same file: each save writes only the session's own
    // changes, and a value the other session changed keeps its new value and its line.
    // ST-SET-05
    #[test]
    fn each_sessions_save_writes_only_its_own_changes() {
        let dir = test_dir("settings_two_sessions");
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "[common]\n\
             thread_count = 4\n\
             theme = \"dark\"\n\
             \n\
             [compiler]\n\
             label = 'x'\n\
             dt00_overwrite_allow = false\n",
        )
        .unwrap();

        let mut session_a = Settings::load(&path).unwrap();
        let mut session_b = Settings::load(&path).unwrap();
        session_a.common.thread_count = 8;
        let mut tool = session_a.tool("compiler").unwrap().clone();
        tool["dt00_overwrite_allow"] = Value::Boolean(true);
        session_a.set_tool("compiler", tool);
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
            loaded.tool("compiler").unwrap()["dt00_overwrite_allow"],
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

        // The second session then changes `thread_count` too and saves; the file holds
        // its `thread_count` over the first's.
        session_b.common.thread_count = 2;
        session_b.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert_eq!(loaded.common.thread_count, 2);
        assert_eq!(loaded.common.theme, Theme::Light);
        assert_eq!(
            loaded.tool("compiler").unwrap()["dt00_overwrite_allow"],
            Value::Boolean(true)
        );
    }

    /// A settings file whose `[common.pes_folders]` table a member wrote by hand, the
    /// PES 2019 row under the four-digit year the reader also takes.
    const HAND_FOLDER_FILE: &str = "[common]\n\
                                    \n\
                                    [common.pes_folders]\n\
                                    2019 = 'D:\\PES 2019'\n\
                                    21 = 'F:\\PES2021'\n";

    // A row the member wrote under the four-digit year reads as the two-digit one, but
    // a save writes two digits. A changed row's old spelling must go with the save:
    // kept beside the new `19` row it would win the next load, the reader keeping a
    // version's later key in key order.
    #[test]
    fn a_changed_folder_row_drops_its_four_digit_key() {
        let dir = test_dir("settings_year_key_changed");
        let path = dir.join("settings.toml");
        fs::write(&path, HAND_FOLDER_FILE).unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings
            .common
            .pes_folders
            .insert(PesVersion::Pes19, r"E:\PES 2019".to_owned());
        settings.save(&path).unwrap();

        let (folders, text) = folders_in(&path);
        assert_eq!(
            folders,
            BTreeMap::from([
                (PesVersion::Pes19, r"E:\PES 2019".to_owned()),
                (PesVersion::Pes21, r"F:\PES2021".to_owned()),
            ]),
            "{text}"
        );
        assert!(
            !text.contains("2019 ="),
            "the old-spelling key went with the save: {text}"
        );
    }

    // The same when the session empties the row: a kept `2019` would read back as the
    // row the session removed.
    #[test]
    fn an_emptied_folder_row_drops_its_four_digit_key() {
        let dir = test_dir("settings_year_key_emptied");
        let path = dir.join("settings.toml");
        fs::write(&path, HAND_FOLDER_FILE).unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings.common.pes_folders.remove(&PesVersion::Pes19);
        settings.save(&path).unwrap();

        let (folders, text) = folders_in(&path);
        assert_eq!(
            folders,
            BTreeMap::from([(PesVersion::Pes21, r"F:\PES2021".to_owned())]),
            "PES 2019's row is gone: {text}"
        );
    }

    // A four-digit row whose version the session did not touch is not the session's
    // change: it stays in the file exactly as the member wrote it.
    #[test]
    fn an_untouched_folder_row_keeps_its_four_digit_key() {
        let dir = test_dir("settings_year_key_untouched");
        let path = dir.join("settings.toml");
        fs::write(&path, HAND_FOLDER_FILE).unwrap();

        let mut settings = Settings::load(&path).unwrap();
        settings
            .common
            .pes_folders
            .insert(PesVersion::Pes21, r"G:\PES2021".to_owned());
        settings.save(&path).unwrap();

        let (folders, text) = folders_in(&path);
        assert!(
            text.contains("2019 = 'D:\\PES 2019'"),
            "the member's line is kept byte for byte: {text}"
        );
        assert_eq!(
            folders.get(&PesVersion::Pes21).map(String::as_str),
            Some(r"G:\PES2021"),
            "{text}"
        );
    }

    // Two sessions changing the same value leave the later save's.
    // ST-SET-05
    #[test]
    fn the_later_save_of_the_same_value_wins() {
        let dir = test_dir("settings_same_value");
        let path = dir.join("settings.toml");
        fs::write(&path, "[common]\nthread_count = 4\ntheme = \"dark\"\n").unwrap();

        let mut session_a = Settings::load(&path).unwrap();
        let mut session_b = Settings::load(&path).unwrap();
        session_a.common.thread_count = 8;
        session_a.save(&path).unwrap();
        session_b.common.thread_count = 2;
        session_b.save(&path).unwrap();

        assert_eq!(Settings::load(&path).unwrap().common.thread_count, 2);
    }

    // A key one session removed stays removed when the other saves.
    // ST-SET-05
    #[test]
    fn a_key_the_other_session_removed_stays_removed() {
        let dir = test_dir("settings_removed_key");
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "[common]\n\
             theme = \"dark\"\n\
             \n\
             [compiler]\n\
             label = 'x'\n\
             stale = 1\n",
        )
        .unwrap();

        let mut session_a = Settings::load(&path).unwrap();
        let mut session_b = Settings::load(&path).unwrap();
        session_a.set_tool("compiler", table("label = 'x'"));
        session_a.save(&path).unwrap();
        session_b.common.theme = Theme::Light;
        session_b.save(&path).unwrap();

        let loaded = Settings::load(&path).unwrap();
        assert_eq!(
            loaded.tool("compiler").unwrap(),
            &table("label = 'x'"),
            "`stale` stayed removed"
        );
    }

    // A key one session added survives the other's save of an unrelated change:
    // the second session never saw it, so it is no change of its own.
    #[test]
    fn a_key_the_other_session_added_survives() {
        let dir = test_dir("settings_added_key");
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "[common]\ntheme = \"dark\"\n\n[compiler]\nlabel = 'x'\n",
        )
        .unwrap();

        let mut session_a = Settings::load(&path).unwrap();
        let mut session_b = Settings::load(&path).unwrap();
        let mut tool = session_a.tool("compiler").unwrap().clone();
        tool.insert("added".to_owned(), Value::Integer(7));
        session_a.set_tool("compiler", tool);
        session_a.set_tool("save-editor", table("speed = 1"));
        session_a.save(&path).unwrap();
        session_b.common.theme = Theme::Light;
        session_b.save(&path).unwrap();

        let loaded = Settings::load(&path).unwrap();
        assert_eq!(
            loaded.tool("compiler").unwrap()["added"],
            Value::Integer(7),
            "session B's save kept a key it never had"
        );
        assert_eq!(
            loaded.tool("save-editor"),
            Some(&table("speed = 1")),
            "and a section it never had"
        );
    }

    // A save remembers the session's own values, not the file's: a value another
    // session changed stays different from this session's copy, so it is still
    // not this session's change at the save after next.
    #[test]
    fn a_save_remembers_the_sessions_values_not_the_files() {
        let dir = test_dir("settings_remembered");
        let path = dir.join("settings.toml");
        fs::write(&path, "[common]\nthread_count = 4\ntheme = \"dark\"\n").unwrap();

        let mut session_a = Settings::load(&path).unwrap();
        let mut session_b = Settings::load(&path).unwrap();
        session_b.common.check_for_updates = false;
        session_b.save(&path).unwrap();

        session_a.common.theme = Theme::Light;
        session_a.save(&path).unwrap();
        assert!(
            !Settings::load(&path).unwrap().common.check_for_updates,
            "A's save kept B's value"
        );
        session_a.common.thread_count = 60;
        session_a.save(&path).unwrap();
        assert!(
            !Settings::load(&path).unwrap().common.check_for_updates,
            "A's second save still keeps B's value"
        );

        // A writes a key and B then writes another value to it: A's next save must not
        // write A's stale copy back over B's value.
        session_a.common.last_tool = Some("save-editor".to_owned());
        session_a.save(&path).unwrap();
        session_b.common.last_tool = Some("export-upgrader".to_owned());
        session_b.save(&path).unwrap();
        session_a.common.thread_count = 30;
        session_a.save(&path).unwrap();
        assert_eq!(
            Settings::load(&path).unwrap().common.last_tool,
            Some("export-upgrader".to_owned()),
            "A's third save kept B's value"
        );
    }

    // A failed save leaves the remembered values as they were, so the change is
    // still the session's change at the next save that succeeds.
    #[test]
    fn a_failed_save_keeps_the_change_for_the_next_one() {
        let dir = test_dir("settings_retry");
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
    }

    // Settings that came from no file hold nothing to compare against: saved
    // over an existing file they write their whole state, as before.
    #[test]
    fn a_settings_from_no_file_writes_its_whole_state() {
        let dir = test_dir("settings_no_file");
        let path = dir.join("settings.toml");
        fs::write(&path, "# hand note\n[compiler]\nlabel = 'x'\nstale = 1\n").unwrap();

        let mut settings = Settings::default();
        settings.common.thread_count = 8;
        settings.set_tool("compiler", table("label = 'x'"));
        settings.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# hand note\n"), "{text}");
        assert_eq!(Settings::load(&path).unwrap(), settings);
    }

    // A default a tool fills in at start is no change of the session's: a key the file
    // lacks, and a section it lacks, changed and saved by one session are not put back
    // to their defaults by the other's save.
    #[test]
    fn a_default_filled_in_at_start_is_not_the_sessions_change() {
        let dir = test_dir("settings_defaults");
        let path = dir.join("settings.toml");
        fs::write(
            &path,
            "[common]\ntheme = \"dark\"\n\n[compiler]\nlabel = 'x'\n",
        )
        .unwrap();
        let compiler = table("label = ''\ndt00_overwrite_allow = false");
        let editor = table("speed = 1");

        let mut session_a = Settings::load(&path).unwrap();
        let mut session_b = Settings::load(&path).unwrap();
        for session in [&mut session_a, &mut session_b] {
            session.merge_defaults("compiler", &compiler);
            session.merge_defaults("save-editor", &editor);
        }
        let mut tool = session_a.tool("compiler").unwrap().clone();
        tool["dt00_overwrite_allow"] = Value::Boolean(true);
        session_a.set_tool("compiler", tool);
        session_a.set_tool("save-editor", table("speed = 3"));
        session_a.save(&path).unwrap();
        session_b.common.theme = Theme::Light;
        session_b.save(&path).unwrap();

        let loaded = Settings::load(&path).unwrap();
        assert_eq!(
            loaded.tool("compiler").unwrap()["dt00_overwrite_allow"],
            Value::Boolean(true),
            "B's default did not undo A's change"
        );
        assert_eq!(loaded.tool("save-editor"), Some(&table("speed = 3")));
        assert_eq!(loaded.common.theme, Theme::Light);
    }

    // An edit is kept only when it reads back as the values it meant to leave.
    #[test]
    fn an_edit_that_reads_back_otherwise_or_not_at_all_is_not_kept() {
        let values = table("[common]\nthread_count = 4");
        let good = "# kept\n[common]\nthread_count = 4\n".to_owned();
        assert_eq!(reads_back(good.clone(), &values), Some(good));
        assert_eq!(
            reads_back("[common]\nthread_count = 8\n".to_owned(), &values),
            None,
            "valid TOML holding another value"
        );
        assert_eq!(
            reads_back("[common\n".to_owned(), &values),
            None,
            "not TOML"
        );
    }

    // Equality is what the settings hold: one loaded from a file equals one built to the
    // same values, whatever it remembers of the file, and a difference in the common
    // section, a tool's table or the template is a difference.
    #[test]
    fn settings_compare_by_what_they_hold() {
        let loaded = Settings::parse("[common]\nthread_count = 4\n").unwrap();
        let mut built = Settings::default();
        built.common.thread_count = 4;
        assert_eq!(loaded, built);

        built.common.thread_count = 8;
        assert_ne!(loaded, built, "the common section");
        let mut with_tool = loaded.clone();
        with_tool.set_tool("compiler", table("label = 'x'"));
        assert_ne!(loaded, with_tool, "a tool's table");
        assert_ne!(loaded, loaded.clone().with_template(""), "the template");
    }

    /// The file at `path` read back: its per-version PES folders and its text.
    fn folders_in(path: &Path) -> (BTreeMap<PesVersion, String>, String) {
        let text = fs::read_to_string(path).unwrap();
        (Settings::parse(&text).unwrap().common.pes_folders, text)
    }

    // A table one side lacks is still merged key by key. `[common.pes_folders]` is left
    // out of the file while it is empty, so two sessions that each fill a row start from
    // no table at all: the second's row must not replace the first's.
    #[test]
    fn two_sessions_each_filling_a_folder_row_keep_both() {
        let dir = test_dir("settings_folder_rows");
        let path = dir.join("settings.toml");
        fs::write(&path, "[common]\ntheme = \"dark\"\n").unwrap();

        let mut session_a = Settings::load(&path).unwrap();
        let mut session_b = Settings::load(&path).unwrap();
        let (pes19, pes21) = (r"G:\PES 2019".to_owned(), r"F:\PES2021".to_owned());
        session_a
            .common
            .pes_folders
            .insert(PesVersion::Pes19, pes19.clone());
        session_a.save(&path).unwrap();
        session_b
            .common
            .pes_folders
            .insert(PesVersion::Pes21, pes21.clone());
        session_b.save(&path).unwrap();

        let (folders, text) = folders_in(&path);
        assert_eq!(
            folders,
            BTreeMap::from([(PesVersion::Pes19, pes19), (PesVersion::Pes21, pes21)]),
            "{text}"
        );
    }

    // The other way round: a session that empties its only row removes that row, not
    // the row another session added meanwhile; and with no other row the table goes
    // with it, as a file with no per-version folder has none.
    #[test]
    fn a_session_emptying_its_folder_row_keeps_the_other_sessions_row() {
        let dir = test_dir("settings_folder_emptied");
        let path = dir.join("settings.toml");
        let original = "[common]\ntheme = \"dark\"\n\n[common.pes_folders]\n19 = 'G:\\PES 2019'\n";
        fs::write(&path, original).unwrap();

        let mut session_a = Settings::load(&path).unwrap();
        let mut session_b = Settings::load(&path).unwrap();
        let mut alone = Settings::load(&path).unwrap();
        let pes21 = r"F:\PES2021".to_owned();
        session_a
            .common
            .pes_folders
            .insert(PesVersion::Pes21, pes21.clone());
        session_a.save(&path).unwrap();
        session_b.common.pes_folders.clear();
        session_b.save(&path).unwrap();

        let (folders, text) = folders_in(&path);
        assert_eq!(
            folders,
            BTreeMap::from([(PesVersion::Pes21, pes21)]),
            "{text}"
        );

        fs::write(&path, original).unwrap();
        alone.common.pes_folders.clear();
        alone.save(&path).unwrap();
        let (folders, text) = folders_in(&path);
        assert!(folders.is_empty(), "{text}");
        assert!(
            !text.contains("pes_folders"),
            "no empty table is left: {text}"
        );
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
        assert!(loaded.tool("compiler").is_none());
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
