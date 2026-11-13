use super::*;

// The Appearance section lays out the draft's two physique pairs a row
// at the default pane (1224 x 756) — Neck length and Neck size share a
// row's top, Shoulder height opens the next, and the seven left-column
// labels share one x0.
#[test]
fn the_physique_grid_holds_two_pairs_a_row() {
    let (mut view, _) = open_734_with("physique_two_pairs", &[], 0);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let first = label_bounds(&nodes, "Neck length").y0;
    assert!(
        (label_bounds(&nodes, "Neck size").y0 - first).abs() < 1.0,
        "Neck size shares the first row"
    );
    assert!(
        label_bounds(&nodes, "Shoulder height").y0 > first + 5.0,
        "Shoulder height opens the second row"
    );
    let x0 = label_bounds(&nodes, "Neck length").x0;
    for label in [
        "Shoulder height",
        "Chest",
        "Arm size",
        "Thigh",
        "Leg length",
        "Head width",
    ] {
        assert!(
            (label_bounds(&nodes, label).x0 - x0).abs() < 1.0,
            "{label} shares the left column's x0"
        );
    }
}

// The draft's strip table at the default pane (1224 x 756): four pairs
// a row — Boots ID, Gloves ID, Sleeves, Inners on one row, Socks,
// Undershorts and the two checkboxes on the next, the checkboxes a label
// cell and a bare box (the header's Captain shape); the wrist-taping,
// spectacles and gloves rows a second grid below, a label cell and a
// contents cell each.
#[test]
fn the_strip_groups_hold_the_drafts_rows() {
    let (mut view, _) = open_734_with("strip_grid_rows", &[], 0);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let untucked = |view: &SaveView| player_entry(view, 73401).appearance.untucked;
    assert!(untucked(&view), "premise: the shirt is out (untucked)");
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let first = label_bounds(&nodes, "Boots ID").y0;
    for label in ["Gloves ID", "Sleeves", "Inners"] {
        assert!(
            (label_bounds(&nodes, label).y0 - first).abs() < 1.0,
            "{label} shares the first row"
        );
    }
    let second = label_bounds(&nodes, "Socks").y0;
    assert!(second > first + 5.0, "the second row below the first");
    for label in ["Undershorts", "Shirttail out", "Ankle taping"] {
        assert!(
            (label_bounds(&nodes, label).y0 - second).abs() < 1.0,
            "{label} shares the second row"
        );
    }
    assert_eq!(
        label_bounds(&nodes, "Socks").x0,
        label_bounds(&nodes, "Boots ID").x0,
        "the rows' first label columns share an x0"
    );
    assert_eq!(
        label_bounds(&nodes, "Shirttail out").x0,
        label_bounds(&nodes, "Sleeves").x0,
        "the rows' third label columns share an x0"
    );

    // The second grid: the three rows' labels in one column, each below
    // the one before, all below the strip table's last row.
    let rows_x0 = label_bounds(&nodes, "Wrist taping").x0;
    let mut last_y = second;
    for label in ["Wrist taping", "Spectacles", "Player gloves"] {
        let bounds = label_bounds(&nodes, label);
        assert_eq!(bounds.x0, rows_x0, "{label} shares the grid's column");
        assert!(bounds.y0 > last_y + 5.0, "{label} sits below the row above");
        last_y = bounds.y0;
    }

    // Shirttail out's control is the bare checkbox right of its label
    // cell; a click writes `untucked` as one undoable edit. The row sits
    // below the fold: scroll until the clip covers it (its label's row: the
    // bare checkbox carries no label of its own).
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Shirttail out")
    });
    let shirttail = right_of_label(&nodes, "Shirttail out", Role::CheckBox)
        .first()
        .copied()
        .expect("the Shirttail out checkbox");
    click(&mut window, shirttail, &mut view);
    assert!(!untucked(&view), "the click stores untucked");
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
}

