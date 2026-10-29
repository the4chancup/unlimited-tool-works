//! The `save-editor` command line (`spec/save_editor.md` "CLI"): the batch surface for the
//! operations the cups script. Every command reads an explicit save; a refusal (a bad
//! argument, an unreadable file, a team the save lacks) comes before anything is written.

use std::fs;
use std::io::{self, Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::anyhow;
use clap::{ArgGroup, Args, Command, FromArgMatches, Subcommand};
use cpk::{CpkArchive, CpkWriter};
use pes_savefile::interchange::aesthetics_patch::AestheticsPatch;
use pes_savefile::interchange::team_toml::TeamToml;
use pes_savefile::ops::compare::{CompareError, DiffScope};
use pes_savefile::player_tables::{AppearanceTable, ItemList, PlayerTable, PlayerTableFile};
use pes_version::{Engine, PesVersion};
use studio_core::{CliError, RunningPes, Severity, TOOL_VERSION, ToolContext};

use crate::events::Report;
use crate::messages::{
    Code, TOOL_ID, aatf_check_failed, aatf_ruleset_gone, aatf_violation, appearance_id_restored,
    import_note, message, patch_skip, strip_base_copied_face, version_mismatch_sentence,
};
use crate::session::{self, Session};
use crate::{diff, settings, teams_list};

// The exit codes are the command line's contract with scripts, the Team compiler's meanings.
/// Exit code of a command that did its work with no Error finding.
const DONE: u8 = 0;
/// Exit code of a command that did its work with an Error finding (a patch's team the save
/// lacks).
const ERRORS: u8 = 1;
/// Exit code of a command refused before anything was written.
const INVALID: u8 = 2;
/// Exit code of a command whose write failed; what was already written stands.
const WRITE_FAILED: u8 = 3;

#[derive(Debug, Subcommand)]
enum SaveEditorCommand {
    /// Write one team of a save as a Team TOML file, or every team as one file each.
    ExportToml(ExportTomlArgs),
    /// Patch a save's team from a Team TOML file. The team its [team] id names takes the
    /// file's keys, and the rest of the save is untouched.
    ImportToml(ApplyArgs),
    /// Apply the Team compiler's aesthetics patch (its players' motions and names) to a save.
    ApplyPatch(ApplyArgs),
    /// List everything that differs between two saves of the same version, player by player.
    Diff(DiffArgs),
    /// Check the save's cup teams against an AATF ruleset, one line per violation.
    CheckAatf(CheckAatfArgs),
    /// Seed a CPK's Fox player tables from a save's players, then strip the save.
    StripAndSeed(StripAndSeedArgs),
    /// List the players whose appearance id came back while others stay stripped.
    CheckStripped(CheckStrippedArgs),
    /// Merge the save's team names and ids into the data directory's teams_list.txt.
    ExportTeamsList(ExportTeamsListArgs),
}

/// `strip-and-seed --base <save> --bins <cpk> --out <save> --bins-out <cpk>
/// [--edit-mode-closed]`.
#[derive(Debug, Args)]
struct StripAndSeedArgs {
    /// The save to strip (EDIT00000000), which is never written unless it is also --out.
    #[arg(long, value_name = "SAVE")]
    base: PathBuf,
    /// The cup's 4cc_08_bins.cpk the player tables are seeded into.
    #[arg(long, value_name = "CPK")]
    bins: PathBuf,
    /// Where the stripped save goes. An existing file there is kept as <out>.bak.
    #[arg(long, value_name = "SAVE")]
    out: PathBuf,
    /// Where the seeded CPK goes. An existing file there is kept as <bins-out>.bak.
    #[arg(long, value_name = "CPK")]
    bins_out: PathBuf,
    /// The game's Edit mode is closed: write the savefile even while its PES runs.
    #[arg(long)]
    edit_mode_closed: bool,
}

/// `check-stripped <save> [--strip-again --out <save>] [--edit-mode-closed]`.
#[derive(Debug, Args)]
struct CheckStrippedArgs {
    /// The save to check (EDIT00000000).
    save: PathBuf,
    /// Strip the listed players again and write --out.
    #[arg(long, requires = "out")]
    strip_again: bool,
    /// Where the re-stripped save goes. An existing file there is kept as <out>.bak.
    #[arg(long, value_name = "SAVE", requires = "strip_again")]
    out: Option<PathBuf>,
    /// The game's Edit mode is closed: write the savefile even while its PES runs. It
    /// matters only with --strip-again, the one form that writes the save.
    #[arg(long)]
    edit_mode_closed: bool,
}

/// `check-aatf <save> [--teams <list>] [--rules <file>]`.
#[derive(Debug, Args)]
struct CheckAatfArgs {
    /// The save to read (EDIT00000000).
    save: PathBuf,
    /// Only these teams, comma-separated ids and inclusive ranges (701,705-710). Without it,
    /// every team of the save in the cup range (701 to 920) is checked.
    #[arg(long, value_name = "LIST")]
    teams: Option<String>,
    /// The AATF rules file to check against. Without it, the ruleset picked in the Save
    /// editor's toolbar dropdown is used (the aatf_ruleset setting).
    #[arg(long, value_name = "FILE")]
    rules: Option<PathBuf>,
}

/// `export-teams-list <save> [--yes]`.
#[derive(Debug, Args)]
struct ExportTeamsListArgs {
    /// The save whose teams are merged in (EDIT00000000).
    save: PathBuf,
    /// Write the merged list instead of only printing what would change.
    #[arg(long)]
    yes: bool,
}

/// `diff <first> <second> [--aesthetics | --gameplay] [--json]`.
#[derive(Debug, Args)]
#[command(group = ArgGroup::new("scope").args(["aesthetics", "gameplay"]))]
struct DiffArgs {
    /// The save whose values are the old ones.
    first: PathBuf,
    /// The save whose values are the new ones.
    second: PathBuf,
    /// Only the look: boots, gloves, base copy, physique, strip, face.
    #[arg(long)]
    aesthetics: bool,
    /// Only the rest: names, basics, abilities, positions, skills, motion.
    #[arg(long)]
    gameplay: bool,
    /// The report as one JSON document on stdout, for tooling.
    #[arg(long)]
    json: bool,
}

/// `<file> --base <save> --out <save> [--edit-mode-closed]`: the file to apply, the save it
/// is applied to, and where the result goes (`--out` may be `--base`; an existing file there
/// is first copied to `<out>.bak`).
#[derive(Debug, Args)]
struct ApplyArgs {
    /// The Team TOML or aesthetics patch to apply.
    file: PathBuf,
    /// The save to read (EDIT00000000), which is never written unless it is also --out.
    #[arg(long, value_name = "SAVE")]
    base: PathBuf,
    /// Where the patched save goes. An existing file there is kept as <out>.bak.
    #[arg(long, value_name = "SAVE")]
    out: PathBuf,
    /// The game's Edit mode is closed: write the savefile even while its PES runs.
    #[arg(long)]
    edit_mode_closed: bool,
}

/// `export-toml <save> --team <id> --out <file>` or `--all-teams --out-dir <folder>`: the
/// two forms are exclusive, one is required, and each takes its own destination.
#[derive(Debug, Args)]
#[command(group = ArgGroup::new("selection").required(true).args(["team", "all_teams"]))]
struct ExportTomlArgs {
    /// The save to read (EDIT00000000).
    save: PathBuf,
    /// The one team to write, by id.
    #[arg(long, value_name = "ID", requires = "out")]
    team: Option<u32>,
    /// The file the one team goes to. It is replaced when it exists.
    #[arg(short, long, value_name = "FILE", requires = "team")]
    out: Option<PathBuf>,
    /// Every team of the save, one <id>.toml each.
    #[arg(long, requires = "out_dir")]
    all_teams: bool,
    /// The folder the <id>.toml files go in. It is created when missing, and files already
    /// there are replaced.
    #[arg(long, value_name = "FOLDER", requires = "all_teams")]
    out_dir: Option<PathBuf>,
}

/// The `save-editor` subcommand and its own subcommands.
pub(crate) fn command() -> Command {
    SaveEditorCommand::augment_subcommands(Command::new(TOOL_ID).subcommand_required(true))
}

/// Runs a parsed `save-editor` command. `running` is the running-PES probe the commands that
/// write a save ask before writing.
pub(crate) fn run(
    matches: &clap::ArgMatches,
    ctx: &ToolContext,
    running: fn() -> Vec<RunningPes>,
) -> Result<u8, CliError> {
    let command = SaveEditorCommand::from_arg_matches(matches)
        .map_err(|error| CliError::new(INVALID, error))?;
    let mut report = Report::new(ctx);
    match command {
        SaveEditorCommand::ExportToml(args) => export_toml(&args, &mut report)?,
        SaveEditorCommand::ImportToml(args) => import_toml(&args, running, &mut report)?,
        SaveEditorCommand::ApplyPatch(args) => apply_patch(&args, running, &mut report)?,
        SaveEditorCommand::Diff(args) => diff_saves(&args, &mut report)?,
        SaveEditorCommand::CheckAatf(args) => check_aatf(&args, ctx, &mut report)?,
        SaveEditorCommand::StripAndSeed(args) => strip_and_seed(&args, running, &mut report)?,
        SaveEditorCommand::CheckStripped(args) => check_stripped(&args, running, &mut report)?,
        SaveEditorCommand::ExportTeamsList(args) => {
            export_teams_list(&args, ctx.paths().data_dir.as_deref(), &mut report)?;
        }
    }
    Ok(exit_code(report.worst()))
}

/// `ERRORS` when a finding was an Error, else `DONE`, warnings and notes included.
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

/// The refusal a writing command makes while the PES of the save's `version` runs and it was
/// not given `--edit-mode-closed` (`edit_mode_closed`): the game writes the save it holds over
/// the file when the member leaves its Edit mode after changing anything there. A game of
/// another version never writes this save, so it does not refuse.
fn refuse_while_running(
    running: &[RunningPes],
    version: PesVersion,
    edit_mode_closed: bool,
) -> Result<(), CliError> {
    session::refuse_while_running(running, version, edit_mode_closed).map_err(invalid)
}

/// Opens the save a command reads; an unreadable one is a refusal.
fn open(path: &Path) -> Result<Session, CliError> {
    Session::open(path).map_err(invalid)
}

/// Reads the text file a command applies; an unreadable one is a refusal naming it.
fn read_text(path: &Path) -> Result<String, CliError> {
    fs::read_to_string(path).map_err(|error| {
        invalid(anyhow!(
            "{}: cannot read the file ({error})",
            path.display()
        ))
    })
}

/// Writes `text` to `path`, replacing an existing file; a failure names the file.
fn write_text(path: &Path, text: &str) -> Result<(), CliError> {
    fs::write(path, text).map_err(|error| {
        CliError::new(
            WRITE_FAILED,
            anyhow!("{}: cannot write the file ({error})", path.display()),
        )
    })
}

/// Writes the session's save to `out`; a failure is the write-failed code.
fn write_save(session: &Session, out: &Path) -> Result<(), CliError> {
    session
        .write(out)
        .map_err(|error| CliError::new(WRITE_FAILED, error))
}

/// `import-toml`: the document is parsed and applied in memory first, so a document the lib
/// refuses (a team name or short name too long, a key it does not know) refuses the command
/// with `--out` unwritten; each note is one line, emitted once `--out` is written, so a
/// failed write reports no note.
fn import_toml(
    args: &ApplyArgs,
    running: fn() -> Vec<RunningPes>,
    report: &mut Report,
) -> Result<(), CliError> {
    let mut session = open(&args.base)?;
    refuse_while_running(&running(), session.version(), args.edit_mode_closed)?;
    let document = TeamToml::parse(&read_text(&args.file)?)
        .map_err(|error| invalid(anyhow!("{}: {error}", args.file.display())))?;
    let notes = session.import_team(&document).map_err(invalid)?;
    let lines: Vec<_> = notes.iter().map(import_note).collect();
    write_save(&session, &args.out)?;
    for line in lines {
        report.message(line);
    }
    Ok(())
}

/// `apply-patch`: a version or allocation-scheme mismatch (and a pre-Fox patch) is refused
/// before anything is touched; a team or player the save lacks is an Error finding and the
/// rest is written. The applied and skipped lines are emitted once `--out` is written, so a
/// failed write reports none of them.
fn apply_patch(
    args: &ApplyArgs,
    running: fn() -> Vec<RunningPes>,
    report: &mut Report,
) -> Result<(), CliError> {
    let mut session = open(&args.base)?;
    refuse_while_running(&running(), session.version(), args.edit_mode_closed)?;
    let patch = AestheticsPatch::parse(&read_text(&args.file)?)
        .map_err(|error| invalid(anyhow!("{}: {error}", args.file.display())))?;
    let summary = session
        .apply_patch(&patch)
        .map_err(|error| invalid(anyhow!("{}: {error}", args.file.display())))?;
    let mut lines = Vec::new();
    for team in &summary.applied_teams {
        // A team is applied only when at least one of its players was written: a team the
        // save holds whose every patch player it lacks prints only their skips (a cup
        // player id is its team's id x 100 + slot).
        if summary
            .applied_players
            .iter()
            .any(|player| player / 100 == *team)
        {
            lines.push(message(
                Code::PatchTeamApplied,
                vec![("team", team.to_string())],
            ));
        }
    }
    for skip in &summary.skipped {
        lines.push(patch_skip(*skip));
    }
    write_save(&session, &args.out)?;
    for line in lines {
        report.message(line);
    }
    Ok(())
}

/// `export-toml`: every team's text is built before the first file is written, so a team
/// that cannot be exported refuses the command with nothing written.
fn export_toml(args: &ExportTomlArgs, report: &mut Report) -> Result<(), CliError> {
    let session = open(&args.save)?;
    let files: Vec<(u32, PathBuf, String)> = match (args.team, &args.out, &args.out_dir) {
        (Some(id), Some(out), _) => {
            // An --out naming the save itself is refused before anything is written: the
            // no-backup rule is for a Team TOML, and replacing the save with one would
            // lose it. The two are compared canonicalized, so one file under two names
            // still matches; a missing --out canonicalizes to nothing and is no match.
            if let (Ok(out_file), Ok(save)) = (fs::canonicalize(out), fs::canonicalize(&args.save))
                && out_file == save
            {
                return Err(invalid(anyhow!(
                    "--out {}: that is the save being read, and a Team TOML written over it would lose it",
                    out.display()
                )));
            }
            vec![(id, out.clone(), session.export_team(id).map_err(invalid)?)]
        }
        (None, _, Some(out_dir)) => {
            let mut files = Vec::new();
            for id in session.team_ids() {
                let path = out_dir.join(format!("{id}.toml"));
                files.push((id, path, session.export_team(id).map_err(invalid)?));
            }
            fs::create_dir_all(out_dir).map_err(|error| {
                CliError::new(
                    WRITE_FAILED,
                    anyhow!("{}: cannot create the folder ({error})", out_dir.display()),
                )
            })?;
            files
        }
        // clap requires one form and each form's destination.
        (Some(_), None, _) | (None, _, None) => {
            return Err(invalid(anyhow!(
                "export-toml takes --team <id> with --out <file>, or --all-teams with --out-dir <folder>"
            )));
        }
    };
    for (id, path, text) in files {
        write_text(&path, &text)?;
        report.message(message(
            Code::TeamExported,
            vec![
                ("team", id.to_string()),
                ("path", path.display().to_string()),
            ],
        ));
    }
    Ok(())
}

/// The refusal of two saves of different versions: `version_mismatch_sentence`
/// as the command's error.
fn version_mismatch(
    first_role: &str,
    first: PesVersion,
    second_role: &str,
    second: PesVersion,
) -> CliError {
    invalid(anyhow!(version_mismatch_sentence(
        first_role,
        first,
        second_role,
        second
    )))
}

/// `diff`: the comparator's report as findings; differences are its output, so the exit
/// code is 0, and two versions are refused.
fn diff_saves(args: &DiffArgs, report: &mut Report) -> Result<(), CliError> {
    let first = open(&args.first)?;
    let second = open(&args.second)?;
    let scope = match (args.aesthetics, args.gameplay) {
        (true, false) => Some(DiffScope::Aesthetics),
        (false, true) => Some(DiffScope::Gameplay),
        // clap makes the two flags exclusive.
        (false, false) | (true, true) => None,
    };
    let result = if args.json {
        diff::report_json(first.file(), second.file(), scope, report)
    } else {
        diff::report(first.file(), second.file(), scope, report)
    };
    result.map_err(|error| match error {
        CompareError::VersionMismatch { a, b } => {
            version_mismatch("the first save", a, "the second", b)
        }
        CompareError::Codec(_) => invalid(error.into()),
    })
}

/// `check-aatf`: the ruleset is `--rules`, else the one the `aatf_ruleset` setting picks
/// (`rules`); a `--rules` file that does not read or fails load-time validation is refused
/// naming it, so the command doubles as a rules file's check. The teams are checked in
/// parallel on the one loaded file (each check is ~50 ms of script) and reported in id order; a
/// team's failed check is an error line and the other teams are still reported; a violation is
/// one line at its severity, suggestions aside.
fn check_aatf(
    args: &CheckAatfArgs,
    ctx: &ToolContext,
    report: &mut Report,
) -> Result<(), CliError> {
    let rules = rules(args, ctx, report)?;
    let session = open(&args.save)?;
    let ids = match &args.teams {
        Some(text) => parse_teams(text).map_err(|error| invalid(anyhow!("--teams: {error}")))?,
        None => {
            let mut ids: Vec<u32> = session
                .team_ids()
                .filter(|id| (701..=920).contains(id))
                .collect();
            ids.sort_unstable();
            ids
        }
    };
    for id in &ids {
        if session.file().team(*id).is_none() {
            return Err(invalid(anyhow!("the save has no team {id}")));
        }
    }
    let views = session::rules::team_views(session.file(), &ids);
    let results = session::rules::check_teams(&rules, &views);
    for (id, result) in ids.into_iter().zip(results) {
        let team = session.file().team(id).expect("checked above");
        let violations = match result {
            Ok(violations) => violations,
            Err(error) => {
                report.message(aatf_check_failed(id, error));
                continue;
            }
        };
        for violation in violations {
            let severity = match violation.severity {
                aatf::Severity::Error => Severity::Error,
                aatf::Severity::Warning => Severity::Warning,
                // Suggestions are not printed: there is nothing to fix in an unused allowance.
                aatf::Severity::Suggestion => continue,
            };
            let player = violation
                .slot
                .and_then(|slot| team.roster.get(usize::from(slot)))
                .map(|slot| slot.player_id);
            report.message(aatf_violation(id, player, &violation, severity));
        }
    }
    Ok(())
}

/// The ruleset `check-aatf` checks against: `--rules`, a command-line path used as given,
/// else the `aatf_ruleset` setting resolved as the window resolves it
/// (`session::rules::resolve`), a choice that is gone reported as a warning line.
fn rules(
    args: &CheckAatfArgs,
    ctx: &ToolContext,
    report: &mut Report,
) -> Result<Arc<aatf::Rules>, CliError> {
    if let Some(path) = &args.rules {
        return session::rules::load_file(path)
            .map(Arc::new)
            .map_err(invalid);
    }
    // A toml error's text ends with a newline, which would leave a blank line under `error: `.
    let settings = settings::from_table(&ctx.tool_settings(TOOL_ID)).map_err(|error| {
        invalid(anyhow!(
            "the [{TOOL_ID}] settings cannot be read: {}",
            error.to_string().trim_end()
        ))
    })?;
    let imported = session::rules::imported(ctx.paths().data_dir.as_deref());
    let choice = session::rules::resolve(&settings.aatf_ruleset, &imported);
    if choice.notice.is_some() {
        report.message(aatf_ruleset_gone(
            &settings.aatf_ruleset,
            &choice.rules.ruleset().name,
        ));
    }
    Ok(choice.rules)
}

/// `--teams`' list: comma-separated ids and inclusive ranges (`701,705-710`), sorted with
/// duplicates dropped; a malformed item or a range whose start is past its end is an `Err`
/// naming it.
fn parse_teams(text: &str) -> Result<Vec<u32>, String> {
    let mut ids = Vec::new();
    for item in text.split(',') {
        let item = item.trim();
        let (start, end) = match item.split_once('-') {
            Some((first, last)) => (
                first
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| format!("{item:?} is not a team id or a range"))?,
                last.trim()
                    .parse::<u32>()
                    .map_err(|_| format!("{item:?} is not a team id or a range"))?,
            ),
            None => {
                let id = item
                    .parse::<u32>()
                    .map_err(|_| format!("{item:?} is not a team id or a range"))?;
                (id, id)
            }
        };
        if start > end {
            return Err(format!("{item:?} is a range whose start is past its end"));
        }
        ids.extend(start..=end);
    }
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// `strip-and-seed`: the seed, then the strip (`spec/save_editor.md` "Stripped save"). A
/// refusal of the save or of `--bins` writes nothing, and an
/// unwritable `--bins-out` leaves `--out` unwritten (a seeded CPK beside an unstripped save
/// changes no look: the records still decide).
fn strip_and_seed(
    args: &StripAndSeedArgs,
    running: fn() -> Vec<RunningPes>,
    report: &mut Report,
) -> Result<(), CliError> {
    let mut session = open(&args.base)?;
    refuse_pre_fox(session.version())?;
    refuse_while_running(&running(), session.version(), args.edit_mode_closed)?;
    let mut archive = open_cpk(&args.bins)?;
    let mut appearance: AppearanceTable =
        read_table(&mut archive, PlayerTableFile::PlayerAppearance)?;
    let mut boots: ItemList = read_table(&mut archive, PlayerTableFile::BootsList)?;
    let mut gloves: ItemList = read_table(&mut archive, PlayerTableFile::GloveList)?;
    let seed = session.seed().map_err(|error| invalid(error.into()))?;
    for row in &seed {
        appearance.set(row.player, row.row[4..].try_into().expect("56 bytes"));
        boots.set(row.player, row.boots.to_le_bytes());
        gloves.set(row.player, row.gloves.to_le_bytes());
    }
    write_bins(
        &mut archive,
        &args.bins,
        &args.bins_out,
        &appearance,
        &boots,
        &gloves,
    )?;
    session.strip(&seed.iter().map(|row| row.player).collect::<Vec<_>>());
    write_save(&session, &args.out)?;
    // Only once the strip is on disk: a base-copied face the seed rows cannot carry is
    // lost, and the warning is about the strip that happened.
    for row in &seed {
        if let Some(copied) = row.base_copy {
            let player = session
                .file()
                .player(row.player)
                .expect("the seed's own ids");
            report.message(strip_base_copied_face(player, copied));
        }
    }
    Ok(())
}

/// `check-stripped`: each player whose appearance id came back is one
/// `appearance_id_restored` line, an Error while he stands listed (exit 1). With
/// `--strip-again` (which clap couples to `--out`) his id is set to -1 and `--out` is
/// written; the lines then report what was fixed, so they go out at Info once the write
/// has landed (a failed write reports nobody fixed) and the exit code is 0.
fn check_stripped(
    args: &CheckStrippedArgs,
    running: fn() -> Vec<RunningPes>,
    report: &mut Report,
) -> Result<(), CliError> {
    let mut session = open(&args.save)?;
    refuse_pre_fox(session.version())?;
    if args.strip_again {
        refuse_while_running(&running(), session.version(), args.edit_mode_closed)?;
    }
    let restored = session.restored_players();
    if let Some(out) = &args.out {
        let ids: Vec<u32> = restored.iter().map(|player| player.id).collect();
        let lines: Vec<_> = restored
            .iter()
            .copied()
            .map(|player| appearance_id_restored(player, Severity::Info))
            .collect();
        session.strip(&ids);
        write_save(&session, out)?;
        for line in lines {
            report.message(line);
        }
    } else {
        for player in restored {
            report.message(appearance_id_restored(player, Severity::Error));
        }
    }
    Ok(())
}

/// Refuses a pre-Fox save (PES 15 to 17): stripping is not supported there; the database
/// route is untested, and the aesthetics patch is their route (`spec/save_editor.md`
/// "Stripped save").
fn refuse_pre_fox(version: PesVersion) -> Result<(), CliError> {
    match version.engine() {
        Engine::Fox => Ok(()),
        Engine::PreFox => Err(invalid(anyhow!(
            "{version}: stripping a pre-Fox save is not supported, because the database \
             route is untested there. A pre-Fox save takes the aesthetics patch instead."
        ))),
    }
}

/// Opens the CPK at `path`; one that does not open or does not read as a CPK is a refusal.
fn open_cpk(path: &Path) -> Result<CpkArchive<fs::File>, CliError> {
    let file = fs::File::open(path).map_err(|error| {
        invalid(anyhow!(
            "{}: cannot read the file ({error})",
            path.display()
        ))
    })?;
    CpkArchive::open(file)
        .map_err(|error| invalid(anyhow!("{}: cannot read the CPK ({error})", path.display())))
}

/// The CPK's copy of `file`'s table; a table the archive lacks starts empty, and a copy
/// that does not read as the table (a length that is not a whole number of rows) is a
/// refusal naming it.
fn read_table<const VALUE: usize>(
    archive: &mut CpkArchive<impl Read + Seek>,
    file: PlayerTableFile,
) -> Result<PlayerTable<VALUE>, CliError> {
    let entry = archive
        .entries()
        .iter()
        .find(|entry| entry.path == file.cpk_path())
        .cloned();
    let Some(entry) = entry else {
        return Ok(PlayerTable::new());
    };
    let bytes = archive
        .read(&entry)
        .map_err(|error| invalid(anyhow!("{}: cannot read the entry ({error})", entry.path)))?;
    PlayerTable::read(file, &bytes).map_err(|error| invalid(error.into()))
}

/// Writes `out` as `--bins` with the three tables seeded: every other entry with its path,
/// bytes and timestamp unchanged, in `--bins`' order, a table's new bytes in its entry's
/// place (its timestamp kept), and a table `--bins` lacked appended in `PlayerTableFile`
/// order. The archive is built in memory so a bad `--bins` still refuses before the write;
/// a write that fails is the write-failed code.
fn write_bins(
    archive: &mut CpkArchive<impl Read + Seek>,
    bins: &Path,
    out: &Path,
    appearance: &AppearanceTable,
    boots: &ItemList,
    gloves: &ItemList,
) -> Result<(), CliError> {
    let tables = [
        (PlayerTableFile::PlayerAppearance, appearance.to_bytes()),
        (PlayerTableFile::BootsList, boots.to_bytes()),
        (PlayerTableFile::GloveList, gloves.to_bytes()),
    ];
    let mut writer = CpkWriter::new(Cursor::new(Vec::new()), TOOL_VERSION)
        .map_err(|error| invalid(anyhow!("{error}")))?;
    for entry in archive.entries().to_vec() {
        let bytes = match tables
            .iter()
            .find(|(file, _)| file.cpk_path() == entry.path)
        {
            Some((_, bytes)) => bytes.clone(),
            None => archive.read(&entry).map_err(|error| {
                invalid(anyhow!(
                    "{}: cannot read the entry {} ({error})",
                    bins.display(),
                    entry.path
                ))
            })?,
        };
        writer
            .add(&entry.path, &bytes, entry.modified)
            .map_err(|error| invalid(anyhow!("{}: {error}", bins.display())))?;
    }
    for (file, bytes) in &tables {
        if archive
            .entries()
            .iter()
            .any(|entry| entry.path == file.cpk_path())
        {
            continue;
        }
        writer
            .add(file.cpk_path(), bytes, None)
            .map_err(|error| invalid(anyhow!("{}: {error}", bins.display())))?;
    }
    let bytes = writer
        .finish()
        .map_err(|error| invalid(anyhow!("{error}")))?
        .into_inner();
    write_file(out, &bytes)
}

/// Writes `bytes` to `path` under the rule "Writing a save" gives every output file: the
/// content lands in a `.tmp` sibling, an existing `path` is copied to `<path>.bak` (one
/// level), then the sibling is renamed over it. A failure is the write-failed code and
/// leaves `path` as it was.
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    let sibling = |ext: &str| {
        let mut name = path.as_os_str().to_owned();
        name.push(ext);
        PathBuf::from(name)
    };
    let tmp = sibling(".tmp");
    let bak = sibling(".bak");
    let write_and_swap = || -> io::Result<()> {
        fs::write(&tmp, bytes)?;
        if path.exists() {
            fs::copy(path, &bak)?;
        }
        fs::rename(&tmp, path)
    };
    write_and_swap().map_err(|error| {
        // The write's own error is the one reported; the temporary file is removed best
        // effort, a leftover being a diagnostic rather than a finding.
        if let Err(removal) = fs::remove_file(&tmp) {
            log::debug!("{}: kept: {removal}", tmp.display());
        }
        CliError::new(
            WRITE_FAILED,
            anyhow!("{}: cannot write the file ({error})", path.display()),
        )
    })
}

