//! Writing a [`PreFoxModel`] back out: lays a fresh file out in the export
//! layout — sections in the order ModelBounds, BoneData, BoneNames,
//! MaterialNames, AnnotationRecords, Cloth, MaterialCombinations, Locators,
//! Geometry, AnnotationStrings, Meshes, each padded to 4, the version-19
//! record sizes throughout, no alignment gaps.

use crate::format::datum::DatumFormat;
use crate::format::model::{EditorValue, Geometry, Mesh, PreFoxModel};
use crate::format::{ModelContainer, ModelError, Section, SectionKind};

/// Bytes needed to bring `len` up to a multiple of 4.
fn pad4(len: usize) -> usize {
    (4 - len % 4) % 4
}

/// A record array being built: a 12-byte table of contents, a header, the
/// declared number of fixed-size records, then the blobs the records point
/// at, each padded to 4. Offsets handed out are relative to the array's own
/// start.
struct SectionBuilder {
    record_size: usize,
    record_count: usize,
    header: Vec<u8>,
    records: Vec<u8>,
    blobs: Vec<u8>,
}

impl SectionBuilder {
    fn new(record_size: usize, record_count: usize, header: &[u8]) -> Self {
        SectionBuilder {
            record_size,
            record_count,
            header: header.to_vec(),
            records: Vec::new(),
            blobs: Vec::new(),
        }
    }

    /// Where the record region starts (after the table of contents and the
    /// header).
    fn records_start(&self) -> usize {
        12 + self.header.len()
    }

    /// Where the blob region starts (after the declared records).
    /// Saturating: `finish` rejects the impossible product anyway.
    fn blob_base(&self) -> usize {
        self.records_start()
            .saturating_add(self.record_count.saturating_mul(self.record_size))
    }

    /// Appends a `record_size`-byte record; its offset from the array
    /// start.
    fn add_record(&mut self, record: &[u8]) -> u32 {
        let offset = self.records_start() + self.records.len();
        self.records.extend_from_slice(record);
        offset as u32
    }

    /// Appends a blob after the record region, zero-padded to 4; its
    /// offset from the array start.
    fn add_blob(&mut self, blob: &[u8]) -> u32 {
        let offset = self.blob_base() + self.blobs.len();
        self.blobs.extend_from_slice(blob);
        self.blobs.resize(self.blobs.len() + pad4(blob.len()), 0);
        offset as u32
    }

    /// The finished array's bytes.
    fn finish(self) -> Result<Vec<u8>, ModelError> {
        if self.record_count.checked_mul(self.record_size) != Some(self.records.len()) {
            return Err(ModelError::WriteLayout(
                "record count differs from declared",
            ));
        }
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(self.records_start() as u32).to_le_bytes());
        bytes.extend_from_slice(&(self.record_count as u32).to_le_bytes());
        bytes.extend_from_slice(&(self.record_size as u32).to_le_bytes());
        bytes.extend_from_slice(&self.header);
        bytes.extend_from_slice(&self.records);
        bytes.extend_from_slice(&self.blobs);
        Ok(bytes)
    }
}

fn u32s(words: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(4 * words.len());
    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes
}

