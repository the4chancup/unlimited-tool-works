use super::import::bone_weights;
use super::*;

use ::pes_model::format::mtl;
use ::pes_model::model::Model;

use crate::affine::Affine;
use crate::formats::{ConvertError, fmdl};
use crate::ir::{Bone, CanonicalModel, Material, Mesh, MeshGroup, SourceFormat, Texture, Vertices};
use crate::loss::{Finding, Subject};
use crate::materials::{FoxMaterial, PreFoxMaterial, TextureRole, to_fox, to_prefox};
use crate::skeletons;

use crate::materials::MaterialFamily;
use pes_version::PesVersion;

const CAP_M: &[u8] = include_bytes!("../../../tests/fixtures/konami_modD_cap.model");
const CAP_T: &[u8] = include_bytes!("../../../tests/fixtures/konami_modD_cap.mtl");
const CARD_M: &[u8] = include_bytes!("../../../tests/fixtures/konami_card.model");
const CARD_T: &[u8] = include_bytes!("../../../tests/fixtures/konami_card_red.mtl");
const GLASSES_M: &[u8] = include_bytes!("../../../tests/fixtures/konami_glasses_02.wesys.model");
const GLASSES_T: &[u8] = include_bytes!("../../../tests/fixtures/konami_accessory.mtl");
const CARDHEAD_M: &[u8] = include_bytes!("../../../tests/fixtures/cardhead_face_high.model");
const CARDHEAD_T: &[u8] = include_bytes!("../../../tests/fixtures/cardhead_materials.mtl");
const HIGHNECK: &[u8] = include_bytes!("../../../tests/fixtures/konami_highneck.fmdl");
const ORAL: &[u8] = include_bytes!("../../../tests/fixtures/addon_oral.fmdl");
const MAXFILTER_T: &[u8] =
    include_bytes!("../../../../pes_model/tests/fixtures/community_maxfilter.mtl");

fn model(bytes: &[u8]) -> Model {
    Model::from_file(&::pes_model::format::PreFoxModel::read(bytes).expect("parse")).expect("model")
}

fn set(bytes: &[u8]) -> mtl::MaterialSet {
    mtl::MaterialSet::read(bytes).expect("mtl")
}

/// `ir_to_model`'s output applied to `model` the same way: split decode, the
/// indices-only weight synthesis, then the vertex-loop and split encoders with the
/// render parents.
fn expected(model: &Model) -> Model {
    let mut model = model.clone();
    ::pes_model::ops::split::decode(&mut model).expect("split decode");
    for mesh in &mut model.meshes {
        let synthesized =
            mesh.vertices.bone_indices.is_some() && mesh.vertices.bone_weights.is_none();
        mesh.vertices.bone_weights = bone_weights(&mesh.vertices);
        if synthesized {
            mesh.vertices.bone_weight_width = 4;
        }
    }
    let owners: Vec<Vec<usize>> = model
        .meshes
        .iter()
        .map(::pes_model::ops::vertex_enc::decode)
        .collect::<Result<_, _>>()
        .expect("vertex decode");
    ::pes_model::ops::vertex_enc::encode_model(&mut model, &owners).expect("vertex encode");
    let parents: Vec<Option<usize>> = model
        .bones
        .iter()
        .map(|bone| {
            skeletons::render_parent(&bone.name)
                .and_then(|parent| model.bones.iter().position(|b| b.name == parent))
        })
        .collect();
    ::pes_model::ops::split::encode(&mut model, &parents).expect("split encode");
    model
}

/// One `.mtl` definition with no entries.
fn mtl_of(name: &str, shader: &str) -> mtl::MaterialSet {
    mtl::MaterialSet {
        materials: vec![mtl::Material {
            name: name.to_string(),
            shader: shader.to_string(),
            entries: vec![],
        }],
        style: mtl::MtlStyle::default(),
    }
}

/// A minimal mesh over `bone_group`, weighted to its first two slots.
fn pes_mesh(bone_group: Vec<usize>) -> ::pes_model::model::Mesh {
    ::pes_model::model::Mesh {
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
        bone_group,
        material: 0,
        bounds: ::pes_model::format::BoundingBox::of(&[[0.0; 3]]),
        order: 0,
        editor_data: vec![],
    }
}

/// A one-mesh `Model` over `bones` for the synthetic cases.
fn pes_model(bones: Vec<&str>, mesh: ::pes_model::model::Mesh) -> Model {
    let bounds = ::pes_model::format::BoundingBox::of(&mesh.vertices.positions);
    Model {
        flags: 0,
        bones: bones
            .into_iter()
            .map(|name| ::pes_model::format::Bone {
                name: name.to_string(),
                matrix: Affine::IDENTITY.0,
            })
            .collect(),
        materials: vec!["mat".to_string()],
        meshes: vec![mesh],
        extension_headers: vec![],
        bounds,
        lod: ::pes_model::format::LodRecord::for_levels(0),
    }
}

