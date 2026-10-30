//! The unsaved-changes prompt (RE-GUI-15): each of the four ways out —
//! a window close, an Open, New from template, a template pick — meets
//! the prompt on a dirty document, and each of its three answers does
//! what it says.

use studio_core::{ShellRequest, StudioTool};

use super::*;

/// `test`'s scratch folder, a context and a headless window.
fn prompt_setup(test: &str) -> (PathBuf, ToolContext, Headless) {
    let dir = crate::test_support::scratch(test);
    let (ctx, _events) = crate::test_support::context();
    (dir, ctx, Headless::new())
}

/// A document with unsaved changes over `dir`'s `edited.4ccrules` (the
/// file exists for the Save answers): the name edited to "Edited".
fn dirty_view(ctx: &ToolContext, dir: &Path) -> RulesetView {
    let file = write_file(dir, "edited.4ccrules", &valid_file());
    let mut view = RulesetView::new();
    open_path(&mut view, ctx, &file);
    edit_ruleset(&mut view, |ruleset| ruleset.name = "Edited".to_string());
    assert!(
        view.document.as_ref().expect("open").is_dirty(),
        "premise: unsaved changes"
    );
    view
}

/// Whether the prompt is up.
fn prompt_drawn(nodes: &[accesskit::Node]) -> bool {
    drawn(nodes, "Save changes to Edited?")
}

/// `menu`'s `item` clicked, the prompt asserted up.
fn menu_pick(
    window: &mut Headless,
    view: &mut RulesetView,
    ctx: &ToolContext,
    menu: &str,
    item: &str,
) {
    menu_click(window, view, ctx, menu, item);
    let frame = window.draw(view, ctx);
    assert!(prompt_drawn(&frame.nodes), "the prompt is up");
}

/// The prompt's `answer` clicked.
fn answer(window: &mut Headless, view: &mut RulesetView, ctx: &ToolContext, answer: &str) {
    let frame = window.draw(view, ctx);
    assert!(prompt_drawn(&frame.nodes), "the prompt is up");
    click_labeled(window, &frame.nodes, answer, view, ctx);
}

/// `pick` on a fresh dirty document, then `answer`: the view after.
fn answered(
    window: &mut Headless,
    ctx: &ToolContext,
    dir: &Path,
    pick: impl Fn(&mut Headless, &mut RulesetView, &ToolContext),
    button: &str,
) -> RulesetView {
    let mut view = dirty_view(ctx, dir);
    pick(window, &mut view, ctx);
    answer(window, &mut view, ctx, button);
    view
}

/// `label` picked on the modal start page (opened over a fresh dirty
/// document), then `button`: the view after.
fn answered_start(
    window: &mut Headless,
    ctx: &ToolContext,
    dir: &Path,
    label: &str,
    button: &str,
) -> RulesetView {
    let mut view = dirty_view(ctx, dir);
    open_start_modal(window, &mut view, ctx);
    let frame = window.draw(&mut view, ctx);
    click_prefix(window, &frame.nodes, label, &mut view, ctx);
    let frame = window.draw(&mut view, ctx);
    assert!(prompt_drawn(&frame.nodes), "the prompt is up");
    answer(window, &mut view, ctx, button);
    view
}

/// `label` picked on the modal start page prompts; each of its three
/// answers does what it says, `name` the document after Discard.
fn pick_answers(window: &mut Headless, ctx: &ToolContext, dir: &Path, label: &str, name: &str) {
    let view = answered_start(window, ctx, dir, label, "Cancel");
    assert_name(&view, "Edited");
    assert!(view.document.as_ref().unwrap().is_dirty());
    assert!(view.pending_action.is_none(), "Cancel kept everything");
    assert!(view.start_open, "Cancel keeps the start page up");
    let view = answered_start(window, ctx, dir, label, "Discard");
    assert_name(&view, name);
    assert!(!view.start_open, "Discard went on past the start page");
    let view = answered_start(window, ctx, dir, label, "Save");
    assert_saved(dir, "edited.4ccrules", "Edited");
    assert_name(&view, name);
    assert!(!view.start_open, "Save went on past the start page");
}

