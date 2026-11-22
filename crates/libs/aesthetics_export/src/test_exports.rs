//! Clean aesthetics exports for tests. A test gives an export only the files it is about, and
//! `missing` adds what the export still lacks to pass the structure checks, one rule per
//! `Requirement`. A test of one of those checks turns its requirement off with
//! `TestExport::without`. The module owns no I/O: it yields (path, bytes) pairs, which a listing
//! reads by size and a test's sandbox writes.

use std::collections::{BTreeMap, BTreeSet};

use pes_version::{Engine, PesVersion};

use crate::conventions::{FileKind, classify, folder_slot, is_small_metadata};
use crate::listing::{CanonicalListing, ListedEntry, ListedKind, SmallMetadata};
use crate::parse::{ExportCoverage, coverage, team_name};
use crate::validate::{is_logo_name, is_portrait_name, stem};

/// What a clean export holds beyond the files a test gives it, one variant per check that
/// requires something. A test of that check turns its requirement off with `without`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// Some content (`export_empty`): an export whose files and empty folders all sit at its
    /// root gets the default player, `Players/03 - A/` (`Players/Ref A/` and a `players.txt`
    /// of `01 Ref A` for a referee export).
    Content,
    /// A `Full` team export's root logo (`logo_missing`): `logo.png` when no root file's stem
    /// starts with `logo`.
    Logo,
    /// On PES 2017 to 2021, a `portrait.dds` in each player folder of a `Full` team export
    /// that has no portrait of its own and no `Portraits/NN` file for its folder's number
    /// (`portrait_missing`).
    Portraits,
}

/// The bytes of the files the requirements add.
#[derive(Debug, Clone)]
pub struct CleanFiles {
    /// The default player's files on a Fox target, by name: `face_high.fmdl`.
    pub fox_player: Vec<(String, Vec<u8>)>,
    /// The same on a pre-Fox target: `face_high.model`, `face_high.mtl`, `skin.dds`.
    pub pre_fox_player: Vec<(String, Vec<u8>)>,
    /// A player's `portrait.dds`.
    pub portrait: Vec<u8>,
    /// A `Full` export's `logo.png`.
    pub logo: Vec<u8>,
}

impl CleanFiles {
    /// The real names holding a few bytes each: enough for a listing, which reads sizes.
    pub fn placeholders() -> CleanFiles {
        let named = |names: &[&str]| {
            names
                .iter()
                .map(|name| ((*name).to_owned(), b"bytes".to_vec()))
                .collect()
        };
        CleanFiles {
            fox_player: named(&["face_high.fmdl"]),
            pre_fox_player: named(&["face_high.model", "face_high.mtl", "skin.dds"]),
            portrait: b"portrait".to_vec(),
            logo: b"logo".to_vec(),
        }
    }

    /// The default player's files on `engine`.
    pub fn player(&self, engine: Engine) -> &[(String, Vec<u8>)] {
        match engine {
            Engine::Fox => &self.fox_player,
            Engine::PreFox => &self.pre_fox_player,
        }
    }
}

/// The files the export `name` lacks to be clean on `version`, given `paths`, every file and
/// empty folder it holds (relative to its root, `/`-separated), with the requirements in
/// `without` left out: (path, bytes), sorted by path. The export's kind comes from `name` as
/// `parse_listing` reads it (`team_name`, then `coverage`). The one place a requirement is
/// written.
pub fn missing(
    name: &str,
    version: PesVersion,
    paths: &[&str],
    without: &[Requirement],
    clean: &CleanFiles,
) -> Vec<(String, Vec<u8>)> {
    let referee = team_name(name).is_some_and(|team| team.is_referees());
    let full_team = !referee && coverage(name) == Some(ExportCoverage::Full);
    let mut added: Vec<(String, Vec<u8>)> = Vec::new();
    if !without.contains(&Requirement::Content) && paths.iter().all(|path| at_root(path)) {
        let folder = if referee {
            added.push(("players.txt".to_owned(), b"01 Ref A\n".to_vec()));
            "Players/Ref A"
        } else {
            "Players/03 - A"
        };
        for (file, bytes) in clean.player(version.engine()) {
            added.push((format!("{folder}/{file}"), bytes.clone()));
        }
    }
    let has_logo = paths.iter().any(|path| at_root(path) && is_logo_name(path));
    if full_team && !without.contains(&Requirement::Logo) && !has_logo {
        added.push(("logo.png".to_owned(), clean.logo.clone()));
    }
    if full_team && version >= PesVersion::Pes17 && !without.contains(&Requirement::Portraits) {
        let all: Vec<&str> = paths
            .iter()
            .copied()
            .chain(added.iter().map(|(path, _)| path.as_str()))
            .collect();
        for folder in without_portrait(&all) {
            added.push((format!("{folder}/portrait.dds"), clean.portrait.clone()));
        }
    }
    // A default file never replaces one the test gave (a referee's own `players.txt`).
    added.retain(|(path, _)| !paths.iter().any(|given| given.eq_ignore_ascii_case(path)));
    added.sort();
    added
}

