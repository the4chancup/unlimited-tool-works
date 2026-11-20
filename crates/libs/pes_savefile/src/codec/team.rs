//! The team, roster and tactics record codecs: the three record kinds the
//! `TeamEntry` merges.

use super::{CodecError, bits, runs, single_byte, text_bytes, write_record, write_run, write_text};
use crate::model::team::TeamEntry;
use crate::schema::fields::{RosterField, TacticsField, TeamField, TeamText};
use crate::schema::{RecordSchema, TacticsSchema};

/// Every bit run of a tactics record: the `fields`, the expanded `arrays`,
/// then for each of the three presets every `PresetSpec` byte, the eleven
/// man-marking bytes, every formation slot (position, y, x), and every
/// instruction part — each block only where the schema maps it.
pub(crate) fn tactics_runs(schema: &TacticsSchema) -> Vec<(TacticsField, u32, u32)> {
    use crate::schema::fields::{FormationPart, InstructionPart, InstructionSide};
    let mut out: Vec<(TacticsField, u32, u32)> = Vec::new();
    for spec in schema.fields {
        out.push((spec.field, spec.bit_offset, spec.bit_width));
    }
    for array in schema.arrays {
        for i in 0..array.count {
            out.push((
                (array.make)(i),
                array.base_bit + u32::from(i) * array.stride_bits,
                array.bit_width,
            ));
        }
    }
    for preset in 0..3u8 {
        let preset_base = u32::from(preset) * schema.preset_stride_bits;
        for spec in schema.presets {
            out.push((
                TacticsField::Preset {
                    preset,
                    field: spec.field,
                },
                preset_base + spec.bit_offset,
                8,
            ));
        }
        if let Some(man_marking_bit) = schema.man_marking_bit {
            for slot in 0..11u8 {
                out.push((
                    TacticsField::ManMarking { preset, slot },
                    preset_base + man_marking_bit + u32::from(slot) * 8,
                    8,
                ));
            }
        }
        if let Some(f) = &schema.formations {
            for formation in 0..3u8 {
                let base =
                    preset_base + f.base_bit + u32::from(formation) * f.formation_stride_bits;
                for slot in 0..11u8 {
                    for (part, offset) in [
                        (
                            FormationPart::Position,
                            base + u32::from(slot) * f.slot_stride_bits,
                        ),
                        (
                            FormationPart::Y,
                            base + f.y_offset_bits + u32::from(slot) * f.pair_stride_bits,
                        ),
                        (
                            FormationPart::X,
                            base + f.x_offset_bits + u32::from(slot) * f.pair_stride_bits,
                        ),
                    ] {
                        out.push((
                            TacticsField::Formation {
                                preset,
                                formation,
                                slot,
                                part,
                            },
                            offset,
                            8,
                        ));
                    }
                }
            }
        }
        if let Some(ins) = &schema.instructions {
            for (side, s) in [
                (InstructionSide::Attack, 0u32),
                (InstructionSide::Defence, 1u32),
            ] {
                for index in 0..2u8 {
                    for (part, p) in [
                        (InstructionPart::Instruction, 0u32),
                        (InstructionPart::PlayerId, 1u32),
                    ] {
                        out.push((
                            TacticsField::Instruction {
                                preset,
                                side,
                                index,
                                part,
                            },
                            preset_base
                                + ins.base_bit
                                + s * ins.side_stride_bits
                                + u32::from(index) * ins.index_stride_bits
                                + p * ins.part_stride_bits,
                            8,
                        ));
                    }
                }
            }
        }
    }
    out
}

/// Decodes one team record into a fresh `TeamEntry` (id, name, colours, edit
/// flags, kit slots; roster and tactics come from their own records).
pub fn read_team(
    record: &[u8],
    schema: &RecordSchema<TeamField, TeamText>,
) -> Result<TeamEntry, CodecError> {
    let mut team = TeamEntry::default();
    if record.len() != schema.size {
        return Err(CodecError::RecordSize {
            expected: schema.size,
            got: record.len(),
        });
    }
    for (field, offset, width) in runs(schema) {
        team.set(field, bits::read_bits(record, offset, width))?;
    }
    for spec in schema.texts {
        let bytes = text_bytes(record, spec.byte_offset, spec.len);
        match spec.text {
            TeamText::Name => {
                team.name = String::from_utf8(bytes.to_vec()).map_err(|_| CodecError::Text {
                    text: "Name".to_string(),
                    reason: "team name is not UTF-8",
                })?;
            }
            TeamText::ShortName => {
                team.short_name = bytes.iter().map(|&b| char::from(b)).collect();
            }
        }
    }
    Ok(team)
}

