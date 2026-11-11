//! The deep pass's texture lookup for a pre-Fox `.mtl`: each texture path a `.mtl` names must
//! be supplied, by the stem rule the face task points the paths with
//! (`processing::prefox_face`). On Fox, which reads only a `.mtl` a selected `.model` pairs
//! with, the Models task points them (`processing::model`). So a path this calls supplied is
//! one the task points, and a path it calls missing is one the task leaves as written.
//!
//! A missing texture of a material a paired model's mesh binds is `mtl_texture_not_found`, a
//! Warning that keeps what holds the `.mtl`: which samplers a pre-Fox shader reads is not
//! known, and the game plays faces with such a miss. One only materials no mesh binds name is
//! `mtl_texture_unused_missing`, an Info, since the game never loads it.

use std::collections::BTreeSet;

use aesthetics_export::{
    ContentFinding, Disposition, FileDescriptor, FileKind, IssueScope, SharedKind,
    SharedModelFolder, legacy_kit_tokens,
};
use pes_version::Engine;
use vtree::ScopePath;

use super::model::MaterialRead;
use crate::messages::Code;
use crate::plan::roles::{
    FolderModels, PlayerFile, below_common, common_folder, file_stem, player_file,
};
use crate::texture_lookup::{self, TextureFolders, TexturePlace};
use crate::user_face_xml::{Reference, reference};

/// Where the textures a `.mtl`'s paths name may come from, each by its folded stem.
pub(super) struct TextureSources<'a> {
    /// The places a texture name of the `.mtl` resolves in, nearest first
    /// (`HeldTextures::of`): its model folder's textures from the `.mtl`'s own folder up,
    /// those of the shared folders its packages are built from at each folder's own path,
    /// and the stems its texture links stand for; for a
    /// `Common/` `.mtl`, `Common/`'s.
    pub(super) held: &'a [&'a TexturePlace],
    /// For a path below the `.mtl`'s folder (`texture_lookup::path_below`), which resolves at
    /// that path alone: its model folder's textures and the `.mtl`'s path (`HeldTextures::at`).
    pub(super) below: (&'a HeldTextures, &'a ScopePath),
    /// The stems of the textures in `Common/` the pass keeps, which the export's Common
    /// textures task packs into the team's Common output.
    pub(super) common: &'a BTreeSet<String>,
    /// The paths below `Common/` without their extensions, folded (`jessie/skin`), of the
    /// `Common/` textures the pass keeps and the texture links of the folder the pass is on,
    /// which a Common path below a subfolder resolves at (`supply`), with `installed`
    /// answering for the installed CPKs'.
    pub(super) common_below: &'a BTreeSet<String>,
    /// The stems the installed CPKs listed before the run's hold in the team's Common output;
    /// `None` when that lookup cannot be made or the export has no team ID, which holds
    /// nothing.
    pub(super) installed: Option<&'a BTreeSet<String>>,
}

/// Whether the texture a `.mtl` path names is supplied (`supply`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Supply {
    /// Supplied, or not looked for: the game's own texture, or a `dummy_` one it substitutes.
    Supplied,
    /// Nobody supplies it: neither the folder, nor for a Common path `Common/` or an installed
    /// CPK loaded before the run's.
    Missing,
}

/// Whether the texture `path`, a `.mtl` sampler's path as written, is supplied from `sources`.
/// A `dummy_` stem or a kit stand-in (`team_kit*`, `team_gk_kit*`), which the exes
/// substitute, is never looked for. A path below the `.mtl`'s folder
/// (`./shorts/y.dds`, `texture_lookup::path_below`) is supplied when the place that path names
/// holds its stem (folded; the places file each kit set's member under its set's `kitN`
/// key, so a `kitN` reference finds a member and a `kitF` path none), and missing
/// otherwise: it resolves there alone. Whatever directory any other
/// path spells, it is supplied when its stem, folded, is one a place of `sources.held`
/// holds: that is the stem the face task points at the folder's textures. Past the folder, a
/// `./` path and a
/// bare name (read as `./`) are missing: the face packs no texture the folder does not hold. A
/// path into the team's uniform Common folder
/// (`team_common_folder/<name>` or `model/character/uniform/common/team_folder/<name>`,
/// `<name>` a name or a `Common/` subfolder's
/// `jessie/skin.dds`) is supplied when `Common/` holds the stem at that path
/// (`sources.common` for a direct one, `sources.common_below` for a subfolder's, which
/// resolves there alone), or an installed CPK holds it there; an installed lookup that
/// cannot be made holds nothing. Any other path
/// names the game's own files, which are not looked in.
fn supply(path: &str, sources: &TextureSources) -> Supply {
    let (directory, file_name) = texture_lookup::split(path);
    let stem = file_stem(file_name);
    let key = vtree::fold_name(stem);
    let holds = |stems: &BTreeSet<String>| stems.contains(&key);
    let place_holds = |place: &TexturePlace| place.contains_key(&key);
    // A `dummy_` stem and a kit stand-in (`team_kit*`, `team_gk_kit*`) are the game's to
    // substitute: never looked for.
    if key.starts_with("dummy_") || crate::paths::kit_stand_in_game_name(file_name).is_some() {
        return Supply::Supplied;
    }
    if let Some(subdirectory) = texture_lookup::path_below(directory) {
        let (held, mtl) = sources.below;
        return if held
            .at(mtl, subdirectory)
            .iter()
            .any(|place| place_holds(place))
        {
            Supply::Supplied
        } else {
            Supply::Missing
        };
    }
    if sources.held.iter().any(|place| place_holds(place)) {
        return Supply::Supplied;
    }
    if !path.contains('/') {
        return Supply::Missing;
    }
    match reference(path) {
        Reference::Local(_) => Supply::Missing,
        // A Common path into a `Common/` subfolder resolves at its path below `Common/`
        // among the Common textures the export packs there (`sources.common_below`), then
        // the installed CPKs'.
        Reference::Common { file_name } if file_name.contains('/') => {
            let below = vtree::fold_name(file_stem(&file_name));
            let holds_below = |stems: &BTreeSet<String>| stems.contains(&below);
            if holds_below(sources.common_below) {
                Supply::Supplied
            } else {
                match sources.installed {
                    Some(installed) if holds_below(installed) => Supply::Supplied,
                    _ => Supply::Missing,
                }
            }
        }
        Reference::Common { .. } if holds(sources.common) => Supply::Supplied,
        Reference::Common { .. } if sources.installed.is_some_and(holds) => Supply::Supplied,
        Reference::Common { .. } => Supply::Missing,
        // The game's own file directly below `common_folder/`, and any other form: not
        // looked up.
        // A refused path never reaches here (`texture_findings` reports it first).
        Reference::CommonInvalid | Reference::GameCommon(_) | Reference::Unchecked(_) => {
            Supply::Supplied
        }
    }
}

