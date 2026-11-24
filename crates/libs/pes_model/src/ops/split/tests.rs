use std::collections::HashSet;

use super::*;
use crate::format::{Bone, BoundingBox, LodRecord, PreFoxModel};

fn bone(name: &str) -> Bone {
    Bone {
        name: name.to_owned(),
        matrix: [
            1.0, 0.0, 0.0, 0.0, // row 0
            0.0, 1.0, 0.0, 0.0, // row 1
            0.0, 0.0, 1.0, 0.0, // row 2
        ],
    }
}

/// A width x height grid: vertex `y * width + x` at `[x, y, 0]`,
/// weighted to bone `x * bones / width`, two faces per cell where
/// every index fits in u16 (larger grids leave the rest loose).
fn grid(width: usize, height: usize, bones: usize) -> Mesh {
    let mut vertices = MeshVertices {
        positions: Vec::new(),
        normals: Some(Vec::new()),
        tangents: None,
        bitangents: None,
        colors: None,
        uvs: vec![Vec::new()],
        bone_indices: (bones > 0).then(Vec::new),
        bone_weights: (bones > 0).then(Vec::new),
        bone_weight_width: 4,
    };
    for y in 0..height {
        for x in 0..width {
            vertices.positions.push([x as f32, y as f32, 0.0]);
            vertices.normals.as_mut().unwrap().push([0.0, 0.0, 1.0]);
            vertices.uvs[0].push([x as f32 / width as f32, y as f32 / height as f32]);
            if bones > 0 {
                let bone = (x * bones / width) as u8;
                vertices
                    .bone_weights
                    .as_mut()
                    .unwrap()
                    .push([1.0, 0.0, 0.0, 0.0]);
                vertices
                    .bone_indices
                    .as_mut()
                    .unwrap()
                    .push([bone, 0, 0, 0]);
            }
        }
    }
    let mut faces = Vec::new();
    for y in 0..height - 1 {
        for x in 0..width - 1 {
            let v00 = y * width + x;
            let v10 = v00 + 1;
            let v01 = v00 + width;
            let v11 = v01 + 1;
            if v11 <= usize::from(u16::MAX) {
                faces.push([v00 as u16, v10 as u16, v11 as u16]);
                faces.push([v00 as u16, v11 as u16, v01 as u16]);
            }
        }
    }
    let bounds = BoundingBox::of(&vertices.positions);
    Mesh {
        name: None,
        extension_headers: Vec::new(),
        tags: Vec::new(),
        vertices,
        faces,
        lower_lods: Vec::new(),
        bone_group: (0..bones).collect(),
        material: 0,
        bounds,
        order: 0,
        editor_data: Vec::new(),
    }
}

/// One mesh in a model with `bones` bones (all roots).
fn grid_model(mesh: Mesh, bones: usize) -> Model {
    Model {
        flags: 0,
        bones: (0..bones)
            .map(|index| bone(&format!("bone{index}")))
            .collect(),
        materials: vec!["Material".to_owned()],
        meshes: vec![mesh],
        extension_headers: Vec::new(),
        bounds: BoundingBox::of(&[]),
        lod: LodRecord::for_levels(0),
    }
}

/// A vertex resolved to the values a face sees: position, normal, uv
/// maps and the model-level bone mapping (bits, so tuples order).
type VertexTuple = ([u32; 3], Option<[u32; 3]>, Vec<[u32; 2]>, Vec<(usize, u32)>);

fn vertex_tuple(mesh: &Mesh, index: usize) -> VertexTuple {
    (
        mesh.vertices.positions[index].map(f32::to_bits),
        mesh.vertices
            .normals
            .as_ref()
            .map(|normals| normals[index].map(f32::to_bits)),
        mesh.vertices
            .uvs
            .iter()
            .map(|map| map[index].map(f32::to_bits))
            .collect(),
        bone_mapping(mesh, index)
            .unwrap()
            .iter()
            .map(|&(bone, weight)| (bone, weight.to_bits()))
            .collect(),
    )
}

/// The mesh's faces as vertex tuples, sorted — a multiset comparison
/// keeps a deliberately repeated face's multiplicity visible.
fn face_tuples(mesh: &Mesh) -> Vec<[VertexTuple; 3]> {
    let mut faces: Vec<[VertexTuple; 3]> = mesh
        .faces
        .iter()
        .map(|face| face.map(|index| vertex_tuple(mesh, usize::from(index))))
        .collect();
    faces.sort();
    faces
}

fn components_under_limits(model: &Model) {
    for mesh in &model.meshes {
        assert!(mesh.bone_group.len() <= BONE_LIMIT_SOFT);
        assert!(mesh.vertices.positions.len() <= VERTEX_LIMIT_SOFT);
        assert!(mesh.faces.len() <= FACE_LIMIT_SOFT);
    }
}

/// The source's used bones in source bone-group order.
fn used_bones(mesh: &Mesh) -> Vec<usize> {
    let mut used = HashSet::new();
    for index in 0..mesh.vertices.positions.len() {
        for (bone, _) in bone_mapping(mesh, index).unwrap() {
            used.insert(bone);
        }
    }
    let mut bones: Vec<usize> = used.iter().copied().collect();
    bones.sort_unstable();
    bones
}

fn split_headers(mesh: &Mesh) -> Vec<String> {
    mesh.extension_headers
        .iter()
        .filter(|header| split_group_key(header).is_some())
        .cloned()
        .collect()
}

