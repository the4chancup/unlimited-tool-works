//! Conversion routing: `convert` takes a PES *version*, not a format — the format follows
//! from the engine, and the skeleton retargeting pass runs inside the same IR round trip,
//! so every model the compiler emits is on the target's skeleton without a second pass.
//! `needs_conversion` is the native pre-check: it reads bone names, poses and the vertex
//! weight rows, never vertex positions.

use std::collections::BTreeSet;

use pes_version::{Engine, PesVersion};

use crate::affine::Affine;
use crate::formats::{ConvertError, Imported};
use crate::loss::Finding;
use crate::ops::hand_split::{MIN_CARRIED_WEIGHT, fox_has_hand_weights, prefox_has_hand_weights};
use crate::skeletons::retarget::{bone_delta, bone_moved, retarget};
use crate::skeletons::{
    is_face_bone, is_standard, skeletons, version_bone, version_bone_hand_first,
};

/// A model in one of the native formats, at the semantic layer of its format crate.
pub enum NativeModelBundle {
    /// FMDL plus its companion `.skl` when one exists (PES 2018–2021).
    Fox {
        /// The FMDL model.
        model: ::fmdl::Model,
        /// The companion skeleton, where the bind pose comes from when present.
        skl: Option<::fmdl::SklFile>,
    },
    /// `.model` plus its `.mtl` (PES 2015–2017).
    PreFox {
        /// The `.model`.
        model: ::pes_model::model::Model,
        /// The `.mtl`.
        mtl: ::pes_model::format::mtl::MaterialSet,
    },
    // No glTF source yet: glTF import is not built.
}

/// `convert`'s result: the converted bundle and every finding the round trip produced.
pub struct Converted {
    /// The bundle in the target's format.
    pub bundle: NativeModelBundle,
    /// What the target could not keep, import → retarget → export in order.
    pub findings: Vec<Finding>,
}

/// The bundle in `target`'s format (`target.engine()`), on `target`'s skeleton: the input
/// itself when it already is both, otherwise source → IR → `retarget` → target.
/// `Converted::findings` lists what the target could not keep (`loss.rs`), so the caller
/// reports rather than the user discovers.
pub fn convert(bundle: NativeModelBundle, target: PesVersion) -> Result<Converted, ConvertError> {
    if !needs_conversion(&bundle, target) {
        return Ok(Converted {
            bundle,
            findings: vec![],
        });
    }
    let Imported {
        model,
        mut findings,
    } = match &bundle {
        NativeModelBundle::Fox { model, skl } => {
            crate::formats::fmdl::fmdl_to_ir(model, skl.as_ref())?
        }
        NativeModelBundle::PreFox { model, mtl } => {
            crate::formats::pes_model::model_to_ir(model, mtl)?
        }
    };
    let (ir, retargeted) = retarget(model, target)?;
    findings.extend(retargeted);
    let bundle = match target.engine() {
        Engine::Fox => {
            let exported = crate::formats::fmdl::ir_to_fmdl(&ir)?;
            findings.extend(exported.findings);
            NativeModelBundle::Fox {
                model: exported.model,
                skl: exported.skl,
            }
        }
        Engine::PreFox => {
            let exported = crate::formats::pes_model::ir_to_model(&ir)?;
            findings.extend(exported.findings);
            NativeModelBundle::PreFox {
                model: exported.model,
                mtl: exported.mtl,
            }
        }
    };
    Ok(Converted { bundle, findings })
}

/// Whether `convert` would go through the IR: another engine, a bone the bundle's groups use
/// that `target` lacks, or a vertex blending bones whose deltas to `target`'s tables differ
/// (`spec/model_conversion/conversion.md` "Skeleton retargeting and bone conformance"). A
/// vertex weighted to one
/// bone, or to bones sharing one delta, draws the same re-bound or not, so a model off the
/// target's pose but moved rigidly is native. A face's `skf_*` bones and custom bones take
/// no part. Reads bone names, the source poses, the target's tables and the weight rows; no
/// vertex position.
/// A group entry past the bone table, or a weighted slot past its group, counts as needing
/// conversion: such a bundle cannot be proven native here, and `convert`'s importer reports
/// the bad index.
pub fn needs_conversion(bundle: &NativeModelBundle, target: PesVersion) -> bool {
    match bundle {
        NativeModelBundle::Fox { model, skl } => {
            match target.engine() {
                Engine::Fox => {}
                Engine::PreFox => return true,
            }
            let tables = skeletons(target);
            let pes21 = skeletons(PesVersion::Pes21);
            // A model weighted to `skh_` bones conforms to the `hand_*`
            // tables (same rule retarget uses).
            let hand = fox_has_hand_weights(model);
            let mut deltas: Vec<Option<Affine>> = vec![None; model.bones.len()];
            for index in used_bones(model.meshes.iter().map(|m| &m.bone_group)) {
                let Some(bone) = model.bones.get(index) else {
                    return true;
                };
                if !is_standard(&bone.name) || is_face_bone(&bone.name) {
                    continue;
                }
                let Some(table_bone) = conforming(tables, hand, &bone.name) else {
                    return true;
                };
                // The source pose: the SKL bone of that name, else PES21's tables; an
                // unknown pose has no delta, so the bone takes no part.
                let Some(source) = skl
                    .as_ref()
                    .and_then(|skl| skl.bones.iter().find(|b| b.name == bone.name))
                    .map(|b| Affine::from_rotation_translation(b.rotation, b.translation))
                    .or_else(|| version_bone(pes21, &bone.name).map(|b| b.matrix))
                else {
                    continue;
                };
                let Some(delta) = bone_delta(&source, &table_bone.matrix) else {
                    return true;
                };
                deltas[index] = Some(delta);
            }
            blends_differing_deltas(
                &deltas,
                model.meshes.iter().map(|mesh| {
                    (
                        mesh.bone_group.as_slice(),
                        mesh.vertices.bone_indices.as_deref(),
                        mesh.vertices.bone_weights.as_deref(),
                    )
                }),
                |weight: u8| weight > 0,
            )
        }
        NativeModelBundle::PreFox { model, .. } => {
            match target.engine() {
                Engine::PreFox => {}
                Engine::Fox => return true,
            }
            let tables = skeletons(target);
            let hand = prefox_has_hand_weights(model);
            let mut deltas: Vec<Option<Affine>> = vec![None; model.bones.len()];
            for index in used_bones(model.meshes.iter().map(|m| &m.bone_group)) {
                let Some(bone) = model.bones.get(index) else {
                    return true;
                };
                if !is_standard(&bone.name) || is_face_bone(&bone.name) {
                    continue;
                }
                let Some(table_bone) = conforming(tables, hand, &bone.name) else {
                    return true;
                };
                // The source pose is the inline inverse bind matrix, inverted; a singular
                // one counts as needing conversion.
                let Some(delta) = Affine(bone.matrix)
                    .inverse()
                    .and_then(|source| bone_delta(&source, &table_bone.matrix))
                else {
                    return true;
                };
                deltas[index] = Some(delta);
            }
            blends_differing_deltas(
                &deltas,
                model.meshes.iter().map(|mesh| {
                    (
                        mesh.bone_group.as_slice(),
                        mesh.vertices.bone_indices.as_deref(),
                        mesh.vertices.bone_weights.as_deref(),
                    )
                }),
                |weight: f32| weight >= MIN_CARRIED_WEIGHT,
            )
        }
    }
}

