//! `team.toml`'s `[tactics]` half: emit, parse and apply for the squad order,
//! set pieces, auto flags and the three presets' styles, sliders, formations
//! and instructions.

use pes_version::PesVersion;
use toml_edit::Item;

use crate::interchange::team_toml::{
    AutoSection, ImportNote, InstructionEntry, InstructionsSection, PresetSection,
    SetPiecesSection, SlidersSection, StyleSection, TacticsSection, TeamTomlError,
};
use crate::model::instruction::Instruction;
use crate::model::tactics::{AdvancedInstruction, FormationSlot, bench_fill};
use crate::model::team::TeamEntry;
use crate::schema::TacticsSchema;
use crate::schema::fields::{PresetField, TacticsField};
use crate::schema::instruction;

use super::items::{
    as_table, boolean, emit, gated, int_array, label, label_position, opt, ranged, reject,
    required, u8_val,
};
use super::labels;

pub(super) fn emit_tactics(
    out: &mut String,
    tactics: &TacticsSection,
) -> Result<(), TeamTomlError> {
    out.push_str("\n[tactics]\n");
    emit(
        out,
        "starting_eleven",
        tactics.starting_eleven.map(array_text),
        "[]",
        "roster slots, counting from 0",
    );
    emit(
        out,
        "bench_order",
        tactics.bench_order.map(array_text),
        "[]",
        "the bench: 12 roster slots, counting from 0",
    );
    emit(
        out,
        "players_to_join_attack",
        tactics.players_to_join_attack.map(array_text),
        "[]",
        "roster slots, counting from 0; 255 = nobody",
    );
    let sp = &tactics.set_pieces;
    out.push_str("\n[tactics.set_pieces]\n");
    emit(
        out,
        "free_kick_long",
        sp.free_kick_long.map(|v| v.to_string()),
        "255",
        "roster slots, counting from 0; 255 = nobody",
    );
    emit(
        out,
        "free_kick_short",
        sp.free_kick_short.map(|v| v.to_string()),
        "255",
        "roster slots, counting from 0; 255 = nobody",
    );
    emit(
        out,
        "free_kick_second",
        sp.free_kick_second.map(|v| v.to_string()),
        "255",
        "roster slots, counting from 0; 255 = nobody",
    );
    emit(
        out,
        "corner_left",
        sp.corner_left.map(|v| v.to_string()),
        "255",
        "roster slots, counting from 0; 255 = nobody",
    );
    emit(
        out,
        "corner_right",
        sp.corner_right.map(|v| v.to_string()),
        "255",
        "roster slots, counting from 0; 255 = nobody",
    );
    emit(
        out,
        "penalty",
        sp.penalty.map(|v| v.to_string()),
        "255",
        "roster slots, counting from 0; 255 = nobody",
    );
    emit(
        out,
        "captain",
        sp.captain.map(|v| v.to_string()),
        "255",
        "roster slots, counting from 0; 255 = nobody",
    );
    let auto = &tactics.auto;
    out.push_str("\n[tactics.auto]\n");
    emit(
        out,
        "substitution",
        auto.substitution.map(|v| v.to_string()),
        "255",
        "as the game stores it",
    );
    emit(
        out,
        "offside_trap",
        auto.offside_trap.map(|v| v.to_string()),
        "false",
        "",
    );
    emit(
        out,
        "preset_change",
        auto.preset_change.map(|v| v.to_string()),
        "false",
        "",
    );
    emit(
        out,
        "attack_defence_levels",
        auto.attack_defence_levels.map(|v| v.to_string()),
        "false",
        "PES 17 and later",
    );
    for (i, preset) in tactics.presets.iter().enumerate() {
        let n = i + 1;
        out.push_str(&format!("\n[tactics.preset_{n}]\n"));
        emit(
            out,
            "man_marking",
            preset.man_marking.map(array_text),
            "[255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255]",
            "for each formation slot, the opponent's formation slot he marks; 255 = nobody",
        );
        out.push_str(&format!("\n[tactics.preset_{n}.style]\n"));
        let style = &preset.style;
        let switches: [Option<bool>; 7] = [
            style.attacking_style,
            style.attacking_zone,
            style.buildup,
            style.positioning,
            style.defensive_style,
            style.containment_area,
            style.pressure,
        ];
        for ((key, pair), value) in labels::STYLE_SWITCHES.iter().zip(switches) {
            let text = value.map(|on| format!("\"{}\"", pair[usize::from(on)]));
            emit(
                out,
                key,
                text,
                &format!("\"{}\"", pair[0]),
                &format!("\"{}\" | \"{}\"", pair[0], pair[1]),
            );
        }
        emit(
            out,
            "fluid",
            style.fluid.map(|v| v.to_string()),
            "false",
            "PES 16 and later",
        );
        out.push_str(&format!("\n[tactics.preset_{n}.sliders]\n"));
        let sliders = &preset.sliders;
        emit(
            out,
            "support_range",
            sliders.support_range.map(|v| v.to_string()),
            "1",
            "1 to 10",
        );
        emit(
            out,
            "defensive_line",
            sliders.defensive_line.map(|v| v.to_string()),
            "1",
            "1 to 10",
        );
        emit(
            out,
            "compactness",
            sliders.compactness.map(|v| v.to_string()),
            "1",
            "1 to 10",
        );
        emit(
            out,
            "numbers_in_attack",
            sliders.numbers_in_attack.map(|v| v.to_string()),
            "1",
            "1 few, 2 medium, 3 many",
        );
        emit(
            out,
            "numbers_in_defence",
            sliders.numbers_in_defence.map(|v| v.to_string()),
            "1",
            "1 few, 2 medium, 3 many",
        );
        for (f, formation) in preset.formations.iter().enumerate() {
            let Some(players) = formation else { continue };
            if n == 1 && f == 0 {
                out.push_str(
                    "\n# players: in the order of starting_eleven (its first entry is the first \
                     player here); roster slots count from 0, so slot 0 is [players.01]\n",
                );
            } else {
                out.push('\n');
            }
            out.push_str(&format!("[tactics.preset_{n}.formation_{}]\n", f + 1));
            out.push_str("players = [\n");
            for slot in players.iter() {
                let entry = slot_text(
                    slot,
                    &format!("tactics.preset_{n}.formation_{}.players", f + 1),
                )?;
                out.push_str(&format!("    {entry},\n"));
            }
            out.push_str("]\n");
        }
        if let Some(instructions) = &preset.instructions {
            out.push_str(&format!("\n[tactics.preset_{n}.instructions]\n"));
            let attack: Vec<String> = instructions.attack.iter().map(instruction_text).collect();
            let defence: Vec<String> = instructions.defence.iter().map(instruction_text).collect();
            out.push_str(&format!("attack = [{}]\n", attack.join(", ")));
            out.push_str(&format!("defence = [{}]\n", defence.join(", ")));
        }
    }
    Ok(())
}

