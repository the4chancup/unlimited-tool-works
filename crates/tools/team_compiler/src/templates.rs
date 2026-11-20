//! The resources bundled into the binary, and the data directory's `templates/` folder, whose
//! files replace them one by one (`spec/team_compiler/pipeline.md` "Resolved decisions",
//! "Templates and fallback bins"): a cup maintainer swaps a fallback bin or a template between
//! releases without a new build.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use pes_version::{Engine, PesVersion};
use studio_core::{Disposition, Message, Scope, ToolContext};
use uniparam::UniformParameter;

use crate::bins::{TeamColorBin, UniColorBin};
use crate::events::RunEvents;
use crate::messages::{Code, tool_message};
use crate::paths;

/// The folder's name in the data directory.
const FOLDER_NAME: &str = "templates";

/// A resource the compiler embeds: the file name its replacement in `templates/` carries, its
/// embedded bytes, and what its replacement is parsed as when it is read.
struct Resource {
    name: &'static str,
    embedded: &'static [u8],
    format: Format,
}

/// What a resource's replacement in `templates/` is parsed as when it is read: the three bins
/// and the official `DpFileList.bin` are, so one that does not parse stops the run before any
/// export is read; the other resources are packed or converted as they are, and fail where the
/// embedded one would be used.
#[derive(Clone, Copy)]
enum Format {
    /// Not parsed.
    Unparsed,
    /// A `TeamColor.bin`.
    TeamColor,
    /// A `UniColor.bin`.
    UniColor,
    /// A `UniformParameter.bin`.
    UniformParameter,
    /// A `DpFileList.bin`.
    DpFileList,
}

impl Format {
    /// `bytes`, given back once they parse as the format; the error says why they do not.
    fn check(self, bytes: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        match self {
            Format::Unparsed => Ok(bytes),
            // The bins take the bytes and give them back, so the check copies nothing.
            Format::TeamColor => Ok(TeamColorBin::read(bytes)?.into_bytes()),
            Format::UniColor => Ok(UniColorBin::read(bytes)?.into_bytes()),
            Format::UniformParameter => {
                UniformParameter::read(&bytes)?;
                Ok(bytes)
            }
            Format::DpFileList => {
                cpk::dp_file_list::read(&bytes)?;
                Ok(bytes)
            }
        }
    }
}

/// The `TeamColor.bin` a compile sets its teams' colors in when it has no installed one to
/// start from: the same for every version.
const TEAM_COLOR: Resource = Resource {
    name: "TeamColor.bin",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/bins/TeamColor.bin"
    )),
    format: Format::TeamColor,
};

/// The `UniColor.bin` a compile sets its kits' menu colors in when it has no installed one to
/// start from: the same for every version.
const UNI_COLOR: Resource = Resource {
    name: "UniColor.bin",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/bins/UniColor.bin"
    )),
    format: Format::UniColor,
};

/// PES 18's `UniformParameter.bin` base (`uniform_parameter_base`).
const UNIFORM_PARAMETER_18: Resource = Resource {
    name: "UniformParameter18.bin",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/bins/UniformParameter18.bin"
    )),
    format: Format::UniformParameter,
};

/// PES 19 to 21's `UniformParameter.bin` base (`uniform_parameter_base`).
const UNIFORM_PARAMETER_19: Resource = Resource {
    name: "UniformParameter19.bin",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/bins/UniformParameter19.bin"
    )),
    format: Format::UniformParameter,
};

/// The `kit` texture of a placeholder kit: the magenta/black checkerboard, as a DDS.
const PLACEHOLDER_KIT: Resource = Resource {
    name: "placeholder_kit.dds",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/kits/placeholder_kit.dds"
    )),
    format: Format::Unparsed,
};

/// The `kit_mask` texture of a kit compiled for PES 2015 to 2017 whose effective textures hold
/// none: a flat mask, as a DDS.
const KIT_MASK: Resource = Resource {
    name: "kit_mask.dds",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/kits/kit_mask.dds"
    )),
    format: Format::Unparsed,
};

/// The skeleton packed under a slot's name (`boots.skl`, `fcl_hair_sim.skl`) beside a boots or
/// hair model that brings no skeleton of its own: PES 2021's full-body `body.skl`, not the
/// game's four-bone boots skeleton, since exports put full-body models in both slots.
const BODY_SKELETON: Resource = Resource {
    name: "body.skl",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/skeletons/pes21/body.skl"
    )),
    format: Format::Unparsed,
};

/// The `face_diff.bin` packed into a Fox face package whose sources hold none: the face
/// parameter file the game expects beside every face's models.
const FACE_DIFF: Resource = Resource {
    name: "face_diff.bin",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/templates/face_diff.bin"
    )),
    format: Format::Unparsed,
};

/// The `fcl_hair_sim.fclo` packed beside a `fcl_hair.fmdl` whose sources hold none: a cloth
/// simulation with nothing in it.
const FCL_HAIR_SIM_FCLO: Resource = Resource {
    name: "fcl_hair_sim.fclo",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/templates/fcl_hair_sim.fclo"
    )),
    format: Format::Unparsed,
};

/// The model a pre-Fox face CPK packs as `oral_dummy_win32.model` when its `face.xml` lists no
/// `face_neck` model, the blank face's included: a model with nothing to draw.
const DUMMY_MODEL: Resource = Resource {
    name: "dummy.model",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/templates/dummy.model"
    )),
    format: Format::Unparsed,
};

/// The `.mtl` packed beside `DUMMY_MODEL` as `dummy.mtl`: an empty material set.
const DUMMY_MTL: Resource = Resource {
    name: "dummy.mtl",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/templates/dummy.mtl"
    )),
    format: Format::Unparsed,
};

/// PES 17's stock `uniform.mtl`, the material set a `.model` collar is read with.
const UNIFORM_MTL: Resource = Resource {
    name: "uniform.mtl",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/templates/uniform.mtl"
    )),
    format: Format::Unparsed,
};

