use super::*;

// SE-GUI-05's last clause (the Save editor becomes the active view on
// the held close) is the shell's own:
// `a_close_is_held_by_the_first_tool_with_unsaved_work_then_the_next`
// (`studio_core::shell`'s tests).
// SE-GUI-05
#[test]
fn a_window_close_prompts_and_each_answer_does_what_it_says() {
    let path = fixture(PesVersion::Pes21, "prompt_close");
    let original = std::fs::read(&path).unwrap();
    let (ctx, requests) = tool_ctx("prompt_close_ctx");
    let mut window = Headless::new();

    // Cancel keeps everything: the session as it was, no close asked.
    let mut tool = dirty_tool(&path);
    assert!(
        StudioTool::hold_close(&mut tool),
        "a dirty session holds the close"
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(prompt_drawn(&nodes), "the prompt is up");
    click_labeled_tool(&mut window, &nodes, "Cancel", &mut tool, &ctx);
    assert!(tool.view.prompt.is_none(), "Cancel closes it");
    assert!(tool.view.is_dirty(), "the session is as it was");
    assert!(
        requests.try_recv().is_err(),
        "no close is asked after Cancel"
    );

    // Discard drops the session and re-asks: the shell gets the close,
    // and the re-asked hold_close lets it through.
    assert!(
        StudioTool::hold_close(&mut tool),
        "a later close asks again"
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    click_labeled_tool(&mut window, &nodes, "Discard", &mut tool, &ctx);
    assert_eq!(
        requests.try_recv().expect("the close request"),
        ShellRequest::Close
    );
    assert!(
        !StudioTool::hold_close(&mut tool),
        "the re-asked close is not held"
    );

    // Save writes it first, then goes on: the file holds the edit and the
    // close follows.
    let mut tool = dirty_tool(&path);
    assert!(StudioTool::hold_close(&mut tool));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| tool.view(ui, &ctx));
    press_release(
        &mut window,
        centre(modal_button(&pairs, "Unsaved changes", "Save")),
        |ui| tool.view(ui, &ctx),
    );
    assert_eq!(
        requests.try_recv().expect("the close request"),
        ShellRequest::Close
    );
    let reopened = Session::open(&path, Recorder::default()).unwrap();
    assert_eq!(
        reopened.file().player(70101).unwrap().basic.height,
        170,
        "the reopened file holds the edit"
    );
    assert_ne!(std::fs::read(&path).unwrap(), original, "the save wrote");

    // With the probe reporting the save's own PES, Save opens the
    // question instead of writing (the field `Tool::new` fills with a
    // no-game stub under `cargo test`).
    let path = fixture(PesVersion::Pes21, "prompt_close_question");
    let original = std::fs::read(&path).unwrap();
    let mut tool = dirty_tool(&path);
    tool.running = || {
        vec![RunningPes {
            version: PesVersion::Pes21,
            exe: None,
        }]
    };
    assert!(StudioTool::hold_close(&mut tool));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| tool.view(ui, &ctx));
    press_release(
        &mut window,
        centre(modal_button(&pairs, "Unsaved changes", "Save")),
        |ui| tool.view(ui, &ctx),
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(
        nodes.iter().any(|node| node
            .value()
            .is_some_and(|value| value.contains("PES 2021 is running"))),
        "the question opens instead of writing"
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "nothing wrote while the question waits"
    );
    assert!(tool.view.prompt.is_some(), "the prompt stays up behind it");
    assert!(
        requests.try_recv().is_err(),
        "no close is asked while the question waits"
    );
}

// SE-GUI-05
#[test]
fn opening_another_save_prompts_and_each_answer_does_what_it_says() {
    let first = fixture(PesVersion::Pes21, "prompt_open_first");
    let original = std::fs::read(&first).unwrap();
    let other = fixture(PesVersion::Pes21, "prompt_open_second");
    let mut window = Headless::new();

    // A dirty session's Choose file… shows the prompt, not the picker (the
    // test-build default dialog panics if it is reached).
    let mut view = dirty_view(&first);
    choose_file(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(prompt_drawn(&nodes), "the prompt is up");

    // Cancel keeps the first save open and dirty.
    click_label(&mut window, &mut view, "Cancel");
    assert!(view.prompt.is_none(), "Cancel closes it");
    assert_eq!(view.session.as_ref().unwrap().path(), first.as_path());
    assert!(view.is_dirty(), "the first save stays dirty");

    // Discard goes on: the picker follows and the picked save opens; the
    // first file is unchanged on disk.
    let picked = other.clone();
    view.open_dialog = Box::new(move |_| Some(picked.clone()));
    choose_file(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(prompt_drawn(&nodes), "Discard's prompt is up");
    click_label(&mut window, &mut view, "Discard");
    assert_eq!(
        view.session.as_ref().unwrap().path(),
        other.as_path(),
        "the picked save opened"
    );
    assert!(!view.is_dirty(), "a fresh open has no unsaved changes");
    assert_eq!(
        std::fs::read(&first).unwrap(),
        original,
        "the first file is unchanged"
    );
}

// SE-GUI-05
#[test]
fn the_prompts_save_runs_the_window_save_first() {
    let path = fixture(PesVersion::Pes21, "prompt_save_question");
    let original = std::fs::read(&path).unwrap();
    let mut view = dirty_view(&path);
    prompt::open(&mut view, Pending::Close);
    let running_own = || {
        vec![RunningPes {
            version: PesVersion::Pes21,
            exe: None,
        }]
    };
    let mut window = Headless::new();

    // Save opens the question over the prompt; its Cancel keeps the
    // prompt and asks nothing of the shell.
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &running_own));
    click_with(
        &mut window,
        &mut view,
        &running_own,
        modal_button(&pairs, "Unsaved changes", "Save"),
    );
    let pairs = question_nodes_with(&mut window, &mut view, &running_own);
    click_with(
        &mut window,
        &mut view,
        &running_own,
        modal_button(&pairs, "PES 2021 is running", "Cancel"),
    );
    assert!(view.prompt.is_some(), "the prompt stays up");
    assert!(view.edit_mode_question.is_none());
    assert!(!view.request_close, "no close is asked");
    assert_eq!(std::fs::read(&path).unwrap(), original);

    // Its Save writes and goes on: the file holds the edit and the close
    // follows.
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &running_own));
    click_with(
        &mut window,
        &mut view,
        &running_own,
        modal_button(&pairs, "Unsaved changes", "Save"),
    );
    let pairs = question_nodes_with(&mut window, &mut view, &running_own);
    click_with(
        &mut window,
        &mut view,
        &running_own,
        modal_button(&pairs, "PES 2021 is running", "Save"),
    );
    let reopened = Session::open(&path, Recorder::default()).unwrap();
    assert_eq!(
        reopened.file().player(70101).unwrap().basic.height,
        170,
        "the file holds the edit"
    );
    assert!(view.prompt.is_none(), "the prompt is done");
    assert!(view.session.is_none(), "the session dropped");
    assert!(view.request_close, "the close is asked");
}

