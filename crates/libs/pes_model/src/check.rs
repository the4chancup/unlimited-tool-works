//! Model and material-set lint: rules that flag things known to be wrong
//! (or suspicious) in a loaded `Model` or `MaterialSet`. Structural
//! corruption is already an error at `Model::from_file`/`MaterialSet::read`;
//! `check` looks past it at semantics the game cares about. Returns
//! findings with stable codes — consuming tools map codes to their
//! message catalog.
//!
//! Rules, in firing order:
//!
//! - `model_mesh_over_bone_limit` / `model_mesh_over_vertex_limit` /
//!   `model_mesh_over_face_limit` (Error): the mesh exceeds a hard `.model`
//!   limit; the game cannot load it. `ops::split::encode` is the fix.
//! - `model_vertex_far_from_origin` (Error): a vertex more than 5000 units
//!   from the origin; the game lags for the whole matchday.
//! - `model_face_index_out_of_range` (Error): a face of any LOD level
//!   references a vertex that does not exist; the mesh reads out of bounds.
//! - `model_bone_slot_out_of_range` (Warning): a weighted bone index
//!   points past the end of the mesh's bone group; the game ignores that
//!   weight, so the vertex is under-skinned.
//! - `model_weights_not_normalized` (Info): a vertex's weights do not sum
//!   to 1; the skinning result is off by that fraction.
//! - `model_degenerate_face` (Info): a face of any LOD level uses one
//!   vertex twice; it renders nothing and may be leftover loose geometry.
//! - `model_mesh_empty` (Warning): a mesh with no faces renders nothing.
//! - `model_material_unused` (Info): a material name no mesh uses.
//! - `model_duplicate_bone_name` (Warning): a second bone with an already
//!   used name; name-based lookups get ambiguous.
//! - `model_lod_record_mismatch` (Warning): the section-7 LOD record's
//!   level count disagrees with the meshes' LOD levels.
//! - `mtl_material_duplicate` (Error): a material listed twice; binding
//!   by name is ambiguous.
//! - `mtl_state_invalid` (Error): `ztest` is not 1, or `blendmode`/
//!   `alphablend` is not 0/1; the game misrenders or rejects the material.
//! - `mtl_blendmode_nonzero` (Warning): `blendmode` 1 works but is not
//!   recommended.
//! - `mtl_state_nonrecommended` (Info): `alphablend` 1 with `zwrite` 1 —
//!   the transparent state set writes no depth (Konami's own hair does
//!   this, hence Info).
//! - `mtl_state_missing` (Info): one of the seven schema state names is
//!   absent; the game uses its default.
//! - `mtl_state_unknown` (Info): a state name outside the seven the schema
//!   knows (`shadowcaster` in 18 Konami files).
//! - `model_material_undefined` (Error): a `Model` material name with no
//!   entry of that name in the sibling `.mtl`; the mesh renders with the
//!   game's fallback.

use std::collections::{HashMap, HashSet};

use crate::format::mtl::{MaterialEntry, MaterialSet};
use crate::model::Model;
use crate::ops::split::{BONE_LIMIT_HARD, FACE_LIMIT_HARD, VERTEX_LIMIT_HARD, weight_of};

/// How bad a finding is for the model in game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Worth knowing; harmless.
    Info,
    /// Suspicious; the game tolerates it but a modder should look.
    Warning,
    /// The game cannot load or renders wrongly.
    Error,
}

/// What a finding is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    /// The model as a whole.
    Model,
    /// The mesh at this index.
    Mesh(usize),
    /// Index into `Model::materials`.
    Material(usize),
    /// The bone at this index.
    Bone(usize),
    /// Index into `MaterialSet::materials`.
    MtlMaterial(usize),
}

/// One rule that fired; the consuming tool maps `code` to its message
/// catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The stable finding code.
    pub code: &'static str,
    /// Its severity.
    pub severity: Severity,
    /// What it is about.
    pub subject: Subject,
    /// How many items tripped the rule: the faces, vertices or bone slots
    /// inside the subject, or 1 when the subject itself is the item (an
    /// empty mesh, an unused material, a duplicated bone).
    pub count: usize,
}

