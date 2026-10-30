use super::*;

/// The painted TextShape of `label` inside `bounds`' y range (a button's
/// own galley, not another label's on the same line).
fn painted_label<'a>(
    shapes: &'a [egui::epaint::ClippedShape],
    label: &str,
    bounds: accesskit::Rect,
) -> &'a egui::epaint::TextShape {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape)
                if shape.galley.job.text == label
                    && f64::from(shape.pos.y) >= bounds.y0
                    && f64::from(shape.pos.y) <= bounds.y1 =>
            {
                Some(shape)
            }
            _ => None,
        })
        .next()
        .unwrap_or_else(|| panic!("no painted {label:?} inside {bounds:?}"))
}

/// The two columns' natural widths for `left`'s and `right`'s labels: each
/// its widest cell's, a label plus the button padding. Computed here from
/// the labels alone, never the code's own function. `right`'s last is the
/// Set stats cell (its button, the 8-point gap and the number's 52-point
/// box).
fn natural_columns(window: &Headless, left: &[&str], right: &[&str]) -> [f64; 2] {
    let style = window.ctx.global_style();
    let button = egui::TextStyle::Button.resolve(&style);
    let padding = 2.0 * f64::from(style.spacing.button_padding.x);
    let natural = |text: &str, font: &egui::FontId| {
        window.ctx.fonts_mut(|fonts| {
            f64::from(
                fonts
                    .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
                    .size()
                    .x,
            )
        })
    };
    let widest = |labels: &[&str]| {
        labels
            .iter()
            .map(|&label| {
                if label == "Set stats" {
                    natural("Set stats", &button) + padding + 8.0 + 52.0
                } else {
                    natural(label, &button) + padding
                }
            })
            .fold(0.0_f64, f64::max)
    };
    [widest(left), widest(right)]
}

