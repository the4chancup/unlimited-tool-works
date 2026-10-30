//! The Live AATF dock's tests (SE-GUI-30): the docked list shrinking and
//! growing back with the open team's edits, the `No issues` header, the
//! remembered toggle, the checking mark, ALL's team and a row's click.

use pes_savefile::model::names::display_name;

use super::*;

/// A click on `node` through `tool.view` (press on one frame, release on
/// the next): the `live_aatf` write lives in `Tool::view`, not `show`.
fn tool_click(
    window: &mut Headless,
    tool: &mut crate::Tool,
    ctx: &studio_core::ToolContext,
    node: &accesskit::Node,
) {
    use studio_core::StudioTool;
    press_release(window, centre(node), |ui| tool.view(ui, ctx));
}

/// Asserts no violation row is drawn under `header` (a row would be a
/// Button below the header line; a gated control's `-` stand-in — drawn
/// off the scroll's clip with real bounds — is never one, and nor is a
/// positions cell's `C`/`B`/`A` rating).
fn assert_no_violation_row(nodes: &[accesskit::Node], header: &str) {
    let header_bottom = nodes
        .iter()
        .find(|node| node.value() == Some(header))
        .and_then(accesskit::Node::bounds)
        .expect("the header's bounds")
        .y1;
    assert!(
        !nodes.iter().any(|node| {
            node.role() == Role::Button
                && !matches!(node.label(), Some("-") | Some("C") | Some("B") | Some("A"))
                && node.bounds().is_some_and(|b| b.y0 >= header_bottom)
        }),
        "no violation row"
    );
}

/// The dock lists the open team's violations under `N issues · <ruleset>`;
/// an edit fixing one drops its row and N by one, an undo brings both back.
// SE-GUI-30
#[test]
fn the_list_shrinks_as_edits_fix_violations_and_grows_back_on_undo() {
    let (mut view, mut window) = open_734_with("live_aatf_shrink", &[73401, 73404], 183);
    view.live_aatf = true;
    // Before the first result: `Checking <team>…`, no count yet. The
    // pointer is held (on the toolbar's empty end), so no check starts: a
    // started one can reply between a frame's two passes, and the frame
    // would draw its count instead.
    let at = egui::pos2(1090.0, 12.0);
    let (_, nodes, _) = window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Checking /lit/\u{2026}")),
        "the first check's header"
    );
    assert!(
        view.live.check.pending.is_none() && view.live.check.result.is_none(),
        "premise: no check started while the pointer is held"
    );
    assert!(
        !nodes.iter().any(|node| node
            .value()
            .is_some_and(|v| v.ends_with("issues · 4cc Autumn 26"))),
        "no count before the first result"
    );
    window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let nodes = live_checked(&mut window, &mut view, &[734]);
    assert_eq!(height_rows(&nodes), 2, "both players' Height rows");
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("6 issues · 4cc Autumn 26")),
        "the count header"
    );

    // 73401 back to his fixture height through the session (one sealed
    // edit): his Height row is gone and the count is one less.
    let mut entry = player_entry(&view, 73401);
    entry.basic.height = 185;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    let nodes = live_checked(&mut window, &mut view, &[734]);
    assert_eq!(height_rows(&nodes), 1, "73401's Height row is gone");
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("5 issues · 4cc Autumn 26")),
        "the smaller count"
    );

    // An undo brings the edit, the row and the count back.
    view.session.as_mut().expect("the save opened").undo();
    let nodes = live_checked(&mut window, &mut view, &[734]);
    assert_eq!(height_rows(&nodes), 2, "the row is back");
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("6 issues · 4cc Autumn 26")),
        "the count is back"
    );
}

/// A team that passes reads `<team> passes <ruleset> with no issues.` and
/// draws no violation row; a player's height edit then reads the check's
/// count.
// SE-GUI-30
#[test]
fn a_clean_team_reads_no_issues_and_a_broken_rule_the_count() {
    let (mut view, mut window) =
        open_legal_714("live_aatf_clean", Headless::sized(DEFAULT_PANE.x, 800.0));
    view.live_aatf = true;
    let nodes = live_checked(&mut window, &mut view, &[714]);
    let name = view
        .session
        .as_ref()
        .expect("the save opened")
        .file()
        .team(714)
        .expect("team 714")
        .name
        .clone();
    let header = format!("{name} passes 4cc Autumn 26 with no issues.");
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some(header.as_str())),
        "the passing header: {header}"
    );
    assert_no_violation_row(&nodes, &header);

    // One player's height broken: the count header reads the check's
    // count.
    let first = view
        .session
        .as_ref()
        .expect("the save opened")
        .file()
        .team(714)
        .expect("team 714")
        .roster[0]
        .player_id;
    let mut entry = player_entry(&view, first);
    entry.basic.height = 183;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    let nodes = live_checked(&mut window, &mut view, &[714]);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("3 issues · 4cc Autumn 26")),
        "the count after the edit"
    );

    // The last one fixed: `No issues` again, and no row is drawn.
    view.session.as_mut().expect("the save opened").undo();
    let nodes = live_checked(&mut window, &mut view, &[714]);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some(header.as_str())),
        "the passing header is back"
    );
    assert_no_violation_row(&nodes, &header);
}

