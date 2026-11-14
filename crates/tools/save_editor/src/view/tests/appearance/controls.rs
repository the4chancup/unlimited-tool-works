use super::*;

// Randomize draws every motion but the dribbling one from 1 to the
// version's shown top, one undoable edit: the measured tops, so a draw
// never writes a motion the game does not have. The pinned source lands
// each at its top, so the assertions name the stored tops exactly.
#[test]
fn randomize_draws_within_pes21s_measured_tops_one_undo_entry() {
    let path = fixture(PesVersion::Pes21, "appearance_random_21");
    // A nonzero dribbling motion, so a Randomize that zeroed it shows.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().motion.dribbling = Some(2);
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    view.randomize_draw = Box::new(|top| top);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);
    let motion_of = |view: &SaveView| -> PlayerMotion {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .motion
            .clone()
    };
    let before = motion_of(&view);
    assert_eq!(before.dribbling, Some(2), "premise: the dribbling motion");
    assert_ne!(before.goal_celebration_1, 162, "premise: gc1 starts else");

    // The button sits below the fold under the player header: the click
    // goes through its AccessKit action (a pointer click lands only
    // inside the scroll's clip).
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (button, node) = nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("Randomize"))
        .expect("the Randomize button");
    assert!(!node.is_disabled(), "unlocked the button is enabled");
    action_click(&mut window, &mut view, *button);

    // The pinned draw returns the shown top, so every motion lands at its
    // measured stored top; the dribbling motion is untouched.
    let motion = motion_of(&view);
    assert_eq!(motion.hunching_dribbling, 4);
    assert_eq!(motion.hunching_running, 4);
    assert_eq!(motion.arm_movement_dribbling, 9);
    assert_eq!(motion.arm_movement_running, 9);
    assert_eq!(motion.corner_kick, 9);
    assert_eq!(motion.free_kick, 19);
    assert_eq!(motion.penalty_kick, 6);
    assert_eq!(motion.goal_celebration_1, 162);
    assert_eq!(motion.goal_celebration_2, 162);
    assert_eq!(motion.dribbling, Some(2), "the dribbling motion is left");

    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
    assert_eq!(motion_of(&view), before, "the one undo restores them all");
}

// The same rule on a version with no dribbling motion at all (PES 15's
// measured tops are the small ones).
#[test]
fn randomize_draws_within_pes15s_measured_tops_one_undo_entry() {
    let path = fixture(PesVersion::Pes15, "appearance_random_15");
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    view.randomize_draw = Box::new(|top| top);
    // The dribbling stand-in sits below the 700-point fold since the
    // physique grid's cap lengthened the section; the hover needs it inside
    // the scroll viewport, so the window is taller.
    let mut window = Headless::sized(DEFAULT_PANE.x, 1000.0);
    open_appearance(&mut window, &mut view);
    let motion_of = |view: &SaveView| -> PlayerMotion {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .motion
            .clone()
    };
    let before = motion_of(&view);
    assert_eq!(before.dribbling, None, "PES 15 has no dribbling motion");

    // Its stand-in control is disabled with the version tooltip — the
    // hover needs it inside the scroll's clip.
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Dribbling motion")
    });
    let dribbling = right_of_label(&nodes, "Dribbling motion", Role::SpinButton)
        .into_iter()
        .find(|node| node.is_disabled())
        .expect("the disabled dribbling motion");
    assert_eq!(
        tooltip_after_hover(&mut window, &mut view, centre(dribbling)),
        Some("PES 20+".to_string()),
        "the dribbling motion's tooltip"
    );
    // Below the fold: the click goes through the AccessKit action.
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (button, _) = nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("Randomize"))
        .expect("the Randomize button");
    action_click(&mut window, &mut view, *button);

    let motion = motion_of(&view);
    assert_eq!(motion.hunching_dribbling, 2);
    assert_eq!(motion.hunching_running, 2);
    assert_eq!(motion.arm_movement_dribbling, 4);
    assert_eq!(motion.arm_movement_running, 4);
    assert_eq!(motion.corner_kick, 5);
    assert_eq!(motion.free_kick, 15);
    assert_eq!(motion.penalty_kick, 3);
    assert_eq!(motion.goal_celebration_1, 112);
    assert_eq!(motion.goal_celebration_2, 112);
    assert_eq!(motion.dribbling, None, "still none on PES 15");

    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
    assert_eq!(motion_of(&view), before, "the one undo restores them all");
}