/// The seven state names the material schema validates.
const STATE_NAMES: [&str; 7] = [
    "ztest",
    "zwrite",
    "twosided",
    "alphatest",
    "alpharef",
    "alphablend",
    "blendmode",
];

/// Every code `check`, `check_materials` and `check_bundle` can report,
/// with its severity, in the module doc's firing order: the one place a
/// code's severity is written, so a tool's message catalog can be tested
/// against it.
pub const CODES: &[(&str, Severity)] = &[
    ("model_mesh_over_bone_limit", Severity::Error),
    ("model_mesh_over_vertex_limit", Severity::Error),
    ("model_mesh_over_face_limit", Severity::Error),
    ("model_vertex_far_from_origin", Severity::Error),
    ("model_face_index_out_of_range", Severity::Error),
    ("model_bone_slot_out_of_range", Severity::Warning),
    ("model_weights_not_normalized", Severity::Info),
    ("model_degenerate_face", Severity::Info),
    ("model_mesh_empty", Severity::Warning),
    ("model_material_unused", Severity::Info),
    ("model_duplicate_bone_name", Severity::Warning),
    ("model_lod_record_mismatch", Severity::Warning),
    ("mtl_material_duplicate", Severity::Error),
    ("mtl_state_invalid", Severity::Error),
    ("mtl_blendmode_nonzero", Severity::Warning),
    ("mtl_state_nonrecommended", Severity::Info),
    ("mtl_state_missing", Severity::Info),
    ("mtl_state_unknown", Severity::Info),
    ("model_material_undefined", Severity::Error),
];

/// A finding of `code`, at the severity `CODES` gives it.
fn finding(code: &'static str, subject: Subject, count: usize) -> Finding {
    let severity = CODES
        .iter()
        .find(|(known, _)| *known == code)
        .map(|(_, severity)| *severity)
        .expect("every code the checks report is listed in `CODES`");
    Finding {
        code,
        severity,
        subject,
        count,
    }
}

/// Far geometry lags the game for the whole matchday.
const MAX_DISTANCE_FROM_ORIGIN: f32 = 5000.0;

