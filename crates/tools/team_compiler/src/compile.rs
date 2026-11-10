//! The `compile` command (`spec/team_compiler/pipeline.md` "Run driver shapes"). For a run
//! that deploys, it first checks that it can install into the game. Then it runs the
//! validation `check` runs (the structure pass and the deep pass) and run planning. Each task's
//! files are read in manifest order and the task processed on the worker pool, and the writer
//! thread commits the batches in manifest order.
//!
//! At the end the CPK is installed into the game's `download/` or promoted from staging to the
//! output folder: in multi-CPK mode, the bins CPK and the teams parts; with a refs export, the
//! refs CPK too; in test and sideload mode, the loose tree promoted to its place. The staging
//! is discarded instead when writing or promoting it fails, when a team does not fit the teams
//! parts, or when an export's file changes while it is read, so the previous CPK stays as it
//! was.

use std::collections::BTreeMap;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aesthetics_export::ExportIdentity;
use anyhow::Context;
use crossbeam_channel::{Receiver, Sender, unbounded};
use pes_savefile::interchange::aesthetics_patch::{AestheticsPatch, PatchTeam};
use pes_version::PesVersion;
use pipeline::{Cancelled, CpkStem, MemoryBudget, Permit};
use studio_core::{Disposition, ExportId, Message, RunningPes, Scope, Severity, ToolContext};

use crate::bins::WorkingBins;
use crate::bins::installed::{self, InstalledPaths};
use crate::cli::RunInputs;
use crate::events::RunEvents;
use crate::messages::{Code, size_text, tool_message};
use crate::output::deploy::{self, DeployFailure, Download, Installed, Staging};
use crate::output::parts::{self, TeamsParts, Unplaced};
use crate::output::patch;
use crate::output::savefile;
use crate::output::sink::OutputSink;
use crate::output::teamnotes;
use crate::output::writer::{BinsEdits, CpkOutput, Referees};
use crate::paths::REFEREE_TEAM_ID;
use crate::plan::{
    BuildManifest, BuildTask, ExportToPlan, overrides, patch as plan_patch, plan_run,
};
use crate::processing::{
    CompileContext, EntryTarget, TEST_BINS_PREFIX, TaskBatch, TaskFiles, dropped_unit, process_task,
};
use crate::reader::{self, ContentSource, ExportSource, SourceFailure, SourceKind, SourceRevision};
use crate::settings::SavefilePath;
use crate::templates::{self, Templates};
use crate::validation::{run_budget, run_pool, validation_pass};

/// Where a compile puts what it writes, decided on the command line before the run starts.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputMode {
    /// The CPKs of the run's `CpkLayout`, installed into the PES folder's `download/`, or
    /// promoted to the output folder when the run cannot install them (its finding says why)
    /// or `no_deploy` is set (the run says it installed nothing).
    Normal { no_deploy: bool },
    /// What the compiler made of each export, as loose files that replace the output folder's
    /// `test_output/`: one folder per export, the model packages unpacked, the bins under
    /// `_bins/`. Nothing is installed, and the overrides are not applied: an override is not
    /// an export's.
    Test,
    /// The CPK's entries as loose files that replace the contents of `pes_folder`'s `livecpk/`,
    /// which a sideloading runtime serves to the running game. `pes_folder` was checked to be a
    /// folder.
    Sideload { pes_folder: PathBuf },
}

impl OutputMode {
    /// Whether the run installs into the PES folder's `download/`: only a normal run without
    /// `--no-deploy` does, so a missing `DpFileList.bin` is an Error for it alone.
    fn deploys(&self) -> bool {
        match self {
            OutputMode::Normal { no_deploy } => !no_deploy,
            OutputMode::Test | OutputMode::Sideload { .. } => false,
        }
    }

    /// Whether the run applies the `overrides/` files: test mode shows the exports' own output
    /// alone.
    fn applies_overrides(&self) -> bool {
        match self {
            OutputMode::Normal { .. } | OutputMode::Sideload { .. } => true,
            OutputMode::Test => false,
        }
    }
}

/// How the savefile step treats the compile target's game running: the running-PES probe it
/// asks, and whether the member said the game's Edit mode is closed (`--edit-mode-closed`),
/// which lets the step write the save while the game runs.
pub(crate) struct RunningGame {
    pub(crate) probe: fn() -> Vec<RunningPes>,
    pub(crate) edit_mode_closed: bool,
}

/// The two destinations a run's outputs go to, resolved on the command line: the output
/// folder, and the local save the `aesthetics_patch.toml` is applied to when the game loads the
/// run's output. Also how the savefile step treats the compile target's game running.
pub(crate) struct RunPaths {
    pub(crate) output_folder: PathBuf,
    pub(crate) savefile_path: SavefilePath,
    pub(crate) game: RunningGame,
}

/// The CPKs a normal compile writes, decided on the command line from the settings.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CpkLayout {
    /// One CPK, `name`, warned past `cap` (`cpk_size_over_limit`).
    Single { name: CpkStem, cap: u64 },
    /// Multi-CPK mode: team content into the official list's `teams_stem` slots, `cap`
    /// each, the overrides and the bins into `bins`.
    Parts {
        bins: CpkStem,
        teams_stem: String,
        cap: u64,
    },
}

impl CpkLayout {
    /// The CPK the working-bin walk and the installed texture lookup start below: the run's
    /// first CPK in the list's order, which is the bins CPK in multi-CPK mode. Every part the
    /// run writes replaces the one installed under its name, so the lookup starts below them.
    pub(crate) fn boundary(&self) -> &CpkStem {
        match self {
            CpkLayout::Single { name, .. } => name,
            CpkLayout::Parts { bins, .. } => bins,
        }
    }

    /// The CPKs the run writes, in the order they are installed and promoted: the one CPK, or
    /// the bins CPK, then the `official` list's teams slots by number (`parts::slots`); then
    /// the refs CPK `refs`, when the run holds a refs export.
    pub(crate) fn cpks(&self, official: &[String], refs: Option<&CpkStem>) -> Vec<CpkStem> {
        let mut cpks = match self {
            CpkLayout::Single { name, .. } => vec![name.clone()],
            CpkLayout::Parts {
                bins, teams_stem, ..
            } => std::iter::once(bins.clone())
                .chain(parts::slots(official, teams_stem))
                .collect(),
        };
        cpks.extend(refs.cloned());
        cpks
    }

    /// Where a run that does not install its CPKs leaves them, which its findings name: the
    /// one CPK's path in `output_folder`, or the folder itself, which takes every CPK of a
    /// multi-CPK run, or of a run with a refs CPK `refs`.
    fn promoted(&self, output_folder: &Path, refs: Option<&CpkStem>) -> PathBuf {
        match (self, refs) {
            (CpkLayout::Single { name, .. }, None) => {
                output_folder.join(deploy::cpk_file_name(name))
            }
            (CpkLayout::Single { .. }, Some(_)) | (CpkLayout::Parts { .. }, _) => {
                output_folder.to_owned()
            }
        }
    }
}

/// Compiles every export validation keeps into the CPKs of `layout` (`<name>.cpk`, or the
/// bins CPK and the teams parts), and a refs export into the refs CPK `refs_name` when there
/// is one (a normal `mode`'s alone), installed into the PES folder's `download/` when `mode`
/// deploys and the checks made before any export is read pass, else into `<output_folder>`
/// (in sideload `mode`, into the PES folder's `livecpk/` as loose files; in test `mode`, into
/// `<output_folder>/test_output/`), after the files of the data directory's `overrides/`
/// folder unless in test mode, reported as events, then collects the compiled exports' notes
/// into `<output_folder>/teamnotes.txt`. Its resources are the embedded ones or the data
/// directory's `templates/` files replacing them (`templates`), and its bins are built on those
/// of the installed CPKs listed before its first but the refs CPK (`bins::installed`). Returns
/// the worst severity reported: a `templates/` file or an installed bin that cannot be read, an
/// output that cannot be written or put in place, a team the parts cannot take, or an export
/// file that changes while the run reads it, is a Fatal finding, after which the previous
/// output is all that is left. An exports folder or an `overrides/` folder that cannot be read
/// is an error. A run of several CPKs installs all of them or none. The
/// `aesthetics_patch.toml` written beside the output is applied to the save `savefile_path`
/// resolves to when the game loads the run's output (`spec/team_compiler/pipeline.md` "6.
/// Post-processing").
pub(crate) fn run(
    inputs: &RunInputs,
    layout: &CpkLayout,
    refs_name: Option<&CpkStem>,
    paths: &RunPaths,
    mode: &OutputMode,
    ctx: &ToolContext,
) -> anyhow::Result<Option<Severity>> {
    let mut events = RunEvents::new(ctx);
    let Some(templates) = templates::read_reported(ctx, &mut events) else {
        return Ok(events.worst());
    };
    // Listed before the preflight: whether the run holds a refs export, and so writes the refs
    // CPK, is known from the exports' names before any file is read.
    let discovery = reader::discover(&inputs.exports_root, &inputs.exports)?;
    for path in &discovery.skipped {
        events.message(tool_message(
            Code::ExportUpgradeUnfinished,
            Scope::Run,
            Disposition::Keep,
            vec![("path", path.display().to_string())],
        ));
    }
    let sources = discovery.sources;
    let refs = refs_name.filter(|_| sources.iter().any(ExportSource::is_referees));
    let download = if mode.deploys() {
        let official = templates.official_list();
        let (download, messages) = deploy::preflight(
            &inputs.common.pes_folder(),
            inputs.common.pes_version,
            &layout.cpks(&official, refs),
            &layout.promoted(&paths.output_folder, refs),
            &official,
        );
        for message in messages {
            events.message(message);
        }
        download
    } else {
        None
    };
    let Some((bins, installed)) = working_bins(
        inputs,
        layout,
        refs,
        mode.deploys(),
        &templates,
        &mut events,
    ) else {
        return Ok(events.worst());
    };
    let planned = plan(inputs, sources, installed, mode, download, events, ctx)?;
    build(planned, bins, templates, layout, refs, paths, mode)
}

