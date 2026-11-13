use super::*;

// SE-GUI-13
#[test]
fn export_writes_the_commands_text_whatever_the_team_filter() {
    let dir = crate::test_support::scratch("comparator_export_scope");
    // The open save written to disk with its edits: 73401's height and
    // 74301's (a gameplay row in each team) and 74301's boots id (an
    // aesthetics row, 0 in the fixture).
    let open_path = picked_save(&dir, "open", |file| {
        for id in [73401u32, 74301] {
            file.player_mut(id).expect("a player").basic.height = 183;
        }
        file.player_mut(74301).expect("74301").appearance.boots_id = 3601;
    });
    let picked = picked_save(&dir, "picked", |_| {});
    let mut view = SaveView::new();
    view.open(&open_path);
    view.panel = Some(Panel::Comparator);
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    pick(&mut window, &mut view, picked.clone());
    compared(&mut window, &mut view);

    // The scope Gameplay, the team filter on 734: the export still holds
    // the whole save's report (74301's height row included).
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Gameplay"),
        &mut view,
    );
    compared(&mut window, &mut view);
    pick_comparator_team(&mut window, &mut view, "/lit/");
    let target = dir.join("diff.txt");
    view.export_dialog = Box::new({
        let target = target.clone();
        move |name| {
            assert_eq!(name, "diff.txt", "the suggested name");
            Some(target.clone())
        }
    });
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Export\u{2026}"),
        &mut view,
    );
    let first = picked.to_str().unwrap().to_owned();
    let second = open_path.to_str().unwrap().to_owned();
    assert_eq!(
        std::fs::read_to_string(&target).expect("the export written"),
        cli_stdout(&dir, &["diff", &first, &second, "--gameplay"]),
        "the file is `diff --gameplay`'s stdout, the team filter notwithstanding"
    );

    // All: the command's stdout with no flag.
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "All"),
        &mut view,
    );
    compared(&mut window, &mut view);
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Export\u{2026}"),
        &mut view,
    );
    assert_eq!(
        std::fs::read_to_string(&target).expect("the export rewritten"),
        cli_stdout(&dir, &["diff", &first, &second]),
        "the file is `diff`'s stdout with no flag"
    );
}

// SE-GUI-13
#[test]
fn an_empty_scopes_export_is_the_commands_empty_text() {
    // 73401's height the one difference (gameplay): the Aesthetics scope
    // holds no row, and the command prints nothing for it.
    let (mut view, mut window, _) = compared_73401("comparator_export_empty");
    let dir = scratch_of(&view);
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Aesthetics"),
        &mut view,
    );
    compared(&mut window, &mut view);
    let target = dir.join("diff.txt");
    view.export_dialog = Box::new({
        let target = target.clone();
        move |_| Some(target.clone())
    });
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Export\u{2026}"),
        &mut view,
    );
    let open = view
        .session
        .as_ref()
        .unwrap()
        .path()
        .to_str()
        .unwrap()
        .to_owned();
    let picked = dir.join("picked").to_str().unwrap().to_owned();
    let stdout = cli_stdout(&dir, &["diff", &picked, &open, "--aesthetics"]);
    assert_eq!(stdout, "", "premise: the command prints nothing");
    assert_eq!(
        std::fs::read_to_string(&target).expect("the export written"),
        stdout,
        "an empty report exports an empty file"
    );
}

// SE-GUI-13
#[test]
fn export_carries_the_unsaved_edits() {
    // 73401's height to 183 in the session, no save written: the export
    // still holds the command's line for it.
    let (mut view, mut window) = open_734_with("comparator_export_unsaved", &[73401], 183);
    let dir = scratch_of(&view);
    let picked = picked_save(&dir, "picked", |_| {});
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let target = dir.join("diff.txt");
    view.export_dialog = Box::new({
        let target = target.clone();
        move |_| Some(target.clone())
    });
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Export\u{2026}"),
        &mut view,
    );
    let text = std::fs::read_to_string(&target).expect("the export written");
    assert!(
        text.contains("- Info player_diff (player=73401, field=Height, old=185, new=183)\n"),
        "the unsaved edit's line is in the export: {text}"
    );
}

