use super::*;

/// The title's toggle's segment labeled `label`.
fn segment<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
    node_labeled(nodes, Role::Button, label)
}

/// The toggle's rect: the three segments' bounds' union.
fn toggle_rect(nodes: &[accesskit::Node]) -> accesskit::Rect {
    ["Stats", "Appearance", "Gameplan"]
        .iter()
        .map(|label| segment(nodes, label).bounds().expect("bounds"))
        .reduce(|left, right| left.union(right))
        .expect("the three segments")
}

/// Whether `label`'s segment is the toggle's held one.
fn chosen(nodes: &[accesskit::Node], label: &str) -> bool {
    segment(nodes, label).toggled() == Some(accesskit::Toggled::True)
}

/// The toggle's `label` segment pressed, through the AccessKit click (a
/// press lands whatever the pane's scroll left under the pointer).
fn press_segment(window: &mut Headless, view: &mut SaveView, label: &str) {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let (id, _) = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(label))
        .unwrap_or_else(|| panic!("the {label} segment"));
    action_click(window, view, *id);
}

/// Whether the Appearance screen's one control shows: the `Hide default
/// body` button, which only it draws.
fn appearance_shown(nodes: &[accesskit::Node]) -> bool {
    nodes
        .iter()
        .any(|node| node.role() == Role::Button && node.label() == Some("Hide default body"))
}

/// The y of the first stroke-coloured line under `top` that spans the
/// pane (`left`/`right` being the pane's edges).
fn pane_line_below(
    shapes: &[egui::epaint::ClippedShape],
    stroke: egui::Stroke,
    top: f64,
    left: f64,
    right: f64,
) -> Option<f64> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke: s }
                if *s == stroke
                    && points[0].y == points[1].y
                    && f64::from(points[0].y) > top
                    && f64::from(points[0].x.min(points[1].x)) <= left + 1.0
                    && f64::from(points[0].x.max(points[1].x)) >= right - 1.0 =>
            {
                Some(f64::from(points[0].y))
            }
            _ => None,
        })
        .min_by(f64::total_cmp)
}

/// The title's row with its three segments on all three screens, the
/// toggle following the selection and moving it.
// SE-GUI-82
#[test]
fn the_toggle_moves_the_detail_between_the_three_screens() {
    let path = fixture(PesVersion::Pes17, "toggle_screens");
    let mut view = SaveView::new();
    view.open(&path);
    // The save open on team 718's player 05 (its fifth roster slot).
    assert_eq!(
        view.session.as_ref().unwrap().slot_player(718, 4),
        Some(71805),
        "premise: slot 05 holds player 71805"
    );
    view.selected = Some(Selection::Player(718, 4));
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);

    // The title's row starts at the pane's left edge with the three
    // segments, Stats on, then the id and the name; a separator under the
    // row; no Appearance header in the detail.
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let pane_left = f64::from(pane_frame(&shapes, window.screen).min.x + PANEL_MARGIN);
    let stats = segment(&nodes, "Stats").bounds().expect("bounds");
    assert!(
        (stats.x0 - pane_left).abs() <= 0.5,
        "the title's row starts at the pane's left ({stats:?} vs {pane_left})"
    );
    for (label, held) in [("Stats", true), ("Appearance", false), ("Gameplan", false)] {
        assert_eq!(chosen(&nodes, label), held, "{label}");
    }
    let title = right_of(&nodes, segment(&nodes, "Gameplan"), Role::Label);
    assert!(
        title.value().is_some_and(|v| v.starts_with("71805 ")),
        "the id and name after the toggle: {:?}",
        title.value()
    );
    let row_bottom = ["Stats", "Appearance", "Gameplan"]
        .iter()
        .map(|label| segment(&nodes, label).bounds().expect("bounds").y1)
        .fold(0.0, f64::max);
    let stroke = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    pane_line_below(
        &shapes,
        stroke,
        row_bottom,
        pane_left,
        f64::from(PANE_RIGHT),
    )
    .expect("a separator under the title's row");
    assert!(
        !nodes
            .iter()
            .any(|node| node.role() == Role::DisclosureTriangle),
        "no Appearance header in the detail"
    );
    assert!(label_value(&nodes, "General info"), "Stats shows");

    // Appearance pressed: its screen shows, the toggle where it stood,
    // and no undo entry or unsaved mark follows.
    assert!(
        !appearance_shown(&nodes),
        "premise: Stats does not show the control only Appearance has"
    );
    let before = toggle_rect(&nodes);
    press_segment(&mut window, &mut view, "Appearance");
    let nodes = draw(&mut window, &mut view);
    assert!(appearance_shown(&nodes), "the Appearance screen shows");
    assert_eq!(
        toggle_rect(&nodes),
        before,
        "the toggle's rect where it was"
    );
    assert_drawing_edited_nothing(&view);

    // Another player's row clicked: Appearance still shows, for him.
    // Player 07 is not team 718's first slot (that is 01), so a press
    // back to him later proves the last-shown rule, not the fallback.
    view.team = TeamChoice::Team(718);
    let nodes = draw(&mut window, &mut view);
    let row07 = nodes
        .iter()
        .find(|node| player_row(node) && node.label().is_some_and(|l| l.starts_with("07 ")))
        .expect("player 07's row");
    click(&mut window, row07, &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(718, 6)),
        "the click selected player 07"
    );
    let nodes = draw(&mut window, &mut view);
    assert!(
        appearance_shown(&nodes),
        "Appearance still shows for player 07"
    );
    assert!(
        nodes.iter().any(|node| {
            node.role() == Role::Label && node.value().is_some_and(|v| v.starts_with("71807 "))
        }),
        "the title names player 71807"
    );

    // Gameplan pressed: his team's row is selected, the team view shows,
    // Gameplan chosen, the toggle's rect where it was.
    press_segment(&mut window, &mut view, "Gameplan");
    assert_eq!(
        view.selected,
        Some(Selection::Team(718)),
        "Gameplan selected the team's row"
    );
    assert!(
        view.reveal_selected,
        "the list scrolls to show the selected row"
    );
    let nodes = draw(&mut window, &mut view);
    assert!(chosen(&nodes, "Gameplan"), "Gameplan chosen");
    node_labeled(&nodes, Role::Button, "Kit slots\u{2026}");
    assert_eq!(
        toggle_rect(&nodes),
        before,
        "the toggle's rect where it was"
    );

    // Stats pressed from the team's row: the player last shown on it
    // (player 07, not its first slot) is selected, Stats shows.
    press_segment(&mut window, &mut view, "Stats");
    assert_eq!(
        view.selected,
        Some(Selection::Player(718, 6)),
        "the player last shown on team 718 selected"
    );
    let nodes = draw(&mut window, &mut view);
    assert!(chosen(&nodes, "Stats"), "Stats chosen");
    assert!(label_value(&nodes, "General info"), "Stats shows");

    // Team 701's row selected with none of its players shown since the
    // save opened (it opened on 71805's view): Appearance pressed selects
    // the team's first filled slot on the Appearance screen.
    assert_eq!(
        view.session.as_ref().unwrap().slot_player(701, 0),
        Some(70101),
        "premise: 701's first filled slot"
    );
    view.team = TeamChoice::All;
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, team_row(&nodes, &view, 701), &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Team(701)),
        "the click selected team 701's row"
    );
    press_segment(&mut window, &mut view, "Appearance");
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "701's first filled slot selected"
    );
    let nodes = draw(&mut window, &mut view);
    assert!(
        appearance_shown(&nodes),
        "the Appearance screen shows for 70101"
    );

    // Opening another save: Stats again, and the remembered players gone —
    // team 718, whose player 07 was shown before the reopen, goes to its
    // first filled slot, not to him.
    open_again(&mut view, &path);
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "the save opens on its first player row"
    );
    let nodes = draw(&mut window, &mut view);
    assert!(chosen(&nodes, "Stats"), "Stats on the reopened save");
    view.selected = Some(Selection::Team(718));
    press_segment(&mut window, &mut view, "Appearance");
    assert_eq!(
        view.selected,
        Some(Selection::Player(718, 0)),
        "718's first filled slot, not the remembered player 07"
    );
}

