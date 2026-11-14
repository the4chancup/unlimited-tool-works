use std::collections::{BTreeMap, HashMap};
use std::io::Cursor;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError};
use semver::Version;

use super::*;
use crate::shell::test_harness::TestDir;

/// The new bundle's folder inside its archive.
pub(super) const TOP: &str = "unlimited-tool-works-0.0.2";

/// The empty folder the new bundle ships and the old folder already holds,
/// with a member's file inside.
pub(super) const KEPT_FOLDER: &str = "exports";

/// The empty folder the new bundle adds.
pub(super) const NEW_FOLDER: &str = "old_exports";

/// A file name and its text.
pub(crate) type Files = Vec<(String, &'static str)>;

/// The new bundle's files, relative to its folder: the program, the readme,
/// the default settings, a license, a launcher, and a second launcher the old
/// bundle lacks.
pub(crate) fn new_files() -> Files {
    vec![
        (exe_file_name(), "new program"),
        ("README.txt".to_owned(), "new readme"),
        ("data/settings.toml".to_owned(), "# the default settings"),
        ("licenses/LICENSE-MIT".to_owned(), "new license"),
        ("quick_compile.bat".to_owned(), "new launcher"),
        ("quick_upgrade.bat".to_owned(), "new second launcher"),
    ]
}

/// The program's folder before the update: the new bundle's first files with
/// the old version's bytes, the member's own settings, and three files of the
/// member's the new bundle lacks: one at the root, one in `data/` and one in
/// `exports/`, a folder the new bundle ships empty.
pub(crate) fn old_files() -> Files {
    vec![
        (exe_file_name(), "old program"),
        ("README.txt".to_owned(), "old readme"),
        ("data/settings.toml".to_owned(), "pes_version = 21"),
        ("licenses/LICENSE-MIT".to_owned(), "old license"),
        ("quick_compile.bat".to_owned(), "old launcher"),
        ("compile_log.txt".to_owned(), "a member's compile log"),
        ("data/teams_list.txt".to_owned(), "a member's teams list"),
        (
            "exports/co Full Spring/notes.txt".to_owned(),
            "a member's notes",
        ),
    ]
}

/// A fresh temp folder of its own holding `files`, removed when the guard is
/// dropped, so a test that fails still leaves no folder behind.
pub(crate) fn folder_with(name: &str, files: &Files) -> TestDir {
    let dir = TestDir::new(&format!("studio_core_updater_{name}"));
    write_files(dir.path(), files);
    dir
}

/// Writes `files` under `root`, creating the folders they need.
pub(crate) fn write_files(root: &Path, files: &Files) {
    for (relative, text) in files {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
}

/// Every folder (`None`) and file (its text) under `root`, by its path
/// relative to `root` with `/` separators.
pub(crate) fn snapshot(root: &Path) -> BTreeMap<String, Option<String>> {
    fn walk(root: &Path, folder: &Path, into: &mut BTreeMap<String, Option<String>>) {
        for entry in fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                into.insert(relative, None);
                walk(root, &path, into);
            } else {
                into.insert(relative, Some(fs::read_to_string(&path).unwrap()));
            }
        }
    }
    let mut into = BTreeMap::new();
    walk(root, root, &mut into);
    into
}

/// `files` (by their paths in the archive) and the empty `folders`, packed
/// as an archive of `kind`. In a `.tar.gz` the program is executable
/// (`0o755`) and every other file `0o644`, as the release packs them.
pub(super) fn pack_entries(kind: ArchiveKind, files: &Files, folders: &[String]) -> Vec<u8> {
    match kind {
        ArchiveKind::SevenZ => {
            let mut writer = sevenz_rust2::ArchiveWriter::new(Cursor::new(Vec::new())).unwrap();
            for (path, text) in files {
                let entry = sevenz_rust2::ArchiveEntry::new_file(path);
                writer
                    .push_archive_entry(entry, Some(text.as_bytes()))
                    .unwrap();
            }
            for folder in folders {
                let entry = sevenz_rust2::ArchiveEntry::new_directory(folder);
                writer.push_archive_entry::<&[u8]>(entry, None).unwrap();
            }
            writer.finish().unwrap().into_inner()
        }
        ArchiveKind::TarGz => {
            let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            let mut builder = tar::Builder::new(gzip);
            for (path, text) in files {
                let is_program =
                    Path::new(path).file_name() == Some(std::ffi::OsStr::new(&exe_file_name()));
                let mut header = tar::Header::new_gnu();
                header.set_size(text.len() as u64);
                header.set_mode(if is_program { 0o755 } else { 0o644 });
                builder
                    .append_data(&mut header, path, text.as_bytes())
                    .unwrap();
            }
            for folder in folders {
                let mut header = tar::Header::new_gnu();
                header.set_entry_type(tar::EntryType::Directory);
                header.set_size(0);
                header.set_mode(0o755);
                builder
                    .append_data(&mut header, format!("{folder}/"), io::empty())
                    .unwrap();
            }
            builder.into_inner().unwrap().finish().unwrap()
        }
    }
}

/// `files` and the empty `KEPT_FOLDER` and `NEW_FOLDER`, packed under `TOP`
/// as an archive of `kind`: the shape a release's archive has.
pub(super) fn pack(kind: ArchiveKind, files: &Files) -> Vec<u8> {
    let under_top: Files = files
        .iter()
        .map(|(relative, text)| (format!("{TOP}/{relative}"), *text))
        .collect();
    let folders = [KEPT_FOLDER, NEW_FOLDER].map(|folder| format!("{TOP}/{folder}"));
    pack_entries(kind, &under_top, &folders)
}

