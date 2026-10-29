//! The toolbar's tests: its groups in order with a separator between each,
//! the menus switching on hover, the menus' entries grouped by what they
//! act on (no Panels menu), a shortcut right of its label, and the unsaved
//! marker in the link blue.

use super::*;

/// The toolbar's band: the Open menu button's vertical span.
fn toolbar_band(nodes: &[accesskit::Node]) -> (f64, f64) {
    let open = menu_button(nodes, "Open")
        .bounds()
        .expect("the Open menu button's bounds");
    (open.y0, open.y1)
}

/// The x of each toolbar separator's line, left to right: the vertical
/// lines in the panel's stroke colour inside the toolbar's band.
fn separator_xs(
    window: &Headless,
    nodes: &[accesskit::Node],
    shapes: &[egui::epaint::ClippedShape],
) -> Vec<f64> {
    let (top, bottom) = toolbar_band(nodes);
    let stroke = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    let mut xs: Vec<f64> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke: s }
                if *s == stroke
                    && points[0].x == points[1].x
                    && f64::from(points[0].y.min(points[1].y)) >= top - 1.0
                    && f64::from(points[0].y.max(points[1].y)) <= bottom + 1.0 =>
            {
                Some(f64::from(points[0].x))
            }
            _ => None,
        })
        .collect();
    xs.sort_by(|a, b| a.partial_cmp(b).expect("ordered xs"));
    xs
}

/// The ruleset dropdown: the combo box in the toolbar's band.
fn ruleset_dropdown(nodes: &[accesskit::Node]) -> &accesskit::Node {
    let (top, bottom) = toolbar_band(nodes);
    nodes
        .iter()
        .find(|node| {
            node.role() == Role::ComboBox
                && node.bounds().is_some_and(|b| b.y0 < bottom && b.y1 > top)
        })
        .expect("the ruleset dropdown in the toolbar")
}

/// The toolbar's groups read left to right: Open, Save, Save as | Undo, Redo |
/// Player, Team, Savefile | the ruleset dropdown, Live AATF; each `|` one
/// separator's line, standing between the widgets either side of it.
#[test]
fn the_toolbar_reads_its_groups_in_order_with_a_separator_between_each() {
    let path = fixture(PesVersion::Pes21, "toolbar_order");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let x = |node: &accesskit::Node| {
        let bounds = node.bounds().expect("a toolbar widget's bounds");
        (bounds.x0, bounds.x1)
    };
    let widgets = [
        x(menu_button(&nodes, "Open")),
        x(node_labeled(&nodes, Role::Button, "Save")),
        x(node_labeled(&nodes, Role::Button, "Save as")),
        x(node_labeled(&nodes, Role::Button, "Undo")),
        x(node_labeled(&nodes, Role::Button, "Redo")),
        x(menu_button(&nodes, "Player")),
        x(menu_button(&nodes, "Team")),
        x(menu_button(&nodes, "Savefile")),
        x(ruleset_dropdown(&nodes)),
        x(node_labeled(&nodes, Role::Button, "\u{25b6} Live AATF")),
    ];
    for pair in widgets.windows(2) {
        assert!(pair[0].1 <= pair[1].0, "left to right: {widgets:?}");
    }
    let separators = separator_xs(&window, &nodes, &shapes);
    assert_eq!(separators.len(), 3, "three separators: {separators:?}");
    // After Save as, after Redo, after Savefile.
    for (separator, after) in separators.iter().zip([2, 4, 7]) {
        assert!(
            widgets[after].1 < *separator && *separator < widgets[after + 1].0,
            "a separator between {:?} and {:?}: {separators:?}",
            widgets[after],
            widgets[after + 1]
        );
    }
}

