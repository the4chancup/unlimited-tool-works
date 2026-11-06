use super::export::{quantize_weights, split_parents};
use super::*;

use crate::affine::Affine;
use crate::formats::ConvertError;
use crate::ir::{Bone, CanonicalModel, Material, Mesh, MeshGroup, SourceFormat, Texture, Vertices};
use crate::loss::{Finding, Subject};
use crate::materials::MaterialFamily;

use crate::materials::TextureRole;

const HIGHNECK: &[u8] = include_bytes!("../../../tests/fixtures/konami_highneck.fmdl");
const HEAD_HI: &[u8] =
    include_bytes!("../../../../pes_model/tests/fixtures/konami_headHi.wesys.model");
const HEAD_HI_MTL: &[u8] = include_bytes!("../../../../pes_model/tests/fixtures/konami_headHi.mtl");
const ORAL: &[u8] = include_bytes!("../../../tests/fixtures/addon_oral.fmdl");
const AU: &[u8] = include_bytes!("../../../tests/fixtures/konami_au_Low_parts.fmdl");
const AU_SKL: &[u8] = include_bytes!("../../../tests/fixtures/konami_au00.skl");

fn read_fmdl(bytes: &[u8]) -> ::fmdl::Model {
    ::fmdl::Model::from_file(&::fmdl::FmdlFile::read(bytes).expect("parse")).expect("model")
}

/// `fmdl_to_ir`'s decode half: the whole-mesh form of `model`.
fn decoded(model: &::fmdl::Model) -> ::fmdl::Model {
    let mut model = model.clone();
    ::fmdl::ops::split::decode(&mut model).expect("split decode");
    ::fmdl::ops::antiblur::decode(&mut model).expect("antiblur decode");
    model
}

/// `ir_to_fmdl`'s encoder half, run on a decoded model for comparison.
fn encoded(decoded: &::fmdl::Model) -> ::fmdl::Model {
    let mut model = decoded.clone();
    // The exporter writes every mesh's anti-blur request clear.
    for mesh in &mut model.meshes {
        mesh.has_antiblur_meshes = false;
    }
    let owners: Vec<Vec<usize>> = model
        .meshes
        .iter()
        .map(::fmdl::ops::vertex_enc::decode)
        .collect::<Result<Vec<_>, _>>()
        .expect("vertex decode");
    ::fmdl::ops::vertex_enc::encode_model(&mut model, &owners).expect("vertex encode");
    let parents = split_parents(&model.bones);
    ::fmdl::ops::split::encode(&mut model, Some(&parents)).expect("split encode");
    model
}

/// A one-bone, one-mesh `fmdl::Model` for the synthetic cases.
fn fmdl_model(mesh: ::fmdl::Mesh, materials: Vec<::fmdl::MaterialInstance>) -> ::fmdl::Model {
    ::fmdl::Model {
        bones: vec![::fmdl::Bone {
            name: "sk_belly".to_string(),
            parent: None,
            bounding_box: ::fmdl::BoundingBox {
                min: [0.0; 4],
                max: [0.0; 4],
            },
            local_position: [0.0; 4],
            world_position: [0.0; 4],
        }],
        materials,
        meshes: vec![mesh],
        mesh_groups: vec![::fmdl::MeshGroup {
            name: "group".to_string(),
            parent: None,
            meshes: vec![0],
            bounding_box: None,
            visible: true,
            split_mesh_group: false,
        }],
        extensions: ::fmdl::Extensions::default(),
        bone_matrices: None,
    }
}

fn instance(name: &str) -> ::fmdl::MaterialInstance {
    ::fmdl::MaterialInstance {
        name: name.to_string(),
        shader: "fox3ddf_blin".to_string(),
        technique: "fox3DDF_Blin".to_string(),
        textures: vec![],
        parameters: vec![],
    }
}

/// A minimal consistent IR: one unskinned mesh, one `Shaded` material, one group.
fn minimal_ir() -> CanonicalModel {
    CanonicalModel {
        bones: vec![],
        meshes: vec![Mesh {
            vertices: Vertices {
                positions: vec![[0.0; 3]],
                ..Vertices::default()
            },
            faces: vec![[0, 0, 0]],
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

/// A flat `fmdl::Mesh` over no bone group with the given alpha flags and material.
fn fmdl_mesh(alpha: u8, material: usize) -> ::fmdl::Mesh {
    ::fmdl::Mesh {
        vertices: ::fmdl::format::MeshVertices {
            positions: vec![[0.0; 3]],
            ..::fmdl::format::MeshVertices::default()
        },
        faces: vec![[0, 0, 0]],
        bone_group: vec![],
        material,
        alpha_flags: alpha,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    }
}

mod export;
mod import;
mod round_trip;
