//! The Compile button: a click reported on release, disabled while a run is in
//! flight, and the run a click starts.

use super::*;

/// The node of the button (or collapsing header) labeled `label`.
fn button<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| node.label() == Some(label))
        .unwrap_or_else(|| panic!("no widget labeled {label:?}"))
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

// TC-GUI-03
#[test]
fn without_a_click_nothing_is_reported_and_compile_is_enabled() {
    let ctx = settings_only_context();
    let log = RunLog::default();
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let (clicked, nodes) = Headless::new().frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    assert!(!clicked);
    assert!(!button(&nodes, "Compile").is_disabled());
}

#[test]
fn a_click_on_compile_is_reported_on_the_frame_of_the_release() {
    let ctx = settings_only_context();
    let log = RunLog::default();
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let mut window = Headless::new();
    // The first frame lays the button out; egui hit-tests a click against the previous
    // frame's widgets, so the press comes on the second frame and the release on the third.
    let (_, nodes) = window.frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    let target = centre(button(&nodes, "Compile"));
    let (clicked, _) = window.frame(press(target), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    assert!(!clicked, "not before the release");
    let (clicked, _) = window.frame(release(target), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    assert!(clicked);
}

// TC-GUI-03
#[test]
fn compile_is_disabled_while_a_run_is_in_flight() {
    let ctx = settings_only_context();
    let (release_run, until) = unbounded();
    let mut log = RunLog::in_flight(until);
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let (_, nodes) = Headless::new().frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    assert!(button(&nodes, "Compile").is_disabled());
    release_run.send(()).unwrap();
    poll_until_done(|| {
        log.poll();
        log.is_running()
    });
    assert_eq!(log.texts(), ["Run finished with exit code 0"]);
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
    // The view started the grid's check on its first frame, and the run's end another.
    finish_check(&mut tool, &ctx);
}
