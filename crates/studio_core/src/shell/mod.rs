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
#[cfg(not(target_arch = "wasm32"))]
mod report;
mod settings_menu;
mod sidebar;
mod status_bar;
mod taskbar;
#[cfg(test)]
pub(crate) mod test_harness;
mod title;
#[cfg(not(target_arch = "wasm32"))]
mod update_dialog;
mod window_size;

use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use crossbeam_channel::Receiver;
#[cfg(not(target_arch = "wasm32"))]
use crossbeam_channel::TryRecvError;
use log::debug;
#[cfg(not(target_arch = "wasm32"))]
use log::warn;

pub use launch::{Launch, LaunchMode, parse_launch, run_cli};

use crate::help::{self, HelpTarget};
use crate::settings::{Theme, WindowPlace};
use crate::status::ActionEffect;
#[cfg(not(target_arch = "wasm32"))]
use crate::status::{Notice, NoticeAction, NoticeKind, ShellCondition};
use crate::tool::{PlannedTool, ShellRequest, StudioTool, ToolContext};
#[cfg(not(target_arch = "wasm32"))]
use crate::updater;

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
/// The `Tvers` field of every CPK the suite writes (the Compiler's output CPK and the
/// pre-Fox face CPKs nested in it, the Save editor's bins CPK): the display name and the
/// version, naming the tool that wrote the file. One string per release, so a release
/// compiling the same exports writes the same bytes.
pub const TOOL_VERSION: &str = concat!(app_name!(), " ", env!("CARGO_PKG_VERSION"));

