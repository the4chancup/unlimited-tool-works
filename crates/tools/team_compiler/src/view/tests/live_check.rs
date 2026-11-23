//! The grid check's lifecycle: when the window starts one, what holds its rows
//! back, what a refused check leaves behind, and a settings change's re-check.

use super::*;

/// `root`'s PES folder switched to `other`'s through the shared settings, as the sidebar's
/// pick writes it, then the check `tick` starts for the change run to its end.
fn switch_install(
    settings: &Arc<Mutex<Settings>>,
    other: &Path,
    tool: &mut Tool,
    ctx: &ToolContext,
) {
    set_pes_folder(settings, &other.join("PES"));
    tool.tick(ctx);
    assert!(
        tool.live_check.is_running(),
        "the settings change started a check"
    );
    finish_check(tool, ctx);
}

// The outdated list's button and disabled Compile describe the install the check ran on:
// a switch to an install whose list is the official one checks the grid again, and they
// are gone once that check ends.
#[test]
fn switching_to_an_official_install_checks_the_grid_again() {
    let templates = crate::templates::Templates::embedded();
    let official = templates.official_list_file();
    let (_temp, other, ctx, settings) =
        two_installs("view_check_to_official", &pes17_list(), official);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = first_check(&mut tool, &ctx, &mut window);
    assert!(labeled(&nodes, "Upgrade DpFileList"), "premise: the button");
    assert!(
        button(&nodes, "Compile").is_disabled(),
        "premise: Compile disabled"
    );

    switch_install(&settings, other.path(), &mut tool, &ctx);

    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(!labeled(&nodes, "Upgrade DpFileList"), "the button is gone");
    assert!(
        !button(&nodes, "Compile").is_disabled(),
        "Compile is enabled"
    );
}

// The other way: a switch to an install whose list is the outdated one checks the grid
// again, and the button is up with Compile disabled once that check ends.
#[test]
fn switching_to_an_outdated_install_checks_the_grid_again() {
    let templates = crate::templates::Templates::embedded();
    let official = templates.official_list_file();
    let (_temp, other, ctx, settings) =
        two_installs("view_check_to_outdated", official, &pes17_list());
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = first_check(&mut tool, &ctx, &mut window);
    assert!(
        !labeled(&nodes, "Upgrade DpFileList") && !button(&nodes, "Compile").is_disabled(),
        "premise: no button, Compile enabled"
    );

    switch_install(&settings, other.path(), &mut tool, &ctx);

    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(labeled(&nodes, "Upgrade DpFileList"), "the button is back");
    assert!(
        button(&nodes, "Compile").is_disabled(),
        "Compile is disabled"
    );
}

// The shell writes `last_tool` at every tool switch, and the window's own settings
// (the theme, the update check, the window's place) change without the install: none of
// them checks the grid again.
#[test]
fn a_tool_switch_or_a_theme_change_starts_no_check() {
    let temp = sandbox("view_check_tool_switch");
    let (ctx, settings) = tool_context_with_settings(temp.path(), PesVersion::Pes17);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    first_check(&mut tool, &ctx, &mut window);

    settings.lock().unwrap().common.last_tool = Some("save-editor".to_owned());
    tool.tick(&ctx);
    assert!(
        !tool.live_check.is_running(),
        "a tool switch starts no check"
    );
    settings.lock().unwrap().common.theme = studio_core::Theme::Light;
    tool.tick(&ctx);
    assert!(
        !tool.live_check.is_running(),
        "a theme change starts no check"
    );
}

// The exports folder is a setting the check reads: pointing it elsewhere checks the
// grid again (the new folder's exports, or its refusal in the log).
#[test]
fn an_exports_folder_change_checks_the_grid_again() {
    let temp = sandbox("view_check_exports_folder");
    let (ctx, settings) = tool_context_with_settings(temp.path(), PesVersion::Pes17);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    first_check(&mut tool, &ctx, &mut window);

    settings.lock().unwrap().common.exports_folder_path = PathBuf::from("elsewhere");
    tool.tick(&ctx);
    // The check it starts is refused at once (no such folder) and `tick` polls it in the
    // same call, so it may be over already: the settings a started check keeps are read.
    assert_eq!(
        tool.checked_settings
            .as_ref()
            .map(|(common, _)| common.exports_folder_path.clone()),
        Some(PathBuf::from("elsewhere")),
        "an exports folder change started a check"
    );
    finish_check(&mut tool, &ctx);
}

