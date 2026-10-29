use super::*;

// Undo/redo and unsaved changes, the mechanism SE-GUI-03's scenario is
// built on: an edit marks the row while the file stands; Ctrl+S writes it
// and clears the mark.
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
    let reopened = Session::open(&path).unwrap();
    assert_eq!(
        reopened.file().player(70101).unwrap().basic.height,
        170,
        "the reopened file holds the height"
    );
}

// SE-GUI-04
#[test]
fn undo_and_redo_step_the_edits_back_and_forth() {
    let path = fixture(PesVersion::Pes21, "undo_redo");
    let original = std::fs::read(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let basics = |view: &SaveView| {
        let player = view.session.as_ref().unwrap().file().player(70101).unwrap();
        (player.basic.height, player.basic.weight)
    };

    // Height 180 to 170, then Weight 80 to 90: two gestures, two entries.
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
    let weight = nodes
        .iter()
        .find(|node| node.role() == Role::SpinButton && node.value() == Some("80"))
        .expect("the Weight control");
    click(&mut window, weight, &mut view);
    window.frame(vec![egui::Event::Text("90".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(basics(&view), (170, 90));

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

// A second gesture on the same control, after the first ended, is its own
// undo entry.
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

    // Its Save writes with the member's answer.
    assert!(!save(&mut view, &running_own));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        prompt::modal_button(&pairs, "PES 2021 is running", "Save"),
        &mut view,
    );
    assert!(view.edit_mode_question.is_none());
    assert!(!view.session.as_ref().unwrap().is_dirty(), "the save wrote");
    let reopened = Session::open(&path).unwrap();
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

/// The tool, the PES 21 fixture save under `test`'s scratch folder, and a context whose
/// selector reads `selector`, with the handle the test moves the selector through.
fn version_warning_tool(
    test: &str,
    selector: PesVersion,
) -> (
    crate::Tool,
    PathBuf,
    studio_core::ToolContext,
    std::sync::Arc<std::sync::Mutex<studio_core::Settings>>,
) {
    let path = fixture(PesVersion::Pes21, test);
    let mut settings = studio_core::Settings::default();
    settings.common.pes_version = selector;
    let (ctx, settings, _events_rx, _requests_rx) =
        crate::test_support::shared_tool_context(path.parent().unwrap(), settings);
    (crate::Tool::new(), path, ctx, settings)
}

/// The sidebar's selector set to `version`, the write the shell's selector makes.
fn set_selector(settings: &std::sync::Mutex<studio_core::Settings>, version: PesVersion) {
    settings.lock().unwrap().common.pes_version = version;
}

/// The warning `standing_notice` returns, failing when there is none.
#[track_caller]
fn warning_text(tool: &crate::Tool, ctx: &studio_core::ToolContext) -> String {
    use studio_core::StudioTool;

    tool.standing_notice(ctx)
        .expect("the version warning is kept")
        .text
}

// The tool's half of the version warning: when it keeps one and what it says. The
// shell's half (the click writing the setting, a version-change blocker disabling the
// button, another notice ending a shown one) is ST-BAR-07's and ST-BAR-08's.
// SE-GUI-28
#[test]
fn the_version_warning_names_both_versions_until_its_switch() {
    use studio_core::StudioTool;

    let (mut tool, path, ctx, settings) =
        version_warning_tool("version_warning_switch", PesVersion::Pes19);
    assert_eq!(tool.standing_notice(&ctx), None, "no save is open");
    tool.view.open(&path);
    assert_eq!(
        tool.standing_notice(&ctx),
        Some(Notice {
            text: "Although the sidebar is set to PES 2019, this save is from PES 2021, so the \
                   editor will keep treating it as a PES 2021 save."
                .to_owned(),
            kind: NoticeKind::Warning,
            action: Some(NoticeAction {
                label: "Switch to PES 2021".to_owned(),
                effect: ActionEffect::SwitchVersion(PesVersion::Pes21),
            }),
        })
    );
    assert_eq!(
        tool.view.session.as_ref().unwrap().version(),
        PesVersion::Pes21,
        "the save keeps its own version"
    );
    assert_eq!(
        ctx.common().pes_version,
        PesVersion::Pes19,
        "the selector's setting is unchanged"
    );

    // The switch as the shell takes it: the notice ended, then the setting written.
    tool.standing_notice_ended();
    set_selector(&settings, PesVersion::Pes21);
    assert_eq!(tool.standing_notice(&ctx), None, "the switch removes it");
    set_selector(&settings, PesVersion::Pes19);
    assert_eq!(
        tool.standing_notice(&ctx),
        None,
        "the selector set back by hand shows nothing"
    );
}

// SE-GUI-28
#[test]
fn the_version_warning_follows_the_selector_while_it_lives() {
    use studio_core::StudioTool;

    let (mut tool, path, ctx, settings) =
        version_warning_tool("version_warning_follow", PesVersion::Pes19);
    tool.view.open(&path);
    set_selector(&settings, PesVersion::Pes17);
    assert_eq!(
        warning_text(&tool, &ctx),
        version_warning_text(PesVersion::Pes17, PesVersion::Pes21),
        "reworded for the selector's version"
    );
    assert!(warning_text(&tool, &ctx).contains("set to PES 2017"));

    // Matching the save hides it without ending it: the tool is told nothing.
    set_selector(&settings, PesVersion::Pes21);
    assert_eq!(tool.standing_notice(&ctx), None, "gone while they match");
    set_selector(&settings, PesVersion::Pes19);
    assert_eq!(
        warning_text(&tool, &ctx),
        version_warning_text(PesVersion::Pes19, PesVersion::Pes21),
        "back once they part"
    );
}

// SE-GUI-28
#[test]
fn a_dismissed_version_warning_stays_gone_until_another_open() {
    use studio_core::StudioTool;

    let (mut tool, path, ctx, settings) =
        version_warning_tool("version_warning_dismiss", PesVersion::Pes19);
    tool.view.open(&path);
    assert!(
        tool.standing_notice(&ctx).is_some(),
        "the premise: it shows"
    );
    tool.standing_notice_ended();
    for version in [PesVersion::Pes19, PesVersion::Pes17, PesVersion::Pes19] {
        set_selector(&settings, version);
        assert_eq!(
            tool.standing_notice(&ctx),
            None,
            "no selector change brings it back ({version})"
        );
    }
    assert_eq!(ctx.common().pes_version, PesVersion::Pes19);

    tool.view.open(&path);
    assert_eq!(
        warning_text(&tool, &ctx),
        version_warning_text(PesVersion::Pes19, PesVersion::Pes21),
        "another open starts its life over"
    );
}

// SE-GUI-28
#[test]
fn a_save_while_the_versions_match_does_not_end_the_version_warning() {
    use studio_core::StudioTool;

    let (mut tool, path, ctx, settings) =
        version_warning_tool("version_warning_save", PesVersion::Pes21);
    tool.view.open(&path);
    assert_eq!(tool.standing_notice(&ctx), None, "the versions match");
    assert!(save(&mut tool.view, &|| Vec::new()), "the save is written");
    set_selector(&settings, PesVersion::Pes19);
    assert_eq!(
        warning_text(&tool, &ctx),
        version_warning_text(PesVersion::Pes19, PesVersion::Pes21),
        "the warning shows once the selector parts from the save"
    );
}
