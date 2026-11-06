//! Skeleton retargeting: conforming an IR model to another PES version's skeleton. Bones the
//! target lacks fold onto the bones that take their weight; every surviving standard bone
//! re-binds from its source pose to the target's (the `Dᵢ = B_target · B_source⁻¹` blend per
//! vertex). A same-version retarget is a no-op by construction.

use pes_version::PesVersion;

use crate::affine::Affine;
use crate::formats::ConvertError;
use crate::ir::{Bone, CanonicalModel, validate};
use crate::loss::{Finding, Subject};

use super::{
    PesBone, fold_target, is_face_bone, is_standard, render_parent, skeletons, version_bone,
    version_bone_hand_first,
};

/// The moved threshold on a delta matrix's components, past which a bone counts as moved
/// and its vertices are re-bound. It sits above Konami's own PES 17 glove `g101`, 0.0025
/// off the hand table on three finger bones, and below the smallest difference between two versions' body tables, 0.0042 (one hem bone,
/// PES 15 against 17), so the games' own files pass and every version difference is caught.
pub(crate) const MOVED_TOLERANCE: f32 = 3e-3;

/// `target · source⁻¹`, `None` when `source` is singular.
pub(crate) fn bone_delta(source: &Affine, target: &Affine) -> Option<Affine> {
    source.inverse().map(|inverse| target.multiply(&inverse))
}

/// Whether the re-bind delta `target · source⁻¹` differs from the identity by more than
/// the moved tolerance (`None` on a singular source — an uncomparable pose).
pub(crate) fn bone_moved(source: &Affine, target: &Affine) -> Option<bool> {
    bone_delta(source, target)
        .map(|delta| delta.max_component_delta(&Affine::IDENTITY) > MOVED_TOLERANCE)
}

/// The Euclidean distance between two translations.
fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// `v` renormalized, or `None` when it came out zero-length (the caller keeps the original).
fn normalized(v: [f32; 3]) -> Option<[f32; 3]> {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    (len > 0.0).then(|| [v[0] / len, v[1] / len, v[2] / len])
}

