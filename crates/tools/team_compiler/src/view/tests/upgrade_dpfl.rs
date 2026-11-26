//! The Upgrade DpFileList button and its dialog (TC-GUI-33): the button the check's
//! findings about the game's `DpFileList.bin` bring beside Compile, the dialog its click
//! opens on the upgrade's preview, and the upgrade run the dialog's Upgrade starts.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::upgrade::{self, Preview};

/// The files of `folder`, by name, with their bytes.
fn snapshot(folder: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fs::read_dir(folder)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name().into(), fs::read(entry.path()).unwrap())
        })
        .collect()
}

/// What the upgrade leaves of `download`, run on a `twin/` sibling copy of it with
/// `delete`: the dialog's expected result, by `upgrade::run` itself.
fn upgrade_leaves(
    download: &Path,
    delete: &BTreeSet<String>,
    ctx: &ToolContext,
) -> BTreeMap<PathBuf, Vec<u8>> {
    let twin = download.with_file_name("twin");
    copy_tree(download, &twin);
    upgrade::run(&twin, true, delete, ctx).unwrap();
    snapshot(&twin)
}

/// The nodes after the Upgrade DpFileList button's click on the `nodes` frame opens the
/// dialog: the click, then two frames (a fresh modal's first is a sizing pass, the
/// second draws it).
fn dialog_open(
    nodes: &[accesskit::Node],
    tool: &mut Tool,
    ctx: &ToolContext,
    window: &mut Headless,
) -> Vec<accesskit::Node> {
    click_labeled(window, nodes, "Upgrade DpFileList", tool, ctx);
    assert!(
        tool.detail.dpfl_dialog.is_some(),
        "the click opened the dialog"
    );
    window.frame(Vec::new(), |ui| tool.view(ui, ctx));
    window.frame(Vec::new(), |ui| tool.view(ui, ctx)).1
}

/// The TC-GUI-33 scene to the open dialog: `first_check`, then the button's click.
fn open_dialog(tool: &mut Tool, ctx: &ToolContext, window: &mut Headless) -> Vec<accesskit::Node> {
    dialog_open(&first_check(tool, ctx, window), tool, ctx, window)
}

// TC-GUI-33
#[test]
fn the_outdated_list_brings_the_button_whose_dialog_upgrades_it() {
    let temp = sandbox("view_dpfl_upgrade");
    let root = temp.path();
    install(root, 2017, &pes17_list(), None);
    let ctx = tool_context_at(root, PesVersion::Pes17);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = first_check(&mut tool, &ctx, &mut window);

    // Before any Compile: the check's finding is a line of the log, the button shows,
    // and Compile is disabled with why.
    let texts = tool.run_log.texts();
    assert!(
        texts
            .iter()
            .any(|line| line.contains("dpfilelist_outdated")),
        "the first check found the outdated list: {texts:?}"
    );
    assert!(labeled(&nodes, "Upgrade DpFileList"));
    let compile = button(&nodes, "Compile");
    assert!(compile.is_disabled());
    tooltip_is(
        &mut window,
        compile,
        "The game's DpFileList.bin does not list this compile's CPK, so the game would \
         not load it. Upgrade DpFileList first",
        |window, events| window.frame(events, |ui| tool.view(ui, &ctx)).1,
    );

    dialog_open(&nodes, &mut tool, &ctx, &mut window);
    let download = root.join("PES/download");

    // The dialog says what the upgrade will do before anything is written.
    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    for text in [
        "Upgrade replaces the game's DpFileList.bin with the cup's official list, keeping the old one beside it as DpFileList.bin.bak.",
        "Renamed to their official names, so the game keeps loading them (13):",
        "Current name",
        "Official name",
        "4cc_40_faces.cpk",
        "4cc_41_teams.cpk",
        "An empty placeholder is written for each of the 19 official CPKs with no file, because the game loads nothing from download/ while a CPK its list names is missing.",
        "No longer loaded by the game (5). Tick any you want deleted:",
    ] {
        assert!(shows_text(&nodes, text), "the dialog shows {text:?}");
    }
    let boxes = nodes
        .iter()
        .filter(|node| node.role() == accesskit::Role::CheckBox)
        .count();
    assert_eq!(boxes, 5, "one delete box per dropped CPK with a file");
    assert!(labeled(&nodes, "4cc_60_midcup.cpk (6.1 KiB)"));
    assert!(labeled(&nodes, "Cancel") && labeled(&nodes, "Upgrade"));

    // Upgrade: `download/` holds what `upgrade-dpfl --yes` leaves, its lines appended to
    // the log, the dialog closed, and the button gone.
    let expected = upgrade_leaves(&download, &BTreeSet::new(), &ctx);
    click_labeled(&mut window, &nodes, "Upgrade", &mut tool, &ctx);
    assert!(
        tool.detail.dpfl_dialog.is_none(),
        "Upgrade closed the dialog"
    );
    poll_until_done(|| {
        tool.tick(&ctx);
        tool.run_log.is_running()
    });
    assert_eq!(snapshot(&download), expected);
    let texts = tool.run_log.texts();
    for line in [
        "dpfilelist_cpk_renamed (from=4cc_40_faces.cpk, to=4cc_41_teams.cpk)",
        "dpfilelist_replaced",
        "dpfilelist_cpk_dropped (cpk=4cc_60_midcup.cpk, size=6.1 KiB)",
        "Run finished with exit code 0",
    ] {
        assert!(
            texts.iter().any(|text| text.contains(line)),
            "the log holds {line:?}: {texts:?}"
        );
    }
    // The run's end starts a check: once it ends, the button is gone and Compile is
    // enabled again.
    finish_check(&mut tool, &ctx);
    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(
        !labeled(&nodes, "Upgrade DpFileList"),
        "the button is gone once the list is replaced"
    );
    assert!(!button(&nodes, "Compile").is_disabled());

    // The next Compile passes the preflight.
    click_labeled(&mut window, &nodes, "Compile", &mut tool, &ctx);
    poll_until_done(|| {
        tool.tick(&ctx);
        tool.run_log.is_running()
    });
    let texts = tool.run_log.texts();
    assert!(
        !texts
            .iter()
            .any(|line| line.contains("dpfilelist_outdated")),
        "the second run passes the preflight: {texts:?}"
    );
    // The run reached past it: the installed CPKs' walk refuses the fake archives.
    assert!(
        texts
            .iter()
            .any(|line| line.contains("installed_bin_unreadable")),
        "the second run reached the bins walk: {texts:?}"
    );
}

