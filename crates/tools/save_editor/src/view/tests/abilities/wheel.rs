use super::*;

// ST-WHEEL-01 in the editor: each wheel notch over the registered
// position's combo takes the item below, and the notches of one run are
// one undo step.
#[test]
fn wheel_notches_over_the_registered_position_in_one_go_are_one_undo_step() {
    let path = fixture(PesVersion::Pes21, "wheel_registered");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let registered = |view: &SaveView| player_entry(view, 70101).positions.registered;
    assert_eq!(registered(&view), 1, "premise: 70101 is registered CB");
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let combo = right_of_label(&nodes, "Registered position", Role::ComboBox)
        .first()
        .copied()
        .expect("the registered combo");
    let at = centre(combo);
    // CB, then LB, then RB: the items below it in the list's order.
    for expected in [2, 3] {
        notch(&mut window, &mut view, at, false);
        assert_eq!(
            registered(&view),
            expected,
            "a notch down takes the next item"
        );
    }
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        registered(&view),
        1,
        "one Ctrl+Z puts back CB: the run was one entry"
    );
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "the two notches were the only edits"
    );
}

// ST-WHEEL-04 in the editor: notches over the registered position's combo
// while the Name field holds the keyboard take it from the field — the
// focus move ends the typing's entry — and the two notches of one run are
// another: two undo steps in all.
#[test]
fn wheel_notches_while_the_name_field_holds_the_keyboard_are_their_own_undo_steps() {
    let path = fixture(PesVersion::Pes21, "wheel_after_typing");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let registered = |view: &SaveView| player_entry(view, 70101).positions.registered;
    let name = |view: &SaveView| player_entry(view, 70101).name;
    let focused = |window: &Headless| window.ctx.memory(|m| m.focused());
    assert_eq!(registered(&view), 1, "premise: 70101 is registered CB");
    let name_before = name(&view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, name_field(&nodes), &mut view);
    window.frame(vec![egui::Event::Text("Z".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_ne!(name(&view), name_before, "premise: the typed Z landed");
    // Nothing else was clicked: whatever holds the keyboard is the field.
    assert!(
        focused(&window).is_some(),
        "premise: the Name field holds the keyboard"
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let combo = right_of_label(&nodes, "Registered position", Role::ComboBox)
        .first()
        .copied()
        .expect("the registered combo");
    let at = centre(combo);
    let notch_down = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -1.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: egui::Modifiers::default(),
    };
    window.frame(
        vec![
            egui::Event::PointerMoved(at),
            notch_down.clone(),
            notch_down,
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(focused(&window), None, "the notch took the keyboard");
    // CB, then LB, then RB: the items below it in the list's order. The
    // frames are bounded: the two steps come within a few.
    for _ in 0..8 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    assert_eq!(registered(&view), 3, "two notches down take two items");
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        registered(&view),
        1,
        "one Ctrl+Z undoes the notches' run, the typing stays"
    );
    assert_ne!(name(&view), name_before, "the typing stays");
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        name(&view),
        name_before,
        "the second Ctrl+Z undoes the typing"
    );
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "the typing and the notches' run were the only edits"
    );
}

/// `label`'s stat field scrolled into the pane's clip and the wheel
/// rested, its centre aimed at (the notches' pointer target).
fn aimed_stat(window: &mut Headless, view: &mut SaveView, label: &str) -> egui::Pos2 {
    let nodes = scroll_into_view(window, view, |node| {
        node.role() == Role::Label && node.value() == Some(label)
    });
    let control = *right_of_label(&nodes, label, Role::SpinButton)
        .first()
        .unwrap_or_else(|| panic!("the {label} control"));
    let at = centre(control);
    // The scroll's own wheel leaves the pane's wheel unrested 500 ms:
    // the run's pause is a second, so the rest passes inside it.
    idle_frames(window, view, 31);
    at
}

// SE-GUI-55
#[test]
fn ten_wheel_notches_over_a_stat_in_one_go_are_one_undo_step() {
    let path = fixture(PesVersion::Pes21, "wheel_run_stat");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let speed = |view: &SaveView| player_entry(view, 70101).stats.speed;
    assert_eq!(speed(&view), 77, "premise: Speed is 77");
    let at = aimed_stat(&mut window, &mut view, "Speed");

    for _ in 0..10 {
        notch(&mut window, &mut view, at, true);
    }
    assert_eq!(speed(&view), 87, "ten notches up took Speed to 87");
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(speed(&view), 77, "one Ctrl+Z puts back 77");
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "the ten notches were one entry"
    );
}

// SE-GUI-55
#[test]
fn a_notch_on_another_stat_between_two_on_one_is_three_undo_steps() {
    let path = fixture(PesVersion::Pes21, "wheel_run_two_stats");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let attacking = |view: &SaveView| player_entry(view, 70101).stats.attacking_prowess;
    let control = |view: &SaveView| player_entry(view, 70101).stats.ball_control;
    let (attacking_before, control_before) = (attacking(&view), control(&view));

    // One notch on Attacking Prowess, one on Ball Control, one on
    // Attacking Prowess: the second stat ends the first's run.
    let attacking_at = aimed_stat(&mut window, &mut view, "Attacking Prowess");
    let control_at = aimed_stat(&mut window, &mut view, "Ball Control");
    notch(&mut window, &mut view, attacking_at, true);
    notch(&mut window, &mut view, control_at, true);
    notch(&mut window, &mut view, attacking_at, true);
    assert_eq!(
        attacking(&view),
        attacking_before + 2,
        "premise: two notches landed on Attacking Prowess"
    );
    assert_eq!(
        control(&view),
        control_before + 1,
        "premise: one on Ball Control"
    );

    // Three entries, the last undone first.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        attacking(&view),
        attacking_before + 1,
        "the second Attacking Prowess notch undone"
    );
    assert_eq!(control(&view), control_before + 1, "Ball Control stays");
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        control(&view),
        control_before,
        "the Ball Control notch undone"
    );
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(attacking(&view), attacking_before, "the first notch undone");
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "the three notches were three entries"
    );
}