fn f32s(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(4 * values.len());
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

/// `offset` as a `u32`.
fn offset(value: u32) -> Vec<u8> {
    value.to_le_bytes().to_vec()
}

/// The section-0 entries: entry 0 the inverse bind matrices, then one bone
/// group each. Returns the section's bytes and every entry's
/// section-relative offset (meshes point at them).
fn bone_data(model: &PreFoxModel) -> Result<(Vec<u8>, Vec<u32>), ModelError> {
    let mut section = SectionBuilder::new(4, 1 + model.bone_groups.len(), &[]);
    let mut entry_offsets = Vec::with_capacity(1 + model.bone_groups.len());

    let mut matrices = SectionBuilder::new(12, model.bones.len(), &[2, 0, 0, 0]);
    for bone in &model.bones {
        let at = matrices.add_blob(&f32s(&bone.matrix));
        matrices.add_record(&u32s(&[at, DatumFormat::Float32Matrix34.word(), 1]));
    }
    entry_offsets.push(section.add_blob(&matrices.finish()?));
    section.add_record(&offset(entry_offsets[0]));

    for group in &model.bone_groups {
        let mut entry = SectionBuilder::new(12, 1, &[2, 0, 0, 0]);
        let mut list = Vec::with_capacity(2 * group.len());
        for bone in group {
            list.extend_from_slice(&bone.to_le_bytes());
        }
        let at = entry.add_blob(&list);
        entry.add_record(&u32s(&[at, 1, group.len() as u32]));
        let entry_offset = section.add_blob(&entry.finish()?);
        entry_offsets.push(entry_offset);
        section.add_record(&offset(entry_offset));
    }
    Ok((section.finish()?, entry_offsets))
}

/// A name table section (5 or 6): one `name\0` blob and one `u32` record
/// per name. Returns the section's bytes and every record's offset.
fn name_section<'a>(
    names: impl Iterator<Item = &'a str>,
) -> Result<(Vec<u8>, Vec<u32>), ModelError> {
    let names: Vec<&str> = names.collect();
    let mut section = SectionBuilder::new(4, names.len(), &[]);
    let mut record_offsets = Vec::with_capacity(names.len());
    for name in names {
        let mut blob = name.as_bytes().to_vec();
        blob.push(0);
        let at = section.add_blob(&blob);
        record_offsets.push(section.add_record(&offset(at)));
    }
    Ok((section.finish()?, record_offsets))
}

/// The Meshes-section words shared by every pointer a mesh record carries:
/// the arrays' offsets relative to the section start.
struct MeshArrays {
    /// The bone-group array's offset (always written, empty or not).
    bone_group: u32,
    /// The annotation array's offset; 0 when the mesh has none.
    annotation: u32,
    /// The editor-data array's offset; 0 when `editor_data` is `None`.
    editor: u32,
}

/// The file offsets and record offsets mesh pointers are built from: the
/// sections the mesh records point into, all finished before Meshes.
struct CrossRefs {
    /// Section-0 entry offsets (section-relative), entry 0 first.
    entry_offsets: Vec<u32>,
    /// File offset of the BoneData section.
    bone_data: usize,
    /// Section 2 record offsets (section-relative), in string order.
    strings: Vec<u32>,
    /// File offset of the AnnotationStrings section.
    string_section: usize,
    /// Section 3 record offsets (section-relative), in order.
    records: Vec<u32>,
    /// File offset of the AnnotationRecords section.
    record_section: usize,
    /// File offset of the Meshes section.
    meshes: usize,
}

impl CrossRefs {
    /// A mesh pointer's file address as a signed delta from the Meshes
    /// section's start.
    fn delta(&self, file_offset: usize) -> i32 {
        (file_offset as i64 - self.meshes as i64) as i32
    }
}

