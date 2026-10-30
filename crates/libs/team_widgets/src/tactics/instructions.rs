//! The "Advanced instructions" grid: the headers Attack and Defence over
//! their two equal columns, each instruction's number
//! beside its two combos — the attack ones listing the attacking
//! instructions, the defence ones the defending, less Tight Marking —
//! and each target a chip under its combo, armed to a pitch click.

use pes_savefile::model::instruction::Instruction;
use pes_savefile::model::tactics::{AdvancedInstruction, TacticsPreset, TeamTactics};
use pes_savefile::schema::fields::{InstructionPart, InstructionSide, TacticsField};
use pes_savefile::schema::instruction;
use pes_version::PesVersion;
use studio_core::widgets::{add_enabled_ui, step_choice, wheel_steps};

use super::assignments::{Assign, chip};
use super::{RosterRow, TacticsView, gate_tip, name_at, text_width};

/// The stored "no player" value of an instruction's target.
pub(super) const NO_TARGET: u8 = 0xFF;

/// The instructions' columns' and rows' gaps (the draft's `.tacinstr`
/// `column-gap: 8px; row-gap: 3px`).
const INSTR_COLUMN_GAP: f32 = 8.0;
const INSTR_ROW_GAP: f32 = 3.0;

/// What a side's combo offers (the draft's `instructionsFor`): the
/// version's stored table in its stored order, its own side's
/// instructions plus `Off`, less Tight Marking — it marks an opponent,
/// which Edit mode does not offer.
fn offered(version: PesVersion, side: InstructionSide) -> Vec<Instruction> {
    instruction::instructions(version)
        .iter()
        .copied()
        .filter(|inst| *inst != Instruction::TightMarking)
        .filter(|inst| *inst == Instruction::Off || inst.side() == Some(side))
        .collect()
}

/// One instruction's combo (the draft's `select`, filling its cell):
/// the side's list in stored order — a held value the list lacks
/// (Tight Marking or one of the other side's) appended as the current
/// choice — a stored byte the table does not list shown as its hex
/// and kept until a pick. Picking an instruction that takes no target
/// clears the pick.
fn instruction_combo(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    version: PesVersion,
    side: InstructionSide,
    entry: &mut AdvancedInstruction,
) {
    let current = instruction::decode(version, entry.instruction)
        .ok()
        .flatten();
    let mut list = offered(version, side);
    if let Some(inst) = current
        && !list.contains(&inst)
    {
        list.push(inst);
    }
    let selected = match current {
        Some(inst) => instruction_label(inst).to_owned(),
        None => format!("Unknown (0x{:02X})", entry.instruction),
    };
    let mut pick = current;
    let combo = egui::ComboBox::from_id_salt(id_salt)
        .width(ui.available_width())
        .selected_text(selected.clone())
        .truncate()
        .show_ui(ui, |ui| {
            for inst in &list {
                ui.selectable_value(&mut pick, Some(*inst), instruction_label(*inst));
            }
        })
        .response;
    // An unknown stored byte is not among the items, so the wheel leaves it.
    let items = list.iter().copied().map(Some);
    if let Some(next) = step_choice(items, &pick, wheel_steps(ui, &combo)) {
        pick = next;
    }
    if let Some(inst) = pick
        && pick != current
    {
        entry.instruction =
            instruction::encode(version, inst).expect("the offered list is the version's own");
        if !inst.takes_player() {
            entry.player = NO_TARGET;
        }
    }
}

