//! The "Report a problem" dialog's tests: the tail entry opening it (rail and
//! overlay), the "The file holds:" list, the Save… write and its answers, the
//! tool's line, `installs.txt`, and the two formatters (the `hh:mm` of an
//! output's write and the suggested file name).

use std::path::Path;
use std::sync::Arc;

use egui::accesskit::{self, Role};

use super::*;
use crate::repro::{
    SessionFile, SessionHeader, SessionMessage, SessionWriter, session_folder_name,
};
use crate::settings::SETTINGS_FILE_NAME;
use crate::shell::sidebar::REPORT_LABEL;
use crate::shell::test_harness::*;
use crate::tool::ToolContext;

/// Records `tool`'s three steps, one kept input and one kept output through
/// `ctx`'s shared journal, as the stub tools' session work would.
fn record(ctx: &ToolContext, tool: &'static str, dir: &Path) {
    record_named(ctx, tool, dir, "EDIT00000000");
}

/// `record` with the save's file named `name` (a member's renamed copy).
fn record_named(ctx: &ToolContext, tool: &'static str, dir: &Path, name: &str) {
    let input = dir.join(name);
    let output = dir.join("out").join(name);
    ctx.recorder()
        .step(tool, format!("opened {} (PES 2021)", input.display()));
    ctx.recorder().step(tool, "player 70101: Height 180 → 190");
    ctx.recorder()
        .step(tool, format!("wrote {}", output.display()));
    // 7.4 MB as the dialog's "opened" clause writes it.
    ctx.recorder()
        .keep_input(tool, &input, Arc::from(vec![0u8; 7_759_462].as_slice()));
    ctx.recorder()
        .keep_output(tool, &output, Arc::from(&b"OUT"[..]));
}

/// The text node carrying `needle`, for assertions on a line's full text.
fn text_line<'a>(nodes: &'a [accesskit::Node], needle: &str) -> &'a str {
    nodes
        .iter()
        .filter_map(|node| node.value())
        .find(|value| value.contains(needle))
        .unwrap_or_else(|| panic!("no text node holds {needle:?}"))
}

/// The words box's node (the dialog's one text field).
fn words_box(nodes: &[accesskit::Node]) -> &accesskit::Node {
    nodes
        .iter()
        .find(|node| matches!(node.role(), Role::MultilineTextInput | Role::TextInput))
        .expect("the words box is drawn")
}

/// The dialog's opening assertions: the modal's title, the draft's fixed
/// lines and its two buttons.
fn assert_dialog_open(nodes: &[accesskit::Node]) {
    text_node(nodes, "Report a problem");
    assert!(
        has_text(
            nodes,
            "Saves what UTW did since it opened, so the problem can be reproduced."
        ),
        "the intro line"
    );
    assert!(
        has_text(
            nodes,
            "The file is not sent automatically, please post it wherever you usually report \
             problems."
        ),
        "the second line"
    );
    assert!(has_text(nodes, "What happened?"), "the box's heading");
    assert_eq!(
        words_box(nodes).placeholder(),
        Some(
            "What you did, what you expected and what you got; also anything done \
             outside UTW, in the game or another tool"
        ),
        "the box's hint"
    );
    assert!(has_text(nodes, "The file holds:"), "the list's heading");
    assert!(
        has_text(nodes, "the files as they were opened, before any change"),
        "the files' line"
    );
    assert!(
        has_text(
            nodes,
            "utw.log, the version, your settings and the PES installs found"
        ),
        "the log's line"
    );
    assert_eq!(
        text_line(nodes, "Your user folder"),
        "Your user folder is written as ~ in the file's paths.",
        "the privacy line"
    );
    assert!(
        has_label(nodes, "Save…") && has_label(nodes, "Cancel"),
        "the two answers"
    );
}

// ST-REP-01
#[test]
fn the_entry_opens_the_dialog_from_the_rail_and_the_opened_sidebar() {
    let dir = TestDir::new("studio_core_report_open");
    let Shell {
        mut app, ctx, _dir, ..
    } = shell();
    let egui_ctx = test_context();
    record(&ctx, "a", dir.path());

    // The rail's entry.
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    // A modal's opening frame is an invisible sizing pass; the next draws it.
    let out = frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_dialog_open(&out.nodes);

    // The tool's line names its steps, its input's file name and size, and
    // its output's name with the write's local `hh:mm`.
    let line = text_line(&out.nodes, "A: 3 steps").to_owned();
    assert!(
        line.contains("; opened EDIT00000000 (7.4 MB)"),
        "the opened clause: {line}"
    );
    let wrote = line
        .rsplit("; wrote ")
        .next()
        .expect("the wrote clause is present");
    assert!(
        wrote.starts_with("EDIT00000000 at "),
        "the wrote clause: {line}"
    );
    let clock = &wrote[wrote.len() - 5..];
    assert!(
        clock.chars().nth(2) == Some(':')
            && clock.chars().filter(|c| c.is_ascii_digit()).count() == 4,
        "the write's time is `hh:mm`: {line}"
    );

    // The opened sidebar's entry opens the same dialog.
    let Shell { mut app, _dir, .. } = shell();
    record(&app.ctx, "a", dir.path());
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    let nodes = open_sidebar(&egui_ctx, &mut app, entry(&nodes, "A"), 1.0).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let out = frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_dialog_open(&out.nodes);
    assert!(
        has_text(&out.nodes, "A: 3 steps; opened EDIT00000000 (7.4 MB)"),
        "the tool's line"
    );
}

