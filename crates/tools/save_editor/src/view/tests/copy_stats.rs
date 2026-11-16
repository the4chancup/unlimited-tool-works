//! The Copy/swap stats window (SE-GUI-78): the Player menu's one
//! stat-moving entry opens a non-modal window whose Source row is picked
//! once and whose Destination row follows the list's selection — Copy
//! and Swap each land as one undo entry without closing it. The field
//! set's coverage is SE-GUI-24's.

use super::player::{mark_copy_players, rostered, slot_label, with_stats_of};
use super::*;
use crate::test_support::cup_save::outside_cup_file;

/// The Source/Destination row's two dropdowns (team, then player): the
/// largest group of the row's combos sharing one top edge. The window's
/// own two stand on one grid row; a control on the view behind can stand
/// within a point of the row's y without sharing its exact top.
pub(super) fn row_combos<'a>(nodes: &'a [accesskit::Node], row: &str) -> [&'a accesskit::Node; 2] {
    let mut groups: Vec<Vec<&accesskit::Node>> = Vec::new();
    for combo in right_of_label(nodes, row, Role::ComboBox) {
        let y0 = combo.bounds().expect("a combo's bounds").y0;
        match groups
            .iter_mut()
            .find(|group: &&mut Vec<&accesskit::Node>| {
                (group[0].bounds().expect("bounds").y0 - y0).abs() <= 0.5
            }) {
            Some(group) => group.push(combo),
            None => groups.push(vec![combo]),
        }
    }
    let combos = groups
        .iter()
        .max_by_key(|group| group.len())
        .expect("the row's dropdowns");
    assert_eq!(combos.len(), 2, "{row}: the team and player dropdowns");
    [combos[0], combos[1]]
}

/// Opens the Player menu's Copy/swap stats… and returns the nodes of a
/// settled frame (a new window's first frame is egui's sizing pass).
pub(super) fn open_window(window: &mut Headless, view: &mut SaveView) -> Vec<accesskit::Node> {
    let nodes = open_menu(window, view, "Player", "Copy/swap stats");
    click(window, menu_item(&nodes, "Copy/swap stats"), view);
    assert!(view.copy_swap.is_some(), "the window opened");
    draw(window, view);
    draw(window, view)
}

/// The window's bounds (`Role::Window` labeled by its title).
fn window_bounds(nodes: &[accesskit::Node]) -> accesskit::Rect {
    nodes
        .iter()
        .find(|node| node.role() == Role::Window && node.label() == Some("Copy/swap stats"))
        .and_then(accesskit::Node::bounds)
        .expect("the Copy/swap stats window")
}

/// Picks `label` in `combo`'s open list. A list row can carry the same
/// label, so the pick is scoped to the popup's `ListBox` subtree, and it
/// goes by a pointer click: an AccessKit action sets the pick but leaves
/// the popup open, and an open popup (the window stands low, so it opens
/// upward) then swallows the next pick's click.
pub(super) fn pick_option(
    window: &mut Headless,
    view: &mut SaveView,
    combo: &accesskit::Node,
    label: &str,
) {
    click(window, combo, view);
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let parent: HashMap<accesskit::NodeId, accesskit::NodeId> = nodes
        .iter()
        .flat_map(|(id, node)| node.children().iter().map(move |child| (*child, *id)))
        .collect();
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        nodes.iter().map(|(id, node)| (*id, node)).collect();
    let in_listbox = |mut at: accesskit::NodeId| {
        for _ in 0..64 {
            if by_id[&at].role() == Role::ListBox {
                return true;
            }
            let Some(id) = parent.get(&at) else {
                return false;
            };
            at = *id;
        }
        panic!("a node deeper than 64")
    };
    let (option, _) = nodes
        .iter()
        .find(|(id, node)| node.label() == Some(label) && in_listbox(*id))
        .unwrap_or_else(|| panic!("no {label:?} in the combo"));
    click(window, by_id[option], view);
}

/// The Destination row's player dropdown's shown text.
fn destination_label(nodes: &[accesskit::Node]) -> String {
    row_combos(nodes, "Destination")[1]
        .value()
        .expect("the destination's shown text")
        .to_owned()
}

// The entry opens a window titled Copy/swap stats with Source and
// Destination both on the selected player and Copy and Swap disabled —
// their hover naming why — while the view under the open window keeps
// taking clicks: a click on another player row selects it.
// SE-GUI-78
#[test]
fn the_window_opens_on_the_selected_player_and_the_list_under_it_takes_clicks() {
    let path = fixture(PesVersion::Pes21, "copy_window_open");
    let mut view = SaveView::new();
    view.open(&path);
    // One team in the list: the row labels are unique to it.
    view.team = TeamChoice::Team(701);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (a_slot, a_id) = rostered(&view, 701)
        .first()
        .copied()
        .expect("a filled slot");
    let a = player_entry(&view, a_id);
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, a_slot as u8)),
        "premise: A is selected"
    );

    let nodes = open_window(&mut window, &mut view);
    window_bounds(&nodes);
    let a_label = slot_label(&a, a_slot);
    let [_, source] = row_combos(&nodes, "Source");
    let [_, destination] = row_combos(&nodes, "Destination");
    assert_eq!(
        source.value(),
        Some(a_label.as_str()),
        "the Source starts on A"
    );
    assert_eq!(
        destination.value(),
        Some(a_label.as_str()),
        "the Destination starts on A"
    );
    for label in ["Copy", "Swap"] {
        let button = node_labeled(&nodes, Role::Button, label);
        assert!(
            button.is_disabled(),
            "{label} disabled while both rows name one player"
        );
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(button), "same player"),
            Some("The source and the destination are the same player".to_string()),
            "{label}'s hover"
        );
    }

    // A click on ANOTHER row under the open window selects it — clicking
    // A's own row would prove nothing.
    let (b_slot, b_id) = rostered(&view, 701)[1];
    let b = player_entry(&view, b_id);
    let nodes = draw(&mut window, &mut view);
    click(&mut window, row(&nodes, &slot_label(&b, b_slot)), &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, b_slot as u8)),
        "the row click under the window selects it"
    );
    assert!(view.copy_swap.is_some(), "the window stays open");
}