// SE-GUI-13
#[test]
fn export_reports_the_written_path_and_refuses_a_folder_that_is_not() {
    let (mut view, mut window, _) = compared_73401("comparator_export_wrote");
    let dir = scratch_of(&view);
    let target = dir.join("diff.txt");
    view.export_dialog = Box::new({
        let target = target.clone();
        move |_| Some(target.clone())
    });
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Export\u{2026}"),
        &mut view,
    );
    assert_eq!(
        view.notice
            .as_ref()
            .map(|notice| (notice.text.as_str(), notice.kind)),
        Some(("diff.txt written", studio_core::NoticeKind::Success)),
        "the export's notice"
    );
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines
            .iter()
            .any(|line| line == &format!("{} written", target.display())),
        "the written line: {lines:?}"
    );

    // The export is a journal step and its bytes the kept output.
    assert_wrote_and_kept(&view.recorder, &dir, &target, "outputs/0-diff.txt");

    // A folder that does not exist: the refusal dialog names the path.
    let missing = dir.join("none").join("diff.txt");
    view.export_dialog = Box::new({
        let missing = missing.clone();
        move |_| Some(missing.clone())
    });
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Export\u{2026}"),
        &mut view,
    );
    // The refusal's modal spends its first frame sizing itself, with no
    // nodes; the last of the frames drawn here holds them.
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Cannot export the differences")),
        "the refusal's title"
    );
    assert!(
        nodes.iter().any(|node| {
            node.value()
                .is_some_and(|value| value.contains(&missing.display().to_string()))
        }),
        "the refusal names the path"
    );
}

/// Export's dialog stubbed to `destination`, a spelling of one of the two
/// saves, and Export clicked: the refusal names it in the export panel's
/// sentence, the save's bytes are unchanged, no written line shows and the
/// refused write records `could not write <path>`, keeping no output.
fn assert_export_refused(view: &mut SaveView, window: &mut Headless, destination: PathBuf) {
    let dir = scratch_of(view);
    let before = std::fs::read(&destination).unwrap();
    view.export_dialog = Box::new({
        let destination = destination.clone();
        move |_| Some(destination.clone())
    });
    let nodes = draw(window, view);
    click(
        window,
        node_labeled(&nodes, Role::Button, "Export\u{2026}"),
        view,
    );
    assert!(
        std::fs::read(&destination).unwrap() == before,
        "{}: the save's bytes are unchanged",
        destination.display()
    );
    // The refusal's modal spends its first frame sizing itself, with no
    // nodes; the last of the frames drawn here holds them.
    draw(window, view);
    let nodes = draw(window, view);
    let sentence = format!(
        "{}: that is the open save, and writing the difference report over it would destroy the save",
        destination.display()
    );
    for text in ["Cannot export the differences", sentence.as_str()] {
        assert!(label_value(&nodes, text), "the refusal shows {text:?}");
    }
    assert!(view.comparator.exported.is_none(), "no written line");
    assert_write_refused(&view.recorder, &dir, &destination);
}

#[test]
fn export_refuses_the_open_save_as_its_destination() {
    let (mut view, mut window, _) = compared_73401("comparator_export_open_save");
    let open = view.session.as_ref().unwrap().path().to_path_buf();
    assert_export_refused(&mut view, &mut window, open);
}

#[test]
fn export_refuses_the_picked_save_as_its_destination() {
    let (mut view, mut window, _) = compared_73401("comparator_export_picked_save");
    let picked = scratch_of(&view).join("picked");
    assert_eq!(
        view.comparator.picked.as_ref().map(|(path, _)| path),
        Some(&picked),
        "premise: the picked save"
    );
    assert_export_refused(&mut view, &mut window, picked);
}