// 1
#[test]
fn needs_splitting_limits() {
    let mut mesh = grid(8, 8, 0);
    assert!(!needs_splitting(&mesh));
    mesh.bone_group = (0..BONE_LIMIT_HARD).collect();
    assert!(!needs_splitting(&mesh));
    mesh.bone_group = (0..BONE_LIMIT_HARD + 1).collect();
    assert!(needs_splitting(&mesh));
    mesh.bone_group = Vec::new();

    mesh.vertices.positions = vec![[0.0; 3]; VERTEX_LIMIT_HARD];
    assert!(!needs_splitting(&mesh));
    mesh.vertices.positions.push([0.0; 3]);
    assert!(needs_splitting(&mesh));
    mesh.vertices.positions = vec![[0.0; 3]; 4];

    mesh.faces = vec![[0, 1, 2]; FACE_LIMIT_HARD];
    assert!(!needs_splitting(&mesh));
    mesh.faces.push([0, 1, 2]);
    assert!(needs_splitting(&mesh));
}

// 2
#[test]
fn effective_parents_chain() {
    let model = Model {
        flags: 0,
        bones: vec![bone("dsk_hip"), bone("sk_belly"), bone("sk_chest")],
        materials: Vec::new(),
        meshes: Vec::new(),
        extension_headers: Vec::new(),
        bounds: BoundingBox::of(&[]),
        lod: LodRecord::for_levels(0),
    };
    let parents = effective_parents(&model, &[None, Some(0), Some(1)]);
    assert_eq!(parents[2], None); // sk_chest is the root
    assert_eq!(parents[1], Some(2)); // sk_belly's parent is the chest
    assert_eq!(parents[0], Some(1)); // dsk_hip's parent is the belly

    // A parent cycle is cut.
    let cycle = effective_parents(&model, &[Some(1), Some(0), None]);
    assert_eq!(cycle[1], None);

    // All roots stays all roots.
    assert_eq!(
        effective_parents(&model, &[None, None, None]),
        [None, None, None]
    );
}

// 3
#[test]
fn untouched_mesh() {
    let mut model = grid_model(grid(8, 8, 2), 2);
    let before = model.clone();
    assert!(!encode(&mut model, &[None, None]).unwrap());
    assert_eq!(model, before);
}

// 4
#[test]
fn split_bones() {
    let source = grid(70, 20, 70);
    let source_faces = face_tuples(&source);
    let mut model = grid_model(source, 70);
    let parents: Vec<Option<usize>> = vec![None; 70];
    assert!(encode(&mut model, &parents).unwrap());

    components_under_limits(&model);
    assert!(model.meshes.len() > 1);
    let mut union: Vec<[VertexTuple; 3]> = model.meshes.iter().flat_map(face_tuples).collect();
    union.sort();
    assert_eq!(union, source_faces);

    decode(&mut model).unwrap();
    assert_eq!(model.meshes.len(), 1);
    let combined = &model.meshes[0];
    assert_eq!(combined.vertices.positions.len(), 70 * 20);
    assert_eq!(face_tuples(combined), source_faces);
}

// 5
#[test]
fn split_vertices() {
    let source = grid(300, 300, 0);
    let source_faces = face_tuples(&source);
    let mut model = grid_model(source, 0);
    assert!(encode(&mut model, &[]).unwrap());

    components_under_limits(&model);
    assert!(model.meshes.len() > 1);
    let mut union: Vec<[VertexTuple; 3]> = model.meshes.iter().flat_map(face_tuples).collect();
    union.sort();
    assert_eq!(union, source_faces);
}

// 6
fn check_round_trip(mut model: Model, source: Mesh, parents: &[Option<usize>]) {
    let source_faces = face_tuples(&source);
    let source_bones = used_bones(&source);
    assert!(encode(&mut model, parents).unwrap());
    decode(&mut model).unwrap();

    assert_eq!(model.meshes.len(), 1);
    let combined = &model.meshes[0];
    assert_eq!(face_tuples(combined), source_faces);
    assert_eq!(combined.material, source.material);
    assert_eq!(combined.name, source.name);
    assert_eq!(combined.tags, source.tags);
    assert_eq!(combined.order, source.order);
    assert_eq!(combined.editor_data, source.editor_data);
    assert_eq!(combined.bone_group, source_bones);
    assert!(split_headers(combined).is_empty());
}

#[test]
fn round_trip_bones() {
    let source = grid(70, 20, 70);
    let model = grid_model(source.clone(), 70);
    check_round_trip(model, source, &vec![None; 70]);
}

#[test]
fn round_trip_vertices() {
    let source = grid(300, 300, 0);
    let model = grid_model(source.clone(), 0);
    check_round_trip(model, source, &[]);
}

// A face written twice in the source must come back twice — the multiset
// comparison in `check_round_trip` would not see a collapsed copy.
#[test]
fn a_repeated_triangle_survives_the_round_trip() {
    let mut source = grid(70, 20, 70);
    source.faces.push(source.faces[0]);
    let model = grid_model(source.clone(), 70);
    check_round_trip(model, source, &vec![None; 70]);
}