/// `export-teams-list`: the save's teams merged into the data directory's list, the summary
/// one line per change, the file written only with `--yes`. A data directory that cannot
/// take the file (none chosen yet, or unwritable, found by attempting the write) is the
/// `teams_list_read_only` error with nothing written. A dropped change is an error too; the
/// other changes are written.
fn export_teams_list(
    args: &ExportTeamsListArgs,
    data_dir: Option<&Path>,
    report: &mut Report,
) -> Result<(), CliError> {
    let session = open(&args.save)?;
    // The working list is read before the first finding: a refusal emits nothing, and a
    // list that cannot be read is one (the upstream list is never merged over it).
    let path = data_dir.map(|dir| dir.join(teams_list::FILE_NAME));
    let working = teams_list::load_working(path.as_deref()).map_err(invalid)?;
    let (incoming, duplicates) = teams_list::from_save(session.file());
    for duplicate in &duplicates {
        report.message(message(
            Code::TeamsListNameDuplicate,
            vec![
                ("team", duplicate.team.to_string()),
                ("name", duplicate.name.as_str().to_owned()),
                ("kept", duplicate.kept.to_string()),
            ],
        ));
    }
    let (merged, summary) = teams_list::reconcile(&working, &incoming);
    for (id, name) in &summary.added {
        report.message(message(
            Code::TeamsListAdded,
            vec![
                ("team", id.get().to_string()),
                ("name", name.as_str().to_owned()),
            ],
        ));
    }
    for (name, old, new) in &summary.overridden {
        report.message(message(
            Code::TeamsListOverridden,
            vec![
                ("name", name.as_str().to_owned()),
                ("old", old.get().to_string()),
                ("new", new.get().to_string()),
            ],
        ));
    }
    for (id, name) in &summary.unresolved {
        report.message(message(
            Code::TeamsListUnresolved,
            vec![
                ("team", id.get().to_string()),
                ("name", name.as_str().to_owned()),
            ],
        ));
    }
    // `placeholders_added` stays empty: the save's list carries team rows alone.
    report.message(message(
        Code::TeamsListKept,
        vec![("count", summary.kept.to_string())],
    ));
    let shown = path.as_deref().map_or_else(
        || teams_list::FILE_NAME.to_owned(),
        |path| path.display().to_string(),
    );
    if !args.yes {
        report.message(message(Code::TeamsListNotWritten, vec![("path", shown)]));
        return Ok(());
    }
    let written = match &path {
        Some(path) => teams_list::write(path, &merged),
        None => Err(io::Error::other("no data directory is chosen yet")),
    };
    match written {
        Ok(()) => report.message(message(Code::TeamsListWritten, vec![("path", shown)])),
        Err(error) => report.message(message(
            Code::TeamsListReadOnly,
            vec![("path", shown), ("error", error.to_string())],
        )),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_exit_code_is_one_for_an_error_and_zero_otherwise() {
        assert_eq!(exit_code(None), 0);
        assert_eq!(exit_code(Some(Severity::Info)), 0);
        assert_eq!(exit_code(Some(Severity::Warning)), 0);
        assert_eq!(exit_code(Some(Severity::Error)), 1);
        assert_eq!(exit_code(Some(Severity::Fatal)), 1);
    }

    #[test]
    fn refuse_while_running_refuses_only_the_saves_own_version() {
        let game = |version| RunningPes { version, exe: None };
        assert!(refuse_while_running(&[], PesVersion::Pes21, false).is_ok());
        assert!(refuse_while_running(&[game(PesVersion::Pes15)], PesVersion::Pes21, false).is_ok());
        let error =
            refuse_while_running(&[game(PesVersion::Pes21)], PesVersion::Pes21, false).unwrap_err();
        assert_eq!(error.exit_code, 2);
        assert!(error.error.to_string().contains("PES 2021"));
        assert!(refuse_while_running(&[game(PesVersion::Pes21)], PesVersion::Pes21, true).is_ok());
    }

    #[test]
    fn parse_teams_reads_ids_and_inclusive_ranges_sorted_without_duplicates() {
        assert_eq!(parse_teams("718").unwrap(), vec![718]);
        assert_eq!(
            parse_teams("705-707,701,706").unwrap(),
            vec![701, 705, 706, 707]
        );
        for text in ["710-705", "7x", ""] {
            assert!(parse_teams(text).is_err(), "{text}");
        }
    }

    #[test]
    fn export_toml_requires_one_form_with_its_own_destination() {
        let parse = |args: &[&str]| {
            command()
                .try_get_matches_from(std::iter::once("save-editor").chain(args.iter().copied()))
        };
        assert!(parse(&["export-toml", "EDIT", "--team", "714", "--out", "a.toml"]).is_ok());
        assert!(parse(&["export-toml", "EDIT", "--all-teams", "--out-dir", "teams"]).is_ok());
        for bad in [
            vec!["export-toml", "EDIT"],
            vec!["export-toml", "EDIT", "--team", "714"],
            vec!["export-toml", "EDIT", "--all-teams"],
            vec!["export-toml", "EDIT", "--team", "714", "--out-dir", "teams"],
            vec![
                "export-toml",
                "EDIT",
                "--team",
                "714",
                "--out",
                "a.toml",
                "--all-teams",
                "--out-dir",
                "t",
            ],
        ] {
            assert!(parse(&bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_command_is_required() {
        // A bare `save-editor` is a usage refusal: the parse fails before a command runs,
        // so nothing is written.
        let error = command().try_get_matches_from(["save-editor"]).unwrap_err();
        assert_eq!(error.exit_code(), 2);
        assert_eq!(
            error.to_string().lines().next().unwrap(),
            "error: 'save-editor' requires a subcommand but one was not provided"
        );
    }

    #[test]
    fn parse_teams_trims_around_items_and_range_ends() {
        assert_eq!(
            parse_teams(" 701 , 705 - 707 ").unwrap(),
            vec![701, 705, 706, 707]
        );
        assert_eq!(parse_teams("701, 702").unwrap(), vec![701, 702]);
        assert_eq!(parse_teams("705 -707").unwrap(), vec![705, 706, 707]);
        assert_eq!(parse_teams("705- 707").unwrap(), vec![705, 706, 707]);
    }
}
