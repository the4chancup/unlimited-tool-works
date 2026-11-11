//! Stage 2, run planning (`spec/team_compiler/pipeline.md` "Run driver shapes"): the
//! identity-resolved exports become one manifest of tasks, each an atomic unit that commits
//! whole or not at all.

pub(crate) mod collars;
pub(crate) mod ids;
pub(crate) mod item_rows;
pub(crate) mod legacy_blocks;
pub(crate) mod legacy_ids;
pub(crate) mod overrides;
pub(crate) mod patch;
pub(crate) mod roles;

use std::collections::{BTreeMap, BTreeSet};
use std::iter;
use std::ops::{Range, RangeInclusive};

use aesthetics_export::{
    ExportCoverage, ExportIdentity, FileDescriptor, FileKind, FpcDirective, KitFolder, KitsFolder,
    LogoFiles, ModelFormat, PlayerFolder, PlayerIndex, PlayerSlot, ResolvedAestheticsExport,
    SharedKind, SharedModelFolder, ValidatedAestheticsExport, ValidatedRoster, common_link_target,
    kit_folder,
};
use kit_config::{KitSlot, TexturePresence};
use pes_savefile::settings_toml::PlayerSettings;
use pes_version::{Engine, PesVersion};
use studio_core::{Disposition, ExportId, Message, Scope};
use teams_list::TeamId;
use vtree::ScopePath;

use crate::bins::Rgb;
use crate::bins::shared_ids::SharedIds;
use crate::kit_variants::{kit_number, model_variant_sets};
use crate::messages::{Code, tool_message};
use crate::mtl_search::mtl_for;
use crate::paths::{PackageKey, REFEREE_TEAM_ID, TextureHome};
use collars::export_collar;
use ids::{PlannedModelIds, assigned_shared_ids, folder_ids, legacy_folder_id};
use item_rows::{ItemTable, RowPlayer, export_rows};
use legacy_blocks::{block_text, in_block};
use legacy_ids::LegacyIds;
use roles::{
    FolderModels, ModelPackage, PlayerFile, below_common, combined_folders, common_file,
    common_folder, common_skeleton, directory_stem, emits_kit_texture, file_stem,
    has_effective_part, is_common_file, is_direct_root_folder_file, is_hand_split,
    is_package_model, is_read_common_file, is_selected_common_model, leaves_out_kit_variants,
    link_combines, link_feeds_own_package, link_name, linked_common_files, linked_folder,
    native_format, other_format, package_of, part_source_models, player_file, role_files,
    selected_common_model, shared_folders, shared_kind_of, skeleton_slot, texture_format,
};

/// What planning produced: the manifest and the findings planning itself made.
pub(crate) struct PlanReport {
    /// Every task of the run, in canonical order.
    pub(crate) manifest: BuildManifest,
    /// Planning's findings (`team_colors_missing`, `link_combined`, `kit_texture_not_used`,
    /// `kit_config_generated`, `kit_placeholder`, `kit_variant_model_left_out`,
    /// `collar_id_conflict`, `model_gltf_unsupported`).
    pub(crate) messages: Vec<Message>,
}

/// The run's tasks in canonical order: by export, then each mapped player folder's tasks (its
/// face, boots and gloves packages, then its textures) by first roster slot, the shared boots
/// folders taking an id (each its package, then its textures) in id order, then the shared
/// gloves folders the same way, the export's Common textures as one task, the portraits by
/// player id, the kits by slot, the logo, then the collar. The writer lays the CPK out in this
/// order whatever order the tasks finish in, so the same exports always give the same bytes.
pub(crate) struct BuildManifest {
    /// The tasks, in canonical order.
    pub(crate) tasks: Vec<BuildTask>,
    /// Each planned team's id and the colors its root `colors.txt` gives, at most four, for
    /// its `TeamColor.bin` record, in export order. A team whose file gives no color, or that
    /// has no file, is not listed: its record keeps its bytes.
    pub(crate) team_colors: Vec<(u16, Vec<Rgb>)>,
    /// Each planned team export's kits, in export order, for its team's `UniColor.bin` record
    /// and `UniformParameter.bin` configs ("Bins accumulation").
    pub(crate) team_kits: Vec<TeamKits>,
    /// The `[[teams]]` blocks of the run's `aesthetics_patch.toml` (`patch::patch_team`), in
    /// export order, each with the export's item rows `compile` settles its ID keys against
    /// once every batch's outcome is known (`patch::settle`): empty when no team export
    /// compiled a player.
    pub(crate) patch_teams: Vec<patch::PlannedPatchTeam>,
    /// Each planned export's team name as messages show it (`/co/`, `/refs/` for the
    /// referees) and the text of its root `notes.txt`, in export order, for `teamnotes.txt`.
    /// An export without a note is not listed.
    pub(crate) notes: Vec<(String, String)>,
    /// Each planned team export's shared-IDs record, by team ID, in export order. It is written
    /// into the CPK whatever its folders' tasks' outcomes. A `Midcup` export keeps the shared
    /// IDs the team already has, so a `Midcup` over no installed record whose folders took no
    /// ID has none.
    pub(crate) shared_ids: Vec<(u16, SharedIds)>,
}

/// A planned team export's kits.
pub(crate) struct TeamKits {
    /// The export, which an absent slot's FPC finding is reported on.
    pub(crate) export_id: ExportId,
    /// The export's team id.
    pub(crate) team_id: u16,
    /// Whether the export rebuilds its team's kits (`Full`) or adds to them (`Midcup`).
    pub(crate) coverage: ExportCoverage,
    /// What the export sets in every kit config of its team.
    pub(crate) edits: TeamKitEdits,
    /// The slots of its kit tasks, failed or not, ascending.
    pub(crate) slots: Vec<KitSlot>,
    /// Its `Midcup` export's kits holding no `config.toml`, ascending: each emits no
    /// config of its own — its slot's installed config is edited in place as an
    /// unresent slot's is (`kit_configs`), or the template written where none is
    /// installed. Empty for a `Full` export, whose kits without one get the template
    /// as part of their task.
    pub(crate) configless: Vec<ConfiglessKit>,
}

/// A `Midcup` export's kit holding no `config.toml` (`TeamKits::configless`).
pub(crate) struct ConfiglessKit {
    /// The kit's slot.
    pub(crate) slot: KitSlot,
    /// The kit folder, the scope `kit_config_generated` names when the template is
    /// written for the slot.
    pub(crate) path: ScopePath,
    /// Which of the config's five texture names the kit's effective textures set
    /// (the kit task's `TexturePresence` for them).
    pub(crate) textures: TexturePresence,
}

/// Whether `kit`'s installed config is the one to keep, on an export of `coverage`: a
/// `Midcup` kit holding no `config.toml` emits no config of its own — its slot's installed
/// config is edited in place, or the template written where none is installed
/// (`kit_configs`).
pub(crate) fn keeps_installed(kit: &KitFolder, coverage: ExportCoverage) -> bool {
    coverage == ExportCoverage::Midcup && kit.config.is_none()
}

/// One unit of work: one package of a player folder's models, the folder's textures, one
/// player's portrait, one kit, the team's logo or its collar.
pub(crate) struct BuildTask {
    /// The export the task's content comes from.
    pub(crate) export_id: ExportId,
    /// The export's team id, which game paths and file names carry: 999 for a refs export
    /// (`REFEREE_TEAM_ID`).
    pub(crate) team_id: u16,
    /// What the task compiles.
    pub(crate) kind: TaskKind,
    /// The source bytes the task reads, which is what its memory permit is charged
    /// (`spec/libs/pipeline.md` "Memory budget").
    pub(crate) charge: usize,
    /// The player folder's group the task belongs to, when its folder has textures; `None`
    /// for every other task, which the writer commits on its own.
    pub(crate) group: Option<TaskGroup>,
}

/// A model folder's tasks as one unit for the writer: its packages (face, boots, gloves in
/// canonical order; a shared folder has one), then its textures task last, contiguous in the
/// manifest. The writer holds the packages until the textures batch arrives and decides the
/// group, so a folder whose textures failed leaves no package pointing at textures that are
/// not in the CPK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskGroup {
    /// The group's manifest positions, the textures task at `tasks.end - 1`.
    pub(crate) tasks: Range<usize>,
    /// The sum of the members' charges: the coordinator acquires it once, as one permit the
    /// members share, since a textures task waiting for a permit of its own while the writer
    /// holds its packages' would wait forever.
    pub(crate) charge: usize,
}

/// The stem of the environment map a converted metal material names in its model folder's
/// texture home: the folder's own `env` texture, else the template the textures task emits
/// there (`ModelFolder::takes_template_environment_map`); a texture link of the stem makes it
/// the Common one.
pub(crate) const ENVIRONMENT_MAP_STEM: &str = "env";

/// The folder a `Models` or `Textures` task compiles: a mapped player folder, or a shared
/// boots or gloves folder a mapped player links plainly. Its files take the roles of a player
/// folder's (`roles::player_file`); where its textures go differs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelFolder {
    /// The folder's export path (`Players/05 - A`, `Boots/Crocs`): the scope its findings name.
    pub(crate) path: ScopePath,
    /// Its own files.
    pub(crate) files: Vec<FileDescriptor>,
    /// The player folder holds `ingame_face`: it gets no face package, and a model of its own
    /// that would be a part of the face's `fcl_hair` is a part of its boots. Never set for a
    /// shared folder.
    pub(crate) ingame_face: bool,
    /// The target's engine, which decides the role each of the folder's files takes
    /// (`roles::player_file`): each engine builds its own model format.
    pub(crate) engine: Engine,
    /// The shared folders a player folder combines, in link order. Empty for a shared folder,
    /// and for a player linking plainly or not at all.
    pub(crate) combined: Vec<CombinedFolder>,
    /// The `.common` model links of the folder and, for a player folder, of each combined
    /// shared folder, each resolved to the Common model it brings in as a part
    /// (`common_models`; on pre-Fox only an `ingame_face` player's, a shared folder's links
    /// having no role there). Empty for a folder holding none.
    pub(crate) common_models: Vec<CommonModel>,
    /// The stems, as spelled, of the textures directly in the export's `Common/` folder, which
    /// the export's Common textures task emits into the team's Common output. A Common part's
    /// texture paths of these stems name that output, not the folder's texture home. A texture
    /// any part names in that output counts as supplied when its stem is among them.
    pub(crate) common_texture_stems: BTreeSet<String>,
    /// The files of the export's `Common/` folder this folder's tasks may read.
    ///
    /// On a pre-Fox target, for a player folder: the `.model`, FMDL, `.mtl` and texture files
    /// of every directory of `Common/`. A subfolder's files are what a member's `face.xml` names
    /// with `<subfolder>/<name>`, and what a link at the same path below the player folder
    /// stands for (`common_link_target`).
    ///
    /// On Fox: the `.mtl` files directly in `Common/` and those below a subfolder a player's
    /// link reaches (`linked_common_files`). A `.mtl.common` link of the folder stands for one
    /// of them in its `.model`'s search (`mtl_search::mtl_for`). Also the textures below a
    /// subfolder a link reaches: the one a texture link stands for and the ones a linked model
    /// or `.mtl` brings, each spelled as `Common/` spells it (`roles::common_texture_below`).
    /// Fox's Common parts are resolved at planning (`common_models`).
    ///
    /// A shared folder holds the same on Fox, its links resolving as a player folder's, and
    /// nothing on pre-Fox, where its links stay unread.
    ///
    /// The face task tells a model link's Common `.model` from a Common FMDL the Common models
    /// task converts (`roles::selected_common_model`). Its `.mtl` search looks in Common for a
    /// `.model.common` link and resolves a `.mtl.common` link there (`mtl_search::mtl_for`). A
    /// texture link names its Common texture by the stem that file spells. An `ingame_face`
    /// player's boots and gloves resolve a `.mtl.common` link there too (`ModelFolder::roles`),
    /// and point a copied Common `.mtl` at the textures among them.
    pub(crate) common_files: Vec<FileDescriptor>,
    /// The export paths of the player folder's **hand-split parts**: its face models whose
    /// vertices the deep pass found carrying hand weights (`hand_split_parts`). On Fox the face
    /// task packs each one's body, and the folder's gloves task, planned even with no
    /// glove-named file, reads each one and makes its hands `glove_l`/`glove_r` parts. On
    /// pre-Fox the face task alone splits each one, packing its hands as two more `face.xml`
    /// entries, and no gloves task is planned for them. Empty for a shared folder, whose models
    /// are never split.
    pub(crate) hand_split: BTreeSet<ScopePath>,
    /// One of the folder's packages converts an FMDL holding a metal material for PES 15-17
    /// (`converts_metal`): a player's face, an `ingame_face` player's boots or gloves, or a
    /// shared boots or gloves output. Its textures task emits the template environment map as
    /// `env.dds` in its texture home as that home spells it (a player's common folder; a shared
    /// output's own folder, which its `.mtl` files name as `./`), and is planned for it even
    /// when the folder holds no texture, unless one of its sources holds an `env` texture or a
    /// texture link of that stem (`env.dds.common`; `takes_template_environment_map`). The
    /// converting package points each converted `Basic_CNSR` material with no environment
    /// sampler at it, and a link's material at the one in the team's Common output: a
    /// `Basic_CNSR` material always has its environment map, since a metal without a reflection
    /// is not metal. Never set on PES 18-21.
    pub(crate) environment_map: bool,
    /// The face's xml takes the referee template's body entries after the folder's own. Set
    /// for a referee folder holding `fpc_off`, on PES 15-17, with a face to put them in: a
    /// referee is full FPC, so a head-only referee shows no body unless his face lists one.
    /// Never set for a team player (the body is the referee template's) nor on PES 18-21 (the
    /// Fox refkit is not made yet).
    pub(crate) refkit_body: bool,
    /// Where its textures go, which its models' texture paths are rewritten to name.
    pub(crate) textures: TextureHome,
}

/// A model folder's `.common` link to an FMDL or a `.model` (on pre-Fox, only an `ingame_face`
/// player's), resolved against the export's `Common/` folder as validation resolved it. The
/// Common model is a part of the package the link's role names. Its skeleton travels with it,
/// and so does the `.mtl` a Common `.model` uses (`spec/aesthetics_export/player_folders.md`
/// "Common model links and model merging").
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommonModel {
    /// The link file's export path (`Players/05 - A/legs.fmdl.common`), which the folder's
    /// files list and whose role (`PlayerFile::CommonModel`, or `PlayerFile::PreFoxPart` on
    /// pre-Fox) the Common model takes.
    pub(crate) link: ScopePath,
    /// The Common model the link loads (`Common/legs.fmdl`), the one of its stem in the
    /// target's own format when `Common/` holds both (`roles::selected_common_model`): what
    /// the Models task reads.
    pub(crate) model: FileDescriptor,
    /// The `.skl` of the model's stem beside it in `Common/` (`Common/legs.skl`,
    /// `Common/jessie/legs.skl`), when there is one and, on Fox, the role has a skeleton slot,
    /// on pre-Fox the model is an FMDL, whose conversion takes it as its bind pose; `None`
    /// otherwise, a slotless Fox role's skeleton being the structure pass's `skl_no_slot`.
    pub(crate) skeleton: Option<FileDescriptor>,
    /// For a Common `.model`, the `.mtl` the link's search finds (`mtl_search::mtl_for`): a
    /// `Common/` one, or the player's own override of it. On Fox the Models task converts the
    /// model with it. On pre-Fox it is read and written with the Common model, which is a part
    /// of his boots or gloves under `ingame_face`. `None` for a Common FMDL, whose
    /// materials travel inside it on Fox and whose conversion writes its material set on
    /// pre-Fox.
    pub(crate) material: Option<FileDescriptor>,
}

/// A shared folder a player folder combines with its own models, with the package it feeds: a
/// `Faces/` folder the face, a `Boots/` folder the boots, a `Gloves/` folder the gloves. Its
/// models are parts of that package like the player's own. Its textures join the player's
/// textures task, counting for that package when a stem conflicts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CombinedFolder {
    /// The package the folder's models and textures feed.
    pub(crate) package: ModelPackage,
    /// The shared folder.
    pub(crate) folder: SharedModelFolder,
}

/// One source of a model folder's files with their roles (`ModelFolder::roles`): the package
/// the source feeds, the source folder's path, and each of its files `compile` builds with
/// its role.
pub(crate) type SourceRoles<'a> = (
    ModelPackage,
    &'a ScopePath,
    Vec<(&'a FileDescriptor, PlayerFile)>,
);

impl ModelFolder {
    /// The folder's files by source with each file's role, the folder's own files first and
    /// then each combined folder's. A role resolves against the file's own source
    /// (`Boots/Crocs`'s `boots.skl` pairs with `Boots/Crocs`'s `boots.fmdl`, not the
    /// player's). The face's `face_diff.bin` and `fcl_hair_sim.fclo` come once: a combined
    /// face folder's copy is left out when the player folder holds one, and never read. A
    /// `face_diff.xml` counts as the `face_diff.bin` here. One source never holds both: the
    /// deep pass dropped such a folder (`xml_dif_conflict`). A `.common` model link
    /// stands for the Common files it resolved to (`common_models`): the Common model under
    /// the link's role, in the link's place, and its Common skeleton under the role's slot,
    /// paired with it by their shared `Common/<stem>`; the empty link itself is never read. A
    /// Common `.model`'s `.mtl`, when its search finds one in `Common/`, follows it as a
    /// `PlayerFile::Material`, which the package converting it (Fox) or writing it (pre-Fox)
    /// reads. On pre-Fox only a link of a folder holding `ingame_face` is one (a part of his
    /// boots or gloves, `PlayerFile::PreFoxPart`), a Common FMDL bringing its Common `.skl`
    /// as a `PlayerFile::ConversionSkeleton`; under the marker a `.mtl.common`
    /// link likewise brings in its Common `.mtl` as one, the link kept beside it. Each Common
    /// file comes once per folder, however many links of its sources stand for it, the
    /// first source's winning: the player's own folder is the first source.
    pub(crate) fn roles(&self) -> Vec<SourceRoles<'_>> {
        let own_kind = match &self.textures {
            TextureHome::PlayerCommon { .. } => None,
            TextureHome::SharedOutput { package, .. } => Some(shared_kind_of(*package)),
        };
        let combined: Vec<(SharedKind, &SharedModelFolder)> = self
            .combined
            .iter()
            .map(|shared| (shared_kind_of(shared.package), &shared.folder))
            .collect();
        let (own, combined_models) = part_source_models(
            &self.path,
            &self.files,
            own_kind,
            self.ingame_face,
            &combined,
            self.engine,
        );
        let mut sources = vec![(self.own_package(), &self.path, &self.files, own)];
        for (shared, models) in self.combined.iter().zip(combined_models) {
            sources.push((
                shared.package,
                &shared.folder.path,
                &shared.folder.files,
                models,
            ));
        }
        // The names the face files kept so far pack as: a combined face folder's copy of one
        // the player folder holds (`face_diff.bin` in each) is left out. One source holds one
        // of each: the singletons take a role directly in the folder alone (`role_position`).
        let mut packed: Vec<&'static str> = Vec::new();
        // The Common files already pushed for the folder: two link spellings of one Common
        // file, from one source or from two, are one part, not two reads of one model, the
        // first source's link winning (the player's own), and a Common `.mtl` links find is
        // read once.
        let mut common_pushed = BTreeSet::new();
        let mut roles = Vec::new();
        for (package, path, files, models) in sources {
            let mut source_roles = Vec::new();
            for file in files {
                let Some(role) = player_file(path, file, &models) else {
                    continue;
                };
                let packs_as = match role {
                    PlayerFile::Packed { name, .. } => Some(name),
                    PlayerFile::FaceDiffXml => Some("face_diff.bin"),
                    PlayerFile::CommonModel { package, name } => {
                        let resolved = self.common_model(&file.path);
                        if push_common(&mut source_roles, &mut common_pushed, &resolved.model, role)
                        {
                            if let Some(skeleton) = &resolved.skeleton {
                                let name = skeleton_slot(package, name).expect(
                                    "planning pairs a skeleton only with a part that has a slot",
                                );
                                source_roles
                                    .push((skeleton, PlayerFile::Skeleton { package, name }));
                            }
                            push_common_material(&mut source_roles, &mut common_pushed, resolved);
                        }
                        continue;
                    }
                    // Only a marked player's link to a model is a part: the Common model in
                    // its place, with a Common `.mtl` its search finds (his own one is among
                    // his files already), or a Common FMDL with its skeleton, the bind pose
                    // its conversion reads.
                    PlayerFile::PreFoxPart { .. } if file.kind == FileKind::CommonLink => {
                        let resolved = self.common_model(&file.path);
                        if push_common(&mut source_roles, &mut common_pushed, &resolved.model, role)
                        {
                            if let Some(skeleton) = &resolved.skeleton {
                                push_common(
                                    &mut source_roles,
                                    &mut common_pushed,
                                    skeleton,
                                    PlayerFile::ConversionSkeleton,
                                );
                            }
                            push_common_material(&mut source_roles, &mut common_pushed, resolved);
                        }
                        continue;
                    }
                    // Under the marker a part's `.mtl` found through a link comes in with it
                    // (copied in on pre-Fox, read for the conversion on Fox); the link stays
                    // too, for the part's search to see among his files.
                    PlayerFile::CommonMaterial if self.ingame_face => {
                        let linked = common_link_target(&file.path, path).expect(
                            "a CommonMaterial role implies a `.common` link below its folder",
                        );
                        let material = common_file(&self.common_files, &linked).expect(
                            "validation drops a player folder whose link names no Common file",
                        );
                        push_common(
                            &mut source_roles,
                            &mut common_pushed,
                            material,
                            PlayerFile::Material,
                        );
                        None
                    }
                    PlayerFile::Model { .. }
                    | PlayerFile::Skeleton { .. }
                    | PlayerFile::SlotlessSkeleton
                    | PlayerFile::UnusedFaceFile
                    | PlayerFile::LeftOutKitVariant
                    | PlayerFile::Texture { .. }
                    | PlayerFile::CommonTexture(_)
                    | PlayerFile::PreFoxModel { .. }
                    | PlayerFile::PreFoxPart { .. }
                    | PlayerFile::PreFoxCommonModel { .. }
                    | PlayerFile::Material
                    | PlayerFile::CommonMaterial
                    | PlayerFile::FaceXml
                    | PlayerFile::ConversionSkeleton
                    | PlayerFile::UnsupportedGltf => None,
                };
                if let Some(packs_as) = packs_as {
                    if packed.contains(&packs_as) {
                        continue;
                    }
                    packed.push(packs_as);
                }
                source_roles.push((file, role));
            }
            roles.push((package, path, source_roles));
        }
        roles
    }

    /// The `face.xml` the face reads, with the export path of the source holding it: the
    /// member's own, the first of the folder's own files with the role `PlayerFile::FaceXml`
    /// (pre-Fox, a face file, so never under `ingame_face`), else the one of the shared face
    /// folder the player combines; `None` when neither holds one. The deep pass checked every
    /// one, and dropped a player folder holding its own beside a face link
    /// (`xml_shared_face_conflict`). The face task writes this one back in place of a generated
    /// `face.xml` (`spec/team_compiler/messages.md` "User-supplied face.xml").
    pub(crate) fn face_xml(&self) -> Option<(&ScopePath, &FileDescriptor)> {
        self.roles().into_iter().find_map(|(_, source, files)| {
            files
                .into_iter()
                .find(|(_, role)| *role == PlayerFile::FaceXml)
                .map(|(file, _)| (source, file))
        })
    }

    /// Whether the folder's textures task emits the template environment map: the folder is
    /// flagged (`environment_map`) and none of its sources holds an `env` texture directly in
    /// it or a texture link of that stem (`env.dds.common`), the one the converted metal
    /// materials then name. A link's `below` carries its whole path below `Common/`, so only a
    /// root link matches: a subfolder's `env`, its own or its link's (`jessie/env.dds.common`),
    /// sits at its own path, so the template, at the root of the texture home, does not
    /// collide with it, and a model beside it names it first (`texture_lookup`).
    pub(crate) fn takes_template_environment_map(&self) -> bool {
        self.environment_map
            && !self.roles().into_iter().any(|(_, _, files)| {
                files.into_iter().any(|(_, role)| {
                matches!(&role, PlayerFile::Texture { below, .. } | PlayerFile::CommonTexture(below)
                        if vtree::fold_name(below) == ENVIRONMENT_MAP_STEM)
            })
            })
    }

    /// Whether `file`, a skeleton (`PlayerFile::Skeleton`), is the `.skl` of one of the
    /// folder's hand-split parts (`hand_split`), paired with it as the Models task pairs a
    /// skeleton: by the path up to the extension, folded. The gloves task reads it as the bind
    /// pose of the part whose hands it converts again.
    pub(crate) fn hand_split_skeleton(&self, file: &FileDescriptor) -> bool {
        let stem = vtree::fold_name(file_stem(file.path.as_str()));
        self.hand_split
            .iter()
            .any(|model| vtree::fold_name(file_stem(model.as_str())) == stem)
    }

    /// The Common model the folder's `.common` link at `link` resolved to (`common_models`).
    fn common_model(&self, link: &ScopePath) -> &CommonModel {
        self.common_models
            .iter()
            .find(|common| &common.link == link)
            .expect("planning resolves every `.common` model link of a folder")
    }

    /// The `.mtl` planning resolved for the Common `.model` at `model`, which a `.common` link
    /// of the folder's sources loads (`CommonModel::material`); `None` for a Common FMDL.
    /// `roles` reads the model once, under its first source's link's role, so the first
    /// link's `.mtl` is the one.
    pub(crate) fn common_material(&self, model: &ScopePath) -> Option<&FileDescriptor> {
        self.common_models
            .iter()
            .find(|common| &common.model.path == model)
            .and_then(|common| common.material.as_ref())
    }

    /// The package the folder's own files feed, which its pre-Fox models
    /// (`PlayerFile::PreFoxModel`) go into: a player folder's stand for its face, a shared
    /// folder's for its one package. A player holding `ingame_face` has no face on either
    /// engine, so his own files feed his boots. His models are parts of his boots or
    /// gloves, each role saying which (`PlayerFile::PreFoxPart` on pre-Fox), and his pre-Fox
    /// `.mtl` files are read by both (`TaskKind::files`): this answer is not where they go.
    fn own_package(&self) -> ModelPackage {
        match &self.textures {
            TextureHome::PlayerCommon { .. } if self.ingame_face => ModelPackage::Boots,
            TextureHome::PlayerCommon { .. } => ModelPackage::Face,
            TextureHome::SharedOutput { package, .. } => *package,
        }
    }
}

/// Pushes the Common `file` under `role` onto `roles` unless `pushed`, the Common files pushed
/// so far for the folder, holds it already, and says whether it pushed: two links standing for
/// one Common file give it once.
fn push_common<'a>(
    roles: &mut Vec<(&'a FileDescriptor, PlayerFile)>,
    pushed: &mut BTreeSet<String>,
    file: &'a FileDescriptor,
    role: PlayerFile,
) -> bool {
    if !pushed.insert(file.path.fold_key()) {
        return false;
    }
    roles.push((file, role));
    true
}

/// Pushes the `.mtl` the Common `.model` of `common` takes onto `roles` as a
/// `PlayerFile::Material` (`push_common`), when its search found a `Common/` one: a player's
/// own override is among his files already. Nothing for a Common FMDL, which takes none.
fn push_common_material<'a>(
    roles: &mut Vec<(&'a FileDescriptor, PlayerFile)>,
    pushed: &mut BTreeSet<String>,
    common: &'a CommonModel,
) {
    if let Some(material) = common
        .material
        .as_ref()
        .filter(|material| is_common_file(&material.path))
    {
        push_common(roles, pushed, material, PlayerFile::Material);
    }
}

