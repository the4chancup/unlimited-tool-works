//! One package of a model folder's Fox models. Its `.model` sources are converted to FMDL, its
//! models renamed to their allowed names, and each part's texture paths pointed at where its
//! textures go, the textures its meshes use being looked for. The parts resolving to one name
//! are merged into one model. The result, with the files that go beside the models, is the
//! package's files, which `materialize` packs or places.

use std::collections::{BTreeMap, BTreeSet};
use std::iter;

use aesthetics_export::{FileKind, ModelFormat, classify};
use fmdl::ops::merge::{MergeError, merge};
use fmdl::ops::paths::{TexturePath, rewrite_texture_paths, used_texture_paths};
use fmdl::{FmdlFile, Model};
use model_convert::formats::fmdl::{fmdl_to_ir, ir_to_fmdl};
use model_convert::ir::CanonicalModel;
use model_convert::ops::hand_split::split_by_skeleton_group;
use pes_version::Engine;
use vtree::ScopePath;

use super::conversion::{ConvertedMaterials, fmdl_for_fox, model_for_fox};
use super::materialize::PackageFiles;
use super::{CompileContext, Finding, TaskFailure, TaskFiles, take};
use crate::bins::installed::InstalledPaths;
use crate::deep::named_on_folder;
use crate::face_diff;
use crate::messages::Code;
use crate::mtl_search::mtl_for;
use crate::paths::{self, TextureDirectory};
use crate::plan::ModelFolder;
use crate::plan::roles::{
    ModelPackage, PlayerFile, below_common, common_folder, common_texture_below, file_stem,
    is_common_file, skeleton_slot,
};
use crate::texture_lookup::{self, TextureFolders, TexturePlace};
use crate::user_face_xml::{Reference, UNIFORM_COMMON, reference};

/// One model of the package: a part of the output model its allowed name names, from the
/// folder's own files, a combined shared folder's, or the export's `Common/` folder through a
/// `.common` link.
struct Part {
    /// The allowed name the part resolves to (`boots`), the output model's.
    name: &'static str,
    /// The part's export path, which with its file name orders the parts of one output.
    path: ScopePath,
    /// The part's source folder (`Players/05 - A`, `Faces/Round`): its files stand at their
    /// paths below it, so `references_from` is read below it (`texture_lookup`).
    source: ScopePath,
    /// The file whose folder a texture path below it (`./textures/skin`) starts from: the
    /// model itself, or for a `.model` converted for Fox, the `.mtl` it took its materials from
    /// (`mtl_for`'s result), since the path was written in that `.mtl`.
    references_from: ScopePath,
    /// The part's bytes, an FMDL: a `.model`'s converted, a member's FMDL as it is or moved
    /// onto the version's skeleton (`convert_part`).
    bytes: Vec<u8>,
    /// The skeleton paired with the part: the `.skl` of its stem in its own source folder, or
    /// the one its conversion writes.
    skeleton: Option<Vec<u8>>,
    /// Where the part's own textures are packed.
    textures: PartTextures,
}

/// Where a part's own textures go, which its texture paths are rewritten to name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PartTextures {
    /// The folder's texture home: the part is the folder's own or a combined folder's, and
    /// its textures are the folder's textures task's.
    Folder,
    /// The team's Common output: the part is a Common model a `.common` link brings in, or a
    /// folder's `.model` converted with a `Common/` `.mtl` (a `.mtl.common` link, or the search
    /// reaching `Common/`), whose stems resolve in `Common/` (`spec/model_format.md` "Link files": a
    /// stem resolves in the folder of the material file that set it). Its textures stay in
    /// `Common/` and are the export's Common textures task's, never relocated.
    Common,
    /// The folder's texture home for a stem the folder holds, else the team's Common output
    /// for a stem `Common/` holds: a Common `.model` converted with the folder's own `.mtl`,
    /// which the search prefers to `Common/`'s. Its stems resolve in the folder, where its
    /// `.mtl` set them, and one the folder lacks still finds the Common model's own texture.
    CommonSet,
}

