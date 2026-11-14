//! One export source read (`spec/team_compiler/pipeline.md` "1. Reader", step 4), from a folder, a
//! `.zip` or a `.7z`: its listing, which routing takes, with its revision, which `compile`
//! checks its reads against, and its file contents, which its check (the small metadata files
//! the structure pass needs, then the deep pass's files) and its tasks read.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use aesthetics_export::{
    CanonicalListing, ListedEntry, ListedKind, SmallMetadata, is_small_metadata,
};
use archives::Archive;
use pipeline::{MemoryBudget, Permit};

use super::{ExportSource, SourceKind};

/// Why a source could not be listed (`export_extract_failed`): the path that failed, as the
/// file system spells it, and the reason.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SourceFailure {
    /// The file or folder that could not be read.
    pub(crate) path: String,
    /// The reason, from the operating system or the archive reader.
    pub(crate) error: String,
}

/// What a source looked like when it was listed: `compile` compares the files a task read with
/// it after the read, and a difference aborts the run. A replacement of the same size that
/// keeps the modified time goes unseen: the check is against a member saving over a file
/// mid-run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceRevision {
    /// A folder: each listed file's stamp, by its path in the source.
    Folder(BTreeMap<String, FileStamp>),
    /// An archive: the archive file's stamp.
    Archive(FileStamp),
}

/// A file's size and modified time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileStamp {
    size: u64,
    modified: SystemTime,
}

impl FileStamp {
    /// The stamp of the file `metadata` describes; a modified time the system cannot give is a
    /// failure naming `path`.
    fn of(path: &Path, metadata: &fs::Metadata) -> Result<FileStamp, SourceFailure> {
        Ok(FileStamp {
            size: metadata.len(),
            modified: metadata.modified().map_err(|error| failure(path, &error))?,
        })
    }

    /// The stamp of the file at `path` now; `None` when it cannot be asked (gone).
    fn now(path: &Path) -> Option<FileStamp> {
        let metadata = fs::metadata(path).ok()?;
        FileStamp::of(path, &metadata).ok()
    }
}

impl SourceRevision {
    /// The first of `files` (paths in the source) that is gone or whose stamp is no longer
    /// the listing's, as the path displayed on this system; for an archive, the archive
    /// itself, whatever `files` holds. `None` when nothing changed.
    pub(crate) fn changed<'a>(
        &self,
        source: &ExportSource,
        files: impl IntoIterator<Item = &'a str>,
    ) -> Option<String> {
        match self {
            SourceRevision::Archive(stamp) => (FileStamp::now(&source.path) != Some(*stamp))
                .then(|| source.path.display().to_string()),
            SourceRevision::Folder(stamps) => files.into_iter().find_map(|path| {
                let file = source.path.join(path);
                // A path the listing never held has no stamp to match: a change too.
                let unchanged = stamps
                    .get(path)
                    .is_some_and(|stamp| FileStamp::now(&file) == Some(*stamp));
                (!unchanged).then(|| file.display().to_string())
            }),
        }
    }
}

/// One source as `list` found it.
pub(super) struct Listed {
    /// Every file (with its size) and every folder in the source.
    pub(super) listing: CanonicalListing,
    /// What the source looked like when it was listed.
    pub(super) revision: SourceRevision,
    /// The bytes a `.7z`'s first read decompresses and charges to the budget
    /// (`decompressed_size`: the sum of every entry the archive holds, files the listing's
    /// parse later drops included); 0 for a folder or a `.zip`, whose reads charge nothing.
    pub(super) decompressed: usize,
}