/// What a task compiles.
pub(crate) enum TaskKind {
    /// One package of a model folder's models: a player folder's face, boots or gloves, or a
    /// shared folder's one package, with the files packed beside them. One task per package,
    /// whatever the number of roster slots mapping a player folder: the package is built once
    /// and emitted under each slot's key.
    Models {
        /// The model folder.
        folder: ModelFolder,
        /// Which of its packages.
        package: ModelPackage,
        /// The keys the package is emitted under: for a player folder one per roster slot
        /// mapping it, in slot order (the slot's player id for the face, its planned
        /// boots/gloves id for the other two; a referee slot's own key for all three); for a
        /// shared folder its one shared id.
        ids: Vec<PackageKey>,
        /// The export's kit numbers, against which a pre-Fox face completes the per-kit model
        /// sets its `face.xml` lists.
        kits: Vec<u8>,
    },
    /// A model folder's own textures, converted once into the folder's texture home, which
    /// every package of the folder points at. Always the last task of the folder's `TaskGroup`.
    Textures {
        /// The model folder.
        folder: ModelFolder,
        /// The export's kit numbers, against which its texture variant sets are completed.
        kits: Vec<u8>,
    },
    /// The textures of a texture directory of the export's `Common/` folder, converted once
    /// into the team's Common output, whether or not a `.common` link uses them. A texture
    /// directory is `Common/` itself or a subfolder, packed at its own path. It is the
    /// directory above the first kit-token folder below `Common/`, so a set's `kit1`…`kit9`
    /// folders complete in the task holding them all. One task per such directory, in no
    /// group: its textures serve every linking player, so it commits on its own, and when it
    /// fails the linking players still commit.
    CommonTextures {
        /// The directory's export path, the scope the task's findings name and, below
        /// `Common/`, the path its output takes under the team's Common output.
        folder: ScopePath,
        /// Its textures, its own and the files below its kit-token folders, in any
        /// accepted image format.
        textures: Vec<FileDescriptor>,
        /// The export's kit numbers, against which its texture variant sets are completed.
        kits: Vec<u8>,
        /// A Common models task of any directory of `Common/` converts an FMDL holding a metal
        /// material for PES 15-17 (`ExportToPlan::metal_models`) and no texture directly in
        /// `Common/` has the stem `env`: `Common/`'s own task emits the template environment map
        /// as `env.dds` in the team's Common output, and is planned for it even when `Common/`
        /// holds no texture (or no file). The conversion points each converted `Basic_CNSR`
        /// material with no environment sampler at it, whichever directory its FMDL sits in.
        /// Never set on PES 18-21, nor on a subfolder's task.
        environment_map: bool,
    },
    /// Pre-Fox: the `.model` and `.mtl` files of a directory of the export's `Common/` folder
    /// (`Common/` itself or a subfolder, packed at its own path), written once into the team's
    /// Common output, whether or not a `.common` link or a member's `face.xml` names them. The
    /// game loads a linked Common model or `.mtl` from there. An FMDL there no `.model`
    /// of its stem beats is converted once into a `.model` and its material set, the `.skl` of
    /// its stem its bind pose. One task per directory holding such a file, in no group, as
    /// `CommonTextures`.
    CommonModels {
        /// The directory's export path, the scope the task's findings name and, below
        /// `Common/`, the path its output takes under the team's Common output.
        folder: ScopePath,
        /// Its `.model` and `.mtl` files directly in it, and each FMDL it converts with the
        /// `.skl` of that FMDL's stem (`common_model_files`).
        files: Vec<FileDescriptor>,
        /// The paths below `Common/` of the export's Common textures, without their
        /// extensions (`below_common`, `file_stem`): the places the task's `.mtl` files
        /// resolve their texture paths in as a model folder's do, `Common/` their model
        /// folder: a name nearest first from the `.mtl`'s directory up to `Common/`, a
        /// path below its directory at that path (`prefox_common`).
        textures: Vec<String>,
    },
    /// One player's portrait, emitted as a DDS under the target version's file name: a DDS
    /// source as it is, any other accepted format encoded to BC3. One task per player id: a
    /// folder two roster slots map gives two tasks over its one `portrait.*`.
    Portrait {
        /// The player id the portrait is for.
        player_id: u32,
        /// The portrait file: the player folder's `portrait.*`, or `Portraits/NN.*`.
        file: FileDescriptor,
    },
    /// One kit, its `all/` inheritance already applied to its textures.
    Kit {
        /// The game's kit slot.
        slot: KitSlot,
        /// The kit folder.
        kit: KitFolder,
        /// What its team's export sets in every kit config of the team.
        edits: TeamKitEdits,
        /// The export's coverage: a `Midcup` kit keeps what it does not resend — no
        /// config of its own without `config.toml`, no placeholder without a main
        /// texture, no `UniColor.bin` entry without colors to give.
        coverage: ExportCoverage,
    },
    /// The team's logo: the game's three PNGs made from the export's root `logo*` file, the
    /// smallest from its `logo_small*` file when there is one. One task per export that has a
    /// logo, all three or none.
    Logo {
        /// The main file and the small one, each with its fit tag.
        logo: LogoFiles,
    },
    /// The referees' marker: the refs export's `ref_marker.dds` converted into the referees'
    /// Common output, and the bundled marker model written as their reserved collar, its
    /// texture pointed at it. Both or neither: a
    /// collar naming a texture the CPK lacks would draw untextured under every referee. One
    /// task per refs export holding the file.
    RefereeMarker {
        /// The export's root `ref_marker.dds`.
        marker: FileDescriptor,
    },
    /// The team's collar: its `Collars/collar_<ID>` model in the format the target reads
    /// (`.fmdl` on Fox, `.model` on pre-Fox) written unchanged in place of stock collar `id`,
    /// keeping the materials its author gave it. One task per team export holding a collar it
    /// claimed (`collars::export_collar`).
    Collar {
        /// The collar file.
        file: FileDescriptor,
        /// The stock collar it replaces.
        id: u8,
    },
}

/// What a team export sets in every kit config of its team: its own kits' and, for a `Midcup`
/// export, those of the kits it does not resend. The FPC values go first, then its collar, so
/// the custom collar wins over the FPC collar
/// (`spec/aesthetics_export/fpc_toggle.md` "Team kit-FPC status and kit configs").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TeamKitEdits {
    /// The team's kit-FPC status.
    pub(crate) fpc: EffectiveTeamKitFpc,
    /// The stock collar the export's collar replaces, set as every config's collar and winter
    /// collar; `None` when the export holds no collar or lost its claim.
    pub(crate) collar: Option<u8>,
}

/// Whether the team's kit configs must carry the FPC values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EffectiveTeamKitFpc {
    /// At least one player folder of the validated export carries `fpc_on`.
    On,
    /// No `fpc_on` marker: no claim about the team, supplied configs are left as they are.
    Unknown,
}

impl EffectiveTeamKitFpc {
    /// The status of the team of `export`, the validated export: `On` when one of its player
    /// folders a roster slot maps (`mapped_players`) carries `fpc_on`. A folder validation
    /// dropped, or planning dropped for a selected glTF (`drop_gltf_folders`, which removes its
    /// slots), compiles no player, so its marker does not count, and `fpc_off` is a statement
    /// about its own player alone.
    pub(crate) fn of(export: &ValidatedAestheticsExport) -> EffectiveTeamKitFpc {
        if mapped_players(export)
            .iter()
            .any(|folder| folder.fpc == Some(FpcDirective::On))
        {
            EffectiveTeamKitFpc::On
        } else {
            EffectiveTeamKitFpc::Unknown
        }
    }
}

impl TaskKind {
    /// The folder the task compiles, as the export spells it (a portrait's is its file, the
    /// logo's its main file): the scope its findings name.
    pub(crate) fn folder_path(&self) -> ScopePath {
        match self {
            TaskKind::Models { folder, .. } | TaskKind::Textures { folder, .. } => {
                folder.path.clone()
            }
            TaskKind::CommonTextures { folder, .. } | TaskKind::CommonModels { folder, .. } => {
                folder.clone()
            }
            TaskKind::Portrait { file, .. } => file.path.clone(),
            TaskKind::Kit { kit, .. } => kit.path.clone(),
            TaskKind::Logo { logo } => logo.main.file.path.clone(),
            TaskKind::RefereeMarker { marker } => marker.path.clone(),
            TaskKind::Collar { file, .. } => file.path.clone(),
        }
    }

    /// Every file the task reads from its export: a package's models (a `.common` link's
    /// Common model and skeleton, never the link) and the files packed beside them, the Fox
    /// gloves' also the folder's hand-split face parts, whose hands they take, with each part's
    /// `.skl`, its bind pose (`ModelFolder::hand_split_skeleton`), a Fox package
    /// converting a `.model` also the `.mtl` files of the model's source (for a Common
    /// `.model`, the player's and the Common `.mtl` its search found) and the Common `.mtl`
    /// each `.mtl.common` link of the folder stands for, a pre-Fox package
    /// also the skeleton of each FMDL it may convert, and the pre-Fox face the Common `.mtl`
    /// each `.mtl.common` link of the folder stands for, which the conversion pre-check of a
    /// model using it reads; a folder's
    /// textures; the Common textures; the Common models and `.mtl` files, a converted Common
    /// FMDL's skeleton included (pre-Fox); a
    /// portrait's one file; a kit's config and `colors.txt`,
    /// when it has them, and its effective textures; the logo's main file and its small one,
    /// when it has one; the referees' marker texture; the collar file.
    pub(crate) fn files(&self) -> Vec<&FileDescriptor> {
        match self {
            TaskKind::Models {
                folder, package, ..
            } => {
                let reads_model = |source, file: &FileDescriptor, role: &PlayerFile| {
                    let hands = match folder.engine {
                        Engine::Fox => {
                            *package == ModelPackage::Gloves
                                && (folder.hand_split.contains(&file.path)
                                    || (matches!(role, PlayerFile::Skeleton { .. })
                                        && folder.hand_split_skeleton(file)))
                        }
                        // A pre-Fox split model is the face's alone (`folder_tasks`).
                        Engine::PreFox => false,
                    };
                    // A `.mtl` and a pre-Fox model are the face's only by `PlayerFile::package`'s
                    // pre-Fox answer, whatever their source: each is read by the package its
                    // source feeds (below), so a pre-Fox face does not read the shared boots of
                    // a referee's plain link, which his slot's boots folder writes alone.
                    let read_by_source =
                        matches!(role, PlayerFile::Material | PlayerFile::PreFoxModel { .. });
                    (role.package() == Some(*package) && !read_by_source)
                        || hands
                        // A pre-Fox model goes into the package its source feeds: a shared
                        // boots or gloves folder's into its own output, a folder an
                        // `ingame_face` player combines into his package of its kind.
                        || (matches!(role, PlayerFile::PreFoxModel { .. }) && *package == source)
                };
                // The sources of the `.model` files this package reads, whose `.mtl` files it
                // reads too on Fox (below). There only a package converting a `.model` reads a
                // `.mtl`: the FMDLs carry their materials, and a `.mtl` beside a `.model` an
                // FMDL beats is read by nothing.
                let model_sources: BTreeSet<&ScopePath> = folder
                    .roles()
                    .into_iter()
                    .filter(|(source, _, files)| {
                        files.iter().any(|(file, role)| {
                            file.kind == FileKind::Model(ModelFormat::PesModel)
                                && reads_model(*source, file, role)
                        })
                    })
                    .map(|(_, source_path, _)| source_path)
                    .collect();
                let mut read = folder_files(folder, |source, source_path, file, role| {
                    reads_model(source, file, role)
                        || (matches!(role, PlayerFile::Material)
                            && match folder.engine {
                                // A `.mtl` belongs to the `.model` files of its source, which
                                // take their materials from it (`mtl_for`), so it goes wherever
                                // one of them is read. Not by the package the source feeds: a
                                // linked face folder's `boots.model`, a part of the player's
                                // boots, takes that folder's `.mtl` into his boots, though the
                                // folder feeds his face.
                                Engine::Fox => model_sources.contains(source_path),
                                // A `.mtl` goes where its source's models go, each package
                                // packing the ones its models use: a combined folder's into
                                // the player's package of its kind, the folder's own into each
                                // of its packages, a `.common` link to a `.model` taking a
                                // `.mtl` of its name from them though no package reads the
                                // link (TC-MOD-24), and an `ingame_face` player's models being
                                // parts of his boots and of his gloves.
                                Engine::PreFox => *package == source || source_path == &folder.path,
                            })
                        // A pre-Fox FMDL's skeleton, its bind pose, goes where its models go,
                        // as a `.mtl` does: the face's, a shared boots or gloves output's, a
                        // combined folder's into the player's package of its kind, and an
                        // `ingame_face` player's own into his boots and his gloves, either of
                        // which may convert the FMDL it pairs with.
                        || (matches!(role, PlayerFile::ConversionSkeleton)
                            && (*package == source || source_path == &folder.path))
                });
                // The Common `.mtl` each `.mtl.common` link stands for is read where a model's
                // search may find it: on Fox by a package converting a `.model`, which converts
                // it with the one its search finds (`processing::model`); on pre-Fox by the
                // face, where a member's model whose search finds it runs the conversion
                // pre-check with it (`prefox_face::face`). Only the face there: an
                // `ingame_face` player, who has none, gets his link's Common `.mtl` as a
                // `PlayerFile::Material` already (`ModelFolder::roles`), as he does on Fox,
                // where it is read once.
                let reads_links = match folder.engine {
                    Engine::Fox => !model_sources.is_empty(),
                    Engine::PreFox => *package == ModelPackage::Face,
                };
                if reads_links {
                    for material in linked_common_materials(folder) {
                        if !read.iter().any(|file| file.path == material.path) {
                            read.push(material);
                        }
                    }
                }
                read
            }
            TaskKind::Textures { folder, .. } => folder_files(folder, |_, _, _, role| {
                matches!(role, PlayerFile::Texture { .. })
            }),
            TaskKind::CommonTextures { textures, .. } => textures.iter().collect(),
            TaskKind::CommonModels { files, .. } => files.iter().collect(),
            TaskKind::Portrait { file, .. } => vec![file],
            TaskKind::Kit { kit, .. } => kit
                .config
                .iter()
                .chain(&kit.colors)
                .chain(kit.textures.iter().map(|texture| &texture.file))
                .collect(),
            TaskKind::Logo { logo } => std::iter::once(&logo.main)
                .chain(&logo.small)
                .map(|file| &file.file)
                .collect(),
            TaskKind::RefereeMarker { marker } => vec![marker],
            TaskKind::Collar { file, .. } => vec![file],
        }
    }
}

/// The files of `folder` that `wanted` accepts, given each with its source (`ModelFolder::roles`:
/// the package it feeds and its export path) and its role: its own in their order, then each
/// combined folder's.
fn folder_files(
    folder: &ModelFolder,
    wanted: impl Fn(ModelPackage, &ScopePath, &FileDescriptor, &PlayerFile) -> bool,
) -> Vec<&FileDescriptor> {
    let mut wanted_files = Vec::new();
    for (source, source_path, files) in folder.roles() {
        for (file, role) in files {
            if wanted(source, source_path, file, &role) {
                wanted_files.push(file);
            }
        }
    }
    wanted_files
}

/// The `Common/` `.mtl` files `folder`'s `.mtl.common` links stand for
/// (`PlayerFile::CommonMaterial`), its own files' and its combined folders', each once however
/// many links name it (`x.mtl.common` and `x.mtl.common.txt`).
fn linked_common_materials(folder: &ModelFolder) -> Vec<&FileDescriptor> {
    let mut materials: Vec<&FileDescriptor> = Vec::new();
    for (_, source_path, files) in folder.roles() {
        for (file, role) in files {
            if !matches!(role, PlayerFile::CommonMaterial) {
                continue;
            }
            let linked = common_link_target(&file.path, source_path)
                .expect("a CommonMaterial role implies a `.common` link below its folder");
            let material = common_file(&folder.common_files, &linked)
                .expect("validation drops a player folder whose link names no Common file");
            if !materials.iter().any(|known| known.path == material.path) {
                materials.push(material);
            }
        }
    }
    materials
}

/// The export paths of `folder`'s hand-split parts on a target of `engine`
/// (`ModelFolder::hand_split`): its face models whose path is among `hand_weighted`, the
/// models the deep pass found carrying hand weights (`is_hand_split`). On Fox those are its
/// face parts, its own, a combined folder's or a Common model's. On pre-Fox they are the
/// `.model` files its face packs and names as face content, its own and a combined shared
/// face's: never a model a `.common` link brings in, which the face lists by reference in the
/// team's Common output and packs nothing of, and never an `ingame_face` player's part, which
/// has no face. A face with a `face.xml`, the member's own or his linked shared face's, has
/// none (`ModelFolder::face_xml`).
fn hand_split_parts(
    folder: &ModelFolder,
    hand_weighted: &BTreeSet<ScopePath>,
    engine: Engine,
) -> BTreeSet<ScopePath> {
    // The member's xml says what the face loads: a split would add glove entries he did not
    // write.
    if folder.face_xml().is_some() {
        return BTreeSet::new();
    }
    folder
        .roles()
        .into_iter()
        .flat_map(|(_, _, files)| {
            files
                .into_iter()
                .filter(move |(file, role)| is_hand_split(engine, file, role, hand_weighted))
        })
        .map(|(file, _)| file.path.clone())
        .collect()
}

/// Whether one of `folder`'s packages converts an FMDL among `metal_models`, the FMDLs the deep
/// pass found holding a metal material (`ModelFolder::environment_map`): one of its sources'
/// files a pre-Fox package converts, a face's or a shared boots or gloves output's model
/// (`PlayerFile::PreFoxModel`) or a part of an `ingame_face` player's boots or gloves
/// (`PlayerFile::PreFoxPart`), roles only a pre-Fox target gives (a `.model` with one is never
/// among the metal models, which are FMDLs). A folder holding its own `face.xml` converts
/// none: the xml lists the face's models, and names no converted one. With the `face.xml` of
/// the shared face he links, that folder's models are the xml's, but his own are converted
/// as without it (`ModelFolder::face_xml`).
fn converts_metal(folder: &ModelFolder, metal_models: &BTreeSet<ScopePath>) -> bool {
    let xml_source = folder.face_xml().map(|(source, _)| source);
    if xml_source == Some(&folder.path) {
        return false;
    }
    let mut sources = folder
        .roles()
        .into_iter()
        .filter(|(_, source, _)| Some(*source) != xml_source);
    sources.any(|(_, _, files)| {
        files.into_iter().any(|(file, role)| {
            let converted = matches!(
                role,
                PlayerFile::PreFoxModel { .. }
                    | PlayerFile::PreFoxPart {
                        package: ModelPackage::Boots | ModelPackage::Gloves,
                        ..
                    }
            );
            converted && metal_models.contains(&file.path)
        })
    })
}

/// Every player folder a roster slot maps, in the export's folder order. A folder no slot maps
/// is not compiled.
pub(crate) fn mapped_players(export: &ValidatedAestheticsExport) -> Vec<&PlayerFolder> {
    let mapped: Vec<PlayerIndex> = match &export.roster {
        ValidatedRoster::Team(slots) => slots.values().copied().collect(),
        ValidatedRoster::Referees(slots) => slots.values().copied().collect(),
    };
    export
        .players
        .iter()
        .enumerate()
        .filter(|(index, _)| mapped.contains(&PlayerIndex(*index)))
        .map(|(_, folder)| folder)
        .collect()
}

/// One identity-resolved export as planning takes it, with what validation read beside it.
pub(crate) struct ExportToPlan {
    /// The export's id.
    pub(crate) export_id: ExportId,
    /// The export, its identity resolved.
    pub(crate) export: ResolvedAestheticsExport,
    /// The valid colors of its root `colors.txt`; `None` when it has no such file.
    pub(crate) team_colors: Option<Vec<Rgb>>,
    /// The text of its root `notes.txt`; `None` when it has none.
    pub(crate) notes: Option<String>,
    /// The team's shared-IDs record the installed CPKs hold; `None` for a referee export, or
    /// when no installed CPK holds one for the team.
    pub(crate) shared_ids: Option<SharedIds>,
    /// The export paths of its models (`.fmdl`, `.model`) whose vertices carry hand weights,
    /// as the deep pass found them (`deep::ContentPass::hand_weighted`): a player's face model
    /// among them is hand auto-split (`ModelFolder::hand_split`).
    pub(crate) hand_weighted: BTreeSet<ScopePath>,
    /// The export paths of its FMDLs holding a metal material, as the deep pass found them
    /// (`deep::ContentPass::metal_models`): a model folder one of whose pre-Fox packages
    /// converts one of them gets the template environment map (`ModelFolder::environment_map`).
    pub(crate) metal_models: BTreeSet<ScopePath>,
    /// Each player folder's settings, its `settings.toml` joined with its
    /// `ingame_face.toml`'s keys, by folder path, as the deep pass found them
    /// (`deep::ContentPass::settings`): the compiled players' patch entries resolve from
    /// them (`export_rows`, `patch::patch_team`).
    pub(crate) settings: BTreeMap<ScopePath, PlayerSettings>,
    /// The player folders whose `settings.toml` or `ingame_face.toml` was refused, by
    /// folder path, as the deep pass found them (`deep::ContentPass::settings_refused`):
    /// the patch holds no `[appearance]` for their players, an invalid file holding back
    /// the player's whole look.
    pub(crate) settings_refused: BTreeSet<ScopePath>,
    /// The export's `legacy_ids.toml` parsed, as the deep pass found it
    /// (`deep::ContentPass::legacy_ids`): the old boots and gloves IDs of a VGL27 export,
    /// which `mapped_folders` and `export_rows` pack and patch under instead of the
    /// team's block; `None` when the export has none.
    pub(crate) legacy_ids: Option<LegacyIds>,
    /// The export's team's VGL27 old-ID block (`legacy_blocks::LegacyBlocks`), read only
    /// under `legacy_ids`: `None` then means the table does not list the team and every
    /// recorded old ID is out of block (`legacy_id_out_of_block`).
    pub(crate) legacy_block: Option<RangeInclusive<u32>>,
}