// 7: two byte-identical vertices, each referenced by faces that must
// land in different components (disjoint bone sets over the limit).
#[test]
fn duplicate_vertices() {
    let bones = 70;
    let mut mesh = grid(1, 1, bones);
    mesh.vertices.positions.clear();
    mesh.vertices.normals.as_mut().unwrap().clear();
    mesh.vertices.uvs[0].clear();
    mesh.vertices.bone_weights.as_mut().unwrap().clear();
    mesh.vertices.bone_indices.as_mut().unwrap().clear();
    mesh.faces.clear();
    let add = |mesh: &mut Mesh, position: [f32; 3], bone: u8| {
        mesh.vertices.positions.push(position);
        mesh.vertices
            .normals
            .as_mut()
            .unwrap()
            .push([0.0, 0.0, 1.0]);
        mesh.vertices.uvs[0].push([position[0] / 200.0, position[1]]);
        mesh.vertices
            .bone_weights
            .as_mut()
            .unwrap()
            .push([1.0, 0.0, 0.0, 0.0]);
        mesh.vertices
            .bone_indices
            .as_mut()
            .unwrap()
            .push([bone, 0, 0, 0]);
    };
    // Vertices 0 and 1: identical in every stored byte.
    add(&mut mesh, [0.0, 0.0, 0.0], 0);
    add(&mut mesh, [0.0, 0.0, 0.0], 0);
    // Cluster A: 45 vertices on bones 0..45, faces through vertex 0.
    for i in 0..45 {
        add(&mut mesh, [10.0 + i as f32, 0.0, 0.0], i);
    }
    for i in 0..44usize {
        mesh.faces.push([0, (2 + i) as u16, (2 + i + 1) as u16]);
    }
    // Cluster B: 25 vertices on bones 45..70, faces through vertex 1.
    for i in 0..25 {
        add(&mut mesh, [100.0 + i as f32, 1.0, 0.0], 45 + i);
    }
    for i in 0..24usize {
        mesh.faces.push([1, (47 + i) as u16, (47 + i + 1) as u16]);
    }
    let source_faces = face_tuples(&mesh);
    let source_count = mesh.vertices.positions.len();
    let mut model = grid_model(mesh, bones);
    let parents: Vec<Option<usize>> = vec![None; bones];
    assert!(encode(&mut model, &parents).unwrap());

    // The duplicate key: position [0,0,0], uv [0,0].
    let is_dup = |mesh: &Mesh, index: usize| {
        mesh.vertices.positions[index] == [0.0, 0.0, 0.0]
            && mesh.vertices.uvs[0][index] == [0.0, 0.0]
    };
    let mut components_with_dups = 0;
    for component in &model.meshes {
        let dups: Vec<usize> = (0..component.vertices.positions.len())
            .filter(|&index| is_dup(component, index))
            .collect();
        if !dups.is_empty() {
            assert_eq!(dups.len(), 2);
            assert!(dups[0] < dups[1]);
            components_with_dups += 1;
        }
    }
    assert!(components_with_dups >= 2);

    decode(&mut model).unwrap();
    assert_eq!(model.meshes.len(), 1);
    let combined = &model.meshes[0];
    let dups: Vec<usize> = (0..combined.vertices.positions.len())
        .filter(|&index| is_dup(combined, index))
        .collect();
    assert_eq!(dups.len(), 2);
    assert_eq!(combined.vertices.positions.len(), source_count);
    assert_eq!(face_tuples(combined), source_faces);
}

// 8
#[test]
fn decode_errors() {
    // A component with a face index past its vertices.
    let mut model = grid_model(grid(4, 4, 0), 0);
    let mut second = model.meshes[0].clone();
    model.meshes[0]
        .extension_headers
        .push("Split-Mesh: 1".to_owned());
    second.extension_headers.push("Split-Mesh: 1".to_owned());
    second.faces[0][0] = u16::MAX;
    model.meshes.push(second);
    let before = model.clone();
    assert!(matches!(
        decode(&mut model),
        Err(ModelError::BadReference { what: "vertex", .. })
    ));
    // A failed decode leaves the model as it was.
    assert_eq!(model, before);

    // A split component carrying LOD levels is not one mesh with the
    // others: the group stays split, components and markers as they are.
    let mut model = grid_model(grid(4, 4, 0), 0);
    let mut second = model.meshes[0].clone();
    model.meshes[0]
        .extension_headers
        .push("Split-Mesh: 1".to_owned());
    second.lower_lods = vec![vec![[0, 1, 2]]];
    second.extension_headers.push("Split-Mesh: 1".to_owned());
    model.meshes.push(second);
    let before = model.clone();
    assert_eq!(decode(&mut model), Ok(()));
    assert_eq!(model, before);
}

#[test]
fn a_group_whose_components_are_not_one_mesh_stays_split() {
    // Material, bone weight width and vertex layout: a group whose
    // components disagree was never one mesh, so it stays split while
    // the other groups combine.
    let mutates: [fn(&mut Mesh); 3] = [
        |component| component.material = 1,
        |component| component.vertices.bone_weight_width = 3,
        |component| {
            component
                .vertices
                .uvs
                .push(vec![[0.0; 2]; component.vertices.uvs[0].len()])
        },
    ];
    for mutate in mutates {
        let mut model = grid_model(grid(4, 4, 2), 2);
        model.materials.push("Other".to_owned());
        let mut second = model.meshes[0].clone();
        model.meshes[0]
            .extension_headers
            .push("Split-Mesh: 1".to_owned());
        mutate(&mut second);
        second.extension_headers.push("Split-Mesh: 1".to_owned());
        model.meshes.push(second);
        // A second group whose components do combine.
        for _ in 0..2 {
            let mut component = model.meshes[0].clone();
            component.extension_headers = vec!["Split-Mesh: 2".to_owned()];
            model.meshes.push(component);
        }
        let components = model.meshes[..2].to_vec();
        decode(&mut model).expect("incompatible groups stay split");
        assert_eq!(model.meshes.len(), 3);
        assert_eq!(model.meshes[..2], components[..]);
        assert!(model.meshes[2].extension_headers.is_empty());
    }
}

