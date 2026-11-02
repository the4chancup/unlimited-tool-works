//! A pre-Fox boots or gloves output: loose files in the shape of the game's own boots and
//! glove folders, written under an id. A `Boots/` or `Gloves/` folder a player links plainly is
//! written once under its shared id. An `ingame_face` player's own boots and gloves are written
//! under his exclusive id: his parts of each (the Common models his `.common` links copy in
//! included, with the `.mtl` each uses) and the models of a folder of their kind his link
//! combines.
//!
//! Boots are one model as `boots.model` and the `.mtl` it uses as `boots.mtl`, the names the
//! game loads, several models merged into one (`pes_model::ops::merge`). Gloves are unmerged:
//! each model at its path below the folder and the `.mtl` files they use at the output's root,
//! each named by its path below the folder with each `/` written `_`, lowercased, and listed in
//! a generated `glove.xml`.
//!
//! An `.fmdl` among the models is converted as the face converts one, its `.model` and
//! material set taking the place of a member's `.model` and the `.mtl` it uses. A `.model` the
//! conversion pre-check finds posed off the version's skeleton is moved onto it as the face
//! moves one, the `.mtl` it uses packed as written.
//!
//! A shared folder's textures sit at their paths beside them
//! (`TextureHome::SharedOutput`), each `.mtl` path naming its texture from the output's root,
//! where the `.mtl` packs (`./<stem>.dds`, `./<sub>/<stem>.dds`); a player's are
//! in his common folder (`TextureHome::PlayerCommon`), named as his face's `.mtl` files name
//! them, and a Common `.mtl`'s stay in the team's Common output. `materialize` writes the files
//! into the output's folder.

use std::collections::{BTreeMap, BTreeSet};
use std::iter;
use std::sync::Arc;

use aesthetics_export::{FileDescriptor, FileKind, ModelFormat};
use pes_model::format::PreFoxModel;
use pes_model::format::mtl::MaterialSet;
use pes_model::model::Model;
use pes_model::ops::merge::{MergeError, merge};
use pes_version::Engine;
use pipeline::MemoryBudget;
use vtree::ScopePath;

use super::conversion::{
    ConvertedMaterials, PreFoxConversion, fmdl_for_pre_fox, model_for_pre_fox,
};
use super::materialize::PackageFiles;
use super::prefox_face::{
    FolderPlaces, MaterialPlaces, add_environment_map, directory_below, insert, point_materials,
    point_reserved_kit_stems, read_materials, rewritten_materials,
};
use super::{CompileContext, Finding, TaskFailure, TaskFiles, take};
use crate::deep::named_on_folder;
use crate::face_xml::{XmlEntry, glove_xml, ratio};
use crate::messages::Code;
use crate::mtl_search::mtl_for;
use crate::paths;
use crate::plan::ModelFolder;
use crate::plan::roles::{
    ModelPackage, PlayerFile, below_common, common_folder, common_texture_below, file_stem,
    is_common_file, path_stem,
};
use crate::texture_lookup::TextureFolders;

/// What the deep pass guarantees of every `.model` a task reads on pre-Fox.
const MTL_FOUND: &str = "the deep pass drops a folder holding a `.model` no `.mtl` is found for \
                         (`model_material_undefined`)";

/// One `.model` of the output, or an `.fmdl` converted to one, with the source folder it was
/// found in: a `.model`'s `.mtl` is searched for there (`mtl_for`), a combined folder's never
/// in the player's folder. A Common model a marked player's link copies in has his folder as
/// its source, a Common `.model`'s `.mtl` resolved at planning (`material_of`), and so is a
/// Common FMDL's skeleton (`plan::CommonModel::skeleton`).
struct SourceModel<'a> {
    /// The model file.
    file: &'a FileDescriptor,
    /// The type a `glove.xml` lists it under, its role's (`PlayerFile::PreFoxModel`,
    /// `PlayerFile::PreFoxPart`); the boots list no model.
    xml_type: String,
    /// Its source folder's export path.
    source_path: &'a ScopePath,
    /// Its source folder's files.
    source_files: &'a [FileDescriptor],
    /// The `.skl` its source pairs with an FMDL by path stem, the conversion's bind pose
    /// (`PlayerFile::ConversionSkeleton`), as the face pairs one; `None` for a `.model`, which
    /// no such skeleton pairs with, and for an FMDL its source holds none for.
    skeleton: Option<&'a FileDescriptor>,
}

