//! The canonical-order CPK writer (`spec/team_compiler/pipeline.md` "5. Writer"): the `overrides/`
//! files first, then task batches in manifest order whatever order they arrive in, a player
//! folder's group decided as one once its textures batch is in, then the bins. In multi-CPK
//! mode the batches' entries go to the teams parts instead, a team at a time (`parts`). In a
//! normal run holding a refs export, that export's entries go to the refs CPK instead, followed
//! by the referee template tree (`Referees`).

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::ops::Range;
use std::path::PathBuf;

use aesthetics_export::ExportCoverage;
use anyhow::{Context, ensure};
use kit_config::{KitConfig, KitSlot};
use pes_version::{Engine, PesVersion};
use studio_core::{Disposition, Message, Scope};

use crate::bins::kit_configs::{kit_configs, loose_kit_configs};
use crate::bins::shared_ids::SharedIds;
use crate::bins::{KitColorEntry, Rgb, WorkingBins, uni_color_kit_number};
use crate::messages::{Code, tool_message};
use crate::output::parts::TeamsParts;
use crate::output::sink::OutputSink;
use crate::paths::{self, REFEREE_MARKER_COLLAR};
use crate::plan::TeamKits;
use crate::processing::TaskBatch;
use crate::templates::Templates;

/// The manifest's per-team writes beside the tasks' own: the `TeamColor.bin` colors, the
/// `UniColor.bin`/`UniformParameter.bin` kits and the shared-IDs records.
#[derive(Clone, Copy)]
pub(crate) struct BinsEdits<'a> {
    /// Each planned team's colors for its `TeamColor.bin` record.
    pub(crate) team_colors: &'a [(u16, Vec<Rgb>)],
    /// Each planned team export's kits, for its `UniColor.bin` record and
    /// `UniformParameter.bin` configs.
    pub(crate) team_kits: &'a [TeamKits],
    /// Each planned team export's shared-IDs record to write, whatever its folders' tasks'
    /// outcomes.
    pub(crate) shared_ids: &'a [(u16, SharedIds)],
    /// A patch team was planned: the patch is the run's work as an override is, so a run
    /// that commits nothing still writes a CPK to publish it.
    pub(crate) patch_planned: bool,
}

impl BinsEdits<'static> {
    /// The empty edits: no team compiled anything.
    #[cfg(test)]
    pub(crate) fn empty() -> BinsEdits<'static> {
        BinsEdits {
            team_colors: &[],
            team_kits: &[],
            shared_ids: &[],
            patch_planned: false,
        }
    }
}

/// One CPK's entries being written into `sink`, a CPK file or a folder of loose files. The
/// overrides go in with the first committed entry, or at the end when only they go in, so a
/// run that commits nothing and has no override adds nothing to the sink, which then creates
/// no file and no folder.
pub(crate) struct CpkOutput {
    /// Where the entries are written.
    sink: OutputSink,
    /// Whether the overrides are in the sink: they are its first entries.
    started: bool,
    /// The `overrides/` files, by CPK path: the first entries of the CPK, and the paths no task
    /// or bin may take.
    overrides: BTreeMap<String, PathBuf>,
    /// The manifest position of the next batch to commit.
    next: usize,
    /// Batches that arrived before an earlier one, or a player folder's packages held for its
    /// textures batch, by manifest position.
    pending: BTreeMap<usize, TaskBatch>,
    /// The committed kits' and, on PES 18-21, the referee template tree's kit configs'
    /// `UniformParameter.bin` entries.
    uniform_parameters: Vec<(String, Vec<u8>)>,
    /// The committed kits' `UniColor.bin` entries, each with its team ID, in commit order.
    kit_colors: Vec<(u16, KitColorEntry)>,
    /// The committed `Midcup` kits that emitted no config, each with its team ID and slot:
    /// `kit_configs` edits each slot's installed config in place, or writes the template
    /// where none is installed.
    configless_kits: Vec<(u16, KitSlot)>,
    /// The manifest positions of the batches committed: a player's boots or gloves ID is
    /// settled only when the task building them is among them.
    committed: BTreeSet<usize>,
    /// What goes before each bin's game path in the sink: empty, or test mode's `_bins/`.
    bins_prefix: String,
    /// Multi-CPK mode's teams parts, which take the batches' entries, the sink then being the
    /// bins CPK; `None` when every entry goes into the sink.
    parts: Option<TeamsParts>,
    /// The refs export's tasks, whose entries go to the refs CPK when there is one and are
    /// followed by the referee template tree; `None` when the run writes no tree (test mode,
    /// or a run without a refs export).
    referees: Option<Referees>,
    /// The entries written across the run's outputs: the sink's, the parts' and the refs
    /// CPK's, for `Complete`'s files-written count.
    entries: usize,
}

/// Which of a run's outputs `CpkOutput::finish` wrote: each is written only when something
/// went into it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Written {
    /// The team side: the sink (the single CPK, the bins CPK or the loose tree), with the teams
    /// parts and their placeholders when there are parts.
    pub(crate) team: bool,
    /// The refs CPK.
    pub(crate) refs: bool,
}

/// The refs export's tasks being written: into the refs CPK, which no entry of the team side
/// goes into, in a normal run; with the team side's entries in sideload mode. Whether an entry
/// of them went in decides whether the referee template tree is written after them.
pub(crate) struct Referees {
    /// The manifest positions of the refs export's tasks: one export's, so one range.
    tasks: Range<usize>,
    /// The refs CPK, created with its first entry; `None` when the refs export's entries go
    /// into the team side's sink with the others (sideload mode).
    cpk: Option<OutputSink>,
    /// Whether an entry of the tasks went in: a refs export that commits nothing writes no
    /// refs CPK and no tree, so the installed referees stay.
    committed: bool,
    /// The paths of the tasks' entries that went in, wherever they went: the tree's file at
    /// one of them is left out, the member's file laying over the template's.
    entry_paths: BTreeSet<String>,
    /// Whether the referees' marker went in: the tree's file at `marker_path` is then left
    /// out, and on PES 18-21 the referee kit configs of the tree are written wearing its
    /// collar.
    marker: bool,
    /// The game path of the entry that is the marker for the run's engine
    /// (`paths::referee_marker`): the collar's model on Fox, the prop texture on pre-Fox. The
    /// entry that notes the marker.
    marker_path: String,
}

impl Referees {
    /// The tasks at the manifest positions `tasks`, compiled for a target of `engine`, their
    /// entries going into the refs CPK at `path`. Nothing is written yet.
    pub(crate) fn cpk(path: PathBuf, tasks: Range<usize>, engine: Engine) -> Referees {
        Referees {
            tasks,
            cpk: Some(OutputSink::cpk(path)),
            committed: false,
            entry_paths: BTreeSet::new(),
            marker: false,
            marker_path: paths::referee_marker(engine),
        }
    }

    /// The tasks at the manifest positions `tasks`, compiled for a target of `engine`, their
    /// entries going into the team side's sink with the others.
    pub(crate) fn in_sink(tasks: Range<usize>, engine: Engine) -> Referees {
        Referees {
            tasks,
            cpk: None,
            committed: false,
            entry_paths: BTreeSet::new(),
            marker: false,
            marker_path: paths::referee_marker(engine),
        }
    }

    /// Takes the entry `path` of the task at manifest position `index` when that task is the
    /// refs export's: noted as committed, its path noted (and as the marker, at the marker's
    /// path), and written as `bytes` into the refs CPK when there is one. Whether the refs CPK
    /// took it; an entry it did not take goes to the team side.
    fn add(&mut self, index: usize, path: &str, bytes: &[u8]) -> anyhow::Result<bool> {
        if !self.tasks.contains(&index) {
            return Ok(false);
        }
        self.committed = true;
        self.entry_paths.insert(path.to_owned());
        // The path alone tells: only the marker task writes the marker's path (on Fox with its
        // texture in the same batch, and a batch commits whole or not at all).
        if path == self.marker_path {
            self.marker = true;
        }
        let Some(cpk) = &mut self.cpk else {
            return Ok(false);
        };
        cpk.add(path, bytes)?;
        Ok(true)
    }
}

/// The referee kit config at game path `path`, holding `bytes`, wearing the referees' marker:
/// its collar and winter collar set to the marker's collar and nothing else changed, encoded
/// for `version`. A config that does not decode is the error, naming its path.
fn wearing_marker(path: &str, bytes: &[u8], version: PesVersion) -> anyhow::Result<Vec<u8>> {
    let mut config =
        KitConfig::decode(bytes, version).with_context(|| format!("{path} is not a kit config"))?;
    config.shirt.collar = REFEREE_MARKER_COLLAR;
    config.shirt.winter_collar = REFEREE_MARKER_COLLAR;
    Ok(config.encode(version).to_vec())
}

impl CpkOutput {
    /// A CPK's entries to be written into `sink`, starting with `overrides` (CPK path, file on
    /// disk), each file read when it is added, the bins at their game paths after
    /// `bins_prefix`; the batches' entries go to `parts` instead when there are parts.
    /// Nothing is written yet.
    pub(crate) fn new(
        sink: OutputSink,
        overrides: BTreeMap<String, PathBuf>,
        bins_prefix: &str,
        parts: Option<TeamsParts>,
    ) -> CpkOutput {
        CpkOutput {
            sink,
            started: false,
            overrides,
            next: 0,
            pending: BTreeMap::new(),
            uniform_parameters: Vec::new(),
            kit_colors: Vec::new(),
            configless_kits: Vec::new(),
            committed: BTreeSet::new(),
            bins_prefix: bins_prefix.to_owned(),
            parts,
            referees: None,
            entries: 0,
        }
    }

    /// The same output, with the entries of `referees`' tasks going where it says, and the
    /// referee template tree after them when one went in.
    pub(crate) fn with_referees(self, referees: Referees) -> CpkOutput {
        CpkOutput {
            referees: Some(referees),
            ..self
        }
    }

    /// Takes `batch` and commits every batch that is now next in manifest order. Committing in
    /// manifest order, not arrival order, is what makes the CPK's layout the same on every run.
    /// A player folder's group (`TaskBatch::group`) is decided only once every batch of it is
    /// in: its packages wait, their memory still charged, for the textures batch that decides
    /// them. Returns the batches decided, in manifest order, as each one's manifest position
    /// and messages (a `duplicate_path` for each entry an override replaced among them), so
    /// the caller reports findings in manifest order too; empty when `batch` waits for an
    /// earlier one. With parts, a team is placed once the batches decided hold its export's
    /// last task, whether that task committed or not; a team the parts cannot take is the
    /// error (`parts::Unplaced`).
    pub(crate) fn submit(
        &mut self,
        batch: TaskBatch,
    ) -> anyhow::Result<Vec<(usize, Vec<Message>)>> {
        self.pending.insert(batch.index, batch);
        let mut committed = Vec::new();
        while let Some(next) = self.pending.get(&self.next) {
            let group = next.group.clone();
            let range = group.clone().unwrap_or(self.next..self.next + 1);
            if !range.clone().all(|index| self.pending.contains_key(&index)) {
                break;
            }
            let mut batches: Vec<TaskBatch> = range
                .clone()
                .map(|index| {
                    self.pending
                        .remove(&index)
                        .expect("every position was checked")
                })
                .collect();
            let result = match group {
                Some(_) => self.commit_folder(&mut batches),
                None => batches.iter_mut().try_for_each(|batch| self.commit(batch)),
            };
            // After the decision, whatever it was: a failed last task ends its team too.
            let result = result.and_then(|()| match &mut self.parts {
                Some(parts) => parts.decided(range.clone()),
                None => Ok(()),
            });
            if let Err(error) = result {
                // The CPK is lost, so the batches waiting here go now: their permits may be
                // what the coordinator is waiting for, and it must reach the end of the run.
                self.pending.clear();
                return Err(error);
            }
            for batch in batches {
                committed.push((batch.index, batch.messages));
            }
            self.next = range.end;
        }
        Ok(committed)
    }

    /// Decides a player folder's group, `batches` in manifest order with the textures batch
    /// last. When the textures failed nothing of the folder commits: a package in the CPK
    /// would point at textures that are not. Otherwise each package that succeeded commits,
    /// then the textures, when at least one package did. A batch that failed has
    /// no entries; one that succeeded emits at least one, unless a texture link stood at
    /// every path a source supplied, the Common texture each link names serving the models
    /// that look for them — then the batch is legitimately empty and the folder's packages
    /// commit all the same.
    fn commit_folder(&mut self, batches: &mut [TaskBatch]) -> anyhow::Result<()> {
        let (textures, packages) = batches
            .split_last_mut()
            .expect("a player folder's group ends with its textures batch");
        if textures.entries.is_empty()
            && textures
                .messages
                .iter()
                .any(|message| message.disposition != Disposition::Keep)
        {
            return Ok(());
        }
        let mut committed = false;
        for package in packages {
            committed |= !package.entries.is_empty();
            self.commit(package)?;
        }
        if committed {
            self.commit(textures)?;
        }
        Ok(())
    }