// The opened clause names the first input alone and counts the rest: the
// three kept inputs read ", and 2 more".
#[test]
fn the_opened_clause_names_the_first_input_and_counts_the_rest() {
    let dir = TestDir::new("studio_core_report_more_inputs");
    let Shell {
        mut app, ctx, _dir, ..
    } = shell();
    let egui_ctx = test_context();
    record(&ctx, "a", dir.path());
    ctx.recorder().keep_input(
        "a",
        &dir.path().join("EDIT00000001"),
        Arc::from(&b"SECOND"[..]),
    );
    ctx.recorder().keep_input(
        "a",
        &dir.path().join("EDIT00000002"),
        Arc::from(&b"THIRD"[..]),
    );

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let out = frame(&egui_ctx, &mut app, Vec::new(), true);
    assert!(
        has_text(
            &out.nodes,
            "A: 3 steps; opened EDIT00000000 (7.4 MB), and 2 more"
        ),
        "the opened clause counts the rest: {:?}",
        out.nodes
            .iter()
            .filter_map(|node| node.value())
            .find(|value| value.contains("steps"))
    );
}

// ST-REP-01's "no tool has recorded anything" clause: the list holds the
// fixed lines only.
// ST-REP-01
#[test]
fn the_list_holds_the_fixed_lines_only_when_nothing_was_recorded() {
    let Shell {
        mut app,
        ctx: _,
        _dir,
        ..
    } = shell();
    let egui_ctx = test_context();
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let out = frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_dialog_open(&out.nodes);
    assert_eq!(
        out.nodes
            .iter()
            .filter(|node| node.role() == Role::Label && node.value() == Some("•"))
            .count(),
        2,
        "the draft's two fixed lines only"
    );
    assert!(
        !out.nodes
            .iter()
            .filter_map(|node| node.value())
            .any(|value| value.contains(" steps")),
        "no tool line is listed"
    );
}