// The window's grid spaces its rows by the gap between their boxes, as the
// draft's does: the Destination row's two dropdowns stand 6 under the
// Source row's, 24 apart top to top.
#[test]
fn the_destination_dropdowns_stand_24_under_the_source_ones() {
    let path = fixture(PesVersion::Pes21, "copy_window_gaps");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = open_window(&mut window, &mut view);
    let top = |node: &accesskit::Node| node.bounds().expect("a dropdown's bounds").y0;
    let source = row_combos(&nodes, "Source").map(top);
    let destination = row_combos(&nodes, "Destination").map(top);
    for (column, (upper, lower)) in ["team", "player"]
        .iter()
        .zip(source.iter().zip(destination))
    {
        assert!(
            (lower - upper - 24.0).abs() <= 0.5,
            "the Destination {column} dropdown stands {} under the Source one, not 24",
            lower - upper
        );
    }
}

// Copy with the Source's player dropdown on B: A holds B's field set
// (height, an ability, the registered position and a playable position;
// A's name and appearance stay), the window is still open and reads
// `Copied B's stats to A.`, and one Undo restores A.
// SE-GUI-78
#[test]
fn copy_writes_the_sources_field_set_and_the_window_stays_open() {
    let path = fixture(PesVersion::Pes21, "copy_window_copy");
    mark_copy_players(&path, PesVersion::Pes21);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let a_before = player_entry(&view, 70101);
    let b_before = player_entry(&view, 70102);
    assert_ne!(
        a_before.basic.height, b_before.basic.height,
        "premise: the heights differ"
    );

    let nodes = open_window(&mut window, &mut view);
    pick_option(
        &mut window,
        &mut view,
        row_combos(&nodes, "Source")[1],
        &slot_label(&b_before, 1),
    );
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Copy"),
        &mut view,
    );

    let a_after = player_entry(&view, 70101);
    assert_eq!(
        a_after,
        with_stats_of(&a_before, &b_before),
        "A holds B's field set and keeps the rest"
    );
    assert_eq!(a_after.basic.height, b_before.basic.height, "B's height");
    assert_eq!(a_after.stats.speed, b_before.stats.speed, "B's speed");
    assert_eq!(
        a_after.positions.registered, b_before.positions.registered,
        "B's registered position"
    );
    assert_eq!(
        a_after.positions.playable[0], b_before.positions.playable[0],
        "B's playable position"
    );
    assert_eq!(a_after.name, a_before.name, "A's name stays");
    assert_eq!(
        a_after.appearance.boots_id, a_before.appearance.boots_id,
        "A's appearance stays"
    );
    assert_eq!(
        player_entry(&view, 70102),
        b_before,
        "the source is untouched"
    );

    assert!(view.copy_swap.is_some(), "the window stays open");
    let nodes = draw(&mut window, &mut view);
    let line = format!(
        "Copied {}'s stats to {}.",
        display_name(&b_before.name),
        display_name(&a_before.name)
    );
    assert!(label_value(&nodes, &line), "the result line reads {line:?}");

    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(player_entry(&view, 70101), a_before, "one Undo restores A");
}

