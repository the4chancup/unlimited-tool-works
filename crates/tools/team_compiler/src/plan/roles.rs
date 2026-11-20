//! The role of each file of a model folder in the output of the target's engine: a player
//! folder's, a shared `Faces/`, `Boots/` or `Gloves/` folder's, and the `Common/` files that a
//! `.common` link names. One classification, which validation reads for its
//! findings (a file with no role is `file_not_used`) and planning for each task's files, so
//! the two never disagree: a file no role names is never read.

use std::collections::BTreeSet;

use aesthetics_export::{
    FileDescriptor, FileKind, ModelFormat, ModelSuffix, PlayerFolder, SharedKind, SharedLink,
    SharedModelFolder, ValidatedAestheticsExport, ValidatedRoster, classify, common_link_files,
    common_link_name, common_link_target, is_player_singleton, kit_token, model_suffix,
};
use dds_convert::SourceFormat;
use pes_version::Engine;
use vtree::ScopePath;

use crate::deep::relative;
use crate::face_xml;
use crate::kit_variants::model_variant_sets;

/// The kit texture stems `compile` builds: first the five the kit config names, in the order
/// of its texture-name fields, then the two maps the games read by name convention alone,
/// `kit_mask` (PES 15-17) and `kit_srm` (PES 18-21). Planning drops, before the kit's task is
/// made, the map the target's engine does not read and any `kit_*` stem outside these seven.
pub(crate) const KIT_TEXTURE_STEMS: [&str; 7] = [
    "kit",
    "kit_back",
    "kit_chest",
    "kit_leg",
    "kit_name",
    "kit_mask",
    "kit_srm",
];

/// Whether a target of `engine` emits a kit texture of `stem`: one of `KIT_TEXTURE_STEMS` but
/// the other engine's map (`kit_srm` on PES 15-17, `kit_mask` on PES 18-21; neither is
/// converted into the other). Planning drops every other kit texture before the kit's task is
/// made, and the deep pass does not check one: nothing reads it.
pub(crate) fn emits_kit_texture(engine: Engine, stem: &str) -> bool {
    let other_engine_map = match engine {
        Engine::PreFox => "kit_srm",
        Engine::Fox => "kit_mask",
    };
    stem != other_engine_map && KIT_TEXTURE_STEMS.contains(&stem)
}

/// The source format of the texture file `name`, by its extension in any case: one of the
/// image formats `dds_convert` converts (`spec/libs/dds_convert.md` "Accepted image formats");
/// `None` for a name without one.
pub(crate) fn texture_format(name: &str) -> Option<SourceFormat> {
    let (_, extension) = name.rsplit_once('.')?;
    SourceFormat::from_extension(extension)
}

/// The three Fox packages a player folder's models go to, each a `.fpk` and `.fpkd` pair in
/// its own game folder: the face by player id, the boots and the gloves by the player's
/// planned model id, all three of a referee's by his slot (`paths::PackageKey`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModelPackage {
    /// `face/real/{player id}/#Win/face.fpk`.
    Face,
    /// `boots/k{id}/#Win/boots.fpk`.
    Boots,
    /// `glove/g{id}/#Win/glove.fpk`.
    Gloves,
}

impl ModelPackage {
    /// The three packages in a folder's canonical order: the face, the boots, the gloves.
    pub(crate) const ALL: [ModelPackage; 3] = [
        ModelPackage::Face,
        ModelPackage::Boots,
        ModelPackage::Gloves,
    ];

    /// The first package of the canonical order `claim` holds for: the winner when several
    /// packages claim one texture destination.
    pub(crate) fn first_claimed(claim: impl Fn(ModelPackage) -> bool) -> Option<ModelPackage> {
        Self::ALL.into_iter().find(|package| claim(*package))
    }

    /// The package's file stem: `face.fpk`, `boots.fpk`, `glove.fpk`.
    pub(crate) fn file_stem(self) -> &'static str {
        match self {
            ModelPackage::Face => "face",
            ModelPackage::Boots => "boots",
            ModelPackage::Gloves => "glove",
        }
    }

    /// The package as a finding names it (`shared_texture_conflict`'s `packages`): `face`,
    /// `boots`, `gloves`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            ModelPackage::Face => "face",
            ModelPackage::Boots => "boots",
            ModelPackage::Gloves => "gloves",
        }
    }
}

/// The package a shared folder of `kind` is loaded as: `Boots/` holds boots models, `Gloves/`
/// gloves models, `Faces/` face parts.
pub(crate) fn package_of(kind: SharedKind) -> ModelPackage {
    match kind {
        SharedKind::Face => ModelPackage::Face,
        SharedKind::Boots => ModelPackage::Boots,
        SharedKind::Gloves => ModelPackage::Gloves,
    }
}

/// The kind of the shared folder loaded as `package` (`package_of`'s inverse): a shared output
/// or a combined folder knows the package it feeds, and its kind decides its per-kit models
/// (`FolderModels::of_shared`).
pub(crate) fn shared_kind_of(package: ModelPackage) -> SharedKind {
    match package {
        ModelPackage::Face => SharedKind::Face,
        ModelPackage::Boots => SharedKind::Boots,
        ModelPackage::Gloves => SharedKind::Gloves,
    }
}

/// The link file for the shared folder `name` of `kind`, as the export spells it without the
/// tolerated `.txt` tail (`Crocs.boots`).
pub(crate) fn link_name(kind: SharedKind, name: &str) -> String {
    let extension = match kind {
        SharedKind::Face => "face",
        SharedKind::Boots => "boots",
        SharedKind::Gloves => "gloves",
    };
    format!("{name}.{extension}")
}

/// Whether `player`'s `link`, `player` being a player folder of `export`, combines on a target
/// of `engine`: the shared folder's models become parts of the player's own package instead of
/// the shared output being loaded as it is. A face link always does, on both engines, a shared
/// face having no output of its own: on Fox it is merged into the player's face, on pre-Fox
/// copied into his face CPK. A boots or gloves link does when the player's effective package of
/// that kind has a part (`has_effective_part`). On Fox that part is one of his own, a linked
/// face's boots or gloves model, or for his gloves the hands split off a face model. On pre-Fox
/// it is only a part `ingame_face` gives him: without the marker his and a linked face's boots
/// and gloves models are parts of his face, and the link keeps its plain meaning.
/// `hand_weighted` is the models the deep pass found carrying hand weights.
pub(crate) fn link_combines(
    export: &ValidatedAestheticsExport,
    player: &PlayerFolder,
    link: &SharedLink,
    engine: Engine,
    hand_weighted: &BTreeSet<ScopePath>,
) -> bool {
    match link.kind {
        SharedKind::Face => true,
        SharedKind::Boots | SharedKind::Gloves => {
            has_effective_part(export, player, package_of(link.kind), engine, hand_weighted)
        }
    }
}

/// Whether the shared folder `player`'s `link` names is built, for `engine`, into the
/// player's own package of its kind instead of compiled on its own under a shared id;
/// `player` is a player folder of `export`. A referee's every link is: he has no team block to
/// give the shared folder an id of its own, so it is written as his slot's `k99NN`/`g99NN`.
/// A team player's link is when it combines
/// (`link_combines`, with `hand_weighted`).
pub(crate) fn link_feeds_own_package(
    export: &ValidatedAestheticsExport,
    engine: Engine,
    player: &PlayerFolder,
    link: &SharedLink,
    hand_weighted: &BTreeSet<ScopePath>,
) -> bool {
    match export.roster {
        ValidatedRoster::Referees(_) => true,
        ValidatedRoster::Team(_) => link_combines(export, player, link, engine, hand_weighted),
    }
}

/// The shared folder `link` names in `export`, matched as validation matched it: by
/// case-folded name. `None` only for a link validation did not resolve, and it drops the
/// folder of such a link.
pub(crate) fn linked_folder<'a>(
    export: &'a ValidatedAestheticsExport,
    link: &SharedLink,
) -> Option<&'a SharedModelFolder> {
    let folders = match link.kind {
        SharedKind::Face => &export.faces,
        SharedKind::Boots => &export.boots,
        SharedKind::Gloves => &export.gloves,
    };
    let name_key = vtree::fold_name(&link.name);
    folders
        .iter()
        .find(|folder| vtree::fold_name(&folder.folder_name) == name_key)
}

/// The shared folders `player`, a player folder of `export` compiled for a target of `engine`,
/// builds his own packages from, in link order, resolved as planning resolves them
/// (`linked_folder`): his face link's, and each boots or gloves link's that feeds his own
/// package (`link_feeds_own_package`, with `hand_weighted`), the `ModelFolder::combined`
/// planning gives him.
pub(crate) fn combined_folders<'a>(
    export: &'a ValidatedAestheticsExport,
    player: &PlayerFolder,
    engine: Engine,
    hand_weighted: &BTreeSet<ScopePath>,
) -> Vec<(SharedKind, &'a SharedModelFolder)> {
    player
        .links
        .iter()
        .filter(|link| link_feeds_own_package(export, engine, player, link, hand_weighted))
        .filter_map(|link| Some((link.kind, linked_folder(export, link)?)))
        .collect()
}

/// Every shared folder of `export` with its kind: the faces, then the boots, then the gloves,
/// each in the export's order.
pub(crate) fn shared_folders(
    export: &ValidatedAestheticsExport,
) -> impl Iterator<Item = (SharedKind, &SharedModelFolder)> {
    let faces = export.faces.iter().map(|folder| (SharedKind::Face, folder));
    let boots = export
        .boots
        .iter()
        .map(|folder| (SharedKind::Boots, folder));
    let gloves = export
        .gloves
        .iter()
        .map(|folder| (SharedKind::Gloves, folder));
    faces.chain(boots).chain(gloves)
}

/// What one file of a player folder becomes in the output of the target's engine.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PlayerFile {
    /// A model, packed into `package` as `<name>.fmdl` with its texture paths rewritten:
    /// `name` is its allowed name, the free prefix stripped (`kit_boots.fmdl` packs as
    /// `boots.fmdl`). A `.model` with no `.fmdl` of its path stem beside it takes the role too:
    /// the Models task converts it to an FMDL with the `.mtl` its search finds.
    Model {
        package: ModelPackage,
        name: &'static str,
    },
    /// A `.common` link to an FMDL or a `.model` (`legs.fmdl.common` → `Common/legs.fmdl`,
    /// `legs.model.common` → `Common/legs.model`): the Common model is a part of `package` under
    /// `name`, merged with the folder's parts of that name, since Fox cannot load a model from
    /// Common. A Common `.model` is first converted with the `.mtl` its search finds, as the
    /// player's own `.model` is. The
    /// role is the link's; the Models task reads the Common model, which planning resolves
    /// (`plan::CommonModel`, an FMDL of its stem beating a linked `.model`), never the empty
    /// link.
    CommonModel {
        package: ModelPackage,
        name: &'static str,
    },
    /// A face file packed into `package` as it is, under `name`: `face_diff.bin` and
    /// `fcl_hair_sim.fclo`. Not a part to merge: the package holds one, the player folder's
    /// own over a combined face folder's.
    Packed {
        package: ModelPackage,
        name: &'static str,
    },
    /// A `face_diff.xml`: the face's `face_diff.bin` given as text, the base64 of the binary
    /// file, decoded and packed into the face package as `face_diff.bin`. It goes wherever a
    /// `face_diff.bin` would, and stands for one: the player folder's own, in either form, over
    /// a combined face folder's.
    FaceDiffXml,
    /// A `face_diff.bin`, `face_diff.xml` or `fcl_hair_sim.fclo` in a folder with no face
    /// model and no face link, `ingame_face` or not: there is no face for it to shape, so it
    /// is reported as `face_file_not_used` by the structure pass and never read. A player
    /// folder's blank face takes the bundled face diff. A shared folder's is reported the same
    /// way.
    UnusedFaceFile,
    /// A model's skeleton, the `.skl` named after a `fcl_hair` or `boots` part, packed into
    /// `package` under its slot's `name` (`fcl_hair_sim.skl`, `boots.skl`). Every part may
    /// bring one, and the merge decides whose is packed: parts merged into one output must
    /// share one skeleton.
    Skeleton {
        package: ModelPackage,
        name: &'static str,
    },
    /// The `.skl` named after a `face_high`, `hair_high` or `oral` model, which have no
    /// skeleton slot: reported as `skl_no_slot` by the structure pass, and never read.
    SlotlessSkeleton,
    /// A per-kit model other than its set's fallback (`pants_kit2.fmdl` or
    /// `pants_kit2.model` beside `pants_kit1.fmdl`, or any variant beside `pants_kitF`)
    /// where no `face.xml` names the set (on Fox;
    /// on pre-Fox under `ingame_face` and in a shared boots or gloves folder,
    /// `leaves_out_kit_variants`): nothing can switch models with the kit, so planning reports
    /// `kit_variant_model_left_out` and the file is never read
    /// (`kit_variants::model_variant_sets`).
    LeftOutKitVariant,
    /// A texture, converted once into the folder's texture home at its path below its source
    /// folder (`spec/aesthetics_export/player_folders.md` "Subfolders"). A model's texture name
    /// resolves to it nearest first (`texture_lookup`).
    Texture {
        /// The texture's path below its source folder without its extension, as spelled:
        /// `jessie/skin` for `Players/05 - A/jessie/skin.dds` or a shared folder's
        /// `Faces/Round/jessie/skin.dds`, `skin` for a file directly in the folder. Not a
        /// stem: two textures of a tree may share a stem in different folders of it.
        below: String,
        /// The format the texture is converted from.
        format: SourceFormat,
    },
    /// A `.common` link to a texture (`hair.dds.common` → `Common/hair.dds`,
    /// `jessie/hair.dds.common` → `Common/jessie/hair.dds`): the linked texture's path below
    /// `Common/` without its extension, as the link spells it (`hair`, `jessie/hair`), stands
    /// for the Common texture, which a Common textures task packs once in the team's Common
    /// output (`spec/model_format.md` "Link files (`.common`)"). In the folder the link sits in it
    /// counts as the texture being present there under its name, so the folder's models point
    /// the name at the texture's path below the team's Common output; nothing reads the empty
    /// link.
    CommonTexture(String),
    /// Pre-Fox: a `.model`, packed into the player's face CPK as `oral_<stem>_win32.model`
    /// and listed in its generated `face.xml` under `xml_type` (`face_xml::xml_type`, by its
    /// name wherever it sits). Boots and gloves models included: the
    /// typed XML is why a pre-Fox player needs no boots or gloves folder of his own. In a shared
    /// boots or gloves folder it is a model of that output, typed for its `glove.xml`. An
    /// `.fmdl` with no `.model` of its stem beside it takes the role too: the task packing it
    /// converts it to a `.model` and its material set, packed as `<stem>.mtl` beside a face's
    /// or a glove's, merged into `boots.mtl` for the boots.
    PreFoxModel {
        /// The model's `face.xml` type.
        xml_type: String,
    },
    /// Pre-Fox, in a player folder holding `ingame_face`: a `.model`, or an `.fmdl` with no
    /// `.model` of its stem beside it, converted as the face converts one, that is a part of the
    /// player's own `package`, the one Fox gives it under the marker (`model_role`). The marker
    /// means no face, so there is no `face.xml` to list it. The boots take every model but a
    /// glove, a model the face would take included, and are written as one `boots.model`, its
    /// parts merged. The gloves are written unmerged, each model under its own name and listed
    /// in the gloves' `glove.xml`. The folder's `.mtl` files go into each of the two, which
    /// packs the ones its parts use (`TaskKind::files`).
    ///
    /// A `.common` link to a `.model` or an FMDL takes this role too, as a `.model` of the
    /// linked stem would in its place; a link to a per-kit model has no role (`pre_fox_link`).
    /// With no `face.xml` to name the Common path, the Common model is copied in as the part.
    /// Planning puts it in the link's place with the `.mtl` its search finds, or puts a Common
    /// FMDL there with the `.skl` of its stem, its bind pose, to convert as his own FMDL parts
    /// are (`ModelFolder::roles`).
    PreFoxPart {
        /// The package the model is a part of.
        package: ModelPackage,
        /// The type a `glove.xml` lists the model under, as it types a shared gloves folder's
        /// (`face_xml::xml_type`). The boots list no model, so a part of theirs leaves it
        /// unread.
        xml_type: String,
    },
    /// A `.mtl`. On pre-Fox it is packed into the player's face CPK under its
    /// own name with its texture paths pointed at the player's common folder, and the
    /// `face.xml` names it for each model whose search finds it (`mtl_search::mtl_for`). On Fox
    /// it is packed nowhere: a Models task converting a `.model` reads the folder's `.mtl`
    /// files (`TaskKind::files`) and gives the model the one its search finds, so one beside
    /// only FMDLs, a `.model` an FMDL beats included, is read by nothing. Planning gives a
    /// `Common/` `.mtl` this role too when a Common `.model` a link brings in takes it, on Fox
    /// and on pre-Fox under `ingame_face` (`ModelFolder::roles`).
    Material,
    /// Pre-Fox: a `.common` link to a `.model` (`legs.model.common` → `Common/legs.model`) or
    /// to an FMDL (`legs.fmdl.common` → `Common/legs.fmdl`), listed in the player's generated
    /// `face.xml` under `xml_type`, the type a `.model` of the linked stem would have in the
    /// link's place, at the team's Common output, where the export's Common models task packs
    /// the Common `.model` once, or converts the Common FMDL once into a `.model` and its
    /// material set (`selected_common_model` says which the link loads). The game loads it from
    /// there, so nothing is packed into the face and nothing reads the empty link. Under
    /// `ingame_face` such a link is a `PreFoxPart` instead.
    PreFoxCommonModel {
        /// The linked model's `face.xml` type.
        xml_type: String,
    },
    /// A `.common` link to a `.mtl` (`body.mtl.common` → `Common/body.mtl`), which the `.mtl`
    /// search counts as a `.mtl` of the linked name in the link's folder
    /// (`mtl_search::mtl_for`). On pre-Fox a model whose search finds it names the Common
    /// `.mtl` in the team's Common output; under `ingame_face` the Common `.mtl` is also copied
    /// into his boots and gloves, which pack it when a part of theirs uses it
    /// (`ModelFolder::roles`). On Fox a package converting a `.model` whose search finds it
    /// converts the model with it, reading the Common `.mtl` (`TaskKind::files`). Nothing reads
    /// the empty link.
    CommonMaterial,
    /// Pre-Fox: a member's own `face.xml`, directly in a player folder or a shared face folder
    /// (`role_position`). It is the authority on what the face loads, in place of the one the
    /// compiler generates; a shared face's is that of each face linking it. The deep pass reads
    /// and checks it (`user_face_xml`). Its folder
    /// has a face whatever models it holds (`FolderModels::of_player_files`).
    FaceXml,
    /// Pre-Fox: the `.skl` paired with an `.fmdl` converted (`PreFoxModel`, or `PreFoxPart`
    /// under `ingame_face`), by the path stem a Fox skeleton pairs by: the conversion's bind
    /// pose (`NativeModelBundle::Fox { skl }`), packed nowhere.
    ConversionSkeleton,
    /// A `.glb` or `.gltf` model with no model of the target's own format of its path stem
    /// beside it: the selected representation of its stem, which beats a model of the other
    /// engine's format beside it. The compiler does not read glTF yet, so planning drops the
    /// player folder holding one, or the shared folder holding one with every player folder
    /// linking it (`model_gltf_unsupported`), rather than compile the other format in its
    /// place: the same export never compiles differently once glTF is read.
    UnsupportedGltf,
}

