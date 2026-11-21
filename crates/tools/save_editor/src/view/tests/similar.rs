use super::*;

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
