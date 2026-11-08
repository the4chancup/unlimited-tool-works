use super::*;

// SE-GUI-13
#[test]
fn the_panel_lists_the_differences_per_team_and_a_rows_click_selects() {
    let (mut view, mut window) = open_734_with("comparator_rows", &[73401], 183);
    let dir = scratch_of(&view);
    let picked = picked_save(&dir, "picked", |_| {});
    let pairs = open_comparator(&mut window, &mut view);
    assert!(
        pairs
            .iter()
            .any(|(_, node)| node.label() == Some("Comparator")),
        "the window's title"
    );
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Player 73401 - 49 - Muh Spooks");
    let nodes = comparator_nodes(&pairs);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Team 734 /lit/")),
        "734's heading"
    );
    assert!(
        nodes.iter().any(|node| {
            node.role() == Role::Button && node.label() == Some("Player 73401 - 49 - Muh Spooks")
        }),
        "73401's head"
    );
    let (_, row) = comparator_button(&pairs, "Height 185 \u{2192} 183");

    // The selector on another team: the click selects the player and
    // moves the selector to his, as Live AATF's row click does.
    view.team = TeamChoice::Team(718);
    view.selected = Some(Selection::Player(718, 0));
    click(&mut window, row, &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(734, 0)),
        "73401's team and slot"
    );
    assert_eq!(view.team, TeamChoice::Team(734), "the selector moved");
}

/// A diff row of a player the open save rosters (73401's height) says on
/// hover what its click does.
#[test]
fn a_rows_hover_says_its_click_switches_to_the_player() {
    let (mut view, mut window) = open_734_with("comparator_row_hover", &[73401], 183);
    let dir = scratch_of(&view);
    let picked = picked_save(&dir, "picked", |_| {});
    open_comparator(&mut window, &mut view);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Player 73401 - 49 - Muh Spooks");
    let (_, row) = comparator_button(&pairs, "Height 185 \u{2192} 183");
    let at = centre(row);
    assert_eq!(
        tooltip_containing(&mut window, &mut view, at, "this player").as_deref(),
        Some("Click to switch to this player")
    );
}

// A field's row is the draft's `.drow`: the label in the name's column
// the draft's `padding` (4 points) in, the `old → new` the draft's `gap`
// (8) past the column (4 + 190 + 8) in the strong colour; a player head
// keeps the row's left edge.
// SE-GUI-13
#[test]
fn a_row_stands_in_the_drows_two_columns() {
    let (mut view, mut window, _) = compared_73401("comparator_drow");
    scroll_to(&mut window, &mut view, "Player 73401 - 49 - Muh Spooks");
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    /// The painted text inside `bounds` whose galley starts with
    /// `starts_with`.
    fn text_in_row<'a>(
        shapes: &'a [egui::epaint::ClippedShape],
        bounds: accesskit::Rect,
        starts_with: &str,
    ) -> &'a egui::epaint::TextShape {
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape)
                    if shape.galley.job.text.starts_with(starts_with)
                        && f64::from(shape.pos.x) >= bounds.x0 - 0.5
                        && f64::from(shape.pos.x) <= bounds.x1 + 0.5
                        && f64::from(shape.pos.y) >= bounds.y0 - 0.5
                        && f64::from(shape.pos.y) <= bounds.y1 + 0.5 =>
                {
                    Some(shape)
                }
                _ => None,
            })
            .next()
            .unwrap_or_else(|| panic!("no painted {starts_with:?} inside {bounds:?}"))
    }
    let row = comparator_button(&pairs, "Height 185 \u{2192} 183")
        .1
        .bounds()
        .expect("the row's bounds");
    let label = text_in_row(&shapes, row, "Height");
    assert!(
        (f64::from(label.pos.x) - (row.x0 + 4.0)).abs() <= 0.5,
        "the label the row's 4-point inset in: {:?} vs {row:?}",
        label.pos
    );
    let values = text_in_row(&shapes, row, "185 \u{2192} 183");
    assert!(
        (f64::from(values.pos.x) - (row.x0 + 202.0)).abs() <= 0.5,
        "the values 4 + 190 + 8 in: {:?} vs {row:?}",
        values.pos
    );
    let strong = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .strong_text_color();
    assert_eq!(galley_color(values), strong, "the values' strong colour");
    let head_bounds = comparator_button(&pairs, "Player 73401 - 49 - Muh Spooks")
        .1
        .bounds()
        .expect("the head's bounds");
    let head = text_in_row(&shapes, head_bounds, "Player 73401 ");
    assert!(
        (f64::from(head.pos.x) - row.x0).abs() <= 0.5,
        "a head stays at the row's left edge: {:?} vs {row:?}",
        head.pos
    );
}

