//! The per-version player skeletons: the games' own `.skl` files, embedded and parsed once at
//! first use. Each version's skeleton differs in bone count and rest poses
//! (`spec/model_conversion/conversion.md` "Skeleton data"), so importers and retargeting always
//! go through [`skeletons`] rather than assuming one skeleton.
//!
//! Two hierarchies exist side by side: `skl_parent` is the `.skl` file's own parent column,
//! which makes almost every `dsk_*` bone a root; `render_parent` is the transcribed hierarchy
//! used for mesh splitting and Blender armatures (a bone absent from it is a render root).

mod fold;
mod render_parents;
/// Skeleton retargeting: conforming an IR model to another version's skeleton.
pub mod retarget;

use std::sync::LazyLock;

use pes_version::PesVersion;

use crate::affine::Affine;

/// One bone of a version's skeleton: name, both parent columns, and its bind transform.
#[derive(Debug, Clone, PartialEq)]
pub struct PesBone {
    /// The bone's name in the game files (`sk_hand_l`, `dsk_deltoid_r`, ...).
    pub name: String,
    /// The `.skl` file's own `parent_index`, resolved to a name; `None` for a file-level root.
    /// Almost every `dsk_*` bone is a root here — this is not the render hierarchy.
    pub skl_parent: Option<String>,
    /// The render hierarchy parent used by mesh splitting and Blender armatures
    /// (`render_parents.rs`); `None` for a render root or a bone outside the table.
    pub render_parent: Option<String>,
    /// The model-space 3x4 bind transform the `.skl` file stores.
    pub matrix: Affine,
}

/// One skeleton file's bones, sorted by name for binary search.
#[derive(Debug, Clone, PartialEq)]
pub struct Skeleton {
    /// The bones, sorted by name.
    pub bones: Vec<PesBone>,
}

impl Skeleton {
    /// The bone called `name`, if the skeleton has one.
    pub fn bone(&self, name: &str) -> Option<&PesBone> {
        self.bones
            .binary_search_by(|bone| bone.name.as_str().cmp(name))
            .ok()
            .map(|index| &self.bones[index])
    }

    /// Parses an embedded `.skl` into the sorted bone list.
    fn from_skl(bytes: &'static [u8]) -> Skeleton {
        let skl = fmdl::SklFile::read(bytes)
            .expect("embedded game skeletons from resources/skeletons/ parse");
        let mut bones: Vec<PesBone> = skl
            .bones
            .iter()
            .map(|bone| PesBone {
                name: bone.name.clone(),
                skl_parent: bone.parent.map(|parent| skl.bones[parent].name.clone()),
                render_parent: render_parent(&bone.name).map(str::to_string),
                matrix: Affine::from_rotation_translation(bone.rotation, bone.translation),
            })
            .collect();
        bones.sort_by(|a, b| a.name.cmp(&b.name));
        Skeleton { bones }
    }
}

/// A version's four skeleton files: body always; face and hands from the version's own files
/// on Fox versions, from PES19's on pre-Fox (pre-Fox ships no face or hand SKL).
#[derive(Debug)]
pub struct VersionSkeletons {
    /// The player body skeleton.
    pub body: Skeleton,
    /// The face skeleton.
    pub face: Skeleton,
    /// The left-hand skeleton.
    pub hand_l: Skeleton,
    /// The right-hand skeleton.
    pub hand_r: Skeleton,
    /// The verbatim game file, for Fox template injection.
    pub body_skl_bytes: &'static [u8],
}

const PES15_BODY: &[u8] = include_bytes!("../../../../../resources/skeletons/pes15/body.skl");
const PES16_BODY: &[u8] = include_bytes!("../../../../../resources/skeletons/pes16/body.skl");
const PES17_BODY: &[u8] = include_bytes!("../../../../../resources/skeletons/pes17/body.skl");
const PES18_BODY: &[u8] = include_bytes!("../../../../../resources/skeletons/pes18/body.skl");
const PES19_BODY: &[u8] = include_bytes!("../../../../../resources/skeletons/pes19/body.skl");
const PES21_BODY: &[u8] = include_bytes!("../../../../../resources/skeletons/pes21/body.skl");

/// The Fox face/hand skeletons are byte-identical across 18/19/21, and pre-Fox versions have
/// none of their own, so PES19's copy serves every version.
const PES19_FACE: &[u8] = include_bytes!("../../../../../resources/skeletons/pes19/face.skl");
const PES19_HAND_L: &[u8] = include_bytes!("../../../../../resources/skeletons/pes19/hand_l.skl");
const PES19_HAND_R: &[u8] = include_bytes!("../../../../../resources/skeletons/pes19/hand_r.skl");