fn array_text<const N: usize>(a: [u8; N]) -> String {
    let items: Vec<String> = a.iter().map(|v| v.to_string()).collect();
    format!("[{}]", items.join(", "))
}

fn slot_text(slot: &FormationSlot, key: &str) -> Result<String, TeamTomlError> {
    let position = labels::POSITIONS
        .get(usize::from(slot.position))
        .ok_or_else(|| TeamTomlError::OutOfRange {
            key: key.to_string(),
            value: i64::from(slot.position),
            range: "0 to 12".to_string(),
        })?;
    Ok(format!(
        "{{ position = \"{position}\", x = {}, y = {} }}",
        slot.x, slot.y
    ))
}

fn instruction_text(entry: &InstructionEntry) -> String {
    let index = Instruction::ALL
        .iter()
        .position(|i| *i == entry.instruction)
        .expect("a canonical instruction is in ALL");
    format!(
        "{{ instruction = \"{}\", player = {} }}",
        labels::INSTRUCTIONS[index],
        entry.player
    )
}

pub(super) fn parse_tactics(item: &Item) -> Result<TacticsSection, TeamTomlError> {
    let table = as_table(item, "tactics")?;
    let mut tactics = TacticsSection {
        starting_eleven: opt(table, "tactics", "starting_eleven", int_array)?,
        bench_order: opt(table, "tactics", "bench_order", bench_order)?,
        players_to_join_attack: opt(table, "tactics", "players_to_join_attack", int_array)?,
        ..TacticsSection::default()
    };
    if let Some(item) = table.get("set_pieces") {
        let t = as_table(item, "tactics.set_pieces")?;
        tactics.set_pieces = SetPiecesSection {
            free_kick_long: opt(t, "tactics.set_pieces", "free_kick_long", u8_val)?,
            free_kick_short: opt(t, "tactics.set_pieces", "free_kick_short", u8_val)?,
            free_kick_second: opt(t, "tactics.set_pieces", "free_kick_second", u8_val)?,
            corner_left: opt(t, "tactics.set_pieces", "corner_left", u8_val)?,
            corner_right: opt(t, "tactics.set_pieces", "corner_right", u8_val)?,
            penalty: opt(t, "tactics.set_pieces", "penalty", u8_val)?,
            captain: opt(t, "tactics.set_pieces", "captain", u8_val)?,
        };
        reject(
            t,
            "tactics.set_pieces",
            &[
                "free_kick_long",
                "free_kick_short",
                "free_kick_second",
                "corner_left",
                "corner_right",
                "penalty",
                "captain",
            ],
        )?;
    }
    if let Some(item) = table.get("auto") {
        let t = as_table(item, "tactics.auto")?;
        tactics.auto = AutoSection {
            substitution: opt(t, "tactics.auto", "substitution", u8_val)?,
            offside_trap: opt(t, "tactics.auto", "offside_trap", boolean)?,
            preset_change: opt(t, "tactics.auto", "preset_change", boolean)?,
            attack_defence_levels: opt(t, "tactics.auto", "attack_defence_levels", boolean)?,
        };
        reject(
            t,
            "tactics.auto",
            &[
                "substitution",
                "offside_trap",
                "preset_change",
                "attack_defence_levels",
            ],
        )?;
    }
    for (i, preset) in tactics.presets.iter_mut().enumerate() {
        let key = format!("preset_{}", i + 1);
        let Some(item) = table.get(&key) else {
            continue;
        };
        *preset = parse_preset(item, &format!("tactics.preset_{}", i + 1))?;
    }
    reject(
        table,
        "tactics",
        &[
            "starting_eleven",
            "bench_order",
            "players_to_join_attack",
            "set_pieces",
            "auto",
            "preset_1",
            "preset_2",
            "preset_3",
        ],
    )?;
    Ok(tactics)
}

