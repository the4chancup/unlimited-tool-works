use super::*;

#[test]
fn an_exported_skl_carries_each_bones_ir_matrix() {
    let skl = ::fmdl::SklFile::read(AU_SKL).expect("skl");
    let imported = fmdl_to_ir(&read_fmdl(AU), Some(&skl)).expect("import");
    let exported = ir_to_fmdl(&imported.model).expect("export");
    let exported_skl = exported.skl.expect("skl");
    for bone in &exported_skl.bones {
        let matrix = imported
            .model
            .bones
            .iter()
            .find(|b| b.name == bone.name)
            .map(|b| b.matrix)
            .unwrap_or_else(|| panic!("SKL bone {} has no IR bone", bone.name));
        assert_eq!(bone.rotation, matrix.rotation(), "bone {}", bone.name);
        assert_eq!(bone.translation, matrix.translation(), "bone {}", bone.name);
    }
}

#[test]
fn a_vertexless_mesh_gets_an_empty_bone_group() {
    // A face export's marker mesh: no vertices or faces, a 33-entry bone
    // group holding the whole skeleton as data — over the format's 32.
    let mut ir = minimal_ir();
    ir.bones = (0..33)
        .map(|index| Bone {
            name: format!("b{index}"),
            parent: None,
            matrix: Affine::IDENTITY,
            global_position: None,
            local_position: None,
            bounding_box: None,
        })
        .collect();
    ir.meshes[0].vertices = Vertices {
        bone_indices: Some(vec![]),
        bone_weights: Some(vec![]),
        ..Vertices::default()
    };
    ir.meshes[0].faces = vec![];
    ir.meshes[0].bone_group = (0..33).collect();
    let exported = ir_to_fmdl(&ir).expect("export");
    assert_eq!(exported.model.meshes[0].bone_group, Vec::<usize>::new());
    assert_eq!(
        exported
            .findings
            .iter()
            .filter(|f| f.code == "empty_mesh_bone_group_dropped")
            .map(|f| (f.subject.clone(), f.detail.as_str()))
            .collect::<Vec<_>>(),
        [(Subject::Mesh(0), "33")]
    );
    // It writes and re-imports — before, to_file met the 33-entry group.
    let bytes = exported.model.to_file().expect("to_file").write();
    let model = ::fmdl::Model::from_file(&::fmdl::FmdlFile::read(&bytes).expect("read"))
        .expect("from_file");
    fmdl_to_ir(&model, exported.skl.as_ref()).expect("reimport");
}

#[test]
fn missing_maps_get_the_game_dummies() {
    let mut ir = minimal_ir();
    ir.textures = vec![Texture {
        directory: "./".to_string(),
        file_name: "t.dds".to_string(),
    }];
    ir.materials[0].textures = vec![(TextureRole::Base, 0)];
    ir.meshes[0].vertices.bitangents = Some(vec![[0.0; 3]]);
    let exported = ir_to_fmdl(&ir).expect("export");
    let textures = &exported.model.materials[0].textures;
    assert_eq!(
        textures
            .iter()
            .map(|(sampler, texture)| (sampler.as_str(), texture.file_name.as_str()))
            .collect::<Vec<_>>(),
        [
            ("Base_Tex_SRGB", "t.dds"),
            ("NormalMap_Tex_NRM", "dummy_nrm.dds"),
            ("SpecularMap_Tex_LIN", "dummy_srm.dds"),
        ]
    );
    assert_eq!(
        exported.findings,
        vec![
            Finding {
                code: "dummy_texture_added",
                subject: Subject::Material(0),
                detail: "NormalMap_Tex_NRM".to_string(),
            },
            Finding {
                code: "dummy_texture_added",
                subject: Subject::Material(0),
                detail: "SpecularMap_Tex_LIN".to_string(),
            },
            Finding {
                code: "vertex_bitangents_dropped",
                subject: Subject::Mesh(0),
                detail: String::new(),
            },
            Finding {
                code: "static_bone_added",
                subject: Subject::Mesh(0),
                detail: String::new(),
            },
        ]
    );
}