/// Rules over a `Model`, in model order (meshes, materials, bones, then
/// the model).
pub fn check(model: &Model) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut used_materials = HashSet::new();

    for (index, mesh) in model.meshes.iter().enumerate() {
        let vertex_count = mesh.vertices.positions.len();
        used_materials.insert(mesh.material);
        let subject = Subject::Mesh(index);

        // The game cannot load a mesh over a hard limit; splitting with
        // `ops::split::encode` is the fix.
        if mesh.bone_group.len() > BONE_LIMIT_HARD {
            findings.push(finding(
                "model_mesh_over_bone_limit",
                subject.clone(),
                mesh.bone_group.len(),
            ));
        }
        if vertex_count > VERTEX_LIMIT_HARD {
            findings.push(finding(
                "model_mesh_over_vertex_limit",
                subject.clone(),
                vertex_count,
            ));
        }
        if mesh.faces.len() > FACE_LIMIT_HARD {
            findings.push(finding(
                "model_mesh_over_face_limit",
                subject.clone(),
                mesh.faces.len(),
            ));
        }

        // f64: a huge coordinate cannot overflow the squared-distance sum.
        let limit = f64::from(MAX_DISTANCE_FROM_ORIGIN) * f64::from(MAX_DISTANCE_FROM_ORIGIN);
        let far = mesh
            .vertices
            .positions
            .iter()
            .filter(|&position| {
                let [x, y, z] = position.map(f64::from);
                x * x + y * y + z * z > limit
            })
            .count();
        if far > 0 {
            findings.push(finding(
                "model_vertex_far_from_origin",
                subject.clone(),
                far,
            ));
        }

        let out_of_range = mesh
            .faces
            .iter()
            .chain(mesh.lower_lods.iter().flatten())
            .filter(|face| {
                face.iter()
                    .any(|&vertex| usize::from(vertex) >= vertex_count)
            })
            .count();
        if out_of_range > 0 {
            findings.push(finding(
                "model_face_index_out_of_range",
                subject.clone(),
                out_of_range,
            ));
        }

        if let Some(indices) = &mesh.vertices.bone_indices {
            // A slot past the end of the group whose weight is zero never
            // loads, so only weighted slots count.
            let bad_slots = indices
                .iter()
                .enumerate()
                .filter(|(vertex, index_row)| {
                    index_row.iter().enumerate().any(|(component, &slot)| {
                        usize::from(slot) >= mesh.bone_group.len()
                            && weight_of(&mesh.vertices, *vertex, component) > 0.0
                    })
                })
                .count();
            if bad_slots > 0 {
                findings.push(finding(
                    "model_bone_slot_out_of_range",
                    subject.clone(),
                    bad_slots,
                ));
            }
        }
        if let Some(weights) = &mesh.vertices.bone_weights {
            let unnormalized = weights
                .iter()
                .filter(|row| {
                    let sum: f32 = row.iter().sum();
                    (sum - 1.0).abs() > 1e-3
                })
                .count();
            if unnormalized > 0 {
                findings.push(finding(
                    "model_weights_not_normalized",
                    subject.clone(),
                    unnormalized,
                ));
            }
        }

        let degenerate = mesh
            .faces
            .iter()
            .chain(mesh.lower_lods.iter().flatten())
            .filter(|face| face[0] == face[1] || face[1] == face[2] || face[0] == face[2])
            .count();
        if degenerate > 0 {
            findings.push(finding(
                "model_degenerate_face",
                subject.clone(),
                degenerate,
            ));
        }

        if mesh.faces.is_empty() {
            findings.push(finding("model_mesh_empty", subject, 1));
        }
    }

    for (index, _) in model.materials.iter().enumerate() {
        if !used_materials.contains(&index) {
            findings.push(finding(
                "model_material_unused",
                Subject::Material(index),
                1,
            ));
        }
    }

    let mut seen = HashSet::new();
    for (index, bone) in model.bones.iter().enumerate() {
        if !seen.insert(bone.name.as_str()) {
            findings.push(finding(
                "model_duplicate_bone_name",
                Subject::Bone(index),
                1,
            ));
        }
    }

    // The LOD record's level count: 0 when no mesh has lower levels, else
    // the largest 1 + lower_lods over the meshes.
    let expected = model
        .meshes
        .iter()
        .map(|mesh| mesh.lower_lods.len() + 1)
        .max()
        .unwrap_or(0);
    let expected = if expected <= 1 { 0 } else { expected } as u32;
    if model.lod.level_count != expected {
        findings.push(finding("model_lod_record_mismatch", Subject::Model, 1));
    }

    findings
}

/// Rules over a `.mtl`, in material order.
pub fn check_materials(set: &MaterialSet) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut seen = HashSet::new();
    for (index, material) in set.materials.iter().enumerate() {
        let subject = Subject::MtlMaterial(index);
        if !seen.insert(material.name.as_str()) {
            findings.push(finding("mtl_material_duplicate", subject.clone(), 1));
        }

        // A state name appearing twice is legal XML and unremarkable;
        // the first value wins for every value rule below, as
        // `model_convert`'s import does.
        let mut states: HashMap<&str, u32> = HashMap::new();
        let mut unknown = 0;
        for entry in &material.entries {
            if let MaterialEntry::State(state) = entry {
                if STATE_NAMES.contains(&state.name.as_str()) {
                    states.entry(state.name.as_str()).or_insert(state.value);
                } else {
                    unknown += 1;
                }
            }
        }

        let mut invalid = 0;
        if states.get("ztest").is_some_and(|&value| value != 1) {
            invalid += 1;
        }
        for name in ["blendmode", "alphablend"] {
            if states.get(name).is_some_and(|&value| value > 1) {
                invalid += 1;
            }
        }
        if invalid > 0 {
            findings.push(finding("mtl_state_invalid", subject.clone(), invalid));
        }
        if states.get("blendmode") == Some(&1) {
            findings.push(finding("mtl_blendmode_nonzero", subject.clone(), 1));
        }
        if states.get("alphablend") == Some(&1) && states.get("zwrite") == Some(&1) {
            findings.push(finding("mtl_state_nonrecommended", subject.clone(), 1));
        }
        let missing = STATE_NAMES
            .iter()
            .filter(|name| !states.contains_key(*name))
            .count();
        if missing > 0 {
            findings.push(finding("mtl_state_missing", subject.clone(), missing));
        }
        if unknown > 0 {
            findings.push(finding("mtl_state_unknown", subject, unknown));
        }
    }
    findings
}