/// The quick actions are `left`'s labels down the left column and
/// `right`'s down the right, in the given order — each column's cells
/// sharing one x inside the list rows' x range, every cell
/// `QUICK_ACTION_HEIGHT` (20 points) tall and below the list's drawn rows,
/// the Set stats row the right column's last at the buttons' own pitch.
/// `shapes` is the frame's painted shapes: every cell is half the grid's
/// band less the gap, the label eliding only when the cell cannot hold
/// it.
fn assert_quick_actions(
    window: &Headless,
    nodes: &[accesskit::Node],
    shapes: &[egui::epaint::ClippedShape],
    left: &[&str],
    right: &[&str],
) {
    // A player row's bounds stand for the list's x range: the buttons
    // sit inside it and below the list's last drawn row.
    let row = nodes
        .iter()
        .find(|node| player_row(node))
        .and_then(accesskit::Node::bounds)
        .expect("a player row in the list");
    // Every button fills its column's width, half the grid's band less
    // the gap, its label eliding only when it cannot fit. The band ends
    // at the list's frame.
    let style = window.ctx.global_style();
    let half = (grid_band_right(window, shapes) - row.x0 - 8.0) / 2.0;
    let columns = [half, half];
    let ctx = &window.ctx;
    let mut column_xs = Vec::new();
    for (labels, cell) in [(left, columns[0]), (right, columns[1])] {
        assert!(!labels.is_empty(), "a column is expected");
        let col_x = node_labeled(nodes, Role::Button, labels[0])
            .bounds()
            .expect("bounds")
            .x0;
        column_xs.push(col_x);
        let mut last_y = f64::NEG_INFINITY;
        let mut last_gap = None;
        for label in labels {
            let button = node_labeled(nodes, Role::Button, label);
            let bounds = button.bounds().expect("bounds");
            assert!(
                (bounds.x0 - col_x).abs() <= 0.5,
                "{label} shares the column's x ({bounds:?} vs {col_x})"
            );
            assert!(
                bounds.y0 > last_y + 5.0,
                "{label} below the cell before it ({bounds:?} vs {last_y})"
            );
            if last_y > f64::NEG_INFINITY {
                let pitch = bounds.y0 - last_y;
                if let Some(gap) = last_gap.replace(pitch) {
                    assert!(
                        (gap - pitch).abs() < 0.5,
                        "{label} at the buttons' own pitch"
                    );
                }
            }
            last_y = bounds.y0;
            assert!(
                (bounds.y1 - bounds.y0 - 20.0).abs() < 0.5,
                "{label} is 20 points tall ({bounds:?})"
            );
            // The Set stats cell holds the button at its own width then
            // its field; its span is the cell's.
            let span = if *label == "Set stats" {
                let f = set_stats_field(nodes).bounds().expect("bounds");
                f.x1 - bounds.x0
            } else {
                bounds.x1 - bounds.x0
            };
            assert!(
                (span - cell).abs() <= 1.0,
                "{label}'s cell fills its column ({bounds:?} vs cell {cell})"
            );
            let shape = painted_label(shapes, label, bounds);
            let font = shape
                .galley
                .job
                .sections
                .first()
                .map(|section| section.format.font_id.clone())
                .expect("a painted label has a section");
            let natural = ctx.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap((*label).to_owned(), font, egui::Color32::WHITE)
                    .size()
                    .x
            });
            // Set stats' button is drawn the cell less the 8-point gap and
            // the number's 52-point box (the draft's number input), not the
            // whole cell as a Make button is.
            let room = if *label == "Set stats" {
                cell - 8.0 - 52.0
            } else {
                cell
            };
            let elides =
                f64::from(natural) + 2.0 * f64::from(style.spacing.button_padding.x) > room + 0.5;
            assert_eq!(
                shape.galley.elided, elides,
                "{label}'s galley elided={} ({bounds:?}, cell {cell})",
                shape.galley.elided
            );
        }
    }
    column_xs.sort_by(f64::total_cmp);
    column_xs.dedup();
    assert_eq!(column_xs.len(), 2, "the two columns take two x positions");
    assert!(
        (column_xs[0] - row.x0).abs() <= 1.0,
        "the first column starts at the rows' left edge ({column_xs:?} vs {row:?})"
    );
    // Every cell inside the grid's band.
    let band_right = grid_band_right(window, shapes);
    for label in left.iter().chain(right.iter()) {
        let bounds = node_labeled(nodes, Role::Button, label)
            .bounds()
            .expect("bounds");
        assert!(
            bounds.x0 >= row.x0 - 1.0 && bounds.x1 <= band_right + 1.0,
            "{label} inside the grid's band ({bounds:?} vs {band_right})"
        );
    }
    // The Set stats number stays inside the list's width with its
    // button (it takes the cell's remaining width), and it is 20 tall.
    let field = set_stats_field(nodes).bounds().expect("bounds");
    assert!(
        field.x1 <= band_right + 1.0,
        "the Set stats field inside the grid's band ({field:?} vs {band_right})"
    );
    assert!(
        (field.y1 - field.y0 - 20.0).abs() < 0.5,
        "the Set stats field is 20 points tall ({field:?})"
    );
    let first_y0 = node_labeled(nodes, Role::Button, left[0])
        .bounds()
        .expect("bounds")
        .y0;
    // The list's scroll clip's bottom: the leftmost ScrollBar's y1 (the
    // bottom panel takes the height off it).
    let clip_bottom = nodes
        .iter()
        .filter(|node| node.role() == Role::ScrollBar)
        .filter_map(|node| node.bounds())
        .min_by(|a, b| a.x0.partial_cmp(&b.x0).expect("ordered bounds"))
        .map(|bounds| bounds.y1)
        .expect("the list's scroll bar marks its clip's bottom");
    assert!(
        first_y0 >= clip_bottom - 1.0,
        "the grid starts below the list's scroll clip ({first_y0} vs {clip_bottom})"
    );
}

// The grid spans the list's frame, the draft's `1fr 1fr`: at the default
// width each column is a half of the 245-point frame less the 8-point gap,
// 118.5, the left column's buttons starting at the frame's left edge and
// the right column's cells (Set stats' button and box together) ending at
// its right edge, not at the scroll bar beside it.
#[test]
fn the_quick_actions_columns_are_halves_of_the_lists_frame() {
    let (_path, mut view, mut window) = open_701("quick_halves");
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let frame = list_frame(&window, &shapes);
    assert!(
        (frame.width() - 245.0).abs() <= 1.0,
        "premise: the default 245-point frame ({frame:?})"
    );
    let span = |label: &str| {
        let bounds = node_labeled(&nodes, Role::Button, label)
            .bounds()
            .expect("bounds");
        let right = if label == "Set stats" {
            set_stats_field(&nodes).bounds().expect("bounds").x1
        } else {
            bounds.x1
        };
        (bounds.x0, right)
    };
    for label in ["Make Gold", "Make Silver", "Make Bronze"] {
        let (left, right) = span(label);
        assert!(
            (left - f64::from(frame.min.x)).abs() <= 1.0,
            "{label} starts at the frame's left ({left} vs {frame:?})"
        );
        assert!(
            (right - left - 118.5).abs() <= 1.0,
            "{label} is 118.5 wide ({left} to {right})"
        );
    }
    for label in ["Make Goalkeeper", "Make Regular", "Set stats"] {
        let (left, right) = span(label);
        assert!(
            (right - f64::from(frame.max.x)).abs() <= 1.0,
            "{label} ends at the frame's right ({right} vs {frame:?})"
        );
        assert!(
            (right - left - 118.5).abs() <= 1.0,
            "{label} is 118.5 wide ({left} to {right})"
        );
    }
}

