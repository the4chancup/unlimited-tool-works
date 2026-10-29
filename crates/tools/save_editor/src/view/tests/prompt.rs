//! The unsaved-changes prompt (SE-GUI-05): a dirty session's close or open
//! meets "Unsaved changes" over the table of what would be written or lost,
//! and each of its three answers does what it says. The Edit-mode question
//! the window's Save asks first while the save's own PES runs (SE-GUI-21's
//! window rule) is covered here too.

use pes_savefile::model::player::STRIPPED_APPEARANCE_ID;
use studio_core::{ShellRequest, StudioTool, ToolContext};

use super::*;
use crate::view::prompt::{self, Pending};

/// A context and its request receiver, `tool`'s `hold_close` legs driven
/// through `Tool::view` as the shell does.
fn tool_ctx(test: &str) -> (ToolContext, crossbeam_channel::Receiver<ShellRequest>) {
    let dir = crate::test_support::scratch(test);
    let (ctx, _events, requests) = tool_context(&dir, studio_core::Settings::default());
    (ctx, requests)
}

/// A `Tool` with `path` open and one edit on player 70101's height.
fn dirty_tool(path: &Path) -> crate::Tool {
    let mut tool = crate::Tool::new();
    tool.view.open(path);
    let mut edited = player_entry(&tool.view, 70101);
    edited.basic.height = 170;
    tool.view
        .session
        .as_mut()
        .expect("the save opened")
        .edit_player(1, edited);
    assert!(tool.view.is_dirty(), "premise: unsaved changes");
    tool
}

/// A `SaveView` with `path` open and one edit on 70101's height.
fn dirty_view(path: &Path) -> SaveView {
    let mut view = SaveView::new();
    view.open(path);
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_player(1, edited);
    assert!(view.is_dirty(), "premise: unsaved changes");
    view
}

/// A click on the node `nodes` holds labeled `label`, on `tool`'s view.
fn click_labeled_tool(
    window: &mut Headless,
    nodes: &[accesskit::Node],
    label: &str,
    tool: &mut crate::Tool,
    ctx: &ToolContext,
) {
    let at = centre(node_labeled(nodes, Role::Button, label));
    window.frame(pointer_at(at, true), |ui| tool.view(ui, ctx));
    window.frame(pointer_at(at, false), |ui| tool.view(ui, ctx));
}

/// A click on the view's `label` button (the prompt's and the question's).
fn click_label(window: &mut Headless, view: &mut SaveView, label: &str) {
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let button = node_labeled(&nodes, Role::Button, label);
    click(window, button, view);
}

/// The nodes of the `Role::Dialog` subtree holding `heading` (an
/// `egui::Modal`'s content, as `frame_ids` reports it): the prompt's own
/// cells, the view's behind-the-backdrop nodes left out.
pub(super) fn dialog_nodes<'a>(
    pairs: &'a [(accesskit::NodeId, accesskit::Node)],
    heading: &str,
) -> Vec<&'a accesskit::Node> {
    let parent: HashMap<accesskit::NodeId, accesskit::NodeId> = pairs
        .iter()
        .flat_map(|(id, node)| node.children().iter().map(move |child| (*child, *id)))
        .collect();
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        pairs.iter().map(|(id, node)| (*id, node)).collect();
    let mut at = pairs
        .iter()
        .find(|(_, node)| node.value() == Some(heading))
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("no heading {heading:?}"));
    let root = loop {
        let node = by_id[&at];
        if node.role() == Role::Dialog {
            break at;
        }
        at = *parent
            .get(&at)
            .unwrap_or_else(|| panic!("no Dialog ancestor of {heading:?}"));
    };
    let mut nodes = Vec::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        let node = by_id[&id];
        stack.extend(node.children());
        nodes.push(node);
    }
    nodes
}

