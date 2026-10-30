//! The Tiers section (the draft's `VIEWS.tiers`): the tiers table, one
//! row per tier, highest first and reordered by dragging the handle,
//! each row's expander holding the per-stat targets, the free cards and
//! the conditions, and the medal systems under the table.

use std::collections::{BTreeMap, BTreeSet};

use aatf::{MedalSystem, Ruleset, Stat, Tier, TierId};
use pes_savefile::interchange::team_toml::labels::ABILITY_STATS;
use studio_core::widgets::{
    add_enabled, checkbox, collapsing_header, step_choice, step_number, text_box, wheel_steps,
};

use super::Section;
use super::widgets;
use crate::summary::human;

/// The table's columns (the draft's `th` row): the handle, label, id,
/// count, counts as, rate, form, injury resistance, weak foot, paid
/// cards, minimum cards, A positions, name colour, warning band, stats
/// within targets and the ✕.
const COLUMNS: usize = 16;

/// The Tiers form; `id_edit` is the id field's typed text and its row
/// (`RulesetView::tier_id_edit`).
pub(crate) fn show(
    ui: &mut egui::Ui,
    ruleset: &mut Ruleset,
    id_edit: &mut Option<(usize, String)>,
) {
    ui.heading(Section::Tiers.label());
    ui.weak(
        "Highest first: a player gets the first tier whose conditions he meets and whose \
         rating he reaches. The Save editor's tier buttons follow this order.",
    );
    let medal_tiers = medal_tier_ids(ruleset);
    tiers_table(ui, ruleset, &medal_tiers, id_edit);
    ui.separator();
    medal_systems(ui, ruleset, &medal_tiers);
}

/// The tier ids the medal systems count, as a set (the draft's
/// `medalTiers`): a tier one names takes its count from the systems.
fn medal_tier_ids(ruleset: &Ruleset) -> BTreeSet<TierId> {
    ruleset
        .medal_systems
        .iter()
        .flat_map(|system| system.counts.keys().cloned())
        .collect()
}

/// The tiers' `(id, label)` pairs in the ruleset's order (the combo
/// and the medal header show the labels, the systems key on the ids).
fn tier_ids(ruleset: &Ruleset) -> Vec<(TierId, String)> {
    ruleset
        .tiers
        .iter()
        .map(|tier| (tier.id.clone(), tier.label.clone()))
        .collect()
}

/// The tiers table (an `egui::Grid`): the header row, one row per
/// tier plus its expander row, and "+ Add tier". A drop or a removal
/// lands after the loop so the rows keep their indices while drawn.
fn tiers_table(
    ui: &mut egui::Ui,
    ruleset: &mut Ruleset,
    medal_tiers: &BTreeSet<TierId>,
    id_edit: &mut Option<(usize, String)>,
) {
    let mut drop = None;
    let mut remove = None;
    egui::Grid::new("tiers-table")
        .num_columns(COLUMNS)
        .show(ui, |ui| {
            for head in [
                "",
                "Label",
                "Id",
                "Count",
                "Counts as",
                "Rate",
                "Form",
                "Injury res.",
                "Weak foot",
                "Paid cards",
                "Min. cards",
                "A positions",
                "Name colour",
                "Warn band",
                "Stats ≤ targets",
                "",
            ] {
                ui.strong(head);
            }
            ui.end_row();
            for index in 0..ruleset.tiers.len() {
                tier_row(
                    ui,
                    ruleset,
                    index,
                    medal_tiers,
                    &mut drop,
                    &mut remove,
                    id_edit,
                );
                expander_row(ui, ruleset, index);
            }
        });
    // The draft's `reorderable` drop: the dragged tier takes the drop
    // row's place.
    if let Some((from, to)) = drop
        && from != to
    {
        let moved = ruleset.tiers.remove(from);
        ruleset.tiers.insert(to, moved);
    }
    if let Some(index) = remove {
        drop_tier(ruleset, index);
    }
    if ui.button("+ Add tier").clicked() {
        ruleset.tiers.push(new_tier(ruleset.tiers.len() + 1));
    }
}