impl PlayerFile {
    /// The package the file goes into; `None` for a texture, which goes to the player's
    /// common folder for every package to point at, for a `.common` link, whose file is the
    /// team's (Common's texture, `.model` or `.mtl`, packed once in the team's Common output),
    /// and for a skeleton with no slot, an unused face file, a left-out kit variant, a
    /// conversion's skeleton and a glTF, which go nowhere (the task converting its FMDL reads
    /// the conversion's skeleton, `TaskKind::files`; planning drops a glTF's folder). A Fox model
    /// link's role is its Common model's, and so is a pre-Fox one's under `ingame_face`, which
    /// planning puts in the link's place (`ModelFolder::roles`).
    pub(crate) fn package(&self) -> Option<ModelPackage> {
        match self {
            PlayerFile::Model { package, .. }
            | PlayerFile::CommonModel { package, .. }
            | PlayerFile::Packed { package, .. }
            | PlayerFile::Skeleton { package, .. }
            | PlayerFile::PreFoxPart { package, .. } => Some(*package),
            PlayerFile::FaceDiffXml
            | PlayerFile::PreFoxModel { .. }
            | PlayerFile::Material
            | PlayerFile::FaceXml => Some(ModelPackage::Face),
            PlayerFile::UnusedFaceFile
            | PlayerFile::SlotlessSkeleton
            | PlayerFile::LeftOutKitVariant
            | PlayerFile::Texture { .. }
            | PlayerFile::CommonTexture(_)
            | PlayerFile::PreFoxCommonModel { .. }
            | PlayerFile::CommonMaterial
            | PlayerFile::ConversionSkeleton
            | PlayerFile::UnsupportedGltf => None,
        }
    }
}

/// The name the skeleton of a part of `package` under `name` packs as: `fcl_hair_sim.skl` for
/// the hair, `boots.skl` for the boots, the two destinations with a skeleton slot. `None` for
/// `face_high`, `hair_high`, `oral` and the gloves.
pub(crate) fn skeleton_slot(package: ModelPackage, name: &str) -> Option<&'static str> {
    match (package, name) {
        (ModelPackage::Face, "fcl_hair") => Some("fcl_hair_sim.skl"),
        (ModelPackage::Boots, _) => Some("boots.skl"),
        (ModelPackage::Face | ModelPackage::Gloves, _) => None,
    }
}

/// The model a `.common` link named `link_name` stands for, a `.model` or an FMDL
/// (`legs.model.common` → `legs.model`, `legs.fmdl.common` → `legs.fmdl`), on either engine;
/// `None` for a link to anything else.
fn linked_model(link_name: &str) -> Option<String> {
    common_link_name(link_name).filter(|name| {
        matches!(
            classify(name),
            FileKind::Model(ModelFormat::PesModel | ModelFormat::Fmdl)
        )
    })
}

/// The path below `Common/` of the texture `file`, a `.common` link of the model folder at
/// `folder`, stands for, without its extension, as the link spells it (`hair.dds.common` →
/// `hair`, `jessie/hair.dds.common` → `jessie/hair`), when the linked name is in an image
/// format `dds_convert` converts; `None` for a link to anything else (a model, a material
/// file).
fn linked_texture_below(folder: &ScopePath, file: &FileDescriptor) -> Option<String> {
    let linked = common_link_target(&file.path, folder)?;
    texture_format(&linked)?;
    Some(file_stem(&linked).to_owned())
}

/// Whether `path`, a file of one of the export's root folders (`Common/`, `Collars/`), is
/// directly in that folder, not below a subfolder of it: the only place `compile` reads a
/// collar from, the textures directly in `Common/` being the ones a part's stems name in the
/// team's Common output. A player's or a kit's file sits at least two folders deep
/// (`Players/05 - A/x.mtl`), so this holds for none.
pub(crate) fn is_direct_root_folder_file(path: &ScopePath) -> bool {
    path.segments().count() == 2
}

/// Whether `path` is a file of the export's `Common/` folder, at any depth: a file a `.common`
/// link stands for, or a `.mtl` a Common model's search found there, rather than one of a
/// player or shared folder's own.
pub(crate) fn is_common_file(path: &ScopePath) -> bool {
    path.segments()
        .next()
        .is_some_and(|root| vtree::fold_name(root) == "common")
}

/// The export's `Common/` folder as a scope: the model folder a `Common/` `.mtl` resolves
/// its textures in (`prefox_face::FolderPlaces::common`, `deep::materials::held_common`).
/// Lookups compare it folded (`texture_lookup`), so it matches whatever case the export
/// spells the folder in (`is_common_file`).
pub(crate) fn common_folder() -> ScopePath {
    ScopePath::new("Common").expect("`Common` is a scope path")
}

/// The path of `path`, a file of the export's `Common/` folder, below that folder, as spelled
/// (`jessie/body.fmdl` for `Common/jessie/body.fmdl`, `body.fmdl` for a direct file): the path
/// a `.common` link names it by (`common_link_target`) and the one it has below the team's
/// Common output.
pub(crate) fn below_common(path: &ScopePath) -> &str {
    path.as_str().split_once('/').map_or("", |(_, below)| below)
}

/// Whether a target of `engine` reads the `Common/` file at `path` at all. On PES 15-17 every
/// directory of `Common/` is a Common folder of its own, packed at its own path, so any file
/// is read. On PES 18-21 a file directly in `Common/` is, and below a subfolder only one of
/// `linked` (`linked_common_files`), since Fox reaches Common through links alone.
pub(crate) fn is_read_common_file(
    path: &ScopePath,
    engine: Engine,
    linked: &BTreeSet<ScopePath>,
) -> bool {
    match engine {
        Engine::PreFox => true,
        Engine::Fox => is_direct_root_folder_file(path) || linked.contains(path),
    }
}

/// The `Common/` files below a subfolder that a standing `.common` link of a kept player or
/// shared folder of `export` reaches, by their export paths. This is what PES 18-21 reads of a
/// subfolder (`is_read_common_file`), computed once per export from the validated export.
///
/// A link mirrors `Common/`'s tree (`spec/model_format.md` "Link files"). It reaches the file
/// it names when it takes a role (`common_link_target`); on Fox, `player_file` gives a role
/// only to a link to a model, a `.mtl` or a texture. When a texture link's `kitN` stem or
/// directory names a set, it reaches every member of the set (`common_link_files`), and each
/// member's directory then counts as linked.
///
/// A link naming a model also reaches the files that travel with it in its directory: the
/// models of its stem in any format (the target selects one, `selected_common_model`), the
/// `.skl` of its stem, and every `.mtl`, since its `.mtl` search may land on any of them
/// (`mtl_search::mtl_for`). It also reaches every texture directly in its directory and in each
/// directory above it up to `Common/`, where its names resolve nearest first, and a link naming
/// a `.mtl` brings those as well. The shared model keeps the textures beside it, as `Common/`'s
/// own direct textures are packed whether a model names them or not.
pub(crate) fn linked_common_files(export: &ValidatedAestheticsExport) -> BTreeSet<ScopePath> {
    let mut linked = BTreeSet::new();
    for player in &export.players {
        let models = FolderModels::of_player_files(
            &player.path,
            &player.files,
            player.ingame_face.is_some(),
            Engine::Fox,
        );
        linked_common_of(
            &player.path,
            &player.files,
            &models,
            &export.common,
            &mut linked,
        );
    }
    // A shared folder's `.common` links reach `Common/` as a player's do, in its own
    // output's names and in each combined player's.
    for (kind, folder) in shared_folders(export) {
        let models = FolderModels::of_shared(&folder.path, &folder.files, kind, Engine::Fox);
        linked_common_of(
            &folder.path,
            &folder.files,
            &models,
            &export.common,
            &mut linked,
        );
    }
    linked
}

/// What the `.common` links among `files` of the folder at `folder` (a player folder's or a
/// shared one's, whose models are `models`) reach below `Common/`'s subfolders, pushed onto
/// `linked`: the body of `linked_common_files` per folder, the same for both.
fn linked_common_of(
    folder: &ScopePath,
    files: &[FileDescriptor],
    models: &FolderModels,
    common: &[FileDescriptor],
    linked: &mut BTreeSet<ScopePath>,
) {
    for file in files {
        if file.kind != FileKind::CommonLink || player_file(folder, file, models).is_none() {
            continue;
        }
        let Some(below) = common_link_target(&file.path, folder) else {
            continue;
        };
        // Every `Common/` file the link reaches (`common_link_files`), each member's
        // directory then counting as linked.
        for target in common_link_files(common, &below) {
            if is_direct_root_folder_file(&target.path) {
                continue;
            }
            linked.insert(target.path.clone());
            let (directory, stem) = directory_stem(target);
            // The directories above the target's own up to `Common/` (`Common/`'s own
            // direct textures are packed anyway): `Common/jessie` for
            // `Common/jessie/parts/body.fmdl`. A name resolves nearest first from the
            // file's own directory up to `Common/`, so a model or `.mtl` link brings the
            // textures directly in each of them too; a subfolder of any of them stays
            // out, a bare name never resolving down.
            let mut ancestors = BTreeSet::new();
            let mut above = target.path.parent().and_then(|own| own.parent());
            while let Some(directory_above) = above {
                if directory_above.segments().count() == 1 {
                    break;
                }
                ancestors.insert(directory_above.fold_key());
                above = directory_above.parent();
            }
            let travelling = common.iter().filter(|other| {
                let (other_directory, other_stem) = directory_stem(other);
                let in_ancestor = matches!(target.kind, FileKind::Model(_) | FileKind::Mtl)
                    && other.kind == FileKind::Texture
                    && other_directory
                        .as_ref()
                        .is_some_and(|other| ancestors.contains(other.as_str()));
                in_ancestor
                    || other_directory == directory
                        && match target.kind {
                            FileKind::Model(_) => {
                                other.kind == FileKind::Texture
                                    || other.kind == FileKind::Mtl
                                    || (other_stem == stem
                                        && matches!(other.kind, FileKind::Model(_) | FileKind::Skl))
                            }
                            FileKind::Mtl => other.kind == FileKind::Texture,
                            FileKind::Texture
                            | FileKind::Skl
                            | FileKind::Fclo
                            | FileKind::Xml
                            | FileKind::MaterialsToml
                            | FileKind::Bin
                            | FileKind::SharedLink(_)
                            | FileKind::CommonLink
                            | FileKind::Marker(_)
                            | FileKind::Metadata(_)
                            | FileKind::Other => false,
                        }
            });
            linked.extend(travelling.map(|other| other.path.clone()));
        }
    }
}

/// The file of `common` (the export's `Common/` files) a `.common` link's target `below`
/// below `Common/` (`body.fmdl` directly in it, `jessie/body.fmdl` in its subfolder
/// `jessie/`) names, matched as validation matches it (`common_link_files`'s first, the
/// representative): the file at that path for a non-texture target, the kit set's
/// representative for a texture one.
pub(crate) fn common_file<'a>(
    common: &'a [FileDescriptor],
    below: &str,
) -> Option<&'a FileDescriptor> {
    common_link_files(common, below).first().copied()
}

/// The path below `Common/` of the texture `below` names (`PlayerFile::CommonTexture`'s,
/// without its extension), as `Common/` spells it: `jessie/hair` for `Common/jessie/Hair.dds`.
/// `below` itself when `common`, the folder's `common_files`, holds no file of its stem: the
/// Fox installed case, a link an installed CPK satisfies, whose target is not in the export
/// (`common_link_missing`'s exception).
pub(crate) fn common_texture_below(common: &[FileDescriptor], below: &str) -> String {
    let key = vtree::fold_name(below);
    common
        .iter()
        .find(|file| {
            file.kind == FileKind::Texture
                && vtree::fold_name(file_stem(below_common(&file.path))) == key
        })
        .map(|file| file_stem(below_common(&file.path)).to_owned())
        .unwrap_or_else(|| below.to_owned())
}

/// The `.skl` beside the Common model at `model` below `Common/`, in its directory, paired with
/// it by stem (`legs.skl` for `legs.fmdl`, `jessie/legs.skl` for `jessie/legs.fmdl`), when
/// there is one: the skeleton that travels with a `.common` link.
pub(crate) fn common_skeleton<'a>(
    common: &'a [FileDescriptor],
    model: &str,
) -> Option<&'a FileDescriptor> {
    common_file(common, &format!("{}.skl", file_stem(model)))
}

/// The Common model a `.common` link to the model at `linked` below `Common/`
/// (`common_link_target`) loads on a target of `engine`, among `common` (the export's `Common/`
/// files): the file at that path, found as validation found it, unless a model of its stem
/// beside it in the target's own format beats it: selection is per stem, target-native first,
/// so a `.model` beats an FMDL on pre-Fox and an FMDL a `.model` on Fox. Such a link loads the
/// target's own model, found by the linked path stem, and the beaten one is ignored as a
/// player folder's beaten model is. `None` when `common` holds no file at the path.
pub(crate) fn selected_common_model<'a>(
    common: &'a [FileDescriptor],
    linked: &str,
    engine: Engine,
) -> Option<&'a FileDescriptor> {
    let named = common_file(common, linked)?;
    let beaten = other_format(engine);
    let native_extension = match engine {
        Engine::Fox => "fmdl",
        Engine::PreFox => "model",
    };
    if named.kind != FileKind::Model(beaten) {
        return Some(named);
    }
    let native = format!("{}.{native_extension}", file_stem(linked));
    Some(common_file(common, &native).unwrap_or(named))
}

/// Whether `file`, a model among `common` (the export's `Common/` files), is the one a target
/// of `engine` selects for its stem in its own directory, by the rule `selected_common_model`
/// applies to a link: false for a model of the other engine's format beside a target-native
/// one of its stem, which nothing reads. Compared within the directory, not directly in
/// `Common/` alone: on PES 15-17 each subfolder is a Common folder of its own
/// (`is_read_common_file`).
pub(crate) fn is_selected_common_model(
    common: &[FileDescriptor],
    file: &FileDescriptor,
    engine: Engine,
) -> bool {
    if file.kind != FileKind::Model(other_format(engine)) {
        return true;
    }
    let native = FileKind::Model(native_format(engine));
    !common
        .iter()
        .any(|other| other.kind == native && directory_stem(other) == directory_stem(file))
}

/// The directory of `file` and its stem, both case-folded: what tells two models of one stem in
/// one `Common/` directory, the unit per-stem selection compares within.
pub(crate) fn directory_stem(file: &FileDescriptor) -> (Option<String>, String) {
    (
        file.path.parent().map(|parent| parent.fold_key()),
        vtree::fold_name(file_stem(file.path.name())),
    )
}

/// Whether `file` sits below the folder at `folder`, at any depth: where a player folder's files
/// take their roles, since a subfolder of any name is a player folder of its own merged into
/// the root player's (`spec/aesthetics_export/player_folders.md` "Subfolders").
fn below(folder: &ScopePath, file: &FileDescriptor) -> bool {
    file.path
        .as_str()
        .strip_prefix(folder.as_str())
        .is_some_and(|rest| rest.starts_with('/'))
}

/// Whether `file` sits directly in the folder at `folder`.
fn directly_in(folder: &ScopePath, file: &FileDescriptor) -> bool {
    file.path.parent().as_ref() == Some(folder)
}

/// Whether `file` of the model folder at `folder` sits where it may
/// take a role: anywhere below it (`below`), a shared folder's tree following a player
/// folder's. A `.common` link is included, standing for the `Common/` file at its own path
/// below the folder (`common_link_target`). The per-player
/// singletons (`face.xml`, `face_diff.bin`, `face_diff.xml`, `fcl_hair_sim.fclo`,
/// `is_player_singleton`) take a role directly in the folder alone: a folder has one of each,
/// and a subfolder's, kept by a lenient check, would be a second. A file the allowlist does
/// not admit is one the structure pass names (`file_type_disallowed`), so validation does not
/// report it again as `file_not_used`.
pub(crate) fn role_position(folder: &ScopePath, file: &FileDescriptor) -> bool {
    if is_player_singleton(file.path.name()) {
        return directly_in(folder, file);
    }
    below(folder, file)
}

/// Whether `file` of the folder at `folder` is a member's own `face.xml`: named so in any case,
/// where a face's files go (`role_position`). Pre-Fox reads a
/// player folder's and a shared face folder's (`PlayerFile::FaceXml`), the latter the xml of
/// each face linking it; Fox has no `face.xml` and ignores it (`xml_ignored_fox`). A shared
/// boots or gloves folder's is ignored on pre-Fox too (`xml_ignored_shared`): its output is one
/// model or a `glove.xml`, which no face xml drives.
pub(crate) fn is_user_face_xml(folder: &ScopePath, file: &FileDescriptor) -> bool {
    file.kind == FileKind::Xml
        && file.path.name().eq_ignore_ascii_case("face.xml")
        && role_position(folder, file)
}

/// The files among `files` of the folder at `folder` where a
/// model may take a role (`role_position`): the per-kit model files there are one source of a
/// part's sets (`FolderModels::of_part_source`), and a file nothing reads is no variant.
pub(crate) fn role_files<'a>(
    folder: &ScopePath,
    files: &'a [FileDescriptor],
) -> Vec<&'a FileDescriptor> {
    files
        .iter()
        .filter(|file| role_position(folder, file))
        .collect()
}

/// The package and allowed name a model named `<free part>_<suffix>` goes to by its suffix
/// alone (`spec/aesthetics_export/player_folders.md` "Model names").
fn suffix_role(suffix: Option<ModelSuffix>) -> (ModelPackage, &'static str) {
    match suffix {
        Some(ModelSuffix::FaceHigh) => (ModelPackage::Face, "face_high"),
        Some(ModelSuffix::HairHigh) => (ModelPackage::Face, "hair_high"),
        Some(ModelSuffix::Oral) => (ModelPackage::Face, "oral"),
        // A model whose name says nothing about what it is is face content, a part of the
        // hair merge; the structure pass reports each one as `fmdl_fcl_hair_fallback`.
        Some(ModelSuffix::FclHair) | None => (ModelPackage::Face, "fcl_hair"),
        Some(ModelSuffix::Boots) => (ModelPackage::Boots, "boots"),
        // Hand-skeleton models have nowhere else to go on Fox.
        Some(ModelSuffix::GloveL | ModelSuffix::HandL) => (ModelPackage::Gloves, "glove_l"),
        Some(ModelSuffix::GloveR | ModelSuffix::HandR) => (ModelPackage::Gloves, "glove_r"),
    }
}

/// The package and allowed name of the model with file stem `stem`, wherever it sits in its
/// folder: by its suffix (`suffix_role`). When `ingame_face` is set (the folder holds the
/// marker), a model that would be a part of the face's `fcl_hair` is a part of the boots named
/// `boots`.
fn model_role(stem: &str, ingame_face: bool) -> (ModelPackage, &'static str) {
    let role = suffix_role(model_suffix(stem));
    // The marker means no face package: the hair's parts would be lost, and the boots take
    // the full body skeleton as the hair does.
    // The explicit face names have no such home; validation drops a marked folder holding one.
    if ingame_face && role == (ModelPackage::Face, "fcl_hair") {
        return (ModelPackage::Boots, "boots");
    }
    role
}

/// `file`'s export path up to its extension (`Players/05 - A/boots/kit_boots`): what a model
/// and the skeleton named after it share, and nothing in another directory does.
pub(crate) fn path_stem(file: &FileDescriptor) -> &str {
    file_stem(file.path.as_str())
}

