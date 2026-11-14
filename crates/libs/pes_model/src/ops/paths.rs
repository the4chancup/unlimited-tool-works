//! Texture path editing on a `.mtl`: the Compiler rewrites texture
//! directories in Konami-derived material sets constantly. A path is split
//! at its last `/` so an edit sees directory and file name separately; the
//! material and sampler names the reference is listed under name the slot,
//! not the texture, and are not part of the edit.

use crate::format::mtl::{MaterialEntry, MaterialSet, Sampler};

/// One texture reference of a material set: which material and sampler bind it, and the path split at its last `/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TexturePath {
    /// The material's name.
    pub material: String,
    /// The sampler's name.
    pub sampler: String,
    /// Up to and including the last `/` (`./`, `model/character/parts/referee/`); empty when the path has no `/`.
    pub directory: String,
    /// After the last `/` (`card_red_bsm.dds`).
    pub file_name: String,
}

/// The reference `sampler` of `material` holds, its path split at the last `/`.
fn texture_path(material: &str, sampler: &Sampler) -> TexturePath {
    let (directory, file_name) = match sampler.path.rfind('/') {
        Some(at) => (&sampler.path[..at + 1], &sampler.path[at + 1..]),
        None => ("", sampler.path.as_str()),
    };
    TexturePath {
        material: material.to_owned(),
        sampler: sampler.name.clone(),
        directory: directory.to_owned(),
        file_name: file_name.to_owned(),
    }
}

/// Every sampler path of `set`, in file order.
pub fn texture_paths(set: &MaterialSet) -> Vec<TexturePath> {
    let mut paths = Vec::new();
    for material in &set.materials {
        for entry in &material.entries {
            if let MaterialEntry::Sampler(sampler) = entry {
                paths.push(texture_path(&material.name, sampler));
            }
        }
    }
    paths
}

/// Applies `edit` to every reference and writes back `directory + file_name` for the ones it changed; `material` and `sampler` edits are ignored (they name the slot, not the texture). Returns how many paths changed; 0 leaves `set` untouched.
pub fn rewrite_texture_paths(
    set: &mut MaterialSet,
    mut edit: impl FnMut(&mut TexturePath),
) -> usize {
    let mut changed = 0;
    for material in &mut set.materials {
        for entry in &mut material.entries {
            if let MaterialEntry::Sampler(sampler) = entry {
                let mut path = texture_path(&material.name, sampler);
                edit(&mut path);
                // `material`/`sampler` edits name the slot, not the texture:
                // only a directory or file-name change rewrites the path.
                let rewritten = path.directory + &path.file_name;
                if rewritten != sampler.path {
                    sampler.path = rewritten;
                    changed += 1;
                }
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::fixtures::*;
    use crate::format::mtl::{Material, Sampler};

    fn read(bytes: &[u8]) -> MaterialSet {
        MaterialSet::read(bytes).expect("fixture parses")
    }

    #[test]
    fn texture_paths_lists_every_sampler() {
        assert_eq!(
            texture_paths(&read(CARD_RED_MTL)),
            [TexturePath {
                material: "judge_card_red".to_owned(),
                sampler: "DiffuseMap".to_owned(),
                directory: "model/character/parts/referee/".to_owned(),
                file_name: "card_red_bsm.dds".to_owned(),
            }]
        );
        let head = texture_paths(&read(HEAD_HI_MTL));
        assert_eq!(head.len(), 3);
        for (path, file_name) in head.iter().zip([
            "head_normal_default.dds",
            "head_normal_animation.dds",
            "head_normal_mask.dds",
        ]) {
            assert_eq!(path.directory, "./");
            assert_eq!(path.file_name, file_name);
        }
        assert!(texture_paths(&read(SHADOW_MTL)).is_empty());
        assert_eq!(texture_paths(&read(ACCESSORY_MTL)).len(), 22);
    }

    #[test]
    fn a_noop_edit_changes_nothing() {
        for bytes in ALL_MTL {
            let mut set = read(bytes);
            let before = set.write();
            assert_eq!(rewrite_texture_paths(&mut set, |_| {}), 0);
            assert_eq!(set.write(), before);
        }
        for bytes in [CARDHEAD_MTL, HEAD_HI_MTL, HAIR_MTL, SHADOW_MTL] {
            let mut set = read(bytes);
            rewrite_texture_paths(&mut set, |_| {});
            assert_eq!(set.write(), bytes);
        }
    }

    #[test]
    fn edits_rewrite_the_path() {
        let mut set = read(CARD_RED_MTL);
        assert_eq!(
            rewrite_texture_paths(&mut set, |path| path.directory = "./".to_owned()),
            1
        );
        match &set.materials[0].entries[0] {
            MaterialEntry::Sampler(sampler) => {
                assert_eq!(sampler.path, "./card_red_bsm.dds");
            }
            entry => panic!("expected a sampler, got {entry:?}"),
        }

        let mut set = read(ACCESSORY_MTL);
        assert_eq!(
            rewrite_texture_paths(&mut set, |path| {
                if path.file_name == "Glasses01.dds" {
                    path.file_name = "Glasses01_kit2.dds".to_owned();
                }
            }),
            2
        );
        let rewritten = texture_paths(&set)
            .iter()
            .filter(|path| path.file_name == "Glasses01_kit2.dds" && path.directory == "./")
            .count();
        assert_eq!(rewritten, 2);
    }

    #[test]
    fn a_path_without_a_slash_has_no_directory() {
        let mut set = MaterialSet {
            materials: vec![Material {
                name: "m".to_owned(),
                shader: "s".to_owned(),
                entries: vec![MaterialEntry::Sampler(Sampler {
                    name: "n".to_owned(),
                    path: "texture.dds".to_owned(),
                    srgb: None,
                    minfilter: None,
                    maxfilter: None,
                    magfilter: None,
                    mipfilter: None,
                    uaddr: None,
                    vaddr: None,
                    waddr: None,
                    maxaniso: None,
                })],
            }],
            style: crate::format::mtl::MtlStyle::default(),
        };
        assert_eq!(
            texture_paths(&set),
            [TexturePath {
                material: "m".to_owned(),
                sampler: "n".to_owned(),
                directory: String::new(),
                file_name: "texture.dds".to_owned(),
            }]
        );
        assert_eq!(rewrite_texture_paths(&mut set, |_| {}), 0);
        match &set.materials[0].entries[0] {
            MaterialEntry::Sampler(sampler) => assert_eq!(sampler.path, "texture.dds"),
            entry => panic!("expected a sampler, got {entry:?}"),
        }
    }
}
