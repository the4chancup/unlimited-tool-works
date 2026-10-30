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

/// The open dropdown's rows, top to bottom: the Buttons inside its list
/// box, an imported row's × right after its row.
fn popup_rows(pairs: &[(accesskit::NodeId, accesskit::Node)]) -> Vec<String> {
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        pairs.iter().map(|(id, node)| (*id, node)).collect();
    let (list, _) = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::ListBox)
        .expect("the dropdown's open list");
    let mut rows: Vec<(f64, f64, String)> = Vec::new();
    let mut stack = vec![*list];
    while let Some(id) = stack.pop() {
        let node = by_id[&id];
        stack.extend(node.children());
        if node.role() == Role::Button
            && let (Some(label), Some(bounds)) = (node.label(), node.bounds())
        {
            rows.push((bounds.y0, bounds.x0, label.to_owned()));
        }
    }
    rows.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .expect("ordered bounds")
            .then(a.1.partial_cmp(&b.1).expect("ordered bounds"))
    });
    rows.into_iter().map(|(_, _, label)| label).collect()
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

/// The official file with its description `description`.
fn renamed(description: &str) -> String {
    let text = aatf::official_file().replacen(
        "name: \"4cc Autumn 26\"",
        &format!("name: \"{description}\""),
        1,
    );
    assert!(text.contains(description), "premise: the name was replaced");
    text
}

/// `text` written as `folder`'s `picked/<name>`, outside `rules/`: the file
/// a member picks to import. Its path.
fn picked(folder: &Path, name: &str, text: &str) -> PathBuf {
    let folder = folder.join("picked");
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join(name);
    std::fs::write(&path, text).unwrap();
    path
}

/// `text` placed in `dir`'s `rules/` as `name`: a ruleset imported before
/// the tool's first frame.
fn place_import(dir: &Path, name: &str, text: &str) {
    std::fs::create_dir_all(dir.join("rules")).unwrap();
    std::fs::write(dir.join("rules").join(name), text).unwrap();
}

/// `tool`'s Import rules file… picker answering `pick`; the count of its
/// calls.
fn stub_picker(tool: &mut crate::Tool, pick: Option<PathBuf>) -> std::rc::Rc<std::cell::Cell<u32>> {
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = std::rc::Rc::clone(&calls);
    tool.view.rules_file_dialog = Box::new(move || {
        counted.set(counted.get() + 1);
        pick.clone()
    });
    calls
}

/// The dropdown open: a click on the closed combo unless its list is
/// already up. The open frame's node pairs.
fn open_dropdown(
    window: &mut Headless,
    tool: &mut crate::Tool,
    ctx: &studio_core::ToolContext,
) -> Vec<(accesskit::NodeId, accesskit::Node)> {
    let pairs = tool_frame(window, tool, ctx, Vec::new());
    if pairs.iter().any(|(_, node)| node.role() == Role::ListBox) {
        return pairs;
    }
    let at = centre(dropdown(&nodes_of(&pairs)));
    tool_frame(window, tool, ctx, pointer_at(at, true));
    tool_frame(window, tool, ctx, pointer_at(at, false));
    tool_frame(window, tool, ctx, Vec::new())
}

/// The open dropdown's Button labeled `label` (a row, an ×, the command)
/// clicked through its AccessKit action.
fn click_in_dropdown(
    window: &mut Headless,
    tool: &mut crate::Tool,
    ctx: &studio_core::ToolContext,
    label: &str,
) {
    let pairs = open_dropdown(window, tool, ctx);
    let (id, _) = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(label))
        .unwrap_or_else(|| panic!("no {label:?} in the open dropdown"));
    let click = egui::Event::AccessKitActionRequest(accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_tree: accesskit::TreeId::ROOT,
        target_node: *id,
        data: None,
    });
    tool_frame(window, tool, ctx, vec![click]);
}

/// The modal holding `heading`: its Label values and Button labels, top
/// to bottom and left to right.
fn dialog_lines(pairs: &[(accesskit::NodeId, accesskit::Node)], heading: &str) -> Vec<String> {
    let mut lines: Vec<(f64, f64, String)> = super::prompt::dialog_nodes(pairs, heading)
        .into_iter()
        .filter_map(|node| {
            let text = match node.role() {
                Role::Label => node.value(),
                Role::Button => node.label(),
                _ => None,
            }?;
            let bounds = node.bounds()?;
            Some((bounds.y0, bounds.x0, text.to_owned()))
        })
        .collect();
    lines.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .expect("drawn bounds")
            .then(a.1.partial_cmp(&b.1).expect("drawn bounds"))
    });
    lines.into_iter().map(|(_, _, text)| text).collect()
}