/// What a player folder's models say about its other files, for the target engine they were
/// computed for: which have a package to go in, and which model a `.skl` pairs with. Computed
/// once per folder and passed to `player_file`, which gives the roles of that engine. A model
/// is listed by its `path_stem`, so a skeleton pairs with the model beside it, not with one of
/// the same name in another of the folder's directories.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FolderModels {
    /// The engine of the target the models were read for: each engine builds its own model
    /// format, so the same files give other models, and `player_file` other roles.
    engine: Engine,
    /// The player folder holds `ingame_face`, so its models take the roles `model_role` gives
    /// under the marker. A shared folder has none.
    ingame_face: bool,
    /// The kind of the shared folder when the folder is a shared `Faces/`, `Boots/` or
    /// `Gloves/` one; `None` for a player folder. On Fox a shared folder's `.common` links
    /// take the roles a player folder's take (`fox_file`); on pre-Fox every one stays
    /// unread (`file_not_used`).
    shared: Option<SharedKind>,
    /// The folder's face files have a package to go in: it holds a face model, on pre-Fox its
    /// own `face.xml`, or links a shared face that gives one (`with_linked_face`). Never set
    /// for a shared boots or gloves folder, which gives no face.
    face: bool,
    /// The path stems of its `fcl_hair` parts, a model with no recognized suffix included,
    /// since that is face content the hair merge takes. Each one's skeleton packs as
    /// `fcl_hair_sim.skl`: several parts merge into one `fcl_hair.fmdl`, and the merge decides
    /// whose skeleton that is.
    hair_stems: Vec<String>,
    /// The path stems of its boots models, each of whose skeletons packs as `boots.skl`, by
    /// the same rule.
    boots_stems: Vec<String>,
    /// The path stems of its `face_high`, `hair_high` and `oral` models, which have no
    /// skeleton slot: a skeleton named after one is `skl_no_slot`. The gloves have none
    /// either, and no `.skl` pairs with a glove.
    slotless_stems: Vec<String>,
    /// The export paths of its per-kit models other than their set's fallback (the `kitF`
    /// file, else the lowest variant), where no `face.xml` names the set
    /// (`leaves_out_kit_variants`: on Fox; on pre-Fox under `ingame_face` and in a shared boots
    /// or gloves folder): they are not models of the folder
    /// (`PlayerFile::LeftOutKitVariant`), no skeleton pairs with one and none makes a face.
    /// Empty for a pre-Fox face, which packs every variant.
    left_out_variants: Vec<ScopePath>,
    /// The path stems of its models in the target engine's own format, `.fmdl` files on Fox
    /// and `.model` files on pre-Fox, each of which beats a glTF or a model of the other format
    /// of its path stem, since selection is target-native first. The beaten model then has no
    /// role and no finding (TC-MOD-26).
    native_model_stems: Vec<String>,
    /// The path stems of its `.glb` and `.gltf` models, each of which beats a model of the
    /// other engine's format of its path stem and is beaten by one of the target's: selection
    /// is target-native first, then glTF.
    gltf_stems: Vec<String>,
    /// Pre-Fox: the path stems of the `.fmdl` files converted, by the face or a shared boots or
    /// gloves output (`PlayerFile::PreFoxModel`) or, under `ingame_face`, by his own boots or
    /// gloves (`PlayerFile::PreFoxPart`), each of whose skeletons is the conversion's bind pose
    /// (`PlayerFile::ConversionSkeleton`).
    converted_stems: Vec<String>,
}

impl FolderModels {
    /// The models among `files` of the shared folder of `kind` at `folder` (`Faces/`, `Boots/`
    /// or `Gloves/`), for a target of `engine`: as a player folder's without `ingame_face`,
    /// its `.common` links taking the roles `FolderModels::shared` gives them (a player
    /// folder's, on Fox; none on pre-Fox).
    pub(crate) fn of_shared(
        folder: &ScopePath,
        files: &[FileDescriptor],
        kind: SharedKind,
        engine: Engine,
    ) -> FolderModels {
        let sources = [role_files(folder, files)];
        FolderModels::of_part_source(folder, files, &sources, false, Some(kind), engine)
    }

    /// The models among `files` of the folder at `folder`, which holds `ingame_face` when
    /// `ingame_face` is set, for a target of `engine`. On Fox: its `.fmdl` files, its `.model`
    /// files with no `.fmdl` or glTF of their path stem, which the Models task converts, and its
    /// `.common` links to an FMDL or a `.model`, at any depth (`role_position`), each with its
    /// resolved role. A link counts as a model of its role's package, but pairs no
    /// skeleton of the folder's: a Common model's skeleton is Common's, resolved at planning
    /// (`common_skeleton`). A per-kit model other than its set's fallback is left
    /// out (`model_variant_sets`). On pre-Fox: its `.model` files, its `.fmdl` files with no
    /// `.model` or glTF of their path stem, which the face converts, and its `.common` links to a
    /// `.model` or an FMDL with a role, every one of them a part of the face (`PlayerFile::PreFoxModel`,
    /// `PlayerFile::PreFoxCommonModel`), every variant of a per-kit set included, or under
    /// `ingame_face` the parts of his boots and gloves, the folder having no face
    /// (`PlayerFile::PreFoxPart`), a per-kit set's fallback alone; only a converted FMDL
    /// pairs a skeleton, its bind pose.
    pub(crate) fn of_player_files(
        folder: &ScopePath,
        files: &[FileDescriptor],
        ingame_face: bool,
        engine: Engine,
    ) -> FolderModels {
        let sources = [role_files(folder, files)];
        FolderModels::of_part_source(folder, files, &sources, ingame_face, None, engine)
    }

    /// `of_player_files` for a player folder, or `of_shared` for a shared one of the kind
    /// `shared` names, the folder being one source of
    /// a part whose per-kit model sets span `sources` (`kit_variants::model_variant_sets`):
    /// the files of every folder the part is built from that may take a role there
    /// (`role_files`), this folder's among them, the player's own first and then each combined
    /// shared folder's. A set split between them is one set, its variants in this folder left
    /// out but for its lowest (`ModelFolder::roles`).
    pub(crate) fn of_part_source(
        folder: &ScopePath,
        files: &[FileDescriptor],
        sources: &[Vec<&FileDescriptor>],
        ingame_face: bool,
        shared: Option<SharedKind>,
        engine: Engine,
    ) -> FolderModels {
        let mut models = FolderModels {
            engine,
            ingame_face,
            shared,
            face: false,
            hair_stems: Vec::new(),
            boots_stems: Vec::new(),
            slotless_stems: Vec::new(),
            left_out_variants: if leaves_out_kit_variants(engine, ingame_face, shared) {
                model_variant_sets(sources, engine)
                    .into_iter()
                    .flat_map(|set| set.left_out)
                    .collect()
            } else {
                Vec::new()
            },
            // Read before the loop: on pre-Fox `boots.fmdl` sorts before the `boots.model`
            // or `boots.glb` beating it.
            native_model_stems: model_stems(folder, files, native_format(engine)),
            gltf_stems: model_stems(folder, files, ModelFormat::Gltf),
            converted_stems: Vec::new(),
        };
        for file in files {
            if !role_position(folder, file) {
                continue;
            }
            if models.left_out_variants.contains(&file.path) {
                continue;
            }
            match engine {
                Engine::Fox => {}
                Engine::PreFox => {
                    // An FMDL is converted where its role puts it: the face, or under the
                    // marker his boots or gloves (`pre_fox_part`).
                    let converted =
                        file.kind == FileKind::Model(ModelFormat::Fmdl) && !models.beaten(file);
                    if converted {
                        models
                            .converted_stems
                            .push(vtree::fold_name(path_stem(file)));
                    }
                    // Under the marker there is no face for any model to make (`PreFoxPart`),
                    // so the face files are not used.
                    if ingame_face {
                        continue;
                    }
                    let model_link = file.kind == FileKind::CommonLink
                        && !models.is_shared()
                        && matches!(
                            pre_fox_link(folder, file, false),
                            Some(PlayerFile::PreFoxCommonModel { .. })
                        );
                    // A folder holding its own `face.xml` has a face whatever models it
                    // holds: the xml may name only Common models.
                    models.face |= model_link
                        || file.kind == FileKind::Model(ModelFormat::PesModel)
                        || converted
                        || is_user_face_xml(folder, file);
                    continue;
                }
            }
            let file_name = file.path.name();
            // A `.model` an FMDL or a glTF of its path stem beats is no model of the folder's.
            let converted =
                file.kind == FileKind::Model(ModelFormat::PesModel) && !models.beaten(file);
            let (model_name, local) =
                if file.kind == FileKind::Model(ModelFormat::Fmdl) || converted {
                    (file_name.to_owned(), true)
                } else if file.kind == FileKind::CommonLink
                    && let Some(linked) = linked_model(file_name)
                {
                    (linked, false)
                } else {
                    continue;
                };
            let (package, name) = model_role(file_stem(&model_name), ingame_face);
            if !local {
                models.face |= package == ModelPackage::Face;
                continue;
            }
            // The stems fold, the way the file system folds the pair: `boots.fmdl`
            // beside `Boots.skl` is one model and its skeleton.
            let stem = vtree::fold_name(path_stem(file));
            match (package, name) {
                (ModelPackage::Face, "fcl_hair") => {
                    models.face = true;
                    models.hair_stems.push(stem);
                }
                (ModelPackage::Face, _) => {
                    models.face = true;
                    models.slotless_stems.push(stem);
                }
                (ModelPackage::Boots, _) => models.boots_stems.push(stem),
                (ModelPackage::Gloves, _) => {}
            }
        }
        // A shared boots or gloves folder's output is one boots model or a `glove.xml`, and
        // no face, whatever models it holds: its face files and its own `face.xml` are not
        // used.
        if models.is_shared_boots_or_gloves() {
            models.face = false;
        }
        models
    }

    /// The engine of the target the models were read for.
    pub(crate) fn engine(&self) -> Engine {
        self.engine
    }

    /// Whether the models are a shared `Faces/`, `Boots/` or `Gloves/` folder's
    /// (`FolderModels::of_shared`), whose `.common` links have no role on pre-Fox.
    pub(crate) fn is_shared(&self) -> bool {
        self.shared.is_some()
    }

    /// Whether the models are a shared `Boots/` or `Gloves/` folder's, whose `face.xml` no
    /// face reads on pre-Fox (`xml_ignored_shared`): its output is one model or a
    /// `glove.xml`.
    pub(crate) fn is_shared_boots_or_gloves(&self) -> bool {
        matches!(self.shared, Some(SharedKind::Boots | SharedKind::Gloves))
    }

    /// Whether `file` is a model another representation of its path stem beats, selection being
    /// target-native first, then glTF, then the other engine's format. That is a glTF with a
    /// model of the target's format beside it, or a model in the other
    /// engine's format, a `.model` on Fox or an `.fmdl` on pre-Fox, with a model of the
    /// target's format or a glTF beside it. The beaten model has no role and no finding
    /// (TC-MOD-26): nothing reads it.
    pub(crate) fn beaten(&self, file: &FileDescriptor) -> bool {
        let FileKind::Model(format) = file.kind else {
            return false;
        };
        if format == native_format(self.engine) {
            return false;
        }
        let stem = vtree::fold_name(path_stem(file));
        let native_beside = self.native_model_stems.contains(&stem);
        match format {
            ModelFormat::Gltf => native_beside,
            ModelFormat::Fmdl | ModelFormat::PesModel => {
                native_beside || self.gltf_stems.contains(&stem)
            }
        }
    }

    /// The models of a player folder linking a shared face that gives a face (its own models
    /// give one, `FolderModels::face`). The shared face is then the player's whole face, so
    /// the folder's
    /// `face_diff.bin` and `fcl_hair_sim.fclo` have a package to go in with no face model of
    /// the folder's own.
    pub(crate) fn with_linked_face(mut self) -> FolderModels {
        self.face = true;
        self
    }
}

/// The models of a model folder and of each shared folder it combines, as planning reads them
/// (`ModelFolder::roles`) and the deep pass checks them, for a target of `engine`: the folder
/// at `folder` holding `files`, a shared one of the kind `own_kind` or a player folder
/// (`None`) holding `ingame_face` when `ingame_face` is set, and `combined`, the shared folders
/// it combines, in link order. Returned: the folder's own models, a combined face that gives a
/// face counting as a face model of the folder's (`with_linked_face`), then each combined
/// folder's, in `combined`'s order.
pub(crate) fn part_source_models(
    folder: &ScopePath,
    files: &[FileDescriptor],
    own_kind: Option<SharedKind>,
    ingame_face: bool,
    combined: &[(SharedKind, &SharedModelFolder)],
    engine: Engine,
) -> (FolderModels, Vec<FolderModels>) {
    // Where no `face.xml` names a per-kit set, the part merges every source's models, so a
    // set split between the player's own files and a combined folder's is one set. A
    // pre-Fox face lists every variant, and a pre-Fox referee's combined boots or gloves
    // are an output of their own, not parts of his face: there each source is its own.
    let part_files: Vec<Vec<&FileDescriptor>> = std::iter::once(role_files(folder, files))
        .chain(
            combined
                .iter()
                .map(|(_, shared)| role_files(&shared.path, &shared.files)),
        )
        .collect();
    let spans_sources = leaves_out_kit_variants(engine, ingame_face, own_kind);
    let part_sources = |index: usize| {
        if spans_sources {
            &part_files[..]
        } else {
            &part_files[index..=index]
        }
    };
    let shared: Vec<FolderModels> = combined
        .iter()
        .enumerate()
        .map(|(index, (kind, shared))| {
            FolderModels::of_part_source(
                &shared.path,
                &shared.files,
                part_sources(index + 1),
                false,
                Some(*kind),
                engine,
            )
        })
        .collect();
    let mut own = FolderModels::of_part_source(
        folder,
        files,
        part_sources(0),
        ingame_face,
        own_kind,
        engine,
    );
    // A combined face folder holding textures alone is a texture source, not a face: his
    // blank face takes the bundled diff, and his own face files are not used.
    if combined
        .iter()
        .zip(&shared)
        .any(|((kind, _), models)| matches!(kind, SharedKind::Face) && models.face)
    {
        own = own.with_linked_face();
    }
    (own, shared)
}

/// What `file` of the model folder at `folder`, a player folder's or a shared one's, whose
/// models are `models`, becomes in the
/// output of a target of the engine `models` were computed for; `None` when it has no role:
/// no task reads it, and validation reports it as `file_not_used` unless something else
/// explains it (`validation::file_role_messages`). A file below a subfolder of a model folder
/// is a part of the folder like a file directly in it, its role given by its name as in the
/// root (`model_role`), a `.common` link of a player folder's or, on Fox, a shared folder's
/// included, which stands for the `Common/` file at its own path below the folder; only a
/// per-player singleton is read
/// directly in the folder alone (`role_position`). Its
/// textures are the folder's own, each at its path. Each engine builds its own model format
/// (`fox_file`, `pre_fox_file`); the textures and the face diff take the same roles on both
/// (`texture_or_face_diff`).
pub(crate) fn player_file(
    folder: &ScopePath,
    file: &FileDescriptor,
    models: &FolderModels,
) -> Option<PlayerFile> {
    if !role_position(folder, file) {
        return None;
    }
    match models.engine {
        Engine::Fox => fox_file(folder, file, models),
        Engine::PreFox => pre_fox_file(folder, file, models),
    }
}

/// The folded path stems of the models of `format` among `files` of the folder at `folder`
/// where they may take a role (`role_position`).
fn model_stems(folder: &ScopePath, files: &[FileDescriptor], format: ModelFormat) -> Vec<String> {
    files
        .iter()
        .filter(|file| file.kind == FileKind::Model(format) && role_position(folder, file))
        .map(|file| vtree::fold_name(path_stem(file)))
        .collect()
}

/// Whether the per-kit model sets of a folder (`kit_variants::model_variant_sets`) are left out
/// but for their fallback — the `kitF` file, else the lowest variant — on a target of `engine`
/// (`kit_variant_model_left_out`): where
/// no `face.xml` names the set. That is every folder on Fox, which has no model-path
/// indirection; on pre-Fox a player folder holding `ingame_face` (his boots and gloves hold
/// parts, every one worn at once) and a shared boots or gloves folder of kind `shared` (one
/// `boots.model` merging every model, one `glove.xml` listing every glove), never a face, a
/// player's or a shared one, which lists the set once through its `face.xml` entry for the game
/// to respell. The fallback is what both engines agree on.
pub(crate) fn leaves_out_kit_variants(
    engine: Engine,
    ingame_face: bool,
    shared: Option<SharedKind>,
) -> bool {
    match engine {
        Engine::Fox => true,
        Engine::PreFox => {
            ingame_face || matches!(shared, Some(SharedKind::Boots | SharedKind::Gloves))
        }
    }
}

/// The model format a target of `engine` reads natively: FMDL on Fox, `.model` on pre-Fox.
pub(crate) fn native_format(engine: Engine) -> ModelFormat {
    match engine {
        Engine::Fox => ModelFormat::Fmdl,
        Engine::PreFox => ModelFormat::PesModel,
    }
}

/// The model format of the engine a target of `engine` does not read natively, which a model
/// of the target's format or a glTF of its stem beats: `.model` on Fox, FMDL on pre-Fox.
pub(crate) fn other_format(engine: Engine) -> ModelFormat {
    match engine {
        Engine::Fox => ModelFormat::PesModel,
        Engine::PreFox => ModelFormat::Fmdl,
    }
}

/// `player_file` on Fox, for `file`: a `.model` with no `.fmdl` or glTF of its path stem beside
/// it takes the role an FMDL of its name would, the Models task converting it with the `.mtl`
/// its search finds, every `.mtl` is a material set such a conversion may read
/// (`PlayerFile::Material`), and so is the Common `.mtl` a `.mtl.common` link stands for
/// (`PlayerFile::CommonMaterial`), and a glTF with no `.fmdl` of its path stem beside it is
/// `PlayerFile::UnsupportedGltf`. A model `FolderModels::beaten` names has no role. A shared
/// folder's `.common` links take the roles a player folder's take (`FolderModels::shared`).
fn fox_file(
    folder: &ScopePath,
    file: &FileDescriptor,
    models: &FolderModels,
) -> Option<PlayerFile> {
    let name = file.path.name();
    let stem = file_stem(name);
    let path_fold = vtree::fold_name(path_stem(file));
    match file.kind {
        // A left-out kit variant is one only where it would be a model: a file planning gives no
        // role keeps none.
        FileKind::Model(ModelFormat::Fmdl | ModelFormat::PesModel) => {
            if models.beaten(file) {
                return None;
            }
            let (package, name) = model_role(stem, models.ingame_face);
            Some(if models.left_out_variants.contains(&file.path) {
                PlayerFile::LeftOutKitVariant
            } else {
                PlayerFile::Model { package, name }
            })
        }
        // A model link, to an FMDL or a `.model`, takes the role the linked model would have in
        // its place; a `.mtl` link is the material set a `.model` of the folder's search may
        // find (`mtl_search::mtl_for`), as on pre-Fox; a texture link stands for its stem. A
        // shared folder's links are a player folder's; a link to anything else has no role.
        FileKind::CommonLink => match linked_model(name) {
            Some(linked) => {
                let (package, name) = model_role(file_stem(&linked), models.ingame_face);
                Some(PlayerFile::CommonModel { package, name })
            }
            None if common_link_name(name)
                .is_some_and(|linked| classify(&linked) == FileKind::Mtl) =>
            {
                Some(PlayerFile::CommonMaterial)
            }
            _ => linked_texture_below(folder, file).map(PlayerFile::CommonTexture),
        },
        // The game loads a skeleton under its slot's name; the export names it after the
        // model it pairs with, and without that model the skeleton has nothing to drive.
        // The stems fold, as `hair_stems`/`boots_stems`/`slotless_stems` build them.
        FileKind::Skl if models.hair_stems.iter().any(|hair| hair == &path_fold) => {
            Some(PlayerFile::Skeleton {
                package: ModelPackage::Face,
                name: "fcl_hair_sim.skl",
            })
        }
        FileKind::Skl if models.boots_stems.iter().any(|boots| boots == &path_fold) => {
            Some(PlayerFile::Skeleton {
                package: ModelPackage::Boots,
                name: "boots.skl",
            })
        }
        FileKind::Skl
            if models
                .slotless_stems
                .iter()
                .any(|model| model == &path_fold) =>
        {
            Some(PlayerFile::SlotlessSkeleton)
        }
        FileKind::Fclo if name.eq_ignore_ascii_case("fcl_hair_sim.fclo") => face_file(
            models,
            PlayerFile::Packed {
                package: ModelPackage::Face,
                name: "fcl_hair_sim.fclo",
            },
        ),
        FileKind::Mtl => Some(PlayerFile::Material),
        FileKind::Texture | FileKind::Bin | FileKind::Xml => {
            texture_or_face_diff(folder, file, models)
        }
        FileKind::Model(ModelFormat::Gltf) if !models.beaten(file) => {
            Some(PlayerFile::UnsupportedGltf)
        }
        FileKind::Model(ModelFormat::Gltf)
        | FileKind::Skl
        | FileKind::Fclo
        | FileKind::MaterialsToml
        | FileKind::SharedLink(_)
        | FileKind::Marker(_)
        | FileKind::Metadata(_)
        | FileKind::Other => None,
    }
}