fn parse_preset(item: &Item, path: &str) -> Result<PresetSection, TeamTomlError> {
    let table = as_table(item, path)?;
    let mut preset = PresetSection {
        man_marking: opt(table, path, "man_marking", man_marking)?,
        ..PresetSection::default()
    };
    if let Some(item) = table.get("style") {
        let t = as_table(item, &format!("{path}.style"))?;
        let style_path = format!("{path}.style");
        let mut switches: [Option<bool>; 7] = [None; 7];
        for ((key, pair), slot) in labels::STYLE_SWITCHES.iter().zip(switches.iter_mut()) {
            *slot = opt(t, &style_path, key, |i, k| label(i, k, pair))?;
        }
        preset.style = StyleSection {
            attacking_style: switches[0],
            attacking_zone: switches[1],
            buildup: switches[2],
            positioning: switches[3],
            defensive_style: switches[4],
            containment_area: switches[5],
            pressure: switches[6],
            fluid: opt(t, &style_path, "fluid", boolean)?,
        };
        reject(
            t,
            &style_path,
            &[
                "attacking_style",
                "attacking_zone",
                "buildup",
                "positioning",
                "defensive_style",
                "containment_area",
                "pressure",
                "fluid",
            ],
        )?;
    }
    if let Some(item) = table.get("sliders") {
        let t = as_table(item, &format!("{path}.sliders"))?;
        let sliders_path = format!("{path}.sliders");
        preset.sliders = SlidersSection {
            support_range: opt(t, &sliders_path, "support_range", |i, k| {
                ranged(i, k, 1, 10)
            })?,
            defensive_line: opt(t, &sliders_path, "defensive_line", |i, k| {
                ranged(i, k, 1, 10)
            })?,
            compactness: opt(t, &sliders_path, "compactness", |i, k| ranged(i, k, 1, 10))?,
            numbers_in_attack: opt(t, &sliders_path, "numbers_in_attack", |i, k| {
                ranged(i, k, 1, 3)
            })?,
            numbers_in_defence: opt(t, &sliders_path, "numbers_in_defence", |i, k| {
                ranged(i, k, 1, 3)
            })?,
        };
        reject(
            t,
            &sliders_path,
            &[
                "support_range",
                "defensive_line",
                "compactness",
                "numbers_in_attack",
                "numbers_in_defence",
            ],
        )?;
    }
    for (f, formation) in preset.formations.iter_mut().enumerate() {
        let key = format!("formation_{}", f + 1);
        let Some(item) = table.get(&key) else {
            continue;
        };
        let t = as_table(item, &format!("{path}.{key}"))?;
        *formation = Some(required(
            t,
            &format!("{path}.{key}"),
            "players",
            formation_players,
        )?);
        reject(t, &format!("{path}.{key}"), &["players"])?;
    }
    if let Some(item) = table.get("instructions") {
        let t = as_table(item, &format!("{path}.instructions"))?;
        let instructions_path = format!("{path}.instructions");
        preset.instructions = Some(InstructionsSection {
            attack: required(t, &instructions_path, "attack", instruction_entries)?,
            defence: required(t, &instructions_path, "defence", instruction_entries)?,
        });
        reject(t, &instructions_path, &["attack", "defence"])?;
    }
    reject(
        table,
        path,
        &[
            "man_marking",
            "style",
            "sliders",
            "formation_1",
            "formation_2",
            "formation_3",
            "instructions",
        ],
    )?;
    Ok(preset)
}

