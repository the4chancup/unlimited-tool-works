//! The `.model` + `.mtl` → IR half: `model_to_ir`.

use std::collections::HashMap;

use ::pes_model::format::mtl;
use ::pes_model::model::Model;

use crate::affine::Affine;
use crate::ir::{
    Bone, CanonicalModel, Material, Mesh, MeshGroup, SourceFormat, Texture, Vertices, validate,
};
use crate::loss::{Finding, Subject};
use crate::materials::{self, PreFoxMaterial, SamplerSettings, TextureRole, family, to_prefox};
use crate::skeletons;

use super::super::{ConvertError, Imported, drop_out_of_group_slots};

/// `pes_model`'s filter value as `materials`'s.
fn filter(filter: mtl::Filter) -> materials::Filter {
    match filter {
        mtl::Filter::Linear => materials::Filter::Linear,
        mtl::Filter::Point => materials::Filter::Point,
        mtl::Filter::Anisotropic => materials::Filter::Anisotropic,
    }
}

/// `pes_model`'s address mode as `materials`'s.
fn address(address: mtl::Address) -> materials::Address {
    match address {
        mtl::Address::Wrap => materials::Address::Wrap,
        mtl::Address::Clamp => materials::Address::Clamp,
        mtl::Address::Repeat => materials::Address::Repeat,
    }
}

/// The `Sampler`'s optional attributes as `SamplerSettings`.
fn sampler_settings(sampler: &mtl::Sampler) -> SamplerSettings {
    SamplerSettings {
        srgb: sampler.srgb,
        minfilter: sampler.minfilter.map(filter),
        maxfilter: sampler.maxfilter.map(filter),
        magfilter: sampler.magfilter.map(filter),
        mipfilter: sampler.mipfilter.map(filter),
        uaddr: sampler.uaddr.map(address),
        vaddr: sampler.vaddr.map(address),
        waddr: sampler.waddr.map(address),
        maxaniso: sampler.maxaniso,
    }
}

/// `[x, y, z]` rows widened with `w = 1.0` (the 16→21 converter's convention).
fn widen(rows: &[[f32; 3]]) -> Vec<[f32; 4]> {
    rows.iter().map(|&[x, y, z]| [x, y, z, 1.0]).collect()
}

/// The mesh's bone weights, synthesizing `[1, 0, 0, 0]` per vertex when it stores bone
/// indices only (the referee-card case: slot 0 binds the vertex fully). The IR keeps one
/// representation — indices and weights together — so the same binding is carried in the
/// other of the format's two spellings; no finding.
pub(super) fn bone_weights(vertices: &::pes_model::format::MeshVertices) -> Option<Vec<[f32; 4]>> {
    match (&vertices.bone_indices, &vertices.bone_weights) {
        (Some(indices), None) => Some(vec![[1.0, 0.0, 0.0, 0.0]; indices.len()]),
        (_, weights) => weights.clone(),
    }
}