/// The entries of `entries` that are samplers (kind 0), states (1) or vectors (2).
fn of_kind(entries: &[mtl::MaterialEntry], kind: u8) -> Vec<mtl::MaterialEntry> {
    entries
        .iter()
        .filter(|entry| {
            matches!(
                (entry, kind),
                (mtl::MaterialEntry::Sampler(_), 0)
                    | (mtl::MaterialEntry::State(_), 1)
                    | (mtl::MaterialEntry::Vector(_), 2)
            )
        })
        .cloned()
        .collect()
}

#[test]
fn maxfilter_round_trips() {
    // `community_maxfilter.mtl`'s `fox_eyeOcclusion_mat` is the one sampler in
    // the fixture carrying `maxfilter`; it survives `.model -> IR -> .model`.
    let mut model = pes_model(vec!["sk_belly"], pes_mesh(vec![0]));
    model.materials[0] = "fox_eyeOcclusion_mat".to_string();
    let ir = model_to_ir(&model, &set(MAXFILTER_T))
        .expect("import")
        .model;
    let exported = ir_to_model(&ir).expect("export");
    let material = &exported.mtl.materials[0];
    let sampler = material
        .entries
        .iter()
        .find_map(|entry| match entry {
            mtl::MaterialEntry::Sampler(sampler) if sampler.name == "DiffuseMap" => Some(sampler),
            _ => None,
        })
        .expect("DiffuseMap");
    assert_eq!(sampler.maxfilter, Some(mtl::Filter::Linear));
}

#[test]
fn cap_imports_on_the_pes17_skeleton() {
    let imported = model_to_ir(&model(CAP_M), &set(CAP_T)).expect("import");
    let model = &imported.model;
    // The file order was already parent-first.
    assert_eq!(
        model
            .bones
            .iter()
            .map(|bone| bone.name.as_str())
            .collect::<Vec<_>>(),
        [
            "sk_shoulder_l",
            "sk_upperarm_l",
            "dsk_deltoid_l",
            "dsk_upperarm_long_l"
        ]
    );
    // `dsk_deltoid_l` under `sk_shoulder_l`; `dsk_upperarm_long_l`'s render parent
    // `dsk_upperarm_l` is absent, so it is a root.
    assert_eq!(model.bones[2].parent, Some(0));
    assert_eq!(model.bones[3].parent, None);
    let body = &crate::skeletons::skeletons(PesVersion::Pes17).body;
    for bone in &model.bones {
        let template = body.bone(&bone.name).expect("template bone");
        assert!(
            bone.matrix.max_component_delta(&template.matrix) < 2e-5,
            "{}",
            bone.name
        );
    }
    let material = &model.materials[0];
    assert_eq!(material.family, MaterialFamily::Shaded);
    assert_eq!(material.two_sided, None);
    assert_eq!(material.transparent, None);
    assert_eq!(
        imported.findings,
        vec![Finding {
            code: "native_field_dropped",
            subject: Subject::Mesh(0),
            detail: "tags".to_string(),
        }]
    );
}

#[test]
fn pes_model_round_trip() {
    for (model_bytes, mtl_bytes, konami) in [
        (CAP_M, CAP_T, true),
        (CARD_M, CARD_T, true),
        (GLASSES_M, GLASSES_T, true),
        (CARDHEAD_M, CARDHEAD_T, false),
    ] {
        let input = model(model_bytes);
        let input_mtl = set(mtl_bytes);
        let imported = model_to_ir(&input, &input_mtl).expect("import");
        let exported = ir_to_model(&imported.model).expect("export");
        let expected = expected(&input);

        assert_eq!(
            exported
                .model
                .bones
                .iter()
                .map(|bone| bone.name.as_str())
                .collect::<Vec<_>>(),
            expected
                .bones
                .iter()
                .map(|bone| bone.name.as_str())
                .collect::<Vec<_>>()
        );
        for (got, want) in exported.model.bones.iter().zip(&expected.bones) {
            assert!(
                Affine(got.matrix).max_component_delta(&Affine(want.matrix)) < 1e-5,
                "{}",
                got.name
            );
        }
        assert_eq!(exported.model.materials, expected.materials);
        assert_eq!(exported.model.meshes.len(), expected.meshes.len());
        for (index, (got, want)) in exported
            .model
            .meshes
            .iter()
            .zip(&expected.meshes)
            .enumerate()
        {
            assert_eq!(got.vertices, want.vertices, "mesh {index}");
            assert_eq!(got.faces, want.faces, "mesh {index}");
            assert!(got.lower_lods.is_empty(), "mesh {index}");
            assert_eq!(got.bone_group, want.bone_group, "mesh {index}");
            assert_eq!(got.material, want.material, "mesh {index}");
            assert!(got.tags.is_empty(), "mesh {index}");
            assert_eq!(got.order, 0, "mesh {index}");
            assert!(got.editor_data.is_empty(), "mesh {index}");
            assert_eq!(
                got.extension_headers, want.extension_headers,
                "mesh {index}"
            );
            if konami {
                assert_eq!(got.name, Some(format!("mesh_{index}")), "mesh {index}");
            } else {
                assert_eq!(got.name, want.name, "mesh {index}");
            }
            assert_eq!(got.bounds, want.bounds, "mesh {index}");
        }
        assert_eq!(exported.model.extension_headers, expected.extension_headers);
        assert_eq!(exported.model.flags, 0);
        assert_eq!(
            exported.model.lod,
            ::pes_model::format::LodRecord::for_levels(0)
        );
        assert_eq!(exported.model.bounds, expected.bounds);

        // The `.mtl`: the input's definitions filtered to the bound names in model
        // order, compared per entry kind (the export regroups to samplers, states,
        // vectors — Konami's majority order).
        let want: Vec<&mtl::Material> = input
            .materials
            .iter()
            .map(|name| {
                input_mtl
                    .materials
                    .iter()
                    .find(|material| material.name == *name)
                    .expect("bound material")
            })
            .collect();
        assert_eq!(exported.mtl.materials.len(), want.len());
        for (got, want) in exported.mtl.materials.iter().zip(want) {
            assert_eq!(got.name, want.name);
            assert_eq!(got.shader, want.shader);
            for kind in 0..3 {
                assert_eq!(
                    of_kind(&got.entries, kind),
                    of_kind(&want.entries, kind),
                    "{} kind {kind}",
                    got.name
                );
            }
        }
        assert_eq!(exported.mtl.style, mtl::MtlStyle::default());
    }
}