/// Label nodes inside the side panel (left of the detail pane's edge):
/// the quick actions' band's load error standing in for the Make buttons.
/// The detail view's own labels sit at its x and never pass.
/// The quick actions grid's right edge: the player list's frame's. The
/// grid ends where the list's rows do, not at the scroll bar beside them.
fn grid_band_right(window: &Headless, shapes: &[egui::epaint::ClippedShape]) -> f64 {
    f64::from(list_frame(window, shapes).max.x)
}

/// The Set stats field: the row's one number control, right of the button.
fn set_stats_field(nodes: &[accesskit::Node]) -> &accesskit::Node {
    right_of(
        nodes,
        node_labeled(nodes, Role::Button, "Set stats"),
        Role::SpinButton,
    )
}

/// `text` typed into `field`: a click to focus it, select all, the
/// characters, then Enter to commit — the crate's number-typing gesture.
fn type_number(window: &mut Headless, view: &mut SaveView, field: &accesskit::Node, text: &str) {
    click(window, field, view);
    window.frame(
        vec![
            key(egui::Key::A, egui::Modifiers::COMMAND, true),
            egui::Event::Text(text.to_owned()),
        ],
        |ui| show(ui, view, &|| Vec::new()),
    );
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, view, &|| Vec::new()),
    );
}

/// `rules`' `apply_tier` on the roster slot's player of `view`'s save, as the
/// click should leave his entry.
fn tiered(view: &SaveView, rules: &aatf::Rules, team_id: u32, slot: u8, tier: &str) -> PlayerEntry {
    let file = view.session.as_ref().unwrap().file();
    let team = aatf::TeamView::new(file, team_id).unwrap();
    let player = team.player(slot).unwrap();
    let id = file.team(team_id).unwrap().roster[usize::from(slot)].player_id;
    let mut entry = file.player(id).unwrap().clone();
    rules.apply_tier(&team, player, tier, &mut entry).unwrap();
    entry
}

/// `test`'s fixture save's path, opened with team 701's slot 0 selected, the
/// window at the tests' usual size.
fn open_701(test: &str) -> (PathBuf, SaveView, Headless) {
    let path = fixture(PesVersion::Pes21, test);
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    (path, view, Headless::sized(DEFAULT_PANE.x, 700.0))
}