/// `.model` + `.mtl` → IR. Decodes the mesh splitting on a copy (the IR keeps the file's
/// vertex order), rebuilds the parent-first bone order from the render hierarchy, and
/// validates the result.
pub fn model_to_ir(model: &Model, mtl: &mtl::MaterialSet) -> Result<Imported, ConvertError> {
    let mut model = model.clone();
    ::pes_model::ops::split::decode(&mut model)?;
    let model = &model;
    let mut findings = Vec::new();

    // Each bone's render parent resolved to a bone index; a parent the model does not
    // carry leaves the bone a root (no climbing, as the legacy converters did).
    let render_parents: Vec<Option<usize>> = model
        .bones
        .iter()
        .map(|bone| {
            skeletons::render_parent(&bone.name)
                .and_then(|parent| model.bones.iter().position(|b| b.name == parent))
        })
        .collect();

    // Parent-first order: a bone listed before its render parent moves right after it.
    let order = crate::ir::parent_first_order(&render_parents).map_err(|_| {
        ConvertError::PesModel(::pes_model::format::ModelError::InvalidModel(
            "bone parent cycle",
        ))
    })?;
    let mut new_index = vec![0usize; model.bones.len()];
    for (new, &old) in order.iter().enumerate() {
        new_index[old] = new;
    }

    let mut bones = Vec::with_capacity(model.bones.len());
    for &old in &order {
        let bone = &model.bones[old];
        bones.push(Bone {
            name: bone.name.clone(),
            parent: render_parents[old].map(|parent| new_index[parent]),
            matrix: Affine(bone.matrix)
                .inverse()
                .ok_or(ConvertError::SingularBoneMatrix(old))?,
            global_position: None,
            local_position: None,
            bounding_box: None,
        });
    }

    // Every texture the bound materials reference, deduped in first-occurrence order over
    // `model.materials` order; definitions no mesh binds are skipped.
    let mut textures: Vec<Texture> = Vec::new();
    let mut texture_index: HashMap<(&str, &str), usize> = HashMap::new();
    let mut sampler_texture: HashMap<(&str, &str), usize> = HashMap::new();
    let paths = ::pes_model::ops::paths::texture_paths(mtl);
    for name in &model.materials {
        for path in paths.iter().filter(|path| path.material == *name) {
            let key = (path.directory.as_str(), path.file_name.as_str());
            let index = match texture_index.entry(key) {
                std::collections::hash_map::Entry::Occupied(entry) => *entry.get(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(textures.len());
                    textures.push(Texture {
                        directory: path.directory.clone(),
                        file_name: path.file_name.clone(),
                    });
                    textures.len() - 1
                }
            };
            sampler_texture
                .entry((name.as_str(), path.sampler.as_str()))
                .or_insert(index);
        }
    }

    let mut materials = Vec::with_capacity(model.materials.len());
    for name in &model.materials {
        let definition = mtl
            .materials
            .iter()
            .find(|material| material.name == *name)
            .ok_or_else(|| ConvertError::MaterialUndefined(name.clone()))?;
        let inferred = family::from_prefox_shader(&definition.shader);
        if inferred.approximate {
            findings.push(Finding {
                code: "material_family_approximated",
                subject: Subject::Material(materials.len()),
                detail: name.clone(),
            });
        }
        let state = |name: &str| -> Option<u32> {
            definition.entries.iter().find_map(|entry| match entry {
                mtl::MaterialEntry::State(state) if state.name == name => Some(state.value),
                _ => None,
            })
        };
        let mut canonical: Vec<(TextureRole, usize)> = Vec::new();
        let mut native: Vec<(String, usize)> = Vec::new();
        let mut samplers = Vec::new();
        let mut states = Vec::new();
        let mut parameters = Vec::new();
        for entry in &definition.entries {
            match entry {
                mtl::MaterialEntry::Sampler(sampler) => {
                    samplers.push((sampler.name.clone(), sampler_settings(sampler)));
                    let index = *sampler_texture
                        .get(&(name.as_str(), sampler.name.as_str()))
                        .expect("texture_paths covers every sampler of a bound material");
                    match to_prefox::role_for_sampler(&sampler.name) {
                        // The first sampler a role gets is canonical; the rest are native.
                        Some(role) if !canonical.iter().any(|(seen, _)| *seen == role) => {
                            canonical.push((role, index));
                        }
                        _ => native.push((sampler.name.clone(), index)),
                    }
                }
                mtl::MaterialEntry::State(state) => {
                    states.push((state.name.clone(), state.value));
                }
                mtl::MaterialEntry::Vector(vector) => {
                    parameters.push((vector.name.clone(), vector.components.clone()));
                }
            }
        }
        materials.push(Material {
            name: name.clone(),
            family: inferred.family,
            two_sided: state("twosided").map(|value| value != 0),
            transparent: state("alphablend").map(|value| value != 0),
            textures: canonical,
            parameters: vec![],
            fox: None,
            prefox: Some(PreFoxMaterial {
                shader: definition.shader.clone(),
                states,
                samplers,
                textures: native,
                parameters,
            }),
        });
    }

    let mut meshes = Vec::with_capacity(model.meshes.len());
    for (index, mesh) in model.meshes.iter().enumerate() {
        let vertices = &mesh.vertices;
        let synthesized = vertices.bone_indices.is_some() && vertices.bone_weights.is_none();
        let mut bone_weights = bone_weights(vertices);
        // A synthesized column writes `QuadFloat32`; a stored one keeps its width.
        let bone_weight_width = bone_weights.is_some().then_some(if synthesized {
            4
        } else {
            vertices.bone_weight_width
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
        // `Model::from_file` does not range-check group entries against the bone table;
        // a malformed index is an error here, not a panic on the remap.
        let mut bone_group = Vec::with_capacity(mesh.bone_group.len());
        for &index in &mesh.bone_group {
            if index >= model.bones.len() {
                return Err(ConvertError::PesModel(
                    ::pes_model::format::ModelError::BadReference {
                        what: "bone",
                        offset: index as i64,
                    },
                ));
            }
            bone_group.push(new_index[index]);
        }
        meshes.push(Mesh {
            vertices: Vertices {
                positions: vertices.positions.clone(),
                normals: vertices.normals.as_deref().map(widen),
                tangents: vertices.tangents.as_deref().map(widen),
                bitangents: vertices.bitangents.clone(),
                colors: vertices.colors.clone(),
                uvs: vertices.uvs.clone(),
                uv_high_precision: vec![false; vertices.uvs.len()],
                bone_indices: vertices.bone_indices.clone(),
                bone_weights,
                bone_weight_width,
            },
            faces: mesh.faces.iter().map(|&[a, b, c]| [a, c, b]).collect(),
            bone_group,
            material: mesh.material,
            // The `Split-Mesh` marker a kept split group still carries is dropped like
            // a decoded group's: the IR keeps the component as an ordinary mesh.
            extension_headers: mesh
                .extension_headers
                .iter()
                .filter(|header| {
                    header.as_str() != ::pes_model::ops::vertex_enc::VERTEX_LOOP_PRESERVATION
                        && !header
                            .split_once(':')
                            .is_some_and(|(key, _)| key.trim().eq_ignore_ascii_case("Split-Mesh"))
                })
                .cloned()
                .collect(),
            custom_bounding_box: None,
        });
        for (field, non_default) in [
            ("lower_lods", !mesh.lower_lods.is_empty()),
            ("tags", !mesh.tags.is_empty()),
            ("editor_data", !mesh.editor_data.is_empty()),
            ("order", mesh.order != 0),
        ] {
            if non_default {
                findings.push(Finding {
                    code: "native_field_dropped",
                    subject: Subject::Mesh(index),
                    detail: field.to_string(),
                });
            }
        }
    }
    if model.flags != 0 {
        findings.push(Finding {
            code: "native_field_dropped",
            subject: Subject::Model,
            detail: "flags".to_string(),
        });
    }

    let ir = CanonicalModel {
        bones,
        meshes,
        mesh_groups: model
            .meshes
            .iter()
            .enumerate()
            .map(|(index, mesh)| MeshGroup {
                name: mesh.name.clone().unwrap_or_else(|| format!("mesh_{index}")),
                parent: None,
                meshes: vec![index],
                visible: true,
            })
            .collect(),
        materials,
        textures,
        extension_headers: model.extension_headers.iter().cloned().collect(),
        source_format: SourceFormat::PreFox,
    };
    validate(&ir)?;
    Ok(Imported {
        model: ir,
        findings,
    })
}
