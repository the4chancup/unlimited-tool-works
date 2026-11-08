use super::*;

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

/// Asserts `shape`'s galley centres vertically on `bounds`' row within
/// half a point.
#[track_caller]
fn assert_centred_on_row(shape: &egui::epaint::TextShape, bounds: accesskit::Rect, what: &str) {
    let row_centre = (bounds.y0 + bounds.y1) / 2.0;
    let centre = f64::from(shape.pos.y + shape.galley.size().y / 2.0);
    assert!(
        (centre - row_centre).abs() <= 0.5,
        "{what} centres on the row ({centre} vs {row_centre})"
    );
}

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

// SE-GUI-46
#[test]
fn a_teams_name_in_the_filter_lists_its_squad_and_a_double_click_shows_it_alone() {
    let path = fixture(PesVersion::Pes17, "team_name_filter");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    let mut window = Headless::sized(DEFAULT_PANE.x, 1000.0);
    // The teams whose rows the frame lists, top to bottom.
    let listed_teams = |nodes: &[accesskit::Node], view: &SaveView| -> Vec<u32> {
        let file = view.session.as_ref().expect("a save is open").file();
        let mut rows: Vec<(f64, u32)> = file
            .teams()
            .iter()
            .filter_map(|team| {
                let label = format!("{} {}", team.id, display_name(&team.name));
                nodes
                    .iter()
                    .find(|node| node.role() == Role::Button && node.label() == Some(&label))
                    .map(|node| (node.bounds().expect("a row has bounds").y0, team.id))
            })
            .collect();
        rows.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("ordered bounds"));
        rows.into_iter().map(|(_, id)| id).collect()
    };
    let squad = team_entry(&view, 718)
        .roster
        .iter()
        .filter(|slot| slot.player_id != 0)
        .count();
    assert!(squad > 1, "premise: 718 has a squad to list");

    // Type 718's name in the filter box. No player of 718 holds it in his
    // name, shirt name or id; of the whole pool only 77206 does, team
    // 772's "PRESS /f/".
    let file = view.session.as_ref().expect("a save is open").file();
    let holding: Vec<u32> = file
        .players()
        .iter()
        .filter(|player| {
            display_name(&player.name).to_lowercase().contains("/f/")
                || player.shirt_name.to_lowercase().contains("/f/")
                || player.id.to_string().contains("/f/")
        })
        .map(|player| player.id)
        .collect();
    assert_eq!(holding, [77206], "premise: the players holding /f/");
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let filter_box = nodes
        .iter()
        .find(|node| {
            node.role() == Role::TextInput
                && node.placeholder() == Some("Filter every team (name, shirt name or id)")
        })
        .expect("the filter box");
    click(&mut window, filter_box, &mut view);
    window.frame(vec![egui::Event::Text("/f/".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });

    // 718's row heads every one of its players; 772's row and its
    // "PRESS /f/" follow.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(listed_teams(&nodes, &view), [718, 772], "the listed teams");
    let top_718 = team_row(&nodes, &view, 718).bounds().unwrap().y0;
    let top_772 = team_row(&nodes, &view, 772).bounds().unwrap().y0;
    let player_tops: Vec<f64> = nodes
        .iter()
        .filter(|node| player_row(node))
        .map(|node| node.bounds().unwrap().y0)
        .collect();
    assert_eq!(
        player_tops
            .iter()
            .filter(|top| (top_718..top_772).contains(*top))
            .count(),
        squad,
        "one row per rostered player of 718, under its row"
    );
    assert!(
        player_tops.iter().all(|top| *top > top_718),
        "no player row above 718's"
    );
    assert_eq!(player_tops.len(), squad + 1, "718's squad and 77206");
    assert_eq!(
        row_labels(&nodes).last().map(String::as_str),
        Some("06 CMF PRESS /f/"),
        "772's matching player under its row"
    );

    // A single click selects the row and leaves the selector and the
    // filter as they were.
    click(&mut window, team_row(&nodes, &view, 718), &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Team(718)),
        "the click selects"
    );
    assert_eq!(view.team, TeamChoice::All, "a click keeps ALL");
    assert_eq!(view.filter, "/f/", "a click keeps the filter");

    // Past egui's 0.3 s double-click window (a frame is 1/60 s), so the
    // next two clicks alone make the double click.
    for _ in 0..30 {
        draw(&mut window, &mut view);
    }
    let nodes = draw(&mut window, &mut view);
    let row_718 = team_row(&nodes, &view, 718);
    click(&mut window, row_718, &mut view);
    click(&mut window, row_718, &mut view);
    assert_eq!(view.team, TeamChoice::Team(718), "the selector shows 718");
    assert_eq!(view.filter, "", "the filter box is empty");
    assert_eq!(
        view.selected,
        Some(Selection::Team(718)),
        "718's row selected"
    );
    let nodes = draw(&mut window, &mut view);
    assert_eq!(listed_teams(&nodes, &view), [718], "718 alone");
    assert_eq!(
        nodes.iter().filter(|node| player_row(node)).count(),
        squad,
        "718's players"
    );

    // The team row's tooltip says what a double click does.
    let at = centre(team_row(&nodes, &view, 718));
    let tooltip = tooltip_containing(&mut window, &mut view, at, "Double-click")
        .expect("the team row's tooltip");
    assert!(
        tooltip.contains("Double-click to filter to this team"),
        "{tooltip:?}"
    );
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
        assert_centred_on_row(handle, row, "the handle");
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

mod chrome;
mod drag;
mod keys;
mod rows;
