//! PES 2014's Game Plan (the draft's `tacticsTab14`): the Preset tactics
//! panel's Formation and Strategy dropdowns and its eight sliders, the
//! game's formation list a pick writes, and the sliders a Strategy pick
//! sets.

use pes_savefile::model::tactics::{
    CombinationSlot, Formation, FormationSlot, GamePlan, TacticsPreset, TeamTactics,
};
use pes_savefile::schema::fields::PresetField;
use studio_core::widgets::{add_enabled, add_enabled_ui, slider as suite_slider};

use super::{
    CELL_GAP, COMBINATION_PLAY_LABEL, GAME_PLAN_SLIDERS, HALVES_GAP, STRATEGY_LABEL, TITLE_GAP,
    TacticsView, choice_combo, control_box, greyed_choice_combo, panel, text_width,
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

/// One Combination Play strategy: its stored `id` (1 to 14, not the
/// picker's order), its `name`, the area the game's area picker opens on
/// and the `areas` it can run in, in the game's area numbers (rows from
/// the top of its area screen: 1 and 2, 3 to 5, 6 to 8, 9 to 11).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Combination {
    /// The stored strategy id.
    pub id: u8,
    /// The strategy's name in the game's picker.
    pub name: &'static str,
    /// The area the game's area picker opens on.
    pub opens_on: u8,
    /// The areas the strategy can run in.
    pub areas: &'static [u8],
}

/// The six midfield areas most strategies take.
const MIDFIELD: &[u8] = &[3, 4, 5, 6, 7, 8];

/// Combination Play's strategies in the game's picker order, each with
/// the areas it takes when the other two slots are empty.
pub const COMBINATIONS: [Combination; 14] = [
    Combination {
        id: 10,
        name: "Dummy Run and Cross",
        opens_on: 1,
        areas: &[1, 2, 3, 5],
    },
    Combination {
        id: 11,
        name: "Cross and Pull Back",
        opens_on: 1,
        areas: &[1, 2, 3, 5],
    },
    Combination {
        id: 12,
        name: "Cross and Dummy",
        opens_on: 1,
        areas: &[1, 2, 3, 5],
    },
    Combination {
        id: 13,
        name: "Switch Flanks",
        opens_on: 6,
        areas: &[6, 8, 9, 11],
    },
    Combination {
        id: 14,
        name: "Dummy and Lay Off",
        opens_on: 3,
        areas: MIDFIELD,
    },
    Combination {
        id: 1,
        name: "Hold up the ball",
        opens_on: 3,
        areas: MIDFIELD,
    },
    Combination {
        id: 2,
        name: "Diagonal Run",
        opens_on: 3,
        areas: MIDFIELD,
    },
    Combination {
        id: 3,
        name: "Cut Inside",
        opens_on: 3,
        areas: &[3, 5],
    },
    Combination {
        id: 4,
        name: "Diagonal Dribble",
        opens_on: 3,
        areas: MIDFIELD,
    },
    Combination {
        id: 5,
        name: "Hold Up and Overlap",
        opens_on: 3,
        areas: MIDFIELD,
    },
    Combination {
        id: 6,
        name: "Straight Run",
        opens_on: 3,
        areas: MIDFIELD,
    },
    Combination {
        id: 7,
        name: "Hold Up and Switch",
        opens_on: 3,
        areas: &[3, 5, 6, 8],
    },
    Combination {
        id: 8,
        name: "Overlap and Pull Back",
        opens_on: 7,
        areas: &[4, 5, 7, 10],
    },
    Combination {
        id: 9,
        name: "Drop Deep and Overlap",
        opens_on: 7,
        areas: &[4, 5, 7, 10],
    },
];

/// The strategy id of an empty Combination Play slot.
const NO_COMBINATION: u8 = 0;

/// A stored Combination Play strategy id as the panel names it: "None
/// Selected" for an empty slot, the strategy's name, or the number for
/// an id `COMBINATIONS` does not hold.
pub(crate) fn combination_text(id: u8) -> String {
    if id == NO_COMBINATION {
        return "None Selected".to_owned();
    }
    COMBINATIONS
        .iter()
        .find(|combination| combination.id == id)
        .map_or_else(|| id.to_string(), |combination| combination.name.to_owned())
}