/// Patches the schema's fields and texts of `team` into `record`.
pub fn write_team(
    team: &TeamEntry,
    record: &mut [u8],
    schema: &RecordSchema<TeamField, TeamText>,
) -> Result<(), CodecError> {
    write_record(schema, record, |field| team.get(field))?;
    for spec in schema.texts {
        let bytes = match spec.text {
            TeamText::Name => team.name.as_bytes().to_vec(),
            TeamText::ShortName => single_byte(&team.short_name, "ShortName")?,
        };
        if bytes.len() > spec.len as usize {
            return Err(CodecError::Text {
                text: format!("{:?}", spec.text),
                reason: "the text does not fit its field",
            });
        }
        write_text(record, spec.byte_offset, spec.len, &bytes);
    }
    Ok(())
}

/// Applies a roster record onto an existing `TeamEntry` whose `id` must match
/// the record's team id.
pub fn read_roster_into(
    team: &mut TeamEntry,
    record: &[u8],
    schema: &RecordSchema<RosterField, TeamText>,
) -> Result<(), CodecError> {
    if record.len() != schema.size {
        return Err(CodecError::RecordSize {
            expected: schema.size,
            got: record.len(),
        });
    }
    for (field, offset, width) in runs(schema) {
        let value = bits::read_bits(record, offset, width);
        if field == RosterField::TeamId && value != team.id {
            return Err(CodecError::TeamIdMismatch {
                expected: team.id,
                got: value,
            });
        }
        team.roster_set(field, value)?;
    }
    Ok(())
}

/// Patches the schema's roster fields of `team` into `record`. A roster
/// longer than the schema's slot count is `RosterTooLong` — the extra slots
/// have no run and would be silently dropped.
pub fn write_roster(
    team: &TeamEntry,
    record: &mut [u8],
    schema: &RecordSchema<RosterField, TeamText>,
) -> Result<(), CodecError> {
    let slots = runs(schema)
        .filter_map(|(field, _, _)| match field {
            RosterField::Player(i) => Some(usize::from(i) + 1),
            RosterField::TeamId | RosterField::Number(_) => None,
        })
        .max()
        .unwrap_or(0);
    if team.roster.len() > slots {
        return Err(CodecError::RosterTooLong {
            slots,
            got: team.roster.len(),
        });
    }
    write_record(schema, record, |field| team.roster_get(field))
}

/// Applies a tactics record onto an existing `TeamEntry` whose `id` must match
/// the record's team id.
pub fn read_tactics_into(
    team: &mut TeamEntry,
    record: &[u8],
    schema: &TacticsSchema,
) -> Result<(), CodecError> {
    if record.len() != schema.size {
        return Err(CodecError::RecordSize {
            expected: schema.size,
            got: record.len(),
        });
    }
    for (field, offset, width) in tactics_runs(schema) {
        let value = bits::read_bits(record, offset, width);
        if field == TacticsField::TeamId && value != team.id {
            return Err(CodecError::TeamIdMismatch {
                expected: team.id,
                got: value,
            });
        }
        team.tactics_set(field, value)?;
    }
    Ok(())
}

/// Patches the schema's tactics fields of `team` into `record`.
pub fn write_tactics(
    team: &TeamEntry,
    record: &mut [u8],
    schema: &TacticsSchema,
) -> Result<(), CodecError> {
    if record.len() != schema.size {
        return Err(CodecError::RecordSize {
            expected: schema.size,
            got: record.len(),
        });
    }
    for (field, offset, width) in tactics_runs(schema) {
        write_run(record, offset, width, field, team.tactics_get(field)?)?;
    }
    Ok(())
}