/// The template environment map: a cubemap a PES 15-17 compile emits as `env.dds` into a
/// player's texture home for a metal material converted from a Fox model that names no
/// environment texture of its own, so its `Basic_CNSR` shader has a reflection to draw.
const ENVIRONMENT_MAP: Resource = Resource {
    name: "env.dds",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/templates/env.dds"
    )),
    format: Format::Unparsed,
};

/// The cup's official `DpFileList.bin`, which a compile that deploys compares the installed
/// list with: one list for every PES version.
const DPFILELIST: Resource = Resource {
    name: "DpFileList.bin",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/templates/DpFileList.bin"
    )),
    format: Format::DpFileList,
};

/// The empty CPK `upgrade-dpfl` writes for an official list entry with no file in `download/`:
/// the game loads none of the folder's CPKs when a listed one is missing.
const PLACEHOLDER_CPK: Resource = Resource {
    name: "placeholder.cpk",
    embedded: include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resources/templates/placeholder.cpk"
    )),
    format: Format::Unparsed,
};

/// The Fox referee marker: the flat square under every referee, written into the refs CPK as
/// the referees' collar when the refs export holds `ref_marker.dds`. Its base texture names
/// `common/000/sourceimages/cup_logo.dds` until the compile points it at the converted marker.
/// Not a `Resource`: no `templates/` file replaces it.
const REFEREE_MARKER: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../resources/templates/referee_marker.fmdl"
));

/// Every resource a `templates/` file can replace, in the order the replacements are reported.
const RESOURCES: [&Resource; 15] = [
    &TEAM_COLOR,
    &UNI_COLOR,
    &UNIFORM_PARAMETER_18,
    &UNIFORM_PARAMETER_19,
    &PLACEHOLDER_KIT,
    &KIT_MASK,
    &BODY_SKELETON,
    &FACE_DIFF,
    &FCL_HAIR_SIM_FCLO,
    &DUMMY_MODEL,
    &DUMMY_MTL,
    &DPFILELIST,
    &PLACEHOLDER_CPK,
    &ENVIRONMENT_MAP,
    &UNIFORM_MTL,
];

/// The folder of `templates/` whose files, each at its game path below it, replace the files
/// of the Fox referee template tree.
const REFEREES_FOX_FOLDER: &str = "referees_fox";

/// The folder of `templates/` whose files, each at its game path below it, replace the files
/// of the pre-Fox referee template tree.
const REFEREES_PREFOX_FOLDER: &str = "referees_prefox";

/// The `(game path, bytes)` entries of a referee template tree, each file embedded from
/// `resources/templates/<folder>/<game path>`. A macro because `include_bytes!` takes only
/// a literal path; the files are listed rather than the folder embedded, so a file that goes
/// missing fails the build.
macro_rules! referee_tree {
    ($folder:literal; $($path:literal),* $(,)?) => {
        [$((
            $path,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../resources/templates/",
                $folder,
                "/",
                $path
            )) as &[u8],
        )),*]
    };
}

/// The Fox referee template tree, in path order: the referee kits' textures and kit configs
/// and `RefereeAppearance.bin`, which the game needs beside the referees' own files in the
/// refs CPK. Identical files stay separate entries: each is a file of the tree, at its own
/// game path.
static REFEREES_FOX: [(&str, &[u8]); 31] = referee_tree![
    "referees_fox";
    "Asset/model/character/uniform/texture/#windx11/referee_1.ftex",
    "Asset/model/character/uniform/texture/#windx11/referee_1_srm.ftex",
    "Asset/model/character/uniform/texture/#windx11/referee_2.ftex",
    "Asset/model/character/uniform/texture/#windx11/referee_2_srm.ftex",
    "Asset/model/character/uniform/texture/#windx11/referee_3.ftex",
    "Asset/model/character/uniform/texture/#windx11/referee_3_srm.ftex",
    "Asset/model/character/uniform/texture/#windx11/referee_4.ftex",
    "Asset/model/character/uniform/texture/#windx11/referee_4_srm.ftex",
    "Asset/model/character/uniform/texture/#windx11/referee_5.ftex",
    "Asset/model/character/uniform/texture/#windx11/referee_5_srm.ftex",
    "common/character0/model/character/appearance/RefereeAppearance.bin",
    "common/character0/model/character/uniform/team/referee/referee_ACL_1.bin",
    "common/character0/model/character/uniform/team/referee/referee_ACL_2.bin",
    "common/character0/model/character/uniform/team/referee/referee_ACL_3.bin",
    "common/character0/model/character/uniform/team/referee/referee_ACL_4.bin",
    "common/character0/model/character/uniform/team/referee/referee_ACL_5.bin",
    "common/character0/model/character/uniform/team/referee/referee_CL_1.bin",
    "common/character0/model/character/uniform/team/referee/referee_CL_2.bin",
    "common/character0/model/character/uniform/team/referee/referee_CL_3.bin",
    "common/character0/model/character/uniform/team/referee/referee_CL_4.bin",
    "common/character0/model/character/uniform/team/referee/referee_DEF_1.bin",
    "common/character0/model/character/uniform/team/referee/referee_DEF_2.bin",
    "common/character0/model/character/uniform/team/referee/referee_DEF_3.bin",
    "common/character0/model/character/uniform/team/referee/referee_DEF_4.bin",
    "common/character0/model/character/uniform/team/referee/referee_DEF_5.bin",
    "common/character0/model/character/uniform/team/referee/referee_LB_1.bin",
    "common/character0/model/character/uniform/team/referee/referee_LB_2.bin",
    "common/character0/model/character/uniform/team/referee/referee_LB_3.bin",
    "common/character0/model/character/uniform/team/referee/referee_SDA_1.bin",
    "common/character0/model/character/uniform/team/referee/referee_SDA_2.bin",
    "common/character0/model/character/uniform/team/referee/referee_SDA_3.bin",
];

