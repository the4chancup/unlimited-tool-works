//! The deep pass (`spec/team_compiler/pipeline.md` "2. Per-export serial steps", "Deep format
//! pass"): the checks only a file's contents can answer, run over the sanitized export the
//! structure pass leaves. What they find goes back to `aesthetics_export` as content findings,
//! which derive the sanitized export again, so a finding drops, cascades and passes through by
//! the structure pass's own rules.
//!
//! It checks what `compile` reads and nothing else: a file planning leaves unread has no
//! finding, which would drop what `compile` builds without it. This module reads every native
//! model (`.fmdl`, `.model`) and every `.mtl` of the player folders, the shared folders and
//! `Common/` that a task of the target reads, whatever the target version, since a model of
//! either format is a source for either target. The roles are read as planning reads them, so
//! it skips:
//!
//! - a model another of its stem beats (a `Common/` one is dropped with its winner);
//! - a model with no role, and a per-kit variant left out;
//! - a model or `.mtl` a pre-Fox source's own `face.xml` does not name;
//! - for PES 2018 to 2021, a file below a `Common/` subfolder no model folder's link reaches,
//!   and a `.mtl` no `.model` is converted with, a model folder's or a `Common/` one;
//! - a refs export's kits, logo, portraits and collars.
//!
//! It reports what the format crates' checks find
//! (`spec/team_compiler/messages.md` "Model checks"): one finding per file and code, at the
//! severity the format crate gives it. An Error drops what holds the file and may pass
//! through; a Warning or an Info only informs. The far vertex, which both formats check, is
//! reported as `vertex_too_far_from_origin` and never passes through. A file that does not
//! parse is `model_broken` or `mtl_broken`.
//!
//! A `.model`, or a `.common` link to one, for which no `.mtl` is found (`mtl_search`) has
//! every material undefined, `model_material_undefined`, for PES 2015 to 2017. So has a model
//! folder's `.model` no `.fmdl` of its stem beats for PES 2018 to 2021. The folder is dropped,
//! `pass_through` or not. One whose `.mtl` lacks a material it binds is
//! `model_material_undefined` too, naming those materials, and may pass through. Which `.mtl`
//! each model pairs with, and that finding, live in `pairings`. glTF models are not read yet.
//!
//! It also checks every texture of those folders, of `Common/`, of the kits (but one the target
//! does not emit) and every portrait from its header alone: a file renamed from another
//! format, a side under one block, and the size rules
//! of a kit's main texture, of a mipmapped Fox texture and of a portrait. A slot whose two
//! portraits, its player folder's and its `Portraits/` file, differ in bytes is
//! `portrait_conflict`, which skips the export. A logo source is the one texture decoded in
//! full: one that does not decode is `logo_file_invalid`.
//!
//! Each file directly in `Collars/` (planning takes no other) must be named for a stock
//! collar of the target version that a kit config can name, and no collar the suite holds
//! itself (`collar`); a collar model is then checked as any model, its Errors dropping the
//! file.
//!
//! For PES 2015 to 2017 it also reads a face folder's own `face.xml` (`user_face_xml`), a
//! player's or a shared face folder's, and reports its content checks. The xml then decides
//! which `.mtl` a model's `model_material_undefined` compares with: the one its entry names,
//! for the models it lists (a `Common/` model an entry names included), instead of the one the
//! search finds. A player folder holding its own `face.xml` and linking a shared face folder
//! is `xml_shared_face_conflict`: a face takes one xml.
//!
//! Last, it reads the small data files whole (`documents`): the face diff of each player
//! folder and shared face folder (`face_diff_invalid`, `xml_dif_conflict`), but on PES 2015
//! to 2017 not a `face_diff.bin` beside a member's `face.xml` holding a `<dif>`, which
//! replaces it; each kit's `config.toml` (`kit_config_invalid`), each player's `settings.toml`
//! (`settings_toml_invalid`) and `ingame_face.toml` (`ingame_face_toml_invalid`), and each kit's and a team export's root `colors.txt`, one
//! `color_entry_invalid` per line the file refuses (a referee export's is read by nothing).
//!
//! The model folders and `Common/`'s files are checked in parallel on the caller's rayon
//! pool, each worker reading and holding one file at a time, and the findings are collected
//! in file order (`content_findings`). Each model it parses (`.fmdl`, `.model`) is also asked,
//! on the same parse, whether its vertices carry hand weights, which planning needs for the
//! hand auto-split.

pub(crate) mod collar;
mod documents;
mod materials;
mod model;
mod pairings;
mod portrait;
mod texture;

use std::collections::{BTreeMap, BTreeSet};

use aesthetics_export::{
    ContentFinding, Disposition, FileDescriptor, FileKind, IssueScope, KitTextureSource,
    ModelFormat, PlayerFolder, SharedKind, SharedModelFolder, ValidatedAestheticsExport,
    common_link_target, legacy_kit_tokens,
};
use dds_convert::SourceFormat;
use pes_savefile::settings_toml::PlayerSettings;
use pes_version::{Engine, PesVersion};
use rayon::prelude::*;
use vtree::ScopePath;

use crate::bins::{KIT_COLORS, TEAM_COLORS};
use crate::kit_variants::kit_lookup_keys;
use crate::messages::Code;
use crate::plan::EffectiveTeamKitFpc;
use crate::plan::legacy_ids::LegacyIds;
use crate::plan::roles::{
    FolderModels, PlayerFile, below_common, combined_folders, common_file, directory_stem,
    emits_kit_texture, file_stem, is_common_file, is_direct_root_folder_file, is_read_common_file,
    is_selected_common_model, is_user_face_xml, linked_common_files, part_source_models,
    player_file, role_position, texture_format,
};
use crate::reader::ContentSource;
use crate::templates;
use crate::user_face_xml::{
    self, Child, FaceFiles, UserFaceXml, XmlError, XmlFinding, names_file, reference, resolve,
};
use collar::collar_findings;
use documents::{
    colors_findings, face_diff_findings, kit_config_findings, legacy_ids, player_ingame_face,
    player_settings,
};
use materials::{TextureSources, held_common, held_textures, names_export_file, texture_findings};
use model::{MaterialRead, ModelKind, fired};
use pairings::{material_finding, pairings, searched_common_mtls, used_names};
use portrait::{folder_portrait, portrait_conflict, portrait_findings};
use texture::{SizeRule, texture_finding};

pub(crate) use documents::version_clamped;
pub(crate) use model::{FAR_VERTEX_CODES, Fired, summed};

/// What the deep pass found in one export (`content_findings`).
#[derive(Debug, Default, PartialEq)]
pub(crate) struct ContentPass {
    /// The content findings, in file order.
    pub(crate) findings: Vec<ContentFinding>,
    /// The export paths of the models it parsed (`.fmdl`, `.model`; a player folder's, a shared
    /// folder's or a `Common/` one) whose vertices carry a positive weight on a hand-skeleton
    /// bone, whatever their role and the target. Planning decides which of them the hand
    /// auto-split takes. A file that does not parse is not among them.
    pub(crate) hand_weighted: BTreeSet<ScopePath>,
    /// The export paths of the FMDLs it parsed that hold a material of the `metal` family
    /// (`model::ModelRead::metal`), whatever the target, as `hand_weighted` is recorded:
    /// planning gives a player folder whose pre-Fox face converts one of them the template
    /// environment map (`plan::ModelFolder::environment_map`), and no Fox folder converts
    /// one. A file that does not parse is not among them.
    pub(crate) metal_models: BTreeSet<ScopePath>,
    /// The export path of each pre-Fox `.model` and `.mtl` it parsed, with its materials
    /// (`model::ModelRead::materials`), which a model's `model_material_undefined` compares and
    /// a `.mtl`'s texture lookup reads. A file that does not parse is not among them.
    materials: BTreeMap<ScopePath, Vec<MaterialRead>>,
    /// The export paths of the FMDLs it dropped on a PES 15-17 run because every mesh of them
    /// is hidden (`model_hidden_dropped`), which a `.common` link naming one is dropped with
    /// (`hidden_link_findings`).
    hidden_models: BTreeSet<ScopePath>,
    /// Each player folder's parsed `settings.toml` joined with its `ingame_face.toml`'s
    /// keys, by folder path, which planning resolves the compiled players' patch entries from.
    /// A folder holding neither file is not among them; a file that does not parse
    /// (`settings_toml_invalid`, the file dropped; `ingame_face_toml_invalid`, kept but
    /// ignored) contributes nothing of its own.
    pub(crate) settings: BTreeMap<ScopePath, PlayerSettings>,
    /// The player folders whose `settings.toml` or `ingame_face.toml` was refused, by
    /// folder path: an invalid settings file holds back the player's whole look, so the
    /// patch's entries for his players hold no `[appearance]`.
    pub(crate) settings_refused: BTreeSet<ScopePath>,
    /// The team export's `legacy_ids.toml` parsed (`documents::legacy_ids`): the old boots
    /// and gloves IDs of a VGL27 export, which planning emits the affected slots' own
    /// packages under; `None` when the export has none or it did not parse
    /// (`legacy_ids_invalid`, the file dropped).
    pub(crate) legacy_ids: Option<LegacyIds>,
}

impl ContentPass {
    /// The pass of a file that gives `findings`, no weighted model and no material names.
    fn findings_only(findings: Vec<ContentFinding>) -> ContentPass {
        ContentPass {
            findings,
            ..ContentPass::default()
        }
    }

    /// `other`'s findings after this pass's, and its weighted models, metal models, material
    /// names and hidden models with this pass's.
    fn append(&mut self, other: ContentPass) {
        self.findings.extend(other.findings);
        self.hand_weighted.extend(other.hand_weighted);
        self.metal_models.extend(other.metal_models);
        self.materials.extend(other.materials);
        self.hidden_models.extend(other.hidden_models);
        self.settings.extend(other.settings);
        self.settings_refused.extend(other.settings_refused);
        self.legacy_ids = self.legacy_ids.take().or(other.legacy_ids);
    }
}

/// The `Common/` files the deep pass keeps, which a model's `.mtl` search looks among, with the
/// materials of those that are parsed pre-Fox models and material sets
/// (`ContentPass::materials`), and the textures the team's Common output will hold, which a
/// pre-Fox `.mtl`'s Common path may name (`materials::supply`).
#[derive(Debug, Default)]
struct KeptCommon {
    /// The kept files, in `Common/`'s order.
    files: Vec<FileDescriptor>,
    /// The materials of the kept pre-Fox models and material sets, by export path.
    materials: BTreeMap<ScopePath, Vec<MaterialRead>>,
    /// The stems, folded, of the kept textures, which the export's Common textures task packs,
    /// each kit set's member under its set's `kitN` reference key too (`kit_lookup_keys`).
    texture_stems: BTreeSet<String>,
    /// The paths below `Common/` without their extensions, folded (`jessie/skin`), of the
    /// kept textures, each kit set's member under its set's `kitN` reference key too
    /// (`kit_lookup_keys`): on pre-Fox every one is packed at that path, on Fox the subfolder ones
    /// a link reaches are, so a Common path into a `Common/` subfolder resolves among them
    /// (`materials::supply`), on both engines.
    texture_belows: BTreeSet<String>,
    /// The stems, folded, the installed CPKs loaded before the run's hold in the team's Common
    /// output, each kit set's member under its set's `kitN` reference key too; `None` when
    /// that lookup cannot be made or the export has no team ID.
    installed: Option<BTreeSet<String>>,
    /// For a refs export, the referee template tree's files in the referees' Common output
    /// (`templates::referee_common_paths`), which a `face.xml` Common reference may name too
    /// (`FaceFiles::template_common`); empty for a team export.
    template_paths: Vec<String>,
}

impl KeptCommon {
    /// The materials of the parsed pre-Fox file at `path`: a folder's, among
    /// `folder_materials`, or a kept `Common/` one's; `None` for a file that did not parse.
    fn materials_of<'a>(
        &'a self,
        path: &ScopePath,
        folder_materials: &'a BTreeMap<ScopePath, Vec<MaterialRead>>,
    ) -> Option<&'a Vec<MaterialRead>> {
        folder_materials
            .get(path)
            .or_else(|| self.materials.get(path))
    }
}