impl SourceModel<'_> {
    /// Whether the model is an FMDL, converted for the target (`fmdl_for_pre_fox`) rather than
    /// packed with the `.mtl` it uses, from its bytes or moved onto the version's skeleton
    /// when the conversion pre-check flags it (`model_for_pre_fox`).
    fn converts(&self) -> bool {
        self.file.kind == FileKind::Model(ModelFormat::Fmdl)
    }

    /// The name a gloves output packs the model under: its file's path below its source folder
    /// with its name lowercased (`grip/glove_r.model`), an FMDL's as the `.model` its
    /// conversion writes (`grip/glove_l.fmdl` packs as `grip/glove_l.model`); a Common file's
    /// bare name, copied in under its own.
    fn packed_name(&self) -> String {
        let name = self.file.path.name();
        if is_common_file(&self.file.path) {
            if self.converts() {
                return format!("{}.model", file_stem(name)).to_ascii_lowercase();
            }
            return name.to_ascii_lowercase();
        }
        let directory = directory_below(&self.file.path, self.source_path);
        if self.converts() {
            return format!("{directory}{}.model", file_stem(name)).to_ascii_lowercase();
        }
        format!("{directory}{}", name.to_ascii_lowercase())
    }
}

/// The name a gloves output packs the `.mtl` `file` of the source folder `source` under: its
/// path below `source` with each `/` written `_`, lowercased (`grip/glove_r.mtl` packs as
/// `grip_glove_r.mtl`, at the output's root, the game loading no `../` texture path); a
/// Common file's bare name, copied in under its own.
fn packed_mtl_name(file: &FileDescriptor, source: &ScopePath) -> String {
    if is_common_file(&file.path) {
        return file.path.name().to_ascii_lowercase();
    }
    let directory = directory_below(&file.path, source);
    format!("{directory}{}", file.path.name())
        .replace('/', "_")
        .to_ascii_lowercase()
}

/// One part of a boots output: its `.model`'s export path, which names it in an error, the
/// model's bytes, and the material set it uses, its texture paths already pointed where the
/// output's textures are.
struct BootsPart<'a> {
    /// The model's export path.
    path: &'a ScopePath,
    /// The model's bytes.
    model: Vec<u8>,
    /// The material set the model uses.
    materials: &'a MaterialSet,
}

