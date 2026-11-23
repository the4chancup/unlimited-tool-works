//! The Upgrade DpFileList button and its dialog (TC-GUI-33): the button the last run's
//! preflight findings bring beside Compile, the dialog its click opens on the upgrade's
//! preview, and the upgrade run the dialog's Upgrade starts.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::upgrade::{self, Preview};

/// A test CPK's bytes: `name`'s own bytes cycled to the placeholder's 6,272 (`size_text`'s
/// "6.1 KiB"), so a renamed file is told from the file that took its name.
fn cpk_bytes(name: &str) -> Vec<u8> {
    name.bytes().cycle().take(6272).collect()
}

/// The PES 2017 install's `DpFileList.bin` (`examples/DpFileList.bin`), the old
/// faces/uniform list.
fn pes17_list() -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/DpFileList.bin"))
        .unwrap()
}

/// A PES 2017 install, TC-GUI-33's: `PES2017.exe` and `download/` holding the old list,
/// `cpk_bytes` for every CPK it names.
fn install_pes17(root: &Path) {
    let download = root.join("PES/download");
    fs::create_dir_all(&download).unwrap();
    fs::write(root.join("PES/PES2017.exe"), "the game").unwrap();
    let list = pes17_list();
    fs::write(download.join("DpFileList.bin"), &list).unwrap();
    for name in cpk::dp_file_list::read(&list).unwrap() {
        fs::write(download.join(&name), cpk_bytes(&name)).unwrap();
    }
}

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

/// A click on the node `nodes` holds labelled `label`, on `tool`'s view: press, release,
/// then the frame after, whose nodes it returns.
fn click_labeled(
    window: &mut Headless,
    nodes: &[accesskit::Node],
    label: &str,
    tool: &mut Tool,
    ctx: &ToolContext,
) -> Vec<accesskit::Node> {
    let at = centre(button(nodes, label));
    window.frame(press(at), |ui| tool.view(ui, ctx));
    window.frame(release(at), |ui| tool.view(ui, ctx));
    window.frame(Vec::new(), |ui| tool.view(ui, ctx)).1
}

/// Whether some widget's node is labelled `label` (a button's or a check box's).
fn labeled(nodes: &[accesskit::Node], label: &str) -> bool {
    nodes.iter().any(|node| node.label() == Some(label))
}

/// `root`'s PES 17 install compiled once: the run stopped by its outdated list, and a
/// frame's nodes with the Upgrade DpFileList button up.
fn compile_stops(
    root: &Path,
    tool: &mut Tool,
    ctx: &ToolContext,
    window: &mut Headless,
) -> Vec<accesskit::Node> {
    // The first frame starts the grid's check.
    window.frame(Vec::new(), |ui| tool.view(ui, ctx));
    finish_check(tool, ctx);
    let nodes = window.frame(Vec::new(), |ui| tool.view(ui, ctx)).1;
    click_labeled(window, &nodes, "Compile", tool, ctx);
    assert!(tool.run_log.is_running(), "the click started the run");
    poll_until_done(|| {
        tool.tick(ctx);
        tool.run_log.is_running()
    });
    let texts = tool.run_log.texts();
    assert!(
        texts
            .iter()
            .any(|line| line.contains("dpfilelist_outdated")),
        "premise: the run stopped at {}'s outdated list: {texts:?}",
        root.display()
    );
    window.frame(Vec::new(), |ui| tool.view(ui, ctx)).1
}

/// The TC-GUI-33 scene to the open dialog: `compile_stops`, then the button's click, and
/// the last frame's nodes (a fresh modal's first frame is a sizing pass, the second
/// draws it).
fn open_dialog(
    root: &Path,
    tool: &mut Tool,
    ctx: &ToolContext,
    window: &mut Headless,
) -> Vec<accesskit::Node> {
    let nodes = compile_stops(root, tool, ctx, window);
    click_labeled(window, &nodes, "Upgrade DpFileList", tool, ctx);
    assert!(
        tool.detail.dpfl_dialog.is_some(),
        "the click opened the dialog"
    );
    // A fresh modal's first frames are a sizing pass; the dialog's widgets show after.
    window.frame(Vec::new(), |ui| tool.view(ui, ctx));
    window.frame(Vec::new(), |ui| tool.view(ui, ctx)).1
}

