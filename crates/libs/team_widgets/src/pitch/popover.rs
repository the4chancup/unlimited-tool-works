//! The slot popover, the draft's `slotPopover`: a marker click's per-slot
//! panel — the starter's name and roster number, the Position combo
//! beside Make GK and the X and Y fields.

use pes_savefile::model::tactics::TeamTactics;
use studio_core::widgets::{add_enabled, add_enabled_ui, step_choice, step_number, wheel_steps};

use super::{GRID_X, GRID_Y, OUTFIELD_Y_MIN, drop_rule, overalls_of, position_label};
use crate::names::append_name;
use crate::tactics::{RosterRow, TacticsView, raw_name};

/// The slot popover (the draft's `slotPopover`): its top-right corner on
/// the marker's bottom-left corner as the marker stood at the click
/// (`OpenSlot::at`, kept relative to `canvas` so it scrolls with the
/// pitch — it stays while the slot's X and Y move him, so the fields'
/// arrows stay under the pointer), and a press anywhere outside
/// it closes it, as Close does. The starter's name and `player
/// <n>` head it (`(empty roster slot)` an empty roster slot), under it
/// the unlabeled Position combo of the positions his spot offers
/// (`drop_rule::spot_choices`, as the game's Select Position menu lists
/// them; his own first when the spot offers neither) with Make GK beside
/// it at its own width, the two filling the popover's width, and the X
/// (0 to 104) and Y (4 to 48, `OUTFIELD_Y_MIN` up) steppers (`stepper`)
/// and Close. A change of X or Y, typed or stepped, sets his position by
/// the drop rule at the new spot, as a release there does. The keeper's slot shows a
/// disabled "GK", the two steppers disabled under "The goalkeeper's spot
/// is fixed" and Make GK disabled. Make GK exchanges the slot's
/// `starting_eleven` entry with the keeper slot's — the formation slots
/// stay — and closes the popover, as does Close; `pitch` also closes it
/// when the preset, the formation or the team changes.
pub(super) fn slot_popover(
    ui: &mut egui::Ui,
    canvas: egui::Rect,
    tactics: &mut TeamTactics,
    view: &mut TacticsView,
    roster: &[RosterRow<'_>],
) {
    let Some(open) = view.popover else {
        return;
    };
    // A press anywhere but inside the popover closes it (the draft's
    // document `pointerdown` listener, which fires on any button): a
    // press inside it — or inside a popup it opened, like the Position
    // combo's list — lands on a Foreground layer, everything else is
    // outside.
    let pressed_at = ui.input(|input| {
        input.events.iter().find_map(|event| match event {
            egui::Event::PointerButton {
                pos, pressed: true, ..
            } => Some(*pos),
            _ => None,
        })
    });
    if pressed_at.is_some_and(|pos| {
        ui.ctx()
            .layer_id_at(pos)
            .is_none_or(|layer| layer.order < egui::Order::Foreground)
    }) {
        view.popover = None;
        // The popover's widgets are gone this frame — a keyboard gesture
        // inside it (a DragValue's typed edit) cannot end on its own, so
        // the outside press ends it here as the focused widget would have
        // under `SurrenderFocusOn::Presses` had it still been drawn. A
        // press on the focused widget itself is not elsewhere: it keeps
        // the keyboard.
        if let Some(focused) = ui.ctx().memory(|memory| memory.focused())
            && !pressed_at.is_some_and(|pos| {
                ui.ctx()
                    .read_response(focused)
                    .is_some_and(|response| response.interact_rect.contains(pos))
            })
        {
            ui.ctx()
                .memory_mut(|memory| memory.surrender_focus(focused));
        }
        return;
    }
    let index = open.slot;
    let overalls = overalls_of(roster, tactics, index);
    let TeamTactics {
        presets,
        starting_eleven,
        ..
    } = tactics;
    let formation = &mut presets[open.preset.min(2)].formations[open.formation.min(2)];
    let slot = formation.players[index];
    let keeper = slot.position == 0;
    // `pitch` has closed it unless `open.scope` is this widget's scope,
    // so the id is keyed to this pitch.
    egui::Popup::new(
        open.area_id(),
        ui.ctx().clone(),
        egui::PopupAnchor::Position(canvas.min + open.at),
        ui.layer_id(),
    )
    // The popover's top-right corner on the point (the draft's
    // `.tpopover` `translateX(-100%)`).
    .align(egui::RectAlign::outside(egui::Align2::LEFT_BOTTOM))
    .show(|ui| {
        ui.horizontal(|ui| {
            // The name's colour codes, plain runs kept strong.
            let mut name = egui::text::LayoutJob::default();
            append_name(
                &mut name,
                raw_name(roster, starting_eleven[index]).unwrap_or("(empty roster slot)"),
                &egui::TextStyle::Body.resolve(ui.style()),
            );
            for section in &mut name.sections {
                if section.format.color == egui::Color32::PLACEHOLDER {
                    section.format.color = ui.visuals().strong_text_color();
                }
            }
            ui.label(name);
            ui.weak(format!(
                "player {}",
                usize::from(starting_eleven[index]) + 1
            ));
        });
        // The draft's `.pos-row`: no label — the combo's abbreviations
        // say what it holds ("Position" is its hover and its accessible
        // name, as the draft's `title`) — it takes the row's width less
        // Make GK's, which keeps its own, the two spanning the popover.
        let goal = formation.players.iter().position(|slot| slot.position == 0);
        egui::Sides::new().shrink_left().show(
            ui,
            |ui| {
                // `available_width` here is the row less Make GK's
                // width and the gap — the popover's content width,
                // which its other rows set. It is unbounded on the
                // popover's sizing pass, so the combo takes 0 then and
                // the row stays its narrowest (the draft's
                // `.pos-row select { width: 0 }`).
                let width = if ui.is_sizing_pass() {
                    0.0
                } else {
                    ui.available_width()
                };
                if keeper {
                    add_enabled_ui(ui, false, |ui| {
                        egui::ComboBox::from_id_salt("slot position")
                            .width(width)
                            .selected_text("GK")
                            .show_ui(ui, |_ui| {})
                            .response
                            .on_hover_text("Position")
                            .on_disabled_hover_text("Position");
                    });
                } else {
                    let offered = drop_rule::spot_choices(formation, index, slot.x, slot.y);
                    let own = (!offered.contains(&slot.position)).then_some(slot.position);
                    let choices: Vec<u8> = own.into_iter().chain(offered).collect();
                    let mut position = slot.position;
                    let combo = egui::ComboBox::from_id_salt("slot position")
                        .width(width)
                        .selected_text(position_label(slot.position))
                        .show_ui(ui, |ui| {
                            for &value in &choices {
                                ui.selectable_value(&mut position, value, position_label(value));
                            }
                        })
                        .response
                        .on_hover_text("Position");
                    if let Some(next) = step_choice(choices, &position, wheel_steps(ui, &combo)) {
                        position = next;
                    }
                    if position != slot.position {
                        formation.players[index].position = position;
                    }
                }
            },
            |ui| {
                if add_enabled(ui, !keeper, egui::Button::new("Make GK"))
                    .on_hover_text("Swaps him into goal; the keeper takes his slot")
                    .clicked()
                {
                    if let Some(goal) = goal
                        && goal != index
                    {
                        starting_eleven.swap(index, goal);
                    }
                    view.popover = None;
                }
            },
        );
        ui.horizontal(|ui| {
            // A step needs no `mark_changed`: the move below compares the
            // spot with the slot's.
            ui.label("X");
            stepper(ui, &mut formation.players[index].x, 0, GRID_X, !keeper);
            ui.label("Y");
            stepper(
                ui,
                &mut formation.players[index].y,
                OUTFIELD_Y_MIN,
                GRID_Y,
                !keeper,
            );
            if ui.button("Close").clicked() {
                view.popover = None;
            }
        });
        // A Y typed into another band changes his position with it.
        let moved = formation.players[index];
        if (moved.x, moved.y) != (slot.x, slot.y) {
            formation.players[index].position =
                drop_rule::dropped_position(formation, index, moved.x, moved.y, overalls);
        }
    });
}

/// The hover of the keeper's X and Y fields and their arrows.
const SPOT_FIXED: &str = "The goalkeeper's spot is fixed";

/// A stepper over `value` in `min..=max` (the draft's `.spin`): `⏴`, the
/// number and `⏵` joined into one box as wide as the number box alone
/// was, so the popover is no wider. An arrow's click steps the value by
/// one, clamped into the range, so a stored value past either end steps
/// back to it; each arrow is disabled at its end of the range. The
/// number keeps the wheel's step and does not clamp a stored value on
/// draw. `enabled` false (the keeper's spot) disables all three under
/// `SPOT_FIXED`.
fn stepper(ui: &mut egui::Ui, value: &mut u8, min: u8, max: u8, enabled: bool) {
    // The draft's widths, 52 points in all: `104` fits the number's 28.
    // An arrow's cell is 12, but the glyph's advance at the button font is
    // 12.5, which widens the button by half a point, its glyph centred:
    // within the one pixel a match allows.
    const ARROW_WIDTH: f32 = 12.0;
    const NUMBER_WIDTH: f32 = 28.0;
    let round = ui.visuals().widgets.inactive.corner_radius;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        // The padding would widen an arrow past its 12 points, and the
        // number past its 28 with `104`.
        ui.spacing_mut().button_padding.x = 0.0;
        let arrow = |ui: &mut egui::Ui, glyph: &str, corner_radius, can_step: bool| {
            let response = cell(ui, ARROW_WIDTH, |ui| {
                add_enabled(
                    ui,
                    enabled && can_step,
                    // Clicks only, as a system spinner's arrows: Tab goes
                    // from X to Y, and the focused number steps with the
                    // arrow keys.
                    egui::Button::new(glyph)
                        .corner_radius(corner_radius)
                        .sense(egui::Sense::CLICK),
                )
            });
            // An arrow disabled at its end of the range is no fixed spot.
            if enabled {
                response
            } else {
                response.on_disabled_hover_text(SPOT_FIXED)
            }
        };
        let left = egui::CornerRadius {
            ne: 0,
            se: 0,
            ..round
        };
        if arrow(ui, "⏴", left, *value > min).clicked() {
            step_number(value, -1, min..=max);
        }
        let number = cell(ui, NUMBER_WIDTH, |ui| {
            // Square, so the arrows join it into one box.
            let widgets = &mut ui.visuals_mut().widgets;
            for visuals in [
                &mut widgets.noninteractive,
                &mut widgets.inactive,
                &mut widgets.hovered,
                &mut widgets.active,
                &mut widgets.open,
            ] {
                visuals.corner_radius = egui::CornerRadius::ZERO;
            }
            let size = ui.available_size();
            add_enabled(
                ui,
                enabled,
                egui::DragValue::new(&mut *value)
                    .range(min..=max)
                    .clamp_existing_to_range(false)
                    .min_size(size),
            )
            .on_disabled_hover_text(SPOT_FIXED)
        });
        step_number(value, wheel_steps(ui, &number), min..=max);
        let right = egui::CornerRadius {
            nw: 0,
            sw: 0,
            ..round
        };
        if arrow(ui, "⏵", right, *value < max).clicked() {
            step_number(value, 1, min..=max);
        }
    });
}

