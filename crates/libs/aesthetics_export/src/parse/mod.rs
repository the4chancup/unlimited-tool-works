//! The structure pass's front half: a `CanonicalListing` plus the small
//! metadata bytes → a `ParsedAestheticsExport` (draft, raw roster, issues).
//! Parsing preserves invalid raw input for diagnostics and is source-neutral:
//! the consumer supplies the listing and metadata; this module owns no I/O.

mod draft;
mod identity;
mod roster;

use std::collections::BTreeMap;

use vtree::{ScopePath, VirtualTree};

pub use draft::{
    AestheticsExportDraft, ExportKind, FileDescriptor, FolderDraft, RawRoster, RawRosterEntry,
};
pub use identity::{ExportCoverage, coverage, is_name_separator, team_name};

use crate::conventions::{
    CONTENT_FOLDERS, ContentFolder, classify, is_logo_texture, is_os_artifact,
};
use crate::listing::{CanonicalListing, ListedKind, SmallMetadata};
use crate::validate::{Disposition, IssueScope, ValidationIssue, issue};

/// A listing that cannot become a tree: the consumer's `export_extract_failed`.
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    /// A path escapes the root or is not canonical.
    #[error("{path}: {error}")]
    Path {
        /// The raw listing path that failed.
        path: String,
        /// Why `ScopePath::new` refused it.
        error: vtree::PathError,
    },
    /// Two names fold to one, or a folder entry names a file.
    #[error("{path}: {error}")]
    Collision {
        /// The raw listing path that failed.
        path: String,
        /// Why the insert was refused.
        error: vtree::InsertError,
    },
}

/// The parse output: the draft (everything retained), the raw roster, and the
/// structural issues found while parsing. The consumer's `SmallMetadata`
/// travels with it for later stages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAestheticsExport {
    /// Everything the normalized tree holds.
    pub draft: AestheticsExportDraft,
    /// The authoritative roster file and its raw entries; `None` when the
    /// export has no roster file at all.
    pub raw_roster: Option<RawRoster>,
    /// The small metadata bytes as the consumer supplied them.
    pub metadata: SmallMetadata,
    /// Every structural issue found while parsing.
    pub issues: Vec<ValidationIssue>,
}

/// One canonical file: its value keeps the path it was listed under (the
/// `SmallMetadata` map is keyed by the raw listing path).
struct FileEntry {
    /// The canonical path the source gave it (pre-normalization).
    source: ScopePath,
    /// The raw `ListedEntry::path`, for the metadata map lookup.
    raw: String,
    /// The file's size.
    size: u64,
}

/// The canonical export: files in the tree (implied folders included) plus
/// the listed empty folders the tree cannot see.
struct CanonicalTree {
    files: VirtualTree<FileEntry>,
    /// fold key → canonical path of each listed empty folder (nothing below
    /// it: the tree's implicit folders cover the rest).
    empty_folders: BTreeMap<String, ScopePath>,
}

impl CanonicalTree {
    /// Immediate children under `parent` (`None` = the root): direct files
    /// and direct folders, fold-sorted.
    fn children<'a>(
        &'a self,
        parent: Option<&ScopePath>,
    ) -> (Vec<(ScopePath, &'a FileEntry)>, Vec<ScopePath>) {
        let mut files = Vec::new();
        let mut folders: BTreeMap<String, ScopePath> = BTreeMap::new();
        for entry in self.files.children(parent) {
            match entry {
                vtree::Entry::File(path) => {
                    let entry = self
                        .files
                        .get(&path)
                        .expect("a child of the tree is in the tree");
                    files.push((path, entry));
                }
                vtree::Entry::Folder(path) => {
                    folders.insert(path.fold_key(), path);
                }
            }
        }
        // A retained empty folder contributes itself and every implied
        // ancestor the files cannot see (a listed `Kits/p2` implies `Kits/`).
        let depth = parent.map(|path| path.segments().count()).unwrap_or(0) + 1;
        let parent_key = parent.map(|path| path.fold_key());
        for path in self.empty_folders.values() {
            if path.segments().count() < depth {
                continue;
            }
            let ancestor =
                ScopePath::new(&path.segments().take(depth).collect::<Vec<_>>().join("/"))
                    .expect("a canonical path's prefix is canonical");
            let under = match (ancestor.parent(), &parent_key) {
                (Some(actual), Some(key)) => actual.fold_key() == *key,
                (None, None) => true,
                _ => false,
            };
            if under {
                folders.entry(ancestor.fold_key()).or_insert(ancestor);
            }
        }
        (files, folders.into_values().collect())
    }

    /// Every file under `folder`, in fold order.
    fn files_under<'a>(
        &'a self,
        folder: &ScopePath,
    ) -> impl Iterator<Item = (&'a ScopePath, &'a FileEntry)> {
        self.files.files_under(Some(folder))
    }
}

