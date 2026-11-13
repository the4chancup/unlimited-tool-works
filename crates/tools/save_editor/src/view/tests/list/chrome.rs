use studio_core::StudioTool as _;

use super::*;

/// The team selector's combo: the list panel's leftmost.
fn selector_combo(nodes: &[accesskit::Node]) -> &accesskit::Node {
    nodes
        .iter()
        .filter(|node| node.role() == Role::ComboBox)
        .min_by(|a, b| {
            let x = |node: &&accesskit::Node| node.bounds().map(|r| r.x0).unwrap_or(f64::MAX);
            x(a).partial_cmp(&x(b)).expect("ordered bounds")
        })
        .expect("the team selector")
}

/// The category radio button labeled `label` in the open popup.
fn radio<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| node.role() == Role::RadioButton && node.label() == Some(label))
        .unwrap_or_else(|| panic!("no {label:?} radio button"))
}

/// The team selector's combo, clicked open; the popup's `entry` item
/// picked through its AccessKit action.
fn pick_in_selector(window: &mut Headless, view: &mut SaveView, entry: &str) {
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    // A pick may have closed the popup; click the combo open only while
    // `entry` is not drawn (a click while it is open closes it).
    if !nodes
        .iter()
        .any(|node| node.role() == Role::Button && node.label() == Some(entry))
    {
        click(window, selector_combo(&nodes), view);
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
// detail shows the team's view and its tactics; picking All teams keeps the
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

    pick_in_selector(&mut window, &mut view, "All teams");
    assert_eq!(
        view.selected,
        Some(Selection::Team(702)),
        "picking All teams keeps the selection"
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

// The four category radio buttons under the selector's teams shorten the
// dropdown and the list to one competition's teams: `All teams`, then the
// category's teams alone, in the popup as in the list; a team the
// category leaves out gives way to `All teams`, and a selection it leaves
// out moves to the list's first player.
// SE-GUI-83
#[test]
fn the_selectors_categories_shorten_the_dropdown_and_the_list() {
    let path = fixture(PesVersion::Pes21, "team_categories");
    let file = EditFile::load(&path).unwrap();
    // The fixture's boundary teams: Backup 9 is the last 4cc team, /@/
    // the first VGL one.
    assert_eq!(file.team(780).unwrap().name, "Backup 9", "premise: 780");
    assert_eq!(file.team(781).unwrap().name, "/@/", "premise: 781");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();

    // The cup teams' display names in `lo`..=`hi`, the save's order.
    let names = |lo: u32, hi: u32| -> Vec<String> {
        file.teams()
            .iter()
            .filter(|team| (lo..=hi).contains(&team.id))
            .map(|team| display_name(&team.name))
            .collect()
    };
    let all_names = names(701, 920);
    // The open popup's entries, in order: the `All teams` row and the
    // listed teams.
    let entries = |nodes: &[accesskit::Node]| -> Vec<String> {
        let mut items: Vec<&accesskit::Node> = nodes
            .iter()
            .filter(|node| {
                node.role() == Role::Button
                    && node.label().is_some_and(|label| {
                        label == "All teams" || all_names.iter().any(|name| name == label)
                    })
            })
            .collect();
        items.sort_by(|a, b| {
            let y = |node: &&accesskit::Node| node.bounds().map(|r| r.y0).unwrap_or(0.0);
            y(a).partial_cmp(&y(b)).expect("ordered bounds")
        });
        items
            .iter()
            .filter_map(|node| node.label().map(str::to_owned))
            .collect()
    };
    // The popup's radio buttons in column order, each with its tick.
    let radios = |nodes: &[accesskit::Node]| -> Vec<(String, bool)> {
        let mut radios: Vec<&accesskit::Node> = nodes
            .iter()
            .filter(|node| {
                node.role() == Role::RadioButton
                    && node
                        .label()
                        .is_some_and(|label| ["All", "4cc", "VGL", "Invitational"].contains(&label))
            })
            .collect();
        radios.sort_by(|a, b| {
            let y = |node: &&accesskit::Node| node.bounds().map(|r| r.y0).unwrap_or(0.0);
            y(a).partial_cmp(&y(b)).expect("ordered bounds")
        });
        radios
            .iter()
            .map(|node| {
                (
                    node.label().expect("a radio's label").to_owned(),
                    node.toggled() == Some(accesskit::Toggled::True),
                )
            })
            .collect()
    };
    let expected = |names: Vec<String>| -> Vec<String> {
        std::iter::once("All teams".to_owned())
            .chain(names)
            .collect()
    };

    // `All teams` first, then the save's teams; under them the four
    // radios, `All` ticked.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        selector_combo(&nodes).value(),
        Some("All teams"),
        "the closed combo's text"
    );
    click(&mut window, selector_combo(&nodes), &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        entries(&nodes),
        expected(all_names.clone()),
        "All's entries"
    );
    assert_eq!(
        radios(&nodes),
        [
            ("All".to_owned(), true),
            ("4cc".to_owned(), false),
            ("VGL".to_owned(), false),
            ("Invitational".to_owned(), false),
        ],
        "the radios under the teams, All ticked"
    );

    // `VGL` ticked through a pointer click: the popup stays open and
    // lists `All teams` and the 781 to 850 teams alone.
    click(&mut window, radio(&nodes, "VGL"), &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        radios(&nodes),
        [
            ("All".to_owned(), false),
            ("4cc".to_owned(), false),
            ("VGL".to_owned(), true),
            ("Invitational".to_owned(), false),
        ],
        "VGL ticked, the popup still open"
    );
    assert_eq!(
        entries(&nodes),
        expected(names(781, 850)),
        "`All teams` and the VGL teams alone"
    );
    // The radios stand under the teams' first rows, in view while the
    // teams scroll: the first radio tops out before the seventh item's
    // place, not below all seventy.
    let mut item_tops: Vec<f64> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && node.label().is_some_and(|label| {
                    label == "All teams" || all_names.iter().any(|name| name == label)
                })
        })
        .filter_map(|node| node.bounds().map(|bounds| bounds.y0))
        .collect();
    item_tops.sort_by(|a, b| a.partial_cmp(b).expect("ordered tops"));
    let radio_top = nodes
        .iter()
        .filter(|node| node.role() == Role::RadioButton)
        .filter_map(|node| node.bounds().map(|bounds| bounds.y0))
        .fold(f64::MAX, f64::min);
    assert!(
        item_tops.first().is_some_and(|top| radio_top > *top) && radio_top < item_tops[6],
        "the radios in view under the teams ({radio_top} vs {item_tops:?})"
    );

    // The list holds the VGL teams' rows and players alone, and the
    // selection is its first player row.
    let drawn_teams: Vec<u32> = nodes
        .iter()
        .filter(|node| node.role() == Role::Button && !player_row(node))
        .filter_map(|node| node.label())
        .filter_map(|label| label.split(' ').next()?.parse().ok())
        .collect();
    assert!(
        drawn_teams.iter().all(|id| (781..=850).contains(id)),
        "only VGL team rows drawn: {drawn_teams:?}"
    );
    assert!(drawn_teams.contains(&781), "781's row among them");
    assert_eq!(
        view.selected,
        Some(Selection::Player(781, 0)),
        "the selection moved to the list's first player row"
    );

    // `4cc` ticked: the 701 to 780 teams alone, Backup 9 (780) among
    // them and no 781.
    click(&mut window, radio(&nodes, "4cc"), &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let four_cc = entries(&nodes);
    assert_eq!(four_cc, expected(names(701, 780)), "the 4cc teams alone");
    assert!(
        four_cc.iter().any(|name| name == "Backup 9"),
        "780 among them"
    );
    assert!(!four_cc.iter().any(|name| name == "/@/"), "no 781");

    // 781 picked under `VGL`, then `Invitational` ticked: the picked
    // team is out of the new category, so the selector reads `All
    // teams` and the selection is Invitational's first player.
    click(&mut window, radio(&nodes, "VGL"), &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (item, _) = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("/@/"))
        .expect("781's entry in the VGL popup");
    action_click(&mut window, &mut view, *item);
    assert_eq!(view.team, TeamChoice::Team(781), "premise: 781 picked");
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, selector_combo(&nodes), &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, radio(&nodes, "Invitational"), &mut view);
    assert_eq!(
        view.team,
        TeamChoice::All,
        "the picked team out of the category gives way to All teams"
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        selector_combo(&nodes).value(),
        Some("All teams"),
        "the selector reads All teams"
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(851, 0)),
        "the selection is the list's first player row"
    );
}

