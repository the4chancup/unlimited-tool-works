use super::*;
use crate::format::datum::DatumType;
use crate::format::fixtures::*;

fn round_f32s(values: &[f32], places: u32) -> Vec<f32> {
    let scale = 10f32.powi(places as i32);
    values
        .iter()
        .map(|value| (value * scale).round() / scale)
        .collect()
}

fn field_kinds(geometry: &Geometry) -> Vec<(DatumType, DatumFormat, usize)> {
    geometry
        .vertex_fields
        .iter()
        .map(|field| (field.datum_type, field.datum_format, field.count()))
        .collect()
}

#[test]
fn every_fixture_round_trips_semantically() {
    for bytes in ALL {
        let container = ModelContainer::read(bytes).unwrap();
        let model = PreFoxModel::from_container(&container).unwrap();
        let written = model.to_container().unwrap();
        // The written file parses and has the export section order.
        assert_eq!(
            written.sections.iter().map(|s| s.kind).collect::<Vec<_>>(),
            [
                SectionKind::ModelBounds,
                SectionKind::BoneData,
                SectionKind::BoneNames,
                SectionKind::MaterialNames,
                SectionKind::AnnotationRecords,
                SectionKind::Cloth,
                SectionKind::MaterialCombinations,
                SectionKind::Locators,
                SectionKind::Geometry,
                SectionKind::AnnotationStrings,
                SectionKind::Meshes,
            ]
        );
        let reread_bytes = written.write();
        ModelContainer::read(&reread_bytes).unwrap();
        let reread = PreFoxModel::read(&reread_bytes).unwrap();
        // `write` always emits header version 19 — and therefore the
        // version-19 geometry layout, a missing order word becoming 0.
        let mut expected = model.clone();
        expected.version = 19;
        for geometry in &mut expected.geometries {
            if geometry.order.is_none() {
                geometry.order = Some(0);
            }
        }
        assert_eq!(reread, expected);
    }
    assert_eq!(PreFoxModel::read(CARDHEAD).unwrap().version, 19);
    // The version-17 file gains the order word 0 on write.
    let shadow = PreFoxModel::read(SHADOW).unwrap();
    assert_eq!(shadow.to_container().unwrap().version, 19);
    let shadow_bytes = shadow.to_container().unwrap().write();
    let shadow_reread = PreFoxModel::read(&shadow_bytes).unwrap();
    assert_eq!(shadow_reread.version, 19);
    assert!(
        shadow_reread
            .geometries
            .iter()
            .all(|geometry| geometry.order == Some(0))
    );
}

