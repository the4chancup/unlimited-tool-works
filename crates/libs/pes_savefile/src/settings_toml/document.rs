//! The `settings.toml` and `ingame_face.toml` document texts: the headers,
//! `PlayerSettings::parse` and `to_toml`, the in-place `update_toml`
//! rewrite, `IngameFaceSettings::parse` and `to_toml`, and the appearance
//! table readers/emitters both share with `team_toml`.

use toml_edit::{DocumentMut, Item, Value};

use super::keys::{self};
use super::{
    AppearanceSettings, IngameFaceSettings, KeyTable, Kind, MotionSettings, NameSetting,
    PlayerSettings, SettingKey, SettingsError, get_appearance, set_appearance,
};

// The TOML half: `parse` reads a `settings.toml` text, `to_toml` emits the
// template (the spec's key-table block verbatim for a fully set settings),
// `update_toml` rewrites the `Some` values inside an existing document while
// preserving its comments and formatting.

/// The header of a generated `settings.toml`: the spec block's six comment
/// lines (the in-game face's sentence pointing at `ingame_face.toml`
/// included) and its four `name`/`shirt_name` comment lines, verbatim.
const HEADER: &str = "\
# settings.toml, inside a player folder. Every key is optional: an absent key takes the
# value shown here (name and shirt_name: not written). A commented key shows what can be
# set and its range. The file never references the export's models: what a player wears
# is decided by the folder contents (models present, link files pointing at shared
# folders); boots_id/gloves_id only name the game's stock models.
# The in-game face's keys (iris, face, hair) are in ingame_face.toml.

# true = derive from the folder name (\"15 - Snuffy\" gives \"Snuffy\"; the whole folder
# name for players.txt-mapped folders); \"text\" = write as is; absent = not written.
# name and shirt_name are used only when the export's root holds the `write_names` marker;
# otherwise they are ignored and the names in the save stand.
";

/// The header of a generated `ingame_face.toml`: the spec block's four
/// comment lines, verbatim.
const INGAME_FACE_HEADER: &str = "\
# ingame_face.toml, inside a player folder: the in-game face, which the game draws when
# the folder holds no face model and no face link. The file marks the folder as wearing
# it. Every key is optional: an absent key takes the value shown here, and an empty file
# is the game's default face. A commented key shows what can be set and its range.
";

/// The dotted path a settings value reports `key` under, naming the file the
/// key lives in: `ingame_face.toml`'s for a moved key (`iris_color`,
/// `face.facial_hair_type`, `hair.style`), `settings.toml`'s for the rest
/// ("appearance.strip.sleeves"; a top-level key is just its name).
pub(super) fn dotted(key: SettingKey) -> String {
    let spec = key.spec();
    if key.is_ingame_face() {
        leaf_path("", &spec)
    } else if spec.table == KeyTable::Top {
        spec.name.to_string()
    } else {
        format!("{}.{}", spec.table.path(), spec.name)
    }
}

/// The stored bound for `key` in `stored_ranges` (team TOML) mode: the
/// widest bit width the key's field is stored at across the version tables —
/// a `Player` source via `schema::widest_bit_width`, a `Face` source via the
/// ingame-face table. The stored value must fit it; the editor ranges the
/// strict mode enforces do not apply.
fn stored_bound(key: SettingKey) -> i64 {
    let width = match key.source() {
        keys::Source::Player(field) => crate::schema::widest_bit_width(field),
        keys::Source::Face(field) => crate::schema::ingame_face::INGAME_FACE_FIELDS
            .iter()
            .find(|spec| spec.field == field)
            .map(|spec| spec.bit_width)
            .expect("every face field is in the table"),
        // The angle + 1, 0 to 2: coded into `NoseType`, no width of its own.
        keys::Source::NoseAngle => return 2,
    };
    (1i64 << width) - 1
}