/// The files of `folder`'s pre-Fox `package`, boots or gloves, compiled from its files' bytes in
/// `files` for team `team_id`, by their names in the output's folder. The models of the
/// package are a shared folder's, or an `ingame_face` player's parts, the Common models his
/// `.common` links copy in included, with a combined folder's of the package's kind. Each
/// `.model` runs the conversion pre-check (`model_for_pre_fox`, with the `.mtl` it uses, its
/// findings naming it by `named_on_folder`) and, when flagged, is moved onto `ctx.version`'s
/// skeleton before it is merged or packed, the `.mtl` packed as written. Boots:
/// the models as `boots.model` and the `.mtl` each uses (`material_of`) as `boots.mtl`
/// (`boots_files`), the merge of several noted in `findings` as
/// `model_merged`. Gloves: each model at its path below the folder and the `.mtl` each uses
/// at the output's root, its path below the folder with each `/` written `_`, lowercased,
/// once however many use the `.mtl`, and a `glove.xml` listing the
/// models in the order of
/// their export paths, case-folded, each typed by its role and naming its `.mtl` at the root
/// (`./grip/glove_r.model`, `./grip_glove_r.mtl`); a player's
/// model or `.mtl` replaces a combined folder's packing under its name (`without_replaced`).
/// A `.mtl` no model uses is not packed. Every `.mtl` packed has each texture path naming one
/// of the folder's textures by its stem pointed at the folder's texture home, and one naming a
/// stem a texture link of the folder stands for at that texture in the team's Common output,
/// as the face's are (`rewritten_materials`); a copied Common `.mtl` resolves in `Common/`
/// alone, `Common/` its model folder (a name nearest first from its directory up to `Common/`,
/// a path below its directory at that path), each texture named at its own path below
/// `Common/` in the team's Common output, while the folder's own `.mtl` files reach `Common/`
/// only through a `.common` link and leave any other Common path as written. A Common model and
/// its `.mtl` pack under their file names as the folder's own do. An `.fmdl` is converted
/// (`fmdl_for_pre_fox`, the `.skl` its source pairs with it as the bind pose, a Common FMDL's
/// the Common `.skl` of its stem, its findings naming it by `named_on_folder`, a Common one by
/// its export path), its `.model` taking a member's `.model`'s place and its
/// material set the place of the `.mtl` that model uses: merged into `boots.mtl`, or packed
/// at the output's root as its model's packed stem flattened (`grip/glove_r.fmdl`'s as
/// `grip_glove_r.mtl`) and named by its `glove.xml` entry. The
/// set's texture paths are pointed as the face points a converted one's: each metal material
/// first given the environment map in the texture home (`add_environment_map`, as the face
/// does), then the folder's textures and links as a `.mtl`'s, a Common FMDL's set resolving in
/// `Common/` as a copied Common `.mtl` does, then the reserved kit stems at
/// the team's Common texture directory. A conversion that fails fails the task. Two files
/// packing under one name otherwise fail the task, a member's `.mtl` of a converted glove's
/// `<stem>.mtl` name among them. A merge is charged to `ctx`'s memory budget.
pub(super) fn package(
    folder: &ModelFolder,
    package: ModelPackage,
    team_id: u16,
    ctx: &CompileContext,
    files: &mut TaskFiles,
    findings: &mut Vec<Finding>,
) -> Result<PackageFiles, TaskFailure> {
    let mut models = Vec::new();
    // The folder's textures by the folder of its tree holding each (an `ingame_face` player's
    // parts may sit in his subfolders), and its combined folders', each with the path its
    // converted DDS has below the texture home (`folder_textures`), and its texture links the
    // same way (`texture_lookup`). A shared folder holds no link.
    let mut textures = TextureFolders::default();
    // `roles()` yields the folder's own files, then each combined folder's in `combined`'s
    // order, so each source's roles pair with its own file list here.
    let source_files = iter::once(&folder.files).chain(
        folder
            .combined
            .iter()
            .map(|combined| &combined.folder.files),
    );
    for ((source_package, source_path, source_roles), source_files) in
        folder.roles().into_iter().zip(source_files)
    {
        // The skeletons of this source's FMDLs, each its FMDL's bind pose.
        let skeletons: Vec<&FileDescriptor> = source_roles
            .iter()
            .filter(|(_, role)| matches!(role, PlayerFile::ConversionSkeleton))
            .map(|(file, _)| *file)
            .collect();
        for (file, role) in source_roles {
            let source_model = |xml_type| {
                let path_fold = vtree::fold_name(path_stem(file));
                SourceModel {
                    file,
                    xml_type,
                    source_path,
                    source_files,
                    skeleton: skeletons
                        .iter()
                        .find(|skeleton| vtree::fold_name(path_stem(skeleton)) == path_fold)
                        .copied(),
                }
            };
            match role {
                // A shared folder's model is a part of the package the folder feeds, a
                // combined one's of the player's.
                PlayerFile::PreFoxModel { xml_type } if source_package == package => {
                    models.push(source_model(xml_type));
                }
                PlayerFile::PreFoxPart {
                    package: owner,
                    xml_type,
                } if owner == package => models.push(source_model(xml_type)),
                PlayerFile::Texture { below, .. } => {
                    textures.insert(source_path == &folder.path, &below);
                }
                PlayerFile::CommonTexture(below) => {
                    let below = common_texture_below(&folder.common_files, &below);
                    textures.insert_link(source_path == &folder.path, &below);
                }
                // A `.mtl` is packed when a model uses it (`material_of`), below; a marked
                // player's `.mtl` link has brought in the Common `.mtl` as one
                // (`ModelFolder::roles`).
                PlayerFile::Material | PlayerFile::CommonMaterial => {}
                // A model of another package, and the roles planning gives none of the
                // package's files: the other roles are no package's here and are not
                // read (validation reports a file with no role as `file_not_used`), and a
                // marked player's model link is a part (`PlayerFile::PreFoxPart`), not
                // listed by reference.
                PlayerFile::PreFoxModel { .. }
                | PlayerFile::PreFoxPart { .. }
                | PlayerFile::Model { .. }
                | PlayerFile::CommonModel { .. }
                | PlayerFile::Packed { .. }
                | PlayerFile::FaceDiffXml
                | PlayerFile::UnusedFaceFile
                | PlayerFile::Skeleton { .. }
                | PlayerFile::SlotlessSkeleton
                | PlayerFile::LeftOutKitVariant
                | PlayerFile::PreFoxCommonModel { .. }
                | PlayerFile::FaceXml => {}
                // Read with the FMDL it is the bind pose of (`SourceModel::skeleton`).
                PlayerFile::ConversionSkeleton => {}
                // Planning drops a player folder holding one, and `drop_gltf_folders` removes a
                // shared folder whose selected model is one before any task is made, so no
                // task meets it.
                PlayerFile::UnsupportedGltf => {}
            }
        }
    }
    let home = folder.textures.directory(Engine::PreFox, team_id);
    let common_directory = paths::common_texture_directory(Engine::PreFox, team_id);
    let common_home = paths::common_home(Engine::PreFox, team_id);
    // The places of the folder's own `.mtl` files, nearest first from the file's folder
    // (`texture_lookup`): its own stem before one a texture link stands for, a path below
    // the file's folder at that path, as in the face (`prefox_face::face`). A `Common/`
    // texture is reached only through a `.common` link (`spec/model_format.md` "Link files"),
    // so an unlinked Common stem one names is left as written.
    let folder_places = FolderPlaces {
        textures,
        home: &home,
        common_directory: &common_home,
    };
    // The places a `Common/` `.mtl` copied in with a part, and a converted Common FMDL's
    // set, resolve in: `Common/`'s own, `Common/` their model folder and the team's Common
    // output their home (`FolderPlaces::common`). Nothing of the player's folder is
    // consulted for a `Common/` `.mtl`: a texture of his shadows nothing it names.
    let common_scope = common_folder();
    let common_below: Vec<String> = folder
        .common_files
        .iter()
        .filter(|file| file.kind == FileKind::Texture)
        .map(|file| file_stem(below_common(&file.path)).to_owned())
        .collect();
    let common_places = FolderPlaces::common(&common_below, &common_home);
    // The places of a `.mtl` `file`, packed at the output's root: a `Common/` `.mtl`
    // resolves in `Common/` alone.
    let places_for = |file: &FileDescriptor, source: &ScopePath| -> MaterialPlaces {
        if is_common_file(&file.path) {
            common_places.of(&common_scope, &file.path)
        } else {
            folder_places.of(source, &file.path)
        }
    };
    // An FMDL converted (`fmdl_for_pre_fox`, its source's skeleton of its path stem the bind
    // pose, its findings naming it by `named_on_folder`), its material set pointed as the face
    // points a converted one, in the face's order: the environment map added first, since
    // the pointing respells it as the folder spells its own `env` texture.
    let convert = |model: &SourceModel<'_>,
                   files: &mut TaskFiles,
                   findings: &mut Vec<Finding>|
     -> Result<PreFoxConversion, TaskFailure> {
        let skeleton = model.skeleton.map(|skeleton| take(files, skeleton));
        let mut conversion = fmdl_for_pre_fox(
            &named_on_folder(&model.file.path, &folder.path),
            &take(files, model.file),
            skeleton.as_deref(),
            ctx,
            findings,
            ConvertedMaterials::Converted,
        )?;
        // The flag is not consulted, as the face does not: only a `Basic_CNSR` material gets
        // the sampler, which the converter writes for the `metal` family the deep pass flags
        // with the converter's own rule, so the shader and the flag agree and the sampler
        // names the texture the textures task emits (or the folder's own `env`, or the Common
        // one its link names, `point_materials` respelling it below).
        add_environment_map(&mut conversion.materials, &home.of(""));
        point_materials(
            &mut conversion.materials,
            &places_for(model.file, model.source_path),
        );
        point_reserved_kit_stems(&mut conversion.materials, &common_directory);
        Ok(conversion)
    };
    let mut contents = PackageFiles::new();
    match package {
        ModelPackage::Boots => {
            // The order the Fox merge takes the same parts in (`model::package`): by file
            // name, folded as the file system folds it, then by export path.
            models.sort_by_cached_key(|model| {
                (
                    vtree::fold_name(model.file.path.name()),
                    model.file.path.as_str().to_owned(),
                )
            });
            // The material sets the parts use, each part naming its own by index: a member's
            // `.mtl` read once however many parts use it, a converted FMDL's own set. A
            // member's set keeps its source bytes beside its index, which the pre-check of
            // each model using it reads.
            let mut sets: Vec<MaterialSet> = Vec::new();
            let mut member_sets: BTreeMap<&ScopePath, (usize, Vec<u8>)> = BTreeMap::new();
            let mut sources: Vec<(&ScopePath, Vec<u8>, usize)> = Vec::new();
            for model in &models {
                if model.converts() {
                    let conversion = convert(model, files, findings)?;
                    sets.push(conversion.materials);
                    sources.push((&model.file.path, conversion.model, sets.len() - 1));
                    continue;
                }
                let material = material_of(folder, model);
                if !member_sets.contains_key(&material.path) {
                    let bytes = take(files, material);
                    // The set merges into `boots.mtl` at the output's root: its texture
                    // paths are pointed relative to there, not to the `.mtl` read.
                    sets.push(read_materials(
                        material,
                        &bytes,
                        &places_for(material, model.source_path),
                    )?);
                    member_sets.insert(&material.path, (sets.len() - 1, bytes));
                }
                let (index, mtl) = &member_sets[&material.path];
                let bytes = model_for_pre_fox(
                    &named_on_folder(&model.file.path, &folder.path),
                    take(files, model.file),
                    mtl,
                    ctx,
                    findings,
                    ConvertedMaterials::Converted,
                )?;
                sources.push((&model.file.path, bytes, *index));
            }
            let merged = sources.len() > 1;
            let parts = sources
                .into_iter()
                .map(|(path, model, index)| BootsPart {
                    path,
                    model,
                    materials: &sets[index],
                })
                .collect();
            let (model, materials) = boots_files(&ctx.budget, parts)?;
            if merged {
                findings.push((Code::ModelMerged, vec![("model", "boots.model".to_owned())]));
            }
            insert(&mut contents, package, "boots.model".to_owned(), model)?;
            insert(&mut contents, package, "boots.mtl".to_owned(), materials)?;
        }
        ModelPackage::Gloves => {
            let own = |source_path: &ScopePath| *source_path == folder.path;
            let mut models = without_replaced(models, |model| {
                (model.packed_name(), own(model.source_path))
            });
            // By export path, case-folded, then as spelled, so a recompile lists them alike.
            models.sort_by_cached_key(|model| {
                (
                    model.file.path.fold_key(),
                    model.file.path.as_str().to_owned(),
                )
            });
            let mut entries = Vec::new();
            // The `.mtl` files the models use, each once, with its source folder: a model's
            // search finds one in its own source folder.
            let mut used: Vec<(&FileDescriptor, &ScopePath)> = Vec::new();
            for model in models {
                let packed = model.packed_name();
                let (bytes, material) = if model.converts() {
                    let conversion = convert(&model, files, findings)?;
                    // Packed at the root under its model's packed stem flattened, as a
                    // member's `.mtl` is: a member's `.mtl` of that name the folder also
                    // packs is two files of one name (`insert`).
                    let material = format!("{}.mtl", file_stem(&packed).replace('/', "_"));
                    insert(
                        &mut contents,
                        package,
                        material.clone(),
                        conversion.materials.write(),
                    )?;
                    (conversion.model, material)
                } else {
                    let material = material_of(folder, &model);
                    if !used.iter().any(|(file, _)| file.path == material.path) {
                        used.push((material, model.source_path));
                    }
                    let name = packed_mtl_name(material, model.source_path);
                    let source = take(files, model.file);
                    // Read in place: the `.mtl` is packed below, through `used`.
                    let mtl = files.get(&material.path).expect(
                        "the task's files include every `.mtl` its models use, taken after \
                         the models (`TaskKind::files`)",
                    );
                    let bytes = model_for_pre_fox(
                        &named_on_folder(&model.file.path, &folder.path),
                        source,
                        mtl,
                        ctx,
                        findings,
                        ConvertedMaterials::Converted,
                    )?;
                    (bytes, name)
                };
                entries.push(XmlEntry {
                    xml_type: model.xml_type,
                    path: format!("./{packed}"),
                    material: format!("./{material}"),
                    ratio: ratio(file_stem(model.file.path.name())).map(str::to_owned),
                });
                insert(&mut contents, package, packed, bytes)?;
            }
            let used = without_replaced(used, |(file, source)| {
                (packed_mtl_name(file, source), own(source))
            });
            for (file, source) in used {
                let packed = packed_mtl_name(file, source);
                let bytes =
                    rewritten_materials(file, &take(files, file), &places_for(file, source))?;
                insert(&mut contents, package, packed, bytes)?;
            }
            insert(
                &mut contents,
                package,
                "glove.xml".to_owned(),
                glove_xml(&entries),
            )?;
        }
        ModelPackage::Face => unreachable!(
            "a shared face has no output of its own: it is copied into each linking player's \
             face (`prefox_face::face`)"
        ),
    }
    Ok(contents)
}

