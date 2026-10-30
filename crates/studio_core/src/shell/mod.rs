//! The app shell: `StudioApp`, the window with a sidebar listing the registered tools, the
//! status bar under everything, the window title, the settings menu, the help window, the
//! first-run data-location dialog while no data location exists, the active tool's view,
//! every tool ticked each frame and the tools' shell requests applied; the window's close
//! asks the tools in sidebar order until one holds it. Plus the launch modes' argument
//! parsing (`launch`). The common widgets live in `crate::widgets`.

mod about;
mod first_run;
mod help_window;
mod launch;
mod settings_menu;
mod sidebar;
mod status_bar;
#[cfg(test)]
mod test_harness;
mod title;
mod window_size;

use std::path::PathBuf;

use crossbeam_channel::Receiver;
use log::debug;
#[cfg(not(target_arch = "wasm32"))]
use log::warn;

pub use launch::{Launch, LaunchMode, parse_launch, run_cli};

use crate::help::{self, HelpTarget};
use crate::settings::{Theme, WindowPlace};
use crate::status::ActionEffect;
#[cfg(not(target_arch = "wasm32"))]
use crate::status::ShellCondition;
use crate::tool::{PlannedTool, ShellRequest, StudioTool, ToolContext};

use self::first_run::FirstRun;
use self::help_window::HelpWindow;
use self::settings_menu::SettingsMenu;
use self::sidebar::{Sidebar, SidebarClick};
use self::status_bar::StatusBar;
use self::title::WindowTitle;

/// The executable's name: `UTW` on Windows, `utw` elsewhere, the spelling the CLI's usage
/// lines print. It is a literal so a `&'static str` table can `concat!` it; `EXE_NAME` is
/// the same name for code that formats at run time.
#[cfg(windows)]
#[macro_export]
macro_rules! exe_name {
    () => {
        "UTW"
    };
}

/// The executable's name: `UTW` on Windows, `utw` elsewhere, the spelling the CLI's usage
/// lines print. It is a literal so a `&'static str` table can `concat!` it; `EXE_NAME` is
/// the same name for code that formats at run time.
#[cfg(not(windows))]
#[macro_export]
macro_rules! exe_name {
    () => {
        "utw"
    };
}

/// The display name, the one the window title, the About box and the log's first line
/// show. A literal for `concat!`, like `exe_name!`.
#[macro_export]
macro_rules! app_name {
    () => {
        "Unlimited Tool Works"
    };
}

/// The executable's name: `exe_name!()` where a `concat!` needs the literal.
pub const EXE_NAME: &str = exe_name!();
/// The display name: `app_name!()` where a `concat!` needs the literal (the tools' catalogs
/// use the macro, so the constant serves the shell alone).
pub(crate) const APP_NAME: &str = app_name!();
/// The `Tvers` field of every CPK the suite writes (the Team compiler's output CPK and the
/// pre-Fox face CPKs nested in it, the Save editor's bins CPK): the display name and the
/// version, naming the tool that wrote the file. One string per release, so a release
/// compiling the same exports writes the same bytes.
pub const TOOL_VERSION: &str = concat!(app_name!(), " ", env!("CARGO_PKG_VERSION"));

/// The version line the shell shows in four places: the status bar's left slot, the About
/// block's first line, `UTW --version` and the window's log's first line. It reads
/// `v{version}`, the release stage while there is one (`pre-alpha` for 0.0.x,
/// `alpha` for 0.y.z, nothing from 1.0.0), then ` (debug)` for a debug build.
pub fn version_label() -> String {
    version_label_of(env!("CARGO_PKG_VERSION"), cfg!(debug_assertions))
}

/// `version_label` over a given version string and build flag: the stage comes from the
/// version's major and minor, so a bump can never leave a stale word. A version that
/// does not parse is a build error the workspace cannot have, so `expect`, not a
/// fallback.
fn version_label_of(version: &str, debug: bool) -> String {
    let mut parts = version.split('.');
    let number = |part: Option<&str>| {
        part.and_then(|part| part.parse::<u32>().ok())
            .expect("CARGO_PKG_VERSION is a semver triple; the workspace cannot build without one")
    };
    let stage = match (number(parts.next()), number(parts.next())) {
        (0, 0) => " pre-alpha",
        (0, _) => " alpha",
        _ => "",
    };
    let build = if debug { " (debug)" } else { "" };
    format!("v{version}{stage}{build}")
}