/// One instruction's target chip (the draft's `pick`, a `rolechip` of
/// its column's width): armed, the next pitch click writes the clicked
/// starter to the target (`mode`, the slot's `Assign::Target`) — a
/// same-player write skipped — and a click on the armed chip disarms;
/// under an instruction that takes none the chip is disabled, reading
/// `No target` — or `An opponent` under a held Tight Marking — the
/// hovers the draft's titles.
fn target_chip(
    ui: &mut egui::Ui,
    version: PesVersion,
    mode: Assign,
    entry: &AdvancedInstruction,
    view: &mut TacticsView,
    roster: &[RosterRow<'_>],
) {
    let current = instruction::decode(version, entry.instruction)
        .ok()
        .flatten();
    let targetable = current.is_some_and(|inst| inst.takes_player());
    let marking = current == Some(Instruction::TightMarking);
    let armed = view.assign == Some(mode);
    // `target_text`'s wording except a byte the table does not decode
    // (`current == None`, read as no target), so the chip and the
    // changes row cannot drift.
    let text = if current.is_none() {
        "No target".to_owned()
    } else {
        target_text(current, entry.player, roster)
    };
    let title = if marking {
        "Its target is an opponent, picked in a match"
    } else if !targetable {
        "This instruction takes no target"
    } else {
        "The instruction's target: click, then a pitch player"
    };
    if !targetable {
        add_enabled_ui(ui, false, |ui| chip(ui, &text, false, title))
            .response
            .on_disabled_hover_text(title);
    } else if chip(ui, &text, armed, title).clicked() {
        view.assign = (!armed).then_some(mode);
        view.popover = None;
    }
}

/// The "Advanced instructions" group's grid (the draft's `.tacinstr`,
/// PES 17+): the headers Attack and Defence over their two equal
/// columns, each instruction's number weak beside its two combos and
/// its target chip under its combo. A version with no instruction
/// table holds the weak line `(none on this version)` with the `PES
/// NN+` tooltip instead of disabled rows. The panel's heading is the
/// caller's `panel`.
pub(super) fn instruction_table(
    ui: &mut egui::Ui,
    version: PesVersion,
    preset: &mut TacticsPreset,
    view: &mut TacticsView,
    roster: &[RosterRow<'_>],
) {
    let (Some(attack), Some(defence)) = (
        preset.attack_instructions.as_mut(),
        preset.defence_instructions.as_mut(),
    ) else {
        add_enabled_ui(ui, false, |ui| ui.weak("(none on this version)"))
            .response
            .on_disabled_hover_text(gate_tip(TacticsField::Instruction {
                preset: 0,
                side: InstructionSide::Attack,
                index: 0,
                part: InstructionPart::Instruction,
            }));
        return;
    };
    // `.tacinstr`: the numbers' `auto` column, then the two equal combo
    // columns `1fr 1fr`, all `INSTR_COLUMN_GAP` apart, `INSTR_ROW_GAP`
    // between rows.
    let inner = ui.max_rect();
    let number_width = text_width(ui, "1").max(text_width(ui, "2"));
    let column = (inner.width() - number_width - 2.0 * INSTR_COLUMN_GAP) / 2.0;
    let attack_x = inner.min.x + number_width + INSTR_COLUMN_GAP;
    let defence_x = attack_x + column + INSTR_COLUMN_GAP;
    let combo_height = ui.spacing().interact_size.y;
    let cell = |ui: &mut egui::Ui, left: f32, top: f32, height: f32| {
        ui.new_child(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_size(
                    egui::pos2(left, top),
                    egui::vec2(column, height),
                ))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        )
    };
    // The header row: an empty number cell, the side names over their
    // columns, one line of text tall (the draft's header cells are plain
    // text, not a combo's height).
    let header_height = ui.text_style_height(&egui::TextStyle::Body);
    let mut top = ui.cursor().min.y;
    for (x, header) in [(attack_x, "Attack"), (defence_x, "Defence")] {
        cell(ui, x, top, header_height).label(header);
    }
    top += header_height + INSTR_ROW_GAP;
    for index in 0..2 {
        let mut number_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_size(
                    egui::pos2(inner.min.x, top),
                    egui::vec2(number_width, combo_height),
                ))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        number_ui.weak((index + 1).to_string());
        for (x, side, entry) in [
            (attack_x, InstructionSide::Attack, &mut attack[index]),
            (defence_x, InstructionSide::Defence, &mut defence[index]),
        ] {
            instruction_combo(
                &mut cell(ui, x, top, combo_height),
                (side, index),
                version,
                side,
                entry,
            );
        }
        top += combo_height + INSTR_ROW_GAP;
        for (x, side, entries) in [
            (attack_x, InstructionSide::Attack, &*attack),
            (defence_x, InstructionSide::Defence, &*defence),
        ] {
            let mut chip_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(egui::Rect::from_min_size(
                        egui::pos2(x, top),
                        egui::vec2(column, combo_height),
                    ))
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );
            target_chip(
                &mut chip_ui,
                version,
                Assign::Target { side, index },
                &entries[index],
                view,
                roster,
            );
        }
        top += combo_height + INSTR_ROW_GAP;
    }
    // `new_child` cells don't grow this `Ui`'s `min_rect`: advance over
    // the grid's whole extent so the frame wraps the last chip and the
    // next panel starts below it.
    ui.advance_cursor_after_rect(egui::Rect::from_min_max(
        inner.min,
        egui::pos2(inner.max.x, top - INSTR_ROW_GAP),
    ));
}

