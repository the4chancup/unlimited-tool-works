//! The IR → FMDL half: `ir_to_fmdl`.

use crate::affine::Affine;
use crate::ir::{BoundingBox, CanonicalModel, validate};
use crate::loss::{self, Finding, Subject};
use crate::materials::{MaterialFamily, to_fox, to_prefox};
use crate::skeletons;

use super::super::ConvertError;
use super::{TEXTURE_DIRECTORY, template_matrix};

/// A model exported to FMDL: the companion SKL only when an exported bone sits outside the
/// template tables or off their pose (a model wholly on the template gets none; the compiler
/// injects the template).
#[derive(Debug)]
pub struct ExportedFox {
    /// The FMDL model.
    pub model: ::fmdl::Model,
    /// The companion skeleton for bones outside PES21's template tables or off their pose.
    pub skl: Option<::fmdl::SklFile>,
    /// The findings the export produced.
    pub findings: Vec<loss::Finding>,
}

/// The per-vertex weight quantization: a lane above 1 clamps to 1 first (`u8`
/// cannot carry it; the caller counts those vertices for `weight_clamped`), then
/// the byte total `round(clamped_sum * 255)` (in f64 — the f32 product lands on
/// representable halves that round wrong) is preserved, 255 per lane at most.
/// `[1.5, 0.5]` keeps the clamped lanes' total
/// `round(1.5 * 255)` = 383 as `[255, 128]`, not `[255, 255]`. Slots in
/// decreasing weight (ties by slot index) get
/// `round(weight / remaining_weight * remaining_total)` each, the last gets what
/// is left.
pub(super) fn quantize_weights(weights: [f32; 4]) -> [u8; 4] {
    let weights = weights.map(|weight| weight.min(1.0));
    let sum: f32 = weights.iter().sum();
    let mut total = (sum as f64 * 255.0).round() as i32;
    let mut remaining_weight = sum;
    let mut order = [0usize, 1, 2, 3];
    order.sort_by(|a, b| weights[*b].total_cmp(&weights[*a]).then(a.cmp(b)));
    let mut out = [0u8; 4];
    for (position, slot) in order.iter().enumerate() {
        let byte = if position == 3 {
            total.clamp(0, 255)
        } else if remaining_weight > 0.0 {
            (weights[*slot] / remaining_weight * total as f32).round() as i32
        } else {
            0
        }
        .clamp(0, 255);
        out[*slot] = byte as u8;
        remaining_weight -= weights[*slot];
        total -= byte;
    }
    out
}

/// The bounding box (w 1.0 on both corners) of every vertex with a positive weight on
/// `bone` across all meshes; `[0.0; 4]`/`[0.0; 4]` when none.
fn bone_bounding_box(ir: &CanonicalModel, bone: usize) -> BoundingBox {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let mut any = false;
    for mesh in &ir.meshes {
        if let (Some(indices), Some(weights)) =
            (&mesh.vertices.bone_indices, &mesh.vertices.bone_weights)
        {
            for (vertex, (row, row_weights)) in indices.iter().zip(weights).enumerate() {
                for (slot, index) in row.iter().enumerate() {
                    if row_weights[slot] > 0.0
                        && mesh
                            .bone_group
                            .get(usize::from(*index))
                            .is_some_and(|b| *b == bone)
                    {
                        let position = mesh.vertices.positions[vertex];
                        for axis in 0..3 {
                            min[axis] = min[axis].min(position[axis]);
                            max[axis] = max[axis].max(position[axis]);
                        }
                        any = true;
                    }
                }
            }
        }
    }
    if any {
        BoundingBox {
            min: [min[0], min[1], min[2], 1.0],
            max: [max[0], max[1], max[2], 1.0],
        }
    } else {
        BoundingBox {
            min: [0.0; 4],
            max: [0.0; 4],
        }
    }
}