/// The content findings of `export`, the sanitized export read from `content` and compiled for
/// `version`, with the models among its files that carry hand weights and the FMDLs among
/// them holding a metal material (`ContentPass`). The
/// findings come in file order: each player folder's models, material sets and textures, then
/// its face diff, its portrait, its `settings.toml` and its `ingame_face.toml`; then each
/// shared folder's (faces with
/// their face diff, boots, gloves), then `Common/`'s, then each collar's (a file directly in
/// `Collars/`), then each `Portraits/` file with its slot's `portrait_conflict`, then each
/// kit's `config.toml`, `colors.txt` and textures, then the logo's, then the root
/// `colors.txt`'s; a refs export's collars, portraits, kits, logo and root `colors.txt` are
/// read by nothing and not checked. An Error on a folder's or a kit's file drops the folder;
/// one on a `Common/` file drops the file, and the cascade then drops the players linking it;
/// one on a collar, a portrait, a `settings.toml` or a logo file drops that file. A refused
/// `colors.txt` line is a Warning on the file, which drops nothing. `Common/`'s files are
/// checked before the folders, whose models' `.mtl` search sees only the ones the validation
/// report keeps, with `pass_through` as set, and whose models' materials are compared with
/// the kept ones' names (`KeptCommon`). A refs export's `face.xml` Common reference may also
/// name a file of the referee template tree's `common/999/`, which the refs CPK carries
/// (`templates::referee_common_paths`).
///
/// It checks only what `compile` reads: not for PES 2018 to 2021 a file below a `Common/`
/// subfolder that no model folder's link reaches (`is_read_common_file`, `linked_common_files`), nor a
/// `Common/` model another of its stem beats (`is_selected_common_model`), which is dropped
/// with that winner instead (`drop_beaten_common_models`), nor for PES 2018 to 2021 a
/// `Common/` `.mtl` no model folder's search may read (`searched_common_mtls`), nor a kit
/// texture the target does not emit (`emits_kit_texture`), nor a folder's file
/// `folder_findings` leaves unread, each player folder's roles read as planning reads them
/// (`part_source_models`).
///
/// For PES 2015 to 2017 each `.mtl`'s texture paths are looked up (`materials`), and for PES
/// 2018 to 2021 those of each folder `.mtl` a selected `.model` pairs with, right after its own
/// findings, and of each `Common/` `.mtl` a model folder's link to a Common `.model` pairs
/// with, on that folder against `Common/`'s textures and `installed` (`folder_findings`), each
/// finding
/// keeping what holds the `.mtl`: a folder's against the
/// textures the folder holds, `Common/`'s and `installed`, the stems the installed CPKs loaded
/// before the run's hold in the team's Common output (`None` when that lookup cannot be made or
/// the export has no team ID); a kept `Common/` `.mtl`'s against `Common/`'s textures and
/// `installed`, its mesh-used materials being those the kept `Common/` models bind.
///
/// The player folders, the shared folders, the files of each folder and `Common/`'s files are
/// checked in parallel, on the rayon pool the caller runs this in; the rest in order. Every
/// file is read through `content` and each worker holds one file's bytes at a time, so at most
/// one file per worker thread is held (a slot's two portraits while they are compared). A
/// solid `.7z` is decompressed once per `content`, on its first read (the caller's metadata
/// read, when the export has a metadata file), and stays charged until `content` is dropped.
pub(crate) fn content_findings(
    export: &ValidatedAestheticsExport,
    content: &ContentSource,
    version: PesVersion,
    installed: Option<BTreeSet<String>>,
    pass_through: bool,
) -> ContentPass {
    let size_rule = SizeRule::of(version);
    let engine = version.engine();
    let referees = export.team_name.is_referees();
    // Each player's sources and his roles as planning reads them: under his marker, and over
    // every source a part is built from, so a per-kit variant his package leaves out is not
    // read. Read before `Common/`, whose `.mtl` files on Fox only the players' searches read.
    let player_sources: Vec<(Vec<(SharedKind, &SharedModelFolder)>, FolderModels)> = export
        .players
        .iter()
        .map(|player| {
            // This pass is what finds the hand-weighted models, so it cannot see a gloves link
            // that combines only with split hands: that folder is not a source of his here,
            // though planning makes it one.
            let combined = combined_folders(export, player, engine, &BTreeSet::new());
            let (models, _) = part_source_models(
                &player.path,
                &player.files,
                None,
                player.ingame_face.is_some(),
                &combined,
                engine,
            );
            (combined, models)
        })
        .collect();
    // What PES 18-21 reads of a `Common/` subfolder: the files a model folder's link
    // reaches.
    let linked_common = linked_common_files(export);
    // On Fox a `Common/` `.mtl` is read only by the search of a model converted with it.
    let searched_mtls = match engine {
        Engine::Fox => {
            let models: Vec<&FolderModels> =
                player_sources.iter().map(|(_, models)| models).collect();
            Some(searched_common_mtls(export, &models))
        }
        Engine::PreFox => None,
    };
    // A name holding a legacy kit token is refused whatever the target reads, and what it
    // drops (the file, or every file below the subfolder) is neither checked nor kept.
    let common_names = common_kit_token_names(&export.common);
    let refused_by_name = |file: &FileDescriptor| {
        common_names.iter().any(|finding| match &finding.scope {
            IssueScope::File(path) | IssueScope::Folder(path) => {
                at_or_below(file.path.as_str(), path.as_str())
            }
            IssueScope::Export | IssueScope::RosterEntry { .. } => false,
        })
    };
    // Each group below is collected in its items' order (rayon's indexed `collect`), so the
    // findings come out in file order whatever the workers' scheduling.
    let mut common: Vec<ContentPass> = export
        .common
        .par_iter()
        .map(|file| {
            if refused_by_name(file) {
                return ContentPass::default();
            }
            // The Common tasks and the linking players read only the files the target reads
            // (`is_read_common_file`: on Fox `Common/` itself and the subfolder files a link
            // reaches), and a model there only when the target selects it for its stem, and on
            // Fox a `.mtl` only when a search finds it: nothing reads the others.
            let unread = !is_read_common_file(&file.path, engine, &linked_common)
                || (matches!(file.kind, FileKind::Model(_))
                    && !is_selected_common_model(&export.common, file, engine))
                || (file.kind == FileKind::Mtl
                    && searched_mtls
                        .as_ref()
                        .is_some_and(|searched| !searched.contains(&file.path)));
            let Some(checked) = checked_as(file, size_rule).filter(|_| !unread) else {
                return ContentPass::default();
            };
            file_outcome(
                content,
                file,
                checked,
                &IssueScope::File(file.path.clone()),
                Disposition::DropFile,
                file.path.name(),
                engine,
            )
        })
        .collect();
    drop_beaten_common_models(
        &export.common,
        &mut common,
        engine,
        &linked_common,
        pass_through,
    );
    // The `Common/` FMDLs dropped as hidden, which a player's link naming one is dropped with.
    let hidden_common: Vec<FileDescriptor> = export
        .common
        .iter()
        .zip(&common)
        .filter(|(file, pass)| pass.hidden_models.contains(&file.path))
        .map(|(file, _)| file.clone())
        .collect();
    // The refs CPK carries the referee template tree beside the export's own files.
    let template_paths = if referees {
        templates::referee_common_paths(engine)
    } else {
        Vec::new()
    };
    // A model's `.mtl` search looks only among the `Common/` files the report keeps: planning
    // sees the export after the drops, so the face task searches the same files, and a model
    // whose only `.mtl` is a dropped Common one is `model_material_undefined` here rather than
    // a material the face task cannot find. Under `pass_through` a file whose every Error is
    // eligible is kept and packed, so it is found here too: leaving it out would drop a
    // player for a file `compile` packs. On PES 15-17 a subfolder's file is kept too: a
    // member's `face.xml` names it, and the Common tasks read it; on PES 18-21 a subfolder's
    // file a link reaches is kept, which the linking player's tasks read.
    let mut kept_common = KeptCommon {
        installed,
        template_paths,
        ..KeptCommon::default()
    };
    for (file, pass) in export.common.iter().zip(&common) {
        if !is_read_common_file(&file.path, engine, &linked_common)
            || drops_file(&pass.findings, pass_through)
            || refused_by_name(file)
        {
            continue;
        }
        kept_common.files.push(file.clone());
        if let Some(read) = pass.materials.get(&file.path) {
            kept_common
                .materials
                .insert(file.path.clone(), read.clone());
        }
    }
    // A subfolder's textures are packed under its own path, which no player's paths name.
    // A kit set's member goes in under its set's `kitN` reference key beside its own
    // (`kit_lookup_keys`), a `kitF` member under that alone: a path naming its own spelling
    // finds nothing.
    kept_common.texture_stems = kept_common
        .files
        .iter()
        .filter(|file| file.kind == FileKind::Texture && is_direct_root_folder_file(&file.path))
        .flat_map(|file| kit_lookup_keys(file_stem(file.path.name())))
        .collect();
    kept_common.texture_belows = kept_common
        .files
        .iter()
        .filter(|file| file.kind == FileKind::Texture)
        .flat_map(|file| kit_lookup_keys(file_stem(below_common(&file.path))))
        .collect();
    match engine {
        Engine::Fox => {}
        Engine::PreFox => {
            common_mtl_findings(&export.common, &mut common, &kept_common);
            // The texture findings may drop a `Common/` `.mtl` (`mtl_common_path_invalid`):
            // it is no kept Common file for the players' `.mtl` search — remove it and its
            // materials, as the initial filter did, so the pairing finds `model_material_undefined`
            // rather than materials that do not exist.
            let dropped: BTreeSet<&ScopePath> = export
                .common
                .iter()
                .zip(&common)
                .filter(|(_, pass)| drops_file(&pass.findings, pass_through))
                .map(|(file, _)| &file.path)
                .collect();
            kept_common
                .files
                .retain(|file| !dropped.contains(&file.path));
            kept_common
                .materials
                .retain(|path, _| !dropped.contains(path));
        }
    }
    let players: Vec<ContentPass> = export
        .players
        .par_iter()
        .zip(player_sources.par_iter())
        .map(|(player, (combined, models))| {
            let folder = &player.path;
            // Under the marker the face files are not used, his own `face.xml` among them.
            let face = FaceUse::of_player(player, combined);
            let conflict = shared_face_conflict(folder, &player.files, &face, engine);
            let read = ReadFolder {
                path: folder,
                files: &player.files,
                models,
                combined,
                hidden_common: &hidden_common,
            };
            let (mut pass, xml_dif) = folder_findings(content, &read, &kept_common, version, face);
            let findings = &mut pass.findings;
            findings.extend(conflict);
            findings.extend(face_diff_findings(
                content,
                folder,
                &player.files,
                models,
                xml_dif,
            ));
            // Not among the folder's files: a portrait and a `settings.toml` are dropped
            // alone, the folder keeping the rest. A referee's portrait is read by nothing.
            if let Some(portrait) = &player.portrait
                && !referees
            {
                findings.extend(portrait_findings(
                    content,
                    portrait,
                    &relative(&portrait.path, folder),
                    engine,
                ));
            }
            // The player's settings are his `settings.toml`'s joined with his
            // `ingame_face.toml`'s keys: one file refused leaves the other's keys in
            // `settings` for his name and findings, and holds back from the patch
            // his whole look and every ID those keys would resolve.
            let mut settings_refused = false;
            let mut settings = match player_settings(content, player) {
                Ok(settings) => settings.unwrap_or_default(),
                Err(finding) => {
                    settings_refused = true;
                    findings.push(finding);
                    PlayerSettings::default()
                }
            };
            match player_ingame_face(content, player) {
                Ok(face) => settings.set_ingame_face(face.unwrap_or_default()),
                Err(finding) => {
                    settings_refused = true;
                    findings.push(finding);
                }
            }
            if player.settings.is_some() || player.ingame_face.is_some() {
                // The parser bounds a value to the key's widest range; the compile
                // target's own is narrower. Each value past it resets to the key's
                // default here, one finding apiece, so `check` and `compile` report
                // alike and planning sees the row's values.
                for reset in settings.narrow_to(version) {
                    findings.push(ContentFinding {
                        code: Code::SettingsValueOutOfRange.as_str(),
                        scope: IssueScope::Folder(player.path.clone()),
                        context: vec![
                            ("key", reset.key),
                            ("value", reset.value.to_string()),
                            ("range", reset.range),
                        ],
                        disposition: Disposition::Keep,
                        pass_through_eligible: false,
                    });
                }
                pass.settings.insert(player.path.clone(), settings);
            }
            if settings_refused {
                pass.settings_refused.insert(player.path.clone());
            }
            pass
        })
        .collect();
    // A shared face folder is part of the face of each player linking it, so its face diff is
    // checked as a player folder's is, against its own models as planning resolves it.
    let faces: Vec<ContentPass> = export
        .faces
        .par_iter()
        .map(|face| {
            let models = FolderModels::of_shared(&face.path, &face.files, SharedKind::Face, engine);
            // A shared face links no other shared folder.
            let read = ReadFolder {
                path: &face.path,
                files: &face.files,
                models: &models,
                combined: &[],
                hidden_common: &[],
            };
            let (mut pass, xml_dif) = folder_findings(
                content,
                &read,
                &kept_common,
                version,
                FaceUse::Used { linked_face: None },
            );
            pass.findings.extend(face_diff_findings(
                content,
                &face.path,
                &face.files,
                &models,
                xml_dif,
            ));
            pass
        })
        .collect();
    let boots = export
        .boots
        .par_iter()
        .map(|folder| (SharedKind::Boots, folder));
    let gloves = export
        .gloves
        .par_iter()
        .map(|folder| (SharedKind::Gloves, folder));
    let boots_and_gloves: Vec<ContentPass> = boots
        .chain(gloves)
        .map(|(kind, shared)| {
            let read = ReadFolder {
                path: &shared.path,
                files: &shared.files,
                models: &FolderModels::of_shared(&shared.path, &shared.files, kind, engine),
                combined: &[],
                hidden_common: &[],
            };
            // A boots or gloves folder has no face, so no xml.
            let (pass, _) = folder_findings(content, &read, &kept_common, version, FaceUse::Unused);
            pass
        })
        .collect();
    let mut pass = ContentPass::default();
    let common_names = vec![ContentPass::findings_only(common_names)];
    for group in [players, faces, boots_and_gloves, common_names, common] {
        for found in group {
            pass.append(found);
        }
    }
    // A referee has no kit slot, team record, logo or player id, so planning gives a refs
    // export's collars, portraits, kits, logo and root `colors.txt` no reader
    // (`file_not_used`): nothing below is read for it.
    if referees {
        return pass;
    }
    let findings = &mut pass.findings;
    // Planning takes a collar only directly in `Collars/`: nothing reads a subfolder's file.
    let collars = export
        .collars
        .iter()
        .filter(|file| is_direct_root_folder_file(&file.path));
    for file in collars {
        findings.extend(collar_findings(content, file, version));
    }
    for (slot, file) in &export.portraits {
        findings.extend(portrait_findings(content, file, file.path.name(), engine));
        if let Some(folder_portrait) = folder_portrait(export, *slot) {
            findings.extend(portrait_conflict(content, folder_portrait, file));
        }
    }
    // An `all/` texture is read once, however many kits inherit it, and its findings are kept
    // by its path; each inheriting kit gets them on its own scope.
    let mut inherited: BTreeMap<&str, Vec<ContentFinding>> = BTreeMap::new();
    let fpc = EffectiveTeamKitFpc::of(export);
    for kit in export.kits.kits.values() {
        findings.extend(kit_config_findings(content, kit, version, fpc));
        if let Some(colors) = &kit.colors {
            findings.extend(colors_findings(content, colors, KIT_COLORS));
        }
        let scope = IssueScope::Folder(kit.path.clone());
        // Planning drops a texture the target does not emit before the kit's task reads any.
        for texture in kit
            .textures
            .iter()
            .filter(|texture| emits_kit_texture(engine, &texture.stem))
        {
            let rule = if texture.stem == "kit" {
                match engine {
                    Engine::PreFox => SizeRule::MainKit,
                    Engine::Fox => size_rule,
                }
            } else {
                size_rule
            };
            let Some(checked) = checked_as(&texture.file, rule) else {
                continue;
            };
            match texture.source {
                KitTextureSource::Own => findings.extend(file_findings(
                    content,
                    &texture.file,
                    checked,
                    &scope,
                    Disposition::DropFolder,
                    &relative(&texture.file.path, &kit.path),
                    engine,
                )),
                KitTextureSource::Shared => {
                    // Named by its path in the export, so the member sees it is not in the
                    // kit's own folder.
                    let path = texture.file.path.as_str();
                    let found = inherited.entry(path).or_insert_with(|| {
                        file_findings(
                            content,
                            &texture.file,
                            checked,
                            &scope,
                            Disposition::DropFolder,
                            path,
                            engine,
                        )
                    });
                    findings.extend(found.iter().map(|finding| ContentFinding {
                        scope: scope.clone(),
                        ..finding.clone()
                    }));
                }
            }
        }
    }
    let logo_files = export
        .logo
        .iter()
        .flat_map(|logo| std::iter::once(&logo.main).chain(&logo.small));
    for logo in logo_files {
        let file = &logo.file;
        let format = texture_format(file.path.name())
            .expect("a logo is classified a texture by an extension `dds_convert` accepts");
        findings.extend(file_findings(
            content,
            file,
            Checked::Logo(format),
            &IssueScope::File(file.path.clone()),
            Disposition::DropFile,
            file.path.name(),
            engine,
        ));
    }
    if let Some(colors) = &export.root.team_colors {
        findings.extend(colors_findings(content, colors, TEAM_COLORS));
    }
    match legacy_ids(content, &export.root) {
        Ok(ids) => pass.legacy_ids = ids,
        Err(finding) => findings.push(finding),
    }
    pass
}

/// The structure pass's code for a root `logo*` file that cannot be the team's logo, which the
/// deep pass also reports for a logo source that does not decode.
const LOGO_FILE_INVALID: &str = "logo_file_invalid";

/// `pes_model`'s code for a model material with no entry in the `.mtl` the model uses, which
/// the deep pass reports for a `.model` whose `.mtl` (`mtl_search::mtl_for`) lacks a material
/// it binds, and for one no `.mtl` is found for: every one of its materials is undefined
/// (`material_finding`).
const MODEL_MATERIAL_UNDEFINED: &str = "model_material_undefined";

/// What the deep pass reads a file as, and what checks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Checked {
    /// A model or material set, checked by its format crate.
    Model(ModelKind),
    /// A texture in the format its extension names, checked from its header.
    Texture(SourceFormat, SizeRule),
    /// A logo source in the format its extension names, decoded in full: an export has at
    /// most two, and only the decoded image shows that the game's logo sizes can be made
    /// from it.
    Logo(SourceFormat),
}

/// What `file` is read as, a texture held to `size_rule`, or `None` for a file the deep pass
/// does not read.
fn checked_as(file: &FileDescriptor, size_rule: SizeRule) -> Option<Checked> {
    match file.kind {
        FileKind::Model(ModelFormat::Fmdl) => Some(Checked::Model(ModelKind::Fmdl)),
        FileKind::Model(ModelFormat::PesModel) => Some(Checked::Model(ModelKind::PreFoxModel)),
        FileKind::Mtl => Some(Checked::Model(ModelKind::Mtl)),
        FileKind::Texture => {
            let format = texture_format(file.path.name())
                .expect("a texture is classified by an extension `dds_convert` accepts");
            Some(Checked::Texture(format, size_rule))
        }
        // glTF is not read yet; nothing else is checked.
        FileKind::Model(ModelFormat::Gltf)
        | FileKind::Skl
        | FileKind::Fclo
        | FileKind::Xml
        | FileKind::MaterialsToml
        | FileKind::Bin
        | FileKind::SharedLink(_)
        | FileKind::CommonLink
        | FileKind::Marker(_)
        | FileKind::Metadata(_)
        | FileKind::Other => None,
    }
}

/// Whether `findings`, a `Common/` file's, drop it from the validation report: one drops the
/// file and `pass_through` cannot keep it, being off or the finding not eligible
/// (`spec/aesthetics_export/object_model.md` "Validation semantics").
fn drops_file(findings: &[ContentFinding], pass_through: bool) -> bool {
    findings.iter().any(|finding| {
        finding.disposition == Disposition::DropFile
            && !(pass_through && finding.pass_through_eligible)
    })
}

