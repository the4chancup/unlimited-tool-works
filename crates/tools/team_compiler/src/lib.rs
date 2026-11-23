//! The Compiler: Studio-format aesthetics exports compiled to CPK archives.
//!
//! `Tool` registers the tool with the shell: its window (`view`, over the run log in
//! `gui_run`), its settings section, its help chapter and its `compiler` command line,
//! whose preflight refuses an invalid invocation or configuration. `check` runs validation (the
//! structure pass, then the deep pass over file contents) on every export; `compile` runs the
//! same validation, plans the kept exports' face and kit tasks, processes them and writes the
//! CPK; `upgrade-dpfl` installs the cup's official `DpFileList.bin` in the game.

mod bins;
mod check;
mod cli;
mod compile;
mod deep;
mod events;
mod face_diff;
mod face_xml;
mod gui_run;
mod kit_variants;
mod messages;
mod mtl_search;
mod output;
mod paths;
mod plan;
mod processing;
mod reader;
mod settings;
mod templates;
#[cfg(any(test, feature = "test-support"))]
pub mod test_files;
#[cfg(test)]
mod testing;
mod texture_lookup;
mod upgrade;
mod user_face_xml;
mod validation;
mod view;

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use studio_core::{
    CliError, CommonSettings, HelpSection, HelpTopic, LogPanel, RunningPes, StudioTool, Theme,
    ToolContext,
};

use crate::check::live::{LiveCheck, Polled};
use crate::gui_run::RunLog;
use crate::messages::TOOL_ID;
use crate::view::{DpflDialog, GridDetail, Intent};

/// The Compiler, as registered with the `studio` binary. Its window state is
/// the run log — the `compile` the Compile button started, and its lines — the
/// grid's check and its rows, the grid's detail window, and the log panel's own
/// view state.
pub struct Tool {
    run_log: RunLog,
    live_check: LiveCheck,
    detail: GridDetail,
    log_panel: LogPanel,
    /// The settings the grid's check last started with, the common ones and the tool's
    /// table: the rows, the toolbar's buttons and an open dialog all describe what a
    /// check found under them, so a change to either checks the grid again.
    checked_settings: Option<(CommonSettings, toml::Table)>,
    /// The running-PES probe the savefile step asks: the real process list in
    /// production, a stub reporting no game under `cargo test` (`running_probe`),
    /// the in-process command-line tests' probe through `with_running_probe`.
    running: fn() -> Vec<RunningPes>,
}

impl Tool {
    /// The tool with no run started. The log panel's egui state keys on the tool id.
    pub fn new() -> Tool {
        Tool::with_running_probe(running_probe())
    }

    /// The tool with `running` as its running-PES probe in place of the real process list: the
    /// in-process command-line tests' probe (`spec/team_compiler/README.md` TC-SAV-17), so no test
    /// reads the machine's processes.
    pub fn with_running_probe(running: fn() -> Vec<RunningPes>) -> Tool {
        Tool {
            run_log: RunLog::default(),
            live_check: LiveCheck::default(),
            detail: GridDetail::new(),
            log_panel: LogPanel::new(TOOL_ID),
            checked_settings: None,
            running,
        }
    }

    /// The Upgrade DpFileList button's click: `upgrade`'s plan for `download/` (found as the
    /// command finds it), read once now, opens the dialog on it, unless the list is already
    /// the official one with every CPK it names there (`up_to_date`, the member having fixed
    /// it by hand or through the command line), when the upgrade runs at once with nothing
    /// deleted. The folder the plan was made for goes with the dialog and the run, since the
    /// settings may have changed while the dialog was open. A plan that cannot be made,
    /// `download/` missing or a file it cannot read, is a line in the log as a refused
    /// run's is, and opens nothing.
    fn open_upgrade(&mut self, ctx: &ToolContext) {
        let preview = cli::download_folder(&ctx.common())
            .map_err(|error| error.to_string())
            .and_then(|download| {
                upgrade::preview(&download, ctx)
                    .map(|preview| (download, preview))
                    .map_err(|error| format!("{error:#}"))
            });
        match preview {
            Ok((download, preview)) if preview.up_to_date => {
                self.run_log.start_upgrade(download, BTreeSet::new(), ctx)
            }
            Ok((download, preview)) => {
                self.detail.dpfl_dialog = Some(DpflDialog::new(preview, download))
            }
            Err(error) => self.run_log.append_refusal(&error),
        }
    }

    /// Starts the grid's check and keeps the settings it started with (`checked_settings`).
    fn start_check(&mut self, ctx: &ToolContext) {
        self.checked_settings = Some(check_settings(ctx));
        self.live_check.start(ctx);
    }

    /// The Compile menu's Open output folder: `create_output_folder`'s folder opened in
    /// the file manager; a folder that cannot be resolved or created is a line in the
    /// log, as a refused run's is.
    fn open_output_folder(&mut self, ctx: &ToolContext) {
        match create_output_folder(ctx) {
            Ok(folder) => studio_core::files::open_folder(&folder),
            Err(error) => self.run_log.append_refusal(&error),
        }
    }
}

/// `ctx`'s output folder, `cli::output_folder`'s, created with `fs::create_dir_all` when
/// no run has made it yet: a file manager asked for a missing folder opens another one
/// instead. The path, or the error's text as `append_refusal` prints it.
pub(crate) fn create_output_folder(ctx: &ToolContext) -> Result<PathBuf, String> {
    let folder = cli::output_folder(ctx).map_err(|error| error.to_string())?;
    fs::create_dir_all(&folder).map_err(|error| {
        format!(
            "cannot create the output folder {}: {error}",
            folder.display()
        )
    })?;
    Ok(folder)
}