/// The toggle is a setting: clicking it writes `live_aatf`, the tool's
/// other keys untouched, and a fresh `Tool` on the same settings draws the
/// dock on its first view frame; toggled back, the dock is gone.
// SE-GUI-30
#[test]
fn the_toggle_is_remembered_in_the_settings() {
    use studio_core::StudioTool;

    let save_path = fixture(PesVersion::Pes17, "live_aatf_remembered");
    let dir = save_path.parent().expect("the scratch dir").to_path_buf();
    let mut tool = crate::Tool::new();
    tool.view.open(&save_path);
    let mut settings = studio_core::Settings::default();
    let mut table = tool.default_settings();
    table.insert("unread_key".to_owned(), toml::Value::Integer(3));
    settings.set_tool("save-editor", table);
    let (ctx, _events_rx, _requests_rx) = tool_context(&dir, settings);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);

    // The toggle clicked: the setting holds true, the unread key stands.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    tool_click(
        &mut window,
        &mut tool,
        &ctx,
        node_labeled(&nodes, Role::Button, "\u{25b6} Live AATF"),
    );
    let table = ctx.tool_settings("save-editor");
    assert_eq!(
        table["live_aatf"].as_bool(),
        Some(true),
        "the toggle set the setting"
    );
    assert_eq!(
        table["unread_key"].as_integer(),
        Some(3),
        "the tool's other key is unchanged"
    );

    // A fresh tool on the same context: the dock drawn on the first view
    // frame.
    let mut tool2 = crate::Tool::new();
    tool2.view.open(&save_path);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool2.view(ui, &ctx));
    assert!(
        nodes.iter().any(|node| node.value() == Some("Live AATF")),
        "the dock's header on the first frame"
    );

    // Toggled back: the setting is false and the dock is gone.
    tool_click(
        &mut window,
        &mut tool2,
        &ctx,
        node_labeled(&nodes, Role::Button, "\u{25b6} Live AATF"),
    );
    assert_eq!(
        ctx.tool_settings("save-editor")["live_aatf"].as_bool(),
        Some(false),
        "the toggle wrote false"
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool2.view(ui, &ctx));
    assert!(
        !nodes.iter().any(|node| node.value() == Some("Live AATF")),
        "the dock is gone"
    );
}

/// Show suggestions is a checkbox right of the Live AATF toggle, drawn
/// only while Live AATF is on, off by default; ticking it writes
/// `live_suggestions`, the tool's other keys untouched, and a fresh
/// `Tool` on the same settings draws it ticked.
#[test]
fn show_suggestions_is_drawn_while_live_and_remembered() {
    use studio_core::StudioTool;

    let save_path = fixture(PesVersion::Pes17, "live_aatf_suggestions_setting");
    let dir = save_path.parent().expect("the scratch dir").to_path_buf();
    let mut tool = crate::Tool::new();
    tool.view.open(&save_path);
    let mut settings = studio_core::Settings::default();
    let mut table = tool.default_settings();
    assert_eq!(
        table["live_suggestions"].as_bool(),
        Some(false),
        "off by default"
    );
    table.insert("unread_key".to_owned(), toml::Value::Integer(3));
    settings.set_tool("save-editor", table);
    let (ctx, _events_rx, _requests_rx) = tool_context(&dir, settings);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let checkbox = |nodes: &[accesskit::Node]| {
        nodes
            .iter()
            .find(|node| node.role() == Role::CheckBox && node.label() == Some("Show suggestions"))
            .cloned()
    };

    // Live AATF off: no checkbox. On: the checkbox right of the toggle,
    // on its row, unticked.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(
        checkbox(&nodes).is_none(),
        "no checkbox while Live AATF is off"
    );
    tool_click(
        &mut window,
        &mut tool,
        &ctx,
        node_labeled(&nodes, Role::Button, "\u{25b6} Live AATF"),
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    let toggle = node_labeled(&nodes, Role::Button, "\u{25b6} Live AATF")
        .bounds()
        .expect("the toggle's bounds");
    let tick = checkbox(&nodes).expect("the checkbox while Live AATF is on");
    let bounds = tick.bounds().expect("the checkbox's bounds");
    assert!(
        bounds.x0 >= toggle.x1 && bounds.y0 < toggle.y1 && toggle.y0 < bounds.y1,
        "right of the toggle, on its row ({bounds:?} against {toggle:?})"
    );
    assert_eq!(tick.toggled(), Some(accesskit::Toggled::False), "unticked");

    // Ticked: the setting holds true, Live AATF and the unread key stand.
    tool_click(&mut window, &mut tool, &ctx, &tick);
    let table = ctx.tool_settings("save-editor");
    assert_eq!(
        table["live_suggestions"].as_bool(),
        Some(true),
        "the tick set the setting"
    );
    assert_eq!(
        table["live_aatf"].as_bool(),
        Some(true),
        "Live AATF stays on"
    );
    assert_eq!(
        table["unread_key"].as_integer(),
        Some(3),
        "the tool's other key is unchanged"
    );

    // A fresh tool on the same context draws it ticked.
    let mut tool2 = crate::Tool::new();
    tool2.view.open(&save_path);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool2.view(ui, &ctx));
    assert_eq!(
        checkbox(&nodes).and_then(|node| node.toggled()),
        Some(accesskit::Toggled::True),
        "ticked on the first frame"
    );
}

