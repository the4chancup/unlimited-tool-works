//! `upgrade`: each old export read, resolved, written under its derived name
//! and validated, in the order `spec/export_upgrader.md` "CLI" gives: the
//! invocation checks first (nothing written by a refusal), then the source,
//! the identity, the staging folder, the migration report and the shared
//! validation's findings on what was written. A run of several exports
//! upgrades them one after the other, each as if it ran alone.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use aesthetics_export::{
    CanonicalListing, ListedEntry, ListedKind, SmallMetadata, SourceError, ValidationContext,
    ValidationIssue, is_small_metadata, parse_listing, read_colors_txt,
};
use anyhow::anyhow;
use pes_savefile::EditFile;
use pes_savefile::model::names::display_name;
use pes_version::PesVersion;
use studio_core::{
    CliError, Disposition, ExportId, Message, PipelineEvent, PipelineEventEnvelope, RunId, Scope,
    Severity, ToolContext,
};
use teams_list::TeamName;
use vtree::ScopePath;

use crate::carried::{carried_files, under};
use crate::cli::{ABORTED, CLEAN, ERRORS, INVALID, UpgradeArgs};
use crate::face_xml::{face_xml_rewrites, kept_common_files};
use crate::far::far_drops;
use crate::kit_magic::{drop_collisions, respell_outputs};
use crate::kits::{KitFolders, kit_folders};
use crate::materials::material_references;
use crate::messages::{
    Code, INPUT, OUTPUT, UnmigratedReason, extract_failed, issue_message, read_failed,
    team_name_unknown, tool_message, unmigrated,
};
use crate::model_names::rename_models;
use crate::naming::output_name;
use crate::note;
use crate::players::{path_scope, player_folders};
use crate::referees::{find_refs_txt, players_txt, referee_folders};
use crate::roster::Roster;
use crate::settings::{SettingsFiles, legacy_ids, loose_settings_files, settings_files};
use crate::source::{self, OldExport, SourceFailure};
use crate::staging::{
    PlannedWrite, claim_staging, finish_staging, write_output_file, write_planned,
};
use crate::texture_pairs;

/// The run id of a command's first run. Each export `upgrade` runs is a run
/// of its own, numbered on from this one, so the export ids `INPUT` and
/// `OUTPUT` always name the current export's input and output; every other
/// command has this one run.
pub(crate) const FIRST_RUN: RunId = RunId(1);

/// The input path made absolute, so its name and parent are the export's own:
/// `upgrade .` run inside an export resolves to the export folder itself, and
/// `upgrade ..` to its parent — `absolute` keeps a trailing `..` on POSIX, so
/// a path with no file name after it is canonicalized (the folder it points
/// at).
pub(crate) fn resolve_input(path: &Path) -> Result<PathBuf, CliError> {
    let resolved = std::path::absolute(path).map_err(|error| {
        invalid(anyhow!(
            "cannot resolve the path {}: {error}",
            path.display()
        ))
    })?;
    let resolved = if resolved.file_name().is_some() {
        resolved
    } else {
        fs::canonicalize(&resolved).map_err(|error| {
            invalid(anyhow!(
                "cannot resolve the path {}: {error}",
                path.display()
            ))
        })?
    };
    // A filesystem root still has no file name: it is no export (and
    // `source_name`'s `expect` holds).
    if resolved.file_name().is_none() {
        return Err(invalid(anyhow!(
            "{} is a filesystem root, not an export",
            path.display()
        )));
    }
    Ok(resolved)
}

/// The run's verdict: `migration_clean` and
/// exit 0 when no finding is an Error or Fatal and no finding drops anything
/// but a leftover; `migration_not_clean` and exit 1 otherwise, a Warning that
/// drops a folder included, since the compile would lose it.
pub(crate) fn verdict(worst: Option<Severity>, dropped: bool) -> (Code, u8) {
    if worst.is_none_or(|worst| worst < Severity::Error) && !dropped {
        (Code::MigrationClean, CLEAN)
    } else {
        (Code::MigrationNotClean, ERRORS)
    }
}

/// What the run upgrades: a team's export, the referee package, or a
/// folder of model folders (no team). The Studio format names no team ID:
/// the save's team is found by name, whatever ID the save gives it.
#[derive(Debug)]
pub(crate) enum Identity {
    /// A team, by the canonical `/xx/` name.
    Team {
        /// The canonical `/xx/` name.
        name: TeamName,
    },
    /// A referee export; its name's first word is `refs`.
    Referees,
    /// Model-folder mode's: no team.
    ModelFolders,
}

/// A run's event stream: the findings it reports to the console (one line
/// each) or the shell, and the worst severity among them and whether any
/// drops something, which decide the exit code.
pub(crate) struct RunEvents {
    ctx: ToolContext,
    run_id: RunId,
    worst: Option<Severity>,
    dropped: bool,
    processed_sent: bool,
}

impl RunEvents {
    /// Events of the run `run_id` sent through `ctx`, none yet.
    pub(crate) fn new(ctx: &ToolContext, run_id: RunId) -> RunEvents {
        // A clone shares the context's channels, so the events reach the same
        // receivers.
        RunEvents {
            ctx: ctx.clone(),
            run_id,
            worst: None,
            dropped: false,
            processed_sent: false,
        }
    }

    /// The event stream of the run after this one, none of its events sent
    /// yet: the next export of a run of several.
    fn next_run(&self) -> RunEvents {
        RunEvents::new(&self.ctx, RunId(self.run_id.0 + 1))
    }