// The quick actions under the list on either selection: on a player row
// they act on him; on a team row every control draws disabled with the
// reason on hover, the Player menu's entries the same, and Ctrl+I and
// Ctrl+B do nothing. There is no "Quick actions" heading and no `AATF…`
// button (Live AATF is the window's check).
// SE-GUI-37
#[test]
fn the_quick_actions_and_player_menu_disable_on_a_team_row() {
    let (_path, mut view, mut window) = open_701("quick_on_team_row");

    /// No node of the frame carries the tool's name: the window's title
    /// bar carries it.
    fn draws_tool_name(nodes: &[accesskit::Node]) -> bool {
        nodes.iter().any(|node| {
            node.label().is_some_and(|l| l.contains("Save editor"))
                || node.value().is_some_and(|v| v.contains("Save editor"))
        })
    }

    // A player row: the controls are enabled.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        !nodes
            .iter()
            .any(|node| { node.role() == Role::Label && node.value() == Some("Quick actions") }),
        "no heading"
    );
    assert!(!draws_tool_name(&nodes), "no view draws \"Save editor\"");
    for label in ["Make Gold", "Set stats"] {
        assert!(
            !node_labeled(&nodes, Role::Button, label).is_disabled(),
            "{label} enabled on a player row"
        );
    }
    assert!(
        nodes
            .iter()
            .all(|node| node.label() != Some("AATF\u{2026}")),
        "no AATF button"
    );

    // The team row: every control disabled with the tooltip, and the
    // Player menu's entries the same.
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        !draws_tool_name(&nodes),
        "the team view draws no \"Save editor\" either"
    );
    let tip = "The make buttons act on a player: pick a player row";
    for label in ["Make Gold", "Make Regular", "Set stats"] {
        let button = node_labeled(&nodes, Role::Button, label);
        assert!(button.is_disabled(), "{label} disabled on a team row");
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(button), "make buttons"),
            Some(tip.to_string()),
            "{label}'s tooltip"
        );
    }
    let field = set_stats_field(&nodes);
    assert!(field.is_disabled(), "the number disabled too");
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(field), "make buttons"),
        Some(tip.to_string()),
        "the number's tooltip"
    );

    // Ctrl+I and Ctrl+B act on a player, so on a team row they change no
    // player and open no dialog.
    let players = |view: &SaveView| {
        let file = view.session.as_ref().unwrap().file();
        file.team(701)
            .unwrap()
            .roster
            .iter()
            .filter_map(|slot| file.player(slot.player_id).cloned())
            .collect::<Vec<PlayerEntry>>()
    };
    let before = players(&view);
    assert!(!view.aesthetics_locked(), "premise: Ctrl+I is not locked");
    for shortcut in [egui::Key::I, egui::Key::B] {
        window.frame(vec![key(shortcut, egui::Modifiers::CTRL, true)], |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
    }
    assert_eq!(players(&view), before, "no player changed");
    assert!(!view.session.as_ref().unwrap().is_dirty(), "nothing edited");
    assert!(view.stat_num.is_none(), "no Bump dialog");

    let nodes = open_menu(&mut window, &mut view, "Player", "Set all stats");
    for label in [
        "Set all stats",
        "Bump all stats",
        "Toggle FPC settings",
        "Copy stats from",
        "Swap stats with",
        "Generate settings.toml",
        "Generate in-game face",
    ] {
        let item = menu_item(&nodes, label);
        assert!(item.is_disabled(), "{label} disabled on a team row");
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(item), "acts on a player"),
            Some("The Player menu acts on a player: pick a player row".to_string()),
            "{label}'s tooltip"
        );
    }
}

// SE-GUI-09
#[test]
fn the_quick_actions_apply_a_tier_as_one_undoable_edit() {
    let (_path, mut view, mut window) = open_701("quick_make_gold");
    let rules = aatf::Rules::official();

    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // The medal tiers down the left column in the ruleset's order, the
    // others then Set stats down the right.
    assert_quick_actions(
        &window,
        &nodes,
        &shapes,
        &["Make Gold", "Make Silver", "Make Bronze"],
        &["Make Goalkeeper", "Make Regular", "Set stats"],
    );

    // Make Gold: his entry is the lib's `apply_tier` run on a clone of it.
    let before = player_entry(&view, 70101);
    let expected = tiered(&view, &rules, 701, 0, "gold");
    assert_ne!(expected, before, "premise: Gold changes him");
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Make Gold"),
        &mut view,
    );
    assert_eq!(player_entry(&view, 70101), expected, "Gold's values");

    // check_team then reports no Stat-family violation on his slot.
    let team = aatf::TeamView::new(view.session.as_ref().unwrap().file(), 701).unwrap();
    let violations = rules.check_team(&team).unwrap();
    let own: Vec<&aatf::Violation> = violations
        .iter()
        .filter(|violation| {
            violation.slot == Some(0)
                && matches!(
                    violation.kind,
                    aatf::ViolationKind::Stat { .. }
                        | aatf::ViolationKind::StatTotal { .. }
                        | aatf::ViolationKind::Form { .. }
                        | aatf::ViolationKind::InjuryResistance { .. }
                        | aatf::ViolationKind::WeakFootUsage { .. }
                        | aatf::ViolationKind::WeakFootAccuracy { .. }
                        | aatf::ViolationKind::NameColor { .. }
                )
        })
        .collect();
    assert!(
        own.is_empty(),
        "no violation of what the tier sets on his slot: {own:?}"
    );

    // One Ctrl+Z restores the old entry.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        player_entry(&view, 70101),
        before,
        "the one undo restores it"
    );
    assert!(!view.session.as_ref().unwrap().can_undo(), "one entry only");
}

