use super::*;

#[test]
fn fmdl_round_trip() {
    let skl = ::fmdl::SklFile::read(AU_SKL).expect("skl");
    for (input, skl, has_unknown_bone) in [
        (read_fmdl(HIGHNECK), None, false),
        (read_fmdl(ORAL), None, false),
        (read_fmdl(AU), Some(&skl), true),
    ] {
        let imported = fmdl_to_ir(&input, skl).expect("import");
        let exported = ir_to_fmdl(&imported.model).expect("export");
        let expected = encoded(&decoded(&input));
        assert_eq!(exported.model.bones, expected.bones);
        assert_eq!(exported.model.materials, expected.materials);
        assert_eq!(exported.model.meshes, expected.meshes);
        assert_eq!(exported.model.extensions, expected.extensions);
        assert_eq!(exported.model.mesh_groups.len(), expected.mesh_groups.len());
        for (got, want) in exported.model.mesh_groups.iter().zip(&expected.mesh_groups) {
            assert_eq!(
                (
                    &got.name,
                    got.parent,
                    &got.meshes,
                    got.visible,
                    got.split_mesh_group
                ),
                (
                    &want.name,
                    want.parent,
                    &want.meshes,
                    want.visible,
                    want.split_mesh_group
                )
            );
        }
        assert_eq!(exported.skl.is_some(), has_unknown_bone);
        assert_eq!(exported.findings, Vec::<Finding>::new());
    }
}

#[test]
fn ir_is_a_fixed_point_after_one_encode() {
    for bytes in [HIGHNECK, ORAL] {
        let ir1 = fmdl_to_ir(&read_fmdl(bytes), None).expect("import").model;
        let ir2 = fmdl_to_ir(&ir_to_fmdl(&ir1).expect("export").model, None)
            .expect("reimport")
            .model;
        let ir3 = fmdl_to_ir(&ir_to_fmdl(&ir2).expect("export").model, None)
            .expect("reimport")
            .model;
        assert_eq!(ir2, ir3);
        assert_eq!(ir1.bones, ir2.bones);
        assert_eq!(ir1.materials, ir2.materials);
        assert_eq!(ir1.textures, ir2.textures);
        assert_eq!(ir1.mesh_groups, ir2.mesh_groups);
        assert_eq!(ir1.extension_headers, ir2.extension_headers);
        for (before, after) in ir1.meshes.iter().zip(&ir2.meshes) {
            assert_eq!(
                before.vertices.positions.len(),
                after.vertices.positions.len()
            );
            assert_eq!(before.faces.len(), after.faces.len());
            assert_eq!(before.bone_group, after.bone_group);
            assert_eq!(before.material, after.material);
        }
    }
}

#[test]
fn the_game_s_dummy_maps_import_as_the_role_absent_and_export_again_unchanged() {
    let mut ir = minimal_ir();
    ir.textures = vec![Texture {
        directory: "./".to_string(),
        file_name: "t.dds".to_string(),
    }];
    ir.materials[0].textures = vec![(TextureRole::Base, 0)];
    let first = ir_to_fmdl(&ir).expect("export").model;
    let imported = fmdl_to_ir(&first, None).expect("import").model;
    let material = &imported.materials[0];
    assert_eq!(material.textures, vec![(TextureRole::Base, 0)]);
    // The dummies stay native samplers, which only a Fox export writes.
    assert_eq!(
        material.fox.as_ref().expect("fox").textures,
        vec![
            ("NormalMap_Tex_NRM".to_string(), 1),
            ("SpecularMap_Tex_LIN".to_string(), 2),
        ]
    );
    let second = ir_to_fmdl(&imported).expect("export again").model;
    assert_eq!(second.materials[0].textures, first.materials[0].textures);
}

#[test]
fn every_mesh_hidden_is_the_pre_fox_export_s_every_mesh_hidden() {
    let flagged = |bytes: &[u8], meshes: &[usize]| {
        let mut model = read_fmdl(bytes);
        for &index in meshes {
            model.meshes[index].shadow_flags |= crate::materials::to_fox::INVISIBLE_BIT;
        }
        model
    };
    let mut unhidden = read_fmdl(ORAL);
    for mesh in &mut unhidden.meshes {
        mesh.shadow_flags &= !crate::materials::to_fox::INVISIBLE_BIT;
    }
    // Konami's parts model, whose several meshes the skeleton binds.
    let skl = ::fmdl::SklFile::read(AU_SKL).expect("skl");
    let au_meshes: Vec<usize> = (0..read_fmdl(AU).meshes.len()).collect();
    assert!(au_meshes.len() > 1);
    for (model, skl, hidden) in [
        (read_fmdl(ORAL), None, true),
        (unhidden, None, false),
        (read_fmdl(HIGHNECK), None, false),
        (read_fmdl(AU), Some(&skl), false),
        (flagged(AU, &au_meshes[1..]), Some(&skl), false),
        (flagged(AU, &au_meshes), Some(&skl), true),
    ] {
        assert_eq!(every_mesh_hidden(&model), hidden);
        let ir = fmdl_to_ir(&model, skl).expect("import").model;
        assert_eq!(
            matches!(
                crate::formats::pes_model::ir_to_model(&ir),
                Err(ConvertError::EveryMeshHidden)
            ),
            hidden
        );
    }
    let mut empty = read_fmdl(ORAL);
    empty.meshes.clear();
    assert!(!every_mesh_hidden(&empty));
}