/// A key's TOML value for a stored `u8` (label string, signed physique
/// number, 1-based motion number, bool); `OutOfRange` for a value written
/// directly into a public field that the kind cannot represent. In
/// `stored_ranges` mode the bound is the field's stored width, and a
/// width-valid value with no label emits as the integer. `pub(crate)` for
/// `narrow_to`'s report and the aesthetics patch's refusal, each of the
/// value as the file wrote it.
pub(crate) fn toml_form(
    key: SettingKey,
    stored: u8,
    stored_ranges: bool,
) -> Result<Value, SettingsError> {
    let spec = &key.spec();
    if stored_ranges {
        let max = stored_bound(key);
        if i64::from(stored) > max {
            return Err(SettingsError::OutOfRange {
                key: dotted(key),
                value: i64::from(stored),
                range: format!("0 to {max}"),
            });
        }
    } else if !spec.kind.accepts_stored(stored) {
        return Err(SettingsError::OutOfRange {
            key: dotted(key),
            value: i64::from(stored),
            range: range_text(spec.kind),
        });
    }
    Ok(match spec.kind {
        Kind::Number { .. } => i64::from(stored).into(),
        Kind::Signed { limit } => (i64::from(stored) - i64::from(limit)).into(),
        Kind::OneBased { .. } => (i64::from(stored) + 1).into(),
        Kind::PlusOne { .. } => (i64::from(stored) - 1).into(),
        // Stored mode's width check lets through a preset id the editor
        // never writes; it has no TOML index, so it is refused here.
        Kind::Mapped(map) => match map.iter().position(|id| *id == stored) {
            Some(index) => i64::try_from(index).expect("a short map").into(),
            None => {
                return Err(SettingsError::OutOfRange {
                    key: dotted(key),
                    value: i64::from(stored),
                    range: range_text(spec.kind),
                });
            }
        },
        Kind::Bool => (stored != 0).into(),
        // Strict mode's accepts_stored bounds `stored` to a label; stored
        // mode falls back to the integer for a width-valid value with none.
        Kind::Labels(labels) => labels
            .get(usize::from(stored))
            .map_or_else(|| i64::from(stored).into(), |label| (*label).into()),
    })
}

/// The range text a `Kind` reports (`"0 to 7"`, `"-7 to 7"`, `"1 to 10"`,
/// `"true, false"`, or the quoted label list). `pub(crate)` for the
/// aesthetics patch, whose `OutOfRange` carries the same text.
pub(crate) fn range_text(kind: Kind) -> String {
    match kind {
        Kind::Number { min, max } => format!("{min} to {max}"),
        Kind::Signed { limit } => format!("-{limit} to {limit}"),
        Kind::OneBased { max } => format!("1 to {max}"),
        Kind::PlusOne { max } => format!("-1 to {max}"),
        Kind::Mapped(map) => format!("0 to {}", map.len() - 1),
        Kind::Bool => "true, false".to_string(),
        Kind::Labels(labels) => labels
            .iter()
            .map(|label| format!("\"{label}\""))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// `body` padded so `#` starts at character 33 (1-based); a body longer than
/// the pad still gets one space before `#`. An empty comment leaves `body`
/// unpadded.
pub(crate) fn padded(body: &str, comment: &str) -> String {
    if comment.is_empty() {
        return body.to_string();
    }
    let pad = 32_usize.saturating_sub(body.len()).max(1);
    format!("{body}{}# {comment}", " ".repeat(pad))
}

/// `root` and `name` joined by a dot; `name` alone when the root is empty
/// (`ingame_face.toml`'s keys sit at the file's top level).
fn dotted_join(root: &str, name: &str) -> String {
    if root.is_empty() {
        name.to_string()
    } else {
        format!("{root}.{name}")
    }
}

/// The key's dotted path under `root` (`root` = the appearance table's own
/// path: "appearance" in settings.toml, "players.03.appearance" in team.toml,
/// "" in ingame_face.toml); a `Top` key never reaches it — its callers
/// filter `Top` first.
pub(crate) fn leaf_path(root: &str, spec: &keys::KeySpec) -> String {
    dotted_join(&absolute(root, spec.table), spec.name)
}

/// The item at a key's `(table, name)` inside the `appearance` table, `None`
/// when absent; `path` is the appearance table's dotted path for error keys;
/// a `Top` key never reaches it — its callers filter `Top` first.
fn lookup<'a>(
    appearance: &'a dyn toml_edit::TableLike,
    spec: &keys::KeySpec,
    path: &str,
) -> Result<Option<&'a Item>, SettingsError> {
    let table = match spec.table.sub_table() {
        None => appearance,
        Some(sub) => {
            let Some(item) = appearance.get(sub) else {
                return Ok(None);
            };
            match item.as_table_like() {
                Some(table) => table,
                None => {
                    return Err(SettingsError::WrongType {
                        key: dotted_join(path, sub),
                        expected: "a table",
                    });
                }
            }
        }
    };
    Ok(table.get(spec.name))
}

