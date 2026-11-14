use std::collections::{BTreeSet, HashSet};

use super::*;
use crate::format::FmdlFile;
use crate::model::{Bone, BoundingBox, Extensions, MaterialInstance};

const HIGHNECK: &[u8] = include_bytes!("../../../tests/fixtures/konami_highneck.fmdl");
const MOUTH: &[u8] = include_bytes!("../../../tests/fixtures/konami_mouth.fmdl");
const AU_LOW: &[u8] = include_bytes!("../../../tests/fixtures/konami_au_Low_parts.fmdl");
const ORAL: &[u8] = include_bytes!("../../../tests/fixtures/addon_oral.fmdl");
const PLACEHOLDER: &[u8] = include_bytes!("../../../tests/fixtures/addon_placeholder.fmdl");
const FIXTURES: [&[u8]; 5] = [HIGHNECK, MOUTH, AU_LOW, ORAL, PLACEHOLDER];

fn fixture_model(bytes: &[u8]) -> Model {
    Model::from_file(&FmdlFile::read(bytes).unwrap()).unwrap()
}

fn bone(name: &str, parent: Option<usize>) -> Bone {
    Bone {
        name: name.to_owned(),
        parent,
        bounding_box: BoundingBox {
            max: [0.0; 4],
            min: [0.0; 4],
        },
        local_position: [0.0; 4],
        world_position: [0.0; 4],
    }
}

