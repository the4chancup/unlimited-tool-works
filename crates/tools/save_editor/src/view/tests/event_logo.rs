//! The event logo's tests: where it stands at the default window, that it
//! moves no other widget, its tooltip and link, when it is not drawn, and
//! the toolbar's band ending short of it.

use std::time::SystemTime;

use pes_savefile::discovery::SavefileCandidate;

use super::*;

/// DBC7's ruleset as built in: its name and page are the logo's tooltip
/// and link.
fn dbc7() -> aatf::Ruleset {
    aatf::builtin("dbc7")
        .expect("DBC7 is built in")
        .ruleset()
        .clone()
}

/// DBC7's rules file with `edit` applied to its ruleset, as an imported
/// copy would hold it.
fn dbc7_file(edit: impl FnOnce(&mut aatf::Ruleset)) -> String {
    let mut ruleset = dbc7();
    edit(&mut ruleset);
    aatf::assemble_file(&ruleset, None, None)
}

/// A PES 21 save open in a window `width` points wide, then `pick` choosing
/// the ruleset, then three frames so every grid has measured itself, then
/// frames until the view's workers have replied.
fn opened_at(test: &str, width: f32, pick: impl FnOnce(&mut SaveView)) -> (SaveView, Headless) {
    let path = fixture(PesVersion::Pes21, test);
    let mut view = SaveView::new();
    view.open(&path);
    pick(&mut view);
    let mut window = Headless::sized(width, DEFAULT_PANE.y);
    for _ in 0..3 {
        draw(&mut window, &mut view);
    }
    // A window compared with another waits for every reply, or one may draw
    // the selected player's Cards used badge the other has not received yet.
    workers_replied(&mut window, &mut view);
    (view, window)
}

/// The logo's node: the picture labeled with the event's name.
fn logo<'a>(nodes: &'a [accesskit::Node], name: &str) -> Option<&'a accesskit::Node> {
    nodes
        .iter()
        .find(|node| node.role() == Role::Image && node.label() == Some(name))
}

/// Every placed node but the logo, each as its role, label, value and
/// bounds, sorted: two frames list the same widgets at the same places
/// exactly when these are equal. A node without bounds has no place (the
/// logo's own container is one).
fn layout_without_logo(nodes: &[accesskit::Node], name: &str) -> Vec<String> {
    let mut found: Vec<String> = nodes
        .iter()
        .filter(|node| !(node.role() == Role::Image && node.label() == Some(name)))
        .filter_map(|node| {
            let bounds = node.bounds()?;
            Some(format!(
                "{:?} {:?} {:?} {bounds:?}",
                node.role(),
                node.label(),
                node.value(),
            ))
        })
        .collect();
    found.sort();
    found
}

/// The x where the toolbar's last widget ends: the rightmost edge of the
/// nodes overlapping the File button's row, the logo left out.
fn toolbar_end(nodes: &[accesskit::Node], name: &str) -> f64 {
    let file = menu_button(nodes, "File")
        .bounds()
        .expect("the File menu button's bounds");
    nodes
        .iter()
        .filter(|node| !(node.role() == Role::Image && node.label() == Some(name)))
        .filter_map(accesskit::Node::bounds)
        .filter(|bounds| bounds.y0 < file.y1 && bounds.y1 > file.y0)
        .map(|bounds| bounds.x1)
        .fold(f64::MIN, f64::max)
}

/// The toolbar's band in the frame's shapes: the rect painted in the
/// panel's fill from the view's top (the headless window's 0) and less
/// than the logo's height, and the horizontal line in the panel's stroke
/// under it, each as its left and right x.
fn band(window: &Headless, shapes: &[egui::epaint::ClippedShape]) -> ((f32, f32), (f32, f32)) {
    let visuals = &window.ctx.global_style().visuals;
    let fill = shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect)
                if rect.fill == visuals.panel_fill
                    && rect.rect.top() == 0.0
                    && rect.rect.height() < 28.0 =>
            {
                Some((rect.rect.left(), rect.rect.right()))
            }
            _ => None,
        })
        .expect("the band's fill");
    let stroke = visuals.widgets.noninteractive.bg_stroke;
    let line = shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke: s }
                if *s == stroke && points[0].y == points[1].y && points[0].y < 28.0 =>
            {
                Some((points[0].x.min(points[1].x), points[0].x.max(points[1].x)))
            }
            _ => None,
        })
        .expect("the band's line");
    (fill, line)
}