/// The prompt's rows as `(item, old, new)`: each Item cell (a `Label`
/// holding " · " — the `TextRun` twin of every drawn text is skipped)
/// with its row's two value cells in x order.
fn prompt_rows(
    pairs: &[(accesskit::NodeId, accesskit::Node)],
    heading: &str,
) -> Vec<(String, String, String)> {
    let nodes = dialog_nodes(pairs, heading);
    let mut items: Vec<(accesskit::Rect, String)> = nodes
        .iter()
        .filter_map(|node| {
            if node.role() != Role::Label {
                return None;
            }
            let bounds = node.bounds()?;
            let value = node.value()?;
            value.contains(" · ").then(|| (bounds, value.to_owned()))
        })
        .collect();
    items.sort_by(|a, b| {
        (a.0.y0, a.0.x0)
            .partial_cmp(&(b.0.y0, b.0.x0))
            .expect("ordered bounds")
    });
    items
        .iter()
        .map(|(bounds, item)| {
            let mut cells: Vec<(f64, String)> = nodes
                .iter()
                .filter(|node| {
                    node.role() == Role::Label
                        && node.bounds().is_some_and(|b| {
                            (b.y0 - bounds.y0).abs() < 4.0 && b.x0 >= bounds.x0 - 1.0
                        })
                })
                .filter_map(|node| Some((node.bounds()?.x0, node.value()?.to_owned())))
                .collect();
            cells.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("ordered bounds"));
            assert_eq!(
                cells.len(),
                3,
                "the row's Item, Old and New cells: {cells:?}"
            );
            (item.clone(), cells[1].1.clone(), cells[2].1.clone())
        })
        .collect()
}

/// The prompt's painted rect: the modal's `egui::Area` id is the one
/// `prompt::show` registers (the `Role::Dialog` node carries no bounds).
fn prompt_rect(window: &mut Headless) -> egui::Rect {
    window
        .ctx
        .memory(|memory| memory.area_rect(egui::Id::unique("save_editor_unsaved")))
        .expect("the prompt's area")
}

/// Whether the prompt ("Unsaved changes") is up.
fn prompt_drawn(nodes: &[accesskit::Node]) -> bool {
    label_value(nodes, "Unsaved changes") && label_value(nodes, "Item")
}

/// The Edit-mode question's nodes asserted up, `running` the probe.
fn question_nodes_with(
    window: &mut Headless,
    view: &mut SaveView,
    running: &dyn Fn() -> Vec<RunningPes>,
) -> Vec<(accesskit::NodeId, accesskit::Node)> {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, running));
    let nodes: Vec<accesskit::Node> = pairs.iter().map(|(_, node)| node.clone()).collect();
    assert!(
        label_value(&nodes, "PES 2021 is running"),
        "the Edit-mode question is up"
    );
    pairs
}

// SE-GUI-05
#[test]
fn the_prompt_lists_each_changed_field_in_the_views_terms() {
    let path = fixture(PesVersion::Pes21, "prompt_table");
    let mut view = SaveView::new();
    view.open(&path);
    // Height and Speed edited, a third field edited then undone: the
    // table holds the two rows, each under its view's own label.
    let before = player_entry(&view, 70101);
    assert_eq!(before.name, "PLACEHOLDER", "premise: the fixture name");
    let (height, speed) = (before.basic.height, before.stats.speed);
    assert_ne!((height, speed), (170, 90), "premise: real changes");
    let mut edited = before.clone();
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    let mut edited = player_entry(&view, 70101);
    edited.stats.speed = 90;
    view.session.as_mut().unwrap().edit_player(2, edited);
    let mut edited = player_entry(&view, 70101);
    edited.basic.age = 33;
    view.session.as_mut().unwrap().edit_player(3, edited);
    assert!(view.session.as_mut().unwrap().undo(), "the third is undone");
    assert!(view.is_dirty(), "premise: two changes stand");

    prompt::open(&mut view, Pending::Close);
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(prompt_drawn(&nodes), "the prompt is up");
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        prompt_rows(&pairs, "Unsaved changes"),
        [
            (
                "PLACEHOLDER · Height".to_owned(),
                "180".to_owned(),
                "170".to_owned()
            ),
            (
                "PLACEHOLDER · Speed".to_owned(),
                speed.to_string(),
                "90".to_owned()
            ),
        ],
        "exactly the two rows, each naming him and the field"
    );
}

// SE-GUI-05: the prompt is its heading, the table and the three answers —
// no "unsaved changes" line under the heading and no line about the path
// above the buttons (the title and the answers already say both).
#[test]
fn the_prompt_shows_only_the_table_and_the_answers() {
    let path = fixture(PesVersion::Pes21, "prompt_no_prose");
    let mut view = dirty_view(&path);
    prompt::open(&mut view, Pending::Close);
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let nodes = dialog_nodes(&pairs, "Unsaved changes");
    assert!(
        !nodes
            .iter()
            .any(|node| node.value() == Some("The open save has unsaved changes.")),
        "no unsaved-changes line under the heading"
    );
    assert!(
        !nodes.iter().any(|node| {
            node.value()
                .is_some_and(|value| value.starts_with("Save writes them to"))
        }),
        "no line about the path above the buttons"
    );
}