/// The pre-Fox referee template tree, in path order: `RefereeAppearance.bin`, the referees'
/// boots, kit models and their materials and textures, the 20 kit configs, and the referee
/// prop model with its `.mtl` and texture, which the game needs beside the referees' own files
/// in a PES 2015 to 2017 refs CPK. Identical files stay separate entries, as in
/// `REFEREES_FOX`.
static REFEREES_PREFOX: [(&str, &[u8]); 51] = referee_tree![
    "referees_prefox";
    "common/character0/model/character/appearance/RefereeAppearance.bin",
    "common/character0/model/character/boots/k0062/boots.model",
    "common/character0/model/character/boots/k0062/boots.mtl",
    "common/character0/model/character/uniform/nocloth/referee_collar_026.model",
    "common/character0/model/character/uniform/nocloth/referee_pants_016.model",
    "common/character0/model/character/uniform/team/referee/referee_ACL_1.bin",
    "common/character0/model/character/uniform/team/referee/referee_ACL_2.bin",
    "common/character0/model/character/uniform/team/referee/referee_ACL_3.bin",
    "common/character0/model/character/uniform/team/referee/referee_ACL_4.bin",
    "common/character0/model/character/uniform/team/referee/referee_ACL_5.bin",
    "common/character0/model/character/uniform/team/referee/referee_CL_1.bin",
    "common/character0/model/character/uniform/team/referee/referee_CL_2.bin",
    "common/character0/model/character/uniform/team/referee/referee_CL_3.bin",
    "common/character0/model/character/uniform/team/referee/referee_CL_4.bin",
    "common/character0/model/character/uniform/team/referee/referee_DEF_1.bin",
    "common/character0/model/character/uniform/team/referee/referee_DEF_2.bin",
    "common/character0/model/character/uniform/team/referee/referee_DEF_3.bin",
    "common/character0/model/character/uniform/team/referee/referee_DEF_4.bin",
    "common/character0/model/character/uniform/team/referee/referee_DEF_5.bin",
    "common/character0/model/character/uniform/team/referee/referee_LB_1.bin",
    "common/character0/model/character/uniform/team/referee/referee_LB_2.bin",
    "common/character0/model/character/uniform/team/referee/referee_LB_3.bin",
    "common/character0/model/character/uniform/team/referee/referee_SDA_1.bin",
    "common/character0/model/character/uniform/team/referee/referee_SDA_2.bin",
    "common/character0/model/character/uniform/team/referee/referee_SDA_3.bin",
    "common/character0/model/character/uniform/texture/referee_1.dds",
    "common/character0/model/character/uniform/texture/referee_2.dds",
    "common/character0/model/character/uniform/texture/referee_3.dds",
    "common/character0/model/character/uniform/texture/referee_4.dds",
    "common/character0/model/character/uniform/texture/referee_5.dds",
    "common/character1/model/character/parts/referee/incom_bsm.dds",
    "common/character1/model/character/parts/referee/referee_prop.model",
    "common/character1/model/character/parts/referee/referee_prop.mtl",
    "common/character1/model/character/uniform/common/999/refkit/boots_c.dds",
    "common/character1/model/character/uniform/common/999/refkit/boots_n.dds",
    "common/character1/model/character/uniform/common/999/refkit/boots_sr.dds",
    "common/character1/model/character/uniform/common/999/refkit/oral_arm_win32.model",
    "common/character1/model/character/uniform/common/999/refkit/oral_boots_win32.model",
    "common/character1/model/character/uniform/common/999/refkit/oral_hand_l_win32.model",
    "common/character1/model/character/uniform/common/999/refkit/oral_hand_r_win32.model",
    "common/character1/model/character/uniform/common/999/refkit/oral_pants_sub_win32.model",
    "common/character1/model/character/uniform/common/999/refkit/oral_pants_win32.model",
    "common/character1/model/character/uniform/common/999/refkit/oral_refshirt_win32.model",
    "common/character1/model/character/uniform/common/999/refkit/oral_sleeve_win32.model",
    "common/character1/model/character/uniform/common/999/refkit/oral_socks_win32.model",
    "common/character1/model/character/uniform/common/999/refkit/oral_thigh_win32.model",
    "common/character1/model/character/uniform/common/999/refkit/refkit.mtl",
    "common/character1/model/character/uniform/common/999/refkit/skin_color.dds",
    "common/character1/model/character/uniform/common/999/refkit/texture.dds",
    "common/character1/model/character/uniform/nocloth/socks_noguard.model",
    "common/character1/model/character/uniform/nocloth/socks_noguard_high.model",
];

/// The referee template tree of `engine`: the folder of `templates/` whose files replace its
/// files, and its embedded files.
fn referee_tree_of(engine: Engine) -> (&'static str, &'static [(&'static str, &'static [u8])]) {
    match engine {
        Engine::Fox => (REFEREES_FOX_FOLDER, &REFEREES_FOX),
        Engine::PreFox => (REFEREES_PREFOX_FOLDER, &REFEREES_PREFOX),
    }
}

/// The files of `engine`'s referee template tree in the referees' pre-Fox Common output
/// (`common/999/`, the `refkit` body), each by its path relative to it, case-folded, in path
/// order: what a refs export's `face.xml` Common reference may name besides the export's own
/// `Common/` files. Empty on Fox, whose tree holds none. A free function, not a `Templates`
/// method: a `templates/` override replaces a tree file's bytes and never adds a path, and the
/// deep pass, which needs the list, runs in `check` too, where no templates are read.
pub(crate) fn referee_common_paths(engine: Engine) -> Vec<String> {
    let common = paths::pre_fox_common_file(paths::REFEREE_TEAM_ID, "");
    let (_, tree) = referee_tree_of(engine);
    tree.iter()
        .filter_map(|(path, _)| path.strip_prefix(common.as_str()))
        .map(vtree::fold_name)
        .collect()
}

/// An override in `templates/` that cannot be read, or one of the bins or the list that does
/// not parse:
/// `template_override_unreadable`'s context.
#[derive(Debug)]
pub(crate) struct Unreadable {
    /// The override file, or the folder when it cannot be listed.
    pub(crate) path: PathBuf,
    /// What failed.
    pub(crate) error: anyhow::Error,
}