#[test]
fn an_export_writes_no_anti_blur_meshes() {
    // A Shadeless material and a Fox import whose mesh asks for the
    // anti-blur duplicates both export clean: no anti-blur mesh, neither
    // flag set, the `antiblur` extension absent. The vertex-loop
    // convention every export encodes is still declared.
    let no_antiblur = |model: &::fmdl::Model| {
        assert!(!model.extensions.antiblur);
        assert!(model.meshes.iter().all(|mesh| !mesh.has_antiblur_meshes));
        assert!(model.meshes.iter().all(|mesh| !mesh.is_antiblur_mesh));
        assert!(
            model
                .materials
                .iter()
                .all(|material| !material.shader.contains("fuzzblock"))
        );
    };
    let mut ir = minimal_ir();
    ir.materials[0].family = MaterialFamily::Shadeless;
    let exported = ir_to_fmdl(&ir).expect("export");
    let bytes = exported.model.to_file().expect("to_file").write();
    let model = read_fmdl(&bytes);
    no_antiblur(&model);
    assert!(model.extensions.vertex_loop_preservation);
    assert_eq!(model.meshes.len(), 1);
    assert_eq!(model.materials.len(), 1);

    let mut fox = fmdl_model(fmdl_mesh(0, 0), vec![instance("mat")]);
    fox.meshes[0].has_antiblur_meshes = true;
    let imported = fmdl_to_ir(&fox, None).expect("import");
    let exported = ir_to_fmdl(&imported.model).expect("export");
    let bytes = exported.model.to_file().expect("to_file").write();
    no_antiblur(&read_fmdl(&bytes));
}

#[test]
fn quantization_preserves_the_total() {
    assert_eq!(quantize_weights([0.5, 0.5, 0.0, 0.0]), [128, 127, 0, 0]);
    let weights = quantize_weights([1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0, 0.0]);
    assert_eq!(weights.iter().map(|w| u32::from(*w)).sum::<u32>(), 255);
}

#[test]
fn quantization_rounds_the_largest_weights_first() {
    // Three nonzero weights out of slot order: the biggest lane's share is
    // rounded against the whole total first — in slot order the lightest
    // lane would keep the rounding byte the heaviest earns.
    assert_eq!(quantize_weights([0.1, 0.9, 0.5, 0.0]), [25, 230, 128, 0]);
}

#[test]
fn quantization_preserves_the_total_past_one() {
    // A vertex the IR accepted unnormalized keeps `round(sum * 255)`
    // across the positive lanes, each at most 255.
    assert_eq!(quantize_weights([0.7, 0.0, 0.0, 0.0]), [178, 0, 0, 0]);
    assert_eq!(quantize_weights([1.0, 1.0, 0.0, 0.0]), [255, 255, 0, 0]);
    // One lane alone can hold 255; the rest of its total has nowhere to
    // go — never onto a zero-weight lane.
    assert_eq!(quantize_weights([2.0, 0.0, 0.0, 0.0]), [255, 0, 0, 0]);
    assert_eq!(quantize_weights([1.5, 0.0, 0.0, 0.0]), [255, 0, 0, 0]);
    assert_eq!(quantize_weights([1.0000001, 0.0, 0.0, 0.0]), [255, 0, 0, 0]);
    assert_eq!(quantize_weights([2.0, 2.0, 2.0, 2.0]), [255, 255, 255, 255]);
    // The clamped lanes keep *their* total: `round(1.5 * 255)` = 383 lands as
    // `round(1.0 / 1.5 * 383)` = 255 then `round(0.5 / 0.5 * 128)` = 128 —
    // the excess does not carry onto the second lane.
    assert_eq!(quantize_weights([1.5, 0.5, 0.0, 0.0]), [255, 128, 0, 0]);
}

#[test]
fn a_lane_above_one_reports_weight_clamped() {
    let skinned = |weights: [f32; 4]| CanonicalModel {
        bones: vec![Bone {
            name: "sk_belly".to_string(),
            parent: None,
            matrix: Affine::IDENTITY,
            global_position: None,
            local_position: None,
            bounding_box: None,
        }],
        meshes: vec![Mesh {
            vertices: Vertices {
                positions: vec![[0.0; 3]],
                bone_indices: Some(vec![[0, 0, 0, 0]]),
                bone_weights: Some(vec![weights]),
                ..Vertices::default()
            },
            faces: vec![[0, 0, 0]],
            bone_group: vec![0],
            material: 0,
            extension_headers: Default::default(),
            custom_bounding_box: None,
        }],
        ..minimal_ir()
    };
    let clamped = |weights| {
        ir_to_fmdl(&skinned(weights))
            .expect("export")
            .findings
            .iter()
            .filter(|f| f.code == "weight_clamped")
            .map(|f| (f.subject.clone(), f.detail.clone()))
            .collect::<Vec<_>>()
    };
    let exported = ir_to_fmdl(&skinned([1.5, 0.0, 0.0, 0.0])).expect("export");
    assert_eq!(
        exported.model.meshes[0].vertices.bone_weights,
        Some(vec![[255, 0, 0, 0]])
    );
    assert_eq!(
        clamped([1.5, 0.0, 0.0, 0.0]),
        [(Subject::Mesh(0), "1".to_string())]
    );
    // `[1.5, 0.5]` quantizes to `[255, 128]` — the clamped lanes' total —
    // and still reports the lane above one.
    let exported = ir_to_fmdl(&skinned([1.5, 0.5, 0.0, 0.0])).expect("export");
    assert_eq!(
        exported.model.meshes[0].vertices.bone_weights,
        Some(vec![[255, 128, 0, 0]])
    );
    assert_eq!(
        clamped([1.5, 0.5, 0.0, 0.0]),
        [(Subject::Mesh(0), "1".to_string())]
    );
    // Float noise at and below the threshold clamps silently.
    let exported = ir_to_fmdl(&skinned([1.0000001, 0.0, 0.0, 0.0])).expect("export");
    assert_eq!(
        exported.model.meshes[0].vertices.bone_weights,
        Some(vec![[255, 0, 0, 0]])
    );
    assert!(clamped([1.0000001, 0.0, 0.0, 0.0]).is_empty());
    assert!(clamped([1.0 + 1e-6, 0.0, 0.0, 0.0]).is_empty());
    // Unnormalized but under 1 clamps nothing.
    let exported = ir_to_fmdl(&skinned([0.7, 0.0, 0.0, 0.0])).expect("export");
    assert_eq!(
        exported.model.meshes[0].vertices.bone_weights,
        Some(vec![[178, 0, 0, 0]])
    );
    assert!(clamped([0.7, 0.0, 0.0, 0.0]).is_empty());
}