/// A click on the node `nodes` holds labeled `label`, on `tool`'s view
/// (the `hold_close` leg drives `Tool::view` as the shell does).
fn click_labeled_tool(
    window: &mut Headless,
    nodes: &[accesskit::Node],
    label: &str,
    tool: &mut crate::Tool,
    ctx: &ToolContext,
) {
    let at = centre(node(nodes, label));
    window.frame(pointer_at(at, true), |ui| tool.view(ui, ctx));
    window.frame(pointer_at(at, false), |ui| tool.view(ui, ctx));
}

// RE-GUI-15
#[test]
fn a_window_close_prompts_and_each_answer_does_what_it_says() {
    let dir = crate::test_support::scratch("prompt_close");
    let (ctx, _events, requests) = crate::test_support::context_requests();
    let mut window = Headless::new();

    // Cancel keeps everything: the document as it was, no close asked.
    let mut tool = crate::Tool::new();
    blank_dirty(&mut tool.view);
    assert!(StudioTool::hold_close(&mut tool), "the close is held");
    let frame = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    click_labeled_tool(&mut window, &frame.nodes, "Cancel", &mut tool, &ctx);
    assert!(tool.view.pending_action.is_none());
    assert!(
        tool.view.document.as_ref().expect("kept").is_dirty(),
        "cancel keeps the document as it was"
    );
    assert!(
        requests.try_recv().is_err(),
        "no close is asked after Cancel"
    );

    // Discard goes on: the shell is asked to close, and the dropped
    // document lets the re-asked hold_close through.
    assert!(
        StudioTool::hold_close(&mut tool),
        "a later close asks again"
    );
    let frame = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    click_labeled_tool(&mut window, &frame.nodes, "Discard", &mut tool, &ctx);
    assert_eq!(
        requests.try_recv().expect("the close request"),
        ShellRequest::Close
    );
    assert!(
        !StudioTool::hold_close(&mut tool),
        "the re-asked close is not held"
    );

    // Save writes it first (Save as for a never-saved document) and
    // goes on.
    let mut tool = crate::Tool::new();
    blank_dirty(&mut tool.view);
    let saved = dir.join("kept.4ccrules");
    tool.view.save_as_dialog = save_as_picking(saved);
    assert!(StudioTool::hold_close(&mut tool));
    let frame = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    click_labeled_tool(&mut window, &frame.nodes, "Save", &mut tool, &ctx);
    assert_saved(&dir, "kept.4ccrules", "Edited");
    let sent: Vec<ShellRequest> = requests.try_iter().collect();
    assert!(
        sent.contains(&ShellRequest::Close),
        "the close is asked after the saved write (the settings write precedes it): {sent:?}"
    );
}

// RE-GUI-15
/// A tier id typed and not committed is an unsaved change: on an
/// otherwise clean document, the window's close is held and prompts.
#[test]
fn a_typed_tier_id_holds_the_close() {
    let (ctx, _events, _path, view) = official_opened("prompt_typed_id");
    let mut window = Headless::sized(1800.0, 900.0);
    let mut tool = crate::Tool::new();
    tool.view = view;
    tool.view.section = Section::Tiers;
    super::tiers::click_role(&mut window, &mut tool.view, &ctx, Role::TextInput, "gold");
    super::tiers::type_replacing(&mut window, &mut tool.view, &ctx, "royal");
    let document = tool.view.document.as_ref().expect("open");
    assert!(
        document.ruleset.tiers[0].id == "gold" && !document.is_dirty(),
        "premise: the id typed, not committed, the document clean"
    );
    assert!(StudioTool::hold_close(&mut tool), "the close is held");
    assert!(
        matches!(tool.view.pending_action, Some(Pending::Close)),
        "the prompt is pending for the close"
    );
}

