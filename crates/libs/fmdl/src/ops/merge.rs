//! Multi-FMDL merge: the Team compiler assembles several part models into
//! one model. Bones are unioned by name (per-mesh bone groups remapped to
//! the unioned indices — per-vertex `bone_indices` index the group and do
//! not change), materials merged by name with equal-definition checking,
//! meshes and mesh groups concatenated in caller-supplied part order, and
//! extension flags OR-ed. `parts` order is canonical and preserved, so the
//! result is deterministic by construction.

use std::collections::{HashMap, HashSet};

use crate::format::FmdlError;
use crate::model::{Mesh, MeshGroup, Model};

/// Why several parts could not be merged into one model.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MergeError {
    /// Two parts define a material instance of this name differently.
    #[error("material instance {name} is defined differently in two parts")]
    MaterialConflict {
        /// The conflicting material name.
        name: String,
    },
    /// A bone present in several parts has different positions or a
    /// different parent name.
    #[error("bone {name} differs between parts (position or parent)")]
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
    /// A part fails `Model::validate`.
    #[error(transparent)]
    Other(#[from] FmdlError),
}

/// Merges `parts` (caller-supplied canonical order, preserved) into one
/// model.
pub fn merge(parts: &[Model]) -> Result<Model, MergeError> {
    let mut output = Model {
        bones: Vec::new(),
        materials: Vec::new(),
        meshes: Vec::new(),
        mesh_groups: Vec::new(),
        extensions: crate::model::Extensions::default(),
        bone_matrices: None,
    };

    let mut bone_of: HashMap<&str, usize> = HashMap::new();
    let mut material_of: HashMap<&str, usize> = HashMap::new();
    // bone name -> (part index, bone index) that first defined it, for the
    // bone-matrices union.
    let mut bone_source: Vec<(usize, usize)> = Vec::new();
    let mut matrices_ok = true;

    for (part_index, part) in parts.iter().enumerate() {
        part.validate()?;

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
        // agree on positions and on the parent's *name*.
        let mut bone_remap = vec![usize::MAX; part.bones.len()];
        for (index, bone) in part.bones.iter().enumerate() {
            match bone_of.get(bone.name.as_str()) {
                Some(&unioned) => {
                    let existing = &output.bones[unioned];
                    let same_parent = match (existing.parent, bone.parent) {
                        (None, None) => true,
                        (Some(a), Some(b)) => output.bones[a].name == part.bones[b].name,
                        _ => false,
                    };
                    if existing.local_position != bone.local_position
                        || existing.world_position != bone.world_position
                        || !same_parent
                    {
                        return Err(MergeError::SkeletonConflict {
                            name: bone.name.clone(),
                        });
                    }
                    // A bone's box covers every vertex weighted to it: the
                    // unioned bone's box is the union (w is unused — keep
                    // the first part's).
                    let bounding_box = &mut output.bones[unioned].bounding_box;
                    for axis in 0..3 {
                        bounding_box.max[axis] =
                            bounding_box.max[axis].max(bone.bounding_box.max[axis]);
                        bounding_box.min[axis] =
                            bounding_box.min[axis].min(bone.bounding_box.min[axis]);
                    }
                    bone_remap[index] = unioned;
                }
                None => {
                    let unioned = output.bones.len();
                    bone_of.insert(&bone.name, unioned);
                    bone_source.push((part_index, index));
                    // The parent name is resolved below; store the index
                    // into the unioned list via the name map once all of
                    // this part's bones are placed.
                    output.bones.push(bone.clone());
                    bone_remap[index] = unioned;
                }
            }
        }
        // Parent indices of newly added bones still point at the part's
        // bone list; remap them to unioned indices.
        for (index, remap) in bone_remap.iter().enumerate() {
            let unioned = *remap;
            if let Some(parent) = part.bones[index].parent {
                output.bones[unioned].parent = Some(bone_remap[parent]);
            }
        }

        if !part.bones.is_empty() {
            match &part.bone_matrices {
                Some(bytes) if bytes.len() == 64 * part.bones.len() => {}
                _ => matrices_ok = false,
            }
        }

        // Material union: same name must mean an equal definition.
        let mut material_remap = vec![usize::MAX; part.materials.len()];
        for (index, material) in part.materials.iter().enumerate() {
            match material_of.get(material.name.as_str()) {
                Some(&unioned) => {
                    if output.materials[unioned] != *material {
                        return Err(MergeError::MaterialConflict {
                            name: material.name.clone(),
                        });
                    }
                    material_remap[index] = unioned;
                }
                None => {
                    material_remap[index] = output.materials.len();
                    material_of.insert(&material.name, output.materials.len());
                    output.materials.push(material.clone());
                }
            }
        }

        let mesh_offset = output.meshes.len();
        for mesh in &part.meshes {
            let mut mesh: Mesh = mesh.clone();
            mesh.material = material_remap[mesh.material];
            for bone in &mut mesh.bone_group {
                *bone = bone_remap[*bone];
            }
            output.meshes.push(mesh);
        }

        let group_offset = output.mesh_groups.len();
        for group in &part.mesh_groups {
            let mut group: MeshGroup = group.clone();
            group.parent = group.parent.map(|parent| parent + group_offset);
            group.meshes = group.meshes.iter().map(|mesh| mesh + mesh_offset).collect();
            output.mesh_groups.push(group);
        }

        output.extensions.mesh_splitting |= part.extensions.mesh_splitting;
        output.extensions.antiblur |= part.extensions.antiblur;
        output.extensions.vertex_loop_preservation |= part.extensions.vertex_loop_preservation;
        for extension in &part.extensions.other {
            if !output.extensions.other.contains(extension) {
                output.extensions.other.push(extension.clone());
            }
        }
    }

    // Bone matrices: one 64-byte matrix per unioned bone, taken from the
    // first part that defines the bone; `None` unless every boned part
    // carried a correctly sized block.
    if matrices_ok && !bone_source.is_empty() {
        let mut bytes = vec![0u8; 64 * bone_source.len()];
        for (slot, &(part_index, bone_index)) in bone_source.iter().enumerate() {
            let source = &parts[part_index].bone_matrices;
            if let Some(source) = source {
                bytes[64 * slot..64 * slot + 64]
                    .copy_from_slice(&source[64 * bone_index..64 * bone_index + 64]);
            }
        }
        output.bone_matrices = Some(bytes);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::FmdlFile;
    use crate::model::BoundingBox;

    const HIGHNECK: &[u8] = include_bytes!("../../tests/fixtures/konami_highneck.fmdl");
    const MOUTH: &[u8] = include_bytes!("../../tests/fixtures/konami_mouth.fmdl");
    const ORAL: &[u8] = include_bytes!("../../tests/fixtures/addon_oral.fmdl");
    const PLACEHOLDER: &[u8] = include_bytes!("../../tests/fixtures/addon_placeholder.fmdl");

    fn model(bytes: &[u8]) -> Model {
        Model::from_file(&FmdlFile::read(bytes).unwrap()).unwrap()
    }

    // G1
    #[test]
    fn merge_two_parts() {
        let highneck = model(HIGHNECK);
        let mouth = model(MOUTH);
        // mouth's `sk_head` is a root; highneck's parents `sk_neck`.
        assert_eq!(
            merge(&[highneck.clone(), mouth]),
            Err(MergeError::SkeletonConflict {
                name: "sk_head".to_owned()
            })
        );

        let mut extra = highneck.clone();
        let scm = extra
            .bones
            .iter()
            .position(|bone| bone.name == "dsk_scm")
            .unwrap();
        extra.bones[scm].name = "dsk_extra".to_owned();
        extra.materials[0].name = "extra_mat".to_owned();
        let merged = merge(&[highneck.clone(), extra]).unwrap();

        assert_eq!(merged.bones.len(), 8);
        assert_eq!(merged.bones[7].name, "dsk_extra");
        assert_eq!(merged.materials.len(), 2);
        assert_eq!(merged.meshes.len(), 2);
        assert_eq!(merged.mesh_groups.len(), 2);
        assert_eq!(merged.mesh_groups[1].meshes, vec![1]);
        // `dsk_extra` is slot `scm` of the second mesh's bone group,
        // remapped to union index 7.
        assert_eq!(merged.meshes[1].bone_group[scm], 7);
        assert_eq!(merged.meshes[1].material, 1);
        // The shared bones and material keep the first part's indices.
        assert_eq!(merged.meshes[0], highneck.meshes[0]);
        // 8 bones -> 8 matrices; the first 7 are highneck's, bone 7's comes
        // from the second part.
        let matrices = merged.bone_matrices.unwrap();
        let source = highneck.bone_matrices.as_ref().unwrap();
        assert_eq!(matrices.len(), 64 * 8);
        assert_eq!(matrices[..64 * 7], source[..]);
        assert_eq!(matrices[64 * 7..], source[64 * scm..64 * scm + 64]);
    }

    // G2
    #[test]
    fn shared_material() {
        let oral = model(ORAL);
        let merged = merge(&[oral.clone(), oral.clone()]).unwrap();
        assert_eq!(merged.materials.len(), 1);
        assert_eq!(merged.meshes[0].material, 0);
        assert_eq!(merged.meshes[1].material, 0);
    }

    // G3
    #[test]
    fn material_conflict() {
        let mut a = model(ORAL);
        a.materials[0].name = "accessory".to_owned();
        let mut b = a.clone();
        b.materials[0].shader = "different".to_owned();
        assert_eq!(
            merge(&[a, b]),
            Err(MergeError::MaterialConflict {
                name: "accessory".to_owned()
            })
        );
    }

    // G4
    #[test]
    fn skeleton_conflict() {
        let highneck = model(HIGHNECK);
        let mut changed = highneck.clone();
        changed.bones[0].local_position[1] += 1.0;
        assert_eq!(
            merge(&[highneck, changed]),
            Err(MergeError::SkeletonConflict {
                name: "sk_chest".to_owned()
            })
        );
    }

    // A bone differing only in world position is a skeleton conflict.
    #[test]
    fn skeleton_conflict_world_position() {
        let highneck = model(HIGHNECK);
        let mut moved = highneck.clone();
        moved.bones[0].world_position[0] += 1.0;
        assert_eq!(
            merge(&[highneck, moved]),
            Err(MergeError::SkeletonConflict {
                name: "sk_chest".to_owned()
            })
        );
    }

    // Group parents re-base by the first part's group count.
    #[test]
    fn group_parents_rebase() {
        let mut part = model(HIGHNECK);
        part.mesh_groups.push(MeshGroup {
            name: "child".to_owned(),
            parent: Some(0),
            meshes: Vec::new(),
            bounding_box: None,
            visible: true,
            split_mesh_group: false,
        });
        let merged = merge(&[model(HIGHNECK), part]).unwrap();
        assert_eq!(merged.mesh_groups.len(), 3);
        assert_eq!(merged.mesh_groups[2].parent, Some(1));
    }

    // The three extension flags OR across parts, either order; an `other`
    // flag both parts carry appears once.
    #[test]
    fn extension_flags_or() {
        let mut a = model(ORAL);
        a.extensions.mesh_splitting = true;
        let b = model(ORAL);
        for parts in [[a.clone(), b.clone()], [b.clone(), a.clone()]] {
            assert!(merge(&parts).unwrap().extensions.mesh_splitting);
        }
        let mut a = model(ORAL);
        a.extensions.antiblur = true;
        let b = model(ORAL);
        for parts in [[a.clone(), b.clone()], [b.clone(), a.clone()]] {
            assert!(merge(&parts).unwrap().extensions.antiblur);
        }
        let mut a = model(ORAL);
        a.extensions.vertex_loop_preservation = true;
        let b = model(ORAL);
        for parts in [[a.clone(), b.clone()], [b, a]] {
            assert!(merge(&parts).unwrap().extensions.vertex_loop_preservation);
        }

        let mut a = model(ORAL);
        a.extensions.other.push("custom".to_owned());
        let mut b = model(ORAL);
        b.extensions.other.push("custom".to_owned());
        let merged = merge(&[a, b]).unwrap();
        assert_eq!(
            merged
                .extensions
                .other
                .iter()
                .filter(|flag| *flag == "custom")
                .count(),
            1
        );
    }

    // G5
    #[test]
    fn file_round_trip() {
        let highneck = model(HIGHNECK);
        let mut extra = highneck.clone();
        extra.bones[3].name = "dsk_extra".to_owned();
        extra.materials[0].name = "extra_mat".to_owned();
        let merged = merge(&[highneck, extra]).unwrap();
        let written = merged.to_file().unwrap();
        let again = Model::from_file(&FmdlFile::read(&written.write()).unwrap()).unwrap();
        assert_eq!(again, merged);
        assert_eq!(again.bones.len(), 8);
    }

    // G6
    #[test]
    fn bone_matrices() {
        let highneck = model(HIGHNECK);
        let placeholder = model(PLACEHOLDER);
        let merged = merge(&[highneck.clone(), placeholder]).unwrap();
        assert_eq!(merged.bone_matrices, highneck.bone_matrices);

        // oral has one bone and `bone_matrices == Some([])`: mis-sized.
        let oral = model(ORAL);
        assert_eq!(oral.bones.len(), 1);
        let merged = merge(&[oral.clone(), oral]).unwrap();
        assert_eq!(merged.bone_matrices, None);
    }

    // G7
    #[test]
    fn single_and_empty() {
        let highneck = model(HIGHNECK);
        assert_eq!(merge(std::slice::from_ref(&highneck)).unwrap(), highneck);
        let empty = merge(&[]).unwrap();
        assert!(empty.bones.is_empty());
        assert!(empty.materials.is_empty());
        assert!(empty.meshes.is_empty());
        assert!(empty.mesh_groups.is_empty());
        assert_eq!(empty.bone_matrices, None);
    }

    #[test]
    fn bone_parent_out_of_range() {
        let mut part = model(ORAL);
        part.bones[0].parent = Some(9);
        assert!(matches!(
            merge(&[part]),
            Err(MergeError::Other(FmdlError::BadReference {
                what: "bone",
                index: 9
            }))
        ));
    }

    // A shared bone's bounding box is the union of the parts' boxes.
    #[test]
    fn a_shared_bones_box_is_the_union() {
        let mut a = model(HIGHNECK);
        let mut b = model(HIGHNECK);
        a.bones[0].bounding_box = BoundingBox {
            max: [2.0, 2.0, 2.0, 1.0],
            min: [-1.0, -1.0, -1.0, 1.0],
        };
        b.bones[0].bounding_box = BoundingBox {
            max: [1.0, 3.0, 1.0, 9.0],
            min: [-2.0, 0.0, -2.0, 9.0],
        };
        let merged = merge(&[a, b]).unwrap();
        assert_eq!(merged.bones[0].bounding_box.max, [2.0, 3.0, 2.0, 1.0]);
        assert_eq!(merged.bones[0].bounding_box.min, [-2.0, -1.0, -2.0, 1.0]);
    }

    /// The smallest part: one mesh of one triangle, the named bones.
    fn part(bones: &[(&str, Option<usize>)]) -> Model {
        Model {
            bones: bones
                .iter()
                .map(|&(name, parent)| crate::model::Bone {
                    name: name.to_owned(),
                    parent,
                    bounding_box: BoundingBox {
                        max: [0.0; 4],
                        min: [0.0; 4],
                    },
                    local_position: [0.0; 4],
                    world_position: [0.0; 4],
                })
                .collect(),
            materials: vec![crate::model::MaterialInstance {
                name: "mat".to_owned(),
                shader: "shader".to_owned(),
                technique: "technique".to_owned(),
                textures: Vec::new(),
                parameters: Vec::new(),
            }],
            meshes: vec![Mesh {
                vertices: crate::format::MeshVertices {
                    positions: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                    ..crate::format::MeshVertices::default()
                },
                faces: vec![[0, 1, 2]],
                bone_group: Vec::new(),
                material: 0,
                alpha_flags: 0,
                shadow_flags: 0,
                has_antiblur_meshes: false,
                is_antiblur_mesh: false,
                custom_bounding_box: None,
            }],
            mesh_groups: vec![MeshGroup {
                name: "group".to_owned(),
                parent: None,
                meshes: vec![0],
                bounding_box: None,
                visible: true,
                split_mesh_group: false,
            }],
            extensions: crate::model::Extensions::default(),
            bone_matrices: None,
        }
    }

    /// ORAL requesting anti-blur without declaring it: `decode` cleared
    /// `extensions.antiblur`, and the mesh's request is set again.
    fn unencoded_oral() -> Model {
        let mut oral = model(ORAL);
        crate::ops::antiblur::decode(&mut oral).unwrap();
        oral.meshes[0].has_antiblur_meshes = true;
        oral
    }

    /// ORAL requesting anti-blur and declaring it: the file's extension
    /// flag set, as written.
    fn encoded_oral() -> Model {
        let mut oral = model(ORAL);
        oral.meshes[0].has_antiblur_meshes = true;
        oral
    }

    #[test]
    fn a_part_with_repeated_bone_names_errors() {
        let part = part(&[("A", Some(2)), ("A", Some(2)), ("P", None)]);
        assert!(matches!(
            merge(&[part]),
            Err(MergeError::DuplicateBoneName { ref name }) if name == "A"
        ));
    }

    #[test]
    fn parts_disagreeing_on_the_anti_blur_flag_merge() {
        // Nothing encodes after a merge, so parts disagreeing on the
        // anti-blur flag merge all the same: every mesh kept as it was
        // and the parts' extension flag OR-ed in.
        let encoded = encoded_oral();
        let unencoded = unencoded_oral();
        let merged = merge(&[encoded.clone(), unencoded.clone()]).unwrap();
        assert_eq!(merged.meshes, [encoded.meshes, unencoded.meshes].concat());
        assert!(merged.extensions.antiblur);
    }

    #[test]
    fn unencoded_requesting_parts_merge_flag_clear() {
        let mut merged = merge(&[unencoded_oral(), unencoded_oral()]).unwrap();
        assert!(!merged.extensions.antiblur);
        // A later encode duplicates the flagged meshes of both parts.
        crate::ops::antiblur::encode(&mut merged).unwrap();
        assert!(merged.extensions.antiblur);
        assert_eq!(
            merged
                .meshes
                .iter()
                .filter(|mesh| mesh.is_antiblur_mesh)
                .count(),
            2
        );
    }

    #[test]
    fn a_set_flag_on_a_non_requesting_part_is_kept() {
        // No mesh of this part asks for anti-blur; its extension flag
        // still ORs into the output, like every other extension's.
        let mut flagged = part(&[]);
        flagged.extensions.antiblur = true;
        let merged = merge(&[flagged, unencoded_oral()]).unwrap();
        assert!(merged.extensions.antiblur);
    }

    // Two genuinely encoded, requesting parts merge with the flag set,
    // and a following encode is a no-op.
    #[test]
    fn encoded_requesting_parts_stay_encoded() {
        let mut merged = merge(&[encoded_oral(), encoded_oral()]).unwrap();
        assert!(merged.extensions.antiblur);
        let before = merged.clone();
        crate::ops::antiblur::encode(&mut merged).unwrap();
        assert_eq!(merged, before);
    }
}