/// The resources a compile builds with: each embedded one, or the data directory's
/// `templates/` file of the same name that replaces it, so a cup's maintainers can swap a
/// fallback bin or a template between releases. Also the two referee template trees, Fox's and
/// pre-Fox's, each file embedded or replaced by the file at its game path below
/// `templates/referees_fox/` or `templates/referees_prefox/`.
pub(crate) struct Templates {
    /// The replacements read from `templates/`, by resource file name.
    overrides: BTreeMap<&'static str, Vec<u8>>,
    /// The replacements read from `templates/referees_fox/`, by game path.
    fox_referee_overrides: BTreeMap<&'static str, Vec<u8>>,
    /// The replacements read from `templates/referees_prefox/`, by game path: a map of their
    /// own, since the two trees share game paths (the kit configs', `RefereeAppearance.bin`'s).
    prefox_referee_overrides: BTreeMap<&'static str, Vec<u8>>,
}

impl Templates {
    /// The embedded resources alone.
    pub(crate) fn embedded() -> Templates {
        Templates {
            overrides: BTreeMap::new(),
            fox_referee_overrides: BTreeMap::new(),
            prefox_referee_overrides: BTreeMap::new(),
        }
    }

    /// The resources with the overrides of `<data_dir>/templates/`, and one
    /// `template_override_active` per override read, in `RESOURCES` order, then those of the
    /// Fox referee tree, each a file of `templates/referees_fox/` at one of the tree's game
    /// paths, in the tree's order, then those of the pre-Fox tree, from
    /// `templates/referees_prefox/`, the same way. Nothing is read without a data directory or
    /// without the folder, and a file there naming no resource or no game path, exactly as it
    /// is spelled, is not read. An override that cannot be read, an override of one of the
    /// bins or of `DpFileList.bin` that does not parse as it (whatever the run's version), or a
    /// folder that cannot be listed, is the error. The other overrides' bytes are not checked:
    /// one that does not decode fails where the embedded resource would be used.
    pub(crate) fn read(data_dir: Option<&Path>) -> Result<(Templates, Vec<Message>), Unreadable> {
        let mut templates = Templates::embedded();
        let mut messages = Vec::new();
        let Some(data_dir) = data_dir else {
            return Ok((templates, messages));
        };
        let folder = data_dir.join(FOLDER_NAME);
        if !folder.is_dir() {
            return Ok((templates, messages));
        }
        let names = file_names(&folder)?;
        for resource in RESOURCES {
            if !names.contains(OsStr::new(resource.name)) {
                continue;
            }
            let path = folder.join(resource.name);
            let read = fs::read(&path)
                .map_err(anyhow::Error::from)
                .and_then(|bytes| resource.format.check(bytes));
            let bytes = match read {
                Ok(bytes) => bytes,
                Err(error) => return Err(Unreadable { path, error }),
            };
            messages.push(override_active(&path));
            templates.overrides.insert(resource.name, bytes);
        }
        for engine in [Engine::Fox, Engine::PreFox] {
            let (tree_folder, tree) = referee_tree_of(engine);
            if !names.contains(OsStr::new(tree_folder)) {
                continue;
            }
            let files = tree_files(&folder.join(tree_folder), tree)?;
            let overrides = match engine {
                Engine::Fox => &mut templates.fox_referee_overrides,
                Engine::PreFox => &mut templates.prefox_referee_overrides,
            };
            for (game_path, _) in tree {
                let Some(path) = files.get(game_path) else {
                    continue;
                };
                let bytes = fs::read(path).map_err(|error| Unreadable {
                    path: path.clone(),
                    error: error.into(),
                })?;
                messages.push(override_active(path));
                overrides.insert(game_path, bytes);
            }
        }
        Ok((templates, messages))
    }

    /// `resource`'s bytes: its override when one was read, else the embedded ones.
    fn bytes(&self, resource: &Resource) -> &[u8] {
        self.overrides
            .get(resource.name)
            .map_or(resource.embedded, Vec::as_slice)
    }

    /// The `TeamColor.bin` a compile sets its teams' colors in when it has no installed one to
    /// start from.
    pub(crate) fn team_color(&self) -> &[u8] {
        self.bytes(&TEAM_COLOR)
    }

    /// The `UniColor.bin` a compile sets its kits' menu colors in when it has no installed one
    /// to start from.
    pub(crate) fn uni_color(&self) -> &[u8] {
        self.bytes(&UNI_COLOR)
    }

    /// The `UniformParameter.bin` a compile adds its kit configs to when it has no installed
    /// one to start from: one base for PES 18, one for 19-21. `None` for the pre-Fox versions,
    /// which have no such bin.
    pub(crate) fn uniform_parameter_base(&self, version: PesVersion) -> Option<&[u8]> {
        match version {
            PesVersion::Pes18 => Some(self.bytes(&UNIFORM_PARAMETER_18)),
            PesVersion::Pes19 | PesVersion::Pes20 | PesVersion::Pes21 => {
                Some(self.bytes(&UNIFORM_PARAMETER_19))
            }
            PesVersion::Pes14 | PesVersion::Pes15 | PesVersion::Pes16 | PesVersion::Pes17 => None,
        }
    }

    /// The `kit` texture of a placeholder kit.
    pub(crate) fn placeholder_kit(&self) -> &[u8] {
        self.bytes(&PLACEHOLDER_KIT)
    }

    /// The `kit_mask` texture of a kit compiled for PES 2015 to 2017 whose effective textures
    /// hold none: a DDS, emitted as it is.
    pub(crate) fn kit_mask(&self) -> &[u8] {
        self.bytes(&KIT_MASK)
    }

    /// The skeleton packed beside a boots or hair model that brings none of its own.
    pub(crate) fn body_skeleton(&self) -> &[u8] {
        self.bytes(&BODY_SKELETON)
    }