/// The stored `u8` behind one present item, range-checked per its `Kind`.
/// `stored_ranges` is `team_toml`'s mode: the editor range is documentation,
/// and any value the field's widest stored bit width can hold is accepted
/// (a real save's face types and celebration numbers go past the selectable
/// range, but not past the storage).
fn value(
    dotted: &str,
    key: SettingKey,
    item: &Item,
    stored_ranges: bool,
) -> Result<u8, SettingsError> {
    let kind = key.spec().kind;
    let wide = |value: i64| -> Result<u8, SettingsError> {
        let max = stored_bound(key);
        if !(0..=max).contains(&value) {
            return Err(SettingsError::OutOfRange {
                key: dotted.to_string(),
                value,
                range: format!("0 to {max}"),
            });
        }
        Ok(u8::try_from(value).expect("bounded by max"))
    };
    let integer = |expected: &'static str| -> Result<i64, SettingsError> {
        item.as_value()
            .and_then(|v| v.as_integer())
            .ok_or_else(|| SettingsError::WrongType {
                key: dotted.to_string(),
                expected,
            })
    };
    match kind {
        Kind::Number { min, max } => {
            let value = integer("an integer")?;
            if stored_ranges {
                return wide(value);
            }
            if !(i64::from(min)..=i64::from(max)).contains(&value) {
                return Err(SettingsError::OutOfRange {
                    key: dotted.to_string(),
                    value,
                    range: range_text(kind),
                });
            }
            Ok(u8::try_from(value).expect("the range check bounds it"))
        }
        Kind::Signed { limit } => {
            let limit = i64::from(limit);
            let value = integer("an integer")?;
            if stored_ranges {
                let Some(shifted) = value.checked_add(limit) else {
                    return Err(SettingsError::OutOfRange {
                        key: dotted.to_string(),
                        value,
                        range: format!("0 to {}", stored_bound(key)),
                    });
                };
                return wide(shifted);
            }
            if !(-limit..=limit).contains(&value) {
                return Err(SettingsError::OutOfRange {
                    key: dotted.to_string(),
                    value,
                    range: range_text(kind),
                });
            }
            Ok(u8::try_from(value + limit).expect("in 0..=2 * limit"))
        }
        Kind::PlusOne { max } => {
            let value = integer("an integer")?;
            if stored_ranges {
                let Some(shifted) = value.checked_add(1) else {
                    return Err(SettingsError::OutOfRange {
                        key: dotted.to_string(),
                        value,
                        range: format!("0 to {}", stored_bound(key)),
                    });
                };
                return wide(shifted);
            }
            if !(-1..=i64::from(max)).contains(&value) {
                return Err(SettingsError::OutOfRange {
                    key: dotted.to_string(),
                    value,
                    range: range_text(kind),
                });
            }
            Ok(u8::try_from(value + 1).expect("in 0..=max + 1"))
        }
        // Both modes: only an index has a stored id to map to.
        Kind::Mapped(map) => {
            let value = integer("an integer")?;
            usize::try_from(value)
                .ok()
                .and_then(|index| map.get(index))
                .copied()
                .ok_or_else(|| SettingsError::OutOfRange {
                    key: dotted.to_string(),
                    value,
                    range: range_text(kind),
                })
        }
        Kind::OneBased { max } => {
            let value = integer("an integer")?;
            if stored_ranges {
                let Some(shifted) = value.checked_sub(1) else {
                    return Err(SettingsError::OutOfRange {
                        key: dotted.to_string(),
                        value,
                        range: format!("0 to {}", stored_bound(key)),
                    });
                };
                return wide(shifted);
            }
            if !(1..=i64::from(max)).contains(&value) {
                return Err(SettingsError::OutOfRange {
                    key: dotted.to_string(),
                    value,
                    range: range_text(kind),
                });
            }
            Ok(u8::try_from(value - 1).expect("in 0..=max-1"))
        }
        Kind::Bool => item
            .as_value()
            .and_then(|v| v.as_bool())
            .map(u8::from)
            .ok_or_else(|| SettingsError::WrongType {
                key: dotted.to_string(),
                expected: "true or false",
            }),
        Kind::Labels(labels) => {
            if let Some(text) = item.as_value().and_then(|v| v.as_str()) {
                labels
                    .iter()
                    .position(|label| *label == text)
                    .map(|index| u8::try_from(index).expect("labels fit u8"))
                    .ok_or_else(|| SettingsError::UnknownLabel {
                        key: dotted.to_string(),
                        label: text.to_string(),
                        allowed: range_text(kind),
                    })
            } else if stored_ranges {
                // A width-valid value with no label round-trips as the
                // integer the emitter wrote.
                let value = integer("a label string or an integer")?;
                wide(value)
            } else {
                Err(SettingsError::WrongType {
                    key: dotted.to_string(),
                    expected: "a string",
                })
            }
        }
    }
}

