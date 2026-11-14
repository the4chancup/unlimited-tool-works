//! The tool plugin interface. Tools are compile-time plugins: each tool crate implements
//! `StudioTool`, and the `studio` binary registers them in a static list. `ToolContext` is the
//! tool's one handle on the platform: settings, the event sink,
//! and requests to the shell (switch tool, notify).

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use crossbeam_channel::Sender;
use log::debug;
use pes_version::PesVersion;

use crate::events::PipelineEventEnvelope;
use crate::help::{HelpSection, HelpTarget};
use crate::repro::Recorder;
use crate::settings::{CommonSettings, Settings};
use crate::status::{Notice, ToolActivity};

/// A tool of the suite. Implemented once per tool crate, by its `Tool` type.
pub trait StudioTool {
    /// Stable identifier, used for CLI dispatch and as the settings key (`team-compiler`).
    fn id(&self) -> &'static str;

    /// Sidebar label (`Team compiler`).
    fn label(&self) -> &'static str;

    /// The tool's icon glyph (`crate::icons`), drawn at 24 points in the
    /// sidebar's rail and its opened overlay.
    fn icon(&self) -> &'static str;

    /// The tool's main view, rendered in the panel right of the tool selector.
    fn view(&mut self, ui: &mut egui::Ui, ctx: &ToolContext);

    /// Per-frame background work, called for every registered tool each frame, active view or
    /// not: drain event channels, check timer deadlines. Default no-op.
    fn tick(&mut self, _ctx: &ToolContext) {}

    /// Whether `settings_view` draws a form: the settings menu draws the tool's section only
    /// when this is true. A non-empty `default_settings()` is not the test, because the table
    /// also holds state the tool writes for itself (a last folder, recent files), and a header
    /// over nothing is a dead end. Default: false.
    fn has_settings_view(&self) -> bool {
        false
    }

    /// The tool's settings section, injected into the settings menu. The menu draws the
    /// section only for a tool whose `has_settings_view()` is true; the tool writes its
    /// table through `ctx.set_tool_settings`. Default: nothing drawn.
    fn settings_view(&mut self, _ui: &mut egui::Ui, _ctx: &ToolContext) {}

    /// Defaults merged into the tool's settings table for every key it lacks, on first run and
    /// when a new version adds a key.
    fn default_settings(&self) -> toml::Table;

    /// The tool's chapter of the help window: its prose topics, and `messages`, the catalog
    /// rows the generated Messages topic is rendered from.
    fn help(&self) -> HelpSection;

    /// The tool's CLI subcommand. Its name must equal `id()`; the shell enforces it.
    fn cli_command(&self) -> clap::Command;

    /// Runs the already-parsed CLI subcommand headless. `Ok` carries the process exit code of a
    /// command that reached a verdict (its findings went out as events); `Err` is a command
    /// refused or failed before one, printed by the binary as `error: …`.
    fn cli_run(&self, matches: &clap::ArgMatches, ctx: &ToolContext) -> Result<u8, CliError>;

    /// GUI autorun for `UTW --gui <tool-id> <command>`: performs the parsed command inside the
    /// GUI as if the user had pressed the corresponding button. Called once after the tool's view
    /// exists; the tool may queue the action until its own readiness condition holds. Default: no
    /// command is GUI-runnable, so the launch fails with a message instead of silently opening.
    fn gui_run(&mut self, _matches: &clap::ArgMatches, _ctx: &ToolContext) -> anyhow::Result<()> {
        anyhow::bail!("{} has no GUI-runnable commands", self.id())
    }

    /// Whether the tool works with the PES version the sidebar's selector holds:
    /// `true` for a tool whose work is for the selected game (the Team compiler
    /// compiles for it, the Export upgrader upgrades into it). The selector shows
    /// the version only while the active tool does; under one it governs nothing
    /// in (the Save editor follows its open save's version, a ruleset names no
    /// game) it stands disabled and empty. Default: false.
    fn targets_pes_version(&self) -> bool {
        false
    }

