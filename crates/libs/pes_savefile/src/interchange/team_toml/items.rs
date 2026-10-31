//! `team.toml`'s shared TOML item helpers: the leaf readers, the emit line
//! and the apply-side checks that `team`, `tactics` and `player` all go
//! through.

use toml_edit::{Item, TableLike};

use crate::codec::{TextMisfit, text_misfit};
use crate::interchange::team_toml::{ImportNote, TeamTomlError};
use crate::model::team::{KitSlot, TeamColor};
use crate::settings_toml::document::padded;

/// The TOML text of a `Some` value, or `neutral` for a `None` key.
pub(super) fn value_or(value: Option<String>, neutral: &str) -> String {
    value.unwrap_or_else(|| neutral.to_string())
}

/// One line: `name = value` with the comment at column 33, commented out with
/// the neutral value when the key is `None`.
pub(super) fn emit(
    out: &mut String,
    name: &str,
    value: Option<String>,
    neutral: &str,
    comment: &str,
) {
    let set = value.is_some();
    let body = format!("{name} = {}", value_or(value, neutral));
    let line = padded(&body, comment);
    if set {
        out.push_str(&line);
    } else {
        out.push_str(&format!("# {line}"));
    }
    out.push('\n');
}

// ---------------------------------------------------------------------------
// Value helpers

pub(super) fn as_table<'a>(item: &'a Item, key: &str) -> Result<&'a dyn TableLike, TeamTomlError> {
    item.as_table_like()
        .ok_or_else(|| TeamTomlError::WrongType {
            key: key.to_string(),
            expected: "a table",
        })
}

pub(super) fn integer(item: &Item, key: &str) -> Result<i64, TeamTomlError> {
    item.as_value()
        .and_then(|v| v.as_integer())
        .ok_or_else(|| TeamTomlError::WrongType {
            key: key.to_string(),
            expected: "an integer",
        })
}

pub(super) fn ranged(item: &Item, key: &str, min: i64, max: i64) -> Result<u8, TeamTomlError> {
    let value = integer(item, key)?;
    if !(min..=max).contains(&value) {
        return Err(TeamTomlError::OutOfRange {
            key: key.to_string(),
            value,
            range: format!("{min} to {max}"),
        });
    }
    u8::try_from(value).map_err(|_| TeamTomlError::OutOfRange {
        key: key.to_string(),
        value,
        range: format!("{min} to {max}"),
    })
}

pub(super) fn u8_val(item: &Item, key: &str) -> Result<u8, TeamTomlError> {
    ranged(item, key, 0, 255)
}

pub(super) fn u16_val(item: &Item, key: &str) -> Result<u16, TeamTomlError> {
    let value = integer(item, key)?;
    u16::try_from(value).map_err(|_| TeamTomlError::OutOfRange {
        key: key.to_string(),
        value,
        range: "0 to 65535".to_string(),
    })
}

pub(super) fn u32_val(item: &Item, key: &str) -> Result<u32, TeamTomlError> {
    let value = integer(item, key)?;
    u32::try_from(value).map_err(|_| TeamTomlError::OutOfRange {
        key: key.to_string(),
        value,
        range: "0 to 4294967295".to_string(),
    })
}

pub(super) fn boolean(item: &Item, key: &str) -> Result<bool, TeamTomlError> {
    item.as_value()
        .and_then(|v| v.as_bool())
        .ok_or_else(|| TeamTomlError::WrongType {
            key: key.to_string(),
            expected: "true or false",
        })
}

pub(super) fn text(item: &Item, key: &str) -> Result<String, TeamTomlError> {
    let text =
        item.as_value()
            .and_then(|v| v.as_str())
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected: "a string",
            })?;
    // The codec's text fields read up to the first NUL; an embedded NUL
    // would reload truncated, so it is refused like settings.toml's.
    if text.contains('\0') {
        return Err(TeamTomlError::WrongType {
            key: key.to_string(),
            expected: "a string without NUL",
        });
    }
    Ok(text.to_string())
}

pub(super) fn label(item: &Item, key: &str, labels: &[&str]) -> Result<bool, TeamTomlError> {
    let text =
        item.as_value()
            .and_then(|v| v.as_str())
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected: "a string",
            })?;
    Ok(label_position(text, key, labels)? == 1)
}

/// `text`'s index in `labels`; a string none of them spells is
/// `UnknownLabel` at `key`, listing them all.
pub(super) fn label_position(
    text: &str,
    key: &str,
    labels: &[&str],
) -> Result<usize, TeamTomlError> {
    labels
        .iter()
        .position(|label| *label == text)
        .ok_or_else(|| TeamTomlError::UnknownLabel {
            key: key.to_string(),
            label: text.to_string(),
            allowed: labels
                .iter()
                .map(|label| format!("\"{label}\""))
                .collect::<Vec<_>>()
                .join(", "),
        })
}

pub(super) fn int_array<const N: usize>(item: &Item, key: &str) -> Result<[u8; N], TeamTomlError> {
    let array =
        item.as_value()
            .and_then(|v| v.as_array())
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected: "an array of integers",
            })?;
    let mut out = [0u8; N];
    if array.len() != N {
        return Err(TeamTomlError::WrongType {
            key: key.to_string(),
            expected: "an array of integers",
        });
    }
    for (slot, value) in out.iter_mut().zip(array.iter()) {
        *slot = ranged(&Item::Value(value.clone()), key, 0, 255)?;
    }
    Ok(out)
}

