//! The Apply aesthetics patch panel's tests (SE-GUI-18, SE-GUI-22): the
//! preview in the views' terms, Apply as one undoable edit, a refusal of
//! another version's patch, the skips' warning lines, the preview
//! following the save, the row's click, and the entries that open it.

use pes_savefile::interchange::aesthetics_patch::{ALLOCATION_SCHEME_VERSION, AestheticsPatch};

use super::*;

/// Team `/co/`, 714 in the PES 21 fixture, whose slot 3 is player 71403.
const CO: u32 = 714;
const CO_SLOT_3: u32 = 71403;

/// The panel's title, its window's label.
const TITLE: &str = "Apply aesthetics patch";

/// A patch text for `version` under `scheme`, holding `teams` (`[[teams]]` blocks).
fn patch_text(version: u16, scheme: u32, teams: &str) -> String {
    format!(
        "pes_version = {version}\ntool_version = \"0.1.0\"\nallocation_scheme_version = {scheme}\ncpks = []\ncompiled = 2026-10-13T00:00:00Z\n{teams}"
    )
}

/// Team `/co/`'s block: slot 03 renamed, with two motions.
fn co_block() -> String {
    format!(
        "\n[[teams]]\nid = {CO}\nname = \"/co/\"\n\n[teams.players.03]\nname = \"PATCHED\"\nshirt_name = \"PATCHED\"\n\n[teams.players.03.appearance.motion]\nhunching_dribbling = 2\ncorner_kick = 3\n"
    )
}

/// `version`'s fixture save open under `test`, in a window tall enough
/// for the panel's list.
fn open_save(version: PesVersion, test: &str) -> (SaveView, Headless) {
    let path = fixture(version, test);
    let mut view = SaveView::new();
    view.open(&path);
    (view, Headless::sized(DEFAULT_PANE.x, 800.0))
}

/// `text` written as `patch.toml` beside the open save, in the test's
/// scratch folder.
fn write_patch(view: &SaveView, text: &str) -> PathBuf {
    let path = view
        .session
        .as_ref()
        .expect("an open save")
        .path()
        .with_file_name("patch.toml");
    std::fs::write(&path, text).unwrap();
    path
}

/// The panel opened from the Savefile menu's entry.
fn open_panel(window: &mut Headless, view: &mut SaveView) {
    let nodes = open_menu(window, view, "Savefile", "Apply aesthetics patch");
    click(
        window,
        node_labeled(&nodes, Role::Button, "Apply aesthetics patch\u{2026}"),
        view,
    );
}

/// `path` stubbed as the Open patch dialog's choice; `Open patch…`
/// clicked in the panel.
fn open_patch(window: &mut Headless, view: &mut SaveView, path: PathBuf) {
    view.patch_dialog = Box::new(move || Some(path.clone()));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let button = panel_nodes(&pairs)
        .into_iter()
        .find(|node| node.role() == Role::Button && node.label() == Some("Open patch\u{2026}"))
        .expect("the Open patch button");
    click(window, button, view);
}

/// The panel open from the menu with `text` opened as its patch.
fn previewed(window: &mut Headless, view: &mut SaveView, text: &str) -> PathBuf {
    let path = write_patch(view, text);
    open_panel(window, view);
    open_patch(window, view, path.clone());
    path
}

/// The panel window's nodes out of `pairs` (`frame_ids`' output): the
/// subtree of the node labeled exactly `TITLE` that is not the ownership
/// line's link.
fn panel_nodes(pairs: &[(accesskit::NodeId, accesskit::Node)]) -> Vec<&accesskit::Node> {
    let (window, _) = pairs
        .iter()
        .find(|(_, node)| node.label() == Some(TITLE) && node.role() != Role::Link)
        .expect("the patch panel");
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        pairs.iter().map(|(id, node)| (*id, node)).collect();
    let mut nodes = Vec::new();
    let mut stack = vec![*window];
    while let Some(id) = stack.pop() {
        let node = by_id[&id];
        stack.extend(node.children());
        nodes.push(node);
    }
    nodes
}

/// Whether the panel is drawn on a frame.
fn panel_drawn(window: &mut Headless, view: &mut SaveView) -> bool {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    pairs
        .iter()
        .any(|(_, node)| node.label() == Some(TITLE) && node.role() != Role::Link)
}

