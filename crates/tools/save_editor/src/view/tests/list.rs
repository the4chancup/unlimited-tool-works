use super::*;

// SE-GUI-01
#[test]
fn an_open_save_lists_every_rostered_player_and_shows_the_first() {
    let path = fixture(PesVersion::Pes21, "open_lists");
    let file = EditFile::load(&path).unwrap();
    let rostered: usize = file
        .teams()
        .iter()
        .map(|team| {
            team.roster
                .iter()
                .filter(|slot| slot.player_id != 0)
                .count()
        })
        .sum();
    let mut view = SaveView::new();
    view.open(&path);
    // Tall enough for a team's row plus all of 701's 23 player rows (the
    // team row eats one row's height).
    let mut window = Headless::sized(DEFAULT_PANE.x, 1100.0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // The team selector is the left panel's combo (the detail holds combos of
    // its own now).
    let selector = nodes
        .iter()
        .filter(|node| node.role() == Role::ComboBox)
        .min_by(|a, b| {
            let x = |node: &&accesskit::Node| node.bounds().map(|r| r.x0).unwrap_or(f64::MAX);
            x(a).partial_cmp(&x(b)).expect("ordered bounds")
        })
        .expect("the team selector");
    assert_eq!(selector.value(), Some("ALL"));

    // Open it: ALL first, then the save's teams in order.
    click(&mut window, selector, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let mut expected = vec!["ALL".to_string()];
    expected.extend(file.teams().iter().map(|team| team.name.clone()));
    let mut items: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && node
                    .label()
                    .is_some_and(|label| expected.iter().any(|e| e == label))
        })
        .collect();
    items.sort_by(|a, b| {
        let y = |node: &&accesskit::Node| node.bounds().map(|r| r.y0).unwrap_or(0.0);
        y(a).partial_cmp(&y(b)).expect("ordered bounds")
    });
    let items: Vec<String> = items
        .iter()
        .filter_map(|node| node.label().map(str::to_owned))
        .collect();
    assert_eq!(items, expected, "the selector's entries");

    // Click 701's entry: the list holds only its players, headed by its
    // own row.
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (item, _) = nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("/3/"))
        .expect("team 701's entry");
    action_click(&mut window, &mut view, *item);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let rows: Vec<&accesskit::Node> = nodes.iter().filter(|node| player_row(node)).collect();
    assert_eq!(
        rows.len(),
        file.team(701)
            .unwrap()
            .roster
            .iter()
            .filter(|slot| slot.player_id != 0)
            .count(),
        "team 701's roster"
    );
    assert_eq!(view.team, TeamChoice::Team(701), "701 is the chosen team");

    // The list lays out only the visible rows (`show_rows`): a screenful is
    // drawn, and scrolling reaches every rostered player (the tail's test).
    // Picking the team selected its own row (the selector's pick rule);
    // re-select the first player for the info-strip checks below.
    view.team = TeamChoice::All;
    view.selected = Some(Selection::Player(701, 0));
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let drawn = nodes.iter().filter(|node| player_row(node)).count();
    assert!(
        drawn >= 20 && drawn < rostered,
        "a full first page, not the whole save ({drawn} of {rostered})"
    );
    // The first row is team 701's first rostered player: number, position
    // and display name.
    assert_eq!(
        row_labels(&nodes).first().map(String::as_str),
        Some("01 CB PLACEHOLDER"),
        "the first row"
    );

    // The info strip shows the first row's general fields as the file holds
    // them: id 70101 (the title's first text), height 180, weight 80.
    assert!(
        nodes.iter().any(|node| {
            node.role() == Role::Label
                && node.value().is_some_and(|value| value.starts_with("70101"))
        }),
        "the title's id"
    );
    for value in ["180", "80"] {
        assert!(
            nodes
                .iter()
                .any(|node| node.role() == Role::SpinButton && node.value() == Some(value)),
            "a {value} control"
        );
    }
}

// SE-GUI-01
#[test]
fn a_players_name_paints_in_the_colors_his_codes_set() {
    let path = fixture(PesVersion::Pes17, "colored_names");
    let file = EditFile::load(&path).unwrap();
    let player = file.player(71801).expect("player 71801");
    let spans = name_spans(&player.name);
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(718);
    let mut window = Headless::new();
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // The name is the row's third column, a galley of its own.
    let bounds = row(&nodes, "01 GK ROADTRIP").bounds().expect("bounds");
    let shape = text_in_rect(&shapes, "ROADTRIP", bounds);
    let painted: Vec<(egui::Color32, String)> = shape
        .galley
        .job
        .sections
        .iter()
        .map(|section| {
            (
                section.format.color,
                shape.galley.job.text[section.byte_range.start.0..section.byte_range.end.0]
                    .to_owned(),
            )
        })
        .collect();
    // The painted sections' plain runs carry `Color32::PLACEHOLDER`, which
    // the widget replaces with its own text color when painting (the
    // visuals' `text_color` for an unselected row): it is what
    // `name_spans`' `None` means. The layout merges adjacent equal-color
    // sections, so the expectation merges too.
    let normal = egui::Color32::PLACEHOLDER;
    let mut expected: Vec<(egui::Color32, String)> = Vec::new();
    for (color, text) in &spans {
        let color = match color {
            Some([red, green, blue]) => egui::Color32::from_rgb(*red, *green, *blue),
            None => normal,
        };
        match expected.last_mut() {
            Some((last_color, last_text)) if *last_color == color => last_text.push_str(text),
            _ => expected.push((color, text.to_string())),
        }
    }
    assert_eq!(painted, expected, "the row's painted runs");
}

