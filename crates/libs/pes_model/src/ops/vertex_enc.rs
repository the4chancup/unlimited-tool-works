//! Vertex-loop preservation: a `.model` stores vertex/face geometry only,
//! so a Blender-style vertex with several loops becomes several `.model`
//! vertices sharing a *topological key* (position plus each
//! positive-weight bone lane's stored index and weight — a zero-weight
//! lane contributes nothing). The vertex/loop relation is
//! encoded in the vertex order: consecutive vertices with equal topological
//! keys and strictly increasing *nontopological encoding* (normal, color, uv
//! maps in order, tangent, bitangent — the stored bytes concatenated) are
//! loops of one vertex. The `vertex-loop-preservation` per-mesh extension
//! header marks meshes written under this convention.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::format::{MeshVertices, ModelError};
use crate::model::{Mesh, Model};
use crate::ops::split::weight_of;

/// The extension header marking a mesh whose vertex order carries the
/// vertex/loop relation.
pub const VERTEX_LOOP_PRESERVATION: &str = "vertex-loop-preservation";

/// For every vertex of `mesh`, the index of the vertex it is a loop of
/// (its own index when it starts a run), read off the ordering convention.
/// The mesh is not changed; meaningful when the mesh carries
/// `VERTEX_LOOP_PRESERVATION` in `extension_headers`, otherwise every
/// vertex owns itself except where the convention happens to hold by
/// chance. An invalid mesh is an error, never a panic.
pub fn decode(mesh: &Mesh) -> Result<Vec<usize>, ModelError> {
    mesh.validate()?;
    let vertices = &mesh.vertices;
    let count = vertices.positions.len();
    let topological: Vec<Vec<u8>> = (0..count)
        .map(|index| topological_key(vertices, index))
        .collect();
    let nontopological: Vec<Vec<u8>> = (0..count)
        .map(|index| nontopological_encoding(vertices, index))
        .collect();
    let mut owner = Vec::with_capacity(count);
    for index in 0..count {
        if index > 0
            && topological[index] == topological[index - 1]
            && nontopological[index - 1] < nontopological[index]
        {
            owner.push(owner[index - 1]);
        } else {
            owner.push(index);
        }
    }
    Ok(owner)
}