// SE-GUI-09
#[test]
fn the_quick_actions_follow_the_rulesets_tiers() {
    let (_path, mut view, mut window) = open_701("quick_vtl11");
    let rules = aatf::builtin("vtl11").expect("VTL11 is built in");
    view.aatf_ruleset = "vtl11".to_owned();

    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // VTL11's medal tiers are Gold and Silver (no bronze): Regular and
    // Buffed then Set stats down the right column.
    assert_quick_actions(
        &window,
        &nodes,
        &shapes,
        &["Make Gold", "Make Silver"],
        &["Make Regular", "Make Buffed", "Set stats"],
    );

    // Make Silver gives him VTL11's Silver values.
    let before = player_entry(&view, 70101);
    let expected = tiered(&view, &rules, 701, 0, "silver");
    assert_ne!(expected, before, "premise: Silver changes him");
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Make Silver"),
        &mut view,
    );
    let after = player_entry(&view, 70101);
    assert_eq!(after, expected, "VTL11's Silver values");
    // `speed` is no `stats` override of Silver's: its target is the tier's rate, 89.
    assert_eq!(after.stats.speed, 89);
}

// SE-GUI-09, its Set stats clause: Set stats is the Player menu's Set all
// stats in one click, its number 40 to 99 starting at 77.
#[test]
fn set_stats_writes_every_ability_to_the_number() {
    // Speed moved off the fixture's 77, so the click is a real write and the
    // undo has something to restore.
    let path = fixture(PesVersion::Pes21, "quick_set_stats");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().stats.speed = 60;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let before = player_entry(&view, 70101);
    assert_eq!(before.stats.speed, 60, "premise: the moved stat");
    assert_eq!(view.set_stats, 77, "the number starts at 77");

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Set stats"),
        &mut view,
    );
    {
        let stats = &player_entry(&view, 70101).stats;
        for (index, ability) in stats.abilities().iter().enumerate() {
            assert_eq!(*ability, Some(77), "ability {index} at the starting 77");
        }
        assert_eq!(stats.weak_foot_usage, before.stats.weak_foot_usage);
        assert_eq!(stats.weak_foot_accuracy, before.stats.weak_foot_accuracy);
        assert_eq!(stats.form, before.stats.form);
        assert_eq!(stats.injury_resistance, before.stats.injury_resistance);
    }

    // One Ctrl+Z restores the entry.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        player_entry(&view, 70101),
        before,
        "the one undo restores it"
    );

    // A new number typed into the row's field is the value written.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let field = set_stats_field(&nodes);
    type_number(&mut window, &mut view, field, "85");
    assert_eq!(view.set_stats, 85, "the field holds the typed 85");
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Set stats"),
        &mut view,
    );
    for (index, ability) in player_entry(&view, 70101)
        .stats
        .abilities()
        .iter()
        .enumerate()
    {
        assert_eq!(*ability, Some(85), "ability {index} at the typed 85");
    }

    // The field's 40 to 99 range settles a typed number past its top.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let field = set_stats_field(&nodes);
    type_number(&mut window, &mut view, field, "120");
    assert_eq!(view.set_stats, 99, "a typed 120 settles at 99");
}

// An `apply_tier` that fails leaves the player as he was and shows the
// error in the refusal dialog.
#[test]
fn a_failed_make_shows_the_refusal_and_changes_nothing() {
    let (_path, mut view, mut window) = open_701("quick_make_fails");
    // The smallest file whose `tier_values` throws for one tier: the official
    // file assembled, with a `throw` added at the function's head for "silver"
    // alone.
    let text = aatf::assemble_file(aatf::Rules::official().ruleset(), None, None).replacen(
        "fn tier_values(player, tier, team) {",
        "fn tier_values(player, tier, team) {\n    if tier == \"silver\" { throw \"a staged failure\"; }",
        1,
    );
    pick_imported(&mut view, "broken_tier.4ccrules", &text);
    let before = player_entry(&view, 70101);

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Make Silver"),
        &mut view,
    );
    assert_eq!(
        player_entry(&view, 70101),
        before,
        "the player is unchanged"
    );
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "nothing on the undo stack"
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Cannot apply the tier")),
        "the refusal dialog's heading"
    );
    assert!(
        nodes.iter().any(|node| {
            node.value()
                .is_some_and(|value| value.contains("a staged failure"))
        }),
        "the error's text"
    );
}