// A click on player C's row moves the Destination to C (read before and
// after the click: the click is what moves it), and a second Copy — no
// menu opened in between — writes C. Down and Up move the selection —
// and the Destination — the same way.
// SE-GUI-78
#[test]
fn the_list_selection_picks_each_destination() {
    let path = fixture(PesVersion::Pes21, "copy_window_list");
    mark_copy_players(&path, PesVersion::Pes21);
    let mut view = SaveView::new();
    view.open(&path);
    // One team in the list: the row labels are unique to it.
    view.team = TeamChoice::Team(701);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let b_before = player_entry(&view, 70102);
    let c_before = player_entry(&view, 70103);
    let d_before = player_entry(&view, 70104);

    let nodes = open_window(&mut window, &mut view);
    pick_option(
        &mut window,
        &mut view,
        row_combos(&nodes, "Source")[1],
        &slot_label(&b_before, 1),
    );

    // Click C's row in the list under the open window.
    let nodes = draw(&mut window, &mut view);
    let before = destination_label(&nodes);
    assert_eq!(
        before,
        slot_label(&player_entry(&view, 70101), 0),
        "premise: the Destination was A"
    );
    click(
        &mut window,
        row(&nodes, &slot_label(&c_before, 2)),
        &mut view,
    );
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 2)),
        "the click selected C"
    );
    assert_eq!(
        destination_label(&nodes),
        slot_label(&c_before, 2),
        "the Destination followed the click to C"
    );

    // A second Copy writes C with no menu opened in between.
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Copy"),
        &mut view,
    );
    assert_eq!(
        player_entry(&view, 70103),
        with_stats_of(&c_before, &b_before),
        "C holds B's field set"
    );

    // Down and Up move the selection the same way: the Destination
    // follows to the next player (slot 3) and back to C.
    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 3)),
        "Down moved the selection under the open window"
    );
    assert_eq!(
        destination_label(&nodes),
        slot_label(&d_before, 3),
        "the Destination followed Down to the next player"
    );
    window.frame(
        vec![key(egui::Key::ArrowUp, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let nodes = draw(&mut window, &mut view);
    // C's label is read from his CURRENT entry: the copy moved B's
    // registered position into it.
    assert_eq!(
        destination_label(&nodes),
        slot_label(&player_entry(&view, 70103), 2),
        "the Destination followed Up back to C"
    );
}

// Swap: the Source and Destination pick each other up — B and C hold
// each other's field set — as one undoable edit that the line reports.
// SE-GUI-78
#[test]
fn swap_writes_both_field_sets_as_one_undoable_edit() {
    let path = fixture(PesVersion::Pes21, "copy_window_swap");
    mark_copy_players(&path, PesVersion::Pes21);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let b_before = player_entry(&view, 70102);
    let c_before = player_entry(&view, 70103);
    assert_ne!(b_before.stats, c_before.stats, "premise");

    let nodes = open_window(&mut window, &mut view);
    pick_option(
        &mut window,
        &mut view,
        row_combos(&nodes, "Source")[1],
        &slot_label(&b_before, 1),
    );
    let nodes = draw(&mut window, &mut view);
    pick_option(
        &mut window,
        &mut view,
        row_combos(&nodes, "Destination")[1],
        &slot_label(&c_before, 2),
    );
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Swap"),
        &mut view,
    );

    assert_eq!(
        player_entry(&view, 70102),
        with_stats_of(&b_before, &c_before),
        "B holds C's field set"
    );
    assert_eq!(
        player_entry(&view, 70103),
        with_stats_of(&c_before, &b_before),
        "C holds B's field set"
    );
    let nodes = draw(&mut window, &mut view);
    let line = format!(
        "Swapped the stats of {} and {}.",
        display_name(&b_before.name),
        display_name(&c_before.name)
    );
    assert!(label_value(&nodes, &line), "the result line reads {line:?}");

    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(player_entry(&view, 70102), b_before, "one Undo restores B");
    assert_eq!(player_entry(&view, 70103), c_before, "one Undo restores C");
}

// A row's team dropdown changed: its player dropdown shows the new
// team's first filled slot — labeled by the roster place, not the shirt
// number — even when the pick stood on another slot and the team's slot
// 01 is empty.
// SE-GUI-78
#[test]
fn a_changed_team_dropdown_takes_the_teams_first_filled_slot() {
    let path = fixture(PesVersion::Pes21, "copy_window_team_pick");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    // The selected player's shirt number moved off his place, so the
    // dropdown label shows which one it reads.
    let session = view.session.as_mut().unwrap();
    let mut team = session.file().team(701).unwrap().clone();
    team.roster[0].number = 19;
    session.edit_team(1, team);
    assert_ne!(usize::from(team_entry(&view, 701).roster[0].number), 1);
    let second_name = display_name(&team_entry(&view, 702).name);

    let nodes = open_window(&mut window, &mut view);
    // Both rows on a slot past 01 before the team change, so a pick that
    // never moved shows.
    let (third_slot, third_id) = rostered(&view, 701)[2];
    let third = slot_label(&player_entry(&view, third_id), third_slot);
    let (fourth_slot, fourth_id) = rostered(&view, 701)[3];
    let fourth = slot_label(&player_entry(&view, fourth_id), fourth_slot);
    pick_option(
        &mut window,
        &mut view,
        row_combos(&nodes, "Source")[1],
        &fourth,
    );
    let nodes = draw(&mut window, &mut view);
    pick_option(
        &mut window,
        &mut view,
        row_combos(&nodes, "Destination")[1],
        &third,
    );
    // 702's slot 01 emptied, so its first filled slot is 02.
    let session = view.session.as_mut().unwrap();
    let mut team = session.file().team(702).unwrap().clone();
    assert_ne!(team.roster[0].player_id, 0, "premise: 702's 01 filled");
    team.roster[0].player_id = 0;
    session.edit_team(1, team);
    let (second_slot, second_id) = rostered(&view, 702)
        .first()
        .copied()
        .expect("702 has a filled slot");
    assert_eq!(second_slot, 1, "premise: 702's first filled slot is 02");
    let second = player_entry(&view, second_id);

    let nodes = draw(&mut window, &mut view);
    pick_option(
        &mut window,
        &mut view,
        row_combos(&nodes, "Destination")[0],
        &second_name,
    );
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        destination_label(&nodes),
        slot_label(&second, second_slot),
        "the Destination's player is the new team's first filled slot"
    );

    // The Source row's team dropdown behaves the same.
    pick_option(
        &mut window,
        &mut view,
        row_combos(&nodes, "Source")[0],
        &second_name,
    );
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        row_combos(&nodes, "Source")[1].value(),
        Some(slot_label(&second, second_slot).as_str()),
        "the Source's player is the new team's first filled slot"
    );
}