/// The colour, then the text, of each of `shape`'s painted runs.
fn painted_runs(shape: &egui::epaint::TextShape) -> Vec<(egui::Color32, String)> {
    shape
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
        .collect()
}

// A rostered player's head draws as the AATF lists' heads do: `Player
// <id> -` and the second ` - ` in the weak colour, the number and his
// name's runs in the strong text — a coded run in its code's colour, the
// codes themselves never drawn.
// SE-GUI-13
#[test]
fn a_players_head_draws_the_id_weak_the_number_strong_and_the_name_coded() {
    let dir = crate::test_support::scratch("comparator_head_runs");
    // 73401's height broken and his name given a colour code ahead of its
    // letters; the unedited fixture picked.
    let open_path = picked_save(&dir, "open", |file| {
        let player = file.player_mut(73401).expect("73401 is a player");
        player.basic.height = 183;
        player.name = format!("\x11cff0000ff{}", player.name);
    });
    let picked = picked_save(&dir, "picked", |_| {});
    let mut view = SaveView::new();
    view.open(&open_path);
    view.panel = Some(Panel::Comparator);
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    let raw = player_entry(&view, 73401).name;
    let spans = name_spans(&raw);
    assert!(
        spans.iter().any(|(color, _)| color.is_some()),
        "premise: 73401's name is coded"
    );
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let head = "Player 73401 - 49 - Muh Spooks";
    scroll_to(&mut window, &mut view, head);
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    comparator_button(&pairs, head);
    let (weak, strong) = {
        let visuals = &window.ctx.style_of(window.ctx.theme()).visuals;
        (visuals.weak_text_color(), visuals.strong_text_color())
    };
    let expected: Vec<(egui::Color32, String)> = [
        (weak, "Player 73401 -".to_owned()),
        (strong, " 49".to_owned()),
        (weak, " - ".to_owned()),
    ]
    .into_iter()
    .chain(spans.iter().map(|(color, text)| {
        (
            color.map_or(strong, |[red, green, blue]| {
                egui::Color32::from_rgb(red, green, blue)
            }),
            (*text).to_owned(),
        )
    }))
    .collect();
    assert_eq!(painted_runs(text_shape(&shapes, "Player 73401")), expected);
}

/// A team whose name holds a colour code heads its group by its display
/// name: 734's name in the open save coloured red.
// SE-GUI-13
#[test]
fn a_coded_teams_heading_reads_its_display_name() {
    let (mut view, mut window) = open_734_with("comparator_coded_heading", &[73401], 183);
    edit_team(view.session.as_mut().unwrap(), 734, 1, |team| {
        team.name = format!("\x11cff0000ff{}", team.name);
    });
    let raw = team_entry(&view, 734).name;
    let shown = display_name(&raw);
    assert_ne!(raw, shown, "premise: 734's name holds a colour code");
    let picked = picked_save(&scratch_of(&view), "picked", |_| {});
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    let pairs = compared(&mut window, &mut view);
    let headings: Vec<&str> = comparator_nodes(&pairs)
        .into_iter()
        .filter_map(|node| node.value())
        .filter(|value| value.starts_with("Team 734 "))
        .collect();
    assert_eq!(
        headings,
        [format!("Team 734 {shown}")],
        "734's heading by its display name"
    );
}

// SE-GUI-13
#[test]
fn a_motion_row_reads_as_the_view_shows_it() {
    let (mut view, mut window) = open_734_with("comparator_motion", &[], 183);
    let dir = scratch_of(&view);
    // The open save's 73401 free-kick motion stored 14 (the picked's
    // stored 0): the view shows one above the stored.
    let mut entry = player_entry(&view, 73401);
    assert_eq!(entry.motion.free_kick, 0, "premise: stored 0");
    entry.motion.free_kick = 14;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    let picked = picked_save(&dir, "picked", |_| {});
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Free kick 1 \u{2192} 15");
    comparator_button(&pairs, "Free kick 1 \u{2192} 15");
}