fn version_skeletons(body_skl_bytes: &'static [u8]) -> VersionSkeletons {
    VersionSkeletons {
        body: Skeleton::from_skl(body_skl_bytes),
        face: Skeleton::from_skl(PES19_FACE),
        hand_l: Skeleton::from_skl(PES19_HAND_L),
        hand_r: Skeleton::from_skl(PES19_HAND_R),
        body_skl_bytes,
    }
}

static PES15: LazyLock<VersionSkeletons> = LazyLock::new(|| version_skeletons(PES15_BODY));
static PES16: LazyLock<VersionSkeletons> = LazyLock::new(|| version_skeletons(PES16_BODY));
static PES17: LazyLock<VersionSkeletons> = LazyLock::new(|| version_skeletons(PES17_BODY));
static PES18: LazyLock<VersionSkeletons> = LazyLock::new(|| version_skeletons(PES18_BODY));
static PES19: LazyLock<VersionSkeletons> = LazyLock::new(|| version_skeletons(PES19_BODY));
static PES21: LazyLock<VersionSkeletons> = LazyLock::new(|| version_skeletons(PES21_BODY));

/// A version's skeletons, parsed once. PES20 was never installed for verification and is
/// assumed to equal PES21.
pub fn skeletons(version: PesVersion) -> &'static VersionSkeletons {
    match version {
        // No model tool reaches PES 2014; PES 2015's until one does.
        PesVersion::Pes14 => &PES15,
        PesVersion::Pes15 => &PES15,
        PesVersion::Pes16 => &PES16,
        PesVersion::Pes17 => &PES17,
        PesVersion::Pes18 => &PES18,
        PesVersion::Pes19 => &PES19,
        PesVersion::Pes20 => &PES21,
        PesVersion::Pes21 => &PES21,
    }
}

/// The render hierarchy: `None` for a render root or a name outside the table.
pub(crate) fn render_parent(name: &str) -> Option<&'static str> {
    render_parents::RENDER_PARENTS
        .binary_search_by(|(key, _)| key.cmp(&name))
        .ok()
        .map(|index| render_parents::RENDER_PARENTS[index].1)
}

/// The bone `name` in `tables`, body first then face and hands.
pub(crate) fn version_bone<'a>(tables: &'a VersionSkeletons, name: &str) -> Option<&'a PesBone> {
    [&tables.body, &tables.face, &tables.hand_l, &tables.hand_r]
        .iter()
        .find_map(|skeleton| skeleton.bone(name))
}

/// `version_bone` with the hand tables first: the conforming lookup for a model carrying
/// `skh_` weights. Such a model conforms to the target's `hand_*` tables, and shared
/// wrist/forearm names sit at a different pose in `hand_*` than in `body`. On versions
/// where the tables agree this returns the same bone either way.
pub(crate) fn version_bone_hand_first<'a>(
    tables: &'a VersionSkeletons,
    name: &str,
) -> Option<&'a PesBone> {
    [&tables.hand_l, &tables.hand_r, &tables.body, &tables.face]
        .iter()
        .find_map(|skeleton| skeleton.bone(name))
}

/// Whether `name` is a bone some version's tables know (PES20 is PES21).
pub(crate) fn is_standard(name: &str) -> bool {
    [
        PesVersion::Pes15,
        PesVersion::Pes16,
        PesVersion::Pes17,
        PesVersion::Pes18,
        PesVersion::Pes19,
        PesVersion::Pes21,
    ]
    .iter()
    .any(|version| version_bone(skeletons(*version), name).is_some())
}

/// Whether `name` is a face bone (`skf_*`): posed per face (the base face table moved by the
/// player's own face diff), so no shared table is a reference for it. It keeps the model's own
/// matrix through conformance, as a custom bone does.
pub(crate) fn is_face_bone(name: &str) -> bool {
    name.starts_with("skf_")
}