#[test]
fn a_default_valued_prefox_parameter_is_not_a_finding() {
    // `MatParamIndex_0 = [0,0,0,0]` is the family's Fox default: the resolver
    // regenerates it, so its `.mtl` name dropping carries no loss. Metal's
    // `Reflection` is likewise what a `.mtl` back-conversion regenerates.
    // A non-default `SelfColor` is still reported.
    let mut ir = minimal_ir();
    ir.materials[0].family = MaterialFamily::Metal;
    ir.materials[0].prefox = Some(crate::materials::PreFoxMaterial {
        shader: "Basic_CNSR".to_string(),
        states: vec![],
        samplers: vec![],
        textures: vec![],
        parameters: vec![
            ("MatParamIndex_0".to_string(), vec![0.0, 0.0, 0.0, 0.0]),
            ("Reflection".to_string(), vec![1.0, 1.0, 1.0, 0.0]),
            // A default name with a *different* value is a real loss.
            ("Shininess".to_string(), vec![9.0, 9.0, 9.0, 9.0]),
            ("SelfColor".to_string(), vec![1.0, 1.0, 1.0, 1.0]),
        ],
    });
    let exported = ir_to_fmdl(&ir).expect("export");
    assert_eq!(
        exported
            .findings
            .iter()
            .filter(|f| f.code == "material_parameter_dropped")
            .map(|f| (f.subject.clone(), f.detail.clone()))
            .collect::<Vec<_>>(),
        [
            (Subject::Material(0), "Shininess".to_string()),
            (Subject::Material(0), "SelfColor".to_string()),
        ]
    );
}

#[test]
fn uncarried_prefox_samplers_and_parameters_are_findings() {
    // `konami_headHi`'s native `.mtl` samplers `Normal2` and `Mapping` bind
    // textures the Fox resolver does not carry; the canonical `NormalMap`
    // maps to `NormalMap_Tex_NRM` and is not reported.
    let file = ::pes_model::format::PreFoxModel::read(HEAD_HI).expect("read");
    let model = ::pes_model::model::Model::from_file(&file).expect("model");
    let mtl = ::pes_model::format::mtl::MaterialSet::read(HEAD_HI_MTL).expect("mtl");
    let ir = crate::formats::pes_model::model_to_ir(&model, &mtl)
        .expect("import")
        .model;
    let exported = ir_to_fmdl(&ir).expect("export");
    assert_eq!(
        exported.findings,
        [
            Finding {
                code: "material_texture_unused",
                subject: Subject::Material(0),
                detail: "Normal2".to_string(),
            },
            Finding {
                code: "material_texture_unused",
                subject: Subject::Material(0),
                detail: "Mapping".to_string(),
            },
            Finding {
                code: "dummy_texture_added",
                subject: Subject::Material(0),
                detail: "SpecularMap_Tex_LIN".to_string(),
            },
            Finding {
                code: "vertex_bitangents_dropped",
                subject: Subject::Mesh(0),
                detail: String::new(),
            },
        ]
    );
}