/// `add` in a cell `width` points wide and a widget's row high, its widget
/// filling the cell with its content centred.
fn cell<R>(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.allocate_ui_with_layout(
        egui::vec2(width, ui.spacing().interact_size.y),
        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
        add,
    )
    .inner
}

#[cfg(test)]
mod tests {
    use egui::accesskit::{self, Role};
    use pes_version::PesVersion;

    use super::*;
    use crate::pitch::tests::{marker_point, party_hard};
    use crate::test_support::{
        Headless, centre, control_right_of, controls_right_of, entry, fixture_rows,
        fixture_tactics, roster_rows,
    };

    // The draft's `slotPopover`: a click opens the player's slot details —
    // name and roster number, the outfield Position combo, Make GK, the X
    // and Y fields and Close — and Make GK exchanges his `starting_eleven`
    // entry with the keeper slot's, the formation slots untouched, closing
    // the popover.
    #[test]
    fn a_click_opens_the_popover_and_make_gk_exchanges_the_starters() {
        let mut window = Headless::sized(600.0, 450.0);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        let mut view = TacticsView::default();
        let rows = fixture_rows(PesVersion::Pes21);
        let roster = roster_rows(&rows);
        let formation = tactics.presets[0].formations[0].clone();
        let keeper = formation
            .players
            .iter()
            .position(|slot| slot.position == 0)
            .expect("the fixture has a keeper slot");
        let index = 4;
        assert_ne!(index, keeper, "premise: not the keeper's slot");
        // A non-identity eleven: an arm writing the two indices into the
        // eleven would look the same as an exchange on the fixture's
        // [0, 1, 2, ...].
        tactics.starting_eleven.swap(index, 9);
        tactics.starting_eleven.swap(keeper, 7);
        assert_ne!(
            tactics.starting_eleven[index], index as u8,
            "premise: his slot and index differ"
        );
        assert_ne!(
            tactics.starting_eleven[keeper], keeper as u8,
            "premise: the keeper's slot and index differ"
        );
        let before = tactics.clone();

        let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        let at = marker_point(&nodes, &formation.players, index);
        let (_, changed) =
            window.click_pitch(at, PesVersion::Pes21, &mut tactics, &mut view, &roster);
        assert!(!changed, "opening the popover writes no record");
        let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        assert_eq!(
            view.popover.map(|open| open.slot),
            Some(index),
            "the popover is the clicked slot's"
        );
        entry(&nodes, Role::Button, "Make GK");
        entry(&nodes, Role::Button, "Close");
        assert_eq!(
            entry(&nodes, Role::ComboBox, "Position").value(),
            Some(position_label(formation.players[index].position)),
            "the position combo"
        );

        let make_gk = entry(&nodes, Role::Button, "Make GK");
        let (_, changed) = window.click_pitch(
            centre(make_gk),
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
        );
        assert!(changed, "Make GK is an edit");
        assert_eq!(
            tactics.starting_eleven[index], before.starting_eleven[keeper],
            "he takes the goal"
        );
        assert_eq!(
            tactics.starting_eleven[keeper], before.starting_eleven[index],
            "the keeper takes his slot"
        );
        assert_eq!(tactics.presets, before.presets, "the formation slots stay");
        let mut expect = before;
        expect.starting_eleven.swap(index, keeper);
        assert_eq!(tactics, expect, "the two entries exchanged, nothing else");
        assert_eq!(view.popover, None, "and the popover closes");
    }