/// The bone that takes `name`'s weight when a version lacks it (one hop; callers chain).
pub(crate) fn fold_target(name: &str) -> Option<&'static str> {
    fold::FOLD_TARGETS
        .binary_search_by(|(key, _)| key.cmp(&name))
        .ok()
        .map(|index| fold::FOLD_TARGETS[index].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bone_counts_per_version() {
        let bodies = [
            (PesVersion::Pes15, 76usize),
            (PesVersion::Pes16, 70),
            (PesVersion::Pes17, 70),
            (PesVersion::Pes18, 76),
            (PesVersion::Pes19, 116),
            (PesVersion::Pes21, 175),
        ];
        for (version, count) in bodies {
            let set = skeletons(version);
            assert_eq!(set.body.bones.len(), count, "{version:?}");
            assert_eq!(set.face.bones.len(), 49, "{version:?}");
            assert_eq!(set.hand_l.bones.len(), 24, "{version:?}");
            assert_eq!(set.hand_r.bones.len(), 24, "{version:?}");
        }
        assert!(std::ptr::eq(
            skeletons(PesVersion::Pes20),
            skeletons(PesVersion::Pes21)
        ));
    }

    #[test]
    fn skl_and_render_parents() {
        let body = &skeletons(PesVersion::Pes21).body;
        assert_eq!(
            body.bone("sk_chest")
                .expect("sk_chest")
                .skl_parent
                .as_deref(),
            Some("sk_belly")
        );
        assert_eq!(body.bone("dsk_hip").expect("dsk_hip").skl_parent, None);
        assert_eq!(
            body.bone("sk_belly")
                .expect("sk_belly")
                .render_parent
                .as_deref(),
            Some("dsk_hip")
        );
        assert_eq!(body.bone("dsk_hip").expect("dsk_hip").render_parent, None);
    }

    #[test]
    fn bind_poses_vary_by_version() {
        let head = skeletons(PesVersion::Pes17)
            .body
            .bone("sk_head")
            .expect("sk_head");
        let translation = head.matrix.translation();
        assert!((translation[0] - 0.0).abs() < 1e-6, "{translation:?}");
        assert!((translation[1] - 1.640058).abs() < 1e-6, "{translation:?}");
        assert!((translation[2] - 0.054333).abs() < 1e-6, "{translation:?}");

        let pes15 = skeletons(PesVersion::Pes15)
            .body
            .bone("dsk_scapula_r")
            .expect("pes15 dsk_scapula_r");
        let pes21 = skeletons(PesVersion::Pes21)
            .body
            .bone("dsk_scapula_r")
            .expect("pes21 dsk_scapula_r");
        assert!(pes15.matrix.max_component_delta(&pes21.matrix) > 1.8);

        for (a, b) in skeletons(PesVersion::Pes16)
            .body
            .bones
            .iter()
            .zip(skeletons(PesVersion::Pes17).body.bones.iter())
        {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn tables_are_sorted_without_duplicates() {
        for table in [render_parents::RENDER_PARENTS, fold::FOLD_TARGETS] {
            for pair in table.windows(2) {
                assert!(pair[0].0 < pair[1].0, "{} >= {}", pair[0].0, pair[1].0);
            }
        }
    }

    #[test]
    fn render_parents_form_a_tree() {
        for (_, parent) in render_parents::RENDER_PARENTS {
            assert!(
                *parent == "dsk_hip"
                    || render_parents::RENDER_PARENTS
                        .binary_search_by(|(key, _)| key.cmp(parent))
                        .is_ok(),
                "{parent} is neither a key nor the render root"
            );
        }
    }

    #[test]
    fn render_parent_chains_end_at_a_root() {
        // `model_to_ir`'s parent-first placement waits a bone on its unplaced parent; a
        // cycle in this table would leave bones waiting forever.
        for &(name, _) in render_parents::RENDER_PARENTS {
            let mut bone = name;
            for _ in 0..render_parents::RENDER_PARENTS.len() {
                match render_parent(bone) {
                    Some(parent) => bone = parent,
                    None => break,
                }
            }
            assert!(
                render_parent(bone).is_none(),
                "{name}: chain still going after {} hops",
                render_parents::RENDER_PARENTS.len()
            );
        }
    }

    #[test]
    fn fold_chains_land_on_a_bone() {
        for version in PesVersion::ALL {
            let body = &skeletons(version).body;
            for (key, _) in fold::FOLD_TARGETS {
                let mut name = *key;
                let mut hops = 0;
                let mut seen = Vec::new();
                while body.bone(name).is_none() {
                    assert!(hops < 10, "{version:?} {key}: chain past 10 hops");
                    assert!(!seen.contains(&name), "{version:?} {key}: loop at {name}");
                    seen.push(name);
                    name = fold_target(name)
                        .unwrap_or_else(|| panic!("{version:?} {key}: chain ends at {name}"));
                    hops += 1;
                }
            }
        }
    }

    #[test]
    fn unknown_names_have_no_table_entry() {
        assert_eq!(render_parent("nonexistent"), None);
        assert_eq!(fold_target("sk_head"), None);
    }
}