/// The bins the run builds on, from the installed CPKs of the PES folder listed before
/// `layout`'s boundary but the refs CPK `refs` the run replaces, or the bundled bases in
/// `templates`, and the entry paths of the installed CPKs walked, for the texture lookup,
/// their findings reported first. In multi-CPK mode the boundary is the bins CPK, whose
/// installed copy the shared-IDs records' walk starts at. A file of the walk that cannot be
/// read is `installed_bin_unreadable`, Fatal, and `None`: the run stops before any export is read,
/// the previous CPK kept, rather than build on an older copy that its CPK, loaded above it,
/// would put back for every team.
fn working_bins(
    inputs: &RunInputs,
    layout: &CpkLayout,
    refs: Option<&CpkStem>,
    deploys: bool,
    templates: &Templates,
    events: &mut RunEvents,
) -> Option<(WorkingBins, InstalledPaths)> {
    let pes_folder = inputs.common.pes_folder();
    let version = inputs.common.pes_version;
    let multicpk = matches!(layout, CpkLayout::Parts { .. });
    match installed::working_bins(
        &pes_folder,
        layout.boundary(),
        refs,
        multicpk,
        version,
        deploys,
        templates,
    ) {
        Ok((bins, installed, messages)) => {
            for message in messages {
                events.message(message);
            }
            Some((bins, installed))
        }
        Err(unreadable) => {
            events.message(tool_message(
                Code::InstalledBinUnreadable,
                Scope::Run,
                Disposition::AbortRun,
                vec![
                    ("path", unreadable.path.display().to_string()),
                    ("error", format!("{:#}", unreadable.error)),
                ],
            ));
            None
        }
    }
}

/// One export source as validation hands it to the coordinator.
pub(crate) struct CompileSource {
    /// The export's folder or archive as discovered (`reader::discover`).
    source: ExportSource,
    /// What the source looked like when it was listed, which its tasks' reads are checked
    /// against; `None` when routing or the listing's parse set it aside.
    revision: Option<SourceRevision>,
    /// The `.7z` kept decompressed from its check for its tasks
    /// (`validation::keeps_archive`), holding its permit; `None` when the coordinator opens
    /// the source itself.
    content: Option<ContentSource>,
}

/// A run validated and planned, which `build` compiles: its own function so a test can change
/// an export's files between planning and building.
struct PlannedRun {
    version: PesVersion,
    /// The events so far: validation's and planning's findings.
    events: RunEvents,
    /// The `download/` the run's CPKs are installed into, and whether the game will load
    /// them; `None` when the run does not deploy or cannot.
    download: Option<Download>,
    budget: Arc<MemoryBudget>,
    pool: rayon::ThreadPool,
    /// The `overrides/` folder's files, by CPK path.
    overrides: BTreeMap<String, PathBuf>,
    /// Every source in export order, as the coordinator reads its tasks from it.
    sources: Vec<CompileSource>,
    /// The entry paths of the installed CPKs walked, which validation looked in and the
    /// tasks look in.
    installed: InstalledPaths,
    manifest: BuildManifest,
    /// Whether the tasks WESYS-zlib every DDS they emit (`CompileContext::compress_dds`).
    compress_dds: bool,
}

/// `compile`'s first half: the `overrides/` folder listed when `mode` applies it, the
/// validation pass over the discovered `sources`, in which a texture link may name a texture
/// one of the `installed` CPKs holds, each export's findings reported through `events`, and
/// the run planned, to be installed into `download` when there is one.
fn plan(
    inputs: &RunInputs,
    sources: Vec<ExportSource>,
    installed: InstalledPaths,
    mode: &OutputMode,
    download: Option<Download>,
    mut events: RunEvents,
    ctx: &ToolContext,
) -> anyhow::Result<PlannedRun> {
    let version = inputs.common.pes_version;
    // Listed before any export is read, so a tree that cannot be listed stops the run first.
    let (overrides, overrides_active) = if mode.applies_overrides() {
        overrides::list(ctx.paths().data_dir.as_deref())?
    } else {
        (BTreeMap::new(), None)
    };
    let budget = run_budget(inputs);
    let pool = run_pool(inputs)?;
    let pass = validation_pass(inputs, sources, &installed, &budget, &pool, true)?;
    for message in pass.run_messages {
        events.message(message);
    }
    let mut sources = Vec::new();
    let mut exports = Vec::new();
    for checked in pass.sources {
        events.started(&checked.source);
        for message in checked.messages {
            events.message(message);
        }
        if let Some(resolved) = checked.resolved {
            let shared_ids = match &resolved.identity {
                ExportIdentity::Team { id, .. } => installed.shared_ids(id.get()).cloned(),
                ExportIdentity::Referees => None,
            };
            exports.push(ExportToPlan {
                export_id: checked.source.export_id,
                export: resolved,
                team_colors: checked.team_colors,
                notes: checked.notes,
                shared_ids,
                hand_weighted: checked.hand_weighted,
                metal_models: checked.metal_models,
                settings: checked.settings,
                settings_refused: checked.settings_refused,
                legacy_ids: checked.legacy_ids,
            });
        }
        sources.push(CompileSource {
            source: checked.source,
            revision: checked.revision,
            content: checked.content,
        });
    }

    let report = plan_run(exports, version, inputs.hide_ids);
    for message in report.messages.into_iter().chain(overrides_active) {
        events.message(message);
    }
    Ok(PlannedRun {
        version,
        events,
        download,
        budget,
        pool,
        overrides,
        sources,
        installed,
        manifest: report.manifest,
        compress_dds: inputs.settings.compresses_dds(version),
    })
}

