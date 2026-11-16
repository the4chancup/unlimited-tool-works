use super::*;

/// `view`'s list rows as `Selection`s, the team selector and the filter
/// applied, for computing where a key move should land.
fn listed_selections(view: &SaveView) -> Vec<Selection> {
    selection::list(
        view.session.as_ref().unwrap().file(),
        &view.team,
        &view.filter,
        view.team_category,
    )
    .iter()
    .map(|row| match row {
        selection::ListRow::Team(id) => Selection::Team(*id),
        selection::ListRow::Player(row) => Selection::Player(row.team_id, row.slot),
    })
    .collect()
}

/// A PageDown (`forward`) or PageUp press as one frame of the view: the
/// frame's nodes and shapes.
fn page_key_press(
    window: &mut Headless,
    view: &mut SaveView,
    forward: bool,
) -> (Vec<accesskit::Node>, Vec<egui::epaint::ClippedShape>) {
    let (_, nodes, shapes) = window.frame(
        vec![key(
            if forward {
                egui::Key::PageDown
            } else {
                egui::Key::PageUp
            },
            egui::Modifiers::NONE,
            true,
        )],
        |ui| show(ui, view, &|| Vec::new()),
    );
    (nodes, shapes)
}

/// A window drawing one pass a frame: a scroll landing on a later pass
/// could borrow another view's `request_discard`, and the tests using it
/// read what the key's own frame draws.
fn one_pass_window() -> Headless {
    let window = Headless::new();
    window
        .ctx
        .options_mut(|options| options.max_passes = std::num::NonZeroUsize::MIN);
    window
}

// SE-GUI-53
#[test]
fn page_down_and_up_step_seven_rows_from_a_number_field() {
    // With one team listed, PageDown and PageUp step seven rows at a
    // time even while a number field holds the keyboard; the field's
    // gesture ends on the move.
    let path = fixture(PesVersion::Pes17, "page_keys_seven");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(718);
    view.selected = Some(Selection::Player(718, 0));
    let roster = team_entry(&view, 718).roster;
    assert_eq!(
        roster.iter().filter(|slot| slot.player_id != 0).count(),
        23,
        "premise: 718's roster fills 23 slots"
    );
    let first_height = player_entry(&view, roster[0].player_id).basic.height;
    let mut window = Headless::new();
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let height = node_id_right_of(&pairs, "Height", Role::SpinButton);
    action_focus(&mut window, &mut view, height);
    assert!(
        window.ctx.egui_wants_keyboard_input(),
        "premise: the Height field holds the keyboard"
    );
    // The list is 718's team row then its 23 players' rows: seven rows
    // from player 01's (the list's second) land on places 08, 15, 22 and
    // 23, the last row stopping the fourth press.
    let drawn = listed_selections(&view);
    let expected: Vec<Selection> = [Selection::Team(718)]
        .into_iter()
        .chain((0..23).map(|slot| Selection::Player(718, slot)))
        .collect();
    assert_eq!(drawn, expected, "premise: the list's rows as drawn");

    page_key_press(&mut window, &mut view, true);
    assert_eq!(
        view.selected,
        Some(Selection::Player(718, 7)),
        "PageDown moved seven rows, to place 08"
    );
    assert_eq!(
        player_entry(&view, roster[0].player_id).basic.height,
        first_height,
        "the key edited no field"
    );
    for slot in [14_u8, 21, 22] {
        page_key_press(&mut window, &mut view, true);
        assert_eq!(
            view.selected,
            Some(Selection::Player(718, slot)),
            "PageDown moved seven rows, to slot {:02}",
            slot + 1
        );
    }

    page_key_press(&mut window, &mut view, false);
    assert_eq!(
        view.selected,
        Some(Selection::Player(718, 15)),
        "PageUp moved seven rows back, to place 16"
    );
    page_key_press(&mut window, &mut view, false);
    assert_eq!(
        view.selected,
        Some(Selection::Player(718, 8)),
        "PageUp moved seven rows back, to place 09"
    );

    // From the list's second row, a page up lands on its first, the team
    // row.
    view.selected = Some(Selection::Player(718, 0));
    page_key_press(&mut window, &mut view, false);
    assert_eq!(
        view.selected,
        Some(Selection::Team(718)),
        "PageUp from player 01's row selected the team row, the list's first"
    );
}