    // The popover's fields write the stored slot: X typed to 55 lands on
    // the slot's x (0 to 104), leaving y alone.
    #[test]
    fn an_x_typed_in_the_popover_writes_x() {
        let mut window = Headless::sized(600.0, 450.0);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        let mut view = TacticsView::default();
        let rows = fixture_rows(PesVersion::Pes21);
        let roster = roster_rows(&rows);
        let index = 4;
        let formation = tactics.presets[0].formations[0].clone();
        assert_ne!(formation.players[index].x, 55, "premise: a real change");
        let y = formation.players[index].y;

        let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        let at = marker_point(&nodes, &formation.players, index);
        window.click_pitch(at, PesVersion::Pes21, &mut tactics, &mut view, &roster);
        window.type_in_spin_field(
            "X",
            "55",
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
        );
        let slot = tactics.presets[0].formations[0].players[index];
        assert_eq!(slot.x, 55, "the typed x is written");
        assert_eq!(slot.y, y, "y stays");
    }

    // A stored coordinate past the grid (a hand-edited or corrupt save)
    // shows as stored in the popover's fields and is not rewritten by
    // drawing them: the frame reports no change and x stays 200.
    #[test]
    fn an_out_of_range_stored_spot_is_not_clamped_on_draw() {
        let mut window = Headless::sized(600.0, 450.0);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        let index = 4;
        tactics.presets[0].formations[0].players[index].x = 200;
        let mut view = TacticsView::default();
        let rows = fixture_rows(PesVersion::Pes21);
        let roster = roster_rows(&rows);

        let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        let at = marker_point(&nodes, &tactics.presets[0].formations[0].players, index);
        let (_, changed) =
            window.click_pitch(at, PesVersion::Pes21, &mut tactics, &mut view, &roster);
        assert!(!changed, "opening the popover writes nothing");
        let (nodes, changed) =
            window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        assert!(!changed, "drawing the popover writes nothing");
        assert_eq!(
            tactics.presets[0].formations[0].players[index].x, 200,
            "the stored x stays"
        );
        let field = controls_right_of(&nodes, "X", Role::SpinButton)
            .into_iter()
            .next()
            .expect("the X field");
        assert_eq!(
            field.numeric_value(),
            Some(200.0),
            "the field shows the stored 200"
        );
        let (_, changed) =
            window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        assert!(!changed, "and an idle frame keeps it");
        assert_eq!(tactics.presets[0].formations[0].players[index].x, 200);
    }