    /// A short reason the global PES version cannot change right now (the match tracker holding
    /// live memory addresses), shown as the selector's disabled-state tooltip. Default: none.
    fn version_change_blocker(&self) -> Option<&str> {
        None
    }

    /// Called when the window's close is requested, in sidebar order until one tool returns
    /// `true`. A tool with unsaved work or a run in flight returns `true` and opens its own
    /// prompt in its view, which the shell makes the active one; the tools after it are not
    /// asked. The prompt answers by calling `ToolContext::request_close` once the work is
    /// saved or discarded, or once the run has ended after the member chose to close when it
    /// ends. Or it does nothing, which keeps the window and ends the close. Default: nothing
    /// holds the close.
    fn hold_close(&mut self) -> bool {
        false
    }

    /// What the tool is doing in the background, for the status bar and the window title.
    /// Default: nothing.
    fn activity(&self) -> Option<ToolActivity> {
        None
    }

    /// What the tool has open, shown in square brackets after its name in the idle window
    /// title (`Save editor [C:\…\EDIT00000000] - <app>`): the path of the one file it edits.
    /// Not in the busy or notice titles. Default: nothing.
    fn title_detail(&self) -> Option<String> {
        None
    }
}

/// A suite tool a build does not register. `studio` keeps
/// the full registry as data and hands it to the shell, which lists every entry whose `id` no
/// registered tool has, grayed after the registered ones, as the roadmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedTool {
    /// The tool's id once it is built (`stadium-compiler`).
    pub id: &'static str,
    /// Sidebar label (`Stadium compiler`).
    pub label: &'static str,
    /// The rail's icon glyph (`crate::icons`); the planned entry repeats what the tool's
    /// `icon()` will return.
    pub icon: &'static str,
}

/// A CLI command refused or failed before it reached a verdict.
#[derive(Debug, thiserror::Error)]
#[error("{error:#}")]
pub struct CliError {
    /// The process exit code: the tool's own mapping (the Team compiler's 2 or 3).
    pub exit_code: u8,
    /// What went wrong, printed as `error: …`.
    pub error: anyhow::Error,
}

impl CliError {
    /// A refusal with the tool's exit code for it.
    pub fn new(exit_code: u8, error: impl Into<anyhow::Error>) -> CliError {
        CliError {
            exit_code,
            error: error.into(),
        }
    }
}

/// The base folders relative path settings resolve against
/// (spec/team_compiler/settings.md "Path resolution").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    /// The folder holding the executable.
    pub exe_dir: PathBuf,
    /// The selected data directory; `None` when no settings file exists yet.
    pub data_dir: Option<PathBuf>,
}

/// Something a tool asks the shell to do at the end of the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellRequest {
    /// Make the named tool the active view.
    SwitchTool(String),
    /// Show a notice in the status bar.
    Notify(Notice),
    /// A tool changed its settings section; save the file (best-effort).
    SettingsChanged,
    /// Open the help window at the target (a log line's message-code link).
    OpenHelp(HelpTarget),
    /// Close the window: a close prompt's answer. The shell sends it back through the
    /// window's own close path, which asks every tool again.
    Close,
}

/// What the sidebar's exe check found at its last listing of the resolved PES folder
/// (ST-SIDE-03): the PES selector is red on `OtherVersion`, amber on `NoExe` and
/// `NoFolder`, uncolored on `Found`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExeCheck {
    /// The resolved PES folder holds the selected version's exe.
    Found,
    /// It exists but holds a different version's exe (wrong selection or wrong path).
    OtherVersion(PesVersion),
    /// It exists but holds no `PES20NN.exe` at all.
    NoExe,
    /// It does not exist (or cannot be listed).
    NoFolder,
}

/// The GUI's repaint waker: `run_gui` installs
/// `egui::Context::request_repaint_after` once the window exists, so a run's `emit` or a tool's
/// own deadline reaches a frame while no input arrives. egui stays out of this type, which the
/// CLI never fills.
struct Wake(Box<dyn Fn(Duration) + Send + Sync>);

