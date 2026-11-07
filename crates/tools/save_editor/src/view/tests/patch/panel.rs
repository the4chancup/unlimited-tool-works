use super::*;
use crate::test_support::{journal_lines, zip_entry};

/// Whether the panel is drawn on a frame.
fn panel_drawn(window: &mut Headless, view: &mut SaveView) -> bool {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    pairs
        .iter()
        .any(|(_, node)| node.label() == Some(TITLE) && node.role() != Role::Link)
}

/// The panel's node showing `text` (a Label's value or a widget's label),
/// `None` when it is not drawn.
fn panel_cell(window: &mut Headless, view: &mut SaveView, text: &str) -> Option<accesskit::Node> {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    panel_nodes(&pairs)
        .into_iter()
        .find(|node| node.value() == Some(text) || node.label() == Some(text))
        .cloned()
}

/// `player_id`'s roster slot (0-based) on `team` in the open save.
fn roster_slot(view: &SaveView, team: u32, player_id: u32) -> u8 {
    let slot = view
        .session
        .as_ref()
        .expect("an open save")
        .file()
        .team(team)
        .expect("the team")
        .roster
        .iter()
        .position(|entry| entry.player_id == player_id)
        .expect("the player is on the team's roster");
    u8::try_from(slot).expect("a roster slot fits a u8")
}

// SE-GUI-18
#[test]
fn a_rows_click_selects_its_player() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_row_click");
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view)],
    );
    previewed(&mut window, &mut view, &text);
    let slot = view
        .session
        .as_ref()
        .unwrap()
        .file()
        .team(CO)
        .unwrap()
        .roster
        .iter()
        .position(|entry| entry.player_id == CO_SLOT_3)
        .expect("71403 is on 714's roster");
    let slot = u8::try_from(slot).unwrap();
    view.team = TeamChoice::Team(718);
    view.selected = Some(Selection::Player(718, 0));

    let item = panel_cell(&mut window, &mut view, CO_ROWS[0][0]).expect("the corner kick's item");
    click(&mut window, &item, &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(CO, slot)),
        "71403 selected"
    );
    assert_eq!(
        view.team,
        TeamChoice::Team(CO),
        "the selector moved to his team"
    );
}

/// A preview row's Item cell (71403's corner kick) says on hover what its
/// click does.
#[test]
fn an_items_hover_says_its_click_switches_to_the_player() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_item_hover");
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view)],
    );
    previewed(&mut window, &mut view, &text);
    let item = panel_cell(&mut window, &mut view, CO_ROWS[0][0]).expect("the corner kick's item");
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(&item), "this player").as_deref(),
        Some("Click to switch to this player")
    );
}

// SE-GUI-18
#[test]
fn a_row_of_the_second_player_selects_him() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_second_row");
    let missing = co_missing_slot(&view);
    let text = two_team_patch(&view, missing);
    previewed(&mut window, &mut view, &text);
    let first = roster_slot(&view, CO, CO_SLOT_3);
    let second = roster_slot(&view, CO, CO_SLOT_5);
    assert_ne!(first, second, "premise: two roster slots");
    view.team = TeamChoice::Team(F);
    view.selected = Some(Selection::Player(F, 5));

    let item = panel_cell(&mut window, &mut view, TWO_TEAM_ROWS[1][0]).expect("71405's item");
    click(&mut window, &item, &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(CO, second)),
        "71405 selected, not 71403"
    );
    assert_eq!(
        view.team,
        TeamChoice::Team(CO),
        "the selector moved to his team"
    );
}

#[test]
fn the_close_button_closes_the_panel() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_close");
    open_panel(&mut window, &mut view);
    assert_eq!(view.panel, Some(Panel::Patch), "premise: open");
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let close = panel_nodes(&pairs)
        .into_iter()
        .find(|node| node.role() == Role::Button && node.label() == Some("Close window"))
        .expect("the close button")
        .clone();
    click(&mut window, &close, &mut view);
    assert_eq!(view.panel, None, "the panel closes");
}

// SE-GUI-22
#[test]
fn the_savefile_entry_stays_enabled_under_the_aesthetics_lock() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_entry_locked");
    view.aesthetics_from_exports = true;
    assert!(view.aesthetics_locked(), "premise: the lock is on");
    let nodes = open_menu(&mut window, &mut view, "Savefile", "Apply aesthetics patch");
    let entry = node_labeled(&nodes, Role::Button, "Apply aesthetics patch\u{2026}");
    assert!(!entry.is_disabled(), "the patch entry is enabled");
    click(&mut window, entry, &mut view);
    assert!(panel_drawn(&mut window, &mut view), "the panel opens");
}

// SE-GUI-22
#[test]
fn the_pre_fox_ownership_link_opens_the_panel() {
    let (mut view, mut window) = open_save(PesVersion::Pes17, "patch_ownership_link");
    view.aesthetics_from_exports = true;
    open_appearance(&mut window, &mut view);
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Link && node.label() == Some(TITLE)
    });
    assert!(!panel_drawn(&mut window, &mut view), "premise: closed");
    let link = nodes
        .iter()
        .find(|node| node.role() == Role::Link && node.label() == Some(TITLE))
        .expect("the ownership line's link");
    click(&mut window, link, &mut view);
    assert!(
        panel_drawn(&mut window, &mut view),
        "the link opens the panel"
    );
}

#[test]
fn opening_one_floating_panel_closes_the_other() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_one_panel");
    let nodes = open_menu(&mut window, &mut view, "Savefile", "Comparator");
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Comparator\u{2026}"),
        &mut view,
    );
    assert_eq!(
        view.panel,
        Some(Panel::Comparator),
        "premise: the comparator is open"
    );

    open_panel(&mut window, &mut view);
    assert_eq!(
        view.panel,
        Some(Panel::Patch),
        "the patch panel opens, the comparator closes"
    );

    let nodes = open_menu(&mut window, &mut view, "Savefile", "Comparator");
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Comparator\u{2026}"),
        &mut view,
    );
    assert_eq!(
        view.panel,
        Some(Panel::Comparator),
        "the comparator opens, the patch panel closes"
    );
}

// A file the member picks that cannot be parsed is a step of its own and
// a kept input, like a save that fails to open: `could not read <path>`.
#[test]
fn a_picked_file_that_cannot_be_read_records_the_step_and_keeps_it() {
    let path = fixture(PesVersion::Pes21, "patch_pick_fails");
    let dir = path.parent().unwrap().to_path_buf();
    let recorder = Recorder::default();
    let mut view = SaveView::new();
    view.recorder = recorder.clone();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    let bad = dir.join("patch.toml");
    std::fs::write(&bad, b"not a patch").unwrap();
    open_panel(&mut window, &mut view);
    open_patch(&mut window, &mut view, bad.clone());

    let lines = journal_lines(&recorder, &dir);
    let refused = lines
        .iter()
        .find(|line| line.starts_with("could not read "))
        .expect("the failed pick records its own step");
    assert!(
        refused.contains("patch.toml"),
        "the step names the file that failed: {refused}"
    );
    let bundle = dir.join("journal.zip");
    assert_eq!(
        zip_entry(&bundle, "inputs/1-patch.toml"),
        b"not a patch",
        "the picked file's bytes are kept as an input"
    );
}