// SE-GUI-44
#[test]
fn the_logo_stands_at_the_top_right_and_moves_nothing() {
    let name = dbc7().name;
    // Both windows hold DBC7 without its logo as an imported copy, so the
    // ruleset dropdown, sized by its list, is the same in both; this one
    // picks the built-in DBC7, the other the copy. Every built-in ruleset
    // carries a logo, Autumn 26 too, so none of them is the bare frame.
    let bare_file = dbc7_file(|ruleset| ruleset.logo = None);
    let (mut view, mut window) = opened_at("event_logo_place", DEFAULT_PANE.x, |view| {
        pick_imported(view, "dbc7.4ccrules", &bare_file);
        view.aatf_ruleset = "dbc7".to_owned();
    });
    let nodes = draw(&mut window, &mut view);
    let bounds = logo(&nodes, &name)
        .and_then(accesskit::Node::bounds)
        .expect("DBC7's logo is drawn");
    // The headless window is the shell's central panel: its right edge is
    // the window's, its top the view's.
    assert_eq!(bounds.y1 - bounds.y0, 28.0, "28 points high");
    assert_eq!(
        bounds.x1,
        f64::from(DEFAULT_PANE.x) - 14.0,
        "14 from the window's right"
    );
    assert_eq!(bounds.y0, 8.0, "8 below the view's top");

    // The same ruleset without its logo: every other widget where it was.
    let (mut bare, mut bare_window) = opened_at("event_logo_bare", DEFAULT_PANE.x, |view| {
        pick_imported(view, "dbc7.4ccrules", &bare_file);
    });
    let bare_nodes = draw(&mut bare_window, &mut bare);
    assert!(
        logo(&bare_nodes, &name).is_none(),
        "a ruleset without a logo draws none"
    );
    assert_eq!(
        layout_without_logo(&nodes, &name),
        layout_without_logo(&bare_nodes, &name)
    );
}

// A picture wider than 3:1 is drawn less than 28 high, in the middle of
// the 28-point box: a 4:1 one is 84 by 21, its top 3.5 below the box's.
#[test]
fn a_logo_wider_than_3_to_1_sits_in_the_middle_of_the_box() {
    let name = dbc7().name;
    let pixels = vec![200; 160 * 40 * 4];
    let png = dds_convert::encode_png(&pixels, 160, 40).expect("a plain picture encodes");
    let wide_file = dbc7_file(|ruleset| {
        ruleset.set_logo(&png).expect("a small PNG is a valid logo");
    });
    let (mut view, mut window) = opened_at("event_logo_wide", DEFAULT_PANE.x, |view| {
        pick_imported(view, "dbc7.4ccrules", &wide_file);
    });
    let bounds = logo(&draw(&mut window, &mut view), &name)
        .and_then(accesskit::Node::bounds)
        .expect("the wide logo is drawn");
    assert_eq!(bounds.x1 - bounds.x0, 84.0, "84 points wide");
    assert_eq!(bounds.y1 - bounds.y0, 21.0, "21 points high");
    assert_eq!(
        bounds.y0,
        8.0 + 3.5,
        "3.5 below the box's top, 8 below the view's"
    );
}

// SE-GUI-44
#[test]
fn the_logo_links_to_the_event_page() {
    let ruleset = dbc7();
    let page = ruleset.page.clone().expect("DBC7 carries a page");
    let (mut view, mut window) = opened_at("event_logo_link", DEFAULT_PANE.x, |view| {
        view.aatf_ruleset = "dbc7".to_owned();
    });
    let nodes = draw(&mut window, &mut view);
    let at = centre(logo(&nodes, &ruleset.name).expect("DBC7's logo is drawn"));

    assert_eq!(
        tooltip_containing(&mut window, &mut view, at, "Click here").as_deref(),
        Some(
            format!(
                "{}\n\nClick here to open the event's page:\n{page}",
                ruleset.name
            )
            .as_str()
        )
    );
    let (cursor, _, _) = window.frame(vec![egui::Event::PointerMoved(at)], |ui| {
        show(ui, &mut view, &|| Vec::new());
        ui.ctx().output(|output| output.cursor_icon)
    });
    assert_eq!(cursor, egui::CursorIcon::PointingHand);

    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (commands, _, _) = window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new());
        ui.ctx().output(|output| output.commands.clone())
    });
    assert!(
        commands.contains(&egui::OutputCommand::OpenUrl(egui::OpenUrl::new_tab(&page))),
        "the click opens DBC7's page: {commands:?}"
    );
}

#[test]
fn a_logo_without_a_page_names_the_event_and_opens_nothing() {
    let name = dbc7().name;
    let (mut view, mut window) = opened_at("event_logo_no_page", DEFAULT_PANE.x, |view| {
        pick_imported(
            view,
            "dbc7.4ccrules",
            &dbc7_file(|ruleset| ruleset.page = None),
        );
    });
    let nodes = draw(&mut window, &mut view);
    let at = centre(logo(&nodes, &name).expect("the logo is drawn"));

    assert_eq!(
        tooltip_containing(&mut window, &mut view, at, &name).as_deref(),
        Some(name.as_str())
    );
    let (cursor, _, _) = window.frame(vec![egui::Event::PointerMoved(at)], |ui| {
        show(ui, &mut view, &|| Vec::new());
        ui.ctx().output(|output| output.cursor_icon)
    });
    assert_eq!(cursor, egui::CursorIcon::Default);

    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (commands, _, _) = window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new());
        ui.ctx().output(|output| output.commands.clone())
    });
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, egui::OutputCommand::OpenUrl(_))),
        "a click opens nothing: {commands:?}"
    );
}