/// The files of `folder`'s `package`, compiled from its files' bytes in `files` for team
/// `team_id`, by their names in the package, a file the game needs beside the models that no
/// source holds taken from the run's templates. Every model is first put through
/// `convert_part`: a `.model` with no `.fmdl` of its stem is converted, with the `.mtl` its
/// search finds among its source's files, and so is a Common `.model` a `.common` link brings
/// in, with the `.mtl` planning resolved for it (`ModelFolder::common_material`); an FMDL, a
/// member's own, a shared folder's or a Common one, runs the conversion pre-check with the
/// `.skl` of its stem as its bind pose, and is moved onto the version's skeleton when it is
/// posed off it. The skeleton a conversion writes is the part's, as a member's `.skl` of its
/// stem would be (one beside it of other bytes is `skl_merge_conflict`), and is dropped with
/// no finding for a role with no skeleton slot. A hand-split face part, converted or moved
/// first, gives the face its body and the gloves its hands (`parts_of`), the gloves reading
/// the part's `.skl` for that, before any texture path is rewritten. A merge of several parts
/// into one model is noted in `findings` as `fmdl_merged`. A texture a
/// part's mesh uses that nothing supplies, or whose supply cannot be decided
/// (`texture_supply`), is noted in `findings` as `fmdl_texture_not_found`,
/// once per texture, the package kept. A texture path into
/// the game's kit texture folder is left as written and noted once per path as
/// `fmdl_texture_kit_path` instead: the modded exe swaps the kit textures per match, so a
/// fixed file there is not the kit the player wears.
pub(super) fn package(
    folder: &ModelFolder,
    package: ModelPackage,
    team_id: u16,
    ctx: &CompileContext,
    files: &mut TaskFiles,
    findings: &mut Vec<Finding>,
) -> Result<PackageFiles, TaskFailure> {
    let mut parts: Vec<Part> = Vec::new();
    // The folder's textures by the folder of its tree holding each, and its combined folders',
    // its texture links the same way (`texture_lookup`): a link counts as the `Common/`
    // texture it stands for being present in the folder holding it; and the `below` of each,
    // which a Common part's lookup counts as a `Common/` texture and `texture_supply` held at
    // the team's Common output, validation having checked the link. Kept as spelled, not
    // folded: a `kitF` file's token is read from its spelling (`kit_lookup_keys`).
    let mut textures = TextureFolders::default();
    let mut linked_stems = BTreeSet::new();
    let mut contents = PackageFiles::new();
    // `roles()` yields the folder's own files, then each combined folder's in `combined`'s
    // order, so each source's roles pair with its own file list here: a `.model`'s `.mtl` is
    // searched for among its source's files.
    let source_files = iter::once(&folder.files).chain(
        folder
            .combined
            .iter()
            .map(|combined| &combined.folder.files),
    );
    // A hand-split face part (`ModelFolder::hand_split`) is read by the face task, which keeps
    // its body, and by the gloves task, which keeps its hands, each with the part's `.skl`.
    let reads_hand_split = match package {
        ModelPackage::Face | ModelPackage::Gloves => true,
        ModelPackage::Boots => false,
    };
    // The claim each combined source stands at a path below the folder, kept to the winner
    // `resolve_path` (`processing::texture`) picks: the highest package's in canonical
    // order, a texture `.common` link claiming the path as its folder's copy would, so no
    // model names the losing one.
    let mut claims: BTreeMap<String, Vec<(ModelPackage, bool, String)>> = BTreeMap::new();
    for ((source_package, source_path, source_roles), source_files) in
        folder.roles().into_iter().zip(source_files)
    {
        // A skeleton pairs with the model of its stem in the same directory: keyed by the
        // path up to the extension, case-folded as the file system folds it (planning pairs
        // a Common skeleton with its model folded too), since a player folder's subfolder
        // may hold a model of the same name as one directly in the folder. They are
        // read before the models: a `.skl` may sort after its model, and an FMDL needs its
        // skeleton, its bind pose, for the conversion pre-check.
        let mut skeletons: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for (file, role) in &source_roles {
            if let PlayerFile::Skeleton { package: owner, .. } = role
                && (*owner == package || (reads_hand_split && folder.hand_split_skeleton(file)))
            {
                skeletons.insert(
                    vtree::fold_name(file_stem(file.path.as_str())),
                    take(files, file),
                );
            }
        }
        for (file, role) in source_roles {
            let hand_split = reads_hand_split && folder.hand_split.contains(&file.path);
            let mut part = |name, textures| Part {
                name,
                path: file.path.clone(),
                source: source_path.clone(),
                references_from: file.path.clone(),
                bytes: take(files, file),
                skeleton: skeletons.remove(&vtree::fold_name(file_stem(file.path.as_str()))),
                textures,
            };
            match role {
                PlayerFile::Model {
                    package: owner,
                    name,
                } if owner == package || hand_split => {
                    let mut part = part(name, PartTextures::Folder);
                    let model = named_on_folder(&file.path, &folder.path);
                    let mtl = if file.kind == FileKind::Model(ModelFormat::PesModel) {
                        // A combined source's search resolves its `.mtl.common` links in
                        // `Common/` as the folder's own does: `folder.common_files` holds
                        // every `Common/` file Fox reads (`deep::pairings` searches the same).
                        let mtl = mtl_for(
                            &file.path,
                            source_path,
                            source_files,
                            &folder.common_files,
                        )
                        .expect(
                            "the deep pass drops a folder holding a selected `.model` no `.mtl` \
                             is found for (`model_material_undefined`)",
                        );
                        // A Common set's stems resolve in `Common/`, where the deep pass checked
                        // them: a texture of the folder's of one of those stems shadows nothing.
                        if is_common_file(&mtl.path) {
                            part.textures = PartTextures::Common;
                        }
                        part.references_from = mtl.path.clone();
                        let mtl = files.get(&mtl.path).expect(
                            "a package converting a `.model` reads its source's `.mtl` files \
                             (`TaskKind::files`)",
                        );
                        Some(mtl.as_slice())
                    } else {
                        None
                    };
                    convert_part(&mut part, &model, mtl, owner, package, ctx, findings)?;
                    parts.extend(parts_of(part, &model, hand_split, package, ctx, findings)?);
                }
                // A Common `.model` converts here, in each linking player's task, as his own
                // `.model` does: Fox has no Common model output for it to convert once into,
                // so two players linking it convert it twice, as a Common FMDL is baked into
                // each of their packages.
                PlayerFile::CommonModel {
                    package: owner,
                    name,
                } if owner == package || hand_split => {
                    let mut part = part(name, PartTextures::Common);
                    let model = named_on_folder(&file.path, &folder.path);
                    let mtl = if file.kind == FileKind::Model(ModelFormat::PesModel) {
                        let mtl = folder.common_material(&file.path).expect(
                            "planning resolves a Common `.model`'s `.mtl`, the deep pass having \
                             dropped a folder linking one with none (`model_material_undefined`)",
                        );
                        // The player's own `.mtl`, which the search prefers to `Common/`'s,
                        // set the stems: they are looked for in his folder first.
                        if !is_common_file(&mtl.path) {
                            part.textures = PartTextures::CommonSet;
                        }
                        part.references_from = mtl.path.clone();
                        let mtl = files.get(&mtl.path).expect(
                            "a package converting a Common `.model` reads the `.mtl` planning \
                             resolved for it (`TaskKind::files`)",
                        );
                        Some(mtl.as_slice())
                    } else {
                        None
                    };
                    convert_part(&mut part, &model, mtl, owner, package, ctx, findings)?;
                    parts.extend(parts_of(part, &model, hand_split, package, ctx, findings)?);
                }
                PlayerFile::Packed {
                    package: owner,
                    name,
                } if owner == package => {
                    contents.insert(name.to_owned(), take(files, file));
                }
                // The deep pass has dropped a folder whose face diff fails to decode, or that
                // gives it in both forms, so a failure here is not a member's mistake.
                PlayerFile::FaceDiffXml if package == ModelPackage::Face => {
                    let bytes = face_diff::from_xml(&take(files, file)).map_err(|error| {
                        anyhow::anyhow!("cannot read the face diff {}: {error}", file.path.as_str())
                    })?;
                    contents.insert("face_diff.bin".to_owned(), bytes);
                }
                // The textures are the textures task's, and a linked one the Common textures
                // task's; this task only points its models at them. Stems fold, as validation
                // folds the name a texture claims.
                PlayerFile::Texture { below, .. } => {
                    if source_path == &folder.path {
                        textures.insert(true, &below);
                    } else {
                        claims.entry(vtree::fold_name(&below)).or_default().push((
                            source_package,
                            false,
                            below,
                        ));
                    }
                }
                PlayerFile::CommonTexture(below) => {
                    let below = common_texture_below(&folder.common_files, &below);
                    linked_stems.insert(below.clone());
                    if source_path == &folder.path {
                        textures.insert_link(true, &below);
                    } else {
                        claims.entry(vtree::fold_name(&below)).or_default().push((
                            source_package,
                            true,
                            below,
                        ));
                    }
                }
                PlayerFile::Model { .. }
                | PlayerFile::CommonModel { .. }
                | PlayerFile::SlotlessSkeleton
                | PlayerFile::UnusedFaceFile
                | PlayerFile::LeftOutKitVariant
                | PlayerFile::FaceDiffXml
                | PlayerFile::Packed { .. } => {}
                // Read in place with the `.model` it defines, above.
                PlayerFile::Material => {}
                // Read before the models, above.
                PlayerFile::Skeleton { .. } => {}
                // The search resolves a material link where it finds it (`mtl_for`), and
                // the task reads the Common `.mtl` it names (`TaskKind::files`).
                PlayerFile::CommonMaterial => {}
                // Pre-Fox roles: a Fox target gives no file one.
                PlayerFile::PreFoxModel { .. }
                | PlayerFile::PreFoxPart { .. }
                | PlayerFile::PreFoxCommonModel { .. }
                | PlayerFile::FaceXml
                | PlayerFile::ConversionSkeleton => {}
                // Planning drops a player folder holding one, and `drop_gltf_folders` removes a
                // shared folder whose selected model is one before any task is made, so no
                // task meets it.
                PlayerFile::UnsupportedGltf => {}
            }
        }
    }
    for (_, claimants) in claims {
        let winner = ModelPackage::first_claimed(|package| {
            claimants
                .iter()
                .any(|(candidate, _, _)| *candidate == package)
        })
        .expect("a path's claimants are non-empty");
        let (_, link, below) = claimants
            .into_iter()
            .find(|(package, _, _)| *package == winner)
            .expect("the winning package claims the path");
        if link {
            textures.insert_link(false, &below);
        } else {
            textures.insert(false, &below);
        }
    }

    // The parts of one output model go in alphabetical source order, by file name folded as
    // the file system folds it and then by export path, so a recompile gives the same model.
    parts.sort_by_cached_key(|part| {
        (
            vtree::fold_name(part.path.name()),
            part.path.as_str().to_owned(),
        )
    });
    // The parsed parts, their merges and their written models, charged at the parts' source
    // size, an estimate of each form, until the package's files are built.
    let parts_size = parts.iter().map(|part| part.bytes.len()).sum();
    let _parts_charge = ctx.budget.charge(parts_size);
    let mut by_name: BTreeMap<&'static str, Vec<Part>> = BTreeMap::new();
    for part in parts {
        by_name.entry(part.name).or_default().push(part);
    }

    // A folder's textures sit in its one texture home, once however many ids the package is
    // emitted under: every copy of the model points at that one location, a texture name
    // resolving nearest first from the part's own folder (`texture_lookup`), a texture link
    // counting as the Common texture it stands for in the folder holding it. A texture resolved
    // in Common, a Common part's own or one a folder's link stands for, stays in the team's
    // Common output, where the export's Common textures task puts it once for every player: a
    // texture resolved in Common is never relocated. A folder part
    // looks in the folder's places first, and a path of its into the team's pre-Fox Common
    // folder reaches `Common/`'s (`point_texture`); one converted with a `Common/` `.mtl` looks
    // in `Common/`'s alone (`PartTextures::Common`), and a Common part converted with the
    // folder's own `.mtl` in the folder's, then in `Common/`'s nearest first from the
    // model's own directory (`PartTextures::CommonSet`).
    // Validation refuses a player folder holding a texture and a link of one stem in one of its
    // folders (`texture_stem_conflict`), but not a link beside a combined shared folder's
    // texture of its stem: there the link wins, as the folder's own texture of its stem does.
    // A texture named with a path below its file's folder (`./textures/skin`) resolves at that
    // path alone (`point_texture_below`), for a Common part at that path below its own file's
    // directory in `Common/`.
    let texture_directory = folder.textures.directory(ctx.version.engine(), team_id);
    let common_home = paths::common_home(Engine::Fox, team_id);
    // The Common textures Fox packs, each at its path below `Common/` without its extension:
    // `Common/`'s own (`common_texture_stems`) and the subfolder ones a link reaches
    // (`common_files`). A Common part's stems resolve among them as a `Common/` `.mtl`'s do
    // (`prefox_common::common_models`): `Common/` the model folder, a name nearest first from
    // the file's own directory up to it, a path below the file's directory at that path.
    let common_scope = common_folder();
    let mut common_folders = TextureFolders::default();
    for stem in &folder.common_texture_stems {
        common_folders.insert(true, stem);
    }
    for file in &folder.common_files {
        if file.kind == FileKind::Texture {
            common_folders.insert(true, file_stem(below_common(&file.path)));
        }
    }
    // And each texture a link of the folder's stands for, an installed CPK's included, as its
    // own file: a real `Common/` texture of its stem resolves first (`nearest_first` gives a
    // folder's textures before its links).
    for below in &linked_stems {
        common_folders.insert_link(true, below);
    }
    // A Common part's places (`FolderPlaces::common`, the pre-Fox shape): every place is the
    // team's Common output, so `common_home` stands for both homes. `path` is the file whose
    // references they serve: the `.mtl` that set the stems, or the Common model's own path
    // for `CommonSet`'s fallback, whose `.mtl` sits in the folder, not below `Common/`.
    let common_places =
        |path: &ScopePath| common_folders.places(&common_scope, path, &common_home, &common_home);
    // A folder part's places: its folder's textures and links nearest first, each in its
    // home, resolved from the material file's folder (`Part::references_from`) below its own
    // source folder: a stem resolves in the folder of the material file that set it
    // (`spec/model_format.md` "Link files"), which for a `.model` converted with a `.mtl` of another
    // folder is that `.mtl`'s.
    let folder_places = |part: &Part| {
        textures.places(
            &part.source,
            &part.references_from,
            &texture_directory,
            &common_home,
        )
    };
    let installed_holds = installed_lookup(&ctx.installed, team_id);
    // A texture the part's source does not hold is one of the game's own; its directory names
    // the team as `000`, which becomes the team's id.
    let team_segment = format!("/{team_id}/");
    // The skeleton the package's parts bring. Only the `fcl_hair` and the `boots` parts pair
    // one (`player_file`), so at most one name's parts have any.
    let mut skeleton = None;
    for (name, mut parts) in by_name {
        if let Some(found) = merged_skeleton(&mut parts)? {
            skeleton = Some(found);
        }
        // Each part's paths are rewritten before the merge: which textures are a part's own
        // depends on where the part came from, and the merged model no longer tells its parts
        // apart. One material two parts define over textures that now sit in different
        // directories is the merge's `merge_material_conflict`, as intended.
        let mut models = Vec::with_capacity(parts.len());
        for part in &parts {
            let places = match part.textures {
                PartTextures::Folder => folder_places(part),
                PartTextures::Common => common_places(&part.references_from),
                PartTextures::CommonSet => {
                    let mut places = folder_places(part);
                    places.extend(common_places(&part.path));
                    places
                }
            };
            let mut model = FmdlFile::read(&part.bytes)?;
            // The verdicts the path pointing decides for the part's textures, by the path
            // it writes them to (`texture_supply`): a pointed texture is supplied, and a
            // Common path's is what `point_texture` answered.
            let mut common_supplies = BTreeMap::new();
            rewrite_texture_paths(&mut model, |path| {
                if is_kit_texture_directory(&path.directory) {
                    // The game's kit texture folder: a path into it is kept as written and
                    // reported below (`fmdl_texture_kit_path`), never pointed at a same-stem
                    // file of the folder.
                    return;
                }
                let below = texture_lookup::path_below(&path.directory).map(|subdirectory| {
                    match part.textures {
                        PartTextures::Common => common_folders
                            .at(&common_scope, &part.references_from, subdirectory)
                            .into_iter()
                            .map(|(place, which)| {
                                (place, which.directory(&common_home, &common_home))
                            })
                            .collect::<Vec<_>>(),
                        PartTextures::Folder | PartTextures::CommonSet => textures
                            .at(&part.source, &part.references_from, subdirectory)
                            .into_iter()
                            .map(|(place, which)| {
                                (place, which.directory(&texture_directory, &common_home))
                            })
                            .collect::<Vec<_>>(),
                    }
                });
                let supply = match below {
                    Some(places) => {
                        point_texture_below(path, &places).then_some(TextureSupply::Supplied)
                    }
                    None => point_texture(
                        path,
                        &places,
                        &common_folders,
                        &common_home,
                        &team_segment,
                        &installed_holds,
                    ),
                };
                if let Some(supply) = supply {
                    common_supplies.insert(format!("{}{}", path.directory, path.file_name), supply);
                }
            })?;
            // A converted `.model`'s texture paths are its `.mtl`'s, whose stems nothing
            // supplies are `mtl_texture_not_found`, a warning that keeps the folder.
            let converted = classify(part.path.name()) == FileKind::Model(ModelFormat::PesModel);
            for path in used_texture_paths(&model)? {
                let context = || {
                    vec![
                        ("model", named_on_folder(&part.path, &folder.path)),
                        ("texture", format!("{}{}", path.directory, path.file_name)),
                    ]
                };
                if is_kit_texture_directory(&path.directory) {
                    // Once per path: two entries of the table may name one path.
                    let finding = (Code::FmdlTextureKitPath, context());
                    if !findings.contains(&finding) {
                        findings.push(finding);
                    }
                    continue;
                }
                match texture_supply(
                    &path,
                    &common_supplies,
                    &common_folders,
                    &common_home,
                    &installed_holds,
                ) {
                    TextureSupply::Supplied => {}
                    // A bare stem the `.mtl` check reports already.
                    TextureSupply::Missing if converted && is_bare(&path.directory) => {}
                    // Once per texture: two entries of the table may name one path.
                    TextureSupply::Missing | TextureSupply::Unknown => {
                        let finding = (Code::FmdlTextureNotFound, context());
                        if !findings.contains(&finding) {
                            findings.push(finding);
                        }
                    }
                }
            }
            models.push(model);
        }
        let bytes = match models.as_slice() {
            [model] => model.write(),
            _ => {
                let merged = merge_parts(&models)?;
                findings.push((Code::FmdlMerged, vec![("model", format!("{name}.fmdl"))]));
                merged.write()
            }
        };
        contents.insert(format!("{name}.fmdl"), bytes);
    }
    // The game loads the boots and the hair with a skeleton beside them, under the slot's
    // name: the parts' own when they bring one, else the standard full-body one. A face also
    // needs its `face_diff.bin`, and a hair its `fcl_hair_sim.fclo`: a source's own when one
    // holds it, else the run's template.
    match package {
        ModelPackage::Boots => {
            contents.insert(
                "boots.skl".to_owned(),
                skeleton.unwrap_or_else(|| ctx.templates.body_skeleton().to_vec()),
            );
        }
        ModelPackage::Face => {
            if !contents.contains_key("face_diff.bin") {
                contents.insert(
                    "face_diff.bin".to_owned(),
                    ctx.templates.face_diff().to_vec(),
                );
            }
            if contents.contains_key("fcl_hair.fmdl") {
                if !contents.contains_key("fcl_hair_sim.fclo") {
                    contents.insert(
                        "fcl_hair_sim.fclo".to_owned(),
                        ctx.templates.fcl_hair_sim().to_vec(),
                    );
                }
                contents.insert(
                    "fcl_hair_sim.skl".to_owned(),
                    skeleton.unwrap_or_else(|| ctx.templates.body_skeleton().to_vec()),
                );
            }
        }
        // The gloves have no skeleton slot, and no `.skl` pairs with a glove.
        ModelPackage::Gloves => {}
    }

    Ok(contents)
}

