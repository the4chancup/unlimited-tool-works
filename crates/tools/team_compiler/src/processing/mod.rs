//! One task's work, whatever its kind (`plan::TaskKind`: a model folder's package or
//! textures, a kit, a portrait, the logo, ...): its source files, already read, converted and
//! placed by `materialize` as the entries the task commits, whole or not at all.

mod conversion;
mod kit;
mod kit_layout;
mod materialize;
mod model;
mod prefox_common;
mod prefox_face;
mod prefox_shared;
mod prefox_split;
mod referee_marker;
mod team_assets;
mod texture;

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use aesthetics_export::{FileDescriptor, FileKind, ModelFormat};
use dds_convert::{CachePolicy, Converter};
use kit_config::KitSlot;
use pes_version::{Engine, PesVersion};
use pipeline::{MemoryBudget, Permit};
use studio_core::{Disposition, Message, Scope};
use vtree::ScopePath;

use crate::bins::KitColorEntry;
use crate::bins::installed::InstalledPaths;
use crate::messages::{Code, tool_message};
use crate::paths;
use crate::plan::roles::{ModelPackage, texture_format};
use crate::plan::{BuildTask, TaskKind, keeps_installed};
use crate::templates::Templates;
use conversion::ConvertedMaterials;
pub(crate) use materialize::{EntryTarget, TEST_BINS_PREFIX};
use materialize::{TaskOutput, materialize};

/// The bytes of every file a task reads (`TaskKind::files`), keyed by the file's export path:
/// read from the export's source by the coordinator before the task is processed.
pub(crate) type TaskFiles = BTreeMap<ScopePath, Vec<u8>>;

/// What every task of a run reads besides its own content.
pub(crate) struct CompileContext {
    /// The target version, which decides each texture's codec and container, the kit config's
    /// encoding and the portraits' file names.
    pub(crate) version: PesVersion,
    /// The run's texture converter, one for every task: the conversion cache in front of
    /// `dds_convert` (`spec/libs/dds_convert.md` "In-memory conversion cache").
    pub(crate) converter: Converter,
    /// Whether the converter keeps what it converts, from the run's export count.
    pub(crate) cache: CachePolicy,
    /// The run's resources: the placeholder kit texture, and the files a model package gets
    /// when its sources hold none.
    pub(crate) templates: Templates,
    /// The entry paths of the installed CPKs loaded before the run's, where a model task
    /// looks for a texture its model names in the team's Common output that the export does
    /// not supply.
    pub(crate) installed: InstalledPaths,
    /// Where every task's entries go: the run's output mode, consumed by `materialize` alone.
    pub(crate) target: EntryTarget,
    /// The run's memory budget, the one the coordinator acquires each task's source bytes
    /// from: a running task charges what it allocates to it (`MemoryBudget::charge`).
    pub(crate) budget: Arc<MemoryBudget>,
    /// Whether every `.dds` entry a task emits is WESYS-zlibbed (`wrap_dds`): the
    /// `dds_compression` setting resolved for the run's version
    /// (`TeamCompilerSettings::compresses_dds`), never on Fox.
    pub(crate) compress_dds: bool,
}

impl CompileContext {
    /// The context of a run for `version` compiling `compiled_exports` exports (those with at
    /// least one planned task) with `templates` and the `installed` CPKs' entry paths, its
    /// entries going to `target`, its tasks charging `budget`, its DDS entries wrapped when
    /// `compress_dds`, with a fresh converter.
    pub(crate) fn new(
        version: PesVersion,
        compiled_exports: usize,
        templates: Templates,
        installed: InstalledPaths,
        target: EntryTarget,
        budget: Arc<MemoryBudget>,
        compress_dds: bool,
    ) -> CompileContext {
        CompileContext {
            version,
            converter: Converter::new(),
            cache: cache_policy(compiled_exports),
            templates,
            installed,
            target,
            budget,
            compress_dds,
        }
    }
}

/// Whether a run compiling `compiled_exports` exports caches its conversions: at most two,
/// the edit-and-test loop the cache is for; a larger run converts mostly first-seen textures
/// and would retain every one for nothing (`spec/libs/dds_convert.md` "Engagement limit").
fn cache_policy(compiled_exports: usize) -> CachePolicy {
    if compiled_exports <= 2 {
        CachePolicy::Use
    } else {
        CachePolicy::Bypass
    }
}

/// One file in a container: its path in a CPK, or its name in a bin, and its bytes.
pub(crate) type Entry = (String, Vec<u8>);