/// Reorders `mesh.vertices` and remaps `mesh.faces` and `mesh.lower_lods` so
/// that, per `owner` (same shape as `decode`'s result), the loops of one
/// vertex are contiguous in increasing nontopological order, identical loops
/// collapse to one, and distinct vertices that share a topological key are
/// ordered by decreasing first loop encoding (so they are never read as one
/// vertex). Adds the marker to `mesh.extension_headers` when absent. Returns
/// the owner map of the new order. `owner.len() != vertex count`, an owner
/// index out of range, an owner map joining different topological keys, or
/// an invalid mesh -> `ModelError::VertexMismatch` or the validation error.
pub fn encode(mesh: &mut Mesh, owner: &[usize]) -> Result<Vec<usize>, ModelError> {
    mesh.validate()?;
    let count = mesh.vertices.positions.len();
    if owner.len() != count {
        return Err(ModelError::VertexMismatch(
            "owner map length does not match vertex count",
        ));
    }
    if owner.iter().any(|index| *index >= count) {
        return Err(ModelError::VertexMismatch("owner index out of range"));
    }
    let vertices = &mesh.vertices;
    let topological: Vec<Vec<u8>> = (0..count)
        .map(|index| topological_key(vertices, index))
        .collect();
    let nontopological: Vec<Vec<u8>> = (0..count)
        .map(|index| nontopological_encoding(vertices, index))
        .collect();

    // Owner -> its loops, in original order. Identical loops collapse to
    // the first occurrence; survivors sort by increasing encoding. A member
    // must share its owner's topological key; joining two keys leaves a key
    // with no entry in `key_owners`.
    let mut members: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (index, vertex) in owner.iter().enumerate() {
        if topological[index] != topological[*vertex] {
            return Err(ModelError::VertexMismatch(
                "owner map joins vertices with different topological keys",
            ));
        }
        members.entry(*vertex).or_default().push(index);
    }
    let mut collapsed_to: Vec<usize> = (0..count).collect();
    for loops in members.values_mut() {
        let mut seen: HashMap<&[u8], usize> = HashMap::new();
        let mut kept = Vec::with_capacity(loops.len());
        for index in loops.iter() {
            match seen.get(nontopological[*index].as_slice()) {
                Some(kept_index) => collapsed_to[*index] = *kept_index,
                None => {
                    seen.insert(&nontopological[*index], *index);
                    kept.push(*index);
                }
            }
        }
        kept.sort_by(|a, b| nontopological[*a].cmp(&nontopological[*b]));
        *loops = kept;
    }

    // Topological key -> owners, ordered by decreasing first-loop encoding
    // so that distinct vertices can never read as one vertex's loops.
    let mut key_owners: HashMap<&[u8], Vec<usize>> = HashMap::new();
    for vertex in members.keys() {
        key_owners
            .entry(&topological[*vertex])
            .or_default()
            .push(*vertex);
    }
    for owners in key_owners.values_mut() {
        owners.sort_by(|a, b| nontopological[members[b][0]].cmp(&nontopological[members[a][0]]));
    }

    // Emit in original order: the first time a topological key appears,
    // all its owners' loops come out contiguously.
    let mut order: Vec<usize> = Vec::with_capacity(count);
    let mut new_owner: Vec<usize> = Vec::with_capacity(count);
    let mut seen_keys: HashSet<&[u8]> = HashSet::new();
    for index in 0..count {
        if seen_keys.insert(&topological[index]) {
            for vertex in &key_owners[topological[index].as_slice()] {
                let run_start = order.len();
                for loop_index in &members[vertex] {
                    order.push(*loop_index);
                    new_owner.push(run_start);
                }
            }
        }
    }

    let mut new_index = vec![0usize; count];
    for (index, vertex) in order.iter().enumerate() {
        new_index[*vertex] = index;
    }

    // The remapped faces are fallible; compute them before anything is
    // permuted, so an error leaves the mesh untouched.
    let remap = |faces: &[[u16; 3]]| -> Result<Vec<[u16; 3]>, ModelError> {
        faces
            .iter()
            .map(|face| {
                let mut mapped = [0u16; 3];
                for (slot, &index) in mapped.iter_mut().zip(face) {
                    *slot = u16::try_from(new_index[collapsed_to[usize::from(index)]]).map_err(
                        |_| ModelError::VertexMismatch("a remapped face index exceeds u16"),
                    )?;
                }
                Ok(mapped)
            })
            .collect()
    };
    let new_faces = remap(&mesh.faces)?;
    let new_lower_lods = mesh
        .lower_lods
        .iter()
        .map(|lod| remap(lod))
        .collect::<Result<Vec<_>, _>>()?;

    fn permute<T: Clone>(values: &mut Vec<T>, order: &[usize]) {
        *values = order.iter().map(|index| values[*index].clone()).collect();
    }
    let vertices = &mut mesh.vertices;
    permute(&mut vertices.positions, &order);
    if let Some(normals) = &mut vertices.normals {
        permute(normals, &order);
    }
    if let Some(tangents) = &mut vertices.tangents {
        permute(tangents, &order);
    }
    if let Some(bitangents) = &mut vertices.bitangents {
        permute(bitangents, &order);
    }
    if let Some(colors) = &mut vertices.colors {
        permute(colors, &order);
    }
    for uvs in &mut vertices.uvs {
        permute(uvs, &order);
    }
    if let Some(indices) = &mut vertices.bone_indices {
        permute(indices, &order);
    }
    if let Some(weights) = &mut vertices.bone_weights {
        permute(weights, &order);
    }
    mesh.faces = new_faces;
    mesh.lower_lods = new_lower_lods;
    if !mesh
        .extension_headers
        .iter()
        .any(|header| header == VERTEX_LOOP_PRESERVATION)
    {
        mesh.extension_headers
            .push(VERTEX_LOOP_PRESERVATION.to_owned());
    }
    Ok(new_owner)
}