/// The quick actions follow the `aatf_ruleset` setting as the tool reads
/// it: VTL11's tier buttons while it names `vtl11`, the official ones once
/// it names `autumn26` again.
// SE-GUI-32
#[test]
fn the_quick_actions_follow_the_aatf_ruleset_setting() {
    use studio_core::StudioTool;

    let save_path = fixture(PesVersion::Pes21, "settings_ruleset_key");
    let dir = save_path.parent().unwrap().to_path_buf();
    let mut tool = crate::Tool::new();
    tool.view.open(&save_path);
    let mut settings = studio_core::Settings::default();
    let mut table = tool.default_settings();
    table.insert(
        "aatf_ruleset".to_owned(),
        toml::Value::String("vtl11".to_owned()),
    );
    settings.set_tool("save-editor", table);
    let (ctx, _events_rx, _requests_rx) = tool_context(&dir, settings);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);

    // The grid's first frame is the invisible sizing pass: a settle frame
    // so the cells' measured widths land before asserting.
    window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert_quick_actions(
        &window,
        &nodes,
        &shapes,
        &["Make Gold", "Make Silver"],
        &["Make Regular", "Make Buffed", "Set stats"],
    );

    let mut table = ctx.tool_settings("save-editor");
    table.insert(
        "aatf_ruleset".to_owned(),
        toml::Value::String("autumn26".to_owned()),
    );
    ctx.set_tool_settings("save-editor", table);
    // The grid's column widths are stored per frame — a rules change is
    // one frame stale (the Grid repaints on the next): draw a settle frame.
    window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert_quick_actions(
        &window,
        &nodes,
        &shapes,
        &["Make Gold", "Make Silver", "Make Bronze"],
        &["Make Goalkeeper", "Make Regular", "Set stats"],
    );
}

/// The player list's painted pane and the quick actions' band in it, the
/// bottom panel's painted frame, in a frame's `shapes`.
fn pane_and_band(
    window: &Headless,
    shapes: &[egui::epaint::ClippedShape],
) -> (egui::Rect, egui::Rect) {
    let panel_fill = window.ctx.style_of(window.ctx.theme()).visuals.panel_fill;
    let frame = |pick: &dyn Fn(&egui::epaint::RectShape) -> bool| {
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape) if shape.fill == panel_fill && pick(shape) => {
                    Some(shape.rect)
                }
                _ => None,
            })
            .next()
            .expect("a panel frame")
    };

    // The pane the quick actions sit in: the player list's own painted
    // frame, full height inside the window's left margin.
    let pane = frame(&|shape| {
        shape.rect.min.x > 2.0 && shape.rect.max.x < 400.0 && shape.rect.height() > 600.0
    });
    // The quick actions' band: the bottom panel's painted frame under the
    // list — a shallow strip inside the pane's x range near its bottom.
    let band = frame(&|shape| {
        shape.rect.min.x >= pane.min.x
            && shape.rect.max.x < pane.max.x - 1.0
            && shape.rect.height() < 200.0
            && shape.rect.max.y > pane.max.y - 100.0
    });
    (pane, band)
}

// The quick actions' band starts 3 over its first row's buttons, the
// draft's `.plist` gap over the grid: the bottom panel's top margin.
#[test]
fn the_quick_actions_band_starts_3_over_its_first_row() {
    let (_path, mut view, _window) = open_701("quick_band_top");
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, band) = pane_and_band(&window, &shapes);
    for label in ["Make Gold", "Make Goalkeeper"] {
        let top = node_labeled(&nodes, Role::Button, label)
            .bounds()
            .expect("bounds")
            .y0;
        let gap = top - f64::from(band.min.y);
        assert!(
            (gap - 3.0).abs() <= 0.01,
            "{label} stands 3 under the band's top ({band:?}), got {gap}"
        );
    }
}

// The view fills its pane: the list and the quick actions run to the
// pane's bottom — the actions' band's painted bottom is the pane's own
// painted bottom at the 1280 x 780 window, no dead strip under it (the
// bottom panel takes its height off the list's scroll, whose floating
// bar reserves no strip of its own).
#[test]
fn the_quick_actions_reach_the_panes_bottom() {
    let (_path, mut view, _window) = open_701("quick_fills_pane");
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (pane, band) = pane_and_band(&window, &shapes);
    assert!(
        (band.max.y - pane.max.y).abs() <= 1.0,
        "the quick actions' bottom within a point of the pane's ({:?} vs {:?})",
        band,
        pane
    );
}

