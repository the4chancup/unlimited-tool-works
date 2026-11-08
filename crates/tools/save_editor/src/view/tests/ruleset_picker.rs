//! The toolbar's ruleset dropdown (SE-GUI-15, SE-GUI-32): where it stands,
//! what it lists, the `aatf_ruleset` setting a pick writes and the
//! window's reading of it, and a setting naming a ruleset that is gone.

use studio_core::StudioTool;

use super::*;

/// The built-in rulesets' descriptions, in the dropdown's order.
const BUILTINS: [&str; 7] = [
    "4cc Autumn 26",
    "4cc Spring 26",
    "/vg/ League 27",
    "/vg/ League 26",
    "/vt/ League 11",
    "/vt/ League 10",
    "Divegrass Before Christmas 7",
];

/// The dropdown's command row, after the rulesets.
const IMPORT_ROW: &str = "Import rules file\u{2026}";

/// The toolbar's Live AATF toggle.
fn live_toggle(nodes: &[accesskit::Node]) -> &accesskit::Node {
    node_labeled(nodes, Role::Button, "\u{25b6} Live AATF")
}

/// The toolbar's ruleset dropdown: the combo box on the Live AATF
/// toggle's row nearest left of it.
fn dropdown(nodes: &[accesskit::Node]) -> &accesskit::Node {
    let toggle = live_toggle(nodes).bounds().expect("the toggle's bounds");
    nodes
        .iter()
        .filter(|node| {
            node.role() == Role::ComboBox
                && node
                    .bounds()
                    .is_some_and(|b| b.y0 < toggle.y1 && b.y1 > toggle.y0 && b.x1 <= toggle.x0)
        })
        .max_by(|a, b| {
            let x = |node: &&accesskit::Node| node.bounds().expect("bounds").x1;
            x(a).partial_cmp(&x(b)).expect("ordered bounds")
        })
        .expect("a combo box left of the Live AATF toggle on its row")
}

/// One open-dropdown row's label, `toggled` state and bounds.
type DropdownRow = (String, Option<accesskit::Toggled>, accesskit::Rect);

/// The open dropdown's row nodes, top to bottom: the Buttons inside its
/// list box, an imported row's × right after its row.
fn popup_row_nodes(pairs: &[(accesskit::NodeId, accesskit::Node)]) -> Vec<&accesskit::Node> {
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        pairs.iter().map(|(id, node)| (*id, node)).collect();
    let (list, _) = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::ListBox)
        .expect("the dropdown's open list");
    let mut rows: Vec<&accesskit::Node> = Vec::new();
    let mut stack = vec![*list];
    while let Some(id) = stack.pop() {
        let node = by_id[&id];
        stack.extend(node.children());
        if node.role() == Role::Button && node.label().is_some() {
            rows.push(node);
        }
    }
    rows.sort_by(|a, b| {
        let key = |node: &&accesskit::Node| {
            let bounds = node.bounds().expect("row bounds");
            (bounds.y0, bounds.x0)
        };
        key(a).partial_cmp(&key(b)).expect("ordered bounds")
    });
    rows
}

/// The open dropdown's rows, top to bottom: the Buttons inside its list
/// box, an imported row's × right after its row.
fn popup_rows(pairs: &[(accesskit::NodeId, accesskit::Node)]) -> Vec<String> {
    popup_row_nodes(pairs)
        .into_iter()
        .map(|node| node.label().expect("a labeled row").to_owned())
        .collect()
}

/// The ys of the horizontal lines spanning the open dropdown's rows (its
/// separators), top to bottom; the `ListBox` node carries no bounds, so
/// the rows' own rect is the measure.
fn list_separators(
    pairs: &[(accesskit::NodeId, accesskit::Node)],
    shapes: &[egui::epaint::ClippedShape],
) -> Vec<f64> {
    let rows = popup_row_nodes(pairs);
    let span = rows.iter().filter_map(|node| node.bounds()).fold(
        (f64::MAX, f64::MIN, f64::MAX, f64::MIN),
        |(top, bottom, left, right), bounds| {
            (
                top.min(bounds.y0),
                bottom.max(bounds.y1),
                left.min(bounds.x0),
                right.max(bounds.x1),
            )
        },
    );
    let (top, bottom, left, right) = span;
    let mut ys: Vec<f64> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, .. }
                if points[0].y == points[1].y
                    && f64::from(points[0].y) >= top
                    && f64::from(points[0].y) <= bottom
                    && f64::from(points[0].x.min(points[1].x)) <= left + 1.0
                    && f64::from(points[0].x.max(points[1].x)) >= right - 1.0 =>
            {
                Some(f64::from(points[0].y))
            }
            _ => None,
        })
        .collect();
    ys.sort_by(|a, b| a.partial_cmp(b).expect("ordered ys"));
    ys
}

