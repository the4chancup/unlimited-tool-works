use super::*;

#[test]
fn highneck_imports_on_the_template_skeleton() {
    let imported = fmdl_to_ir(&read_fmdl(HIGHNECK), None).expect("import");
    let model = &imported.model;
    assert_eq!(model.bones.len(), 7);
    assert_eq!(model.meshes.len(), 1);
    assert_eq!(model.materials.len(), 1);
    let body = &skeletons(PesVersion::Pes21).body;
    for bone in &model.bones {
        let template = body.bone(&bone.name).expect("template bone");
        assert!(
            bone.matrix.max_component_delta(&template.matrix) < 1e-6,
            "{}",
            bone.name
        );
    }
    let material = &model.materials[0];
    assert_eq!(material.two_sided, Some(false));
    assert_eq!(
        material.fox.as_ref().expect("fox").shader,
        "pes_3ddf_basic_color_translucent"
    );
    // The file carries the redundant local-space bone-matrix block, and the
    // `translucent` shader is one of the rare names with no family.
    assert_eq!(
        imported.findings,
        vec![
            Finding {
                code: "native_field_dropped",
                subject: Subject::Model,
                detail: "bone_matrices".to_string(),
            },
            Finding {
                code: "material_family_approximated",
                subject: Subject::Material(0),
                detail: "accessory".to_string(),
            },
        ]
    );
}

#[test]
fn audience_import_reads_the_skl() {
    let input = read_fmdl(AU);
    let skl = ::fmdl::SklFile::read(AU_SKL).expect("skl");
    let imported = fmdl_to_ir(&input, Some(&skl)).expect("import");
    let ir_hand = imported
        .model
        .bones
        .iter()
        .find(|bone| bone.name == "sk_hand_l")
        .expect("bone");
    let skl_hand = skl
        .bones
        .iter()
        .find(|bone| bone.name == "sk_hand_l")
        .expect("skl bone");
    assert_eq!(ir_hand.matrix.translation(), skl_hand.translation);
    // The SKL pose is not the template's: `sk_hand_l` differs by ~0.50.
    let template = skeletons(PesVersion::Pes21)
        .body
        .bone("sk_hand_l")
        .expect("template bone");
    assert!(ir_hand.matrix.max_component_delta(&template.matrix) > 1e-3);
    // The fixture's own convention the exporter relies on: every child's
    // `local_position` is `global - parent.global`.
    for bone in &input.bones {
        let Some(parent) = bone.parent else { continue };
        for axis in 0..3 {
            let delta = bone.local_position[axis]
                - (bone.world_position[axis] - input.bones[parent].world_position[axis]);
            assert!(delta.abs() < 1e-5, "{} axis {axis}", bone.name);
        }
    }
}