// The quick actions stand where the draft's do on either view: their rows'
// tops at 684, 707 and 730 of the 1280 x 780 client, whose y the pane
// shares, so 72, 49 and 26 over the pane's bottom (the status bar's line);
// the last row then ends 6 over it, the draft's padding under the view.
#[test]
fn the_quick_actions_stand_where_the_drafts_do_on_either_view() {
    const ROWS: [[&str; 2]; 3] = [
        ["Make Gold", "Make Goalkeeper"],
        ["Make Silver", "Make Regular"],
        ["Make Bronze", "Set stats"],
    ];
    const OVER_THE_BOTTOM: [f64; 3] = [72.0, 49.0, 26.0];
    for team in [false, true] {
        let (_path, mut view, _window) = open_701("quick_draft_tops");
        let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
        if team {
            open_team_view(&mut window, &mut view, 701);
        }
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        for (labels, over) in ROWS.iter().zip(OVER_THE_BOTTOM) {
            for label in labels {
                let top = node_labeled(&nodes, Role::Button, label)
                    .bounds()
                    .expect("bounds")
                    .y0;
                let shown = f64::from(DEFAULT_PANE.y) - top;
                assert!(
                    (shown - over).abs() <= 1.0,
                    "{label} stands {over} over the pane's bottom on the {} view, got {shown}",
                    if team { "team" } else { "player" }
                );
            }
        }
    }
}

// Set stats' cell is the draft's `.row`: the number's box at the draft's
// 52-point number width, the button taking the rest of the 118.5-point
// half less the 8-point gap, 58.5.
#[test]
fn set_stats_takes_what_the_number_box_leaves() {
    let (_path, mut view, mut window) = open_701("quick_set_stats_split");
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let button = node_labeled(&nodes, Role::Button, "Set stats")
        .bounds()
        .expect("bounds");
    let field = set_stats_field(&nodes).bounds().expect("bounds");
    assert!(
        (button.x1 - button.x0 - 58.5).abs() <= 1.0,
        "Set stats is 58.5 wide ({button:?})"
    );
    assert!(
        (field.x1 - field.x0 - 52.0).abs() <= 1.0,
        "its number's box is 52 wide ({field:?})"
    );
}

// The columns stand 8 apart, egui's item spacing, as the draft's `.qagrid`
// gap: the right column's buttons start 8 past the left's, and Set stats'
// box 8 past its button.
#[test]
fn the_quick_actions_columns_and_set_stats_stand_8_apart() {
    let (_path, mut view, mut window) = open_701("quick_column_gap");
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let button = |label: &str| {
        node_labeled(&nodes, Role::Button, label)
            .bounds()
            .expect("bounds")
    };
    for (left, right) in [
        ("Make Gold", "Make Goalkeeper"),
        ("Make Silver", "Make Regular"),
    ] {
        let gap = button(right).x0 - button(left).x1;
        assert!(
            (gap - 8.0).abs() <= 0.5,
            "{right} stands 8 past {left}, got {gap}"
        );
    }
    let gap = set_stats_field(&nodes).bounds().expect("bounds").x0 - button("Set stats").x1;
    assert!(
        (gap - 8.0).abs() <= 0.5,
        "Set stats' box stands 8 past its button, got {gap}"
    );
}