// SE-GUI-02
#[test]
fn the_filter_finds_a_player_of_another_team_and_selecting_shows_him() {
    let path = fixture(PesVersion::Pes17, "cross_team_filter");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();

    // Click the filter box (its hint is its AccessKit placeholder), then type
    // the name.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let filter_box = nodes
        .iter()
        .find(|node| {
            node.role() == Role::TextInput
                && node.placeholder() == Some("Filter every team (name, shirt name or id)")
        })
        .expect("the filter box");
    click(&mut window, filter_box, &mut view);
    let (_, nodes, _) = window.frame(vec![egui::Event::Text("quality".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });

    // 70202 of team 702 is listed under a filter.
    let row = nodes
        .iter()
        .find(|node| node.role() == Role::Button && node.label() == Some("02 GK QUALITY"))
        .expect("the filtered row");
    click(&mut window, row, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        nodes.iter().any(|node| {
            node.role() == Role::Label
                && node.value().is_some_and(|value| value.starts_with("70202"))
        }),
        "the title's id"
    );

    // His id and his shirt name find him too: "anime" is in his shirt name
    // alone.
    let entry = player_entry(&view, 70202);
    assert_eq!(
        (entry.name.as_str(), entry.shirt_name.as_str()),
        ("QUALITY", "QUALITY ANIME"),
        "premise: 70202's names"
    );
    for typed in ["70202", "anime"] {
        view.filter = typed.to_owned();
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        assert!(
            nodes
                .iter()
                .any(|node| node.role() == Role::Button && node.label() == Some("02 GK QUALITY")),
            "{typed:?} finds 70202"
        );
    }
}

// The selected row's AccessKit node reports toggled, the others not (egui
// maps a selectable's `selected` to `toggled`).
#[test]
fn the_selected_row_reports_selected() {
    let path = fixture(PesVersion::Pes21, "selected_row");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let rows: Vec<&accesskit::Node> = nodes.iter().filter(|node| player_row(node)).collect();
    let (selected, rest): (Vec<&&accesskit::Node>, Vec<&&accesskit::Node>) = rows
        .iter()
        .partition(|node| node.toggled() == Some(accesskit::Toggled::True));
    assert_eq!(selected.len(), 1, "exactly one row selected");
    assert_eq!(selected[0].label(), Some("01 CB PLACEHOLDER"));
    assert!(
        rest.iter()
            .all(|node| node.toggled() == Some(accesskit::Toggled::False))
    );
}

// A player row's drag handle stands 4 points from the row's left edge and
// 4 before his place, in ink, so the name gets the room: on the first row
// (`01`, whose `0` paints from its box's edge) and the tenth (`10`, whose
// `1` paints a point in), so boxes alone would miss one of the two.
#[test]
fn a_player_rows_handle_stands_4_from_its_edge_and_4_before_the_place() {
    let path = fixture(PesVersion::Pes21, "row_handle");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for place in ["01 ", "10 "] {
        let row = nodes
            .iter()
            .filter(|node| player_row(node) && node.label().is_some_and(|l| l.starts_with(place)))
            .filter_map(accesskit::Node::bounds)
            .min_by(|a, b| a.y0.partial_cmp(&b.y0).expect("ordered bounds"))
            .unwrap_or_else(|| panic!("a player row at place {place:?}"));
        let in_row = |shape: &&egui::epaint::TextShape| {
            let y = f64::from(shape.pos.y);
            y >= row.y0 && y < row.y1
        };
        let texts: Vec<&egui::epaint::TextShape> = shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) => Some(shape),
                _ => None,
            })
            .filter(in_row)
            .collect();
        let handle = texts
            .iter()
            .find(|shape| shape.galley.job.text == "\u{2261}")
            .unwrap_or_else(|| panic!("the row {place:?} paints its handle"));
        let text = texts
            .iter()
            .find(|shape| shape.galley.job.text == place.trim_end())
            .unwrap_or_else(|| panic!("the row {place:?} paints its place"));
        let handle_ink = ink(handle);
        let place_ink = ink(text);
        let inset = f64::from(handle_ink.left()) - row.x0;
        assert!(
            (inset - 4.0).abs() <= 0.5,
            "{place:?}: the handle's ink {inset} from the row's edge"
        );
        let gap = place_ink.left() - handle_ink.right();
        assert!(
            (gap - 4.0).abs() <= 0.5,
            "{place:?}: the handle's ink {gap} before the place's"
        );
    }
}

