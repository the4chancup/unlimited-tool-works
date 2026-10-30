//! The headless shell harness: stub tools, a `StudioApp` over them and the helpers that drive
//! frames, read the AccessKit tree and click widgets. What a scenario observes is what a frame
//! draws.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{Receiver, unbounded};
use eframe::App;
use egui::accesskit::{self, Role};
use pes_version::PesVersion;

use super::StudioApp;
use crate::events::{PipelineEvent, PipelineEventEnvelope, RunId};
use crate::help::{HelpSection, HelpTarget};
use crate::settings::Settings;
use crate::status::{Notice, ToolActivity};
use crate::tool::{AppPaths, CliError, PlannedTool, StudioTool, ToolContext};
use crate::widgets::{LogLine, LogPanel, checkbox_without_text, path_row};

/// What `activity()` reports while a stub works: `(tool_label, progress, elapsed)`.
pub(super) type Activity = (&'static str, Option<(usize, usize)>, Duration);

/// What the stub's close prompt asks about while `hold_close` holds a close: unsaved
/// work offers `Discard`/`Cancel`, a run in flight `Close when it ends`/`Keep the
/// window`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ClosePrompt {
    /// `Discard` and `Cancel`, for unsaved work.
    Unsaved,
    /// `Close when it ends` and `Keep the window`, for a run in flight.
    Run,
}

/// How often the shell called a stub tool's `tick` and `view`, what its `tick` read and the
/// knobs a test turns mid-run: the blocker reason, the reported activity, a run's event
/// receiver for `tick` to drain and the table `default_settings()` reports.
#[derive(Default)]
pub(super) struct Calls {
    pub(super) ticks: AtomicUsize,
    pub(super) views: AtomicUsize,
    pub(super) version_seen: Mutex<Option<PesVersion>>,
    /// The `version_change_blocker` reason while `Some`; a test clears it mid-run.
    pub(super) blocker: Mutex<Option<&'static str>>,
    /// What `activity()` reports: `(tool_label, progress, elapsed)`, `None` while idle.
    pub(super) activity: Mutex<Option<Activity>>,
    /// What `title_detail()` reports: the open file's path, `None` while the stub has
    /// none (the default).
    pub(super) title_detail: Mutex<Option<String>>,
    /// A run's event receiver: `tick` drains it and any envelope ends the activity, as a
    /// run's last event does.
    pub(super) events: Mutex<Option<Receiver<PipelineEventEnvelope>>>,
    /// The table `default_settings()` reports; `stub()` fills a non-empty one.
    pub(super) default_settings: Mutex<toml::Table>,
    /// While `Some`, the stub's settings section draws a path row, the
    /// string being the setting the row shows (a commit writes it back).
    pub(super) file_row: Mutex<Option<String>>,
    /// Whether the stub reports unsaved work: `hold_close` holds the close for it.
    pub(super) unsaved: Mutex<bool>,
    /// What `has_settings_view` reports: `stub()` sets it, the stub drawing a
    /// settings body; a test clears it for a tool with no form.
    pub(super) has_settings_view: Mutex<bool>,
    /// The prompt `view` draws: written by every `hold_close` ask (`None` when nothing holds
    /// any more) and cleared by an answer.
    pub(super) close_prompt: Mutex<Option<ClosePrompt>>,
    /// Set by a "Close when it ends" click: `tick` sends `request_close` when the run's
    /// last event lands.
    pub(super) close_at_run_end: Mutex<bool>,
    /// How often the shell asked `hold_close`: the asks stop at the first tool that holds.
    pub(super) holds: AtomicUsize,
    /// The `Ui::max_rect` the last `view` ran against: where the tool's view sits
    /// and how wide it is (the sidebar's overlay must never move it).
    pub(super) view_rect: Mutex<Option<egui::Rect>>,
    /// What `standing_notice` returns; `standing_notice_ended` clears it, as a real tool
    /// stops returning its notice once told.
    pub(super) standing: Mutex<Option<Notice>>,
    /// How often the shell called `standing_notice_ended`.
    pub(super) standing_ends: AtomicUsize,
}