// A value the save holds outside its range draws as stored and changes
// only when edited, as in the player view's columns: drawing a player
// never edits him (the egui clamping trap stays off).
#[test]
fn a_stored_celebration_past_its_top_draws_as_stored_without_marking_the_player() {
    let path = fixture(PesVersion::Pes21, "appearance_celebration_200");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().motion.goal_celebration_1 = 200;
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);
    for _ in 0..5 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let celebration = right_of_label(&nodes, "Goal celebration 1", Role::SpinButton)
        .first()
        .copied()
        .expect("the celebration control");
    assert_eq!(
        celebration.value(),
        Some("200"),
        "the control draws the stored 200"
    );
    assert_drawing_edited_nothing(&view);
    let session = view.session.as_ref().unwrap();
    assert_eq!(
        session
            .file()
            .player(70101)
            .unwrap()
            .motion
            .goal_celebration_1,
        200,
        "the value stands as stored"
    );
}

// The same rule on a signed physique field: a stored 15 on Neck length
// (a ±7 scale) draws the 8 it means, never clamped to the shown top.
#[test]
fn a_stored_physique_past_its_top_draws_as_stored_without_marking_the_player() {
    let path = fixture(PesVersion::Pes21, "appearance_neck_15");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    assert_ne!(
        file.player(70101).unwrap().appearance.neck_length,
        15,
        "premise: Neck length starts else"
    );
    file.player_mut(70101).unwrap().appearance.neck_length = 15;
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Neck length")
    });
    let neck = right_of_label(&nodes, "Neck length", Role::SpinButton)
        .first()
        .copied()
        .expect("the Neck length control");
    assert_eq!(neck.value(), Some("8"), "the control draws the stored 15");
    assert_drawing_edited_nothing(&view);
    let session = view.session.as_ref().unwrap();
    assert_eq!(
        session.file().player(70101).unwrap().appearance.neck_length,
        15,
        "the value stands as stored"
    );
}

// A wheel notch down on a signed physique field showing 0 steps it to
// -1 (stored 6 on the ±7 scale), not to the range's other end.
#[test]
fn a_wheel_notch_down_on_a_physique_field_at_zero_steps_below() {
    let path = fixture(PesVersion::Pes21, "appearance_neck_wheel");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().appearance.neck_length = 7;
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);
    let neck_of = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .appearance
            .neck_length
    };
    assert_eq!(neck_of(&view), 7, "premise: Neck length shows 0");
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Neck length")
    });
    let neck = right_of_label(&nodes, "Neck length", Role::SpinButton)
        .first()
        .copied()
        .expect("the Neck length control");
    assert_eq!(neck.value(), Some("0"), "premise: the stored 7 shows 0");
    // The scroll's own wheel leaves the pane's wheel unrested 500 ms.
    idle_frames(&mut window, &mut view, 31);
    notch(&mut window, &mut view, centre(neck), false);

    assert_eq!(neck_of(&view), 6, "a notch down from 0 stores -1's 6");
    let nodes = draw(&mut window, &mut view);
    let neck = right_of_label(&nodes, "Neck length", Role::SpinButton)
        .first()
        .copied()
        .expect("the Neck length control");
    assert_eq!(neck.value(), Some("-1"), "the control draws -1");
}

// A stored star the save holds draws in the Presentation group's box.
#[test]
fn a_stored_star_draws_in_the_presentation_box() {
    let path = fixture(PesVersion::Pes21, "appearance_star_3");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    assert_ne!(
        file.player(70101).unwrap().stats.star,
        Some(3),
        "premise: Star starts else"
    );
    file.player_mut(70101).unwrap().stats.star = Some(3);
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Star")
    });
    let star = right_of_label(&nodes, "Star", Role::SpinButton)
        .first()
        .copied()
        .expect("the Star control");
    assert_eq!(star.value(), Some("3"), "the box draws the stored star");
}

