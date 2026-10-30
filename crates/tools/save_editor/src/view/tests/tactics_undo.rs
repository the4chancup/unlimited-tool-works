//! The Tactics editor's undo-entries' tests: which frames merge into one
//! entry and which split.

use super::*;

// One undo entry per gesture in this editor: a slider drag lands as one
// undoable edit and the team is marked changed.
#[test]
fn a_slider_drag_on_the_editor_is_one_undo_entry_and_marks_the_team() {
    let path = fixture(PesVersion::Pes21, "tactics_slider_drag");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    open_team_view(&mut window, &mut view, 701);
    let before = team_entry(&view, 701).tactics.presets[0].sliders.clone();

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let slider = widgets_below(&nodes, "Support range", Role::Slider)
        .into_iter()
        .next()
        .expect("the Support range slider");
    let bounds = slider.bounds().expect("slider bounds");
    let from = centre(slider);
    let to = egui::pos2((bounds.x1 - 4.0) as f32, from.y);
    drag(&mut window, from, to, &mut view);

    assert_ne!(
        team_entry(&view, 701).tactics.presets[0].sliders,
        before,
        "the drag changed preset 1's sliders"
    );
    assert!(
        view.session.as_ref().unwrap().is_dirty(),
        "the team is marked changed"
    );
    assert_eq!(
        undo_entries(&mut window, &mut view),
        1,
        "the whole drag was one undo entry"
    );
}

// A starter swapped with a bench player: the roles follow the swap, one
// undo entry, saved.
#[test]
fn a_bench_player_dragged_onto_a_starter_swaps_them_as_one_undo_entry() {
    let path = fixture(PesVersion::Pes21, "tactics_lineup_swap");
    // The starter the drag replaces is the penalty taker, so the role has
    // to follow the swap.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let starter_slot = file.team(701).unwrap().tactics.starting_eleven[8];
    let bench_slot = file.team(701).unwrap().tactics.bench_order[0];
    assert_ne!(starter_slot, bench_slot, "premise: two different slots");
    file.team_mut(701).unwrap().tactics.set_pieces.penalty = starter_slot;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 1400.0);
    open_team_view(&mut window, &mut view, 701);

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let names = lineup_names(&nodes);
    assert_eq!(names.len(), 23, "the eleven and the bench's 12 players");
    drag(&mut window, centre(names[11]), centre(names[8]), &mut view);

    let tactics = &team_entry(&view, 701).tactics;
    assert_eq!(
        tactics.starting_eleven[8], bench_slot,
        "the bench player starts"
    );
    assert_eq!(tactics.bench_order[0], starter_slot, "the starter sits");
    assert_eq!(
        tactics.set_pieces.penalty, bench_slot,
        "the taker followed the swap"
    );

    // One undo entry restores both.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let tactics = &team_entry(&view, 701).tactics;
    assert_eq!(tactics.starting_eleven[8], starter_slot, "the eleven back");
    assert_eq!(tactics.bench_order[0], bench_slot, "the bench back");
    assert_eq!(tactics.set_pieces.penalty, starter_slot, "the taker back");
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "the whole drop was one undo entry"
    );

    // Redo and save: the reopened file holds the swap and the moved taker.
    window.frame(
        vec![key(
            egui::Key::Z,
            egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
            true,
        )],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let reopened = EditFile::load(&path).unwrap();
    let tactics = &reopened.team(701).unwrap().tactics;
    assert_eq!(tactics.starting_eleven[8], bench_slot, "the swap saved");
    assert_eq!(tactics.bench_order[0], starter_slot, "the bench saved");
    assert_eq!(tactics.set_pieces.penalty, bench_slot, "the taker saved");
}