/// Show suggestions off, the default: the check asks for no suggestion,
/// so the dock lists no `Unused` row and no `Suggestions` heading and no
/// blue mark is drawn anywhere. On: the dock lists them under
/// `Suggestions` after the team's own rows, which stand under `Team-wide
/// issues`; a player whose rows are suggestions alone gets a blue check
/// mark and his allowance's field a blue circled i; the header still
/// counts errors and warnings alone. Team 734's own rows are its three
/// tier counts; eight `Unused` rows fall on four of its players.
#[test]
fn suggestions_are_asked_for_only_while_show_suggestions_is_on() {
    let path = fixture(PesVersion::Pes17, "live_aatf_suggestions");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(734);
    view.selected = Some(Selection::Player(734, 0));
    view.live_aatf = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    let blue = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .hyperlink_color;
    let blue_marks = |shapes: &[egui::epaint::ClippedShape]| {
        mark_shapes_colored(shapes, "\u{2714}", blue)
            + mark_shapes_colored(shapes, "\u{24d8}", blue)
    };
    let issues = "3 issues \u{b7} 4cc Autumn 26";

    assert!(!view.live_suggestions, "premise: off by default");
    let nodes = live_checked(&mut window, &mut view, &[734]);
    let held = view.live.check.result.as_ref().unwrap().1[0]
        .as_ref()
        .unwrap();
    assert!(
        held.iter()
            .all(|violation| violation.severity != aatf::Severity::Suggestion),
        "no suggestion asked for"
    );
    assert!(
        nodes.iter().any(|node| node.value() == Some(issues)),
        "the header counts the team's three rows"
    );
    assert!(
        !nodes.iter().any(|node| node.value() == Some("Suggestions")
            || node
                .label()
                .is_some_and(|label| label.starts_with("Unused"))),
        "no Suggestions heading and no Unused row"
    );
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(blue_marks(&shapes), 0, "no blue mark in the list");

    // On: the suggestions asked for and listed after the team's rows.
    view.live_suggestions = true;
    let nodes = live_checked(&mut window, &mut view, &[734]);
    assert!(
        nodes.iter().any(|node| node.value() == Some(issues)),
        "the header still counts errors and warnings alone"
    );
    let y_of = |value: &str| {
        nodes
            .iter()
            .find(|node| node.value() == Some(value))
            .and_then(accesskit::Node::bounds)
            .unwrap_or_else(|| panic!("no {value:?}"))
            .y0
    };
    let team_wide = y_of("Team-wide issues");
    let heading = y_of("Suggestions");
    let tier_rows: Vec<f64> = nodes
        .iter()
        // A team row only reports: a Label node, its line its value.
        .filter(|node| {
            node.label()
                .or(node.value())
                .is_some_and(|line| line.starts_with("Tier count"))
        })
        .filter_map(|node| node.bounds().map(|bounds| bounds.y0))
        .collect();
    assert_eq!(tier_rows.len(), 3, "the team's three tier counts");
    assert!(
        tier_rows.iter().all(|y| *y > team_wide && *y < heading),
        "the team's rows under Team-wide issues, before Suggestions"
    );
    let unused: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.label()
                .is_some_and(|label| label.starts_with("Unused"))
        })
        .collect();
    assert!(!unused.is_empty(), "the Unused rows are listed");
    assert!(
        unused
            .iter()
            .all(|node| node.bounds().is_some_and(|bounds| bounds.y0 > heading)),
        "under the Suggestions heading"
    );

    // A player whose rows are suggestions alone: a blue check mark on
    // his row, and, selected, a blue circled i on his allowance's field.
    let slot = held_slot_with_suggestions_alone(&view);
    view.selected = Some(Selection::Player(734, slot));
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        mark_shapes_colored(&shapes, "\u{2714}", blue) >= 1,
        "a blue check mark in the list"
    );
    assert!(
        mark_shapes_colored(&shapes, "\u{24d8}", blue) >= 1,
        "a blue circled i on his field"
    );
}

