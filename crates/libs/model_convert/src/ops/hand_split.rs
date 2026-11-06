//! The hand auto-split: separating glove geometry (vertices weighted to `skh_` hand bones)
//! from a body model. A vertex class (same position, bone row and weight bits, so a UV
//! seam stays one vertex) is selected when any entry carries weight on one of
//! the hand's `skh_` bones; the selection then grows once along faces, a face touching a
//! seed class selecting every class it touches. The glove part gets the faces fully
//! inside the selection, the body the rest, in place — no geometric cut, no
//! reweighting, positions and weights untouched.

use std::collections::HashMap;

use crate::ir::{
    Bone, CanonicalModel, Mesh, MeshGroup, Vertices, bone_is_weighted, rebuild_bone_list,
    remap_bone_group, validate,
};

/// Which hand a bone or a split part belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hand {
    /// `_l` suffixed.
    Left,
    /// `_r` suffixed.
    Right,
}

/// A model split at the wrists: the body keeps every face not fully inside a hand
/// selection; a glove is `Some` only when its hand had weighted `skh_`
/// vertices.
#[derive(Debug, Clone, PartialEq)]
pub struct HandSplit {
    /// The model with the hand geometry removed.
    pub body: CanonicalModel,
    /// The left-hand part, when any vertex weights an `skh_*_l` bone.
    pub glove_l: Option<CanonicalModel>,
    /// The right-hand part, when any vertex weights an `skh_*_r` bone.
    pub glove_r: Option<CanonicalModel>,
}

/// Whether `bone` is a hand-skeleton-exclusive bone (`skh_` prefix) and for which hand
/// (`_l` / `_r` suffix); `None` for every other name.
pub(crate) fn hand_of(bone: &str) -> Option<Hand> {
    if !bone.starts_with("skh_") {
        return None;
    }
    if bone.ends_with("_l") {
        Some(Hand::Left)
    } else if bone.ends_with("_r") {
        Some(Hand::Right)
    } else {
        None
    }
}

/// The smallest `f32` weight that counts as a slot carrying weight, in the hand detectors
/// and the conversion pre-check: half of Fox's 1/255 weight step, so a weight the Fox format
/// would round to zero carries nothing on either engine. Community exporters leave float
/// noise on the unused slots of a vertex weighted fully to one bone (2^-24 on the other
/// leg's bone, on 225 vertices of the tracer's boots), which moves the drawn vertex by a
/// negligible amount; with `> 0.0` one such slot on a hand bone would make a body a hand
/// model, and the pre-check would flag such a model on every version.
pub(crate) const MIN_CARRIED_WEIGHT: f32 = 0.5 / 255.0;

/// Whether any vertex carries weight (at least [`MIN_CARRIED_WEIGHT`]) on a hand bone.
/// Names in a bone list do not count, weights do.
pub(crate) fn has_hand_weights(ir: &CanonicalModel) -> bool {
    ir.meshes.iter().any(|mesh| {
        let (Some(indices), Some(weights)) =
            (&mesh.vertices.bone_indices, &mesh.vertices.bone_weights)
        else {
            return false;
        };
        indices.iter().zip(weights).any(|(row, ws)| {
            row.iter().enumerate().any(|(slot, &entry)| {
                ws[slot] >= MIN_CARRIED_WEIGHT
                    && hand_of(&ir.bones[mesh.bone_group[usize::from(entry)]].name).is_some()
            })
        })
    })
}

/// Whether the FMDL `model` carries a positive weight on a hand-skeleton bone (`skh_*_l` or
/// `skh_*_r`), the hand auto-split's detection rule: read from the model's own bone weights
/// and bone names, with no IR import, and only a weighted slot counts, so a hand bone that
/// a bone group lists without any vertex weighing on it does not.
pub fn fox_has_hand_weights(model: &::fmdl::Model) -> bool {
    model.meshes.iter().any(|mesh| {
        let (Some(indices), Some(weights)) =
            (&mesh.vertices.bone_indices, &mesh.vertices.bone_weights)
        else {
            return false;
        };
        indices.iter().zip(weights).any(|(row, ws)| {
            row.iter().enumerate().any(|(slot, &entry)| {
                ws[slot] > 0
                    && mesh
                        .bone_group
                        .get(usize::from(entry))
                        .and_then(|&bone| model.bones.get(bone))
                        .is_some_and(|bone| hand_of(&bone.name).is_some())
            })
        })
    })
}

/// Whether the pre-Fox `model` (a `.model`) carries weight (half a Fox weight step or more,
/// so exporter float noise does not count) on a hand-skeleton bone (`skh_*_l` or
/// `skh_*_r`), the hand auto-split's detection rule: read from the model's own bone weights
/// and bone names, with no IR import, and only a weighted slot counts, so a hand bone that a
/// bone group lists without any vertex weighing on it does not. A model with no bone weights
/// at all has none.
pub fn prefox_has_hand_weights(model: &::pes_model::model::Model) -> bool {
    model.meshes.iter().any(|mesh| {
        let (Some(indices), Some(weights)) =
            (&mesh.vertices.bone_indices, &mesh.vertices.bone_weights)
        else {
            return false;
        };
        indices.iter().zip(weights).any(|(row, ws)| {
            row.iter().enumerate().any(|(slot, &entry)| {
                ws[slot] >= MIN_CARRIED_WEIGHT
                    && mesh
                        .bone_group
                        .get(usize::from(entry))
                        .and_then(|&bone| model.bones.get(bone))
                        .is_some_and(|bone| hand_of(&bone.name).is_some())
            })
        })
    })
}

/// The topological identity key: position bits, the bone index row, the weight bits —
/// the same fields `fmdl::ops::vertex_enc`'s `topological_key` groups on. UVs are
/// deliberately absent: a UV seam is two entries, one topological vertex.
fn class_key(mesh: &Mesh, index: usize) -> Vec<u8> {
    let vertices = &mesh.vertices;
    let mut key = Vec::new();
    for component in vertices.positions[index] {
        key.extend(component.to_le_bytes());
    }
    if let Some(indices) = &vertices.bone_indices {
        key.extend(indices[index]);
    }
    if let Some(weights) = &vertices.bone_weights {
        for component in weights[index] {
            key.extend(component.to_le_bytes());
        }
    }
    key
}