    /// Adds the referee template tree of `templates` and closes the refs CPK
    /// (`finish_referees`), then finishes the team side (`finish_team`). Returns which were
    /// written, the entries written across them, the findings: the tree's, then the team
    /// side's, and the manifest positions of the batches committed, which the patch's
    /// provisional IDs are settled against.
    pub(crate) fn finish(
        mut self,
        version: PesVersion,
        bins: WorkingBins,
        edits: BinsEdits<'_>,
        templates: &Templates,
    ) -> anyhow::Result<(Written, usize, Vec<Message>, BTreeSet<usize>)> {
        ensure!(
            self.pending.is_empty(),
            "the writer never received task {} of the manifest",
            self.next
        );
        let mut messages = Vec::new();
        let refs = match self.referees.take() {
            Some(referees) => self.finish_referees(referees, version, templates, &mut messages)?,
            None => false,
        };
        let (team, entries, team_messages, committed) = self.finish_team(version, bins, &edits)?;
        messages.extend(team_messages);
        Ok((Written { team, refs }, entries, messages, committed))
    }

    /// When an entry of the refs export went in, adds `version`'s engine's referee template
    /// tree of `templates` after it, each file at its game path unless an entry of the refs
    /// export went in at that path (left out with no finding: the member's file lays over the
    /// template's) or an override holds it (`overridden`,
    /// its `duplicate_path` going to `messages`): into the refs CPK, or without one into the
    /// team side's sink, which it then starts. When the referees' marker went in, the tree's
    /// file at the marker's path is left out (pre-Fox: the prop texture), and on PES 18-21
    /// each referee kit config of the tree is written wearing its collar, encoded for
    /// `version` (`wearing_marker`); a config that does not decode is the error. On PES 18-21
    /// each kit config written is also staged as the `UniformParameter.bin` entry of its file
    /// name, with the bytes written. Closes the refs CPK, and returns whether it was written.
    fn finish_referees(
        &mut self,
        referees: Referees,
        version: PesVersion,
        templates: &Templates,
        messages: &mut Vec<Message>,
    ) -> anyhow::Result<bool> {
        let Referees {
            mut cpk,
            committed,
            entry_paths,
            marker,
            marker_path,
            ..
        } = referees;
        // The pre-Fox marker is the prop's texture, no collar: the configs keep their own.
        let configs_wear_marker = marker
            && match version.engine() {
                Engine::Fox => true,
                Engine::PreFox => false,
            };
        if committed {
            for (path, bytes) in templates.referee_tree(version.engine()) {
                // The marker's task already wrote this path, and the sink refuses a duplicate
                // path; the bytes there are the member's marker, not the template's file.
                if marker && path == marker_path {
                    continue;
                }
                // Exact comparison, as the CPK's own duplicate check compares.
                if entry_paths.contains(path) {
                    continue;
                }
                if self.overridden(path, messages) {
                    continue;
                }
                let config_name = path.strip_prefix(paths::REFEREE_KIT_CONFIGS);
                let bytes = if configs_wear_marker && config_name.is_some() {
                    Cow::Owned(wearing_marker(path, bytes, version)?)
                } else {
                    Cow::Borrowed(bytes)
                };
                match &mut cpk {
                    Some(cpk) => cpk.add(path, &bytes)?,
                    None => {
                        self.start()?;
                        self.sink.add(path, &bytes)?;
                    }
                }
                self.entries += 1;
                let Some(name) = config_name else {
                    continue;
                };
                match version.engine() {
                    // The game needs the loose file but reads a referee config's values from
                    // its `UniformParameter.bin` entry, as it does for a team's kit configs.
                    Engine::Fox => self
                        .uniform_parameters
                        .push((name.to_owned(), bytes.into_owned())),
                    Engine::PreFox => {}
                }
            }
        }
        match cpk {
            Some(cpk) => {
                cpk.finish()?;
                Ok(committed)
            }
            None => Ok(false),
        }
    }

    /// Writes the bins and closes the CPK, when a team's batch committed something, there is
    /// an override, or the referee configs changed the bins on PES 18-21 (else no file exists,
    /// no bin is built and nothing is reported):
    /// `UniformParameter.bin`, built on `bins`' by `kit_configs` from `team_kits` and the
    /// committed kit configs, when that changed it; on PES 15-17, which have no such bin (a
    /// committed kit config there is an error), the installed loose kit configs that
    /// `loose_kit_configs` edits for `team_kits`, each at its loose path; then `TeamColor.bin`,
    /// built on `bins`' with every record's header set from its position and each of
    /// `team_colors` (team id, colors) set in its team's record; then `UniColor.bin`, built on
    /// `bins`' the same way, each `Full` export's team of `team_kits` keeping only its kit
    /// tasks' kits in its record, then each committed kit's entry merged into its team's record
    /// in commit order; with parts, the teams parts are finished after the sink, the slots
    /// left over written as the placeholder (no part and no placeholder when no file is
    /// written). Returns whether a CPK was written, the entries written across the run's
    /// outputs so far (`self.entries`: the sink's, the parts' and the refs CPK's), the
    /// findings to report: `kit_configs`'
    /// (or `loose_kit_configs`') FPC findings, one `bin_header_repaired` per working bin that
    /// had a header wrong, naming the teams, and a `duplicate_path` for each bin an override
    /// replaced; and the manifest positions of the batches committed.
    fn finish_team(
        mut self,
        version: PesVersion,
        bins: WorkingBins,
        edits: &BinsEdits<'_>,
    ) -> anyhow::Result<(bool, usize, Vec<Message>, BTreeSet<usize>)> {
        let BinsEdits {
            team_colors,
            team_kits,
            shared_ids,
            patch_planned,
        } = *edits;
        // A run in which no team committed anything writes no file, so it adds no bin either,
        // unless the overrides go into the CPK: they are written whatever the exports bring;
        // or the referee configs changed the bins on PES 18-21: their entries need the bin;
        // or a patch team was planned: the patch is the run's work as an override is, a
        // settings-only export's compile otherwise losing it with no error; or the
        // team colors, a kit's committed `UniColor.bin` entry or a configless slot's edit
        // went in: the bins are then the run's whole work.
        if !self.started
            && self.overrides.is_empty()
            && self.uniform_parameters.is_empty()
            && self.kit_colors.is_empty()
            && self.configless_kits.is_empty()
            && team_colors.is_empty()
            && !patch_planned
        {
            return Ok((
                false,
                self.entries,
                Vec::new(),
                std::mem::take(&mut self.committed),
            ));
        }
        self.start()?;
        let WorkingBins {
            team_color,
            uni_color,
            uniform_parameter,
            loose_kit_configs: installed_kit_configs,
        } = bins;
        let mut messages = Vec::new();
        // The bins hold what every committed kit contributed, so they are built only once
        // every batch is in, and go last.
        let committed_configs = std::mem::take(&mut self.uniform_parameters);
        match uniform_parameter {
            Some(mut bin) => {
                // Before `UniColor.bin` is edited below: the kits an absent slot is patched
                // for are those the team's working record holds.
                let (changed, generated, findings) = kit_configs(
                    &mut bin,
                    &uni_color,
                    team_kits,
                    committed_configs,
                    &self.configless_kits,
                    version,
                )?;
                messages.extend(findings);
                if changed {
                    self.add_bin(paths::UNIFORM_PARAMETER, &bin.write(), &mut messages)?;
                }
                // The template a `Midcup` kit without `config.toml` gets where nothing
                // is installed goes out as a kit task's own does: the loose file too.
                for (path, config) in generated {
                    self.add_bin(&path, &config, &mut messages)?;
                }
            }
            None => {
                // A committed kit's loose config is its task's entry on PES 15-17.
                ensure!(
                    committed_configs.is_empty(),
                    "{version} has no UniformParameter.bin"
                );
                // As above, before `UniColor.bin` is edited. An absent slot has no kit task,
                // so no committed entry shares its path.
                let (configs, findings) = loose_kit_configs(
                    &installed_kit_configs,
                    &uni_color,
                    team_kits,
                    &self.configless_kits,
                    version,
                )?;
                messages.extend(findings);
                for (path, config) in configs {
                    self.add_bin(&path, &config, &mut messages)?;
                }
            }
        }
        let mut bin = team_color;
        messages.extend(header_repaired("TeamColor.bin", &bin.repair_headers()));
        for (team_id, colors) in team_colors {
            bin.set_colors(*team_id, colors)?;
        }
        self.add_bin(paths::TEAM_COLOR, &bin.into_bytes(), &mut messages)?;
        let mut bin = uni_color;
        messages.extend(header_repaired("UniColor.bin", &bin.repair_headers()));
        // Before the merge: a failed kit's number is among a `Full` team's, so its entry stays
        // as it was, and every committed entry then replaces or joins what is kept.
        for team in team_kits {
            match team.coverage {
                ExportCoverage::Full => {
                    let numbers: Vec<u8> = team
                        .slots
                        .iter()
                        .map(|slot| uni_color_kit_number(*slot))
                        .collect();
                    bin.keep_kits(team.team_id, &numbers)?;
                }
                ExportCoverage::Midcup => {}
            }
        }
        for (team_id, entry) in std::mem::take(&mut self.kit_colors) {
            bin.set_kit(team_id, &entry)?;
        }
        self.add_bin(paths::UNI_COLOR, &bin.into_bytes(), &mut messages)?;
        // Every compiled team's shared-IDs record, the planned assignment whatever the
        // folders' tasks' outcomes, beside the bins.
        for (team_id, record) in shared_ids {
            self.add_bin(&paths::shared_ids(*team_id), &record.write(), &mut messages)?;
        }
        self.sink.finish()?;
        if let Some(parts) = self.parts {
            self.entries += parts.finish()?;
        }
        Ok((
            true,
            self.entries,
            messages,
            std::mem::take(&mut self.committed),
        ))
    }

    /// A failed task contributes nothing: no entry, no kit config or kit colors to the bins.
    /// A task with an entry an override replaced still contributes the
    /// rest, its kit config and kit colors included, and the `duplicate_path` joins its
    /// messages. With parts, the entries are held for the task's team instead of written. A
    /// task of the refs export writes into the refs CPK when there is one, and then does not
    /// start the team side. A `Midcup` kit may commit with no file at all: one resending only
    /// its colors holds only a `UniColor.bin` entry.
    fn commit(&mut self, batch: &mut TaskBatch) -> anyhow::Result<()> {
        if let Some(kit) = batch.configless_kit.take() {
            self.configless_kits.push(kit);
        }
        if batch.entries.is_empty() && batch.uniparam.is_none() && batch.uni_color.is_none() {
            return Ok(());
        }
        for (path, bytes) in std::mem::take(&mut batch.entries) {
            if self.overridden(&path, &mut batch.messages) {
                continue;
            }
            let in_refs_cpk = match &mut self.referees {
                Some(referees) => referees.add(batch.index, &path, &bytes)?,
                None => false,
            };
            if in_refs_cpk {
                self.entries += 1;
                continue;
            }
            self.start()?;
            match &mut self.parts {
                Some(parts) => parts.hold(path, bytes),
                None => self.sink.add(&path, &bytes)?,
            }
            self.entries += 1;
        }
        self.uniform_parameters.extend(batch.uniparam.take());
        self.kit_colors.extend(batch.uni_color.take());
        self.committed.insert(batch.index);
        // The task's bytes are in the CPK, or held for its team, and the batch holds none, so
        // its permit and its output's charge go now. The release is explicit: a grouped batch
        // can stay in `submit`'s vector after its own commit, so waiting on its destruction
        // would hold them longer. A held team's bytes are thus outside the budget: its next
        // task cannot start until its charge fits, so a team whose charges exceed the budget
        // would otherwise wait forever on its own held batches.
        batch.permit = None;
        batch.output = None;
        Ok(())
    }

    /// Adds `bytes` at `path` to the sink, after the overrides when this is its first entry,
    /// unless an override holds `path` (`overridden`).
    fn add(&mut self, path: &str, bytes: &[u8], messages: &mut Vec<Message>) -> anyhow::Result<()> {
        if self.overridden(path, messages) {
            return Ok(());
        }
        self.start()?;
        self.sink.add(path, bytes)?;
        self.entries += 1;
        Ok(())
    }

