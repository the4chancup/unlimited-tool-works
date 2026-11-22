//! Clean aesthetics exports for tests. A test gives an export only the files it is about, and
//! `missing` adds what the export still lacks to pass the structure checks, one rule per
//! `Requirement`. A test of one of those checks turns its requirement off with
//! `TestExport::without`. The module owns no I/O: it yields (path, bytes) pairs, which a listing
//! reads by size and a test's sandbox writes.

use std::collections::{BTreeMap, BTreeSet};

use pes_version::{Engine, PesVersion};

use crate::conventions::{
    FileKind, MetadataFile, classify, folder_slot, is_logo_texture, is_small_metadata,
};
use crate::listing::{CanonicalListing, ListedEntry, ListedKind, SmallMetadata};
use crate::parse::{ExportCoverage, coverage, team_name};
use crate::validate::{is_logo_name, is_portrait_name, stem};

/// What a clean export holds beyond the files a test gives it, one variant per check that
/// requires something. A test of that check turns its requirement off with `without`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// Some content (`export_empty`): an export whose files and empty folders all sit at its
    /// root gets the default player, `Players/03 - A/` (`Players/Ref A/` and a `players.txt`
    /// of `01 Ref A` for a referee export) — but nothing beside a root roster (`players.txt`,
    /// or a referee export's `refs.txt`), whose lines decide the folders instead, nor a root
    /// `logo*` texture, which alone satisfies the check. That texture is `Logo`'s fill, so
    /// `without(Content)` alone stays clean on a `Full` export; a test of `export_empty`
    /// there turns `Logo` off too.
    Content,
    /// A `Full` team export's root logo (`logo_missing`): `logo.png` when no root file's stem
    /// starts with `logo`.
    Logo,
    /// On PES 2017 to 2021, a `portrait.dds` in each player folder of a `Full` team export
    /// that has no portrait of its own and no `Portraits/NN` texture for its folder's number
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