// The draft's motion rows at the default pane (1224 x 756, two pairs a
// row — `COL_PANELS` `[2, 194, 14]`): the labels take two x positions,
// the hunching pair the first row, the arm pair the second, the kicks
// the third, the penalty and dribbling motions the fourth, the
// celebrations the fifth.
#[test]
fn the_motion_grid_holds_two_pairs_a_row_in_two_columns() {
    let (mut view, _) = open_734_with("motion_two_pairs", &[], 0);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let labels = [
        "Hunching (dribbling)",
        "Hunching (running)",
        "Arm movement (dribbling)",
        "Arm movement (running)",
        "Corner kick",
        "Free kick",
        "Penalty kick",
        "Dribbling motion",
        "Goal celebration 1",
        "Goal celebration 2",
    ];
    let mut xs: Vec<f64> = labels
        .iter()
        .map(|label| label_bounds(&nodes, label).x0)
        .collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    assert_eq!(xs.len(), 2, "the motion labels take two x positions");
    // Row by row the draft's order: each row's pair shares one top.
    for (row, pair) in labels.chunks(2).enumerate() {
        let top = label_bounds(&nodes, pair[0]).y0;
        assert!(
            (label_bounds(&nodes, pair[1]).y0 - top).abs() < 1.0,
            "row {}: {:?} shares {:?}'s top",
            row + 1,
            pair[1],
            pair[0]
        );
        if row + 1 < labels.len() / 2 {
            assert!(
                label_bounds(&nodes, labels[2 * (row + 1)]).y0 > top + 5.0,
                "row {} opens below row {}",
                row + 2,
                row + 1
            );
        }
    }
}

// The same pane rule as the player view's
// (`spec/core/gui.md` "Shell layout"): on the pane the shell leaves at
// the 1280 x 780 window, the narrowest a member sees, since a narrower
// window scales the UI down, every drawn node ends inside it — on PES 17
// unlocked, and on PES 21 with the switch on.
#[test]
fn the_appearance_section_fits_the_pane_the_shell_leaves() {
    for (version, test) in [
        (PesVersion::Pes17, "appearance_pane_17"),
        (PesVersion::Pes21, "appearance_pane_21"),
    ] {
        let path = fixture(version, test);
        let mut view = SaveView::new();
        view.open(&path);
        view.aesthetics_from_exports = true;
        view.edit_aesthetics = true;
        if version == PesVersion::Pes17 {
            view.team = TeamChoice::Team(734);
            view.selected = Some(Selection::Player(734, 0));
        }
        let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
        open_appearance(&mut window, &mut view);
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        for node in &nodes {
            if let Some(bounds) = node.bounds() {
                assert!(
                    bounds.x1 <= f64::from(DEFAULT_PANE.x),
                    "{version:?}: {:?} {} clips the pane's right edge ({bounds:?})",
                    node.role(),
                    node.label().or(node.value()).unwrap_or("?")
                );
            }
        }
    }
}

/// One `tool.view` frame's nodes (the settings-menu path's tests draw the
/// real `Tool`, which refreshes the view's settings each frame).
fn tool_frame(
    window: &mut Headless,
    tool: &mut crate::Tool,
    ctx: &studio_core::ToolContext,
) -> Vec<accesskit::Node> {
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, ctx));
    nodes
}

/// `open_menu` for `tool.view` frames: the toolbar menu `menu`'s popup open
/// (skipped when a previous call left `item` drawn), then the frame's nodes.
fn open_tool_menu(
    window: &mut Headless,
    tool: &mut crate::Tool,
    ctx: &studio_core::ToolContext,
    menu: &str,
    item: &str,
) -> Vec<accesskit::Node> {
    let shows_item = |nodes: &[accesskit::Node]| {
        nodes.iter().any(|node| {
            node.role() == Role::Button && node.label().is_some_and(|l| l.starts_with(item))
        })
    };
    let nodes = tool_frame(window, tool, ctx);
    if shows_item(&nodes) {
        return nodes;
    }
    // The pointer onto the button first: while another menu is open that
    // opens this one, and a click would close it again (`open_menu`).
    let button = centre(menu_button(&nodes, menu));
    window.frame(vec![egui::Event::PointerMoved(button)], |ui| {
        tool.view(ui, ctx)
    });
    let nodes = tool_frame(window, tool, ctx);
    if shows_item(&nodes) {
        return nodes;
    }
    press_release(window, centre(menu_button(&nodes, menu)), |ui| {
        tool.view(ui, ctx)
    });
    tool_frame(window, tool, ctx)
}