/// A pointer click on the Button `label` of the modal holding `heading`.
fn click_in_dialog(
    window: &mut Headless,
    tool: &mut crate::Tool,
    ctx: &studio_core::ToolContext,
    heading: &str,
    label: &str,
) {
    let pairs = tool_frame(window, tool, ctx, Vec::new());
    let button = super::prompt::dialog_nodes(&pairs, heading)
        .into_iter()
        .find(|node| node.role() == Role::Button && node.label() == Some(label))
        .unwrap_or_else(|| panic!("no {label:?} in {heading:?}"));
    let at = centre(button);
    tool_frame(window, tool, ctx, pointer_at(at, true));
    tool_frame(window, tool, ctx, pointer_at(at, false));
}

/// Whether a modal is up.
fn dialog_up(pairs: &[(accesskit::NodeId, accesskit::Node)]) -> bool {
    pairs.iter().any(|(_, node)| node.role() == Role::Dialog)
}

/// The open dropdown's rows expected: the built-ins, `imported` (each its
/// row then its ×) and the command.
fn rows_with(imported: &[&str]) -> Vec<String> {
    let mut rows: Vec<String> = BUILTINS.iter().map(|name| (*name).to_owned()).collect();
    for name in imported {
        rows.push((*name).to_owned());
        rows.push(format!("Remove {name}"));
    }
    rows.push(IMPORT_ROW.to_owned());
    rows
}

/// Import rules file… is a command, not a value: picking it opens the
/// file picker, and a cancelled pick leaves the ruleset, the setting and
/// the data folder as they were, with no dialog.
// SE-GUI-15
#[test]
fn import_rules_file_opens_the_picker_and_never_becomes_the_selection() {
    let (mut tool, ctx, requests, dir) = tool_with("ruleset_import_command", "autumn26");
    let calls = stub_picker(&mut tool, None);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    click_in_dropdown(&mut window, &mut tool, &ctx, IMPORT_ROW);
    assert_eq!(calls.get(), 1, "the picker opened once");
    let pairs = tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    assert_eq!(dropdown(&nodes_of(&pairs)).value(), Some("4cc Autumn 26"));
    assert_eq!(
        ctx.tool_settings("save-editor")["aatf_ruleset"].as_str(),
        Some("autumn26")
    );
    assert!(!dialog_up(&pairs), "no dialog");
    assert!(notices(&requests).is_empty(), "no notice");
    assert!(!dir.join("rules").exists(), "nothing copied");
}

/// A file the loader refuses: a dialog lists its findings, one a line in
/// the monospace font, each with its key path; nothing is copied, and
/// after OK the list is as it was.
// SE-GUI-15
#[test]
fn a_file_with_errors_is_refused_listing_its_findings() {
    let (mut tool, ctx, _requests, dir) = tool_with("ruleset_import_errors", "autumn26");
    let text = renamed("4cc Autumn 25")
        .replacen(
            "label: \"Gold\", count: 1,",
            "label: \"Gold\", count: 21,",
            1,
        )
        .replacen("suggestions: [\"cards\"", "suggestions: [\"bogus\"", 1);
    stub_picker(&mut tool, Some(picked(&dir, "autumn25.4ccrules", &text)));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    click_in_dropdown(&mut window, &mut tool, &ctx, IMPORT_ROW);
    let findings = [
        "tiers: counts total 25 over the squad of 23",
        "suggestions[0]: unknown suggestion \"bogus\"",
    ];
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| tool.view(ui, &ctx));
    assert_eq!(
        dialog_lines(&pairs, "The rules file has errors"),
        [
            "The rules file has errors",
            "autumn25.4ccrules was not imported. Fix these errors, then import it again:",
            findings[0],
            findings[1],
            "OK"
        ]
    );
    for finding in findings {
        let family = &text_shape(&shapes, finding).galley.job.sections[0]
            .format
            .font_id
            .family;
        assert_eq!(*family, egui::FontFamily::Monospace, "{finding}");
    }
    assert!(!dir.join("rules").exists(), "nothing copied");

    click_in_dialog(
        &mut window,
        &mut tool,
        &ctx,
        "The rules file has errors",
        "OK",
    );
    assert!(
        !dialog_up(&tool_frame(&mut window, &mut tool, &ctx, Vec::new())),
        "OK closed it"
    );
    let pairs = open_dropdown(&mut window, &mut tool, &ctx);
    assert_eq!(popup_rows(&pairs), rows_with(&[]), "the list as it was");
}