// With one team listed and its rows overflowing the clip, a page step
// is `Nearest`: landing on a player the list already shows whole leaves
// the top row where it was; landing on one below the clip puts his row
// at the clip's bottom edge.
// SE-GUI-53
#[test]
fn a_one_teams_page_step_scrolls_only_as_far_as_it_needs() {
    let path = fixture(PesVersion::Pes17, "page_keys_one_team_nearest");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(718);
    view.selected = Some(Selection::Player(718, 0));
    let drawn = listed_selections(&view);
    assert_eq!(drawn.len(), 24, "premise: 718's row and its 23 players");
    let mut window = Headless::sized(DEFAULT_PANE.x, 400.0);
    window
        .ctx
        .options_mut(|options| options.max_passes = std::num::NonZeroUsize::MIN);

    // The rows drawn with the list at its top, top to bottom: the last
    // of them inside the clip is where a first `Nearest` step can land.
    // A first, settling frame comes before them: the window's status bar
    // takes its height once drawn, and the pane is deeper until it does.
    draw(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let selected = nodes
        .iter()
        .filter(|node| list_row(node))
        .find(|node| node.toggled() == Some(accesskit::Toggled::True))
        .and_then(accesskit::Node::bounds)
        .expect("the selected row's node");
    let clip = list_clip(&window, &shapes, selected);
    let mut rows: Vec<accesskit::Rect> = nodes
        .iter()
        .filter(|node| list_row(node))
        .filter_map(accesskit::Node::bounds)
        .collect();
    rows.sort_by(|a, b| a.y0.partial_cmp(&b.y0).expect("ordered bounds"));
    let last = rows
        .iter()
        .rposition(|bounds| bounds.y1 <= f64::from(clip.bottom()) - 1.0)
        .expect("a row the list shows whole");
    assert!(
        last > 7 && last + 7 < drawn.len(),
        "premise: a seven-row step lands inside the clip and the next below it ({last})"
    );
    let top_before = rows[0];
    view.selected = Some(drawn[last - 7]);

    // PgDn to a player the list shows whole: the top row keeps its place.
    let (nodes, _) = page_key_press(&mut window, &mut view, true);
    assert_eq!(
        view.selected,
        Some(drawn[last]),
        "premise: PgDn stepped seven rows"
    );
    let mut after: Vec<accesskit::Rect> = nodes
        .iter()
        .filter(|node| list_row(node))
        .filter_map(accesskit::Node::bounds)
        .collect();
    after.sort_by(|a, b| a.y0.partial_cmp(&b.y0).expect("ordered bounds"));
    assert!(
        (after[0].y0 - top_before.y0).abs() <= 0.5 && (after[0].y1 - top_before.y1).abs() <= 0.5,
        "the top row kept its place ({:?} vs {top_before:?})",
        after[0]
    );

    // PgDn to a player below the clip: his row stands at its bottom edge.
    let (nodes, shapes) = page_key_press(&mut window, &mut view, true);
    assert_eq!(
        view.selected,
        Some(drawn[last + 7]),
        "premise: PgDn stepped seven rows"
    );
    let bounds = nodes
        .iter()
        .filter(|node| list_row(node))
        .find(|node| node.toggled() == Some(accesskit::Toggled::True))
        .and_then(accesskit::Node::bounds)
        .expect("the selected row's node");
    let clip = list_clip(&window, &shapes, bounds);
    assert!(
        (bounds.y1 - f64::from(clip.bottom())).abs() <= 1.0,
        "his row at the clip's bottom edge ({bounds:?} vs {clip:?})"
    );
}

/// PES 17's team 734 listed with roster `slot`'s player selected and the
/// keyboard in his Height field, the window's first frame drawn.
fn page_keys_from_height(test: &str, slot: u8) -> (SaveView, Headless) {
    let path = fixture(PesVersion::Pes17, test);
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(734);
    view.selected = Some(Selection::Player(734, slot));
    let mut window = Headless::new();
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let height = node_id_right_of(&pairs, "Height", Role::SpinButton);
    action_focus(&mut window, &mut view, height);
    assert!(
        window.ctx.egui_wants_keyboard_input(),
        "premise: the Height field holds the keyboard"
    );
    (view, window)
}

// A page key that moves nothing (PageDown on the last row) leaves the
// field its keyboard: the member types on where he was.
// SE-GUI-53
#[test]
fn a_page_key_that_moves_nothing_keeps_the_fields_keyboard() {
    // 734's roster fills its 23 slots, so slot 22's row is the list's last.
    let (mut view, mut window) = page_keys_from_height("page_keys_dead_end", 22);
    window.frame(
        vec![key(egui::Key::PageDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(view.selected, Some(Selection::Player(734, 22)), "no move");
    assert!(
        window.ctx.egui_wants_keyboard_input(),
        "the Height field keeps the keyboard"
    );
}

// A press on something not focusable — a field's label — while a field
// holds the keyboard ends the field's focus on the press frame, not the
// click's: a drag elsewhere would keep the field's undo entries merging
// and Ctrl+Z swallowed were focus held to the release (the editor's
// surrender-on-press).
#[test]
fn a_press_elsewhere_takes_the_fields_keyboard() {
    let (mut view, mut window) = page_keys_from_height("focus_surrender_press", 0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let label = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some("Height"))
        .expect("the Height label");
    let at = centre(label);
    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        window.ctx.memory(|memory| memory.focused()).is_none(),
        "the press surrendered the field's keyboard"
    );
    window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        window.ctx.memory(|memory| memory.focused()).is_none(),
        "the release took no focus back"
    );
}

// While a pointer button is held (a number being dragged, a button
// pressed), the page keys do nothing: the gesture in flight would
// otherwise finish on the next player.
#[test]
fn the_page_keys_wait_while_a_pointer_button_is_held() {
    let (mut view, mut window) = page_keys_from_height("page_keys_pointer_held", 0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let first = nodes
        .iter()
        .filter(|node| player_row(node))
        .find(|node| node.toggled() == Some(accesskit::Toggled::True))
        .expect("the selected row");
    window.frame(pointer_at(centre(first), true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::PageDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(734, 0)),
        "PageDown with the button held moved nothing"
    );
}

// The frame a button is released on still holds the page keys off: a
// click completes on it, and would act on the player the key stepped to.
// The press is on the Height label, which takes no click, so only the
// page keys' own gate can keep the selection.
#[test]
fn the_page_keys_wait_on_the_frame_a_pointer_button_is_released() {
    let (mut view, mut window) = page_keys_from_height("page_keys_pointer_released", 0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let label = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some("Height"))
        .expect("the Height label");
    let at = centre(label);
    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let mut release = pointer_at(at, false);
    release.push(key(egui::Key::PageDown, egui::Modifiers::NONE, true));
    window.frame(release, |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        view.selected,
        Some(Selection::Player(734, 0)),
        "PageDown on the release frame moved nothing"
    );
}

// Walked past the rows in view, the list scrolls with the selection: the
// selected row stays drawn inside the list's frame.
#[test]
fn the_list_scrolls_to_keep_a_paged_selection_in_view() {
    let path = fixture(PesVersion::Pes21, "page_keys_scroll");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let in_view = nodes.iter().filter(|node| player_row(node)).count();
    for _ in 0..in_view + 10 {
        window.frame(
            vec![key(egui::Key::PageDown, egui::Modifiers::NONE, true)],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
    }
    assert_ne!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "premise: the selection moved"
    );
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let frame = list_frame(&window, &shapes);
    // The walk lands on team rows: any of the list's Buttons (a row's
    // reach ends at the frame's right edge).
    let selected = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && node
                    .bounds()
                    .is_some_and(|bounds| bounds.x0 < f64::from(frame.max.x))
        })
        .find(|node| node.toggled() == Some(accesskit::Toggled::True))
        .expect("the selected row is drawn");
    let bounds = selected.bounds().expect("the row's bounds");
    assert!(
        bounds.y0 >= f64::from(frame.min.y) - 0.5 && bounds.y1 <= f64::from(frame.max.y) + 0.5,
        "the selected row inside the list: {bounds:?} vs {frame:?}"
    );
}

// A team row the page keys select stands at the list's top edge on the
// press's own frame, so its players fill the list under it: the scroll
// lands with the selection it follows.
// SE-GUI-103
#[test]
fn a_team_row_the_page_keys_select_scrolls_to_the_lists_top() {
    let path = fixture(PesVersion::Pes21, "page_keys_top");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    let drawn = listed_selections(&view);
    let teams: Vec<Selection> = drawn
        .iter()
        .copied()
        .filter(|row| matches!(row, Selection::Team(_)))
        .collect();
    let (first, second) = (teams[0], teams[1]);
    let (Selection::Team(first_id), Selection::Team(second_id)) = (first, second) else {
        unreachable!("teams holds team rows")
    };
    view.selected = Some(first);
    let mut window = one_pass_window();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // The list stands at its top, the first team's row at the clip's edge.
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = team_row(&nodes, &view, first_id).bounds().expect("bounds");
    let clip = list_clip(&window, &shapes, bounds);
    assert!(
        (bounds.y0 - f64::from(clip.top())).abs() <= 1.0,
        "premise: the list is at its top ({bounds:?} vs {clip:?})"
    );

    let (nodes, shapes) = page_key_press(&mut window, &mut view, true);
    assert_eq!(
        view.selected,
        Some(second),
        "premise: PgDn selected the second team's row"
    );
    let bounds = team_row(&nodes, &view, second_id)
        .bounds()
        .expect("the second team's row is drawn");
    let clip = list_clip(&window, &shapes, bounds);
    assert!(
        (bounds.y0 - f64::from(clip.top())).abs() <= 1.0,
        "the second team's row at the list's top on the press's frame ({bounds:?} vs {clip:?})"
    );

    let (nodes, shapes) = page_key_press(&mut window, &mut view, false);
    assert_eq!(
        view.selected,
        Some(first),
        "premise: PgUp selected the first team's row again"
    );
    let bounds = team_row(&nodes, &view, first_id)
        .bounds()
        .expect("the first team's row is drawn");
    let clip = list_clip(&window, &shapes, bounds);
    assert!(
        (bounds.y0 - f64::from(clip.top())).abs() <= 1.0,
        "the first team's row back at the list's top on the press's frame ({bounds:?} vs {clip:?})"
    );
}

// PgDn held over the team rows: every press's own frame draws its
// selected row at the list's top edge; a scroll landing a frame late
// would flicker the rows between their old and new places. The last
// team's row stands as high as the list's end lets it.
// SE-GUI-103
#[test]
fn a_held_pgdn_draws_each_selected_team_row_at_the_lists_top() {
    let path = fixture(PesVersion::Pes21, "page_keys_held");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    let drawn = listed_selections(&view);
    let teams: Vec<Selection> = drawn
        .iter()
        .copied()
        .filter(|row| matches!(row, Selection::Team(_)))
        .collect();
    assert!(
        teams.len() > 11,
        "premise: eleven team rows or more listed ({})",
        teams.len()
    );
    view.selected = Some(teams[0]);
    let mut window = one_pass_window();
    draw(&mut window, &mut view);

    for (press, team) in teams.iter().enumerate().skip(1).take(10) {
        let (nodes, shapes) = page_key_press(&mut window, &mut view, true);
        assert_eq!(
            view.selected,
            Some(*team),
            "press {press}: the next team's row selected"
        );
        let Selection::Team(team_id) = team else {
            unreachable!("teams holds team rows")
        };
        let bounds = team_row(&nodes, &view, *team_id)
            .bounds()
            .expect("the selected team's row is drawn");
        let clip = list_clip(&window, &shapes, bounds);
        assert!(
            (bounds.y0 - f64::from(clip.top())).abs() <= 1.0,
            "press {press}: the selected team's row at the list's top ({bounds:?} vs {clip:?})"
        );
    }

    // The last team's row: the list scrolls to its end, the row as high
    // as that leaves it. The end shows as the last listed row's bottom
    // edge at the clip's.
    let Selection::Team(last_id) = teams.last().copied().expect("a last team") else {
        unreachable!("teams holds team rows")
    };
    view.selected = Some(teams[teams.len() - 2]);
    let (nodes, shapes) = page_key_press(&mut window, &mut view, true);
    assert_eq!(
        view.selected,
        Some(Selection::Team(last_id)),
        "premise: PgDn selected the last team's row"
    );
    let bounds = team_row(&nodes, &view, last_id)
        .bounds()
        .expect("the last team's row is drawn");
    let clip = list_clip(&window, &shapes, bounds);
    let bottom = nodes
        .iter()
        .filter(|node| list_row(node))
        .filter_map(accesskit::Node::bounds)
        .filter(|bounds| {
            bounds.y0 >= f64::from(clip.top()) - 1.0 && bounds.y1 <= f64::from(clip.bottom()) + 1.0
        })
        .map(|bounds| bounds.y1)
        .fold(f64::MIN, f64::max);
    assert!(
        (bottom - f64::from(clip.bottom())).abs() <= 1.0,
        "the list scrolled to its end ({bottom} vs {:?})",
        clip.bottom()
    );
    assert!(
        bounds.y0 >= f64::from(clip.top()) - 1.0 && bounds.y1 <= f64::from(clip.bottom()) + 1.0,
        "the last team's row fully in view ({bounds:?} vs {clip:?})"
    );
}

// Down on the last row the list shows whole selects the next, whose row
// the press's own frame draws whole at the list's bottom edge.
// SE-GUI-103
#[test]
fn down_on_the_last_shown_row_draws_the_next_whole_at_the_lists_bottom_edge() {
    let path = fixture(PesVersion::Pes21, "down_last_shown");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    view.selected = Some(Selection::Player(701, 0));
    let drawn = listed_selections(&view);
    let mut window = one_pass_window();
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let clip = list_clip(
        &window,
        &shapes,
        nodes
            .iter()
            .filter(|node| player_row(node))
            .find(|node| node.toggled() == Some(accesskit::Toggled::True))
            .and_then(accesskit::Node::bounds)
            .expect("the selected row's node"),
    );
    // The list's drawn rows in screen order, their places the rows'
    // indices in `drawn`, the list standing at its top: the last wholly
    // inside the clip is the last the list shows whole.
    let mut rows: Vec<accesskit::Rect> = nodes
        .iter()
        .filter(|node| list_row(node))
        .filter_map(accesskit::Node::bounds)
        .collect();
    rows.sort_by(|a, b| a.y0.partial_cmp(&b.y0).expect("ordered bounds"));
    let last = rows
        .iter()
        .rposition(|bounds| bounds.y1 <= f64::from(clip.bottom()) + 1.0)
        .expect("a row the list shows whole");
    view.selected = Some(drawn[last]);

    let (_, nodes, shapes) = window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(drawn[last + 1]),
        "premise: Down selected the next row"
    );
    let bounds = nodes
        .iter()
        .filter(|node| list_row(node))
        .find(|node| node.toggled() == Some(accesskit::Toggled::True))
        .and_then(accesskit::Node::bounds)
        .expect("the selected row's node");
    // The press frame's own clip: the list's height follows the quick
    // actions' panel, which the selection's change can resize.
    let press_clip = list_clip(&window, &shapes, bounds);
    assert!(
        bounds.y0 >= f64::from(press_clip.top()) - 1.0,
        "the next row shown whole ({bounds:?} vs {press_clip:?})"
    );
    assert!(
        (bounds.y1 - f64::from(press_clip.bottom())).abs() <= 1.0,
        "the next row at the list's bottom edge ({bounds:?} vs {press_clip:?})"
    );
}