// A player row is one line, truncated, its full name on hover: a 52-byte
// name keeps every row at one line, and the scroll still reaches the last
// player.
#[test]
fn a_long_name_truncates_instead_of_wrapping_the_row() {
    let path = fixture(PesVersion::Pes21, "long_row");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().name = "N".repeat(52);
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    // 600 high: at the default pane's 756, 701's roster fits and the list
    // does not scroll.
    let mut window = Headless::sized(DEFAULT_PANE.x, 600.0);
    let row_height = window
        .ctx
        .style_of(window.ctx.theme())
        .spacing
        .interact_size
        .y;
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let rows: Vec<&accesskit::Node> = nodes.iter().filter(|node| player_row(node)).collect();
    assert!(!rows.is_empty());
    for row in &rows {
        let bounds = row.bounds().expect("row bounds");
        assert_eq!(
            bounds.y1 - bounds.y0,
            f64::from(row_height),
            "every row is one line"
        );
    }

    // Drag the scroll bar to the bottom: the last rostered row still shows.
    let bar = nodes
        .iter()
        .filter(|node| node.role() == Role::ScrollBar)
        .min_by(|a, b| {
            let x = |node: &&accesskit::Node| node.bounds().map(|r| r.x0).unwrap_or(0.0);
            x(a).partial_cmp(&x(b)).expect("ordered bounds")
        })
        .expect("the list's scroll bar");
    let track = centre(bar);
    window.frame(
        pointer_at(egui::pos2(track.x, track.y - 200.0), true),
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(
        vec![egui::Event::PointerMoved(egui::pos2(track.x, 595.0))],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(pointer_at(egui::pos2(track.x, 595.0), false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let file2 = EditFile::load(&path).unwrap();
    let (index, roster_slot) = file2
        .team(701)
        .unwrap()
        .roster
        .iter()
        .enumerate()
        .rev()
        .find(|(_, slot)| slot.player_id != 0)
        .expect("701 has a rostered slot");
    let player = file2
        .player(roster_slot.player_id)
        .expect("the last player");
    let label = format!(
        "{:02} {} {}",
        index + 1,
        pes_savefile::interchange::team_toml::labels::POSITIONS
            [usize::from(player.positions.registered)],
        pes_savefile::model::names::display_name(&player.name)
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    row(&nodes, &label);
}

// The list's scroll reaches every rostered player (SE-GUI-01's list drawn
// through `show_rows`): the last row of ALL clicks like any other.
#[test]
fn scrolling_to_the_end_of_all_selects_the_last_player() {
    let path = fixture(PesVersion::Pes21, "scroll_to_end");
    let mut file = EditFile::load(&path).unwrap();
    // The last rostered slot of the last team, renamed so its label —
    // which carries no team name — cannot be another team's too.
    let team_id = file.teams().last().expect("teams exist").id;
    let last_id = file
        .team(team_id)
        .expect("the last team")
        .roster
        .iter()
        .rev()
        .find(|slot| slot.player_id != 0)
        .expect("the last team has a rostered slot")
        .player_id;
    file.player_mut(last_id).unwrap().name = "LAST".to_owned();
    file.save(&path).unwrap();
    let file = EditFile::load(&path).unwrap();
    let place = file
        .team(team_id)
        .expect("the last team")
        .roster
        .iter()
        .position(|slot| slot.player_id == last_id)
        .expect("the renamed slot")
        + 1;
    let player = file.player(last_id).expect("the last player");
    let label = format!(
        "{place:02} {} LAST",
        pes_savefile::interchange::team_toml::labels::POSITIONS
            [usize::from(player.positions.registered)]
    );

    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    // Drag the scroll bar's handle to the bottom: a pointer press inside the
    // bar, a move to its last pixels, release. The list's bar is the
    // leftmost (the detail's content scrolls too).
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bar = nodes
        .iter()
        .filter(|node| node.role() == Role::ScrollBar)
        .min_by(|a, b| {
            let x = |node: &&accesskit::Node| node.bounds().map(|r| r.x0).unwrap_or(0.0);
            x(a).partial_cmp(&x(b)).expect("ordered bounds")
        })
        .expect("the list's scroll bar");
    let track = centre(bar);
    let bottom = window.screen.y - 5.0;
    window.frame(
        pointer_at(egui::pos2(track.x, track.y - 200.0), true),
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(
        vec![egui::Event::PointerMoved(egui::pos2(track.x, bottom))],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(pointer_at(egui::pos2(track.x, bottom), false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let last = row(&nodes, &label);
    click(&mut window, last, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let id = last_id.to_string();
    assert!(
        nodes.iter().any(|node| {
            node.role() == Role::Label && node.value().is_some_and(|value| value.starts_with(&id))
        }),
        "the title's id"
    );
}

// SE-GUI-08
#[test]
fn a_row_dragged_above_another_trades_their_records_between_the_ids() {
    let path = fixture(PesVersion::Pes21, "drag_reorder");
    // Distinguish the two records the drag trades (the fixture's placeholders
    // are identical): a name, a stat, a boot id and a base copy each.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().name = "FIRST".to_owned();
    file.player_mut(70101).unwrap().stats.speed = 70;
    file.player_mut(70101).unwrap().appearance.boots_id = 11;
    file.player_mut(70101).unwrap().appearance.base_copy_id = 9001;
    file.player_mut(70102).unwrap().name = "SECOND".to_owned();
    file.player_mut(70102).unwrap().stats.speed = 71;
    file.player_mut(70102).unwrap().appearance.boots_id = 12;
    file.player_mut(70102).unwrap().appearance.base_copy_id = 9002;
    file.save(&path).unwrap();
    let roster_before = file.team(701).unwrap().roster.clone();
    let first = file.player(70101).unwrap().clone();
    let second = file.player(70102).unwrap().clone();

    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // Select another row first, so the selection following the dragged
    // record is not vacuous.
    click(&mut window, row(&nodes, "03 CB PLACEHOLDER"), &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 2)),
        "premise: row 03 selected"
    );

    // Drag player 02's row above player 01's.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let from = centre(row(&nodes, "02 CB SECOND"));
    let onto = centre(row(&nodes, "01 CB FIRST"));
    drag(&mut window, from, onto, &mut view);

    // The list shows the new order, the moved rows marked unsaved; the
    // selection followed the dragged record to its new slot.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let labels = row_labels(&nodes);
    assert_eq!(&labels[..2], ["01 CB SECOND ●", "02 CB FIRST ●"]);
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "the selection sits on the dropped record's slot"
    );

    // Save, reopen: slot 01 still holds player 70101, whose record is the
    // one player 02 had, and the other way round; the roster is untouched.
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let reopened = EditFile::load(&path).unwrap();
    let team = reopened.team(701).expect("team 701");
    assert_eq!(
        team.roster, roster_before,
        "the roster's ids and shirt numbers are as they were"
    );
    let held = reopened.player(70101).expect("player 70101");
    assert_eq!(record(held), record(&second), "slot 01's record is 70102's");
    assert_eq!(held.id, 70101);
    assert_eq!(held.appearance_id, Some(70101));
    assert_eq!(held.appearance.base_copy_id, 9001);
    let held = reopened.player(70102).expect("player 70102");
    assert_eq!(record(held), record(&first), "slot 02's record is 70101's");
    assert_eq!(held.id, 70102);
    assert_eq!(held.appearance_id, Some(70102));
    assert_eq!(held.appearance.base_copy_id, 9002);

    // The roster-order rule is unbroken.
    let view_team = aatf::TeamView::new(&reopened, 701).expect("the team views");
    let violations = aatf::Rules::official()
        .check_team(&view_team)
        .expect("the check runs");
    assert!(
        violations
            .iter()
            .all(|violation| !matches!(violation.kind, aatf::ViolationKind::RosterOrder { .. })),
        "no RosterOrder: {violations:?}"
    );
}

// SE-GUI-08, under ALL, where each team's rows run unbroken under its
// team row: a drop on a row of the dragged player's own team moves him as
// in one team's list; a drop on another team's row or on a team row does
// nothing.
#[test]
fn under_all_a_drag_moves_within_the_team_and_nowhere_else() {
    let path = fixture(PesVersion::Pes21, "drag_reorder_all");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().name = "FIRST".to_owned();
    file.player_mut(70102).unwrap().name = "SECOND".to_owned();
    // Team 702's first rostered player renamed too: a row's label carries
    // no team name, so the drop target's cannot be another team's too.
    let other_id = file
        .team(702)
        .unwrap()
        .roster
        .iter()
        .find(|slot| slot.player_id != 0)
        .expect("team 702 rosters someone")
        .player_id;
    file.player_mut(other_id).unwrap().name = "OTHER".to_owned();
    file.save(&path).unwrap();
    let other_label = {
        let place = file
            .team(702)
            .unwrap()
            .roster
            .iter()
            .position(|slot| slot.player_id == other_id)
            .expect("the renamed slot")
            + 1;
        let player = file.player(other_id).unwrap();
        format!(
            "{place:02} {} OTHER",
            pes_savefile::interchange::team_toml::labels::POSITIONS
                [usize::from(player.positions.registered)]
        )
    };

    let mut view = SaveView::new();
    view.open(&path);
    assert_eq!(view.team, TeamChoice::All, "open selects ALL");
    // Tall enough for team 702's block to be drawn under team 701's.
    let mut window = Headless::sized(DEFAULT_PANE.x, 1100.0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // A drag onto another team's row: nothing moves.
    let before = row_labels(&nodes);
    let from = centre(row(&nodes, "02 CB SECOND"));
    let other = centre(row(&nodes, &other_label));
    drag(&mut window, from, other, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        row_labels(&nodes),
        before,
        "a drop on another team's row moved nothing"
    );
    assert!(!view.session.as_ref().unwrap().is_dirty());
    assert!(!view.session.as_ref().unwrap().can_undo());

    // A drag of the team row itself onto a player row: nothing moves.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let from = centre(team_row(&nodes, &view, 701));
    let onto = centre(row(&nodes, "01 CB FIRST"));
    drag(&mut window, from, onto, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        row_labels(&nodes),
        before,
        "a drag of the team row moved nothing"
    );
    assert!(!view.session.as_ref().unwrap().is_dirty());
    assert!(!view.session.as_ref().unwrap().can_undo());

    // A drag onto the team row: nothing moves.
    let from = centre(row(&nodes, "02 CB SECOND"));
    let onto = centre(team_row(&nodes, &view, 701));
    drag(&mut window, from, onto, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        row_labels(&nodes),
        before,
        "a drop on the team row moved nothing"
    );
    assert!(!view.session.as_ref().unwrap().is_dirty());
    assert!(!view.session.as_ref().unwrap().can_undo());

    // A drag onto the player's own team's row: the records trade, as in
    // one team's list.
    let from = centre(row(&nodes, "02 CB SECOND"));
    let onto = centre(row(&nodes, "01 CB FIRST"));
    drag(&mut window, from, onto, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        &row_labels(&nodes)[..2],
        ["01 CB SECOND ●", "02 CB FIRST ●"],
        "the drop on his own team moved him"
    );
}

// Drag is off while a filter is typed, where neighbouring rows are not
// one team's neighbouring slots: under a filter listing two non-adjacent
// players of one team, a drag of one onto the other moves nothing, since
// a drop there would move the hidden records between them too.
#[test]
fn under_a_filter_a_drag_moves_nothing() {
    let path = fixture(PesVersion::Pes21, "drag_filtered");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    // Two non-adjacent players of team 701 the filter matches (slot 1
    // stays hidden).
    file.player_mut(70101).unwrap().name = "QQ FIRST".to_owned();
    file.player_mut(70103).unwrap().name = "QQ THIRD".to_owned();
    file.save(&path).unwrap();
    let roster_before = file.team(701).unwrap().roster.clone();

    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    view.filter = "qq".to_owned();
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let labels_before = row_labels(&nodes);
    assert_eq!(
        labels_before,
        ["01 CB QQ FIRST", "03 CB QQ THIRD"],
        "premise: the filter lists the two"
    );

    let from = centre(row(&nodes, "01 CB QQ FIRST"));
    let onto = centre(row(&nodes, "03 CB QQ THIRD"));
    drag(&mut window, from, onto, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        row_labels(&nodes),
        labels_before,
        "the filtered rows did not move"
    );
    let session = view.session.as_ref().unwrap();
    assert_eq!(
        session.file().team(701).unwrap().roster,
        roster_before,
        "the roster is unchanged"
    );
    assert!(!session.is_dirty(), "nothing is marked unsaved");
    assert!(!session.can_undo(), "no undo entry");
}

/// The `TextShape` whose galley text starts with `starts_with`, drawn
/// inside `rect`.
fn text_in_rect<'a>(
    shapes: &'a [egui::epaint::ClippedShape],
    starts_with: &str,
    rect: accesskit::Rect,
) -> &'a egui::epaint::TextShape {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape)
                if shape.galley.job.text.starts_with(starts_with)
                    && f64::from(shape.pos.y) >= rect.y0
                    && f64::from(shape.pos.y) <= rect.y1
                    && f64::from(shape.pos.x) >= rect.x0
                    && f64::from(shape.pos.x) <= rect.x1 =>
            {
                Some(shape)
            }
            _ => None,
        })
        .next()
        .unwrap_or_else(|| panic!("no painted text {starts_with:?} inside {rect:?}"))
}

// The team row's own paint, the draft's `trow`: the id in the weak
// colour at the row's left padding, the colours 1-2 as swatches 3 apart,
// the name in the strong colour after a further item spacing, each
// centred on the row — over the faint fill that sets it apart from a
// player row, the selection's or the hover's fill over that.
#[test]
fn the_team_row_paints_its_parts_over_its_fills() {
    /// Whether a `Shape::Rect` of `fill` covers `rect`'s centre and spans
    /// its width (the row's own fill, not a swatch's or the panel's).
    fn covers_row(
        shapes: &[egui::epaint::ClippedShape],
        rect: accesskit::Rect,
        fill: egui::Color32,
    ) -> bool {
        let centre = egui::pos2(
            ((rect.x0 + rect.x1) / 2.0) as f32,
            ((rect.y0 + rect.y1) / 2.0) as f32,
        );
        shapes.iter().any(|clipped| {
            matches!(&clipped.shape, egui::Shape::Rect(shape) if shape.fill == fill
                && shape.rect.contains(centre)
                && f64::from(shape.rect.width()) >= rect.x1 - rect.x0 - 1.0)
        })
    }

    let path = fixture(PesVersion::Pes21, "team_row_paint");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    // The stored colours the swatches show, read as a premise.
    let colors = team_entry(&view, 701)
        .colors
        .expect("PES 21 stores team colours");
    let mut window = Headless::new();
    let (spacing, visuals) = {
        let style = window.ctx.style_of(window.ctx.theme());
        (style.spacing.clone(), style.visuals.clone())
    };
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let row = team_row(&nodes, &view, 701)
        .bounds()
        .expect("the team row's bounds");
    let centre_y = (row.y0 + row.y1) / 2.0;
    let padding = f64::from(spacing.button_padding.x);
    let item_spacing = f64::from(spacing.item_spacing.x);

    // The id's galley: its left at the row's left plus the button padding,
    // centred on the row, in the weak colour.
    let id_shape = text_in_rect(&shapes, "701", row);
    assert!(
        (f64::from(id_shape.pos.x) - (row.x0 + padding)).abs() <= 0.5,
        "the id starts at the row's left plus the padding: {row:?} vs {:?}",
        id_shape.pos
    );
    assert!(
        (f64::from(id_shape.pos.y) + f64::from(id_shape.galley.size().y) / 2.0 - centre_y).abs()
            <= 0.5,
        "the id centres on the row"
    );
    assert_eq!(
        id_shape.galley.job.sections.first().map(|s| s.format.color),
        Some(visuals.weak_text_color()),
        "the id in the weak colour"
    );

    // The two swatches, each 10 x 10 centred on the row: the first an item
    // spacing after the id, the second 3 after the first.
    let mut swatches: Vec<(egui::Rect, egui::Color32)> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if (f64::from(shape.rect.width()) - 10.0).abs() < 0.5
                    && (f64::from(shape.rect.height()) - 10.0).abs() < 0.5
                    && (f64::from(shape.rect.center().y) - centre_y).abs() < 1.0
                    && f64::from(shape.rect.min.x) >= row.x0
                    && f64::from(shape.rect.max.x) <= row.x1 =>
            {
                Some((shape.rect, shape.fill))
            }
            _ => None,
        })
        .collect();
    swatches.sort_by(|a, b| a.0.min.x.total_cmp(&b.0.min.x));
    assert_eq!(swatches.len(), 2, "the two colour swatches");
    let id_end = f64::from(id_shape.pos.x + id_shape.galley.size().x);
    assert!(
        (f64::from(swatches[0].0.min.x) - (id_end + item_spacing)).abs() <= 0.5,
        "the first swatch an item spacing after the id"
    );
    assert!(
        (f64::from(swatches[1].0.min.x) - (f64::from(swatches[0].0.max.x) + 3.0)).abs() <= 0.5,
        "the second swatch 3 after the first"
    );
    for (index, (_rect, fill)) in swatches.iter().enumerate() {
        let color = colors[index];
        let expected = egui::Color32::from_rgb(
            crate::view::team::channel_to_show(color.red),
            crate::view::team::channel_to_show(color.green),
            crate::view::team::channel_to_show(color.blue),
        );
        assert_eq!(*fill, expected, "colour {}'s swatch", index + 1);
    }

    // The name's galley an item spacing after the second swatch, centred
    // on the row, the plain runs in the strong colour.
    let name_shape = text_in_rect(&shapes, "/3/", row);
    assert!(
        (f64::from(name_shape.pos.x) - (f64::from(swatches[1].0.max.x) + 3.0 + item_spacing)).abs()
            <= 0.5,
        "the name follows the second swatch"
    );
    assert!(
        (f64::from(name_shape.pos.y) + f64::from(name_shape.galley.size().y) / 2.0 - centre_y)
            .abs()
            <= 0.5,
        "the name centres on the row"
    );
    assert_eq!(
        name_shape.fallback_color,
        visuals.strong_text_color(),
        "the name in the strong colour"
    );

    // Unselected under no pointer the row paints the faint fill that sets
    // it apart from a player row; hovered the hover's, selected the
    // selection's.
    assert!(
        covers_row(&shapes, row, visuals.faint_bg_color),
        "the faint fill under the unselected row"
    );
    let (_, _, shapes) = window.frame(
        vec![egui::Event::PointerMoved(egui::pos2(
            ((row.x0 + row.x1) / 2.0) as f32,
            centre_y as f32,
        ))],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert!(
        covers_row(&shapes, row, visuals.widgets.hovered.bg_fill),
        "the hover fill under the hovered row"
    );
    assert!(
        !covers_row(&shapes, row, visuals.faint_bg_color),
        "no faint fill under the hovered row"
    );
    // Selected with the pointer gone (a frame before: the frame that
    // receives `PointerGone` still reports the row hovered), so the
    // selection alone holds the faint fill back.
    window.frame(vec![egui::Event::PointerGone], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    view.selected = Some(Selection::Team(701));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let row = team_row(&nodes, &view, 701)
        .bounds()
        .expect("the team row's bounds");
    assert!(
        covers_row(&shapes, row, visuals.selection.bg_fill),
        "the selection fill under the selected row"
    );
    assert!(
        !covers_row(&shapes, row, visuals.faint_bg_color),
        "no faint fill under the selected row"
    );

    // Unselected, the pointer still gone, focused by the keyboard: no
    // faint fill either.
    view.selected = None;
    let (_, ids, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let label = format!("701 {}", display_name(&team_entry(&view, 701).name));
    let (row_id, _) = ids
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(label.as_str()))
        .expect("the team row's node");
    action_focus(&mut window, &mut view, *row_id);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        !covers_row(&shapes, row, visuals.faint_bg_color),
        "no faint fill under the focused row"
    );
}

// The same row, the name too long for the side panel: its galley ends
// inside the row's right minus the padding rather than past it.
#[test]
fn a_team_name_too_long_truncates_inside_the_row() {
    let path = fixture(PesVersion::Pes21, "team_row_long_name");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    // The version's own field length in bytes, read from the schema —
    // long enough that the name never fits the side panel.
    let name = "A".repeat(
        usize::try_from(
            pes_savefile::schema::schema_for(PesVersion::Pes21)
                .team
                .texts
                .iter()
                .find(|spec| spec.text == pes_savefile::schema::fields::TeamText::Name)
                .expect("every version stores the team name")
                .len,
        )
        .expect("a text field's length fits usize"),
    );
    file.team_mut(701).unwrap().name = name.clone();
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();
    let padding = f64::from(
        window
            .ctx
            .style_of(window.ctx.theme())
            .spacing
            .button_padding
            .x,
    );
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let row = team_row(&nodes, &view, 701)
        .bounds()
        .expect("the team row's bounds");
    let name_shape = text_in_rect(&shapes, &name, row);
    assert!(name_shape.galley.elided, "the galley is truncated");
    let end = f64::from(name_shape.pos.x + name_shape.galley.size().x);
    assert!(
        end <= row.x1 - padding + 0.5 && end >= row.x1 - padding - 10.0,
        "the name's galley ends on the row's right minus the padding ({end} vs {})",
        row.x1 - padding
    );
}

// The whole player row is one click target, as the team row is: every
// player row spans the team row's width, the selected row's selection
// fill covers it, and a click past a short label's text still selects
// him.
#[test]
fn every_player_row_spans_the_team_rows_width_and_selects_at_its_end() {
    let path = fixture(PesVersion::Pes21, "row_right_click");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::new();
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let team = team_row(&nodes, &view, 701)
        .bounds()
        .expect("team row bounds");

    // Every player row's bounds span the team row's x range.
    let rows: Vec<&accesskit::Node> = nodes.iter().filter(|node| player_row(node)).collect();
    assert!(!rows.is_empty(), "player rows are drawn");
    for row in &rows {
        let bounds = row.bounds().expect("row bounds");
        assert!(
            (bounds.x0 - team.x0).abs() <= 1.0 && (bounds.x1 - team.x1).abs() <= 1.0,
            "a player row spans the team row's x range ({bounds:?} vs {team:?})"
        );
    }

    // The selected row's fill covers that width (a content-wide button
    // leaves the row's right half unpainted).
    let selected = rows
        .iter()
        .find(|node| node.toggled() == Some(accesskit::Toggled::True))
        .expect("the selected row")
        .bounds()
        .expect("bounds");
    let centre = egui::pos2(
        ((selected.x0 + selected.x1) / 2.0) as f32,
        ((selected.y0 + selected.y1) / 2.0) as f32,
    );
    assert!(
        shapes.iter().any(|clipped| matches!(&clipped.shape,
            egui::Shape::Rect(shape) if shape.fill == visuals.selection.bg_fill
                && shape.rect.contains(centre)
                && f64::from(shape.rect.width()) >= team.x1 - team.x0 - 1.0)),
        "the selection fill covers the row's width"
    );

    // A click at the row's right end, past a short label's text, selects
    // him (the list's scroll bar stands outside the rows' frame).
    view.selected = Some(Selection::Player(701, 3));
    let at = egui::pos2((team.x1 - 2.0) as f32, centre.y);
    click_at(&mut window, at, &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "the click selected the row"
    );
}

// SE-GUI-37's team-row dot: a team's own unsaved changes mark its row's
// right end — after a short-name edit the `●` paints there in the
// warning colour, right of the painted name — and no player row's label
// carries one (the dot is no part of the row's AccessKit label).
#[test]
fn the_team_rows_dot_paints_at_the_rows_right_end() {
    let path = fixture(PesVersion::Pes21, "team_row_dot");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();
    let mut team = team_entry(&view, 701);
    team.short_name = "ZED".to_owned();
    view.session.as_mut().unwrap().edit_team(1, team);

    let padding = f64::from(
        window
            .ctx
            .style_of(window.ctx.theme())
            .spacing
            .button_padding
            .x,
    );
    let warn = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .warn_fg_color;
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = team_row(&nodes, &view, 701).bounds().expect("bounds");
    let name = text_in_rect(&shapes, "/3/", bounds);
    let dot = dot_in_row(&shapes, bounds);
    assert_eq!(galley_color(dot), warn, "the dot in the warning colour");
    let dot_right = f64::from(dot.pos.x + dot.galley.rect.width());
    assert!(
        (dot_right - (bounds.x1 - padding)).abs() <= 0.5,
        "the dot's right edge at the row's right less the padding ({dot_right} vs {})",
        bounds.x1 - padding
    );
    assert!(
        f64::from(dot.pos.x) >= f64::from(name.pos.x + name.galley.rect.width()),
        "the dot sits right of the painted name"
    );
    let row_centre = (bounds.y0 + bounds.y1) / 2.0;
    let dot_centre = f64::from(dot.pos.y + dot.galley.size().y / 2.0);
    assert!(
        (dot_centre - row_centre).abs() <= 0.5,
        "the dot centres on the row ({dot_centre} vs {row_centre})"
    );
    assert!(
        row_labels(&nodes)
            .iter()
            .all(|label| !label.contains('\u{25cf}')),
        "no player row's label carries one"
    );
}

// A player row's dot the same way: a player's unsaved changes paint it
// at his row's right end in the warning colour, right of the text (the
// draft's `.plist .dot`), with no Live AATF mark to sit before.
#[test]
fn the_player_rows_dot_paints_at_the_rows_right_end() {
    let path = fixture(PesVersion::Pes21, "player_row_dot");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut entry = player_entry(&view, 70101);
    entry.basic.height = 177;
    view.session.as_mut().unwrap().edit_players(vec![entry]);
    let mut window = Headless::new();
    let padding = f64::from(
        window
            .ctx
            .style_of(window.ctx.theme())
            .spacing
            .button_padding
            .x,
    );
    let warn = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .warn_fg_color;

    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = row(&nodes, "01 CB PLACEHOLDER ●")
        .bounds()
        .expect("the row's bounds");
    let text = text_in_rect(&shapes, "PLACEHOLDER", bounds);
    let dot = dot_in_row(&shapes, bounds);
    assert_eq!(galley_color(dot), warn, "the dot in the warning colour");
    let dot_right = f64::from(dot.pos.x + dot.galley.rect.width());
    assert!(
        (dot_right - (bounds.x1 - padding)).abs() <= 0.5,
        "the dot's right edge at the row's right less the padding ({dot_right} vs {})",
        bounds.x1 - padding
    );
    assert!(
        f64::from(dot.pos.x) >= f64::from(text.pos.x + text.galley.rect.width()),
        "the dot sits right of the text"
    );
    let row_centre = (bounds.y0 + bounds.y1) / 2.0;
    for (what, shape) in [("text", text), ("dot", dot)] {
        let centre = f64::from(shape.pos.y + shape.galley.size().y / 2.0);
        assert!(
            (centre - row_centre).abs() <= 0.5,
            "the {what} centres on the row ({centre} vs {row_centre})"
        );
    }
}

// The list is 255 points wide and not resizable, since under the design
// width the window scales rather than narrows: a drag on the list's right
// edge leaves it.
#[test]
fn a_drag_on_the_lists_right_edge_does_not_resize_it() {
    let path = fixture(PesVersion::Pes21, "list_fixed_width");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    // The list's column, from its frame's left edge to the end of the
    // scroll bar's strip (the list scrolls under ALL).
    let column = |window: &mut Headless, view: &mut SaveView| {
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
        let width = list_scroll_bar(&nodes).x1 - f64::from(list_frame(window, &shapes).min.x);
        (width, pane_frame(&shapes, window.screen).0)
    };
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (width, before) = column(&mut window, &mut view);
    assert!(
        (width - 255.0).abs() <= 0.5,
        "the list's column is 255 wide ({width})"
    );

    // A drag from the list's right edge, the pane's left, 80 points to the
    // right.
    let from = egui::pos2(before as f32, 400.0);
    let onto = egui::pos2(before as f32 + 80.0, 400.0);
    drag(&mut window, from, onto, &mut view);
    let (width, after) = column(&mut window, &mut view);
    assert!(
        (width - 255.0).abs() <= 0.5 && (after - before).abs() <= 0.5,
        "the drag left the list at 255 ({width}) and the pane's edge where it was ({after} vs \
         {before})"
    );
}

/// The team selector's combo (the list panel's leftmost), clicked open;
/// the popup's `entry` item picked through its AccessKit action.
fn pick_in_selector(window: &mut Headless, view: &mut SaveView, entry: &str) {
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    // The popup stays open after an AccessKit pick; a second call's click
    // would toggle it closed.
    if !nodes
        .iter()
        .any(|node| node.role() == Role::Button && node.label() == Some(entry))
    {
        let selector = nodes
            .iter()
            .filter(|node| node.role() == Role::ComboBox)
            .min_by(|a, b| {
                let x = |node: &&accesskit::Node| node.bounds().map(|r| r.x0).unwrap_or(f64::MAX);
                x(a).partial_cmp(&x(b)).expect("ordered bounds")
            })
            .expect("the team selector");
        click(window, selector, view);
    }
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let (item, _) = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(entry))
        .unwrap_or_else(|| panic!("no selector entry {entry:?}"));
    let item = *item;
    action_click(window, view, item);
}

// Picking a team in the team selector selects its team row, so the
// detail shows the team's view and its tactics; picking ALL keeps the
// selection.
#[test]
fn picking_a_team_in_the_selector_selects_its_team_row() {
    let path = fixture(PesVersion::Pes21, "selector_selects_row");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "premise: the first row"
    );

    pick_in_selector(&mut window, &mut view, "/a/");
    assert_eq!(
        view.selected,
        Some(Selection::Team(702)),
        "picking /a/ selected its team row"
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        nodes
            .iter()
            .any(|node| node.role() == Role::Button && node.label() == Some("Kit slots\u{2026}")),
        "the detail shows the team view"
    );

    pick_in_selector(&mut window, &mut view, "ALL");
    assert_eq!(
        view.selected,
        Some(Selection::Team(702)),
        "picking ALL keeps the selection"
    );
}

// The team row draws `[click for tactics]` after the name, in the weak
// colour: nothing else says where the tactics are.
#[test]
fn the_team_row_hints_click_for_tactics() {
    let path = fixture(PesVersion::Pes21, "team_row_hint");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();
    let weak = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .weak_text_color();
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = team_row(&nodes, &view, 701)
        .bounds()
        .expect("the team row's bounds");
    let hint = text_in_rect(&shapes, "[click for tactics]", bounds);
    assert_eq!(galley_color(hint), weak, "the hint in the weak colour");
    let name = text_in_rect(&shapes, "/3/", bounds);
    let gap = hint.pos.x - (name.pos.x + name.galley.size().x);
    assert!(
        gap >= 0.0
            && gap
                <= window
                    .ctx
                    .style_of(window.ctx.theme())
                    .spacing
                    .item_spacing
                    .x
                    + 1.0,
        "the hint follows the name with at most an item spacing's gap ({gap})"
    );
}

// The hint gives way first on a narrow row (the draft's `.thint`): a
// name leaving part of its room elides the hint while painting whole
// itself; a name that fills the row drops the hint entirely.
#[test]
fn the_team_rows_hint_elides_before_the_name() {
    let path = fixture(PesVersion::Pes21, "team_row_hint_elides");
    // A name that leaves only part of the hint's room.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let name = "N".repeat(12);
    file.team_mut(701).unwrap().name = name.clone();
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = team_row(&nodes, &view, 701)
        .bounds()
        .expect("the team row's bounds");
    let name_shape = text_in_rect(&shapes, &name, bounds);
    assert!(!name_shape.galley.elided, "the name still paints whole");
    let hint = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape)
                if shape.galley.job.text.starts_with("[click")
                    && f64::from(shape.pos.y) >= bounds.y0
                    && f64::from(shape.pos.y) <= bounds.y1 =>
            {
                Some(shape)
            }
            _ => None,
        })
        .next()
        .expect("the hint still paints, elided");
    assert!(hint.galley.elided, "the hint elides before the name does");

    // A name the panel cannot hold keeps the row to itself: no hint at
    // all, the name elided instead.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let name = "N".repeat(52);
    file.team_mut(701).unwrap().name = name.clone();
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = team_row(&nodes, &view, 701)
        .bounds()
        .expect("the team row's bounds");
    let name_shape = text_in_rect(&shapes, "NNNN", bounds);
    assert!(name_shape.galley.elided, "the name elides");
    assert!(
        !shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) => {
                shape.galley.job.text.starts_with("[click")
                    && f64::from(shape.pos.y) >= bounds.y0
                    && f64::from(shape.pos.y) <= bounds.y1
            }
            _ => false,
        }),
        "no hint paints on the full row"
    );
}