/// The open dropdown's rows' `(label, toggled, bounds)` and separator ys
/// for one `view`: the dropdown opened if closed, the open frame read.
fn dropdown_state(window: &mut Headless, view: &mut SaveView) -> (Vec<DropdownRow>, Vec<f64>) {
    let nodes = draw(window, view);
    if !nodes.iter().any(|node| node.role() == Role::ListBox) {
        click(window, dropdown(&nodes), view);
    }
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let rows = popup_row_nodes(&pairs)
        .into_iter()
        .map(|node| {
            (
                node.label().expect("a labeled row").to_owned(),
                node.toggled(),
                node.bounds().expect("row bounds"),
            )
        })
        .collect();
    (rows, list_separators(&pairs, &shapes))
}

/// The quick actions' Make buttons' labels, sorted.
fn make_buttons(nodes: &[accesskit::Node]) -> Vec<String> {
    let mut labels: Vec<String> = nodes
        .iter()
        .filter(|node| node.role() == Role::Button)
        .filter_map(|node| node.label())
        .filter(|label| label.starts_with("Make "))
        .map(str::to_owned)
        .collect();
    labels.sort();
    labels
}

/// `tool.view`'s frame with `events`, its node pairs.
fn tool_frame(
    window: &mut Headless,
    tool: &mut crate::Tool,
    ctx: &studio_core::ToolContext,
    events: Vec<egui::Event>,
) -> Vec<(accesskit::NodeId, accesskit::Node)> {
    let (_, pairs, _) = window.frame_ids(events, |ui| tool.view(ui, ctx));
    pairs
}

/// The nodes of `pairs`, ids dropped.
fn nodes_of(pairs: &[(accesskit::NodeId, accesskit::Node)]) -> Vec<accesskit::Node> {
    pairs.iter().map(|(_, node)| node.clone()).collect()
}

/// The tool on `test`'s PES 21 fixture save, team 701's first player
/// selected, its settings the defaults plus `aatf_ruleset` and a key it
/// does not read; the context's data folder is the save's scratch folder.
fn tool_with(
    test: &str,
    aatf_ruleset: &str,
) -> (
    crate::Tool,
    studio_core::ToolContext,
    crossbeam_channel::Receiver<studio_core::ShellRequest>,
    PathBuf,
) {
    let save_path = fixture(PesVersion::Pes21, test);
    let dir = save_path.parent().expect("the scratch dir").to_path_buf();
    let mut tool = crate::Tool::new();
    tool.view.open(&save_path);
    tool.view.selected = Some(Selection::Player(701, 0));
    let mut settings = studio_core::Settings::default();
    let mut table = tool.default_settings();
    table.insert(
        "aatf_ruleset".to_owned(),
        toml::Value::String(aatf_ruleset.to_owned()),
    );
    table.insert("unread_key".to_owned(), toml::Value::Integer(3));
    settings.set_tool("save-editor", table);
    let (ctx, _events, requests) = tool_context(&dir, settings);
    (tool, ctx, requests, dir)
}

/// The notices `requests` holds so far, their texts.
fn notices(requests: &crossbeam_channel::Receiver<studio_core::ShellRequest>) -> Vec<String> {
    requests
        .try_iter()
        .filter_map(|request| match request {
            studio_core::ShellRequest::Notify(notice) => Some(notice.text),
            _ => None,
        })
        .collect()
}