/// `compile`'s second half: the planned tasks read and processed with the run's `templates`
/// and the installed CPKs' entry paths, and written into the staged CPKs of `layout` and the
/// refs CPK `refs`, the refs export's tasks into the latter (or loose tree, in test and
/// sideload `mode`), followed by the referee template tree when one of them committed (not in
/// test `mode`), the bins built on `bins`; those written are then promoted, and
/// `teamnotes.txt` written. A source that changed while its tasks were read aborts the run
/// with `source_changed_during_run`, and a team the parts cannot take with its finding, the
/// staging discarded.
fn build(
    planned: PlannedRun,
    bins: WorkingBins,
    templates: Templates,
    layout: &CpkLayout,
    refs: Option<&CpkStem>,
    paths: &RunPaths,
    mode: &OutputMode,
) -> anyhow::Result<Option<Severity>> {
    let output_folder = &paths.output_folder;
    let savefile_path = &paths.savefile_path;
    let PlannedRun {
        version,
        mut events,
        download,
        budget,
        pool,
        overrides,
        sources,
        installed,
        manifest,
        compress_dds,
    } = planned;
    let tasks = manifest.tasks;
    let task_count = tasks.len();
    let team_colors = manifest.team_colors;
    let team_kits = manifest.team_kits;
    let patch_teams = manifest.patch_teams;
    let notes = manifest.notes;
    let shared_ids = manifest.shared_ids;
    let mut last_task_of: BTreeMap<ExportId, usize> = BTreeMap::new();
    for (index, task) in tasks.iter().enumerate() {
        last_task_of.insert(task.export_id, index);
    }
    for CompileSource { source, .. } in &sources {
        if !last_task_of.contains_key(&source.export_id) {
            events.processed(source.export_id);
        }
    }
    let last_tasks: BTreeMap<usize, ExportId> = last_task_of
        .into_iter()
        .map(|(export_id, index)| (index, export_id))
        .collect();

    // `target` is the path the output is promoted to, which a failure names: the previous
    // output there is what a failed run leaves (in multi-CPK mode, the CPKs of the output
    // folder).
    let target = match mode {
        OutputMode::Normal { .. } => layout.promoted(output_folder, refs),
        OutputMode::Test => output_folder.join(deploy::TEST_OUTPUT),
        OutputMode::Sideload { pes_folder } => pes_folder.join(deploy::LIVECPK),
    };
    // Dropped on every way out of this function, which removes what is left in it: a failed
    // run's staged output included.
    let staging = match Staging::create(output_folder) {
        Ok(staging) => staging,
        Err(error) => {
            abort_output(&mut events, &target, Code::CpkWriteFailed, error);
            return Ok(events.worst());
        }
    };
    let source_names: BTreeMap<ExportId, String> = sources
        .iter()
        .map(|CompileSource { source, .. }| (source.export_id, source.file_name.clone()))
        .collect();
    let referee_tasks = referee_tasks(&tasks);
    // The refs export is no team to place in the parts: its entries go to the refs CPK.
    let ends = last_tasks
        .iter()
        .filter(|(index, _)| !referee_tasks.contains(index))
        .map(|(index, export_id)| {
            let name = source_names
                .get(export_id)
                .expect("every task's export is among the run's sources");
            (*index, name.clone())
        })
        .collect();
    let (sink, staged, parts) =
        staged_output(mode, layout, refs, staging.folder(), &templates, ends);
    let (entry_target, bins_prefix) = match mode {
        OutputMode::Normal { .. } | OutputMode::Sideload { .. } => (
            EntryTarget::GamePaths {
                engine: version.engine(),
            },
            "",
        ),
        OutputMode::Test => (
            EntryTarget::TestOutput {
                sources: source_names,
            },
            TEST_BINS_PREFIX,
        ),
    };
    let mut output = CpkOutput::new(sink, overrides, bins_prefix, parts);
    // Test mode writes no referee template tree: it is no export's, as the overrides are not.
    let referees = match (mode, refs) {
        (OutputMode::Normal { .. }, Some(refs)) => {
            let path = staging.folder().join(deploy::cpk_file_name(refs));
            Some(Referees::cpk(path, referee_tasks, version.engine()))
        }
        (OutputMode::Sideload { .. }, _) => {
            Some(Referees::in_sink(referee_tasks, version.engine()))
        }
        (OutputMode::Normal { .. }, None) | (OutputMode::Test, _) => None,
    };
    if let Some(referees) = referees {
        output = output.with_referees(referees);
    }
    let context = CompileContext::new(
        version,
        last_tasks.len(),
        templates,
        installed,
        entry_target,
        Arc::clone(&budget),
        compress_dds,
    );
    let (coordinated, (mut events, written)) = std::thread::scope(|scope| {
        let (batches_tx, batches_rx) = unbounded();
        let last_tasks = &last_tasks;
        let team_colors = &team_colors;
        let team_kits = &team_kits;
        let shared_ids = &shared_ids;
        let patch_teams = &patch_teams;
        let budget = &budget;
        let templates = &context.templates;
        let writer = scope.spawn(move || {
            let mut output = output;
            let mut events = events;
            // The writer finishes the CPK too, so its file is closed when the thread ends,
            // before a failure removes the staging folder.
            let written = write_batches(
                batches_rx,
                &mut output,
                &mut events,
                last_tasks,
                task_count,
                budget,
            )
            .and_then(|()| {
                output.finish(
                    version,
                    bins,
                    BinsEdits {
                        team_colors,
                        team_kits,
                        shared_ids,
                        patch_teams,
                    },
                    templates,
                )
            });
            (events, written)
        });
        let coordinated = pool.in_place_scope(|pool_scope| {
            coordinate(sources, tasks, &context, &batches_tx, pool_scope)
        });
        drop(batches_tx);
        let written = writer.join().expect("the writer thread does not panic");
        (coordinated, written)
    });
    if let Some(change) = coordinated {
        // The staged CPK holds only the batches spawned before the change, and its writer may
        // have failed on the ones that never came: the change is reported alone, never also
        // as a write failure, and the staging goes either way.
        if let Err(error) = written {
            log::debug!("the aborted run's staged CPK: {error:#}");
        }
        events.message(tool_message(
            Code::SourceChangedDuringRun,
            Scope::Export {
                export_id: change.export_id,
            },
            Disposition::AbortRun,
            vec![("path", change.path)],
        ));
        return Ok(end_run(&events, 0));
    }

    // A bins failure is a CPK write failure too, with no finding of its own.
    // A working bin that does not parse never gets here, stopped where it was read.
    let (written, files_written, committed) = match written {
        Ok((written, entries, messages, committed)) => {
            for message in messages {
                events.message(message);
            }
            (written, entries, committed)
        }
        Err(error) => {
            // A team the parts cannot take is its own finding, not a write failure; no part
            // is written either way.
            match error.downcast::<Unplaced>() {
                Ok(Unplaced(message)) => events.message(message),
                Err(error) => abort_output(&mut events, &target, Code::CpkWriteFailed, error),
            }
            return Ok(end_run(&events, 0));
        }
    };
    if !written.team && !written.refs {
        return Ok(end_run(&events, files_written));
    }
    if written.team
        && let (OutputMode::Normal { .. }, CpkLayout::Single { name, cap }) = (mode, layout)
    {
        match size_over_limit(staging.folder(), name, *cap) {
            Ok(Some(message)) => events.message(message),
            Ok(None) => {}
            Err(error) => {
                abort_output(&mut events, &target, Code::CpkWriteFailed, error);
                return Ok(end_run(&events, 0));
            }
        }
    }
    // Only what was written is put in place: a side nothing went into leaves the installed or
    // promoted CPKs of its names as they were. The refs CPK is named apart from the team
    // side's, which `compile_settings` refuses it to share.
    let staged: Vec<CpkStem> = staged
        .into_iter()
        .filter(|cpk| match refs {
            Some(refs) if refs == cpk => written.refs,
            Some(_) | None => written.team,
        })
        .collect();
    match promote(
        mode,
        &staging,
        output_folder,
        &staged,
        download.as_ref(),
        &target,
        &mut events,
    ) {
        Ok(published) => {
            // The patch's provisional boots and gloves IDs settled against the
            // committed batches: an ID whose row's task did not commit is removed,
            // an entry left with nothing dropped, then a team left with no player.
            let patch_teams = plan_patch::settle(patch_teams, &committed);
            match write_patch(&mut events, output_folder, version, &patch_teams, &staged) {
                Ok(patch) => {
                    // The savefile step applies the patch the run just wrote, never a
                    // re-read of the file, and only when the game loads the run's output: a
                    // deployment the game runs vanilla over commits the same, silently.
                    let failure = match (&published, &patch) {
                        (
                            Published::Deployed { loaded: true, .. } | Published::Sideloaded,
                            Some(patch),
                        ) => savefile::update(
                            &mut events,
                            savefile::locate(savefile_path, || {
                                pes_savefile::discovery::discover_savefiles(version)
                            }),
                            &(paths.game.probe)(),
                            paths.game.edit_mode_closed,
                            &output_folder.join(patch::FILE_NAME),
                            patch,
                        )
                        .err(),
                        (Published::Output | Published::Deployed { loaded: false, .. }, _)
                        | (_, None) => None,
                    };
                    match (published, failure) {
                        (Published::Deployed { installed, .. }, None) => {
                            installed.commit();
                            write_teamnotes(&mut events, output_folder, &notes);
                        }
                        (Published::Deployed { installed, .. }, Some(failure)) => {
                            installed.roll_back();
                            // Reported before the promotion, so a promotion that fails adds
                            // its own finding rather than hiding why the run rolled back.
                            let mut context = failure.context;
                            context.push(("output", output_folder.display().to_string()));
                            events.message(tool_message(
                                failure.code,
                                Scope::Run,
                                Disposition::AbortRun,
                                context,
                            ));
                            // The compiled artifacts remain available: the staged CPKs
                            // promoted to the output folder beside the patch naming them.
                            for cpk_stem in &staged {
                                if let Err(error) =
                                    deploy::promote(&staging, output_folder, cpk_stem)
                                {
                                    abort_output(
                                        &mut events,
                                        &target,
                                        Code::OutputCommitFailed,
                                        error,
                                    );
                                    return Ok(end_run(&events, 0));
                                }
                            }
                            // An AbortRun publishes nothing more: no teamnotes.
                        }
                        (Published::Sideloaded, failure) => {
                            if let Some(failure) = failure {
                                events.message(tool_message(
                                    failure.code,
                                    Scope::Run,
                                    Disposition::Keep,
                                    failure.context,
                                ));
                            }
                            write_teamnotes(&mut events, output_folder, &notes);
                        }
                        // `update` does not run for `Output`, which is `Ok` or no patch.
                        (Published::Output, _) => {
                            write_teamnotes(&mut events, output_folder, &notes);
                        }
                    }
                }
                Err(error) => {
                    let rolled_back = if let Published::Deployed { installed, .. } = published {
                        installed.roll_back();
                        true
                    } else {
                        false
                    };
                    abort_output(
                        &mut events,
                        &output_folder.join(patch::FILE_NAME),
                        Code::PatchWriteFailed,
                        error,
                    );
                    if rolled_back {
                        // Nothing was promoted to the output folder either: the run put
                        // nothing in place.
                        return Ok(end_run(&events, 0));
                    }
                }
            }
        }
        Err(error) => {
            abort_output(&mut events, &target, Code::OutputCommitFailed, error);
            return Ok(end_run(&events, 0));
        }
    }
    Ok(end_run(&events, files_written))
}

/// The run's `Complete` (sent once, here at each way out of `build`, where the compile ran),
/// then the worst severity `build` returns. `files_written` counts the entries of the CPKs
/// the run put in place: the output folder, or the game's `download/` when it deploys.
fn end_run(events: &RunEvents, files_written: usize) -> Option<Severity> {
    events.complete(files_written);
    events.worst()
}

/// Where `build` writes in the run's staging folder `run_folder` for `mode` and `layout`: the
/// sink; the CPKs staged there, the refs CPK `refs` among them, in the order they are promoted
/// (`CpkLayout::cpks`), none for a loose tree; and multi-CPK mode's teams parts, the slots
/// those of the official list in `templates`, each team ending at a manifest position of
/// `ends`.
fn staged_output(
    mode: &OutputMode,
    layout: &CpkLayout,
    refs: Option<&CpkStem>,
    run_folder: &Path,
    templates: &Templates,
    ends: BTreeMap<usize, String>,
) -> (OutputSink, Vec<CpkStem>, Option<TeamsParts>) {
    match mode {
        OutputMode::Normal { .. } => {
            let official = templates.official_list();
            let staged = layout.cpks(&official, refs);
            match layout {
                CpkLayout::Single { name, .. } => {
                    let sink = OutputSink::cpk(run_folder.join(deploy::cpk_file_name(name)));
                    (sink, staged, None)
                }
                CpkLayout::Parts {
                    bins,
                    teams_stem,
                    cap,
                } => {
                    let slots = parts::slots(&official, teams_stem);
                    let parts = TeamsParts::new(
                        run_folder.to_owned(),
                        slots,
                        teams_stem,
                        *cap,
                        templates.placeholder_cpk().to_vec(),
                        ends,
                    );
                    let sink = OutputSink::cpk(run_folder.join(deploy::cpk_file_name(bins)));
                    (sink, staged, Some(parts))
                }
            }
        }
        OutputMode::Test => (
            OutputSink::loose(run_folder.join(deploy::TEST_OUTPUT)),
            Vec::new(),
            None,
        ),
        OutputMode::Sideload { .. } => (
            OutputSink::loose(run_folder.join(deploy::LIVECPK)),
            Vec::new(),
            None,
        ),
    }
}