// ST-REP-02
#[test]
fn save_writes_the_bundle_at_the_picked_path_and_the_words_stay() {
    let dir = TestDir::new("studio_core_report_bundle");
    let home = dir.path().join("member");
    fs::create_dir_all(&home).unwrap();
    let out = home.join("utw-report.zip");
    let Shell {
        mut app,
        ctx,
        calls,
        _dir,
        ..
    } = shell();
    let egui_ctx = test_context();
    record(&ctx, "a", dir.path());
    *calls[0].repro_state.lock().unwrap() =
        vec![("EDIT00000000".to_owned(), b"THE SAVE AS NOW".to_vec())];
    *calls[0].repro_readme.lock().unwrap() = Some("STUB PART\n");
    // The window's log, and the settings file the shell's first frame
    // writes, which the bundle's settings are compared against.
    let log = dir.path().join("utw.log");
    fs::write(&log, "the window's log lines\n").unwrap();
    app = app.with_log_file(Some(log));
    let settings_file = ctx
        .paths()
        .data_dir
        .as_ref()
        .unwrap()
        .join(SETTINGS_FILE_NAME);
    app.report.home = Some(home.display().to_string());
    let words = format!("it broke in {}", home.display());

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    // A setting changed in the session and not saved, as after a save that
    // failed: the bundle's settings are the ones the session runs under.
    ctx.settings().lock().unwrap().common.thread_count = 3;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    // A modal's opening frame is an invisible sizing pass; the next draws it.
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, words_box(&nodes));
    frame(
        &egui_ctx,
        &mut app,
        vec![egui::Event::Text(words.clone())],
        true,
    );
    let out_path = out.clone();
    app.report.save_dialog = Box::new(move |_| Some(out_path.clone()));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "Save…"));
    assert!(out.exists(), "the zip was written");
    // The "Saved to" line and the Close-only buttons draw on the next frame.
    let out_nodes = frame(&egui_ctx, &mut app, Vec::new(), true);

    // The dialog says where and offers only Close.
    let expected = format!(
        "Saved to ~{}{}",
        std::path::MAIN_SEPARATOR,
        out.file_name().unwrap().to_string_lossy()
    );
    assert_eq!(
        text_line(&out_nodes.nodes, "Saved to"),
        expected,
        "the saved line writes the home as ~ and names the file"
    );
    assert!(
        has_label(&out_nodes.nodes, "Close"),
        "only Close is offered"
    );
    assert!(
        !has_label(&out_nodes.nodes, "Save…"),
        "no Save… once written"
    );
    assert!(
        !has_label(&out_nodes.nodes, "Cancel"),
        "no Cancel once written"
    );

    // The zip: journal.toml with the three steps, the input under inputs/,
    // the output under outputs/, the stub's repro_state under state/, the
    // log, the settings, installs.txt and the words with ~ written.
    let zip = crate::repro::tests::zip_text;
    let journal = crate::repro::tests::journal_doc(&zip(&out, "journal.toml"));
    let steps = journal["step"].as_array().expect("the steps' table");
    assert_eq!(steps.len(), 3, "the three recorded steps");
    assert_eq!(zip(&out, "inputs/0-EDIT00000000").len(), 7_759_462);
    assert_eq!(zip(&out, "outputs/0-EDIT00000000"), "OUT");
    assert_eq!(
        zip(&out, "state/a/EDIT00000000"),
        "THE SAVE AS NOW",
        "the stub's repro_state prefixed by its tool id"
    );
    assert_eq!(zip(&out, "utw.log"), "the window's log lines\n");
    assert_eq!(
        zip(&out, "README.txt"),
        format!("{README_CORE}\nSTUB PART\n"),
        "the core part, a blank line, then the stub's part"
    );
    // Spelled out, not built from `README_CORE`: a core part that lost its
    // last newline would join the stub's part with no blank line between.
    assert!(
        zip(&out, "README.txt").ends_with("first line.\n\nSTUB PART\n"),
        "a blank line stands between the core part and the stub's"
    );
    // The readme sends its reader to this line for the version that ran.
    assert!(
        zip(&out, "journal.toml").starts_with("version = "),
        "journal.toml's first line is the version"
    );
    let thread_count = |text: &str| {
        toml::from_str::<toml::Table>(text).unwrap()["common"]["thread_count"].as_integer()
    };
    assert_ne!(
        thread_count(&fs::read_to_string(&settings_file).unwrap()),
        Some(3),
        "premise: the file on disk does not hold the session's change"
    );
    assert_eq!(
        thread_count(&zip(&out, "settings.toml")),
        Some(3),
        "the settings the session runs under"
    );
    let installs = zip(&out, "installs.txt");
    assert!(
        installs.lines().count() == 7 && installs.contains("PES 2021:"),
        "installs.txt at the zip's root lists every version the selector offers: {installs}"
    );
    let state: Vec<&str> = journal["state"]
        .as_array()
        .expect("the stub's state is listed")
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    assert_eq!(
        state,
        ["a/EDIT00000000"],
        "`state` lists the tools' files alone"
    );
    assert_eq!(
        zip(&out, "words.txt"),
        "it broke in ~",
        "the words write the home as ~"
    );

    // Closed and opened again: the words are still there, and the dialog is
    // fresh for a new report: Save… again and no "Saved to".
    click(&egui_ctx, &mut app, entry(&out_nodes.nodes, "Close"));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert_eq!(words_box(&nodes).value(), Some(words.as_str()));
    assert!(has_label(&nodes, "Save…"), "Save… is offered again");
    assert!(!has_text(&nodes, "Saved to"), "no saved line");
}

// With a tool that gives no part (the trait's default), `README.txt` is
// the core part alone: nothing is appended for it, not even a newline.
#[test]
fn the_readme_is_the_core_part_alone_when_no_tool_has_a_part() {
    let (tool, _calls) = stub("t", "T", None);
    assert_eq!(super::readme(&[tool]), README_CORE);
}

// A run with no log installed (a first run has no data location, so none
// was chosen): the zip holds no `utw.log` then, and the journal's
// `missing` names it — the dialog lists `utw.log` among the contents.
#[test]
fn a_bundle_without_a_log_names_utw_log_under_missing() {
    let dir = TestDir::new("studio_core_report_no_log");
    let out = dir.path().join("utw-report.zip");
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();
    let out_path = out.clone();
    app.report.save_dialog = Box::new(move |_| Some(out_path.clone()));

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "Save…"));
    assert!(out.exists(), "the zip was written");

    let journal =
        crate::repro::tests::journal_doc(&crate::repro::tests::zip_text(&out, "journal.toml"));
    let missing = journal
        .get("missing")
        .and_then(toml::Value::as_array)
        .map(|list| {
            list.iter()
                .map(|name| name.as_str().unwrap())
                .collect::<Vec<_>>()
        });
    assert_eq!(
        missing.as_deref(),
        Some(&["utw.log"][..]),
        "the log the dialog lists is named under `missing`: {journal}"
    );
}