/// The dropdown stands left of the Live AATF toggle whether Live AATF is
/// on or off, and lists the built-in rulesets by their descriptions in
/// the templates' order, then Import rules file…; the closed combo shows
/// the active one's.
// SE-GUI-15
#[test]
fn the_dropdown_stands_left_of_live_aatf_and_lists_the_builtins() {
    let path = fixture(PesVersion::Pes17, "ruleset_picker_rows");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    for live in [false, true] {
        view.live_aatf = live;
        let nodes = draw(&mut window, &mut view);
        let combo = dropdown(&nodes).bounds().expect("bounds");
        let toggle = live_toggle(&nodes).bounds().expect("bounds");
        assert!(
            combo.x1 <= toggle.x0,
            "live {live}: the dropdown is left of the toggle ({combo:?} vs {toggle:?})"
        );
        assert_eq!(
            dropdown(&nodes).value(),
            Some("4cc Autumn 26"),
            "live {live}: the closed combo shows the active ruleset"
        );
    }

    let nodes = draw(&mut window, &mut view);
    click(&mut window, dropdown(&nodes), &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let mut expected: Vec<&str> = BUILTINS.to_vec();
    expected.push(IMPORT_ROW);
    assert_eq!(popup_rows(&pairs), expected, "the built-ins, in order");
}

/// In the open dropdown exactly the active ruleset's row reads selected:
/// the active built-in's alone while there are no imports; with an
/// import active, its row's alone (Import rules file…, a command, reads
/// never selected). The separator between the built-ins and the imports
/// stands only when imports exist.
// SE-GUI-15
#[test]
fn only_the_active_rulesets_row_reads_selected() {
    let toggled = |rows: &[DropdownRow]| -> Vec<String> {
        rows.iter()
            .filter(|(_, flag, _)| *flag == Some(accesskit::Toggled::True))
            .map(|(label, _, _)| label.clone())
            .collect()
    };
    // A separator stands between the rows labeled `above` and `below`.
    let gap_split = |rows: &[DropdownRow], separators: &[f64], above: &str, below: &str| -> bool {
        let at = |label: &str| {
            rows.iter()
                .find(|(name, _, _)| name == label)
                .map(|(_, _, bounds)| *bounds)
                .unwrap_or_else(|| panic!("{label:?}'s bounds"))
        };
        let (above, below) = (at(above), at(below));
        separators.iter().any(|&y| above.y1 <= y && y <= below.y0)
    };

    let path = fixture(PesVersion::Pes17, "ruleset_picker_toggles");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();

    // No imports: the active built-in's row alone selected, one
    // separator (before the command row).
    let (rows, separators) = dropdown_state(&mut window, &mut view);
    assert_eq!(
        toggled(&rows),
        ["4cc Autumn 26"],
        "only the active row reads selected: {rows:?}"
    );
    assert_eq!(
        rows.last().map(|(label, flag, _)| (label.as_str(), *flag)),
        Some((IMPORT_ROW, Some(accesskit::Toggled::False))),
        "the command row reads unselected"
    );
    assert_eq!(
        separators.len(),
        1,
        "one separator, before the command row: {separators:?}"
    );
    assert!(
        gap_split(
            &rows,
            &separators,
            "Divegrass Before Christmas 7",
            IMPORT_ROW
        ),
        "between the last built-in and the command"
    );

    // With an import active: its row selected alone, the separator
    // between the built-ins and it (the command's own stands below the
    // popup's fold, as in the no-imports case).
    pick_imported(
        &mut view,
        "cup.4ccrules",
        &aatf::official_file().replacen("name: \"4cc Autumn 26\"", "name: \"Cup Rules\"", 1),
    );
    let (rows, separators) = dropdown_state(&mut window, &mut view);
    assert_eq!(
        toggled(&rows),
        ["Cup Rules"],
        "the import's row alone: {rows:?}"
    );
    assert_eq!(
        rows.last().map(|(label, flag, _)| (label.as_str(), *flag)),
        Some((IMPORT_ROW, Some(accesskit::Toggled::False))),
        "the command row reads unselected"
    );
    assert_eq!(
        separators.len(),
        1,
        "the imports' separator: {separators:?}"
    );
    assert!(
        gap_split(
            &rows,
            &separators,
            "Divegrass Before Christmas 7",
            "Cup Rules"
        ),
        "between the last built-in and the import"
    );
}

/// With no save open the dropdown is drawn too, the ruleset driving
/// nothing yet but already picked.
// SE-GUI-15
#[test]
fn the_dropdown_is_drawn_with_no_save_open() {
    let mut view = SaveView::new();
    let mut window = Headless::new();
    let nodes = draw(&mut window, &mut view);
    assert_eq!(dropdown(&nodes).value(), Some("4cc Autumn 26"));
}

/// No toolbar menu holds an entry for an AATF panel: there is no floating
/// AATF panel, Live AATF being the window's check. The one entry naming
/// AATF is the Savefile menu's Multi-team AATF…, a report on a range of
/// teams.
// SE-GUI-15
#[test]
fn no_toolbar_menu_holds_an_aatf_panel_entry() {
    let path = fixture(PesVersion::Pes17, "ruleset_picker_menus");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(734, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    for menu in ["Player", "Team", "Savefile"] {
        let closed = draw(&mut window, &mut view);
        let before: Vec<String> = closed
            .iter()
            .filter_map(|node| node.label().or(node.value()))
            .map(str::to_owned)
            .collect();
        click(&mut window, menu_button(&closed, menu), &mut view);
        let open = draw(&mut window, &mut view);
        let entries: Vec<&str> = open
            .iter()
            .filter(|node| node.role() == Role::Button)
            .filter_map(|node| node.label())
            .filter(|label| !before.iter().any(|seen| seen == label))
            .collect();
        assert!(!entries.is_empty(), "premise: the {menu} menu opened");
        let aatf: Vec<&str> = entries
            .iter()
            .copied()
            .filter(|entry| entry.contains("AATF"))
            .collect();
        let expected: &[&str] = match menu {
            "Savefile" => &["Multi-team AATF\u{2026}"],
            _ => &[],
        };
        assert_eq!(aatf, expected, "the {menu} menu: {entries:?}");
        window.frame(
            vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
    }
}

/// VTL11 picked in the dropdown writes `aatf_ruleset = "vtl11"`, the
/// tool's other keys unchanged, and the quick actions are VTL11's on the
/// next frame; the tool reopened on that setting shows VTL11's
/// description closed and its quick actions.
// SE-GUI-15
// SE-GUI-32
#[test]
fn a_pick_writes_the_setting_and_a_reopened_tool_reads_it() {
    let (mut tool, ctx, _requests, _dir) = tool_with("ruleset_picker_pick", "autumn26");
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    let nodes = nodes_of(&tool_frame(&mut window, &mut tool, &ctx, Vec::new()));
    assert_eq!(
        make_buttons(&nodes),
        [
            "Make Bronze",
            "Make Goalkeeper",
            "Make Gold",
            "Make Regular",
            "Make Silver"
        ],
        "premise: Autumn 26's tiers"
    );

    // The dropdown opened, VTL11's row clicked through its AccessKit action.
    let at = centre(dropdown(&nodes));
    tool_frame(&mut window, &mut tool, &ctx, pointer_at(at, true));
    tool_frame(&mut window, &mut tool, &ctx, pointer_at(at, false));
    let pairs = tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    let (row, _) = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("/vt/ League 11"))
        .expect("VTL11's row");
    let click = egui::Event::AccessKitActionRequest(accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_tree: accesskit::TreeId::ROOT,
        target_node: *row,
        data: None,
    });
    tool_frame(&mut window, &mut tool, &ctx, vec![click]);
    let table = ctx.tool_settings("save-editor");
    assert_eq!(table["aatf_ruleset"].as_str(), Some("vtl11"), "the pick");
    assert_eq!(
        table["unread_key"].as_integer(),
        Some(3),
        "the tool's other key is unchanged"
    );
    assert_eq!(
        table["live_aatf"].as_bool(),
        Some(false),
        "Live AATF unchanged"
    );
    // The grid's column widths are stored per frame: a settle frame first.
    tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    let nodes = nodes_of(&tool_frame(&mut window, &mut tool, &ctx, Vec::new()));
    let vtl11 = ["Make Buffed", "Make Gold", "Make Regular", "Make Silver"];
    assert_eq!(make_buttons(&nodes), vtl11, "VTL11's quick actions");
    assert_eq!(dropdown(&nodes).value(), Some("/vt/ League 11"));

    // The tool reopened on the same settings: VTL11 closed, its actions.
    let mut reopened = crate::Tool::new();
    reopened
        .view
        .open(tool.view.save_path().expect("a save is open"));
    reopened.view.selected = Some(Selection::Player(701, 0));
    tool_frame(&mut window, &mut reopened, &ctx, Vec::new());
    let nodes = nodes_of(&tool_frame(&mut window, &mut reopened, &ctx, Vec::new()));
    assert_eq!(dropdown(&nodes).value(), Some("/vt/ League 11"));
    assert_eq!(make_buttons(&nodes), vtl11, "VTL11's quick actions again");
}

/// A setting naming an imported copy no longer in the data folder's
/// `rules/` falls back to the first built-in ruleset, with one warning
/// notice saying so, and no load error is drawn anywhere; the setting is
/// left as it was until a member picks another.
// SE-GUI-32
#[test]
fn a_copy_that_is_gone_falls_back_with_a_notice_and_no_load_error() {
    let (mut tool, ctx, requests, dir) = tool_with("ruleset_picker_gone", "cup.4ccrules");
    std::fs::create_dir_all(dir.join("rules")).unwrap();
    assert!(
        !dir.join("rules").join("cup.4ccrules").exists(),
        "premise: the copy is gone"
    );
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    let nodes = nodes_of(&tool_frame(&mut window, &mut tool, &ctx, Vec::new()));
    assert_eq!(
        notices(&requests),
        [
            "The AATF ruleset picked last is gone from the data folder: 4cc Autumn 26 is used \
          instead."
        ],
        "one warning notice, shown once"
    );
    assert_eq!(dropdown(&nodes).value(), Some("4cc Autumn 26"));
    assert_eq!(
        make_buttons(&nodes),
        [
            "Make Bronze",
            "Make Goalkeeper",
            "Make Gold",
            "Make Regular",
            "Make Silver"
        ],
        "Autumn 26's quick actions"
    );
    assert!(
        !nodes.iter().any(|node| {
            node.label()
                .or(node.value())
                .is_some_and(|text| text.contains("could not be loaded"))
        }),
        "no load error anywhere"
    );
    assert_eq!(
        ctx.tool_settings("save-editor")["aatf_ruleset"].as_str(),
        Some("cup.4ccrules"),
        "the setting is left as it was"
    );
}

/// An imported copy in the data folder's `rules/` is listed under the
/// built-ins after a separator, and a setting naming it picks it.
// SE-GUI-32
#[test]
fn an_imported_copy_is_listed_after_the_builtins_and_picked() {
    let (mut tool, ctx, requests, dir) = tool_with("ruleset_picker_import", "cup.4ccrules");
    std::fs::create_dir_all(dir.join("rules")).unwrap();
    std::fs::write(
        dir.join("rules").join("cup.4ccrules"),
        aatf::official_file().replacen("name: \"4cc Autumn 26\"", "name: \"Cup Rules\"", 1),
    )
    .unwrap();
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    let nodes = nodes_of(&tool_frame(&mut window, &mut tool, &ctx, Vec::new()));
    assert_eq!(dropdown(&nodes).value(), Some("Cup Rules"));
    assert!(
        notices(&requests).is_empty(),
        "no notice: the copy is there"
    );

    let at = centre(dropdown(&nodes));
    tool_frame(&mut window, &mut tool, &ctx, pointer_at(at, true));
    tool_frame(&mut window, &mut tool, &ctx, pointer_at(at, false));
    let pairs = tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    let mut expected: Vec<&str> = BUILTINS.to_vec();
    expected.extend(["Cup Rules", "Remove Cup Rules", IMPORT_ROW]);
    assert_eq!(popup_rows(&pairs), expected);
}

// The wheel over the closed dropdown steps between the rulesets alone:
// from the second-to-last built-in, a notch down picks the last, the next
// the imported copy, and a third stays there, never reaching Import rules
// file… below it.
// SE-GUI-15
#[test]
fn the_wheel_over_the_closed_dropdown_steps_the_rulesets_and_never_imports() {
    let mut view = SaveView::new();
    pick_imported(&mut view, "cup.4ccrules", &aatf::official_file());
    let stems: Vec<&str> = crate::session::rules::builtins()
        .iter()
        .map(|(stem, _)| *stem)
        .collect();
    let last = stems.len() - 1;
    view.aatf_ruleset = stems[last - 1].to_owned();
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let at = centre(dropdown(&nodes));
    window.frame(vec![egui::Event::PointerMoved(at)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let notch_down = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -1.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::default(),
        source: egui::MouseWheelSource::Wheel,
    };
    for expected in [stems[last], "cup.4ccrules", "cup.4ccrules"] {
        window.frame(vec![notch_down.clone()], |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
        for _ in 0..3 {
            window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        }
        assert_eq!(view.aatf_ruleset, expected, "a notch down");
        assert!(view.rules_dialog.is_none(), "the wheel never imports");
    }
}

/// The settings menu's section holds no rules file row: the ruleset is
/// the dropdown's alone, and the section's one row is the aesthetics lock.
// SE-GUI-32
#[test]
fn the_settings_section_holds_no_rules_file_row() {
    let (mut tool, ctx, _requests, _dir) = tool_with("ruleset_picker_settings", "autumn26");
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| tool.settings_view(ui, &ctx));
    assert!(
        label_value(&nodes, "Aesthetics from exports"),
        "premise: the section is drawn"
    );
    let rules_text: Vec<&str> = nodes
        .iter()
        .filter_map(|node| node.label().or(node.value()))
        .filter(|text| text.contains("AATF") || text.contains("rules"))
        .collect();
    assert!(rules_text.is_empty(), "no rules row: {rules_text:?}");
    assert!(
        !nodes.iter().any(|node| node.role() == Role::TextInput),
        "no path edit"
    );
}

mod import;