// SE-GUI-21's window rule: the running-game question says how to keep
// the write, Edit mode open or closed.
#[test]
fn the_question_says_how_to_keep_the_write() {
    let path = fixture(PesVersion::Pes21, "question_text");
    let mut view = dirty_view(&path);
    view.edit_mode_question = Some(EditModeQuestion {
        game: PesVersion::Pes21,
        target: path,
    });
    let mut window = Headless::new();
    let pairs = question_nodes_with(&mut window, &mut view, &|| Vec::new());
    let nodes: Vec<accesskit::Node> = pairs.into_iter().map(|(_, node)| node).collect();
    assert!(
        label_value(
            &nodes,
            "If the game's Edit mode is open, use its Data Management > Load after saving, \
             before you leave Edit mode: leaving it after changing anything there writes the \
             game's own copy over this save."
        ),
        "the open Edit mode's sentence"
    );
    assert!(
        label_value(
            &nodes,
            "If Edit mode is closed, the changes show when you open it."
        ),
        "the closed Edit mode's sentence"
    );
}

// SE-GUI-05: a window close asked while the Open prompt waits retargets
// it — the answer goes toward the close, the picker never runs.
#[test]
fn a_close_asked_while_the_open_prompt_waits_takes_the_answer() {
    let first = fixture(PesVersion::Pes21, "prompt_close_retarget");
    let (ctx, requests) = tool_ctx("prompt_close_retarget_ctx");
    let mut window = Headless::new();

    let mut tool = dirty_tool(&first);
    choose_file(&mut window, &mut tool.view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(prompt_drawn(&nodes), "the Open prompt is up");

    // The close retargets it: Discard drops the session and asks the
    // shell to close — the picker (the test-build stub, a panic if
    // reached) is never called.
    assert!(StudioTool::hold_close(&mut tool), "the close is held");
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    click_labeled_tool(&mut window, &nodes, "Discard", &mut tool, &ctx);
    assert_eq!(
        requests.try_recv().expect("the close request"),
        ShellRequest::Close
    );
    assert!(tool.view.session.is_none(), "the session dropped");
    assert!(
        !StudioTool::hold_close(&mut tool),
        "the re-asked close is not held"
    );
}

// SE-GUI-05: a Save the write refuses leaves the prompt up and shows
// the refusal on top of it (the modals' top layer), its OK clickable.
#[test]
fn a_refused_write_under_the_prompt_shows_the_refusal_on_top() {
    let path = fixture(PesVersion::Pes21, "prompt_refusal_top");
    let mut view = dirty_view(&path);
    prompt::open(&mut view, Pending::Close);
    let mut window = Headless::new();

    // A directory in the save's place: `EditFile::save`'s copy to `.bak`
    // then fails on every OS, so the write refuses and the prompt keeps
    // up.
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        modal_button(&pairs, "Unsaved changes", "Save"),
        &mut view,
    );
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(view.prompt.is_some(), "the failed write keeps the prompt");
    assert!(view.is_dirty(), "and the session stays dirty");
    let ok = dialog_nodes(&pairs, "Save refused")
        .into_iter()
        .find(|node| node.role() == Role::Button && node.label() == Some("OK"))
        .expect("the refusal's OK in the top dialog's subtree");
    click(&mut window, ok, &mut view);
    assert!(
        view.refusal.is_none(),
        "the click reached the refusal's OK, not the backdrop"
    );
    assert!(view.prompt.is_some(), "the prompt stays up behind it");
}