/// The entry → topological-class map for `mesh`: entries sharing position, bone
/// indices and bone weights are one vertex.
fn classes(mesh: &Mesh) -> Vec<usize> {
    let mut ids: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut class = Vec::with_capacity(mesh.vertices.len());
    for index in 0..mesh.vertices.len() {
        let next = ids.len();
        class.push(*ids.entry(class_key(mesh, index)).or_insert(next));
    }
    class
}

/// The per-class selection for `hand` on `mesh`: seeds (a carried weight on a slot
/// whose group bone is `hand`'s) grown once along `grow_faces` — the faces still
/// present, which for the right hand excludes what the left separation took.
/// `class` is the entry → class map from [`classes`]; the returned vector is
/// indexed by class.
fn selected(
    mesh: &Mesh,
    class: &[usize],
    bones: &[Bone],
    hand: Hand,
    grow_faces: &[[u16; 3]],
) -> Vec<bool> {
    let classes = class.iter().copied().max().map_or(0, |max| max + 1);
    let mut sel = vec![false; classes];
    if let (Some(indices), Some(weights)) =
        (&mesh.vertices.bone_indices, &mesh.vertices.bone_weights)
    {
        for (vertex, (row, ws)) in indices.iter().zip(weights).enumerate() {
            sel[class[vertex]] |= row.iter().enumerate().any(|(slot, &entry)| {
                ws[slot] >= MIN_CARRIED_WEIGHT
                    && hand_of(&bones[mesh.bone_group[usize::from(entry)]].name) == Some(hand)
            });
        }
    }
    // Grow once: one pass over the faces against the seed set (chaining off a
    // just-grown class would be a flood fill, not one Select More).
    let seeds = sel.clone();
    for face in grow_faces {
        if face.iter().any(|&i| seeds[class[usize::from(i)]]) {
            for &i in face {
                sel[class[usize::from(i)]] = true;
            }
        }
    }
    sel
}

/// `mesh` restricted to `faces`, vertices re-indexed in first-use order and every present
/// vertex column copied.
fn part_mesh(mesh: &Mesh, faces: &[[u16; 3]]) -> Mesh {
    // `None` = not emitted yet; a u16 face table can name at most 65,536 distinct
    // vertices, so `u16::MAX` is free to mean a real index only through Option.
    let mut remap: Vec<Option<u16>> = vec![None; mesh.vertices.len()];
    let mut order: Vec<usize> = Vec::new();
    let faces: Vec<[u16; 3]> = faces
        .iter()
        .map(|face| {
            face.map(|i| {
                let i = usize::from(i);
                *remap[i].get_or_insert_with(|| {
                    let new = u16::try_from(order.len())
                        .expect("u16 face indices admit at most 65,536 vertices");
                    order.push(i);
                    new
                })
            })
        })
        .collect();
    fn take<T: Clone>(column: &[T], order: &[usize]) -> Vec<T> {
        order.iter().map(|&i| column[i].clone()).collect()
    }
    let v = &mesh.vertices;
    Mesh {
        vertices: Vertices {
            positions: take(&v.positions, &order),
            normals: v.normals.as_ref().map(|c| take(c, &order)),
            tangents: v.tangents.as_ref().map(|c| take(c, &order)),
            bitangents: v.bitangents.as_ref().map(|c| take(c, &order)),
            colors: v.colors.as_ref().map(|c| take(c, &order)),
            uvs: v.uvs.iter().map(|set| take(set, &order)).collect(),
            uv_high_precision: v.uv_high_precision.clone(),
            bone_indices: v.bone_indices.as_ref().map(|c| take(c, &order)),
            bone_weights: v.bone_weights.as_ref().map(|c| take(c, &order)),
            bone_weight_width: v.bone_weight_width,
        },
        faces,
        bone_group: mesh.bone_group.clone(),
        material: mesh.material,
        extension_headers: mesh.extension_headers.clone(),
        custom_bounding_box: mesh.custom_bounding_box,
    }
}

/// The model made of the mesh parts `parts` gives (`None` drops the mesh): mesh indices
/// in groups remap, a group survives when it lists a surviving mesh or is an ancestor of
/// one, and unused bones, materials and textures are pruned — a glove keeps only the
/// materials its faces use. Extension headers and the source format copy whole.
fn assemble(source: &CanonicalModel, parts: Vec<Option<Mesh>>) -> CanonicalModel {
    let mut mesh_of = vec![usize::MAX; source.meshes.len()];
    let mut meshes = Vec::new();
    for (old, part) in parts.into_iter().enumerate() {
        if let Some(mesh) = part {
            mesh_of[old] = meshes.len();
            meshes.push(mesh);
        }
    }
    // Parents precede children, so one reverse pass decides a group's survival.
    let mut keep = vec![false; source.mesh_groups.len()];
    for g in (0..source.mesh_groups.len()).rev() {
        keep[g] = source.mesh_groups[g]
            .meshes
            .iter()
            .any(|&m| mesh_of[m] != usize::MAX)
            || source
                .mesh_groups
                .iter()
                .enumerate()
                .skip(g + 1)
                .any(|(h, child)| keep[h] && child.parent == Some(g));
    }
    let mut group_of = vec![usize::MAX; source.mesh_groups.len()];
    let mut groups = Vec::new();
    for (g, group) in source.mesh_groups.iter().enumerate() {
        if !keep[g] {
            continue;
        }
        group_of[g] = groups.len();
        groups.push(MeshGroup {
            name: group.name.clone(),
            parent: group.parent.map(|p| group_of[p]),
            meshes: group
                .meshes
                .iter()
                .filter_map(|&m| (mesh_of[m] != usize::MAX).then_some(mesh_of[m]))
                .collect(),
            visible: group.visible,
        });
    }
    let mut model = CanonicalModel {
        bones: source.bones.clone(),
        meshes,
        mesh_groups: groups,
        materials: source.materials.clone(),
        textures: source.textures.clone(),
        extension_headers: source.extension_headers.clone(),
        source_format: source.source_format,
    };
    // Prune unused bones; slots pointing at a dropped bone are unweighted by
    // construction and become index 0, weight 0.
    let keep_bone: Vec<bool> = (0..model.bones.len())
        .map(|bone| bone_is_weighted(&model, bone))
        .collect();
    let old_to_new = rebuild_bone_list(&mut model.bones, &keep_bone);
    let no_redirect = vec![None; old_to_new.len()];
    for mesh in &mut model.meshes {
        remap_bone_group(mesh, &old_to_new, &no_redirect);
    }
    // Materials and textures prune the same way: a part keeps only what its
    // meshes reference, indices remapped to the smaller lists.
    let mut material_of = vec![usize::MAX; model.materials.len()];
    let mut materials = Vec::new();
    for (old, material) in model.materials.iter().enumerate() {
        if model.meshes.iter().any(|mesh| mesh.material == old) {
            material_of[old] = materials.len();
            materials.push(material.clone());
        }
    }
    for mesh in &mut model.meshes {
        mesh.material = material_of[mesh.material];
    }
    let mut used = vec![false; model.textures.len()];
    for material in &materials {
        for &(_, index) in &material.textures {
            used[index] = true;
        }
        if let Some(fox) = &material.fox {
            for &(_, index) in &fox.textures {
                used[index] = true;
            }
        }
        if let Some(prefox) = &material.prefox {
            for &(_, index) in &prefox.textures {
                used[index] = true;
            }
        }
    }
    let mut texture_of = vec![usize::MAX; model.textures.len()];
    let mut textures = Vec::new();
    for (old, texture) in model.textures.iter().enumerate() {
        if used[old] {
            texture_of[old] = textures.len();
            textures.push(texture.clone());
        }
    }
    for material in &mut materials {
        for (_, index) in &mut material.textures {
            *index = texture_of[*index];
        }
        if let Some(fox) = &mut material.fox {
            for (_, index) in &mut fox.textures {
                *index = texture_of[*index];
            }
        }
        if let Some(prefox) = &mut material.prefox {
            for (_, index) in &mut prefox.textures {
                *index = texture_of[*index];
            }
        }
    }
    model.materials = materials;
    model.textures = textures;
    validate(&model).expect("hand split keeps a consistent IR");
    model
}