// The boxed closure has no `Debug`.
impl fmt::Debug for Wake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Wake(..)")
    }
}

/// The tool's handle on the platform. Cheap to clone; every clone shares the same settings,
/// channels and wake slot — a `with_events` clone made before the window exists still wakes it.
#[derive(Debug, Clone)]
pub struct ToolContext {
    settings: Arc<Mutex<Settings>>,
    paths: AppPaths,
    events: Sender<PipelineEventEnvelope>,
    requests: Sender<ShellRequest>,
    /// The repaint waker, shared by every clone and filled once, when the window is created.
    wake: Arc<OnceLock<Wake>>,
    /// The sidebar's latest exe check, shared by every clone; `None` until the sidebar's
    /// first frame, and always in the CLI.
    pes_exe_check: Arc<Mutex<Option<ExeCheck>>>,
    /// The per-version exe checks `pes_exe_check_for` fills, each entry keyed by the
    /// folder the settings last resolved for the version: a version's folder is listed
    /// again once it resolves elsewhere, and `clear_pes_exe_checks` drops the map for
    /// the window's regained focus.
    exe_checks: Arc<Mutex<BTreeMap<PesVersion, (PathBuf, ExeCheck)>>>,
    /// The repro journal every tool's model steps record into; every clone
    /// shares it, so the bundle covers the whole session.
    recorder: Recorder,
}

impl ToolContext {
    /// Builds a context over the shell's settings, the app's base folders and its two receiving
    /// channels.
    pub fn new(
        settings: Arc<Mutex<Settings>>,
        paths: AppPaths,
        events: Sender<PipelineEventEnvelope>,
        requests: Sender<ShellRequest>,
    ) -> Self {
        ToolContext {
            settings,
            paths,
            events,
            requests,
            wake: Arc::new(OnceLock::new()),
            pes_exe_check: Arc::new(Mutex::new(None)),
            exe_checks: Arc::new(Mutex::new(BTreeMap::new())),
            recorder: Recorder::default(),
        }
    }

    /// The same handle (settings, paths, shell requests) with its events sent to `events`. A
    /// tool whose view shows a run's events gives the run its own sink this way and drains the
    /// receiver in `tick`: an envelope names no tool, so events routed through the shell's
    /// receiver could not be told apart by tool.
    pub fn with_events(&self, events: Sender<PipelineEventEnvelope>) -> ToolContext {
        ToolContext {
            events,
            ..self.clone()
        }
    }