/// Plans the run over the identity-resolved exports, given in `ExportId` order, for the target
/// `version`. A mapped player folder whose model is a selected glTF is dropped first, and so
/// is a shared folder whose model is one, with every player folder linking it
/// (`drop_gltf_folders`), and a glTF directly in `Common/` with no model of the target's format
/// of its stem, the file alone, with the other engine's model of its stem it beats
/// (`drop_common_gltfs`). The textures a team export's kit holds that the target does not emit
/// are dropped from it (`drop_unused_kit_textures`). Every export's note goes into the
/// manifest, and a team export's colors; one with no root `colors.txt` reports
/// `team_colors_missing`, and its team keeps the colors it had. A team export's collar is
/// claimed against the run-wide list of the collars earlier exports claimed
/// (`collars::export_collar`), and every kit config of the team wears the collar it keeps. A
/// refs export plans its referee folders, shared folders, Common textures and marker under
/// team id 999 (`REFEREE_TEAM_ID`), each folder's packages keyed by its referee slots, and
/// nothing else: it has no colors record, kits, rows, portraits, logo or collar.
/// `hide_ids` is the common settings' FPC hide IDs, the `fpc_on` marker's
/// default boots and gloves IDs (`item_rows::default_id`).
pub(crate) fn plan_run(
    exports: Vec<ExportToPlan>,
    version: PesVersion,
    hide_ids: fpc::HideIds,
) -> PlanReport {
    let mut tasks = Vec::new();
    let mut team_colors = Vec::new();
    let mut team_kits = Vec::new();
    let mut patch_teams = Vec::new();
    let mut notes = Vec::new();
    let mut shared_ids = Vec::new();
    let mut messages = Vec::new();
    // Each claimed collar's ID and its claimant's name. Planning is serial and takes the
    // exports in canonical order, so the earlier export keeps a collar two exports claim.
    let mut claimed_collars: BTreeMap<u8, String> = BTreeMap::new();
    for ExportToPlan {
        export_id,
        export: mut resolved,
        team_colors: colors,
        notes: note,
        shared_ids: installed_ids,
        hand_weighted,
        metal_models,
        settings: mut player_settings,
        settings_refused,
        legacy_ids,
        legacy_block,
    } in exports
    {
        drop_gltf_folders(export_id, &mut resolved, version, &mut messages);
        // The referees have no team record (no colors, kits or rows) and no player ids, so
        // only their folders compile: planning keeps their kits, logo, portraits and collars
        // out (below), and validation reports each as `file_not_used`.
        let team = match resolved.identity {
            ExportIdentity::Team { id, .. } => Some(id),
            ExportIdentity::Referees => None,
        };
        // A refs export's kit folder is `file_not_used` whole and gets no task: its files are
        // not reported again one by one.
        if team.is_some() {
            drop_unused_kit_textures(
                version.engine(),
                export_id,
                &mut resolved.export.kits,
                &mut messages,
            );
        }
        let team_id = team.map_or(REFEREE_TEAM_ID, TeamId::get);
        if team.is_some() {
            match colors {
                None => messages.push(tool_message(
                    Code::TeamColorsMissing,
                    Scope::Export { export_id },
                    Disposition::Keep,
                    vec![],
                )),
                // A file whose every line was refused: the deep pass reported the lines, and
                // there is nothing to write.
                Some(colors) if colors.is_empty() => {}
                Some(colors) => team_colors.push((team_id, colors)),
            }
        }
        let mut export = resolved.export;
        // The export's team name (`/co/`, `/refs/`) heads its note.
        if let Some(note) = note {
            notes.push((export.team_name.as_str().to_owned(), note));
        }
        let model_ids = team.map(PlannedModelIds::for_team);
        let fpc = EffectiveTeamKitFpc::of(&export);
        // The kit numbers the export defines, ascending (its kits go by slot), which each
        // textures task, and each pre-Fox face, completes its variant sets against. A refs
        // export's kit folder is `file_not_used` whole: it defines none.
        let kits: Vec<u8> = match team {
            Some(_) => export
                .kits
                .kits
                .keys()
                .filter_map(|slot| kit_number(*slot))
                .collect(),
            None => Vec::new(),
        };
        kit_variant_model_messages(
            export_id,
            &export,
            version.engine(),
            &hand_weighted,
            &mut messages,
        );
        // What PES 18-21 reads of a `Common/` subfolder: the files a model folder's link
        // reaches.
        let linked_common = linked_common_files(&export);
        drop_common_gltfs(
            version.engine(),
            export_id,
            &mut export.common,
            &linked_common,
            &mut messages,
        );
        // The stems of the textures directly in `Common/`, which the Common textures task emits
        // into the team's Common output and a Common part's paths, in a model of any folder,
        // name that output for. A subfolder's textures are packed under its own path, which no
        // player's model points at. As spelled, not folded: the Models task's Common lookup
        // reads a `kitF` file's token from its spelling (`kit_lookup_keys`).
        let common_texture_stems: BTreeSet<String> = export
            .common
            .iter()
            .filter(|file| {
                is_direct_root_folder_file(&file.path) && texture_format(file.path.name()).is_some()
            })
            .map(|file| file_stem(file.path.name()).to_owned())
            .collect();
        // Pre-Fox loads Common at run time: its `.model` and `.mtl` files, and its FMDLs
        // converted, are one more task of the export's per directory, and a player's face
        // names them and its textures, a member's `face.xml` a subfolder's too, which its task
        // finds among the model, `.mtl` and texture files. Fox reads Common through a model
        // folder's links alone: a model link's Common model is resolved per folder (`common_models`),
        // and a `.mtl.common` link's Common `.mtl` is among the `.mtl` files a player's
        // `.model` search resolves the link against (`mtl_search::mtl_for`), the ones directly
        // in `Common/` and the subfolder ones a link reaches (`linked_common_files`): a player
        // whose links stand elsewhere has those of another's too, which no search of his looks
        // at, a `.model` link searching its own model's directory and a `.mtl` link naming its
        // file. The subfolder textures a link reaches are there too, the one a texture link
        // names and the ones a linked model or `.mtl` brings from its directory, the tasks
        // spelling each path as `Common/` does (`roles::common_texture_below`). Nothing reads
        // any other `Common/` file.
        let player_common_files: Vec<FileDescriptor> = match version.engine() {
            Engine::Fox => export
                .common
                .iter()
                .filter(|file| {
                    (file.kind == FileKind::Mtl
                        && is_read_common_file(&file.path, Engine::Fox, &linked_common))
                        || (file.kind == FileKind::Texture && linked_common.contains(&file.path))
                })
                .cloned()
                .collect(),
            Engine::PreFox => export
                .common
                .iter()
                .filter(|file| {
                    matches!(
                        file.kind,
                        FileKind::Model(ModelFormat::PesModel | ModelFormat::Fmdl) | FileKind::Mtl
                    ) || texture_format(file.path.name()).is_some()
                })
                .cloned()
                .collect(),
        };
        // The shared folders taking an id, each with its package and that id, in the id order
        // of the kind: the boots folders, then the gloves folders. A folder exactly one roster
        // slot links plainly takes that slot's exclusive id unless the record names it; every
        // other takes a shared id, which a `Midcup` export's record keeps as the installed
        // record gives it and a `Full` export's, or one the installed CPKs hold no record of,
        // numbers on its own. The shared ones alone are recorded.
        let mut shared: Vec<(ModelFolder, ModelPackage, u32)> = Vec::new();
        let midcup_record = match export.coverage {
            ExportCoverage::Midcup => installed_ids.as_ref(),
            ExportCoverage::Full => None,
        };
        let mut output_record = midcup_record.cloned().unwrap_or_default();
        for kind in [SharedKind::Boots, SharedKind::Gloves] {
            let package = package_of(kind);
            let taking = folder_ids(
                &export,
                version.engine(),
                kind,
                &hand_weighted,
                midcup_record,
            );
            let assignment = match model_ids {
                Some(ids) => assigned_shared_ids(ids, kind, &taking.shared, midcup_record).expect(
                    "a referee's links take no shared id, and the structure pass drops a \
                         team export whose shared pool is exhausted",
                ),
                None => Vec::new(),
            };
            let assigned: Vec<(u16, String)> = taking
                .shared
                .iter()
                .zip(assignment.iter())
                .map(|(folder, id)| (*id, folder.folder_name.clone()))
                .collect();
            output_record = output_record.extended(kind, &assigned);
            let mut packing: Vec<(&SharedModelFolder, u32)> = match model_ids {
                Some(ids) => taking
                    .own
                    .iter()
                    .map(|(folder, slot)| (*folder, u32::from(ids.exclusive(*slot))))
                    .chain(
                        taking
                            .shared
                            .iter()
                            .zip(assignment)
                            .map(|(folder, id)| (*folder, u32::from(id))),
                    )
                    .collect(),
                None => Vec::new(),
            };
            // A legacy export's `legacy_ids.toml`: a folder packs under the old ID its
            // first plain linker in roster order records for the kind, when one does
            // and it lies in the team's VGL27 block; outside the block the folder is
            // not packed at all (`legacy_id_out_of_block`), its wearers' patch entries
            // pointing at the old ID. Otherwise it keeps the ID the team's block gave it.
            if let Some(legacy) = &legacy_ids {
                let mut refused = Vec::new();
                for (folder, id) in &mut packing {
                    if let Some(old) = legacy_folder_id(
                        &export,
                        version.engine(),
                        kind,
                        folder,
                        &hand_weighted,
                        legacy,
                    ) {
                        if in_block(legacy_block.as_ref(), old) {
                            *id = old;
                        } else {
                            refused.push((folder.path.clone(), old));
                        }
                    }
                }
                packing.retain(|(folder, _)| !refused.iter().any(|(path, _)| path == &folder.path));
                for (path, old) in refused {
                    messages.push(tool_message(
                        Code::LegacyIdOutOfBlock,
                        Scope::Folder {
                            export_id,
                            path: path.clone(),
                        },
                        Disposition::Keep,
                        vec![
                            ("id", old.to_string()),
                            ("folder", path.as_str().to_owned()),
                            ("block", block_text(legacy_block.as_ref())),
                        ],
                    ));
                }
            }
            packing.sort_by_key(|(_, id)| *id);
            for (folder, shared_id) in packing {
                let mut folder = ModelFolder {
                    path: folder.path.clone(),
                    files: folder.files.clone(),
                    ingame_face: false,
                    engine: version.engine(),
                    combined: Vec::new(),
                    common_models: common_models(
                        &folder.path,
                        &folder.files,
                        Some(kind),
                        false,
                        &[],
                        &export.common,
                        version.engine(),
                    ),
                    common_texture_stems: common_texture_stems.clone(),
                    // A shared folder's `.common` links resolve in `Common/` as a player
                    // folder's do, against the same files; on pre-Fox its links are unread.
                    common_files: match version.engine() {
                        Engine::Fox => player_common_files.clone(),
                        Engine::PreFox => Vec::new(),
                    },
                    hand_split: BTreeSet::new(),
                    environment_map: false,
                    refkit_body: false,
                    textures: TextureHome::SharedOutput {
                        package,
                        id: shared_id,
                    },
                };
                folder.environment_map = converts_metal(&folder, &metal_models);
                shared.push((folder, package, shared_id));
            }
        }
        // A folder's portrait goes out once per slot mapping the folder. A player id with a
        // `Portraits/` file too comes up twice; the deep pass has skipped an export whose two
        // files differ, so they are the same bytes and the folder's is packed once (below).
        let mut portraits: Vec<(u32, FileDescriptor)> = Vec::new();
        // The player folders are taken out so the rest of the export (its shared folders)
        // stays readable while each folder's links are resolved against it.
        let players = std::mem::take(&mut export.players);
        // The export's first task, from which its compiled players' rows look for the tasks
        // building their boots and gloves.
        let first_task = tasks.len();
        let mut row_players = Vec::new();
        let mapped = mapped_folders(
            players,
            &export.roster,
            team,
            legacy_ids.as_ref(),
            legacy_block.as_ref(),
        );
        for MappedFolder {
            folder,
            packages,
            player_ids,
            refused,
        } in mapped
        {
            // The packages the folder's effective boots and gloves have a part of
            // (`has_effective_part`), computed only under `legacy_ids`: its rows and
            // a refused old ID's report read them.
            let own_parts: Vec<ModelPackage> = match legacy_ids {
                Some(_) => [ModelPackage::Boots, ModelPackage::Gloves]
                    .into_iter()
                    .filter(|package| {
                        has_effective_part(
                            &export,
                            &folder,
                            *package,
                            version.engine(),
                            &hand_weighted,
                        )
                    })
                    .collect(),
                None => Vec::new(),
            };
            // A refused old ID is named once on the folder that would have packed
            // it: this one when it has a part of a kind recording it. A folder only
            // linking a kept-shared folder packs nothing of the kind, so the shared
            // folder alone is named.
            let mut reported = BTreeSet::new();
            for (package, of_kind) in [
                (ModelPackage::Boots, &refused.0),
                (ModelPackage::Gloves, &refused.1),
            ] {
                if !own_parts.contains(&package) {
                    continue;
                }
                for old in of_kind {
                    if reported.insert(*old) {
                        messages.push(tool_message(
                            Code::LegacyIdOutOfBlock,
                            Scope::Folder {
                                export_id,
                                path: folder.path.clone(),
                            },
                            Disposition::Keep,
                            vec![
                                ("id", old.to_string()),
                                ("folder", folder.path.as_str().to_owned()),
                                ("block", block_text(legacy_block.as_ref())),
                            ],
                        ));
                    }
                }
            }
            row_players.push(RowPlayer {
                path: folder.path.clone(),
                player_ids: player_ids.clone(),
                player_name: folder.player_name.clone(),
                linked: folder
                    .links
                    .iter()
                    .filter_map(|link| linked_folder(&export, link))
                    .map(|shared| shared.path.clone())
                    .collect(),
                fpc: folder.fpc,
                own_parts,
                settings: player_settings.remove(&folder.path),
                settings_refused: settings_refused.contains(&folder.path),
            });
            if let Some(portrait) = &folder.portrait {
                portraits.extend(
                    player_ids
                        .iter()
                        .map(|player_id| (*player_id, portrait.clone())),
                );
            }
            // A link feeding the player's own package (`link_feeds_own_package`) makes the
            // shared folder's files a second source of the player's folder. One beside a
            // local model of its package combines and is reported, once however many slots
            // map the folder; a referee's plain link is his package as it is, which tells the
            // member nothing new.
            let mut combined = Vec::new();
            for link in folder.links.iter().filter(|link| {
                link_feeds_own_package(&export, version.engine(), &folder, link, &hand_weighted)
            }) {
                let shared = linked_folder(&export, link)
                    .expect("validation drops a player folder whose link names no shared folder");
                if link_combines(&export, &folder, link, version.engine(), &hand_weighted) {
                    messages.push(tool_message(
                        Code::LinkCombined,
                        Scope::Folder {
                            export_id,
                            path: folder.path.clone(),
                        },
                        Disposition::Keep,
                        vec![("link", link_name(link.kind, &link.name))],
                    ));
                }
                combined.push(CombinedFolder {
                    package: package_of(link.kind),
                    folder: shared.clone(),
                });
            }
            // A referee's `fpc_off` says he needs his body, which a refs export takes from the
            // referee template's `refkit`. A team player's body is the kit's. Fox has no refkit
            // yet, so there the marker sets the preset alone.
            let refkit_body = team.is_none()
                && folder.fpc == Some(FpcDirective::Off)
                && match version.engine() {
                    Engine::PreFox => true,
                    Engine::Fox => false,
                };
            let mut model_folder = ModelFolder {
                textures: TextureHome::PlayerCommon {
                    folder_name: folder.path.name().to_owned(),
                },
                common_models: common_models(
                    &folder.path,
                    &folder.files,
                    None,
                    folder.ingame_face.is_some(),
                    &combined,
                    &export.common,
                    version.engine(),
                ),
                common_texture_stems: common_texture_stems.clone(),
                common_files: player_common_files.clone(),
                hand_split: BTreeSet::new(),
                environment_map: false,
                refkit_body,
                path: folder.path,
                files: folder.files,
                ingame_face: folder.ingame_face.is_some(),
                engine: version.engine(),
                combined,
            };
            model_folder.hand_split =
                hand_split_parts(&model_folder, &hand_weighted, version.engine());
            model_folder.environment_map = converts_metal(&model_folder, &metal_models);
            // Without a face folder the game shows the head made in its face editor, which
            // `ingame_face` asks for; every other team player gets one, blank when it holds no
            // face model (the last part of FPC: the body brings its own head, or none). A
            // referee has no FPC body to bring a head, so a referee folder with no face model
            // gets no face folder and the game's referee head stays.
            let blank_face = !model_folder.ingame_face && team.is_some();
            folder_tasks(
                export_id,
                team_id,
                model_folder,
                &packages,
                blank_face,
                &kits,
                &mut tasks,
            );
        }
        for (folder, package, shared_id) in shared {
            folder_tasks(
                export_id,
                team_id,
                folder,
                &[(package, vec![PackageKey::Id(shared_id)])],
                false,
                &kits,
                &mut tasks,
            );
        }
        // After the shared folders' tasks, which a player linking one takes his ID from. A
        // referee has none: the game's referee hook loads slot NN's `k99NN`/`g99NN` by number.
        if team.is_some() {
            let rows = export_rows(
                &tasks,
                first_task,
                &row_players,
                export.coverage,
                hide_ids,
                legacy_ids.as_ref(),
                legacy_block.as_ref(),
            );
            // The same compiled players get the patch's `[[teams]]` block: the run's one
            // patch across exports, whose header `build` fills in and whose provisional
            // IDs `settle` resolves against the committed batches.
            if let Some(team) = patch::patch_team(
                &row_players,
                &rows,
                team_id,
                export.team_name.as_str(),
                export.root.write_names,
                version,
            ) {
                patch_teams.push(patch::PlannedPatchTeam { team, rows });
            }
        }
        // A Common textures task per texture directory of `Common/` a target packs — a
        // texture's is the directory above the first kit-token folder below `Common/`
        // (`texture_directory`), so the `kit1` and `kit2` folders of one set
        // complete in the task holding both — and a Common models task per directory
        // holding a file of its own, models having no folder form: every directory on PES
        // 15-17, each over that directory's own files, its output under the directory's
        // path. On PES 18-21 `Common/` itself as today and the textures in
        // `linked_common_files` alone, a textures task only: Fox has no Common model
        // output, so no models task, and the template environment map is `Common/`'s own.
        // A subfolder file a Fox link reaches that is no texture (a model and what travels
        // with it) is read by the linking folders' own tasks, never by a task of its
        // directory.
        // Each models task's files.
        let read_directories: Vec<(ScopePath, Vec<FileDescriptor>)> =
            common_directories(&export.common)
                .into_iter()
                .map(|(directory, files)| {
                    let model_files = match version.engine() {
                        Engine::Fox => Vec::new(),
                        Engine::PreFox => common_model_files(&files),
                    };
                    (directory, model_files)
                })
                .collect();
        // Whether any directory's models task converts an FMDL holding a metal material: its
        // material names the template environment map at the Common output's own directory,
        // whichever directory it sits in, so `Common/`'s textures task must emit it.
        let converts_metal = read_directories
            .iter()
            .flat_map(|(_, model_files)| model_files)
            .any(|file| {
                file.kind == FileKind::Model(ModelFormat::Fmdl) && metal_models.contains(&file.path)
            });
        // The texture directories keyed case-folded, `Common/` itself first. On PES 18-21 a
        // texture outside `Common/` itself is packed only when a link reaches it
        // (`linked_common_files`: the ones a link names, and the ones a linked model or
        // `.mtl` brings from its directory), a per-file test since the task above a
        // kit-token folder's set holds subfolder textures; the directory's other files are
        // the linking folders' or are read by nothing (`file_not_used`).
        let mut texture_directories: BTreeMap<String, (ScopePath, Vec<FileDescriptor>)> =
            BTreeMap::new();
        for file in export.common.iter().filter(|file| {
            texture_format(file.path.name()).is_some()
                && is_read_common_file(&file.path, version.engine(), &linked_common)
        }) {
            let directory = texture_directory(&file.path);
            texture_directories
                .entry(directory.fold_key())
                .or_insert_with(|| (directory, Vec::new()))
                .1
                .push(file.clone());
        }
        // `Common/` holding no texture of its own has no entry, but still emits the map:
        // an empty entry for it, first, as the texture directories order it.
        if converts_metal {
            let common = common_folder();
            texture_directories
                .entry(common.fold_key())
                .or_insert_with(|| (common, Vec::new()));
        }
        // The export's Common textures' paths below `Common/` without their extensions,
        // which a `Common/` `.mtl` resolves its texture paths in as a model folder's does
        // (`prefox_common`), `Common/` its model folder.
        let common_textures_below: Vec<String> = export
            .common
            .iter()
            .filter(|file| texture_format(file.path.name()).is_some())
            .map(|file| file_stem(below_common(&file.path)).to_owned())
            .collect();
        for (directory, textures) in texture_directories.into_values() {
            // The template environment map goes into the team's Common output, by `Common/`'s
            // own task, for a converted FMDL of any directory holding a metal material, unless
            // `Common/` holds an `env` texture, which the converted material then names. A
            // subfolder's task never emits it.
            let environment_map = directory.segments().count() == 1
                && converts_metal
                && !textures.iter().any(|file| {
                    vtree::fold_name(file_stem(below_common(&file.path))) == ENVIRONMENT_MAP_STEM
                });
            // Planned for the template environment map alone when `Common/` holds no texture.
            if !textures.is_empty() || environment_map {
                tasks.push(task(
                    export_id,
                    team_id,
                    TaskKind::CommonTextures {
                        folder: directory,
                        textures,
                        kits: kits.clone(),
                        environment_map,
                    },
                ));
            }
        }
        for (directory, model_files) in read_directories {
            if !model_files.is_empty() {
                tasks.push(task(
                    export_id,
                    team_id,
                    TaskKind::CommonModels {
                        folder: directory,
                        files: model_files,
                        textures: common_textures_below.clone(),
                    },
                ));
            }
        }
        // Validation keeps a `ref_marker.dds` on a refs export alone. Its task lies in the
        // export's range, so its entries go where the referees' do.
        if let Some(marker) = export.root.referee_marker.take() {
            tasks.push(task(export_id, team_id, TaskKind::RefereeMarker { marker }));
        }
        // The rest is a team's: a refs export's portraits, kits, logo and collars have no
        // referee to go to (validation's `file_not_used`), and the referees have no
        // `UniColor.bin` record for a kits entry to edit.
        let Some(id) = team else {
            continue;
        };
        portraits.extend(
            export
                .portraits
                .into_iter()
                .map(|(slot, file)| (slot.player_id(id), file)),
        );
        // A stable sort keeps the folder's file, pushed first, ahead of the `Portraits/` file
        // of its player id, and `dedup` keeps the first of each id.
        portraits.sort_by_key(|(player_id, _)| *player_id);
        portraits.dedup_by_key(|(player_id, _)| *player_id);
        for (player_id, file) in portraits {
            tasks.push(task(
                export_id,
                team_id,
                TaskKind::Portrait { player_id, file },
            ));
        }
        let collar = export_collar(
            export_id,
            &export.export_display_name,
            &export.collars,
            version.engine(),
            &mut claimed_collars,
            &mut messages,
        );
        let edits = TeamKitEdits {
            fpc,
            collar: collar.as_ref().map(|(_, id)| *id),
        };
        // The kits go by slot, so `slots` is ascending.
        let mut slots = Vec::new();
        let mut configless = Vec::new();
        let midcup = export.coverage == ExportCoverage::Midcup;
        for (slot, kit) in export.kits.kits {
            slots.push(slot);
            let folder = || Scope::Folder {
                export_id,
                path: kit.path.clone(),
            };
            // A `Midcup` kit keeps what it does not resend: without `config.toml` it
            // emits no config of its own (the slot's installed one is edited in
            // place, or the template written where none is installed, `kit_configs`),
            // and without a main texture it is no placeholder (the installed
            // textures stay). The two findings are a `Full` export's.
            if kit.config.is_none() {
                if keeps_installed(&kit, export.coverage) {
                    let has = |stem: &str| kit.textures.iter().any(|t| t.stem == stem);
                    configless.push(ConfiglessKit {
                        slot,
                        path: kit.path.clone(),
                        textures: TexturePresence {
                            kit: true,
                            back: has("kit_back"),
                            chest: has("kit_chest"),
                            leg: has("kit_leg"),
                            name: has("kit_name"),
                        },
                    });
                } else {
                    messages.push(tool_message(
                        Code::KitConfigGenerated,
                        folder(),
                        Disposition::Keep,
                        vec![],
                    ));
                }
            }
            if !midcup && !kit.textures.iter().any(|texture| texture.stem == "kit") {
                messages.push(tool_message(
                    Code::KitPlaceholder,
                    folder(),
                    Disposition::Keep,
                    vec![],
                ));
            }
            tasks.push(task(
                export_id,
                team_id,
                TaskKind::Kit {
                    slot,
                    kit,
                    edits,
                    coverage: export.coverage,
                },
            ));
        }
        team_kits.push(TeamKits {
            export_id,
            team_id,
            coverage: export.coverage,
            edits,
            slots,
            configless,
        });
        // A Midcup over no record whose folders took no ID writes none: an empty one would
        // read to the next Midcup above it as "no shared folders".
        if export.coverage != ExportCoverage::Midcup
            || midcup_record.is_some()
            || !output_record.is_empty()
        {
            shared_ids.push((team_id, output_record));
        }
        if let Some(logo) = export.logo {
            tasks.push(task(export_id, team_id, TaskKind::Logo { logo }));
        }
        if let Some((file, id)) = collar {
            tasks.push(task(export_id, team_id, TaskKind::Collar { file, id }));
        }
    }
    PlanReport {
        manifest: BuildManifest {
            tasks,
            team_colors,
            team_kits,
            patch_teams,
            notes,
            shared_ids,
        },
        messages,
    }
}

/// Pushes `folder`'s tasks onto `tasks`: one `Models` task for each of `packages` any of the
/// folder's sources holds a model of (a face link alone makes the shared face the player's),
/// for a team's face whatever the folder holds when `blank_face` is set, and on Fox for the
/// gloves when the folder has a hand-split part (`ModelFolder::hand_split`), emitted under that
/// package's keys, then, when it planned one of those and the folder has textures or takes the
/// template environment map (`ModelFolder::environment_map`), its `Textures` task, the lot as
/// one `TaskGroup`. The
/// textures task, and a pre-Fox face, complete their variant sets against `kits`, the
/// export's kit numbers.
fn folder_tasks(
    export_id: ExportId,
    team_id: u16,
    folder: ModelFolder,
    packages: &[(ModelPackage, Vec<PackageKey>)],
    blank_face: bool,
    kits: &[u8],
    tasks: &mut Vec<BuildTask>,
) {
    let first = tasks.len();
    for (package, ids) in packages {
        // A pre-Fox model is a part of the package its source feeds, whatever its `face.xml`
        // type: a player's face, a shared folder's boots or gloves, the boots or gloves of an
        // `ingame_face` player combining a folder of their kind. His own parts say their
        // package.
        let models = folder_files(&folder, |source, _, _, role| {
            is_package_model(role, source, *package)
        });
        let blank = blank_face && *package == ModelPackage::Face;
        let hands = match folder.engine {
            // A hand-split face part gives the folder gloves, whatever its files are named.
            Engine::Fox => *package == ModelPackage::Gloves && !folder.hand_split.is_empty(),
            // The face task packs a split model's hands as entries of its own `face.xml`.
            Engine::PreFox => false,
        };
        // A package no slot of the folder gives a key to is not planned at all: under
        // `legacy_ids.toml` a slot records no old ID of the kind, or an earlier slot took
        // his, and nothing is emitted for it.
        if ids.is_empty() || (models.is_empty() && !blank && !hands) {
            continue;
        }
        tasks.push(task(
            export_id,
            team_id,
            TaskKind::Models {
                folder: folder.clone(),
                package: *package,
                ids: ids.clone(),
                kits: kits.to_vec(),
            },
        ));
    }
    // A folder planning no package plans no textures task: the writer commits a folder's
    // textures after a package of it, and nothing would name them (validation reports each
    // `file_not_used`, `plans_a_package`).
    if tasks.len() == first {
        return;
    }
    // The template environment map is one of the folder's textures, so a folder holding no
    // texture of its own still gets a textures task for it. One emitting nothing is not
    // planned: the writer takes an empty textures batch for a failed one and drops the folder.
    if !folder.takes_template_environment_map()
        && folder_files(&folder, |_, _, _, role| {
            matches!(role, PlayerFile::Texture { .. })
        })
        .is_empty()
    {
        return;
    }
    tasks.push(task(
        export_id,
        team_id,
        TaskKind::Textures {
            folder,
            kits: kits.to_vec(),
        },
    ));
    let members = &mut tasks[first..];
    let charge = members
        .iter()
        .fold(0usize, |sum, task| sum.saturating_add(task.charge));
    let group = TaskGroup {
        tasks: first..first + members.len(),
        charge,
    };
    for task in members {
        task.group = Some(group.clone());
    }
}

/// Removes from every kit's effective textures each one a target of `engine` does not emit
/// (`emits_kit_texture`), in the set's order: the other engine's map (its `kit_srm` on PES
/// 15-17, its `kit_mask` on PES 18-21; neither is converted into the other) and a `kit_*` stem
/// outside the seven the compiler builds (`KIT_TEXTURE_STEMS`, so `kit_spec`), each reported
/// as `kit_texture_not_used` on its kit folder, naming the file. It goes before the kit's task
/// is made, which therefore never reads them.
fn drop_unused_kit_textures(
    engine: Engine,
    export_id: ExportId,
    kits: &mut KitsFolder,
    messages: &mut Vec<Message>,
) {
    for kit in kits.kits.values_mut() {
        let (dropped, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut kit.textures)
            .into_iter()
            .partition(|texture| !emits_kit_texture(engine, &texture.stem));
        kit.textures = kept;
        for texture in dropped {
            messages.push(tool_message(
                Code::KitTextureNotUsed,
                Scope::Folder {
                    export_id,
                    path: kit.path.clone(),
                },
                Disposition::DropFile,
                vec![("file", texture.file.path.name().to_owned())],
            ));
        }
    }
}

/// Drops every mapped player folder of `export` holding a glTF model selected for its stem on
/// `version` (`PlayerFile::UnsupportedGltf`), and every shared folder holding one with each
/// mapped player folder linking it. The compiler does not read glTF yet, and a selected glTF
/// drops its folder rather than falling through to the other engine's format, so the same
/// export never compiles differently once glTF is read. A player without the face, boots or
/// gloves he linked would compile to something he did not ask for. A mapped player folder
/// whose `.common` model link names a Common model a Common glTF of its stem beats
/// (`selected_common_gltfs`) is dropped too: the link would load the glTF, and the model it
/// names is not loaded in the glTF's place. `model_gltf_unsupported`
/// is reported on the player folder once per such file: his own, named below his folder, or,
/// when he holds none, those of the shared folders he links and the Common glTFs his links
/// select, named by their export paths; each in the export's folder order. The player folder's
/// roster slots are removed, as validation removes a dropped folder's, so it plans no task and
/// its slots compile as empty ones do, and the shared folder is removed from the export, as is
/// every shared folder no remaining mapped player links, with no finding of its own (the
/// drop's names the cause, and such a folder would get no id and no task). It goes before any
/// of the export's tasks is made, so no task meets a selected glTF.
fn drop_gltf_folders(
    export_id: ExportId,
    export: &mut ResolvedAestheticsExport,
    version: PesVersion,
    messages: &mut Vec<Message>,
) {
    let export = &mut export.export;
    let engine = version.engine();
    let mapped: Vec<PlayerIndex> = match &export.roster {
        ValidatedRoster::Team(slots) => slots.values().copied().collect(),
        ValidatedRoster::Referees(slots) => slots.values().copied().collect(),
    };
    // Each selected glTF of a shared folder, with that folder's path, faces first, then boots,
    // then gloves, as the export lists them.
    let mut shared_gltfs: Vec<(ScopePath, ScopePath)> = Vec::new();
    for (kind, folder) in shared_folders(export) {
        let models = FolderModels::of_shared(&folder.path, &folder.files, kind, engine);
        for file in &folder.files {
            if player_file(&folder.path, file, &models) == Some(PlayerFile::UnsupportedGltf) {
                shared_gltfs.push((folder.path.clone(), file.path.clone()));
            }
        }
    }
    let linked_common = linked_common_files(export);
    let common_gltfs = selected_common_gltfs(&export.common, engine, &linked_common);
    let mut dropped = Vec::new();
    for (index, folder) in export.players.iter().enumerate() {
        let index = PlayerIndex(index);
        if !mapped.contains(&index) {
            continue;
        }
        let models = FolderModels::of_player_files(
            &folder.path,
            &folder.files,
            folder.ingame_face.is_some(),
            engine,
        );
        // Named below the folder, as `xml_ignored_fox` names its file: two glTFs of one name
        // in different subfolders are two findings a member can tell apart.
        let mut files: Vec<String> = folder
            .files
            .iter()
            .filter(|file| {
                player_file(&folder.path, file, &models) == Some(PlayerFile::UnsupportedGltf)
            })
            .map(|file| crate::deep::relative(&file.path, &folder.path))
            .collect();
        // A folder dropped for its own glTF is not told about a linked one as well.
        if files.is_empty() {
            let linked: Vec<(SharedKind, &SharedModelFolder)> = folder
                .links
                .iter()
                .filter_map(|link| Some((link.kind, linked_folder(export, link)?)))
                .collect();
            files = shared_gltfs
                .iter()
                .filter(|(shared, _)| linked.iter().any(|(_, folder)| &folder.path == shared))
                .map(|(_, file)| file.as_str().to_owned())
                .collect();
            // A `.common` model link whose stem selects a Common glTF would load that glTF,
            // which beats the model the link names, so it has nothing to load in its place.
            // A shared folder's link is a player folder's: the glTF is the linking player's
            // problem the same way.
            for file in &folder.files {
                let Some(gltf) = linked_common_gltf(&folder.path, file, &models, &common_gltfs)
                else {
                    continue;
                };
                if !files.contains(&gltf) {
                    files.push(gltf);
                }
            }
            for (kind, shared) in &linked {
                let shared_models =
                    FolderModels::of_shared(&shared.path, &shared.files, *kind, engine);
                for file in &shared.files {
                    let Some(gltf) =
                        linked_common_gltf(&shared.path, file, &shared_models, &common_gltfs)
                    else {
                        continue;
                    };
                    if !files.contains(&gltf) {
                        files.push(gltf);
                    }
                }
            }
        }
        if !files.is_empty() {
            dropped.push(index);
        }
        for file in files {
            messages.push(tool_message(
                Code::ModelGltfUnsupported,
                Scope::Folder {
                    export_id,
                    path: folder.path.clone(),
                },
                Disposition::DropFolder,
                vec![("file", file)],
            ));
        }
    }
    let holds_gltf = |folder: &SharedModelFolder| {
        shared_gltfs
            .iter()
            .any(|(shared, _)| *shared == folder.path)
    };
    export.faces.retain(|folder| !holds_gltf(folder));
    export.boots.retain(|folder| !holds_gltf(folder));
    export.gloves.retain(|folder| !holds_gltf(folder));
    match &mut export.roster {
        ValidatedRoster::Team(slots) => slots.retain(|_, index| !dropped.contains(index)),
        ValidatedRoster::Referees(slots) => slots.retain(|_, index| !dropped.contains(index)),
    }
    // A shared folder only dropped players linked would get no id and no task; validation
    // drops such orphans before planning, so outside a drop this removes nothing.
    let linked: Vec<ScopePath> = mapped_players(export)
        .iter()
        .flat_map(|folder| &folder.links)
        .filter_map(|link| linked_folder(export, link))
        .map(|shared| shared.path.clone())
        .collect();
    export.faces.retain(|folder| linked.contains(&folder.path));
    export.boots.retain(|folder| linked.contains(&folder.path));
    export.gloves.retain(|folder| linked.contains(&folder.path));
}