/// `path`'s last segment.
fn file_name(path: &ScopePath) -> &str {
    path.segments().last().unwrap_or("")
}

/// `path` minus its leading segment, rejoined (`wrapper/Players/x` → `Players/x`).
fn strip_first_segment(path: &ScopePath) -> ScopePath {
    drop_segment(path, 0)
}

/// `path` minus the segment at `depth`, rejoined (`a/x/b` → `a/b` at depth 1).
fn drop_segment(path: &ScopePath, depth: usize) -> ScopePath {
    let kept: Vec<&str> = path
        .segments()
        .enumerate()
        .filter(|(index, _)| *index != depth)
        .map(|(_, segment)| segment)
        .collect();
    ScopePath::new(&kept.join("/")).expect("a canonical path minus a segment is still canonical")
}

/// Whether `folder` directly holds a content folder or a root `logo*` image,
/// which makes it a usable export root. Metadata alone does not make one.
fn is_usable_root(tree: &CanonicalTree, folder: Option<&ScopePath>) -> bool {
    let (files, folders) = tree.children(folder);
    folders.iter().any(|path| {
        CONTENT_FOLDERS
            .iter()
            .any(|(name, _)| file_name(path).eq_ignore_ascii_case(name))
    }) || files
        .iter()
        .any(|(path, _)| is_logo_texture(file_name(path)))
}

/// Lists one export source into a draft: canonicalize, normalize the root,
/// group by content folder, and read the roster. A refused listing is a
/// `SourceError` (the consumer's `export_extract_failed`); everything
/// recoverable is a `ValidationIssue` instead.
pub fn parse_listing(
    listing: CanonicalListing,
    metadata: SmallMetadata,
) -> Result<ParsedAestheticsExport, SourceError> {
    // Step 1: canonicalize into the tree.
    let mut files: VirtualTree<FileEntry> = VirtualTree::new();
    let mut empty_entries = Vec::new();
    for entry in &listing.entries {
        let path = ScopePath::new(&entry.path).map_err(|error| SourceError::Path {
            path: entry.path.clone(),
            error,
        })?;
        match entry.kind {
            // OS artifacts (Thumbs.db, desktop.ini, .DS_Store) never reach the
            // tree and are reported nowhere.
            ListedKind::File { .. } if is_os_artifact(file_name(&path)) => {}
            ListedKind::File { size } => files
                .insert(
                    path.clone(),
                    FileEntry {
                        source: path,
                        raw: entry.path.clone(),
                        size,
                    },
                )
                .map_err(|error| SourceError::Collision {
                    path: entry.path.clone(),
                    error,
                })?,
            ListedKind::Folder => empty_entries.push(path),
        }
    }
    // Folder entries matter only when nothing is below them.
    let mut empty_folders: BTreeMap<String, ScopePath> = BTreeMap::new();
    for path in empty_entries {
        if files.contains_folder(&path) {
            continue;
        }
        if files.contains_file(&path) {
            return Err(SourceError::Collision {
                path: path.as_str().to_owned(),
                error: vtree::InsertError::FileFolderConflict {
                    existing: path.clone(),
                },
            });
        }
        if let Some(existing) = empty_folders.get(&path.fold_key()) {
            if existing != &path {
                return Err(SourceError::Collision {
                    path: path.as_str().to_owned(),
                    error: vtree::InsertError::Collision {
                        existing: existing.clone(),
                    },
                });
            }
            continue;
        }
        empty_folders.insert(path.fold_key(), path);
    }
    let mut tree = CanonicalTree {
        files,
        empty_folders,
    };

    // Step 2: team name and root normalization.
    let team_name = identity::team_name(&listing.display_name);
    let mut issues = Vec::new();
    normalize(&mut tree, &mut issues);

    // Step 3: the draft.
    let draft = build_draft(&listing.display_name, team_name, &tree);

    // Step 4: the roster. An undecided root reports no root-level finding, the roster
    // file's included: it is a root file the ambiguity has not assigned to the export.
    let raw_roster = if issues
        .iter()
        .any(|issue| matches!(issue.code, "nested_root_ambiguous" | "nested_root_conflict"))
    {
        None
    } else {
        roster::read_roster(&draft, &tree, &metadata, &mut issues)
    };

    Ok(ParsedAestheticsExport {
        draft,
        raw_roster,
        metadata,
        issues,
    })
}