/// The window: the registered tools in sidebar order, the platform handle they share, the
/// receiving end of their shell requests, and which tool's view is on screen.
pub struct StudioApp {
    tools: Vec<Box<dyn StudioTool>>,
    /// The suite's whole registry as data: the entries no registered tool has are the
    /// sidebar's grayed planned list.
    planned: &'static [PlannedTool],
    /// The registered tools' ids this build shows only as planned entries: their CLI
    /// and help chapter stay, and the settings menu draws no section for them.
    cli_only: &'static [&'static str],
    ctx: ToolContext,
    requests: Receiver<ShellRequest>,
    /// Index into `tools` of the active view.
    active: usize,
    sidebar: Sidebar,
    status_bar: StatusBar,
    settings_menu: SettingsMenu,
    /// The help window, built over the registered tools' chapters; drawn only while
    /// the first-run dialog is not up.
    help_window: HelpWindow,
    title: WindowTitle,
    /// The common `theme` setting last applied to the context; `None` until the first
    /// frame, which applies it.
    applied_theme: Option<Theme>,
    /// What the first frame does to the window: `Some(Opening::Fit)` from `new` (the fit of
    /// ST-WIN-01), `Some(Opening::Place(_))` once `opening` found a remembered place (the
    /// first frame checks it against the monitors), `None` once sent.
    window_start: Option<window_size::Opening>,
    /// The last normal place a frame reported (not maximized or fullscreen, its rects known
    /// and in the pass's own points), written to `window_main` at exit.
    window_place: Option<WindowPlace>,
    /// Whether the window was maximized on its last frame that was not minimized (a window
    /// minimized from maximized and closed from the taskbar closed maximized), written with
    /// `window_place` at exit: the window reopens maximized, restoring to that place.
    window_maximized: bool,
    /// Reads the screen's monitors from the frame's window (`window_size::monitors`); a
    /// test replaces it with a fixed list, since a test's frame has no window.
    #[cfg(not(target_arch = "wasm32"))]
    monitors: fn(&eframe::Frame) -> Vec<window_size::Monitor>,
    /// The first-run data-location dialog's state.
    first_run: FirstRun,
}