/// The areas strategy `id` could run in from slot `index`: its own areas
/// less those the other two slots hold. A slot whose strategy is 0 is
/// empty, so its area holds nothing, whatever the byte says. Empty for an
/// id `COMBINATIONS` does not hold.
pub fn free_areas(slots: &[CombinationSlot; 3], index: usize, id: u8) -> Vec<u8> {
    let Some(combination) = COMBINATIONS.iter().find(|combination| combination.id == id) else {
        return Vec::new();
    };
    let held = |area: u8| {
        slots.iter().enumerate().any(|(other, slot)| {
            other != index && slot.strategy != NO_COMBINATION && slot.area == area
        })
    };
    combination
        .areas
        .iter()
        .copied()
        .filter(|area| !held(*area))
        .collect()
}

/// Puts strategy `id` in slot `index`, as the game's pick does: the
/// slot's area stays when it is one of `free_areas`, else it becomes the
/// area the game's picker opens on when that one is free, else the first
/// free one. `id` must have a free area: the panel greys one that has
/// none.
pub fn pick_combination(slots: &mut [CombinationSlot; 3], index: usize, id: u8) {
    let free = free_areas(slots, index, id);
    let opens_on = COMBINATIONS
        .iter()
        .find(|combination| combination.id == id)
        .map(|combination| combination.opens_on);
    let slot = &mut slots[index];
    if !free.contains(&slot.area) {
        slot.area = match opens_on {
            Some(area) if free.contains(&area) => area,
            _ => *free
                .first()
                .expect("the panel offers no strategy without a free area"),
        };
    }
    slot.strategy = id;
}

/// Empties slot `index` as the game's Clear does: strategy 0 and area 0.
pub fn clear_combination(slots: &mut [CombinationSlot; 3], index: usize) {
    slots[index] = CombinationSlot {
        area: 0,
        strategy: NO_COMBINATION,
    };
}

/// The Area header's tooltip. The game has no text for the areas, so it
/// gives the game's numbering, with no claim about which end is whose.
const COMBINATION_AREA_TIP: &str = "The game's area screen numbers the pitch in rows, from \
                                    its top: 1 and 2, then 3 to 5, 6 to 8, and 9 to 11.";
/// The Combination play grid's column and row gaps (the draft's
/// `.tacinstr`).
const COMBINATION_COLUMN_GAP: f32 = 8.0;
const COMBINATION_ROW_GAP: f32 = 3.0;
/// The area column's width (the draft's `52px` track).
const AREA_WIDTH: f32 = 52.0;

/// The "Combination play" panel (the draft's `#tac-combination`): the
/// Strategy and Area headers, then the three slots, each its number, its
/// strategy combo, its area combo and a Clear button. Draws nothing on a
/// record that stores no Combination Play. Answers `true` only when
/// `tactics` changed this frame.
pub fn combination_play(ui: &mut egui::Ui, tactics: &mut TeamTactics) -> bool {
    let Some(slots) = tactics.combination.as_mut() else {
        return false;
    };
    let before = *slots;
    panel(ui, COMBINATION_PLAY_LABEL, |ui| {
        // `add_space` adds to the item gap: the rows start `TITLE_GAP`
        // under the heading, as the other panels' do.
        ui.add_space(TITLE_GAP - ui.spacing().item_spacing.y);
        combination_grid(ui, slots);
    });
    *slots != before
}

