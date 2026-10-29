//! The slot popover, the draft's `slotPopover`: a marker click's per-slot
//! panel — the starter's name and roster number, the Position combo
//! beside Make GK and the X and Y fields.

use pes_savefile::model::tactics::TeamTactics;
use studio_core::widgets::{step_choice, step_number, wheel_steps};

use super::{GRID_X, GRID_Y, drop_rule, overalls_of, position_label};
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
/// (0 to 104) and Y (0 to 48) fields and Close. A change of X or Y sets
/// his position by the drop rule at the new spot, as a release there
/// does. The keeper's slot shows a
/// disabled "GK", the two fields disabled under "The goalkeeper's spot is
/// fixed" and Make GK disabled. Make GK exchanges the slot's
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
                    ui.add_enabled_ui(false, |ui| {
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
                if ui
                    .add_enabled(!keeper, egui::Button::new("Make GK"))
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
            let x = ui
                .add_enabled(
                    !keeper,
                    egui::DragValue::new(&mut formation.players[index].x)
                        .range(0..=GRID_X)
                        .clamp_existing_to_range(false),
                )
                .on_disabled_hover_text("The goalkeeper's spot is fixed");
            step_number(
                &mut formation.players[index].x,
                wheel_steps(ui, &x),
                0..=GRID_X,
            );
            ui.label("Y");
            let y = ui
                .add_enabled(
                    !keeper,
                    egui::DragValue::new(&mut formation.players[index].y)
                        .range(0..=GRID_Y)
                        .clamp_existing_to_range(false),
                )
                .on_disabled_hover_text("The goalkeeper's spot is fixed");
            step_number(
                &mut formation.players[index].y,
                wheel_steps(ui, &y),
                0..=GRID_Y,
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

#[cfg(test)]
mod tests {
    use egui::accesskit::Role;
    use pes_version::PesVersion;

    use super::*;
    use crate::pitch::tests::marker_point;
    use crate::test_support::{
        Headless, centre, controls_right_of, entry, fixture_rows, fixture_tactics, roster_rows,
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
}
