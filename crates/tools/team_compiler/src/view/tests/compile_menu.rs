//! The Compile button's split: its arrow's menu of output modes, a pick relabelling
//! the main part for the session, Open output folder and Copy report (TC-GUI-11).

use super::*;

/// The Compile button's arrow, labelled by the arrow alone (the mode entries and the
/// menu buttons carry their text first).
const ARROW: &str = "\u{23f7}";

/// The sideload entry's hover text while no sideloading runtime serves the selected
/// PES version.
const UNSERVED_TIP: &str = "No runtime serves sideloaded files on PES 2015 and 2016";
/// Its hover text while one does: where the sideloaded files are unpacked.
const SERVED_TIP: &str = "Unpacked into the game folder's livecpk";

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

/// The arrow clicked once: the nodes of the frame the click leaves, the mode list
/// drawn open over the toolbar.
fn open_mode_menu(
    window: &mut Headless,
    ctx: &ToolContext,
    log: &RunLog,
    panel: &mut LogPanel,
    detail: &mut GridDetail,
) -> Vec<accesskit::Node> {
    let (_, nodes) = window.frame(Vec::new(), |ui| {
        show(ui, ctx, log, panel, &[], false, detail)
    });
    click_on(
        window,
        button(&nodes, ARROW),
        view_frame(ctx, log, panel, detail),
    )
}

/// A click on the main part labelled `label`: the press frame asserting nothing comes
/// before the release, which is where the click lands. Returns that frame's `Intent`
/// and nodes.
fn main_part_click(
    window: &mut Headless,
    ctx: &ToolContext,
    log: &RunLog,
    panel: &mut LogPanel,
    detail: &mut GridDetail,
    label: &str,
) -> (Option<Intent>, Vec<accesskit::Node>) {
    let (_, nodes) = window.frame(Vec::new(), |ui| {
        show(ui, ctx, log, panel, &[], false, detail)
    });
    let at = centre(button(&nodes, label));
    let (clicked, _) = window.frame(press(at), |ui| {
        show(ui, ctx, log, panel, &[], false, detail)
    });
    assert!(clicked.is_none(), "not before the release");
    window.frame(release(at), |ui| {
        show(ui, ctx, log, panel, &[], false, detail)
    })
}

// TC-GUI-11
#[test]
fn a_mode_pick_relabels_the_main_part_and_its_click_runs_that_mode() {
    let ctx = settings_only_context();
    let log = RunLog::default();
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let mut window = Headless::new();
    let mut nodes = open_mode_menu(&mut window, &ctx, &log, &mut panel, &mut detail);
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
    // Normal to Test output to Compile for sideloading to Compile: each pick is
    // looked up while the main part reads another mode, so its label is the menu
    // entry's alone, and the main part relabels to it for the rest of the session.
    for (pick, mode) in [
        ("Test output", Mode::Test),
        ("Compile for sideloading", Mode::Sideload),
        ("Compile", Mode::Normal),
    ] {
        nodes = click_on(
            &mut window,
            button(&nodes, pick),
            view_frame(&ctx, &log, &mut panel, &mut detail),
        );
        assert!(
            !labeled(&nodes, "Open output folder"),
            "the pick closed the menu"
        );
        assert!(labeled(&nodes, pick), "the main part reads {pick}");
        let (clicked, _) = main_part_click(&mut window, &ctx, &log, &mut panel, &mut detail, pick);
        assert!(
            matches!(clicked, Some(Intent::Compile(ran)) if ran == mode),
            "the main part runs the picked mode"
        );
        // Reopened for the next pick.
        nodes = open_mode_menu(&mut window, &ctx, &log, &mut panel, &mut detail);
    }
}

