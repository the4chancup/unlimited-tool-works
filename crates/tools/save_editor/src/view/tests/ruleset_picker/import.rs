use super::*;

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
