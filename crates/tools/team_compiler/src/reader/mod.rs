//! Stage 1, where exports come from (`spec/team_compiler/pipeline.md` "1. Reader", steps 1 and 2):
//! discovery of the export sources, their team names, and the routing that sets aside the
//! disabled, balls and conflicting referee exports before any validation. The only module that
//! touches export sources.

mod source;

use std::fs;
use std::path::{Path, PathBuf};

use aesthetics_export::{CanonicalListing, ListedKind};
use anyhow::Context;
use rayon::prelude::*;
use studio_core::ExportId;
use teams_list::TeamName;

pub(crate) use source::{ContentSource, SourceFailure, SourceRevision};
use source::{Listed, list};

/// One export source found by discovery: a folder or an archive.
#[derive(Debug)]
pub(crate) struct ExportSource {
    /// The source's id for this run, sequential in discovery order.
    pub(crate) export_id: ExportId,
    /// Where the source is on disk.
    pub(crate) path: PathBuf,
    /// What the source is, which decides how it is read.
    pub(crate) kind: SourceKind,
    /// The folder or archive name, extension included: what every console line names, so a
    /// folder and an archive sharing a stem stay apart.
    pub(crate) file_name: String,
    /// The stem: the folder name, or the archive name without extension.
    pub(crate) display_name: String,
    /// The canonical team name from the stem; `None` when the stem has no token, or when its
    /// first token does not fold to a valid team name.
    pub(crate) team_name: Option<TeamName>,
}

impl ExportSource {
    /// Whether the source's team name is the referees' (`refs`): a refs export, known before
    /// any of its files is read.
    pub(crate) fn is_referees(&self) -> bool {
        self.team_name.as_ref().is_some_and(TeamName::is_referees)
    }
}

/// What an export source is, which decides how it is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceKind {
    /// A plain folder.
    Folder,
    /// A `.zip`: read one entry at a time.
    Zip,
    /// A `.7z`: its first read decompresses the whole archive.
    SevenZ,
}

/// What the reader decided for one source before validation.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Route {
    /// The source could not be listed: `export_extract_failed`.
    Unreadable(SourceFailure),
    /// A `NO_USE` marker at its root: `export_disabled`, and nothing else.
    Disabled,
    /// A `/balls/` export, the Balls compiler's: `export_balls_skipped`.
    Balls,
    /// One of several non-disabled `/refs/` exports: `multiple_ref_exports`.
    ConflictingRefs,
    /// To be validated, from this listing.
    Validate {
        /// Every file and folder in the source.
        listing: CanonicalListing,
        /// What the source looked like when it was listed, which `compile` checks its reads
        /// against.
        revision: SourceRevision,
        /// The bytes a `.7z`'s first read decompresses and charges to the budget (the sum of
        /// its entries' sizes, every entry the archive holds); 0 for a folder or a `.zip`.
        /// `compile`'s keep rule reads it for every `.7z` before any is checked
        /// (`validation::keeps_archive`).
        decompressed: usize,
    },
}

/// Whether `path` names a `.zip` or `.7z` archive.
pub(crate) fn is_archive(path: &Path) -> bool {
    archive_kind(path).is_some()
}

/// The archive kind `path`'s extension names, compared ASCII-case-insensitively, as Windows
/// treats file names; `None` for any other extension.
fn archive_kind(path: &Path) -> Option<SourceKind> {
    let extension = path.extension()?.to_str()?;
    if extension.eq_ignore_ascii_case("zip") {
        Some(SourceKind::Zip)
    } else if extension.eq_ignore_ascii_case("7z") {
        Some(SourceKind::SevenZ)
    } else {
        None
    }
}

/// What `path` is as an export source: a folder, or an archive by its extension.
fn source_kind(path: &Path) -> Option<SourceKind> {
    if path.is_dir() {
        Some(SourceKind::Folder)
    } else {
        archive_kind(path)
    }
}