/// The panel's Label values and Button labels from `Open patch…` down, in
/// top-to-bottom, left-to-right order: what a member reads (the window's
/// resize handles left out).
fn panel_lines(window: &mut Headless, view: &mut SaveView) -> Vec<String> {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let mut lines: Vec<(f64, f64, String)> = Vec::new();
    for node in panel_nodes(&pairs) {
        let text = match node.role() {
            Role::Label => node.value().map(str::to_owned),
            _ => node.label().map(str::to_owned),
        };
        if let (Some(text), Some(bounds)) = (text, node.bounds()) {
            lines.push((bounds.y0, bounds.x0, text));
        }
    }
    lines.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .expect("drawn bounds")
            .then(a.1.partial_cmp(&b.1).expect("drawn bounds"))
    });
    lines
        .into_iter()
        .map(|(_, _, text)| text)
        .skip_while(|text| text != "Open patch\u{2026}")
        .filter(|text| !text.starts_with("Resize window"))
        .collect()
}

/// The panel's Button labeled `label`, `None` when it is not drawn.
fn panel_button(
    window: &mut Headless,
    view: &mut SaveView,
    label: &str,
) -> Option<accesskit::Node> {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    panel_nodes(&pairs)
        .into_iter()
        .find(|node| node.role() == Role::Button && node.label() == Some(label))
        .cloned()
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

/// The open save's revision.
fn revision(view: &SaveView) -> u64 {
    view.session.as_ref().expect("an open save").revision()
}

/// `player_id`'s entry after `text`'s patch is applied, through the lib,
/// to a copy of the open save.
fn patched_entry(view: &SaveView, text: &str, player_id: u32) -> PlayerEntry {
    let mut copy = view.session.as_ref().expect("an open save").file().clone();
    AestheticsPatch::parse(text)
        .expect("the patch parses")
        .apply(&mut copy)
        .expect("the patch applies");
    copy.player(player_id).expect("the patched player").clone()
}

/// The `/co/` patch's table rows for 71403 on the PES 21 fixture (item,
/// old value, new value), in the views' terms and the comparison's order:
/// the fixture stores his corner kick and hunching (dribbling) motions 0
/// (shown 1), and his name and shirt name are `PLACEHOLDER`.
const CO_ROWS: [[&str; 3]; 4] = [
    ["PLACEHOLDER \u{b7} Corner kick", "1", "3"],
    ["PLACEHOLDER \u{b7} Hunching (dribbling)", "1", "2"],
    ["PLACEHOLDER \u{b7} Name", "PLACEHOLDER", "PATCHED"],
    ["PLACEHOLDER \u{b7} Shirt", "PLACEHOLDER", "PATCHED"],
];

/// `rows` under the table's header row, as `panel_lines` reads them: cell
/// after cell, row by row.
fn table(rows: &[[&str; 3]]) -> Vec<String> {
    ["Item", "Old value", "New value"]
        .iter()
        .chain(rows.iter().flatten())
        .map(|cell| (*cell).to_owned())
        .collect()
}

/// Team 999 `/zz/`'s block. The PES 21 fixture holds every team of the
/// teams list (701 to 920), so the team it lacks is the command line
/// suite's own, no cup's.
const ZZ_BLOCK: &str =
    "\n[[teams]]\nid = 999\nname = \"/zz/\"\n\n[teams.players.01]\nname = \"NOBODY\"\n";

// SE-GUI-18
#[test]
fn the_preview_lists_each_changed_value_and_writes_nothing() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_preview");
    let before = revision(&view);
    previewed(
        &mut window,
        &mut view,
        &patch_text(21, ALLOCATION_SCHEME_VERSION, &co_block()),
    );

    let mut expected = vec![
        "Open patch\u{2026}".to_owned(),
        "patch.toml: team 714 /co/, players 03".to_owned(),
        "Apply".to_owned(),
    ];
    expected.extend(table(&CO_ROWS));
    assert_eq!(panel_lines(&mut window, &mut view), expected);
    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    assert!(!apply.is_disabled(), "Apply is enabled");
    assert_eq!(revision(&view), before, "the preview writes nothing");
    assert!(!view.session.as_ref().unwrap().can_undo(), "no undo entry");
}

// SE-GUI-18
#[test]
fn apply_is_one_undoable_edit_and_the_list_gives_way_to_the_applied_line() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_apply");
    let text = patch_text(21, ALLOCATION_SCHEME_VERSION, &co_block());
    let before = player_entry(&view, CO_SLOT_3);
    let expected = patched_entry(&view, &text, CO_SLOT_3);
    assert_ne!(before, expected, "premise: the patch changes 71403");
    previewed(&mut window, &mut view, &text);

    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    click(&mut window, &apply, &mut view);
    assert_eq!(player_entry(&view, CO_SLOT_3), expected, "the lib's write");
    assert_eq!(
        panel_lines(&mut window, &mut view),
        [
            "Open patch\u{2026}",
            "patch.toml: team 714 /co/, players 03",
            "Apply",
            "Applied. The changes stay unsaved until Save.",
        ]
    );
    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    assert!(apply.is_disabled(), "nothing left to apply");

    // Ctrl+Z until the stack is empty: one press, the Apply's.
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo entry");
    assert_eq!(player_entry(&view, CO_SLOT_3), before, "71403 as before");
    let lines = panel_lines(&mut window, &mut view);
    assert_eq!(lines[3..], table(&CO_ROWS));
    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    assert!(!apply.is_disabled(), "Apply is enabled again");
}