#[test]
fn a_bone_slot_past_the_group_is_dropped() {
    let mesh = ::fmdl::Mesh {
        vertices: ::fmdl::format::MeshVertices {
            positions: vec![[0.0; 3]],
            bone_indices: Some(vec![[0, 3, 0, 0]]),
            bone_weights: Some(vec![[128, 127, 0, 0]]),
            ..::fmdl::format::MeshVertices::default()
        },
        faces: vec![[0, 0, 0]],
        bone_group: vec![0],
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    let imported = fmdl_to_ir(&fmdl_model(mesh, vec![instance("mat")]), None).expect("import");
    assert_eq!(
        imported.model.meshes[0].vertices.bone_weights,
        Some(vec![[1.0, 0.0, 0.0, 0.0]])
    );
    assert_eq!(
        imported.findings,
        vec![Finding {
            code: "bone_slot_dropped",
            subject: Subject::Mesh(0),
            detail: "1".to_string(),
        }]
    );
}

#[test]
fn a_zero_weight_slot_past_the_group_is_kept() {
    // Only a weighted stale slot drops; a zero-weight one is inert and the
    // vertex keeps the file's exact (unnormalized) weights.
    let mesh = ::fmdl::Mesh {
        vertices: ::fmdl::format::MeshVertices {
            positions: vec![[0.0; 3]],
            bone_indices: Some(vec![[0, 3, 0, 0]]),
            bone_weights: Some(vec![[128, 0, 0, 0]]),
            ..::fmdl::format::MeshVertices::default()
        },
        faces: vec![[0, 0, 0]],
        bone_group: vec![0],
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    let imported = fmdl_to_ir(&fmdl_model(mesh, vec![instance("mat")]), None).expect("import");
    assert_eq!(
        imported.model.meshes[0].vertices.bone_weights,
        Some(vec![[128.0 / 255.0, 0.0, 0.0, 0.0]])
    );
    assert_eq!(imported.findings, vec![]);
}

#[test]
fn a_vertex_that_loses_all_its_slots_keeps_zero_weights() {
    // Every weighted slot is out of group: they drop to an all-zero row,
    // not a division by the remaining zero sum.
    let mesh = ::fmdl::Mesh {
        vertices: ::fmdl::format::MeshVertices {
            positions: vec![[0.0; 3]],
            bone_indices: Some(vec![[1, 1, 1, 1]]),
            bone_weights: Some(vec![[255, 0, 0, 0]]),
            ..::fmdl::format::MeshVertices::default()
        },
        faces: vec![[0, 0, 0]],
        bone_group: vec![0],
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    let imported = fmdl_to_ir(&fmdl_model(mesh, vec![instance("mat")]), None).expect("import");
    assert_eq!(
        imported.model.meshes[0].vertices.bone_weights,
        Some(vec![[0.0, 0.0, 0.0, 0.0]])
    );
    assert_eq!(
        imported.findings,
        vec![Finding {
            code: "bone_slot_dropped",
            subject: Subject::Mesh(0),
            detail: "1".to_string(),
        }]
    );
}

#[test]
fn a_bone_listed_before_its_parent_moves_after_it() {
    let mesh = ::fmdl::Mesh {
        vertices: ::fmdl::format::MeshVertices {
            positions: vec![[0.0; 3]],
            bone_indices: Some(vec![[0, 1, 0, 0]]),
            bone_weights: Some(vec![[128, 127, 0, 0]]),
            ..::fmdl::format::MeshVertices::default()
        },
        faces: vec![[0, 0, 0]],
        bone_group: vec![0, 1],
        material: 0,
        alpha_flags: 0,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    let mut model = fmdl_model(mesh, vec![instance("mat")]);
    // File order: the child first, its parent second — the `body.skl` order.
    let parent_of = |name: &str, parent| ::fmdl::Bone {
        name: name.to_string(),
        parent,
        bounding_box: ::fmdl::BoundingBox {
            min: [0.0; 4],
            max: [0.0; 4],
        },
        local_position: [0.0; 4],
        world_position: [0.0; 4],
    };
    model.bones = vec![parent_of("sk_chest", Some(1)), parent_of("sk_belly", None)];
    let imported = fmdl_to_ir(&model, None).expect("import");
    assert_eq!(
        imported
            .model
            .bones
            .iter()
            .map(|bone| (bone.name.as_str(), bone.parent))
            .collect::<Vec<_>>(),
        [("sk_belly", None), ("sk_chest", Some(0))]
    );
    // The group remapped to the new order; the vertex slots did not move.
    assert_eq!(imported.model.meshes[0].bone_group, [1, 0]);
    assert_eq!(
        imported.model.meshes[0].vertices.bone_indices,
        Some(vec![[0, 1, 0, 0]])
    );
    // Export goes back through the same parents.
    let exported = ir_to_fmdl(&imported.model).expect("export");
    assert_eq!(exported.model.bones[1].parent, Some(0));
}

#[test]
fn a_bone_parent_cycle_stays_an_error() {
    let mut model = fmdl_model(fmdl_mesh(0, 0), vec![instance("mat")]);
    let parent_of = |name: &str, parent| ::fmdl::Bone {
        name: name.to_string(),
        parent,
        bounding_box: ::fmdl::BoundingBox {
            min: [0.0; 4],
            max: [0.0; 4],
        },
        local_position: [0.0; 4],
        world_position: [0.0; 4],
    };
    model.bones = vec![parent_of("a", Some(1)), parent_of("b", Some(0))];
    assert!(matches!(
        fmdl_to_ir(&model, None),
        Err(ConvertError::Fmdl(::fmdl::FmdlError::ParentCycle("bone")))
    ));
}

#[test]
fn a_group_that_stays_split_imports_as_an_ordinary_group() {
    let component = |offset: usize, count: usize, faces: usize| ::fmdl::Mesh {
        vertices: ::fmdl::format::MeshVertices {
            positions: (0..count)
                .map(|index| [(offset + index) as f32, 0.0, 0.0])
                .collect(),
            ..::fmdl::format::MeshVertices::default()
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
    // Two components referencing 70000 distinct vertices: u16 faces cannot
    // index the combined mesh, so the decode keeps the container split.
    let mut model = fmdl_model(component(0, 40000, 13333), vec![instance("mat")]);
    model.meshes.push(component(40000, 30000, 10000));
    model.mesh_groups[0].meshes = Vec::new();
    model.mesh_groups.push(::fmdl::MeshGroup {
        name: "split-mesh".to_string(),
        parent: Some(0),
        meshes: vec![0, 1],
        bounding_box: None,
        visible: true,
        split_mesh_group: true,
    });
    model.extensions.mesh_splitting = true;
    let imported = fmdl_to_ir(&model, None).expect("import");
    // The container reads as an ordinary group holding its two components.
    assert_eq!(imported.model.meshes.len(), 2);
    let group = imported
        .model
        .mesh_groups
        .iter()
        .find(|group| group.name == "split-mesh")
        .expect("the container's group");
    assert_eq!(group.meshes, vec![0, 1]);
    // Export and re-import keep working.
    let exported = ir_to_fmdl(&imported.model).expect("export");
    fmdl_to_ir(&exported.model, exported.skl.as_ref()).expect("reimport");
}

#[test]
fn a_dummy_map_is_the_game_s_by_directory_and_stem_whatever_the_case_and_extension() {
    let texture = |directory: &str, file_name: &str| ::fmdl::Texture {
        file_name: file_name.to_string(),
        directory: directory.to_string(),
    };
    let game = "/Assets/pes16/model/character/common/sourceimages/";
    let mut inst = instance("mat");
    inst.textures = vec![
        ("Base_Tex_SRGB".to_string(), texture("./", "t.dds")),
        (
            "NormalMap_Tex_NRM".to_string(),
            texture(game, "Dummy_NRM.tga"),
        ),
        (
            "SpecularMap_Tex_LIN".to_string(),
            texture("./", "dummy_srm.dds"),
        ),
    ];
    let imported = fmdl_to_ir(&fmdl_model(fmdl_mesh(0, 0), vec![inst]), None).expect("import");
    let material = &imported.model.materials[0];
    // A member's own `dummy_srm` beside the model is a real texture.
    assert_eq!(
        material.textures,
        vec![(TextureRole::Base, 0), (TextureRole::Specular, 2)]
    );
    assert_eq!(
        material.fox.as_ref().expect("fox").textures,
        vec![("NormalMap_Tex_NRM".to_string(), 1)]
    );
}

#[test]
fn one_instance_two_flag_sets_splits() {
    let mesh = |alpha| ::fmdl::Mesh {
        vertices: ::fmdl::format::MeshVertices {
            positions: vec![[0.0; 3]],
            ..::fmdl::format::MeshVertices::default()
        },
        faces: vec![[0, 0, 0]],
        bone_group: vec![],
        material: 0,
        alpha_flags: alpha,
        shadow_flags: 0,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    let mut model = fmdl_model(mesh(0), vec![instance("mat")]);
    model.meshes.push(mesh(32));
    model.mesh_groups[0].meshes.push(1);
    let imported = fmdl_to_ir(&model, None).expect("import");
    assert_eq!(imported.model.materials.len(), 2);
    assert_eq!(imported.model.materials[0].name, "mat");
    assert_eq!(imported.model.materials[0].two_sided, Some(false));
    assert_eq!(imported.model.materials[1].name, "mat_2");
    assert_eq!(imported.model.materials[1].two_sided, Some(true));
    assert_eq!(
        imported.findings,
        vec![Finding {
            code: "material_split_by_flags",
            subject: Subject::Material(1),
            detail: "mat_2".to_string(),
        }]
    );
}

#[test]
fn the_anti_blur_request_flag_is_no_split_reason() {
    // One instance behind two meshes differing only in the anti-blur
    // request flag: alpha and shadow flags equal, one IR material.
    let mut model = fmdl_model(fmdl_mesh(0, 0), vec![instance("mat")]);
    let mut asking = fmdl_mesh(0, 0);
    asking.has_antiblur_meshes = true;
    model.meshes.push(asking);
    model.mesh_groups[0].meshes = vec![0, 1];
    let imported = fmdl_to_ir(&model, None).expect("import");
    assert_eq!(
        imported
            .model
            .materials
            .iter()
            .map(|material| material.name.as_str())
            .collect::<Vec<_>>(),
        ["mat"]
    );
    assert_eq!(imported.model.meshes[0].material, 0);
    assert_eq!(imported.model.meshes[1].material, 0);
    assert!(imported.findings.is_empty());
}

#[test]
fn a_mesh_matches_its_own_flag_combination() {
    // Two flag combinations on one instance: a mesh's material is the
    // entry matching both flags, not the first sharing one.
    let mesh = |alpha, shadow| ::fmdl::Mesh {
        vertices: ::fmdl::format::MeshVertices {
            positions: vec![[0.0; 3]],
            ..::fmdl::format::MeshVertices::default()
        },
        faces: vec![[0, 0, 0]],
        bone_group: vec![],
        material: 0,
        alpha_flags: alpha,
        shadow_flags: shadow,
        has_antiblur_meshes: false,
        is_antiblur_mesh: false,
        custom_bounding_box: None,
    };
    let mut model = fmdl_model(mesh(1, 1), vec![instance("mat")]);
    model.meshes.push(mesh(0, 1));
    model.mesh_groups[0].meshes = vec![0, 1];
    let imported = fmdl_to_ir(&model, None).expect("import");
    assert_eq!(
        imported
            .model
            .materials
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>(),
        ["mat", "mat_2"]
    );
    assert_eq!(imported.model.meshes[0].material, 0);
    assert_eq!(imported.model.meshes[1].material, 1);
}

#[test]
fn a_second_sampler_for_one_role_goes_native() {
    // The first `Base_Tex_*` wins the canonical Base role; a second is a
    // native sampler, not a duplicate canonical entry.
    let mut inst = instance("mat");
    inst.textures = vec![
        (
            "Base_Tex_LIN".to_string(),
            ::fmdl::Texture {
                file_name: "a.tga".to_string(),
                directory: String::new(),
            },
        ),
        (
            "Base_Tex_SRGB".to_string(),
            ::fmdl::Texture {
                file_name: "b.tga".to_string(),
                directory: String::new(),
            },
        ),
    ];
    let imported = fmdl_to_ir(&fmdl_model(fmdl_mesh(0, 0), vec![inst]), None).expect("import");
    let material = &imported.model.materials[0];
    assert_eq!(material.textures, vec![(TextureRole::Base, 0)]);
    assert_eq!(
        material.fox.as_ref().expect("fox").textures,
        vec![("Base_Tex_SRGB".to_string(), 1)]
    );
}

#[test]
fn a_split_material_never_reuses_an_instance_name() {
    // `mat` splits on two flag combinations while `mat_2` already exists: the
    // generated name must skip the taken one.
    let mut model = fmdl_model(fmdl_mesh(0, 0), vec![instance("mat"), instance("mat_2")]);
    model.meshes.push(fmdl_mesh(32, 0));
    model.meshes.push(fmdl_mesh(0, 1));
    model.mesh_groups[0].meshes = vec![0, 1, 2];
    let imported = fmdl_to_ir(&model, None).expect("import");
    let names: Vec<&str> = imported
        .model
        .materials
        .iter()
        .map(|material| material.name.as_str())
        .collect();
    assert_eq!(names, ["mat", "mat_3", "mat_2"]);
    assert_eq!(
        imported.findings,
        vec![Finding {
            code: "material_split_by_flags",
            subject: Subject::Material(1),
            detail: "mat_3".to_string(),
        }]
    );
}

#[test]
fn bone_matrices_report_native_field_dropped() {
    let mut model = fmdl_model(fmdl_mesh(0, 0), vec![instance("mat")]);
    model.bone_matrices = Some(vec![0; 64]);
    let imported = fmdl_to_ir(&model, None).expect("import");
    assert_eq!(
        imported.findings,
        vec![Finding {
            code: "native_field_dropped",
            subject: Subject::Model,
            detail: "bone_matrices".to_string(),
        }]
    );
}

#[test]
fn a_disagreeing_skl_parent_reports_native_field_dropped() {
    // The SKL parents `sk_chest` under `sk_belly`; the FMDL leaves it a root and
    // the FMDL wins.
    let mut model = fmdl_model(fmdl_mesh(0, 0), vec![instance("mat")]);
    model.bones.push(::fmdl::Bone {
        name: "sk_chest".to_string(),
        parent: None,
        bounding_box: ::fmdl::BoundingBox {
            min: [0.0; 4],
            max: [0.0; 4],
        },
        local_position: [0.0; 4],
        world_position: [0.0; 4],
    });
    let skl = ::fmdl::SklFile::new(vec![
        ::fmdl::format::SklBone {
            name: "sk_belly".to_string(),
            parent: None,
            rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation: [0.0; 3],
        },
        ::fmdl::format::SklBone {
            name: "sk_chest".to_string(),
            parent: Some(0),
            rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation: [0.0; 3],
        },
    ]);
    let imported = fmdl_to_ir(&model, Some(&skl)).expect("import");
    assert_eq!(imported.model.bones[1].parent, None);
    assert_eq!(
        imported.findings,
        vec![Finding {
            code: "native_field_dropped",
            subject: Subject::Bone(1),
            detail: "skl_parent".to_string(),
        }]
    );
}

#[test]
fn out_of_range_parents_error() {
    let model = fmdl_model(fmdl_mesh(0, 0), vec![instance("mat")]);
    // An SKL bone parented past the skeleton's own bone count.
    let skl = ::fmdl::SklFile::new(vec![::fmdl::format::SklBone {
        name: "sk_belly".to_string(),
        parent: Some(5),
        rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        translation: [0.0; 3],
    }]);
    assert!(matches!(
        fmdl_to_ir(&model, Some(&skl)),
        Err(ConvertError::Fmdl(::fmdl::FmdlError::BadReference {
            what: "skl parent",
            index: 5
        }))
    ));
    // The same for the model's own parent index — caught by the model
    // validation `split::decode` runs before the importer's name lookup.
    let mut model = fmdl_model(fmdl_mesh(0, 0), vec![instance("mat")]);
    model.bones[0].parent = Some(7);
    assert!(matches!(
        fmdl_to_ir(&model, None),
        Err(ConvertError::Fmdl(::fmdl::FmdlError::BadReference {
            what: "bone",
            index: 7
        }))
    ));
}