    /// The two base folders relative path settings resolve against: the executable's folder and
    /// the data directory (`None` before a settings file exists, as in a CLI run ahead of the
    /// first GUI start). Tools never see the settings file itself.
    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    /// Sets the data directory the first-run dialog's choice wrote. Crate-private, called
    /// once by the shell before any tool's `view` has
    /// run: `paths` is a per-clone copy, so a `ToolContext` cloned earlier (a run's
    /// `with_events` sink) keeps the copy it was made with. The browser build asks no
    /// data-location question and has no caller for it.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn set_data_dir(&mut self, dir: PathBuf) {
        self.paths.data_dir = Some(dir);
    }

    /// The repro journal's recorder: a tool records every step that
    /// changes its model and keeps the files it read and wrote, so the
    /// bundle "Report a problem" writes can replay the session. Clone it
    /// to hand it around; every clone writes the one journal.
    pub fn recorder(&self) -> &Recorder {
        &self.recorder
    }

    /// A copy of the common settings as they are now.
    pub fn common(&self) -> CommonSettings {
        self.settings.lock().unwrap().common.clone()
    }

    /// A copy of the tool's own settings table; empty if the file has no section for it yet.
    pub fn tool_settings(&self, tool_id: &str) -> toml::Table {
        self.settings
            .lock()
            .unwrap()
            .tool(tool_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Replaces the tool's settings table and asks the shell to save.
    pub fn set_tool_settings(&self, tool_id: &str, table: toml::Table) {
        self.settings.lock().unwrap().set_tool(tool_id, table);
        self.request(ShellRequest::SettingsChanged);
    }

    /// What the PES selector's exe check found in the PES folder, as the sidebar drew it
    /// this frame (it draws before the active view): a tool that needs the folder to hold
    /// the selected game reads this verdict rather than listing the folder itself, so it
    /// never disagrees with the selector's colour. `None` before the sidebar's first frame
    /// and in the CLI.
    pub fn pes_exe_check(&self) -> Option<ExeCheck> {
        *self.pes_exe_check.lock().unwrap()
    }

    /// Stores the exe check the sidebar's PES selector was coloured by this frame.
    /// Crate-private: the sidebar alone runs the check.
    pub(crate) fn set_pes_exe_check(&self, check: ExeCheck) {
        *self.pes_exe_check.lock().unwrap() = Some(check);
    }

    /// The exe check's verdict for `version`: what `check_pes_folder` finds in the PES
    /// folder the settings resolve for it (`CommonSettings::pes_folder_for`), the same
    /// listing the sidebar's selector is coloured by, so a tool that works a file of one
    /// version asks that version's folder's verdict here rather than the selector's. An
    /// entry is kept per version keyed by the resolved folder and listed again when the
    /// resolution changes, never per ask; `clear_pes_exe_checks` drops the map. Unlike
    /// `pes_exe_check`, which waits for the sidebar, the ask runs the listing itself on
    /// a miss, so there is always a verdict.
    pub fn pes_exe_check_for(&self, version: PesVersion) -> ExeCheck {
        let folder = self.settings.lock().unwrap().common.pes_folder_for(version);
        let mut checks = self.exe_checks.lock().unwrap();
        if let Some((resolved, check)) = checks.get(&version)
            && *resolved == folder
        {
            return *check;
        }
        let check = crate::pes_process::check_pes_folder(version, &folder);
        checks.insert(version, (folder, check));
        check
    }

    /// Drops every per-version exe check so the next ask lists its folder again. The
    /// shell calls it when the window regains focus, the trigger the sidebar's own
    /// check re-lists on, so a folder repaired on disk is seen by the two alike.
    pub(crate) fn clear_pes_exe_checks(&self) {
        self.exe_checks.lock().unwrap().clear();
    }

    /// Points the settings at a PES folder the member picked for `version`, as the
    /// running-PES notice's Fix PES path does: that version's own entry, the
    /// folder as it is, once any version has one; otherwise the main row, with the year
    /// in the folder's name written `20**`. Then asks the shell to save. The version's
    /// cached exe check is dropped, so the next ask lists the folder again even when
    /// the pick resolves to the folder already checked: a member who repaired it and
    /// picked it once more would otherwise keep the old verdict until the window's
    /// next regained focus.
    pub fn set_pes_folder_for(&self, folder: &Path, version: PesVersion) {
        self.settings
            .lock()
            .unwrap()
            .common
            .fix_pes_folder(folder, version);
        self.exe_checks.lock().unwrap().remove(&version);
        self.request(ShellRequest::SettingsChanged);
    }

    /// The settings behind `common` and `tool_settings`, for the shell's own writes (the
    /// sidebar's `pes_version` and `last_tool`, the settings menu). A write is followed by a
    /// `ShellRequest::SettingsChanged`, which has the shell save the file at the next frame.
    pub(crate) fn settings(&self) -> &Arc<Mutex<Settings>> {
        &self.settings
    }

    /// Installs the repaint waker; `run_gui` calls it inside the window's creator closure, on
    /// the context it already holds. A second call is a no-op. Crate-private: tools never
    /// install it.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn set_wake(&self, wake: impl Fn(Duration) + Send + Sync + 'static) {
        if self.wake.set(Wake(Box::new(wake))).is_err() {
            debug!("a repaint wake is already installed");
        }
    }

    /// Asks the GUI for a frame after `delay`: a tool's own deadlines (timers it checks in
    /// `tick`) call this so they fire while another tool is in view or no input arrives. A
    /// no-op in the CLI, where no wake is installed.
    pub fn request_repaint_after(&self, delay: Duration) {
        if let Some(wake) = self.wake.get() {
            (wake.0)(delay);
        }
    }

    /// Sends a pipeline event. A receiver that has gone away (the CLI printer finished, the GUI
    /// closed) is not an error for the sender, so the failure is logged here, once, and dropped.
    /// A sent event wakes the GUI, so the frame that drains it need not wait for input.
    pub fn emit(&self, envelope: PipelineEventEnvelope) {
        if self.events.send(envelope).is_err() {
            debug!("pipeline event dropped: no receiver");
            return;
        }
        self.request_repaint_after(Duration::ZERO);
    }

    /// Asks the shell to make the named tool the active view at the end of the frame. Id-based,
    /// so tools can link to each other without depending on each other's crates.
    pub fn switch_to_tool(&self, tool_id: &str) {
        self.request(ShellRequest::SwitchTool(tool_id.to_owned()));
    }

    /// Publishes a notice to the status bar: the one way a tool puts something there. The send
    /// wakes the GUI, so a notice published from a run's own thread is drawn while no input
    /// arrives (and reaches the window title while the window is unfocused).
    pub fn notify(&self, notice: Notice) {
        self.request(ShellRequest::Notify(notice));
    }

    /// Asks the shell to open the help window at `target`: the way a tool's view opens
    /// it, a log line's message-code link being the case it exists for. The shell
    /// closes a modal first, so it cannot
    /// cover the window. The send wakes the GUI, as every request does.
    pub fn open_help(&self, target: HelpTarget) {
        self.request(ShellRequest::OpenHelp(target));
    }

    /// Asks the shell to close the window: the way a tool's close prompt answers. The close
    /// goes back through the window's own close path, so the
    /// tools are asked again and another one may still hold. The send wakes the GUI, as
    /// every request does.
    pub fn request_close(&self) {
        self.request(ShellRequest::Close);
    }

    /// Queues a shell request for the end of the frame. Crate-private: the tools' ways in are
    /// the methods above; the shell's own widgets (the sidebar) send theirs through this. A
    /// sent request wakes the GUI like a sent event does.
    pub(crate) fn request(&self, request: ShellRequest) {
        if self.requests.send(request).is_err() {
            debug!("shell request dropped: no receiver");
            return;
        }
        self.request_repaint_after(Duration::ZERO);
    }
}