// SE-GUI-18
#[test]
fn a_patch_of_another_version_is_refused_with_the_save_untouched() {
    let (mut view, mut window) = open_save(PesVersion::Pes17, "patch_refused");
    let before = revision(&view);
    let path = previewed(
        &mut window,
        &mut view,
        &patch_text(21, ALLOCATION_SCHEME_VERSION, &co_block()),
    );

    assert_eq!(
        panel_lines(&mut window, &mut view),
        [
            "Open patch\u{2026}".to_owned(),
            format!(
                "{}: the patch is for PES 2021, the save is PES 2017",
                path.display()
            ),
        ]
    );
    assert!(
        panel_button(&mut window, &mut view, "Apply").is_none(),
        "no Apply button"
    );
    assert_eq!(revision(&view), before, "the save untouched");
}

// SE-GUI-18
#[test]
fn a_team_the_save_lacks_is_a_warning_line_and_the_rest_previews() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_skipped_team");
    assert!(
        view.session.as_ref().unwrap().file().team(999).is_none(),
        "premise: the fixture lacks team 999"
    );
    previewed(
        &mut window,
        &mut view,
        &patch_text(
            21,
            ALLOCATION_SCHEME_VERSION,
            &format!("{}{ZZ_BLOCK}", co_block()),
        ),
    );

    let mut expected = vec![
        "Open patch\u{2026}".to_owned(),
        "patch.toml: team 714 /co/, players 03".to_owned(),
        "Apply".to_owned(),
        "Team 999 /zz/ is not in this save: its players are skipped.".to_owned(),
    ];
    expected.extend(table(&CO_ROWS));
    assert_eq!(panel_lines(&mut window, &mut view), expected);
}

// SE-GUI-18
#[test]
fn a_patch_the_save_holds_no_player_of_shows_its_warnings_alone() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_all_skipped");
    assert!(
        view.session.as_ref().unwrap().file().team(999).is_none(),
        "premise: the fixture lacks team 999"
    );
    previewed(
        &mut window,
        &mut view,
        &patch_text(21, ALLOCATION_SCHEME_VERSION, ZZ_BLOCK),
    );

    assert_eq!(
        panel_lines(&mut window, &mut view),
        [
            "Open patch\u{2026}",
            "patch.toml",
            "Apply",
            "Team 999 /zz/ is not in this save: its players are skipped.",
        ]
    );
    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    assert!(apply.is_disabled(), "nothing to apply");
}

// SE-GUI-18
#[test]
fn the_preview_follows_the_save_and_an_applied_patch_lists_nothing() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_live");
    let text = patch_text(21, ALLOCATION_SCHEME_VERSION, &co_block());
    let patched = patched_entry(&view, &text, CO_SLOT_3);
    previewed(&mut window, &mut view, &text);
    assert!(
        panel_lines(&mut window, &mut view).contains(&CO_ROWS[0][0].to_owned()),
        "premise: the corner kick row is listed"
    );

    // The corner kick set to the patch's value, as an edit would.
    let mut entry = player_entry(&view, CO_SLOT_3);
    assert_ne!(
        entry.motion.corner_kick, patched.motion.corner_kick,
        "premise: the patch changes the corner kick"
    );
    entry.motion.corner_kick = patched.motion.corner_kick;
    view.session.as_mut().unwrap().edit_players(vec![entry]);
    let lines = panel_lines(&mut window, &mut view);
    assert_eq!(lines[3..], table(&CO_ROWS[1..]));

    // Every value of the patch already held: nothing to list or apply.
    view.session.as_mut().unwrap().edit_players(vec![patched]);
    assert_eq!(
        panel_lines(&mut window, &mut view),
        [
            "Open patch\u{2026}",
            "patch.toml: team 714 /co/, players 03",
            "Apply",
            "The save already holds every value this patch writes.",
        ]
    );
    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    assert!(apply.is_disabled(), "nothing to apply");
}

// SE-GUI-18
#[test]
fn a_rows_click_selects_its_player() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_row_click");
    previewed(
        &mut window,
        &mut view,
        &patch_text(21, ALLOCATION_SCHEME_VERSION, &co_block()),
    );
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
