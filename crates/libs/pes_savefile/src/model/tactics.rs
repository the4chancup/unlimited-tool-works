//! `TeamTactics`: the tactics record's contents, version-independent. The
//! record carries three presets of style flags, sliders, three formations each
//! and (PES 17+) advanced instructions, plus the shared starting eleven, bench
//! order, set-piece takers and auto flags.

use pes_version::PesVersion;

use crate::codec::{self, CodecError};
use crate::model::team::TeamEntry;
use crate::schema::fields::{
    FormationPart, InstructionPart, InstructionSide, PresetField, TacticsField,
};
use crate::schema::schema_for;

/// A tactics record's contents: the three presets and the team-wide settings.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TeamTactics {
    /// The three presets the game cycles through.
    pub presets: [TacticsPreset; 3],
    /// Starting eleven as roster slots.
    pub starting_eleven: [u8; 11],
    /// Bench order as roster slots.
    pub bench_order: [u8; 21],
    /// The set-piece takers (roster slots, 0xFF none).
    pub set_pieces: SetPieceTakers,
    /// Players joining the attack (roster slots).
    pub players_to_join_attack: [u8; 3],
    /// The automatic-settings flags.
    pub auto: TeamAutoFlags,
}

/// One preset: a style block, the sliders, three formations, the advanced
/// instructions and the man-marking assignments.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TacticsPreset {
    /// The eight style switches.
    pub style: TacticStyle,
    /// The five sliders.
    pub sliders: TacticSliders,
    /// Kick-off, in-possession and out-of-possession formations.
    pub formations: [Formation; 3],
    /// The two attacking instructions (PES 17+).
    pub attack_instructions: Option<[AdvancedInstruction; 2]>,
    /// The two defending instructions (PES 17+).
    pub defence_instructions: Option<[AdvancedInstruction; 2]>,
    /// Man marking: own formation slot `i`'s assignment is the marked
    /// opponent's formation slot, 0xFF none (PES 15+).
    pub man_marking: [u8; 11],
}

/// The style switches; each is one byte on disk carrying 0 or 1.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TacticStyle {
    /// 0 counter attack, 1 possession.
    pub attacking_style: bool,
    /// 0 centre, 1 wide.
    pub attacking_zone: bool,
    /// 0 long pass, 1 short pass.
    pub buildup: bool,
    /// 0 maintain, 1 flexible.
    pub positioning: bool,
    /// 0 frontline pressure, 1 all-out defence.
    pub defensive_style: bool,
    /// 0 middle, 1 wide.
    pub containment_area: bool,
    /// 0 aggressive, 1 conservative.
    pub pressure: bool,
    /// Fluid formation on/off (PES 16+).
    pub fluid: Option<bool>,
}

/// The five sliders.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TacticSliders {
    /// Support range, 1 to 10.
    pub support_range: u8,
    /// Defensive line, 1 to 10.
    pub defensive_line: u8,
    /// Compactness, 1 to 10.
    pub compactness: u8,
    /// Numbers in attack, 1 few to 3 many.
    pub numbers_in_attack: u8,
    /// Numbers in defence, 1 few to 3 many.
    pub numbers_in_defence: u8,
}

/// One formation: eleven slot layouts.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Formation {
    /// The eleven players' position and coordinates.
    pub players: [FormationSlot; 11],
}

/// One formation slot.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FormationSlot {
    /// Position, GK 0 to CF 12.
    pub position: u8,
    /// Horizontal coordinate.
    pub x: u8,
    /// Vertical coordinate.
    pub y: u8,
}

/// One advanced instruction (PES 17+).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AdvancedInstruction {
    /// The instruction id (version-specific meaning).
    pub instruction: u8,
    /// The targeted player, for instructions that take one (roster slot).
    pub player: u8,
}

/// The set-piece takers, roster slots with 0xFF for none.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SetPieceTakers {
    /// Long free kick taker.
    pub free_kick_long: u8,
    /// Short free kick taker.
    pub free_kick_short: u8,
    /// Second free kick taker.
    pub free_kick_second: u8,
    /// Left corner taker.
    pub corner_left: u8,
    /// Right corner taker.
    pub corner_right: u8,
    /// Penalty taker.
    pub penalty: u8,
    /// Captain.
    pub captain: u8,
}