// TC-GUI-11
#[test]
fn the_mode_menu_opens_above_the_arrow_right_aligned() {
    let ctx = settings_only_context();
    let log = RunLog::default();
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let mut window = Headless::new();
    open_mode_menu(&mut window, &ctx, &log, &mut panel, &mut detail);
    // A popup's first pass only measures; the next frame holds its aligned place.
    let margin = std::cell::Cell::new(0.0_f32);
    let stroke = std::cell::Cell::new(0.0_f32);
    let (_, nodes) = window.frame(Vec::new(), |ui| {
        margin.set(ui.spacing().menu_margin.right as f32);
        stroke.set(ui.visuals().window_stroke().width);
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    let arrow = node_rect(button(&nodes, ARROW)).expect("the arrow has bounds");
    // "Compile" is left out: the main part reads the same while the mode is
    // Normal. The four measured entries span the menu's width and bottom edge.
    let rects: Vec<egui::Rect> = [
        "Compile for sideloading",
        "Test output",
        "Open output folder",
        "Copy report",
    ]
    .into_iter()
    .map(|label| node_rect(button(&nodes, label)).expect("an open menu entry has bounds"))
    .collect();
    let bottom = rects
        .iter()
        .map(|rect| rect.max.y)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        bottom <= arrow.min.y,
        "the lowest entry's bottom edge {bottom} sits at or above the arrow's top {}",
        arrow.min.y
    );
    // The entries' nodes carry their own rects, not the popup's: the menu's right
    // edge is their shared right edge plus the popup's margin and frame stroke,
    // and it is that edge that meets the arrow's right edge.
    let right = rects
        .iter()
        .map(|rect| rect.max.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let inset = arrow.max.x - right;
    assert!(
        0.0 < inset && inset <= margin.get() + stroke.get() + 1.0,
        "the widest entry's right edge {right} sits the popup's margin {} and stroke {} \
         inside the arrow's right edge {}",
        margin.get(),
        stroke.get(),
        arrow.max.x
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

    // The run's end started the grid's check again, whose rows replace the last
    // check's; the picked mode survives it.
    finish_check(&mut tool, &ctx);
    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(
        labeled(&nodes, "Test output") && !labeled(&nodes, "Compile"),
        "the run's closing check left the main part on the picked mode"
    );

    // The mode lasted the session alone: a fresh tool over the same context draws the
    // main part as "Compile", the mode not a saved one.
    let mut fresh = Tool::new();
    let (_, nodes) = window.frame(Vec::new(), |ui| fresh.view(ui, &ctx));
    assert!(
        labeled(&nodes, "Compile") && !labeled(&nodes, "Test output"),
        "a fresh tool's main part reads Compile"
    );
    finish_check(&mut fresh, &ctx);
}

// TC-GUI-11
#[test]
fn sideloading_is_off_on_pes_2015_and_2016_and_on_later_ones() {
    for (version, disabled) in [
        (PesVersion::Pes15, true),
        (PesVersion::Pes16, true),
        (PesVersion::Pes17, false),
        (PesVersion::Pes21, false),
    ] {
        let temp = sandbox("view_compile_menu_sideload_version");
        let ctx = tool_context_at(temp.path(), version);
        let mut tool = Tool::new();
        let mut window = Headless::new();
        let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
        let nodes = click_labeled(&mut window, &nodes, ARROW, &mut tool, &ctx);
        let entry = button(&nodes, "Compile for sideloading");
        assert_eq!(entry.is_disabled(), disabled, "{version}");
        // The entry's hover text is the member's only why on a greyed-out version;
        // the pointer resting on the entry holds the menu open while it shows.
        let (tip, other) = if disabled {
            (UNSERVED_TIP, SERVED_TIP)
        } else {
            (SERVED_TIP, UNSERVED_TIP)
        };
        tooltip_is(&mut window, entry, tip, |window, events| {
            window.frame(events, |ui| tool.view(ui, &ctx)).1
        });
        let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
        assert!(
            !shows_text(&nodes, other),
            "{version}: no {other:?} shows beside {tip:?}"
        );
    }
}

// TC-GUI-11
#[test]
fn a_sideload_pick_disables_the_main_part_on_an_unserved_version() {
    let temp = sandbox("view_compile_menu_sideload_pick");
    let (ctx, settings, _requests) = tool_context_with_settings(temp.path(), PesVersion::Pes17);
    let log = RunLog::default();
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let mut window = Headless::new();
    let nodes = open_mode_menu(&mut window, &ctx, &log, &mut panel, &mut detail);
    let nodes = click_on(
        &mut window,
        button(&nodes, "Compile for sideloading"),
        view_frame(&ctx, &log, &mut panel, &mut detail),
    );
    assert!(
        !button(&nodes, "Compile for sideloading").is_disabled(),
        "premise: the pick sticks and the main part stays enabled on PES 2017"
    );
    settings.lock().unwrap().common.pes_version = PesVersion::Pes16;
    let (_, nodes) = window.frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    let main = button(&nodes, "Compile for sideloading");
    assert!(
        main.is_disabled(),
        "the pick stays but PES 2016 cannot serve it: the main part is disabled"
    );
    tooltip_is(
        &mut window,
        main,
        UNSERVED_TIP,
        view_frame(&ctx, &log, &mut panel, &mut detail),
    );
    settings.lock().unwrap().common.pes_version = PesVersion::Pes17;
    let (_, nodes) = window.frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    assert!(
        !button(&nodes, "Compile for sideloading").is_disabled(),
        "a switch back to PES 2017 enables the main part again"
    );
}

// An outdated list stops the mode that installs the CPKs alone.
// TC-GUI-33
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

    let nodes = click_labeled(&mut window, &nodes, ARROW, &mut tool, &ctx);
    let nodes = click_labeled(&mut window, &nodes, "Test output", &mut tool, &ctx);
    assert!(
        !labeled(&nodes, "Open output folder"),
        "the pick closed the menu"
    );
    assert!(
        !button(&nodes, "Test output").is_disabled(),
        "the test mode installs no CPK, so it stays enabled"
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