fn material() -> MaterialInstance {
    MaterialInstance {
        name: "Material".to_owned(),
        shader: "fox3ddf_blin".to_owned(),
        technique: "fox3DDF_Blin".to_owned(),
        textures: Vec::new(),
        parameters: Vec::new(),
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
        colors: None,
        uvs: vec![Vec::new()],
        uv_high_precision: vec![true],
        bone_weights: (bones > 0).then(Vec::new),
        bone_indices: (bones > 0).then(Vec::new),
    };
    for y in 0..height {
        for x in 0..width {
            vertices.positions.push([x as f32, y as f32, 0.0]);
            vertices
                .normals
                .as_mut()
                .unwrap()
                .push([0.0, 0.0, 1.0, 0.0]);
            vertices.uvs[0].push([x as f32 / width as f32, y as f32 / height as f32]);
            if bones > 0 {
                let bone = (x * bones / width) as u8;
                vertices.bone_weights.as_mut().unwrap().push([255, 0, 0, 0]);
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
    Mesh {
        vertices,
        faces,
        bone_group: (0..bones).collect(),
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    }
}

/// One mesh in one group in a model with `bones` chain-parented bones.
fn grid_model(mesh: Mesh, bones: usize) -> Model {
    Model {
        bones: (0..bones)
            .map(|index| bone(&format!("bone{index}"), index.checked_sub(1)))
            .collect(),
        materials: vec![material()],
        meshes: vec![mesh],
        mesh_groups: vec![MeshGroup {
            name: "group".to_owned(),
            parent: None,
            meshes: vec![0],
            bounding_box: Some(BoundingBox {
                max: [10.0, 10.0, 10.0, 1.0],
                min: [-10.0, -10.0, -10.0, 1.0],
            }),
            visible: true,
            split_mesh_group: false,
        }],
        extensions: Extensions::default(),
        bone_matrices: Some(Vec::new()),
    }
}

/// A vertex resolved to the values a face sees: position, normal, uv
/// maps and the model-level bone mapping (bits, so tuples order).
type VertexTuple = ([u32; 3], Option<[u32; 4]>, Vec<[u32; 2]>, BoneMapping);

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
        bone_mapping(mesh, index).unwrap(),
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
    mesh.bone_group
        .iter()
        .copied()
        .filter(|bone| used.contains(bone))
        .collect()
}

// S1
#[test]
fn needs_splitting_limits() {
    for bytes in FIXTURES {
        let model = fixture_model(bytes);
        for mesh in &model.meshes {
            assert!(!needs_splitting(mesh));
        }
    }
    assert!(needs_splitting(&grid(60, 20, 40)));
    assert!(needs_splitting(&grid(300, 300, 0)));
    assert!(needs_splitting(&grid(150, 150, 0)));
}

// S2
#[test]
fn split_bones() {
    let source = grid(60, 20, 40);
    let source_faces = face_tuples(&source);
    let mut model = grid_model(source, 40);
    assert!(encode(&mut model, None).unwrap());

    components_under_limits(&model);
    assert!(model.meshes.len() > 1);
    let mut union: Vec<[VertexTuple; 3]> = model.meshes.iter().flat_map(face_tuples).collect();
    union.sort();
    assert_eq!(union, source_faces);
    assert_eq!(
        model
            .meshes
            .iter()
            .map(|mesh| mesh.faces.len())
            .sum::<usize>(),
        source_faces.len()
    );

    assert_eq!(model.mesh_groups.len(), 2);
    assert_eq!(model.mesh_groups[0].name, "group");
    assert!(model.mesh_groups[0].meshes.is_empty());
    let split = &model.mesh_groups[1];
    assert_eq!(split.name, "split-mesh");
    assert_eq!(split.parent, Some(0));
    assert!(split.split_mesh_group);
    assert_eq!(split.bounding_box, model.mesh_groups[0].bounding_box);
    assert_eq!(split.visible, model.mesh_groups[0].visible);
    assert_eq!(
        split.meshes,
        (0..model.meshes.len()).collect::<Vec<usize>>()
    );
    assert!(model.extensions.mesh_splitting);
}

// S3
#[test]
fn split_vertices() {
    let source = grid(300, 300, 0);
    let source_faces = face_tuples(&source);
    let mut model = grid_model(source, 0);
    assert!(encode(&mut model, None).unwrap());

    components_under_limits(&model);
    assert!(model.meshes.len() > 1);
    let mut union: Vec<[VertexTuple; 3]> = model.meshes.iter().flat_map(face_tuples).collect();
    union.sort();
    assert_eq!(union, source_faces);
    assert_eq!(
        model
            .meshes
            .iter()
            .map(|mesh| mesh.faces.len())
            .sum::<usize>(),
        source_faces.len()
    );
}

// A face written twice in the source must come back twice — the multiset
// comparison in `check_round_trip` would not see a collapsed copy.
#[test]
fn a_repeated_triangle_survives_the_round_trip() {
    let mut source = grid(60, 20, 40);
    source.faces.push(source.faces[0]);
    check_round_trip(grid_model(source.clone(), 40), source);
}

// A referenced vertex can land past index 65535 in the merged emission
// order even though every source face index fits u16: the first
// component fills on the 20000-face cap and tops up on loose vertices,
// so the one remaining face lands in the second component behind the
// loose sets — and `combine` must reorder rather than wrap.
#[test]
fn a_referenced_vertex_past_u16_reorders_the_combine() {
    const LOOSE_BEFORE: usize = 2537; // deferred to component 2, emit first
    const FILLERS: usize = 20000; // the per-component face cap
    const LOOSE_FILL: usize = 3000; // taken into component 1's loose fill
    let mut vertices = MeshVertices {
        positions: Vec::new(),
        normals: Some(Vec::new()),
        tangents: None,
        colors: None,
        uvs: vec![Vec::new()],
        uv_high_precision: vec![true],
        bone_weights: None,
        bone_indices: None,
    };
    let push = |vertices: &mut MeshVertices, position: [f32; 3]| {
        vertices.positions.push(position);
        vertices
            .normals
            .as_mut()
            .unwrap()
            .push([0.0, 0.0, 1.0, 0.0]);
        vertices.uvs[0].push([0.0, 0.0]);
    };
    // Component-2 loose vertices first: projection 0, the lowest scores.
    for index in 0..LOOSE_BEFORE {
        push(&mut vertices, [0.0, index as f32 + 1.0, 0.0]);
    }
    // Filler faces: three distinct vertices each, all scoring above 0.
    for index in 0..FILLERS * 3 {
        push(&mut vertices, [index as f32 + 1.0, 0.0, 0.0]);
    }
    let target = vertices.positions.len();
    push(&mut vertices, [0.0, 0.0, 0.0]);
    push(&mut vertices, [0.0, 0.0, 1.0]);
    push(&mut vertices, [0.0, 1.0, 0.0]);
    // Component-1 loose vertices: the highest scores of the loose sets.
    for index in 0..LOOSE_FILL {
        push(&mut vertices, [index as f32 + 60001.0, 0.0, 0.0]);
    }
    let mut faces: Vec<[u16; 3]> = (0..FILLERS)
        .map(|face| {
            let first = LOOSE_BEFORE + 3 * face;
            [first as u16, (first + 1) as u16, (first + 2) as u16]
        })
        .collect();
    faces.push([target as u16, (target + 1) as u16, (target + 2) as u16]);
    let mesh = Mesh {
        vertices,
        faces,
        bone_group: Vec::new(),
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    check_round_trip(grid_model(mesh.clone(), 0), mesh);
}

// S4
fn check_round_trip(mut model: Model, source: Mesh) {
    let source_faces = face_tuples(&source);
    let source_bones = used_bones(&source);
    assert!(encode(&mut model, None).unwrap());
    decode(&mut model).unwrap();

    assert_eq!(model.meshes.len(), 1);
    let combined = &model.meshes[0];
    assert_eq!(face_tuples(combined), source_faces);
    assert_eq!(combined.faces.len(), source.faces.len());
    assert_eq!(combined.material, source.material);
    assert_eq!(combined.alpha_flags, source.alpha_flags);
    assert_eq!(combined.shadow_flags, source.shadow_flags);
    assert_eq!(combined.has_antiblur_meshes, source.has_antiblur_meshes);
    assert_eq!(combined.is_antiblur_mesh, source.is_antiblur_mesh);
    assert_eq!(combined.bone_group, source_bones);

    assert_eq!(model.mesh_groups.len(), 1);
    assert_eq!(model.mesh_groups[0].name, "group");
    assert_eq!(model.mesh_groups[0].meshes, vec![0]);
    assert!(!model.extensions.mesh_splitting);
}

#[test]
fn round_trip_bones() {
    let source = grid(60, 20, 40);
    let model = grid_model(source.clone(), 40);
    check_round_trip(model, source);
}

#[test]
fn round_trip_vertices() {
    let source = grid(300, 300, 0);
    let model = grid_model(source.clone(), 0);
    check_round_trip(model, source);
}

// S5: two byte-identical vertices, each referenced by faces that must
// land in different components (disjoint bone sets over the limit).
#[test]
fn duplicate_vertices() {
    let bones = 40;
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
            .push([0.0, 0.0, 1.0, 0.0]);
        mesh.vertices.uvs[0].push([position[0] / 200.0, position[1]]);
        mesh.vertices
            .bone_weights
            .as_mut()
            .unwrap()
            .push([255, 0, 0, 0]);
        mesh.vertices
            .bone_indices
            .as_mut()
            .unwrap()
            .push([bone, 0, 0, 0]);
    };
    // Vertices 0 and 1: identical in every stored byte.
    add(&mut mesh, [0.0, 0.0, 0.0], 0);
    add(&mut mesh, [0.0, 0.0, 0.0], 0);
    // Cluster A: 16 vertices on bones 0..16, faces through vertex 0.
    for i in 0..16 {
        add(&mut mesh, [10.0 + i as f32, 0.0, 0.0], i);
    }
    for i in 0..15usize {
        mesh.faces.push([0, (2 + i) as u16, (2 + i + 1) as u16]);
    }
    // Cluster B: 20 vertices on bones 20..40, faces through vertex 1.
    for i in 0..20 {
        add(&mut mesh, [100.0 + i as f32, 1.0, 0.0], 20 + i);
    }
    for i in 0..19usize {
        mesh.faces.push([1, (18 + i) as u16, (18 + i + 1) as u16]);
    }
    let source_faces = face_tuples(&mesh);
    let source_count = mesh.vertices.positions.len();
    let mut model = grid_model(mesh, bones);
    assert!(encode(&mut model, None).unwrap());

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

// S6: a small mesh alongside an oversized one.
#[test]
fn untouched_mesh() {
    let small = grid(8, 8, 2);
    let source = grid(60, 20, 40);
    let source_faces = face_tuples(&source);
    let source_bones = used_bones(&source);
    let mut model = Model {
        bones: (0..40usize)
            .map(|index| bone(&format!("bone{index}"), index.checked_sub(1)))
            .collect(),
        materials: vec![material()],
        meshes: vec![small.clone(), source],
        mesh_groups: vec![MeshGroup {
            name: "group".to_owned(),
            parent: None,
            meshes: vec![0, 1],
            bounding_box: Some(BoundingBox {
                max: [10.0, 10.0, 10.0, 1.0],
                min: [-10.0, -10.0, -10.0, 1.0],
            }),
            visible: true,
            split_mesh_group: false,
        }],
        extensions: Extensions::default(),
        bone_matrices: None,
    };

    assert!(encode(&mut model, None).unwrap());
    assert_eq!(model.meshes[0], small);
    assert_eq!(model.mesh_groups[0].meshes, vec![0]);
    assert!(model.mesh_groups[1].split_mesh_group);

    decode(&mut model).unwrap();
    assert_eq!(model.meshes.len(), 2);
    assert_eq!(model.meshes[0], small);
    let combined = &model.meshes[1];
    // Vertex order is not restored by construction; compare the face
    // tuple set, vertex count, and every non-order field.
    assert_eq!(combined.vertices.positions.len(), 60 * 20);
    assert_eq!(face_tuples(combined), source_faces);
    assert_eq!(combined.bone_group, source_bones);
    assert_eq!(combined.material, 0);
    assert_eq!(combined.alpha_flags, 0);
    assert_eq!(combined.shadow_flags, 0);
    assert_eq!(model.mesh_groups.len(), 1);
    assert_eq!(model.mesh_groups[0].meshes, vec![0, 1]);
    assert!(!model.extensions.mesh_splitting);
}

// A face export's bone-marker mesh — no vertices or faces, its bone
// group advertising the skeleton — needs splitting by bone count but
// yields no components: it stays in place, unsplit, with no container
// and no marker.
#[test]
fn a_mesh_with_no_split_items_stays_unsplit() {
    let mut marker = grid(1, 1, 0);
    marker.vertices.positions.clear();
    marker.vertices.normals.as_mut().unwrap().clear();
    marker.vertices.uvs[0].clear();
    marker.faces.clear();
    marker.bone_group = (0..33).collect();
    marker.vertices.bone_indices = Some(Vec::new());
    marker.vertices.bone_weights = Some(Vec::new());
    let mut model = grid_model(marker.clone(), 33);
    assert_eq!(encode(&mut model, None), Ok(false));
    assert_eq!(model.meshes, vec![marker]);
    decode(&mut model).unwrap();
}

// S7
#[test]
fn file_round_trip() {
    let mut model = grid_model(grid(60, 20, 40), 40);
    assert!(encode(&mut model, None).unwrap());
    let written = model.to_file().unwrap();
    let again = Model::from_file(&FmdlFile::read(&written.write()).unwrap()).unwrap();
    assert_eq!(again, model);
}

// S8
#[test]
fn effective_parents_chain() {
    let model = Model {
        bones: vec![
            bone("dsk_hip", None),
            bone("sk_belly", Some(0)),
            bone("sk_chest", Some(1)),
        ],
        materials: Vec::new(),
        meshes: Vec::new(),
        mesh_groups: Vec::new(),
        extensions: Extensions::default(),
        bone_matrices: None,
    };
    let parents = effective_parents(&model);
    assert_eq!(parents[2], None); // sk_chest is the root
    assert_eq!(parents[1], Some(2)); // sk_belly's parent is the chest
    assert_eq!(parents[0], Some(1)); // dsk_hip's parent is the belly

    let highneck = fixture_model(HIGHNECK);
    let parents = effective_parents(&highneck);
    let expected: Vec<Option<usize>> = highneck.bones.iter().map(|bone| bone.parent).collect();
    assert_eq!(parents, expected);
}

// A container whose combined mesh cannot be indexed in u16 stays split:
// the group, its flag and its components are kept as they are while
// another container still combines.
#[test]
fn a_group_over_65536_stays_split() {
    let component = |offset: usize, count: usize, faces: usize| Mesh {
        vertices: MeshVertices {
            positions: (0..count)
                .map(|index| [(offset + index) as f32, 0.0, 0.0])
                .collect(),
            normals: Some(vec![[0.0, 0.0, 1.0, 0.0]; count]),
            tangents: None,
            colors: None,
            uvs: vec![vec![[0.0, 0.0]; count]],
            uv_high_precision: vec![true],
            bone_weights: None,
            bone_indices: None,
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
        bone_group: Vec::new(),
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    // Container 1's two components reference 69999 distinct vertices
    // together; container 2's two fit.
    let big_a = component(0, 40000, 13333);
    let big_b = component(40000, 30000, 10000);
    let small_a = component(70000, 4, 1);
    let small_b = component(70004, 4, 1);
    let container = |name: &str, meshes: Vec<usize>| MeshGroup {
        name: name.to_owned(),
        parent: Some(0),
        meshes,
        bounding_box: None,
        visible: true,
        split_mesh_group: true,
    };
    let mut model = Model {
        bones: Vec::new(),
        materials: vec![material()],
        meshes: vec![big_a.clone(), big_b.clone(), small_a, small_b],
        mesh_groups: vec![
            MeshGroup {
                name: "root".to_owned(),
                parent: None,
                meshes: Vec::new(),
                bounding_box: None,
                visible: true,
                split_mesh_group: false,
            },
            container("split-mesh", vec![0, 1]),
            container("split-mesh.001", vec![2, 3]),
        ],
        extensions: Extensions::default(),
        bone_matrices: None,
    };
    model.extensions.mesh_splitting = true;
    decode(&mut model).unwrap();
    // The oversized container kept its flag and its components; the
    // fitting one collapsed into the root group.
    assert_eq!(model.meshes.len(), 3);
    assert_eq!(model.meshes[0], big_a);
    assert_eq!(model.meshes[1], big_b);
    assert_eq!(model.mesh_groups.len(), 2);
    assert_eq!(model.mesh_groups[0].meshes, vec![2]);
    assert!(model.mesh_groups[1].split_mesh_group);
    assert_eq!(model.mesh_groups[1].meshes, vec![0, 1]);
    assert!(model.extensions.mesh_splitting);
}

// S9
#[test]
fn decode_errors() {
    let mut model = grid_model(grid(4, 4, 0), 0);
    model.mesh_groups.push(MeshGroup {
        name: "split-mesh".to_owned(),
        parent: None,
        meshes: vec![0],
        bounding_box: None,
        visible: true,
        split_mesh_group: true,
    });
    model.mesh_groups[0].meshes.clear();
    assert!(matches!(
        decode(&mut model),
        Err(FmdlError::BadMeshGroupAssignment(_))
    ));

    let mut model = grid_model(grid(4, 4, 0), 0);
    model.mesh_groups.push(MeshGroup {
        name: "split-mesh".to_owned(),
        parent: Some(0),
        meshes: vec![0],
        bounding_box: None,
        visible: true,
        split_mesh_group: true,
    });
    model.mesh_groups[0].meshes.clear();
    model.mesh_groups.push(MeshGroup {
        name: "child".to_owned(),
        parent: Some(1),
        meshes: Vec::new(),
        bounding_box: None,
        visible: true,
        split_mesh_group: false,
    });
    assert!(matches!(
        decode(&mut model),
        Err(FmdlError::BadMeshGroupAssignment(_))
    ));
}

// S10
#[test]
fn encode_face_index_out_of_range_errors() {
    // 16 vertices; face index 16 does not exist.
    let mut mesh = grid(4, 4, 0);
    mesh.faces = vec![[0, 0, 16]; FACE_LIMIT_HARD + 1];
    let mut model = grid_model(mesh, 0);
    assert!(matches!(
        encode(&mut model, None),
        Err(FmdlError::BadReference {
            what: "face vertex",
            index: 16
        })
    ));
}

// S11
#[test]
fn decode_face_index_out_of_range_errors() {
    let mut mesh = grid(4, 4, 0); // 16 vertices
    mesh.faces = vec![[0, 0, 16]];
    let mut model = grid_model(mesh, 0);
    model.mesh_groups[0].meshes.clear();
    model.mesh_groups.push(MeshGroup {
        name: "split-mesh".to_owned(),
        parent: Some(0),
        meshes: vec![0],
        bounding_box: None,
        visible: true,
        split_mesh_group: true,
    });
    assert!(matches!(
        decode(&mut model),
        Err(FmdlError::BadReference {
            what: "face vertex",
            index: 16
        })
    ));
}

// S12
#[test]
fn decode_mesh_index_out_of_range_errors() {
    // A split group naming a mesh that does not exist.
    let mut model = grid_model(grid(4, 4, 0), 0);
    model.mesh_groups.push(MeshGroup {
        name: "split-mesh".to_owned(),
        parent: Some(0),
        meshes: vec![9],
        bounding_box: None,
        visible: true,
        split_mesh_group: true,
    });
    assert!(matches!(
        decode(&mut model),
        Err(FmdlError::BadReference {
            what: "mesh",
            index: 9
        })
    ));
    // A normal group naming a mesh that does not exist.
    let mut model = grid_model(grid(4, 4, 0), 0);
    model.mesh_groups[0].meshes = vec![9];
    assert!(matches!(
        decode(&mut model),
        Err(FmdlError::BadReference {
            what: "mesh",
            index: 9
        })
    ));
}

// S14
#[test]
fn needs_splitting_boundaries() {
    let mut mesh = grid(4, 4, 0);
    mesh.bone_group = vec![0; BONE_LIMIT_HARD];
    assert!(!needs_splitting(&mesh));
    mesh.bone_group = vec![0; BONE_LIMIT_HARD + 1];
    assert!(needs_splitting(&mesh));

    let mut mesh = grid(4, 4, 0);
    mesh.vertices.positions = vec![[0.0; 3]; VERTEX_LIMIT_HARD];
    assert!(!needs_splitting(&mesh));
    mesh.vertices.positions.push([0.0; 3]);
    assert!(needs_splitting(&mesh));

    let mut mesh = grid(4, 4, 0);
    mesh.faces = vec![[0, 1, 2]; FACE_LIMIT_HARD];
    assert!(!needs_splitting(&mesh));
    mesh.faces.push([0, 1, 2]);
    assert!(needs_splitting(&mesh));
}

#[test]
fn encode_rejects_a_wrong_length_parents() {
    // The model has no bones; one entry is one too many.
    let mut model = grid_model(grid(4, 4, 0), 0);
    assert!(matches!(
        encode(&mut model, Some(&[None])),
        Err(FmdlError::VertexMismatch(
            "parents length does not match bone count"
        ))
    ));
}

// S15: a weighted slot past the component's group is a bad reference;
// the same slot at zero weight writes 0.
#[test]
fn zero_weight_slot_past_the_bone_group() {
    let mut mesh = grid(60, 20, 40);
    mesh.vertices.bone_indices.as_mut().unwrap()[0] = [40, 0, 0, 0];
    mesh.vertices.bone_weights.as_mut().unwrap()[0] = [0, 0, 0, 0];
    // The zero-weight slot does not appear in the vertex's bone mapping.
    assert!(bone_mapping(&mesh, 0).unwrap().is_empty());
    let mut model = grid_model(mesh, 40);
    assert!(encode(&mut model, None).unwrap());
    // The component holding vertex 0's position wrote slot 0.
    let (component, index) = model
        .meshes
        .iter()
        .enumerate()
        .flat_map(|(m, mesh)| (0..mesh.vertices.positions.len()).map(move |v| (m, v)))
        .find(|&(m, v)| model.meshes[m].vertices.positions[v] == [0.0, 0.0, 0.0])
        .unwrap();
    assert_eq!(
        model.meshes[component]
            .vertices
            .bone_indices
            .as_ref()
            .unwrap()[index][0],
        0
    );

    // With weight the same slot is a bad reference.
    let mut mesh = grid(60, 20, 40);
    mesh.vertices.bone_indices.as_mut().unwrap()[0] = [40, 0, 0, 0];
    assert!(matches!(
        bone_mapping(&mesh, 0),
        Err(FmdlError::BadReference {
            what: "bone",
            index: 40
        })
    ));
    let mut model = grid_model(mesh, 40);
    assert!(matches!(
        encode(&mut model, None),
        Err(FmdlError::BadReference {
            what: "bone",
            index: 40
        })
    ));
}

// S16: a combined mesh can name more bones than the u8 slot field holds.
#[test]
fn combine_past_256_bones_overflows() {
    let skinned_vertex = |slot: u8| {
        let mut vertices = MeshVertices::default();
        vertices.positions.push([0.0; 3]);
        vertices.bone_weights = Some(vec![[255, 0, 0, 0]]);
        vertices.bone_indices = Some(vec![[slot, 0, 0, 0]]);
        vertices
    };
    let component = |bones: std::ops::Range<usize>, vertices| Mesh {
        vertices,
        faces: Vec::new(),
        bone_group: bones.collect(),
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    // Component A uses model bones 0..256; B uses bone 256. Their union
    // needs 257 slots — one past the u8 bone index.
    let a = component(
        0..256,
        (0..256).fold(MeshVertices::default(), |mut vertices, slot| {
            vertices.positions.push([slot as f32, 0.0, 0.0]);
            vertices
                .bone_weights
                .get_or_insert_with(Vec::new)
                .push([255, 0, 0, 0]);
            vertices
                .bone_indices
                .get_or_insert_with(Vec::new)
                .push([slot as u8, 0, 0, 0]);
            vertices
        }),
    );
    let b = component(256..257, skinned_vertex(0));
    let group = MeshGroup {
        name: "split-mesh".to_owned(),
        parent: Some(0),
        meshes: vec![0, 1],
        bounding_box: None,
        visible: true,
        split_mesh_group: true,
    };
    // Not one mesh: `decode` keeps such a group split.
    assert!(matches!(
        combine::combine(&[a, b], &group),
        Err(FmdlError::SplitIncompatible(_))
    ));
}

// S17: components with different attribute sets cannot combine.
#[test]
fn combine_rejects_mismatched_layouts() {
    let with_normals = grid(4, 4, 0);
    let mut without_normals = grid(4, 4, 0);
    without_normals.vertices.normals = None;
    let group = MeshGroup {
        name: "split-mesh".to_owned(),
        parent: Some(0),
        meshes: vec![0, 1],
        bounding_box: None,
        visible: true,
        split_mesh_group: true,
    };
    assert!(matches!(
        combine::combine(&[with_normals, without_normals], &group),
        Err(FmdlError::SplitIncompatible("vertex layouts disagree"))
    ));
}

// S18: a fitting subtree climbs from a preferred bone to its ancestors.
#[test]
fn a_fitting_subtree_climbs() {
    let mut model = grid_model(grid(60, 20, 40), 40);
    model.bones[20].name = "sk_hand_l".to_owned();
    assert!(encode(&mut model, None).unwrap());
    // The first component's bone group covers the whole fitting subtree
    // (without the climb it would stop at bone 20's own, about 21 bones).
    assert!(model.meshes[0].bone_group.len() >= 28);
}

// S19: vertices no face references still travel into a component.
fn loose_vertices_travel(mesh: Mesh, bones: usize) -> Model {
    let source: BTreeSet<VertexTuple> = (0..mesh.vertices.positions.len())
        .map(|index| vertex_tuple(&mesh, index))
        .collect();
    let mut model = grid_model(mesh, bones);
    assert!(encode(&mut model, None).unwrap());
    components_under_limits(&model);
    let covered: BTreeSet<VertexTuple> = model
        .meshes
        .iter()
        .flat_map(|mesh| (0..mesh.vertices.positions.len()).map(|index| vertex_tuple(mesh, index)))
        .collect();
    assert_eq!(covered, source);
    model
}

#[test]
fn loose_vertices_unskinned() {
    loose_vertices_travel(grid(400, 400, 0), 0);
}

// S20
#[test]
fn decode_group_parent_out_of_range_errors() {
    let mut model = grid_model(grid(4, 4, 0), 0);
    model.mesh_groups.push(MeshGroup {
        name: "orphan".to_owned(),
        parent: Some(9),
        meshes: Vec::new(),
        bounding_box: None,
        visible: true,
        split_mesh_group: false,
    });
    assert!(matches!(
        decode(&mut model),
        Err(FmdlError::BadReference {
            what: "mesh group",
            index: 9
        })
    ));
}

// A caller-supplied hierarchy with a cycle must not hang the climb; the
// same cycle cut `effective_parents` applies runs on the slice.
#[test]
fn a_cyclic_parents_override_terminates() {
    let model = grid_model(grid(60, 20, 40), 40);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut model = model;
        // Every bone's parent is its successor, the last pointing at 0.
        let parents: Vec<Option<usize>> = (0..40).map(|index| Some((index + 1) % 40)).collect();
        assert!(encode(&mut model, Some(&parents)).unwrap());
        tx.send(model).expect("send the encoded model");
    });
    let model = rx
        .recv_timeout(std::time::Duration::from_secs(120))
        .expect("encode hung on a cyclic parents slice");
    components_under_limits(&model);
}

// Vertex-limit binding in the fragment's face and loose loops: every
// position carries eight loops, so a component keeps filling on loose
// sets after the faces are in, and a few sets defer to the next
// component.
#[test]
fn vertex_limit_binds_before_faces() {
    const WIDTH: usize = 96;
    const HEIGHT: usize = 81;
    let mut vertices = MeshVertices {
        positions: Vec::new(),
        normals: Some(Vec::new()),
        tangents: None,
        colors: None,
        uvs: vec![Vec::new()],
        uv_high_precision: vec![true],
        bone_weights: None,
        bone_indices: None,
    };
    for base in 0..WIDTH * HEIGHT {
        for copy in 0..8 {
            vertices.positions.push([base as f32, 0.0, 0.0]);
            vertices
                .normals
                .as_mut()
                .unwrap()
                .push([0.0, 0.0, 1.0, 0.0]);
            vertices.uvs[0].push([copy as f32, 0.0]);
        }
    }
    // Loose vertices in 16-copy sets: a mis-scaled bound in either loop
    // moves the count by whole sets, and a `+`-as-`-` bound overshoots
    // the cap instead of landing on it.
    for loose in 0..500 {
        for _ in 0..16 {
            vertices.positions.push([0.0, 0.0, loose as f32 + 1.0]);
            vertices
                .normals
                .as_mut()
                .unwrap()
                .push([0.0, 0.0, 1.0, 0.0]);
            vertices.uvs[0].push([0.0, 0.0]);
        }
    }
    let mut mesh = Mesh {
        vertices,
        faces: Vec::new(),
        bone_group: Vec::new(),
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    for y in 0..HEIGHT - 1 {
        for x in 0..WIDTH - 1 {
            let v00 = 8 * (y * WIDTH + x);
            let v10 = v00 + 8;
            let v01 = v00 + 8 * WIDTH;
            let v11 = v01 + 8;
            mesh.faces.push([v00 as u16, v10 as u16, v11 as u16]);
            mesh.faces.push([v00 as u16, v11 as u16, v01 as u16]);
        }
    }
    let model = loose_vertices_travel(mesh, 0);
    // The loose loop keeps filling: the first component ends within one
    // set of the vertex cap.
    assert!(model.meshes[0].vertices.positions.len() >= VERTEX_LIMIT_SOFT - 8);
}

// Bone-limit binding in the loose loop: one face on bone 0 plus 40 loose
// vertices weighted to a different bone each; the root's items do not fit
// a 30-bone group, so the fragment path must bound the loose loop by it.
#[test]
fn bone_limit_binds_in_the_loose_loop() {
    let mut vertices = MeshVertices {
        positions: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        normals: Some(vec![[0.0, 0.0, 1.0, 0.0]; 3]),
        tangents: None,
        colors: None,
        uvs: vec![vec![[0.0; 2]; 3]],
        uv_high_precision: vec![true],
        bone_weights: Some(vec![[255, 0, 0, 0]; 3]),
        bone_indices: Some(vec![[0, 0, 0, 0]; 3]),
    };
    for bone in 0..40u8 {
        vertices.positions.push([0.0, f32::from(bone) + 1.0, 0.0]);
        vertices
            .normals
            .as_mut()
            .unwrap()
            .push([0.0, 0.0, 1.0, 0.0]);
        vertices.uvs[0].push([0.0, 0.0]);
        vertices.bone_weights.as_mut().unwrap().push([255, 0, 0, 0]);
        vertices
            .bone_indices
            .as_mut()
            .unwrap()
            .push([bone, 0, 0, 0]);
    }
    let mesh = Mesh {
        vertices,
        faces: vec![[0, 1, 2]],
        bone_group: (0..40).collect(),
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    let source: BTreeSet<VertexTuple> = (0..mesh.vertices.positions.len())
        .map(|index| vertex_tuple(&mesh, index))
        .collect();
    let mut model = grid_model(mesh, 40);
    assert!(encode(&mut model, None).unwrap());
    for component in &model.meshes {
        assert!(component.bone_group.len() <= BONE_LIMIT_SOFT);
    }
    let covered: BTreeSet<VertexTuple> = model
        .meshes
        .iter()
        .flat_map(|mesh| (0..mesh.vertices.positions.len()).map(|index| vertex_tuple(mesh, index)))
        .collect();
    assert_eq!(covered, source);
}

// A loose vertex weighted to a bone lands in the climbed subtree's
// component, not with whatever lower subtree owned it.
#[test]
fn a_loose_vertex_joins_the_climbed_subtree() {
    let mut mesh = grid(60, 20, 40);
    mesh.vertices.positions.push([999.0, -999.0, 999.0]);
    mesh.vertices
        .normals
        .as_mut()
        .unwrap()
        .push([0.0, 0.0, 1.0, 0.0]);
    mesh.vertices.uvs[0].push([0.0, 0.0]);
    mesh.vertices
        .bone_weights
        .as_mut()
        .unwrap()
        .push([255, 0, 0, 0]);
    mesh.vertices
        .bone_indices
        .as_mut()
        .unwrap()
        .push([39, 0, 0, 0]);
    let mut model = grid_model(mesh, 40);
    model.bones[20].name = "sk_hand_l".to_owned();
    assert!(encode(&mut model, None).unwrap());
    assert!(
        model.meshes[0]
            .vertices
            .positions
            .contains(&[999.0, -999.0, 999.0])
    );
}

// Coincident vertices differing only in a zero-weight bone-index lane
// share one equipresent set: the split key is the positive-weight bone
// mapping, not the raw index lanes.
#[test]
fn coincident_vertices_share_the_mapping_key() {
    const FILLERS: usize = 63_000;
    let mut vertices = MeshVertices {
        positions: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        normals: Some(vec![[0.0, 0.0, 1.0, 0.0]; 3]),
        tangents: None,
        colors: None,
        uvs: vec![vec![[0.0; 2]; 3]],
        uv_high_precision: vec![true],
        bone_weights: Some(vec![[255, 0, 0, 0]; 3]),
        bone_indices: Some(vec![[0, 0, 0, 0]; 3]),
    };
    // Vertex 3: a loose copy of vertex 0 whose only difference is a
    // zero-weight lane (index 7, weight 0).
    vertices.positions.push([0.0; 3]);
    vertices
        .normals
        .as_mut()
        .unwrap()
        .push([0.0, 0.0, 1.0, 0.0]);
    vertices.uvs[0].push([0.0, 0.0]);
    vertices.bone_weights.as_mut().unwrap().push([255, 0, 0, 0]);
    vertices.bone_indices.as_mut().unwrap().push([0, 7, 0, 0]);
    // Loose vertices at strictly higher projections fill the first
    // component, so the copy is what spills over.
    for filler in 0..FILLERS {
        vertices.positions.push([filler as f32 + 2.0, 0.0, 0.0]);
        vertices
            .normals
            .as_mut()
            .unwrap()
            .push([0.0, 0.0, 1.0, 0.0]);
        vertices.uvs[0].push([0.0, 0.0]);
        vertices.bone_weights.as_mut().unwrap().push([255, 0, 0, 0]);
        vertices.bone_indices.as_mut().unwrap().push([0, 0, 0, 0]);
    }
    let mesh = Mesh {
        vertices,
        faces: vec![[0, 1, 2]],
        bone_group: (0..40).collect(),
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    let faces_before = face_tuples(&mesh);
    let count_before = mesh.vertices.positions.len();
    let mut model = grid_model(mesh, 40);
    assert!(encode(&mut model, None).unwrap());

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

// A combined mesh keeps the emission order when every referenced index
// fits u16: a loose loop stays right after its owner.
#[test]
fn combine_keeps_loose_loops_next_to_their_owners() {
    // [A0, A1, B, C] where A1 is an unreferenced loop of A0 (same position,
    // later uv); the face references A0, B, C.
    let component = Mesh {
        vertices: MeshVertices {
            positions: vec![[0.0; 3], [0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: Some(vec![[0.0, 0.0, 1.0, 0.0]; 4]),
            tangents: None,
            colors: None,
            uvs: vec![vec![[0.0, 0.0], [1.0, 0.0], [0.0, 0.0], [0.0, 0.0]]],
            uv_high_precision: vec![true],
            bone_weights: None,
            bone_indices: None,
        },
        faces: vec![[0, 2, 3]],
        bone_group: Vec::new(),
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    let group = MeshGroup {
        name: SPLIT_GROUP_NAME.to_owned(),
        parent: Some(0),
        meshes: vec![0],
        bounding_box: None,
        visible: true,
        split_mesh_group: true,
    };
    let combined = combine::combine(&[component], &group).unwrap();
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
        vertices: MeshVertices {
            positions: (0..count).map(|index| [index as f32, 0.0, 0.0]).collect(),
            normals: Some(vec![[0.0, 0.0, 1.0, 0.0]; count]),
            tangents: None,
            colors: None,
            uvs: vec![vec![[0.0, 0.0]; count]],
            uv_high_precision: vec![true],
            bone_weights: None,
            bone_indices: None,
        },
        // Vertices 1 and 3..=65534 are loose; 65535 is the largest
        // referenced index and still fits u16.
        faces: vec![[0, 2, 65535]],
        bone_group: Vec::new(),
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    let group = MeshGroup {
        name: SPLIT_GROUP_NAME.to_owned(),
        parent: Some(0),
        meshes: vec![0],
        bounding_box: None,
        visible: true,
        split_mesh_group: true,
    };
    let combined = combine::combine(std::slice::from_ref(&component), &group).unwrap();
    assert_eq!(combined.vertices.positions, component.vertices.positions);
    assert_eq!(combined.faces, vec![[0, 2, 65535]]);
}

// A combine failure leaves the model untouched: the second split group's
// second component weights a slot past its bone group, so combine errors —
// with the mesh list still populated.
#[test]
fn a_failed_combine_leaves_the_model_intact() {
    let mut model = grid_model(grid(2, 2, 0), 0);
    // Group 0 stays a plain parent; add two split groups under it.
    let parent = MeshGroup {
        name: "parent".to_owned(),
        parent: None,
        meshes: Vec::new(),
        bounding_box: None,
        visible: true,
        split_mesh_group: false,
    };
    let split_group = |meshes: Vec<usize>| MeshGroup {
        name: "split".to_owned(),
        parent: Some(0),
        meshes,
        bounding_box: None,
        visible: true,
        split_mesh_group: true,
    };
    let component = |colors: Option<Vec<[u8; 4]>>| Mesh {
        vertices: MeshVertices {
            positions: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: Some(vec![[0.0, 0.0, 1.0, 0.0]; 3]),
            colors,
            uvs: vec![vec![[0.0; 2]; 3]],
            uv_high_precision: vec![true],
            ..MeshVertices::default()
        },
        faces: vec![[0, 1, 2]],
        bone_group: Vec::new(),
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    model.mesh_groups[0] = parent;
    // Group 1's components are identical, so its combine succeeds.
    model.meshes = vec![
        component(Some(vec![[1, 2, 3, 4]; 3])),
        component(Some(vec![[1, 2, 3, 4]; 3])),
        component(None),
        component(None),
    ];
    model.mesh_groups.push(split_group(vec![0, 1]));
    // The second group's second component weights a slot past its bone
    // group: a real combine error, not a kept group.
    for mesh in &mut model.meshes[2..] {
        mesh.vertices.bone_weights = Some(vec![[255, 0, 0, 0]; 3]);
        mesh.vertices.bone_indices = Some(vec![[0, 0, 0, 0]; 3]);
        mesh.bone_group = vec![0];
    }
    model.meshes[3].vertices.bone_indices = Some(vec![[9, 0, 0, 0]; 3]);
    model.mesh_groups.push(split_group(vec![2, 3]));

    let before = model.clone();
    assert!(decode(&mut model).is_err());
    assert_eq!(model, before);
}

// A group whose components are not one mesh stays split: their vertex
// layouts disagree, so the components and flag pass through while the
// other groups combine.
#[test]
fn a_group_whose_components_are_not_one_mesh_stays_split() {
    let mut model = grid_model(grid(2, 2, 0), 0);
    let split_group = |meshes: Vec<usize>| MeshGroup {
        name: "split".to_owned(),
        parent: Some(0),
        meshes,
        bounding_box: None,
        visible: true,
        split_mesh_group: true,
    };
    let component = |normals: Option<Vec<[f32; 4]>>| Mesh {
        vertices: MeshVertices {
            positions: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals,
            uvs: vec![vec![[0.0; 2]; 3]],
            uv_high_precision: vec![true],
            ..MeshVertices::default()
        },
        faces: vec![[0, 1, 2]],
        bone_group: Vec::new(),
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    let normals = Some(vec![[0.0, 0.0, 1.0, 0.0]; 3]);
    model.meshes = vec![
        component(normals.clone()),
        component(None),
        component(normals.clone()),
        component(normals),
    ];
    model.mesh_groups[0].meshes = Vec::new();
    // Group 1's components disagree on layout; group 2's combine.
    model.mesh_groups.push(split_group(vec![0, 1]));
    model.mesh_groups.push(split_group(vec![2, 3]));
    let components = model.meshes[..2].to_vec();

    decode(&mut model).expect("an incompatible group stays split");

    assert_eq!(model.meshes.len(), 3);
    assert_eq!(model.meshes[..2], components[..]);
    // The kept container stays flagged and lists its components; the
    // combined group's container is gone and its mesh joined the parent.
    assert_eq!(model.mesh_groups.len(), 2);
    assert!(model.mesh_groups[1].split_mesh_group);
    assert_eq!(model.mesh_groups[1].meshes, vec![0, 1]);
    assert_eq!(model.mesh_groups[0].meshes, vec![2]);
    assert!(model.extensions.mesh_splitting);
}

// Every field a combined mesh takes from its first component —
// material, the flag words, the anti-blur pair, the custom box — marks
// the group as never-one-mesh when components disagree on it.
#[test]
fn a_group_whose_components_disagree_on_fields_stays_split() {
    let mutates: [fn(&mut Mesh); 6] = [
        |component| component.material = 1,
        |component| component.alpha_flags = 2,
        |component| component.shadow_flags = 1,
        |component| component.has_antiblur_meshes = true,
        |component| component.is_antiblur_mesh = true,
        |component| {
            component.custom_bounding_box = Some(BoundingBox {
                max: [1.0; 4],
                min: [0.0; 4],
            })
        },
    ];
    for mutate in mutates {
        let mut model = grid_model(grid(2, 2, 0), 0);
        let split_group = |meshes: Vec<usize>| MeshGroup {
            name: "split".to_owned(),
            parent: Some(0),
            meshes,
            bounding_box: None,
            visible: true,
            split_mesh_group: true,
        };
        let component = || Mesh {
            vertices: MeshVertices {
                positions: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                normals: Some(vec![[0.0, 0.0, 1.0, 0.0]; 3]),
                uvs: vec![vec![[0.0; 2]; 3]],
                uv_high_precision: vec![true],
                ..MeshVertices::default()
            },
            faces: vec![[0, 1, 2]],
            bone_group: Vec::new(),
            material: 0,
            alpha_flags: 0,
            shadow_flags: 0,
            has_antiblur_meshes: false,
            is_antiblur_mesh: false,
            custom_bounding_box: None,
        };
        let mut second = component();
        mutate(&mut second);
        model.materials.push(material());
        model.meshes = vec![component(), second, component(), component()];
        model.mesh_groups[0].meshes = Vec::new();
        model.mesh_groups.push(split_group(vec![0, 1]));
        model.mesh_groups.push(split_group(vec![2, 3]));
        let components = model.meshes[..2].to_vec();

        decode(&mut model).expect("an incompatible group stays split");

        assert_eq!(model.meshes.len(), 3);
        assert_eq!(model.meshes[..2], components[..]);
        assert_eq!(model.mesh_groups[1].meshes, vec![0, 1]);
    }
}