// With a team row selected the entry stays enabled — it names both
// players itself — and opens with both rows on the team's first filled
// slot.
// SE-GUI-78
#[test]
fn a_team_row_selection_opens_the_window_on_the_teams_first_filled() {
    let path = fixture(PesVersion::Pes21, "copy_window_team_row");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    // 701's slot 01 emptied: its first filled slot is 02.
    let session = view.session.as_mut().unwrap();
    let mut team = session.file().team(701).unwrap().clone();
    assert_ne!(team.roster[0].player_id, 0, "premise: 701's 01 filled");
    team.roster[0].player_id = 0;
    session.edit_team(1, team);
    open_team_view(&mut window, &mut view, 701);
    let (first_slot, first_id) = rostered(&view, 701)
        .first()
        .copied()
        .expect("701 has a filled slot");
    let first = player_entry(&view, first_id);

    let nodes = open_menu(&mut window, &mut view, "Player", "Copy/swap stats");
    let entry = menu_item(&nodes, "Copy/swap stats");
    assert!(
        !entry.is_disabled(),
        "the entry stays enabled on a team row"
    );
    click(&mut window, entry, &mut view);
    assert!(view.copy_swap.is_some(), "the window opened");
    draw(&mut window, &mut view);
    let nodes = draw(&mut window, &mut view);
    let label = slot_label(&first, first_slot);
    assert_eq!(
        row_combos(&nodes, "Source")[1].value(),
        Some(label.as_str()),
        "the Source starts on the team's first filled slot"
    );
    assert_eq!(
        destination_label(&nodes),
        label,
        "the Destination starts on the team's first filled slot"
    );
}

// A team row clicked under the open window gives that team's first
// filled player as the Destination, as the window's opening on a team
// row does.
#[test]
fn a_team_row_click_under_the_window_gives_its_first_player() {
    let path = fixture(PesVersion::Pes21, "copy_window_team_click");
    // The marked players' names single A out: a Destination that stayed
    // on him reads "01 … DEST", not the clicked team's first player's.
    mark_copy_players(&path, PesVersion::Pes21);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (first_slot, first_id) = rostered(&view, 702)
        .first()
        .copied()
        .expect("702 has a filled slot");
    let first = player_entry(&view, first_id);

    let nodes = open_window(&mut window, &mut view);
    // The Destination on a slot past 01 first, so a pick that never
    // moved shows.
    let (third_slot, third_id) = rostered(&view, 701)[2];
    let third = slot_label(&player_entry(&view, third_id), third_slot);
    pick_option(
        &mut window,
        &mut view,
        row_combos(&nodes, "Destination")[1],
        &third,
    );
    let nodes = draw(&mut window, &mut view);
    click(&mut window, team_row(&nodes, &view, 702), &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Team(702)),
        "the click selected 702's row"
    );
    assert!(view.copy_swap.is_some(), "the window stays open");

    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        destination_label(&nodes),
        slot_label(&first, first_slot),
        "the Destination is the clicked team's first filled slot"
    );
}

// A roster write under the open window re-reads each pick's player off
// the roster: the Destination dropdown names the player the slot now
// holds and Copy writes him. The test writes the roster itself
// (`edit_team` trading two slots' player ids), a state the window
// defends against rather than one a member's edit makes: a move trades
// the players' records and leaves the roster's ids where they are.
#[test]
fn a_roster_write_under_the_window_moves_the_pick_with_the_slot() {
    let path = fixture(PesVersion::Pes21, "copy_window_roster_write");
    mark_copy_players(&path, PesVersion::Pes21);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let a_before = player_entry(&view, 70101);
    let b_before = player_entry(&view, 70102);

    // The window open on Source 701's 01 and Destination 701's 02.
    let nodes = open_window(&mut window, &mut view);
    pick_option(
        &mut window,
        &mut view,
        row_combos(&nodes, "Destination")[1],
        &slot_label(&b_before, 1),
    );

    // 01 and 02 trade roster places: slot 02 now holds another player.
    let session = view.session.as_mut().unwrap();
    let mut team = session.file().team(701).unwrap().clone();
    (team.roster[0].player_id, team.roster[1].player_id) =
        (team.roster[1].player_id, team.roster[0].player_id);
    session.edit_team(1, team);
    assert_eq!(
        team_entry(&view, 701).roster[1].player_id,
        70101,
        "premise: slot 02 holds another player"
    );

    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        destination_label(&nodes),
        slot_label(&a_before, 1),
        "the Destination dropdown names the player 02 now holds"
    );
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Copy"),
        &mut view,
    );
    assert_eq!(
        player_entry(&view, 70101),
        with_stats_of(&a_before, &b_before),
        "Copy wrote the player 02 now holds"
    );
}

