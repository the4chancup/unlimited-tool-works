//! Multi-part merge: the Compiler assembles several part models into
//! one `.model` + `.mtl`. Bones are unioned by name (per-mesh bone groups
//! remapped to the unioned indices — per-vertex `bone_indices` index the
//! group and do not change); a shared name must carry the same matrix
//! within `BONE_MATRIX_TOLERANCE`, float noise between Konami parts of one
//! skeleton rather than a real bind-pose difference. Materials are merged
//! by name with equal-definition checking, meshes concatenated in
//! caller-supplied part order, extension headers unioned and
//! deduplicated, `flags` must agree, `bounds` is the union of the parts'
//! boxes, and `lod` is rebuilt from the deepest mesh. `parts` order is
//! canonical and preserved, so the result is deterministic by
//! construction.

use std::collections::{HashMap, HashSet};

use crate::format::ModelError;
use crate::format::mtl::MaterialSet;
use crate::format::{BoundingBox, LodRecord};
use crate::model::Model;

/// Why several parts could not be merged into one model.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MergeError {
    /// Two parts define one material name differently.
    #[error("material {name} is defined differently in two parts")]
    MaterialConflict {
        /// The conflicting material name.
        name: String,
    },
    /// Two parts carry one bone name with different matrices.
    #[error("bone {name} differs between parts")]
    SkeletonConflict {
        /// The conflicting bone name.
        name: String,
    },
    /// A part lists the same bone name twice; union by name is ambiguous
    /// there.
    #[error("bone {name} appears twice in one part")]
    DuplicateBoneName {
        /// The repeated bone name.
        name: String,
    },
    /// Header flags differ (meaning unknown, so no merging rule).
    #[error("header flags differ between parts")]
    FlagsConflict,
    /// A part fails `Model::validate`, or an operation error.
    #[error(transparent)]
    Other(#[from] ModelError),
}

/// The absolute per-component difference at which two same-named bones are
/// still the same bone: bones shared between Konami parts of one skeleton
/// differ by at most 3.6e-5 (float noise), real bind-pose differences
/// start at 2.0e-4.
const BONE_MATRIX_TOLERANCE: f32 = 1e-4;

/// Whether every one of the twelve components of `a` and `b` differs by
/// less than `BONE_MATRIX_TOLERANCE`.
fn same_matrix(a: &[f32; 12], b: &[f32; 12]) -> bool {
    a.iter()
        .zip(b)
        .all(|(a, b)| (a - b).abs() < BONE_MATRIX_TOLERANCE)
}