/// The manifest positions of the refs export's tasks, those of team `REFEREE_TEAM_ID`: the
/// manifest keeps an export's tasks together, and a run compiles one refs export at most
/// (`multiple_ref_exports`), so they are one range; empty when there are none.
fn referee_tasks(tasks: &[BuildTask]) -> Range<usize> {
    let is_referee = |task: &BuildTask| task.team_id == REFEREE_TEAM_ID;
    let Some(start) = tasks.iter().position(is_referee) else {
        return 0..0;
    };
    let count = tasks[start..]
        .iter()
        .take_while(|task| is_referee(task))
        .count();
    start..start + count
}

/// `cpk_size_over_limit` when the CPK `name` staged in `run_folder` is longer than `cap`
/// bytes: a single CPK is not split, only warned about.
/// The CPK was just written, so a size that cannot be read is the error.
fn size_over_limit(run_folder: &Path, name: &CpkStem, cap: u64) -> anyhow::Result<Option<Message>> {
    let file_name = deploy::cpk_file_name(name);
    let path = run_folder.join(&file_name);
    let size = fs::metadata(&path)
        .with_context(|| format!("cannot read the size of {}", path.display()))?
        .len();
    if size <= cap {
        return Ok(None);
    }
    Ok(Some(tool_message(
        Code::CpkSizeOverLimit,
        Scope::Run,
        Disposition::Keep,
        vec![
            ("cpk", file_name),
            ("size", size_text(size)),
            ("cap", size_text(cap)),
        ],
    )))
}

/// What `promote` published, and so what the rest of the run commits or undoes.
enum Published {
    /// The CPKs (or test mode's tree) in the output folder: the game loads nothing new.
    Output,
    /// The CPKs installed in `download/`, the old ones kept aside until the run commits;
    /// `loaded` `false` when the game will not load them (`dpfilelist_cpk_missing`): the run
    /// commits the same, and only the savefile step is skipped.
    Deployed { installed: Installed, loaded: bool },
    /// Sideload mode's tree in `livecpk/`: in place, with no undo.
    Sideloaded,
}

/// Puts the output staged in `staging` in place for `mode`: in normal mode the CPKs `staged`,
/// in that order. A run that deploys installs all of them into `download`'s folder when there
/// is one, else, or when installing fails (one `old_cpk_locked` or `deploy_target_unwritable`,
/// an Error naming `promoted`, the CPK's path in the output folder or the folder itself), every
/// one goes into the output folder; under `--no-deploy` each CPK goes there,
/// `deploy_skipped_by_flag` naming each. In test and sideload mode the loose tree becomes the
/// output folder's `test_output/` or the PES folder's `livecpk/`. Returns what was published:
/// a `Deployed` install's old CPKs stay aside for the run to commit or roll back, and the
/// savefile step runs for a `Deployed` the game will load and for `Sideloaded` alone, the
/// publications whose output the game loads (`spec/team_compiler/pipeline.md` "6.
/// Post-processing"). Only a failure to put the output in the output folder or the loose tree
/// in place is the error.
fn promote(
    mode: &OutputMode,
    staging: &Staging,
    output_folder: &Path,
    staged: &[CpkStem],
    download: Option<&Download>,
    promoted: &Path,
    events: &mut RunEvents,
) -> anyhow::Result<Published> {
    match mode {
        OutputMode::Normal { no_deploy } => {
            if let Some(download) = download {
                match deploy::deploy(staging, &download.folder, staged) {
                    Ok(installed) => {
                        return Ok(Published::Deployed {
                            installed,
                            loaded: download.loaded,
                        });
                    }
                    Err(failure) => {
                        events.message(deploy_failed(failure, &download.folder, promoted));
                    }
                }
            }
            for cpk_stem in staged {
                let path = deploy::promote(staging, output_folder, cpk_stem)?;
                if *no_deploy {
                    events.message(tool_message(
                        Code::DeploySkippedByFlag,
                        Scope::Run,
                        Disposition::Keep,
                        vec![("path", path.display().to_string())],
                    ));
                }
            }
            Ok(Published::Output)
        }
        OutputMode::Test => deploy::promote_tree(
            staging,
            deploy::TEST_OUTPUT,
            &output_folder.join(deploy::TEST_OUTPUT),
        )
        .map(|()| Published::Output),
        OutputMode::Sideload { pes_folder } => {
            deploy::promote_tree(staging, deploy::LIVECPK, &pes_folder.join(deploy::LIVECPK))
                .map(|()| Published::Sideloaded)
        }
    }
}

/// The Error for CPKs that could not be installed into `download`, by the step that failed:
/// a copy is `deploy_target_unwritable` naming the folder, a rename `old_cpk_locked` naming
/// the old CPK whose rename failed; each names the OS error and `output`, where the CPKs go
/// instead.
fn deploy_failed(failure: DeployFailure, download: &Path, output: &Path) -> Message {
    let (code, path, error) = match failure {
        DeployFailure::Copy(error) => (Code::DeployTargetUnwritable, download.to_owned(), error),
        DeployFailure::Rename { cpk, error } => (
            Code::OldCpkLocked,
            download.join(deploy::cpk_file_name(&cpk)),
            error,
        ),
    };
    tool_message(
        code,
        Scope::Run,
        Disposition::Keep,
        vec![
            ("path", path.display().to_string()),
            ("error", error.to_string()),
            ("output", output.display().to_string()),
        ],
    )
}