/// `encode` on every mesh (owners per mesh). An invalid model is an
/// error, never a panic.
pub fn encode_model(
    model: &mut Model,
    owners: &[Vec<usize>],
) -> Result<Vec<Vec<usize>>, ModelError> {
    model.validate()?;
    if owners.len() != model.meshes.len() {
        return Err(ModelError::VertexMismatch(
            "owner maps do not match the mesh count",
        ));
    }
    let mut result = Vec::with_capacity(owners.len());
    for (mesh, owner) in model.meshes.iter_mut().zip(owners) {
        result.push(encode(mesh, owner)?);
    }
    Ok(result)
}

/// `decode` on every mesh that carries the marker; identity maps otherwise.
/// An invalid model is an error, never a panic.
pub fn decode_model(model: &Model) -> Result<Vec<Vec<usize>>, ModelError> {
    model.validate()?;
    model
        .meshes
        .iter()
        .map(|mesh| {
            if mesh
                .extension_headers
                .iter()
                .any(|header| header == VERTEX_LOOP_PRESERVATION)
            {
                decode(mesh)
            } else {
                Ok((0..mesh.vertices.positions.len()).collect())
            }
        })
        .collect()
}

/// Position plus each positive-weight lane's (bone index, weight) as the
/// bytes the file stores, in lane order; a zero-weight lane — whose index
/// a split rewrites — contributes nothing.
pub(crate) fn topological_key(vertices: &MeshVertices, index: usize) -> Vec<u8> {
    let mut key = Vec::new();
    for component in vertices.positions[index] {
        key.extend(component.to_le_bytes());
    }
    if let Some(indices) = &vertices.bone_indices {
        for (lane, &bone) in indices[index].iter().enumerate() {
            let weight = weight_of(vertices, index, lane);
            if weight > 0.0 {
                key.push(bone);
                key.extend(weight.to_le_bytes());
            }
        }
    }
    key
}

