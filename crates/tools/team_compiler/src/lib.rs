//! The Team compiler: Studio-format aesthetics exports compiled to CPK archives.
//!
//! `Tool` registers the tool with the shell: its window (`view`, over the run log in
//! `gui_run`), its settings section, its help chapter and its `team-compiler` command line,
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
#[cfg(test)]
mod testing;
mod texture_lookup;
mod upgrade;
mod user_face_xml;
mod validation;
mod view;

use studio_core::{
    CliError, HelpSection, HelpTopic, LogPanel, RunningPes, StudioTool, ToolContext,
};

use crate::gui_run::RunLog;
use crate::messages::TOOL_ID;

/// The Team compiler, as registered with the `studio` binary. Its window state is
/// the run log — the `compile` the Compile button started, and its lines — and the
/// log panel's own view state.
pub struct Tool {
    run_log: RunLog,
    log_panel: LogPanel,
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
            log_panel: LogPanel::new(TOOL_ID),
            running,
        }
    }
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
        "Team compiler"
    }

    fn icon(&self) -> &'static str {
        studio_core::icons::TEAM_COMPILER
    }

    fn view(&mut self, ui: &mut egui::Ui, ctx: &ToolContext) {
        if view::show(ui, ctx, &self.run_log, &mut self.log_panel) {
            self.run_log.start_compile(ctx, self.running);
        }
    }

    fn tick(&mut self, _ctx: &ToolContext) {
        self.run_log.poll();
    }

    fn settings_view(&mut self, ui: &mut egui::Ui, _ctx: &ToolContext) {
        ui.label("The Team compiler's settings are not built yet.");
    }

    fn default_settings(&self) -> toml::Table {
        settings::default_table()
    }

    fn help(&self) -> HelpSection {
        HelpSection {
            title: "Team compiler",
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tool_is_registered_as_the_team_compiler() {
        let tool = Tool::new();
        assert_eq!(tool.id(), "team-compiler");
        assert_eq!(tool.label(), "Team compiler");
        assert_eq!(
            tool.default_settings()["cpk_name"].as_str(),
            Some("4cc_99_test")
        );
    }
}