/// The `.mtl` that `model`, a model of `folder`'s package, uses: for a Common model a marked
/// player's `.common` link copies in, the one planning found from the link
/// (`CommonModel::material`), since a search from the Common path would look in the wrong
/// folders; for any other, the one its search finds in its source folder (`mtl_for`), among
/// `Common/`'s files only in the player's own folder.
fn material_of<'a>(folder: &'a ModelFolder, model: &SourceModel<'a>) -> &'a FileDescriptor {
    if !is_common_file(&model.file.path) {
        // A combined shared folder's search sees no `Common/` file, as the deep pass's does
        // (`deep::pairings`): its `.common` links have no role, and the Common `.mtl` one
        // names is not among the task's files. A shared output's own folder has none either
        // (`ModelFolder::common_files`).
        let common: &[FileDescriptor] = if model.source_path == &folder.path {
            &folder.common_files
        } else {
            &[]
        };
        return mtl_for(
            &model.file.path,
            model.source_path,
            model.source_files,
            common,
        )
        .expect(MTL_FOUND);
    }
    folder
        .common_models
        .iter()
        .find(|common| common.model.path == model.file.path)
        .and_then(|common| common.material.as_ref())
        .expect("planning resolves the `.mtl` of every Common part it puts in a link's place")
}

/// `items`, gloves files of a player or a shared folder, less each of a folder the player
/// combines that packs under a path one of his own packs under: his file replaces it, as a
/// copy of his over the linked folder's would. `packed` gives an item's packed path and
/// whether the player's own folder holds it; a shared folder's items are all its own, so none
/// is left out.
fn without_replaced<T>(items: Vec<T>, packed: impl Fn(&T) -> (String, bool)) -> Vec<T> {
    let own_names: BTreeSet<String> = items
        .iter()
        .map(&packed)
        .filter_map(|(name, own)| own.then_some(name))
        .collect();
    items
        .into_iter()
        .filter(|item| {
            let (name, own) = packed(item);
            own || !own_names.contains(&name)
        })
        .collect()
}

