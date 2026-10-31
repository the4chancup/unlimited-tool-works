//! The generic record codec: the one engine that walks a `RecordSchema`'s
//! field table and moves values between record bytes and the model. It knows
//! nothing about versions or encryption; every offset lives in `schema/`.

/// LSB-first bit-run reads and writes.
pub(crate) mod bits;
/// The player record codec.
mod player;
/// The team, roster and tactics record codecs.
mod team;

pub use player::{read_player, read_player_into, write_player};
pub(crate) use team::tactics_runs;
pub use team::{
    read_roster_into, read_tactics_into, read_team, write_roster, write_tactics, write_team,
};

use pes_version::PesVersion;

use crate::schema::{RecordSchema, TextSpec};

/// A player/team record could not be decoded or re-encoded.
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    /// A model value does not fit the schema's bit run.
    #[error("{field}: value {value} does not fit a {width}-bit run")]
    ValueTooWide {
        /// The field whose value is too wide.
        field: String,
        /// The offending value.
        value: u32,
        /// The run's width in bits.
        width: u32,
    },
    /// The version's schema stores a field the player has no value for.
    #[error("{field}: the version stores this field but the player has no value for it")]
    Missing {
        /// The gated field that is `None`.
        field: String,
    },
    /// A text field could not be decoded or encoded.
    #[error("text field {text}: {reason}")]
    Text {
        /// The field (`"Name"`, `"ShirtName"`).
        text: String,
        /// Why the text failed.
        reason: &'static str,
    },
    /// A record slice is not the schema's size.
    #[error("record is {got} bytes, the schema expects {expected}")]
    RecordSize {
        /// The schema's record size.
        expected: usize,
        /// The slice's actual length.
        got: usize,
    },
    /// The model's ingame-face run is not the schema run's length.
    #[error("ingame-face run is {got} bytes, the schema expects {expected}")]
    RunSize {
        /// The schema's run length.
        expected: usize,
        /// The run's actual length.
        got: usize,
    },
    /// An ingame-face field was read or written on a run that does not reach it.
    #[error(
        "{field}: the player carries no ingame-face run covering it (never read from a record)"
    )]
    NoIngameFaceRun {
        /// The field past the run's end.
        field: String,
    },
    /// An ingame-face field was read or written on a run of the other layout
    /// (a PES 15 hair field on a PES 16 to 21 run, or the reverse).
    #[error("{field}: not in this version's ingame-face run")]
    FieldNotInLayout {
        /// The field the run's layout lacks.
        field: String,
    },
    /// An indexed field named an element past its array.
    #[error("{field}: index {index} is out of range")]
    Index {
        /// The field whose index is out of range.
        field: String,
        /// The offending element index.
        index: u8,
    },
    /// A byte the format treats as boolean holds another value.
    #[error("{field}: byte value {value} is not 0 or 1")]
    NotBoolean {
        /// The boolean field whose byte is out of range.
        field: String,
        /// The offending value.
        value: u32,
    },
    /// A roster or tactics record names another team's id.
    #[error("record team id {got}, expected {expected}")]
    TeamIdMismatch {
        /// The id of the team the record is applied to.
        expected: u32,
        /// The id the record carries.
        got: u32,
    },
    /// A stored playing-style index names no style of the version's list.
    #[error("{version}: playing-style index {value} is not a value of its list")]
    UnknownPlayingStyle {
        /// The version whose list was consulted.
        version: PesVersion,
        /// The stored index (out of range or a hole).
        value: u8,
    },
    /// A stored advanced-instruction value names no instruction of the
    /// version's table.
    #[error("{version}: advanced-instruction value {value} is not a value of its table")]
    UnknownInstruction {
        /// The version whose table was consulted.
        version: PesVersion,
        /// The stored value past the table's end.
        value: u8,
    },
    /// A roster longer than the schema's slot count: the extra slots have
    /// no run to write to and would be silently dropped.
    #[error("roster is {got} slots, the schema's roster record holds {slots}")]
    RosterTooLong {
        /// The schema's roster slot count.
        slots: usize,
        /// The roster's actual length.
        got: usize,
    },
}

/// Every `FieldSpec` and every expanded `ArraySpec` element of a record schema
/// as `(field, bit_offset, bit_width)`; the three record kinds reuse it.
pub(crate) fn runs<F: Copy, T>(
    schema: &RecordSchema<F, T>,
) -> impl Iterator<Item = (F, u32, u32)> + '_ {
    schema
        .fields
        .iter()
        .map(|spec| (spec.field, spec.bit_offset, spec.bit_width))
        .chain(schema.arrays.iter().flat_map(|array| {
            (0..array.count).map(move |i| {
                (
                    (array.make)(i),
                    array.base_bit + u32::from(i) * array.stride_bits,
                    array.bit_width,
                )
            })
        }))
}