/// One tier's row: the drag handle, the fields of the draft's columns
/// and the ✕ (while a second tier remains), every cell a drop target
/// for the reorder (the draft's `reorderable`).
fn tier_row(
    ui: &mut egui::Ui,
    ruleset: &mut Ruleset,
    index: usize,
    medal_tiers: &BTreeSet<TierId>,
    drop: &mut Option<(usize, usize)>,
    remove: &mut Option<usize>,
    id_edit: &mut Option<(usize, String)>,
) {
    // The handle is the drag source (the draft's `handle`).
    cell(ui, index, drop, |ui| {
        ui.dnd_drag_source(ui.make_persistent_id(("tier-handle", index)), index, |ui| {
            ui.label("⠇").on_hover_text("Drag to reorder")
        })
        .response
    });

    cell(ui, index, drop, |ui| {
        widgets::text(ui, &mut ruleset.tiers[index].label, 100.0)
    });

    // The id field edits a buffer that commits on Enter or focus loss
    // (a path row's edit without Browse): the commit carries the
    // tier's key in the medal systems (`rename_tier_id`), a keystroke
    // at a time it would carry through another tier's id. Unfocused,
    // the buffer follows the tier's id. The buffer is `id_edit`, the
    // view's, not egui's memory: Save commits it and the unsaved test
    // sees it.
    let mut id = id_edit
        .as_ref()
        .filter(|(row, _)| *row == index)
        .map_or_else(|| ruleset.tiers[index].id.clone(), |(_, text)| text.clone());
    let response = cell(ui, index, drop, |ui| {
        ui.add(text_box(
            egui::TextEdit::singleline(&mut id)
                .desired_width(80.0)
                .font(egui::TextStyle::Monospace),
        ))
        .on_hover_text("What conditions and other tiers refer to")
    });
    if response.lost_focus() && id != ruleset.tiers[index].id {
        rename_tier_id(ruleset, index, id.clone());
    }
    // The buffer follows the tier's id only while the field lacks
    // focus in egui's memory: `Response::has_focus` is also false
    // while the window is unfocused, so resetting on it would drop a
    // half-typed id on an Alt+Tab.
    if ui.ctx().memory(|memory| memory.has_focus(response.id)) {
        // A click on an earlier row's field takes the keyboard on the
        // press, before the typed row is drawn to see its focus loss:
        // its text commits here, or the slot's overwrite would drop it.
        if let Some((row, text)) = id_edit.take_if(|(row, _)| *row != index)
            && ruleset.tiers.get(row).is_some_and(|tier| tier.id != text)
        {
            rename_tier_id(ruleset, row, text);
        }
        *id_edit = Some((index, id));
    } else if id_edit.as_ref().is_some_and(|(row, _)| *row == index) {
        *id_edit = None;
    }

    let medal_counted = medal_tiers.contains(&ruleset.tiers[index].id);
    cell(ui, index, drop, |ui| {
        ui.horizontal(|ui| count_cell(ui, ruleset, index, medal_counted))
            .response
    });

    cell(ui, index, drop, |ui| counts_as_cell(ui, ruleset, index));

    cell(ui, index, drop, |ui| {
        widgets::number(ui, &mut ruleset.tiers[index].rate, 0..=99)
    });
    cell(ui, index, drop, |ui| {
        widgets::number(ui, &mut ruleset.tiers[index].form, 0..=8)
    });
    cell(ui, index, drop, |ui| {
        widgets::number(ui, &mut ruleset.tiers[index].injury_resistance, 0..=3)
    });
    cell(ui, index, drop, |ui| {
        ui.horizontal(|ui| {
            widgets::number(ui, &mut ruleset.tiers[index].weak_foot.usage, 0..=4)
                .on_hover_text("Usage");
            widgets::number(ui, &mut ruleset.tiers[index].weak_foot.accuracy, 0..=4)
                .on_hover_text("Accuracy");
        })
        .response
    });
    cell(ui, index, drop, |ui| {
        widgets::number(ui, &mut ruleset.tiers[index].cards, 0..=48)
    });
    cell(ui, index, drop, |ui| {
        widgets::number(ui, &mut ruleset.tiers[index].min_cards, 0..=48)
    });
    cell(ui, index, drop, |ui| {
        widgets::number(ui, &mut ruleset.tiers[index].a_positions, 0..=13)
    });

    cell(ui, index, drop, |ui| {
        ui.horizontal(|ui| name_color_cell(ui, ruleset, index))
            .response
    });

    cell(ui, index, drop, |ui| {
        widgets::number(ui, &mut ruleset.tiers[index].stat_warning, 0..=99).on_hover_text(
            "Points over the target that warn rather than err, for the ability stats that may sit below \
             their target (0: every point errs)",
        )
    });

    cell(ui, index, drop, |ui| {
        checkbox(ui, &mut ruleset.tiers[index].total_within_targets, "").on_hover_text(
            "The ability stats that may sit below their target add up to at most their targets, an \
             error past them",
        )
    });

    if ruleset.tiers.len() > 1 {
        let response = cell(ui, index, drop, |ui| {
            ui.add(egui::Button::new("✕").frame_when_inactive(false))
                .on_hover_text("Remove")
        });
        if response.clicked() {
            *remove = Some(index);
        }
    }
    ui.end_row();
}