// `Nearest` from a scrolled list: Down on a row the list shows whole
// keeps the offset, and Up from the top shown row brings the row above
// it to the clip's top.
#[test]
fn the_arrowed_selection_scrolls_only_as_far_as_it_needs() {
    let path = fixture(PesVersion::Pes21, "arrow_nearest_offset");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    view.selected = Some(Selection::Player(701, 0));
    let mut window = one_pass_window();
    let drawn = listed_selections(&view);

    // Wheel the list down until its first row is out of the clip.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let at = centre(
        nodes
            .iter()
            .find(|node| player_row(node))
            .expect("a player row"),
    );
    for _ in 0..64 {
        notch(&mut window, &mut view, at, false);
        let nodes = draw(&mut window, &mut view);
        if !nodes
            .iter()
            .any(|node| node.role() == Role::Button && node.label() == Some("701 /3/"))
        {
            break;
        }
    }
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        !nodes
            .iter()
            .any(|node| node.role() == Role::Button && node.label() == Some("701 /3/")),
        "premise: the list scrolled off its top"
    );
    // The list's rows drawn, top to bottom, and a click on `bounds`'
    // centre.
    let rows_in_view = |nodes: &[accesskit::Node]| -> Vec<accesskit::Rect> {
        let mut rows: Vec<accesskit::Rect> = nodes
            .iter()
            .filter(|node| list_row(node))
            .filter_map(accesskit::Node::bounds)
            .collect();
        rows.sort_by(|a, b| a.y0.partial_cmp(&b.y0).expect("ordered bounds"));
        rows
    };
    let click_row = |window: &mut Headless, view: &mut SaveView, bounds: accesskit::Rect| {
        press_release(
            window,
            egui::pos2(
                ((bounds.x0 + bounds.x1) / 2.0) as f32,
                ((bounds.y0 + bounds.y1) / 2.0) as f32,
            ),
            |ui| show(ui, view, &|| Vec::new()),
        );
    };
    let frame = list_frame(&window, &shapes);
    let rows = rows_in_view(&nodes);
    let top_before = rows.first().copied().expect("a top row");

    // Down on the row nearest the clip's middle keeps the offset: the
    // next row was in view already, so the list moves nothing.
    let middle = rows
        .iter()
        .copied()
        .min_by(|a, b| {
            let mid = |bounds: accesskit::Rect| {
                ((bounds.y0 + bounds.y1) / 2.0 - f64::from(frame.center().y)).abs()
            };
            mid(*a).partial_cmp(&mid(*b)).expect("ordered")
        })
        .expect("a middle row");
    click_row(&mut window, &mut view, middle);
    let before = view.selected.expect("the click selected the middle row");
    let index = drawn
        .iter()
        .position(|row| *row == before)
        .expect("the clicked row is listed");
    let (_, nodes, _) = window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(view.selected, Some(drawn[index + 1]), "Down moved one row");
    let top_after = rows_in_view(&nodes).first().copied().expect("a top row");
    assert!(
        (top_after.y0 - top_before.y0).abs() <= 0.5 && (top_after.y1 - top_before.y1).abs() <= 0.5,
        "the offset kept ({top_after:?} vs {top_before:?})"
    );

    // Up from the row shown highest brings the row above it to the
    // clip's top.
    let top_shown = rows_in_view(&nodes)
        .into_iter()
        .find(|bounds| bounds.y0 >= f64::from(frame.min.y) - 1.0)
        .expect("a row shown whole");
    click_row(&mut window, &mut view, top_shown);
    let top = view.selected.expect("the click selected the top row");
    let top_index = drawn
        .iter()
        .position(|row| *row == top)
        .expect("the clicked row is listed");
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let clip = list_clip(&window, &shapes, top_shown);
    let (_, nodes, _) = window.frame(
        vec![key(egui::Key::ArrowUp, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(drawn[top_index - 1]),
        "Up moved to the row above"
    );
    let previous = nodes
        .iter()
        .filter(|node| list_row(node))
        .find(|node| node.toggled() == Some(accesskit::Toggled::True))
        .and_then(accesskit::Node::bounds)
        .expect("the selected row is drawn");
    assert!(
        (previous.y0 - f64::from(clip.top())).abs() <= 1.0,
        "the row above at the clip's top ({previous:?} vs {clip:?})"
    );
}

// SE-GUI-52
#[test]
fn the_arrows_walk_the_lists_rows_across_the_team_rows() {
    // Down on team 701's last player row selects team 702's row, the next
    // row in the list as drawn; Up selects the player again.
    let path = fixture(PesVersion::Pes17, "arrow_keys_walk");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    let rows = selection::list(
        view.session.as_ref().unwrap().file(),
        &TeamChoice::All,
        "",
        TeamCategory::All,
    );
    let last = rows
        .iter()
        .rposition(|row| matches!(row, selection::ListRow::Player(row) if row.team_id == 701))
        .expect("a player row of team 701");
    let selection::ListRow::Player(player) = rows[last] else {
        unreachable!("rposition matched a player row")
    };
    assert_eq!(
        rows.get(last + 1),
        Some(&selection::ListRow::Team(702)),
        "premise: team 702's row follows 701's last player row"
    );
    view.selected = Some(Selection::Player(701, player.slot));
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        window.ctx.memory(|memory| memory.focused()).is_none(),
        "premise: no widget holds the keyboard"
    );

    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(Selection::Team(702)),
        "Down selected team 702's row"
    );
    window.frame(
        vec![key(egui::Key::ArrowUp, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, player.slot)),
        "Up selected 701's last player again"
    );
}