// SE-GUI-05: the prompt is as wide as its widest row up to nine tenths of
// the window: a row between egui's 600-point default area and the cap
// sits on one line, the modal at least as wide.
#[test]
fn a_row_wider_than_the_default_keeps_one_line() {
    let path = fixture(PesVersion::Pes21, "prompt_wide_row");
    let mut view = SaveView::new();
    view.open(&path);
    // A name of fifty-two letters: the Name row lands between egui's
    // 600-point default area and nine tenths of the window.
    let mut edited = player_entry(&view, 70101);
    edited.name = "N".repeat(52);
    view.session.as_mut().unwrap().edit_player(1, edited);
    prompt::open(&mut view, Pending::Close);
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let rows = prompt_rows(&pairs, "Unsaved changes");
    let [(item, old, new)] = rows.as_slice() else {
        panic!("the one Name row: {rows:?}")
    };
    assert_eq!(
        (old.as_str(), new.as_str()),
        ("PLACEHOLDER", "N".repeat(52).as_str()),
        "the Name row"
    );
    let dialog = dialog_nodes(&pairs, "Unsaved changes");
    let item_bounds = dialog
        .iter()
        .find(|node| node.value() == Some(item.as_str()))
        .and_then(|node| node.bounds())
        .expect("the Name row's item cell");
    // The row's three cells share one y, each one line tall.
    let cells: Vec<accesskit::Rect> = dialog
        .iter()
        .filter(|node| {
            node.role() == Role::Label
                && node.bounds().is_some_and(|bounds| {
                    (bounds.y0 - item_bounds.y0).abs() < 1.0 && bounds.x0 >= item_bounds.x0 - 1.0
                })
        })
        .filter_map(|node| node.bounds())
        .collect();
    assert_eq!(cells.len(), 3, "the row's three cells: {cells:?}");
    assert!(
        cells.iter().all(|bounds| bounds.y1 - bounds.y0 < 25.0),
        "each cell is one line ({cells:?})"
    );
    let span = cells
        .iter()
        .map(|bounds| bounds.x1)
        .fold(f64::MIN, f64::max)
        - item_bounds.x0;
    assert!(span > 600.0, "premise: the row is wider than 600 ({span})");
    assert!(
        span < f64::from(window.screen.x) * 0.9,
        "premise: under nine tenths of the window ({span})"
    );
    let dialog_rect = prompt_rect(&mut window);
    assert!(
        f64::from(dialog_rect.width()) >= span - 1.0,
        "the modal is at least as wide as the row ({dialog_rect:?} vs {span})"
    );
}

// SE-GUI-05: a row wider than nine tenths of the window caps the modal
// there and scrolls sideways inside the table.
#[test]
fn a_row_past_the_cap_scrolls_sideways() {
    let path = fixture(PesVersion::Pes21, "prompt_over_cap");
    let mut view = SaveView::new();
    view.open(&path);
    let mut edited = player_entry(&view, 70101);
    edited.name = "N".repeat(150);
    view.session.as_mut().unwrap().edit_player(1, edited);
    prompt::open(&mut view, Pending::Close);
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let dialog = dialog_nodes(&pairs, "Unsaved changes");
    let dialog_rect = prompt_rect(&mut window);
    let cap = f64::from(window.screen.x) * 0.9;
    assert!(
        (f64::from(dialog_rect.width()) - cap).abs() <= 1.0,
        "the modal caps at nine tenths of the window ({dialog_rect:?} vs {cap})"
    );
    assert!(
        dialog.iter().any(|node| {
            node.role() == Role::ScrollBar
                && node
                    .bounds()
                    .is_some_and(|bounds| bounds.x1 - bounds.x0 > bounds.y1 - bounds.y0)
        }),
        "a horizontal scroll bar inside the prompt"
    );
}