// RE-GUI-15
#[test]
fn a_save_the_member_cancels_keeps_the_prompt_and_the_document() {
    let dir = crate::test_support::scratch("prompt_save_cancelled");
    let other = write_file(&dir, "other.4ccrules", &valid_file());

    // File > Open… on a dirty, never-saved document, the member
    // cancelling the Save as dialog: the prompt stays up, the document
    // kept and dirty.
    let (ctx, _events, mut window, mut view) = fresh_blank();
    click_drawn_nth(&mut window, &mut view, &ctx, "Attacking Prowess", 1);
    // The member cancels the Save as dialog; the suggested name is the
    // ruleset's name slugged.
    view.save_as_dialog = Box::new(|_, suggested| {
        assert_eq!(
            suggested, "new_ruleset.4ccrules",
            "Save as suggests the ruleset's name slugged"
        );
        None
    });
    view.open_dialog = Box::new(move |_| Some(other.clone()));
    menu_click(&mut window, &mut view, &ctx, "File", "Open…");
    window.draw(&mut view, &ctx);
    assert!(view.pending_action.is_some(), "the dirty document prompts");
    let frame = window.draw(&mut view, &ctx);
    assert!(drawn(&frame.nodes, "Save changes to New ruleset?"));
    click_labeled(&mut window, &frame.nodes, "Save", &mut view, &ctx);
    assert!(
        view.pending_action.is_some(),
        "a save that did not happen keeps the prompt"
    );
    let document = view.document.as_ref().expect("the document kept");
    assert_eq!(
        document.ruleset.name, "New ruleset",
        "the open went nowhere"
    );
    assert!(document.is_dirty(), "still dirty");

    // The same behind the window's close: the prompt stays, no close is
    // asked.
    let (ctx, _events, requests) = crate::test_support::context_requests();
    let mut tool = crate::Tool::new();
    blank_dirty(&mut tool.view);
    tool.view.save_as_dialog = Box::new(|_, _| None);
    assert!(StudioTool::hold_close(&mut tool), "the close is held");
    let frame = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    click_labeled_tool(&mut window, &frame.nodes, "Save", &mut tool, &ctx);
    assert!(
        tool.view.pending_action.is_some(),
        "a save that did not happen keeps the prompt"
    );
    assert!(
        tool.view
            .document
            .as_ref()
            .expect("the document kept")
            .is_dirty(),
        "still dirty"
    );
    assert!(
        requests.try_recv().is_err(),
        "no close is asked while the prompt stays"
    );
}

// RE-GUI-15
#[test]
fn an_open_prompts_and_each_answer_does_what_it_says() {
    let (dir, ctx, mut window) = prompt_setup("prompt_open");
    let other = write_file(&dir, "other.4ccrules", &valid_file());

    // Cancel keeps the document as it was.
    let view = answered(
        &mut window,
        &ctx,
        &dir,
        |w, v, c| menu_pick(w, v, c, "File", "Open…"),
        "Cancel",
    );
    assert_eq!(
        view.document.as_ref().expect("kept").ruleset.name,
        "Edited",
        "cancel keeps the document as it was"
    );
    assert!(view.document.as_ref().unwrap().is_dirty());
    assert!(view.pending_action.is_none(), "Cancel kept everything");

    // Discard goes on: the file dialog follows and the file opens. The
    // dialog starts in the settings' `last_folder` — opening
    // `edited.4ccrules` set it to `dir`, so a different folder proves
    // the argument is the settings', not the fixture's.
    let mut view = dirty_view(&ctx, &dir);
    let start_dir = crate::test_support::scratch("prompt_open_start");
    seed_settings(&ctx, &start_dir, Vec::new());
    view.open_dialog = open_picking(start_dir, other.clone());
    menu_pick(&mut window, &mut view, &ctx, "File", "Open…");
    answer(&mut window, &mut view, &ctx, "Discard");
    assert_eq!(
        view.document.as_ref().expect("opened").ruleset.name,
        "4cc Autumn 26",
        "the picked file opened"
    );

    // Save writes it first, then goes on.
    let view = answered(
        &mut window,
        &ctx,
        &dir,
        |w, v, c| {
            let picked = other.clone();
            v.open_dialog = Box::new(move |_| Some(picked.clone()));
            menu_pick(w, v, c, "File", "Open…");
        },
        "Save",
    );
    assert_saved(&dir, "edited.4ccrules", "Edited");
    assert_eq!(
        view.document.as_ref().expect("opened").ruleset.name,
        "4cc Autumn 26"
    );
}