/// The bone indices any mesh's bone group references.
fn used_bones<'a>(groups: impl Iterator<Item = &'a Vec<usize>>) -> BTreeSet<usize> {
    groups.flatten().copied().collect()
}

/// Whether some vertex blends two bones whose deltas differ beyond the moved tolerance.
/// `deltas` holds each model bone's delta to the target (`None` for a bone that takes no
/// part); each mesh comes as its bone group, index rows and weight rows, `weighted` telling
/// a slot that carries weight (the weight type is the format's: `u8` on Fox, `f32` on
/// pre-Fox). A weighted slot past its group counts as differing, for the importer to report.
fn blends_differing_deltas<'a, W: Copy + 'a>(
    deltas: &[Option<Affine>],
    meshes: impl Iterator<Item = (&'a [usize], Option<&'a [[u8; 4]]>, Option<&'a [[W; 4]]>)>,
    weighted: impl Fn(W) -> bool,
) -> bool {
    for (group, indices, weights) in meshes {
        let (Some(indices), Some(weights)) = (indices, weights) else {
            continue;
        };
        for (row, row_weights) in indices.iter().zip(weights) {
            let mut blended: [Option<Affine>; 4] = [None; 4];
            for (slot, (&entry, &weight)) in row.iter().zip(row_weights).enumerate() {
                if !weighted(weight) {
                    continue;
                }
                let Some(&bone) = group.get(usize::from(entry)) else {
                    return true;
                };
                blended[slot] = deltas.get(bone).copied().flatten();
            }
            for (slot, first) in blended.iter().enumerate() {
                for other in &blended[slot + 1..] {
                    if let (Some(first), Some(other)) = (first, other)
                        && bone_moved(first, other) != Some(false)
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// `version_bone`, or `version_bone_hand_first` for a `skh_`-weighted model.
fn conforming<'a>(
    tables: &'a crate::skeletons::VersionSkeletons,
    hand: bool,
    name: &str,
) -> Option<&'a crate::skeletons::PesBone> {
    if hand {
        version_bone_hand_first(tables, name)
    } else {
        version_bone(tables, name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{CanonicalModel, Material, Mesh, MeshGroup, SourceFormat, Vertices};
    use crate::loss::Subject;
    use crate::materials::MaterialFamily;

    const HIGHNECK: &[u8] = include_bytes!("../tests/fixtures/konami_highneck.fmdl");
    const ORAL: &[u8] = include_bytes!("../tests/fixtures/addon_oral.fmdl");
    const CAP_M: &[u8] = include_bytes!("../tests/fixtures/konami_modD_cap.model");
    const CAP_T: &[u8] = include_bytes!("../tests/fixtures/konami_modD_cap.mtl");
    const LEGACY_M: &[u8] = include_bytes!("../tests/fixtures/legacy19to16_oral.model");
    const LEGACY_T: &[u8] = include_bytes!("../tests/fixtures/legacy19to16_oral.mtl");
    const BOOTS17_M: &[u8] = include_bytes!("../tests/fixtures/konami17_boots_k0051.model");
    const BOOTS17_T: &[u8] = include_bytes!("../tests/fixtures/konami17_boots_k0051.mtl");
    const GLOVE17_M: &[u8] = include_bytes!("../tests/fixtures/konami17_glove_r_g101.model");
    const GLOVE17_T: &[u8] = include_bytes!("../tests/fixtures/konami17_glove_r_g101.mtl");
    const BOOTS21_F: &[u8] = include_bytes!("../tests/fixtures/konami21_boots_k0051.fmdl");
    const BOOTS21_S: &[u8] = include_bytes!("../tests/fixtures/konami21_boots_k0051.skl");
    const TRACER_BOOTS_M: &[u8] = include_bytes!("../tests/fixtures/tracer_prefox_boots.model");
    const TRACER_BOOTS_T: &[u8] = include_bytes!("../tests/fixtures/tracer_prefox_boots.mtl");
    const TRACER_GLOVE_M: &[u8] = include_bytes!("../tests/fixtures/tracer_prefox_glove_l.model");
    const TRACER_GLOVE_T: &[u8] = include_bytes!("../tests/fixtures/tracer_prefox_glove_l.mtl");
    const TRACER_FACE_M: &[u8] = include_bytes!("../tests/fixtures/tracer_prefox_face_high.model");
    const TRACER_FACE_T: &[u8] = include_bytes!("../tests/fixtures/tracer_prefox_face.mtl");
    const CARD_M: &[u8] = include_bytes!("../tests/fixtures/konami_card.model");
    const CARD_T: &[u8] = include_bytes!("../tests/fixtures/konami_card_red.mtl");

    fn fox(bytes: &[u8]) -> NativeModelBundle {
        NativeModelBundle::Fox {
            model: ::fmdl::Model::from_file(&::fmdl::FmdlFile::read(bytes).expect("fmdl"))
                .expect("model"),
            skl: None,
        }
    }

    /// An FMDL with its companion `.skl`.
    fn fox_with_skl(fmdl: &[u8], skl: &[u8]) -> NativeModelBundle {
        NativeModelBundle::Fox {
            model: ::fmdl::Model::from_file(&::fmdl::FmdlFile::read(fmdl).expect("fmdl"))
                .expect("model"),
            skl: Some(::fmdl::SklFile::read(skl).expect("skl")),
        }
    }

    fn prefox(model: &[u8], mtl: &[u8]) -> NativeModelBundle {
        NativeModelBundle::PreFox {
            model: ::pes_model::model::Model::from_file(
                &::pes_model::format::PreFoxModel::read(model).expect("model"),
            )
            .expect("from_file"),
            mtl: ::pes_model::format::mtl::MaterialSet::read(mtl).expect("mtl"),
        }
    }

    /// `(code, subject, detail)` triples of `findings`.
    fn codes(findings: &[Finding]) -> Vec<(&'static str, Subject, &str)> {
        findings
            .iter()
            .map(|f| (f.code, f.subject.clone(), f.detail.as_str()))
            .collect()
    }

    #[test]
    fn same_version_same_engine_is_the_input() {
        let NativeModelBundle::Fox { model: fmdl, .. } = fox(HIGHNECK) else {
            unreachable!()
        };
        assert!(!needs_conversion(&fox(HIGHNECK), PesVersion::Pes21));
        let converted = convert(fox(HIGHNECK), PesVersion::Pes21).expect("convert");
        let NativeModelBundle::Fox { model, .. } = converted.bundle else {
            panic!("expected a Fox bundle");
        };
        assert_eq!(model, fmdl);
        assert_eq!(converted.findings, vec![]);

        let NativeModelBundle::PreFox { model: cap, .. } = prefox(CAP_M, CAP_T) else {
            unreachable!()
        };
        assert!(!needs_conversion(&prefox(CAP_M, CAP_T), PesVersion::Pes17));
        let converted = convert(prefox(CAP_M, CAP_T), PesVersion::Pes17).expect("convert");
        let NativeModelBundle::PreFox { model, .. } = converted.bundle else {
            panic!("expected a pre-Fox bundle");
        };
        assert_eq!(model, cap);
        assert_eq!(converted.findings, vec![]);
    }

    #[test]
    fn nothing_drawn_right_is_flagged_on_its_own_version() {
        use PesVersion::{Pes15, Pes16, Pes17, Pes18, Pes19, Pes21};
        // Stock boots on the boots pose, a stock glove 0.0025 off the hand table, the
        // community's PES 15-posed gloves, a face with its own `skf_*` pose, a one-bone
        // model: every one draws right on these versions.
        /// A fixture's name, how to read it, and the versions it draws right on.
        type Case = (
            &'static str,
            fn() -> NativeModelBundle,
            &'static [PesVersion],
        );
        let cases: [Case; 7] = [
            (
                "konami17_boots_k0051",
                || prefox(BOOTS17_M, BOOTS17_T),
                &[Pes17, Pes16, Pes15],
            ),
            (
                "konami17_glove_r_g101",
                || prefox(GLOVE17_M, GLOVE17_T),
                &[Pes17, Pes16],
            ),
            (
                "tracer_prefox_boots",
                || prefox(TRACER_BOOTS_M, TRACER_BOOTS_T),
                &[Pes17, Pes16, Pes15],
            ),
            (
                "tracer_prefox_glove_l",
                || prefox(TRACER_GLOVE_M, TRACER_GLOVE_T),
                &[Pes17, Pes16, Pes15],
            ),
            (
                "tracer_prefox_face_high",
                || prefox(TRACER_FACE_M, TRACER_FACE_T),
                &[Pes17],
            ),
            (
                "konami21_boots_k0051",
                || fox_with_skl(BOOTS21_F, BOOTS21_S),
                &[Pes21, Pes19, Pes18],
            ),
            ("konami_card", || prefox(CARD_M, CARD_T), &[Pes15]),
        ];
        for (name, bundle, versions) in cases {
            for &version in versions {
                assert!(
                    !needs_conversion(&bundle(), version),
                    "{name} on {version:?}"
                );
            }
        }
    }

    #[test]
    fn a_blend_of_differing_deltas_is_flagged() {
        // The stock boots' first mesh blends `sk_foot_l` with `dsk_toe_l` and with
        // `dsk_foot_l` (the right side likewise); the second blends nothing, and no
        // vertex blends the two sides.
        let moved = |names: &[&str]| {
            let NativeModelBundle::Fox { model, skl } = fox_with_skl(BOOTS21_F, BOOTS21_S) else {
                unreachable!()
            };
            let mut skl = skl.expect("skl");
            for bone in &mut skl.bones {
                if names.contains(&bone.name.as_str()) {
                    bone.translation[0] += 0.05;
                }
            }
            NativeModelBundle::Fox {
                model,
                skl: Some(skl),
            }
        };
        assert!(needs_conversion(&moved(&["sk_foot_l"]), PesVersion::Pes21));
        assert!(needs_conversion(&moved(&["dsk_foot_l"]), PesVersion::Pes21));
        // The whole left foot moved together keeps one delta: drawn the same.
        assert!(!needs_conversion(
            &moved(&["sk_foot_l", "dsk_toe_l", "dsk_foot_l"]),
            PesVersion::Pes21
        ));
    }

    /// `blends_differing_deltas` over one mesh whose group is every bone in order.
    fn blends(deltas: &[Option<Affine>], rows: &[([u8; 4], [f32; 4])]) -> bool {
        let group: Vec<usize> = (0..deltas.len()).collect();
        let indices: Vec<[u8; 4]> = rows.iter().map(|row| row.0).collect();
        let weights: Vec<[f32; 4]> = rows.iter().map(|row| row.1).collect();
        blends_differing_deltas(
            deltas,
            std::iter::once((
                group.as_slice(),
                Some(indices.as_slice()),
                Some(weights.as_slice()),
            )),
            |weight: f32| weight > 0.0,
        )
    }

    /// A translation by `x` along the x axis.
    fn shifted(x: f32) -> Affine {
        Affine::from_rotation_translation(
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [x, 0.0, 0.0],
        )
    }

    #[test]
    fn only_a_vertex_blending_differing_deltas_counts() {
        let differing = [Some(Affine::IDENTITY), Some(shifted(0.1))];
        assert!(blends(&differing, &[([0, 1, 0, 0], [0.5, 0.5, 0.0, 0.0])]));
        // Each bone alone in its own vertex: moved rigidly, drawn the same.
        assert!(!blends(
            &differing,
            &[
                ([0, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
                ([1, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
            ]
        ));
        // Two bones sharing one delta, however far it is from the identity.
        let shared = [Some(shifted(0.1)), Some(shifted(0.1))];
        assert!(!blends(&shared, &[([0, 1, 0, 0], [0.5, 0.5, 0.0, 0.0])]));
        // A slot with weight zero blends nothing.
        assert!(!blends(&differing, &[([0, 1, 0, 0], [1.0, 0.0, 0.0, 0.0])]));
        // A bone with no delta takes no part, and every pair is compared, not
        // only the first slot against the others.
        let later = [None, Some(Affine::IDENTITY), Some(shifted(0.1))];
        assert!(blends(&later, &[([0, 1, 2, 0], [0.4, 0.3, 0.3, 0.0])]));
        // A weighted slot past the group cannot be proven native.
        assert!(blends(&differing, &[([0, 7, 0, 0], [0.5, 0.5, 0.0, 0.0])]));
        assert!(!blends(&differing, &[([0, 7, 0, 0], [1.0, 0.0, 0.0, 0.0])]));
    }

    /// A pre-Fox bundle with one bone per `(name, bind pose)` and one mesh of `rows`.
    fn prefox_skinned(bones: &[(&str, Affine)], rows: &[([u8; 4], [f32; 4])]) -> NativeModelBundle {
        let positions = vec![[0.0; 3]; rows.len()];
        NativeModelBundle::PreFox {
            model: ::pes_model::model::Model {
                flags: 0,
                bones: bones
                    .iter()
                    .map(|(name, pose)| ::pes_model::format::Bone {
                        name: name.to_string(),
                        matrix: pose.inverse().expect("invertible").0,
                    })
                    .collect(),
                materials: vec!["mat".to_string()],
                meshes: vec![::pes_model::model::Mesh {
                    name: None,
                    extension_headers: vec![],
                    tags: vec![],
                    vertices: ::pes_model::format::MeshVertices {
                        positions: positions.clone(),
                        normals: None,
                        tangents: None,
                        bitangents: None,
                        colors: None,
                        uvs: vec![],
                        bone_indices: Some(rows.iter().map(|row| row.0).collect()),
                        bone_weights: Some(rows.iter().map(|row| row.1).collect()),
                        bone_weight_width: 4,
                    },
                    faces: vec![[0, 0, 0]],
                    lower_lods: vec![],
                    bone_group: (0..bones.len()).collect(),
                    material: 0,
                    bounds: ::pes_model::format::BoundingBox::of(&positions),
                    order: 0,
                    editor_data: vec![],
                }],
                extension_headers: vec![],
                bounds: ::pes_model::format::BoundingBox::of(&positions),
                lod: ::pes_model::format::LodRecord::for_levels(0),
            },
            mtl: ::pes_model::format::mtl::MaterialSet {
                materials: vec![],
                style: ::pes_model::format::mtl::MtlStyle::default(),
            },
        }
    }

    #[test]
    fn a_face_bone_takes_no_part_in_the_pre_check() {
        // `skf_eyelid_b_l` 0.1 off the face table, blended with `sk_head` on the
        // body table: the face bone's pose is the face's own, so nothing differs.
        let tables = skeletons(PesVersion::Pes17);
        let head = tables.body.bone("sk_head").expect("sk_head").matrix;
        let eyelid = shifted(0.1).multiply(
            &tables
                .face
                .bone("skf_eyelid_b_l")
                .expect("skf_eyelid_b_l")
                .matrix,
        );
        let bundle = prefox_skinned(
            &[("sk_head", head), ("skf_eyelid_b_l", eyelid)],
            &[([0, 1, 0, 0], [0.5, 0.5, 0.0, 0.0])],
        );
        assert!(!needs_conversion(&bundle, PesVersion::Pes17));
    }

    #[test]
    fn a_prefox_weight_below_a_fox_step_blends_nothing() {
        // `sk_neck` 0.1 off the table, `sk_head` on it: float noise on the neck's
        // slot (2^-24, as the tracer's boots carry) is no blend; half a 1/255 step is.
        let body = &skeletons(PesVersion::Pes17).body;
        let head = body.bone("sk_head").expect("sk_head").matrix;
        let neck = shifted(0.1).multiply(&body.bone("sk_neck").expect("sk_neck").matrix);
        let bones = [("sk_head", head), ("sk_neck", neck)];
        let noise = 2.0f32.powi(-24);
        assert!(!needs_conversion(
            &prefox_skinned(&bones, &[([0, 1, 0, 0], [1.0, noise, 0.0, 0.0])]),
            PesVersion::Pes17
        ));
        let step = 0.5 / 255.0;
        assert!(needs_conversion(
            &prefox_skinned(&bones, &[([0, 1, 0, 0], [1.0 - step, step, 0.0, 0.0])]),
            PesVersion::Pes17
        ));
    }

    #[test]
    fn a_converted_face_carries_its_own_skf_pose_in_the_skl_it_writes() {
        // The tracer's face sits up to 0.041 off the face table on its `skf_*` bones. Converted
        // for PES 21 they are not re-bound, and the export writes an SKL for them, since a
        // bone off the template's pose needs one for a reimport not to assume the template
        // (`formats/fmdl/export.rs`): the eyelid's SKL pose is the face's own.
        let NativeModelBundle::PreFox { model: source, .. } = prefox(TRACER_FACE_M, TRACER_FACE_T)
        else {
            unreachable!()
        };
        let source_pose = Affine(
            source
                .bones
                .iter()
                .find(|bone| bone.name == "skf_eyelid_b_l")
                .expect("skf_eyelid_b_l")
                .matrix,
        )
        .inverse()
        .expect("invertible");
        let Converted { bundle, .. } =
            convert(prefox(TRACER_FACE_M, TRACER_FACE_T), PesVersion::Pes21).expect("convert");
        let NativeModelBundle::Fox { skl, .. } = bundle else {
            unreachable!()
        };
        let skl = skl.expect("a face off the template's skf_* pose writes an SKL");
        let eyelid = skl
            .bones
            .iter()
            .find(|bone| bone.name == "skf_eyelid_b_l")
            .expect("skf_eyelid_b_l in the SKL");
        let written = Affine::from_rotation_translation(eyelid.rotation, eyelid.translation);
        assert_eq!(bone_moved(&source_pose, &written), Some(false));
        let table = skeletons(PesVersion::Pes21)
            .face
            .bone("skf_eyelid_b_l")
            .expect("table")
            .matrix;
        assert_eq!(bone_moved(&table, &written), Some(true));
    }

    #[test]
    fn same_engine_other_version_checks_the_tables() {
        // Highneck's seven bones all exist in PES18 within tolerance.
        assert!(!needs_conversion(&fox(HIGHNECK), PesVersion::Pes18));
        // The cap's `dsk_deltoid_l`/`dsk_upperarm_long_l` are absent from PES15.
        assert!(needs_conversion(&prefox(CAP_M, CAP_T), PesVersion::Pes15));
    }

    #[test]
    fn prefox_to_fox_goes_through_the_ir() {
        let converted = convert(prefox(CAP_M, CAP_T), PesVersion::Pes21).expect("convert");
        let NativeModelBundle::Fox { model, skl } = converted.bundle else {
            panic!("expected a Fox bundle");
        };
        assert_eq!(
            model
                .bones
                .iter()
                .map(|b| b.name.as_str())
                .collect::<Vec<_>>(),
            [
                "sk_shoulder_l",
                "sk_upperarm_l",
                "dsk_deltoid_l",
                "dsk_upperarm_long_l",
            ]
        );
        assert!(skl.is_none(), "every bone is in PES21's tables");
        assert_eq!(
            codes(&converted.findings),
            [
                ("native_field_dropped", Subject::Mesh(0), "tags"),
                ("skeleton_retargeted", Subject::Model, "1"),
                (
                    "material_parameter_dropped",
                    Subject::Material(0),
                    "SpecularColor"
                ),
                (
                    "material_parameter_dropped",
                    Subject::Material(0),
                    "Shininess"
                ),
                (
                    "dummy_texture_added",
                    Subject::Material(0),
                    "NormalMap_Tex_NRM"
                ),
                (
                    "dummy_texture_added",
                    Subject::Material(0),
                    "SpecularMap_Tex_LIN"
                ),
                ("vertex_bitangents_dropped", Subject::Mesh(0), ""),
            ]
        );
    }

    #[test]
    fn a_bone_moved_off_the_template_pose_writes_an_skl() {
        // PES18's `dsk_deltoid_l` pose differs from PES21's; the Fox export
        // carries it in an SKL, and the converted bundle is already native
        // to PES18 (a second conversion is a no-op).
        let converted = convert(prefox(CAP_M, CAP_T), PesVersion::Pes18).expect("convert");
        let NativeModelBundle::Fox { model, skl } = converted.bundle else {
            panic!("expected a Fox bundle");
        };
        let skl = skl.expect("a moved bone's pose must be recorded");
        assert!(skl.bones.iter().any(|bone| bone.name == "dsk_deltoid_l"));
        assert!(!needs_conversion(
            &NativeModelBundle::Fox {
                model,
                skl: Some(skl)
            },
            PesVersion::Pes18
        ));
    }

    /// The (position, normal xyz, uv0) multiset comparison: vertex order is free, every
    /// component must match within `eps`.
    fn vertex_multiset(
        positions: &[[f32; 3]],
        normals: &[[f32; 3]],
        uvs: &[[f32; 2]],
    ) -> Vec<([f32; 3], [f32; 3], [f32; 2])> {
        let mut rows: Vec<_> = positions
            .iter()
            .zip(normals)
            .zip(uvs)
            .map(|((p, n), uv)| (*p, *n, *uv))
            .collect();
        rows.sort_by(|a, b| {
            (a.0, a.1, a.2)
                .partial_cmp(&(b.0, b.1, b.2))
                .expect("no NaN")
        });
        rows
    }

    /// Faces as position triples rotated smallest-first, then sorted.
    fn face_set(positions: &[[f32; 3]], faces: &[[u16; 3]]) -> Vec<[[f32; 3]; 3]> {
        let mut out: Vec<[[f32; 3]; 3]> = faces
            .iter()
            .map(|face| {
                let pts = face.map(|i| positions[usize::from(i)]);
                let start = (0..3)
                    .min_by(|&a, &b| pts[a].partial_cmp(&pts[b]).expect("no NaN"))
                    .expect("three corners");
                [pts[start], pts[(start + 1) % 3], pts[(start + 2) % 3]]
            })
            .collect();
        out.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
        out
    }

    #[test]
    fn fox_oral_to_pes16_matches_the_legacy_reference() {
        // The fixture's one mesh is hidden, so as it is it converts to nothing; the legacy
        // converter ignored the flag, so the parity runs on the mesh unhidden.
        assert!(matches!(
            convert(fox(ORAL), PesVersion::Pes16),
            Err(ConvertError::EveryMeshHidden)
        ));
        let NativeModelBundle::Fox { mut model, skl } = fox(ORAL) else {
            unreachable!()
        };
        for mesh in &mut model.meshes {
            mesh.shadow_flags &= !crate::materials::to_fox::INVISIBLE_BIT;
        }
        let converted =
            convert(NativeModelBundle::Fox { model, skl }, PesVersion::Pes16).expect("convert");
        // The game's dummy normal and specular maps the FMDL names stand for no map, so
        // leaving them out of the `.mtl` loses nothing (no `material_texture_unused`).
        assert_eq!(
            codes(&converted.findings),
            vec![
                ("native_field_dropped", Subject::Model, "bone_matrices"),
                (
                    "native_field_dropped",
                    Subject::Material(0),
                    "no_shadow_cast"
                ),
            ]
        );
        let NativeModelBundle::PreFox { model, mtl } = converted.bundle else {
            panic!("expected a pre-Fox bundle");
        };
        let legacy = ::pes_model::model::Model::from_file(
            &::pes_model::format::PreFoxModel::read(LEGACY_M).expect("legacy model"),
        )
        .expect("legacy from_file");
        let legacy_mtl = ::pes_model::format::mtl::MaterialSet::read(LEGACY_T).expect("legacy mtl");

        assert_eq!(
            model
                .bones
                .iter()
                .map(|b| b.name.as_str())
                .collect::<Vec<_>>(),
            ["sk_head"]
        );
        let delta =
            Affine(model.bones[0].matrix).max_component_delta(&Affine(legacy.bones[0].matrix));
        assert!(delta < 1e-5, "sk_head matrix delta {delta}");

        assert_eq!(model.meshes.len(), 1);
        let (ours, theirs) = (&model.meshes[0], &legacy.meshes[0]);
        assert_eq!(
            ours.vertices.positions.len(),
            theirs.vertices.positions.len()
        );
        assert_eq!(
            face_set(&ours.vertices.positions, &ours.faces),
            face_set(&theirs.vertices.positions, &theirs.faces)
        );
        let our_normals: Vec<[f32; 3]> = ours
            .vertices
            .normals
            .as_ref()
            .expect("normals")
            .iter()
            .map(|n| [n[0], n[1], n[2]])
            .collect();
        let a = vertex_multiset(
            &ours.vertices.positions,
            &our_normals,
            &ours.vertices.uvs[0],
        );
        let b = vertex_multiset(
            &theirs.vertices.positions,
            theirs.vertices.normals.as_ref().expect("normals"),
            &theirs.vertices.uvs[0],
        );
        assert_eq!(a.len(), b.len());
        for ((pa, na, ua), (pb, nb, ub)) in a.iter().zip(&b) {
            for (x, y) in pa
                .iter()
                .chain(na)
                .chain(ua)
                .zip(pb.iter().chain(nb).chain(ub))
            {
                assert!((x - y).abs() < 1e-6, "{x} vs {y}");
            }
        }

        // The material: the FMDL's Normal/Specular maps are the game's dummies, which
        // stand for no map, so the `Basic_*` ladder gives `Basic_C` with the diffuse
        // sampler alone, as the legacy wrote. The shader, the sampler names, the seven
        // states and DiffuseMap's three attributes equal the legacy's; the path is
        // excluded by design.
        let mat = mtl.materials.first().expect("a material");
        let legacy_mat = legacy_mtl.materials.first().expect("a legacy material");
        assert_eq!(mat.shader, legacy_mat.shader);
        assert_eq!(mat.shader, "Basic_C");
        fn sampler<'a>(
            m: &'a ::pes_model::format::mtl::Material,
            name: &str,
        ) -> Option<&'a ::pes_model::format::mtl::Sampler> {
            m.entries.iter().find_map(|e| match e {
                ::pes_model::format::mtl::MaterialEntry::Sampler(s) if s.name == name => Some(s),
                _ => None,
            })
        }
        let sampler_names = |m: &::pes_model::format::mtl::Material| -> Vec<String> {
            m.entries
                .iter()
                .filter_map(|e| match e {
                    ::pes_model::format::mtl::MaterialEntry::Sampler(s) => Some(s.name.clone()),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(sampler_names(mat), sampler_names(legacy_mat));
        let diffuse = sampler(mat, "DiffuseMap").expect("DiffuseMap");
        assert!(
            diffuse.path.ends_with("dummy_bsm.dds"),
            "DiffuseMap: {}",
            diffuse.path
        );
        let states = |m: &::pes_model::format::mtl::Material| -> BTreeSet<(String, u32)> {
            m.entries
                .iter()
                .filter_map(|e| match e {
                    ::pes_model::format::mtl::MaterialEntry::State(s) => {
                        Some((s.name.clone(), s.value))
                    }
                    _ => None,
                })
                .collect()
        };
        assert_eq!(states(mat), states(legacy_mat));
        let (s, ls) = (
            sampler(mat, "DiffuseMap").expect("DiffuseMap"),
            sampler(legacy_mat, "DiffuseMap").expect("DiffuseMap"),
        );
        assert_eq!(s.srgb, ls.srgb);
        assert_eq!(s.minfilter, ls.minfilter);
        assert_eq!(s.magfilter, ls.magfilter);
    }

    /// The unskinned IR of `unskinned_mesh_gets_the_static_bone`.
    fn unskinned_ir() -> CanonicalModel {
        CanonicalModel {
            bones: vec![],
            meshes: vec![Mesh {
                vertices: Vertices {
                    positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                    ..Vertices::default()
                },
                faces: vec![[0, 1, 2]],
                bone_group: vec![],
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
            source_format: SourceFormat::PreFox,
        }
    }

    #[test]
    fn unskinned_mesh_gets_the_static_bone() {
        let ir = unskinned_ir();
        let exported = crate::formats::fmdl::ir_to_fmdl(&ir).expect("export");
        assert_eq!(
            exported
                .model
                .bones
                .iter()
                .map(|b| b.name.as_str())
                .collect::<Vec<_>>(),
            ["static"]
        );
        assert_eq!(exported.model.bones[0].world_position, [0.2, 0.0, 0.0, 1.0]);
        assert_eq!(exported.model.bones[0].local_position, [0.0; 4]);
        assert_eq!(exported.model.meshes[0].bone_group, vec![0]);
        assert_eq!(
            exported.model.meshes[0].vertices.bone_weights,
            Some(vec![[255, 0, 0, 0]; 3])
        );
        let skl = exported.skl.expect("an SKL for the unknown bone");
        assert_eq!(skl.bones.len(), 1);
        assert_eq!(skl.bones[0].name, "static");
        assert_eq!(
            skl.bones[0].rotation,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        );
        assert_eq!(skl.bones[0].translation, [0.0; 3]);
        assert_eq!(
            codes(&exported.findings),
            vec![
                (
                    "dummy_texture_added",
                    Subject::Material(0),
                    "NormalMap_Tex_NRM"
                ),
                (
                    "dummy_texture_added",
                    Subject::Material(0),
                    "SpecularMap_Tex_LIN"
                ),
                ("static_bone_added", Subject::Mesh(0), ""),
            ]
        );

        // The SKL passed back gives the bind pose; nothing is unknown.
        let Imported { model, findings } =
            crate::formats::fmdl::fmdl_to_ir(&exported.model, Some(&skl)).expect("import");
        assert_eq!(findings, vec![]);
        let mesh = &model.meshes[0];
        assert_eq!(model.bones[0].name, "static");
        assert_eq!(mesh.bone_group, vec![0]);
        assert_eq!(
            mesh.vertices.bone_weights,
            Some(vec![[1.0, 0.0, 0.0, 0.0]; 3])
        );
    }

    #[test]
    fn an_existing_static_bone_is_reused() {
        // The same IR plus a bone already named `static`: the unskinned mesh
        // binds to it rather than gaining a second `static` bone.
        let mut ir = unskinned_ir();
        ir.bones.push(crate::ir::Bone {
            name: "static".to_string(),
            parent: None,
            matrix: Affine::IDENTITY,
            global_position: None,
            local_position: None,
            bounding_box: None,
        });
        let exported = crate::formats::fmdl::ir_to_fmdl(&ir).expect("export");
        assert_eq!(
            exported
                .model
                .bones
                .iter()
                .filter(|b| b.name == "static")
                .count(),
            1
        );
        assert_eq!(exported.model.meshes[0].bone_group, vec![0]);
        assert_eq!(
            exported.model.meshes[0].vertices.bone_weights,
            Some(vec![[255, 0, 0, 0]; 3])
        );
    }

    #[test]
    fn an_undefined_material_is_an_error_not_a_bundle() {
        let NativeModelBundle::PreFox { mut model, mtl } = prefox(CAP_M, CAP_T) else {
            unreachable!()
        };
        model.materials = vec!["nope".to_string()];
        assert!(matches!(
            convert(
                NativeModelBundle::PreFox { model, mtl },
                PesVersion::Pes15
            ),
            Err(ConvertError::MaterialUndefined(name)) if name == "nope"
        ));
    }

    #[test]
    fn a_bone_group_entry_past_the_bone_table_needs_conversion() {
        let NativeModelBundle::Fox { mut model, skl } = fox(ORAL) else {
            unreachable!()
        };
        model.meshes[0].bone_group = vec![9];
        assert!(needs_conversion(
            &NativeModelBundle::Fox { model, skl },
            PesVersion::Pes21
        ));

        let NativeModelBundle::PreFox { mut model, mtl } = prefox(CAP_M, CAP_T) else {
            unreachable!()
        };
        model.meshes[0].bone_group = vec![9];
        assert!(needs_conversion(
            &NativeModelBundle::PreFox { model, mtl },
            PesVersion::Pes17
        ));
    }

    #[test]
    fn a_custom_bone_in_a_used_group_does_not_force_conversion() {
        // A non-table name is skipped, not sent to `version_bone`: the rest
        // of the bundle matches PES21, so it still passes through native.
        let NativeModelBundle::Fox { mut model, skl } = fox(HIGHNECK) else {
            unreachable!()
        };
        model.bones.push(::fmdl::Bone {
            name: "cup_flair".to_string(),
            parent: None,
            bounding_box: ::fmdl::BoundingBox {
                min: [0.0; 4],
                max: [0.0; 4],
            },
            local_position: [0.0; 4],
            world_position: [0.0; 4],
        });
        model.meshes[0].bone_group.push(model.bones.len() - 1);
        assert!(!needs_conversion(
            &NativeModelBundle::Fox { model, skl },
            PesVersion::Pes21
        ));
    }

    #[test]
    fn an_skl_pose_that_differs_needs_conversion() {
        // The SKL is the source pose; the PES21 table is only the fallback
        // for a name the SKL lacks. Highneck's vertices blend the moved bone
        // with bones the SKL does not move.
        let NativeModelBundle::Fox { model, .. } = fox(HIGHNECK) else {
            unreachable!()
        };
        let used = model.meshes[0]
            .bone_group
            .iter()
            .copied()
            .find(|&index| is_standard(&model.bones[index].name))
            .expect("a standard used bone");
        let moved = ::fmdl::format::SklBone {
            name: model.bones[used].name.clone(),
            parent: None,
            rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation: [9.0, 0.0, 0.0],
        };
        let skl = ::fmdl::SklFile {
            bones: vec![moved],
            trailing: Vec::new(),
        };
        assert!(needs_conversion(
            &NativeModelBundle::Fox {
                model,
                skl: Some(skl)
            },
            PesVersion::Pes21
        ));
    }

    #[test]
    fn an_skl_pose_matching_the_table_does_not_convert() {
        // The name lookup must be an equality: with the used bone's SKL pose
        // equal to its PES21 table pose, nothing asks for conversion — even
        // with an unrelated SKL bone present.
        let NativeModelBundle::Fox { model, .. } = fox(HIGHNECK) else {
            unreachable!()
        };
        let used = model.meshes[0]
            .bone_group
            .iter()
            .copied()
            .find(|&index| is_standard(&model.bones[index].name))
            .expect("a standard used bone");
        let name = model.bones[used].name.clone();
        let pose = skeletons(PesVersion::Pes21)
            .body
            .bone(&name)
            .expect("a PES21 table bone")
            .matrix;
        let skl = ::fmdl::SklFile {
            bones: vec![
                ::fmdl::format::SklBone {
                    name,
                    parent: None,
                    rotation: pose.rotation(),
                    translation: pose.translation(),
                },
                ::fmdl::format::SklBone {
                    name: "sk_decoy".to_string(),
                    parent: None,
                    rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    translation: [9.0, 0.0, 0.0],
                },
            ],
            trailing: Vec::new(),
        };
        assert!(!needs_conversion(
            &NativeModelBundle::Fox {
                model,
                skl: Some(skl)
            },
            PesVersion::Pes21
        ));
    }

    #[test]
    fn a_hand_bundle_on_the_hand_pose_needs_no_conversion() {
        // The glove's shared forearm is bound at the PES15 hand pose; with
        // the hand tables checked first the bundle is already native.
        let tables = skeletons(PesVersion::Pes15);
        let inverse = |name: &str| {
            tables
                .hand_l
                .bone(name)
                .expect("hand table")
                .matrix
                .inverse()
                .expect("invertible")
                .0
        };
        let model = ::pes_model::model::Model {
            flags: 0,
            bones: vec![
                ::pes_model::format::Bone {
                    name: "skh_index_mcp_l".to_string(),
                    matrix: inverse("skh_index_mcp_l"),
                },
                ::pes_model::format::Bone {
                    name: "sk_forearm_l".to_string(),
                    matrix: inverse("sk_forearm_l"),
                },
            ],
            materials: vec!["mat".to_string()],
            meshes: vec![::pes_model::model::Mesh {
                name: None,
                extension_headers: vec![],
                tags: vec![],
                vertices: ::pes_model::format::MeshVertices {
                    positions: vec![[0.0; 3]],
                    normals: None,
                    tangents: None,
                    bitangents: None,
                    colors: None,
                    uvs: vec![],
                    bone_indices: Some(vec![[0, 1, 0, 0]]),
                    bone_weights: Some(vec![[0.5, 0.5, 0.0, 0.0]]),
                    bone_weight_width: 4,
                },
                faces: vec![[0, 0, 0]],
                lower_lods: vec![],
                bone_group: vec![0, 1],
                material: 0,
                bounds: ::pes_model::format::BoundingBox::of(&[[0.0; 3]]),
                order: 0,
                editor_data: vec![],
            }],
            extension_headers: vec![],
            bounds: ::pes_model::format::BoundingBox::of(&[[0.0; 3]]),
            lod: ::pes_model::format::LodRecord::for_levels(0),
        };
        assert!(!needs_conversion(
            &NativeModelBundle::PreFox {
                model,
                mtl: ::pes_model::format::mtl::MaterialSet {
                    materials: vec![],
                    style: ::pes_model::format::mtl::MtlStyle::default(),
                },
            },
            PesVersion::Pes15
        ));
    }

    #[test]
    fn a_body_bundle_stays_on_the_body_table() {
        // A `skh_` name in the group without weight is not a hand model: the
        // shared forearm at the *body* pose is already native to PES15. It is
        // blended with the upper arm, which no hand table holds, so a forearm
        // conformed to the hand table would blend two differing deltas.
        let tables = skeletons(PesVersion::Pes15);
        let inverse = |skeleton: &crate::skeletons::Skeleton, name: &str| {
            skeleton
                .bone(name)
                .expect("table")
                .matrix
                .inverse()
                .expect("invertible")
                .0
        };
        let model = ::pes_model::model::Model {
            flags: 0,
            bones: vec![
                ::pes_model::format::Bone {
                    name: "skh_index_mcp_l".to_string(),
                    matrix: inverse(&tables.hand_l, "skh_index_mcp_l"),
                },
                ::pes_model::format::Bone {
                    name: "sk_forearm_l".to_string(),
                    matrix: inverse(&tables.body, "sk_forearm_l"),
                },
                ::pes_model::format::Bone {
                    name: "sk_upperarm_l".to_string(),
                    matrix: inverse(&tables.body, "sk_upperarm_l"),
                },
            ],
            materials: vec!["mat".to_string()],
            meshes: vec![::pes_model::model::Mesh {
                name: None,
                extension_headers: vec![],
                tags: vec![],
                vertices: ::pes_model::format::MeshVertices {
                    positions: vec![[0.0; 3]],
                    normals: None,
                    tangents: None,
                    bitangents: None,
                    colors: None,
                    uvs: vec![],
                    bone_indices: Some(vec![[0, 1, 2, 0]]),
                    // Weight lands on the forearm and upper arm only: `skh_` is a
                    // group entry, not a weighted bone.
                    bone_weights: Some(vec![[0.0, 0.5, 0.5, 0.0]]),
                    bone_weight_width: 4,
                },
                faces: vec![[0, 0, 0]],
                lower_lods: vec![],
                bone_group: vec![0, 1, 2],
                material: 0,
                bounds: ::pes_model::format::BoundingBox::of(&[[0.0; 3]]),
                order: 0,
                editor_data: vec![],
            }],
            extension_headers: vec![],
            bounds: ::pes_model::format::BoundingBox::of(&[[0.0; 3]]),
            lod: ::pes_model::format::LodRecord::for_levels(0),
        };
        assert!(!needs_conversion(
            &NativeModelBundle::PreFox {
                model,
                mtl: ::pes_model::format::mtl::MaterialSet {
                    materials: vec![],
                    style: ::pes_model::format::mtl::MtlStyle::default(),
                },
            },
            PesVersion::Pes15
        ));
    }
}