/// A cell of a tier row drawn and made a drop target for the reorder:
/// a drop on any cell lands the dragged tier on this row (the draft's
/// `reorderable`). The response comes back for the cells that act on
/// a click or a change.
fn cell(
    ui: &mut egui::Ui,
    index: usize,
    drop: &mut Option<(usize, usize)>,
    draw: impl FnOnce(&mut egui::Ui) -> egui::Response,
) -> egui::Response {
    let response = draw(ui);
    if let Some(from) = response.dnd_release_payload::<usize>() {
        *drop = Some((*from, index));
    }
    response
}

/// The row's count cell (the draft's): "by medal system" while a
/// system counts the tier, "—" under `counts_as`, else the number or
/// "the rest" with the rest checkbox.
fn count_cell(ui: &mut egui::Ui, ruleset: &mut Ruleset, index: usize, medal_counted: bool) {
    let tier = &mut ruleset.tiers[index];
    if medal_counted {
        ui.weak("by medal system");
        return;
    }
    if tier.counts_as.is_some() {
        widgets::dash(ui);
        return;
    }
    match &mut tier.count {
        Some(count) => {
            widgets::number(ui, count, 0..=32);
        }
        None => {
            ui.weak("the rest");
        }
    }
    let mut rest = tier.count.is_none();
    if checkbox(ui, &mut rest, "rest")
        .on_hover_text("The rest of the squad: one tier at most")
        .changed()
    {
        tier.count = if rest { None } else { Some(1) };
    }
}

/// The row's `counts_as` combo (the draft's): "—" for none, else one
/// of the other tiers by its label; setting one clears the tier's
/// `count`.
fn counts_as_cell(ui: &mut egui::Ui, ruleset: &mut Ruleset, index: usize) -> egui::Response {
    let ids = tier_ids(ruleset);
    let own_id = ruleset.tiers[index].id.clone();
    let mut choice = ruleset.tiers[index].counts_as.clone();
    let selected = choice
        .as_ref()
        .map_or_else(|| "—".to_string(), |id| label_of(&ids, id));
    let others = || ids.iter().filter(|(id, _)| id != &own_id);
    let response = egui::ComboBox::from_id_salt(("counts_as", index))
        .selected_text(selected)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut choice, None, "—");
            for (id, label) in others() {
                ui.selectable_value(&mut choice, Some(id.clone()), label);
            }
        })
        .response
        .on_hover_text("The tier whose count this tier's players fill");
    let items = std::iter::once(None).chain(others().map(|(id, _)| Some(id.clone())));
    if let Some(next) = step_choice(items, &choice, wheel_steps(ui, &response)) {
        choice = next;
    }
    let tier = &mut ruleset.tiers[index];
    if choice != tier.counts_as {
        tier.counts_as = choice;
        if tier.counts_as.is_some() {
            tier.count = None;
        }
    }
    response
}