#[cfg(test)]
mod tests {
    use crossbeam_channel::{Receiver, unbounded};

    use super::*;
    use crate::events::{PipelineEvent, RunId};
    use crate::repro::tests::{journal_doc, scratch, zip_text};
    use crate::status::NoticeKind;

    /// A context over fresh channels, with the receivers it sends to.
    fn context() -> (
        ToolContext,
        Receiver<PipelineEventEnvelope>,
        Receiver<ShellRequest>,
    ) {
        let (events_tx, events_rx) = unbounded();
        let (requests_tx, requests_rx) = unbounded();
        let ctx = ToolContext::new(
            Arc::new(Mutex::new(Settings::default())),
            AppPaths {
                exe_dir: PathBuf::from("exe"),
                data_dir: None,
            },
            events_tx,
            requests_tx,
        );
        (ctx, events_rx, requests_rx)
    }

    fn envelope() -> PipelineEventEnvelope {
        PipelineEventEnvelope {
            run_id: RunId(1),
            export_id: None,
            export_revision: None,
            made_at: None,
            event: PipelineEvent::UnmigratedContentFound,
        }
    }

    #[test]
    fn common_returns_the_settings_common_section_as_it_is() {
        let mut settings = Settings::default();
        settings.common.thread_count = 4;
        let (events_tx, _events_rx) = unbounded();
        let (requests_tx, _requests_rx) = unbounded();
        let ctx = ToolContext::new(
            Arc::new(Mutex::new(settings)),
            AppPaths {
                exe_dir: PathBuf::from("exe"),
                data_dir: None,
            },
            events_tx,
            requests_tx,
        );
        assert_eq!(ctx.common().thread_count, 4);
    }