    /// Whether an override holds `path`, so that an entry there does not go into the output,
    /// the override having won: a `duplicate_path` naming the path then goes to `messages`.
    /// Paths compare exactly, as the CPK's own duplicate check does.
    fn overridden(&self, path: &str, messages: &mut Vec<Message>) -> bool {
        if !self.overrides.contains_key(path) {
            return false;
        }
        messages.push(tool_message(
            Code::DuplicatePath,
            Scope::Run,
            Disposition::Keep,
            vec![("path", path.to_owned())],
        ));
        true
    }

    /// Adds the bin `bytes` at its game path `path`, after the bins prefix, as `add` adds an
    /// entry.
    fn add_bin(
        &mut self,
        path: &str,
        bytes: &[u8],
        messages: &mut Vec<Message>,
    ) -> anyhow::Result<()> {
        let path = format!("{}{path}", self.bins_prefix);
        self.add(&path, bytes, messages)
    }

    /// Adds the overrides, in path order, so they are the sink's first entries; nothing once
    /// they are in. Each override is read as it is added: a handful of files, never charged to
    /// the memory budget. One that cannot be read fails the output, naming it: a file the
    /// operator put there on purpose is not skipped.
    fn start(&mut self) -> anyhow::Result<()> {
        if self.started {
            return Ok(());
        }
        for (path, file) in &self.overrides {
            let bytes = fs::read(file)
                .with_context(|| format!("cannot read the override {}", file.display()))?;
            self.sink.add(path, &bytes)?;
            self.entries += 1;
        }
        self.started = true;
        Ok(())
    }
}