    /// How many exports the command runs one after the other, sent before
    /// the first: the console's progress line counts them all from its
    /// first frame.
    fn exports_expected(&self, total: usize) {
        self.emit(None, PipelineEvent::ExportsExpected { total });
    }

    /// A line printed as it is, around the findings (a run of several
    /// exports' naming lines and tally).
    fn output(&self, line: String) {
        self.emit(None, PipelineEvent::Output(line));
    }

    /// An export entered the run: the console names its findings by this name.
    pub(crate) fn started(&self, export_id: ExportId, display_name: &str) {
        self.emit(
            Some(export_id),
            PipelineEvent::ExportStarted {
                export_id,
                display_name: display_name.to_owned(),
            },
        );
    }

    /// A name for the findings of `export_id`, which is no export of its own
    /// in this run (the upgrade's output, the second half of the export):
    /// the console names its findings by this name, and no counter counts
    /// it.
    pub(crate) fn named(&self, export_id: ExportId, display_name: &str) {
        self.emit(
            Some(export_id),
            PipelineEvent::ExportNamed {
                export_id,
                display_name: display_name.to_owned(),
            },
        );
    }

    /// One finding, addressed to the export its scope names.
    pub(crate) fn message(&mut self, message: Message) {
        self.worst = self.worst.max(Some(message.severity));
        // A leftover is the one drop that keeps the run clean (step 10):
        // the Studio export is complete without it and the Warning tells
        // the author what was left behind. A dropped `.ftex` of a resolved
        // `.dds`/`.ftex` pair is the same: the `.dds` it duplicated stayed.
        // So is a far-hidden model: it never drew anything in game either.
        self.dropped |= message.disposition != Disposition::Keep
            && message.code.code.as_ref() != Code::LeftoverDropped.as_str()
            && message.code.code.as_ref() != Code::TextureDuplicateDropped.as_str()
            && message.code.code.as_ref() != Code::ModelFarDropped.as_str();
        self.emit(message.scope.export_id(), PipelineEvent::Message(message));
    }

    /// The one event for unmigrated content existing at all.
    pub(crate) fn unmigrated(&self) {
        self.emit(None, PipelineEvent::UnmigratedContentFound);
    }

    /// The export left the pipeline, whatever its outcome. Sent once per run,
    /// however often it is asked: a refusal or an abort returns before the
    /// upgrade sends it, a failed rename or removal of the staging folder
    /// after, and the driver of a run of several exports asks either way.
    pub(crate) fn processed(&mut self, export_id: ExportId) {
        if self.processed_sent {
            return;
        }
        self.processed_sent = true;
        self.emit(
            Some(export_id),
            PipelineEvent::ExportProcessed { export_id },
        );
    }

    /// `processed` of `total` parts done (the files the run writes into its
    /// staging folder).
    pub(crate) fn progress(&self, processed: usize, total: usize) {
        self.emit(None, PipelineEvent::Progress { processed, total });
    }

    /// The worst severity reported so far; `None` when no finding was.
    pub(crate) fn worst(&self) -> Option<Severity> {
        self.worst
    }

    /// Whether a finding reported so far drops something (a file, a folder,
    /// the export): the verdict's "nothing dropped" half for the tool's own
    /// findings, beside the output validation's issues.
    pub(crate) fn dropped(&self) -> bool {
        self.dropped
    }

    fn emit(&self, export_id: Option<ExportId>, event: PipelineEvent) {
        self.ctx.emit(PipelineEventEnvelope {
            run_id: self.run_id,
            export_id,
            export_revision: None,
            made_at: None,
            event,
        });
    }
}

/// A refusal before anything was written.
pub(crate) fn invalid(error: anyhow::Error) -> CliError {
    CliError::new(INVALID, error)
}

/// A run aborted before anything was written, or while writing the staging
/// folder (`cli.rs`'s `ABORTED`: the incomplete staging folder is left).
pub(crate) fn aborted(error: anyhow::Error) -> CliError {
    CliError::new(ABORTED, error)
}

/// What `check_export` approved (the "CLI" paragraph's checks of one
/// export): the input resolved and named, the flags checked against its
/// kind.
struct Invocation {
    /// The input path, made absolute.
    path: PathBuf,
    /// The export's name: the folder's name, or the archive file's stem.
    name: String,
    /// Whether the name's first word is `refs`: a referee export.
    referees: bool,
}

/// The "CLI" paragraph's invocation checks of the export at `path`, before
/// anything is written (`spec/export_upgrader.md` "CLI").
fn check_export(path: &Path, args: &UpgradeArgs) -> Result<Invocation, CliError> {
    // "CLI": the export must exist and be a folder or a `.zip`/`.7z`.
    if !path.exists() {
        return Err(invalid(anyhow!("{} does not exist", path.display())));
    }
    if !path.is_dir() && !source::is_archive(path) {
        return Err(invalid(anyhow!(
            "{} is not a folder, a .zip or a .7z file",
            path.display()
        )));
    }
    let path = resolve_input(path)?;
    let name = source::source_name(&path);

    // "CLI": a referee export (the name's first word is `refs`) takes none
    // of the flags; a team export takes exactly one of --savefile and
    // --loose.
    let referees = aesthetics_export::team_name(&name).is_some_and(|name| name.is_referees());
    if referees {
        let flag = if args.savefile.is_some() {
            Some("--savefile")
        } else if args.loose {
            Some("--loose")
        } else {
            args.coverage.is_some().then_some("--coverage")
        };
        if let Some(flag) = flag {
            return Err(invalid(anyhow!(
                "{flag} does not apply to a referee export, which takes none of --savefile, \
                 --loose or --coverage"
            )));
        }
    } else if args.savefile.is_some() == args.loose {
        return Err(invalid(anyhow!(
            "a team export takes exactly one of --savefile or --loose"
        )));
    }

    Ok(Invocation {
        path,
        name,
        referees,
    })
}

