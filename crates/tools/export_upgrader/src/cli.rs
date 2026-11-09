//! The `export-upgrader` command line (`spec/export_upgrader.md` "CLI"): a
//! parsed command turned into a run, and the run's outcome turned into the
//! exit code scripts read.

use std::path::PathBuf;

use clap::{Args, Command, FromArgMatches, Subcommand, ValueEnum};
use studio_core::{CliError, ToolContext};

use crate::messages::TOOL_ID;
use crate::{model_folders, upgrade};

// The exit codes are the command line's contract with scripts. They mean
// what the Team compiler's do.
/// Exit code of a run that finished with no Error finding and nothing
/// dropped.
pub(crate) const CLEAN: u8 = 0;
/// Exit code of a run that finished, but reported an Error finding or one
/// that drops something (unmigrated content, the unknown team, an export that
/// does not extract).
pub(crate) const ERRORS: u8 = 1;
/// Exit code of an invalid invocation: nothing was written.
pub(crate) const INVALID: u8 = 2;
/// Exit code of a run aborted: before anything is written (the output or
/// staging folder exists or cannot be created), or when a file of the
/// output cannot be written, the incomplete staging folder left.
pub(crate) const ABORTED: u8 = 3;

#[derive(Debug, Subcommand)]
enum ExportUpgraderCommand {
    /// Migrate an old (AET) export to the new format.
    Upgrade(UpgradeArgs),
    /// Migrate a folder of old-format model folders, each into a folder to
    /// drop into an export.
    UpgradeModels(UpgradeModelsArgs),
}

/// `upgrade <export> (--savefile <EDIT> | --loose) [--coverage full|midcup] [-o <dir>]`.
#[derive(Debug, Args)]
pub(crate) struct UpgradeArgs {
    /// The old export: a folder, a .zip or a .7z file.
    pub(crate) export: PathBuf,
    /// The save (EDIT00000000) that says which player wears which
    /// boots and gloves.
    #[arg(long, value_name = "EDIT")]
    pub(crate) savefile: Option<PathBuf>,
    /// Without a savefile, the wearer of each boots or gloves folder is
    /// inferred from its ID's position in the team's old block of 25 IDs.
    #[arg(long)]
    pub(crate) loose: bool,
    /// The coverage tag the output's name takes, instead of the one guessed
    /// from the old name.
    #[arg(long, value_enum)]
    pub(crate) coverage: Option<Coverage>,
    /// The folder the output goes in, created when missing. Without it, the
    /// input's own folder is used.
    #[arg(short, long, value_name = "DIR")]
    pub(crate) output: Option<PathBuf>,
}

/// `--coverage`'s value: the coverage tag the output's name takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Coverage {
    /// Everything the team has.
    Full,
    /// Additions: only what the export holds.
    Midcup,
}

impl Coverage {
    /// The flag's value as typed, for refusal messages.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Coverage::Full => "full",
            Coverage::Midcup => "midcup",
        }
    }
}

/// `upgrade-models <folder> [-o <dir>]`.
#[derive(Debug, Args)]
pub(crate) struct UpgradeModelsArgs {
    /// The folder holding the old-format model folders (face, boots and
    /// gloves folders, and an optional Common).
    pub(crate) folder: PathBuf,
    /// The output folder, created by the run. `<folder> upgraded` beside the
    /// input by default. It must not exist yet.
    #[arg(short, long, value_name = "DIR")]
    pub(crate) output: Option<PathBuf>,
}

/// The `export-upgrader` subcommand and its own subcommands.
pub(crate) fn command() -> Command {
    ExportUpgraderCommand::augment_subcommands(Command::new(TOOL_ID).subcommand_required(true))
}

/// Runs a parsed `export-upgrader` command.
pub(crate) fn run(matches: &clap::ArgMatches, ctx: &ToolContext) -> Result<u8, CliError> {
    let command = ExportUpgraderCommand::from_arg_matches(matches)
        .map_err(|error| CliError::new(INVALID, error))?;
    match command {
        ExportUpgraderCommand::Upgrade(args) => upgrade::run(&args, ctx),
        ExportUpgraderCommand::UpgradeModels(args) => model_folders::run(&args, ctx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bare_command_is_a_usage_error() {
        let error = command()
            .try_get_matches_from(["export-upgrader"])
            .expect_err("a bare `export-upgrader` takes a subcommand");
        assert_eq!(error.exit_code(), i32::from(INVALID));
    }
}