// The list's scroll bar stands outside its frame: egui's solid bar, 4
// points from the content and 6 wide, right of the frame, so a selected
// row's fill reaches the frame's right edge and the bar covers no part of
// a row. The team combo and the filter above end at the frame's right
// edge, not at the bar's.
#[test]
fn the_lists_scroll_bar_stands_outside_its_frame() {
    let path = fixture(PesVersion::Pes21, "list_bar_outside");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::new();
    let selection = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .selection
        .bg_fill;
    // The bar shows from the frame after the list first measured its rows.
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let frame = list_frame(&window, &shapes);
    let content_right = f64::from(frame.max.x);

    let bar = list_scroll_bar(&nodes);
    assert!(
        (bar.x0 - content_right - 4.0).abs() <= 0.5,
        "the bar 4 points right of the content ({bar:?} vs {content_right})"
    );
    assert!(
        (bar.x1 - bar.x0 - 6.0).abs() <= 0.5,
        "the bar 6 points wide ({bar:?})"
    );

    // The selected row's fill, as far as it shows, reaches the frame's
    // right edge.
    let selected = nodes
        .iter()
        .filter(|node| player_row(node))
        .find(|node| node.toggled() == Some(accesskit::Toggled::True))
        .and_then(accesskit::Node::bounds)
        .expect("the selected row");
    let centre = egui::pos2(
        ((selected.x0 + selected.x1) / 2.0) as f32,
        ((selected.y0 + selected.y1) / 2.0) as f32,
    );
    let fill = shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if shape.fill == selection && shape.rect.contains(centre) => {
                Some(shape.rect.intersect(clipped.clip_rect))
            }
            _ => None,
        })
        .expect("the selected row's fill");
    assert!(
        (f64::from(fill.max.x) - content_right).abs() <= 0.5,
        "the selection fill reaches the frame's edge ({fill:?} vs {content_right})"
    );

    // The combo and the filter end at the frame's right edge.
    for role in [Role::ComboBox, Role::TextInput] {
        let control = nodes
            .iter()
            .filter(|node| node.role() == role)
            .filter_map(accesskit::Node::bounds)
            .find(|bounds| bounds.x0 < f64::from(frame.max.x))
            .unwrap_or_else(|| panic!("the list's {role:?}"));
        assert!(
            (control.x0 - f64::from(frame.min.x)).abs() <= 0.5
                && (control.x1 - content_right).abs() <= 0.5,
            "the {role:?} spans the frame ({control:?} vs {frame:?})"
        );
    }
}