impl Calls {
    pub(super) fn ticks(&self) -> usize {
        self.ticks.load(Ordering::SeqCst)
    }
    pub(super) fn views(&self) -> usize {
        self.views.load(Ordering::SeqCst)
    }
    pub(super) fn version_seen(&self) -> Option<PesVersion> {
        *self.version_seen.lock().unwrap()
    }
    /// What the stub reports from `activity()`.
    pub(super) fn set_activity(
        &self,
        tool_label: &'static str,
        progress: Option<(usize, usize)>,
        elapsed: Duration,
    ) {
        *self.activity.lock().unwrap() = Some((tool_label, progress, elapsed));
    }
    /// Ends the reported activity, as a finished run does.
    pub(super) fn clear_activity(&self) {
        *self.activity.lock().unwrap() = None;
    }
    /// How often `hold_close` ran.
    pub(super) fn holds(&self) -> usize {
        self.holds.load(Ordering::SeqCst)
    }
    /// The rect the last `view` was given.
    pub(super) fn view_rect(&self) -> Option<egui::Rect> {
        *self.view_rect.lock().unwrap()
    }
    /// How often the stub was told its standing notice ended.
    pub(super) fn standing_ends(&self) -> usize {
        self.standing_ends.load(Ordering::SeqCst)
    }
}

pub(super) struct StubTool {
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    /// The chapter `help()` reports; `stub` fills an empty one titled by `label`,
    /// `stub_help` the section the test spells out.
    help: HelpSection,
    /// A `LogPanel` the `view` draws over these lines, `stub_log`'s: the clicked
    /// message code goes to `ctx.open_help` (the log-to-help wiring).
    log: Option<(LogPanel, Vec<LogLine>)>,
    /// The `file_row` path row's edit buffer; `stub_parts` sets it empty.
    file_buffer: String,
    calls: Arc<Calls>,
}

impl StudioTool for StubTool {
    fn id(&self) -> &'static str {
        self.id
    }
    fn label(&self) -> &'static str {
        self.label
    }
    fn icon(&self) -> &'static str {
        self.icon
    }
    fn view(&mut self, ui: &mut egui::Ui, ctx: &ToolContext) {
        self.calls.views.fetch_add(1, Ordering::SeqCst);
        *self.calls.view_rect.lock().unwrap() = Some(ui.max_rect());
        // Bound first: a `match` scrutinee's guard would live through the arms, which
        // lock the same mutex to answer.
        let prompt = *self.calls.close_prompt.lock().unwrap();
        match prompt {
            Some(ClosePrompt::Unsaved) => {
                if ui.button("Discard").clicked() {
                    *self.calls.unsaved.lock().unwrap() = false;
                    *self.calls.close_prompt.lock().unwrap() = None;
                    ctx.request_close();
                }
                if ui.button("Cancel").clicked() {
                    *self.calls.close_prompt.lock().unwrap() = None;
                }
            }
            Some(ClosePrompt::Run) => {
                if ui.button("Close when it ends").clicked() {
                    *self.calls.close_at_run_end.lock().unwrap() = true;
                    *self.calls.close_prompt.lock().unwrap() = None;
                }
                // The latest answer wins: a member who first chose to close once the run
                // ends, then pressed X again and kept the window, keeps it.
                if ui.button("Keep the window").clicked() {
                    *self.calls.close_at_run_end.lock().unwrap() = false;
                    *self.calls.close_prompt.lock().unwrap() = None;
                }
            }
            None => {}
        }
        if let Some((panel, lines)) = &mut self.log
            && let Some(code) = panel.show(ui, lines)
        {
            ctx.open_help(HelpTarget::message(&code));
        }
    }
    fn tick(&mut self, ctx: &ToolContext) {
        *self.calls.version_seen.lock().unwrap() = Some(ctx.common().pes_version);
        self.calls.ticks.fetch_add(1, Ordering::SeqCst);
        // A run's receiver, when the test gave the stub one: any envelope ends the
        // activity, as a run's last event does — and sends on the close the member
        // asked for once the run ends.
        let events = self.calls.events.lock().unwrap();
        if let Some(rx) = events.as_ref()
            && rx.try_recv().is_ok()
        {
            while rx.try_recv().is_ok() {}
            self.calls.clear_activity();
            if std::mem::replace(&mut *self.calls.close_at_run_end.lock().unwrap(), false) {
                ctx.request_close();
            }
        }
    }
    fn settings_view(&mut self, ui: &mut egui::Ui, _ctx: &ToolContext) {
        ui.label(format!("{} settings body", self.label));
        // The `file_row` knob draws a tool's settings section: a path
        // row and a long-labelled checkbox row, in their own grid, so the
        // section is wider than the menu's 660 points.
        if let Some(current) = self.calls.file_row.lock().unwrap().as_mut() {
            egui::Grid::new(("stub-file-row", self.id))
                .num_columns(2)
                .show(ui, |ui| {
                    path_row(
                        ui,
                        "A tool's file",
                        "tool_file",
                        "A tool's file path row.",
                        &mut self.file_buffer,
                        current,
                    );
                    ui.label("Aesthetics from exports");
                    checkbox_without_text(ui, &mut false);
                    ui.end_row();
                });
        }
    }
    fn has_settings_view(&self) -> bool {
        *self.calls.has_settings_view.lock().unwrap()
    }
    fn default_settings(&self) -> toml::Table {
        self.calls.default_settings.lock().unwrap().clone()
    }
    fn help(&self) -> HelpSection {
        self.help.clone()
    }
    fn cli_command(&self) -> clap::Command {
        clap::Command::new("stub")
    }
    fn cli_run(&self, _matches: &clap::ArgMatches, _ctx: &ToolContext) -> Result<u8, CliError> {
        Ok(0)
    }
    fn version_change_blocker(&self) -> Option<&str> {
        *self.calls.blocker.lock().unwrap()
    }
    fn hold_close(&mut self) -> bool {
        self.calls.holds.fetch_add(1, Ordering::SeqCst);
        // Unsaved work is asked about before a run, as a tool's own prompt would;
        // the scenarios never set both.
        let prompt = if *self.calls.unsaved.lock().unwrap() {
            Some(ClosePrompt::Unsaved)
        } else if self.calls.activity.lock().unwrap().is_some() {
            Some(ClosePrompt::Run)
        } else {
            None
        };
        *self.calls.close_prompt.lock().unwrap() = prompt;
        prompt.is_some()
    }
    fn activity(&self) -> Option<ToolActivity> {
        let (tool_label, progress, elapsed) = *self.calls.activity.lock().unwrap().as_ref()?;
        Some(ToolActivity {
            tool_label,
            progress,
            elapsed,
        })
    }
    fn title_detail(&self) -> Option<String> {
        self.calls.title_detail.lock().unwrap().clone()
    }
    fn standing_notice(&self, _ctx: &ToolContext) -> Option<Notice> {
        self.calls.standing.lock().unwrap().clone()
    }
    fn standing_notice_ended(&mut self) {
        self.calls.standing_ends.fetch_add(1, Ordering::SeqCst);
        *self.calls.standing.lock().unwrap() = None;
    }
}