// SE-GUI-52
#[test]
fn up_on_the_lists_first_row_moves_nothing() {
    // 701's team row is the list's first: Up stops there.
    let path = fixture(PesVersion::Pes17, "arrow_keys_first_row");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    let rows = selection::list(
        view.session.as_ref().unwrap().file(),
        &TeamChoice::All,
        "",
        TeamCategory::All,
    );
    assert_eq!(
        rows.first(),
        Some(&selection::ListRow::Team(701)),
        "premise: 701's team row is the list's first"
    );
    view.selected = Some(Selection::Team(701));
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    window.frame(
        vec![key(egui::Key::ArrowUp, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(Selection::Team(701)),
        "Up on the first row moved nothing"
    );
}

// SE-GUI-52
#[test]
fn down_on_the_lists_last_row_moves_nothing() {
    // All teams' last row, a player of the last team: Down stops there, and
    // the step itself reports no move.
    let path = fixture(PesVersion::Pes17, "arrow_keys_last_row");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    let rows = selection::list(
        view.session.as_ref().unwrap().file(),
        &TeamChoice::All,
        "",
        TeamCategory::All,
    );
    let selection::ListRow::Player(player) = rows.last().expect("a last row") else {
        panic!("premise: the last row is a player row")
    };
    let last_selection = Selection::Player(player.team_id, player.slot);
    view.selected = Some(last_selection);
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(last_selection),
        "Down on the last row moved nothing"
    );
    assert!(
        !player_list::step_selection(&mut view, true, 1),
        "the step on the last row reports no move"
    );
    assert_eq!(view.selected, Some(last_selection));
}

// SE-GUI-52
#[test]
fn the_arrows_wait_while_a_field_holds_the_keyboard() {
    // A number field editing under the keyboard keeps the key: Down steps
    // its value, and the selection stays.
    let path = fixture(PesVersion::Pes17, "arrow_keys_focused");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(734);
    view.selected = Some(Selection::Player(734, 0));
    let mut window = Headless::new();
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let height_node = node_id_right_of(&pairs, "Height", Role::SpinButton);
    // A click puts the field in edit mode: its text takes the keyboard.
    action_click(&mut window, &mut view, height_node);
    assert!(
        window.ctx.egui_wants_keyboard_input(),
        "premise: the Height field holds the keyboard"
    );
    let player_id = team_entry(&view, 734).roster[0].player_id;
    let height = player_entry(&view, player_id).basic.height;
    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(734, 0)),
        "Down with the Height field holding the keyboard moved nothing"
    );
    assert_eq!(
        player_entry(&view, player_id).basic.height,
        height - 1,
        "the field kept the key and stepped"
    );
    assert!(
        window.ctx.egui_wants_keyboard_input(),
        "the Height field keeps the keyboard"
    );
}