    /// The `face_diff.bin` a face takes when its sources hold none: packed into a Fox face
    /// package, the `<dif>` of a pre-Fox `face.xml`.
    pub(crate) fn face_diff(&self) -> &[u8] {
        self.bytes(&FACE_DIFF)
    }

    /// The model a pre-Fox face packs as `oral_dummy_win32.model` when its `face.xml` lists
    /// no `face_neck` model.
    pub(crate) fn dummy_model(&self) -> &[u8] {
        self.bytes(&DUMMY_MODEL)
    }

    /// The `.mtl` packed beside the dummy model as `dummy.mtl`.
    pub(crate) fn dummy_mtl(&self) -> &[u8] {
        self.bytes(&DUMMY_MTL)
    }

    /// PES 17's stock `uniform.mtl`: the material set a `.model` collar is read with on both
    /// engines, its `.mtl` when it converts for PES 18-21 and its set for the same-engine
    /// pre-check on PES 15-17, since a collar carries no materials of its own and no export
    /// holds the game's shared set.
    pub(crate) fn uniform_mtl(&self) -> &[u8] {
        self.bytes(&UNIFORM_MTL)
    }

    /// The template environment map: the cubemap a PES 15-17 compile emits as `env.dds` in a
    /// player's texture home when a Fox metal material (`fox3ddf_ggx`) converted for him names
    /// no environment texture, unless his folder holds an `env` texture of its own. A DDS,
    /// emitted as it is.
    pub(crate) fn environment_map(&self) -> &[u8] {
        self.bytes(&ENVIRONMENT_MAP)
    }

    /// The `fcl_hair_sim.fclo` packed beside a `fcl_hair.fmdl` whose sources hold none.
    pub(crate) fn fcl_hair_sim(&self) -> &[u8] {
        self.bytes(&FCL_HAIR_SIM_FCLO)
    }

    /// The CPK file names of the official `DpFileList.bin`, in load order: the list a compile
    /// that deploys compares the installed one with.
    pub(crate) fn official_list(&self) -> Vec<String> {
        cpk::dp_file_list::read(self.official_list_file()).expect(
            "the official list reads: an override is read as a list when it is read (`read`), \
             and the embedded one is checked by a test",
        )
    }

    /// The official `DpFileList.bin` itself, which `upgrade-dpfl` installs byte for byte.
    pub(crate) fn official_list_file(&self) -> &[u8] {
        self.bytes(&DPFILELIST)
    }

    /// The empty CPK `upgrade-dpfl` writes for an official entry with no file.
    pub(crate) fn placeholder_cpk(&self) -> &[u8] {
        self.bytes(&PLACEHOLDER_CPK)
    }

    /// The Fox referee marker model, as bundled: always the embedded one.
    pub(crate) fn referee_marker(&self) -> &[u8] {
        REFEREE_MARKER
    }

    /// The files of `engine`'s referee template tree, in path order, each as its game path and
    /// its bytes: its override when one was read, else the embedded ones.
    pub(crate) fn referee_tree(
        &self,
        engine: Engine,
    ) -> impl Iterator<Item = (&'static str, &[u8])> {
        let (_, tree) = referee_tree_of(engine);
        let overrides = match engine {
            Engine::Fox => &self.fox_referee_overrides,
            Engine::PreFox => &self.prefox_referee_overrides,
        };
        tree.iter().map(move |(path, embedded)| {
            let bytes = overrides.get(path).map_or(*embedded, Vec::as_slice);
            (*path, bytes)
        })
    }
}