// Ctrl+C then Ctrl+V on the player list's rows under the open window
// copy and paste as they do without it: A's stats land on B's row.
#[test]
fn ctrl_c_and_ctrl_v_work_under_the_open_window() {
    let path = fixture(PesVersion::Pes21, "copy_window_clipboard");
    mark_copy_players(&path, PesVersion::Pes21);
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let a_before = player_entry(&view, 70101);
    let b_before = player_entry(&view, 70102);
    let expected = with_stats_of(&b_before, &a_before);
    assert_ne!(expected, b_before, "premise: the paste changes B");

    let nodes = open_window(&mut window, &mut view);
    click(
        &mut window,
        row(&nodes, &slot_label(&a_before, 0)),
        &mut view,
    );
    window.frame(vec![egui::Event::Copy], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let copied = window
        .commands
        .iter()
        .find_map(|command| match command {
            egui::OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .expect("Ctrl+C put a text on the system clipboard");

    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        row(&nodes, &slot_label(&b_before, 1)),
        &mut view,
    );
    window.frame(vec![egui::Event::Paste(copied)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });

    assert_eq!(
        player_entry(&view, 70102),
        expected,
        "B's record is what Copy stats from A writes"
    );
    assert!(view.copy_swap.is_some(), "the window stays open");
}

// The window first stands centred horizontally, its bottom edge 56
// points above the content rect's bottom; dragged by its title bar it
// stands where it was dropped.
// SE-GUI-78
#[test]
fn the_window_stands_centred_over_the_bottom_and_stays_where_dropped() {
    let path = fixture(PesVersion::Pes21, "copy_window_drag");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_window(&mut window, &mut view);
    let id = egui::Id::unique("save_editor_copy_stats");
    let first = window
        .ctx
        .memory(|memory| memory.area_rect(id))
        .expect("the window's area");
    let content = window.ctx.content_rect();
    assert!(
        (first.center().x - content.center().x).abs() <= 1.0,
        "centred horizontally: {} vs {}",
        first.center().x,
        content.center().x
    );
    assert!(
        (first.bottom() - (content.bottom() - 56.0)).abs() <= 1.0,
        "its bottom 56 over the content's bottom: {} vs {}",
        first.bottom(),
        content.bottom() - 56.0
    );

    // Drag the title bar (its drag area's centre, off the close button).
    let bar = window
        .ctx
        .read_response(id.with("__title_click"))
        .expect("the title bar's drag area")
        .rect;
    let to = bar.center() + egui::vec2(-80.0, -60.0);
    drag(&mut window, bar.center(), to, &mut view);
    let moved = window
        .ctx
        .memory(|memory| memory.area_rect(id))
        .expect("the window's area");
    let delta = to - bar.center();
    assert!(
        (moved.min.x - (first.min.x + delta.x)).abs() <= 2.0
            && (moved.min.y - (first.min.y + delta.y)).abs() <= 2.0,
        "the window moved by the drag: {first:?} + {delta:?} = {moved:?}"
    );
    let nodes = draw(&mut window, &mut view);
    let settled = window
        .ctx
        .memory(|memory| memory.area_rect(id))
        .expect("the window's area");
    assert_eq!(settled, moved, "it stands where it was dropped");
    window_bounds(&nodes);
}

// Opening another save closes the window.
// SE-GUI-78
#[test]
fn opening_another_save_closes_the_window() {
    let path = fixture(PesVersion::Pes21, "copy_window_reopen");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_window(&mut window, &mut view);
    assert!(view.copy_swap.is_some(), "premise: the window is open");

    open_again(&mut view, &path);
    assert!(
        view.copy_swap.is_none(),
        "opening another save closes the window"
    );
}

// The window closes on its Close button and on the title bar's close
// button (`view.copy_swap` is `None` after both).
#[test]
fn the_window_closes_on_close_and_on_its_title_bar_button() {
    let path = fixture(PesVersion::Pes21, "copy_window_close");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);

    for button in ["Close", "Close window"] {
        let nodes = open_window(&mut window, &mut view);
        click(
            &mut window,
            node_labeled(&nodes, Role::Button, button),
            &mut view,
        );
        assert!(view.copy_swap.is_none(), "{button} closed the window");
    }
}

// The window's width is its grid's, held while the member works: each
// combo is its widest option plus the combo's chrome and the result
// line truncates at the grid, so a longer pick or a long line moves
// neither the window nor the buttons under the pointer.
#[test]
fn the_window_holds_its_width_while_the_picks_and_the_line_change() {
    let path = fixture(PesVersion::Pes21, "copy_window_width");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70102).unwrap().name = "A SOURCE NAME WIDER THAN THE GRID HOLDS".to_owned();
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);

    let nodes = open_window(&mut window, &mut view);
    let rect_and_copy = |nodes: &[accesskit::Node]| {
        (
            window_bounds(nodes),
            node_labeled(nodes, Role::Button, "Copy")
                .bounds()
                .expect("Copy's bounds")
                .x0,
        )
    };
    let before = rect_and_copy(&nodes);

    // The Source's player dropdown on the wide name: the window stands.
    let wide = slot_label(&player_entry(&view, 70102), 1);
    assert!(
        wide.len() > destination_label(&nodes).len(),
        "premise: the pick is wider than the open one"
    );
    pick_option(
        &mut window,
        &mut view,
        row_combos(&nodes, "Source")[1],
        &wide,
    );
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        rect_and_copy(&nodes),
        before,
        "a wider pick moves neither the window nor Copy"
    );

    // A Copy whose line outgrows the grid, then nothing moved either.
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Copy"),
        &mut view,
    );
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        rect_and_copy(&nodes),
        before,
        "a wide result line moves neither the window nor Copy"
    );

    // A row's team changed to one wider-named than 701's and differently
    // widest: each combo is as wide as the widest of every cup team, so
    // still nothing moves.
    let (other_team, team_widest, chrome) = {
        let file = view.session.as_ref().unwrap().file();
        let labels_of = |team: u32| {
            rostered(&view, team)
                .iter()
                .map(|&(slot, id)| {
                    file.player(id).map_or_else(
                        || format!("{:02} ?", slot + 1),
                        |player| slot_label(player, slot),
                    )
                })
                .collect::<Vec<_>>()
        };
        let teams: Vec<u32> = crate::session::cup::cup_teams(file)
            .map(|team| team.id)
            .collect();
        window
            .frame(Vec::new(), |ui| {
                let font = egui::TextStyle::Button.resolve(ui.style());
                let text_w = |text: &str| {
                    ui.painter()
                        .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::PLACEHOLDER)
                        .size()
                        .x
                };
                let widest_of = |team: u32| {
                    labels_of(team)
                        .iter()
                        .map(|label| text_w(label))
                        .fold(0.0_f32, f32::max)
                };
                let name_w = |team: u32| text_w(&display_name(&file.team(team).unwrap().name));
                let spacing = ui.spacing();
                (
                    teams.iter().copied().find(|&id| {
                        (widest_of(id) - widest_of(701)).abs() > 0.5
                            && name_w(id) > name_w(701) + 0.5
                    }),
                    teams.iter().copied().map(name_w).fold(0.0_f32, f32::max),
                    2.0 * spacing.button_padding.x + spacing.icon_spacing + spacing.icon_width,
                )
            })
            .0
    };
    let other_team = other_team.expect("a team wider-named than 701 and differently widest");
    let other_name = display_name(&team_entry(&view, other_team).name);
    let combo_widths = |nodes: &[accesskit::Node]| {
        ["Source", "Destination"].map(|row| {
            row_combos(nodes, row).map(|combo| {
                let bounds = combo.bounds().expect("bounds");
                bounds.x1 - bounds.x0
            })
        })
    };
    let widths_before = combo_widths(&nodes);
    for (row, widths) in ["Source", "Destination"].iter().zip(widths_before) {
        assert!(
            (widths[0] - f64::from(team_widest + chrome)).abs() <= 0.5,
            "the {row} Team combo is the widest name's plus chrome: {widths:?} vs {}",
            team_widest + chrome
        );
    }
    let nodes = draw(&mut window, &mut view);
    pick_option(
        &mut window,
        &mut view,
        row_combos(&nodes, "Destination")[0],
        &other_name,
    );
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        rect_and_copy(&nodes),
        before,
        "a team change moves neither the window nor Copy"
    );
    assert_eq!(
        combo_widths(&nodes),
        widths_before,
        "the four combos keep their width"
    );

    // An idle frame keeps the measured pair: a seeded pair at this
    // revision is used as it is, not re-measured — a pair wider than
    // every option shows the floor each combo's `.width` gives it.
    let revision = view.session.as_ref().unwrap().revision();
    view.copy_swap.as_mut().expect("the window").widths = Some((revision, (400.0, 500.0)));
    let nodes = draw(&mut window, &mut view);
    let expected = [
        f64::from(400.0 + chrome),
        f64::from(500.0 + chrome),
        f64::from(400.0 + chrome),
        f64::from(500.0 + chrome),
    ];
    assert_eq!(
        combo_widths(&nodes).concat(),
        expected,
        "the seeded pair is used as is"
    );
    assert_eq!(
        view.copy_swap.as_ref().unwrap().widths,
        Some((revision, (400.0, 500.0))),
        "and kept, not re-measured"
    );

    // A rename wider than every label on offer re-measures on the new
    // revision: the Player combos widen to fit him.
    let mut entry = player_entry(&view, 70101);
    entry.name = "A PLAYER NAME WIDER THAN EVERY LABEL THE LIST OFFERS".to_owned();
    let new_label = slot_label(&entry, 0);
    view.session.as_mut().unwrap().edit_players(vec![entry]);
    let new_widest = window
        .frame(Vec::new(), |ui| {
            ui.painter()
                .layout_no_wrap(
                    new_label.clone(),
                    egui::TextStyle::Button.resolve(ui.style()),
                    egui::Color32::PLACEHOLDER,
                )
                .size()
                .x
        })
        .0;
    assert!(
        f64::from(new_widest) > widths_before[0][1],
        "premise: the rename is wider than the list's widest was"
    );
    let nodes = draw(&mut window, &mut view);
    let players_now = combo_widths(&nodes).map(|widths| widths[1]);
    assert_eq!(
        players_now,
        [
            f64::from(new_widest + chrome),
            f64::from(new_widest + chrome)
        ],
        "the re-measure widened both Player combos to fit him"
    );
}