/// Builds a mesh's bone-group, annotation and editor-data arrays into the
/// Meshes section and returns their offsets.
fn mesh_arrays(
    section: &mut SectionBuilder,
    mesh: &Mesh,
    refs: &CrossRefs,
) -> Result<MeshArrays, ModelError> {
    let mut bone_group = SectionBuilder::new(4, if mesh.bone_group.is_some() { 1 } else { 0 }, &[]);
    if let Some(group) = mesh.bone_group {
        let entry = refs
            .entry_offsets
            .get(group.checked_add(1).ok_or(ModelError::BadReference {
                what: "bone group",
                offset: group as i64,
            })?)
            .ok_or(ModelError::BadReference {
                what: "bone group",
                offset: group as i64,
            })?;
        let delta = refs.delta(refs.bone_data + *entry as usize);
        bone_group.add_record(&delta.to_le_bytes());
    }
    let bone_group = section.add_blob(&bone_group.finish()?);

    let annotation = if mesh.annotations.is_empty() {
        0
    } else {
        let mut array = SectionBuilder::new(16, mesh.annotations.len(), &[]);
        for annotation in &mesh.annotations {
            let string = refs
                .strings
                .get(annotation.string)
                .ok_or(ModelError::BadReference {
                    what: "annotation string",
                    offset: annotation.string as i64,
                })?;
            let record = match annotation.record {
                Some(index) => {
                    let at = refs.records.get(index).ok_or(ModelError::BadReference {
                        what: "annotation record",
                        offset: index as i64,
                    })?;
                    refs.delta(refs.record_section + *at as usize)
                }
                None => 0,
            };
            let string_delta = refs.delta(refs.string_section + *string as usize);
            let mut record_bytes = string_delta.to_le_bytes().to_vec();
            record_bytes.extend_from_slice(&record.to_le_bytes());
            record_bytes.extend_from_slice(&annotation.unknown.to_le_bytes());
            record_bytes.extend_from_slice(&annotation.kind.to_le_bytes());
            array.add_record(&record_bytes);
        }
        section.add_blob(&array.finish()?)
    };

    let editor = match &mesh.editor_data {
        None => 0,
        Some(items) => {
            let mut array = SectionBuilder::new(4, items.len(), &[]);
            for item in items {
                if matches!(&item.value, EditorValue::Text(_)) != (item.kind == 1) {
                    return Err(ModelError::WriteLayout(
                        "editor item kind and value disagree",
                    ));
                }
                let mut blob = item.kind.to_le_bytes().to_vec();
                blob.extend_from_slice(&item.unknown.to_le_bytes());
                match &item.value {
                    EditorValue::Text(text) => {
                        blob.extend_from_slice(text.as_bytes());
                        blob.push(0);
                    }
                    EditorValue::Word(word) => blob.extend_from_slice(&word.to_le_bytes()),
                }
                let at = array.add_blob(&blob);
                array.add_record(&offset(at));
            }
            section.add_blob(&array.finish()?)
        }
    };

    Ok(MeshArrays {
        bone_group,
        annotation,
        editor,
    })
}

/// One geometry's blobs, sub-arrays and record, appended to `section`;
/// returns the geometry record's offset.
fn geometry(section: &mut SectionBuilder, model_geometry: &Geometry) -> Result<u32, ModelError> {
    let mut data_offsets = Vec::with_capacity(model_geometry.vertex_fields.len());
    for field in &model_geometry.vertex_fields {
        let size = field.datum_format.size();
        if field.data.len() % size != 0 {
            return Err(ModelError::WriteLayout(
                "vertex field data not a multiple of its format",
            ));
        }
        data_offsets.push(section.add_blob(&field.data));
    }
    let mut descriptors = SectionBuilder::new(20, model_geometry.vertex_fields.len(), &[0; 8]);
    for (field, data_offset) in model_geometry.vertex_fields.iter().zip(&data_offsets) {
        descriptors.add_record(&u32s(&[
            *data_offset,
            field.datum_type.word(),
            field.datum_format.word(),
            field.count() as u32,
            0,
        ]));
    }
    let descriptors_offset = section.add_blob(&descriptors.finish()?);
    let mut vertex_set = SectionBuilder::new(4, 1, &[]);
    vertex_set.add_record(&offset(descriptors_offset));
    let vertex_set_offset = section.add_blob(&vertex_set.finish()?);

    let mut faces = Vec::with_capacity(2 * model_geometry.faces.indices.len());
    for index in &model_geometry.faces.indices {
        faces.extend_from_slice(&index.to_le_bytes());
    }
    let faces_offset = section.add_blob(&faces);
    let lod_table_offset = if model_geometry.faces.lod_ranges.is_empty() {
        0
    } else {
        let mut table = Vec::with_capacity(8 * model_geometry.faces.lod_ranges.len());
        for (start, end) in &model_geometry.faces.lod_ranges {
            table.extend_from_slice(&start.to_le_bytes());
            table.extend_from_slice(&end.to_le_bytes());
        }
        section.add_blob(&table)
    };
    let mut face_descriptor = SectionBuilder::new(24, 1, &[]);
    face_descriptor.add_record(&u32s(&[
        faces_offset,
        1,
        DatumFormat::Uint16.word(),
        model_geometry.faces.indices.len() as u32,
        model_geometry.faces.lod_ranges.len() as u32,
        lod_table_offset,
    ]));
    let face_descriptor_offset = section.add_blob(&face_descriptor.finish()?);

    // `write` always emits the version-19 layout: three extras records,
    // the order word 0 when the file had none.
    let mut extras = SectionBuilder::new(4, 3, &[]);
    let mut bounds = f32s(&model_geometry.bounds.min);
    bounds.extend_from_slice(&f32s(&model_geometry.bounds.max));
    let at = extras.add_blob(&bounds);
    extras.add_record(&offset(at));
    let at = extras.add_blob(&u32s(&[1, 0, 0, 0]));
    extras.add_record(&offset(at));
    let at = extras.add_blob(&model_geometry.order.unwrap_or(0).to_le_bytes());
    extras.add_record(&offset(at));
    let extras_offset = section.add_blob(&extras.finish()?);

    Ok(section.add_record(&u32s(&[
        1,
        vertex_set_offset,
        face_descriptor_offset,
        0,
        extras_offset,
    ])))
}