    // The keeper's popover ("The goalkeeper's slot is fixed"): Position a
    // disabled "GK", X and Y disabled under "The goalkeeper's spot is
    // fixed", Make GK disabled.
    #[test]
    fn the_keepers_popover_locks_the_spot() {
        let mut window = Headless::sized(600.0, 450.0);
        window
            .ctx
            .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        let mut view = TacticsView::default();
        let rows = fixture_rows(PesVersion::Pes21);
        let roster = roster_rows(&rows);
        let keeper = tactics.presets[0].formations[0]
            .players
            .iter()
            .position(|slot| slot.position == 0)
            .expect("the fixture has a keeper slot");

        let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        let at = marker_point(&nodes, &tactics.presets[0].formations[0].players, keeper);
        window.click_pitch(at, PesVersion::Pes21, &mut tactics, &mut view, &roster);
        let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        let position = entry(&nodes, Role::ComboBox, "Position");
        assert!(position.is_disabled(), "the keeper's position is fixed");
        assert_eq!(position.value(), Some("GK"), "and shows GK");
        for label in ["X", "Y"] {
            let field = controls_right_of(&nodes, label, Role::SpinButton)
                .into_iter()
                .next()
                .unwrap_or_else(|| panic!("no {label} field"));
            assert!(field.is_disabled(), "the keeper's {label} is disabled");
            assert_eq!(
                window.pitch_tooltip(
                    centre(field),
                    "goalkeeper",
                    PesVersion::Pes21,
                    &mut tactics,
                    &mut view,
                    &roster,
                ),
                Some("The goalkeeper's spot is fixed".to_owned()),
                "the {label} field's hover"
            );
        }
        assert!(
            entry(&nodes, Role::Button, "Make GK").is_disabled(),
            "the keeper's Make GK is disabled"
        );
    }