// The window's team dropdown names a team by its display name — the
// shown text and the entries alike — a colour code's control character
// and hex digits never show.
#[test]
fn the_team_dropdown_names_a_coded_team_by_its_display_name() {
    let path = fixture(PesVersion::Pes21, "copy_window_display_name");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let name = file.team(701).unwrap().name.clone();
    file.team_mut(701).unwrap().name = format!("\x11cff0000ff{name}");
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);

    let nodes = open_window(&mut window, &mut view);
    let team_combo = row_combos(&nodes, "Source")[0];
    assert_eq!(team_combo.value(), Some(name.as_str()), "the shown text");

    click(&mut window, team_combo, &mut view);
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let entries: Vec<&str> = nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button)
        .filter_map(|(_, node)| node.label())
        .collect();
    assert!(entries.contains(&name.as_str()), "the entry: {entries:?}");
    assert!(
        entries.iter().all(|entry| !entry.contains('\u{11}')),
        "no colour code byte in the popup: {entries:?}"
    );
}

// Each combo steps with the wheel over it: a notch down takes the next
// item — a team change moving the player pick to the new team's first
// filled slot — and a notch up takes the one before.
#[test]
fn the_combos_step_with_the_wheel() {
    let path = fixture(PesVersion::Pes21, "copy_window_wheel");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (a_slot, a_id) = rostered(&view, 701)[0];
    let (b_slot, b_id) = rostered(&view, 701)[1];
    let (a, b) = (player_entry(&view, a_id), player_entry(&view, b_id));
    let first_team = display_name(&team_entry(&view, 701).name);
    let second_team = display_name(&team_entry(&view, 702).name);

    let nodes = open_window(&mut window, &mut view);
    // The player combo: down to the next slot, up back to 01.
    let at = centre(row_combos(&nodes, "Destination")[1]);
    notch(&mut window, &mut view, at, false);
    idle_frames(&mut window, &mut view, 3);
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        destination_label(&nodes),
        slot_label(&b, b_slot),
        "a notch down takes the next player"
    );
    notch(&mut window, &mut view, at, true);
    idle_frames(&mut window, &mut view, 3);
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        destination_label(&nodes),
        slot_label(&a, a_slot),
        "a notch up takes the one before"
    );

    // The team combo the same: down to the next cup team — its player
    // pick moving to the new team's first filled slot — and back.
    let at = centre(row_combos(&nodes, "Destination")[0]);
    notch(&mut window, &mut view, at, false);
    idle_frames(&mut window, &mut view, 3);
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        row_combos(&nodes, "Destination")[0].value(),
        Some(second_team.as_str()),
        "a notch down takes the next team"
    );
    let (slot, id) = rostered(&view, 702)
        .first()
        .copied()
        .expect("702 has a filled slot");
    assert_eq!(
        destination_label(&nodes),
        slot_label(&player_entry(&view, id), slot),
        "and its player pick moved to the first filled slot"
    );
    notch(&mut window, &mut view, at, true);
    idle_frames(&mut window, &mut view, 3);
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        row_combos(&nodes, "Destination")[0].value(),
        Some(first_team.as_str()),
        "a notch up takes the team back"
    );
}

