//! The view's headless-window tests: one egui context with AccessKit on, run a frame
//! at a time. The tests are `tests/`'s files, one per concern; the helpers they
//! share live here.

use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use aesthetics_export::test_exports::Requirement;
use crossbeam_channel::{Receiver, unbounded};
use egui::accesskit;
use export_grid::{Cell, IdCell, Position, RowBody, Tone, TopBox};
use pes_version::PesVersion;
use studio_core::{
    AppPaths, Disposition, ExportId, Message, MessageCode, Scope, Settings, ShellRequest,
    StudioTool,
};

use super::*;
use crate::Tool;
use crate::check::live::LiveCheck;
use crate::settings::default_table;
use crate::test_files::complete;
use crate::testing::{
    ScratchFolder, copy_tree, install_pes, poll_until_done, sandbox, tool_context,
};

/// A headless window: one egui context with AccessKit on and animations off (so a
/// collapsing header is fully open on the frame after its click), run a frame at a time.
struct Headless {
    ctx: egui::Context,
    /// The last frame's ask for the next repaint: `Duration::MAX` when nothing asked.
    repaint_delay: Duration,
    /// The last frame's painted shapes.
    shapes: Vec<egui::epaint::ClippedShape>,
    /// The clipboard: the text the latest `CopyText` command put on it, kept until
    /// another copy replaces it, as a real clipboard's contents are.
    copied: Option<String>,
}

impl Headless {
    fn new() -> Headless {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        // The window draws with these fonts and style; the test measures
        // what a member sees, whichever crates the build unifies.
        studio_core::theme::install(&ctx);
        ctx.global_style_mut(|style| style.animation_time = 0.0);
        Headless {
            ctx,
            repaint_delay: Duration::MAX,
            shapes: Vec::new(),
            copied: None,
        }
    }

    /// Runs one frame of `contents` on a 1280 by 780 screen, the window's own size, with
    /// `events` as its input; returns what `contents` returned and the frame's AccessKit
    /// nodes, and keeps its shapes in `shapes`.
    fn frame<R>(
        &mut self,
        events: Vec<egui::Event>,
        mut contents: impl FnMut(&mut egui::Ui) -> R,
    ) -> (R, Vec<accesskit::Node>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 780.0),
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
        self.repaint_delay = output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .expect("every frame produces the root viewport")
            .repaint_delay;
        for command in &output.platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command {
                self.copied = Some(text.clone());
            }
        }
        self.shapes = std::mem::take(&mut output.shapes);
        // Nothing renders here, so the frame's texture deltas are dropped on purpose.
        output.drop_without_applying_deltas();
        let nodes = update.nodes.into_iter().map(|(_, node)| node).collect();
        (result.expect("run_ui calls the closure"), nodes)
    }
}

/// A context with no sandbox behind it, for frames that start no run.
fn settings_only_context() -> ToolContext {
    tool_context(Path::new("exe"), "")
}

/// `tool_context` for a `version` install (`tool_context` itself is always PES 21's):
/// the same settings built by hand, `pes_version` aside.
fn tool_context_at(root: &Path, version: PesVersion) -> ToolContext {
    tool_context_with_settings(root, version).0
}

/// `tool_context_at`'s context, the shared settings it is built on, so a test writes
/// them through the lock as the sidebar does, and the receiving end of its shell
/// requests, so a test reads what a click asked of the shell.
fn tool_context_with_settings(
    root: &Path,
    version: PesVersion,
) -> (ToolContext, Arc<Mutex<Settings>>, Receiver<ShellRequest>) {
    let mut settings = Settings::parse("").unwrap();
    settings.common.pes_version = version;
    settings.common.pes_folder_path = root.join("PES").display().to_string();
    // Before `merge_defaults` fills `savefile_path` with `auto`: an existing key is kept.
    let mut tool = settings.tool(TOOL_ID).cloned().unwrap_or_default();
    tool.entry("savefile_path").or_insert_with(|| {
        toml::Value::String(root.join("save").join("EDIT00000000").display().to_string())
    });
    settings.set_tool(TOOL_ID, tool);
    settings.merge_defaults(TOOL_ID, &default_table());
    let (events_tx, _events_rx) = unbounded();
    let (requests_tx, requests_rx) = unbounded();
    let settings = Arc::new(Mutex::new(settings));
    let ctx = ToolContext::new(
        settings.clone(),
        AppPaths {
            exe_dir: root.to_path_buf(),
            data_dir: Some(root.join("data")),
        },
        events_tx,
        requests_tx,
    );
    (ctx, settings, requests_rx)
}