#[test]
fn pes_model_round_trip_findings() {
    // cap: one Konami tag dropped.
    let imported = model_to_ir(&model(CAP_M), &set(CAP_T)).expect("import");
    assert_eq!(
        imported.findings,
        vec![Finding {
            code: "native_field_dropped",
            subject: Subject::Mesh(0),
            detail: "tags".to_string(),
        }]
    );
    assert_eq!(
        ir_to_model(&imported.model).expect("export").findings,
        Vec::<Finding>::new()
    );
    // card: nothing to report.
    let imported = model_to_ir(&model(CARD_M), &set(CARD_T)).expect("import");
    assert_eq!(imported.findings, Vec::<Finding>::new());
    assert_eq!(
        ir_to_model(&imported.model).expect("export").findings,
        Vec::<Finding>::new()
    );
    // glasses: `Accessory` is no shader rule's name; both meshes carry Konami tags.
    let imported = model_to_ir(&model(GLASSES_M), &set(GLASSES_T)).expect("import");
    assert_eq!(
        imported.findings,
        vec![
            Finding {
                code: "material_family_approximated",
                subject: Subject::Material(1),
                detail: "glasses_02C".to_string(),
            },
            Finding {
                code: "native_field_dropped",
                subject: Subject::Mesh(0),
                detail: "tags".to_string(),
            },
            Finding {
                code: "native_field_dropped",
                subject: Subject::Mesh(1),
                detail: "tags".to_string(),
            },
        ]
    );
    assert_eq!(
        ir_to_model(&imported.model).expect("export").findings,
        Vec::<Finding>::new()
    );
    // card head: nothing to report.
    let imported = model_to_ir(&model(CARDHEAD_M), &set(CARDHEAD_T)).expect("import");
    assert_eq!(imported.findings, Vec::<Finding>::new());
    assert_eq!(
        ir_to_model(&imported.model).expect("export").findings,
        Vec::<Finding>::new()
    );
}

#[test]
fn a_bone_listed_before_its_parent_moves_after_it() {
    let input = pes_model(
        vec!["dsk_forearm_l", "dsk_forearm_t_l"],
        pes_mesh(vec![0, 1]),
    );
    let imported = model_to_ir(&input, &mtl_of("mat", "Basic_C")).expect("import");
    let bones = &imported.model.bones;
    assert_eq!(bones[0].name, "dsk_forearm_t_l");
    assert_eq!(bones[0].parent, None);
    assert_eq!(bones[1].name, "dsk_forearm_l");
    assert_eq!(bones[1].parent, Some(0));
    assert_eq!(imported.model.meshes[0].bone_group, [1, 0]);
    assert_eq!(
        imported.model.meshes[0].vertices.bone_indices,
        Some(vec![[0, 1, 0, 0]])
    );
}

#[test]
fn a_bound_material_without_a_definition_is_an_error() {
    let mut input = pes_model(vec!["sk_belly"], pes_mesh(vec![0]));
    input.materials = vec!["nope".to_string()];
    let empty = mtl::MaterialSet {
        materials: vec![],
        style: mtl::MtlStyle::default(),
    };
    let Err(ConvertError::MaterialUndefined(name)) = model_to_ir(&input, &empty) else {
        panic!("expected MaterialUndefined")
    };
    assert_eq!(name, "nope");
}