// The button row's order: Copy, Swap, Close, left to right.
#[test]
fn the_buttons_stand_copy_swap_close_left_to_right() {
    let path = fixture(PesVersion::Pes21, "copy_window_buttons");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = open_window(&mut window, &mut view);
    let x = |label: &str| {
        let bounds = node_labeled(&nodes, Role::Button, label)
            .bounds()
            .expect("bounds");
        (bounds.x0, bounds.x1)
    };
    let (copy, swap, close) = (x("Copy"), x("Swap"), x("Close"));
    assert!(
        copy.1 <= swap.0 && swap.1 <= close.0,
        "Copy, Swap, Close left to right ({copy:?} {swap:?} {close:?})"
    );
}

// The team combo lists the cup's teams, not the list's ticked
// category's: under VGL it still offers 701 (a 4cc team), and team 300,
// outside the cup's range, is never offered.
#[test]
fn the_team_combo_lists_every_cup_team_whatever_the_category() {
    let (file, _) = outside_cup_file(PesVersion::Pes21);
    let path = crate::test_support::scratch("copy_window_cup_only").join("EDIT00000000");
    std::fs::write(
        &path,
        file.to_bytes(&crate::test_support::TEST_SALT).unwrap(),
    )
    .unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team_category = TeamCategory::Vgl;
    let four_cc = display_name(&team_entry(&view, 701).name);
    let outside = display_name(&team_entry(&view, 300).name);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);

    let nodes = open_window(&mut window, &mut view);
    click(&mut window, row_combos(&nodes, "Source")[0], &mut view);
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let labels: Vec<&str> = nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button)
        .filter_map(|(_, node)| node.label())
        .collect();
    assert!(
        labels.contains(&four_cc.as_str()),
        "701 offered under the VGL tick: {labels:?}"
    );
    assert!(
        !labels.contains(&outside.as_str()),
        "team 300 never offered: {labels:?}"
    );
}