impl TeamTactics {
    /// A record of `version` with every field the version stores at zero
    /// and the fields it lacks absent (the gated `Option` fields `None`):
    /// what a document is applied onto when no save record exists, so a
    /// key the document omits reads 0 like any other field, never as a
    /// field the version lacks. Decoded from a zeroed record of the
    /// version's tactics layout, so the schema alone decides what is
    /// present. The fields that name a roster slot (set-piece takers, man
    /// marking, starting eleven, bench order, players to join the attack)
    /// are 0xFF instead, the record's none: at 0 they would all name roster
    /// slot 0.
    pub fn bare(version: PesVersion) -> TeamTactics {
        let schema = schema_for(version).tactic;
        let record = vec![0; schema.size];
        let mut team = TeamEntry::default();
        codec::read_tactics_into(&mut team, &record, schema)
            .expect("every field of the version's tactics record accepts 0");
        let mut tactics = team.tactics;
        tactics.set_pieces = SetPieceTakers {
            free_kick_long: 0xFF,
            free_kick_short: 0xFF,
            free_kick_second: 0xFF,
            corner_left: 0xFF,
            corner_right: 0xFF,
            penalty: 0xFF,
            captain: 0xFF,
        };
        for preset in &mut tactics.presets {
            preset.man_marking = [0xFF; 11];
        }
        tactics.starting_eleven = [0xFF; 11];
        tactics.bench_order = [0xFF; 21];
        tactics.players_to_join_attack = [0xFF; 3];
        tactics
    }
}

impl TacticsPreset {
    /// The formations the game fields under this preset, as indices into
    /// `formations`: all three while the fluid formation is on, else the
    /// kick-off one alone (the game switches to the other two only when
    /// fluid). The one rule the AATF check's fielded positions, the
    /// Tactics pitch's marks and their row's jump all read.
    pub fn fielded_formations(&self) -> &'static [usize] {
        match self.style.fluid {
            Some(true) => &[0, 1, 2],
            Some(false) | None => &[0],
        }
    }
}

/// The automatic-settings flags; each is one byte on disk carrying 0 or 1
/// except `substitution`, a setting byte.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TeamAutoFlags {
    /// Auto substitution setting (PES 15+).
    pub substitution: Option<u8>,
    /// Auto offside trap (PES 15+).
    pub offside_trap: Option<bool>,
    /// Auto preset change (PES 15+).
    pub preset_change: Option<bool>,
    /// Auto attack/defence level change (PES 17+).
    pub attack_defence_levels: Option<bool>,
}

