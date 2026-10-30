//! The `ruleset-editor` command line: `validate`, `update-logic` and
//! `summary`. A refusal (an unreadable file, a file whose checking code
//! is not this build's) comes before anything is written.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use clap::{Args, Command, FromArgMatches, Subcommand};
use studio_core::{CliError, Severity, ToolContext};

use crate::document::{Document, Logic, OpenError, SaveError, write_text};
use crate::events::Report;
use crate::messages::{TOOL_ID, findings, logic_already_current, logic_updated, summary_written};

// The exit codes are the command line's contract with scripts, the Team
// compiler's meanings.
/// Exit code of a command that did its work with no Error finding.
const DONE: u8 = 0;
/// Exit code of a command that did its work with an Error finding.
const ERRORS: u8 = 1;
/// Exit code of a command refused before anything was written.
const INVALID: u8 = 2;
/// Exit code of a command whose write failed; what was already written stands.
const WRITE_FAILED: u8 = 3;

#[derive(Debug, Subcommand)]
enum RulesetCommand {
    /// Load-time validation of a rules file: each finding prints, exit 1 if any.
    Validate(FileArgs),
    /// Replace an older official logic section with this build's current one;
    /// the replaced file is kept as <file>.bak.
    UpdateLogic(FileArgs),
    /// The ruleset as readable rules text, printed, or written with `-o`.
    Summary(SummaryArgs),
}

/// `<file>`: the rules file the command works on.
#[derive(Debug, Args)]
struct FileArgs {
    /// The rules file (.4ccrules).
    file: PathBuf,
}

/// `summary <file> [-o <out>]`.
#[derive(Debug, Args)]
struct SummaryArgs {
    /// The rules file (.4ccrules).
    file: PathBuf,
    /// Write the text here instead of printing it; a file it replaces is
    /// kept as <out>.bak.
    #[arg(short = 'o')]
    out: Option<PathBuf>,
}

/// The `ruleset-editor` subcommand and its own subcommands.
pub(crate) fn command() -> Command {
    RulesetCommand::augment_subcommands(Command::new(TOOL_ID).subcommand_required(true))
}

/// Runs a parsed `ruleset-editor` command.
pub(crate) fn run(matches: &clap::ArgMatches, ctx: &ToolContext) -> Result<u8, CliError> {
    let command =
        RulesetCommand::from_arg_matches(matches).map_err(|error| CliError::new(INVALID, error))?;
    let mut report = Report::new(ctx);
    match command {
        RulesetCommand::Validate(args) => validate(&args, &mut report)?,
        RulesetCommand::UpdateLogic(args) => update_logic(&args, &mut report)?,
        RulesetCommand::Summary(args) => summary_command(&args, &mut report)?,
    }
    Ok(exit_code(report.worst()))
}

/// `ERRORS` when a finding was an Error, else `DONE`.
fn exit_code(worst: Option<Severity>) -> u8 {
    match worst {
        Some(Severity::Error | Severity::Fatal) => ERRORS,
        Some(Severity::Warning | Severity::Info) | None => DONE,
    }
}

/// A refusal before anything was written.
fn invalid(error: anyhow::Error) -> CliError {
    CliError::new(INVALID, error)
}

/// A write that failed after validation passed.
fn write_failed(error: anyhow::Error) -> CliError {
    CliError::new(WRITE_FAILED, error)
}

/// `validate`: the file's load findings as `rules_finding` lines; a clean
/// file prints nothing. A file that cannot be read is a refusal.
fn validate(args: &FileArgs, report: &mut Report) -> Result<(), CliError> {
    let text = fs::read_to_string(&args.file).map_err(|error| {
        invalid(anyhow!(
            "{}: cannot read the file: {error}",
            args.file.display()
        ))
    })?;
    if let Err(error) = aatf::Rules::load(&text) {
        for finding in findings(&error) {
            report.message(finding);
        }
    }
    Ok(())
}

/// `update-logic`: a current file is left as it was; a modified logic is
/// refused (updating would discard someone's code); an older one gets the
/// current logic written, the replaced file kept as `<file>.bak` first.
fn update_logic(args: &FileArgs, report: &mut Report) -> Result<(), CliError> {
    let mut document = open(&args.file)?;
    update_document(&mut document, &args.file, report)
}

/// The file opened as `update-logic` and `summary` open it; a file that
/// does not open is a refusal before anything is written.
fn open(path: &Path) -> Result<Document, CliError> {
    Document::open(path).map_err(|error| match error {
        // The `LoadError`'s text names no file; the read error's already does.
        OpenError::Invalid(load) => invalid(anyhow!("{}: {load}", path.display())),
        other => invalid(other.into()),
    })
}

/// `path` copied to `<path>.bak`: the file a writing command replaces,
/// kept one level deep (overwritten each time). Nothing when `path`
/// does not exist.
fn keep_bak(path: &Path) -> Result<(), CliError> {
    if !path.exists() {
        return Ok(());
    }
    let mut bak = path.as_os_str().to_owned();
    bak.push(".bak");
    let bak = PathBuf::from(bak);
    fs::copy(path, &bak)
        .map_err(|error| write_failed(anyhow!("{}: cannot keep a .bak: {error}", bak.display())))?;
    Ok(())
}