/// With the Player menu open, the pointer moving onto Team's button (no
/// click) opens the Team menu and closes the Player menu.
#[test]
fn hovering_team_while_player_is_open_switches_the_menus() {
    let path = fixture(PesVersion::Pes21, "toolbar_hover");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let nodes = open_menu(&mut window, &mut view, "Player", "Set all stats");
    assert!(
        nodes
            .iter()
            .any(|node| node.label() == Some("Set all stats\u{2026}")),
        "premise: the Player menu is open"
    );
    let team = centre(menu_button(&nodes, "Team"));
    window.frame(vec![egui::Event::PointerMoved(team)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let nodes = draw(&mut window, &mut view);
    assert!(
        nodes
            .iter()
            .any(|node| node.label() == Some("Set edited flag")),
        "the Team menu opened"
    );
    assert!(
        !nodes
            .iter()
            .any(|node| node.label() == Some("Set all stats\u{2026}")),
        "the Player menu closed"
    );
}

/// `menu`'s entries top to bottom, and the y of each separator line
/// between them (the horizontal lines in the panel's stroke colour across
/// the entries' column).
fn menu_entries(
    window: &mut Headless,
    view: &mut SaveView,
    menu: &str,
    first: &str,
) -> (Vec<(String, f64, f64)>, Vec<f64>) {
    open_menu(window, view, menu, first);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let first = menu_item(&nodes, first).bounds().expect("bounds");
    let mut entries: Vec<(String, f64, f64)> = nodes
        .iter()
        .filter(|node| node.role() == Role::Button)
        .filter_map(|node| {
            let bounds = node.bounds()?;
            let label = node.label()?;
            ((bounds.x0 - first.x0).abs() < 0.5 && bounds.y0 >= first.y0)
                .then(|| (label.to_owned(), bounds.y0, bounds.y1))
        })
        .collect();
    entries.sort_by(|a, b| a.1.partial_cmp(&b.1).expect("ordered entries"));
    let stroke = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    let mut lines: Vec<f64> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke: s }
                if *s == stroke
                    && points[0].y == points[1].y
                    && f64::from(points[0].x.min(points[1].x)) <= first.x0 + 8.0
                    && f64::from(points[0].x.max(points[1].x)) >= first.x1 - 8.0 =>
            {
                Some(f64::from(points[0].y))
            }
            _ => None,
        })
        .collect();
    lines.sort_by(|a, b| a.partial_cmp(b).expect("ordered lines"));
    (entries, lines)
}

/// Whether a separator line stands between entries `above` and `below`.
fn separated(entries: &[(String, f64, f64)], lines: &[f64], above: usize, below: usize) -> bool {
    lines
        .iter()
        .any(|&y| entries[above].2 <= y && y <= entries[below].1)
}

/// No Panels menu; the Savefile menu holds the whole save's work grouped
/// by what it acts on: the Comparator, a separator, the roster sheet and
/// Multi-team AATF, a separator, Fix database.
#[test]
fn the_savefile_menu_takes_the_comparator_and_there_is_no_panels_menu() {
    let path = fixture(PesVersion::Pes21, "toolbar_savefile");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    let nodes = draw(&mut window, &mut view);
    assert!(
        !nodes
            .iter()
            .any(|node| node.label() == Some(menu_label("Panels").as_str())),
        "no Panels menu"
    );
    let (entries, lines) = menu_entries(&mut window, &mut view, "Savefile", "Comparator");
    let labels: Vec<&str> = entries.iter().map(|entry| entry.0.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Comparator\u{2026}",
            "Output rosters to TSV\u{2026}",
            "Multi-team AATF\u{2026}",
            "Fix database\u{2026}",
        ]
    );
    assert!(separated(&entries, &lines, 0, 1), "after the Comparator");
    assert!(!separated(&entries, &lines, 1, 2), "the reports together");
    assert!(separated(&entries, &lines, 2, 3), "before Fix database");
}

/// A menu item's shortcut is drawn right of its label in the weak colour,
/// inside the menu's width: Bump all stats' `Ctrl+B`.
#[test]
fn a_shortcut_is_drawn_right_of_its_label_inside_the_menu() {
    let path = fixture(PesVersion::Pes21, "toolbar_shortcut");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_menu(&mut window, &mut view, "Player", "Bump all stats");
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let item = menu_item(&nodes, "Bump all stats")
        .bounds()
        .expect("bounds");
    let label = text_shape(&shapes, "Bump all stats");
    let shortcut = text_shape(&shapes, "Ctrl+B");
    let label_right = label.pos.x + label.galley.size().x;
    assert!(
        shortcut.pos.x > label_right,
        "the shortcut right of the label ({} vs {label_right})",
        shortcut.pos.x
    );
    assert!(
        f64::from(shortcut.pos.x + shortcut.galley.size().x) <= item.x1,
        "inside the menu's width"
    );
    assert_eq!(
        galley_color(shortcut),
        window.ctx.global_style().visuals.weak_text_color(),
        "the weak colour"
    );
}