impl TeamEntry {
    /// The model value a tactics bit run carries.
    pub(crate) fn tactics_get(&self, field: TacticsField) -> Result<u32, CodecError> {
        let v = match field {
            TacticsField::TeamId => self.id,
            TacticsField::Preset { preset, field: pf } => {
                let p = self
                    .tactics
                    .presets
                    .get(usize::from(preset))
                    .ok_or_else(|| codec::index(field, preset))?;
                match pf {
                    PresetField::AttackingStyle => u32::from(p.style.attacking_style),
                    PresetField::Buildup => u32::from(p.style.buildup),
                    PresetField::AttackingZone => u32::from(p.style.attacking_zone),
                    PresetField::Positioning => u32::from(p.style.positioning),
                    PresetField::DefensiveStyle => u32::from(p.style.defensive_style),
                    PresetField::ContainmentArea => u32::from(p.style.containment_area),
                    PresetField::Pressure => u32::from(p.style.pressure),
                    PresetField::FluidFormation => {
                        u32::from(p.style.fluid.ok_or_else(|| codec::missing(field))?)
                    }
                    PresetField::SupportRange => u32::from(p.sliders.support_range),
                    PresetField::DefensiveLine => u32::from(p.sliders.defensive_line),
                    PresetField::Compactness => u32::from(p.sliders.compactness),
                    PresetField::NumbersInAttack => u32::from(p.sliders.numbers_in_attack),
                    PresetField::NumbersInDefence => u32::from(p.sliders.numbers_in_defence),
                }
            }
            TacticsField::Formation {
                preset,
                formation,
                slot,
                part,
            } => {
                let s = self
                    .tactics
                    .presets
                    .get(usize::from(preset))
                    .and_then(|p| p.formations.get(usize::from(formation)))
                    .and_then(|f| f.players.get(usize::from(slot)))
                    .ok_or_else(|| codec::index(field, slot))?;
                match part {
                    FormationPart::Position => u32::from(s.position),
                    FormationPart::X => u32::from(s.x),
                    FormationPart::Y => u32::from(s.y),
                }
            }
            TacticsField::Instruction {
                preset,
                side,
                index,
                part,
            } => {
                let p = self
                    .tactics
                    .presets
                    .get(usize::from(preset))
                    .ok_or_else(|| codec::index(field, preset))?;
                let instructions = match side {
                    InstructionSide::Attack => &p.attack_instructions,
                    InstructionSide::Defence => &p.defence_instructions,
                };
                let instruction = instructions
                    .as_ref()
                    .ok_or_else(|| codec::missing(field))?
                    .get(usize::from(index))
                    .ok_or_else(|| codec::index(field, index))?;
                match part {
                    InstructionPart::Instruction => u32::from(instruction.instruction),
                    InstructionPart::PlayerId => u32::from(instruction.player),
                }
            }
            TacticsField::ManMarking { preset, slot } => u32::from(
                *self
                    .tactics
                    .presets
                    .get(usize::from(preset))
                    .ok_or_else(|| codec::index(field, preset))?
                    .man_marking
                    .get(usize::from(slot))
                    .ok_or_else(|| codec::index(field, slot))?,
            ),
            TacticsField::FreeKickTakerLong => u32::from(self.tactics.set_pieces.free_kick_long),
            TacticsField::FreeKickTakerShort => u32::from(self.tactics.set_pieces.free_kick_short),
            TacticsField::FreeKickTakerSecond => {
                u32::from(self.tactics.set_pieces.free_kick_second)
            }
            TacticsField::CornerTakerLeft => u32::from(self.tactics.set_pieces.corner_left),
            TacticsField::CornerTakerRight => u32::from(self.tactics.set_pieces.corner_right),
            TacticsField::PenaltyTaker => u32::from(self.tactics.set_pieces.penalty),
            TacticsField::Captain => u32::from(self.tactics.set_pieces.captain),
            TacticsField::AutoSubstitution => u32::from(
                self.tactics
                    .auto
                    .substitution
                    .ok_or_else(|| codec::missing(field))?,
            ),
            TacticsField::AutoOffsideTrap => u32::from(
                self.tactics
                    .auto
                    .offside_trap
                    .ok_or_else(|| codec::missing(field))?,
            ),
            TacticsField::AutoPresetChange => u32::from(
                self.tactics
                    .auto
                    .preset_change
                    .ok_or_else(|| codec::missing(field))?,
            ),
            TacticsField::AutoAttackDefenceLevels => u32::from(
                self.tactics
                    .auto
                    .attack_defence_levels
                    .ok_or_else(|| codec::missing(field))?,
            ),
            TacticsField::Starting(i) => u32::from(
                *self
                    .tactics
                    .starting_eleven
                    .get(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))?,
            ),
            TacticsField::Bench(i) => u32::from(
                *self
                    .tactics
                    .bench_order
                    .get(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))?,
            ),
            TacticsField::PlayerToJoinAttack(i) => u32::from(
                *self
                    .tactics
                    .players_to_join_attack
                    .get(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))?,
            ),
        };
        Ok(v)
    }

    /// Patches a tactics bit-run value into the model.
    pub(crate) fn tactics_set(
        &mut self,
        field: TacticsField,
        value: u32,
    ) -> Result<(), CodecError> {
        match field {
            TacticsField::TeamId => self.id = value,
            TacticsField::Preset { preset, field: pf } => {
                let p = self
                    .tactics
                    .presets
                    .get_mut(usize::from(preset))
                    .ok_or_else(|| codec::index(field, preset))?;
                match pf {
                    PresetField::AttackingStyle => {
                        p.style.attacking_style = codec::boolean(field, value)?;
                    }
                    PresetField::Buildup => p.style.buildup = codec::boolean(field, value)?,
                    PresetField::AttackingZone => {
                        p.style.attacking_zone = codec::boolean(field, value)?;
                    }
                    PresetField::Positioning => {
                        p.style.positioning = codec::boolean(field, value)?;
                    }
                    PresetField::DefensiveStyle => {
                        p.style.defensive_style = codec::boolean(field, value)?;
                    }
                    PresetField::ContainmentArea => {
                        p.style.containment_area = codec::boolean(field, value)?;
                    }
                    PresetField::Pressure => p.style.pressure = codec::boolean(field, value)?,
                    PresetField::FluidFormation => {
                        p.style.fluid = Some(codec::boolean(field, value)?);
                    }
                    PresetField::SupportRange => p.sliders.support_range = value as u8,
                    PresetField::DefensiveLine => p.sliders.defensive_line = value as u8,
                    PresetField::Compactness => p.sliders.compactness = value as u8,
                    PresetField::NumbersInAttack => p.sliders.numbers_in_attack = value as u8,
                    PresetField::NumbersInDefence => p.sliders.numbers_in_defence = value as u8,
                }
            }
            TacticsField::Formation {
                preset,
                formation,
                slot,
                part,
            } => {
                let s = self
                    .tactics
                    .presets
                    .get_mut(usize::from(preset))
                    .and_then(|p| p.formations.get_mut(usize::from(formation)))
                    .and_then(|f| f.players.get_mut(usize::from(slot)))
                    .ok_or_else(|| codec::index(field, slot))?;
                match part {
                    FormationPart::Position => s.position = value as u8,
                    FormationPart::X => s.x = value as u8,
                    FormationPart::Y => s.y = value as u8,
                }
            }
            TacticsField::Instruction {
                preset,
                side,
                index,
                part,
            } => {
                let p = self
                    .tactics
                    .presets
                    .get_mut(usize::from(preset))
                    .ok_or_else(|| codec::index(field, preset))?;
                let instructions = match side {
                    InstructionSide::Attack => &mut p.attack_instructions,
                    InstructionSide::Defence => &mut p.defence_instructions,
                };
                let instruction = instructions
                    .get_or_insert_default()
                    .get_mut(usize::from(index))
                    .ok_or_else(|| codec::index(field, index))?;
                match part {
                    InstructionPart::Instruction => instruction.instruction = value as u8,
                    InstructionPart::PlayerId => instruction.player = value as u8,
                }
            }
            TacticsField::ManMarking { preset, slot } => {
                *self
                    .tactics
                    .presets
                    .get_mut(usize::from(preset))
                    .ok_or_else(|| codec::index(field, preset))?
                    .man_marking
                    .get_mut(usize::from(slot))
                    .ok_or_else(|| codec::index(field, slot))? = value as u8;
            }
            TacticsField::FreeKickTakerLong => {
                self.tactics.set_pieces.free_kick_long = value as u8;
            }
            TacticsField::FreeKickTakerShort => {
                self.tactics.set_pieces.free_kick_short = value as u8;
            }
            TacticsField::FreeKickTakerSecond => {
                self.tactics.set_pieces.free_kick_second = value as u8;
            }
            TacticsField::CornerTakerLeft => self.tactics.set_pieces.corner_left = value as u8,
            TacticsField::CornerTakerRight => {
                self.tactics.set_pieces.corner_right = value as u8;
            }
            TacticsField::PenaltyTaker => self.tactics.set_pieces.penalty = value as u8,
            TacticsField::Captain => self.tactics.set_pieces.captain = value as u8,
            TacticsField::AutoSubstitution => {
                self.tactics.auto.substitution = Some(value as u8);
            }
            TacticsField::AutoOffsideTrap => {
                self.tactics.auto.offside_trap = Some(codec::boolean(field, value)?);
            }
            TacticsField::AutoPresetChange => {
                self.tactics.auto.preset_change = Some(codec::boolean(field, value)?);
            }
            TacticsField::AutoAttackDefenceLevels => {
                self.tactics.auto.attack_defence_levels = Some(codec::boolean(field, value)?);
            }
            TacticsField::Starting(i) => {
                *self
                    .tactics
                    .starting_eleven
                    .get_mut(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))? = value as u8;
            }
            TacticsField::Bench(i) => {
                *self
                    .tactics
                    .bench_order
                    .get_mut(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))? = value as u8;
            }
            TacticsField::PlayerToJoinAttack(i) => {
                *self
                    .tactics
                    .players_to_join_attack
                    .get_mut(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))? = value as u8;
            }
        }
        Ok(())
    }
}