// ST-REP-03
#[test]
fn a_cancelled_or_failed_save_writes_nothing_and_keeps_save() {
    let dir = TestDir::new("studio_core_report_failed");
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));

    // The picker cancelled: nothing is written and the dialog is as it was.
    app.report.save_dialog = Box::new(|_| None);
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "Save…"));
    let out = frame(&egui_ctx, &mut app, Vec::new(), true);
    assert_dialog_open(&out.nodes);
    assert!(app.report.saved_to.is_none(), "no write marked");
    assert!(!has_text(&out.nodes, "Saved to"), "no saved line");

    // A path that cannot be written: the error shows under the buttons,
    // Save… stays, and no file lands at the path.
    let missing = dir.path().join("missing/report.zip");
    let out_path = missing.clone();
    app.report.save_dialog = Box::new(move |_| Some(out_path.clone()));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "Save…"));
    assert!(!missing.exists(), "nothing was written at the path");
    let out = frame(&egui_ctx, &mut app, Vec::new(), true);
    assert!(has_label(&out.nodes, "Save…"), "Save… is kept");
    assert!(
        has_text(&out.nodes, "cannot write the bundle"),
        "the write's error shows"
    );

    // Closed with the error standing and opened again: the dialog is fresh.
    click(&egui_ctx, &mut app, entry(&out.nodes, "Cancel"));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert!(has_label(&nodes, "Save…"), "premise: the dialog is open");
    assert!(
        !has_text(&nodes, "cannot write the bundle"),
        "the reopened dialog shows no earlier error"
    );

    // The failed write again, so the retry below has an error to clear.
    click(&egui_ctx, &mut app, entry(&nodes, "Save…"));
    let out = frame(&egui_ctx, &mut app, Vec::new(), true);
    assert!(
        has_text(&out.nodes, "cannot write the bundle"),
        "premise: the error stands again"
    );

    // A retry that writes clears the error.
    let written = dir.path().join("report.zip");
    let out_path = written.clone();
    app.report.save_dialog = Box::new(move |_| Some(out_path.clone()));
    click(&egui_ctx, &mut app, entry(&out.nodes, "Save…"));
    assert!(written.exists(), "premise: the retry wrote the zip");
    let out = frame(&egui_ctx, &mut app, Vec::new(), true);
    assert!(
        !has_text(&out.nodes, "cannot write the bundle"),
        "the earlier error is gone"
    );
}

// Cancel and Escape each close the dialog without a write, and the words
// stay for the next open.
#[test]
fn cancel_and_escape_close_the_dialog_and_keep_the_words() {
    let escape = egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    };
    for (how, closes) in [("Cancel", None), ("Escape", Some(escape))] {
        let Shell { mut app, _dir, .. } = shell();
        let egui_ctx = test_context();
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(&egui_ctx, &mut app, words_box(&nodes));
        frame(
            &egui_ctx,
            &mut app,
            vec![egui::Event::Text("it broke".to_owned())],
            true,
        );
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        match closes {
            None => click(&egui_ctx, &mut app, entry(&nodes, how)),
            Some(key) => frame(&egui_ctx, &mut app, vec![key], true),
        };
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(!has_label(&nodes, "Save…"), "{how} closed the dialog");

        click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert_eq!(
            words_box(&nodes).value(),
            Some("it broke"),
            "the words stay after {how}"
        );
    }
}

// The tools' lines follow the sidebar's order, not the order they first
// recorded in: B records before A, and A is listed first.
#[test]
fn the_tools_lines_follow_the_sidebar_order() {
    let dir = TestDir::new("studio_core_report_order");
    let Shell {
        mut app, ctx, _dir, ..
    } = shell();
    let egui_ctx = test_context();
    record(&ctx, "b", dir.path());
    record(&ctx, "a", dir.path());

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert!(
        line_bounds(&nodes, "A: 3 steps").y0 < line_bounds(&nodes, "B: 3 steps").y0,
        "A's line comes first"
    );
}

// Save… offers the save dialog the suggested name, `utw-report-<date>.zip`.
#[test]
fn save_offers_the_suggested_file_name() {
    let Shell { mut app, _dir, .. } = shell();
    let egui_ctx = test_context();
    let offered = Arc::new(std::sync::Mutex::new(String::new()));
    let seen = Arc::clone(&offered);
    app.report.save_dialog = Box::new(move |suggested| {
        *seen.lock().unwrap() = suggested.to_owned();
        None
    });
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "Save…"));
    let offered = offered.lock().unwrap();
    assert!(
        offered.starts_with("utw-report-") && offered.ends_with(".zip"),
        "the offered name: {offered:?}"
    );
}

// The lines are built when the dialog opens, not every frame: a step
// recorded while it is up shows at the next open.
#[test]
fn the_lines_are_built_at_open() {
    let dir = TestDir::new("studio_core_report_built_once");
    let Shell {
        mut app, ctx, _dir, ..
    } = shell();
    let egui_ctx = test_context();
    record(&ctx, "a", dir.path());
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    frame(&egui_ctx, &mut app, Vec::new(), true);

    ctx.recorder().step("a", "player 70101: Weight 75 → 80");
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert!(has_text(&nodes, "A: 3 steps"), "the lines as at open");

    click(&egui_ctx, &mut app, entry(&nodes, "Cancel"));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert!(has_text(&nodes, "A: 4 steps"), "the next open counts it");
}