/// The first roster slot of the held result whose rows are all
/// suggestions.
fn held_slot_with_suggestions_alone(view: &SaveView) -> u8 {
    let held = view.live.check.result.as_ref().unwrap().1[0]
        .as_ref()
        .unwrap();
    held.iter()
        .filter_map(|violation| violation.slot)
        .find(|slot| {
            held.iter()
                .filter(|violation| violation.slot == Some(*slot))
                .all(|violation| violation.severity == aatf::Severity::Suggestion)
        })
        .expect("a player with suggestions alone")
}

/// How many `text` shapes the frame paints in `color`.
fn mark_shapes_colored(
    shapes: &[egui::epaint::ClippedShape],
    text: &str,
    color: egui::Color32,
) -> usize {
    shapes
        .iter()
        .filter(|clipped| {
            matches!(&clipped.shape, egui::Shape::Text(shape)
                if shape.galley.job.text == text && galley_color(shape) == color)
        })
        .count()
}

/// An edit marks the previous result `checking…`, its count still shown,
/// until the new check's result replaces it. The edit lands while the
/// pointer is held, so the check waits for the release: a started check
/// can reply between a frame's two passes, before the mark is read.
// SE-GUI-30
#[test]
fn an_edit_marks_the_previous_result_checking() {
    let (mut view, mut window) = open_734_with("live_aatf_checking", &[73401, 73404], 183);
    view.live_aatf = true;
    let nodes = live_checked(&mut window, &mut view, &[734]);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("6 issues · 4cc Autumn 26")),
        "premise: the count header"
    );

    // The press lands on the toolbar's empty end.
    let at = egui::pos2(1090.0, 12.0);
    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let mut entry = player_entry(&view, 73401);
    entry.basic.height = 185;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    let nodes = draw(&mut window, &mut view);
    assert!(
        view.live.check.pending.is_none(),
        "premise: no check started while the pointer is held"
    );
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("checking\u{2026}")),
        "the checking mark beside the old header"
    );
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("6 issues · 4cc Autumn 26")),
        "the previous count still shown"
    );
    assert_eq!(height_rows(&nodes), 2, "the stale rows stay while checking");

    window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let nodes = live_checked(&mut window, &mut view, &[734]);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("5 issues · 4cc Autumn 26")),
        "the new count replaces it"
    );
    assert!(
        !nodes
            .iter()
            .any(|node| node.value() == Some("checking\u{2026}")),
        "and the mark is gone"
    );
}

/// The dock follows the open team and the ruleset: another team picked
/// re-checks and re-counts it, and a ruleset whose result differs
/// replaces the rows.
// SE-GUI-30
#[test]
fn the_dock_follows_the_open_team_and_the_ruleset() {
    let (mut view, mut window) = open_734_with("live_aatf_team_rules", &[73401, 73404], 183);
    view.live_aatf = true;
    let nodes = live_checked(&mut window, &mut view, &[734]);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("6 issues · 4cc Autumn 26")),
        "premise: 734's count"
    );

    // Another team picked, as the selector does: the dock re-checks it.
    view.team = TeamChoice::Team(714);
    view.selected = Some(Selection::Player(714, 0));
    let nodes = live_checked(&mut window, &mut view, &[714]);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("3 issues · 4cc Autumn 26")),
        "714's count"
    );
    assert!(
        !nodes
            .iter()
            .any(|node| node.value() == Some("Team 734 /lit/")),
        "no row of 734's"
    );

    // Back to 734 under an imported ruleset whose result differs: the Tall
    // bracket also takes 183, so the two Height rows go and the red
    // quota is met.
    view.team = TeamChoice::Team(734);
    view.selected = Some(Selection::Player(734, 0));
    pick_imported(
        &mut view,
        "taller.4ccrules",
        &aatf::official_file().replacen("exactly: [185]", "exactly: [183, 185]", 1),
    );
    let nodes = live_checked(&mut window, &mut view, &[734]);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("3 issues · 4cc Autumn 26")),
        "the new file's count"
    );
    assert_eq!(height_rows(&nodes), 0, "183 takes the Tall bracket");
}