// SE-GUI-55
#[test]
fn two_notches_over_a_second_apart_are_two_undo_steps() {
    let path = fixture(PesVersion::Pes21, "wheel_run_paused");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let speed = |view: &SaveView| player_entry(view, 70101).stats.speed;
    assert_eq!(speed(&view), 77, "premise: Speed is 77");
    let at = aimed_stat(&mut window, &mut view, "Speed");

    notch(&mut window, &mut view, at, true);
    // A second of idle ends the run: the next notch is its own entry.
    idle_frames(&mut window, &mut view, 61);
    notch(&mut window, &mut view, at, true);
    assert_eq!(speed(&view), 79, "two notches took Speed to 79");

    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(speed(&view), 78, "the second notch is its own entry");
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(speed(&view), 77, "the first notch the other");
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "the two notches were two entries"
    );
}

// SE-GUI-55
#[test]
fn a_notch_after_an_undo_and_redo_joins_neither_entry() {
    let path = fixture(PesVersion::Pes21, "wheel_after_undo");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let speed = |view: &SaveView| player_entry(view, 70101).stats.speed;
    assert_eq!(speed(&view), 77, "premise: Speed is 77");
    let at = aimed_stat(&mut window, &mut view, "Speed");

    notch(&mut window, &mut view, at, true);
    assert_eq!(speed(&view), 78, "the notch landed");
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(speed(&view), 77, "undone");
    window.frame(
        vec![key(
            egui::Key::Z,
            egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
            true,
        )],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(speed(&view), 78, "redone");

    // A notch within the pause of the first is a fresh entry: an undo or
    // a redo ends a run, so it merges into neither the undone nor the
    // redone entry.
    notch(&mut window, &mut view, at, true);
    assert_eq!(speed(&view), 79, "the second notch landed");
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        speed(&view),
        78,
        "the second notch is its own entry, not merged into the redone one"
    );
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(speed(&view), 77, "the first notch's entry intact under it");
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "the two notches were two entries"
    );
}