// TC-GUI-33
#[test]
fn a_list_not_the_official_one_shows_the_button_and_keeps_compile_enabled() {
    let temp = sandbox("view_dpfl_not_official");
    let root = temp.path();
    // The list names every CPK the run installs plus one entry the official list lacks.
    let mut names = crate::templates::Templates::embedded().official_list();
    names.push("4cc_80_mine.cpk".to_owned());
    let borrowed: Vec<&str> = names.iter().map(String::as_str).collect();
    install(root, 2017, &crate::testing::dpfilelist(&borrowed), None);
    let ctx = tool_context_at(root, PesVersion::Pes17);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = first_check(&mut tool, &ctx, &mut window);

    let texts = tool.run_log.texts();
    assert!(
        texts
            .iter()
            .any(|line| line.contains("dpfilelist_not_official")),
        "the first check found the list not the official one: {texts:?}"
    );
    assert!(labeled(&nodes, "Upgrade DpFileList"));
    assert!(
        !button(&nodes, "Compile").is_disabled(),
        "the Warning leaves Compile enabled"
    );

    // The upgrade replaces the list; once the check after its run ends, the button is gone.
    let nodes = dialog_open(&nodes, &mut tool, &ctx, &mut window);
    click_labeled(&mut window, &nodes, "Upgrade", &mut tool, &ctx);
    poll_until_done(|| {
        tool.tick(&ctx);
        tool.run_log.is_running()
    });
    finish_check(&mut tool, &ctx);
    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(
        !labeled(&nodes, "Upgrade DpFileList"),
        "the button is gone once the check finds the list official"
    );
}

// TC-GUI-33
#[test]
fn an_upgrade_that_only_writes_placeholders_takes_the_button_away() {
    let temp = sandbox("view_dpfl_placeholder");
    let root = temp.path();
    install(
        root,
        2017,
        crate::templates::Templates::embedded().official_list_file(),
        Some("4cc_16_balls.cpk"),
    );
    let ctx = tool_context_at(root, PesVersion::Pes17);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = first_check(&mut tool, &ctx, &mut window);

    let texts = tool.run_log.texts();
    assert!(
        texts
            .iter()
            .any(|line| line.contains("dpfilelist_cpk_missing")),
        "the first check found the missing CPK: {texts:?}"
    );
    assert!(labeled(&nodes, "Upgrade DpFileList"));
    assert!(
        !button(&nodes, "Compile").is_disabled(),
        "the Warning leaves Compile enabled"
    );

    // The upgrade only writes the missing placeholder: no `dpfilelist_replaced` line
    // clears the state, so the check after the run does.
    let nodes = dialog_open(&nodes, &mut tool, &ctx, &mut window);
    click_labeled(&mut window, &nodes, "Upgrade", &mut tool, &ctx);
    poll_until_done(|| {
        tool.tick(&ctx);
        tool.run_log.is_running()
    });
    let texts = tool.run_log.texts();
    assert!(
        texts
            .iter()
            .any(|line| line.contains("dpfilelist_placeholder_written"))
            && !texts
                .iter()
                .any(|line| line.contains("dpfilelist_replaced")),
        "the placeholder written, the list not replaced: {texts:?}"
    );
    finish_check(&mut tool, &ctx);
    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(
        !labeled(&nodes, "Upgrade DpFileList"),
        "the button is gone once the check finds the list official"
    );
}

