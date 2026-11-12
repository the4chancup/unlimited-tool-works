use super::*;

#[test]
fn apply_stays_inside_the_window_with_every_note_group_open() {
    let (mut view, _) = open_on(PesVersion::Pes19, "interchange_notes_scroll", CO);
    // The default pane, where the real window's height bounds the panel.
    let mut window = Headless::new();
    let mut source = SaveView::new();
    source.open(&fixture(
        PesVersion::Pes21,
        "interchange_notes_scroll_source",
    ));
    picked(
        &mut window,
        &mut view,
        "co21.toml",
        export(&source, CO).as_bytes(),
    );
    open_note_groups(&mut window, &mut view);

    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let nodes = nodes_in_window(&pairs, TITLE);
    let notes: usize = nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::DisclosureTriangle)
        .map(|(_, node)| header_players(node.label().expect("a labeled header")))
        .sum();
    assert!(notes > 100, "premise: a long list of notes, {notes}");
    let bounds = |role: Role, label: &str| {
        nodes
            .iter()
            .find(|(_, node)| node.role() == role && node.label() == Some(label))
            .and_then(|(_, node)| node.bounds())
            .unwrap_or_else(|| panic!("the {label:?} {role:?}"))
    };
    let (panel, apply) = (bounds(Role::Window, TITLE), bounds(Role::Button, "Apply"));
    assert!(
        apply.y0 >= panel.y0 && apply.y1 <= panel.y1,
        "Apply {apply:?} inside the panel {panel:?}"
    );
    assert!(
        apply.y1 <= f64::from(DEFAULT_PANE.y),
        "Apply {apply:?} on the screen"
    );
}

#[test]
fn the_close_button_closes_the_panel() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_close", CO);
    open_panel(&mut window, &mut view);
    assert_eq!(view.panel, Some(Panel::Interchange), "premise: open");
    let close =
        panel_node(&mut window, &mut view, Role::Button, "Close window").expect("the close button");
    click(&mut window, &close, &mut view);
    assert_eq!(view.panel, None, "the panel closes");
}

#[test]
fn the_team_menu_ends_with_the_entry_that_opens_the_panel() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_menu", CO);
    open_menu(&mut window, &mut view, "Team", "Import / export team file");
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // The popup's buttons: the descendants of the node holding its first
    // entry, top to bottom.
    let (first, _) = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("Set edited flag"))
        .expect("the Team menu's first entry");
    let (popup, _) = pairs
        .iter()
        .find(|(_, node)| node.children().contains(first))
        .expect("the entry's container");
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        pairs.iter().map(|(id, node)| (*id, node)).collect();
    let mut buttons: Vec<(f64, String)> = Vec::new();
    let mut stack = vec![*popup];
    while let Some(id) = stack.pop() {
        let node = by_id[&id];
        stack.extend(node.children());
        if node.role() == Role::Button
            && let (Some(bounds), Some(label)) = (node.bounds(), node.label())
        {
            buttons.push((bounds.y0, label.to_owned()));
        }
    }
    buttons.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("drawn bounds"));
    let labels: Vec<&str> = buttons.iter().map(|(_, label)| label.as_str()).collect();
    // Prefixes: an entry's label carries its shortcut after the name.
    let entries = [
        "Set edited flag",
        "Set boots and gloves IDs for everyone\u{2026}",
        "Set player names to positions",
        "Clear visual flags",
        "Set FPC invisibility",
        "Remove FPC invisibility",
        "Import / export team file\u{2026}",
    ];
    assert!(
        labels.len() == entries.len()
            && labels
                .iter()
                .zip(entries)
                .all(|(label, entry)| label.starts_with(entry)),
        "the entries in order, the new one last: {labels:?}"
    );

    // The separator: a horizontal line across the popup between the FPC
    // entry and the new one.
    let nodes: Vec<accesskit::Node> = pairs.into_iter().map(|(_, node)| node).collect();
    let fpc = menu_item(&nodes, "Remove FPC invisibility")
        .bounds()
        .expect("drawn");
    let last = menu_item(&nodes, "Import / export team file")
        .bounds()
        .expect("drawn");
    let separated = shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::LineSegment { points, .. } => {
            let (y, left, right) = (
                f64::from(points[0].y),
                f64::from(points[0].x.min(points[1].x)),
                f64::from(points[0].x.max(points[1].x)),
            );
            points[0].y == points[1].y
                && y > fpc.y1
                && y < last.y0
                && left < last.x1
                && right > last.x0
        }
        _ => false,
    });
    assert!(separated, "a separator before the new entry");

    click(
        &mut window,
        menu_item(&nodes, "Import / export team file"),
        &mut view,
    );
    assert_eq!(view.panel, Some(Panel::Interchange), "the panel opens");
    assert!(
        panel_lines(&mut window, &mut view)
            .contains(&"Team TOML, .4ccs, .4cct and Texport files are accepted.".to_owned()),
        "the sentence before a pick"
    );
}

#[test]
fn the_panel_without_a_team_row_asks_for_one() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_no_team", CO);
    view.selected = None;
    view.panel = Some(Panel::Interchange);
    // A window's first frame only measures it.
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        panel_lines(&mut window, &mut view),
        ["Pick a team row to export or import."]
    );
}

#[test]
fn a_player_row_selected_makes_his_team_the_target() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_player_row", CO);
    view.selected = Some(Selection::Player(CO, 0));
    view.panel = Some(Panel::Interchange);
    // A window's first frame only measures it.
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let lines = panel_lines(&mut window, &mut view);
    assert!(
        lines.contains(&"Import into /co/ (714) ".to_owned()),
        "{lines:#?}"
    );
}

#[test]
fn one_floating_panel_at_a_time() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_one_panel", CO);
    let savefile = |window: &mut Headless, view: &mut SaveView, entry: &str| {
        let nodes = open_menu(window, view, "Savefile", entry);
        click(window, menu_item(&nodes, entry), view);
    };
    savefile(&mut window, &mut view, "Comparator\u{2026}");
    assert_eq!(
        view.panel,
        Some(Panel::Comparator),
        "premise: the comparator"
    );
    open_panel(&mut window, &mut view);
    assert_eq!(
        view.panel,
        Some(Panel::Interchange),
        "the comparator closes"
    );
    savefile(&mut window, &mut view, "Apply aesthetics patch\u{2026}");
    assert_eq!(view.panel, Some(Panel::Patch), "the team file panel closes");
    open_panel(&mut window, &mut view);
    assert_eq!(
        view.panel,
        Some(Panel::Interchange),
        "the patch panel closes"
    );
    savefile(&mut window, &mut view, "Comparator\u{2026}");
    assert_eq!(
        view.panel,
        Some(Panel::Comparator),
        "the team file panel closes"
    );
}
