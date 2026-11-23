use super::*;
use crate::view::similar;

/// The fixture's /gd/ team's id; its roster places 10, 12, 13 and 19
/// (slots 9, 11, 12 and 18) are the players the scenarios select
/// together.
const GD: u32 = 722;

/// The fixture save open on /gd/'s list, `slot`'s player selected: the
/// selector names the team so its rows are drawn.
fn gd_view(test: &str, slot: u8) -> (SaveView, Headless) {
    let path = fixture(PesVersion::Pes21, test);
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(GD);
    view.selected = Some(Selection::Player(GD, slot));
    (view, Headless::sized(DEFAULT_PANE.x, 1000.0))
}

/// The player row whose label starts `place` ("10 " is roster slot 9).
fn row_at<'a>(nodes: &'a [accesskit::Node], place: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| player_row(node) && node.label().is_some_and(|l| l.starts_with(place)))
        .unwrap_or_else(|| panic!("no player row at place {place:?}"))
}

/// The places of the player rows reporting selected (`toggled`), in
/// draw order (the update's nodes are unordered, so sorted by top).
fn selected_places(nodes: &[accesskit::Node]) -> Vec<String> {
    let mut rows: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| player_row(node) && node.toggled() == Some(accesskit::Toggled::True))
        .collect();
    rows.sort_by(|a, b| {
        let y = |node: &&accesskit::Node| node.bounds().map(|b| b.y0).unwrap_or(0.0);
        y(a).partial_cmp(&y(b)).expect("ordered bounds")
    });
    rows.iter()
        .map(|node| {
            node.label()
                .and_then(|label| label.split_whitespace().next().map(str::to_owned))
                .expect("a place")
        })
        .collect()
}

/// The heading the title row reads: the Label right of the toggle's
/// Gameplan segment (`<id> <name>` for one player, `N players` for a
/// group).
fn heading(nodes: &[accesskit::Node]) -> Option<&str> {
    let segment = nodes
        .iter()
        .find(|node| node.role() == Role::Button && node.label() == Some("Gameplan"))?;
    first_right_of(nodes, segment, |node| node.role() == Role::Label).and_then(|node| node.value())
}

/// The heading `<id> <name>` `slot`'s rostered player gets: his id by
/// the slot rule (`team × 100 + place`) and his display name.
fn own_heading(view: &SaveView, slot: u8) -> String {
    let session = view.session.as_ref().expect("the save opened");
    let id = session.slot_player(GD, slot).expect("a rostered player");
    let name = display_name(&player_entry(view, id).name);
    format!("{id} {name}")
}

// Ctrl+click on a player row adds it to the selection or takes it out:
// the selected one first, then the clicked rows. Taking out the group's
// first row selects the next one; a plain click ends the group.
// SE-GUI-93 (its Select all similar clause is another slice's)
#[test]
fn ctrl_click_builds_the_group_and_a_plain_click_ends_it() {
    let (mut view, mut window) = gd_view("similar_93", 9);
    assert_eq!(
        view.session.as_ref().unwrap().slot_player(GD, 11),
        Some(72212),
        "premise: slot 12 holds player 72212"
    );
    let nodes = draw(&mut window, &mut view);

    // Ctrl+click player 12: rows 10 and 12 report selected, the view is
    // titled `2 players`.
    ctrl_click(&mut window, row_at(&nodes, "12 "), &mut view);
    let nodes = draw(&mut window, &mut view);
    assert_eq!(selected_places(&nodes), ["10", "12"], "the group's rows");
    assert_eq!(heading(&nodes), Some("2 players"));
    assert_eq!(
        view.selected,
        Some(Selection::Player(GD, 9)),
        "the group's first row stays selected"
    );

    // Ctrl+click the group's first row: player 12 alone is selected,
    // his own view shown.
    ctrl_click(&mut window, row_at(&nodes, "10 "), &mut view);
    let nodes = draw(&mut window, &mut view);
    assert_eq!(selected_places(&nodes), ["12"], "12 alone selected");
    assert_eq!(
        view.selected,
        Some(Selection::Player(GD, 11)),
        "the next entry became the selection"
    );
    assert_eq!(heading(&nodes), Some(own_heading(&view, 11).as_str()));

    // Ctrl+click 13 then a plain click on 19: 19 alone is selected.
    ctrl_click(&mut window, row_at(&nodes, "13 "), &mut view);
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        selected_places(&nodes),
        ["12", "13"],
        "premise: the group re-formed"
    );
    click(&mut window, row_at(&nodes, "19 "), &mut view);
    let nodes = draw(&mut window, &mut view);
    assert_eq!(selected_places(&nodes), ["19"], "19 alone selected");
    assert_eq!(
        view.selected,
        Some(Selection::Player(GD, 18)),
        "the plain click selected 19 alone"
    );
    assert_eq!(heading(&nodes), Some(own_heading(&view, 18).as_str()));
}