// SE-GUI-05
#[test]
fn a_teams_row_comes_after_the_players() {
    let path = fixture(PesVersion::Pes21, "prompt_team_row");
    let mut view = SaveView::new();
    view.open(&path);
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    let mut team = team_entry(&view, 701);
    assert_ne!(
        (
            team.tactics.presets[0].sliders.support_range,
            team.roster[0].number
        ),
        (9, 99),
        "premise: real changes"
    );
    let (old_slider, old_number) = (
        team.tactics.presets[0].sliders.support_range,
        team.roster[0].number,
    );
    team.tactics.presets[0].sliders.support_range = 9;
    team.roster[0].number = 99;
    view.session.as_mut().unwrap().edit_team(4, team);
    prompt::open(&mut view, Pending::Close);
    let mut window = Headless::new();
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        prompt_rows(&pairs, "Unsaved changes"),
        [
            (
                "PLACEHOLDER · Height".to_owned(),
                "180".to_owned(),
                "170".to_owned()
            ),
            (
                // The slot's shirt number under its player, after his
                // own rows (his dot covers it).
                "PLACEHOLDER · Shirt number".to_owned(),
                old_number.to_string(),
                "99".to_owned()
            ),
            (
                "/3/ · Preset 1 · Support range".to_owned(),
                old_slider.to_string(),
                "9".to_owned()
            ),
        ],
        "the shirt number under its player, the team's own row last"
    );
}

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
    let reopened = Session::open(&path).unwrap();
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

/// A click on `node` with `running` as the running-PES probe (the
/// question legs, which an empty probe would never reach).
fn click_with(
    window: &mut Headless,
    view: &mut SaveView,
    running: &dyn Fn() -> Vec<RunningPes>,
    node: &accesskit::Node,
) {
    press_release(window, centre(node), |ui| show(ui, view, running));
}

/// The `label` button of the modal headed `heading`, found in that
/// modal's own `Role::Dialog` subtree (`dialog_nodes`): the toolbar's
/// same-labeled "Save" and the other modal's buttons sit outside it, so a
/// button of the prompt under the question never stands in for the
/// question's own (a nearest-by-position pick did, its click then landing
/// on the question's backdrop, which cancels it too).
pub(super) fn modal_button<'a>(
    pairs: &'a [(accesskit::NodeId, accesskit::Node)],
    heading: &str,
    label: &str,
) -> &'a accesskit::Node {
    dialog_nodes(pairs, heading)
        .into_iter()
        .find(|node| node.role() == Role::Button && node.label() == Some(label))
        .unwrap_or_else(|| panic!("no {label:?} in the {heading:?} modal"))
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
    let reopened = Session::open(&path).unwrap();
    assert_eq!(
        reopened.file().player(70101).unwrap().basic.height,
        170,
        "the file holds the edit"
    );
    assert!(view.prompt.is_none(), "the prompt is done");
    assert!(view.session.is_none(), "the session dropped");
    assert!(view.request_close, "the close is asked");
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

// SE-GUI-05, SE-GUI-20: Strip again's change is a `Stripped` row under
// the player's own (the appearance id is no `compare_players` row).
#[test]
fn a_strip_again_is_a_stripped_row_under_the_player() {
    let path = fixture(PesVersion::Pes21, "prompt_strip_row");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    for player in file.players_mut() {
        player.appearance_id = Some(STRIPPED_APPEARANCE_ID);
    }
    // One restored player on team 702 (the Strip again window's list).
    let id = file
        .team(702)
        .unwrap()
        .roster
        .iter()
        .map(|slot| slot.player_id)
        .find(|&id| id != 0)
        .expect("team 702 rosters someone");
    file.player_mut(id).unwrap().appearance_id = Some(id);
    let name = pes_savefile::model::names::display_name(&file.player(id).unwrap().name);
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    assert!(view.strip_again.is_some(), "the window lists him");
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Strip again"),
        &mut view,
    );
    assert_eq!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(id)
            .unwrap()
            .appearance_id,
        Some(STRIPPED_APPEARANCE_ID),
        "premise: stripped again"
    );

    prompt::open(&mut view, Pending::Close);
    // One frame so the modal's area has registered (its first frame is
    // egui's invisible sizing pass).
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        prompt_rows(&pairs, "Unsaved changes"),
        [(
            format!("{name} · Stripped"),
            "No".to_owned(),
            "Yes".to_owned()
        )],
        "the strip is his one row"
    );
}