impl StudioApp {
    /// A shell over `tools`, opening on the one the `last_tool` setting names (the first
    /// window tool when it names none, an unregistered one or a CLI-only one; ST-SIDE-01,
    /// ST-SIDE-08, ST-RUN-02). `planned` is the suite's registry including the tools this
    /// build does not register; `cli_only` is the registered tools' ids this build shows
    /// only as planned entries: their CLI and help chapter stay, their settings section
    /// does not.
    /// `requests` is the receiving end of the channel `ctx` sends shell requests to.
    /// `config_dir` is the user config folder the first-run dialog's "In my user folder"
    /// choice offers (`studio` passes `user_config_dir()`; `None` drops the choice — and
    /// is what the browser build gets, which never asks).
    pub fn new(
        tools: Vec<Box<dyn StudioTool>>,
        planned: &'static [PlannedTool],
        cli_only: &'static [&'static str],
        ctx: ToolContext,
        requests: Receiver<ShellRequest>,
        config_dir: Option<PathBuf>,
    ) -> StudioApp {
        // A CLI-only tool is never the active view: a `last_tool` naming one falls to the
        // first window tool; a build whose every tool is CLI-only is not one we make, so
        // it falls to 0 rather than getting a path of its own.
        let active = ctx
            .common()
            .last_tool
            .and_then(|last| tools.iter().position(|tool| tool.id() == last))
            .filter(|&index| !cli_only.contains(&tools[index].id()))
            .or_else(|| tools.iter().position(|tool| !cli_only.contains(&tool.id())))
            .unwrap_or(0);
        let help_window = HelpWindow::new(help::chapters(sidebar::drawn_order(
            &tools, planned, cli_only,
        )));
        StudioApp {
            tools,
            planned,
            cli_only,
            ctx,
            requests,
            active,
            sidebar: Sidebar::default(),
            status_bar: StatusBar::default(),
            settings_menu: SettingsMenu::default(),
            help_window,
            title: WindowTitle::default(),
            applied_theme: None,
            window_start: Some(window_size::Opening::Fit),
            window_place: None,
            window_maximized: false,
            #[cfg(not(target_arch = "wasm32"))]
            monitors: window_size::monitors,
            first_run: FirstRun::new(config_dir),
        }
    }

    /// The native window's options for this start, and what the first frame then does:
    /// the main class's remembered place when there is one (the first frame checks it
    /// against the monitors), else the fit.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn opening(&mut self) -> eframe::NativeOptions {
        let opening = self
            .ctx
            .common()
            .window_main
            .map_or(window_size::Opening::Fit, window_size::Opening::Place);
        self.window_start = Some(opening);
        window_size::native_options(&opening)
    }

    /// Makes `tools[index]` the active view and records it in `last_tool`, through the same
    /// best-effort settings save every change uses: a switch writes it, not the exit, so a
    /// crash keeps the record (ST-RUN-02). The one guard of "a CLI-only tool is never the
    /// active view" (ST-SIDE-08): the sidebar never emits its index, but a `SwitchTool`
    /// request or notice action can name it, so every switch funnels through here.
    fn set_active(&mut self, index: usize) {
        if index == self.active {
            return;
        }
        if self.cli_only.contains(&self.tools[index].id()) {
            debug!(
                "switch to CLI-only tool `{}` ignored",
                self.tools[index].id()
            );
            return;
        }
        // The outgoing tool hears of an end the bar saw before the switch (a notice
        // published earlier in the same drain), and its standing notice leaves the bar
        // without ending: the incoming tool is never told on its behalf.
        self.tell_standing_end();
        self.status_bar.stand(None);
        self.active = index;
        self.ctx.settings().lock().unwrap().common.last_tool =
            Some(self.tools[index].id().to_owned());
        self.ctx.request(ShellRequest::SettingsChanged);
    }

    /// Writes the settings file, best-effort: a failed save keeps the in-memory change for
    /// the session and raises the `data_dir_read_only` condition in the status bar (its only
    /// notice — no dialog, no retry); a save that succeeds drops it (ST-SET-03).
    #[cfg(not(target_arch = "wasm32"))]
    fn save_settings(&mut self) {
        if let Some(dir) = &self.ctx.paths().data_dir {
            let path = dir.join(crate::settings::SETTINGS_FILE_NAME);
            let result = self.ctx.settings().lock().unwrap().save(&path);
            self.status_bar
                .set_condition(ShellCondition::DataDirReadOnly, result.is_err());
            if let Err(error) = result {
                warn!(
                    "the settings file could not be written to {}: {error}",
                    path.display()
                );
            }
        }
    }

    /// The browser build has no settings file; its storage is the web phase's.
    #[cfg(target_arch = "wasm32")]
    fn save_settings(&mut self) {}

    /// Applies every request the tools queued since the last frame.
    fn apply_requests(&mut self, egui_ctx: &egui::Context) {
        let focused = egui_ctx.input(|input| input.focused);
        while let Ok(request) = self.requests.try_recv() {
            match request {
                ShellRequest::SwitchTool(id) => self.switch_to(id.as_str()),
                // A settings write queues this; the save is best-effort, at the next frame.
                ShellRequest::SettingsChanged => self.save_settings(),
                // A close prompt's answer: sent back through the window's own close
                // path, which asks the tools again: there is no second path.
                ShellRequest::Close => {
                    egui_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                // The newest notice replaces the one before it; while the window is
                // unfocused its text also holds in the title until focus returns.
                ShellRequest::Notify(notice) => {
                    self.title.notice(&notice.text, focused);
                    self.status_bar.set_notice(notice);
                }
                // A log line's message-code link: the same `open_help` the sidebar's
                // `?` uses, modal close included.
                ShellRequest::OpenHelp(target) => self.open_help(&target),
            }
        }
    }

    /// Tells the active tool its standing notice ended, once, if the bar saw it end since
    /// the last ask.
    fn tell_standing_end(&mut self) {
        if self.status_bar.take_standing_end()
            && let Some(tool) = self.tools.get_mut(self.active)
        {
            tool.standing_notice_ended();
        }
    }

    /// Makes the tool with `id` the active view, if it is registered and not CLI-only.
    fn switch_to(&mut self, id: &str) {
        match self.tools.iter().position(|tool| tool.id() == id) {
            Some(index) => self.set_active(index),
            None => debug!("switch to unknown tool `{id}` ignored"),
        }
    }

    /// The `activity()` texts of the registered tools in sidebar order; `include_active`
    /// puts the active tool's first (the window title counts it too — the status bar's
    /// right slot skips it because the tool's own run strip already shows it).
    fn activities(&self, include_active: bool) -> Vec<String> {
        let mut texts = Vec::new();
        if include_active
            && let Some(activity) = self.tools.get(self.active).and_then(|tool| tool.activity())
        {
            texts.push(activity.text());
        }
        for (index, tool) in self.tools.iter().enumerate() {
            if index == self.active {
                continue;
            }
            if let Some(activity) = tool.activity() {
                texts.push(activity.text());
            }
        }
        texts
    }

    /// Opens the help window at `target`. A shell modal (the settings menu, the About popup)
    /// closes first so it cannot cover the non-modal window.
    fn open_help(&mut self, target: &HelpTarget) {
        self.settings_menu.close();
        self.sidebar.about_open = false;
        let active = self
            .tools
            .get(self.active)
            .map(|tool| tool.id())
            .unwrap_or("");
        self.help_window.open(target, active);
    }

    /// Applies a clicked notice action (the notice is already dismissed). `SwitchTool` goes
    /// through the shell's own switch; `FixPesPath` points the settings at the running exe's
    /// folder, the main row or the version's own entry once any exists, then saves as any
    /// change; `SwitchVersion` makes the selector's manual pick; `OpenSettings` opens the
    /// settings menu on the active tool's section;
    /// `OpenHelp` opens the help window at the target. `OpenFolder` has no target yet (the
    /// compile's "Open output" is a later GUI-phase step): it only dismisses.
    fn apply_notice_effect(&mut self, effect: ActionEffect) {
        match effect {
            ActionEffect::SwitchTool(id) => self.switch_to(&id),
            ActionEffect::FixPesPath { folder, version } => {
                self.ctx
                    .settings()
                    .lock()
                    .unwrap()
                    .common
                    .fix_pes_folder(&folder, version);
                self.ctx.request(ShellRequest::SettingsChanged);
            }
            ActionEffect::SwitchVersion(version) => self.sidebar.pick_version(version, &self.ctx),
            ActionEffect::OpenSettings => self.settings_menu.open(self.active, &self.ctx),
            ActionEffect::OpenHelp(target) => self.open_help(&target),
            effect @ ActionEffect::OpenFolder(_) => {
                debug!("notice action {effect:?} has no target yet");
            }
        }
    }
}