/// Conforms `ir` to `target`'s skeleton: folds standard bones the target lacks onto the
/// bones that take their weight, then re-binds every surviving standard bone from its
/// source pose (`Bone.matrix`) to the target's. Custom bones and a face's `skf_*` bones pass
/// through with their own matrices. Takes the model by value, so an error can never leave a
/// half-retargeted model behind; returns it with the losses.
pub fn retarget(
    mut ir: CanonicalModel,
    target: PesVersion,
) -> Result<(CanonicalModel, Vec<Finding>), ConvertError> {
    let tables = skeletons(target);
    let mut findings = Vec::new();
    // A model with `skh_` weights conforms to the `hand_*` tables: shared
    // wrist/forearm names carry a different pose there than in `body`.
    // Existence checks keep `version_bone`.
    // A face's `skf_*` bones have no conforming bone: they keep their own matrices.
    let hand = crate::ops::hand_split::has_hand_weights(&ir);
    let conforming = |name: &str| -> Option<&PesBone> {
        if is_face_bone(name) {
            None
        } else if hand {
            version_bone_hand_first(tables, name)
        } else {
            version_bone(tables, name)
        }
    };

    // ---- Fold: resolve every fold before mutating (two folds may share a target). ----
    let count = ir.bones.len();
    let weighted = |bone: usize| -> bool { crate::ir::bone_is_weighted(&ir, bone) };
    let mut folds: Vec<Option<String>> = vec![None; count];
    for (index, bone) in ir.bones.iter().enumerate() {
        if !is_standard(&bone.name) || version_bone(tables, &bone.name).is_some() {
            continue;
        }
        // Follow the fold table until a name the target has; a name with no entry (or a
        // loop, which the table test forbids) falls back to the nearest target body bone.
        let mut resolved = None;
        let mut name = bone.name.as_str();
        let mut seen = vec![name];
        while let Some(next) = fold_target(name) {
            if version_bone(tables, next).is_some() {
                resolved = Some(next.to_string());
                break;
            }
            if seen.contains(&next) {
                break;
            }
            seen.push(next);
            name = next;
        }
        let (target_name, code) = match resolved {
            Some(name) => (name, "bone_folded_for_version"),
            None => {
                let point = bone.matrix.translation();
                let nearest = tables
                    .body
                    .bones
                    .iter()
                    .min_by(|a, b| {
                        distance(point, a.matrix.translation())
                            .total_cmp(&distance(point, b.matrix.translation()))
                    })
                    .expect("a version's body table is never empty");
                (nearest.name.clone(), "bone_folded_by_position")
            }
        };
        if weighted(index) {
            findings.push(Finding {
                code,
                subject: Subject::Bone(index),
                detail: format!("{} -> {}", bone.name, target_name),
            });
        }
        folds[index] = Some(target_name);
    }

    if folds.iter().any(Option::is_some) {
        let removed: Vec<bool> = folds.iter().map(Option::is_some).collect();
        // The new bone list: survivors in order (children of a removed bone take its
        // parent, chasing up past chains of removed bones), then each fold target
        // appended when absent with the target table's matrix and its render parent
        // resolved in the list built so far.
        let keep: Vec<bool> = removed.iter().map(|r| !r).collect();
        let old_to_new = crate::ir::rebuild_bone_list(&mut ir.bones, &keep);
        for name in folds.iter().flatten() {
            if ir.bones.iter().any(|bone| bone.name == *name) {
                continue;
            }
            let table_bone =
                conforming(name).expect("a resolved fold target is in the target's tables");
            let parent = render_parent(name)
                .and_then(|parent| ir.bones.iter().position(|bone| bone.name == parent));
            ir.bones.push(Bone {
                name: name.clone(),
                parent,
                matrix: table_bone.matrix,
                global_position: None,
                local_position: None,
                bounding_box: None,
            });
        }
        // The new index of each removed bone's fold target.
        let redirect: Vec<Option<usize>> = (0..count)
            .map(|old| {
                folds[old].as_ref().map(|name| {
                    ir.bones
                        .iter()
                        .position(|bone| bone.name == *name)
                        .expect("appended above")
                })
            })
            .collect();
        for mesh in &mut ir.meshes {
            crate::ir::remap_bone_group(mesh, &old_to_new, &redirect);
        }
        // A fold target just gained the removed bone's weighted vertices;
        // its cached bounding box no longer covers them — the FMDL export
        // recomputes a missing box from the weighted vertices.
        for target in redirect.iter().flatten() {
            ir.bones[*target].bounding_box = None;
        }
    }

    // ---- Re-bind: every surviving standard bone from its source pose to the target's. ----
    let mut deltas: Vec<Option<Affine>> = vec![None; ir.bones.len()];
    let mut moved = 0;
    for (index, bone) in ir.bones.iter_mut().enumerate() {
        let Some(table_bone) = conforming(&bone.name) else {
            continue;
        };
        let source = bone.matrix;
        let delta = bone_delta(&source, &table_bone.matrix)
            .ok_or(ConvertError::SingularBoneMatrix(index))?;
        if delta.max_component_delta(&Affine::IDENTITY) > MOVED_TOLERANCE {
            deltas[index] = Some(delta);
            moved += 1;
            bone.matrix = table_bone.matrix;
            bone.global_position = None;
            bone.local_position = None;
            bone.bounding_box = None;
        }
    }
    if moved > 0 {
        for mesh in &mut ir.meshes {
            let (Some(indices), Some(weights)) = (
                mesh.vertices.bone_indices.clone(),
                mesh.vertices.bone_weights.clone(),
            ) else {
                continue;
            };
            for (vertex, (row, ws)) in indices.iter().zip(&weights).enumerate() {
                let mut delta = [Affine::IDENTITY; 4];
                let mut any_moved = false;
                for (slot, &entry) in row.iter().enumerate() {
                    if ws[slot] > 0.0
                        && let Some(d) = deltas[mesh.bone_group[usize::from(entry)]]
                    {
                        delta[slot] = d;
                        any_moved = true;
                    }
                }
                if !any_moved {
                    continue;
                }
                // The stored weights may not sum to 1 (validation only bounds them);
                // divide the blend by their sum so a part-weighted vertex still
                // tracks its bone's full delta instead of sinking toward the origin.
                let sum: f32 = ws.iter().sum();
                let blend = |point: [f32; 3]| -> [f32; 3] {
                    let mut out = [0.0; 3];
                    for slot in 0..4 {
                        if ws[slot] > 0.0 {
                            let moved = delta[slot].transform_point(point);
                            for axis in 0..3 {
                                out[axis] += ws[slot] * moved[axis];
                            }
                        }
                    }
                    if sum > 0.0 {
                        for component in &mut out {
                            *component /= sum;
                        }
                    }
                    out
                };
                let blend_direction = |direction: [f32; 3]| -> [f32; 3] {
                    let mut out = [0.0; 3];
                    for slot in 0..4 {
                        if ws[slot] > 0.0 {
                            let moved = delta[slot].transform_direction(direction);
                            for axis in 0..3 {
                                out[axis] += ws[slot] * moved[axis];
                            }
                        }
                    }
                    if sum > 0.0 {
                        for component in &mut out {
                            *component /= sum;
                        }
                    }
                    out
                };
                mesh.vertices.positions[vertex] = blend(mesh.vertices.positions[vertex]);
                if let Some(normals) = &mut mesh.vertices.normals {
                    let n = normals[vertex];
                    if let Some(blended) = normalized(blend_direction([n[0], n[1], n[2]])) {
                        normals[vertex] = [blended[0], blended[1], blended[2], n[3]];
                    }
                }
                if let Some(tangents) = &mut mesh.vertices.tangents {
                    let t = tangents[vertex];
                    if let Some(blended) = normalized(blend_direction([t[0], t[1], t[2]])) {
                        tangents[vertex] = [blended[0], blended[1], blended[2], t[3]];
                    }
                }
                if let Some(bitangents) = &mut mesh.vertices.bitangents {
                    let b = bitangents[vertex];
                    if let Some(blended) = normalized(blend_direction(b)) {
                        bitangents[vertex] = blended;
                    }
                }
            }
        }
        findings.push(Finding {
            code: "skeleton_retargeted",
            subject: Subject::Model,
            detail: moved.to_string(),
        });
    }

    validate(&ir)?;
    Ok((ir, findings))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::ir::{Material, Mesh, MeshGroup, SourceFormat, Vertices};
    use crate::materials::MaterialFamily;

    /// `names` reordered parent-first under the render hierarchy (parents absent from the
    /// list are ignored), as the IR requires.
    fn parent_first(names: &[&str]) -> Vec<usize> {
        let mut order = Vec::with_capacity(names.len());
        let mut placed = vec![false; names.len()];
        let mut stack = Vec::new();
        for index in 0..names.len() {
            if placed[index] {
                continue;
            }
            stack.push(index);
            while let Some(&top) = stack.last() {
                let parent = render_parent(names[top])
                    .and_then(|parent| names.iter().position(|name| *name == parent));
                match parent {
                    Some(parent) if !placed[parent] => stack.push(parent),
                    _ => {
                        placed[top] = true;
                        order.push(top);
                        stack.pop();
                    }
                }
            }
        }
        order
    }

    /// One bone per `name` with `source`'s bind pose (`IDENTITY` for names no table
    /// knows), one mesh grouping every bone with one vertex fully weighted to each.
    fn ir_with_bones(names: &[&str], source: PesVersion) -> CanonicalModel {
        let tables = skeletons(source);
        let order = parent_first(names);
        let mut bones: Vec<Bone> = Vec::with_capacity(names.len());
        for &old in &order {
            let name = names[old];
            bones.push(Bone {
                name: name.to_string(),
                parent: render_parent(name)
                    .and_then(|parent| order.iter().position(|&o| names[o] == parent)),
                matrix: version_bone(tables, name)
                    .map(|bone| bone.matrix)
                    .unwrap_or(Affine::IDENTITY),
                global_position: None,
                local_position: None,
                bounding_box: None,
            });
        }
        CanonicalModel {
            meshes: vec![Mesh {
                vertices: Vertices {
                    positions: bones.iter().map(|bone| bone.matrix.translation()).collect(),
                    normals: Some(vec![[0.0, 1.0, 0.0, 1.0]; bones.len()]),
                    bone_indices: Some(
                        (0..bones.len()).map(|slot| [slot as u8, 0, 0, 0]).collect(),
                    ),
                    bone_weights: Some(vec![[1.0, 0.0, 0.0, 0.0]; bones.len()]),
                    bone_weight_width: Some(4),
                    ..Vertices::default()
                },
                faces: vec![[0, 0, 0]],
                bone_group: (0..bones.len()).collect(),
                material: 0,
                extension_headers: Default::default(),
                custom_bounding_box: None,
            }],
            mesh_groups: vec![MeshGroup {
                name: "group".to_string(),
                parent: None,
                meshes: vec![0],
                visible: true,
            }],
            materials: vec![Material {
                name: "mat".to_string(),
                family: MaterialFamily::Shaded,
                two_sided: None,
                transparent: None,
                textures: vec![],
                parameters: vec![],
                fox: None,
                prefox: None,
            }],
            textures: vec![],
            extension_headers: Default::default(),
            source_format: SourceFormat::Fox,
            bones,
        }
    }

    /// Every bone name of `version`'s body table.
    fn body_names(version: PesVersion) -> Vec<String> {
        skeletons(version)
            .body
            .bones
            .iter()
            .map(|bone| bone.name.clone())
            .collect()
    }

    /// The `(code, detail)` pairs of `findings`.
    fn codes(findings: &[Finding]) -> Vec<(&'static str, &str)> {
        findings
            .iter()
            .map(|finding| (finding.code, finding.detail.as_str()))
            .collect()
    }

    #[test]
    fn pes17_to_pes15_folds_the_legacy_six() {
        let names: Vec<String> = body_names(PesVersion::Pes17);
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let ir = ir_with_bones(&refs, PesVersion::Pes17);
        let (ir, findings) = retarget(ir, PesVersion::Pes15).expect("retarget");
        let folds: Vec<&Finding> = findings
            .iter()
            .filter(|finding| finding.code == "bone_folded_for_version")
            .collect();
        assert_eq!(
            folds
                .iter()
                .map(|finding| finding.detail.as_str())
                .collect::<Vec<_>>(),
            [
                "dsk_deltoid_l -> dsk_upperarm_l",
                "dsk_deltoid_r -> dsk_upperarm_r",
                "dsk_trapezius_l -> sk_shoulder_l",
                "dsk_trapezius_r -> sk_shoulder_r",
                "dsk_upperarm_long_l -> dsk_upperarm_l",
                "dsk_upperarm_long_r -> dsk_upperarm_r",
            ]
        );
        assert!(
            !findings
                .iter()
                .any(|finding| finding.code == "bone_folded_by_position")
        );
        // The legacy `movedBones` stay, re-bound to PES15's pose.
        let pes15 = &skeletons(PesVersion::Pes15).body;
        let pes17 = &skeletons(PesVersion::Pes17).body;
        for name in [
            "dsk_belly_scale",
            "dsk_pectoralis_l",
            "dsk_pectoralis_r",
            "dsk_scapula_r",
        ] {
            let bone = ir.bones.iter().find(|bone| bone.name == name).expect(name);
            assert_eq!(bone.matrix, pes15.bone(name).expect(name).matrix);
        }
        // Every surviving bone is exactly PES15's matrix, or PES17's when unmoved.
        for bone in &ir.bones {
            let target = pes15.bone(&bone.name).expect("target bone");
            let source = pes17.bone(&bone.name).expect("source bone");
            assert!(
                bone.matrix == target.matrix || bone.matrix == source.matrix,
                "{}",
                bone.name
            );
        }
        assert_eq!(
            findings
                .last()
                .map(|f| (f.code, f.subject.clone(), f.detail.as_str())),
            Some(("skeleton_retargeted", Subject::Model, "18"))
        );
        assert_eq!(findings.len(), 7);
    }

    #[test]
    fn pes19_to_pes16_folds_46() {
        let names: Vec<String> = body_names(PesVersion::Pes19);
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let ir = ir_with_bones(&refs, PesVersion::Pes19);
        let (ir, findings) = retarget(ir, PesVersion::Pes16).expect("retarget");
        assert_eq!(
            findings
                .iter()
                .filter(|f| f.code == "bone_folded_for_version")
                .count(),
            46
        );
        assert!(!findings.iter().any(|f| f.code == "bone_folded_by_position"));
        let details = codes(&findings);
        for want in [
            "dsk_pos_1_wrist_l -> sk_hand_l",
            "dsk_pos_trapezius_l -> dsk_trapezius_l",
            "dsk_upperarm_skin_t_l -> dsk_upperarm_l",
        ] {
            assert!(details.iter().any(|(_, detail)| *detail == want), "{want}");
        }
        // The result is exactly the bones both versions share, plus the fold targets.
        let pes16: BTreeSet<&str> = skeletons(PesVersion::Pes16)
            .body
            .bones
            .iter()
            .map(|bone| bone.name.as_str())
            .collect();
        let expected: BTreeSet<&str> = names
            .iter()
            .filter(|name| pes16.contains(name.as_str()))
            .map(String::as_str)
            .chain(
                findings
                    .iter()
                    .filter_map(|finding| finding.detail.split(" -> ").nth(1)),
            )
            .collect();
        let got: BTreeSet<&str> = ir.bones.iter().map(|bone| bone.name.as_str()).collect();
        assert_eq!(got, expected);
        // The legacy `movedBones` survive the retarget (PES16 keeps them).
        for name in [
            "dsk_belly_scale",
            "dsk_pectoralis_l",
            "dsk_pectoralis_r",
            "dsk_scapula_r",
        ] {
            assert!(got.contains(name), "{name}");
        }
    }

    #[test]
    fn pes19_to_pes21_folds_the_pos_helpers() {
        let names: Vec<String> = body_names(PesVersion::Pes19);
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let ir = ir_with_bones(&refs, PesVersion::Pes19);
        let (_, findings) = retarget(ir, PesVersion::Pes21).expect("retarget");
        assert_eq!(
            findings
                .iter()
                .filter(|f| f.code == "bone_folded_for_version")
                .map(|f| f.detail.clone())
                .collect::<BTreeSet<_>>(),
            [
                "dsk_pos_1_wrist_l -> sk_hand_l",
                "dsk_pos_1_wrist_r -> sk_hand_r",
                "dsk_pos_2_wrist_l -> sk_hand_l",
                "dsk_pos_2_wrist_r -> sk_hand_r",
                "dsk_pos_3_wrist_l -> sk_hand_l",
                "dsk_pos_3_wrist_r -> sk_hand_r",
                "dsk_pos_clavicle_l -> dsk_clavicle_l",
                "dsk_pos_clavicle_r -> dsk_clavicle_r",
                "dsk_pos_trapezius_l -> dsk_trapezius_l",
                "dsk_pos_trapezius_r -> dsk_trapezius_r",
            ]
            .into_iter()
            .map(str::to_string)
            .collect()
        );
    }

    #[test]
    fn same_version_is_a_noop() {
        let names: Vec<String> = body_names(PesVersion::Pes21);
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let ir = ir_with_bones(&refs, PesVersion::Pes21);
        let before = ir.clone();
        let (ir, findings) = retarget(ir, PesVersion::Pes21).expect("retarget");
        assert_eq!(ir, before);
        assert_eq!(findings, Vec::<Finding>::new());
    }

    #[test]
    fn a_bone_with_no_fold_entry_folds_by_position() {
        let ir = ir_with_bones(&["sk_belly", "dsk_back"], PesVersion::Pes21);
        let (ir, findings) = retarget(ir, PesVersion::Pes15).expect("retarget");
        assert!(findings.contains(&Finding {
            code: "bone_folded_by_position",
            subject: Subject::Bone(1),
            detail: "dsk_back -> sk_belly".to_string(),
        }));
        assert!(!ir.bones.iter().any(|bone| bone.name == "dsk_back"));
    }

    #[test]
    fn a_custom_bone_passes_through() {
        let ir = ir_with_bones(&["sk_belly", "sk_flag"], PesVersion::Pes21);
        let (ir, findings) = retarget(ir, PesVersion::Pes15).expect("retarget");
        let flag = ir
            .bones
            .iter()
            .find(|bone| bone.name == "sk_flag")
            .expect("sk_flag");
        assert_eq!(flag.matrix, Affine::IDENTITY);
        assert!(
            !findings
                .iter()
                .any(|finding| finding.detail.starts_with("sk_flag"))
        );
    }

    #[test]
    fn weights_merge_into_the_earliest_slot() {
        let mut ir = ir_with_bones(
            &["sk_shoulder_l", "dsk_upperarm_l", "dsk_deltoid_l"],
            PesVersion::Pes17,
        );
        let vertices = &mut ir.meshes[0].vertices;
        vertices.bone_indices = Some(vec![[1, 2, 0, 0], [1, 0, 0, 0], [2, 0, 0, 0]]);
        vertices.bone_weights = Some(vec![
            [0.5, 0.5, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
        ]);
        let ir = retarget(ir, PesVersion::Pes15).expect("retarget").0;
        let vertices = &ir.meshes[0].vertices;
        assert_eq!(ir.meshes[0].bone_group, [0, 1]);
        assert_eq!(
            vertices.bone_indices,
            Some(vec![[1, 0, 0, 0], [1, 0, 0, 0], [1, 0, 0, 0]])
        );
        assert_eq!(
            vertices.bone_weights,
            Some(vec![
                [1.0, 0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0, 0.0]
            ])
        );
    }

    #[test]
    fn rebind_moves_vertices_by_the_delta() {
        let target = skeletons(PesVersion::Pes15)
            .body
            .bone("sk_belly")
            .expect("sk_belly")
            .matrix;
        // Translation case: source pose is the target's shifted +0.1 on x.
        let mut ir = ir_with_bones(&["sk_belly"], PesVersion::Pes15);
        let mut source = target;
        source.0[3] += 0.1;
        ir.bones[0].matrix = source;
        let at = ir.meshes[0].vertices.positions[0];
        let (ir, findings) = retarget(ir, PesVersion::Pes15).expect("retarget");
        let moved = ir.meshes[0].vertices.positions[0];
        for axis in 0..3 {
            let want = at[axis] + [-0.1, 0.0, 0.0][axis];
            assert!((moved[axis] - want).abs() < 1e-6, "axis {axis}");
        }
        let normal = ir.meshes[0].vertices.normals.as_ref().expect("normals")[0];
        for axis in 0..3 {
            assert!(
                (normal[axis] - [0.0, 1.0, 0.0][axis]).abs() < 1e-6,
                "axis {axis}: {normal:?}"
            );
        }
        assert_eq!(ir.bones[0].matrix, target);
        assert_eq!(
            findings,
            vec![Finding {
                code: "skeleton_retargeted",
                subject: Subject::Model,
                detail: "1".to_string(),
            }]
        );

        // Rotation case: the source pose is a 90° z-rotation composed before the table.
        let mut ir = ir_with_bones(&["sk_belly"], PesVersion::Pes15);
        ir.bones[0].matrix = Affine::from_rotation_translation(
            [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            target.translation(),
        )
        .multiply(&target);
        let ir = retarget(ir, PesVersion::Pes15).expect("retarget").0;
        let normal = ir.meshes[0].vertices.normals.as_ref().expect("normals")[0];
        for axis in 0..3 {
            assert!(
                (normal[axis] - [1.0, 0.0, 0.0][axis]).abs() < 1e-6,
                "axis {axis}: {normal:?}"
            );
        }
    }

    #[test]
    fn a_part_weighted_vertex_tracks_the_full_delta() {
        // A single weight of 0.5 on a moved bone: the vertex moves by the
        // full delta, not by half the transformed position; stored weights
        // stay as authored.
        let target = skeletons(PesVersion::Pes15)
            .body
            .bone("sk_belly")
            .expect("sk_belly")
            .matrix;
        let mut ir = ir_with_bones(&["sk_belly"], PesVersion::Pes15);
        let mut source = target;
        source.0[3] += 0.1;
        ir.bones[0].matrix = source;
        ir.meshes[0].vertices.bone_weights = Some(vec![[0.5, 0.0, 0.0, 0.0]]);
        let at = ir.meshes[0].vertices.positions[0];
        let (ir, _) = retarget(ir, PesVersion::Pes15).expect("retarget");
        let moved = ir.meshes[0].vertices.positions[0];
        for axis in 0..3 {
            assert!(
                (moved[axis] - (at[axis] + [-0.1, 0.0, 0.0][axis])).abs() < 1e-6,
                "axis {axis}: {moved:?}"
            );
        }
        assert_eq!(
            ir.meshes[0].vertices.bone_weights,
            Some(vec![[0.5, 0.0, 0.0, 0.0]])
        );
    }

    #[test]
    fn unmoved_bones_and_vertices_are_untouched() {
        let ir = ir_with_bones(&["sk_belly", "dsk_deltoid_l"], PesVersion::Pes21);
        let before = ir.clone();
        let ir = retarget(ir, PesVersion::Pes18).expect("retarget").0;
        assert_eq!(ir.bones[0], before.bones[0]); // sk_belly, bit for bit
        assert_eq!(
            ir.meshes[0].vertices.positions[0],
            before.meshes[0].vertices.positions[0]
        );
        assert_eq!(
            ir.meshes[0].vertices.normals.as_ref().expect("n")[0],
            before.meshes[0].vertices.normals.as_ref().expect("n")[0]
        );
        assert_ne!(
            ir.meshes[0].vertices.positions[1],
            before.meshes[0].vertices.positions[1]
        );
    }

    #[test]
    fn normalized_recovers_unit_length() {
        assert_eq!(normalized([2.0, 0.0, 0.0]), Some([1.0, 0.0, 0.0]));
        let out = normalized([1.0, 1.0, 1.0]).expect("normalizes");
        let len = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
        assert!((len - 1.0).abs() < 1e-6, "{out:?}");
        assert!(normalized([0.0, 0.0, 0.0]).is_none());
    }

    #[test]
    fn distance_is_a_difference_not_a_sum() {
        assert_eq!(distance([3.0, 0.0, 0.0], [1.0, 0.0, 0.0]), 2.0);
        assert_eq!(distance([0.0, 0.0, 3.0], [0.0, 0.0, 1.0]), 2.0);
    }

    #[test]
    fn a_delta_at_the_moved_bound_is_not_moved() {
        // Exactly 3e-3 is not "moved"; anything past it is. Konami's glove sits
        // 0.0025 off its table, the closest two versions' tables differ by 0.0042.
        let target = Affine::IDENTITY;
        let off = |x: f32| {
            Affine::from_rotation_translation(
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                [-x, 0.0, 0.0],
            )
        };
        assert_eq!(bone_moved(&off(3e-3), &target), Some(false));
        assert_eq!(bone_moved(&off(6e-3), &target), Some(true));
        assert_eq!(bone_moved(&off(0.0025), &target), Some(false));
        assert_eq!(bone_moved(&off(0.0042), &target), Some(true));
    }

    #[test]
    fn a_bone_at_the_moved_bound_keeps_its_pose() {
        // dsk_pos_belly_ba_l's table matrix is the identity; a source pose
        // translated by exactly MOVED_TOLERANCE is not re-bound.
        let mut ir = ir_with_bones(&["dsk_pos_belly_ba_l"], PesVersion::Pes21);
        let source = Affine::from_rotation_translation(
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [-MOVED_TOLERANCE, 0.0, 0.0],
        );
        ir.bones[0].matrix = source;
        let (ir, findings) = retarget(ir, PesVersion::Pes21).expect("retarget");
        assert_eq!(ir.bones[0].matrix, source);
        assert!(
            !findings
                .iter()
                .any(|finding| finding.code == "skeleton_retargeted")
        );
    }

    #[test]
    fn an_inserted_fold_target_parents_to_its_render_parent() {
        // dsk_back folds onto sk_belly; the inserted belly's parent is its
        // render parent dsk_hip, not whichever bone the lookup happens to
        // hit first.
        let ir = ir_with_bones(&["dsk_hip", "dsk_back"], PesVersion::Pes21);
        let (ir, _) = retarget(ir, PesVersion::Pes15).expect("retarget");
        let belly = ir
            .bones
            .iter()
            .position(|bone| bone.name == "sk_belly")
            .expect("inserted sk_belly");
        let hip = ir
            .bones
            .iter()
            .position(|bone| bone.name == "dsk_hip")
            .expect("dsk_hip");
        assert_eq!(ir.bones[belly].parent, Some(hip));
    }

    #[test]
    fn a_stale_index_in_a_weighted_row_is_safe() {
        // The stale slot shares the row with a weighted one: the blend must
        // still skip the unweighted stale entry. sk_belly keeps `moved` on
        // so the per-vertex pass runs.
        let mut ir = ir_with_bones(&["dsk_deltoid_l", "sk_belly"], PesVersion::Pes17);
        ir.meshes[0].vertices.bone_indices = Some(vec![[0, 5, 0, 0], [0, 0, 0, 0]]);
        ir.meshes[0].vertices.bone_weights = Some(vec![[0.5, 0.0, 0.5, 0.0], [1.0, 0.0, 0.0, 0.0]]);
        let (ir, _) = retarget(ir, PesVersion::Pes15).expect("retarget");
        let vertices = &ir.meshes[0].vertices;
        assert!(vertices.positions[0].iter().all(|c| c.is_finite()));
        // The weighted slot remaps to the fold target's group slot; the
        // stale unweighted entry zeroes.
        assert_eq!(
            vertices.bone_indices.as_ref().expect("indices")[0],
            [1, 0, 0, 0]
        );
    }

    #[test]
    fn a_fold_targets_bounding_box_is_recomputed() {
        // `dsk_thigh_l` folds onto `sk_thigh_l` for PES18: the vertex that
        // lands on the target must be inside the box the export writes —
        // a cached box from before the fold would be stale.
        let mut ir = ir_with_bones(&["dsk_thigh_l", "sk_thigh_l"], PesVersion::Pes19);
        ir.bones[1].bounding_box = Some(crate::ir::BoundingBox {
            min: [99.0, 99.0, 99.0, 1.0],
            max: [100.0, 100.0, 100.0, 1.0],
        });
        let (ir, findings) = retarget(ir, PesVersion::Pes18).expect("retarget");
        assert_eq!(ir.bones.len(), 1);
        assert_eq!(ir.bones[0].name, "sk_thigh_l");
        assert!(ir.bones[0].bounding_box.is_none());
        assert!(findings.iter().any(|f| f.code == "bone_folded_for_version"));
        let exported = crate::formats::fmdl::ir_to_fmdl(&ir).expect("export");
        let bone = &exported.model.bones[0];
        let position = exported.model.meshes[0].vertices.positions[0];
        for axis in 0..3 {
            assert!(
                bone.bounding_box.min[axis] <= position[axis]
                    && position[axis] <= bone.bounding_box.max[axis],
                "axis {axis}: {position:?} not in {:?}..{:?}",
                bone.bounding_box.min,
                bone.bounding_box.max
            );
        }
    }

    #[test]
    fn a_hand_model_conforms_shared_bones_to_the_hand_table() {
        // PES15's hand table carries the wrist/forearm chain at a different
        // pose than its body table; a `skh_`-weighted model conforms to it.
        let tables = skeletons(PesVersion::Pes15);
        let hand_pose = tables
            .hand_l
            .bone("sk_forearm_l")
            .expect("hand table")
            .matrix;
        let body_pose = tables.body.bone("sk_forearm_l").expect("body table").matrix;
        assert!(hand_pose.max_component_delta(&body_pose) > MOVED_TOLERANCE);
        let mut ir = ir_with_bones(&["skh_index_mcp_l", "sk_forearm_l"], PesVersion::Pes15);
        // The glove's forearm sits at the PES15 hand pose, not the body pose.
        ir.bones[1].matrix = hand_pose;
        let (ir, _) = retarget(ir, PesVersion::Pes15).expect("retarget");
        assert_eq!(ir.bones[1].matrix, hand_pose);
    }

    #[test]
    fn a_face_bone_keeps_its_own_matrix() {
        // A face's `skf_*` pose is per face, so no table is a reference for it: the
        // body bone off the target's pose re-binds, the `skf_*` bone keeps its matrix.
        let mut ir = ir_with_bones(&["sk_head", "skf_eyelid_b_l"], PesVersion::Pes17);
        let mut off = |name: &str| -> Affine {
            let bone = ir
                .bones
                .iter_mut()
                .find(|bone| bone.name == name)
                .expect(name);
            bone.matrix.0[3] += 0.1;
            bone.matrix
        };
        let face_pose = off("skf_eyelid_b_l");
        off("sk_head");
        let (ir, findings) = retarget(ir, PesVersion::Pes17).expect("retarget");
        let matrix = |name: &str| {
            ir.bones
                .iter()
                .find(|bone| bone.name == name)
                .expect(name)
                .matrix
        };
        assert_eq!(matrix("skf_eyelid_b_l"), face_pose);
        let head = skeletons(PesVersion::Pes17)
            .body
            .bone("sk_head")
            .expect("sk_head")
            .matrix;
        assert_eq!(matrix("sk_head"), head);
        assert_eq!(codes(&findings), [("skeleton_retargeted", "1")]);
    }

    #[test]
    fn a_stale_unweighted_index_is_skipped_when_nothing_folds() {
        // No fold remaps the row: the out-of-group unweighted entry reaches
        // the delta pick as stored and must be skipped, not indexed.
        let mut ir = ir_with_bones(&["sk_belly"], PesVersion::Pes15);
        // A pose away from the PES15 table keeps `moved` on so the
        // per-vertex pass runs.
        ir.bones[0].matrix = Affine::IDENTITY;
        ir.meshes[0].vertices.bone_indices = Some(vec![[0, 5, 0, 0]]);
        let (ir, _) = retarget(ir, PesVersion::Pes15).expect("retarget");
        assert!(
            ir.meshes[0].vertices.positions[0]
                .iter()
                .all(|c| c.is_finite())
        );
    }

    #[test]
    fn a_stale_index_in_an_unweighted_slot_is_safe() {
        // PES17 -> PES15 folds dsk_deltoid_l onto dsk_upperarm_l; slot 1's index 5 is
        // past the group but unweighted and must not be remapped. sk_belly keeps
        // `moved` on so the per-vertex pass runs.
        let mut ir = ir_with_bones(&["dsk_deltoid_l", "sk_belly"], PesVersion::Pes17);
        ir.meshes[0].vertices.bone_indices = Some(vec![[0, 5, 0, 0], [0, 0, 0, 0]]);
        ir.meshes[0].vertices.bone_weights = Some(vec![[1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]]);
        let ir = retarget(ir, PesVersion::Pes15).expect("retarget").0;
        let vertices = &ir.meshes[0].vertices;
        assert_eq!(
            vertices.bone_indices.as_ref().expect("indices")[0],
            [1, 0, 0, 0]
        );
        assert_eq!(
            vertices.bone_weights.as_ref().expect("weights")[0],
            [1.0, 0.0, 0.0, 0.0]
        );
    }
}