// SE-GUI-44
#[test]
fn the_band_ends_14_points_left_of_the_logo_with_the_fullest_row_inside_it() {
    let name = dbc7().name;
    let (mut view, mut window) = opened_at("event_logo_band", DEFAULT_PANE.x, |view| {
        view.aatf_ruleset = "dbc7".to_owned();
        view.live_aatf = true;
        view.live_suggestions = true;
    });
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    draw(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        label_value(&nodes, "\u{2022} unsaved changes"),
        "premise: the unsaved-changes marker is drawn"
    );
    node_labeled(&nodes, Role::CheckBox, "Show suggestions");
    let logo_left = logo(&nodes, &name)
        .and_then(accesskit::Node::bounds)
        .expect("DBC7's logo is drawn")
        .x0 as f32;
    let end = logo_left - 14.0;
    assert!(
        end < DEFAULT_PANE.x - 1.0,
        "premise: the band's end with the logo is not the view's edge"
    );
    let (fill, line) = band(&window, &shapes);
    assert_eq!(fill, (0.0, end), "the fill from the view's left edge");
    assert_eq!(line, (0.0, end), "the line from the view's left edge");
    assert!(
        toolbar_end(&nodes, &name) < f64::from(end),
        "the row's last widget ends inside the band ({} against {end})",
        toolbar_end(&nodes, &name)
    );
}

// A save the toolbar opens on a frame (a File menu entry or a shortcut)
// draws no logo on that frame, where the band's fill still ran to the
// view's right edge under it; on the next frame the logo stands over a
// band already ending 14 left of it.
#[test]
fn a_save_opened_from_the_toolbar_shows_the_logo_on_the_next_frame() {
    let name = dbc7().name;
    let path = fixture(PesVersion::Pes21, "event_logo_open_frame");
    let mut view = SaveView::new();
    view.aatf_ruleset = "dbc7".to_owned();
    let discovered = path.clone();
    view.discover = Box::new(move |_| {
        vec![SavefileCandidate {
            path: discovered.clone(),
            account: None,
            modified: SystemTime::now(),
        }]
    });
    let mut window = Headless::new();
    // One pass a frame: the fresh layout on the open's own frame asks for
    // a re-pass, whose output already holds the opened save, so with two
    // the pass under test is never returned.
    window
        .ctx
        .options_mut(|options| options.max_passes = std::num::NonZeroUsize::MIN);
    draw(&mut window, &mut view);
    assert!(view.session.is_none(), "premise: no save open");

    // Ctrl+1 opens the newest discovered PES 21 save on this frame.
    let (_, nodes, shapes) = window.frame(
        vec![key(egui::Key::Num1, egui::Modifiers::CTRL, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert!(view.session.is_some(), "premise: the shortcut opened it");
    if let Some(bounds) = logo(&nodes, &name).and_then(accesskit::Node::bounds) {
        let (fill, _) = band(&window, &shapes);
        assert_eq!(
            fill.1,
            bounds.x0 as f32 - 14.0,
            "a logo on the open's own frame stands over a band ending 14 left of it"
        );
    }
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = logo(&nodes, &name)
        .and_then(accesskit::Node::bounds)
        .expect("the logo is drawn on the next frame");
    let (fill, line) = band(&window, &shapes);
    assert_eq!(
        fill.1,
        bounds.x0 as f32 - 14.0,
        "the fill ends 14 left of the logo"
    );
    assert_eq!(
        line.1,
        bounds.x0 as f32 - 14.0,
        "the line ends 14 left of the logo"
    );
}

// SE-GUI-44
#[test]
fn without_a_logo_the_band_runs_to_the_views_right_edge() {
    let bare_file = dbc7_file(|ruleset| ruleset.logo = None);
    let (mut view, mut window) = opened_at("event_logo_band_bare", DEFAULT_PANE.x, |view| {
        pick_imported(view, "dbc7.4ccrules", &bare_file);
    });
    draw(&mut window, &mut view);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        logo(&nodes, &dbc7().name).is_none(),
        "premise: no logo is drawn"
    );
    let (fill, line) = band(&window, &shapes);
    assert_eq!(fill, (0.0, DEFAULT_PANE.x), "the fill to the view's edge");
    assert_eq!(line, (0.0, DEFAULT_PANE.x), "the line to the view's edge");
}