// The wheel over the closed selector steps `All teams` and the
// category's teams alone: under VGL the step after `All teams` is the
// first VGL team, not the first 4cc one.
#[test]
fn the_closed_selectors_wheel_steps_the_categorys_teams() {
    let path = fixture(PesVersion::Pes21, "team_category_wheel");
    let mut view = SaveView::new();
    view.open(&path);
    view.team_category = TeamCategory::Vgl;
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // One notch down over the closed combo.
    window.frame(
        vec![
            egui::Event::PointerMoved(centre(selector_combo(&nodes))),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, -1.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        view.team,
        TeamChoice::Team(781),
        "one notch down: the first VGL team"
    );
}

/// The hint's text shape painted inside `bounds` (a team row's), when
/// one draws.
fn hint_in_row(
    shapes: &[egui::epaint::ClippedShape],
    bounds: accesskit::Rect,
) -> Option<&egui::epaint::TextShape> {
    shapes
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
}

// The team row is the team's id, swatches and name alone: no hint
// after the name, the toggle's Gameplan taking the member there
// (SE-GUI-82's last clause).
// SE-GUI-82
#[test]
fn the_team_row_paints_no_click_for_gameplan() {
    let path = fixture(PesVersion::Pes21, "team_row_no_hint");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = team_row(&nodes, &view, 701)
        .bounds()
        .expect("the team row's bounds");
    assert!(
        hint_in_row(&shapes, bounds).is_none(),
        "no hint paints in the row"
    );
    // The row still paints its name where the hint's room was.
    text_in_rect(&shapes, "/3/", bounds);
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

/// One frame of the PES 21 fixture under All teams, the list scrolling, after a
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
    assert_eq!(view.team, TeamChoice::All, "premise: All teams selected");
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
    let radius = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .widgets
        .noninteractive
        .corner_radius;
    for (what, shape) in [("fill", fill), ("stroke", stroke)] {
        assert_eq!(shape.corner_radius, radius, "the frame's {what} radius");
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
    let pane_left = f64::from(pane_frame(&shapes, window.screen).min.x);
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
// 1280 x 780 window: the team combo's top 29, 4 under the toolbar band's
// line (the band runs 0 to 25: 3, the 18-point row, 3 and the 1-point
// line), then the filter box and the rows' frame each 3 under the last,
// egui's item spacing, the frame's top at 71.
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
    for (top, draft) in tops.iter().zip([29.0, 50.0, 71.0]) {
        assert!(
            (top - draft).abs() <= 0.5,
            "the combo, the filter and the frame at the draft's 29, 50 and 71 ({tops:?})"
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
// too, under All teams. A player row's place, position and name are columns:
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

// The category picked in the selector is the `team_category` setting,
// written on the tick: a restarted editor ticks it back, opening
// another save keeps it, and an unknown value in the file reads as
// `all`.
// SE-GUI-83
#[test]
fn the_category_setting_survives_a_restart_and_another_save() {
    let dir = crate::test_support::scratch("team_category_setting");
    let path = dir.join("EDIT00000000");
    std::fs::write(&path, fixture_save(PesVersion::Pes21)).unwrap();
    let (ctx, shared, _events_rx, _requests_rx) =
        crate::test_support::shared_tool_context(&dir, studio_core::Settings::default());
    let mut tool = crate::Tool::new();
    tool.view.open(&path);
    let mut window = Headless::new();

    // Tick VGL in the selector, a pointer click as the member does it.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    press_release(&mut window, centre(selector_combo(&nodes)), |ui| {
        tool.view(ui, &ctx)
    });
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    press_release(&mut window, centre(radio(&nodes, "VGL")), |ui| {
        tool.view(ui, &ctx)
    });
    assert_eq!(
        ctx.tool_settings(crate::messages::TOOL_ID)
            .get("team_category")
            .and_then(toml::Value::as_str),
        Some("vgl"),
        "the tick wrote the setting"
    );

    // The editor started again ticks it; opening another save keeps it.
    let mut restarted = crate::Tool::new();
    window.frame(Vec::new(), |ui| restarted.view(ui, &ctx));
    assert_eq!(
        restarted.view.team_category,
        TeamCategory::Vgl,
        "the restart reads the setting"
    );
    let second = dir.join("second");
    std::fs::write(&second, fixture_save(PesVersion::Pes17)).unwrap();
    restarted.view.open(&second);
    assert_eq!(
        restarted.view.team_category,
        TeamCategory::Vgl,
        "another save keeps it"
    );

    // An unknown value in the file reads as `all`.
    let mut table = toml::Table::new();
    table.insert(
        "team_category".to_owned(),
        toml::Value::String("cup".to_owned()),
    );
    shared
        .lock()
        .unwrap()
        .set_tool(crate::messages::TOOL_ID, table);
    let mut third = crate::Tool::new();
    window.frame(Vec::new(), |ui| third.view(ui, &ctx));
    assert_eq!(
        third.view.team_category,
        TeamCategory::All,
        "an unknown value reads as all"
    );
}
