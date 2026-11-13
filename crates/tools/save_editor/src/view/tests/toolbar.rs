//! The toolbar's tests: its groups in order with a separator between each,
//! the menus switching on hover, the menus' entries grouped by what they
//! act on (no Panels menu), a shortcut right of its label, and the unsaved
//! marker in the link blue.

use super::*;

/// The toolbar's row: the File menu button's vertical span.
fn toolbar_band(nodes: &[accesskit::Node]) -> (f64, f64) {
    let file = menu_button(nodes, "File")
        .bounds()
        .expect("the File menu button's bounds");
    (file.y0, file.y1)
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

/// The toolbar's groups read left to right: File, Edit, Player, Team,
/// Savefile | the ruleset dropdown, Live AATF; the `|` one separator's
/// line, standing between the widgets either side of it. No Save, Save as,
/// Undo or Redo button stands in the row: they are the menus' entries.
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
        x(menu_button(&nodes, "File")),
        x(menu_button(&nodes, "Edit")),
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
    assert_eq!(separators.len(), 1, "one separator: {separators:?}");
    assert!(
        widgets[4].1 < separators[0] && separators[0] < widgets[5].0,
        "the separator between Savefile and the dropdown: {separators:?}"
    );
    let (top, bottom) = toolbar_band(&nodes);
    for gone in ["Save", "Save as\u{2026}", "Undo", "Redo"] {
        assert!(
            !nodes.iter().any(|node| {
                node.role() == Role::Button
                    && node.label() == Some(gone)
                    && node.bounds().is_some_and(|b| b.y0 < bottom && b.y1 > top)
            }),
            "no {gone} button in the row"
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

/// No Panels menu; the Savefile menu holds the whole save's work grouped
/// by what it acts on: the Comparator and Apply aesthetics patch, a
/// separator, the roster sheet and Multi-team AATF, a separator, Fix
/// database.
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
            "Apply aesthetics patch\u{2026}",
            "Output rosters to TSV\u{2026}",
            "Multi-team AATF\u{2026}",
            "Fix database\u{2026}",
        ]
    );
    assert!(
        !separated(&entries, &lines, 0, 1),
        "the panels reading another file together"
    );
    assert!(separated(&entries, &lines, 1, 2), "after the patch entry");
    assert!(!separated(&entries, &lines, 2, 3), "the reports together");
    assert!(separated(&entries, &lines, 3, 4), "before Fix database");
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

/// The bottom of the toolbar band's line: the horizontal line in the
/// panel's stroke that starts at the view's left edge, the first under the
/// toolbar's buttons. It is the bar's lower edge as the eye sees it.
fn band_bottom(
    window: &Headless,
    nodes: &[accesskit::Node],
    shapes: &[egui::epaint::ClippedShape],
) -> f64 {
    let stroke = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    let buttons = toolbar_band(nodes).1 as f32;
    let line = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke: s }
                if *s == stroke
                    && points[0].y == points[1].y
                    && points[0].x.min(points[1].x) == 0.0
                    && points[0].y >= buttons =>
            {
                Some(points[0].y)
            }
            _ => None,
        })
        .min_by(|a, b| a.partial_cmp(b).expect("ordered lines"))
        .expect("the band's line under the toolbar's buttons");
    f64::from(line + 0.5 * stroke.width)
}

/// The ink gap under a view's title's row and the separator's painted
/// row: the band's bottom to the title's ink, and whether a
/// stroke-coloured line spanning the pane stands between the title's ink
/// and `first_row`'s.
fn title_row_and_separator(
    window: &Headless,
    nodes: &[accesskit::Node],
    shapes: &[egui::epaint::ClippedShape],
    title: &str,
    first_row: &str,
) -> (f64, Option<f64>) {
    let band = band_bottom(window, nodes, shapes);
    let title_ink = ink(text_below(shapes, title, 0.0));
    let row_ink = ink(text_below(shapes, first_row, title_ink.bottom()));
    let stroke = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    let line = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke: s }
                if *s == stroke
                    && points[0].y == points[1].y
                    && f64::from(points[0].y) > f64::from(title_ink.bottom())
                    && f64::from(points[0].y) < f64::from(row_ink.top())
                    && f64::from(points[0].x.min(points[1].x)) <= title_ink.left() as f64 + 1.0
                    && f64::from(points[0].x.max(points[1].x)) >= f64::from(PANE_RIGHT) - 1.0 =>
            {
                Some(f64::from(points[0].y))
            }
            _ => None,
        })
        .min_by(f64::total_cmp);
    (f64::from(title_ink.top()) - band, line)
}

