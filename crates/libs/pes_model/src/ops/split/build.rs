//! Component construction: equipresent sets, per-bone storable items, the
//! base-bone heuristic, the principal-axis sort, and the fragment builder.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::format::{Bone, BoundingBox, ModelError};
use crate::model::{Mesh, Model};

use super::{
    BONE_LIMIT_SOFT, BoneMapping, FACE_LIMIT_SOFT, PREFERRED_BASE_BONES, StorableItems,
    VERTEX_LIMIT_SOFT, bone_mapping, empty_like, push_vertex,
};
/// Vertex `index`'s split key: the stored bytes of its position followed
/// by its `(model bone, weight)` pairs — the identity `combine` matches
/// on, so a zero-weight lane's raw index never splits coincident
/// vertices.
fn split_key(mesh: &Mesh, index: usize, mapping: &BoneMapping) -> Vec<u8> {
    let mut key = Vec::new();
    for component in mesh.vertices.positions[index] {
        key.extend(component.to_le_bytes());
    }
    for &(bone, weight) in mapping {
        key.extend(bone.to_le_bytes());
        key.extend(weight.to_le_bytes());
    }
    key
}

/// The static data of one mesh being split: faces as vertex indices, the
/// equipresent vertex sets, and each vertex's model-level bone mapping.
struct Pieces {
    face_vertices: Vec<[usize; 3]>,
    sets: Vec<Vec<usize>>,
    set_of: Vec<usize>,
    mappings: Vec<BoneMapping>,
    skinned: bool,
}

/// Whether `items` fits under the soft limits.
fn fits_in_submesh(items: &StorableItems, pieces: &Pieces) -> bool {
    if items.faces.len() > FACE_LIMIT_SOFT {
        return false;
    }
    let mut used_sets: HashSet<usize> = items.loose.iter().copied().collect();
    for &face in &items.faces {
        for &vertex in &pieces.face_vertices[face] {
            used_sets.insert(pieces.set_of[vertex]);
        }
    }
    let vertex_count: usize = used_sets.iter().map(|&set| pieces.sets[set].len()).sum();
    if vertex_count > VERTEX_LIMIT_SOFT {
        return false;
    }
    if pieces.skinned {
        let mut bones = HashSet::new();
        for &set in &used_sets {
            for &(bone, _) in &pieces.mappings[pieces.sets[set][0]] {
                bones.insert(bone);
            }
        }
        if bones.len() > BONE_LIMIT_SOFT {
            return false;
        }
    }
    true
}

/// A bone under which storable items remain: the first preferred name that
/// has any, else the lowest-index bone that has any. `None` when no bone
/// has items left (the whole-mesh root takes over).
fn select_base_bone(
    model: &Model,
    items_per_bone: &HashMap<Option<usize>, StorableItems>,
) -> Option<usize> {
    let has_items = |bone: usize| {
        items_per_bone
            .get(&Some(bone))
            .is_some_and(|items| !items.faces.is_empty() || !items.loose.is_empty())
    };
    for name in PREFERRED_BASE_BONES {
        if let Some(index) = model.bones.iter().position(|bone| bone.name == name)
            && has_items(index)
        {
            return Some(index);
        }
    }
    (0..model.bones.len()).find(|&bone| has_items(bone))
}

/// The bone's world position, recovered from its inverse bind matrix:
/// `-R^T·t` of the rigid 3x4 `matrix` (row-major rows `m[0..4]`, `m[4..8]`,
/// `m[8..12]`).
fn bone_position(bone: &Bone) -> [f32; 3] {
    let m = bone.matrix;
    [
        -(m[0] * m[3] + m[4] * m[7] + m[8] * m[11]),
        -(m[1] * m[3] + m[5] * m[7] + m[9] * m[11]),
        -(m[2] * m[3] + m[6] * m[7] + m[10] * m[11]),
    ]
}

/// The first principal axis of the vertex cloud in `items`, oriented from
/// `bone_position` toward the cloud's centre. Power iteration on the 3x3
/// covariance matrix, eight iterations from each basis vector, keeping the
/// result with the largest Rayleigh quotient.
fn sort_vector(points: &[[f32; 3]], bone_position: [f32; 3]) -> [f32; 3] {
    let count = points.len() as f32;
    let mut mean = [0.0f32; 3];
    for point in points {
        for axis in 0..3 {
            mean[axis] += point[axis] / count;
        }
    }
    let mut covariance = [[0.0f32; 3]; 3];
    for point in points {
        let d = [point[0] - mean[0], point[1] - mean[1], point[2] - mean[2]];
        for a in 0..3 {
            for b in 0..3 {
                covariance[a][b] += d[a] * d[b];
            }
        }
    }
    // Power-iterate from each basis vector in turn — a single seed can sit
    // on a non-principal eigenvector — and keep the result with the
    // largest Rayleigh quotient v . (C v) (strictly larger replaces, so a
    // tie keeps the earlier start; a degenerate cloud keeps x).
    let mut vector = [1.0f32, 0.0, 0.0];
    let mut best = f32::NEG_INFINITY;
    for start in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] {
        let mut candidate = start;
        for _ in 0..8 {
            let next = multiply(&covariance, candidate);
            // A start in the covariance's null space normalizes to NaN;
            // its Rayleigh quotient never beats `best`.
            let norm = (next[0] * next[0] + next[1] * next[1] + next[2] * next[2]).sqrt();
            candidate = [next[0] / norm, next[1] / norm, next[2] / norm];
        }
        let product = multiply(&covariance, candidate);
        let quotient = dot(candidate, product);
        if quotient > best {
            best = quotient;
            vector = candidate;
        }
    }
    let towards = dot(
        [
            mean[0] - bone_position[0],
            mean[1] - bone_position[1],
            mean[2] - bone_position[2],
        ],
        vector,
    );
    if towards < 0.0 {
        [-vector[0], -vector[1], -vector[2]]
    } else {
        vector
    }
}

/// `matrix` times `vector`: one power-iteration step of `sort_vector`.
fn multiply(matrix: &[[f32; 3]; 3], vector: [f32; 3]) -> [f32; 3] {
    let mut next = [0.0f32; 3];
    for a in 0..3 {
        for b in 0..3 {
            next[a] += matrix[a][b] * vector[b];
        }
    }
    next
}