    // The draft's `.pos-row`: no "Position" label — the combo's
    // abbreviations say what it holds — it takes the row's width less
    // Make GK's, which keeps its own, the two spanning the popover the
    // other rows size. The keeper's disabled "GK" combo lays out the
    // same.
    #[test]
    fn the_position_row_spans_the_popover() {
        let mut window = Headless::sized(600.0, 450.0);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        let mut view = TacticsView::default();
        let rows = fixture_rows(PesVersion::Pes21);
        let roster = roster_rows(&rows);
        let formation = tactics.presets[0].formations[0].clone();
        let keeper = formation
            .players
            .iter()
            .position(|slot| slot.position == 0)
            .expect("the fixture has a keeper slot");
        // A bare Make GK's own width in this window's style.
        let (bare, _) = window.frame(vec![], |ui| {
            ui.add(egui::Button::new("Make GK"));
        });
        let own = entry(&bare, Role::Button, "Make GK")
            .bounds()
            .expect("the bare button's bounds");
        let own_width = own.x1 - own.x0;
        let spacing = f64::from(window.ctx.global_style().spacing.item_spacing.x);

        for index in [4, keeper] {
            view.popover = None;
            let (nodes, _) =
                window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
            let at = marker_point(&nodes, &formation.players, index);
            window.click_pitch(at, PesVersion::Pes21, &mut tactics, &mut view, &roster);
            let (nodes, _) =
                window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
            assert!(
                nodes
                    .iter()
                    .all(|node| { node.role() != Role::Label || node.value() != Some("Position") }),
                "slot {index}: no \"Position\" label"
            );
            let popover = view
                .popover
                .and_then(|open| window.ctx.memory(|memory| memory.area_rect(open.area_id())))
                .expect("the popover is open");
            // The header's name label: the popover's top-left label.
            let name_left = nodes
                .iter()
                .filter(|node| node.role() == Role::Label)
                .filter_map(|node| node.bounds())
                .filter(|bounds| {
                    popover.contains(egui::pos2(
                        ((bounds.x0 + bounds.x1) / 2.0) as f32,
                        ((bounds.y0 + bounds.y1) / 2.0) as f32,
                    ))
                })
                .map(|bounds| (bounds.y0, bounds.x0))
                .min_by(|a, b| a.partial_cmp(b).expect("ordered bounds"))
                .expect("the header's name label")
                .1;
            let combo = entry(&nodes, Role::ComboBox, "Position")
                .bounds()
                .expect("the combo's bounds");
            let make_gk = entry(&nodes, Role::Button, "Make GK")
                .bounds()
                .expect("Make GK's bounds");
            let close = entry(&nodes, Role::Button, "Close")
                .bounds()
                .expect("Close's bounds");
            assert!(
                (combo.x0 - name_left).abs() <= 0.5,
                "slot {index}: the combo at the row's left ({combo:?} vs {name_left})"
            );
            assert!(
                (make_gk.x1 - close.x1).abs() <= 0.5,
                "slot {index}: Make GK at the row's right ({make_gk:?} vs {close:?})"
            );
            assert!(
                (combo.x1 + spacing - make_gk.x0).abs() <= 0.5,
                "slot {index}: the combo fills to the gap ({combo:?} vs {make_gk:?})"
            );
            assert!(
                (make_gk.x1 - make_gk.x0 - own_width).abs() <= 0.5,
                "slot {index}: Make GK keeps its own width ({} vs {own_width})",
                make_gk.x1 - make_gk.x0
            );
        }
    }

    // The popover follows the slot's shown formation: a formation change
    // closes it.
    #[test]
    fn the_popover_closes_when_the_formation_changes() {
        let mut window = Headless::sized(600.0, 450.0);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        // Fluid formation on, so the preset's In-possession can be shown.
        tactics.presets[0].style.fluid = Some(true);
        let mut view = TacticsView::default();
        let rows = fixture_rows(PesVersion::Pes21);
        let roster = roster_rows(&rows);
        let index = 4;

        let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        let at = marker_point(&nodes, &tactics.presets[0].formations[0].players, index);
        window.click_pitch(at, PesVersion::Pes21, &mut tactics, &mut view, &roster);
        let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        entry(&nodes, Role::Button, "Close");

        // A formation switch on the bar (as `view.preset = 1` is a preset
        // switch in `a_preset_switch_closes_the_popover`).
        view.formation = 1;
        window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        assert_eq!(view.formation, 1, "in-possession is on screen");
        assert_eq!(view.popover, None, "and the popover closed");
        let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        assert!(
            nodes.iter().all(|node| node.label() != Some("Close")),
            "no popover is drawn"
        );
    }