// SE-GUI-52
#[test]
fn an_arrow_that_moves_the_selection_grabs_no_widget_focus() {
    // egui's own arrow navigation between widgets must not grab one on a
    // press that moved the selection.
    let path = fixture(PesVersion::Pes17, "arrow_keys_no_focus");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    view.selected = Some(Selection::Team(701));
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_ne!(
        view.selected,
        Some(Selection::Team(701)),
        "premise: the press moved the selection"
    );
    assert_eq!(
        window.ctx.memory(|memory| memory.focused()),
        None,
        "the arrow grabbed no widget's focus"
    );
}

// SE-GUI-52
#[test]
fn the_list_scrolls_to_keep_an_arrowed_selection_in_view() {
    // Walked past the rows in view, the list scrolls with the selection:
    // the selected row stays drawn inside the list's frame.
    let path = fixture(PesVersion::Pes21, "arrow_keys_scroll");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let in_view = nodes.iter().filter(|node| player_row(node)).count();
    for _ in 0..in_view + 10 {
        window.frame(
            vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
    }
    assert_ne!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "premise: the selection moved"
    );
    // Only the list's player rows are searched: the player view beside
    // the list draws toggled buttons of its own (the playable positions)
    // within the list's height, which would pass with the row not drawn.
    assert!(
        matches!(view.selected, Some(Selection::Player(..))),
        "premise: the walk ended on a player row: {:?}",
        view.selected
    );
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let frame = list_frame(&window, &shapes);
    let selected = nodes
        .iter()
        .filter(|node| player_row(node))
        .find(|node| node.toggled() == Some(accesskit::Toggled::True))
        .expect("the selected row is drawn");
    let bounds = selected.bounds().expect("the row's bounds");
    assert!(
        bounds.y0 >= f64::from(frame.min.y) - 0.5 && bounds.y1 <= f64::from(frame.max.y) + 0.5,
        "the selected row inside the list: {bounds:?} vs {frame:?}"
    );
}

