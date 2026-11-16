//! The toolbar's tests: its groups in order with a separator between each,
//! the menus switching on hover, the menus' entries grouped by what they
//! act on (no Panels menu), a shortcut right of its label, and the unsaved
//! marker in the link blue.

use super::changes_popup::{click_label, three_changes};
use super::file_menu::save_as_stub;
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

/// The row stands `BAND_MARGIN` inside its band (the draft's `.top`
/// padding): the File menu button 8 in from the left edge and 3 under the
/// band's top, the band's bottom line's top edge 3 under the button.
#[test]
fn the_toolbars_row_stands_inside_the_bands_margin() {
    let path = fixture(PesVersion::Pes21, "toolbar_band_margin");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let file = menu_button(&nodes, "File")
        .bounds()
        .expect("the File menu button's bounds");
    assert_eq!(file.x0, 8.0, "the File menu 8 in from the left edge");
    assert_eq!(file.y0, 3.0, "and 3 under the top");
    let stroke = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    // `band_bottom` is the line's bottom edge; its top is the band's.
    assert_eq!(
        band_bottom(&window, &nodes, &shapes) - f64::from(stroke.width),
        file.y1 + 3.0,
        "the band's bottom 3 under the row"
    );
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

// A shortcut pressed while a menu is open closes it, as a click on an
// entry does: every arm of the toolbar's close guard, each from a fresh
// scene with the Player menu up, the key's own premise set before it.
#[test]
fn a_shortcut_pressed_over_an_open_menu_closes_it() {
    /// 70101's height set: one undo step, the save dirty.
    fn edited(view: &mut SaveView) {
        let mut entry = player_entry(view, 70101);
        entry.basic.height = 170;
        view.session.as_mut().unwrap().edit_player(1, entry);
    }
    // The row's events as the window hears them, `build` having set the
    // key's premise (an edit, discovery stubbed, a player copied, the
    // Save as picker answered); `check`, the cases that carry one,
    // asserts the arm the key reached fired.
    type Check = Box<dyn FnOnce(&mut Headless, &mut SaveView)>;
    type Case = (
        &'static str,
        &'static dyn Fn(&mut Headless, &mut SaveView) -> (Vec<egui::Event>, Option<Check>),
    );
    let cases: Vec<Case> = vec![
        ("Ctrl+S", &|_, _| {
            (vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], None)
        }),
        ("Ctrl+Z", &|_, view| {
            edited(view);
            (vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], None)
        }),
        ("Ctrl+Y", &|_, view| {
            edited(view);
            assert!(
                view.session.as_mut().unwrap().undo(),
                "premise: the edit undid"
            );
            (vec![key(egui::Key::Y, egui::Modifiers::CTRL, true)], None)
        }),
        ("Ctrl+F", &|_, _| {
            (vec![key(egui::Key::F, egui::Modifiers::CTRL, true)], None)
        }),
        ("Ctrl+I", &|_, _| {
            (vec![key(egui::Key::I, egui::Modifiers::CTRL, true)], None)
        }),
        ("Ctrl+B", &|_, _| {
            (vec![key(egui::Key::B, egui::Modifiers::CTRL, true)], None)
        }),
        ("Ctrl+C", &|_, _| (vec![egui::Event::Copy], None)),
        ("Ctrl+9", &|_, view| {
            view.discover = Box::new(|_| Vec::new());
            view.open_dialog = Box::new(|_| None);
            (
                vec![key(egui::Key::Num9, egui::Modifiers::CTRL, true)],
                None,
            )
        }),
        ("Ctrl+V", &|window, view| {
            window.frame(vec![egui::Event::Copy], |ui| show(ui, view, &|| Vec::new()));
            let text = window
                .commands
                .iter()
                .find_map(|command| match command {
                    egui::OutputCommand::CopyText(text) => Some(text.clone()),
                    _ => None,
                })
                .expect("the copy put a player on the clipboard");
            (vec![egui::Event::Paste(text)], None)
        }),
        ("Page Down", &|_, _| {
            (
                vec![key(egui::Key::PageDown, egui::Modifiers::NONE, true)],
                None,
            )
        }),
        ("Ctrl+Shift+S", &|_, view| {
            let handed = save_as_stub(view, None);
            (
                vec![key(
                    egui::Key::S,
                    egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                    true,
                )],
                Some(Box::new(move |_, view: &mut SaveView| {
                    assert_eq!(
                        handed.borrow().as_slice(),
                        [view.save_path().expect("the save is open")],
                        "the key reached Save as's arm: its picker ran on the open file"
                    );
                })),
            )
        }),
        ("Ctrl+W", &|_, view| {
            edited(view);
            (
                vec![key(egui::Key::W, egui::Modifiers::CTRL, true)],
                Some(Box::new(|window: &mut Headless, view: &mut SaveView| {
                    assert!(
                        label_value(&draw(window, view), "Unsaved changes"),
                        "the key reached Close save's arm: the unsaved-changes prompt is up"
                    );
                })),
            )
        }),
    ];
    for (name, build) in cases {
        let path = fixture(PesVersion::Pes21, "shortcut_closes_menu");
        let mut view = SaveView::new();
        view.open(&path);
        let mut window = Headless::new();
        let (events, check) = build(&mut window, &mut view);
        open_menu(&mut window, &mut view, "Player", "Bump all stats");
        assert!(
            egui::Popup::is_any_open(&window.ctx),
            "premise: the Player menu is open"
        );

        window.frame(events, |ui| show(ui, &mut view, &|| Vec::new()));
        draw(&mut window, &mut view);
        if let Some(check) = check {
            check(&mut window, &mut view);
        }
        assert!(
            !egui::Popup::is_any_open(&window.ctx),
            "{name}: the menu closed on the shortcut's frame"
        );
    }
}