/// The save `--savefile` names, loaded once for the whole run; `None`
/// without the flag. "CLI": it must read as a save.
fn load_save(args: &UpgradeArgs) -> Result<Option<EditFile>, CliError> {
    let Some(savefile) = &args.savefile else {
        return Ok(None);
    };
    let save = EditFile::load(savefile).map_err(|error| {
        invalid(anyhow!(
            "cannot read the savefile {}: {error}",
            savefile.display()
        ))
    })?;
    Ok(Some(save))
}

/// The exports a run upgrades, in order: the ones named, then `--all`'s.
fn export_paths(args: &UpgradeArgs) -> Result<Vec<PathBuf>, CliError> {
    let mut paths = args.exports.clone();
    if let Some(folder) = &args.all {
        paths.extend(exports_in(folder)?);
    }
    Ok(paths)
}

/// Every export directly inside `folder` (`--all`): its folders and its
/// `.zip` and `.7z` files, sorted by name without regard to letter case
/// (ties by the exact name), any other file passed over. A folder that is
/// missing, cannot be listed or holds no export refuses the whole run.
fn exports_in(folder: &Path) -> Result<Vec<PathBuf>, CliError> {
    if !folder.exists() {
        return Err(invalid(anyhow!("{} does not exist", folder.display())));
    }
    if !folder.is_dir() {
        return Err(invalid(anyhow!("{} is not a folder", folder.display())));
    }
    let cannot_list = |error: std::io::Error| {
        invalid(anyhow!(
            "cannot list the folder {}: {error}",
            folder.display()
        ))
    };
    let mut exports = Vec::new();
    for entry in fs::read_dir(folder).map_err(cannot_list)? {
        let entry = entry.map_err(cannot_list)?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        // A folder ending in `.upgrading` is the incomplete output an
        // interrupted run left, not an export.
        if path.is_dir() && name.to_ascii_lowercase().ends_with(".upgrading") {
            continue;
        }
        if path.is_dir() || source::is_archive(&path) {
            exports.push((name.to_lowercase(), name, path));
        }
    }
    if exports.is_empty() {
        return Err(invalid(anyhow!(
            "{} holds no export (a folder, a .zip or a .7z file)",
            folder.display()
        )));
    }
    exports.sort();
    Ok(exports.into_iter().map(|(_, _, path)| path).collect())
}

/// The name a run of several exports gives `path` on its naming line: the
/// folder's or the archive file's name, as the path ends.
fn naming_line_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// Runs `upgrade`: the exports listed and the save loaded once, then each
/// export upgraded as if it ran alone. A run of one named export prints what
/// that export's upgrade prints. A run of several, or of `--all`, prints a
/// line naming each export before its findings, a refused or aborted
/// export's error as the last line of its block, and the tally of exports
/// upgraded at the end. The exit code is the highest any export gave.
pub(crate) fn run(args: &UpgradeArgs, ctx: &ToolContext) -> Result<u8, CliError> {
    let paths = export_paths(args)?;
    if let ([path], None) = (paths.as_slice(), &args.all) {
        // The export's own checks before the save's, as a run of one export
        // has always made them: the same refusal wins.
        let invocation = check_export(path, args)?;
        let save = load_save(args)?;
        let mut events = RunEvents::new(ctx, FIRST_RUN);
        return upgrade_export(&invocation, args, save.as_ref(), &mut events);
    }
    let save = load_save(args)?;
    let mut events = RunEvents::new(ctx, FIRST_RUN);
    events.exports_expected(paths.len());
    let mut worst = CLEAN;
    let mut upgraded = 0;
    for (index, path) in paths.iter().enumerate() {
        if index > 0 {
            events = events.next_run();
            events.output(String::new());
        }
        events.output(format!("Upgrading {}", naming_line_name(path)));
        let outcome = check_export(path, args)
            .and_then(|invocation| upgrade_export(&invocation, args, save.as_ref(), &mut events));
        let exit_code = match outcome {
            Ok(exit_code) => exit_code,
            Err(error) => {
                // The progress line counts every export done once, whatever
                // its outcome.
                events.processed(INPUT);
                events.output(format!("error: {error}"));
                error.exit_code
            }
        };
        worst = worst.max(exit_code);
        if exit_code == CLEAN {
            upgraded += 1;
        }
    }
    events.output(String::new());
    events.output(format!(
        "Upgraded: {upgraded}. Not upgraded: {}.",
        paths.len() - upgraded
    ));
    Ok(worst)
}