// SE-GUI-05: a player rostered on two teams gets his own rows under his
// first listing only — the row a `Number` diff would take per slot stays
// per slot, his field rows do not repeat.
#[test]
fn a_player_on_two_rosters_gets_his_rows_once() {
    let path = fixture(PesVersion::Pes21, "prompt_two_rosters");
    let mut view = SaveView::new();
    view.open(&path);
    // Team 702's first slot rostered to player 70101 (already on 701's).
    let mut edited = team_entry(&view, 702);
    edited.roster[0].player_id = 70101;
    view.session.as_mut().unwrap().edit_team(1, edited);
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(2, edited);
    let file = view.session.as_ref().unwrap().file();
    assert_eq!(
        selection::rows(file, &TeamChoice::All, "")
            .iter()
            .filter(|row| row.player_id == 70101)
            .count(),
        2,
        "premise: 70101 is listed twice"
    );

    prompt::open(&mut view, Pending::Close);
    let mut window = Headless::new();
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let rows = prompt_rows(&pairs, "Unsaved changes");
    let heights: Vec<&(String, String, String)> = rows
        .iter()
        .filter(|(item, _, _)| item == "PLACEHOLDER · Height")
        .collect();
    assert_eq!(heights.len(), 1, "one Height row: {rows:?}");
}

// SE-GUI-05: a changed player no roster lists is shown after the listed
// players' rows.
#[test]
fn an_unlisted_players_rows_come_after_the_listed() {
    let path = fixture(PesVersion::Pes21, "prompt_unlisted");
    let mut view = SaveView::new();
    view.open(&path);
    // A listed player's edit, so the order shows.
    let mut edited = player_entry(&view, 70102);
    edited.basic.weight = 90;
    view.session.as_mut().unwrap().edit_player(1, edited);
    // 70101's height changed, then his slot emptied: no roster lists him.
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(2, edited);
    let mut team = team_entry(&view, 701);
    team.roster[0].player_id = 0;
    view.session.as_mut().unwrap().edit_team(3, team);
    let file = view.session.as_ref().unwrap().file();
    assert!(
        !selection::rows(file, &TeamChoice::All, "")
            .iter()
            .any(|row| row.player_id == 70101),
        "premise: no roster lists him"
    );

    prompt::open(&mut view, Pending::Close);
    let mut window = Headless::new();
    // One frame so the modal's area has registered (its first frame is
    // egui's invisible sizing pass).
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let rows = prompt_rows(&pairs, "Unsaved changes");
    let height = rows
        .iter()
        .position(|(item, _, _)| item == "PLACEHOLDER · Height")
        .expect("his row is there");
    let weight = rows
        .iter()
        .position(|(item, _, _)| item == "PLACEHOLDER · Weight")
        .expect("the listed player's row");
    let team_row = rows
        .iter()
        .position(|(item, _, _)| item.starts_with("/3/ ·"))
        .expect("the team's row");
    assert!(
        height > weight && height < team_row,
        "his row after the listed players' and before the teams' rows: {rows:?}"
    );
}