// The `• unsaved changes` list is the view's own popup, which egui's
// registry never sees: a shortcut closes it too — Ctrl+B with the list
// up raises the Bump dialog with nothing floating over it.
#[test]
fn a_shortcut_closes_the_unsaved_changes_list() {
    let (mut view, _) = three_changes("shortcut_closes_list");
    let mut window = Headless::new();
    click_label(&mut window, &mut view);
    assert!(view.changes_list.is_some(), "premise: the list is open");

    window.frame(vec![key(egui::Key::B, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(view.changes_list.is_none(), "the shortcut closed the list");
    assert!(
        view.stat_num.is_some(),
        "the Bump dialog opened: the shortcut took effect"
    );
}

// The tactics' slot popover is the other popup the view keeps outside
// egui's registry: a shortcut closes it too — Ctrl+S with an edit made
// writes the save and keeps the team row selected, nothing floating over
// the team view.
#[test]
fn a_shortcut_closes_the_slot_popover() {
    let path = fixture(PesVersion::Pes21, "shortcut_closes_popover");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 624.0);
    open_team_view(&mut window, &mut view, 701);

    // The popover opened on an outfield marker a click can reach (as the
    // list's page-keys test opens it).
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

    // The edit gives Ctrl+S something to write: it changes no selection.
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        view.tactics.popover_area().is_none(),
        "the shortcut closed the popover"
    );
    assert_eq!(
        view.selected,
        Some(Selection::Team(701)),
        "the team row kept the selection"
    );
}

/// No Panels menu; the Savefile menu holds the whole save's work grouped
/// by what it acts on: the Comparator and Apply aesthetics patch, a
/// separator, the roster sheet, the range's formation pictures and
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
            "Apply aesthetics patch\u{2026}",
            "Output rosters to TSV\u{2026}",
            "Save formation pictures for a range of teams\u{2026}",
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
    assert!(!separated(&entries, &lines, 3, 4), "the pictures with them");
    assert!(separated(&entries, &lines, 4, 5), "before Fix database");
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

// With a team row selected the Player menu reads the same — every entry,
// both separators and the Ctrl+B and Ctrl+I texts — the items acting on a
// player disabled, Copy/swap stats… enabled (it picks its players itself).
#[test]
fn the_player_menu_is_one_menu_on_a_player_or_a_team_row() {
    let path = fixture(PesVersion::Pes21, "player_menu_team");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();

    let (player_entries, player_lines) =
        menu_entries(&mut window, &mut view, "Player", "Set all stats");
    let nodes = draw(&mut window, &mut view);
    let labels: Vec<&str> = player_entries
        .iter()
        .map(|entry| entry.0.as_str())
        .collect();
    assert_eq!(
        labels,
        [
            "Set all stats\u{2026}",
            "Bump all stats\u{2026} Ctrl+B",
            "Hide default body (Enable FPC) Ctrl+I",
            "Copy/swap stats\u{2026}",
            "Generate settings.toml\u{2026}",
            "Generate in-game face\u{2026}",
        ],
        "the menu on a player row"
    );
    assert_eq!(
        player_lines.len(),
        2,
        "the separators around Copy/swap stats"
    );
    for (label, _, _) in &player_entries {
        assert!(
            !node_labeled(&nodes, Role::Button, label).is_disabled(),
            "{label} is enabled on a player row"
        );
    }

    // The menu closed first: a click reaching the list through the open
    // popup would not hit the team row.
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    open_team_view(&mut window, &mut view, 701);
    let (team_entries, team_lines) =
        menu_entries(&mut window, &mut view, "Player", "Set all stats");
    let nodes = draw(&mut window, &mut view);
    let team_labels: Vec<&str> = team_entries.iter().map(|entry| entry.0.as_str()).collect();
    assert_eq!(team_labels, labels, "the same entries on a team row");
    assert_eq!(team_lines.len(), player_lines.len(), "the same separators");
    for (index, label) in labels.iter().enumerate().take(labels.len() - 1) {
        assert_eq!(
            separated(&team_entries, &team_lines, index, index + 1),
            separated(&player_entries, &player_lines, index, index + 1),
            "the separator after {label:?} stands the same"
        );
    }
    for label in [
        "Set all stats\u{2026}",
        "Bump all stats\u{2026} Ctrl+B",
        "Hide default body (Enable FPC) Ctrl+I",
        "Generate settings.toml\u{2026}",
        "Generate in-game face\u{2026}",
    ] {
        assert!(
            node_labeled(&nodes, Role::Button, label).is_disabled(),
            "{label} is disabled on a team row"
        );
    }
    assert!(
        !node_labeled(&nodes, Role::Button, "Copy/swap stats\u{2026}").is_disabled(),
        "Copy/swap stats stays enabled"
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
/// id and name's ink about 10 below it (the draft's `.pheader`
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
        (above - 10.0).abs() <= 1.0,
        "the title's ink {above} under the toolbar's band"
    );
    assert!(line.is_some(), "a separator under the title's row");
}

/// The team view's title's row holds `<id> <name>` after the toggle,
/// team 701's `701 /3/` in the PES 21 fixture, the id in the weak colour,
/// its ink about 10 below the toolbar's band, the separator under the
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
        (above - 10.0).abs() <= 1.0,
        "the title's ink {above} under the toolbar's band"
    );
    assert!(line.is_some(), "a separator under the title's row");
}

