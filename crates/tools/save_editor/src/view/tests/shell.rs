use super::*;

// Undo/redo and unsaved changes, the mechanism SE-GUI-03's scenario is
// built on: an edit marks the row while the file stands; Ctrl+S writes it
// and clears the mark.
// SE-GUI-03
#[test]
fn an_edit_marks_the_row_until_ctrl_s_writes_the_save() {
    let path = fixture(PesVersion::Pes21, "edit_then_save");
    let original = std::fs::read(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();

    // 70101's Height is 180: focus the control and type a new one.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let height = nodes
        .iter()
        .find(|node| node.role() == Role::SpinButton && node.value() == Some("180"))
        .expect("the Height control");
    click(&mut window, height, &mut view);
    window.frame(vec![egui::Event::Text("170".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        row(&nodes, "01 CB PLACEHOLDER ●").label(),
        Some("01 CB PLACEHOLDER ●")
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "the file is unchanged until Save"
    );

    // Ctrl+S writes it; the dot goes.
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    row(&nodes, "01 CB PLACEHOLDER");
    assert!(
        !nodes
            .iter()
            .any(|node| node.label().is_some_and(|label| label.contains('●'))),
        "no dirty player"
    );
    let reopened = Session::open(&path, Recorder::default()).unwrap();
    assert_eq!(
        reopened.file().player(70101).unwrap().basic.height,
        170,
        "the reopened file holds the height"
    );
}

// SE-GUI-03
#[test]
fn the_file_menus_save_writes_the_save_and_clears_the_mark() {
    let path = fixture(PesVersion::Pes21, "edit_then_menu_save");
    let original = std::fs::read(&path).unwrap();
    let mut view = SaveView::new();
    view.discover = Box::new(|_| Vec::new());
    view.open(&path);
    let mut edited = player_entry(&view, 70101);
    assert_ne!(edited.basic.height, 170, "premise: a real change");
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    let mut window = Headless::new();
    let nodes = draw(&mut window, &mut view);
    row(&nodes, "01 CB PLACEHOLDER ●");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "the file is unchanged until Save"
    );

    choose(&mut window, &mut view, "File", "Save Ctrl+S");
    let nodes = draw(&mut window, &mut view);
    row(&nodes, "01 CB PLACEHOLDER");
    let reopened = Session::open(&path, Recorder::default()).unwrap();
    assert_eq!(
        reopened.file().player(70101).unwrap().basic.height,
        170,
        "the reopened file holds the height"
    );
}

/// Player 70101's Height and Weight as the session holds them.
fn basics(view: &SaveView) -> (u8, u8) {
    let player = view.session.as_ref().unwrap().file().player(70101).unwrap();
    (player.basic.height, player.basic.weight)
}

/// Two edits made in order through the player view: 70101's Height 180 to
/// 170, then his Weight 80 to 90, two gestures and so two undo entries.
fn two_edits(window: &mut Headless, view: &mut SaveView) {
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let height = nodes
        .iter()
        .find(|node| node.role() == Role::SpinButton && node.value() == Some("180"))
        .expect("the Height control");
    click(window, height, view);
    window.frame(vec![egui::Event::Text("170".to_owned())], |ui| {
        show(ui, view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, view, &|| Vec::new()),
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let weight = nodes
        .iter()
        .find(|node| node.role() == Role::SpinButton && node.value() == Some("80"))
        .expect("the Weight control");
    click(window, weight, view);
    window.frame(vec![egui::Event::Text("90".to_owned())], |ui| {
        show(ui, view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, view, &|| Vec::new()),
    );
    window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    assert_eq!(basics(view), (170, 90));
}

// SE-GUI-04
#[test]
fn undo_and_redo_step_the_edits_back_and_forth() {
    let path = fixture(PesVersion::Pes21, "undo_redo");
    let original = std::fs::read(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    two_edits(&mut window, &mut view);

    // Ctrl+Z takes back the second edit only; Ctrl+Y redoes it.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        basics(&view),
        (170, 80),
        "the weight edit undone, height kept"
    );
    window.frame(vec![key(egui::Key::Y, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(basics(&view), (170, 90), "the weight edit redone");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "the file changes only on Save"
    );
}

/// Whether the Edit menu's Undo and Redo are enabled, the menu opened to
/// read them.
fn history_entries(window: &mut Headless, view: &mut SaveView) -> (bool, bool) {
    let nodes = open_menu(window, view, "Edit", "Undo");
    let enabled = |entry: &str| !node_labeled(&nodes, Role::Button, entry).is_disabled();
    (enabled("Undo Ctrl+Z"), enabled("Redo Ctrl+Y"))
}

// SE-GUI-04
#[test]
fn the_edit_menus_undo_and_redo_step_the_edits_and_wait_for_one() {
    let path = fixture(PesVersion::Pes21, "undo_redo_menu");
    let original = std::fs::read(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    assert_eq!(
        history_entries(&mut window, &mut view),
        (false, false),
        "nothing to undo or redo"
    );
    // A press on the Height control closes the menu before the edits.
    two_edits(&mut window, &mut view);
    assert_eq!(
        history_entries(&mut window, &mut view),
        (true, false),
        "Undo enabled after two edits, Redo not"
    );

    choose(&mut window, &mut view, "Edit", "Undo");
    assert_eq!(
        basics(&view),
        (170, 80),
        "the weight edit undone, height kept"
    );
    assert_eq!(
        history_entries(&mut window, &mut view),
        (true, true),
        "Redo enabled after the Undo"
    );
    choose(&mut window, &mut view, "Edit", "Redo");
    assert_eq!(basics(&view), (170, 90), "the weight edit redone");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "the file changes only on Save"
    );
}

// Changes from one widget merge into one undo entry while the gesture
// lasts, so a drag is one undo step, not one per frame.
#[test]
fn a_drag_on_a_control_is_one_undo_entry() {
    let path = fixture(PesVersion::Pes21, "drag_one_entry");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let height = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .basic
            .height
    };

    // Drag the Height control over several frames.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let control = nodes
        .iter()
        .find(|node| node.role() == Role::SpinButton && node.value() == Some("180"))
        .expect("the Height control");
    let at = centre(control);
    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    for step in 1..=5 {
        window.frame(
            vec![egui::Event::PointerMoved(egui::pos2(
                at.x + 3.0 * step as f32,
                at.y,
            ))],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
    }
    window.frame(pointer_at(egui::pos2(at.x + 15.0, at.y), false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let dragged = height(&view);
    assert_ne!(dragged, 180, "the drag changed the height");
    // One Ctrl+Z undoes the whole drag: the stack held one entry and the
    // height is back to before the drag.
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one entry");
    assert_eq!(height(&view), 180, "the one undo restores it");
}

// A second gesture on the same control, over a second after the first, is
// its own undo entry.
#[test]
fn a_second_gesture_on_the_same_control_is_its_own_entry() {
    let path = fixture(PesVersion::Pes21, "two_gestures");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let at = centre(
        nodes
            .iter()
            .find(|node| node.role() == Role::SpinButton && node.value() == Some("180"))
            .expect("the Height control"),
    );
    let drag = |window: &mut Headless, view: &mut SaveView, at: egui::Pos2| {
        window.frame(pointer_at(at, true), |ui| show(ui, view, &|| Vec::new()));
        window.frame(
            vec![egui::Event::PointerMoved(egui::pos2(at.x + 30.0, at.y))],
            |ui| show(ui, view, &|| Vec::new()),
        );
        window.frame(pointer_at(egui::pos2(at.x + 30.0, at.y), false), |ui| {
            show(ui, view, &|| Vec::new())
        });
    };
    drag(&mut window, &mut view, at);
    // Past the run's one-second pause: a second gesture within it would
    // join the first's run instead.
    idle_frames(&mut window, &mut view, 61);
    drag(&mut window, &mut view, at);
    assert_eq!(undo_entries(&mut window, &mut view), 2, "two entries");
}

// SE-GUI-21 (the scenario is manual; this is its logic)
#[test]
fn save_asks_the_edit_mode_question_while_the_saves_own_pes_runs() {
    let path = fixture(PesVersion::Pes21, "save_question");
    let original = std::fs::read(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    // An edit, so the question leaves something behind.
    let mut edited = view
        .session
        .as_ref()
        .unwrap()
        .file()
        .player(70101)
        .unwrap()
        .clone();
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    let running_own = || {
        vec![RunningPes {
            version: PesVersion::Pes21,
            exe: None,
        }]
    };
    let mut window = Headless::new();

    // The window's Save opens the question; its Cancel writes nothing.
    assert!(!save(&mut view, &running_own), "not written yet");
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        pairs.iter().any(|(_, node)| {
            node.value()
                .is_some_and(|value| value.contains("PES 2021 is running"))
        }),
        "the question names the game"
    );
    click(
        &mut window,
        prompt::modal_button(&pairs, "PES 2021 is running", "Cancel"),
        &mut view,
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "Cancel writes nothing"
    );
    assert!(
        view.session.as_ref().unwrap().is_dirty(),
        "the cancelled save did not write"
    );
    assert!(
        view.edit_mode_question.is_none(),
        "Cancel closes the question"
    );

    // Its Save writes, the dialog having said how to keep the write.
    assert!(!save(&mut view, &running_own));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        prompt::modal_button(&pairs, "PES 2021 is running", "Save"),
        &mut view,
    );
    assert!(view.edit_mode_question.is_none());
    assert!(!view.session.as_ref().unwrap().is_dirty(), "the save wrote");
    let reopened = Session::open(&path, Recorder::default()).unwrap();
    assert_eq!(
        reopened.file().player(70101).unwrap().basic.height,
        170,
        "the reopened file holds the edit"
    );

    // A game of another version writes with no question.
    let mut edited = player_entry(&view, 70101);
    edited.basic.age = 33;
    view.session.as_mut().unwrap().edit_player(5, edited);
    let running_other = || {
        vec![RunningPes {
            version: PesVersion::Pes19,
            exe: None,
        }]
    };
    assert!(save(&mut view, &running_other), "the write went through");
    assert!(view.edit_mode_question.is_none(), "no question asked");
}

// Undoing back to the saved state clears the unsaved dot, since it is the
// difference from the file as opened or last saved, not the undo stack's
// length.
#[test]
fn undoing_back_to_the_saved_state_clears_the_dot() {
    let path = fixture(PesVersion::Pes21, "undo_clears_dot");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let height = nodes
        .iter()
        .find(|node| node.role() == Role::SpinButton && node.value() == Some("180"))
        .expect("the Height control");
    click(&mut window, height, &mut view);
    window.frame(vec![egui::Event::Text("170".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    row(&nodes, "01 CB PLACEHOLDER ●");
    // Undoing the only edit clears the dot.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    row(&nodes, "01 CB PLACEHOLDER");
    assert!(
        !nodes
            .iter()
            .any(|node| node.label().is_some_and(|label| label.contains('●'))),
        "the dot is gone"
    );
}

// The more specific shortcut answers first: Ctrl+Shift+Z is Redo, not
// Undo.
#[test]
fn ctrl_shift_z_redoes_not_undoes() {
    let path = fixture(PesVersion::Pes21, "shift_z_redo");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let height = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .basic
            .height
    };
    // One edit: Height 180 to 170.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let height_control = nodes
        .iter()
        .find(|node| node.role() == Role::SpinButton && node.value() == Some("180"))
        .expect("the Height control");
    click(&mut window, height_control, &mut view);
    window.frame(vec![egui::Event::Text("170".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(height(&view), 170);

    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(height(&view), 180, "Ctrl+Z undoes");
    window.frame(
        vec![key(
            egui::Key::Z,
            egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
            true,
        )],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(height(&view), 170, "Ctrl+Shift+Z redoes");
}

// Ctrl+Z while a text field holds the keyboard undoes nothing: the
// field's own undo is egui's.
// SE-GUI-62
#[test]
fn ctrl_z_while_a_text_field_holds_the_keyboard_undoes_nothing() {
    let path = fixture(PesVersion::Pes21, "keys_typing");
    let mut view = SaveView::new();
    view.open(&path);
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    let revision = view.session.as_ref().unwrap().revision();
    let mut window = Headless::new();

    // Focus the Name field, then Ctrl+Z.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let field = name_field(&nodes);
    click(&mut window, field, &mut view);
    let mut wants = false;
    window.frame(Vec::new(), |ui| {
        wants = ui.ctx().egui_wants_keyboard_input();
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(wants, "premise: the field holds the keyboard");
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        view.session.as_ref().unwrap().revision(),
        revision,
        "the last edit is not undone"
    );
    assert_eq!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .basic
            .height,
        170
    );
}

// A rating cell focused by Tab keeps the keyboard after a Space press
// settles its edit, and it is no text field, so Ctrl+Z there is the
// editor's undo.
// SE-GUI-62
#[test]
fn ctrl_z_while_a_rating_cell_holds_the_keyboard_undoes_the_step() {
    let path = fixture(PesVersion::Pes17, "keys_rating_cell");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(734, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    let player_id = team_entry(&view, 734).roster[0].player_id;
    let playable = |view: &SaveView| player_entry(view, player_id).positions.playable;
    assert_eq!(playable(&view)[2], 0, "premise: LWF stores C");

    // The LWF cell's A rating, focused as a Tab lands it (the AccessKit
    // Focus, which leaves the keyboard on it as Tab would), then Space.
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let nodes: Vec<accesskit::Node> = pairs.iter().map(|(_, node)| node.clone()).collect();
    let rating_bounds = rating_buttons(&nodes, "LWF")[2]
        .bounds()
        .expect("the A rating's bounds");
    let rating_id = pairs
        .iter()
        .find(|(_, node)| {
            node.role() == Role::Button
                && node.label() == Some("A")
                && node.bounds() == Some(rating_bounds)
        })
        .map(|(id, _)| *id)
        .expect("the A rating's AccessKit id");
    action_focus(&mut window, &mut view, rating_id);
    window.frame(
        vec![
            key(egui::Key::Space, egui::Modifiers::NONE, true),
            key(egui::Key::Space, egui::Modifiers::NONE, false),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        playable(&view)[2],
        2,
        "premise: the Space press stored the A rating"
    );

    // The cell still holds the keyboard, as no text field.
    let mut wants = false;
    let mut text_edit = true;
    window.frame(Vec::new(), |ui| {
        wants = ui.ctx().egui_wants_keyboard_input();
        text_edit = ui.ctx().text_edit_focused();
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(wants, "premise: the cell holds the keyboard");
    assert!(!text_edit, "premise: the cell is no text field");
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(playable(&view)[2], 0, "the rating is back as it was");
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "Undo is disabled"
    );

    // Ctrl+Y, still with the cell focused, puts the step back.
    window.frame(vec![key(egui::Key::Y, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(playable(&view)[2], 2, "the step is back");
}

// `Tool::view` draws the window through the tool impl (a settings error
// counts as the defaults).
#[test]
fn the_tool_view_draws_the_window() {
    use studio_core::StudioTool;

    let dir = crate::test_support::scratch("tool_view_window");
    let (ctx, _events_rx, _requests_rx) = tool_context(&dir, studio_core::Settings::default());
    let mut tool = crate::Tool::new();
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("No save is open.")),
        "the window drew"
    );
}

// No view draws the open save's path: `title_detail` hands the path to
// the idle window title, so the view draws neither it nor the file's
// name, and the toolbar keeps the unsaved marker alone.
#[test]
fn the_save_path_rides_the_window_title_not_the_view() {
    use studio_core::StudioTool;

    let path = fixture(PesVersion::Pes21, "title_path");
    let dir = crate::test_support::scratch("title_path_ctx");
    let (ctx, _events_rx, _requests_rx) = tool_context(&dir, studio_core::Settings::default());
    let mut tool = crate::Tool::new();
    assert_eq!(tool.title_detail(), None, "no save open");
    tool.view.open(&path);
    assert_eq!(
        tool.title_detail(),
        Some(path.display().to_string()),
        "the open save's path"
    );

    let mut window = Headless::new();
    // The file name is part of the whole path, so this also rules out the path.
    let file_name = path.file_name().unwrap().to_str().unwrap().to_owned();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(
        !nodes.iter().any(|node| {
            node.label().is_some_and(|text| text.contains(&file_name))
                || node.value().is_some_and(|text| text.contains(&file_name))
        }),
        "no node carries the save's path or file name"
    );
    assert!(
        !nodes
            .iter()
            .any(|node| node.value() == Some("\u{2022} unsaved changes")),
        "a clean session draws no unsaved marker"
    );

    // A dirty session marks the toolbar with the marker alone.
    let mut edited = player_entry(&tool.view, 70101);
    edited.basic.height = 170;
    tool.view.session.as_mut().unwrap().edit_player(1, edited);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("\u{2022} unsaved changes")),
        "the dirty session's marker is drawn"
    );
}

// The refusal dialog closes on Escape as on OK (`view::refusal_dialog`).
#[test]
fn the_refusal_dialog_dismisses_on_escape() {
    let good = fixture(PesVersion::Pes21, "escape_refusal");
    let mut view = SaveView::new();
    view.open(&good);
    let bad = good.with_file_name("not_a_save.bin");
    std::fs::write(&bad, b"this is no save").unwrap();
    view.open(&bad);
    assert!(view.refusal.is_some());
    let mut window = Headless::new();
    // One frame for the modal to register as the top one, then Escape.
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert!(view.refusal.is_none(), "Escape closes it");
}

/// A dialog the view does not own, drawn after it as the shell's Settings
/// and About are, holds the editor's shortcuts as the editor's own
/// dialogs do: PageDown moves no selection and Ctrl+S writes nothing.
#[test]
fn a_dialog_drawn_after_the_view_holds_its_shortcuts() {
    let path = fixture(PesVersion::Pes21, "shell_dialog_holds_keys");
    let original = std::fs::read(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let selected = view.selected;
    let (_, _, player) = selected_player(&view).expect("premise: a player row is selected");
    edit_player(view.session.as_mut().unwrap(), player, 1, |entry| {
        entry.basic.height += 1;
    });
    assert!(view.is_dirty(), "premise: an unsaved edit");
    let mut window = Headless::new();
    // The view, then a dialog of the shell's over it.
    let with_dialog = |window: &mut Headless, view: &mut SaveView, events| {
        window.frame(events, |ui| {
            show(ui, view, &|| Vec::new());
            studio_core::widgets::modal(
                ui.ctx(),
                egui::Id::unique("shell_settings"),
                "Settings",
                |ui| {
                    ui.label("A setting");
                },
            );
        });
    };
    with_dialog(&mut window, &mut view, Vec::new());
    with_dialog(
        &mut window,
        &mut view,
        vec![key(egui::Key::PageDown, egui::Modifiers::NONE, true)],
    );
    assert_eq!(view.selected, selected, "PageDown moved nothing");
    with_dialog(
        &mut window,
        &mut view,
        vec![key(egui::Key::S, egui::Modifiers::CTRL, true)],
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "Ctrl+S wrote nothing"
    );
    assert!(view.is_dirty(), "the edit is still unsaved");
}

// The open path's refusal: a file that is no save shows the
// `Session::open` error and the open save is untouched.
#[test]
fn an_unreadable_file_is_refused_and_keeps_the_open_save() {
    let good = fixture(PesVersion::Pes21, "refused_open");
    let mut view = SaveView::new();
    view.open(&good);
    let bad = good.with_file_name("not_a_save.bin");
    std::fs::write(&bad, b"this is no save").unwrap();
    view.open(&bad);

    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let message = format!("{}: cannot open the save", bad.display());
    assert!(
        nodes
            .iter()
            .any(|node| node.value().is_some_and(|value| value.contains(&message))),
        "the refusal names the file: {message}"
    );
    // The open save is untouched: its rows still list.
    assert!(
        nodes
            .iter()
            .any(|node| node.label() == Some("01 CB PLACEHOLDER")),
        "the session still shows its players"
    );
}

// The panes reach two points into the shell's bottom margin and end 6
// over the status bar, the draft's padding under the view: the detail
// pane's frame fills the pane to that edge.
#[test]
fn the_panes_end_six_over_the_status_bar() {
    let path = fixture(PesVersion::Pes21, "pane_bottom");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let frame = pane_frame(&shapes, window.screen);
    assert!(
        (window.screen.y - frame.max.y - 6.0).abs() <= 0.5,
        "the detail pane's frame ends 6 over the status bar ({frame:?})"
    );
}

// A save open under any selector value publishes no standing notice and the
// selector's setting is untouched: the editor reads the selector for nothing.
#[test]
fn an_open_save_publishes_no_standing_notice_under_any_selector() {
    use studio_core::StudioTool;

    let path = fixture(PesVersion::Pes21, "no_standing_notice");
    let (ctx, settings, _events_rx, _requests_rx) = crate::test_support::shared_tool_context(
        path.parent().unwrap(),
        studio_core::Settings::default(),
    );
    let mut tool = crate::Tool::new();
    assert_eq!(tool.standing_notice(&ctx), None, "no save is open");
    tool.view.open(&path);
    for version in PesVersion::ALL {
        settings.lock().unwrap().common.pes_version = version;
        assert_eq!(
            tool.standing_notice(&ctx),
            None,
            "the selector on {version} publishes nothing"
        );
    }
}