/// A stub tool with its call counts shared with the test; `blocker` reports a
/// `version_change_blocker` while `Some`. Its help chapter is empty, titled `label`.
pub(super) fn stub(
    id: &'static str,
    label: &'static str,
    blocker: Option<&'static str>,
) -> (Box<dyn StudioTool>, Arc<Calls>) {
    stub_help(
        id,
        label,
        blocker,
        HelpSection {
            title: label,
            topics: Vec::new(),
            messages: Vec::new(),
        },
    )
}

/// A stub tool whose help chapter is `section` (the help window's tests).
pub(super) fn stub_help(
    id: &'static str,
    label: &'static str,
    blocker: Option<&'static str>,
    help: HelpSection,
) -> (Box<dyn StudioTool>, Arc<Calls>) {
    stub_parts(id, label, blocker, help, None)
}

/// A stub whose `view` is a `LogPanel` over `lines` (the log-to-help wiring's
/// tests): its help chapter is `help`, so its line codes are its own catalog's.
pub(super) fn stub_log(
    id: &'static str,
    label: &'static str,
    help: HelpSection,
    lines: Vec<LogLine>,
) -> (Box<dyn StudioTool>, Arc<Calls>) {
    stub_parts(id, label, None, help, Some((LogPanel::new(id), lines)))
}

fn stub_parts(
    id: &'static str,
    label: &'static str,
    blocker: Option<&'static str>,
    help: HelpSection,
    log: Option<(LogPanel, Vec<LogLine>)>,
) -> (Box<dyn StudioTool>, Arc<Calls>) {
    let calls = Arc::new(Calls::default());
    *calls.blocker.lock().unwrap() = blocker;
    *calls.default_settings.lock().unwrap() =
        toml::Table::from_iter([("enabled".to_owned(), toml::Value::Boolean(true))]);
    *calls.has_settings_view.lock().unwrap() = true;
    (
        Box::new(StubTool {
            id,
            label,
            icon: "i",
            help,
            log,
            file_buffer: String::new(),
            calls: Arc::clone(&calls),
        }),
        calls,
    )
}