/// The skeleton the parts of one output model share, taken out of them: the one `.skl` every
/// part brings (byte-identical files under several names are one skeleton), or `None` when no
/// part brings one. Parts merged into one model must reference one skeleton, so any other mix,
/// a part with a skeleton beside one without included, is `skl_merge_conflict`.
fn merged_skeleton(parts: &mut [Part]) -> Result<Option<Vec<u8>>, TaskFailure> {
    let mut skeletons = parts.iter_mut().map(|part| part.skeleton.take());
    let Some(first) = skeletons.next() else {
        return Ok(None);
    };
    if skeletons.any(|skeleton| skeleton != first) {
        return Err(skeleton_conflict());
    }
    Ok(first)
}

/// `skl_merge_conflict` for one output model whose parts bring different skeletons.
fn skeleton_conflict() -> TaskFailure {
    TaskFailure {
        code: Code::SklMergeConflict,
        context: vec![("skeleton", "differs".to_owned())],
    }
}

/// Converts `part`, a model that the task's findings name `model`, for the Fox target: a
/// `.model`, with `mtl`, the material set its search found, becomes an FMDL
/// (`conversion::model_for_fox`); an FMDL (`mtl` `None`) is packed as it is unless the
/// pre-check finds it posed off the version's skeleton, when it is moved onto it
/// (`conversion::fmdl_for_fox`). The skeleton the conversion writes takes the path a member's
/// `.skl` of the part's stem would: beside one, they are two skeletons of the part, one when
/// they are equal, and an FMDL moved off the pose its `.skl` describes drops that `.skl` for
/// the conversion's. A part of `owner` under a name with no skeleton slot keeps none, with no
/// `skl_no_slot` finding: the member authored no file. What the conversion reports goes to
/// `findings` only when the task builds `owner`, its `package`: the gloves task converts a
/// hand-split face part again, for its hands, and that is the face's to tell. A conversion
/// that fails, or whose FMDL the game cannot load, fails the task.
fn convert_part(
    part: &mut Part,
    model: &str,
    mtl: Option<&[u8]>,
    owner: ModelPackage,
    package: ModelPackage,
    ctx: &CompileContext,
    findings: &mut Vec<Finding>,
) -> Result<(), TaskFailure> {
    let mut reported = Vec::new();
    match mtl {
        Some(mtl) => {
            let converted = model_for_fox(
                model,
                &part.bytes,
                mtl,
                ctx,
                &mut reported,
                ConvertedMaterials::Converted,
            )?;
            part.bytes = converted.model;
            part.skeleton = match (converted.skeleton, part.skeleton.take()) {
                (Some(converted), Some(member)) if converted != member => {
                    return Err(skeleton_conflict());
                }
                (converted, member) => member.or(converted),
            };
        }
        None => {
            let skeleton = part.skeleton.as_deref();
            if let Some(converted) = fmdl_for_fox(model, &part.bytes, skeleton, ctx, &mut reported)?
            {
                part.bytes = converted.model;
                part.skeleton = converted.skeleton;
            }
        }
    }
    // A member's own `.skl` of a slotless role never gets here (`PlayerFile::SlotlessSkeleton`,
    // reported by validation and never read): only a conversion's can.
    if skeleton_slot(owner, part.name).is_none() {
        part.skeleton = None;
    }
    if owner == package {
        findings.extend(reported);
    }
    Ok(())
}