/// The spec's table path under the appearance root `root` (`Appearance` →
/// `root`, `Physique` → `root.physique`; a top-level key's table is the root
/// itself, though its callers filter `Top` first).
fn absolute(root: &str, table: KeyTable) -> String {
    match table.sub_table() {
        None => root.to_string(),
        Some(sub) => dotted_join(root, sub),
    }
}

/// Everything the value walk did not consume is refused. A leaf is known
/// iff its dotted path is `name` or some key's; a table is known iff its
/// path is `appearance` or some key's table. The first unknown item is
/// `UnknownKey`; a table position holding a non-table is `WrongType`.
fn reject_unknown(document: &DocumentMut) -> Result<(), SettingsError> {
    for (name, item) in document.iter() {
        match name {
            "name" | "shirt_name" => {}
            "appearance" => reject_unknown_table(
                item,
                "appearance",
                "appearance",
                AppearanceDocument::Settings,
            )?,
            _ if SettingKey::ALL
                .iter()
                .any(|key| key.spec().table == KeyTable::Top && key.spec().name == name) => {}
            _ => {
                return Err(SettingsError::UnknownKey {
                    key: name.to_string(),
                });
            }
        }
    }
    Ok(())
}

/// The recursive half of [`reject_unknown`] and [`parse_appearance`]: `root`
/// is the appearance table's own dotted path, `path` the table being walked,
/// `document` which keys the walk holds.
fn reject_unknown_table(
    item: &Item,
    root: &str,
    path: &str,
    document: AppearanceDocument,
) -> Result<(), SettingsError> {
    let Some(table) = item.as_table_like() else {
        return Err(SettingsError::WrongType {
            key: path.to_string(),
            expected: "a table",
        });
    };
    for (leaf, sub) in table.iter() {
        let child = dotted_join(path, leaf);
        if SettingKey::ALL
            .iter()
            .any(|key| document.holds(*key) && absolute(root, key.spec().table) == child)
        {
            reject_unknown_table(sub, root, &child, document)?;
        } else if !SettingKey::ALL.iter().any(|key| {
            document.holds(*key)
                && absolute(root, key.spec().table) == path
                && key.spec().name == leaf
        }) {
            return Err(SettingsError::UnknownKey { key: child });
        }
    }
    Ok(())
}

/// A top-level model-ID key's parsed value: `None` when absent or `""`
/// (both mean default), `Some` for an integer inside the key's range. Any
/// other string is `WrongType`; a non-integer defers to the `Number` kind's
/// errors.
fn model_id(document: &DocumentMut, key: SettingKey) -> Result<Option<u8>, SettingsError> {
    let name = key.spec().name;
    let Some(item) = document.get(name) else {
        return Ok(None);
    };
    if let Some(text) = item.as_value().and_then(|v| v.as_str()) {
        return if text.is_empty() {
            Ok(None)
        } else {
            Err(SettingsError::WrongType {
                key: name.to_string(),
                expected: "an integer or \"\"",
            })
        };
    }
    value(name, key, item, false).map(Some)
}