/// The glTFs among `common`, the export's `Common/` files, that a target of `engine` reads
/// (`is_read_common_file`, `linked` the subfolder files a link reaches) and selects for their
/// stem: those with no model of the target's own format of their stem beside them in their
/// directory. Selection is target-native first, then glTF, then the other engine's format, the
/// one rule for every model folder, each Common directory included. The compiler does not read
/// glTF yet, so planning drops each (`drop_common_gltfs`).
fn selected_common_gltfs<'a>(
    common: &'a [FileDescriptor],
    engine: Engine,
    linked: &BTreeSet<ScopePath>,
) -> Vec<&'a FileDescriptor> {
    let native = FileKind::Model(native_format(engine));
    let native_stems: BTreeSet<(Option<String>, String)> = common
        .iter()
        .filter(|file| file.kind == native)
        .map(directory_stem)
        .collect();
    common
        .iter()
        .filter(|file| {
            is_read_common_file(&file.path, engine, linked)
                && file.kind == FileKind::Model(ModelFormat::Gltf)
                && !native_stems.contains(&directory_stem(file))
        })
        .collect()
}

/// The export path of the Common glTF that `file`, a file of the player folder at `folder`
/// whose models are `models`, would load: `file` is a `.common` link to a model with a role,
/// and `common_gltfs` (`selected_common_gltfs`) holds a glTF of the linked stem, which beats
/// the model the link names. `None` for any other file.
fn linked_common_gltf(
    folder: &ScopePath,
    file: &FileDescriptor,
    models: &FolderModels,
    common_gltfs: &[&FileDescriptor],
) -> Option<String> {
    if file.kind != FileKind::CommonLink
        || !matches!(
            player_file(folder, file, models)?,
            PlayerFile::CommonModel { .. }
                | PlayerFile::PreFoxCommonModel { .. }
                | PlayerFile::PreFoxPart { .. }
        )
    {
        return None;
    }
    // The glTF beside the linked model, of its path stem (`jessie/body` for
    // `jessie/body.fmdl.common`).
    let linked = vtree::fold_name(file_stem(&common_link_target(&file.path, folder)?));
    common_gltfs
        .iter()
        .find(|gltf| vtree::fold_name(file_stem(below_common(&gltf.path))) == linked)
        .map(|gltf| gltf.path.as_str().to_owned())
}

/// Removes from `common`, the export's `Common/` files, each glTF a target of `engine` selects
/// for its stem in its directory (`selected_common_gltfs`), reporting
/// `model_gltf_unsupported` on the file, named by its export path. The file alone is dropped,
/// as a collar's is, and the rest of the export compiles without it. A model of the other
/// engine's format of its stem in its directory is beaten by it and removed too, with no
/// finding of its own, as a player folder's beaten model has none. It is removed rather than
/// left in place, so that neither the pre-Fox Common models task nor a link resolves to it in
/// the glTF's place (a mapped player folder linking it was dropped first, `drop_gltf_folders`).
/// Its `.skl` stays, read by nothing: only a converted FMDL pairs one. A glTF a target-native
/// model of its stem beats is left in place, read by nothing, with no finding.
fn drop_common_gltfs(
    engine: Engine,
    export_id: ExportId,
    common: &mut Vec<FileDescriptor>,
    linked: &BTreeSet<ScopePath>,
    messages: &mut Vec<Message>,
) {
    let selected = selected_common_gltfs(common, engine, linked);
    let dropped_stems: BTreeSet<(Option<String>, String)> =
        selected.iter().map(|file| directory_stem(file)).collect();
    let dropped: Vec<ScopePath> = selected.into_iter().map(|file| file.path.clone()).collect();
    for path in &dropped {
        messages.push(tool_message(
            Code::ModelGltfUnsupported,
            Scope::File {
                export_id,
                path: path.clone(),
            },
            Disposition::DropFile,
            vec![("file", path.as_str().to_owned())],
        ));
    }
    let other_engine_format = FileKind::Model(other_format(engine));
    common.retain(|file| {
        let beaten =
            file.kind == other_engine_format && dropped_stems.contains(&directory_stem(file));
        !beaten && !dropped.contains(&file.path)
    });
}

/// `kit_variant_model_left_out` for each set of per-kit model files (`pants_kit1.fmdl`,
/// `pants_kit2.model`) where no `face.xml` names the set, in a folder of `export` that is
/// compiled, on that folder: a mapped player folder, or a shared folder a mapped player links
/// (validation drops one no mapped player links, and `drop_gltf_folders` one only the players
/// it dropped linked). Which folders those are is `roles::leaves_out_kit_variants`: on Fox
/// every one, Fox having no model-path indirection; on pre-Fox a mapped player folder holding
/// `ingame_face` and a shared boots or gloves folder, since the shared boots writer merges every
/// model into one `boots.model`, the gloves writer lists every glove in `glove.xml` and a
/// marked player's parts are all worn. A pre-Fox face, a player's or a shared one, packs the
/// whole set and lists it once as `pants_kitN` for the game to respell
/// (`spec/model_format.md` "Kit-dependent assets"). Where no xml names it, the fallback (the
/// `kitF` file, else the lowest variant) is what both engines agree on.
///
/// A mapped player's sets span his own files and those of the shared folders whose link feeds
/// his own package (`link_feeds_own_package`), as his packages are built
/// (`ModelFolder::roles`). `hand_weighted` is the models the deep pass found carrying hand
/// weights. A set is reported on the player when it holds a file of his own, a set split
/// between him and such a folder included. A shared folder reports the sets of its own files,
/// once however many players combine it.
fn kit_variant_model_messages(
    export_id: ExportId,
    export: &ValidatedAestheticsExport,
    engine: Engine,
    hand_weighted: &BTreeSet<ScopePath>,
    messages: &mut Vec<Message>,
) {
    let mut folders: Vec<(&ScopePath, Vec<Vec<&FileDescriptor>>)> = Vec::new();
    for player in mapped_players(export) {
        if !leaves_out_kit_variants(engine, player.ingame_face.is_some(), None) {
            continue;
        }
        let combined = combined_folders(export, player, engine, hand_weighted)
            .into_iter()
            .map(|(_, shared)| role_files(&shared.path, &shared.files));
        let sources = iter::once(role_files(&player.path, &player.files))
            .chain(combined)
            .collect();
        folders.push((&player.path, sources));
    }
    for (kind, shared) in shared_folders(export) {
        if leaves_out_kit_variants(engine, false, Some(kind)) {
            folders.push((&shared.path, vec![role_files(&shared.path, &shared.files)]));
        }
    }
    for (path, sources) in folders {
        let own = &sources[0];
        for set in model_variant_sets(&sources, engine) {
            let holds_own = own
                .iter()
                .any(|file| file.path == set.used || set.left_out.contains(&file.path));
            if !holds_own {
                continue;
            }
            messages.push(tool_message(
                Code::KitVariantModelLeftOut,
                Scope::Folder {
                    export_id,
                    path: path.clone(),
                },
                Disposition::Keep,
                vec![
                    ("model", set.reference),
                    ("used", set.used.name().to_owned()),
                ],
            ));
        }
    }
}

/// A roster-mapped player folder as planning builds it: the keys each of its packages is
/// emitted under, one per slot mapping it in slot order, those slots' player ids, which
/// its portrait and its rows go by (none for a referee, who has no player id), and the
/// recorded old IDs the block check refuses (`legacy_id_out_of_block`).
struct MappedFolder {
    /// The player folder.
    folder: PlayerFolder,
    /// The face, boots and gloves packages, each with its keys.
    packages: [(ModelPackage, Vec<PackageKey>); 3],
    /// The player id of each slot mapping the folder, in slot order.
    player_ids: Vec<u32>,
    /// The recorded old IDs the block check refuses, the boots' then the gloves'
    /// (`boots_gloves_key`): each is reported once on the folder packing its kind
    /// (`legacy_id_out_of_block`). Always empty for a referee, who records none.
    refused: (BTreeSet<u32>, BTreeSet<u32>),
}

/// The roster-mapped folders of `players`, by `player_folders`, as `MappedFolder`s: of a team
/// export, `team` given, a slot's face keyed by its player id and its boots and gloves by
/// its exclusive id of the team's block, or under a `legacy_ids.toml` by the old ID the
/// file records for him, each old ID packed once by the first slot in roster order
/// recording it, a slot the file gives no ID of a kind getting no key of it, and one
/// recording an ID outside `legacy_block` getting none either, each such ID collected by
/// kind for the caller to report once on the folder that would have packed it
/// (`legacy_id_out_of_block`); of a refs export, every package of referee slot NN keyed
/// by the slot (`referee0NN`, `k99NN`, `g99NN`).
fn mapped_folders(
    players: Vec<PlayerFolder>,
    roster: &ValidatedRoster,
    team: Option<TeamId>,
    legacy_ids: Option<&LegacyIds>,
    legacy_block: Option<&RangeInclusive<u32>>,
) -> Vec<MappedFolder> {
    match (roster, team) {
        (ValidatedRoster::Team(slots), Some(id)) => {
            let model_ids = PlannedModelIds::for_team(id);
            // The old IDs the export's earlier slots already packed, boots and gloves
            // separately: an old ID another slot recorded first gets no key.
            let mut taken = (BTreeSet::new(), BTreeSet::new());
            player_folders(players, slots)
                .into_iter()
                .map(|(folder, slots)| {
                    // The folder's recorded old IDs the block check refuses, boots
                    // and gloves separately: an ID of either is reported once,
                    // however many slots record it.
                    let mut refused = (BTreeSet::new(), BTreeSet::new());
                    MappedFolder {
                        packages: ModelPackage::ALL.map(|package| {
                            let keys = slots
                                .iter()
                                .filter_map(|slot| match package {
                                    ModelPackage::Face => Some(PackageKey::Id(slot.player_id(id))),
                                    ModelPackage::Boots => boots_gloves_key(
                                        legacy_ids,
                                        legacy_block,
                                        &mut taken.0,
                                        &mut refused.0,
                                        *slot,
                                        ItemTable::Boots,
                                        model_ids.exclusive(*slot),
                                    ),
                                    ModelPackage::Gloves => boots_gloves_key(
                                        legacy_ids,
                                        legacy_block,
                                        &mut taken.1,
                                        &mut refused.1,
                                        *slot,
                                        ItemTable::Gloves,
                                        model_ids.exclusive(*slot),
                                    ),
                                })
                                .collect();
                            (package, keys)
                        }),
                        player_ids: slots.iter().map(|slot| slot.player_id(id)).collect(),
                        folder,
                        refused,
                    }
                })
                .collect()
        }
        (ValidatedRoster::Referees(slots), None) => player_folders(players, slots)
            .into_iter()
            .map(|(folder, slots)| MappedFolder {
                packages: ModelPackage::ALL.map(|package| {
                    (
                        package,
                        slots.iter().copied().map(PackageKey::Referee).collect(),
                    )
                }),
                player_ids: Vec::new(),
                folder,
                refused: (BTreeSet::new(), BTreeSet::new()),
            })
            .collect(),
        (ValidatedRoster::Team(_), None) | (ValidatedRoster::Referees(_), Some(_)) => {
            unreachable!("validation and identity both read a refs export from its `/refs/` name")
        }
    }
}

/// The key a team player's boots or gloves `package` is emitted under for `slot`: without
/// `legacy_ids` his exclusive ID of the team's block, with it the old ID the file records
/// for him when `block` holds it and `taken` does not already (each old ID packed once
/// per team, by the first slot in roster order recording it); `None` for a slot the file
/// gives no ID of the kind, whose old ID an earlier slot took, or whose old ID the block
/// check refuses, the refused one collected into `refused` for the caller to report once
/// on the folder packing the kind (`legacy_id_out_of_block`).
fn boots_gloves_key(
    legacy_ids: Option<&LegacyIds>,
    block: Option<&RangeInclusive<u32>>,
    taken: &mut BTreeSet<u32>,
    refused: &mut BTreeSet<u32>,
    slot: PlayerSlot,
    table: ItemTable,
    exclusive: u16,
) -> Option<PackageKey> {
    match legacy_ids {
        None => Some(PackageKey::Id(u32::from(exclusive))),
        Some(ids) => match ids.old_id(slot, table) {
            // A recorded old ID outside the team's VGL27 block is not packed: under it
            // the model would land on another team's own ID.
            Some(id) if !in_block(block, id) => {
                refused.insert(id);
                None
            }
            Some(id) => taken.insert(id).then_some(PackageKey::Id(id)),
            None => None,
        },
    }
}

/// Every player folder `slots` maps, with the slots mapping it, in slot order, the folders
/// ordered by their first slot. A folder no slot maps is not compiled.
fn player_folders<Slot: Copy>(
    players: Vec<PlayerFolder>,
    slots: &BTreeMap<Slot, PlayerIndex>,
) -> Vec<(PlayerFolder, Vec<Slot>)> {
    let mut mapped: Vec<(PlayerIndex, Vec<Slot>)> = Vec::new();
    for (slot, index) in slots {
        match mapped.iter_mut().find(|(known, _)| known == index) {
            Some((_, folder_slots)) => folder_slots.push(*slot),
            None => mapped.push((*index, vec![*slot])),
        }
    }
    let mut players: Vec<Option<PlayerFolder>> = players.into_iter().map(Some).collect();
    mapped
        .into_iter()
        .filter_map(|(index, folder_slots)| {
            let folder = players.get_mut(index.0)?.take()?;
            Some((folder, folder_slots))
        })
        .collect()
}

/// The files of `Common/` grouped by the directory holding them (`Common/` itself, or a
/// subfolder at any depth), in path order, `Common/` itself first, each directory's files in
/// `common`'s order; a directory holding no file has no entry. On PES 15-17 each is a Common
/// folder of its own, planned and packed at its own path.
fn common_directories(common: &[FileDescriptor]) -> Vec<(ScopePath, Vec<FileDescriptor>)> {
    // Keyed case-folded, as the game's file system folds paths.
    let mut directories: BTreeMap<String, (ScopePath, Vec<FileDescriptor>)> = BTreeMap::new();
    for file in common {
        let directory = file
            .path
            .parent()
            .expect("a Common file sits in the export's Common/ folder or below it");
        directories
            .entry(directory.fold_key())
            .or_insert_with(|| (directory, Vec::new()))
            .1
            .push(file.clone());
    }
    directories.into_values().collect()
}

/// The `Common/` directory whose textures task covers the `Common/` file at `path`: its own
/// directory, unless a directory segment below `Common/` is a kit-token folder
/// (`kit_folder`). Then it is the directory above the first of them, so the `kit1` and `kit2`
/// folders of one set complete in the one task holding both.
fn texture_directory(path: &ScopePath) -> ScopePath {
    let mut directory = path
        .parent()
        .expect("a Common file sits in the export's Common/ folder or below it");
    let keep = directory
        .segments()
        .skip(1)
        .position(|segment| kit_folder(segment).is_some())
        .map_or_else(|| directory.segments().count(), |at| at + 1);
    for _ in keep..directory.segments().count() {
        directory = directory.parent().expect("`keep` is a segment count");
    }
    directory
}

/// The files among `common`, the files of one directory of `Common/` (`common_directories`),
/// that the pre-Fox Common models task reads (`TaskKind::CommonModels`), in `common`'s order:
/// every `.model` and `.mtl`, every FMDL no `.model` of its stem beats
/// (`is_selected_common_model`), which the task converts, and the `.skl` of each such FMDL's
/// stem, its bind pose. A beaten FMDL and a `.skl` no converted FMDL pairs are ignored, as a
/// player folder's are.
fn common_model_files(common: &[FileDescriptor]) -> Vec<FileDescriptor> {
    let converted = |file: &FileDescriptor| is_selected_common_model(common, file, Engine::PreFox);
    // The stems, folded, of the FMDLs converted: a `.skl` of one of them is its bind pose.
    let converted_stems: BTreeSet<String> = common
        .iter()
        .filter(|file| file.kind == FileKind::Model(ModelFormat::Fmdl) && converted(file))
        .map(|file| vtree::fold_name(file_stem(file.path.name())))
        .collect();
    common
        .iter()
        .filter(|file| match file.kind {
            FileKind::Model(ModelFormat::PesModel) | FileKind::Mtl => true,
            FileKind::Model(ModelFormat::Fmdl) => converted(file),
            FileKind::Skl => {
                converted_stems.contains(&vtree::fold_name(file_stem(file.path.name())))
            }
            FileKind::Model(ModelFormat::Gltf)
            | FileKind::Texture
            | FileKind::Fclo
            | FileKind::Xml
            | FileKind::MaterialsToml
            | FileKind::Bin
            | FileKind::SharedLink(_)
            | FileKind::CommonLink
            | FileKind::Marker(_)
            | FileKind::Metadata(_)
            | FileKind::Other => false,
        })
        .cloned()
        .collect()
}

/// The `.common` model links of the model folder at `folder` (a player folder's or, of kind
/// `own_kind`, a shared output's, holding `files` and `ingame_face` when a player folder's
/// holds the marker) and of each of its `combined` shared folders, resolved against
/// `common`, the export's `Common/` files, exactly as validation resolved them (the file at
/// the link's own path below its source folder, `common_link_target`, matched by
/// case-folded path), each with the Common model it loads on a target of `engine`
/// (`selected_common_model`: a model of the linked stem in the target's own format beats the
/// other one). On Fox each link to an FMDL or a `.model` (`PlayerFile::CommonModel`), with,
/// when the link's role has a skeleton slot, the Common `.skl` of the model's stem; on
/// pre-Fox each link that is a part of an `ingame_face` player's boots or gloves
/// (`PlayerFile::PreFoxPart`), an FMDL with the Common `.skl` of its stem, its bind pose, and
/// a `.model` with none, carrying its own; a shared folder's links take those roles on Fox
/// alone. On both engines a `.model` is given the `.mtl` its
/// search finds (`mtl_for`, run in its own source, whose folder it is searched against),
/// which a pre-Fox package writes and a Fox one converts it with,
/// and an FMDL none, its materials travelling inside it on Fox and its conversion writing its
/// material set on pre-Fox. A pre-Fox link without the marker stays a link the face's
/// `face.xml` names (`PlayerFile::PreFoxCommonModel`) and is not resolved here. Validation
/// drops a folder whose link names no Common file, so every link here resolves. The sources
/// are read as `ModelFolder::roles` reads them (`part_source_models`): the folder's own
/// first, then each combined folder's, so the player's own link is the one `common_material`
/// finds for a model two sources link.
fn common_models(
    folder: &ScopePath,
    files: &[FileDescriptor],
    own_kind: Option<SharedKind>,
    ingame_face: bool,
    combined: &[CombinedFolder],
    common: &[FileDescriptor],
    engine: Engine,
) -> Vec<CommonModel> {
    let combined: Vec<(SharedKind, &SharedModelFolder)> = combined
        .iter()
        .map(|shared| (shared_kind_of(shared.package), &shared.folder))
        .collect();
    let (own, combined_models) =
        part_source_models(folder, files, own_kind, ingame_face, &combined, engine);
    let sources = std::iter::once((folder, files, own)).chain(
        combined
            .iter()
            .zip(combined_models)
            .map(|((_, shared), models)| (&shared.path, shared.files.as_slice(), models)),
    );
    sources
        .flat_map(|(source, files, models)| {
            files.iter().filter_map(move |file| {
                let slot = match player_file(source, file, &models)? {
                    PlayerFile::CommonModel { package, name } => skeleton_slot(package, name),
                    // Only a marked folder gives a link a part's role. Its skeleton has no
                    // slot: only a converted FMDL reads one, its bind pose (below).
                    PlayerFile::PreFoxPart { .. } if file.kind == FileKind::CommonLink => None,
                    PlayerFile::Model { .. }
                    | PlayerFile::PreFoxPart { .. }
                    | PlayerFile::Packed { .. }
                    | PlayerFile::FaceDiffXml
                    | PlayerFile::UnusedFaceFile
                    | PlayerFile::Skeleton { .. }
                    | PlayerFile::SlotlessSkeleton
                    | PlayerFile::LeftOutKitVariant
                    | PlayerFile::Texture { .. }
                    | PlayerFile::CommonTexture(_)
                    | PlayerFile::PreFoxModel { .. }
                    | PlayerFile::Material
                    | PlayerFile::PreFoxCommonModel { .. }
                    | PlayerFile::CommonMaterial
                    | PlayerFile::FaceXml
                    | PlayerFile::ConversionSkeleton
                    | PlayerFile::UnsupportedGltf => return None,
                };
                let linked = common_link_target(&file.path, source)
                    .expect("a model link's role implies a `.common` link below its folder");
                let model = selected_common_model(common, &linked, engine)
                    .expect("validation drops a folder whose link names no Common file");
                let fmdl = model.kind == FileKind::Model(ModelFormat::Fmdl);
                let skeleton = match engine {
                    Engine::Fox => slot.and_then(|_| common_skeleton(common, &linked)),
                    Engine::PreFox if fmdl => common_skeleton(common, &linked),
                    Engine::PreFox => None,
                };
                let material = (!fmdl).then(|| {
                    mtl_for(&file.path, source, files, common).expect(
                        "the deep pass drops a folder holding a link to a `.model` no `.mtl` \
                         is found for (`model_material_undefined`)",
                    )
                });
                Some(CommonModel {
                    link: file.path.clone(),
                    model: model.clone(),
                    skeleton: skeleton.cloned(),
                    material: material.cloned(),
                })
            })
        })
        .collect()
}

/// The task compiling `kind`, charged the bytes of the files it reads. A sum past `usize` (a
/// 32-bit host only) is over any memory cap, and so is the saturated value.
fn task(export_id: ExportId, team_id: u16, kind: TaskKind) -> BuildTask {
    let size: u64 = kind.files().iter().map(|file| file.size).sum();
    BuildTask {
        export_id,
        team_id,
        charge: usize::try_from(size).unwrap_or(usize::MAX),
        kind,
        group: None,
    }
}

#[cfg(test)]
mod tests {
    use studio_core::Severity;

    use super::*;
    use crate::testing::{resolved, resolved_with_issues, to_plan, two_team_colors};
    use item_rows::{ItemRow, ItemTable};

    /// `keys` as a list (`[71405, 71407]`, `[referee 1, referee 20]`).
    fn keys_text(keys: &[PackageKey]) -> String {
        let keys: Vec<String> = keys
            .iter()
            .map(|key| match key {
                PackageKey::Id(id) => id.to_string(),
                PackageKey::Referee(slot) => format!("referee {}", slot.get()),
            })
            .collect();
        format!("[{}]", keys.join(", "))
    }

    /// Each task as one line: export, team, what it compiles, charge.
    fn summary(report: &PlanReport) -> Vec<String> {
        report
            .manifest
            .tasks
            .iter()
            .map(|task| {
                let what = match &task.kind {
                    TaskKind::Models {
                        folder,
                        package,
                        ids,
                        ..
                    } => format!("{package:?} {} {}", folder.path.as_str(), keys_text(ids)),
                    TaskKind::Textures { folder, .. } => {
                        format!("textures {}", folder.path.as_str())
                    }
                    TaskKind::CommonTextures {
                        folder, textures, ..
                    } => {
                        format!("common textures {} ({})", folder.as_str(), textures.len())
                    }
                    TaskKind::CommonModels { folder, files, .. } => {
                        format!("common models {} ({})", folder.as_str(), files.len())
                    }
                    TaskKind::Portrait { player_id, file } => {
                        format!("portrait {player_id} {}", file.path.as_str())
                    }
                    TaskKind::Kit { slot, kit, .. } => {
                        format!("kit {} {}", slot.as_str(), kit.path.as_str())
                    }
                    TaskKind::Logo { logo } => format!("logo {}", logo.main.file.path.as_str()),
                    TaskKind::RefereeMarker { marker } => {
                        format!("referee marker {}", marker.path.as_str())
                    }
                    TaskKind::Collar { file, id } => {
                        format!("collar {id} {}", file.path.as_str())
                    }
                };
                format!(
                    "{} {} {what} charge {}",
                    task.export_id.0, task.team_id, task.charge
                )
            })
            .collect()
    }