// The list's frame stands where the draft's does at the 1280 x 780
// window: its left edge at the view's, the shell's margin into the pane
// (64 in the window), and its right edge 245 further (the draft's `.rows`,
// its border included), the 255-point list column being the frame and the
// bar's 10-point strip.
#[test]
fn the_lists_frame_stands_where_the_drafts_does() {
    let path = fixture(PesVersion::Pes21, "list_frame_edges");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let frame = list_frame(&window, &shapes);
    let right = PANEL_MARGIN + 245.0;
    assert!(
        (frame.min.x - PANEL_MARGIN).abs() <= 0.5 && (frame.max.x - right).abs() <= 0.5,
        "the frame from the view's left edge {PANEL_MARGIN} to {right}, as the draft's \
         ({frame:?})"
    );
}

// The list's column stands where the draft's `.plist` does at the
// 1280 x 780 window: the team combo's top 30, 4 under the toolbar's
// buttons, then the filter box and the rows' frame each 3 under the
// last, egui's item spacing, the frame's top at 72.
#[test]
fn the_lists_controls_stand_3_apart_from_the_drafts_top() {
    let path = fixture(PesVersion::Pes21, "list_column_tops");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let frame = list_frame(&window, &shapes);
    let top_of = |role: Role| {
        nodes
            .iter()
            .filter(|node| node.role() == role)
            .filter_map(accesskit::Node::bounds)
            .find(|bounds| bounds.x0 < f64::from(frame.max.x))
            .unwrap_or_else(|| panic!("the list's {role:?}"))
            .y0
    };
    let tops = [
        top_of(Role::ComboBox),
        top_of(Role::TextInput),
        f64::from(frame.min.y),
    ];
    for (top, draft) in tops.iter().zip([30.0, 51.0, 72.0]) {
        assert!(
            (top - draft).abs() <= 0.5,
            "the combo, the filter and the frame at the draft's 30, 51 and 72 ({tops:?})"
        );
    }
}