/// The settings the grid's check is compared against (`checked_settings`): `ctx`'s common
/// ones with the fields only the window and the updater read reset to fixed values
/// (`theme`, `invert_number_scrolling`, `check_for_updates`, `skipped_version`,
/// `last_tool`, `last_run_version`, `window_main`), and the tool's table whole.
/// `last_tool` matters because the shell writes it at every tool switch, which the member
/// does freely while the window is up: compared, each switch would re-check every export.
/// Struct-update syntax over `ctx.common()`, so a field added later is compared unless
/// someone resets it here: a needless re-check is the safe way to be wrong, a missed one
/// leaves the grid stale.
fn check_settings(ctx: &ToolContext) -> (CommonSettings, toml::Table) {
    let common = CommonSettings {
        theme: Theme::Dark,
        invert_number_scrolling: false,
        check_for_updates: false,
        skipped_version: None,
        last_tool: None,
        last_run_version: None,
        window_main: None,
        ..ctx.common()
    };
    (common, ctx.tool_settings(TOOL_ID))
}

impl Default for Tool {
    fn default() -> Tool {
        Tool::new()
    }
}

/// The running-PES probe: the real process list (`studio_core::running_pes`).
#[cfg(not(test))]
fn running_probe() -> fn() -> Vec<RunningPes> {
    studio_core::running_pes
}

/// `running_probe` under `cargo test`: no game at all — a unit test never reads
/// the real process list, which would skip the savefile step on a machine with
/// the fixture's PES running.
#[cfg(test)]
fn running_probe() -> fn() -> Vec<RunningPes> {
    || Vec::new()
}

impl StudioTool for Tool {
    fn id(&self) -> &'static str {
        TOOL_ID
    }

    fn label(&self) -> &'static str {
        "The Compiler"
    }

    fn icon(&self) -> &'static str {
        studio_core::icons::TEAM_COMPILER
    }

    /// The window; the first time it is shown, it starts the grid's check.
    fn view(&mut self, ui: &mut egui::Ui, ctx: &ToolContext) {
        if !self.live_check.has_started() && !self.run_log.is_running() {
            self.start_check(ctx);
        }
        let rows = self.live_check.rows();
        let checking = self.live_check.is_running();
        let panel = &mut self.log_panel;
        match view::show(
            ui,
            ctx,
            &self.run_log,
            panel,
            rows,
            checking,
            &mut self.detail,
        ) {
            Some(Intent::Compile(mode)) => self.run_log.start_compile(ctx, mode, self.running),
            Some(Intent::OpenOutputFolder) => self.open_output_folder(ctx),
            Some(Intent::UpgradeDpfl) => self.open_upgrade(ctx),
            Some(Intent::Upgrade(download, delete)) => {
                self.run_log.start_upgrade(download, delete, ctx)
            }
            None => {}
        }
    }

    /// The run's events into the log; the grid checked again once a run ends. A
    /// check's rows reach the grid only while no run is in flight, since the run may
    /// change what they describe; its detail window then reads its pictures again, and the
    /// check's findings replace the last check's at the top of the log. A refused check is
    /// a line of the log.
    fn tick(&mut self, ctx: &ToolContext) {
        let compiling = self.run_log.is_running();
        self.run_log.poll();
        if self.run_log.is_running() {
            return;
        }
        if compiling {
            self.start_check(ctx);
        }
        // `common` and `tool_settings` are small in-memory copies under the settings lock
        // (no file read), so comparing them each frame is cheap.
        let settings = check_settings(ctx);
        if let Some(checked) = &self.checked_settings
            && *checked != settings
        {
            // What the grid and the toolbar's buttons describe changed: an open dialog's
            // preview is stale, so it closes, and the check runs again.
            self.detail.dpfl_dialog = None;
            self.start_check(ctx);
        }
        match self.live_check.poll(ctx) {
            Polled::Nothing => {}
            Polled::Rows => {
                self.detail.rows_replaced();
                self.run_log
                    .set_check(self.live_check.rows(), self.live_check.list_findings());
            }
            Polled::Refused(refusal) => self.run_log.append_refusal(&refusal),
        }
    }

    fn settings_view(&mut self, ui: &mut egui::Ui, _ctx: &ToolContext) {
        ui.label("The Compiler's settings are not built yet.");
    }

    fn default_settings(&self) -> toml::Table {
        settings::default_table()
    }

    fn help(&self) -> HelpSection {
        HelpSection {
            title: "The Compiler",
            topics: vec![HelpTopic {
                slug: "command-line",
                title: "Running from a terminal",
                body: include_str!("../help/01_command_line.md"),
            }],
            messages: messages::catalog(),
        }
    }

    fn cli_command(&self) -> clap::Command {
        cli::command()
    }

    fn cli_run(&self, matches: &clap::ArgMatches, ctx: &ToolContext) -> Result<u8, CliError> {
        cli::run(matches, ctx, self.running)
    }

    /// The compile's CPK and the conversions into it are for the version the
    /// sidebar's selector holds.
    fn targets_pes_version(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tool_is_registered_as_the_compiler() {
        let tool = Tool::new();
        assert_eq!(tool.id(), "compiler");
        assert_eq!(tool.label(), "The Compiler");
        assert_eq!(
            tool.default_settings()["cpk_name"].as_str(),
            Some("4cc_99_test")
        );
        assert!(tool.targets_pes_version());
    }
}
