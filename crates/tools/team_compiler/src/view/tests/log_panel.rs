//! The log panel: a line arriving while it is scrolled to the bottom keeps the
//! last line in view.

use super::*;

/// The log body's scroll offset: its `scroll_area::State` under the id the
/// panel's `id_salt` chain builds — the root ui's top id, the open dock panel's
/// resolved id (egui 0.37 salts the panel's child scope with it: the parent
/// scope salted by the tool's own id), the panel frame's `child` content salt,
/// the body frame's, then the dock id's `body` child salt (`studio_core`'s
/// `widgets/log_panel.rs`).
fn scroll_offset(ctx: &egui::Context) -> f32 {
    let top = egui::Id::unique((egui::ViewportId::ROOT, "__top_ui"));
    let id = top
        .with(egui::IdSalt::new(top.with(egui::Id::unique(TOOL_ID))))
        .with(egui::IdSalt::new("child"))
        .with(egui::IdSalt::new("child"))
        .with(egui::IdSalt::new(egui::Id::unique(TOOL_ID).with("body")));
    egui::scroll_area::State::load(ctx, id)
        .map(|state| state.clamped_offset().y)
        .expect("the log body's scroll area")
}

// TC-GUI-03
#[test]
fn a_log_line_arriving_keeps_the_scroll_area_at_the_bottom() {
    let ctx = settings_only_context();
    let mut log = RunLog::default();
    let lines: Vec<String> = (1..=40).map(|line| format!("log line {line}")).collect();
    log.append_test_lines(&lines.iter().map(String::as_str).collect::<Vec<_>>());
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let mut window = Headless::new();
    // The log overflows the panel's body: stuck to the end, the offset is off the top.
    window.frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    let before = scroll_offset(&window.ctx);
    assert!(before > 0.0, "stuck to the bottom of a log that overflows");
    // A line arrives: the offset grows with the content, the last line still shown.
    log.append_test_lines(&["log line 41"]);
    window.frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    let after = scroll_offset(&window.ctx);
    assert!(
        after > before,
        "the bottom follows new lines: {after} vs {before}"
    );
}