/// Whether the `.mtl` or `face.xml` path `value` names a file of the export: its folder's own
/// (`./`) or one of the team's `Common/`. A path into the game's own folders is the game's
/// spelling, which the game keeps for its kit textures (`u0???pN`), so `kit_token_old`
/// checks only these.
pub(super) fn names_export_file(value: &str) -> bool {
    match reference(value) {
        Reference::Local(_) | Reference::Common { .. } => true,
        Reference::CommonInvalid | Reference::GameCommon(_) | Reference::Unchecked(_) => false,
    }
}

/// `mtl_texture_not_found` and `mtl_texture_unused_missing` on `scope` for the `.mtl` whose
/// `materials` are read, which the findings name `name`: one finding per distinct texture path
/// (compared as written, in the order the `.mtl` first names it) that `sources` do not supply,
/// naming the file, the path as written and the materials naming it, in the `.mtl`'s order.
/// When one of those materials is among `used`, the names the meshes of the models paired with
/// the `.mtl` bind, it is `mtl_texture_not_found`, else `mtl_texture_unused_missing`; both keep
/// what holds the `.mtl`. A Common path naming a folder other than `team_folder`, a
/// folder after `common_folder/`, or one
/// starting `common/` (a CPK's root folder, which no reference names), is
/// `mtl_common_path_invalid` instead, which discards
/// what holds the `.mtl` (`xml_common_path_invalid`'s rule for a `.mtl`, on every target:
/// the spelling is the export's, so `check` and a compile for either engine judge it
/// alike). A path naming the kit stand-in's old spelling (a stem that, folded, is
/// `dummy_kit` or `dummy_gk_kit`, alone or `_`-suffixed, whatever its directory) is
/// `dummy_kit_name_old`, discarding the same; one path gets both findings when it trips
/// both rules. A path naming a file of the export (`names_export_file`) and holding a
/// legacy kit token (`aesthetics_export::legacy_kit_tokens`, the path as written) is
/// `kit_token_old`, discarding the same, beside any other.
/// `scope` is a model folder's
/// `Folder` (the `.mtl` of a player or shared folder's tree, whose finding drops the folder)
/// or a `Common/` `.mtl`'s `File` (whose finding drops the file alone).
pub(super) fn texture_findings(
    name: &str,
    materials: &[MaterialRead],
    used: &BTreeSet<&str>,
    sources: &TextureSources,
    scope: &IssueScope,
) -> Vec<ContentFinding> {
    // Each distinct path with the materials naming it, in the order the `.mtl` names them.
    let mut named: Vec<(&str, Vec<&str>)> = Vec::new();
    for material in materials {
        for path in &material.paths {
            let index = match named.iter().position(|(known, _)| known == path) {
                Some(index) => index,
                None => {
                    named.push((path, Vec::new()));
                    named.len() - 1
                }
            };
            let naming = &mut named[index].1;
            if !naming.contains(&material.name.as_str()) {
                naming.push(&material.name);
            }
        }
    }
    named
        .into_iter()
        .flat_map(|(path, naming)| {
            // A `Common/` `.mtl` is a file of `Common/`; a folder's takes the folder.
            // The call's other scopes do not reach here.
            let drop = match scope {
                IssueScope::File(_) => Disposition::DropFile,
                IssueScope::Folder(_) | IssueScope::Export | IssueScope::RosterEntry { .. } => {
                    Disposition::DropFolder
                }
            };
            let mut findings = Vec::new();
            if let Reference::CommonInvalid = reference(path) {
                findings.push(ContentFinding {
                    code: Code::MtlCommonPathInvalid.as_str(),
                    scope: scope.clone(),
                    context: vec![
                        ("file", name.to_owned()),
                        ("attribute", "path".to_owned()),
                        ("value", path.to_owned()),
                    ],
                    disposition: drop,
                    pass_through_eligible: false,
                });
            }
            // The stand-in's old spelling, refused on the spelling as written: a path
            // that is also `CommonInvalid` gets both findings, each naming its own fix.
            if crate::paths::is_old_kit_stand_in(texture_lookup::split(path).1) {
                findings.push(ContentFinding {
                    code: Code::DummyKitNameOld.as_str(),
                    scope: scope.clone(),
                    context: vec![("file", name.to_owned()), ("texture", path.to_owned())],
                    disposition: drop,
                    pass_through_eligible: false,
                });
            }
            if names_export_file(path) && !legacy_kit_tokens(path).is_empty() {
                findings.push(ContentFinding {
                    code: Code::KitTokenOld.as_str(),
                    scope: scope.clone(),
                    context: vec![("file", name.to_owned()), ("reference", path.to_owned())],
                    disposition: drop,
                    pass_through_eligible: false,
                });
            }
            if !findings.is_empty() || supply(path, sources) != Supply::Missing {
                return findings;
            }
            let code = if naming.iter().any(|material| used.contains(material)) {
                Code::MtlTextureNotFound
            } else {
                Code::MtlTextureUnusedMissing
            };
            findings.push(ContentFinding {
                code: code.as_str(),
                scope: scope.clone(),
                context: vec![
                    ("file", name.to_owned()),
                    ("texture", path.to_owned()),
                    ("materials", naming.join(", ")),
                ],
                disposition: Disposition::Keep,
                pass_through_eligible: false,
            });
            findings
        })
        .collect()
}

