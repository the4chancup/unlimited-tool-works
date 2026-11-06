//! The IR's internal consistency: every index and every attribute length. Exporters and ops
//! assume a validated model, so the importers and any op that rebuilds indices call
//! [`validate`] before returning.

use super::CanonicalModel;

/// Why a `CanonicalModel` is not internally consistent. Exporters and ops assume a validated
/// model, so the importers and any op that rebuilds indices call `validate` before returning.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ValidationError {
    /// A bone's parent index is past the list or not earlier than the bone itself.
    #[error("bone {bone}'s parent {parent} is out of range or not earlier in the list")]
    BoneParent {
        /// The bone's index.
        bone: usize,
        /// The offending parent index.
        parent: usize,
    },
    /// A face references a vertex the mesh does not have.
    #[error("mesh {mesh}: face index {index} is out of range ({vertices} vertices)")]
    FaceIndex {
        /// The mesh's index.
        mesh: usize,
        /// The offending face index.
        index: u16,
        /// The mesh's vertex count.
        vertices: usize,
    },
    /// A bone group entry references a bone the model does not have.
    #[error("mesh {mesh}: bone group entry {bone} is out of range ({bones} bones)")]
    BoneGroup {
        /// The mesh's index.
        mesh: usize,
        /// The offending group entry.
        bone: usize,
        /// The model's bone count.
        bones: usize,
    },
    /// A mesh references a material the model does not have.
    #[error("mesh {mesh}: material {material} is out of range ({materials} materials)")]
    Material {
        /// The mesh's index.
        mesh: usize,
        /// The offending material index.
        material: usize,
        /// The model's material count.
        materials: usize,
    },
    /// A per-vertex attribute's length differs from the vertex count.
    #[error("mesh {mesh}: attribute {attribute} has {len} entries for {vertices} vertices")]
    AttributeLength {
        /// The mesh's index.
        mesh: usize,
        /// Which attribute (`"normals"`, `"uvs[0]"`, `"uv_high_precision"`, ...).
        attribute: &'static str,
        /// The attribute's actual length.
        len: usize,
        /// The expected length (`positions.len()`, or `uvs.len()` for `uv_high_precision`).
        vertices: usize,
    },
    /// A vertex's weighted bone slot points past the mesh's bone group (unweighted slots are
    /// not checked: the game ignores them and Konami files leave stale indices there).
    #[error("mesh {mesh}: vertex {vertex} bone slot {slot} is past the bone group ({group} bones)")]
    BoneSlot {
        /// The mesh's index.
        mesh: usize,
        /// The vertex's index.
        vertex: usize,
        /// The offending bone index value.
        slot: u8,
        /// The bone group's length.
        group: usize,
    },
    /// A weight component is not a finite, non-negative value — community exports carry
    /// unnormalized and above-1 weights, which render as stored and are kept.
    #[error(
        "mesh {mesh}: vertex {vertex} weight slot {slot} must be a finite, non-negative weight"
    )]
    BoneWeightRange {
        /// The mesh's index.
        mesh: usize,
        /// The vertex's index.
        vertex: usize,
        /// The offending slot.
        slot: usize,
    },
    /// Only one half of the skinning pair (`bone_indices`/`bone_weights`) is present.
    #[error("mesh {mesh}: bone indices and weights must both be present or both absent")]
    SkinHalf {
        /// The mesh's index.
        mesh: usize,
    },
    /// A mesh group's parent index is past the list or not earlier than the group itself.
    #[error("mesh group {group}'s parent {parent} is out of range or not earlier in the list")]
    GroupParent {
        /// The group's index.
        group: usize,
        /// The offending parent index.
        parent: usize,
    },
    /// A mesh group lists a mesh the model does not have.
    #[error("mesh group {group} lists mesh {mesh}, which is out of range ({meshes} meshes)")]
    GroupMesh {
        /// The group's index.
        group: usize,
        /// The offending mesh index.
        mesh: usize,
        /// The model's mesh count.
        meshes: usize,
    },
    /// A mesh belongs to no group or to several; exactly one is required.
    #[error("mesh {mesh} is listed by {count} mesh groups; exactly one is required")]
    GroupMembership {
        /// The mesh's index.
        mesh: usize,
        /// How many groups list it.
        count: usize,
    },
    /// A material references a texture the model does not have.
    #[error("material {material}: texture {texture} is out of range ({textures} textures)")]
    MaterialTexture {
        /// The material's index.
        material: usize,
        /// The offending texture index.
        texture: usize,
        /// The model's texture count.
        textures: usize,
    },
    /// Two bones carry the same name.
    #[error("bone name {name:?} is used twice")]
    DuplicateBoneName {
        /// The repeated name.
        name: String,
    },
}