// SE-GUI-13
#[test]
fn the_scope_toggles_narrow_the_rows() {
    let (mut view, mut window) = open_734_with("comparator_scopes", &[73401], 183);
    let dir = scratch_of(&view);
    // A gameplay row (height, through `open_734_with`) and an aesthetics
    // one (73401's boots id, 0 in the fixture).
    let mut entry = player_entry(&view, 73401);
    assert_eq!(entry.appearance.boots_id, 0, "premise: no boots");
    entry.appearance.boots_id = 3601;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    let picked = picked_save(&dir, "picked", |_| {});
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Boots ID 0 \u{2192} 3601");
    comparator_button(&pairs, "Boots ID 0 \u{2192} 3601");
    comparator_button(&pairs, "Height 185 \u{2192} 183");

    let has = |pairs: &[(accesskit::NodeId, accesskit::Node)], label: &str| {
        comparator_nodes(pairs)
            .iter()
            .any(|node| node.role() == Role::Button && node.label() == Some(label))
    };
    // Gameplay: the height alone; Aesthetics: the boots alone. `compared`
    // waits for the new scope's result before the scroll, or the held
    // previous result's rows would read as the new scope's.
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Gameplay"),
        &mut view,
    );
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Height 185 \u{2192} 183");
    assert!(has(&pairs, "Height 185 \u{2192} 183"), "the gameplay row");
    assert!(
        !has(&pairs, "Boots ID 0 \u{2192} 3601"),
        "the aesthetics row narrowed out"
    );
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Aesthetics"),
        &mut view,
    );
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Boots ID 0 \u{2192} 3601");
    assert!(
        has(&pairs, "Boots ID 0 \u{2192} 3601"),
        "the aesthetics row"
    );
    assert!(
        !has(&pairs, "Height 185 \u{2192} 183"),
        "the gameplay row narrowed out"
    );
}

/// An edit after the first result marks the held list `Comparing…` and
/// starts no run until the gesture ends — the mark read on a held-pointer
/// frame, where no reply can land (`an_edit_rechecks_and_the_old_result_marks_checking`'s
/// premise).
#[test]
fn an_edit_marks_the_held_list_comparing() {
    let (mut view, mut window, _) = compared_73401("comparator_mark");
    scroll_to(&mut window, &mut view, "Height 185 \u{2192} 183");

    // The pointer held on the toolbar's empty end so the re-run waits for
    // the release: a started run could reply before the mark is read.
    let at = egui::pos2(1090.0, 12.0);
    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    // 73401 back to his fixture height: the saves are identical again.
    let mut entry = player_entry(&view, 73401);
    entry.basic.height = 185;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        view.comparator.comparison.pending.is_none(),
        "premise: no run started while the pointer is held"
    );
    let nodes = comparator_nodes(&pairs);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Comparing\u{2026}")),
        "the edit marks the held result"
    );
    assert!(
        nodes.iter().any(|node| {
            node.role() == Role::Button && node.label() == Some("Height 185 \u{2192} 183")
        }),
        "the stale row stays while comparing"
    );
    window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        view.comparator.comparison.pending.is_some(),
        "the run starts on the release frame"
    );
    let pairs = compared(&mut window, &mut view);
    let nodes = comparator_nodes(&pairs);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("No differences.")),
        "identical saves read empty"
    );
    assert!(
        !nodes.iter().any(|node| {
            node.role() == Role::Button
                && node
                    .label()
                    .is_some_and(|label| label.starts_with("Height"))
        }),
        "the old row is gone"
    );
}

// SE-GUI-13
#[test]
fn a_save_of_another_version_is_refused_naming_both() {
    let (mut view, mut window) = open_734_with("comparator_version", &[], 183);
    let dir = scratch_of(&view);
    let picked = picked_pes21(&dir);
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line
            == "The picked save is a PES 2021 save and the open save a PES 2017 one, but both must be of the same version"),
        "the refusal's literal text: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line == "(none)"),
        "the pick dropped: {lines:?}"
    );
    assert!(
        !lines
            .iter()
            .any(|line| line.starts_with("Player ") || line.starts_with("Team ")),
        "no rows: {lines:?}"
    );
    assert!(
        view.comparator.comparison.result.is_none() && view.comparator.comparison.pending.is_none(),
        "no comparison ran"
    );
}