/// The shared settings' PES folder made `folder`, as the sidebar's pick writes it.
fn set_pes_folder(settings: &Arc<Mutex<Settings>>, folder: &Path) {
    settings.lock().unwrap().common.pes_folder_path = folder.display().to_string();
}

/// The scene of a PES-folder switch: `root`'s install holds `first` as its `DpFileList.bin`
/// and `other`'s `second` (each a PES 2017 install with every listed CPK in `download/`),
/// the context and its shared settings pointing at `root`'s.
fn two_installs(
    test: &str,
    first: &[u8],
    second: &[u8],
) -> (
    ScratchFolder,
    ScratchFolder,
    ToolContext,
    Arc<Mutex<Settings>>,
) {
    let temp = sandbox(test);
    install(temp.path(), 2017, first, None);
    let other = crate::testing::scratch(&format!("{test}_other"));
    install(other.path(), 2017, second, None);
    let (ctx, settings, _requests) = tool_context_with_settings(temp.path(), PesVersion::Pes17);
    (temp, other, ctx, settings)
}

/// The tracer export, whose player and goalkeeper kit the grid's exports are built from.
fn tracer() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracer/studio/egg Midcup Tracer")
}

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

/// A `PES<year>.exe` install under `root`: `download/` holding `list` as its
/// `DpFileList.bin` and `cpk_bytes` for every CPK it names but `missing` (`None` for none).
fn install(root: &Path, year: u16, list: &[u8], missing: Option<&str>) {
    let download = root.join("PES/download");
    fs::create_dir_all(&download).unwrap();
    fs::write(root.join(format!("PES/PES{year}.exe")), "the game").unwrap();
    fs::write(download.join("DpFileList.bin"), list).unwrap();
    for name in cpk::dp_file_list::read(list).unwrap() {
        if Some(name.as_str()) != missing {
            fs::write(download.join(&name), cpk_bytes(&name)).unwrap();
        }
    }
}

/// `root`'s teams list: `702 /a/`, `714 /co/` and the sandbox's `792 /egg/`.
fn list_teams(root: &Path) {
    fs::write(
        root.join("data/teams_list.txt"),
        "ID\tName\n702\t/a/\n714\t/co/\n792\t/egg/\n",
    )
    .unwrap();
}

/// The player folder `folder` of `root`'s export `export`, a copy of the tracer's player,
/// which checks clean.
fn add_player(root: &Path, export: &str, folder: &str) {
    let player = tracer().join("Players/05 - The Chad Stormworks Player");
    copy_tree(
        &player,
        &root
            .join("exports")
            .join(export)
            .join("Players")
            .join(folder),
    );
}

/// The kit folder `folder` of `root`'s export `export`, a copy of the tracer's
/// goalkeeper kit.
fn add_kit(root: &Path, export: &str, folder: &str) {
    copy_tree(
        &tracer().join("Kits/g1"),
        &root.join("exports").join(export).join("Kits").join(folder),
    );
}

/// Writes `bytes` to `path` under `root`'s exports, its folders created.
fn add_file(root: &Path, path: &str, bytes: &[u8]) {
    let path = root.join("exports").join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// The rows the tool's grid check makes of `root`'s exports.
fn checked_rows(root: &Path) -> Vec<Row> {
    let ctx = tool_context(root, "");
    let mut tool = Tool::new();
    tool.live_check.start(&ctx);
    finish_check(&mut tool, &ctx);
    tool.live_check.rows().to_vec()
}

/// The row labelled `name`.
fn row<'a>(rows: &'a [Row], name: &str) -> &'a Row {
    rows.iter()
        .find(|row| row.label.text == name)
        .unwrap_or_else(|| panic!("no row {name:?}"))
}

/// The cell at `position` of `row`'s line `line`, column group `group`.
fn position(row: &Row, line: usize, group: usize, position: usize) -> Option<&Cell> {
    let RowBody::Lines(lines) = &row.body else {
        panic!("{} has a message in place of its cells", row.label.text);
    };
    match &lines[line].groups[group][position] {
        Position::Cell(cell) => Some(cell),
        Position::Empty => None,
    }
}

/// Player `slot`'s cell: 01 to 23 on the first line, a referee's 24 to 35 on the second.
fn player(row: &Row, slot: usize) -> Option<&Cell> {
    position(row, (slot - 1) / 23, 0, (slot - 1) % 23)
}

/// The portraits' cell.
fn portraits(row: &Row) -> Option<&Cell> {
    position(row, 0, 1, 0)
}