// Only one widget's changes merge into the open undo entry: a popover X
// typed into, then a style pair clicked in the same unsealed span, is two
// undo steps — the pitch's and the settings group's own.
#[test]
fn a_pitch_edit_and_a_settings_click_undo_as_two_entries() {
    let path = fixture(PesVersion::Pes21, "tactics_undo_keys");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(1600.0, 1400.0);
    open_team_view(&mut window, &mut view, 701);
    let before = team_entry(&view, 701).tactics;
    let players = before.presets[0].formations[0].players;
    assert!(
        !before.presets[0].style.attacking_style,
        "premise: a real change"
    );

    // The popover on an outfield marker a click can reach (a point inside
    // its box no marker drawn after it covers), the X field focused, "55"
    // typed (no Enter: the field keeps the keyboard, the span unsealed).
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let canvas = canvas_of(&nodes);
    let (index, at) = (0..11)
        .filter(|&index| players[index].position != 0)
        .find_map(|index| marker_point(canvas, &players, index).map(|at| (index, at)))
        .expect("a marker a click can reach");
    let slot_before = players[index];
    click_at(&mut window, at, &mut view);
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let x_label = ids
        .iter()
        .find(|(_, node)| node.role() == Role::Label && node.value() == Some("X"))
        .and_then(|(_, node)| node.bounds())
        .expect("the X label");
    let mut fields: Vec<accesskit::NodeId> = ids
        .iter()
        .filter(|(_, node)| {
            node.role() == Role::SpinButton
                && node.bounds().is_some_and(|bounds| {
                    (bounds.y0 - x_label.y0).abs() < 4.0 && bounds.x0 >= x_label.x1 - 1.0
                })
        })
        .map(|(id, _)| *id)
        .collect();
    fields.sort_by(|a, b| {
        let bounds = |id: &accesskit::NodeId| {
            ids.iter()
                .find(|(other, _)| other == id)
                .and_then(|(_, node)| node.bounds())
                .expect("bounds")
                .x0
        };
        bounds(a).partial_cmp(&bounds(b)).expect("ordered bounds")
    });
    let field_id = *fields.first().expect("the X field's id");
    action_focus(&mut window, &mut view, field_id);
    window.frame(vec![egui::Event::Text("55".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].formations[0].players[index].x,
        55,
        "the typed x is written"
    );

    // A style pair click in the same span.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Possession"),
        &mut view,
    );
    assert!(
        team_entry(&view, 701).tactics.presets[0]
            .style
            .attacking_style,
        "the click wrote"
    );

    // One undo each: the settings click first, then the pitch's X.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let tactics = &team_entry(&view, 701).tactics;
    assert!(
        !tactics.presets[0].style.attacking_style,
        "the style click reverted"
    );
    assert_eq!(
        tactics.presets[0].formations[0].players[index].x, 55,
        "the pitch edit still applied"
    );
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].formations[0].players[index].x,
        slot_before.x,
        "the X edit reverted"
    );
}

// One undo entry per gesture where the same widget
// key covers two gestures: a typed popover X then a Make GK click (the
// click's press keeps the span open, and both writes are the pitch's)
// are two steps — a gesture ends when its widget loses the keyboard.
#[test]
fn a_typed_popover_edit_then_make_gk_undo_as_two_entries() {
    let path = fixture(PesVersion::Pes21, "tactics_undo_focus");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(1600.0, 1400.0);
    open_team_view(&mut window, &mut view, 701);
    let before = team_entry(&view, 701).tactics;
    let players = before.presets[0].formations[0].players;
    let keeper = players
        .iter()
        .position(|slot| slot.position == 0)
        .expect("the fixture has a keeper slot");

    // The popover on an outfield marker a click can reach (a point inside
    // its box no marker drawn after it covers), the X field focused, "55"
    // typed (no Enter: the field keeps the keyboard, the span unsealed).
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let canvas = canvas_of(&nodes);
    let (index, at) = (0..11)
        .filter(|&index| players[index].position != 0)
        .find_map(|index| marker_point(canvas, &players, index).map(|at| (index, at)))
        .expect("a marker a click can reach");
    assert_ne!(index, keeper, "premise: not the keeper's slot");
    let slot_before = players[index];
    click_at(&mut window, at, &mut view);
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let x_label = ids
        .iter()
        .find(|(_, node)| node.role() == Role::Label && node.value() == Some("X"))
        .and_then(|(_, node)| node.bounds())
        .expect("the X label");
    let mut fields: Vec<accesskit::NodeId> = ids
        .iter()
        .filter(|(_, node)| {
            node.role() == Role::SpinButton
                && node.bounds().is_some_and(|bounds| {
                    (bounds.y0 - x_label.y0).abs() < 4.0 && bounds.x0 >= x_label.x1 - 1.0
                })
        })
        .map(|(id, _)| *id)
        .collect();
    fields.sort_by(|a, b| {
        let bounds = |id: &accesskit::NodeId| {
            ids.iter()
                .find(|(other, _)| other == id)
                .and_then(|(_, node)| node.bounds())
                .expect("bounds")
                .x0
        };
        bounds(a).partial_cmp(&bounds(b)).expect("ordered bounds")
    });
    let field_id = *fields.first().expect("the X field's id");
    action_focus(&mut window, &mut view, field_id);
    window.frame(vec![egui::Event::Text("55".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].formations[0].players[index].x,
        55,
        "the typed x is written"
    );

    // Make GK in the same popover: the keeper swap is its own gesture.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Make GK"),
        &mut view,
    );
    let tactics = team_entry(&view, 701).tactics;
    assert_eq!(
        tactics.starting_eleven[index], before.starting_eleven[keeper],
        "he takes the goal"
    );
    assert_eq!(
        tactics.starting_eleven[keeper], before.starting_eleven[index],
        "the keeper takes his slot"
    );

    // One Ctrl+Z reverts the keeper swap alone; the X still applied.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let tactics = team_entry(&view, 701).tactics;
    assert_eq!(
        tactics.starting_eleven[index], before.starting_eleven[index],
        "one undo reverts the keeper swap"
    );
    assert_eq!(
        tactics.starting_eleven[keeper], before.starting_eleven[keeper],
        "and the keeper's slot"
    );
    assert_eq!(
        tactics.presets[0].formations[0].players[index].x, 55,
        "the X edit still applied"
    );
    // A second reverts the typed X.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].formations[0].players[index].x,
        slot_before.x,
        "a second undo reverts the X edit"
    );
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "exactly two entries"
    );
}