// SE-GUI-35
#[test]
fn the_setting_off_draws_no_lock_and_the_setting_on_locks_the_paths() {
    // The default, a cup whose looks live in the save: no ownership line
    // and no switch, every path SE-GUI-22 names enabled, and an
    // Appearance edit landing like an Abilities one.
    let path = fixture(PesVersion::Pes21, "aesthetics_setting_default");
    let (ctx, _events_rx, _requests_rx) =
        tool_context(path.parent().unwrap(), studio_core::Settings::default());
    let mut tool = crate::Tool::new();
    tool.view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 1000.0);
    open_appearance(&mut window, &mut tool.view);

    let nodes = tool_frame(&mut window, &mut tool, &ctx);
    assert!(
        !nodes.iter().any(|node| {
            node.value()
                .is_some_and(|value| value.starts_with("The team's export owns"))
        }),
        "no ownership line under the default"
    );
    assert!(
        !nodes.iter().any(|node| {
            node.role() == Role::CheckBox && node.label() == Some("Edit aesthetics anyway")
        }),
        "no switch under the default"
    );
    let chest = right_of_label(&nodes, "Chest", Role::SpinButton)
        .first()
        .copied()
        .expect("the Chest control");
    assert!(!chest.is_disabled(), "the section's controls are enabled");

    let nodes = open_tool_menu(&mut window, &mut tool, &ctx, "Player", "Set all stats");
    assert!(
        !menu_item(&nodes, "Toggle FPC settings").is_disabled(),
        "the FPC toggle is enabled"
    );
    let nodes = open_tool_menu(&mut window, &mut tool, &ctx, "Team", "Set edited flag");
    for label in [
        "Set boots and gloves IDs for everyone",
        "Clear visual flags",
        "Hide default body (FPC)",
        "Show default body (FPC)",
    ] {
        assert!(
            !menu_item(&nodes, label).is_disabled(),
            "{label} is enabled"
        );
    }
    let nodes = open_tool_menu(&mut window, &mut tool, &ctx, "Savefile", "Fix database");
    assert!(
        !menu_item(&nodes, "Fix database").is_disabled(),
        "Fix database is enabled"
    );

    // The edit lands: premise Chest stores 7 (shown 0), typed 5 stores 12.
    // The row sits below the fold: scroll until the clip covers it.
    assert_eq!(
        tool.view
            .session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .appearance
            .chest,
        7,
        "premise: Chest stores 7 (shown 0)"
    );
    let nodes = scroll_into_view(&mut window, &mut tool.view, |node| {
        node.role() == Role::Label && node.value() == Some("Chest")
    });
    let chest = right_of_label(&nodes, "Chest", Role::SpinButton)
        .first()
        .copied()
        .expect("the Chest control");
    press_release(&mut window, centre(chest), |ui| tool.view(ui, &ctx));
    window.frame(vec![egui::Event::Text("5".to_owned())], |ui| {
        tool.view(ui, &ctx)
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| tool.view(ui, &ctx),
    );
    assert_eq!(
        tool.view
            .session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .appearance
            .chest,
        12,
        "typed 5 stores 12"
    );

    // The setting switched on in the settings menu: the next frame draws
    // the switch, off, and the paths disabled.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.settings_view(ui, &ctx));
    let checkbox = right_of_label(&nodes, "Aesthetics from exports", Role::CheckBox)
        .first()
        .copied()
        .expect("the setting's checkbox");
    assert_eq!(
        checkbox.toggled(),
        Some(accesskit::Toggled::False),
        "off by default"
    );
    // The checkbox's hover text is the setting's sentence.
    let at = centre(checkbox);
    window.frame(vec![egui::Event::PointerMoved(egui::Pos2::ZERO)], |ui| {
        tool.settings_view(ui, &ctx)
    });
    window.frame(vec![egui::Event::PointerMoved(at)], |ui| {
        tool.settings_view(ui, &ctx)
    });
    let mut tooltip = None;
    for _ in 0..90 {
        let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.settings_view(ui, &ctx));
        if let Some(value) = nodes
            .iter()
            .filter_map(|node| node.value())
            .find(|value| value.contains("the cup's looks are compiled from exports"))
        {
            tooltip = Some(value.to_owned());
            break;
        }
    }
    assert_eq!(
        tooltip.as_deref(),
        Some(
            "On when the cup's looks are compiled from exports: the save's aesthetics are \
             then read-only, behind Edit aesthetics anyway on the Appearance section."
        ),
        "the checkbox's hover text"
    );
    press_release(&mut window, at, |ui| tool.settings_view(ui, &ctx));
    let nodes = tool_frame(&mut window, &mut tool, &ctx);
    assert_eq!(
        edit_anyway(&nodes).toggled(),
        Some(accesskit::Toggled::False),
        "the switch drawn, off"
    );
    assert!(
        right_of_label(&nodes, "Chest", Role::SpinButton)[0].is_disabled(),
        "locked once the setting is on"
    );
    let nodes = open_tool_menu(&mut window, &mut tool, &ctx, "Team", "Set edited flag");
    assert!(
        menu_item(&nodes, "Clear visual flags").is_disabled(),
        "the lock's paths are disabled"
    );
}