/// A result checked under 4cc Autumn 26 is not drawn once 4cc Spring 26
/// is picked: its rows would be worded with the other ruleset's tiers.
/// Until the recheck returns the dock reads `Checking <team>…`, as before
/// its first result; then Spring 26's rows are drawn. The pick lands while
/// the pointer is held, so the recheck waits for the release: a started
/// check can reply between a frame's two passes.
// SE-GUI-30
#[test]
fn a_result_checked_under_another_ruleset_is_not_drawn() {
    let (mut view, mut window) = open_734_with("live_aatf_other_ruleset", &[73401, 73404], 183);
    view.live_aatf = true;
    let held_group = "Player 73404  45 Yuki-kun ^~^";
    let nodes = live_checked(&mut window, &mut view, &[734]);
    assert!(
        nodes.iter().any(|node| node.value() == Some(held_group)),
        "premise: Autumn 26's result lists 73404"
    );

    // The press lands on the toolbar's empty end.
    let at = egui::pos2(1090.0, 12.0);
    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    view.aatf_ruleset = "spring26".to_owned();
    let nodes = draw(&mut window, &mut view);
    assert!(
        view.live.check.pending.is_none()
            && view
                .live
                .check
                .result
                .as_ref()
                .is_some_and(|(key, _)| key.ruleset == "autumn26"),
        "premise: Autumn 26's result held, no recheck started"
    );
    assert!(
        !nodes.iter().any(|node| node.value() == Some(held_group)),
        "the held result's rows are not drawn"
    );
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Checking /lit/\u{2026}")),
        "the header before a result"
    );
    assert!(
        !nodes.iter().any(|node| node
            .value()
            .is_some_and(|value| value.ends_with("\u{b7} 4cc Spring 26"))),
        "no count under Spring 26's name"
    );

    window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let nodes = live_checked(&mut window, &mut view, &[734]);
    assert!(
        nodes.iter().any(|node| node
            .label()
            .is_some_and(|label| label.ends_with("77, over the 74 the rules allow"))),
        "Spring 26's rows are drawn"
    );
    assert!(
        nodes.iter().any(|node| node
            .value()
            .is_some_and(|value| value.ends_with("issues \u{b7} 4cc Spring 26"))),
        "Spring 26's count"
    );
}

/// At ALL the dock checks the selected player's team; with no player
/// selected it reads `No team is open.` and starts no check.
// SE-GUI-30
#[test]
fn at_all_the_dock_checks_the_selected_players_team() {
    let (mut view, mut window) = open_734_with("live_aatf_all", &[73401], 183);
    view.live_aatf = true;
    view.team = TeamChoice::All;
    view.selected = Some(Selection::Player(734, 0));
    let nodes = live_checked(&mut window, &mut view, &[734]);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Team 734 /lit/")),
        "734's rows under ALL"
    );

    view.selected = None;
    let nodes = draw(&mut window, &mut view);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("No team is open.")),
        "the empty selection's line"
    );
    assert!(view.live.check.pending.is_none(), "no check started");
    assert!(
        !nodes
            .iter()
            .any(|node| node.value() == Some("Team 734 /lit/")),
        "the stale team's rows are not drawn"
    );
}

