//! The listing a consumer supplies: one export source as a flat path list plus
//! the small metadata files' bytes, and the settings the structure pass needs.

use std::collections::{BTreeMap, BTreeSet};

use pes_version::PesVersion;

use crate::slots::PlayerSlot;

/// One export source's canonical listing; `parse_listing` canonicalizes it.
/// `display_name` is the source's stem: the folder name, or the archive name
/// without extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalListing {
    /// The source's stem: folder name, or archive name without extension.
    pub display_name: String,
    /// Every path in the source, in any order.
    pub entries: Vec<ListedEntry>,
}

/// One path in the listing, spelled as the source spells it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedEntry {
    /// As the source spells it, relative to the source root.
    pub path: String,
    /// Whether the entry is a file or a folder.
    pub kind: ListedKind,
}

/// What a `ListedEntry` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ListedKind {
    /// A file, with its stored size.
    File {
        /// The file's size in bytes.
        size: u64,
    },
    /// A directory; needed only for a folder with nothing below it (an empty kit).
    Folder,
}

/// The small files the structure pass reads, keyed by `ListedEntry::path`. The
/// consumer reads every listed file `is_small_metadata` accepts; a failed read
/// carries its reason (`source_read_failed`, with the disposition of what the
/// file is: `DropExport` for a roster, `DropFile` for `notes.txt`). A listed
/// file missing from the map was not read, and counts as a failed read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmallMetadata {
    /// Each small metadata file's content, or its read failure's reason.
    pub files: BTreeMap<String, Result<Vec<u8>, String>>,
}

impl SmallMetadata {
    /// The bytes for a descriptor's `source` path: the map is keyed by raw
    /// listing paths, so each key is canonicalized before comparing. A
    /// missing entry is a failed read (`"not read"`), as the roster's.
    pub(crate) fn bytes_for(&self, file: &crate::parse::FileDescriptor) -> Result<&[u8], String> {
        for (key, result) in &self.files {
            if vtree::ScopePath::new(key).ok().as_ref() == Some(&file.source) {
                return result.as_deref().map_err(Clone::clone);
            }
        }
        Err("not read".to_owned())
    }
}

/// A small metadata file's text: an optional UTF-8 BOM stripped, then strict
/// UTF-8 (the roster and `notes.txt` are both read this way).
pub(crate) fn metadata_text(bytes: &[u8]) -> Result<&str, std::str::Utf8Error> {
    str::from_utf8(bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes))
}

/// The settings that change a consequence, supplied by the consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationContext {
    /// The target version: Fox or pre-Fox allowed names.
    pub version: PesVersion,
    /// Disallowed file types are errors instead of info notes.
    pub strict_file_type_check: bool,
    /// Keep eligible dropped content instead of discarding it.
    pub pass_through: bool,
    /// The stems, folded, of the textures the installed CPKs loaded before the one being compiled
    /// hold in this export's team's Common output: a texture `.common` link whose target is not
    /// in `Common/` names one of them instead of being `common_link_missing`. Empty when the
    /// lookup cannot be made or finds none.
    pub installed_common_textures: BTreeSet<String>,
    /// The team's slots whose portrait the installed CPKs loaded before the one being compiled
    /// already hold (`common/render/symbol/player/<player id>.dds`). `None` when that lookup
    /// cannot be made: nothing is then known about what is installed, so a `Midcup` export
    /// reports no `portrait_missing`. A `Full` export ignores it.
    pub installed_portrait_slots: Option<BTreeSet<PlayerSlot>>,
}