// 9
#[test]
fn lods_refuse_to_split() {
    let mut mesh = grid(150, 150, 0);
    mesh.lower_lods = vec![vec![[0, 1, 2]]];
    let mut model = grid_model(mesh, 0);
    assert!(matches!(
        encode(&mut model, &[]),
        Err(ModelError::InvalidModel(message)) if message == "cannot split a mesh with LOD levels"
    ));
}

// 10
#[test]
fn file_round_trip() {
    let mut model = grid_model(grid(70, 20, 70), 70);
    let parents: Vec<Option<usize>> = vec![None; 70];
    assert!(encode(&mut model, &parents).unwrap());
    let count = model.meshes.len();
    let written = model.to_file().unwrap().write().unwrap();
    let mut again = Model::from_file(&PreFoxModel::read(&written).unwrap()).unwrap();
    assert_eq!(again.meshes.len(), count);
    for mesh in &again.meshes {
        assert_eq!(split_headers(mesh), ["Split-Mesh: 1".to_owned()]);
    }
    decode(&mut again).unwrap();
    assert_eq!(again.meshes.len(), 1);
}

// 11
#[test]
fn split_parents_must_index_bones() {
    // `sk_foot_l` is the first preferred base bone; with nothing under it
    // it hands over to its parent, so a parent past the end reaches that
    // read and must error, not panic. All-zero weights leave every bone's
    // items empty, so the walk reaches the bad parent.
    let mut mesh = grid(8, 8, 65);
    mesh.vertices.bone_weights.as_mut().unwrap().fill([0.0; 4]);
    let mut model = Model {
        flags: 0,
        bones: std::iter::once(bone("sk_foot_l"))
            .chain((1..66).map(|index| bone(&format!("bone{index}"))))
            .collect(),
        materials: vec!["Material".to_owned()],
        meshes: vec![mesh],
        extension_headers: Vec::new(),
        bounds: BoundingBox::of(&[]),
        lod: LodRecord::for_levels(0),
    };
    let mut parents: Vec<Option<usize>> = vec![None; 66];
    parents[0] = Some(66);
    assert_eq!(
        encode(&mut model, &parents),
        Err(ModelError::VertexMismatch(
            "split parents index out of range"
        ))
    );
}

// 12
#[test]
fn combine_limits_the_bone_group_to_u8() {
    // Two components of one split group weighted to disjoint halves of
    // 257 model bones: their combined group cannot fit a u8 slot.
    let component = |group: Vec<usize>| {
        let count = group.len();
        Mesh {
            name: None,
            extension_headers: vec!["Split-Mesh: 1".to_owned()],
            tags: Vec::new(),
            vertices: MeshVertices {
                positions: (0..count).map(|i| [i as f32, 0.0, 0.0]).collect(),
                normals: None,
                tangents: None,
                bitangents: None,
                colors: None,
                uvs: vec![vec![[0.0, 0.0]; count]],
                bone_indices: Some((0..count).map(|i| [i as u8, 0, 0, 0]).collect()),
                bone_weights: Some(vec![[1.0, 0.0, 0.0, 0.0]; count]),
                bone_weight_width: 4,
            },
            faces: vec![[0, 1, 2]],
            lower_lods: Vec::new(),
            bone_group: group,
            material: 0,
            bounds: BoundingBox::of(&[]),
            order: 0,
            editor_data: Vec::new(),
        }
    };
    let mut model = Model {
        flags: 0,
        bones: (0..257)
            .map(|index| bone(&format!("bone{index}")))
            .collect(),
        materials: vec!["Material".to_owned()],
        meshes: vec![
            component((0..129).collect()),
            component((129..257).collect()),
        ],
        extension_headers: Vec::new(),
        bounds: BoundingBox::of(&[]),
        lod: LodRecord::for_levels(0),
    };
    // Not one mesh: the group stays split.
    let before = model.clone();
    assert_eq!(decode(&mut model), Ok(()));
    assert_eq!(model, before);
}

// A foreign split group can hold coincident vertices that differ in one
// merge-key field — nontopological bytes, bone, or weight — and each
// difference must keep them apart in the combined mesh.
#[test]
fn merge_key_keeps_coincident_vertices_apart() {
    // Two `Split-Mesh: 1` components whose single vertices are identical
    // except where `mutate` changes the second.
    let welded_pair = |mutate: fn(&mut Mesh)| {
        let component = Mesh {
            name: None,
            extension_headers: vec!["Split-Mesh: 1".to_owned()],
            tags: Vec::new(),
            vertices: MeshVertices {
                positions: vec![[0.0; 3]],
                normals: Some(vec![[0.0, 0.0, 1.0]]),
                tangents: None,
                bitangents: None,
                colors: None,
                uvs: vec![vec![[0.0, 0.0]]],
                bone_indices: Some(vec![[0, 0, 0, 0]]),
                bone_weights: Some(vec![[1.0, 0.0, 0.0, 0.0]]),
                bone_weight_width: 4,
            },
            faces: Vec::new(),
            lower_lods: Vec::new(),
            bone_group: vec![0],
            material: 0,
            bounds: BoundingBox::of(&[]),
            order: 0,
            editor_data: Vec::new(),
        };
        let mut other = component.clone();
        mutate(&mut other);
        let mut model = grid_model(component, 2);
        model.meshes.push(other);
        model
    };
    let mutates: [fn(&mut Mesh); 3] = [
        // nontopological bytes (a uv)
        |mesh| mesh.vertices.uvs[0][0] = [1.0, 0.0],
        // the positive-weight bone
        |mesh| mesh.bone_group = vec![1],
        // the weight
        |mesh| mesh.vertices.bone_weights = Some(vec![[0.5, 0.0, 0.0, 0.0]]),
    ];
    for mutate in mutates {
        let mut model = welded_pair(mutate);
        decode(&mut model).unwrap();
        assert_eq!(model.meshes.len(), 1);
        assert_eq!(model.meshes[0].vertices.positions.len(), 2);
    }
}