// The same rule's low end: a draw of 1 stores 0 in a 1-based motion and 1
// in a celebration (a celebration is never OFF), the dribbling motion
// unchanged.
#[test]
fn randomize_low_end_stores_zero_for_one_based_and_one_for_celebrations() {
    let path = fixture(PesVersion::Pes21, "appearance_random_low");
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    view.randomize_draw = Box::new(|_| 1);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);
    let motion_of = |view: &SaveView| -> PlayerMotion {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .motion
            .clone()
    };

    // Below the fold: the click goes through the AccessKit action.
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (button, _) = nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("Randomize"))
        .expect("the Randomize button");
    action_click(&mut window, &mut view, *button);

    let motion = motion_of(&view);
    for (label, value) in [
        ("hunching dribbling", motion.hunching_dribbling),
        ("hunching running", motion.hunching_running),
        ("arm dribbling", motion.arm_movement_dribbling),
        ("arm running", motion.arm_movement_running),
        ("corner", motion.corner_kick),
        ("free", motion.free_kick),
        ("penalty", motion.penalty_kick),
    ] {
        assert_eq!(value, 0, "{label} stores 0 for a drawn 1");
    }
    assert_eq!(motion.goal_celebration_1, 1, "a celebration stores 1");
    assert_eq!(motion.goal_celebration_2, 1, "a celebration stores 1");
    assert_eq!(motion.dribbling, Some(0), "the dribbling motion is left");
}

// The physique scale: a `Signed` key shows `stored - limit` and stores
// `shown + limit` (limit 7). Stored 12 draws 5; typing 3 stores 10.
#[test]
fn a_physique_field_shows_stored_minus_limit_and_stores_shown_plus_it() {
    let path = fixture(PesVersion::Pes21, "physique_scale");
    // A stored value off the neutral 7, so `- limit` and `/ limit` differ.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().appearance.neck_length = 12;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let neck_length = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .appearance
            .neck_length
    };
    assert_eq!(neck_length(&view), 12, "premise: stored 12");
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);

    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Neck length")
    });
    let neck = right_of_label(&nodes, "Neck length", Role::SpinButton)
        .first()
        .copied()
        .expect("the Neck length control");
    assert_eq!(neck.value(), Some("5"), "stored 12 shows 5");
    click(&mut window, neck, &mut view);
    window.frame(vec![egui::Event::Text("3".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(neck_length(&view), 10, "typed 3 stores 3 + 7");
}

// The physique scale's arrow step on the same ±7 range: an ArrowDown
// takes the shown value down one and an ArrowUp puts it back.
// SE-GUI-57
#[test]
fn a_physique_fields_arrow_press_steps_the_shown_value_by_one() {
    let path = fixture(PesVersion::Pes21, "physique_arrows");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().appearance.neck_length = 12;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let neck_length = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .appearance
            .neck_length
    };
    assert_eq!(neck_length(&view), 12, "premise: stored 12");
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);

    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Neck length")
    });
    let neck = right_of_label(&nodes, "Neck length", Role::SpinButton)
        .first()
        .copied()
        .expect("the Neck length control");
    assert_eq!(neck.value(), Some("5"), "stored 12 shows 5");
    click(&mut window, neck, &mut view);
    // The settle frame lets the field's edit install its key filter,
    // which keeps the arrows for itself.
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        window.ctx.egui_wants_keyboard_input(),
        "premise: the Neck length field holds the keyboard"
    );
    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(neck_length(&view), 11, "one ArrowDown stores 4 + 7");
    window.frame(
        vec![key(egui::Key::ArrowUp, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(neck_length(&view), 12, "one ArrowUp stores 5 + 7 again");
}

// A palette swatch click writes its index as one undoable edit.
#[test]
fn a_swatch_click_stores_its_index() {
    let path = fixture(PesVersion::Pes21, "appearance_swatch");
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);
    let skin_of = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .appearance
            .ingame_face
            .get(IngameFaceField::SkinColor)
            .unwrap()
    };
    assert_eq!(skin_of(&view), 1, "premise: skin is light (1)");

    // The swatch sits below the fold under the player header: the click
    // goes through its AccessKit action (a pointer click lands only
    // inside the scroll's clip).
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (swatch, _) = nodes
        .iter()
        .find(|(_, node)| node.role() == Role::ColorWell && node.label() == Some("Skin Brown"))
        .expect("the brown swatch");
    action_click(&mut window, &mut view, *swatch);

    assert_eq!(skin_of(&view), 5, "brown stores 5");
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
    assert_eq!(skin_of(&view), 1, "the undo restores it");
}