    #[test]
    fn tasks_go_by_export_then_faces_by_first_slot_then_kits_by_slot() {
        let first = resolved(
            "dbg Midcup Two",
            &[
                ("Players/Zed/face_high.fmdl", 10),
                ("Players/Zed/face_diff.bin", 0),
                ("Players/Zed/hair.dds", 5),
                ("Players/Amy/face_high.fmdl", 20),
                ("Players/Amy/face_diff.bin", 0),
                ("Kits/g1/kit.dds", 7),
                ("Kits/g1/config.toml", 3),
                ("Kits/p2 - Away/kit.dds", 8),
            ],
            &["Kits/p1"],
            // Amy is listed first in the file but holds the later slot.
            Some(b"07 Amy\n03 Zed\n09 Amy\n"),
        );
        let second = resolved(
            "co Midcup One",
            &[
                ("Players/04 - B/face_high.fmdl", 1),
                ("Players/04 - B/face_diff.bin", 0),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![
                to_plan(ExportId(0), first, two_team_colors(), None),
                to_plan(ExportId(1), second, two_team_colors(), None),
            ],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        assert_eq!(
            summary(&report),
            [
                "0 790 Face Players/Zed [79003] charge 10",
                "0 790 textures Players/Zed charge 5",
                "0 790 Face Players/Amy [79007, 79009] charge 20",
                "0 790 kit p1 Kits/p1 charge 0",
                "0 790 kit p2 Kits/p2 - Away charge 8",
                "0 790 kit g1 Kits/g1 charge 10",
                "1 714 Face Players/04 - B [71404] charge 1",
            ]
        );
        // Zed's two tasks are one group charged as one; Amy, with no textures, is ungrouped.
        let groups: Vec<Option<TaskGroup>> = report
            .manifest
            .tasks
            .iter()
            .map(|task| task.group.clone())
            .collect();
        let zed = Some(TaskGroup {
            tasks: 0..2,
            charge: 15,
        });
        assert_eq!(groups, [zed.clone(), zed, None, None, None, None, None]);
    }

    #[test]
    fn an_export_s_logo_is_one_task_after_its_kits_reading_both_files() {
        let export = resolved(
            "co Midcup Logo",
            &[
                ("logo_small_crop.dds", 9),
                ("Players/04 - B/face_high.fmdl", 1),
                ("Players/04 - B/face_diff.bin", 0),
                ("Kits/p1/kit.dds", 8),
                ("logo.png", 40),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/04 - B [71404] charge 1",
                "0 714 kit p1 Kits/p1 charge 8",
                "0 714 logo logo.png charge 49",
            ]
        );
        let logo = &report.manifest.tasks[2];
        assert_eq!(logo.group, None);
        assert_eq!(logo.kind.folder_path(), scope_path("logo.png"));
        let files: Vec<&str> = logo
            .kind
            .files()
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        assert_eq!(files, ["logo.png", "logo_small_crop.dds"]);
    }

    #[test]
    fn a_folder_s_packages_go_face_boots_gloves_then_its_textures_under_the_planned_ids() {
        let export = resolved(
            "co Midcup Models",
            &[
                ("Players/A/glove_l.fmdl", 4),
                ("Players/A/kit_boots.skl", 2),
                ("Players/A/kit_boots.fmdl", 8),
                ("Players/A/shirt.dds", 16),
                ("Players/A/face_high.fmdl", 32),
                ("Players/A/face_high.skl", 128),
                ("Players/A/face_diff.bin", 1),
                ("Players/23 - B/boots.fmdl", 64),
            ],
            &[],
            Some(b"05 A\n07 A\n23 23 - B\n"),
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        // Slots 05 and 07 of team 714 own the exclusive ids 625 and 627; slot 23 owns 643.
        // `face_high.skl` has no slot: no task reads it, so no charge counts it. Slot 23
        // holds no face model: its face is the blank one, reading nothing.
        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/A [71405, 71407] charge 33",
                "0 714 Boots Players/A [625, 627] charge 10",
                "0 714 Gloves Players/A [625, 627] charge 4",
                "0 714 textures Players/A charge 16",
                "0 714 Face Players/23 - B [71423] charge 0",
                "0 714 Boots Players/23 - B [643] charge 64",
            ]
        );
        let group = Some(TaskGroup {
            tasks: 0..4,
            charge: 63,
        });
        for index in 0..4 {
            assert_eq!(report.manifest.tasks[index].group, group, "task {index}");
        }
        assert_eq!(report.manifest.tasks[4].group, None);
        assert_eq!(report.manifest.tasks[5].group, None);
        let files = |index: usize| -> Vec<&str> {
            report.manifest.tasks[index]
                .kind
                .files()
                .iter()
                .map(|file| file.path.name())
                .collect()
        };
        assert_eq!(files(0), ["face_diff.bin", "face_high.fmdl"]);
        assert_eq!(files(1), ["kit_boots.fmdl", "kit_boots.skl"]);
        assert_eq!(files(2), ["glove_l.fmdl"]);
        assert_eq!(files(3), ["shirt.dds"]);
        for index in 0..4 {
            assert_eq!(
                report.manifest.tasks[index].kind.folder_path(),
                scope_path("Players/A"),
                "task {index}"
            );
        }
    }

    #[test]
    fn per_kit_models_plan_the_lowest_variant_alone_and_report_the_set_once() {
        let export = resolved(
            "co Midcup Variants",
            &[
                ("Players/05 - A/pants_kit2.fmdl", 16),
                ("Players/05 - A/pants_kit1.fmdl", 8),
                ("Players/05 - A/face_diff.bin", 1),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        let [message] = report.messages.as_slice() else {
            panic!("{:?}", report.messages);
        };
        assert_eq!(message.code.code, "kit_variant_model_left_out");
        assert_eq!(
            (message.severity, message.disposition),
            (Severity::Warning, Disposition::Keep)
        );
        assert_eq!(
            message.scope,
            Scope::Folder {
                export_id: ExportId(0),
                path: scope_path("Players/05 - A"),
            }
        );
        assert_eq!(
            message.context,
            [
                ("model".to_owned(), "pants_kitN.fmdl".to_owned()),
                ("used".to_owned(), "pants_kit1.fmdl".to_owned())
            ]
        );
        // `pants_kit2.fmdl` is read by no task, so no charge counts it.
        assert_eq!(
            summary(&report),
            ["0 714 Face Players/05 - A [71405] charge 9"]
        );
        let files: Vec<&str> = report.manifest.tasks[0]
            .kind
            .files()
            .iter()
            .map(|file| file.path.name())
            .collect();
        assert_eq!(files, ["face_diff.bin", "pants_kit1.fmdl"]);

        // A variant alone is an ordinary model, reported by nothing.
        let alone = resolved(
            "co Midcup Alone",
            &[("Players/05 - A/pants_kit2.fmdl", 16)],
            &[],
            None,
        );
        let report = plan_run(
            vec![to_plan(ExportId(0), alone, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );
        assert!(report.messages.is_empty(), "{:?}", report.messages);
        assert_eq!(
            summary(&report),
            ["0 714 Face Players/05 - A [71405] charge 16"]
        );

        // On a pre-Fox target the face converts and packs the whole set, listed once for the
        // game to respell: nothing to warn about, both variants read.
        let export = resolved(
            "co Midcup Variants",
            &[
                ("Players/05 - A/pants_kit2.fmdl", 16),
                ("Players/05 - A/pants_kit1.fmdl", 8),
            ],
            &[],
            None,
        );
        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes17,
            fpc::HideIds::CONVENTION,
        );
        assert!(report.messages.is_empty(), "{:?}", report.messages);
        let files: Vec<&str> = report.manifest.tasks[0]
            .kind
            .files()
            .iter()
            .map(|file| file.path.name())
            .collect();
        assert_eq!(files, ["pants_kit1.fmdl", "pants_kit2.fmdl"]);
    }

    #[test]
    fn on_pre_fox_a_set_no_face_xml_names_is_reported_once_per_folder_and_a_face_s_is_not() {
        let plan = |files: &[(&str, u64)]| {
            let export = resolved("co Midcup Variants", files, &[], None);
            plan_run(
                vec![to_plan(ExportId(0), export, two_team_colors(), None)],
                PesVersion::Pes17,
                fpc::HideIds::CONVENTION,
            )
        };

        // Under `ingame_face` his boots hold parts, and a shared boots folder is one
        // `boots.model`: neither has a `face.xml` to name the set.
        let report = plan(&[
            ("Players/05 - A/ingame_face.toml", 0),
            ("Players/05 - A/boots_kit1.model", 8),
            ("Players/05 - A/boots_kit2.model", 8),
            ("Players/07 - B/Studs.boots", 0),
            ("Boots/Studs/boots_kit1.model", 8),
            ("Boots/Studs/boots_kit2.model", 8),
        ]);
        assert_eq!(
            message_summary(&report),
            [
                (
                    "kit_variant_model_left_out",
                    "Players/05 - A",
                    Disposition::Keep
                ),
                (
                    "kit_variant_model_left_out",
                    "Boots/Studs",
                    Disposition::Keep
                ),
            ]
        );
        for message in &report.messages {
            assert_eq!(message.severity, Severity::Warning);
            assert_eq!(
                message.context,
                [
                    ("model".to_owned(), "boots_kitN.model".to_owned()),
                    ("used".to_owned(), "boots_kit1.model".to_owned())
                ]
            );
        }

        // A player's face and a shared face list the set once through the `face.xml`.
        let report = plan(&[
            ("Players/05 - A/pants_kit1.model", 8),
            ("Players/05 - A/pants_kit2.model", 8),
            ("Players/07 - B/Round.face", 0),
            ("Faces/Round/pants_kit1.model", 8),
            ("Faces/Round/pants_kit2.model", 8),
        ]);
        assert_eq!(
            message_summary(&report),
            [("link_combined", "Players/07 - B", Disposition::Keep)]
        );
    }

    #[test]
    fn the_textures_tasks_carry_the_export_s_player_kit_numbers() {
        let with_kits = resolved(
            "co Midcup Kits",
            &[
                ("Players/05 - A/face_high.fmdl", 1),
                ("Players/05 - A/pants_kit1.dds", 1),
                ("Common/tape_kit1.dds", 1),
                ("Kits/g1/kit.dds", 1),
                ("Kits/p3/kit.dds", 1),
            ],
            &["Kits/p1"],
            None,
        );
        let without_kits = resolved(
            "co Midcup Plain",
            &[
                ("Players/05 - A/face_high.fmdl", 1),
                ("Players/05 - A/pants_kit1.dds", 1),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![
                to_plan(ExportId(0), with_kits, two_team_colors(), None),
                to_plan(ExportId(1), without_kits, two_team_colors(), None),
            ],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        // The goalkeeper's `g1` is not a number of its own.
        let kits: Vec<(String, &[u8])> = report
            .manifest
            .tasks
            .iter()
            .filter_map(|task| match &task.kind {
                TaskKind::Textures { folder, kits } => Some((
                    format!("{} {}", task.export_id.0, folder.path.as_str()),
                    kits.as_slice(),
                )),
                TaskKind::CommonTextures { folder, kits, .. } => Some((
                    format!("{} {}", task.export_id.0, folder.as_str()),
                    kits.as_slice(),
                )),
                TaskKind::Models { .. }
                | TaskKind::CommonModels { .. }
                | TaskKind::Portrait { .. }
                | TaskKind::Kit { .. }
                | TaskKind::Logo { .. }
                | TaskKind::RefereeMarker { .. }
                | TaskKind::Collar { .. } => None,
            })
            .collect();
        assert_eq!(
            kits,
            [
                ("0 Players/05 - A".to_owned(), &[1, 3][..]),
                ("0 Common".to_owned(), &[1, 3][..]),
                ("1 Players/05 - A".to_owned(), &[][..]),
            ]
        );
    }

    #[test]
    fn a_plainly_linked_boots_folder_with_no_model_gets_no_task() {
        let export = resolved(
            "co Midcup Studs",
            &[
                ("Players/05 - A/Studs.boots", 0),
                ("Players/06 - B/Zebra.boots", 0),
                ("Boots/Studs/studs.dds", 16),
                ("Boots/Zebra/boots.fmdl", 8),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        // Neither a boots nor a textures task for Studs, and Zebra takes slot 06's
        // exclusive id, he being its one wearer.
        assert!(report.messages.is_empty(), "{:?}", report.messages);
        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/05 - A [71405] charge 0",
                "0 714 Face Players/06 - B [71406] charge 0",
                "0 714 Boots Boots/Zebra [626] charge 8",
            ]
        );
    }

    #[test]
    fn shared_folders_follow_the_players_boots_then_gloves_under_the_shared_ids_in_name_order() {
        let export = resolved(
            "co Midcup Shared",
            &[
                ("Players/03 - A/Zebra.boots", 0),
                ("Players/03 - A/Grip.gloves", 0),
                ("Players/07 - B/face_high.fmdl", 32),
                ("Players/07 - B/face_diff.bin", 1),
                ("Players/07 - B/apple.boots", 0),
                ("Players/11 - C/Zebra.boots", 0),
                ("Boots/Zebra/boots.fmdl", 8),
                ("Boots/Zebra/boots.skl", 2),
                ("Boots/Zebra/shirt.dds", 16),
                ("Boots/apple/kit_boots.fmdl", 4),
                ("Gloves/Grip/glove_l.fmdl", 64),
                ("Gloves/Grip/grip.dds", 128),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        // Slots 03 and 11 wear Zebra and 07 apple, so no player has a boots package; Zebra's
        // two wearers give it the block's first shared id, and apple and Grip their one
        // wearer's exclusive id. 03 and 11 hold no face model: theirs are blank.
        assert!(report.messages.is_empty(), "{:?}", report.messages);
        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/03 - A [71403] charge 0",
                "0 714 Face Players/07 - B [71407] charge 33",
                "0 714 Face Players/11 - C [71411] charge 0",
                "0 714 Boots Boots/apple [627] charge 4",
                "0 714 Boots Boots/Zebra [644] charge 10",
                "0 714 textures Boots/Zebra charge 16",
                "0 714 Gloves Gloves/Grip [623] charge 64",
                "0 714 textures Gloves/Grip charge 128",
            ]
        );
        let groups: Vec<Option<TaskGroup>> = report
            .manifest
            .tasks
            .iter()
            .map(|task| task.group.clone())
            .collect();
        let zebra = Some(TaskGroup {
            tasks: 4..6,
            charge: 26,
        });
        let grip = Some(TaskGroup {
            tasks: 6..8,
            charge: 192,
        });
        assert_eq!(
            groups,
            [
                None,
                None,
                None,
                None,
                zebra.clone(),
                zebra,
                grip.clone(),
                grip
            ]
        );
        let homes: Vec<&TextureHome> = report
            .manifest
            .tasks
            .iter()
            .filter_map(|task| match &task.kind {
                TaskKind::Models { folder, .. } | TaskKind::Textures { folder, .. } => {
                    Some(&folder.textures)
                }
                TaskKind::CommonTextures { .. }
                | TaskKind::CommonModels { .. }
                | TaskKind::Portrait { .. }
                | TaskKind::Kit { .. }
                | TaskKind::Logo { .. }
                | TaskKind::RefereeMarker { .. }
                | TaskKind::Collar { .. } => None,
            })
            .collect();
        assert_eq!(
            homes,
            [
                &TextureHome::PlayerCommon {
                    folder_name: "03 - A".to_owned()
                },
                &TextureHome::PlayerCommon {
                    folder_name: "07 - B".to_owned()
                },
                &TextureHome::PlayerCommon {
                    folder_name: "11 - C".to_owned()
                },
                &TextureHome::SharedOutput {
                    package: ModelPackage::Boots,
                    id: 627
                },
                &TextureHome::SharedOutput {
                    package: ModelPackage::Boots,
                    id: 644
                },
                &TextureHome::SharedOutput {
                    package: ModelPackage::Boots,
                    id: 644
                },
                &TextureHome::SharedOutput {
                    package: ModelPackage::Gloves,
                    id: 623
                },
                &TextureHome::SharedOutput {
                    package: ModelPackage::Gloves,
                    id: 623
                },
            ]
        );
        let files = |index: usize| -> Vec<&str> {
            report.manifest.tasks[index]
                .kind
                .files()
                .iter()
                .map(|file| file.path.as_str())
                .collect()
        };
        assert!(files(0).is_empty());
        assert_eq!(
            files(4),
            ["Boots/Zebra/boots.fmdl", "Boots/Zebra/boots.skl"]
        );
        assert_eq!(files(5), ["Boots/Zebra/shirt.dds"]);
        assert_eq!(
            report.manifest.tasks[4].kind.folder_path(),
            scope_path("Boots/Zebra")
        );
    }

    #[test]
    fn a_selected_gltf_drops_its_player_folder_at_planning_with_its_textures() {
        let files = [
            ("Players/05 - A/boots.glb", 4),
            ("Players/05 - A/boots.model", 8),
            ("Players/05 - A/skin.png", 2),
            ("Players/07 - B/boots.fmdl", 16),
        ];
        let export = resolved("co Midcup Gltf", &files, &[], None);

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        // Slot 05's folder plans nothing: no face 71405, no boots 625, no textures; the
        // `.model` the glTF beats is not converted in its place.
        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/07 - B [71407] charge 0",
                "0 714 Boots Players/07 - B [627] charge 16",
            ]
        );
        assert_eq!(
            message_summary(&report),
            [(
                "model_gltf_unsupported",
                "Players/05 - A",
                Disposition::DropFolder
            )]
        );
        let message = &report.messages[0];
        assert_eq!(message.severity, Severity::Error);
        assert_eq!(
            message.context,
            [("file".to_owned(), "boots.glb".to_owned())]
        );

        // On pre-Fox the glTF beats the FMDL beside it the same way, and each selected glTF
        // is reported; a glTF a `.model` of its stem beats is ignored.
        let files = [
            ("Players/05 - A/boots.fmdl", 8),
            ("Players/05 - A/boots.glb", 4),
            ("Players/05 - A/face/hat.gltf", 4),
            ("Players/07 - B/boots.glb", 4),
            ("Players/07 - B/boots.model", 16),
        ];
        let export = resolved("co Midcup Gltf", &files, &[], None);
        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes17,
            fpc::HideIds::CONVENTION,
        );
        let files: Vec<&str> = report
            .messages
            .iter()
            .map(|message| message.context[0].1.as_str())
            .collect();
        assert_eq!(files, ["boots.glb", "face/hat.gltf"]);
        assert_eq!(
            message_summary(&report),
            [
                (
                    "model_gltf_unsupported",
                    "Players/05 - A",
                    Disposition::DropFolder
                ),
                (
                    "model_gltf_unsupported",
                    "Players/05 - A",
                    Disposition::DropFolder
                ),
            ]
        );
        let folders: Vec<ScopePath> = report
            .manifest
            .tasks
            .iter()
            .map(|task| task.kind.folder_path())
            .collect();
        assert!(
            !folders.is_empty()
                && folders
                    .iter()
                    .all(|folder| folder.as_str() == "Players/07 - B"),
            "{folders:?}"
        );

        // A referee's folder leaves the referee roster the same way.
        let referees = resolved(
            "refs Cup",
            &[
                ("Players/Keeper/boots.glb", 4),
                ("Players/Ref B/boots.fmdl", 16),
            ],
            &[],
            Some(b"01 Keeper\n02 Ref B\n"),
        );
        let report = plan_run(
            vec![to_plan(ExportId(0), referees, None, None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );
        assert_eq!(
            message_summary(&report),
            [(
                "model_gltf_unsupported",
                "Players/Keeper",
                Disposition::DropFolder
            )]
        );
        // No blank face: a referee with no face model keeps the game's referee head.
        assert_eq!(
            summary(&report),
            ["0 999 Boots Players/Ref B [referee 2] charge 16"]
        );
    }

    #[test]
    fn a_shared_folder_s_selected_gltf_drops_it_with_every_player_linking_it() {
        let files = [
            ("Players/05 - A/Crocs.boots", 0),
            // Dropped for his own glTF, which is the one reported.
            ("Players/06 - B/Crocs.boots", 0),
            ("Players/06 - B/face/hat.gltf", 4),
            ("Players/07 - C/Round.face", 0),
            ("Players/09 - D/Mud.boots", 0),
            ("Boots/Crocs/boots.glb", 4),
            ("Boots/Mud/boots.glb", 4),
            ("Faces/Round/hat.glb", 4),
        ];
        // A model of the target's format beats Mud's glTF of its stem: Mud compiles.
        for (version, mud) in [
            (PesVersion::Pes21, &[("Boots/Mud/boots.fmdl", 16)][..]),
            (
                PesVersion::Pes17,
                &[("Boots/Mud/boots.model", 16), ("Boots/Mud/boots.mtl", 1)][..],
            ),
        ] {
            let export = resolved("co Midcup Gltf", &[&files[..], mud].concat(), &[], None);

            let mut kept = export.clone();
            let mut messages = Vec::new();
            drop_gltf_folders(ExportId(0), &mut kept, version, &mut messages);
            let names = |folders: &[SharedModelFolder]| -> Vec<String> {
                folders
                    .iter()
                    .map(|folder| folder.folder_name.clone())
                    .collect()
            };
            assert_eq!(names(&kept.export.boots), ["Mud"], "{version}");
            assert_eq!(names(&kept.export.faces), Vec::<String>::new(), "{version}");

            let report = plan_run(
                vec![to_plan(ExportId(0), export, two_team_colors(), None)],
                version,
                fpc::HideIds::CONVENTION,
            );

            assert_eq!(
                message_summary(&report),
                [
                    (
                        "model_gltf_unsupported",
                        "Players/05 - A",
                        Disposition::DropFolder
                    ),
                    (
                        "model_gltf_unsupported",
                        "Players/06 - B",
                        Disposition::DropFolder
                    ),
                    (
                        "model_gltf_unsupported",
                        "Players/07 - C",
                        Disposition::DropFolder
                    ),
                ],
                "{version}"
            );
            // A shared file by its export path, the folder's own below the folder.
            let files: Vec<&str> = report
                .messages
                .iter()
                .map(|message| message.context[0].1.as_str())
                .collect();
            assert_eq!(
                files,
                [
                    "Boots/Crocs/boots.glb",
                    "face/hat.gltf",
                    "Faces/Round/hat.glb"
                ],
                "{version}"
            );
            // Only slot 09 and Mud plan tasks.
            let folders: BTreeSet<String> = report
                .manifest
                .tasks
                .iter()
                .map(|task| task.kind.folder_path().as_str().to_owned())
                .collect();
            assert_eq!(
                folders,
                BTreeSet::from(["Boots/Mud".to_owned(), "Players/09 - D".to_owned()]),
                "{version}"
            );
        }
    }

    #[test]
    fn a_linked_shared_folder_s_link_to_a_gltf_beaten_common_model_drops_its_linker() {
        // `Faces/Round`'s `body.model.common` names the `.model` `Common/body.glb` beats on
        // Fox: the link would load the glTF, as the player's own would, so the linker is
        // dropped naming it, and the orphaned shared folder goes with him.
        let files = [
            ("Players/05 - A/Round.face", 0),
            ("Faces/Round/fcl_hair.fmdl", 8),
            ("Faces/Round/body.model.common", 0),
            ("Common/body.model", 8),
            ("Common/body.glb", 4),
        ];
        let export = resolved("co Midcup Gltf", &files, &[], None);

        let mut kept = export;
        let mut messages = Vec::new();
        drop_gltf_folders(ExportId(0), &mut kept, PesVersion::Pes21, &mut messages);

        let [message] = messages.as_slice() else {
            panic!("{messages:?}");
        };
        assert_eq!(message.code.code, "model_gltf_unsupported");
        assert_eq!(message.disposition, Disposition::DropFolder);
        assert_eq!(
            message.scope,
            Scope::Folder {
                export_id: ExportId(0),
                path: scope_path("Players/05 - A")
            }
        );
        assert_eq!(
            message.context,
            [("file".to_owned(), "Common/body.glb".to_owned())]
        );
        assert!(kept.export.faces.is_empty(), "{:?}", kept.export.faces);
        assert!(
            mapped_players(&kept.export).is_empty(),
            "the dropped player maps no slot"
        );
    }

    #[test]
    fn an_ingame_face_player_s_link_to_a_gltf_beaten_common_model_drops_him_on_pre_fox() {
        // On pre-Fox a marked player's `.common` link to a model is a part of his own
        // (`PreFoxPart`): a Common glTF of its stem beats the linked model, so the link
        // would load the glTF and the linker is dropped naming it.
        let files = [
            ("Players/05 - A/ingame_face.toml", 0),
            ("Players/05 - A/kit_boots.fmdl", 16),
            ("Players/05 - A/body.fmdl.common", 0),
            ("Common/body.fmdl", 8),
            ("Common/body.glb", 4),
        ];
        let export = resolved("co Midcup Gltf", &files, &[], None);

        let mut kept = export;
        let mut messages = Vec::new();
        drop_gltf_folders(ExportId(0), &mut kept, PesVersion::Pes17, &mut messages);

        let [message] = messages.as_slice() else {
            panic!("{messages:?}");
        };
        assert_eq!(message.code.code, "model_gltf_unsupported");
        assert_eq!(message.disposition, Disposition::DropFolder);
        assert_eq!(
            message.scope,
            Scope::Folder {
                export_id: ExportId(0),
                path: scope_path("Players/05 - A")
            }
        );
        assert_eq!(
            message.context,
            [("file".to_owned(), "Common/body.glb".to_owned())]
        );
        assert!(
            mapped_players(&kept.export).is_empty(),
            "the dropped player maps no slot"
        );
    }

    #[test]
    fn a_shared_folder_the_gltf_drop_orphans_is_removed_and_reports_nothing() {
        let files = [
            ("Players/05 - A/boots.glb", 4),
            ("Players/05 - A/Round.face", 0),
            ("Players/07 - C/face_high.fmdl", 8),
            ("Faces/Round/pants_kit1.fmdl", 8),
            ("Faces/Round/pants_kit2.fmdl", 16),
        ];
        let export = resolved("co Midcup Orphan", &files, &[], None);

        let mut kept = export.clone();
        drop_gltf_folders(ExportId(0), &mut kept, PesVersion::Pes21, &mut Vec::new());
        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        assert_eq!(
            message_summary(&report),
            [(
                "model_gltf_unsupported",
                "Players/05 - A",
                Disposition::DropFolder
            )]
        );
        assert!(kept.export.faces.is_empty(), "{:?}", kept.export.faces);
        assert_eq!(
            summary(&report),
            ["0 714 Face Players/07 - C [71407] charge 8"]
        );

        // Slot 07 linking it too keeps the folder, whose set is reported once.
        let linked = [&files[..], &[("Players/07 - C/Round.face", 0)]].concat();
        let export = resolved("co Midcup Orphan", &linked, &[], None);
        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        assert_eq!(
            message_summary(&report),
            [
                (
                    "model_gltf_unsupported",
                    "Players/05 - A",
                    Disposition::DropFolder
                ),
                (
                    "kit_variant_model_left_out",
                    "Faces/Round",
                    Disposition::Keep
                ),
                ("link_combined", "Players/07 - C", Disposition::Keep),
            ]
        );
        let variants: Vec<&Message> = report
            .messages
            .iter()
            .filter(|message| message.code.code == "kit_variant_model_left_out")
            .collect();
        assert_eq!(
            variants[0].context,
            [
                ("model".to_owned(), "pants_kitN.fmdl".to_owned()),
                ("used".to_owned(), "pants_kit1.fmdl".to_owned())
            ]
        );
    }

    #[test]
    fn a_link_beside_a_local_model_combines_the_shared_folder_into_the_player_s_package() {
        let files = [
            ("Players/05 - A/Crocs.boots", 0),
            ("Players/05 - A/kit_boots.fmdl", 8),
            ("Players/05 - A/kit_boots.skl", 2),
            ("Players/05 - A/Grip.gloves", 0),
            ("Players/05 - A/glove_l.fmdl", 4),
            ("Boots/Crocs/boots.fmdl", 16),
            ("Boots/Crocs/boots.skl", 1),
            ("Boots/Crocs/sole.dds", 32),
            ("Gloves/Grip/glove_r.fmdl", 64),
        ];
        let export = resolved("co Midcup Combined", &files, &[], None);

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        // The shared folders are only sources of parts: no shared id, no output of their own.
        // The player holds no face model: its face is blank.
        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/05 - A [71405] charge 0",
                "0 714 Boots Players/05 - A [625] charge 27",
                "0 714 Gloves Players/05 - A [625] charge 68",
                "0 714 textures Players/05 - A charge 32",
            ]
        );
        assert_eq!(
            message_summary(&report),
            [
                ("link_combined", "Players/05 - A", Disposition::Keep),
                ("link_combined", "Players/05 - A", Disposition::Keep),
            ]
        );
        let links: Vec<&str> = report
            .messages
            .iter()
            .map(|message| message.context[0].1.as_str())
            .collect();
        assert_eq!(links, ["Crocs.boots", "Grip.gloves"]);
        assert!(
            report
                .messages
                .iter()
                .all(|message| message.context[0].0 == "link"),
            "{:?}",
            report.messages
        );
        let files = |index: usize| -> Vec<&str> {
            report.manifest.tasks[index]
                .kind
                .files()
                .iter()
                .map(|file| file.path.as_str())
                .collect()
        };
        // The player's own files first, then each combined folder's, each skeleton pairing
        // with the model of its stem in its own folder.
        assert!(files(0).is_empty());
        assert_eq!(
            files(1),
            [
                "Players/05 - A/kit_boots.fmdl",
                "Players/05 - A/kit_boots.skl",
                "Boots/Crocs/boots.fmdl",
                "Boots/Crocs/boots.skl",
            ]
        );
        assert_eq!(
            files(2),
            ["Players/05 - A/glove_l.fmdl", "Gloves/Grip/glove_r.fmdl"]
        );
        assert_eq!(files(3), ["Boots/Crocs/sole.dds"]);
        let TaskKind::Models { folder, .. } = &report.manifest.tasks[1].kind else {
            panic!("a package task");
        };
        let combined: Vec<(ModelPackage, &str)> = folder
            .combined
            .iter()
            .map(|shared| (shared.package, shared.folder.path.as_str()))
            .collect();
        assert_eq!(
            combined,
            [
                (ModelPackage::Boots, "Boots/Crocs"),
                (ModelPackage::Gloves, "Gloves/Grip")
            ]
        );
        assert_eq!(
            folder.textures,
            TextureHome::PlayerCommon {
                folder_name: "05 - A".to_owned()
            }
        );
        let group = Some(TaskGroup {
            tasks: 0..4,
            charge: 127,
        });
        for index in 0..4 {
            assert_eq!(report.manifest.tasks[index].group, group, "task {index}");
        }
    }

    #[test]
    fn a_subfolder_s_parts_join_the_task_their_names_give_and_a_link_beside_them_combines() {
        let export = resolved(
            "co Midcup Subfolders",
            &[
                ("Players/05 - A/boots/boots.fmdl", 8),
                ("Players/05 - A/boots/boots.skl", 2),
                ("Players/05 - A/common/skin.dds", 16),
                ("Players/05 - A/Crocs.boots", 0),
                ("Boots/Crocs/boots.fmdl", 4),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        // The boots alone: the face of a folder whose only model is in `boots/` is blank.
        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/05 - A [71405] charge 0",
                "0 714 Boots Players/05 - A [625] charge 14",
                "0 714 textures Players/05 - A charge 16",
            ]
        );
        assert_eq!(
            message_summary(&report),
            [("link_combined", "Players/05 - A", Disposition::Keep)]
        );
        let files = |index: usize| -> Vec<&str> {
            report.manifest.tasks[index]
                .kind
                .files()
                .iter()
                .map(|file| file.path.as_str())
                .collect()
        };
        assert!(files(0).is_empty());
        assert_eq!(
            files(1),
            [
                "Players/05 - A/boots/boots.fmdl",
                "Players/05 - A/boots/boots.skl",
                "Boots/Crocs/boots.fmdl",
            ]
        );
        assert_eq!(files(2), ["Players/05 - A/common/skin.dds"]);
    }

    #[test]
    fn a_common_link_s_task_reads_the_common_model_and_skeleton_and_the_common_textures_are_one_task()
     {
        let export = resolved(
            "co Midcup Common",
            &[
                ("Players/05 - A/torso.fmdl", 8),
                ("Players/05 - A/legs.fmdl.common", 0),
                ("Players/05 - A/Kit_Boots.fmdl.common", 0),
                ("Players/05 - A/face_high.fmdl.common", 0),
                ("Players/05 - A/skin.dds", 4),
                ("Players/07 - B/legs.fmdl.common", 0),
                ("Players/07 - B/Crocs.boots", 0),
                ("Boots/Crocs/boots.fmdl", 2),
                ("Common/Legs.fmdl", 16),
                ("Common/legs.skl", 1),
                ("Common/kit_boots.fmdl", 32),
                ("Common/kit_boots.skl", 64),
                ("Common/face_high.fmdl", 128),
                ("Common/face_high.skl", 256),
                ("Common/spare.fmdl", 512),
                ("Common/Cloth.dds", 1024),
                ("Common/hair.ftex", 2048),
                ("Portraits/05.dds", 3),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        // Slot 05's face reads the local part, the two Common models and the hair's skeleton,
        // never the links nor the slotless `face_high.skl`; its boots read Common's model and
        // skeleton. The Common textures follow the shared folders and precede the portraits,
        // and the unlinked `spare.fmdl` is read by nothing.
        assert!(report.messages.is_empty(), "{:?}", report.messages);
        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/05 - A [71405] charge 153",
                "0 714 Boots Players/05 - A [625] charge 96",
                "0 714 textures Players/05 - A charge 4",
                "0 714 Face Players/07 - B [71407] charge 17",
                "0 714 Boots Boots/Crocs [627] charge 2",
                "0 714 common textures Common (2) charge 3072",
                "0 714 portrait 71405 Portraits/05.dds charge 3",
            ]
        );
        let files = |index: usize| -> Vec<&str> {
            report.manifest.tasks[index]
                .kind
                .files()
                .iter()
                .map(|file| file.path.as_str())
                .collect()
        };
        assert_eq!(
            files(0),
            [
                "Common/face_high.fmdl",
                "Common/Legs.fmdl",
                "Common/legs.skl",
                "Players/05 - A/torso.fmdl",
            ]
        );
        assert_eq!(files(1), ["Common/kit_boots.fmdl", "Common/kit_boots.skl"]);
        assert_eq!(files(3), ["Common/Legs.fmdl", "Common/legs.skl"]);
        assert_eq!(files(5), ["Common/Cloth.dds", "Common/hair.ftex"]);
        let TaskKind::Models { folder, .. } = &report.manifest.tasks[0].kind else {
            panic!("a package task");
        };
        let links: Vec<(&str, &str, Option<&str>)> = folder
            .common_models
            .iter()
            .map(|common| {
                (
                    common.link.as_str(),
                    common.model.path.as_str(),
                    common.skeleton.as_ref().map(|file| file.path.as_str()),
                )
            })
            .collect();
        assert_eq!(
            links,
            [
                (
                    "Players/05 - A/face_high.fmdl.common",
                    "Common/face_high.fmdl",
                    None
                ),
                (
                    "Players/05 - A/Kit_Boots.fmdl.common",
                    "Common/kit_boots.fmdl",
                    Some("Common/kit_boots.skl")
                ),
                (
                    "Players/05 - A/legs.fmdl.common",
                    "Common/Legs.fmdl",
                    Some("Common/legs.skl")
                ),
            ]
        );
        assert_eq!(
            folder.common_texture_stems,
            BTreeSet::from(["Cloth".to_owned(), "hair".to_owned()])
        );
        // The Common task is in no group: a player's packages never wait for it.
        let common = &report.manifest.tasks[5];
        assert_eq!(common.group, None);
        assert_eq!(common.kind.folder_path(), scope_path("Common"));
        assert_eq!(
            report.manifest.tasks[2].group,
            Some(TaskGroup {
                tasks: 0..3,
                charge: 253,
            })
        );
    }

    #[test]
    fn a_common_model_two_sources_link_is_one_part_of_the_player_s_own_source() {
        let export = resolved(
            "co Midcup Links",
            &[
                ("Players/05 - A/face_high.fmdl", 8),
                ("Players/05 - A/jessie/body.fmdl.common", 0),
                ("Players/05 - A/Round.face", 0),
                ("Faces/Round/fcl_hair.fmdl", 8),
                ("Faces/Round/jessie/body.fmdl.common", 0),
                ("Common/jessie/body.fmdl", 8),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        let Some(folder) = report
            .manifest
            .tasks
            .iter()
            .find_map(|task| match &task.kind {
                TaskKind::Models {
                    folder,
                    package: ModelPackage::Face,
                    ..
                } => Some(folder),
                _ => None,
            })
        else {
            panic!("a face task");
        };
        // Both links resolve, but `Common/jessie/body.fmdl` is one part, in the first
        // source it is linked from: the player's own.
        assert_eq!(folder.common_models.len(), 2);
        let sources: Vec<&str> = folder
            .roles()
            .iter()
            .filter(|(_, _, files)| {
                files.iter().any(|(file, role)| {
                    file.path.as_str() == "Common/jessie/body.fmdl"
                        && matches!(role, PlayerFile::CommonModel { .. })
                })
            })
            .map(|(_, path, _)| path.as_str())
            .collect();
        assert_eq!(sources, ["Players/05 - A"]);
    }

    #[test]
    fn a_texture_link_is_read_by_no_task_of_its_folder_and_its_texture_by_the_common_task() {
        let export = resolved(
            "co Midcup Links",
            &[
                ("Players/05 - A/face_high.fmdl", 8),
                ("Players/05 - A/skin.dds", 4),
                ("Players/05 - A/hair.dds.common", 0),
                ("Players/07 - B/face_high.fmdl", 16),
                ("Players/07 - B/hair.dds.common", 0),
                ("Players/07 - B/sole.png.common", 0),
                ("Common/hair.dds", 32),
                ("Common/sole.png", 64),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        // Slot 05's textures task reads its own `skin.dds` alone; slot 07, holding a model
        // and texture links only, has no textures task.
        assert!(report.messages.is_empty(), "{:?}", report.messages);
        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/05 - A [71405] charge 8",
                "0 714 textures Players/05 - A charge 4",
                "0 714 Face Players/07 - B [71407] charge 16",
                "0 714 common textures Common (2) charge 96",
            ]
        );
        let files = |index: usize| -> Vec<&str> {
            report.manifest.tasks[index]
                .kind
                .files()
                .iter()
                .map(|file| file.path.as_str())
                .collect()
        };
        assert_eq!(files(0), ["Players/05 - A/face_high.fmdl"]);
        assert_eq!(files(1), ["Players/05 - A/skin.dds"]);
        assert_eq!(files(2), ["Players/07 - B/face_high.fmdl"]);
        assert_eq!(files(3), ["Common/hair.dds", "Common/sole.png"]);
        assert_eq!(report.manifest.tasks[2].group, None);
    }

    #[test]
    fn a_face_link_alone_makes_the_shared_face_the_player_s_and_the_player_s_face_files_win() {
        let export = resolved(
            "co Midcup Faces",
            &[
                ("Players/05 - A/Longhair.face", 0),
                ("Players/05 - A/face_diff.bin", 3),
                ("Players/05 - A/skin.dds", 5),
                ("Faces/Longhair/hair_high.fmdl", 16),
                ("Faces/Longhair/face_diff.bin", 7),
                ("Faces/Longhair/hair.dds", 32),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        // The face task exists for the shared model alone, and is charged the player's
        // `face_diff.bin`, not the shared folder's, which is never read.
        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/05 - A [71405] charge 19",
                "0 714 textures Players/05 - A charge 37",
            ]
        );
        assert_eq!(
            message_summary(&report),
            [("link_combined", "Players/05 - A", Disposition::Keep)]
        );
        assert_eq!(report.messages[0].context[0].1, "Longhair.face");
        let files = |index: usize| -> Vec<&str> {
            report.manifest.tasks[index]
                .kind
                .files()
                .iter()
                .map(|file| file.path.as_str())
                .collect()
        };
        assert_eq!(
            files(0),
            [
                "Players/05 - A/face_diff.bin",
                "Faces/Longhair/hair_high.fmdl"
            ]
        );
        assert_eq!(
            files(1),
            ["Players/05 - A/skin.dds", "Faces/Longhair/hair.dds"]
        );
        let TaskKind::Models { folder, .. } = &report.manifest.tasks[0].kind else {
            panic!("a package task");
        };
        assert_eq!(folder.combined[0].package, ModelPackage::Face);
        let group = Some(TaskGroup {
            tasks: 0..2,
            charge: 56,
        });
        assert_eq!(report.manifest.tasks[0].group, group);
    }

    /// The files the face task of `co Midcup Faces` reads, its player folder `Players/05 - A`
    /// linking `Faces/Longhair` and holding `player_files`, the shared folder holding
    /// `hair_high.fmdl` and `shared_files`.
    fn face_task_files(player_files: &[&str], shared_files: &[&str]) -> Vec<String> {
        let mut files = vec![
            ("Players/05 - A/Longhair.face".to_owned(), 0),
            ("Faces/Longhair/hair_high.fmdl".to_owned(), 16),
        ];
        files.extend(
            player_files
                .iter()
                .map(|name| (format!("Players/05 - A/{name}"), 3)),
        );
        files.extend(
            shared_files
                .iter()
                .map(|name| (format!("Faces/Longhair/{name}"), 7)),
        );
        let files: Vec<(&str, u64)> = files
            .iter()
            .map(|(path, size)| (path.as_str(), *size))
            .collect();
        let export = resolved("co Midcup Faces", &files, &[], None);
        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );
        assert_eq!(
            report
                .messages
                .iter()
                .map(|message| message.code.code.as_ref())
                .collect::<Vec<&str>>(),
            ["link_combined"]
        );
        report.manifest.tasks[0]
            .kind
            .files()
            .iter()
            .map(|file| file.path.as_str().to_owned())
            .collect()
    }

    #[test]
    fn a_face_diff_xml_stands_for_the_face_diff_bin_and_a_second_copy_is_left_out() {
        // The player's own face diff wins over the shared folder's, whichever form each has,
        // and the shared one is never read.
        assert_eq!(
            face_task_files(&["face_diff.xml"], &["face_diff.bin"]),
            [
                "Players/05 - A/face_diff.xml",
                "Faces/Longhair/hair_high.fmdl"
            ]
        );
        assert_eq!(
            face_task_files(&["face_diff.bin"], &["face_diff.xml"]),
            [
                "Players/05 - A/face_diff.bin",
                "Faces/Longhair/hair_high.fmdl"
            ]
        );
    }

    #[test]
    fn a_combined_shared_folder_still_compiles_on_its_own_for_a_player_linking_it_plainly() {
        let export = resolved(
            "co Midcup Combined",
            &[
                ("Players/05 - A/Crocs.boots", 0),
                ("Players/05 - A/kit_boots.fmdl", 8),
                ("Players/07 - B/Crocs.boots", 0),
                ("Boots/Crocs/boots.fmdl", 16),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/05 - A [71405] charge 0",
                "0 714 Boots Players/05 - A [625] charge 24",
                "0 714 Face Players/07 - B [71407] charge 0",
                "0 714 Boots Boots/Crocs [627] charge 16",
            ]
        );
        assert_eq!(
            message_summary(&report),
            [("link_combined", "Players/05 - A", Disposition::Keep)]
        );
        let TaskKind::Models { folder, .. } = &report.manifest.tasks[3].kind else {
            panic!("a package task");
        };
        assert!(folder.combined.is_empty(), "the shared folder's own task");
    }

    #[test]
    fn a_marked_folder_plans_no_face_and_every_other_folder_a_face_blank_without_a_face_model() {
        let export = resolved(
            "co Midcup Faces",
            &[
                ("Players/05 - A/ingame_face.toml", 0),
                ("Players/05 - A/torso.fmdl", 8),
                ("Players/05 - A/glove_l.fmdl", 4),
                ("Players/07 - B/boots.fmdl", 16),
            ],
            &["Players/09 - C/face"],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        assert!(report.messages.is_empty(), "{:?}", report.messages);
        assert_eq!(
            summary(&report),
            [
                "0 714 Boots Players/05 - A [625] charge 8",
                "0 714 Gloves Players/05 - A [625] charge 4",
                "0 714 Face Players/07 - B [71407] charge 0",
                "0 714 Boots Players/07 - B [627] charge 16",
                "0 714 Face Players/09 - C [71409] charge 0",
            ]
        );
        let files = |index: usize| -> Vec<&str> {
            report.manifest.tasks[index]
                .kind
                .files()
                .iter()
                .map(|file| file.path.as_str())
                .collect()
        };
        assert_eq!(files(0), ["Players/05 - A/torso.fmdl"]);
        assert!(files(2).is_empty());
        assert!(files(4).is_empty());
    }

    #[test]
    fn under_ingame_face_a_boots_link_beside_a_model_the_face_would_take_combines() {
        let plan = |files: &[(&str, u64)]| {
            let export = resolved("co Midcup Marked", files, &[], None);
            plan_run(
                vec![to_plan(ExportId(0), export, two_team_colors(), None)],
                PesVersion::Pes21,
                fpc::HideIds::CONVENTION,
            )
        };
        let shared = ("Boots/Crocs/boots.fmdl", 16);

        // The rerouted model is a boots part, so the link combines and Crocs takes no id.
        let report = plan(&[
            ("Players/05 - A/ingame_face.toml", 0),
            ("Players/05 - A/torso.fmdl", 8),
            ("Players/05 - A/Crocs.boots", 0),
            shared,
        ]);
        assert_eq!(
            summary(&report),
            ["0 714 Boots Players/05 - A [625] charge 24"]
        );
        assert_eq!(
            message_summary(&report),
            [("link_combined", "Players/05 - A", Disposition::Keep)]
        );

        // The link alone loads Crocs as it is, under slot 05's exclusive id, he being its
        // one wearer; the player has no task.
        let report = plan(&[
            ("Players/05 - A/ingame_face.toml", 0),
            ("Players/05 - A/Crocs.boots", 0),
            shared,
        ]);
        assert_eq!(
            summary(&report),
            ["0 714 Boots Boots/Crocs [625] charge 16"]
        );
        assert!(report.messages.is_empty(), "{:?}", report.messages);
    }

    #[test]
    fn portraits_follow_the_faces_by_player_id_one_per_slot_of_their_folder() {
        let export = resolved(
            "dbg Midcup Portraits",
            &[
                ("Players/Zed/face_high.fmdl", 10),
                ("Players/Zed/face_diff.bin", 0),
                ("Players/Zed/portrait.dds", 4),
                ("Portraits/05.dds", 6),
                ("Portraits/01.dds", 5),
                ("Kits/g1/kit.dds", 7),
            ],
            &[],
            Some(b"07 Zed\n03 Zed\n"),
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        assert_eq!(
            summary(&report),
            [
                "0 790 Face Players/Zed [79003, 79007] charge 10",
                "0 790 portrait 79001 Portraits/01.dds charge 5",
                "0 790 portrait 79003 Players/Zed/portrait.dds charge 4",
                "0 790 portrait 79005 Portraits/05.dds charge 6",
                "0 790 portrait 79007 Players/Zed/portrait.dds charge 4",
                "0 790 kit g1 Kits/g1 charge 7",
            ]
        );
        let portrait = &report.manifest.tasks[2].kind;
        assert_eq!(
            portrait.folder_path(),
            scope_path("Players/Zed/portrait.dds")
        );
        let files: Vec<&str> = portrait
            .files()
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        assert_eq!(files, ["Players/Zed/portrait.dds"]);
    }

    #[test]
    fn a_player_id_with_a_portrait_from_both_sources_gets_one_task_over_the_folder_s_file() {
        // Slot 07's `Portraits/` file beside the folder slots 03 and 07 map: the deep pass
        // has found the two identical, so the folder's file is the one portrait.
        let export = resolved(
            "dbg Midcup Portraits",
            &[
                ("Players/Zed/face_high.fmdl", 10),
                ("Players/Zed/face_diff.bin", 0),
                ("Players/Zed/portrait.dds", 4),
                ("Portraits/07.dds", 4),
                ("Portraits/05.dds", 6),
            ],
            &[],
            Some(b"07 Zed\n03 Zed\n"),
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        assert_eq!(
            summary(&report),
            [
                "0 790 Face Players/Zed [79003, 79007] charge 10",
                "0 790 portrait 79003 Players/Zed/portrait.dds charge 4",
                "0 790 portrait 79005 Portraits/05.dds charge 6",
                "0 790 portrait 79007 Players/Zed/portrait.dds charge 4",
            ]
        );
    }

    #[test]
    fn a_kit_without_config_toml_reports_its_generated_config() {
        let export = resolved(
            "co Full Kits",
            &[("Kits/p1/config.toml", 3), ("Kits/p2 - Away/kit.dds", 8)],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(3), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        let messages: Vec<(&str, &Scope, Disposition)> = report
            .messages
            .iter()
            .map(|message| {
                (
                    message.code.code.as_ref(),
                    &message.scope,
                    message.disposition,
                )
            })
            .collect();
        let scope = Scope::Folder {
            export_id: ExportId(3),
            path: ScopePath::new("Kits/p2 - Away").unwrap(),
        };
        // p1 holds only its config, so it is a placeholder kit.
        let p1 = Scope::Folder {
            export_id: ExportId(3),
            path: ScopePath::new("Kits/p1").unwrap(),
        };
        assert_eq!(
            messages,
            [
                ("kit_placeholder", &p1, Disposition::Keep),
                ("kit_config_generated", &scope, Disposition::Keep),
                // A `Full` export holding no `g1/` plans an empty one: generated,
                // a placeholder.
                (
                    "kit_config_generated",
                    &Scope::Folder {
                        export_id: ExportId(3),
                        path: ScopePath::new("Kits/g1").unwrap(),
                    },
                    Disposition::Keep
                ),
                (
                    "kit_placeholder",
                    &Scope::Folder {
                        export_id: ExportId(3),
                        path: ScopePath::new("Kits/g1").unwrap(),
                    },
                    Disposition::Keep
                ),
            ]
        );
        assert_eq!(
            report.manifest.tasks[1].kind.folder_path(),
            scope_path("Kits/p2 - Away")
        );
        assert_eq!(
            report.manifest.tasks[0].kind.folder_path(),
            scope_path("Kits/p1")
        );
    }

    /// Each planning message as (code, scope path, disposition).
    fn message_summary(report: &PlanReport) -> Vec<(&str, &str, Disposition)> {
        report
            .messages
            .iter()
            .map(|message| {
                let Scope::Folder { path, .. } = &message.scope else {
                    panic!("{:?}", message.scope);
                };
                (
                    message.code.code.as_ref(),
                    path.as_str(),
                    message.disposition,
                )
            })
            .collect()
    }

    #[test]
    fn a_kit_without_a_main_texture_reports_the_placeholder_after_its_config() {
        let export = resolved(
            "co Full Kits",
            &[
                ("Kits/p1/kit.dds", 8),
                ("Kits/p3/kit_back.dds", 8),
                ("Kits/p4/config.toml", 3),
            ],
            &["Kits/p2"],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        assert_eq!(
            message_summary(&report),
            [
                ("kit_config_generated", "Kits/p1", Disposition::Keep),
                ("kit_config_generated", "Kits/p2", Disposition::Keep),
                ("kit_placeholder", "Kits/p2", Disposition::Keep),
                ("kit_config_generated", "Kits/p3", Disposition::Keep),
                ("kit_placeholder", "Kits/p3", Disposition::Keep),
                ("kit_placeholder", "Kits/p4", Disposition::Keep),
                ("kit_config_generated", "Kits/g1", Disposition::Keep),
                ("kit_placeholder", "Kits/g1", Disposition::Keep),
            ]
        );
        assert!(
            report
                .messages
                .iter()
                .all(|message| message.context.is_empty()),
            "{:?}",
            report.messages
        );
    }

    #[test]
    fn a_kit_inheriting_the_main_texture_from_all_is_no_placeholder() {
        // The inheritance finding is validation's, not planning's.
        let (export, issues) = resolved_with_issues(
            "co Midcup Kits",
            &[("Kits/all/kit.dds", 8), ("Kits/p1/config.toml", 3)],
            &[],
            None,
        );
        assert_eq!(issues, ["kit_textures_inherited"]);

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        assert!(report.messages.is_empty(), "{:?}", report.messages);
        let [task] = report.manifest.tasks.as_slice() else {
            panic!("{}", report.manifest.tasks.len());
        };
        let TaskKind::Kit { kit, .. } = &task.kind else {
            panic!("a kit task");
        };
        let stems: Vec<&str> = kit
            .textures
            .iter()
            .map(|texture| texture.stem.as_str())
            .collect();
        assert_eq!(stems, ["kit"]);
    }

    /// The team kit-FPC status each kit task of the export `co Midcup Fpc` carries, the export
    /// holding the kits `p1` and `g1`, `players` (path, size) and the roster `players_txt`, and
    /// validation reporting exactly the codes `issues` on it.
    fn kit_fpc(
        players: &[(&str, u64)],
        players_txt: Option<&[u8]>,
        issues: &[&str],
    ) -> Vec<EffectiveTeamKitFpc> {
        let files: Vec<(&str, u64)> = [("Kits/p1/kit.dds", 1), ("Kits/g1/kit.dds", 1)]
            .into_iter()
            .chain(players.iter().copied())
            .collect();
        let (export, codes_found) = resolved_with_issues("co Midcup Fpc", &files, &[], players_txt);
        assert_eq!(codes_found, issues, "validation's issues");
        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );
        report
            .manifest
            .tasks
            .iter()
            .filter_map(|task| match &task.kind {
                TaskKind::Kit { edits, .. } => Some(edits.fpc),
                TaskKind::Models { .. }
                | TaskKind::Textures { .. }
                | TaskKind::CommonTextures { .. }
                | TaskKind::CommonModels { .. }
                | TaskKind::Portrait { .. }
                | TaskKind::Logo { .. }
                | TaskKind::RefereeMarker { .. }
                | TaskKind::Collar { .. } => None,
            })
            .collect()
    }

    #[test]
    fn one_fpc_on_folder_among_others_makes_every_kit_of_the_team_fpc() {
        use EffectiveTeamKitFpc::{On, Unknown};
        let a = ("Players/03 - A/face_high.fmdl", 1);
        let b = ("Players/05 - B/face_high.fmdl", 1);
        let c = ("Players/07 - C/face_high.fmdl", 1);
        let on = ("Players/05 - B/fpc_on", 0);
        let off = ("Players/07 - C/fpc_off", 0);

        assert_eq!(kit_fpc(&[a, b, on, c, off], None, &[]), [On, On]);
        assert_eq!(kit_fpc(&[a, b, c, off], None, &[]), [Unknown, Unknown]);
        assert_eq!(kit_fpc(&[a, b, c], None, &[]), [Unknown, Unknown]);
        // A folder validation dropped (no roster line maps it) compiles no player.
        assert_eq!(
            kit_fpc(&[a, b, on], Some(b"03 03 - A\n"), &["player_unlisted"]),
            [Unknown, Unknown]
        );
    }

    #[test]
    fn an_fpc_on_folder_planning_drops_for_its_gltf_does_not_count() {
        use EffectiveTeamKitFpc::Unknown;
        let a = ("Players/03 - A/face_high.fmdl", 1);
        // A selected glTF: planning drops the folder, which compiles no player.
        let b = ("Players/05 - B/face_high.glb", 1);
        let on = ("Players/05 - B/fpc_on", 0);

        assert_eq!(kit_fpc(&[a, b, on], None, &[]), [Unknown, Unknown]);
    }

    #[test]
    fn a_face_task_names_its_player_folder() {
        let export = resolved(
            "co Midcup One",
            &[
                ("Players/04 - B/face_high.fmdl", 1),
                ("Players/04 - B/face_diff.bin", 0),
            ],
            &[],
            None,
        );
        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );
        assert_eq!(
            report.manifest.tasks[0].kind.folder_path(),
            scope_path("Players/04 - B")
        );
    }

    /// Each planning message's code, on the export or one of its folders.
    fn codes(report: &PlanReport) -> Vec<&str> {
        report
            .messages
            .iter()
            .map(|message| message.code.code.as_ref())
            .collect()
    }

    #[test]
    fn a_team_s_colors_go_into_the_manifest_and_a_team_without_them_reports_it() {
        let kit = || resolved("co Full Kit", &[("Kits/p1/kit.dds", 1)], &[], None);
        let colors = vec![[0xc1, 0x12, 0x00], [0x41, 0x41, 0x41]];

        let with = plan_run(
            vec![to_plan(ExportId(0), kit(), Some(colors.clone()), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );
        assert_eq!(with.manifest.team_colors, [(714, colors.clone())]);
        // The export holds no `g1/`: a `Full` export plans an empty one beside its p1.
        assert_eq!(
            codes(&with),
            [
                "kit_config_generated",
                "kit_config_generated",
                "kit_placeholder"
            ]
        );

        let without = plan_run(
            vec![to_plan(ExportId(4), kit(), None, None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );
        assert_eq!(without.manifest.team_colors, []);
        let [missing, generated, ..] = without.messages.as_slice() else {
            panic!("{:?}", without.messages);
        };
        assert_eq!(missing.code.code, "team_colors_missing");
        assert_eq!(
            (missing.severity, missing.disposition),
            (Severity::Info, Disposition::Keep)
        );
        assert_eq!(
            missing.scope,
            Scope::Export {
                export_id: ExportId(4)
            }
        );
        assert!(missing.context.is_empty());
        assert_eq!(generated.code.code, "kit_config_generated");

        // A file whose every line was refused: the deep pass reported the lines.
        let refused_lines = plan_run(
            vec![to_plan(ExportId(0), kit(), Some(Vec::new()), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );
        assert_eq!(refused_lines.manifest.team_colors, []);
        assert_eq!(
            codes(&refused_lines),
            [
                "kit_config_generated",
                "kit_config_generated",
                "kit_placeholder"
            ]
        );

        // Two teams' colors, in export order.
        let dbg = resolved("dbg Midcup Kit", &[("Kits/p1/kit.dds", 1)], &[], None);
        let both = plan_run(
            vec![
                to_plan(ExportId(0), dbg, Some(vec![[1, 2, 3]]), None),
                to_plan(ExportId(1), kit(), Some(colors.clone()), None),
            ],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );
        assert_eq!(
            both.manifest.team_colors,
            [(790, vec![[1, 2, 3]]), (714, colors)]
        );
    }

    #[test]
    fn the_notes_of_the_planned_exports_go_into_the_manifest_in_export_order() {
        let kit = |name| resolved(name, &[("Kits/p1/kit.dds", 1)], &[], None);
        let referees = resolved(
            "refs Cup",
            &[
                ("Players/Keeper/face_high.fmdl", 1),
                ("Players/Keeper/face_diff.bin", 0),
            ],
            &[],
            Some(b"01 Keeper\n"),
        );
        let note = |text: &str| Some(text.to_owned());

        let report = plan_run(
            vec![
                to_plan(ExportId(0), kit("dbg Midcup Kit"), None, note("dbg's note")),
                to_plan(ExportId(1), kit("co Midcup Plain"), None, None),
                to_plan(ExportId(2), referees, None, note("the referees' note")),
                to_plan(ExportId(3), kit("co Midcup Kit"), None, note("co's note")),
            ],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        assert_eq!(
            report.manifest.notes,
            [
                ("/dbg/".to_owned(), "dbg's note".to_owned()),
                ("/refs/".to_owned(), "the referees' note".to_owned()),
                ("/co/".to_owned(), "co's note".to_owned())
            ]
        );
    }

    fn referee(slot: u8) -> PackageKey {
        PackageKey::Referee(aesthetics_export::RefSlot::new(slot).unwrap())
    }

    /// The keys of `report`'s `Models` tasks, in manifest order.
    fn model_keys(report: &PlanReport) -> Vec<Vec<PackageKey>> {
        report
            .manifest
            .tasks
            .iter()
            .filter_map(|task| match &task.kind {
                TaskKind::Models { ids, .. } => Some(ids.clone()),
                TaskKind::Textures { .. }
                | TaskKind::CommonTextures { .. }
                | TaskKind::CommonModels { .. }
                | TaskKind::Portrait { .. }
                | TaskKind::Kit { .. }
                | TaskKind::Logo { .. }
                | TaskKind::RefereeMarker { .. }
                | TaskKind::Collar { .. } => None,
            })
            .collect()
    }

    #[test]
    fn a_referee_folder_is_prepared_once_and_emitted_under_each_of_his_slots() {
        let referees = resolved(
            "refs Cup",
            &[
                ("Players/Ref A/face_high.fmdl", 10),
                ("Players/Ref A/boots.fmdl", 20),
                ("Players/Ref A/skin.dds", 5),
            ],
            &[],
            Some(b"20 Ref A\n01 Ref A\n35 Ref A\n"),
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), referees, None, None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        assert_eq!(
            summary(&report),
            [
                "0 999 Face Players/Ref A [referee 1, referee 20, referee 35] charge 10",
                "0 999 Boots Players/Ref A [referee 1, referee 20, referee 35] charge 20",
                "0 999 textures Players/Ref A charge 5",
            ]
        );
        let slots = vec![referee(1), referee(20), referee(35)];
        assert_eq!(model_keys(&report), [slots.clone(), slots]);
        let TaskKind::Textures { folder, .. } = &report.manifest.tasks[2].kind else {
            panic!("the third task is the folder's textures");
        };
        assert_eq!(
            folder.textures,
            TextureHome::PlayerCommon {
                folder_name: "Ref A".to_owned()
            }
        );
        assert!(report.messages.is_empty(), "{:?}", report.messages);
        assert!(report.manifest.team_colors.is_empty());
        assert!(report.manifest.team_kits.is_empty());
        assert!(report.manifest.patch_teams.is_empty());
    }

    #[test]
    fn a_folder_planning_no_package_plans_no_textures_task_as_plans_a_package_says() {
        for version in [PesVersion::Pes21, PesVersion::Pes17] {
            // Ref A and the marked 05 hold textures alone; Ref B and 07 a model beside them,
            // Ref C a link to a boots folder his own package is built from.
            let referees = resolved(
                "refs Cup",
                &[
                    ("Players/Ref A/skin.dds", 5),
                    ("Players/Ref B/face_high.fmdl", 10),
                    ("Players/Ref B/skin.dds", 5),
                    ("Players/Ref C/Studs.boots", 0),
                    ("Players/Ref C/skin.dds", 5),
                    ("Boots/Studs/boots.fmdl", 20),
                ],
                &[],
                Some(b"01 Ref A\n02 Ref B\n03 Ref C\n"),
            );
            let team = resolved(
                "co Midcup Faces",
                &[
                    ("Players/05 - A/ingame_face.toml", 0),
                    ("Players/05 - A/skin.dds", 5),
                    ("Players/07 - B/ingame_face.toml", 0),
                    ("Players/07 - B/boots.fmdl", 16),
                    ("Players/07 - B/skin.dds", 5),
                ],
                &[],
                None,
            );
            let plans = |export: &ResolvedAestheticsExport| -> Vec<(String, bool)> {
                mapped_players(&export.export)
                    .into_iter()
                    .map(|folder| {
                        let plans = roles::plans_a_package(
                            &export.export,
                            folder,
                            version.engine(),
                            &BTreeSet::new(),
                        );
                        (folder.path.as_str().to_owned(), plans)
                    })
                    .collect()
            };
            let mut expected = plans(&referees);
            expected.extend(plans(&team));
            assert_eq!(
                expected,
                [
                    ("Players/Ref A".to_owned(), false),
                    ("Players/Ref B".to_owned(), true),
                    ("Players/Ref C".to_owned(), true),
                    ("Players/05 - A".to_owned(), false),
                    ("Players/07 - B".to_owned(), true),
                ],
                "{version:?}"
            );

            let report = plan_run(
                vec![
                    to_plan(ExportId(0), referees, None, None),
                    to_plan(ExportId(1), team, two_team_colors(), None),
                ],
                version,
                fpc::HideIds::CONVENTION,
            );

            let textures: Vec<&str> = report
                .manifest
                .tasks
                .iter()
                .filter_map(|task| match &task.kind {
                    TaskKind::Textures { folder, .. } => Some(folder.path.as_str()),
                    TaskKind::Models { .. }
                    | TaskKind::CommonTextures { .. }
                    | TaskKind::CommonModels { .. }
                    | TaskKind::Portrait { .. }
                    | TaskKind::Kit { .. }
                    | TaskKind::Logo { .. }
                    | TaskKind::RefereeMarker { .. }
                    | TaskKind::Collar { .. } => None,
                })
                .collect();
            assert_eq!(
                textures,
                ["Players/Ref B", "Players/Ref C", "Players/07 - B"],
                "{version:?}"
            );
        }
    }

    #[test]
    fn a_referee_s_plain_link_is_a_part_of_his_slots_own_package_and_he_gets_no_face_task() {
        for version in [PesVersion::Pes21, PesVersion::Pes17] {
            let referees = resolved(
                "refs Cup",
                &[
                    ("Players/Ref A/Studs.boots", 0),
                    ("Boots/Studs/boots.fmdl", 20),
                ],
                &[],
                Some(b"01 Ref A\n20 Ref A\n"),
            );

            let report = plan_run(
                vec![to_plan(ExportId(0), referees, None, None)],
                version,
                fpc::HideIds::CONVENTION,
            );

            // No blank face: a referee has no FPC body to bring a head.
            assert_eq!(
                summary(&report),
                ["0 999 Boots Players/Ref A [referee 1, referee 20] charge 20"],
                "{version:?}: no task of Boots/Studs's own"
            );
            let TaskKind::Models { folder, .. } = &report.manifest.tasks[0].kind else {
                panic!("{version:?}: the one task is the boots");
            };
            assert_eq!(
                folder
                    .combined
                    .iter()
                    .map(|combined| (combined.package, combined.folder.path.as_str()))
                    .collect::<Vec<_>>(),
                [(ModelPackage::Boots, "Boots/Studs")],
                "{version:?}"
            );
            assert!(
                report.messages.is_empty(),
                "{version:?}: {:?}",
                report.messages
            );
        }
    }

    #[test]
    fn a_referee_with_a_face_model_and_a_plain_link_gets_his_face_and_his_boots_tasks() {
        for version in [PesVersion::Pes21, PesVersion::Pes17] {
            let referees = resolved(
                "refs Cup",
                &[
                    ("Players/Ref A/face_high.fmdl", 10),
                    ("Players/Ref A/Studs.boots", 0),
                    ("Boots/Studs/boots.fmdl", 20),
                ],
                &[],
                Some(b"01 Ref A\n20 Ref A\n"),
            );

            let report = plan_run(
                vec![to_plan(ExportId(0), referees, None, None)],
                version,
                fpc::HideIds::CONVENTION,
            );

            assert_eq!(
                summary(&report),
                [
                    "0 999 Face Players/Ref A [referee 1, referee 20] charge 10",
                    "0 999 Boots Players/Ref A [referee 1, referee 20] charge 20",
                ],
                "{version:?}"
            );
            assert!(
                report.messages.is_empty(),
                "{version:?}: {:?}",
                report.messages
            );
        }
    }

    #[test]
    fn a_pre_fox_referee_holding_fpc_off_takes_the_refkit_body_and_no_one_else_does() {
        let refs_face = |version: PesVersion| {
            let referees = resolved(
                "refs Cup",
                &[
                    ("Players/Ref A/face_high.fmdl", 10),
                    ("Players/Ref A/fpc_off", 0),
                ],
                &[],
                Some(b"01 Ref A\n"),
            );
            let report = plan_run(
                vec![to_plan(ExportId(0), referees, None, None)],
                version,
                fpc::HideIds::CONVENTION,
            );
            assert_eq!(
                summary(&report),
                ["0 999 Face Players/Ref A [referee 1] charge 10"],
                "{version:?}"
            );
            models_folder(&report.manifest.tasks[0]).refkit_body
        };
        assert!(refs_face(PesVersion::Pes17));
        // The Fox refkit is not made yet.
        assert!(!refs_face(PesVersion::Pes21));

        // A team player's body is the kit's, `fpc_off` or not.
        let team = resolved(
            "co Midcup One",
            &[
                ("Players/05 - A/face_high.fmdl", 10),
                ("Players/05 - A/fpc_off", 0),
            ],
            &[],
            None,
        );
        let report = plan_run(
            vec![to_plan(ExportId(0), team, two_team_colors(), None)],
            PesVersion::Pes17,
            fpc::HideIds::CONVENTION,
        );
        assert_eq!(
            summary(&report),
            ["0 714 Face Players/05 - A [71405] charge 10"]
        );
        assert!(!models_folder(&report.manifest.tasks[0]).refkit_body);
    }

    #[test]
    fn a_pre_fox_face_link_combines_and_a_boots_link_loads_the_shared_output() {
        let export = resolved(
            "co Midcup Shared",
            &[
                ("Players/05 - A/face_high.model", 4),
                ("Players/05 - A/face_high.mtl", 1),
                ("Players/05 - A/Round.face", 0),
                ("Players/06 - B/Crocs.boots", 0),
                ("Faces/Round/hair_high.model", 8),
                ("Faces/Round/hair_high.mtl", 2),
                ("Boots/Crocs/boots.model", 16),
                ("Boots/Crocs/boots.mtl", 1),
                ("Boots/Crocs/crocs.dds", 32),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes17,
            fpc::HideIds::CONVENTION,
        );

        // The shared face is a second source of slot 05's face; the shared boots are their
        // own output under slot 06's exclusive id, he being their one wearer.
        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/05 - A [71405] charge 15",
                "0 714 Face Players/06 - B [71406] charge 0",
                "0 714 Boots Boots/Crocs [626] charge 17",
                "0 714 textures Boots/Crocs charge 32",
            ]
        );
        assert_eq!(
            message_summary(&report),
            [("link_combined", "Players/05 - A", Disposition::Keep)]
        );
        assert_eq!(
            report.messages[0].context,
            [("link".to_owned(), "Round.face".to_owned())]
        );
        let combined = |index: usize| -> Vec<(ModelPackage, &str)> {
            models_folder(&report.manifest.tasks[index])
                .combined
                .iter()
                .map(|shared| (shared.package, shared.folder.path.as_str()))
                .collect()
        };
        assert_eq!(combined(0), [(ModelPackage::Face, "Faces/Round")]);
        assert_eq!(combined(1), []);
        assert_eq!(
            task_files(&report.manifest.tasks[2]),
            ["Boots/Crocs/boots.model", "Boots/Crocs/boots.mtl"]
        );
    }

    #[test]
    fn a_pre_fox_face_reads_the_skeleton_of_each_fmdl_it_converts_and_not_the_fclo() {
        let export = resolved(
            "co Midcup Convert",
            &[
                ("Players/05 - A/face_diff.bin", 1),
                ("Players/05 - A/fcl_hair.fmdl", 4),
                ("Players/05 - A/fcl_hair.skl", 2),
                ("Players/05 - A/fcl_hair_sim.fclo", 8),
                ("Players/05 - A/shirt.dds", 16),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes17,
            fpc::HideIds::CONVENTION,
        );

        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/05 - A [71405] charge 7",
                "0 714 textures Players/05 - A charge 16",
            ]
        );
        assert_eq!(
            task_files(&report.manifest.tasks[0]),
            [
                "Players/05 - A/face_diff.bin",
                "Players/05 - A/fcl_hair.fmdl",
                "Players/05 - A/fcl_hair.skl",
            ]
        );
    }

    #[test]
    fn a_fox_package_converting_a_model_reads_the_folder_s_mtl_files_and_one_of_fmdls_none() {
        let export = resolved(
            "co Midcup Convert",
            &[
                ("Players/05 - A/boots.model", 4),
                ("Players/05 - A/boots.mtl", 1),
                ("Players/05 - A/materials.mtl", 2),
                // The face holds only FMDLs: the `.model` of the hair's stem is beaten.
                ("Players/05 - A/fcl_hair.fmdl", 8),
                ("Players/05 - A/fcl_hair.model", 16),
                ("Players/05 - A/shirt.dds", 32),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/05 - A [71405] charge 8",
                "0 714 Boots Players/05 - A [625] charge 7",
                "0 714 textures Players/05 - A charge 32",
            ]
        );
        assert_eq!(
            task_files(&report.manifest.tasks[1]),
            [
                "Players/05 - A/boots.model",
                "Players/05 - A/boots.mtl",
                "Players/05 - A/materials.mtl",
            ]
        );
    }

    #[test]
    fn a_pre_fox_marked_player_s_mtl_files_are_read_by_his_boots_and_his_gloves() {
        let export = resolved(
            "co Midcup Marked",
            &[
                ("Players/05 - A/ingame_face.toml", 0),
                ("Players/05 - A/kit_boots.model", 4),
                ("Players/05 - A/x_gloveL.model", 2),
                ("Players/05 - A/materials.mtl", 1),
                ("Players/05 - A/Keeper.gloves", 0),
                ("Players/07 - B/ingame_face.toml", 0),
                ("Players/07 - B/x_gloveR.model", 2),
                ("Players/07 - B/x_gloveR.mtl", 1),
                ("Players/07 - B/Crocs.boots", 0),
                ("Players/07 - B/Keeper.gloves", 0),
                ("Gloves/Keeper/glove_r.model", 8),
                ("Gloves/Keeper/glove_r.mtl", 1),
                ("Boots/Crocs/boots.model", 16),
                ("Boots/Crocs/boots.mtl", 1),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes17,
            fpc::HideIds::CONVENTION,
        );

        // Each marked player's boots and gloves under his exclusive id, a package for each
        // kind he holds a part of: slot 07 has no boots part, so his Crocs link is plain and
        // Crocs takes his exclusive id as its one wearer, and Keeper, which both combine,
        // takes no id.
        assert_eq!(
            summary(&report),
            [
                "0 714 Boots Players/05 - A [625] charge 5",
                "0 714 Gloves Players/05 - A [625] charge 12",
                "0 714 Gloves Players/07 - B [627] charge 12",
                "0 714 Boots Boots/Crocs [627] charge 17",
            ]
        );
        assert_eq!(
            message_summary(&report),
            [
                ("link_combined", "Players/05 - A", Disposition::Keep),
                ("link_combined", "Players/07 - B", Disposition::Keep),
            ]
        );
        // His `.mtl` is a file of both; Keeper's goes with Keeper's model, into the gloves.
        assert_eq!(
            task_files(&report.manifest.tasks[0]),
            [
                "Players/05 - A/kit_boots.model",
                "Players/05 - A/materials.mtl"
            ]
        );
        assert_eq!(
            task_files(&report.manifest.tasks[1]),
            [
                "Players/05 - A/materials.mtl",
                "Players/05 - A/x_gloveL.model",
                "Gloves/Keeper/glove_r.model",
                "Gloves/Keeper/glove_r.mtl",
            ]
        );
    }

    /// The PES 17 plan of slot 05 holding `ingame_face`, a boots and a gloves FMDL each with
    /// its skeleton, and linking `Gloves/Keeper/` (an FMDL with its skeleton), which his
    /// gloves part combines; and of slot 07 linking `Boots/Crocs/` (an FMDL with its
    /// skeleton) plainly. `metal_models` are the deep pass's metal models.
    fn converting_plan(metal_models: &[&str]) -> PlanReport {
        let export = resolved(
            "co Midcup Convert",
            &[
                ("Players/05 - A/ingame_face.toml", 0),
                ("Players/05 - A/boots.fmdl", 4),
                ("Players/05 - A/boots.skl", 1),
                ("Players/05 - A/x_gloveL.fmdl", 2),
                ("Players/05 - A/x_gloveL.skl", 1),
                ("Players/05 - A/Keeper.gloves", 0),
                ("Players/07 - B/Crocs.boots", 0),
                ("Gloves/Keeper/glove_r.fmdl", 8),
                ("Gloves/Keeper/glove_r.skl", 1),
                ("Boots/Crocs/boots.fmdl", 16),
                ("Boots/Crocs/boots.skl", 1),
            ],
            &[],
            None,
        );
        let mut planned = to_plan(ExportId(0), export, two_team_colors(), None);
        planned.metal_models = metal_models.iter().map(|path| scope_path(path)).collect();
        plan_run(vec![planned], PesVersion::Pes17, fpc::HideIds::CONVENTION)
    }

    #[test]
    fn a_pre_fox_boots_or_gloves_package_reads_the_skeleton_of_each_fmdl_it_may_convert() {
        let report = converting_plan(&[]);

        assert_eq!(
            summary(&report),
            [
                "0 714 Boots Players/05 - A [625] charge 6",
                "0 714 Gloves Players/05 - A [625] charge 13",
                "0 714 Face Players/07 - B [71407] charge 0",
                "0 714 Boots Boots/Crocs [627] charge 17",
            ]
        );
        // His own skeletons go into both his packages, as his `.mtl` files do, each of which
        // converts the FMDL one pairs with; a combined folder's into his package of its kind,
        // and a shared folder's into its own output.
        let tasks = &report.manifest.tasks;
        assert_eq!(
            task_files(&tasks[0]),
            [
                "Players/05 - A/boots.fmdl",
                "Players/05 - A/boots.skl",
                "Players/05 - A/x_gloveL.skl",
            ]
        );
        assert_eq!(
            task_files(&tasks[1]),
            [
                "Players/05 - A/boots.skl",
                "Players/05 - A/x_gloveL.fmdl",
                "Players/05 - A/x_gloveL.skl",
                "Gloves/Keeper/glove_r.fmdl",
                "Gloves/Keeper/glove_r.skl",
            ]
        );
        assert_eq!(
            task_files(&tasks[3]),
            ["Boots/Crocs/boots.fmdl", "Boots/Crocs/boots.skl"]
        );
    }

    #[test]
    fn a_marked_player_s_part_or_a_shared_folder_s_metal_fmdl_gets_the_template_in_its_home() {
        let owned = |package: &str| package.to_owned();
        // His gloves part and the shared boots: each folder gets the template, a textures task
        // planned for it with no texture of its own.
        let report = converting_plan(&["Players/05 - A/x_gloveL.fmdl", "Boots/Crocs/boots.fmdl"]);
        assert_eq!(
            environment_maps(&report),
            [
                (owned("Boots"), "Players/05 - A", true),
                (owned("Gloves"), "Players/05 - A", true),
                (owned("textures"), "Players/05 - A", true),
                (owned("Face"), "Players/07 - B", false),
                (owned("Boots"), "Boots/Crocs", true),
                (owned("textures"), "Boots/Crocs", true),
            ]
        );
        // A combined folder's metal FMDL is one his gloves convert.
        let report = converting_plan(&["Gloves/Keeper/glove_r.fmdl"]);
        assert_eq!(
            environment_maps(&report)[..3],
            [
                (owned("Boots"), "Players/05 - A", true),
                (owned("Gloves"), "Players/05 - A", true),
                (owned("textures"), "Players/05 - A", true),
            ]
        );
        // With none, no folder gets one.
        let report = converting_plan(&[]);
        assert!(
            environment_maps(&report)
                .iter()
                .all(|(package, _, environment_map)| package != "textures" && !environment_map),
            "{:?}",
            environment_maps(&report)
        );
    }

    #[test]
    fn a_pre_fox_marked_player_s_common_links_are_parts_read_with_the_mtl_their_search_finds() {
        let export = resolved(
            "co Midcup Common",
            &[
                ("Players/05 - A/ingame_face.toml", 0),
                // Two spellings of one link: one part, its files read once.
                ("Players/05 - A/kit_boots.model.common", 0),
                ("Players/05 - A/kit_boots.model.common.txt", 0),
                // A link whose `.mtl` is his own override of the Common one.
                ("Players/05 - A/glove_l.model.common", 0),
                ("Players/05 - A/glove_l.mtl", 1),
                // A model of his whose `.mtl` is a Common one, through two link spellings.
                ("Players/05 - A/glove_r.model", 2),
                ("Players/05 - A/glove_r.mtl.common", 0),
                ("Players/05 - A/glove_r.mtl.common.txt", 0),
                ("Common/kit_boots.model", 4),
                ("Common/kit_boots.mtl", 8),
                ("Common/glove_l.model", 16),
                ("Common/glove_l.mtl", 32),
                ("Common/glove_r.mtl", 64),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes17,
            fpc::HideIds::CONVENTION,
        );

        assert!(report.messages.is_empty(), "{:?}", report.messages);
        // Each package reads its parts and every `.mtl` of his, the Common ones his parts'
        // searches find included; the Common models task reads all of `Common/` again.
        assert_eq!(
            summary(&report),
            [
                "0 714 Boots Players/05 - A [625] charge 77",
                "0 714 Gloves Players/05 - A [625] charge 91",
                "0 714 common models Common (5) charge 124",
            ]
        );
        let tasks = &report.manifest.tasks;
        assert_eq!(
            task_files(&tasks[0]),
            [
                "Players/05 - A/glove_l.mtl",
                "Common/glove_r.mtl",
                "Common/kit_boots.model",
                "Common/kit_boots.mtl",
            ]
        );
        assert_eq!(
            task_files(&tasks[1]),
            [
                "Common/glove_l.model",
                "Players/05 - A/glove_l.mtl",
                "Players/05 - A/glove_r.model",
                "Common/glove_r.mtl",
                "Common/kit_boots.mtl",
            ]
        );
        let folder = models_folder(&tasks[0]);
        let links: Vec<(&str, &str, Option<&str>)> = folder
            .common_models
            .iter()
            .map(|common| {
                assert_eq!(common.skeleton, None, "a `.model` carries its own skeleton");
                (
                    common.link.as_str(),
                    common.model.path.as_str(),
                    common.material.as_ref().map(|file| file.path.as_str()),
                )
            })
            .collect();
        assert_eq!(
            links,
            [
                (
                    "Players/05 - A/glove_l.model.common",
                    "Common/glove_l.model",
                    Some("Players/05 - A/glove_l.mtl")
                ),
                (
                    "Players/05 - A/kit_boots.model.common",
                    "Common/kit_boots.model",
                    Some("Common/kit_boots.mtl")
                ),
                (
                    "Players/05 - A/kit_boots.model.common.txt",
                    "Common/kit_boots.model",
                    Some("Common/kit_boots.mtl")
                ),
            ]
        );
        // The Common model in the link's place under the link's role, each Common `.mtl`
        // once, before the `.mtl` links, which are kept for the search to see.
        let (_, _, roles) = folder.roles().swap_remove(0);
        let roles: Vec<(&str, PlayerFile)> = roles
            .into_iter()
            .map(|(file, role)| (file.path.as_str(), role))
            .collect();
        let part = |package, xml_type: &str| PlayerFile::PreFoxPart {
            package,
            xml_type: xml_type.to_owned(),
        };
        assert_eq!(
            roles,
            [
                ("Common/glove_l.model", part(ModelPackage::Gloves, "gloveL")),
                ("Players/05 - A/glove_l.mtl", PlayerFile::Material),
                (
                    "Players/05 - A/glove_r.model",
                    part(ModelPackage::Gloves, "gloveR")
                ),
                ("Common/glove_r.mtl", PlayerFile::Material),
                (
                    "Players/05 - A/glove_r.mtl.common",
                    PlayerFile::CommonMaterial
                ),
                (
                    "Players/05 - A/glove_r.mtl.common.txt",
                    PlayerFile::CommonMaterial
                ),
                ("Common/kit_boots.model", part(ModelPackage::Boots, "parts")),
                ("Common/kit_boots.mtl", PlayerFile::Material),
            ]
        );

        // On Fox a link's materials travel inside the FMDL: none is resolved.
        let fox = resolved(
            "co Midcup Fox",
            &[
                ("Players/05 - A/legs.fmdl.common", 0),
                ("Common/legs.fmdl", 4),
            ],
            &[],
            None,
        );
        let report = plan_run(
            vec![to_plan(ExportId(0), fox, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );
        let folder = models_folder(&report.manifest.tasks[0]);
        assert_eq!(folder.common_models.len(), 1);
        assert_eq!(folder.common_models[0].material, None);
    }

    #[test]
    fn a_fox_link_to_a_common_model_is_converted_with_its_common_mtl_unless_an_fmdl_beats_it() {
        let export = resolved(
            "co Midcup Fox",
            &[
                ("Players/05 - A/legs.model.common", 0),
                // An FMDL of its stem beats the linked `.model`: the link loads the FMDL.
                ("Players/05 - A/hat.model.common", 0),
                ("Common/legs.model", 1),
                ("Common/legs.mtl", 2),
                ("Common/hat.model", 4),
                ("Common/hat.fmdl", 8),
            ],
            &[],
            None,
        );

        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );

        assert!(report.messages.is_empty(), "{:?}", report.messages);
        let task = &report.manifest.tasks[0];
        let folder = models_folder(task);
        let links: Vec<(&str, &str, Option<&str>)> = folder
            .common_models
            .iter()
            .map(|common| {
                (
                    common.link.as_str(),
                    common.model.path.as_str(),
                    common.material.as_ref().map(|file| file.path.as_str()),
                )
            })
            .collect();
        assert_eq!(
            links,
            [
                ("Players/05 - A/hat.model.common", "Common/hat.fmdl", None),
                (
                    "Players/05 - A/legs.model.common",
                    "Common/legs.model",
                    Some("Common/legs.mtl")
                ),
            ]
        );
        // The face converts the Common `.model` with the Common `.mtl`, which follows it as a
        // `.mtl` of the player's, and packs the FMDL as it is.
        let (_, _, roles) = folder.roles().swap_remove(0);
        let roles: Vec<(&str, PlayerFile)> = roles
            .into_iter()
            .map(|(file, role)| (file.path.as_str(), role))
            .collect();
        let hair = || PlayerFile::CommonModel {
            package: ModelPackage::Face,
            name: "fcl_hair",
        };
        assert_eq!(
            roles,
            [
                ("Common/hat.fmdl", hair()),
                ("Common/legs.model", hair()),
                ("Common/legs.mtl", PlayerFile::Material),
            ]
        );
        assert_eq!(
            task_files(task),
            ["Common/hat.fmdl", "Common/legs.model", "Common/legs.mtl"]
        );
    }

    /// The PES 17 plan of an export whose `Common/` holds `legs.fmdl` with `legs.skl`, a
    /// `hat.fmdl` its `hat.model` (with `hat.mtl`) beats with `hat.skl`, a stray `stray.skl`
    /// and `extra`; slot 05 holding `ingame_face` links `legs.fmdl.common` and `hat.fmdl.common`,
    /// slot 06 `legs.fmdl.common`. `metal_models` are the deep pass's metal models.
    fn common_fmdl_plan(extra: &[(&str, u64)], metal_models: &[&str]) -> PlanReport {
        let files = [
            ("Players/05 - A/ingame_face.toml", 0),
            ("Players/05 - A/legs.fmdl.common", 0),
            ("Players/05 - A/hat.fmdl.common", 0),
            ("Players/06 - B/legs.fmdl.common", 0),
            ("Common/legs.fmdl", 1),
            ("Common/legs.skl", 2),
            ("Common/hat.fmdl", 4),
            ("Common/hat.skl", 8),
            ("Common/hat.model", 16),
            ("Common/hat.mtl", 32),
            ("Common/stray.skl", 64),
        ];
        let export = resolved(
            "co Midcup Common",
            &[files.as_slice(), extra].concat(),
            &[],
            None,
        );
        let mut planned = to_plan(ExportId(0), export, two_team_colors(), None);
        planned.metal_models = metal_models.iter().map(|path| scope_path(path)).collect();
        plan_run(vec![planned], PesVersion::Pes17, fpc::HideIds::CONVENTION)
    }

    /// The `environment_map` flag of the Common textures task in `report`, `None` when it has
    /// none.
    fn common_environment_map(report: &PlanReport) -> Option<bool> {
        report
            .manifest
            .tasks
            .iter()
            .find_map(|task| match &task.kind {
                TaskKind::CommonTextures {
                    environment_map, ..
                } => Some(*environment_map),
                TaskKind::Models { .. }
                | TaskKind::Textures { .. }
                | TaskKind::CommonModels { .. }
                | TaskKind::Portrait { .. }
                | TaskKind::Kit { .. }
                | TaskKind::Logo { .. }
                | TaskKind::RefereeMarker { .. }
                | TaskKind::Collar { .. } => None,
            })
    }

    #[test]
    fn a_pre_fox_common_fmdl_is_read_by_the_common_models_task_with_its_skeleton() {
        let report = common_fmdl_plan(&[], &[]);

        assert!(report.messages.is_empty(), "{:?}", report.messages);
        // The beaten FMDL, its skeleton and the stray one are read by nothing; no metal model,
        // so no Common textures task.
        let common = report
            .manifest
            .tasks
            .iter()
            .find(|task| matches!(task.kind, TaskKind::CommonModels { .. }))
            .unwrap();
        assert_eq!(
            task_files(common),
            [
                "Common/hat.model",
                "Common/hat.mtl",
                "Common/legs.fmdl",
                "Common/legs.skl",
            ]
        );
        assert_eq!(common_environment_map(&report), None);
        // His FMDL link brings in the Common FMDL with its skeleton, no `.mtl`; his link to the
        // beaten FMDL loads the `.model` with the `.mtl` its search finds.
        let boots = report
            .manifest
            .tasks
            .iter()
            .find(|task| {
                matches!(
                    task.kind,
                    TaskKind::Models {
                        package: ModelPackage::Boots,
                        ..
                    }
                )
            })
            .unwrap();
        let folder = models_folder(boots);
        let links: Vec<(&str, Option<&str>, Option<&str>)> = folder
            .common_models
            .iter()
            .map(|common| {
                (
                    common.model.path.as_str(),
                    common.skeleton.as_ref().map(|file| file.path.as_str()),
                    common.material.as_ref().map(|file| file.path.as_str()),
                )
            })
            .collect();
        assert_eq!(
            links,
            [
                ("Common/hat.model", None, Some("Common/hat.mtl")),
                ("Common/legs.fmdl", Some("Common/legs.skl"), None),
            ]
        );
        assert_eq!(
            task_files(boots),
            [
                "Common/hat.model",
                "Common/hat.mtl",
                "Common/legs.fmdl",
                "Common/legs.skl",
            ]
        );
    }

    #[test]
    fn a_metal_common_fmdl_plans_the_common_template_environment_map_unless_common_has_env() {
        // No texture in `Common/`: the task is planned for the template alone.
        let report = common_fmdl_plan(&[], &["Common/legs.fmdl"]);
        assert_eq!(common_environment_map(&report), Some(true));
        // A beaten FMDL converts nothing.
        let report = common_fmdl_plan(&[], &["Common/hat.fmdl"]);
        assert_eq!(common_environment_map(&report), None);
        // `Common/`'s own `env` texture, in any case, is the map.
        let report = common_fmdl_plan(&[("Common/ENV.png", 1)], &["Common/legs.fmdl"]);
        assert_eq!(common_environment_map(&report), Some(false));
        // On Fox, never.
        let export = resolved(
            "co Midcup Common",
            &[("Common/legs.fmdl", 1), ("Common/shirt.dds", 1)],
            &[],
            None,
        );
        let mut planned = to_plan(ExportId(0), export, two_team_colors(), None);
        planned.metal_models = [scope_path("Common/legs.fmdl")].into();
        let report = plan_run(vec![planned], PesVersion::Pes21, fpc::HideIds::CONVENTION);
        assert_eq!(common_environment_map(&report), Some(false));
    }

    #[test]
    fn a_subfolder_s_metal_fmdl_plans_the_template_environment_map_in_common_s_own_task() {
        // (folder, texture count, environment map) of each Common textures task, and the
        // folder of each Common models task, of the PES 17 plan of `files` with
        // `Common/sub/legs.fmdl` holding a metal material.
        let planned = |files: &[(&str, u64)]| {
            let export = resolved("co Midcup Common", files, &[], None);
            let mut planned = to_plan(ExportId(0), export, two_team_colors(), None);
            planned.metal_models = [scope_path("Common/sub/legs.fmdl")].into();
            let report = plan_run(vec![planned], PesVersion::Pes17, fpc::HideIds::CONVENTION);
            let mut textures = Vec::new();
            let mut models = Vec::new();
            for task in &report.manifest.tasks {
                match &task.kind {
                    TaskKind::CommonTextures {
                        folder,
                        textures: files,
                        environment_map,
                        ..
                    } => textures.push((folder.as_str().to_owned(), files.len(), *environment_map)),
                    TaskKind::CommonModels { folder, .. } => {
                        models.push(folder.as_str().to_owned())
                    }
                    TaskKind::Models { .. }
                    | TaskKind::Textures { .. }
                    | TaskKind::Portrait { .. }
                    | TaskKind::Kit { .. }
                    | TaskKind::Logo { .. }
                    | TaskKind::RefereeMarker { .. }
                    | TaskKind::Collar { .. } => {}
                }
            }
            (textures, models)
        };
        // `Common/` holds no texture: its task is planned for the map alone.
        assert_eq!(
            planned(&[("Common/hat.model", 1), ("Common/sub/legs.fmdl", 1)]),
            (
                vec![("Common".to_owned(), 0, true)],
                vec!["Common".to_owned(), "Common/sub".to_owned()]
            )
        );
        // Nor any file: still planned, for the map.
        assert_eq!(
            planned(&[("Common/sub/legs.fmdl", 1)]),
            (
                vec![("Common".to_owned(), 0, true)],
                vec!["Common/sub".to_owned()]
            )
        );
        // `Common/`'s own `env` texture is the map.
        assert_eq!(
            planned(&[("Common/env.dds", 1), ("Common/sub/legs.fmdl", 1)]),
            (
                vec![("Common".to_owned(), 1, false)],
                vec!["Common/sub".to_owned()]
            )
        );
    }

    #[test]
    fn a_common_gltf_with_no_model_of_its_stem_is_dropped_the_file_alone_on_either_engine() {
        // A glTF directly in `Common/` is selected as a player folder's is: target-native
        // first, then glTF, then the other engine's format. Slot 05 links `y.model`, slot 06
        // `z.fmdl`; slot 07 links `y.mtl`, a material link, which no glTF beats, so he stays.
        // On PES 15-17 a subfolder is selected within itself: `sub/y.glb` beats `sub/y.fmdl`
        // alone, and no link reaches it.
        let files = [
            ("Players/05 - A/y.model.common", 0),
            ("Players/06 - B/z.fmdl.common", 0),
            ("Players/07 - C/y.mtl.common", 0),
            ("Common/x.glb", 1),
            ("Common/y.glb", 1),
            ("Common/y.model", 1),
            ("Common/y.mtl", 1),
            ("Common/Z.glb", 1),
            ("Common/z.fmdl", 1),
            ("Common/shirt.dds", 1),
            ("Common/sub/y.glb", 1),
            ("Common/sub/y.fmdl", 1),
        ];
        // (version, the findings, the files of the pre-Fox Common models task)
        let cases: [(PesVersion, &[&str], &[&str]); 2] = [
            // `Z.glb` is beaten by `z.fmdl`, silent; `y.glb` beats `y.model`, which no link
            // then loads: slot 05 is dropped for the glTF his link's stem selects.
            (
                PesVersion::Pes21,
                &[
                    "model_gltf_unsupported DropFolder Players/05 - A Common/y.glb",
                    "model_gltf_unsupported DropFile Common/x.glb Common/x.glb",
                    "model_gltf_unsupported DropFile Common/y.glb Common/y.glb",
                ],
                &[],
            ),
            // The mirror: `y.glb` is beaten by `y.model`, packed; `Z.glb` beats `z.fmdl`, which
            // the task does not convert, and slot 06 is dropped.
            (
                PesVersion::Pes17,
                &[
                    "model_gltf_unsupported DropFolder Players/06 - B Common/Z.glb",
                    "model_gltf_unsupported DropFile Common/sub/y.glb Common/sub/y.glb",
                    "model_gltf_unsupported DropFile Common/x.glb Common/x.glb",
                    "model_gltf_unsupported DropFile Common/Z.glb Common/Z.glb",
                ],
                &["Common/y.model", "Common/y.mtl"],
            ),
        ];
        for (version, findings, common_models) in cases {
            let export = resolved("co Midcup Common", &files, &[], None);
            let report = plan_run(
                vec![to_plan(ExportId(0), export, two_team_colors(), None)],
                version,
                fpc::HideIds::CONVENTION,
            );
            let lines: Vec<String> = report
                .messages
                .iter()
                .map(|message| {
                    assert_eq!(message.severity, Severity::Error, "{version}");
                    let path = match &message.scope {
                        Scope::Folder { path, .. } | Scope::File { path, .. } => path.as_str(),
                        Scope::Run | Scope::Export { .. } | Scope::RosterEntry { .. } => {
                            panic!("{version}: {:?}", message.scope)
                        }
                    };
                    let [(key, file)] = message.context.as_slice() else {
                        panic!("{version}: {:?}", message.context);
                    };
                    assert_eq!(key, "file", "{version}");
                    format!(
                        "{} {:?} {path} {file}",
                        message.code.code, message.disposition
                    )
                })
                .collect();
            assert_eq!(lines, findings, "{version}");
            let read: Vec<&str> = report
                .manifest
                .tasks
                .iter()
                .filter_map(|task| {
                    if let TaskKind::CommonModels { files, .. } = &task.kind {
                        Some(files)
                    } else {
                        None
                    }
                })
                .flatten()
                .map(|file| file.path.as_str())
                .collect();
            assert_eq!(read, common_models, "{version}");
            // The export still compiles its Common textures.
            assert!(
                report
                    .manifest
                    .tasks
                    .iter()
                    .any(|task| matches!(task.kind, TaskKind::CommonTextures { .. })),
                "{version}"
            );
        }
    }

    #[test]
    fn each_common_directory_s_models_task_reads_its_own_files() {
        // Each directory is grouped on its own, `Common/` first; a `.skl` pairs a converted
        // FMDL of its own directory only, and a `.model` beats an FMDL of its stem there alone.
        let common: Vec<FileDescriptor> = [
            "Common/legs.model",
            "Common/legs.mtl",
            "Common/sub/hat.model",
            "Common/sub/hat.fmdl",
            "Common/sub/hat.mtl",
            "Common/sub/x.skl",
            "Common/sub/y.fmdl",
            "Common/sub/legs.fmdl",
            "Common/x.fmdl",
            "Common/x.skl",
            "Common/sub/deep/hat.fmdl",
        ]
        .iter()
        .map(|path| {
            let path = scope_path(path);
            FileDescriptor {
                size: 1,
                kind: aesthetics_export::classify(path.name()),
                source: path.clone(),
                path,
            }
        })
        .collect();
        let read: Vec<(String, Vec<String>)> = common_directories(&common)
            .into_iter()
            .map(|(directory, files)| {
                let read = common_model_files(&files)
                    .into_iter()
                    .map(|file| file.path.as_str().to_owned())
                    .collect();
                (directory.as_str().to_owned(), read)
            })
            .collect();
        let expected: Vec<(String, Vec<String>)> = [
            (
                "Common",
                &[
                    "Common/legs.model",
                    "Common/legs.mtl",
                    "Common/x.fmdl",
                    "Common/x.skl",
                ][..],
            ),
            (
                "Common/sub",
                &[
                    "Common/sub/hat.model",
                    "Common/sub/hat.mtl",
                    "Common/sub/y.fmdl",
                    "Common/sub/legs.fmdl",
                ][..],
            ),
            ("Common/sub/deep", &["Common/sub/deep/hat.fmdl"][..]),
        ]
        .iter()
        .map(|(directory, files)| {
            let files = files.iter().map(|file| (*file).to_owned()).collect();
            ((*directory).to_owned(), files)
        })
        .collect();
        assert_eq!(read, expected);
    }

    #[test]
    fn a_kit_token_folder_s_textures_join_the_directory_above_while_models_stay_their_own() {
        let export = resolved(
            "co Midcup Common",
            &[
                ("Common/hair.dds", 1),
                ("Common/kit1/a.dds", 2),
                ("Common/kit2/a.dds", 4),
                ("Common/jessie/b.dds", 8),
                ("Common/kit1/legs.model", 16),
            ],
            &[],
            None,
        );
        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes17,
            fpc::HideIds::CONVENTION,
        );
        // The textures tasks, each by folder with the paths of the textures it reads, and
        // the folders of the models tasks.
        let mut textures = Vec::new();
        let mut models = Vec::new();
        for task in &report.manifest.tasks {
            match &task.kind {
                TaskKind::CommonTextures {
                    folder,
                    textures: files,
                    ..
                } => textures.push((
                    folder.as_str().to_owned(),
                    files
                        .iter()
                        .map(|file| file.path.as_str().to_owned())
                        .collect::<Vec<_>>(),
                )),
                TaskKind::CommonModels { folder, .. } => {
                    models.push(folder.as_str().to_owned());
                }
                TaskKind::Models { .. }
                | TaskKind::Textures { .. }
                | TaskKind::Portrait { .. }
                | TaskKind::Kit { .. }
                | TaskKind::Logo { .. }
                | TaskKind::RefereeMarker { .. }
                | TaskKind::Collar { .. } => {}
            }
        }
        // `kit1/` and `kit2/` of one set complete in `Common/`'s task; `jessie/`'s keeps
        // its own. Models have no folder form: `kit1/`'s stays its own task's.
        assert_eq!(
            textures,
            [
                (
                    "Common".to_owned(),
                    vec![
                        "Common/hair.dds".to_owned(),
                        "Common/kit1/a.dds".to_owned(),
                        "Common/kit2/a.dds".to_owned()
                    ]
                ),
                (
                    "Common/jessie".to_owned(),
                    vec!["Common/jessie/b.dds".to_owned()]
                ),
            ]
        );
        assert_eq!(models, ["Common/kit1".to_owned()]);
    }

    #[test]
    fn on_fox_a_folder_s_common_files_are_the_direct_common_mtl_files() {
        // The export paths of `Players/03 - A`'s `common_files`, read from its face task, the
        // export planned for `version`.
        let common_files = |version: PesVersion| -> Vec<String> {
            let mut export = resolved(
                "co Midcup Common",
                &[
                    ("Players/03 - A/face_high.fmdl", 1),
                    ("Players/03 - A/face_diff.bin", 1),
                    ("Common/body.mtl", 1),
                    ("Common/skin.dds", 1),
                    ("Common/legs.fmdl", 1),
                    ("Common/legs.skl", 1),
                ],
                &[],
                None,
            );
            // Added after `Common/`'s files, as the structure pass would keep it.
            let nested = scope_path("Common/sub/x.mtl");
            export.export.common.push(FileDescriptor {
                size: 1,
                kind: aesthetics_export::classify(nested.name()),
                source: nested.clone(),
                path: nested,
            });
            let report = plan_run(
                vec![to_plan(ExportId(0), export, two_team_colors(), None)],
                version,
                fpc::HideIds::CONVENTION,
            );
            let folder = report
                .manifest
                .tasks
                .iter()
                .find_map(|task| match &task.kind {
                    TaskKind::Models { folder, .. } if folder.path.as_str() == "Players/03 - A" => {
                        Some(folder)
                    }
                    TaskKind::Models { .. }
                    | TaskKind::Textures { .. }
                    | TaskKind::CommonTextures { .. }
                    | TaskKind::CommonModels { .. }
                    | TaskKind::Portrait { .. }
                    | TaskKind::Kit { .. }
                    | TaskKind::Logo { .. }
                    | TaskKind::RefereeMarker { .. }
                    | TaskKind::Collar { .. } => None,
                })
                .expect("the player's face task");
            folder
                .common_files
                .iter()
                .map(|file| file.path.as_str().to_owned())
                .collect()
        };
        assert_eq!(common_files(PesVersion::Pes21), ["Common/body.mtl"]);
        // Pre-Fox: every model, `.mtl` and texture in `Common/`'s order, a subfolder's
        // included, which a member's `face.xml` may name; never a `.skl`.
        assert_eq!(
            common_files(PesVersion::Pes17),
            [
                "Common/body.mtl",
                "Common/legs.fmdl",
                "Common/skin.dds",
                "Common/sub/x.mtl"
            ]
        );
    }

    #[test]
    fn on_fox_a_folder_s_common_files_hold_the_mtl_a_subfolder_s_link_names() {
        let export = resolved(
            "co Midcup Common",
            &[
                ("Players/03 - A/face_high.model", 1),
                ("Players/03 - A/jessie/body.mtl.common", 0),
                ("Common/body.mtl", 1),
                ("Common/jessie/body.mtl", 1),
                ("Common/other/x.mtl", 1),
            ],
            &[],
            None,
        );
        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );
        let folder = report
            .manifest
            .tasks
            .iter()
            .find_map(|task| match &task.kind {
                TaskKind::Models { folder, .. } => Some(folder),
                TaskKind::Textures { .. }
                | TaskKind::CommonTextures { .. }
                | TaskKind::CommonModels { .. }
                | TaskKind::Portrait { .. }
                | TaskKind::Kit { .. }
                | TaskKind::Logo { .. }
                | TaskKind::RefereeMarker { .. }
                | TaskKind::Collar { .. } => None,
            })
            .expect("the player's face task");
        let paths: Vec<&str> = folder
            .common_files
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        assert_eq!(paths, ["Common/body.mtl", "Common/jessie/body.mtl"]);
    }

    fn scope_path(text: &str) -> ScopePath {
        ScopePath::new(text).unwrap()
    }

    /// The `/co/` export `co Midcup Hands` planned for `version`: slot 05 holds `body.fmdl`, a
    /// face part by its name, and slot 06 `boots.fmdl`, both of which the deep pass found
    /// weighted to hand bones.
    fn hand_weighted_plan(version: PesVersion) -> PlanReport {
        let export = resolved(
            "co Midcup Hands",
            &[
                ("Players/05 - A/body.fmdl", 3),
                ("Players/06 - B/boots.fmdl", 5),
            ],
            &[],
            None,
        );
        let mut planned = to_plan(ExportId(0), export, two_team_colors(), None);
        planned.hand_weighted = [
            scope_path("Players/05 - A/body.fmdl"),
            scope_path("Players/06 - B/boots.fmdl"),
        ]
        .into();
        plan_run(vec![planned], version, fpc::HideIds::CONVENTION)
    }

    /// The paths of the files `task` reads.
    fn task_files(task: &BuildTask) -> Vec<&str> {
        task.kind
            .files()
            .into_iter()
            .map(|file| file.path.as_str())
            .collect()
    }

    /// The model folder of the `Models` task `task`.
    fn models_folder(task: &BuildTask) -> &ModelFolder {
        let TaskKind::Models { folder, .. } = &task.kind else {
            panic!("a Models task");
        };
        folder
    }

    #[test]
    fn a_hand_weighted_face_part_gives_its_fox_folder_a_gloves_task_reading_it() {
        let report = hand_weighted_plan(PesVersion::Pes21);

        // Slot 05's gloves are planned under its exclusive ID, 625 (block 621 + 5 - 1), with
        // no glove-named file; slot 06's boots are not split, so it has no gloves.
        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/05 - A [71405] charge 3",
                "0 714 Gloves Players/05 - A [625] charge 3",
                "0 714 Face Players/06 - B [71406] charge 0",
                "0 714 Boots Players/06 - B [626] charge 5",
            ]
        );
        let tasks = &report.manifest.tasks;
        // Both the face and the gloves read the model; the face reads what it did before.
        assert_eq!(task_files(&tasks[0]), ["Players/05 - A/body.fmdl"]);
        assert_eq!(task_files(&tasks[1]), ["Players/05 - A/body.fmdl"]);
        let body: BTreeSet<ScopePath> = [scope_path("Players/05 - A/body.fmdl")].into();
        assert_eq!(models_folder(&tasks[1]).hand_split, body);
        assert_eq!(models_folder(&tasks[3]).hand_split, BTreeSet::new());
        // The player's gloves ID follows the gloves task, as an authored glove's.
        let gloves: Vec<&ItemRow> = report.manifest.patch_teams[0]
            .rows
            .iter()
            .filter(|row| row.table == ItemTable::Gloves)
            .collect();
        assert_eq!(
            gloves,
            [&ItemRow {
                table: ItemTable::Gloves,
                player_id: 71405,
                id: 625,
                task: Some(1),
            }]
        );
    }

    #[test]
    fn a_pre_fox_face_model_weighted_to_hand_bones_is_split_by_the_face_alone() {
        let export = resolved(
            "co Midcup Hands",
            &[
                // A face model of his own and one of the shared face he links: both split.
                ("Players/05 - A/body.model", 3),
                ("Players/05 - A/body.mtl", 1),
                ("Players/05 - A/Round.face", 0),
                ("Faces/Round/hair_high.model", 5),
                ("Faces/Round/hair_high.mtl", 1),
                // A Common model behind a link: the face lists it by reference, packs nothing.
                ("Players/06 - B/legs.model.common", 0),
                ("Common/legs.model", 7),
                ("Common/legs.mtl", 1),
                // Under `ingame_face` no model is face content.
                ("Players/07 - C/ingame_face.toml", 0),
                ("Players/07 - C/torso.model", 9),
                ("Players/07 - C/torso.mtl", 1),
                // Models named as gloves or boots, packed in his face beside nothing else.
                ("Players/08 - D/glove_l.model", 2),
                ("Players/08 - D/boots.model", 4),
                ("Players/08 - D/x.mtl", 1),
            ],
            &[],
            None,
        );
        let mut planned = to_plan(ExportId(0), export, two_team_colors(), None);
        // Every model weighted to hand bones; the link's own path too, so a link taken by its
        // role would be seen whichever path it were recorded under.
        planned.hand_weighted = [
            "Players/05 - A/body.model",
            "Faces/Round/hair_high.model",
            "Players/06 - B/legs.model.common",
            "Common/legs.model",
            "Players/07 - C/torso.model",
            "Players/08 - D/glove_l.model",
            "Players/08 - D/boots.model",
        ]
        .map(scope_path)
        .into();

        let report = plan_run(vec![planned], PesVersion::Pes17, fpc::HideIds::CONVENTION);

        // One face task per face, and no gloves task for the split: its hands are entries of
        // the face's own `face.xml`.
        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/05 - A [71405] charge 10",
                "0 714 Face Players/06 - B [71406] charge 0",
                "0 714 Boots Players/07 - C [627] charge 10",
                "0 714 Face Players/08 - D [71408] charge 7",
                "0 714 common models Common (2) charge 8",
            ]
        );
        let tasks = &report.manifest.tasks;
        assert_eq!(
            models_folder(&tasks[0]).hand_split,
            ["Players/05 - A/body.model", "Faces/Round/hair_high.model"]
                .map(scope_path)
                .into()
        );
        for task in &tasks[1..4] {
            let folder = models_folder(task);
            assert_eq!(
                folder.hand_split,
                BTreeSet::new(),
                "{}",
                folder.path.as_str()
            );
        }
        // The face reads what it did before the split: its models and their `.mtl` files.
        assert_eq!(
            task_files(&tasks[0]),
            [
                "Players/05 - A/body.model",
                "Players/05 - A/body.mtl",
                "Faces/Round/hair_high.model",
                "Faces/Round/hair_high.mtl",
            ]
        );
    }

    #[test]
    fn a_pre_fox_folder_holding_its_own_face_xml_splits_no_hand_weighted_model() {
        let export = resolved(
            "co Midcup Hands",
            &[
                ("Players/05 - A/body.model", 3),
                ("Players/05 - A/body.mtl", 1),
                ("Players/05 - A/face.xml", 1),
                // A face whose linked shared face holds the xml: his own model is not split
                // either.
                ("Players/07 - B/body.model", 3),
                ("Players/07 - B/body.mtl", 1),
                ("Players/07 - B/Round.face", 0),
                ("Faces/Round/face_high.model", 5),
                ("Faces/Round/face_high.mtl", 1),
                ("Faces/Round/face.xml", 1),
            ],
            &[],
            None,
        );
        let mut planned = to_plan(ExportId(0), export, two_team_colors(), None);
        planned.hand_weighted = [
            scope_path("Players/05 - A/body.model"),
            scope_path("Players/07 - B/body.model"),
            scope_path("Faces/Round/face_high.model"),
        ]
        .into();

        let report = plan_run(vec![planned], PesVersion::Pes17, fpc::HideIds::CONVENTION);

        let tasks = &report.manifest.tasks;
        assert_eq!(
            summary(&report),
            [
                "0 714 Face Players/05 - A [71405] charge 5",
                "0 714 Face Players/07 - B [71407] charge 11",
            ]
        );
        // The xml says what the face loads: the face task splits nothing, so it reports no
        // `model_hand_split`, and lists no glove entry the member did not write.
        assert_eq!(models_folder(&tasks[0]).hand_split, BTreeSet::new());
        assert_eq!(models_folder(&tasks[1]).hand_split, BTreeSet::new());
        // The face reads the xml beside the model and its `.mtl`.
        assert_eq!(
            task_files(&tasks[0]),
            [
                "Players/05 - A/body.model",
                "Players/05 - A/body.mtl",
                "Players/05 - A/face.xml",
            ]
        );
    }

    /// The `PlanReport` for `version` of slot 05 holding `boots.fmdl` with no texture, slot 06
    /// `boots.fmdl` beside `boots.model` and its `.mtl`, which the `.model` beats on PES
    /// 15-17, slot 07 `boots.fmdl` beside its own `face.xml`, and slot 08 linking `Faces/Round`,
    /// which holds `boots.fmdl` beside its `face.xml`; every FMDL is among the deep pass's metal
    /// models.
    fn metal_plan(version: PesVersion) -> PlanReport {
        let export = resolved(
            "co Midcup Metal",
            &[
                ("Players/05 - A/boots.fmdl", 3),
                ("Players/06 - B/boots.fmdl", 3),
                ("Players/06 - B/boots.model", 5),
                ("Players/06 - B/boots.mtl", 1),
                ("Players/07 - C/boots.fmdl", 3),
                ("Players/07 - C/face.xml", 1),
                ("Players/08 - D/Round.face", 0),
                ("Faces/Round/boots.fmdl", 3),
                ("Faces/Round/face.xml", 1),
            ],
            &[],
            None,
        );
        let mut planned = to_plan(ExportId(0), export, two_team_colors(), None);
        planned.metal_models = [
            scope_path("Players/05 - A/boots.fmdl"),
            scope_path("Players/06 - B/boots.fmdl"),
            scope_path("Players/07 - C/boots.fmdl"),
            scope_path("Faces/Round/boots.fmdl"),
        ]
        .into();
        plan_run(vec![planned], version, fpc::HideIds::CONVENTION)
    }

    /// Each `Models` and `Textures` task of `report` as its folder's path with whether the
    /// folder takes the template environment map.
    fn environment_maps(report: &PlanReport) -> Vec<(String, &str, bool)> {
        report
            .manifest
            .tasks
            .iter()
            .filter_map(|task| match &task.kind {
                TaskKind::Models {
                    folder, package, ..
                } => Some((
                    format!("{package:?}"),
                    folder.path.as_str(),
                    folder.environment_map,
                )),
                TaskKind::Textures { folder, .. } => Some((
                    "textures".to_owned(),
                    folder.path.as_str(),
                    folder.environment_map,
                )),
                TaskKind::CommonTextures { .. }
                | TaskKind::CommonModels { .. }
                | TaskKind::Portrait { .. }
                | TaskKind::Kit { .. }
                | TaskKind::Logo { .. }
                | TaskKind::RefereeMarker { .. }
                | TaskKind::Collar { .. } => None,
            })
            .collect()
    }

    #[test]
    fn a_folder_whose_pre_fox_face_converts_a_metal_fmdl_gets_a_textures_task_for_the_template() {
        let report = metal_plan(PesVersion::Pes17);

        // Slot 05's face converts its metal FMDL: its textures task is planned, with no
        // texture of its own, to emit the template. Slot 06's `.model` beats its FMDL, and
        // slot 07's own `face.xml` and slot 08's linked one list the face's models, so no
        // other face converts one.
        let owned = |package: &str| package.to_owned();
        assert_eq!(
            environment_maps(&report),
            [
                (owned("Face"), "Players/05 - A", true),
                (owned("textures"), "Players/05 - A", true),
                (owned("Face"), "Players/06 - B", false),
                (owned("Face"), "Players/07 - C", false),
                (owned("Face"), "Players/08 - D", false),
            ]
        );
        let tasks = &report.manifest.tasks;
        assert_eq!(task_files(&tasks[1]), Vec::<&str>::new());
        assert_eq!(tasks[0].group, tasks[1].group);
        assert!(tasks[1].group.is_some());

        // PES 21 converts no FMDL: the same models plan no environment map, nor a textures
        // task for one.
        let report = metal_plan(PesVersion::Pes21);
        assert!(
            environment_maps(&report)
                .iter()
                .all(|(package, _, environment_map)| package != "textures" && !environment_map),
            "{:?}",
            environment_maps(&report)
        );
    }

    #[test]
    fn an_env_texture_link_plans_no_textures_task_for_the_template() {
        let files = [
            ("Players/05 - A/boots.fmdl", 3),
            ("Players/05 - A/env.dds.common", 0),
            ("Common/env.dds", 4),
        ];
        let plan = |files: &[(&str, u64)]| {
            let export = resolved("co Midcup Metal", files, &[], None);
            let mut planned = to_plan(ExportId(0), export, two_team_colors(), None);
            planned.metal_models = [scope_path("Players/05 - A/boots.fmdl")].into();
            plan_run(vec![planned], PesVersion::Pes17, fpc::HideIds::CONVENTION)
        };

        // The link's Common texture is the map: a textures task would emit nothing.
        let report = plan(&files);
        assert_eq!(
            environment_maps(&report),
            [("Face".to_owned(), "Players/05 - A", true)]
        );
        assert_eq!(report.manifest.tasks[0].group, None);

        // A texture of his own still plans one, for that texture alone.
        let report = plan(&[&files[..], &[("Players/05 - A/hair.dds", 4)]].concat());
        let tasks = &report.manifest.tasks;
        assert_eq!(
            environment_maps(&report),
            [
                ("Face".to_owned(), "Players/05 - A", true),
                ("textures".to_owned(), "Players/05 - A", true),
            ]
        );
        assert_eq!(task_files(&tasks[1]), ["Players/05 - A/hair.dds"]);
    }
}