/// Step 1 of the coordinator's serial work, applied to the listing: exactly
/// one usable nested root is flattened; several reject the export; loose
/// files colliding with the flattened names reject it too. Then each content
/// folder's doubled `Name/Name` layer is removed.
fn normalize(tree: &mut CanonicalTree, issues: &mut Vec<ValidationIssue>) {
    if !is_usable_root(tree, None) {
        // The root's direct child folders that are usable export roots.
        let (_, children) = tree.children(None);
        let mut usable = Vec::new();
        for path in children {
            if is_usable_root(tree, Some(&path)) {
                usable.push(path);
            }
        }
        if usable.len() > 1 {
            let names = usable
                .iter()
                .map(|path| file_name(path).to_owned())
                .collect::<Vec<_>>()
                .join(", ");
            issues.push(issue(
                "nested_root_ambiguous",
                IssueScope::Export,
                vec![("folders", names)],
                Disposition::DropExport,
            ));
        } else if usable.len() == 1 {
            let child = &usable[0];
            match try_flatten_child(tree, child) {
                Ok(()) => issues.push(issue(
                    "nested_folders_fixed",
                    IssueScope::Export,
                    vec![("folder", file_name(child).to_owned())],
                    Disposition::Keep,
                )),
                Err(conflicts) => {
                    for path in conflicts {
                        issues.push(issue(
                            "nested_root_conflict",
                            IssueScope::File(path),
                            vec![],
                            Disposition::DropExport,
                        ));
                    }
                }
            }
        }
    }

    // A doubled `Name/Name` layer inside a content folder is removed when the
    // inner folder is its only entry and holds no files directly.
    let (_, root_folders) = tree.children(None);
    for (name, _) in CONTENT_FOLDERS {
        let folder = match root_folders
            .iter()
            .find(|path| file_name(path).eq_ignore_ascii_case(name))
        {
            Some(path) => path.clone(),
            None => continue,
        };
        let (files, folders) = tree.children(Some(&folder));
        if files.is_empty() && folders.len() == 1 {
            let inner = folders[0].clone();
            if !file_name(&inner).eq_ignore_ascii_case(name) {
                continue;
            }
            let (inner_files, _) = tree.children(Some(&inner));
            if !inner_files.is_empty() {
                continue;
            }
            remove_layer(tree, &folder, &inner);
            issues.push(issue(
                "nested_folders_fixed",
                IssueScope::Folder(folder.clone()),
                vec![(
                    "folder",
                    format!("{}/{}", file_name(&folder), file_name(&inner)),
                )],
                Disposition::Keep,
            ));
        }
    }
}

/// Moves `child`'s contents to the root (`wrapper/Players/x` → `Players/x`)
/// by building the moved tree through insertion: every insert error is a name
/// claimed twice — `Err(conflicts)`, the moved tree discarded and the export
/// unmoved. `child` itself is gone when it succeeds — it became the root (its
/// listed folder entry, if any, was dropped when its files went in the tree;
/// only empties are held).
fn try_flatten_child(tree: &mut CanonicalTree, child: &ScopePath) -> Result<(), Vec<ScopePath>> {
    let prefix = format!("{}/", child.fold_key());
    let mut moved = VirtualTree::new();
    let mut conflicts = Vec::new();
    let insert = |moved: &mut VirtualTree<FileEntry>, path: ScopePath, entry: &FileEntry| {
        moved
            .insert(
                path.clone(),
                FileEntry {
                    source: entry.source.clone(),
                    raw: entry.raw.clone(),
                    size: entry.size,
                },
            )
            .map_err(|error| match error {
                vtree::InsertError::Duplicate => path.clone(),
                vtree::InsertError::Collision { existing }
                | vtree::InsertError::FileFolderConflict { existing } => existing,
            })
    };
    // The loose files first, then the flattened ones.
    for (path, entry) in tree.files.iter() {
        if !path.fold_key().starts_with(&prefix) {
            insert(&mut moved, path.clone(), entry)
                .expect("the loose files already share the tree");
        }
    }
    for (path, entry) in tree.files.iter() {
        if path.fold_key().starts_with(&prefix)
            && let Err(path) = insert(&mut moved, strip_first_segment(path), entry)
        {
            conflicts.push(path);
        }
    }
    if conflicts.is_empty() {
        let mut empty_folders = BTreeMap::new();
        for (key, path) in std::mem::take(&mut tree.empty_folders) {
            // The flattened wrapper's own entry: it was the root, so it is gone with it.
            if key == child.fold_key() {
                continue;
            }
            let new_path = if key.starts_with(&prefix) {
                strip_first_segment(&path)
            } else {
                path
            };
            empty_folders.insert(new_path.fold_key(), new_path);
        }
        tree.files = moved;
        tree.empty_folders = empty_folders;
        Ok(())
    } else {
        conflicts.sort_by_key(|path| path.fold_key());
        conflicts.dedup_by(|a, b| a.fold_key() == b.fold_key());
        Err(conflicts)
    }
}