/// `man_marking`'s eleven integers: an own formation slot's assignment is the
/// marked opponent's formation slot (0 to 10) or 255 for none.
pub(super) fn man_marking(item: &Item, key: &str) -> Result<[u8; 11], TeamTomlError> {
    let marking = int_array::<11>(item, key)?;
    for &value in marking.iter() {
        if value > 10 && value != 255 {
            return Err(TeamTomlError::OutOfRange {
                key: key.to_string(),
                value: i64::from(value),
                range: "0 to 10 or 255".to_string(),
            });
        }
    }
    Ok(marking)
}

/// `bench_order`'s twelve integers (the bench: the record's first
/// twelve bench entries of twenty-one).
pub(super) fn bench_order(item: &Item, key: &str) -> Result<[u8; 12], TeamTomlError> {
    int_array::<12>(item, key).map_err(|error| match error {
        TeamTomlError::WrongType { key, .. } => TeamTomlError::WrongType {
            key,
            expected: "an array of twelve integers",
        },
        error => error,
    })
}

pub(super) fn formation_players(
    item: &Item,
    key: &str,
) -> Result<[FormationSlot; 11], TeamTomlError> {
    let expected = "an array of eleven { position, x, y } tables";
    let array =
        item.as_value()
            .and_then(|v| v.as_array())
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected,
            })?;
    if array.len() != 11 {
        return Err(TeamTomlError::WrongType {
            key: key.to_string(),
            expected,
        });
    }
    let mut players = [FormationSlot::default(); 11];
    for (i, (slot, value)) in players.iter_mut().zip(array.iter()).enumerate() {
        let entry = value
            .as_inline_table()
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected,
            })?;
        reject(entry, &format!("{key}.{i}"), &["position", "x", "y"])?;
        let position = entry
            .get("position")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected,
            })?;
        let position = label_position(position, key, &labels::POSITIONS)?;
        let x = entry.get("x").ok_or_else(|| TeamTomlError::WrongType {
            key: key.to_string(),
            expected,
        })?;
        let y = entry.get("y").ok_or_else(|| TeamTomlError::WrongType {
            key: key.to_string(),
            expected,
        })?;
        *slot = FormationSlot {
            position: u8::try_from(position).expect("13 positions fit u8"),
            x: ranged(&Item::Value(x.clone()), key, 0, 255)?,
            y: ranged(&Item::Value(y.clone()), key, 0, 255)?,
        };
    }
    Ok(players)
}

pub(super) fn instruction_entries(
    item: &Item,
    key: &str,
) -> Result<[InstructionEntry; 2], TeamTomlError> {
    let expected = "an array of two { instruction, player } tables";
    let array =
        item.as_value()
            .and_then(|v| v.as_array())
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected,
            })?;
    if array.len() != 2 {
        return Err(TeamTomlError::WrongType {
            key: key.to_string(),
            expected,
        });
    }
    let mut entries = [InstructionEntry::default(); 2];
    for (i, (entry, value)) in entries.iter_mut().zip(array.iter()).enumerate() {
        let table = value
            .as_inline_table()
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected,
            })?;
        reject(table, &format!("{key}.{i}"), &["instruction", "player"])?;
        let name = table
            .get("instruction")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected,
            })?;
        let index = label_position(name, key, &labels::INSTRUCTIONS)?;
        let player = table
            .get("player")
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected,
            })?;
        *entry = InstructionEntry {
            instruction: Instruction::ALL[index],
            player: ranged(&Item::Value(player.clone()), key, 0, 255)?,
        };
    }
    Ok(entries)
}