// A long line (a renamed save) wraps inside the dialog's width, the bullet
// at its first line's top.
#[test]
fn a_long_line_wraps_inside_the_dialog() {
    let dir = TestDir::new("studio_core_report_wrap");
    let Shell {
        mut app, ctx, _dir, ..
    } = shell();
    let egui_ctx = test_context();
    record_named(&ctx, "a", dir.path(), "EDIT00000000 - VGL26 backup");
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;

    let line = line_bounds(&nodes, "A: 3 steps");
    let body = words_box(&nodes).bounds().expect("the box is drawn");
    assert!(
        (body.width() - f64::from(WIDTH)).abs() < 0.5,
        "premise: the box spans the body's {WIDTH} points: {body:?}"
    );
    assert!(
        line.x1 <= body.x1 + 0.5,
        "the line ends inside the body: {line:?} against {body:?}"
    );
    let row = line_bounds(&nodes, "The file holds:").height();
    assert!(line.height() > row * 1.5, "the line wraps: {line:?}");
    let bullet = nodes
        .iter()
        .filter(|node| node.value() == Some("•"))
        .filter_map(|node| node.bounds())
        .min_by(|a, b| a.y0.total_cmp(&b.y0))
        .expect("the bullets are drawn");
    assert!(
        (bullet.y0 - line.y0).abs() < 0.5,
        "the bullet sits at the line's top: {bullet:?} against {line:?}"
    );
}

// The draft's own row pitch: 15 between the tops of stacked text rows,
// "The file holds:" 19 above its first line (the `ul`'s 4-point margin
// over the 15), and the 84-point box right under its label.
#[test]
fn the_rows_and_the_box_stand_as_the_draft_spaces_them() {
    let dir = TestDir::new("studio_core_report_rows");
    let Shell {
        mut app, ctx, _dir, ..
    } = shell();
    let egui_ctx = test_context();
    record(&ctx, "a", dir.path());
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;

    let intro = line_bounds(&nodes, "Saves what UTW did");
    let second = line_bounds(&nodes, "The file is not sent");
    assert!(
        (second.y0 - intro.y0 - 15.0).abs() < 0.5,
        "the intro's second line stands 15 under the first: {intro:?} against {second:?}"
    );

    let heading = line_bounds(&nodes, "The file holds:");
    let rows = [
        line_bounds(&nodes, "A: 3 steps"),
        line_bounds(&nodes, "the files as they were opened"),
        line_bounds(&nodes, "utw.log, the version"),
    ];
    assert!(
        (rows[0].y0 - heading.y0 - 19.0).abs() < 0.5,
        "the first line stands 19 under the list's heading: {heading:?} against {:?}",
        rows[0]
    );
    for pair in rows.windows(2) {
        assert!(
            (pair[1].y0 - pair[0].y0 - 15.0).abs() < 0.5,
            "a list row stands 15 under the previous one: {:?} against {:?}",
            pair[0],
            pair[1]
        );
    }

    let what = line_bounds(&nodes, "What happened?");
    let body = words_box(&nodes).bounds().expect("the box is drawn");
    assert!(
        (body.height() - 84.0).abs() < 0.5,
        "the box is the draft's 84 points high: {body:?}"
    );
    assert!(
        (body.y0 - what.y1).abs() < 0.5,
        "the box stands right under its label: {what:?} against {body:?}"
    );
}

/// The "Include earlier sessions" box's node.
fn earlier_box(nodes: &[accesskit::Node]) -> &accesskit::Node {
    nodes
        .iter()
        .find(|node| {
            node.role() == Role::CheckBox && node.label() == Some("Include earlier sessions")
        })
        .expect("the box is drawn")
}

/// A step line for `session_folder`.
fn step(what: &str) -> SessionMessage {
    SessionMessage::Step {
        seconds: 0.0,
        tool: "stub".to_owned(),
        what: what.to_owned(),
        op: None,
    }
}

/// A kept input for `session_folder`: `path` as read, `bytes` its content.
fn kept_input(path: &str, bytes: &'static [u8]) -> SessionMessage {
    SessionMessage::Input {
        tool: "stub".to_owned(),
        path: Path::new(path).to_path_buf(),
        file: SessionFile::Raw(Arc::from(bytes)),
    }
}

/// A kept output for `session_folder`.
fn kept_output(path: &str, bytes: &'static [u8]) -> SessionMessage {
    SessionMessage::Output {
        tool: "stub".to_owned(),
        path: Path::new(path).to_path_buf(),
        file: SessionFile::Raw(Arc::from(bytes)),
    }
}