/// Checks every index and length invariant of `model`; the first violation, in model order
/// (bones, then meshes, then groups, then materials).
pub fn validate(model: &CanonicalModel) -> Result<(), ValidationError> {
    let mut names = std::collections::BTreeSet::new();
    for (bone, item) in model.bones.iter().enumerate() {
        if !names.insert(&item.name) {
            return Err(ValidationError::DuplicateBoneName {
                name: item.name.clone(),
            });
        }
        if let Some(parent) = item.parent
            && parent >= bone
        {
            return Err(ValidationError::BoneParent { bone, parent });
        }
    }
    for (mesh, item) in model.meshes.iter().enumerate() {
        let vertices = item.vertices.len();
        for face in &item.faces {
            for index in face {
                if usize::from(*index) >= vertices {
                    return Err(ValidationError::FaceIndex {
                        mesh,
                        index: *index,
                        vertices,
                    });
                }
            }
        }
        for bone in &item.bone_group {
            if *bone >= model.bones.len() {
                return Err(ValidationError::BoneGroup {
                    mesh,
                    bone: *bone,
                    bones: model.bones.len(),
                });
            }
        }
        if item.material >= model.materials.len() {
            return Err(ValidationError::Material {
                mesh,
                material: item.material,
                materials: model.materials.len(),
            });
        }
        let attributes: [(&'static str, Option<usize>); 6] = [
            ("normals", item.vertices.normals.as_ref().map(Vec::len)),
            ("tangents", item.vertices.tangents.as_ref().map(Vec::len)),
            (
                "bitangents",
                item.vertices.bitangents.as_ref().map(Vec::len),
            ),
            ("colors", item.vertices.colors.as_ref().map(Vec::len)),
            (
                "bone_indices",
                item.vertices.bone_indices.as_ref().map(Vec::len),
            ),
            (
                "bone_weights",
                item.vertices.bone_weights.as_ref().map(Vec::len),
            ),
        ];
        for (attribute, len) in attributes {
            if let Some(len) = len
                && len != vertices
            {
                return Err(ValidationError::AttributeLength {
                    mesh,
                    attribute,
                    len,
                    vertices,
                });
            }
        }
        for uvs in &item.vertices.uvs {
            if uvs.len() != vertices {
                return Err(ValidationError::AttributeLength {
                    mesh,
                    attribute: "uvs",
                    len: uvs.len(),
                    vertices,
                });
            }
        }
        if item.vertices.uv_high_precision.len() != item.vertices.uvs.len() {
            return Err(ValidationError::AttributeLength {
                mesh,
                attribute: "uv_high_precision",
                len: item.vertices.uv_high_precision.len(),
                vertices: item.vertices.uvs.len(),
            });
        }
        match (&item.vertices.bone_indices, &item.vertices.bone_weights) {
            (Some(_), None) | (None, Some(_)) => {
                return Err(ValidationError::SkinHalf { mesh });
            }
            (Some(indices), Some(weights)) => {
                for (vertex, row) in weights.iter().enumerate() {
                    for (slot, weight) in row.iter().enumerate() {
                        if !weight.is_finite() || *weight < 0.0 {
                            return Err(ValidationError::BoneWeightRange { mesh, vertex, slot });
                        }
                    }
                }
                // Only weighted slots matter: the game ignores an unweighted slot, and Konami
                // files leave stale indices in them.
                for (vertex, (row, weights)) in indices.iter().zip(weights).enumerate() {
                    for (slot, weight) in row.iter().zip(weights) {
                        if *weight > 0.0 && usize::from(*slot) >= item.bone_group.len() {
                            return Err(ValidationError::BoneSlot {
                                mesh,
                                vertex,
                                slot: *slot,
                                group: item.bone_group.len(),
                            });
                        }
                    }
                }
            }
            (None, None) => {}
        }
    }
    for (group, item) in model.mesh_groups.iter().enumerate() {
        if let Some(parent) = item.parent
            && parent >= group
        {
            return Err(ValidationError::GroupParent { group, parent });
        }
        for mesh in &item.meshes {
            if *mesh >= model.meshes.len() {
                return Err(ValidationError::GroupMesh {
                    group,
                    mesh: *mesh,
                    meshes: model.meshes.len(),
                });
            }
        }
    }
    for (mesh, _) in model.meshes.iter().enumerate() {
        let count = model
            .mesh_groups
            .iter()
            .filter(|group| group.meshes.contains(&mesh))
            .count();
        if count != 1 {
            return Err(ValidationError::GroupMembership { mesh, count });
        }
    }
    for (material, item) in model.materials.iter().enumerate() {
        let indices = item
            .textures
            .iter()
            .map(|(_, texture)| *texture)
            .chain(
                item.fox
                    .iter()
                    .flat_map(|fox| fox.textures.iter().map(|(_, texture)| *texture)),
            )
            .chain(
                item.prefox
                    .iter()
                    .flat_map(|prefox| prefox.textures.iter().map(|(_, texture)| *texture)),
            );
        for texture in indices {
            if texture >= model.textures.len() {
                return Err(ValidationError::MaterialTexture {
                    material,
                    texture,
                    textures: model.textures.len(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::affine::Affine;
    use crate::ir::{Bone, Mesh, MeshGroup, SourceFormat, Texture, Vertices};
    use crate::materials::{Material, MaterialFamily, TextureRole};

    /// A minimal valid model: two bones, two one-triangle meshes (one skinned, one not),
    /// one material on one texture, one root group holding both meshes.
    fn valid() -> CanonicalModel {
        let bone = |name: &str, parent: Option<usize>| Bone {
            name: name.to_string(),
            parent,
            matrix: Affine::IDENTITY,
            global_position: None,
            local_position: None,
            bounding_box: None,
        };
        let vertices = |skinned: bool| Vertices {
            positions: vec![[0.0, 0.0, 0.0]; 3],
            normals: None,
            tangents: None,
            bitangents: None,
            colors: None,
            uvs: vec![vec![[0.0, 0.0]; 3]],
            uv_high_precision: vec![false],
            bone_indices: skinned.then(|| vec![[0, 1, 0, 0]; 3]),
            bone_weights: skinned.then(|| vec![[0.5, 0.5, 0.0, 0.0]; 3]),
            bone_weight_width: None,
        };
        let mesh = |skinned: bool, group: Vec<usize>| Mesh {
            vertices: vertices(skinned),
            faces: vec![[0, 1, 2]],
            bone_group: group,
            material: 0,
            extension_headers: Default::default(),
            custom_bounding_box: None,
        };
        CanonicalModel {
            bones: vec![bone("sk_belly", None), bone("sk_chest", Some(0))],
            meshes: vec![mesh(true, vec![0, 1]), mesh(false, vec![])],
            mesh_groups: vec![MeshGroup {
                name: "body".to_string(),
                parent: None,
                meshes: vec![0, 1],
                visible: true,
            }],
            materials: vec![Material {
                name: "body".to_string(),
                family: MaterialFamily::Shaded,
                two_sided: None,
                transparent: None,
                textures: vec![(TextureRole::Base, 0)],
                parameters: vec![],
                fox: None,
                prefox: None,
            }],
            textures: vec![Texture {
                directory: "./".to_string(),
                file_name: "body.dds".to_string(),
            }],
            extension_headers: Default::default(),
            source_format: SourceFormat::PreFox,
        }
    }

    #[test]
    fn the_minimal_model_validates() {
        assert_eq!(validate(&valid()), Ok(()));
        assert!(Vertices::default().is_empty());
        assert!(!valid().meshes[0].vertices.is_empty());
    }

    #[test]
    fn bone_parent_must_be_earlier() {
        let mut model = valid();
        model.bones[1].parent = Some(5);
        assert_eq!(
            validate(&model),
            Err(ValidationError::BoneParent { bone: 1, parent: 5 })
        );
        model.bones[1].parent = Some(1);
        assert_eq!(
            validate(&model),
            Err(ValidationError::BoneParent { bone: 1, parent: 1 })
        );
    }

    #[test]
    fn face_index_in_range() {
        let mut model = valid();
        model.meshes[0].faces[0] = [0, 1, 9];
        assert_eq!(
            validate(&model),
            Err(ValidationError::FaceIndex {
                mesh: 0,
                index: 9,
                vertices: 3
            })
        );
    }

    #[test]
    fn bone_group_entries_in_range() {
        let mut model = valid();
        model.meshes[0].bone_group.push(7);
        assert_eq!(
            validate(&model),
            Err(ValidationError::BoneGroup {
                mesh: 0,
                bone: 7,
                bones: 2
            })
        );
    }

    #[test]
    fn mesh_material_in_range() {
        let mut model = valid();
        model.meshes[0].material = 3;
        assert_eq!(
            validate(&model),
            Err(ValidationError::Material {
                mesh: 0,
                material: 3,
                materials: 1
            })
        );
    }

    #[test]
    fn attribute_lengths_match_vertices() {
        let mut model = valid();
        model.meshes[0].vertices.normals = Some(vec![[0.0, 0.0, 1.0, 1.0]; 2]);
        assert_eq!(
            validate(&model),
            Err(ValidationError::AttributeLength {
                mesh: 0,
                attribute: "normals",
                len: 2,
                vertices: 3
            })
        );
        let mut model = valid();
        model.meshes[0].vertices.uv_high_precision = vec![false, false];
        assert_eq!(
            validate(&model),
            Err(ValidationError::AttributeLength {
                mesh: 0,
                attribute: "uv_high_precision",
                len: 2,
                vertices: 1
            })
        );
    }

    #[test]
    fn weights_are_finite_and_non_negative() {
        // Community exports carry sums far from 1; they render as stored.
        let mut model = valid();
        model.meshes[0].vertices.bone_weights = Some(vec![
            [0.7, 0.0, 0.0, 0.0],
            [1.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
        ]);
        assert_eq!(validate(&model), Ok(()));
        // A lane above 1 is kept.
        model.meshes[0].vertices.bone_weights = Some(vec![
            [0.5, 0.5, 0.0, 0.0],
            [1.5, 0.0, 0.0, 0.0],
            [0.5, 0.5, 0.0, 0.0],
        ]);
        assert_eq!(validate(&model), Ok(()));
        // What is rejected: a negative or non-finite lane.
        model.meshes[0].vertices.bone_weights = Some(vec![
            [0.5, 0.5, 0.0, 0.0],
            [0.5, 0.5, 0.0, 0.0],
            [-0.5, 1.5, 0.0, 0.0],
        ]);
        assert_eq!(
            validate(&model),
            Err(ValidationError::BoneWeightRange {
                mesh: 0,
                vertex: 2,
                slot: 0
            })
        );
        model.meshes[0].vertices.bone_weights = Some(vec![
            [0.5, 0.5, 0.0, 0.0],
            [0.5, 0.5, 0.0, 0.0],
            [f32::NAN, 1.0, 0.0, 0.0],
        ]);
        assert_eq!(
            validate(&model),
            Err(ValidationError::BoneWeightRange {
                mesh: 0,
                vertex: 2,
                slot: 0
            })
        );
        model.meshes[0].vertices.bone_weights = Some(vec![
            [0.5, 0.5, 0.0, 0.0],
            [0.5, 0.5, 0.0, 0.0],
            [0.5, 0.5, 0.0, f32::INFINITY],
        ]);
        assert_eq!(
            validate(&model),
            Err(ValidationError::BoneWeightRange {
                mesh: 0,
                vertex: 2,
                slot: 3
            })
        );
    }

    #[test]
    fn bone_slots_stay_inside_the_group() {
        let mut model = valid();
        model.meshes[0].vertices.bone_indices =
            Some(vec![[0, 1, 0, 0], [0, 1, 0, 0], [0, 1, 5, 0]]);
        // A stale index in an unweighted slot is not an error.
        assert_eq!(validate(&model), Ok(()));
        model.meshes[0].vertices.bone_weights = Some(vec![
            [0.5, 0.5, 0.0, 0.0],
            [0.5, 0.5, 0.0, 0.0],
            [0.5, 0.0, 0.5, 0.0],
        ]);
        assert_eq!(
            validate(&model),
            Err(ValidationError::BoneSlot {
                mesh: 0,
                vertex: 2,
                slot: 5,
                group: 2
            })
        );
    }

    #[test]
    fn skinning_is_both_halves_or_neither() {
        let mut model = valid();
        model.meshes[1].vertices.bone_weights = Some(vec![[0.0; 4]; 3]);
        assert_eq!(validate(&model), Err(ValidationError::SkinHalf { mesh: 1 }));
    }

    #[test]
    fn group_parent_must_be_earlier() {
        let mut model = valid();
        model.mesh_groups[0].parent = Some(0);
        assert_eq!(
            validate(&model),
            Err(ValidationError::GroupParent {
                group: 0,
                parent: 0
            })
        );
    }

    #[test]
    fn group_meshes_in_range() {
        let mut model = valid();
        model.mesh_groups[0].meshes = vec![0, 9];
        assert_eq!(
            validate(&model),
            Err(ValidationError::GroupMesh {
                group: 0,
                mesh: 9,
                meshes: 2
            })
        );
    }

    #[test]
    fn every_mesh_in_exactly_one_group() {
        let mut model = valid();
        model.mesh_groups[0].meshes = vec![0];
        assert_eq!(
            validate(&model),
            Err(ValidationError::GroupMembership { mesh: 1, count: 0 })
        );
        let mut model = valid();
        model.mesh_groups.push(MeshGroup {
            name: "extra".to_string(),
            parent: Some(0),
            meshes: vec![0],
            visible: true,
        });
        assert_eq!(
            validate(&model),
            Err(ValidationError::GroupMembership { mesh: 0, count: 2 })
        );
    }

    #[test]
    fn material_textures_in_range() {
        let mut model = valid();
        model.materials[0].textures[0].1 = 5;
        assert_eq!(
            validate(&model),
            Err(ValidationError::MaterialTexture {
                material: 0,
                texture: 5,
                textures: 1
            })
        );
        let mut model = valid();
        model.materials[0].prefox = Some(crate::materials::PreFoxMaterial {
            shader: "Basic_C".to_string(),
            states: vec![],
            samplers: vec![],
            textures: vec![("RoughnessMap".to_string(), 9)],
            parameters: vec![],
        });
        assert_eq!(
            validate(&model),
            Err(ValidationError::MaterialTexture {
                material: 0,
                texture: 9,
                textures: 1
            })
        );
    }

    #[test]
    fn every_weight_is_finite_and_non_negative() {
        // A lane above 1 is legal; a negative one is not — 2.0 passes, -1.0 is
        // the offending slot.
        let mut model = valid();
        model.meshes[0].vertices.bone_weights = Some(vec![
            [0.5, 0.5, 0.0, 0.0],
            [2.0, -1.0, 0.0, 0.0],
            [0.5, 0.5, 0.0, 0.0],
        ]);
        assert_eq!(
            validate(&model),
            Err(ValidationError::BoneWeightRange {
                mesh: 0,
                vertex: 1,
                slot: 1
            })
        );
        let mut model = valid();
        model.meshes[0].vertices.bone_weights = Some(vec![
            [0.5, 0.5, 0.0, 0.0],
            [f32::NAN, 0.0, 0.0, 0.0],
            [0.5, 0.5, 0.0, 0.0],
        ]);
        assert_eq!(
            validate(&model),
            Err(ValidationError::BoneWeightRange {
                mesh: 0,
                vertex: 1,
                slot: 0
            })
        );
    }

    #[test]
    fn bone_names_are_unique() {
        let mut model = valid();
        model.bones[1].name = "sk_belly".to_string();
        assert_eq!(
            validate(&model),
            Err(ValidationError::DuplicateBoneName {
                name: "sk_belly".to_string()
            })
        );
    }
}
