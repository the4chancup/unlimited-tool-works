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
    let (bounds, clip) = assert_selected_row_in_clip(&mut window, &mut view);
    assert!(
        (bounds.y0 - f64::from(clip.top())).abs() <= 1.0,
        "820's row at the clip's top, `Top` ({bounds:?} vs {clip:?})"
    );
}

// A category tick that keeps the selection scrolls the list to its row:
// `VGL` ticked with 781's first player selected (the list's first row),
// then `All` — the 4cc teams' rows stand before his again, and his row
// is inside the clip on the next frame.
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
    // The 4cc teams' rows over him outnumber the rows the clip shows, so
    // `Top` would stand his row at the clip's top edge while `Nearest`
    // leaves it at the bottom edge.
    let index = selection::list(
        view.session.as_ref().unwrap().file(),
        &view.team,
        &view.filter,
        view.team_category,
    )
    .iter()
    .position(|row| {
        matches!(row, selection::ListRow::Player(player) if player.team_id == 781 && player.slot == 0)
    })
    .expect("78101's row is listed");
    let (bounds, clip) = assert_selected_row_in_clip(&mut window, &mut view);
    let shown = (f64::from(clip.height()) / (bounds.y1 - bounds.y0)) as usize;
    assert!(index > shown, "premise: {index} rows over his row");
    assert!(
        (bounds.y1 - f64::from(clip.bottom())).abs() <= 1.0,
        "his row at the clip's bottom edge, `Nearest` ({bounds:?} vs {clip:?})"
    );
}

// A category tick that leaves the listed rows as they were moves
// nothing: 781 picked, its first player scrolled out of the clip, `VGL`
// ticked — 781 is a VGL team, so the list holds the same rows — and the
// top row stays where it was.
#[test]
fn a_category_tick_that_relists_nothing_keeps_the_scroll() {
    let path = fixture(PesVersion::Pes21, "category_same_rows");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 400.0);
    pick_in_selector(&mut window, &mut view, "/@/");
    assert_eq!(
        view.selected,
        Some(Selection::Team(781)),
        "premise: 781's row selected"
    );
    assert!(
        TeamCategory::Vgl.contains(781),
        "premise: 781 is a VGL team"
    );
    view.selected = Some(Selection::Player(781, 0));

    // Wheel down until his row is out of the clip.
    wheel_row_out_of_clip(&mut window, &mut view, |node| {
        node.toggled() == Some(accesskit::Toggled::True)
    });
    let nodes = draw(&mut window, &mut view);
    let top_before = top_list_row(&nodes);

    click(&mut window, selector_combo(&nodes), &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, radio(&nodes, "VGL"), &mut view);
    assert_eq!(view.team_category, TeamCategory::Vgl, "premise: VGL ticked");
    assert_eq!(
        view.selected,
        Some(Selection::Player(781, 0)),
        "premise: the selection kept"
    );
    let nodes = draw(&mut window, &mut view);
    assert_eq!(top_list_row(&nodes), top_before, "the tick moved no row");
}

// A team picked in the selector under a typed filter selects its row,
// the list scrolled to it though the listed rows stay as they were (a
// filter already lists every team it names): `82` typed, the list
// scrolled, the filter's last team picked — its row inside the clip on
// the next frame.
#[test]
fn a_team_pick_under_a_filter_scrolls_the_list_to_its_row() {
    let path = fixture(PesVersion::Pes21, "team_pick_filter_reveal");
    let mut view = SaveView::new();
    view.open(&path);
    view.filter = "82".to_owned();
    // The last team the filter lists: its row stands rows deep, off the
    // clip's end.
    let (teams, name) = {
        let file = view.session.as_ref().unwrap().file();
        let teams: Vec<u32> = selection::list(file, &view.team, &view.filter, view.team_category)
            .iter()
            .filter_map(|row| match row {
                selection::ListRow::Team(id) => Some(*id),
                _ => None,
            })
            .collect();
        let last = *teams.last().expect("a team listed under the filter");
        let name = display_name(&file.team(last).unwrap().name);
        assert!(
            file.teams()
                .iter()
                .filter(|team| display_name(&team.name) == name)
                .count()
                == 1,
            "premise: {name:?} is the only one in the selector"
        );
        (teams, name)
    };
    assert!(
        teams.len() > 3,
        "premise: the filter lists several teams: {teams:?}"
    );
    let mut window = Headless::new();

    // The first listed row out of the clip, then the last team picked.
    let nodes = draw(&mut window, &mut view);
    let top_label = top_list_row(&nodes).0;
    wheel_row_out_of_clip(&mut window, &mut view, |node| {
        node.label() == Some(top_label.as_str())
    });
    pick_in_selector(&mut window, &mut view, &name);
    assert_eq!(
        view.selected,
        Some(Selection::Team(*teams.last().unwrap())),
        "the pick selected its row"
    );
    assert_selected_row_in_clip(&mut window, &mut view);
}