/// `player_file` on pre-Fox, for `file`: a `.model` typed for the face's `face.xml` by its name
/// (`face_xml::xml_type`), or under `ingame_face` a part of the package Fox gives it
/// (`pre_fox_part`), an `.fmdl` with no `.model` or glTF of its path stem beside it the same,
/// the `.skl` paired with such an FMDL its conversion's bind pose
/// (`PlayerFile::ConversionSkeleton`), a `.mtl`, a `.common` link to a `.model`, an FMDL, a
/// `.mtl` or a texture (`pre_fox_link`), under `ingame_face` a link to a model being a part as
/// the `.model` itself would be, a member's own
/// `face.xml` or a shared face folder's, a face file (`PlayerFile::FaceXml`), and a glTF with
/// no `.model` of its path stem beside it (`PlayerFile::UnsupportedGltf`). A model
/// `FolderModels::left_out_variants` names is `PlayerFile::LeftOutKitVariant`. A model
/// `FolderModels::beaten` names, any other `.skl` and `fcl_hair_sim.fclo` have no role, nor any
/// other link, any link in a shared folder (`FolderModels::shared`), a shared boots or gloves
/// folder's own `face.xml` or any `.xml` but the face diff.
fn pre_fox_file(
    folder: &ScopePath,
    file: &FileDescriptor,
    models: &FolderModels,
) -> Option<PlayerFile> {
    let path_fold = vtree::fold_name(path_stem(file));
    match file.kind {
        FileKind::Model(ModelFormat::PesModel | ModelFormat::Fmdl) => {
            if models.beaten(file) {
                return None;
            }
            let stem = file_stem(file.path.name());
            // As on Fox, a left-out kit variant is one only where it would be a model.
            Some(if models.left_out_variants.contains(&file.path) {
                PlayerFile::LeftOutKitVariant
            } else if models.ingame_face {
                pre_fox_part(stem)
            } else {
                PlayerFile::PreFoxModel {
                    xml_type: face_xml::xml_type(stem),
                }
            })
        }
        FileKind::Mtl => Some(PlayerFile::Material),
        // As on Fox: a link a lenient file-type check keeps in a shared folder names nothing.
        FileKind::CommonLink if models.is_shared() => None,
        FileKind::CommonLink => pre_fox_link(folder, file, models.ingame_face),
        // A shared boots or gloves folder's own xml is ignored (`xml_ignored_shared`): its
        // output is one model or a `glove.xml`, which no face xml drives, and nothing checks it.
        FileKind::Xml
            if file.path.name().eq_ignore_ascii_case("face.xml")
                && models.is_shared_boots_or_gloves() =>
        {
            None
        }
        FileKind::Xml if file.path.name().eq_ignore_ascii_case("face.xml") => {
            face_file(models, PlayerFile::FaceXml)
        }
        FileKind::Texture | FileKind::Bin | FileKind::Xml => {
            texture_or_face_diff(folder, file, models)
        }
        FileKind::Skl if models.converted_stems.contains(&path_fold) => {
            Some(PlayerFile::ConversionSkeleton)
        }
        FileKind::Model(ModelFormat::Gltf) if !models.beaten(file) => {
            Some(PlayerFile::UnsupportedGltf)
        }
        FileKind::Model(ModelFormat::Gltf)
        | FileKind::Skl
        | FileKind::Fclo
        | FileKind::MaterialsToml
        | FileKind::SharedLink(_)
        | FileKind::Marker(_)
        | FileKind::Metadata(_)
        | FileKind::Other => None,
    }
}

/// The pre-Fox role of a model with file stem `stem` in a player folder holding
/// `ingame_face`, a `.model` of his or the Common one a `.common` link of his names: a part of
/// the package Fox gives it under the marker (`PlayerFile::PreFoxPart`), since the marker
/// means no face and so no `face.xml` to list it, typed as it would be there
/// (`face_xml::xml_type`) for a `glove.xml` to list. The two engines agree on which models
/// have a role. A model named as face content never reaches here: validation drops a marked
/// folder holding one, file or link (`ingame_face_explicit_face_model`). A per-kit model is a
/// part too: its set's fallback is used and the others are left out
/// (`kit_variant_model_left_out`, `FolderModels::left_out_variants`), as on Fox, since as parts
/// of his boots or gloves every variant would be worn at once.
fn pre_fox_part(stem: &str) -> PlayerFile {
    PlayerFile::PreFoxPart {
        package: model_role(stem, true).0,
        xml_type: face_xml::xml_type(stem),
    }
}