/// The `boots.model` and `boots.mtl` bytes of `parts`, given in merge order: one part's model
/// as it is beside its material set; several merged into one model and one material set
/// (`pes_model::ops::merge`), a disagreement between them failing the task with its own code
/// (`From<MergeError>`). A model that does not read fails the task naming it.
fn boots_files(
    budget: &Arc<MemoryBudget>,
    mut parts: Vec<BootsPart>,
) -> Result<(Vec<u8>, Vec<u8>), TaskFailure> {
    if parts.len() == 1 {
        let part = parts.swap_remove(0);
        return Ok((part.model, part.materials.write()));
    }
    // The parsed parts, their merge and its written model, charged at the parts' source size,
    // an estimate of each form, as the Fox merge charges them (`model::package`).
    let _parts_charge = budget.charge(parts.iter().map(|part| part.model.len()).sum());
    let models = parts
        .iter()
        .map(|part| {
            PreFoxModel::read(&part.model)
                .and_then(|file| Model::from_file(&file))
                .map_err(|error| {
                    anyhow::anyhow!("cannot read the model {}: {error}", part.path.as_str()).into()
                })
        })
        .collect::<Result<Vec<Model>, TaskFailure>>()?;
    let pairs: Vec<(&Model, &MaterialSet)> = models
        .iter()
        .zip(parts.iter().map(|part| part.materials))
        .collect();
    let (model, materials) = merge(&pairs)?;
    let bytes = model
        .to_file()
        .and_then(|file| file.write())
        .map_err(|error| anyhow::anyhow!("cannot write boots.model: {error}"))?;
    Ok((bytes, materials.write()))
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
            MergeError::FlagsConflict => TaskFailure {
                code: Code::ModelMergeFlagsConflict,
                context: Vec::new(),
            },
            // A part that fails validation: not a disagreement between the parts a member
            // resolves, and the deep pass has dropped a folder whose `.model` does not read.
            MergeError::Other(_) => TaskFailure::from(anyhow::Error::from(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use pes_model::format::ModelError;

    use super::*;

    /// The bytes of `pes_model`'s fixture `name`.
    fn fixture(name: &str) -> Vec<u8> {
        fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../libs/pes_model/tests/fixtures")
                .join(name),
        )
        .unwrap()
    }

    /// The card-head template's face model: one bone, one mesh, one material, `card`.
    fn card() -> Model {
        Model::from_file(&PreFoxModel::read(&fixture("cardhead_face_high.model")).unwrap()).unwrap()
    }

    /// The card-head template's material set, defining `card`.
    fn card_set() -> MaterialSet {
        MaterialSet::read(&fixture("cardhead_materials.mtl")).unwrap()
    }

    fn written(model: &Model) -> Vec<u8> {
        model.to_file().unwrap().write().unwrap()
    }

    /// `boots_files` over several `parts`, each a model and the material set it uses, in this
    /// order, asserting the merge was charged at the parts' written size.
    fn boots_of(parts: &[(&Model, &MaterialSet)]) -> Result<(Vec<u8>, Vec<u8>), TaskFailure> {
        let paths: Vec<ScopePath> = (0..parts.len())
            .map(|index| ScopePath::new(&format!("Boots/Twin/part{index}_boots.model")).unwrap())
            .collect();
        let parts: Vec<BootsPart> = parts
            .iter()
            .zip(&paths)
            .map(|((model, materials), path)| BootsPart {
                path,
                model: written(model),
                materials,
            })
            .collect();
        let size: usize = parts.iter().map(|part| part.model.len()).sum();
        let budget = MemoryBudget::new(usize::MAX);
        let result = boots_files(&budget, parts);
        assert_eq!(budget.peak(), size);
        result
    }

    /// The card-head model and its material set with the material renamed `name`.
    fn card_named(name: &str) -> (Model, MaterialSet) {
        let mut model = card();
        model.materials[0] = name.to_owned();
        let mut set = card_set();
        set.materials[0].name = name.to_owned();
        (model, set)
    }

    /// The files of `result`, which must have succeeded.
    fn written_boots(result: Result<(Vec<u8>, Vec<u8>), TaskFailure>) -> (Vec<u8>, Vec<u8>) {
        match result {
            Ok(files) => files,
            Err(failure) => panic!("{:?} {:?}", failure.code, failure.context),
        }
    }

    #[test]
    fn one_boots_part_is_written_as_it_is_and_several_are_merged_in_order() {
        let source = fixture("cardhead_face_high.model");
        let (studs, studs_set) = card_named("studs");
        let set = card_set();
        let path = ScopePath::new("Boots/Twin/boots.model").unwrap();
        let one = vec![BootsPart {
            path: &path,
            model: source.clone(),
            materials: &set,
        }];
        // One part is not parsed, so nothing is charged.
        let budget = MemoryBudget::new(usize::MAX);
        let (model, materials) = written_boots(boots_files(&budget, one));
        assert_eq!(budget.peak(), 0);
        assert_eq!(model, source);
        assert_eq!(materials, set.write());

        let (model, materials) = written_boots(boots_of(&[(&card(), &set), (&studs, &studs_set)]));
        let merged = Model::from_file(&PreFoxModel::read(&model).unwrap()).unwrap();
        assert_eq!(merged.meshes.len(), 2);
        assert_eq!(merged.materials, ["card", "studs"]);
        let names: Vec<String> = MaterialSet::read(&materials)
            .unwrap()
            .materials
            .into_iter()
            .map(|material| material.name)
            .collect();
        assert_eq!(names, ["card", "studs"]);
    }

    #[test]
    fn a_player_s_own_gloves_file_replaces_a_combined_folder_s_of_its_packed_name() {
        // Each a packed name and whether the player's own folder holds it.
        let items = vec![
            ("glove_l.mtl", true),
            ("x_glover.mtl", true),
            ("glove_l.mtl", false),
            ("glove_r.mtl", false),
        ];
        let kept = without_replaced(items, |(name, own)| ((*name).to_owned(), *own));
        assert_eq!(
            kept,
            [
                ("glove_l.mtl", true),
                ("x_glover.mtl", true),
                ("glove_r.mtl", false)
            ]
        );
        // A shared folder's are all its own: two of one name are both kept, for `insert` to
        // refuse.
        let shared = vec![("glove_l.mtl", true), ("glove_l.mtl", true)];
        assert_eq!(
            without_replaced(shared.clone(), |(name, own)| ((*name).to_owned(), *own)),
            shared
        );
    }

    #[test]
    fn a_converted_glove_s_packed_name_is_lowercased_at_its_path_below_the_source() {
        let descriptor = |path: &str| FileDescriptor {
            path: ScopePath::new(path).unwrap(),
            source: ScopePath::new(path).unwrap(),
            size: 16,
            kind: FileKind::Model(ModelFormat::Fmdl),
        };
        let player = ScopePath::new("Players/05 - A").unwrap();
        let grip = ScopePath::new("Gloves/Grip").unwrap();
        let common = descriptor("Common/GLOVE_L.FMDL");
        let nested = descriptor("Gloves/Grip/Grip2/GLOVE_L.FMDL");
        fn model<'a>(file: &'a FileDescriptor, source_path: &'a ScopePath) -> SourceModel<'a> {
            SourceModel {
                file,
                xml_type: "gloveL".to_owned(),
                source_path,
                source_files: &[],
                skeleton: None,
            }
        }
        // A Common FMDL packs under its bare name, a source folder's own below the folder.
        assert_eq!(model(&common, &player).packed_name(), "glove_l.model");
        assert_eq!(model(&nested, &grip).packed_name(), "grip2/glove_l.model");
    }

    /// The code and context of `failure`.
    fn failure_of(failure: TaskFailure) -> (Code, Vec<(&'static str, String)>) {
        (failure.code, failure.context)
    }

    #[test]
    fn each_merge_conflict_fails_with_its_own_code_and_context() {
        let material = MergeError::MaterialConflict {
            name: "skin".to_owned(),
        };
        assert_eq!(
            failure_of(material.into()),
            (
                Code::MergeMaterialConflict,
                vec![("material", "skin".to_owned())]
            )
        );
        let bone = MergeError::SkeletonConflict {
            name: "sk_head".to_owned(),
        };
        assert_eq!(
            failure_of(bone.into()),
            (Code::SklMergeConflict, vec![("bone", "sk_head".to_owned())])
        );
        let repeated = MergeError::DuplicateBoneName {
            name: "sk_shoulder_l".to_owned(),
        };
        assert_eq!(
            failure_of(repeated.into()),
            (
                Code::SklMergeConflict,
                vec![("bone", "sk_shoulder_l".to_owned())]
            )
        );
        assert_eq!(
            failure_of(MergeError::FlagsConflict.into()),
            (Code::ModelMergeFlagsConflict, Vec::new())
        );
        assert_eq!(
            failure_of(MergeError::Other(ModelError::Truncated).into()),
            (
                Code::FolderPackFailed,
                vec![("error", "model is truncated".to_owned())]
            )
        );
        // Parts whose headers carry different flags.
        let mut flagged = card();
        flagged.flags = 4;
        let set = card_set();
        let Err(failure) = boots_of(&[(&card(), &set), (&flagged, &set)]) else {
            panic!("parts with different flags merged");
        };
        assert_eq!(
            failure_of(failure),
            (Code::ModelMergeFlagsConflict, Vec::new())
        );
    }
}