// The last selected row's Ctrl+click does nothing (one stays
// selected); with a team row selected, or Ctrl+clicked, the click
// selects as a plain click does.
#[test]
fn ctrl_click_on_the_only_selected_row_or_on_a_team_row() {
    let (mut view, mut window) = gd_view("similar_ctrl_edges", 9);
    let nodes = draw(&mut window, &mut view);

    // Ctrl+click the only selected row: nothing changes.
    ctrl_click(&mut window, row_at(&nodes, "10 "), &mut view);
    assert_eq!(view.selected, Some(Selection::Player(GD, 9)), "unchanged");
    let nodes = draw(&mut window, &mut view);
    assert_eq!(selected_places(&nodes), ["10"], "10 alone still");

    // With the team's row selected, a Ctrl+click selects the player
    // alone.
    view.selected = Some(Selection::Team(GD));
    let nodes = draw(&mut window, &mut view);
    ctrl_click(&mut window, row_at(&nodes, "12 "), &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(GD, 11)),
        "12 alone selected"
    );

    // A Ctrl+click on the team row selects the team, as a plain click
    // does.
    let nodes = draw(&mut window, &mut view);
    ctrl_click(&mut window, team_row(&nodes, &view, GD), &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Team(GD)),
        "the team row selected"
    );
}

// A plain click on the group's first row — the row `selected` already
// names — ends the group all the same (a value comparison on `selected`
// sees no move, so the click itself empties it).
#[test]
fn a_plain_click_on_the_groups_first_row_ends_it() {
    let (mut view, mut window) = gd_view("similar_first_row", 9);
    let nodes = draw(&mut window, &mut view);
    ctrl_click(&mut window, row_at(&nodes, "12 "), &mut view);
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        selected_places(&nodes),
        ["10", "12"],
        "premise: the group is on"
    );

    click(&mut window, row_at(&nodes, "10 "), &mut view);
    let nodes = draw(&mut window, &mut view);
    assert_eq!(selected_places(&nodes), ["10"], "10 alone selected");
    assert_eq!(
        view.selected,
        Some(Selection::Player(GD, 9)),
        "10 stays selected"
    );
    assert_eq!(heading(&nodes), Some(own_heading(&view, 9).as_str()));
}

/// The group {10, 12} formed through its Ctrl+click; the frame that
/// shows it, the premise checked.
fn form_group(window: &mut Headless, view: &mut SaveView) -> Vec<accesskit::Node> {
    let nodes = draw(window, view);
    ctrl_click(window, row_at(&nodes, "12 "), view);
    let nodes = draw(window, view);
    assert_eq!(
        selected_places(&nodes),
        ["10", "12"],
        "premise: the group is on"
    );
    nodes
}

// Every other way of selecting ends the group: an arrow key's step, a
// team picked in the selector, the toggle's Gameplan.
#[test]
fn the_group_ends_on_the_other_ways_of_selecting() {
    // Down steps the selection to the next row, alone.
    let (mut view, mut window) = gd_view("similar_end_down", 9);
    form_group(&mut window, &mut view);
    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let nodes = draw(&mut window, &mut view);
    assert_eq!(selected_places(&nodes), ["11"], "Down stepped one row");
    assert_eq!(
        view.selected,
        Some(Selection::Player(GD, 10)),
        "the next row alone"
    );

    // A team picked in the selector selects its row, alone.
    let (mut view, mut window) = gd_view("similar_end_selector", 9);
    form_group(&mut window, &mut view);
    pick_team(&mut window, &mut view, "/f/");
    assert_eq!(
        view.selected,
        Some(Selection::Team(718)),
        "picking /f/ selected its team row"
    );
    let nodes = draw(&mut window, &mut view);
    assert_eq!(selected_places(&nodes), Vec::<String>::new(), "none");

    // The toggle's Gameplan selects the team's row, alone.
    let (mut view, mut window) = gd_view("similar_end_gameplan", 9);
    form_group(&mut window, &mut view);
    press_segment(&mut window, &mut view, "Gameplan");
    assert_eq!(
        view.selected,
        Some(Selection::Team(GD)),
        "Gameplan selected the team's row"
    );
    let nodes = draw(&mut window, &mut view);
    assert_eq!(selected_places(&nodes), Vec::<String>::new(), "none");
}