/// Whether `path` sits at the export's root: a root file, or a root folder with nothing in it.
fn at_root(path: &str) -> bool {
    !path.contains('/')
}

/// The player folders (`Players/<name>`) among `paths` that have no `portrait.*` texture
/// directly in them and no `Portraits/NN` file for the number their name starts with.
fn without_portrait(paths: &[&str]) -> BTreeSet<String> {
    let mut folders = BTreeSet::new();
    let mut with_portrait = BTreeSet::new();
    let mut portrait_slots = BTreeSet::new();
    for path in paths {
        let segments: Vec<&str> = path.split('/').collect();
        match segments.as_slice() {
            [root, file] if root.eq_ignore_ascii_case("Portraits") => {
                portrait_slots.extend(folder_slot(stem(file)));
            }
            [root, folder, rest @ ..] if root.eq_ignore_ascii_case("Players") => {
                let folder = format!("{root}/{folder}");
                if let [file] = rest
                    && classify(file) == FileKind::Texture
                    && is_portrait_name(file)
                {
                    with_portrait.insert(folder.clone());
                }
                folders.insert(folder);
            }
            _ => {}
        }
    }
    folders
        .into_iter()
        .filter(|folder| !with_portrait.contains(folder))
        .filter(|folder| {
            let name = folder.split_once('/').map_or("", |(_, name)| name);
            folder_slot(name).is_none_or(|slot| !portrait_slots.contains(&slot))
        })
        .collect()
}

/// An export built in memory: the test's own files, completed by `missing` when listed or
/// written.
#[derive(Debug, Clone)]
pub struct TestExport {
    name: String,
    version: PesVersion,
    files: BTreeMap<String, FileContent>,
    folders: BTreeSet<String>,
    without: Vec<Requirement>,
}

/// A test's file: its bytes, or only its size when no step reads it.
#[derive(Debug, Clone)]
enum FileContent {
    Bytes(Vec<u8>),
    Size(u64),
}

impl TestExport {
    /// The export `name` (`co Full Spring`, `refs Cup`) for `version`, holding nothing yet.
    pub fn new(name: &str, version: PesVersion) -> TestExport {
        TestExport {
            name: name.to_owned(),
            version,
            files: BTreeMap::new(),
            folders: BTreeSet::new(),
            without: Vec::new(),
        }
    }

    /// Adds the file, or replaces the one at `path`.
    pub fn file(mut self, path: &str, bytes: impl Into<Vec<u8>>) -> TestExport {
        self.files
            .insert(path.to_owned(), FileContent::Bytes(bytes.into()));
        self
    }

    /// A file whose bytes no step reads: listed at `size`, written as zeros, and left out of the
    /// small metadata even when its name is `players.txt`, `refs.txt` or `notes.txt`.
    pub fn sized(mut self, path: &str, size: u64) -> TestExport {
        self.files.insert(path.to_owned(), FileContent::Size(size));
        self
    }

    /// An empty folder (a placeholder kit, a referee with no file yet).
    pub fn folder(mut self, path: &str) -> TestExport {
        self.folders.insert(path.to_owned());
        self
    }

    /// Turns `requirement` off: `missing` adds nothing for it.
    pub fn without(mut self, requirement: Requirement) -> TestExport {
        self.without.push(requirement);
        self
    }

    /// The export's name, as its folder or archive stem.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The PES version `missing` completes the export for.
    pub fn version(&self) -> PesVersion {
        self.version
    }

    /// Every file, the test's and then `missing`'s, sorted by path: for a test that writes the
    /// export as a folder or packs it as an archive.
    pub fn files(&self, clean: &CleanFiles) -> Vec<(String, Vec<u8>)> {
        let mut files: Vec<(String, Vec<u8>)> = self
            .files
            .iter()
            .map(|(path, content)| {
                let bytes = match content {
                    FileContent::Bytes(bytes) => bytes.clone(),
                    FileContent::Size(size) => vec![0; *size as usize],
                };
                (path.clone(), bytes)
            })
            .chain(self.missing(clean))
            .collect();
        files.sort();
        files
    }

    /// The empty folders the test added.
    pub fn folders(&self) -> Vec<String> {
        self.folders.iter().cloned().collect()
    }

    /// The listing and small metadata a source read gives, the files `missing` adds holding
    /// `CleanFiles::placeholders`' bytes. The small metadata holds every `players.txt`,
    /// `refs.txt` and `notes.txt` that has bytes: the test's own and a referee's default roster.
    pub fn listing(&self) -> (CanonicalListing, SmallMetadata) {
        let mut entries = Vec::new();
        let mut metadata = BTreeMap::new();
        for (path, content) in &self.files {
            let size = match content {
                FileContent::Bytes(bytes) => {
                    if is_small_metadata(path) {
                        metadata.insert(path.clone(), Ok(bytes.clone()));
                    }
                    bytes.len() as u64
                }
                FileContent::Size(size) => *size,
            };
            entries.push(file_entry(path, size));
        }
        for (path, bytes) in self.missing(&CleanFiles::placeholders()) {
            entries.push(file_entry(&path, bytes.len() as u64));
            if is_small_metadata(&path) {
                metadata.insert(path, Ok(bytes));
            }
        }
        entries.extend(self.folders.iter().map(|path| ListedEntry {
            path: path.clone(),
            kind: ListedKind::Folder,
        }));
        let listing = CanonicalListing {
            display_name: self.name.clone(),
            entries,
        };
        (listing, SmallMetadata { files: metadata })
    }