/// Lists every file (with its size) and every folder in `source`, paths relative to its root
/// joined with `/` (an archive's as `archives` normalized them), with the source's revision,
/// taken from the same metadata, and a `.7z`'s decompressed size. An archive is opened for its
/// entry list only, nothing decompressed, and closed again. An archive that cannot be opened
/// (damaged, encrypted, an entry named outside the root, two entries naming one path) is a
/// failure naming the archive.
pub(super) fn list(source: &ExportSource) -> Result<Listed, SourceFailure> {
    let mut entries = Vec::new();
    let mut decompressed = 0;
    let revision = match source.kind {
        SourceKind::Folder => {
            let mut stamps = BTreeMap::new();
            walk(&source.path, "", &mut entries, &mut stamps)?;
            SourceRevision::Folder(stamps)
        }
        SourceKind::Zip | SourceKind::SevenZ => {
            // Stamped before the archive is opened, so a replacement during the listing
            // leaves a stamp the later check no longer matches.
            let metadata =
                fs::metadata(&source.path).map_err(|error| failure(&source.path, &error))?;
            let stamp = FileStamp::of(&source.path, &metadata)?;
            let seven_z = source.kind == SourceKind::SevenZ;
            let archive = open_archive(&source.path, seven_z)?;
            if seven_z {
                decompressed = decompressed_size(&archive);
            }
            for entry in archive.entries() {
                entries.push(ListedEntry {
                    path: entry.path.clone(),
                    kind: ListedKind::File { size: entry.size },
                });
            }
            // An empty folder exists in an archive only as a directory entry, and an empty
            // kit folder is a placeholder kit: leaving these out would lose the kit.
            for folder in archive.folders() {
                entries.push(ListedEntry {
                    path: folder.clone(),
                    kind: ListedKind::Folder,
                });
            }
            SourceRevision::Archive(stamp)
        }
    };
    let listing = CanonicalListing {
        display_name: source.display_name.clone(),
        entries,
    };
    Ok(Listed {
        listing,
        revision,
        decompressed,
    })
}

/// The file contents of one export (`spec/team_compiler/pipeline.md` "1. Reader", step 4, "Load"),
/// read by its check (the small metadata, then the deep pass) and by its tasks. A folder reads
/// each file from disk. An archive is opened on the first read and stays open for the later
/// reads, so its header is read once per source; a `.7z` is decompressed whole on that first
/// read and held under a permit for its whole decompressed size until the source is dropped
/// or its permit handed on (`into_permit`). A `.7z` that `compile` keeps from its check to its
/// tasks (`validation::keeps_archive`) is read by both through the one source; any other
/// export's tasks read through a source of their own.
///
/// One source is shared by reference by the deep pass's workers. A folder's reads are
/// independent; an archive is one handle, so its reads take turns behind a lock.
pub(crate) struct ContentSource {
    path: PathBuf,
    kind: SourceKind,
    budget: Arc<MemoryBudget>,
    /// An archive source's handle and the permit charged for it, one lock for both: the permit
    /// is acquired on the first read, which opens the archive. Never locked for a folder.
    opened: Mutex<OpenedArchive>,
}

/// An archive opened by its first read, with the `.7z` permit charged for it.
#[derive(Default)]
struct OpenedArchive {
    // Declared before `permit`, so the decompressed buffer is freed before the bytes it was
    // charged are released.
    archive: Option<Archive<File>>,
    permit: Option<Permit>,
}

impl ContentSource {
    /// `source`'s contents, nothing opened yet; a `.7z`'s read is charged to `budget`.
    pub(crate) fn new(source: &ExportSource, budget: &Arc<MemoryBudget>) -> ContentSource {
        ContentSource {
            path: source.path.clone(),
            kind: source.kind,
            budget: Arc::clone(budget),
            opened: Mutex::new(OpenedArchive::default()),
        }
    }

    /// The bytes of the file at `path`, a path of the source as a `FileDescriptor.source` gives
    /// it. A failure names the file or archive that could not be read.
    ///
    /// A folder's file is read with no lock. An archive's is read under its lock, held for this
    /// one read only (a `.zip` inflating one entry, a `.7z` copying out of its buffer), and the
    /// bytes come back owned with the lock released. So no caller holds the lock while it starts
    /// parallel work: a worker waiting on such work takes other work, which could lock again on
    /// the same thread and wait for itself forever.
    pub(crate) fn read(&self, path: &str) -> Result<Vec<u8>, SourceFailure> {
        if self.kind == SourceKind::Folder {
            let file = self.path.join(path);
            return fs::read(&file).map_err(|error| failure(&file, &error));
        }
        let seven_z = self.kind == SourceKind::SevenZ;
        let mut opened = self.opened.lock().unwrap();
        let archive = match opened.archive.take() {
            Some(archive) => archive,
            None => {
                let archive = open_archive(&self.path, seven_z)?;
                if seven_z {
                    let permit = self
                        .budget
                        .acquire(decompressed_size(&archive))
                        .map_err(|cancelled| failure(&self.path, &cancelled))?;
                    opened.permit = Some(permit);
                }
                archive
            }
        };
        opened
            .archive
            .insert(archive)
            .read(path)
            .map_err(|error| SourceFailure {
                path: path.to_owned(),
                error: error.to_string(),
            })
    }