/// The dot product: a vertex's projection on the sort axis, and the
/// Rayleigh quotient in `sort_vector`.
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Splits off one component from `items_per_bone`; returns it together with
/// the face and loose-set indices it consumed.
fn build_component(
    model: &Model,
    mesh: &Mesh,
    parents: &[Option<usize>],
    items_per_bone: &HashMap<Option<usize>, StorableItems>,
    pieces: &Pieces,
) -> Result<(Mesh, StorableItems), ModelError> {
    let face_vertices = &pieces.face_vertices;
    let sets = &pieces.sets;
    let set_of = &pieces.set_of;
    let mappings = &pieces.mappings;
    let skinned = pieces.skinned;

    let base_bone = if skinned {
        select_base_bone(model, items_per_bone)
    } else {
        None
    };
    let mut taken;
    let mut selected_bones = BTreeSet::new();
    let mut selected_sets = HashSet::new();

    if fits_in_submesh(&items_per_bone[&base_bone], pieces) {
        // Climb to the highest ancestor that still fits a single component.
        let mut bone = base_bone;
        taken = items_per_bone[&bone].clone();
        while let Some(current) = bone {
            let parent = parents[current];
            let candidate = &items_per_bone[&parent];
            if !fits_in_submesh(candidate, pieces) {
                break;
            }
            bone = parent;
            taken = candidate.clone();
        }
        for &face in &taken.faces {
            for &vertex in &face_vertices[face] {
                selected_sets.insert(set_of[vertex]);
            }
        }
        for &set in &taken.loose {
            selected_sets.insert(set);
        }
        if skinned {
            for &set in &selected_sets {
                for &(bone_index, _) in &mappings[sets[set][0]] {
                    selected_bones.insert(bone_index);
                }
            }
        }
    } else {
        // A fragment: faces in decreasing order of their best projection on
        // the sort vector, added while the soft limits hold.
        let storable = &items_per_bone[&base_bone];
        // Faces by index, then loose sets by index: the f32 accumulation
        // in `sort_vector` must not depend on HashSet iteration order.
        let mut faces: Vec<usize> = storable.faces.iter().copied().collect();
        faces.sort_unstable();
        let mut loose: Vec<usize> = storable.loose.iter().copied().collect();
        loose.sort_unstable();
        let mut points = Vec::new();
        for &face in &faces {
            for &vertex in &face_vertices[face] {
                points.push(mesh.vertices.positions[vertex]);
            }
        }
        for &set in &loose {
            points.push(mesh.vertices.positions[sets[set][0]]);
        }
        let bone_position = base_bone.map_or([0.0; 3], |bone| bone_position(&model.bones[bone]));
        let axis = sort_vector(&points, bone_position);
        let score = |vertex: usize| -> f32 { dot(mesh.vertices.positions[vertex], axis) };

        faces.sort_by(|a, b| {
            let score_a = face_vertices[*a]
                .iter()
                .map(|&vertex| score(vertex))
                .fold(f32::MIN, f32::max);
            let score_b = face_vertices[*b]
                .iter()
                .map(|&vertex| score(vertex))
                .fold(f32::MIN, f32::max);
            score_b.total_cmp(&score_a).then(a.cmp(b))
        });
        loose.sort_by(|a, b| {
            score(sets[*b][0])
                .total_cmp(&score(sets[*a][0]))
                .then(a.cmp(b))
        });

        let mut taken_faces = HashSet::new();
        let mut taken_loose = HashSet::new();
        let mut vertex_count = 0usize;
        for face in faces {
            if taken_faces.len() >= FACE_LIMIT_SOFT {
                break;
            }
            let mut added_bones = HashSet::new();
            let mut added_sets = HashSet::new();
            let mut added_count = 0usize;
            for &vertex in &face_vertices[face] {
                let set = set_of[vertex];
                if !selected_sets.contains(&set) && added_sets.insert(set) {
                    added_count += sets[set].len();
                    for &(bone, _) in &mappings[vertex] {
                        if !selected_bones.contains(&bone) {
                            added_bones.insert(bone);
                        }
                    }
                }
            }
            if selected_bones.len() + added_bones.len() <= BONE_LIMIT_SOFT
                && vertex_count + added_count <= VERTEX_LIMIT_SOFT
            {
                taken_faces.insert(face);
                selected_bones.extend(added_bones);
                selected_sets.extend(added_sets);
                vertex_count += added_count;
            }
        }
        for set in loose {
            if vertex_count >= VERTEX_LIMIT_SOFT {
                break;
            }
            let mut added_bones = HashSet::new();
            for &(bone, _) in &mappings[sets[set][0]] {
                if !selected_bones.contains(&bone) {
                    added_bones.insert(bone);
                }
            }
            if selected_bones.len() + added_bones.len() <= BONE_LIMIT_SOFT
                && vertex_count + sets[set].len() <= VERTEX_LIMIT_SOFT
            {
                taken_loose.insert(set);
                selected_bones.extend(added_bones);
                selected_sets.insert(set);
                vertex_count += sets[set].len();
            }
        }
        // Guarantee progress: a single face bigger than the soft limits
        // still goes out as its own component. The minimum index keeps
        // the pick off HashSet iteration order.
        if taken_faces.is_empty() && taken_loose.is_empty() {
            let storable = &items_per_bone[&base_bone];
            if let Some(&face) = storable.faces.iter().min() {
                taken_faces.insert(face);
                for &vertex in &face_vertices[face] {
                    let set = set_of[vertex];
                    if selected_sets.insert(set) {
                        for &(bone, _) in &mappings[vertex] {
                            selected_bones.insert(bone);
                        }
                    }
                }
            } else if let Some(&set) = storable.loose.iter().min() {
                taken_loose.insert(set);
                selected_sets.insert(set);
                for &(bone, _) in &mappings[sets[set][0]] {
                    selected_bones.insert(bone);
                }
            }
        }
        taken = StorableItems {
            faces: taken_faces,
            loose: taken_loose,
        };
    }

    // Emit the component: each selected equipresent set in the order of its
    // first member, its members in source order (the Nth-occurrence rule
    // needs identical vertices in identical order in every component).
    let mut sorted_sets: Vec<usize> = selected_sets.iter().copied().collect();
    sorted_sets.sort_by_key(|&set| sets[set][0]);
    let mut vertices = empty_like(&mesh.vertices, vertex_capacity(&sorted_sets, sets));
    let mut remap: HashMap<usize, usize> = HashMap::new();
    // The component's bone group: the selected model bones sorted by model
    // bone index (deterministic).
    let bone_group: Vec<usize> = selected_bones.iter().copied().collect();
    let mut index_of: HashMap<usize, u8> = HashMap::with_capacity(bone_group.len());
    for (slot, &bone) in bone_group.iter().enumerate() {
        index_of.insert(
            bone,
            u8::try_from(slot)
                .map_err(|_| ModelError::InvalidModel("bone group over 256 bones"))?,
        );
    }
    for &set in &sorted_sets {
        for &vertex in &sets[set] {
            remap.insert(vertex, vertices.positions.len());
            push_vertex(&mut vertices, mesh, vertex, &index_of)?;
        }
    }
    let mut faces = Vec::with_capacity(taken.faces.len());
    let mut taken_faces: Vec<usize> = taken.faces.iter().copied().collect();
    taken_faces.sort_unstable();
    for face in taken_faces {
        faces.push(face_vertices[face].map(|vertex| remap[&vertex] as u16));
    }

    let bounds = BoundingBox::of(&vertices.positions);
    let component = Mesh {
        name: mesh.name.clone(),
        extension_headers: mesh.extension_headers.clone(),
        tags: mesh.tags.clone(),
        vertices,
        faces,
        lower_lods: Vec::new(),
        bone_group,
        material: mesh.material,
        bounds,
        order: mesh.order,
        editor_data: mesh.editor_data.clone(),
    };
    Ok((component, taken))
}