/// Writes the run's `aesthetics_patch.toml` into `output_folder` — `patch::write` moves an
/// earlier one aside to `.bak` when `teams` is empty (no player holds a key once settled) — reporting
/// `patch_written` with the path and the team ids it covers, and returns
/// the patch written, for the savefile step to apply. A write or rename failing is
/// `patch_write_failed` through `abort_output` at the caller, which first rolls a deployment
/// back and then aborts the run. Called once the run's outputs are
/// in place, so a run that published nothing (returned above) leaves the patch alone.
fn write_patch(
    events: &mut RunEvents,
    output_folder: &Path,
    version: PesVersion,
    teams: &[PatchTeam],
    staged: &[CpkStem],
) -> anyhow::Result<Option<AestheticsPatch>> {
    let patch = patch::write(output_folder, version, teams, staged)?;
    if let Some(patch) = &patch {
        events.message(tool_message(
            Code::PatchWritten,
            Scope::Run,
            Disposition::Keep,
            vec![
                (
                    "path",
                    output_folder.join(patch::FILE_NAME).display().to_string(),
                ),
                (
                    "teams",
                    patch
                        .teams
                        .iter()
                        .map(|team| team.id.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
            ],
        ));
    }
    Ok(patch)
}

/// Writes `notes` (team name, note text) as `<output_folder>/teamnotes.txt`, or removes a
/// previous one when there is no note: a file left over would show the notes of exports this
/// run did not compile. Called once the output is in place, so a run that writes none leaves
/// the previous file. A failure is `teamnotes_write_failed`, an Error on the run naming the file
/// and the whole error chain; the CPK stays.
fn write_teamnotes(events: &mut RunEvents, output_folder: &Path, notes: &[(String, String)]) {
    let path = output_folder.join(teamnotes::FILE_NAME);
    if let Err(error) = teamnotes::write(&path, teamnotes::render(notes).as_deref()) {
        events.message(tool_message(
            Code::TeamnotesWriteFailed,
            Scope::Run,
            Disposition::Keep,
            vec![
                ("path", path.display().to_string()),
                ("error", format!("{error:#}")),
            ],
        ));
    }
}

/// A failure writing the output or putting it in place, reported as `code`, Fatal on the run,
/// naming `target` (the CPK, or `livecpk/`) and the whole error chain: the run's staging goes
/// when it is dropped, so the previous output at `target` is all that is left.
fn abort_output(events: &mut RunEvents, target: &Path, code: Code, error: anyhow::Error) {
    events.message(tool_message(
        code,
        Scope::Run,
        Disposition::AbortRun,
        vec![
            ("path", target.display().to_string()),
            ("error", format!("{error:#}")),
        ],
    ));
}

/// A file of an export that changed while the run read it: `source_changed_during_run`.
#[derive(Debug, PartialEq, Eq)]
struct SourceChange {
    /// The export whose file changed.
    export_id: ExportId,
    /// The file, or the archive, as the system displays its path.
    path: String,
}

/// The coordinator: every task's files read from its export's source, in manifest order, and
/// the task handed to `pool` with its permit, acquired from the run's budget (`context`'s),
/// each finished batch sent to `batches` with its entries' bytes charged to that budget. Each
/// export's source is opened once for all its tasks, which the manifest keeps together, or
/// read through the `.7z` validation kept open for them. After
/// each read the files read are checked against the source's revision; on a change no further
/// task is read or spawned, the spawned ones finish, and the change is returned. A cancelled
/// budget stops the coordinator before the next task's read and at a waiting acquire, with
/// no change to report: the cancellation's only trigger is the writer's `cpk_write_failed`,
/// which the writer reports itself.
fn coordinate<'scope>(
    sources: Vec<CompileSource>,
    tasks: Vec<BuildTask>,
    context: &'scope CompileContext,
    batches: &'scope Sender<TaskBatch>,
    pool: &rayon::Scope<'scope>,
) -> Option<SourceChange> {
    let budget = &context.budget;
    let spawn = move |index: usize,
                      task: BuildTask,
                      files: Result<TaskFiles, SourceFailure>,
                      permit: Option<Arc<Permit>>| {
        pool.spawn(move |_| {
            let mut batch = task_batch(index, task, files, context);
            batch.permit = permit;
            // Charged, never acquired: a task waiting here would hold its source permit
            // while it waits (`MemoryBudget::charge`).
            batch.output = Some(context.budget.charge(batch.output_len()));
            batches
                .send(batch)
                .expect("the writer receives until every sender is gone");
        });
    };
    // The coordinator reads, not the task: an archive is one sequential stream, so tasks
    // sharing it would only wait on each other, and processing needs no source handle.
    let mut tasks = tasks.into_iter().enumerate().peekable();
    for CompileSource {
        source,
        revision,
        content,
    } in sources
    {
        // A kept `.7z` is already decompressed and charged: its tasks read the same buffer.
        let content = content.unwrap_or_else(|| ContentSource::new(&source, budget));
        if source.kind == SourceKind::SevenZ {
            // A `.7z` holds one permit for its whole decompressed buffer, and a task asking
            // for its own while that is held waits forever when the archive is over the cap.
            // So every task is read first, the buffer freed, and the tasks share its permit.
            let mut read = Vec::new();
            while let Some((index, task)) =
                tasks.next_if(|(_, task)| task.export_id == source.export_id)
            {
                // The cancelled check sits at each task, not each source: a `.7z` acquires
                // inside its first `read`, so a cancel landing while its permit is held
                // would otherwise read and spawn the rest of the archive.
                if budget.is_cancelled() {
                    return None;
                }
                let files = read_files(&task, &content);
                read.push((index, task, files));
            }
            // Once, after the one read that decompressed the archive for every task; the
            // archive's stamp is checked whatever the files.
            if !read.is_empty()
                && let Some(change) = source_change(&source, revision.as_ref(), [])
            {
                return Some(change);
            }
            let permit = content.into_permit().map(Arc::new);
            for (index, task, files) in read {
                spawn(index, task, files, permit.clone());
            }
        } else {
            // A player folder's group is charged once, at its first task, and its tasks share
            // the permit: the writer holds the group's packages until its textures batch, so
            // a textures task waiting for a permit of its own would wait for memory the held
            // packages never release.
            let mut group_permit: Option<Arc<Permit>> = None;
            while let Some((index, task)) =
                tasks.next_if(|(_, task)| task.export_id == source.export_id)
            {
                // The same check at each task: a group's later tasks share its permit, so
                // they never reach an `acquire` that would turn a cancel into `None`.
                if budget.is_cancelled() {
                    return None;
                }
                let permit = match &task.group {
                    Some(group) if index == group.tasks.start => {
                        // The previous group's tasks are all spawned (groups
                        // are contiguous), so the coordinator's share goes
                        // before the wait: only its batches still hold it.
                        drop(group_permit.take());
                        match budget.acquire(group.charge) {
                            Ok(held) => {
                                let permit = Arc::new(held);
                                group_permit = Some(permit.clone());
                                permit
                            }
                            Err(Cancelled) => return None,
                        }
                    }
                    Some(_) => group_permit
                        .clone()
                        .expect("a group's first task acquired the shared permit"),
                    None => {
                        // The same drop for an ungrouped task's acquire: the
                        // group before it, when there was one, is done.
                        drop(group_permit.take());
                        match budget.acquire(task.charge) {
                            Ok(held) => Arc::new(held),
                            Err(Cancelled) => return None,
                        }
                    }
                };
                let files = read_files(&task, &content);
                // After the read, not before it: a file saved over while it was being read is
                // caught too. A file gone before the read is a change, not a failed read.
                let read = task.kind.files();
                let read = read.iter().map(|file| file.source.as_str());
                if let Some(change) = source_change(&source, revision.as_ref(), read) {
                    return Some(change);
                }
                spawn(index, task, files, Some(permit));
            }
        }
    }
    None
}

/// The change `source`'s `revision` sees among `files`, the paths in the source a task read;
/// an archive's own stamp is checked whatever they are.
fn source_change<'a>(
    source: &ExportSource,
    revision: Option<&SourceRevision>,
    files: impl IntoIterator<Item = &'a str>,
) -> Option<SourceChange> {
    let revision = revision.expect("a source with a task was validated, so it was listed");
    let path = revision.changed(source, files)?;
    Some(SourceChange {
        export_id: source.export_id,
        path,
    })
}

/// The writer thread's work: each batch received is committed in manifest order, then every
/// committed batch's messages reported, its `Progress` (`done` of `tasks_total`, the
/// manifest's task count), and its export's `ExportProcessed` after the export's last task
/// (`last_tasks`, by manifest position). Returns the first error writing the CPK,
/// once every batch has been received. A failed `submit` cancels the budget, so the
/// coordinator stops reading and the pool stops processing the rest of the run: the failure
/// this function returns is the finding the run reports.
fn write_batches(
    batches: Receiver<TaskBatch>,
    output: &mut CpkOutput,
    events: &mut RunEvents,
    last_tasks: &BTreeMap<usize, ExportId>,
    tasks_total: usize,
    budget: &Arc<MemoryBudget>,
) -> anyhow::Result<()> {
    let mut done = 0;
    for batch in &batches {
        let committed = match output.submit(batch) {
            Ok(committed) => committed,
            Err(error) => {
                budget.cancel();
                // Keep receiving until the channel closes: a pool task's send must never
                // find it closed, and the coordinator may be waiting for a permit that only
                // dropping a later batch frees.
                for unwritten in &batches {
                    drop(unwritten);
                }
                return Err(error);
            }
        };
        for (index, messages) in committed {
            for message in messages {
                events.message(message);
            }
            if let Some(export_id) = last_tasks.get(&index) {
                events.processed(*export_id);
            }
            done += 1;
            events.progress(done, tasks_total);
        }
    }
    Ok(())
}

/// The batch of `task`, the manifest's task number `index`, processed over `files`, the
/// coordinator's read of it. A file that could not be read fails the task with
/// `source_read_failed` naming it, on the task's folder, dropping the task's unit as a failed
/// processing does (`dropped_unit`), and the task is not processed.
fn task_batch(
    index: usize,
    task: BuildTask,
    files: Result<TaskFiles, SourceFailure>,
    ctx: &CompileContext,
) -> TaskBatch {
    match files {
        Ok(files) => process_task(index, task, files, ctx),
        Err(failure) => TaskBatch {
            index,
            entries: Vec::new(),
            group: task.group.as_ref().map(|group| group.tasks.clone()),
            uniparam: None,
            uni_color: None,
            configless_kit: None,
            messages: vec![tool_message(
                Code::SourceReadFailed,
                Scope::Folder {
                    export_id: task.export_id,
                    path: task.kind.folder_path(),
                },
                dropped_unit(&task.kind),
                vec![("path", failure.path), ("error", failure.error)],
            )],
            permit: None,
            output: None,
        },
    }
}