/// `check` + `check_materials` + the rules that need both (a `.model`
/// with its sibling `.mtl`).
pub fn check_bundle(model: &Model, set: &MaterialSet) -> Vec<Finding> {
    let mut findings = check(model);
    findings.extend(check_materials(set));
    let defined: HashSet<&str> = set
        .materials
        .iter()
        .map(|material| material.name.as_str())
        .collect();
    for (index, name) in model.materials.iter().enumerate() {
        if !defined.contains(name.as_str()) {
            findings.push(finding(
                "model_material_undefined",
                Subject::Material(index),
                1,
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::PreFoxModel;
    use crate::format::fixtures::*;
    use crate::format::mtl::{Material, MaterialEntry, MtlStyle, Sampler, State};

    fn load(bytes: &[u8]) -> Model {
        Model::from_file(&PreFoxModel::read(bytes).unwrap()).unwrap()
    }

    fn set(bytes: &[u8]) -> MaterialSet {
        MaterialSet::read(bytes).unwrap()
    }

    fn finding(code: &'static str, severity: Severity, subject: Subject, count: usize) -> Finding {
        Finding {
            code,
            severity,
            subject,
            count,
        }
    }

    fn material(name: &str, entries: Vec<MaterialEntry>) -> Material {
        Material {
            name: name.to_owned(),
            shader: "s".to_owned(),
            entries,
        }
    }

    fn materials(materials: Vec<Material>) -> MaterialSet {
        MaterialSet {
            materials,
            style: MtlStyle::default(),
        }
    }

    #[test]
    fn the_code_list_is_the_module_doc_s_rules_in_firing_order() {
        let codes: Vec<(&str, Severity)> = CODES.to_vec();
        assert_eq!(
            codes,
            [
                ("model_mesh_over_bone_limit", Severity::Error),
                ("model_mesh_over_vertex_limit", Severity::Error),
                ("model_mesh_over_face_limit", Severity::Error),
                ("model_vertex_far_from_origin", Severity::Error),
                ("model_face_index_out_of_range", Severity::Error),
                ("model_bone_slot_out_of_range", Severity::Warning),
                ("model_weights_not_normalized", Severity::Info),
                ("model_degenerate_face", Severity::Info),
                ("model_mesh_empty", Severity::Warning),
                ("model_material_unused", Severity::Info),
                ("model_duplicate_bone_name", Severity::Warning),
                ("model_lod_record_mismatch", Severity::Warning),
                ("mtl_material_duplicate", Severity::Error),
                ("mtl_state_invalid", Severity::Error),
                ("mtl_blendmode_nonzero", Severity::Warning),
                ("mtl_state_nonrecommended", Severity::Info),
                ("mtl_state_missing", Severity::Info),
                ("mtl_state_unknown", Severity::Info),
                ("model_material_undefined", Severity::Error),
            ]
        );
    }

    #[test]
    fn fixtures_are_clean() {
        for bytes in ALL
            .iter()
            .filter(|bytes| **bytes != COMMUNITY_EMPTY_GEOMETRY)
        {
            let findings = check(&load(bytes));
            assert!(findings.is_empty(), "{findings:?}");
        }
        // The boots template's two meshes are legitimately empty.
        assert_eq!(
            check(&load(COMMUNITY_EMPTY_GEOMETRY)),
            [
                finding("model_mesh_empty", Severity::Warning, Subject::Mesh(0), 1,),
                finding("model_mesh_empty", Severity::Warning, Subject::Mesh(1), 1,),
            ]
        );
    }

    #[test]
    fn over_limits() {
        let mut model = load(CARD);
        model.meshes[0].bone_group = vec![0; 65];
        let findings = check(&model);
        assert!(findings.contains(&finding(
            "model_mesh_over_bone_limit",
            Severity::Error,
            Subject::Mesh(0),
            65,
        )));

        let mut model = load(CARD);
        let face = model.meshes[0].faces[0];
        model.meshes[0].faces = vec![face; 21846];
        let findings = check(&model);
        assert!(findings.contains(&finding(
            "model_mesh_over_face_limit",
            Severity::Error,
            Subject::Mesh(0),
            21846,
        )));

        let mut model = load(CARD);
        let vertices = &mut model.meshes[0].vertices;
        let last = vertices.positions.len() - 1;
        while vertices.positions.len() <= 65535 {
            vertices.positions.push(vertices.positions[last]);
            if let Some(normals) = &mut vertices.normals {
                normals.push(normals[last]);
            }
            if let Some(tangents) = &mut vertices.tangents {
                tangents.push(tangents[last]);
            }
            if let Some(bitangents) = &mut vertices.bitangents {
                bitangents.push(bitangents[last]);
            }
            if let Some(colors) = &mut vertices.colors {
                colors.push(colors[last]);
            }
            for uvs in &mut vertices.uvs {
                uvs.push(uvs[last]);
            }
            if let Some(indices) = &mut vertices.bone_indices {
                indices.push(indices[last]);
            }
            if let Some(weights) = &mut vertices.bone_weights {
                weights.push(weights[last]);
            }
        }
        let findings = check(&model);
        assert!(findings.contains(&finding(
            "model_mesh_over_vertex_limit",
            Severity::Error,
            Subject::Mesh(0),
            65536,
        )));
    }

    #[test]
    fn at_limits() {
        // Each hard limit itself is allowed.
        let mut model = load(CARD);
        model.meshes[0].bone_group = vec![0; 64];
        assert!(
            !check(&model)
                .iter()
                .any(|finding| finding.code == "model_mesh_over_bone_limit")
        );

        let mut model = load(CARD);
        model.meshes[0].faces = vec![model.meshes[0].faces[0]; 21845];
        assert!(
            !check(&model)
                .iter()
                .any(|finding| finding.code == "model_mesh_over_face_limit")
        );

        let mut model = load(CARD);
        let vertices = &mut model.meshes[0].vertices;
        let last = vertices.positions.len() - 1;
        while vertices.positions.len() < 65535 {
            vertices.positions.push(vertices.positions[last]);
            if let Some(normals) = &mut vertices.normals {
                normals.push(normals[last]);
            }
            if let Some(tangents) = &mut vertices.tangents {
                tangents.push(tangents[last]);
            }
            if let Some(bitangents) = &mut vertices.bitangents {
                bitangents.push(bitangents[last]);
            }
            if let Some(colors) = &mut vertices.colors {
                colors.push(colors[last]);
            }
            for uvs in &mut vertices.uvs {
                uvs.push(uvs[last]);
            }
            if let Some(indices) = &mut vertices.bone_indices {
                indices.push(indices[last]);
            }
            if let Some(weights) = &mut vertices.bone_weights {
                weights.push(weights[last]);
            }
        }
        assert!(
            !check(&model)
                .iter()
                .any(|finding| finding.code == "model_mesh_over_vertex_limit")
        );
    }

    #[test]
    fn vertices_past_5000_units_from_the_origin_are_errors() {
        let mut model = load(CARD);
        {
            let positions = &mut model.meshes[0].vertices.positions;
            // 5657 away, only by the Euclidean sum.
            positions[0] = [4000.0, 4000.0, 0.0];
            positions[1] = [0.0, 0.0, -6000.0];
            // Exactly on the limit: passes.
            positions[2] = [5000.0, 0.0, 0.0];
            // Loose, referenced by no face: still counts.
            positions.push([1e30, 0.0, 0.0]);
        }
        let findings = check(&model);
        let mut matching = findings
            .iter()
            .filter(|finding| finding.code == "model_vertex_far_from_origin");
        assert_eq!(
            matching.next(),
            Some(&finding(
                "model_vertex_far_from_origin",
                Severity::Error,
                Subject::Mesh(0),
                3,
            ))
        );
        assert!(matching.next().is_none());

        let mut model = load(CARD);
        {
            let positions = &mut model.meshes[0].vertices.positions;
            positions[0] = [4000.0, 2999.0, 0.0];
            positions[1] = [0.0, 0.0, -4999.0];
            positions[2] = [5000.0, 0.0, 0.0];
            positions.push([1.0, 1.0, 1.0]);
        }
        assert!(
            !check(&model)
                .iter()
                .any(|finding| finding.code == "model_vertex_far_from_origin")
        );
    }

    #[test]
    fn face_index_out_of_range() {
        let mut model = load(CARD);
        model.meshes[0].faces[0] = [0, 1, 999];
        let findings = check(&model);
        assert!(findings.contains(&finding(
            "model_face_index_out_of_range",
            Severity::Error,
            Subject::Mesh(0),
            1,
        )));

        let mut model = load(COLLAR);
        model.meshes[0].lower_lods[0][0] = [0, 1, 9999];
        let findings = check(&model);
        assert!(findings.contains(&finding(
            "model_face_index_out_of_range",
            Severity::Error,
            Subject::Mesh(0),
            1,
        )));
    }

    #[test]
    fn bone_slot_out_of_range() {
        // The card stores bone indices without weights: slot 0 binds with
        // weight 1.
        let mut model = load(CARD);
        model.meshes[0].vertices.bone_indices.as_mut().unwrap()[0][0] = 7;
        let findings = check(&model);
        assert!(findings.contains(&finding(
            "model_bone_slot_out_of_range",
            Severity::Warning,
            Subject::Mesh(0),
            1,
        )));

        let mut model = load(CARD);
        model.meshes[0].vertices.bone_indices.as_mut().unwrap()[0][1] = 7;
        let findings = check(&model);
        assert!(
            !findings
                .iter()
                .any(|finding| finding.code == "model_bone_slot_out_of_range")
        );
    }

    #[test]
    fn weights_not_normalized() {
        let mut model = load(CAP);
        model.meshes[0].vertices.bone_weights.as_mut().unwrap()[0] = [0.5, 0.1, 0.0, 0.0];
        let findings = check(&model);
        assert!(findings.contains(&finding(
            "model_weights_not_normalized",
            Severity::Info,
            Subject::Mesh(0),
            1,
        )));

        let mut model = load(CAP);
        model.meshes[0].vertices.bone_weights.as_mut().unwrap()[0] = [0.3335, 0.3335, 0.333, 0.0];
        let findings = check(&model);
        assert!(
            !findings
                .iter()
                .any(|finding| finding.code == "model_weights_not_normalized")
        );
    }

    #[test]
    fn degenerate_face() {
        let mut model = load(CARD);
        model.meshes[0].faces[0] = [1, 1, 2];
        let findings = check(&model);
        assert!(findings.contains(&finding(
            "model_degenerate_face",
            Severity::Info,
            Subject::Mesh(0),
            1,
        )));

        // Two equal indices, whichever pair they are.
        let mut model = load(CARD);
        model.meshes[0].faces[0] = [1, 2, 2];
        assert!(check(&model).contains(&finding(
            "model_degenerate_face",
            Severity::Info,
            Subject::Mesh(0),
            1,
        )));
    }

    #[test]
    fn model_level_rules() {
        let mut model = load(CARD);
        model.meshes[0].faces.clear();
        let findings = check(&model);
        assert!(findings.contains(&finding(
            "model_mesh_empty",
            Severity::Warning,
            Subject::Mesh(0),
            1,
        )));

        let mut model = load(CARD);
        model.materials.push("nobody_uses_this".to_owned());
        let findings = check(&model);
        assert!(findings.contains(&finding(
            "model_material_unused",
            Severity::Info,
            Subject::Material(1),
            1,
        )));

        let mut model = load(CARD);
        let duplicate = model.bones[0].clone();
        model.bones.push(duplicate);
        let at = model.bones.len() - 1;
        let findings = check(&model);
        assert!(findings.contains(&finding(
            "model_duplicate_bone_name",
            Severity::Warning,
            Subject::Bone(at),
            1,
        )));

        let mut model = load(COLLAR);
        model.lod.level_count = 5;
        let findings = check(&model);
        assert!(findings.contains(&finding(
            "model_lod_record_mismatch",
            Severity::Warning,
            Subject::Model,
            1,
        )));

        let mut model = load(CARD);
        model.lod.level_count = 2;
        let findings = check(&model);
        assert!(findings.contains(&finding(
            "model_lod_record_mismatch",
            Severity::Warning,
            Subject::Model,
            1,
        )));
    }

    #[test]
    fn mtl_fixtures() {
        assert!(check_materials(&set(CARDHEAD_MTL)).is_empty());
        assert_eq!(
            check_materials(&set(HAIR_MTL)),
            [finding(
                "mtl_state_nonrecommended",
                Severity::Info,
                Subject::MtlMaterial(0),
                1,
            )]
        );
        assert_eq!(
            check_materials(&set(SHADOW_MTL)),
            [finding(
                "mtl_state_missing",
                Severity::Info,
                Subject::MtlMaterial(0),
                7,
            )]
        );
        for bytes in ALL_MTL {
            assert!(
                check_materials(&set(bytes))
                    .iter()
                    .all(|finding| finding.severity < Severity::Error)
            );
        }
    }

    #[test]
    fn mtl_rules() {
        let state = |name: &str, value: u32| {
            MaterialEntry::State(State {
                name: name.to_owned(),
                value,
            })
        };
        let sampler = MaterialEntry::Sampler(Sampler {
            name: "n".to_owned(),
            path: "p".to_owned(),
            srgb: None,
            minfilter: None,
            maxfilter: None,
            magfilter: None,
            mipfilter: None,
            uaddr: None,
            vaddr: None,
            waddr: None,
            maxaniso: None,
        });
        let set = materials(vec![
            material("a", vec![sampler.clone()]),
            material("a", vec![sampler.clone()]),
        ]);
        let findings = check_materials(&set);
        assert!(findings.contains(&finding(
            "mtl_material_duplicate",
            Severity::Error,
            Subject::MtlMaterial(1),
            1,
        )));

        let set = materials(vec![material(
            "m",
            vec![
                state("ztest", 0),
                state("blendmode", 3),
                state("alphablend", 2),
            ],
        )]);
        let findings = check_materials(&set);
        assert!(findings.contains(&finding(
            "mtl_state_invalid",
            Severity::Error,
            Subject::MtlMaterial(0),
            3,
        )));

        // A state listed twice keeps its first value, as the importer
        // does: `ztest` is 1 here, not 0.
        let set = materials(vec![material(
            "m",
            vec![state("ztest", 1), state("ztest", 0)],
        )]);
        let findings = check_materials(&set);
        assert!(
            !findings
                .iter()
                .any(|finding| finding.code == "mtl_state_invalid")
        );

        let set = materials(vec![material("m", vec![state("blendmode", 1)])]);
        let findings = check_materials(&set);
        assert!(findings.contains(&finding(
            "mtl_blendmode_nonzero",
            Severity::Warning,
            Subject::MtlMaterial(0),
            1,
        )));

        let set = materials(vec![material("m", vec![state("shadowcaster", 1)])]);
        let findings = check_materials(&set);
        assert!(findings.contains(&finding(
            "mtl_state_unknown",
            Severity::Info,
            Subject::MtlMaterial(0),
            1,
        )));
    }

    #[test]
    fn bundle() {
        let model = load(CAP);
        let findings = check_bundle(&model, &set(CAP_MTL));
        assert!(
            findings
                .iter()
                .all(|finding| finding.severity < Severity::Error)
        );

        let mut model = load(CAP);
        model.materials[0] = "nope".to_owned();
        let findings = check_bundle(&model, &set(CAP_MTL));
        assert!(findings.contains(&finding(
            "model_material_undefined",
            Severity::Error,
            Subject::Material(0),
            1,
        )));

        // The `.mtl` rules run too: a material name listed twice.
        let findings = check_bundle(
            &load(CAP),
            &materials(vec![material("a", vec![]), material("a", vec![])]),
        );
        assert!(findings.contains(&finding(
            "mtl_material_duplicate",
            Severity::Error,
            Subject::MtlMaterial(1),
            1,
        )));
    }
}