/// The pre-Fox role of `file`, a `.common` link of the model folder at `folder`: a link to a
/// `.model` or an FMDL takes the type a `.model` of the linked stem would have
/// (`face_xml::xml_type`), the Common models
/// task having packed the `.model`, or converted the FMDL, into the team's Common output; under
/// `ingame_face` (`ingame_face` set) such a link stands for the Common model as a part of his
/// own instead, copied in or converted (`pre_fox_part`), since no `face.xml` names the Common
/// path. A link to a `.mtl` is a material link, a link to a texture stands for its path below
/// `Common/`, as on Fox. A link to a per-kit model has none yet, with the marker or without:
/// the Common models task, which packs the linked model, would have to list its set, and as a
/// part every variant would be worn at once. A link to anything else has none.
fn pre_fox_link(
    folder: &ScopePath,
    file: &FileDescriptor,
    ingame_face: bool,
) -> Option<PlayerFile> {
    let linked = common_link_name(file.path.name())?;
    match classify(&linked) {
        FileKind::Model(ModelFormat::PesModel | ModelFormat::Fmdl) => {
            let stem = file_stem(&linked);
            if kit_token(stem).is_some() {
                return None;
            }
            if ingame_face {
                return Some(pre_fox_part(stem));
            }
            Some(PlayerFile::PreFoxCommonModel {
                xml_type: face_xml::xml_type(stem),
            })
        }
        FileKind::Mtl => Some(PlayerFile::CommonMaterial),
        FileKind::Texture => linked_texture_below(folder, file).map(PlayerFile::CommonTexture),
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

/// The role of `file` of the folder at `folder` that both engines give alike: a texture by its
/// path below the folder (`PlayerFile::Texture`), and the face diff, `face_diff.bin` or
/// `face_diff.xml`, a face file (`face_file`); `None` for any other file.
fn texture_or_face_diff(
    folder: &ScopePath,
    file: &FileDescriptor,
    models: &FolderModels,
) -> Option<PlayerFile> {
    let name = file.path.name();
    match file.kind {
        FileKind::Texture => texture_format(name).map(|format| PlayerFile::Texture {
            below: file_stem(&relative(&file.path, folder)).to_owned(),
            format,
        }),
        FileKind::Bin if name.eq_ignore_ascii_case("face_diff.bin") => face_file(
            models,
            PlayerFile::Packed {
                package: ModelPackage::Face,
                name: "face_diff.bin",
            },
        ),
        FileKind::Xml if name.eq_ignore_ascii_case("face_diff.xml") => {
            face_file(models, PlayerFile::FaceDiffXml)
        }
        FileKind::Model(_)
        | FileKind::Skl
        | FileKind::Bin
        | FileKind::Fclo
        | FileKind::Xml
        | FileKind::Mtl
        | FileKind::MaterialsToml
        | FileKind::SharedLink(_)
        | FileKind::CommonLink
        | FileKind::Marker(_)
        | FileKind::Metadata(_)
        | FileKind::Other => None,
    }
}

/// `role` for a face file in a folder whose models are `models`: without a face to shape it, it
/// is not used (`PlayerFile::UnusedFaceFile`).
fn face_file(models: &FolderModels, role: PlayerFile) -> Option<PlayerFile> {
    Some(if models.face {
        role
    } else {
        PlayerFile::UnusedFaceFile
    })
}

/// Whether `role` is a part of `package`: a model of the folder's own, or a Common model its
/// link brings in, on Fox; a model of an `ingame_face` player's own on pre-Fox.
pub(crate) fn is_part_of(role: &PlayerFile, package: ModelPackage) -> bool {
    matches!(
        role,
        PlayerFile::Model { package: owner, .. }
            | PlayerFile::CommonModel { package: owner, .. }
            | PlayerFile::PreFoxPart { package: owner, .. }
            if *owner == package
    )
}

/// Whether `role`, the role of a file of a source feeding the package `source` (the folder's
/// own, or a combined shared folder's), is a model of `package`'s task: a part of it
/// (`is_part_of`), or a pre-Fox model, which its source's package packs whatever its name.
pub(crate) fn is_package_model(
    role: &PlayerFile,
    source: ModelPackage,
    package: ModelPackage,
) -> bool {
    is_part_of(role, package)
        || (matches!(role, PlayerFile::PreFoxModel { .. }) && package == source)
}

/// Whether planning gives the mapped player folder `player` of `export` a package task on a
/// target of `engine` (`folder_tasks`), `hand_weighted` as for `link_combines`. A folder
/// without `ingame_face` always gets its face, blank when it holds no part, a referee's too:
/// referees are full FPC, so a referee without a face model has a full-body model, split as
/// boots, that brings his head. A marked player gets one when a source of his, his own files
/// or a shared folder a link of his builds into his own package (`link_feeds_own_package`),
/// holds a model of a package (`is_package_model`), or a glTF selected for its stem, for which
/// planning drops the folder with `model_gltf_unsupported`. A folder planning no package plans
/// no textures task, and validation reports each of its textures and texture links
/// `file_not_used`.
pub(crate) fn plans_a_package(
    export: &ValidatedAestheticsExport,
    player: &PlayerFolder,
    engine: Engine,
    hand_weighted: &BTreeSet<ScopePath>,
) -> bool {
    if player.ingame_face.is_none() {
        return true;
    }
    let combined = combined_folders(export, player, engine, hand_weighted);
    let (own, combined_models) = part_source_models(
        &player.path,
        &player.files,
        None,
        player.ingame_face.is_some(),
        &combined,
        engine,
    );
    // Reached only under `ingame_face`, the early return above having answered every other
    // folder: a marked player has no face, so the package his own files feed is his boots, as
    // `ModelFolder::own_package` gives it.
    let own_package = ModelPackage::Boots;
    let combined_sources = combined
        .iter()
        .zip(combined_models)
        .map(|((kind, shared), models)| {
            (
                package_of(*kind),
                &shared.path,
                shared.files.as_slice(),
                models,
            )
        });
    std::iter::once((own_package, &player.path, player.files.as_slice(), own))
        .chain(combined_sources)
        .any(|(source, path, files, models)| {
            files
                .iter()
                .any(|file| match player_file(path, file, &models) {
                    Some(PlayerFile::UnsupportedGltf) => true,
                    Some(role) => ModelPackage::ALL
                        .into_iter()
                        .any(|package| is_package_model(&role, source, package)),
                    None => false,
                })
        })
}

/// Whether the model `file` is named as face content: the package its name gives it
/// (`model_role`, without the `ingame_face` marker) is the face. A pre-Fox face packs boots and
/// gloves models beside its face models, all typed by `face.xml`, so this is what keeps a model
/// named as boots or gloves out of the pre-Fox hand split, as its package keeps it out on Fox
/// (`spec/model_conversion/hand_split.md` "Pipeline integration").
pub(crate) fn named_as_face(file: &FileDescriptor) -> bool {
    model_role(file_stem(file.path.name()), false).0 == ModelPackage::Face
}

/// Whether the model `file` of a source folder, whose role is `role`, is hand auto-split on a
/// target of `engine` (`spec/model_conversion/hand_split.md` "Pipeline integration"): it is among
/// `hand_weighted`, the models the deep pass found carrying hand weights, and is face content.
/// On Fox that is a part of the face; on pre-Fox a `.model` the face packs
/// (`PlayerFile::PreFoxModel`) and names as face content (`named_as_face`). A model named as
/// boots or gloves is never one on either engine, whatever its weights: an authored glove is
/// all hand, and a boots model is on the body skeleton already.
pub(crate) fn is_hand_split(
    engine: Engine,
    file: &FileDescriptor,
    role: &PlayerFile,
    hand_weighted: &BTreeSet<ScopePath>,
) -> bool {
    hand_weighted.contains(&file.path)
        && match engine {
            Engine::Fox => is_part_of(role, ModelPackage::Face),
            Engine::PreFox => matches!(role, PlayerFile::PreFoxModel { .. }) && named_as_face(file),
        }
}

/// Whether the effective package `package` of `player`, a player folder of `export`, has a
/// part on a target of `engine`. A part is what gives him his own package of that kind, which
/// a link of its kind then combines with (`link_combines`). The parts are:
///
/// - those of his own files (`is_part_of`), under his `ingame_face` marker when he holds one,
///   a `.common` link's Common model included;
/// - those of each shared face he links, its files under his roles. On Fox that makes a boots-
///   or glove-named model there a part of his boots or gloves, as his own would be. On pre-Fox
///   it gives neither any part, since a face's models are face content there;
/// - on Fox, for his gloves, the hands split off a face part of his or of a linked face
///   (`is_hand_split`), which join his gloves as authored ones would. On pre-Fox a split stays
///   inside the face, which gives no gloves package. A Fox player's own `face.xml` does not
///   stop a split, since Fox has no `face.xml` role.
///
/// His and his faces' roles are read over both, as his face is built (`part_source_models`):
/// a per-kit variant a lower one of its set leaves out is no part.
pub(crate) fn has_effective_part(
    export: &ValidatedAestheticsExport,
    player: &PlayerFolder,
    package: ModelPackage,
    engine: Engine,
    hand_weighted: &BTreeSet<ScopePath>,
) -> bool {
    let splits_into_package = match engine {
        Engine::Fox => package == ModelPackage::Gloves,
        Engine::PreFox => false,
    };
    // His faces always combine; his boots and gloves links combine on what this decides, so
    // they are not among the sources read here.
    let faces: Vec<(SharedKind, &SharedModelFolder)> = player
        .links
        .iter()
        .filter(|link| link.kind == SharedKind::Face)
        .filter_map(|link| Some((link.kind, linked_folder(export, link)?)))
        .collect();
    let (own, face_models) = part_source_models(
        &player.path,
        &player.files,
        None,
        player.ingame_face.is_some(),
        &faces,
        engine,
    );
    let face_sources = faces
        .iter()
        .zip(face_models)
        .map(|((_, face), models)| (&face.path, face.files.as_slice(), models));
    std::iter::once((&player.path, player.files.as_slice(), own))
        .chain(face_sources)
        .any(|(source, files, models)| {
            files.iter().any(|file| {
                let Some(role) = player_file(source, file, &models) else {
                    return false;
                };
                if is_part_of(&role, package) {
                    return true;
                }
                if !splits_into_package {
                    return false;
                }
                // A link's Common model is split as the file the link loads, which is what
                // the deep pass read.
                let model = if file.kind == FileKind::CommonLink {
                    common_link_target(&file.path, source)
                        .and_then(|linked| selected_common_model(&export.common, &linked, engine))
                } else {
                    Some(file)
                };
                model.is_some_and(|model| is_hand_split(engine, model, &role, hand_weighted))
            })
        })
}

/// A file name's stem: the name up to its last `.`.
pub(crate) fn file_stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

/// What `compile` does with a team export's `Collars/` file for a target of one engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CollarFile {
    /// A model the target's collar task writes: in the format the engine reads (an FMDL on Fox,
    /// a `.model` on pre-Fox), written with its author's material names, or in the other
    /// engine's format, converted (an FMDL on pre-Fox to a `.model` with the stock collars'
    /// material names, a `.model` on Fox to an FMDL with the templates' `uniform.mtl` as its
    /// `.mtl`). It claims the stock collar its name gives and is written in its place.
    Compiled,
    /// A glTF, which the compiler does not read yet: planning drops it with
    /// `model_gltf_unsupported`, the file alone, and the export compiles without it.
    Unsupported,
    /// Any other kind, which the structure pass keeps only with the strict file-type check
    /// off (`file_type_disallowed`): passed over, with no task and no finding.
    PassedOver,
}

/// What `compile` does with the `Collars/` file `file` for a target of `engine`.
pub(crate) fn collar_file(file: &FileDescriptor, engine: Engine) -> CollarFile {
    match file.kind {
        FileKind::Model(format) => match (engine, format) {
            (Engine::Fox | Engine::PreFox, ModelFormat::Fmdl | ModelFormat::PesModel) => {
                CollarFile::Compiled
            }
            (Engine::Fox | Engine::PreFox, ModelFormat::Gltf) => CollarFile::Unsupported,
        },
        FileKind::Texture
        | FileKind::Skl
        | FileKind::Fclo
        | FileKind::Xml
        | FileKind::Mtl
        | FileKind::MaterialsToml
        | FileKind::Bin
        | FileKind::SharedLink(_)
        | FileKind::CommonLink
        | FileKind::Marker(_)
        | FileKind::Metadata(_)
        | FileKind::Other => CollarFile::PassedOver,
    }
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;
    use studio_core::ExportId;

    use super::*;
    use crate::plan::{TaskKind, plan_run};
    use crate::testing::{resolved, resolved_with_issues, to_plan, two_team_colors};

    /// A Fox face folder: a face model and `face_diff.bin`.
    const FACE: [&str; 2] = [
        "Players/03 - A/face_high.fmdl",
        "Players/03 - A/face_diff.bin",
    ];

    /// A pre-Fox face folder: a model, its material set and its texture.
    const PRE_FOX_FACE: [&str; 3] = [
        "Players/03 - A/face_high.model",
        "Players/03 - A/face_high.mtl",
        "Players/03 - A/skin.dds",
    ];

    #[test]
    fn a_model_link_loads_the_common_model_its_stem_selects_target_native_first() {
        let files = |paths: &[&str]| -> Vec<FileDescriptor> {
            paths
                .iter()
                .map(|path| {
                    let path = ScopePath::new(path).unwrap();
                    FileDescriptor {
                        size: 0,
                        kind: aesthetics_export::classify(path.name()),
                        source: path.clone(),
                        path,
                    }
                })
                .collect()
        };
        let loaded_on = |common: &[FileDescriptor], linked: &str, engine: Engine| {
            selected_common_model(common, linked, engine).map(|file| file.path.as_str().to_owned())
        };
        let loaded =
            |common: &[FileDescriptor], linked: &str| loaded_on(common, linked, Engine::PreFox);
        let alone = files(&["Common/legs.fmdl", "Common/Legs.skl"]);
        assert_eq!(
            loaded(&alone, "Legs.fmdl").as_deref(),
            Some("Common/legs.fmdl")
        );
        // A `.model` of its stem beats the FMDL, which a link to the FMDL then does not load.
        let both = files(&["Common/legs.fmdl", "Common/LEGS.model"]);
        assert_eq!(
            loaded(&both, "legs.fmdl").as_deref(),
            Some("Common/LEGS.model")
        );
        assert_eq!(
            loaded(&both, "legs.model").as_deref(),
            Some("Common/LEGS.model")
        );
        assert_eq!(loaded(&both, "hat.fmdl"), None);
        // On Fox the FMDL beats the `.model`, which a link to the `.model` then does not load;
        // a `.model` alone is loaded.
        for linked in ["legs.model", "legs.fmdl"] {
            assert_eq!(
                loaded_on(&both, linked, Engine::Fox).as_deref(),
                Some("Common/legs.fmdl"),
                "{linked}"
            );
        }
        let model = files(&["Common/legs.model", "Common/legs.mtl"]);
        assert_eq!(
            loaded_on(&model, "Legs.model", Engine::Fox).as_deref(),
            Some("Common/legs.model")
        );
        // The model a target selects for its stem is the only one of the two read.
        let selected = |engine| {
            both.iter()
                .filter(|file| is_selected_common_model(&both, file, engine))
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(selected(Engine::PreFox), ["Common/LEGS.model"]);
        assert_eq!(selected(Engine::Fox), ["Common/legs.fmdl"]);
        assert!(is_selected_common_model(&model, &model[0], Engine::Fox));
    }

    #[test]
    fn a_target_emits_the_seven_kit_textures_but_the_other_engine_s_map() {
        let emitted = |engine| {
            [
                "kit",
                "kit_back",
                "kit_chest",
                "kit_leg",
                "kit_name",
                "kit_mask",
                "kit_srm",
                "kit_spec",
            ]
            .into_iter()
            .filter(|stem| emits_kit_texture(engine, stem))
            .collect::<Vec<_>>()
        };
        assert_eq!(
            emitted(Engine::PreFox),
            [
                "kit",
                "kit_back",
                "kit_chest",
                "kit_leg",
                "kit_name",
                "kit_mask"
            ]
        );
        assert_eq!(
            emitted(Engine::Fox),
            [
                "kit",
                "kit_back",
                "kit_chest",
                "kit_leg",
                "kit_name",
                "kit_srm"
            ]
        );
    }

    #[test]
    fn a_collar_compiles_in_the_model_format_its_target_s_engine_reads() {
        let collar = |name: &str, engine| {
            let path = ScopePath::new(&format!("Collars/{name}")).unwrap();
            let file = FileDescriptor {
                size: 1,
                kind: classify(path.name()),
                source: path.clone(),
                path,
            };
            collar_file(&file, engine)
        };
        for (name, fox, pre_fox) in [
            ("collar_12.fmdl", CollarFile::Compiled, CollarFile::Compiled),
            (
                "collar_12.model",
                CollarFile::Compiled,
                CollarFile::Compiled,
            ),
            (
                "collar_12.glb",
                CollarFile::Unsupported,
                CollarFile::Unsupported,
            ),
            (
                "collar_12.gltf",
                CollarFile::Unsupported,
                CollarFile::Unsupported,
            ),
            (
                "collar_12.dds",
                CollarFile::PassedOver,
                CollarFile::PassedOver,
            ),
        ] {
            assert_eq!(collar(name, Engine::Fox), fox, "{name} on Fox");
            assert_eq!(collar(name, Engine::PreFox), pre_fox, "{name} on pre-Fox");
        }
    }

    #[test]
    fn the_other_engine_s_map_is_dropped_before_the_kit_task() {
        // (version, a face folder it compiles, the map it reads, the files it drops in the
        // set's order: the other engine's map and `kit_spec`, a stem the compiler never builds)
        let cases = [
            (
                PesVersion::Pes21,
                FACE.as_slice(),
                "kit_srm",
                ["kit_mask.dds", "kit_spec.dds"],
            ),
            (
                PesVersion::Pes17,
                PRE_FOX_FACE.as_slice(),
                "kit_mask",
                ["kit_spec.dds", "kit_srm.dds"],
            ),
        ];
        for (version, face, kept, dropped) in cases {
            let files: Vec<(&str, u64)> = face
                .iter()
                .chain(&[
                    "Kits/g1/config.toml",
                    "Kits/g1/kit.dds",
                    "Kits/g1/kit_mask.dds",
                    "Kits/g1/kit_spec.dds",
                    "Kits/g1/kit_srm.dds",
                ])
                .map(|path| (*path, 1))
                .collect();
            let export = resolved("co Midcup Maps", &files, &[], None);

            let report = plan_run(
                vec![to_plan(ExportId(0), export, two_team_colors(), None)],
                version,
                fpc::HideIds::CONVENTION,
            );

            // Each drop is reported once, on the kit, naming the file.
            assert_eq!(report.messages.len(), dropped.len(), "{version}");
            for (message, dropped) in report.messages.iter().zip(dropped) {
                assert_eq!(message.code.code, "kit_texture_not_used", "{version}");
                assert_eq!(
                    message.scope,
                    studio_core::Scope::Folder {
                        export_id: ExportId(0),
                        path: ScopePath::new("Kits/g1").unwrap(),
                    },
                    "{version}"
                );
                assert_eq!(
                    message.disposition,
                    studio_core::Disposition::DropFile,
                    "{version}"
                );
                assert_eq!(
                    message.context,
                    [("file".to_owned(), dropped.to_owned())],
                    "{version}"
                );
            }
            let (kit, charge) = report
                .manifest
                .tasks
                .iter()
                .find_map(|task| {
                    if let TaskKind::Kit { kit, .. } = &task.kind {
                        Some((kit, task.charge))
                    } else {
                        None
                    }
                })
                .expect("the kit's task");
            let stems: Vec<&str> = kit
                .textures
                .iter()
                .map(|texture| texture.stem.as_str())
                .collect();
            assert_eq!(stems, ["kit", kept], "{version}");
            assert_eq!(
                charge, 3,
                "{version}: the config, the kit and the map read are read, the dropped files not"
            );
        }
    }

    /// The folder `Players/03 - A` holding the files `names`.
    fn folder(names: &[&str]) -> PlayerFolder {
        let files = names
            .iter()
            .map(|name| {
                let path = ScopePath::new(&format!("Players/03 - A/{name}")).unwrap();
                FileDescriptor {
                    size: 0,
                    kind: aesthetics_export::classify(path.name()),
                    source: path.clone(),
                    path,
                }
            })
            .collect();
        PlayerFolder {
            path: ScopePath::new("Players/03 - A").unwrap(),
            player_name: "A".to_owned(),
            files,
            links: Vec::new(),
            ingame_face: None,
            fpc: None,
            portrait: None,
            settings: None,
        }
    }

    /// The folder's `ingame_face.toml`, the file `PlayerFolder::ingame_face` holds.
    fn ingame_face_toml() -> FileDescriptor {
        let path = ScopePath::new("Players/03 - A/ingame_face.toml").unwrap();
        FileDescriptor {
            size: 0,
            kind: aesthetics_export::classify(path.name()),
            source: path.clone(),
            path,
        }
    }

    /// The role of each file of `names` in `Players/03 - A`.
    fn roles(names: &[&str]) -> Vec<Option<PlayerFile>> {
        let folder = folder(names);
        let models = FolderModels::of_player_files(&folder.path, &folder.files, false, Engine::Fox);
        folder
            .files
            .iter()
            .map(|file| player_file(&folder.path, file, &models))
            .collect()
    }

    /// The role of each file of `names` in `Players/03 - A` holding `ingame_face` too.
    fn marked_roles(names: &[&str]) -> Vec<Option<PlayerFile>> {
        let folder = PlayerFolder {
            ingame_face: Some(ingame_face_toml()),
            ..folder(names)
        };
        let models = FolderModels::of_player_files(
            &folder.path,
            &folder.files,
            folder.ingame_face.is_some(),
            Engine::Fox,
        );
        folder
            .files
            .iter()
            .map(|file| player_file(&folder.path, file, &models))
            .collect()
    }

    /// The role of the file `name` in a folder holding it beside `fcl_hair.fmdl` and
    /// `kit_boots.fmdl`.
    fn role(name: &str) -> Option<PlayerFile> {
        roles(&["fcl_hair.fmdl", "kit_boots.fmdl", name])
            .pop()
            .unwrap()
    }

    fn model(package: ModelPackage, name: &'static str) -> Option<PlayerFile> {
        Some(PlayerFile::Model { package, name })
    }

    fn packed(package: ModelPackage, name: &'static str) -> Option<PlayerFile> {
        Some(PlayerFile::Packed { package, name })
    }

    fn skeleton(package: ModelPackage, name: &'static str) -> Option<PlayerFile> {
        Some(PlayerFile::Skeleton { package, name })
    }

    #[test]
    fn each_model_goes_to_its_package_under_its_allowed_name() {
        for (name, package, allowed) in [
            ("face_high.fmdl", ModelPackage::Face, "face_high"),
            ("hair_high.fmdl", ModelPackage::Face, "hair_high"),
            ("oral.fmdl", ModelPackage::Face, "oral"),
            ("fcl_hair.fmdl", ModelPackage::Face, "fcl_hair"),
            ("x_fcl_hair.fmdl", ModelPackage::Face, "fcl_hair"),
            ("boots.fmdl", ModelPackage::Boots, "boots"),
            ("kit_boots.fmdl", ModelPackage::Boots, "boots"),
            ("Kit_BOOTS.fmdl", ModelPackage::Boots, "boots"),
            ("glove_l.fmdl", ModelPackage::Gloves, "glove_l"),
            ("gloveL.fmdl", ModelPackage::Gloves, "glove_l"),
            ("keeper_gloveR.fmdl", ModelPackage::Gloves, "glove_r"),
            ("glove_r.fmdl", ModelPackage::Gloves, "glove_r"),
            ("handL.fmdl", ModelPackage::Gloves, "glove_l"),
            ("handR.fmdl", ModelPackage::Gloves, "glove_r"),
            // No recognized suffix: face content the hair merge takes.
            ("torso.fmdl", ModelPackage::Face, "fcl_hair"),
            ("myboots.fmdl", ModelPackage::Face, "fcl_hair"),
        ] {
            assert_eq!(roles(&[name]), [model(package, allowed)], "{name}");
        }
    }

    /// The role of each file of `names` in `Players/03 - A` on a target of `engine`.
    fn engine_roles(names: &[&str], engine: Engine) -> Vec<Option<PlayerFile>> {
        let folder = folder(names);
        let models = FolderModels::of_player_files(&folder.path, &folder.files, false, engine);
        folder
            .files
            .iter()
            .map(|file| player_file(&folder.path, file, &models))
            .collect()
    }

    fn pre_fox_model(xml_type: &str) -> Option<PlayerFile> {
        Some(PlayerFile::PreFoxModel {
            xml_type: xml_type.to_owned(),
        })
    }

    /// `pre_fox_link` on the file `name` of `Players/03 - A`.
    fn link_role(name: &str, ingame_face: bool) -> Option<PlayerFile> {
        let folder = folder(&[name]);
        pre_fox_link(&folder.path, &folder.files[0], ingame_face)
    }

    #[test]
    fn on_pre_fox_a_model_a_material_set_and_an_fmdl_to_convert_have_roles_and_the_fclo_none() {
        assert_eq!(
            engine_roles(
                &[
                    "face_high.model",
                    "face_high.mtl",
                    "skin.dds",
                    "face_diff.bin",
                    "boots/x.model",
                    "gloves/left.model",
                    "hat.fmdl",
                    "fcl_hair_sim.fclo",
                    "pants_kit1.model",
                    "face.xml",
                ],
                Engine::PreFox
            ),
            [
                pre_fox_model("face_neck"),
                Some(PlayerFile::Material),
                Some(PlayerFile::Texture {
                    below: "skin".to_owned(),
                    format: SourceFormat::Dds,
                }),
                packed(ModelPackage::Face, "face_diff.bin"),
                pre_fox_model("parts"),
                // Named for no part: typed by its name, as in the root.
                pre_fox_model("parts"),
                pre_fox_model("parts"),
                None,
                pre_fox_model("parts"),
                Some(PlayerFile::FaceXml),
            ]
        );
        // On Fox the same files are a model the Models task converts and its material set.
        assert_eq!(
            engine_roles(&["face_high.model", "face_high.mtl"], Engine::Fox),
            [
                model(ModelPackage::Face, "face_high"),
                Some(PlayerFile::Material)
            ]
        );
        // A glove names its hand, and a model in any subfolder takes its own name's type.
        assert_eq!(
            engine_roles(
                &[
                    "gloves/keeper_glove_r.model",
                    "gloves/x_handL.model",
                    "face/visor_ratio_2_parts.model",
                    "common/hat.model",
                    "common/hat.mtl",
                ],
                Engine::PreFox
            ),
            [
                pre_fox_model("gloveR"),
                pre_fox_model("handL"),
                pre_fox_model("parts"),
                pre_fox_model("parts"),
                Some(PlayerFile::Material),
            ]
        );
    }

    #[test]
    fn on_pre_fox_a_member_s_own_face_xml_is_a_face_file_and_gives_the_folder_a_face() {
        // Directly in the folder, in any case; a subfolder's is none (the structure pass names
        // it, `file_type_disallowed`); never on Fox.
        let names = ["Face.XML", "face/face.xml", "boots/face.xml", "other.xml"];
        assert_eq!(
            engine_roles(&names, Engine::PreFox),
            [Some(PlayerFile::FaceXml), None, None, None]
        );
        assert_eq!(engine_roles(&names, Engine::Fox), [None, None, None, None]);
        assert_eq!(PlayerFile::FaceXml.package(), Some(ModelPackage::Face));
        // A folder holding no model but its xml has a face: its face diff is used.
        assert_eq!(
            engine_roles(&["face.xml", "face_diff.bin"], Engine::PreFox),
            [
                Some(PlayerFile::FaceXml),
                packed(ModelPackage::Face, "face_diff.bin")
            ]
        );
        assert_eq!(
            engine_roles(&["face_diff.bin"], Engine::PreFox),
            [Some(PlayerFile::UnusedFaceFile)]
        );
        // Under `ingame_face` there is no face: the xml is not used.
        let marked = PlayerFolder {
            ingame_face: Some(ingame_face_toml()),
            ..folder(&["kit_boots.model", "face.xml"])
        };
        let models = FolderModels::of_player_files(
            &marked.path,
            &marked.files,
            marked.ingame_face.is_some(),
            Engine::PreFox,
        );
        let roles: Vec<Option<PlayerFile>> = marked
            .files
            .iter()
            .map(|file| player_file(&marked.path, file, &models))
            .collect();
        assert_eq!(roles[1], Some(PlayerFile::UnusedFaceFile));
    }

    #[test]
    fn on_pre_fox_a_common_link_to_a_model_a_mtl_or_a_texture_has_a_role() {
        let common_model = |xml_type: &str| {
            Some(PlayerFile::PreFoxCommonModel {
                xml_type: xml_type.to_owned(),
            })
        };
        assert_eq!(
            engine_roles(
                &[
                    "legs.model.common",
                    "face_high.model.common",
                    "body.mtl.common",
                    "hair.png.common",
                    "x.fmdl.common",
                    "common/legs.model.common",
                    "common/body.mtl.common",
                    "boots/Legs.MODEL.common.txt",
                    "gloves/left.model.common",
                    "pants_kit1.model.common",
                    "pants_kit1.fmdl.common",
                    "x.glb.common",
                ],
                Engine::PreFox
            ),
            [
                common_model("parts"),
                common_model("face_neck"),
                Some(PlayerFile::CommonMaterial),
                Some(PlayerFile::CommonTexture("hair".to_owned())),
                common_model("parts"),
                // A link below a subfolder is one as in the root (`role_position`).
                common_model("parts"),
                Some(PlayerFile::CommonMaterial),
                common_model("parts"),
                common_model("parts"),
                // A per-kit model and a glTF have none.
                None,
                None,
                None,
            ]
        );
        // Under `ingame_face` a link to a `.model` or an FMDL is a part of his boots or gloves.
        let marked = PlayerFolder {
            ingame_face: Some(ingame_face_toml()),
            ..folder(&[
                "legs.model.common",
                "legs.fmdl.common",
                "glove_l.fmdl.common",
            ])
        };
        let models = FolderModels::of_player_files(
            &marked.path,
            &marked.files,
            marked.ingame_face.is_some(),
            Engine::PreFox,
        );
        let roles: Vec<Option<PlayerFile>> = marked
            .files
            .iter()
            .map(|file| player_file(&marked.path, file, &models))
            .collect();
        let part = |package, xml_type: &str| {
            Some(PlayerFile::PreFoxPart {
                package,
                xml_type: xml_type.to_owned(),
            })
        };
        assert_eq!(
            roles,
            [
                part(ModelPackage::Boots, "parts"),
                part(ModelPackage::Boots, "parts"),
                part(ModelPackage::Gloves, "gloveL"),
            ]
        );
        // A typed model link is a face model: the face diff beside it is used.
        assert_eq!(
            engine_roles(&["face_diff.bin", "legs.model.common"], Engine::PreFox),
            [
                packed(ModelPackage::Face, "face_diff.bin"),
                common_model("parts")
            ]
        );
        assert_eq!(
            engine_roles(&["face_diff.bin", "body.mtl.common"], Engine::PreFox),
            [
                Some(PlayerFile::UnusedFaceFile),
                Some(PlayerFile::CommonMaterial)
            ]
        );
        // The team's files: packed into no package of the player's.
        assert_eq!(common_model("parts").unwrap().package(), None);
        assert_eq!(PlayerFile::CommonMaterial.package(), None);
    }

    #[test]
    fn on_pre_fox_an_fmdl_is_converted_unless_a_model_of_its_stem_beats_it() {
        // No `.model` of its stem: the face converts it, the `.skl` of its stem its bind pose.
        assert_eq!(
            engine_roles(
                &[
                    "fcl_hair.fmdl",
                    "Fcl_Hair.skl",
                    "boots.skl",
                    "fcl_hair_sim.fclo"
                ],
                Engine::PreFox
            ),
            [
                pre_fox_model("parts"),
                Some(PlayerFile::ConversionSkeleton),
                None,
                None,
            ]
        );
        // A `.model` of its stem beats it, and its skeleton pairs nothing; a model of its
        // name in another directory does not.
        assert_eq!(
            engine_roles(
                &[
                    "boots.fmdl",
                    "boots.model",
                    "boots.skl",
                    "face/hat.model",
                    "hat.fmdl"
                ],
                Engine::PreFox
            ),
            [
                None,
                pre_fox_model("parts"),
                None,
                pre_fox_model("parts"),
                pre_fox_model("parts"),
            ]
        );
        // A converted FMDL is a face model: the face diff beside it is used.
        assert_eq!(
            engine_roles(&["face_diff.xml", "hat.fmdl"], Engine::PreFox),
            [Some(PlayerFile::FaceDiffXml), pre_fox_model("parts")]
        );
        assert_eq!(PlayerFile::ConversionSkeleton.package(), None);
    }

    #[test]
    fn on_fox_a_model_is_converted_unless_an_fmdl_of_its_stem_beats_it() {
        // No `.fmdl` of its stem: it takes the role an FMDL of its name would, and its `.mtl`
        // is a material set, wherever it sits.
        assert_eq!(
            engine_roles(
                &[
                    "boots.model",
                    "boots.mtl",
                    "x_hair_high.model",
                    "face/hat.model",
                    "gloves/keeper_glove_r.model",
                    "materials.mtl",
                    "boots/materials.mtl",
                    "common/hat.mtl",
                ],
                Engine::Fox
            ),
            [
                model(ModelPackage::Boots, "boots"),
                Some(PlayerFile::Material),
                model(ModelPackage::Face, "hair_high"),
                model(ModelPackage::Face, "fcl_hair"),
                model(ModelPackage::Gloves, "glove_r"),
                Some(PlayerFile::Material),
                Some(PlayerFile::Material),
                Some(PlayerFile::Material),
            ]
        );
        // An FMDL of its stem beats it, its `.mtl` keeping the role nothing reads; one of its
        // name in another directory does not, nor does one of another stem.
        assert_eq!(
            engine_roles(
                &[
                    "boots.fmdl",
                    "Boots.model",
                    "boots.mtl",
                    "boots/boots.fmdl",
                    "fcl_hair.model",
                    "fcl_hair_x.fmdl",
                ],
                Engine::Fox
            ),
            [
                model(ModelPackage::Boots, "boots"),
                None,
                Some(PlayerFile::Material),
                model(ModelPackage::Boots, "boots"),
                model(ModelPackage::Face, "fcl_hair"),
                model(ModelPackage::Face, "fcl_hair"),
            ]
        );
        // Under `ingame_face` a converted model takes the role an FMDL of its name would.
        let marked = |names: &[&str]| -> Vec<Option<PlayerFile>> {
            let folder = PlayerFolder {
                ingame_face: Some(ingame_face_toml()),
                ..folder(names)
            };
            let models = FolderModels::of_player_files(
                &folder.path,
                &folder.files,
                folder.ingame_face.is_some(),
                Engine::Fox,
            );
            folder
                .files
                .iter()
                .map(|file| player_file(&folder.path, file, &models))
                .collect()
        };
        assert_eq!(
            marked(&["torso.model", "torso.skl"]),
            [
                model(ModelPackage::Boots, "boots"),
                skeleton(ModelPackage::Boots, "boots.skl")
            ]
        );
    }

    #[test]
    fn a_model_is_beaten_by_one_of_the_target_s_format_of_its_path_stem() {
        let folder = folder(&[
            "boots.fmdl",
            "Boots.model",
            "boots.mtl",
            "boots/boots.model",
            "face/hat.fmdl",
            "hat.model",
            // A glTF is beaten by the target's format and beats the other engine's.
            "boots.glb",
            "face/hat.glb",
            "boots/boots.gltf",
        ]);
        for (engine, expected) in [
            (
                Engine::Fox,
                [false, true, false, true, false, false, true, true, false],
            ),
            (
                Engine::PreFox,
                [true, false, false, false, true, false, true, false, true],
            ),
        ] {
            let models = FolderModels::of_player_files(&folder.path, &folder.files, false, engine);
            let beaten: Vec<bool> = folder
                .files
                .iter()
                .map(|file| models.beaten(file))
                .collect();
            assert_eq!(beaten, expected, "{engine:?}");
        }
    }

    #[test]
    fn a_gltf_beats_the_other_engine_s_model_of_its_stem_and_loses_to_the_target_s() {
        let gltf = || Some(PlayerFile::UnsupportedGltf);
        for engine in [Engine::Fox, Engine::PreFox] {
            // Alone, in either spelling and at any position, a glTF is its stem's selection.
            for name in ["boots.glb", "boots.gltf", "Boots.GLB", "boots/boots.glb"] {
                assert_eq!(engine_roles(&[name], engine), [gltf()], "{name} {engine:?}");
            }
            // In another directory of the folder it is another stem: the FMDL and the `.model`
            // beside it decide nothing about it.
            assert_eq!(
                engine_roles(&["boots/boots.glb", "boots.fmdl", "boots.model"], engine)[0],
                gltf(),
                "{engine:?}"
            );
        }
        for extension in ["glb", "gltf"] {
            let gltf_name = format!("boots.{extension}");
            // Fox: the FMDL beats the glTF, and the glTF the `.model`.
            assert_eq!(
                engine_roles(&["boots.fmdl", &gltf_name], Engine::Fox),
                [model(ModelPackage::Boots, "boots"), None],
                "{gltf_name}"
            );
            assert_eq!(
                engine_roles(&[&gltf_name, "Boots.model"], Engine::Fox),
                [gltf(), None],
                "{gltf_name}"
            );
            // Pre-Fox: the glTF beats the FMDL, and the `.model` the glTF.
            assert_eq!(
                engine_roles(&["boots.fmdl", &gltf_name], Engine::PreFox),
                [None, gltf()],
                "{gltf_name}"
            );
            assert_eq!(
                engine_roles(&[&gltf_name, "boots.model"], Engine::PreFox),
                [None, pre_fox_model("parts")],
                "{gltf_name}"
            );
        }
        // A model the glTF beats is no model of the folder's: no skeleton pairs with it.
        assert_eq!(
            engine_roles(&["boots.glb", "boots.model", "boots.skl"], Engine::Fox),
            [gltf(), None, None]
        );
        assert_eq!(
            engine_roles(&["boots.fmdl", "boots.glb", "boots.skl"], Engine::PreFox),
            [None, gltf(), None]
        );
    }

    #[test]
    fn on_fox_a_converted_model_is_a_model_of_the_folder_its_skeleton_pairs_with() {
        // The `.skl` of a converted model's stem packs under its slot, as an FMDL's does; a
        // slotless model's is `skl_no_slot`; the face files beside a converted face model are
        // used.
        assert_eq!(
            engine_roles(
                &[
                    "kit_boots.model",
                    "Kit_Boots.skl",
                    "fcl_hair.model",
                    "fcl_hair.skl",
                    "face_high.model",
                    "face_high.skl",
                    "face_diff.bin",
                    "fcl_hair_sim.fclo",
                ],
                Engine::Fox
            ),
            [
                model(ModelPackage::Boots, "boots"),
                skeleton(ModelPackage::Boots, "boots.skl"),
                model(ModelPackage::Face, "fcl_hair"),
                skeleton(ModelPackage::Face, "fcl_hair_sim.skl"),
                model(ModelPackage::Face, "face_high"),
                Some(PlayerFile::SlotlessSkeleton),
                packed(ModelPackage::Face, "face_diff.bin"),
                packed(ModelPackage::Face, "fcl_hair_sim.fclo"),
            ]
        );
        assert_eq!(
            engine_roles(&["boots.fmdl", "hat.model", "hat.skl"], Engine::Fox),
            [
                model(ModelPackage::Boots, "boots"),
                model(ModelPackage::Face, "fcl_hair"),
                skeleton(ModelPackage::Face, "fcl_hair_sim.skl"),
            ]
        );
        // A converted face model gives the folder a face, which the boots alone do not.
        assert_eq!(
            engine_roles(
                &["face_diff.bin", "boots.fmdl", "face_high.model"],
                Engine::Fox
            )[0],
            packed(ModelPackage::Face, "face_diff.bin")
        );
        assert_eq!(
            engine_roles(&["face_diff.bin", "boots.fmdl", "boots.model"], Engine::Fox)[0],
            Some(PlayerFile::UnusedFaceFile)
        );
    }

    #[test]
    fn on_pre_fox_a_face_file_is_used_only_beside_a_model_with_a_role() {
        assert_eq!(
            engine_roles(&["face_diff.xml", "skin.dds"], Engine::PreFox),
            [
                Some(PlayerFile::UnusedFaceFile),
                Some(PlayerFile::Texture {
                    below: "skin".to_owned(),
                    format: SourceFormat::Dds,
                })
            ]
        );
        assert_eq!(
            engine_roles(&["face_diff.xml", "boots/hat.model"], Engine::PreFox),
            [Some(PlayerFile::FaceDiffXml), pre_fox_model("parts")]
        );
    }

    #[test]
    fn on_pre_fox_under_ingame_face_every_model_but_a_glove_is_a_boots_part() {
        let names = [
            "shirt.model",
            "kit_boots.model",
            "torso.model",
            "shirt.mtl",
            "face_diff.bin",
            "x_gloveL.model",
            "gloves/keeper_handR.model",
        ];
        // Each typed as without the marker: a glove's type is what its `glove.xml` lists.
        let part = |package, xml_type: &str| {
            Some(PlayerFile::PreFoxPart {
                package,
                xml_type: xml_type.to_owned(),
            })
        };
        let folder = PlayerFolder {
            ingame_face: Some(ingame_face_toml()),
            ..folder(&names)
        };
        let models = FolderModels::of_player_files(
            &folder.path,
            &folder.files,
            folder.ingame_face.is_some(),
            Engine::PreFox,
        );
        let marked: Vec<Option<PlayerFile>> = folder
            .files
            .iter()
            .map(|file| player_file(&folder.path, file, &models))
            .collect();
        assert_eq!(
            marked,
            [
                part(ModelPackage::Boots, "shirt"),
                part(ModelPackage::Boots, "parts"),
                part(ModelPackage::Boots, "parts"),
                Some(PlayerFile::Material),
                Some(PlayerFile::UnusedFaceFile),
                part(ModelPackage::Gloves, "gloveL"),
                part(ModelPackage::Gloves, "handR"),
            ]
        );
        assert_eq!(
            part(ModelPackage::Boots, "parts").unwrap().package(),
            Some(ModelPackage::Boots)
        );
        // Without the marker the same files are the face's.
        assert_eq!(
            engine_roles(&names, Engine::PreFox),
            [
                pre_fox_model("shirt"),
                pre_fox_model("parts"),
                pre_fox_model("parts"),
                Some(PlayerFile::Material),
                packed(ModelPackage::Face, "face_diff.bin"),
                pre_fox_model("gloveL"),
                pre_fox_model("handR"),
            ]
        );
    }

    #[test]
    fn on_pre_fox_under_ingame_face_an_fmdl_part_s_skeleton_is_its_conversion_s_bind_pose() {
        let names = [
            "torso.fmdl",
            "torso.skl",
            "x_gloveL.fmdl",
            "x_gloveL.skl",
            "pants_kit1.fmdl",
            "pants_kit1.skl",
            "boots.fmdl",
            "boots.model",
            "boots.skl",
            "face_diff.bin",
        ];
        let folder = PlayerFolder {
            ingame_face: Some(ingame_face_toml()),
            ..folder(&names)
        };
        let models = FolderModels::of_player_files(
            &folder.path,
            &folder.files,
            folder.ingame_face.is_some(),
            Engine::PreFox,
        );
        let marked: Vec<Option<PlayerFile>> = folder
            .files
            .iter()
            .map(|file| player_file(&folder.path, file, &models))
            .collect();
        let part = |package, xml_type: &str| {
            Some(PlayerFile::PreFoxPart {
                package,
                xml_type: xml_type.to_owned(),
            })
        };
        // A per-kit model alone is an ordinary part. An FMDL a `.model` of its stem beats is
        // none: it is not converted, so no skeleton is its bind pose. The face files are still
        // not used: the converted parts make no face.
        assert_eq!(
            marked,
            [
                part(ModelPackage::Boots, "parts"),
                Some(PlayerFile::ConversionSkeleton),
                part(ModelPackage::Gloves, "gloveL"),
                Some(PlayerFile::ConversionSkeleton),
                part(ModelPackage::Boots, "parts"),
                Some(PlayerFile::ConversionSkeleton),
                None,
                part(ModelPackage::Boots, "parts"),
                None,
                Some(PlayerFile::UnusedFaceFile),
            ]
        );
    }

    #[test]
    fn on_pre_fox_under_ingame_face_a_common_model_link_is_a_part_as_a_model_of_its_stem_is() {
        let names = [
            "glove_l.model.common",
            "kit_boots.model.common",
            "legs.model.common",
            "keeper_handR.model.common",
            "body.mtl.common",
            "hair.png.common",
        ];
        let part = |package, xml_type: &str| {
            Some(PlayerFile::PreFoxPart {
                package,
                xml_type: xml_type.to_owned(),
            })
        };
        let folder = PlayerFolder {
            ingame_face: Some(ingame_face_toml()),
            ..folder(&names)
        };
        let models = FolderModels::of_player_files(
            &folder.path,
            &folder.files,
            folder.ingame_face.is_some(),
            Engine::PreFox,
        );
        let marked: Vec<Option<PlayerFile>> = folder
            .files
            .iter()
            .map(|file| player_file(&folder.path, file, &models))
            .collect();
        assert_eq!(
            marked,
            [
                part(ModelPackage::Gloves, "gloveL"),
                part(ModelPackage::Boots, "parts"),
                part(ModelPackage::Boots, "parts"),
                part(ModelPackage::Gloves, "handR"),
                Some(PlayerFile::CommonMaterial),
                Some(PlayerFile::CommonTexture("hair".to_owned())),
            ]
        );
        // Without the marker each model link is listed by reference in the face's `face.xml`.
        let common_model = |xml_type: &str| {
            Some(PlayerFile::PreFoxCommonModel {
                xml_type: xml_type.to_owned(),
            })
        };
        assert_eq!(
            engine_roles(&names, Engine::PreFox),
            [
                common_model("gloveL"),
                common_model("parts"),
                common_model("parts"),
                common_model("handR"),
                Some(PlayerFile::CommonMaterial),
                Some(PlayerFile::CommonTexture("hair".to_owned())),
            ]
        );
    }

    #[test]
    fn on_pre_fox_a_per_kit_model_is_typed_as_its_set_where_a_face_xml_lists_it() {
        assert_eq!(
            engine_roles(&["pants_kit1.model", "boots_kit1.model"], Engine::PreFox),
            [pre_fox_model("parts"), pre_fox_model("parts")]
        );
        // Behind a link no `face.xml` entry names the set: the link has no role.
        assert_eq!(link_role("pants_kit1.model.common", false), None);
        // Nor is it a part under `ingame_face`: every variant would be worn at once.
        assert_eq!(link_role("boots_kit1.model.common", true), None);
        // Nor under `ingame_face`, his boots holding parts: the lowest variant is a part, the
        // others are left out.
        let names = ["boots_kit1.model", "boots_kit2.model"];
        let folder = PlayerFolder {
            ingame_face: Some(ingame_face_toml()),
            ..folder(&names)
        };
        let models = FolderModels::of_player_files(
            &folder.path,
            &folder.files,
            folder.ingame_face.is_some(),
            Engine::PreFox,
        );
        let marked: Vec<Option<PlayerFile>> = folder
            .files
            .iter()
            .map(|file| player_file(&folder.path, file, &models))
            .collect();
        assert_eq!(
            marked,
            [
                Some(PlayerFile::PreFoxPart {
                    package: ModelPackage::Boots,
                    xml_type: "parts".to_owned()
                }),
                Some(PlayerFile::LeftOutKitVariant),
            ]
        );
        // Without the marker the face lists the set once: both variants are its models.
        assert_eq!(
            engine_roles(&names, Engine::PreFox),
            [pre_fox_model("parts"), pre_fox_model("parts")]
        );
    }

    /// The role of each file of `names` in the shared folder `folder` of `kind` on a target of
    /// `engine`.
    fn shared_roles(
        folder: &str,
        kind: SharedKind,
        names: &[&str],
        engine: Engine,
    ) -> Vec<Option<PlayerFile>> {
        let folder = ScopePath::new(folder).unwrap();
        let files: Vec<FileDescriptor> = names
            .iter()
            .map(|name| {
                let path = ScopePath::new(&format!("{}/{name}", folder.as_str())).unwrap();
                FileDescriptor {
                    size: 0,
                    kind: classify(path.name()),
                    source: path.clone(),
                    path,
                }
            })
            .collect();
        let models = FolderModels::of_shared(&folder, &files, kind, engine);
        files
            .iter()
            .map(|file| player_file(&folder, file, &models))
            .collect()
    }

    #[test]
    fn on_pre_fox_a_shared_boots_folder_s_per_kit_set_uses_its_lowest_variant_and_a_face_s_all() {
        // One `boots.model` merges every model of the folder: the others are left out.
        assert_eq!(
            shared_roles(
                "Boots/Studs",
                SharedKind::Boots,
                &["boots_kit1.model", "boots_kit2.model"],
                Engine::PreFox
            ),
            [pre_fox_model("parts"), Some(PlayerFile::LeftOutKitVariant)]
        );
        // A shared face's set is listed once by each linking player's `face.xml`.
        assert_eq!(
            shared_roles(
                "Faces/Round",
                SharedKind::Face,
                &["face_high_kit1.model", "face_high_kit2.model"],
                Engine::PreFox
            ),
            [pre_fox_model("face_neck"), pre_fox_model("face_neck")]
        );
    }

    #[test]
    fn on_pre_fox_a_boots_or_gloves_folder_s_face_xml_has_no_role_and_a_face_s_is_a_face_file() {
        let names = ["face_high.model", "face_high.mtl", "face.xml"];
        assert_eq!(
            shared_roles("Faces/Round", SharedKind::Face, &names, Engine::PreFox),
            [
                pre_fox_model("face_neck"),
                Some(PlayerFile::Material),
                Some(PlayerFile::FaceXml)
            ]
        );
        // A face folder holding its xml alone has a face: the xml may name only Common models.
        assert_eq!(
            shared_roles(
                "Faces/Round",
                SharedKind::Face,
                &["face.xml"],
                Engine::PreFox
            ),
            [Some(PlayerFile::FaceXml)]
        );
        for kind in [SharedKind::Boots, SharedKind::Gloves] {
            assert_eq!(
                shared_roles(
                    "Boots/Crocs",
                    kind,
                    &["boots.model", "face.xml"],
                    Engine::PreFox
                )[1],
                None,
                "{kind:?}"
            );
        }
        assert_eq!(
            engine_roles(&names, Engine::PreFox),
            [
                pre_fox_model("face_neck"),
                Some(PlayerFile::Material),
                Some(PlayerFile::FaceXml)
            ]
        );
    }

    #[test]
    fn a_shared_boots_or_gloves_folder_gives_no_face_whatever_models_it_holds() {
        for kind in [SharedKind::Boots, SharedKind::Gloves] {
            for (names, engine) in [
                (["boots.model", "face_diff.bin"], Engine::PreFox),
                (["boots.fmdl", "face_diff.bin"], Engine::Fox),
                // A model named as face content, a part of the hair in a player's folder.
                (["crocs.fmdl", "face_diff.bin"], Engine::Fox),
            ] {
                assert_eq!(
                    shared_roles("Boots/Crocs", kind, &names, engine)[1],
                    Some(PlayerFile::UnusedFaceFile),
                    "{kind:?} {names:?}"
                );
            }
        }
        // A shared face folder's face diff is its face's.
        assert_eq!(
            shared_roles(
                "Faces/Round",
                SharedKind::Face,
                &["crocs.fmdl", "face_diff.bin"],
                Engine::Fox
            )[1],
            Some(PlayerFile::Packed {
                package: ModelPackage::Face,
                name: "face_diff.bin"
            })
        );
    }

    #[test]
    fn a_shared_folder_s_roles_follow_a_player_folder_s_at_any_depth() {
        assert_eq!(
            shared_roles(
                "Boots/Crocs",
                SharedKind::Boots,
                &["boots.fmdl", "boots/extra.fmdl"],
                Engine::Fox
            ),
            [
                Some(PlayerFile::Model {
                    package: ModelPackage::Boots,
                    name: "boots"
                }),
                // As a player's subfolder model is: face content by its name (`fcl_hair`).
                Some(PlayerFile::Model {
                    package: ModelPackage::Face,
                    name: "fcl_hair"
                }),
            ]
        );
        assert_eq!(
            shared_roles(
                "Boots/Crocs",
                SharedKind::Boots,
                &["boots.model", "boots/extra.model", "common/skin.dds"],
                Engine::PreFox
            ),
            [
                pre_fox_model("parts"),
                pre_fox_model("parts"),
                Some(PlayerFile::Texture {
                    below: "common/skin".to_owned(),
                    format: SourceFormat::Dds,
                }),
            ]
        );
        // A per-kit set spans the tree, as a player's: the root's `kit2` is the variant a
        // lower one below a subfolder leaves out.
        assert_eq!(
            shared_roles(
                "Boots/Crocs",
                SharedKind::Boots,
                &["boots_kit2.fmdl", "boots/boots_kit1.fmdl"],
                Engine::Fox
            ),
            [
                Some(PlayerFile::LeftOutKitVariant),
                Some(PlayerFile::Model {
                    package: ModelPackage::Boots,
                    name: "boots"
                }),
            ]
        );
        // A player folder's subfolder is a player folder of its own: its variant is one of the
        // set, the lower one, which the higher one in the root is left out beside.
        assert_eq!(
            engine_roles(&["pants_kit2.fmdl", "common/pants_kit1.fmdl"], Engine::Fox),
            [
                Some(PlayerFile::LeftOutKitVariant),
                Some(PlayerFile::Model {
                    package: ModelPackage::Face,
                    name: "fcl_hair"
                })
            ]
        );
        // A singleton below a shared folder's subfolder takes no role, as a player's does
        // not: a folder has one of each, in the root alone.
        assert_eq!(
            shared_roles(
                "Faces/Round",
                SharedKind::Face,
                &["crocs.fmdl", "face_diff.bin", "jessie/face_diff.bin"],
                Engine::Fox
            ),
            [
                Some(PlayerFile::Model {
                    package: ModelPackage::Face,
                    name: "fcl_hair"
                }),
                Some(PlayerFile::Packed {
                    package: ModelPackage::Face,
                    name: "face_diff.bin"
                }),
                None
            ]
        );
    }

    #[test]
    fn each_other_player_file_has_its_role_or_none() {
        assert_eq!(
            role("skin.dds"),
            Some(PlayerFile::Texture {
                below: "skin".to_owned(),
                format: SourceFormat::Dds,
            })
        );
        assert_eq!(
            role("skin.ftex"),
            Some(PlayerFile::Texture {
                below: "skin".to_owned(),
                format: SourceFormat::Ftex,
            })
        );
        assert_eq!(
            role("skin.png"),
            Some(PlayerFile::Texture {
                below: "skin".to_owned(),
                format: SourceFormat::Png,
            })
        );
        assert_eq!(
            role("fcl_hair.skl"),
            skeleton(ModelPackage::Face, "fcl_hair_sim.skl")
        );
        assert_eq!(
            role("kit_boots.skl"),
            skeleton(ModelPackage::Boots, "boots.skl")
        );
        assert_eq!(
            role("face_diff.bin"),
            packed(ModelPackage::Face, "face_diff.bin")
        );
        assert_eq!(
            role("Face_Diff.bin"),
            packed(ModelPackage::Face, "face_diff.bin")
        );
        assert_eq!(
            role("fcl_hair_sim.fclo"),
            packed(ModelPackage::Face, "fcl_hair_sim.fclo")
        );
        // Each file plays the role its name gives, letter case aside.
        assert_eq!(
            role("Fcl_Hair_Sim.Fclo"),
            packed(ModelPackage::Face, "fcl_hair_sim.fclo")
        );
        assert_eq!(role("face_diff.xml"), Some(PlayerFile::FaceDiffXml));
        assert_eq!(role("Face_Diff.xml"), Some(PlayerFile::FaceDiffXml));
        assert_eq!(
            role("hair.png.common"),
            Some(PlayerFile::CommonTexture("hair".to_owned()))
        );
        assert_eq!(role("body.mtl.common"), Some(PlayerFile::CommonMaterial));
        for refused in [
            // A `.model` the FMDL of its stem beats.
            "kit_boots.model",
            "Fcl_Hair.model",
            "boots.skl",
            "glove_l.skl",
            "face.xml",
            "face_diff2.xml",
            "face_diff2.bin",
            "fcl_hair.fclo",
        ] {
            assert_eq!(role(refused), None, "{refused}");
        }
        // Below a subfolder of any name and depth, as in the root.
        assert_eq!(role("common/body.mtl"), Some(PlayerFile::Material));
        assert_eq!(
            role("face/deep/face_high.fmdl"),
            model(ModelPackage::Face, "face_high")
        );
    }

    #[test]
    fn a_subfolder_s_model_takes_its_role_from_its_name_at_any_depth() {
        // No subfolder name forces a category, `face/`, `boots/`, `gloves/` and `common/`
        // included: the model's own name gives its package and slot, as in the root.
        for (name, package, allowed) in [
            ("face/face_high.fmdl", ModelPackage::Face, "face_high"),
            ("FACE/oral.fmdl", ModelPackage::Face, "oral"),
            ("face/torso.fmdl", ModelPackage::Face, "fcl_hair"),
            ("face/boots.fmdl", ModelPackage::Boots, "boots"),
            ("face/glove_l.fmdl", ModelPackage::Gloves, "glove_l"),
            ("boots/hair_high.fmdl", ModelPackage::Face, "hair_high"),
            ("Boots/torso.fmdl", ModelPackage::Face, "fcl_hair"),
            ("gloves/keeper.fmdl", ModelPackage::Face, "fcl_hair"),
            ("Gloves/keeper_handR.fmdl", ModelPackage::Gloves, "glove_r"),
            ("common/boots.fmdl", ModelPackage::Boots, "boots"),
            ("jessie/body/kit_boots.fmdl", ModelPackage::Boots, "boots"),
            ("jessie/x_hair_high.fmdl", ModelPackage::Face, "hair_high"),
        ] {
            assert_eq!(roles(&[name]), [model(package, allowed)], "{name}");
        }
        // Textures are the player's own at any depth, each by its path below the folder.
        for (name, expected) in [
            ("skin.dds", "skin"),
            ("common/skin.dds", "common/skin"),
            ("COMMON/skin.FTEX", "COMMON/skin"),
            ("jessie/shorts/shorts.dds", "jessie/shorts/shorts"),
        ] {
            let roles = roles(&[name]);
            let [Some(PlayerFile::Texture { below, .. })] = roles.as_slice() else {
                panic!("{name}");
            };
            assert_eq!(below, expected, "{name}");
        }
    }

    #[test]
    fn a_common_link_takes_the_role_the_linked_model_would_have_in_its_place() {
        for (link, package, allowed) in [
            ("legs.fmdl.common", ModelPackage::Face, "fcl_hair"),
            ("legs.FMDL.common.txt", ModelPackage::Face, "fcl_hair"),
            ("x_face_high.fmdl.common", ModelPackage::Face, "face_high"),
            ("kit_boots.fmdl.common", ModelPackage::Boots, "boots"),
            ("handL.fmdl.common", ModelPackage::Gloves, "glove_l"),
            // A link to a `.model`, converted as a player's own `.model` is.
            ("legs.model.common", ModelPackage::Face, "fcl_hair"),
            ("kit_boots.MODEL.common.txt", ModelPackage::Boots, "boots"),
            ("handR.model.common", ModelPackage::Gloves, "glove_r"),
        ] {
            assert_eq!(
                roles(&[link]),
                [Some(PlayerFile::CommonModel {
                    package,
                    name: allowed
                })],
                "{link}"
            );
        }
        // A link below a subfolder of any name takes its role as one in the root, by the
        // linked model's name.
        for (link, package, allowed) in [
            ("boots/kit_boots.fmdl.common", ModelPackage::Boots, "boots"),
            ("gloves/handR.model.common", ModelPackage::Gloves, "glove_r"),
            ("common/legs.fmdl.common", ModelPackage::Face, "fcl_hair"),
            (
                "other/deep/legs.fmdl.common",
                ModelPackage::Face,
                "fcl_hair",
            ),
        ] {
            assert_eq!(
                roles(&[link]),
                [Some(PlayerFile::CommonModel {
                    package,
                    name: allowed
                })],
                "{link}"
            );
        }
        assert_eq!(roles(&["materials.toml.common"]), [None]);
        // The link is a face model for the face's files, but pairs no skeleton of the
        // folder's: the Common model's skeleton is Common's.
        assert_eq!(
            roles(&["legs.fmdl.common", "face_diff.bin", "legs.skl"]),
            [
                Some(PlayerFile::CommonModel {
                    package: ModelPackage::Face,
                    name: "fcl_hair"
                }),
                packed(ModelPackage::Face, "face_diff.bin"),
                None
            ]
        );
        assert_eq!(
            roles(&["kit_boots.fmdl.common", "face_diff.bin"]),
            [
                Some(PlayerFile::CommonModel {
                    package: ModelPackage::Boots,
                    name: "boots"
                }),
                Some(PlayerFile::UnusedFaceFile)
            ]
        );
        assert_eq!(
            PlayerFile::CommonModel {
                package: ModelPackage::Gloves,
                name: "glove_l"
            }
            .package(),
            Some(ModelPackage::Gloves)
        );
    }

    #[test]
    fn a_common_link_to_a_texture_stands_for_its_stem_and_one_to_a_mtl_is_a_material_link() {
        let texture = |stem: &str| Some(PlayerFile::CommonTexture(stem.to_owned()));
        assert_eq!(
            roles(&[
                "hair.dds.common",
                "skin_nrm.png.common",
                "body.mtl.common",
                "materials.toml.common",
                "legs.fmdl.common",
            ]),
            [
                texture("hair"),
                texture("skin_nrm"),
                // The material set a `.model` of the folder's search may find.
                Some(PlayerFile::CommonMaterial),
                None,
                Some(PlayerFile::CommonModel {
                    package: ModelPackage::Face,
                    name: "fcl_hair"
                }),
            ]
        );
        // With the tolerated `.txt` tail too. Below a subfolder of any name a link counts as
        // in the root, standing for the `Common/` file at its own path: `face/hair.ftex.common`
        // for `Common/face/hair.ftex`, `common/body.mtl.common` for `Common/common/body.mtl`.
        assert_eq!(roles(&["Hair.DDS.common.txt"]), [texture("Hair")]);
        assert_eq!(roles(&["face/hair.ftex.common"]), [texture("face/hair")]);
        assert_eq!(
            roles(&["common/hair.dds.common", "common/body.mtl.common"]),
            [texture("common/hair"), Some(PlayerFile::CommonMaterial)]
        );
        for refused in ["hair.gif.common", "body.materials.toml.common"] {
            assert_eq!(roles(&[refused]), [None], "{refused}");
        }
        assert_eq!(PlayerFile::CommonTexture("hair".to_owned()).package(), None);
    }

    #[test]
    fn a_linked_texture_s_path_is_spelled_as_the_common_texture_is() {
        let file = |path: &str| {
            let path = ScopePath::new(path).unwrap();
            FileDescriptor {
                size: 1,
                kind: classify(path.name()),
                source: path.clone(),
                path,
            }
        };
        let common = vec![
            file("Common/jessie/Hair.mtl"),
            file("Common/jessie/HAIR.dds"),
        ];
        // The `.mtl` of the same stem sorts first: the spelling is the texture's alone.
        assert_eq!(common_texture_below(&common, "jessie/hair"), "jessie/HAIR");
        // Absent from `Common/`: the link's own spelling.
        assert_eq!(common_texture_below(&common, "jessie/skin"), "jessie/skin");
    }

    #[test]
    fn a_shared_folder_s_common_links_take_a_player_folder_s_roles_on_fox_only() {
        let names = [
            "face_diff.bin",
            "legs.fmdl.common",
            "x.dds.common",
            "body.mtl.common",
        ];
        // The role of each of `names` in `folder`, read as a shared folder's when `shared`.
        let roles_in = |folder: &str, shared: bool, engine| -> Vec<Option<PlayerFile>> {
            let folder = ScopePath::new(folder).unwrap();
            let files: Vec<FileDescriptor> = names
                .iter()
                .map(|name| {
                    let path = ScopePath::new(&format!("{}/{name}", folder.as_str())).unwrap();
                    FileDescriptor {
                        size: 0,
                        kind: classify(path.name()),
                        source: path.clone(),
                        path,
                    }
                })
                .collect();
            let models = if shared {
                FolderModels::of_shared(&folder, &files, SharedKind::Boots, engine)
            } else {
                FolderModels::of_player_files(&folder, &files, false, engine)
            };
            files
                .iter()
                .map(|file| player_file(&folder, file, &models))
                .collect()
        };
        // On Fox every link takes the role a player folder's link takes; the model link
        // names face content, which a boots folder never packs, and gives it no face for
        // its face diff. On pre-Fox every link stays unread.
        assert_eq!(
            roles_in("Boots/Crocs", true, Engine::Fox),
            [
                Some(PlayerFile::UnusedFaceFile),
                Some(PlayerFile::CommonModel {
                    package: ModelPackage::Face,
                    name: "fcl_hair"
                }),
                Some(PlayerFile::CommonTexture("x".to_owned())),
                Some(PlayerFile::CommonMaterial)
            ]
        );
        assert_eq!(
            roles_in("Boots/Crocs", true, Engine::PreFox),
            [Some(PlayerFile::UnusedFaceFile), None, None, None]
        );
        let texture = || Some(PlayerFile::CommonTexture("x".to_owned()));
        assert_eq!(
            roles_in("Players/05 - A", false, Engine::Fox),
            [
                packed(ModelPackage::Face, "face_diff.bin"),
                Some(PlayerFile::CommonModel {
                    package: ModelPackage::Face,
                    name: "fcl_hair"
                }),
                texture(),
                Some(PlayerFile::CommonMaterial),
            ]
        );
        assert_eq!(
            roles_in("Players/05 - A", false, Engine::PreFox),
            [
                packed(ModelPackage::Face, "face_diff.bin"),
                Some(PlayerFile::PreFoxCommonModel {
                    xml_type: "parts".to_owned()
                }),
                texture(),
                Some(PlayerFile::CommonMaterial),
            ]
        );
    }

    #[test]
    fn a_model_folder_s_file_takes_a_role_at_any_depth_but_a_singleton_s() {
        // Whether the file at the export path `path` takes a role in the folder at `folder`.
        let admitted_path = |folder: &str, path: &str| {
            let path = ScopePath::new(path).unwrap();
            let file = FileDescriptor {
                size: 0,
                kind: classify(path.name()),
                source: path.clone(),
                path,
            };
            role_position(&ScopePath::new(folder).unwrap(), &file)
        };
        let admitted_at =
            |folder: &str, name: &str| admitted_path(folder, &format!("{folder}/{name}"));
        for name in [
            "x.skl",
            "face/x.skl",
            "common/x.skl",
            "extra/x.skl",
            "face/deeper/x.skl",
            "x.fmdl.common",
            "x.dds.common",
            // A link below a subfolder stands for the `Common/` file at its own path.
            "extra/x.fmdl.common",
            "extra/deeper/x.mtl.common",
            "boots/x.dds.common",
        ] {
            assert!(admitted_at("Players/05 - A", name), "{name}");
        }
        // A shared folder follows the same rule: model content at any depth, but a
        // singleton in the root alone.
        assert!(admitted_at("Boots/Crocs", "x.skl"));
        assert!(admitted_at("Boots/Crocs", "boots/x.skl"));
        assert!(admitted_at("Boots/Crocs", "face_diff.bin"));
        assert!(!admitted_at("Boots/Crocs", "extra/face_diff.bin"));
        // Another folder's file takes no role in this one: `prefox_face::names_model` hands it
        // a `target` that may be a linked shared face's model, not the player folder's own; and
        // a sibling whose name starts with this folder's name is not below it.
        for path in ["Faces/Round/x.model", "Players/05 - AB/x.model"] {
            assert!(!admitted_path("Players/05 - A", path), "{path}");
        }
    }

    /// `has_effective_part` for `package` of the one player folder of the export `co Midcup
    /// Parts` holding `files`, compiled for `engine`, `hand_weighted` naming the models the deep
    /// pass found carrying hand weights.
    fn effective_part(
        files: &[&str],
        hand_weighted: &[&str],
        package: ModelPackage,
        engine: Engine,
    ) -> bool {
        let files: Vec<(&str, u64)> = files.iter().map(|path| (*path, 1)).collect();
        let export = resolved("co Midcup Parts", &files, &[], None).export;
        let hand_weighted: BTreeSet<ScopePath> = hand_weighted
            .iter()
            .map(|path| ScopePath::new(path).unwrap())
            .collect();
        has_effective_part(&export, &export.players[0], package, engine, &hand_weighted)
    }

    #[test]
    fn a_folder_holds_a_package_s_model_from_its_subfolders_and_its_model_links() {
        let subfolders = [
            "Players/03 - A/parts/kit_boots.fmdl",
            "Players/03 - A/parts/hands/glove_l.fmdl",
        ];
        let part = |package| effective_part(&subfolders, &[], package, Engine::Fox);
        assert!(part(ModelPackage::Boots));
        assert!(part(ModelPackage::Gloves));
        assert!(!part(ModelPackage::Face));
        // A model in `boots/` is what its name says, here face content.
        let named = ["Players/03 - A/boots/x.fmdl"];
        let part = |package| effective_part(&named, &[], package, Engine::Fox);
        assert!(part(ModelPackage::Face));
        assert!(!part(ModelPackage::Boots));
        let linked = [
            "Players/03 - A/kit_boots.fmdl.common",
            "Common/kit_boots.fmdl",
        ];
        let part = |package| effective_part(&linked, &[], package, Engine::Fox);
        assert!(part(ModelPackage::Boots));
        assert!(!part(ModelPackage::Face));
    }

    #[test]
    fn a_linked_face_s_boots_model_is_a_part_of_the_player_s_boots_on_fox_alone() {
        let files = [
            "Players/03 - A/Round.face",
            "Faces/Round/fcl_hair.fmdl",
            "Faces/Round/boots.fmdl",
        ];
        assert!(effective_part(
            &files,
            &[],
            ModelPackage::Boots,
            Engine::Fox
        ));
        assert!(!effective_part(
            &files,
            &[],
            ModelPackage::Gloves,
            Engine::Fox
        ));
        // On pre-Fox the face's models are face content: his boots link stays plain.
        assert!(!effective_part(
            &files,
            &[],
            ModelPackage::Boots,
            Engine::PreFox
        ));
    }

    #[test]
    fn a_kit_variant_his_linked_face_s_lower_one_leaves_out_is_no_part_of_his() {
        // The face's `pants_kit1` is the set's used variant, as his face is built: his own
        // `pants_kit2` is no part, so its hands split off nothing for his gloves.
        let weighted = ["Players/03 - A/pants_kit2.fmdl"];
        let files = [
            weighted[0],
            "Players/03 - A/Round.face",
            "Faces/Round/pants_kit1.fmdl",
        ];
        assert!(!effective_part(
            &files,
            &weighted,
            ModelPackage::Gloves,
            Engine::Fox
        ));
        assert!(effective_part(
            &weighted,
            &weighted,
            ModelPackage::Gloves,
            Engine::Fox
        ));
    }

    #[test]
    fn a_hand_split_face_model_gives_the_player_s_gloves_a_part_on_fox_alone() {
        // His own face model, a linked face's, and the Common model a link of his loads.
        let cases: [(&[&str], &str); 3] = [
            (&["Players/03 - A/body.fmdl"], "Players/03 - A/body.fmdl"),
            (
                &["Players/03 - A/Round.face", "Faces/Round/body.fmdl"],
                "Faces/Round/body.fmdl",
            ),
            (
                &["Players/03 - A/body.fmdl.common", "Common/body.fmdl"],
                "Common/body.fmdl",
            ),
        ];
        for (files, weighted) in cases {
            let part = |hand_weighted: &[&str], engine| {
                effective_part(files, hand_weighted, ModelPackage::Gloves, engine)
            };
            assert!(part(&[weighted], Engine::Fox), "{weighted}");
            assert!(!part(&[], Engine::Fox), "{weighted}: no hand weights");
            // On pre-Fox the split stays inside the face.
            assert!(!part(&[weighted], Engine::PreFox), "{weighted}");
        }
        // A boots model is never split, whatever its weights.
        let boots = ["Players/03 - A/boots.fmdl"];
        assert!(!effective_part(
            &boots,
            &["Players/03 - A/boots.fmdl"],
            ModelPackage::Gloves,
            Engine::Fox
        ));
    }

    #[test]
    fn only_the_hair_and_the_boots_have_a_skeleton_slot() {
        assert_eq!(
            skeleton_slot(ModelPackage::Face, "fcl_hair"),
            Some("fcl_hair_sim.skl")
        );
        assert_eq!(
            skeleton_slot(ModelPackage::Boots, "boots"),
            Some("boots.skl")
        );
        for name in ["face_high", "hair_high", "oral"] {
            assert_eq!(skeleton_slot(ModelPackage::Face, name), None, "{name}");
        }
        assert_eq!(skeleton_slot(ModelPackage::Gloves, "glove_l"), None);
    }

    #[test]
    fn a_common_file_is_found_directly_in_common_by_folded_name() {
        let common: Vec<FileDescriptor> =
            ["Common/Legs.fmdl", "Common/legs.skl", "Common/sub/x.fmdl"]
                .iter()
                .map(|path| {
                    let path = ScopePath::new(path).unwrap();
                    FileDescriptor {
                        size: 0,
                        kind: aesthetics_export::classify(path.name()),
                        source: path.clone(),
                        path,
                    }
                })
                .collect();
        assert_eq!(
            common_file(&common, "legs.fmdl").map(|file| file.path.as_str()),
            Some("Common/Legs.fmdl")
        );
        assert_eq!(common_file(&common, "x.fmdl"), None);
        assert_eq!(
            common_skeleton(&common, "LEGS.fmdl").map(|file| file.path.as_str()),
            Some("Common/legs.skl")
        );
        assert_eq!(common_skeleton(&common, "torso.fmdl"), None);
    }

    #[test]
    fn a_common_file_below_a_subfolder_is_found_by_its_path_and_its_companions_beside_it() {
        let common: Vec<FileDescriptor> = [
            "Common/x.skl",
            "Common/sub/x.fmdl",
            "Common/jessie/body.fmdl",
            "Common/jessie/Body.model",
            "Common/jessie/body.skl",
            "Common/body.model",
        ]
        .iter()
        .map(|path| {
            let path = ScopePath::new(path).unwrap();
            FileDescriptor {
                size: 0,
                kind: aesthetics_export::classify(path.name()),
                source: path.clone(),
                path,
            }
        })
        .collect();
        let found = |file: Option<&FileDescriptor>| file.map(|file| file.path.as_str().to_owned());
        assert_eq!(
            found(common_file(&common, "SUB/x.fmdl")).as_deref(),
            Some("Common/sub/x.fmdl")
        );
        assert_eq!(common_file(&common, "x.fmdl"), None);
        assert_eq!(common_file(&common, "jessie/sub/x.fmdl"), None);
        // The skeleton and the beating model are looked for beside the named file.
        assert_eq!(
            found(common_skeleton(&common, "jessie/body.fmdl")).as_deref(),
            Some("Common/jessie/body.skl")
        );
        assert_eq!(common_skeleton(&common, "sub/x.fmdl"), None);
        assert_eq!(
            found(selected_common_model(
                &common,
                "jessie/body.fmdl",
                Engine::PreFox
            ))
            .as_deref(),
            Some("Common/jessie/Body.model")
        );
        assert_eq!(
            found(selected_common_model(
                &common,
                "jessie/body.fmdl",
                Engine::Fox
            ))
            .as_deref(),
            Some("Common/jessie/body.fmdl")
        );
    }

    #[test]
    fn a_link_to_a_common_subfolder_s_model_or_mtl_brings_the_textures_beside_it() {
        // The export paths `linked_common_files` reports for `Players/03 - A` holding `link`.
        let linked = |link: &str| -> BTreeSet<String> {
            let (export, _) = resolved_with_issues(
                "co Midcup Link",
                &[
                    ("Players/03 - A/face_high.fmdl", 1),
                    (link, 0),
                    ("Common/jessie/body.fmdl", 1),
                    ("Common/jessie/body.skl", 1),
                    ("Common/jessie/body.mtl", 1),
                    ("Common/jessie/other.mtl", 1),
                    ("Common/jessie/hair.dds", 1),
                    ("Common/jessie/sub/x.dds", 1),
                    ("Common/body.fmdl", 1),
                    ("Common/hair.dds", 1),
                ],
                &[],
                None,
            );
            linked_common_files(&export.export)
                .iter()
                .map(|path| path.as_str().to_owned())
                .collect()
        };
        // A model link brings its stem's models and `.skl`, every `.mtl` and every texture
        // directly in the directory; a `.mtl` link the textures alone; neither goes below
        // `sub/` nor up to `Common/`'s own.
        assert_eq!(
            linked("Players/03 - A/jessie/body.fmdl.common"),
            [
                "Common/jessie/body.fmdl",
                "Common/jessie/body.mtl",
                "Common/jessie/body.skl",
                "Common/jessie/hair.dds",
                "Common/jessie/other.mtl",
            ]
            .iter()
            .map(|path| (*path).to_owned())
            .collect()
        );
        assert_eq!(
            linked("Players/03 - A/jessie/body.mtl.common"),
            ["Common/jessie/body.mtl", "Common/jessie/hair.dds"]
                .iter()
                .map(|path| (*path).to_owned())
                .collect()
        );
        // A texture link still brings nothing beside it.
        assert_eq!(
            linked("Players/03 - A/jessie/hair.dds.common"),
            ["Common/jessie/hair.dds"]
                .iter()
                .map(|path| (*path).to_owned())
                .collect()
        );
    }

    #[test]
    fn a_common_file_naming_a_kit_set_finds_its_lowest_variant_and_the_fallback_nothing() {
        let common: Vec<FileDescriptor> = [
            "Common/kit2/a.dds",
            "Common/kit1/a.dds",
            "Common/kit1/other.dds",
            "Common/kitF/b.dds",
            "Common/pants_kit2.dds",
        ]
        .iter()
        .map(|path| {
            let path = ScopePath::new(path).unwrap();
            FileDescriptor {
                size: 0,
                kind: aesthetics_export::classify(path.name()),
                source: path.clone(),
                path,
            }
        })
        .collect();
        // The set's `kitN` directory names its lowest variant.
        assert_eq!(
            common_file(&common, "kitN/a.dds").map(|file| file.path.as_str()),
            Some("Common/kit1/a.dds")
        );
        // `kit1/other.dds` is itself a member of `other`'s set; `c`'s has none.
        assert_eq!(
            common_file(&common, "kitN/other.dds").map(|file| file.path.as_str()),
            Some("Common/kit1/other.dds")
        );
        assert_eq!(common_file(&common, "kitN/c.dds"), None);
        // The stem form's too.
        assert_eq!(
            common_file(&common, "pants_kitN.dds").map(|file| file.path.as_str()),
            Some("Common/pants_kit2.dds")
        );
        // A path naming the `kitF` fallback finds nothing, `kitF` files included.
        assert_eq!(common_file(&common, "kitF/a.dds"), None);
        assert_eq!(common_file(&common, "kitF/b.dds"), None);
        assert_eq!(common_file(&common, "pants_kitF.dds"), None);
    }

    #[test]
    fn a_skeleton_beside_a_kitf_model_is_found() {
        let common: Vec<FileDescriptor> = ["Common/boots_kitF.fmdl", "Common/boots_kitF.skl"]
            .iter()
            .map(|path| {
                let path = ScopePath::new(path).unwrap();
                FileDescriptor {
                    size: 0,
                    kind: aesthetics_export::classify(path.name()),
                    source: path.clone(),
                    path,
                }
            })
            .collect();
        // A `.skl` lookup is no texture lookup: `kitF` is a plain stem to it.
        assert_eq!(
            common_skeleton(&common, "boots_kitF.fmdl").map(|file| file.path.as_str()),
            Some("Common/boots_kitF.skl")
        );
    }

    #[test]
    fn a_texture_link_naming_a_kit_set_reaches_every_member() {
        let (export, _) = resolved_with_issues(
            "co Midcup Link",
            &[
                ("Players/03 - A/face_high.fmdl", 1),
                ("Players/03 - A/kitN/a.dds.common", 0),
                ("Common/kit1/a.dds", 1),
                ("Common/kit2/a.dds", 1),
                ("Common/kitF/a.dds", 1),
                ("Common/kit1/other.dds", 1),
            ],
            &[],
            None,
        );
        let linked: BTreeSet<String> = linked_common_files(&export.export)
            .iter()
            .map(|path| path.as_str().to_owned())
            .collect();
        // Every member's directory counts as linked, the `kitF` member's included;
        // `kit1`'s other file is no member.
        assert_eq!(
            linked,
            [
                "Common/kit1/a.dds",
                "Common/kit2/a.dds",
                "Common/kitF/a.dds"
            ]
            .iter()
            .map(|path| (*path).to_owned())
            .collect()
        );
    }

    #[test]
    fn a_link_to_a_deeper_common_file_brings_the_textures_above_it_too() {
        // `Common/jessie/parts/body.fmdl`'s names resolve nearest first from `parts/` up
        // to `Common/`: a link to it or to its `.mtl` brings `jessie/`'s textures as well
        // as `parts/`'s, nothing of `jessie/other/` (a bare name never resolves down),
        // and none of `Common/`'s own, packed anyway.
        let linked = |link: &str| -> BTreeSet<String> {
            let (export, _) = resolved_with_issues(
                "co Midcup Link",
                &[
                    ("Players/03 - A/face_high.fmdl", 1),
                    (link, 0),
                    ("Common/jessie/parts/body.fmdl", 1),
                    ("Common/jessie/parts/body.mtl", 1),
                    ("Common/jessie/parts/body.skl", 1),
                    ("Common/jessie/parts/x.dds", 1),
                    ("Common/jessie/skin.dds", 1),
                    ("Common/jessie/other/y.dds", 1),
                    ("Common/skin.dds", 1),
                ],
                &[],
                None,
            );
            linked_common_files(&export.export)
                .iter()
                .map(|path| path.as_str().to_owned())
                .collect()
        };
        assert_eq!(
            linked("Players/03 - A/jessie/parts/body.fmdl.common"),
            [
                "Common/jessie/parts/body.fmdl",
                "Common/jessie/parts/body.mtl",
                "Common/jessie/parts/body.skl",
                "Common/jessie/parts/x.dds",
                "Common/jessie/skin.dds",
            ]
            .iter()
            .map(|path| (*path).to_owned())
            .collect()
        );
        assert_eq!(
            linked("Players/03 - A/jessie/parts/body.mtl.common"),
            [
                "Common/jessie/parts/body.mtl",
                "Common/jessie/parts/x.dds",
                "Common/jessie/skin.dds",
            ]
            .iter()
            .map(|path| (*path).to_owned())
            .collect()
        );
    }

    #[test]
    fn a_subfolder_s_skeleton_pairs_with_the_model_beside_it_under_its_name_s_slot() {
        // The skeleton is named after the model in its own directory: the root's `kit_boots.skl`
        // does not pair with `boots/kit_boots.fmdl`, nor the other way round.
        assert_eq!(
            roles(&[
                "boots/kit_boots.fmdl",
                "boots/kit_boots.skl",
                "kit_boots.skl"
            ]),
            [
                model(ModelPackage::Boots, "boots"),
                skeleton(ModelPackage::Boots, "boots.skl"),
                None
            ]
        );
        assert_eq!(
            roles(&["kit_boots.fmdl", "boots/kit_boots.skl"]),
            [model(ModelPackage::Boots, "boots"), None]
        );
        // The slot follows the model's own name: a `hair_high` in `boots/` is face content
        // with no slot; a part named for no part, at any depth, is the hair's.
        assert_eq!(
            roles(&["boots/hair_high.fmdl", "boots/hair_high.skl"]),
            [
                model(ModelPackage::Face, "hair_high"),
                Some(PlayerFile::SlotlessSkeleton)
            ]
        );
        assert_eq!(
            roles(&["jessie/body/torso.fmdl", "jessie/body/torso.skl"]),
            [
                model(ModelPackage::Face, "fcl_hair"),
                skeleton(ModelPackage::Face, "fcl_hair_sim.skl")
            ]
        );
        // The face's files count directly in the folder alone, whatever model a subfolder
        // holds: a subfolder's are none (the structure pass names them).
        assert_eq!(
            roles(&[
                "parts/face_diff.bin",
                "parts/fcl_hair_sim.fclo",
                "parts/torso.fmdl",
                "a/b/face_diff.xml",
                "fcl_hair_sim.fclo"
            ]),
            [
                None,
                None,
                model(ModelPackage::Face, "fcl_hair"),
                None,
                packed(ModelPackage::Face, "fcl_hair_sim.fclo")
            ]
        );
        assert_eq!(
            roles(&["face_diff.bin", "boots/boots.fmdl"]),
            [
                Some(PlayerFile::UnusedFaceFile),
                model(ModelPackage::Boots, "boots")
            ]
        );
    }

    #[test]
    fn a_skeleton_pairs_with_the_model_of_its_name_and_face_files_need_a_face_model() {
        // The skeleton is named after the model, whatever the model's free prefix.
        assert_eq!(
            roles(&["x_fcl_hair.fmdl", "x_fcl_hair.skl", "fcl_hair.skl"]),
            [
                model(ModelPackage::Face, "fcl_hair"),
                skeleton(ModelPackage::Face, "fcl_hair_sim.skl"),
                None
            ]
        );
        assert_eq!(
            roles(&["boots.fmdl", "boots.skl", "kit_boots.skl"]),
            [
                model(ModelPackage::Boots, "boots"),
                skeleton(ModelPackage::Boots, "boots.skl"),
                None
            ]
        );
        // Every boots part pairs its own skeleton, not only the first, and so does every hair
        // part.
        assert_eq!(
            roles(&[
                "a_boots.fmdl",
                "a_boots.skl",
                "kit_boots.fmdl",
                "kit_boots.skl"
            ]),
            [
                model(ModelPackage::Boots, "boots"),
                skeleton(ModelPackage::Boots, "boots.skl"),
                model(ModelPackage::Boots, "boots"),
                skeleton(ModelPackage::Boots, "boots.skl")
            ]
        );
        assert_eq!(
            roles(&[
                "a_fcl_hair.fmdl",
                "a_fcl_hair.skl",
                "fcl_hair.fmdl",
                "fcl_hair.skl"
            ]),
            [
                model(ModelPackage::Face, "fcl_hair"),
                skeleton(ModelPackage::Face, "fcl_hair_sim.skl"),
                model(ModelPackage::Face, "fcl_hair"),
                skeleton(ModelPackage::Face, "fcl_hair_sim.skl")
            ]
        );
        // An unsuffixed model is a hair part: its skeleton is the hair's.
        assert_eq!(
            roles(&["torso.fmdl", "torso.skl"]),
            [
                model(ModelPackage::Face, "fcl_hair"),
                skeleton(ModelPackage::Face, "fcl_hair_sim.skl")
            ]
        );
        // A skeleton named after a face model with no skeleton slot pairs with nothing to
        // pack, whatever the model's free prefix.
        assert_eq!(
            roles(&[
                "face_high.fmdl",
                "face_high.skl",
                "x_hair_high.fmdl",
                "x_hair_high.skl",
                "oral.fmdl",
                "oral.skl"
            ]),
            [
                model(ModelPackage::Face, "face_high"),
                Some(PlayerFile::SlotlessSkeleton),
                model(ModelPackage::Face, "hair_high"),
                Some(PlayerFile::SlotlessSkeleton),
                model(ModelPackage::Face, "oral"),
                Some(PlayerFile::SlotlessSkeleton)
            ]
        );
        // Without a face model the face's files are not used, unless a shared face is
        // linked: then the shared face is the player's. An unsuffixed model is a face model.
        assert_eq!(
            roles(&["boots.fmdl", "face_diff.bin", "fcl_hair_sim.fclo"]),
            [
                model(ModelPackage::Boots, "boots"),
                Some(PlayerFile::UnusedFaceFile),
                Some(PlayerFile::UnusedFaceFile)
            ]
        );
        assert_eq!(
            roles(&["torso.fmdl", "face_diff.bin"]),
            [
                model(ModelPackage::Face, "fcl_hair"),
                packed(ModelPackage::Face, "face_diff.bin")
            ]
        );
        let folder = folder(&["boots.fmdl", "face_diff.bin", "fcl_hair_sim.fclo"]);
        let models = FolderModels::of_player_files(&folder.path, &folder.files, false, Engine::Fox)
            .with_linked_face();
        let linked: Vec<Option<PlayerFile>> = folder
            .files
            .iter()
            .map(|file| player_file(&folder.path, file, &models))
            .collect();
        assert_eq!(
            linked,
            [
                model(ModelPackage::Boots, "boots"),
                packed(ModelPackage::Face, "face_diff.bin"),
                packed(ModelPackage::Face, "fcl_hair_sim.fclo")
            ]
        );
        assert_eq!(
            roles(&["face_high.fmdl", "face_diff.bin", "fcl_hair_sim.fclo"]),
            [
                model(ModelPackage::Face, "face_high"),
                packed(ModelPackage::Face, "face_diff.bin"),
                packed(ModelPackage::Face, "fcl_hair_sim.fclo")
            ]
        );
    }

    #[test]
    fn under_ingame_face_a_model_the_hair_would_take_is_the_boots_with_its_skeleton() {
        let boots = || model(ModelPackage::Boots, "boots");
        assert_eq!(
            marked_roles(&[
                "torso.fmdl",
                "torso.skl",
                "fcl_hair.fmdl",
                "face/hat.fmdl",
                "legs.fmdl.common",
                "glove_l.fmdl",
                "face_diff.bin",
                "face_diff.xml",
                "fcl_hair_sim.fclo",
            ]),
            [
                boots(),
                skeleton(ModelPackage::Boots, "boots.skl"),
                boots(),
                boots(),
                Some(PlayerFile::CommonModel {
                    package: ModelPackage::Boots,
                    name: "boots"
                }),
                model(ModelPackage::Gloves, "glove_l"),
                Some(PlayerFile::UnusedFaceFile),
                Some(PlayerFile::UnusedFaceFile),
                Some(PlayerFile::UnusedFaceFile),
            ]
        );
    }

    #[test]
    fn a_face_file_is_not_used_without_a_face_model_and_packed_with_one() {
        let unused = || Some(PlayerFile::UnusedFaceFile);
        assert_eq!(
            roles(&["boots.fmdl", "face_diff.xml", "fcl_hair_sim.fclo"]),
            [model(ModelPackage::Boots, "boots"), unused(), unused()]
        );
        assert_eq!(
            roles(&["boots.fmdl", "face_diff.bin"]),
            [model(ModelPackage::Boots, "boots"), unused()]
        );
        assert_eq!(
            roles(&["face_high.fmdl", "face_diff.xml", "fcl_hair_sim.fclo"]),
            [
                model(ModelPackage::Face, "face_high"),
                Some(PlayerFile::FaceDiffXml),
                packed(ModelPackage::Face, "fcl_hair_sim.fclo")
            ]
        );
        assert_eq!(
            roles(&["face_high.fmdl", "face_diff.bin"]),
            [
                model(ModelPackage::Face, "face_high"),
                packed(ModelPackage::Face, "face_diff.bin")
            ]
        );
        assert_eq!(PlayerFile::UnusedFaceFile.package(), None);
    }

    #[test]
    fn a_package_is_the_models_and_packed_files_but_not_the_textures() {
        let texture = PlayerFile::Texture {
            below: "skin".to_owned(),
            format: SourceFormat::Dds,
        };
        assert_eq!(texture.package(), None);
        assert_eq!(PlayerFile::SlotlessSkeleton.package(), None);
        assert_eq!(
            skeleton(ModelPackage::Boots, "boots.skl")
                .unwrap()
                .package(),
            Some(ModelPackage::Boots)
        );
        assert_eq!(
            packed(ModelPackage::Face, "face_diff.bin")
                .unwrap()
                .package(),
            Some(ModelPackage::Face)
        );
        assert_eq!(
            model(ModelPackage::Gloves, "glove_l").unwrap().package(),
            Some(ModelPackage::Gloves)
        );
        assert_eq!(
            ModelPackage::ALL.map(ModelPackage::file_stem),
            ["face", "boots", "glove"]
        );
        assert_eq!(
            ModelPackage::ALL.map(ModelPackage::name),
            ["face", "boots", "gloves"]
        );
    }

    #[test]
    fn a_texture_format_is_its_last_extension_in_any_case() {
        assert_eq!(texture_format("kit.dds"), Some(SourceFormat::Dds));
        assert_eq!(texture_format("kit.DDS"), Some(SourceFormat::Dds));
        assert_eq!(texture_format("kit.FTEX"), Some(SourceFormat::Ftex));
        assert_eq!(texture_format("kit.png"), Some(SourceFormat::Png));
        assert_eq!(texture_format("kit.Tga"), Some(SourceFormat::Tga));
        assert_eq!(texture_format("kit.dds.png"), Some(SourceFormat::Png));
        for refused in ["kit", "kit.gif", "kit.dds."] {
            assert_eq!(texture_format(refused), None, "{refused}");
        }
    }

    #[test]
    fn a_stem_is_the_name_up_to_its_last_dot() {
        assert_eq!(file_stem("hair.png.common"), "hair.png");
        assert_eq!(file_stem("shirt.dds"), "shirt");
        assert_eq!(file_stem("NO_USE"), "NO_USE");
    }
}