/// Upgrades one export, which `check_export` approved, with the run's save:
/// the export opened and resolved, written and reported, validated and
/// judged. A refusal or an abort is the `Err`, the export's verdict the exit
/// code.
fn upgrade_export(
    invocation: &Invocation,
    args: &UpgradeArgs,
    save: Option<&EditFile>,
    events: &mut RunEvents,
) -> Result<u8, CliError> {
    // Step 1: the export itself.
    events.started(INPUT, &invocation.name);
    let mut export = match OldExport::open(&invocation.path) {
        Ok(export) => export,
        Err(failure) => {
            events.message(extract_failed(INPUT, failure));
            events.processed(INPUT);
            return Ok(ERRORS);
        }
    };

    // Step 1: nothing an old export holds at the root is no old export.
    if !source::holds_old_export(&export.files, &export.folders) {
        events.message(tool_message(
            Code::NoOldExport,
            Scope::Export { export_id: INPUT },
            Disposition::DropExport,
            vec![],
        ));
        events.processed(INPUT);
        return Ok(ERRORS);
    }

    // Step 6: the identity — a referee export, else the Note's `Team:` line,
    // else the name's first word, each checked against no list (a team the
    // current list no longer holds still upgrades). A team export's Note is
    // consumed here and below; a referee export's is a leftover (step 9).
    let note_path = note::find(&export.files).map(str::to_owned);
    let note_text = match (&note_path, invocation.referees) {
        (Some(note_path), false) => match export.read(note_path) {
            Ok(bytes) => Some(note::text(&bytes)),
            Err(failure) => {
                // The Note decides the team: its failure stops the run with
                // nothing written.
                events.message(read_failed(INPUT, failure, Disposition::DropExport));
                events.processed(INPUT);
                return Ok(ERRORS);
            }
        },
        _ => None,
    };
    let team_value = note_text.as_deref().and_then(note::team_line);
    let identity = match resolve(
        &export.name,
        invocation.referees,
        team_value,
        note_path.as_deref(),
    ) {
        Ok(identity) => identity,
        Err((scope, name)) => {
            events.message(team_name_unknown(scope, name));
            events.processed(INPUT);
            return Ok(ERRORS);
        }
    };

    // "CLI": the savefile must hold the resolved team — the first in the
    // save's record order folding to its name, whatever ID the save gives
    // it; each later one of the name is reported and not read (step 6).
    let save_team = match (save, &identity) {
        (Some(save), Identity::Team { name }) => match team_in_save(save, name, events) {
            Some(id) => Some(id),
            None => {
                return Err(invalid(anyhow!(
                    "--savefile {} does not hold the export's team {name}",
                    args.savefile
                        .as_ref()
                        .expect("the save came from --savefile")
                        .display()
                )));
            }
        },
        (None, _) | (Some(_), Identity::Referees | Identity::ModelFolders) => None,
    };

    // Step 2: the team's roster, when a save is given — its players by slot,
    // read by the save team's ID (loose mode and referee exports have none).
    let roster = match (save, save_team) {
        (Some(save), Some(id)) => Some(Roster::read(save, id)),
        _ => None,
    };

    // Step 9: a referee export's roster is its root `refs.txt` or `Refs
    // list.txt`, read before anything is written: without one there is
    // nothing to migrate, and a
    // read failure stops the run as the Note's does (the file decides the
    // roster). The path is kept with the `players.txt` text its bytes give
    // (`None` when they are not UTF-8).
    let refs_txt = match &identity {
        Identity::Referees => {
            let Some(path) = find_refs_txt(&export.files).map(str::to_owned) else {
                events.message(tool_message(
                    Code::RefsTxtMissing,
                    Scope::Export { export_id: INPUT },
                    Disposition::DropExport,
                    vec![],
                ));
                events.processed(INPUT);
                return Ok(ERRORS);
            };
            match export.read(&path) {
                Ok(bytes) => Some((path, players_txt(&bytes))),
                Err(failure) => {
                    events.message(read_failed(INPUT, failure, Disposition::DropExport));
                    events.processed(INPUT);
                    return Ok(ERRORS);
                }
            }
        }
        Identity::Team { .. } | Identity::ModelFolders => None,
    };

    // "CLI": the output folder, refused when it exists or cannot be made.
    // Its name carries the coverage tag, and the tag's choice is reported on
    // every team export's upgrade, so the name's guess is never silent
    // (spec's "CLI": a mid-cup export under `Full` compiles placeholder kits
    // over the team's installed ones).
    let named = output_name(&export.name, &identity, args.coverage)?;
    let base = args.output.clone().unwrap_or_else(|| {
        invocation
            .path
            .parent()
            .expect("the input is absolute and not a filesystem root (resolve_input)")
            .to_path_buf()
    });
    let output = base.join(&named.name);
    // "CLI": the staging folder beside the output, claimed before anything
    // is written; an earlier run's leftover is removed and reported.
    let (staging, removed) = claim_staging(&output)?;
    events.named(OUTPUT, &named.name);
    if let Some(choice) = named.tag {
        events.message(tool_message(
            Code::CoverageTagChosen,
            Scope::Export { export_id: INPUT },
            Disposition::Keep,
            vec![
                ("tag", choice.tag.to_owned()),
                ("source", choice.source.as_str().to_owned()),
            ],
        ));
    }
    if removed {
        events.message(tool_message(
            Code::StagingFolderRemoved,
            Scope::Export { export_id: OUTPUT },
            Disposition::Keep,
            vec![("path", staging.display().to_string())],
        ));
    }

    // The Note's team colors become the root `colors.txt`, its "Other Notes"
    // section the root `notes.txt` (step 6; a referee export's Note gives
    // neither). An entry whose colors do not read is unmigrated content
    // naming the Note with its line — save one holding two colors, which
    // keeps its first and reports the second a leftover; the kept entries
    // are written as the Note spells them, one per line. The Note itself is
    // never written.
    let mut any_unmigrated = false;
    // The files written at the output's root next count as parts already done when the first
    // `Progress` goes.
    let mut root_files = 0;
    if let Some(note_text) = &note_text {
        let note_scope = || Scope::File {
            export_id: INPUT,
            path: ScopePath::new(
                note_path
                    .as_deref()
                    .expect("a Note text exists only when a Note was found"),
            )
            .expect("every listed path was checked when the source opened"),
        };
        let mut colors = String::new();
        for entry in note::color_entries(note_text).team {
            let read = read_colors_txt(entry.colors.as_bytes(), 1);
            if read.colors.len() == 1 && read.refused.is_empty() {
                colors.push_str(&entry.colors);
                colors.push('\n');
                continue;
            }
            // A team-color entry in a kit entry's spelling kept its first
            // color under the old compile: it is written, the second a
            // leftover (step 6). Any other entry that does not read is
            // unmigrated content.
            if let Some(first) = first_of_two_colors(&entry.colors) {
                colors.push_str(first);
                colors.push('\n');
                events.message(tool_message(
                    Code::LeftoverDropped,
                    note_scope(),
                    Disposition::DropFile,
                    vec![("line", entry.line)],
                ));
            } else {
                any_unmigrated = true;
                events.message(unmigrated(
                    note_scope(),
                    Disposition::DropFile,
                    UnmigratedReason::ColorsUnreadable,
                    None,
                    vec![("line", entry.line)],
                ));
            }
        }
        if !colors.is_empty() {
            write_output_file(&staging, "colors.txt", colors.as_bytes())?;
            root_files += 1;
        }
        if let Some(notes) = note::other_notes(note_text) {
            write_output_file(&staging, "notes.txt", format!("{notes}\n").as_bytes())?;
            root_files += 1;
        }
    }
    // Step 9: the referee roster becomes the root `players.txt`; a
    // `refs.txt` that is not UTF-8 is unmigrated content and none is
    // written, the output validation then reporting the missing roster.
    match &refs_txt {
        Some((_, Some(text))) => {
            write_output_file(&staging, "players.txt", text.as_bytes())?;
            root_files += 1;
        }
        Some((path, None)) => {
            any_unmigrated = true;
            events.message(unmigrated(
                Scope::File {
                    export_id: INPUT,
                    path: path_scope(path),
                },
                Disposition::DropFile,
                UnmigratedReason::RosterNotUtf8,
                None,
                vec![],
            ));
        }
        None => {}
    }

    // Step 9's other layout: a referee export without `Players/` stands its
    // referee folders at the root (`referees.rs`'s `root_layout`), so files
    // under a top folder no AET name covers are inside the format's layout,
    // not leftovers. Mirrored here for the version check and the fallback,
    // both of which run before the referee plan is built.
    let referees_at_root = invocation.referees
        && !export
            .files
            .iter()
            .any(|file| under(file, "Players").is_some());

    // The migration's target version decides the engine now (steps 3-4 merge
    // only on Fox); "Verification" validates with it too. A `.model` counts
    // only where the old format's layout puts it: a leftover's decides
    // nothing (the real KFG VGL26 export, a Fox one, keeps `.model` copies
    // of its boots in a side folder `FMDL Boots/`).
    let version = match save {
        Some(save) => save.version(),
        None => {
            if export.files.iter().any(|file| {
                file.to_ascii_lowercase().ends_with(".model")
                    && !is_leftover(file, invocation.referees, referees_at_root)
            }) {
                PesVersion::Pes16
            } else {
                PesVersion::Pes21
            }
        }
    };

    // What each identity contributes: a team export, steps 2-4's plan for
    // the face and item folders, step 5's kits and step 8's `settings.toml`
    // files; a referee export, step 9's root files and referee folders
    // (`referees.rs`), and no kits or settings (it holds no `Kit Configs`/
    // `Kit Textures`: the fallback reports any). Then, for both, step 7's
    // carried folders, the kit magic, the model renames, the `.mtl`
    // rewrites and the `face.xml` respells: `run`'s findings, then every
    // file copied and written.
    let files = export.files.clone();
    let folders = export.folders.clone();
    // A model every vertex of which sits past the far checks' 5000 units
    // is dropped before the folders are planned: the compiler refuses
    // that geometry, so the export's player would lose his folder at
    // compile time. The steps then see the folders without it — a gloves
    // or boots folder it leaves modelless hides the hands or feet as any
    // modelless one does (`far.rs`).
    let far = far_drops(&files, referees_at_root, |path| export.read(path));
    let files: Vec<String> = files
        .into_iter()
        .filter(|file| !far.dropped.contains(file))
        .collect();
    let (mut plan, mut kits, settings) = match &identity {
        Identity::Referees => (
            referee_folders(&files, |path| export.read(path)),
            KitFolders::default(),
            SettingsFiles::default(),
        ),
        Identity::Team { .. } => {
            // The coverage tag decides step 5's `all/` kits and step 8's
            // worn-0 and missing-custom rules alike.
            let full = matches!(
                aesthetics_export::coverage(&named.name),
                Some(aesthetics_export::ExportCoverage::Full)
            );
            let plan = player_folders(
                &files,
                &folders,
                roster.as_ref(),
                version.engine(),
                args.legacy_ids,
                |a, b| match (export.read(a), export.read(b)) {
                    (Ok(a), Ok(b)) => a == b,
                    _ => false,
                },
            );
            // Step 8: a `settings.toml` per player folder, from the save's
            // records; loose mode's holds `name = true` alone (step 11).
            let mut settings = match (save, &roster) {
                (Some(save), Some(roster)) => {
                    settings_files(save, roster, &plan.folders, full, &plan.not_worn)
                }
                // Loose mode: no record to read, only the folder's name.
                _ => loose_settings_files(&plan.folders),
            };
            // The root `legacy_ids.toml`, the worn folders' old IDs: only
            // `--legacy-ids` writes it — its presence is what turns the Team
            // compiler's legacy packing on — and only a team export reaches
            // this branch.
            if args.legacy_ids
                && let Some(file) = legacy_ids(&plan.folders)
            {
                settings.written.push(("legacy_ids.toml".to_owned(), file));
            }
            let kits = kit_folders(
                &files,
                version,
                note_path.as_deref().zip(note_text.as_deref()),
                full,
                |path| export.read(path),
            );
            (plan, kits, settings)
        }
        Identity::ModelFolders => {
            unreachable!("upgrade's resolve yields a team or the referees")
        }
    };
    // The dropped models' files are accounted for, so the unmigrated
    // fallback below skips them.
    plan.consumed.extend(far.dropped.iter().cloned());
    let mut carried = carried_files(&files, &plan.portraits, |path| export.read(path));
    // The kit magic respells the output names of the player folders' and the
    // carried files' copies, then the model renames apply. `kept` holds the
    // `Common/` names a member `face.xml` or an xml folder's link spells,
    // computed from the respelled player-folder copies. Both lists are
    // renamed in one call, so a link sees its `Common/` target's rename. A
    // path another already takes is the later input's and is reported, the
    // finding joining its own list's findings.
    let plan_copies = respell_outputs(std::mem::take(&mut plan.copies));
    let kept = kept_common_files(&plan_copies, |path| export.read(path));
    let plan_len = plan_copies.len();
    let mut renamed = rename_models(
        plan_copies
            .into_iter()
            .chain(respell_outputs(std::mem::take(&mut carried.copies)))
            .collect(),
        &kept,
    );
    let carried_copies = renamed.split_off(plan_len);
    let (copies, dropped) = drop_collisions(renamed);
    plan.copies = copies;
    plan.findings.extend(dropped);
    let (copies, dropped) = drop_collisions(carried_copies);
    carried.copies = copies;
    carried.findings.extend(dropped);
    // A `.dds` beside a `.ftex` of one stem in one output folder is a
    // `texture_stem_conflict` the validation would refuse: the `.ftex`
    // drops now so the links below are spelled after the kept `.dds`
    // alone; whether the `.dds` follows is decided once step 7 says which
    // stems the models wear. The files stay consumed — the pair's findings
    // account for them.
    let pairs = texture_pairs::stem_pairs(
        plan.copies
            .iter()
            .chain(&carried.copies)
            .chain(&kits.copies),
    );
    let ftex_dropped: BTreeSet<&str> = pairs.iter().map(|pair| pair.ftex.0.as_str()).collect();
    plan.copies
        .retain(|(input, _)| !ftex_dropped.contains(input.as_str()));
    carried
        .copies
        .retain(|(input, _)| !ftex_dropped.contains(input.as_str()));
    kits.copies
        .retain(|(input, _)| !ftex_dropped.contains(input.as_str()));
    // `Common/` for the materials: the carried files' respelled output
    // paths, which the links are spelled after.
    let common: Vec<String> = carried
        .copies
        .iter()
        .filter(|(_, output)| under(output, "common").is_some())
        .map(|(_, output)| output.clone())
        .collect();
    let all_copies: Vec<(String, String)> =
        plan.copies.iter().chain(&carried.copies).cloned().collect();
    let mut materials =
        material_references(&all_copies, &common, &identity, |path| export.read(path));
    // Now that `materials.used` says which stems the models wear: a used
    // pair keeps its `.dds` and names the dropped `.ftex`; an unused one
    // drops both as leftovers, a `dummy_` stem counting as unused.
    let (pair_findings, dds_dropped) = texture_pairs::resolve(&pairs, &materials.used);
    if !dds_dropped.is_empty() {
        plan.copies
            .retain(|(input, _)| !dds_dropped.contains(input));
        carried
            .copies
            .retain(|(input, _)| !dds_dropped.contains(input));
        kits.copies
            .retain(|(input, _)| !dds_dropped.contains(input));
    }
    materials.findings.extend(pair_findings);
    let face_xml = face_xml_rewrites(&plan.copies, &identity, |path| export.read(path));
    for message in plan
        .findings
        .into_iter()
        .chain(far.findings)
        .chain(kits.findings)
        .chain(carried.findings)
        .chain(materials.findings)
        .chain(settings.findings)
    {
        if message.code.code.as_ref() == Code::UnmigratedContent.as_str() {
            any_unmigrated = true;
        }
        events.message(message);
    }
    // The files the run writes, each a part the progress line counts once
    // planning is over: the copies a rewrite does not replace, then each
    // list's own files. A rewritten `.mtl` or `face.xml` replaces its
    // input's copy, and a written `ingame_face.toml` the empty marker at
    // its path (step 8), so each decision is made once, here.
    let mut planned = Vec::new();
    for (from, to) in plan
        .copies
        .iter()
        .chain(&plan.untouched)
        .chain(&kits.copies)
        .chain(&carried.copies)
    {
        if materials.written.iter().any(|(output, _)| output == to)
            || face_xml.iter().any(|(output, _)| output == to)
        {
            // A rewritten `.mtl` or `face.xml` replaces its input's copy.
            continue;
        }
        planned.push(PlannedWrite::Copy {
            input: from,
            output: to,
        });
    }
    for (path, bytes) in kits
        .written
        .iter()
        .chain(&materials.written)
        .chain(&face_xml)
    {
        planned.push(PlannedWrite::Bytes {
            output: path,
            bytes,
        });
    }
    for link in &materials.links {
        planned.push(PlannedWrite::Bytes {
            output: link,
            bytes: &[],
        });
    }
    for marker in &plan.markers {
        // Step 8's filled `ingame_face.toml` supersedes the empty marker file
        // at the same path, whatever the write order.
        if settings.written.iter().any(|(path, _)| path == marker) {
            continue;
        }
        planned.push(PlannedWrite::Bytes {
            output: marker,
            bytes: &[],
        });
    }
    for (path, bytes) in &settings.written {
        planned.push(PlannedWrite::Bytes {
            output: path,
            bytes,
        });
    }
    write_planned(&staging, &planned, root_files, events, |path| {
        export.read(path)
    })?;
    // Every player folder exists even when no file lands in it: an empty
    // face folder's player folder is what makes the compiler write the
    // blank face (step 3).
    for folder in plan.folders.values() {
        fs::create_dir_all(staging.join(&folder.path)).map_err(|error| {
            aborted(anyhow!(
                "cannot create the folder {}: {error}. The output {} is incomplete",
                staging.join(&folder.path).display(),
                staging.display()
            ))
        })?;
    }

    // Step 10: every input file no step consumed is unmigrated content — or
    // a leftover, content outside the old format's layout (`is_leftover`)
    // or what a step recognized as never having reached the game (the
    // steps' `leftovers` sets), dropped with a Warning that keeps the run
    // clean.
    for file in &export.files {
        // A team export's Note is consumed (the root files above); a referee
        // export's is a leftover (step 9), as `is_leftover` reads it.
        if !invocation.referees && note_path.as_deref() == Some(file.as_str()) {
            continue;
        }
        if plan.consumed.contains(file)
            || kits.consumed.contains(file)
            || carried.consumed.contains(file)
        {
            continue;
        }
        let scope = Scope::File {
            export_id: INPUT,
            path: ScopePath::new(file)
                .expect("every listed path was checked when the source opened"),
        };
        if is_leftover(file, invocation.referees, referees_at_root)
            || plan.leftovers.contains(file)
            || kits.leftovers.contains(file)
            || carried.leftovers.contains(file)
        {
            events.message(tool_message(
                Code::LeftoverDropped,
                scope,
                Disposition::DropFile,
                vec![],
            ));
            continue;
        }
        any_unmigrated = true;
        // A second Note-named file is a candidate for the one Note the
        // export's colors come from; `other` the kept Note's path. Anything
        // else no step claimed (step 10).
        let (reason, other) = if !invocation.referees && note::is_note_name(file) {
            (UnmigratedReason::NoteTaken, note_path.clone())
        } else {
            (UnmigratedReason::Unclaimed, None)
        };
        events.message(unmigrated(
            scope,
            Disposition::DropFile,
            reason,
            other,
            vec![],
        ));
    }
    if any_unmigrated {
        events.unmigrated();
    }

    // "Verification": the shared validation on what was written, read in the
    // staging folder. Every issue goes through `events.message`, whose
    // disposition sets `dropped`.
    validate_output(&staging, &named.name, version, events);
    // The export ends once its output is validated.
    events.processed(INPUT);

    // "CLI": the verdict line and the exit code: not clean when a finding
    // was an Error or dropped anything, the tool's own and the output
    // validation's alike. The verdict publishes the staging folder or removes
    // it: only a clean run writes its output.
    let (code, exit_code) = verdict(events.worst(), events.dropped());
    events.message(tool_message(code, Scope::Run, Disposition::Keep, vec![]));
    finish_staging(&staging, &output, exit_code == CLEAN)?;
    Ok(exit_code)
}