// A combined mesh keeps the emission order when every referenced index
// fits u16: a loose loop stays right after its owner.
#[test]
fn combine_keeps_loose_loops_next_to_their_owners() {
    // [A0, A1, B, C] where A1 is an unreferenced loop of A0 (same
    // position, later uv); the face references A0, B, C.
    let component = Mesh {
        name: None,
        extension_headers: Vec::new(),
        tags: Vec::new(),
        vertices: MeshVertices {
            positions: vec![[0.0; 3], [0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: Some(vec![[0.0, 0.0, 1.0]; 4]),
            tangents: None,
            bitangents: None,
            colors: None,
            uvs: vec![vec![[0.0, 0.0], [1.0, 0.0], [0.0, 0.0], [0.0, 0.0]]],
            bone_indices: None,
            bone_weights: None,
            bone_weight_width: 4,
        },
        faces: vec![[0, 2, 3]],
        lower_lods: Vec::new(),
        bone_group: Vec::new(),
        material: 0,
        bounds: BoundingBox::of(&[]),
        order: 0,
        editor_data: Vec::new(),
    };
    let combined = combine::combine(&[component], &[0]).unwrap();
    // Emission order stands: A1 is still right after A0.
    assert_eq!(
        combined.vertices.positions,
        vec![[0.0; 3], [0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    // And the vertex decode still reads it as A0's loop.
    assert_eq!(
        crate::ops::vertex_enc::decode(&combined).unwrap(),
        vec![0, 0, 2, 3]
    );
}

// At exactly index 65535 every face index still fits u16, so the
// emission order stands; the reorder is only for `> u16::MAX`.
#[test]
fn combine_keeps_emission_order_at_index_65535() {
    let count = usize::from(u16::MAX) + 1;
    let component = Mesh {
        name: None,
        extension_headers: Vec::new(),
        tags: Vec::new(),
        vertices: MeshVertices {
            positions: (0..count).map(|index| [index as f32, 0.0, 0.0]).collect(),
            normals: Some(vec![[0.0, 0.0, 1.0]; count]),
            tangents: None,
            bitangents: None,
            colors: None,
            uvs: vec![vec![[0.0, 0.0]; count]],
            bone_indices: None,
            bone_weights: None,
            bone_weight_width: 4,
        },
        // Vertices 1 and 3..=65534 are loose; 65535 is the largest
        // referenced index and still fits u16.
        faces: vec![[0, 2, 65535]],
        lower_lods: Vec::new(),
        bone_group: Vec::new(),
        material: 0,
        bounds: BoundingBox::of(&[]),
        order: 0,
        editor_data: Vec::new(),
    };
    let combined = combine::combine(std::slice::from_ref(&component), &[0]).unwrap();
    assert_eq!(combined.vertices.positions, component.vertices.positions);
    assert_eq!(combined.faces, vec![[0, 2, 65535]]);
}

// A group whose combined mesh cannot be indexed in u16 stays split:
// its components and their `Split-Mesh` markers are kept as they are
// while another group still combines.
#[test]
fn a_group_over_65536_stays_split() {
    let component = |offset: usize, count: usize, faces: usize, key: &str| Mesh {
        name: None,
        extension_headers: vec![format!("{SPLIT_MESH_HEADER}: {key}")],
        tags: Vec::new(),
        vertices: MeshVertices {
            positions: (0..count)
                .map(|index| [(offset + index) as f32, 0.0, 0.0])
                .collect(),
            normals: Some(vec![[0.0, 0.0, 1.0]; count]),
            tangents: None,
            bitangents: None,
            colors: None,
            uvs: vec![vec![[0.0, 0.0]; count]],
            bone_indices: None,
            bone_weights: None,
            bone_weight_width: 4,
        },
        faces: (0..faces)
            .map(|face| {
                [
                    (3 * face) as u16,
                    (3 * face + 1) as u16,
                    (3 * face + 2) as u16,
                ]
            })
            .collect(),
        lower_lods: Vec::new(),
        bone_group: Vec::new(),
        material: 0,
        bounds: BoundingBox::of(&[]),
        order: 0,
        editor_data: Vec::new(),
    };
    // Group 1's two components reference 69999 distinct vertices
    // together; group 2's two components fit.
    let big_a = component(0, 40000, 13333, "1");
    let big_b = component(40000, 30000, 10000, "1");
    let small_a = component(70000, 4, 1, "2");
    let small_b = component(70004, 4, 1, "2");
    let mut model = grid_model(big_a.clone(), 0);
    model.meshes = vec![big_a.clone(), big_b.clone(), small_a, small_b];
    decode(&mut model).unwrap();
    assert_eq!(model.meshes.len(), 3);
    assert_eq!(model.meshes[0], big_a);
    assert_eq!(model.meshes[1], big_b);
    // The small group combined into one unmarked mesh of 8 vertices and
    // both faces.
    let combined = &model.meshes[2];
    assert!(combined.extension_headers.is_empty());
    assert_eq!(combined.vertices.positions.len(), 8);
    assert_eq!(combined.faces, vec![[0, 1, 2], [4, 5, 6]]);
}

// A face export's bone-marker mesh — no vertices or faces, a bone group
// advertising the whole skeleton — needs splitting by bone count but
// yields no components: it stays in place, unsplit, with no marker.
#[test]
fn a_mesh_with_no_split_items_stays_unsplit() {
    let mut marker = grid(1, 1, 0);
    marker.vertices.positions.clear();
    marker.vertices.normals.as_mut().unwrap().clear();
    marker.vertices.uvs[0].clear();
    marker.faces.clear();
    marker.bone_group = (0..65).collect();
    marker.vertices.bone_indices = Some(Vec::new());
    marker.vertices.bone_weights = Some(Vec::new());
    let mut model = grid_model(marker.clone(), 65);
    assert_eq!(encode(&mut model, &[None; 65]), Ok(false));
    assert_eq!(model.meshes, vec![marker]);
    decode(&mut model).unwrap();
}

// The group key recognizes only `Split-Mesh` headers.
#[test]
fn split_group_key_matches_only_split_headers() {
    assert_eq!(split_group_key("Split-Mesh: 1"), Some("1".to_owned()));
    assert_eq!(split_group_key(" split-mesh :  7 "), Some("7".to_owned()));
    assert!(split_group_key(crate::ops::vertex_enc::VERTEX_LOOP_PRESERVATION).is_none());
    assert!(split_group_key("Material: 1").is_none());
    assert!(split_group_key("no colon").is_none());
}

#[test]
fn bone_mapping_excludes_zero_weights() {
    let mut mesh = grid(1, 1, 8);
    mesh.vertices.bone_indices = Some(vec![[0, 5, 0, 0]]);
    mesh.vertices.bone_weights = Some(vec![[1.0, 0.0, 0.0, 0.0]]);
    assert_eq!(bone_mapping(&mesh, 0).unwrap(), vec![(0, 1.0)]);
}

// A lane past the bone group with no weight writes 0 (Konami files
// carry those); a positive weight past the group is an error.
#[test]
fn push_vertex_guards_past_group_slots() {
    let mut mesh = grid(1, 1, 1);
    mesh.vertices.bone_indices = Some(vec![[5, 0, 0, 0]]);
    mesh.vertices.bone_weights = Some(vec![[0.0, 0.0, 0.0, 0.0]]);
    let index_of: std::collections::HashMap<usize, u8> = [(0usize, 0u8)].into_iter().collect();
    let mut vertices = empty_like(&mesh.vertices, 0);
    push_vertex(&mut vertices, &mesh, 0, &index_of).unwrap();
    assert_eq!(vertices.bone_indices.as_ref().unwrap()[0], [0, 0, 0, 0]);

    mesh.vertices.bone_weights = Some(vec![[1.0, 0.0, 0.0, 0.0]]);
    let mut vertices = empty_like(&mesh.vertices, 0);
    assert_eq!(
        push_vertex(&mut vertices, &mesh, 0, &index_of),
        Err(ModelError::BadReference {
            what: "bone",
            offset: 5
        })
    );
}

// A normal-mapped part carries tangents, bitangents and colors: a split
// must keep one value per vertex in every component.
#[test]
fn split_keeps_every_optional_attribute_per_vertex() {
    let mut mesh = grid(65, 1, 65);
    let count = mesh.vertices.positions.len();
    mesh.vertices.tangents = Some(vec![[1.0, 0.0, 0.0]; count]);
    mesh.vertices.bitangents = Some(vec![[0.0, 1.0, 0.0]; count]);
    mesh.vertices.colors = Some(vec![[255, 0, 0, 255]; count]);
    let mut model = grid_model(mesh, 65);
    let parents = vec![None; 65];
    assert!(encode(&mut model, &parents).unwrap());

    for component in &model.meshes {
        let vertices = &component.vertices;
        let count = vertices.positions.len();
        assert_eq!(vertices.tangents.as_ref().unwrap().len(), count);
        assert_eq!(vertices.bitangents.as_ref().unwrap().len(), count);
        assert_eq!(vertices.colors.as_ref().unwrap().len(), count);
    }
}

#[test]
fn decode_combines_two_groups_in_place() {
    let mut a = grid(2, 2, 0);
    a.extension_headers.push("Split-Mesh: 1".to_owned());
    a.vertices.positions[0] = [10.0, 0.0, 0.0];
    let mut b = a.clone();
    b.vertices.positions[0] = [20.0, 0.0, 0.0];
    let mut c = grid(2, 2, 0);
    c.extension_headers.push("Split-Mesh: 2".to_owned());
    c.vertices.positions[0] = [30.0, 0.0, 0.0];
    let mut d = c.clone();
    d.vertices.positions[0] = [40.0, 0.0, 0.0];
    let plain = grid(2, 2, 0);
    let mut model = grid_model(plain.clone(), 0);
    model.meshes = vec![a, b, plain.clone(), c, d];
    decode(&mut model).unwrap();
    assert_eq!(model.meshes.len(), 3);
    // Each group sits where its first component sat.
    assert!(
        model.meshes[0]
            .vertices
            .positions
            .contains(&[10.0, 0.0, 0.0])
    );
    assert!(
        model.meshes[0]
            .vertices
            .positions
            .contains(&[20.0, 0.0, 0.0])
    );
    assert_eq!(model.meshes[1], plain);
    assert!(
        model.meshes[2]
            .vertices
            .positions
            .contains(&[30.0, 0.0, 0.0])
    );
    assert!(
        model.meshes[2]
            .vertices
            .positions
            .contains(&[40.0, 0.0, 0.0])
    );
}

// The preferred bone is the parent of 61 weighted bones: the parent walk
// must register every face and loose set under it, so the fragment pool
// is the whole subtree — sixty faces fit the bone soft limit, the rest
// and the loose set spill to a second component.
#[test]
fn split_mesh_registers_items_under_their_ancestors() {
    const CHILDREN: usize = BONE_LIMIT_SOFT + 1;
    let mut mesh = grid(1, 1, CHILDREN);
    mesh.vertices.positions.clear();
    mesh.vertices.normals.as_mut().unwrap().clear();
    mesh.vertices.uvs[0].clear();
    mesh.vertices.bone_indices.as_mut().unwrap().clear();
    mesh.vertices.bone_weights.as_mut().unwrap().clear();
    mesh.faces.clear();
    for child in 0..CHILDREN {
        for _ in 0..3 {
            mesh.vertices.positions.push([child as f32, 0.0, 0.0]);
            mesh.vertices
                .normals
                .as_mut()
                .unwrap()
                .push([0.0, 0.0, 1.0]);
            mesh.vertices.uvs[0].push([0.0, 0.0]);
            mesh.vertices
                .bone_indices
                .as_mut()
                .unwrap()
                .push([child as u8, 0, 0, 0]);
            mesh.vertices
                .bone_weights
                .as_mut()
                .unwrap()
                .push([1.0, 0.0, 0.0, 0.0]);
        }
        mesh.faces
            .push([3 * child as u16, 3 * child as u16 + 1, 3 * child as u16 + 2]);
    }
    // One loose vertex weighted to the last child.
    mesh.vertices.positions.push([999.0, 9.0, 9.0]);
    mesh.vertices
        .normals
        .as_mut()
        .unwrap()
        .push([0.0, 0.0, 1.0]);
    mesh.vertices.uvs[0].push([9.0, 9.0]);
    mesh.vertices
        .bone_indices
        .as_mut()
        .unwrap()
        .push([CHILDREN as u8 - 1, 0, 0, 0]);
    mesh.vertices
        .bone_weights
        .as_mut()
        .unwrap()
        .push([1.0, 0.0, 0.0, 0.0]);
    mesh.bone_group = (1..CHILDREN + 1).collect();
    let model = {
        let mut model = grid_model(grid(1, 1, 0), CHILDREN + 1);
        model.bones[0].name = "sk_foot_l".to_owned();
        model
    };
    let mut parents = vec![None; CHILDREN + 1];
    for parent in parents.iter_mut().skip(1) {
        *parent = Some(0);
    }
    let components = build::split_mesh(&model, &mesh, &parents).unwrap();
    assert_eq!(components.len(), 2);
    assert_eq!(components[0].faces.len(), BONE_LIMIT_SOFT);
    assert_eq!(
        components.iter().map(|c| c.faces.len()).sum::<usize>(),
        CHILDREN
    );
}

// A chain leaf -> mid1 -> mid2 -> the preferred bone: faces and loose
// sets must register at every level up the chain, so the preferred
// bone's fragment pool holds the whole subtree. Registering only three
// levels — a vertex short — leaves the preferred bone out.
#[test]
fn split_mesh_registers_items_under_deep_ancestors() {
    const LEAVES: usize = BONE_LIMIT_SOFT + 1;
    const MID2: usize = 1;
    const MID1: usize = 2;
    let mut mesh = grid(1, 1, LEAVES);
    mesh.vertices.positions.clear();
    mesh.vertices.normals.as_mut().unwrap().clear();
    mesh.vertices.uvs[0].clear();
    mesh.vertices.bone_indices.as_mut().unwrap().clear();
    mesh.vertices.bone_weights.as_mut().unwrap().clear();
    mesh.faces.clear();
    for leaf in 0..LEAVES {
        // Leaf 0's face projects past every other, so it sorts first and
        // its bone is selected into the first component.
        let x = if leaf == 0 { 10_000.0 } else { leaf as f32 };
        for _ in 0..3 {
            mesh.vertices.positions.push([x, 0.0, 0.0]);
            mesh.vertices
                .normals
                .as_mut()
                .unwrap()
                .push([0.0, 0.0, 1.0]);
            mesh.vertices.uvs[0].push([0.0, 0.0]);
            mesh.vertices
                .bone_indices
                .as_mut()
                .unwrap()
                .push([leaf as u8, 0, 0, 0]);
            mesh.vertices
                .bone_weights
                .as_mut()
                .unwrap()
                .push([1.0, 0.0, 0.0, 0.0]);
        }
        mesh.faces
            .push([3 * leaf as u16, 3 * leaf as u16 + 1, 3 * leaf as u16 + 2]);
    }
    // One loose vertex weighted to leaf 0's bone.
    mesh.vertices.positions.push([999.0, 9.0, 9.0]);
    mesh.vertices
        .normals
        .as_mut()
        .unwrap()
        .push([0.0, 0.0, 1.0]);
    mesh.vertices.uvs[0].push([9.0, 9.0]);
    mesh.vertices
        .bone_indices
        .as_mut()
        .unwrap()
        .push([0, 0, 0, 0]);
    mesh.vertices
        .bone_weights
        .as_mut()
        .unwrap()
        .push([1.0, 0.0, 0.0, 0.0]);
    mesh.bone_group = (3..LEAVES + 3).collect();
    let model = {
        let mut model = grid_model(grid(1, 1, 0), LEAVES + 3);
        model.bones[0].name = "sk_foot_l".to_owned();
        model
    };
    let mut parents = vec![None; LEAVES + 3];
    parents[MID2] = Some(0);
    parents[MID1] = Some(MID2);
    for parent in parents.iter_mut().skip(3) {
        *parent = Some(MID1);
    }
    let components = build::split_mesh(&model, &mesh, &parents).unwrap();
    assert_eq!(components.len(), 2);
    assert_eq!(components[0].faces.len(), BONE_LIMIT_SOFT);
    // The loose vertex travels in the first component: leaf 0's face
    // selected its bone already.
    assert!(
        components[0]
            .vertices
            .positions
            .contains(&[999.0, 9.0, 9.0])
    );
}

// Coincident vertices differing only in a zero-weight bone-index lane
// share one equipresent set: the split key is the positive-weight bone
// mapping, not the raw index lanes.
#[test]
fn coincident_vertices_share_the_mapping_key() {
    const FILLERS: usize = 65_532;
    let mut vertices = MeshVertices {
        positions: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        normals: Some(vec![[0.0, 0.0, 1.0]; 3]),
        tangents: None,
        bitangents: None,
        colors: None,
        uvs: vec![vec![[0.0; 2]; 3]],
        bone_indices: Some(vec![[0, 0, 0, 0]; 3]),
        bone_weights: Some(vec![[1.0, 0.0, 0.0, 0.0]; 3]),
        bone_weight_width: 4,
    };
    // Vertex 3: a loose copy of vertex 0 whose only difference is a
    // zero-weight lane (index 7, weight 0).
    vertices.positions.push([0.0; 3]);
    vertices.normals.as_mut().unwrap().push([0.0, 0.0, 1.0]);
    vertices.uvs[0].push([0.0, 0.0]);
    vertices.bone_indices.as_mut().unwrap().push([0, 7, 0, 0]);
    vertices
        .bone_weights
        .as_mut()
        .unwrap()
        .push([1.0, 0.0, 0.0, 0.0]);
    // Loose vertices at strictly higher projections fill the first
    // component, so the copy is what spills over.
    for filler in 0..FILLERS {
        vertices.positions.push([filler as f32 + 2.0, 0.0, 0.0]);
        vertices.normals.as_mut().unwrap().push([0.0, 0.0, 1.0]);
        vertices.uvs[0].push([0.0, 0.0]);
        vertices.bone_indices.as_mut().unwrap().push([0, 0, 0, 0]);
        vertices
            .bone_weights
            .as_mut()
            .unwrap()
            .push([1.0, 0.0, 0.0, 0.0]);
    }
    let mut mesh = grid(1, 1, 40);
    mesh.vertices = vertices;
    mesh.faces = vec![[0, 1, 2]];
    mesh.bone_group = (0..40).collect();
    let faces_before = face_tuples(&mesh);
    let count_before = mesh.vertices.positions.len();
    let mut model = grid_model(mesh, 40);
    assert!(encode(&mut model, &[None; 40]).unwrap());

    // One component holds both copies of position [0, 0, 0].
    let holders: Vec<usize> = model
        .meshes
        .iter()
        .enumerate()
        .filter(|(_, mesh)| mesh.vertices.positions.contains(&[0.0; 3]))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(holders.len(), 1);
    assert_eq!(
        model.meshes[holders[0]]
            .vertices
            .positions
            .iter()
            .filter(|position| **position == [0.0; 3])
            .count(),
        2
    );

    // Nothing welds back apart: the combined mesh is the source mesh.
    decode(&mut model).unwrap();
    assert_eq!(model.meshes.len(), 1);
    assert_eq!(model.meshes[0].vertices.positions.len(), count_before);
    assert_eq!(face_tuples(&model.meshes[0]), faces_before);
}

// 13
#[test]
fn two_sources_number_from_one() {
    let mut model = Model {
        flags: 0,
        bones: (0..70).map(|index| bone(&format!("bone{index}"))).collect(),
        materials: vec!["Material".to_owned()],
        meshes: vec![grid(70, 20, 70), grid(8, 8, 2), grid(70, 20, 70)],
        extension_headers: Vec::new(),
        bounds: BoundingBox::of(&[]),
        lod: LodRecord::for_levels(0),
    };
    let parents: Vec<Option<usize>> = vec![None; 70];
    assert!(encode(&mut model, &parents).unwrap());

    // The fitting mesh sits between the two split groups.
    let fitting = model
        .meshes
        .iter()
        .position(|mesh| mesh.vertices.positions.len() == 8 * 8)
        .unwrap();
    for mesh in &model.meshes[..fitting] {
        assert_eq!(split_headers(mesh), ["Split-Mesh: 1".to_owned()]);
    }
    assert!(model.meshes[..fitting].len() > 1);
    for mesh in &model.meshes[fitting + 1..] {
        assert_eq!(split_headers(mesh), ["Split-Mesh: 2".to_owned()]);
    }
    assert!(model.meshes.len() > fitting + 2);
}