// SE-GUI-52
#[test]
fn the_arrows_wait_while_the_unsaved_changes_list_is_open() {
    // The unsaved-changes popup is the view's own state, so the arrows'
    // gate reads it there: egui's popup registry never saw it open.
    let path = fixture(PesVersion::Pes17, "arrow_keys_changes_list");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    view.selected = Some(Selection::Team(701));
    let mut window = Headless::new();

    // The same press with the list closed selects 701's first player row.
    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "premise: Down moves with the list closed"
    );
    view.selected = Some(Selection::Team(701));

    // An edit, so the `• unsaved changes` label draws; a click opens its
    // list under it.
    let mut edited = player_entry(&view, team_entry(&view, 701).roster[0].player_id);
    edited.name = "EDITED".to_owned();
    view.session.as_mut().unwrap().edit_players(vec![edited]);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let label = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some("\u{2022} unsaved changes"))
        .expect("the unsaved-changes label");
    click(&mut window, label, &mut view);
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(view.changes_list.is_some(), "premise: the list is open");

    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(Selection::Team(701)),
        "Down with the changes list open moved nothing"
    );
    assert!(view.changes_list.is_some(), "the list stays open");
}

// SE-GUI-52
#[test]
fn the_arrows_wait_while_a_dialog_is_open() {
    // The Set all stats dialog up: the modal gate the page keys already
    // had. (The Copy/swap stats window is deliberately NOT this test's
    // dialog — it is not a modal: the arrows move the selection under
    // it, which is how it picks each destination.)
    let path = fixture(PesVersion::Pes17, "arrow_keys_dialog");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::new();

    // The same press with no dialog selects the next player row.
    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 1)),
        "premise: Down moves with no dialog"
    );
    view.selected = Some(Selection::Player(701, 0));

    // The Player menu's Set all stats opens the dialog and leaves it up.
    let nodes = open_menu(&mut window, &mut view, "Player", "Set all stats");
    click(&mut window, menu_item(&nodes, "Set all stats"), &mut view);
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(view.stat_num.is_some(), "premise: the dialog is open");

    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "Down with the dialog open moved nothing"
    );
    assert!(view.stat_num.is_some(), "the dialog stays open");
}

