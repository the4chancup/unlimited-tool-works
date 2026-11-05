//! `upgrade`: one old export read, resolved, written under its derived name
//! and validated, in the order `spec/export_upgrader.md` "CLI" gives: the
//! invocation checks first (nothing written by a refusal), then the source,
//! the identity, the staging folder, the migration report and the shared
//! validation's findings on what was written.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use aesthetics_export::{
    CanonicalListing, ListedEntry, ListedKind, SmallMetadata, SourceError, ValidationContext,
    ValidationIssue, is_small_metadata, parse_listing, read_colors_txt,
};
use anyhow::anyhow;
use pes_savefile::EditFile;
use pes_version::PesVersion;
use studio_core::{
    CliError, Disposition, ExportId, Message, PipelineEvent, PipelineEventEnvelope, RunId, Scope,
    Severity, ToolContext,
};
use teams_list::{TeamId, TeamName, TeamsList};
use vtree::ScopePath;

use crate::carried::{carried_files, under};
use crate::cli::{ABORTED, CLEAN, ERRORS, INVALID, UpgradeArgs};
use crate::face_xml::{face_xml_rewrites, kept_common_files};
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
use crate::settings::{SettingsFiles, loose_settings_files, settings_files};
use crate::source::{self, OldExport, SourceFailure};
use crate::staging::{claim_staging, finish_staging, write_output_file};
use crate::texture_pairs;

/// A command is one run per process, so every event carries the same run id.
const RUN_ID: RunId = RunId(1);

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

/// What the run upgrades: a listed team's export, the referee package, or a
/// folder of model folders (no team).
#[derive(Debug)]
pub(crate) enum Identity {
    /// A listed team; `id` is strictly 701-920.
    Team {
        /// The teams-list id.
        id: TeamId,
        /// The canonical `/xx/` name.
        name: TeamName,
    },
    /// A referee export; its name's first word is `refs`.
    Referees,
    /// Model-folder mode's: no team, so no ID is written where a path or a
    /// name carried the `???` placeholder (plan "Model-folder mode").
    ModelFolders,
}

impl Identity {
    /// The three-digit ID a whole-export upgrade writes where a game path or
    /// a name carried the `???` placeholder: the team's, or 999 for a referee
    /// export, the ID the game compiles referees under (plan step 7). `None`
    /// for model-folder mode, which has no team and leaves the placeholder
    /// as written.
    pub(crate) fn path_id(&self) -> Option<u16> {
        match self {
            Identity::Team { id, .. } => Some(id.get()),
            Identity::Referees => Some(999),
            Identity::ModelFolders => None,
        }
    }
}

/// A run's event stream: the findings it reports to the console (one line
/// each) or the shell, and the worst severity among them and whether any
/// drops something, which decide the exit code.
pub(crate) struct RunEvents {
    ctx: ToolContext,
    worst: Option<Severity>,
    dropped: bool,
}

