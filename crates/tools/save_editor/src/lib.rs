//! The Save editor: the PES EDIT savefile's teams and players exchanged (Team TOML, the
//! Team compiler's aesthetics patch, the teams list) and compared.
//!
//! `Tool` registers the tool with the shell: its settings section, its help chapter and its
//! `save-editor` command line (`spec/save_editor.md` "CLI"), and the window (`view/`: the
//! player list beside the selected row's player or team view). `pes_savefile` owns every byte and
//! every operation on the model; this crate owns the session (the file that is open) and the
//! command line over it.

mod cli;
mod diff;
mod events;
mod messages;
mod session;
mod settings;
mod teams_list;
#[cfg(test)]
mod test_support;
mod view;

use studio_core::widgets::{FORM_ROW_SPACING, checkbox_without_text};
use studio_core::{CliError, HelpSection, HelpTopic, Notice, RunningPes, StudioTool, ToolContext};

use crate::messages::TOOL_ID;

/// The "Aesthetics from exports" row's hover text in the settings menu.
const AESTHETICS_FROM_EXPORTS_TIP: &str = "On when the cup's looks are compiled from exports: \
     the save's aesthetics are then read-only, behind Edit aesthetics anyway on the Appearance \
     section.";

/// The Save editor, as registered with the `studio` binary. `view` is the window's
/// state.
pub struct Tool {
    view: view::SaveView,
    /// Whether the data folder and its imported rulesets were handed to the view: they are
    /// read on the first `view` frame, where the data folder is known; afterwards only the
    /// view's own import and × read the list again.
    imports_read: bool,
    /// The running-PES probe `Tool::view` hands `view::show`: the real
    /// process list in production, a stub reporting no game under `cargo
    /// test` — the file pickers' `#[cfg]` split
    /// (`comparator::default_open_dialog`). A unit test sets it to drive
    /// the running-game question.
    running: fn() -> Vec<RunningPes>,
}

impl Tool {
    /// The tool, with no save open.
    pub fn new() -> Tool {
        Tool {
            view: view::SaveView::new(),
            imports_read: false,
            running: running_probe(),
        }
    }

    /// The tool with `probe` standing in for the process list when it asks which PES games
    /// run: a test sets the games its command sees running, whatever the machine runs.
    #[cfg(feature = "test-support")]
    pub fn with_running_probe(self, probe: fn() -> Vec<RunningPes>) -> Tool {
        Tool {
            running: probe,
            ..self
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

/// `running_probe` under `cargo test`: no game at all — a test never reads
/// the real process list, which would open the Edit-mode question on a
/// machine with the fixture's PES running.
#[cfg(test)]
fn running_probe() -> fn() -> Vec<RunningPes> {
    || Vec::new()
}

impl StudioTool for Tool {
    fn id(&self) -> &'static str {
        TOOL_ID
    }

    fn label(&self) -> &'static str {
        "Save editor"
    }

    fn icon(&self) -> &'static str {
        studio_core::icons::SAVE_EDITOR
    }

    /// The unsaved-changes prompt guards the window's close (SE-GUI-05):
    /// a dirty session holds it and opens the prompt for `Close` (its
    /// Discard re-asks through `request_close`, answered once the session
    /// is dropped). A prompt already up retargets.
    fn hold_close(&mut self) -> bool {
        if self.view.is_dirty() {
            view::prompt::open(&mut self.view, view::prompt::Pending::Close);
            true
        } else {
            false
        }
    }

    fn view(&mut self, ui: &mut egui::Ui, ctx: &ToolContext) {
        // A settings error counts as the defaults (the first built-in
        // ruleset, Live AATF off, the aesthetics lock off).
        let table = ctx.tool_settings(TOOL_ID);
        let tool_settings = settings::from_table(&table).unwrap_or_default();
        if !self.imports_read {
            self.view.data_dir = ctx.paths().data_dir.clone();
            self.view.imported = session::rules::imported(self.view.data_dir.as_deref());
            self.imports_read = true;
        }
        self.view.aatf_ruleset = tool_settings.aatf_ruleset.clone();
        self.view.live_aatf = tool_settings.live_aatf;
        self.view.live_suggestions = tool_settings.live_suggestions;
        self.view.aesthetics_from_exports = tool_settings.aesthetics_from_exports;
        self.view.tactics.scroll_names = tool_settings.scroll_names;
        let common = ctx.common();
        self.view.hide_ids = common.fpc_hide_ids();
        self.view.pes_version = common.pes_version;
        self.view
            .select_portraits(ui.ctx(), &common.pes_folder(), common.pes_version);
        view::show(ui, &mut self.view, &self.running);
        if std::mem::take(&mut self.view.request_close) {
            ctx.request_close();
        }
        if let Some(notice) = self.view.notice.take() {
            ctx.notify(notice);
        }
        // The toolbar's ruleset dropdown, its toggle and Show suggestions
        // and the formation bar's Scroll names are settings: the table as
        // read with only those keys replaced, so any other key survives (as
        // `settings_view` writes `aesthetics_from_exports`).
        if self.view.aatf_ruleset != tool_settings.aatf_ruleset
            || self.view.live_aatf != tool_settings.live_aatf
            || self.view.live_suggestions != tool_settings.live_suggestions
            || self.view.tactics.scroll_names != tool_settings.scroll_names
        {
            let mut table = table;
            table.insert(
                "aatf_ruleset".to_owned(),
                toml::Value::String(self.view.aatf_ruleset.clone()),
            );
            table.insert(
                "live_aatf".to_owned(),
                toml::Value::Boolean(self.view.live_aatf),
            );
            table.insert(
                "live_suggestions".to_owned(),
                toml::Value::Boolean(self.view.live_suggestions),
            );
            table.insert(
                "scroll_names".to_owned(),
                toml::Value::Boolean(self.view.tactics.scroll_names),
            );
            ctx.set_tool_settings(TOOL_ID, table);
        }
    }