// SE-GUI-52
#[test]
fn the_arrows_wait_while_a_pointer_button_is_held() {
    // As the page keys', a held button holds the arrows: the press still in
    // flight would finish on the player the key steps to.
    let path = fixture(PesVersion::Pes17, "arrow_keys_pointer_held");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    view.selected = Some(Selection::Team(701));
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // The topmost: the frame lists its nodes in no screen order.
    let first = nodes
        .iter()
        .filter(|node| player_row(node))
        .min_by(|a, b| {
            let y = |node: &&accesskit::Node| node.bounds().map_or(f64::MAX, |r| r.y0);
            y(a).partial_cmp(&y(b)).expect("ordered bounds")
        })
        .expect("the list's first player row");
    window.frame(pointer_at(centre(first), true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(Selection::Team(701)),
        "Down with the button held moved nothing"
    );

    // The release's click selects the row it lands on, and the press after
    // it walks on again.
    window.frame(pointer_at(centre(first), false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "premise: the release's click selected the row"
    );
    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 1)),
        "Down after the release moves one row"
    );
}

// SE-GUI-52
#[test]
fn the_arrows_wait_while_the_slot_popover_is_open() {
    // The tactics' slot popover is the view's own state like the changes
    // list: egui's popup registry never saw it open, so the gate reads it.
    let path = fixture(PesVersion::Pes21, "arrow_keys_popover");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    let mut window = Headless::sized(DEFAULT_PANE.x, 624.0);
    open_team_view(&mut window, &mut view, 701);

    // The same press with no popover selects 701's first player row.
    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "premise: Down moves with no popover"
    );
    view.selected = Some(Selection::Team(701));

    // Open the slot popover on an outfield marker a click can reach.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let canvas = canvas_of(&nodes);
    let players = team_entry(&view, 701).tactics.presets[0].formations[0].players;
    let at = (0..11)
        .filter(|&index| players[index].position != 0)
        .find_map(|index| marker_point(canvas, &players, index))
        .expect("a marker a click can reach");
    click_at(&mut window, at, &mut view);
    assert!(
        view.tactics.popover_area().is_some(),
        "premise: the slot popover is open"
    );

    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(Selection::Team(701)),
        "Down with the slot popover open moved nothing"
    );
}

// SE-GUI-52
#[test]
fn the_arrows_walk_again_once_pgdn_left_the_slot_popover() {
    // The popover's own outside-press rule runs only while the pitch
    // draws, and the page keys are not gated by it: PgDn moves the
    // selection to a player row without a press the popover sees, so the
    // player view's draw closes the popover for it — else its stale gate
    // would hold the arrows with nothing on screen.
    let path = fixture(PesVersion::Pes21, "arrow_keys_popover_pgdn");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::sized(DEFAULT_PANE.x, 624.0);
    open_team_view(&mut window, &mut view, 701);

    // Open the slot popover on an outfield marker a click can reach.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let canvas = canvas_of(&nodes);
    let players = team_entry(&view, 701).tactics.presets[0].formations[0].players;
    let at = (0..11)
        .filter(|&index| players[index].position != 0)
        .find_map(|index| marker_point(canvas, &players, index))
        .expect("a marker a click can reach");
    click_at(&mut window, at, &mut view);
    assert!(
        view.tactics.popover_area().is_some(),
        "premise: the slot popover is open"
    );

    // One team listed, so PgDn steps seven rows: 701's own row to a
    // player row, and the popover goes with the team view.
    page_key_press(&mut window, &mut view, true);
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 6)),
        "premise: PgDn selected a player row"
    );
    assert!(
        view.tactics.popover_area().is_none(),
        "the player view took over with the slot popover still set"
    );

    window.frame(
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 7)),
        "Down moved one row from the paged player"
    );
}