/// What discovery found in the exports root.
#[derive(Debug)]
pub(crate) struct Discovery {
    /// The export sources of the run, sorted by folded file name then the name itself and
    /// numbered in that order.
    pub(crate) sources: Vec<ExportSource>,
    /// The `.upgrading` folders the scan skipped: the Export upgrader's staging folder an
    /// interrupted upgrade left is no export. The run reports them as
    /// `export_upgrade_unfinished`, in the same order.
    pub(crate) skipped: Vec<PathBuf>,
}

/// The export sources of a run: exactly the `exports` paths when any are given (the root is then
/// not scanned — a named path is taken as it is, `.upgrading` ending or not), else every folder
/// and `.zip`/`.7z` file directly in `exports_root`, other files ignored, a folder whose name
/// ends in `.upgrading` skipped aside (`skipped`), sorted by folded file name, then by the name
/// itself, so the order is the same on every file system, names differing only in case included,
/// and numbered in that order. An exports root that cannot be listed is an error naming it, and
/// so is a named path that is no longer a folder or an archive.
pub(crate) fn discover(exports_root: &Path, exports: &[PathBuf]) -> anyhow::Result<Discovery> {
    let (mut paths, mut skipped) = if exports.is_empty() {
        let ScanRoot { paths, skipped } = scan_root(exports_root)?;
        (paths, skipped)
    } else {
        (named_paths(exports)?, Vec::new())
    };
    let by_name = |path: &Path| {
        let name = file_name(path);
        (vtree::fold_name(&name), name)
    };
    paths.sort_by_cached_key(|(path, _)| by_name(path));
    skipped.sort_by_cached_key(|path| by_name(path));
    Ok(Discovery {
        sources: paths
            .into_iter()
            .zip(0..)
            .map(|((path, kind), id)| source(path, kind, ExportId(id)))
            .collect(),
        skipped,
    })
}

/// What `scan_root` read in the exports root.
struct ScanRoot {
    /// The sources found: a folder or an archive each, unsorted.
    paths: Vec<(PathBuf, SourceKind)>,
    /// The `.upgrading` folders skipped.
    skipped: Vec<PathBuf>,
}

/// The exports root's sources, and the `.upgrading` folders skipped beside them.
fn scan_root(exports_root: &Path) -> anyhow::Result<ScanRoot> {
    let cannot_read = || format!("cannot read the exports folder {}", exports_root.display());
    let mut paths = Vec::new();
    let mut skipped = Vec::new();
    for entry in fs::read_dir(exports_root).with_context(cannot_read)? {
        let path = entry.with_context(cannot_read)?.path();
        if path.is_dir()
            && file_name(&path)
                .to_ascii_lowercase()
                .ends_with(".upgrading")
        {
            skipped.push(path);
            continue;
        }
        if let Some(kind) = source_kind(&path) {
            paths.push((path, kind));
        }
    }
    Ok(ScanRoot { paths, skipped })
}

/// The `--export` paths with their kinds. The preflight refused any other path; one that
/// changed since is an error naming it.
fn named_paths(exports: &[PathBuf]) -> anyhow::Result<Vec<(PathBuf, SourceKind)>> {
    exports
        .iter()
        .map(|path| match source_kind(path) {
            Some(kind) => Ok((path.clone(), kind)),
            None => anyhow::bail!(
                "--export {} is not a folder, a .zip or a .7z file",
                path.display()
            ),
        })
        .collect()
}

/// The folder or archive name at the end of `path`. A path ending in `.` or `..`, which
/// `Path::file_name` cannot name, takes the canonical path's last component: `--export ..`
/// names the folder it resolves to, since a relative command-line path resolves against the
/// current directory. Only when the path cannot be canonicalized
/// (it vanished since the preflight; the read reports that) does the path as given stand in.
fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || {
            fs::canonicalize(path)
                .ok()
                .and_then(|canonical| {
                    canonical
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                })
                .unwrap_or_else(|| path.display().to_string())
        },
        |name| name.to_string_lossy().into_owned(),
    )
}