impl eframe::App for StudioApp {
    fn logic(&mut self, egui_ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // The setting, never the system's theme, decides. It is applied once per change, so a
        // change is live at the next frame.
        let theme = self.ctx.common().theme;
        if self.applied_theme != Some(theme) {
            crate::theme::apply(egui_ctx, theme);
            // The zoom factor is the shell's alone: the width rule below sets it
            // every frame and would undo a keyboard zoom one frame later. A member
            // who wants larger text has the system's display scaling, which points
            // follow (spec/core/gui.md "Scaling with the width").
            egui_ctx.options_mut(|options| options.zoom_with_keyboard = false);
            self.applied_theme = Some(theme);
        }
        // The window is created at `DEFAULT_SIZE` or at the remembered place
        // `opening` found; the first frame finishes the job.
        let zoom = egui_ctx.zoom_factor();
        let (viewport, screen_rect) = egui_ctx.input(|input| {
            let viewport = input.viewport();
            (
                viewport.clone(),
                viewport.inner_rect.and(input.raw.screen_rect),
            )
        });
        // The window exists now, so the remembered place it was created at is checked
        // against its monitors: on none (a monitor unplugged since), the first frame fits it
        // as a first start does. The window was created there, off every monitor, so the move
        // goes unseen unless the desk's scales changed too.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(window_size::Opening::Place(place)) = self.window_start {
            let opening = window_size::place_or_fit(place, &(self.monitors)(_frame));
            // Created maximized before this check could run: kept maximized, its restore
            // button would put the window back off every monitor.
            if opening == window_size::Opening::Fit && place.maximized {
                egui_ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            }
            self.window_start = Some(opening);
        }
        match self.window_start {
            // The window is created at `DEFAULT_SIZE`; the first frame that
            // reports the monitor fits and centers it, once.
            Some(window_size::Opening::Fit) => {
                if let Some((size, top_left)) = window_size::fit(&viewport, zoom) {
                    egui_ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
                    egui_ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(top_left));
                    self.window_start = None;
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            Some(window_size::Opening::Place(place)) => {
                if let Some(native) = viewport.native_pixels_per_point {
                    // The position again, in this frame's points: the correction where
                    // winit converted the creation position with another monitor's scale.
                    // No size command: the creation set it, and a move to a monitor of
                    // another scale rescales it by the OS.
                    if window_size::resends_position(&place, native, &(self.monitors)(_frame)) {
                        egui_ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                            (place.position().to_vec2() / (native * zoom)).to_pos2(),
                        ));
                    }
                    // The maximize again, after any move: on Windows `set_outer_position`
                    // clears the maximized flag (a `ShowWindow(SW_RESTORE)`), and where
                    // nothing moved this is a no-op for a window already maximized. An X11
                    // window manager ignores the creation's maximize, asked of the window
                    // eframe keeps hidden until its first paint (egui-winit's
                    // `window_settings.rs`, `after_first_show`), so this is the one it
                    // honours. This frame's commands run after the show.
                    if place.maximized {
                        egui_ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                    }
                    self.window_start = None;
                }
            }
            None => {}
        }
        // The window's last normal place is kept for `on_exit` to write.
        if let Some(place) = window_size::place_of(&viewport, screen_rect, zoom) {
            self.window_place = Some(place);
        }
        if viewport.minimized != Some(true) {
            self.window_maximized = viewport.maximized == Some(true);
        }
        // Under the design width the whole UI scales down with the window's
        // width. The width is measured at zoom 1 so the zoom cannot feed back:
        // `screen_rect` is the one window size egui rescales to the pass's own
        // zoom at `begin_pass` when a queued change lands, so it times the
        // current zoom gives the same width at every zoom — `inner_rect` keeps
        // the zoom the input was built with on that frame and would feed it
        // back. A viewport reporting no inner rect leaves the zoom alone.
        if let Some(rect) = screen_rect {
            let new_zoom = window_size::width_zoom(rect.width() * zoom);
            if new_zoom != zoom {
                egui_ctx.set_zoom_factor(new_zoom);
                // The zoom lands when the next pass begins, so this pass lays out at
                // the old one: without the discard, the frame after a jump (a snap
                // from 1920 points to 960) reaches the screen laid out 960 wide.
                egui_ctx.request_discard("the window's width changed the zoom");
            }
        }
        for tool in &mut self.tools {
            tool.tick(&self.ctx);
        }
        // The poll runs before the requests drain so a switch's own requests (the settings
        // save, the notice) apply in the frame that switches, not one repaint later.
        self.sidebar
            .poll_running_pes(egui_ctx, &self.tools, &self.ctx);
        // Asked before the requests drain, so a notice published this frame replaces the
        // standing one and ends it, and its tool hears so before it is next asked.
        let standing = self
            .tools
            .get(self.active)
            .and_then(|tool| tool.standing_notice(&self.ctx));
        self.status_bar.stand(standing);
        self.apply_requests(egui_ctx);
        self.tell_standing_end();
        // The window's close asks the tools in sidebar order until one holds it. The holder
        // cancels the close and becomes the active view, its prompt answering through
        // `ToolContext::request_close`. A close the shell initiated itself takes the same
        // path, `ShellRequest::Close` coming back as a close request, so no second path
        // exists. A CLI-only tool is never asked:
        // it can never become the active view, so it could never show its prompt.
        let cli_only = self.cli_only;
        if egui_ctx.input(|input| input.viewport().close_requested())
            && let Some(index) = self
                .tools
                .iter_mut()
                .position(|tool| !cli_only.contains(&tool.id()) && tool.hold_close())
        {
            egui_ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.set_active(index);
            // The holder's prompt decides; no shell modal (the settings menu, the About
            // popup) may cover it.
            self.settings_menu.close();
            self.sidebar.about_open = false;
        }
        let focused = egui_ctx.input(|input| input.focused);
        let activities = self.activities(true);
        let active = self.tools.get(self.active);
        let active_label = active.map_or("", |tool| tool.label());
        let active_detail = active.and_then(|tool| tool.title_detail());
        if let Some(title) =
            self.title
                .frame(&activities, focused, active_label, active_detail.as_deref())
        {
            egui_ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The status bar claims its row first, so it spans the whole width under the sidebar
        // and the tool view.
        let activities = self.activities(false);
        let blocker = sidebar::version_blocker(&self.tools);
        #[cfg(not(target_arch = "wasm32"))]
        let monitor_of = |viewport: &egui::ViewportInfo, zoom: f32| {
            window_size::monitor_under(viewport, zoom, &(self.monitors)(_frame))
        };
        #[cfg(target_arch = "wasm32")]
        let monitor_of = |_: &egui::ViewportInfo, _: f32| None;
        let effect = egui::Panel::bottom("status_bar")
            .exact_size(24.0)
            .frame(
                egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::symmetric(8, 0)),
            )
            .show(ui, |ui| {
                self.status_bar.show(ui, &activities, blocker, monitor_of)
            })
            .inner;
        // Before the effect applies: a `SwitchTool` would make another tool the one told.
        self.tell_standing_end();
        if let Some(effect) = effect {
            self.apply_notice_effect(effect);
        }
        let clicked = egui::Panel::left("sidebar")
            .resizable(false)
            .exact_size(sidebar::RAIL_WIDTH)
            .frame(sidebar::rail_frame(ui.style()))
            .show(ui, |ui| {
                self.sidebar.show(
                    ui,
                    &self.tools,
                    self.active,
                    self.planned,
                    self.cli_only,
                    &self.ctx,
                )
            })
            .inner;
        match clicked {
            Some(SidebarClick::SwitchTool(index)) => self.set_active(index),
            Some(SidebarClick::OpenSettings) => {
                self.settings_menu.open(self.active, &self.ctx);
            }
            Some(SidebarClick::OpenHelp(target)) => self.open_help(&target),
            None => {}
        }
        egui::CentralPanel::default().show(ui, |ui| {
            // While the first-run dialog is up no tool's `view` runs: the central panel
            // stays empty behind the backdrop (ST-RUN-01).
            if !self.first_run.up(&self.ctx)
                && let Some(tool) = self.tools.get_mut(self.active)
            {
                tool.view(ui, &self.ctx);
            }
        });
        // The dialog is drawn instead of any other window (the draft's `render`), and
        // while it is up F1 opens nothing either.
        if self.first_run.up(&self.ctx) {
            self.first_run.show(ui.ctx(), &mut self.ctx);
        } else {
            // F1 opens the help on the active tool's chapter, as the sidebar's `?` does.
            if ui.input(|input| input.key_pressed(egui::Key::F1)) {
                let target = HelpTarget {
                    tool: self
                        .tools
                        .get(self.active)
                        .map(|tool| tool.id().to_owned())
                        .unwrap_or_default(),
                    topic: None,
                    anchor: None,
                };
                self.open_help(&target);
            }
            self.settings_menu.show(
                ui.ctx(),
                &mut self.tools,
                self.cli_only,
                &self.ctx,
                &mut self.sidebar,
            );
            self.help_window.show(ui.ctx());
        }
    }

    /// The window's last normal place, and whether it was maximized, go into `window_main` as
    /// the window closes. eframe calls this once, on every exit path.
    fn on_exit(&mut self) {
        if let Some(place) = self.window_place {
            self.ctx.settings().lock().unwrap().common.window_main = Some(WindowPlace {
                maximized: self.window_maximized,
                ..place
            });
            self.save_settings();
        }
    }
}