fn vertex_capacity(sets: &[usize], all_sets: &[Vec<usize>]) -> usize {
    sets.iter().map(|&set| all_sets[set].len()).sum()
}

/// Splits `mesh` into component meshes each under the soft limits.
pub(super) fn split_mesh(
    model: &Model,
    mesh: &Mesh,
    parents: &[Option<usize>],
) -> Result<Vec<Mesh>, ModelError> {
    let count = mesh.vertices.positions.len();
    let skinned = mesh.vertices.bone_indices.is_some();
    let mappings: Vec<BoneMapping> = (0..count)
        .map(|index| bone_mapping(mesh, index))
        .collect::<Result<_, _>>()?;

    // Equipresent vertex sets: vertices sharing a split key travel together.
    let mut set_of = vec![0usize; count];
    let mut sets: Vec<Vec<usize>> = Vec::new();
    let mut key_to_set: HashMap<Vec<u8>, usize> = HashMap::new();
    for (index, slot) in set_of.iter_mut().enumerate() {
        let key = split_key(mesh, index, &mappings[index]);
        match key_to_set.get(&key) {
            Some(&set) => {
                sets[set].push(index);
                *slot = set;
            }
            None => {
                key_to_set.insert(key, sets.len());
                sets.push(vec![index]);
                *slot = sets.len() - 1;
            }
        }
    }

    // Loose sets: no member referenced by any face.
    let mut used = vec![false; sets.len()];
    for face in &mesh.faces {
        for index in face {
            used[set_of[usize::from(*index)]] = true;
        }
    }
    let loose_sets: HashSet<usize> = (0..sets.len()).filter(|&set| !used[set]).collect();

    // For every bone, the faces and loose sets referencing it or one of its
    // descendants; `None` buckets the whole mesh.
    let mut items_per_bone: HashMap<Option<usize>, StorableItems> = HashMap::new();
    items_per_bone.insert(
        None,
        StorableItems {
            faces: (0..mesh.faces.len()).collect(),
            loose: loose_sets.iter().copied().collect(),
        },
    );
    if skinned {
        for bone in 0..model.bones.len() {
            items_per_bone.insert(Some(bone), StorableItems::default());
        }
        for (face_index, face) in mesh.faces.iter().enumerate() {
            for &vertex_index in face {
                for &(bone, _) in &mappings[usize::from(vertex_index)] {
                    let mut current = Some(bone);
                    while let Some(ancestor) = current {
                        let items = items_per_bone.get_mut(&Some(ancestor)).ok_or(
                            ModelError::VertexMismatch("split parents index out of range"),
                        )?;
                        if !items.faces.insert(face_index) {
                            break;
                        }
                        current = parents[ancestor];
                    }
                }
            }
        }
        for &set in &loose_sets {
            for &(bone, _) in &mappings[sets[set][0]] {
                let mut current = Some(bone);
                while let Some(ancestor) = current {
                    let items = items_per_bone.get_mut(&Some(ancestor)).ok_or(
                        ModelError::VertexMismatch("split parents index out of range"),
                    )?;
                    if !items.loose.insert(set) {
                        break;
                    }
                    current = parents[ancestor];
                }
            }
        }
    }

    let pieces = Pieces {
        face_vertices: mesh
            .faces
            .iter()
            .map(|face| face.map(usize::from))
            .collect(),
        sets,
        set_of,
        mappings,
        skinned,
    };
    let mut components = Vec::new();
    loop {
        let root = &items_per_bone[&None];
        if root.faces.is_empty() && root.loose.is_empty() {
            break;
        }
        let (component, taken) = build_component(model, mesh, parents, &items_per_bone, &pieces)?;
        for items in items_per_bone.values_mut() {
            for face in &taken.faces {
                items.faces.remove(face);
            }
            for set in &taken.loose {
                items.loose.remove(set);
            }
        }
        components.push(component);
    }
    Ok(components)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{LodRecord, MeshVertices};

    fn bone(name: &str) -> Bone {
        Bone {
            name: name.to_owned(),
            matrix: [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0,
            ],
        }
    }

    fn model_with(bones: Vec<Bone>) -> Model {
        Model {
            flags: 0,
            bones,
            materials: Vec::new(),
            meshes: Vec::new(),
            extension_headers: Vec::new(),
            bounds: BoundingBox::of(&[]),
            lod: LodRecord::for_levels(0),
        }
    }

    /// `StorableItems` holding `faces` of a mesh's face list.
    fn items_with(faces: impl IntoIterator<Item = usize>) -> StorableItems {
        StorableItems {
            faces: faces.into_iter().collect(),
            loose: HashSet::new(),
        }
    }

    #[test]
    fn split_key_covers_the_bone_mapping() {
        // Coincident vertices that differ only in a positive-weight bone,
        // or only in a weight, must not share an equipresent set.
        let mesh = mesh_of(2);
        let bone_0: BoneMapping = vec![(0, 1.0)];
        let bone_1: BoneMapping = vec![(1, 1.0)];
        assert_ne!(split_key(&mesh, 0, &bone_0), split_key(&mesh, 1, &bone_1));
        let weight_half: BoneMapping = vec![(0, 0.5)];
        assert_ne!(
            split_key(&mesh, 0, &bone_0),
            split_key(&mesh, 1, &weight_half)
        );
        assert_eq!(split_key(&mesh, 0, &bone_0), split_key(&mesh, 1, &bone_0));
    }

    #[test]
    fn dot_is_the_dot_product() {
        // `2 * 4` distinguishes the product from `2 / 4`.
        assert_eq!(dot([1.0, 2.0, 3.0], [0.5, 4.0, 2.0]), 14.5);
    }

    #[test]
    fn multiply_is_matrix_times_vector() {
        // Non-symmetric on purpose: a dropped/add-minus term shows.
        let matrix = [[1.0, 2.0, 3.0], [0.0, 1.0, 4.0], [5.0, 6.0, 0.0]];
        assert_eq!(multiply(&matrix, [1.0, 0.5, -1.0]), [-1.0, -3.5, 8.0]);
    }

    #[test]
    fn bone_position_recovers_the_bind_translation() {
        // Rotation about axis (1, 2, 2)/3 with cos 0.6, sin 0.8 — every
        // entry nonzero so no product term vanishes — plus translation
        // (2, 5, 7): the world position is -R^T * t = (-58/45, -55/9,
        // -281/45).
        let mut bone = bone("bone");
        bone.matrix = [
            29.0 / 45.0,
            -4.0 / 9.0,
            28.0 / 45.0,
            2.0, //
            28.0 / 45.0,
            7.0 / 9.0,
            -4.0 / 45.0,
            5.0, //
            -4.0 / 9.0,
            4.0 / 9.0,
            7.0 / 9.0,
            7.0,
        ];
        let position = bone_position(&bone);
        let expected = [-58.0f32 / 45.0, -55.0 / 9.0, -281.0 / 45.0];
        for component in 0..3 {
            assert!((position[component] - expected[component]).abs() < 1e-6);
        }
    }

    #[test]
    fn sort_vector_points_along_the_cloud() {
        let o = [5.0, -3.0, 2.0];
        let d = [1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0];
        let points: Vec<[f32; 3]> = (0..10)
            .map(|t| {
                let t = t as f32;
                [o[0] + t * d[0], o[1] + t * d[1], o[2] + t * d[2]]
            })
            .collect();
        // Bone behind the cloud: the axis runs from the bone into it.
        let behind = [o[0] - d[0], o[1] - d[1], o[2] - d[2]];
        let vector = sort_vector(&points, behind);
        let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
        assert!((length - 1.0).abs() < 1e-5);
        for axis in 0..3 {
            assert!((vector[axis] - d[axis]).abs() < 1e-5);
        }
        // Bone past the far end: the axis points back at the cloud.
        let ahead = [o[0] + 20.0 * d[0], o[1] + 20.0 * d[1], o[2] + 20.0 * d[2]];
        let vector = sort_vector(&points, ahead);
        for axis in 0..3 {
            assert!((vector[axis] + d[axis]).abs() < 1e-5);
        }
        // A degenerate cloud picks the first axis deterministically.
        let same = vec![[5.0, -3.0, 2.0]; 5];
        assert_eq!(sort_vector(&same, [0.0, 0.0, 0.0]), [1.0, 0.0, 0.0]);
        // A cloud spread along a single axis picks that axis.
        for axis in 0..3 {
            let points: Vec<[f32; 3]> = (0..10)
                .map(|t| {
                    let mut point = [0.0; 3];
                    point[axis] = t as f32;
                    point
                })
                .collect();
            let vector = sort_vector(&points, [-1.0, -1.0, -1.0]);
            let mut expected = [0.0; 3];
            expected[axis] = 1.0;
            assert_eq!(vector, expected);
        }
        // An exactly representable mean of zero: the orientation dot is
        // exactly 0 and the axis must not flip on it.
        let points = vec![
            [-2.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
        ];
        assert_eq!(sort_vector(&points, [0.0, 0.0, 0.0]), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn sort_vector_finds_the_principal_axis() {
        // Covariance [[18,0,0],[0,10,10],[0,10,10]]: the principal axis is
        // (0, 1, 1)/sqrt(2) (eigenvalue 20), not x (18) — a seed on the
        // largest diagonal alone would sit on x.
        let points = vec![
            [3.0, 0.0, 0.0],
            [-3.0, 0.0, 0.0],
            [0.0, 2.0, 2.0],
            [0.0, -2.0, -2.0],
            [0.0, 1.0, 1.0],
            [0.0, -1.0, -1.0],
        ];
        let vector = sort_vector(&points, [0.0, -1.0, -1.0]);
        let s = 1.0f32 / 2.0f32.sqrt();
        for axis in 0..3 {
            assert!(
                (vector[axis] - [0.0, s, s][axis]).abs() < 1e-5,
                "{vector:?} is not (0, 1, 1)/sqrt(2)"
            );
        }
    }

    #[test]
    fn sort_vector_breaks_degenerate_ties() {
        // Equal y/z variance with no covariance: the deterministic pick
        // is y.
        let mut points = Vec::new();
        for t in -5..=5 {
            points.push([0.0, t as f32, 0.0]);
            points.push([0.0, 0.0, t as f32]);
        }
        assert_eq!(sort_vector(&points, [-1.0, -1.0, -1.0]), [0.0, 1.0, 0.0]);

        // Doubling the z arm makes z the principal axis.
        let mut points = Vec::new();
        for t in -5..=5 {
            points.push([0.0, t as f32, 0.0]);
            points.push([0.0, 0.0, 2.0 * t as f32]);
        }
        assert_eq!(sort_vector(&points, [-1.0, -1.0, -1.0]), [0.0, 0.0, 1.0]);

        // An x-only cloud stays x: a far-off bone only orients the axis
        // (a bad orientation computation flips this), and a bone at the
        // mean — a zero dot — keeps the sign.
        let points: Vec<[f32; 3]> = (-5..=5).map(|t| [t as f32, 0.0, 0.0]).collect();
        assert_eq!(sort_vector(&points, [-1.0, 100.0, 100.0]), [1.0, 0.0, 0.0]);
        assert_eq!(sort_vector(&points, [0.0, 0.0, 0.0]), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn select_base_bone_prefers_named_then_lowest() {
        let items_at = |model: &Model, bones: &[usize]| {
            let mut items: HashMap<Option<usize>, StorableItems> = HashMap::new();
            items.insert(None, StorableItems::default());
            for bone in 0..model.bones.len() {
                items.insert(
                    Some(bone),
                    if bones.contains(&bone) {
                        items_with([0])
                    } else {
                        StorableItems::default()
                    },
                );
            }
            items
        };

        // A preferred bone with items beats a lower-indexed one.
        let model = model_with(vec![bone("x0"), bone("sk_foot_l"), bone("x2")]);
        let items = items_at(&model, &[0, 1]);
        assert_eq!(select_base_bone(&model, &items), Some(1));

        // A preferred bone with no items falls back to the lowest index.
        let items = items_at(&model, &[0]);
        assert_eq!(select_base_bone(&model, &items), Some(0));

        // A bone whose items are loose-only still counts.
        let items = {
            let mut items = items_at(&model, &[]);
            items.get_mut(&Some(2)).unwrap().loose.insert(0);
            items
        };
        assert_eq!(select_base_bone(&model, &items), Some(2));

        // Nothing left under any bone: `None`, the whole-mesh root.
        let items = items_at(&model, &[]);
        assert_eq!(select_base_bone(&model, &items), None);
    }

    /// `pieces` over `count` singleton sets of one vertex each; `faces`
    /// selects which of `face_vertices` exist.
    fn pieces_of(
        face_vertices: Vec<[usize; 3]>,
        sets: Vec<Vec<usize>>,
        mappings: Vec<BoneMapping>,
        skinned: bool,
    ) -> Pieces {
        let mut set_of = vec![0usize; mappings.len()];
        for (set, members) in sets.iter().enumerate() {
            for &member in members {
                set_of[member] = set;
            }
        }
        Pieces {
            face_vertices,
            sets,
            set_of,
            mappings,
            skinned,
        }
    }

    #[test]
    fn fits_in_submesh_at_and_past_each_soft_limit() {
        // Faces: exactly the soft limit fits, one more does not.
        let pieces = pieces_of(
            vec![[0, 1, 2]; FACE_LIMIT_SOFT + 1],
            vec![vec![0], vec![1], vec![2]],
            vec![Vec::new(); 3],
            false,
        );
        assert!(fits_in_submesh(&items_with(0..FACE_LIMIT_SOFT), &pieces));
        assert!(!fits_in_submesh(
            &items_with(0..FACE_LIMIT_SOFT + 1),
            &pieces
        ));

        // Vertices: a loose set of exactly the soft limit fits, one
        // member more does not.
        let pieces = pieces_of(
            vec![],
            vec![(0..VERTEX_LIMIT_SOFT).collect()],
            vec![Vec::new(); VERTEX_LIMIT_SOFT],
            false,
        );
        let loose = StorableItems {
            faces: HashSet::new(),
            loose: [0].into_iter().collect(),
        };
        assert!(fits_in_submesh(&loose, &pieces));
        let pieces = pieces_of(
            vec![],
            vec![(0..VERTEX_LIMIT_SOFT + 1).collect()],
            vec![Vec::new(); VERTEX_LIMIT_SOFT + 1],
            false,
        );
        assert!(!fits_in_submesh(&loose, &pieces));

        // Bones: singleton sets mapped to BONE_LIMIT_SOFT bones fit, one
        // bone more does not.
        let mappings: Vec<BoneMapping> = (0..BONE_LIMIT_SOFT + 1)
            .map(|bone| vec![(bone, 1.0)])
            .collect();
        let pieces = pieces_of(
            vec![],
            (0..BONE_LIMIT_SOFT + 1).map(|i| vec![i]).collect(),
            mappings,
            true,
        );
        let loose = StorableItems {
            faces: HashSet::new(),
            loose: (0..BONE_LIMIT_SOFT).collect(),
        };
        assert!(fits_in_submesh(&loose, &pieces));
        let loose = StorableItems {
            faces: HashSet::new(),
            loose: (0..BONE_LIMIT_SOFT + 1).collect(),
        };
        assert!(!fits_in_submesh(&loose, &pieces));
    }

    fn mesh_of(count: usize) -> Mesh {
        Mesh {
            name: None,
            extension_headers: Vec::new(),
            tags: Vec::new(),
            vertices: MeshVertices {
                positions: vec![[0.0; 3]; count],
                normals: None,
                tangents: None,
                bitangents: None,
                colors: None,
                uvs: vec![vec![[0.0; 2]; count]],
                bone_indices: None,
                bone_weights: None,
                bone_weight_width: 4,
            },
            faces: Vec::new(),
            lower_lods: Vec::new(),
            bone_group: Vec::new(),
            material: 0,
            bounds: BoundingBox::of(&[]),
            order: 0,
            editor_data: Vec::new(),
        }
    }

    #[test]
    fn build_component_takes_faces_to_the_soft_limit() {
        // FACE_LIMIT_SOFT + 2 faces of three singleton sets each: the
        // fragment takes exactly FACE_LIMIT_SOFT.
        let count = (FACE_LIMIT_SOFT + 2) * 3;
        let sets: Vec<Vec<usize>> = (0..FACE_LIMIT_SOFT + 2)
            .map(|face| (0..3).map(|vertex| face * 3 + vertex).collect())
            .collect();
        let face_vertices: Vec<[usize; 3]> =
            sets.iter().map(|set| [set[0], set[1], set[2]]).collect();
        let pieces = pieces_of(face_vertices, sets, vec![Vec::new(); count], false);
        let mesh = mesh_of(count);
        let mut items: HashMap<Option<usize>, StorableItems> = HashMap::new();
        items.insert(None, items_with(0..FACE_LIMIT_SOFT + 2));
        let model = model_with(vec![]);
        let (component, taken) = build_component(&model, &mesh, &[], &items, &pieces).unwrap();
        assert_eq!(taken.faces.len(), FACE_LIMIT_SOFT);
        assert_eq!(component.faces.len(), FACE_LIMIT_SOFT);
    }

    #[test]
    fn build_component_takes_loose_sets_to_the_soft_limit() {
        let sets: Vec<Vec<usize>> = (0..VERTEX_LIMIT_SOFT + 2).map(|i| vec![i]).collect();
        let pieces = pieces_of(vec![], sets, vec![Vec::new(); VERTEX_LIMIT_SOFT + 2], false);
        let mesh = mesh_of(VERTEX_LIMIT_SOFT + 2);
        let mut items: HashMap<Option<usize>, StorableItems> = HashMap::new();
        items.insert(
            None,
            StorableItems {
                faces: HashSet::new(),
                loose: (0..VERTEX_LIMIT_SOFT + 2).collect(),
            },
        );
        let model = model_with(vec![]);
        let (component, taken) = build_component(&model, &mesh, &[], &items, &pieces).unwrap();
        assert_eq!(taken.loose.len(), VERTEX_LIMIT_SOFT);
        assert_eq!(component.vertices.positions.len(), VERTEX_LIMIT_SOFT);
    }

    #[test]
    fn build_component_emits_one_over_limit_face_for_progress() {
        // A single face whose equipresent set exceeds the vertex limit
        // cannot fit anywhere: it still goes out as its own component.
        let pieces = pieces_of(
            vec![[0, 1, 2]],
            vec![(0..VERTEX_LIMIT_SOFT + 1).collect()],
            vec![Vec::new(); VERTEX_LIMIT_SOFT + 1],
            false,
        );
        let mesh = mesh_of(VERTEX_LIMIT_SOFT + 1);
        let mut items: HashMap<Option<usize>, StorableItems> = HashMap::new();
        items.insert(None, items_with([0]));
        let model = model_with(vec![]);
        let (component, taken) = build_component(&model, &mesh, &[], &items, &pieces).unwrap();
        assert_eq!(taken.faces.len(), 1);
        assert_eq!(component.faces, vec![[0, 1, 2]]);
    }

    /// `mesh_of(count)` with every vertex fully weighted to bone 0.
    fn skinned_mesh_of(count: usize) -> Mesh {
        let mut mesh = mesh_of(count);
        mesh.bone_group = vec![0];
        mesh.vertices.bone_indices = Some(vec![[0, 0, 0, 0]; count]);
        mesh.vertices.bone_weights = Some(vec![[1.0, 0.0, 0.0, 0.0]; count]);
        mesh
    }

    // The skinned variant: the progress branch must also select the
    // face's bones, or `push_vertex` binds its vertices to slot 0.
    #[test]
    fn build_component_emits_one_skinned_over_limit_face_for_progress() {
        let count = VERTEX_LIMIT_SOFT + 1;
        let pieces = pieces_of(
            vec![[0, 1, 2]],
            vec![(0..count).collect()],
            vec![vec![(0, 1.0)]; count],
            true,
        );
        let mesh = skinned_mesh_of(count);
        let mut items: HashMap<Option<usize>, StorableItems> = HashMap::new();
        items.insert(None, items_with([0]));
        items.insert(Some(0), items_with([0]));
        let model = model_with(vec![bone("bone0")]);
        let (component, taken) = build_component(&model, &mesh, &[None], &items, &pieces).unwrap();
        assert_eq!(taken.faces.len(), 1);
        assert_eq!(component.bone_group, vec![0]);
    }

    // A single loose set over the vertex limit and no faces: the loose
    // progress branch takes the set whole and selects its bones.
    #[test]
    fn build_component_emits_one_over_limit_loose_set_for_progress() {
        let count = VERTEX_LIMIT_SOFT + 1;
        let pieces = pieces_of(
            vec![],
            vec![(0..count).collect()],
            vec![vec![(0, 1.0)]; count],
            true,
        );
        let mesh = skinned_mesh_of(count);
        let loose = StorableItems {
            faces: HashSet::new(),
            loose: [0].into_iter().collect(),
        };
        let mut items: HashMap<Option<usize>, StorableItems> = HashMap::new();
        items.insert(None, loose.clone());
        items.insert(Some(0), loose);
        let model = model_with(vec![bone("bone0")]);
        let (component, taken) = build_component(&model, &mesh, &[None], &items, &pieces).unwrap();
        assert_eq!(taken.loose, [0].into_iter().collect::<HashSet<usize>>());
        assert_eq!(component.vertices.positions.len(), count);
        assert_eq!(component.bone_group, vec![0]);
    }

    // Faces whose equipresent sets hold four members each: the vertex
    // soft limit binds after 15750 faces, before the face one — a
    // miscounted member total takes them all.
    #[test]
    fn build_component_takes_faces_to_the_vertex_limit() {
        const FACES: usize = FACE_LIMIT_SOFT;
        let count = FACES * 4;
        let sets: Vec<Vec<usize>> = (0..FACES)
            .map(|face| (0..4).map(|member| face * 4 + member).collect())
            .collect();
        let face_vertices: Vec<[usize; 3]> =
            sets.iter().map(|set| [set[0], set[1], set[2]]).collect();
        let pieces = pieces_of(face_vertices, sets, vec![Vec::new(); count], false);
        let mesh = mesh_of(count);
        let mut items: HashMap<Option<usize>, StorableItems> = HashMap::new();
        items.insert(None, items_with(0..FACES));
        let model = model_with(vec![]);
        let (component, taken) = build_component(&model, &mesh, &[], &items, &pieces).unwrap();
        assert_eq!(taken.faces.len(), VERTEX_LIMIT_SOFT / 4);
        assert_eq!(component.vertices.positions.len(), VERTEX_LIMIT_SOFT);
    }

    // Sixty-one loose singletons weighted to their own bones, in a
    // skinned fragment: the bone soft limit stops the loose loop at
    // sixty — the bone and vertex checks must both hold for a take.
    #[test]
    fn build_component_takes_loose_sets_to_the_bone_limit() {
        const BONES: usize = BONE_LIMIT_SOFT + 1;
        let sets: Vec<Vec<usize>> = (0..BONES).map(|i| vec![i]).collect();
        let mappings: Vec<BoneMapping> = (0..BONES).map(|bone| vec![(bone, 1.0)]).collect();
        let pieces = pieces_of(vec![], sets, mappings, true);
        let mut mesh = mesh_of(BONES);
        mesh.bone_group = (0..BONES).collect();
        mesh.vertices.bone_indices = Some((0..BONES).map(|bone| [bone as u8, 0, 0, 0]).collect());
        mesh.vertices.bone_weights = Some(vec![[1.0, 0.0, 0.0, 0.0]; BONES]);
        let mut items: HashMap<Option<usize>, StorableItems> = HashMap::new();
        items.insert(
            None,
            StorableItems {
                faces: HashSet::new(),
                loose: (0..BONES).collect(),
            },
        );
        let model = model_with((0..BONES).map(|index| bone(&format!("b{index}"))).collect());
        let (component, taken) = build_component(&model, &mesh, &[], &items, &pieces).unwrap();
        assert_eq!(taken.loose.len(), BONE_LIMIT_SOFT);
        assert_eq!(component.vertices.positions.len(), BONE_LIMIT_SOFT);
    }

    // Two-member loose sets: the vertex count stops the loose loop at
    // the soft limit, and a member length counted wrong — none or all —
    // takes half or twice as many sets.
    #[test]
    fn build_component_vertex_limit_counts_multi_member_loose_sets() {
        const SETS: usize = 32_000;
        let count = SETS * 2;
        let sets: Vec<Vec<usize>> = (0..SETS)
            .map(|set| (0..2).map(|member| set * 2 + member).collect())
            .collect();
        let pieces = pieces_of(vec![], sets, vec![Vec::new(); count], false);
        let mesh = mesh_of(count);
        let mut items: HashMap<Option<usize>, StorableItems> = HashMap::new();
        items.insert(
            None,
            StorableItems {
                faces: HashSet::new(),
                loose: (0..SETS).collect(),
            },
        );
        let model = model_with(vec![]);
        let (component, taken) = build_component(&model, &mesh, &[], &items, &pieces).unwrap();
        assert_eq!(taken.loose.len(), VERTEX_LIMIT_SOFT / 2);
        assert_eq!(component.vertices.positions.len(), VERTEX_LIMIT_SOFT);
    }

    #[test]
    fn vertex_capacity_counts_selected_members() {
        let sets = vec![vec![0], vec![1, 2], vec![3, 4, 5]];
        assert_eq!(vertex_capacity(&[0, 2], &sets), 4);
    }

    /// A `Pieces` over `count` singleton sets of one vertex each, with
    /// `mappings` per vertex and no faces.
    fn singleton_pieces(count: usize, mappings: Vec<BoneMapping>, skinned: bool) -> Pieces {
        pieces_of(
            Vec::new(),
            (0..count).map(|vertex| vec![vertex]).collect(),
            mappings,
            skinned,
        )
    }

    // Sixty-one faces weighted to sixty-one bones under one parent bone:
    // the fragment's face order is the sort axis — the faces sit at
    // increasing x, the axis is +x, and the bone limit drops the
    // furthest-behind face, not the highest-indexed one an index order
    // would reach last.
    #[test]
    fn a_fragment_takes_faces_by_decreasing_projection() {
        const FACES: usize = BONE_LIMIT_SOFT + 1;
        let count = 3 * FACES;
        let mut mesh = mesh_of(count);
        mesh.vertices.positions = (0..count)
            .map(|vertex| [(vertex / 3) as f32, 0.0, 0.0])
            .collect();
        mesh.bone_group = (0..FACES).collect();
        mesh.vertices.bone_indices = Some(
            (0..count)
                .map(|vertex| [(vertex / 3) as u8, 0, 0, 0])
                .collect(),
        );
        mesh.vertices.bone_weights = Some(vec![[1.0, 0.0, 0.0, 0.0]; count]);
        let mut pieces = singleton_pieces(
            count,
            (0..count).map(|vertex| vec![(vertex / 3, 1.0)]).collect(),
            true,
        );
        pieces.face_vertices = (0..FACES)
            .map(|face| [3 * face, 3 * face + 1, 3 * face + 2])
            .collect();
        // Bone 0 parents the other sixty, so every face lands in its
        // bucket — the fragment the take loop walks.
        let parents: Vec<Option<usize>> = (0..FACES)
            .map(|index| index.checked_sub(1).map(|_| 0))
            .collect();
        let model = model_with((0..FACES).map(|index| bone(&format!("b{index}"))).collect());
        let mut items: HashMap<Option<usize>, StorableItems> = HashMap::new();
        items.insert(None, items_with(0..FACES));
        items.insert(Some(0), items_with(0..FACES));
        let (component, taken) = build_component(&model, &mesh, &parents, &items, &pieces).unwrap();
        // Sixty faces fit the bone limit: the sixty furthest along the
        // axis, not the sixty lowest-indexed.
        assert_eq!(taken.faces, (1..FACES).collect::<HashSet<usize>>());
        assert_eq!(component.faces.len(), FACES - 1);
    }

    // Sixty-one loose sets weighted to sixty-one bones: the same axis
    // decides which sixty the fragment takes.
    #[test]
    fn a_fragment_takes_loose_sets_by_decreasing_projection() {
        const SETS: usize = BONE_LIMIT_SOFT + 1;
        let mut mesh = mesh_of(SETS);
        mesh.vertices.positions = (0..SETS).map(|set| [set as f32, 0.0, 0.0]).collect();
        mesh.bone_group = (0..SETS).collect();
        mesh.vertices.bone_indices = Some((0..SETS).map(|set| [set as u8, 0, 0, 0]).collect());
        mesh.vertices.bone_weights = Some(vec![[1.0, 0.0, 0.0, 0.0]; SETS]);
        let pieces = singleton_pieces(SETS, (0..SETS).map(|set| vec![(set, 1.0)]).collect(), true);
        let parents: Vec<Option<usize>> = (0..SETS)
            .map(|index| index.checked_sub(1).map(|_| 0))
            .collect();
        let model = model_with((0..SETS).map(|index| bone(&format!("b{index}"))).collect());
        let loose = StorableItems {
            faces: HashSet::new(),
            loose: (0..SETS).collect(),
        };
        let mut items: HashMap<Option<usize>, StorableItems> = HashMap::new();
        items.insert(None, loose.clone());
        items.insert(Some(0), loose);
        let (component, taken) = build_component(&model, &mesh, &parents, &items, &pieces).unwrap();
        assert_eq!(taken.loose, (1..SETS).collect::<HashSet<usize>>());
        assert_eq!(
            component.vertices.positions,
            (1..SETS)
                .map(|set| [set as f32, 0.0, 0.0])
                .collect::<Vec<_>>()
        );
    }

    // The sort cloud mixes face vertices and loose first members, so each
    // half steers the axis. Here the loose sets sit on two y-spread
    // columns (x = 0 and x = 4000) and the face vertices on the y axis
    // below the loose cloud's centre: the columns' separation makes ~+x
    // the principal axis and the face points tilt it toward +y. Under
    // that axis the far column wins the vertex budget and its highest-y
    // members go first; a loose-only cloud has axis exactly +x (the far
    // column ties and the index order takes over), a face-only cloud
    // axis exactly +y (the columns interleave by y).
    #[test]
    fn a_fragment_sorts_by_the_whole_cloud() {
        const MEMBERS: usize = 3_000;
        const FACES: usize = 20;
        const LOOSE: usize = 12_000;
        const COLUMN: usize = LOOSE / 2;
        const FAR: f32 = 4_000.0;
        let count = FACES * MEMBERS + LOOSE;
        let mut mesh = mesh_of(count);
        for face in 0..FACES {
            for corner in 0..3 {
                mesh.vertices.positions[face * MEMBERS + corner] = [0.0, face as f32 * 100.0, 0.0];
            }
        }
        for set in 0..LOOSE {
            mesh.vertices.positions[FACES * MEMBERS + set] = if set < COLUMN {
                [0.0, set as f32, 0.0]
            } else {
                [FAR, (set - COLUMN) as f32, 0.0]
            };
        }
        let mut sets: Vec<Vec<usize>> = (0..FACES)
            .map(|face| (face * MEMBERS..(face + 1) * MEMBERS).collect())
            .collect();
        sets.extend((0..LOOSE).map(|set| vec![FACES * MEMBERS + set]));
        let face_vertices: Vec<[usize; 3]> = (0..FACES)
            .map(|face| [face * MEMBERS, face * MEMBERS + 1, face * MEMBERS + 2])
            .collect();
        let pieces = pieces_of(face_vertices, sets, vec![Vec::new(); count], false);
        let mut items: HashMap<Option<usize>, StorableItems> = HashMap::new();
        items.insert(
            None,
            StorableItems {
                faces: (0..FACES).collect(),
                loose: (FACES..FACES + LOOSE).collect(),
            },
        );
        let model = model_with(vec![]);
        let (component, taken) = build_component(&model, &mesh, &[], &items, &pieces).unwrap();
        assert_eq!(taken.faces, (0..FACES).collect::<HashSet<usize>>());
        // The far column takes the whole loose budget, highest y first:
        // loose set `s` sits at y = s - FACES - COLUMN on that column.
        assert_eq!(
            taken.loose,
            (FACES + COLUMN + COLUMN / 2..FACES + LOOSE).collect::<HashSet<usize>>()
        );
        assert_eq!(
            component.vertices.positions.len(),
            FACES * MEMBERS + COLUMN / 2
        );
    }

    // The component's vertex order is its sets' first members', and its
    // face order the source index order — not the `HashSet` order the
    // sets and faces were collected in. Fresh `HashSet`s each run, so an
    // order-dependent emit shows.
    #[test]
    fn a_component_emits_sets_by_first_member_and_faces_by_index() {
        // Ten faces on ten singleton sets whose members interleave, plus
        // a loose eleventh set: set `set` holds `MEMBERS[set]` alone.
        const MEMBERS: [usize; 11] = [9, 0, 8, 1, 7, 2, 6, 3, 5, 4, 10];
        let count = MEMBERS.len();
        let mut mesh = mesh_of(count);
        mesh.vertices.positions = (0..count).map(|vertex| [vertex as f32, 0.0, 0.0]).collect();
        let mut pieces = singleton_pieces(count, vec![Vec::new(); count], false);
        pieces.face_vertices = (0..count - 1).map(|face| [MEMBERS[face]; 3]).collect();
        let model = model_with(vec![]);
        let expected_positions: Vec<[f32; 3]> =
            (0..count).map(|vertex| [vertex as f32, 0.0, 0.0]).collect();
        let expected_faces: Vec<[u16; 3]> = (0..count - 1)
            .map(|face| [MEMBERS[face] as u16; 3])
            .collect();
        for _ in 0..8 {
            let mut items: HashMap<Option<usize>, StorableItems> = HashMap::new();
            items.insert(
                None,
                StorableItems {
                    faces: (0..count - 1).collect(),
                    loose: [count - 1].into_iter().collect(),
                },
            );
            let (component, _) = build_component(&model, &mesh, &[], &items, &pieces).unwrap();
            assert_eq!(component.vertices.positions, expected_positions);
            assert_eq!(component.faces, expected_faces);
        }
    }
}
