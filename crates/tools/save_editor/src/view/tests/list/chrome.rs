use super::*;

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

// The team selector names a team by its display name, its shown text and
// its entries alike: a colour code's control character and hex digits
// never show.
#[test]
fn the_team_selector_names_a_coded_team_by_its_display_name() {
    let path = fixture(PesVersion::Pes21, "selector_display_name");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let name = file.team(701).unwrap().name.clone();
    file.team_mut(701).unwrap().name = format!("\x11cff0000ff{name}");
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let selector = nodes
        .iter()
        .filter(|node| node.role() == Role::ComboBox)
        .min_by(|a, b| {
            let x = |node: &&accesskit::Node| node.bounds().map(|r| r.x0).unwrap_or(f64::MAX);
            x(a).partial_cmp(&x(b)).expect("ordered bounds")
        })
        .expect("the team selector");
    assert_eq!(selector.value(), Some(name.as_str()), "the shown text");

    click(&mut window, selector, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let entries: Vec<&str> = nodes
        .iter()
        .filter(|node| node.role() == Role::Button)
        .filter_map(|node| node.label())
        .collect();
    assert!(entries.contains(&name.as_str()), "the entry: {entries:?}");
    assert!(
        entries.iter().all(|entry| !entry.contains('\u{11}')),
        "no colour code byte in the popup: {entries:?}"
    );
}

// The team row draws `[click for gameplan]` after the name, in the weak
// colour: nothing else says where the tactics are.
#[test]
fn the_team_row_hints_click_for_gameplan() {
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
    let hint = text_in_rect(&shapes, "[click for gameplan]", bounds);
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

/// One frame of the PES 21 fixture under ALL, the list scrolling, after a
/// frame that measured the rows: the frame's nodes and shapes.
fn scrolling_list(
    test: &str,
) -> (
    Headless,
    Vec<accesskit::Node>,
    Vec<egui::epaint::ClippedShape>,
) {
    let path = fixture(PesVersion::Pes21, test);
    let mut view = SaveView::new();
    view.open(&path);
    assert_eq!(view.team, TeamChoice::All, "premise: ALL selected");
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    (window, nodes, shapes)
}

// The draft's `.vbar` has no track fill: the panel shows through the
// scroll bar's strip, so nothing in the extreme background is painted
// there while the list scrolls (the rows' frame, its fill, ends at the
// strip's left edge). The handle still paints, in the widgets' fill.
#[test]
fn the_lists_scroll_strip_paints_no_track() {
    let (window, nodes, shapes) = scrolling_list("list_no_track");
    let frame = list_frame(&window, &shapes);
    let bar = list_scroll_bar(&nodes);
    assert!(
        bar.x0 > f64::from(frame.max.x),
        "premise: the list scrolls, its bar right of the frame"
    );
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let extreme = visuals.extreme_bg_color;
    let bar_rect = egui::Rect::from_min_max(
        egui::pos2(bar.x0 as f32, bar.y0 as f32),
        egui::pos2(bar.x1 as f32, bar.y1 as f32),
    );
    let handle = shapes.iter().find_map(|clipped| match &clipped.shape {
        egui::Shape::Rect(shape)
            if shape.fill == visuals.widgets.inactive.bg_fill
                && bar_rect.contains_rect(shape.rect) =>
        {
            Some(shape.rect)
        }
        _ => None,
    });
    assert!(
        handle.is_some_and(|rect| rect.height() > 10.0),
        "the handle paints in the widgets' fill inside the bar {bar_rect:?} ({handle:?})"
    );
    let strip = egui::Rect::from_min_max(
        egui::pos2(frame.max.x, frame.min.y),
        egui::pos2(bar.x1 as f32, frame.max.y),
    );
    let track = shapes.iter().find_map(|clipped| match &clipped.shape {
        egui::Shape::Rect(shape)
            if shape.fill == extreme && shape.rect.intersect(strip).width() > 0.5 =>
        {
            Some(shape.rect)
        }
        _ => None,
    });
    assert_eq!(track, None, "no extreme fill inside the strip {strip:?}");
}

// The draft's `.plist .rows` has `border-radius: 2px`, as every framed
// panel: the rows' frame, its fill and its stroke, is drawn at the
// widgets' 2-point radius.
#[test]
fn the_lists_frame_rounds_its_corners() {
    let (window, _, shapes) = scrolling_list("list_frame_radius");
    let frame = list_frame(&window, &shapes);
    let extreme = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .extreme_bg_color;
    let frame_shapes: Vec<&egui::epaint::RectShape> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if shape.rect == frame => Some(shape),
            _ => None,
        })
        .collect();
    let fill = frame_shapes
        .iter()
        .find(|shape| shape.fill == extreme)
        .expect("the frame's fill");
    let stroke = frame_shapes
        .iter()
        .find(|shape| !shape.stroke.is_empty())
        .expect("the frame's stroke");
    for (what, shape) in [("fill", fill), ("stroke", stroke)] {
        assert_eq!(
            shape.corner_radius,
            egui::CornerRadius::same(2),
            "the frame's {what} radius"
        );
    }
}

// The draft draws no separator lines in the list's column: none at the
// list pane's right edge, between the scroll bar and the detail, and none
// across the column at the quick actions' top, under the rows' frame.
// The column's geometry stays: the frame's right edge (pinned by
// `the_lists_frame_stands_where_the_drafts_does`), the bar (by
// `the_lists_scroll_bar_stands_outside_its_frame`), and the detail pane
// 4 after the bar, its content 12 after it with its own 8-point margin.
#[test]
fn the_lists_column_draws_no_separator_lines() {
    let (window, nodes, shapes) = scrolling_list("list_no_separators");
    let frame = list_frame(&window, &shapes);
    let bar = list_scroll_bar(&nodes);
    let (pane_left, _) = pane_frame(&shapes, window.screen);
    assert!(
        (pane_left - bar.x1 - 4.0).abs() <= 0.5,
        "the detail pane 4 after the bar ({pane_left} vs {})",
        bar.x1
    );
    // A painted line's two ends, as `vline` and `hline` paint them.
    let lines: Vec<[egui::Pos2; 2]> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke } if !stroke.is_empty() => Some(*points),
            _ => None,
        })
        .collect();
    let vertical_at_edge = lines.iter().find(|[a, b]| {
        (a.x - b.x).abs() < 0.5
            && f64::from(a.x) > bar.x1 - 0.5
            && f64::from(a.x) < pane_left + 0.5
            && (a.y - b.y).abs() > window.screen.y * 0.5
    });
    assert_eq!(vertical_at_edge, None, "no line at the pane's right edge");
    let across_column = lines.iter().find(|[a, b]| {
        (a.y - b.y).abs() < 0.5
            && a.y >= frame.max.y - 0.5
            && a.x.min(b.x) <= frame.min.x + 0.5
            && f64::from(a.x.max(b.x)) <= bar.x1 + 0.5
            && f64::from(a.x.max(b.x)) >= f64::from(frame.max.x) - 0.5
    });
    assert_eq!(
        across_column, None,
        "no line across the column at the quick actions' top"
    );
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
