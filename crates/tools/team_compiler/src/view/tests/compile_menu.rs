//! The Compile button's split: its arrow's menu of output modes, a pick relabelling
//! the main part for the session, Open output folder and Copy report (TC-GUI-11).

use super::*;
use crate::cli::Mode;
use crate::view::Intent;

/// The Compile button's arrow, labelled by the arrow alone (the mode entries and the
/// menu buttons carry their text first).
const ARROW: &str = "\u{23f7}";

/// The bare view's frame over `log` and `detail`, for `click_on`: one frame with the
/// events it is given, its nodes returned.
fn view_frame<'a>(
    ctx: &'a ToolContext,
    log: &'a RunLog,
    panel: &'a mut LogPanel,
    detail: &'a mut GridDetail,
) -> impl FnMut(&mut Headless, Vec<egui::Event>) -> Vec<accesskit::Node> + 'a {
    move |window, events| {
        window
            .frame(events, |ui| show(ui, ctx, log, panel, &[], false, detail))
            .1
    }
}

// TC-GUI-11
#[test]
fn a_mode_pick_relabels_the_main_part_and_its_click_runs_that_mode() {
    let ctx = settings_only_context();
    let log = RunLog::default();
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let mut window = Headless::new();
    let (_, nodes) = window.frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    let nodes = click_on(
        &mut window,
        button(&nodes, ARROW),
        view_frame(&ctx, &log, &mut panel, &mut detail),
    );
    // The menu is open over the toolbar: the modes, then the two actions.
    for label in [
        "Compile",
        "Compile for sideloading",
        "Test output",
        "Open output folder",
        "Copy report",
    ] {
        button(&nodes, label);
    }
    let nodes = click_on(
        &mut window,
        button(&nodes, "Test output"),
        view_frame(&ctx, &log, &mut panel, &mut detail),
    );
    assert!(
        !labeled(&nodes, "Open output folder"),
        "the pick closed the menu"
    );
    // The main part relabelled: the pick sticks for the session.
    let at = centre(button(&nodes, "Test output"));
    let (clicked, _) = window.frame(press(at), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    assert!(clicked.is_none(), "not before the release");
    let (clicked, nodes) = window.frame(release(at), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    assert!(
        matches!(clicked, Some(Intent::Compile(Mode::Test))),
        "the main part runs the picked mode"
    );

    // A pick back: while the main part reads "Test output", "Compile" is the menu's
    // entry alone, so this click lands on it.
    let nodes = click_on(
        &mut window,
        button(&nodes, ARROW),
        view_frame(&ctx, &log, &mut panel, &mut detail),
    );
    let nodes = click_on(
        &mut window,
        button(&nodes, "Compile"),
        view_frame(&ctx, &log, &mut panel, &mut detail),
    );
    assert!(
        !labeled(&nodes, "Open output folder"),
        "the pick closed the menu"
    );
    assert!(
        !labeled(&nodes, "Test output"),
        "the main part reads Compile again"
    );
    let at = centre(button(&nodes, "Compile"));
    let (clicked, _) = window.frame(press(at), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    assert!(clicked.is_none(), "not before the release");
    let (clicked, _) = window.frame(release(at), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    assert!(
        matches!(clicked, Some(Intent::Compile(Mode::Normal))),
        "the main part runs Normal again"
    );
}

// TC-GUI-11
#[test]
fn a_picked_mode_reaches_the_run_through_the_tool() {
    let temp = sandbox("view_compile_menu_test_mode");
    let root = temp.path();
    install_pes(root);
    let ctx = tool_context(root, "");
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    let nodes = click_labeled(&mut window, &nodes, ARROW, &mut tool, &ctx);
    let nodes = click_labeled(&mut window, &nodes, "Test output", &mut tool, &ctx);
    click_labeled(&mut window, &nodes, "Test output", &mut tool, &ctx);
    assert!(tool.run_log.is_running());
    poll_until_done(|| {
        tool.tick(&ctx);
        tool.run_log.is_running()
    });
    assert!(
        root.join("output/test_output").is_dir(),
        "the run wrote the test mode's loose files"
    );
    assert_eq!(
        tool.run_log.texts().last().copied(),
        Some("Run finished with exit code 0")
    );
}

// TC-GUI-11
#[test]
fn sideloading_is_off_on_pes_2016_and_on_on_later_ones() {
    for (version, disabled) in [(PesVersion::Pes16, true), (PesVersion::Pes21, false)] {
        let temp = sandbox("view_compile_menu_sideload_version");
        let ctx = tool_context_at(temp.path(), version);
        let mut tool = Tool::new();
        let mut window = Headless::new();
        let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
        let nodes = click_labeled(&mut window, &nodes, ARROW, &mut tool, &ctx);
        assert_eq!(
            button(&nodes, "Compile for sideloading").is_disabled(),
            disabled,
            "{version}"
        );
    }
}

// TC-GUI-33's clause: an outdated list stops the mode that installs the CPKs alone.
#[test]
fn an_outdated_list_blocks_the_normal_mode_alone() {
    let temp = sandbox("view_compile_menu_outdated");
    let root = temp.path();
    install(root, 2017, &pes17_list(), None);
    let ctx = tool_context_at(root, PesVersion::Pes17);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = first_check(&mut tool, &ctx, &mut window);
    assert!(
        button(&nodes, "Compile").is_disabled(),
        "premise: the outdated list disabled Compile"
    );
    let nodes = click_labeled(&mut window, &nodes, ARROW, &mut tool, &ctx);
    let nodes = click_labeled(
        &mut window,
        &nodes,
        "Compile for sideloading",
        &mut tool,
        &ctx,
    );
    assert!(
        !labeled(&nodes, "Open output folder"),
        "the pick closed the menu"
    );
    assert!(
        !button(&nodes, "Compile for sideloading").is_disabled(),
        "the sideload mode installs no CPK, so it stays enabled"
    );
}

#[test]
fn the_arrow_is_disabled_while_a_run_is_in_flight() {
    let ctx = settings_only_context();
    let (release_run, until) = unbounded();
    let mut log = RunLog::in_flight(until);
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let (_, nodes) = Headless::new().frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    assert!(button(&nodes, ARROW).is_disabled());
    release_run.send(()).unwrap();
    poll_until_done(|| {
        log.poll();
        log.is_running()
    });
    assert_eq!(log.texts(), ["Run finished with exit code 0"]);
}

// TC-GUI-11
#[test]
fn copy_report_puts_every_line_on_the_clipboard_whatever_the_filter() {
    let ctx = settings_only_context();
    let mut log = RunLog::default();
    log.set_check(&[], &[finding("export_identified", Severity::Info)]);
    log.append_test_line(Severity::Error, "the run's error line");
    let mut panel = LogPanel::new(TOOL_ID);
    // Errors alone are drawn under this filter; the report is the whole log anyway.
    panel.show_only(studio_core::LogFilter::Errors);
    let mut detail = GridDetail::new();
    let mut window = Headless::new();
    let (_, nodes) = window.frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    let nodes = click_on(
        &mut window,
        button(&nodes, ARROW),
        view_frame(&ctx, &log, &mut panel, &mut detail),
    );
    click_on(
        &mut window,
        button(&nodes, "Copy report"),
        view_frame(&ctx, &log, &mut panel, &mut detail),
    );
    assert_eq!(
        window.copied.as_deref(),
        Some("- export 1: Info export_identified\nthe run's error line")
    );
}

// TC-GUI-11
#[test]
fn an_output_folder_that_cannot_be_created_is_a_log_line() {
    let temp = sandbox("view_compile_menu_output_blocked");
    let root = temp.path();
    fs::write(root.join("blocker"), b"").unwrap();
    let folder = root.join("blocker/out");
    let ctx = tool_context(
        root,
        &format!("[compiler]\noutput_folder_path = '{}'\n", folder.display()),
    );
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    let nodes = click_labeled(&mut window, &nodes, ARROW, &mut tool, &ctx);
    click_labeled(&mut window, &nodes, "Open output folder", &mut tool, &ctx);
    let last = tool.run_log.texts().last().copied().unwrap_or("");
    assert!(
        last.starts_with("error: cannot create the output folder ")
            && last.contains(&folder.display().to_string()),
        "{last}"
    );
    assert!(root.join("blocker").is_file());
}

#[test]
fn the_output_folder_is_resolved_and_created() {
    let temp = sandbox("view_compile_menu_output_folder");
    let root = temp.path();
    assert!(!root.join("output").exists(), "premise: no output/ yet");
    let ctx = tool_context(root, "");
    assert_eq!(
        crate::create_output_folder(&ctx).unwrap(),
        root.join("output")
    );
    assert!(root.join("output").is_dir(), "the folder was created");

    // An absolute `output_folder_path` stands alone.
    let elsewhere = root.join("elsewhere/custom");
    let ctx = tool_context(
        root,
        &format!(
            "[compiler]\noutput_folder_path = '{}'\n",
            elsewhere.display()
        ),
    );
    assert_eq!(crate::create_output_folder(&ctx).unwrap(), elsewhere);
    assert!(elsewhere.is_dir());
}