// SE-GUI-05, SE-GUI-21's window rule: a write refused when asked through
// the running-game question keeps the prompt, the session and the close
// unanswered — a failed write is no Discard — and shows the refusal on
// top.
#[test]
fn a_refused_write_under_the_question_keeps_the_prompt_and_shows_the_refusal() {
    let path = fixture(PesVersion::Pes21, "prompt_question_refusal");
    let mut view = dirty_view(&path);
    prompt::open(&mut view, Pending::Close);
    let running_own = || {
        vec![RunningPes {
            version: PesVersion::Pes21,
            exe: None,
        }]
    };
    let mut window = Headless::new();

    // A directory in the save's place: `EditFile::save`'s copy to `.bak`
    // fails on every OS, so the write refuses.
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &running_own));
    click_with(
        &mut window,
        &mut view,
        &running_own,
        modal_button(&pairs, "Unsaved changes", "Save"),
    );
    let pairs = question_nodes_with(&mut window, &mut view, &running_own);
    click_with(
        &mut window,
        &mut view,
        &running_own,
        modal_button(&pairs, "PES 2021 is running", "Save"),
    );

    assert!(
        view.refusal.is_some(),
        "the refused write shows its refusal"
    );
    assert!(view.prompt.is_some(), "the prompt stays up");
    assert!(view.session.is_some(), "the session stays open");
    assert!(view.is_dirty(), "and dirty");
    assert!(!view.request_close, "no close is asked");
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &running_own));
    dialog_nodes(&pairs, "Save refused")
        .into_iter()
        .find(|node| node.role() == Role::Button && node.label() == Some("OK"))
        .expect("the refusal's OK in the top dialog's subtree");
}