    #[test]
    fn with_events_sends_events_to_the_new_receiver_only() {
        let (ctx, events_rx, _requests_rx) = context();
        let (own_tx, own_rx) = unbounded();
        ctx.with_events(own_tx).emit(envelope());
        assert_eq!(own_rx.try_recv().unwrap(), envelope());
        assert!(
            events_rx.try_recv().is_err(),
            "the original receiver got nothing"
        );
    }

    #[test]
    fn with_events_keeps_the_shell_requests_and_the_settings() {
        let (ctx, _events_rx, requests_rx) = context();
        let (own_tx, _own_rx) = unbounded();
        let run_ctx = ctx.with_events(own_tx);
        run_ctx.switch_to_tool("stub");
        assert_eq!(
            requests_rx.try_recv().unwrap(),
            ShellRequest::SwitchTool("stub".to_owned())
        );
        let mut table = toml::Table::new();
        table.insert("key".to_owned(), toml::Value::Integer(1));
        run_ctx.set_tool_settings("stub", table.clone());
        assert_eq!(ctx.tool_settings("stub"), table);
        assert_eq!(
            requests_rx.try_recv().unwrap(),
            ShellRequest::SettingsChanged
        );
    }

    #[test]
    fn emit_on_a_with_events_clone_wakes_the_wake_installed_on_the_original() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (ctx, _events_rx, _requests_rx) = context();
        let calls = Arc::new(AtomicUsize::new(0));
        let delays = Arc::new(Mutex::new(Vec::new()));
        ctx.set_wake({
            let calls = Arc::clone(&calls);
            let delays = Arc::clone(&delays);
            move |delay| {
                calls.fetch_add(1, Ordering::SeqCst);
                delays.lock().unwrap().push(delay);
            }
        });
        // A second install is a no-op.
        ctx.set_wake(|_| panic!("the first wake stays"));