/// The parent list `split::encode` works on: the bone's own parent, else its `render_parent`
/// looked up in `bones` by name.
pub(super) fn split_parents(bones: &[::fmdl::Bone]) -> Vec<Option<usize>> {
    bones
        .iter()
        .map(|bone| {
            bone.parent.or_else(|| {
                skeletons::render_parent(&bone.name)
                    .and_then(|parent| bones.iter().position(|b| b.name == parent))
            })
        })
        .collect()
}

/// IR → FMDL. Validates, resolves each material for Fox, quantizes weights, derives the bone
/// positions an import did not supply, then runs the format crate's encoders (vertex loops,
/// then mesh splitting).
pub fn ir_to_fmdl(ir: &CanonicalModel) -> Result<ExportedFox, ConvertError> {
    validate(ir)?;
    let mut findings = Vec::new();

    let resolved: Vec<to_fox::ResolvedFox> = ir.materials.iter().map(to_fox::resolve).collect();
    let mut materials = Vec::with_capacity(ir.materials.len());
    for (index, (material, resolved)) in ir.materials.iter().zip(&resolved).enumerate() {
        for role in &resolved.unused_roles {
            findings.push(Finding {
                code: "material_texture_unused",
                subject: Subject::Material(index),
                detail: format!("{role:?}"),
            });
        }
        // The resolver carries nothing of the `prefox` table: every native
        // sampler or parameter it held is a loss worth reporting.
        if let Some(prefox) = &material.prefox {
            for (name, _) in &prefox.textures {
                if !resolved.textures.iter().any(|(sampler, _)| sampler == name) {
                    findings.push(Finding {
                        code: "material_texture_unused",
                        subject: Subject::Material(index),
                        detail: name.clone(),
                    });
                }
            }
            for (name, value) in &prefox.parameters {
                // Equal (padded to four components) to the family's pre-Fox
                // default: converting back to `.mtl` regenerates it verbatim.
                let regenerated = to_prefox::default_parameters(material.family).iter().any(
                    |(default_name, default)| {
                        default_name == name
                            && value.len() <= 4
                            && value
                                .iter()
                                .copied()
                                .chain(std::iter::repeat(0.0))
                                .take(4)
                                .eq(default.iter().copied())
                    },
                );
                if !regenerated
                    && !resolved
                        .parameters
                        .iter()
                        .any(|(parameter, _)| parameter == name)
                {
                    findings.push(Finding {
                        code: "material_parameter_dropped",
                        subject: Subject::Material(index),
                        detail: name.clone(),
                    });
                }
            }
        }
        let mut textures: Vec<(String, ::fmdl::Texture)> = resolved
            .textures
            .iter()
            .map(|(sampler, texture)| {
                let texture = &ir.textures[*texture];
                (
                    sampler.clone(),
                    ::fmdl::Texture {
                        file_name: texture.file_name.clone(),
                        directory: texture.directory.clone(),
                    },
                )
            })
            .collect();
        // A lit material derived from its family gets the game's dummy normal and specular
        // maps (the compiler's texture step replaces them when the folder has real maps); a
        // Fox-native material that lacks them is left as the game shipped it.
        if material.fox.is_none()
            && matches!(
                material.family,
                MaterialFamily::Shaded | MaterialFamily::Metal
            )
        {
            for (sampler, file_name) in [
                ("NormalMap_Tex_NRM", "dummy_nrm.dds"),
                ("SpecularMap_Tex_LIN", "dummy_srm.dds"),
            ] {
                if !textures.iter().any(|(name, _)| name == sampler) {
                    textures.push((
                        sampler.to_string(),
                        ::fmdl::Texture {
                            file_name: file_name.to_string(),
                            directory: TEXTURE_DIRECTORY.to_string(),
                        },
                    ));
                    findings.push(Finding {
                        code: "dummy_texture_added",
                        subject: Subject::Material(index),
                        detail: sampler.to_string(),
                    });
                }
            }
        }
        materials.push(::fmdl::MaterialInstance {
            name: material.name.clone(),
            shader: resolved.shader.clone(),
            technique: resolved.technique.clone(),
            textures,
            parameters: resolved.parameters.clone(),
        });
    }

    let mut bones: Vec<::fmdl::Bone> = Vec::with_capacity(ir.bones.len());
    for (index, bone) in ir.bones.iter().enumerate() {
        let translation = bone.matrix.translation();
        let world =
            bone.global_position
                .unwrap_or([translation[0], translation[1], translation[2], 1.0]);
        let local = bone.local_position.unwrap_or_else(|| match bone.parent {
            Some(parent) => {
                let parent_world = bones[parent].world_position;
                [
                    world[0] - parent_world[0],
                    world[1] - parent_world[1],
                    world[2] - parent_world[2],
                    1.0,
                ]
            }
            None => [0.0, 0.0, 0.0, 1.0],
        });
        bones.push(::fmdl::Bone {
            name: bone.name.clone(),
            parent: bone.parent,
            bounding_box: bone
                .bounding_box
                .map(|b| ::fmdl::BoundingBox {
                    min: b.min,
                    max: b.max,
                })
                .unwrap_or_else(|| {
                    let b = bone_bounding_box(ir, index);
                    ::fmdl::BoundingBox {
                        min: b.min,
                        max: b.max,
                    }
                }),
            local_position: local,
            world_position: world,
        });
    }

    // FMDL binds every vertex to a bone: an unskinned mesh gets one `static` bone (the
    // legacy converter's values — the name no Konami skeleton has, so the game leaves the
    // mesh where it is placed) and is weighted fully to it.
    let static_bone = ir
        .meshes
        .iter()
        .any(|mesh| mesh.vertices.bone_indices.is_none())
        .then(|| {
            // An IR that already has a `static` bone — a re-exported bundle —
            // reuses it rather than appending a second one.
            bones
                .iter()
                .position(|bone| bone.name == "static")
                .unwrap_or_else(|| {
                    bones.push(::fmdl::Bone {
                        name: "static".to_string(),
                        parent: None,
                        bounding_box: ::fmdl::BoundingBox {
                            min: [0.0; 4],
                            max: [0.0; 4],
                        },
                        local_position: [0.0; 4],
                        world_position: [0.2, 0.0, 0.0, 1.0],
                    });
                    bones.len() - 1
                })
        });

    let mut meshes = Vec::with_capacity(ir.meshes.len());
    for (index, mesh) in ir.meshes.iter().enumerate() {
        if mesh.vertices.bitangents.is_some() {
            findings.push(Finding {
                code: "vertex_bitangents_dropped",
                subject: Subject::Mesh(index),
                detail: String::new(),
            });
        }
        let material = &resolved[mesh.material];
        let (bone_weights, bone_indices, mut bone_group) =
            match (static_bone, &mesh.vertices.bone_indices) {
                (Some(static_bone), None) => {
                    findings.push(Finding {
                        code: "static_bone_added",
                        subject: Subject::Mesh(index),
                        detail: String::new(),
                    });
                    (
                        Some(vec![[255, 0, 0, 0]; mesh.vertices.len()]),
                        Some(vec![[0, 0, 0, 0]; mesh.vertices.len()]),
                        vec![static_bone],
                    )
                }
                _ => {
                    // `u8` cannot carry a lane above 1: `quantize_weights` clamps it to
                    // 255 and the excess of the byte total drops. Past float noise
                    // (`1e-6`, what f32 accumulation leaves) that is a finding.
                    if let Some(rows) = &mesh.vertices.bone_weights {
                        let clamped = rows
                            .iter()
                            .filter(|row| row.iter().any(|weight| *weight > 1.0 + 1e-6))
                            .count();
                        if clamped > 0 {
                            findings.push(Finding {
                                code: "weight_clamped",
                                subject: Subject::Mesh(index),
                                detail: clamped.to_string(),
                            });
                        }
                    }
                    (
                        mesh.vertices
                            .bone_weights
                            .as_ref()
                            .map(|rows| rows.iter().map(|row| quantize_weights(*row)).collect()),
                        mesh.vertices.bone_indices.clone(),
                        mesh.bone_group.clone(),
                    )
                }
            };
        // A vertexless mesh skins nothing: its group is written empty, not split —
        // a face export's marker mesh carries a whole-skeleton group as data.
        if mesh.vertices.positions.is_empty() && !bone_group.is_empty() {
            findings.push(Finding {
                code: "empty_mesh_bone_group_dropped",
                subject: Subject::Mesh(index),
                detail: bone_group.len().to_string(),
            });
            bone_group.clear();
        }
        meshes.push(::fmdl::Mesh {
            vertices: ::fmdl::format::MeshVertices {
                positions: mesh.vertices.positions.clone(),
                normals: mesh.vertices.normals.clone(),
                tangents: mesh.vertices.tangents.clone(),
                colors: mesh.vertices.colors.clone(),
                uvs: mesh.vertices.uvs.clone(),
                uv_high_precision: mesh.vertices.uv_high_precision.clone(),
                bone_weights,
                bone_indices,
            },
            faces: mesh.faces.clone(),
            bone_group,
            material: mesh.material,
            alpha_flags: material.alpha_flags,
            shadow_flags: material.shadow_flags,
            has_antiblur_meshes: false,
            is_antiblur_mesh: false,
            custom_bounding_box: mesh.custom_bounding_box.map(|b| ::fmdl::BoundingBox {
                min: b.min,
                max: b.max,
            }),
        });
    }

    let mut model = ::fmdl::Model {
        bones,
        materials,
        meshes,
        mesh_groups: ir
            .mesh_groups
            .iter()
            .map(|group| ::fmdl::MeshGroup {
                name: group.name.clone(),
                parent: group.parent,
                meshes: group.meshes.clone(),
                bounding_box: None,
                visible: group.visible,
                split_mesh_group: false,
            })
            .collect(),
        extensions: ::fmdl::Extensions {
            mesh_splitting: false,
            antiblur: false,
            vertex_loop_preservation: false,
            other: ir.extension_headers.iter().cloned().collect(),
        },
        bone_matrices: None,
    };

    // The encoders in dependency order: the vertex-loop convention, then
    // mesh splitting.
    // Recover the loops from the IR vertex order itself — the flag-gated `decode_model`
    // returns identity owners on a model built from the IR, losing the file's loops.
    let owners: Vec<Vec<usize>> = model
        .meshes
        .iter()
        .map(::fmdl::ops::vertex_enc::decode)
        .collect::<Result<Vec<_>, _>>()?;
    ::fmdl::ops::vertex_enc::encode_model(&mut model, &owners)?;
    let parents = split_parents(&model.bones);
    ::fmdl::ops::split::encode(&mut model, Some(&parents))?;

    // The SKL covers every exported bone the template lacks *or has at a different
    // pose*: without it a reimport assumes the PES21 pose and a retargeted bone
    // gets retargeted again. `static` has no IR bone and never gates by itself —
    // its `None` template lookup is what puts an SKL on an unskinned export.
    let needs_skl = |bone: &::fmdl::Bone| match template_matrix(&bone.name) {
        None => true,
        Some(template) => ir
            .bones
            .iter()
            .find(|b| b.name == bone.name)
            .is_some_and(|b| {
                crate::skeletons::retarget::bone_moved(&b.matrix, &template) != Some(false)
            }),
    };
    let skl = model.bones.iter().any(needs_skl).then(|| {
        ::fmdl::SklFile::new(
            model
                .bones
                .iter()
                .map(|bone| {
                    let matrix = ir
                        .bones
                        .iter()
                        .find(|b| b.name == bone.name)
                        .map(|b| b.matrix)
                        .unwrap_or(Affine::IDENTITY);
                    ::fmdl::format::SklBone {
                        name: bone.name.clone(),
                        parent: bone.parent,
                        rotation: matrix.rotation(),
                        translation: matrix.translation(),
                    }
                })
                .collect(),
        )
    });

    Ok(ExportedFox {
        model,
        skl,
        findings,
    })
}
