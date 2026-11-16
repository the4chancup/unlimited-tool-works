use super::chrome::{pick_in_selector, radio};
use super::*;

// A pick of All teams keeps the selection, the list scrolled to the
// kept row: team 820 picked, then All teams — every team's rows stand
// before 820's again, and its row is inside the clip on the next frame.
#[test]
fn picking_all_teams_scrolls_the_list_to_the_kept_row() {
    let path = fixture(PesVersion::Pes21, "all_teams_pick_reveal");
    let mut view = SaveView::new();
    view.open(&path);
    let name = display_name(&team_entry(&view, 820).name);
    assert!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .teams()
            .iter()
            .filter(|team| display_name(&team.name) == name)
            .count()
            == 1,
        "premise: 820's name {name:?} is the only one in the selector"
    );
    let mut window = Headless::new();

    pick_in_selector(&mut window, &mut view, &name);
    assert_eq!(
        view.selected,
        Some(Selection::Team(820)),
        "premise: 820's row selected"
    );

    pick_in_selector(&mut window, &mut view, "All teams");
    assert_eq!(view.team, TeamChoice::All, "premise: All teams picked");
    assert_eq!(
        view.selected,
        Some(Selection::Team(820)),
        "the pick keeps the selection"
    );
    assert_selected_row_in_clip(&mut window, &mut view);
}

// A category tick that keeps the selection scrolls the list to its row:
// `VGL` ticked with 781's first player selected (the list's first row),
// then `All` — the 4cc teams' rows stand before his again, and his row
// is inside the clip on the next frame.
// SE-GUI-83
#[test]
fn a_category_tick_scrolls_the_list_to_the_kept_selection() {
    let path = fixture(PesVersion::Pes21, "category_kept_reveal");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();

    // `VGL` ticked: the selection moves to the list's first player.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, selector_combo(&nodes), &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, radio(&nodes, "VGL"), &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(781, 0)),
        "premise: the selection moved to the first VGL player"
    );

    // `All` ticked: the selection stays on him, and every team's rows
    // stand before his again.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, radio(&nodes, "All"), &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(781, 0)),
        "the tick keeps the selection"
    );
    assert_selected_row_in_clip(&mut window, &mut view);
}

// An undo or a redo of a roster move puts the moved player back on the
// slot the step gives him, the selection on him and the list scrolled to
// his row: undone from a list scrolled to the slot the drag left him on,
// his row stands inside the clip on the next frame.
// SE-GUI-08
#[test]
fn an_undo_or_redo_scrolls_the_list_to_the_moved_player() {
    let path = fixture(PesVersion::Pes21, "undo_move_reveal");
    // Player 01 renamed: the fixture's records are identical, and a move
    // that writes nothing pushes no undo entry.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().name = "FIRST".to_owned();
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    // What a drag of player 01 to the last filled slot ends in:
    // `move_player`'s edit and the selection on the dropped record's slot.
    let last = team_entry(&view, 701)
        .roster
        .iter()
        .rposition(|slot| slot.player_id != 0)
        .expect("a rostered slot");
    let last = u8::try_from(last).expect("a roster is at most 40 slots");
    assert!(last > 10, "premise: a list longer than the pane shows");
    view.session.as_mut().unwrap().move_player(701, 0, last);
    view.selected = Some(Selection::Player(701, last));
    // A pane shorter than the team's 24-row list, the list scrolled to
    // its end, where the moved player's row stands.
    let mut window = Headless::sized(DEFAULT_PANE.x, 400.0);
    scroll_player_list_to_bottom(&mut window, &mut view);

    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "premise: the undo put the selection back on the moved player's slot"
    );
    assert_selected_row_in_clip(&mut window, &mut view);

    window.frame(vec![key(egui::Key::Y, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, last)),
        "premise: the redo moved it back to the last slot"
    );
    assert_selected_row_in_clip(&mut window, &mut view);
}

// A category tick that leaves the selected player's team out moves the
// selection to the list's first player, the list scrolling to his row:
// All teams scrolled to its end, `VGL` ticked, 78101's row inside the
// clip on the next frame.
// SE-GUI-83
#[test]
fn a_category_tick_scrolls_the_list_to_the_moved_selection() {
    let path = fixture(PesVersion::Pes21, "category_reveal");
    let mut view = SaveView::new();
    view.open(&path);
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "premise: the first player row selected"
    );
    let mut window = Headless::new();
    scroll_player_list_to_bottom(&mut window, &mut view);

    // The selector open, `VGL` ticked: 701 is a 4cc team, out of the
    // category, so the selection moves to the first VGL player.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, selector_combo(&nodes), &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let vgl = nodes
        .iter()
        .find(|node| node.role() == Role::RadioButton && node.label() == Some("VGL"))
        .expect("the VGL radio button");
    click(&mut window, vgl, &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(781, 0)),
        "the selection moved to the first VGL player"
    );
    assert_selected_row_in_clip(&mut window, &mut view);
}

// A team picked in the selector selects its row, the list scrolling to
// it: All teams scrolled to its end, `/a/` picked, its row at the clip's
// top on the next frame.
#[test]
fn a_team_pick_scrolls_the_list_to_its_row() {
    let path = fixture(PesVersion::Pes21, "team_pick_reveal");
    let mut view = SaveView::new();
    view.open(&path);
    // A pane shorter than a team's 24-row list, so the pick's kept offset
    // would clamp to its end and leave the team row off screen.
    let mut window = Headless::sized(DEFAULT_PANE.x, 400.0);
    scroll_player_list_to_bottom(&mut window, &mut view);

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, selector_combo(&nodes), &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (item, _) = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("/a/"))
        .expect("team 702's entry");
    action_click(&mut window, &mut view, *item);
    assert_eq!(
        view.selected,
        Some(Selection::Team(702)),
        "the pick selected /a/'s team row"
    );
    let (bounds, clip) = assert_selected_row_in_clip(&mut window, &mut view);
    assert!(
        (bounds.y0 - f64::from(clip.top())).abs() <= 1.0,
        "the team row at the clip's top ({bounds:?} vs {clip:?})"
    );
}

// Another save opened starts the list at its top, not at the last save's
// scroll offset: scrolled to the end, a save opened, its first player
// row stands inside the clip on the next frame.
#[test]
fn another_save_opened_starts_the_list_at_its_top() {
    let path = fixture(PesVersion::Pes21, "reopen_scroll");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    scroll_player_list_to_bottom(&mut window, &mut view);

    open_again(&mut view, &path);
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "premise: the new save's first player row selected"
    );
    assert_selected_row_in_clip(&mut window, &mut view);
}