// The motion fields' scales (`motion_number`): a 1-based motion shows
// `stored + 1` and its range ends at the version's shown top (a typed value
// past it clamps there), storing `shown - 1`; a celebration shows as stored.
#[test]
fn a_one_based_motion_shows_one_above_stored_within_the_measured_top() {
    let path = fixture(PesVersion::Pes21, "motion_one_based");
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    // Taller than 700 so the motion rows stay inside the scroll viewport
    // for the click and typing below (the physique cap lengthened the
    // section).
    let mut window = Headless::sized(DEFAULT_PANE.x, 1000.0);
    open_appearance(&mut window, &mut view);
    let motion_of = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .motion
            .clone()
    };
    let stored = motion_of(&view).free_kick;

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let free = right_of_label(&nodes, "Free kick", Role::SpinButton)
        .first()
        .copied()
        .expect("the Free kick control");
    assert_eq!(
        free.value(),
        Some((stored + 1).to_string()).as_deref(),
        "a 1-based motion shows stored + 1"
    );
    let celebration = right_of_label(&nodes, "Goal celebration 1", Role::SpinButton)
        .first()
        .copied()
        .expect("the Goal celebration 1 control");
    assert_eq!(
        celebration.value(),
        Some(motion_of(&view).goal_celebration_1.to_string()).as_deref(),
        "a celebration shows as stored"
    );

    // Typing 7 stores 6; typing past the shown top clamps to it (PES 21's
    // free-kick top 19, shown 20) and stores 19.
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Free kick")
    });
    let free = right_of_label(&nodes, "Free kick", Role::SpinButton)
        .first()
        .copied()
        .expect("the Free kick control");
    click(&mut window, free, &mut view);
    window.frame(vec![egui::Event::Text("7".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(motion_of(&view).free_kick, 6, "typed 7 stores 6");
    click(&mut window, free, &mut view);
    window.frame(vec![egui::Event::Text("99".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        motion_of(&view).free_kick,
        19,
        "past the shown top it stops at 20, stored 19"
    );
}

// A combo pick stores the option's index (`combo`'s `picked != stored`
// write): Sleeves' "Long" is index 2, one undo entry.
#[test]
fn a_combo_pick_stores_the_options_index_as_one_undo_entry() {
    let path = fixture(PesVersion::Pes21, "combo_sleeves");
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let sleeves = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .appearance
            .sleeves
    };
    assert_ne!(sleeves(&view), 2, "premise: not Long");
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);

    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Sleeves")
    });
    let combo = right_of_label(&nodes, "Sleeves", Role::ComboBox)
        .first()
        .copied()
        .expect("the Sleeves combo");
    click(&mut window, combo, &mut view);
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (item, _) = nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Long"))
        .expect("the Long option");
    action_click(&mut window, &mut view, *item);

    assert_eq!(sleeves(&view), 2, "Long stores 2");
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
}