#[test]
fn a_bone_group_past_the_bone_table_is_an_error() {
    let input = pes_model(vec!["sk_belly", "sk_chest"], pes_mesh(vec![0, 5]));
    let Err(ConvertError::PesModel(::pes_model::format::ModelError::BadReference { what, offset })) =
        model_to_ir(&input, &mtl_of("mat", "Basic_C"))
    else {
        panic!("expected BadReference")
    };
    assert_eq!(what, "bone");
    assert_eq!(offset, 5);
}

#[test]
fn dropped_fields_report_each_field() {
    let mut mesh = pes_mesh(vec![0, 1]);
    mesh.lower_lods = vec![vec![[0, 0, 0]]];
    mesh.tags = vec![(1, "part".to_string())];
    mesh.order = 3;
    let input = pes_model(vec!["sk_belly", "sk_chest"], mesh);
    let imported = model_to_ir(&input, &mtl_of("mat", "Basic_C")).expect("import");
    assert_eq!(
        imported.findings,
        vec![
            Finding {
                code: "native_field_dropped",
                subject: Subject::Mesh(0),
                detail: "lower_lods".to_string(),
            },
            Finding {
                code: "native_field_dropped",
                subject: Subject::Mesh(0),
                detail: "tags".to_string(),
            },
            Finding {
                code: "native_field_dropped",
                subject: Subject::Mesh(0),
                detail: "order".to_string(),
            },
        ]
    );
}

#[test]
fn a_group_that_stays_split_imports_without_its_marker() {
    let component = |offset: usize, count: usize, faces: usize| {
        let mut mesh = pes_mesh(vec![]);
        mesh.extension_headers = vec!["Split-Mesh: 1".to_string()];
        mesh.vertices = ::pes_model::format::MeshVertices {
            positions: (0..count)
                .map(|index| [(offset + index) as f32, 0.0, 0.0])
                .collect(),
            normals: None,
            tangents: None,
            bitangents: None,
            colors: None,
            uvs: vec![],
            bone_indices: None,
            bone_weights: None,
            bone_weight_width: 4,
        };
        mesh.faces = (0..faces)
            .map(|face| {
                [
                    (3 * face) as u16,
                    (3 * face + 1) as u16,
                    (3 * face + 2) as u16,
                ]
            })
            .collect();
        mesh.bounds = ::pes_model::format::BoundingBox::of(&mesh.vertices.positions);
        mesh
    };
    // Two components referencing 70000 distinct vertices: u16 faces cannot
    // index the combined mesh, so the decode keeps the group split.
    let mut input = pes_model(vec![], component(0, 40000, 13333));
    input.meshes.push(component(40000, 30000, 10000));
    let imported = model_to_ir(&input, &mtl_of("mat", "Basic_C")).expect("import");
    assert_eq!(imported.model.meshes.len(), 2);
    assert_eq!(imported.model.mesh_groups.len(), 2);
    for mesh in &imported.model.meshes {
        assert!(mesh.extension_headers.is_empty());
    }
    // Export and re-import keep working.
    let exported = ir_to_model(&imported.model).expect("export");
    model_to_ir(&exported.model, &exported.mtl).expect("reimport");
}

#[test]
fn imported_mesh_groups_are_visible() {
    // A `.model` has no group visibility field: a converted group is
    // visible, the value an FMDL export carries through.
    let input = pes_model(vec!["sk_belly"], pes_mesh(vec![0]));
    let imported = model_to_ir(&input, &mtl_of("mat", "Basic_C")).expect("import");
    assert_eq!(imported.model.mesh_groups.len(), 1);
    assert!(imported.model.mesh_groups[0].visible);
}

#[test]
fn a_name_the_mtl_cannot_carry_is_an_error() {
    // The `.mtl` is XML: a NUL-holding material name is an error, not a file
    // no reader accepts (the corrupted FMDL name tables' case).
    let mut ir = ir_over(Vertices::default(), vec![]);
    ir.materials[0].name = "bad\0name".to_string();
    assert!(matches!(
        ir_to_model(&ir),
        Err(ConvertError::MtlName(name)) if name == "bad\0name"
    ));
    // A texture path is checked the same way.
    let mut ir = ir_over(Vertices::default(), vec![]);
    ir.materials[0].textures = vec![(TextureRole::Base, 0)];
    ir.textures = vec![Texture {
        directory: "./".to_string(),
        file_name: "t\0.dds".to_string(),
    }];
    assert!(matches!(
        ir_to_model(&ir),
        Err(ConvertError::MtlName(path)) if path == "./t\0.dds"
    ));
    // And a stored sampler name.
    let mut ir = ir_over(Vertices::default(), vec![]);
    ir.materials[0].prefox = Some(PreFoxMaterial {
        shader: "Basic_C".to_string(),
        states: vec![],
        samplers: vec![("sa\0m".to_string(), Default::default())],
        textures: vec![("sa\0m".to_string(), 0)],
        parameters: vec![],
    });
    ir.textures = vec![Texture {
        directory: "./".to_string(),
        file_name: "t.dds".to_string(),
    }];
    assert!(matches!(
        ir_to_model(&ir),
        Err(ConvertError::MtlName(name)) if name == "sa\0m"
    ));
}