/// The instruction byte `tactics` stores on `side` at `index` of
/// `preset`, decoded; `None` where the version or the entry holds none —
/// `target_text` then reads the byte as a player's, as the tab's `None`
/// arm does.
pub(super) fn instruction_at(
    version: PesVersion,
    tactics: &TeamTactics,
    preset: u8,
    side: InstructionSide,
    index: u8,
) -> Option<Instruction> {
    let preset = tactics.presets.get(usize::from(preset))?;
    let entries = match side {
        InstructionSide::Attack => &preset.attack_instructions,
        InstructionSide::Defence => &preset.defence_instructions,
    };
    let entry = entries.as_ref()?.get(usize::from(index))?;
    instruction::decode(version, entry.instruction)
        .ok()
        .flatten()
}

/// The target chip's text under the instruction in force: "An opponent"
/// for a held Tight Marking (its byte is the opposing formation's slot,
/// not one of ours), "No target" under an instruction that takes none
/// (the game reads no target there), "Unassigned" for an unset byte,
/// the roster slot's name else — `target_chip`'s text and the
/// unsaved-changes row's one wording, so the two cannot drift.
pub(crate) fn target_text(
    instruction: Option<Instruction>,
    target: u8,
    roster: &[RosterRow<'_>],
) -> String {
    match instruction {
        Some(Instruction::TightMarking) => "An opponent".to_owned(),
        Some(inst) if !inst.takes_player() => "No target".to_owned(),
        _ if target == NO_TARGET => "Unassigned".to_owned(),
        _ => name_at(roster, target).to_owned(),
    }
}

/// The instruction's text in the combo and its rows (the draft's
/// `INSTRUCTION_LABEL`; `Off` reads "Unassigned").
pub(super) fn instruction_label(instruction: Instruction) -> &'static str {
    match instruction {
        Instruction::Off => "Unassigned",
        Instruction::HugTheTouchline => "Hug the Touchline",
        Instruction::FalseNo9 => "False No. 9",
        Instruction::FalseFullBacks => "False Full-Backs",
        Instruction::AttackingFullBacks => "Attacking Full-Backs",
        Instruction::WingRotation => "Wing Rotation",
        Instruction::TikiTaka => "Tiki-Taka",
        Instruction::CenteringTargets => "Centering Targets",
        Instruction::SwarmTheBox => "Swarm the Box",
        Instruction::DeepDefensiveLine => "Deep Defensive Line",
        Instruction::Gegenpress => "Gegenpress",
        Instruction::TightMarking => "Tight Marking",
        Instruction::CounterTarget => "Counter Target",
        Instruction::Defensive => "Defensive",
        Instruction::FalseWinger => "False Winger",
        Instruction::WingBack => "Wing-Back",
        Instruction::Anchoring => "Anchoring",
    }
}

#[cfg(test)]
mod tests;