// The Outfielder gloves dropdown lists `Off` and `Worn in winter` over
// the face run's PlayerGloves: a stored 0 reads Off, any non-zero Worn
// in winter; picking Worn in winter writes 1, one undo entry, and the
// unsaved-changes table's row reads `Outfielder gloves`, Off to Worn in
// winter.
// SE-GUI-84
#[test]
fn the_outfielder_gloves_dropdown_writes_the_runs_flag() {
    let path = fixture(PesVersion::Pes21, "outfielder_gloves");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    for (id, gloves) in [(70101u32, 0u8), (70102, 1)] {
        file.player_mut(id)
            .unwrap()
            .appearance
            .ingame_face
            .set(IngameFaceField::PlayerGloves, gloves)
            .unwrap();
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let gloves = |view: &SaveView| {
        player_entry(view, 70101)
            .appearance
            .ingame_face
            .get(IngameFaceField::PlayerGloves)
            .unwrap()
    };
    assert_eq!(gloves(&view), 0, "premise: 70101's run holds 0");
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);

    // The dropdown lists the two readings; 70102's stored 1 shows Worn
    // in winter (the flag's reading, not its number).
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let combo = right_of_label(&nodes, "Outfielder gloves", Role::ComboBox)
        .first()
        .copied()
        .expect("the Outfielder gloves dropdown");
    click(&mut window, combo, &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for option in ["Off", "Worn in winter"] {
        assert!(
            pairs.iter().any(|(_, node)| node.label() == Some(option)),
            "{option} is listed"
        );
    }
    let (winter, _) = pairs
        .iter()
        .find(|(_, node)| node.label() == Some("Worn in winter"))
        .expect("the Worn in winter item");
    action_click(&mut window, &mut view, *winter);

    assert_eq!(gloves(&view), 1, "Worn in winter writes 1");
    // The unsaved-changes table's row, the view's own words.
    let rows = crate::view::prompt::rows(view.session.as_ref().unwrap()).unwrap();
    let row = rows
        .iter()
        .find(|row| row.cells.term.label == "Outfielder gloves")
        .expect("the gloves row");
    assert_eq!(
        (row.cells.term.old.as_str(), row.cells.term.new.as_str()),
        ("Off", "Worn in winter"),
        "the row's values"
    );
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
    assert_eq!(gloves(&view), 0, "the undo restores 0");

    // 70102's stored 1 draws Worn in winter, the flag's reading.
    view.selected = Some(Selection::Player(701, 1));
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let combo = right_of_label(&nodes, "Outfielder gloves", Role::ComboBox)
        .first()
        .copied()
        .expect("the Outfielder gloves dropdown");
    assert_eq!(combo.value(), Some("Worn in winter"), "a stored non-zero");
}

/// The swatches of the Skin row (`Role::ColorWell`, labeled "Skin …"),
/// in x order.
fn skin_swatches(nodes: &[accesskit::Node]) -> Vec<&accesskit::Node> {
    let mut swatches: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::ColorWell
                && node.label().is_some_and(|label| label.starts_with("Skin "))
        })
        .collect();
    swatches.sort_by(|a, b| {
        a.bounds()
            .expect("bounds")
            .x0
            .partial_cmp(&b.bounds().expect("bounds").x0)
            .expect("ordered bounds")
    });
    swatches
}

/// A Label node with `value` on `row`'s y (within four points).
fn label_on_row<'a>(
    nodes: &'a [accesskit::Node],
    row: &accesskit::Node,
    value: &str,
) -> Option<&'a accesskit::Node> {
    let y = row.bounds().expect("the row's bounds").y0;
    nodes.iter().find(|node| {
        node.role() == Role::Label
            && node.value() == Some(value)
            && node.bounds().is_some_and(|b| (b.y0 - y).abs() < 4.0)
    })
}

// The Skin palette draws the palette's entries: on Fox the Custom entry
// is dropped (0 to 6), on PES 17 the whole palette (0 to 7); the held
// index's swatch reports selected; a stored value past the top draws its
// number after the swatches, one at the top none.
#[test]
fn the_skin_palette_draws_its_entries_held_swatch_and_a_past_top_number() {
    let path = fixture(PesVersion::Pes21, "palette_skin");
    // Held at 3 ("Skin Medium"), then past the top (7, a PES 21 save can
    // hold it) for the number case.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101)
        .unwrap()
        .appearance
        .ingame_face
        .set(IngameFaceField::SkinColor, 3)
        .unwrap();
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let swatches = skin_swatches(&nodes);
    assert_eq!(
        swatches.len(),
        7,
        "Fox draws entries 0 to 6, Custom dropped"
    );
    assert!(
        !swatches.iter().any(|sw| sw.label() == Some("Skin Custom")),
        "no Custom swatch on PES 21"
    );
    let held: Vec<&accesskit::Node> = swatches
        .iter()
        .copied()
        .filter(|sw| sw.toggled() == Some(accesskit::Toggled::True))
        .collect();
    assert_eq!(held.len(), 1, "exactly the held swatch is selected");
    assert_eq!(held[0].label(), Some("Skin Medium"), "skin 3 held");

    // A stored value past the top draws its number after the swatches.
    file.player_mut(70101)
        .unwrap()
        .appearance
        .ingame_face
        .set(IngameFaceField::SkinColor, 7)
        .unwrap();
    file.save(&path).unwrap();
    view.open(&path);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let swatches = skin_swatches(&nodes);
    assert!(
        label_on_row(&nodes, swatches[0], "7").is_some(),
        "the past-top value's number draws"
    );
    // One at the top draws none.
    file.player_mut(70101)
        .unwrap()
        .appearance
        .ingame_face
        .set(IngameFaceField::SkinColor, 6)
        .unwrap();
    file.save(&path).unwrap();
    view.open(&path);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let swatches = skin_swatches(&nodes);
    assert!(
        label_on_row(&nodes, swatches[0], "6").is_none(),
        "a value at the top draws no number"
    );

    // Pre-Fox draws the whole palette, Custom included.
    let path17 = fixture(PesVersion::Pes17, "palette_skin_17");
    let mut view17 = SaveView::new();
    view17.open(&path17);
    view17.aesthetics_from_exports = true;
    view17.edit_aesthetics = true;
    let mut window17 = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window17, &mut view17);
    let (_, nodes, _) = window17.frame(Vec::new(), |ui| show(ui, &mut view17, &|| Vec::new()));
    let swatches = skin_swatches(&nodes);
    assert_eq!(swatches.len(), 8, "PES 17 draws the whole palette");
    assert!(
        swatches.iter().any(|sw| sw.label() == Some("Skin Custom")),
        "Custom on PES 17"
    );
}