/// The `bin_header_repaired` finding for the working bin `bin` whose headers were wrong for
/// the `repaired` teams; none when every header was sound.
fn header_repaired(bin: &str, repaired: &[u16]) -> Option<Message> {
    if repaired.is_empty() {
        return None;
    }
    let teams: Vec<String> = repaired.iter().map(u16::to_string).collect();
    Some(tool_message(
        Code::BinHeaderRepaired,
        Scope::Run,
        Disposition::Keep,
        vec![("bin", bin.to_owned()), ("teams", teams.join(", "))],
    ))
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::ops::Range;
    use std::path::Path;
    use std::sync::{Arc, mpsc};
    use std::thread;
    use std::time::Duration;

    use cpk::CpkArchive;
    use kit_config::{KitConfig, matches_fpc};
    use pipeline::MemoryBudget;
    use studio_core::{Disposition, ExportId, Scope};
    use uniparam::UniformParameter;

    use super::*;
    use crate::bins::{TeamColorBin, UniColorBin};
    use crate::messages::{Code, tool_message};
    use crate::plan::{EffectiveTeamKitFpc, TeamKitEdits};
    use crate::templates::Templates;
    use crate::testing::scratch;

    /// The bins a PES 21 run with no installed bin builds on.
    fn bundled() -> WorkingBins {
        WorkingBins::bundled(PesVersion::Pes21, &Templates::embedded())
    }

    /// The bundled `TeamColor.bin`'s bytes.
    fn bundled_team_color() -> Vec<u8> {
        Templates::embedded().team_color().to_vec()
    }

    /// The bundled `UniColor.bin`'s bytes.
    fn bundled_uni_color() -> Vec<u8> {
        Templates::embedded().uni_color().to_vec()
    }

    /// `output` finished for PES `version` on the bundled bins with no team colors, the bins
    /// asserted to report nothing: whether a CPK was written.
    fn finish_plain(output: CpkOutput, version: PesVersion) -> anyhow::Result<bool> {
        let (written, _entries, messages, _) = output.finish(
            version,
            WorkingBins::bundled(version, &Templates::embedded()),
            BinsEdits::empty(),
            &Templates::embedded(),
        )?;
        assert_eq!(messages, []);
        Ok(written.team)
    }

    /// A batch whose one message names its index.
    fn batch(index: usize, paths: &[&str], uniparam: Option<&str>) -> TaskBatch {
        TaskBatch {
            index,
            entries: paths
                .iter()
                .map(|path| ((*path).to_owned(), path.as_bytes().to_vec()))
                .collect(),
            group: None,
            uniparam: uniparam.map(|name| (name.to_owned(), vec![7; 120])),
            uni_color: None,
            configless_kit: None,
            messages: vec![note(index)],
            permit: None,
            output: None,
        }
    }

    /// `batch`, as a member of the player folder group at `group`.
    fn grouped(index: usize, group: Range<usize>, paths: &[&str]) -> TaskBatch {
        TaskBatch {
            group: Some(group),
            ..batch(index, paths, None)
        }
    }

    /// A folder's group at 1..5 (face, boots, gloves, textures) between a face at 0 and a kit
    /// at 5, each package's entries given or empty for a failed one, the textures' likewise.
    fn folder_run(packages: [&[&str]; 3], textures: &[&str]) -> Vec<TaskBatch> {
        let [face, boots, gloves] = packages;
        vec![
            batch(0, &["face/real/71403/face.fpk"], None),
            grouped(1, 1..5, face),
            grouped(2, 1..5, boots),
            grouped(3, 1..5, gloves),
            grouped(4, 1..5, textures),
            batch(5, &["kit/kit.ftex"], Some("kit")),
        ]
    }

    /// Writes `batches`, submitted in the given order, to `name.cpk` and returns its layout
    /// and the positions `submit` reported, in order.
    fn write_all(folder: &Path, name: &str, batches: Vec<TaskBatch>) -> (Vec<String>, Vec<usize>) {
        let path = folder.join(format!("{name}.cpk"));
        let mut output = CpkOutput::new(OutputSink::cpk(path.clone()), BTreeMap::new(), "", None);
        let mut reported = Vec::new();
        for batch in batches {
            for (index, messages) in output.submit(batch).unwrap() {
                assert_eq!(messages, [note(index)], "task {index}'s own message");
                reported.push(index);
            }
        }
        assert!(finish_plain(output, PesVersion::Pes21).unwrap());
        (layout(&path), reported)
    }

    #[test]
    fn a_folder_whose_textures_failed_commits_nothing_and_the_rest_of_the_run_goes_on() {
        let temp = scratch("writer_group_textures_failed");
        let batches = folder_run(
            [
                &["face/face.fpk"],
                &["boots/boots.fpk"],
                &["glove/glove.fpk"],
            ],
            &[],
        );
        let (layout, reported) = write_all(temp.path(), "textures_failed", batches);
        assert_eq!(
            layout,
            [
                "face/real/71403/face.fpk",
                "kit/kit.ftex",
                paths::UNIFORM_PARAMETER,
                paths::TEAM_COLOR,
                paths::UNI_COLOR,
            ]
        );
        assert_eq!(reported, [0, 1, 2, 3, 4, 5]);
    }

    /// A writer to `name.cpk` in `folder` that has taken a player folder's face at 0, its
    /// group at 0..2, still waiting for the textures batch at 1.
    fn face_waiting_for_textures(folder: &Path, name: &str) -> CpkOutput {
        let path = folder.join(format!("{name}.cpk"));
        let mut output = CpkOutput::new(OutputSink::cpk(path), BTreeMap::new(), "", None);
        let face = grouped(0, 0..2, &["face/face.fpk"]);
        assert_eq!(
            output.submit(face).unwrap(),
            [],
            "the face waits for its textures"
        );
        output
    }

    /// The textures batch at 1 of the group at 0..2, with no entries and `message` alone.
    fn empty_textures(message: Message) -> TaskBatch {
        TaskBatch {
            messages: vec![message],
            ..grouped(1, 0..2, &[])
        }
    }

    #[test]
    fn an_empty_textures_batch_dropping_its_folder_commits_nothing_of_the_folder() {
        let temp = scratch("writer_group_textures_dropped");
        let mut output = face_waiting_for_textures(temp.path(), "dropped");
        let dropping = tool_message(
            Code::FolderPackFailed,
            Scope::Run,
            Disposition::DropFolder,
            Vec::new(),
        );
        let decided: Vec<usize> = output
            .submit(empty_textures(dropping))
            .unwrap()
            .into_iter()
            .map(|(index, _)| index)
            .collect();
        assert_eq!(decided, [0, 1]);
        output
            .submit(batch(2, &["kit/kit.ftex"], Some("kit")))
            .unwrap();
        assert!(finish_plain(output, PesVersion::Pes21).unwrap());
        assert_eq!(
            layout(&temp.path().join("dropped.cpk")),
            [
                "kit/kit.ftex",
                paths::UNIFORM_PARAMETER,
                paths::TEAM_COLOR,
                paths::UNI_COLOR,
            ]
        );
    }

    #[test]
    fn an_empty_textures_batch_keeping_its_folder_commits_the_packages() {
        // The batch of a folder whose every texture a link of the player's won over a
        // combined folder's: nothing to write, its findings all `Keep`.
        let temp = scratch("writer_group_textures_overridden");
        let mut output = face_waiting_for_textures(temp.path(), "overridden");
        let kept = tool_message(
            Code::SharedTextureOverridden,
            Scope::Run,
            Disposition::Keep,
            Vec::new(),
        );
        let decided: Vec<usize> = output
            .submit(empty_textures(kept))
            .unwrap()
            .into_iter()
            .map(|(index, _)| index)
            .collect();
        assert_eq!(decided, [0, 1]);
        assert!(finish_plain(output, PesVersion::Pes21).unwrap());
        assert_eq!(
            layout(&temp.path().join("overridden.cpk")),
            ["face/face.fpk", paths::TEAM_COLOR, paths::UNI_COLOR,]
        );
    }

    #[test]
    fn a_folder_with_one_failed_package_commits_the_others_and_its_textures() {
        let temp = scratch("writer_group_package_failed");
        let batches = folder_run(
            [&["face/face.fpk"], &[], &["glove/glove.fpk"]],
            &["common/shirt.ftex"],
        );
        let (layout, reported) = write_all(temp.path(), "package_failed", batches);
        assert_eq!(
            layout,
            [
                "face/real/71403/face.fpk",
                "face/face.fpk",
                "glove/glove.fpk",
                "common/shirt.ftex",
                "kit/kit.ftex",
                paths::UNIFORM_PARAMETER,
                paths::TEAM_COLOR,
                paths::UNI_COLOR,
            ]
        );
        assert_eq!(reported, [0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn a_folder_whose_every_package_failed_keeps_its_textures_out() {
        let temp = scratch("writer_group_packages_failed");
        let batches = folder_run([&[], &[], &[]], &["common/shirt.ftex"]);
        let (layout, _) = write_all(temp.path(), "packages_failed", batches);
        assert_eq!(
            layout,
            [
                "face/real/71403/face.fpk",
                "kit/kit.ftex",
                paths::UNIFORM_PARAMETER,
                paths::TEAM_COLOR,
                paths::UNI_COLOR,
            ]
        );
    }

    #[test]
    fn a_group_arriving_out_of_order_lays_the_cpk_out_as_in_order() {
        let temp = scratch("writer_group_out_of_order");
        let folder = temp.path();
        let run = || {
            folder_run(
                [
                    &["face/face.fpk"],
                    &["boots/boots.fpk"],
                    &["glove/glove.fpk"],
                ],
                &["common/shirt.ftex"],
            )
        };
        let (in_order, reported) = write_all(folder, "in_order", run());
        assert_eq!(reported, [0, 1, 2, 3, 4, 5]);
        // The kit first, then the textures before its packages, the packages in reverse, the
        // face at 0 last: nothing is decided before the face arrives, then everything is.
        let mut shuffled = run();
        shuffled.reverse();
        let (out_of_order, reported) = write_all(folder, "out_of_order", shuffled);
        assert_eq!(out_of_order, in_order);
        assert_eq!(reported, [0, 1, 2, 3, 4, 5]);
        assert_eq!(
            in_order,
            [
                "face/real/71403/face.fpk",
                "face/face.fpk",
                "boots/boots.fpk",
                "glove/glove.fpk",
                "common/shirt.ftex",
                "kit/kit.ftex",
                paths::UNIFORM_PARAMETER,
                paths::TEAM_COLOR,
                paths::UNI_COLOR,
            ]
        );
    }

    /// A batch's message naming its `index`, dropping its folder as a failed task's finding
    /// does, so a textures batch with no entries carries what the writer requires of a failed
    /// one.
    fn note(index: usize) -> Message {
        tool_message(
            Code::FolderPackFailed,
            Scope::Run,
            Disposition::DropFolder,
            vec![("task", index.to_string())],
        )
    }

    /// Three batches, the second a kit with a config.
    fn three_batches() -> [TaskBatch; 3] {
        [
            batch(0, &["m/third.bin", "b/fourth.bin"], None),
            batch(1, &["z/second.bin"], Some("kit")),
            batch(2, &["a/first.bin"], None),
        ]
    }

    /// The CPK's entries in the order their bytes sit in the file.
    fn layout(path: &Path) -> Vec<String> {
        let archive = CpkArchive::open(File::open(path).unwrap()).unwrap();
        let mut entries = archive.entries().to_vec();
        entries.sort_by_key(|entry| entry.offset);
        entries.into_iter().map(|entry| entry.path).collect()
    }

    /// The bytes of the CPK at `cpk`'s entry `path`.
    fn entry(cpk: &Path, path: &str) -> Vec<u8> {
        let mut archive = CpkArchive::open(File::open(cpk).unwrap()).unwrap();
        let entry = archive
            .entries()
            .iter()
            .find(|entry| entry.path == path)
            .unwrap_or_else(|| panic!("no {path} in the CPK"))
            .clone();
        archive.read(&entry).unwrap()
    }

    /// What the override file for the CPK path `path` holds: other bytes than a batch's entry
    /// at that path (`batch` writes the path itself).
    fn override_bytes(path: &str) -> Vec<u8> {
        format!("override of {path}").into_bytes()
    }

    /// Overrides at the CPK paths `paths`, each a file in `folder` holding
    /// `override_bytes(path)`.
    fn overrides(folder: &Path, paths: &[&str]) -> BTreeMap<String, PathBuf> {
        let folder = folder.join("overrides");
        fs::create_dir_all(&folder).unwrap();
        paths
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let file = folder.join(format!("{index}.bin"));
                fs::write(&file, override_bytes(path)).unwrap();
                ((*path).to_owned(), file)
            })
            .collect()
    }

    /// The `duplicate_path` finding naming `path`.
    fn duplicate(path: &str) -> Message {
        tool_message(
            Code::DuplicatePath,
            Scope::Run,
            Disposition::Keep,
            vec![("path", path.to_owned())],
        )
    }

    #[test]
    fn the_overrides_go_first_and_replace_a_task_s_entry_at_their_path() {
        let temp = scratch("writer_overrides");
        let folder = temp.path();
        let path = folder.join("cup.cpk");
        let mut output = CpkOutput::new(
            OutputSink::cpk(path.clone()),
            overrides(folder, &["z/over.bin", "a/over.bin"]),
            "",
            None,
        );

        let committed = output
            .submit(batch(0, &["a/over.bin", "b/own.bin"], None))
            .unwrap();

        assert_eq!(committed, [(0, vec![note(0), duplicate("a/over.bin")])]);
        let message = &committed[0].1[1];
        assert_eq!(message.code.code, "duplicate_path");
        assert_eq!(
            (message.severity, message.disposition, &message.scope),
            (
                studio_core::Severity::Warning,
                Disposition::Keep,
                &Scope::Run
            )
        );
        assert!(finish_plain(output, PesVersion::Pes21).unwrap());
        assert_eq!(
            layout(&path),
            [
                "a/over.bin",
                "z/over.bin",
                "b/own.bin",
                paths::TEAM_COLOR,
                paths::UNI_COLOR,
            ]
        );
        assert_eq!(entry(&path, "a/over.bin"), override_bytes("a/over.bin"));
        assert_eq!(entry(&path, "b/own.bin"), b"b/own.bin");
    }

    #[test]
    fn an_override_at_a_bin_s_path_replaces_the_bin_and_finish_reports_it() {
        let temp = scratch("writer_override_bin");
        let folder = temp.path();
        let path = folder.join("cup.cpk");
        let mut output = CpkOutput::new(
            OutputSink::cpk(path.clone()),
            overrides(folder, &[paths::TEAM_COLOR]),
            "",
            None,
        );
        output.submit(batch(0, &["a/b.bin"], None)).unwrap();

        let (written, _entries, messages, _) = output
            .finish(
                PesVersion::Pes21,
                WorkingBins::bundled(PesVersion::Pes21, &Templates::embedded()),
                BinsEdits {
                    team_colors: &team_714_colors(),
                    ..BinsEdits::empty()
                },
                &Templates::embedded(),
            )
            .unwrap();

        assert!(written.team);
        assert_eq!(messages, [duplicate(paths::TEAM_COLOR)]);
        assert_eq!(
            layout(&path),
            [paths::TEAM_COLOR, "a/b.bin", paths::UNI_COLOR]
        );
        assert_eq!(
            entry(&path, paths::TEAM_COLOR),
            override_bytes(paths::TEAM_COLOR)
        );
    }

    #[test]
    fn a_kit_whose_entry_is_overridden_still_gives_its_kit_config_and_colors_to_the_bins() {
        let temp = scratch("writer_override_kit");
        let folder = temp.path();
        let path = folder.join("cup.cpk");
        let mut output = CpkOutput::new(
            OutputSink::cpk(path.clone()),
            overrides(folder, &["kit/kit.ftex"]),
            "",
            None,
        );

        let committed = output.submit(kit_batch(0, &["kit/kit.ftex"])).unwrap();

        assert_eq!(committed, [(0, vec![note(0), duplicate("kit/kit.ftex")])]);
        assert!(finish_plain(output, PesVersion::Pes21).unwrap());
        assert_eq!(entry(&path, "kit/kit.ftex"), override_bytes("kit/kit.ftex"));
        let bin = UniformParameter::read(&entry(&path, paths::UNIFORM_PARAMETER)).unwrap();
        assert_eq!(bin.get("kit"), Some(&[7; 120][..]));
        let kit_colors = entry(&path, paths::UNI_COLOR);
        let start = kit_record(792);
        assert_eq!(
            kit_colors[start + 5..start + 13],
            [0x10, 0x0b, 0xc1, 0x12, 0x00, 0x41, 0x41, 0x41],
            "the kit's entry is merged into team 792's record"
        );
    }

    #[test]
    fn overrides_and_no_committed_batch_still_write_the_cpk_with_the_bins() {
        let temp = scratch("writer_overrides_alone");
        let folder = temp.path();
        let path = folder.join("run/cup.cpk");
        let mut output = CpkOutput::new(
            OutputSink::cpk(path.clone()),
            overrides(folder, &["a/over.bin"]),
            "",
            None,
        );
        // A failed task commits nothing.
        output.submit(batch(0, &[], None)).unwrap();

        assert!(finish_plain(output, PesVersion::Pes21).unwrap());

        assert_eq!(
            layout(&path),
            ["a/over.bin", paths::TEAM_COLOR, paths::UNI_COLOR]
        );
        assert_eq!(entry(&path, "a/over.bin"), override_bytes("a/over.bin"));
        assert!(
            entry(&path, paths::UNI_COLOR) == bundled_uni_color(),
            "no kit committed"
        );
    }

    /// An output into `<folder>/cup.cpk` after `overrides`, the batches of `refs_tasks` going
    /// into `<folder>/refs.cpk`.
    fn with_refs_cpk(
        folder: &Path,
        overrides: BTreeMap<String, PathBuf>,
        refs_tasks: Range<usize>,
    ) -> CpkOutput {
        with_refs_cpk_for(folder, overrides, refs_tasks, Engine::Fox)
    }

    /// `with_refs_cpk` for a target of `engine`.
    fn with_refs_cpk_for(
        folder: &Path,
        overrides: BTreeMap<String, PathBuf>,
        refs_tasks: Range<usize>,
        engine: Engine,
    ) -> CpkOutput {
        CpkOutput::new(OutputSink::cpk(folder.join("cup.cpk")), overrides, "", None)
            .with_referees(Referees::cpk(folder.join("refs.cpk"), refs_tasks, engine))
    }

    /// `entries`, then every game path of the embedded Fox referee template tree but those of
    /// `left_out`: a refs CPK's layout.
    fn then_tree(entries: &[&str], left_out: &[&str]) -> Vec<String> {
        let tree = Templates::embedded()
            .referee_tree(Engine::Fox)
            .map(|(path, _)| path)
            .filter(|path| !left_out.contains(path))
            .collect::<Vec<_>>();
        entries
            .iter()
            .chain(&tree)
            .map(|path| (*path).to_owned())
            .collect()
    }

    /// The game path of the Fox referee template tree's `RefereeAppearance.bin`.
    const REFEREE_APPEARANCE: &str =
        "common/character0/model/character/appearance/RefereeAppearance.bin";

    /// `output` finished for PES 21 on the bundled bins with no team colors.
    fn finish_pes21(output: CpkOutput) -> (Written, Vec<Message>) {
        let (written, _entries, messages, _) = output
            .finish(
                PesVersion::Pes21,
                bundled(),
                BinsEdits::empty(),
                &Templates::embedded(),
            )
            .unwrap();
        (written, messages)
    }

    #[test]
    fn the_refs_export_s_batches_go_into_the_refs_cpk_and_a_team_s_into_the_team_cpk() {
        let temp = scratch("writer_refs_split");
        let folder = temp.path();
        let mut output = with_refs_cpk(folder, BTreeMap::new(), 1..3);
        for batch in [
            batch(0, &["team/a.bin"], None),
            batch(1, &["refs/a.bin"], None),
            batch(2, &["refs/b.bin"], None),
            batch(3, &["team/b.bin"], None),
        ] {
            output.submit(batch).unwrap();
        }

        let (written, messages) = finish_pes21(output);

        assert_eq!(
            written,
            Written {
                team: true,
                refs: true
            }
        );
        assert_eq!(messages, []);
        assert_eq!(
            layout(&folder.join("cup.cpk")),
            [
                "team/a.bin",
                "team/b.bin",
                paths::UNIFORM_PARAMETER,
                paths::TEAM_COLOR,
                paths::UNI_COLOR
            ]
        );
        // The tree follows the refs export's entries, with its embedded bytes.
        let refs = folder.join("refs.cpk");
        assert_eq!(layout(&refs), then_tree(&["refs/a.bin", "refs/b.bin"], &[]));
        for (path, bytes) in Templates::embedded().referee_tree(Engine::Fox) {
            assert!(entry(&refs, path) == bytes, "{path}");
        }
    }

    #[test]
    fn a_run_whose_only_commit_is_the_refs_export_s_writes_the_bins_for_the_referee_configs() {
        let temp = scratch("writer_refs_alone");
        let folder = temp.path();
        let mut output = with_refs_cpk(folder, BTreeMap::new(), 0..1);
        output.submit(batch(0, &["refs/a.bin"], None)).unwrap();
        // A team's failed task commits nothing.
        output.submit(batch(1, &[], None)).unwrap();

        let (written, messages) = finish_pes21(output);

        assert_eq!(
            written,
            Written {
                team: true,
                refs: true
            }
        );
        assert_eq!(messages, []);
        let refs = folder.join("refs.cpk");
        assert_eq!(layout(&refs), then_tree(&["refs/a.bin"], &[]));
        // The team side holds the bins alone, the referee configs' entries among them, and
        // nothing of the failed kit's.
        assert_eq!(
            layout(&folder.join("cup.cpk")),
            [
                paths::UNIFORM_PARAMETER,
                paths::TEAM_COLOR,
                paths::UNI_COLOR
            ]
        );
        let written: Vec<(&str, Vec<u8>)> = Templates::embedded()
            .referee_tree(Engine::Fox)
            .map(|(path, _)| (path, entry(&refs, path)))
            .collect();
        let bin = team_uniform_parameter(folder);
        assert_config_entries(&bin, &written);
        assert_eq!(bin.get("kit"), None);
    }

    #[test]
    fn on_pes_17_a_run_whose_only_commit_is_the_refs_export_s_writes_the_refs_cpk_alone() {
        let temp = scratch("writer_refs_alone_pes17");
        let folder = temp.path();
        let mut output = with_refs_cpk_for(folder, BTreeMap::new(), 0..1, Engine::PreFox);
        output.submit(batch(0, &["refs/a.bin"], None)).unwrap();

        let (written, _entries, messages, _) = output
            .finish(
                PesVersion::Pes17,
                WorkingBins::bundled(PesVersion::Pes17, &Templates::embedded()),
                BinsEdits::empty(),
                &Templates::embedded(),
            )
            .unwrap();

        // The pre-Fox referee configs are loose files alone: no bin to change.
        assert_eq!(
            written,
            Written {
                team: false,
                refs: true
            }
        );
        assert_eq!(messages, []);
        assert!(!folder.join("cup.cpk").exists(), "no team CPK");
    }

    #[test]
    fn a_refs_export_that_commits_nothing_writes_neither_the_refs_cpk_nor_the_tree() {
        let temp = scratch("writer_refs_nothing");
        let folder = temp.path();
        let mut output = with_refs_cpk(folder, BTreeMap::new(), 0..1);
        // The refs export's one task failed.
        output.submit(batch(0, &[], None)).unwrap();
        output.submit(batch(1, &["team/a.bin"], None)).unwrap();

        let (written, messages) = finish_pes21(output);

        assert_eq!(
            written,
            Written {
                team: true,
                refs: false
            }
        );
        assert_eq!(messages, []);
        assert!(!folder.join("refs.cpk").exists(), "no refs CPK");
        assert_eq!(
            layout(&folder.join("cup.cpk")),
            ["team/a.bin", paths::TEAM_COLOR, paths::UNI_COLOR],
            "no tree on the team side"
        );
    }

    #[test]
    fn an_override_at_a_tree_path_leaves_that_entry_out_of_the_refs_cpk_with_duplicate_path() {
        let temp = scratch("writer_refs_tree_override");
        let folder = temp.path();
        let overrides = overrides(folder, &[REFEREE_APPEARANCE]);
        let mut output = with_refs_cpk(folder, overrides, 0..1);
        output.submit(batch(0, &["refs/a.bin"], None)).unwrap();

        let (written, messages) = finish_pes21(output);

        assert_eq!(
            written,
            Written {
                team: true,
                refs: true
            }
        );
        assert_eq!(messages, [duplicate(REFEREE_APPEARANCE)]);
        assert_eq!(
            layout(&folder.join("refs.cpk")),
            then_tree(&["refs/a.bin"], &[REFEREE_APPEARANCE])
        );
        // The override goes into the team side, as every override does.
        let team = folder.join("cup.cpk");
        assert_eq!(
            layout(&team),
            [
                REFEREE_APPEARANCE,
                paths::UNIFORM_PARAMETER,
                paths::TEAM_COLOR,
                paths::UNI_COLOR
            ]
        );
        assert_eq!(
            entry(&team, REFEREE_APPEARANCE),
            override_bytes(REFEREE_APPEARANCE)
        );
    }

    #[test]
    fn a_refs_task_s_entry_at_a_tree_path_replaces_the_tree_s_file_with_no_finding() {
        let temp = scratch("writer_refs_tree_replaced");
        let folder = temp.path();
        let texture = "common/character1/model/character/uniform/common/999/refkit/texture.dds";
        let templates = Templates::embedded();
        let mut output = with_refs_cpk_for(folder, BTreeMap::new(), 0..1, Engine::PreFox);
        output
            .submit(TaskBatch {
                entries: vec![(texture.to_owned(), b"the member's refkit texture".to_vec())],
                ..batch(0, &[], None)
            })
            .unwrap();

        let (written, _entries, messages, _) = output
            .finish(
                PesVersion::Pes17,
                WorkingBins::bundled(PesVersion::Pes17, &templates),
                BinsEdits::empty(),
                &templates,
            )
            .unwrap();

        assert_eq!(
            written,
            Written {
                team: false,
                refs: true
            }
        );
        assert_eq!(messages, []);
        let refs = folder.join("refs.cpk");
        let layout = layout(&refs);
        assert_eq!(
            layout.len(),
            51,
            "the task's texture and the 50 other tree files"
        );
        assert_eq!(layout[0], texture, "the task's entry first");
        assert_eq!(entry(&refs, texture), b"the member's refkit texture");
        let mut others = 0;
        for (path, template) in templates.referee_tree(Engine::PreFox) {
            if path == texture {
                continue;
            }
            others += 1;
            assert!(entry(&refs, path) == template, "{path}");
        }
        assert_eq!(others, 50);
    }

    /// The refs CPK written into `folder` for `version` with `templates` when the refs export's
    /// one batch commits `entries`: each game path of the version's template tree with its
    /// bytes there.
    fn refs_tree(
        folder: &Path,
        version: PesVersion,
        templates: &Templates,
        entries: &[&str],
    ) -> Vec<(&'static str, Vec<u8>)> {
        let mut output = with_refs_cpk_for(folder, BTreeMap::new(), 0..1, version.engine());
        output.submit(batch(0, entries, None)).unwrap();
        output
            .finish(
                version,
                WorkingBins::bundled(version, templates),
                BinsEdits::empty(),
                templates,
            )
            .unwrap();
        let refs = folder.join("refs.cpk");
        templates
            .referee_tree(version.engine())
            .map(|(path, _)| (path, entry(&refs, path)))
            .collect()
    }

    /// The offsets at which `written` differs from `template`, both 120-byte kit configs.
    fn differing_offsets(written: &[u8], template: &[u8]) -> Vec<usize> {
        assert_eq!((written.len(), template.len()), (120, 120));
        (0..120)
            .filter(|at| written[*at] != template[*at])
            .collect()
    }

    #[test]
    fn with_the_marker_every_referee_kit_config_wears_its_collar_and_nothing_else_changes() {
        let temp = scratch("writer_refs_marker");
        let marker = paths::collar(Engine::Fox, REFEREE_MARKER_COLLAR);
        let templates = Templates::embedded();
        for version in [
            PesVersion::Pes18,
            PesVersion::Pes19,
            PesVersion::Pes20,
            PesVersion::Pes21,
        ] {
            let folder = temp.path().join(version.to_string());
            fs::create_dir_all(&folder).unwrap();
            let written = refs_tree(&folder, version, &templates, &[&marker]);

            let mut configs = 0;
            for ((path, written), (_, template)) in
                written.iter().zip(templates.referee_tree(Engine::Fox))
            {
                if !path.starts_with(paths::REFEREE_KIT_CONFIGS) {
                    assert!(written == template, "{version} {path}");
                    continue;
                }
                configs += 1;
                assert_eq!(template[0x14..0x16], [105, 105], "{path}");
                assert_eq!(
                    differing_offsets(written, template),
                    [0x14, 0x15],
                    "{version} {path}"
                );
                assert_eq!(written[0x14..0x16], [77, 77], "{version} {path}");
            }
            assert_eq!(configs, 20, "{version}");
        }
    }

    /// The `UniformParameter.bin` of the team CPK `cup.cpk` in `folder`.
    fn team_uniform_parameter(folder: &Path) -> UniformParameter {
        UniformParameter::read(&entry(&folder.join("cup.cpk"), paths::UNIFORM_PARAMETER)).unwrap()
    }

    /// Asserts that `bin` holds each of the 20 referee kit configs among `written` (game path,
    /// bytes) as the entry of its file name, with its bytes.
    fn assert_config_entries(bin: &UniformParameter, written: &[(&str, Vec<u8>)]) {
        let mut configs = 0;
        for (path, bytes) in written {
            let Some(name) = path.strip_prefix(paths::REFEREE_KIT_CONFIGS) else {
                continue;
            };
            configs += 1;
            assert_eq!(bin.get(name), Some(bytes.as_slice()), "{path}");
        }
        assert_eq!(configs, 20);
    }

    #[test]
    fn with_the_marker_every_referee_kit_config_is_also_its_entry_in_the_team_side_s_bin() {
        let temp = scratch("writer_refs_marker_entries");
        let marker = paths::collar(Engine::Fox, REFEREE_MARKER_COLLAR);
        let templates = Templates::embedded();
        for version in [
            PesVersion::Pes18,
            PesVersion::Pes19,
            PesVersion::Pes20,
            PesVersion::Pes21,
        ] {
            let folder = temp.path().join(version.to_string());
            fs::create_dir_all(&folder).unwrap();
            let written = refs_tree(&folder, version, &templates, &[&marker]);

            // No team batch went in: the team side is written for the bin alone.
            assert_eq!(
                layout(&folder.join("cup.cpk")),
                [
                    paths::UNIFORM_PARAMETER,
                    paths::TEAM_COLOR,
                    paths::UNI_COLOR
                ],
                "{version}"
            );
            let bin = team_uniform_parameter(&folder);
            assert_config_entries(&bin, &written);
            assert_eq!(
                bin.get("referee_DEF_1.bin").unwrap()[0x14..0x16],
                [77, 77],
                "{version}"
            );
        }
    }

    #[test]
    fn a_referee_kit_config_an_override_holds_gets_no_entry() {
        let temp = scratch("writer_refs_config_override");
        let folder = temp.path();
        let def_1 = "common/character0/model/character/uniform/team/referee/referee_DEF_1.bin";
        let overrides = overrides(folder, &[def_1]);
        let mut output = with_refs_cpk(folder, overrides, 0..1);
        let marker = paths::collar(Engine::Fox, REFEREE_MARKER_COLLAR);
        output.submit(batch(0, &[&marker], None)).unwrap();

        let (_, messages) = finish_pes21(output);

        assert_eq!(messages, [duplicate(def_1)]);
        let bin = team_uniform_parameter(folder);
        // The base's entry stays, collar 105, while the configs written wear the marker's.
        let base = bundled().uniform_parameter.unwrap();
        assert_eq!(bin.get("referee_DEF_1.bin"), base.get("referee_DEF_1.bin"));
        assert_eq!(
            bin.get("referee_DEF_1.bin").unwrap()[0x14..0x16],
            [105, 105]
        );
        assert_eq!(bin.get("referee_DEF_2.bin").unwrap()[0x14..0x16], [77, 77]);
    }

    #[test]
    fn the_pre_fox_marker_replaces_the_tree_s_prop_texture_and_the_configs_keep_collar_26() {
        let temp = scratch("writer_refs_marker_pes17");
        let folder = temp.path();
        let marker = paths::referee_marker(Engine::PreFox);
        let templates = Templates::embedded();
        let mut output = with_refs_cpk_for(folder, BTreeMap::new(), 0..1, Engine::PreFox);
        output
            .submit(TaskBatch {
                entries: vec![(marker.clone(), b"the member's marker".to_vec())],
                ..batch(0, &[], None)
            })
            .unwrap();

        output
            .finish(
                PesVersion::Pes17,
                WorkingBins::bundled(PesVersion::Pes17, &templates),
                BinsEdits::empty(),
                &templates,
            )
            .unwrap();

        let refs = folder.join("refs.cpk");
        let layout = layout(&refs);
        assert_eq!(layout.len(), 51, "the marker and the 50 other tree files");
        assert_eq!(layout[0], marker, "the marker first");
        assert_eq!(entry(&refs, &marker), b"the member's marker");
        let mut configs = 0;
        let mut others = 0;
        for (path, template) in templates.referee_tree(Engine::PreFox) {
            if path == marker {
                continue;
            }
            others += 1;
            assert!(entry(&refs, path) == template, "{path}");
            if path.starts_with(paths::REFEREE_KIT_CONFIGS) {
                configs += 1;
                // The pre-Fox templates wear collar 26, whose referee model the tree carries.
                assert_eq!(template[0x14..0x16], [26, 26], "{path}");
            }
        }
        assert_eq!((others, configs), (50, 20));
    }

    #[test]
    fn wearing_marker_sets_only_the_collars_of_a_pes_17_kit_config() {
        let def_1 = "common/character0/model/character/uniform/team/referee/referee_DEF_1.bin";
        let templates = Templates::embedded();
        let (_, template) = templates
            .referee_tree(Engine::PreFox)
            .find(|(path, _)| *path == def_1)
            .unwrap();
        assert_eq!(template.len(), 120);

        let written = wearing_marker(def_1, template, PesVersion::Pes17).unwrap();

        assert_eq!(written.len(), 120);
        let config = KitConfig::decode(&written, PesVersion::Pes17).unwrap();
        assert_eq!((config.shirt.collar, config.shirt.winter_collar), (77, 77));
        let mut expected = KitConfig::decode(template, PesVersion::Pes17).unwrap();
        assert_ne!(expected.shirt.collar, 77, "the template's own collar");
        expected.shirt.collar = 77;
        expected.shirt.winter_collar = 77;
        assert_eq!(config, expected, "nothing else changed");
    }

    #[test]
    fn without_the_marker_every_referee_kit_config_is_the_template_s() {
        let temp = scratch("writer_refs_no_marker");
        let templates = Templates::embedded();
        let written = refs_tree(temp.path(), PesVersion::Pes21, &templates, &["refs/a.bin"]);

        for ((path, written), (_, template)) in
            written.iter().zip(templates.referee_tree(Engine::Fox))
        {
            assert!(written == template, "{path}");
        }
    }

    #[test]
    fn a_replaced_referee_kit_config_keeps_its_own_bytes_and_wears_the_marker_s_collar() {
        let temp = scratch("writer_refs_marker_replaced");
        let def_1 = "common/character0/model/character/uniform/team/referee/referee_DEF_1.bin";
        let embedded = Templates::embedded()
            .referee_tree(Engine::Fox)
            .find(|(path, _)| *path == def_1)
            .map(|(_, bytes)| bytes.to_vec())
            .unwrap();
        // A valid config other than the template: one byte of its colors changed.
        let mut replacement = embedded.clone();
        replacement[0x10] ^= 0x01;
        let data = temp.path().join("data");
        let file = data.join("templates/referees_fox").join(def_1);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, &replacement).unwrap();
        let (templates, _) = Templates::read(Some(&data)).unwrap();
        let marker = paths::collar(Engine::Fox, REFEREE_MARKER_COLLAR);
        let output = temp.path().join("output");
        fs::create_dir_all(&output).unwrap();

        let written = refs_tree(&output, PesVersion::Pes21, &templates, &[&marker]);

        let (_, written) = written.iter().find(|(path, _)| *path == def_1).unwrap();
        assert_eq!(differing_offsets(written, &replacement), [0x14, 0x15]);
        assert_eq!(written[0x14..0x16], [77, 77]);
        assert_ne!(written[0x10], embedded[0x10], "the replacement's own byte");
    }

    #[test]
    fn a_referee_kit_config_that_does_not_decode_is_the_error_naming_it_with_the_marker() {
        let temp = scratch("writer_refs_marker_broken");
        let def_1 = "common/character0/model/character/uniform/team/referee/referee_DEF_1.bin";
        let data = temp.path().join("data");
        let file = data.join("templates/referees_fox").join(def_1);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"not a kit config").unwrap();
        let (templates, _) = Templates::read(Some(&data)).unwrap();
        let mut output = with_refs_cpk(temp.path(), BTreeMap::new(), 0..1);
        output
            .submit(batch(
                0,
                &[&paths::collar(Engine::Fox, REFEREE_MARKER_COLLAR)],
                None,
            ))
            .unwrap();

        let error = output
            .finish(PesVersion::Pes21, bundled(), BinsEdits::empty(), &templates)
            .expect_err("an undecodable config with the marker");

        assert!(
            format!("{error:#}").starts_with(&format!("{def_1} is not a kit config")),
            "{error:#}"
        );
    }

    #[test]
    fn referees_without_a_refs_cpk_bring_the_tree_into_the_sink_when_one_of_theirs_committed() {
        let temp = scratch("writer_refs_in_sink");
        let folder = temp.path();
        let in_sink = |name: &str| {
            CpkOutput::new(
                OutputSink::cpk(folder.join(name)),
                BTreeMap::new(),
                "",
                None,
            )
            .with_referees(Referees::in_sink(0..1, Engine::Fox))
        };

        let mut output = in_sink("committed.cpk");
        output.submit(batch(0, &["refs/a.bin"], None)).unwrap();
        let (written, messages) = finish_pes21(output);
        assert_eq!(
            written,
            Written {
                team: true,
                refs: false
            }
        );
        assert_eq!(messages, []);
        let mut expected = then_tree(&["refs/a.bin"], &[]);
        expected.extend([
            paths::UNIFORM_PARAMETER.to_owned(),
            paths::TEAM_COLOR.to_owned(),
            paths::UNI_COLOR.to_owned(),
        ]);
        let committed = folder.join("committed.cpk");
        assert_eq!(layout(&committed), expected);
        let written: Vec<(&str, Vec<u8>)> = Templates::embedded()
            .referee_tree(Engine::Fox)
            .map(|(path, _)| (path, entry(&committed, path)))
            .collect();
        let bin = UniformParameter::read(&entry(&committed, paths::UNIFORM_PARAMETER)).unwrap();
        assert_config_entries(&bin, &written);

        // The refs export's task failed: no tree, though a team's entry went in.
        let mut output = in_sink("failed.cpk");
        output.submit(batch(0, &[], None)).unwrap();
        output.submit(batch(1, &["team/a.bin"], None)).unwrap();
        let (written, _) = finish_pes21(output);
        assert!(written.team);
        assert_eq!(
            layout(&folder.join("failed.cpk")),
            ["team/a.bin", paths::TEAM_COLOR, paths::UNI_COLOR]
        );
    }

    #[test]
    fn an_override_at_a_refs_entry_s_path_leaves_it_out_and_goes_into_the_team_cpk() {
        let temp = scratch("writer_refs_override");
        let folder = temp.path();
        let overrides = overrides(folder, &["refs/over.bin"]);
        let mut output = with_refs_cpk(folder, overrides, 0..1);

        let committed = output
            .submit(batch(0, &["refs/over.bin", "refs/own.bin"], None))
            .unwrap();

        assert_eq!(committed, [(0, vec![note(0), duplicate("refs/over.bin")])]);
        let (written, _) = finish_pes21(output);
        assert_eq!(
            written,
            Written {
                team: true,
                refs: true
            }
        );
        assert_eq!(
            layout(&folder.join("refs.cpk")),
            then_tree(&["refs/own.bin"], &[])
        );
        let team = folder.join("cup.cpk");
        assert_eq!(
            layout(&team),
            [
                "refs/over.bin",
                paths::UNIFORM_PARAMETER,
                paths::TEAM_COLOR,
                paths::UNI_COLOR
            ]
        );
        assert_eq!(
            entry(&team, "refs/over.bin"),
            override_bytes("refs/over.bin")
        );
    }

    #[test]
    fn an_override_that_cannot_be_read_fails_the_cpk_naming_the_file() {
        let temp = scratch("writer_override_gone");
        let folder = temp.path();
        let gone = overrides(folder, &["a/over.bin"]);
        let file = gone["a/over.bin"].clone();
        fs::remove_file(&file).unwrap();
        let expected = format!("cannot read the override {}", file.display());

        // At the first entry a task commits.
        let mut output = CpkOutput::new(
            OutputSink::cpk(folder.join("first.cpk")),
            gone.clone(),
            "",
            None,
        );
        let error = output.submit(batch(0, &["b/own.bin"], None)).unwrap_err();
        assert_eq!(error.to_string(), expected);

        // At the end, when no task committed anything.
        let output = CpkOutput::new(OutputSink::cpk(folder.join("last.cpk")), gone, "", None);
        let error = finish_plain(output, PesVersion::Pes21).unwrap_err();
        assert_eq!(error.to_string(), expected);
    }

    #[test]
    fn batches_are_laid_out_in_manifest_order_and_the_bins_last() {
        let temp = scratch("writer_order");
        let folder = temp.path();
        let path = folder.join("run/cup.cpk");
        let mut output = CpkOutput::new(OutputSink::cpk(path.clone()), BTreeMap::new(), "", None);

        output.submit(batch(2, &["a/first.bin"], None)).unwrap();
        output
            .submit(batch(1, &["z/second.bin"], Some("kit")))
            .unwrap();
        assert!(!path.exists(), "nothing is written before task 0 arrives");
        output
            .submit(batch(0, &["m/third.bin", "b/fourth.bin"], None))
            .unwrap();

        assert!(finish_plain(output, PesVersion::Pes21).unwrap());
        assert_eq!(
            layout(&path),
            [
                "m/third.bin",
                "b/fourth.bin",
                "z/second.bin",
                "a/first.bin",
                paths::UNIFORM_PARAMETER,
                paths::TEAM_COLOR,
                paths::UNI_COLOR,
            ]
        );
        let mut archive = CpkArchive::open(File::open(&path).unwrap()).unwrap();
        let entry = archive
            .entries()
            .iter()
            .find(|entry| entry.path == paths::UNIFORM_PARAMETER)
            .unwrap()
            .clone();
        let bin = UniformParameter::read(&archive.read(&entry).unwrap()).unwrap();
        assert_eq!(bin.get("kit"), Some(&[7; 120][..]));
        let base = UniformParameter::read(
            Templates::embedded()
                .uniform_parameter_base(PesVersion::Pes21)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(bin.len(), base.len() + 1, "the base's entries are kept");
    }

    #[test]
    fn uniform_parameter_bin_is_built_on_the_working_one() {
        let temp = scratch("writer_working_uniform_parameter");
        let path = temp.path().join("cup.cpk");
        let mut output = CpkOutput::new(OutputSink::cpk(path.clone()), BTreeMap::new(), "", None);
        output.submit(batch(0, &["a/b.bin"], Some("kit"))).unwrap();
        // PES 18's base stands in for an installed bin: it is not the one a PES 21 run bundles.
        let templates = Templates::embedded();
        let working = templates.uniform_parameter_base(PesVersion::Pes18).unwrap();
        let bins = WorkingBins {
            uniform_parameter: Some(UniformParameter::read(working).unwrap()),
            ..bundled()
        };

        let (written, _entries, _, _) = output
            .finish(
                PesVersion::Pes21,
                bins,
                BinsEdits::empty(),
                &Templates::embedded(),
            )
            .unwrap();

        assert!(written.team);
        let bin = UniformParameter::read(&entry(&path, paths::UNIFORM_PARAMETER)).unwrap();
        let mut expected = UniformParameter::read(working).unwrap();
        expected.insert("kit".to_owned(), vec![7; 120]).unwrap();
        assert!(
            bin.write() == expected.write(),
            "the working bin with the kit's config added"
        );
    }

    #[test]
    fn the_arrival_order_changes_no_byte_of_the_cpk() {
        let temp = scratch("writer_arrival");
        let folder = temp.path();
        let mut written = Vec::new();
        for (name, reversed) in [("forward", false), ("reversed", true)] {
            let path = folder.join(format!("{name}.cpk"));
            let mut output =
                CpkOutput::new(OutputSink::cpk(path.clone()), BTreeMap::new(), "", None);
            let mut batches = three_batches();
            if reversed {
                batches.reverse();
            }
            for batch in batches {
                output.submit(batch).unwrap();
            }
            assert!(finish_plain(output, PesVersion::Pes21).unwrap());
            written.push(fs::read(&path).unwrap());
        }
        assert_eq!(written[0], written[1]);
    }

    #[test]
    fn submit_returns_each_committed_batch_and_its_messages_in_manifest_order() {
        let temp = scratch("writer_committed");
        let folder = temp.path();
        let mut output = CpkOutput::new(
            OutputSink::cpk(folder.join("cup.cpk")),
            BTreeMap::new(),
            "",
            None,
        );
        let [first, second, third] = three_batches();

        assert_eq!(output.submit(third).unwrap(), [], "task 2 waits for 0");
        assert_eq!(output.submit(first).unwrap(), [(0, vec![note(0)])]);
        assert_eq!(
            output.submit(second).unwrap(),
            [(1, vec![note(1)]), (2, vec![note(2)])]
        );
    }

    #[test]
    fn a_failed_commit_releases_the_permits_of_the_batches_waiting_behind_it() {
        let temp = scratch("writer_failed_commit");
        let folder = temp.path();
        let mut output = CpkOutput::new(
            OutputSink::cpk(folder.join("cup.cpk")),
            BTreeMap::new(),
            "",
            None,
        );
        let budget = MemoryBudget::new(1);
        let mut waiting = batch(1, &["a/waiting.bin"], None);
        waiting.permit = Some(Arc::new(budget.acquire(1).unwrap()));
        output.submit(waiting).unwrap();

        // Two entries with one path: the CPK refuses the second.
        let error = output
            .submit(batch(0, &["a/twice.bin", "a/twice.bin"], None))
            .unwrap_err();
        assert!(
            error.to_string().contains("cannot add a/twice.bin"),
            "{error}"
        );

        let (admitted_tx, admitted) = mpsc::channel();
        thread::spawn(move || admitted_tx.send(budget.acquire(1).is_ok()).unwrap());
        assert_eq!(
            admitted.recv_timeout(Duration::from_secs(5)),
            Ok(true),
            "the waiting batch's permit was released"
        );
    }

    #[test]
    fn a_failed_task_contributes_nothing_not_even_its_kit_config() {
        let temp = scratch("writer_failed");
        let folder = temp.path();
        let path = folder.join("run/cup.cpk");
        let mut output = CpkOutput::new(OutputSink::cpk(path.clone()), BTreeMap::new(), "", None);

        output.submit(batch(0, &[], None)).unwrap();

        assert!(!finish_plain(output, PesVersion::Pes21).unwrap());
        assert!(!folder.join("run").exists(), "no file and no folder");
    }

    #[test]
    fn a_batch_that_never_arrives_is_an_error() {
        let temp = scratch("writer_gap");
        let mut output = CpkOutput::new(
            OutputSink::cpk(temp.path().join("cup.cpk")),
            BTreeMap::new(),
            "",
            None,
        );
        output.submit(batch(1, &["a/b.bin"], None)).unwrap();
        let error = finish_plain(output, PesVersion::Pes21).unwrap_err();
        assert_eq!(
            error.to_string(),
            "the writer never received task 0 of the manifest"
        );
    }

    #[test]
    fn a_kit_config_for_a_version_without_the_bin_is_an_error() {
        let temp = scratch("writer_pre_fox");
        let mut output = CpkOutput::new(
            OutputSink::cpk(temp.path().join("cup.cpk")),
            BTreeMap::new(),
            "",
            None,
        );
        output.submit(batch(0, &["a/b.bin"], Some("kit"))).unwrap();
        let error = finish_plain(output, PesVersion::Pes17).unwrap_err();
        assert_eq!(error.to_string(), "PES 2017 has no UniformParameter.bin");
    }

    /// The start of team `team_id`'s `TeamColor.bin` record.
    fn team_record(team_id: usize) -> usize {
        (team_id - 100) * 16
    }

    /// Team 714's two colors, as its root `colors.txt` gives them.
    fn team_714_colors() -> Vec<(u16, Vec<Rgb>)> {
        vec![(714, vec![[0xc1, 0x12, 0x00], [0x41, 0x41, 0x41]])]
    }

    /// A run committing one entry, finished on `bins` with team 714's colors: the CPK's
    /// `TeamColor.bin` and the findings `finish` returned.
    fn team_color_run(name: &str, bins: WorkingBins) -> (Vec<u8>, Vec<Message>) {
        let temp = scratch(name);
        let path = temp.path().join("cup.cpk");
        let mut output = CpkOutput::new(OutputSink::cpk(path.clone()), BTreeMap::new(), "", None);
        output.submit(batch(0, &["a/b.bin"], None)).unwrap();
        let (written, _entries, messages, _) = output
            .finish(
                PesVersion::Pes21,
                bins,
                BinsEdits {
                    team_colors: &team_714_colors(),
                    ..BinsEdits::empty()
                },
                &Templates::embedded(),
            )
            .unwrap();
        assert!(written.team);
        let mut archive = CpkArchive::open(File::open(&path).unwrap()).unwrap();
        let entry = archive
            .entries()
            .iter()
            .find(|entry| entry.path == paths::TEAM_COLOR)
            .unwrap()
            .clone();
        (archive.read(&entry).unwrap(), messages)
    }

    #[test]
    fn team_color_bin_is_the_working_bin_with_each_team_s_colors_set() {
        let base = bundled_team_color();
        let (bin, messages) = team_color_run("writer_team_colors", bundled());
        let mut expected = base;
        let record = team_record(714);
        expected[record + 4..record + 10].copy_from_slice(&[0xc1, 0x12, 0x00, 0x41, 0x41, 0x41]);
        assert!(bin == expected, "only team 714's first two colors differ");
        assert_eq!(messages, []);
    }

    #[test]
    fn a_working_bin_s_broken_header_is_repaired_and_reported() {
        let mut broken = bundled_team_color();
        let record = team_record(799);
        broken[record..record + 4].copy_from_slice(&[0xc1, 0x12, 0x00, 0x41]);
        let (bin, messages) = team_color_run(
            "writer_team_color_repaired",
            WorkingBins {
                team_color: TeamColorBin::read(broken.clone()).unwrap(),
                ..bundled()
            },
        );
        assert_eq!(bin[record..record + 4], [0x1f, 0x03, 0x04, 0x00]);
        // Team 799's record keeps its colors, team 714's gets its own, and every other
        // record is the working bin's.
        let mut expected = broken;
        expected[record..record + 4].copy_from_slice(&[0x1f, 0x03, 0x04, 0x00]);
        let ours = team_record(714);
        expected[ours + 4..ours + 10].copy_from_slice(&[0xc1, 0x12, 0x00, 0x41, 0x41, 0x41]);
        assert!(bin == expected, "only 799's header and 714's colors differ");
        let [message] = messages.as_slice() else {
            panic!("{messages:?}");
        };
        assert_eq!(message.code.code, "bin_header_repaired");
        assert_eq!(
            (message.severity, message.disposition, &message.scope),
            (
                studio_core::Severity::Warning,
                Disposition::Keep,
                &Scope::Run
            )
        );
        assert_eq!(
            message.context,
            [
                ("bin".to_owned(), "TeamColor.bin".to_owned()),
                ("teams".to_owned(), "799".to_owned())
            ]
        );
    }

    #[test]
    fn a_run_that_commits_nothing_builds_no_bin_and_reports_nothing() {
        let temp = scratch("writer_no_entry_no_colors");
        let folder = temp.path().join("run");
        let mut output = CpkOutput::new(
            OutputSink::cpk(folder.join("cup.cpk")),
            BTreeMap::new(),
            "",
            None,
        );
        output.submit(batch(0, &[], None)).unwrap();
        let mut team_color = bundled_team_color();
        team_color[..4].copy_from_slice(&[1, 2, 3, 4]);
        let broken = WorkingBins {
            team_color: TeamColorBin::read(team_color).unwrap(),
            ..bundled()
        };
        let finished = output
            .finish(
                PesVersion::Pes21,
                broken,
                BinsEdits::empty(),
                &Templates::embedded(),
            )
            .unwrap();
        let nothing = Written {
            team: false,
            refs: false,
        };
        assert_eq!(finished, (nothing, 0, Vec::new(), BTreeSet::new()));
        assert!(!folder.exists(), "no file and no folder");
    }

    /// The patch is the run's work as the tables are: a run that committed nothing but
    /// planned a patch team still writes the CPK for it to publish beside, the bins' own
    /// work as empty.
    #[test]
    fn a_run_that_commits_nothing_but_planned_a_patch_still_writes_the_cpk() {
        let temp = scratch("writer_no_entry_patch");
        let folder = temp.path().join("run");
        let mut output = CpkOutput::new(
            OutputSink::cpk(folder.join("cup.cpk")),
            BTreeMap::new(),
            "",
            None,
        );
        output.submit(batch(0, &[], None)).unwrap();
        let finished = output
            .finish(
                PesVersion::Pes17,
                WorkingBins::bundled(PesVersion::Pes17, &Templates::embedded()),
                BinsEdits {
                    patch_planned: true,
                    ..BinsEdits::empty()
                },
                &Templates::embedded(),
            )
            .unwrap();
        assert_eq!(
            finished,
            (
                Written {
                    team: true,
                    refs: false
                },
                2,
                Vec::new(),
                BTreeSet::new()
            )
        );
        assert!(folder.join("cup.cpk").is_file(), "the CPK is written");
    }

    #[test]
    fn a_run_whose_only_work_is_the_root_colors_writes_the_bins() {
        let temp = scratch("writer_no_entry_colors");
        let folder = temp.path().join("run");
        let mut output = CpkOutput::new(
            OutputSink::cpk(folder.join("cup.cpk")),
            BTreeMap::new(),
            "",
            None,
        );
        output.submit(batch(0, &[], None)).unwrap();
        let mut team_color = bundled_team_color();
        team_color[..4].copy_from_slice(&[1, 2, 3, 4]);
        let broken = WorkingBins {
            team_color: TeamColorBin::read(team_color.clone()).unwrap(),
            ..bundled()
        };
        let (written, _entries, messages, _) = output
            .finish(
                PesVersion::Pes21,
                broken,
                BinsEdits {
                    team_colors: &team_714_colors(),
                    ..BinsEdits::empty()
                },
                &Templates::embedded(),
            )
            .unwrap();

        assert!(written.team);
        let emitted = entry(&folder.join("cup.cpk"), paths::TEAM_COLOR);
        let record = team_record(714);
        assert_eq!(
            emitted[record + 4..record + 10],
            [0xc1, 0x12, 0x00, 0x41, 0x41, 0x41]
        );
        // The base's record 100 kept its colors; its broken header was repaired and reported.
        assert_eq!(emitted[..4], [0x64, 0x00, 0x04, 0x00]);
        assert_eq!(emitted[4..16], team_color[4..16]);
        let [message] = messages.as_slice() else {
            panic!("{messages:?}");
        };
        assert_eq!(message.code.code, "bin_header_repaired");
    }

    /// The start of team `team_id`'s `UniColor.bin` record.
    fn kit_record(team_id: usize) -> usize {
        (team_id - 100) * 85
    }

    /// The tracer's goalkeeper kit's entry: icon 11, `#c11200` and `#414141`.
    fn tracer_g1() -> KitColorEntry {
        KitColorEntry {
            kit: 0x10,
            icon: 11,
            colors: [[0xc1, 0x12, 0x00], [0x41, 0x41, 0x41]],
        }
    }

    /// The kit batch at `index` of team 792's goalkeeper kit, its entries `paths` (empty for a
    /// failed task).
    fn kit_batch(index: usize, paths: &[&str]) -> TaskBatch {
        TaskBatch {
            uni_color: Some((792, tracer_g1())),
            ..batch(index, paths, Some("kit"))
        }
    }

    /// `batches` submitted in order and finished for PES 21 on `bins` with no team colors: the
    /// CPK's `UniColor.bin` and the findings `finish` returned.
    fn uni_color_run(
        name: &str,
        batches: Vec<TaskBatch>,
        bins: WorkingBins,
    ) -> (Vec<u8>, Vec<Message>) {
        let temp = scratch(name);
        let path = temp.path().join("cup.cpk");
        let mut output = CpkOutput::new(OutputSink::cpk(path.clone()), BTreeMap::new(), "", None);
        for batch in batches {
            output.submit(batch).unwrap();
        }
        let (written, _entries, messages, _) = output
            .finish(
                PesVersion::Pes21,
                bins,
                BinsEdits::empty(),
                &Templates::embedded(),
            )
            .unwrap();
        assert!(written.team);
        let mut archive = CpkArchive::open(File::open(&path).unwrap()).unwrap();
        let entry = archive
            .entries()
            .iter()
            .find(|entry| entry.path == paths::UNI_COLOR)
            .unwrap()
            .clone();
        (archive.read(&entry).unwrap(), messages)
    }

    #[test]
    fn uni_color_bin_is_the_working_bin_with_each_committed_kit_s_entry_merged() {
        let (bin, messages) = uni_color_run(
            "writer_uni_color",
            vec![kit_batch(0, &["kit/kit.ftex"])],
            bundled(),
        );

        assert_eq!(messages, []);
        // Team 792's placeholder record now holds the one kit, then nine unused entries.
        let mut record = vec![0x18, 0x03, 0x00, 0x00, 0x01];
        record.extend([0x10, 0x0b, 0xc1, 0x12, 0x00, 0x41, 0x41, 0x41]);
        for _ in 0..9 {
            record.extend([0xff, 0, 0, 0, 0, 0, 0, 0]);
        }
        let mut expected = bundled_uni_color();
        let start = kit_record(792);
        expected[start..start + 85].copy_from_slice(&record);
        assert!(bin == expected, "only team 792's record differs");
    }

    #[test]
    fn a_failed_kit_task_leaves_uni_color_bin_as_the_working_bin() {
        let (bin, messages) = uni_color_run(
            "writer_uni_color_failed",
            vec![batch(0, &[], None), batch(1, &["a/b.bin"], None)],
            bundled(),
        );

        assert_eq!(messages, []);
        assert!(
            bin == bundled_uni_color(),
            "the failed kit's entry is not applied"
        );
    }

    #[test]
    fn each_working_color_bin_s_broken_header_is_reported_teamcolor_first() {
        let mut team_color = bundled_team_color();
        let team = team_record(799);
        team_color[team..team + 4].copy_from_slice(&[0xc1, 0x12, 0x00, 0x41]);
        let mut uni_color = bundled_uni_color();
        let kits = kit_record(799);
        uni_color[kits..kits + 4].copy_from_slice(&[0xc1, 0x12, 0x00, 0x41]);
        let bins = WorkingBins {
            team_color: TeamColorBin::read(team_color).unwrap(),
            uni_color: UniColorBin::read(uni_color).unwrap(),
            ..bundled()
        };

        let (bin, messages) = uni_color_run(
            "writer_uni_color_repaired",
            vec![batch(0, &["a/b.bin"], None)],
            bins,
        );

        assert_eq!(bin[kits..kits + 4], [0x1f, 0x03, 0x00, 0x00]);
        let contexts: Vec<(&str, &[(String, String)])> = messages
            .iter()
            .map(|message| (message.code.code.as_ref(), message.context.as_slice()))
            .collect();
        assert_eq!(
            contexts,
            [
                (
                    "bin_header_repaired",
                    &[
                        ("bin".to_owned(), "TeamColor.bin".to_owned()),
                        ("teams".to_owned(), "799".to_owned())
                    ][..]
                ),
                (
                    "bin_header_repaired",
                    &[
                        ("bin".to_owned(), "UniColor.bin".to_owned()),
                        ("teams".to_owned(), "799".to_owned())
                    ][..]
                ),
            ]
        );
        assert_eq!(
            (
                messages[1].severity,
                messages[1].disposition,
                &messages[1].scope
            ),
            (
                studio_core::Severity::Warning,
                Disposition::Keep,
                &Scope::Run
            )
        );
    }

    /// One kit batch committed (or not, when `commits` is unset), finished for PES 21 on the
    /// bundled bins with team 714's `Midcup` export of no kit, its status `fpc`, a working
    /// `UniformParameter.bin` holding only 714's p1 config, lacking the FPC values, and 714's
    /// working `UniColor.bin` record holding kit 0 (p1) only: the CPK's layout, its
    /// `UniformParameter.bin` when it holds one, and the findings.
    fn finish_with(
        name: &str,
        commits: bool,
        fpc: EffectiveTeamKitFpc,
    ) -> (Vec<String>, Option<Vec<u8>>, Vec<Message>) {
        let mut config = KitConfig::template();
        config.shirt.model = 144;
        assert!(!matches_fpc(&config));
        let mut uniform_parameter = UniformParameter::new();
        uniform_parameter
            .insert(
                "714_DEF_1st_realUni.bin".to_owned(),
                config.encode(PesVersion::Pes21).to_vec(),
            )
            .unwrap();
        // Count 1, kit 0 with icon 3, then nine unused entries.
        let mut record = vec![0xca, 0x02, 0x00, 0x00, 1, 0, 3, 0, 0, 0, 0, 0, 0];
        for _ in 1..10 {
            record.extend([0xff, 0, 0, 0, 0, 0, 0, 0]);
        }
        let mut uni_color = bundled_uni_color();
        let start = kit_record(714);
        uni_color[start..start + 85].copy_from_slice(&record);
        let team = TeamKits {
            export_id: ExportId(3),
            team_id: 714,
            coverage: ExportCoverage::Midcup,
            edits: TeamKitEdits { fpc, collar: None },
            slots: Vec::new(),
            configless: Vec::new(),
        };

        let temp = scratch(name);
        let path = temp.path().join("cup.cpk");
        let mut output = CpkOutput::new(OutputSink::cpk(path.clone()), BTreeMap::new(), "", None);
        let paths: &[&str] = if commits { &["a/b.bin"] } else { &[] };
        output.submit(batch(0, paths, None)).unwrap();
        let bins = WorkingBins {
            uniform_parameter: Some(uniform_parameter),
            uni_color: UniColorBin::read(uni_color).unwrap(),
            ..bundled()
        };
        let (written, _entries, messages, _) = output
            .finish(
                PesVersion::Pes21,
                bins,
                BinsEdits {
                    team_kits: &[team],
                    ..BinsEdits::empty()
                },
                &Templates::embedded(),
            )
            .unwrap();
        if !written.team {
            return (Vec::new(), None, messages);
        }
        let layout = layout(&path);
        let bin = layout
            .contains(&paths::UNIFORM_PARAMETER.to_owned())
            .then(|| entry(&path, paths::UNIFORM_PARAMETER));
        (layout, bin, messages)
    }

    #[test]
    fn a_patched_config_alone_writes_uniform_parameter_bin_first_of_the_bins() {
        let (layout, bin, messages) =
            finish_with("writer_fpc_patched", true, EffectiveTeamKitFpc::On);

        assert_eq!(
            layout,
            [
                "a/b.bin",
                paths::UNIFORM_PARAMETER,
                paths::TEAM_COLOR,
                paths::UNI_COLOR
            ]
        );
        let bin = UniformParameter::read(&bin.unwrap()).unwrap();
        let config = KitConfig::decode(
            bin.get("714_DEF_1st_realUni.bin").unwrap(),
            PesVersion::Pes21,
        )
        .unwrap();
        assert!(matches_fpc(&config));
        assert_eq!(
            messages,
            [tool_message(
                Code::KitConfigFpcAdjusted,
                Scope::Export {
                    export_id: ExportId(3)
                },
                Disposition::Keep,
                vec![("slot", "p1".to_owned())],
            )]
        );
    }

    #[test]
    fn a_bin_nothing_changed_is_not_written_and_a_run_that_commits_nothing_reports_nothing() {
        // Status unknown: nothing to patch, and no kit committed.
        let (layout, bin, messages) =
            finish_with("writer_fpc_unchanged", true, EffectiveTeamKitFpc::Unknown);
        assert_eq!(layout, ["a/b.bin", paths::TEAM_COLOR, paths::UNI_COLOR]);
        assert_eq!((bin, messages), (None, Vec::new()));

        // A config there is to patch, but no CPK is written.
        let (layout, bin, messages) =
            finish_with("writer_fpc_no_cpk", false, EffectiveTeamKitFpc::On);
        assert_eq!((layout, bin, messages), (Vec::new(), None, Vec::new()));
    }

    /// A multi-CPK writer in `folder/run`: the bins CPK `4cc_08_bins` as the sink after
    /// `overrides`, and the parts of the slots `4cc_41_teams` to `4cc_43_teams` under `cap`,
    /// each export of `ends` (last manifest position, name) a team.
    fn parts_output(
        folder: &Path,
        ends: &[(usize, &str)],
        cap: u64,
        overrides: BTreeMap<String, PathBuf>,
    ) -> CpkOutput {
        let run = folder.join("run");
        let slots = ["4cc_41_teams", "4cc_42_teams", "4cc_43_teams"]
            .map(|slot| pipeline::CpkStem::new(slot).unwrap())
            .to_vec();
        let ends = ends
            .iter()
            .map(|(end, export)| (*end, (*export).to_owned()))
            .collect();
        let parts = TeamsParts::new(
            run.clone(),
            slots,
            "teams",
            cap,
            Templates::embedded().placeholder_cpk().to_vec(),
            ends,
        );
        CpkOutput::new(
            OutputSink::cpk(run.join("4cc_08_bins.cpk")),
            overrides,
            "",
            Some(parts),
        )
    }

    /// The length a part holding `paths`, each entry its own path's bytes as `batch` writes
    /// it, would have.
    fn part_len(folder: &Path, paths: &[&str]) -> u64 {
        crate::output::sink::create_cpk(&folder.join("probe/probe.cpk"))
            .unwrap()
            .len_with(paths.iter().map(|path| (*path, path.len() as u64)))
    }

    #[test]
    fn with_parts_a_team_s_batches_out_of_order_land_after_the_earlier_team_s_and_the_bins_apart() {
        let temp = scratch("writer_parts_order");
        let folder = temp.path();
        let mut output = parts_output(
            folder,
            &[(0, "co Full Spring"), (2, "a Full Spring")],
            1 << 20,
            BTreeMap::new(),
        );

        output
            .submit(batch(2, &["kit/u0702p1.ftex"], None))
            .unwrap();
        output
            .submit(batch(0, &["kit/u0714p1.ftex"], None))
            .unwrap();
        output
            .submit(batch(1, &["kit/u0702g1.ftex"], Some("kit")))
            .unwrap();
        assert!(finish_plain(output, PesVersion::Pes21).unwrap());

        let run = folder.join("run");
        assert_eq!(
            layout(&run.join("4cc_41_teams.cpk")),
            ["kit/u0714p1.ftex", "kit/u0702g1.ftex", "kit/u0702p1.ftex"]
        );
        assert_eq!(
            layout(&run.join("4cc_08_bins.cpk")),
            [
                paths::UNIFORM_PARAMETER,
                paths::TEAM_COLOR,
                paths::UNI_COLOR
            ]
        );
        for slot in ["4cc_42_teams.cpk", "4cc_43_teams.cpk"] {
            assert!(
                fs::read(run.join(slot)).unwrap() == Templates::embedded().placeholder_cpk(),
                "{slot}"
            );
        }
    }

    #[test]
    fn with_parts_a_team_whose_last_task_failed_is_still_placed_before_the_next() {
        let temp = scratch("writer_parts_last_failed");
        let folder = temp.path();
        // One team per part: /co/'s entry and /a/'s together do not fit.
        let cap = part_len(folder, &["kit/u0714p1.ftex"]);
        let mut output = parts_output(
            folder,
            &[(1, "co Full Spring"), (2, "a Full Spring")],
            cap,
            BTreeMap::new(),
        );

        output
            .submit(batch(0, &["kit/u0714p1.ftex"], None))
            .unwrap();
        // /co/'s last task failed: it committed nothing.
        output.submit(batch(1, &[], None)).unwrap();
        output
            .submit(batch(2, &["kit/u0702p1.ftex"], None))
            .unwrap();
        assert!(finish_plain(output, PesVersion::Pes21).unwrap());

        let run = folder.join("run");
        assert_eq!(layout(&run.join("4cc_41_teams.cpk")), ["kit/u0714p1.ftex"]);
        assert_eq!(layout(&run.join("4cc_42_teams.cpk")), ["kit/u0702p1.ftex"]);
    }

    #[test]
    fn with_parts_a_held_batch_releases_its_permit_when_it_is_held() {
        let temp = scratch("writer_parts_permit");
        let folder = temp.path();
        let mut output = parts_output(folder, &[(1, "co Full Spring")], 1 << 20, BTreeMap::new());
        let budget = MemoryBudget::new(1);
        let mut held = batch(0, &["kit/u0714p1.ftex"], None);
        held.permit = Some(Arc::new(budget.acquire(1).unwrap()));

        // Committed, and held: the team's last task, 1, is still to come. `commit` itself,
        // as below: `submit` drops the batch before it returns, releasing the permit anyway.
        output.commit(&mut held).unwrap();

        assert!(drained(budget), "the held batch's permit was released");
        drop(held);
        drop(output);
    }

    /// Whether `budget`, of cap 1, admits a request of 1 byte within the guard: whether
    /// everything charged to it was released.
    fn drained(budget: Arc<MemoryBudget>) -> bool {
        let (admitted_tx, admitted) = mpsc::channel();
        thread::spawn(move || admitted_tx.send(budget.acquire(1).is_ok()).unwrap());
        admitted.recv_timeout(Duration::from_secs(5)) == Ok(true)
    }

    // `commit` itself, not `submit`: `submit` drops every batch it decided before it returns,
    // which would release the charge whether or not `commit` does. The batch is still alive
    // when the budget is checked.
    #[test]
    fn with_parts_a_held_batch_releases_its_output_charge_when_it_is_held() {
        let temp = scratch("writer_parts_output");
        let folder = temp.path();
        let mut output = parts_output(folder, &[(1, "co Full Spring")], 1 << 20, BTreeMap::new());
        let budget = MemoryBudget::new(1);
        let mut held = batch(0, &["kit/u0714p1.ftex"], None);
        held.output = Some(budget.charge(1));

        // Committed, and held: the team's last task, 1, is still to come.
        output.commit(&mut held).unwrap();

        assert!(
            drained(budget),
            "the held batch's output charge was released"
        );
        drop(held);
        drop(output);
    }

    #[test]
    fn a_written_batch_releases_its_output_charge() {
        let temp = scratch("writer_output");
        let mut output = CpkOutput::new(
            OutputSink::cpk(temp.path().join("cup.cpk")),
            BTreeMap::new(),
            "",
            None,
        );
        let budget = MemoryBudget::new(1);
        let mut written = batch(0, &["a/written.bin"], None);
        written.output = Some(budget.charge(1));

        output.commit(&mut written).unwrap();

        assert!(
            drained(budget),
            "the written batch's output charge was released"
        );
        drop(written);
        drop(output);
    }

    #[test]
    fn with_parts_an_override_s_path_is_left_out_of_the_parts_and_reported() {
        let temp = scratch("writer_parts_override");
        let folder = temp.path();
        let mut output = parts_output(
            folder,
            &[(0, "co Full Spring")],
            1 << 20,
            overrides(folder, &["a/over.bin"]),
        );

        let committed = output
            .submit(batch(0, &["a/over.bin", "b/own.bin"], None))
            .unwrap();

        assert_eq!(committed, [(0, vec![note(0), duplicate("a/over.bin")])]);
        assert!(finish_plain(output, PesVersion::Pes21).unwrap());
        let run = folder.join("run");
        assert_eq!(layout(&run.join("4cc_41_teams.cpk")), ["b/own.bin"]);
        let bins = run.join("4cc_08_bins.cpk");
        assert_eq!(
            layout(&bins),
            ["a/over.bin", paths::TEAM_COLOR, paths::UNI_COLOR]
        );
        assert_eq!(entry(&bins, "a/over.bin"), override_bytes("a/over.bin"));
    }

    #[test]
    fn with_parts_a_run_that_commits_nothing_writes_no_part_and_no_placeholder() {
        let temp = scratch("writer_parts_nothing");
        let folder = temp.path();
        let mut output = parts_output(folder, &[(0, "co Full Spring")], 1 << 20, BTreeMap::new());
        output.submit(batch(0, &[], None)).unwrap();

        assert!(!finish_plain(output, PesVersion::Pes21).unwrap());
        assert!(!folder.join("run").exists());
    }
}