/// The first color of a team-color entry spelled as a kit entry's (step 6):
/// `colors` split at its one `-` into two parts that each read as exactly
/// one color, the trimmed first part. `None` for any other shape, which the
/// caller reports as unmigrated content.
fn first_of_two_colors(colors: &str) -> Option<&str> {
    let parts: Vec<&str> = colors.split('-').map(str::trim).collect();
    let two_colors = parts.len() == 2
        && parts.iter().all(|part| {
            let read = read_colors_txt(part.as_bytes(), 1);
            read.colors.len() == 1 && read.refused.is_empty()
        });
    two_colors.then_some(parts[0])
}

/// The export's identity (step 6): `Referees` for a referee export, else the
/// Note's `Team:` value folded, else the name's first word folded — each
/// checked against no list, so a team no list holds still resolves.
/// `Err((scope, name))` is `team_name_unknown` reporting the `Team:` line's
/// file or the export, with the value as written.
fn resolve(
    export_name: &str,
    referees: bool,
    team_value: Option<String>,
    note_path: Option<&str>,
) -> Result<Identity, (Scope, String)> {
    if referees {
        return Ok(Identity::Referees);
    }
    if let Some(value) = team_value {
        let scope = Scope::File {
            export_id: INPUT,
            path: ScopePath::new(note_path.expect("a `Team:` value came from a Note"))
                .expect("every listed path was checked when the source opened"),
        };
        return match TeamName::new(&value) {
            Ok(name) => Ok(Identity::Team { name }),
            Err(_) => Err((scope, value)),
        };
    }
    match aesthetics_export::team_name(export_name) {
        Some(name) => Ok(Identity::Team { name }),
        None => Err((Scope::Export { export_id: INPUT }, export_name.to_owned())),
    }
}