/// A shell over its stub tools, the context the tools send requests through, the shared
/// settings and each stub's call counts.
pub(super) struct Shell {
    pub(super) app: StudioApp,
    pub(super) ctx: ToolContext,
    pub(super) settings: Arc<Mutex<Settings>>,
    pub(super) calls: Vec<Arc<Calls>>,
    /// The `TestDir` a `shell()` made: held here so the data folder lives as long as the
    /// `Shell`. A destructure must bind it (`_dir`), since `..` drops it at once.
    pub(super) _dir: Option<TestDir>,
}

/// The full constructor: a shell over `tools` with `settings` (written through the
/// shared handle), `planned` as the sidebar's registry, `cli_only` as the registered
/// tools drawn as planned entries, `paths` as the base folders and `config_dir` as the
/// location the first-run dialog's second choice offers. The sidebar's running-PES
/// probe is stubbed to find nothing: a poll over the developer's real process list
/// must never leak into a test.
pub(super) fn shell_in(
    settings: Settings,
    tools: Vec<Box<dyn StudioTool>>,
    planned: &'static [PlannedTool],
    cli_only: &'static [&'static str],
    paths: AppPaths,
    config_dir: Option<PathBuf>,
    calls: Vec<Arc<Calls>>,
) -> Shell {
    let (events_tx, _events_rx) = unbounded();
    let (requests_tx, requests_rx) = unbounded();
    let settings = Arc::new(Mutex::new(settings));
    let ctx = ToolContext::new(Arc::clone(&settings), paths, events_tx, requests_tx);
    let mut app = StudioApp::new(
        tools,
        planned,
        cli_only,
        ctx.clone(),
        requests_rx,
        config_dir,
    );
    app.sidebar.probe = || Vec::new();
    Shell {
        app,
        ctx,
        settings,
        calls,
        _dir: None,
    }
}

/// A shell over `tools` with `data_dir` as the settings file's folder and a placeholder
/// exe folder: tests that spell the base folders out go through `shell_in`. A `None`
/// `data_dir` opens the first-run dialog (ST-RUN-01).
pub(super) fn shell_with(
    settings: Settings,
    tools: Vec<Box<dyn StudioTool>>,
    planned: &'static [PlannedTool],
    data_dir: Option<PathBuf>,
    calls: Vec<Arc<Calls>>,
) -> Shell {
    shell_in(
        settings,
        tools,
        planned,
        &[],
        AppPaths {
            exe_dir: PathBuf::from("exe"),
            data_dir,
        },
        None,
        calls,
    )
}

/// A shell over the stub tools `a` (labeled `A`) and `b` (`B`), its data directory a
/// `TestDir` the `Shell` holds so the first-run dialog does not stand in front of the
/// views and the folder dies with the `Shell`.
pub(super) fn shell() -> Shell {
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let dir = TestDir::new("studio_core_shell");
    let mut shell = shell_with(
        test_settings(dir.path()),
        vec![a, b],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a, calls_b],
    );
    shell._dir = Some(dir);
    shell
}

/// A temp folder unique to this test process, as `settings`' `test_dir` does it.
pub(super) fn test_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{name}_{}", std::process::id()))
}

/// A temp folder the test owns: `<temp>/<name>_<pid>_<n>` (`n` from a process counter,
/// so no two calls share a folder), created in `new` and removed in `Drop` — a test
/// that panics still leaves the temp folder clean. For the tests that want a real
/// data directory: a `None` `data_dir` opens the first-run dialog (ST-RUN-01) and a
/// missing folder raises `data_dir_read_only` at the first save.
pub(super) struct TestDir(PathBuf);