// The bar's strip stays while a filter leaves nothing to scroll, so the
// list keeps its width: with a filter leaving three rows the frame is as
// wide as with the whole list, its rows as wide as it, and no bar shows.
#[test]
fn a_filter_leaving_three_rows_keeps_the_bars_strip() {
    let path = fixture(PesVersion::Pes21, "list_strip_kept");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().name = "QQ FIRST".to_owned();
    file.player_mut(70103).unwrap().name = "QQ THIRD".to_owned();
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let whole = list_frame(&window, &shapes);
    assert!(
        list_scroll_bar(&nodes).x0 > f64::from(whole.max.x),
        "premise: the whole list scrolls"
    );

    view.filter = "qq".to_owned();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let rows: Vec<accesskit::Rect> = nodes
        .iter()
        .filter(|node| node.role() == Role::Button && node.bounds().is_some_and(|b| b.x1 < 300.0))
        .filter(|node| player_row(node) || node.label() == Some("701 /3/"))
        .filter_map(accesskit::Node::bounds)
        .collect();
    assert_eq!(rows.len(), 3, "premise: the filter leaves three rows");
    let filtered = list_frame(&window, &shapes);
    assert!(
        (filtered.min.x - whole.min.x).abs() <= 0.5 && (filtered.max.x - whole.max.x).abs() <= 0.5,
        "the frame keeps its width ({filtered:?} vs {whole:?})"
    );
    for row in rows {
        assert!(
            (row.x1 - f64::from(filtered.max.x)).abs() <= 0.5,
            "a row spans the frame ({row:?} vs {filtered:?})"
        );
    }
    assert!(
        !nodes.iter().any(|node| {
            node.role() == Role::ScrollBar && node.bounds().is_some_and(|b| b.x0 < 300.0)
        }),
        "no bar shows while nothing scrolls"
    );
}

