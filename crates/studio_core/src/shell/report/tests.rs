//! The "Report a problem" dialog's tests: the tail entry opening it (rail and
//! overlay), the "The file holds:" list, the Save… write and its answers, and
//! the two formatters (the `hh:mm` of an output's write and the suggested file
//! name).

use std::fs;
use std::path::Path;
use std::sync::Arc;

use egui::accesskit::{self, Role};

use super::*;
use crate::shell::sidebar::REPORT_LABEL;
use crate::shell::test_harness::*;
use crate::tool::ToolContext;

/// Records `tool`'s three steps, one kept input and one kept output through
/// `ctx`'s shared journal, as the stub tools' session work would.
fn record(ctx: &ToolContext, tool: &'static str, dir: &Path) {
    let input = dir.join("EDIT00000000");
    let output = dir.join("out/EDIT00000000");
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
            &format!(
                "Saves what {} did since it opened, so the problem can be reproduced.",
                crate::EXE_NAME
            )
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
    words_box(nodes);
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
    assert!(
        has_text(nodes, "Your username is rewritten as ~."),
        "the username line"
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
    let mut app = shell().app;
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

// ST-REP-01's "more than one file" clause: a second input appends ", and 2
// more" to the first's name; the clause names the first input alone.
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
    // The window's log and the settings file the `files` list carries.
    let log = dir.path().join("utw.log");
    fs::write(&log, "the window's log lines\n").unwrap();
    app = app.with_log_file(Some(log));
    // The shell's first frame writes the settings file itself; the bundle's
    // entry carries its bytes as they are then.
    let settings_file = ctx
        .paths()
        .data_dir
        .as_ref()
        .unwrap()
        .join(SETTINGS_FILE_NAME);
    app.report.home = Some(home.display().to_string());
    let words = format!("it broke in {}", home.display());

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
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
    assert!(
        has_text(&out_nodes.nodes, "Saved to ~"),
        "the saved line writes the home as ~"
    );
    assert!(
        has_label(&out_nodes.nodes, "Close"),
        "only Close is offered"
    );
    assert!(
        !has_label(&out_nodes.nodes, "Save…"),
        "no Save… once written"
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
        zip(&out, "settings.toml"),
        fs::read_to_string(&settings_file).unwrap(),
        "the settings file as the shell wrote it"
    );
    let installs = zip(&out, "state/installs.txt");
    assert!(
        installs.lines().count() == 7
            && installs.contains("PES 2021:")
            && installs.contains("(no PES exe)"),
        "installs.txt lists every version the selector offers: {installs}"
    );
    assert_eq!(
        zip(&out, "words.txt"),
        "it broke in ~",
        "the words write the home as ~"
    );

    // Closed and opened again: the words are still there.
    click(&egui_ctx, &mut app, entry(&out_nodes.nodes, "Close"));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, REPORT_LABEL));
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert_eq!(words_box(&nodes).value(), Some(words.as_str()));
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