// The copy's full field set — height, weight, the whole stats, positions
// and skills, and the eight gameplay edit flags — lands on the
// destination and nothing outside it moves: names, nationality, age,
// every motion, the motion and appearance flags and the whole
// appearance stay. One undoable edit, saved and reopened.
// SE-GUI-24
#[test]
fn copy_stats_writes_the_field_set_and_nothing_else() {
    let path = fixture(PesVersion::Pes21, "player_copy");
    mark_copy_players(&path, PesVersion::Pes21);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let dest_before = player_entry(&view, 70101);
    let source_before = player_entry(&view, 70102);
    // Premises: the fields a copy must and must not carry all differ.
    assert_ne!(dest_before.stats, source_before.stats);
    assert_ne!(
        (dest_before.stats.star, dest_before.stats.playing_attitude),
        (
            source_before.stats.star,
            source_before.stats.playing_attitude
        ),
        "premise: the kept star and attitude differ"
    );
    assert_ne!(dest_before.positions, source_before.positions);
    assert_ne!(dest_before.name, source_before.name);
    assert_ne!(dest_before.motion, source_before.motion);
    assert_ne!(dest_before.appearance, source_before.appearance);

    // The window opens on the selected player's team (701); the Source's
    // player dropdown picks B by his place, position and name.
    let nodes = open_window(&mut window, &mut view);
    pick_option(
        &mut window,
        &mut view,
        row_combos(&nodes, "Source")[1],
        &slot_label(&source_before, 1),
    );
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Copy"),
        &mut view,
    );

    assert_eq!(
        player_entry(&view, 70101),
        with_stats_of(&dest_before, &source_before),
        "the field set copied, the rest 70101's own"
    );
    assert_eq!(
        player_entry(&view, 70102),
        source_before,
        "the source is untouched"
    );
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
    assert_eq!(player_entry(&view, 70101), dest_before, "restored");

    // Redo, save, reopen: the file holds the copy.
    window.frame(vec![key(egui::Key::Y, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        EditFile::load(&path).unwrap().player(70101).unwrap(),
        &with_stats_of(&dest_before, &source_before),
        "the reopened file holds the copy"
    );
}

// Swap stats: the same field set traded both ways — here on PES 16,
// whose appearance block is a separate record and stays out of the
// trade.
// SE-GUI-24
#[test]
fn swap_stats_trades_the_field_set_on_pes16() {
    let path = fixture(PesVersion::Pes16, "player_swap16");
    mark_copy_players(&path, PesVersion::Pes16);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let first_before = player_entry(&view, 70101);
    let second_before = player_entry(&view, 70102);
    assert_ne!(first_before.appearance, second_before.appearance);

    let nodes = open_window(&mut window, &mut view);
    pick_option(
        &mut window,
        &mut view,
        row_combos(&nodes, "Source")[1],
        &slot_label(&second_before, 1),
    );
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Swap"),
        &mut view,
    );

    assert_eq!(
        player_entry(&view, 70101),
        with_stats_of(&first_before, &second_before),
        "70101 takes 70102's set, keeps the rest"
    );
    assert_eq!(
        player_entry(&view, 70102),
        with_stats_of(&second_before, &first_before),
        "70102 takes 70101's set, keeps the rest"
    );
    assert_eq!(
        player_entry(&view, 70101).appearance,
        first_before.appearance,
        "the appearance record stays"
    );
    assert_eq!(
        player_entry(&view, 70102).appearance,
        second_before.appearance,
        "the appearance record stays"
    );

    // One undo trades them back; redo trades again; saved file holds it.
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
    assert_eq!(player_entry(&view, 70101), first_before);
    assert_eq!(player_entry(&view, 70102), second_before);
    window.frame(vec![key(egui::Key::Y, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let reopened = EditFile::load(&path).unwrap();
    assert_eq!(
        reopened.player(70101).unwrap(),
        &with_stats_of(&first_before, &second_before)
    );
    assert_eq!(
        reopened.player(70102).unwrap(),
        &with_stats_of(&second_before, &first_before)
    );
}

// The free-kick motion is a look the team's export owns, not a stat:
// Copy and Swap leave each player's his own, and still move the rest of
// the field set.
#[test]
fn copy_and_swap_stats_keep_each_players_free_kick_motion() {
    for button in ["Copy", "Swap"] {
        let path = fixture(PesVersion::Pes21, "player_copy_free_kick");
        mark_copy_players(&path, PesVersion::Pes21);
        let mut view = SaveView::new();
        view.open(&path);
        assert!(!view.aesthetics_locked(), "premise: the lock is off");
        let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
        let first_before = player_entry(&view, 70101);
        let second_before = player_entry(&view, 70102);
        assert_ne!(
            first_before.motion.free_kick, second_before.motion.free_kick,
            "premise: the free-kick motions differ"
        );
        assert_ne!(
            first_before.stats, second_before.stats,
            "premise: the stats differ"
        );

        let nodes = open_window(&mut window, &mut view);
        pick_option(
            &mut window,
            &mut view,
            row_combos(&nodes, "Source")[1],
            &slot_label(&second_before, 1),
        );
        let nodes = draw(&mut window, &mut view);
        click(
            &mut window,
            node_labeled(&nodes, Role::Button, button),
            &mut view,
        );

        assert_eq!(
            player_entry(&view, 70101),
            with_stats_of(&first_before, &second_before),
            "{button}: 70101 takes the field set but his own free-kick motion"
        );
        let second_after = if button == "Copy" {
            second_before.clone()
        } else {
            with_stats_of(&second_before, &first_before)
        };
        assert_eq!(
            player_entry(&view, 70102),
            second_after,
            "{button}: 70102 keeps his free-kick motion"
        );
    }
}