/// A clean file named like no built-in is copied into the data folder's
/// `rules/`, listed under the built-ins with an × at its row's right end,
/// and picked, with a success notice. The same file name imported again
/// asks first, naming the import it replaces: Cancel changes nothing;
/// Replace leaves one row, the new file's, which is the ruleset in use.
// SE-GUI-15
#[test]
fn a_clean_file_is_copied_listed_and_picked_and_a_reimport_replaces_it() {
    let (mut tool, ctx, requests, dir) = tool_with("ruleset_import_clean", "autumn26");
    let first = renamed("4cc Autumn 25");
    stub_picker(&mut tool, Some(picked(&dir, "autumn25.4ccrules", &first)));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    click_in_dropdown(&mut window, &mut tool, &ctx, IMPORT_ROW);
    let copy = dir.join("rules").join("autumn25.4ccrules");
    assert_eq!(std::fs::read_to_string(&copy).unwrap(), first, "copied");
    assert_eq!(
        ctx.tool_settings("save-editor")["aatf_ruleset"].as_str(),
        Some("autumn25.4ccrules"),
        "picked"
    );
    let success: Vec<(String, studio_core::NoticeKind)> = requests
        .try_iter()
        .filter_map(|request| match request {
            studio_core::ShellRequest::Notify(notice) => Some((notice.text, notice.kind)),
            _ => None,
        })
        .collect();
    assert_eq!(
        success,
        [(
            "Imported 4cc Autumn 25.".to_owned(),
            studio_core::NoticeKind::Success
        )]
    );
    let nodes = nodes_of(&tool_frame(&mut window, &mut tool, &ctx, Vec::new()));
    assert_eq!(dropdown(&nodes).value(), Some("4cc Autumn 25"));
    let pairs = open_dropdown(&mut window, &mut tool, &ctx);
    assert_eq!(popup_rows(&pairs), rows_with(&["4cc Autumn 25"]));
    let bounds = |label: &str| {
        pairs
            .iter()
            .find(|(_, node)| node.role() == Role::Button && node.label() == Some(label))
            .and_then(|(_, node)| node.bounds())
            .unwrap_or_else(|| panic!("{label:?}'s bounds"))
    };
    let (row, remove) = (bounds("4cc Autumn 25"), bounds("Remove 4cc Autumn 25"));
    assert!(
        remove.y0 >= row.y0
            && remove.y1 <= row.y1
            && remove.x1 <= row.x1 + 0.5
            && remove.x1 >= row.x1 - 4.0,
        "the × at its row's right end ({remove:?} in {row:?})"
    );

    // The corrected rules, under the same file name.
    let corrected = renamed("4cc Autumn 25 corrected");
    stub_picker(
        &mut tool,
        Some(picked(&dir.join("v2"), "autumn25.4ccrules", &corrected)),
    );
    let replace = "Replace an imported ruleset?";
    click_in_dropdown(&mut window, &mut tool, &ctx, IMPORT_ROW);
    let pairs = tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    assert_eq!(
        dialog_lines(&pairs, replace),
        [
            replace,
            "Importing autumn25.4ccrules replaces 4cc Autumn 25 (autumn25.4ccrules).",
            "Replace",
            "Cancel"
        ]
    );
    assert_eq!(
        std::fs::read_to_string(&copy).unwrap(),
        first,
        "nothing written before the answer"
    );
    click_in_dialog(&mut window, &mut tool, &ctx, replace, "Cancel");
    assert!(!dialog_up(&tool_frame(
        &mut window,
        &mut tool,
        &ctx,
        Vec::new()
    )));
    assert_eq!(
        std::fs::read_to_string(&copy).unwrap(),
        first,
        "Cancel wrote nothing"
    );
    let pairs = open_dropdown(&mut window, &mut tool, &ctx);
    assert_eq!(popup_rows(&pairs), rows_with(&["4cc Autumn 25"]));

    click_in_dropdown(&mut window, &mut tool, &ctx, IMPORT_ROW);
    click_in_dialog(&mut window, &mut tool, &ctx, replace, "Replace");
    assert_eq!(std::fs::read_to_string(&copy).unwrap(), corrected);
    assert_eq!(notices(&requests), ["Imported 4cc Autumn 25 corrected."]);
    let nodes = nodes_of(&tool_frame(&mut window, &mut tool, &ctx, Vec::new()));
    assert_eq!(
        dropdown(&nodes).value(),
        Some("4cc Autumn 25 corrected"),
        "the new copy is the ruleset in use"
    );
    let pairs = open_dropdown(&mut window, &mut tool, &ctx);
    assert_eq!(popup_rows(&pairs), rows_with(&["4cc Autumn 25 corrected"]));
}