/// Normal, color, uv maps in order, tangent, bitangent — the stored bytes,
/// concatenated.
pub(crate) fn nontopological_encoding(vertices: &MeshVertices, index: usize) -> Vec<u8> {
    let mut encoding = Vec::new();
    if let Some(normals) = &vertices.normals {
        for component in normals[index] {
            encoding.extend(component.to_le_bytes());
        }
    }
    if let Some(colors) = &vertices.colors {
        encoding.extend(colors[index]);
    }
    for uvs in &vertices.uvs {
        encoding.extend(uvs[index][0].to_le_bytes());
        encoding.extend(uvs[index][1].to_le_bytes());
    }
    if let Some(tangents) = &vertices.tangents {
        for component in tangents[index] {
            encoding.extend(component.to_le_bytes());
        }
    }
    if let Some(bitangents) = &vertices.bitangents {
        for component in bitangents[index] {
            encoding.extend(component.to_le_bytes());
        }
    }
    encoding
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::fixtures::*;
    use crate::format::{Bone, BoundingBox, LodRecord, PreFoxModel};
    use std::collections::BTreeSet;

    fn load(bytes: &[u8]) -> Model {
        Model::from_file(&PreFoxModel::read(bytes).unwrap()).unwrap()
    }

    /// The partition an owner map describes: owner -> its members.
    fn partition(owner: &[usize]) -> BTreeSet<BTreeSet<usize>> {
        let mut groups: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        for (index, vertex) in owner.iter().enumerate() {
            groups.entry(*vertex).or_default().insert(index);
        }
        groups.values().cloned().collect()
    }

    /// A mesh over `vertices` with one face level.
    fn bare_mesh(vertices: MeshVertices, faces: Vec<[u16; 3]>) -> Mesh {
        Mesh {
            name: None,
            extension_headers: Vec::new(),
            tags: Vec::new(),
            bounds: BoundingBox::of(&vertices.positions),
            vertices,
            faces,
            lower_lods: Vec::new(),
            bone_group: Vec::new(),
            material: 0,
            order: 0,
            editor_data: Vec::new(),
        }
    }

    #[test]
    fn decode_cardhead_owner_map() {
        let model = load(CARDHEAD);
        let mesh = &model.meshes[0];
        assert!(
            mesh.extension_headers
                .contains(&VERTEX_LOOP_PRESERVATION.to_owned())
        );
        let owner = decode(mesh).unwrap();
        assert_eq!(owner.len(), 16);
        // The same map computed straight from the convention.
        let mut expected = Vec::new();
        for index in 0..mesh.vertices.positions.len() {
            if index > 0
                && topological_key(&mesh.vertices, index)
                    == topological_key(&mesh.vertices, index - 1)
                && nontopological_encoding(&mesh.vertices, index - 1)
                    < nontopological_encoding(&mesh.vertices, index)
            {
                expected.push(expected[index - 1]);
            } else {
                expected.push(index);
            }
        }
        assert_eq!(owner, expected);
    }

    /// A synthetic unskinned mesh: A0, B0, A1, B1 where A's vertices share
    /// a position and differ in uv, B's likewise.
    fn loop_mesh(identical_a: bool) -> Mesh {
        let position_a = [1.0f32, 2.0, 3.0];
        let position_b = [4.0f32, 5.0, 6.0];
        let uv = |u: f32| [u, 0.25f32];
        bare_mesh(
            MeshVertices {
                positions: vec![position_a, position_b, position_a, position_b],
                normals: Some(vec![[0.0, 0.0, 1.0]; 4]),
                tangents: None,
                bitangents: None,
                colors: None,
                uvs: vec![vec![
                    uv(0.0),
                    uv(0.0),
                    if identical_a { uv(0.0) } else { uv(1.0) },
                    uv(1.0),
                ]],
                bone_indices: None,
                bone_weights: None,
                bone_weight_width: 4,
            },
            vec![[0, 1, 2], [1, 3, 2]],
        )
    }

    /// The (position, uv) each face corner resolves to, as bit patterns so
    /// the tuples order.
    fn face_tuples(mesh: &Mesh) -> BTreeSet<Vec<([u32; 3], [u32; 2])>> {
        mesh.faces
            .iter()
            .map(|face| {
                face.iter()
                    .map(|index| {
                        (
                            mesh.vertices.positions[usize::from(*index)].map(f32::to_bits),
                            mesh.vertices.uvs[0][usize::from(*index)].map(f32::to_bits),
                        )
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn encode_collapses_identical_loops() {
        let mut mesh = loop_mesh(true);
        let faces_before = face_tuples(&mesh);
        let owner = encode(&mut mesh, &[0, 1, 0, 1]).unwrap();
        assert_eq!(mesh.vertices.positions.len(), 3);
        // A has a single loop left, owning itself.
        assert_eq!(owner, [0, 1, 1]);
        assert_eq!(face_tuples(&mesh), faces_before);
        assert_eq!(decode(&mesh).unwrap(), owner);
        assert!(
            mesh.extension_headers
                .contains(&VERTEX_LOOP_PRESERVATION.to_owned())
        );
    }

    #[test]
    fn encode_groups_loops_of_one_vertex() {
        let mut mesh = loop_mesh(false);
        let faces_before = face_tuples(&mesh);
        let owner = encode(&mut mesh, &[0, 1, 0, 1]).unwrap();
        assert_eq!(mesh.vertices.positions.len(), 4);
        assert_eq!(owner, [0, 0, 2, 2]);
        // A's loops first (topological key met at vertex 0), each pair in
        // increasing uv order.
        assert_eq!(mesh.vertices.positions[0], [1.0, 2.0, 3.0]);
        assert_eq!(mesh.vertices.positions[1], [1.0, 2.0, 3.0]);
        assert_eq!(mesh.vertices.positions[2], [4.0, 5.0, 6.0]);
        assert_eq!(mesh.vertices.positions[3], [4.0, 5.0, 6.0]);
        assert!(mesh.vertices.uvs[0][0][0] < mesh.vertices.uvs[0][1][0]);
        assert!(mesh.vertices.uvs[0][2][0] < mesh.vertices.uvs[0][3][0]);
        assert_eq!(decode(&mesh).unwrap(), owner);
        assert_eq!(face_tuples(&mesh), faces_before);
    }

    #[test]
    fn distinct_vertices_with_equal_keys_stay_distinct() {
        // Two owners sharing a position: the decreasing first-loop order
        // keeps them from reading as one vertex.
        let mut mesh = bare_mesh(
            MeshVertices {
                positions: vec![[1.0, 2.0, 3.0], [1.0, 2.0, 3.0]],
                normals: Some(vec![[0.0, 0.0, 1.0]; 2]),
                tangents: None,
                bitangents: None,
                colors: None,
                uvs: vec![vec![[0.0, 0.0], [1.0, 0.0]]],
                bone_indices: None,
                bone_weights: None,
                bone_weight_width: 4,
            },
            vec![[0, 1, 0]],
        );
        let owner = encode(&mut mesh, &[0, 1]).unwrap();
        // The vertex with the larger loop encoding comes first.
        assert_eq!(mesh.vertices.uvs[0][0], [1.0, 0.0]);
        assert_eq!(mesh.vertices.uvs[0][1], [0.0, 0.0]);
        assert_eq!(owner, [0, 1]);
        // decode still separates them into two vertices.
        let decoded = decode(&mesh).unwrap();
        assert_eq!(decoded, [0, 1]);
        assert_eq!(partition(&decoded).len(), 2);
    }

    #[test]
    fn encode_orders_an_owners_loops_by_encoding() {
        // Two loops of one vertex arrive in decreasing uv order: the kept
        // loops still come out increasing, so a later `decode` reads them
        // as one vertex's run.
        let mut mesh = bare_mesh(
            MeshVertices {
                positions: vec![[1.0, 2.0, 3.0], [1.0, 2.0, 3.0]],
                normals: Some(vec![[0.0, 0.0, 1.0]; 2]),
                tangents: None,
                bitangents: None,
                colors: None,
                uvs: vec![vec![[1.0, 0.0], [0.0, 0.0]]],
                bone_indices: None,
                bone_weights: None,
                bone_weight_width: 4,
            },
            vec![[0, 1, 0]],
        );
        let owner = encode(&mut mesh, &[0, 0]).unwrap();
        assert_eq!(mesh.vertices.uvs[0], vec![[0.0, 0.0], [1.0, 0.0]]);
        assert_eq!(owner, [0, 0]);
        assert_eq!(decode(&mesh).unwrap(), [0, 0]);
    }

    #[test]
    fn owner_errors() {
        let mut mesh = loop_mesh(false);
        assert!(matches!(
            encode(&mut mesh, &[0, 1, 0]),
            Err(ModelError::VertexMismatch(_))
        ));
        assert!(matches!(
            encode(&mut mesh, &[0, 1, 0, 9]),
            Err(ModelError::VertexMismatch(_))
        ));
    }

    #[test]
    fn owner_map_must_share_topological_keys() {
        // Vertex 1 is not a loop of vertex 0: their positions differ.
        let mut mesh = loop_mesh(false);
        assert_eq!(
            encode(&mut mesh, &[0, 0, 0, 0]),
            Err(ModelError::VertexMismatch(
                "owner map joins vertices with different topological keys"
            ))
        );
    }

    // The last vertex shares vertex 0's topological key with the larger
    // loop encoding, so the remap puts it first and index 65535 lands at
    // index 65536.
    #[test]
    fn a_remapped_face_index_past_u16_errors() {
        let count = usize::from(u16::MAX) + 2;
        let mut vertices = MeshVertices {
            positions: (0..count - 1)
                .map(|index| [index as f32, 0.0, 0.0])
                .collect(),
            normals: None,
            tangents: None,
            bitangents: None,
            colors: None,
            uvs: vec![vec![[0.0; 2]; count]],
            bone_indices: None,
            bone_weights: None,
            bone_weight_width: 4,
        };
        vertices.positions.push(vertices.positions[0]);
        vertices.uvs[0][count - 1] = [1.0, 0.0];
        let mut mesh = bare_mesh(vertices, vec![[u16::MAX, 0, 1]]);
        let owners: Vec<usize> = (0..count).collect();
        let before = mesh.clone();
        assert_eq!(
            encode(&mut mesh, &owners),
            Err(ModelError::VertexMismatch(
                "a remapped face index exceeds u16"
            ))
        );
        // A failed remap must not leave the mesh half-permuted.
        assert_eq!(mesh, before);
    }

    // A zero-weight lane's index means nothing to the key — a split
    // rewrites it — so vertices differing only there share an owner.
    #[test]
    fn a_zero_weight_lane_does_not_split_owners() {
        let mesh = bare_mesh(
            MeshVertices {
                positions: vec![[0.0; 3], [0.0; 3]],
                normals: None,
                tangents: None,
                bitangents: None,
                colors: None,
                uvs: vec![vec![[0.0, 0.0], [1.0, 0.0]]],
                bone_indices: Some(vec![[0, 0, 0, 0], [0, 1, 0, 0]]),
                bone_weights: Some(vec![[1.0, 0.0, 0.0, 0.0]; 2]),
                bone_weight_width: 4,
            },
            Vec::new(),
        );
        assert_eq!(decode(&mesh).unwrap(), vec![0, 0]);
    }

    // Two coincident vertices whose positive-weight lanes differ have
    // different keys: the second starts its own owner run even at the
    // same position.
    #[test]
    fn different_weights_keep_their_own_owners() {
        let mesh = bare_mesh(
            MeshVertices {
                positions: vec![[0.0; 3], [0.0; 3]],
                normals: None,
                tangents: None,
                bitangents: None,
                colors: None,
                uvs: vec![vec![[0.0, 0.0], [1.0, 0.0]]],
                bone_indices: Some(vec![[0, 0, 0, 0]; 2]),
                bone_weights: Some(vec![[1.0, 0.0, 0.0, 0.0], [0.5, 0.5, 0.0, 0.0]]),
                bone_weight_width: 4,
            },
            Vec::new(),
        );
        assert_eq!(decode(&mesh).unwrap(), vec![0, 1]);
    }

    // Two coincident vertices keep separate owners when their
    // positive-weight lanes differ in the bone index or in the weight
    // alone: the key stores both halves of each lane.
    #[test]
    fn weighted_lanes_distinguish_coincident_vertices() {
        let mesh = |bone_indices: [[u8; 4]; 2], bone_weights: [[f32; 4]; 2]| {
            bare_mesh(
                MeshVertices {
                    positions: vec![[0.0; 3], [0.0; 3]],
                    normals: None,
                    tangents: None,
                    bitangents: None,
                    colors: None,
                    uvs: vec![vec![[0.0, 0.0], [1.0, 0.0]]],
                    bone_indices: Some(bone_indices.to_vec()),
                    bone_weights: Some(bone_weights.to_vec()),
                    bone_weight_width: 4,
                },
                Vec::new(),
            )
        };
        // Only the weighted lane's bone index differs.
        assert_eq!(
            decode(&mesh(
                [[0, 0, 0, 0], [1, 0, 0, 0]],
                [[1.0, 0.0, 0.0, 0.0]; 2]
            ))
            .unwrap(),
            [0, 1]
        );
        // Same weighted lanes, different weights.
        assert_eq!(
            decode(&mesh(
                [[0, 1, 0, 0]; 2],
                [[0.6, 0.4, 0.0, 0.0], [0.5, 0.5, 0.0, 0.0]]
            ))
            .unwrap(),
            [0, 1]
        );
    }

    // Two loops of one vertex identical but for a single nontopological
    // field still encode as two loops: that field alone separates their
    // encodings.
    #[test]
    fn each_nontopological_field_keeps_its_loops() {
        let vertices = || MeshVertices {
            positions: vec![[0.0; 3], [0.0; 3]],
            normals: Some(vec![[0.0, 0.0, 1.0]; 2]),
            tangents: Some(vec![[1.0, 0.0, 0.0]; 2]),
            bitangents: Some(vec![[0.0, 1.0, 0.0]; 2]),
            colors: Some(vec![[255, 0, 0, 255]; 2]),
            uvs: vec![vec![[0.5, 0.5]; 2]],
            bone_indices: None,
            bone_weights: None,
            bone_weight_width: 4,
        };
        for field in ["normal", "color", "uv v", "tangent", "bitangent"] {
            let mut vertices = vertices();
            match field {
                "normal" => vertices.normals = Some(vec![[0.0, 0.0, 1.0], [0.0, 1.0, 0.0]]),
                "color" => vertices.colors = Some(vec![[255, 0, 0, 255], [0, 255, 0, 255]]),
                "uv v" => vertices.uvs = vec![vec![[0.5, 0.5], [0.5, 1.0]]],
                "tangent" => vertices.tangents = Some(vec![[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]]),
                "bitangent" => vertices.bitangents = Some(vec![[0.0, 1.0, 0.0], [0.0, -1.0, 0.0]]),
                _ => unreachable!("the loop above lists every field"),
            }
            let mut mesh = bare_mesh(vertices, Vec::new());
            encode(&mut mesh, &[0, 0]).unwrap();
            assert_eq!(mesh.vertices.positions.len(), 2, "{field}");
        }
    }

    // A combine rewrites a zero-weight lane whose slot falls outside the
    // component's bone group; a topological key counting it would merge
    // the encoded owners on the way back.
    #[test]
    fn combine_rewriting_zero_lanes_preserves_owners() {
        let mut mesh = bare_mesh(
            MeshVertices {
                positions: vec![[0.0; 3], [0.0; 3]],
                normals: None,
                tangents: None,
                bitangents: None,
                colors: None,
                uvs: vec![vec![[0.0, 0.0], [1.0, 0.0]]],
                bone_indices: Some(vec![[0, 7, 0, 0], [0, 5, 0, 0]]),
                bone_weights: Some(vec![[1.0, 0.0, 0.0, 0.0]; 2]),
                bone_weight_width: 4,
            },
            Vec::new(),
        );
        mesh.bone_group = vec![0];
        mesh.extension_headers.push("Split-Mesh: 1".to_owned());
        let owners = decode(&mesh).unwrap();
        let owners = encode(&mut mesh, &owners).unwrap();
        let mut model = Model {
            flags: 0,
            bones: vec![Bone {
                name: "bone".to_owned(),
                matrix: [
                    1.0, 0.0, 0.0, 0.0, //
                    0.0, 1.0, 0.0, 0.0, //
                    0.0, 0.0, 1.0, 0.0,
                ],
            }],
            materials: vec!["material".to_owned()],
            meshes: vec![mesh],
            extension_headers: Vec::new(),
            bounds: BoundingBox::of(&[]),
            lod: LodRecord::for_levels(0),
        };
        crate::ops::split::decode(&mut model).unwrap();
        assert_eq!(decode(&model.meshes[0]).unwrap(), owners);
    }

    // Two consecutive vertices identical in every byte are not loops of
    // one vertex: only a strictly increasing encoding continues a run.
    #[test]
    fn identical_consecutive_vertices_own_themselves() {
        let mesh = bare_mesh(
            MeshVertices {
                positions: vec![[0.0; 3], [0.0; 3]],
                normals: None,
                tangents: None,
                bitangents: None,
                colors: None,
                uvs: vec![vec![[0.0, 0.0], [0.0, 0.0]]],
                bone_indices: None,
                bone_weights: None,
                bone_weight_width: 4,
            },
            Vec::new(),
        );
        assert_eq!(decode(&mesh).unwrap(), vec![0, 1]);
    }

    #[test]
    fn encode_adds_the_marker_only_once() {
        let mut mesh = loop_mesh(false);
        mesh.extension_headers
            .push(VERTEX_LOOP_PRESERVATION.to_owned());
        encode(&mut mesh, &[0, 1, 0, 1]).unwrap();
        assert_eq!(
            mesh.extension_headers
                .iter()
                .filter(|header| *header == VERTEX_LOOP_PRESERVATION)
                .count(),
            1
        );
    }

    // Without the marker a mesh decodes to the identity, even where its
    // vertex order happens to satisfy the ordering convention — and even
    // with another extension header present.
    #[test]
    fn decode_model_is_identity_without_the_marker() {
        let mut mesh = bare_mesh(
            MeshVertices {
                positions: vec![[0.0; 3], [0.0; 3]],
                normals: None,
                tangents: None,
                bitangents: None,
                colors: None,
                uvs: vec![vec![[0.0, 0.0], [1.0, 0.0]]],
                bone_indices: None,
                bone_weights: None,
                bone_weight_width: 4,
            },
            Vec::new(),
        );
        mesh.extension_headers.push("unrelated".to_owned());
        let model = Model {
            flags: 0,
            bones: Vec::new(),
            materials: vec!["material".to_owned()],
            meshes: vec![mesh],
            extension_headers: Vec::new(),
            bounds: BoundingBox::of(&[]),
            lod: LodRecord::for_levels(0),
        };
        assert_eq!(decode_model(&model).unwrap(), vec![vec![0, 1]]);
    }

    #[test]
    fn model_level_round_trip() {
        for bytes in ALL {
            let model = load(bytes);
            let owners = decode_model(&model).unwrap();
            let mut again = model.clone();
            let new_owners = encode_model(&mut again, &owners).unwrap();
            assert_eq!(decode_model(&again).unwrap(), new_owners);
            for (index, mesh) in again.meshes.iter().enumerate() {
                for index in mesh
                    .faces
                    .iter()
                    .chain(mesh.lower_lods.iter().flatten())
                    .flatten()
                {
                    assert!(usize::from(*index) < mesh.vertices.positions.len());
                }
                assert_eq!(mesh.lower_lods.len(), model.meshes[index].lower_lods.len());
                for (level, before) in mesh.lower_lods.iter().zip(&model.meshes[index].lower_lods) {
                    assert_eq!(level.len(), before.len());
                }
                // A mesh already in convention re-encodes to itself.
                if model.meshes[index]
                    .extension_headers
                    .contains(&VERTEX_LOOP_PRESERVATION.to_owned())
                {
                    assert_eq!(mesh.vertices, model.meshes[index].vertices);
                }
            }
            again.to_file().unwrap();
        }
    }

    #[test]
    fn lower_lods_remap() {
        let model = load(COLLAR);
        let mut mesh = model.meshes[0].clone();
        let owner = decode(&mesh).unwrap();
        let owner = encode(&mut mesh, &owner).unwrap();
        assert_eq!(mesh.lower_lods.len(), 5);
        assert_eq!(decode(&mesh).unwrap(), owner);
        for (level, before) in mesh.lower_lods.iter().zip(&model.meshes[0].lower_lods) {
            assert_eq!(level.len(), before.len());
            for face in level {
                for index in face {
                    assert!(usize::from(*index) < mesh.vertices.positions.len());
                }
            }
        }
    }
}