/// Select, grow once, then separate, per hand in turn
/// (`spec/model_conversion/hand_split.md` "Split"): the left hand first, then the right hand on what the left separation left, so a
/// face fully selected by both goes to `glove_l` only and is never duplicated.
pub fn split_by_skeleton_group(ir: &CanonicalModel) -> HandSplit {
    if !has_hand_weights(ir) {
        return HandSplit {
            body: ir.clone(),
            glove_l: None,
            glove_r: None,
        };
    }
    // Selections run over topological classes: two entries at a UV seam are one vertex,
    // so a class is selected or not, never half.
    let class: Vec<Vec<usize>> = ir.meshes.iter().map(classes).collect();
    let sel_l: Vec<Vec<bool>> = ir
        .meshes
        .iter()
        .enumerate()
        .map(|(m, mesh)| selected(mesh, &class[m], &ir.bones, Hand::Left, &mesh.faces))
        .collect();
    // What the left separation leaves.
    let remaining: Vec<Vec<[u16; 3]>> = ir
        .meshes
        .iter()
        .enumerate()
        .map(|(m, mesh)| {
            mesh.faces
                .iter()
                .copied()
                .filter(|f| !f.iter().all(|&i| sel_l[m][class[m][usize::from(i)]]))
                .collect()
        })
        .collect();
    let sel_r: Vec<Vec<bool>> = ir
        .meshes
        .iter()
        .enumerate()
        .map(|(m, mesh)| selected(mesh, &class[m], &ir.bones, Hand::Right, &remaining[m]))
        .collect();
    // A face goes to a glove when all three corners' classes are in its
    // selection; every other remaining face stays with the body.
    let part = |mesh: &Mesh, faces: Vec<[u16; 3]>| -> Option<Mesh> {
        (!faces.is_empty()).then(|| part_mesh(mesh, &faces))
    };
    let mut glove_l = Vec::with_capacity(ir.meshes.len());
    let mut glove_r = Vec::with_capacity(ir.meshes.len());
    let mut body_parts = Vec::with_capacity(ir.meshes.len());
    for (m, mesh) in ir.meshes.iter().enumerate() {
        glove_l.push(part(
            mesh,
            mesh.faces
                .iter()
                .copied()
                .filter(|f| f.iter().all(|&i| sel_l[m][class[m][usize::from(i)]]))
                .collect(),
        ));
        glove_r.push(part(
            mesh,
            remaining[m]
                .iter()
                .copied()
                .filter(|f| f.iter().all(|&i| sel_r[m][class[m][usize::from(i)]]))
                .collect(),
        ));
        body_parts.push(part(
            mesh,
            remaining[m]
                .iter()
                .copied()
                .filter(|f| !f.iter().all(|&i| sel_r[m][class[m][usize::from(i)]]))
                .collect(),
        ));
    }
    HandSplit {
        body: assemble(ir, body_parts),
        glove_l: glove_l
            .iter()
            .any(Option::is_some)
            .then(|| assemble(ir, glove_l)),
        glove_r: glove_r
            .iter()
            .any(Option::is_some)
            .then(|| assemble(ir, glove_r)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::affine::Affine;
    use crate::ir::{Material, SourceFormat};
    use crate::materials::MaterialFamily;

    /// A bone with no parent and the identity bind.
    fn bone(name: &str) -> Bone {
        Bone {
            name: name.to_string(),
            parent: None,
            matrix: Affine::IDENTITY,
            global_position: None,
            local_position: None,
            bounding_box: None,
        }
    }

    /// A model around `mesh` with one material and one group per mesh.
    fn model(bones: Vec<Bone>, mesh: Mesh) -> CanonicalModel {
        CanonicalModel {
            bones,
            meshes: vec![mesh],
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
        }
    }

    /// The mesh of `hand_split_wrist.py`: a 5x3 grid (x 0..4, y 0..2) split along the
    /// (x,y)-(x+1,y+1) diagonals, plus the fan vertex V = (1.5, 3, 0). Weights by column
    /// over the bone group `[sk_forearm_l, sk_hand_l, skh_index_l]`.
    fn wrist_mesh() -> Mesh {
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        let mut weights = Vec::new();
        let at = |x: usize, y: usize| (x * 3 + y) as u16;
        for x in 0..5usize {
            for y in 0..3usize {
                positions.push([x as f32, y as f32, 0.0]);
                let (row, ws) = match x {
                    0 | 1 => ([0, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
                    2 => ([1, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
                    3 => ([1, 2, 0, 0], [0.5, 0.5, 0.0, 0.0]),
                    _ => ([2, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
                };
                indices.push(row);
                weights.push(ws);
            }
        }
        let v = positions.len() as u16;
        positions.push([1.5, 3.0, 0.0]);
        indices.push([1, 0, 0, 0]);
        weights.push([1.0, 0.0, 0.0, 0.0]);
        let mut faces = Vec::new();
        for x in 0..4usize {
            for y in 0..2usize {
                let (a, b, c, d) = (at(x, y), at(x + 1, y), at(x + 1, y + 1), at(x, y + 1));
                faces.push([a, b, c]);
                faces.push([a, c, d]);
            }
        }
        faces.push([at(1, 2), at(2, 2), v]);
        faces.push([at(2, 2), at(3, 2), v]);
        Mesh {
            vertices: Vertices {
                positions,
                bone_indices: Some(indices),
                bone_weights: Some(weights),
                bone_weight_width: Some(4),
                ..Vertices::default()
            },
            faces,
            bone_group: vec![0, 1, 2],
            material: 0,
            extension_headers: Default::default(),
            custom_bounding_box: None,
        }
    }

    /// Every face as its three positions rotated smallest-first, the whole list sorted —
    /// the normalization `hand_split_wrist.py` uses.
    fn face_set(part: &CanonicalModel) -> Vec<[[f32; 3]; 3]> {
        let mut out: Vec<[[f32; 3]; 3]> = part
            .meshes
            .iter()
            .flat_map(|mesh| {
                mesh.faces.iter().map(|face| {
                    let pts = face.map(|i| mesh.vertices.positions[usize::from(i)]);
                    let start = (0..3)
                        .min_by(|&a, &b| pts[a].partial_cmp(&pts[b]).expect("no NaN"))
                        .expect("three corners");
                    [pts[start], pts[(start + 1) % 3], pts[(start + 2) % 3]]
                })
            })
            .collect();
        out.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
        out
    }

    /// The (index, weight) row of the vertex at `position`.
    fn skinning(part: &CanonicalModel, position: [f32; 3]) -> ([u8; 4], [f32; 4]) {
        for mesh in &part.meshes {
            if let Some(v) = mesh.vertices.positions.iter().position(|p| *p == position) {
                return (
                    mesh.vertices.bone_indices.as_ref().expect("indices")[v],
                    mesh.vertices.bone_weights.as_ref().expect("weights")[v],
                );
            }
        }
        panic!("no vertex at {position:?}");
    }

    #[test]
    fn blender_reference_wrist() {
        let ir = model(
            vec![bone("sk_forearm_l"), bone("sk_hand_l"), bone("skh_index_l")],
            wrist_mesh(),
        );
        let split = split_by_skeleton_group(&ir);
        assert_eq!(split.glove_r, None);
        let glove = split.glove_l.expect("left glove");
        assert_eq!(split.body.meshes[0].vertices.len(), 10);
        assert_eq!(split.body.meshes[0].faces.len(), 9);
        assert_eq!(glove.meshes[0].vertices.len(), 10);
        assert_eq!(glove.meshes[0].faces.len(), 9);
        // The literal face sets of hand_split_wrist.json (the file is the source).
        assert_eq!(
            face_set(&split.body),
            vec![
                [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
                [[0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
                [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 2.0, 0.0]],
                [[0.0, 1.0, 0.0], [1.0, 2.0, 0.0], [0.0, 2.0, 0.0]],
                [[1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 1.0, 0.0]],
                [[1.0, 0.0, 0.0], [2.0, 1.0, 0.0], [1.0, 1.0, 0.0]],
                [[1.0, 1.0, 0.0], [2.0, 1.0, 0.0], [2.0, 2.0, 0.0]],
                [[1.0, 1.0, 0.0], [2.0, 2.0, 0.0], [1.0, 2.0, 0.0]],
                [[1.0, 2.0, 0.0], [2.0, 2.0, 0.0], [1.5, 3.0, 0.0]],
            ]
        );
        assert_eq!(
            face_set(&glove),
            vec![
                [[1.5, 3.0, 0.0], [2.0, 2.0, 0.0], [3.0, 2.0, 0.0]],
                [[2.0, 0.0, 0.0], [3.0, 0.0, 0.0], [3.0, 1.0, 0.0]],
                [[2.0, 0.0, 0.0], [3.0, 1.0, 0.0], [2.0, 1.0, 0.0]],
                [[2.0, 1.0, 0.0], [3.0, 1.0, 0.0], [3.0, 2.0, 0.0]],
                [[2.0, 1.0, 0.0], [3.0, 2.0, 0.0], [2.0, 2.0, 0.0]],
                [[3.0, 0.0, 0.0], [4.0, 0.0, 0.0], [4.0, 1.0, 0.0]],
                [[3.0, 0.0, 0.0], [4.0, 1.0, 0.0], [3.0, 1.0, 0.0]],
                [[3.0, 1.0, 0.0], [4.0, 1.0, 0.0], [4.0, 2.0, 0.0]],
                [[3.0, 1.0, 0.0], [4.0, 2.0, 0.0], [3.0, 2.0, 0.0]],
            ]
        );
        // The duplicated wrist column keeps its full sk_hand_l weight in both parts.
        for (part, want) in [(&split.body, 1u8), (&glove, 0u8)] {
            let (indices, ws) = skinning(part, [2.0, 0.0, 0.0]);
            assert_eq!(ws, [1.0, 0.0, 0.0, 0.0]);
            let entry = mesh_group_bone(part, indices[0]);
            assert_eq!(want, indices[0]);
            assert_eq!(entry, "sk_hand_l");
        }
        assert_eq!(
            split
                .body
                .bones
                .iter()
                .map(|b| b.name.as_str())
                .collect::<Vec<_>>(),
            ["sk_forearm_l", "sk_hand_l"]
        );
        assert_eq!(
            glove
                .bones
                .iter()
                .map(|b| b.name.as_str())
                .collect::<Vec<_>>(),
            ["sk_hand_l", "skh_index_l"]
        );
    }

    /// The bone name a mesh's group entry `slot` indexes.
    fn mesh_group_bone(part: &CanonicalModel, slot: u8) -> &str {
        part.bones[part.meshes[0].bone_group[usize::from(slot)]]
            .name
            .as_str()
    }

    #[test]
    fn detection_follows_weights_not_names() {
        let mut mesh = wrist_mesh();
        // Every vertex weighted to sk_hand_l (group entry 1); the group still names
        // skh_index_l, which must not trigger a split.
        mesh.vertices.bone_indices = Some(vec![[1, 0, 0, 0]; mesh.vertices.len()]);
        mesh.vertices.bone_weights = Some(vec![[1.0, 0.0, 0.0, 0.0]; mesh.vertices.len()]);
        let ir = model(
            vec![bone("sk_forearm_l"), bone("sk_hand_l"), bone("skh_index_l")],
            mesh,
        );
        assert!(!has_hand_weights(&ir));
        let split = split_by_skeleton_group(&ir);
        assert_eq!(split.body, ir);
        assert_eq!(split.glove_l, None);
        assert_eq!(split.glove_r, None);
    }

    /// The wrist mesh weighted fully to sk_hand_l, its column x = 4 carrying 2^-24 of
    /// exporter float noise on the skh_index_l slot.
    fn noisy_wrist_model() -> CanonicalModel {
        let mut mesh = wrist_mesh();
        let len = mesh.vertices.len();
        let mut indices = vec![[1, 0, 0, 0]; len];
        let mut weights = vec![[1.0, 0.0, 0.0, 0.0]; len];
        for (vertex, position) in mesh.vertices.positions.iter().enumerate() {
            if position[0] == 4.0 {
                indices[vertex] = [1, 2, 0, 0];
                weights[vertex] = [1.0, 2.0f32.powi(-24), 0.0, 0.0];
            }
        }
        mesh.vertices.bone_indices = Some(indices);
        mesh.vertices.bone_weights = Some(weights);
        model(
            vec![bone("sk_forearm_l"), bone("sk_hand_l"), bone("skh_index_l")],
            mesh,
        )
    }

    #[test]
    fn float_noise_on_a_hand_slot_splits_nothing() {
        let ir = noisy_wrist_model();
        assert!(!has_hand_weights(&ir));
        let split = split_by_skeleton_group(&ir);
        assert_eq!(split.body, ir);
        assert_eq!(split.glove_l, None);
        assert_eq!(split.glove_r, None);
    }

    #[test]
    fn float_noise_on_a_hand_slot_is_no_hand_weight_in_a_model() {
        let model = crate::formats::pes_model::ir_to_model(&noisy_wrist_model())
            .unwrap()
            .model;
        let noisy = model.meshes.iter().any(|mesh| {
            mesh.vertices
                .bone_weights
                .as_ref()
                .is_some_and(|weights| weights.iter().any(|ws| ws[1] == 2.0f32.powi(-24)))
        });
        assert!(noisy, "the written model keeps the noise weight");
        assert!(!prefox_has_hand_weights(&model));
    }

    #[test]
    fn the_fmdl_check_follows_weights_not_names() {
        // The wrist mesh written as an FMDL: its column x = 4 weighs fully on skh_index_l.
        let ir = model(
            vec![bone("sk_forearm_l"), bone("sk_hand_l"), bone("skh_index_l")],
            wrist_mesh(),
        );
        let mut fmdl = crate::formats::fmdl::ir_to_fmdl(&ir).unwrap().model;
        assert!(fox_has_hand_weights(&fmdl));

        // Every slot naming the hand bone weighted 0: the bone stays in the group.
        let hand = fmdl
            .bones
            .iter()
            .position(|bone| bone.name == "skh_index_l")
            .unwrap();
        let mut listed = false;
        for mesh in &mut fmdl.meshes {
            let Some(entry) = mesh.bone_group.iter().position(|&bone| bone == hand) else {
                continue;
            };
            listed = true;
            let indices = mesh.vertices.bone_indices.as_ref().unwrap();
            let weights = mesh.vertices.bone_weights.as_mut().unwrap();
            for (row, ws) in indices.iter().zip(weights.iter_mut()) {
                for (slot, &index) in row.iter().enumerate() {
                    if usize::from(index) == entry {
                        ws[slot] = 0;
                    }
                }
            }
        }
        assert!(listed, "a bone group still lists skh_index_l");
        assert!(!fox_has_hand_weights(&fmdl));
    }

    #[test]
    fn the_model_check_follows_weights_not_names() {
        // The wrist mesh written as a `.model`: its column x = 4 weighs fully on skh_index_l.
        let ir = model(
            vec![bone("sk_forearm_l"), bone("sk_hand_l"), bone("skh_index_l")],
            wrist_mesh(),
        );
        let weighted = crate::formats::pes_model::ir_to_model(&ir).unwrap().model;
        assert!(prefox_has_hand_weights(&weighted));

        // Every slot naming the hand bone weighted 0: the bone stays in the group.
        let mut unweighted = weighted.clone();
        let hand = unweighted
            .bones
            .iter()
            .position(|bone| bone.name == "skh_index_l")
            .unwrap();
        let mut listed = false;
        for mesh in &mut unweighted.meshes {
            let Some(entry) = mesh.bone_group.iter().position(|&bone| bone == hand) else {
                continue;
            };
            listed = true;
            let indices = mesh.vertices.bone_indices.as_ref().unwrap();
            let weights = mesh.vertices.bone_weights.as_mut().unwrap();
            for (row, ws) in indices.iter().zip(weights.iter_mut()) {
                for (slot, &index) in row.iter().enumerate() {
                    if usize::from(index) == entry {
                        ws[slot] = 0.0;
                    }
                }
            }
        }
        assert!(listed, "a bone group still lists skh_index_l");
        assert!(!prefox_has_hand_weights(&unweighted));

        // No bone weights at all: nothing to read a hand from.
        let mut rigid = weighted;
        for mesh in &mut rigid.meshes {
            mesh.vertices.bone_indices = None;
            mesh.vertices.bone_weights = None;
        }
        assert!(!prefox_has_hand_weights(&rigid));
    }

    #[test]
    fn two_hands_split_independently() {
        // Three disjoint triangles: skh_index_l, skh_index_r, sk_forearm_l.
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        let mut weights = Vec::new();
        let mut faces = Vec::new();
        for (tri, entry) in [1u8, 2, 0].iter().enumerate() {
            let base = positions.len() as u16;
            for corner in [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]] {
                positions.push([tri as f32 * 3.0 + corner[0], corner[1], 0.0]);
                indices.push([*entry, 0, 0, 0]);
                weights.push([1.0, 0.0, 0.0, 0.0]);
            }
            faces.push([base, base + 1, base + 2]);
        }
        let mesh = Mesh {
            vertices: Vertices {
                positions,
                bone_indices: Some(indices),
                bone_weights: Some(weights),
                bone_weight_width: Some(4),
                ..Vertices::default()
            },
            faces,
            bone_group: vec![0, 1, 2],
            material: 0,
            extension_headers: Default::default(),
            custom_bounding_box: None,
        };
        let ir = model(
            vec![
                bone("sk_forearm_l"),
                bone("skh_index_l"),
                bone("skh_index_r"),
            ],
            mesh,
        );
        let split = split_by_skeleton_group(&ir);
        for (part, bones) in [
            (&split.body, vec!["sk_forearm_l"]),
            (&split.glove_l.clone().expect("l"), vec!["skh_index_l"]),
            (&split.glove_r.clone().expect("r"), vec!["skh_index_r"]),
        ] {
            assert_eq!(part.meshes.len(), 1);
            assert_eq!(part.meshes[0].faces.len(), 1);
            assert_eq!(part.meshes[0].vertices.len(), 3);
            assert_eq!(
                part.bones
                    .iter()
                    .map(|b| b.name.as_str())
                    .collect::<Vec<_>>(),
                bones
            );
        }
    }

    #[test]
    fn a_face_selected_by_both_hands_goes_left_only() {
        // One triangle weighted half left, half right per vertex: every
        // class seeds both selections.
        let mesh = Mesh {
            vertices: Vertices {
                positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                bone_indices: Some(vec![[0, 1, 0, 0]; 3]),
                bone_weights: Some(vec![[0.5, 0.5, 0.0, 0.0]; 3]),
                bone_weight_width: Some(4),
                ..Vertices::default()
            },
            faces: vec![[0, 1, 2]],
            bone_group: vec![0, 1],
            material: 0,
            extension_headers: Default::default(),
            custom_bounding_box: None,
        };
        let ir = model(vec![bone("skh_index_l"), bone("skh_index_r")], mesh);
        let split = split_by_skeleton_group(&ir);
        let glove = split.glove_l.expect("left glove");
        assert_eq!(glove.meshes[0].faces.len(), 1);
        // The right hand gets no part of it and the body keeps no copy.
        assert_eq!(split.glove_r, None);
        assert_eq!(split.body.meshes.len(), 0);
    }

    #[test]
    fn a_glove_keeps_only_its_own_materials_and_textures() {
        // A body-only mesh on material 0 and a hand mesh on material 1 with
        // its own texture; each part keeps only what its meshes use.
        let mut body_mesh = wrist_mesh();
        body_mesh.vertices.bone_indices = Some(vec![[0, 0, 0, 0]; body_mesh.vertices.len()]);
        body_mesh.vertices.bone_weights =
            Some(vec![[1.0, 0.0, 0.0, 0.0]; body_mesh.vertices.len()]);
        body_mesh.bone_group = vec![0];
        body_mesh.material = 0;
        let mut hand_mesh = wrist_mesh();
        hand_mesh.material = 1;
        let mut ir = model(
            vec![bone("sk_forearm_l"), bone("sk_hand_l"), bone("skh_index_l")],
            body_mesh,
        );
        ir.meshes.push(hand_mesh);
        ir.mesh_groups[0].meshes = vec![0, 1];
        ir.materials.push(Material {
            name: "glove".to_string(),
            family: MaterialFamily::Shaded,
            two_sided: None,
            transparent: None,
            textures: vec![(crate::materials::TextureRole::Base, 1)],
            parameters: vec![],
            fox: None,
            prefox: None,
        });
        // A material and a texture nothing references drop from every result.
        ir.materials.push(Material {
            name: "spare".to_string(),
            family: MaterialFamily::Shaded,
            two_sided: None,
            transparent: None,
            textures: vec![(crate::materials::TextureRole::Base, 2)],
            parameters: vec![],
            fox: None,
            prefox: None,
        });
        ir.textures = vec![
            crate::ir::Texture {
                directory: "./".to_string(),
                file_name: "body.dds".to_string(),
            },
            crate::ir::Texture {
                directory: "./".to_string(),
                file_name: "glove.dds".to_string(),
            },
            crate::ir::Texture {
                directory: "./".to_string(),
                file_name: "spare.dds".to_string(),
            },
        ];
        let split = split_by_skeleton_group(&ir);
        let glove = split.glove_l.expect("left glove");
        assert_eq!(glove.materials.len(), 1);
        assert_eq!(glove.materials[0].name, "glove");
        assert_eq!(
            glove.materials[0].textures,
            vec![(crate::materials::TextureRole::Base, 0)]
        );
        assert_eq!(glove.textures.len(), 1);
        assert_eq!(glove.textures[0].file_name, "glove.dds");
        // The hand mesh's boundary part stays in the body on material 1, so
        // the body keeps both used materials — but "mat" carries no texture,
        // so body.dds drops with the spare.
        assert_eq!(
            split
                .body
                .materials
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>(),
            ["mat", "glove"]
        );
        assert_eq!(
            split
                .body
                .textures
                .iter()
                .map(|t| t.file_name.as_str())
                .collect::<Vec<_>>(),
            ["glove.dds"]
        );
    }

    #[test]
    fn groups_remap_and_empty_ones_drop() {
        let mut ir = model(
            vec![bone("sk_forearm_l"), bone("sk_hand_l"), bone("skh_index_l")],
            wrist_mesh(),
        );
        // A second, body-only mesh so `head` survives on the body side.
        let mut head = wrist_mesh();
        head.vertices.bone_indices = Some(vec![[0, 0, 0, 0]; head.vertices.len()]);
        head.vertices.bone_weights = Some(vec![[1.0, 0.0, 0.0, 0.0]; head.vertices.len()]);
        head.bone_group = vec![0];
        ir.meshes.push(head);
        ir.mesh_groups = vec![
            MeshGroup {
                name: "root".to_string(),
                parent: None,
                meshes: vec![],
                visible: true,
            },
            MeshGroup {
                name: "arm".to_string(),
                parent: Some(0),
                meshes: vec![0],
                visible: true,
            },
            MeshGroup {
                name: "head".to_string(),
                parent: Some(0),
                meshes: vec![1],
                visible: true,
            },
        ];
        let split = split_by_skeleton_group(&ir);
        let glove = split.glove_l.expect("left glove");
        assert_eq!(
            glove
                .mesh_groups
                .iter()
                .map(|g| (g.name.as_str(), g.parent, g.meshes.clone()))
                .collect::<Vec<_>>(),
            [("root", None, vec![]), ("arm", Some(0), vec![0])]
        );
        assert_eq!(
            split
                .body
                .mesh_groups
                .iter()
                .map(|g| (g.name.as_str(), g.parent, g.meshes.clone()))
                .collect::<Vec<_>>(),
            [
                ("root", None, vec![]),
                ("arm", Some(0), vec![0]),
                ("head", Some(0), vec![1]),
            ]
        );
    }

    #[test]
    fn a_parent_with_no_surviving_descendant_drops() {
        // The child's only mesh went whole to the glove; the root that held
        // nothing but the child drops on the body side too.
        let mesh = Mesh {
            vertices: Vertices {
                positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                bone_indices: Some(vec![[0, 1, 0, 0]; 3]),
                bone_weights: Some(vec![[0.5, 0.5, 0.0, 0.0]; 3]),
                bone_weight_width: Some(4),
                ..Vertices::default()
            },
            faces: vec![[0, 1, 2]],
            bone_group: vec![0, 1],
            material: 0,
            extension_headers: Default::default(),
            custom_bounding_box: None,
        };
        let mut ir = model(vec![bone("skh_index_l"), bone("skh_index_r")], mesh);
        ir.mesh_groups = vec![
            MeshGroup {
                name: "root".to_string(),
                parent: None,
                meshes: vec![],
                visible: true,
            },
            MeshGroup {
                name: "child".to_string(),
                parent: Some(0),
                meshes: vec![0],
                visible: true,
            },
        ];
        let split = split_by_skeleton_group(&ir);
        assert!(split.body.mesh_groups.is_empty());
        // The glove side keeps the chain: child holds the mesh, root its parent.
        let glove = split.glove_l.expect("left glove");
        assert_eq!(
            glove
                .mesh_groups
                .iter()
                .map(|g| (g.name.as_str(), g.parent))
                .collect::<Vec<_>>(),
            [("root", None), ("child", Some(0))]
        );
    }

    #[test]
    fn pruned_bones_reparent_to_the_nearest_survivor() {
        let mut bones = vec![
            bone("sk_upperarm_l"),
            bone("sk_forearm_l"),
            bone("sk_hand_l"),
            bone("skh_index_l"),
        ];
        bones[1].parent = Some(0);
        bones[2].parent = Some(1);
        bones[3].parent = Some(2);
        // One glove triangle weighted sk_hand_l + skh_index_l, one disjoint body
        // triangle weighted sk_forearm_l.
        let mesh = Mesh {
            vertices: Vertices {
                positions: vec![
                    [0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [5.0, 0.0, 0.0],
                    [6.0, 0.0, 0.0],
                    [5.0, 1.0, 0.0],
                ],
                bone_indices: Some(vec![
                    [2, 3, 0, 0],
                    [2, 3, 0, 0],
                    [2, 3, 0, 0],
                    [1, 0, 0, 0],
                    [1, 0, 0, 0],
                    [1, 0, 0, 0],
                ]),
                bone_weights: Some(vec![
                    [0.5, 0.5, 0.0, 0.0],
                    [0.5, 0.5, 0.0, 0.0],
                    [0.5, 0.5, 0.0, 0.0],
                    [1.0, 0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0, 0.0],
                ]),
                bone_weight_width: Some(4),
                ..Vertices::default()
            },
            faces: vec![[0, 1, 2], [3, 4, 5]],
            bone_group: vec![0, 1, 2, 3],
            material: 0,
            extension_headers: Default::default(),
            custom_bounding_box: None,
        };
        let split = split_by_skeleton_group(&model(bones, mesh));
        let glove = split.glove_l.expect("left glove");
        assert_eq!(
            glove
                .bones
                .iter()
                .map(|b| (b.name.as_str(), b.parent))
                .collect::<Vec<_>>(),
            [("sk_hand_l", None), ("skh_index_l", Some(0))]
        );
        assert_eq!(
            split
                .body
                .bones
                .iter()
                .map(|b| (b.name.as_str(), b.parent))
                .collect::<Vec<_>>(),
            [("sk_forearm_l", None)]
        );
    }

    /// `wrist_mesh` with column 2 duplicated as a UV seam: the copies (indices 16-18)
    /// carry UV `[1.0, 0.0]`; the x=1..2 quads reference the originals, the x=2..3
    /// quads and the fan reference the copies. The last face joins the *originals*
    /// of (2,0)/(2,1) — which no seed-grown face touches — to the fan vertex: entry
    /// identity would leave it unselected, class identity sends it to the glove.
    fn wrist_mesh_uv_seam() -> Mesh {
        let mut mesh = wrist_mesh();
        let at = |x: usize, y: usize| (x * 3 + y) as u16;
        let v = 15u16;
        for orig in [at(2, 0), at(2, 1), at(2, 2)] {
            let i = usize::from(orig);
            let position = mesh.vertices.positions[i];
            let indices = mesh.vertices.bone_indices.as_ref().expect("indices")[i];
            let weights = mesh.vertices.bone_weights.as_ref().expect("weights")[i];
            mesh.vertices.positions.push(position);
            mesh.vertices
                .bone_indices
                .as_mut()
                .expect("indices")
                .push(indices);
            mesh.vertices
                .bone_weights
                .as_mut()
                .expect("weights")
                .push(weights);
        }
        mesh.vertices.uvs = vec![[vec![[0.0, 0.0]; 16], vec![[1.0, 0.0]; 3]].concat()];
        mesh.vertices.uv_high_precision = vec![false];
        let copy = |y: usize| 16 + y as u16;
        // The x=2..3 quads (faces 8..12) and both fan faces (16, 17) use the copies.
        for y in 0..2usize {
            mesh.faces[8 + y * 2] = [copy(y), at(3, y), at(3, y + 1)];
            mesh.faces[8 + y * 2 + 1] = [copy(y), at(3, y + 1), copy(y + 1)];
        }
        mesh.faces[16] = [at(1, 2), copy(2), v];
        mesh.faces[17] = [copy(2), at(3, 2), v];
        mesh.faces.push([at(2, 0), at(2, 1), v]);
        mesh
    }

    #[test]
    fn a_uv_seam_is_one_topological_vertex() {
        let bones = || vec![bone("sk_forearm_l"), bone("sk_hand_l"), bone("skh_index_l")];
        let seamed = split_by_skeleton_group(&model(bones(), wrist_mesh_uv_seam()));
        let plain = split_by_skeleton_group(&model(bones(), wrist_mesh()));
        // The seam changes no topology: the body keeps the Blender reference's face
        // set (asserted literally in `blender_reference_wrist`), and the glove gets
        // that set plus the discriminating face on the column-2 originals.
        assert_eq!(face_set(&seamed.body), face_set(&plain.body));
        let glove_part = seamed.glove_l.expect("left glove");
        let glove_faces = face_set(&glove_part);
        assert_eq!(glove_faces.len(), 10);
        assert!(
            glove_faces.contains(&[[1.5, 3.0, 0.0], [2.0, 0.0, 0.0], [2.0, 1.0, 0.0]]),
            "the face on the column-2 originals landed in the glove"
        );
        for face in face_set(&plain.glove_l.expect("left glove")) {
            assert!(glove_faces.contains(&face), "reference face {face:?}");
        }
        let body = &seamed.body.meshes[0];
        let glove = &glove_part.meshes[0];
        // Body: columns 0-1 (6), column 2's originals (3), the fan's copy (1) and the
        // fan vertex — 11 vertices, 9 faces. Glove: column 2's copies (3), the
        // originals (2,0)/(2,1) on the extra face (2), columns 3-4 (6), the fan
        // vertex — 12 vertices, 10 faces.
        assert_eq!(body.vertices.len(), 11);
        assert_eq!(body.faces.len(), 9);
        assert_eq!(glove.vertices.len(), 12);
        assert_eq!(glove.faces.len(), 10);
        // Each copy lands in the part its faces use: column 2 is body-side at UV
        // [0, 0] on the originals plus [1, 0] for the fan's copy, and glove-side at
        // [1, 0] on the three copies plus [0, 0] on the originals the extra
        // face references.
        let column2_uv = |mesh: &Mesh| -> Vec<[f32; 2]> {
            mesh.vertices
                .positions
                .iter()
                .zip(&mesh.vertices.uvs[0])
                .filter(|(position, _)| position[0] == 2.0)
                .map(|(_, uv)| *uv)
                .collect()
        };
        assert_eq!(
            column2_uv(body),
            vec![[0.0, 0.0], [0.0, 0.0], [0.0, 0.0], [1.0, 0.0]]
        );
        assert_eq!(
            column2_uv(glove),
            vec![[1.0, 0.0], [1.0, 0.0], [1.0, 0.0], [0.0, 0.0], [0.0, 0.0]]
        );
    }

    #[test]
    fn a_stale_index_in_an_unweighted_slot_is_safe() {
        // Slot 1's index 5 is past the two-entry group but unweighted; the split's
        // bone-group remap must not look it up.
        let mesh = Mesh {
            vertices: Vertices {
                positions: vec![
                    [0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [5.0, 0.0, 0.0],
                    [6.0, 0.0, 0.0],
                    [5.0, 1.0, 0.0],
                ],
                bone_indices: Some(vec![
                    [0, 5, 0, 0],
                    [0, 5, 0, 0],
                    [0, 5, 0, 0],
                    [1, 0, 0, 0],
                    [1, 0, 0, 0],
                    [1, 0, 0, 0],
                ]),
                bone_weights: Some(vec![
                    [1.0, 0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0, 0.0],
                ]),
                bone_weight_width: Some(4),
                ..Vertices::default()
            },
            faces: vec![[0, 1, 2], [3, 4, 5]],
            bone_group: vec![0, 1],
            material: 0,
            extension_headers: Default::default(),
            custom_bounding_box: None,
        };
        let split = split_by_skeleton_group(&model(
            vec![bone("sk_forearm_l"), bone("skh_index_l")],
            mesh,
        ));
        let body = &split.body.meshes[0];
        assert_eq!(body.faces.len(), 1);
        assert_eq!(
            body.vertices.bone_indices.as_ref().expect("indices"),
            &vec![[0, 0, 0, 0]; 3]
        );
        assert!(split.glove_l.is_some());
    }

    #[test]
    fn hand_of_names() {
        assert_eq!(hand_of("skh_thumb_mata_l"), Some(Hand::Left));
        assert_eq!(hand_of("skh_index_dip_r"), Some(Hand::Right));
        assert_eq!(hand_of("skf_jaw"), None);
        assert_eq!(hand_of("sk_hand_l"), None);
        assert_eq!(hand_of("skh_x"), None);
    }

    #[test]
    fn the_65536th_vertex_keeps_its_own_index() {
        // Faces referencing every u16 index once, then the last one again: part
        // index 65,535 is a real index, not "unmapped" — the repeat must not
        // re-emit the vertex and wrap the face to 0.
        let mut faces: Vec<[u16; 3]> = (0..21845u32)
            .map(|face| {
                [
                    (3 * face) as u16,
                    (3 * face + 1) as u16,
                    (3 * face + 2) as u16,
                ]
            })
            .collect();
        faces.push([u16::MAX, u16::MAX, u16::MAX]);
        faces.push([u16::MAX, 0, 0]);
        let mesh = Mesh {
            vertices: Vertices {
                positions: vec![[0.0; 3]; 65_537],
                ..Vertices::default()
            },
            faces,
            bone_group: vec![],
            material: 0,
            extension_headers: Default::default(),
            custom_bounding_box: None,
        };
        let part = part_mesh(&mesh, &mesh.faces.clone());
        assert_eq!(part.vertices.len(), 65_536);
        assert_eq!(part.faces.last(), Some(&[u16::MAX, 0, 0]));
    }
}