// TC-GUI-33
#[test]
fn a_ticked_delete_box_removes_only_that_cpk() {
    let temp = sandbox("view_dpfl_tick");
    let root = temp.path();
    install(root, 2017, &pes17_list(), None);
    let ctx = tool_context_at(root, PesVersion::Pes17);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = open_dialog(&mut tool, &ctx, &mut window);
    let download = root.join("PES/download");

    // The box starts unticked and stays ticked once clicked, over the frames the dialog
    // is up: the preview it was opened on is not read again.
    let midcup = "4cc_60_midcup.cpk (6.1 KiB)";
    assert_eq!(
        button(&nodes, midcup).toggled(),
        Some(accesskit::Toggled::False),
        "premise: the box starts unticked"
    );
    click_labeled(&mut window, &nodes, midcup, &mut tool, &ctx);
    window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert_eq!(
        button(&nodes, midcup).toggled(),
        Some(accesskit::Toggled::True),
        "the box stays ticked"
    );

    // That file alone is deleted, its line `dpfilelist_cpk_deleted`.
    let delete = BTreeSet::from(["4cc_60_midcup.cpk".to_owned()]);
    let expected = upgrade_leaves(&download, &delete, &ctx);
    click_labeled(&mut window, &nodes, "Upgrade", &mut tool, &ctx);
    poll_until_done(|| {
        tool.tick(&ctx);
        tool.run_log.is_running()
    });
    assert_eq!(snapshot(&download), expected);
    assert!(!download.join("4cc_60_midcup.cpk").exists());
    for name in ["4cc_76_midcup.cpk", "4cc_77_midcup.cpk"] {
        assert!(download.join(name).exists(), "{name} kept");
    }
    let texts = tool.run_log.texts();
    assert!(
        texts
            .iter()
            .any(|line| line.contains("dpfilelist_cpk_deleted (cpk=4cc_60_midcup.cpk")),
        "the deleted line: {texts:?}"
    );
}

// TC-GUI-33
#[test]
fn an_unticked_box_deletes_nothing() {
    let temp = sandbox("view_dpfl_untick");
    let root = temp.path();
    install(root, 2017, &pes17_list(), None);
    let ctx = tool_context_at(root, PesVersion::Pes17);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = open_dialog(&mut tool, &ctx, &mut window);
    let download = root.join("PES/download");
    let midcup = "4cc_60_midcup.cpk (6.1 KiB)";

    let nodes = click_labeled(&mut window, &nodes, midcup, &mut tool, &ctx);
    assert_eq!(
        button(&nodes, midcup).toggled(),
        Some(accesskit::Toggled::True),
        "premise: the box is ticked"
    );
    let nodes = click_labeled(&mut window, &nodes, midcup, &mut tool, &ctx);
    assert_eq!(
        button(&nodes, midcup).toggled(),
        Some(accesskit::Toggled::False),
        "the box is unticked again"
    );

    let expected = upgrade_leaves(&download, &BTreeSet::new(), &ctx);
    click_labeled(&mut window, &nodes, "Upgrade", &mut tool, &ctx);
    poll_until_done(|| {
        tool.tick(&ctx);
        tool.run_log.is_running()
    });
    assert_eq!(snapshot(&download), expected);
    assert!(download.join("4cc_60_midcup.cpk").exists());
    let texts = tool.run_log.texts();
    assert!(
        texts
            .iter()
            .any(|line| line.contains("dpfilelist_cpk_dropped (cpk=4cc_60_midcup.cpk")),
        "the file kept, its line the dropped one: {texts:?}"
    );
    assert!(
        !texts
            .iter()
            .any(|line| line.contains("dpfilelist_cpk_deleted")),
        "and no deleted line: {texts:?}"
    );
}

// TC-GUI-33
#[test]
fn the_dialog_s_cancel_writes_nothing() {
    let temp = sandbox("view_dpfl_cancel");
    let root = temp.path();
    install(root, 2017, &pes17_list(), None);
    let ctx = tool_context_at(root, PesVersion::Pes17);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let download = root.join("PES/download");
    let before = snapshot(&download);
    let nodes = open_dialog(&mut tool, &ctx, &mut window);

    let nodes = click_labeled(&mut window, &nodes, "Cancel", &mut tool, &ctx);
    assert!(
        tool.detail.dpfl_dialog.is_none(),
        "Cancel closed the dialog"
    );
    assert_eq!(snapshot(&download), before, "nothing written");
    // The button is back, the list still not the official one.
    assert!(labeled(&nodes, "Upgrade DpFileList"));
}