#[test]
fn bones_and_groups() {
    let cap = PreFoxModel::read(CAP).unwrap();
    assert_eq!(
        cap.bones
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
    assert_eq!(cap.bone_groups, [vec![0, 1, 3, 2]]);
    assert_eq!(
        round_f32s(&cap.bones[0].matrix, 6),
        [
            1.0, 0.0, 0.0, -0.105067, 0.0, 0.0, 1.0, -0.033469, 0.0, -1.0, 0.0, 1.467055
        ]
    );

    let flag = PreFoxModel::read(FLAG).unwrap();
    assert!(flag.bones.is_empty());
    assert!(flag.bone_groups.is_empty());
    assert_eq!(flag.meshes[0].bone_group, None);

    let shadow = PreFoxModel::read(SHADOW).unwrap();
    assert_eq!(shadow.bones.len(), 19);
    assert_eq!(
        shadow.bone_groups[0],
        [
            5, 4, 1, 2, 9, 8, 6, 3, 7, 10, 0, 13, 18, 14, 17, 11, 16, 12, 15
        ]
    );

    let hair = PreFoxModel::read(HAIR_HIGH).unwrap();
    assert_eq!(
        hair.bone_groups,
        [vec![4, 2, 1, 3, 10, 9, 7, 5, 8, 6, 0], vec![2, 0, 1, 3]]
    );
}

#[test]
fn geometry_fields_and_faces() {
    let cap = PreFoxModel::read(CAP).unwrap();
    let geometry = &cap.geometries[0];
    assert_eq!(
        field_kinds(geometry),
        [
            (DatumType::Position, DatumFormat::TripleFloat32, 56),
            (DatumType::Normal, DatumFormat::TripleFloat32, 56),
            (DatumType::Bitangent, DatumFormat::TripleFloat32, 56),
            (DatumType::Tangent, DatumFormat::TripleFloat32, 56),
            (DatumType::Uv0, DatumFormat::DoubleFloat32, 56),
            (DatumType::Uv1, DatumFormat::DoubleFloat32, 56),
            (DatumType::BoneIndices, DatumFormat::QuadInt8, 56),
            (DatumType::BoneWeights, DatumFormat::QuadFloat32, 56),
        ]
    );
    assert_eq!(geometry.faces.indices.len(), 234);
    assert_eq!(&geometry.faces.indices[..6], [0, 4, 1, 0, 1, 2]);
    assert!(geometry.faces.lod_ranges.is_empty());
    let first = floats_at::<3>(&geometry.vertex_fields[0].data, 0).unwrap();
    assert_eq!(round_f32s(&first, 5), [0.34969, 1.30588, -0.02537]);
    assert_eq!(
        round_f32s(&geometry.bounds.min, 4),
        [0.2507, 1.2711, -0.0383, 0.0]
    );
    assert_eq!(geometry.order, Some(0));

    let shadow = PreFoxModel::read(SHADOW).unwrap();
    assert_eq!(
        field_kinds(&shadow.geometries[0]),
        [
            (DatumType::Position, DatumFormat::TripleFloat32, 426),
            (DatumType::BoneIndices, DatumFormat::QuadInt8, 426),
            (DatumType::BoneWeights, DatumFormat::DoubleFloat32, 426),
        ]
    );
    assert_eq!(shadow.geometries[0].order, None);

    let taping = PreFoxModel::read(TAPING).unwrap();
    assert!(
        taping.geometries[0]
            .vertex_fields
            .iter()
            .any(|field| field.datum_type == DatumType::BoneWeights
                && field.datum_format == DatumFormat::TripleFloat32)
    );

    let hair = PreFoxModel::read(HAIR_HIGH).unwrap();
    assert!(field_kinds(&hair.geometries[0]).contains(&(
        DatumType::Color,
        DatumFormat::QuadFloat8,
        427
    )));
}

#[test]
fn lod_ranges_and_record() {
    let collar = PreFoxModel::read(COLLAR).unwrap();
    assert_eq!(collar.geometries[0].faces.indices.len(), 38496);
    assert_eq!(
        collar.geometries[0].faces.lod_ranges,
        [
            (0, 12240),
            (12240, 21510),
            (21510, 28353),
            (28353, 33231),
            (33231, 36510),
            (36510, 38496)
        ]
    );
    assert_eq!(collar.lod.level_count, 6);
    assert_eq!(collar.lod.parameters, [0.0625, 4.0, 0.3]);

    let card = PreFoxModel::read(CARD).unwrap();
    assert_eq!(card.lod.level_count, 0);
    assert_eq!(card.lod.parameters, [0.0625, 4.0, 0.0]);
    assert!(card.geometries[0].faces.lod_ranges.is_empty());
}

fn annotations(mesh: &Mesh) -> Vec<(usize, Option<usize>, u32, u32)> {
    mesh.annotations
        .iter()
        .map(|a| (a.string, a.record, a.unknown, a.kind))
        .collect()
}

#[test]
fn meshes_and_annotations() {
    let glasses = PreFoxModel::read(GLASSES).unwrap();
    assert_eq!(glasses.geometries.len(), 2);
    assert_eq!(glasses.meshes.len(), 2);
    assert_eq!(glasses.material_names, ["glasses_02T", "glasses_02C"]);
    assert_eq!(glasses.annotation_strings, ["glasses_02"]);
    assert_eq!(glasses.annotation_records.len(), 2);
    for record in &glasses.annotation_records {
        assert_eq!(record, &[0, 0, 0, 2, 2, 2, 0]);
    }
    let mesh = &glasses.meshes[0];
    assert_eq!(mesh.geometry, 0);
    assert_eq!(mesh.material, 0);
    assert_eq!(mesh.bone_group, Some(0));
    assert_eq!(annotations(mesh), [(0, Some(1), 7, 1), (0, Some(1), 7, 10)]);
    assert_eq!(mesh.editor_data, Some(vec![]));
    let mesh = &glasses.meshes[1];
    assert_eq!(mesh.geometry, 1);
    assert_eq!(mesh.material, 1);
    assert_eq!(mesh.bone_group, Some(1));
    assert_eq!(annotations(mesh), [(0, Some(0), 7, 1), (0, Some(0), 7, 10)]);

    let collar = PreFoxModel::read(COLLAR).unwrap();
    assert_eq!(collar.annotation_strings, ["prt", "DSpecularS", "DNormalS"]);
    assert_eq!(
        annotations(&collar.meshes[0]),
        [(0, Some(0), 7, 1), (1, Some(1), 7, 2), (2, Some(2), 7, 7)]
    );

    let head = PreFoxModel::read(HEAD_HI).unwrap();
    assert_eq!(
        annotations(&head.meshes[0]),
        [(0, Some(0), 7, 1), (1, Some(1), 7, 7)]
    );

    let cardhead = PreFoxModel::read(CARDHEAD).unwrap();
    assert_eq!(
        cardhead.annotation_strings,
        [
            "vertex-loop-preservation",
            "card",
            "Skeleton-Type: Simplified"
        ]
    );
    assert!(cardhead.annotation_records.is_empty());
    assert_eq!(
        annotations(&cardhead.meshes[0]),
        [(1, None, 7, 128), (0, None, 7, 129)]
    );
    assert_eq!(cardhead.meshes[0].editor_data, None);

    let shadow = PreFoxModel::read(SHADOW).unwrap();
    assert_eq!(shadow.meshes[0].editor_data, None);
    assert!(shadow.meshes[0].annotations.is_empty());

    let card = PreFoxModel::read(CARD).unwrap();
    assert_eq!(
        round_f32s(&card.bounds.min, 4),
        [-0.7339, 0.8666, 0.1158, 0.0]
    );
    assert_eq!(
        round_f32s(&card.bounds.max, 4),
        [-0.6078, 0.9905, 0.1463, 0.0]
    );
}

#[test]
fn editor_data_round_trips() {
    let mut model = PreFoxModel::read(CARD).unwrap();
    model.meshes[0].editor_data = Some(vec![
        EditorItem {
            kind: 1,
            unknown: 0,
            value: EditorValue::Text("TI_Util_CN_InfoNode".into()),
        },
        EditorItem {
            kind: 2,
            unknown: 0,
            value: EditorValue::Word(0),
        },
        EditorItem {
            kind: 3,
            unknown: 0,
            value: EditorValue::Word(0x13a9),
        },
    ]);
    let written = model.write().unwrap();
    assert_eq!(PreFoxModel::read(&written).unwrap(), model);
}

#[test]
fn a_text_item_is_nul_terminated() {
    // A text whose length is a multiple of 4 gets no pad byte after it:
    // only the item's own NUL keeps the read from scanning into the next
    // item's kind word.
    let mut model = PreFoxModel::read(CARD).unwrap();
    let text = "node".to_owned();
    assert_eq!(text.len() % 4, 0);
    model.meshes[0].editor_data = Some(vec![
        EditorItem {
            kind: 1,
            unknown: 0,
            value: EditorValue::Text(text),
        },
        EditorItem {
            kind: 2,
            unknown: 0,
            value: EditorValue::Word(0),
        },
    ]);
    let written = model.write().unwrap();
    assert_eq!(PreFoxModel::read(&written).unwrap(), model);
}

#[test]
fn unsupported_and_dangling_references_error() {
    // A non-empty section 8 is cloth, which the typed layer does not cover.
    let mut container = ModelContainer::read(CARD).unwrap();
    for section in &mut container.sections {
        if section.kind == SectionKind::Cloth {
            section.bytes = [12, 0, 0, 0, 1, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0].to_vec();
        }
    }
    assert_eq!(
        PreFoxModel::from_container(&container),
        Err(ModelError::Unsupported("cloth"))
    );

    // A material-combination extras word that is not `1,0,0,0`.
    let mut container = ModelContainer::read(CARD).unwrap();
    let geometry = &mut container
        .sections
        .iter_mut()
        .find(|section| section.kind == SectionKind::Geometry)
        .unwrap()
        .bytes;
    // The extras array's second entry: find the 16 bytes `1,0,0,0`.
    let needle = [1u32, 0, 0, 0]
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect::<Vec<u8>>();
    let at = geometry
        .windows(16)
        .position(|window| window == needle.as_slice())
        .unwrap();
    geometry[at..at + 4].copy_from_slice(&2u32.to_le_bytes());
    assert_eq!(
        PreFoxModel::from_container(&container),
        Err(ModelError::Unsupported("material combinations"))
    );

    let mut model = PreFoxModel::read(CARD).unwrap();
    model.meshes[0].material = 7;
    assert!(matches!(
        model.to_container(),
        Err(ModelError::BadReference {
            what: "material",
            ..
        })
    ));
    let mut model = PreFoxModel::read(CARD).unwrap();
    model.meshes[0].bone_group = Some(5);
    assert!(matches!(
        model.to_container(),
        Err(ModelError::BadReference {
            what: "bone group",
            ..
        })
    ));
    let mut model = PreFoxModel::read(CARD).unwrap();
    model.meshes[0].bone_group = Some(usize::MAX);
    assert!(matches!(
        model.to_container(),
        Err(ModelError::BadReference {
            what: "bone group",
            ..
        })
    ));
}

/// Patches one word of a card geometry-section record and returns the
/// re-read result. `pick` locates the word inside the section's bytes.
fn patch_geometry_word(
    pick: impl Fn(&[u8]) -> usize,
    value: u32,
) -> Result<PreFoxModel, ModelError> {
    let mut container = ModelContainer::read(CARD).unwrap();
    let geometry = &mut container
        .sections
        .iter_mut()
        .find(|section| section.kind == SectionKind::Geometry)
        .unwrap()
        .bytes;
    let at = pick(geometry);
    geometry[at..at + 4].copy_from_slice(&value.to_le_bytes());
    PreFoxModel::from_container(&container)
}

#[test]
fn hostile_geometry_counts_error() {
    use crate::format::records::RecordArray;
    // A vertex-field descriptor's count word at u32::MAX/2.
    assert_eq!(
        patch_geometry_word(
            |geometry| {
                let array = RecordArray::read(geometry, 0).unwrap();
                let vertex_set_offset =
                    u32::from_le_bytes(array.records[0].bytes[4..8].try_into().unwrap()) as usize;
                let vertex_set = RecordArray::read(geometry, vertex_set_offset).unwrap();
                let descriptors_offset =
                    u32::from_le_bytes(vertex_set.records[0].bytes[..4].try_into().unwrap())
                        as usize;
                let descriptors = RecordArray::read(geometry, descriptors_offset).unwrap();
                descriptors.records[0].offset + 12
            },
            0x4000_0000
        ),
        Err(ModelError::Truncated)
    );
    // The face descriptor's lod_levels at u32::MAX.
    assert_eq!(
        patch_geometry_word(
            |geometry| {
                let array = RecordArray::read(geometry, 0).unwrap();
                let face_descriptor_offset =
                    u32::from_le_bytes(array.records[0].bytes[8..12].try_into().unwrap()) as usize;
                let faces = RecordArray::read(geometry, face_descriptor_offset).unwrap();
                faces.records[0].offset + 16
            },
            u32::MAX
        ),
        Err(ModelError::Truncated)
    );
}

#[test]
fn editor_kind_and_value_must_agree() {
    let mut model = PreFoxModel::read(CARD).unwrap();
    model.meshes[0].editor_data = Some(vec![EditorItem {
        kind: 2,
        unknown: 0,
        value: EditorValue::Text("ABCD".into()),
    }]);
    assert_eq!(
        model.to_container(),
        Err(ModelError::WriteLayout(
            "editor item kind and value disagree"
        ))
    );
}

#[test]
fn annotation_with_dangling_record_pointer_is_dropped() {
    use crate::format::records::RecordArray;
    // A resolvable string pointer but a non-zero record pointer that
    // does not land on a section-3 record: the annotation is dropped,
    // the mesh's other annotations stay.
    let mut container = ModelContainer::read(GLASSES).unwrap();
    let delta = (container.section_offset(SectionKind::BoneData) as i64
        - container.section_offset(SectionKind::Meshes) as i64) as i32;
    let meshes = &mut container
        .sections
        .iter_mut()
        .find(|section| section.kind == SectionKind::Meshes)
        .unwrap()
        .bytes;
    let mesh_array = RecordArray::read(meshes, 0).unwrap();
    let annotation_offset =
        u32::from_le_bytes(mesh_array.records[0].bytes[8..12].try_into().unwrap()) as usize;
    let annotations_array = RecordArray::read(meshes, annotation_offset).unwrap();
    let at = annotations_array.records[0].offset + 4;
    meshes[at..at + 4].copy_from_slice(&delta.to_le_bytes());
    let model = PreFoxModel::from_container(&container).unwrap();
    assert_eq!(annotations(&model.meshes[0]), [(0, Some(1), 7, 10)]);
    assert_eq!(
        annotations(&model.meshes[1]),
        [(0, Some(0), 7, 1), (0, Some(0), 7, 10)]
    );
}

#[test]
fn invalid_utf8_string_reports_its_offset() {
    use crate::format::records::RecordArray;
    let mut container = ModelContainer::read(CARD).unwrap();
    let names = &mut container
        .sections
        .iter_mut()
        .find(|section| section.kind == SectionKind::MaterialNames)
        .unwrap()
        .bytes;
    let array = RecordArray::read(names, 0).unwrap();
    let offset = u32::from_le_bytes(array.records[0].bytes[..4].try_into().unwrap()) as usize;
    names[offset] = 0xFF;
    assert_eq!(
        PreFoxModel::from_container(&container),
        Err(ModelError::InvalidUtf8 { offset })
    );
}

#[test]
fn unexpected_constants_report_their_values() {
    use crate::format::records::RecordArray;
    // A bone-data entry whose header is not [2, 0, 0, 0] reports its
    // first word as the unexpected constant's value.
    let mut container = ModelContainer::read(CAP).unwrap();
    let bone_data = &mut container
        .sections
        .iter_mut()
        .find(|section| section.kind == SectionKind::BoneData)
        .unwrap()
        .bytes;
    let entries = RecordArray::read(bone_data, 0).unwrap();
    let entry = u32::from_le_bytes(entries.records[1].bytes[..4].try_into().unwrap()) as usize;
    bone_data[entry + 12] = 9;
    assert_eq!(
        PreFoxModel::from_container(&container),
        Err(ModelError::UnexpectedConstant {
            what: "bone data entry header",
            value: 9
        })
    );
}

#[test]
fn record_size_rules() {
    // Every section whose records the reader decodes checks their size.
    for (kind, size, what) in [
        (
            SectionKind::AnnotationRecords,
            24u32,
            "annotation record size",
        ),
        (SectionKind::Geometry, 16, "geometry record size"),
        (SectionKind::Meshes, 12, "mesh record size"),
    ] {
        let mut container = ModelContainer::read(CAP).unwrap();
        container
            .sections
            .iter_mut()
            .find(|section| section.kind == kind)
            .unwrap()
            .bytes[8..12]
            .copy_from_slice(&size.to_le_bytes());
        assert_eq!(
            PreFoxModel::from_container(&container),
            Err(ModelError::UnexpectedConstant { what, value: size })
        );
    }
}

#[test]
fn bone_matrix_record_needs_kind_and_count() {
    use crate::format::records::RecordArray;
    // The right kind word with a wrong count still fails the check.
    let mut container = ModelContainer::read(CAP).unwrap();
    let bone_data = &mut container
        .sections
        .iter_mut()
        .find(|section| section.kind == SectionKind::BoneData)
        .unwrap()
        .bytes;
    let entries = RecordArray::read(bone_data, 0).unwrap();
    let entry = u32::from_le_bytes(entries.records[0].bytes[..4].try_into().unwrap()) as usize;
    let matrices = RecordArray::read(bone_data, entry).unwrap();
    let count_at = matrices.records[0].offset + 8;
    bone_data[count_at..count_at + 4].copy_from_slice(&2u32.to_le_bytes());
    assert_eq!(
        PreFoxModel::from_container(&container),
        Err(ModelError::UnexpectedConstant {
            what: "bone matrix record",
            value: 7
        })
    );
}

#[test]
fn empty_array_with_header_past_the_buffer_reads() {
    // CARD's section 8 (Cloth) is an empty array; community exports can
    // carry a garbage first_record_offset there. With zero records the
    // array reads as having no header, so the file reads identically.
    let mut bytes = CARD.to_vec();
    let cloth = ModelContainer::read(&bytes)
        .unwrap()
        .section_offset(SectionKind::Cloth);
    bytes[cloth..cloth + 4].copy_from_slice(&0x6E6F_6974u32.to_le_bytes());
    assert_eq!(PreFoxModel::read(&bytes), PreFoxModel::read(CARD));
}