/// The popover on an outfield marker a click can reach (a point inside
/// its box no marker drawn after it covers), opened and its X field
/// focused (the DragValue holding the keyboard, no Enter): the marker's
/// formation index and the X and Y fields' AccessKit ids.
fn open_popover_and_focus_x(
    window: &mut Headless,
    view: &mut SaveView,
) -> (usize, accesskit::NodeId, accesskit::NodeId) {
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let canvas = canvas_of(&nodes);
    let players = team_entry(view, 701).tactics.presets[0].formations[0].players;
    // The popover on an outfield marker a click can reach.
    let (index, at) = (0..11)
        .filter(|&index| players[index].position != 0)
        .find_map(|index| marker_point(canvas, &players, index).map(|at| (index, at)))
        .expect("a marker a click can reach");
    click_at(window, at, view);
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let x_label = ids
        .iter()
        .find(|(_, node)| node.role() == Role::Label && node.value() == Some("X"))
        .and_then(|(_, node)| node.bounds())
        .expect("the X label");
    let mut fields: Vec<accesskit::NodeId> = ids
        .iter()
        .filter(|(_, node)| {
            node.role() == Role::SpinButton
                && node.bounds().is_some_and(|bounds| {
                    (bounds.y0 - x_label.y0).abs() < 4.0 && bounds.x0 >= x_label.x1 - 1.0
                })
        })
        .map(|(id, _)| *id)
        .collect();
    fields.sort_by(|a, b| {
        let bounds = |id: &accesskit::NodeId| {
            ids.iter()
                .find(|(other, _)| other == id)
                .and_then(|(_, node)| node.bounds())
                .expect("bounds")
                .x0
        };
        bounds(a).partial_cmp(&bounds(b)).expect("ordered bounds")
    });
    action_focus(window, view, *fields.first().expect("the X field's id"));
    (index, fields[0], fields[1])
}