/// Removes `inner`'s extra `Name` layer under `folder` (`Players/Players/x` →
/// `Players/x`): the layer is a segment, removed from every path below it.
fn remove_layer(tree: &mut CanonicalTree, folder: &ScopePath, inner: &ScopePath) {
    let prefix = format!("{}/", inner.fold_key());
    // The layer segment sits one past `folder`'s depth.
    let depth = folder.segments().count();
    let mut moved = VirtualTree::new();
    for (path, entry) in tree.files.iter() {
        let new_path = if path.fold_key().starts_with(&prefix) {
            drop_segment(path, depth)
        } else {
            path.clone()
        };
        moved
            .insert(
                new_path,
                FileEntry {
                    source: entry.source.clone(),
                    raw: entry.raw.clone(),
                    size: entry.size,
                },
            )
            .expect("a doubled layer holds only folders, nothing collides");
    }
    let mut empty_folders = BTreeMap::new();
    for (key, path) in std::mem::take(&mut tree.empty_folders) {
        // The doubled layer's own entry: it was the layer, so it is gone with it.
        if key == inner.fold_key() {
            continue;
        }
        let new_path = if key.starts_with(&prefix) {
            drop_segment(&path, depth)
        } else {
            path
        };
        empty_folders.insert(new_path.fold_key(), new_path);
    }
    tree.files = moved;
    tree.empty_folders = empty_folders;
}

/// A file's draft descriptor: canonical path, raw source path, size, kind.
fn descriptor(path: &ScopePath, entry: &FileEntry) -> FileDescriptor {
    FileDescriptor {
        path: path.clone(),
        source: entry.source.clone(),
        size: entry.size,
        kind: classify(file_name(path)),
    }
}

/// `folder`'s direct files (strays) and each direct child folder as a draft
/// group of the folder kind it names (`Players/03 - A`, `Kits/all`).
fn folder_drafts(
    tree: &CanonicalTree,
    folder: &ScopePath,
    stray: &mut Vec<FileDescriptor>,
    target: &mut Vec<FolderDraft>,
) {
    let (files, folders) = tree.children(Some(folder));
    for (path, entry) in files {
        stray.push(descriptor(&path, entry));
    }
    for folder in folders {
        target.push(FolderDraft {
            path: folder.clone(),
            files: tree
                .files_under(&folder)
                .map(|(path, entry)| descriptor(path, entry))
                .collect(),
        });
    }
}