// TC-GUI-33
#[test]
fn the_outdated_list_brings_the_button_whose_dialog_upgrades_it() {
    let temp = sandbox("view_dpfl_upgrade");
    let root = temp.path();
    install_pes17(root);
    let ctx = tool_context_pes17(root);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    open_dialog(root, &mut tool, &ctx, &mut window);
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
    assert!(labeled(&nodes, "4cc_60_midcup.cpk (6.1 KiB)"));
    assert!(labeled(&nodes, "Cancel") && labeled(&nodes, "Upgrade"));

    // Upgrade: `download/` holds what `upgrade-dpfl --yes` leaves, its lines appended to
    // the log, and the button gone.
    let expected = upgrade_leaves(&download, &BTreeSet::new(), &ctx);
    click_labeled(&mut window, &nodes, "Upgrade", &mut tool, &ctx);
    poll_until_done(|| {
        tool.tick(&ctx);
        tool.run_log.is_running()
    });
    assert_eq!(snapshot(&download), expected);
    let texts = tool.run_log.texts();
    for line in [
        "dpfilelist_cpk_renamed (from=4cc_40_faces.cpk, to=4cc_41_teams.cpk)",
        "dpfilelist_replaced",
        "dpfilelist_cpk_dropped (cpk=4cc_60_midcup.cpk",
        "Run finished with exit code 0",
    ] {
        assert!(
            texts.iter().any(|text| text.contains(line)),
            "the log holds {line:?}: {texts:?}"
        );
    }
    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(
        !labeled(&nodes, "Upgrade DpFileList"),
        "the button is gone once the list is replaced"
    );

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
}

// TC-GUI-33
#[test]
fn a_ticked_delete_box_removes_only_that_cpk() {
    let temp = sandbox("view_dpfl_tick");
    let root = temp.path();
    install_pes17(root);
    let ctx = tool_context_pes17(root);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = open_dialog(root, &mut tool, &ctx, &mut window);
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
fn the_dialog_s_cancel_writes_nothing() {
    let temp = sandbox("view_dpfl_cancel");
    let root = temp.path();
    install_pes17(root);
    let ctx = tool_context_pes17(root);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = open_dialog(root, &mut tool, &ctx, &mut window);
    let download = root.join("PES/download");
    let before = snapshot(&download);

    let nodes = click_labeled(&mut window, &nodes, "Cancel", &mut tool, &ctx);
    assert!(
        tool.detail.dpfl_dialog.is_none(),
        "Cancel closed the dialog"
    );
    assert_eq!(snapshot(&download), before, "nothing written");
    // The button is back, the list still not the official one.
    assert!(labeled(&nodes, "Upgrade DpFileList"));
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
    install_pes17(root);
    let ctx = tool_context_pes17(root);
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = compile_stops(root, &mut tool, &ctx, &mut window);
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

/// A `DpflDialog` over `preview`, drawn for three frames (a fresh modal's first is a
/// sizing pass): the last frame's nodes.
fn dialog_nodes(preview: Preview) -> Vec<accesskit::Node> {
    let mut dialog = DpflDialog::new(preview);
    let mut window = Headless::new();
    let mut nodes = Vec::new();
    for _ in 0..3 {
        nodes = window.frame(Vec::new(), |ui| dialog.show(ui)).1;
    }
    nodes
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
        "Renamed to their official names, so the game keeps loading them (1):"
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
    let temp = sandbox("view_dpfl_same_list");
    let root = temp.path();
    let download = root.join("PES/download");
    fs::create_dir_all(&download).unwrap();
    let templates = crate::templates::Templates::embedded();
    fs::write(
        download.join("DpFileList.bin"),
        templates.official_list_file(),
    )
    .unwrap();
    for name in templates.official_list() {
        if name != "4cc_16_balls.cpk" {
            fs::write(download.join(&name), cpk_bytes(&name)).unwrap();
        }
    }
    let ctx = tool_context(root, "");
    let preview = upgrade::preview(&download, &ctx).unwrap();
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
        "An empty placeholder is written for each of the 1 official CPKs with no file, \
         because the game loads nothing from download/ while a CPK its list names is missing."
    ));
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