// A category tick under a typed filter that leaves the selection's team
// out moves it to the list's first player row, the list scrolled to
// him: `79` typed (several VGL teams), a 4cc player selected and the
// list scrolled, `VGL` ticked — his row inside the clip on the next
// frame.
#[test]
fn a_category_tick_under_a_filter_scrolls_the_list_to_the_moved_selection() {
    let path = fixture(PesVersion::Pes21, "category_filter_reveal");
    let mut view = SaveView::new();
    view.open(&path);
    view.filter = "79".to_owned();
    view.selected = Some(Selection::Player(701, 0));
    // The row the tick moves the selection to: the filtered list's first
    // player row.
    let (expected, expected_label) = {
        let file = view.session.as_ref().unwrap().file();
        let row = selection::list(file, &view.team, &view.filter, TeamCategory::Vgl)
            .into_iter()
            .find_map(|row| match row {
                selection::ListRow::Player(row) => Some(row),
                _ => None,
            })
            .expect("a player row under the filter");
        let player = file.player(row.player_id).expect("the listed player");
        let position = POSITIONS
            .get(usize::from(player.positions.registered))
            .copied()
            .unwrap_or("?");
        (
            Selection::Player(row.team_id, row.slot),
            format!(
                "{:02} {} {}",
                row.slot + 1,
                position,
                display_name(&player.name)
            ),
        )
    };
    let mut window = Headless::new();
    wheel_row_out_of_clip(&mut window, &mut view, |node| {
        node.label() == Some(expected_label.as_str())
    });

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, selector_combo(&nodes), &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, radio(&nodes, "VGL"), &mut view);
    assert_eq!(
        view.selected,
        Some(expected),
        "the selection moved to the first VGL player row"
    );
    assert_selected_row_in_clip(&mut window, &mut view);
}

// Picking All teams under a typed filter keeps the scroll: the list's
// rows are the same whichever team the dropdown names. A team picked,
// its selected row wheeled out of the clip, `All teams` picked — the
// top row stays where it was.
#[test]
fn picking_all_teams_under_a_filter_keeps_the_scroll() {
    let path = fixture(PesVersion::Pes21, "all_teams_filter_scroll");
    let mut view = SaveView::new();
    view.open(&path);
    view.filter = "82".to_owned();
    let mut window = Headless::new();
    // A team the filter lists, picked: its row can wheel out of the clip.
    pick_in_selector(&mut window, &mut view, "/umaJP/");
    assert_eq!(
        view.selected,
        Some(Selection::Team(827)),
        "premise: /umaJP/'s row selected"
    );

    wheel_row_out_of_clip(&mut window, &mut view, |node| {
        node.toggled() == Some(accesskit::Toggled::True)
    });
    let nodes = draw(&mut window, &mut view);
    let top_before = top_list_row(&nodes);

    pick_in_selector(&mut window, &mut view, "All teams");
    assert_eq!(view.team, TeamChoice::All, "premise: All teams picked");
    assert_eq!(
        view.selected,
        Some(Selection::Team(827)),
        "the pick keeps the selection"
    );
    let nodes = draw(&mut window, &mut view);
    assert_eq!(top_list_row(&nodes), top_before, "the pick moved no row");
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
    // What a drag of player 01 to a filled slot off the clip with rows
    // still below it ends in: `move_player`'s edit and the selection on
    // the dropped record's slot. (On the last row `Top` and `Nearest`
    // clamp to the same end; three rows under his, they part.)
    let last = team_entry(&view, 701)
        .roster
        .iter()
        .rposition(|slot| slot.player_id != 0)
        .expect("a rostered slot");
    let last = u8::try_from(last).expect("a roster is at most 40 slots");
    assert!(last > 10, "premise: a list longer than the pane shows");
    let target = last - 3;
    view.session.as_mut().unwrap().move_player(701, 0, target);
    view.selected = Some(Selection::Player(701, target));
    // A pane shorter than the team's 24-row list, the list scrolled to
    // its end.
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
        Some(Selection::Player(701, target)),
        "premise: the redo moved it back to his slot"
    );
    let (bounds, clip) = assert_selected_row_in_clip(&mut window, &mut view);
    assert!(
        (bounds.y1 - f64::from(clip.bottom())).abs() <= 1.0,
        "his row at the clip's bottom edge, `Nearest` ({bounds:?} vs {clip:?})"
    );
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
    click(&mut window, radio(&nodes, "VGL"), &mut view);
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

    pick_in_selector(&mut window, &mut view, "/a/");
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