/// Appends `common_model_beaten_dropped` to the pass, among `passes`, of each model of
/// `Common/` a target of `engine` reads (`is_read_common_file`, `linked` the subfolder files a
/// link reaches), among `common`
/// (the export's `Common/` files, whose passes are `passes`), that the target does not select
/// for its stem (`is_selected_common_model`), when the pass dropped the model of its stem in
/// its directory that beats it (`drops_file`, with `pass_through`): the finding
/// names the file and that winner, and drops the file, never passing through. The deep pass
/// leaves a beaten model unread, so without the winner planning would select it for the stem
/// (`selected_common_model`) though nothing checked it.
fn drop_beaten_common_models(
    common: &[FileDescriptor],
    passes: &mut [ContentPass],
    engine: Engine,
    linked: &BTreeSet<ScopePath>,
    pass_through: bool,
) {
    let is_read_model = |file: &FileDescriptor| {
        matches!(file.kind, FileKind::Model(_)) && is_read_common_file(&file.path, engine, linked)
    };
    let dropped_winners: Vec<&FileDescriptor> = common
        .iter()
        .zip(passes.iter())
        .filter(|(file, pass)| is_read_model(file) && drops_file(&pass.findings, pass_through))
        .map(|(file, _)| file)
        .collect();
    for (file, pass) in common.iter().zip(passes) {
        if !is_read_model(file) || is_selected_common_model(common, file, engine) {
            continue;
        }
        // A model and the one beating it share a directory and a stem.
        let Some(winner) = dropped_winners
            .iter()
            .find(|winner| directory_stem(winner) == directory_stem(file))
        else {
            continue;
        };
        pass.findings.push(ContentFinding {
            code: Code::CommonModelBeatenDropped.as_str(),
            scope: IssueScope::File(file.path.clone()),
            context: vec![
                ("file", file.path.as_str().to_owned()),
                ("winner", winner.path.as_str().to_owned()),
            ],
            disposition: Disposition::DropFile,
            pass_through_eligible: false,
        });
    }
}

/// `model_hidden_dropped` on `file` of the player folder at `folder` when it is a `.common`
/// link, at any depth of the folder, whose target, the `Common/` file at the link's own path
/// (`common_link_target`) matched as validation matches it (`common_file`), is among `hidden`,
/// the `Common/` FMDLs the pass dropped as hidden: an Info on the link that drops the link alone, never passing
/// through, naming the link below the folder and the Common file by its export path. A dropped
/// link is no link, so the player is not taken down with its target (`link_target_dropped`),
/// and his folder and other files stand.
fn hidden_link_finding(
    folder: &ScopePath,
    file: &FileDescriptor,
    hidden: &[FileDescriptor],
) -> Option<ContentFinding> {
    if file.kind != FileKind::CommonLink {
        return None;
    }
    let linked = common_link_target(&file.path, folder)?;
    let model = common_file(hidden, &linked)?;
    Some(ContentFinding {
        code: Code::ModelHiddenDropped.as_str(),
        scope: IssueScope::File(file.path.clone()),
        context: vec![
            ("file", relative(&file.path, folder)),
            ("model", model.path.as_str().to_owned()),
        ],
        disposition: Disposition::DropFile,
        pass_through_eligible: false,
    })
}

/// The texture findings of each `.mtl` among `common`, the export's `Common/` files whose
/// passes are `passes`, that `kept` holds, a pre-Fox target's (`materials::texture_findings`):
/// each on its file, keeping it, appended to its pass after its own findings. A `Common/`
/// `.mtl` is packed once for the team, before any player's pairing is known, so its mesh-used
/// materials are those every kept `Common/` model's meshes bind, and its paths resolve as
/// its Common models task points them (`prefox_common::common_models`), `Common/` its model
/// folder: a name nearest first from its own directory up to `Common/` among the kept
/// Common textures, a path below its directory at that path alone (`held_common`), then
/// for a Common path the kept textures at the path it names below `Common/` and the
/// installed CPKs'.
fn common_mtl_findings(common: &[FileDescriptor], passes: &mut [ContentPass], kept: &KeptCommon) {
    let model_kind = FileKind::Model(ModelFormat::PesModel);
    let used: BTreeSet<&str> = kept
        .files
        .iter()
        .filter(|file| file.kind == model_kind)
        .filter_map(|file| kept.materials.get(&file.path))
        .flatten()
        .filter(|material| material.mesh_used)
        .map(|material| material.name.as_str())
        .collect();
    let held = held_common(&kept.files);
    for (file, pass) in common.iter().zip(passes) {
        if file.kind != FileKind::Mtl {
            continue;
        }
        // A file the pass dropped, or that did not parse, has no kept materials.
        let Some(read) = kept.materials.get(&file.path) else {
            continue;
        };
        let places = held.of(&file.path);
        let sources = TextureSources {
            held: &places,
            below: (&held, &file.path),
            common: &kept.texture_stems,
            common_below: &kept.texture_belows,
            installed: kept.installed.as_ref(),
        };
        pass.findings.extend(texture_findings(
            file.path.name(),
            read,
            &used,
            &sources,
            &IssueScope::File(file.path.clone()),
        ));
    }
}

/// Whether a model folder's face files are used (`folder_findings`).
enum FaceUse<'a> {
    /// The face files are not used: a boots or gloves folder, or a player under `ingame_face`.
    Unused,
    /// The face files are used: a player folder without `ingame_face`, or a shared face
    /// folder.
    Used {
        /// The shared face folder the folder links, whose files its own `face.xml` may name and
        /// whose textures its `.mtl` paths may name; `None` for a shared face folder and a
        /// player linking none.
        linked_face: Option<&'a SharedModelFolder>,
    },
}

impl<'a> FaceUse<'a> {
    /// How the face files of `player` are used, `combined` being the shared folders his
    /// package is built from (`combined_folders`).
    fn of_player(
        player: &PlayerFolder,
        combined: &[(SharedKind, &'a SharedModelFolder)],
    ) -> FaceUse<'a> {
        if player.ingame_face.is_some() {
            return FaceUse::Unused;
        }
        let linked_face = combined
            .iter()
            .find(|(kind, _)| matches!(kind, SharedKind::Face))
            .map(|(_, face)| *face);
        FaceUse::Used { linked_face }
    }
}

/// `xml_shared_face_conflict` on the player folder at `folder`, holding `files`, when on
/// pre-Fox it holds its own `face.xml` (`is_user_face_xml`) and its face links a shared face
/// folder (`face`), whether or not that folder holds one: the face's xml is the shared
/// folder's, or the one generated, and two xmls leave no rule for which one speaks. It names
/// his xml below his folder and the shared folder by its export path, and drops his folder,
/// never passing through: no choice of xml builds the face the member meant. Fox ignores
/// every `face.xml` (`xml_ignored_fox`).
fn shared_face_conflict(
    folder: &ScopePath,
    files: &[FileDescriptor],
    face: &FaceUse,
    engine: Engine,
) -> Option<ContentFinding> {
    match engine {
        Engine::Fox => return None,
        Engine::PreFox => {}
    }
    let FaceUse::Used {
        linked_face: Some(shared),
        ..
    } = face
    else {
        return None;
    };
    let xml = files.iter().find(|file| is_user_face_xml(folder, file))?;
    Some(ContentFinding {
        code: Code::XmlSharedFaceConflict.as_str(),
        scope: IssueScope::Folder(folder.clone()),
        context: vec![
            ("file", relative(&xml.path, folder)),
            ("link", shared.path.as_str().to_owned()),
        ],
        disposition: Disposition::DropFolder,
        pass_through_eligible: false,
    })
}

/// A model folder as `folder_findings` reads it: a player folder, or a shared face, boots or
/// gloves folder.
struct ReadFolder<'a> {
    /// The folder's export path, the scope its findings name.
    path: &'a ScopePath,
    /// Its own files.
    files: &'a [FileDescriptor],
    /// Its models as planning reads them (`part_source_models` for a player folder,
    /// `FolderModels::of_shared` for a shared one).
    models: &'a FolderModels,
    /// The shared folders a player folder's packages are built from (`combined_folders`);
    /// empty for a shared folder.
    combined: &'a [(SharedKind, &'a SharedModelFolder)],
    /// The `Common/` FMDLs the pass dropped as hidden, which a player folder's `.common` link
    /// naming one is dropped with (`hidden_link_finding`); empty for a shared folder: the
    /// drop is a pre-Fox one, where its links have no role.
    hidden_common: &'a [FileDescriptor],
}

/// The findings of the files of the model folder `read`: first `kit_token_old` for each name
/// in it holding a legacy kit token, every file's whether read or not (`kit_token_names`);
/// then those of the files the deep pass reads
/// (`checked_as`, textures held to `size_rule`): each on the folder's scope,
/// an Error dropping the folder, the file named below the folder (not a file with no role, such
/// as a model the target's own format beats, nor a per-kit model variant left out, nor on Fox
/// a `.mtl` no `.model` pairs with, nor on pre-Fox, where the folder's own `face.xml` controls
/// the face, a model or a `.mtl` it does not name (`xml_unnamed`), which nothing reads); for
/// each model `pairings` pairs (on
/// pre-Fox each `.model` with a role and each typed `.common` link to one, on Fox each
/// `.model` no FMDL beats and each `.common` link loading a Common `.model`),
/// `model_material_undefined` (`material_finding`, its `.mtl` searched among the folder's files
/// and `common`'s), right after the model's own findings; each read `.mtl`'s texture lookup
/// (`materials::texture_findings`), right after the `.mtl`'s own findings, and on Fox that of
/// a `Common/` `.mtl` a link pairs with, against `Common/`'s textures, right after the link's
/// (once per folder); and for a player folder's `.common` link whose target the pass dropped as
/// hidden, its `model_hidden_dropped` at its place (`hidden_link_finding`); with the models
/// among them that carry hand weights and the materials of the pre-Fox ones. The files are
/// read and checked in parallel, each worker holding one file, and
/// the models' materials compared after, from what each read kept.
///
/// When `face` says the folder's face files are used (`FaceUse::Used`), each member's own
/// `face.xml` among its files (`PlayerFile::FaceXml`) is read and checked (`user_xml_findings`),
/// its findings at its place in file order, and the xml overrides the search:
/// `model_material_undefined` compares only the models it lists with the `.mtl` each entry
/// names (`listed_materials`), a `Common/` model an entry names included, its finding after
/// every file's, naming it by its export path, and none at all when an xml has an Error,
/// which drops the folder; nor is any `.mtl`'s texture looked up then. The textures of the
/// shared folders the folder's packages are built from count for the folder's `.mtl` paths, as
/// they do for the task building them: the linked face's when the face files are used, and the
/// boots and gloves folders among its `combined` whether or not they are: under `ingame_face`
/// the marker leaves the face files unused, not the parts his boots and gloves are built from.
/// Returned with whether such an xml holds a `<dif>`, which the face writes in place of the
/// folder's `face_diff.bin` (`face_diff_findings`).
fn folder_findings(
    content: &ContentSource,
    read: &ReadFolder,
    common: &KeptCommon,
    version: PesVersion,
    face: FaceUse,
) -> (ContentPass, bool) {
    let ReadFolder {
        path: folder,
        files,
        models,
        combined,
        hidden_common,
    } = *read;
    let engine = version.engine();
    let size_rule = SizeRule::of(version);
    let scope = IssueScope::Folder(folder.clone());
    let xmls: Vec<(&FileDescriptor, XmlOutcome)> = match &face {
        FaceUse::Used { linked_face, .. } => {
            let face_files = FaceFiles {
                own: files,
                linked_face: *linked_face,
                common: &common.files,
                template_common: &common.template_paths,
                folder,
            };
            files
                .iter()
                .filter(|file| player_file(folder, file, models) == Some(PlayerFile::FaceXml))
                .map(|file| {
                    let outcome = user_xml_findings(content, file, &face_files, version, &scope);
                    (file, outcome)
                })
                .collect()
        }
        FaceUse::Unused => Vec::new(),
    };
    let xml_drops_folder = xmls.iter().any(|(_, outcome)| {
        outcome.parsed.is_none()
            || outcome
                .findings
                .iter()
                .any(|finding| finding.disposition == Disposition::DropFolder)
    });
    // With an xml, the models it lists and the `.mtl` each entry names, none when an xml
    // drops the folder; `None` when there is no xml, and the search pairs each model.
    let listed = (!xmls.is_empty()).then(|| {
        if xml_drops_folder {
            Vec::new()
        } else {
            listed_materials(&xmls, files, common, folder)
        }
    });
    // The face of a folder holding its own `face.xml` packs only what its entries name.
    let xml_controls = !xmls.is_empty() && !xml_drops_folder;
    let pairings = pairings(folder, files, models, engine, listed, common);
    // Nothing reads a file with no role (a model the target's own format beats,
    // `FolderModels::beaten`; a file the allowlist does not admit, which the structure pass
    // names) or a per-kit model variant left out. On Fox a `.mtl` is read only by the
    // conversion of a `.model` paired with it, so nothing reads one no such model pairs with
    // (beside only FMDLs, or a `.model` an FMDL beats). On pre-Fox, where the folder's own
    // `face.xml` controls the face, nothing reads a model or a `.mtl` it does not name
    // (`xml_unnamed`). None of these is checked: its findings would drop a folder `compile`
    // builds without it.
    let unread = |file: &FileDescriptor| {
        matches!(
            player_file(folder, file, models),
            None | Some(PlayerFile::LeftOutKitVariant)
        ) || match engine {
            Engine::Fox => {
                file.kind == FileKind::Mtl
                    && !pairings
                        .iter()
                        .any(|pairing| pairing.mtl.is_some_and(|mtl| mtl.path == file.path))
            }
            Engine::PreFox => xml_controls && xml_unnamed(folder, file, &xmls),
        }
    };
    // Collected in file order (an indexed `collect`), whatever the scheduling.
    let mut per_file: Vec<ContentPass> = files
        .par_iter()
        .map(|file| match checked_as(file, size_rule) {
            Some(checked) if !unread(file) => file_outcome(
                content,
                file,
                checked,
                &scope,
                Disposition::DropFolder,
                &relative(&file.path, folder),
                engine,
            ),
            Some(_) | None => ContentPass::default(),
        })
        .collect();
    let mut materials = BTreeMap::new();
    for found in &mut per_file {
        materials.append(&mut found.materials);
    }
    let linked_face = match &face {
        FaceUse::Used { linked_face } => *linked_face,
        FaceUse::Unused => None,
    };
    // A face link is in `combined` too; it counts only through `linked_face`, when the face
    // files are used.
    let shared: Vec<(SharedKind, &SharedModelFolder)> = linked_face
        .map(|face| (SharedKind::Face, face))
        .into_iter()
        .chain(combined.iter().copied().filter(|(kind, _)| match kind {
            SharedKind::Face => false,
            SharedKind::Boots | SharedKind::Gloves => true,
        }))
        .collect();
    let held = match engine {
        // A folder an xml Error drops gets no texture finding: the member fixes the xml first,
        // and its entries may name other `.mtl` files.
        Engine::PreFox if xml_drops_folder => None,
        // The `.mtl` checked is the member's source, the same whatever the target, so on Fox
        // the one a selected `.model` pairs with, the only one the pass reads there, is looked
        // up as on pre-Fox.
        Engine::Fox | Engine::PreFox => Some(held_textures(folder, files, models, &shared, engine)),
    };
    // A Common part's texture paths resolve in `Common/` (`processing::model`), as a
    // `Common/` `.mtl`'s do (`common_mtl_findings`): the kept Common textures, `Common/` the
    // model folder, a name nearest first from the `.mtl`'s own directory up to it, a path
    // below that directory at that path alone. The folder's texture links stand for the
    // `Common/` textures they name in both places, as the Models task's `common_folders`
    // reads them (`processing::model`), one the export lacks being an installed CPK's.
    let mut common_held = held_common(&common.files);
    // The kept Common textures by their paths below `Common/` without their extensions,
    // which a Common path into a `Common/` subfolder resolves at on both engines.
    let mut common_below = common.texture_belows.clone();
    for file in files {
        if let Some(PlayerFile::CommonTexture(below)) = player_file(folder, file, models) {
            common_held.insert_link(&below);
            common_below.extend(kit_lookup_keys(&below));
        }
    }
    // The combined folders' texture links too, standing at their paths as the Models
    // task's `linked_stems` reads every source's (`processing::model`).
    for (kind, source) in combined {
        let source_models = FolderModels::of_shared(&source.path, &source.files, *kind, engine);
        for file in &source.files {
            if let Some(PlayerFile::CommonTexture(below)) =
                player_file(&source.path, file, &source_models)
            {
                common_held.insert_link(&below);
                common_below.extend(kit_lookup_keys(&below));
            }
        }
    }
    // The Common `.mtl` files looked up for this folder so far: two links finding one are one
    // lookup.
    let mut looked_up: BTreeSet<&ScopePath> = BTreeSet::new();
    let mut pass = ContentPass::findings_only(kit_token_names(folder, files));
    for (file, found) in files.iter().zip(per_file) {
        pass.append(found);
        pass.findings
            .extend(hidden_link_finding(folder, file, hidden_common));
        if let Some(held) = &held
            && file.kind == FileKind::Mtl
            && let Some(read) = materials.get(&file.path)
        {
            let used = used_names(&pairings, &file.path, &materials, common);
            // A `.mtl` a Common `.model` pairs with on Fox: the task resolves its stems in
            // the folder, where the `.mtl` set them, then in `Common/`'s nearest first
            // from the model's own directory (`processing::model`'s `PartTextures::
            // CommonSet`). On pre-Fox the `.mtl` is packed in the face and resolves in
            // the folder alone (`prefox_face::face`).
            let common_model = match engine {
                Engine::Fox => {
                    pairings
                        .iter()
                        .find_map(|pairing| match (pairing.mtl, pairing.model) {
                            (Some(mtl), Some(model))
                                if mtl.path == file.path && is_common_file(model) =>
                            {
                                Some(model)
                            }
                            _ => None,
                        })
                }
                Engine::PreFox => None,
            };
            let mut places = held.of(&file.path);
            if let Some(model) = common_model {
                places.extend(common_held.of(model));
            }
            let sources = TextureSources {
                held: &places,
                below: (held, &file.path),
                common: &common.texture_stems,
                common_below: &common_below,
                installed: common.installed.as_ref(),
            };
            pass.findings.extend(texture_findings(
                &relative(&file.path, folder),
                read,
                &used,
                &sources,
                &scope,
            ));
        }
        if let Some((_, outcome)) = xmls.iter().find(|(xml, _)| xml.path == file.path) {
            pass.findings.extend(outcome.findings.iter().cloned());
        }
        for pairing in pairings
            .iter()
            .filter(|pairing| pairing.file.path == file.path)
        {
            pass.findings.extend(material_finding(
                pairing, folder, common, &materials, &scope,
            ));
            let common_mtl = match engine {
                // Fox has no Common model output: each linking player's Models task converts
                // the Common `.model` with this `.mtl`, so its lookup runs on his folder's
                // pass, right after the link's own finding.
                Engine::Fox => pairing.mtl.filter(|mtl| is_common_file(&mtl.path)),
                // Pre-Fox packs a Common `.mtl` once for the team and looks it up on its own
                // file (`common_mtl_findings`).
                Engine::PreFox => None,
            };
            if let Some(mtl) = common_mtl
                && looked_up.insert(&mtl.path)
                && let Some(read) = common.materials.get(&mtl.path)
            {
                let used = used_names(&pairings, &mtl.path, &materials, common);
                let places = common_held.of(&mtl.path);
                let sources = TextureSources {
                    held: &places,
                    below: (&common_held, &mtl.path),
                    common: &common.texture_stems,
                    common_below: &common_below,
                    installed: common.installed.as_ref(),
                };
                pass.findings.extend(texture_findings(
                    mtl.path.as_str(),
                    read,
                    &used,
                    &sources,
                    &scope,
                ));
            }
        }
    }
    // An xml entry may name a `Common/` model, which is no file of the folder's.
    for pairing in pairings
        .iter()
        .filter(|pairing| !files.iter().any(|file| file.path == pairing.file.path))
    {
        pass.findings.extend(material_finding(
            pairing, folder, common, &materials, &scope,
        ));
    }
    let xml_dif = xmls.iter().any(|(_, outcome)| {
        outcome.parsed.as_ref().is_some_and(|xml| {
            xml.children
                .iter()
                .any(|child| matches!(child, Child::Dif(_)))
        })
    });
    pass.materials = materials;
    (pass, xml_dif)
}