        // The emit goes through a `with_events` clone, as a run's own sink is.
        let (own_tx, _own_rx) = unbounded();
        let run_ctx = ctx.with_events(own_tx);
        run_ctx.emit(envelope());
        assert_eq!(calls.load(Ordering::SeqCst), 1, "the emit woke the GUI");
        run_ctx.request_repaint_after(Duration::from_secs(2));
        assert_eq!(
            delays.lock().unwrap().as_slice(),
            [Duration::ZERO, Duration::from_secs(2)],
            "a send wakes at once, a deadline with its delay"
        );
    }

    #[test]
    fn notify_on_a_with_events_clone_wakes_the_wake_installed_on_the_original() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (ctx, _events_rx, _requests_rx) = context();
        let calls = Arc::new(AtomicUsize::new(0));
        let delays = Arc::new(Mutex::new(Vec::new()));
        ctx.set_wake({
            let calls = Arc::clone(&calls);
            let delays = Arc::clone(&delays);
            move |delay| {
                calls.fetch_add(1, Ordering::SeqCst);
                delays.lock().unwrap().push(delay);
            }
        });
        // The notice goes through a `with_events` clone, as a run's own context publishes it.
        let (own_tx, _own_rx) = unbounded();
        ctx.with_events(own_tx).notify(Notice {
            text: "Compiled 48 teams".to_owned(),
            kind: NoticeKind::Info,
            action: None,
        });
        assert_eq!(calls.load(Ordering::SeqCst), 1, "the notice woke the GUI");
        assert_eq!(delays.lock().unwrap().as_slice(), [Duration::ZERO]);
    }

    // A picked PES folder lands where Fix PES path puts one, for the version named:
    // the main row with the year written `20**` while no version has its own entry, the
    // version's own entry, the folder as given, once one does; each write asks for a save.
    #[test]
    fn set_pes_folder_for_writes_the_versions_folder_and_asks_for_a_save() {
        let dir = Path::new("/games");
        let picked = dir.join("Pro Evolution Soccer 2017");

        let (ctx, _events_rx, requests_rx) = context();
        {
            let mut settings = ctx.settings().lock().unwrap();
            settings.common.pes_version = PesVersion::Pes17;
            settings.common.pes_folder_path = "/member/set/folder".to_owned();
        }
        ctx.set_pes_folder_for(&picked, PesVersion::Pes17);
        assert_eq!(
            PathBuf::from(ctx.common().pes_folder_path),
            dir.join("Pro Evolution Soccer 20**"),
            "no entries: the main row takes the folder with its year as 20**"
        );
        assert!(ctx.common().pes_folders.is_empty(), "no entry appeared");
        assert_eq!(
            requests_rx.try_recv().unwrap(),
            ShellRequest::SettingsChanged
        );

        let (ctx, _events_rx, requests_rx) = context();
        {
            let mut settings = ctx.settings().lock().unwrap();
            settings.common.pes_version = PesVersion::Pes17;
            settings.common.pes_folder_path = "/member/set/folder".to_owned();
            settings
                .common
                .pes_folders
                .insert(PesVersion::Pes21, "/elsewhere/PES2021".to_owned());
        }
        ctx.set_pes_folder_for(&picked, PesVersion::Pes17);
        let common = ctx.common();
        assert_eq!(
            common
                .pes_folders
                .get(&PesVersion::Pes17)
                .map(PathBuf::from),
            Some(picked),
            "an entry exists: PES 17's own entry takes the folder as given"
        );
        assert_eq!(
            common.pes_folder_path, "/member/set/folder",
            "the main row unchanged"
        );
        assert_eq!(
            requests_rx.try_recv().unwrap(),
            ShellRequest::SettingsChanged
        );
    }

    // A picked folder lands under the version named, wherever the selector
    // stands: the main row takes PES 17's pick as `20**` with the selector on
    // PES 19, and PES 19's setting is unmoved.
    #[test]
    fn set_pes_folder_for_stores_the_picked_folder_for_its_version() {
        let (ctx, _events_rx, requests_rx) = context();
        ctx.settings().lock().unwrap().common.pes_version = PesVersion::Pes19;
        let picked = Path::new("/games/Pro Evolution Soccer 2017");
        ctx.set_pes_folder_for(picked, PesVersion::Pes17);
        assert_eq!(
            PathBuf::from(ctx.common().pes_folder_path),
            PathBuf::from("/games/Pro Evolution Soccer 20**"),
            "the main row holds the pick with its year as 20**"
        );
        assert_eq!(
            ctx.common().pes_version,
            PesVersion::Pes19,
            "the selector unmoved"
        );
        assert_eq!(
            requests_rx.try_recv().unwrap(),
            ShellRequest::SettingsChanged
        );
    }

    // A pick lists its version's folder again even when it resolves to the folder
    // the cached verdict was for: the member repaired it and picked it once more.
    #[test]
    fn a_picked_folder_is_listed_again_though_it_resolves_unchanged() {
        let dir = scratch("tool_exe_check_repick");
        let folder = dir.join("PES 2017");
        std::fs::create_dir_all(&folder).unwrap();
        let (ctx, _events_rx, _requests_rx) = context();
        ctx.set_pes_folder_for(&folder, PesVersion::Pes17);
        assert_eq!(
            ctx.pes_exe_check_for(PesVersion::Pes17),
            ExeCheck::NoExe,
            "an empty folder"
        );
        std::fs::write(folder.join("PES2017.exe"), b"").unwrap();
        assert_eq!(
            ctx.pes_exe_check_for(PesVersion::Pes17),
            ExeCheck::NoExe,
            "premise: an ask alone keeps the cached verdict"
        );
        ctx.set_pes_folder_for(&folder, PesVersion::Pes17);
        assert_eq!(
            ctx.pes_exe_check_for(PesVersion::Pes17),
            ExeCheck::Found,
            "the pick listed the same folder again"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // `pes_exe_check_for` answers each version's own folder's verdict (PES 17's
    // folder holds its exe, PES 19's holds another version's), re-lists a version
    // when the settings resolve a new folder for it, and lists a folder that
    // gained its exe only after the cache's drop, never on the next ask.
    #[test]
    fn pes_exe_check_for_checks_each_versions_own_folder() {
        let dir = scratch("tool_exe_check_for");
        let games = dir.join("games");
        std::fs::create_dir_all(games.join("PES 2017")).unwrap();
        std::fs::write(games.join("PES 2017/PES2017.exe"), b"").unwrap();
        let (ctx, _events_rx, _requests_rx) = context();
        ctx.settings().lock().unwrap().common.pes_folder_path =
            games.join("PES 20**").display().to_string();

        assert_eq!(
            ctx.pes_exe_check_for(PesVersion::Pes17),
            ExeCheck::Found,
            "PES 17's folder holds PES2017.exe"
        );
        assert_eq!(
            ctx.pes_exe_check_for(PesVersion::Pes19),
            ExeCheck::NoFolder,
            "PES 19's folder does not exist, whatever the selector's PES 19"
        );

        // A version's resolved folder changing re-lists it: the entry given
        // PES 19 names the folder PES2017.exe sits in.
        ctx.settings().lock().unwrap().common.pes_folders.insert(
            PesVersion::Pes19,
            games.join("PES 2017").display().to_string(),
        );
        assert_eq!(
            ctx.pes_exe_check_for(PesVersion::Pes19),
            ExeCheck::OtherVersion(PesVersion::Pes17),
            "PES 19's new folder holds another version's exe"
        );

        // A folder gaining its exe between two asks reads Found only after the
        // cache's drop (the window's regained focus): the next ask alone keeps
        // the cached verdict.
        let pes21 = games.join("PES 2021");
        std::fs::create_dir_all(&pes21).unwrap();
        assert_eq!(
            ctx.pes_exe_check_for(PesVersion::Pes21),
            ExeCheck::NoExe,
            "an empty folder"
        );
        std::fs::write(pes21.join("PES2021.exe"), b"").unwrap();
        assert_eq!(
            ctx.pes_exe_check_for(PesVersion::Pes21),
            ExeCheck::NoExe,
            "cached: the ask does not re-list"
        );
        ctx.clear_pes_exe_checks();
        assert_eq!(
            ctx.pes_exe_check_for(PesVersion::Pes21),
            ExeCheck::Found,
            "the drop re-lists"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_context_with_a_wake_debug_formats_the_wake_slot() {
        let (ctx, _events_rx, _requests_rx) = context();
        ctx.set_wake(|_| {});
        let debug = format!("{ctx:?}");
        assert!(debug.contains("Wake(..)"), "the wake is named: {debug}");
    }

    #[test]
    fn without_a_wake_emit_and_repaint_requests_do_nothing() {
        let (ctx, events_rx, _requests_rx) = context();
        ctx.emit(envelope());
        assert!(
            events_rx.try_recv().is_ok(),
            "the event still reaches its receiver"
        );
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    // A step recorded through a clone lands in the one journal the
    // original's recorder writes: every `recorder()` hands out the same
    // one, so the bundle covers the whole session.
    #[test]
    fn every_clones_recorder_writes_the_one_journal() {
        let (ctx, _events_rx, _requests_rx) = context();
        ctx.recorder()
            .step("save-editor", "a step through the context");
        ctx.clone()
            .recorder()
            .step("save-editor", "a step through a clone");

        let out = scratch("tool_recorder").join("repro.zip");
        ctx.recorder()
            .write_bundle(&out, "", &[], &[], None)
            .unwrap();

        let journal = zip_text(&out, "journal.toml");
        let doc = journal_doc(&journal);
        let steps = doc["step"].as_array().unwrap();
        assert_eq!(
            steps
                .iter()
                .map(|step| step["what"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["a step through the context", "a step through a clone"],
            "{journal}"
        );
    }
}