    fn has_settings_view(&self) -> bool {
        true
    }

    fn settings_view(&mut self, ui: &mut egui::Ui, ctx: &ToolContext) {
        let mut table = ctx.tool_settings(TOOL_ID);
        let mut aesthetics = table
            .get("aesthetics_from_exports")
            .and_then(toml::Value::as_bool)
            .unwrap_or_default();
        let mut committed_aesthetics = false;
        egui::Grid::new("save_editor_settings")
            .num_columns(2)
            .spacing(egui::vec2(ui.spacing().item_spacing.x, FORM_ROW_SPACING))
            .show(ui, |ui| {
                // The common grid's row shape: the label cell's tooltip
                // leads with the key, the widget's is the sentence alone.
                let label = ui.label("Aesthetics from exports").on_hover_text(format!(
                    "aesthetics_from_exports: {AESTHETICS_FROM_EXPORTS_TIP}"
                ));
                committed_aesthetics = checkbox_without_text(ui, &mut aesthetics)
                    .labelled_by(label.id)
                    .on_hover_text(AESTHETICS_FROM_EXPORTS_TIP)
                    .changed();
                ui.end_row();
            });
        // The table as read with only the changed key replaced, so any
        // other key survives.
        if committed_aesthetics {
            table.insert(
                "aesthetics_from_exports".to_owned(),
                toml::Value::Boolean(aesthetics),
            );
            ctx.set_tool_settings(TOOL_ID, table);
        }
    }

    fn default_settings(&self) -> toml::Table {
        settings::default_table()
    }

    fn help(&self) -> HelpSection {
        HelpSection {
            title: "Save editor",
            topics: vec![
                HelpTopic {
                    slug: "command-line",
                    title: "Running from a terminal",
                    body: include_str!("../help/01_command_line.md"),
                },
                HelpTopic {
                    slug: "window",
                    title: "The window",
                    body: include_str!("../help/02_window.md"),
                },
                HelpTopic {
                    slug: "abilities",
                    title: "Abilities & Skills",
                    body: include_str!("../help/03_abilities.md"),
                },
                HelpTopic {
                    slug: "appearance",
                    title: "Appearance",
                    body: include_str!("../help/04_appearance.md"),
                },
                HelpTopic {
                    slug: "team",
                    title: "The team view and the Team menu",
                    body: include_str!("../help/05_team.md"),
                },
                HelpTopic {
                    slug: "player",
                    title: "The Player menu",
                    body: include_str!("../help/06_player.md"),
                },
                HelpTopic {
                    slug: "savefile",
                    title: "The Savefile menu",
                    body: include_str!("../help/07_savefile.md"),
                },
                HelpTopic {
                    slug: "tactics",
                    title: "The Tactics editor",
                    body: include_str!("../help/08_tactics.md"),
                },
            ],
            messages: messages::catalog(),
        }
    }

    fn cli_command(&self) -> clap::Command {
        cli::command()
    }

    /// The open save's path, bracketed in the idle window title. No view draws it, so the
    /// toolbar keeps its one row.
    fn title_detail(&self) -> Option<String> {
        self.view.save_path().map(|path| path.display().to_string())
    }

    /// The version warning, while a save of another version than the sidebar's selector is
    /// open and the member has not seen it yet.
    fn standing_notice(&self, ctx: &ToolContext) -> Option<Notice> {
        self.view.version_notice(ctx.common().pes_version)
    }

    fn standing_notice_ended(&mut self) {
        self.view.version_warning = false;
    }

    fn cli_run(&self, matches: &clap::ArgMatches, ctx: &ToolContext) -> Result<u8, CliError> {
        cli::run(matches, ctx, self.running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tool_is_registered_as_the_save_editor() {
        let tool = Tool::new();
        assert_eq!(tool.id(), "save-editor");
        assert_eq!(tool.label(), "Save editor");
        assert_eq!(
            tool.default_settings()["aatf_ruleset"].as_str(),
            Some("autumn26")
        );
        assert_eq!(tool.help().topics.len(), 8);
    }
}