/// A session folder under `root`, written by `SessionWriter`: `pid`'s
/// header at 09:`minute`:`second` UTC on 2026-11-21, `messages` in order,
/// then `end`'s `[end]` when `Some`. Returns the folder's name.
fn session_folder(
    root: &Path,
    pid: u32,
    minute: i8,
    second: i8,
    messages: Vec<SessionMessage>,
    end: Option<SessionEnd>,
) -> String {
    let header = SessionHeader {
        version: "v-test".to_owned(),
        os: "test-os".to_owned(),
        started: jiff::civil::date(2026, 11, 21)
            .at(9, minute, second, 0)
            .to_zoned(jiff::tz::TimeZone::UTC)
            .unwrap(),
        pid,
    };
    let name = session_folder_name(&header);
    let mut writer = SessionWriter::new(root, header);
    for message in &messages {
        writer.write(message).unwrap();
    }
    if let Some(end) = end {
        writer.end(end).unwrap();
    }
    name
}

// The box lists each kept session but the window's own, unticked, its line
// the draft's. Unticked, the saved zip holds no `sessions/` entry; ticked,
// each listed session lands in it as a bundle of its own, which
// `read_bundle` reads. A journal gone between the opening and the save is
// named under `missing`, and no earlier folder means no box.
// ST-REP-09
#[test]
fn the_box_lists_the_kept_sessions_and_ticked_packs_each() {
    let dir = TestDir::new("studio_core_report_sessions");
    let Shell {
        mut app, ctx, _dir, ..
    } = shell();
    let egui_ctx = test_context();
    let sessions_root = ctx.paths().data_dir.clone().unwrap().join("sessions");
    // Two earlier sessions, and the window's own.
    let name_a = session_folder(
        &sessions_root,
        4242,
        30,
        15,
        vec![
            step("the closed session's step"),
            kept_input("C:/Data/in/EDIT00000000", b"INPUT_A"),
            kept_output("C:/Data/out/EDIT00000000", b"OUT_A"),
        ],
        Some(SessionEnd::Closed),
    );
    let name_b = session_folder(
        &sessions_root,
        4243,
        31,
        15,
        vec![
            step("the crashed session's step"),
            kept_input("C:/Data/in/EDIT00000001", b"INPUT_B"),
        ],
        Some(SessionEnd::Crashed),
    );
    let own = session_folder(
        &sessions_root,
        4244,
        32,
        15,
        vec![step("the window's own step")],
        Some(SessionEnd::Closed),
    );
    app = app.with_session_folder(Some(own.clone()));

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;

    // The unticked box and a line per earlier session, not the window's own.
    assert_eq!(
        earlier_box(&nodes).toggled(),
        Some(accesskit::Toggled::False),
        "the box stands unticked"
    );
    let wrote = fs::metadata(sessions_root.join(&name_a).join("outputs/0-EDIT00000000"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(
        text_line(&nodes, "21 Nov 09:30"),
        format!(
            "21 Nov 09:30: opened EDIT00000000; wrote at {}; closed",
            clock_text(local_clock(wrote))
        )
    );
    assert_eq!(
        text_line(&nodes, "21 Nov 09:31"),
        "21 Nov 09:31: opened EDIT00000001; wrote nothing; crashed"
    );
    assert!(
        !has_text(&nodes, "21 Nov 09:32"),
        "the window's own is not listed"
    );

    // Saved unticked: the zip holds no `sessions/` entry.
    let out = dir.path().join("unticked.zip");
    let out_path = out.clone();
    app.report.save_dialog = Box::new(move |_| Some(out_path.clone()));
    click(&egui_ctx, &mut app, entry(&nodes, "Save…"));
    assert!(out.exists(), "the zip was written");
    let names = crate::repro::tests::zip_names(&out);
    assert!(
        !names.iter().any(|name| name.starts_with("sessions/")),
        "unticked holds no sessions/ entry: {names:?}"
    );

    // Closed and opened again: the box is unticked at every opening.
    // Ticked this time: `sessions/<name>.zip` per listed session — each a
    // bundle `read_bundle` reads — and not the window's own.
    let out_nodes = frame(&egui_ctx, &mut app, Vec::new(), true);
    click(&egui_ctx, &mut app, entry(&out_nodes.nodes, "Close"));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert_eq!(
        earlier_box(&nodes).toggled(),
        Some(accesskit::Toggled::False),
        "unticked at every opening"
    );
    click(&egui_ctx, &mut app, earlier_box(&nodes));
    let out = dir.path().join("ticked.zip");
    let out_path = out.clone();
    app.report.save_dialog = Box::new(move |_| Some(out_path.clone()));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "Save…"));
    assert!(out.exists(), "the zip was written");
    let names = crate::repro::tests::zip_names(&out);
    for name in [&name_a, &name_b] {
        let entry = format!("sessions/{name}.zip");
        assert!(names.contains(&entry), "{entry} is missing: {names:?}");
    }
    let own_entry = format!("sessions/{own}.zip");
    assert!(
        !names.contains(&own_entry),
        "the window's own is not packed: {names:?}"
    );
    for (name, step_line) in [
        (&name_a, "the closed session's step"),
        (&name_b, "the crashed session's step"),
    ] {
        let inner = dir.path().join(format!("inner-{name}.zip"));
        fs::write(
            &inner,
            crate::repro::tests::zip_bytes(&out, &format!("sessions/{name}.zip")),
        )
        .unwrap();
        let bundle =
            crate::repro::read_bundle(&inner, &|_| panic!("no packed rows in these folders"))
                .unwrap();
        assert!(
            bundle.steps.iter().any(|step| step.what == step_line),
            "the inner bundle holds the session's steps"
        );
    }

    // A journal gone between the opening and the save: the session is
    // named under `missing`, and the other still packs.
    let out_nodes = frame(&egui_ctx, &mut app, Vec::new(), true);
    click(&egui_ctx, &mut app, entry(&out_nodes.nodes, "Close"));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    fs::remove_file(sessions_root.join(&name_b).join("journal.toml")).unwrap();
    click(&egui_ctx, &mut app, earlier_box(&nodes));
    let out = dir.path().join("missing.zip");
    let out_path = out.clone();
    app.report.save_dialog = Box::new(move |_| Some(out_path.clone()));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "Save…"));
    assert!(out.exists(), "the zip was written");
    let names = crate::repro::tests::zip_names(&out);
    let entry_a = format!("sessions/{name_a}.zip");
    assert!(
        names.contains(&entry_a),
        "the other session still packs: {names:?}"
    );
    let journal =
        crate::repro::tests::journal_doc(&crate::repro::tests::zip_text(&out, "journal.toml"));
    let missing = journal
        .get("missing")
        .and_then(toml::Value::as_array)
        .map(|list| {
            list.iter()
                .map(|name| name.as_str().unwrap())
                .collect::<Vec<_>>()
        });
    let expected = format!("sessions/{name_b}.zip");
    assert!(
        missing
            .as_ref()
            .is_some_and(|list| list.contains(&expected.as_str())),
        "the lost session is named under missing: {missing:?}"
    );

    // No earlier folder: no box, no session lines.
    let Shell { mut app, _dir, .. } = shell();
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert!(
        !nodes.iter().any(|node| node.role() == Role::CheckBox
            && node.label() == Some("Include earlier sessions")),
        "no box without an earlier session"
    );
}