/// The bytes of every file `task` reads, from `content`; the first file that cannot be read
/// stops the reading.
fn read_files(task: &BuildTask, content: &ContentSource) -> Result<TaskFiles, SourceFailure> {
    let mut files = TaskFiles::new();
    for file in task.kind.files() {
        let bytes = content.read(file.source.as_str())?;
        files.insert(file.path.clone(), bytes);
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs::{self, File};
    use std::path::Path;
    use std::thread;
    use std::time::Duration;

    use aesthetics_export::{
        ExportCoverage, FileDescriptor, KitFolder, KitTexture, KitTextureSource,
    };
    use kit_config::KitSlot;
    use pes_version::{Engine, PesVersion};
    use pipeline::MemoryBudget;
    use studio_core::{ExportId, Message, PipelineEvent};
    use teams_list::TeamsList;
    use vtree::ScopePath;

    use super::*;
    use crate::paths::{PackageKey, TextureHome};
    use crate::plan::roles::ModelPackage;
    use crate::plan::{EffectiveTeamKitFpc, ModelFolder, TaskGroup, TaskKind, TeamKitEdits};
    use crate::reader::{ExportSource, Route};
    use crate::settings::TeamCompilerSettings;
    use crate::testing::{sandbox, scratch, tool_context};

    const PLAYER: &str = "Players/05 - The Chad Stormworks Player";

    /// The tracer bullet's export folder, as export 4.
    fn tracer_source() -> ExportSource {
        ExportSource {
            export_id: ExportId(4),
            path: Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/tracer/studio/egg Midcup Tracer"),
            kind: SourceKind::Folder,
            file_name: "egg Midcup Tracer".to_owned(),
            display_name: "egg Midcup Tracer".to_owned(),
            team_name: None,
        }
    }

    /// The tracer's player folder holding the files `names`, sizes unknown.
    fn player(names: &[&str]) -> ModelFolder {
        ModelFolder {
            path: ScopePath::new(PLAYER).unwrap(),
            files: names
                .iter()
                .map(|name| {
                    let path = ScopePath::new(&format!("{PLAYER}/{name}")).unwrap();
                    FileDescriptor {
                        size: 0,
                        kind: aesthetics_export::classify(path.name()),
                        source: path.clone(),
                        path,
                    }
                })
                .collect(),
            ingame_face: false,
            engine: Engine::Fox,
            combined: Vec::new(),
            common_models: Vec::new(),
            common_texture_stems: BTreeSet::new(),
            common_files: Vec::new(),
            hand_split: BTreeSet::new(),
            environment_map: false,
            refkit_body: false,
            textures: TextureHome::PlayerCommon {
                folder_name: "05 - The Chad Stormworks Player".to_owned(),
            },
        }
    }

    /// `source` with the revision its listing gives now, as validation hands it on.
    fn listed(source: ExportSource) -> CompileSource {
        let route = crate::reader::route(std::slice::from_ref(&source)).remove(0);
        let Route::Validate { revision, .. } = route else {
            panic!("{route:?}");
        };
        CompileSource {
            source,
            revision: Some(revision),
            content: None,
        }
    }

    /// Three tasks of the tracer's player 05, as export 4: its face package and its textures
    /// (`shirt.dds`) as one group of charge 30, then a portrait (`shirt.dds` again) of charge 7.
    fn tracer_tasks() -> Vec<BuildTask> {
        let folder = player(&[
            "face_diff.bin",
            "fcl_hair.fmdl",
            "fcl_hair.skl",
            "fcl_hair_sim.fclo",
            "shirt.dds",
        ]);
        let group = TaskGroup {
            tasks: 0..2,
            charge: 30,
        };
        let task = |kind, charge, group| BuildTask {
            export_id: ExportId(4),
            team_id: 792,
            kind,
            charge,
            group,
        };
        vec![
            task(
                TaskKind::Models {
                    folder: folder.clone(),
                    package: ModelPackage::Face,
                    ids: vec![PackageKey::Id(79205)],
                    kits: Vec::new(),
                },
                10,
                Some(group.clone()),
            ),
            task(
                TaskKind::Textures {
                    folder: folder.clone(),
                    kits: Vec::new(),
                },
                20,
                Some(group),
            ),
            // The tracer's `shirt.dds` stands in for a portrait: a DDS that passes through.
            task(
                TaskKind::Portrait {
                    player_id: 79205,
                    file: folder
                        .files
                        .iter()
                        .find(|file| file.path.name() == "shirt.dds")
                        .unwrap()
                        .clone(),
                },
                7,
                None,
            ),
        ]
    }

    /// The context of a PES 21 run of one export over the bundled templates, its tasks charging
    /// `budget`.
    fn context_with(budget: &Arc<MemoryBudget>) -> CompileContext {
        CompileContext::new(
            PesVersion::Pes21,
            1,
            Templates::embedded(),
            InstalledPaths::Unknown,
            EntryTarget::GamePaths {
                engine: Engine::Fox,
            },
            Arc::clone(budget),
            false,
        )
    }

    /// `coordinate` over the tracer's source and `tasks` on a spawned thread with a budget of
    /// `cap` bytes, the batches dropped as they arrive (the writer's role, so permits free
    /// mid-run): the change it returned and the budget's peak, or `None` when it did not come
    /// back within the guard.
    fn coordinated_with_cap(
        tasks: Vec<BuildTask>,
        cap: usize,
    ) -> Option<(Option<SourceChange>, usize)> {
        let (done_tx, done) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let budget = MemoryBudget::new(cap);
            let sources = vec![listed(tracer_source())];
            let context = context_with(&budget);
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(2)
                .build()
                .unwrap();
            let (batches_tx, batches_rx) = unbounded();
            thread::spawn(move || {
                for batch in &batches_rx {
                    drop(batch);
                }
            });
            let change = pool
                .in_place_scope(|scope| coordinate(sources, tasks, &context, &batches_tx, scope));
            drop(batches_tx);
            done_tx.send((change, budget.peak())).unwrap();
        });
        done.recv_timeout(Duration::from_secs(30)).ok()
    }

    /// The coordinator over `sources` and `tasks` on a pool of two threads, charging `budget`:
    /// what it returned, and every batch it sent, in manifest order.
    fn coordinated_on(
        budget: &Arc<MemoryBudget>,
        sources: Vec<CompileSource>,
        tasks: Vec<BuildTask>,
    ) -> (Option<SourceChange>, Vec<TaskBatch>) {
        let context = context_with(budget);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let (batches_tx, batches_rx) = unbounded();

        let change =
            pool.in_place_scope(|scope| coordinate(sources, tasks, &context, &batches_tx, scope));
        drop(batches_tx);

        let mut batches: Vec<TaskBatch> = batches_rx.iter().collect();
        batches.sort_by_key(|batch| batch.index);
        (change, batches)
    }

    /// `coordinated_on` a budget no test fills.
    fn coordinated(
        sources: Vec<CompileSource>,
        tasks: Vec<BuildTask>,
    ) -> (Option<SourceChange>, Vec<TaskBatch>) {
        coordinated_on(&MemoryBudget::new(1 << 30), sources, tasks)
    }

    #[test]
    fn a_group_s_tasks_share_one_permit_of_the_group_s_charge_and_other_tasks_have_their_own() {
        let (change, batches) = coordinated(vec![listed(tracer_source())], tracer_tasks());

        assert_eq!(change, None);
        let permits: Vec<&Arc<Permit>> = batches
            .iter()
            .map(|batch| batch.permit.as_ref().expect("every task is charged"))
            .collect();
        assert_eq!(permits.len(), 3);
        assert!(
            Arc::ptr_eq(permits[0], permits[1]),
            "the group's two tasks hold the one permit"
        );
        assert_eq!(permits[0].size(), 30, "the group's whole charge, once");
        assert!(!Arc::ptr_eq(permits[0], permits[2]));
        assert_eq!(permits[2].size(), 7, "the ungrouped task's own charge");
    }

    /// The tracer's tasks reworked: `groups` as (tasks range, charge) pairs, `ungrouped`
    /// charges as single tasks after them, every task's files the tracer's five.
    fn grouped_tasks(groups: &[(usize, usize, usize)], ungrouped: &[usize]) -> Vec<BuildTask> {
        let folder = player(&[
            "face_diff.bin",
            "fcl_hair.fmdl",
            "fcl_hair.skl",
            "fcl_hair_sim.fclo",
            "shirt.dds",
        ]);
        let kind = || TaskKind::Models {
            folder: folder.clone(),
            package: ModelPackage::Face,
            ids: vec![PackageKey::Id(79205)],
            kits: Vec::new(),
        };
        let task = |group| BuildTask {
            export_id: ExportId(4),
            team_id: 792,
            kind: kind(),
            charge: 0,
            group,
        };
        let mut tasks = Vec::new();
        for (start, end, charge) in groups {
            let group = TaskGroup {
                tasks: *start..*end,
                charge: *charge,
            };
            for _ in *start..*end {
                tasks.push(task(Some(group.clone())));
            }
        }
        for charge in ungrouped {
            tasks.push(BuildTask {
                charge: *charge,
                ..task(None)
            });
        }
        tasks
    }

    #[test]
    fn an_oversized_group_lets_the_next_acquire_once_its_batches_are_free() {
        // A group over the cap admits alone; the ungrouped task after it
        // acquires once the group's batches are gone — the coordinator's own
        // share must not hold the group charged.
        let tasks = grouped_tasks(&[(0, 2, 50)], &[5]);
        assert!(
            coordinated_with_cap(tasks, 30).is_some(),
            "the coordinator never returned"
        );
    }

    #[test]
    fn two_groups_over_the_cap_together_do_not_hang_the_coordinator() {
        // Each group fits alone; together they exceed the cap, so the second
        // acquires only after the first's batches are dropped — again only if
        // the coordinator released its own share first.
        let tasks = grouped_tasks(&[(0, 2, 30), (2, 4, 30)], &[]);
        assert!(
            coordinated_with_cap(tasks, 40).is_some(),
            "the coordinator never returned"
        );
    }

    /// The tracer's file at `relative`, size unknown.
    fn tracer_file(relative: &str) -> FileDescriptor {
        let path = ScopePath::new(relative).unwrap();
        FileDescriptor {
            size: 0,
            kind: aesthetics_export::classify(path.name()),
            source: path.clone(),
            path,
        }
    }

    /// The task of the tracer's `g1` kit, its config and its 16x16 `kit.dds`, as export 4's,
    /// charged `charge`.
    fn tracer_kit_task(charge: usize) -> BuildTask {
        BuildTask {
            export_id: ExportId(4),
            team_id: 792,
            kind: TaskKind::Kit {
                slot: KitSlot::G1,
                kit: KitFolder {
                    path: ScopePath::new("Kits/g1").unwrap(),
                    label: None,
                    config: Some(tracer_file("Kits/g1/config.toml")),
                    colors: None,
                    icon: Some(11),
                    layout: None,
                    textures: vec![KitTexture {
                        stem: "kit".to_owned(),
                        file: tracer_file("Kits/g1/kit.dds"),
                        source: KitTextureSource::Own,
                    }],
                },
                edits: TeamKitEdits {
                    fpc: EffectiveTeamKitFpc::Unknown,
                    collar: None,
                },
                coverage: ExportCoverage::Full,
            },
            charge,
            group: None,
        }
    }

    #[test]
    fn a_running_task_charges_its_decode_on_top_of_its_source_permit() {
        // The tracer's `shirt.dds`: 128x128 BC3 with its eight levels, the only texture of a
        // textures task of source charge 7. Its FTEX, about the size of the DDS, is charged
        // after the decode is gone, so the decode is the peak.
        let task = BuildTask {
            export_id: ExportId(4),
            team_id: 792,
            kind: TaskKind::Textures {
                folder: player(&["shirt.dds"]),
                kits: Vec::new(),
            },
            charge: 7,
            group: None,
        };
        let budget = MemoryBudget::new(1 << 30);

        let (change, batches) = coordinated_on(&budget, vec![listed(tracer_source())], vec![task]);

        assert_eq!(change, None);
        assert_eq!(batches.len(), 1);
        assert!(batches[0].messages.is_empty(), "{:?}", batches[0].messages);
        let pixels = 128 * 128 + 64 * 64 + 32 * 32 + 16 * 16 + 8 * 8 + 4 * 4 + 2 * 2 + 1;
        let file_len = 22_000;
        assert_eq!(budget.peak(), 7 + 4 * pixels + file_len);
    }

    #[test]
    fn a_batch_carries_its_entries_and_kit_config_s_bytes_as_its_output_charge() {
        let (change, batches) =
            coordinated(vec![listed(tracer_source())], vec![tracer_kit_task(5)]);

        assert_eq!(change, None);
        let batch = &batches[0];
        let (_, config) = batch.uniparam.as_ref().expect("the kit's config");
        let entries: usize = batch.entries.iter().map(|(_, bytes)| bytes.len()).sum();
        assert_eq!(batch.entries.len(), 2, "the kit's texture and its config");
        assert_eq!(
            batch.output.as_ref().map(Permit::size),
            Some(entries + config.len())
        );
    }

    #[test]
    fn a_run_whose_decodes_pass_the_cap_charges_them_and_still_completes() {
        // A cap of 1 KiB is under every decode of the run: the kit's 16x16 texture alone is
        // 4 * 341 bytes decoded, the face folder's `shirt.dds` 4 * 21845. A charge that waited
        // for room would wait on its own task's permit.
        let mut tasks = tracer_tasks();
        tasks.push(tracer_kit_task(5));

        let (change, peak) =
            coordinated_with_cap(tasks, 1024).expect("the coordinator never returned");

        assert_eq!(change, None);
        assert!(peak > 1024, "{peak}");
    }

    #[test]
    fn a_cancelled_budget_stops_the_coordinator_before_any_task_is_spawned() {
        let budget = MemoryBudget::new(1 << 30);
        budget.cancel();
        let context = context_with(&budget);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let (batches_tx, batches_rx) = unbounded::<TaskBatch>();
        let sources = vec![listed(tracer_source())];

        let change = pool.in_place_scope(|scope| {
            coordinate(
                sources,
                grouped_tasks(&[], &[10, 20, 30]),
                &context,
                &batches_tx,
                scope,
            )
        });
        drop(batches_tx);

        assert_eq!(change, None);
        assert!(
            batches_rx.try_iter().next().is_none(),
            "a task was spawned on a cancelled budget"
        );
    }

    #[test]
    fn a_cancelled_budget_stops_the_coordinator_before_a_7z_source() {
        // A `.7z` source never calls `acquire` itself: its first `read` does, inside
        // `ContentSource`, which reports a cancelled one as a failed read rather than
        // stopping the run. The cancelled check must come before the source is opened.
        let budget = MemoryBudget::new(1 << 30);
        budget.cancel();
        let context = context_with(&budget);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let (batches_tx, batches_rx) = unbounded::<TaskBatch>();
        let sources = vec![listed(tracer_7z_source())];

        let change = pool.in_place_scope(|scope| {
            coordinate(
                sources,
                grouped_tasks(&[], &[10, 20, 30]),
                &context,
                &batches_tx,
                scope,
            )
        });
        drop(batches_tx);

        assert_eq!(change, None);
        assert!(
            batches_rx.try_iter().next().is_none(),
            "a task was spawned on a cancelled budget"
        );
    }

    /// The tracer export as the solid `.7z` fixture, as export 4.
    fn tracer_7z_source() -> ExportSource {
        ExportSource {
            path: Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/sources/egg Midcup Tracer.7z"),
            kind: SourceKind::SevenZ,
            file_name: "egg Midcup Tracer.7z".to_owned(),
            ..tracer_source()
        }
    }

    #[test]
    fn a_7z_kept_from_its_check_is_read_by_its_tasks_without_a_second_decompression() {
        let budget = MemoryBudget::new(1 << 30);
        let mut kept = listed(tracer_7z_source());
        // As the check leaves it: decompressed by a read, holding its permit.
        let content = ContentSource::new(&kept.source, &budget);
        content.read(&format!("{PLAYER}/shirt.dds")).unwrap();
        let decompressed = content.held();
        assert!(decompressed > 0);
        kept.content = Some(content);
        // Kit tasks, whose decodes and outputs charge a few KB: the face package's tasks
        // charge more than the archive's size, which would hide a second decompression.
        let tasks = vec![tracer_kit_task(5), tracer_kit_task(5), tracer_kit_task(5)];

        let (change, batches) = coordinated_on(&budget, vec![kept], tasks);

        assert_eq!(change, None);
        let permits: Vec<&Arc<Permit>> = batches
            .iter()
            .map(|batch| {
                batch
                    .permit
                    .as_ref()
                    .expect("every task holds the archive's")
            })
            .collect();
        assert_eq!(permits.len(), 3);
        assert!(
            permits.iter().all(|permit| Arc::ptr_eq(permit, permits[0])),
            "the tasks share one permit"
        );
        assert_eq!(permits[0].size(), decompressed, "the kept source's permit");
        // Decompressed once: a second `ContentSource` would acquire `decompressed` again while
        // the kept permit is held.
        let peak = budget.peak();
        assert!(
            peak < 2 * decompressed,
            "peak {peak}, decompressed {decompressed}"
        );
    }

    #[test]
    fn a_failed_cpk_write_cancels_the_budget_so_no_task_waits_on_it() {
        let temp = scratch("write_batches_cancel");
        let root = temp.path();
        // A file where the CPK's folder goes: it cannot be created in it.
        fs::write(root.join("blocker"), "").unwrap();
        let mut output = CpkOutput::new(
            OutputSink::cpk(root.join("blocker").join("cup.cpk")),
            BTreeMap::new(),
            "",
            None,
        );
        let budget = MemoryBudget::new(1 << 30);
        let (batches_tx, batches_rx) = unbounded();
        batches_tx
            .send(TaskBatch {
                index: 0,
                entries: vec![("common/etc/TeamColor.bin".to_owned(), b"bin".to_vec())],
                group: None,
                uniparam: None,
                uni_color: None,
                configless_kit: None,
                messages: Vec::new(),
                permit: None,
                output: None,
            })
            .unwrap();
        drop(batches_tx);
        let mut events = RunEvents::new(&tool_context(root, ""));

        assert!(
            write_batches(
                batches_rx,
                &mut output,
                &mut events,
                &BTreeMap::new(),
                1,
                &budget
            )
            .is_err(),
            "the CPK could not be created"
        );
        assert!(
            matches!(budget.acquire(1), Err(Cancelled)),
            "the budget is cancelled"
        );
    }

    #[test]
    fn a_file_that_cannot_be_read_fails_its_task_naming_it() {
        let source = tracer_source();
        let tracer = source.path.clone();
        // The tracer's player folder holds no `face_high.fmdl`.
        let folder = player(&["face_high.fmdl"]);
        let task = BuildTask {
            export_id: ExportId(4),
            team_id: 792,
            kind: TaskKind::Models {
                folder,
                package: ModelPackage::Face,
                ids: vec![PackageKey::Id(79205)],
                kits: Vec::new(),
            },
            charge: 0,
            group: None,
        };

        let files = read_files(
            &task,
            &ContentSource::new(&source, &MemoryBudget::new(1 << 30)),
        );
        let batch = task_batch(3, task, files, &context_with(&MemoryBudget::new(1 << 30)));

        assert_eq!(batch.index, 3);
        assert!(batch.entries.is_empty() && batch.uniparam.is_none());
        let [message] = batch.messages.as_slice() else {
            panic!("{:?}", batch.messages);
        };
        assert_eq!(message.code.code, "source_read_failed");
        assert_eq!(
            (message.severity, message.disposition),
            (Severity::Error, Disposition::DropFolder)
        );
        assert_eq!(
            message.scope,
            Scope::Folder {
                export_id: ExportId(4),
                path: ScopePath::new(PLAYER).unwrap(),
            }
        );
        let path = tracer.join(format!("{PLAYER}/face_high.fmdl"));
        assert_eq!(
            message.context[0],
            ("path".to_owned(), path.display().to_string())
        );
        assert_eq!(message.context[1].0, "error");
    }

    #[test]
    fn a_portrait_that_cannot_be_read_drops_its_file_as_a_failed_processing_does() {
        let source = tracer_source();
        // The tracer holds no `Portraits/` folder.
        let task = BuildTask {
            export_id: ExportId(4),
            team_id: 792,
            kind: TaskKind::Portrait {
                player_id: 79205,
                file: tracer_file("Portraits/05.dds"),
            },
            charge: 0,
            group: None,
        };

        let files = read_files(
            &task,
            &ContentSource::new(&source, &MemoryBudget::new(1 << 30)),
        );
        let batch = task_batch(0, task, files, &context_with(&MemoryBudget::new(1 << 30)));

        let [message] = batch.messages.as_slice() else {
            panic!("{:?}", batch.messages);
        };
        assert_eq!(message.code.code, "source_read_failed");
        assert_eq!(
            message.disposition,
            Disposition::DropFile,
            "a portrait task's unit is its one file, whichever step fails"
        );
    }

    /// How far a test moves a modified time: far past any file system's time granularity.
    const MOVED: Duration = Duration::from_secs(10);

    fn move_modified_time(path: &Path) {
        let listed = fs::metadata(path).unwrap().modified().unwrap();
        File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(listed + MOVED)
            .unwrap();
    }

    #[test]
    fn a_file_rewritten_after_the_listing_stops_the_coordinator_at_the_task_that_reads_it() {
        let temp = sandbox("coordinate_folder_changed");
        let source = ExportSource {
            path: temp.path().join("exports").join("egg Midcup Tracer"),
            ..tracer_source()
        };
        let shirt = source.path.join(format!("{PLAYER}/shirt.dds"));
        let sources = vec![listed(source)];
        // Read by the second task, the group's textures, and by the third, the portrait.
        fs::write(&shirt, b"saved over").unwrap();

        let (change, batches) = coordinated(sources, tracer_tasks());

        let indices: Vec<usize> = batches.iter().map(|batch| batch.index).collect();
        assert_eq!(indices, [0], "only the first task's batch is sent");
        assert_eq!(
            change,
            Some(SourceChange {
                export_id: ExportId(4),
                path: shirt.display().to_string(),
            })
        );
    }

    #[test]
    fn an_archive_whose_modified_time_moved_after_the_listing_sends_no_batch() {
        for name in ["egg Midcup Tracer.zip", "egg Midcup Tracer.7z"] {
            let temp = scratch("coordinate_archive_changed");
            let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/sources")
                .join(name);
            let kind = if name.ends_with(".7z") {
                SourceKind::SevenZ
            } else {
                SourceKind::Zip
            };
            let source = ExportSource {
                path: temp.path().join(name),
                kind,
                file_name: name.to_owned(),
                ..tracer_source()
            };
            fs::copy(&fixture, &source.path).unwrap();
            let archive = source.path.display().to_string();
            let sources = vec![listed(source)];
            move_modified_time(&sources[0].source.path);

            let (change, batches) = coordinated(sources, tracer_tasks());

            assert!(batches.is_empty(), "{name}: {} batches", batches.len());
            assert_eq!(
                change,
                Some(SourceChange {
                    export_id: ExportId(4),
                    path: archive,
                }),
                "{name}"
            );
        }
    }

    /// The run inputs `compile` resolves for a `sandbox` with no arguments, over `ctx`'s
    /// settings.
    fn sandbox_inputs(root: &Path, ctx: &ToolContext) -> RunInputs {
        RunInputs {
            settings: TeamCompilerSettings::default(),
            common: ctx.common(),
            hide_ids: fpc::HideIds::CONVENTION,
            teams_list: TeamsList::parse("ID\tName\n792\t/egg/\n").unwrap(),
            exports_root: root.join("exports"),
            exports: Vec::new(),
        }
    }

    #[test]
    fn a_7z_export_s_source_keeps_its_decompressed_content_into_the_planned_run() {
        // `check` lets the archive go (`check.rs`'s call), `compile` keeps it: the tasks
        // read the bytes the pass decompressed, not a second read of the archive.
        let temp = scratch("compile_7z_kept");
        let root = temp.path();
        fs::create_dir_all(root.join("exports")).unwrap();
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/sources/egg Midcup Tracer.7z"),
            root.join("exports/egg Midcup Tracer.7z"),
        )
        .unwrap();
        let ctx = tool_context(root, "");
        let inputs = sandbox_inputs(root, &ctx);
        let sources = reader::discover(&inputs.exports_root, &[]).unwrap().sources;

        let planned = plan(
            &inputs,
            sources,
            InstalledPaths::Unknown,
            &OutputMode::Test,
            None,
            RunEvents::new(&ctx),
            &ctx,
        )
        .unwrap();

        let [source] = planned.sources.as_slice() else {
            panic!("{} sources", planned.sources.len());
        };
        assert!(
            source.content.is_some(),
            "the .7z stays decompressed for its tasks"
        );
    }

    // TC-PLN-06
    #[test]
    fn a_file_replaced_after_planning_aborts_the_run_and_keeps_the_previous_cpk() {
        // The boots are read before any batch is committed, so nothing is staged yet; the kit
        // is read after the player's group and the portrait are, so a staged CPK exists and
        // must be discarded.
        for (scratch_name, replaced) in [
            ("compile_boots_changed", format!("{PLAYER}/boots.fmdl")),
            ("compile_kit_changed", "Kits/g1/kit.dds".to_owned()),
        ] {
            let temp = sandbox(scratch_name);
            let root = temp.path();
            let output = root.join("output");
            deploy::prepare_output_folder(&output).unwrap();
            let layout = CpkLayout::Single {
                name: CpkStem::new("4cc_99_test").unwrap(),
                cap: 3 << 30,
            };
            let cpk = output.join("4cc_99_test.cpk");
            let ctx = tool_context(root, "");
            let inputs = sandbox_inputs(root, &ctx);
            let normal = OutputMode::Normal { no_deploy: false };
            let paths = RunPaths {
                output_folder: output.clone(),
                savefile_path: SavefilePath::Path(root.join("save/EDIT00000000")),
                game: RunningGame {
                    probe: Vec::new,
                    edit_mode_closed: false,
                },
            };
            run(&inputs, &layout, None, &paths, &normal, &ctx).unwrap();
            let previous = fs::read(&cpk).unwrap();

            let (events_tx, events) = unbounded();
            let ctx = ctx.with_events(events_tx);
            let sources = reader::discover(&inputs.exports_root, &[]).unwrap().sources;
            let planned = plan(
                &inputs,
                sources,
                InstalledPaths::Unknown,
                &normal,
                None,
                RunEvents::new(&ctx),
                &ctx,
            )
            .unwrap();
            let file = root
                .join("exports")
                .join("egg Midcup Tracer")
                .join(&replaced);
            fs::write(&file, b"saved over from Blender").unwrap();
            let bins = WorkingBins::bundled(PesVersion::Pes21, &Templates::embedded());
            let worst = build(
                planned,
                bins,
                Templates::embedded(),
                &layout,
                None,
                &paths,
                &normal,
            )
            .unwrap();

            // Fatal is exit code 3: `cli.rs`'s `the_worst_severity_decides_the_exit_code`.
            assert_eq!(worst, Some(Severity::Fatal), "{replaced}");
            let fatal: Vec<Message> = events
                .try_iter()
                .filter_map(|envelope| {
                    let PipelineEvent::Message(message) = envelope.event else {
                        return None;
                    };
                    (message.severity == Severity::Fatal).then_some(message)
                })
                .collect();
            let [message] = fatal.as_slice() else {
                panic!("{replaced}: {fatal:?}");
            };
            assert_eq!(message.code.code, "source_changed_during_run");
            assert_eq!(message.disposition, Disposition::AbortRun);
            assert_eq!(
                message.scope,
                Scope::Export {
                    export_id: ExportId(0)
                }
            );
            assert_eq!(
                message.context,
                [("path".to_owned(), file.display().to_string())]
            );
            assert!(
                fs::read(&cpk).unwrap() == previous,
                "{replaced}: the previous CPK is kept"
            );
            assert!(!output.join(".staging").exists(), "{replaced}");
        }
    }

    // TC-OUT-15
    #[test]
    fn a_single_cpk_longer_than_the_cap_is_cpk_size_over_limit_and_one_at_the_cap_is_not() {
        let temp = scratch("compile_size_over_limit");
        let folder = temp.path();
        let name = CpkStem::new("4cc_61_midcup").unwrap();
        fs::write(folder.join("4cc_61_midcup.cpk"), vec![0; 5000]).unwrap();

        assert_eq!(
            size_over_limit(folder, &name, 4096).unwrap(),
            Some(tool_message(
                Code::CpkSizeOverLimit,
                Scope::Run,
                Disposition::Keep,
                vec![
                    ("cpk", "4cc_61_midcup.cpk".to_owned()),
                    ("size", "4.9 KiB".to_owned()),
                    ("cap", "4 KiB".to_owned()),
                ],
            ))
        );
        assert_eq!(size_over_limit(folder, &name, 5000).unwrap(), None);

        // The CPK was just written: a size that cannot be read is the error, naming it.
        let missing = CpkStem::new("4cc_62_midcup").unwrap();
        let error = size_over_limit(folder, &missing, 4096).unwrap_err();
        let path = folder.join("4cc_62_midcup.cpk");
        assert_eq!(
            error.to_string(),
            format!("cannot read the size of {}", path.display())
        );
    }

    #[test]
    fn the_refs_cpk_is_the_run_s_last_cpk_in_either_layout_and_sends_its_cpks_to_the_folder() {
        let stem = |name: &str| CpkStem::new(name).unwrap();
        let refs = stem("4cc_18_referees");
        let official = [
            "4cc_08_bins.cpk".to_owned(),
            "4cc_18_referees.cpk".to_owned(),
            "4cc_42_teams.cpk".to_owned(),
            "4cc_41_teams.cpk".to_owned(),
        ];
        let single = CpkLayout::Single {
            name: stem("4cc_99_test"),
            cap: 3 << 30,
        };
        let parts = CpkLayout::Parts {
            bins: stem("4cc_08_bins"),
            teams_stem: "teams".to_owned(),
            cap: 3 << 30,
        };

        assert_eq!(single.cpks(&official, None), [stem("4cc_99_test")]);
        assert_eq!(
            single.cpks(&official, Some(&refs)),
            [stem("4cc_99_test"), refs.clone()]
        );
        assert_eq!(
            parts.cpks(&official, Some(&refs)),
            [
                stem("4cc_08_bins"),
                stem("4cc_41_teams"),
                stem("4cc_42_teams"),
                refs.clone()
            ]
        );

        let output = Path::new("output");
        assert_eq!(
            single.promoted(output, None),
            output.join("4cc_99_test.cpk")
        );
        assert_eq!(single.promoted(output, Some(&refs)), output);
        assert_eq!(parts.promoted(output, Some(&refs)), output);
    }
}
