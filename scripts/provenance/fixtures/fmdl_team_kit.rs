//! Respells every FMDL texture entry naming the kit stand-in in the game's spelling
//! (`dummy_kit`, `dummy_kit_<rest>`, `dummy_gk_kit`, `dummy_gk_kit_<rest>`, folded) in the
//! Studio format's (`team_kit…`, `team_gk_kit…`: the stem in lowercase with `dummy_` replaced
//! by `team_`, the extension kept), in place, for the Team compiler's Studio-format fixtures,
//! which the compiler refuses in the old spelling (`dummy_kit_name_old`). The game's own
//! `dummy_nrm`, `dummy_srm` and `dummy_bsm` are no stand-in and are kept.
//!
//! Not part of the workspace: run as the bin of a scratch crate depending on `fmdl` by path
//! (`.tmp/0_0_2/poc12/respell_kit/`, with its own `[workspace]`), `cargo run -- <fmdl>...`.
//! Each file is written only after its rewrite reads back with every texture reference equal
//! to the original but for that one file name, and with the same count of meshes and textures.

use fmdl::FmdlFile;
use fmdl::ops::paths::{rewrite_texture_paths, texture_paths};

/// `file_name` in the Studio spelling when its stem is the kit stand-in's game spelling, or
/// `None`.
fn respelled(file_name: &str) -> Option<String> {
    let (stem, extension) = match file_name.rsplit_once('.') {
        Some((stem, extension)) => (stem, format!(".{extension}")),
        None => (file_name, String::new()),
    };
    let folded = stem.to_ascii_lowercase();
    let stand_in = ["dummy_kit", "dummy_gk_kit"]
        .iter()
        .any(|prefix| folded == *prefix || folded.starts_with(&format!("{prefix}_")));
    stand_in.then(|| format!("team_{}{extension}", &folded["dummy_".len()..]))
}

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    assert!(!files.is_empty(), "respell_team_kit <fmdl>...");
    for path in files {
        let bytes = std::fs::read(&path).expect("the file reads");
        let mut file = FmdlFile::read(&bytes).expect("an FMDL");
        let before = texture_paths(&file).expect("its texture table");
        let changed = rewrite_texture_paths(&mut file, |texture| {
            if let Some(file_name) = respelled(&texture.file_name) {
                texture.file_name = file_name;
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
            assert_eq!(old.directory, new.directory);
            let expected = respelled(&old.file_name).unwrap_or_else(|| old.file_name.clone());
            assert_eq!(new.file_name, expected);
            if old.file_name != new.file_name {
                println!("  {} {} -> {}", old.directory, old.file_name, new.file_name);
            }
        }
        std::fs::write(&path, written).expect("the file writes");
        println!("respelled {changed} reference(s) in {path}");
    }
}