/// An imported row whose description is longer than every built-in's
/// ends its text clear of its ×, and the ×'s glyph is drawn inside the
/// ×'s own bounds.
// SE-GUI-15
#[test]
fn an_imported_rows_text_stays_clear_of_its_x() {
    let long = "Divegrass Before Christmas 7 and a half";
    assert!(
        BUILTINS.iter().all(|name| name.len() < long.len()),
        "premise: longer than every built-in"
    );
    let (mut tool, ctx, _requests, dir) = tool_with("ruleset_import_clear_of_x", "autumn26");
    place_import(&dir, "dbc7half.4ccrules", &renamed(long));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    open_dropdown(&mut window, &mut tool, &ctx);
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| tool.view(ui, &ctx));
    let remove = pairs
        .iter()
        .find(|(_, node)| {
            node.role() == Role::Button && node.label() == Some(&format!("Remove {long}"))
        })
        .and_then(|(_, node)| node.bounds())
        .expect("the ×'s bounds");
    let text = text_shape(&shapes, long).visual_bounding_rect();
    assert!(
        f64::from(text.max.x) <= remove.x0 - 2.0,
        "the row's text ends at {} and the × starts at {}",
        text.max.x,
        remove.x0
    );
    let glyph = text_shape(&shapes, studio_core::icons::X).visual_bounding_rect();
    assert!(
        f64::from(glyph.min.x) >= remove.x0
            && f64::from(glyph.max.x) <= remove.x1
            && f64::from(glyph.min.y) >= remove.y0
            && f64::from(glyph.max.y) <= remove.y1,
        "the × glyph {glyph:?} inside its bounds {remove:?}"
    );
}

/// A file matching one import by its file name and another by its
/// description names both in the confirmation; Replace leaves one row.
// SE-GUI-15
#[test]
fn a_reimport_matching_two_imports_names_both() {
    let (mut tool, ctx, _requests, dir) = tool_with("ruleset_import_two", "autumn26");
    place_import(&dir, "autumn25.4ccrules", &renamed("4cc Autumn 25"));
    place_import(&dir, "vgl25.4ccrules", &renamed("/vg/ League 25"));
    stub_picker(
        &mut tool,
        Some(picked(
            &dir,
            "autumn25.4ccrules",
            &renamed("/vg/ League 25"),
        )),
    );
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    click_in_dropdown(&mut window, &mut tool, &ctx, IMPORT_ROW);
    let replace = "Replace an imported ruleset?";
    let pairs = tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    assert_eq!(
        dialog_lines(&pairs, replace)[1],
        "Importing autumn25.4ccrules replaces /vg/ League 25 (vgl25.4ccrules) and 4cc Autumn 25 \
         (autumn25.4ccrules)."
    );
    click_in_dialog(&mut window, &mut tool, &ctx, replace, "Replace");
    assert!(!dir.join("rules").join("vgl25.4ccrules").exists());
    let pairs = open_dropdown(&mut window, &mut tool, &ctx);
    assert_eq!(popup_rows(&pairs), rows_with(&["/vg/ League 25"]));
}

