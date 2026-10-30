//! The Team compiler's window, render-only (`spec/team_compiler/README.md` "Crate layout"): it
//! reads the tool's state and reports the user's intent (Compile clicked); it never calls into
//! the pipeline itself. It shows the effective settings read-only, the Compile button and the
//! log panel; the export grid and the toolbar are not built yet.

use std::time::Duration;

use studio_core::widgets::add_enabled;
use studio_core::{HelpTarget, LogPanel, ToolContext};

use crate::gui_run::RunLog;
use crate::messages::TOOL_ID;

/// Shows the window; `true` when Compile was clicked this frame.
pub(crate) fn show(
    ui: &mut egui::Ui,
    ctx: &ToolContext,
    log: &RunLog,
    panel: &mut LogPanel,
) -> bool {
    // The dock claims the bottom first (`spec/core/gui.md` "Shell layout"); the
    // settings and the button take what is left. A
    // clicked message code is a shell request, not a pipeline call: the view
    // stays render-only.
    if let Some(code) = panel.show(ui, log.lines()) {
        ctx.open_help(HelpTarget::message(&code));
    }
    studio_core::widgets::collapsing_header("Settings").show(ui, |ui| show_settings(ui, ctx));
    let clicked = add_enabled(ui, !log.is_running(), egui::Button::new("Compile")).clicked();
    if log.is_running() {
        // egui repaints only on input, and the run's events arrive on their own: without this
        // the log would advance only when the mouse moves.
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
    clicked
}

/// The effective settings, as the settings file spells them: the common ones the compiler
/// reads, then the tool's own section.
fn show_settings(ui: &mut egui::Ui, ctx: &ToolContext) {
    let common = ctx.common();
    ui.monospace(format!("pes_version = {}", common.pes_version.number()));
    ui.monospace(format!("pes_folder_path = {}", common.pes_folder_path));
    ui.monospace(format!(
        "exports_folder_path = {}",
        common.exports_folder_path.display()
    ));
    for (key, value) in &ctx.tool_settings(TOOL_ID) {
        ui.monospace(format!("{key} = {value}"));
    }
    ui.label("Edit them in the settings menu, or in settings.toml.");
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crossbeam_channel::unbounded;
    use egui::accesskit;
    use studio_core::StudioTool;

    use super::*;
    use crate::Tool;
    use crate::testing::{install_pes, poll_until_done, sandbox, tool_context};

    /// A headless window: one egui context with AccessKit on and animations off (so a
    /// collapsing header is fully open on the frame after its click), run a frame at a time.
    struct Headless {
        ctx: egui::Context,
    }

    impl Headless {
        fn new() -> Headless {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            // The window draws with these fonts and style; the test measures
            // what a member sees, whichever crates the build unifies.
            studio_core::theme::install(&ctx);
            ctx.global_style_mut(|style| style.animation_time = 0.0);
            Headless { ctx }
        }

        /// Runs one frame of `contents` on an 800 by 600 screen with `events` as its input;
        /// returns what `contents` returned and the frame's AccessKit nodes.
        fn frame<R>(
            &mut self,
            events: Vec<egui::Event>,
            mut contents: impl FnMut(&mut egui::Ui) -> R,
        ) -> (R, Vec<accesskit::Node>) {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                events,
                ..Default::default()
            };
            let mut result = None;
            let mut output = self.ctx.run_ui(input, |ui| result = Some(contents(ui)));
            let update = output
                .platform_output
                .accesskit_update
                .take()
                .expect("AccessKit is enabled on the context");
            // Nothing renders here, so the frame's texture deltas are dropped on purpose.
            output.drop_without_applying_deltas();
            let nodes = update.nodes.into_iter().map(|(_, node)| node).collect();
            (result.expect("run_ui calls the closure"), nodes)
        }
    }

    /// The node of the button (or collapsing header) labeled `label`.
    fn button<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
        nodes
            .iter()
            .find(|node| node.label() == Some(label))
            .unwrap_or_else(|| panic!("no widget labeled {label:?}"))
    }

    /// Whether some label widget shows exactly `text`. egui puts a label's text in its
    /// AccessKit node's value, not its label.
    fn shows_text(nodes: &[accesskit::Node], text: &str) -> bool {
        nodes.iter().any(|node| node.value() == Some(text))
    }

    /// The centre of a node's bounds, where a click lands.
    fn centre(node: &accesskit::Node) -> egui::Pos2 {
        let bounds = node.bounds().expect("a widget has bounds");
        egui::pos2(
            ((bounds.x0 + bounds.x1) / 2.0) as f32,
            ((bounds.y0 + bounds.y1) / 2.0) as f32,
        )
    }

    /// The pointer moved to `pos` and the primary button went down there.
    fn press(pos: egui::Pos2) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
        ]
    }

    /// The primary button came up at `pos`: egui reports the click on this frame.
    fn release(pos: egui::Pos2) -> Vec<egui::Event> {
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        }]
    }

    /// A context with no sandbox behind it, for frames that start no run.
    fn settings_only_context() -> ToolContext {
        tool_context(Path::new("exe"), "")
    }

    // TC-GUI-03
    #[test]
    fn without_a_click_nothing_is_reported_and_compile_is_enabled() {
        let ctx = settings_only_context();
        let log = RunLog::default();
        let mut panel = LogPanel::new(TOOL_ID);
        let (clicked, nodes) =
            Headless::new().frame(Vec::new(), |ui| show(ui, &ctx, &log, &mut panel));
        assert!(!clicked);
        assert!(!button(&nodes, "Compile").is_disabled());
    }

    #[test]
    fn a_click_on_compile_is_reported_on_the_frame_of_the_release() {
        let ctx = settings_only_context();
        let log = RunLog::default();
        let mut panel = LogPanel::new(TOOL_ID);
        let mut window = Headless::new();
        // The first frame lays the button out; egui hit-tests a click against the previous
        // frame's widgets, so the press comes on the second frame and the release on the third.
        let (_, nodes) = window.frame(Vec::new(), |ui| show(ui, &ctx, &log, &mut panel));
        let target = centre(button(&nodes, "Compile"));
        let (clicked, _) = window.frame(press(target), |ui| show(ui, &ctx, &log, &mut panel));
        assert!(!clicked, "not before the release");
        let (clicked, _) = window.frame(release(target), |ui| show(ui, &ctx, &log, &mut panel));
        assert!(clicked);
    }

    // TC-GUI-03
    #[test]
    fn compile_is_disabled_while_a_run_is_in_flight() {
        let ctx = settings_only_context();
        let (release_run, until) = unbounded();
        let mut log = RunLog::in_flight(until);
        let mut panel = LogPanel::new(TOOL_ID);
        let (_, nodes) = Headless::new().frame(Vec::new(), |ui| show(ui, &ctx, &log, &mut panel));
        assert!(button(&nodes, "Compile").is_disabled());
        release_run.send(()).unwrap();
        poll_until_done(|| {
            log.poll();
            log.is_running()
        });
        assert_eq!(log.texts(), ["Run finished with exit code 0"]);
    }

    // TC-GUI-03
    #[test]
    fn the_settings_section_opens_on_the_effective_settings() {
        let ctx = settings_only_context();
        let log = RunLog::default();
        let mut panel = LogPanel::new(TOOL_ID);
        let mut window = Headless::new();
        let (_, nodes) = window.frame(Vec::new(), |ui| show(ui, &ctx, &log, &mut panel));
        assert!(
            !shows_text(&nodes, "pes_version = 21"),
            "collapsed at first"
        );
        let header = centre(button(&nodes, "Settings"));
        window.frame(press(header), |ui| show(ui, &ctx, &log, &mut panel));
        window.frame(release(header), |ui| show(ui, &ctx, &log, &mut panel));
        let (_, nodes) = window.frame(Vec::new(), |ui| show(ui, &ctx, &log, &mut panel));
        assert!(shows_text(&nodes, "pes_version = 21"));
        assert!(shows_text(&nodes, "cpk_name = \"4cc_99_test\""));
    }

    // TC-GUI-02
    #[test]
    fn a_compile_click_through_the_tool_starts_a_run() {
        let temp = sandbox("view_compile_click");
        let root = temp.path();
        install_pes(root);
        let ctx = tool_context(root, "");
        let mut tool = Tool::new();
        let mut window = Headless::new();
        let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
        let target = centre(button(&nodes, "Compile"));
        window.frame(press(target), |ui| tool.view(ui, &ctx));
        window.frame(release(target), |ui| tool.view(ui, &ctx));
        assert!(tool.run_log.is_running());
        poll_until_done(|| {
            tool.tick(&ctx);
            tool.run_log.is_running()
        });
        assert_eq!(
            tool.run_log.texts(),
            [
                "- Info bin_source (bin=TeamColor.bin, cpk=bundled)".to_owned(),
                "- Info bin_source (bin=UniColor.bin, cpk=bundled)".to_owned(),
                "- Info bin_source (bin=UniformParameter.bin, cpk=bundled)".to_owned(),
                "- egg Midcup Tracer: Info fmdl_weights_not_normalized at Players/05 - The Chad Stormworks Player (file=boots.fmdl, count=1662)".to_owned(),
                "- egg Midcup Tracer: Info fmdl_weights_not_normalized at Players/05 - The Chad Stormworks Player (file=fcl_hair.fmdl, count=1662)".to_owned(),
                "- egg Midcup Tracer: Info fmdl_weights_not_normalized at Players/05 - The Chad Stormworks Player (file=glove_l.fmdl, count=2)".to_owned(),
                "- egg Midcup Tracer: Info export_identified (team=/egg/, id=792)".to_owned(),
                "- Warning player_table_missing (table=BootsList.bin, rows=1)".to_owned(),
                "- Warning player_table_missing (table=GloveList.bin, rows=1)".to_owned(),
                "- Warning player_table_missing (table=PlayerAppearance.bin, rows=1)".to_owned(),
                format!(
                    "- Info patch_written (path={}, teams=792)",
                    root.join("output")
                        .join("aesthetics_patch.toml")
                        .display()
                ),
                format!(
                    "- Warning savefile_missing (patch={}, path={}, command={})",
                    root.join("output")
                        .join("aesthetics_patch.toml")
                        .display(),
                    root.join("save").join("EDIT00000000").display(),
                    crate::output::savefile::APPLY_COMMAND
                ),
                "Run finished with exit code 0".to_owned(),
            ]
        );
    }

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
        let mut window = Headless::new();
        // The log overflows the panel's body: stuck to the end, the offset is off the top.
        window.frame(Vec::new(), |ui| show(ui, &ctx, &log, &mut panel));
        let before = scroll_offset(&window.ctx);
        assert!(before > 0.0, "stuck to the bottom of a log that overflows");
        // A line arrives: the offset grows with the content, the last line still shown.
        log.append_test_lines(&["log line 41"]);
        window.frame(Vec::new(), |ui| show(ui, &ctx, &log, &mut panel));
        let after = scroll_offset(&window.ctx);
        assert!(
            after > before,
            "the bottom follows new lines: {after} vs {before}"
        );
    }
}