/// The source at `path`, of `kind`.
fn source(path: PathBuf, kind: SourceKind, export_id: ExportId) -> ExportSource {
    let file_name = file_name(&path);
    let display_name = match kind {
        SourceKind::Folder => file_name.clone(),
        SourceKind::Zip | SourceKind::SevenZ => path.file_stem().map_or_else(
            || file_name.clone(),
            |stem| stem.to_string_lossy().into_owned(),
        ),
    };
    let team_name = aesthetics_export::team_name(&display_name);
    ExportSource {
        export_id,
        path,
        kind,
        file_name,
        display_name,
        team_name,
    }
}

/// Each source's route, in the order given; the sources are listed in parallel, on the rayon
/// pool the caller runs this in. The duplicate-refs rule counts only the `/refs/` exports still
/// headed for validation, so a disabled or unreadable one never conflicts.
pub(crate) fn route(sources: &[ExportSource]) -> Vec<Route> {
    // A source is only listed here, nothing read that waits for memory. The indexed `collect`
    // keeps the source order.
    let mut routes: Vec<Route> = sources.par_iter().map(route_source).collect();
    let refs: Vec<usize> = sources
        .iter()
        .zip(&routes)
        .enumerate()
        .filter(|(_, (source, route))| {
            matches!(route, Route::Validate { .. }) && source.is_referees()
        })
        .map(|(index, _)| index)
        .collect();
    if refs.len() > 1 {
        for index in refs {
            routes[index] = Route::ConflictingRefs;
        }
    }
    routes
}

/// One source's route before the duplicate-refs rule, from its listing alone: a disabled export
/// is recognized by its root's files, a balls export by its team name. No source is read
/// beyond its listing, so no `.7z` is decompressed by routing, and a disabled, balls or
/// conflicting-refs one never is.
fn route_source(source: &ExportSource) -> Route {
    let Listed {
        listing,
        revision,
        decompressed,
    } = match list(source) {
        Ok(listed) => listed,
        Err(failure) => return Route::Unreadable(failure),
    };
    if is_disabled(&listing) {
        return Route::Disabled;
    }
    if source.team_name.as_ref().is_some_and(TeamName::is_balls) {
        return Route::Balls;
    }
    Route::Validate {
        listing,
        revision,
        decompressed,
    }
}

/// A `NO_USE` or `NO_USE.txt` file directly in the source's own root. Compared folded, as
/// Windows does; a marker below a wrapper folder is the nested root's content, not the
/// source's, and is reported as an unexpected root file instead.
fn is_disabled(listing: &CanonicalListing) -> bool {
    let markers = [vtree::fold_name("NO_USE"), vtree::fold_name("NO_USE.txt")];
    listing.entries.iter().any(|entry| {
        matches!(entry.kind, ListedKind::File { .. })
            && !entry.path.contains('/')
            && markers.contains(&vtree::fold_name(&entry.path))
    })
}

#[cfg(test)]
mod tests {
    use aesthetics_export::ListedEntry;

    use super::*;
    use crate::testing::scratch;

    fn listing(files: &[&str]) -> CanonicalListing {
        CanonicalListing {
            display_name: "co Midcup".to_owned(),
            entries: files
                .iter()
                .map(|path| ListedEntry {
                    path: (*path).to_owned(),
                    kind: ListedKind::File { size: 0 },
                })
                .collect(),
        }
    }

    #[test]
    fn archive_extensions_are_case_insensitive() {
        assert_eq!(archive_kind(Path::new("a.zip")), Some(SourceKind::Zip));
        assert_eq!(archive_kind(Path::new("a.ZIP")), Some(SourceKind::Zip));
        assert_eq!(archive_kind(Path::new("a.7z")), Some(SourceKind::SevenZ));
        assert_eq!(archive_kind(Path::new("a.7Z")), Some(SourceKind::SevenZ));
        assert_eq!(archive_kind(Path::new("a.rar")), None);
        assert_eq!(archive_kind(Path::new("zip")), None);
        assert!(is_archive(Path::new("a.7z")));
        assert!(!is_archive(Path::new("a.rar")));
    }