/// SE-GUI-30's current-team rule: the dock follows the selected row's
/// team, another team's too when the filter lists its rows — the selector
/// at team 701 with `QUALITY` typed lists 70202 of team 702 (`list.rs`'
/// filter test), and his row clicked makes the dock check his team.
// SE-GUI-30
#[test]
fn the_checks_follow_the_selected_rows_team_under_a_filter() {
    // Team 702's players made tall enough to take no bracket, so the check
    // lists violations; 70202 named as `list.rs`' filter test's (the PES 21
    // fixture's players are all PLACEHOLDER). 78101 named to match too, so
    // the filter lists a second team's row, and broken the same way so 781
    // has a violation row to show.
    let path = fixture(PesVersion::Pes21, "live_aatf_filtered_team");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    for id in [70201u32, 70202, 78101] {
        file.player_mut(id).unwrap().basic.height = 183;
    }
    file.player_mut(70202).unwrap().name = "QUALITY".to_owned();
    file.player_mut(78101).unwrap().name = "QUALITY2".to_owned();
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    view.live_aatf = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);

    // The filter lists 70202 of team 702 across teams; clicking his row
    // selects him.
    view.filter = "quality".to_owned();
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (target, _) = nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("02 CB QUALITY"))
        .expect("the filtered row");
    action_click(&mut window, &mut view, *target);
    assert!(
        matches!(view.selected, Some(Selection::Player(702, _))),
        "70202's row selected: {:?}",
        view.selected
    );
    let nodes = live_checked(&mut window, &mut view, &[702]);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Team 702 /a/")),
        "the dock's rows are team 702's"
    );

    // A team row's own team (SE-GUI-30's other clause): the filter lists
    // team 781's row too (78101 renamed QUALITY2), and clicking it makes
    // the dock check 781, not the selector's 701 nor the player's 702.
    let name_781 = view
        .session
        .as_ref()
        .unwrap()
        .file()
        .team(781)
        .unwrap()
        .name
        .clone();
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let label_781 = format!("781 {}", display_name(&name_781));
    let (target, _) = nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(label_781.as_str()))
        .expect("team 781's row");
    action_click(&mut window, &mut view, *target);
    assert_eq!(
        view.selected,
        Some(Selection::Team(781)),
        "team 781's row selected"
    );
    let header_781 = format!("Team 781 {name_781}");
    let nodes = live_checked(&mut window, &mut view, &[781]);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some(header_781.as_str())),
        "the dock's rows are team 781's"
    );
}

/// A team whose name holds a colour code (the legal team 714, its name
/// coloured red) reads as its display name in both headers, `Checking
/// <team>…` before the first result and `<team> passes <ruleset> with no
/// issues.` after it.
// SE-GUI-30
#[test]
fn the_headers_name_a_coded_team_by_its_display_name() {
    let (mut view, mut window) = open_legal_714(
        "live_aatf_coded_headers",
        Headless::sized(DEFAULT_PANE.x, 800.0),
    );
    edit_team(view.session.as_mut().unwrap(), 714, 1, |team| {
        team.name = format!("\x11cff0000ff{}", team.name);
    });
    view.live_aatf = true;
    let raw = team_entry(&view, 714).name;
    let shown = display_name(&raw);
    assert_ne!(raw, shown, "premise: 714's name holds a colour code");

    // The pointer is held on the toolbar's empty end, so no check starts
    // and the frame draws the header before a result.
    let at = egui::pos2(1090.0, 12.0);
    let (_, nodes, _) = window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        view.live.check.pending.is_none() && view.live.check.result.is_none(),
        "premise: no check started while the pointer is held"
    );
    let checking = format!("Checking {shown}\u{2026}");
    assert!(
        label_value(&nodes, &checking),
        "the header before a result: {checking}"
    );
    assert!(
        !label_value(&nodes, &format!("Checking {raw}\u{2026}")),
        "the header does not hold the raw name"
    );
    window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });

    let nodes = live_checked(&mut window, &mut view, &[714]);
    let passes = format!("{shown} passes 4cc Autumn 26 with no issues.");
    assert!(label_value(&nodes, &passes), "the passing header: {passes}");
    assert!(
        !label_value(
            &nodes,
            &format!("{raw} passes 4cc Autumn 26 with no issues.")
        ),
        "the header does not hold the raw name"
    );
}

/// A violation row's click moves the selector to the player's team, so the
/// list shows the row it selects; at ALL the selector stays.
// SE-GUI-30
#[test]
fn a_rows_click_moves_the_selector_unless_it_reads_all() {
    let (mut view, mut window) = open_734_with("live_aatf_click_selector", &[73401, 73404], 183);
    view.live_aatf = true;
    // The selector on another team, a 734 player's row selected: the dock
    // checks 734.
    view.team = TeamChoice::Team(718);
    view.selected = Some(Selection::Player(734, 0));
    let nodes = live_checked(&mut window, &mut view, &[734]);
    click(
        &mut window,
        height_row_below(&nodes, "Player 73404  45 Yuki-kun ^~^"),
        &mut view,
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(734, 3)),
        "73404's team and slot"
    );
    assert_eq!(view.team, TeamChoice::Team(734), "the selector moved");

    // At ALL it stays. Another of 734's players selected first, or a
    // click that did nothing would satisfy both assertions.
    view.team = TeamChoice::All;
    view.selected = Some(Selection::Player(734, 0));
    let nodes = live_checked(&mut window, &mut view, &[734]);
    click(
        &mut window,
        height_row_below(&nodes, "Player 73404  45 Yuki-kun ^~^"),
        &mut view,
    );
    assert_eq!(view.selected, Some(Selection::Player(734, 3)));
    assert_eq!(view.team, TeamChoice::All, "ALL is kept");
}