/// A note a task that succeeded makes on its folder: the code and its context. What a task
/// drops is its failure, a `TaskFailure`, not a finding.
pub(crate) type Finding = (Code, Vec<(&'static str, String)>);

/// Why a task failed: the finding reported on its folder (its file, for a portrait; its main
/// file, for the logo). A merge conflict between parts has its own code
/// (`merge_material_conflict`, `skl_merge_conflict`), and so have conversion's texture finding
/// (`texture_codec_unsupported`) and a kit wearing the referees' collar
/// (`kit_collar_reserved`); any other error is `folder_pack_failed` carrying the error chain.
pub(crate) struct TaskFailure {
    /// The finding's code.
    pub(crate) code: Code,
    /// Its context entries (`material=shirt`, `error=<chain>`).
    pub(crate) context: Vec<(&'static str, String)>,
}

impl From<anyhow::Error> for TaskFailure {
    fn from(error: anyhow::Error) -> TaskFailure {
        TaskFailure {
            code: Code::FolderPackFailed,
            context: vec![("error", format!("{error:#}"))],
        }
    }
}

impl From<fmdl::FmdlError> for TaskFailure {
    fn from(error: fmdl::FmdlError) -> TaskFailure {
        TaskFailure::from(anyhow::Error::from(error))
    }
}

/// One task's result, handed to the writer.
pub(crate) struct TaskBatch {
    /// The task's position in the manifest: the writer commits batches in this order.
    pub(crate) index: usize,
    /// The entries the task commits, at their paths in the output (`materialize`); empty when
    /// the task failed.
    pub(crate) entries: Vec<Entry>,
    /// The manifest positions of the player folder's group the task belongs to
    /// (`TaskGroup::tasks`), its textures batch last: the writer holds the group's packages
    /// until that batch arrives and decides them all. `None` for a task the writer commits on
    /// its own.
    pub(crate) group: Option<Range<usize>>,
    /// A kit's config as a `UniformParameter.bin` entry, applied only when the batch is
    /// committed; `None` for a kit compiled for PES 15-17, which have no such bin.
    pub(crate) uniparam: Option<Entry>,
    /// A kit's `UniColor.bin` entry with its team ID, applied only when the batch is
    /// committed.
    pub(crate) uni_color: Option<(u16, KitColorEntry)>,
    /// A committed kit of a `Midcup` export that emitted no config (its folder held no
    /// `config.toml`): its team and slot. The writer edits the slot's installed config
    /// in place, or has the template written where none is installed (`kit_configs`).
    pub(crate) configless_kit: Option<(u16, KitSlot)>,
    /// The task's findings.
    pub(crate) messages: Vec<Message>,
    /// The memory the task was charged, released once the writer has its entries. A `.7z`
    /// export's tasks share one, its whole decompressed size, released with the last of them.
    pub(crate) permit: Option<Arc<Permit>>,
    /// Its entries' bytes (`output_len`), charged when the task returns, released with
    /// `permit`.
    pub(crate) output: Option<Permit>,
}

impl TaskBatch {
    /// The bytes of every buffer the batch carries to the writer: its entries' and its kit
    /// config's. The paths and the kit colors are a few bytes each and not counted.
    pub(crate) fn output_len(&self) -> usize {
        let entries = self.entries.iter().chain(&self.uniparam);
        entries.map(|(_, bytes)| bytes.len()).sum()
    }
}

/// Runs `task`, the manifest's task number `index`, over `files`, the bytes of every file it
/// reads. A task that fails commits nothing: its batch has no entries and reports its
/// `TaskFailure` on its folder, which is left out of the CPK.
pub(crate) fn process_task(
    index: usize,
    task: BuildTask,
    mut files: TaskFiles,
    ctx: &CompileContext,
) -> TaskBatch {
    let mut findings = Vec::new();
    let result = match &task.kind {
        TaskKind::Models {
            folder,
            package,
            ids,
            kits,
        } => match (ctx.version.engine(), package) {
            (Engine::Fox, _) => model::package(
                folder,
                *package,
                task.team_id,
                ctx,
                &mut files,
                &mut findings,
            ),
            (Engine::PreFox, ModelPackage::Face) => {
                prefox_face::face(folder, kits, task.team_id, ctx, &mut files, &mut findings)
            }
            (Engine::PreFox, ModelPackage::Boots | ModelPackage::Gloves) => prefox_shared::package(
                folder,
                *package,
                task.team_id,
                ctx,
                &mut files,
                &mut findings,
            ),
        }
        .map(|files| {
            let output = TaskOutput::Package {
                package: *package,
                ids: ids.clone(),
                files,
            };
            (output, None)
        }),
        TaskKind::Textures { folder, kits } => {
            texture::folder_textures(folder, kits, task.team_id, ctx, &mut files, &mut findings)
                .map(|entries| (TaskOutput::Entries(entries), None))
        }
        TaskKind::CommonTextures {
            folder,
            textures,
            kits,
            environment_map,
        } => texture::common_textures(
            folder,
            textures,
            kits,
            *environment_map,
            ctx,
            &mut files,
            &mut findings,
        )
        .map(|converted| {
            let engine = ctx.version.engine();
            let entries = texture::common_texture_entries(folder, task.team_id, engine, converted);
            (TaskOutput::Entries(entries), None)
        }),
        TaskKind::CommonModels {
            folder,
            files: common,
            textures,
        } => prefox_common::common_models(
            folder,
            common,
            textures,
            task.team_id,
            ctx,
            &mut files,
            &mut findings,
        )
        .map(|entries| (TaskOutput::Entries(entries), None)),
        TaskKind::Portrait { player_id, file } => {
            let name = file.path.name();
            let format = texture_format(name)
                .expect("planning lists a portrait by an extension `dds_convert` accepts");
            texture::portrait(
                &ctx.budget,
                ctx.version,
                format,
                name,
                take(&mut files, file),
                &mut findings,
            )
            .map(|bytes| {
                let entry = (
                    pes_savefile::portrait::portrait_path(ctx.version, *player_id),
                    bytes,
                );
                (TaskOutput::Entries(vec![entry]), None)
            })
            .map_err(TaskFailure::from)
        }
        TaskKind::Kit {
            slot,
            kit,
            edits,
            coverage,
        } => {
            let configless = keeps_installed(kit, *coverage).then_some((task.team_id, *slot));
            let job = kit::KitJob {
                slot: *slot,
                kit,
                edits: *edits,
                coverage: *coverage,
                team_id: task.team_id,
            };
            kit::kit(&job, ctx, &mut files, &mut findings).map(|commit| {
                (
                    TaskOutput::Entries(commit.entries),
                    Some((commit.uniform_parameter, commit.uni_color, configless)),
                )
            })
        }
        TaskKind::Logo { logo } => team_assets::logo(
            logo,
            task.team_id,
            ctx.version,
            &ctx.budget,
            &mut files,
            &mut findings,
        )
        .map(|entries| (TaskOutput::Entries(entries), None)),
        TaskKind::RefereeMarker { marker } => {
            referee_marker::referee_marker(marker, ctx, &mut files)
                .map(|entries| (TaskOutput::Entries(entries), None))
        }
        // The deep pass has read and checked the model. One in the format the target reads
        // keeps the materials its author gave it (embedded in an FMDL, named against the
        // shared `uniform.mtl` in a `.model`). An FMDL on Fox goes out as it is. A `.model` on
        // PES 15-17 runs the same-engine pre-check, read with the templates' `uniform.mtl` as
        // its set since a collar carries none: one posed off the version's skeleton is moved
        // onto it, its material names kept, and any other goes out as it is, a WESYS-wrapped
        // one still wrapped: the game reads both. An FMDL for PES 15-17 is converted on the
        // version's body table, no `.skl` being read beside a collar, its materials named as
        // the stock collars' for the shared `uniform.mtl`, which dresses it: its own material
        // set is not written. A `.model` for PES 18-21 is converted with the templates'
        // `uniform.mtl` as its `.mtl`, its materials the stock collars' Fox set by name; the
        // skeleton the conversion may write is dropped with no finding, a collar having no
        // skeleton slot.
        TaskKind::Collar { file, id } => {
            let bytes = take(&mut files, file);
            let name = file.path.name();
            let is_fmdl = file.kind == FileKind::Model(ModelFormat::Fmdl);
            let uniform_mtl = ctx.templates.uniform_mtl();
            let written = match ctx.version.engine() {
                Engine::PreFox if is_fmdl => conversion::fmdl_for_pre_fox(
                    name,
                    &bytes,
                    None,
                    ctx,
                    &mut findings,
                    ConvertedMaterials::StockCollar,
                )
                .map(|converted| converted.model),
                Engine::PreFox => conversion::model_for_pre_fox(
                    name,
                    bytes,
                    uniform_mtl,
                    ctx,
                    &mut findings,
                    ConvertedMaterials::StockCollar,
                ),
                Engine::Fox if is_fmdl => Ok(bytes),
                Engine::Fox => conversion::model_for_fox(
                    name,
                    &bytes,
                    uniform_mtl,
                    ctx,
                    &mut findings,
                    ConvertedMaterials::StockCollar,
                )
                .map(|converted| converted.model),
            };
            written.map(|bytes| {
                let entries = vec![(paths::collar(ctx.version.engine(), *id), bytes)];
                (TaskOutput::Entries(entries), None)
            })
        }
    };
    let mut batch = TaskBatch {
        index,
        entries: Vec::new(),
        group: task.group.as_ref().map(|group| group.tasks.clone()),
        uniparam: None,
        uni_color: None,
        configless_kit: None,
        messages: Vec::new(),
        permit: None,
        output: None,
    };
    let scope = Scope::Folder {
        export_id: task.export_id,
        path: task.kind.folder_path(),
    };
    match result {
        Ok((output, kit_bins)) => {
            batch.entries = materialize(output, &task, &ctx.target);
            if ctx.compress_dds {
                wrap_dds(&mut batch.entries);
            }
            if let Some((uniform_parameter, colors, configless_kit)) = kit_bins {
                batch.uniparam = uniform_parameter;
                batch.uni_color = colors.map(|entry| (task.team_id, entry));
                batch.configless_kit = configless_kit;
            }
            batch.messages = findings
                .into_iter()
                .map(|(code, context)| {
                    tool_message(code, scope.clone(), Disposition::Keep, context)
                })
                .collect();
        }
        // A failed task reports its failure alone: a note about a merge whose output is not
        // in the CPK would describe nothing the member can find.
        Err(failure) => {
            batch.messages.push(tool_message(
                failure.code,
                scope,
                dropped_unit(&task.kind),
                failure.context,
            ));
        }
    }
    batch
}

/// What a failed task drops, whichever step failed (its sources' read or its processing):
/// its unit. A portrait task is its one file, the logo task its files, the marker task its
/// texture, the collar task its file, every other task a folder.
pub(crate) fn dropped_unit(kind: &TaskKind) -> Disposition {
    match kind {
        TaskKind::Portrait { .. }
        | TaskKind::Logo { .. }
        | TaskKind::RefereeMarker { .. }
        | TaskKind::Collar { .. } => Disposition::DropFile,
        TaskKind::Models { .. }
        | TaskKind::Textures { .. }
        | TaskKind::CommonTextures { .. }
        | TaskKind::CommonModels { .. }
        | TaskKind::Kit { .. } => Disposition::DropFolder,
    }
}

/// Replaces each `.dds` entry of `entries` (the extension in any case) by its bytes
/// WESYS-zlibbed, which PES 15-17 read as they read a plain DDS. An entry already wrapped (a
/// source `texture::convert` passes through as it is) is left as it is. Called on the worker
/// that made the entries, so the deflate runs in parallel and the writer stays a pass-through
/// for bytes; a face CPK holds no DDS, its textures being entries of the folder's textures
/// task.
fn wrap_dds(entries: &mut [Entry]) {
    for (path, bytes) in entries {
        let is_dds = Path::new(path.as_str())
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("dds"));
        if is_dds && !wezlib::is_wrapped(bytes) {
            *bytes = wezlib::compress(bytes);
        }
    }
}

/// The bytes of `file`, taken out of the task's `files`.
fn take(files: &mut TaskFiles, file: &FileDescriptor) -> Vec<u8> {
    files
        .remove(&file.path)
        .expect("the coordinator reads every file `TaskKind::files` lists, each once")
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};
    use std::path::{Path, PathBuf};

    use aesthetics_export::{
        ExportCoverage, KitFolder, KitLayout, KitTexture, KitTextureSource, SharedModelFolder,
    };
    use fmdl::ops::paths::{rewrite_texture_paths, texture_paths};
    use fmdl::{FmdlFile, Model};
    use fpk::{FpkFile, FpkKind};
    use kit_config::{KitConfig, KitSlot, TexturePresence, texture_names};
    use studio_core::{ExportId, Severity};

    use super::*;
    use crate::paths::{PackageKey, TextureHome};
    use crate::plan::{
        CombinedFolder, CommonModel, EffectiveTeamKitFpc, ModelFolder, TaskGroup, TeamKitEdits,
    };

    const PLAYER: &str = "Players/05 - The Chad Stormworks Player";
    const COMMON: &str = "Asset/model/character/common/792/05 - The Chad Stormworks Player/sourceimages/#windx11/shirt.ftex";
    const COMMON_DIRECTORY: &str =
        "/Assets/pes16/model/character/common/792/05 - The Chad Stormworks Player/sourceimages/";

    /// The tracer fixture's export folder.
    fn tracer() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracer/studio/egg Midcup Tracer")
    }

    /// The export file at `relative`, described as the structure pass would.
    fn file(relative: &str) -> FileDescriptor {
        let path = ScopePath::new(relative).unwrap();
        FileDescriptor {
            size: std::fs::metadata(tracer().join(relative)).map_or(0, |data| data.len()),
            kind: aesthetics_export::classify(path.name()),
            source: path.clone(),
            path,
        }
    }

    /// The tracer's player folder holding the files `names`, its textures going to its common
    /// subfolder.
    fn player(names: &[&str]) -> ModelFolder {
        ModelFolder {
            path: ScopePath::new(PLAYER).unwrap(),
            files: names
                .iter()
                .map(|name| file(&format!("{PLAYER}/{name}")))
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

    fn kit(config: bool, layout: Option<KitLayout>) -> KitFolder {
        KitFolder {
            path: ScopePath::new("Kits/g1").unwrap(),
            label: None,
            config: config.then(|| file("Kits/g1/config.toml")),
            colors: None,
            icon: Some(11),
            layout,
            textures: vec![KitTexture {
                stem: "kit".to_owned(),
                file: file("Kits/g1/kit.dds"),
                source: KitTextureSource::Own,
            }],
        }
    }

    /// `kind` processed for PES 21 as task 3 of export 4, over its files read from the tracer
    /// folder.
    fn run(kind: TaskKind) -> TaskBatch {
        run_for(kind, PesVersion::Pes21)
    }

    /// `run` for the target `version`, the task in no group.
    fn run_for(kind: TaskKind, version: PesVersion) -> TaskBatch {
        run_task(kind, version, None)
    }

    /// `kind` processed for `version` as task 3 of export 4, in `group`, over its files read
    /// from the tracer folder.
    fn run_task(kind: TaskKind, version: PesVersion, group: Option<TaskGroup>) -> TaskBatch {
        let files = kind
            .files()
            .into_iter()
            .map(|file| {
                let bytes = std::fs::read(tracer().join(file.source.as_str())).unwrap();
                (file.path.clone(), bytes)
            })
            .collect();
        process(kind, version, group, files)
    }

    /// `kind` processed for `version` as task 3 of export 4, over `files`, with the
    /// installed CPKs' entry paths `installed`.
    fn process_installed(
        kind: TaskKind,
        version: PesVersion,
        files: TaskFiles,
        installed: InstalledPaths,
    ) -> TaskBatch {
        let task = BuildTask {
            export_id: ExportId(4),
            team_id: 792,
            kind,
            charge: 0,
            group: None,
        };
        process_task(
            3,
            task,
            files,
            &CompileContext::new(
                version,
                1,
                Templates::embedded(),
                installed,
                EntryTarget::GamePaths {
                    engine: version.engine(),
                },
                MemoryBudget::new(usize::MAX),
                false,
            ),
        )
    }

    /// `kind` processed for `version` as task 3 of export 4, in `group`, over `files`.
    fn process(
        kind: TaskKind,
        version: PesVersion,
        group: Option<TaskGroup>,
        files: TaskFiles,
    ) -> TaskBatch {
        let task = BuildTask {
            export_id: ExportId(4),
            team_id: 792,
            kind,
            charge: 0,
            group,
        };
        process_task(
            3,
            task,
            files,
            &CompileContext::new(
                version,
                1,
                Templates::embedded(),
                InstalledPaths::Unknown,
                EntryTarget::GamePaths {
                    engine: version.engine(),
                },
                MemoryBudget::new(usize::MAX),
                false,
            ),
        )
    }

    fn paths(batch: &TaskBatch) -> Vec<&str> {
        batch
            .entries
            .iter()
            .map(|(path, _)| path.as_str())
            .collect()
    }

    /// The tracer's player folder, every file the lead's fixture holds.
    fn whole_player() -> ModelFolder {
        player(&[
            "boots.fmdl",
            "face_diff.bin",
            "fcl_hair.fmdl",
            "fcl_hair.skl",
            "fcl_hair_sim.fclo",
            "glove_l.fmdl",
            "glove_r.fmdl",
            "shirt.dds",
        ])
    }

    /// The (file name, directory) of every texture path of the package's model `name`.
    fn texture_directories(package: &FpkFile, name: &str) -> Vec<(String, String)> {
        let model = FmdlFile::read(package.get(name).unwrap()).unwrap();
        texture_paths(&model)
            .unwrap()
            .into_iter()
            .map(|path| (path.file_name, path.directory))
            .collect()
    }

    /// Asserts the model's own texture points at the player's common folder and the game's
    /// own at the team's, with no `000` placeholder left.
    fn assert_rewritten(directories: &[(String, String)]) {
        assert!(
            directories.contains(&("shirt.dds".to_owned(), COMMON_DIRECTORY.to_owned())),
            "{directories:?}"
        );
        assert!(
            directories.iter().any(|(_, directory)| directory
                == "/Assets/pes16/model/character/common/792/sourceimages/"),
            "{directories:?}"
        );
        assert!(
            directories
                .iter()
                .all(|(_, directory)| !directory.contains("/000/")),
            "{directories:?}"
        );
    }

    #[test]
    fn a_model_task_charges_its_parts_source_size_while_it_builds_its_package() {
        // The gloves package of the tracer's player: its two parts, `glove_l.fmdl` and
        // `glove_r.fmdl`, are the only models it parses. The task's source permit and its
        // batch's output charge are the coordinator's, so here the parts are the whole peak.
        let kind = TaskKind::Models {
            folder: whole_player(),
            package: ModelPackage::Gloves,
            ids: vec![PackageKey::Id(79205)],
            kits: Vec::new(),
        };
        let files: TaskFiles = kind
            .files()
            .into_iter()
            .map(|file| {
                let bytes = std::fs::read(tracer().join(file.source.as_str())).unwrap();
                (file.path.clone(), bytes)
            })
            .collect();
        let part_len = |name: &str| {
            let path = tracer().join(format!("{PLAYER}/{name}"));
            usize::try_from(std::fs::metadata(path).unwrap().len()).unwrap()
        };
        let parts = part_len("glove_l.fmdl") + part_len("glove_r.fmdl");
        let budget = MemoryBudget::new(usize::MAX);
        let task = BuildTask {
            export_id: ExportId(4),
            team_id: 792,
            kind,
            charge: 0,
            group: None,
        };
        let context = CompileContext::new(
            PesVersion::Pes21,
            1,
            Templates::embedded(),
            InstalledPaths::Unknown,
            EntryTarget::GamePaths {
                engine: Engine::Fox,
            },
            Arc::clone(&budget),
            false,
        );

        let batch = process_task(3, task, files, &context);

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(parts, 2 * 8161, "the tracer's two gloves");
        assert_eq!(budget.peak(), parts);
    }

    #[test]
    fn a_face_package_is_packed_once_per_slot_without_the_textures() {
        let batch = run(TaskKind::Models {
            folder: whole_player(),
            package: ModelPackage::Face,
            ids: vec![PackageKey::Id(79205), PackageKey::Id(79207)],
            kits: Vec::new(),
        });

        assert_eq!(batch.index, 3);
        assert!(batch.messages.is_empty() && batch.uniparam.is_none());
        assert_eq!(batch.group, None);
        assert_eq!(
            paths(&batch),
            [
                "Asset/model/character/face/real/79205/#Win/face.fpk",
                "Asset/model/character/face/real/79205/#Win/face.fpkd",
                "Asset/model/character/face/real/79207/#Win/face.fpk",
                "Asset/model/character/face/real/79207/#Win/face.fpkd",
            ]
        );
        assert_eq!(batch.entries[0].1, batch.entries[2].1, "one package, twice");
        let fpkd = FpkFile::read(&batch.entries[1].1).unwrap();
        assert_eq!((fpkd.kind(), fpkd.len()), (FpkKind::Fpkd, 0));

        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(
            names,
            [
                "face_diff.bin",
                "fcl_hair.fmdl",
                "fcl_hair_sim.fclo",
                "fcl_hair_sim.skl"
            ]
        );
        // The folder's own `face_diff.bin` is packed, not the template. (Its `.fclo` and
        // `.skl` are byte-identical to the templates, so they prove nothing here.)
        let own = std::fs::read(tracer().join(format!("{PLAYER}/face_diff.bin"))).unwrap();
        assert_ne!(own, Templates::embedded().face_diff());
        assert_eq!(package.get("face_diff.bin").unwrap(), own);
        assert_rewritten(&texture_directories(&package, "fcl_hair.fmdl"));
    }

    /// PES 19's `body.skl`: a real skeleton that differs from the bundled PES 21 template,
    /// standing in for a custom one.
    fn other_skeleton() -> Vec<u8> {
        let bytes = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../resources/skeletons/pes19/body.skl"),
        )
        .unwrap();
        assert_ne!(bytes, Templates::embedded().body_skeleton());
        bytes
    }

    /// `kind`'s files read from the tracer folder, each file at a path of `replacements`
    /// replaced by the bytes given with it, processed for PES 21 in no group.
    fn run_with(kind: TaskKind, replacements: &[(&str, &[u8])]) -> TaskBatch {
        let mut files: TaskFiles = kind
            .files()
            .into_iter()
            .map(|file| {
                let bytes = std::fs::read(tracer().join(file.source.as_str())).unwrap();
                (file.path.clone(), bytes)
            })
            .collect();
        for (path, bytes) in replacements {
            files.insert(ScopePath::new(path).unwrap(), bytes.to_vec());
        }
        process(kind, PesVersion::Pes21, None, files)
    }

    #[test]
    fn a_face_package_gets_the_face_files_and_the_skeleton_its_sources_lack() {
        // The hair alone: all three injected.
        let batch = run(TaskKind::Models {
            folder: player(&["fcl_hair.fmdl"]),
            package: ModelPackage::Face,
            ids: vec![PackageKey::Id(79205)],
            kits: Vec::new(),
        });
        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(
            names,
            [
                "face_diff.bin",
                "fcl_hair.fmdl",
                "fcl_hair_sim.fclo",
                "fcl_hair_sim.skl"
            ]
        );
        assert_eq!(
            package.get("face_diff.bin").unwrap(),
            Templates::embedded().face_diff()
        );
        assert_eq!(
            package.get("fcl_hair_sim.fclo").unwrap(),
            Templates::embedded().fcl_hair_sim()
        );
        assert_eq!(
            package.get("fcl_hair_sim.skl").unwrap(),
            Templates::embedded().body_skeleton()
        );

        // The hair with its own skeleton: that one is packed.
        let custom = other_skeleton();
        let batch = run_with(
            TaskKind::Models {
                folder: player(&["fcl_hair.fmdl", "fcl_hair.skl"]),
                package: ModelPackage::Face,
                ids: vec![PackageKey::Id(79205)],
                kits: Vec::new(),
            },
            &[(&format!("{PLAYER}/fcl_hair.skl"), &custom)],
        );
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        assert_eq!(package.get("fcl_hair_sim.skl").unwrap(), custom);

        // A face without a hair part gets only the `face_diff.bin`: no simulation, no
        // skeleton, and a skeleton named after the face model is never read.
        let batch = run(TaskKind::Models {
            folder: player_with(
                vec![
                    named(&format!("{PLAYER}/face_high.fmdl"), "fcl_hair.fmdl"),
                    named(&format!("{PLAYER}/face_high.skl"), "fcl_hair.skl"),
                ],
                Vec::new(),
            ),
            package: ModelPackage::Face,
            ids: vec![PackageKey::Id(79205)],
            kits: Vec::new(),
        });
        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(names, ["face_diff.bin", "face_high.fmdl"]);
        assert_eq!(
            package.get("face_diff.bin").unwrap(),
            Templates::embedded().face_diff()
        );
    }

    #[test]
    fn the_face_of_a_folder_with_no_face_model_is_the_bundled_face_diff_alone() {
        let folder = player(&["boots.fmdl", "face_diff.bin"]);
        let own = std::fs::read(tracer().join(format!("{PLAYER}/face_diff.bin"))).unwrap();
        assert_ne!(own, Templates::embedded().face_diff());
        let kind = TaskKind::Models {
            folder,
            package: ModelPackage::Face,
            ids: vec![PackageKey::Id(79205)],
            kits: Vec::new(),
        };
        assert!(kind.files().is_empty(), "the face reads nothing");

        let batch = run(kind);

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(
            paths(&batch),
            [
                "Asset/model/character/face/real/79205/#Win/face.fpk",
                "Asset/model/character/face/real/79205/#Win/face.fpkd",
            ]
        );
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(names, ["face_diff.bin"]);
        assert_eq!(
            package.get("face_diff.bin").unwrap(),
            Templates::embedded().face_diff()
        );
        let fpkd = FpkFile::read(&batch.entries[1].1).unwrap();
        assert_eq!((fpkd.kind(), fpkd.len()), (FpkKind::Fpkd, 0));
    }

    #[test]
    fn an_unsuffixed_model_is_a_hair_part_whose_skeleton_is_the_hair_s() {
        let custom = other_skeleton();
        let batch = run_with(
            TaskKind::Models {
                folder: player_with(
                    vec![
                        named(&format!("{PLAYER}/torso.fmdl"), "fcl_hair.fmdl"),
                        named(&format!("{PLAYER}/torso.skl"), "fcl_hair.skl"),
                    ],
                    Vec::new(),
                ),
                package: ModelPackage::Face,
                ids: vec![PackageKey::Id(79205)],
                kits: Vec::new(),
            },
            &[(&format!("{PLAYER}/torso.skl"), &custom)],
        );

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(
            names,
            [
                "face_diff.bin",
                "fcl_hair.fmdl",
                "fcl_hair_sim.fclo",
                "fcl_hair_sim.skl"
            ]
        );
        assert_eq!(package.get("fcl_hair_sim.skl").unwrap(), custom);
        assert_eq!(
            packed_model(&package, "fcl_hair.fmdl").meshes.len(),
            tracer_model("fcl_hair.fmdl").meshes.len()
        );
    }

    #[test]
    fn a_boots_package_holds_the_model_and_the_standard_skeleton_when_it_brings_none() {
        let batch = run(TaskKind::Models {
            folder: whole_player(),
            package: ModelPackage::Boots,
            ids: vec![PackageKey::Id(3745)],
            kits: Vec::new(),
        });

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(
            paths(&batch),
            [
                "Asset/model/character/boots/k3745/#Win/boots.fpk",
                "Asset/model/character/boots/k3745/#Win/boots.fpkd",
            ]
        );
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(names, ["boots.fmdl", "boots.skl"]);
        assert_eq!(
            package.get("boots.skl").unwrap(),
            Templates::embedded().body_skeleton()
        );
        assert_rewritten(&texture_directories(&package, "boots.fmdl"));
    }

    #[test]
    fn a_skeleton_named_after_the_boots_model_is_packed_in_the_standard_one_s_place() {
        let custom = other_skeleton();
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl"),
                named(&format!("{PLAYER}/kit_boots.skl"), "fcl_hair.skl"),
            ],
            Vec::new(),
        );

        let batch = run_with(
            TaskKind::Models {
                folder,
                package: ModelPackage::Boots,
                ids: vec![PackageKey::Id(3745), PackageKey::Id(3747)],
                kits: Vec::new(),
            },
            &[(&format!("{PLAYER}/kit_boots.skl"), &custom)],
        );

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(
            paths(&batch),
            [
                "Asset/model/character/boots/k3745/#Win/boots.fpk",
                "Asset/model/character/boots/k3745/#Win/boots.fpkd",
                "Asset/model/character/boots/k3747/#Win/boots.fpk",
                "Asset/model/character/boots/k3747/#Win/boots.fpkd",
            ]
        );
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(names, ["boots.fmdl", "boots.skl"]);
        assert_eq!(package.get("boots.skl").unwrap(), custom);
    }

    #[test]
    fn a_gloves_package_holds_both_hands_under_their_allowed_names() {
        let batch = run(TaskKind::Models {
            folder: whole_player(),
            package: ModelPackage::Gloves,
            ids: vec![PackageKey::Id(3745)],
            kits: Vec::new(),
        });

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(
            paths(&batch),
            [
                "Asset/model/character/glove/g3745/#Win/glove.fpk",
                "Asset/model/character/glove/g3745/#Win/glove.fpkd",
            ]
        );
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(names, ["glove_l.fmdl", "glove_r.fmdl"]);
        let fpkd = FpkFile::read(&batch.entries[1].1).unwrap();
        assert_eq!((fpkd.kind(), fpkd.len()), (FpkKind::Fpkd, 0));
    }

    /// The hand-split fixture's full-body model (`tests/fixtures/hand_split/README.md`): 40
    /// faces, of which each hand's split takes 8 and the body keeps 24.
    fn hand_split_body() -> Vec<u8> {
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hand_split/body.fmdl"),
        )
        .unwrap()
    }

    /// The tracer's player folder holding `body.fmdl`, a hand-split face part, beside the
    /// tracer files `others`.
    fn hand_split_player(others: &[&str]) -> ModelFolder {
        let body = format!("{PLAYER}/body.fmdl");
        let mut files = vec![named(&body, "fcl_hair.fmdl")];
        files.extend(others.iter().map(|name| file(&format!("{PLAYER}/{name}"))));
        ModelFolder {
            hand_split: [ScopePath::new(&body).unwrap()].into(),
            environment_map: false,
            ..player_with(files, Vec::new())
        }
    }

    /// `package` of `folder` processed for PES 21 under id `id`, `body.fmdl` holding `body`
    /// and every other file the tracer's.
    fn run_hand_split(
        folder: ModelFolder,
        package: ModelPackage,
        id: u32,
        body: &[u8],
    ) -> TaskBatch {
        let body_path = format!("{PLAYER}/body.fmdl");
        run_with(
            TaskKind::Models {
                folder,
                package,
                ids: vec![PackageKey::Id(id)],
                kits: Vec::new(),
            },
            &[(&body_path, body)],
        )
    }

    /// The number of faces of the package's model `name`, read back with `fmdl`.
    fn face_count(package: &FpkFile, name: &str) -> usize {
        packed_model(package, name)
            .meshes
            .iter()
            .map(|mesh| mesh.faces.len())
            .sum()
    }

    #[test]
    fn a_hand_split_face_part_packs_its_body_in_the_face_and_says_so() {
        let batch = run_hand_split(
            hand_split_player(&[]),
            ModelPackage::Face,
            71405,
            &hand_split_body(),
        );

        assert_eq!(
            one_message(&batch),
            (
                "model_hand_split",
                Severity::Info,
                Disposition::Keep,
                &[
                    ("model".to_owned(), "body.fmdl".to_owned()),
                    ("gloves".to_owned(), "glove_l, glove_r".to_owned())
                ][..]
            )
        );
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(
            names,
            [
                "face_diff.bin",
                "fcl_hair.fmdl",
                "fcl_hair_sim.fclo",
                "fcl_hair_sim.skl"
            ]
        );
        assert_eq!(face_count(&package, "fcl_hair.fmdl"), 24);
    }

    #[test]
    fn a_hand_split_face_part_gives_the_gloves_package_both_hands() {
        let batch = run_hand_split(
            hand_split_player(&[]),
            ModelPackage::Gloves,
            625,
            &hand_split_body(),
        );

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(
            paths(&batch),
            [
                "Asset/model/character/glove/g0625/#Win/glove.fpk",
                "Asset/model/character/glove/g0625/#Win/glove.fpkd",
            ]
        );
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(names, ["glove_l.fmdl", "glove_r.fmdl"]);
        assert_eq!(face_count(&package, "glove_l.fmdl"), 8);
        assert_eq!(face_count(&package, "glove_r.fmdl"), 8);
    }

    #[test]
    fn a_split_hand_merges_with_an_authored_glove_of_its_name() {
        // The authored left glove is the hand the split cuts from the fixture, 8 faces on the
        // same bones: the tracer's own glove, bound differently, could not merge (below).
        let alone = run_hand_split(
            hand_split_player(&[]),
            ModelPackage::Gloves,
            625,
            &hand_split_body(),
        );
        let authored = FpkFile::read(&alone.entries[0].1)
            .unwrap()
            .get("glove_l.fmdl")
            .unwrap()
            .to_vec();
        let glove_l = format!("{PLAYER}/glove_l.fmdl");
        let folder = hand_split_player(&["glove_l.fmdl"]);
        let kind = TaskKind::Models {
            folder,
            package: ModelPackage::Gloves,
            ids: vec![PackageKey::Id(625)],
            kits: Vec::new(),
        };
        let body_path = format!("{PLAYER}/body.fmdl");

        let batch = run_with(
            kind,
            &[(&body_path, &hand_split_body()), (&glove_l, &authored)],
        );

        assert_eq!(
            one_message(&batch),
            (
                "fmdl_merged",
                Severity::Info,
                Disposition::Keep,
                &[("model".to_owned(), "glove_l.fmdl".to_owned())][..]
            )
        );
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        assert_eq!(face_count(&package, "glove_l.fmdl"), 8 + 8);
        assert_eq!(face_count(&package, "glove_r.fmdl"), 8);

        // The tracer's left glove parents `sk_hand_l` to its forearm, which the split hand,
        // pruned of its forearm, does not: parts on two skeletons, as for any merge.
        let batch = run_hand_split(
            hand_split_player(&["glove_l.fmdl"]),
            ModelPackage::Gloves,
            625,
            &hand_split_body(),
        );
        assert_eq!(
            one_message(&batch),
            (
                "skl_merge_conflict",
                Severity::Error,
                Disposition::DropFolder,
                &[("bone".to_owned(), "sk_hand_l".to_owned())][..]
            )
        );
    }

    #[test]
    fn a_face_part_that_is_all_hand_leaves_no_body_in_the_face() {
        // The fixture with every vertex weighed fully on its left index finger: the whole
        // model is the left hand.
        let all_hand = {
            let bytes = hand_split_body();
            let mut model = Model::from_file(&FmdlFile::read(&bytes).unwrap()).unwrap();
            let finger = model
                .bones
                .iter()
                .position(|bone| bone.name == "skh_index_mcp_l")
                .unwrap();
            for mesh in &mut model.meshes {
                let entry = mesh.bone_group.iter().position(|&bone| bone == finger);
                let entry = u8::try_from(entry.unwrap()).unwrap();
                let count = mesh.vertices.positions.len();
                mesh.vertices.bone_indices = Some(vec![[entry, 0, 0, 0]; count]);
                mesh.vertices.bone_weights = Some(vec![[255, 0, 0, 0]; count]);
            }
            model.to_file().unwrap().write()
        };

        let batch = run_hand_split(hand_split_player(&[]), ModelPackage::Face, 71405, &all_hand);

        assert_eq!(
            one_message(&batch).3,
            [
                ("model".to_owned(), "body.fmdl".to_owned()),
                ("gloves".to_owned(), "glove_l".to_owned())
            ]
        );
        // No face model is left: the face holds its face diff alone, the template's here.
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(names, ["face_diff.bin"]);

        let batch = run_hand_split(hand_split_player(&[]), ModelPackage::Gloves, 625, &all_hand);
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(names, ["glove_l.fmdl"]);
        assert_eq!(face_count(&package, "glove_l.fmdl"), 40);
    }

    #[test]
    fn a_hand_split_part_that_cannot_be_read_fails_the_task_with_model_conversion_failed() {
        for package in [ModelPackage::Face, ModelPackage::Gloves] {
            let batch = run_hand_split(hand_split_player(&[]), package, 625, b"not a model");

            assert!(batch.entries.is_empty(), "{package:?}");
            let (code, severity, disposition, context) = one_message(&batch);
            assert_eq!(
                (code, severity, disposition),
                (
                    "model_conversion_failed",
                    Severity::Error,
                    Disposition::DropFolder
                ),
                "{package:?}"
            );
            let error = FmdlFile::read(b"not a model").unwrap_err().to_string();
            assert_eq!(
                context,
                [
                    ("model".to_owned(), "body.fmdl".to_owned()),
                    ("error".to_owned(), error)
                ]
            );
        }
    }

    #[test]
    fn a_shared_gloves_folder_s_package_and_textures_go_under_its_shared_id() {
        // The tracer's boots model stands in for a left glove: a real FMDL naming `shirt.dds`.
        let folder_path = ScopePath::new("Gloves/Grip").unwrap();
        let glove = ScopePath::new("Gloves/Grip/glove_l.fmdl").unwrap();
        let shirt = ScopePath::new("Gloves/Grip/shirt.dds").unwrap();
        let folder = ModelFolder {
            path: folder_path,
            files: vec![
                FileDescriptor {
                    kind: aesthetics_export::classify(glove.name()),
                    path: glove,
                    ..file(&format!("{PLAYER}/boots.fmdl"))
                },
                FileDescriptor {
                    kind: aesthetics_export::classify(shirt.name()),
                    path: shirt,
                    ..file(&format!("{PLAYER}/shirt.dds"))
                },
            ],
            ingame_face: false,
            engine: Engine::Fox,
            combined: Vec::new(),
            common_models: Vec::new(),
            common_texture_stems: BTreeSet::new(),
            common_files: Vec::new(),
            hand_split: BTreeSet::new(),
            environment_map: false,
            refkit_body: false,
            textures: TextureHome::SharedOutput {
                package: ModelPackage::Gloves,
                id: 644,
            },
        };

        let package = run(TaskKind::Models {
            folder: folder.clone(),
            package: ModelPackage::Gloves,
            ids: vec![PackageKey::Id(644)],
            kits: Vec::new(),
        });
        let textures = run(TaskKind::Textures {
            folder,
            kits: Vec::new(),
        });

        assert!(package.messages.is_empty(), "{:?}", package.messages);
        assert_eq!(
            paths(&package),
            [
                "Asset/model/character/glove/g0644/#Win/glove.fpk",
                "Asset/model/character/glove/g0644/#Win/glove.fpkd",
            ]
        );
        let fpk = FpkFile::read(&package.entries[0].1).unwrap();
        let names: Vec<&str> = fpk.entries().map(|(name, _)| name).collect();
        assert_eq!(names, ["glove_l.fmdl"]);
        let directories = texture_directories(&fpk, "glove_l.fmdl");
        assert!(
            directories.contains(&(
                "shirt.dds".to_owned(),
                "/Assets/pes16/model/character/glove/g0644/".to_owned()
            )),
            "{directories:?}"
        );
        assert!(textures.messages.is_empty(), "{:?}", textures.messages);
        assert_eq!(
            paths(&textures),
            ["Asset/model/character/glove/g0644/#windx11/shirt.ftex"]
        );
    }

    #[test]
    fn a_folder_s_textures_are_converted_once_into_its_common_folder_for_its_packages() {
        let batch = run_task(
            TaskKind::Textures {
                folder: whole_player(),
                kits: Vec::new(),
            },
            PesVersion::Pes21,
            Some(TaskGroup {
                tasks: 0..4,
                charge: 0,
            }),
        );

        assert!(batch.messages.is_empty() && batch.uniparam.is_none());
        assert_eq!(
            batch.group,
            Some(0..4),
            "the batch carries its task's group"
        );
        assert_eq!(paths(&batch), [COMMON]);
        let dds = std::fs::read(tracer().join(format!("{PLAYER}/shirt.dds"))).unwrap();
        assert_eq!(
            batch.entries[0].1,
            ftex::dds_to_ftex(&dds, ftex::ColorSpace::Normal).unwrap()
        );
    }

    #[test]
    fn the_conversion_cache_engages_for_at_most_two_exports() {
        assert_eq!(cache_policy(0), CachePolicy::Use);
        assert_eq!(cache_policy(2), CachePolicy::Use);
        assert_eq!(cache_policy(3), CachePolicy::Bypass);
        assert_eq!(
            CompileContext::new(
                PesVersion::Pes21,
                48,
                Templates::embedded(),
                InstalledPaths::Unknown,
                EntryTarget::GamePaths {
                    engine: Engine::Fox,
                },
                MemoryBudget::new(usize::MAX),
                false,
            )
            .cache,
            CachePolicy::Bypass
        );
    }

    #[test]
    fn a_portrait_is_emitted_as_it_is_under_the_version_s_name() {
        let file = file(&format!("{PLAYER}/portrait.dds"));
        let source = std::fs::read(tracer().join(file.source.as_str())).unwrap();
        assert!(source.starts_with(b"DDS "), "the fixture is a DDS");
        for (version, path) in [
            (PesVersion::Pes21, "common/render/symbol/player/79205.dds"),
            (
                PesVersion::Pes18,
                "common/render/symbol/player/player_79205.dds",
            ),
        ] {
            let batch = run_for(
                TaskKind::Portrait {
                    player_id: 79205,
                    file: file.clone(),
                },
                version,
            );

            assert!(batch.messages.is_empty() && batch.uniparam.is_none());
            assert_eq!(paths(&batch), [path], "{version}");
            assert_eq!(batch.entries[0].1, source, "{version}");
        }
    }

    /// The logo task over the root files `main` and `small` (name, bytes), untagged, with their
    /// bytes.
    fn logo_task(main: (&str, Vec<u8>), small: Option<(&str, Vec<u8>)>) -> (TaskKind, TaskFiles) {
        let mut files = TaskFiles::new();
        let mut logo_file = |(name, bytes): (&str, Vec<u8>)| {
            let path = ScopePath::new(name).unwrap();
            files.insert(path.clone(), bytes);
            aesthetics_export::LogoFile {
                file: FileDescriptor {
                    size: 0,
                    kind: aesthetics_export::classify(path.name()),
                    source: path.clone(),
                    path,
                },
                fit: None,
            }
        };
        let logo = aesthetics_export::LogoFiles {
            main: logo_file(main),
            small: small.map(logo_file),
        };
        (TaskKind::Logo { logo }, files)
    }

    #[test]
    fn a_logo_task_notes_on_its_main_file_and_a_failed_one_drops_its_files_alone() {
        let main = || {
            let pixels = vec![255; 300 * 200 * 4];
            (
                "logo.png",
                dds_convert::encode_png(&pixels, 300, 200).unwrap(),
            )
        };
        let (kind, files) = logo_task(main(), None);
        let batch = process(kind, PesVersion::Pes19, None, files);

        assert_eq!(
            paths(&batch),
            [
                "common/render/symbol/flag/emblem_0792_r_ll.png",
                "common/render/symbol/flag/emblem_0792_r_l.png",
                "common/render/symbol/flag/emblem_0792_r.png",
            ]
        );
        let on_main = Scope::Folder {
            export_id: ExportId(4),
            path: ScopePath::new("logo.png").unwrap(),
        };
        let findings: Vec<(&str, Severity, Disposition, &Scope)> = batch
            .messages
            .iter()
            .map(|message| {
                (
                    message.code.code.as_ref(),
                    message.severity,
                    message.disposition,
                    &message.scope,
                )
            })
            .collect();
        assert_eq!(
            findings,
            [
                (
                    "logo_fit_applied",
                    Severity::Info,
                    Disposition::Keep,
                    &on_main
                ),
                (
                    "logo_upscaled",
                    Severity::Warning,
                    Disposition::Keep,
                    &on_main
                ),
            ]
        );

        // A small file that does not decode fails the task: no logo at all, reported on the
        // main file, without the notes about it.
        let (kind, files) = logo_task(main(), Some(("logo_small.png", b"not an image".to_vec())));
        let batch = process(kind, PesVersion::Pes21, None, files);

        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        let [message] = batch.messages.as_slice() else {
            panic!("{:?}", batch.messages);
        };
        assert_eq!(message.code.code, "folder_pack_failed");
        assert_eq!(message.disposition, Disposition::DropFile);
        assert_eq!(message.scope, on_main);
        assert!(
            message.context[0]
                .1
                .starts_with("cannot convert logo_small.png"),
            "{:?}",
            message.context
        );
    }

    /// The marker task over a root `ref_marker.dds` holding `bytes`, with its bytes.
    fn marker_task(bytes: Vec<u8>) -> (TaskKind, TaskFiles) {
        let path = ScopePath::new("ref_marker.dds").unwrap();
        let marker = FileDescriptor {
            size: 0,
            kind: aesthetics_export::classify(path.name()),
            source: path.clone(),
            path: path.clone(),
        };
        (
            TaskKind::RefereeMarker { marker },
            TaskFiles::from([(path, bytes)]),
        )
    }

    #[test]
    fn the_marker_task_writes_the_marker_texture_and_the_collar_naming_it_or_neither() {
        let dds = std::fs::read(tracer().join(format!("{PLAYER}/shirt.dds"))).unwrap();
        let (kind, files) = marker_task(dds.clone());
        let batch = process(kind, PesVersion::Pes21, None, files);

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(
            paths(&batch),
            [
                "Asset/model/character/common/999/sourceimages/#windx11/ref_marker.ftex",
                "Asset/model/character/uniform/nocloth/#Win/collar_077.fmdl",
            ]
        );
        let converted = texture::convert(
            &CompileContext::new(
                PesVersion::Pes21,
                1,
                Templates::embedded(),
                InstalledPaths::Unknown,
                EntryTarget::GamePaths {
                    engine: Engine::Fox,
                },
                MemoryBudget::new(usize::MAX),
                false,
            ),
            dds_convert::SourceFormat::Dds,
            "ref_marker.dds",
            &dds,
        )
        .unwrap();
        assert!(batch.entries[0].1 == converted, "converted as a texture");
        let collar = FmdlFile::read(&batch.entries[1].1).unwrap();
        let base = texture_paths(&collar).unwrap().remove(0);
        assert_eq!(
            (base.directory.as_str(), base.file_name.as_str()),
            (
                "/Assets/pes16/model/character/common/999/sourceimages/",
                "ref_marker.dds"
            )
        );

        // A codec the converter refuses: the texture's code on the file, and neither entry.
        let bc6h = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/textures/bc6h.dds"),
        )
        .unwrap();
        let (kind, files) = marker_task(bc6h);
        let batch = process(kind, PesVersion::Pes21, None, files);

        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        let [message] = batch.messages.as_slice() else {
            panic!("{:?}", batch.messages);
        };
        assert_eq!(message.code.code, "texture_codec_unsupported");
        assert_eq!(message.severity, Severity::Error);
        assert_eq!(message.disposition, Disposition::DropFile);
        assert_eq!(
            message.scope,
            Scope::Folder {
                export_id: ExportId(4),
                path: ScopePath::new("ref_marker.dds").unwrap(),
            }
        );
        assert_eq!(
            message.context,
            [("file".to_owned(), "ref_marker.dds".to_owned())]
        );
    }

    #[test]
    fn the_pre_fox_marker_task_writes_the_converted_texture_as_the_prop_s_alone() {
        let dds = std::fs::read(tracer().join(format!("{PLAYER}/shirt.dds"))).unwrap();
        let (kind, files) = marker_task(dds);
        let batch = process(kind, PesVersion::Pes17, None, files);

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(paths(&batch), [crate::paths::REFEREE_PROP_TEXTURE]);
        assert!(
            batch.entries[0].1.starts_with(b"DDS "),
            "the marker converted, a DDS"
        );
    }

    #[test]
    fn a_kit_commits_its_texture_and_its_config_as_entry_and_bin_entry() {
        let batch = run(TaskKind::Kit {
            slot: KitSlot::G1,
            kit: kit(true, Some(KitLayout::Fox)),
            edits: unknown_fpc(),
            coverage: ExportCoverage::Full,
        });

        assert_eq!(
            paths(&batch),
            [
                "Asset/model/character/uniform/texture/#windx11/u0792g1.ftex",
                "common/character0/model/character/uniform/team/792/792_DEF_GK1st_realUni.bin",
            ]
        );
        let (name, config) = batch.uniparam.as_ref().unwrap();
        assert_eq!(name, "792_DEF_GK1st_realUni.bin");
        assert_eq!(config, &batch.entries[1].1);
    }

    #[test]
    fn a_kit_without_config_toml_is_encoded_from_the_template() {
        let batch = run(TaskKind::Kit {
            slot: KitSlot::G1,
            kit: kit(false, None),
            edits: unknown_fpc(),
            coverage: ExportCoverage::Full,
        });

        let presence = TexturePresence {
            kit: true,
            ..TexturePresence::default()
        };
        let names = texture_names(792, KitSlot::G1, presence);
        let template = KitConfig::template().encode_with_names(PesVersion::Pes21, &names);
        assert_eq!(batch.uniparam.unwrap().1, template);
    }

    /// `kind`'s files read from the tracer folder, as `run_task` loads them.
    fn files_of(kind: &TaskKind) -> TaskFiles {
        kind.files()
            .into_iter()
            .map(|file| {
                let bytes = std::fs::read(tracer().join(file.source.as_str())).unwrap();
                (file.path.clone(), bytes)
            })
            .collect()
    }

    #[test]
    fn a_midcup_kit_s_config_names_the_number_textures_the_install_holds() {
        let task = TaskKind::Kit {
            slot: KitSlot::G1,
            kit: kit(true, None),
            edits: unknown_fpc(),
            coverage: ExportCoverage::Midcup,
        };
        let paths = ["u0792g1_back", "u0792g1_chest", "u0792g1_leg"]
            .map(|name| vtree::fold_name(&paths::kit_texture(Engine::Fox, name)));
        let installed = InstalledPaths::Known(crate::bins::installed::WalkedCpks::from(
            HashSet::from(paths),
        ));
        let files = files_of(&task);

        let batch = process_installed(task, PesVersion::Pes21, files, installed);

        // The three number textures the folder does not hold are still named, the
        // installed CPKs holding them; `u0792g1_name` is not, being installed nowhere.
        let names = texture_names(
            792,
            KitSlot::G1,
            TexturePresence {
                kit: true,
                back: true,
                chest: true,
                leg: true,
                ..TexturePresence::default()
            },
        );
        let config = std::fs::read_to_string(tracer().join("Kits/g1/config.toml")).unwrap();
        let expected = KitConfig::from_toml(&config)
            .unwrap()
            .encode_with_names(PesVersion::Pes21, &names);
        assert_eq!(batch.uniparam.unwrap().1, expected.to_vec());
    }

    /// `configured_kit()` with its main texture `main` and, when given, the map
    /// `(stem, bytes)` (`kit_mask`, `kit_srm`), processed for `version`.
    fn kit_with_map(version: PesVersion, main: &[u8], map: Option<(&str, &[u8])>) -> TaskBatch {
        let mut kit = configured_kit();
        let mut files: TaskFiles = ["Kits/g1/config.toml", "Kits/g1/colors.txt"]
            .into_iter()
            .map(|path| {
                let bytes = std::fs::read(tracer().join(path)).unwrap();
                (ScopePath::new(path).unwrap(), bytes)
            })
            .collect();
        files.insert(ScopePath::new("Kits/g1/kit.dds").unwrap(), main.to_vec());
        if let Some((stem, bytes)) = map {
            let path = format!("Kits/g1/{stem}.dds");
            kit.textures.push(KitTexture {
                stem: stem.to_owned(),
                file: file(&path),
                source: KitTextureSource::Own,
            });
            files.insert(ScopePath::new(&path).unwrap(), bytes.to_vec());
        }
        process(g1(kit), version, None, files)
    }

    #[test]
    fn a_pre_fox_kit_emits_its_own_mask_converted_or_the_template_as_it_is_and_no_bin_entry() {
        let tracer_kit = std::fs::read(tracer().join("Kits/g1/kit.dds")).unwrap();
        let green = solid_bc1_dds(0x07e0);

        let templated = kit_with_map(PesVersion::Pes17, &tracer_kit, None);
        let own = kit_with_map(PesVersion::Pes17, &tracer_kit, Some(("kit_mask", &green)));
        // The same source as a main texture, for the conversion the mask must go through.
        let green_main = kit_with_map(PesVersion::Pes17, &green, None);

        for batch in [&templated, &own] {
            assert_eq!(
                paths(batch),
                [
                    "common/character0/model/character/uniform/texture/u0792g1.dds",
                    "common/character0/model/character/uniform/texture/u0792g1_mask.dds",
                    "common/character0/model/character/uniform/team/792/792_DEF_GK1st_realUni.bin",
                ]
            );
            assert_eq!(kit_findings(batch), []);
            // PES 15-17 have no UniformParameter.bin: the loose config is the kit's only one.
            assert!(batch.uniparam.is_none());
        }
        assert!(templated.entries[1].1 == Templates::embedded().kit_mask());
        assert!(own.entries[1].1 == green_main.entries[0].1);
        assert!(own.entries[1].1 != Templates::embedded().kit_mask());
    }

    #[test]
    fn a_fox_kit_emits_its_own_srm_and_no_template_when_it_has_none() {
        let tracer_kit = std::fs::read(tracer().join("Kits/g1/kit.dds")).unwrap();
        let green = solid_bc1_dds(0x07e0);

        let with_srm = kit_with_map(PesVersion::Pes21, &tracer_kit, Some(("kit_srm", &green)));
        let without = kit_with_map(PesVersion::Pes21, &tracer_kit, None);
        let green_main = kit_with_map(PesVersion::Pes21, &green, None);

        assert_eq!(
            paths(&with_srm),
            [
                "Asset/model/character/uniform/texture/#windx11/u0792g1.ftex",
                "Asset/model/character/uniform/texture/#windx11/u0792g1_srm.ftex",
                "common/character0/model/character/uniform/team/792/792_DEF_GK1st_realUni.bin",
            ]
        );
        assert!(with_srm.entries[1].1 == green_main.entries[0].1);
        assert_eq!(
            paths(&without),
            [
                "Asset/model/character/uniform/texture/#windx11/u0792g1.ftex",
                "common/character0/model/character/uniform/team/792/792_DEF_GK1st_realUni.bin",
            ]
        );
        for batch in [&with_srm, &without] {
            assert_eq!(kit_findings(batch), []);
            assert!(batch.uniparam.is_some());
        }
    }

    /// The tracer's `g1` kit folder with its `colors.txt` when `colors`, the icon marker
    /// `icon`, and the effective `textures`; no config.
    fn colored_kit(colors: bool, icon: Option<u8>, textures: Vec<KitTexture>) -> KitFolder {
        KitFolder {
            colors: colors.then(|| file("Kits/g1/colors.txt")),
            icon,
            textures,
            ..kit(false, None)
        }
    }

    /// The kit's own main texture, the tracer's `kit.dds` unless `run_with` replaces it.
    fn own_main_texture() -> Vec<KitTexture> {
        vec![KitTexture {
            stem: "kit".to_owned(),
            file: file("Kits/g1/kit.dds"),
            source: KitTextureSource::Own,
        }]
    }

    /// The kit task of `g1` over `kit`, its team's kit-FPC status unknown.
    fn g1(kit: KitFolder) -> TaskKind {
        g1_with(kit, EffectiveTeamKitFpc::Unknown)
    }

    /// The kit task of `g1` over `kit`, its team's kit-FPC status `fpc`, the team holding no
    /// collar.
    fn g1_with(kit: KitFolder, fpc: EffectiveTeamKitFpc) -> TaskKind {
        g1_wearing(kit, fpc, None)
    }

    /// The kit task of `g1` over `kit`, its team's kit-FPC status `fpc` and its collar
    /// `collar`, compiled for a `Full` export.
    fn g1_wearing(kit: KitFolder, fpc: EffectiveTeamKitFpc, collar: Option<u8>) -> TaskKind {
        TaskKind::Kit {
            slot: KitSlot::G1,
            kit,
            edits: TeamKitEdits { fpc, collar },
            coverage: ExportCoverage::Full,
        }
    }

    /// The edits of a team whose kit-FPC status is unknown and that holds no collar.
    fn unknown_fpc() -> TeamKitEdits {
        TeamKitEdits {
            fpc: EffectiveTeamKitFpc::Unknown,
            collar: None,
        }
    }

    /// The (code, severity, disposition) of each of the batch's messages, asserting each is on
    /// the kit folder with no context.
    fn kit_findings(batch: &TaskBatch) -> Vec<(&str, Severity, Disposition)> {
        batch
            .messages
            .iter()
            .map(|message| {
                assert_eq!(
                    message.scope,
                    Scope::Folder {
                        export_id: ExportId(4),
                        path: ScopePath::new("Kits/g1").unwrap(),
                    }
                );
                assert!(message.context.is_empty(), "{:?}", message.context);
                (
                    message.code.code.as_ref(),
                    message.severity,
                    message.disposition,
                )
            })
            .collect()
    }

    /// The batch's `UniColor.bin` entry, asserting it is team 792's.
    fn uni_color(batch: &TaskBatch) -> &KitColorEntry {
        let (team_id, entry) = batch.uni_color.as_ref().expect("a kit's entry");
        assert_eq!(*team_id, 792);
        entry
    }

    /// A 64x64 BC1 DDS, one level, every pixel the opaque 5:6:5 color `color`.
    fn solid_bc1_dds(color: u16) -> Vec<u8> {
        let [low, high] = color.to_le_bytes();
        // Both endpoints the one color and every index 0: each pixel is the first endpoint.
        let block = [low, high, low, high, 0, 0, 0, 0];
        let decoded = dds_convert::Decoded {
            width: 64,
            height: 64,
            mips: vec![vec![0; 64 * 64 * 4]],
            blocks: Some(dds_convert::Blocks {
                codec: dds_convert::BlockCodec::Bc1,
                mips: vec![block.repeat(16 * 16)],
            }),
            authored_mips: true,
        };
        dds_convert::encode_dds(&decoded, dds_convert::BlockCodec::Bc1).unwrap()
    }

    /// What `extract_kit_colors` gives for the tracer's `kit.dds`, decoded.
    fn tracer_kit_colors() -> [[u8; 3]; 2] {
        let bytes = std::fs::read(tracer().join("Kits/g1/kit.dds")).unwrap();
        let decoded = dds_convert::decode(&bytes, dds_convert::SourceFormat::Dds).unwrap();
        let colors =
            color_tools::kit::extract_kit_colors(&decoded.mips[0], decoded.width, decoded.height)
                .expect("the tracer's kit texture gives colors");
        [colors.color1, colors.color2]
    }

    #[test]
    fn a_kit_s_two_colors_and_icon_marker_make_its_uni_color_entry_with_no_finding() {
        let batch = run(g1(colored_kit(true, Some(7), own_main_texture())));

        assert_eq!(kit_findings(&batch), []);
        assert_eq!(
            uni_color(&batch),
            &KitColorEntry {
                kit: 0x10,
                icon: 7,
                colors: [[0xc1, 0x12, 0x00], [0x41, 0x41, 0x41]],
            }
        );
    }

    /// The tracer's `g1` kit folder with its `config.toml` and its two-color `colors.txt`, so
    /// the only finding its task can make is about its config.
    fn configured_kit() -> KitFolder {
        KitFolder {
            config: Some(file("Kits/g1/config.toml")),
            ..colored_kit(true, None, own_main_texture())
        }
    }

    /// A kit config carrying shirt model 144 and the template's values otherwise.
    const SHIRT_144: &[u8] = b"[shirt]\nmodel = 144\n";

    /// The four FPC fields of the kit config the batch emitted, decoded from its 120 bytes:
    /// shirt model, shorts model, collar, winter collar.
    fn emitted_fpc_fields(batch: &TaskBatch) -> (u8, u8, u8, u8) {
        let (_, bytes) = batch.uniparam.as_ref().expect("a kit's config");
        let config = KitConfig::decode(bytes, PesVersion::Pes21).unwrap();
        (
            config.shirt.model,
            config.shorts.model,
            config.shirt.collar,
            config.shirt.winter_collar,
        )
    }

    #[test]
    fn with_fpc_on_a_supplied_config_lacking_the_fpc_values_gets_them_and_says_so() {
        let batch = run_with(
            g1_with(configured_kit(), EffectiveTeamKitFpc::On),
            &[("Kits/g1/config.toml", SHIRT_144)],
        );

        assert_eq!(
            kit_findings(&batch),
            [("kit_config_fpc_adjusted", Severity::Info, Disposition::Keep)]
        );
        assert_eq!(emitted_fpc_fields(&batch), (176, 16, 105, 105));
    }

    #[test]
    fn with_fpc_on_a_config_already_carrying_the_fpc_values_is_not_reported() {
        // The tracer's own config carries them; the template does too.
        for kit in [
            configured_kit(),
            colored_kit(true, None, own_main_texture()),
        ] {
            let batch = run(g1_with(kit, EffectiveTeamKitFpc::On));

            assert_eq!(kit_findings(&batch), []);
            assert_eq!(emitted_fpc_fields(&batch), (176, 16, 105, 105));
        }
    }

    #[test]
    fn with_the_fpc_status_unknown_a_supplied_config_is_encoded_as_it_is() {
        let batch = run_with(g1(configured_kit()), &[("Kits/g1/config.toml", SHIRT_144)]);

        assert_eq!(kit_findings(&batch), []);
        assert_eq!(emitted_fpc_fields(&batch), (144, 16, 105, 105));
    }

    #[test]
    fn a_kit_whose_collar_or_winter_collar_is_the_referees_is_left_out_naming_the_field() {
        let configs: [(&[u8], &str); 3] = [
            (b"[shirt]\ncollar = 77\n", "collar"),
            (b"[shirt]\nwinter_collar = 77\n", "winter_collar"),
            (b"[shirt]\ncollar = 77\nwinter_collar = 77\n", "collar"),
        ];
        for (config, field) in configs {
            let batch = run_with(g1(configured_kit()), &[("Kits/g1/config.toml", config)]);

            assert!(
                batch.entries.is_empty() && batch.uniparam.is_none() && batch.uni_color.is_none(),
                "{field}: nothing of the kit commits"
            );
            let [message] = batch.messages.as_slice() else {
                panic!("{field}: {:?}", batch.messages);
            };
            assert_eq!(
                (
                    message.code.code.as_ref(),
                    message.severity,
                    message.disposition
                ),
                (
                    "kit_collar_reserved",
                    Severity::Error,
                    Disposition::DropFolder
                ),
                "{field}"
            );
            assert_eq!(
                message.scope,
                Scope::Folder {
                    export_id: ExportId(4),
                    path: ScopePath::new("Kits/g1").unwrap(),
                }
            );
            assert_eq!(message.context, [("field".to_owned(), field.to_owned())]);
        }
    }

    #[test]
    fn with_fpc_on_a_config_naming_collar_77_compiles_with_the_fpc_collar() {
        let batch = run_with(
            g1_with(configured_kit(), EffectiveTeamKitFpc::On),
            &[("Kits/g1/config.toml", b"[shirt]\ncollar = 77\n")],
        );

        assert_eq!(
            kit_findings(&batch),
            [("kit_config_fpc_adjusted", Severity::Info, Disposition::Keep)]
        );
        assert_eq!(emitted_fpc_fields(&batch), (176, 16, 105, 105));
    }

    #[test]
    fn the_team_s_collar_is_set_after_the_fpc_values_and_a_config_naming_77_wears_it() {
        let batch = run_with(
            g1_wearing(configured_kit(), EffectiveTeamKitFpc::On, Some(12)),
            &[("Kits/g1/config.toml", b"[shirt]\ncollar = 77\n")],
        );

        assert_eq!(
            kit_findings(&batch),
            [("kit_config_fpc_adjusted", Severity::Info, Disposition::Keep)]
        );
        assert_eq!(emitted_fpc_fields(&batch), (176, 16, 12, 12));
        // The loose config is the bin entry's bytes.
        let (_, config) = batch.uniparam.as_ref().unwrap();
        assert_eq!(&batch.entries[1].1, config);
    }

    #[test]
    fn a_collar_task_writes_its_file_unchanged_as_the_stock_collar_it_replaces() {
        let bytes = std::fs::read(tracer().join(format!("{PLAYER}/glove_r.fmdl"))).unwrap();
        let path = ScopePath::new("Collars/collar_12.fmdl").unwrap();
        let file = FileDescriptor {
            size: 0,
            kind: aesthetics_export::classify(path.name()),
            source: path.clone(),
            path: path.clone(),
        };

        let batch = process(
            TaskKind::Collar { file, id: 12 },
            PesVersion::Pes21,
            None,
            TaskFiles::from([(path, bytes.clone())]),
        );

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert!(batch.uniparam.is_none() && batch.uni_color.is_none());
        assert_eq!(
            paths(&batch),
            ["Asset/model/character/uniform/nocloth/#Win/collar_012.fmdl"]
        );
        assert!(batch.entries[0].1 == bytes, "the file's bytes as they are");
    }

    #[test]
    fn a_pes_17_collar_task_writes_its_model_unchanged_at_the_pre_fox_nocloth_path() {
        // Konami's WESYS-wrapped shirt model from `pes_model`'s fixtures: a `.model` that reads.
        let bytes = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../libs/pes_model/tests/fixtures/konami_collar_052.wesys.model"),
        )
        .unwrap();
        let path = ScopePath::new("Collars/collar_12.model").unwrap();
        let file = FileDescriptor {
            size: 0,
            kind: aesthetics_export::classify(path.name()),
            source: path.clone(),
            path: path.clone(),
        };

        let batch = process(
            TaskKind::Collar { file, id: 12 },
            PesVersion::Pes17,
            None,
            TaskFiles::from([(path, bytes.clone())]),
        );

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert!(batch.uniparam.is_none() && batch.uni_color.is_none());
        assert_eq!(
            paths(&batch),
            ["common/character0/model/character/uniform/nocloth/collar_012.model"]
        );
        assert!(batch.entries[0].1 == bytes, "the file's bytes as they are");
    }

    #[test]
    fn a_kit_whose_collar_is_76_compiles_as_it_is() {
        let batch = run_with(
            g1(configured_kit()),
            &[(
                "Kits/g1/config.toml",
                b"[shirt]\ncollar = 76\nwinter_collar = 76\n",
            )],
        );

        assert_eq!(kit_findings(&batch), []);
        let (_, _, collar, winter_collar) = emitted_fpc_fields(&batch);
        assert_eq!((collar, winter_collar), (76, 76));
    }

    #[test]
    fn a_kit_without_an_icon_marker_gets_icon_3() {
        let batch = run(g1(colored_kit(true, None, own_main_texture())));

        assert_eq!(uni_color(&batch).icon, 3);
    }

    #[test]
    fn a_kit_without_colors_txt_takes_its_colors_from_its_main_texture() {
        // 5:6:5 red 24, green 20, blue 10: 197, 81, 82 once scaled to eight bits (24 * 255 / 31,
        // 20 * 255 / 63, 10 * 255 / 31, rounded).
        let solid = solid_bc1_dds(0xc28a);
        let batch = run_with(
            g1(colored_kit(false, Some(11), own_main_texture())),
            &[("Kits/g1/kit.dds", &solid)],
        );

        assert_eq!(
            kit_findings(&batch),
            [("kit_colors_derived", Severity::Info, Disposition::Keep)]
        );
        assert_eq!(uni_color(&batch).colors, [[0xc5, 0x51, 0x52]; 2]);
    }

    #[test]
    fn a_kit_whose_colors_txt_gives_one_valid_color_takes_both_from_its_main_texture() {
        let batch = run_with(
            g1(colored_kit(true, Some(11), own_main_texture())),
            &[("Kits/g1/colors.txt", b"#c11200\nnot a color\n")],
        );

        assert_eq!(
            kit_findings(&batch),
            [("kit_colors_derived", Severity::Info, Disposition::Keep)]
        );
        assert_eq!(uni_color(&batch).colors, tracer_kit_colors());
    }

    #[test]
    fn a_main_texture_inherited_from_all_gives_the_colors() {
        let inherited = vec![KitTexture {
            stem: "kit".to_owned(),
            file: FileDescriptor {
                path: ScopePath::new("Kits/all/kit.dds").unwrap(),
                ..file("Kits/g1/kit.dds")
            },
            source: KitTextureSource::Shared,
        }];
        let batch = run(g1(colored_kit(false, Some(11), inherited)));

        assert_eq!(
            kit_findings(&batch),
            [("kit_colors_derived", Severity::Info, Disposition::Keep)]
        );
        assert_eq!(uni_color(&batch).colors, tracer_kit_colors());
    }

    #[test]
    fn a_placeholder_kit_without_colors_gets_magenta_and_black() {
        let batch = run(g1(colored_kit(false, None, Vec::new())));

        assert_eq!(
            kit_findings(&batch),
            [("kit_colors_missing", Severity::Warning, Disposition::Keep)]
        );
        assert_eq!(
            uni_color(&batch),
            &KitColorEntry {
                kit: 0x10,
                icon: 3,
                colors: [[255, 0, 255], [0, 0, 0]],
            }
        );
    }

    #[test]
    fn a_placeholder_kit_with_two_colors_in_colors_txt_takes_them_and_reports_nothing() {
        let batch = run(g1(colored_kit(true, Some(11), Vec::new())));

        assert_eq!(kit_findings(&batch), []);
        assert_eq!(
            uni_color(&batch).colors,
            [[0xc1, 0x12, 0x00], [0x41, 0x41, 0x41]]
        );
    }

    /// The tracer's `g1` kit with its two colors and the layout marker `layout`, over the
    /// effective `textures`, so the only finding its task can make is about its layout.
    fn marked_kit(layout: Option<KitLayout>, textures: Vec<KitTexture>) -> KitFolder {
        KitFolder {
            layout,
            ..colored_kit(true, Some(11), textures)
        }
    }

    /// What the batch emitted as the kit's main texture, `u0792g1`.
    fn main_texture(batch: &TaskBatch) -> &[u8] {
        let (path, bytes) = &batch.entries[0];
        assert_eq!(
            path,
            "Asset/model/character/uniform/texture/#windx11/u0792g1.ftex"
        );
        bytes
    }

    #[test]
    fn a_kit_marked_pre_fox_on_a_fox_target_is_re_laid_and_says_so() {
        let batch = run(g1(marked_kit(Some(KitLayout::PreFox), own_main_texture())));

        let [message] = batch.messages.as_slice() else {
            panic!("{:?}", batch.messages);
        };
        assert_eq!(message.code.code, "kit_layout_converted");
        assert_eq!(message.severity, Severity::Info);
        assert_eq!(message.disposition, Disposition::Keep);
        assert_eq!(
            message.scope,
            Scope::Folder {
                export_id: ExportId(4),
                path: ScopePath::new("Kits/g1").unwrap(),
            }
        );
        assert_eq!(
            message.context,
            [
                ("from".to_owned(), "pre-fox".to_owned()),
                ("to".to_owned(), "fox".to_owned())
            ]
        );
        let source = std::fs::read(tracer().join("Kits/g1/kit.dds")).unwrap();
        let decoded = dds_convert::decode(&source, dds_convert::SourceFormat::Dds).unwrap();
        let target = dds_convert::Target {
            version: PesVersion::Pes21,
            role: dds_convert::TextureRole::Color,
        };
        let relaid = dds_convert::convert(
            &kit_layout::relaid(&MemoryBudget::new(usize::MAX), &decoded, KitLayout::PreFox)
                .unwrap(),
            target,
        )
        .unwrap();
        assert_eq!(main_texture(&batch), relaid);
        assert_ne!(relaid, dds_convert::convert(&decoded, target).unwrap());
    }

    #[test]
    fn a_kit_marked_fox_or_unmarked_on_a_fox_target_is_converted_as_it_is() {
        let unmarked = run(g1(marked_kit(None, own_main_texture())));
        let fox = run(g1(marked_kit(Some(KitLayout::Fox), own_main_texture())));

        for batch in [&unmarked, &fox] {
            assert_eq!(kit_findings(batch), []);
        }
        let source = std::fs::read(tracer().join("Kits/g1/kit.dds")).unwrap();
        let as_it_is = dds_convert::convert(
            &dds_convert::decode(&source, dds_convert::SourceFormat::Dds).unwrap(),
            dds_convert::Target {
                version: PesVersion::Pes21,
                role: dds_convert::TextureRole::Color,
            },
        )
        .unwrap();
        assert_eq!(main_texture(&unmarked), as_it_is);
        assert_eq!(fox.entries, unmarked.entries);
    }

    #[test]
    fn a_placeholder_kit_marked_pre_fox_is_not_re_laid() {
        let marked = run(g1(marked_kit(Some(KitLayout::PreFox), Vec::new())));
        let unmarked = run(g1(marked_kit(None, Vec::new())));

        assert_eq!(kit_findings(&marked), []);
        assert_eq!(marked.entries, unmarked.entries);
    }

    /// One opaque color per digit, every channel 0, 128 or 255, which the block codecs keep
    /// within `CODEC_TOLERANCE`.
    const DIGIT_COLORS: [[u8; 4]; 10] = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
        [255, 0, 255, 255],
        [0, 255, 255, 255],
        [255, 255, 255, 255],
        [128, 0, 0, 255],
        [0, 128, 0, 255],
        [0, 0, 128, 255],
    ];

    /// How far a channel of a compiled texel may be from the color drawn: a block codec's
    /// 5:6:5 endpoints move 128 by up to 4.
    const CODEC_TOLERANCE: u8 = 8;

    /// The middle of each digit's slot along a 2048-long PES 18-21 row, written out apart from
    /// the table so a wrong table entry shows.
    const SLOT_MIDDLES_2048: [u32; 10] = [80, 265, 420, 620, 820, 1020, 1220, 1420, 1620, 1820];

    /// A `width` x `height` BC1 DDS with its generated mip chain, its texel at (x, y)
    /// `color(x, y)`.
    fn bc1_dds(width: u32, height: u32, color: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                pixels.extend_from_slice(&color(x, y));
            }
        }
        let decoded = dds_convert::Decoded {
            width,
            height,
            mips: vec![pixels],
            blocks: None,
            authored_mips: false,
        };
        dds_convert::encode_dds(&decoded, dds_convert::BlockCodec::Bc1).unwrap()
    }

    /// The tracer's `g1` kit with its two colors, its main texture, and each texture of
    /// `extra` (stem, DDS bytes) as its own `Kits/g1/<stem>.dds`, processed for PES 21.
    fn run_with_kit_textures(extra: &[(&str, &[u8])]) -> TaskBatch {
        let mut textures = own_main_texture();
        let mut files: TaskFiles = ["Kits/g1/kit.dds", "Kits/g1/colors.txt"]
            .into_iter()
            .map(|path| {
                let bytes = std::fs::read(tracer().join(path)).unwrap();
                (ScopePath::new(path).unwrap(), bytes)
            })
            .collect();
        for (stem, bytes) in extra {
            let path = format!("Kits/g1/{stem}.dds");
            textures.push(KitTexture {
                stem: (*stem).to_owned(),
                file: file(&path),
                source: KitTextureSource::Own,
            });
            files.insert(ScopePath::new(&path).unwrap(), bytes.to_vec());
        }
        process(
            g1(colored_kit(true, Some(11), textures)),
            PesVersion::Pes21,
            None,
            files,
        )
    }

    /// The bytes of the batch's kit texture entry `u0792g1<suffix>`.
    fn kit_entry<'a>(batch: &'a TaskBatch, suffix: &str) -> &'a [u8] {
        let path = format!("Asset/model/character/uniform/texture/#windx11/u0792g1{suffix}.ftex");
        let (_, bytes) = batch
            .entries
            .iter()
            .find(|(entry, _)| *entry == path)
            .unwrap_or_else(|| panic!("no entry {path} in {:?}", paths(batch)));
        bytes
    }

    #[test]
    fn a_column_number_atlas_on_a_fox_target_becomes_a_row_with_its_digits_in_order() {
        let column = bc1_dds(128, 2048, |_, y| DIGIT_COLORS[(y * 10 / 2048) as usize]);

        let batch = run_with_kit_textures(&[("kit_back", &column)]);

        assert_eq!(kit_findings(&batch), []);
        let row = dds_convert::decode(kit_entry(&batch, "_back"), dds_convert::SourceFormat::Ftex)
            .unwrap();
        assert_eq!((row.width, row.height), (2048, 256));
        for (digit, x) in SLOT_MIDDLES_2048.into_iter().enumerate() {
            let at = ((128 * 2048 + x) * 4) as usize;
            let texel = &row.mips[0][at..at + 4];
            let near = (0..4).all(|c| texel[c].abs_diff(DIGIT_COLORS[digit][c]) <= CODEC_TOLERANCE);
            assert!(near, "digit {digit} at x {x}: {texel:?}");
        }
    }

    #[test]
    fn a_row_number_atlas_and_a_tall_name_atlas_on_a_fox_target_convert_as_they_are() {
        let row = bc1_dds(2048, 256, |x, _| DIGIT_COLORS[(x * 10 / 2048) as usize]);
        let name = bc1_dds(64, 256, |_, y| DIGIT_COLORS[(y / 64) as usize]);

        let batch = run_with_kit_textures(&[("kit_back", &row), ("kit_name", &name)]);

        let context = CompileContext::new(
            PesVersion::Pes21,
            1,
            Templates::embedded(),
            InstalledPaths::Unknown,
            EntryTarget::GamePaths {
                engine: Engine::Fox,
            },
            MemoryBudget::new(usize::MAX),
            false,
        );
        let as_it_is = |file_name: &str, bytes: &[u8]| {
            texture::convert(&context, dds_convert::SourceFormat::Dds, file_name, bytes).unwrap()
        };
        assert_eq!(kit_entry(&batch, "_back"), as_it_is("kit_back.dds", &row));
        assert_eq!(kit_entry(&batch, "_name"), as_it_is("kit_name.dds", &name));
    }

    /// The export file at `path`, described as the structure pass would, whose bytes `run`
    /// reads from the tracer's player file `tracer_name`.
    fn named(path: &str, tracer_name: &str) -> FileDescriptor {
        let path = ScopePath::new(path).unwrap();
        FileDescriptor {
            kind: aesthetics_export::classify(path.name()),
            path,
            ..file(&format!("{PLAYER}/{tracer_name}"))
        }
    }

    /// The tracer's player folder holding `files` and combining the shared folders
    /// `combined`, its textures going to its common subfolder.
    fn player_with(files: Vec<FileDescriptor>, combined: Vec<CombinedFolder>) -> ModelFolder {
        ModelFolder {
            combined,
            files,
            ..player(&[])
        }
    }

    /// The shared folder at `path` (`Boots/Crocs`) holding `files`, combined into `package`.
    fn shared(package: ModelPackage, path: &str, files: Vec<FileDescriptor>) -> CombinedFolder {
        let path = ScopePath::new(path).unwrap();
        CombinedFolder {
            package,
            folder: SharedModelFolder {
                folder_name: path.name().to_owned(),
                path,
                files,
            },
        }
    }

    /// The boots task over the player folder `folder` under id 3745.
    fn boots(folder: ModelFolder) -> TaskKind {
        TaskKind::Models {
            folder,
            package: ModelPackage::Boots,
            ids: vec![PackageKey::Id(3745)],
            kits: Vec::new(),
        }
    }

    /// The tracer's model `name`, decoded.
    fn tracer_model(name: &str) -> Model {
        let bytes = std::fs::read(tracer().join(format!("{PLAYER}/{name}"))).unwrap();
        Model::from_file(&FmdlFile::read(&bytes).unwrap()).unwrap()
    }

    /// The package's model `name`, decoded.
    fn packed_model(package: &FpkFile, name: &str) -> Model {
        Model::from_file(&FmdlFile::read(package.get(name).unwrap()).unwrap()).unwrap()
    }

    /// The batch's one message as (code, severity, disposition, context), asserting it is on
    /// the tracer's player folder.
    fn one_message(batch: &TaskBatch) -> (&str, Severity, Disposition, &[(String, String)]) {
        let [message] = batch.messages.as_slice() else {
            panic!("{:?}", batch.messages);
        };
        assert_eq!(
            message.scope,
            Scope::Folder {
                export_id: ExportId(4),
                path: ScopePath::new(PLAYER).unwrap(),
            }
        );
        (
            &message.code.code,
            message.severity,
            message.disposition,
            &message.context,
        )
    }

    #[test]
    fn two_boots_parts_merge_into_one_model_with_the_standard_skeleton_and_the_merge_is_noted() {
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl"),
                named(&format!("{PLAYER}/a_boots.fmdl"), "boots.fmdl"),
                named(&format!("{PLAYER}/shirt.dds"), "shirt.dds"),
            ],
            Vec::new(),
        );

        let batch = run(boots(folder));

        assert_eq!(
            one_message(&batch),
            (
                "fmdl_merged",
                Severity::Info,
                Disposition::Keep,
                &[("model".to_owned(), "boots.fmdl".to_owned())][..]
            )
        );
        assert_eq!(
            paths(&batch),
            [
                "Asset/model/character/boots/k3745/#Win/boots.fpk",
                "Asset/model/character/boots/k3745/#Win/boots.fpkd",
            ]
        );
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(names, ["boots.fmdl", "boots.skl"]);
        assert_eq!(
            package.get("boots.skl").unwrap(),
            Templates::embedded().body_skeleton()
        );
        let merged = packed_model(&package, "boots.fmdl");
        let part = tracer_model("boots.fmdl");
        assert_eq!(merged.meshes.len(), 2 * part.meshes.len());
        assert_eq!(
            merged.bones.len(),
            part.bones.len(),
            "one skeleton, unioned by name"
        );
        // The merged model's own texture points at the player's common folder.
        assert_rewritten(&texture_directories(&package, "boots.fmdl"));
    }

    #[test]
    fn parts_merge_in_case_folded_name_order_whatever_the_folder_s_order() {
        // `B_boots` is the tracer's boots with its first material renamed, so the merged
        // model's first material says which part came first: `a_boots` folds before
        // `B_boots`, though byte order would put `B` first.
        let mut renamed = tracer_model("boots.fmdl");
        renamed.materials[0].name = "kit_b".to_owned();
        let renamed = renamed.to_file().unwrap().write();
        let b_boots = ScopePath::new(&format!("{PLAYER}/B_boots.fmdl")).unwrap();
        let folder = player_with(
            vec![
                FileDescriptor {
                    kind: aesthetics_export::classify(b_boots.name()),
                    path: b_boots.clone(),
                    ..file(&format!("{PLAYER}/boots.fmdl"))
                },
                named(&format!("{PLAYER}/a_boots.fmdl"), "boots.fmdl"),
            ],
            Vec::new(),
        );
        let kind = boots(folder);
        let mut files: TaskFiles = kind
            .files()
            .into_iter()
            .map(|file| {
                let bytes = std::fs::read(tracer().join(file.source.as_str())).unwrap();
                (file.path.clone(), bytes)
            })
            .collect();
        files.insert(b_boots, renamed);

        let batch = process(kind, PesVersion::Pes21, None, files);

        assert_eq!(one_message(&batch).0, "fmdl_merged");
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let merged = packed_model(&package, "boots.fmdl");
        let names: Vec<&str> = merged
            .materials
            .iter()
            .map(|material| material.name.as_str())
            .collect();
        assert_eq!(names, ["kit", "shirt", "shirt antiblur", "kit_b"]);
    }

    #[test]
    fn parts_with_one_skeleton_pack_it_and_a_part_without_one_beside_one_with_is_a_conflict() {
        // One custom skeleton under both parts' names.
        let custom = other_skeleton();
        let kit_boots_skl = format!("{PLAYER}/kit_boots.skl");
        let a_boots_skl = format!("{PLAYER}/a_boots.skl");
        let both = player_with(
            vec![
                named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl"),
                named(&kit_boots_skl, "fcl_hair.skl"),
                named(&format!("{PLAYER}/a_boots.fmdl"), "boots.fmdl"),
                named(&a_boots_skl, "fcl_hair.skl"),
            ],
            Vec::new(),
        );
        let batch = run_with(
            boots(both),
            &[(&kit_boots_skl, &custom), (&a_boots_skl, &custom)],
        );
        assert_eq!(one_message(&batch).0, "fmdl_merged");
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        assert_eq!(package.get("boots.skl").unwrap(), custom);

        let one = player_with(
            vec![
                named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl"),
                named(&format!("{PLAYER}/kit_boots.skl"), "fcl_hair.skl"),
                named(&format!("{PLAYER}/a_boots.fmdl"), "boots.fmdl"),
            ],
            Vec::new(),
        );
        let batch = run(boots(one));
        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        assert_eq!(
            one_message(&batch),
            (
                "skl_merge_conflict",
                Severity::Error,
                Disposition::DropFolder,
                &[("skeleton".to_owned(), "differs".to_owned())][..]
            )
        );
    }

    #[test]
    fn a_boots_subfolder_s_skeleton_pairs_with_the_model_beside_it_and_a_root_part_keeps_its_own() {
        let custom = other_skeleton();
        let sub_skl = format!("{PLAYER}/boots/kit_boots.skl");
        let alone = player_with(
            vec![
                named(&format!("{PLAYER}/boots/kit_boots.fmdl"), "boots.fmdl"),
                named(&sub_skl, "fcl_hair.skl"),
            ],
            Vec::new(),
        );
        let batch = run_with(boots(alone), &[(&sub_skl, &custom)]);
        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(names, ["boots.fmdl", "boots.skl"]);
        assert_eq!(package.get("boots.skl").unwrap(), custom);

        // A root part of the same name beside it: each pairs the skeleton in its own
        // directory, and the two, one file, merge without a conflict.
        let root_skl = format!("{PLAYER}/kit_boots.skl");
        let both = player_with(
            vec![
                named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl"),
                named(&root_skl, "fcl_hair.skl"),
                named(&format!("{PLAYER}/boots/kit_boots.fmdl"), "boots.fmdl"),
                named(&sub_skl, "fcl_hair.skl"),
            ],
            Vec::new(),
        );
        let batch = run_with(boots(both), &[(&root_skl, &custom), (&sub_skl, &custom)]);
        assert_eq!(one_message(&batch).0, "fmdl_merged");
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        assert_eq!(package.get("boots.skl").unwrap(), custom);
        assert_eq!(
            packed_model(&package, "boots.fmdl").meshes.len(),
            2 * tracer_model("boots.fmdl").meshes.len()
        );
    }

    #[test]
    fn parts_disagreeing_on_a_bone_or_a_material_fail_the_package_with_the_conflict_s_code() {
        // The tracer's left glove as a boots part: its `sk_forearm_l` differs from the boots'.
        let bone = player_with(
            vec![
                named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl"),
                named(&format!("{PLAYER}/a_boots.fmdl"), "glove_l.fmdl"),
            ],
            Vec::new(),
        );
        let batch = run(boots(bone));
        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        assert_eq!(
            one_message(&batch),
            (
                "skl_merge_conflict",
                Severity::Error,
                Disposition::DropFolder,
                &[("bone".to_owned(), "sk_forearm_l".to_owned())][..]
            )
        );

        // The tracer's hair as a boots part: its `shirt` material names its texture in another
        // directory than the boots', and no `shirt.dds` of the player's points both at one.
        let material = player_with(
            vec![
                named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl"),
                named(&format!("{PLAYER}/a_boots.fmdl"), "fcl_hair.fmdl"),
            ],
            Vec::new(),
        );
        let batch = run(boots(material));
        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        assert_eq!(
            one_message(&batch),
            (
                "merge_material_conflict",
                Severity::Error,
                Disposition::DropFolder,
                &[("material".to_owned(), "shirt".to_owned())][..]
            )
        );
    }

    #[test]
    fn a_combined_shared_folder_s_models_are_parts_and_its_textures_go_to_the_player_s_common() {
        let folder = player_with(
            vec![named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl")],
            vec![shared(
                ModelPackage::Boots,
                "Boots/Crocs",
                vec![
                    named("Boots/Crocs/boots.fmdl", "boots.fmdl"),
                    named("Boots/Crocs/shirt.dds", "shirt.dds"),
                ],
            )],
        );

        let package = run(boots(folder.clone()));
        let textures = run(TaskKind::Textures {
            folder,
            kits: Vec::new(),
        });

        assert_eq!(one_message(&package).0, "fmdl_merged");
        let fpk = FpkFile::read(&package.entries[0].1).unwrap();
        assert_eq!(
            packed_model(&fpk, "boots.fmdl").meshes.len(),
            2 * tracer_model("boots.fmdl").meshes.len()
        );
        // The shared folder's texture is the player's now: the merged model points at the
        // player's common folder, where the textures task puts it.
        assert_rewritten(&texture_directories(&fpk, "boots.fmdl"));
        assert!(textures.messages.is_empty(), "{:?}", textures.messages);
        assert_eq!(paths(&textures), [COMMON]);
    }

    #[test]
    fn a_combined_gloves_folder_bringing_the_other_hand_merges_nothing() {
        let folder = player_with(
            vec![named(&format!("{PLAYER}/glove_l.fmdl"), "glove_l.fmdl")],
            vec![shared(
                ModelPackage::Gloves,
                "Gloves/Grip",
                vec![named("Gloves/Grip/glove_r.fmdl", "glove_r.fmdl")],
            )],
        );

        let batch = run(TaskKind::Models {
            folder,
            package: ModelPackage::Gloves,
            ids: vec![PackageKey::Id(3745)],
            kits: Vec::new(),
        });

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let names: Vec<&str> = package.entries().map(|(name, _)| name).collect();
        assert_eq!(names, ["glove_l.fmdl", "glove_r.fmdl"]);
        assert_eq!(
            packed_model(&package, "glove_r.fmdl").meshes.len(),
            tracer_model("glove_r.fmdl").meshes.len(),
            "a single part is packed as it is"
        );
    }

    /// The tracer's boots model with every texture path naming `armor_kitN.dds`, a kit
    /// set's reference.
    fn naming_kit_reference() -> Vec<u8> {
        let mut model =
            FmdlFile::read(&std::fs::read(tracer().join(format!("{PLAYER}/boots.fmdl"))).unwrap())
                .unwrap();
        rewrite_texture_paths(&mut model, |path| {
            path.directory.clear();
            path.file_name = "armor_kitN.dds".to_owned();
        })
        .unwrap();
        model.write()
    }

    /// The (file name, directory) pairs of the model `name` in `batch`'s package.
    fn directories_of(batch: &TaskBatch, name: &str) -> Vec<(String, String)> {
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        texture_directories(&package, name)
    }

    #[test]
    fn a_folder_s_own_texture_of_a_kit_set_stem_wins_its_own_link_of_the_next() {
        // `armor_kit1.dds` and `armor_kit2.dds.common` both file `armor_kitn` in the
        // folder's own directory: the folder's own texture resolves the reference, as a
        // stem the two spell the same resolves.
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/boots.fmdl"), "boots.fmdl"),
                named(&format!("{PLAYER}/armor_kit1.dds"), "shirt.dds"),
                file(&format!("{PLAYER}/armor_kit2.dds.common")),
            ],
            Vec::new(),
        );

        let batch = run_with(
            boots(folder),
            &[(&format!("{PLAYER}/boots.fmdl"), &naming_kit_reference())],
        );

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        let directories = directories_of(&batch, "boots.fmdl");
        assert!(!directories.is_empty(), "a texture path renamed");
        assert!(
            directories
                .iter()
                .all(|path| *path == ("armor_kitN.dds".to_owned(), COMMON_DIRECTORY.to_owned())),
            "{directories:?}"
        );
    }

    #[test]
    fn a_combined_texture_of_a_kit_set_stem_wins_the_combined_link_of_the_next() {
        // The shared boots folder's `armor_kit1.dds` and `armor_kit2.dds.common` both file
        // `armor_kitn` as its folder's textures and links do: the texture resolves the
        // reference, the link standing at the path it names.
        let folder = player_with(
            vec![named(&format!("{PLAYER}/boots.fmdl"), "boots.fmdl")],
            vec![shared(
                ModelPackage::Boots,
                "Boots/Crocs",
                vec![
                    named("Boots/Crocs/armor_kit1.dds", "shirt.dds"),
                    file("Boots/Crocs/armor_kit2.dds.common"),
                ],
            )],
        );

        let batch = run_with(
            boots(folder),
            &[(&format!("{PLAYER}/boots.fmdl"), &naming_kit_reference())],
        );

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        let directories = directories_of(&batch, "boots.fmdl");
        assert!(!directories.is_empty(), "a texture path renamed");
        assert!(
            directories
                .iter()
                .all(|path| *path == ("armor_kitN.dds".to_owned(), COMMON_DIRECTORY.to_owned())),
            "{directories:?}"
        );
    }

    #[test]
    fn a_common_models_task_s_entries_are_in_path_order() {
        // `b.fmdl` converts into `oral_b_win32.model` and `b.mtl`; `a.model`, packed as it
        // is, sorts between them: the file order (`b.fmdl`, `a.model`) is not the entries'.
        let common = vec![file("Common/b.fmdl"), file("Common/a.model")];
        let files: TaskFiles = [
            (
                ScopePath::new("Common/b.fmdl").unwrap(),
                std::fs::read(tracer().join(format!("{PLAYER}/boots.fmdl"))).unwrap(),
            ),
            (ScopePath::new("Common/a.model").unwrap(), Vec::new()),
        ]
        .into_iter()
        .collect();

        let batch = process(
            TaskKind::CommonModels {
                folder: ScopePath::new("Common").unwrap(),
                files: common,
                textures: Vec::new(),
            },
            PesVersion::Pes17,
            None,
            files,
        );

        assert_eq!(
            paths(&batch),
            [
                "common/character1/model/character/uniform/common/792/b.mtl",
                "common/character1/model/character/uniform/common/792/oral_a_win32.model",
                "common/character1/model/character/uniform/common/792/oral_b_win32.model",
            ]
        );
    }

    #[test]
    fn a_common_models_task_finds_the_packed_model_s_mtl_by_its_inner_stem() {
        // The VG export's `Common/`: `oral_fbm_u_win32.model` beside `apc.mtl` and
        // `fbm.mtl`. The cap model needs conversion on PES 15 (its `dsk_*` bones
        // are absent there), so the `.mtl` the search picks reaches the
        // conversion's material check: `apc.mtl`, the first "any", defines none
        // of its materials and would fail the task; `fbm.mtl` is the inner
        // stem's name match.
        let common = vec![
            file("Common/oral_fbm_u_win32.model"),
            file("Common/apc.mtl"),
            file("Common/fbm.mtl"),
        ];
        let files: TaskFiles = [
            (
                ScopePath::new("Common/oral_fbm_u_win32.model").unwrap(),
                pes_fixture("konami_modD_cap.model"),
            ),
            (
                ScopePath::new("Common/apc.mtl").unwrap(),
                pes_fixture("konami_hair.mtl"),
            ),
            (
                ScopePath::new("Common/fbm.mtl").unwrap(),
                pes_fixture("konami_modD_cap.mtl"),
            ),
        ]
        .into_iter()
        .collect();

        let batch = process(
            TaskKind::CommonModels {
                folder: ScopePath::new("Common").unwrap(),
                files: common,
                textures: Vec::new(),
            },
            PesVersion::Pes15,
            None,
            files,
        );

        assert!(
            batch
                .messages
                .iter()
                .all(|message| message.severity != Severity::Error),
            "{:?}",
            batch.messages
        );
        assert_eq!(
            paths(&batch),
            [
                "common/character1/model/character/uniform/common/792/apc.mtl",
                "common/character1/model/character/uniform/common/792/fbm.mtl",
                "common/character1/model/character/uniform/common/792/oral_fbm_u_win32.model",
            ]
        );
    }

    #[test]
    fn a_pre_fox_face_packs_none_of_a_combined_boots_folder_s_files() {
        let folder = ModelFolder {
            engine: Engine::PreFox,
            ..player_with(
                vec![named(&format!("{PLAYER}/face_high.fmdl"), "fcl_hair.fmdl")],
                vec![shared(
                    ModelPackage::Boots,
                    "Boots/Studs",
                    vec![
                        named("Boots/Studs/boots.fmdl", "boots.fmdl"),
                        named("Boots/Studs/shirt.dds", "shirt.dds"),
                    ],
                )],
            )
        };

        let batch = run_for(
            TaskKind::Models {
                folder,
                package: ModelPackage::Face,
                ids: vec![PackageKey::Id(79205)],
                kits: Vec::new(),
            },
            PesVersion::Pes17,
        );

        let codes: Vec<&str> = batch
            .messages
            .iter()
            .map(|message| message.code.code.as_ref())
            .collect();
        assert!(!codes.contains(&"xml_face_neck_added"), "{codes:?}");
        assert_eq!(batch.entries.len(), 1, "{:?}", paths(&batch));
        let mut cpk = cpk::CpkArchive::open(std::io::Cursor::new(&batch.entries[0].1)).unwrap();
        let entries = cpk.entries().to_vec();
        let names: Vec<&str> = entries
            .iter()
            .map(|entry| {
                entry
                    .path
                    .rsplit_once('/')
                    .map_or(entry.path.as_str(), |(_, name)| name)
            })
            .collect();
        // The shared boots are the Boots task's, written as their own folder.
        assert_eq!(
            names,
            ["face.xml", "face_high.mtl", "oral_face_high_win32.model"]
        );
        let xml = entries
            .iter()
            .find(|entry| entry.path.ends_with("/face.xml"))
            .unwrap();
        let xml = String::from_utf8(cpk.read(xml).unwrap()).unwrap();
        let document = roxmltree::Document::parse(&xml).unwrap();
        let types: Vec<&str> = document
            .root_element()
            .children()
            .filter(|node| node.has_tag_name("model"))
            .map(|node| node.attribute("type").unwrap())
            .collect();
        assert_eq!(types, ["face_neck"]);
        // The boots folder's `shirt.dds` goes to the player's texture home with his own
        // textures, so the face's converted material naming it points there.
        let mtl = entries
            .iter()
            .find(|entry| entry.path.ends_with("/face_high.mtl"))
            .unwrap();
        let set = pes_model::format::mtl::MaterialSet::read(&cpk.read(mtl).unwrap()).unwrap();
        let shirts: Vec<String> = pes_model::ops::paths::texture_paths(&set)
            .into_iter()
            .filter(|path| path.file_name == "shirt.dds")
            .map(|path| path.directory)
            .collect();
        assert!(!shirts.is_empty());
        assert!(
            shirts.iter().all(|directory| directory
                == "model/character/uniform/common/792/05 - The Chad Stormworks Player/"),
            "{shirts:?}"
        );
    }

    /// The tracer's hair model as a Common model of its own: its materials renamed with a
    /// `_common` tail and its `shirt.dds` renamed `cloth.dds`, so it merges with the hair itself
    /// with no material in common.
    fn common_hair_model() -> Vec<u8> {
        let mut model = tracer_model("fcl_hair.fmdl");
        for material in &mut model.materials {
            material.name.push_str("_common");
            for (_, texture) in &mut material.textures {
                if texture.file_name == "shirt.dds" {
                    texture.file_name = "cloth.dds".to_owned();
                }
            }
        }
        model.to_file().unwrap().write()
    }

    /// The tracer's player folder holding `torso.fmdl` (the hair model), `shirt.dds` and the
    /// link `legs.fmdl.common` resolved to `Common/legs.fmdl`, the Common textures' stems being
    /// `common_stems`; with `skeletons`, `torso.skl` beside the model and `Common/legs.skl` with
    /// the Common one. The Common files' bytes come from the tracer's hair model and skeleton
    /// unless `run_with` replaces them.
    fn player_linking_common(common_stems: &[&str], skeletons: bool) -> ModelFolder {
        let link = ScopePath::new(&format!("{PLAYER}/legs.fmdl.common")).unwrap();
        let mut files = vec![
            named(&format!("{PLAYER}/torso.fmdl"), "fcl_hair.fmdl"),
            named(&format!("{PLAYER}/shirt.dds"), "shirt.dds"),
            FileDescriptor {
                kind: aesthetics_export::classify(link.name()),
                path: link.clone(),
                size: 0,
                ..file(&format!("{PLAYER}/face_diff.bin"))
            },
        ];
        if skeletons {
            files.push(named(&format!("{PLAYER}/torso.skl"), "fcl_hair.skl"));
        }
        ModelFolder {
            common_models: vec![CommonModel {
                link,
                model: named("Common/legs.fmdl", "fcl_hair.fmdl"),
                skeleton: skeletons.then(|| named("Common/legs.skl", "fcl_hair.skl")),
                material: None,
            }],
            common_texture_stems: common_stems.iter().map(|stem| (*stem).to_owned()).collect(),
            ..player_with(files, Vec::new())
        }
    }

    #[test]
    fn a_common_part_merges_in_with_its_skeleton_and_its_paths_name_the_team_s_common_output() {
        let custom = other_skeleton();
        let folder = player_linking_common(&["cloth"], true);
        let kind = face(folder.clone());
        let read: Vec<&str> = kind.files().iter().map(|file| file.path.as_str()).collect();
        assert_eq!(
            read,
            [
                "Players/05 - The Chad Stormworks Player/torso.fmdl",
                "Common/legs.fmdl",
                "Common/legs.skl",
                "Players/05 - The Chad Stormworks Player/torso.skl",
            ],
            "the Common files in the link's place; the link itself is never read"
        );

        // Both parts bring the one custom skeleton: the Common part's pairs with it by their
        // shared `Common/legs` stem.
        let torso_skl = format!("{PLAYER}/torso.skl");
        let batch = run_with(
            face(folder),
            &[
                ("Common/legs.fmdl", &common_hair_model()),
                ("Common/legs.skl", &custom),
                (&torso_skl, &custom),
            ],
        );

        assert_eq!(
            one_message(&batch),
            (
                "fmdl_merged",
                Severity::Info,
                Disposition::Keep,
                &[("model".to_owned(), "fcl_hair.fmdl".to_owned())][..]
            )
        );
        assert_eq!(
            package_names(&batch),
            [
                "face_diff.bin",
                "fcl_hair.fmdl",
                "fcl_hair_sim.fclo",
                "fcl_hair_sim.skl"
            ]
        );
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        assert_eq!(package.get("fcl_hair_sim.skl").unwrap(), custom);
        assert_eq!(
            packed_model(&package, "fcl_hair.fmdl").meshes.len(),
            2 * tracer_model("fcl_hair.fmdl").meshes.len()
        );
        // The player's own `shirt` points at its common subfolder, the Common model's `cloth`
        // at the team's Common output, and the game's own textures at the team's.
        let directories = texture_directories(&package, "fcl_hair.fmdl");
        assert_rewritten(&directories);
        assert!(
            directories.contains(&(
                "cloth.dds".to_owned(),
                "/Assets/pes16/model/character/common/792/sourceimages/".to_owned()
            )),
            "{directories:?}"
        );
        assert!(
            !directories.contains(&("cloth.dds".to_owned(), COMMON_DIRECTORY.to_owned())),
            "{directories:?}"
        );
    }

    #[test]
    fn a_material_two_parts_define_over_textures_in_different_places_is_a_conflict() {
        // The same hair model as the local part and the Common part, both naming `shirt.dds`:
        // the player's copy is the player's own, Common's copy is the team's, so the two
        // `shirt` materials no longer agree. (No skeleton with the Common part: the local part
        // brings none, and a mix would be the skeleton's conflict first.)
        let linking = |common_stems: &[&str]| player_linking_common(common_stems, false);
        let batch = run(face(linking(&["shirt"])));
        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        assert_eq!(
            one_message(&batch),
            (
                "merge_material_conflict",
                Severity::Error,
                Disposition::DropFolder,
                &[("material".to_owned(), "shirt".to_owned())][..]
            )
        );

        // With no `shirt` in Common, the Common part's `shirt` is one of the game's own and
        // still differs from the player's; renamed materials merge.
        let batch = run(face(linking(&[])));
        assert_eq!(one_message(&batch).0, "merge_material_conflict");
        let batch = run_with(
            face(linking(&[])),
            &[("Common/legs.fmdl", &common_hair_model())],
        );
        assert_eq!(one_message(&batch).0, "fmdl_merged");
    }

    #[test]
    fn a_common_part_goes_only_into_the_package_its_link_s_role_names() {
        // A local hair model beside a Common boots link: the Common boots are the boots
        // package's part and nothing of the face's.
        let link = ScopePath::new(&format!("{PLAYER}/kit_boots.fmdl.common")).unwrap();
        let folder = ModelFolder {
            common_models: vec![CommonModel {
                link: link.clone(),
                model: named("Common/kit_boots.fmdl", "boots.fmdl"),
                skeleton: None,
                material: None,
            }],
            ..player_with(
                vec![
                    named(&format!("{PLAYER}/fcl_hair.fmdl"), "fcl_hair.fmdl"),
                    FileDescriptor {
                        kind: aesthetics_export::classify(link.name()),
                        source: link.clone(),
                        path: link,
                        size: 0,
                    },
                ],
                Vec::new(),
            )
        };

        // The face task is handed the Common boots' bytes too, which it never lists: a face
        // package taking them as a part then fails on its contents, not on the coordinator's
        // "every file listed" invariant.
        let face_batch = run_with(
            face(folder.clone()),
            &[("Common/kit_boots.fmdl", &tracer_player_file("boots.fmdl"))],
        );
        let boots_batch = run(boots(folder));

        assert!(face_batch.messages.is_empty(), "{:?}", face_batch.messages);
        assert_eq!(
            package_names(&face_batch),
            [
                "face_diff.bin",
                "fcl_hair.fmdl",
                "fcl_hair_sim.fclo",
                "fcl_hair_sim.skl"
            ]
        );
        assert!(
            boots_batch.messages.is_empty(),
            "{:?}",
            boots_batch.messages
        );
        assert_eq!(package_names(&boots_batch), ["boots.fmdl", "boots.skl"]);
        let package = FpkFile::read(&boots_batch.entries[0].1).unwrap();
        assert_eq!(
            packed_model(&package, "boots.fmdl").meshes.len(),
            tracer_model("boots.fmdl").meshes.len()
        );
    }

    #[test]
    fn a_texture_link_s_stem_names_the_team_s_common_output_and_a_local_texture_the_folder_s() {
        // The tracer's hair model naming `hair.dds` in its `shirt` material and `skin.dds` in
        // the other material that named `shirt.dds`.
        let mut model = tracer_model("fcl_hair.fmdl");
        for material in &mut model.materials {
            let renamed = if material.name == "shirt" {
                "hair.dds"
            } else {
                "skin.dds"
            };
            for (_, texture) in &mut material.textures {
                if texture.file_name == "shirt.dds" {
                    texture.file_name = renamed.to_owned();
                }
            }
        }
        let face_high = format!("{PLAYER}/face_high.fmdl");
        let link = ScopePath::new(&format!("{PLAYER}/hair.dds.common")).unwrap();
        let folder = player_with(
            vec![
                named(&face_high, "fcl_hair.fmdl"),
                named(&format!("{PLAYER}/skin.dds"), "shirt.dds"),
                FileDescriptor {
                    kind: aesthetics_export::classify(link.name()),
                    source: link.clone(),
                    path: link,
                    size: 0,
                },
            ],
            Vec::new(),
        );

        let batch = run_with(
            face(folder),
            &[(&face_high, &model.to_file().unwrap().write())],
        );

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let directories = texture_directories(&package, "face_high.fmdl");
        let team_common = "/Assets/pes16/model/character/common/792/sourceimages/";
        let directories_of = |file_name: &str| -> Vec<&str> {
            directories
                .iter()
                .filter(|(name, _)| name == file_name)
                .map(|(_, directory)| directory.as_str())
                .collect()
        };
        assert_eq!(directories_of("hair.dds"), [team_common], "{directories:?}");
        assert_eq!(
            directories_of("skin.dds"),
            [COMMON_DIRECTORY],
            "{directories:?}"
        );
    }

    #[test]
    fn the_common_textures_go_once_into_the_team_s_common_output_or_fail_as_one() {
        let folder = ScopePath::new("Common").unwrap();
        let textures = vec![
            named("Common/Cloth.dds", "shirt.dds"),
            named("Common/hair.ftex", "shirt.dds"),
        ];
        let converted =
            ftex::dds_to_ftex(&tracer_player_file("shirt.dds"), ftex::ColorSpace::Normal).unwrap();

        let batch = run_with(
            TaskKind::CommonTextures {
                folder: folder.clone(),
                textures: textures.clone(),
                kits: Vec::new(),
                environment_map: false,
            },
            &[("Common/hair.ftex", &converted)],
        );

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(batch.group, None);
        assert_eq!(
            paths(&batch),
            [
                "Asset/model/character/common/792/sourceimages/#windx11/Cloth.ftex",
                "Asset/model/character/common/792/sourceimages/#windx11/hair.ftex",
            ],
            "each stem as spelled"
        );
        assert_eq!(batch.entries[0].1, converted);
        assert_eq!(batch.entries[1].1, converted);

        // One texture that cannot convert fails the task, on the Common folder.
        let batch = run_with(
            TaskKind::CommonTextures {
                folder,
                textures,
                kits: Vec::new(),
                environment_map: false,
            },
            &[("Common/Cloth.dds", b"not a DDS")],
        );
        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        let [message] = batch.messages.as_slice() else {
            panic!("{:?}", batch.messages);
        };
        assert_eq!(message.code.code, "folder_pack_failed");
        assert_eq!(
            message.scope,
            Scope::Folder {
                export_id: ExportId(4),
                path: ScopePath::new("Common").unwrap(),
            }
        );
        let [(key, error)] = message.context.as_slice() else {
            panic!("{:?}", message.context);
        };
        assert_eq!(key, "error");
        assert!(error.starts_with("cannot convert Cloth.dds"), "{error}");
    }

    #[test]
    fn a_common_texture_with_a_finding_fails_the_task() {
        let folder = ScopePath::new("Common").unwrap();
        // A codec conversion refuses is the finding conversion itself reports, failing the
        // task as any other conversion failure does; the deep pass has already dropped the
        // files its own checks find wrong.
        let textures = vec![
            named("Common/bc6h.dds", "shirt.dds"),
            named("Common/hair.dds", "shirt.dds"),
        ];
        let bc6h = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/textures/bc6h.dds"),
        )
        .unwrap();

        let batch = run_with(
            TaskKind::CommonTextures {
                folder,
                textures,
                kits: Vec::new(),
                environment_map: false,
            },
            &[("Common/bc6h.dds", &bc6h)],
        );

        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        let [message] = batch.messages.as_slice() else {
            panic!("{:?}", batch.messages);
        };
        assert_eq!(message.code.code, "texture_codec_unsupported");
        assert_eq!(
            (message.severity, message.disposition),
            (Severity::Error, Disposition::DropFolder)
        );
        assert_eq!(
            message.scope,
            Scope::Folder {
                export_id: ExportId(4),
                path: ScopePath::new("Common").unwrap(),
            }
        );
        assert_eq!(
            message.context,
            [("file".to_owned(), "bc6h.dds".to_owned())]
        );
    }

    #[test]
    fn a_common_subfolder_texture_s_failure_names_its_path_below_the_folder() {
        let folder = ScopePath::new("Common").unwrap();
        // `kit2/` is a codec conversion refuses: the failure names the file's path below
        // `Common/`, not its name alone, and no Common texture is emitted.
        let bc6h = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/textures/bc6h.dds"),
        )
        .unwrap();
        let batch = run_with(
            TaskKind::CommonTextures {
                folder: folder.clone(),
                textures: vec![
                    named("Common/kit1/armor_bsm.dds", "shirt.dds"),
                    named("Common/kit2/armor_bsm.dds", "shirt.dds"),
                ],
                kits: Vec::new(),
                environment_map: false,
            },
            &[("Common/kit2/armor_bsm.dds", &bc6h)],
        );

        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        let [message] = batch.messages.as_slice() else {
            panic!("{:?}", batch.messages);
        };
        assert_eq!(message.code.code, "texture_codec_unsupported");
        assert_eq!(
            (message.severity, message.disposition),
            (Severity::Error, Disposition::DropFolder)
        );
        assert_eq!(
            message.context,
            [("file".to_owned(), "kit2/armor_bsm.dds".to_owned())]
        );
    }

    #[test]
    fn a_portrait_with_a_finding_fails_its_task_dropping_the_file() {
        let file = file(&format!("{PLAYER}/portrait.dds"));
        let odd = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/textures/odd.png"),
        )
        .unwrap();
        // PNG bytes under the `.dds` name: decoding them as a DDS fails the task.
        let batch = run_with(
            TaskKind::Portrait {
                player_id: 79205,
                file: file.clone(),
            },
            &[(&format!("{PLAYER}/portrait.dds"), &odd)],
        );

        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        let [message] = batch.messages.as_slice() else {
            panic!("{:?}", batch.messages);
        };
        assert_eq!(message.code.code, "folder_pack_failed");
        assert_eq!(
            (message.severity, message.disposition),
            (Severity::Error, Disposition::DropFile)
        );
        assert_eq!(
            message.scope,
            Scope::Folder {
                export_id: ExportId(4),
                path: file.path,
            }
        );
        assert_eq!(
            message.context,
            [(
                "error".to_owned(),
                "cannot convert portrait.dds: container conversion failed: invalid magic: invalid magic"
                    .to_owned()
            )]
        );
    }

    /// The face task over the player folder `folder` under id 79205.
    fn face(folder: ModelFolder) -> TaskKind {
        TaskKind::Models {
            folder,
            package: ModelPackage::Face,
            ids: vec![PackageKey::Id(79205)],
            kits: Vec::new(),
        }
    }

    /// The entry names of the batch's first package.
    fn package_names(batch: &TaskBatch) -> Vec<String> {
        FpkFile::read(&batch.entries[0].1)
            .unwrap()
            .entries()
            .map(|(name, _)| name.to_owned())
            .collect()
    }

    #[test]
    fn two_face_parts_under_one_name_merge_and_hair_parts_bringing_one_skeleton_pack_it() {
        // The tracer's hair model stands in for two `face_high` parts and two hair parts.
        let faces = player_with(
            vec![
                named(&format!("{PLAYER}/face_diff.bin"), "face_diff.bin"),
                named(&format!("{PLAYER}/face_high.fmdl"), "fcl_hair.fmdl"),
                named(&format!("{PLAYER}/x_face_high.fmdl"), "fcl_hair.fmdl"),
            ],
            Vec::new(),
        );
        let batch = run(face(faces));
        assert_eq!(
            one_message(&batch),
            (
                "fmdl_merged",
                Severity::Info,
                Disposition::Keep,
                &[("model".to_owned(), "face_high.fmdl".to_owned())][..]
            )
        );
        assert_eq!(package_names(&batch), ["face_diff.bin", "face_high.fmdl"]);
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        assert_eq!(
            packed_model(&package, "face_high.fmdl").meshes.len(),
            2 * tracer_model("fcl_hair.fmdl").meshes.len()
        );

        let custom = std::fs::read(tracer().join(format!("{PLAYER}/fcl_hair.skl"))).unwrap();
        let hairs = player_with(
            vec![
                named(&format!("{PLAYER}/face_diff.bin"), "face_diff.bin"),
                named(&format!("{PLAYER}/fcl_hair.fmdl"), "fcl_hair.fmdl"),
                named(&format!("{PLAYER}/fcl_hair.skl"), "fcl_hair.skl"),
                named(&format!("{PLAYER}/fcl_hair_sim.fclo"), "fcl_hair_sim.fclo"),
                named(&format!("{PLAYER}/x_fcl_hair.fmdl"), "fcl_hair.fmdl"),
                named(&format!("{PLAYER}/x_fcl_hair.skl"), "fcl_hair.skl"),
            ],
            Vec::new(),
        );
        let batch = run(face(hairs));
        assert_eq!(one_message(&batch).0, "fmdl_merged");
        assert_eq!(
            package_names(&batch),
            [
                "face_diff.bin",
                "fcl_hair.fmdl",
                "fcl_hair_sim.fclo",
                "fcl_hair_sim.skl"
            ]
        );
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        assert_eq!(package.get("fcl_hair_sim.skl").unwrap(), custom);

        // A hair part without a skeleton beside one with is a conflict.
        let mixed = player_with(
            vec![
                named(&format!("{PLAYER}/face_diff.bin"), "face_diff.bin"),
                named(&format!("{PLAYER}/fcl_hair.fmdl"), "fcl_hair.fmdl"),
                named(&format!("{PLAYER}/fcl_hair.skl"), "fcl_hair.skl"),
                named(&format!("{PLAYER}/fcl_hair_sim.fclo"), "fcl_hair_sim.fclo"),
                named(&format!("{PLAYER}/x_fcl_hair.fmdl"), "fcl_hair.fmdl"),
            ],
            Vec::new(),
        );
        let batch = run(face(mixed));
        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        assert_eq!(
            one_message(&batch),
            (
                "skl_merge_conflict",
                Severity::Error,
                Disposition::DropFolder,
                &[("skeleton".to_owned(), "differs".to_owned())][..]
            )
        );
    }

    #[test]
    fn a_combined_face_folder_is_the_face_and_the_player_s_own_face_files_win_over_its() {
        // The shared face alone: the player folder holds nothing of the face but the link.
        let shared_face = shared(
            ModelPackage::Face,
            "Faces/Round",
            vec![
                named("Faces/Round/face_diff.bin", "face_diff.bin"),
                named("Faces/Round/hair_high.fmdl", "boots.fmdl"),
                named("Faces/Round/shirt.dds", "shirt.dds"),
            ],
        );
        let alone = player_with(Vec::new(), vec![shared_face.clone()]);
        let batch = run(face(alone.clone()));
        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(package_names(&batch), ["face_diff.bin", "hair_high.fmdl"]);
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        // The shared folder's texture is the player's now.
        assert_rewritten(&texture_directories(&package, "hair_high.fmdl"));
        let textures = run(TaskKind::Textures {
            folder: alone,
            kits: Vec::new(),
        });
        assert_eq!(paths(&textures), [COMMON]);

        // Both sources holding `face_diff.bin`: the player's own is packed, the shared one
        // never read (the simulation file's bytes stand in for a differing shared copy).
        let own = tracer_player_file("face_diff.bin");
        let both = player_with(
            vec![named(&format!("{PLAYER}/face_diff.bin"), "face_diff.bin")],
            vec![shared(
                ModelPackage::Face,
                "Faces/Round",
                vec![
                    named("Faces/Round/face_diff.bin", "fcl_hair_sim.fclo"),
                    named("Faces/Round/hair_high.fmdl", "boots.fmdl"),
                ],
            )],
        );
        let kind = face(both);
        let read: Vec<&str> = kind.files().iter().map(|file| file.path.as_str()).collect();
        assert_eq!(
            read,
            [
                "Players/05 - The Chad Stormworks Player/face_diff.bin",
                "Faces/Round/hair_high.fmdl"
            ]
        );
        let batch = run(kind);
        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        assert_eq!(package.get("face_diff.bin").unwrap(), own);
    }

    /// The tracer's hair model with every texture path naming the bare local
    /// `./missing.dds`.
    fn naming_missing() -> Vec<u8> {
        let mut model = FmdlFile::read(
            &std::fs::read(tracer().join(format!("{PLAYER}/fcl_hair.fmdl"))).unwrap(),
        )
        .unwrap();
        rewrite_texture_paths(&mut model, |path| {
            path.directory = "./".to_owned();
            path.file_name = "missing.dds".to_owned();
        })
        .unwrap();
        model.write()
    }

    #[test]
    fn a_bare_local_texture_nothing_supplies_is_a_warning_and_the_package_kept() {
        // `./missing.dds` names a texture of the part's own folder; with
        // none there, the pointing leaves it and nothing supplies it.
        let round = |extra: Vec<FileDescriptor>| {
            player_with(
                Vec::new(),
                vec![shared(
                    ModelPackage::Face,
                    "Faces/Round",
                    [
                        vec![
                            named("Faces/Round/face_diff.bin", "face_diff.bin"),
                            named("Faces/Round/hair_high.fmdl", "fcl_hair.fmdl"),
                        ],
                        extra,
                    ]
                    .concat(),
                )],
            )
        };
        let batch = run_with(
            face(round(Vec::new())),
            &[("Faces/Round/hair_high.fmdl", &naming_missing())],
        );
        let (code, severity, disposition, context) = one_message(&batch);
        assert_eq!(
            (code, severity, disposition),
            (
                "fmdl_texture_not_found",
                Severity::Warning,
                Disposition::Keep
            )
        );
        assert!(
            context
                .iter()
                .any(|(key, value)| key == "texture" && value == "./missing.dds"),
            "{context:?}"
        );
        assert!(!batch.entries.is_empty(), "the package is kept");

        // `missing.dds` beside the model resolves the stem: no finding.
        let batch = run_with(
            face(round(vec![named("Faces/Round/missing.dds", "shirt.dds")])),
            &[("Faces/Round/hair_high.fmdl", &naming_missing())],
        );
        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
    }

    /// The tracer's player folder's `face_diff.bin` bytes.
    fn tracer_player_file(name: &str) -> Vec<u8> {
        std::fs::read(tracer().join(format!("{PLAYER}/{name}"))).unwrap()
    }

    /// The export file at `path`, described as the structure pass would, whose bytes `run`
    /// reads from the tracer's `kit.dds`: a real DDS whose bytes differ from `shirt.dds`.
    fn other_dds(path: &str) -> FileDescriptor {
        let path = ScopePath::new(path).unwrap();
        FileDescriptor {
            kind: aesthetics_export::classify(path.name()),
            path,
            ..file("Kits/g1/kit.dds")
        }
    }

    /// The textures task over `folder` as task 3 of a group at 0..4 holding its face, boots
    /// and gloves tasks.
    fn textures_in_group(folder: ModelFolder) -> TaskBatch {
        run_task(
            TaskKind::Textures {
                folder,
                kits: Vec::new(),
            },
            PesVersion::Pes21,
            Some(TaskGroup {
                tasks: 0..4,
                charge: 0,
            }),
        )
    }

    #[test]
    fn a_stem_two_sources_hold_with_the_same_bytes_is_one_texture_and_no_finding() {
        // The player's own `shirt.dds` and the combined boots folder's `Shirt.dds`: one
        // stem, compared case-folded, written once under the player's spelling.
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl"),
                named(&format!("{PLAYER}/shirt.dds"), "shirt.dds"),
            ],
            vec![shared(
                ModelPackage::Boots,
                "Boots/Crocs",
                vec![
                    named("Boots/Crocs/boots.fmdl", "boots.fmdl"),
                    named("Boots/Crocs/Shirt.dds", "shirt.dds"),
                ],
            )],
        );

        let batch = textures_in_group(folder);

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(paths(&batch), [COMMON]);
    }

    #[test]
    fn a_stem_the_player_and_a_combined_boots_folder_hold_differently_is_the_player_s() {
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/face_diff.bin"), "face_diff.bin"),
                named(&format!("{PLAYER}/fcl_hair.fmdl"), "fcl_hair.fmdl"),
                named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl"),
                named(&format!("{PLAYER}/shirt.dds"), "shirt.dds"),
            ],
            vec![shared(
                ModelPackage::Boots,
                "Boots/Crocs",
                vec![
                    named("Boots/Crocs/boots.fmdl", "boots.fmdl"),
                    other_dds("Boots/Crocs/shirt.dds"),
                    named("Boots/Crocs/sole.dds", "shirt.dds"),
                ],
            )],
        );

        let batch = textures_in_group(folder);

        assert_eq!(
            one_message(&batch),
            (
                "shared_texture_overridden",
                Severity::Info,
                Disposition::Keep,
                &[
                    ("texture".to_owned(), "shirt".to_owned()),
                    ("folder".to_owned(), "Boots/Crocs".to_owned()),
                ][..]
            )
        );
        // His `shirt` is written, the boots' `sole` with it.
        assert_eq!(paths(&batch), [COMMON, player_texture("sole").as_str()]);
        assert_eq!(
            batch.entries[0].1,
            ftex::dds_to_ftex(&tracer_player_file("shirt.dds"), ftex::ColorSpace::Normal).unwrap()
        );
    }

    #[test]
    fn a_stem_two_combined_folders_hold_differently_fails_the_textures_task() {
        // The player's own folder holds no `shirt`: the path is the combined folders', which
        // hold it with different bytes; nothing of the player folder is emitted.
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl"),
                named(&format!("{PLAYER}/glove_l.fmdl"), "glove_l.fmdl"),
            ],
            vec![
                shared(
                    ModelPackage::Gloves,
                    "Gloves/Grip",
                    vec![
                        named("Gloves/Grip/glove_r.fmdl", "glove_r.fmdl"),
                        other_dds("Gloves/Grip/shirt.dds"),
                        named("Gloves/Grip/grip.dds", "shirt.dds"),
                    ],
                ),
                shared(
                    ModelPackage::Boots,
                    "Boots/Crocs",
                    vec![
                        named("Boots/Crocs/boots.fmdl", "boots.fmdl"),
                        named("Boots/Crocs/shirt.dds", "shirt.dds"),
                    ],
                ),
            ],
        );

        let batch = textures_in_group(folder);

        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        assert_eq!(
            one_message(&batch),
            (
                "shared_texture_conflict",
                Severity::Error,
                Disposition::DropFolder,
                &[
                    ("texture".to_owned(), "shirt".to_owned()),
                    ("packages".to_owned(), "boots,gloves".to_owned()),
                ][..]
            )
        );
    }

    #[test]
    fn a_stem_the_player_and_a_combined_face_folder_hold_differently_is_the_player_s() {
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/face_diff.bin"), "face_diff.bin"),
                named(&format!("{PLAYER}/fcl_hair.fmdl"), "fcl_hair.fmdl"),
                named(&format!("{PLAYER}/shirt.dds"), "shirt.dds"),
            ],
            vec![shared(
                ModelPackage::Face,
                "Faces/Round",
                vec![
                    named("Faces/Round/hair_high.fmdl", "boots.fmdl"),
                    other_dds("Faces/Round/shirt.dds"),
                ],
            )],
        );

        let batch = textures_in_group(folder);

        assert_eq!(paths(&batch), [COMMON]);
        assert_eq!(
            batch.entries[0].1,
            ftex::dds_to_ftex(&tracer_player_file("shirt.dds"), ftex::ColorSpace::Normal).unwrap()
        );
        assert_eq!(
            one_message(&batch),
            (
                "shared_texture_overridden",
                Severity::Info,
                Disposition::Keep,
                &[
                    ("texture".to_owned(), "shirt".to_owned()),
                    ("folder".to_owned(), "Faces/Round".to_owned()),
                ][..]
            )
        );
    }

    #[test]
    fn a_stem_the_player_and_a_combined_face_folder_hold_alike_is_packed_once_and_silent() {
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/face_diff.bin"), "face_diff.bin"),
                named(&format!("{PLAYER}/fcl_hair.fmdl"), "fcl_hair.fmdl"),
                named(&format!("{PLAYER}/shirt.dds"), "shirt.dds"),
            ],
            vec![shared(
                ModelPackage::Face,
                "Faces/Round",
                vec![
                    named("Faces/Round/hair_high.fmdl", "boots.fmdl"),
                    named("Faces/Round/Shirt.dds", "shirt.dds"),
                ],
            )],
        );

        let batch = textures_in_group(folder);

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(paths(&batch), [COMMON]);
    }

    #[test]
    fn a_stem_two_combined_folders_of_different_packages_hold_differently_fails_the_task() {
        // The player's own folder holds no `shirt`: the path is the packages', and the
        // conflict between the combined face's and boots' copies leaves the folder out.
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/face_diff.bin"), "face_diff.bin"),
                named(&format!("{PLAYER}/fcl_hair.fmdl"), "fcl_hair.fmdl"),
                named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl"),
            ],
            vec![
                shared(
                    ModelPackage::Face,
                    "Faces/Round",
                    vec![
                        named("Faces/Round/hair_high.fmdl", "boots.fmdl"),
                        named("Faces/Round/shirt.dds", "shirt.dds"),
                    ],
                ),
                shared(
                    ModelPackage::Boots,
                    "Boots/Crocs",
                    vec![
                        named("Boots/Crocs/boots.fmdl", "boots.fmdl"),
                        other_dds("Boots/Crocs/shirt.dds"),
                        named("Boots/Crocs/sole.dds", "shirt.dds"),
                    ],
                ),
            ],
        );

        let batch = textures_in_group(folder);

        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        assert_eq!(
            one_message(&batch),
            (
                "shared_texture_conflict",
                Severity::Error,
                Disposition::DropFolder,
                &[
                    ("texture".to_owned(), "shirt".to_owned()),
                    ("packages".to_owned(), "face,boots".to_owned()),
                ][..]
            )
        );
    }

    #[test]
    fn a_conflict_s_packages_name_every_claimant_in_canonical_order() {
        // Face and boots agree on `shirt`, the gloves differ: the conflict's `packages`
        // names every claimant of the path, the agreeing boots included.
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/face_diff.bin"), "face_diff.bin"),
                named(&format!("{PLAYER}/fcl_hair.fmdl"), "fcl_hair.fmdl"),
                named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl"),
                named(&format!("{PLAYER}/glove_l.fmdl"), "glove_l.fmdl"),
            ],
            vec![
                shared(
                    ModelPackage::Face,
                    "Faces/Round",
                    vec![
                        named("Faces/Round/hair_high.fmdl", "boots.fmdl"),
                        named("Faces/Round/shirt.dds", "shirt.dds"),
                    ],
                ),
                shared(
                    ModelPackage::Boots,
                    "Boots/Crocs",
                    vec![
                        named("Boots/Crocs/boots.fmdl", "boots.fmdl"),
                        named("Boots/Crocs/shirt.dds", "shirt.dds"),
                    ],
                ),
                shared(
                    ModelPackage::Gloves,
                    "Gloves/Grip",
                    vec![
                        named("Gloves/Grip/glove_r.fmdl", "glove_r.fmdl"),
                        other_dds("Gloves/Grip/shirt.dds"),
                    ],
                ),
            ],
        );

        let batch = textures_in_group(folder);

        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        assert_eq!(
            one_message(&batch),
            (
                "shared_texture_conflict",
                Severity::Error,
                Disposition::DropFolder,
                &[
                    ("texture".to_owned(), "shirt".to_owned()),
                    ("packages".to_owned(), "face,boots,gloves".to_owned()),
                ][..]
            )
        );
    }

    #[test]
    fn a_stem_two_combined_folders_of_different_packages_hold_alike_is_packed_once_and_silent() {
        // The face's `shirt` is written and everything else of the two is packed with it,
        // no drop and no finding.
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/face_diff.bin"), "face_diff.bin"),
                named(&format!("{PLAYER}/fcl_hair.fmdl"), "fcl_hair.fmdl"),
                named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl"),
            ],
            vec![
                shared(
                    ModelPackage::Face,
                    "Faces/Round",
                    vec![
                        named("Faces/Round/hair_high.fmdl", "boots.fmdl"),
                        named("Faces/Round/shirt.dds", "shirt.dds"),
                    ],
                ),
                shared(
                    ModelPackage::Boots,
                    "Boots/Crocs",
                    vec![
                        named("Boots/Crocs/boots.fmdl", "boots.fmdl"),
                        named("Boots/Crocs/shirt.dds", "shirt.dds"),
                        named("Boots/Crocs/sole.dds", "shirt.dds"),
                    ],
                ),
            ],
        );

        let batch = textures_in_group(folder);

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(paths(&batch), [COMMON, player_texture("sole").as_str()]);
        assert_eq!(
            batch.entries[0].1,
            ftex::dds_to_ftex(&tracer_player_file("shirt.dds"), ftex::ColorSpace::Normal).unwrap()
        );
    }

    #[test]
    fn a_link_standing_at_a_stem_a_combined_folder_holds_leaves_its_copy_out() {
        // `shirt.dds.common` stands at `shirt` for `Common/shirt.dds`: the combined face
        // folder's `shirt.dds` is left out of his texture home entirely.
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/face_diff.bin"), "face_diff.bin"),
                named(&format!("{PLAYER}/fcl_hair.fmdl"), "fcl_hair.fmdl"),
                named(&format!("{PLAYER}/shirt.dds.common"), "shirt.dds"),
            ],
            vec![shared(
                ModelPackage::Face,
                "Faces/Round",
                vec![
                    named("Faces/Round/hair_high.fmdl", "boots.fmdl"),
                    other_dds("Faces/Round/shirt.dds"),
                ],
            )],
        );

        let batch = textures_in_group(folder);

        assert!(batch.entries.is_empty(), "{:?}", paths(&batch));
        assert_eq!(
            one_message(&batch),
            (
                "shared_texture_overridden",
                Severity::Info,
                Disposition::Keep,
                &[
                    ("texture".to_owned(), "shirt".to_owned()),
                    ("folder".to_owned(), "Faces/Round".to_owned()),
                ][..]
            )
        );
    }

    /// The bytes of `pes_model`'s fixture `name`.
    fn pes_fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../libs/pes_model/tests/fixtures")
                .join(name),
        )
        .unwrap()
    }

    /// `run_with` for `version`: `kind`'s files read from the tracer, each file at a
    /// path of `replacements` replaced by the bytes given with it.
    fn process_with(
        kind: TaskKind,
        version: PesVersion,
        replacements: &[(&str, &[u8])],
    ) -> TaskBatch {
        let mut files: TaskFiles = kind
            .files()
            .into_iter()
            .map(|file| {
                let bytes = std::fs::read(tracer().join(file.source.as_str())).unwrap();
                (file.path.clone(), bytes)
            })
            .collect();
        for (path, bytes) in replacements {
            files.insert(ScopePath::new(path).unwrap(), bytes.to_vec());
        }
        process(kind, version, None, files)
    }

    /// The card-head material set's bytes naming `./skin.dds` alone, as a `.mtl` of
    /// the folder gives them.
    fn skin_materials() -> Vec<u8> {
        let mut set =
            pes_model::format::mtl::MaterialSet::read(&pes_fixture("cardhead_materials.mtl"))
                .unwrap();
        pes_model::ops::paths::rewrite_texture_paths(&mut set, |path| {
            path.directory = "./".to_owned();
            path.file_name = "skin.dds".to_owned();
        });
        set.write()
    }

    /// The directory each `skin.dds` texture path of the `.mtl` `bytes` names.
    fn skin_directories(bytes: &[u8]) -> Vec<String> {
        let set = pes_model::format::mtl::MaterialSet::read(bytes).unwrap();
        pes_model::ops::paths::texture_paths(&set)
            .into_iter()
            .filter(|path| path.file_name == "skin.dds")
            .map(|path| path.directory)
            .collect()
    }

    /// The tracer's boots model naming `pants_kitN.dds`, the kit-set reference a
    /// member writes.
    fn boots_naming_kit_set() -> Vec<u8> {
        let mut model = tracer_model("boots.fmdl");
        for material in &mut model.materials {
            for (_, texture) in &mut material.textures {
                if texture.file_name == "shirt.dds" {
                    texture.file_name = "pants_kitN.dds".to_owned();
                }
            }
        }
        model.to_file().unwrap().write()
    }

    #[test]
    fn a_kit_reference_in_a_model_names_the_player_s_link_not_the_combined_member() {
        // `pants_kit1.dds.common` stands for `Common/pants_kit1.dds` of the set
        // `pants_kitN`: the combined boots folder's `pants_kit2.dds`, the set's other
        // member, does not take the reference from it, so the model points at the team's
        // Common output, where the link's texture is packed.
        let kitned = boots_naming_kit_set();
        let folder = ModelFolder {
            common_files: vec![named("Common/pants_kit1.dds", "shirt.dds")],
            ..player_with(
                vec![
                    named(&format!("{PLAYER}/kit_boots.fmdl"), "boots.fmdl"),
                    named(&format!("{PLAYER}/pants_kit1.dds.common"), "face_diff.bin"),
                ],
                vec![shared(
                    ModelPackage::Boots,
                    "Boots/Crocs",
                    vec![named("Boots/Crocs/pants_kit2.dds", "shirt.dds")],
                )],
            )
        };
        let model = format!("{PLAYER}/kit_boots.fmdl");

        let batch = run_with(boots(folder), &[(&model, &kitned)]);

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        let package = FpkFile::read(&batch.entries[0].1).unwrap();
        let directories = texture_directories(&package, "boots.fmdl");
        assert!(
            directories.contains(&(
                "pants_kitN.dds".to_owned(),
                "/Assets/pes16/model/character/common/792/sourceimages/".to_owned()
            )),
            "{directories:?}"
        );
    }

    #[test]
    fn a_face_s_material_names_a_stem_at_the_player_s_link_not_the_combined_texture() {
        // `skin.dds.common` stands at `skin` for `Common/skin.dds`: the face's `.mtl`
        // names the texture in the team's Common output, not the combined face
        // folder's copy of the stem.
        let folder = ModelFolder {
            engine: Engine::PreFox,
            common_files: vec![named("Common/skin.dds", "shirt.dds")],
            ..player_with(
                vec![
                    named(&format!("{PLAYER}/face_high.model"), "fcl_hair.fmdl"),
                    named(&format!("{PLAYER}/face_high.mtl"), "fcl_hair.fmdl"),
                    named(&format!("{PLAYER}/skin.dds.common"), "face_diff.bin"),
                ],
                vec![shared(
                    ModelPackage::Face,
                    "Faces/Round",
                    vec![named("Faces/Round/skin.dds", "shirt.dds")],
                )],
            )
        };
        let kind = face(folder);
        let model = pes_fixture("cardhead_face_high.model");
        let materials = skin_materials();
        let face_model = format!("{PLAYER}/face_high.model");
        let face_mtl = format!("{PLAYER}/face_high.mtl");

        let batch = process_with(
            kind,
            PesVersion::Pes17,
            &[(&face_model, &model), (&face_mtl, &materials)],
        );

        let mut cpk = cpk::CpkArchive::open(std::io::Cursor::new(&batch.entries[0].1)).unwrap();
        let entries = cpk.entries().to_vec();
        let mtl = entries
            .iter()
            .find(|entry| entry.path.ends_with("/face_high.mtl"))
            .unwrap_or_else(|| panic!("no face_high.mtl among {:?}", paths(&batch)));
        let set = cpk.read(mtl).unwrap();
        assert_eq!(
            skin_directories(&set),
            ["model/character/uniform/common/792/".to_owned()]
        );
    }

    #[test]
    fn a_marked_player_s_material_names_a_stem_at_his_link_not_the_combined_texture() {
        // Under `ingame_face` his `.common` texture link stands at `skin` as on the
        // face: the boots' `.mtl` names the Common texture at the team's output, not
        // the combined boots folder's copy of its stem.
        let folder = ModelFolder {
            engine: Engine::PreFox,
            ingame_face: true,
            common_files: vec![named("Common/skin.dds", "shirt.dds")],
            ..player_with(
                vec![
                    named(&format!("{PLAYER}/boots.model"), "fcl_hair.fmdl"),
                    named(&format!("{PLAYER}/boots.mtl"), "fcl_hair.fmdl"),
                    named(&format!("{PLAYER}/skin.dds.common"), "face_diff.bin"),
                ],
                vec![shared(
                    ModelPackage::Boots,
                    "Boots/Crocs",
                    vec![named("Boots/Crocs/skin.dds", "shirt.dds")],
                )],
            )
        };
        let kind = boots(folder);
        let model = pes_fixture("cardhead_face_high.model");
        let materials = skin_materials();
        let boots_model = format!("{PLAYER}/boots.model");
        let boots_mtl = format!("{PLAYER}/boots.mtl");

        let batch = process_with(
            kind,
            PesVersion::Pes17,
            &[(&boots_model, &model), (&boots_mtl, &materials)],
        );

        let mtl = batch
            .entries
            .iter()
            .find(|(path, _)| path.ends_with("/boots.mtl"))
            .unwrap_or_else(|| panic!("no boots.mtl among {:?}", paths(&batch)));
        assert_eq!(
            skin_directories(&mtl.1),
            ["model/character/uniform/common/792/".to_owned()]
        );
    }

    /// The CPK path of the tracer player's texture `stem`, in its common subfolder.
    fn player_texture(stem: &str) -> String {
        format!(
            "Asset/model/character/common/792/05 - The Chad Stormworks Player/sourceimages/#windx11/{stem}.ftex"
        )
    }

    /// The bytes of the batch's entry at `path`.
    fn entry<'a>(batch: &'a TaskBatch, path: &str) -> &'a [u8] {
        &batch
            .entries
            .iter()
            .find(|(entry, _)| entry == path)
            .unwrap_or_else(|| panic!("no {path} among {:?}", paths(batch)))
            .1
    }

    /// Each `kit_variant_missing` of the batch as (texture, kit, copied), asserting the batch
    /// has no other message and each is a warning that keeps everything.
    fn variants_missing(batch: &TaskBatch) -> Vec<(String, String, String)> {
        batch
            .messages
            .iter()
            .map(|message| {
                assert_eq!(message.code.code, "kit_variant_missing");
                assert_eq!(
                    (message.severity, message.disposition),
                    (Severity::Warning, Disposition::Keep)
                );
                let [(texture_key, texture), (kit_key, kit), (copied_key, copied)] =
                    message.context.as_slice()
                else {
                    panic!("{:?}", message.context);
                };
                assert_eq!((texture_key.as_str(), kit_key.as_str()), ("texture", "kit"));
                assert_eq!(copied_key, "copied");
                (texture.clone(), kit.clone(), copied.clone())
            })
            .collect()
    }

    /// (texture, kit, copied) as `variants_missing` gives it.
    fn missing(texture: &str, kit: &str, copied: &str) -> (String, String, String) {
        (texture.to_owned(), kit.to_owned(), copied.to_owned())
    }

    #[test]
    fn a_variant_set_s_fallback_serves_every_kit_number_it_lacks() {
        // `pants_kit1` and `pants_kit3` hold different images.
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/pants_kit1.dds"), "shirt.dds"),
                other_dds(&format!("{PLAYER}/pants_kit3.dds")),
            ],
            Vec::new(),
        );
        let textures = |kits: &[u8]| {
            run(TaskKind::Textures {
                folder: folder.clone(),
                kits: kits.to_vec(),
            })
        };

        // On PES 18-21 the fallback is written once under the reference's spelling,
        // whatever the export's numbers, and each one the set lacks is still reported.
        let batch = textures(&[1, 2, 3]);
        assert_eq!(
            variants_missing(&batch),
            [missing("pants_kitN", "2", "pants_kit1")]
        );
        assert_eq!(
            paths(&batch),
            [
                player_texture("pants_kit1"),
                player_texture("pants_kit3"),
                player_texture("pants_kitN"),
            ]
        );
        let kit1 = entry(&batch, &player_texture("pants_kit1"));
        assert_eq!(
            kit1,
            ftex::dds_to_ftex(&tracer_player_file("shirt.dds"), ftex::ColorSpace::Normal).unwrap()
        );
        assert_eq!(entry(&batch, &player_texture("pants_kitN")), kit1);
        assert_ne!(entry(&batch, &player_texture("pants_kit3")), kit1);

        // Every number has a variant, or the export defines none: the `kitN` spelling all
        // the same, and no finding.
        for kits in [&[1, 3][..], &[]] {
            let batch = textures(kits);
            assert!(batch.messages.is_empty(), "{kits:?}: {:?}", batch.messages);
            assert_eq!(
                paths(&batch),
                [
                    player_texture("pants_kit1"),
                    player_texture("pants_kit3"),
                    player_texture("pants_kitN"),
                ],
                "{kits:?}"
            );
        }

        // Two numbers lack one: one finding each, in kit order.
        let batch = textures(&[1, 2, 3, 4]);
        assert_eq!(
            variants_missing(&batch),
            [
                missing("pants_kitN", "2", "pants_kit1"),
                missing("pants_kitN", "4", "pants_kit1")
            ]
        );

        // On PES 15-17 the fallback is copied under each missing number's name instead.
        let prefox_texture = |stem: &str| {
            format!(
                "common/character1/model/character/uniform/common/792/05 - The Chad Stormworks Player/{stem}.dds"
            )
        };
        let batch = run_for(
            TaskKind::Textures {
                folder,
                kits: vec![1, 2, 3, 4],
            },
            PesVersion::Pes17,
        );
        assert_eq!(
            variants_missing(&batch),
            [
                missing("pants_kitN", "2", "pants_kit1"),
                missing("pants_kitN", "4", "pants_kit1")
            ]
        );
        assert_eq!(
            paths(&batch),
            [
                prefox_texture("pants_kit1"),
                prefox_texture("pants_kit3"),
                prefox_texture("pants_kit2"),
                prefox_texture("pants_kit4"),
            ]
        );
        assert_eq!(
            entry(&batch, &prefox_texture("pants_kit2")),
            entry(&batch, &prefox_texture("pants_kit1"))
        );
        assert_eq!(
            entry(&batch, &prefox_texture("pants_kit4")),
            entry(&batch, &prefox_texture("pants_kit1"))
        );
    }

    #[test]
    fn each_set_of_a_folder_is_completed_from_its_own_fallback_and_nothing_else_is() {
        // `pants` has only kit 3, `socks` kits 2 and 3 in different images; `shirt` and
        // `skit1` hold no token.
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/pants_kit3.dds"), "shirt.dds"),
                other_dds(&format!("{PLAYER}/socks_kit2.dds")),
                named(&format!("{PLAYER}/socks_kit3.dds"), "shirt.dds"),
                named(&format!("{PLAYER}/shirt.dds"), "shirt.dds"),
                other_dds(&format!("{PLAYER}/skit1.dds")),
            ],
            Vec::new(),
        );

        let batch = run(TaskKind::Textures {
            folder,
            kits: vec![1],
        });

        assert_eq!(
            variants_missing(&batch),
            [
                missing("pants_kitN", "1", "pants_kit3"),
                missing("socks_kitN", "1", "socks_kit2")
            ]
        );
        assert_eq!(
            paths(&batch),
            [
                player_texture("pants_kit3"),
                player_texture("shirt"),
                player_texture("skit1"),
                player_texture("socks_kit2"),
                player_texture("socks_kit3"),
                player_texture("pants_kitN"),
                player_texture("socks_kitN"),
            ]
        );
        assert_eq!(
            entry(&batch, &player_texture("pants_kitN")),
            entry(&batch, &player_texture("pants_kit3"))
        );
        let socks2 = entry(&batch, &player_texture("socks_kit2"));
        assert_eq!(entry(&batch, &player_texture("socks_kitN")), socks2);
        assert_ne!(entry(&batch, &player_texture("socks_kit3")), socks2);
    }

    #[test]
    fn the_common_textures_complete_their_variant_sets_at_the_team_s_common_output() {
        let folder = ScopePath::new("Common").unwrap();
        let textures = vec![
            named("Common/tape_kit2.dds", "shirt.dds"),
            other_dds("Common/hair.dds"),
        ];

        let batch = run(TaskKind::CommonTextures {
            folder: folder.clone(),
            textures,
            kits: vec![1, 2],
            environment_map: false,
        });

        assert_eq!(
            variants_missing(&batch),
            [missing("tape_kitN", "1", "tape_kit2")]
        );
        assert_eq!(
            batch.messages[0].scope,
            Scope::Folder {
                export_id: ExportId(4),
                path: folder,
            }
        );
        let common = |stem: &str| {
            format!("Asset/model/character/common/792/sourceimages/#windx11/{stem}.ftex")
        };
        assert_eq!(
            paths(&batch),
            [common("tape_kit2"), common("hair"), common("tape_kitN")]
        );
        assert_eq!(
            entry(&batch, &common("tape_kitN")),
            entry(&batch, &common("tape_kit2"))
        );
    }

    #[test]
    fn a_kits_fallback_serves_the_set_s_gaps_and_never_its_own_name() {
        // `pants_kit1` and `pants_kitF` hold different images.
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/pants_kit1.dds"), "shirt.dds"),
                other_dds(&format!("{PLAYER}/pants_kitF.dds")),
            ],
            Vec::new(),
        );

        // On PES 18-21: `kitN` alone, from the `kitF` file, and no finding.
        let batch = run(TaskKind::Textures {
            folder: folder.clone(),
            kits: vec![1, 2],
        });
        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(
            paths(&batch),
            [player_texture("pants_kit1"), player_texture("pants_kitN")]
        );
        let kitf = std::fs::read(tracer().join("Kits/g1/kit.dds")).unwrap();
        assert_eq!(
            entry(&batch, &player_texture("pants_kitN")),
            ftex::dds_to_ftex(&kitf, ftex::ColorSpace::Normal).unwrap()
        );

        // On PES 15-17: the `kitF` file copied under each missing number's name, still
        // with no finding and never under its own name.
        let prefox_texture = |stem: &str| {
            format!(
                "common/character1/model/character/uniform/common/792/05 - The Chad Stormworks Player/{stem}.dds"
            )
        };
        let batch = run_for(
            TaskKind::Textures {
                folder,
                kits: vec![1, 2, 3],
            },
            PesVersion::Pes17,
        );
        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(
            paths(&batch),
            [
                prefox_texture("pants_kit1"),
                prefox_texture("pants_kit2"),
                prefox_texture("pants_kit3"),
            ]
        );
        assert_eq!(
            entry(&batch, &prefox_texture("pants_kit2")),
            entry(&batch, &prefox_texture("pants_kit3"))
        );
        assert_ne!(
            entry(&batch, &prefox_texture("pants_kit2")),
            entry(&batch, &prefox_texture("pants_kit1"))
        );
    }

    #[test]
    fn a_kits_fallback_alone_is_a_set_written_as_kitn_on_fox_and_every_gap_on_pes_17() {
        let folder = player_with(
            vec![other_dds(&format!("{PLAYER}/pants_kitF.dds"))],
            Vec::new(),
        );

        let batch = run(TaskKind::Textures {
            folder: folder.clone(),
            kits: vec![1, 2],
        });
        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(paths(&batch), [player_texture("pants_kitN")]);

        let prefox_texture = |stem: &str| {
            format!(
                "common/character1/model/character/uniform/common/792/05 - The Chad Stormworks Player/{stem}.dds"
            )
        };
        let batch = run_for(
            TaskKind::Textures {
                folder,
                kits: vec![1, 2, 3],
            },
            PesVersion::Pes17,
        );
        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        assert_eq!(
            paths(&batch),
            [
                prefox_texture("pants_kit1"),
                prefox_texture("pants_kit2"),
                prefox_texture("pants_kit3"),
            ]
        );
    }

    #[test]
    fn a_variant_set_in_a_subfolder_completes_at_its_own_path() {
        // `face/` holds a set: the `kitF` file serves `kitN` there too, and nothing emits
        // its own name.
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/face/pants_kit1.dds"), "shirt.dds"),
                other_dds(&format!("{PLAYER}/face/pants_kitF.dds")),
                other_dds(&format!("{PLAYER}/old_kit2_hair/skin.dds")),
            ],
            Vec::new(),
        );

        let batch = run(TaskKind::Textures {
            folder,
            kits: vec![1, 2],
        });

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        // A texture of `face/` or `old_kit2_hair/` packs at `<dir>/sourceimages/#windx11/`.
        let sub_dir = |directory: &str, stem: &str| {
            format!(
                "Asset/model/character/common/792/05 - The Chad Stormworks Player/{directory}sourceimages/#windx11/{stem}.ftex"
            )
        };
        assert_eq!(
            paths(&batch),
            [
                sub_dir("face/", "pants_kit1"),
                sub_dir("old_kit2_hair/", "skin"),
                sub_dir("face/", "pants_kitN"),
            ]
        );
        assert_eq!(
            entry(&batch, &sub_dir("face/", "pants_kitN")),
            entry(&batch, &sub_dir("old_kit2_hair/", "skin"))
        );
        assert_ne!(
            entry(&batch, &sub_dir("face/", "pants_kitN")),
            entry(&batch, &sub_dir("face/", "pants_kit1"))
        );
    }

    #[test]
    fn a_variant_set_spelled_by_kit_folders_completes_at_their_parent() {
        // `kit1/armor_bsm` and `kitF/armor_bsm` are the set `kitN/armor_bsm`, of different
        // images.
        let folder = player_with(
            vec![
                named(&format!("{PLAYER}/kit1/armor_bsm.dds"), "shirt.dds"),
                other_dds(&format!("{PLAYER}/kitF/armor_bsm.dds")),
            ],
            Vec::new(),
        );

        let batch = run(TaskKind::Textures {
            folder,
            kits: vec![1, 2],
        });

        assert!(batch.messages.is_empty(), "{:?}", batch.messages);
        let armor = |folder: &str| {
            format!(
                "Asset/model/character/common/792/05 - The Chad Stormworks Player/{folder}/sourceimages/#windx11/armor_bsm.ftex"
            )
        };
        assert_eq!(paths(&batch), [armor("kit1"), armor("kitN")], "no kitF/");
        assert_eq!(
            entry(&batch, &armor("kitN")),
            ftex::dds_to_ftex(
                &std::fs::read(tracer().join("Kits/g1/kit.dds")).unwrap(),
                ftex::ColorSpace::Normal
            )
            .unwrap(),
            "the `kitF` file's conversion under `kitN`"
        );
        assert_ne!(entry(&batch, &armor("kit1")), entry(&batch, &armor("kitN")));
    }

    #[test]
    fn a_model_that_cannot_be_read_fails_its_task() {
        // `face_diff.bin`'s bytes under a face model's name: a file whose name gives it a
        // model's role but whose content cannot be read as an FMDL.
        let mut folder = player(&["face_diff.bin"]);
        let model = ScopePath::new(&format!("{PLAYER}/face_high.fmdl")).unwrap();
        folder.files.push(FileDescriptor {
            kind: aesthetics_export::classify(model.name()),
            path: model,
            ..file(&format!("{PLAYER}/face_diff.bin"))
        });

        let batch = run(TaskKind::Models {
            folder,
            package: ModelPackage::Face,
            ids: vec![PackageKey::Id(79205)],
            kits: Vec::new(),
        });

        assert!(batch.entries.is_empty() && batch.uniparam.is_none());
        let [message] = batch.messages.as_slice() else {
            panic!("{:?}", batch.messages);
        };
        // A member's FMDL is read first by the conversion pre-check, whose read failure names
        // the model (`conversion::fmdl_for_fox`).
        assert_eq!(message.code.code, "model_conversion_failed");
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
    }

    #[test]
    fn every_dds_entry_in_any_case_is_wrapped_once_and_nothing_else_is() {
        let dds = std::fs::read(tracer().join("Kits/g1/kit.dds")).unwrap();
        let wrapped = wezlib::compress(&dds);
        let mtl = b"a material set".to_vec();
        let mut entries = vec![
            ("common/714/skin.dds".to_owned(), dds.clone()),
            ("common/714/HAIR.DDS".to_owned(), dds.clone()),
            ("common/714/eyes.dds".to_owned(), wrapped.clone()),
            ("common/714/face_high.mtl".to_owned(), mtl.clone()),
        ];

        wrap_dds(&mut entries);

        for (path, bytes) in &entries[..2] {
            assert!(wezlib::is_wrapped(bytes), "{path}");
            assert!(wezlib::decompress(bytes).unwrap() == dds, "{path}");
        }
        assert!(
            entries[2].1 == wrapped,
            "an entry already wrapped is left as it is"
        );
        assert!(entries[3].1 == mtl, "a `.mtl` is no DDS");
    }
}