// With no save open the menus that act on one are drawn disabled: Player,
// Team and Savefile alike (SE-GUI-22's disabled-in-place rule). File and
// Edit stay enabled: inside them the opens are enabled, and Save, Save as,
// Undo and Redo disabled.
// SE-GUI-26
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
    let nodes = open_menu(&mut window, &mut view, "File", "Open\u{2026}");
    assert!(
        !menu_item(&nodes, "Open\u{2026}").is_disabled(),
        "the picker is enabled"
    );
    for entry in [
        "Save Ctrl+S",
        "Save as\u{2026} Ctrl+Shift+S",
        "Close save Ctrl+W",
    ] {
        assert!(
            node_labeled(&nodes, Role::Button, entry).is_disabled(),
            "{entry} is disabled"
        );
    }
    assert!(!menu_item(&nodes, "Exit").is_disabled(), "Exit is enabled");
    let nodes = open_menu(&mut window, &mut view, "Edit", "Undo");
    for entry in ["Undo Ctrl+Z", "Redo Ctrl+Y"] {
        assert!(
            node_labeled(&nodes, Role::Button, entry).is_disabled(),
            "{entry} is disabled"
        );
    }
}

// With a save open but no row selected (the cup's teams all outside the
// remembered team category), the Player menu is disabled as a whole, as
// the Team menu is: every entry acts on the selected row's team, so on
// no row there is nothing to act on.
#[test]
fn the_player_menu_is_disabled_with_no_row_selected() {
    let path = fixture(PesVersion::Pes21, "player_menu_no_selection");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = None;
    let mut window = Headless::new();
    let nodes = draw(&mut window, &mut view);
    for menu in ["Player", "Team"] {
        assert!(
            menu_button(&nodes, menu).is_disabled(),
            "{menu} is disabled with no row selected"
        );
    }
    assert!(
        !menu_button(&nodes, "Savefile").is_disabled(),
        "the whole save's menu stays enabled"
    );
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

/// Whether a visibly stroked rect is painted inside `bounds` (a widget's
/// outline: the bar's menu style strips them, the framed scope keeps
/// them).
fn paints_stroke(shapes: &[egui::epaint::ClippedShape], bounds: accesskit::Rect) -> bool {
    shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::Rect(rect) => {
            !rect.stroke.is_empty()
                && f64::from(rect.rect.min.x) >= bounds.x0 - 0.5
                && f64::from(rect.rect.min.y) >= bounds.y0 - 0.5
                && f64::from(rect.rect.max.x) <= bounds.x1 + 0.5
                && f64::from(rect.rect.max.y) <= bounds.y1 + 0.5
        }
        _ => false,
    })
}

/// The bar's menus are unframed as a menu bar draws them; the ruleset
/// dropdown and the Live AATF toggle keep their frames in the framed
/// scope, a flat combo or toggle reading as one more menu or as a label;
/// and under the pointer the `Show suggestions` box keeps its outline.
#[test]
fn the_dropdown_and_the_toggle_paint_frames_where_the_menus_paint_none() {
    let path = fixture(PesVersion::Pes21, "toolbar_frames");
    let mut view = SaveView::new();
    view.open(&path);
    view.live_aatf = true;
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

    // Live AATF on: `Show suggestions` is in the framed scope too, so its
    // unchecked box shows the pointer an outline.
    let suggestions = node_labeled(&nodes, Role::CheckBox, "Show suggestions");
    let at = centre(suggestions);
    window.frame(vec![egui::Event::PointerMoved(at)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    // egui styles the box from its state on the frame before.
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        paints_stroke(&shapes, suggestions.bounds().expect("bounds")),
        "the unchecked box's outline under the pointer"
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
