//! Provenance of `crates/tools/team_compiler/tests/fixtures/hand_split/body.fmdl`
//! (TC-MOD-31): a full-body model reduced to one strip, both arms with their hands still on,
//! for the compiler's hand auto-split.
//!
//! The strip is a grid of 11 columns (x = -5..5) by 3 rows (y = 0..2), each square cut along
//! its (x,y)-(x+1,y+1) diagonal: 33 vertices, 40 faces, one mesh, one material. Weights by
//! column, on PES 21's real bone names, each bone bound at its PES 21 template pose:
//!
//! | column | bone (weight)                              |
//! |--------|--------------------------------------------|
//! | -1..1  | `sk_chest` (1)                             |
//! | 2      | `sk_forearm_l` (1)                         |
//! | 3      | `sk_hand_l` (1)                            |
//! | 4      | `sk_hand_l` (0.5), `skh_index_mcp_l` (0.5) |
//! | 5      | `skh_index_mcp_l` (1)                      |
//!
//! and the negative columns the same with `_r`. The split selects columns 4 and 5 of a side
//! (a positive `skh_` weight), grows once to column 3, and separates the faces between
//! columns 3 and 5: each glove gets 8 faces and 9 vertices, the body the other 24 faces and
//! the 21 vertices of columns -3..3 (columns 3 and -3 are copied into both, the boundary).
//!
//! Not part of the workspace. Run it as a bin of a scratch crate that depends on `fmdl`,
//! `model_convert` and `pes_version` by path (`.tmp/phase_04/4_18/hand_4_18/roundtrip/` was the one used):
//! `hand_split_body <out fmdl>`. It writes the output once it has read it back, checked it with
//! `fmdl::check`, imported it again and split it to the counts above.
//! `hand_split_body_model.rs` writes the `.model` twin from the same strip.

use fmdl::{FmdlFile, Model};
use model_convert::Imported;
use model_convert::formats::fmdl::{fmdl_to_ir, ir_to_fmdl};
use model_convert::ir::{Bone, CanonicalModel, Material, Mesh, MeshGroup, SourceFormat, Vertices};
use model_convert::materials::MaterialFamily;
use model_convert::ops::hand_split::split_by_skeleton_group;
use model_convert::skeletons::skeletons;
use pes_version::PesVersion;

const BONES: [&str; 7] = [
    "sk_chest",
    "sk_forearm_l",
    "sk_hand_l",
    "skh_index_mcp_l",
    "sk_forearm_r",
    "sk_hand_r",
    "skh_index_mcp_r",
];

/// The bone-group indices and weights of a vertex in column `x`.
fn weights(x: i32) -> ([u8; 4], [f32; 4]) {
    // The left side's bones are 1..=3, the right side's 4..=6.
    let side = if x < 0 { 3 } else { 0 };
    match x.abs() {
        0 | 1 => ([0, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
        2 => ([1 + side, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
        3 => ([2 + side, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
        4 => ([2 + side, 3 + side, 0, 0], [0.5, 0.5, 0.0, 0.0]),
        _ => ([3 + side, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
    }
}

pub fn strip() -> CanonicalModel {
    let tables = skeletons(PesVersion::Pes21);
    let bones = BONES
        .iter()
        .map(|&name| {
            let table = tables
                .body
                .bone(name)
                .or_else(|| tables.hand_l.bone(name))
                .or_else(|| tables.hand_r.bone(name))
                .unwrap_or_else(|| panic!("{name} is a PES 21 bone"));
            Bone {
                name: name.to_string(),
                parent: None,
                matrix: table.matrix,
                global_position: None,
                local_position: None,
                bounding_box: None,
            }
        })
        .collect();

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let mut bone_weights = Vec::new();
    for x in -5..=5i32 {
        for y in 0..3i32 {
            positions.push([x as f32 * 0.1, 1.4 + y as f32 * 0.02, 0.0]);
            normals.push([0.0, 0.0, 1.0, 1.0]);
            uvs.push([(x + 5) as f32 / 10.0, y as f32 / 2.0]);
            let (row, ws) = weights(x);
            indices.push(row);
            bone_weights.push(ws);
        }
    }
    let at = |column: usize, y: usize| (column * 3 + y) as u16;
    let mut faces = Vec::new();
    for column in 0..10usize {
        for y in 0..2usize {
            let (a, b, c, d) = (
                at(column, y),
                at(column + 1, y),
                at(column + 1, y + 1),
                at(column, y + 1),
            );
            faces.push([a, b, c]);
            faces.push([a, c, d]);
        }
    }
    CanonicalModel {
        bones,
        meshes: vec![Mesh {
            vertices: Vertices {
                positions,
                normals: Some(normals),
                uvs: vec![uvs],
                uv_high_precision: vec![false],
                bone_indices: Some(indices),
                bone_weights: Some(bone_weights),
                bone_weight_width: Some(4),
                ..Vertices::default()
            },
            faces,
            bone_group: (0..BONES.len()).collect(),
            material: 0,
            extension_headers: Default::default(),
            custom_bounding_box: None,
        }],
        mesh_groups: vec![MeshGroup {
            name: "body".to_string(),
            parent: None,
            meshes: vec![0],
            visible: true,
        }],
        materials: vec![Material {
            name: "body_mat".to_string(),
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

/// (vertices, faces) over every mesh of `model`.
pub fn counts(model: &CanonicalModel) -> (usize, usize) {
    let vertices = model.meshes.iter().map(|m| m.vertices.positions.len()).sum();
    let faces = model.meshes.iter().map(|m| m.faces.len()).sum();
    (vertices, faces)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [out] = args.as_slice() else {
        panic!("hand_split_body <out fmdl>")
    };
    let exported = ir_to_fmdl(&strip()).expect("the strip exports");
    for finding in &exported.findings {
        println!("export: {} {:?} {}", finding.code, finding.subject, finding.detail);
    }
    assert!(exported.skl.is_none(), "every bone is in PES 21's tables");
    let written = exported.model.to_file().expect("the model writes").write();

    let read_back = Model::from_file(&FmdlFile::read(&written).expect("an FMDL")).expect("a model");
    let checked = fmdl::check::check(&read_back);
    for finding in &checked {
        println!("check: {} {:?} {}", finding.code, finding.severity, finding.count);
    }
    assert!(
        checked
            .iter()
            .all(|finding| finding.severity != fmdl::check::Severity::Error),
        "fmdl::check raises no Error"
    );

    let Imported { model, findings } = fmdl_to_ir(&read_back, None).expect("the model imports");
    for finding in &findings {
        println!("import: {} {:?} {}", finding.code, finding.subject, finding.detail);
    }
    assert_eq!(counts(&model), (33, 40));
    let split = split_by_skeleton_group(&model);
    let glove_l = split.glove_l.as_ref().expect("a left glove");
    let glove_r = split.glove_r.as_ref().expect("a right glove");
    assert_eq!(counts(glove_l), (9, 8));
    assert_eq!(counts(glove_r), (9, 8));
    assert_eq!(counts(&split.body), (21, 24));

    let temporary = format!("{out}.tmp");
    std::fs::write(&temporary, &written).expect("the fixture writes");
    std::fs::rename(&temporary, out).expect("the fixture moves into place");
    println!("{out}: {} bytes", written.len());
}