#[test]
fn exported_bone_bounding_boxes_cover_their_weighted_vertices() {
    // A bone's SKL box spans the vertices weighted to it: unweighted
    // slots and other bones' vertices do not reach in.
    let mut ir = minimal_ir();
    ir.bones = vec![
        Bone {
            name: "sk_a".to_string(),
            parent: None,
            matrix: Affine::IDENTITY,
            global_position: None,
            local_position: None,
            bounding_box: None,
        },
        Bone {
            name: "sk_b".to_string(),
            parent: None,
            matrix: Affine::IDENTITY,
            global_position: None,
            local_position: None,
            bounding_box: None,
        },
    ];
    ir.meshes = vec![Mesh {
        vertices: Vertices {
            positions: vec![
                [1.0, 2.0, 3.0],
                [4.0, 5.0, 6.0],
                [99.0, 99.0, 99.0],
                [-50.0, -50.0, -50.0],
            ],
            bone_indices: Some(vec![[0, 0, 0, 0]; 4]),
            bone_weights: Some(vec![
                [1.0, 0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
            ]),
            bone_weight_width: Some(4),
            ..Vertices::default()
        },
        faces: vec![[0, 1, 3]],
        bone_group: vec![0, 1],
        material: 0,
        extension_headers: Default::default(),
        custom_bounding_box: None,
    }];
    // v3 weights slot 1 -> bone group[1] = bone 1; bone_indices slot 0
    // is a decoy pointing at group[0] on an unweighted slot.
    ir.meshes[0]
        .vertices
        .bone_indices
        .as_mut()
        .expect("indices")[3] = [0, 1, 0, 0];
    let exported = ir_to_fmdl(&ir).expect("export");
    assert_eq!(
        exported.model.bones[0].bounding_box.min,
        [1.0, 2.0, 3.0, 1.0]
    );
    assert_eq!(
        exported.model.bones[0].bounding_box.max,
        [4.0, 5.0, 6.0, 1.0]
    );
    assert_eq!(
        exported.model.bones[1].bounding_box.min,
        [-50.0, -50.0, -50.0, 1.0]
    );
    assert_eq!(
        exported.model.bones[1].bounding_box.max,
        [-50.0, -50.0, -50.0, 1.0]
    );
}

#[test]
fn split_parents_uses_the_render_parent_name() {
    let bone = |name: &str, parent: Option<usize>| ::fmdl::Bone {
        name: name.to_string(),
        parent,
        bounding_box: ::fmdl::BoundingBox {
            min: [0.0; 4],
            max: [0.0; 4],
        },
        local_position: [0.0; 4],
        world_position: [0.0; 4],
    };
    let parents = split_parents(&[bone("dsk_hip", None), bone("sk_belly", None)]);
    assert_eq!(parents, vec![None, Some(0)]);
}

#[test]
fn exported_local_position_is_world_minus_parent_world() {
    let mut ir = minimal_ir();
    ir.bones = vec![
        Bone {
            name: "sk_a".to_string(),
            parent: None,
            matrix: Affine::IDENTITY,
            global_position: Some([10.0, 10.0, 10.0, 1.0]),
            local_position: None,
            bounding_box: None,
        },
        Bone {
            name: "sk_b".to_string(),
            parent: Some(0),
            matrix: Affine::IDENTITY,
            global_position: Some([11.0, 13.0, 15.0, 1.0]),
            local_position: None,
            bounding_box: None,
        },
    ];
    let exported = ir_to_fmdl(&ir).expect("export");
    assert_eq!(exported.model.bones[1].local_position, [1.0, 3.0, 5.0, 1.0]);
    assert_eq!(exported.model.bones[0].local_position, [0.0, 0.0, 0.0, 1.0]);
}

/// A skinned IR whose vertices 1 and 2 share position and skinning and differ only
/// in UV: one topological vertex encoded as two loops.
fn seam_ir() -> CanonicalModel {
    let mut ir = minimal_ir();
    ir.bones = vec![Bone {
        name: "sk_belly".to_string(),
        parent: None,
        matrix: Affine::IDENTITY,
        global_position: None,
        local_position: None,
        bounding_box: None,
    }];
    ir.source_format = SourceFormat::Fox;
    let mesh = &mut ir.meshes[0];
    mesh.vertices = Vertices {
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
    };
    mesh.faces = vec![[0, 1, 3], [1, 2, 3]];
    mesh.bone_group = vec![0];
    ir
}

#[test]
fn vertex_owners_come_from_the_ir_order() {
    let exported = ir_to_fmdl(&seam_ir()).expect("export");
    assert_eq!(exported.model.meshes.len(), 1);
    let owner = ::fmdl::ops::vertex_enc::decode(&exported.model.meshes[0]).expect("decode");
    // The flag-gated `decode_model` returned identity owners [0, 1, 2, 3]; here
    // vertex 2 is the second loop of vertex 1.
    assert_eq!(owner, vec![0, 1, 1, 3]);
}