/// The painted text shape whose text is `text` exactly, the topmost below
/// `below`.
fn text_below<'a>(
    shapes: &'a [egui::epaint::ClippedShape],
    text: &str,
    below: f32,
) -> &'a egui::epaint::TextShape {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.job.text == text && shape.pos.y > below => {
                Some(shape)
            }
            _ => None,
        })
        .min_by(|a, b| a.pos.y.partial_cmp(&b.pos.y).expect("ordered shapes"))
        .unwrap_or_else(|| panic!("no painted {text:?} below {below}"))
}

/// The view's title `title` in the frame's shapes, and the two ink gaps
/// around it: from the toolbar's buttons' bottom to the title's ink, and
/// from the title's ink to the ink of `first_row`, the view's first row's
/// text.
fn title_gaps(
    nodes: &[accesskit::Node],
    shapes: &[egui::epaint::ClippedShape],
    title: &str,
    first_row: &str,
) -> (f64, f64) {
    let buttons = toolbar_band(nodes).1;
    let title = text_below(shapes, title, 0.0);
    let title_ink = ink(title);
    let row_ink = ink(text_below(shapes, first_row, title_ink.bottom()));
    (
        f64::from(title_ink.top()) - buttons,
        f64::from(row_ink.top() - title_ink.bottom()),
    )
}

/// The player view's title `<id> <name>` stands as far below the
/// toolbar's buttons as General info's title is below it, in ink (within
/// a point).
#[test]
fn the_player_views_title_sits_midway_in_ink_between_the_toolbar_and_the_first_row() {
    let path = fixture(PesVersion::Pes21, "title_player_gaps");
    let mut view = SaveView::new();
    view.open(&path);
    let (_, _, player_id) = selected_player(&view).expect("a player row is selected");
    let name = display_name(&player_entry(&view, player_id).name);
    let mut window = Headless::new();
    draw(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (above, below) = title_gaps(
        &nodes,
        &shapes,
        &format!("{player_id} {name}"),
        "General info",
    );
    assert!(
        (above - below).abs() <= 1.0,
        "the ink gaps {above} above and {below} below"
    );
}

/// The team view's title is `<id> <name>`, team 701's `701 /3/` in the
/// PES 21 fixture, the id in the weak colour, and stands as far below the
/// toolbar's buttons as the Name field's row is below it, in ink (within a
/// point).
#[test]
fn the_team_views_title_reads_id_and_name_and_sits_midway_in_ink() {
    let path = fixture(PesVersion::Pes21, "title_team_gaps");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let title = text_below(&shapes, "701 /3/", 0.0);
    let visuals = window.ctx.global_style().visuals.clone();
    assert_eq!(
        title.galley.job.sections[0].format.color,
        visuals.weak_text_color(),
        "the id in the weak colour"
    );
    let (above, below) = title_gaps(&nodes, &shapes, "701 /3/", crate::view::team::TEAM_LABEL);
    assert!(
        (above - below).abs() <= 1.0,
        "the ink gaps {above} above and {below} below"
    );
}

/// A dirty session's `• unsaved changes` paints in the link blue, not the
/// weak grey that hid it among the toolbar's quieter text.
#[test]
fn the_unsaved_marker_is_in_the_link_blue() {
    let path = fixture(PesVersion::Pes21, "toolbar_unsaved");
    let mut view = SaveView::new();
    view.open(&path);
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    let mut window = Headless::new();
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let visuals = window.ctx.global_style().visuals.clone();
    assert_ne!(
        visuals.hyperlink_color,
        visuals.weak_text_color(),
        "premise: the two colours differ"
    );
    assert_eq!(
        galley_color(text_shape(&shapes, "\u{2022} unsaved changes")),
        visuals.hyperlink_color
    );
}