/// The kit cell of column `column`: 0 is GK, then K1 to K9.
fn kit(row: &Row, column: usize) -> Option<&Cell> {
    position(row, 0, 2, column)
}

/// Ticks `tool` until its grid check has ended and its rows are in.
fn finish_check(tool: &mut Tool, ctx: &ToolContext) {
    poll_until_done(|| {
        tool.tick(ctx);
        tool.live_check.is_running()
    });
}

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

/// Whether some label widget shows exactly `text`. egui puts a label's text in its
/// AccessKit node's value, not its label.
fn shows_text(nodes: &[accesskit::Node], text: &str) -> bool {
    nodes.iter().any(|node| node.value() == Some(text))
}

/// Whether some widget's node is labelled `label` (a button's or a check box's).
fn labeled(nodes: &[accesskit::Node], label: &str) -> bool {
    nodes.iter().any(|node| node.label() == Some(label))
}

/// A node's bounds as an `egui::Rect`, when it has them.
fn node_rect(node: &accesskit::Node) -> Option<egui::Rect> {
    node.bounds().map(|bounds| {
        egui::Rect::from_min_max(
            egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
            egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
        )
    })
}

/// The rectangle of the painted grid cell reading `text`.
fn cell_rect(nodes: &[accesskit::Node], text: &str) -> egui::Rect {
    let found: Vec<egui::Rect> = nodes
        .iter()
        .filter(|node| node.role() == accesskit::Role::Label && node.value() == Some(text))
        .filter_map(node_rect)
        .collect();
    assert_eq!(found.len(), 1, "one cell reads {text:?}");
    found[0]
}

/// A click on `node`: the press frame, the release frame, and the frame after, which shows
/// what the click changed. Returns that frame's nodes.
fn click_on(
    window: &mut Headless,
    node: &accesskit::Node,
    mut frame: impl FnMut(&mut Headless, Vec<egui::Event>) -> Vec<accesskit::Node>,
) -> Vec<accesskit::Node> {
    let at = centre(node);
    frame(window, press(at));
    frame(window, release(at));
    frame(window, Vec::new())
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
    click_on(window, button(nodes, label), |window, events| {
        window.frame(events, |ui| tool.view(ui, ctx)).1
    })
}

/// Asserts the widget `node` shows the hover text `expected`, its enabled state aside,
/// `frame` running the frames that draw it (as `click_on`'s does): the pointer rests on
/// it until the tooltip shows, failing rather than hanging. The wait needs no real time
/// with `tooltip_delay` turned off, only the frames the pointer needs to read as still.
fn tooltip_is(
    window: &mut Headless,
    node: &accesskit::Node,
    expected: &str,
    mut frame: impl FnMut(&mut Headless, Vec<egui::Event>) -> Vec<accesskit::Node>,
) {
    window
        .ctx
        .all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);
    frame(window, vec![egui::Event::PointerMoved(centre(node))]);
    for _ in 0..30 {
        if shows_text(&frame(window, Vec::new()), expected) {
            return;
        }
    }
    panic!("no tooltip {expected:?} showed over the widget");
}

/// The view's first frames: the check the window opens with ended, and the frame after's
/// nodes, the state the check left (the log's lines, the toolbar's buttons).
fn first_check(tool: &mut Tool, ctx: &ToolContext, window: &mut Headless) -> Vec<accesskit::Node> {
    // The first frame starts the grid's check.
    window.frame(Vec::new(), |ui| tool.view(ui, ctx));
    finish_check(tool, ctx);
    window.frame(Vec::new(), |ui| tool.view(ui, ctx)).1
}

/// A finding of the Compiler's `code` at `severity`, about the whole export.
fn finding(code: &'static str, severity: Severity) -> Message {
    Message {
        code: MessageCode {
            tool_id: TOOL_ID,
            code: Cow::Borrowed(code),
        },
        severity,
        disposition: Disposition::Keep,
        scope: Scope::Export {
            export_id: ExportId(1),
        },
        context: Vec::new(),
    }
}

/// A row a test check hands back: /egg/'s label alone.
fn label_only_row() -> Row {
    Row {
        key: "egg Midcup Tracer".to_owned(),
        label: Cell::new("/egg/", CellState::FullOk),
        id: Some(IdCell::Known("792".to_owned())),
        body: RowBody::Lines(Vec::new()),
        log_lines: Vec::new(),
    }
}

mod compile;
mod compile_menu;
mod detail;
mod grid;
mod live_check;
mod log_panel;
mod pictures;
mod toolbar;
mod upgrade_dpfl;