// SE-GUI-05's Save under "another save opened": the open file is written
// first, then the picked save opens.
#[test]
fn opening_anothers_save_writes_the_first_then_opens_the_picked() {
    let first = fixture(PesVersion::Pes21, "prompt_open_save_first");
    let original = std::fs::read(&first).unwrap();
    let other = fixture(PesVersion::Pes21, "prompt_open_save_second");
    let mut window = Headless::new();

    let mut view = dirty_view(&first);
    let picked = other.clone();
    view.open_dialog = Box::new(move |_| Some(picked.clone()));
    choose_file(&mut window, &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let nodes: Vec<accesskit::Node> = pairs.iter().map(|(_, node)| node.clone()).collect();
    assert!(prompt_drawn(&nodes), "the prompt is up");
    click(
        &mut window,
        modal_button(&pairs, "Unsaved changes", "Save"),
        &mut view,
    );
    assert_eq!(
        view.session.as_ref().unwrap().path(),
        other.as_path(),
        "the picked save is open"
    );
    let reopened = Session::open(&first).unwrap();
    assert_eq!(
        reopened.file().player(70101).unwrap().basic.height,
        170,
        "the first file holds the edit"
    );
    assert_ne!(std::fs::read(&first).unwrap(), original, "the write ran");

    // The same with the question between (the probe reporting the save's
    // own PES): the prompt's Save asks it, its Save writes and opens the
    // picked save.
    let first = fixture(PesVersion::Pes21, "prompt_open_save_q_first");
    let other = fixture(PesVersion::Pes21, "prompt_open_save_q_second");
    let running_own = || {
        vec![RunningPes {
            version: PesVersion::Pes21,
            exe: None,
        }]
    };
    let mut view = dirty_view(&first);
    let picked = other.clone();
    view.open_dialog = Box::new(move |_| Some(picked.clone()));
    choose_file(&mut window, &mut view);
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
    assert_eq!(
        view.session.as_ref().unwrap().path(),
        other.as_path(),
        "the second save is open"
    );
    let reopened = Session::open(&first).unwrap();
    assert_eq!(
        reopened.file().player(70101).unwrap().basic.height,
        170,
        "the first file holds the edit"
    );
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

// A tactics row naming a player words it with the roster's display name
// (the prompt's `RosterRow`s, `roster_for`).
#[test]
fn a_tactics_row_names_the_player_as_the_roster_shows_him() {
    let path = fixture(PesVersion::Pes21, "prompt_tactics_name");
    let mut view = SaveView::new();
    view.open(&path);
    // Team 701's Captain set to the slot-3 player: the row names him.
    let mut team = team_entry(&view, 701);
    team.tactics.set_pieces.captain = 3;
    let file = view.session.as_ref().unwrap().file();
    let id = file.team(701).unwrap().roster[3].player_id;
    let name = pes_savefile::model::names::display_name(&file.player(id).unwrap().name);
    assert_ne!(name, "(empty)", "premise: the rostered player has a name");
    view.session.as_mut().unwrap().edit_team(1, team);

    prompt::open(&mut view, Pending::Close);
    let mut window = Headless::new();
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let rows = prompt_rows(&pairs, "Unsaved changes");
    assert!(
        rows.iter()
            .any(|(item, _, new)| item == "/3/ · Captain" && new == &name),
        "the Captain row names him ({name}): {rows:?}"
    );
}

// SE-GUI-05: a tactics row's old side names the player the saved file
// holds in the slot — a drag moves records between the roster's ids, so
// the current roster would name the wrong one (the new side reads the
// current roster, the old the baseline's).
#[test]
fn a_tactics_rows_old_side_names_the_baseline_records() {
    let path = fixture(PesVersion::Pes21, "prompt_captain_baseline");
    // A (ALPHA, player 70101) captains slot 1 (stored 0); B is 70102.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    assert_eq!(
        file.team(701).unwrap().roster[0].player_id,
        70101,
        "premise: 70101 rosters slot 1"
    );
    file.player_mut(70101).unwrap().name = "ALPHA".to_owned();
    file.player_mut(70102).unwrap().name = "BETA".to_owned();
    file.team_mut(701).unwrap().tactics.set_pieces.captain = 0;
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    // Drag B's row onto slot 1: the roster's ids and numbers stay, the
    // two records trade places (slot 1 now shows BETA, slot 2 ALPHA).
    view.session.as_mut().unwrap().move_player(701, 1, 0);
    let file = view.session.as_ref().unwrap().file();
    assert_eq!(file.player(70101).unwrap().name, "BETA", "premise");
    assert_eq!(file.player(70102).unwrap().name, "ALPHA", "premise");
    // Make slot 2 (roster index 1) the captain: A stays the captain,
    // only his slot changed (0 to 1).
    let mut team = team_entry(&view, 701);
    team.tactics.set_pieces.captain = 1;
    view.session.as_mut().unwrap().edit_team(2, team);

    prompt::open(&mut view, Pending::Close);
    assert!(view.prompt.is_some(), "the prompt opened");
    let mut window = Headless::new();
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let rows = prompt_rows(&pairs, "Unsaved changes");
    let captain = rows
        .iter()
        .find(|(item, _, _)| item == "/3/ · Captain")
        .expect("the Captain row");
    assert_eq!(
        (captain.1.as_str(), captain.2.as_str()),
        ("ALPHA", "ALPHA"),
        "A captained then and captains still — both sides name him: {rows:?}"
    );
}

// SE-GUI-05: a team's own row names the team by its display name too.
#[test]
fn a_team_rows_item_is_the_teams_display_name() {
    let path = fixture(PesVersion::Pes21, "prompt_team_display_name");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.team_mut(701).unwrap().name = "\x11cff0000ff/3/".to_owned();
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut team = view
        .session
        .as_ref()
        .unwrap()
        .file()
        .team(701)
        .unwrap()
        .clone();
    let old = team.tactics.presets[0].sliders.support_range;
    team.tactics.presets[0].sliders.support_range = if old == 9 { 8 } else { 9 };
    let new = team.tactics.presets[0].sliders.support_range;
    view.session.as_mut().unwrap().edit_team(1, team);

    prompt::open(&mut view, Pending::Close);
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        prompt_rows(&pairs, "Unsaved changes"),
        [(
            "/3/ · Preset 1 · Support range".to_owned(),
            old.to_string(),
            new.to_string()
        )],
        "the item carries no \\x11"
    );
}