/// Opens the window titled `APP_NAME` on `app` and runs it until it closes; an error is a
/// window or graphics context that could not be set up.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_gui(mut app: StudioApp) -> anyhow::Result<()> {
    let options = app.opening();
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|creation| {
            #[cfg(windows)]
            window_size::hide_title_bar_icon(creation);
            crate::theme::install(&creation.egui_ctx);
            // The context's clones share the wake slot, so a `with_events` sink made before
            // the window exists still wakes it.
            let egui_ctx = creation.egui_ctx.clone();
            app.ctx
                .set_wake(move |delay| egui_ctx.request_repaint_after(delay));
            Ok(Box::new(app))
        }),
    )
    .map_err(|error| anyhow::anyhow!("cannot open the window ({error})"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Duration;

    use crossbeam_channel::{TryRecvError, unbounded};
    use eframe::App;

    use super::test_harness::*;
    use super::*;
    use crate::settings::Settings;
    use crate::shell::sidebar::SETTINGS_LABEL;
    use crate::status::{Notice, NoticeKind};

    /// `apply_requests` outside a frame: a default egui context.
    fn apply_requests(app: &mut StudioApp) {
        app.apply_requests(&egui::Context::default());
    }

    #[test]
    fn the_version_label_carries_the_release_stage_and_the_debug_mark() {
        assert_eq!(version_label_of("0.0.1", false), "v0.0.1 pre-alpha");
        assert_eq!(version_label_of("0.0.12", false), "v0.0.12 pre-alpha");
        assert_eq!(version_label_of("0.1.0", false), "v0.1.0 alpha");
        assert_eq!(version_label_of("0.12.3", false), "v0.12.3 alpha");
        assert_eq!(version_label_of("1.0.0", false), "v1.0.0");
        assert_eq!(version_label_of("2.3.4", false), "v2.3.4");
        assert_eq!(version_label_of("0.0.1", true), "v0.0.1 pre-alpha (debug)");
    }

    #[test]
    fn a_switch_request_makes_the_named_tool_active() {
        let Shell {
            mut app, ctx, _dir, ..
        } = shell();
        assert_eq!(app.active, 0);
        ctx.switch_to_tool("b");
        apply_requests(&mut app);
        assert_eq!(app.active, 1);
    }

    #[test]
    fn a_switch_to_an_unknown_tool_changes_nothing() {
        let Shell {
            mut app, ctx, _dir, ..
        } = shell();
        ctx.switch_to_tool("b");
        ctx.switch_to_tool("zz");
        apply_requests(&mut app);
        assert_eq!(app.active, 1);
    }

    #[test]
    fn notices_and_settings_changes_are_drained() {
        let Shell {
            mut app, ctx, _dir, ..
        } = shell();
        ctx.notify(Notice {
            text: "hello".to_owned(),
            kind: NoticeKind::Info,
            action: None,
        });
        ctx.set_tool_settings("a", toml::Table::new());
        apply_requests(&mut app);
        assert_eq!(app.requests.try_recv(), Err(TryRecvError::Empty));
        assert_eq!(app.active, 0);
    }

    #[test]
    fn logic_ticks_every_tool_then_applies_the_queued_requests() {
        let Shell {
            mut app,
            ctx,
            calls,
            _dir,
            ..
        } = shell();
        ctx.switch_to_tool("b");
        // eframe's own constructor of a `Frame` for testing an `App` without a window.
        app.logic(
            &egui::Context::default(),
            &mut eframe::Frame::_new_kittest(),
        );
        assert_eq!((calls[0].ticks(), calls[1].ticks()), (1, 1));
        assert_eq!(app.active, 1);
    }

    // ST-RUN-02
    #[test]
    fn the_gui_reopens_on_the_tool_active_when_it_closed_on() {
        let dir = test_dir("studio_core_sidebar_last_tool");
        fs::create_dir_all(&dir).unwrap();
        let egui_ctx = test_context();

        {
            let (a, calls_a) = stub("a", "A", None);
            let (b, calls_b) = stub("b", "B", None);
            let Shell { mut app, .. } = shell_with(
                test_settings(&dir),
                vec![a, b],
                &[],
                Some(dir.clone()),
                vec![calls_a, calls_b],
            );
            let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
            click(&egui_ctx, &mut app, entry(&nodes, "B"));
            assert_eq!(app.active, 1);
            frame(&egui_ctx, &mut app, Vec::new(), true);
        }

        let written = Settings::load(&dir.join(crate::settings::SETTINGS_FILE_NAME)).unwrap();
        assert_eq!(
            written.common.last_tool.as_deref(),
            Some("b"),
            "the switch wrote last_tool"
        );

        let (a, calls_a) = stub("a", "A", None);
        let (b, calls_b) = stub("b", "B", None);
        let Shell { mut app, calls, .. } = shell_with(
            written,
            vec![a, b],
            &[],
            Some(dir.clone()),
            vec![calls_a, calls_b],
        );
        frame(&egui_ctx, &mut app, Vec::new(), true);
        assert_eq!(app.active, 1, "the GUI reopens on the second tool");
        assert_eq!(calls[1].views(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }

    /// A shell over `a`, `b`, `c` with `data_dir` under `dir`: the close scenarios need
    /// the first-run dialog out of the way, or it would hide the prompts' views.
    fn close_shell(dir: &std::path::Path) -> Shell {
        let (a, calls_a) = stub("a", "A", None);
        let (b, calls_b) = stub("b", "B", None);
        let (c, calls_c) = stub("c", "C", None);
        shell_with(
            test_settings(dir),
            vec![a, b, c],
            &[],
            Some(dir.to_path_buf()),
            vec![calls_a, calls_b, calls_c],
        )
    }

    // ST-RUN-05
    #[test]
    fn a_close_is_held_by_the_first_tool_with_unsaved_work_then_the_next() {
        let dir = TestDir::new("studio_core_close_unsaved");
        let egui_ctx = test_context();
        let Shell { mut app, calls, .. } = close_shell(dir.path());
        *calls[0].unsaved.lock().unwrap() = true;
        *calls[1].unsaved.lock().unwrap() = true;
        app.set_active(2);

        // The window's close is held by the first tool that reports work, in sidebar
        // order: the tools after it are never asked.
        let out = frame_closing(&egui_ctx, &mut app);
        assert!(
            out.commands.contains(&egui::ViewportCommand::CancelClose),
            "the window stays while a tool holds the close"
        );
        assert_eq!(app.active, 0, "A's prompt is the active view");
        assert_eq!(
            (calls[0].holds(), calls[1].holds(), calls[2].holds()),
            (1, 0, 0),
            "the asks stop at the first tool that holds"
        );
        entry(&out.nodes, "Discard");
        entry(&out.nodes, "Cancel");

        // A's Discard answers the prompt with a close request; the shell sends it
        // back through the window's own close path.
        click(&egui_ctx, &mut app, entry(&out.nodes, "Discard"));
        let out = frame(&egui_ctx, &mut app, Vec::new(), true);
        assert!(
            out.commands.contains(&egui::ViewportCommand::Close),
            "the answered prompt sends the close on"
        );

        // The close comes back and asks again: B still holds, its prompt is active.
        let out = frame_closing(&egui_ctx, &mut app);
        assert!(out.commands.contains(&egui::ViewportCommand::CancelClose));
        assert_eq!(app.active, 1, "B's prompt is the active view");
        entry(&out.nodes, "Discard");

        click(&egui_ctx, &mut app, entry(&out.nodes, "Discard"));
        frame(&egui_ctx, &mut app, Vec::new(), true);
        let out = frame_closing(&egui_ctx, &mut app);
        assert!(
            !out.commands.contains(&egui::ViewportCommand::CancelClose),
            "nothing left to hold: the window closes"
        );
    }

    // ST-RUN-05
    #[test]
    fn a_cancelled_prompt_keeps_the_window_and_asks_no_other_tool() {
        let dir = TestDir::new("studio_core_close_cancel");
        let egui_ctx = test_context();
        let Shell { mut app, calls, .. } = close_shell(dir.path());
        *calls[0].unsaved.lock().unwrap() = true;
        *calls[1].unsaved.lock().unwrap() = true;
        app.set_active(2);

        let out = frame_closing(&egui_ctx, &mut app);
        assert_eq!(app.active, 0);
        click(&egui_ctx, &mut app, entry(&out.nodes, "Cancel"));

        // A cancel sends nothing: the window stays, no other tool's prompt shows.
        for _ in 0..3 {
            let out = frame(&egui_ctx, &mut app, Vec::new(), true);
            assert!(
                !out.commands.contains(&egui::ViewportCommand::Close),
                "a cancel sends no close"
            );
            for label in ["Discard", "Cancel", "Close when it ends", "Keep the window"] {
                assert!(
                    !has_label(&out.nodes, label),
                    "no prompt is drawn anymore, B's included: {label}"
                );
            }
        }
        assert_eq!(calls[1].holds(), 0, "B was never asked");
    }

    // ST-RUN-05
    #[test]
    fn a_held_close_closes_the_settings_menu_over_the_prompt() {
        let dir = TestDir::new("studio_core_close_menu");
        let egui_ctx = test_context();
        let (a, calls_a) = stub("a", "A", None);
        let (b, calls_b) = stub("b", "B", None);
        let Shell { mut app, calls, .. } = shell_with(
            test_settings(dir.path()),
            vec![a, b],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a, calls_b],
        );
        *calls[0].unsaved.lock().unwrap() = true;
        app.set_active(1);

        // The settings menu open over B's view when the close is asked.
        let nodes = idle(&egui_ctx, &mut app, 2).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        assert!(has_text(&nodes, "Threads"), "the menu is open");

        // The close is held: A's prompt becomes the view, and the menu's modal no
        // longer covers it.
        let out = frame_closing(&egui_ctx, &mut app);
        assert!(out.commands.contains(&egui::ViewportCommand::CancelClose));
        assert_eq!(app.active, 0);
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        assert!(
            !has_text(&nodes, "Threads"),
            "the settings menu is gone once the close is held"
        );

        // A's prompt answers: Discard sends the close on. The pointer rested on
        // the rail while the menu closed, so the sidebar's overlay is open over
        // the view's left edge: it leaves first, as a member's would.
        let middle = calls[0].view_rect().unwrap().center();
        frame(
            &egui_ctx,
            &mut app,
            vec![egui::Event::PointerMoved(middle)],
            true,
        );
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Discard"));
        let out = frame(&egui_ctx, &mut app, Vec::new(), true);
        assert!(
            out.commands.contains(&egui::ViewportCommand::Close),
            "the answered prompt sends the close on"
        );
    }

    // ST-RUN-05
    #[test]
    fn a_held_close_closes_the_about_popup_over_the_prompt() {
        let dir = TestDir::new("studio_core_close_about");
        let egui_ctx = test_context();
        let (a, calls_a) = stub("a", "A", None);
        let (b, calls_b) = stub("b", "B", None);
        let Shell { mut app, calls, .. } = shell_with(
            test_settings(dir.path()),
            vec![a, b],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a, calls_b],
        );
        *calls[0].unsaved.lock().unwrap() = true;
        app.set_active(1);

        // The About popup open over B's view when the close is asked.
        app.sidebar.about_open = true;
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        assert!(has_label(&nodes, "Copy"), "the About popup is open");

        let out = frame_closing(&egui_ctx, &mut app);
        assert!(out.commands.contains(&egui::ViewportCommand::CancelClose));
        assert_eq!(app.active, 0);
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        assert!(
            !has_label(&nodes, "Copy"),
            "the About popup is gone once the close is held"
        );
        click(&egui_ctx, &mut app, entry(&nodes, "Discard"));
        let out = frame(&egui_ctx, &mut app, Vec::new(), true);
        assert!(
            out.commands.contains(&egui::ViewportCommand::Close),
            "the prompt is uncovered and answers"
        );
    }

    // ST-RUN-06
    #[test]
    fn a_run_in_flight_holds_the_close_until_the_run_ends() {
        let dir = TestDir::new("studio_core_close_run");
        let egui_ctx = test_context();
        let (a, calls_a) = stub("a", "A", None);
        let (b, calls_b) = stub("b", "B", None);
        calls_a.set_activity("A", Some((12, 48)), Duration::from_secs(72));
        let (run_tx, run_rx) = unbounded();
        *calls_a.events.lock().unwrap() = Some(run_rx);
        let Shell { mut app, .. } = shell_with(
            test_settings(dir.path()),
            vec![a, b],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a, calls_b],
        );
        app.set_active(1);

        let out = frame_closing(&egui_ctx, &mut app);
        assert!(
            out.commands.contains(&egui::ViewportCommand::CancelClose),
            "the window stays while the run goes"
        );
        assert_eq!(
            app.active, 0,
            "the running tool's prompt is the active view"
        );
        entry(&out.nodes, "Close when it ends");
        entry(&out.nodes, "Keep the window");

        click(&egui_ctx, &mut app, entry(&out.nodes, "Close when it ends"));
        // While the run goes on, no close is sent.
        for _ in 0..3 {
            let out = frame(&egui_ctx, &mut app, Vec::new(), true);
            assert!(
                !out.commands.contains(&egui::ViewportCommand::Close),
                "the run still goes: no close"
            );
        }

        // The run's last event lands: the tool's tick sees it and sends the close on.
        run_tx.send(envelope()).unwrap();
        let out = frame(&egui_ctx, &mut app, Vec::new(), true);
        assert!(
            out.commands.contains(&egui::ViewportCommand::Close),
            "the run ended: the held close goes through"
        );
        let out = frame_closing(&egui_ctx, &mut app);
        assert!(
            !out.commands.contains(&egui::ViewportCommand::CancelClose),
            "nothing holds it anymore"
        );
    }

    // ST-RUN-06
    #[test]
    fn keeping_the_window_lets_the_run_go_on() {
        let dir = TestDir::new("studio_core_close_keep");
        let egui_ctx = test_context();
        let (a, calls_a) = stub("a", "A", None);
        let (b, calls_b) = stub("b", "B", None);
        calls_a.set_activity("A", Some((12, 48)), Duration::from_secs(72));
        let Shell { mut app, calls, .. } = shell_with(
            test_settings(dir.path()),
            vec![a, b],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a, calls_b],
        );
        app.set_active(1);

        let out = frame_closing(&egui_ctx, &mut app);
        assert_eq!(app.active, 0);
        click(&egui_ctx, &mut app, entry(&out.nodes, "Keep the window"));

        for _ in 0..3 {
            let out = frame(&egui_ctx, &mut app, Vec::new(), true);
            assert!(
                !out.commands.contains(&egui::ViewportCommand::Close),
                "keeping the window sends no close"
            );
        }
        assert!(
            calls[0].activity.lock().unwrap().is_some(),
            "the run is still reported"
        );
    }

    #[test]
    fn a_close_no_tool_holds_goes_through() {
        let egui_ctx = test_context();
        let Shell {
            mut app,
            calls,
            _dir,
            ..
        } = shell();

        let out = frame_closing(&egui_ctx, &mut app);
        assert!(
            !out.commands.contains(&egui::ViewportCommand::CancelClose),
            "nothing held: no cancel"
        );
        assert_eq!(
            (calls[0].holds(), calls[1].holds()),
            (1, 1),
            "every tool was asked, none held"
        );
    }
}