/// The archive asset's name for `kind`.
pub(super) fn archive_name(kind: ArchiveKind) -> String {
    format!("unlimited-tool-works-0.0.2{}", kind.suffix())
}

/// `SHA256SUMS.txt` as the release workflow writes it: the Windows line,
/// then the Linux one; `kind`'s line carries `hash`, the other a hash of
/// nothing in the test.
pub(super) fn checksums(kind: ArchiveKind, hash: &str) -> Vec<u8> {
    let other = [0xab_u8; 32];
    [ArchiveKind::SevenZ, ArchiveKind::TarGz]
        .into_iter()
        .map(|line_kind| {
            let line_hash = if line_kind == kind {
                hash.to_owned()
            } else {
                to_hex(&other)
            };
            format!("{line_hash}  {}\n", archive_name(line_kind))
        })
        .collect::<String>()
        .into_bytes()
}

/// The URL release v0.0.2 serves the asset `name` from, as the releases
/// list names it.
fn asset_url(name: &str) -> String {
    format!("https://github.com/the4chancup/unlimited-tool-works/releases/download/v0.0.2/{name}")
}

/// A download's body: its first half, then, once `gate` lets go (a send,
/// or its sender dropped), the rest. A gate held ten seconds fails the
/// read, so a test that never lets go fails instead of hanging.
struct HeldBody {
    /// The first half.
    first: Cursor<Vec<u8>>,
    /// What the second half waits for; `None` once passed, or for a body
    /// that never waits.
    gate: Option<Receiver<()>>,
    /// The second half.
    rest: Cursor<Vec<u8>>,
}

impl Read for HeldBody {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.first.read(buf)?;
        if read > 0 {
            return Ok(read);
        }
        if let Some(gate) = self.gate.take()
            && gate.recv_timeout(Duration::from_secs(120)) == Err(RecvTimeoutError::Timeout)
        {
            return Err(io::Error::other("the test never let the download go on"));
        }
        self.rest.read(buf)
    }
}

/// The network seam serving each of `assets` (name, bytes) at its release
/// URL, every body in two reads so a download reports progress between
/// them. With a `gate`, an archive's second half waits for it
/// (`SHA256SUMS.txt` never does).
pub(crate) fn serve(assets: Vec<(String, Vec<u8>)>, gate: Option<Receiver<()>>) -> Box<OpenUrl> {
    let served: HashMap<String, (bool, Vec<u8>)> = assets
        .into_iter()
        .map(|(name, bytes)| (asset_url(&name), (name != CHECKSUMS_ASSET, bytes)))
        .collect();
    Box::new(
        move |requested: &str| -> Result<Box<dyn Read + Send>, ureq::Error> {
            let (is_archive, bytes) = served.get(requested).ok_or(ureq::Error::StatusCode(404))?;
            let (first, rest) = bytes.split_at(bytes.len() / 2);
            Ok(Box::new(HeldBody {
                first: Cursor::new(first.to_vec()),
                gate: gate.clone().filter(|_| *is_archive),
                rest: Cursor::new(rest.to_vec()),
            }))
        },
    )
}

/// Release v0.0.2 listing `assets` (name, bytes), and the network seam
/// serving each by its URL (`serve`, no gate).
pub(super) fn release(assets: Vec<(String, Vec<u8>)>) -> (Release, Box<OpenUrl>) {
    let release = Release {
        version: Version::new(0, 0, 2),
        notes: String::new(),
        page_url: "https://github.com/the4chancup/unlimited-tool-works/releases/tag/v0.0.2"
            .to_owned(),
        assets: assets
            .iter()
            .map(|(name, bytes)| ReleaseAsset {
                name: name.clone(),
                download_url: asset_url(name),
                size: bytes.len() as u64,
            })
            .collect(),
    };
    (release, serve(assets, None))
}

/// The assets (name, bytes) of a release whose `kind` archive is
/// `archive`: the archive first, then a `SHA256SUMS.txt` listing `hash`
/// for it (`None`: the archive's own).
pub(super) fn archive_assets(
    kind: ArchiveKind,
    archive: Vec<u8>,
    hash: Option<String>,
) -> Vec<(String, Vec<u8>)> {
    let hash = hash.unwrap_or_else(|| to_hex(&Sha256::digest(&archive)));
    vec![
        (archive_name(kind), archive),
        (CHECKSUMS_ASSET.to_owned(), checksums(kind, &hash)),
    ]
}

/// A release whose `kind` archive is `archive`, and a `SHA256SUMS.txt`
/// listing `hash` for it (`None`: the archive's own).
pub(super) fn archive_release(
    kind: ArchiveKind,
    archive: Vec<u8>,
    hash: Option<String>,
) -> (Release, Box<OpenUrl>) {
    release(archive_assets(kind, archive, hash))
}

/// A release whose `kind` archive holds the new bundle, and a
/// `SHA256SUMS.txt` listing `hash` for it (`None`: the archive's own).
pub(super) fn bundle_release(kind: ArchiveKind, hash: Option<String>) -> (Release, Box<OpenUrl>) {
    archive_release(kind, pack(kind, &new_files()), hash)
}

/// The running platform's archive of the new bundle and a
/// `SHA256SUMS.txt` listing `hash` for it (`None`: the archive's own), by
/// name, the archive first: the assets the releases list's `v0.0.2`
/// names for this platform.
pub(crate) fn running_assets(hash: Option<String>) -> Vec<(String, Vec<u8>)> {
    let kind = ArchiveKind::RUNNING;
    archive_assets(kind, pack(kind, &new_files()), hash)
}