// An undo keeps the group: the selected players stand through it, so
// the next edit still writes to all of them (SE-GUI-92's undo is a
// later slice's; the selection's part is this one's).
#[test]
fn an_undo_keeps_the_group() {
    let (mut view, mut window) = gd_view("similar_undo", 9);
    form_group(&mut window, &mut view);
    let before = player_entry(&view, 72210).basic.age;
    {
        let session = view.session.as_mut().expect("the open save");
        edit_player(session, 72210, 0, |player| {
            player.basic.age = player.basic.age.wrapping_add(1);
        });
    }
    assert!(
        view.session.as_ref().unwrap().can_undo(),
        "premise: the edit pushed an entry"
    );

    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        player_entry(&view, 72210).basic.age,
        before,
        "the undo put the age back"
    );
    let nodes = draw(&mut window, &mut view);
    assert_eq!(selected_places(&nodes), ["10", "12"], "the group still on");
    assert_eq!(heading(&nodes), Some("2 players"));
}

// A group is on only while its first entry is the selected row: a
// selection moved off it any other way — a write to `selected` that
// knew nothing of the group — ends it for good, and moving the
// selection back does not bring it back (`show` empties a stale group
// once a frame, before the list draws).
#[test]
fn a_stale_group_does_not_come_back() {
    let (mut view, mut window) = gd_view("similar_stale", 9);
    form_group(&mut window, &mut view);

    view.selected = Some(Selection::Player(GD, 18));
    let nodes = draw(&mut window, &mut view);
    assert_eq!(selected_places(&nodes), ["19"], "19 alone selected");
    assert_eq!(heading(&nodes), Some(own_heading(&view, 18).as_str()));

    view.selected = Some(Selection::Player(GD, 9));
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        selected_places(&nodes),
        ["10"],
        "10 alone: the old group did not come back"
    );
    assert_eq!(heading(&nodes), Some(own_heading(&view, 9).as_str()));
}

// The on rule itself: two or more entries, the first of them the
// selected row — a group holding entries whose first is not selected is
// not on.
#[test]
fn group_on_needs_two_or_more_with_the_first_selected() {
    let group = vec![(GD, 9), (GD, 11)];
    assert!(
        group_on(Some(Selection::Player(GD, 9)), &group),
        "the first entry selected is on"
    );
    assert!(
        !group_on(Some(Selection::Player(GD, 11)), &group),
        "a later entry selected is not"
    );
    assert!(
        !group_on(Some(Selection::Team(GD)), &group),
        "a team row selected is not"
    );
    assert!(
        !group_on(Some(Selection::Player(GD, 9)), &group[..1]),
        "one entry is not"
    );
}

// While the group is on the toggle shows Stats as the screen —
// whichever screen stood before — with Appearance disabled, its reason
// on hover; the screen that stood before comes back when the group ends.
#[test]
fn the_toggle_shows_stats_with_appearance_disabled_while_the_group_is_on() {
    let (mut view, mut window) = gd_view("similar_toggle", 9);
    open_appearance(&mut window, &mut view);
    let nodes = draw(&mut window, &mut view);
    assert!(appearance_shown(&nodes), "premise: the Appearance screen");
    assert!(chosen(&nodes, "Appearance"), "its segment held");

    ctrl_click(&mut window, row_at(&nodes, "12 "), &mut view);
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        selected_places(&nodes),
        ["10", "12"],
        "premise: the group is on"
    );
    assert!(chosen(&nodes, "Stats"), "Stats is the shown screen");
    assert!(
        !appearance_shown(&nodes),
        "the Stats screen, not Appearance's"
    );
    let appearance = segment(&nodes, "Appearance");
    assert!(
        appearance.is_disabled(),
        "Appearance disabled while the group is on"
    );
    assert_eq!(
        tooltip_containing(
            &mut window,
            &mut view,
            centre(appearance),
            "Appearance is one player's"
        )
        .as_deref(),
        Some("Appearance is one player's: select one row"),
        "its tooltip"
    );

    // A plain click ends the group: Appearance shows again.
    let nodes = draw(&mut window, &mut view);
    click(&mut window, row_at(&nodes, "19 "), &mut view);
    let nodes = draw(&mut window, &mut view);
    assert!(appearance_shown(&nodes), "Appearance shown again");
    assert!(chosen(&nodes, "Appearance"), "its segment held again");
}

/// The Select all similar players button's node (the quick actions'
/// head, under the list).
fn similar_button(nodes: &[accesskit::Node]) -> &accesskit::Node {
    node_labeled(nodes, Role::Button, "Select all similar players")
}