    #[test]
    fn discovery_takes_folders_and_archives_in_folded_name_order() {
        let temp = scratch("reader_discovery");
        let root = temp.path();
        for folder in ["b Midcup Two", "A Midcup One"] {
            fs::create_dir(root.join(folder)).unwrap();
        }
        for file in ["c Midcup Three.ZIP", "D Midcup Four.7z", "e.rar", "f.txt"] {
            fs::write(root.join(file), "").unwrap();
        }

        let sources = discover(root, &[]).unwrap().sources;

        let summary: Vec<(u64, &str, &str, SourceKind, Option<&str>)> = sources
            .iter()
            .map(|source| {
                (
                    source.export_id.0,
                    source.file_name.as_str(),
                    source.display_name.as_str(),
                    source.kind,
                    source.team_name.as_ref().map(TeamName::as_str),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                (
                    0,
                    "A Midcup One",
                    "A Midcup One",
                    SourceKind::Folder,
                    Some("/a/")
                ),
                (
                    1,
                    "b Midcup Two",
                    "b Midcup Two",
                    SourceKind::Folder,
                    Some("/b/")
                ),
                (
                    2,
                    "c Midcup Three.ZIP",
                    "c Midcup Three",
                    SourceKind::Zip,
                    Some("/c/")
                ),
                (
                    3,
                    "D Midcup Four.7z",
                    "D Midcup Four",
                    SourceKind::SevenZ,
                    Some("/d/")
                ),
            ]
        );
        assert_eq!(sources[0].path, root.join("A Midcup One"));
    }

    #[test]
    fn named_exports_replace_the_scan() {
        let temp = scratch("reader_named");
        let root = temp.path();
        fs::create_dir(root.join("co Midcup In root")).unwrap();
        fs::create_dir(root.join("zz Midcup Named")).unwrap();
        fs::write(root.join("aa Midcup Named.zip"), "").unwrap();

        let named = [
            root.join("zz Midcup Named"),
            root.join("aa Midcup Named.zip"),
        ];
        let sources = discover(Path::new("no such root"), &named).unwrap().sources;

        let names: Vec<&str> = sources
            .iter()
            .map(|source| source.file_name.as_str())
            .collect();
        assert_eq!(names, ["aa Midcup Named.zip", "zz Midcup Named"]);
        assert_eq!(sources[1].export_id, ExportId(1));
    }

    #[test]
    fn names_differing_only_in_case_take_the_same_order_whatever_order_they_come_in() {
        let temp = scratch("reader_case_order");
        let root = temp.path();
        for folder in ["one/co Midcup a", "two/co Midcup A", "three/CO Midcup A"] {
            fs::create_dir_all(root.join(folder)).unwrap();
        }
        let given = [
            root.join("one/co Midcup a"),
            root.join("three/CO Midcup A"),
            root.join("two/co Midcup A"),
        ];
        let mut reversed = given.clone();
        reversed.reverse();

        for paths in [given, reversed] {
            let sources = discover(Path::new("no such root"), &paths).unwrap().sources;
            let names: Vec<&str> = sources
                .iter()
                .map(|source| source.file_name.as_str())
                .collect();
            assert_eq!(names, ["CO Midcup A", "co Midcup A", "co Midcup a"]);
        }
    }

    #[test]
    fn a_source_path_ending_in_dot_dot_takes_the_canonical_name() {
        let temp = scratch("reader_dot_dot");
        let root = temp.path();
        fs::create_dir_all(root.join("egg Midcup Tracer/Players")).unwrap();

        let source = source(
            root.join("egg Midcup Tracer/Players/.."),
            SourceKind::Folder,
            ExportId(0),
        );

        assert_eq!(source.file_name, "egg Midcup Tracer");
        assert_eq!(source.display_name, "egg Midcup Tracer");
        assert_eq!(
            source.team_name.map(|name| name.as_str().to_owned()),
            Some("/egg/".to_owned())
        );
    }

    #[test]
    fn a_named_path_that_is_no_longer_an_export_is_an_error_naming_it() {
        let temp = scratch("reader_named_gone");
        let root = temp.path();
        let error = discover(Path::new("no such root"), &[root.join("gone.txt")]).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!(
                "--export {} is not a folder, a .zip or a .7z file",
                root.join("gone.txt").display()
            )
        );
    }