/// Merges `parts` (caller-supplied canonical order, preserved) into one
/// model and one material set.
pub fn merge(parts: &[(&Model, &MaterialSet)]) -> Result<(Model, MaterialSet), MergeError> {
    let (first, first_set) = parts
        .first()
        .copied()
        .ok_or(ModelError::InvalidModel("no parts to merge"))?;

    let mut output = Model {
        flags: first.flags,
        bones: Vec::new(),
        materials: Vec::new(),
        meshes: Vec::new(),
        extension_headers: Vec::new(),
        bounds: BoundingBox {
            min: [f32::MAX; 4],
            max: [f32::MIN; 4],
        },
        lod: LodRecord::for_levels(0),
    };
    let mut output_set = MaterialSet {
        materials: Vec::new(),
        style: first_set.style.clone(),
    };

    let mut bone_of: HashMap<&str, usize> = HashMap::new();
    let mut material_of: HashMap<&str, usize> = HashMap::new();
    let mut mtl_material_of: HashMap<&str, usize> = HashMap::new();
    let mut level_count = 0usize;
    // Split-group renumbering: each part numbers its `Split-Mesh` groups
    // from 1, so merged parts renumber to keep group identity per part
    // and no collisions across parts.
    let mut next_group = 1usize;

    for &(part, part_set) in parts {
        part.validate()?;
        if part.flags != output.flags {
            return Err(MergeError::FlagsConflict);
        }

        // Within one part a bone name must be unique: union by name is
        // ambiguous otherwise.
        let mut names = HashSet::new();
        for bone in &part.bones {
            if !names.insert(bone.name.as_str()) {
                return Err(MergeError::DuplicateBoneName {
                    name: bone.name.clone(),
                });
            }
        }

        // Bone union: identity of a bone is its name; a shared bone must
        // carry the same matrix within the tolerance.
        let mut bone_remap = vec![usize::MAX; part.bones.len()];
        for (index, bone) in part.bones.iter().enumerate() {
            match bone_of.get(bone.name.as_str()) {
                Some(&unioned) => {
                    if !same_matrix(&output.bones[unioned].matrix, &bone.matrix) {
                        return Err(MergeError::SkeletonConflict {
                            name: bone.name.clone(),
                        });
                    }
                    bone_remap[index] = unioned;
                }
                None => {
                    bone_remap[index] = output.bones.len();
                    bone_of.insert(bone.name.as_str(), output.bones.len());
                    output.bones.push(bone.clone());
                }
            }
        }

        // Model-material union by name; `.mtl` union by name with
        // equal-definition checking (a `.mtl` material no model names is
        // kept — Konami shares one `.mtl` between models).
        let mut material_remap = vec![usize::MAX; part.materials.len()];
        for (index, name) in part.materials.iter().enumerate() {
            match material_of.get(name.as_str()) {
                Some(&unioned) => material_remap[index] = unioned,
                None => {
                    material_remap[index] = output.materials.len();
                    material_of.insert(name.as_str(), output.materials.len());
                    output.materials.push(name.clone());
                }
            }
        }
        for material in &part_set.materials {
            match mtl_material_of.get(material.name.as_str()) {
                Some(&unioned) => {
                    if output_set.materials[unioned] != *material {
                        return Err(MergeError::MaterialConflict {
                            name: material.name.clone(),
                        });
                    }
                }
                None => {
                    mtl_material_of.insert(material.name.as_str(), output_set.materials.len());
                    output_set.materials.push(material.clone());
                }
            }
        }

        let mut split_groups: HashMap<String, usize> = HashMap::new();
        for mesh in &part.meshes {
            let mut mesh = mesh.clone();
            mesh.material = material_remap[mesh.material];
            for slot in &mut mesh.bone_group {
                *slot = bone_remap[*slot];
            }
            for header in &mut mesh.extension_headers {
                if let Some(value) = crate::ops::split::split_group_key(header) {
                    let number = *split_groups.entry(value).or_insert_with(|| {
                        let number = next_group;
                        next_group += 1;
                        number
                    });
                    *header = format!("{}: {number}", crate::ops::split::SPLIT_MESH_HEADER);
                }
            }
            if !mesh.lower_lods.is_empty() {
                level_count = level_count.max(1 + mesh.lower_lods.len());
            }
            output.meshes.push(mesh);
        }

        for header in &part.extension_headers {
            // Small lists; a linear dedup is fine at these sizes.
            if !output.extension_headers.contains(header) {
                output.extension_headers.push(header.clone());
            }
        }

        for axis in 0..3 {
            output.bounds.min[axis] = output.bounds.min[axis].min(part.bounds.min[axis]);
            output.bounds.max[axis] = output.bounds.max[axis].max(part.bounds.max[axis]);
        }
    }
    output.bounds.min[3] = 0.0;
    output.bounds.max[3] = 0.0;
    output.lod = LodRecord::for_levels(level_count as u32);

    Ok((output, output_set))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::fixtures::*;
    use crate::format::mtl::{MaterialSet, MtlStyle};
    use crate::format::{LodRecord, MeshVertices, PreFoxModel};
    use crate::model::Model;

    fn model(bytes: &[u8]) -> Model {
        Model::from_file(&PreFoxModel::read(bytes).unwrap()).unwrap()
    }

    fn set(bytes: &[u8]) -> MaterialSet {
        MaterialSet::read(bytes).unwrap()
    }

    // A model whose single mesh is over the hard bone limit: each face is
    // weighted to its own bone. `shift` separates two parts' geometry.
    fn over_limit_model(shift: f32) -> Model {
        const BONES: usize = 65;
        let mut positions = Vec::new();
        let mut faces = Vec::new();
        let mut indices = Vec::new();
        for bone in 0..BONES {
            for corner in 0..3 {
                positions.push([shift + bone as f32, corner as f32, 0.0]);
                indices.push([bone as u8, 0, 0, 0]);
            }
            faces.push([
                (3 * bone) as u16,
                (3 * bone) as u16 + 1,
                (3 * bone) as u16 + 2,
            ]);
        }
        let mesh = crate::model::Mesh {
            name: None,
            extension_headers: Vec::new(),
            tags: Vec::new(),
            vertices: MeshVertices {
                positions,
                normals: Some(vec![[0.0, 0.0, 1.0]; 3 * BONES]),
                tangents: None,
                bitangents: None,
                colors: None,
                uvs: vec![vec![[0.0, 0.0]; 3 * BONES]],
                bone_indices: Some(indices),
                bone_weights: Some(vec![[1.0, 0.0, 0.0, 0.0]; 3 * BONES]),
                bone_weight_width: 4,
            },
            faces,
            lower_lods: Vec::new(),
            bone_group: (0..BONES).collect(),
            material: 0,
            bounds: crate::format::BoundingBox::of(&[]),
            order: 0,
            editor_data: Vec::new(),
        };
        Model {
            flags: 0,
            bones: (0..BONES)
                .map(|index| crate::format::Bone {
                    name: format!("b{index}"),
                    matrix: [0.0; 12],
                })
                .collect(),
            materials: vec!["material".to_owned()],
            meshes: vec![mesh],
            extension_headers: Vec::new(),
            bounds: crate::format::BoundingBox::of(&[]),
            lod: LodRecord::for_levels(0),
        }
    }

    #[test]
    fn one_part_is_the_identity() {
        for (model_bytes, set_bytes) in [
            (CARD, CARD_RED_MTL),
            (CAP, CAP_MTL),
            (SHADOW, SHADOW_MTL),
            (GLASSES, ACCESSORY_MTL),
            (TAPING, ACCESSORY_MTL),
            (HEAD_HI, HEAD_HI_MTL),
            (HAIR_HIGH, HAIR_MTL),
            (CARDHEAD, CARDHEAD_MTL),
            (COLLAR, CAP_MTL),
        ] {
            let model = model(model_bytes);
            let set = set(set_bytes);
            assert_eq!(merge(&[(&model, &set)]).unwrap(), (model, set));
        }
    }

    #[test]
    fn same_part_twice_shares_bones_and_materials() {
        let cap = model(CAP);
        let cap_mtl = set(CAP_MTL);
        let (merged, merged_set) = merge(&[(&cap, &cap_mtl), (&cap, &cap_mtl)]).unwrap();
        assert_eq!(merged.bones.len(), 4);
        assert_eq!(merged.materials.len(), 1);
        assert_eq!(merged.meshes.len(), 2);
        assert_eq!(merged.meshes[1], merged.meshes[0]);
        assert_eq!(merged_set, cap_mtl);
    }

    #[test]
    fn two_parts_remap_indices() {
        let card = model(CARD);
        let cap = model(CAP);
        let card_red = set(CARD_RED_MTL);
        let cap_mtl = set(CAP_MTL);
        // Precondition: the two parts share no bone names.
        assert!(
            card.bones
                .iter()
                .all(|bone| !cap.bones.iter().any(|other| other.name == bone.name))
        );

        let (merged, merged_set) = merge(&[(&card, &card_red), (&cap, &cap_mtl)]).unwrap();
        assert_eq!(merged.bones.len(), card.bones.len() + cap.bones.len());
        assert_eq!(
            merged.materials,
            [card.materials[0].clone(), cap.materials[0].clone()]
        );
        assert_eq!(merged.meshes[1].material, 1);
        for (slot, &bone) in merged.meshes[1].bone_group.iter().enumerate() {
            assert_eq!(
                merged.bones[bone].name,
                cap.bones[cap.meshes[0].bone_group[slot]].name
            );
        }
        let expected: Vec<String> = card_red
            .materials
            .iter()
            .chain(cap_mtl.materials.iter())
            .map(|material| material.name.clone())
            .collect();
        assert_eq!(
            merged_set
                .materials
                .iter()
                .map(|material| material.name.clone())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(merged_set.style, card_red.style);
    }

    #[test]
    fn bone_matrix_conflict() {
        let cap = model(CAP);
        let cap_mtl = set(CAP_MTL);
        let mut changed = cap.clone();
        changed.bones[0].matrix[3] += 1.0;
        assert_eq!(
            merge(&[(&cap, &cap_mtl), (&changed, &cap_mtl)]),
            Err(MergeError::SkeletonConflict {
                name: cap.bones[0].name.clone(),
            })
        );
    }

    #[test]
    fn a_part_with_repeated_bone_names_errors() {
        let mut cap = model(CAP);
        cap.bones.push(cap.bones[0].clone());
        assert_eq!(cap.bones[0].name, "sk_shoulder_l");
        assert_eq!(
            merge(&[(&cap, &set(CAP_MTL))]),
            Err(MergeError::DuplicateBoneName {
                name: "sk_shoulder_l".to_owned(),
            })
        );
    }

    #[test]
    fn bone_matrix_tolerance_boundary() {
        let cap = model(CAP);
        let cap_mtl = set(CAP_MTL);
        let mut within = cap.clone();
        within.bones[0].matrix[3] += 5e-5;
        let (merged, _) = merge(&[(&cap, &cap_mtl), (&within, &cap_mtl)]).unwrap();
        // The merged bone keeps the first part's matrix.
        assert_eq!(merged.bones[0].matrix, cap.bones[0].matrix);

        let mut beyond = cap.clone();
        beyond.bones[0].matrix[3] += 2e-4;
        assert_eq!(
            merge(&[(&cap, &cap_mtl), (&beyond, &cap_mtl)]),
            Err(MergeError::SkeletonConflict {
                name: cap.bones[0].name.clone(),
            })
        );
    }

    #[test]
    fn material_definition_conflict() {
        let cap = model(CAP);
        let cap_mtl = set(CAP_MTL);
        let mut changed = cap_mtl.clone();
        changed.materials[0].shader = "Other".to_owned();
        assert_eq!(
            merge(&[(&cap, &cap_mtl), (&cap, &changed)]),
            Err(MergeError::MaterialConflict {
                name: cap_mtl.materials[0].name.clone(),
            })
        );
    }

    #[test]
    fn flags_conflict() {
        let cap = model(CAP);
        let cap_mtl = set(CAP_MTL);
        let mut changed = cap.clone();
        changed.flags = 4;
        assert_eq!(
            merge(&[(&cap, &cap_mtl), (&changed, &cap_mtl)]),
            Err(MergeError::FlagsConflict)
        );
    }

    #[test]
    fn lod_record_takes_the_deepest_mesh() {
        let cap = model(CAP);
        let collar = model(COLLAR);
        let cap_mtl = set(CAP_MTL);
        let (merged, _) = merge(&[(&cap, &cap_mtl), (&collar, &cap_mtl)]).unwrap();
        assert_eq!(merged.lod, LodRecord::for_levels(6));
        assert_eq!(merged.lod.parameters, [0.0625, 4.0, 0.3]);
    }

    #[test]
    fn extension_headers_deduplicated() {
        let cardhead = model(CARDHEAD);
        let cardhead_mtl = set(CARDHEAD_MTL);
        let (merged, _) = merge(&[(&cardhead, &cardhead_mtl), (&cardhead, &cardhead_mtl)]).unwrap();
        assert_eq!(merged.extension_headers, cardhead.extension_headers);
    }

    #[test]
    fn no_parts_is_an_error() {
        assert!(matches!(
            merge(&[]),
            Err(MergeError::Other(ModelError::InvalidModel(_)))
        ));
    }

    #[test]
    fn same_matrix_tolerance_is_exclusive() {
        let a = [0.0f32; 12];
        let mut b = a;
        // A component exactly at the tolerance is a different bone.
        b[0] = BONE_MATRIX_TOLERANCE;
        assert!(!same_matrix(&a, &b));
        // Just under it is the same bone.
        b[0] = BONE_MATRIX_TOLERANCE * 0.99;
        assert!(same_matrix(&a, &b));
    }

    // Independently encoded parts all number their groups from 1: the
    // merge must renumber them, or `split::decode` welds different parts'
    // components into one mesh.
    #[test]
    fn split_groups_are_renumbered_across_parts() {
        let mut part_a = over_limit_model(0.0);
        let mut part_b = over_limit_model(1000.0);
        let parents: Vec<Option<usize>> = vec![None; 65];
        crate::ops::split::encode(&mut part_a, &parents).unwrap();
        crate::ops::split::encode(&mut part_b, &parents).unwrap();
        assert!(part_a.meshes.len() > 1 && part_b.meshes.len() > 1);

        let set = MaterialSet {
            materials: Vec::new(),
            style: MtlStyle::default(),
        };
        let (mut merged, _) = merge(&[(&part_a, &set), (&part_b, &set)]).unwrap();
        crate::ops::split::decode(&mut merged).unwrap();

        let mut solo_a = part_a.clone();
        crate::ops::split::decode(&mut solo_a).unwrap();
        let mut solo_b = part_b.clone();
        crate::ops::split::decode(&mut solo_b).unwrap();
        assert_eq!(merged.meshes.len(), 2);
        assert_eq!(merged.meshes[0], solo_a.meshes[0]);
        assert_eq!(merged.meshes[1], solo_b.meshes[0]);
    }

    #[test]
    fn parts_are_validated() {
        // A part's own invariants are checked before its indices reach
        // the remap vectors: a face index out of range is an error, not
        // a merged model.
        let mut bad = model(CARD);
        bad.meshes[0].faces[0][0] = 999;
        let card_red = set(CARD_RED_MTL);
        assert_eq!(
            merge(&[(&bad, &card_red)]),
            Err(MergeError::Other(ModelError::BadReference {
                what: "vertex",
                offset: 999
            }))
        );
    }
}