/// The window's nodes: `dialog_nodes` for its title, `pairs` a
/// `frame_ids` frame's.
fn similar_nodes(pairs: &[(accesskit::NodeId, accesskit::Node)]) -> Vec<&accesskit::Node> {
    prompt::dialog_nodes(pairs, "Select all similar players")
}

/// The window's CheckBox whose label starts with `label` (`Position`,
/// `Cards`, `Stats`).
fn similar_checkbox<'a>(
    pairs: &'a [(accesskit::NodeId, accesskit::Node)],
    label: &str,
) -> &'a accesskit::Node {
    similar_nodes(pairs)
        .into_iter()
        .find(|node| {
            node.role() == Role::CheckBox
                && node.label().is_some_and(|text| text.starts_with(label))
        })
        .unwrap_or_else(|| panic!("no box {label:?} in the window"))
}

/// The window's RadioButton labeled `label` (the team's name, `Every
/// team`).
fn similar_radio<'a>(
    pairs: &'a [(accesskit::NodeId, accesskit::Node)],
    label: &str,
) -> &'a accesskit::Node {
    similar_nodes(pairs)
        .into_iter()
        .find(|node| node.role() == Role::RadioButton && node.label() == Some(label))
        .unwrap_or_else(|| panic!("no radio {label:?} in the window"))
}

/// The window's action button: `Select N players`, or the disabled `No
/// other player matches` when he alone matches.
fn select_button(pairs: &[(accesskit::NodeId, accesskit::Node)]) -> &accesskit::Node {
    similar_nodes(pairs)
        .into_iter()
        .find(|node| {
            node.role() == Role::Button
                && node.label().is_some_and(|label| {
                    label.starts_with("Select ") || label == "No other player matches"
                })
        })
        .expect("the window's Select button")
}

/// One `frame_ids` frame's pairs (the window's nodes are looked up in
/// them).
fn draw_pairs(
    window: &mut Headless,
    view: &mut SaveView,
) -> Vec<(accesskit::NodeId, accesskit::Node)> {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    pairs
}

/// The button clicked: the window open on the selected player, its
/// frame's pairs returned.
fn open_similar(
    window: &mut Headless,
    view: &mut SaveView,
) -> Vec<(accesskit::NodeId, accesskit::Node)> {
    let nodes = draw(window, view);
    click(window, similar_button(&nodes), view);
    assert!(view.similar.is_some(), "the window opened");
    draw_pairs(window, view)
}

/// `team`'s rostered rows as `(team id, slot)`, in roster order: the
/// slots holding a player id other than 0.
fn rostered(team: &TeamEntry) -> Vec<(u32, u8)> {
    team.roster
        .iter()
        .enumerate()
        .filter(|(_, roster)| roster.player_id != 0)
        .map(|(slot, _)| (team.id, u8::try_from(slot).unwrap()))
        .collect()
}

/// The cup rows matching `player` on the named boxes, as a test's own
/// loop counts them: `position` the registered position, `cards` the
/// skills and COM styles both, `stats` the ability stats. Every cup
/// team's roster (a player on two teams is two rows).
fn own_matches(
    file: &EditFile,
    player: &PlayerEntry,
    position: bool,
    cards: bool,
    stats: bool,
) -> Vec<(u32, u8)> {
    let players: HashMap<u32, &PlayerEntry> = file
        .players()
        .iter()
        .map(|entry| (entry.id, entry))
        .collect();
    crate::session::cup::cup_teams(file)
        .flat_map(|team| {
            rostered(team).into_iter().filter_map(|(team_id, slot)| {
                players
                    .get(&team.roster[usize::from(slot)].player_id)
                    .map(|entry| (team_id, slot, entry))
            })
        })
        .filter(|(_, _, entry)| {
            (!position || entry.positions.registered == player.positions.registered)
                && (!cards
                    || entry.skills.skills == player.skills.skills
                        && entry.skills.com_styles == player.skills.com_styles)
                && (!stats || entry.stats.abilities() == player.stats.abilities())
        })
        .map(|(team_id, slot, _)| (team_id, slot))
        .collect()
}