    /// `missing` over the test's files and folders.
    fn missing(&self, clean: &CleanFiles) -> Vec<(String, Vec<u8>)> {
        let paths: Vec<&str> = self
            .files
            .keys()
            .chain(&self.folders)
            .map(String::as_str)
            .collect();
        missing(&self.name, self.version, &paths, &self.without, clean)
    }
}

/// A listed file at `path`, `size` bytes long.
fn file_entry(path: &str, size: u64) -> ListedEntry {
    ListedEntry {
        path: path.to_owned(),
        kind: ListedKind::File { size },
    }
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion::{Pes15, Pes16, Pes17, Pes21};

    use super::*;
    use crate::listing::ValidationContext;
    use crate::testing::context;

    /// The codes of the issues the structure pass reports on `export`, for its own version.
    fn issue_codes(export: &TestExport) -> Vec<&'static str> {
        let (listing, metadata) = export.listing();
        let context = ValidationContext {
            version: export.version(),
            ..context()
        };
        let report = crate::parse_listing(listing, metadata)
            .unwrap()
            .validate(&context);
        report.issues.iter().map(|issue| issue.code).collect()
    }

    /// The paths `missing` adds to `export`.
    fn added(export: &TestExport) -> Vec<String> {
        export
            .missing(&CleanFiles::placeholders())
            .into_iter()
            .map(|(path, _)| path)
            .collect()
    }

    #[test]
    fn an_export_given_nothing_is_clean() {
        for name in ["co Midcup Spring", "co Full Spring", "refs Cup"] {
            for version in [Pes15, Pes16, Pes17, Pes21] {
                let export = TestExport::new(name, version);
                assert_eq!(
                    issue_codes(&export),
                    Vec::<&str>::new(),
                    "{name} on {version:?}"
                );
            }
        }
    }

    #[test]
    fn an_export_given_nothing_gets_the_smallest_clean_set() {
        let model = "Players/03 - A/face_high.model";
        let mtl = "Players/03 - A/face_high.mtl";
        let skin = "Players/03 - A/skin.dds";
        let fmdl = "Players/03 - A/face_high.fmdl";
        let portrait = "Players/03 - A/portrait.dds";
        let cases: [(&str, PesVersion, &[&str]); 8] = [
            ("co Midcup Spring", Pes15, &[model, mtl, skin]),
            ("co Midcup Spring", Pes21, &[fmdl]),
            ("co Full Spring", Pes15, &[model, mtl, skin, "logo.png"]),
            ("co Full Spring", Pes16, &[model, mtl, skin, "logo.png"]),
            (
                "co Full Spring",
                Pes17,
                &[model, mtl, portrait, skin, "logo.png"],
            ),
            ("co Full Spring", Pes21, &[fmdl, portrait, "logo.png"]),
            (
                "refs Cup",
                Pes16,
                &[
                    "Players/Ref A/face_high.model",
                    "Players/Ref A/face_high.mtl",
                    "Players/Ref A/skin.dds",
                    "players.txt",
                ],
            ),
            (
                "refs Cup",
                Pes21,
                &["Players/Ref A/face_high.fmdl", "players.txt"],
            ),
        ];
        for (name, version, files) in cases {
            assert_eq!(
                added(&TestExport::new(name, version)),
                files,
                "{name} on {version:?}"
            );
        }
    }

    #[test]
    fn each_requirement_turned_off_gives_its_check_s_finding() {
        let cases = [
            ("co Midcup Spring", Requirement::Content, "export_empty"),
            ("co Full Spring", Requirement::Logo, "logo_missing"),
            ("co Full Spring", Requirement::Portraits, "portrait_missing"),
        ];
        for (name, requirement, code) in cases {
            let export = TestExport::new(name, Pes21).without(requirement);
            assert_eq!(
                issue_codes(&export),
                [code],
                "{name} without {requirement:?}"
            );
        }
    }

    #[test]
    fn a_full_export_gets_a_portrait_for_each_player_without_one() {
        let export = TestExport::new("co Full Spring", Pes21)
            .sized("Players/05 - A/face_high.fmdl", 4)
            .sized("Players/07 - B/face_high.fmdl", 4);
        assert_eq!(
            added(&export),
            [
                "Players/05 - A/portrait.dds",
                "Players/07 - B/portrait.dds",
                "logo.png"
            ]
        );
        assert_eq!(issue_codes(&export), Vec::<&str>::new());

        let export = export.sized("Portraits/05.dds", 4);
        assert_eq!(added(&export), ["Players/07 - B/portrait.dds", "logo.png"]);
        assert_eq!(issue_codes(&export), Vec::<&str>::new());
    }
}