// SE-GUI-53
#[test]
fn the_page_keys_walk_the_team_rows_with_several_listed() {
    // With two team rows or more listed (All teams lists every team's), a
    // press walks the team rows: PgDn the next one's, PgUp a player's own
    // then the one above; the list's first row and the last team row
    // stop them, whatever the window's height.
    let path = fixture(PesVersion::Pes17, "page_keys_teams");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    view.selected = Some(Selection::Player(718, 4));
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y - 9.0);
    let drawn = listed_selections(&view);
    let team_rows: Vec<usize> = drawn
        .iter()
        .enumerate()
        .filter_map(|(index, row)| matches!(row, Selection::Team(_)).then_some(index))
        .collect();
    let at = drawn
        .iter()
        .position(|row| *row == Selection::Player(718, 4))
        .expect("premise: 718's player 05 is listed");
    let after: Vec<usize> = team_rows
        .iter()
        .copied()
        .filter(|index| *index > at)
        .collect();
    assert!(
        after.len() >= 2,
        "premise: two team rows follow 718's player 05"
    );
    let own = team_rows
        .iter()
        .copied()
        .rev()
        .find(|index| *index < at)
        .expect("premise: a team row heads 718's player 05");
    assert_eq!(
        drawn[own],
        Selection::Team(718),
        "premise: the row heading 718's player 05 is 718's"
    );
    let above = team_rows
        .iter()
        .copied()
        .rev()
        .find(|index| *index < own)
        .expect("premise: a team row stands above 718's");
    assert!(
        matches!(drawn.first(), Some(Selection::Team(_))),
        "premise: the list's first row is a team row"
    );
    draw(&mut window, &mut view);

    page_key_press(&mut window, &mut view, true);
    assert_eq!(
        view.selected,
        Some(drawn[after[0]]),
        "PgDn selected the next team row"
    );
    page_key_press(&mut window, &mut view, true);
    assert_eq!(
        view.selected,
        Some(drawn[after[1]]),
        "PgDn again selected the one after it"
    );

    // The last team row stops PgDn; on a player of that last team there
    // is no team row to step to either.
    let last_team = *team_rows.last().unwrap();
    let Selection::Team(last_id) = drawn[last_team] else {
        unreachable!("team_rows holds team rows' indices")
    };
    view.selected = Some(drawn[last_team]);
    page_key_press(&mut window, &mut view, true);
    assert_eq!(
        view.selected,
        Some(drawn[last_team]),
        "PgDn on the last team row moved nothing"
    );
    let last_player = drawn[last_team + 1];
    assert!(
        matches!(last_player, Selection::Player(team, _) if team == last_id),
        "premise: the row under the last team row is one of its players"
    );
    view.selected = Some(last_player);
    page_key_press(&mut window, &mut view, true);
    assert_eq!(
        view.selected,
        Some(last_player),
        "PgDn with no team row past the selection moved nothing"
    );

    // PgUp on a player selects his own team's row first, then the one
    // above it.
    view.selected = Some(Selection::Player(718, 4));
    page_key_press(&mut window, &mut view, false);
    assert_eq!(
        view.selected,
        Some(Selection::Team(718)),
        "PgUp on a player selected his own team's row"
    );
    page_key_press(&mut window, &mut view, false);
    assert_eq!(
        view.selected,
        Some(drawn[above]),
        "PgUp on a team row selected the one above"
    );

    // The list's first row is a team row: PgUp there moves nothing.
    view.selected = drawn.first().copied();
    page_key_press(&mut window, &mut view, false);
    assert_eq!(
        view.selected,
        drawn.first().copied(),
        "PgUp on the list's first row moved nothing"
    );
}

// SE-GUI-53
#[test]
fn the_page_keys_walk_the_team_rows_a_filter_leaves() {
    // "piss" matches one player of 707 ("OCEAN OF PISS") and one of 745
    // ("PISS BOTTLE"), so the list holds exactly two team rows: the
    // bound where the team-row walk still applies.
    let path = fixture(PesVersion::Pes17, "page_keys_filter");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::All;
    view.filter = "piss".to_owned();
    let mut window = Headless::new();
    let drawn = listed_selections(&view);
    let team_rows: Vec<usize> = drawn
        .iter()
        .enumerate()
        .filter_map(|(index, row)| matches!(row, Selection::Team(_)).then_some(index))
        .collect();
    assert_eq!(
        team_rows.len(),
        2,
        "premise: the filter leaves exactly two team rows"
    );
    assert_eq!(
        (drawn[team_rows[0]], drawn[team_rows[1]]),
        (Selection::Team(707), Selection::Team(745)),
        "premise: the two are 707's and 745's"
    );
    let first_player = drawn[team_rows[0] + 1];
    assert!(
        matches!(first_player, Selection::Player(707, _)),
        "premise: the row under 707's is one of its players"
    );
    view.selected = Some(first_player);
    draw(&mut window, &mut view);

    page_key_press(&mut window, &mut view, true);
    assert_eq!(
        view.selected,
        Some(drawn[team_rows[1]]),
        "PgDn selected the second team's row"
    );
}