#[test]
fn fox_ir_exports_to_pre_fox() {
    let highneck = fmdl::fmdl_to_ir(
        &::fmdl::Model::from_file(&::fmdl::FmdlFile::read(HIGHNECK).expect("parse"))
            .expect("model"),
        None,
    )
    .expect("import");
    let exported = ir_to_model(&highneck.model).expect("export");
    assert_eq!(exported.mtl.materials.len(), 1);
    let material = &highneck.model.materials[0];
    let roles: Vec<TextureRole> = material.textures.iter().map(|(role, _)| *role).collect();
    assert_eq!(
        exported.mtl.materials[0].shader,
        to_prefox::default_shader(material.family, &roles)
    );
    // The pre-Fox resolver carries nothing of the `fox` table: the native
    // sampler and parameters it held are findings.
    assert_eq!(
        exported.findings,
        [
            Finding {
                code: "material_texture_unused",
                subject: Subject::Material(0),
                detail: "Translucent_Tex_LIN".to_string(),
            },
            Finding {
                code: "material_parameter_dropped",
                subject: Subject::Material(0),
                detail: "MatParamIndex_0".to_string(),
            },
            Finding {
                code: "material_parameter_dropped",
                subject: Subject::Material(0),
                detail: "SelfColor".to_string(),
            },
        ]
    );
}

#[test]
fn cast_shadow_and_invisible_override_the_raw_flag_bits() {
    let ir_for = |cast_shadow: Option<bool>, invisible: Option<bool>, shadow_flags| {
        let mut ir = ir_over(
            Vertices {
                positions: vec![[0.0; 3]],
                ..Vertices::default()
            },
            vec![[0, 0, 0]],
        );
        ir.materials[0].fox = Some(FoxMaterial {
            shader: "fox3ddf_blin".to_string(),
            technique: "fox3DDF_Blin".to_string(),
            alpha_flags: 0,
            shadow_flags,
            cast_shadow,
            invisible,
            base_linear: false,
            textures: vec![],
            parameters: vec![],
        });
        ir
    };
    let dropped_details = |ir: &CanonicalModel| {
        ir_to_model(ir)
            .expect("export")
            .findings
            .into_iter()
            .filter(|f| f.code == "native_field_dropped")
            .map(|f| f.detail)
            .collect::<Vec<_>>()
    };
    // The one mesh hidden leaves the export with no mesh.
    let hidden =
        |ir: &CanonicalModel| matches!(ir_to_model(ir), Err(ConvertError::EveryMeshHidden));
    // `cast_shadow = false` sets the no-shadow bit even when the raw flags
    // don't carry it; `invisible = true` sets the invisible bit.
    assert_eq!(
        dropped_details(&ir_for(Some(false), Some(false), 0)),
        ["no_shadow_cast".to_string()],
    );
    assert!(hidden(&ir_for(None, Some(true), 0)));
    assert!(hidden(&ir_for(Some(false), Some(true), 0)));
    // A set raw bit stays set when the boolean agrees with it, or says nothing.
    assert_eq!(
        dropped_details(&ir_for(Some(false), None, to_fox::NO_SHADOW_CAST_BIT)),
        ["no_shadow_cast".to_string()],
    );
    assert!(hidden(&ir_for(None, Some(true), to_fox::INVISIBLE_BIT)));
    assert!(hidden(&ir_for(None, None, to_fox::INVISIBLE_BIT)));
    // `cast_shadow = true` clears a set raw bit — the material casts shadows.
    assert_eq!(
        dropped_details(&ir_for(Some(true), None, to_fox::NO_SHADOW_CAST_BIT)),
        Vec::<String>::new(),
    );
    // `invisible = false` clears a set raw bit — the material is visible, its
    // mesh written.
    assert_eq!(
        dropped_details(&ir_for(None, Some(false), to_fox::INVISIBLE_BIT)),
        Vec::<String>::new(),
    );
    // Clearing one bit must not clobber the other.
    assert!(hidden(&ir_for(
        Some(true),
        None,
        to_fox::NO_SHADOW_CAST_BIT | to_fox::INVISIBLE_BIT
    )));
    assert_eq!(
        dropped_details(&ir_for(
            None,
            Some(false),
            to_fox::NO_SHADOW_CAST_BIT | to_fox::INVISIBLE_BIT
        )),
        ["no_shadow_cast".to_string()],
    );
}