#[test]
fn a_check_s_rows_reach_the_grid_only_once_the_compile_has_ended() {
    let temp = sandbox("view_grid_compile_guard");
    let ctx = tool_context(temp.path(), "");
    let mut tool = Tool::new();
    let (release_check, check_until) = unbounded();
    tool.live_check = LiveCheck::in_flight(check_until, vec![label_only_row()]);
    let (release_run, run_until) = unbounded();
    tool.run_log = RunLog::in_flight(run_until);
    release_check.send(()).unwrap();
    poll_until_done(|| !tool.live_check.has_ended());
    tool.tick(&ctx);
    assert!(
        tool.live_check.rows().is_empty(),
        "held while the Compile runs"
    );

    release_run.send(()).unwrap();
    poll_until_done(|| {
        tool.tick(&ctx);
        tool.run_log.is_running()
    });
    // The ended check's outcome is dropped: it predates the check the run's end
    // asks for, which delivers its own rows.
    assert!(
        tool.live_check.rows().is_empty(),
        "the stale check's rows never reach the grid"
    );
    assert!(tool.live_check.is_running(), "the run's end starts a check");
    finish_check(&mut tool, &ctx);
    assert_eq!(tool.live_check.rows()[0].label.text, "/egg/");
}

#[test]
fn a_compile_s_end_starts_a_check() {
    let temp = sandbox("view_grid_compile_end");
    let ctx = tool_context(temp.path(), "");
    let mut tool = Tool::new();
    let (release_run, run_until) = unbounded();
    tool.run_log = RunLog::in_flight(run_until);
    tool.tick(&ctx);
    assert!(!tool.live_check.is_running());
    release_run.send(()).unwrap();
    poll_until_done(|| {
        tool.tick(&ctx);
        tool.run_log.is_running()
    });
    assert!(tool.live_check.is_running());
    finish_check(&mut tool, &ctx);
    assert_eq!(
        row(tool.live_check.rows(), "/egg/").id,
        Some(IdCell::Known("792".to_owned()))
    );
}

// The window starts no check of its own while a Compile run is in flight, nor once a
// check has started.
#[test]
fn the_window_starts_no_check_under_a_run_or_once_one_has_started() {
    let temp = sandbox("view_grid_check_once");
    let ctx = tool_context(temp.path(), "");
    let mut tool = Tool::new();
    let (release_run, run_until) = unbounded();
    tool.run_log = RunLog::in_flight(run_until);
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(!tool.live_check.is_running(), "none under the run");

    release_run.send(()).unwrap();
    poll_until_done(|| {
        tool.tick(&ctx);
        tool.run_log.is_running()
    });
    assert!(
        tool.live_check.is_running(),
        "premise: the run's end checks"
    );
    finish_check(&mut tool, &ctx);
    window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(!tool.live_check.is_running(), "no other once one started");
}

// While a check is in flight the window asks for a repaint, so its rows show without
// an input.
#[test]
fn the_window_repaints_while_a_check_is_in_flight() {
    let ctx = settings_only_context();
    let log = RunLog::default();
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let mut window = Headless::new();
    let mut frame = |checking| {
        window.frame(Vec::new(), |ui| {
            show(ui, &ctx, &log, &mut panel, &[], checking, &mut detail)
        });
        window.repaint_delay
    };
    // A new window's first two frames ask for an immediate repaint of their own.
    frame(false);
    frame(false);
    assert_eq!(frame(false), Duration::MAX, "premise: none asked when idle");
    assert!(frame(true) <= Duration::from_millis(100));
}

#[test]
fn a_refused_check_is_a_log_line_and_the_grid_keeps_its_rows() {
    let temp = sandbox("view_grid_refused");
    // An absolute exports folder that does not exist: the preflight refuses it.
    let missing = temp.path().join("missing");
    let setting = format!("exports_folder_path = '{}'\n", missing.display());
    let ctx = tool_context(temp.path(), &setting);
    let mut tool = Tool::new();
    let (release_check, check_until) = unbounded();
    tool.live_check = LiveCheck::in_flight(check_until, vec![label_only_row()]);
    release_check.send(()).unwrap();
    finish_check(&mut tool, &ctx);
    tool.live_check.start(&ctx);
    finish_check(&mut tool, &ctx);
    assert_eq!(tool.live_check.rows(), [label_only_row()]);
    let lines = tool.run_log.texts();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("error: "), "{lines:?}");
}
