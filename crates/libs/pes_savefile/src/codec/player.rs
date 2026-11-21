//! The player record codec: `read_player`/`read_player_into`/`write_player`.

use pes_version::PesVersion;

use super::{
    CodecError, bits, runs, scalar_runs, single_byte, split_stored, text_bytes, write_record,
    write_text,
};
use crate::model::ingame_face::IngameFace;
use crate::model::player::PlayerEntry;
use crate::schema::fields::{PlayerField, PlayerText};
use crate::schema::{RecordSchema, stored_offset, stored_order};

/// Decodes one player record into a fresh `PlayerEntry`. `version` is the
/// save's own, whatever the schema names: it picks the stored offsets the
/// read adds (`stored_offset`).
pub fn read_player(
    version: PesVersion,
    record: &[u8],
    schema: &RecordSchema<PlayerField, PlayerText>,
) -> Result<PlayerEntry, CodecError> {
    let mut player = PlayerEntry::default();
    read_player_into(version, &mut player, record, schema)?;
    Ok(player)
}

/// Applies a record's fields onto an existing `PlayerEntry`: what the PES 15/16
/// appearance record is applied with. Only the fields and texts the schema
/// lists are touched.
pub fn read_player_into(
    version: PesVersion,
    player: &mut PlayerEntry,
    record: &[u8],
    schema: &RecordSchema<PlayerField, PlayerText>,
) -> Result<(), CodecError> {
    if record.len() != schema.size {
        return Err(CodecError::RecordSize {
            expected: schema.size,
            got: record.len(),
        });
    }
    for (field, offset, width) in scalar_runs(schema) {
        let stored = bits::read_bits(record, offset, width);
        player.set(field, model_value(version, field, stored))?;
    }
    for spec in schema.split {
        let stored = split_stored(record, spec);
        player.set(spec.field, model_value(version, spec.field, stored))?;
    }
    for spec in schema.texts {
        let bytes = text_bytes(record, spec.byte_offset, spec.len);
        match spec.text {
            PlayerText::Name => {
                player.name = String::from_utf8(bytes.to_vec()).map_err(|_| CodecError::Text {
                    text: "Name".to_string(),
                    reason: "player name is not UTF-8",
                })?;
            }
            PlayerText::ShirtName => {
                // Single-byte text: byte b is the char U+00bB.
                player.shirt_name = bytes.iter().map(|&b| char::from(b)).collect();
            }
        }
    }
    if let Some(run) = &schema.ingame_face {
        let at = run.byte_offset as usize;
        player.appearance.ingame_face =
            IngameFace::from_bytes(record[at..at + run.len as usize].to_vec());
    }
    Ok(())
}

/// Patches the schema's fields and texts of `player` into `record`; every byte
/// outside those runs is left as it was.
pub fn write_player(
    version: PesVersion,
    player: &PlayerEntry,
    record: &mut [u8],
    schema: &RecordSchema<PlayerField, PlayerText>,
) -> Result<(), CodecError> {
    write_record(schema, record, |field| {
        let value = player.get(field)?;
        if let Some(order) = stored_order(version, field) {
            return order
                .iter()
                .position(|&model| model == value)
                .map(|stored| stored as u32)
                .ok_or(CodecError::OutsideStoredOrder {
                    field: format!("{field:?}"),
                    value,
                });
        }
        let offset = stored_offset(version, field);
        value
            .checked_sub(offset)
            .ok_or(CodecError::BelowStoredOffset {
                field: format!("{field:?}"),
                value,
                offset,
            })
    })?;
    for spec in schema.texts {
        let bytes = match spec.text {
            PlayerText::Name => player.name.as_bytes().to_vec(),
            PlayerText::ShirtName => single_byte(&player.shirt_name, "ShirtName")?,
        };
        if bytes.len() > spec.len as usize {
            return Err(CodecError::Text {
                text: format!("{:?}", spec.text),
                reason: "the text does not fit its field",
            });
        }
        write_text(record, spec.byte_offset, spec.len, &bytes);
    }
    if let Some(run) = &schema.ingame_face {
        let bytes = player.appearance.ingame_face.bytes();
        if bytes.len() != run.len as usize {
            return Err(CodecError::RunSize {
                expected: run.len as usize,
                got: bytes.len(),
            });
        }
        let at = run.byte_offset as usize;
        record[at..at + run.len as usize].copy_from_slice(bytes);
    }
    Ok(())
}

/// Sets the flags the game sets on a PES 2014 player record it changed:
/// `EditedPlayer` when `record` differs from the record as loaded in any
/// byte, `EditedName` when the Name text's bytes differ, and
/// `EditedBasicSettings` when the nationality, age or stronger-foot run
/// differs. Every run is found through `schema`; bits are set, never
/// cleared.
pub(crate) fn mark_edit_flags(
    loaded: &[u8],
    record: &mut [u8],
    schema: &RecordSchema<PlayerField, PlayerText>,
) {
    if loaded == record {
        return;
    }
    set_flag(record, schema, PlayerField::EditedPlayer);
    let name = schema
        .texts
        .iter()
        .find(|spec| spec.text == PlayerText::Name)
        .expect("the player record maps the Name text");
    let (start, len) = (name.byte_offset as usize, name.len as usize);
    if loaded[start..start + len] != record[start..start + len] {
        set_flag(record, schema, PlayerField::EditedName);
    }
    for wanted in [
        PlayerField::Nationality,
        PlayerField::Age,
        PlayerField::StrongerFoot,
    ] {
        let (_, offset, width) = runs(schema)
            .find(|(field, _, _)| *field == wanted)
            .expect("the player record maps the basic settings");
        if bits::read_bits(loaded, offset, width) != bits::read_bits(record, offset, width) {
            set_flag(record, schema, PlayerField::EditedBasicSettings);
            break;
        }
    }
}

/// The model value `stored` decodes to for `field`: the stored order's
/// table where the version stores one (PES 2014's Sleeves and shirttail),
/// else `stored` plus the version's offset (PES 2014's stats less 40 and
/// friends; 0 everywhere else). The run's width bounds the stored value
/// inside the table.
fn model_value(version: PesVersion, field: PlayerField, stored: u32) -> u32 {
    match stored_order(version, field) {
        Some(order) => order[stored as usize],
        None => stored + stored_offset(version, field),
    }
}

/// Sets `field`'s bit in `record`, its run found through `schema`.
fn set_flag(record: &mut [u8], schema: &RecordSchema<PlayerField, PlayerText>, flag: PlayerField) {
    let (_, offset, width) = runs(schema)
        .find(|(field, _, _)| *field == flag)
        .expect("the player record maps the flag");
    bits::write_bits(record, offset, width, 1);
}
