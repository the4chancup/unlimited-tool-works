//! PES 2014's Game Plan (the draft's `tacticsTab14`): the Preset tactics
//! panel's Formation and Strategy dropdowns and its eight sliders, the
//! game's formation list a pick writes, and the sliders a Strategy pick
//! sets.

use pes_savefile::model::tactics::{
    Formation, FormationSlot, GamePlan, TacticsPreset, TeamTactics,
};
use pes_savefile::schema::fields::PresetField;
use studio_core::widgets::slider as suite_slider;

use super::{
    CELL_GAP, GAME_PLAN_SLIDERS, HALVES_GAP, STRATEGY_LABEL, TITLE_GAP, TacticsView, choice_combo,
    control_box, panel,
};
use crate::pitch::formation_name;

/// One formation list entry's outfielders: `(position, x, y)` in slot
/// order 1 to 10 (x the width, y the depth, as `FormationSlot` holds
/// them).
type Outfielders = [(u8, u8, u8); 10];

/// The game's formation list, in its order: each entry's name and its
/// ten outfielders. The keeper's slot is no part of an entry.
pub const LIST_FORMATIONS: [(&str, Outfielders); 11] = [
    (
        "4-4-2",
        [
            (1, 63, 10),
            (1, 41, 10),
            (3, 89, 12),
            (2, 15, 12),
            (5, 64, 27),
            (5, 40, 27),
            (5, 87, 30),
            (5, 17, 30),
            (12, 66, 43),
            (12, 38, 43),
        ],
    ),
    (
        "4-3-1-2",
        [
            (1, 63, 10),
            (1, 41, 10),
            (3, 89, 12),
            (2, 15, 12),
            (4, 52, 18),
            (5, 70, 27),
            (5, 34, 27),
            (8, 52, 35),
            (12, 66, 43),
            (12, 38, 43),
        ],
    ),
    (
        "4-2-3-1",
        [
            (1, 63, 10),
            (1, 41, 10),
            (3, 89, 12),
            (2, 15, 12),
            (4, 64, 18),
            (4, 40, 18),
            (7, 86, 30),
            (6, 18, 30),
            (8, 52, 32),
            (12, 52, 43),
        ],
    ),
    (
        "4-2-1-3",
        [
            (1, 63, 10),
            (1, 41, 10),
            (3, 89, 12),
            (2, 15, 12),
            (4, 64, 18),
            (4, 40, 18),
            (8, 52, 32),
            (10, 81, 42),
            (9, 23, 42),
            (12, 52, 43),
        ],
    ),
    (
        "4-3-2-1",
        [
            (1, 63, 10),
            (1, 41, 10),
            (3, 89, 12),
            (2, 15, 12),
            (4, 52, 18),
            (5, 70, 25),
            (5, 34, 25),
            (7, 86, 31),
            (6, 18, 31),
            (12, 52, 43),
        ],
    ),
    (
        "4-1-2-3",
        [
            (1, 63, 10),
            (1, 41, 10),
            (3, 89, 12),
            (2, 15, 12),
            (4, 52, 18),
            (8, 70, 32),
            (8, 34, 32),
            (10, 81, 42),
            (9, 23, 42),
            (12, 52, 43),
        ],
    ),
    (
        "4-3-3",
        [
            (1, 63, 10),
            (1, 41, 10),
            (3, 89, 12),
            (2, 15, 12),
            (5, 52, 24),
            (5, 78, 27),
            (5, 20, 27),
            (10, 81, 42),
            (9, 23, 42),
            (12, 52, 43),
        ],
    ),
    (
        "3-2-3-2",
        [
            (1, 52, 10),
            (1, 73, 10),
            (1, 31, 10),
            (4, 64, 18),
            (4, 40, 18),
            (7, 86, 30),
            (6, 18, 30),
            (8, 52, 32),
            (12, 66, 43),
            (12, 38, 43),
        ],
    ),
    (
        "3-1-4-2",
        [
            (1, 52, 10),
            (1, 73, 10),
            (1, 31, 10),
            (4, 52, 18),
            (7, 86, 30),
            (6, 18, 30),
            (8, 67, 32),
            (8, 37, 32),
            (12, 66, 43),
            (12, 38, 43),
        ],
    ),
    (
        "3-2-2-3",
        [
            (1, 52, 10),
            (1, 73, 10),
            (1, 31, 10),
            (5, 67, 23),
            (5, 37, 23),
            (7, 86, 31),
            (6, 18, 31),
            (10, 81, 42),
            (9, 23, 42),
            (12, 52, 43),
        ],
    ),
    (
        "5-2-2-1",
        [
            (1, 52, 10),
            (1, 73, 10),
            (1, 31, 10),
            (3, 89, 12),
            (2, 15, 12),
            (4, 64, 18),
            (4, 40, 18),
            (7, 86, 31),
            (6, 18, 31),
            (12, 52, 43),
        ],
    ),
];

