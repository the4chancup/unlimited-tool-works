//! The export format as data, no logic: name conventions and kind tables every
//! consumer shares, so a new allowed name is a table entry, not a behavior.

mod file_types;
mod kit_token;
mod player_folder;

use crate::slots::PlayerSlot;

pub use file_types::{
    FileKind, Marker, MetadataFile, ModelFormat, SharedKind, classify, common_link_name,
    common_link_target, is_txt_note,
};
pub(crate) use file_types::{icon_number, is_logo_texture, shared_link_name, strip_prefix_ci};
pub(crate) use kit_token::names_kit_fallback;
pub use kit_token::{
    KitToken, LegacyKitToken, kit_folder, kit_path_token, kit_set_member, kit_token,
    kit_token_starts, legacy_kit_tokens, variant_path, variant_stem, without_kit_token,
};
pub use player_folder::{ModelSuffix, ends_with_name, is_player_singleton, model_suffix};
pub(crate) use player_folder::{is_boots, is_explicit_face, is_gloves};

/// One of the eight content folders at the export root; the draft groups each
/// by kind. `all/` lives inside `Kits/` and is not a kind of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum ContentFolder {
    /// `Players/`, the player folders.
    Players,
    /// `Faces/`, shared face folders.
    Faces,
    /// `Boots/`, shared boots folders.
    Boots,
    /// `Gloves/`, shared gloves folders.
    Gloves,
    /// `Kits/`, the kit folders (`all/` included).
    Kits,
    /// `Portraits/`.
    Portraits,
    /// `Collars/`.
    Collars,
    /// `Common/`, the team's shared content.
    Common,
}

/// The eight content folder names at the export root (matched
/// ASCII-case-insensitively): everything else at the root is an unrecognized
/// folder.
pub(crate) const CONTENT_FOLDERS: [(&str, ContentFolder); 8] = [
    ("Players", ContentFolder::Players),
    ("Faces", ContentFolder::Faces),
    ("Boots", ContentFolder::Boots),
    ("Gloves", ContentFolder::Gloves),
    ("Kits", ContentFolder::Kits),
    ("Portraits", ContentFolder::Portraits),
    ("Common", ContentFolder::Common),
    ("Collars", ContentFolder::Collars),
];

/// The small file names the structure pass reads eagerly: `players.txt`,
/// `refs.txt`, `notes.txt`, by name (ASCII-case-insensitively) at any depth,
/// so the consumer needs no root normalization of its own.
pub fn is_small_metadata(path: &str) -> bool {
    const NAMES: [&str; 3] = ["players.txt", "refs.txt", "notes.txt"];
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    NAMES.iter().any(|known| name.eq_ignore_ascii_case(known))
}

/// File names a file browser writes, never an author: `parse_listing` leaves
/// them out of the draft and reports nothing ("Validation semantics" → "OS
/// artifacts").
pub(crate) const OS_ARTIFACTS: [&str; 3] = ["thumbs.db", "desktop.ini", ".ds_store"];

/// Whether `name` (a path's last segment) is an OS artifact, ASCII-case-
/// insensitively: one of `Thumbs.db`, `desktop.ini` or `.DS_Store`, the files a
/// file browser writes and an export never means to carry.
pub fn is_os_artifact(name: &str) -> bool {
    OS_ARTIFACTS
        .iter()
        .any(|known| name.eq_ignore_ascii_case(known))
}

/// A player or kit folder's `<head>[ - <label>]` split ("Validation
/// semantics" → "Folder names"): the name before the first `-`, trimmed, and
/// the trimmed remainder when it is non-empty.
pub(crate) fn split_folder_name(name: &str) -> (&str, Option<&str>) {
    match name.split_once('-') {
        Some((head, label)) => {
            let label = label.trim();
            (head.trim(), (!label.is_empty()).then_some(label))
        }
        None => (name.trim(), None),
    }
}

/// The slot a player folder's or portrait's name claims ("Validation
/// semantics" → "Folder names" and the `Portraits/` row): the
/// `split_folder_name` head all ASCII digits making a `PlayerSlot` (`05`,
/// `05 - Bob` and `05-Bob` alike read as slot 05; `081` is no slot 08,
/// `player_05` no slot at all).
pub(crate) fn folder_slot(name: &str) -> Option<PlayerSlot> {
    let (head, _) = split_folder_name(name);
    head.bytes()
        .all(|byte| byte.is_ascii_digit())
        .then(|| head.parse::<u8>().ok())
        .flatten()
        .and_then(PlayerSlot::new)
}

/// The name a numbered player folder's `name = true` writes ("Validation
/// semantics" → "Folder names"): its label after the first `-` (`15 -
/// Snuffy` gives `Snuffy`), or the whole name when it has none.
pub fn folder_player_name(folder_name: &str) -> &str {
    split_folder_name(folder_name).1.unwrap_or(folder_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_small_metadata_recognizes_the_three_names_at_any_depth() {
        for path in [
            "players.txt",
            "wrapper/players.txt",
            "wrapper/NOTES.TXT",
            "notes.txt",
            "refs.txt",
            "a\\b\\players.txt",
        ] {
            assert!(is_small_metadata(path), "{path}");
        }
        for path in [
            "Kits/p1/icon.txt",
            "colors.txt",
            "Players/03 - A/settings.toml",
            "config.toml",
            "players.txt.bak",
        ] {
            assert!(!is_small_metadata(path), "{path}");
        }
    }

    #[test]
    fn folder_player_name_is_the_label_or_the_whole_name() {
        assert_eq!(folder_player_name("03 - Jean-Pierre"), "Jean-Pierre");
        assert_eq!(folder_player_name("07"), "07");
    }
}