/// The textures a model folder's `.mtl` paths may name, as the face task collects them
/// (`processing::prefox_face::face`), built once per folder (`held_textures`) and sliced per
/// `.mtl` (`of`).
pub(super) struct HeldTextures {
    /// The model folder.
    folder: ScopePath,
    /// Its textures (`PlayerFile::Texture`) by the folder of its tree holding each, those of
    /// the shared folders its packages are built from at each file's path below its folder,
    /// and their texture links the same way
    /// (`PlayerFile::CommonTexture`): a link counts as the texture it stands for being present
    /// in the folder holding it.
    textures: TextureFolders,
}

impl HeldTextures {
    /// The places a texture name of the `.mtl` at `mtl` resolves in, nearest first from its
    /// own folder (`texture_lookup`): each folder's textures, then its links.
    pub(super) fn of(&self, mtl: &ScopePath) -> Vec<&TexturePlace> {
        self.textures
            .nearest_first(&self.folder, mtl)
            .into_iter()
            .map(|(place, _)| place)
            .collect()
    }

    /// The places a path of the `.mtl` at `mtl` naming `subdirectory` below its folder
    /// resolves in alone (`TextureFolders::at`).
    pub(super) fn at(&self, mtl: &ScopePath, subdirectory: &str) -> Vec<&TexturePlace> {
        self.textures
            .at(&self.folder, mtl, subdirectory)
            .into_iter()
            .map(|(place, _)| place)
            .collect()
    }

    /// Adds the texture `.common` link standing for the `Common/` texture at `below`, its
    /// path below `Common/` without its extension (`PlayerFile::CommonTexture`): it counts
    /// as the texture present in the directory `below` sits in, as the Models task reads it.
    pub(super) fn insert_link(&mut self, below: &str) {
        self.textures.insert_link(true, below);
    }
}

/// The textures the model folder at `folder` holds for its `.mtl` paths: each of its `files`
/// and of the files of the `shared` folders its packages are built from that is a texture
/// (`PlayerFile::Texture`), and the `Common/` path each texture link among them stands for
/// (`PlayerFile::CommonTexture`; the link's own spelling, `Common/`'s being read only at task
/// time, and only the stems being looked up here); `models` are the folder's, read for a
/// target of `engine`.
pub(super) fn held_textures(
    folder: &ScopePath,
    files: &[FileDescriptor],
    models: &FolderModels,
    shared: &[(SharedKind, &SharedModelFolder)],
    engine: Engine,
) -> HeldTextures {
    let mut held = HeldTextures {
        folder: folder.clone(),
        textures: TextureFolders::default(),
    };
    // Each role with whether it is one of the folder's own files.
    let mut roles: Vec<(bool, PlayerFile)> = files
        .iter()
        .filter_map(|file| player_file(folder, file, models))
        .map(|role| (true, role))
        .collect();
    for (kind, source) in shared {
        let source_models = FolderModels::of_shared(&source.path, &source.files, *kind, engine);
        roles.extend(
            source
                .files
                .iter()
                .filter_map(|file| player_file(&source.path, file, &source_models))
                .map(|role| (false, role)),
        );
    }
    for (own, role) in roles {
        match role {
            PlayerFile::Texture { below, .. } => held.textures.insert(own, &below),
            PlayerFile::CommonTexture(below) => held.textures.insert_link(own, &below),
            PlayerFile::Model { .. }
            | PlayerFile::CommonModel { .. }
            | PlayerFile::Packed { .. }
            | PlayerFile::FaceDiffXml
            | PlayerFile::Skeleton { .. }
            | PlayerFile::SlotlessSkeleton
            | PlayerFile::LeftOutKitVariant
            | PlayerFile::PreFoxModel { .. }
            | PlayerFile::PreFoxPart { .. }
            | PlayerFile::Material
            | PlayerFile::PreFoxCommonModel { .. }
            | PlayerFile::CommonMaterial
            | PlayerFile::FaceXml
            | PlayerFile::UnusedFaceFile
            | PlayerFile::ConversionSkeleton
            | PlayerFile::UnsupportedGltf => {}
        }
    }
    held
}