/// The save's team of the export's name (step 6): the ID of the first of
/// `save.teams()` in record order whose name, colour codes stripped
/// (`display_name`), trims and folds to `name` — whatever ID the save
/// gives it, the save the export was made against numbering the team
/// otherwise than today. Each later one folding to the same name is
/// reported (`save_team_name_repeated`, W, on the export; `team` the
/// later team's ID, `kept` the first's) and not read. `None` when the
/// save holds none.
fn team_in_save(save: &EditFile, name: &TeamName, events: &mut RunEvents) -> Option<u32> {
    let mut first = None;
    for team in save.teams() {
        let display = display_name(&team.name);
        let Ok(found) = TeamName::new(display.trim()) else {
            continue;
        };
        if found != *name {
            continue;
        }
        match first {
            None => first = Some(team.id),
            Some(kept) => events.message(tool_message(
                Code::SaveTeamNameRepeated,
                Scope::Export { export_id: INPUT },
                Disposition::Keep,
                vec![("team", team.id.to_string()), ("kept", kept.to_string())],
            )),
        }
    }
    first
}

/// Whether the input `path` is a leftover (step 10's two kinds of entry):
/// content outside the old format's layout that nothing compiled, dropped
/// with a Warning rather than as `unmigrated_content` — a leftover the
/// Studio export does not need never blocks a migration; the author is
/// told. A file at the root is one: a referee export's unconsumed root file
/// always (the referee plan consumes its own root files — `refs.txt`/`refs
/// list.txt`, `ref_marker.dds` — before this fallback asks, and nothing
/// compiled reads a referee's Note either, step 9); a team export's unless
/// it is a Note. And so is a file directly in `Faces/`, `Boots/` or
/// `Gloves/` (the old walks list subfolders only: archives made to share
/// model folders, removed before a compile), a file under `Other/` (the
/// folder the AET keeps for the author alone) or under a top-level folder
/// `OLD_FOLDERS` does not name — save that `referees_at_root` stands a
/// referee export's referee folders there (`referees.rs`), so theirs are
/// inside the format.
fn is_leftover(path: &str, referees: bool, referees_at_root: bool) -> bool {
    let Some((top, rest)) = path.split_once('/') else {
        return referees || !note::is_note_name(path);
    };
    if !rest.contains('/')
        && ["faces", "boots", "gloves"]
            .iter()
            .any(|folder| top.eq_ignore_ascii_case(folder))
    {
        return true;
    }
    if source::OLD_FOLDERS
        .iter()
        .any(|folder| top.eq_ignore_ascii_case(folder))
    {
        return top.eq_ignore_ascii_case("other");
    }
    !referees_at_root
}