/// The parts `part`, a model of the folder's that its task's findings name `model`
/// (`deep::named_on_folder`), gives `package`: the part itself, or, when it is a hand-split
/// face part (`hand_split`), what the hand auto-split leaves the package
/// (`spec/model_conversion/hand_split.md` "Pipeline integration"). The face keeps the body,
/// under the part's name and path, so it pairs the part's skeleton, and is told so by
/// `model_hand_split` naming the model and the gloves made; a body left with no face (a model
/// that was all hand) is no part at all. The gloves get a `glove_l` and a `glove_r` part, for
/// each hand the model has, with the part's path and textures and no skeleton, merged with any
/// authored glove of that name like any other part. A model the split cannot read or write
/// fails the task with `model_conversion_failed`.
fn parts_of(
    part: Part,
    model: &str,
    hand_split: bool,
    package: ModelPackage,
    ctx: &CompileContext,
    findings: &mut Vec<Finding>,
) -> Result<Vec<Part>, TaskFailure> {
    if !hand_split {
        return Ok(vec![part]);
    }
    let split = {
        // The split's model, its IR and its written parts, charged at the source's size, an
        // estimate of each form, while they are built.
        let _split_charge = ctx.budget.charge(part.bytes.len());
        split_fmdl(&part.bytes).map_err(|error| TaskFailure {
            code: Code::ModelConversionFailed,
            context: vec![("model", model.to_owned()), ("error", format!("{error:#}"))],
        })?
    };
    let gloves = [("glove_l", split.glove_l), ("glove_r", split.glove_r)];
    match package {
        ModelPackage::Face => {
            let made: Vec<&str> = gloves
                .iter()
                .filter(|(_, bytes)| bytes.is_some())
                .map(|(name, _)| *name)
                .collect();
            findings.push((
                Code::ModelHandSplit,
                vec![("model", model.to_owned()), ("gloves", made.join(", "))],
            ));
            Ok(split
                .body
                .map(|bytes| Part { bytes, ..part })
                .into_iter()
                .collect())
        }
        ModelPackage::Gloves => Ok(gloves
            .into_iter()
            .filter_map(|(name, bytes)| {
                Some(Part {
                    name,
                    path: part.path.clone(),
                    source: part.source.clone(),
                    references_from: part.references_from.clone(),
                    bytes: bytes?,
                    skeleton: None,
                    textures: part.textures,
                })
            })
            .collect()),
        ModelPackage::Boots => {
            unreachable!("only the face and gloves tasks read a hand-split part (`package`)")
        }
    }
}

/// An FMDL after the hand auto-split, each part written back as an FMDL.
struct SplitFmdl {
    /// The model without its hands; `None` when no face is left.
    body: Option<Vec<u8>>,
    /// The left hand, when the model weighs vertices on an `skh_*_l` bone.
    glove_l: Option<Vec<u8>>,
    /// The right hand, when the model weighs vertices on an `skh_*_r` bone.
    glove_r: Option<Vec<u8>>,
}

/// The FMDL `bytes` split at the wrists (`model_convert`'s `split_by_skeleton_group`), through
/// `model_convert`'s IR and back.
fn split_fmdl(bytes: &[u8]) -> anyhow::Result<SplitFmdl> {
    let model = Model::from_file(&FmdlFile::read(bytes)?)?;
    // Imported without the paired `.skl`: measured, the round trip then keeps the FMDL's own
    // bone table, which is what the split keeps. The
    // import's and the export's findings, `model_convert`'s loss codes, are left unreported
    // on purpose: cross-format conversion, a later step, maps those codes for every converted
    // model, and on the measured model the round trip reported only the dropped bone-matrix
    // block, which no community FMDL carries.
    let ir = fmdl_to_ir(&model, None)?.model;
    let split = split_by_skeleton_group(&ir);
    // The export's own `.skl`, for bones the game's template lacks, is not packed: the body
    // keeps the part's paired skeleton, and a glove has no skeleton slot.
    let write = |part: &CanonicalModel| -> anyhow::Result<Vec<u8>> {
        Ok(ir_to_fmdl(part)?.model.to_file()?.write())
    };
    let body = if split.body.meshes.is_empty() {
        None
    } else {
        Some(write(&split.body)?)
    };
    Ok(SplitFmdl {
        body,
        glove_l: split.glove_l.as_ref().map(write).transpose()?,
        glove_r: split.glove_r.as_ref().map(write).transpose()?,
    })
}

/// Points `path`, one texture reference of a part, at where its texture is: the first of
/// `places`, each the textures packed in a directory for the part in lookup order
/// (`texture_lookup::TexturePlace`), that holds its stem — the places filing each kit set's
/// member under its set's `kitN` key too, so a `kitN` reference finds a member and a `kitF`
/// name none — gives the directory, with that texture's own subdirectory
/// below it (`jessie/`), and the texture is supplied. Else a path into the team's pre-Fox
/// Common folder
/// (`team_common_folder/` or `model/character/uniform/common/team_folder/`, how a member's
/// pre-Fox `.mtl` names a texture of
/// `Common/`), read as the deep pass reads it (`user_face_xml::reference`), is pointed at the
/// team's Fox Common output at its subdirectory below `Common/` (`common_home` for a direct
/// one, its `jessie/` subdirectory for `jessie/skin`), whether or not the export holds it
/// there, and its supply is returned, decided here where the reference is resolved
/// (`common_below_supply`). A file directly below `common_folder/`, the game's own, keeps
/// the long
/// form `model/character/uniform/common/`, a short `common_folder/` respelled to it. Any
/// other texture is one
/// of the game's own, whose directory names the team as `000` or `team_folder`, replaced by
/// `team_segment`.
/// The file name is never changed: the game itself respells a reference for the kit picked.
fn point_texture(
    path: &mut TexturePath,
    places: &[(&TexturePlace, &TextureDirectory)],
    common_folders: &TextureFolders,
    common_home: &TextureDirectory,
    team_segment: &str,
    installed_holds: impl Fn(&str) -> Option<bool>,
) -> Option<TextureSupply> {
    let stem = file_stem(&path.file_name);
    let key = vtree::fold_name(stem);
    let place = places.iter().find_map(|(textures, directory)| {
        let below = textures.get(&key).map(String::as_str)?;
        Some(directory.of(texture_lookup::split(below).0))
    });
    let (place, supply) = match place {
        Some(directory) => (Some(directory), Some(TextureSupply::Supplied)),
        None => {
            let written = format!("{}{}", path.directory, path.file_name);
            match reference(&written) {
                Reference::Common { file_name } => {
                    let (subdirectory, _) = texture_lookup::split(&file_name);
                    let supply = common_below_supply(
                        file_stem(&file_name),
                        common_folders,
                        &installed_holds,
                    );
                    (Some(common_home.of(subdirectory)), Some(supply))
                }
                // The game's own file, its short `common_folder/` spelling written as the
                // long one.
                Reference::GameCommon(_) => (Some(UNIFORM_COMMON.to_owned()), None),
                Reference::Local(_) | Reference::CommonInvalid | Reference::Unchecked(_) => {
                    (None, None)
                }
            }
        }
    };
    path.directory = match place {
        Some(directory) => directory,
        None => team_directory(&path.directory, team_segment),
    };
    supply
}