// A player row numbers him by his place in the roster, `01` to `23` (the
// slot `NN` of his id), not by his shirt number: team 757 (/v/) of the
// test save, whose shirt numbers (111, 333, 10, ...) are not its places.
#[test]
fn a_player_row_numbers_his_place_in_the_roster() {
    let path = fixture(PesVersion::Pes21, "list_places");
    let file = EditFile::load(&path).unwrap();
    let team = file.team(757).expect("team 757");
    let expected: Vec<String> = team
        .roster
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.player_id != 0)
        .map(|(index, slot)| {
            let player = file.player(slot.player_id).expect("a rostered player");
            format!(
                "{:02} {} {}",
                index + 1,
                POSITIONS[usize::from(player.positions.registered)],
                display_name(&player.name)
            )
        })
        .collect();
    assert_eq!(expected.len(), 23, "premise: 757 rosters 23 players");
    assert!(
        team.roster
            .iter()
            .take(3)
            .enumerate()
            .all(|(index, slot)| usize::from(slot.number) != index + 1),
        "premise: 757's first three shirt numbers are not their places"
    );

    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(757);
    let mut window = Headless::sized(DEFAULT_PANE.x, 1100.0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(row_labels(&nodes), expected, "the rows read 01 to 23");
}

// The rows are the draft's `.prow`s: 18 high and touching, the team's row
// too, under ALL. A player row's place, position and name are columns:
// the place in the weak colour, the position 5 points after it in a
// 28-point column, the name 5 points after that column.
#[test]
fn the_rows_are_18_high_touching_with_the_place_and_position_in_columns() {
    let path = fixture(PesVersion::Pes21, "list_row_columns");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let frame = list_frame(&window, &shapes);
    // Every row spans the frame: the team rows and the player rows, top
    // to bottom, the ones the frame shows whole.
    let mut rows: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| node.role() == Role::Button)
        .filter(|node| {
            node.bounds().is_some_and(|b| {
                (b.x0 - f64::from(frame.min.x)).abs() <= 1.0
                    && (b.x1 - f64::from(frame.max.x)).abs() <= 1.0
                    && b.y0 >= f64::from(frame.min.y) - 1.0
                    && b.y1 <= f64::from(frame.max.y)
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        let y = |node: &&accesskit::Node| node.bounds().map_or(0.0, |r| r.y0);
        y(a).total_cmp(&y(b))
    });
    assert!(
        rows.iter().any(|node| !player_row(node))
            && rows.iter().filter(|n| player_row(n)).count() > 5,
        "premise: a team row and its players are drawn"
    );
    for pair in rows.windows(2) {
        let [above, below] = [pair[0], pair[1]].map(|node| node.bounds().expect("bounds"));
        assert!(
            (below.y0 - above.y0 - 18.0).abs() <= 1.0,
            "{:?} starts 18 below {:?} ({below:?} vs {above:?})",
            pair[1].label(),
            pair[0].label()
        );
    }
    for row in &rows {
        let bounds = row.bounds().expect("bounds");
        assert!(
            (bounds.y1 - bounds.y0 - 18.0).abs() <= 1.0,
            "{:?} is 18 high ({bounds:?})",
            row.label()
        );
    }

    // The first player row's three columns, read off its painted galleys.
    let row = rows
        .iter()
        .find(|node| player_row(node))
        .expect("a player row");
    let label = row.label().expect("a player row's label");
    let mut words = label.splitn(3, ' ');
    let (place, position, name) = (
        words.next().expect("the place"),
        words.next().expect("the position"),
        words.next().expect("the name"),
    );
    let bounds = row.bounds().expect("bounds");
    let place_shape = text_in_rect(&shapes, place, bounds);
    let position_shape = text_in_rect(&shapes, position, bounds);
    let name_shape = text_in_rect(&shapes, name, bounds);
    assert_eq!(
        place_shape.galley.job.text, place,
        "the place is its own galley"
    );
    assert_eq!(
        position_shape.galley.job.text, position,
        "the position is its own galley"
    );
    let place_right = place_shape.pos.x + place_shape.galley.size().x;
    assert!(
        place_right < position_shape.pos.x,
        "the place ends before the position starts ({place_right} vs {})",
        position_shape.pos.x
    );
    assert!(
        (position_shape.pos.x - (place_right + 5.0)).abs() <= 1.0,
        "the position 5 after the place ({} vs {place_right})",
        position_shape.pos.x
    );
    assert!(
        (name_shape.pos.x - (position_shape.pos.x + 28.0 + 5.0)).abs() <= 1.0,
        "the name 5 after the position's 28-point column ({} vs {})",
        name_shape.pos.x,
        position_shape.pos.x
    );
    let weak = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .weak_text_color();
    assert_eq!(
        galley_color(place_shape),
        weak,
        "the place in the weak colour"
    );
}

// The list draws its text at the body size, the drafts' 12.5, so every
// position the list can show fits the 28-point column: on a DMF's row,
// the widest, the name stands 5 points after the column and after the
// position's last letter.
#[test]
fn every_position_fits_its_column_at_the_lists_font() {
    let path = fixture(PesVersion::Pes21, "list_dmf_row");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let dmf = u8::try_from(POSITIONS.iter().position(|label| *label == "DMF").unwrap()).unwrap();
    let player = file.player_mut(70101).unwrap();
    assert_ne!(player.positions.registered, dmf, "premise: 70101 is no DMF");
    player.positions.registered = dmf;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = row(&nodes, "01 DMF PLACEHOLDER").bounds().expect("bounds");
    let place = text_in_rect(&shapes, "01", bounds);
    let position = text_in_rect(&shapes, "DMF", bounds);
    let name = text_in_rect(&shapes, "PLACEHOLDER", bounds);

    // Every position laid out in the font the row draws its place in.
    let font = place.galley.job.sections[0].format.font_id.clone();
    let widths: Vec<(&str, f32)> = window.ctx.fonts_mut(|fonts| {
        POSITIONS
            .iter()
            .map(|label| {
                let galley =
                    fonts.layout_no_wrap((*label).to_owned(), font.clone(), egui::Color32::WHITE);
                (*label, galley.size().x)
            })
            .collect()
    });
    for (label, width) in widths {
        assert!(
            width <= 28.0,
            "{label} is {width} wide, past the 28-point column"
        );
    }
    assert_eq!(
        font,
        egui::TextStyle::Body.resolve(&window.ctx.style_of(window.ctx.theme())),
        "the row's text at the body size"
    );

    let column_end = position.pos.x + 28.0;
    assert!(
        (name.pos.x - (column_end + 5.0)).abs() <= 1.0,
        "the name 5 after the position's column ({} vs {column_end})",
        name.pos.x
    );
    let letters_end = position.pos.x + position.galley.size().x;
    assert!(
        name.pos.x - letters_end >= 5.0 - 0.5,
        "the name at least 5 after the position's letters ({} vs {letters_end})",
        name.pos.x
    );
}