/// The files the export `name` lacks to be clean on `version`, given `files` and `folders`
/// (the export's empty folders), both relative to its root and `/`-separated, with the
/// requirements in `without` left out: (path, bytes), sorted by path. The export's kind comes
/// from `name` as `parse_listing` reads it (`team_name`, then `coverage`). The one place a
/// requirement is written.
pub fn missing(
    name: &str,
    version: PesVersion,
    files: &[&str],
    folders: &[&str],
    without: &[Requirement],
    clean: &CleanFiles,
) -> Vec<(String, Vec<u8>)> {
    let referee = team_name(name).is_some_and(|team| team.is_referees());
    let full_team = !referee && coverage(name) == Some(ExportCoverage::Full);
    let mut added: Vec<(String, Vec<u8>)> = Vec::new();
    // Content adds nothing beside a root roster, whose lines decide the player folders,
    // or a root `logo*` texture, which alone satisfies `export_empty`.
    let at_root_file =
        |predicate: fn(&str) -> bool| files.iter().any(|path| at_root(path) && predicate(path));
    if !without.contains(&Requirement::Content)
        && files.iter().chain(folders.iter()).all(|path| at_root(path))
        && !files
            .iter()
            .any(|path| at_root(path) && is_roster_name(path, referee))
        && !at_root_file(is_logo_texture)
    {
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
    let has_logo = at_root_file(is_logo_name);
    if full_team && !without.contains(&Requirement::Logo) && !has_logo {
        added.push(("logo.png".to_owned(), clean.logo.clone()));
    }
    if full_team && version >= PesVersion::Pes17 && !without.contains(&Requirement::Portraits) {
        let all: Vec<&str> = files
            .iter()
            .copied()
            .chain(added.iter().map(|(path, _)| path.as_str()))
            .collect();
        for folder in without_portrait(&all, folders) {
            added.push((format!("{folder}/portrait.dds"), clean.portrait.clone()));
        }
    }
    // A default file never replaces one the test gave (a referee's own `players.txt`).
    added.retain(|(path, _)| {
        !files
            .iter()
            .chain(folders.iter())
            .any(|given| given.eq_ignore_ascii_case(path))
    });
    added.sort();
    added
}

/// Whether `path` sits at the export's root: a root file, or a root folder with nothing in it.
fn at_root(path: &str) -> bool {
    !path.contains('/')
}

/// Whether `name`, a root file's, is a roster file name: `players.txt`, or `refs.txt` on a
/// `referee` export (a team's `refs.txt` is no roster), matched whole,
/// ASCII-case-insensitively.
fn is_roster_name(name: &str, referee: bool) -> bool {
    match classify(name) {
        FileKind::Metadata(MetadataFile::PlayersTxt) => true,
        FileKind::Metadata(MetadataFile::RefsTxt) => referee,
        _ => false,
    }
}

/// The player folders with no `portrait.*` texture directly in them and no `Portraits/NN`
/// texture for the number their name starts with. `Players/<x>` is a player folder when it
/// is an empty folder in `folders` or some file lies below it (`Players/<x>/...`); a file
/// directly in `Players/` is not.
fn without_portrait(files: &[&str], folders: &[&str]) -> BTreeSet<String> {
    let mut players = BTreeSet::new();
    let mut with_portrait = BTreeSet::new();
    let mut portrait_slots = BTreeSet::new();
    for path in files {
        let segments: Vec<&str> = path.split('/').collect();
        match segments.as_slice() {
            // `check_portraits` counts only a texture at `Portraits/NN`.
            [root, file] if root.eq_ignore_ascii_case("Portraits") => {
                if classify(file) == FileKind::Texture {
                    portrait_slots.extend(folder_slot(stem(file)));
                }
            }
            [root, name, _, ..] if root.eq_ignore_ascii_case("Players") => {
                let folder = format!("{root}/{name}");
                if let [file] = &segments[2..]
                    && classify(file) == FileKind::Texture
                    && is_portrait_name(file)
                {
                    with_portrait.insert(folder.clone());
                }
                players.insert(folder);
            }
            _ => {}
        }
    }
    for path in folders {
        if let [root, name, ..] = path.split('/').collect::<Vec<_>>().as_slice()
            && root.eq_ignore_ascii_case("Players")
        {
            players.insert(format!("{root}/{name}"));
        }
    }
    players
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
        let files: Vec<&str> = self.files.keys().map(String::as_str).collect();
        let folders: Vec<&str> = self.folders.iter().map(String::as_str).collect();
        missing(
            &self.name,
            self.version,
            &files,
            &folders,
            &self.without,
            clean,
        )
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

    #[test]
    fn a_file_directly_in_players_is_no_player_folder() {
        let export = TestExport::new("co Full Spring", Pes21)
            .sized("Players/notes.txt", 5)
            .sized("Players/05 - A/face_high.fmdl", 4);
        // `Players/notes.txt` is a stray file, not a folder: no `portrait.dds` beneath it.
        assert_eq!(added(&export), ["Players/05 - A/portrait.dds", "logo.png"]);
        // The stray itself is the one finding the pass reports.
        assert_eq!(issue_codes(&export), ["root_file_unexpected"]);
    }

    #[test]
    fn an_empty_player_folder_is_a_player_folder() {
        let export = TestExport::new("co Full Spring", Pes21).folder("Players/05 - A");
        assert_eq!(added(&export), ["Players/05 - A/portrait.dds", "logo.png"]);
        assert_eq!(issue_codes(&export), Vec::<&str>::new());
    }

    #[test]
    fn a_team_s_refs_txt_is_no_roster() {
        let export = TestExport::new("co Midcup Spring", Pes21).file("refs.txt", "01 arbiter\n");
        assert_eq!(added(&export), ["Players/03 - A/face_high.fmdl"]);
        // A team ignores a `refs.txt`: it is a stray, and the default player keeps the
        // export from being empty.
        assert_eq!(issue_codes(&export), ["root_file_unexpected"]);
    }

    #[test]
    fn a_given_roster_decides_the_player_folders() {
        let midcup = TestExport::new("co Midcup Spring", Pes21)
            .file("players.txt", "03 B\n")
            .sized("Players/B/face_high.fmdl", 4);
        assert_eq!(added(&midcup), Vec::<String>::new());
        assert_eq!(issue_codes(&midcup), Vec::<&str>::new());

        let refs = TestExport::new("refs Cup", Pes21)
            .file("refs.txt", "01 arbiter\n")
            .sized("Players/arbiter/face_high.fmdl", 4);
        assert_eq!(added(&refs), Vec::<String>::new());
        assert_eq!(issue_codes(&refs), Vec::<&str>::new());

        // A roster alone still blocks the default player: the folders its
        // lines do not name are unlisted, so the export stays empty instead.
        let midcup = TestExport::new("co Midcup Spring", Pes21).file("players.txt", "03 B\n");
        assert_eq!(added(&midcup), Vec::<String>::new());
        assert_eq!(
            issue_codes(&midcup),
            ["export_empty", "players_txt_target_missing"]
        );

        // A referee export's empty surviving roster also fails it.
        let refs = TestExport::new("refs Cup", Pes21).file("refs.txt", "03 B\n");
        assert_eq!(added(&refs), Vec::<String>::new());
        assert_eq!(
            issue_codes(&refs),
            [
                "export_empty",
                "players_txt_target_missing",
                "players_txt_invalid"
            ]
        );
    }

    #[test]
    fn a_root_logo_texture_alone_satisfies_content() {
        let export = TestExport::new("co Midcup Spring", Pes21).sized("logo.png", 4);
        assert_eq!(added(&export), Vec::<String>::new());
        assert_eq!(issue_codes(&export), Vec::<&str>::new());
    }

    #[test]
    fn only_a_portraits_texture_covers_a_players_slot() {
        let export = TestExport::new("co Full Spring", Pes21)
            .sized("Players/05 - A/face_high.fmdl", 4)
            .sized("Portraits/05.txt", 3);
        assert_eq!(added(&export), ["Players/05 - A/portrait.dds", "logo.png"]);
        // `Portraits/05.txt` counts for no slot; the pass reports it alone.
        assert_eq!(issue_codes(&export), ["portrait_name_invalid"]);
    }

    #[test]
    fn a_player_folder_s_own_portrait_texture_covers_it() {
        // `portrait.png`, not the `portrait.dds` the builder would add: a given file at that
        // path is kept over the added one, which would hide whether the folder counted.
        let export = TestExport::new("co Full Spring", Pes21)
            .sized("Players/05 - A/face_high.fmdl", 4)
            .sized("Players/05 - A/portrait.png", 4)
            .sized("Players/07 - B/face_high.fmdl", 4)
            .sized("Players/07 - B/portrait.txt", 4);
        // B's `portrait.txt` is no texture, so his folder still gets one.
        assert_eq!(added(&export), ["Players/07 - B/portrait.dds", "logo.png"]);
    }

    #[test]
    fn a_texture_named_for_a_slot_outside_portraits_covers_no_player() {
        let export = TestExport::new("co Full Spring", Pes21)
            .sized("Players/05 - A/face_high.fmdl", 4)
            .sized("Common/05.dds", 4);
        assert_eq!(added(&export), ["Players/05 - A/portrait.dds", "logo.png"]);
    }

    #[test]
    fn the_accessors_give_the_export_as_the_test_built_it() {
        let export = TestExport::new("co Midcup Spring", Pes21)
            .file("notes.txt", "\n")
            .sized("Players/05 - A/face_high.fmdl", 2)
            .folder("Players/06 - B");
        assert_eq!(export.name(), "co Midcup Spring");
        assert_eq!(export.folders(), ["Players/06 - B"]);
        // A Midcup export with a player's model lacks nothing, so the test's files alone,
        // sorted by path.
        assert_eq!(
            export.files(&CleanFiles::placeholders()),
            [
                ("Players/05 - A/face_high.fmdl".to_owned(), vec![0, 0]),
                ("notes.txt".to_owned(), b"\n".to_vec()),
            ]
        );
    }

    #[test]
    fn a_full_export_without_content_stays_clean_thanks_to_its_logo() {
        let export = TestExport::new("co Full Spring", Pes21).without(Requirement::Content);
        assert_eq!(issue_codes(&export), Vec::<&str>::new());
        // With the logo fill off too, nothing satisfies `export_empty`.
        assert_eq!(
            issue_codes(&export.without(Requirement::Logo)),
            ["export_empty"]
        );
    }
}