/// A file whose description a built-in ruleset has is refused naming the
/// built-in, and nothing is copied.
// SE-GUI-15
#[test]
fn a_file_named_like_a_builtin_is_refused_naming_it() {
    let (mut tool, ctx, _requests, dir) = tool_with("ruleset_import_builtin", "autumn26");
    stub_picker(
        &mut tool,
        Some(picked(&dir, "vgl27.4ccrules", &renamed("/vg/ League 27"))),
    );
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    click_in_dropdown(&mut window, &mut tool, &ctx, IMPORT_ROW);
    let title = "The rules file was not imported";
    let pairs = tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    assert_eq!(
        dialog_lines(&pairs, title),
        [
            title,
            "vgl27.4ccrules is named /vg/ League 27, as a built-in ruleset is. Rename it in its \
             RULESET block, then import it again.",
            "OK"
        ]
    );
    assert!(!dir.join("rules").exists(), "nothing copied");
    assert_eq!(
        ctx.tool_settings("save-editor")["aatf_ruleset"].as_str(),
        Some("autumn26")
    );
}

/// With no data folder there is nowhere to keep a copy: a clean file is
/// refused saying so.
// SE-GUI-15
#[test]
fn with_no_data_folder_an_import_is_refused() {
    let dir = crate::test_support::scratch("ruleset_import_no_data_folder");
    let mut view = SaveView::new();
    view.rules_file_dialog = {
        let path = picked(&dir, "autumn25.4ccrules", &renamed("4cc Autumn 25"));
        Box::new(move || Some(path.clone()))
    };
    let mut window = Headless::new();
    let nodes = draw(&mut window, &mut view);
    click(&mut window, dropdown(&nodes), &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (row, _) = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(IMPORT_ROW))
        .expect("the command row");
    action_click(&mut window, &mut view, *row);
    let title = "The rules file was not imported";
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        dialog_lines(&pairs, title),
        [
            title,
            "autumn25.4ccrules was not imported because there is no data folder to keep its \
             copy in.",
            "OK"
        ]
    );
    assert_eq!(view.aatf_ruleset, "autumn26");
}

/// The imported row's × asks first, naming the ruleset and saying its
/// copy is deleted and the file it came from stays; Cancel keeps it;
/// Remove deletes the copy, the row is gone and, the ruleset having been
/// in use, the first built-in one is, with no notice.
// SE-GUI-15
#[test]
fn the_x_asks_then_deletes_the_copy_and_falls_back_to_the_first_builtin() {
    let (mut tool, ctx, requests, dir) = tool_with("ruleset_import_remove", "autumn25.4ccrules");
    place_import(&dir, "autumn25.4ccrules", &renamed("4cc Autumn 25"));
    let copy = dir.join("rules").join("autumn25.4ccrules");
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    let nodes = nodes_of(&tool_frame(&mut window, &mut tool, &ctx, Vec::new()));
    assert_eq!(
        dropdown(&nodes).value(),
        Some("4cc Autumn 25"),
        "premise: the import is in use"
    );

    let title = "Remove an imported ruleset?";
    click_in_dropdown(&mut window, &mut tool, &ctx, "Remove 4cc Autumn 25");
    let pairs = tool_frame(&mut window, &mut tool, &ctx, Vec::new());
    assert_eq!(
        dialog_lines(&pairs, title),
        [
            title,
            "Remove 4cc Autumn 25 from the list? Its copy in the data folder is deleted; the \
             file you imported it from (autumn25.4ccrules) stays.",
            "Remove",
            "Cancel"
        ]
    );
    assert!(copy.exists(), "nothing deleted before the answer");
    click_in_dialog(&mut window, &mut tool, &ctx, title, "Cancel");
    assert!(copy.exists(), "Cancel deleted nothing");

    click_in_dropdown(&mut window, &mut tool, &ctx, "Remove 4cc Autumn 25");
    click_in_dialog(&mut window, &mut tool, &ctx, title, "Remove");
    assert!(!copy.exists(), "the copy is deleted");
    assert_eq!(
        ctx.tool_settings("save-editor")["aatf_ruleset"].as_str(),
        Some("autumn26")
    );
    let nodes = nodes_of(&tool_frame(&mut window, &mut tool, &ctx, Vec::new()));
    assert_eq!(dropdown(&nodes).value(), Some("4cc Autumn 26"));
    let pairs = open_dropdown(&mut window, &mut tool, &ctx);
    assert_eq!(popup_rows(&pairs), rows_with(&[]), "the row is gone");
    assert!(notices(&requests).is_empty(), "no notice");
}