/// The Strategy dropdown's entries, in the game's picker order: the
/// stored value is the index.
pub const STRATEGIES: [&str; 6] = [
    "Custom",
    "Long Ball",
    "Quick Counter",
    "Possession Game",
    "All-out Defence",
    "All-out Attack",
];

/// The sliders a Strategy pick sets, shown values in the panel's order;
/// `None` leaves a slider as it is (the draft's `STRATEGY_SLIDERS14`).
/// Custom sets none, and a slider moved afterwards keeps the strategy,
/// as the game's pick does.
const STRATEGY_SLIDERS: [[Option<u8>; 8]; 6] = [
    [None; 8],
    [None, None, None, None, None, Some(5), Some(15), None],
    [
        Some(10),
        Some(10),
        None,
        Some(15),
        Some(15),
        Some(15),
        Some(15),
        Some(10),
    ],
    [
        Some(15),
        Some(10),
        None,
        Some(10),
        Some(10),
        Some(10),
        None,
        Some(10),
    ],
    [None, None, None, None, None, Some(10), None, None],
    [
        None,
        None,
        None,
        Some(15),
        Some(15),
        None,
        Some(15),
        Some(20),
    ],
];

/// The list entry `formation` is, `None` for one the list does not
/// hold: the entry whose ten outfielders equal slots 1 to 10 in
/// position, x and y. The keeper's slot is not compared.
pub fn listed_formation(formation: &Formation) -> Option<usize> {
    LIST_FORMATIONS
        .iter()
        .position(|(_, slots)| outfielders(slots).as_slice() == &formation.players[1..])
}

/// `slots` as the `FormationSlot`s a formation's outfielders compare
/// against.
fn outfielders(slots: &Outfielders) -> [FormationSlot; 10] {
    slots.map(|(position, x, y)| FormationSlot { position, x, y })
}

/// Writes `entry`'s formation to `formation`: slots 1 to 10 take the
/// entry's, and the keeper's depth (`players[0].y`) becomes 3, his
/// width and position kept, as the game's pick writes.
pub fn pick_formation(formation: &mut Formation, entry: usize) {
    formation.players[1..].copy_from_slice(&outfielders(&LIST_FORMATIONS[entry].1));
    formation.players[0].y = 3;
}

/// Writes `strategy` and the sliders its `STRATEGY_SLIDERS` row sets to
/// `plan`, as the game's pick does. `strategy` is an index into
/// `STRATEGIES`, so 0 to 5.
pub fn pick_strategy(plan: &mut GamePlan, strategy: u8) {
    plan.strategy = strategy;
    for (slider, value) in plan
        .sliders
        .iter_mut()
        .zip(&STRATEGY_SLIDERS[usize::from(strategy)])
    {
        if let Some(value) = value {
            *slider = *value;
        }
    }
}

/// The "Preset tactics" panel over the shown preset (the draft's
/// `#tac-settings` group on PES 2014): the Formation and Strategy
/// dropdowns, then the eight Game Plan sliders, 0 to 20, five rows of
/// two halves sharing the panel's free height, as `formation_settings`
/// builds its cells. Edits the preset on screen; answers `true` only
/// when `tactics` changed this frame.
pub fn preset_tactics(ui: &mut egui::Ui, tactics: &mut TeamTactics, view: &TacticsView) -> bool {
    let preset = &mut tactics.presets[view.preset.min(2)];
    // The answer is the deep compare, not the responses' `changed()`: a
    // control re-picked at its own value reports nothing.
    let before = preset.clone();
    panel(ui, "Preset tactics", |ui| {
        // The frame's bottom is the cell's, as the pitch's is.
        ui.set_min_height(ui.available_height());
        ui.add_space(TITLE_GAP - ui.spacing().item_spacing.y);
        let rows_top = ui.cursor().min.y;
        let inner = ui.max_rect();
        // The cells — Formation, Strategy, the eight sliders — two a
        // row, the five rows sharing the free height.
        const CELLS: usize = 10;
        let row_height = (inner.max.y - rows_top) / (CELLS / 2) as f32;
        let half = (inner.width() - HALVES_GAP) / 2.0;
        for index in 0..CELLS {
            let top = rows_top + (index / 2) as f32 * row_height;
            let left = inner.min.x + (index % 2) as f32 * (half + HALVES_GAP);
            let mut cell_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(egui::Rect::from_min_size(
                        egui::pos2(left, top),
                        egui::vec2(half, row_height),
                    ))
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );
            // The cell's content at its top, the control 2 under the
            // label (the draft's `.tset`).
            cell_ui.spacing_mut().item_spacing.y = CELL_GAP;
            preset_cell(&mut cell_ui, preset, index);
        }
    });
    *preset != before
}