// The strip and motion grids break a row after the draft's cells (three
// for the strip, two for the motion): the first row's controls share one
// top, the next cell starts the row below.
#[test]
fn the_strip_and_motion_grids_break_after_the_plans_cells() {
    let path = fixture(PesVersion::Pes21, "grid_wraps");
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let y = |label: &str| control_right_of(&nodes, label).bounds().expect("bounds").y0;

    let first = y("Boots ID");
    for label in ["Sleeves", "Inners"] {
        assert!((y(label) - first).abs() < 1.0, "{label} on the first row");
    }
    assert!(
        y("Gloves ID") > first + 5.0,
        "Gloves ID opens the second row"
    );

    let first = y("Hunching (dribbling)");
    assert!(
        (y("Hunching (running)") - first).abs() < 1.0,
        "Hunching (running) on the first row"
    );
    assert!(
        y("Arm movement (dribbling)") > first + 5.0,
        "Arm movement (dribbling) wraps below"
    );
}

// The four strip colours are dropdowns of the colours' names — the
// gloves' and the spectacles' list White to Turquoise — and no swatch of
// theirs remains; skin and iris keep their swatch rows in Colors.
// SE-GUI-84
#[test]
fn the_strip_colours_are_dropdowns_and_the_swatch_rows_stay_in_colors() {
    let path = fixture(PesVersion::Pes21, "strip_colour_dropdowns");
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);

    for label in ["Gloves color", "Spectacles color"] {
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let combo = right_of_label(&nodes, label, Role::ComboBox)
            .first()
            .copied()
            .unwrap_or_else(|| panic!("the {label} dropdown"));
        click(&mut window, combo, &mut view);
        let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        for name in [
            "White",
            "Black",
            "Red",
            "Blue",
            "Yellow",
            "Green",
            "Pink",
            "Turquoise",
        ] {
            assert!(
                pairs.iter().any(|(_, node)| node.label() == Some(name)),
                "{label} lists {name}"
            );
        }
        window.frame(
            vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
    }

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for prefix in ["Gloves color", "Spectacles color", "Wrist tape"] {
        assert!(
            !nodes.iter().any(|node| {
                node.role() == Role::ColorWell
                    && node.label().is_some_and(|label| label.starts_with(prefix))
            }),
            "no {prefix} swatch remains"
        );
    }
    for prefix in ["Skin ", "Iris "] {
        assert!(
            nodes.iter().any(|node| {
                node.role() == Role::ColorWell
                    && node.label().is_some_and(|label| label.starts_with(prefix))
            }),
            "the {prefix}swatch row stays"
        );
    }
}