/// `summary`: the ruleset's readable rules text on stdout, or written to
/// `-o`'s file, a replaced file kept as `<out>.bak` first.
fn summary_command(args: &SummaryArgs, report: &mut Report) -> Result<(), CliError> {
    let document = open(&args.file)?;
    let text = crate::summary::summary(&document.ruleset);
    match &args.out {
        // An `-o` naming the rules file itself is refused before anything
        // is written: replacing it with the summary would lose the
        // ruleset. The two are compared canonicalized, so one file under
        // two names still matches; a missing `-o` file canonicalizes to
        // nothing and is no match.
        Some(out)
            if let (Ok(out_file), Ok(rules)) =
                (fs::canonicalize(out), fs::canonicalize(&args.file))
                && out_file == rules =>
        {
            return Err(invalid(anyhow!(
                "-o {}: that is the rules file being read; the summary written over it would lose it",
                out.display()
            )));
        }
        Some(out) => {
            keep_bak(out)?;
            write_text(out, &text).map_err(|error| {
                write_failed(anyhow!("{}: cannot write the file: {error}", out.display()))
            })?;
            report.message(summary_written(out));
        }
        // The console ends an `Output` line itself, so the text goes
        // without its own final newline: stdout is it byte for byte.
        None => report.output(text.strip_suffix('\n').unwrap_or(&text).to_string()),
    }
    Ok(())
}

/// `update-logic` on an opened document. The unit tests drive the older path
/// through this: no real file of an older official logic exists while logic
/// version 1 is the only one.
fn update_document(
    document: &mut Document,
    path: &Path,
    report: &mut Report,
) -> Result<(), CliError> {
    match &document.logic {
        Logic::Current => report.message(logic_already_current(path)),
        Logic::Modified { .. } => {
            return Err(invalid(anyhow!(
                "{}: the file's checking code is not this build's, so updating would discard it",
                path.display()
            )));
        }
        Logic::Older { version, .. } => {
            let from = *version;
            document.update_logic();
            keep_bak(path)?;
            document.save(path).map_err(|error| match error {
                // A refused save: the file is as it was (the `.bak` keeps
                // the same bytes).
                SaveError::Invalid(_) => invalid(anyhow!("{error}")),
                SaveError::Write { .. } => write_failed(anyhow!("{error}")),
            })?;
            report.message(logic_updated(path, from));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use studio_core::PipelineEvent;

    use super::*;
    use crate::test_support::{context, scratch, valid_file, write_file};

    /// `ruleset-editor` with no subcommand is a usage error naming the
    /// missing subcommand, not a run.
    #[test]
    fn no_subcommand_is_refused() {
        let error = command()
            .try_get_matches_from(["ruleset-editor"])
            .expect_err("refused");
        assert_eq!(error.kind(), clap::error::ErrorKind::MissingSubcommand);
    }

    /// `update-logic` on a document whose logic is `Older`: the file gets the
    /// current logic and the replaced file is kept as `.bak`. No real file of
    /// an older official logic exists while version 1 is the only one, so the
    /// document's logic is set to `Older` by hand.
    #[test]
    fn an_older_logic_is_replaced_with_the_old_file_kept_as_bak() {
        let dir = scratch("update_logic_older");
        let original = valid_file();
        let path = write_file(&dir, "rules.4ccrules", &original);
        let mut document = Document::open(&path).expect("opens");
        document.logic = Logic::Older {
            version: 1,
            section: String::new(),
        };
        let (ctx, events) = context();
        let mut report = Report::new(&ctx);
        update_document(&mut document, &path, &mut report).expect("updated");
        assert!(matches!(document.logic, Logic::Current));
        assert_eq!(
            fs::read_to_string(dir.join("rules.4ccrules.bak")).expect("the .bak"),
            original,
            "the .bak holds the replaced file"
        );
        let reopened = Document::open(&path).expect("reopens");
        assert!(
            matches!(reopened.logic, Logic::Current),
            "the file carries the current logic"
        );
        drop(report);
        drop(ctx);
        let envelopes: Vec<_> = events.iter().collect();
        assert_eq!(envelopes.len(), 1, "one logic_updated finding");
        let PipelineEvent::Message(finding) = &envelopes[0].event else {
            panic!("expected a Message, got {:?}", envelopes[0].event);
        };
        assert_eq!(finding.code.code, "logic_updated");
        assert!(
            finding
                .context
                .contains(&("from".to_owned(), "1".to_owned())),
            "the finding names the version it replaced: {finding:?}"
        );
        assert!(
            finding.context.iter().any(|(key, _)| key == "path"),
            "the finding names the file: {finding:?}"
        );
    }

    /// `update-logic` on a document whose assembled file fails validation
    /// (two tiers sharing an id) is a refusal, not a write failure: exit
    /// code 2, the file unchanged. The `.bak` written before the save
    /// holds the same bytes.
    #[test]
    fn a_save_that_fails_validation_is_a_refusal() {
        let dir = scratch("update_logic_invalid_save");
        let original = valid_file();
        let path = write_file(&dir, "rules.4ccrules", &original);
        let mut document = Document::open(&path).expect("opens");
        document.ruleset.tiers[1].id = document.ruleset.tiers[0].id.clone();
        document.logic = Logic::Older {
            version: 1,
            section: String::new(),
        };
        let (ctx, _events) = context();
        let mut report = Report::new(&ctx);
        let error = update_document(&mut document, &path, &mut report).expect_err("refused");
        assert_eq!(
            error.exit_code, 2,
            "a save refused before anything is written is exit 2"
        );
        assert_eq!(
            fs::read(&path).unwrap(),
            original.as_bytes(),
            "the file is unchanged"
        );
        assert_eq!(
            fs::read(dir.join("rules.4ccrules.bak")).unwrap(),
            original.as_bytes(),
            "the .bak written before the save holds the same bytes"
        );
    }
}