/// Which document an appearance table is parsed from or emitted into:
/// `settings.toml`'s `[appearance]`, Team TOML's `[players.NN.appearance]`,
/// or `ingame_face.toml` (the in-game face's keys alone, at the top level).
/// The document settles the value bounds, which keys the walks visit and
/// the `[appearance.motion]` note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppearanceDocument {
    /// `settings.toml`: the editor ranges the key comments document apply,
    /// and the motion table carries its note (a settings apply writes the
    /// motions through the aesthetics patch). The in-game face's keys are
    /// `ingame_face.toml`'s: one left here is an unknown key.
    Settings,
    /// Team TOML's `[players.NN.appearance]`: bounds are each field's
    /// widest stored bit width (a real save's values go past the editor
    /// ranges), and no motion note.
    Team,
    /// `ingame_face.toml`: `iris_color` at the top level, `[face]` and
    /// `[hair]` — the `[appearance]` face run with the root dropped. The
    /// editor ranges apply, as `settings.toml`'s; there is no motion note
    /// (the document holds no motion keys).
    IngameFace,
}

impl AppearanceDocument {
    /// Team TOML's range mode: bounds are the field's widest stored bit
    /// width, not the editor ranges `settings.toml` documents.
    fn stored_ranges(self) -> bool {
        matches!(self, Self::Team)
    }

    /// Whether the document holds `key`: `settings.toml` every non-top key
    /// but the in-game face's (which `ingame_face.toml` holds), Team TOML
    /// every non-top key, `ingame_face.toml` the in-game face's alone.
    fn holds(self, key: SettingKey) -> bool {
        match self {
            Self::Settings => key.spec().table != KeyTable::Top && !key.is_ingame_face(),
            Self::Team => key.spec().table != KeyTable::Top,
            Self::IngameFace => key.is_ingame_face(),
        }
    }
}

/// The `[appearance]` table at `item` (dotted path `path`: "appearance" in a
/// settings.toml, "players.03.appearance" in a team.toml, "" for an
/// ingame_face.toml, whose whole document is the table) parsed into stored
/// values. Shared with `team_toml` so the nested key handling has one home.
pub(crate) fn parse_appearance(
    item: &Item,
    path: &str,
    document: AppearanceDocument,
) -> Result<AppearanceSettings, SettingsError> {
    let Some(appearance) = item.as_table_like() else {
        return Err(SettingsError::WrongType {
            key: path.to_string(),
            expected: "a table",
        });
    };
    let mut out = AppearanceSettings::default();
    for key in SettingKey::ALL {
        let spec = key.spec();
        if !document.holds(key) {
            continue; // a key the document does not hold
        }
        let Some(item) = lookup(appearance, &spec, path)? else {
            continue;
        };
        let leaf = leaf_path(path, &spec);
        let stored = value(&leaf, key, item, document.stored_ranges())?;
        set_appearance(&mut out, key, Some(stored));
    }
    reject_unknown_table(item, path, path, document)?;
    Ok(out)
}

