//! The Ruleset editor: an AATF ruleset for your own invitational built through
//! forms and saved as a rules file the Save editor and the Team creator use
//! like the official one.
//!
//! `Tool` registers the tool with the shell: its help chapter and its
//! `ruleset-editor` command line (`validate`, `update-logic`), and the window
//! (`view/`: the start page, the rail over the section forms, the optional
//! data-block preview, and the unsaved-changes prompt the close is held for).
//! `document` owns a rules file open in memory (its ruleset, its logic status,
//! its custom checks as read, the undo snapshots and the dirty flag) and saves
//! it back through the same validation `Rules::load` applies; `summary`
//! renders the ruleset as readable rules text; `cli` drives them headless;
//! `settings` is the tool's settings-file section; `messages` is the finding
//! catalog.

mod cli;
mod document;
mod events;
mod messages;
mod settings;
mod summary;
#[cfg(test)]
mod test_support;
mod view;

use studio_core::{CliError, HelpSection, HelpTopic, StudioTool, ToolContext};

use crate::messages::TOOL_ID;

/// The Ruleset editor, as registered with the `studio` binary. `view` is the
/// window's state.
#[derive(Default)]
pub struct Tool {
    /// The window's state; `pub(crate)` so the view's tests drive it.
    pub(crate) view: view::RulesetView,
}

impl Tool {
    /// The tool, with no rules file open.
    pub fn new() -> Tool {
        Tool {
            view: view::RulesetView::new(),
        }
    }
}

impl StudioTool for Tool {
    fn id(&self) -> &'static str {
        TOOL_ID
    }

    /// "(WIP)" because some of the ruleset's sections have no form yet.
    fn label(&self) -> &'static str {
        "Ruleset editor (WIP)"
    }

    fn icon(&self) -> &'static str {
        studio_core::icons::RULESET_EDITOR
    }

    fn view(&mut self, ui: &mut egui::Ui, ctx: &ToolContext) {
        view::show(ui, &mut self.view, ctx);
    }

    /// The unsaved-changes prompt guards the window's close too (RE-GUI-15):
    /// a dirty document holds the close and opens the prompt.
    fn hold_close(&mut self) -> bool {
        if self
            .view
            .document
            .as_ref()
            .is_some_and(|document| document.is_dirty())
        {
            if self.view.pending_action.is_none() {
                self.view.pending_action = Some(view::Pending::Close);
            }
            true
        } else {
            false
        }
    }

    fn default_settings(&self) -> toml::Table {
        settings::default_table()
    }

    fn help(&self) -> HelpSection {
        HelpSection {
            title: "Ruleset editor",
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
            ],
            messages: messages::catalog(),
        }
    }

    fn cli_command(&self) -> clap::Command {
        cli::command()
    }

    fn cli_run(&self, matches: &clap::ArgMatches, ctx: &ToolContext) -> Result<u8, CliError> {
        cli::run(matches, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tool_is_registered_as_the_ruleset_editor() {
        let tool = Tool::new();
        assert_eq!(tool.id(), "ruleset-editor");
        assert_eq!(tool.label(), "Ruleset editor (WIP)");
        assert_eq!(tool.help().title, "Ruleset editor");
        assert_eq!(tool.default_settings()["last_folder"].as_str(), Some(""));
        assert_eq!(
            tool.default_settings()["recent_files"].as_array(),
            Some(&Vec::new())
        );
        assert_eq!(tool.help().topics.len(), 2);
    }
}