    /// The small metadata files of `listing` (every listed file `is_small_metadata` accepts),
    /// each read through `read`, so a `.7z` read here stays decompressed and charged for the
    /// deep pass's reads. A file that cannot be read carries its reason, which the structure
    /// pass reports as `source_read_failed`. A listing with no metadata file reads nothing, so
    /// opens and charges nothing.
    pub(crate) fn read_metadata(&self, listing: &CanonicalListing) -> SmallMetadata {
        let files = listing
            .entries
            .iter()
            .filter(|entry| {
                matches!(entry.kind, ListedKind::File { .. }) && is_small_metadata(&entry.path)
            })
            .map(|entry| {
                let bytes = self.read(&entry.path).map_err(|failure| failure.error);
                (entry.path.clone(), bytes)
            })
            .collect();
        SmallMetadata { files }
    }

    /// The bytes of the `.7z` permit this source holds: the archive's whole decompressed size
    /// once it was read, 0 for a `.7z` never read, a folder or a `.zip`.
    pub(crate) fn held(&self) -> usize {
        let opened = self.opened.lock().unwrap();
        opened.permit.as_ref().map_or(0, Permit::size)
    }

    /// Closes the source and returns the permit it holds: `Some` only for a `.7z` that was read,
    /// charged for its whole decompressed size. The archive is dropped first, since the permit
    /// stands for its buffer; the caller keeps the permit for the bytes already read out of it.
    pub(crate) fn into_permit(self) -> Option<Permit> {
        let opened = self
            .opened
            .into_inner()
            .expect("no thread panicked while reading from the archive");
        drop(opened.archive);
        opened.permit
    }
}

/// Opens the archive at `path`, a `.7z` when `seven_z`, else a `.zip`: the dispatch the
/// extension decided once, at discovery.
fn open_archive(path: &Path, seven_z: bool) -> Result<Archive<File>, SourceFailure> {
    let file = File::open(path).map_err(|error| failure(path, &error))?;
    let archive = if seven_z {
        Archive::seven_z(file)
    } else {
        Archive::zip(file)
    };
    archive.map_err(|error| failure(path, &error))
}

/// What a `.7z`'s first read decompresses: the sum of its entries' sizes. A sum past `usize` (a
/// 32-bit host only) is over any cap, and so is the saturated value: the request is the same
/// oversized one.
fn decompressed_size(archive: &Archive<File>) -> usize {
    let total: u64 = archive.entries().iter().map(|entry| entry.size).sum();
    usize::try_from(total).unwrap_or(usize::MAX)
}

/// Lists `folder` (at `prefix` within the source) and everything below it into `entries`, and
/// each file's stamp into `stamps`, by the same path.
fn walk(
    folder: &Path,
    prefix: &str,
    entries: &mut Vec<ListedEntry>,
    stamps: &mut BTreeMap<String, FileStamp>,
) -> Result<(), SourceFailure> {
    for entry in fs::read_dir(folder).map_err(|error| failure(folder, &error))? {
        let entry = entry.map_err(|error| failure(folder, &error))?;
        let path = entry.path();
        let name = entry.file_name();
        // Export paths are UTF-8 strings from here on (`ScopePath`); a name that is not cannot
        // be represented, so the source is refused rather than misnamed.
        let Some(name) = name.to_str() else {
            return Err(SourceFailure {
                path: path.display().to_string(),
                error: "the name is not valid UTF-8".to_owned(),
            });
        };
        let relative = if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{prefix}/{name}")
        };
        let metadata = fs::metadata(&path).map_err(|error| failure(&path, &error))?;
        if metadata.is_dir() {
            entries.push(ListedEntry {
                path: relative.clone(),
                kind: ListedKind::Folder,
            });
            walk(&path, &relative, entries, stamps)?;
        } else {
            stamps.insert(relative.clone(), FileStamp::of(&path, &metadata)?);
            entries.push(ListedEntry {
                path: relative,
                kind: ListedKind::File {
                    size: metadata.len(),
                },
            });
        }
    }
    Ok(())
}