// The draft's sessions block: the box 8 under the "The file holds:" list,
// its first line 4 under the box, its lines 15 apart in the weak colour
// and the privacy line 8 under the last one.
#[test]
fn the_box_and_its_lines_stand_as_the_draft_spaces_them() {
    let Shell {
        mut app, ctx, _dir, ..
    } = shell();
    let egui_ctx = test_context();
    let sessions_root = ctx.paths().data_dir.clone().unwrap().join("sessions");
    session_folder(&sessions_root, 4242, 30, 15, vec![step("a step")], None);
    session_folder(
        &sessions_root,
        4243,
        31,
        15,
        vec![step("a step")],
        Some(SessionEnd::Ended),
    );

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;

    let last = line_bounds(&nodes, "utw.log, the version");
    let box_row = earlier_box(&nodes).bounds().expect("the box is drawn");
    assert!(
        (box_row.y0 - last.y1 - 8.0).abs() < 0.5,
        "the box stands 8 under the list: {last:?} against {box_row:?}"
    );
    let first = line_bounds(&nodes, "21 Nov 09:31");
    let second = line_bounds(&nodes, "21 Nov 09:30");
    assert!(
        (first.y0 - box_row.y1 - 4.0).abs() < 0.5,
        "the first line stands 4 under the box: {box_row:?} against {first:?}"
    );
    assert!(
        (second.y0 - first.y0 - 15.0).abs() < 0.5,
        "a session line stands 15 under the previous one: {first:?} against {second:?}"
    );
    // The draft's two endings these sessions write, each line's text whole.
    assert_eq!(
        text_line(&nodes, "21 Nov 09:31"),
        "21 Nov 09:31: opened nothing; wrote nothing; stopped without closing"
    );
    assert_eq!(
        text_line(&nodes, "21 Nov 09:30"),
        "21 Nov 09:30: opened nothing; wrote nothing; ending unknown"
    );
    let privacy = line_bounds(&nodes, "Your user folder");
    assert!(
        (privacy.y0 - second.y1 - 8.0).abs() < 0.5,
        "the privacy line stands 8 under the last session line: {second:?} against {privacy:?}"
    );
}