pub(super) fn apply_tactics(
    section: &TacticsSection,
    schema: &TacticsSchema,
    to: PesVersion,
    foreign: bool,
    next: &mut TeamEntry,
    notes: &mut Vec<ImportNote>,
) -> Result<(), TeamTomlError> {
    let tactics = &mut next.tactics;
    if let Some(starting) = section.starting_eleven
        && gated(
            notes,
            foreign,
            schema.has(TacticsField::Starting(0)),
            "tactics.starting_eleven",
        )?
    {
        tactics.starting_eleven = starting;
    }
    if let Some(bench) = section.bench_order
        && gated(
            notes,
            foreign,
            schema.has(TacticsField::Bench(0)),
            "tactics.bench_order",
        )?
    {
        tactics.bench_order[..12].copy_from_slice(&bench);
        // `tactics.starting_eleven` is the section's eleven when it
        // carried one (applied above), the record's else.
        tactics.bench_order[12..].copy_from_slice(&bench_fill(&tactics.starting_eleven, &bench));
    }
    if let Some(join) = section.players_to_join_attack
        && gated(
            notes,
            foreign,
            schema.has(TacticsField::PlayerToJoinAttack(0)),
            "tactics.players_to_join_attack",
        )?
    {
        tactics.players_to_join_attack = join;
    }
    let sp = &section.set_pieces;
    let takers = &mut tactics.set_pieces;
    let pairs: [(Option<u8>, TacticsField, &str, &mut u8); 7] = [
        (
            sp.free_kick_long,
            TacticsField::FreeKickTakerLong,
            "tactics.set_pieces.free_kick_long",
            &mut takers.free_kick_long,
        ),
        (
            sp.free_kick_short,
            TacticsField::FreeKickTakerShort,
            "tactics.set_pieces.free_kick_short",
            &mut takers.free_kick_short,
        ),
        (
            sp.free_kick_second,
            TacticsField::FreeKickTakerSecond,
            "tactics.set_pieces.free_kick_second",
            &mut takers.free_kick_second,
        ),
        (
            sp.corner_left,
            TacticsField::CornerTakerLeft,
            "tactics.set_pieces.corner_left",
            &mut takers.corner_left,
        ),
        (
            sp.corner_right,
            TacticsField::CornerTakerRight,
            "tactics.set_pieces.corner_right",
            &mut takers.corner_right,
        ),
        (
            sp.penalty,
            TacticsField::PenaltyTaker,
            "tactics.set_pieces.penalty",
            &mut takers.penalty,
        ),
        (
            sp.captain,
            TacticsField::Captain,
            "tactics.set_pieces.captain",
            &mut takers.captain,
        ),
    ];
    for (value, field, path, target) in pairs {
        if let Some(value) = value
            && gated(notes, foreign, schema.has(field), path)?
        {
            *target = value;
        }
    }
    let auto = &section.auto;
    if let Some(substitution) = auto.substitution
        && gated(
            notes,
            foreign,
            schema.has(TacticsField::AutoSubstitution),
            "tactics.auto.substitution",
        )?
    {
        tactics.auto.substitution = Some(substitution);
    }
    if let Some(offside_trap) = auto.offside_trap
        && gated(
            notes,
            foreign,
            schema.has(TacticsField::AutoOffsideTrap),
            "tactics.auto.offside_trap",
        )?
    {
        tactics.auto.offside_trap = Some(offside_trap);
    }
    if let Some(preset_change) = auto.preset_change
        && gated(
            notes,
            foreign,
            schema.has(TacticsField::AutoPresetChange),
            "tactics.auto.preset_change",
        )?
    {
        tactics.auto.preset_change = Some(preset_change);
    }
    if let Some(levels) = auto.attack_defence_levels
        && gated(
            notes,
            foreign,
            schema.has(TacticsField::AutoAttackDefenceLevels),
            "tactics.auto.attack_defence_levels",
        )?
    {
        tactics.auto.attack_defence_levels = Some(levels);
    }
    for (i, preset) in section.presets.iter().enumerate() {
        let n = i + 1;
        let target = &mut tactics.presets[i];
        if let Some(marking) = preset.man_marking
            && gated(
                notes,
                foreign,
                schema.has(TacticsField::ManMarking { preset: 0, slot: 0 }),
                &format!("tactics.preset_{n}.man_marking"),
            )?
        {
            target.man_marking = marking;
        }
        let style = &preset.style;
        let target_style = &mut target.style;
        let switches: [(Option<bool>, PresetField, &str, &mut bool); 7] = [
            (
                style.attacking_style,
                PresetField::AttackingStyle,
                "attacking_style",
                &mut target_style.attacking_style,
            ),
            (
                style.attacking_zone,
                PresetField::AttackingZone,
                "attacking_zone",
                &mut target_style.attacking_zone,
            ),
            (
                style.buildup,
                PresetField::Buildup,
                "buildup",
                &mut target_style.buildup,
            ),
            (
                style.positioning,
                PresetField::Positioning,
                "positioning",
                &mut target_style.positioning,
            ),
            (
                style.defensive_style,
                PresetField::DefensiveStyle,
                "defensive_style",
                &mut target_style.defensive_style,
            ),
            (
                style.containment_area,
                PresetField::ContainmentArea,
                "containment_area",
                &mut target_style.containment_area,
            ),
            (
                style.pressure,
                PresetField::Pressure,
                "pressure",
                &mut target_style.pressure,
            ),
        ];
        for (value, field, name, target) in switches {
            let path = format!("tactics.preset_{n}.style.{name}");
            if let Some(value) = value
                && gated(notes, foreign, schema.has_preset(field), &path)?
            {
                *target = value;
            }
        }
        if let Some(fluid) = style.fluid {
            let path = format!("tactics.preset_{n}.style.fluid");
            if gated(
                notes,
                foreign,
                schema.has_preset(PresetField::FluidFormation),
                &path,
            )? {
                target.style.fluid = Some(fluid);
            }
        }
        let sliders = &preset.sliders;
        let target_sliders = &mut target.sliders;
        let slider_pairs: [(Option<u8>, PresetField, &str, &mut u8); 5] = [
            (
                sliders.support_range,
                PresetField::SupportRange,
                "support_range",
                &mut target_sliders.support_range,
            ),
            (
                sliders.defensive_line,
                PresetField::DefensiveLine,
                "defensive_line",
                &mut target_sliders.defensive_line,
            ),
            (
                sliders.compactness,
                PresetField::Compactness,
                "compactness",
                &mut target_sliders.compactness,
            ),
            (
                sliders.numbers_in_attack,
                PresetField::NumbersInAttack,
                "numbers_in_attack",
                &mut target_sliders.numbers_in_attack,
            ),
            (
                sliders.numbers_in_defence,
                PresetField::NumbersInDefence,
                "numbers_in_defence",
                &mut target_sliders.numbers_in_defence,
            ),
        ];
        for (value, field, name, target) in slider_pairs {
            let path = format!("tactics.preset_{n}.sliders.{name}");
            if let Some(value) = value
                && gated(notes, foreign, schema.has_preset(field), &path)?
            {
                *target = value;
            }
        }
        for (f, formation) in preset.formations.iter().enumerate() {
            if let Some(players) = formation {
                target.formations[f].players = *players;
            }
        }
        if let Some(instructions) = &preset.instructions
            && schema.instructions.is_some()
        {
            target.attack_instructions = Some(encode_side(
                to,
                instructions.attack,
                notes,
                &format!("tactics.preset_{n}.instructions.attack"),
            )?);
            target.defence_instructions = Some(encode_side(
                to,
                instructions.defence,
                notes,
                &format!("tactics.preset_{n}.instructions.defence"),
            )?);
        }
    }
    Ok(())
}

/// One side's two entries re-encoded for `to`; an instruction the version
/// cannot store becomes `Off` (stored 0) with a `NotEncodable` note.
fn encode_side(
    to: PesVersion,
    entries: [InstructionEntry; 2],
    notes: &mut Vec<ImportNote>,
    path: &str,
) -> Result<[AdvancedInstruction; 2], TeamTomlError> {
    let mut out = [AdvancedInstruction::default(); 2];
    for (target, entry) in out.iter_mut().zip(entries.iter()) {
        let stored = match instruction::encode(to, entry.instruction) {
            Some(stored) => stored,
            None => {
                let index = Instruction::ALL
                    .iter()
                    .position(|i| *i == entry.instruction)
                    .expect("a canonical instruction is in ALL");
                notes.push(ImportNote::NotEncodable {
                    path: path.to_string(),
                    label: labels::INSTRUCTIONS[index].to_string(),
                });
                0
            }
        };
        *target = AdvancedInstruction {
            instruction: stored,
            player: entry.player,
        };
    }
    Ok(out)
}