impl RunEvents {
    /// Events sent through `ctx`, none yet.
    pub(crate) fn new(ctx: &ToolContext) -> RunEvents {
        // A clone shares the context's channels, so the events reach the same
        // receivers.
        RunEvents {
            ctx: ctx.clone(),
            worst: None,
            dropped: false,
        }
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

    /// One finding, addressed to the export its scope names.
    pub(crate) fn message(&mut self, message: Message) {
        self.worst = self.worst.max(Some(message.severity));
        // A leftover is the one drop that keeps the run clean (step 10):
        // the Studio export is complete without it and the Warning tells
        // the author what was left behind. A dropped `.ftex` of a resolved
        // `.dds`/`.ftex` pair is the same: the `.dds` it duplicated stayed.
        self.dropped |= message.disposition != Disposition::Keep
            && message.code.code.as_ref() != Code::LeftoverDropped.as_str()
            && message.code.code.as_ref() != Code::TextureDuplicateDropped.as_str();
        self.emit(message.scope.export_id(), PipelineEvent::Message(message));
    }

    /// The one event for unmigrated content existing at all.
    pub(crate) fn unmigrated(&self) {
        self.emit(None, PipelineEvent::UnmigratedContentFound);
    }

    /// The export left the pipeline, whatever its outcome.
    pub(crate) fn processed(&self, export_id: ExportId) {
        self.emit(
            Some(export_id),
            PipelineEvent::ExportProcessed { export_id },
        );
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
            run_id: RUN_ID,
            export_id,
            export_revision: None,
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

/// What `check_invocation` approved (the "CLI" paragraph's checks): the
/// input resolved and named, the flags checked, the save loaded, the teams
/// list parsed.
struct Invocation {
    /// The input path, made absolute.
    path: PathBuf,
    /// The export's name: the folder's name, or the archive file's stem.
    name: String,
    /// Whether the name's first word is `refs`: a referee export.
    referees: bool,
    /// The save `--savefile` loaded, when given.
    save: Option<EditFile>,
    /// The working teams list.
    teams_list: TeamsList,
}

/// The "CLI" paragraph's invocation checks, before anything is written
/// (`run`'s order, `spec/export_upgrader.md` "CLI").
fn check_invocation(args: &UpgradeArgs, ctx: &ToolContext) -> Result<Invocation, CliError> {
    // "CLI": the export must exist and be a folder or a `.zip`/`.7z`.
    let path = &args.export;
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

    // "CLI": the savefile, when one is given, must read as a save.
    let save = match &args.savefile {
        Some(savefile) => Some(EditFile::load(savefile).map_err(|error| {
            invalid(anyhow!(
                "cannot read the savefile {}: {error}",
                savefile.display()
            ))
        })?),
        None => None,
    };

    // "CLI": the teams list, the data directory's file or the embedded one.
    let teams_list = load_teams_list(ctx.paths().data_dir.as_deref())?;
    Ok(Invocation {
        path,
        name,
        referees,
        save,
        teams_list,
    })
}

/// Runs `upgrade`: the invocation check, then the export opened and resolved,
/// written and reported, validated and judged.
pub(crate) fn run(args: &UpgradeArgs, ctx: &ToolContext) -> Result<u8, CliError> {
    let invocation = check_invocation(args, ctx)?;
    let mut events = RunEvents::new(ctx);

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
    // else the name's first word, each against the teams list. A team export's
    // Note is consumed here and below; a referee export's is a leftover
    // (step 9).
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
        &invocation.teams_list,
    ) {
        Ok(identity) => identity,
        Err((scope, name)) => {
            events.message(team_name_unknown(scope, name));
            events.processed(INPUT);
            return Ok(ERRORS);
        }
    };

    // "CLI": the savefile must hold the resolved team.
    if let (Some(save), Identity::Team { id, name }) = (&invocation.save, &identity)
        && save.team(u32::from(id.get())).is_none()
    {
        return Err(invalid(anyhow!(
            "--savefile {} does not hold the export's team {name} ({id})",
            args.savefile
                .as_ref()
                .expect("the save came from --savefile")
                .display()
        )));
    }

    // Step 2: the team's roster, when a save is given — its players by slot,
    // read by ID (loose mode and referee exports have none).
    let roster = match (&invocation.save, &identity) {
        (Some(save), Identity::Team { id, .. }) => Some(Roster::read(save, *id)),
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
    events.started(OUTPUT, &named.name);
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
        }
        if let Some(notes) = note::other_notes(note_text) {
            write_output_file(&staging, "notes.txt", format!("{notes}\n").as_bytes())?;
        }
    }
    // Step 9: the referee roster becomes the root `players.txt`; a
    // `refs.txt` that is not UTF-8 is unmigrated content and none is
    // written, the output validation then reporting the missing roster.
    match &refs_txt {
        Some((_, Some(text))) => write_output_file(&staging, "players.txt", text.as_bytes())?,
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
    let version = match &invocation.save {
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
                |a, b| match (export.read(a), export.read(b)) {
                    (Ok(a), Ok(b)) => a == b,
                    _ => false,
                },
            );
            // Step 8: a `settings.toml` per player folder, from the save's
            // records; loose mode's holds `name = true` alone (step 11).
            let settings = match (&invocation.save, &roster) {
                (Some(save), Some(roster)) => {
                    settings_files(save, roster, &plan.folders, full, &plan.not_worn)
                }
                // Loose mode: no record to read, only the folder's name.
                _ => loose_settings_files(&plan.folders),
            };
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
    let mut carried = carried_files(&files, &plan.portraits, |path| export.read(path));
    // The kit magic respells the output names of the player folders' and the
    // carried files' copies, then the model renames apply. `kept` holds the
    // `Common/` names a member `face.xml` or an xml folder's link spells,
    // computed from the respelled player-folder copies. Both lists are
    // renamed in one call, so a link sees its `Common/` target's rename. A
    // path another already takes is the later input's and is reported, the
    // finding joining its own list's findings.
    let plan_copies = respell_outputs(std::mem::take(&mut plan.copies), &identity);
    let kept = kept_common_files(&plan_copies, &identity, |path| export.read(path));
    let plan_len = plan_copies.len();
    let mut renamed = rename_models(
        plan_copies
            .into_iter()
            .chain(respell_outputs(
                std::mem::take(&mut carried.copies),
                &identity,
            ))
            .collect(),
        &kept,
    );
    let carried_copies = renamed.split_off(plan_len);
    let (copies, dropped) = drop_collisions(renamed, identity.path_id());
    plan.copies = copies;
    plan.findings.extend(dropped);
    let (copies, dropped) = drop_collisions(carried_copies, identity.path_id());
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
        match export.read(from) {
            Ok(bytes) => write_output_file(&staging, to, &bytes)?,
            Err(failure) => {
                // One file's failure does not cost the rest.
                events.message(read_failed(INPUT, failure, Disposition::DropFile));
            }
        }
    }
    for (path, bytes) in &kits.written {
        write_output_file(&staging, path, bytes)?;
    }
    for (path, bytes) in &materials.written {
        write_output_file(&staging, path, bytes)?;
    }
    for (path, bytes) in &face_xml {
        write_output_file(&staging, path, bytes)?;
    }
    for link in &materials.links {
        write_output_file(&staging, link, b"")?;
    }
    for marker in &plan.markers {
        // Step 8's filled `ingame_face.toml` supersedes the empty marker file
        // at the same path, whatever the write order.
        if settings.written.iter().any(|(path, _)| path == marker) {
            continue;
        }
        write_output_file(&staging, marker, b"")?;
    }
    for (path, bytes) in &settings.written {
        write_output_file(&staging, path, bytes)?;
    }
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
    // The input's half ends here: read and migrated, whatever it left behind.
    events.processed(INPUT);

    // "Verification": the shared validation on what was written, read in the
    // staging folder. Every issue goes through `events.message`, whose
    // disposition sets `dropped`.
    validate_output(&staging, &named.name, version, &mut events);
    // The output's half ends with its validation.
    events.processed(OUTPUT);

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
/// Note's `Team:` value folded and looked up in the teams list, else the
/// name's first word the same way. `Err((scope, name))` is `team_name_unknown`
/// reporting the `Team:` line's file or the export, with the value as written.
fn resolve(
    export_name: &str,
    referees: bool,
    team_value: Option<String>,
    note_path: Option<&str>,
    teams_list: &TeamsList,
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
        return match TeamName::new(&value)
            .ok()
            .and_then(|name| lookup(name, teams_list))
        {
            Some(identity) => Ok(identity),
            None => Err((scope, value)),
        };
    }
    match aesthetics_export::team_name(export_name).and_then(|name| lookup(name, teams_list)) {
        Some(identity) => Ok(identity),
        None => Err((Scope::Export { export_id: INPUT }, export_name.to_owned())),
    }
}

/// The team `name`'s row in the teams list, as an identity.
fn lookup(name: TeamName, teams_list: &TeamsList) -> Option<Identity> {
    teams_list
        .id_of(&name)
        .map(|id| Identity::Team { id, name })
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

/// The teams list: `<data_dir>/teams_list.txt` when present, else the embedded
/// list (the upgrader has no setting of its own for it, step 6); unreadable or
/// unparsable aborts, as the compiler's loader's.
fn load_teams_list(data_dir: Option<&Path>) -> Result<TeamsList, CliError> {
    let path = match data_dir {
        Some(dir) => dir.join("teams_list.txt"),
        None => return Ok(embedded_teams_list()),
    };
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(embedded_teams_list());
        }
        Err(error) => {
            return Err(aborted(anyhow!(
                "cannot read the teams list {}: {error}",
                path.display()
            )));
        }
    };
    TeamsList::parse(&text).map_err(|error| {
        aborted(anyhow!(
            "{} is not a valid teams list: {error}",
            path.display()
        ))
    })
}