fn failure(path: &Path, error: &impl Display) -> SourceFailure {
    SourceFailure {
        path: path.display().to_string(),
        error: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{RecvTimeoutError, channel};
    use std::thread;
    use std::time::Duration;

    use studio_core::ExportId;

    use super::*;
    use crate::testing::{ScratchFolder, scratch};

    /// Long enough that a scheduling hiccup never outlives it; short enough that a permit that
    /// never comes fails the test rather than hanging it.
    const GUARD: Duration = Duration::from_secs(120);
    /// How long "still waiting" takes to observe.
    const BLOCKED: Duration = Duration::from_millis(100);

    fn folder_source(path: PathBuf) -> ExportSource {
        ExportSource {
            export_id: ExportId(0),
            path,
            kind: SourceKind::Folder,
            file_name: "co Midcup Spring".to_owned(),
            display_name: "co Midcup Spring".to_owned(),
            team_name: None,
        }
    }

    /// The archive fixture `tests/fixtures/sources/<name>` as a source, read in place.
    fn archive_source(name: &str) -> ExportSource {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sources")
            .join(name);
        let kind = if name.ends_with(".7z") {
            SourceKind::SevenZ
        } else {
            SourceKind::Zip
        };
        ExportSource {
            kind,
            file_name: name.to_owned(),
            ..folder_source(path)
        }
    }

    fn sorted_entries(listing: &CanonicalListing) -> Vec<(&str, ListedKind)> {
        let mut entries: Vec<(&str, ListedKind)> = listing
            .entries
            .iter()
            .map(|entry| (entry.path.as_str(), entry.kind))
            .collect();
        entries.sort();
        entries
    }

    /// Whether `budget` admits `size` bytes within `GUARD`, asked from another thread so a
    /// budget that never frees fails the test instead of hanging it.
    fn admits(budget: &Arc<MemoryBudget>, size: usize) -> bool {
        let (admitted_tx, admitted) = channel();
        let budget = Arc::clone(budget);
        thread::spawn(move || {
            let permit = budget.acquire(size);
            admitted_tx.send(permit.is_ok()).unwrap();
        });
        admitted.recv_timeout(GUARD) == Ok(true)
    }

    /// Lists the archive fixture `name` and reads its metadata on another thread, under
    /// `budget`; the receiver gets the metadata once the read is done and the source dropped.
    fn read_in_background(
        name: &'static str,
        budget: &Arc<MemoryBudget>,
    ) -> std::sync::mpsc::Receiver<SmallMetadata> {
        let (done_tx, done) = channel();
        let budget = Arc::clone(budget);
        thread::spawn(move || {
            let source = archive_source(name);
            let listing = list(&source).unwrap().listing;
            let content = ContentSource::new(&source, &budget);
            let metadata = content.read_metadata(&listing);
            drop(content);
            done_tx.send(metadata).unwrap();
        });
        done
    }

    /// `work`'s result, run on another thread so a read that waits for a permit that never
    /// comes fails the test instead of hanging it; `None` when it is not done within `GUARD`.
    fn within_guard<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
        let (done_tx, done) = channel();
        thread::spawn(move || done_tx.send(work()).unwrap());
        done.recv_timeout(GUARD).ok()
    }

    #[test]
    fn a_folder_lists_every_file_with_its_size_and_every_folder() {
        let temp = scratch("source_listing");
        let root = temp.path();
        fs::create_dir_all(root.join("Players/03 - A")).unwrap();
        fs::create_dir_all(root.join("Kits/p2")).unwrap();
        fs::write(root.join("Players/03 - A/face_high.fmdl"), "12345").unwrap();
        fs::write(root.join("players.txt"), "03 A").unwrap();

        let listing = list(&folder_source(root.to_path_buf())).unwrap().listing;

        assert_eq!(listing.display_name, "co Midcup Spring");
        assert_eq!(
            sorted_entries(&listing),
            [
                ("Kits", ListedKind::Folder),
                ("Kits/p2", ListedKind::Folder),
                ("Players", ListedKind::Folder),
                ("Players/03 - A", ListedKind::Folder),
                (
                    "Players/03 - A/face_high.fmdl",
                    ListedKind::File { size: 5 }
                ),
                ("players.txt", ListedKind::File { size: 4 }),
            ]
        );
    }

    #[test]
    fn a_folder_that_cannot_be_listed_is_a_failure_naming_it() {
        let temp = scratch("source_unlistable");
        let missing = temp.path().join("gone");
        let Err(failure) = list(&folder_source(missing.clone())) else {
            panic!("listed a missing folder");
        };
        assert_eq!(failure.path, missing.display().to_string());
        assert!(!failure.error.is_empty());
    }

    #[test]
    fn an_archive_lists_its_files_and_its_directory_entries() {
        for name in ["co Midcup Spring.zip", "co Midcup Spring.7z"] {
            let listing = list(&archive_source(name)).unwrap().listing;
            assert_eq!(listing.display_name, "co Midcup Spring");
            assert_eq!(
                sorted_entries(&listing),
                [
                    ("Kits", ListedKind::Folder),
                    ("Kits/p2", ListedKind::Folder),
                    ("notes.txt", ListedKind::File { size: 24 }),
                    ("players.txt", ListedKind::File { size: 10 }),
                ],
                "{name}"
            );
        }
    }

    #[test]
    fn an_archive_that_cannot_be_opened_is_a_failure_naming_it() {
        let source = archive_source("co Midcup Escape.zip");
        let Err(failure) = list(&source) else {
            panic!("listed an entry outside the root");
        };
        assert_eq!(
            failure,
            SourceFailure {
                path: source.path.display().to_string(),
                error: "invalid entry name \"../x\"".to_owned(),
            }
        );
    }

    #[test]
    fn a_7z_is_read_under_a_permit_for_its_whole_size_and_releases_it() {
        // The fixture's entries sum to 34 bytes: over this cap, an oversized request.
        let budget = MemoryBudget::new(1);
        let held = budget.acquire(1).unwrap();

        let done = read_in_background("co Midcup Spring.7z", &budget);

        // An oversized request waits for the budget to empty: the read is charged.
        assert!(matches!(
            done.recv_timeout(BLOCKED),
            Err(RecvTimeoutError::Timeout)
        ));
        drop(held);
        let metadata = done.recv_timeout(GUARD).expect("the read runs alone");
        assert_eq!(metadata.files["players.txt"], Ok(b"01 Keeper\n".to_vec()));
        assert_eq!(
            metadata.files["notes.txt"],
            Ok(b"Spring kit placeholder.\n".to_vec())
        );
        // The permit went with the archive: the whole cap is free again.
        assert!(admits(&budget, 1), "the 7z's permit was not released");
    }

    #[test]
    fn a_7z_with_no_metadata_file_to_read_is_not_charged() {
        let budget = MemoryBudget::new(1);
        let _held = budget.acquire(1).unwrap();
        let (done_tx, done) = channel();
        let unheld = Arc::clone(&budget);
        thread::spawn(move || {
            let source = archive_source("co Midcup Spring.7z");
            let mut listing = list(&source).unwrap().listing;
            listing
                .entries
                .retain(|entry| matches!(entry.kind, ListedKind::Folder));
            let content = ContentSource::new(&source, &unheld);
            done_tx.send(content.read_metadata(&listing)).unwrap();
        });

        let metadata = done
            .recv_timeout(GUARD)
            .expect("nothing to read does not wait for the budget");
        assert!(metadata.files.is_empty());
    }

    #[test]
    fn content_is_read_from_a_folder_a_zip_and_a_7z() {
        let temp = scratch("source_content");
        let root = temp.path();
        fs::create_dir_all(root.join("Kits")).unwrap();
        fs::write(root.join("Kits/notes.txt"), "a note").unwrap();
        let budget = MemoryBudget::new(1 << 20);

        let folder = ContentSource::new(&folder_source(root.to_path_buf()), &budget);
        assert_eq!(folder.read("Kits/notes.txt").unwrap(), b"a note");
        for name in ["co Midcup Spring.zip", "co Midcup Spring.7z"] {
            let archive = ContentSource::new(&archive_source(name), &budget);
            assert_eq!(archive.read("players.txt").unwrap(), b"01 Keeper\n");
            assert_eq!(
                archive.read("notes.txt").unwrap(),
                b"Spring kit placeholder.\n",
                "{name}: a second read from the open archive"
            );
        }
    }

    #[test]
    fn threads_sharing_one_archive_read_the_bytes_a_serial_read_gets() {
        let budget = MemoryBudget::new(1 << 30);
        for name in ["egg Midcup Tracer.zip", "egg Midcup Tracer.7z"] {
            let source = archive_source(name);
            let listing = list(&source).unwrap().listing;
            let paths: Vec<&str> = listing
                .entries
                .iter()
                .filter(|entry| matches!(entry.kind, ListedKind::File { .. }))
                .map(|entry| entry.path.as_str())
                .collect();
            assert!(paths.len() >= 4, "{name}: {paths:?}");
            let serial = ContentSource::new(&source, &budget);
            let expected: Vec<Vec<u8>> = paths
                .iter()
                .map(|path| serial.read(path).unwrap())
                .collect();

            let shared = ContentSource::new(&source, &budget);
            // Each thread reads every file, starting from its own, so different files are
            // read at once from the first read on.
            let read: Vec<Vec<Vec<u8>>> = thread::scope(|scope| {
                let threads: Vec<_> = (0..paths.len())
                    .map(|start| {
                        let (shared, paths) = (&shared, &paths);
                        scope.spawn(move || {
                            let mut bytes = vec![Vec::new(); paths.len()];
                            for offset in 0..paths.len() {
                                let i = (start + offset) % paths.len();
                                bytes[i] = shared.read(paths[i]).unwrap();
                            }
                            bytes
                        })
                    })
                    .collect();
                threads
                    .into_iter()
                    .map(|thread| thread.join().unwrap())
                    .collect()
            });

            for (start, bytes) in read.iter().enumerate() {
                assert!(*bytes == expected, "{name}: thread {start}");
            }
        }
    }

    #[test]
    fn a_7z_holds_its_charge_until_its_content_source_is_dropped() {
        // The fixture's entries sum to 34 bytes, the whole cap: a second byte must wait.
        let budget = MemoryBudget::new(34);
        let content = ContentSource::new(&archive_source("co Midcup Spring.7z"), &budget);
        assert!(
            admits(&budget, 34),
            "nothing is charged before the first read"
        );

        content.read("players.txt").unwrap();

        let (admitted_tx, admitted) = channel();
        let waiting = Arc::clone(&budget);
        thread::spawn(move || {
            admitted_tx.send(waiting.acquire(1).is_ok()).unwrap();
        });
        assert_eq!(
            admitted.recv_timeout(BLOCKED),
            Err(RecvTimeoutError::Timeout),
            "the read is charged"
        );
        drop(content);
        assert_eq!(
            admitted.recv_timeout(GUARD),
            Ok(true),
            "and released with the source"
        );
    }

    #[test]
    fn a_7z_s_metadata_and_its_later_reads_share_one_permit_held_until_the_source_is_dropped() {
        // The fixture's entries sum to 34 bytes, the whole cap: a second permit for the same
        // archive could never be granted while the first is held, so a later read that asked
        // for one would never return.
        let budget = MemoryBudget::new(34);
        let source = archive_source("co Midcup Spring.7z");
        let listing = list(&source).unwrap().listing;
        let content = ContentSource::new(&source, &budget);

        let (content, metadata) = within_guard(move || {
            let metadata = content.read_metadata(&listing);
            (content, metadata)
        })
        .expect("the metadata read waits for nothing");
        assert_eq!(metadata.files["players.txt"], Ok(b"01 Keeper\n".to_vec()));

        let (admitted_tx, admitted) = channel();
        let waiting = Arc::clone(&budget);
        thread::spawn(move || {
            admitted_tx.send(waiting.acquire(1).is_ok()).unwrap();
        });
        // Checked before the later read: a source that let its permit go after the metadata
        // would take a new one there, and look charged again.
        assert_eq!(
            admitted.recv_timeout(BLOCKED),
            Err(RecvTimeoutError::Timeout),
            "the metadata read leaves the 7z charged"
        );

        let (content, notes) = within_guard(move || {
            let notes = content.read("notes.txt");
            (content, notes)
        })
        .expect("a later read asks for no second permit");
        assert_eq!(notes.unwrap(), b"Spring kit placeholder.\n");
        assert_eq!(
            admitted.recv_timeout(BLOCKED),
            Err(RecvTimeoutError::Timeout),
            "still charged while the source lives"
        );

        drop(content);
        assert_eq!(
            admitted.recv_timeout(GUARD),
            Ok(true),
            "and released with the source"
        );
    }

    #[test]
    fn a_read_7z_hands_its_permit_on_and_the_charge_lasts_until_that_permit_is_dropped() {
        // The fixture's entries sum to 34 bytes, the whole cap: a second byte must wait.
        let budget = MemoryBudget::new(34);
        let content = ContentSource::new(&archive_source("co Midcup Spring.7z"), &budget);
        content.read("players.txt").unwrap();

        let permit = content.into_permit().expect("a read 7z holds a permit");

        let (admitted_tx, admitted) = channel();
        let waiting = Arc::clone(&budget);
        thread::spawn(move || {
            admitted_tx.send(waiting.acquire(1).is_ok()).unwrap();
        });
        assert_eq!(
            admitted.recv_timeout(BLOCKED),
            Err(RecvTimeoutError::Timeout),
            "the charge outlives the content source"
        );
        drop(permit);
        assert_eq!(
            admitted.recv_timeout(GUARD),
            Ok(true),
            "and goes with the permit"
        );
    }

    #[test]
    fn a_read_zip_and_an_unread_7z_hand_on_no_permit() {
        let budget = MemoryBudget::new(1 << 20);
        let zip = ContentSource::new(&archive_source("co Midcup Spring.zip"), &budget);
        zip.read("players.txt").unwrap();
        assert!(zip.into_permit().is_none(), "zip");

        let unread = ContentSource::new(&archive_source("co Midcup Spring.7z"), &budget);
        assert!(unread.into_permit().is_none(), "unread 7z");
    }

    #[test]
    fn a_zip_read_is_not_charged() {
        let budget = MemoryBudget::new(1);
        let _held = budget.acquire(1).unwrap();
        let (done_tx, done) = channel();
        let unheld = Arc::clone(&budget);
        thread::spawn(move || {
            let content = ContentSource::new(&archive_source("co Midcup Spring.zip"), &unheld);
            done_tx.send(content.read("players.txt").is_ok()).unwrap();
        });
        assert_eq!(done.recv_timeout(GUARD), Ok(true));
    }

    #[test]
    fn content_that_cannot_be_read_names_the_file_or_the_archive() {
        let budget = MemoryBudget::new(1 << 20);
        let temp = scratch("source_unreadable");
        let root = temp.path();
        let folder = ContentSource::new(&folder_source(root.to_path_buf()), &budget);
        let failure = folder.read("Kits/gone.dds").unwrap_err();
        assert_eq!(
            failure.path,
            root.join("Kits/gone.dds").display().to_string()
        );

        let archive = ContentSource::new(&archive_source("co Midcup Spring.zip"), &budget);
        let failure = archive.read("Kits/gone.dds").unwrap_err();
        assert_eq!(
            failure,
            SourceFailure {
                path: "Kits/gone.dds".to_owned(),
                error: "no such entry \"Kits/gone.dds\"".to_owned(),
            }
        );

        let source = archive_source("co Midcup Escape.zip");
        let refused = ContentSource::new(&source, &budget);
        let failure = refused.read("x").unwrap_err();
        assert_eq!(failure.path, source.path.display().to_string());
    }

    #[test]
    fn a_zip_is_read_without_a_charge() {
        let budget = MemoryBudget::new(1);
        let _held = budget.acquire(1).unwrap();

        let done = read_in_background("co Midcup Spring.zip", &budget);

        let metadata = done
            .recv_timeout(GUARD)
            .expect("a zip read does not wait for the budget");
        assert_eq!(metadata.files["players.txt"], Ok(b"01 Keeper\n".to_vec()));
    }

    #[test]
    fn a_7z_read_in_a_cancelled_run_fails_each_file_with_the_reason() {
        let budget = MemoryBudget::new(1 << 20);
        budget.cancel();
        let source = archive_source("co Midcup Spring.7z");
        let listing = list(&source).unwrap().listing;

        let metadata = ContentSource::new(&source, &budget).read_metadata(&listing);

        let keys: Vec<&str> = metadata.files.keys().map(String::as_str).collect();
        assert_eq!(keys, ["notes.txt", "players.txt"]);
        for result in metadata.files.values() {
            assert_eq!(result, &Err("the run was cancelled".to_owned()));
        }
    }

    #[test]
    fn only_the_small_metadata_files_are_read_and_a_failed_read_keeps_its_reason() {
        let temp = scratch("source_metadata");
        let root = temp.path();
        fs::create_dir_all(root.join("wrapper")).unwrap();
        fs::write(root.join("players.txt"), "03 A").unwrap();
        fs::write(root.join("wrapper/players.txt"), "03 B").unwrap();
        fs::write(root.join("colors.txt"), "not metadata").unwrap();
        let file = |path: &str| ListedEntry {
            path: path.to_owned(),
            kind: ListedKind::File { size: 1 },
        };
        let listing = CanonicalListing {
            display_name: "co Midcup Spring".to_owned(),
            entries: vec![
                file("players.txt"),
                file("wrapper/players.txt"),
                file("colors.txt"),
                file("notes.txt"),
                ListedEntry {
                    path: "refs.txt".to_owned(),
                    kind: ListedKind::Folder,
                },
            ],
        };

        let metadata = ContentSource::new(
            &folder_source(root.to_path_buf()),
            &MemoryBudget::new(1 << 20),
        )
        .read_metadata(&listing);

        let keys: Vec<&str> = metadata.files.keys().map(String::as_str).collect();
        assert_eq!(keys, ["notes.txt", "players.txt", "wrapper/players.txt"]);
        assert_eq!(metadata.files["players.txt"], Ok(b"03 A".to_vec()));
        assert_eq!(metadata.files["wrapper/players.txt"], Ok(b"03 B".to_vec()));
        // Listed but missing on disk: the read fails with the system's reason.
        assert!(metadata.files["notes.txt"].is_err());
    }

    /// How far a test moves a modified time: far past any file system's time granularity.
    const MOVED: Duration = Duration::from_secs(10);

    const FACE: &str = "Players/03 - A/face_high.fmdl";
    const ROSTER: &str = "players.txt";

    /// A scratch folder export holding `FACE` and `ROSTER`, listed: the source and its revision.
    fn listed_folder(name: &str) -> (ScratchFolder, ExportSource, SourceRevision) {
        let temp = scratch(name);
        let root = temp.path();
        fs::create_dir_all(root.join("Players/03 - A")).unwrap();
        fs::write(root.join(FACE), "12345").unwrap();
        fs::write(root.join(ROSTER), "03 A").unwrap();
        let source = folder_source(root.to_path_buf());
        let revision = list(&source).unwrap().revision;
        (temp, source, revision)
    }

    fn modified(path: &Path) -> SystemTime {
        fs::metadata(path).unwrap().modified().unwrap()
    }

    fn set_modified(path: &Path, time: SystemTime) {
        File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(time)
            .unwrap();
    }

    /// `path` in `source` as `changed` names it.
    fn displayed(source: &ExportSource, path: &str) -> Option<String> {
        Some(source.path.join(path).display().to_string())
    }

    #[test]
    fn a_folder_nothing_touched_since_its_listing_has_not_changed() {
        let (_temp, source, revision) = listed_folder("revision_untouched");
        assert!(matches!(&revision, SourceRevision::Folder(stamps) if stamps.len() == 2));
        assert_eq!(revision.changed(&source, [FACE, ROSTER]), None);
    }

    #[test]
    fn a_file_one_byte_longer_with_its_old_modified_time_has_changed() {
        let (_temp, source, revision) = listed_folder("revision_size");
        let face = source.path.join(FACE);
        let listed = modified(&face);
        fs::write(&face, "123456").unwrap();
        set_modified(&face, listed);
        assert_eq!(
            revision.changed(&source, [ROSTER, FACE]),
            displayed(&source, FACE)
        );
    }

    #[test]
    fn a_file_of_the_same_size_with_its_modified_time_moved_has_changed() {
        let (_temp, source, revision) = listed_folder("revision_time");
        let face = source.path.join(FACE);
        let listed = modified(&face);
        fs::write(&face, "54321").unwrap();
        set_modified(&face, listed + MOVED);
        assert_eq!(revision.changed(&source, [FACE]), displayed(&source, FACE));
    }

    #[test]
    fn a_removed_file_has_changed_and_a_file_added_after_the_listing_changes_nothing() {
        let (_temp, source, revision) = listed_folder("revision_removed");
        fs::write(source.path.join("Players/03 - A/boots.fmdl"), "new").unwrap();
        assert_eq!(revision.changed(&source, [FACE, ROSTER]), None);
        fs::remove_file(source.path.join(ROSTER)).unwrap();
        assert_eq!(
            revision.changed(&source, [FACE, ROSTER]),
            displayed(&source, ROSTER)
        );
    }

    #[test]
    fn a_path_the_listing_never_held_has_changed() {
        let (_temp, source, revision) = listed_folder("revision_unlisted");
        let added = "Players/03 - A/boots.fmdl";
        fs::write(source.path.join(added), "new").unwrap();
        assert_eq!(
            revision.changed(&source, [added]),
            displayed(&source, added)
        );
    }

    #[test]
    fn a_changed_file_that_is_not_among_the_files_read_changes_nothing() {
        let (_temp, source, revision) = listed_folder("revision_not_read");
        let face = source.path.join(FACE);
        let listed = modified(&face);
        fs::write(&face, "54321").unwrap();
        set_modified(&face, listed + MOVED);
        assert_eq!(revision.changed(&source, [ROSTER]), None);
    }

    #[test]
    fn an_archive_whose_modified_time_moved_has_changed_whatever_the_files_read() {
        let temp = scratch("revision_archive");
        let fixture = archive_source("co Midcup Spring.zip");
        let source = ExportSource {
            path: temp.path().join("co Midcup Spring.zip"),
            ..archive_source("co Midcup Spring.zip")
        };
        fs::copy(&fixture.path, &source.path).unwrap();
        let revision = list(&source).unwrap().revision;
        assert!(matches!(revision, SourceRevision::Archive(_)));
        assert_eq!(revision.changed(&source, [ROSTER]), None);

        set_modified(&source.path, modified(&source.path) + MOVED);

        let archive = Some(source.path.display().to_string());
        assert_eq!(revision.changed(&source, [ROSTER]), archive);
        assert_eq!(revision.changed(&source, []), archive);
    }
}