/// `directory` with its team's folder segment replaced by `team_segment` (`/714/`): `000`,
/// which the compiler's own template models name the team by (a member's `000` below the
/// Fox common directory is `fmdl_common_path_invalid` before this runs, so the replacement
/// serves the templates alone), or `team_folder` in any letter case, the
/// Studio spelling. Any other segment stays as it is: a member never writes a directory
/// the path table holds, so none is refused.
fn team_directory(directory: &str, team_segment: &str) -> String {
    let id = team_segment.trim_matches('/');
    directory
        .replace("/000/", team_segment)
        .split('/')
        .map(|segment| {
            if segment.eq_ignore_ascii_case("team_folder") {
                id
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Points `path`, a texture reference naming a path below its file's folder
/// (`texture_lookup::path_below`), at the texture of the first of `places` holding its stem
/// (`TextureFolders::at`: the folder that path names, its textures then its links, each with
/// the directory its home names it by), returning whether one did. With none it is left as
/// written, which `texture_supply` calls missing: such a path resolves there alone
/// (`spec/model_format.md` "Stem-based texture references").
fn point_texture_below(
    path: &mut TexturePath,
    places: &[(&TexturePlace, &TextureDirectory)],
) -> bool {
    let stem = file_stem(&path.file_name);
    let found = places.iter().find_map(|(place, directory)| {
        let below = place.get(&vtree::fold_name(stem)).map(String::as_str)?;
        Some((directory, below))
    });
    let Some((directory, below)) = found else {
        return false;
    };
    path.directory = directory.of(texture_lookup::split(below).0);
    true
}

/// Whether the texture `below`, its path below `Common/` without its extension
/// (`jessie/hair`), is supplied: the Common textures Fox packs at the subdirectory `below`
/// names (`common_folders`' places there, `""` for `Common/` itself, holding the stem —
/// the places file each kit set's member under its set's `kitN` key too, so a `kitN`
/// subdirectory or reference finds a member and a `kitF` path none), else what
/// `installed_holds` answers for `below` (`None` when the installed CPKs cannot be looked
/// in).
fn common_below_supply(
    below: &str,
    common_folders: &TextureFolders,
    installed_holds: &impl Fn(&str) -> Option<bool>,
) -> TextureSupply {
    let (subdirectory, name) = texture_lookup::split(below);
    let key = vtree::fold_name(name);
    let held = common_folders
        .folder(subdirectory)
        .iter()
        .any(|(place, _)| place.contains_key(&key));
    if held {
        return TextureSupply::Supplied;
    }
    match installed_holds(below) {
        Some(true) => TextureSupply::Supplied,
        Some(false) => TextureSupply::Missing,
        None => TextureSupply::Unknown,
    }
}

/// Whether `directory`, a texture path's directory, is the game's kit texture folder
/// (`paths::KIT_TEXTURE_DIRECTORY`) or below it, folded as texture directories compare
/// (`TextureDirectory::subdirectory`).
fn is_kit_texture_directory(directory: &str) -> bool {
    vtree::fold_name(directory).starts_with(&vtree::fold_name(paths::KIT_TEXTURE_DIRECTORY))
}

/// Whether a texture one of a part's meshes uses is supplied: a player whose texture nobody
/// supplies does not look as its author meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextureSupply {
    /// Supplied, or not looked for: a `dummy_` stem, which the game substitutes, or a path
    /// outside the team's Common output (the folder's own textures, the game's own).
    Supplied,
    /// Neither the export nor an installed CPK loaded before the run's holds it.
    Missing,
    /// The export does not hold it, and the installed CPKs cannot be looked in.
    Unknown,
}

/// Whether an FMDL texture directory is bare, `./` or empty: the stem is a name, looked up
/// nearest first from the file's folder (`spec/model_format.md` "Stem-based texture
/// references").
fn is_bare(directory: &str) -> bool {
    directory.is_empty() || directory == "./"
}

/// Whether the texture at `path`, already pointed where it goes, is supplied: not looked for
/// when its stem starts with `dummy_`; what `supplies` says for it, the verdicts the path
/// pointing returned for it (`point_texture`, `point_texture_below`), keyed by the path it
/// wrote; missing when its directory still names a path below its file's folder
/// (`texture_lookup::path_below`: no texture sat there), a bare directory (`is_bare`: the
/// file's own folder, where the pointing found nothing) counting as such a path. A path whose
/// directory is a
/// `common_home` spelling, `""` for the Common texture directory itself or a subdirectory
/// below `Common/` (`jessie/`; a player's own texture home has the same shape), resolves
/// among the Common textures packed at that subdirectory (`common_below_supply`): the
/// export's `Common/` textures and the paths the folder's texture links stand for, then the
/// installed CPKs' answer (`None`: they cannot be looked in); not looked for when its
/// directory is no `of` spelling of `common_home` (the game's own). Directories and stems
/// compare folded.
fn texture_supply(
    path: &TexturePath,
    supplies: &BTreeMap<String, TextureSupply>,
    common_folders: &TextureFolders,
    common_home: &TextureDirectory,
    installed_holds: impl Fn(&str) -> Option<bool>,
) -> TextureSupply {
    let stem = file_stem(&path.file_name);
    if vtree::fold_name(stem).starts_with("dummy_") {
        return TextureSupply::Supplied;
    }
    if let Some(supply) = supplies.get(&format!("{}{}", path.directory, path.file_name)) {
        return *supply;
    }
    if texture_lookup::path_below(&path.directory).is_some() {
        return TextureSupply::Missing;
    }
    // A bare `./` or empty directory names the file's own folder, where the
    // pointing found nothing: the stem resolves nowhere.
    if is_bare(&path.directory) {
        return TextureSupply::Missing;
    }
    match common_home.subdirectory(&path.directory) {
        Some(subdirectory) => common_below_supply(
            &format!("{subdirectory}{stem}"),
            common_folders,
            &installed_holds,
        ),
        // A home of another shape: the game's own files, not looked for.
        None => TextureSupply::Supplied,
    }
}

/// `texture_supply`'s question to the `installed` CPKs for team `team_id`: whether the
/// Common texture stems they hold include the folded stem a path below `Common/` names
/// (`common_texture_stems` files each installed texture under its own folded path and its
/// `kitN` key, so an installed `kit1/` copy supplies `kitN/`); `None` when they cannot be
/// looked in. Their Common stems are gathered once, for every texture it is asked.
fn installed_lookup(
    installed: &InstalledPaths,
    team_id: u16,
) -> impl Fn(&str) -> Option<bool> + '_ {
    let stems = installed.common_texture_stems(Engine::Fox, team_id);
    move |stem| match installed {
        InstalledPaths::Unknown => None,
        InstalledPaths::Known(_) => Some(stems.contains(&vtree::fold_name(stem))),
    }
}

/// `parts`, several models resolving to one allowed name with their texture paths rewritten,
/// merged into one FMDL in the given order.
fn merge_parts(parts: &[FmdlFile]) -> Result<FmdlFile, TaskFailure> {
    let models = parts
        .iter()
        .map(Model::from_file)
        .collect::<Result<Vec<Model>, fmdl::FmdlError>>()?;
    Ok(merge(&models)?.to_file()?)
}

impl From<MergeError> for TaskFailure {
    fn from(error: MergeError) -> TaskFailure {
        match error {
            MergeError::MaterialConflict { name } => TaskFailure {
                code: Code::MergeMaterialConflict,
                context: vec![("material", name)],
            },
            MergeError::SkeletonConflict { name } | MergeError::DuplicateBoneName { name } => {
                TaskFailure {
                    code: Code::SklMergeConflict,
                    context: vec![("bone", name)],
                }
            }
            // Not a disagreement between the parts a member resolves by name: a part that
            // fails validation.
            MergeError::Other(_) => TaskFailure::from(anyhow::Error::from(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::bins::installed::WalkedCpks;

    use super::*;

    /// Team 714's Common texture directory.
    const COMMON_714: &str = "/Assets/pes16/model/character/common/714/sourceimages/";

    /// What `texture_supply` makes of `file_name` in `directory` for team 714, the export's
    /// Common textures being `common` at their paths below `Common/` and the folder's links
    /// `linked`, an installed CPK answering `installed` for every stem.
    fn supply(
        directory: &str,
        file_name: &str,
        common: &[&str],
        linked: &[&str],
        installed: Option<bool>,
    ) -> TextureSupply {
        let path = TexturePath {
            file_name: file_name.to_owned(),
            directory: directory.to_owned(),
        };
        let mut folders = TextureFolders::default();
        for stem in common {
            folders.insert(true, stem);
        }
        for below in linked {
            folders.insert_link(true, below);
        }
        let home = paths::common_home(Engine::Fox, 714);
        texture_supply(&path, &BTreeMap::new(), &folders, &home, |_| installed)
    }

    #[test]
    fn a_dummy_stem_and_a_path_outside_the_team_s_common_output_are_not_looked_for() {
        for file_name in ["dummy_kit.dds", "dummy_kit_srm.dds", "Dummy_Kit.dds"] {
            assert_eq!(
                supply(COMMON_714, file_name, &[], &[], Some(false)),
                TextureSupply::Supplied,
                "{file_name}"
            );
        }
        // A path naming the player's own texture home is a subdirectory of the team's
        // Common output as a `Common/` subfolder's is: looked up the same way.
        let home = "/Assets/pes16/model/character/common/714/05 - A/sourceimages/";
        assert_eq!(
            supply(home, "skin.dds", &[], &[], Some(false)),
            TextureSupply::Missing
        );
        assert_eq!(
            supply(home, "skin.dds", &[], &[], Some(true)),
            TextureSupply::Supplied
        );
        let game = "/Assets/pes16/model/character/common/sourceimages/";
        assert_eq!(
            supply(game, "skin.dds", &[], &[], None),
            TextureSupply::Supplied
        );
        // Not `dummy_`: looked for like any other stem.
        assert_eq!(
            supply(COMMON_714, "dummy.dds", &[], &[], Some(false)),
            TextureSupply::Missing
        );
    }

    #[test]
    fn a_common_path_is_supplied_by_common_a_link_or_an_installed_cpk() {
        let missing = Some(false);
        assert_eq!(
            supply(COMMON_714, "hair.dds", &["hair"], &[], missing),
            TextureSupply::Supplied
        );
        assert_eq!(
            supply(COMMON_714, "hair.dds", &[], &["hair"], missing),
            TextureSupply::Supplied
        );
        // Stems and directories compare folded.
        let shouted = "/ASSETS/pes16/model/character/common/714/sourceimages/";
        assert_eq!(
            supply(shouted, "Hair.dds", &["hair"], &[], missing),
            TextureSupply::Supplied
        );
        assert_eq!(
            supply(shouted, "Hair.dds", &[], &[], missing),
            TextureSupply::Missing
        );
        assert_eq!(
            supply(COMMON_714, "pants_kitN.dds", &["pants_kit1"], &[], missing),
            TextureSupply::Supplied
        );
        assert_eq!(
            supply(COMMON_714, "pants_kitN.dds", &["socks_kit1"], &[], missing),
            TextureSupply::Missing
        );
        assert_eq!(
            supply(COMMON_714, "hair.dds", &["skin"], &["face"], Some(true)),
            TextureSupply::Supplied
        );
        assert_eq!(
            supply(COMMON_714, "hair.dds", &["skin"], &["face"], Some(false)),
            TextureSupply::Missing
        );
        assert_eq!(
            supply(COMMON_714, "hair.dds", &["skin"], &["face"], None),
            TextureSupply::Unknown
        );
    }

    #[test]
    fn an_installed_cpk_holding_a_variant_of_a_kit_reference_s_set_supplies_it() {
        let path = |file_name: &str| TexturePath {
            file_name: file_name.to_owned(),
            directory: COMMON_714.to_owned(),
        };
        let installed = InstalledPaths::Known(WalkedCpks::from(HashSet::from([vtree::fold_name(
            &paths::common_texture(Engine::Fox, 714, "pants_kit1"),
        )])));
        let lookup = installed_lookup(&installed, 714);
        let none = TextureFolders::default();
        let no_verdict = BTreeMap::new();
        let home = paths::common_home(Engine::Fox, 714);
        let supplied =
            |file_name: &str| texture_supply(&path(file_name), &no_verdict, &none, &home, &lookup);
        assert_eq!(supplied("pants_kitN.dds"), TextureSupply::Supplied);
        assert_eq!(supplied("pants_kit1.dds"), TextureSupply::Supplied);
        assert_eq!(supplied("socks_kitN.dds"), TextureSupply::Missing);
        assert_eq!(supplied("pants_kit2.dds"), TextureSupply::Missing);
        // Another team's Common output holds nothing of team 714's.
        let other_team = installed_lookup(&installed, 702);
        assert_eq!(
            texture_supply(
                &path("pants_kitN.dds"),
                &no_verdict,
                &none,
                &home,
                &other_team
            ),
            TextureSupply::Missing
        );
        let unknown = installed_lookup(&InstalledPaths::Unknown, 714);
        assert_eq!(
            texture_supply(&path("pants_kitN.dds"), &no_verdict, &none, &home, &unknown),
            TextureSupply::Unknown
        );
    }

    #[test]
    fn an_installed_cpk_holding_a_kit_folder_member_supplies_the_set_s_reference() {
        // The installed CPK's `kit1/armor_bsm` alone supplies the `kitN/` subdirectory the
        // model names, `common_texture_stems` filing it under the set's `kitN` key too.
        let kitn = "/Assets/pes16/model/character/common/714/kitN/sourceimages/";
        let path = |file_name: &str| TexturePath {
            file_name: file_name.to_owned(),
            directory: kitn.to_owned(),
        };
        let installed = InstalledPaths::Known(WalkedCpks::from(HashSet::from([vtree::fold_name(
            &paths::common_texture(Engine::Fox, 714, "kit1/armor_bsm"),
        )])));
        let lookup = installed_lookup(&installed, 714);
        let none = TextureFolders::default();
        let no_verdict = BTreeMap::new();
        let home = paths::common_home(Engine::Fox, 714);
        let supplied =
            |file_name: &str| texture_supply(&path(file_name), &no_verdict, &none, &home, &lookup);
        assert_eq!(supplied("armor_bsm.dds"), TextureSupply::Supplied);
        assert_eq!(supplied("other.dds"), TextureSupply::Missing);
    }

    #[test]
    fn a_fox_path_into_a_common_subfolder_is_looked_up_at_that_path() {
        let jessie = "/Assets/pes16/model/character/common/714/jessie/sourceimages/";
        // What nobody there holds is the installed CPKs' answer, `Unknown` when they cannot
        // be looked in.
        assert_eq!(
            supply(jessie, "hair.dds", &[], &[], Some(false)),
            TextureSupply::Missing
        );
        assert_eq!(
            supply(jessie, "hair.dds", &[], &[], None),
            TextureSupply::Unknown
        );
        // `Common/`'s own `hair.dds` is not at that path, which resolves there alone.
        assert_eq!(
            supply(jessie, "hair.dds", &["hair"], &[], Some(false)),
            TextureSupply::Missing
        );
        // `Common/jessie/hair.dds`, a link standing for it, and an installed CPK each
        // supply it.
        assert_eq!(
            supply(jessie, "hair.dds", &["jessie/hair"], &[], Some(false)),
            TextureSupply::Supplied
        );
        assert_eq!(
            supply(jessie, "hair.dds", &[], &["jessie/hair"], Some(false)),
            TextureSupply::Supplied
        );
        assert_eq!(
            supply(jessie, "hair.dds", &[], &[], Some(true)),
            TextureSupply::Supplied
        );
    }

    #[test]
    fn a_common_path_into_a_kitn_folder_is_supplied_by_a_member_of_its_set() {
        // The `kitN/` subdirectory stands for the set of its `kit1`-`kit9` siblings:
        // `Common/kit1/armor.dds`, `Common/kitF/armor.dds`, a link standing for one, and
        // an installed CPK's copy each supply it; missing with none of them.
        let kitn = "/Assets/pes16/model/character/common/714/kitN/sourceimages/";
        assert_eq!(
            supply(kitn, "armor.dds", &["kit1/armor"], &[], Some(false)),
            TextureSupply::Supplied
        );
        assert_eq!(
            supply(kitn, "armor.dds", &["kitF/armor"], &[], Some(false)),
            TextureSupply::Supplied
        );
        assert_eq!(
            supply(kitn, "armor.dds", &[], &["kit1/armor"], Some(false)),
            TextureSupply::Supplied
        );
        assert_eq!(
            supply(kitn, "armor.dds", &[], &[], Some(true)),
            TextureSupply::Supplied
        );
        assert_eq!(
            supply(kitn, "armor.dds", &[], &[], Some(false)),
            TextureSupply::Missing
        );
        assert_eq!(
            supply(kitn, "armor.dds", &[], &[], None),
            TextureSupply::Unknown
        );
        // A `kitF/` path names no file: missing even when `Common/kitF/armor.dds` is held.
        let kitf = "/Assets/pes16/model/character/common/714/kitF/sourceimages/";
        assert_eq!(
            supply(kitf, "armor.dds", &["kitF/armor"], &[], Some(false)),
            TextureSupply::Missing
        );
    }

    /// The place `TextureFolders::insert` files the textures `below` under `directory`
    /// (`""` for textures directly in the folder, `jessie` for `jessie/skin`).
    fn place(directory: &str, below: &[&str]) -> TexturePlace {
        let mut folders = TextureFolders::default();
        for &below in below {
            folders.insert(true, below);
        }
        folders
            .folder(directory)
            .into_iter()
            .next()
            .map_or_else(TexturePlace::default, |(place, _)| place.clone())
    }

    /// The directory `point_texture` gives the path `file_name` in the game's team
    /// `team_folder`
    /// folder for team 792, the part's own stems being `own`, going to `/home/`, and its
    /// linked stems `linked`, going to `/common/`; asserts the file name is kept.
    fn pointed_between(file_name: &str, own: &[&str], linked: &[&str]) -> String {
        let mut path = TexturePath {
            file_name: file_name.to_owned(),
            directory: "/Assets/pes16/model/character/common/team_folder/sourceimages/".to_owned(),
        };
        let (own, linked) = (place("", own), place("", linked));
        let (home, common) = (plain("/home/"), plain("/common/"));
        let folders = TextureFolders::default();
        point_texture(
            &mut path,
            &[(&own, &home), (&linked, &common)],
            &folders,
            &common,
            "/792/",
            |_| None,
        );
        assert_eq!(path.file_name, file_name);
        path.directory
    }

    /// The directory `point_texture` gives `file_name` in `directory` for team 714, the part's
    /// own stems being `own`, going to `/home/`, and `Common/`'s textures `common` at their
    /// paths below `Common/`, going to `/common/` for a pre-Fox Common path.
    fn pointed_from(directory: &str, file_name: &str, own: &[&str], common: &[&str]) -> String {
        let mut path = TexturePath {
            file_name: file_name.to_owned(),
            directory: directory.to_owned(),
        };
        let own = place("", own);
        let mut folders = TextureFolders::default();
        for stem in common {
            folders.insert(true, stem);
        }
        point_texture(
            &mut path,
            &[(&own, &plain("/home/"))],
            &folders,
            &plain("/common/"),
            "/714/",
            |_| None,
        );
        assert_eq!(path.file_name, file_name);
        path.directory
    }

    /// The directory `directory`, below which a subdirectory follows directly.
    fn plain(directory: &str) -> TextureDirectory {
        TextureDirectory::plain(directory.to_owned())
    }

    #[test]
    fn a_pre_fox_common_path_goes_to_the_common_directory_when_common_holds_its_stem() {
        let pre_fox = "model/character/uniform/common/team_folder/";
        assert_eq!(
            pointed_from(pre_fox, "shirt.dds", &[], &["shirt"]),
            "/common/"
        );
        // `team_folder` in any letter case, and its short form, as a member's pre-Fox
        // `.mtl` spells the team's Common, and a kit reference.
        for segment in ["team_folder", "Team_Folder"] {
            let directory = format!("model/character/uniform/common/{segment}/");
            assert_eq!(
                pointed_from(&directory, "shirt.dds", &[], &["shirt"]),
                "/common/",
                "{segment}"
            );
        }
        assert_eq!(
            pointed_from("team_common_folder/", "shirt.dds", &[], &["shirt"]),
            "/common/"
        );
        assert_eq!(
            pointed_from(pre_fox, "pants_kitN.dds", &[], &["pants_kit2"]),
            "/common/"
        );
        // The folder's own texture of the stem comes first, as for any path.
        assert_eq!(
            pointed_from(pre_fox, "shirt.dds", &["shirt"], &["shirt"]),
            "/home/"
        );
        // Pointed at the team's Common output too when `Common/` does not hold the stem:
        // the installed CPKs decide its supply.
        assert_eq!(
            pointed_from(pre_fox, "shirt.dds", &[], &["skin"]),
            "/common/"
        );
        // Only a pre-Fox Common path reaches `Common/`: a `./` path or a Fox path of the
        // game's own does not. One below a directory of the team's folder is a `Common/`
        // subfolder path, at its subdirectory of the Fox output.
        for directory in ["./", "/Assets/pes16/model/character/common/sourceimages/"] {
            assert_eq!(
                pointed_from(directory, "shirt.dds", &[], &["shirt"]),
                directory,
                "{directory}"
            );
        }
        assert_eq!(
            pointed_from(
                "model/character/uniform/common/team_folder/05 - A/",
                "shirt.dds",
                &[],
                &["shirt"]
            ),
            "/common/05 - A/"
        );
    }

    #[test]
    fn a_game_file_directly_below_common_keeps_the_long_form_in_either_spelling() {
        // The game's own file, not looked up in `Common/` even when it holds the stem; the
        // short form is written as the long one, which the long form already is.
        for directory in ["common_folder/", "model/character/uniform/common/"] {
            assert_eq!(
                pointed_from(
                    directory,
                    "uni_pattern_nrm00.dds",
                    &[],
                    &["uni_pattern_nrm00"]
                ),
                "model/character/uniform/common/",
                "{directory}"
            );
        }
    }

    /// The directory `point_texture` gives the pre-Fox Common path `directory` + `file_name`
    /// and the supply `texture_supply` reports for it: `below` the Common textures the export
    /// packs at their paths below `Common/`, `holds` the installed CPKs' answer.
    fn common_supply(
        directory: &str,
        file_name: &str,
        below: &[&str],
        holds: impl Fn(&str) -> Option<bool>,
    ) -> (String, TextureSupply) {
        let mut path = TexturePath {
            file_name: file_name.to_owned(),
            directory: directory.to_owned(),
        };
        let mut folders = TextureFolders::default();
        for &below in below {
            folders.insert(true, below);
        }
        let home = paths::common_home(Engine::Fox, 714);
        let mut supplies = BTreeMap::new();
        if let Some(supply) = point_texture(&mut path, &[], &folders, &home, "/714/", &holds) {
            supplies.insert(format!("{}{}", path.directory, path.file_name), supply);
        }
        (
            path.directory.clone(),
            texture_supply(&path, &supplies, &folders, &home, &holds),
        )
    }

    #[test]
    fn a_common_path_is_pointed_at_the_fox_common_output_whether_held_or_not() {
        // The installed CPKs hold `skin` and `jessie/skin` below `Common/` and nothing else.
        let installed = InstalledPaths::Known(WalkedCpks::from(
            ["skin", "jessie/skin"]
                .iter()
                .map(|below| vtree::fold_name(&paths::common_texture(Engine::Fox, 714, below)))
                .collect::<HashSet<String>>(),
        ));
        let known = installed_lookup(&installed, 714);
        let uninstalled = InstalledPaths::Unknown;
        let unknown = installed_lookup(&uninstalled, 714);
        let direct = "model/character/uniform/common/team_folder/";
        let jessie_dir = "model/character/uniform/common/team_folder/jessie/";
        let jessie = "/Assets/pes16/model/character/common/714/jessie/sourceimages/";
        assert_eq!(
            common_supply(direct, "skin.dds", &[], &known),
            (COMMON_714.to_owned(), TextureSupply::Supplied)
        );
        assert_eq!(
            common_supply(jessie_dir, "skin.dds", &[], &known),
            (jessie.to_owned(), TextureSupply::Supplied)
        );
        // What nothing holds is still pointed there: `other`'s supply is the installed
        // answer, `Unknown` when they cannot be looked in.
        assert_eq!(
            common_supply(direct, "other.dds", &[], &known),
            (COMMON_714.to_owned(), TextureSupply::Missing)
        );
        assert_eq!(
            common_supply(jessie_dir, "other.dds", &[], &known),
            (jessie.to_owned(), TextureSupply::Missing)
        );
        assert_eq!(
            common_supply(direct, "other.dds", &[], &unknown),
            (COMMON_714.to_owned(), TextureSupply::Unknown)
        );
        assert_eq!(
            common_supply(jessie_dir, "other.dds", &[], &unknown),
            (jessie.to_owned(), TextureSupply::Unknown)
        );
        // One the export does hold stays supplied without asking the installed CPKs.
        assert_eq!(
            common_supply(jessie_dir, "skin.dds", &["jessie/skin"], &unknown),
            (jessie.to_owned(), TextureSupply::Supplied)
        );
        // An FMDL's own texture table names the team `team_folder` too (in any letter
        // case) beside the game's own `000`: each becomes the team's segment, so the
        // path is the team's Common output's and its supply is decided there.
        for segment in ["team_folder", "Team_Folder", "000"] {
            let directory = format!("/Assets/pes16/model/character/common/{segment}/sourceimages/");
            assert_eq!(
                common_supply(&directory, "skin.dds", &[], &known),
                (COMMON_714.to_owned(), TextureSupply::Supplied),
                "{segment}"
            );
        }
    }

    /// `pointed_from` with the `Common/` textures `below`, each at its path below `Common/`:
    /// a Common path into a subfolder resolves at that path alone.
    fn pointed_at_below(directory: &str, file_name: &str, below: &[&str]) -> String {
        let mut path = TexturePath {
            file_name: file_name.to_owned(),
            directory: directory.to_owned(),
        };
        let mut folders = TextureFolders::default();
        for &below in below {
            folders.insert(true, below);
        }
        point_texture(
            &mut path,
            &[],
            &folders,
            &plain("/common/"),
            "/714/",
            |_| None,
        );
        path.directory
    }

    #[test]
    fn a_common_path_into_a_subfolder_resolves_at_that_path_alone() {
        let pre_fox = "team_common_folder/jessie/";
        assert_eq!(
            pointed_at_below(pre_fox, "skin.dds", &["jessie/skin"]),
            "/common/jessie/"
        );
        // With nothing there it is still pointed at the subdirectory of the team's
        // Common output: the installed CPKs decide its supply.
        for below in [&["other/skin"][..], &[][..]] {
            assert_eq!(
                pointed_at_below(pre_fox, "skin.dds", below),
                "/common/jessie/",
                "{below:?}"
            );
        }
    }

    /// `pointed_between` with the part's own stems `stems` and no linked stem.
    fn pointed(file_name: &str, stems: &[&str]) -> String {
        pointed_between(file_name, stems, &[])
    }

    #[test]
    fn a_stem_goes_to_the_first_place_holding_it_and_a_linked_one_to_the_common_directory() {
        let own = ["skin"];
        let linked = ["hair", "pants_kit2"];
        let game = "/Assets/pes16/model/character/common/792/sourceimages/";
        assert_eq!(pointed_between("skin.dds", &own, &linked), "/home/");
        assert_eq!(pointed_between("hair.dds", &own, &linked), "/common/");
        // Kit references resolve in each place by the variants that place holds.
        assert_eq!(pointed_between("pants_kitN.dds", &own, &linked), "/common/");
        assert_eq!(
            pointed_between("pants_kitN.dds", &["pants_kit1"], &linked),
            "/home/"
        );
        assert_eq!(pointed_between("other.dds", &own, &linked), game);
        // A stem held in both places goes to the first.
        assert_eq!(pointed_between("hair.dds", &["hair"], &linked), "/home/");
    }

    /// Slot 05's texture home for team 714 on Fox, as a model names it.
    fn home_714_05() -> TextureDirectory {
        paths::TextureHome::PlayerCommon {
            folder_name: "05 - A".to_owned(),
        }
        .directory(Engine::Fox, 714)
    }

    #[test]
    fn a_subfolder_s_texture_is_pointed_at_its_own_directory_below_the_home() {
        let home = home_714_05();
        let pointed = |file_name: &str, below: &[&str]| {
            let mut path = TexturePath {
                file_name: file_name.to_owned(),
                directory: "/Assets/pes16/model/character/common/team_folder/sourceimages/"
                    .to_owned(),
            };
            let folders = TextureFolders::default();
            let directory = below
                .first()
                .map_or("", |below| texture_lookup::split(below).0);
            point_texture(
                &mut path,
                &[(&place(directory, below), &home)],
                &folders,
                &plain("/common/"),
                "/714/",
                |_| None,
            );
            path.directory
        };
        // The subdirectory sits before `sourceimages/`.
        assert_eq!(
            pointed("skin.dds", &["jessie/skin"]),
            "/Assets/pes16/model/character/common/714/05 - A/jessie/sourceimages/"
        );
        assert_eq!(
            pointed("pants_kitN.dds", &["jessie/body/pants_kit2"]),
            "/Assets/pes16/model/character/common/714/05 - A/jessie/body/sourceimages/"
        );
        assert_eq!(
            pointed("skin.dds", &["skin"]),
            "/Assets/pes16/model/character/common/714/05 - A/sourceimages/"
        );
    }

    #[test]
    fn a_path_below_the_file_s_folder_is_pointed_at_the_texture_there_or_left_as_written() {
        let home = home_714_05();
        let pointed = |file_name: &str, place: Option<&TexturePlace>| {
            let mut path = TexturePath {
                file_name: file_name.to_owned(),
                directory: "./textures/".to_owned(),
            };
            let places: Vec<(&TexturePlace, &TextureDirectory)> =
                place.map(|place| (place, &home)).into_iter().collect();
            point_texture_below(&mut path, &places);
            assert_eq!(path.file_name, file_name);
            path.directory
        };
        let textures = place("textures", &["textures/Skin", "textures/pants_kit2"]);
        let there = "/Assets/pes16/model/character/common/714/05 - A/textures/sourceimages/";
        assert_eq!(pointed("skin.dds", Some(&textures)), there);
        assert_eq!(pointed("pants_kitN.dds", Some(&textures)), there);
        // Nothing of its stem there, or no folder there: left as written.
        assert_eq!(pointed("hair.dds", Some(&textures)), "./textures/");
        assert_eq!(pointed("skin.dds", None), "./textures/");
    }

    #[test]
    fn a_path_still_below_the_file_s_folder_after_pointing_is_missing() {
        // Whatever a lookup in the installed CPKs would answer: nothing sat there.
        for installed in [Some(true), None] {
            assert_eq!(
                supply("./textures/", "skin.dds", &["skin"], &[], installed),
                TextureSupply::Missing
            );
        }
        assert_eq!(
            supply("./textures/", "dummy_kit.dds", &[], &[], Some(false)),
            TextureSupply::Supplied
        );
        // A bare `./` names a texture by name, and pointing found none: nothing supplies it.
        assert_eq!(
            supply("./", "skin.dds", &[], &[], Some(false)),
            TextureSupply::Missing
        );
    }

    #[test]
    fn a_bare_local_directory_the_pointing_left_is_missing() {
        // `./` or an empty directory names the file's own folder: nothing sat
        // there, whatever a lookup in the installed CPKs would answer.
        for directory in ["./", ""] {
            for installed in [Some(true), Some(false), None] {
                assert_eq!(
                    supply(directory, "missing.dds", &[], &[], installed),
                    TextureSupply::Missing,
                    "{directory:?} {installed:?}"
                );
            }
        }
        // A `dummy_` stem is still never looked for.
        assert_eq!(
            supply("./", "dummy_kit.dds", &[], &[], Some(false)),
            TextureSupply::Supplied
        );
    }

    #[test]
    fn a_path_naming_the_set_s_fallback_file_finds_nothing() {
        // `pants_kitF.dds` supplies the set's `kitN` reference, never a path naming the
        // `kitF` file itself, which is never a file under its own name.
        let game = "/Assets/pes16/model/character/common/792/sourceimages/";
        assert_eq!(pointed("pants_kitN.dds", &["pants_kitF"]), "/home/");
        assert_eq!(pointed("pants_kitF.dds", &["pants_kitF"]), game);
    }

    #[test]
    fn a_kit_reference_is_the_part_s_own_texture_when_a_variant_of_its_set_is() {
        let game = "/Assets/pes16/model/character/common/792/sourceimages/";
        assert_eq!(pointed("pants_kitN.dds", &["pants_kit2"]), "/home/");
        assert_eq!(pointed("shirt.dds", &["shirt", "pants_kit2"]), "/home/");
        // With no variant of its own set among the stems, it is any other path: one of the
        // game's own.
        assert_eq!(pointed("pants_kitN.dds", &[]), game);
        assert_eq!(
            pointed("pants_kitN.dds", &["socks_kit2", "pants", "pants_kitN_x"]),
            game
        );
        // The legacy `dummy_kit` holds no token: only its team folder is replaced.
        assert_eq!(pointed("dummy_kit.dds", &["pants_kit2"]), game);
    }
}
