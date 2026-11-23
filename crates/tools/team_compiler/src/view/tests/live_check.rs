//! The grid check's lifecycle: when the window starts one, what holds its rows
//! back, and what a refused check leaves behind.

use super::*;

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