#[test]
fn a_hidden_fox_oral_exports_nothing_and_its_shadow_bit_is_a_finding() {
    // `addon_oral`'s `shadow_flags = 131` carries both engine-only bits: its
    // one mesh is hidden, so the export has nothing to write.
    let mut oral = fmdl::fmdl_to_ir(
        &::fmdl::Model::from_file(&::fmdl::FmdlFile::read(ORAL).expect("parse")).expect("model"),
        None,
    )
    .expect("import")
    .model;
    assert!(matches!(
        ir_to_model(&oral),
        Err(ConvertError::EveryMeshHidden)
    ));
    // Unhidden, the mesh is written and the no-shadow bit, which the `.mtl`
    // has no home for, is the one flag finding.
    for material in &mut oral.materials {
        material.fox.as_mut().expect("a Fox material").invisible = Some(false);
    }
    let exported = ir_to_model(&oral).expect("export");
    let flag_details: Vec<String> = exported
        .findings
        .into_iter()
        .filter(|f| f.code == "native_field_dropped" && matches!(f.subject, Subject::Material(_)))
        .map(|f| f.detail)
        .collect();
    assert_eq!(flag_details, ["no_shadow_cast".to_string()]);
}

#[test]
fn a_hidden_mesh_is_left_out_of_the_export() {
    let mut ir = ir_over(
        Vertices {
            positions: vec![[0.0; 3]],
            ..Vertices::default()
        },
        vec![[0, 0, 0]],
    );
    // Mesh 1, on material 1 whose raw flags hide it, carries a normal `w` the
    // `.model` could not keep and a vertex far outside mesh 0's box.
    ir.meshes.push(Mesh {
        vertices: Vertices {
            positions: vec![[10.0, 10.0, 10.0]],
            normals: Some(vec![[0.0, 0.0, 1.0, 0.5]]),
            ..Vertices::default()
        },
        faces: vec![[0, 0, 0]],
        bone_group: vec![0],
        material: 1,
        extension_headers: Default::default(),
        custom_bounding_box: None,
    });
    ir.mesh_groups[0].meshes.push(1);
    let mut hidden = ir.materials[0].clone();
    hidden.name = "hidden".to_string();
    hidden.fox = Some(FoxMaterial {
        shader: "fox3ddf_blin".to_string(),
        technique: "fox3DDF_Blin".to_string(),
        alpha_flags: 0,
        shadow_flags: to_fox::INVISIBLE_BIT,
        cast_shadow: None,
        invisible: None,
        base_linear: false,
        textures: vec![],
        parameters: vec![],
    });
    ir.materials.push(hidden);

    let exported = ir_to_model(&ir).expect("export");
    assert_eq!(exported.model.meshes.len(), 1);
    assert_eq!(exported.model.meshes[0].material, 0);
    // The model's box is mesh 0's alone.
    assert_eq!(exported.model.bounds.max, [0.0; 4]);
    // The hidden mesh's material stays in the model and the `.mtl`.
    assert_eq!(exported.model.materials, ["mat", "hidden"]);
    assert_eq!(exported.mtl.materials.len(), 2);
    assert!(
        !exported
            .findings
            .iter()
            .any(|f| f.detail == "invisible" || f.subject == Subject::Mesh(1)),
        "{:?}",
        exported.findings
    );
}