/// A violation row's click clears the player list's filter: the selection
/// is only worth it if the row it lands on can show.
// SE-GUI-30
#[test]
fn a_rows_click_clears_the_filter_so_the_row_shows() {
    let (mut view, mut window) = open_734_with("live_aatf_click_filter", &[73401, 73404], 183);
    view.live_aatf = true;
    view.filter = "no player named this".to_owned();
    let nodes = live_checked(&mut window, &mut view, &[734]);
    assert!(
        row_labels(&nodes).is_empty(),
        "premise: the filter hides every player"
    );
    click(
        &mut window,
        height_row_below(&nodes, "Player 73404  45 Yuki-kun ^~^"),
        &mut view,
    );
    assert!(view.filter.is_empty(), "the click cleared the filter");
    let nodes = draw(&mut window, &mut view);
    assert!(
        row_labels(&nodes)
            .iter()
            .any(|label| label.contains("Yuki-kun")),
        "the selected player's row shows: {:?}",
        row_labels(&nodes)
    );
}

/// Another save starts the dock's check afresh (`SaveView::open`): the
/// result and the check in flight go with the old save, Live AATF stays on.
#[test]
fn another_save_starts_the_docks_check_afresh() {
    let (mut view, mut window) = open_734_with("live_aatf_open_reset", &[73401, 73404], 183);
    view.live_aatf = true;
    live_checked(&mut window, &mut view, &[734]);
    assert!(view.live.check.result.is_some(), "premise: a check ran");

    let save17 = fixture(PesVersion::Pes17, "live_aatf_open_reset_17");
    view.open(&save17);
    assert!(
        view.live.check.result.is_none() && view.live.check.pending.is_none(),
        "the result and its check went with the save"
    );
    assert!(view.live_aatf, "Live AATF stays on");
}

/// A worker that ends without replying: `show` drops the dead check's
/// `pending` and the next settled frame starts a fresh one.
#[test]
fn a_worker_that_ends_without_a_reply_is_checked_again() {
    let (mut view, mut window) = open_734_with("live_aatf_dead_worker", &[73401, 73404], 183);
    view.live_aatf = true;
    view.selected = Some(Selection::Player(734, 0));
    // The ruleset resolved, so the dead check's key names it.
    draw(&mut window, &mut view);
    // A pending whose sender is already gone: the worker died.
    let (sender, replies) = crossbeam_channel::unbounded();
    drop(sender);
    view.live.check.result = None;
    view.live.check.pending = Some(aatf_check::PendingCheck {
        key: aatf_check::CheckKey {
            revision: view.session.as_ref().expect("the save opened").revision(),
            ids: vec![734],
            ruleset: "autumn26".to_owned(),
            suggestions: view.live_suggestions,
        },
        replies,
    });
    // The dead pending is dropped and a fresh check for the key starts.
    let nodes = live_checked(&mut window, &mut view, &[734]);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Team 734 /lit/")),
        "a fresh check ran"
    );
}

/// A violation row's click selects the player: `view.selected` is his
/// (team, slot).
// SE-GUI-30
#[test]
fn a_rows_click_selects_the_player() {
    let (mut view, mut window) = open_734_with("live_aatf_click", &[73401, 73404], 183);
    view.live_aatf = true;
    let nodes = live_checked(&mut window, &mut view, &[734]);
    // Another of 734's players selected first: a click that did nothing
    // would satisfy the assertion anyway.
    view.selected = Some(Selection::Player(734, 0));
    click(
        &mut window,
        height_row_below(&nodes, "Player 73404  45 Yuki-kun ^~^"),
        &mut view,
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(734, 3)),
        "73404's team and slot"
    );
}