// SE-GUI-13
#[test]
fn another_save_resets_the_team_filter() {
    let (mut view, mut window, _) = compared_73401("comparator_filter_reset");
    let dir = scratch_of(&view);
    pick_comparator_team(&mut window, &mut view, "/lit/");
    assert_eq!(
        view.comparator.filter_team,
        TeamChoice::Team(734),
        "premise: 734 picked"
    );

    // Another save opened (the fixture again through `SaveView::open`):
    // its teams are not the old one's, so the filter reads ALL.
    let second = picked_save(&dir, "second", |_| {});
    view.open(&second);
    assert_eq!(
        view.comparator.filter_team,
        TeamChoice::All,
        "the filter reset"
    );
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        comparator_nodes(&pairs)
            .iter()
            .find(|node| node.role() == Role::ComboBox)
            .and_then(|combo| combo.value()),
        Some("ALL"),
        "the combo reads ALL"
    );
}

/// An invoked Pick… (Tab + Space, or AccessKit's Click — which leaves the
/// keyboard focus where it was) starts the comparison: the button's hold
/// on the keyboard is given up, so `edit_settled` reports settled.
#[test]
fn an_invoked_pick_starts_the_comparison() {
    let (mut view, mut window) = open_734_with("comparator_invoke", &[73401], 183);
    let dir = scratch_of(&view);
    let picked = picked_save(&dir, "picked", |_| {});
    view.panel = Some(Panel::Comparator);
    view.open_save_dialog = Box::new(move || Some(picked.clone()));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (pick, _) = nodes_in_window(&pairs, "Comparator")
        .into_iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("Pick\u{2026}"))
        .expect("the Pick button");
    action_focus(&mut window, &mut view, pick);
    action_click(&mut window, &mut view, pick);
    compared(&mut window, &mut view);
}

/// An invoked scope choice (Tab + Space, or AccessKit's Click) starts the
/// comparison of its scope with no further input: the choice gives up the
/// keyboard it holds.
// SE-GUI-13
#[test]
fn an_invoked_scope_choice_starts_the_comparison() {
    let (mut view, mut window, _) = compared_73401("comparator_invoke_scope");
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (gameplay, _) = comparator_button(&pairs, "Gameplay");
    action_focus(&mut window, &mut view, gameplay);
    action_click(&mut window, &mut view, gameplay);
    assert_eq!(
        view.comparator.scope,
        Some(DiffScope::Gameplay),
        "premise: the click picked Gameplay"
    );
    draw(&mut window, &mut view);
    assert!(
        view.comparator.comparison.pending.is_some()
            || view
                .comparator
                .comparison
                .result
                .as_ref()
                .is_some_and(|(key, _)| key.scope == Some(DiffScope::Gameplay)),
        "the Gameplay comparison started"
    );
    compared(&mut window, &mut view);
}

/// The pick row stays one line however long the picked save's path is:
/// the label, the truncated path and the buttons share a top, every
/// button inside the window.
#[test]
fn the_pick_row_keeps_the_buttons_on_one_line_with_a_long_path() {
    let (mut view, mut window) = open_734_with("comparator_row", &[73401], 183);
    let dir = scratch_of(&view);
    let picked = picked_save(&dir, &"picked".repeat(40), |_| {});
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, window_node) = pairs
        .iter()
        .find(|(_, node)| {
            node.label()
                .is_some_and(|label| label.starts_with("Comparator"))
        })
        .expect("the window");
    let window_rect = window_node.bounds().expect("the window's bounds");
    let nodes = comparator_nodes(&pairs);
    let top = |keep: &dyn Fn(&accesskit::Node) -> bool| -> f64 {
        nodes
            .iter()
            .find(|node| keep(node))
            .and_then(|node| node.bounds())
            .expect("a drawn node's bounds")
            .y0
    };
    let tops = [
        top(&|node| node.role() == Role::Label && node.value() == Some("Second save ")),
        top(&|node| {
            node.role() == Role::Label
                && node
                    .value()
                    .is_some_and(|value| value.contains("pickedpicked"))
        }),
        top(&|node| node.role() == Role::Button && node.label() == Some("Pick\u{2026}")),
        top(&|node| node.role() == Role::Button && node.label() == Some("Export\u{2026}")),
    ];
    for (a, b) in tops.iter().zip(tops.iter().skip(1)) {
        assert!((a - b).abs() < 0.5, "the row's tops: {tops:?}");
    }
    for label in ["Pick\u{2026}", "Export\u{2026}"] {
        let bounds = comparator_button(&pairs, label)
            .1
            .bounds()
            .expect("the button's bounds");
        assert!(
            bounds.x1 <= window_rect.x1,
            "{label} ends inside the window: {bounds:?} vs {window_rect:?}"
        );
    }
}