/// The pane's width at the default window, `window_size::DEFAULT_SIZE`'s width
/// less `sidebar::RAIL_WIDTH`, which the tools' views are calibrated for. A
/// larger window keeps a view at this width, centred; a smaller one scales the
/// whole UI down rather than narrowing the pane.
pub const DEFAULT_PANE_WIDTH: f32 = window_size::DEFAULT_SIZE.x - sidebar::RAIL_WIDTH;

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
pub(crate) fn version_label_of(version: &str, debug: bool) -> String {
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

/// The status bar's notice on the first start of a newer version, `Updated to v0.0.2
/// pre-alpha` with a What's new that opens the help on the changelog, when `last_run`,
/// the version whose window last started, is older than `own`, the program's. No
/// notice when none is stored (a first run, or a file only versions without the notice
/// wrote), when it is the same or newer (an older copy started after a newer one), or
/// when it does not read as a version (edited by hand).
#[cfg(not(target_arch = "wasm32"))]
fn updated_notice(last_run: Option<&str>, own: &str) -> Option<Notice> {
    let last_run = semver::Version::parse(last_run?).ok()?;
    let own_version = semver::Version::parse(own)
        .expect("the program's version is a semver triple; the workspace cannot build without one");
    (last_run < own_version).then(|| Notice {
        text: format!("Updated to {}", version_label_of(own, false)),
        kind: NoticeKind::Info,
        action: Some(NoticeAction {
            label: "What's new".to_owned(),
            effect: ActionEffect::OpenHelp(HelpTarget {
                tool: help::WHATS_NEW_ID.to_owned(),
                topic: None,
                anchor: None,
            }),
        }),
    })
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
    /// The window's focus last frame: a regain drops the context's per-version
    /// exe checks, as it re-lists the sidebar's own.
    was_focused: bool,
    status_bar: StatusBar,
    settings_menu: SettingsMenu,
    /// The help window, built over the registered tools' chapters; drawn only while
    /// the first-run dialog is not up.
    help_window: HelpWindow,
    title: WindowTitle,
    /// The common `theme` setting last applied to the context; `None` until the first
    /// frame, which applies it.
    applied_theme: Option<Theme>,
    /// The common `invert_number_scrolling` setting last applied to the context's
    /// wheel flag, as `applied_theme` is for the theme.
    applied_inverted: Option<bool>,
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
    /// The program's version: the build's own in the window, a test's in the tests, so
    /// a test's outcome does not change when the workspace's version does. The first
    /// frame compares it with `last_run_version` and stores it there; the update check
    /// offers only a release newer than it.
    #[cfg(not(target_arch = "wasm32"))]
    version: &'static str,
    /// Whether the first frame has recorded `version` in `last_run_version`.
    #[cfg(not(target_arch = "wasm32"))]
    version_recorded: bool,
    /// The first-run data-location dialog's state.
    first_run: FirstRun,
    /// The update check's fetch: `updater::fetch_releases` in the window, a fixture
    /// in the tests. A `dyn Fn`, not a plain `fn` as the sidebar's probe is, so a
    /// test's fake can wait on a channel it captured.
    #[cfg(not(target_arch = "wasm32"))]
    update_fetch: Arc<updater::Fetch>,
    /// Where the update check stands: `poll_update` starts one on the first
    /// frame and again each time `next_at` is reached, and drains a finished
    /// check's reply on a later frame.
    #[cfg(not(target_arch = "wasm32"))]
    update_poll: UpdatePoll,
    /// The release the status bar's offer is about, while the offer stands: the
    /// Details dialog and the download read it. An `Arc`, so the install's
    /// worker thread shares it with the dialog.
    #[cfg(not(target_arch = "wasm32"))]
    update_release: Option<Arc<updater::Release>>,
    /// The update's download: `updater::open_url` in the window, a fake in
    /// the tests, as `update_fetch` is for the check.
    #[cfg(not(target_arch = "wasm32"))]
    update_open_url: Arc<updater::OpenUrl>,
    /// The program's file in the program's folder as this window found it when
    /// it started (`None` when it could not be read): Update now is refused
    /// once the file is another one, which an update from a second window
    /// running from the same folder put there.
    #[cfg(not(target_arch = "wasm32"))]
    update_program: Option<updater::ProgramStamp>,
    /// The update dialog: its open state, its stage with the install while one
    /// runs, and the notes' cache.
    #[cfg(not(target_arch = "wasm32"))]
    update_dialog: update_dialog::UpdateDialog,
    /// The program to start once the window has closed: set by `Update ready`'s
    /// Restart now, cleared by a close a tool holds. Shared with `run_gui`, which
    /// hands it back once the window and this app are gone.
    #[cfg(not(target_arch = "wasm32"))]
    update_restart: Arc<Mutex<Option<PathBuf>>>,
    /// The "Report a problem" dialog the sidebar's `⚠` entry opens.
    #[cfg(not(target_arch = "wasm32"))]
    report: report::Report,
    /// The window's log's path (`utw.log` — one per window slot): `studio`'s
    /// `gui_log` knows it and `studio_core` does not, so `with_log_file`
    /// hands it in; `None` when the run has no log. The repro bundle's
    /// `utw.log` entry reads it.
    #[cfg(not(target_arch = "wasm32"))]
    log_file: Option<PathBuf>,
    /// The window's own session folder's name under `<data
    /// location>/sessions`: `run_gui_mode` makes the session's header and
    /// hands the name in, so Report a problem's list of earlier sessions
    /// can leave it out; `None` when the run has no data location.
    #[cfg(not(target_arch = "wasm32"))]
    session_folder: Option<String>,
}

/// Where the update check stands (native only; the browser build checks
/// nothing): one check at a time, off the UI thread, and again once
/// `CHECK_PERIOD` has passed since the last one ended.
#[cfg(not(target_arch = "wasm32"))]
enum UpdatePoll {
    /// Nothing in flight. `next_at` is the instant the next check may start
    /// at: the window's start for the first check, the last check's end plus
    /// `CHECK_PERIOD` after. A monotonic instant: nothing is stored across
    /// starts, so the wall clock has no part left.
    Idle { next_at: Instant },
    /// The check's worker runs; its reply lands here once.
    Running(Receiver<Result<Option<updater::Release>, updater::UpdateError>>),
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
        #[cfg(not(target_arch = "wasm32"))]
        let update_program = updater::ProgramStamp::of(&ctx.paths().exe_dir);
        StudioApp {
            tools,
            planned,
            cli_only,
            ctx,
            requests,
            active,
            sidebar: Sidebar::default(),
            was_focused: false,
            status_bar: StatusBar::default(),
            settings_menu: SettingsMenu::default(),
            help_window,
            title: WindowTitle::default(),
            applied_theme: None,
            applied_inverted: None,
            window_start: Some(window_size::Opening::Fit),
            window_place: None,
            window_maximized: false,
            #[cfg(not(target_arch = "wasm32"))]
            monitors: window_size::monitors,
            #[cfg(not(target_arch = "wasm32"))]
            version: env!("CARGO_PKG_VERSION"),
            #[cfg(not(target_arch = "wasm32"))]
            version_recorded: false,
            first_run: FirstRun::new(config_dir),
            #[cfg(not(target_arch = "wasm32"))]
            update_fetch: Arc::new(updater::fetch_releases),
            #[cfg(not(target_arch = "wasm32"))]
            update_poll: UpdatePoll::Idle {
                next_at: Instant::now(),
            },
            #[cfg(not(target_arch = "wasm32"))]
            update_release: None,
            #[cfg(not(target_arch = "wasm32"))]
            update_open_url: Arc::new(updater::open_url),
            #[cfg(not(target_arch = "wasm32"))]
            update_program,
            #[cfg(not(target_arch = "wasm32"))]
            update_dialog: update_dialog::UpdateDialog::default(),
            #[cfg(not(target_arch = "wasm32"))]
            update_restart: Arc::new(Mutex::new(None)),
            #[cfg(not(target_arch = "wasm32"))]
            report: report::Report::default(),
            #[cfg(not(target_arch = "wasm32"))]
            log_file: None,
            #[cfg(not(target_arch = "wasm32"))]
            session_folder: None,
        }
    }

    /// The log's path, for the repro bundle's `utw.log` entry: `run_gui_mode`
    /// calls this once with `installed_log`'s path. Not on `AppPaths`, which
    /// every caller of `ToolContext::new` would have to fill.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_log_file(mut self, log_file: Option<PathBuf>) -> StudioApp {
        self.log_file = log_file;
        self
    }

    /// The window's own session folder's name, which Report a problem's
    /// earlier-sessions list leaves out: `run_gui_mode` calls this once
    /// with `session_folder_name` of the header it made. `None` when the
    /// run has no data location — then no session is the window's own.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_session_folder(mut self, session_folder: Option<String>) -> StudioApp {
        self.session_folder = session_folder;
        self
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
        self.active = index;
        self.ctx.settings().lock().unwrap().common.last_tool =
            Some(self.tools[index].id().to_owned());
        self.ctx.request(ShellRequest::SettingsChanged);
    }

    /// Writes the settings file, best-effort: a failed save keeps the in-memory change for
    /// the session and raises the `settings_save_failed` condition in the status bar (its only
    /// notice — no dialog, no retry); a save that succeeds drops it (ST-SET-03).
    #[cfg(not(target_arch = "wasm32"))]
    fn save_settings(&mut self) {
        if let Some(dir) = &self.ctx.paths().data_dir {
            let path = dir.join(crate::settings::SETTINGS_FILE_NAME);
            let result = self.ctx.settings().lock().unwrap().save(&path);
            self.status_bar.clear_save_failed();
            if let Err(error) = result {
                self.status_bar.set_condition(
                    ShellCondition::SettingsSaveFailed {
                        error: error.to_string(),
                    },
                    true,
                );
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

    /// The update check while the window is open (native only; the browser
    /// build checks nothing): with `check_for_updates` on, a check starts on
    /// the first frame and again each time `next_at` is reached, one
    /// `CHECK_PERIOD` after the last check ended, a failed one included. A
    /// finished worker's reply is drained on a later frame, so no frame waits
    /// on the fetch. A check stores nothing: a release newer than the build
    /// and not the skipped one raises the status bar's offer and stays for
    /// the Details dialog, replacing a standing offer's release unless it is
    /// the same one; a failed check writes one log line and shows nothing.
    /// While a check is Idle the frame asks egui for the repaint at `next_at`,
    /// so an idle window wakes for it; the frame that drains a finished
    /// check's reply schedules the same way. Nothing runs while an install
    /// runs or after one succeeded: a check could replace the release under
    /// the update dialog.
    #[cfg(not(target_arch = "wasm32"))]
    fn poll_update(&mut self, egui_ctx: &egui::Context) {
        if self.update_dialog.holds_checks() {
            return;
        }
        // A finished check's reply is drained before the Idle scheduling
        // below, so the frame that ends it already wakes for the next one.
        let reply = match &self.update_poll {
            UpdatePoll::Running(rx) => Some(rx.try_recv()),
            UpdatePoll::Idle { .. } => None,
        };
        let ended = match reply {
            Some(Ok(Ok(newest))) => {
                let own = semver::Version::parse(self.version).expect(
                    "the program's version is a semver triple: the build's own, which \
                     the workspace cannot build without, or a test's",
                );
                let skipped = self.ctx.common().skipped_version;
                if let Some(release) =
                    newest.filter(|release| updater::offers(release, &own, skipped.as_deref()))
                    && !matches!(
                        &self.update_release,
                        Some(standing) if standing.version == release.version
                    )
                {
                    // A newer release replaces the offer it outranks.
                    if let Some(old) = self.update_release.take() {
                        self.status_bar.set_condition(
                            ShellCondition::UpdateAvailable {
                                version: old.version.to_string(),
                            },
                            false,
                        );
                    }
                    self.status_bar.set_condition(
                        ShellCondition::UpdateAvailable {
                            version: release.version.to_string(),
                        },
                        true,
                    );
                    self.update_release = Some(Arc::new(release));
                }
                true
            }
            Some(Ok(Err(error))) => {
                warn!("the update check failed: {error}");
                true
            }
            Some(Err(TryRecvError::Disconnected)) => {
                warn!("the update check's worker ended without a reply");
                true
            }
            // A check still in flight, or none running: nothing to drain.
            Some(Err(TryRecvError::Empty)) | None => false,
        };
        // Whatever the check found, or if it failed, the next one is a
        // period after it ended.
        if ended {
            self.update_poll = UpdatePoll::Idle {
                next_at: Instant::now() + updater::CHECK_PERIOD,
            };
        }
        let UpdatePoll::Idle { next_at } = self.update_poll else {
            return;
        };
        if !self.ctx.common().check_for_updates {
            // Checks off: nothing to start, nothing to wake for.
            return;
        }
        let now = Instant::now();
        if now >= next_at {
            // Due now: the check runs on its own thread, drained on a
            // later frame.
            let rx = updater::start_check(Arc::clone(&self.update_fetch), Some(egui_ctx));
            self.update_poll = UpdatePoll::Running(rx);
        } else {
            // Later: wake for the moment it falls due, so an idle window
            // comes back for it.
            egui_ctx.request_repaint_after(next_at - now);
        }
    }

    /// The browser build checks nothing; its storage is the web phase's.
    #[cfg(target_arch = "wasm32")]
    fn poll_update(&mut self, _egui_ctx: &egui::Context) {}

    /// The first frame's record of the program's `version`: the `Updated to` notice
    /// when the version whose window last started is older, then `version` stored in
    /// `last_run_version` and saved as every settings change is, whatever the
    /// comparison found, so the notice shows on one start only. The command line never
    /// comes here: it reads and writes no `last_run_version`.
    #[cfg(not(target_arch = "wasm32"))]
    fn record_version(&mut self) {
        let own = self.version;
        let last_run = self
            .ctx
            .settings()
            .lock()
            .unwrap()
            .common
            .last_run_version
            .replace(own.to_owned());
        if let Some(notice) = updated_notice(last_run.as_deref(), own) {
            self.ctx.notify(notice);
        }
        self.ctx.request(ShellRequest::SettingsChanged);
    }

    /// The update dialog, drawn while `update_release` stands (the release it
    /// was opened for gone, it closes itself); its Update now installs into
    /// the program's folder, `logic` draining the install's outcome. Its
    /// answers: Skip this version stores the version as `skipped_version`,
    /// drops the offer and the release and saves as every settings change
    /// does; Restart now arms the restart and asks the window to close as its
    /// close button does (`run_gui` hands the new program back); the others
    /// only close it.
    #[cfg(not(target_arch = "wasm32"))]
    fn show_update_dialog(&mut self, egui_ctx: &egui::Context) {
        let Some(release) = self.update_release.as_ref() else {
            self.update_dialog.close();
            return;
        };
        let answer = self.update_dialog.show(
            egui_ctx,
            release,
            self.version,
            &self.ctx.paths().exe_dir,
            self.update_program,
            &self.update_open_url,
        );
        match answer {
            Some(update_dialog::UpdateAnswer::SkipVersion) => {
                let version = release.version.to_string();
                self.ctx.settings().lock().unwrap().common.skipped_version = Some(version.clone());
                self.status_bar
                    .set_condition(ShellCondition::UpdateAvailable { version }, false);
                self.update_release = None;
                self.save_settings();
            }
            // The path is the bundle's own, never this program's: on Linux the
            // running program has moved into `old/` by now.
            Some(update_dialog::UpdateAnswer::RestartNow) => {
                *self.update_restart.lock().unwrap() =
                    Some(updater::program_path(&self.ctx.paths().exe_dir));
                self.ctx.request(ShellRequest::Close);
            }
            Some(update_dialog::UpdateAnswer::RemindLater | update_dialog::UpdateAnswer::Later)
            | None => {}
        }
    }

    /// The browser build has no update dialog.
    #[cfg(target_arch = "wasm32")]
    fn show_update_dialog(&mut self, _egui_ctx: &egui::Context) {}

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

    /// Opens the help window at `target`. A shell modal (the settings menu, the About
    /// popup, the update and report dialogs) closes first so it cannot cover the
    /// non-modal window.
    fn open_help(&mut self, target: &HelpTarget) {
        self.settings_menu.close();
        self.sidebar.about_open = false;
        #[cfg(not(target_arch = "wasm32"))]
        self.update_dialog.close();
        #[cfg(not(target_arch = "wasm32"))]
        self.report.close();
        let active = self
            .tools
            .get(self.active)
            .map(|tool| tool.id())
            .unwrap_or("");
        self.help_window.open(target, active);
    }

    /// Applies the action the status bar returned: the notice's own (the notice is
    /// already dismissed) or a left-slot entry's, which dismisses nothing. `SwitchTool` goes
    /// through the shell's own switch; `FixPesPath` points the settings at the running exe's
    /// folder, the main row or the version's own entry once any exists, then saves as any
    /// change; `OpenSettings` opens the settings menu on the active tool's section;
    /// `OpenHelp` opens the help window at the target; `OpenUpdateDetails` opens the update
    /// offer's dialog; `OpenFolder` opens the folder in the system's file manager
    /// (`files::open_folder`).
    fn apply_notice_effect(&mut self, effect: ActionEffect) {
        match effect {
            ActionEffect::SwitchTool(id) => self.switch_to(&id),
            ActionEffect::FixPesPath { folder, version } => {
                self.ctx.set_pes_folder_for(&folder, version);
            }
            ActionEffect::OpenSettings => self.settings_menu.open(self.active, &self.ctx),
            ActionEffect::OpenHelp(target) => self.open_help(&target),
            #[cfg(not(target_arch = "wasm32"))]
            ActionEffect::OpenUpdateDetails => self.update_dialog.open(),
            #[cfg(target_arch = "wasm32")]
            ActionEffect::OpenUpdateDetails => {
                debug!("the update dialog does not exist in the browser build");
            }
            ActionEffect::OpenFolder(folder) => crate::files::open_folder(&folder),
        }
    }
}

impl eframe::App for StudioApp {
    fn logic(&mut self, egui_ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // The setting, never the system's theme, decides. It is applied once per change, so a
        // change is live at the next frame.
        let common = self.ctx.common();
        if self.applied_theme != Some(common.theme) {
            crate::theme::apply(egui_ctx, common.theme);
            // The zoom factor is the shell's alone: the width rule below sets it
            // every frame and would undo a keyboard zoom one frame later. A member
            // who wants larger text has the system's display scaling, which points
            // follow (spec/core/gui.md "Scaling with the width").
            egui_ctx.options_mut(|options| options.zoom_with_keyboard = false);
            self.applied_theme = Some(common.theme);
        }
        // Invert number scrolling goes where `number_direction` reads it, the
        // context's data, applied once per change as the theme is.
        if self.applied_inverted != Some(common.invert_number_scrolling) {
            crate::widgets::set_inverted(egui_ctx, common.invert_number_scrolling);
            self.applied_inverted = Some(common.invert_number_scrolling);
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
        // Queued before the requests drain, so the notice and the save apply in this frame.
        #[cfg(not(target_arch = "wasm32"))]
        if !self.version_recorded {
            self.version_recorded = true;
            self.record_version();
        }
        // The install's outcome is drained before any panel draws, so the
        // frame that drains `Done` already draws `Update ready` on the bar.
        // Drained in the dialog's own draw (which follows the bar's), an
        // install ending while the dialog is hidden would leave `Update
        // available` standing until a repaint an idle window never gets.
        // Before `poll_update`: a failed or cancelled install frees the
        // checks it held, so the frame that ends one sees them free already.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(release) = self.update_release.as_ref()
            && self.update_dialog.poll_install()
        {
            let version = release.version.to_string();
            self.status_bar.set_condition(
                ShellCondition::UpdateAvailable {
                    version: version.clone(),
                },
                false,
            );
            self.status_bar
                .set_condition(ShellCondition::UpdateReady { version }, true);
        }
        self.poll_update(egui_ctx);
        self.apply_requests(egui_ctx);
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
            // popup, the update and report dialogs) may cover it. An open popup (a
            // menu, a dropdown) is no shell modal either, and it paints over the prompt.
            self.settings_menu.close();
            self.sidebar.about_open = false;
            #[cfg(not(target_arch = "wasm32"))]
            self.update_dialog.close();
            #[cfg(not(target_arch = "wasm32"))]
            self.report.close();
            egui::Popup::close_all(egui_ctx);
            // A held close drops Restart now for good: the member answers the tool's
            // prompt, and a later close is an ordinary one.
            #[cfg(not(target_arch = "wasm32"))]
            {
                *self.update_restart.lock().unwrap() = None;
            }
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
            .show(ui, |ui| self.status_bar.show(ui, &activities, monitor_of))
            .inner;
        if let Some(effect) = effect {
            self.apply_notice_effect(effect);
        }
        // The context's per-version exe checks keep the sidebar's rule of a re-list
        // on the window's regained focus: the drop runs here, where the sidebar's
        // own check learns of the focus on this same frame.
        let focused = ui.input(|input| input.focused);
        if focused && !self.was_focused {
            self.ctx.clear_pes_exe_checks();
        }
        self.was_focused = focused;
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
            #[cfg(not(target_arch = "wasm32"))]
            Some(SidebarClick::OpenReport) => {
                self.report
                    .open(&self.tools, &self.ctx, self.session_folder.as_deref())
            }
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
        // Ctrl+Q closes the window from any tool, as its close button does —
        // a text field holding the keyboard does not hold it off, the
        // first-run dialog neither: it is the shell's key, not a tool's.
        // Ctrl alone (`consume_ctrl` is the exact modifier), and the ask
        // goes through the window's own close command, which comes back as
        // the close event the button sends: every tool is asked first.
        if ui.input_mut(|input| crate::widgets::consume_ctrl(input, egui::Key::Q)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
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
            self.show_update_dialog(ui.ctx());
            #[cfg(not(target_arch = "wasm32"))]
            self.report
                .show(ui.ctx(), &self.tools, &self.ctx, self.log_file.as_deref());
            self.help_window.show(ui.ctx());
        }
    }

    /// The window's last normal place, and whether it was maximized, go into `window_main` as
    /// the window closes. eframe calls this once, on every exit path. A running update
    /// install is cancelled first and waited for, so its swap is never cut midway. It
    /// starts no program: after Restart now, `run_gui` hands the new one back.
    fn on_exit(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.update_dialog.stop_install();
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
/// window or graphics context that could not be set up. Returns the program Restart now
/// asked to start, `None` when it was not asked. The caller starts it once it has
/// released its instance lock and ended its log's session: started earlier, the new
/// program would find the lock held and open as a second copy of the program.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_gui(mut app: StudioApp) -> anyhow::Result<Option<PathBuf>> {
    let options = app.opening();
    let restart = Arc::clone(&app.update_restart);
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|creation| {
            #[cfg(windows)]
            window_size::hide_title_bar_icon(creation);
            #[cfg(windows)]
            taskbar::set_relaunch(creation);
            crate::theme::install(&creation.egui_ctx);
            // The context's clones share the wake slot, so a `with_events` sink made before
            // the window exists still wakes it.
            let egui_ctx = creation.egui_ctx.clone();
            app.ctx
                .set_wake(move |delay| egui_ctx.request_repaint_after(delay));
            Ok(Box::new(app))
        }),
    )
    .map_err(|error| anyhow::anyhow!("cannot open the window ({error})"))?;
    Ok(restart.lock().unwrap().take())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Duration;

    use crossbeam_channel::{TryRecvError, bounded, unbounded};
    use eframe::App;
    use pes_version::PesVersion;

    use super::test_harness::*;
    use super::*;
    use crate::settings::Settings;
    use crate::shell::sidebar::SETTINGS_LABEL;
    use crate::status::{Notice, NoticeKind};

    /// A hang bound on a worker's reply, never a speed bound: a passing
    /// wait returns as soon as the reply lands, and a tighter one fails a
    /// starved worker.
    const GUARD: Duration = Duration::from_secs(120);

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

    // The pane's width at the default window: 1280 less the rail's 56. The
    // draft (`docs/prototypes/index.html`, the comment starting "A tool view's
    // width at the default window") takes the centre's two 8-point margins
    // off too for the view's 1208; the pane is the width before them, which a
    // tool's view (`save_editor`) subtracts its own margins from.
    #[test]
    fn the_default_pane_width_is_the_default_window_less_the_rail() {
        assert_eq!(DEFAULT_PANE_WIDTH, 1224.0);
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

    // The context's per-version exe checks re-list on the window's regained
    // focus, the sidebar's own rule: a folder that gains its exe between two
    // asks reads Found only after the focused frame.
    #[test]
    fn the_contexts_exe_checks_are_dropped_on_the_windows_regained_focus() {
        let egui_ctx = test_context();
        let dir = TestDir::new("studio_core_shell_exe_focus");
        let (a, calls_a) = stub("a", "A", None);
        let mut settings = Settings::default();
        settings.common.pes_version = PesVersion::Pes21;
        settings.common.pes_folder_path = dir.path().display().to_string();
        let Shell { mut app, ctx, .. } = shell_with(
            settings,
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        assert_eq!(
            ctx.pes_exe_check_for(PesVersion::Pes21),
            crate::ExeCheck::NoExe,
            "an empty folder"
        );
        fs::write(dir.path().join("PES2021.exe"), b"").unwrap();
        frame(&egui_ctx, &mut app, Vec::new(), false);
        assert_eq!(
            ctx.pes_exe_check_for(PesVersion::Pes21),
            crate::ExeCheck::NoExe,
            "cached: an unfocused frame does not re-list"
        );
        frame(&egui_ctx, &mut app, Vec::new(), true);
        assert_eq!(
            ctx.pes_exe_check_for(PesVersion::Pes21),
            crate::ExeCheck::Found,
            "the regained focus re-lists"
        );
        fs::remove_file(dir.path().join("PES2021.exe")).unwrap();
        frame(&egui_ctx, &mut app, Vec::new(), true);
        assert_eq!(
            ctx.pes_exe_check_for(PesVersion::Pes21),
            crate::ExeCheck::Found,
            "a focused frame that regained nothing does not re-list"
        );
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

    /// A Ctrl+Q key press: the shell's close key, Ctrl alone.
    fn ctrl_q() -> egui::Event {
        egui::Event::Key {
            key: egui::Key::Q,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::CTRL,
        }
    }

    // ST-RUN-05
    #[test]
    fn ctrl_q_requests_the_close_as_the_close_button_does() {
        let dir = TestDir::new("studio_core_close_ctrl_q");
        let egui_ctx = test_context();
        let Shell { mut app, calls, .. } = close_shell(dir.path());
        *calls[0].unsaved.lock().unwrap() = true;
        app.set_active(2);

        // Ctrl+Q sends the window's close command; the close event it
        // becomes then takes the button's own path: the first tool that
        // reports work holds it, its prompt the active view.
        let out = frame(&egui_ctx, &mut app, vec![ctrl_q()], true);
        assert!(
            out.commands.contains(&egui::ViewportCommand::Close),
            "Ctrl+Q requests the window's close"
        );
        let out = frame_closing(&egui_ctx, &mut app);
        assert!(out.commands.contains(&egui::ViewportCommand::CancelClose));
        assert_eq!(app.active, 0, "A's prompt is the active view");
        entry(&out.nodes, "Discard");
        entry(&out.nodes, "Cancel");

        click(&egui_ctx, &mut app, entry(&out.nodes, "Cancel"));
        let out = frame(&egui_ctx, &mut app, Vec::new(), true);
        assert!(
            !out.commands.contains(&egui::ViewportCommand::Close),
            "the cancelled prompt sends no close"
        );

        // The same while a text field holds the keyboard: the settings
        // menu's Threads box focused, Ctrl+Q still asks for the close.
        let nodes = idle(&egui_ctx, &mut app, 2).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        assert!(has_text(&nodes, "Threads"), "the settings menu is open");
        click(
            &egui_ctx,
            &mut app,
            widget_on_row(&nodes, "Threads", egui::accesskit::Role::SpinButton),
        );
        frame(&egui_ctx, &mut app, Vec::new(), true);
        assert!(
            egui_ctx.egui_wants_keyboard_input(),
            "premise: the Threads box holds the keyboard"
        );
        let out = frame(&egui_ctx, &mut app, vec![ctrl_q()], true);
        assert!(
            out.commands.contains(&egui::ViewportCommand::Close),
            "Ctrl+Q asks for the close with a field holding the keyboard"
        );
        let out = frame_closing(&egui_ctx, &mut app);
        assert!(out.commands.contains(&egui::ViewportCommand::CancelClose));
        assert_eq!(app.active, 0, "A's prompt is the active view");
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        assert!(
            !has_text(&nodes, "Threads"),
            "the settings menu is gone once the close is held"
        );
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
    fn a_held_close_closes_the_about_popup_and_the_report_dialog_over_the_prompt() {
        // Each modal is seen by its own button: the About popup's Copy, the
        // report dialog's Save….
        for (button, report) in [("Copy", false), ("Save…", true)] {
            let dir = TestDir::new("studio_core_close_modal");
            let egui_ctx = test_context();
            let Shell { mut app, calls, .. } = close_shell(dir.path());
            *calls[0].unsaved.lock().unwrap() = true;
            app.set_active(1);

            // The modal open over B's view when the close is asked.
            if report {
                let nodes = idle(&egui_ctx, &mut app, 2).nodes;
                click(&egui_ctx, &mut app, entry(&nodes, sidebar::REPORT_LABEL));
            } else {
                app.sidebar.about_open = true;
            }
            let nodes = idle(&egui_ctx, &mut app, 10).nodes;
            assert!(has_label(&nodes, button), "the {button} modal is open");

            let out = frame_closing(&egui_ctx, &mut app);
            assert!(out.commands.contains(&egui::ViewportCommand::CancelClose));
            assert_eq!(app.active, 0);
            let nodes = idle(&egui_ctx, &mut app, 10).nodes;
            assert!(
                !has_label(&nodes, button),
                "the {button} modal is gone once the close is held"
            );
            // A pointer left on the rail (the report's entry) keeps the
            // sidebar's overlay open over the prompt: it leaves first, as a
            // member's would.
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
                "the prompt is uncovered and answers"
            );
        }
    }

    // ST-RUN-05
    #[test]
    fn a_held_close_closes_an_open_popup_over_the_prompt() {
        let dir = TestDir::new("studio_core_close_popup");
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

        // A popup (a menu, a dropdown) open over B's view when the close is asked.
        idle(&egui_ctx, &mut app, 2);
        egui::Popup::open_id(&egui_ctx, egui::Id::unique("the menu"));
        assert!(
            egui::Popup::is_any_open(&egui_ctx),
            "premise: a popup is open"
        );

        // The close is held: the popup would paint over A's prompt, so it goes
        // with the shell's own modals on that same frame.
        let out = frame_closing(&egui_ctx, &mut app);
        assert!(out.commands.contains(&egui::ViewportCommand::CancelClose));
        assert_eq!(app.active, 0);
        assert!(
            !egui::Popup::is_any_open(&egui_ctx),
            "the popup is closed on the frame the close is held"
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

    /// Frames until a started check's reply is drained: the poll was seen
    /// `Running` (already so at the call, or made so by a frame) and is
    /// `Idle` again, returning that frame. The bound fails a reply that never
    /// lands instead of hanging on it.
    #[cfg(not(target_arch = "wasm32"))]
    fn frame_until_drained(egui_ctx: &egui::Context, app: &mut StudioApp) -> FrameOutput {
        let deadline = std::time::Instant::now() + GUARD;
        let mut running_seen = matches!(app.update_poll, UpdatePoll::Running(_));
        loop {
            let out = frame(egui_ctx, app, Vec::new(), true);
            running_seen |= matches!(app.update_poll, UpdatePoll::Running(_));
            if running_seen && matches!(app.update_poll, UpdatePoll::Idle { .. }) {
                return out;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the update check's reply never landed"
            );
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    // ST-UPD-01
    #[test]
    fn a_due_check_offers_the_newest_release() {
        let dir = TestDir::new("studio_core_update_offer");
        let data = dir.path().join("data");
        fs::create_dir_all(&data).unwrap();
        let (a, calls_a) = stub("a", "A", None);
        let mut settings = test_settings(dir.path());
        settings.common.check_for_updates = true;
        let (gate_tx, gate_rx) = unbounded::<()>();
        let Shell { mut app, .. } =
            shell_with(settings, vec![a], &[], Some(data.clone()), vec![calls_a]);
        app.version = "0.0.1";
        // The fake fetch waits for the gate, so the worker is still fetching
        // when the first frame ends: a fetch on the UI thread would sit the
        // whole timeout out inside `frame` and come back a failed check the
        // asserts below refuse; the bound keeps that a failure, not a hang.
        app.update_fetch = std::sync::Arc::new(move || {
            if gate_rx.recv_timeout(GUARD).is_err() {
                return Err(crate::updater::UpdateError::Fetch(ureq::Error::StatusCode(
                    503,
                )));
            }
            Ok(crate::updater::releases_fixture())
        });
        let egui_ctx = test_context();
        let requested = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let delays = std::sync::Arc::clone(&requested);
        egui_ctx.set_repaint_observer(move |info| {
            delays.lock().unwrap().push(info.delay);
        });

        frame(&egui_ctx, &mut app, Vec::new(), true);
        // The frame's own requests must not stand in for the worker's: the
        // recorder reads only what is asked from here on.
        requested.lock().unwrap().clear();
        gate_tx.send(()).unwrap();
        // The send is followed by the worker's request for an immediate
        // repaint, the wake an idle window needs to drain the reply: the wait
        // is on that request, so no frame the test pumps can stand in for it.
        let deadline = std::time::Instant::now() + GUARD;
        loop {
            if requested.lock().unwrap().iter().any(Duration::is_zero) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the check's worker sent its reply without waking the window"
            );
            std::thread::yield_now();
        }
        // The offer lands with the drained reply.
        let nodes = frame_until_drained(&egui_ctx, &mut app).nodes;
        assert!(
            has_text(&nodes, "Update available: v0.0.2"),
            "the status bar offers the list's newest release"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    // ST-UPD-01
    #[test]
    fn a_due_check_finding_no_newer_release_offers_nothing() {
        for (version, skipped, why) in [
            (
                "0.0.2",
                None,
                "a release no newer than the build is not offered",
            ),
            ("0.0.1", Some("0.0.2"), "the skipped release is not offered"),
        ] {
            let dir = TestDir::new("studio_core_update_not_newer");
            let data = dir.path().join("data");
            fs::create_dir_all(&data).unwrap();
            let (a, calls_a) = stub("a", "A", None);
            let mut settings = test_settings(dir.path());
            settings.common.check_for_updates = true;
            settings.common.skipped_version = skipped.map(str::to_owned);
            let Shell { mut app, .. } =
                shell_with(settings, vec![a], &[], Some(data.clone()), vec![calls_a]);
            // The 0.0.2 case's list newest is the version the window is told
            // it runs, which is not the build's: an offer there means the
            // build's was read. The 0.0.1 case's is newer but skipped.
            app.version = version;
            app.update_fetch = std::sync::Arc::new(|| Ok(crate::updater::releases_fixture()));
            let egui_ctx = test_context();

            // Wait on the drain: before it, "no offer" holds of a check still
            // in flight.
            let nodes = frame_until_drained(&egui_ctx, &mut app).nodes;
            assert!(!has_text(&nodes, "Update available"), "{why}");
            assert!(
                app.update_release.is_none(),
                "no release is kept for the dialog ({why})"
            );
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    // ST-UPD-01
    #[test]
    fn no_check_runs_while_checks_are_off() {
        let egui_ctx = test_context();
        let dir = TestDir::new("studio_core_update_not_due");
        let (a, calls_a) = stub("a", "A", None);
        let settings = test_settings(dir.path());
        assert!(
            !settings.common.check_for_updates,
            "the premise: checks are off"
        );
        let (started_tx, started_rx) = unbounded::<()>();
        let Shell { mut app, .. } = shell_with(
            settings,
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        app.update_fetch = std::sync::Arc::new(move || {
            started_tx.send(()).expect("the test listens");
            Ok(crate::updater::releases_fixture())
        });
        idle(&egui_ctx, &mut app, 3);
        // A started check shows in the state the frame sets itself,
        // which a starved worker cannot delay.
        assert!(
            matches!(app.update_poll, UpdatePoll::Idle { .. }),
            "no check in flight"
        );
        assert!(started_rx.try_recv().is_err(), "no check starts");
    }

    #[cfg(not(target_arch = "wasm32"))]
    // ST-UPD-01
    #[test]
    fn a_failed_check_shows_no_offer() {
        let dir = TestDir::new("studio_core_update_failed");
        let data = dir.path().join("data");
        fs::create_dir_all(&data).unwrap();
        let (a, calls_a) = stub("a", "A", None);
        let mut settings = test_settings(dir.path());
        settings.common.check_for_updates = true;
        let Shell { mut app, .. } =
            shell_with(settings, vec![a], &[], Some(data.clone()), vec![calls_a]);
        app.update_fetch = std::sync::Arc::new(|| {
            Err(crate::updater::UpdateError::Fetch(ureq::Error::StatusCode(
                503,
            )))
        });
        let egui_ctx = test_context();

        // The asserts below would hold for a check still in flight, so wait
        // on the drain.
        let nodes = frame_until_drained(&egui_ctx, &mut app).nodes;
        assert!(
            !has_text(&nodes, "Update available"),
            "a failed check shows nothing"
        );
        assert!(
            app.update_release.is_none(),
            "a failed check keeps no release"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    // ST-UPD-01
    #[test]
    fn a_window_left_open_checks_again_once_the_period_passes() {
        let dir = TestDir::new("studio_core_update_again");
        let (a, calls_a) = stub("a", "A", None);
        let mut settings = test_settings(dir.path());
        settings.common.check_for_updates = true;
        let fetches = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = std::sync::Arc::clone(&fetches);
        let Shell { mut app, .. } = shell_with(
            settings,
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        app.version = "0.0.1";
        app.update_fetch = std::sync::Arc::new(move || {
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(crate::updater::releases_fixture())
        });
        // The sidebar's poll asks for its own repaint to a nearer deadline;
        // spending the once-per-life auto-switch leaves the check's the only
        // requester.
        app.sidebar.auto_switch_used = true;
        let egui_ctx = test_context();
        let requested = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let delays = std::sync::Arc::clone(&requested);
        egui_ctx.set_repaint_observer(move |info| {
            delays.lock().unwrap().push(info.delay);
        });

        // The first check runs and drains; the drain frame asks for the next
        // one's wake, about `CHECK_PERIOD` out. The five minutes are spelled
        // out so a moved `CHECK_PERIOD` moves nothing here.
        frame_until_drained(&egui_ctx, &mut app);
        let offered = app
            .update_release
            .clone()
            .expect("the first check offered v0.0.2 over the 0.0.1 build");
        let period = Duration::from_secs(5 * 60).as_secs_f64();
        assert!(
            requested
                .lock()
                .unwrap()
                .iter()
                .any(|delay| (period - 10.0..=period).contains(&delay.as_secs_f64())),
            "the drain frame wakes for the next check"
        );

        // The period passed: the next frame starts a second check, whose list
        // names the release the offer already stands on — kept, not replaced.
        app.update_poll = UpdatePoll::Idle {
            next_at: std::time::Instant::now(),
        };
        frame_until_drained(&egui_ctx, &mut app);
        assert_eq!(
            fetches.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "the second check fetched"
        );
        assert!(
            std::sync::Arc::ptr_eq(
                &offered,
                app.update_release.as_ref().expect("the offer still stands"),
            ),
            "a second offer of the same release keeps the standing one"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    // ST-UPD-01
    #[test]
    fn a_failed_check_retries_only_once_the_period_passes() {
        let dir = TestDir::new("studio_core_update_backoff");
        let (a, calls_a) = stub("a", "A", None);
        let mut settings = test_settings(dir.path());
        settings.common.check_for_updates = true;
        let fetches = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = std::sync::Arc::clone(&fetches);
        let Shell { mut app, .. } = shell_with(
            settings,
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        app.update_fetch = std::sync::Arc::new(move || {
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err(crate::updater::UpdateError::Fetch(ureq::Error::StatusCode(
                503,
            )))
        });
        let egui_ctx = test_context();

        // `before`/`after` bound the frame that drains the failure: the next
        // check is due one period after it.
        let deadline = std::time::Instant::now() + GUARD;
        let mut running_seen = false;
        let (before, after) = loop {
            running_seen |= matches!(app.update_poll, UpdatePoll::Running(_));
            let before = std::time::Instant::now();
            frame(&egui_ctx, &mut app, Vec::new(), true);
            let after = std::time::Instant::now();
            if running_seen && matches!(app.update_poll, UpdatePoll::Idle { .. }) {
                break (before, after);
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the failing check never drained"
            );
        };
        // The five minutes ST-UPD-01 pins, spelled out: `CHECK_PERIOD` itself
        // would move the bounds with the value under test.
        assert!(
            matches!(
                app.update_poll,
                UpdatePoll::Idle { next_at }
                    if (before + Duration::from_secs(5 * 60)
                        ..=after + Duration::from_secs(5 * 60))
                        .contains(&next_at)
            ),
            "the next check is one period out"
        );
        // Frames well past the drained failure: the period leaves the fetch
        // at one call rather than retrying every frame.
        for _ in 0..30 {
            frame(&egui_ctx, &mut app, Vec::new(), true);
        }
        assert_eq!(
            fetches.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "a failed check waits its period, it does not retry each frame"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    // ST-UPD-01
    #[test]
    fn an_idle_window_wakes_for_the_moment_a_check_falls_due() {
        for check_on in [true, false] {
            let dir = TestDir::new("studio_core_update_repaint");
            let (a, calls_a) = stub("a", "A", None);
            let mut settings = test_settings(dir.path());
            settings.common.check_for_updates = check_on;
            let Shell { mut app, .. } = shell_with(
                settings,
                vec![a],
                &[],
                Some(dir.path().to_path_buf()),
                vec![calls_a],
            );
            // A list holding no release, so no offer's first draw asks for
            // its own repaint: the check's wake is the only requester.
            app.update_fetch = std::sync::Arc::new(|| Ok("[]".to_owned()));
            // The sidebar's poll asks for its own repaint to a nearer
            // deadline; spending the once-per-life auto-switch leaves the
            // check's the only requester.
            app.sidebar.auto_switch_used = true;
            let egui_ctx = test_context();
            // Each request is read on its own, not the frames' merged
            // `repaint_delay`: the check's worker wakes the window right after
            // sending its reply (`updater::start_check`), and on a loaded
            // machine that wake lands during the frames below, merging them to
            // 0 s. The test sends such a wake itself, so the race runs every
            // time.
            let requested = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let delays = std::sync::Arc::clone(&requested);
            egui_ctx.set_repaint_observer(move |info| {
                delays.lock().unwrap().push(info.delay);
            });

            // A drained check leaves the poll Idle with the next one a period
            // out; the frames after it keep waking for that moment.
            if check_on {
                frame_until_drained(&egui_ctx, &mut app);
            }
            requested.lock().unwrap().clear();
            idle(&egui_ctx, &mut app, 2);
            egui_ctx.request_repaint();
            idle(&egui_ctx, &mut app, 1);
            let delays = requested.lock().unwrap();
            // The five minutes ST-UPD-01 pins, spelled out so a moved
            // `CHECK_PERIOD` moves nothing here.
            let period = Duration::from_secs(5 * 60).as_secs_f64();
            let wakes_for_the_check = delays
                .iter()
                .any(|delay| (period - 10.0..=period).contains(&delay.as_secs_f64()));
            if check_on {
                assert!(
                    wakes_for_the_check,
                    "the repaint is asked for the check's due moment, asked for {delays:?}"
                );
            } else {
                assert!(
                    !wakes_for_the_check,
                    "checks off: no due moment to wake for, asked for {delays:?}"
                );
            }
        }
    }

    // The drain frame must ask for the next check's wake-up itself: the
    // frame's merged `repaint_delay` cannot tell its request from a new
    // widget's immediate one (a found release draws the offer for the first
    // time, and egui asks for a repaint to style it), so the observer reads
    // each request's own delay.
    #[cfg(not(target_arch = "wasm32"))]
    // ST-UPD-01
    #[test]
    fn the_frame_that_drains_a_check_schedules_the_next_one() {
        for succeeds in [true, false] {
            let dir = TestDir::new("studio_core_update_drain_wake");
            let (a, calls_a) = stub("a", "A", None);
            let mut settings = test_settings(dir.path());
            settings.common.check_for_updates = true;
            let Shell { mut app, .. } = shell_with(
                settings,
                vec![a],
                &[],
                Some(dir.path().to_path_buf()),
                vec![calls_a],
            );
            app.version = "0.0.1";
            // The sidebar's poll asks for its own repaint to a nearer
            // deadline; spending the once-per-life auto-switch leaves the
            // check's the only requester.
            app.sidebar.auto_switch_used = true;
            let egui_ctx = test_context();
            let requested = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let delays = std::sync::Arc::clone(&requested);
            egui_ctx.set_repaint_observer(move |info| {
                delays.lock().unwrap().push(info.delay);
            });

            // A reply already on the channel, with no worker behind it: the
            // frame that drains it gets no `request_repaint` of the worker's
            // own, so what the frame asks for is all an idle window gets.
            let (tx, rx) = bounded(1);
            let reply: crate::updater::CheckReply = if succeeds {
                Ok(crate::updater::newest_release(&crate::updater::releases_fixture()).unwrap())
            } else {
                Err(crate::updater::UpdateError::Fetch(ureq::Error::StatusCode(
                    503,
                )))
            };
            tx.send(reply).expect("the test holds the receiver");
            app.update_poll = UpdatePoll::Running(rx);
            frame(&egui_ctx, &mut app, Vec::new(), true);

            if succeeds {
                assert!(
                    app.update_release.is_some(),
                    "the completed check offered v0.0.2"
                );
            } else {
                assert!(
                    matches!(app.update_poll, UpdatePoll::Idle { next_at }
                        if next_at > std::time::Instant::now()),
                    "the failed check's retry is a period out"
                );
            }
            let delays = requested.lock().unwrap();
            // The five minutes ST-UPD-01 pins, spelled out so a moved
            // `CHECK_PERIOD` moves nothing here.
            let period = Duration::from_secs(5 * 60).as_secs_f64();
            assert!(
                delays
                    .iter()
                    .any(|delay| (period - 10.0..=period).contains(&delay.as_secs_f64())),
                "the drain frame wakes for the next check ({succeeds}), asked for {delays:?}"
            );
        }
    }

    /// A releases list whose newest is the `0.0.3` release (its tag
    /// `TAG_PREFIX` and the semver, `v0.0.3` on main), the same shape as
    /// `updater::releases_fixture()`'s.
    #[cfg(not(target_arch = "wasm32"))]
    fn newer_fixture() -> String {
        format!(
            r#"[
            {{
                "tag_name": "{TAG_PREFIX}0.0.3",
                "body": "Newer still.\n",
                "html_url": "https://github.com/the4chancup/unlimited-tool-works/releases/tag/v0.0.3",
                "assets": []
            }}
        ]"#,
            TAG_PREFIX = crate::updater::TAG_PREFIX
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    // ST-UPD-01
    #[test]
    fn a_later_checks_newer_release_replaces_the_offer() {
        let dir = TestDir::new("studio_core_update_newer");
        let (a, calls_a) = stub("a", "A", None);
        let mut settings = test_settings(dir.path());
        settings.common.check_for_updates = true;
        let fetches = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = std::sync::Arc::clone(&fetches);
        let Shell { mut app, .. } = shell_with(
            settings,
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        app.version = "0.0.1";
        app.update_fetch = std::sync::Arc::new(move || {
            if count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                Ok(crate::updater::releases_fixture())
            } else {
                Ok(newer_fixture())
            }
        });
        let egui_ctx = test_context();

        // The first check's reply offers v0.0.2.
        let deadline = std::time::Instant::now() + GUARD;
        loop {
            let out = frame(&egui_ctx, &mut app, Vec::new(), true);
            if has_text(&out.nodes, "Update available: v0.0.2") {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the first offer never landed"
            );
        }
        // The period's end moves into the past, so the next frames run a
        // second check, whose list's newest is v0.0.3.
        app.update_poll = UpdatePoll::Idle {
            next_at: std::time::Instant::now(),
        };
        let deadline = std::time::Instant::now() + GUARD;
        let nodes = loop {
            let out = frame(&egui_ctx, &mut app, Vec::new(), true);
            if has_text(&out.nodes, "Update available: v0.0.3") {
                break out.nodes;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the newer offer never landed"
            );
        };
        assert!(
            !has_text(&nodes, "Update available: v0.0.2"),
            "the older offer is replaced"
        );
        assert_eq!(
            app.update_release
                .as_ref()
                .map(|release| release.version.to_string())
                .as_deref(),
            Some("0.0.3"),
            "the release the offer keeps is the newer one"
        );
    }

    // A check stores nothing, so a window opened right after another's check
    // owes a check of its own: the offer the command line's notice points at
    // must be there to click.
    #[cfg(not(target_arch = "wasm32"))]
    // ST-UPD-01
    #[test]
    fn a_shell_reopened_right_after_its_check_checks_again() {
        let dir = TestDir::new("studio_core_update_reopened");
        let data = dir.path().join("data");
        fs::create_dir_all(&data).unwrap();
        let (a, calls_a) = stub("a", "A", None);
        let mut settings = test_settings(dir.path());
        settings.common.check_for_updates = true;
        let fetches = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = std::sync::Arc::clone(&fetches);
        let fetch: std::sync::Arc<crate::updater::Fetch> = std::sync::Arc::new(move || {
            counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(crate::updater::releases_fixture())
        });
        let Shell { mut app, ctx, .. } =
            shell_with(settings, vec![a], &[], Some(data.clone()), vec![calls_a]);
        app.update_fetch = std::sync::Arc::clone(&fetch);
        let egui_ctx = test_context();

        // The first window runs its start check, then closes.
        frame_until_drained(&egui_ctx, &mut app);
        assert_eq!(fetches.load(std::sync::atomic::Ordering::SeqCst), 1);
        let again = ctx.settings().lock().unwrap().clone();
        drop(app);

        let (a, calls_a) = stub("a", "A", None);
        let Shell { mut app, .. } = shell_with(again, vec![a], &[], Some(data), vec![calls_a]);
        app.update_fetch = fetch;
        frame(&egui_ctx, &mut app, Vec::new(), true);
        assert!(
            matches!(app.update_poll, UpdatePoll::Running(_)),
            "the reopened window's first frame starts a check"
        );
        frame_until_drained(&egui_ctx, &mut app);
        assert_eq!(
            fetches.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "the reopened window fetched again"
        );
    }

    /// A shell starting as version 0.0.2 over the settings of `dir`, whose
    /// `last_run_version` is `stored` and whose file is not written yet, after its
    /// first frame: the app, its context and that frame's nodes.
    #[cfg(not(target_arch = "wasm32"))]
    fn started_as_0_0_2(
        dir: &TestDir,
        stored: Option<&str>,
    ) -> (StudioApp, egui::Context, Vec<egui::accesskit::Node>) {
        let (a, calls_a) = stub("a", "A", None);
        let mut settings = test_settings(dir.path());
        settings.common.last_run_version = stored.map(str::to_owned);
        let Shell { mut app, .. } = shell_with(
            settings,
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        app.version = "0.0.2";
        let egui_ctx = test_context();
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        (app, egui_ctx, nodes)
    }

    #[cfg(not(target_arch = "wasm32"))]
    // ST-UPD-06
    #[test]
    fn a_newer_versions_first_start_shows_updated_to_and_records_its_version() {
        for (stored, notice) in [
            (Some("0.0.1"), true),
            (None, false),
            (Some("0.0.2"), false),
            (Some("0.0.3"), false),
            (Some("0.0.10"), false),
            (Some("not a version"), false),
        ] {
            let dir = TestDir::new("studio_core_updated_to");
            let file = dir.path().join(crate::settings::SETTINGS_FILE_NAME);
            assert!(!file.exists(), "the premise: no settings file yet");
            let (_app, _egui_ctx, nodes) = started_as_0_0_2(&dir, stored);
            assert_eq!(
                has_text(&nodes, "Updated to v0.0.2 pre-alpha"),
                notice,
                "the notice, stored {stored:?}"
            );
            assert_eq!(
                has_text(&nodes, "Updated to"),
                notice,
                "no other version's notice, stored {stored:?}"
            );
            assert_eq!(
                Settings::load(&file)
                    .unwrap()
                    .common
                    .last_run_version
                    .as_deref(),
                Some("0.0.2"),
                "the file records the version, stored {stored:?}"
            );
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    // ST-UPD-06
    #[test]
    fn the_updated_to_notices_whats_new_opens_the_help_on_the_changelog() {
        let dir = TestDir::new("studio_core_updated_whats_new");
        let (mut app, egui_ctx, nodes) = started_as_0_0_2(&dir, Some("0.0.1"));
        assert_eq!(app.help_window.showing(), None, "the help window is closed");
        click(&egui_ctx, &mut app, entry(&nodes, "What's new"));
        idle(&egui_ctx, &mut app, 2);
        assert_eq!(app.help_window.showing(), Some(("whats-new", "changelog")));
    }
}