/// The label `ids` gives `id` (the draft's combo shows labels), `id`
/// itself when no tier carries it.
fn label_of(ids: &[(TierId, String)], id: &TierId) -> String {
    ids.iter()
        .find(|(other, _)| other == id)
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| id.clone())
}

/// The row's name-colour cell (the draft's `colour`): the checkbox,
/// then the suite's colour field and the eight-byte hex field, the
/// alpha kept as the code's last two digits (`ff` for a new one);
/// unticked, weak "none".
fn name_color_cell(ui: &mut egui::Ui, ruleset: &mut Ruleset, index: usize) {
    let tier = &mut ruleset.tiers[index];
    let mut on = tier.name_color.is_some();
    if checkbox(ui, &mut on, "")
        .on_hover_text("This tier's names carry a colour code")
        .changed()
    {
        tier.name_color = on.then(|| "cc9900ff".to_string());
    }
    let Some(code) = tier.name_color.clone() else {
        ui.weak("none");
        return;
    };
    let mut rgb = [
        hex_channel(&code, 0..2),
        hex_channel(&code, 2..4),
        hex_channel(&code, 4..6),
    ];
    let output = color_tools::picker::color_field(ui, ("tier name colour", index), &mut rgb, &[]);
    output
        .response
        .on_hover_text("Colour (the code's last two digits are the alpha)");
    if output.changed {
        tier.name_color = Some(code_with_alpha(rgb, tier.name_color.as_deref()));
    }
    if let Some(code) = &mut tier.name_color {
        ui.add(text_box(
            egui::TextEdit::singleline(code)
                .desired_width(76.0)
                .font(egui::TextStyle::Monospace),
        ))
        .on_hover_text("The eight bytes after \\x11c in the name");
    }
}

/// `code`'s two hex digits at `range` as a channel (0 where a code
/// typed mid-edit is short or not hex).
fn hex_channel(code: &str, range: std::ops::Range<usize>) -> u8 {
    code.get(range)
        .and_then(|digits| u8::from_str_radix(digits, 16).ok())
        .unwrap_or(0)
}

/// `rgb` as an eight-byte code keeping `current`'s alpha (its last two
/// digits, `ff` for a new one — the draft's `colour`).
fn code_with_alpha(rgb: [u8; 3], current: Option<&str>) -> String {
    let alpha = current.and_then(|code| code.get(6..8)).unwrap_or("ff");
    format!("{:02x}{:02x}{:02x}{alpha}", rgb[0], rgb[1], rgb[2])
}

/// `index`'s tier renamed to `id`, its key carried into every medal
/// system's counts — but only while the carry is unambiguous: another
/// tier still carrying `old` (a shared id moves the wrong tier's
/// counts) or a system already holding `id` (a key never overwrites
/// one) renames the tier alone.
pub(crate) fn rename_tier_id(ruleset: &mut Ruleset, index: usize, id: TierId) {
    let old = std::mem::replace(&mut ruleset.tiers[index].id, id.clone());
    let shared = ruleset.tiers.iter().any(|tier| tier.id == old);
    let held = ruleset
        .medal_systems
        .iter()
        .any(|system| system.counts.contains_key(&id));
    if shared || held {
        return;
    }
    for system in &mut ruleset.medal_systems {
        if let Some(count) = system.counts.remove(&old) {
            system.counts.insert(id.clone(), count);
        }
    }
}

/// `index`'s tier removed; its id drops out of every medal system's
/// counts only while no remaining tier carries it (the counts of a
/// surviving tier are not the removed one's to take).
fn drop_tier(ruleset: &mut Ruleset, index: usize) {
    let id = ruleset.tiers.remove(index).id;
    if !ruleset.tiers.iter().any(|tier| tier.id == id) {
        for system in &mut ruleset.medal_systems {
            system.counts.remove(&id);
        }
    }
}

/// The tier "+ Add tier" adds (the draft's `tier()`): `tier<N>`,
/// "Tier <N>", count 1, rate 80.
fn new_tier(n: usize) -> Tier {
    crate::document::tier(&format!("tier{n}"), &format!("Tier {n}"), Some(1), 80)
}