/// The sections built so far and where the next one lands: every section
/// is padded to 4, the file's own convention.
struct SectionList {
    sections: Vec<Section>,
    next_offset: usize,
}

impl SectionList {
    fn new() -> Self {
        SectionList {
            sections: Vec::with_capacity(11),
            next_offset: 80,
        }
    }

    /// Appends `bytes` as section `kind`, padded to 4, and returns the
    /// file offset it sits at.
    fn push(&mut self, kind: SectionKind, mut bytes: Vec<u8>) -> usize {
        let at = self.next_offset;
        bytes.resize(bytes.len() + pad4(bytes.len()), 0);
        self.next_offset += bytes.len();
        self.sections.push(Section { kind, bytes });
        at
    }
}

impl PreFoxModel {
    /// The container `write` emits: sections built in the export layout.
    pub(crate) fn to_container(&self) -> Result<ModelContainer, ModelError> {
        if self.bones.len() > u16::MAX as usize {
            return Err(ModelError::WriteLayout("more bones than u16 can index"));
        }

        let mut list = SectionList::new();

        let mut bounds_section = SectionBuilder::new(4, 2, &[]);
        let mut bounds = f32s(&self.bounds.min);
        bounds.extend_from_slice(&f32s(&self.bounds.max));
        let at = bounds_section.add_blob(&bounds);
        bounds_section.add_record(&offset(at));
        let mut lod = self.lod.level_count.to_le_bytes().to_vec();
        lod.extend_from_slice(&f32s(&self.lod.parameters));
        let at = bounds_section.add_blob(&lod);
        bounds_section.add_record(&offset(at));
        list.push(SectionKind::ModelBounds, bounds_section.finish()?);

        let (bone_data, entry_offsets) = bone_data(self)?;
        let bone_data_offset = list.push(SectionKind::BoneData, bone_data);

        let (bone_names, _) = name_section(self.bones.iter().map(|bone| bone.name.as_str()))?;
        list.push(SectionKind::BoneNames, bone_names);

        let (material_names, material_offsets) =
            name_section(self.material_names.iter().map(String::as_str))?;
        let material_names_offset = list.push(SectionKind::MaterialNames, material_names);

        let mut records = SectionBuilder::new(28, self.annotation_records.len(), &[]);
        let mut annotation_record_offsets = Vec::with_capacity(self.annotation_records.len());
        for record in &self.annotation_records {
            annotation_record_offsets.push(records.add_record(&u32s(record)));
        }
        let annotation_records_offset =
            list.push(SectionKind::AnnotationRecords, records.finish()?);

        list.push(SectionKind::Cloth, SectionBuilder::new(4, 0, &[]).finish()?);
        list.push(
            SectionKind::MaterialCombinations,
            SectionBuilder::new(4, 0, &[]).finish()?,
        );
        list.push(
            SectionKind::Locators,
            SectionBuilder::new(16, 0, &[0; 4]).finish()?,
        );

        let mut geometry_section = SectionBuilder::new(20, self.geometries.len(), &[]);
        let mut geometry_offsets = Vec::with_capacity(self.geometries.len());
        for model_geometry in &self.geometries {
            geometry_offsets.push(geometry(&mut geometry_section, model_geometry)?);
        }
        let geometry_section_offset = list.push(SectionKind::Geometry, geometry_section.finish()?);

        let mut strings = SectionBuilder::new(8, self.annotation_strings.len(), &[]);
        let mut string_offsets = Vec::with_capacity(self.annotation_strings.len());
        for text in &self.annotation_strings {
            let mut blob = text.as_bytes().to_vec();
            blob.push(0);
            let at = strings.add_blob(&blob);
            string_offsets.push(strings.add_record(&u32s(&[at, 0])));
        }
        let strings_offset = list.push(SectionKind::AnnotationStrings, strings.finish()?);

        let refs = CrossRefs {
            entry_offsets,
            bone_data: bone_data_offset,
            strings: string_offsets,
            string_section: strings_offset,
            records: annotation_record_offsets,
            record_section: annotation_records_offset,
            meshes: list.next_offset,
        };
        let mut meshes = SectionBuilder::new(24, self.meshes.len(), &[]);
        for mesh in &self.meshes {
            let geometry_record =
                geometry_offsets
                    .get(mesh.geometry)
                    .ok_or(ModelError::BadReference {
                        what: "geometry",
                        offset: mesh.geometry as i64,
                    })?;
            let material_record =
                material_offsets
                    .get(mesh.material)
                    .ok_or(ModelError::BadReference {
                        what: "material",
                        offset: mesh.material as i64,
                    })?;
            let arrays = mesh_arrays(&mut meshes, mesh, &refs)?;
            let geometry_delta = refs.delta(geometry_section_offset + *geometry_record as usize);
            let material_delta = refs.delta(material_names_offset + *material_record as usize);
            let mut record = geometry_delta.to_le_bytes().to_vec();
            record.extend_from_slice(&arrays.bone_group.to_le_bytes());
            record.extend_from_slice(&arrays.annotation.to_le_bytes());
            record.extend_from_slice(&material_delta.to_le_bytes());
            record.extend_from_slice(&0u32.to_le_bytes());
            record.extend_from_slice(&arrays.editor.to_le_bytes());
            meshes.add_record(&record);
        }
        list.push(SectionKind::Meshes, meshes.finish()?);

        ModelContainer::new(19, self.flags, list.sections)
    }