/// The `[prefix]` table plus its subtables appended to `out`, in key
/// order: a `Some` emits its line with the key's comment, a `None` the
/// commented line at the key's default. Shared with `team_toml`'s
/// `[players.NN.appearance]` (`AppearanceDocument::Team`).
pub(crate) fn emit_appearance(
    out: &mut String,
    prefix: &str,
    appearance: &AppearanceSettings,
    document: AppearanceDocument,
) -> Result<(), SettingsError> {
    let mut table = KeyTable::Top;
    for key in SettingKey::ALL {
        let spec = key.spec();
        if !document.holds(key) {
            continue; // a key the document does not hold
        }
        if spec.table != table {
            // The motion table's note is the settings template's: a settings
            // apply writes the motions through the aesthetics patch, where a
            // team TOML import writes the save directly.
            let note = if document == AppearanceDocument::Settings && spec.table == KeyTable::Motion
            {
                "on Fox written by the aesthetics patch: the tables do not hold motions"
            } else {
                ""
            };
            let table_path = absolute(prefix, spec.table);
            // The document's own root table has no header line: its keys
            // (`ingame_face.toml`'s `iris_color`) sit at the top level.
            if !table_path.is_empty() {
                let header = padded(&format!("[{table_path}]"), note);
                out.push_str(&format!("\n{header}\n"));
            }
            table = spec.table;
        }
        let stored = get_appearance(appearance, key);
        let body = match stored {
            Some(stored) => format!(
                "{} = {}",
                spec.name,
                toml_form(key, stored, document.stored_ranges())?
            ),
            None => {
                let default = spec.default.expect("a non-Top key carries a default");
                format!(
                    "{} = {}",
                    spec.name,
                    toml_form(key, default, document.stored_ranges())?
                )
            }
        };
        let line = padded(&body, spec.comment);
        match stored {
            Some(_) => out.push_str(&line),
            None => out.push_str(&format!("# {line}")),
        }
        out.push('\n');
    }
    Ok(())
}

/// A `[…motion]` table parsed into stored values: the motion keys' labels,
/// ranges and stored conversions, nothing else. `path` is the table's own
/// dotted path for error keys (`appearance.motion` in a settings.toml,
/// `teams[0].players.05.appearance.motion` in the aesthetics patch, which
/// shares this so there is no second key table). `settings.toml`'s ranges
/// apply.
pub(crate) fn parse_motion(item: &Item, path: &str) -> Result<MotionSettings, SettingsError> {
    let Some(table) = item.as_table_like() else {
        return Err(SettingsError::WrongType {
            key: path.to_string(),
            expected: "a table",
        });
    };
    let mut out = AppearanceSettings::default();
    for key in SettingKey::ALL {
        let spec = key.spec();
        if spec.table != KeyTable::Motion {
            continue;
        }
        let Some(item) = table.get(spec.name) else {
            continue;
        };
        let leaf = format!("{path}.{}", spec.name);
        let stored = value(&leaf, key, item, false)?;
        set_appearance(&mut out, key, Some(stored));
    }
    for (name, _) in table.iter() {
        if !SettingKey::ALL
            .iter()
            .any(|key| key.spec().table == KeyTable::Motion && key.spec().name == name)
        {
            return Err(SettingsError::UnknownKey {
                key: format!("{path}.{name}"),
            });
        }
    }
    Ok(out.motion)
}

/// Each `Some` motion key's `settings.toml` line, in key-table order —
/// `name = value` plus the key's comment — appended to `out`. The caller
/// writes the `[…motion]` header; the aesthetics patch emits the `Some`
/// keys only, which this already is. The editor ranges `parse_motion`
/// reads apply, so `Err` is a stored value `parse` would refuse back — only
/// reachable on a hand-built `MotionSettings`.
pub(crate) fn emit_motion(out: &mut String, motion: &MotionSettings) -> Result<(), SettingsError> {
    let appearance = AppearanceSettings {
        motion: motion.clone(),
        ..AppearanceSettings::default()
    };
    for key in SettingKey::ALL {
        let spec = key.spec();
        if spec.table != KeyTable::Motion {
            continue;
        }
        let Some(stored) = get_appearance(&appearance, key) else {
            continue;
        };
        let value = toml_form(key, stored, false)?;
        out.push_str(&padded(&format!("{} = {}", spec.name, value), spec.comment));
        out.push('\n');
    }
    Ok(())
}

