//! Respells the team folder of every FMDL texture directory below the Fox common directory
//! (`/Assets/pes16/model/character/common/000/…`) as the Studio format's `team_folder`, in
//! place, for the Team compiler's Studio-format fixtures, which the compiler refuses with
//! any other folder there (`fmdl_common_path_invalid`). The game's own
//! `common/sourceimages/` is no team folder and is kept.
//!
//! Not part of the workspace: run as the bin of a scratch crate depending on `fmdl` by path
//! (`.tmp/0_0_2/poc12/respell/`, with its own `[workspace]`), `cargo run -- <fmdl>...`. Each file is written only after its rewrite
//! reads back with every texture reference equal to the original but for that one segment,
//! and with the same count of meshes and textures.

use fmdl::FmdlFile;
use fmdl::ops::paths::{rewrite_texture_paths, texture_paths};

const COMMON: &str = "/model/character/common/";

/// `directory` with the folder after the Fox common directory spelled `team_folder`, or
/// `None` when it names none or the game's `sourceimages/`.
fn respelled(directory: &str) -> Option<String> {
    let at = directory.to_ascii_lowercase().find(COMMON)? + COMMON.len();
    let (head, rest) = directory.split_at(at);
    let (segment, tail) = rest.split_once('/')?;
    if segment.eq_ignore_ascii_case("sourceimages") || segment == "team_folder" {
        return None;
    }
    Some(format!("{head}team_folder/{tail}"))
}

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    assert!(!files.is_empty(), "respell_team_folder <fmdl>...");
    for path in files {
        let bytes = std::fs::read(&path).expect("the file reads");
        let mut file = FmdlFile::read(&bytes).expect("an FMDL");
        let before = texture_paths(&file).expect("its texture table");
        let changed = rewrite_texture_paths(&mut file, |texture| {
            if let Some(directory) = respelled(&texture.directory) {
                texture.directory = directory;
            }
        })
        .expect("the rewrite");
        if changed == 0 {
            println!("unchanged {path}");
            continue;
        }
        let written = file.write();
        let read_back = FmdlFile::read(&written).expect("the rewrite reads back");
        let after = texture_paths(&read_back).expect("its texture table");
        assert_eq!(before.len(), after.len());
        assert_eq!(read_back.meshes.len(), FmdlFile::read(&bytes).unwrap().meshes.len());
        for (old, new) in before.iter().zip(&after) {
            assert_eq!(old.file_name, new.file_name);
            let expected = respelled(&old.directory).unwrap_or_else(|| old.directory.clone());
            assert_eq!(new.directory, expected);
            if old.directory != new.directory {
                println!("  {} {} -> {}", old.file_name, old.directory, new.directory);
            }
        }
        std::fs::write(&path, written).expect("the file writes");
        println!("respelled {changed} reference(s) in {path}");
    }
}