/// One cell of the Preset tactics panel: Formation, Strategy or the
/// `index - 2`th Game Plan slider, its label over its control.
fn preset_cell(ui: &mut egui::Ui, preset: &mut TacticsPreset, index: usize) {
    match index {
        0 => {
            ui.label("Formation");
            // The dropdowns stand straight under their labels, as the
            // draft's selects do — no `control_box`, which centres a
            // control in a taller box than the draft's.
            formation_combo(ui, &mut preset.formations[0]);
        }
        1 => {
            ui.label(STRATEGY_LABEL);
            strategy_combo(ui, preset);
        }
        _ => {
            let slider = index - 2;
            ui.label(GAME_PLAN_SLIDERS[slider]);
            control_box(ui, |ui| game_plan_slider(ui, preset, slider));
        }
    }
}

/// The Formation cell's combo (the draft's formation `select`): the
/// list's entries; a formation the list does not hold offered first, by
/// its shape marked as such, and a pick of it changes nothing. A pick
/// of a list entry calls `pick_formation` on `formation`.
fn formation_combo(ui: &mut egui::Ui, formation: &mut Formation) {
    let listed = listed_formation(formation);
    let mut choices: Vec<(i32, String)> = Vec::new();
    if listed.is_none() {
        choices.push((
            -1,
            format!("{} (not in the game's list)", formation_name(formation)),
        ));
    }
    choices.extend(
        LIST_FORMATIONS
            .iter()
            .enumerate()
            .map(|(entry, (name, _))| (entry as i32, (*name).to_owned())),
    );
    let held = listed.map(|entry| entry as i32).unwrap_or(-1);
    let pick = choice_combo(ui, "game plan formation", &choices, held);
    if pick >= 0 && Some(pick as usize) != listed {
        pick_formation(formation, pick as usize);
    }
}

/// The Strategy cell's combo (the draft's strategy `select`):
/// `STRATEGIES`, a pick calling `pick_strategy`. A stored value outside
/// the six is offered as its number, so the combo never misreports the
/// save (the Substitution cell's rule).
fn strategy_combo(ui: &mut egui::Ui, preset: &mut TacticsPreset) {
    let plan = preset
        .game_plan
        .as_mut()
        .expect("the codec reads a Game Plan on every PES 2014 preset");
    let mut choices: Vec<(u8, String)> = STRATEGIES
        .iter()
        .enumerate()
        .map(|(index, name)| (index as u8, (*name).to_owned()))
        .collect();
    if !choices.iter().any(|(value, _)| *value == plan.strategy) {
        choices.push((plan.strategy, plan.strategy.to_string()));
    }
    let pick = choice_combo(ui, "game plan strategy", &choices, plan.strategy);
    if pick != plan.strategy {
        pick_strategy(plan, pick);
    }
}

/// One Game Plan slider's control: the suite's slider over 0 to 20
/// (the rail filling the half less the value's field), writing that
/// slider alone — the Strategy stays, as the game keeps it.
fn game_plan_slider(ui: &mut egui::Ui, preset: &mut TacticsPreset, index: usize) {
    let plan = preset
        .game_plan
        .as_mut()
        .expect("the codec reads a Game Plan on every PES 2014 preset");
    let width = (ui.available_width() - ui.spacing().item_spacing.x - ui.spacing().interact_size.x)
        .max(0.0);
    let mut value = plan.sliders[index];
    let wheel_id = ui.scope_id().with(PresetField::GamePlanSlider(index as u8));
    let response = suite_slider(ui, &mut value, 0..=20, width, wheel_id);
    if response.changed() {
        plan.sliders[index] = value;
    }
}

#[cfg(test)]
mod tests;