// SE-GUI-13
#[test]
fn the_players_only_in_one_save_list_under_their_headings() {
    let (mut view, mut window) = open_734_with("comparator_only_in", &[], 183);
    let dir = scratch_of(&view);
    // As `tests/cli/diff.rs`'s `diff_pair`: one id renumbered out, so
    // 9_999_999 is in the picked save alone and 73410 in the open alone.
    let picked = picked_save(&dir, "picked", |file| {
        file.player_mut(73410).expect("73410 is a player").id = 9_999_999;
    });
    let slot = view
        .session
        .as_ref()
        .unwrap()
        .file()
        .team(734)
        .expect("team 734")
        .roster
        .iter()
        .position(|slot| slot.player_id == 73410)
        .map(|slot| u8::try_from(slot).expect("a roster slot"))
        .expect("premise: 73410 rostered");
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Only in the open save");
    let nodes = comparator_nodes(&pairs);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Only in the picked save")),
        "the picked-only heading"
    );
    // His record keeps the name; no roster holds him in the picked save.
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Player 9999999 DFW")),
        "the picked-only head"
    );
    assert!(
        !nodes.iter().any(|node| {
            node.role() == Role::Button
                && node.label().is_some_and(|label| label.contains("9999999"))
        }),
        "the picked-only head is no click target"
    );
    // The open-only head clicks as a row does (an AccessKit Click: the
    // scroll's animation keeps its bounds moving past the found frame, so
    // a pointer position can already be stale).
    view.selected = Some(Selection::Player(734, 0));
    action_click(
        &mut window,
        &mut view,
        comparator_button(&pairs, "Player 73410 - 46 - DFW").0,
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(734, slot)),
        "73410 selected"
    );
}

// SE-GUI-13
#[test]
fn the_rows_group_per_team_in_the_open_saves_order() {
    let dir = crate::test_support::scratch("comparator_groups");
    // 73410's place on team 734's roster given to 74301 (so 74301 is on
    // 734 and on 743, and 73410 on no roster), and a height each on
    // 73401, 73410, 74301 and 74302. The report's order is 73401, 73410,
    // 74301, 74302 (the fixture's record order).
    let open_path = picked_save(&dir, "open", |file| {
        for id in [73401u32, 73410, 74301, 74302] {
            file.player_mut(id).expect("player").basic.height = 183;
        }
        file.team_mut(734).expect("team 734").roster[9].player_id = 74301;
    });
    let picked = picked_save(&dir, "picked", |_| {});
    let mut view = SaveView::new();
    view.open(&open_path);
    view.panel = Some(Panel::Comparator);
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    let at = lines
        .iter()
        .position(|line| line == "Team 734 /lit/")
        .expect("734's heading");
    assert_eq!(
        &lines[at..at + 11],
        [
            "Team 734 /lit/",
            "Player 73401 - 49 - Muh Spooks",
            "Height 185 \u{2192} 183",
            // 74301 is on 734 and on 743: his first team is 734, and his
            // head wears his number on 734's slot.
            "Player 74301 - 46 - NOTHING EVER HAPPENS",
            "Height 189 \u{2192} 183",
            "Team 743 /pol/",
            "Player 74302 - 50 - STOP WATCHING PORN",
            "Height 185 \u{2192} 183",
            "No team",
            "Player 73410 DFW",
            "Height 180 \u{2192} 183",
        ],
        "one group per team in the open save's order: {lines:?}"
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("Team "))
            .count(),
        2,
        "each team headed once"
    );

    // A click on 74301's row (the Height button below his head) selects
    // him on 734; 73410's head and row are not buttons.
    let pairs = scroll_to(&mut window, &mut view, "Player 73410 DFW");
    let nodes = comparator_nodes(&pairs);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Player 73410 DFW")),
        "the no-team head is a label"
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|node| {
                node.role() == Role::Button
                    && node
                        .label()
                        .is_some_and(|label| label.starts_with("Height "))
            })
            .count(),
        3,
        "the three rostered rows are buttons"
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node.value() == Some("Height 180 \u{2192} 183"))
            .count(),
        1,
        "73410's row is not clickable"
    );
    view.team = TeamChoice::Team(718);
    view.selected = Some(Selection::Player(718, 0));
    // 74301 alone stands at 189 in the fixture: his row's label is his.
    action_click(
        &mut window,
        &mut view,
        comparator_button(&pairs, "Height 189 \u{2192} 183").0,
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(734, 9)),
        "74301 on 734's slot"
    );
    assert_eq!(view.team, TeamChoice::Team(734), "the selector moved");
}