// The window opens on the selected player, its boxes ticked
// and his team chosen, the button reading `Select N players` (`No other
// player matches`, disabled, when he alone matches); each box or radio
// change recounts the match. Select makes the match the group, him
// first, and the view is titled `N players`.
// SE-GUI-91
#[test]
fn the_window_counts_the_matches_as_the_boxes_and_scope_change() {
    let path = fixture(PesVersion::Pes21, "similar_91");
    // The fixture's players are clones (all CB, all stats 77, no cards):
    // the scenario's premise is written into the save — 10 and 19 alike
    // on the three boxes as LBs, 12 and 17 on the stats and cards as
    // RBs, 13 on the stats alone, and three of 835's on the stats, so
    // Every team's count reaches a team outside the 4cc category.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let position =
        |name: &str| u8::try_from(POSITIONS.iter().position(|&p| p == name).unwrap()).unwrap();
    for (id, byte) in [(72210, "LB"), (72219, "LB"), (72212, "RB"), (72217, "RB")] {
        file.player_mut(id).unwrap().positions.registered = position(byte);
    }
    for id in [72210, 72212, 72217, 72219] {
        let player = file.player_mut(id).unwrap();
        player.skills.skills[0] = true;
        player.skills.com_styles[0] = true;
    }
    for id in [72210, 72212, 72213, 72217, 72219] {
        file.player_mut(id).unwrap().stats.speed = 82;
    }
    for slot in 0..3 {
        let id = file.team(835).unwrap().roster[slot].player_id;
        file.player_mut(id).unwrap().stats.speed = 82;
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(GD);
    view.selected = Some(Selection::Player(GD, 9));
    let mut window = Headless::sized(DEFAULT_PANE.x, 1000.0);
    // The premises from the save itself: 12 and 17 share 10's ability
    // stats and cards but not his registered position, 13 his ability
    // stats alone, 19 all three.
    let p10 = player_entry(&view, 72210);
    for id in [72212, 72217] {
        let player = player_entry(&view, id);
        assert_eq!(
            player.stats.abilities(),
            p10.stats.abilities(),
            "premise: {id}'s stats are 10's"
        );
        assert_eq!(
            (player.skills.skills, player.skills.com_styles),
            (p10.skills.skills, p10.skills.com_styles),
            "premise: {id}'s cards are 10's"
        );
        assert_ne!(
            player.positions.registered, p10.positions.registered,
            "premise: {id}'s position is not 10's"
        );
    }
    let p13 = player_entry(&view, 72213);
    assert_eq!(
        p13.stats.abilities(),
        p10.stats.abilities(),
        "premise: 13's stats are 10's"
    );
    assert_ne!(
        p13.positions.registered, p10.positions.registered,
        "premise: 13's position is not 10's"
    );
    assert!(
        (p13.skills.skills, p13.skills.com_styles) != (p10.skills.skills, p10.skills.com_styles),
        "premise: 13's cards are not 10's"
    );
    let p19 = player_entry(&view, 72219);
    assert_eq!(
        (
            p19.stats.abilities(),
            p19.skills.skills,
            p19.skills.com_styles,
            p19.positions.registered,
        ),
        (
            p10.stats.abilities(),
            p10.skills.skills,
            p10.skills.com_styles,
            p10.positions.registered,
        ),
        "premise: 19 shares all three"
    );
    let others = own_matches(
        view.session.as_ref().unwrap().file(),
        &p10,
        false,
        false,
        true,
    )
    .into_iter()
    .filter(|&(team_id, _)| team_id != GD)
    .count();
    assert!(others > 0, "premise: other teams' stat-equals exist");

    // The window opens with the three boxes ticked and /gd/ chosen, its
    // button `Select 2 players`.
    let pairs = open_similar(&mut window, &mut view);
    let name = display_name(&p10.name);
    assert!(
        similar_nodes(&pairs).iter().any(|node| {
            node.role() == Role::Label && node.value() == Some(format!("Match {name} on:").as_str())
        }),
        "the `Match <name> on:` line"
    );
    for label in ["Position", "Cards", "Stats"] {
        assert_eq!(
            similar_checkbox(&pairs, label).toggled(),
            Some(accesskit::Toggled::True),
            "{label} ticked at open"
        );
    }
    assert_eq!(
        similar_radio(&pairs, "/gd/").toggled(),
        Some(accesskit::Toggled::True),
        "/gd/ chosen at open"
    );
    assert_ne!(
        similar_radio(&pairs, "Every team").toggled(),
        Some(accesskit::Toggled::True),
        "Every team off at open"
    );
    assert_eq!(
        select_button(&pairs).label(),
        Some("Select 2 players"),
        "10 and 19 match on the three"
    );

    // Position unticked: 12 and 17 join, `Select 4 players`; Cards
    // unticked too: 13 joins, `Select 5 players`.
    click(&mut window, similar_checkbox(&pairs, "Position"), &mut view);
    let pairs = draw_pairs(&mut window, &mut view);
    assert_eq!(select_button(&pairs).label(), Some("Select 4 players"));
    click(&mut window, similar_checkbox(&pairs, "Cards"), &mut view);
    let pairs = draw_pairs(&mut window, &mut view);
    assert_eq!(select_button(&pairs).label(), Some("Select 5 players"));

    // Every team: the count adds every other team's stat-equal.
    click(&mut window, similar_radio(&pairs, "Every team"), &mut view);
    let pairs = draw_pairs(&mut window, &mut view);
    assert_eq!(
        select_button(&pairs).label(),
        Some(format!("Select {} players", 5 + others).as_str()),
        "Every team adds the other teams' stat-equals"
    );

    // The three ticked again on /gd/: `Select 2 players`, and Select
    // makes 10 and 19 the group.
    click(&mut window, similar_checkbox(&pairs, "Position"), &mut view);
    let pairs = draw_pairs(&mut window, &mut view);
    click(&mut window, similar_checkbox(&pairs, "Cards"), &mut view);
    let pairs = draw_pairs(&mut window, &mut view);
    click(&mut window, similar_radio(&pairs, "/gd/"), &mut view);
    let pairs = draw_pairs(&mut window, &mut view);
    assert_eq!(select_button(&pairs).label(), Some("Select 2 players"));
    click(&mut window, select_button(&pairs), &mut view);
    assert!(view.similar.is_none(), "Select closed the window");
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        selected_places(&nodes),
        ["10", "19"],
        "rows 10 and 19 filled"
    );
    assert_eq!(heading(&nodes), Some("2 players"));
}