/// The textures `Common/`'s `.mtl` paths may name (`common_mtl_findings`): the kept
/// `Common/` textures, each its folder's own at its path below `Common/` without its
/// extension (`below_common`, `file_stem`), `Common/` itself the model folder a name
/// resolves nearest first from and a `.mtl`'s path below its directory resolves at.
pub(super) fn held_common(kept: &[FileDescriptor]) -> HeldTextures {
    let mut textures = TextureFolders::default();
    for file in kept {
        if file.kind == FileKind::Texture {
            textures.insert(true, file_stem(below_common(&file.path)));
        }
    }
    HeldTextures {
        folder: common_folder(),
        textures,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::kit_variants::kit_lookup_keys;

    /// A place holding the textures `stems`, each directly in its folder, filed as
    /// `TextureFolders::insert` files them.
    fn place(stems: &[&str]) -> TexturePlace {
        let mut folders = TextureFolders::default();
        for stem in stems {
            folders.insert(true, stem);
        }
        folders
            .folder("")
            .into_iter()
            .next()
            .map_or_else(TexturePlace::default, |(place, _)| place.clone())
    }

    /// What `supply` makes of `path` with the folder holding `held`, `Common/` `common` and the
    /// installed CPKs `installed`, nothing at any path below `Common/`. `common` is keyed as
    /// the pass files it (`kit_lookup_keys`).
    fn supplied(path: &str, held: &[&str], common: &[&str], installed: Option<&[&str]>) -> Supply {
        let installed = installed.map(|stems| {
            stems
                .iter()
                .flat_map(|stem| kit_lookup_keys(stem))
                .collect()
        });
        let folders = slot_05(&[], &[]);
        let mtl = ScopePath::new("Players/05 - A/x.mtl").unwrap();
        let common_below = BTreeSet::new();
        let common: BTreeSet<String> = common
            .iter()
            .flat_map(|stem| kit_lookup_keys(stem))
            .collect();
        let sources = TextureSources {
            held: &[&place(held)],
            below: (&folders, &mtl),
            common: &common,
            common_below: &common_below,
            installed: installed.as_ref(),
        };
        supply(path, &sources)
    }

    /// What `supply` makes of `path`, a Common path into a `Common/` subfolder, with the
    /// kept `Common/` textures at `kept` below `Common/` without their extensions — keyed
    /// as the pass files them (`kit_lookup_keys`) — and the installed CPKs' `installed` the
    /// same way.
    fn supplied_below(path: &str, kept: &[&str], installed: Option<&[&str]>) -> Supply {
        let installed = installed.map(|stems| {
            stems
                .iter()
                .flat_map(|stem| kit_lookup_keys(stem))
                .collect()
        });
        let kept = kept.iter().flat_map(|stem| kit_lookup_keys(stem)).collect();
        let none = BTreeSet::new();
        let folders = slot_05(&[], &[]);
        let mtl = ScopePath::new("Players/05 - A/x.mtl").unwrap();
        let sources = TextureSources {
            held: &[],
            below: (&folders, &mtl),
            common: &none,
            common_below: &kept,
            installed: installed.as_ref(),
        };
        supply(path, &sources)
    }

    /// Slot 05's held textures, its own at each of `below` and its links standing for
    /// `linked`.
    fn slot_05(below: &[&str], linked: &[&str]) -> HeldTextures {
        let mut held = HeldTextures {
            folder: ScopePath::new("Players/05 - A").unwrap(),
            textures: TextureFolders::default(),
        };
        for below in below {
            held.textures.insert(true, below);
        }
        for linked in linked {
            held.textures.insert_link(true, linked);
        }
        held
    }

    /// What `supply` makes of `path` named by the `.mtl` at `mtl` of the folder holding `held`,
    /// with nothing in `Common/` and no installed lookup.
    fn supplied_to(held: &HeldTextures, mtl: &str, path: &str) -> Supply {
        let mtl = ScopePath::new(mtl).unwrap();
        let places = held.of(&mtl);
        let common_below = BTreeSet::new();
        let sources = TextureSources {
            held: &places,
            below: (held, &mtl),
            common: &BTreeSet::new(),
            common_below: &common_below,
            installed: None,
        };
        supply(path, &sources)
    }

    /// `Common/`'s `HeldTextures`, its textures `held` each at its path below `Common/`
    /// without its extension, as `held_common` builds them for `common_mtl_findings`.
    fn common_held(held: &[&str]) -> HeldTextures {
        let mut textures = HeldTextures {
            folder: common_folder(),
            textures: TextureFolders::default(),
        };
        for below in held {
            textures.textures.insert(true, below);
        }
        textures
    }

    #[test]
    fn a_mtl_path_below_its_folder_is_supplied_by_the_texture_at_that_path_alone() {
        let body = "Players/05 - A/jessie/body/x.mtl";
        let root = "Players/05 - A/face_high.mtl";
        let below = slot_05(
            &["jessie/body/shorts/y", "jessie/body/shorts/pants_kit1"],
            &[],
        );
        assert_eq!(
            supplied_to(&below, body, "./shorts/y.dds"),
            Supply::Supplied
        );
        assert_eq!(
            supplied_to(&below, body, "./Shorts/pants_kitN.dds"),
            Supply::Supplied
        );
        assert_eq!(supplied_to(&below, root, "./shorts/y.dds"), Supply::Missing);
        // The root's own `y` and a link of its stem are not at that path.
        let beside = slot_05(&["y"], &["y"]);
        assert_eq!(
            supplied_to(&beside, root, "./shorts/y.dds"),
            Supply::Missing
        );
        assert_eq!(
            supplied_to(&beside, root, "./shorts/dummy_kit.dds"),
            Supply::Supplied
        );
    }

    #[test]
    fn a_mtl_s_texture_name_is_held_in_its_folder_or_a_parent_never_a_subfolder() {
        let held = slot_05(&["jessie/skin", "jessie/shorts"], &["hair"]);
        let supplied_to = |mtl: &str, path: &str| supplied_to(&held, mtl, path);
        let deep = "Players/05 - A/jessie/body/x.mtl";
        assert_eq!(supplied_to(deep, "./skin.dds"), Supply::Supplied);
        assert_eq!(supplied_to(deep, "./hair.dds"), Supply::Supplied);
        let root = "Players/05 - A/face_high.mtl";
        assert_eq!(supplied_to(root, "./shorts.dds"), Supply::Missing);
        assert_eq!(supplied_to(root, "./hair.dds"), Supply::Supplied);
    }

    const COMMON_HAIR: &str = "model/character/uniform/common/team_folder/hair.dds";

    #[test]
    fn a_stem_the_folder_holds_is_supplied_whatever_directory_and_case_the_path_spells() {
        for path in [
            "./skin.dds",
            "./Skin.DDS",
            "skin.dds",
            "model/character/uniform/common/team_folder/SKIN.dds",
            "model/character/face/real/skin.dds",
        ] {
            assert_eq!(
                supplied(path, &["skin"], &[], None),
                Supply::Supplied,
                "{path}"
            );
        }
    }

    #[test]
    fn a_kit_reference_is_supplied_by_a_variant_of_its_set() {
        assert_eq!(
            supplied("./pants_kitN.dds", &["pants_kit1"], &[], None),
            Supply::Supplied
        );
        assert_eq!(
            supplied("./pants_kitN.dds", &["socks_kit1"], &[], None),
            Supply::Missing
        );
    }

    #[test]
    fn a_kit_reference_is_supplied_by_the_set_s_fallback_which_its_own_name_does_not_supply() {
        // `pants_kitF.dds` supplies the set's `kitN` reference; a path naming the `kitF`
        // file itself is missing, the file never being emitted under its own name.
        assert_eq!(
            supplied("./pants_kitN.dds", &["pants_kitF"], &[], None),
            Supply::Supplied
        );
        assert_eq!(
            supplied("./pants_kitF.dds", &["pants_kitF"], &[], None),
            Supply::Missing
        );
        let held = slot_05(&["pants_kitF"], &[]);
        assert_eq!(
            supplied_to(&held, "Players/05 - A/x.mtl", "./pants_kitF.dds"),
            Supply::Missing
        );
    }

    #[test]
    fn a_kit_reference_in_a_subfolder_is_supplied_by_a_prefix_or_suffix_fallback() {
        // `jessie/kitF_pants.dds` and `jessie/pants_kitF.dds` are the `kitF` fallback of
        // their sets: `jessie/kitN_pants.dds` and `jessie/pants_kitN.dds` resolve to them,
        // the `kitF` names themselves finding nothing.
        for path in [
            "model/character/uniform/common/team_folder/jessie/kitN_pants.dds",
            "team_common_folder/jessie/pants_kitN.dds",
        ] {
            assert_eq!(
                supplied_below(path, &["jessie/kitF_pants", "jessie/pants_kitF"], None),
                Supply::Supplied,
                "{path}"
            );
        }
        for path in [
            "model/character/uniform/common/team_folder/jessie/kitF_pants.dds",
            "team_common_folder/jessie/pants_kitF.dds",
        ] {
            assert_eq!(
                supplied_below(path, &["jessie/kitF_pants", "jessie/pants_kitF"], None),
                Supply::Missing,
                "{path}"
            );
        }
    }

    #[test]
    fn a_kit_reference_in_a_common_subfolder_is_supplied_by_a_variant_at_its_path() {
        // A variant of the set at `jessie/` supplies the `kitN` reference there; one held
        // at `Common/`'s root does not: the path resolves at `jessie/` alone.
        let path = "team_common_folder/jessie/kitN_pants.dds";
        assert_eq!(
            supplied_below(path, &["jessie/kit1_pants"], None),
            Supply::Supplied
        );
        assert_eq!(supplied_below(path, &["kit1_pants"], None), Supply::Missing);
    }

    #[test]
    fn a_dummy_stem_and_a_game_path_are_not_looked_for() {
        assert_eq!(
            supplied("./dummy_kit_01.dds", &[], &[], Some(&[])),
            Supply::Supplied
        );
        assert_eq!(
            supplied("./DUMMY_kit_01.dds", &[], &[], Some(&[])),
            Supply::Supplied
        );
        assert_eq!(
            supplied(
                "model/character/face/common/face_eyelash.dds",
                &[],
                &[],
                Some(&[])
            ),
            Supply::Supplied
        );
        // A `./` path into a subfolder of a `Common/` `.mtl` resolves at that path in
        // `Common/` alone: `Common/jessie/sub/skin.dds` supplies it, `Common/sub/skin.dds`
        // or `Common/jessie/skin.dds` does not.
        let held = common_held(&["jessie/sub/skin", "jessie/skin"]);
        assert_eq!(
            supplied_to(&held, "Common/jessie/x.mtl", "./sub/skin.dds"),
            Supply::Supplied
        );
        for held in [
            common_held(&["jessie/skin"]),
            common_held(&["sub/skin"]),
            common_held(&[]),
        ] {
            assert_eq!(
                supplied_to(&held, "Common/jessie/x.mtl", "./sub/skin.dds"),
                Supply::Missing
            );
        }
        // A Common path into a subfolder resolves at its path below `Common/`, checked on
        // every engine: nothing at `refkit/` is a miss.
        assert_eq!(
            supplied(
                "model/character/uniform/common/team_folder/refkit/skin.dds",
                &[],
                &[],
                Some(&[])
            ),
            Supply::Missing
        );
    }

    #[test]
    fn a_local_or_bare_path_the_folder_does_not_hold_is_missing() {
        // `Common/` and the installed CPKs do not count for a texture of the face.
        for path in ["./skin.dds", "skin.dds"] {
            assert_eq!(
                supplied(path, &["hair"], &["skin"], Some(&["skin"])),
                Supply::Missing,
                "{path}"
            );
        }
    }

    #[test]
    fn a_common_path_below_a_subfolder_resolves_at_that_path_among_the_common_textures_a_link_reaches()
     {
        // `model/character/uniform/common/team_folder/jessie/skin.dds` names
        // `Common/jessie/skin.dds`:
        // supplied when that file is among the textures the pass keeps, or an installed CPK
        // holds it at that path below `Common/`; missing otherwise.
        let path = "model/character/uniform/common/team_folder/jessie/skin.dds";
        assert_eq!(
            supplied_below(path, &["jessie/skin"], None),
            Supply::Supplied
        );
        assert_eq!(
            supplied_below(path, &[], Some(&["jessie/skin"])),
            Supply::Supplied
        );
        // Nothing at that path is a miss, `Common/skin.dds` included: it resolves at
        // `jessie/` alone.
        for held in [&[][..], &["skin"][..], &["other/skin"][..]] {
            assert_eq!(
                supplied_below(path, held, Some(&[])),
                Supply::Missing,
                "{held:?}"
            );
        }
    }

    #[test]
    fn a_common_path_is_supplied_by_common_or_an_installed_cpk_and_missing_without_a_lookup() {
        assert_eq!(
            supplied(COMMON_HAIR, &[], &["hair"], None),
            Supply::Supplied
        );
        assert_eq!(
            supplied(COMMON_HAIR, &[], &[], Some(&["hair"])),
            Supply::Supplied
        );
        assert_eq!(
            supplied(COMMON_HAIR, &[], &["skin"], Some(&["skin"])),
            Supply::Missing
        );
        // A lookup that cannot be made holds nothing.
        assert_eq!(supplied(COMMON_HAIR, &[], &[], None), Supply::Missing);
    }

    #[test]
    fn a_common_path_into_a_subfolder_is_looked_up_on_pre_fox_too() {
        // `supplied` has nothing at any path below `Common/` (the pre-Fox shape before the
        // check): the subfolder path is a miss where it was not looked for, supplied where
        // the kept textures or the installed CPKs hold it at that path.
        let path = "team_common_folder/jessie/hair.dds";
        assert_eq!(supplied(path, &[], &[], Some(&[])), Supply::Missing);
        assert_eq!(
            supplied_below(path, &["jessie/hair"], Some(&[])),
            Supply::Supplied
        );
        assert_eq!(
            supplied_below(path, &[], Some(&["jessie/hair"])),
            Supply::Supplied
        );
        // `Common/hair.dds` at the root is not at `jessie/`'s path.
        assert_eq!(supplied_below(path, &["hair"], Some(&[])), Supply::Missing);
    }

    #[test]
    fn a_common_path_into_a_kitn_directory_is_supplied_by_a_member_of_its_set() {
        // The `kitN` directory stands for the set of its `kit1`-`kit9` siblings:
        // `Common/kit1/armor.dds`, `Common/kitF/armor.dds` and an installed CPK's `kit1/`
        // copy each supply it; missing with none of them.
        let path = "team_common_folder/kitN/armor.dds";
        assert_eq!(
            supplied_below(path, &["kit1/armor"], None),
            Supply::Supplied
        );
        assert_eq!(
            supplied_below(path, &["kitF/armor"], None),
            Supply::Supplied
        );
        assert_eq!(
            supplied_below(path, &[], Some(&["kit1/armor"])),
            Supply::Supplied
        );
        assert_eq!(supplied_below(path, &[], Some(&[])), Supply::Missing);
        // A `kitF` path names no file: missing even when `Common/kitF/armor.dds` is held.
        let kitf = "team_common_folder/kitF/armor.dds";
        assert_eq!(
            supplied_below(kitf, &["kitF/armor"], Some(&[])),
            Supply::Missing
        );
    }

    /// A material set's `name` naming `paths`.
    fn material(name: &str, paths: &[&str]) -> MaterialRead {
        MaterialRead {
            name: name.to_owned(),
            paths: paths.iter().map(|path| (*path).to_owned()).collect(),
            mesh_used: false,
        }
    }

    /// The findings of `texture_findings` for `materials` of `face.mtl` in `Players/05 - A`,
    /// `used` mesh-used, nothing held but `Common/`'s `hair` and no installed lookup.
    fn findings(materials: &[MaterialRead], used: &[&str]) -> Vec<ContentFinding> {
        let common = BTreeSet::from(["hair".to_owned()]);
        let folders = slot_05(&[], &[]);
        let mtl = ScopePath::new("Players/05 - A/face.mtl").unwrap();
        let common_below = BTreeSet::new();
        let sources = TextureSources {
            held: &[],
            below: (&folders, &mtl),
            common: &common,
            common_below: &common_below,
            installed: None,
        };
        let scope = IssueScope::Folder(ScopePath::new("Players/05 - A").unwrap());
        texture_findings(
            "face.mtl",
            materials,
            &used.iter().copied().collect(),
            &sources,
            &scope,
        )
    }

    /// The finding `code` on `Players/05 - A` about `face.mtl`'s `texture`, named by
    /// `materials`; every texture finding keeps the folder.
    fn finding(code: &'static str, texture: &str, materials: &str) -> ContentFinding {
        ContentFinding {
            code,
            scope: IssueScope::Folder(ScopePath::new("Players/05 - A").unwrap()),
            context: vec![
                ("file", "face.mtl".to_owned()),
                ("texture", texture.to_owned()),
                ("materials", materials.to_owned()),
            ],
            disposition: Disposition::Keep,
            pass_through_eligible: false,
        }
    }

    #[test]
    fn one_path_several_samplers_name_is_one_finding_naming_each_material_once() {
        let found = findings(
            &[
                material("unused", &["./skin.dds"]),
                material("head", &["./skin.dds", "./skin.dds", "./eye.dds"]),
            ],
            &["head"],
        );
        assert_eq!(
            found,
            [
                finding("mtl_texture_not_found", "./skin.dds", "unused, head"),
                finding("mtl_texture_not_found", "./eye.dds", "head"),
            ]
        );
    }

    #[test]
    fn a_miss_only_unused_materials_name_is_info_and_one_without_a_lookup_is_the_same_warning() {
        let gone = "team_common_folder/gone.dds";
        let cap = "model/character/uniform/common/team_folder/cap.dds";
        let found = findings(
            &[
                material("unused", &["./skin.dds", COMMON_HAIR, gone]),
                material("head", &[cap, "./eye.dds"]),
            ],
            &["head"],
        );
        assert_eq!(
            found,
            [
                finding("mtl_texture_unused_missing", "./skin.dds", "unused"),
                finding("mtl_texture_unused_missing", gone, "unused"),
                finding("mtl_texture_not_found", cap, "head"),
                finding("mtl_texture_not_found", "./eye.dds", "head"),
            ]
        );
    }

    #[test]
    fn a_common_path_naming_a_folder_other_than_the_team_s_is_an_error_dropping_the_folder() {
        let path = "model/character/uniform/common/o/env.dds";
        let found = findings(&[material("head", &[path, "./skin.dds"])], &["head"]);

        // The refusal is for that path alone: the other path's miss is still reported.
        assert_eq!(
            found,
            [
                ContentFinding {
                    code: "mtl_common_path_invalid",
                    scope: IssueScope::Folder(ScopePath::new("Players/05 - A").unwrap()),
                    context: vec![
                        ("file", "face.mtl".to_owned()),
                        ("attribute", "path".to_owned()),
                        ("value", path.to_owned()),
                    ],
                    disposition: Disposition::DropFolder,
                    pass_through_eligible: false,
                },
                finding("mtl_texture_not_found", "./skin.dds", "head"),
            ]
        );
    }

    #[test]
    fn a_kit_stand_in_is_not_looked_for_either() {
        for path in [
            "./team_kit.dds",
            "team_gk_kit.dds",
            "./Team_Kit_Srm.dds",
            "team_common_folder/team_kit_srm.dds",
        ] {
            assert_eq!(
                supplied(path, &[], &[], Some(&[])),
                Supply::Supplied,
                "{path}"
            );
        }
    }

    #[test]
    fn a_path_naming_the_stand_in_s_old_spelling_is_an_error_and_a_refused_directory_gets_both() {
        let old = "team_common_folder/dummy_kit.dds";
        let both = "common/dummy_kit.dds";
        let found = findings(&[material("head", &[old, both, "./skin.dds"])], &["head"]);

        // Both refusals drop the folder and a path tripping both gets both findings,
        // each naming its own fix; the other path's miss is still reported.
        let scope = IssueScope::Folder(ScopePath::new("Players/05 - A").unwrap());
        let invalid = |code: &'static str, context: Vec<(&'static str, String)>| ContentFinding {
            code,
            scope: scope.clone(),
            context,
            disposition: Disposition::DropFolder,
            pass_through_eligible: false,
        };
        assert_eq!(
            found,
            [
                invalid(
                    "dummy_kit_name_old",
                    vec![("file", "face.mtl".to_owned()), ("texture", old.to_owned()),]
                ),
                invalid(
                    "mtl_common_path_invalid",
                    vec![
                        ("file", "face.mtl".to_owned()),
                        ("attribute", "path".to_owned()),
                        ("value", both.to_owned()),
                    ]
                ),
                invalid(
                    "dummy_kit_name_old",
                    vec![
                        ("file", "face.mtl".to_owned()),
                        ("texture", both.to_owned()),
                    ]
                ),
                finding("mtl_texture_not_found", "./skin.dds", "head"),
            ]
        );
    }

    #[test]
    fn a_common_mtl_s_invalid_folder_drops_the_file_alone() {
        // A `Common/` `.mtl` is a file of `Common/`: the finding drops the file, not a folder.
        let path = "common_folder/refs/refkit/env.dds";
        let mtl = ScopePath::new("Common/jessie/x.mtl").unwrap();
        let folders = common_held(&[]);
        let common_below = BTreeSet::new();
        let sources = TextureSources {
            held: &[],
            below: (&folders, &mtl),
            common: &BTreeSet::new(),
            common_below: &common_below,
            installed: None,
        };
        let scope = IssueScope::File(mtl.clone());
        let found = texture_findings(
            "x.mtl",
            &[material("head", &[path])],
            &BTreeSet::from(["head"]),
            &sources,
            &scope,
        );

        assert_eq!(
            found,
            [ContentFinding {
                code: "mtl_common_path_invalid",
                scope,
                context: vec![
                    ("file", "x.mtl".to_owned()),
                    ("attribute", "path".to_owned()),
                    ("value", path.to_owned()),
                ],
                disposition: Disposition::DropFile,
                pass_through_eligible: false,
            }]
        );
    }

    #[test]
    fn a_common_path_in_either_spelling_is_checked_as_usual() {
        // `team_folder`, the folder a member writes for the team's Common in either
        // spelling: kept and looked up, the usual not-found finding, not the refusal.
        for path in [
            "model/character/uniform/common/team_folder/env.dds",
            "Team_Common_Folder/env.dds",
        ] {
            assert_eq!(
                findings(&[material("head", &[path])], &["head"]),
                [finding("mtl_texture_not_found", path, "head")],
                "{path}"
            );
        }
    }

    #[test]
    fn the_invalid_folder_check_runs_on_a_fox_target_too() {
        // The spelling is the export's: a `check` and a compile for either engine judge it
        // alike, so the refusal reports on Fox as on pre-Fox.
        let path = "common_folder/797/hair.dds";
        let common_below = BTreeSet::new();
        let folders = slot_05(&[], &[]);
        let mtl = ScopePath::new("Players/05 - A/face.mtl").unwrap();
        let sources = TextureSources {
            held: &[],
            below: (&folders, &mtl),
            common: &BTreeSet::new(),
            common_below: &common_below,
            installed: None,
        };
        let scope = IssueScope::Folder(ScopePath::new("Players/05 - A").unwrap());
        let found = texture_findings(
            "face.mtl",
            &[material("head", &[path])],
            &BTreeSet::from(["head"]),
            &sources,
            &scope,
        );

        assert_eq!(
            found,
            [ContentFinding {
                code: "mtl_common_path_invalid",
                scope,
                context: vec![
                    ("file", "face.mtl".to_owned()),
                    ("attribute", "path".to_owned()),
                    ("value", path.to_owned()),
                ],
                disposition: Disposition::DropFolder,
                pass_through_eligible: false,
            }]
        );
    }
}