/// The draft grouping from `Object Model`'s "Structure pass types".
fn build_draft(
    display_name: &str,
    team_name: Option<teams_list::TeamName>,
    tree: &CanonicalTree,
) -> AestheticsExportDraft {
    // A referee export carries no tag: it is always full.
    let coverage = if team_name
        .as_ref()
        .is_some_and(teams_list::TeamName::is_referees)
    {
        Some(ExportCoverage::Full)
    } else {
        identity::coverage(display_name)
    };
    let mut draft = AestheticsExportDraft {
        export_display_name: display_name.to_owned(),
        team_name,
        coverage,
        root_files: Vec::new(),
        root_folders: Vec::new(),
        stray_files: Vec::new(),
        players: Vec::new(),
        faces: Vec::new(),
        boots: Vec::new(),
        gloves: Vec::new(),
        kits: Vec::new(),
        portraits: Vec::new(),
        collars: Vec::new(),
        common: Vec::new(),
    };
    let (root_files, root_folders) = tree.children(None);
    for (path, entry) in root_files {
        draft.root_files.push(descriptor(&path, entry));
    }
    for folder in root_folders {
        let content = CONTENT_FOLDERS
            .iter()
            .find(|(name, _)| file_name(&folder).eq_ignore_ascii_case(name))
            .map(|(_, kind)| *kind);
        let Some(content) = content else {
            draft.root_folders.push(folder.clone());
            continue;
        };
        match content {
            ContentFolder::Players => {
                folder_drafts(tree, &folder, &mut draft.stray_files, &mut draft.players)
            }
            ContentFolder::Faces => {
                folder_drafts(tree, &folder, &mut draft.stray_files, &mut draft.faces)
            }
            ContentFolder::Boots => {
                folder_drafts(tree, &folder, &mut draft.stray_files, &mut draft.boots)
            }
            ContentFolder::Gloves => {
                folder_drafts(tree, &folder, &mut draft.stray_files, &mut draft.gloves)
            }
            ContentFolder::Kits => {
                folder_drafts(tree, &folder, &mut draft.stray_files, &mut draft.kits)
            }
            ContentFolder::Portraits => draft
                .portraits
                .extend(tree.files_under(&folder).map(|(p, e)| descriptor(p, e))),
            ContentFolder::Collars => draft
                .collars
                .extend(tree.files_under(&folder).map(|(p, e)| descriptor(p, e))),
            ContentFolder::Common => draft
                .common
                .extend(tree.files_under(&folder).map(|(p, e)| descriptor(p, e))),
        }
    }
    draft
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ISSUE_CODES;
    use crate::testing::{listing, parsed};

    fn parse(name: &str, files: &[(&str, u64)], folders: &[&str]) -> ParsedAestheticsExport {
        parsed(name, files, folders, &[])
    }

    #[test]
    fn os_artifacts_never_reach_the_draft() {
        let parsed = parse(
            "egg Midcup",
            &[
                ("Thumbs.db", 5),
                ("desktop.ini", 5),
                (".DS_Store", 5),
                ("Players/03 - A/Thumbs.DB", 5),
                ("Players/03 - A/face_high.fmdl", 10),
            ],
            &[],
        );
        assert!(parsed.draft.root_files.is_empty());
        assert_eq!(
            parsed.draft.players[0]
                .files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            vec!["Players/03 - A/face_high.fmdl"]
        );
        // A folder that held only an artifact does not exist in the tree.
        let artifacts = parse("egg Midcup", &[("wrapper/Thumbs.db", 5)], &[]);
        assert!(artifacts.draft.root_folders.is_empty());
    }

    #[test]
    fn canonicalization_accepts_backslashes_and_normalizes() {
        let parsed = parse("egg Midcup", &[("Players\\face_high.fmdl", 4)], &[]);
        assert_eq!(
            parsed.draft.stray_files[0].path.as_str(),
            "Players/face_high.fmdl"
        );
        assert_eq!(
            parsed.draft.stray_files[0].source.as_str(),
            "Players/face_high.fmdl"
        );
    }

    #[test]
    fn canonicalization_refuses_traversal_and_collisions() {
        let error = parse_listing(
            listing("egg Midcup", &[("../x", 1)], &[]),
            SmallMetadata {
                files: BTreeMap::new(),
            },
        )
        .unwrap_err();
        assert!(matches!(error, SourceError::Path { .. }));

        let error = parse_listing(
            listing("egg Midcup", &[("a/B.dds", 1), ("a/b.dds", 2)], &[]),
            SmallMetadata {
                files: BTreeMap::new(),
            },
        )
        .unwrap_err();
        assert!(matches!(error, SourceError::Collision { .. }));

        // The same file twice is a collision too.
        let error = parse_listing(
            listing("egg Midcup", &[("a/b.dds", 1), ("a/b.dds", 1)], &[]),
            SmallMetadata {
                files: BTreeMap::new(),
            },
        )
        .unwrap_err();
        assert!(matches!(error, SourceError::Collision { .. }));
    }

    #[test]
    fn empty_folder_entries_give_folder_drafts() {
        let parsed = parse("egg Midcup", &[], &[("Kits/p2")]);
        assert_eq!(parsed.draft.kits.len(), 1);
        assert!(parsed.draft.kits[0].files.is_empty());
        assert_eq!(parsed.draft.kits[0].path.as_str(), "Kits/p2");
    }

    #[test]
    fn a_folder_entry_with_files_below_is_dropped() {
        let parsed = parse("egg Midcup", &[("Kits/p2/kit.dds", 10)], &[("Kits/p2")]);
        assert_eq!(parsed.draft.kits.len(), 1);
        assert_eq!(parsed.draft.kits[0].files.len(), 1);
    }

    #[test]
    fn two_empty_folders_folding_differently_collide() {
        let error = parse_listing(
            listing("egg Midcup", &[], &[("Kits/p2"), ("Kits/P2")]),
            SmallMetadata {
                files: BTreeMap::new(),
            },
        )
        .unwrap_err();
        assert!(matches!(error, SourceError::Collision { .. }));
    }

    #[test]
    fn a_usable_root_needs_no_normalization() {
        let parsed = parse("egg Midcup", &[("Players/03 - A/face_high.fmdl", 10)], &[]);
        assert!(parsed.issues.is_empty());
        assert_eq!(parsed.draft.players.len(), 1);
    }

    #[test]
    fn a_single_usable_child_is_flattened() {
        let parsed = parse(
            "egg Midcup",
            &[
                ("notes.txt", 5),
                ("wrapper/Players/03 - A/face_high.fmdl", 10),
            ],
            &[],
        );
        let folders_fixed = parsed
            .issues
            .iter()
            .find(|issue| issue.code == "nested_folders_fixed")
            .unwrap();
        assert_eq!(
            folders_fixed.context,
            vec![("folder", "wrapper".to_owned())]
        );
        assert_eq!(folders_fixed.disposition, Disposition::Keep);
        assert_eq!(parsed.draft.players.len(), 1);
        let descriptor = &parsed.draft.players[0].files[0];
        assert_eq!(descriptor.path.as_str(), "Players/03 - A/face_high.fmdl");
        assert_eq!(
            descriptor.source.as_str(),
            "wrapper/Players/03 - A/face_high.fmdl"
        );
        assert_eq!(parsed.draft.root_files[0].path.as_str(), "notes.txt");
        // The flattened child is gone: it is the root now, not a stray folder.
        assert!(parsed.draft.root_folders.is_empty());
    }

    #[test]
    fn a_child_usable_by_its_logo_is_flattened() {
        // The `logo` prefix matches case-insensitively.
        for logo in ["logo.png", "Logo.png"] {
            let path = format!("wrapper/{logo}");
            let parsed = parse("egg Midcup", &[(path.as_str(), 10)], &[]);
            assert_eq!(parsed.draft.root_files[0].path.as_str(), logo, "{logo}");
        }
    }

    #[test]
    fn a_non_ascii_root_file_does_not_panic() {
        // `ロゴ.png` is a usable root only through `wrapper/`; the logo check
        // must not slice a multi-byte name.
        let parsed = parse(
            "egg Midcup",
            &[
                ("ロゴ.png", 10),
                ("wrapper/Players/03 - A/face_high.fmdl", 10),
            ],
            &[],
        );
        assert_eq!(parsed.draft.players.len(), 1);
    }

    // TC-STR-02
    #[test]
    fn two_usable_children_are_ambiguous() {
        let parsed = parse(
            "egg Midcup",
            &[
                ("wrapper/Players/03 - A/face_high.fmdl", 10),
                ("other/Kits/p1/kit.dds", 10),
            ],
            &[],
        );
        let ambiguous = parsed
            .issues
            .iter()
            .find(|issue| issue.code == "nested_root_ambiguous")
            .unwrap();
        assert_eq!(ambiguous.disposition, Disposition::DropExport);
        assert_eq!(
            ambiguous.context,
            vec![("folders", "other, wrapper".to_owned())]
        );
        // Nothing moved.
        assert!(parsed.draft.players.is_empty());
    }

    // TC-STR-03
    #[test]
    fn a_loose_file_colliding_with_a_flattened_one_rejects() {
        let parsed = parse(
            "egg Midcup",
            &[
                ("notes.txt", 1),
                ("wrapper/notes.txt", 2),
                ("wrapper/Players/03 - A/face_high.fmdl", 10),
            ],
            &[],
        );
        let conflicts: Vec<_> = parsed
            .issues
            .iter()
            .filter(|issue| issue.code == "nested_root_conflict")
            .collect();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].disposition, Disposition::DropExport);
        assert_eq!(
            conflicts[0].scope,
            IssueScope::File(ScopePath::new("notes.txt").unwrap())
        );
        // The tree stays unmoved.
        assert!(parsed.draft.players.is_empty());
        assert!(
            parsed
                .issues
                .iter()
                .all(|issue| issue.code != "nested_folders_fixed")
        );
    }

    #[test]
    fn flattening_conflicts_list_in_fold_key_order() {
        // The loose `a.txt` collides before `a`'s file/folder conflict is
        // met (`.` sorts before `/` inside `wrapper/`); the findings come
        // out fold-sorted anyway: `a`, then `a.txt`.
        let parsed = parse(
            "egg Midcup",
            &[
                ("a", 1),
                ("a.txt", 1),
                ("wrapper/a.txt", 2),
                ("wrapper/a/x", 2),
                ("wrapper/Players/03 - A/face_high.fmdl", 10),
            ],
            &[],
        );
        let conflicts: Vec<IssueScope> = parsed
            .issues
            .iter()
            .filter(|issue| issue.code == "nested_root_conflict")
            .map(|issue| issue.scope.clone())
            .collect();
        assert_eq!(
            conflicts,
            vec![
                IssueScope::File(ScopePath::new("a").unwrap()),
                IssueScope::File(ScopePath::new("a.txt").unwrap()),
            ]
        );
    }

    #[test]
    fn no_content_is_no_issue() {
        let parsed = parse("egg Midcup", &[("readme.txt", 5)], &[]);
        assert!(parsed.issues.is_empty());
    }

    #[test]
    fn a_doubled_content_folder_layer_is_removed() {
        let parsed = parse(
            "egg Midcup",
            &[("Players/Players/03 - A/face_high.fmdl", 10)],
            &[],
        );
        let fixed = parsed
            .issues
            .iter()
            .find(|issue| issue.code == "nested_folders_fixed")
            .unwrap();
        assert_eq!(
            fixed.scope,
            IssueScope::Folder(ScopePath::new("Players").unwrap())
        );
        assert_eq!(
            fixed.context,
            vec![("folder", "Players/Players".to_owned())]
        );
        assert_eq!(parsed.draft.players.len(), 1);
        assert_eq!(
            parsed.draft.players[0].files[0].path.as_str(),
            "Players/03 - A/face_high.fmdl"
        );
    }

    #[test]
    fn a_doubled_layer_removal_keeps_the_empty_folders() {
        // The layer removal rebuilds the tree; the listed empty `Kits/p2`
        // holds no file and must be carried over.
        let parsed = parse(
            "egg Midcup",
            &[("Players/Players/03 - A/face_high.fmdl", 10)],
            &["Kits/p2"],
        );
        assert!(
            parsed
                .issues
                .iter()
                .any(|issue| issue.code == "nested_folders_fixed")
        );
        assert_eq!(
            parsed
                .draft
                .kits
                .iter()
                .map(|kit| kit.path.as_str())
                .collect::<Vec<_>>(),
            vec!["Kits/p2"]
        );
    }

    #[test]
    fn a_doubled_layer_is_removed_in_any_case_of_the_content_folder() {
        // `PLAYERS` is the `Players` content folder, so `PLAYERS/Players/` is
        // the doubled layer and is removed like `Players/Players/`.
        let parsed = parse(
            "egg Midcup",
            &[("PLAYERS/Players/03 - A/face_high.fmdl", 10)],
            &[],
        );
        assert!(
            parsed
                .issues
                .iter()
                .any(|issue| issue.code == "nested_folders_fixed")
        );
        assert_eq!(parsed.draft.players[0].path.as_str(), "PLAYERS/03 - A");
        assert_eq!(
            parsed.draft.players[0].files[0].path.as_str(),
            "PLAYERS/03 - A/face_high.fmdl"
        );
    }

    #[test]
    fn an_inner_folder_holding_files_is_content() {
        let parsed = parse("egg Midcup", &[("Boots/Boots/boots.fmdl", 10)], &[]);
        assert!(
            parsed
                .issues
                .iter()
                .all(|issue| issue.code != "nested_folders_fixed")
        );
        assert_eq!(parsed.draft.boots.len(), 1);
        assert_eq!(parsed.draft.boots[0].path.as_str(), "Boots/Boots");
    }

    #[test]
    fn a_doubled_layer_with_other_entries_stays() {
        // `Players/` holding a stray file beside `Players/Players/` is not a
        // doubled layer: the inner folder is not the only entry.
        let parsed = parse(
            "egg Midcup",
            &[
                ("Players/loose.fmdl", 4),
                ("Players/Players/03 - A/face_high.fmdl", 10),
            ],
            &[],
        );
        assert!(
            parsed
                .issues
                .iter()
                .all(|issue| issue.code != "nested_folders_fixed")
        );
        assert_eq!(parsed.draft.players[0].path.as_str(), "Players/Players");
    }

    #[test]
    fn the_doubled_layer_is_found_past_other_content_folders() {
        // A sibling content folder sorts before `Players` in fold order: the
        // doubled layer is still found and removed.
        let parsed = parse(
            "egg Midcup",
            &[
                ("Kits/p1/kit.dds", 9),
                ("Players/Players/03 - A/face_high.fmdl", 10),
            ],
            &[],
        );
        assert_eq!(parsed.draft.players.len(), 1);
        assert_eq!(
            parsed.draft.players[0].files[0].path.as_str(),
            "Players/03 - A/face_high.fmdl"
        );
    }

    #[test]
    fn a_non_logo_texture_does_not_make_a_root_usable() {
        let parsed = parse(
            "egg Midcup",
            &[("x.dds", 4), ("wrapper/Players/03 - A/face_high.fmdl", 10)],
            &[],
        );
        // `x.dds` is no `logo*`: the root is not usable, `wrapper/` is the
        // usable child and is flattened.
        assert_eq!(parsed.draft.players.len(), 1);
        assert_eq!(
            parsed.draft.players[0].files[0].path.as_str(),
            "Players/03 - A/face_high.fmdl"
        );
    }

    #[test]
    fn draft_groups_the_root_and_content_folders() {
        let parsed = parse(
            "egg Midcup",
            &[
                ("readme.txt", 3),
                ("wrapper/x.txt", 1),
                ("Players/players.txt", 7),
                ("Players/03 - A/face_high.fmdl", 10),
                ("Portraits/x/01.dds", 9),
                ("Kits/all/kit_back.dds", 9),
                ("Faces/Longhair/face_high.fmdl", 10),
                ("Boots/Crocs/boots.fmdl", 10),
                ("Gloves/Keeper gloves/glove_l.fmdl", 10),
                ("Collars/collar_101.dds", 9),
                ("Common/hair.dds", 9),
            ],
            &[],
        );
        assert_eq!(
            parsed
                .draft
                .root_files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            vec!["readme.txt"]
        );
        assert_eq!(
            parsed
                .draft
                .root_folders
                .iter()
                .map(ScopePath::as_str)
                .collect::<Vec<_>>(),
            vec!["wrapper"]
        );
        assert_eq!(
            parsed.draft.stray_files[0].path.as_str(),
            "Players/players.txt"
        );
        assert_eq!(
            parsed.draft.portraits[0].path.as_str(),
            "Portraits/x/01.dds"
        );
        assert_eq!(parsed.draft.kits[0].path.as_str(), "Kits/all");
        assert_eq!(parsed.draft.players.len(), 1);
        assert_eq!(parsed.draft.faces[0].path.as_str(), "Faces/Longhair");
        assert_eq!(parsed.draft.boots[0].path.as_str(), "Boots/Crocs");
        assert_eq!(parsed.draft.gloves[0].path.as_str(), "Gloves/Keeper gloves");
        assert_eq!(
            parsed.draft.collars[0].path.as_str(),
            "Collars/collar_101.dds"
        );
        assert_eq!(parsed.draft.common[0].path.as_str(), "Common/hair.dds");
    }

    #[test]
    fn a_lowercase_players_folder_is_the_content_folder() {
        // "players/Players folders merged": the content-folder match is
        // case-insensitive, so a lowercase folder is the same content folder.
        let parsed = parse("egg Midcup", &[("players/05 - B/hair_high.fmdl", 10)], &[]);
        assert_eq!(parsed.draft.players.len(), 1);
        assert_eq!(parsed.draft.players[0].path.as_str(), "players/05 - B");
    }

    #[test]
    fn issue_codes_have_no_duplicates() {
        let mut sorted = ISSUE_CODES.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ISSUE_CODES.len());
    }
}