// The Upgrade outcome carries the folder the dialog's preview was made for, so the run
// acts in it: a folder resolved from the settings in the click's own frame would already
// follow a settings change that landed in that frame.
#[test]
fn the_upgrade_acts_in_the_install_its_dialog_previewed() {
    let temp = sandbox("view_dpfl_switched");
    let root = temp.path();
    install(root, 2017, &pes17_list(), None);
    let ctx = tool_context_at(root, PesVersion::Pes17);
    let download = root.join("PES/download");
    let preview = upgrade::preview(&download, &ctx).unwrap();
    let (mut dialog, mut window, nodes) = dialog_window(
        preview,
        download.clone(),
        &[Vec::new(), Vec::new(), Vec::new()],
    );

    let at = centre(button(&nodes, "Upgrade"));
    window.frame(press(at), |ui| dialog.show(ui));
    let (outcome, _) = window.frame(release(at), |ui| dialog.show(ui));
    let Outcome::Upgrade(acted, delete) = outcome else {
        panic!("the Upgrade click made the upgrade's outcome")
    };
    assert_eq!(acted, download, "the preview's download, not re-resolved");
    assert!(delete.is_empty(), "no box ticked, nothing deleted");
}

// A preview describes the install it was made for: a settings change while the dialog is
// open closes it rather than letting Upgrade act on a stale plan.
#[test]
fn a_settings_change_closes_the_dialog() {
    let (_temp, other, ctx, settings) =
        two_installs("view_dpfl_close_switch", &pes17_list(), &pes17_list());
    let mut tool = Tool::new();
    let mut window = Headless::new();
    open_dialog(&mut tool, &ctx, &mut window);

    set_pes_folder(&settings, &other.path().join("PES"));
    tool.tick(&ctx);
    assert!(
        tool.detail.dpfl_dialog.is_none(),
        "the settings change closed the dialog"
    );
}

// The shell may switch the install between the tool's `tick` and its `view` in the same
// frame (to a game that started running): `view` compares the settings again where it
// draws, closing the dialog and starting the check in that frame, not a `tick` later.
#[test]
fn a_settings_change_since_the_last_tick_closes_the_dialog_in_view() {
    let (_temp, other, ctx, settings) =
        two_installs("view_dpfl_close_in_view", &pes17_list(), &pes17_list());
    let mut tool = Tool::new();
    let mut window = Headless::new();
    open_dialog(&mut tool, &ctx, &mut window);

    set_pes_folder(&settings, &other.path().join("PES"));
    window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(
        tool.detail.dpfl_dialog.is_none(),
        "the settings change since the last tick closed the dialog in the drawn frame"
    );
    assert!(tool.live_check.is_running(), "and a check runs from it");
    finish_check(&mut tool, &ctx);
}

#[test]
fn the_button_is_hidden_while_a_run_is_in_flight() {
    let ctx = settings_only_context();
    let (release_run, until) = unbounded();
    let mut log = RunLog::in_flight_dpfl(until);
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let mut window = Headless::new();
    let (_, nodes) = window.frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    assert!(
        !labeled(&nodes, "Upgrade DpFileList"),
        "hidden while running"
    );

    release_run.send(()).unwrap();
    poll_until_done(|| {
        log.poll();
        log.is_running()
    });
    let (_, nodes) = window.frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    assert!(labeled(&nodes, "Upgrade DpFileList"), "back once it ends");
}

/// The non-empty strokes of the rect shapes painted exactly over `bounds` (within half a
/// point), in paint order: a widget's own frame and any edge painted over it.
fn strokes_over(
    shapes: &[egui::epaint::ClippedShape],
    bounds: accesskit::Rect,
) -> Vec<egui::Stroke> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if !shape.stroke.is_empty()
                    && (f64::from(shape.rect.min.x) - bounds.x0).abs() <= 0.5
                    && (f64::from(shape.rect.min.y) - bounds.y0).abs() <= 0.5
                    && (f64::from(shape.rect.max.x) - bounds.x1).abs() <= 0.5
                    && (f64::from(shape.rect.max.y) - bounds.y1).abs() <= 0.5 =>
            {
                Some(shape.stroke)
            }
            _ => None,
        })
        .collect()
}

/// Whether a rect of `fill` is painted over the centre of `bounds`.
fn fills_over(
    shapes: &[egui::epaint::ClippedShape],
    bounds: accesskit::Rect,
    fill: egui::Color32,
) -> bool {
    let centre = egui::pos2(
        ((bounds.x0 + bounds.x1) / 2.0) as f32,
        ((bounds.y0 + bounds.y1) / 2.0) as f32,
    );
    shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::Rect(shape) => shape.fill == fill && shape.rect.contains(centre),
        _ => false,
    })
}