impl PlayerSettings {
    /// Reads a `settings.toml` text. Every key is optional; anything the key
    /// table does not know (a stray table, a compiler-owned field such as
    /// `boots_id`, or an in-game face key — they are `ingame_face.toml`'s) is
    /// `UnknownKey`. The first error met, in `SettingKey::ALL` order then the
    /// unknown-key sweep, is the one reported.
    pub fn parse(text: &str) -> Result<Self, SettingsError> {
        let document: DocumentMut = text
            .parse()
            .map_err(|e: toml_edit::TomlError| SettingsError::Toml(e.to_string()))?;
        let mut settings = PlayerSettings::default();
        if let Some(item) = document.get("name") {
            settings.name = Some(match item.as_value() {
                Some(Value::Boolean(b)) if *b.value() => NameSetting::FromFolder,
                Some(Value::String(s)) if s.value().contains('\0') => {
                    return Err(SettingsError::WrongType {
                        key: "name".to_string(),
                        expected: "a string without NUL",
                    });
                }
                Some(Value::String(s)) => NameSetting::Explicit(s.value().clone()),
                _ => {
                    return Err(SettingsError::WrongType {
                        key: "name".to_string(),
                        expected: "true or a string",
                    });
                }
            });
        }
        if let Some(item) = document.get("shirt_name") {
            settings.shirt_name = Some(match item.as_value() {
                Some(Value::String(s)) if s.value().contains('\0') => {
                    return Err(SettingsError::WrongType {
                        key: "shirt_name".to_string(),
                        expected: "a string without NUL",
                    });
                }
                Some(Value::String(s)) => s.value().clone(),
                _ => {
                    return Err(SettingsError::WrongType {
                        key: "shirt_name".to_string(),
                        expected: "a string",
                    });
                }
            });
        }
        settings.boots_id = model_id(&document, SettingKey::BootsId)?;
        settings.gloves_id = model_id(&document, SettingKey::GlovesId)?;
        if let Some(item) = document.get("appearance") {
            settings.appearance =
                parse_appearance(item, "appearance", AppearanceDocument::Settings)?;
        }
        reject_unknown(&document)?;
        Ok(settings)
    }

    /// The template `settings.toml`: the spec block verbatim for a settings
    /// where every key is `Some`; an unset key is its line commented out at
    /// the key's default (`KeySpec::default`). `OutOfRange` for a value
    /// written directly into a public field that the key's kind cannot
    /// represent.
    pub fn to_toml(&self) -> Result<String, SettingsError> {
        let mut out = HEADER.to_string();
        match &self.name {
            Some(NameSetting::FromFolder) => out.push_str("name = true\n"),
            Some(NameSetting::Explicit(name)) => {
                out.push_str(&format!("name = {}\n", Value::from(name.as_str())));
            }
            None => out.push_str("# name = true\n"),
        }
        let shirt_name = padded(
            &format!(
                "shirt_name = {}",
                Value::from(self.shirt_name.as_deref().unwrap_or(""))
            ),
            "\"text\" = write as is; absent = not written",
        );
        match &self.shirt_name {
            Some(_) => out.push_str(&format!("{shirt_name}\n")),
            None => out.push_str(&format!("# {shirt_name}\n")),
        }
        // The stock-model IDs are uncommented in every case: `""` is what a
        // template shows for default (TOML has no bare `key =`).
        for (key, id) in [
            (SettingKey::BootsId, self.boots_id),
            (SettingKey::GlovesId, self.gloves_id),
        ] {
            let spec = key.spec();
            let body = match id {
                Some(id) => format!("{} = {}", spec.name, toml_form(key, id, false)?),
                None => format!("{} = \"\"", spec.name),
            };
            out.push_str(&padded(&body, spec.comment));
            out.push('\n');
        }
        emit_appearance(
            &mut out,
            "appearance",
            &self.appearance,
            AppearanceDocument::Settings,
        )?;
        Ok(out)
    }

