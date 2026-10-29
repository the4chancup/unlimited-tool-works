//! Savefile discovery: the save always lives under the user's **Documents**
//! folder in a per-game KONAMI subfolder, never in the install folder. The
//! per-version layout is data (`save_layout`); `discover_savefiles_in` is pure
//! over a given root so tests build a temp tree.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use pes_version::PesVersion;

/// One game version's layout under `{Documents}\KONAMI\`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveLayout {
    /// The per-game folder name under `KONAMI`.
    pub game_folder: &'static str,
    /// Whether an account folder (a numeric name: the account's Konami or
    /// Steam ID) sits between the game folder and `save`.
    pub account_folders: bool,
    /// The save's file name inside `save`.
    pub file_name: &'static str,
}

/// A savefile that exists on disk.
#[derive(Debug, Clone, PartialEq)]
pub struct SavefileCandidate {
    /// The savefile's full path.
    pub path: PathBuf,
    /// The account folder the save sits under (PES 19-21), else `None`.
    pub account: Option<String>,
    /// The file's modification time (what "newest" means to the caller).
    pub modified: SystemTime,
}

/// The game folder name and layout for `version`.
pub fn save_layout(version: PesVersion) -> SaveLayout {
    let (game_folder, account_folders, file_name) = match version {
        PesVersion::Pes15 => ("Pro Evolution Soccer 2015", false, "EDIT.bin"),
        PesVersion::Pes16 => ("Pro Evolution Soccer 2016", false, "EDIT00000000"),
        PesVersion::Pes17 => ("Pro Evolution Soccer 2017", false, "EDIT00000000"),
        PesVersion::Pes18 => ("PRO EVOLUTION SOCCER 2018", false, "EDIT00000000"),
        PesVersion::Pes19 => ("PRO EVOLUTION SOCCER 2019", true, "EDIT00000000"),
        PesVersion::Pes20 => ("eFootball PES 2020", true, "EDIT00000000"),
        PesVersion::Pes21 => ("eFootball PES 2021 SEASON UPDATE", true, "EDIT00000000"),
    };
    SaveLayout {
        game_folder,
        account_folders,
        file_name,
    }
}

/// The candidates under `documents/KONAMI/...` for `version`, newest first.
/// Pure over the given root so tests build a temp tree; the native caller
/// passes `UserDirs::document_dir()`. Missing folders are an empty list, not
/// an error; discovery never creates folders or opens the saves.
pub fn discover_savefiles_in(documents: &Path, version: PesVersion) -> Vec<SavefileCandidate> {
    let layout = save_layout(version);
    let game = documents.join("KONAMI").join(layout.game_folder);
    let candidate = |path: PathBuf, account: Option<String>| {
        let modified = path
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        SavefileCandidate {
            path,
            account,
            modified,
        }
    };
    let mut found = Vec::new();
    if layout.account_folders {
        if let Ok(entries) = game.read_dir() {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let Some(account) = name.to_str() else {
                    continue;
                };
                // Digits of any length: a Konami ID has 18, a Steam ID64 (PES 20
                // bought on Steam) 17. The digits keep `WEPES` and the like out.
                if account.is_empty() || !account.bytes().all(|b| b.is_ascii_digit()) {
                    continue;
                }
                let path = entry.path().join("save").join(layout.file_name);
                if path.is_file() {
                    found.push(candidate(path, Some(account.to_string())));
                }
            }
        }
    } else {
        let path = game.join("save").join(layout.file_name);
        if path.is_file() {
            found.push(candidate(path, None));
        }
    }
    found.sort_by_key(|c| std::cmp::Reverse(c.modified));
    found
}

/// `discover_savefiles_in` under the shell's Documents folder (native only).
/// An unknown Documents folder is an empty list.
#[cfg(not(target_arch = "wasm32"))]
pub fn discover_savefiles(version: PesVersion) -> Vec<SavefileCandidate> {
    directories::UserDirs::new()
        .and_then(|dirs| dirs.document_dir().map(Path::to_path_buf))
        .map(|documents| discover_savefiles_in(&documents, version))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique temp root for the discovery tree.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "pes_savefile_discovery_{}_{}",
            name,
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("cleanup");
        }
        dir
    }

    /// The native entry point is `discover_savefiles_in` over the shell's
    /// Documents folder — on a machine with saves, a no-discovery mutant
    /// would return an empty list where the walk finds them.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn discover_savefiles_walks_the_documents_tree() {
        let Some(dirs) = directories::UserDirs::new() else {
            return;
        };
        let Some(documents) = dirs.document_dir() else {
            return;
        };
        for version in PesVersion::ALL {
            assert_eq!(
                discover_savefiles(version),
                discover_savefiles_in(documents, version),
                "{version:?}"
            );
        }
    }

    #[test]
    fn discovery_finds_saves_under_both_layouts() {
        let root = temp_dir("tree");
        let konami = root.join("KONAMI");
        // PES 17: flat layout.
        let p17 = konami
            .join("Pro Evolution Soccer 2017")
            .join("save")
            .join("EDIT00000000");
        // PES 19: two account saves plus an account folder without one.
        let p19a = konami
            .join("PRO EVOLUTION SOCCER 2019")
            .join("111111111111111111")
            .join("save")
            .join("EDIT00000000");
        let p19b = konami
            .join("PRO EVOLUTION SOCCER 2019")
            .join("222222222222222222")
            .join("save")
            .join("EDIT00000000");
        let p19empty = konami
            .join("PRO EVOLUTION SOCCER 2019")
            .join("333333333333333333")
            .join("save");
        // Not an account folder: 18 characters, not all digits. A save
        // under it stays undiscovered.
        let p19bad = konami
            .join("PRO EVOLUTION SOCCER 2019")
            .join("abcdefghijklmnopqr")
            .join("save")
            .join("EDIT00000000");
        // PES 20 bought on Steam: the account folder is the 17-digit Steam
        // ID64 (the maintainer's own), not an 18-digit Konami ID.
        let p20 = konami
            .join("eFootball PES 2020")
            .join("76561199320748656")
            .join("save")
            .join("EDIT00000000");
        for path in [&p17, &p19a, &p19b, &p19bad, &p20] {
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(path, b"save").expect("seed");
        }
        std::fs::create_dir_all(&p19empty).expect("empty account save dir");
        // The 222... save is the newer one.
        let later = std::fs::File::open(&p19a)
            .and_then(|f| f.metadata())
            .and_then(|m| m.modified())
            .expect("mtime")
            + std::time::Duration::from_secs(60);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&p19b)
            .expect("open writable")
            .set_modified(later)
            .expect("set_modified");

        let found = discover_savefiles_in(&root, PesVersion::Pes17);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, p17);
        assert_eq!(found[0].account, None);

        let found = discover_savefiles_in(&root, PesVersion::Pes19);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].path, p19b, "newest first");
        assert_eq!(found[0].account.as_deref(), Some("222222222222222222"));
        assert_eq!(found[1].path, p19a);
        assert_eq!(found[1].account.as_deref(), Some("111111111111111111"));

        let found = discover_savefiles_in(&root, PesVersion::Pes20);
        assert_eq!(found.len(), 1, "the Steam ID64 account folder is found");
        assert_eq!(found[0].path, p20);
        assert_eq!(found[0].account.as_deref(), Some("76561199320748656"));

        assert!(discover_savefiles_in(&root, PesVersion::Pes15).is_empty());
        // A missing KONAMI folder finds nothing.
        let other = temp_dir("empty");
        assert!(discover_savefiles_in(&other, PesVersion::Pes17).is_empty());

        std::fs::remove_dir_all(&root).expect("cleanup");
    }
}