// SE-GUI-05: Esc on the prompt is its Cancel — it closes, the session
// untouched and still dirty.
#[test]
fn escape_on_the_prompt_closes_it_and_changes_nothing() {
    let path = fixture(PesVersion::Pes21, "prompt_escape");
    let original = std::fs::read(&path).unwrap();
    let mut view = dirty_view(&path);
    prompt::open(&mut view, Pending::Close);
    let mut window = Headless::new();
    // One frame for the modal to register as the top one, then Escape.
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(view.prompt.is_some(), "premise: the prompt is up");
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert!(view.prompt.is_none(), "Escape closes the prompt");
    let session = view.session.as_ref().expect("the session kept");
    assert!(session.is_dirty(), "still dirty");
    assert_eq!(
        session.file().player(70101).unwrap().basic.height,
        170,
        "the edit kept"
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "the file unchanged"
    );
    assert!(!view.request_close, "no close is asked");
}

// SE-GUI-21's window rule: Esc on the running-game question closes it;
// the prompt behind stays, the file unchanged.
#[test]
fn escape_on_the_question_closes_it_and_keeps_the_prompt() {
    let path = fixture(PesVersion::Pes21, "question_escape");
    let original = std::fs::read(&path).unwrap();
    let mut view = dirty_view(&path);
    prompt::open(&mut view, Pending::Close);
    view.edit_mode_question = Some(EditModeQuestion {
        game: PesVersion::Pes21,
        target: path.clone(),
    });
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        view.edit_mode_question.is_some(),
        "premise: the question is up"
    );
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert!(
        view.edit_mode_question.is_none(),
        "Escape closes the question"
    );
    assert!(view.prompt.is_some(), "the prompt stays up");
    assert!(view.is_dirty(), "still dirty");
    assert_eq!(std::fs::read(&path).unwrap(), original, "nothing wrote");
}

// The prompt alone holds every shortcut dead while up (the prompt's own
// buttons are the way out): Ctrl+S saves nothing, Ctrl+Z undoes nothing.
#[test]
fn the_shortcuts_do_not_act_behind_the_unsaved_prompt_alone() {
    let path = fixture(PesVersion::Pes21, "keys_prompt_alone");
    let original = std::fs::read(&path).unwrap();
    let mut view = dirty_view(&path);
    prompt::open(&mut view, Pending::Close);
    let revision = view.session.as_ref().unwrap().revision();
    let mut window = Headless::new();
    for key_event in [egui::Key::S, egui::Key::Z] {
        window.frame(vec![key(key_event, egui::Modifiers::CTRL, true)], |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
    }
    assert!(view.prompt.is_some(), "the prompt stays up");
    assert_eq!(
        view.session.as_ref().unwrap().revision(),
        revision,
        "Ctrl+Z undid nothing"
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "Ctrl+S wrote nothing"
    );
    assert!(view.is_dirty(), "still dirty");
}

// The running-game question alone holds them dead too (the question's
// buttons are the way out).
#[test]
fn the_shortcuts_do_not_act_behind_the_edit_mode_question_alone() {
    let path = fixture(PesVersion::Pes21, "keys_question_alone");
    let original = std::fs::read(&path).unwrap();
    let mut view = dirty_view(&path);
    view.edit_mode_question = Some(EditModeQuestion {
        game: PesVersion::Pes21,
        target: path.clone(),
    });
    let revision = view.session.as_ref().unwrap().revision();
    let mut window = Headless::new();
    for key_event in [egui::Key::S, egui::Key::Z] {
        window.frame(vec![key(key_event, egui::Modifiers::CTRL, true)], |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
    }
    assert!(view.edit_mode_question.is_some(), "the question stays up");
    assert_eq!(
        view.session.as_ref().unwrap().revision(),
        revision,
        "Ctrl+Z undid nothing"
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "Ctrl+S wrote nothing"
    );
}