/// A log whose run has ended over a `dpfl` left `Outdated`: the Upgrade DpFileList button
/// the toolbar draws for it.
fn ended_dpfl_log() -> RunLog {
    let (release_run, until) = unbounded();
    let mut log = RunLog::in_flight_dpfl(until);
    release_run.send(()).unwrap();
    poll_until_done(|| {
        log.poll();
        log.is_running()
    });
    log
}

// TC-GUI-33
#[test]
fn the_upgrade_button_is_filled_blue_with_its_edge() {
    let ctx = settings_only_context();
    let log = ended_dpfl_log();
    assert!(
        log.dpfl_found() && log.dpfl_outdated() && !log.is_running(),
        "premise: the button shows"
    );
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let mut window = Headless::new();
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let blue = visuals.selection.bg_fill;
    let edge = studio_core::theme::edge_color(&visuals, blue);
    window.frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    // egui styles a widget from its state on the frame before.
    let (_, nodes) = window.frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    let upgrade = button(&nodes, "Upgrade DpFileList");
    let bounds = upgrade.bounds().expect("a widget has bounds");
    assert!(
        fills_over(&window.shapes, bounds, blue),
        "the button's fill is the selection blue"
    );
    assert_eq!(
        strokes_over(&window.shapes, bounds).last(),
        Some(&egui::Stroke::new(1.0, edge)),
        "the button's edge is the selection blue's own at rest"
    );

    // Beside Compile: its right edge at or just left of Compile's, the two on one row.
    let compile = button(&nodes, "Compile")
        .bounds()
        .expect("a widget has bounds");
    let gap = compile.x0 - bounds.x1;
    assert!(
        (-0.5..=8.5).contains(&gap),
        "the button ends at or within an item spacing of Compile's left edge: {gap}"
    );
    assert!(
        bounds.y0 < compile.y1 && bounds.y1 > compile.y0,
        "the buttons' rows overlap: {bounds:?} vs {compile:?}"
    );
}

// TC-GUI-33
#[test]
fn a_preview_with_nothing_to_do_opens_no_dialog_and_the_button_goes() {
    let temp = sandbox("view_dpfl_up_to_date");
    let root = temp.path();
    install(
        root,
        2021,
        crate::templates::Templates::embedded().official_list_file(),
        None,
    );
    let ctx = tool_context(root, "");
    let mut tool = Tool::new();
    // The list is the official one and every CPK it names is there: the button can only
    // still be up from a state a check has not replaced yet, which the in-flight check
    // stands in for (a real first check would clear it before the click).
    tool.run_log = RunLog::dpfl_other();
    let (release_check, check_until) = unbounded();
    tool.live_check = LiveCheck::in_flight(check_until, Vec::new());
    let mut window = Headless::new();
    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(
        labeled(&nodes, "Upgrade DpFileList"),
        "premise: the button shows"
    );

    click_labeled(&mut window, &nodes, "Upgrade DpFileList", &mut tool, &ctx);
    assert!(
        tool.detail.dpfl_dialog.is_none(),
        "a preview with nothing to do opens no dialog"
    );
    assert!(tool.run_log.is_running(), "the upgrade runs at once");
    poll_until_done(|| {
        tool.tick(&ctx);
        tool.run_log.is_running()
    });
    let texts = tool.run_log.texts();
    assert!(
        texts
            .iter()
            .any(|line| line.contains("dpfilelist_up_to_date")),
        "the up-to-date line: {texts:?}"
    );

    // The run's end starts a check: once it ends, the button is gone.
    release_check.send(()).unwrap();
    finish_check(&mut tool, &ctx);
    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(
        !labeled(&nodes, "Upgrade DpFileList"),
        "the button is gone once the check finds the list official"
    );
}

#[test]
fn no_button_after_a_run_whose_preflight_found_nothing() {
    let temp = sandbox("view_dpfl_clean");
    let root = temp.path();
    install_pes(root);
    let ctx = tool_context(root, "");
    let mut tool = Tool::new();
    let mut window = Headless::new();
    window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    finish_check(&mut tool, &ctx);
    let nodes = window.frame(Vec::new(), |ui| tool.view(ui, &ctx)).1;
    click_labeled(&mut window, &nodes, "Compile", &mut tool, &ctx);
    poll_until_done(|| {
        tool.tick(&ctx);
        tool.run_log.is_running()
    });
    let texts = tool.run_log.texts();
    assert!(
        !texts.iter().any(|line| line.contains("dpfilelist")),
        "premise: the preflight found nothing: {texts:?}"
    );
    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(!labeled(&nodes, "Upgrade DpFileList"));
}

