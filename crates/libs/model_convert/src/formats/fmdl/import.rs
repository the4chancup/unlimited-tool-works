//! The FMDL → IR half: `fmdl_to_ir`.

use std::collections::HashMap;

use crate::affine::Affine;
use crate::ir::{
    Bone, BoundingBox, CanonicalModel, Material, Mesh, MeshGroup, SourceFormat, Texture, Vertices,
    validate,
};
use crate::loss::{Finding, Subject};
use crate::materials::{FoxMaterial, TextureRole, family, to_fox};

use super::super::{ConvertError, drop_out_of_group_slots};
use super::{Imported, template_matrix};

/// FMDL → IR. Decodes the format extensions (mesh splitting, anti-blur) on a copy, reads the
/// bind pose from `skl` (bones matched by name) or PES21's template tables, lifts the
/// per-mesh flags to the materials, and validates the result.
pub fn fmdl_to_ir(
    model: &::fmdl::Model,
    skl: Option<&::fmdl::SklFile>,
) -> Result<Imported, ConvertError> {
    let mut model = model.clone();
    // Reassemble split meshes first, then strip the anti-blur duplicates.
    ::fmdl::ops::split::decode(&mut model)?;
    ::fmdl::ops::antiblur::decode(&mut model)?;
    let model = &model;

    let mut findings = Vec::new();
    // The FMDL's local-space bone matrices are redundant with the SKL/table pose and are
    // not carried into the IR.
    if model.bone_matrices.is_some() {
        findings.push(Finding {
            code: "native_field_dropped",
            subject: Subject::Model,
            detail: "bone_matrices".to_string(),
        });
    }

    // Parent-first order, as `.model` bones already arrive, since every IR consumer relies
    // on it. A bone listed before its parent moves right after it, file order otherwise.
    // Every bone index (parents, bone groups) remaps. `decode` validated the parents' range
    // and acyclicity, so an `Err` here cannot come from a file the crate read.
    let fmdl_parents: Vec<Option<usize>> = model.bones.iter().map(|bone| bone.parent).collect();
    let order = crate::ir::parent_first_order(&fmdl_parents)
        .map_err(|_| ConvertError::Fmdl(::fmdl::FmdlError::ParentCycle("bone")))?;
    let mut new_index = vec![0usize; model.bones.len()];
    for (new, &old) in order.iter().enumerate() {
        new_index[old] = new;
    }

    let mut bones = Vec::with_capacity(model.bones.len());
    for (index, &old) in order.iter().enumerate() {
        let bone = &model.bones[old];
        let skl_bone =
            skl.and_then(|skl| skl.bones.iter().find(|skl_bone| skl_bone.name == bone.name));
        let fmdl_parent = bone.parent.map(|parent| model.bones[parent].name.as_str());
        if let (Some(skl), Some(skl_bone)) = (skl, skl_bone) {
            // The FMDL's own parent wins; an SKL disagreeing loses a field.
            let skl_parent = skl_bone
                .parent
                .map(|parent| {
                    skl.bones
                        .get(parent)
                        .map(|bone| bone.name.as_str())
                        .ok_or(ConvertError::Fmdl(::fmdl::FmdlError::BadReference {
                            what: "skl parent",
                            index: parent,
                        }))
                })
                .transpose()?;
            if skl_parent != fmdl_parent {
                findings.push(Finding {
                    code: "native_field_dropped",
                    subject: Subject::Bone(index),
                    detail: "skl_parent".to_string(),
                });
            }
        }
        let matrix = skl_bone
            .map(|skl_bone| {
                Affine::from_rotation_translation(skl_bone.rotation, skl_bone.translation)
            })
            .or_else(|| template_matrix(&bone.name))
            .unwrap_or_else(|| {
                findings.push(Finding {
                    code: "bone_matrix_unknown",
                    subject: Subject::Bone(index),
                    detail: bone.name.clone(),
                });
                Affine::IDENTITY
            });
        bones.push(Bone {
            name: bone.name.clone(),
            parent: bone.parent.map(|parent| new_index[parent]),
            matrix,
            global_position: Some(bone.world_position),
            local_position: Some(bone.local_position),
            bounding_box: Some(BoundingBox {
                min: bone.bounding_box.min,
                max: bone.bounding_box.max,
            }),
        });
    }

    // Every texture the materials reference, deduped in first-occurrence order.
    let mut textures = Vec::new();
    let mut texture_index: HashMap<(&str, &str), usize> = HashMap::new();
    for instance in &model.materials {
        for (_, texture) in &instance.textures {
            let key = (texture.directory.as_str(), texture.file_name.as_str());
            if let std::collections::hash_map::Entry::Vacant(entry) = texture_index.entry(key) {
                entry.insert(textures.len());
                textures.push(Texture {
                    directory: texture.directory.clone(),
                    file_name: texture.file_name.clone(),
                });
            }
        }
    }
    let texture_of = |texture: &::fmdl::Texture| -> usize {
        texture_index[&(texture.directory.as_str(), texture.file_name.as_str())]
    };

    // One IR material per (instance, flags) combination; the first keeps the instance's
    // name, the rest are `name_2`, `name_3`, ....
    let mut materials = Vec::new();
    // Per instance: the IR material index each flag combination maps to.
    let mut instance_materials: Vec<Vec<(u8, u8, usize)>> = Vec::new();
    for (instance_index, instance) in model.materials.iter().enumerate() {
        let mut combinations: Vec<(u8, u8)> = Vec::new();
        for mesh in &model.meshes {
            if mesh.material == instance_index {
                let flags = (mesh.alpha_flags, mesh.shadow_flags);
                if !combinations.contains(&flags) {
                    combinations.push(flags);
                }
            }
        }
        if combinations.is_empty() {
            combinations.push((0, 0));
        }
        let mut entries = Vec::new();
        for (split, (alpha_flags, shadow_flags)) in combinations.iter().enumerate() {
            // The first `name_N` free among the instance names and the ones generated so
            // far — `mat` splitting while `mat_2` exists must not collide.
            let name = if split == 0 {
                instance.name.clone()
            } else {
                let mut n = 2;
                loop {
                    let candidate = format!("{}_{}", instance.name, n);
                    let taken = model.materials.iter().any(|other| other.name == candidate)
                        || materials.iter().any(|m: &Material| m.name == candidate);
                    if !taken {
                        break candidate;
                    }
                    n += 1;
                }
            };
            if split > 0 {
                findings.push(Finding {
                    code: "material_split_by_flags",
                    subject: Subject::Material(materials.len()),
                    detail: name.clone(),
                });
            }
            let inferred = family::from_fox_shader(&instance.shader);
            if inferred.approximate {
                findings.push(Finding {
                    code: "material_family_approximated",
                    subject: Subject::Material(materials.len()),
                    detail: name.clone(),
                });
            }
            let mut canonical: Vec<(TextureRole, usize)> = Vec::new();
            let mut native: Vec<(String, usize)> = Vec::new();
            let mut base_linear = false;
            for (sampler, texture) in &instance.textures {
                if sampler == "Base_Tex_LIN" {
                    base_linear = true;
                }
                // The game's dummy normal and specular maps stand for "no map": the role
                // stays unset, so pre-Fox writes no sampler for it, and the sampler stays
                // native, so a Fox export writes it back as the file had it (a material
                // with a `fox` table gets no family dummies).
                let dummy = super::is_game_dummy(&texture.directory, &texture.file_name);
                match to_fox::role_for_sampler(sampler) {
                    // The first sampler a role gets wins; a second `Base_Tex_*` is native.
                    Some(role) if !dummy && !canonical.iter().any(|(seen, _)| *seen == role) => {
                        canonical.push((role, texture_of(texture)));
                    }
                    _ => native.push((sampler.clone(), texture_of(texture))),
                }
            }
            materials.push(Material {
                name,
                family: inferred.family,
                two_sided: Some(alpha_flags & 32 != 0),
                transparent: None,
                textures: canonical,
                parameters: vec![],
                fox: Some(FoxMaterial {
                    shader: instance.shader.clone(),
                    technique: instance.technique.clone(),
                    alpha_flags: *alpha_flags,
                    shadow_flags: *shadow_flags,
                    cast_shadow: None,
                    invisible: None,
                    base_linear,
                    textures: native,
                    parameters: instance.parameters.clone(),
                }),
                prefox: None,
            });
            entries.push((*alpha_flags, *shadow_flags, materials.len() - 1));
        }
        instance_materials.push(entries);
    }

    let mut meshes = Vec::with_capacity(model.meshes.len());
    for (index, mesh) in model.meshes.iter().enumerate() {
        let vertices = &mesh.vertices;
        let mut bone_weights: Option<Vec<[f32; 4]>> = vertices.bone_weights.as_ref().map(|rows| {
            rows.iter()
                .map(|row| row.map(|w| w as f32 / 255.0))
                .collect()
        });
        let dropped = match (&vertices.bone_indices, bone_weights.as_mut()) {
            (Some(indices), Some(weights)) => {
                drop_out_of_group_slots(indices, weights, mesh.bone_group.len())
            }
            _ => 0,
        };
        if dropped > 0 {
            findings.push(Finding {
                code: "bone_slot_dropped",
                subject: Subject::Mesh(index),
                detail: dropped.to_string(),
            });
        }
        let material = instance_materials[mesh.material]
            .iter()
            .find(|(alpha, shadow, _)| *alpha == mesh.alpha_flags && *shadow == mesh.shadow_flags)
            .map(|(_, _, material)| *material)
            .expect("every mesh's flag combination was collected");
        meshes.push(Mesh {
            vertices: Vertices {
                positions: vertices.positions.clone(),
                normals: vertices.normals.clone(),
                tangents: vertices.tangents.clone(),
                bitangents: None,
                colors: vertices.colors.clone(),
                uvs: vertices.uvs.clone(),
                uv_high_precision: vertices.uv_high_precision.clone(),
                bone_indices: vertices.bone_indices.clone(),
                bone_weights,
                bone_weight_width: None,
            },
            faces: mesh.faces.clone(),
            bone_group: mesh
                .bone_group
                .iter()
                .map(|bone| new_index[*bone])
                .collect(),
            material,
            extension_headers: Default::default(),
            custom_bounding_box: mesh.custom_bounding_box.map(|b| BoundingBox {
                min: b.min,
                max: b.max,
            }),
        });
    }

    let ir = CanonicalModel {
        bones,
        meshes,
        mesh_groups: model
            .mesh_groups
            .iter()
            .map(|group| MeshGroup {
                name: group.name.clone(),
                parent: group.parent,
                meshes: group.meshes.clone(),
                visible: group.visible,
            })
            .collect(),
        materials,
        textures,
        extension_headers: model.extensions.other.iter().cloned().collect(),
        source_format: SourceFormat::Fox,
    };
    validate(&ir)?;
    Ok(Imported {
        model: ir,
        findings,
    })
}