/// Lists `output`, parses it back and validates it ("Verification"), reporting
/// every issue on the output export; nothing when the listing cannot be read
/// or parsed (`export_extract_failed`, already reported).
fn validate_output(output: &Path, output_name: &str, version: PesVersion, events: &mut RunEvents) {
    for issue in output_issues(output, output_name, version, |path| path.to_owned(), events) {
        events.message(issue_message(&issue, OUTPUT));
    }
}

/// The shared validation's issues on the folder `output` ("Verification"),
/// each listed path given to the validation at `place(path)` (every
/// ancestor of a placed path listed as a folder too), its small metadata
/// read from where it was written, the listing named `display_name`. Empty,
/// `export_extract_failed` on the output reported instead, when the
/// listing cannot be read or parsed.
pub(crate) fn output_issues(
    output: &Path,
    display_name: &str,
    version: PesVersion,
    place: impl Fn(&str) -> String,
    events: &mut RunEvents,
) -> Vec<ValidationIssue> {
    let (files, folders) = match source::list_folder(output) {
        Ok(listed) => listed,
        Err(failure) => {
            events.message(extract_failed(OUTPUT, failure));
            return Vec::new();
        }
    };
    let mut metadata = BTreeMap::new();
    let mut entries = Vec::new();
    for (path, size) in &files {
        let placed = place(path);
        if is_small_metadata(&placed) {
            metadata.insert(
                placed.clone(),
                fs::read(output.join(path)).map_err(|error| error.to_string()),
            );
        }
        entries.push(ListedEntry {
            path: placed,
            kind: ListedKind::File { size: *size },
        });
    }
    let mut placed_folders: BTreeSet<String> = BTreeSet::new();
    for path in &folders {
        placed_folders.insert(place(path));
    }
    // Every ancestor of a placed path is listed as a folder too: the staged
    // `Players`/`Boots`/`Gloves` tops exist nowhere on disk.
    let mut ancestors: BTreeSet<String> = BTreeSet::new();
    for placed in placed_folders
        .iter()
        .cloned()
        .chain(entries.iter().map(|entry| entry.path.clone()))
    {
        let mut rest = placed.as_str();
        while let Some((parent, _)) = rest.rsplit_once('/') {
            ancestors.insert(parent.to_owned());
            rest = parent;
        }
    }
    for path in &folders {
        entries.push(ListedEntry {
            path: place(path),
            kind: ListedKind::Folder,
        });
    }
    for path in ancestors.difference(&placed_folders) {
        entries.push(ListedEntry {
            path: path.clone(),
            kind: ListedKind::Folder,
        });
    }
    let listing = CanonicalListing {
        display_name: display_name.to_owned(),
        entries,
    };
    let metadata = SmallMetadata { files: metadata };
    let parsed = match parse_listing(listing, metadata) {
        Ok(parsed) => parsed,
        Err(error) => {
            let (path, error) = match error {
                SourceError::Path { path, error } => (path, error.to_string()),
                SourceError::Collision { path, error } => (path, error.to_string()),
            };
            events.message(extract_failed(OUTPUT, SourceFailure { path, error }));
            return Vec::new();
        }
    };
    let context = ValidationContext {
        version,
        // On, as the compiler's default has it: a file type no step
        // expected stops the run rather than being written. Off, the output
        // was written and the compile then refused the player's folder, so
        // it was not the clean export a clean run promises.
        strict_file_type_check: true,
        pass_through: false,
        installed_common_textures: BTreeSet::new(),
    };
    parsed.validate(&context).issues
}

#[cfg(test)]
mod tests;