#[test]
fn a_preview_that_cannot_be_made_is_a_log_line_and_no_dialog() {
    let temp = sandbox("view_dpfl_no_preview");
    let root = temp.path();
    install(root, 2017, &pes17_list(), None);
    let ctx = tool_context_at(root, PesVersion::Pes17);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = first_check(&mut tool, &ctx, &mut window);
    // A `DpFileList.bin` that cannot be read (a folder in its place) breaks the preview.
    fs::remove_file(root.join("PES/download/DpFileList.bin")).unwrap();
    fs::create_dir(root.join("PES/download/DpFileList.bin")).unwrap();

    let nodes = click_labeled(&mut window, &nodes, "Upgrade DpFileList", &mut tool, &ctx);
    assert!(tool.detail.dpfl_dialog.is_none(), "no dialog opens");
    let last = tool.run_log.lines().last().expect("a refusal's line");
    assert!(
        last.severity == Severity::Error && last.text.starts_with("error: "),
        "a line as a refused run's: {last:?}"
    );
    assert!(
        last.text.contains("DpFileList.bin"),
        "naming the file: {last:?}"
    );
    assert!(labeled(&nodes, "Upgrade DpFileList"), "the button stays");
}

/// A `DpflDialog` over `preview` acting on `download` drawn in a `Headless`, each of
/// `frames`' event lists one frame (a fresh modal's first is a sizing pass): the dialog,
/// the window and the last frame's nodes, for a test that acts past the setup.
fn dialog_window(
    preview: Preview,
    download: PathBuf,
    frames: &[Vec<egui::Event>],
) -> (DpflDialog, Headless, Vec<accesskit::Node>) {
    let mut dialog = DpflDialog::new(preview, download);
    let mut window = Headless::new();
    let mut nodes = Vec::new();
    for events in frames {
        nodes = window.frame(events.clone(), |ui| dialog.show(ui)).1;
    }
    (dialog, window, nodes)
}

/// A `DpflDialog` over `preview`, drawn for three frames (a fresh modal's first is a
/// sizing pass): the last frame's nodes.
fn dialog_nodes(preview: Preview) -> Vec<accesskit::Node> {
    dialog_window(
        preview,
        PathBuf::from("download"),
        &[Vec::new(), Vec::new(), Vec::new()],
    )
    .2
}

/// The wheel turned down `delta` points at `at`: the pointer there, so the scroll goes to
/// what is under it, then the wheel.
fn wheel_down(at: egui::Pos2, delta: f32) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(at),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -delta),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

#[test]
fn a_part_whose_count_is_zero_is_left_out_and_a_dropped_entry_without_a_file_has_no_box() {
    let preview = Preview {
        renames: vec![("4cc_38_balls.cpk".to_owned(), "4cc_16_balls.cpk".to_owned())],
        placeholders: 0,
        dropped: vec![
            ("4cc_60_midcup.cpk".to_owned(), Some(6272)),
            ("4cc_90_test.cpk".to_owned(), None),
        ],
        up_to_date: false,
        replaces_list: true,
    };
    let nodes = dialog_nodes(preview);
    assert!(shows_text(
        &nodes,
        "Renamed to its official name, so the game keeps loading it (1):"
    ));
    assert!(
        !nodes.iter().any(|node| node
            .value()
            .is_some_and(|text| text.contains("placeholder"))),
        "no placeholders sentence at all"
    );
    // The dropped entry with a file is a check box; the one without is its name alone.
    let boxes: Vec<&str> = nodes
        .iter()
        .filter(|node| node.role() == accesskit::Role::CheckBox)
        .filter_map(accesskit::Node::label)
        .collect();
    assert_eq!(boxes, ["4cc_60_midcup.cpk (6.1 KiB)"]);
    assert!(shows_text(&nodes, "4cc_90_test.cpk"), "the name alone");
}

// The "Upgrade replaces" paragraph shows only when the preview replaces the list: over an
// install already on the official list, one CPK's file missing, it is left out.
#[test]
fn the_first_paragraph_shows_only_when_the_list_is_replaced() {
    let temp = crate::testing::scratch("view_dpfl_same_list");
    let root = temp.path();
    install(
        root,
        2021,
        crate::templates::Templates::embedded().official_list_file(),
        Some("4cc_16_balls.cpk"),
    );
    let ctx = tool_context(root, "");
    let preview = upgrade::preview(&root.join("PES/download"), &ctx).unwrap();
    assert!(
        !preview.replaces_list,
        "premise: the list is the official one"
    );

    let nodes = dialog_nodes(preview);

    assert!(
        !nodes.iter().any(|node| node
            .value()
            .is_some_and(|text| text.contains("DpFileList.bin.bak"))),
        "no replacement paragraph"
    );
    assert!(shows_text(
        &nodes,
        "An empty placeholder is written for the official CPK with no file, because the \
         game loads nothing from download/ while a CPK its list names is missing."
    ));
}