/// What reading a member's own `face.xml` gave (`user_xml_findings`).
struct XmlOutcome {
    /// Its findings, each on the folder.
    findings: Vec<ContentFinding>,
    /// The xml, when it was read and parsed.
    parsed: Option<UserFaceXml>,
}

/// The findings of the member's own `face.xml` `file` among `files`, for the target
/// `version`, each on `scope` and never pass-through-eligible (a malformed xml can crash the
/// game): when it cannot be read, `source_read_failed`; when it does not parse, one finding
/// dropping the folder, `xml_broken` (not UTF-8, not well-formed XML), `xml_root_tag_invalid`
/// (its root is not `<config>`) or `face_diff_invalid` (its `<dif>` is no face diff the game
/// reads), naming the file and the reason; else `user_face_xml::check`'s, then, in a player
/// folder's `face.xml` (`kit_token_checked`), `kit_token_old` for each path holding a legacy
/// kit token (`xml_legacy_kit_paths`), with the xml.
fn user_xml_findings(
    content: &ContentSource,
    file: &FileDescriptor,
    files: &FaceFiles,
    version: PesVersion,
    scope: &IssueScope,
) -> XmlOutcome {
    let finding = |(code, disposition, context): XmlFinding| ContentFinding {
        code: code.as_str(),
        scope: scope.clone(),
        context,
        disposition,
        pass_through_eligible: false,
    };
    let unparsed = |findings| XmlOutcome {
        findings,
        parsed: None,
    };
    let bytes = match read(content, file, scope, Disposition::DropFolder) {
        Ok(bytes) => bytes,
        Err(unread) => return unparsed(vec![unread]),
    };
    let name = relative(&file.path, files.folder);
    let xml = match user_face_xml::parse(&bytes) {
        Ok(xml) => xml,
        Err(error) => {
            let (code, context) = match &error {
                XmlError::Utf8 | XmlError::Xml { .. } => {
                    (Code::XmlBroken, ("error", error.to_string()))
                }
                XmlError::Root(root) => (Code::XmlRootTagInvalid, ("root", root.clone())),
                XmlError::Dif(reason) => (Code::FaceDiffInvalid, ("reason", reason.to_string())),
            };
            let context = vec![("file", name), context];
            return unparsed(vec![finding((code, Disposition::DropFolder, context))]);
        }
    };
    let mut findings: Vec<ContentFinding> = user_face_xml::check(&xml, &name, files, version)
        .into_iter()
        .map(finding)
        .collect();
    let legacy_kit_paths = if kit_token_checked(&file.path) {
        xml_legacy_kit_paths(&xml)
    } else {
        Vec::new()
    };
    findings.extend(
        legacy_kit_paths
            .into_iter()
            .map(|reference| ContentFinding {
                code: Code::KitTokenOld.as_str(),
                scope: scope.clone(),
                context: vec![("file", name.clone()), ("reference", reference.to_owned())],
                disposition: Disposition::DropFolder,
                pass_through_eligible: false,
            }),
    );
    XmlOutcome {
        findings,
        parsed: Some(xml),
    }
}

/// The `path` and `material` values of `xml`'s `<model>` entries naming a file of the export
/// (`materials::names_export_file`) and holding a legacy kit token
/// (`aesthetics_export::legacy_kit_tokens`), as written, each distinct value once in
/// source order: the paths `kit_token_old` refuses in a member's own `face.xml`.
fn xml_legacy_kit_paths(xml: &UserFaceXml) -> Vec<&str> {
    let mut paths: Vec<&str> = Vec::new();
    for child in &xml.children {
        let Child::Model(model) = child else {
            continue;
        };
        for value in [model.attribute("path"), model.attribute("material")]
            .into_iter()
            .flatten()
        {
            if names_export_file(value)
                && !legacy_kit_tokens(value).is_empty()
                && !paths.contains(&value)
            {
                paths.push(value);
            }
        }
    }
    paths
}

/// Whether `file`, of the model folder at `folder` whose own
/// `face.xml` files are `xmls`, is a model or a `.mtl` the face those xmls control does not
/// pack: an FMDL, which such a face never converts (a reference resolves a `.model` alone,
/// `xml_model_not_found` otherwise), a file where it takes no role (`role_position`), or one
/// no entry names, by its `path` for a
/// `.model` and its `material` for a `.mtl`, a `kitN` name naming every member of its set
/// (`user_face_xml::names_file`, the rule of `xml_model_unlisted` and of the face task).
/// False for any other file.
fn xml_unnamed(
    folder: &ScopePath,
    file: &FileDescriptor,
    xmls: &[(&FileDescriptor, XmlOutcome)],
) -> bool {
    let attribute = match file.kind {
        FileKind::Model(ModelFormat::Fmdl) => return true,
        FileKind::Model(ModelFormat::PesModel) => "path",
        FileKind::Mtl => "material",
        FileKind::Model(ModelFormat::Gltf)
        | FileKind::Texture
        | FileKind::Skl
        | FileKind::Fclo
        | FileKind::Xml
        | FileKind::MaterialsToml
        | FileKind::Bin
        | FileKind::SharedLink(_)
        | FileKind::CommonLink
        | FileKind::Marker(_)
        | FileKind::Metadata(_)
        | FileKind::Other => return false,
    };
    let named = xmls
        .iter()
        .filter_map(|(_, outcome)| outcome.parsed.as_ref())
        .flat_map(|xml| &xml.children)
        .filter_map(|child| match child {
            Child::Model(model) => model.attribute(attribute),
            Child::Dif(_) | Child::Other(_) => None,
        })
        .any(|value| names_file(value, &relative(&file.path, folder)));
    !(named && role_position(folder, file))
}

/// The `.model` files among `files`, those of the model folder at `folder`, that the
/// folder's own
/// `face.xml` files list, each with the `.mtl` its entry names (`user_face_xml::resolve`),
/// for `model_material_undefined` to compare: the xml overrides the search. An entry naming no
/// `material`, or one the compiler cannot resolve, is compared with nothing, and a `material`
/// naming no file is `xml_model_not_found` already. The caller asks only when no xml drops the
/// folder.
fn listed_materials<'a>(
    xmls: &[(&FileDescriptor, XmlOutcome)],
    files: &'a [FileDescriptor],
    common: &'a KeptCommon,
    folder: &'a ScopePath,
) -> Vec<(&'a FileDescriptor, &'a FileDescriptor)> {
    // The shared face's files are not the folder's own: a model there is checked in that
    // folder's pass.
    let face_files = FaceFiles {
        own: files,
        linked_face: None,
        common: &common.files,
        template_common: &common.template_paths,
        folder,
    };
    let model_kind = FileKind::Model(ModelFormat::PesModel);
    xmls.iter()
        .filter_map(|(_, outcome)| outcome.parsed.as_ref())
        .flat_map(|xml| &xml.children)
        .filter_map(|child| match child {
            Child::Model(model) => Some(model),
            Child::Dif(_) | Child::Other(_) => None,
        })
        .filter_map(|model| {
            let path = reference(model.attribute("path")?);
            let listed = resolve(&path, &face_files, model_kind)?;
            let material = reference(model.attribute("material")?);
            let mtl = resolve(&material, &face_files, FileKind::Mtl)?;
            Some((listed, mtl))
        })
        .collect()
}

/// The bytes of `file`, or, when they cannot be read, its `source_read_failed` on `scope`
/// with `disposition`, never eligible: there is nothing to keep.
fn read(
    content: &ContentSource,
    file: &FileDescriptor,
    scope: &IssueScope,
    disposition: Disposition,
) -> Result<Vec<u8>, ContentFinding> {
    content
        .read(file.source.as_str())
        .map_err(|failure| ContentFinding {
            code: Code::SourceReadFailed.as_str(),
            scope: scope.clone(),
            context: vec![("path", failure.path), ("error", failure.error)],
            disposition,
            pass_through_eligible: false,
        })
}

/// Whether the Export upgrader respells a legacy kit token in the file at export path `path`,
/// in its name and in the paths it names: it does for a file below a player folder
/// (`Players/<folder>/`), a Boots or Gloves folder (`Boots/<folder>/`, `Gloves/<folder>/`)
/// or in `Common/`, and for no other (a shared face folder's, a collar's). `kit_token_old`
/// checks exactly these files: refusing a token anywhere else would send the member to an
/// upgrade that leaves it as written. The upgrader's half of this rule is its
/// `respell_outputs`. Of the paths a file names, the upgrader respells a model's and a
/// `.mtl`'s in these folders, and a `face.xml`'s only in a player folder's own `face.xml`,
/// the one place the compiler reads one. The root folders are matched in any letter case, as
/// the export's content folders are.
fn kit_token_checked(path: &ScopePath) -> bool {
    let mut segments = path.segments();
    let root = segments.next().unwrap_or_default();
    let is = |name: &str| root.eq_ignore_ascii_case(name);
    // How many segments below the root the file needs: a model folder's own name is not
    // respelled, so a player, Boots or Gloves file sits below its folder.
    let depth = if is("Players") || is("Boots") || is("Gloves") {
        2
    } else if is(COMMON) {
        1
    } else {
        return false;
    };
    segments.count() >= depth
}

/// The outermost names below the folder at `folder` (its export path) holding a legacy kit
/// token, among its `files` the upgrader respells (`kit_token_checked`), as export paths:
/// each file's or subfolder's outermost such name, once (folded), in file order. A subfolder
/// so named stands for every file below it: the fix, an upgrade, is the same for all of them.
/// The folder's own name is not checked: the Export upgrader respells the names below a
/// player, Boots, Gloves or `Common/` folder and keeps that folder's own, so refusing it would
/// send the member to a tool that changes nothing.
fn legacy_token_names<'a>(folder: &str, files: &'a [FileDescriptor]) -> Vec<&'a str> {
    // Below the folder: past its path and the `/` after it.
    let below = folder.len() + 1;
    let mut seen = BTreeSet::new();
    files
        .iter()
        .filter(|file| kit_token_checked(&file.path))
        .filter_map(|file| {
            let path = file.path.as_str();
            let name = &path[..below + legacy_token_name_end(path.get(below..)?)?];
            seen.insert(vtree::fold_name(name)).then_some(name)
        })
        .collect()
}

/// `kit_token_old` on the model folder `folder` (a player's or a shared one) for each
/// outermost name holding a legacy kit token (`legacy_token_names`), naming the name's path
/// in the export and dropping the folder.
fn kit_token_names(folder: &ScopePath, files: &[FileDescriptor]) -> Vec<ContentFinding> {
    legacy_token_names(folder.as_str(), files)
        .into_iter()
        .map(|path| {
            kit_token_name_finding(
                IssueScope::Folder(folder.clone()),
                Disposition::DropFolder,
                path,
            )
        })
        .collect()
}

/// `kit_token_old` for each outermost name in `Common/` holding a legacy kit token
/// (`legacy_token_names`), with that name's path: a file's on the file, which drops it
/// alone, and a subfolder's on the subfolder, which drops every file below it.
fn common_kit_token_names(common: &[FileDescriptor]) -> Vec<ContentFinding> {
    legacy_token_names(COMMON, common)
        .into_iter()
        .map(|name| {
            let (scope, disposition) = match common.iter().find(|file| file.path.as_str() == name) {
                Some(file) => (IssueScope::File(file.path.clone()), Disposition::DropFile),
                None => (
                    IssueScope::Folder(
                        ScopePath::new(name)
                            .expect("a file's export path cut before a `/` is a valid path"),
                    ),
                    Disposition::DropFolder,
                ),
            };
            kit_token_name_finding(scope, disposition, name)
        })
        .collect()
}

/// The `Common/` folder's export path.
const COMMON: &str = "Common";

/// The `kit_token_old` finding on `scope` for the name at export path `path`.
fn kit_token_name_finding(
    scope: IssueScope,
    disposition: Disposition,
    path: &str,
) -> ContentFinding {
    ContentFinding {
        code: Code::KitTokenOld.as_str(),
        scope,
        context: vec![("path", path.to_owned())],
        disposition,
        pass_through_eligible: false,
    }
}