fn embedded_teams_list() -> TeamsList {
    TeamsList::parse(TeamsList::UPSTREAM)
        .expect("the embedded teams list parses; the teams_list crate's tests check it")
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
mod tests {
    use super::*;
    use crate::test_support::{scratch, test_ctx};

    #[test]
    fn an_input_ending_in_a_dot_or_dot_dot_resolves_to_its_folder() {
        let root = scratch("resolve_input");
        fs::create_dir_all(root.join("co Spring/Other")).unwrap();
        for input in [
            root.join("co Spring").join("."),
            root.join("co Spring").join("Other").join(".."),
        ] {
            let resolved = resolve_input(&input).unwrap();
            assert_eq!(source::source_name(&resolved), "co Spring", "{input:?}");
            assert_eq!(resolved.parent(), Some(root.as_path()), "{input:?}");
        }
    }

    #[test]
    fn load_teams_list_falls_back_to_the_embedded_list() {
        let root = scratch("teams_list_missing");
        let embedded = TeamsList::parse(TeamsList::UPSTREAM).unwrap();
        // No `teams_list.txt` in the data directory, and no data directory at
        // all: the embedded list.
        assert_eq!(load_teams_list(Some(&root)).unwrap(), embedded);
        assert_eq!(load_teams_list(None).unwrap(), embedded);
    }

    #[test]
    fn load_teams_list_aborts_on_an_unreadable_or_unparsable_file() {
        let root = scratch("teams_list_bad");
        // A `teams_list.txt` that is a folder cannot be read.
        fs::create_dir(root.join("teams_list.txt")).unwrap();
        let error = load_teams_list(Some(&root)).unwrap_err();
        assert_eq!(error.exit_code, ABORTED, "{error}");
        // One that does not parse is aborted too.
        fs::remove_dir_all(&root).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("teams_list.txt"), "not a list").unwrap();
        let error = load_teams_list(Some(&root)).unwrap_err();
        assert_eq!(error.exit_code, ABORTED, "{error}");
    }

    #[test]
    fn worst_tracks_the_worst_severity_reported() {
        let (ctx, _events_rx) = test_ctx();
        let mut events = RunEvents::new(&ctx);
        assert_eq!(events.worst(), None);
        events.message(tool_message(
            Code::MigrationClean,
            Scope::Run,
            Disposition::Keep,
            vec![],
        ));
        assert_eq!(events.worst(), Some(Severity::Info));
        events.message(unmigrated(
            Scope::Run,
            Disposition::DropFile,
            UnmigratedReason::Unclaimed,
            None,
            vec![],
        ));
        assert_eq!(events.worst(), Some(Severity::Error));
        events.message(tool_message(
            Code::MigrationNotClean,
            Scope::Run,
            Disposition::Keep,
            vec![],
        ));
        assert_eq!(events.worst(), Some(Severity::Error));
    }

    #[test]
    fn a_warning_finding_that_drops_something_is_not_clean() {
        // The verdict's "nothing dropped" half sees the tool's own findings
        // too: a finding below Error that drops a folder still refuses
        // `migration_clean`, since the compile would lose the folder.
        let (ctx, _events_rx) = test_ctx();
        let mut events = RunEvents::new(&ctx);
        assert!(!events.dropped());
        events.message(tool_message(
            Code::RosterSlotEmpty,
            Scope::Folder {
                export_id: INPUT,
                path: ScopePath::new("Faces/XXX15 - A").unwrap(),
            },
            Disposition::DropFolder,
            vec![],
        ));
        assert_eq!(events.worst(), Some(Severity::Warning));
        assert!(events.dropped());
        assert_eq!(
            verdict(events.worst(), events.dropped()),
            (Code::MigrationNotClean, ERRORS)
        );
    }

    #[test]
    fn the_verdict_is_clean_only_with_no_error_and_no_drop() {
        assert_eq!(verdict(None, false), (Code::MigrationClean, CLEAN));
        assert_eq!(
            verdict(Some(Severity::Warning), false),
            (Code::MigrationClean, CLEAN)
        );
        assert_eq!(
            verdict(Some(Severity::Warning), true),
            (Code::MigrationNotClean, ERRORS)
        );
        assert_eq!(
            verdict(Some(Severity::Error), false),
            (Code::MigrationNotClean, ERRORS)
        );
    }

    #[test]
    fn a_team_color_entry_in_a_kit_spelling_keeps_its_first_color() {
        // Step 6: two parts that each read as one color; the first is kept.
        assert_eq!(
            first_of_two_colors("048 146 082 - 255 255 255"),
            Some("048 146 082")
        );
        assert_eq!(first_of_two_colors("#0070DD - #FFF468"), Some("#0070DD"));
    }

    #[test]
    fn an_entry_that_is_not_two_colors_keeps_nothing() {
        // Three parts that each read as one color are no two-color entry,
        // leading bullet or not.
        assert_eq!(
            first_of_two_colors("048 146 082 - 255 255 255 - 000 000 000"),
            None
        );
        assert_eq!(
            first_of_two_colors("- 1st: 048 146 082 - 255 255 255 - 000 000 000"),
            None
        );
        // Nor is a part that reads as two colors (the second past the
        // capacity), one refused outright, or one that is blank.
        assert_eq!(
            first_of_two_colors("048 146 082\n255 255 255 - #FFFFFF"),
            None
        );
        assert_eq!(
            first_of_two_colors("048 146 082 255 255 255 - #FFFFFF"),
            None
        );
        assert_eq!(first_of_two_colors("048 146 082 -"), None);
        // A part that is no color at all.
        assert_eq!(first_of_two_colors("blue - 255 255 255"), None);
        // And one color alone is no entry in a kit entry's spelling.
        assert_eq!(first_of_two_colors("048 146 082"), None);
    }

    #[test]
    fn a_model_file_in_any_letter_case_makes_a_loose_run_pre_fox() {
        let root = scratch("loose_model_case");
        fs::create_dir_all(root.join("co Spring/Boots/k0430 - Crocs")).unwrap();
        fs::write(root.join("co Spring/co Note.txt"), b"Team: /co/\r\n").unwrap();
        // A boots-named model: a valid name under pre-Fox, so the run is
        // clean and writes its output (an invalid one would leave none).
        fs::write(
            root.join("co Spring/Boots/k0430 - Crocs/boots.MODEL"),
            b"model",
        )
        .unwrap();
        let (ctx, _events_rx) = test_ctx();
        let args = UpgradeArgs {
            export: root.join("co Spring"),
            savefile: None,
            loose: true,
            coverage: None,
            output: None,
        };

        run(&args, &ctx).unwrap();
        // On the pre-Fox engine a worn boots folder stays shared; Fox
        // merges it into its one wearer's folder.
        assert!(
            root.join("co Full Spring/Boots/Crocs/boots.MODEL")
                .is_file(),
            "{:?}",
            root
        );
    }

    #[test]
    fn a_refs_txt_that_is_not_utf8_is_unmigrated_and_no_players_txt_is_written() {
        let root = scratch("refs_not_utf8");
        // Notepad's ANSI: an accented name is no UTF-8 (step 9's
        // `Some((path, None))`).
        fs::create_dir_all(root.join("refs Spring/Players/arbiter")).unwrap();
        fs::write(root.join("refs Spring/refs.txt"), b"01 M\xfcller\r\n").unwrap();
        fs::write(
            root.join("refs Spring/Players/arbiter/boots.fmdl"),
            b"boots",
        )
        .unwrap();
        let (ctx, events_rx) = test_ctx();
        let args = UpgradeArgs {
            export: root.join("refs Spring"),
            savefile: None,
            loose: false,
            coverage: None,
            output: None,
        };

        let exit = run(&args, &ctx).unwrap();
        drop(ctx);
        assert_eq!(exit, ERRORS);
        let messages: Vec<Message> = events_rx
            .iter()
            .filter_map(|envelope| match envelope.event {
                PipelineEvent::Message(message) => Some(message),
                _ => None,
            })
            .collect();
        let unmigrated = messages
            .iter()
            .filter(|m| m.code.code.as_ref() == "unmigrated_content")
            .collect::<Vec<_>>();
        assert_eq!(
            unmigrated.iter().map(|m| &m.scope).collect::<Vec<_>>(),
            [&Scope::File {
                export_id: INPUT,
                path: path_scope("refs.txt")
            }],
            "{:?}",
            messages
                .iter()
                .map(|m| m.code.code.as_ref().to_owned())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            unmigrated[0].context,
            [("reason".to_owned(), "roster_not_utf8".to_owned())]
        );
        // Not clean: nothing is written, the staging folder removed.
        assert!(!root.join("refs Spring upgraded").exists());
        assert!(!root.join("refs Spring upgraded.upgrading").exists());
    }

    #[test]
    fn a_second_note_and_an_unclaimed_file_get_their_reasons() {
        let root = scratch("unclaimed_reasons");
        // `team note.txt` is a second Note-named file (`co Note.txt` keeps
        // the place); `Players/` holds no folder a team export's steps read.
        fs::create_dir_all(root.join("co Spring/Players/05 - A")).unwrap();
        fs::write(root.join("co Spring/co Note.txt"), b"Team: /co/\r\n").unwrap();
        fs::write(root.join("co Spring/team note.txt"), b"Team: /co/\r\n").unwrap();
        fs::write(root.join("co Spring/Players/05 - A/face.fmdl"), b"face").unwrap();
        let (ctx, events_rx) = test_ctx();
        let args = UpgradeArgs {
            export: root.join("co Spring"),
            savefile: None,
            loose: true,
            coverage: None,
            output: None,
        };

        let exit = run(&args, &ctx).unwrap();
        drop(ctx);
        assert_eq!(exit, ERRORS);
        let findings: Vec<(String, Vec<(String, String)>)> = events_rx
            .iter()
            .filter_map(|envelope| match envelope.event {
                PipelineEvent::Message(message)
                    if message.code.code.as_ref() == "unmigrated_content" =>
                {
                    let path = match &message.scope {
                        Scope::File { path, .. } | Scope::Folder { path, .. } => {
                            path.as_str().to_owned()
                        }
                        Scope::Run | Scope::Export { .. } | Scope::RosterEntry { .. } => {
                            String::new()
                        }
                    };
                    Some((path, message.context))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            findings,
            [
                (
                    "Players/05 - A/face.fmdl".to_owned(),
                    vec![("reason".to_owned(), "unclaimed".to_owned())]
                ),
                (
                    "team note.txt".to_owned(),
                    vec![
                        ("reason".to_owned(), "note_taken".to_owned()),
                        ("other".to_owned(), "co Note.txt".to_owned()),
                    ]
                ),
            ]
        );
    }

    #[test]
    fn a_disallowed_file_type_drops_its_folder_in_the_output_validation() {
        // The output's check runs with the strict file-type check on, as the
        // compiler's default has it: a disallowed file refuses the folder and
        // stops the run.
        let output = scratch("output_issues_strict");
        fs::create_dir_all(output.join("Players/03 - A")).unwrap();
        fs::write(output.join("Players/03 - A/readme.doc"), b"x").unwrap();
        let (ctx, _events_rx) = test_ctx();
        let mut events = RunEvents::new(&ctx);
        let issues = output_issues(
            &output,
            "co Midcup",
            PesVersion::Pes21,
            |path| path.to_owned(),
            &mut events,
        );
        let issue = issues
            .iter()
            .find(|issue| issue.code == "file_type_disallowed")
            .expect("file_type_disallowed expected");
        assert_eq!(
            issue.disposition,
            aesthetics_export::Disposition::DropFolder
        );
        assert!(!issue.passed_through);
    }

    #[test]
    fn a_missing_shared_link_target_drops_its_folder() {
        // And without pass-through: a link whose target is missing drops
        // what it names rather than keeping it.
        let output = scratch("output_issues_pass_through");
        fs::create_dir_all(output.join("Players/03 - A")).unwrap();
        fs::write(output.join("Players/03 - A/Crocs.boots"), b"").unwrap();
        let (ctx, _events_rx) = test_ctx();
        let mut events = RunEvents::new(&ctx);
        let issues = output_issues(
            &output,
            "co Midcup",
            PesVersion::Pes21,
            |path| path.to_owned(),
            &mut events,
        );
        let issue = issues
            .iter()
            .find(|issue| issue.code == "link_target_missing")
            .expect("link_target_missing expected");
        assert_eq!(
            issue.disposition,
            aesthetics_export::Disposition::DropFolder
        );
        assert!(!issue.passed_through);
    }

    #[test]
    fn each_started_export_is_processed_once_after_its_start() {
        // The console's exports counter reads `0/2` for a whole upgrade when neither the
        // input nor the output is ever reported processed.
        let root = scratch("upgrade_processed");
        fs::create_dir_all(root.join("co Spring/Boots/k0430 - Crocs")).unwrap();
        fs::write(root.join("co Spring/co Note.txt"), b"Team: /co/\r\n").unwrap();
        fs::write(
            root.join("co Spring/Boots/k0430 - Crocs/boots.MODEL"),
            b"model",
        )
        .unwrap();
        let (ctx, events_rx) = test_ctx();
        let args = UpgradeArgs {
            export: root.join("co Spring"),
            savefile: None,
            loose: true,
            coverage: None,
            output: None,
        };

        run(&args, &ctx).unwrap();
        drop(ctx);

        let events: Vec<String> = events_rx
            .iter()
            .filter_map(|envelope| match envelope.event {
                PipelineEvent::ExportStarted { export_id, .. } => {
                    Some(format!("started {}", export_id.0))
                }
                PipelineEvent::ExportProcessed { export_id } => {
                    Some(format!("processed {}", export_id.0))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            events,
            [
                format!("started {}", INPUT.0),
                format!("started {}", OUTPUT.0),
                format!("processed {}", INPUT.0),
                format!("processed {}", OUTPUT.0),
            ]
        );
    }
}