// The Select under `Every team` holds every matching cup roster; the
// selector moves to `All teams`, the typed filter is cleared and a
// category that leaves one of the teams out widens to `All`, so every
// selected row is in the list.
#[test]
fn select_under_every_team_moves_the_selector_and_lists_every_row() {
    let (mut view, mut window) = gd_view("similar_every_select", 9);
    // Premise: with the category on 4cc, some match is on a team it
    // leaves out (the fixture's clones reach the VGL teams).
    let p10 = player_entry(&view, 72210);
    let expected = own_matches(
        view.session.as_ref().unwrap().file(),
        &p10,
        true,
        true,
        true,
    );
    let outside = expected
        .iter()
        .filter(|&&(team_id, _)| !TeamCategory::FourCc.contains(team_id))
        .count();
    assert!(
        outside > 0,
        "premise: a match on a team the category leaves out"
    );
    view.team_category = TeamCategory::FourCc;
    view.filter = "no such".to_owned();
    let pairs = open_similar(&mut window, &mut view);
    click(&mut window, similar_radio(&pairs, "Every team"), &mut view);
    let pairs = draw_pairs(&mut window, &mut view);
    assert_eq!(
        select_button(&pairs).label(),
        Some(format!("Select {} players", expected.len()).as_str()),
        "the count is every cup all-three-equal"
    );
    click(&mut window, select_button(&pairs), &mut view);
    assert_eq!(view.team, TeamChoice::All, "the selector on All teams");
    assert!(view.filter.is_empty(), "the filter cleared");
    assert_eq!(
        view.team_category,
        TeamCategory::All,
        "the category widened to All"
    );
    assert_eq!(view.selected, Some(Selection::Player(GD, 9)));
    assert!(
        view.reveal_selected.is_none(),
        "the Select asks no scroll of the list"
    );
    assert_eq!(
        view.group.first(),
        Some(&(GD, 9)),
        "the group's first entry is the selected row"
    );
    assert_eq!(
        view.group.len(),
        expected.len(),
        "every matching row in the group"
    );
    let listed: std::collections::HashSet<(u32, u8)> = selection::rows(
        view.session.as_ref().unwrap().file(),
        &view.team,
        &view.filter,
        view.team_category,
    )
    .into_iter()
    .map(|row| (row.team_id, row.slot))
    .collect();
    for &row in &view.group {
        assert!(listed.contains(&row), "row {row:?} in the list");
    }
}

// The Select under `Every team` keeps a category that already lists
// every matching row.
#[test]
fn select_under_every_team_keeps_a_category_holding_every_match() {
    let (mut view, mut window) = gd_view("similar_every_keeps_category", 9);
    // 10 and 19 made LBs with 82 speed: on the three boxes they match
    // each other and no player outside /gd/.
    let session = view.session.as_mut().expect("the open save");
    let lb = u8::try_from(POSITIONS.iter().position(|&p| p == "LB").unwrap()).unwrap();
    for (key, id) in [(0, 72210), (1, 72219)] {
        edit_player(session, id, key, |player| {
            player.positions.registered = lb;
            player.stats.speed = 82;
        });
    }
    let p10 = player_entry(&view, 72210);
    let expected = own_matches(
        view.session.as_ref().unwrap().file(),
        &p10,
        true,
        true,
        true,
    );
    assert_eq!(expected, [(GD, 9), (GD, 18)], "premise: 10 and 19 alone");
    assert!(
        expected
            .iter()
            .all(|&(team_id, _)| TeamCategory::FourCc.contains(team_id)),
        "premise: the 4cc category holds every match"
    );
    view.team_category = TeamCategory::FourCc;
    let pairs = open_similar(&mut window, &mut view);
    click(&mut window, similar_radio(&pairs, "Every team"), &mut view);
    let pairs = draw_pairs(&mut window, &mut view);
    assert_eq!(select_button(&pairs).label(), Some("Select 2 players"));
    click(&mut window, select_button(&pairs), &mut view);
    assert_eq!(view.group, [(GD, 9), (GD, 18)], "the Select made the group");
    assert_eq!(
        view.team_category,
        TeamCategory::FourCc,
        "the category stays 4cc"
    );
}