    #[test]
    fn an_exports_root_that_cannot_be_read_is_an_error_naming_it() {
        let temp = scratch("reader_missing");
        let root = temp.path().join("exports");
        let error = discover(&root, &[]).unwrap_err();
        assert!(
            format!("{error:#}").contains(&root.display().to_string()),
            "{error:#}"
        );
    }

    #[test]
    fn the_no_use_marker_counts_only_at_the_root_and_in_any_case() {
        for marker in ["NO_USE", "no_use", "NO_USE.txt", "No_Use.TXT"] {
            assert!(
                is_disabled(&listing(&[marker, "Players/03/face_high.fmdl"])),
                "{marker}"
            );
        }
        for not_marker in ["wrapper/NO_USE", "NO_USE.bak", "NOUSE"] {
            assert!(!is_disabled(&listing(&[not_marker])), "{not_marker}");
        }
        let mut folder = listing(&[]);
        folder.entries.push(ListedEntry {
            path: "NO_USE".to_owned(),
            kind: ListedKind::Folder,
        });
        assert!(!is_disabled(&folder), "a folder is not a marker");
    }

    #[test]
    fn routing_sets_aside_disabled_balls_and_conflicting_refs_exports() {
        let temp = scratch("reader_routing");
        let root = temp.path();
        let write = |relative: &str| {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        };
        write("balls Off/NO_USE");
        write("balls On/ball.dds");
        write("co Midcup Team/notes.txt");
        write("refs a/players.txt");
        write("refs b/players.txt");
        write("refs c/NO_USE.txt");
        write("refs d.zip");

        let sources = discover(root, &[]).unwrap().sources;
        let routes: Vec<String> = route(&sources)
            .into_iter()
            .map(|route| match route {
                Route::Unreadable(failure) => format!("unreadable: {}", failure.error),
                Route::Disabled => "disabled".to_owned(),
                Route::Balls => "balls".to_owned(),
                Route::ConflictingRefs => "conflicting refs".to_owned(),
                Route::Validate { listing, .. } => format!("validate {}", listing.display_name),
            })
            .collect();

        assert_eq!(
            routes,
            [
                // Disabled comes before the balls rule.
                "disabled",
                "balls",
                "validate co Midcup Team",
                "conflicting refs",
                "conflicting refs",
                // A disabled or unreadable refs export does not conflict.
                "disabled",
                "unreadable: zip: invalid Zip archive: Could not find EOCD",
            ]
        );
    }

    #[test]
    fn a_single_refs_export_is_validated() {
        let temp = scratch("reader_single_refs");
        let root = temp.path();
        fs::create_dir_all(root.join("refs a")).unwrap();
        fs::write(root.join("refs a/players.txt"), "").unwrap();
        fs::create_dir_all(root.join("refs b")).unwrap();
        fs::write(root.join("refs b/NO_USE"), "").unwrap();
        let sources = discover(root, &[]).unwrap().sources;
        let routes = route(&sources);
        assert!(matches!(routes[0], Route::Validate { .. }), "{routes:?}");
        assert_eq!(routes[1], Route::Disabled);
    }

    #[test]
    fn a_7z_is_routed_with_its_decompressed_size_and_a_zip_with_none() {
        let temp = scratch("reader_decompressed_size");
        let root = temp.path();
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sources");
        for name in ["egg Midcup Tracer.7z", "egg Midcup Tracer.zip"] {
            fs::copy(fixtures.join(name), root.join(name)).unwrap();
        }
        let sources = discover(root, &[]).unwrap().sources;

        let sizes: Vec<(&str, usize)> = sources
            .iter()
            .zip(route(&sources))
            .map(|(source, route)| {
                let Route::Validate { decompressed, .. } = route else {
                    panic!("{route:?}");
                };
                (source.file_name.as_str(), decompressed)
            })
            .collect();

        // The sum of the `.7z`'s entries, what its first read acquires.
        assert_eq!(
            sizes,
            [
                ("egg Midcup Tracer.7z", 153_582),
                ("egg Midcup Tracer.zip", 0)
            ]
        );
    }
}