// SE-GUI-05: the Item column reads the display name (the colour codes
// dropped), worded in the views' terms as the Comparator panel's rows are.
#[test]
fn the_item_column_is_the_display_name() {
    let path = fixture(PesVersion::Pes21, "prompt_display_name");
    // A colour-coded name, as cup saves carry: `\x11c` plus eight hex.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().name = "\x11cffb50effLORD".to_owned();
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);

    prompt::open(&mut view, Pending::Close);
    let mut window = Headless::new();
    // One frame so the modal's area has registered (its first frame is
    // egui's invisible sizing pass).
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        prompt_rows(&pairs, "Unsaved changes"),
        [(
            "LORD · Height".to_owned(),
            "180".to_owned(),
            "170".to_owned()
        )],
        "the item carries no \\x11"
    );
}

// SE-GUI-05: a click on the backdrop is the prompt's Cancel too — it
// closes, the session untouched and still dirty.
#[test]
fn a_backdrop_click_on_the_prompt_closes_it_and_changes_nothing() {
    let path = fixture(PesVersion::Pes21, "prompt_backdrop");
    let original = std::fs::read(&path).unwrap();
    let mut view = dirty_view(&path);
    prompt::open(&mut view, Pending::Close);
    let mut window = Headless::new();
    // One frame for the modal to register as the top one, then a click
    // beside it, on the backdrop.
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(view.prompt.is_some(), "premise: the prompt is up");
    click_at(&mut window, egui::pos2(5.0, 5.0), &mut view);
    assert!(view.prompt.is_none(), "a backdrop click closes the prompt");
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

// SE-GUI-21's window rule: a click on the backdrop is the question's
// Cancel — it closes, the prompt behind stays, the file unchanged.
#[test]
fn a_backdrop_click_on_the_question_closes_it_and_keeps_the_prompt() {
    let path = fixture(PesVersion::Pes21, "question_backdrop");
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
    click_at(&mut window, egui::pos2(5.0, 5.0), &mut view);
    assert!(
        view.edit_mode_question.is_none(),
        "a backdrop click closes the question"
    );
    assert!(view.prompt.is_some(), "the prompt stays up");
    assert!(view.is_dirty(), "still dirty");
    assert_eq!(std::fs::read(&path).unwrap(), original, "nothing wrote");
}

// The prompt's title draws in the window-title style (the drafts'
// `.window .title`): 14 points in the strong colour, not `Heading`'s 15,
// which a page's heading keeps.
#[test]
fn the_prompts_title_is_14_points_and_strong() {
    let path = fixture(PesVersion::Pes21, "prompt_title_style");
    let mut view = dirty_view(&path);
    prompt::open(&mut view, Pending::Close);
    let mut window = Headless::new();
    // The modal's first frame is a sizing pass; the second's shapes are
    // painted.
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let title = text_shape(&shapes, "Unsaved changes");
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

// `roster_names` and `roster_for` carry the display name and the
// record's own name, colour codes kept (the coloured runs' source).
#[test]
fn roster_for_holds_the_raw_name_and_the_display_name() {
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let team = file.team(701).unwrap().clone();
    let id = team.roster[0].player_id;
    file.player_mut(id).unwrap().name = "\x11cffb50effLORD".to_owned();
    let names = prompt::roster_names(&team, |id| file.player(id));
    let rows = prompt::roster_for(&team, &names, |id| file.player(id));
    assert_eq!(rows[0].name, "LORD", "the display name, codes dropped");
    assert_eq!(
        rows[0].raw_name, "\x11cffb50effLORD",
        "the raw name, codes kept"
    );
}