// The Open the prompt's own save unblocks starts in the folder that save
// chose, not the `last_folder` read before it.
#[test]
fn the_open_after_the_prompts_save_starts_in_the_folder_it_chose() {
    let (dir_a, ctx, mut window) = prompt_setup("prompt_save_start_a");
    let dir_b = crate::test_support::scratch("prompt_save_start_b");
    let other = write_file(&dir_b, "other.4ccrules", &valid_file());
    seed_settings(&ctx, &dir_b, Vec::new());

    // A never-saved document with an edit; File > Open… prompts, the
    // prompt's Save writes to folder A through the stubbed dialog, and
    // the Open that follows starts there.
    let mut view = RulesetView::new();
    blank_dirty(&mut view);
    let saved = dir_a.join("new.4ccrules");
    view.save_as_dialog = save_as_picking(saved);
    view.open_dialog = open_picking(dir_a, other);
    menu_pick(&mut window, &mut view, &ctx, "File", "Open…");
    answer(&mut window, &mut view, &ctx, "Save");
    assert_name(&view, "4cc Autumn 26");
}

// RE-GUI-15
#[test]
fn new_from_template_is_unguarded_but_a_pick_in_it_prompts() {
    let (dir, ctx, mut window) = prompt_setup("prompt_new");

    // On a dirty document File > New from template draws the start page
    // and no prompt (Close returns to the document as it was).
    let mut view = dirty_view(&ctx, &dir);
    open_start_modal(&mut window, &mut view, &ctx);
    let frame = window.draw(&mut view, &ctx);
    assert!(drawn(&frame.nodes, "New from template"));
    assert!(
        !prompt_drawn(&frame.nodes),
        "the page itself is not a way out"
    );

    // Picking a template there meets the prompt; Discard opens it.
    click_prefix(&mut window, &frame.nodes, "4cc Spring 26", &mut view, &ctx);
    let frame = window.draw(&mut view, &ctx);
    assert!(prompt_drawn(&frame.nodes), "the prompt is up");
    click_labeled(&mut window, &frame.nodes, "Discard", &mut view, &ctx);
    assert_eq!(
        view.document.as_ref().expect("opened").ruleset.name,
        "4cc Spring 26"
    );

    // And Close puts the start page away, the document kept as it was.
    let mut view = dirty_view(&ctx, &dir);
    open_start_modal(&mut window, &mut view, &ctx);
    let frame = window.draw(&mut view, &ctx);
    click_labeled(&mut window, &frame.nodes, "Close", &mut view, &ctx);
    assert!(!view.start_open, "Close put the start page away");
    assert_name(&view, "Edited");
    assert!(view.document.as_ref().unwrap().is_dirty());
}

/// The document unchanged behind the modal (the undo entry kept,
/// dirty, nothing written): what dead Ctrl+Z/Y/S leave behind.
fn assert_keys_went_nowhere(view: &RulesetView, file: &Path, before: &[u8]) {
    let document = view.document.as_ref().unwrap();
    assert!(
        !document
            .ruleset
            .stats
            .at_most
            .iter()
            .any(|name| name == "attacking_prowess"),
        "the document is unchanged: no undo, no redo"
    );
    assert!(document.can_undo(), "the undo entry is still there");
    assert!(document.is_dirty(), "and still dirty");
    assert_eq!(std::fs::read(file).unwrap(), before, "Ctrl+S wrote nothing");
}

// RE-GUI-15
#[test]
fn the_keyboard_shortcuts_do_not_act_behind_the_prompt() {
    let (dir, ctx, mut window) = prompt_setup("keys_modal");
    let mut view = dirty_view(&ctx, &dir);
    let file = dir.join("edited.4ccrules");
    let before = std::fs::read(&file).unwrap();
    // One recorded edit, so an undo would have something to take back.
    click_drawn_nth(&mut window, &mut view, &ctx, "Attacking Prowess", 1);
    assert!(
        view.document.as_ref().unwrap().can_undo(),
        "premise: an undoable edit"
    );
    assert!(
        !view
            .document
            .as_ref()
            .unwrap()
            .ruleset
            .stats
            .at_most
            .iter()
            .any(|name| name == "attacking_prowess")
    );
    menu_pick(&mut window, &mut view, &ctx, "File", "Open…");
    // Ctrl+Z, Ctrl+Y and Ctrl+S are all dead while the prompt is up.
    for pressed in [egui::Key::Z, egui::Key::Y, egui::Key::S] {
        ctrl_key(&mut window, &mut view, &ctx, pressed);
    }
    assert_keys_went_nowhere(&view, &file, &before);
}