    /// The serialized `.model`, unwrapped.
    pub fn write(&self) -> Result<Vec<u8>, ModelError> {
        Ok(self.to_container()?.write())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::fixtures::*;

    #[test]
    fn pad4_pads_to_four() {
        for (len, pad) in [(0, 0), (1, 3), (2, 2), (3, 1), (4, 0), (5, 3), (8, 0)] {
            assert_eq!(pad4(len), pad, "pad4({len})");
        }
    }

    #[test]
    fn blob_offsets_stay_four_aligned() {
        // A 3-byte blob is zero-padded to 4, so the next blob's offset
        // keeps the layout's alignment.
        let mut builder = SectionBuilder::new(4, 0, &[]);
        builder.add_blob(&[1, 2, 3]);
        let second = builder.add_blob(&[4]);
        assert_eq!(second % 4, 0);
    }

    #[test]
    fn to_container_allows_the_bone_limit() {
        // 65535 bones fit a u16 index; 65536 do not.
        let mut file = PreFoxModel::read(CAP).unwrap();
        file.bones = vec![file.bones[0].clone(); u16::MAX as usize];
        assert!(file.to_container().is_ok());
        file.bones.push(file.bones[0].clone());
        assert_eq!(
            file.to_container(),
            Err(ModelError::WriteLayout("more bones than u16 can index"))
        );
    }
}
