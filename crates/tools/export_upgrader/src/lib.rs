//! The Export upgrader: exports in the old (AET) layout migrated to the Studio
//! player-folder format the Team compiler reads (`spec/export_upgrader.md`).
//!
//! `Tool` registers the tool with the shell: its help chapter and its
//! `export-upgrader` command line. The window is a later phase's; until then
//! `view` shows a placeholder line. The migration itself lives in `upgrade`:
//! the old export is read through `source`, its Note through `note`, the
//! roster through `roster`, the player folders planned by `players` (a
//! referee export's roster and folders by `referees`), the kit folders by
//! `kits`, the carried folders by `carried`, the `.mtl` rewrites by
//! `materials`, the `face.xml` respells by `face_xml`, the kit magic by
//! `kit_magic`, the pre-Fox model renames by `model_names`, the
//! `settings.toml` files by `settings`, the output's name through `naming`,
//! the staging folder by `staging`, and every finding goes through the
//! `messages` catalog. The
//! `upgrade-models` command's folder-of-model-folders mode lives in
//! `model_folders`.

mod carried;
mod cli;
mod face_xml;
mod kit_magic;
mod kits;
mod materials;
mod messages;
mod model_folders;
mod model_names;
mod naming;
mod note;
mod players;
mod referees;
mod roster;
mod settings;
mod source;
mod staging;
#[cfg(test)]
mod test_support;
mod upgrade;

use studio_core::{CliError, HelpSection, HelpTopic, StudioTool, ToolContext};

use crate::messages::TOOL_ID;

/// The Export upgrader, as registered with the `studio` binary. No window
/// state yet: the view arrives with the GUI phase.
#[derive(Default)]
pub struct Tool;

impl Tool {
    /// The tool.
    pub fn new() -> Tool {
        Tool
    }
}

impl StudioTool for Tool {
    fn id(&self) -> &'static str {
        TOOL_ID
    }

    fn label(&self) -> &'static str {
        "Export upgrader"
    }

    fn icon(&self) -> &'static str {
        studio_core::icons::EXPORT_UPGRADER
    }

    fn view(&mut self, ui: &mut egui::Ui, _ctx: &ToolContext) {
        ui.label(
            "The Export upgrader's window is not built yet; its commands run from a terminal.",
        );
    }

    /// The tool has no settings: the teams list path is the compiler's own
    /// (`spec/export_upgrader.md` step 6).
    fn default_settings(&self) -> toml::Table {
        toml::Table::new()
    }

    fn help(&self) -> HelpSection {
        HelpSection {
            title: "Export upgrader",
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
        cli::run(matches, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tool_is_registered_as_the_export_upgrader() {
        let tool = Tool::new();
        assert_eq!(tool.id(), "export-upgrader");
        assert_eq!(tool.label(), "Export upgrader");
        assert!(tool.default_settings().is_empty());
        assert_eq!(tool.help().topics.len(), 1);
    }
}