// The tick sentence is asked over the delete boxes, not the dropped entries: one box is
// "the box", two or more "any", while the count names every dropped entry.
#[test]
fn the_dropped_sentence_is_asked_over_the_boxes() {
    let cpk = |name: &str, size: Option<u64>| (name.to_owned(), size);
    for (dropped, expected) in [
        (
            vec![
                cpk("4cc_60_midcup.cpk", Some(6272)),
                cpk("4cc_90_test.cpk", None),
            ],
            "No longer loaded by the game (2). Tick the box to have it deleted:",
        ),
        (
            vec![
                cpk("4cc_60_midcup.cpk", Some(6272)),
                cpk("4cc_76_midcup.cpk", Some(6272)),
                cpk("4cc_90_test.cpk", None),
            ],
            "No longer loaded by the game (3). Tick any you want deleted:",
        ),
    ] {
        let nodes = dialog_nodes(Preview {
            renames: Vec::new(),
            placeholders: 0,
            dropped,
            up_to_date: false,
            replaces_list: true,
        });
        assert!(shows_text(&nodes, expected), "{expected}");
    }
}

// With no dropped entry's file in `download/` there is no box to tick, so the sentence
// over the list asks for none.
#[test]
fn the_dropped_sentence_asks_for_a_tick_only_over_boxes() {
    let preview = Preview {
        renames: Vec::new(),
        placeholders: 0,
        dropped: vec![
            ("4cc_60_midcup.cpk".to_owned(), None),
            ("4cc_90_test.cpk".to_owned(), None),
        ],
        up_to_date: false,
        replaces_list: true,
    };
    let nodes = dialog_nodes(preview);
    assert!(shows_text(&nodes, "No longer loaded by the game (2):"));
    assert!(
        !nodes
            .iter()
            .any(|node| { node.value().is_some_and(|text| text.contains("Tick")) }),
        "no tick asked over no box"
    );
}

// One rename is renamed "to its official name"; two or more to theirs.
#[test]
fn the_renames_sentence_is_singular_for_one() {
    for (count, expected) in [
        (
            1,
            "Renamed to its official name, so the game keeps loading it (1):",
        ),
        (
            2,
            "Renamed to their official names, so the game keeps loading them (2):",
        ),
    ] {
        let nodes = dialog_nodes(Preview {
            renames: (1..=count)
                .map(|n| (format!("name_{n}.cpk"), format!("official_{n}.cpk")))
                .collect(),
            placeholders: 0,
            dropped: Vec::new(),
            up_to_date: false,
            replaces_list: true,
        });
        assert!(shows_text(&nodes, expected), "{count}");
    }
}

// One placeholder is "the official CPK with no file"; two or more are counted.
#[test]
fn the_placeholders_sentence_is_singular_for_one() {
    for (count, expected) in [
        (
            1,
            "An empty placeholder is written for the official CPK with no file, because \
             the game loads nothing from download/ while a CPK its list names is missing.",
        ),
        (
            2,
            "An empty placeholder is written for each of the 2 official CPKs with no \
             file, because the game loads nothing from download/ while a CPK its list \
             names is missing.",
        ),
    ] {
        let nodes = dialog_nodes(Preview {
            renames: Vec::new(),
            placeholders: count,
            dropped: Vec::new(),
            up_to_date: false,
            replaces_list: true,
        });
        assert!(shows_text(&nodes, expected), "{count}");
    }
}

// With the first paragraph left out, the first part shown takes its place: no sentence gap
// above it.
#[test]
fn the_first_part_shown_sits_where_the_first_paragraph_would() {
    // How far under the dialog's title the first text starting with `start` sits.
    let below_title = |replaces_list: bool, start: &str| {
        let nodes = dialog_nodes(Preview {
            renames: Vec::new(),
            placeholders: 1,
            dropped: Vec::new(),
            up_to_date: false,
            replaces_list,
        });
        let top = |found: Option<&accesskit::Node>| {
            found
                .and_then(accesskit::Node::bounds)
                .expect("a drawn text")
                .y0
        };
        let title = top(nodes
            .iter()
            .find(|node| node.value() == Some("Upgrade DpFileList")));
        let text = top(nodes
            .iter()
            .find(|node| node.value().is_some_and(|text| text.starts_with(start))));
        text - title
    };
    assert_eq!(
        below_title(false, "An empty placeholder"),
        below_title(true, "Upgrade replaces"),
    );
}