/// The panel's grid (the draft's columns `auto 1fr 52px auto`): the
/// numbers, the strategy combos filling the free width, the area combos
/// and the Clear buttons, under a header row one line of text tall.
fn combination_grid(ui: &mut egui::Ui, slots: &mut [CombinationSlot; 3]) {
    let inner = ui.max_rect();
    let number_width = ["1", "2", "3"]
        .map(|number| text_width(ui, number))
        .into_iter()
        .fold(0.0, f32::max);
    let clear_width = text_width(ui, "Clear") + 2.0 * ui.spacing().button_padding.x;
    let strategy_x = inner.min.x + number_width + COMBINATION_COLUMN_GAP;
    let strategy_width =
        inner.width() - number_width - AREA_WIDTH - clear_width - 3.0 * COMBINATION_COLUMN_GAP;
    let area_x = strategy_x + strategy_width + COMBINATION_COLUMN_GAP;
    let clear_x = area_x + AREA_WIDTH + COMBINATION_COLUMN_GAP;
    let combo_height = ui.spacing().interact_size.y;
    let header_height = ui.text_style_height(&egui::TextStyle::Body);
    let mut top = ui.cursor().min.y;
    grid_cell(ui, strategy_x, top, strategy_width, header_height).label(STRATEGY_LABEL);
    grid_cell(ui, area_x, top, AREA_WIDTH, header_height)
        .label("Area")
        .on_hover_text(COMBINATION_AREA_TIP);
    top += header_height + COMBINATION_ROW_GAP;
    for index in 0..slots.len() {
        grid_cell(ui, inner.min.x, top, number_width, combo_height).weak((index + 1).to_string());
        strategy_combo_cell(
            &mut grid_cell(ui, strategy_x, top, strategy_width, combo_height),
            slots,
            index,
        );
        area_combo_cell(
            &mut grid_cell(ui, area_x, top, AREA_WIDTH, combo_height),
            slots,
            index,
        );
        let empty = slots[index].strategy == NO_COMBINATION;
        if add_enabled(
            &mut grid_cell(ui, clear_x, top, clear_width, combo_height),
            !empty,
            egui::Button::new("Clear"),
        )
        .clicked()
        {
            clear_combination(slots, index);
        }
        top += combo_height + COMBINATION_ROW_GAP;
    }
    // `new_child` cells don't grow this `Ui`'s `min_rect`: advance over
    // the grid's whole extent so the frame wraps the last row.
    ui.advance_cursor_after_rect(egui::Rect::from_min_max(
        inner.min,
        egui::pos2(inner.max.x, top - COMBINATION_ROW_GAP),
    ));
}

/// A cell of the Combination play grid at `left`, `top`, its content
/// centred in its height.
fn grid_cell(ui: &mut egui::Ui, left: f32, top: f32, width: f32, height: f32) -> egui::Ui {
    ui.new_child(
        egui::UiBuilder::new()
            .max_rect(egui::Rect::from_min_size(
                egui::pos2(left, top),
                egui::vec2(width, height),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    )
}

/// Slot `index`'s strategy combo: "None Selected" first while the slot
/// is empty, then `COMBINATIONS`' names, those with no free area for the
/// slot greyed, since the game could store no area for them. A stored id
/// the table does not hold is offered as its number. A pick calls
/// `pick_combination`.
fn strategy_combo_cell(ui: &mut egui::Ui, slots: &mut [CombinationSlot; 3], index: usize) {
    let held = slots[index].strategy;
    let mut choices: Vec<(u8, String)> = Vec::new();
    if held == NO_COMBINATION {
        choices.push((NO_COMBINATION, combination_text(NO_COMBINATION)));
    }
    choices.extend(
        COMBINATIONS
            .iter()
            .map(|combination| (combination.id, combination.name.to_owned())),
    );
    if !choices.iter().any(|(id, _)| *id == held) {
        choices.push((held, combination_text(held)));
    }
    let pick = greyed_choice_combo(ui, ("combination strategy", index), &choices, held, |id| {
        id == NO_COMBINATION || !free_areas(slots, index, id).is_empty()
    });
    if pick != held {
        pick_combination(slots, index, pick);
    }
}

/// Slot `index`'s area combo: the areas `free_areas` leaves its strategy,
/// as numbers, a stored area outside them offered too, so the combo
/// never misreports the save. Empty and disabled for an empty slot. A
/// pick sets the slot's area alone.
fn area_combo_cell(ui: &mut egui::Ui, slots: &mut [CombinationSlot; 3], index: usize) {
    let slot = slots[index];
    let id_salt = ("combination area", index);
    if slot.strategy == NO_COMBINATION {
        add_enabled_ui(ui, false, |ui| {
            choice_combo(ui, id_salt, &[(slot.area, String::new())], slot.area)
        });
        return;
    }
    let mut choices: Vec<(u8, String)> = free_areas(slots, index, slot.strategy)
        .into_iter()
        .map(|area| (area, area.to_string()))
        .collect();
    if !choices.iter().any(|(area, _)| *area == slot.area) {
        choices.push((slot.area, slot.area.to_string()));
    }
    slots[index].area = choice_combo(ui, id_salt, &choices, slot.area);
}

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