    /// The stepper's two arrows right of `label` (`"X"` or `"Y"`): `⏴`,
    /// then `⏵`. A marker under the popover can share the row, so the
    /// other buttons are left out.
    fn arrows<'a>(nodes: &'a [accesskit::Node], label: &str) -> [&'a accesskit::Node; 2] {
        let found: Vec<&accesskit::Node> = controls_right_of(nodes, label, Role::Button)
            .into_iter()
            .filter(|node| matches!(node.label(), Some("⏴" | "⏵")))
            .collect();
        match found.as_slice() {
            [down, up, ..] if down.label() == Some("⏴") && up.label() == Some("⏵") => {
                [down, up]
            }
            found => panic!(
                "no ⏴ ⏵ right of {label:?}: {:?}",
                found.iter().map(|node| node.label()).collect::<Vec<_>>()
            ),
        }
    }

    /// Opens the popover of slot `index` of preset 1's Kick-off with a
    /// click on his marker.
    fn open(
        window: &mut Headless,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
        index: usize,
    ) {
        let players = tactics.presets[0].formations[0].players;
        let (nodes, _) = window.pitch(PesVersion::Pes21, tactics, view, roster, vec![]);
        let at = marker_point(&nodes, &players, index);
        window.click_pitch(at, PesVersion::Pes21, tactics, view, roster);
    }

    /// A click on the open popover's arrow `arrow` (0 for `⏴`, 1 for `⏵`)
    /// right of `label`; answers whether the click's frame changed the
    /// record.
    fn click_arrow(
        window: &mut Headless,
        label: &str,
        arrow: usize,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> bool {
        let (nodes, _) = window.pitch(PesVersion::Pes21, tactics, view, roster, vec![]);
        let at = centre(arrows(&nodes, label)[arrow]);
        window
            .click_pitch(at, PesVersion::Pes21, tactics, view, roster)
            .1
    }

    // A click on X's `⏴` lowers the slot's x by one, a click on Y's `⏵`
    // raises its y by one, each click one edit; nothing else changes.
    #[test]
    fn the_arrows_step_x_down_and_y_up_by_one() {
        let mut window = Headless::sized(600.0, 450.0);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        let mut view = TacticsView::default();
        let rows = fixture_rows(PesVersion::Pes21);
        let roster = roster_rows(&rows);
        let index = 4;
        let before = tactics.clone();
        let slot = before.presets[0].formations[0].players[index];
        assert!(slot.position != 0, "premise: not the keeper's slot");
        assert!(slot.x > 0 && slot.y < GRID_Y, "premise: room to step");

        open(&mut window, &mut tactics, &mut view, &roster, index);
        assert!(
            click_arrow(&mut window, "X", 0, &mut tactics, &mut view, &roster),
            "the click on X's ⏴ is an edit"
        );
        let mut expect = before.clone();
        expect.presets[0].formations[0].players[index].x = slot.x - 1;
        assert_eq!(tactics, expect, "x down by one, alone");

        assert!(
            click_arrow(&mut window, "Y", 1, &mut tactics, &mut view, &roster),
            "the click on Y's ⏵ is an edit"
        );
        expect.presets[0].formations[0].players[index].y = slot.y + 1;
        assert_eq!(tactics, expect, "y up by one, alone");
        assert_eq!(
            view.popover.map(|open| open.slot),
            Some(index),
            "it stays open"
        );
    }

    // The arrows take clicks, never the keyboard's focus: with X's number
    // focused, Tab moves the keyboard straight to Y's number.
    #[test]
    fn tab_from_x_moves_the_keyboard_to_y_past_the_arrows() {
        let mut window = Headless::sized(600.0, 450.0);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        let mut view = TacticsView::default();
        let rows = fixture_rows(PesVersion::Pes21);
        let roster = roster_rows(&rows);
        open(&mut window, &mut tactics, &mut view, &roster, 4);
        let (with_ids, _) = window.frame_at_ids(None, vec![], |ui| {
            crate::pitch::pitch(
                ui,
                "pitch",
                PesVersion::Pes21,
                &mut tactics,
                &mut view,
                &roster,
            );
        });
        let nodes: Vec<accesskit::Node> = with_ids.iter().map(|(_, node)| node.clone()).collect();
        // The number field's id right of `label`, matched by its bounds.
        let field_id = |label: &str| {
            let bounds = control_right_of(&nodes, label, Role::SpinButton).bounds();
            with_ids
                .iter()
                .find(|(_, node)| node.role() == Role::SpinButton && node.bounds() == bounds)
                .map(|(id, _)| *id)
                .unwrap_or_else(|| panic!("no {label} field id"))
        };
        let (x_id, y_id) = (field_id("X"), field_id("Y"));
        let focused = |window: &Headless| {
            window
                .ctx
                .memory(|memory| memory.focused().map(|id| id.accesskit_id()))
        };

        let x_field = control_right_of(&nodes, "X", Role::SpinButton);
        window.click_pitch(
            centre(x_field),
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
        );
        assert_eq!(
            focused(&window),
            Some(x_id),
            "premise: X's number is focused"
        );
        window.pitch(
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
            vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(focused(&window), Some(y_id), "Tab moved the keyboard to Y");
    }

    // A step applies the drop rule as a typed Y does: PARTY HARD, team
    // 718's LMF in slot 8 at (20, 26), stays an LMF while Y's `⏵` takes
    // him up to 34, the midfield band's top; the next click puts him at
    // 35, in the forwards' band, which offers SS and LWF, and his higher
    // overall makes him an LWF, 86 against 84.
    #[test]
    fn a_y_stepped_into_the_forwards_band_makes_party_hard_an_lwf() {
        let rows = fixture_rows(PesVersion::Pes21);
        let mut roster = roster_rows(&rows);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        party_hard(&mut tactics, &mut roster);
        let mut window = Headless::sized(600.0, 450.0);
        let mut view = TacticsView::default();
        let index = 8;
        let before = tactics.clone();
        let slot = before.presets[0].formations[0].players[index];
        assert_eq!(
            (position_label(slot.position), slot.x, slot.y),
            ("LMF", 20, 26),
            "premise: PARTY HARD's spot"
        );

        open(&mut window, &mut tactics, &mut view, &roster, index);
        for _ in 26..34 {
            click_arrow(&mut window, "Y", 1, &mut tactics, &mut view, &roster);
        }
        let stepped = tactics.presets[0].formations[0].players[index];
        assert_eq!(
            (position_label(stepped.position), stepped.x, stepped.y),
            ("LMF", 20, 34),
            "an LMF up to the midfield band's top"
        );
        click_arrow(&mut window, "Y", 1, &mut tactics, &mut view, &roster);
        let stepped = tactics.presets[0].formations[0].players[index];
        assert_eq!(
            (position_label(stepped.position), stepped.x, stepped.y),
            ("LWF", 20, 35),
            "an LWF at 35"
        );
        let mut expect = before;
        expect.presets[0].formations[0].players[index] = stepped;
        assert_eq!(tactics, expect, "his slot alone changes");
    }

    // A stored x past the range (110) steps back into it: `⏵` is
    // disabled and one click on `⏴` gives 104, the range's end.
    #[test]
    fn a_stored_x_past_the_range_steps_back_to_its_end() {
        let mut window = Headless::sized(600.0, 450.0);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        let mut view = TacticsView::default();
        let rows = fixture_rows(PesVersion::Pes21);
        let roster = roster_rows(&rows);
        let index = 4;
        tactics.presets[0].formations[0].players[index].x = 110;

        open(&mut window, &mut tactics, &mut view, &roster, index);
        let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        let [down, up] = arrows(&nodes, "X");
        assert!(!down.is_disabled(), "⏴ is enabled past the range");
        assert!(up.is_disabled(), "⏵ is disabled past the range");
        assert!(
            click_arrow(&mut window, "X", 0, &mut tactics, &mut view, &roster),
            "the click is an edit"
        );
        assert_eq!(
            tactics.presets[0].formations[0].players[index].x, GRID_X,
            "one ⏴ from 110 gives 104"
        );
    }

    // Each arrow is disabled at its end of the range: at x 0 `⏴`, at x
    // 104 `⏵`, and a click on the disabled one writes nothing. Neither
    // shows the keeper's hover: the spot is not fixed.
    #[test]
    fn each_arrow_is_disabled_at_its_end_of_the_range() {
        let mut window = Headless::sized(600.0, 450.0);
        window
            .ctx
            .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        let mut view = TacticsView::default();
        let rows = fixture_rows(PesVersion::Pes21);
        let roster = roster_rows(&rows);
        let index = 4;
        open(&mut window, &mut tactics, &mut view, &roster, index);

        for (x, disabled) in [(0, 0), (GRID_X, 1)] {
            tactics.presets[0].formations[0].players[index].x = x;
            let (nodes, _) =
                window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
            let pair = arrows(&nodes, "X");
            assert!(
                pair[disabled].is_disabled(),
                "at x {x} arrow {disabled} is disabled"
            );
            assert!(
                !pair[1 - disabled].is_disabled(),
                "at x {x} the other is enabled"
            );
            let at = centre(pair[disabled]);
            assert!(
                !click_arrow(&mut window, "X", disabled, &mut tactics, &mut view, &roster),
                "at x {x} the disabled arrow's click writes nothing"
            );
            assert_eq!(tactics.presets[0].formations[0].players[index].x, x);
            assert_eq!(
                window.pitch_tooltip(
                    at,
                    "goalkeeper",
                    PesVersion::Pes21,
                    &mut tactics,
                    &mut view,
                    &roster,
                ),
                None,
                "at x {x} no keeper's hover"
            );
        }
    }

    // Y's range starts at 4, the outfielders' lowest row, as a drag's
    // does: at y 4 `⏴` is disabled and its click writes nothing, and a
    // slot stored at y 2, below the range, steps back in to 4 on `⏵`.
    // SE-GUI-42
    #[test]
    fn y_stops_at_4_and_a_stored_y_below_steps_back_to_4() {
        let mut window = Headless::sized(600.0, 450.0);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        let mut view = TacticsView::default();
        let rows = fixture_rows(PesVersion::Pes21);
        let roster = roster_rows(&rows);
        let index = 4;
        open(&mut window, &mut tactics, &mut view, &roster, index);

        tactics.presets[0].formations[0].players[index].y = 4;
        let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        let [down, up] = arrows(&nodes, "Y");
        assert!(down.is_disabled(), "⏴ is disabled at y 4");
        assert!(!up.is_disabled(), "⏵ is enabled at y 4");
        assert!(
            !click_arrow(&mut window, "Y", 0, &mut tactics, &mut view, &roster),
            "the disabled ⏴'s click writes nothing"
        );
        assert_eq!(
            tactics.presets[0].formations[0].players[index].y, 4,
            "y stays 4"
        );

        tactics.presets[0].formations[0].players[index].y = 2;
        let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
        let [down, up] = arrows(&nodes, "Y");
        assert!(down.is_disabled(), "⏴ is disabled below the range");
        assert!(!up.is_disabled(), "⏵ is enabled below the range");
        assert!(
            click_arrow(&mut window, "Y", 1, &mut tactics, &mut view, &roster),
            "the click is an edit"
        );
        assert_eq!(
            tactics.presets[0].formations[0].players[index].y, 4,
            "one ⏵ from 2 gives 4"
        );
    }

    // The keeper's popover: all four arrows disabled, under the fields'
    // hover "The goalkeeper's spot is fixed".
    #[test]
    fn the_keepers_four_arrows_are_disabled() {
        let mut window = Headless::sized(600.0, 450.0);
        window
            .ctx
            .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        let mut view = TacticsView::default();
        let rows = fixture_rows(PesVersion::Pes21);
        let roster = roster_rows(&rows);
        let keeper = tactics.presets[0].formations[0]
            .players
            .iter()
            .position(|slot| slot.position == 0)
            .expect("the fixture has a keeper slot");
        let slot = tactics.presets[0].formations[0].players[keeper];
        // His y 3 is below Y's range, so Y's `⏴` would be disabled for an
        // outfielder too; the hover below is the keeper's alone.
        assert!(
            slot.x > 0 && slot.x < GRID_X && slot.y < GRID_Y,
            "premise: no arrow but Y's ⏴ is at its end of the range"
        );

        open(&mut window, &mut tactics, &mut view, &roster, keeper);
        for label in ["X", "Y"] {
            for arrow in 0..2 {
                let (nodes, _) =
                    window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
                let node = arrows(&nodes, label)[arrow];
                assert!(node.is_disabled(), "the keeper's {label} arrow {arrow}");
                assert_eq!(
                    window.pitch_tooltip(
                        centre(node),
                        "goalkeeper",
                        PesVersion::Pes21,
                        &mut tactics,
                        &mut view,
                        &roster,
                    ),
                    Some("The goalkeeper's spot is fixed".to_owned()),
                    "the {label} arrow {arrow}'s hover"
                );
            }
        }
    }

    // The draft's `.spin`: each stepper is one box 52 wide and 18 high,
    // `⏴` 12, the number 28 and `⏵` 12, each touching the next; the
    // outer corners rounded as a button's, the inner ones and the
    // number's square. An arrow measures 12.5, its glyph's advance, and
    // the box 53: within the one point allowed.
    #[test]
    fn each_stepper_is_one_box_as_wide_as_the_number_box_was() {
        let mut window = Headless::sized(600.0, 450.0);
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        let mut view = TacticsView::default();
        let rows = fixture_rows(PesVersion::Pes21);
        let roster = roster_rows(&rows);
        open(&mut window, &mut tactics, &mut view, &roster, 4);
        let (nodes, shapes) = window.frame(vec![], |ui| {
            crate::pitch::pitch(
                ui,
                "pitch",
                PesVersion::Pes21,
                &mut tactics,
                &mut view,
                &roster,
            );
        });
        let round = window
            .ctx
            .global_style()
            .visuals
            .widgets
            .inactive
            .corner_radius;
        let left = egui::CornerRadius {
            ne: 0,
            se: 0,
            ..round
        };
        let right = egui::CornerRadius {
            nw: 0,
            sw: 0,
            ..round
        };
        // The corners of the frame painted over `bounds`.
        let corners = |bounds: accesskit::Rect| {
            shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(shape)
                        if (f64::from(shape.rect.min.x) - bounds.x0).abs() <= 0.5
                            && (f64::from(shape.rect.max.x) - bounds.x1).abs() <= 0.5
                            && (f64::from(shape.rect.min.y) - bounds.y0).abs() <= 0.5
                            && (f64::from(shape.rect.max.y) - bounds.y1).abs() <= 0.5 =>
                    {
                        Some(shape.corner_radius)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("no frame painted over {bounds:?}"))
        };
        for label in ["X", "Y"] {
            let [down, up] = arrows(&nodes, label).map(|node| node.bounds().expect("bounds"));
            let number = control_right_of(&nodes, label, Role::SpinButton)
                .bounds()
                .expect("bounds");
            for (part, bounds, width) in
                [("⏴", down, 12.0), ("number", number, 28.0), ("⏵", up, 12.0)]
            {
                assert!(
                    (bounds.x1 - bounds.x0 - width).abs() <= 1.0,
                    "{label}'s {part} is {width} wide: {bounds:?}"
                );
                assert!(
                    (bounds.y1 - bounds.y0 - 18.0).abs() <= 1.0,
                    "{label}'s {part} is 18 high: {bounds:?}"
                );
                assert!(
                    (bounds.y0 - down.y0).abs() <= 1.0,
                    "{label}'s {part} is on the box's row: {bounds:?} vs {down:?}"
                );
            }
            assert!(
                (number.x0 - down.x1).abs() <= 1.0 && (up.x0 - number.x1).abs() <= 1.0,
                "{label}'s three parts touch: {down:?} {number:?} {up:?}"
            );
            assert!(
                (up.x1 - down.x0 - 52.0).abs() <= 1.0,
                "{label}'s stepper is 52 wide: {down:?} to {up:?}"
            );
            assert_eq!(corners(down), left, "{label}'s ⏴ rounds its outer corners");
            assert_eq!(
                corners(number),
                egui::CornerRadius::ZERO,
                "{label}'s number is square"
            );
            assert_eq!(corners(up), right, "{label}'s ⏵ rounds its outer corners");
        }
    }
}