/// The player view's title's row stands under the toolbar's band, the
/// id and name's ink about 11.5 below it (the draft's `.pheader`
/// margin), the separator under the row before General info's title.
#[test]
fn the_player_views_title_row_stands_under_the_toolbar_with_a_separator() {
    let path = fixture(PesVersion::Pes21, "title_player_gaps");
    let mut view = SaveView::new();
    view.open(&path);
    let (_, _, player_id) = selected_player(&view).expect("a player row is selected");
    let name = display_name(&player_entry(&view, player_id).name);
    let mut window = Headless::new();
    draw(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (above, line) = title_row_and_separator(
        &window,
        &nodes,
        &shapes,
        &format!("{player_id} {name}"),
        "General info",
    );
    assert!(
        (above - 11.5).abs() <= 1.0,
        "the title's ink {above} under the toolbar's band"
    );
    assert!(line.is_some(), "a separator under the title's row");
}

/// The team view's title's row holds `<id> <name>` after the toggle,
/// team 701's `701 /3/` in the PES 21 fixture, the id in the weak colour,
/// its ink about 11.5 below the toolbar's band, the separator under the
/// row before the Name field's row.
#[test]
fn the_team_views_title_row_reads_id_and_name_under_a_separator() {
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
    let (above, line) = title_row_and_separator(
        &window,
        &nodes,
        &shapes,
        "701 /3/",
        crate::view::team::TEAM_LABEL,
    );
    assert!(
        (above - 11.5).abs() <= 1.0,
        "the title's ink {above} under the toolbar's band"
    );
    assert!(line.is_some(), "a separator under the title's row");
}

// With no save open the menus that act on one are drawn disabled: Player,
// Team and Savefile alike (SE-GUI-22's disabled-in-place rule). File and
// Edit stay enabled: inside them the opens are enabled, and Save, Save as,
// Undo and Redo disabled.
#[test]
fn the_menus_are_disabled_with_no_save_open() {
    let mut view = SaveView::new();
    view.discover = Box::new(|_| Vec::new());
    let mut window = Headless::new();
    let nodes = draw(&mut window, &mut view);
    for menu in ["Player", "Team", "Savefile"] {
        assert!(menu_button(&nodes, menu).is_disabled(), "{menu}");
    }
    for menu in ["File", "Edit"] {
        assert!(
            !menu_button(&nodes, menu).is_disabled(),
            "{menu} is enabled"
        );
    }
    let nodes = open_menu(&mut window, &mut view, "File", "Choose file");
    assert!(
        !menu_item(&nodes, "Choose file").is_disabled(),
        "the picker is enabled"
    );
    for entry in ["Save Ctrl+S", "Save as\u{2026}"] {
        assert!(
            node_labeled(&nodes, Role::Button, entry).is_disabled(),
            "{entry} is disabled"
        );
    }
    let nodes = open_menu(&mut window, &mut view, "Edit", "Undo");
    for entry in ["Undo Ctrl+Z", "Redo Ctrl+Y"] {
        assert!(
            node_labeled(&nodes, Role::Button, entry).is_disabled(),
            "{entry} is disabled"
        );
    }
}

/// Whether a filled or stroked rectangle the size of `node`'s bounds
/// (within the frame's stroke) is painted over it: the button's frame.
fn paints_frame(shapes: &[egui::epaint::ClippedShape], node: &accesskit::Node) -> bool {
    let bounds = node.bounds().expect("a drawn button has bounds");
    shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::Rect(rect) => {
            let same = (f64::from(rect.rect.left()) - bounds.x0).abs() <= 1.0
                && (f64::from(rect.rect.right()) - bounds.x1).abs() <= 1.0
                && (f64::from(rect.rect.top()) - bounds.y0).abs() <= 1.0
                && (f64::from(rect.rect.bottom()) - bounds.y1).abs() <= 1.0;
            let visible = rect.fill != egui::Color32::TRANSPARENT
                || (rect.stroke.width > 0.0 && rect.stroke.color != egui::Color32::TRANSPARENT);
            same && visible
        }
        _ => false,
    })
}

/// The bar's menus are unframed as a menu bar draws them; the ruleset
/// dropdown and the Live AATF toggle keep their frames in the framed
/// scope, a flat combo or toggle reading as one more menu or as a label.
#[test]
fn the_dropdown_and_the_toggle_paint_frames_where_the_menus_paint_none() {
    let path = fixture(PesVersion::Pes21, "toolbar_frames");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        paints_frame(&shapes, ruleset_dropdown(&nodes)),
        "the ruleset dropdown is framed"
    );
    assert!(
        paints_frame(
            &shapes,
            node_labeled(&nodes, Role::Button, "\u{25b6} Live AATF")
        ),
        "the Live AATF toggle is framed"
    );
    assert!(
        !paints_frame(&shapes, menu_button(&nodes, "File")),
        "the File menu button paints no frame"
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