/// The bottom of the team view's fields' row (its tallest control, read
/// off the topmost Short label's row, its controls right of the pane's
/// left edge — the player list's rows stand left of it).
fn fields_row_bottom(nodes: &[accesskit::Node], pane_left: f64) -> f64 {
    let short = nodes
        .iter()
        .filter(|node| node.role() == Role::Label && node.value() == Some("Short"))
        .filter_map(accesskit::Node::bounds)
        .min_by(|a, b| a.y0.partial_cmp(&b.y0).expect("ordered bounds"))
        .expect("the fields' row's Short label");
    nodes
        .iter()
        .filter(|node| {
            matches!(
                node.role(),
                Role::TextInput | Role::ColorWell | Role::SpinButton | Role::Button
            )
        })
        .filter_map(accesskit::Node::bounds)
        .filter(|b| b.x0 >= pane_left && b.y0 < short.y1 && b.y1 > short.y0)
        .map(|b| b.y1)
        .fold(short.y1, f64::max)
}

/// The team view after the separator moved under the title's row: no
/// line between the fields' row and the Tactics bars, and the row keeps
/// 9 points under it.
// SE-GUI-82
#[test]
fn the_team_view_keeps_9_under_the_fields_row_with_no_separator() {
    let path = fixture(PesVersion::Pes21, "toggle_team_nine");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let pane_left = f64::from(pane_frame(&shapes, window.screen).min.x + PANEL_MARGIN);
    let row_bottom = fields_row_bottom(&nodes, pane_left);
    // The bars' first choices stand 12 under the row's bottom: the 9
    // points the row keeps plus the bar's own margin over a choice. The
    // preset selector's and the formation selector's, the topmost of
    // each label.
    let topmost = |label: &str| {
        nodes
            .iter()
            .filter(|node| node.role() == Role::Button && node.label() == Some(label))
            .filter_map(accesskit::Node::bounds)
            .min_by(|a, b| a.y0.partial_cmp(&b.y0).expect("ordered bounds"))
            .unwrap_or_else(|| panic!("no {label} choice"))
    };
    for label in [
        "1",
        "2",
        "3",
        "Kick-off",
        "In-possession",
        "Out-of-possession",
    ] {
        let choice = topmost(label);
        assert!(
            (choice.y0 - row_bottom - 12.0).abs() <= 1.0,
            "{label} starts 12 under the fields' row ({} vs {row_bottom})",
            choice.y0
        );
    }
    // No separator between the row and the bars: no stroke-coloured line
    // spanning the pane in the gap.
    let stroke = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    // A separator under the row would span the pane, as the title row's
    // own does (above the row's bottom, it is filtered out).
    let line = pane_line_below(
        &shapes,
        stroke,
        row_bottom,
        pane_left,
        f64::from(PANE_RIGHT),
    );
    let preset = topmost("1");
    assert!(
        line.is_none_or(|y| y >= preset.y0),
        "no separator between the fields' row and the Tactics bars ({line:?})"
    );
}