// `No other player matches`, disabled, when the player alone matches on
// the three boxes (the fixture's 78119 has no all-three match in the
// cup).
#[test]
fn the_button_reads_no_other_player_matches_when_he_alone_matches() {
    let path = fixture(PesVersion::Pes21, "similar_no_match");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(781);
    view.selected = Some(Selection::Player(781, 18));
    let mut window = Headless::sized(DEFAULT_PANE.x, 1000.0);
    assert_eq!(
        view.session.as_ref().unwrap().slot_player(781, 18),
        Some(78119),
        "premise: slot 18 holds 78119"
    );
    let player = player_entry(&view, 78119);
    assert!(
        own_matches(
            view.session.as_ref().unwrap().file(),
            &player,
            true,
            true,
            true
        ) == [(781, 18)],
        "premise: no all-three match in the cup"
    );
    let pairs = open_similar(&mut window, &mut view);
    let button = select_button(&pairs);
    assert_eq!(button.label(), Some("No other player matches"));
    assert!(
        button.is_disabled(),
        "Select disabled when he alone matches"
    );
}

// SE-GUI-93's clause: with several rows selected the button is
// disabled, its tooltip `Select one player first`; on a team row the
// same, on a lone player row enabled with the other tooltip.
#[test]
fn the_button_is_disabled_under_a_group_or_a_team_row() {
    let (mut view, mut window) = gd_view("similar_93_button", 11);
    let nodes = draw(&mut window, &mut view);
    ctrl_click(&mut window, row_at(&nodes, "13 "), &mut view);
    let nodes = draw(&mut window, &mut view);
    assert_eq!(
        selected_places(&nodes),
        ["12", "13"],
        "premise: the group is on"
    );
    let button = similar_button(&nodes);
    assert!(button.is_disabled(), "disabled under a group");
    assert_eq!(
        tooltip_containing(
            &mut window,
            &mut view,
            centre(button),
            "Select one player first"
        )
        .as_deref(),
        Some("Select one player first"),
        "its tooltip"
    );

    // On a team row: the same.
    let nodes = draw(&mut window, &mut view);
    click(&mut window, team_row(&nodes, &view, GD), &mut view);
    let nodes = draw(&mut window, &mut view);
    let button = similar_button(&nodes);
    assert!(button.is_disabled(), "disabled on a team row");
    assert_eq!(
        tooltip_containing(
            &mut window,
            &mut view,
            centre(button),
            "Select one player first"
        )
        .as_deref(),
        Some("Select one player first"),
        "its tooltip on a team row"
    );

    // On a lone player row: enabled, the other tooltip.
    click(&mut window, row_at(&nodes, "10 "), &mut view);
    let nodes = draw(&mut window, &mut view);
    let button = similar_button(&nodes);
    assert!(!button.is_disabled(), "enabled on a lone player row");
    assert_eq!(
        tooltip_containing(
            &mut window,
            &mut view,
            centre(button),
            "Select the players like this one"
        )
        .as_deref(),
        Some("Select the players like this one, to edit their stats together"),
        "its tooltip"
    );
}

// Cancel and Escape close the window, the selection and the group
// unchanged.
#[test]
fn cancel_and_escape_close_the_window_changing_nothing() {
    let (mut view, mut window) = gd_view("similar_cancel_escape", 9);
    let pairs = open_similar(&mut window, &mut view);
    click(
        &mut window,
        prompt::modal_button(&pairs, "Select all similar players", "Cancel"),
        &mut view,
    );
    assert!(view.similar.is_none(), "Cancel closed it");
    assert_eq!(view.selected, Some(Selection::Player(GD, 9)), "unchanged");
    assert!(view.group.is_empty(), "no group formed");

    open_similar(&mut window, &mut view);
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert!(view.similar.is_none(), "Escape closed it");
    assert_eq!(view.selected, Some(Selection::Player(GD, 9)), "unchanged");
    assert!(view.group.is_empty(), "no group formed");
}