// The dialog's content is 460 points wide (the draft's `width:460px`): the
// paragraphs wrap at that instead of widening the window to fit their lines,
// and its frame adds the modal frame's margins and stroke on top of it.
#[test]
fn the_dialogs_content_is_460_wide_and_its_frame_adds_the_modals_margins() {
    let temp = crate::testing::scratch("view_dpfl_width");
    let root = temp.path();
    install(root, 2017, &pes17_list(), None);
    let ctx = tool_context_at(root, PesVersion::Pes17);
    let preview = upgrade::preview(&root.join("PES/download"), &ctx).unwrap();
    assert!(
        preview.replaces_list
            && !preview.renames.is_empty()
            && preview.placeholders > 0
            && !preview.dropped.is_empty(),
        "premise: the old list's preview shows every part"
    );

    let (_, window, _) = dialog_window(
        preview,
        root.join("PES/download"),
        &[Vec::new(), Vec::new(), Vec::new()],
    );
    let outer = window
        .ctx
        .memory(|memory| memory.area_rect(egui::Id::unique("upgrade_dpfl")))
        .expect("the dialog's area has a rect");
    let modal = studio_core::widgets::modal_frame(&window.ctx.style_of(window.ctx.theme()));
    let expected = 460.0 + modal.total_margin().sum().x;
    assert!(
        (outer.width() - expected).abs() <= 1.0,
        "the dialog's frame is the 460 content with the modal's margins: \
         {outer:?} vs {expected}"
    );
}

// The delete list's lines draw in the monospace font, the draft's `mono` box holding
// them: each delete box's `name (size)` and a file-less entry's name alike, as the
// renames table's names draw.
#[test]
fn the_dropped_list_names_draw_in_the_monospace_font() {
    let preview = Preview {
        renames: Vec::new(),
        placeholders: 0,
        dropped: vec![
            ("4cc_60_midcup.cpk".to_owned(), Some(6272)),
            ("4cc_76_midcup.cpk".to_owned(), Some(6272)),
            ("4cc_90_test.cpk".to_owned(), None),
        ],
        up_to_date: false,
        replaces_list: true,
    };
    let (_, window, _) = dialog_window(
        preview,
        PathBuf::from("download"),
        &[Vec::new(), Vec::new(), Vec::new()],
    );

    for label in [
        "4cc_60_midcup.cpk (6.1 KiB)",
        "4cc_76_midcup.cpk (6.1 KiB)",
        "4cc_90_test.cpk",
    ] {
        let text = text_shape_in(&window.shapes, label);
        assert!(
            text.galley
                .job
                .sections
                .iter()
                .all(|section| section.format.font_id.family == egui::FontFamily::Monospace),
            "{label:?} in the monospace font: {:?}",
            text.galley
                .job
                .sections
                .iter()
                .map(|section| section.format.font_id.family.clone())
                .collect::<Vec<_>>()
        );
    }
}

// The two lists sit in `list_box`es under the dialog's one parent: each scrolls on its
// own, or the shorter dropped list would clamp the renames' offset back to the top.
#[test]
fn the_renames_list_scrolls_to_its_last_row() {
    // 13 renames overflow `LIST_HEIGHT`; 5 dropped entries fit inside it.
    let preview = Preview {
        renames: (1..=13)
            .map(|n| (format!("name_{n}.cpk"), format!("official_{n}.cpk")))
            .collect(),
        placeholders: 0,
        dropped: (1..=5)
            .map(|n| (format!("dropped_{n}.cpk"), Some(6272)))
            .collect(),
        up_to_date: false,
        replaces_list: true,
    };
    let (mut dialog, mut window, nodes) = dialog_window(
        preview,
        PathBuf::from("download"),
        &[Vec::new(), Vec::new(), Vec::new()],
    );
    // The renames list's box: the extreme-background rect the header row sits in.
    let header = cell_rect(&nodes, "Current name");
    let background = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .extreme_bg_color;
    let renames_box = window
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if shape.fill == background => Some(shape.rect),
            _ => None,
        })
        .find(|rect| rect.contains(header.center()))
        .expect("the renames list's box");
    let last = cell_rect(&nodes, "name_13.cpk");
    assert!(
        !renames_box.contains(last.center()),
        "premise: the thirteenth rename overflows the box: {last:?} vs {renames_box:?}"
    );

    // Ten wheel steps of 80 points cover the ~130 points of overflow with margin, even
    // spread over frames as egui's smooth scroll delivers them.
    let at = egui::pos2(renames_box.center().x, renames_box.bottom() - 10.0);
    let mut nodes = nodes;
    for _ in 0..10 {
        nodes = window.frame(wheel_down(at, 80.0), |ui| dialog.show(ui)).1;
    }
    let last = cell_rect(&nodes, "name_13.cpk");
    assert!(
        renames_box.contains(last.center()),
        "the last rename is drawn inside the list's box once scrolled: \
         {last:?} vs {renames_box:?}"
    );
}

/// The `Shape::Text` whose galley is exactly `text`.
fn text_shape_in<'a>(
    shapes: &'a [egui::epaint::ClippedShape],
    text: &str,
) -> &'a egui::epaint::TextShape {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => Some(shape),
            _ => None,
        })
        .next()
        .unwrap_or_else(|| panic!("no painted text {text:?}"))
}