/// A version-gated field that is `None` while the schema has a run for it.
pub(crate) fn missing(field: impl std::fmt::Debug) -> CodecError {
    CodecError::Missing {
        field: format!("{field:?}"),
    }
}

/// An indexed variant naming an element past its array.
pub(crate) fn index(field: impl std::fmt::Debug, i: u8) -> CodecError {
    CodecError::Index {
        field: format!("{field:?}"),
        index: i,
    }
}

/// A byte boolean on disk: only 0 and 1 are values, anything else means the
/// read is misaligned rather than a third truth value.
pub(crate) fn boolean(field: impl std::fmt::Debug, value: u32) -> Result<bool, CodecError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(CodecError::NotBoolean {
            field: format!("{field:?}"),
            value,
        }),
    }
}

/// The bytes a text field currently stores: everything up to the first NUL or
/// the field end.
pub(crate) fn text_bytes(record: &[u8], byte_offset: u32, len: u32) -> &[u8] {
    let field = &record[byte_offset as usize..(byte_offset + len) as usize];
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    &field[..end]
}

/// Writes `bytes` at the field's start. A text shorter than the field gets one
/// NUL after it and the remaining bytes of the field are left untouched (real
/// saves carry old bytes there); a text that fills the field exactly is written
/// with no NUL. The caller has already checked `bytes` fits `len`.
pub(crate) fn write_text(record: &mut [u8], byte_offset: u32, len: u32, bytes: &[u8]) {
    let field = &mut record[byte_offset as usize..(byte_offset + len) as usize];
    field[..bytes.len()].copy_from_slice(bytes);
    if bytes.len() < field.len() {
        field[bytes.len()] = 0;
    }
}

/// The single-byte encoding shared by shirt and team short names: each char is
/// one byte U+00XX.
pub(crate) fn single_byte(text: &str, name: &'static str) -> Result<Vec<u8>, CodecError> {
    let mut encoded = Vec::with_capacity(text.len());
    for c in text.chars() {
        if u32::from(c) > 0xFF {
            return Err(CodecError::Text {
                text: name.to_string(),
                reason: "a char is above U+00FF",
            });
        }
        encoded.push(c as u8);
    }
    Ok(encoded)
}

/// Why a text does not fit a record's text field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextMisfit {
    /// A NUL: the codec reads up to the first one, so the text would reload cut.
    Nul,
    /// A char above U+00FF in a single-byte field.
    BeyondSingleByte,
    /// Longer than the field: UTF-8 bytes for a name, chars for a single-byte text.
    TooLong { max: usize },
}

/// The apply-side text rule the interchange documents share: no NUL (the
/// codec reads up to the first NUL, so the value would reload truncated), no
/// char above U+00FF when `single_byte` (the codec encodes one byte per
/// char), and the field's capacity — UTF-8 bytes for the `name` fields, chars
/// for the single-byte `short_name`/`shirt_name`.
pub(crate) fn text_misfit(text: &str, max: usize, single_byte: bool) -> Option<TextMisfit> {
    if text.contains('\0') {
        return Some(TextMisfit::Nul);
    }
    if single_byte && text.chars().any(|c| c > '\u{ff}') {
        return Some(TextMisfit::BeyondSingleByte);
    }
    let len = if single_byte {
        text.chars().count()
    } else {
        text.len()
    };
    if len > max {
        return Some(TextMisfit::TooLong { max });
    }
    None
}

/// The capacity of `text` in `texts`: the field's byte length — what the
/// codec accepts before `CodecError::Text`.
pub(crate) fn text_max<T: Copy + PartialEq>(texts: &[TextSpec<T>], text: T) -> usize {
    let len = texts
        .iter()
        .find(|spec| spec.text == text)
        .map(|spec| spec.len)
        .expect("every version stores the text");
    usize::try_from(len).expect("a text field's length fits usize")
}

/// The shared write loop over `runs`: get, width check, patch.
fn write_record<F: Copy + std::fmt::Debug, T>(
    schema: &RecordSchema<F, T>,
    record: &mut [u8],
    get: impl Fn(F) -> Result<u32, CodecError>,
) -> Result<(), CodecError> {
    if record.len() != schema.size {
        return Err(CodecError::RecordSize {
            expected: schema.size,
            got: record.len(),
        });
    }
    for (field, offset, width) in runs(schema) {
        write_run(record, offset, width, field, get(field)?)?;
    }
    Ok(())
}

/// Width-checks `value` against its run and patches it in.
fn write_run<F: std::fmt::Debug>(
    record: &mut [u8],
    offset: u32,
    width: u32,
    field: F,
    value: u32,
) -> Result<(), CodecError> {
    if width < 32 && value >= (1u32 << width) {
        return Err(CodecError::ValueTooWide {
            field: format!("{field:?}"),
            value,
            width,
        });
    }
    bits::write_bits(record, offset, width, value);
    Ok(())
}

#[cfg(test)]
mod tests;