/// A `FieldedNotA` row's click selects his team's row instead of him,
/// the named preset shown on the first of its formations that carries
/// the cross: preset 1's
/// kick-off formation for the non-fluid preset, formation 3 for the
/// fluid one whose third alone puts him at the spot.
// SE-GUI-30
#[test]
fn a_fielded_rows_click_selects_the_team_row_on_the_crosss_formation() {
    let (mut view, mut window) = open_734_with("live_aatf_fielded_jump", &[], 183);
    view.live_aatf = true;
    // A starter of preset 1's kick-off formation moved onto a position
    // he is not rated A at (the `live_marks` test's move).
    let index = 4;
    let mut team = team_entry(&view, 734);
    let slot = team.tactics.starting_eleven[index];
    let position = unrated_position_byte(&view, team.roster[usize::from(slot)].player_id);
    let kick_off = team.tactics.presets[0].formations[0].players[index].position;
    assert_ne!(kick_off, position, "premise: the move changes his spot");
    team.tactics.presets[0].formations[0].players[index].position = position;
    let header = player_header(&view, 734, slot);
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_team(1, team);
    let nodes = live_checked(&mut window, &mut view, &[734]);

    // Another of 734's players selected first and another preset and
    // formation shown: a click that did nothing would satisfy every
    // assertion anyway.
    view.selected = Some(Selection::Player(734, 0));
    view.tactics.preset = 1;
    view.tactics.formation = 2;
    click(&mut window, fielded_row_below(&nodes, &header), &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Team(734)),
        "his team's row selected"
    );
    assert_eq!(
        (view.tactics.preset, view.tactics.formation),
        (0, 0),
        "preset 1's kick-off formation shown"
    );

    // Preset 1 fluid and only its third formation putting him at the
    // position: the click shows formation 3.
    let mut team = team_entry(&view, 734);
    team.tactics.presets[0].style.fluid = Some(true);
    for formation in 0..2 {
        team.tactics.presets[0].formations[formation].players[index].position = kick_off;
    }
    team.tactics.presets[0].formations[2].players[index].position = position;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_team(2, team);
    live_checked(&mut window, &mut view, &[734]);
    view.selected = Some(Selection::Player(734, 0));
    view.tactics.preset = 2;
    view.tactics.formation = 1;
    // His row now lists below the dock's 150-point well, past its clip,
    // where a pointer click lands on nothing: the click is the AccessKit
    // action, which reaches the row's response wherever it is drawn.
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let nodes: Vec<accesskit::Node> = pairs.iter().map(|(_, node)| node.clone()).collect();
    let row = fielded_row_below(&nodes, &header);
    let (target, _) = pairs
        .iter()
        .find(|(_, node)| node.label() == row.label() && node.bounds() == row.bounds())
        .expect("the row's node id");
    action_click(&mut window, &mut view, *target);
    assert_eq!(
        view.selected,
        Some(Selection::Team(734)),
        "his team's row selected"
    );
    assert_eq!(
        (view.tactics.preset, view.tactics.formation),
        (0, 2),
        "preset 1's third formation shown"
    );
}

/// The painted wells: every `Rect` shape in the extreme fill with the
/// window stroke (the log panel's `well_frame`).
fn wells(shapes: &[egui::epaint::ClippedShape], visuals: &egui::Visuals) -> Vec<egui::Rect> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.fill == visuals.extreme_bg_color
                    && shape.stroke == visuals.window_stroke() =>
            {
                Some(shape.rect)
            }
            _ => None,
        })
        .collect()
}

/// The dock's rows sit in the log's well (the drafts' `.live-body`), and
/// the well is 150 points high when the dock first opens (`LIVE.height`):
/// measured on the first frame, right under the dock's header, and again
/// around the rows once the check lists them.
#[test]
fn the_rows_sit_in_a_150_point_well_when_the_dock_opens() {
    // 70201 made too tall for a bracket, so team 702 lists a row.
    let path = fixture(PesVersion::Pes21, "live_aatf_well");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70201).unwrap().basic.height = 183;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(702);
    view.selected = Some(Selection::Player(702, 0));
    view.live_aatf = true;
    let mut window = Headless::new();
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();

    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let title = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some("Live AATF"))
        .and_then(accesskit::Node::bounds)
        .expect("the dock's title");
    let first = wells(&shapes, &visuals)
        .into_iter()
        .find(|well| f64::from(well.top()) >= title.y1 && f64::from(well.top()) <= title.y1 + 20.0)
        .unwrap_or_else(|| panic!("no well under the dock's header: {title:?}"));
    assert!(
        (first.height() - 150.0).abs() <= 1.0,
        "the well's height on the first frame: {first:?}"
    );

    live_checked(&mut window, &mut view, &[702]);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let header = nodes
        .iter()
        .find(|node| node.value() == Some("Team 702 /a/"))
        .and_then(accesskit::Node::bounds)
        .expect("team 702's group header");
    let centre = egui::pos2(
        ((header.x0 + header.x1) / 2.0) as f32,
        ((header.y0 + header.y1) / 2.0) as f32,
    );
    let around = wells(&shapes, &visuals)
        .into_iter()
        .find(|well| well.contains(centre))
        .expect("a well around the rows");
    assert!(
        (around.top() - first.top()).abs() <= 1.0 && (around.height() - 150.0).abs() <= 1.0,
        "the well around the rows: {around:?}, first {first:?}"
    );
}