    /// Rewrites the `Some` values of an existing document, preserving its
    /// comments and formatting; `None` keys are left as the user wrote them
    /// (a commented key stays commented). Missing tables are created as
    /// standard tables; no comments are added. A table position
    /// (`appearance` or one of its subtables) holding a non-table item
    /// is `WrongType`. An in-game face key is never written:
    /// `ingame_face.toml` holds them.
    pub fn update_toml(&self, document: &mut DocumentMut) -> Result<(), SettingsError> {
        fn set(item: &mut Item, value: Value) {
            let decor = item.as_value().map(|old| old.decor().clone());
            *item = Item::Value(value);
            if let (Some(decor), Item::Value(new)) = (decor, item) {
                *new.decor_mut() = decor;
            }
        }

        match &self.name {
            Some(NameSetting::FromFolder) => set(&mut document["name"], true.into()),
            Some(NameSetting::Explicit(name)) => {
                set(&mut document["name"], name.as_str().into());
            }
            None => {}
        }
        if let Some(shirt_name) = &self.shirt_name {
            set(&mut document["shirt_name"], shirt_name.as_str().into());
        }
        for (key, id) in [
            (SettingKey::BootsId, self.boots_id),
            (SettingKey::GlovesId, self.gloves_id),
        ] {
            if let Some(id) = id {
                set(&mut document[key.spec().name], toml_form(key, id, false)?);
            }
        }
        for key in SettingKey::ALL {
            let spec = key.spec();
            if spec.table == KeyTable::Top || key.is_ingame_face() {
                continue; // top-level keys were written above; the in-game face's are `ingame_face.toml`'s
            }
            let Some(stored) = self.get(key) else {
                continue;
            };
            let item = table_at(document, spec.table.path())?;
            set(&mut item[spec.name], toml_form(key, stored, false)?);
        }
        Ok(())
    }
}

impl IngameFaceSettings {
    /// Reads an `ingame_face.toml` text: `iris_color` at the top level,
    /// `[face]` and `[hair]` — the `[appearance]` face run of a
    /// `settings.toml` with the root dropped, in the same ranges. Every key
    /// is optional, an empty text the default face; anything the document
    /// does not hold is `UnknownKey` with its path.
    pub fn parse(text: &str) -> Result<Self, SettingsError> {
        let document: DocumentMut = text
            .parse()
            .map_err(|e: toml_edit::TomlError| SettingsError::Toml(e.to_string()))?;
        let appearance = parse_appearance(document.as_item(), "", AppearanceDocument::IngameFace)?;
        Ok(Self {
            iris_color: appearance.iris_color,
            face: appearance.face,
            hair: appearance.hair,
        })
    }

    /// The template `ingame_face.toml`: the header, then the `iris_color`
    /// line and the `[face]` and `[hair]` tables — `settings.toml`'s key
    /// lines with the `[appearance]` root dropped. An unset key is its line
    /// commented out at the key's default (`KeySpec::default`). `OutOfRange`
    /// for a value written directly into a public field that the key's kind
    /// cannot represent.
    pub fn to_toml(&self) -> Result<String, SettingsError> {
        let mut out = INGAME_FACE_HEADER.to_string();
        out.push('\n');
        let appearance = AppearanceSettings {
            iris_color: self.iris_color,
            face: self.face.clone(),
            hair: self.hair.clone(),
            ..AppearanceSettings::default()
        };
        emit_appearance(&mut out, "", &appearance, AppearanceDocument::IngameFace)?;
        Ok(out)
    }
}

/// The table item at `table_path` ("appearance.strip"), created as a
/// standard table where the document lacks it; a position holding a
/// non-table is `WrongType` naming the walked path.
fn table_at<'a>(
    document: &'a mut DocumentMut,
    table_path: &str,
) -> Result<&'a mut Item, SettingsError> {
    let mut item = document.as_item_mut();
    let mut path = String::new();
    for segment in table_path.split('.') {
        if !path.is_empty() {
            path.push('.');
        }
        path.push_str(segment);
        let child = match item {
            Item::Table(table) => {
                if !table.contains_key(segment) {
                    table.insert(segment, Item::Table(toml_edit::Table::new()));
                }
                table.get_mut(segment).expect("just inserted")
            }
            Item::Value(Value::InlineTable(_)) => {
                let child = &mut item[segment];
                if matches!(child, Item::None) {
                    *child = Item::Value(Value::InlineTable(toml_edit::InlineTable::new()));
                }
                child
            }
            _ => {
                return Err(SettingsError::WrongType {
                    key: path,
                    expected: "a table",
                });
            }
        };
        match child {
            Item::Table(_) | Item::Value(Value::InlineTable(_)) => item = child,
            _ => {
                return Err(SettingsError::WrongType {
                    key: path,
                    expected: "a table",
                });
            }
        }
    }
    Ok(item)
}