// The id controls take the record's own bound (`player_max_value`, the
// codec's 14-bit field — wider than `settings.toml`'s 0 to 100 stock
// range): a compile-allocated 627 types and stores.
#[test]
fn the_boots_id_control_takes_the_records_bound() {
    let path = fixture(PesVersion::Pes21, "boots_id_627");
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Boots ID")
    });
    let boots = right_of_label(&nodes, "Boots ID", Role::SpinButton)
        .first()
        .copied()
        .expect("the Boots ID control");
    click(&mut window, boots, &mut view);
    window.frame(vec![egui::Event::Text("627".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .appearance
            .boots_id,
        627,
        "a compile-allocated id stores"
    );
}

// Ticking Base copy sets the seven gameplay edit flags in the same edit
// (a copy's sections count as edited); unticking clears `base_copy`
// alone. On PES 18, the last version with the flag.
#[test]
fn ticking_base_copy_sets_the_seven_gameplay_flags_one_undoable_edit() {
    let path = fixture(PesVersion::Pes18, "base_copy_flags");
    // The eight cleared, so each set bit is the tick's doing.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes18)).unwrap();
    {
        let flags = &mut file.player_mut(70101).unwrap().edit_flags;
        flags.base_copy = Some(false);
        flags.abilities = false;
        flags.com_styles = false;
        flags.motion = false;
        flags.playable_positions = false;
        flags.registered_position = false;
        flags.skills = false;
        flags.playing_style = false;
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);
    let flags_of = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .edit_flags
            .clone()
    };
    let all_set = |view: &SaveView| {
        let flags = flags_of(view);
        flags.base_copy == Some(true)
            && flags.abilities
            && flags.com_styles
            && flags.motion
            && flags.playable_positions
            && flags.registered_position
            && flags.skills
            && flags.playing_style
    };
    assert!(!all_set(&view), "premise: the eight start cleared");
    let before = flags_of(&view);
    let base_copy_of = |view: &SaveView| flags_of(view).base_copy;
    let seven_set = |view: &SaveView| {
        let flags = flags_of(view);
        flags.abilities
            && flags.com_styles
            && flags.motion
            && flags.playable_positions
            && flags.registered_position
            && flags.skills
            && flags.playing_style
    };

    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::CheckBox && node.label() == Some("Base copy")
    });
    let checkbox = nodes
        .iter()
        .find(|node| node.role() == Role::CheckBox && node.label() == Some("Base copy"))
        .expect("the Base copy flag on PES 18");
    assert!(!checkbox.is_disabled(), "PES 18 holds the flag");
    click(&mut window, checkbox, &mut view);
    assert!(all_set(&view), "the tick sets base copy and the seven");

    // One undo step restores all eight.
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
    assert_eq!(flags_of(&view), before, "the undo restores all eight");

    // Ticked again, then unticked: `base_copy` alone clears.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let checkbox = nodes
        .iter()
        .find(|node| node.role() == Role::CheckBox && node.label() == Some("Base copy"))
        .expect("the Base copy flag");
    click(&mut window, checkbox, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let checkbox = nodes
        .iter()
        .find(|node| node.role() == Role::CheckBox && node.label() == Some("Base copy"))
        .expect("the Base copy flag");
    click(&mut window, checkbox, &mut view);
    assert_eq!(
        base_copy_of(&view),
        Some(false),
        "the untick clears base copy alone"
    );
    assert!(seven_set(&view), "the seven stay set");
}

// On a version that has no `base_copy` flag (PES 19 and on) the Base copy
// checkbox is drawn disabled in place with `PES 15 to 18 only` on hover —
// the group's gate, like the gated `-` stand-ins.
// SE-GUI-06
#[test]
fn the_base_copy_flag_is_disabled_in_place_on_pes21() {
    let path = fixture(PesVersion::Pes21, "base_copy_gated");
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);

    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::CheckBox && node.label() == Some("Base copy")
    });
    // The group is unlocked, so a disabled box is the version gate, not
    // the lock.
    assert!(
        !right_of_label(&nodes, "Copy from ID", Role::SpinButton)[0].is_disabled(),
        "premise: the group is unlocked"
    );
    let base_copy = nodes
        .iter()
        .find(|node| node.role() == Role::CheckBox && node.label() == Some("Base copy"))
        .expect("the Base copy flag on PES 21");
    assert!(base_copy.is_disabled(), "PES 21 holds no such flag");
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(base_copy), "PES 15 to 18"),
        Some("PES 15 to 18 only".to_owned()),
        "the gate's hover"
    );
}