impl TestDir {
    /// Creates `<temp>/<name>_<pid>_<n>`: unique in this process run.
    pub(super) fn new(name: &str) -> TestDir {
        static SEQUENCE: AtomicUsize = AtomicUsize::new(0);
        let dir = test_dir(&format!(
            "{name}_{}",
            SEQUENCE.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        TestDir(dir)
    }

    /// The folder's path.
    pub(super) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        // A drop cannot fail the test, and a leftover folder in the temp folder harms
        // nothing: the failure is logged, not raised.
        if let Err(error) = fs::remove_dir_all(&self.0) {
            log::debug!("test folder {} not removed: {error}", self.0.display());
        }
    }
}

/// The settings a shell test starts from: the defaults, except `pes_folder_path`
/// under `dir` so the exe check never lists the developer's real install
/// (CONTRIBUTING.md "Testing": a setting whose default finds something on the machine
/// is set to a path under the test's own folder).
pub(super) fn test_settings(dir: &Path) -> Settings {
    let mut settings = Settings::default();
    settings.common.pes_folder_path = dir.display().to_string();
    settings
}

/// A run's smallest envelope, for a stub run's emit.
pub(super) fn envelope() -> PipelineEventEnvelope {
    PipelineEventEnvelope {
        run_id: RunId(1),
        export_id: None,
        export_revision: None,
        event: PipelineEvent::UnmigratedContentFound,
    }
}

/// What one headless frame produced: the frame's AccessKit nodes, its platform output
/// (clipboard, opened links), the root viewport's commands (the window title), the
/// repaint delay it asked for (`Duration::MAX` when nothing did) and the shapes it
/// painted, clip rect included (for checks on what was drawn where).
pub(super) struct FrameOutput {
    pub(super) nodes: Vec<accesskit::Node>,
    pub(super) platform: egui::PlatformOutput,
    pub(super) commands: Vec<egui::ViewportCommand>,
    pub(super) repaint_delay: Duration,
    pub(super) shapes: Vec<egui::epaint::ClippedShape>,
}

/// A context with AccessKit on and the window's fonts and style
/// (`crate::theme::install`), so every shell test draws what a member
/// sees.
pub(super) fn test_context() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    crate::theme::install(&ctx);
    ctx
}

/// One headless frame of the shell on an 800 by 600 screen with `events` as its input
/// and `focused` as the window's focus: `logic` first, then `ui`, as eframe calls them
/// inside one pass, so both see this frame's input. The context must have AccessKit
/// enabled.
pub(super) fn frame(
    ctx: &egui::Context,
    app: &mut StudioApp,
    events: Vec<egui::Event>,
    focused: bool,
) -> FrameOutput {
    frame_at(ctx, app, events, focused, None)
}

/// `frame` with the clock set: `time` is `RawInput::time`, in seconds.
pub(super) fn frame_at(
    ctx: &egui::Context,
    app: &mut StudioApp,
    events: Vec<egui::Event>,
    focused: bool,
    time: Option<f64>,
) -> FrameOutput {
    run_frame(ctx, app, raw_input(events, focused, time, None))
}

/// `frame` with the window's close requested: the OS's `ViewportEvent::Close` on the
/// root viewport, as the title bar's X sends it.
pub(super) fn frame_closing(ctx: &egui::Context, app: &mut StudioApp) -> FrameOutput {
    let mut input = raw_input(Vec::new(), true, None, None);
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .expect("`RawInput::default` carries the root viewport")
        .events
        .push(egui::ViewportEvent::Close);
    run_frame(ctx, app, input)
}

/// `frame` with the OS's theme in `RawInput::system_theme`: the shell must not follow
/// it — the `theme` setting decides.
pub(super) fn frame_theme(
    ctx: &egui::Context,
    app: &mut StudioApp,
    events: Vec<egui::Event>,
    focused: bool,
    system_theme: Option<egui::Theme>,
) -> FrameOutput {
    run_frame(ctx, app, raw_input(events, focused, None, system_theme))
}

/// The `RawInput` every harness frame shares: an 800 by 600 screen. A test
/// needing another screen or a root `ViewportInfo` adjusts the result and
/// hands it to `run_frame`.
pub(super) fn raw_input(
    events: Vec<egui::Event>,
    focused: bool,
    time: Option<f64>,
    system_theme: Option<egui::Theme>,
) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 600.0),
        )),
        events,
        focused,
        time,
        system_theme,
        ..Default::default()
    }
}

/// One frame of `input`: the entry point for tests that build their own
/// `RawInput` from `raw_input`.
pub(super) fn run_frame(
    ctx: &egui::Context,
    app: &mut StudioApp,
    input: egui::RawInput,
) -> FrameOutput {
    let mut output = ctx.run_ui(input, |ui| {
        // eframe's own constructor of a `Frame` for testing an `App` without a window.
        app.logic(ui.ctx(), &mut eframe::Frame::_new_kittest());
        app.ui(ui, &mut eframe::Frame::_new_kittest());
    });
    let update = output
        .platform_output
        .accesskit_update
        .take()
        .expect("AccessKit is enabled on the context");
    let platform = std::mem::take(&mut output.platform_output);
    let viewport = output
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .expect("every frame produces the root viewport");
    let commands = viewport.commands.clone();
    let repaint_delay = viewport.repaint_delay;
    let shapes = std::mem::take(&mut output.shapes);
    // Nothing renders here, so the frame's texture deltas are dropped on purpose.
    output.drop_without_applying_deltas();
    FrameOutput {
        nodes: update.nodes.into_iter().map(|(_, node)| node).collect(),
        platform,
        commands,
        repaint_delay,
        shapes,
    }
}