/// Whether the export path `path` is the name at export path `name` or below it.
fn at_or_below(path: &str, name: &str) -> bool {
    path.strip_prefix(name)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// The end, in bytes, of the outermost `/`-separated name of `path` holding a legacy kit
/// token (`aesthetics_export::legacy_kit_tokens`), so that `path` cut there is that name's
/// path; `None` when no name of it holds one. A token never holds a `/`, so the name is the
/// one the first token stands in.
fn legacy_token_name_end(path: &str) -> Option<usize> {
    let start = legacy_kit_tokens(path).first()?.start;
    Some(
        path[start..]
            .find('/')
            .map_or(path.len(), |slash| start + slash),
    )
}

/// `path` below `folder`, its subfolder kept (`face/hair.fmdl`): how a finding on a folder
/// names one of its files.
pub(crate) fn relative(path: &ScopePath, folder: &ScopePath) -> String {
    path.segments()
        .skip(folder.segments().count())
        .collect::<Vec<_>>()
        .join("/")
}

/// How a finding on the model folder at `folder` names the file at `path`: below the folder
/// (`relative`), or by its export path when it sits outside it — in `Common/`
/// (`Common/legs.model`, `Common/refkit/oral_arm_win32.model`, which a member's `face.xml`
/// may name), or in a shared folder the task's folder combines or converts.
pub(crate) fn named_on_folder(path: &ScopePath, folder: &ScopePath) -> String {
    let below_folder = path
        .segments()
        .take(folder.segments().count())
        .eq(folder.segments());
    if below_folder {
        relative(path, folder)
    } else {
        path.as_str().to_owned()
    }
}

/// The findings of `file`, read as `checked`, each on `scope` and naming the file `name`. A
/// model or material set gets one per code its format crate's check fires, with the summed
/// count (an Error with `disposition`, pass-through-eligible unless it is the far vertex; a
/// Warning or an Info `Keep`); one that does not parse is `model_broken` or `mtl_broken`, with
/// `disposition` and never eligible: there is nothing to pack. A texture gets at most one
/// (`texture_finding`), with `disposition`. A logo source that does not decode is
/// `logo_file_invalid`, with `disposition` and never eligible: no logo can be made from it. A
/// file that cannot be read is `source_read_failed`, with `disposition` and never eligible.
///
/// For a target of `engine` PES 15-17, an FMDL that parses and whose every mesh is hidden
/// (`model::ModelRead::hidden`) also gets `model_hidden_dropped`, after its format findings: an
/// Info on the file itself that drops the file alone, whatever `scope` and `disposition` are,
/// and never passes through. Its conversion would leave nothing of it, and dropping it here,
/// before any task is planned, keeps every task from naming it: the task converting a model is
/// not always the one naming it (a Common model's linking face, a shared folder's merge).
fn file_findings(
    content: &ContentSource,
    file: &FileDescriptor,
    checked: Checked,
    scope: &IssueScope,
    disposition: Disposition,
    name: &str,
    engine: Engine,
) -> Vec<ContentFinding> {
    file_outcome(content, file, checked, scope, disposition, name, engine).findings
}

/// `file_findings`, with `file` among the pass's weighted models when it is a model that parses
/// and carries hand weights (`ContentPass::hand_weighted`), among its metal models when it is an
/// FMDL that parses and holds a metal material (`ContentPass::metal_models`), among its hidden
/// models when it gets `model_hidden_dropped` (`ContentPass::hidden_models`), and with its
/// material names when it is a pre-Fox model or material set that parses
/// (`ContentPass::materials`).
fn file_outcome(
    content: &ContentSource,
    file: &FileDescriptor,
    checked: Checked,
    scope: &IssueScope,
    disposition: Disposition,
    name: &str,
    engine: Engine,
) -> ContentPass {
    let finding = |code: &'static str,
                   context: Vec<(&'static str, String)>,
                   disposition: Disposition,
                   pass_through_eligible: bool| ContentFinding {
        code,
        scope: scope.clone(),
        context,
        disposition,
        pass_through_eligible,
    };
    let bytes = match read(content, file, scope, disposition) {
        Ok(bytes) => bytes,
        Err(unread) => return ContentPass::findings_only(vec![unread]),
    };
    let kind = match checked {
        Checked::Model(kind) => kind,
        Checked::Texture(format, rule) => {
            let found = texture_finding(format, rule, file_stem(name), &bytes)
                .map(|code| {
                    // A renamed file cannot be converted as the format its name declares, nor
                    // can one whose header cannot be read or whose codec or kind the converter
                    // refuses: packed, each would fail its task; nor can a pre-Fox side that is
                    // no multiple of 4 be kept: a texture the compile writes block-compressed
                    // is one Direct3D 9 cannot create at such a size. A texture of another
                    // odd size converts, and what the game makes of it is the member's risk.
                    let eligible = !matches!(
                        code,
                        Code::TextureTypeMismatch
                            | Code::TextureUnreadable
                            | Code::TextureCodecUnsupported
                            | Code::TextureNotDiv4
                    );
                    finding(
                        code.as_str(),
                        vec![("file", name.to_owned())],
                        disposition,
                        eligible,
                    )
                })
                .into_iter()
                .collect();
            return ContentPass::findings_only(found);
        }
        Checked::Logo(format) => {
            let found = dds_convert::decode(&bytes, format)
                .err()
                .map(|error| {
                    finding(
                        LOGO_FILE_INVALID,
                        vec![("file", name.to_owned()), ("error", error.to_string())],
                        disposition,
                        false,
                    )
                })
                .into_iter()
                .collect();
            return ContentPass::findings_only(found);
        }
    };
    let read = match fired(kind, &bytes) {
        Ok(read) => read,
        Err(error) => {
            let broken = match kind {
                ModelKind::Fmdl | ModelKind::PreFoxModel => Code::ModelBroken,
                ModelKind::Mtl => Code::MtlBroken,
            };
            return ContentPass::findings_only(vec![finding(
                broken.as_str(),
                vec![("file", name.to_owned()), ("error", error)],
                disposition,
                false,
            )]);
        }
    };
    let mut findings: Vec<ContentFinding> = summed(read.fired)
        .into_iter()
        .map(|rule| {
            let far = FAR_VERTEX_CODES.contains(&rule.code);
            let code = if far {
                Code::VertexTooFarFromOrigin.as_str()
            } else {
                rule.code
            };
            let context = vec![("file", name.to_owned()), ("count", rule.count.to_string())];
            if rule.error {
                // Pass-through packs the file as it is; far geometry lags the whole matchday.
                finding(code, context, disposition, !far)
            } else {
                finding(code, context, Disposition::Keep, false)
            }
        })
        .collect();
    for found in &read.common_directories {
        // The export's spelling, refused wherever the FMDL sits: never passed through.
        findings.push(finding(
            Code::FmdlCommonPathInvalid.as_str(),
            vec![
                ("model", name.to_owned()),
                ("texture", found.texture.clone()),
                ("directory", found.directory.clone()),
            ],
            disposition,
            false,
        ));
    }
    for texture in &read.old_kit_names {
        // The old spelling, refused as the Common one is: never passed through.
        findings.push(finding(
            Code::DummyKitNameOld.as_str(),
            vec![("model", name.to_owned()), ("texture", texture.clone())],
            disposition,
            false,
        ));
    }
    let legacy_kit_paths = if kit_token_checked(&file.path) {
        read.legacy_kit_paths.as_slice()
    } else {
        &[]
    };
    for reference in legacy_kit_paths {
        findings.push(finding(
            Code::KitTokenOld.as_str(),
            vec![("file", name.to_owned()), ("reference", reference.clone())],
            disposition,
            false,
        ));
    }
    // On Fox the file is packed as it is, the game drawing nothing of it as the member saw.
    let dropped_hidden = match engine {
        Engine::PreFox => read.hidden,
        Engine::Fox => false,
    };
    let mut hidden_models = BTreeSet::new();
    if dropped_hidden {
        findings.push(ContentFinding {
            code: Code::ModelHiddenDropped.as_str(),
            scope: IssueScope::File(file.path.clone()),
            context: vec![("file", name.to_owned())],
            disposition: Disposition::DropFile,
            pass_through_eligible: false,
        });
        hidden_models.insert(file.path.clone());
    }
    let mut hand_weighted = BTreeSet::new();
    if read.hand_weighted {
        hand_weighted.insert(file.path.clone());
    }
    let mut metal_models = BTreeSet::new();
    if read.metal {
        metal_models.insert(file.path.clone());
    }
    let mut materials = BTreeMap::new();
    match kind {
        ModelKind::PreFoxModel | ModelKind::Mtl => {
            materials.insert(file.path.clone(), read.materials);
        }
        ModelKind::Fmdl => {}
    }
    ContentPass {
        findings,
        hand_weighted,
        metal_models,
        materials,
        hidden_models,
        settings: BTreeMap::new(),
        settings_refused: BTreeSet::new(),
        legacy_ids: None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::slice;

    use aesthetics_export::{Disposition, IssueScope};
    use dds_convert::{BlockCodec, Blocks, Decoded, encode_dds};
    use fmdl::{FmdlFile, Model};
    use pipeline::MemoryBudget;
    use studio_core::ExportId;
    use vtree::ScopePath;

    use super::*;
    use crate::reader::{ExportSource, SourceKind};
    use crate::testing::{resolved_with_issues, scratch};

    /// The bytes of `tests/fixtures/<relative>`.
    pub(super) fn fixture(relative: &str) -> Vec<u8> {
        fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(relative),
        )
        .unwrap()
    }

    /// The bytes of `pes_model`'s fixture `name`: real pre-Fox models and material sets,
    /// which this crate's own fixtures do not hold (`pes_model/tests/fixtures/README.md`).
    pub(super) fn pre_fox_fixture(name: &str) -> Vec<u8> {
        fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../libs/pes_model/tests/fixtures")
                .join(name),
        )
        .unwrap()
    }

    /// The bytes of the tracer player's file `name`.
    pub(super) fn tracer_file(name: &str) -> Vec<u8> {
        fixture(&format!(
            "tracer/studio/egg Midcup Tracer/Players/05 - The Chad Stormworks Player/{name}"
        ))
    }

    /// The tracer's boots model with one vertex 6000 units from the origin.
    pub(super) fn far_boots() -> Vec<u8> {
        fixture("deep/boots_far.fmdl")
    }

    #[test]
    fn a_folder_names_its_own_file_below_it_and_an_outside_one_by_its_export_path() {
        let folder = ScopePath::new("Players/05 - A").unwrap();
        let named = |path: &str| named_on_folder(&ScopePath::new(path).unwrap(), &folder);
        assert_eq!(named("Players/05 - A/boots.fmdl"), "boots.fmdl");
        assert_eq!(named("Players/05 - A/face/hat.model"), "face/hat.model");
        assert_eq!(named("Common/legs.model"), "Common/legs.model");
        // A subfolder's file, which a member's `face.xml` may name, keeps its whole path.
        assert_eq!(
            named("Common/refkit/oral_thigh_win32.model"),
            "Common/refkit/oral_thigh_win32.model"
        );
        // A shared folder's model a player's task converts, and one of a folder whose name
        // merely starts with the player's.
        assert_eq!(named("Faces/Round/hat.fmdl"), "Faces/Round/hat.fmdl");
        assert_eq!(
            named("Players/05 - AB/hat.fmdl"),
            "Players/05 - AB/hat.fmdl"
        );
        let boots = ScopePath::new("Boots/Mud").unwrap();
        assert_eq!(
            named_on_folder(&ScopePath::new("Boots/Mud/boots.fmdl").unwrap(), &boots),
            "boots.fmdl"
        );
    }

    /// The tracer's own boots model, all of it near the origin; 1662 of its vertices carry
    /// weights that sum to neither 255 nor 0 (1626, 18 and 18 in its three meshes).
    pub(super) fn tracer_boots() -> Vec<u8> {
        tracer_file("boots.fmdl")
    }

    /// The Fox model `bytes` with `edit` applied, written back through `fmdl`.
    pub(super) fn edited(bytes: &[u8], edit: impl FnOnce(&mut Model)) -> Vec<u8> {
        let mut model = Model::from_file(&FmdlFile::read(bytes).unwrap()).unwrap();
        edit(&mut model);
        model.to_file().unwrap().write()
    }

    /// The tracer's right glove, in which `fmdl`'s check finds nothing, with its mesh holding
    /// 21846 faces (its first, repeated), one over the hard limit: one
    /// `fmdl_mesh_over_face_limit`, an Error. (`fmdl` refuses to write a face naming a
    /// vertex that does not exist, or a mesh no group lists.)
    pub(super) fn glove_over_the_face_limit() -> Vec<u8> {
        edited(&tracer_file("glove_r.fmdl"), |model| {
            let mesh = &mut model.meshes[0];
            mesh.faces = vec![mesh.faces[0]; 21_846];
        })
    }

    /// The deep pass for PES 21 over the folder export `co Midcup Deep` at `root`, holding `files`
    /// (path, bytes) and listing `unwritten` too, which is not on disk; the structure pass
    /// finds nothing in it.
    pub(super) fn findings_of(
        root: &Path,
        files: &[(&str, Vec<u8>)],
        unwritten: &[&str],
    ) -> Vec<ContentFinding> {
        findings_for(PesVersion::Pes21, root, files, unwritten, &[])
    }

    /// `findings_of` for PES `version`, the structure pass finding exactly the codes
    /// `structure_codes` (a kit inheriting from `all/` is `kit_textures_inherited`).
    pub(super) fn findings_for(
        version: PesVersion,
        root: &Path,
        files: &[(&str, Vec<u8>)],
        unwritten: &[&str],
        structure_codes: &[&str],
    ) -> Vec<ContentFinding> {
        pass_for(version, root, files, unwritten, structure_codes).findings
    }

    /// `findings_for`'s whole pass, the weighted FMDLs with the findings.
    pub(super) fn pass_for(
        version: PesVersion,
        root: &Path,
        files: &[(&str, Vec<u8>)],
        unwritten: &[&str],
        structure_codes: &[&str],
    ) -> ContentPass {
        export_pass(
            "co Midcup Deep",
            None,
            version,
            root,
            files,
            unwritten,
            structure_codes,
        )
    }

    /// `pass_for` over the folder export `name`, whose `players.txt` is `players_txt`.
    fn export_pass(
        name: &str,
        players_txt: Option<&[u8]>,
        version: PesVersion,
        root: &Path,
        files: &[(&str, Vec<u8>)],
        unwritten: &[&str],
        structure_codes: &[&str],
    ) -> ContentPass {
        let listed = written(root, files, unwritten);
        let (resolved, codes) = resolved_with_issues(name, &listed, &[], players_txt);
        assert_eq!(codes, structure_codes, "the structure pass's findings");
        pass_over(name, root, &resolved.export, version)
    }

    /// Writes `files` below `root` and lists them with `unwritten`, each of those one byte.
    fn written<'a>(
        root: &Path,
        files: &[(&'a str, Vec<u8>)],
        unwritten: &[&'a str],
    ) -> Vec<(&'a str, u64)> {
        for (path, bytes) in files {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        files
            .iter()
            .map(|(path, bytes)| (*path, bytes.len() as u64))
            .chain(unwritten.iter().map(|path| (*path, 1)))
            .collect()
    }

    /// The deep pass over `export`, the folder export `name` at `root`, for `version`.
    fn pass_over(
        name: &str,
        root: &Path,
        export: &ValidatedAestheticsExport,
        version: PesVersion,
    ) -> ContentPass {
        let source = ExportSource {
            export_id: ExportId(0),
            path: root.to_path_buf(),
            kind: SourceKind::Folder,
            file_name: name.to_owned(),
            display_name: name.to_owned(),
            team_name: None,
        };
        let content = ContentSource::new(&source, &MemoryBudget::new(1 << 30));
        content_findings(export, &content, version, None, false)
    }

    /// The bytes of `tests/fixtures/textures/<name>`, described in
    /// `tests/fixtures/textures/README.md`.
    pub(super) fn texture(name: &str) -> Vec<u8> {
        fixture(&format!("textures/{name}"))
    }

    /// A single-level BC1 DDS of `width`x`height`, its blocks zero: built from the blocks, so
    /// nothing is encoded.
    pub(super) fn bc1_dds(width: u32, height: u32) -> Vec<u8> {
        bc1_dds_levels(width, height, 1)
    }

    /// `bc1_dds` with `levels` mip levels, each `max(1, side >> level)`, its block count the
    /// level's size rounded up to whole 4x4 blocks.
    pub(super) fn bc1_dds_levels(width: u32, height: u32, levels: u32) -> Vec<u8> {
        let mut mips = Vec::new();
        let mut blocks = Vec::new();
        for level in 0..levels {
            let w = width.checked_shr(level).unwrap_or(0).max(1);
            let h = height.checked_shr(level).unwrap_or(0).max(1);
            mips.push(vec![0; (w * h * 4) as usize]);
            blocks.push(vec![0; (w.div_ceil(4) * h.div_ceil(4) * 8) as usize]);
        }
        let decoded = Decoded {
            width,
            height,
            mips,
            blocks: Some(Blocks {
                codec: BlockCodec::Bc1,
                mips: blocks,
            }),
            authored_mips: true,
            uncompressed: false,
        };
        encode_dds(&decoded, BlockCodec::Bc1).unwrap()
    }

    /// The texture finding `code` on `scope` about the file `file`.
    pub(super) fn texture_finding_on(
        code: &'static str,
        scope: &IssueScope,
        file: &str,
        disposition: Disposition,
        pass_through_eligible: bool,
    ) -> ContentFinding {
        ContentFinding {
            code,
            scope: scope.clone(),
            context: vec![("file", file.to_owned())],
            disposition,
            pass_through_eligible,
        }
    }

    pub(super) fn path(text: &str) -> ScopePath {
        ScopePath::new(text).unwrap()
    }

    pub(super) fn folder(text: &str) -> IssueScope {
        IssueScope::Folder(path(text))
    }

    /// The finding `code` on `scope` about the file `file`, `count` items of which tripped
    /// the rule.
    pub(super) fn counted(
        code: &'static str,
        scope: &IssueScope,
        file: &str,
        count: usize,
        disposition: Disposition,
        pass_through_eligible: bool,
    ) -> ContentFinding {
        ContentFinding {
            code,
            scope: scope.clone(),
            context: vec![("file", file.to_owned()), ("count", count.to_string())],
            disposition,
            pass_through_eligible,
        }
    }

    /// The finding `code` on the folder `scope` with `context`, never passed through, with
    /// `disposition`.
    fn on_folder(
        code: &'static str,
        scope: &str,
        context: &[(&'static str, &str)],
        disposition: Disposition,
    ) -> ContentFinding {
        ContentFinding {
            code,
            scope: folder(scope),
            context: context
                .iter()
                .map(|(key, value)| (*key, (*value).to_owned()))
                .collect(),
            disposition,
            pass_through_eligible: false,
        }
    }

    /// The finding `code` on the folder `scope`, dropping it and never passed through, with
    /// `context`.
    pub(super) fn dropping(
        code: &'static str,
        scope: &str,
        context: &[(&'static str, &str)],
    ) -> ContentFinding {
        on_folder(code, scope, context, Disposition::DropFolder)
    }

    /// The finding `code` on the folder `scope`, which keeps it, with `context`.
    fn keeping(
        code: &'static str,
        scope: &str,
        context: &[(&'static str, &str)],
    ) -> ContentFinding {
        on_folder(code, scope, context, Disposition::Keep)
    }

    #[test]
    fn a_common_model_s_far_vertex_drops_the_file() {
        let temp = scratch("deep_common");
        let findings = findings_of(
            temp.path(),
            &[
                ("Common/legs.fmdl", far_boots()),
                ("Players/03 - A/legs.fmdl.common", Vec::new()),
            ],
            &[],
        );
        let file = IssueScope::File(path("Common/legs.fmdl"));
        assert_eq!(
            findings,
            [
                counted(
                    "vertex_too_far_from_origin",
                    &file,
                    "legs.fmdl",
                    1,
                    Disposition::DropFile,
                    false
                ),
                counted(
                    "fmdl_weights_not_normalized",
                    &file,
                    "legs.fmdl",
                    1662,
                    Disposition::Keep,
                    false
                ),
            ]
        );
    }

    #[test]
    fn a_model_in_a_subfolder_is_named_with_its_subfolder_and_drops_the_folder() {
        let temp = scratch("deep_subfolder");
        let findings = findings_of(
            temp.path(),
            &[
                ("Players/03 - A/face/hair.fmdl", far_boots()),
                ("Players/05 - B/boots.fmdl", tracer_boots()),
            ],
            &[],
        );
        let a = folder("Players/03 - A");
        let b = folder("Players/05 - B");
        assert_eq!(
            findings,
            [
                counted(
                    "vertex_too_far_from_origin",
                    &a,
                    "face/hair.fmdl",
                    1,
                    Disposition::DropFolder,
                    false
                ),
                counted(
                    "fmdl_weights_not_normalized",
                    &a,
                    "face/hair.fmdl",
                    1662,
                    Disposition::Keep,
                    false
                ),
                counted(
                    "fmdl_weights_not_normalized",
                    &b,
                    "boots.fmdl",
                    1662,
                    Disposition::Keep,
                    false
                ),
            ]
        );
    }

    #[test]
    fn the_fmdls_weighted_to_hand_bones_are_recorded_by_their_export_path() {
        let temp = scratch("deep_hand_weighted");
        let body = fixture("hand_split/body.fmdl");
        let pass = pass_for(
            PesVersion::Pes21,
            temp.path(),
            &[
                ("Players/05 - A/body.fmdl", body.clone()),
                ("Players/05 - A/boots.fmdl", tracer_boots()),
                ("Players/06 - B/torso.fmdl.common", Vec::new()),
                ("Common/torso.fmdl", body),
                ("Players/07 - C/body.fmdl", b"not a model".to_vec()),
            ],
            &[],
            &[],
        );
        let recorded: Vec<&str> = pass.hand_weighted.iter().map(ScopePath::as_str).collect();
        assert_eq!(recorded, ["Common/torso.fmdl", "Players/05 - A/body.fmdl"]);
        // The model's own findings are unchanged: its check finds nothing, the boots their
        // usual weights, and the broken file is `model_broken`.
        let codes: Vec<&str> = pass.findings.iter().map(|finding| finding.code).collect();
        assert_eq!(codes, ["fmdl_weights_not_normalized", "model_broken"]);
    }

    #[test]
    fn the_fmdls_holding_a_metal_material_are_recorded_whatever_the_target() {
        // The tracer's right glove with its material's shader spelled as Fox techniques are,
        // which the converter's rule reads in any letter case; its boots with a shader the
        // converter reads as glass although it names `ggx`; the tracer's own models are not
        // metal.
        let metal = edited(&tracer_file("glove_r.fmdl"), |model| {
            "fox3DDF_GGX".clone_into(&mut model.materials[0].shader);
        });
        let glass = edited(&tracer_boots(), |model| {
            "fox3ddf_glass_ggx".clone_into(&mut model.materials[0].shader);
        });
        let files = [
            ("Players/05 - A/glove_r.fmdl", metal),
            ("Players/05 - A/boots.fmdl", glass),
            ("Players/06 - B/fcl_hair.fmdl", tracer_file("fcl_hair.fmdl")),
            ("Players/06 - B/glove_l.fmdl", tracer_file("glove_l.fmdl")),
        ];
        let recorded = |version: PesVersion, name: &str| {
            let temp = scratch(name);
            let pass = pass_for(version, temp.path(), &files, &[], &[]);
            pass.metal_models
                .iter()
                .map(|path| path.as_str().to_owned())
                .collect::<Vec<_>>()
        };
        // Planning, not the pass, knows which target converts one (`plan::converts_metal`).
        assert_eq!(
            recorded(PesVersion::Pes17, "deep_metal_17"),
            ["Players/05 - A/glove_r.fmdl"]
        );
        assert_eq!(
            recorded(PesVersion::Pes21, "deep_metal_21"),
            ["Players/05 - A/glove_r.fmdl"]
        );
    }

    #[test]
    fn a_common_model_s_format_error_drops_the_file() {
        let temp = scratch("deep_common_error");
        let findings = findings_of(
            temp.path(),
            &[
                ("Common/legs.fmdl", glove_over_the_face_limit()),
                ("Players/03 - A/legs.fmdl.common", Vec::new()),
            ],
            &[],
        );
        assert_eq!(
            findings,
            [counted(
                "fmdl_mesh_over_face_limit",
                &IssueScope::File(path("Common/legs.fmdl")),
                "legs.fmdl",
                21_846,
                Disposition::DropFile,
                true
            )]
        );
    }

    #[test]
    fn a_common_file_is_dropped_unless_pass_through_keeps_its_every_dropping_error() {
        let file = IssueScope::File(path("Common/legs.mtl"));
        let finding = |code, disposition, eligible| {
            counted(code, &file, "legs.mtl", 1, disposition, eligible)
        };
        let eligible = finding("mtl_state_invalid", Disposition::DropFile, true);
        let not_eligible = finding("mtl_broken", Disposition::DropFile, false);
        let warning = finding("mtl_texture_not_found", Disposition::Keep, false);
        for pass_through in [false, true] {
            assert!(!drops_file(&[], pass_through));
            assert!(!drops_file(slice::from_ref(&warning), pass_through));
            assert!(drops_file(slice::from_ref(&not_eligible), pass_through));
            assert!(drops_file(
                &[eligible.clone(), not_eligible.clone()],
                pass_through
            ));
        }
        assert!(drops_file(slice::from_ref(&eligible), false));
        assert!(!drops_file(slice::from_ref(&eligible), true));
        assert!(!drops_file(&[eligible.clone(), warning.clone()], true));
    }

    #[test]
    fn the_tracer_s_texture_passes_and_files_that_are_not_checked_are_not_read() {
        let temp = scratch("deep_not_models");
        let findings = findings_of(
            temp.path(),
            &[
                ("Players/05 - B/glove_r.fmdl", tracer_file("glove_r.fmdl")),
                ("Players/05 - B/shirt.dds", tracer_file("shirt.dds")),
                ("Players/05 - B/fcl_hair.skl", tracer_file("fcl_hair.skl")),
                ("Players/05 - B/face_diff.bin", tracer_file("face_diff.bin")),
            ],
            &[],
        );
        assert_eq!(findings, []);
    }

    #[test]
    fn a_model_that_cannot_be_read_is_source_read_failed_on_its_folder() {
        let temp = scratch("deep_unreadable");
        let findings = findings_of(
            temp.path(),
            &[("Players/05 - B/boots.fmdl", tracer_boots())],
            &["Players/03 - A/boots.fmdl"],
        );
        let [finding, readable] = findings.as_slice() else {
            panic!("{findings:?}");
        };
        assert_eq!(finding.code, "source_read_failed");
        assert_eq!(finding.scope, folder("Players/03 - A"));
        assert_eq!(finding.disposition, Disposition::DropFolder);
        assert!(!finding.pass_through_eligible);
        let missing: PathBuf = temp.path().join("Players/03 - A/boots.fmdl");
        assert_eq!(finding.context[0], ("path", missing.display().to_string()));
        assert_eq!(finding.context[1].0, "error");
        assert_eq!(finding.context.len(), 2);
        assert_eq!(
            *readable,
            counted(
                "fmdl_weights_not_normalized",
                &folder("Players/05 - B"),
                "boots.fmdl",
                1662,
                Disposition::Keep,
                false
            )
        );
    }

    #[test]
    fn a_common_texture_with_a_finding_drops_the_file() {
        let temp = scratch("deep_texture_common");
        let findings = findings_of(
            temp.path(),
            &[
                ("Common/tiny.png", texture("tiny.png")),
                ("Common/hair.png", texture("kit.png")),
            ],
            &[],
        );
        assert_eq!(
            findings,
            [texture_finding_on(
                "texture_too_small",
                &IssueScope::File(path("Common/tiny.png")),
                "tiny.png",
                Disposition::DropFile,
                true
            )]
        );
    }

    #[test]
    fn a_common_file_named_with_a_legacy_kit_token_is_refused_and_not_checked_the_others_are() {
        let temp = scratch("deep_texture_common_token");
        // Both textures are too small: only the one with no token is checked.
        let findings = findings_of(
            temp.path(),
            &[
                ("Common/u0XXXp1_tiny.png", texture("tiny.png")),
                ("Common/tiny.png", texture("tiny.png")),
            ],
            &[],
        );
        assert_eq!(
            findings,
            [
                kit_token_name_finding(
                    IssueScope::File(path("Common/u0XXXp1_tiny.png")),
                    Disposition::DropFile,
                    "Common/u0XXXp1_tiny.png"
                ),
                texture_finding_on(
                    "texture_too_small",
                    &IssueScope::File(path("Common/tiny.png")),
                    "tiny.png",
                    Disposition::DropFile,
                    true
                )
            ]
        );
    }

    #[test]
    fn an_all_texture_s_finding_goes_to_each_kit_inheriting_it_naming_its_path() {
        let temp = scratch("deep_texture_all");
        let findings = findings_for(
            PesVersion::Pes21,
            temp.path(),
            &[
                ("Kits/all/kit_back.png", texture("tiny.png")),
                ("Kits/p1/kit.png", texture("kit.png")),
                ("Kits/p2/kit.png", texture("kit.png")),
                ("Kits/g1/kit.png", texture("kit.png")),
                ("Kits/g1/kit_back.png", texture("kit.png")),
            ],
            &[],
            &["kit_textures_inherited", "kit_textures_inherited"],
        );
        assert_eq!(
            findings,
            ["Kits/p1", "Kits/p2"].map(|kit| texture_finding_on(
                "texture_too_small",
                &folder(kit),
                "Kits/all/kit_back.png",
                Disposition::DropFolder,
                true
            ))
        );
    }

    #[test]
    fn a_logo_that_does_not_decode_is_logo_file_invalid_on_its_file() {
        let temp = scratch("deep_logo_invalid");
        let findings = findings_of(temp.path(), &[("logo.png", b"not an image".to_vec())], &[]);
        let error = dds_convert::decode(b"not an image", SourceFormat::Png)
            .unwrap_err()
            .to_string();
        assert_eq!(
            findings,
            [ContentFinding {
                code: "logo_file_invalid",
                scope: IssueScope::File(path("logo.png")),
                context: vec![("file", "logo.png".to_owned()), ("error", error)],
                disposition: Disposition::DropFile,
                pass_through_eligible: false,
            }]
        );
        assert!(aesthetics_export::ISSUE_CODES.contains(&"logo_file_invalid"));
        let temp = scratch("deep_logo_valid");
        let findings = findings_of(temp.path(), &[("logo.png", texture("portrait.png"))], &[]);
        assert_eq!(findings, []);
    }

    #[test]
    fn findings_keep_the_file_order_whatever_the_workers_scheduling() {
        let temp = scratch("deep_parallel_order");
        let players = ["03 - A", "05 - B", "07 - C", "09 - D"];
        let files: Vec<(String, Vec<u8>)> = players
            .iter()
            .flat_map(|player| {
                [
                    (format!("Players/{player}/skin.png"), texture("tiny.png")),
                    (format!("Players/{player}/boots.fmdl"), tracer_boots()),
                    (format!("Players/{player}/hair.png"), texture("tiny.png")),
                ]
            })
            .collect();
        let files: Vec<(&str, Vec<u8>)> = files
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.clone()))
            .collect();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();

        let runs: Vec<Vec<ContentFinding>> = (0..20)
            .map(|_| pool.install(|| findings_of(temp.path(), &files, &[])))
            .collect();

        for (i, run) in runs.iter().enumerate() {
            assert_eq!(*run, runs[0], "run {i}");
        }
        let expected: Vec<ContentFinding> = players
            .iter()
            .flat_map(|player| {
                let scope = folder(&format!("Players/{player}"));
                [
                    counted(
                        "fmdl_weights_not_normalized",
                        &scope,
                        "boots.fmdl",
                        1662,
                        Disposition::Keep,
                        false,
                    ),
                    texture_finding_on(
                        "texture_too_small",
                        &scope,
                        "hair.png",
                        Disposition::DropFolder,
                        true,
                    ),
                    texture_finding_on(
                        "texture_too_small",
                        &scope,
                        "skin.png",
                        Disposition::DropFolder,
                        true,
                    ),
                ]
            })
            .collect();
        assert_eq!(runs[0], expected);
    }

    /// The card head's model, clean, binding one material, `card`.
    fn card() -> Vec<u8> {
        pre_fox_fixture("cardhead_face_high.model")
    }

    /// The card head's material set, defining `card`.
    fn card_materials() -> Vec<u8> {
        pre_fox_fixture("cardhead_materials.mtl")
    }

    /// The card head's material set with its one material renamed `other`.
    fn other_materials() -> Vec<u8> {
        let mut set = pes_model::format::mtl::MaterialSet::read(&card_materials()).unwrap();
        set.materials[0].name = "other".to_owned();
        set.write()
    }

    /// The PES 17 deep pass's findings over `files`, slot 05's folder holding them by name.
    fn slot_05_findings(name: &str, files: &[(&str, Vec<u8>)]) -> Vec<ContentFinding> {
        let temp = scratch(name);
        let files: Vec<(String, Vec<u8>)> = files
            .iter()
            .map(|(file, bytes)| (format!("Players/05 - A/{file}"), bytes.clone()))
            .collect();
        let files: Vec<(&str, Vec<u8>)> = files
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.clone()))
            .collect();
        findings_for(PesVersion::Pes17, temp.path(), &files, &[], &[])
    }

    /// A `face.xml` whose one `<model>`, typed `parts`, holds `attributes`.
    fn one_model_xml(attributes: &str) -> Vec<u8> {
        format!(r#"<config><model level="0" type="parts" {attributes}/></config>"#).into_bytes()
    }

    #[test]
    fn a_name_holding_the_legacy_kit_token_is_reported_once_on_its_outermost_name() {
        let descriptors = |paths: &[&str]| -> Vec<FileDescriptor> {
            paths
                .iter()
                .map(|text| FileDescriptor {
                    path: path(text),
                    source: path(text),
                    size: 1,
                    kind: aesthetics_export::classify(path(text).name()),
                })
                .collect()
        };
        let refused = |folder_path: &str, name: &str| ContentFinding {
            code: "kit_token_old",
            scope: folder(folder_path),
            context: vec![("path", name.to_owned())],
            disposition: Disposition::DropFolder,
            pass_through_eligible: false,
        };
        // A subfolder so named is reported once, not its two files; a file so named on its
        // own; capitals are no token.
        let files = descriptors(&[
            "Players/05 - A/face_high.fmdl",
            "Players/05 - A/u0XXXp2/a.dds",
            "Players/05 - A/u0XXXp2/kit1_u0714p1.dds",
            "Players/05 - A/kit1_u0714p1.dds",
            "Players/05 - A/U0714P1.dds",
        ]);
        assert_eq!(
            kit_token_names(&path("Players/05 - A"), &files),
            [
                refused("Players/05 - A", "Players/05 - A/u0XXXp2"),
                refused("Players/05 - A", "Players/05 - A/kit1_u0714p1.dds"),
            ]
        );
        // A shared folder's own name is not checked (the upgrader keeps it); a name below it
        // is.
        let files = descriptors(&["Boots/u0XXXp1/boots.fmdl", "Boots/u0XXXp1/x/u0714p1.dds"]);
        assert_eq!(
            kit_token_names(&path("Boots/u0XXXp1"), &files),
            [refused("Boots/u0XXXp1", "Boots/u0XXXp1/x/u0714p1.dds")]
        );
    }

    #[test]
    fn the_legacy_kit_token_is_checked_where_the_upgrader_respells_it() {
        // Below a player, Boots or Gloves folder, its root in any letter case, and in
        // `Common/`; not a model folder's own name, a shared face folder or a collar.
        for (at, checked) in [
            ("Players/05 - A/face.mtl", true),
            ("players/05 - A/x/face.mtl", true),
            ("Boots/Crocs/boots.fmdl", true),
            ("GLOVES/Mitts/glove_l.fmdl", true),
            ("Common/u0XXXp1.dds", true),
            ("common/jessie/u0XXXp1.dds", true),
            ("Players/05 - A", false),
            ("Boots/Crocs", false),
            ("Common", false),
            ("Faces/Round/face.mtl", false),
            ("Collars/collar_12.fmdl", false),
            ("Kits/kit1/u0XXXp1.dds", false),
        ] {
            assert_eq!(kit_token_checked(&path(at)), checked, "{at}");
        }
    }

    #[test]
    fn a_face_xml_path_holding_the_legacy_kit_token_is_refused_once_per_distinct_value() {
        let findings = slot_05_findings(
            "deep_xml_kit_token",
            &[
                ("hat.model", card()),
                ("hat.mtl", card_materials()),
                ("texture.dds", bc1_dds(4, 4)),
                (
                    "face.xml",
                    br#"<config><model level="0" type="parts" path="./hat.model" material="./u0XXXp1.mtl"/><model level="0" type="parts" path="./u0714g1.model" material="./u0XXXp1.mtl"/><model level="0" type="parts" path="./U0714G1.model" material="./hat.mtl"/><model level="0" type="parts" path="model/character/uniform/common/797/u0714p1.model" material="./hat.mtl"/></config>"#.to_vec(),
                ),
            ],
        );
        let refused = |reference: &str| ContentFinding {
            code: "kit_token_old",
            scope: folder("Players/05 - A"),
            context: vec![
                ("file", "face.xml".to_owned()),
                ("reference", reference.to_owned()),
            ],
            disposition: Disposition::DropFolder,
            pass_through_eligible: false,
        };
        let token_findings: Vec<&ContentFinding> = findings
            .iter()
            .filter(|finding| finding.code == "kit_token_old")
            .collect();
        assert_eq!(
            token_findings,
            [&refused("./u0XXXp1.mtl"), &refused("./u0714g1.model")],
            "{findings:#?}"
        );
        // A path into the game's own folders is read (it has a finding of its own) but is
        // the game's spelling, which no `kit_token_old` names.
        assert!(
            findings
                .iter()
                .any(|finding| finding.code == "xml_common_path_invalid"
                    && finding
                        .context
                        .iter()
                        .any(|(_, value)| value
                            == "model/character/uniform/common/797/u0714p1.model")),
            "{findings:#?}"
        );
    }

    #[test]
    fn a_model_an_xml_lists_is_compared_with_the_mtl_its_entry_names_not_the_searched_one() {
        let findings = slot_05_findings(
            "deep_xml_material",
            &[
                ("hat.model", card()),
                ("hat.mtl", card_materials()),
                ("other.mtl", other_materials()),
                ("texture.dds", bc1_dds(4, 4)),
                (
                    "face.xml",
                    one_model_xml(r#"path="./hat.model" material="./other.mtl""#),
                ),
            ],
        );
        let undefined = ContentFinding {
            code: "model_material_undefined",
            scope: folder("Players/05 - A"),
            context: vec![
                ("file", "hat.model".to_owned()),
                ("mtl", "other.mtl".to_owned()),
                ("materials", "card".to_owned()),
            ],
            disposition: Disposition::DropFolder,
            pass_through_eligible: true,
        };
        assert_eq!(findings, std::slice::from_ref(&undefined));
        // A Warning on the xml keeps the folder, so the comparison still runs.
        let findings = slot_05_findings(
            "deep_xml_material_warned",
            &[
                ("hat.model", card()),
                ("hat.mtl", card_materials()),
                ("other.mtl", other_materials()),
                ("texture.dds", bc1_dds(4, 4)),
                (
                    "face.xml",
                    br#"<config><model level="0" type="cape" path="./hat.model" material="./other.mtl"/></config>"#.to_vec(),
                ),
            ],
        );
        assert_eq!(
            findings,
            [
                keeping("xml_type_unknown", "Players/05 - A", &[("type", "cape")]),
                undefined,
            ]
        );
    }

    #[test]
    fn a_reference_to_a_linked_face_s_model_is_found_by_the_deep_pass() {
        let temp = scratch("deep_xml_linked_face");
        let findings_on = |version| {
            findings_for(
                version,
                temp.path(),
                &[
                ("Players/05 - A/hat.model", card()),
                ("Players/05 - A/hat.mtl", card_materials()),
                ("Players/05 - A/Round.face", Vec::new()),
                (
                    "Players/05 - A/face.xml",
                    br#"<config><model level="0" type="parts" path="./hat.model" material="./hat.mtl"/><model level="0" type="parts" path="./hair_high.model" material="./hair_high.mtl"/></config>"#.to_vec(),
                ),
                ("Faces/Round/hair_high.model", card()),
                ("Faces/Round/hair_high.mtl", card_materials()),
                // The texture both `.mtl` files name: the player's through his linked face.
                ("Faces/Round/texture.dds", bc1_dds(4, 4)),
            ],
            &[],
            &[],
            )
        };
        // No `xml_model_not_found` and no `model_material_undefined`: the reference is found.
        // His own xml beside the link is the one finding.
        assert_eq!(
            findings_on(PesVersion::Pes17),
            [ContentFinding {
                code: "xml_shared_face_conflict",
                scope: IssueScope::Folder(ScopePath::new("Players/05 - A").unwrap()),
                context: vec![
                    ("file", "face.xml".to_owned()),
                    ("link", "Faces/Round".to_owned()),
                ],
                disposition: Disposition::DropFolder,
                pass_through_eligible: false,
            }]
        );
        // Fox ignores every `face.xml` (`xml_ignored_fox`, a validation finding): no conflict.
        assert_eq!(findings_on(PesVersion::Pes21), []);
    }

    #[test]
    fn an_xml_naming_an_absent_model_drops_its_folder_and_compares_no_material() {
        let findings = slot_05_findings(
            "deep_xml_absent",
            &[
                ("hat.model", card()),
                // Both name `./texture.dds`, which the folder lacks: with the xml's Error
                // dropping the folder, their textures are not looked for either.
                ("hat.mtl", card_materials()),
                ("other.mtl", other_materials()),
                // The second entry lists `hat.model` with a `.mtl` lacking its material: with
                // the xml's Error dropping the folder, it is not compared.
                (
                    "face.xml",
                    br#"<config><model level="0" type="parts" path="./absent.model" material="./other.mtl"/><model level="0" type="parts" path="./hat.model" material="./other.mtl"/></config>"#.to_vec(),
                ),
            ],
        );
        assert_eq!(
            findings,
            [dropping(
                "xml_model_not_found",
                "Players/05 - A",
                &[("attribute", "path"), ("value", "./absent.model")]
            )]
        );
    }

    #[test]
    fn a_model_the_xml_does_not_list_is_unlisted_and_not_compared() {
        let unlisted = [keeping(
            "xml_model_unlisted",
            "Players/05 - A",
            &[("file", "hat.model")],
        )];
        // Neither the model nor the `.mtl` the xml does not name is read: four bytes no reader
        // accepts give no finding.
        let findings = slot_05_findings(
            "deep_xml_unlisted",
            &[
                ("hat.model", b"junk".to_vec()),
                ("hat.mtl", b"junk".to_vec()),
                ("other.model", card()),
                ("other.mtl", card_materials()),
                ("texture.dds", bc1_dds(4, 4)),
                (
                    "face.xml",
                    one_model_xml(r#"path="./other.model" material="./other.mtl""#),
                ),
            ],
        );
        assert_eq!(findings, unlisted);
        // With no `.mtl` at all, the search would leave both models every material undefined.
        let findings = slot_05_findings(
            "deep_xml_unlisted_no_mtl",
            &[
                ("hat.model", b"junk".to_vec()),
                ("other.model", card()),
                ("face.xml", one_model_xml(r#"path="./other.model""#)),
            ],
        );
        assert_eq!(findings, unlisted);
    }

    #[test]
    fn a_set_the_xml_names_is_read_whole() {
        // `./pants_kitN.model` names every variant of the set: kit 2's is read too.
        let findings = slot_05_findings(
            "deep_xml_set",
            &[
                ("pants_kit1.model", card()),
                ("pants_kit2.model", b"junk".to_vec()),
                ("pants.mtl", card_materials()),
                ("texture.dds", bc1_dds(4, 4)),
                (
                    "face.xml",
                    one_model_xml(r#"path="./pants_kitN.model" material="./pants.mtl""#),
                ),
            ],
        );
        assert_eq!(
            findings,
            [dropping(
                "model_broken",
                "Players/05 - A",
                &[
                    ("file", "pants_kit2.model"),
                    ("error", "model is truncated")
                ]
            )]
        );
    }

    #[test]
    fn an_xml_that_cannot_be_read_drops_its_folder_with_one_finding_and_compares_no_material() {
        // The model has no `.mtl`: the search would make it `model_material_undefined`.
        let broken = |name: &str, xml: &[u8]| {
            slot_05_findings(name, &[("hat.model", card()), ("face.xml", xml.to_vec())])
        };
        assert_eq!(
            broken("deep_xml_broken", b"<config><model"),
            [dropping(
                "xml_broken",
                "Players/05 - A",
                &[
                    ("file", "face.xml"),
                    (
                        "error",
                        "the file is not well-formed XML: the root node was opened but never closed at 1:15"
                    ),
                ]
            )]
        );
        assert_eq!(
            broken("deep_xml_root", b"<cfg/>"),
            [dropping(
                "xml_root_tag_invalid",
                "Players/05 - A",
                &[("file", "face.xml"), ("root", "cfg")]
            )]
        );
        assert_eq!(
            broken("deep_xml_dif", b"<config><dif>RkFD</dif></config>"),
            [dropping(
                "face_diff_invalid",
                "Players/05 - A",
                &[
                    ("file", "face.xml"),
                    (
                        "reason",
                        "the face diff is 3 bytes long, shorter than its 80-byte header"
                    ),
                ]
            )]
        );
    }

    #[test]
    fn an_xml_is_read_in_face_but_not_under_ingame_face_nor_on_fox() {
        let xml = b"<config><model/></config>".to_vec();
        let missing = [
            dropping(
                "xml_model_type_missing",
                "Players/05 - A",
                &[("entry", "1")],
            ),
            dropping(
                "xml_model_path_missing",
                "Players/05 - A",
                &[("entry", "1")],
            ),
        ];
        assert_eq!(
            slot_05_findings("deep_xml_in_face", &[("face.xml", xml.clone())]),
            missing
        );
        assert_eq!(
            slot_05_findings(
                "deep_xml_marked",
                &[("ingame_face.toml", Vec::new()), ("face.xml", xml.clone())]
            ),
            []
        );
        let temp = scratch("deep_xml_fox");
        assert_eq!(
            findings_for(
                PesVersion::Pes21,
                temp.path(),
                &[("Players/05 - A/face.xml", xml)],
                &[],
                &[]
            ),
            []
        );
    }

    #[test]
    fn portraits_stand_with_their_folder_and_after_common_and_the_logo_last() {
        let temp = scratch("deep_portrait_order");
        let findings = findings_of(
            temp.path(),
            &[
                ("logo.png", b"not an image".to_vec()),
                ("Kits/p1/kit_back.png", texture("tiny.png")),
                ("Kits/p1/kit.png", texture("kit.png")),
                ("Portraits/07.png", texture("tiny.png")),
                ("Common/hair.png", texture("tiny.png")),
                ("Players/05 - B/portrait.png", texture("tiny.png")),
                ("Players/03 - A/portrait.png", texture("tiny.png")),
                ("Players/03 - A/skin.png", texture("tiny.png")),
            ],
            &[],
        );
        let order: Vec<(&str, &str)> = findings
            .iter()
            .map(|finding| (finding.code, finding.context[0].1.as_str()))
            .collect();
        assert_eq!(
            order,
            [
                ("texture_too_small", "skin.png"),
                ("texture_too_small", "portrait.png"),
                ("texture_too_small", "portrait.png"),
                ("texture_too_small", "hair.png"),
                ("texture_too_small", "07.png"),
                ("texture_too_small", "kit_back.png"),
                ("logo_file_invalid", "logo.png"),
            ]
        );
        assert_eq!(
            findings[1].scope,
            IssueScope::File(path("Players/03 - A/portrait.png"))
        );
        assert_eq!(
            findings[2].scope,
            IssueScope::File(path("Players/05 - B/portrait.png"))
        );
    }

    /// The card head's material set with one material per entry of `materials`, each the card
    /// material renamed, its one sampler repeated once per path it names.
    fn materials_naming(materials: &[(&str, &[&str])]) -> Vec<u8> {
        use pes_model::format::mtl::{MaterialEntry, MaterialSet};
        let mut set = MaterialSet::read(&card_materials()).unwrap();
        let card = set.materials[0].clone();
        let MaterialEntry::Sampler(sampler) = &card.entries[0] else {
            panic!("the card material's first entry is its sampler");
        };
        set.materials = materials
            .iter()
            .map(|(name, paths)| {
                let mut material = card.clone();
                material.name = (*name).to_owned();
                let samplers = paths.iter().map(|path| {
                    let mut named = sampler.clone();
                    named.path = (*path).to_owned();
                    MaterialEntry::Sampler(named)
                });
                material.entries.splice(0..1, samplers);
                material
            })
            .collect();
        set.write()
    }

    /// The texture finding `code` of the `.mtl` named `file` on `scope`, about `texture`,
    /// named by `materials`; every texture finding keeps what holds the file.
    pub(super) fn mtl_texture(
        code: &'static str,
        scope: IssueScope,
        file: &str,
        texture: &str,
        materials: &str,
    ) -> ContentFinding {
        ContentFinding {
            code,
            scope,
            context: vec![
                ("file", file.to_owned()),
                ("texture", texture.to_owned()),
                ("materials", materials.to_owned()),
            ],
            disposition: Disposition::Keep,
            pass_through_eligible: false,
        }
    }

    #[test]
    fn a_shared_face_s_mtl_counts_its_own_textures_and_a_player_s_his_linked_face_s() {
        let temp = scratch("deep_mtl_shared_face");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                ("Players/05 - A/Round.face", Vec::new()),
                ("Players/05 - A/face_high.model", card()),
                (
                    "Players/05 - A/face_high.mtl",
                    materials_naming(&[("card", &["./round.dds"])]),
                ),
                ("Players/05 - A/skin.dds", bc1_dds(4, 4)),
                ("Faces/Round/hair_high.model", card()),
                (
                    "Faces/Round/hair_high.mtl",
                    materials_naming(&[("card", &["./skin.dds"])]),
                ),
                ("Faces/Round/round.dds", bc1_dds(4, 4)),
            ],
            &[],
            &[],
        );
        // A shared face is complete by itself: the player's `skin.dds` does not count for it.
        assert_eq!(
            findings,
            [mtl_texture(
                "mtl_texture_not_found",
                folder("Faces/Round"),
                "hair_high.mtl",
                "./skin.dds",
                "card"
            )]
        );
    }

    #[test]
    fn a_pre_fox_shared_folder_s_mtl_link_is_unread_its_model_s_materials_undefined() {
        // A shared folder's `.mtl.common` link has no role on pre-Fox: the `.mtl` search sees
        // no `Common/` file for it (`pairings`), so its `.model` has no material set at all.
        let temp = scratch("deep_shared_mtl_link");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                ("Players/05 - A/Crocs.boots", Vec::new()),
                ("Boots/Crocs/boots.model", card()),
                ("Boots/Crocs/boots.mtl.common", Vec::new()),
                (
                    "Common/boots.mtl",
                    materials_naming(&[("card", &["./skin.dds"])]),
                ),
                ("Common/skin.dds", bc1_dds(4, 4)),
            ],
            &[],
            &[],
        );
        assert_eq!(
            findings,
            [dropping(
                "model_material_undefined",
                "Boots/Crocs",
                &[("file", "boots.model")]
            )]
        );
    }

    /// The player folder `player` holding the card head's `face_high.model`, a `face_high.mtl`
    /// naming `./studs.dds` and a link to `Boots/Studs`, which holds the card head as
    /// `boots.model`, a `boots.mtl` naming `./studs.dds`, and `studs.dds` itself.
    fn studs_boots_layout(player: &str) -> Vec<(String, Vec<u8>)> {
        let studs = || materials_naming(&[("card", &["./studs.dds"])]);
        vec![
            (format!("{player}/Studs.boots"), Vec::new()),
            (format!("{player}/face_high.model"), card()),
            (format!("{player}/face_high.mtl"), studs()),
            ("Boots/Studs/boots.model".to_owned(), card()),
            ("Boots/Studs/boots.mtl".to_owned(), studs()),
            ("Boots/Studs/studs.dds".to_owned(), bc1_dds(4, 4)),
        ]
    }

    /// `files` with borrowed paths, as the passes take them.
    fn borrowed(files: &[(String, Vec<u8>)]) -> Vec<(&str, Vec<u8>)> {
        files
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.clone()))
            .collect()
    }

    #[test]
    fn a_referee_s_face_holds_his_combined_boots_textures_and_a_team_player_s_does_not() {
        // A referee's every link feeds his own package, so his face packs the boots' textures.
        let temp = scratch("deep_mtl_referee_boots");
        let pass = export_pass(
            "refs Midcup Deep",
            Some(b"01 Ref A\n"),
            PesVersion::Pes17,
            temp.path(),
            &borrowed(&studs_boots_layout("Players/Ref A")),
            &[],
            &[],
        );
        assert_eq!(pass.findings, []);
        // A team player's boots link stays plain: his face holds no `studs.dds`.
        let temp = scratch("deep_mtl_team_boots");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &borrowed(&studs_boots_layout("Players/05 - A")),
            &[],
            &[],
        );
        assert_eq!(
            findings,
            [mtl_texture(
                "mtl_texture_not_found",
                folder("Players/05 - A"),
                "face_high.mtl",
                "./studs.dds",
                "card"
            )]
        );
    }

    #[test]
    fn the_stem_a_texture_link_stands_for_is_supplied() {
        let temp = scratch("deep_mtl_texture_link");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                ("Players/05 - A/face_high.model", card()),
                (
                    "Players/05 - A/face_high.mtl",
                    materials_naming(&[("card", &["./hair.dds"])]),
                ),
                ("Players/05 - A/hair.dds.common", Vec::new()),
                ("Common/hair.dds", bc1_dds(4, 4)),
            ],
            &[],
            &[],
        );
        assert_eq!(findings, []);
    }

    #[test]
    fn a_mtl_no_parsed_model_binds_reports_its_misses_as_info() {
        let temp = scratch("deep_mtl_unpaired");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                ("Players/05 - A/face_high.model", card()),
                ("Players/05 - A/face_high.mtl", card_materials()),
                ("Players/05 - A/texture.dds", bc1_dds(4, 4)),
                // No model's search finds it.
                (
                    "Players/05 - A/spare.mtl",
                    materials_naming(&[("card", &["./gone.dds"])]),
                ),
                // The model does not parse, so it binds nothing.
                ("Players/06 - B/face_high.model", b"not a model".to_vec()),
                ("Players/06 - B/face_high.mtl", card_materials()),
            ],
            &[],
            &[],
        );
        let error = pes_model::format::PreFoxModel::read(b"not a model")
            .unwrap_err()
            .to_string();
        assert_eq!(
            findings,
            [
                mtl_texture(
                    "mtl_texture_unused_missing",
                    folder("Players/05 - A"),
                    "spare.mtl",
                    "./gone.dds",
                    "card"
                ),
                dropping(
                    "model_broken",
                    "Players/06 - B",
                    &[("file", "face_high.model"), ("error", error.as_str())]
                ),
                mtl_texture(
                    "mtl_texture_unused_missing",
                    folder("Players/06 - B"),
                    "face_high.mtl",
                    "./texture.dds",
                    "card"
                ),
            ]
        );
    }

    #[test]
    fn a_material_the_model_lists_but_no_mesh_binds_is_not_mesh_used() {
        let temp = scratch("deep_mtl_listed_unused");
        let file = pes_model::format::PreFoxModel::read(&card()).unwrap();
        let mut model = pes_model::model::Model::from_file(&file).unwrap();
        model.materials.push("spare".to_owned());
        let listing_spare = model.to_file().unwrap().write().unwrap();
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                ("Players/05 - A/face_high.model", listing_spare),
                (
                    "Players/05 - A/face_high.mtl",
                    materials_naming(&[("card", &["./skin.dds"]), ("spare", &["./gone.dds"])]),
                ),
                ("Players/05 - A/skin.dds", bc1_dds(4, 4)),
            ],
            &[],
            &[],
        );
        assert_eq!(
            findings,
            [
                counted(
                    "model_material_unused",
                    &folder("Players/05 - A"),
                    "face_high.model",
                    1,
                    Disposition::Keep,
                    false
                ),
                mtl_texture(
                    "mtl_texture_unused_missing",
                    folder("Players/05 - A"),
                    "face_high.mtl",
                    "./gone.dds",
                    "spare"
                ),
            ]
        );
    }

    #[test]
    fn a_common_mtl_s_used_materials_are_the_common_models_and_its_miss_is_a_warning_on_the_file() {
        let temp = scratch("deep_mtl_common");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                ("Common/legs.model", card()),
                (
                    "Common/legs.mtl",
                    materials_naming(&[("card", &["./gone.dds"])]),
                ),
                (
                    "Common/spare.mtl",
                    materials_naming(&[("other", &["./lost.dds"])]),
                ),
                // Its search finds `legs.mtl`, still kept, which defines `card`: nothing here.
                ("Players/05 - B/legs.model.common", Vec::new()),
            ],
            &[],
            &[],
        );
        assert_eq!(
            findings,
            [
                mtl_texture(
                    "mtl_texture_not_found",
                    IssueScope::File(path("Common/legs.mtl")),
                    "legs.mtl",
                    "./gone.dds",
                    "card"
                ),
                mtl_texture(
                    "mtl_texture_unused_missing",
                    IssueScope::File(path("Common/spare.mtl")),
                    "spare.mtl",
                    "./lost.dds",
                    "other"
                ),
            ]
        );
    }

    #[test]
    fn a_common_mtl_s_path_below_its_directory_is_looked_for_at_that_path_in_common() {
        let temp = scratch("deep_mtl_common_below");
        let mtl = (
            "Common/jessie/hair.mtl",
            materials_naming(&[("card", &["./sub/hair.dds"])]),
        );
        let files = [
            ("Common/jessie/legs.model", card()),
            mtl.clone(),
            ("Common/jessie/sub/hair.dds", bc1_dds(4, 4)),
        ];
        let findings = findings_for(PesVersion::Pes17, temp.path(), &files, &[], &[]);
        assert_eq!(findings, [], "{findings:?}");

        let without = [
            ("Common/jessie/legs.model", card()),
            mtl,
            // `jessie/`'s own `hair.dds` is not at the path `./sub/` names.
            ("Common/jessie/hair.dds", bc1_dds(4, 4)),
        ];
        let findings = findings_for(PesVersion::Pes17, temp.path(), &without, &[], &[]);
        assert_eq!(
            findings,
            [mtl_texture(
                "mtl_texture_not_found",
                IssueScope::File(path("Common/jessie/hair.mtl")),
                "hair.mtl",
                "./sub/hair.dds",
                "card"
            )]
        );
    }

    #[test]
    fn on_fox_a_common_mtl_resolves_its_textures_from_its_own_common_directory() {
        // `Common/jessie/x.mtl`, which `legs.model.common`'s search lands on, names
        // `./hair.dds`: it resolves in `jessie/`, whose texture the model link brings.
        let temp = scratch("deep_fox_common_mtl_directory");
        let findings = findings_for(
            PesVersion::Pes21,
            temp.path(),
            &[
                ("Players/05 - B/jessie/legs.model.common", Vec::new()),
                ("Common/jessie/legs.model", card()),
                (
                    "Common/jessie/x.mtl",
                    materials_naming(&[("card", &["./hair.dds"])]),
                ),
                ("Common/jessie/hair.dds", bc1_dds(4, 4)),
            ],
            &[],
            &[],
        );
        assert_eq!(findings, [], "{findings:#?}");
    }

    #[test]
    fn on_fox_a_player_mtl_s_common_path_into_a_subfolder_resolves_at_that_path() {
        // `face_high.mtl` names a pre-Fox Common path into `Common/jessie/`; the texture
        // link keeps `Common/jessie/skin.dds`, which supplies it at that path.
        let temp = scratch("deep_fox_common_path_sub");
        let findings = findings_for(
            PesVersion::Pes21,
            temp.path(),
            &[
                ("Players/05 - B/face_high.model", card()),
                (
                    "Players/05 - B/face_high.mtl",
                    materials_naming(&[("card", &["team_common_folder/jessie/skin.dds"])]),
                ),
                ("Players/05 - B/jessie/skin.dds.common", Vec::new()),
                ("Common/jessie/skin.dds", bc1_dds(4, 4)),
            ],
            &[],
            &[],
        );
        assert_eq!(findings, [], "{findings:#?}");
    }

    #[test]
    fn on_fox_a_player_mtl_s_common_path_into_a_subfolder_is_missing_without_the_texture() {
        // `Common/jessie/` holds a kept model and `.mtl` (the kept files a non-texture
        // filter would wrongly feed the lookup) but no texture at `jessie/skin`.
        let temp = scratch("deep_fox_common_path_sub_missing");
        let findings = findings_for(
            PesVersion::Pes21,
            temp.path(),
            &[
                ("Players/05 - B/face_high.model", card()),
                (
                    "Players/05 - B/face_high.mtl",
                    materials_naming(&[("card", &["team_common_folder/jessie/skin.dds"])]),
                ),
                ("Players/05 - B/jessie/legs.model.common", Vec::new()),
                ("Common/jessie/legs.model", card()),
                (
                    "Common/jessie/other.mtl",
                    materials_naming(&[("card", &["./hair.dds"])]),
                ),
                ("Common/hair.dds", bc1_dds(4, 4)),
            ],
            &[],
            &[],
        );
        assert_eq!(
            findings,
            [mtl_texture(
                "mtl_texture_not_found",
                folder("Players/05 - B"),
                "face_high.mtl",
                "team_common_folder/jessie/skin.dds",
                "card"
            )],
            "{findings:#?}"
        );
    }

    #[test]
    fn on_pre_fox_a_mtl_s_common_path_into_a_subfolder_resolves_at_that_path() {
        // On pre-Fox every kept `Common/` texture packs at its own path, so a Common path
        // into `Common/jessie/` resolves there as on Fox.
        let temp = scratch("deep_prefox_common_path_sub");
        let files = [
            ("Players/05 - B/face_high.model", card()),
            (
                "Players/05 - B/face_high.mtl",
                materials_naming(&[(
                    "card",
                    &["model/character/uniform/common/team_folder/jessie/skin.dds"],
                )]),
            ),
            ("Common/jessie/skin.dds", bc1_dds(4, 4)),
        ];
        let findings = findings_for(PesVersion::Pes17, temp.path(), &files, &[], &[]);
        assert_eq!(findings, [], "{findings:#?}");

        // `Common/skin.dds` is not at `jessie/`'s path, nor is no file at all.
        let without = [
            ("Players/05 - B/face_high.model", card()),
            files[1].clone(),
            ("Common/skin.dds", bc1_dds(4, 4)),
        ];
        for files in [&files[..2], &without[..]] {
            assert_eq!(
                findings_for(PesVersion::Pes17, temp.path(), files, &[], &[]),
                [mtl_texture(
                    "mtl_texture_not_found",
                    folder("Players/05 - B"),
                    "face_high.mtl",
                    "model/character/uniform/common/team_folder/jessie/skin.dds",
                    "card"
                )],
                "{files:?}"
            );
        }
    }

    #[test]
    fn a_beaten_common_model_s_drop_never_passes_through() {
        // `Common/legs.model`, beaten by `Common/legs.fmdl` and never read, is dropped with
        // the broken winner: `pass_through` cannot keep it, since nothing checked the file
        // (`common_model_beaten_dropped`). Kept, it is the link's model in the pairing, so the
        // `.mtl`'s lookup reaches `Common/`'s own textures, and `hair.dds` reports no finding.
        let temp = scratch("deep_beaten_pass_through");
        let listed = written(
            temp.path(),
            &[
                ("Common/legs.fmdl", b"not an fmdl".to_vec()),
                ("Common/legs.model", card()),
                ("Common/hair.dds", bc1_dds(4, 4)),
                ("Players/05 - A/legs.model.common", Vec::new()),
                (
                    "Players/05 - A/x.mtl",
                    materials_naming(&[("card", &["./hair.dds"])]),
                ),
            ],
            &[],
        );
        let (resolved, codes) = resolved_with_issues("co Midcup Deep", &listed, &[], None);
        assert_eq!(codes, Vec::<&str>::new(), "the structure pass's findings");
        let source = ExportSource {
            export_id: ExportId(0),
            path: temp.path().to_path_buf(),
            kind: SourceKind::Folder,
            file_name: "co Midcup Deep".to_owned(),
            display_name: "co Midcup Deep".to_owned(),
            team_name: None,
        };
        let content = ContentSource::new(&source, &MemoryBudget::new(1 << 30));
        let pass = content_findings(&resolved.export, &content, PesVersion::Pes21, None, true);
        let textures: Vec<&ContentFinding> = pass
            .findings
            .iter()
            .filter(|finding| finding.code.starts_with("mtl_texture"))
            .collect();
        assert_eq!(
            textures,
            [&mtl_texture(
                "mtl_texture_unused_missing",
                folder("Players/05 - A"),
                "x.mtl",
                "./hair.dds",
                "card"
            )]
        );
    }
}