// RE-GUI-15
#[test]
fn the_keyboard_shortcuts_do_not_act_behind_the_start_page_or_a_dialog() {
    let (dir, ctx, mut window) = prompt_setup("keys_modals");
    let mut view = dirty_view(&ctx, &dir);
    let file = dir.join("edited.4ccrules");
    let before = std::fs::read(&file).unwrap();
    // One recorded edit, so an undo would have something to take back.
    click_drawn_nth(&mut window, &mut view, &ctx, "Attacking Prowess", 1);
    assert!(
        view.document.as_ref().unwrap().can_undo(),
        "premise: an undoable edit"
    );

    // Behind the modal start page (File's "New from template…").
    open_start_modal(&mut window, &mut view, &ctx);
    for pressed in [egui::Key::Z, egui::Key::Y, egui::Key::S] {
        ctrl_key(&mut window, &mut view, &ctx, pressed);
    }
    assert_keys_went_nowhere(&view, &file, &before);

    // Behind a refused open's dialog.
    let frame = window.draw(&mut view, &ctx);
    click_labeled(&mut window, &frame.nodes, "Close", &mut view, &ctx);
    assert!(!view.start_open, "the start page is away");
    let broken = write_file(
        &dir,
        "broken.4ccrules",
        &crate::test_support::parse_error_file(),
    );
    open_path(&mut view, &ctx, &broken);
    assert!(
        matches!(view.dialog, Some(Dialog::OpenRefused { .. })),
        "premise: the refusal dialog is up"
    );
    for pressed in [egui::Key::Z, egui::Key::Y, egui::Key::S] {
        ctrl_key(&mut window, &mut view, &ctx, pressed);
    }
    assert_keys_went_nowhere(&view, &file, &before);
}

// RE-GUI-15
#[test]
fn a_recent_file_pick_prompts_and_each_answer_does_what_it_says() {
    let (dir, ctx, mut window) = prompt_setup("prompt_recent");
    // The recent file the list offers, seeded first; each leg's open of
    // `edited.4ccrules` then adds its own entry above it.
    let other = write_file(&dir, "other.4ccrules", &valid_file());
    seed_settings(
        &ctx,
        Path::new(""),
        vec![other.to_string_lossy().into_owned()],
    );

    pick_answers(&mut window, &ctx, &dir, "other.4ccrules", "4cc Autumn 26");
}

// RE-GUI-15
#[test]
fn a_template_pick_prompts_and_each_answer_does_what_it_says() {
    let (dir, ctx, mut window) = prompt_setup("prompt_template");

    pick_answers(&mut window, &ctx, &dir, "4cc Spring 26", "4cc Spring 26");
}

// The prompt's title draws in the window-title style (the drafts'
// `.window .title`): 14 points in the strong colour, not `Heading`'s 15,
// which a page's heading keeps.
#[test]
fn the_prompts_title_is_14_points_and_strong() {
    let (dir, ctx, mut window) = prompt_setup("prompt_title_style");
    let mut view = dirty_view(&ctx, &dir);
    menu_pick(&mut window, &mut view, &ctx, "File", "Open…");
    let frame = window.draw(&mut view, &ctx);
    let title = frame
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == "Save changes to Edited?" => {
                Some(shape)
            }
            _ => None,
        })
        .next()
        .expect("the prompt's title is painted");
    assert_eq!(
        title.galley.job.sections[0].format.font_id.size, 14.0,
        "the drafts' .window .title"
    );
    let strong = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .strong_text_color();
    assert_eq!(
        title.galley.job.sections[0].format.color, strong,
        "the title's strong colour"
    );
}

// The drafts' window: the prompt's last button, Cancel, stands at its right,
// inside the stroke and the 12-point margin.
#[test]
fn the_prompts_last_button_stands_at_its_right() {
    let (dir, ctx, mut window) = prompt_setup("prompt_buttons_right");
    let mut view = dirty_view(&ctx, &dir);
    menu_pick(&mut window, &mut view, &ctx, "File", "Open…");
    let frame = window.draw(&mut view, &ctx);
    let outer = window
        .ctx
        .memory(|memory| memory.area_rect(egui::Id::unique("ruleset_unsaved")))
        .expect("the prompt's modal has an area");
    let cancel = node(&frame.nodes, "Cancel")
        .bounds()
        .expect("Cancel has bounds");
    let right = f64::from(outer.right()) - cancel.x1;
    assert!(
        (right - 13.0).abs() <= 0.5,
        "Cancel's right is 13 inside the prompt's, not {right}"
    );
}