/// The title strings the frame's viewport commands carry, in order.
pub(super) fn titles(output: &FrameOutput) -> Vec<String> {
    output
        .commands
        .iter()
        .filter_map(|command| match command {
            egui::ViewportCommand::Title(title) => Some(title.clone()),
            _ => None,
        })
        .collect()
}

/// Rests the pointer on `node` (a rail widget) until the sidebar's overlay has
/// had time to open, as a member's hover does: the pointer arrives at `time` and
/// stays `0.2 s` past it. Returns the last frame.
pub(super) fn open_sidebar(
    ctx: &egui::Context,
    app: &mut StudioApp,
    node: &accesskit::Node,
    time: f64,
) -> FrameOutput {
    frame_at(ctx, app, hover(node), true, Some(time));
    frame_at(ctx, app, Vec::new(), true, Some(time + 0.2))
}

/// The widget labeled `label`.
pub(super) fn entry<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| node.label() == Some(label))
        .unwrap_or_else(|| panic!("no widget labeled {label:?}"))
}

/// The centre of `node`'s bounds.
pub(super) fn centre(node: &accesskit::Node) -> egui::Pos2 {
    let bounds = node.bounds().expect("a widget has bounds");
    egui::pos2(
        ((bounds.x0 + bounds.x1) / 2.0) as f32,
        ((bounds.y0 + bounds.y1) / 2.0) as f32,
    )
}

/// The primary button pressed (or released) at the centre of `node`'s bounds, with the
/// pointer moved there first. egui reports the click on the frame of the release.
pub(super) fn pointer(node: &accesskit::Node, pressed: bool) -> Vec<egui::Event> {
    let pos = centre(node);
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        },
    ]
}

/// The pointer moved to the centre of `node` without a button: a hover.
pub(super) fn hover(node: &accesskit::Node) -> Vec<egui::Event> {
    vec![egui::Event::PointerMoved(centre(node))]
}

/// The pointer moved to the centre of `node` and one notch of a mouse wheel,
/// up or down.
pub(super) fn wheel_notch(node: &accesskit::Node, up: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(centre(node)),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, if up { 1.0 } else { -1.0 }),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: egui::Modifiers::default(),
        },
    ]
}

/// A click (press frame, release frame) on `node`, returning the release frame: the one
/// `clicked()` lands in, so the one whose platform output carries the click's commands.
pub(super) fn click(
    ctx: &egui::Context,
    app: &mut StudioApp,
    node: &accesskit::Node,
) -> FrameOutput {
    let press = pointer(node, true);
    let release = pointer(node, false);
    frame(ctx, app, press, true);
    frame(ctx, app, release, true)
}

/// The PES version selector's node.
pub(super) fn combo(nodes: &[accesskit::Node]) -> &accesskit::Node {
    nodes
        .iter()
        .find(|node| node.role() == Role::ComboBox)
        .expect("the selector is drawn")
}

/// `n` frames with no input, returning the last one's output. egui's clock advances by
/// the predicted delta each frame; ten frames clear the post-click tooltip cooldown
/// (`clicked_more_recently_than_moved`, about 100 ms).
pub(super) fn idle(ctx: &egui::Context, app: &mut StudioApp, n: usize) -> FrameOutput {
    let mut out = frame(ctx, app, Vec::new(), true);
    for _ in 1..n {
        out = frame(ctx, app, Vec::new(), true);
    }
    out
}

/// A node carries `needle` as its text: label-role nodes (plain text, tooltips, the
/// planned entries) keep theirs in `value`.
pub(super) fn has_text(nodes: &[accesskit::Node], needle: &str) -> bool {
    nodes
        .iter()
        .any(|node| node.value().is_some_and(|text| text.contains(needle)))
}

/// Whether a widget labeled `label` is present: a button's text sits in `label`,
/// not `value`.
pub(super) fn has_label(nodes: &[accesskit::Node], label: &str) -> bool {
    nodes.iter().any(|node| node.label() == Some(label))
}