/// A command's resources (`Templates::read`), read before anything else, their findings
/// reported first through `events`. A `templates/` file that cannot be read is
/// `template_override_unreadable`, Fatal, and `None`: the command stops before it reads or
/// writes anything else, whatever the resource, because the file was put there on purpose.
pub(crate) fn read_reported(ctx: &ToolContext, events: &mut RunEvents) -> Option<Templates> {
    match Templates::read(ctx.paths().data_dir.as_deref()) {
        Ok((templates, messages)) => {
            for message in messages {
                events.message(message);
            }
            Some(templates)
        }
        Err(unreadable) => {
            events.message(tool_message(
                Code::TemplateOverrideUnreadable,
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

/// The names of the entries of `folder`, as they are spelled: a Windows file system would open
/// `unicolor.bin` for `UniColor.bin`, so a resource's file is found in the listing, by its
/// exact name, rather than by opening its path. A folder that cannot be listed is the error.
fn file_names(folder: &Path) -> Result<BTreeSet<OsString>, Unreadable> {
    let unreadable = |error: std::io::Error| Unreadable {
        path: folder.to_owned(),
        error: error.into(),
    };
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(folder).map_err(unreadable)? {
        names.insert(entry.map_err(unreadable)?.file_name());
    }
    Ok(names)
}

/// The files below `folder` whose path there is a game path of `tree`, by that path: the path
/// is the listing's names joined with `/`, so, as in `file_names`, a path spelled in another
/// letter case is not the tree's. An entry at a game path is taken whatever it is, so a folder
/// there fails when it is read, as a folder at a resource's name does. A folder that cannot be
/// listed is the error.
fn tree_files(
    folder: &Path,
    tree: &[(&'static str, &[u8])],
) -> Result<BTreeMap<&'static str, PathBuf>, Unreadable> {
    let mut files = BTreeMap::new();
    let mut folders = vec![(folder.to_owned(), String::new())];
    while let Some((current, prefix)) = folders.pop() {
        let unreadable = |error: std::io::Error| Unreadable {
            path: current.clone(),
            error: error.into(),
        };
        for entry in fs::read_dir(&current).map_err(unreadable)? {
            let entry = entry.map_err(unreadable)?;
            let name = entry.file_name();
            // A name that is not Unicode is in no game path.
            let Some(name) = name.to_str() else {
                continue;
            };
            let relative = format!("{prefix}{name}");
            if let Some((game_path, _)) = tree.iter().find(|(path, _)| *path == relative) {
                files.insert(*game_path, entry.path());
            } else if entry.file_type().map_err(unreadable)?.is_dir() {
                folders.push((entry.path(), format!("{relative}/")));
            }
        }
    }
    Ok(files)
}

/// The `template_override_active` finding for the override file `path`.
fn override_active(path: &Path) -> Message {
    tool_message(
        Code::TemplateOverrideActive,
        Scope::Run,
        Disposition::Keep,
        vec![("path", path.display().to_string())],
    )
}

#[cfg(test)]
mod tests {
    use studio_core::Severity;

    use super::*;
    use crate::testing::{dpfilelist, scratch};

    #[test]
    fn pes_18_has_its_own_base_and_19_to_21_share_one() {
        let templates = Templates::embedded();
        assert_eq!(
            templates.uniform_parameter_base(PesVersion::Pes18),
            Some(UNIFORM_PARAMETER_18.embedded)
        );
        for version in [PesVersion::Pes19, PesVersion::Pes20, PesVersion::Pes21] {
            assert_eq!(
                templates.uniform_parameter_base(version),
                Some(UNIFORM_PARAMETER_19.embedded)
            );
        }
        for version in [PesVersion::Pes15, PesVersion::Pes16, PesVersion::Pes17] {
            assert_eq!(templates.uniform_parameter_base(version), None);
        }
        assert_ne!(UNIFORM_PARAMETER_18.embedded, UNIFORM_PARAMETER_19.embedded);
    }

    #[test]
    fn the_kit_mask_template_is_a_flat_64_square_dxt1_dds_of_2872_bytes() {
        let mask = Templates::embedded().kit_mask().to_vec();
        assert_eq!(mask.len(), 2_872);
        assert!(mask.starts_with(b"DDS "));
        assert_eq!(&mask[84..88], b"DXT1");
        // The first two level-0 blocks: color0 (255, 190, 0), color1 (98, 101, 0), every
        // index 3.
        let block = [0xe0, 0xfd, 0x20, 0x63, 0xff, 0xff, 0xff, 0xff];
        assert_eq!(mask[128..144], [block, block].concat());
    }

    #[test]
    fn the_uniform_mtl_template_defines_both_collar_materials() {
        let set =
            pes_model::format::mtl::MaterialSet::read(Templates::embedded().uniform_mtl()).unwrap();
        let names: Vec<&str> = set
            .materials
            .iter()
            .map(|material| material.name.as_str())
            .collect();
        assert!(
            names.contains(&"uni_collar") && names.contains(&"uni_shirts"),
            "{names:?}"
        );
    }

    #[test]
    fn every_resource_has_a_name_of_its_own() {
        let names: BTreeSet<&str> = RESOURCES.iter().map(|resource| resource.name).collect();
        assert_eq!(names.len(), RESOURCES.len());
    }

    /// Each resource's bytes in `templates`, through the accessors, in `RESOURCES` order.
    fn every_resource(templates: &Templates) -> [&[u8]; 15] {
        [
            templates.team_color(),
            templates.uni_color(),
            templates.uniform_parameter_base(PesVersion::Pes18).unwrap(),
            templates.uniform_parameter_base(PesVersion::Pes21).unwrap(),
            templates.placeholder_kit(),
            templates.kit_mask(),
            templates.body_skeleton(),
            templates.face_diff(),
            templates.fcl_hair_sim(),
            templates.dummy_model(),
            templates.dummy_mtl(),
            templates.official_list_file(),
            templates.placeholder_cpk(),
            templates.environment_map(),
            templates.uniform_mtl(),
        ]
    }

    /// Asserts that `templates` holds the embedded bytes of every resource.
    fn assert_embedded(templates: &Templates) {
        let embedded = RESOURCES.map(|resource| resource.embedded);
        assert!(
            every_resource(templates) == embedded,
            "every resource embedded"
        );
    }

    #[test]
    fn no_data_directory_or_no_folder_gives_the_embedded_resources_and_no_finding() {
        let temp = scratch("templates_none");
        let (templates, messages) = Templates::read(None).unwrap();
        assert_embedded(&templates);
        assert_eq!(messages, []);

        let (templates, messages) = Templates::read(Some(temp.path())).unwrap();
        assert_embedded(&templates);
        assert_eq!(messages, []);
    }

    #[test]
    fn each_file_naming_a_resource_replaces_it_and_is_reported_in_the_table_s_order() {
        let temp = scratch("templates_read");
        let folder = temp.path().join("templates");
        fs::create_dir(&folder).unwrap();
        // Written in the other order, so the findings' order is the table's, not the folder's.
        fs::write(folder.join("placeholder.cpk"), b"placeholder override").unwrap();
        fs::write(folder.join("face_diff.bin"), b"face diff override").unwrap();
        fs::write(folder.join("dummy.mtl"), b"dummy material override").unwrap();
        // One `UniColor.bin` record, which parses as the bin.
        let kit_colors = [1; 85];
        fs::write(folder.join("UniColor.bin"), kit_colors).unwrap();

        let (templates, messages) = Templates::read(Some(temp.path())).unwrap();

        let mut expected: [&[u8]; 15] = RESOURCES.map(|resource| resource.embedded);
        expected[1] = &kit_colors;
        expected[7] = b"face diff override";
        expected[10] = b"dummy material override";
        expected[12] = b"placeholder override";
        assert!(every_resource(&templates) == expected);
        let active = |name: &str| {
            tool_message(
                Code::TemplateOverrideActive,
                Scope::Run,
                Disposition::Keep,
                vec![("path", folder.join(name).display().to_string())],
            )
        };
        assert_eq!(
            messages,
            [
                active("UniColor.bin"),
                active("face_diff.bin"),
                active("dummy.mtl"),
                active("placeholder.cpk")
            ]
        );
        assert_eq!(messages[0].code.code, "template_override_active");
        assert_eq!(messages[0].severity, Severity::Info);
    }

    #[test]
    fn a_file_naming_no_resource_as_it_is_spelled_is_not_read() {
        let temp = scratch("templates_other_names");
        let folder = temp.path().join("templates");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("notes.txt"), b"notes").unwrap();
        // `UniColor.bin` in another letter case, which a Windows file system would open as it.
        fs::write(folder.join("unicolor.bin"), b"kit colors override").unwrap();

        let (templates, messages) = Templates::read(Some(temp.path())).unwrap();

        assert_embedded(&templates);
        assert_eq!(messages, []);
    }

    #[test]
    fn an_override_that_cannot_be_read_is_the_error_naming_it() {
        let temp = scratch("templates_unreadable");
        let folder = temp.path().join("templates");
        // Reading a folder fails on every system.
        let team_color = folder.join("TeamColor.bin");
        fs::create_dir_all(&team_color).unwrap();

        let Err(unreadable) = Templates::read(Some(temp.path())) else {
            panic!("an override that cannot be read must be the error, not an absent one");
        };
        assert_eq!(unreadable.path, team_color);
    }

    #[test]
    fn a_bin_override_that_does_not_parse_is_the_error_naming_it() {
        let temp = scratch("templates_unparsable");
        let folder = temp.path().join("templates");
        fs::create_dir(&folder).unwrap();
        // One byte short of a whole 85-byte record.
        fs::write(folder.join("UniColor.bin"), [0; 84]).unwrap();

        let Err(unreadable) = Templates::read(Some(temp.path())) else {
            panic!("a bin override that does not parse must be the error");
        };
        assert_eq!(unreadable.path, folder.join("UniColor.bin"));
        assert_eq!(
            unreadable.error.to_string(),
            "UniColor.bin is 84 bytes, not a whole number of 85-byte records"
        );
    }

    #[test]
    fn every_bin_override_is_parsed_whatever_the_run_s_version() {
        for name in [
            "TeamColor.bin",
            "UniColor.bin",
            "UniformParameter18.bin",
            "UniformParameter19.bin",
        ] {
            let temp = scratch(&format!("templates_unparsable_{name}"));
            let folder = temp.path().join("templates");
            fs::create_dir(&folder).unwrap();
            // Seven bytes: no whole record of either color bin, and shorter than the eight
            // bytes of a `UniformParameter.bin`'s header.
            fs::write(folder.join(name), [0; 7]).unwrap();

            let Err(unreadable) = Templates::read(Some(temp.path())) else {
                panic!("{name}: a bin override that does not parse must be the error");
            };
            assert_eq!(unreadable.path, folder.join(name));
        }
    }

    #[test]
    fn the_embedded_official_list_is_the_text_list_s_53_entries_in_order() {
        let text = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../resources/templates/DpFileList.txt"),
        )
        .unwrap();
        let expected: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();

        let official = Templates::embedded().official_list();

        assert_eq!(official.len(), 53);
        assert_eq!(official[0], "4cc_01_db.cpk");
        assert_eq!(official[52], "4cc_99_test.cpk");
        assert_eq!(official, expected);
    }

    #[test]
    fn a_list_override_that_does_not_read_as_a_list_is_the_error_naming_it() {
        let temp = scratch("templates_list_unreadable");
        let folder = temp.path().join("templates");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("DpFileList.bin"), [0; 15]).unwrap();

        let Err(unreadable) = Templates::read(Some(temp.path())) else {
            panic!("a list override that does not read as a list must be the error");
        };
        assert_eq!(unreadable.path, folder.join("DpFileList.bin"));
        assert_eq!(
            unreadable.error.to_string(),
            "the list is 15 bytes, shorter than its 16-byte header"
        );
    }

    /// Every file below `folder`, by its path there spelled with `/`, with its bytes.
    fn files_below(folder: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut files = BTreeMap::new();
        let mut folders = vec![folder.to_owned()];
        while let Some(current) = folders.pop() {
            for entry in fs::read_dir(&current).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    folders.push(path);
                    continue;
                }
                let relative: Vec<&str> = path
                    .strip_prefix(folder)
                    .unwrap()
                    .components()
                    .map(|part| part.as_os_str().to_str().unwrap())
                    .collect();
                files.insert(relative.join("/"), fs::read(&path).unwrap());
            }
        }
        files
    }

    /// `engine`'s referee tree of `templates`, by game path, in the accessor's order.
    fn referee_tree(templates: &Templates, engine: Engine) -> Vec<(&'static str, &[u8])> {
        templates.referee_tree(engine).collect()
    }

    #[test]
    fn each_embedded_referee_tree_is_its_resource_folder_s_files_in_path_order() {
        for (engine, folder, count) in [
            (Engine::Fox, "referees_fox", 31),
            (Engine::PreFox, "referees_prefox", 51),
        ] {
            let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../resources/templates")
                .join(folder);
            let files = files_below(&folder);
            assert_eq!(files.len(), count, "{engine:?}");

            let templates = Templates::embedded();
            let embedded = referee_tree(&templates, engine);

            let paths: Vec<&str> = embedded.iter().map(|(path, _)| *path).collect();
            let expected: Vec<&str> = files.keys().map(String::as_str).collect();
            assert_eq!(
                paths, expected,
                "{engine:?}: the folder's files, in path order"
            );
            for (path, bytes) in embedded {
                assert!(bytes == files[path].as_slice(), "{engine:?}: {path}");
            }
        }
    }

    #[test]
    fn the_referee_common_paths_are_the_pre_fox_refkit_files_and_none_on_fox() {
        assert_eq!(
            referee_common_paths(Engine::PreFox),
            [
                "refkit/boots_c.dds",
                "refkit/boots_n.dds",
                "refkit/boots_sr.dds",
                "refkit/oral_arm_win32.model",
                "refkit/oral_boots_win32.model",
                "refkit/oral_hand_l_win32.model",
                "refkit/oral_hand_r_win32.model",
                "refkit/oral_pants_sub_win32.model",
                "refkit/oral_pants_win32.model",
                "refkit/oral_refshirt_win32.model",
                "refkit/oral_sleeve_win32.model",
                "refkit/oral_socks_win32.model",
                "refkit/oral_thigh_win32.model",
                "refkit/refkit.mtl",
                "refkit/skin_color.dds",
                "refkit/texture.dds",
            ]
        );
        // The Fox tree holds no file under any `common/999/`.
        assert!(
            REFEREES_FOX
                .iter()
                .all(|(path, _)| !path.contains("/common/999/"))
        );
        assert_eq!(referee_common_paths(Engine::Fox), Vec::<String>::new());
    }

    #[test]
    fn the_prop_texture_path_names_a_file_of_the_pre_fox_tree() {
        let path = crate::paths::REFEREE_PROP_TEXTURE;
        let (_, embedded) = REFEREES_PREFOX
            .iter()
            .find(|(tree_path, _)| *tree_path == path)
            .unwrap();
        let templates = Templates::embedded();
        let (_, bytes) = templates
            .referee_tree(Engine::PreFox)
            .find(|(tree_path, _)| *tree_path == path)
            .unwrap();
        assert!(bytes == *embedded);
    }

    /// The game path of one of the kit configs, which both referee trees hold.
    const REFEREE_CL_1: &str =
        "common/character0/model/character/uniform/team/referee/referee_CL_1.bin";

    #[test]
    fn a_file_at_a_referee_tree_path_replaces_its_tree_s_entry_alone_reported_after_the_others() {
        let temp = scratch("templates_referee_override");
        let folder = temp.path().join("templates");
        // The same game path in both trees' folders, each with bytes of its own, so an
        // override of one tree showing in the other fails.
        let files = [
            (Engine::Fox, "referees_fox", &b"Fox kit config override"[..]),
            (
                Engine::PreFox,
                "referees_prefox",
                &b"pre-Fox kit config override"[..],
            ),
        ]
        .map(|(engine, tree_folder, bytes)| {
            // Joined part by part, so the path is spelled as the folder's listing gives it.
            let file = REFEREE_CL_1
                .split('/')
                .fold(folder.join(tree_folder), |path, part| path.join(part));
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, bytes).unwrap();
            (engine, file, bytes)
        });
        fs::write(folder.join("face_diff.bin"), b"face diff override").unwrap();

        let (templates, messages) = Templates::read(Some(temp.path())).unwrap();

        for (engine, _, override_bytes) in &files {
            let (_, embedded_tree) = referee_tree_of(*engine);
            let tree = referee_tree(&templates, *engine);
            assert_eq!(tree.len(), embedded_tree.len());
            // Compared one by one with `assert!`: a failing `assert_eq!` would print megabytes.
            for ((path, bytes), (embedded_path, embedded_bytes)) in
                tree.into_iter().zip(embedded_tree)
            {
                assert_eq!(path, *embedded_path);
                let expected = match path {
                    REFEREE_CL_1 => *override_bytes,
                    _ => *embedded_bytes,
                };
                assert!(bytes == expected, "{engine:?}: {path}");
            }
        }
        let active = |path: &Path| {
            tool_message(
                Code::TemplateOverrideActive,
                Scope::Run,
                Disposition::Keep,
                vec![("path", path.display().to_string())],
            )
        };
        let [(_, fox_file, _), (_, prefox_file, _)] = &files;
        assert_eq!(
            messages,
            [
                active(&folder.join("face_diff.bin")),
                active(fox_file),
                active(prefox_file)
            ],
            "the trees' after the other resources', Fox's first"
        );
    }

    #[test]
    fn a_file_naming_no_referee_tree_path_as_it_is_spelled_is_not_read() {
        for (engine, tree_folder) in [
            (Engine::Fox, "referees_fox"),
            (Engine::PreFox, "referees_prefox"),
        ] {
            let temp = scratch(&format!("templates_referee_other_names_{tree_folder}"));
            let tree = temp.path().join("templates").join(tree_folder);
            let referee = "common/character0/model/character/uniform/team/referee";
            for path in [
                format!("{referee}/referee_XYZ_1.bin"),
                // The tree's file in another letter case, which a Windows file system would
                // open as it.
                format!("{referee}/referee_cl_1.bin"),
                "notes.txt".to_owned(),
            ] {
                let file = tree.join(path);
                fs::create_dir_all(file.parent().unwrap()).unwrap();
                fs::write(file, b"not the tree's").unwrap();
            }

            let (templates, messages) = Templates::read(Some(temp.path())).unwrap();

            assert!(
                referee_tree(&templates, engine) == referee_tree(&Templates::embedded(), engine),
                "{engine:?}"
            );
            assert_eq!(messages, [], "{engine:?}");
        }
    }

    #[test]
    fn a_referee_tree_folder_that_cannot_be_listed_is_the_error_naming_it() {
        for tree_folder in ["referees_fox", "referees_prefox"] {
            let temp = scratch(&format!("templates_referee_unlisted_{tree_folder}"));
            let folder = temp.path().join("templates");
            fs::create_dir(&folder).unwrap();
            // A file where the folder goes: listing it fails on every system.
            fs::write(folder.join(tree_folder), b"not a folder").unwrap();

            let Err(unreadable) = Templates::read(Some(temp.path())) else {
                panic!("{tree_folder}: a tree folder that cannot be listed must be the error");
            };
            assert_eq!(unreadable.path, folder.join(tree_folder));
        }
    }

    #[test]
    fn a_list_override_replaces_the_official_list() {
        let temp = scratch("templates_list_override");
        let folder = temp.path().join("templates");
        fs::create_dir(&folder).unwrap();
        let names = ["4cc_08_bins.cpk", "4cc_61_midcup.cpk", "4cc_99_test.cpk"];
        fs::write(folder.join("DpFileList.bin"), dpfilelist(&names)).unwrap();

        let (templates, messages) = Templates::read(Some(temp.path())).unwrap();

        assert_eq!(templates.official_list(), names);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].code.code, "template_override_active");
    }
}