// The grid is docked under the list (the bottom panel takes its height
// off the list's scroll): scrolling the list to its last row leaves the
// buttons where they were.
#[test]
fn the_buttons_stay_put_when_the_list_scrolls() {
    let (_path, mut view, mut window) = open_701("quick_scrolled_list");
    let make_gold = |window: &mut Headless, view: &mut SaveView| {
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
        node_labeled(&nodes, Role::Button, "Make Gold")
            .bounds()
            .expect("bounds")
            .y0
    };
    let top = make_gold(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let rows_before = row_labels(&nodes);

    // A wheel down inside the list's scroll (the pointer rests over the
    // list, left of its bar).
    for _ in 0..20 {
        window.frame(
            vec![
                egui::Event::PointerMoved(egui::pos2(120.0, 400.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -400.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
    }
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_ne!(
        row_labels(&nodes),
        rows_before,
        "premise: the list scrolled"
    );
    assert_eq!(
        make_gold(&mut window, &mut view),
        top,
        "the buttons did not move"
    );
}

// The draft's `.qagrid` inside the 255-wide `.plist` holds its labels
// whole: at the window's
// default size no Make button's galley elides, and every control — the
// Set stats field included — stays inside the list rows' x range.
#[test]
fn the_quick_actions_fit_the_lists_width() {
    let path = fixture(PesVersion::Pes21, "quick_fit_width");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::new();

    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // A player row's AccessKit bounds stand for the list rows' x range.
    let row = nodes
        .iter()
        .find(|node| player_row(node))
        .and_then(accesskit::Node::bounds)
        .expect("a player row in the list");
    for label in [
        "Make Gold",
        "Make Silver",
        "Make Bronze",
        "Make Goalkeeper",
        "Make Regular",
        "Set stats",
    ] {
        let bounds = node_labeled(&nodes, Role::Button, label)
            .bounds()
            .expect("bounds");
        assert!(
            bounds.x0 >= row.x0 - 1.0 && bounds.x1 <= row.x1 + 1.0,
            "{label} inside the list rows' x range ({bounds:?} vs {row:?})"
        );
        if label.starts_with("Make ") {
            let shape = painted_label(&shapes, label, bounds);
            assert!(!shape.galley.elided, "{label}'s galley whole ({bounds:?})");
        }
    }
    let field = set_stats_field(&nodes).bounds().expect("bounds");
    assert!(
        field.x0 >= row.x0 - 1.0 && field.x1 <= row.x1 + 1.0,
        "the Set stats field inside the rows' x range ({field:?} vs {row:?})"
    );
}

// A label too long for its half elides, its tooltip naming the tier. The
// list is a fixed 255, so it is reached by a long tier label, not a
// narrower list.
#[test]
fn a_label_too_long_for_its_half_elides() {
    const WIDE_TIER: &str = "the longest tier in the ruleset";
    // A ruleset whose first tier's label is too long for its half.
    let mut ruleset = aatf::Rules::official().ruleset().clone();
    ruleset.tiers[0].label = WIDE_TIER.to_owned();
    let path = fixture(PesVersion::Pes21, "quick_halved_columns");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    pick_imported(
        &mut view,
        "wide.4ccrules",
        &aatf::assemble_file(&ruleset, None, None),
    );
    let mut window = Headless::new();

    // The grid's column widths are stored per frame: a settle frame so
    // they land before asserting.
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // The cells as the labels alone say they should be: halves, the long
    // label elided in its own.
    let wide_label = format!("Make {WIDE_TIER}");
    let left = [wide_label.as_str(), "Make Silver", "Make Bronze"];
    let right = ["Make Goalkeeper", "Make Regular", "Set stats"];
    let row = nodes
        .iter()
        .find(|node| player_row(node))
        .and_then(accesskit::Node::bounds)
        .expect("a player row in the list");
    let band_right = grid_band_right(&window, &shapes);
    let half = (band_right - row.x0 - 8.0) / 2.0;
    let natural = natural_columns(&window, &left, &right);
    assert!(
        natural[0] > half,
        "premise: the left column's natural {} does not fit its half {half}",
        natural[0]
    );
    assert_quick_actions(&window, &nodes, &shapes, &left, &right);

    for label in [
        &*format!("Make {WIDE_TIER}"),
        "Make Silver",
        "Make Bronze",
        "Make Goalkeeper",
        "Make Regular",
        "Set stats",
    ] {
        let bounds = node_labeled(&nodes, Role::Button, label)
            .bounds()
            .expect("bounds");
        // The Set stats cell's span ends at its field's right edge.
        let right = if label == "Set stats" {
            set_stats_field(&nodes).bounds().expect("bounds").x1
        } else {
            bounds.x1
        };
        assert!(
            bounds.x0 >= row.x0 - 1.0 && right <= band_right + 1.0,
            "{label} inside the grid's band ({bounds:?} vs {band_right})"
        );
        assert!(
            (right - bounds.x0 - half).abs() <= 1.0,
            "{label}'s cell a half of the band ({bounds:?} vs {half})"
        );
        // The right column's cells end at the list's frame, not at its bar.
        if ["Make Goalkeeper", "Make Regular", "Set stats"].contains(&label) {
            assert!(
                (right - band_right).abs() <= 0.5,
                "{label} ends at the list's frame ({right} vs {band_right})"
            );
        }
    }
    let label = format!("Make {WIDE_TIER}");
    let wide_button = node_labeled(&nodes, Role::Button, &label);
    let bounds = wide_button.bounds().expect("bounds");
    assert!(
        painted_label(&shapes, &label, bounds).galley.elided,
        "the long label elides in its half ({bounds:?})"
    );
    // The elided label's tooltip names the tier.
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(wide_button), "tier's values"),
        Some(format!(
            "The {WIDE_TIER} tier's values (the active ruleset)"
        )),
        "the elided button's tooltip"
    );
}