#[test]
fn a_dropped_fox_parameter_is_a_finding() {
    // A canonical parameter is carried while a `fox`-only name drops next to
    // it: the finding names only the dropped one.
    let mut ir = ir_over(
        Vertices {
            positions: vec![[0.0; 3]],
            ..Vertices::default()
        },
        vec![[0, 0, 0]],
    );
    ir.materials[0].parameters = vec![("Shininess".to_string(), [0.5, 0.0, 0.0, 0.0])];
    ir.materials[0].fox = Some(FoxMaterial {
        shader: "fox3ddf_blin".to_string(),
        technique: "fox3DDF_Blin".to_string(),
        alpha_flags: 0,
        shadow_flags: 0,
        cast_shadow: None,
        invisible: None,
        base_linear: false,
        textures: vec![],
        parameters: vec![("SelfColor".to_string(), [1.0, 1.0, 1.0, 1.0])],
    });
    let exported = ir_to_model(&ir).expect("export");
    assert_eq!(
        exported.findings,
        [Finding {
            code: "material_parameter_dropped",
            subject: Subject::Material(0),
            detail: "SelfColor".to_string(),
        }]
    );
    // The canonical parameter rode along in the `.mtl`.
    let vectors: Vec<String> = exported.mtl.materials[0]
        .entries
        .iter()
        .filter_map(|entry| match entry {
            mtl::MaterialEntry::Vector(vector) => Some(vector.name.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(vectors, ["Shininess".to_string()]);
}

#[test]
fn a_default_valued_fox_parameter_is_not_a_finding() {
    // `MatParamIndex_0 = [0,0,0,0]` is the family's Fox default: converting
    // back to FMDL regenerates it verbatim, so its loss is silent. A
    // non-default `SelfColor` is still reported.
    let mut ir = ir_over(
        Vertices {
            positions: vec![[0.0; 3]],
            ..Vertices::default()
        },
        vec![[0, 0, 0]],
    );
    ir.materials[0].family = MaterialFamily::Shaded;
    ir.materials[0].fox = Some(FoxMaterial {
        shader: "fox3ddf_blin".to_string(),
        technique: "fox3DDF_Blin".to_string(),
        alpha_flags: 0,
        shadow_flags: 0,
        cast_shadow: None,
        invisible: None,
        base_linear: false,
        textures: vec![],
        parameters: vec![
            ("MatParamIndex_0".to_string(), [0.0, 0.0, 0.0, 0.0]),
            ("SelfColor".to_string(), [1.0, 1.0, 1.0, 1.0]),
        ],
    });
    let exported = ir_to_model(&ir).expect("export");
    assert_eq!(
        exported.findings,
        [Finding {
            code: "material_parameter_dropped",
            subject: Subject::Material(0),
            detail: "SelfColor".to_string(),
        }]
    );
}

/// A minimal consistent IR around `vertices`/`faces`: one bone, one mesh weighted
/// to it, one `Shaded` material, one group.
fn ir_over(vertices: Vertices, faces: Vec<[u16; 3]>) -> CanonicalModel {
    CanonicalModel {
        bones: vec![Bone {
            name: "sk_belly".to_string(),
            parent: None,
            matrix: Affine::IDENTITY,
            global_position: None,
            local_position: None,
            bounding_box: None,
        }],
        meshes: vec![Mesh {
            vertices,
            faces,
            bone_group: vec![0],
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
fn vertex_owners_come_from_the_ir_order() {
    // Vertices 1 and 2 share position and skinning, differ only in UV: one
    // topological vertex encoded as two loops.
    let ir = ir_over(
        Vertices {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            uvs: vec![vec![[0.0, 0.0], [0.0, 0.0], [0.5, 0.0], [0.0, 1.0]]],
            uv_high_precision: vec![false],
            bone_indices: Some(vec![[0, 0, 0, 0]; 4]),
            bone_weights: Some(vec![[1.0, 0.0, 0.0, 0.0]; 4]),
            ..Vertices::default()
        },
        vec![[0, 1, 3], [1, 2, 3]],
    );
    let exported = ir_to_model(&ir).expect("export");
    assert_eq!(exported.model.meshes.len(), 1);
    let owner =
        ::pes_model::ops::vertex_enc::decode(&exported.model.meshes[0]).expect("vertex decode");
    // The flag-gated `decode_model` returned identity owners [0, 1, 2, 3]; here
    // vertex 2 is the second loop of vertex 1.
    assert_eq!(owner, vec![0, 1, 1, 3]);
}

#[test]
fn a_non_unit_normal_or_tangent_w_reports() {
    let vertices = |normals, tangents| Vertices {
        positions: vec![[0.0; 3]; 3],
        normals,
        tangents,
        uvs: vec![vec![[0.0, 0.0]; 3]],
        uv_high_precision: vec![false],
        bone_indices: Some(vec![[0, 0, 0, 0]; 3]),
        bone_weights: Some(vec![[1.0, 0.0, 0.0, 0.0]; 3]),
        ..Vertices::default()
    };
    let finding = |detail: &str| Finding {
        code: "native_field_dropped",
        subject: Subject::Mesh(0),
        detail: detail.to_string(),
    };
    let normals = vertices(
        Some(vec![[0.0, 0.0, 1.0, 0.0]; 3]),
        Some(vec![[1.0, 0.0, 0.0, 1.0]; 3]),
    );
    assert_eq!(
        ir_to_model(&ir_over(normals, vec![[0, 1, 2]]))
            .expect("export")
            .findings,
        vec![finding("normal_w")]
    );
    let tangents = vertices(
        Some(vec![[0.0, 0.0, 1.0, 1.0]; 3]),
        Some(vec![[1.0, 0.0, 0.0, -1.0]; 3]),
    );
    assert_eq!(
        ir_to_model(&ir_over(tangents, vec![[0, 1, 2]]))
            .expect("export")
            .findings,
        vec![finding("tangent_w")]
    );
}

#[test]
fn a_stored_sampler_name_without_settings_reports() {
    // `prefox.textures` carries a sampler the `prefox.samplers` table never
    // defined: it exports with the default settings, and that is a finding.
    let mut ir = ir_over(
        Vertices {
            positions: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            ..Vertices::default()
        },
        vec![[0, 1, 2]],
    );
    ir.textures = vec![Texture {
        directory: "./".to_string(),
        file_name: "t.dds".to_string(),
    }];
    ir.materials[0].prefox = Some(PreFoxMaterial {
        shader: "Basic_C".to_string(),
        states: vec![],
        samplers: vec![],
        textures: vec![("ExtraMap".to_string(), 0)],
        parameters: vec![],
    });
    let exported = ir_to_model(&ir).expect("export");
    assert_eq!(
        exported.mtl.materials[0]
            .entries
            .iter()
            .filter_map(|entry| match entry {
                mtl::MaterialEntry::Sampler(sampler) => Some(sampler.name.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>(),
        ["ExtraMap"]
    );
    assert_eq!(
        exported.findings,
        vec![Finding {
            code: "sampler_settings_defaulted",
            subject: Subject::Material(0),
            detail: "ExtraMap".to_string(),
        }]
    );
}

#[test]
fn states_reach_the_canonical_flags() {
    // `two_sided`/`transparent` come from the material's state entries.
    let mut mtl = mtl_of("mat", "Basic_C");
    mtl.materials[0].entries = vec![
        mtl::MaterialEntry::State(mtl::State {
            name: "twosided".to_string(),
            value: 1,
        }),
        mtl::MaterialEntry::State(mtl::State {
            name: "alphablend".to_string(),
            value: 1,
        }),
    ];
    let input = pes_model(vec!["sk_belly"], pes_mesh(vec![0, 0]));
    let imported = model_to_ir(&input, &mtl).expect("import");
    assert_eq!(imported.model.materials[0].two_sided, Some(true));
    assert_eq!(imported.model.materials[0].transparent, Some(true));
}

#[test]
fn a_second_sampler_for_one_role_stays_native() {
    // The first `DiffuseMap` wins the canonical Base role; a second sampler
    // mapping to the same role is a native one.
    let sampler = |name: &str, path: &str| {
        mtl::MaterialEntry::Sampler(mtl::Sampler {
            name: name.to_string(),
            path: path.to_string(),
            srgb: None,
            minfilter: None,
            maxfilter: None,
            magfilter: None,
            mipfilter: None,
            uaddr: None,
            vaddr: None,
            waddr: None,
            maxaniso: None,
        })
    };
    let mut mtl = mtl_of("mat", "Basic_C");
    mtl.materials[0].entries = vec![
        sampler("DiffuseMap", "a.dds"),
        sampler("DiffuseMap", "b.dds"),
        sampler("NormalMap", "n.dds"),
    ];
    let input = pes_model(vec!["sk_belly"], pes_mesh(vec![0, 0]));
    let imported = model_to_ir(&input, &mtl).expect("import");
    let material = &imported.model.materials[0];
    assert_eq!(
        material.textures,
        vec![(TextureRole::Base, 0), (TextureRole::Normal, 2)]
    );
    assert_eq!(
        material.prefox.as_ref().expect("prefox").textures,
        vec![("DiffuseMap".to_string(), 0)]
    );
}

#[test]
fn a_stored_weight_width_is_kept() {
    // `synthesized` is indices-without-weights only; a stored column keeps
    // its own width.
    let mut mesh = pes_mesh(vec![0, 0]);
    mesh.vertices.bone_weight_width = 2;
    mesh.vertices.bone_weights = Some(vec![[1.0, 0.0, 0.0, 0.0]]);
    mesh.vertices.bone_indices = Some(vec![[0, 0, 0, 0]]);
    let input = pes_model(vec!["sk_belly"], mesh);
    let imported = model_to_ir(&input, &mtl_of("mat", "Basic_C")).expect("import");
    assert_eq!(imported.model.meshes[0].vertices.bone_weight_width, Some(2));
}

#[test]
fn a_weighted_slot_past_the_group_drops_and_reports() {
    // The stale weighted index drops, the rest of the row renormalizes,
    // and the drop is a finding.
    let input = pes_model(vec!["sk_belly"], pes_mesh(vec![0]));
    let imported = model_to_ir(&input, &mtl_of("mat", "Basic_C")).expect("import");
    assert_eq!(
        imported.model.meshes[0].vertices.bone_weights,
        Some(vec![[1.0, 0.0, 0.0, 0.0]])
    );
    assert_eq!(
        imported
            .findings
            .iter()
            .filter(|finding| finding.code == "bone_slot_dropped")
            .map(|finding| finding.detail.as_str())
            .collect::<Vec<_>>(),
        ["1"]
    );
}

#[test]
fn an_ordinary_extension_header_survives_import() {
    // Only the vertex-loop marker and Split-Mesh headers are stripped.
    let mut mesh = pes_mesh(vec![0, 0]);
    mesh.extension_headers = vec!["Custom-Header: value".to_string()];
    let input = pes_model(vec!["sk_belly"], mesh);
    let imported = model_to_ir(&input, &mtl_of("mat", "Basic_C")).expect("import");
    assert!(
        imported.model.meshes[0]
            .extension_headers
            .contains("Custom-Header: value")
    );
}
