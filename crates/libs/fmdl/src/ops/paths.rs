//! Texture path rewriting on the typed record layer: the Compiler
//! edits texture directories in Konami-derived models constantly and must
//! not relayout the whole file to do it. New strings are appended to the
//! string table, de-duplicated against everything already there; existing
//! strings are never modified in place because other records may share
//! them. The extension-header tail after the last string is kept verbatim.

use std::collections::HashMap;

use crate::StringRecord;
use crate::format::{FmdlError, FmdlFile, table_u16};

/// One texture reference of the file's texture table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TexturePath {
    /// The texture file name.
    pub file_name: String,
    /// The texture directory.
    pub directory: String,
}

/// The file's texture references in table order.
pub fn texture_paths(file: &FmdlFile) -> Result<Vec<TexturePath>, FmdlError> {
    file.textures
        .iter()
        .map(|texture| {
            Ok(TexturePath {
                file_name: file
                    .string(usize::from(texture.filename_string_id))?
                    .to_owned(),
                directory: file
                    .string(usize::from(texture.directory_string_id))?
                    .to_owned(),
            })
        })
        .collect()
}

/// The texture references a mesh uses, in table order, each once: those in the texture run
/// of the material instance of at least one mesh. An entry no mesh uses is never loaded.
pub fn used_texture_paths(file: &FmdlFile) -> Result<Vec<TexturePath>, FmdlError> {
    let mut used = vec![false; file.textures.len()];
    for mesh in &file.meshes {
        let instance_id = usize::from(mesh.material_instance_id);
        let instance = file
            .material_instances
            .get(instance_id)
            .ok_or(FmdlError::BadReference {
                what: "material instance",
                index: instance_id,
            })?;
        let first = usize::from(instance.first_texture_id);
        for index in first..first + usize::from(instance.texture_count) {
            let assignment =
                file.parameter_assignments
                    .get(index)
                    .ok_or(FmdlError::BadReference {
                        what: "texture / parameter assignment",
                        index,
                    })?;
            let texture = usize::from(assignment.reference_id);
            *used.get_mut(texture).ok_or(FmdlError::BadReference {
                what: "texture",
                index: texture,
            })? = true;
        }
    }
    Ok(texture_paths(file)?
        .into_iter()
        .zip(used)
        .filter_map(|(path, used)| used.then_some(path))
        .collect())
}