// `matching` with no box ticked: every rostered row of the scope, the
// anchor first then the rest in list order — the team's roster order,
// or every cup team's in record order.
#[test]
fn matching_with_no_box_ticked_matches_the_whole_scope() {
    let file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let none = similar::Boxes {
        position: false,
        cards: false,
        stats: false,
    };
    let gd: Vec<(u32, u8)> = rostered(file.team(GD).unwrap());
    assert!(gd.contains(&(GD, 9)), "premise: the anchor rostered");
    let expected: Vec<(u32, u8)> = std::iter::once((GD, 9))
        .chain(gd.into_iter().filter(|&row| row != (GD, 9)))
        .collect();
    assert_eq!(
        similar::matching(&file, (GD, 9), none, similar::Scope::Team),
        expected,
        "the whole roster, the anchor first"
    );
    let expected: Vec<(u32, u8)> = std::iter::once((GD, 9))
        .chain(
            crate::session::cup::cup_teams(&file)
                .flat_map(rostered)
                .filter(|&row| row != (GD, 9)),
        )
        .collect();
    assert_eq!(
        similar::matching(&file, (GD, 9), none, similar::Scope::All),
        expected,
        "every cup roster, the anchor first"
    );
}

// `matching` under a ticked box: the anchor first, and Every team adds
// the other cup teams' matching rows in record order.
#[test]
fn matching_the_anchor_first_and_every_team_adds_other_teams() {
    let file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let stats = similar::Boxes {
        position: false,
        cards: false,
        stats: true,
    };
    let team = similar::matching(&file, (GD, 9), stats, similar::Scope::Team);
    let every = similar::matching(&file, (GD, 9), stats, similar::Scope::All);
    assert_eq!(team.first(), Some(&(GD, 9)), "the anchor first");
    assert_eq!(every.first(), Some(&(GD, 9)), "the anchor first");
    // The fixture's /gd/ are clones: the whole roster shares 10's stats,
    // him first then roster order; Every team adds the other cup teams'
    // stat-equals.
    let gd: Vec<(u32, u8)> = rostered(file.team(GD).unwrap())
        .into_iter()
        .filter(|&row| row != (GD, 9))
        .collect();
    assert_eq!(
        team,
        std::iter::once((GD, 9)).chain(gd).collect::<Vec<_>>(),
        "the team's stat-equals, roster order"
    );
    let p10 = file.player(72210).unwrap();
    let expected = own_matches(&file, p10, false, false, true).len();
    assert_eq!(every.len(), expected, "every stat-equal row in the cup");
    assert!(
        every.iter().any(|&(team_id, _)| team_id != GD),
        "other teams' rows added"
    );
}

// `matching` under Cards: a player matches only when his skills and his
// COM styles are both the anchor's.
#[test]
fn cards_match_on_the_skills_and_the_com_styles_both() {
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let roster = &file.team(GD).unwrap().roster;
    for (slot, id) in [(9, 72210), (11, 72212), (12, 72213), (18, 72219)] {
        assert_eq!(
            roster[slot].player_id, id,
            "premise: slot {slot} holds {id}"
        );
    }
    // The fixture's /gd/ are clones with no cards: 12 gets a COM style
    // 10 lacks, 13 a skill, and 19 is left equal on both.
    file.player_mut(72212).unwrap().skills.com_styles[0] = true;
    file.player_mut(72213).unwrap().skills.skills[0] = true;
    let cards = |id| {
        let skills = &file.player(id).unwrap().skills;
        (skills.skills, skills.com_styles)
    };
    let (skills, com_styles) = cards(72210);
    assert_eq!(cards(72212).0, skills, "premise: 12's skills are 10's");
    assert_ne!(
        cards(72212).1,
        com_styles,
        "premise: 12's COM styles differ"
    );
    assert_eq!(
        cards(72213).1,
        com_styles,
        "premise: 13's COM styles are 10's"
    );
    assert_ne!(cards(72213).0, skills, "premise: 13's skills differ");
    assert_eq!(
        cards(72219),
        (skills, com_styles),
        "premise: 19 equal on both"
    );
    let boxes = similar::Boxes {
        position: false,
        cards: true,
        stats: false,
    };
    let matched = similar::matching(&file, (GD, 9), boxes, similar::Scope::Team);
    assert!(matched.contains(&(GD, 18)), "19 matches");
    assert!(!matched.contains(&(GD, 11)), "12's COM styles differ");
    assert!(!matched.contains(&(GD, 12)), "13's skills differ");
}