// A typed field's gesture ends before the next gesture's first push,
// never mid-gesture: a DragValue holding the keyboard (the popover's X),
// then a press-drag-release on a Formation settings slider — the press's
// rail-jump and the moves are one undo step, and the typed X another.
#[test]
fn a_typed_x_then_a_slider_drag_undo_as_two_entries() {
    let path = fixture(PesVersion::Pes21, "tactics_undo_focus_drag");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(1600.0, 1400.0);
    open_team_view(&mut window, &mut view, 701);
    let before = team_entry(&view, 701).tactics;
    let (index, _, _) = open_popover_and_focus_x(&mut window, &mut view);
    let slot_before = before.presets[0].formations[0].players[index];
    window.frame(vec![egui::Event::Text("55".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].formations[0].players[index].x,
        55,
        "the typed x is written"
    );

    // A press on the Support range rail, off its handle: the rail-jump
    // writes on the press frame; two moves and the release follow. The
    // end lands at the rail's far side of the stored value (support
    // range runs 1 to 10).
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let slider = widgets_below(&nodes, "Support range", Role::Slider)
        .into_iter()
        .next()
        .expect("the Support range slider");
    let bounds = slider.bounds().expect("slider bounds");
    let stored = team_entry(&view, 701).tactics.presets[0]
        .sliders
        .support_range;
    let (press, end) = if stored <= 5 {
        (
            egui::pos2(bounds.x1 as f32 - 4.0, centre(slider).y),
            egui::pos2(bounds.x1 as f32 - 20.0, centre(slider).y),
        )
    } else {
        (
            egui::pos2(bounds.x0 as f32 + 4.0, centre(slider).y),
            egui::pos2(bounds.x0 as f32 + 20.0, centre(slider).y),
        )
    };
    let mid = egui::pos2((bounds.x0 + bounds.x1) as f32 / 2.0, press.y);
    window.frame(pointer_at(press, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(vec![egui::Event::PointerMoved(mid)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(vec![egui::Event::PointerMoved(end)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(pointer_at(end, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let after = team_entry(&view, 701).tactics;
    assert_ne!(
        after.presets[0].sliders.support_range, stored,
        "the drag changed the stored value"
    );
    assert_eq!(
        after.presets[0].formations[0].players[index].x, 55,
        "the typed x stays"
    );

    // One Ctrl+Z restores the slider's stored value entirely — the
    // rail-jump and the moves are one step — and leaves the typed X.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let tactics = team_entry(&view, 701).tactics;
    assert_eq!(
        tactics.presets[0].sliders, before.presets[0].sliders,
        "one undo restores the whole drag"
    );
    assert_eq!(
        tactics.presets[0].formations[0].players[index].x, 55,
        "the typed X still applied"
    );
    // A second reverts the typed X.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].formations[0].players[index].x,
        slot_before.x,
        "a second undo reverts the X edit"
    );
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "exactly two entries"
    );
}

// A gesture that ends by the keyboard moving straight to another field —
// no pointer, no write on the frame focus moves: the X field's typed
// value and the Y field's typed value are two undo steps, sealed on the
// move.
#[test]
fn a_typed_x_then_typing_into_y_undo_as_two_entries() {
    let path = fixture(PesVersion::Pes21, "tactics_undo_focus_move");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(1600.0, 1400.0);
    open_team_view(&mut window, &mut view, 701);
    let before = team_entry(&view, 701).tactics;
    let (index, _x_id, y_id) = open_popover_and_focus_x(&mut window, &mut view);
    let slot_before = before.presets[0].formations[0].players[index];
    window.frame(vec![egui::Event::Text("55".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].formations[0].players[index].x,
        55,
        "the typed x is written"
    );

    // The keyboard moves straight to Y (a Tab: no pointer, no write on
    // the frame).
    window.frame(
        vec![key(egui::Key::Tab, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        window
            .ctx
            .memory(|m| m.focused().map(|id| id.accesskit_id())),
        Some(y_id),
        "Tab moved the keyboard to Y"
    );
    window.frame(vec![egui::Event::Text("33".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].formations[0].players[index].y,
        33,
        "the typed y is written"
    );
    // Escape: the Y gesture ends — the keyboard lets go of the field.
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        window
            .ctx
            .memory(|m| m.focused().map(|id| id.accesskit_id())),
        None,
        "Escape released the keyboard"
    );

    // One Ctrl+Z reverts the Y write alone: X still 55, Y back.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let tactics = team_entry(&view, 701).tactics;
    assert_eq!(
        tactics.presets[0].formations[0].players[index].y, slot_before.y,
        "one undo reverts the Y edit"
    );
    assert_eq!(
        tactics.presets[0].formations[0].players[index].x, 55,
        "the X edit still applied"
    );
    // A second reverts the typed X.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].formations[0].players[index].x,
        slot_before.x,
        "a second undo reverts the X edit"
    );
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "exactly two entries"
    );
}

// A value typed over several frames while the field holds the keyboard
// is one entry: "5" then "5" lands as 55, undone in one step.
#[test]
fn a_value_typed_over_several_frames_is_one_undo_entry() {
    let path = fixture(PesVersion::Pes21, "tactics_undo_typing");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(1600.0, 1400.0);
    open_team_view(&mut window, &mut view, 701);
    let before = team_entry(&view, 701).tactics;
    let (index, _x_id, _y_id) = open_popover_and_focus_x(&mut window, &mut view);
    let slot_before = before.presets[0].formations[0].players[index];
    for _ in 0..2 {
        window.frame(vec![egui::Event::Text("5".to_owned())], |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
    }
    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].formations[0].players[index].x,
        55,
        "the typed frames landed as 55"
    );
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].formations[0].players[index].x,
        slot_before.x,
        "one undo reverts the whole typed value"
    );
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "the two typed frames were one entry"
    );
}

// The same-key half of it: a Substitution pick and the slider drag are
// two gestures under two widget keys, so they must be two steps — the
// press's seal is what keeps them apart.
#[test]
fn a_substitution_pick_then_a_slider_drag_undo_as_two_entries() {
    let path = fixture(PesVersion::Pes21, "tactics_undo_focus_drag2");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(1600.0, 1400.0);
    open_team_view(&mut window, &mut view, 701);
    let before = team_entry(&view, 701).tactics;
    let stored = before.auto.substitution.expect("PES 21 stores it");
    assert_ne!(stored, 3, "premise: a real change");

    // The combo right of the Substitution label, opened by an AccessKit
    // click; its items join the tree the frame after. The pick is a
    // second AccessKit click (a pointer click does not reach a popup
    // item), then a pointer click on a label closes the popup it
    // leaves open.
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let combo = node_id_right_of(&ids, "Substitution", Role::ComboBox);
    action_click(&mut window, &mut view, combo);
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let combo_bounds = ids
        .iter()
        .find(|(id, _)| *id == combo)
        .and_then(|(_, node)| node.bounds())
        .expect("the combo's bounds");
    // The popup's "Very early", below the combo.
    let item = ids
        .iter()
        .find(|(_, node)| {
            node.role() == Role::Button
                && node.label() == Some("Very early")
                && node.bounds().is_some_and(|b| b.y0 > combo_bounds.y0)
        })
        .map(|(id, _)| *id)
        .expect("the popup's Very early item");
    action_click(&mut window, &mut view, item);
    let tactics = team_entry(&view, 701).tactics;
    assert_eq!(
        tactics.auto.substitution.expect("stored"),
        3,
        "the pick is written"
    );
    // A pointer click on a label closes the popup the pick left open.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let heading = label_bounds(&nodes, "Formation settings");
    click_at(
        &mut window,
        egui::pos2(
            ((heading.x0 + heading.x1) / 2.0) as f32,
            ((heading.y0 + heading.y1) / 2.0) as f32,
        ),
        &mut view,
    );

    // Press-drag-release the Compactness slider off its handle.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let slider = widgets_below(&nodes, "Compactness", Role::Slider)
        .into_iter()
        .next()
        .expect("the Compactness slider");
    let bounds = slider.bounds().expect("slider bounds");
    let stored_compact = team_entry(&view, 701).tactics.presets[0]
        .sliders
        .compactness;
    let (press, end) = if stored_compact <= 5 {
        (
            egui::pos2(bounds.x1 as f32 - 4.0, centre(slider).y),
            egui::pos2(bounds.x1 as f32 - 20.0, centre(slider).y),
        )
    } else {
        (
            egui::pos2(bounds.x0 as f32 + 4.0, centre(slider).y),
            egui::pos2(bounds.x0 as f32 + 20.0, centre(slider).y),
        )
    };
    let mid = egui::pos2((bounds.x0 + bounds.x1) as f32 / 2.0, press.y);
    window.frame(pointer_at(press, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(vec![egui::Event::PointerMoved(mid)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(vec![egui::Event::PointerMoved(end)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(pointer_at(end, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let after = team_entry(&view, 701).tactics;
    assert_ne!(
        after.presets[0].sliders.compactness, stored_compact,
        "the drag changed the stored value"
    );
    assert_eq!(
        after.auto.substitution.expect("stored"),
        3,
        "the picked value stays"
    );

    // One Ctrl+Z restores the slider's stored value entirely — the
    // rail-jump and the moves — and leaves the substitution pick.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let tactics = team_entry(&view, 701).tactics;
    assert_eq!(
        tactics.presets[0].sliders, before.presets[0].sliders,
        "one undo restores the whole drag"
    );
    assert_eq!(
        tactics.auto.substitution.expect("stored"),
        3,
        "the pick still applied"
    );
    // A second reverts the pick.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        team_entry(&view, 701)
            .tactics
            .auto
            .substitution
            .expect("stored"),
        stored,
        "a second undo reverts the pick"
    );
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "exactly two entries"
    );
}