/// Applies `edit` to every texture reference. Changed references get new
/// strings appended to the string table (de-duplicated against every
/// string already there) and their texture records repointed; the string
/// block and its descriptors are rebuilt with the extension-header tail
/// kept. Nothing else in the file changes. Returns the number of
/// references changed; when it is 0 the file is untouched byte for byte.
pub fn rewrite_texture_paths(
    file: &mut FmdlFile,
    mut edit: impl FnMut(&mut TexturePath),
) -> Result<usize, FmdlError> {
    let paths = texture_paths(file)?;
    let mut changed: Vec<(usize, TexturePath)> = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        let mut edited = path.clone();
        edit(&mut edited);
        if edited != *path {
            changed.push((index, edited));
        }
    }
    if changed.is_empty() {
        return Ok(0);
    }

    // The tail is whatever followed the last block-3 string's NUL: the
    // extension-header text when present.
    let tail: Vec<u8> = file.extension_tail().to_vec();

    // Every string's text, then the new strings appended de-duplicated.
    let mut texts: Vec<String> = Vec::with_capacity(file.strings.len());
    for index in 0..file.strings.len() {
        texts.push(file.string(index)?.to_owned());
    }
    let mut by_text: HashMap<String, usize> = texts
        .iter()
        .enumerate()
        .map(|(index, text)| (text.clone(), index))
        .collect();
    for (_, path) in &changed {
        for text in [&path.file_name, &path.directory] {
            if !by_text.contains_key(text.as_str()) {
                by_text.insert(text.clone(), texts.len());
                texts.push(text.clone());
            }
        }
    }

    let count = changed.len();
    for (index, path) in changed {
        let texture = &mut file.textures[index];
        texture.filename_string_id = table_u16("strings", by_text[&path.file_name])?;
        texture.directory_string_id = table_u16("strings", by_text[&path.directory])?;
    }

    // Rebuild the descriptors of block 3 and the block itself: each string
    // is `utf-8 bytes + NUL`; records for other blocks are left alone.
    let mut new_table = Vec::new();
    let original_count = file.strings.len();
    for (index, text) in texts.iter().enumerate() {
        let offset = u32::try_from(new_table.len()).map_err(|_| FmdlError::TableOverflow {
            what: "string table bytes",
            count: new_table.len(),
        })?;
        if index < original_count {
            let record = &mut file.strings[index];
            if record.string_block_id != 3 {
                continue;
            }
            record.length = table_u16("string", text.len())?;
            record.offset = offset;
        } else {
            file.strings.push(StringRecord {
                string_block_id: 3,
                length: table_u16("string", text.len())?,
                offset,
            });
        }
        new_table.extend(text.as_bytes());
        new_table.push(0);
    }
    new_table.extend(tail);
    file.string_table = Some(new_table);

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Model;

    const HIGHNECK: &[u8] = include_bytes!("../../tests/fixtures/konami_highneck.fmdl");
    const MOUTH: &[u8] = include_bytes!("../../tests/fixtures/konami_mouth.fmdl");
    const AU_LOW: &[u8] = include_bytes!("../../tests/fixtures/konami_au_Low_parts.fmdl");
    const ORAL: &[u8] = include_bytes!("../../tests/fixtures/addon_oral.fmdl");
    const PLACEHOLDER: &[u8] = include_bytes!("../../tests/fixtures/addon_placeholder.fmdl");

    const FIXTURES: &[&[u8]] = &[HIGHNECK, MOUTH, AU_LOW, ORAL, PLACEHOLDER];

    /// The file names of `paths`.
    fn file_names(paths: &[TexturePath]) -> Vec<&str> {
        paths.iter().map(|path| path.file_name.as_str()).collect()
    }

    /// The highneck model with a second material instance, whose texture run is the
    /// assignments 1 to 3, now naming the textures 3, 1 and 3 (out of table order, one twice);
    /// the first instance's run, assignments 0 to 3, names 0, 3, 1 and 3.
    fn highneck_with_two_instances() -> FmdlFile {
        let mut file = FmdlFile::read(HIGHNECK).unwrap();
        for (assignment, reference) in [(1, 3), (2, 1), (3, 3)] {
            file.parameter_assignments[assignment].reference_id = reference;
        }
        let mut second = file.material_instances[0].clone();
        second.first_texture_id = 1;
        second.texture_count = 3;
        file.material_instances.push(second);
        file
    }

    #[test]
    fn the_used_textures_are_the_mesh_instances_runs_in_table_order_each_once() {
        let mut file = highneck_with_two_instances();
        for mesh in &mut file.meshes {
            mesh.material_instance_id = 1;
        }
        assert_eq!(
            file_names(&used_texture_paths(&file).unwrap()),
            ["accessory_nrm.tga", "accessory_trm.tga"]
        );
        for mesh in &mut file.meshes {
            mesh.material_instance_id = 0;
        }
        assert_eq!(
            file_names(&used_texture_paths(&file).unwrap()),
            [
                "accessory_bsm.tga",
                "accessory_nrm.tga",
                "accessory_trm.tga"
            ],
            "accessory_srm.tga is in no run"
        );
        // An untouched model's one instance uses every texture.
        let file = FmdlFile::read(HIGHNECK).unwrap();
        assert_eq!(
            used_texture_paths(&file).unwrap(),
            texture_paths(&file).unwrap()
        );
    }

    #[test]
    fn a_texture_run_naming_no_texture_of_the_table_is_the_error() {
        let mut file = FmdlFile::read(HIGHNECK).unwrap();
        file.parameter_assignments[2].reference_id = 4;
        assert!(matches!(
            used_texture_paths(&file),
            Err(FmdlError::BadReference {
                what: "texture",
                index: 4
            })
        ));
        let mut file = FmdlFile::read(HIGHNECK).unwrap();
        file.meshes[0].material_instance_id = 1;
        assert!(matches!(
            used_texture_paths(&file),
            Err(FmdlError::BadReference {
                what: "material instance",
                index: 1
            })
        ));
    }

    #[test]
    fn texture_paths_match_the_expected_lists() {
        let directory = "/Assets/pes16/model/character/common/sourceimages/";
        let file = FmdlFile::read(HIGHNECK).unwrap();
        assert_eq!(
            texture_paths(&file).unwrap(),
            vec![
                TexturePath {
                    file_name: "accessory_bsm.tga".to_owned(),
                    directory: directory.to_owned(),
                },
                TexturePath {
                    file_name: "accessory_nrm.tga".to_owned(),
                    directory: directory.to_owned(),
                },
                TexturePath {
                    file_name: "accessory_srm.tga".to_owned(),
                    directory: directory.to_owned(),
                },
                TexturePath {
                    file_name: "accessory_trm.tga".to_owned(),
                    directory: directory.to_owned(),
                },
            ]
        );
        let file = FmdlFile::read(ORAL).unwrap();
        assert_eq!(
            texture_paths(&file).unwrap(),
            vec![
                TexturePath {
                    file_name: "dummy_bsm.dds".to_owned(),
                    directory: directory.to_owned(),
                },
                TexturePath {
                    file_name: "dummy_nrm.dds".to_owned(),
                    directory: directory.to_owned(),
                },
                TexturePath {
                    file_name: "dummy_srm.dds".to_owned(),
                    directory: directory.to_owned(),
                },
            ]
        );
    }

    #[test]
    fn a_noop_edit_leaves_the_file_untouched() {
        for bytes in FIXTURES {
            let mut file = FmdlFile::read(bytes).unwrap();
            assert_eq!(rewrite_texture_paths(&mut file, |_| {}).unwrap(), 0);
            // The typed write is byte-identical for Konami files; for the
            // `addon_*` fixtures the guarantee is `read(write(x)) == x`, so
            // the assertion is against the untouched file's own serialization.
            assert_eq!(file.write(), FmdlFile::read(bytes).unwrap().write());
        }
        for bytes in [HIGHNECK, MOUTH, AU_LOW] {
            let mut file = FmdlFile::read(bytes).unwrap();
            assert_eq!(rewrite_texture_paths(&mut file, |_| {}).unwrap(), 0);
            assert_eq!(file.write(), *bytes);
        }
    }

    #[test]
    fn directory_rewrite_rebuilds_only_the_strings() {
        let original = FmdlFile::read(HIGHNECK).unwrap();
        let mut file = original.clone();
        let changed = rewrite_texture_paths(&mut file, |path| {
            path.directory =
                "/Assets/pes16/model/character/face/real/12345/sourceimages/".to_owned();
        })
        .unwrap();
        assert_eq!(changed, 4);

        let written = file.write();
        let reread = FmdlFile::read(&written).unwrap();
        assert!(
            texture_paths(&reread)
                .unwrap()
                .iter()
                .all(|path| path.directory
                    == "/Assets/pes16/model/character/face/real/12345/sourceimages/")
        );

        let mut rewritten_model = Model::from_file(&reread).unwrap();
        let original_model = Model::from_file(&original).unwrap();
        for material in &mut rewritten_model.materials {
            for (_, texture) in &mut material.textures {
                texture.directory = String::new();
            }
        }
        let mut original_model = original_model;
        for material in &mut original_model.materials {
            for (_, texture) in &mut material.textures {
                texture.directory = String::new();
            }
        }
        assert_eq!(rewritten_model, original_model);

        assert_eq!(file.bones, original.bones);
        assert_eq!(file.mesh_groups, original.mesh_groups);
        assert_eq!(file.mesh_group_assignments, original.mesh_group_assignments);
        assert_eq!(file.meshes, original.meshes);
        assert_eq!(file.material_instances, original.material_instances);
        assert_eq!(file.bone_groups, original.bone_groups);
        assert_eq!(file.parameter_assignments, original.parameter_assignments);
        assert_eq!(file.materials, original.materials);
        assert_eq!(
            file.mesh_format_assignments,
            original.mesh_format_assignments
        );
        assert_eq!(file.mesh_formats, original.mesh_formats);
        assert_eq!(file.vertex_formats, original.vertex_formats);
        assert_eq!(file.bounding_boxes, original.bounding_boxes);
        assert_eq!(file.buffer_offsets, original.buffer_offsets);
        assert_eq!(file.levels_of_detail, original.levels_of_detail);
        assert_eq!(file.face_indices, original.face_indices);
        assert_eq!(file.block_18, original.block_18);
        assert_eq!(file.block_20, original.block_20);
        assert_eq!(file.unknown_blocks, original.unknown_blocks);
        assert_eq!(file.material_parameters, original.material_parameters);
        assert_eq!(file.bone_matrices, original.bone_matrices);
        assert_eq!(file.buffer, original.buffer);
        assert_eq!(file.unknown_buffers, original.unknown_buffers);
    }

    #[test]
    fn renaming_one_texture_keeps_the_shared_directory_string() {
        let mut file = FmdlFile::read(ORAL).unwrap();
        let changed = rewrite_texture_paths(&mut file, |path| {
            if path.file_name == "dummy_bsm.dds" {
                path.file_name = "face_bsm.dds".to_owned();
            }
        })
        .unwrap();
        assert_eq!(changed, 1);
        let paths = texture_paths(&file).unwrap();
        assert_eq!(paths[0].file_name, "face_bsm.dds");
        assert_eq!(paths[1].file_name, "dummy_nrm.dds");
        assert_eq!(paths[2].file_name, "dummy_srm.dds");
        let directory_ids: Vec<u16> = file
            .textures
            .iter()
            .map(|texture| texture.directory_string_id)
            .collect();
        assert!(directory_ids.iter().all(|id| *id == directory_ids[0]));
    }

    #[test]
    fn the_extension_header_tail_survives_a_rewrite() {
        let mut file = FmdlFile::read(ORAL).unwrap();
        rewrite_texture_paths(&mut file, |path| {
            if path.file_name == "dummy_bsm.dds" {
                path.file_name = "face_bsm.dds".to_owned();
            }
        })
        .unwrap();
        let reread = FmdlFile::read(&file.write()).unwrap();
        let model = Model::from_file(&reread).unwrap();
        assert!(model.extensions.antiblur);
        assert!(model.extensions.vertex_loop_preservation);
        assert!(!model.extensions.mesh_splitting);
    }

    #[test]
    fn a_rewrite_appends_one_new_string() {
        let original = FmdlFile::read(HIGHNECK).unwrap();
        let mut file = original.clone();
        rewrite_texture_paths(&mut file, |path| {
            path.directory =
                "/Assets/pes16/model/character/face/real/12345/sourceimages/".to_owned();
        })
        .unwrap();
        // The four references share one directory string: one new entry.
        assert_eq!(file.strings.len(), original.strings.len() + 1);
    }

    #[test]
    fn a_table_without_the_final_nul_rewrites_clean() {
        let mut file = FmdlFile::read(HIGHNECK).unwrap();
        // The last string's NUL is gone: its bytes run to the table's end.
        let last_end = file
            .strings
            .iter()
            .filter(|record| record.string_block_id == 3)
            .map(|record| record.offset as usize + usize::from(record.length))
            .max()
            .unwrap();
        file.string_table = Some(file.string_table.as_deref().unwrap()[..last_end].to_vec());
        rewrite_texture_paths(&mut file, |path| {
            path.directory = "/new/dir/".to_owned();
        })
        .unwrap();
        // The new table is exactly the strings, each NUL-terminated: no
        // copy of the old table is appended.
        let mut expected = Vec::new();
        for index in 0..file.strings.len() {
            expected.extend_from_slice(file.string(index).unwrap().as_bytes());
            expected.push(0);
        }
        assert_eq!(file.string_table.as_deref(), Some(expected.as_slice()));
    }

    #[test]
    fn deduplication_is_idempotent() {
        let mut file = FmdlFile::read(HIGHNECK).unwrap();
        let edit = |path: &mut TexturePath| {
            path.directory =
                "/Assets/pes16/model/character/face/real/12345/sourceimages/".to_owned();
        };
        rewrite_texture_paths(&mut file, edit).unwrap();
        let strings_after_first = file.strings.len();
        rewrite_texture_paths(&mut file, edit).unwrap();
        assert_eq!(file.strings.len(), strings_after_first);
    }
}