/// The row's expander (the draft's `colspan` row under the data row):
/// `egui::Grid` has no colspan, so the expander is emitted in the
/// row's last column — the only column whose available width runs to
/// the parent's right (`grid.rs` `available_rect`, `is_last_column`) —
/// inside a child ui whose rect starts back at the table's left edge.
/// `advance` records a cell's widget rect as its column's minimum
/// width (`set_min_col_width`), so the child's table-wide `min_rect`
/// cannot be what lands: the cell is allocated at the column's
/// minimum width and the child's drawn height.
fn expander_row(ui: &mut egui::Ui, ruleset: &mut Ruleset, index: usize) {
    for _ in 0..COLUMNS - 1 {
        ui.label("");
    }
    let top_left = ui.cursor().min;
    let rect = egui::Rect::from_min_max(
        egui::pos2(
            ui.max_rect().min.x + ui.spacing().interact_size.x,
            top_left.y,
        ),
        ui.max_rect().right_bottom(),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    collapsing_header("Stat targets, free cards and conditions")
        .id_salt(("tier-expander", index))
        .show(&mut child, |ui| expander(ui, ruleset, index));
    ui.advance_cursor_after_rect(egui::Rect::from_min_size(
        top_left,
        egui::vec2(ui.spacing().interact_size.x, child.min_rect().height()),
    ));
    ui.end_row();
}

/// The expander's body (the draft's `collapsing` content): the
/// per-stat targets, the free cards and the conditions' heading.
fn expander(ui: &mut egui::Ui, ruleset: &mut Ruleset, index: usize) {
    ui.horizontal(|ui| {
        ui.strong("Per-stat targets");
        ui.weak(" (unticked: the rate)");
    });
    stat_targets(ui, ruleset, index);
    ui.separator();
    ui.strong("Free cards");
    free_cards(ui, ruleset, index);
    ui.separator();
    ui.horizontal(|ui| {
        ui.strong("Conditions");
        ui.weak(" (what a player must also meet to be this tier)");
    });
    ui.weak("The condition editor comes in a later build.");
}

/// The per-stat targets, five columns as the draft's `perStatGrid`:
/// each ability stat's "own target" checkbox and its number while it
/// has one. Unticked, the stat follows the rate and has no `stats` entry.
/// A checkbox, not an empty number field, because egui's number field
/// cannot be empty, and 0 cannot stand for "the rate": a ruleset may set
/// a target of 0.
fn stat_targets(ui: &mut egui::Ui, ruleset: &mut Ruleset, index: usize) {
    egui::Grid::new(("stat-targets", index))
        .num_columns(5)
        .show(ui, |ui| {
            for (column, name) in ABILITY_STATS.iter().enumerate() {
                let stat = Stat::try_from((*name).to_string()).expect("an ability stat");
                ui.horizontal(|ui| {
                    let tier = &mut ruleset.tiers[index];
                    let mut own = tier.stats.contains_key(&stat);
                    if checkbox(ui, &mut own, human(name)).changed() {
                        if own {
                            tier.stats.insert(stat, tier.rate);
                        } else {
                            tier.stats.remove(&stat);
                        }
                    }
                    if let Some(target) = tier.stats.get_mut(&stat) {
                        widgets::number(ui, target, 0..=99);
                    }
                });
                if column % 5 == 4 {
                    ui.end_row();
                }
            }
        });
}

/// The free-card counts, "COM styles" first then each card group by
/// its words (the draft's `row` of group fields): a count of 0 removes
/// the group's `free` entry.
fn free_cards(ui: &mut egui::Ui, ruleset: &mut Ruleset, index: usize) {
    let groups: Vec<String> = std::iter::once("com".to_string())
        .chain(ruleset.card_groups.keys().cloned())
        .collect();
    ui.horizontal_wrapped(|ui| {
        for group in &groups {
            ui.horizontal(|ui| {
                if group == "com" {
                    ui.label("COM styles");
                } else {
                    ui.label(human(group));
                }
                map_number(ui, &mut ruleset.tiers[index].free, group, 20, false);
            });
        }
    });
}

/// The medal systems under the table (the draft's): the header row
/// holds one checkbox per tier, ticking puts the tier in every system
/// at 0 and clears its count, clearing removes it from every system;
/// one row per system (its label field, ✕ and a count per counted
/// tier); "+ Add medal system".
fn medal_systems(ui: &mut egui::Ui, ruleset: &mut Ruleset, medal_tiers: &BTreeSet<TierId>) {
    ui.strong("Medal systems");
    ui.weak(
        "Alternative medal counts: the team's counts of the ticked tiers must equal one \
         system's. Unticked tiers keep their own count, one of them the rest of the squad.",
    );
    let tiers = tier_ids(ruleset);
    let has_systems = !ruleset.medal_systems.is_empty();
    let mut remove = None;
    egui::Grid::new("medal-systems").show(ui, |ui| {
        ui.label("");
        for (index, (id, label)) in tiers.iter().enumerate() {
            let mut counted = medal_tiers.contains(id);
            let tip = if has_systems {
                "The medal systems count this tier"
            } else {
                "Add a medal system first"
            };
            let cell = ui.horizontal(|ui| {
                add_enabled(ui, has_systems, |ui: &mut egui::Ui| {
                    checkbox(ui, &mut counted, label)
                })
                .changed()
            });
            cell.response.on_hover_text(tip);
            let toggled = cell.inner;
            if toggled {
                set_counted(ruleset, index, counted);
            }
        }
        ui.end_row();
        for row in 0..ruleset.medal_systems.len() {
            ui.horizontal(|ui| {
                widgets::text(ui, &mut ruleset.medal_systems[row].label, 90.0);
                if ui
                    .add(egui::Button::new("✕").frame_when_inactive(false))
                    .on_hover_text("Remove")
                    .clicked()
                {
                    remove = Some(row);
                }
            });
            for (id, _) in &tiers {
                if medal_tiers.contains(id) {
                    map_number(ui, &mut ruleset.medal_systems[row].counts, id, 32, true);
                } else {
                    widgets::dash(ui);
                }
            }
            ui.end_row();
        }
    });
    if let Some(row) = remove {
        ruleset.medal_systems.remove(row);
    }
    if ui.button("+ Add medal system").clicked() {
        ruleset.medal_systems.push(MedalSystem {
            label: format!("System {}", ruleset.medal_systems.len() + 1),
            counts: BTreeMap::new(),
        });
    }
}

/// `index`'s tier put into every medal system at 0 (`counted`, its
/// `count` cleared) or taken out of them all (the draft's header
/// checkbox).
fn set_counted(ruleset: &mut Ruleset, index: usize, counted: bool) {
    if counted {
        for system in &mut ruleset.medal_systems {
            system
                .counts
                .entry(ruleset.tiers[index].id.clone())
                .or_insert(0);
        }
        ruleset.tiers[index].count = None;
    } else {
        for system in &mut ruleset.medal_systems {
            system.counts.remove(&ruleset.tiers[index].id);
        }
    }
}

/// `map[key]` as a `DragValue` whose edits stay in 0..=`max`, the
/// stored value drawn as it is (`widgets::number`'s rule), absent read
/// as 0; a written value is kept, 0 keeping its entry (`keep_zero`) or
/// removing it (a free-cards count of 0 is no entry).
fn map_number(
    ui: &mut egui::Ui,
    map: &mut BTreeMap<String, u8>,
    key: &str,
    max: u8,
    keep_zero: bool,
) -> egui::Response {
    let mut value = map.get(key).copied().unwrap_or(0);
    let mut response = ui.add(
        egui::DragValue::new(&mut value)
            .range(0..=max)
            .clamp_existing_to_range(false),
    );
    if step_number(&mut value, wheel_steps(ui, &response), 0..=max) {
        response.mark_changed();
    }
    if response.changed() {
        if value == 0 && !keep_zero {
            map.remove(key);
        } else {
            map.insert(key.to_string(), value);
        }
    }
    response
}