/// The bounds of the text node whose value starts with `start`.
fn line_bounds(nodes: &[accesskit::Node], start: &str) -> accesskit::Rect {
    nodes
        .iter()
        .find(|node| node.value().is_some_and(|value| value.starts_with(start)))
        .and_then(|node| node.bounds())
        .unwrap_or_else(|| panic!("no drawn line starts {start:?}"))
}

/// A summary of `steps` steps, `inputs` kept inputs the first of which is
/// `first` (its file name and size) and the last output `wrote` at a time.
fn summary(
    steps: usize,
    inputs: usize,
    first: Option<(&str, usize)>,
    wrote: Option<(&str, SystemTime)>,
) -> ToolSummary {
    ToolSummary {
        tool: "save-editor",
        steps,
        inputs,
        first_input: first.map(|(name, size)| (Path::new("saves").join(name), size)),
        last_output: wrote.map(|(name, when)| (Path::new("out").join(name), when)),
    }
}

// A tool's line, whole: one step reads "1 step", a file under 0.1 MB is
// sized in KB (a rules file), and each clause is left out when it has
// nothing to name.
#[test]
fn a_tools_line_reads_whole() {
    let when = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_794_000_000);
    let clock = clock_text(local_clock(when));
    for (summary, expected) in [
        (summary(1, 0, None, None), "1 step".to_owned()),
        (
            summary(0, 1, Some(("EDIT00000000", 7_759_462)), None),
            "0 steps; opened EDIT00000000 (7.4 MB)".to_owned(),
        ),
        (
            summary(3, 1, Some(("EDIT00000000", 7_759_462)), None),
            "3 steps; opened EDIT00000000 (7.4 MB)".to_owned(),
        ),
        (
            summary(3, 3, Some(("EDIT00000000", 7_759_462)), None),
            "3 steps; opened EDIT00000000 (7.4 MB), and 2 more".to_owned(),
        ),
        (
            summary(2, 1, Some(("vgl26.4ccrules", 25_970)), None),
            "2 steps; opened vgl26.4ccrules (25 KB)".to_owned(),
        ),
        (
            summary(2, 1, Some(("vgl26.4ccrules", 104_857)), None),
            "2 steps; opened vgl26.4ccrules (102 KB)".to_owned(),
        ),
        (
            summary(2, 1, Some(("vgl26.4ccrules", 104_858)), None),
            "2 steps; opened vgl26.4ccrules (0.1 MB)".to_owned(),
        ),
        (
            summary(
                4,
                1,
                Some(("EDIT00000000", 7_759_462)),
                Some(("EDIT00000000", when)),
            ),
            format!("4 steps; opened EDIT00000000 (7.4 MB); wrote EDIT00000000 at {clock}"),
        ),
    ] {
        assert_eq!(summary_text(&summary), expected);
    }
}

// `installs.txt` writes each version's own folder and what its exe check
// found there, every outcome on its own line.
#[test]
fn installs_names_each_versions_folder_and_check() {
    let dir = TestDir::new("studio_core_report_installs");
    let games = dir.path().join("games");
    for (folder, exe) in [
        ("PES 2015", Some("PES2015.exe")),
        ("PES 2017", Some("PES2017.exe")),
        ("empty", None),
    ] {
        fs::create_dir_all(games.join(folder)).unwrap();
        if let Some(exe) = exe {
            fs::write(games.join(folder).join(exe), b"").unwrap();
        }
    }
    let Shell { ctx, _dir, .. } = shell();
    {
        let mut settings = ctx.settings().lock().unwrap();
        settings.common.pes_folder_path = String::new();
        for (version, folder) in [
            (PesVersion::Pes15, "PES 2015"),
            (PesVersion::Pes16, "PES 2017"),
            (PesVersion::Pes17, "empty"),
            (PesVersion::Pes18, "missing"),
        ] {
            settings
                .common
                .pes_folders
                .insert(version, games.join(folder).display().to_string());
        }
    }
    let folder = |name: &str| games.join(name).display().to_string();
    assert_eq!(
        installs(&ctx),
        [
            format!("PES 2015: {} (exe found)", folder("PES 2015")),
            format!("PES 2016: {} (PES 2017's exe)", folder("PES 2017")),
            format!("PES 2017: {} (no PES exe)", folder("empty")),
            format!("PES 2018: {} (folder not found)", folder("missing")),
            "PES 2019: no folder set".to_owned(),
            "PES 2020: no folder set".to_owned(),
            "PES 2021: no folder set".to_owned(),
        ]
        .join("\n")
    );
}

#[test]
fn the_clock_text_is_hh_mm() {
    assert_eq!(
        clock_text(jiff::civil::time(2, 41, 0, 0)),
        "02:41",
        "24-hour, zero-padded"
    );
}

#[test]
fn the_suggested_name_sorts_by_the_local_time() {
    assert_eq!(
        bundle_name(&jiff::civil::date(2026, 11, 7).at(2, 45, 0, 0)),
        "utw-report-2026-11-07-0245.zip"
    );
}