// The panel's title bar draws its title in the window-title style (the
// drafts' `.panel .title`): 14 points in the strong colour, where egui's
// title bar falls back to `Heading`'s 15.
#[test]
fn the_panels_title_is_14_points_and_strong() {
    let (mut view, mut window) = open_734_with("comparator_title", &[73401], 183);
    view.panel = Some(Panel::Comparator);
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let title = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == "Comparator" => Some(shape),
            _ => None,
        })
        .next()
        .expect("the panel's title is painted");
    assert_eq!(
        title.galley.job.sections[0].format.font_id.size, 14.0,
        "the drafts' .panel .title"
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

#[test]
fn the_close_button_closes_the_panel() {
    let (mut view, mut window) = open_734_with("comparator_close", &[73401], 183);
    let pairs = open_comparator(&mut window, &mut view);
    assert_eq!(view.panel, Some(Panel::Comparator), "premise: open");
    let close = comparator_button(&pairs, "Close window").1.clone();
    click(&mut window, &close, &mut view);
    assert_eq!(view.panel, None, "the panel closes");
}

/// The panel's outer rect, from its AccessKit window node.
fn panel_rect(nodes: &[accesskit::Node]) -> accesskit::Rect {
    nodes
        .iter()
        .find(|node| node.role() == Role::Window && node.label() == Some("Comparator"))
        .and_then(accesskit::Node::bounds)
        .expect("the panel's window")
}

/// Whether `rect`'s left and top are `left` and `top`, within a point.
fn placed_at(rect: &accesskit::Rect, left: f64, top: f64) -> bool {
    (rect.x0 - left).abs() <= 1.0 && (rect.y0 - top).abs() <= 1.0
}

// A floating panel first opens where the drafts' `.panel` stands: at the
// 1280 x 780 client, 620 wide, its top 90 below the window's and its right
// edge 1 inside it, so (659, 90). Dragged, it stays where the member left
// it, across a close and a reopen.
#[test]
fn the_panel_first_opens_at_the_right_and_stays_where_dragged() {
    let (mut view, _) = open_734_with("comparator_place", &[73401], 183);
    let mut window = Headless::sized(1280.0, 780.0);
    open_comparator(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let rect = panel_rect(&nodes);
    assert!(
        placed_at(&rect, 659.0, 90.0) && (rect.x1 - rect.x0 - 620.0).abs() <= 1.0,
        "the panel's first rect: {rect:?}"
    );
    // Its edges exactly: a point off either reads as the window's edge
    // missed, within `placed_at`'s slack.
    assert!(
        (rect.x1 - (1280.0 - 1.0)).abs() < 0.25,
        "the right edge 1 inside the window's: {rect:?}"
    );
    assert!(
        (rect.x0 - (rect.x1 - 620.0)).abs() < 0.25,
        "the left edge 620 before the right: {rect:?}"
    );

    // Dragged by its title bar, left of the title.
    let from = egui::pos2(rect.x0 as f32 + 8.0, rect.y0 as f32 + 10.0);
    drag(
        &mut window,
        from,
        from + egui::vec2(-300.0, 100.0),
        &mut view,
    );
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let dragged = panel_rect(&nodes);
    assert!(
        placed_at(&dragged, 359.0, 190.0),
        "the dragged panel: {dragged:?}"
    );

    view.panel = None;
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    view.panel = Some(Panel::Comparator);
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let reopened = panel_rect(&nodes);
    assert!(
        placed_at(&reopened, 359.0, 190.0),
        "the reopened panel: {reopened:?}"
    );
}

// The comparator's pick reads `opened <path> for comparison` in the
// journal, so the bundle tells the second save's pick from the session's
// own open.
#[test]
fn a_picked_save_reads_as_an_opened_for_comparison_in_the_journal() {
    let path = fixture(PesVersion::Pes17, "comparator_journal");
    let dir = path.parent().unwrap().to_path_buf();
    let recorder = Recorder::default();
    let mut view = SaveView::new();
    view.recorder = recorder.clone();
    view.open(&path);
    let mut window = Headless::new();
    let picked = picked_save(&dir, "picked", |_| {});
    open_comparator(&mut window, &mut view);
    pick(&mut window, &mut view, picked.clone());

    let lines = journal_lines(&recorder, &dir);
    assert!(
        lines
            .iter()
            .any(|line| line.contains(" for comparison") && line.contains("picked")),
        "the pick reads as an opened for comparison: {lines:?}"
    );
}

// A pick of another version's save records `refused <path>: <reason>`
// beside its opened-for-comparison step, as the other picks' refusals do.
#[test]
fn a_version_mismatched_pick_records_the_refusal_in_the_journal() {
    let path = fixture(PesVersion::Pes21, "comparator_journal_refused");
    let dir = path.parent().unwrap().to_path_buf();
    let recorder = Recorder::default();
    let mut view = SaveView::new();
    view.recorder = recorder.clone();
    view.open(&path);
    let mut window = Headless::new();
    let picked = picked_save(&dir, "picked", |_| {});
    open_comparator(&mut window, &mut view);
    pick(&mut window, &mut view, picked.clone());

    let lines = journal_lines(&recorder, &dir);
    let refused = steps_with(&lines, "refused ");
    assert_eq!(refused.len(), 1, "the refusal records once: {lines:?}");
    assert!(refused[0].contains("picked"), "{}", refused[0]);
    assert!(
        refused[0].contains("PES 2017"),
        "the refusal names the picked save's version: {}",
        refused[0]
    );
}

// A held pick dropped for its version when another version's save opens
// records `refused <path>` once, as the pick's own refusal does: the drop
// is a refusal the panel shows.
#[test]
fn a_held_pick_dropped_on_another_versions_open_records_the_refusal() {
    let path = fixture(PesVersion::Pes21, "comparator_reset_refused");
    let dir = path.parent().unwrap().to_path_buf();
    let recorder = Recorder::default();
    let mut view = SaveView::new();
    view.recorder = recorder.clone();
    view.open(&path);
    let mut window = Headless::new();
    open_comparator(&mut window, &mut view);
    let picked = picked_pes21(&dir);
    pick(&mut window, &mut view, picked.clone());
    assert_eq!(
        view.comparator.picked.as_ref().map(|(path, _)| path),
        Some(&picked),
        "premise: the pick held"
    );

    let pes17 = picked_save(&dir, "pes17", |_| {});
    view.open(&pes17);
    assert!(view.comparator.picked.is_none(), "the pick dropped");

    let lines = journal_lines(&recorder, &dir);
    let refused = steps_with(&lines, "refused ");
    assert_eq!(
        refused.len(),
        1,
        "the dropped pick's refusal records once: {lines:?}"
    );
    assert!(
        refused[0].starts_with(&format!("refused {}: ", picked.display())),
        "the refusal names the dropped pick: {}",
        refused[0]
    );
    assert!(
        refused[0].contains("PES 2021"),
        "the refusal names the dropped save's version: {}",
        refused[0]
    );
}