pub(super) fn color(item: &Item, key: &str) -> Result<TeamColor, TeamTomlError> {
    let array =
        item.as_value()
            .and_then(|v| v.as_array())
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected: "an array of three 0 to 63 integers",
            })?;
    if array.len() != 3 {
        return Err(TeamTomlError::WrongType {
            key: key.to_string(),
            expected: "an array of three 0 to 63 integers",
        });
    }
    let mut channels = [0u8; 3];
    for (channel, value) in channels.iter_mut().zip(array.iter()) {
        *channel = ranged(&Item::Value(value.clone()), key, 0, 63)?;
    }
    Ok(TeamColor {
        red: channels[0],
        green: channels[1],
        blue: channels[2],
    })
}

pub(super) fn kit_slots(item: &Item, key: &str) -> Result<[KitSlot; 10], TeamTomlError> {
    let array =
        item.as_value()
            .and_then(|v| v.as_array())
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected: "an array of ten { number, binding } tables",
            })?;
    if array.len() != 10 {
        return Err(TeamTomlError::WrongType {
            key: key.to_string(),
            expected: "an array of ten { number, binding } tables",
        });
    }
    let mut slots = [KitSlot::default(); 10];
    for (i, (slot, value)) in slots.iter_mut().zip(array.iter()).enumerate() {
        let entry = value
            .as_inline_table()
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected: "an array of ten { number, binding } tables",
            })?;
        reject(entry, &format!("{key}.{i}"), &["number", "binding"])?;
        let number = entry
            .get("number")
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected: "an array of ten { number, binding } tables",
            })?;
        let binding = entry
            .get("binding")
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected: "an array of ten { number, binding } tables",
            })?;
        let binding = integer(&Item::Value(binding.clone()), key)?;
        *slot = KitSlot {
            number: ranged(&Item::Value(number.clone()), key, 0, 255)?,
            binding: u32::try_from(binding).map_err(|_| TeamTomlError::OutOfRange {
                key: key.to_string(),
                value: binding,
                range: "0 to 4294967295".to_string(),
            })?,
        };
    }
    Ok(slots)
}

/// [`opt`] where the key is required once its table is present.
pub(super) fn required<T>(
    table: &dyn TableLike,
    path: &str,
    name: &str,
    parse: impl Fn(&Item, &str) -> Result<T, TeamTomlError>,
) -> Result<T, TeamTomlError> {
    opt(table, path, name, parse)?.ok_or_else(|| TeamTomlError::MissingKey {
        key: format!("{path}.{name}"),
    })
}

/// `table.name` parsed, `None` when absent; `key` is the dotted path prefix.
pub(super) fn opt<T>(
    table: &dyn TableLike,
    path: &str,
    name: &str,
    parse: impl Fn(&Item, &str) -> Result<T, TeamTomlError>,
) -> Result<Option<T>, TeamTomlError> {
    let Some(item) = table.get(name) else {
        return Ok(None);
    };
    parse(item, &format!("{path}.{name}")).map(Some)
}

/// Every leaf of `table` must be in `known`; the first stranger is
/// `UnknownKey` at its dotted path.
pub(super) fn reject(
    table: &dyn TableLike,
    path: &str,
    known: &[&str],
) -> Result<(), TeamTomlError> {
    for (name, _) in table.iter() {
        if !known.contains(&name) {
            return Err(TeamTomlError::UnknownKey {
                key: format!("{path}.{name}"),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Apply

/// `Ok(true)` when the key applies; a foreign document gets a note, a
/// same-version document an error.
pub(super) fn gated(
    notes: &mut Vec<ImportNote>,
    foreign: bool,
    supported: bool,
    path: &str,
) -> Result<bool, TeamTomlError> {
    if supported {
        return Ok(true);
    }
    if foreign {
        notes.push(ImportNote::NotInThisVersion {
            path: path.to_string(),
        });
        Ok(false)
    } else {
        Err(TeamTomlError::NotInThisVersion {
            path: path.to_string(),
        })
    }
}

/// Maps `text_misfit`'s verdict for the field at `path` onto this document's
/// errors.
pub(super) fn check_text(
    path: &str,
    text: &str,
    max: usize,
    single_byte: bool,
) -> Result<(), TeamTomlError> {
    match text_misfit(text, max, single_byte) {
        None => Ok(()),
        Some(TextMisfit::Nul) => Err(TeamTomlError::WrongType {
            key: path.to_string(),
            expected: "a string without NUL",
        }),
        Some(TextMisfit::BeyondSingleByte) => Err(TeamTomlError::WrongType {
            key: path.to_string(),
            expected: "a string of chars at most U+00FF",
        }),
        Some(TextMisfit::TooLong { max }) => Err(TeamTomlError::TextTooLong {
            path: path.to_string(),
            max,
        }),
    }
}

/// The apply-side width rule: a stored value the target's `width`-bit field
/// cannot hold would fail the codec's writer after `apply` returned Ok, so
/// it is refused up front. No field is wider than 32 bits, so the shift
/// cannot overflow.
pub(super) fn check_width(path: &str, value: u32, width: u32) -> Result<(), TeamTomlError> {
    let max = (1u64 << width) - 1;
    if u64::from(value) > max {
        return Err(TeamTomlError::OutOfRange {
            key: path.to_string(),
            value: i64::from(value),
            range: format!("0 to {max}"),
        });
    }
    Ok(())
}
