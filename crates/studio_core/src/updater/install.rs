//! "Update now" below the window: pick the running platform's archive of a
//! release, download it with progress and cancel, check it against the
//! release's `SHA256SUMS.txt`, unpack it and swap it into the program's
//! folder. Everything happens in a temp folder inside the program's folder, so
//! the swap's moves are renames on one volume, and the temp folder is removed
//! whatever the outcome.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use semver::Version;
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::swap::{StagedBundle, SwapStep, exe_file_name, swap};
use super::{Release, ReleaseAsset};

/// The release asset listing each archive's SHA-256, one `sha256sum` line per
/// archive (`<hex>  <archive name>`).
const CHECKSUMS_ASSET: &str = "SHA256SUMS.txt";

/// The temp folder an update works in, inside the program's folder.
const TEMP_FOLDER: &str = ".update";

/// The temp folder's subfolder where the swap moves the files it replaces,
/// until the swap is done.
const BACKUP_FOLDER: &str = "backup";

/// What the download asks the network for: a URL's body as a stream.
/// `open_url` is the real one; a test hands its own reader.
pub type OpenUrl = dyn Fn(&str) -> Result<Box<dyn Read + Send>, ureq::Error> + Send + Sync;

/// How far the download is: `done` bytes of `total`, the archive's size as
/// the release lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    /// Bytes downloaded so far.
    pub done: u64,
    /// The archive's size in bytes.
    pub total: u64,
}

/// A finished update: the version now in the program's folder, and where the
/// program that was running went (`old/UTW.exe`).
#[derive(Debug)]
pub struct Installed {
    /// The installed release's version.
    pub version: Version,
    /// The previous program's path, inside `old/`.
    pub old_exe: PathBuf,
}

/// Why an update failed. Each message is shown to the member after "Update
/// failed". Every failure but `NotRestored` leaves the program's folder as it
/// was.
#[derive(Debug, Error)]
pub enum InstallError {
    /// The release lists no archive for this system.
    #[error("The release has no download for this system (a file ending in {0}).")]
    NoArchive(&'static str),
    /// The release lists no `SHA256SUMS.txt`.
    #[error("The release has no {CHECKSUMS_ASSET} to check the download against.")]
    NoChecksums,
    /// An earlier update left files in the temp folder's backup: it could not
    /// put them back, or the program was closed during its swap. They may be
    /// the only copies of the program's old files, so no update runs until the
    /// member has dealt with them.
    #[error("An earlier update could not put some of the program's old files back, and they are still in {}. Put them back into the program's folder, or remove that folder, then update again.", folder.display())]
    OldFilesLeft {
        /// The backup folder holding them.
        folder: PathBuf,
    },
    /// The temp folder could not be created in the program's folder, which
    /// then cannot take the new files either.
    #[error("The program's folder {} cannot be written to, so the update cannot be put there: {source}", folder.display())]
    NotWritable {
        /// The program's folder.
        folder: PathBuf,
        /// The failure creating the temp folder.
        source: io::Error,
    },
    /// The server refused the download or could not be reached.
    #[error("The download could not start: {0}")]
    Request(ureq::Error),
    /// The download broke off, or its file could not be written.
    #[error("The download stopped: {0}")]
    Download(io::Error),
    /// The member cancelled the download.
    #[error("The download was cancelled.")]
    Cancelled,
    /// `SHA256SUMS.txt` has no line for the archive.
    #[error("{CHECKSUMS_ASSET} has no line for {0}, so the download cannot be checked.")]
    NoChecksumLine(String),
    /// The downloaded archive's SHA-256 is not the one `SHA256SUMS.txt` lists.
    #[error(
        "The downloaded {0} is damaged: its checksum does not match the one in {CHECKSUMS_ASSET}."
    )]
    ChecksumMismatch(String),
    /// The archive could not be unpacked.
    #[error("The downloaded archive could not be unpacked: {0}")]
    Extract(io::Error),
    /// The archive does not hold one folder with the program in it.
    #[error("The downloaded archive does not hold the program inside one folder.")]
    NotABundle,
    /// A move of the swap failed, and every move made before it was undone.
    #[error("The update stopped while {step} ({source}), and every file was put back.")]
    Swap {
        /// The step that failed.
        step: SwapStep,
        /// Its failure.
        source: io::Error,
    },
    /// A move of the swap failed, and undoing the moves made before it failed
    /// too. The temp folder is kept: the files set aside wait there.
    #[error("The update stopped while {step} ({source}), and putting the old files back failed too ({restore}). The old files are in {}.", kept.display())]
    NotRestored {
        /// The step that failed.
        step: SwapStep,
        /// Its failure.
        source: io::Error,
        /// The first undo that failed.
        restore: io::Error,
        /// The folder holding the files set aside.
        kept: PathBuf,
    },
}

/// The release archive one platform downloads: Windows' `.7z`, Linux's
/// `.tar.gz`. Either unpacks on any platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArchiveKind {
    /// `<package>-<version>-windows.7z`.
    SevenZ,
    /// `<package>-<version>-linux.tar.gz`, which keeps the program's executable bit.
    TarGz,
}

impl ArchiveKind {
    /// The archive the running platform downloads. `cfg!` rather than two
    /// `#[cfg]` items, so both kinds are built on every platform.
    const RUNNING: ArchiveKind = if cfg!(windows) {
        ArchiveKind::SevenZ
    } else {
        ArchiveKind::TarGz
    };

    /// How the archive's asset name ends.
    fn suffix(self) -> &'static str {
        match self {
            ArchiveKind::SevenZ => "-windows.7z",
            ArchiveKind::TarGz => "-linux.tar.gz",
        }
    }

    /// Unpacks `archive` into the folder `into`, which exists.
    fn extract(self, archive: &Path, into: &Path) -> io::Result<()> {
        match self {
            ArchiveKind::SevenZ => {
                sevenz_rust2::decompress_file(archive, into).map_err(io::Error::other)
            }
            ArchiveKind::TarGz => {
                let gzip = flate2::read::GzDecoder::new(File::open(archive)?);
                tar::Archive::new(gzip).unpack(into)
            }
        }
    }
}

/// `release`'s archive of `kind` and its `SHA256SUMS.txt`.
fn assets(
    release: &Release,
    kind: ArchiveKind,
) -> Result<(&ReleaseAsset, &ReleaseAsset), InstallError> {
    let archive = release
        .assets
        .iter()
        .find(|asset| asset.name.ends_with(kind.suffix()))
        .ok_or(InstallError::NoArchive(kind.suffix()))?;
    let checksums = release
        .assets
        .iter()
        .find(|asset| asset.name == CHECKSUMS_ASSET)
        .ok_or(InstallError::NoChecksums)?;
    Ok((archive, checksums))
}

/// The SHA-256 `checksums` (a `SHA256SUMS.txt`) lists for `archive_name`.
fn listed_hash(checksums: &str, archive_name: &str) -> Result<String, InstallError> {
    checksums
        .lines()
        .find_map(|line| {
            let (hash, name) = line.split_once("  ")?;
            (name == archive_name).then(|| hash.to_owned())
        })
        .ok_or_else(|| InstallError::NoChecksumLine(archive_name.to_owned()))
}

/// Streams `body` into the new file `path`, hashing and counting the bytes as
/// they come, and reports each step to `progress`. `cancel` is read before
/// every read. Returns the body's SHA-256 in lowercase hex.
fn download(
    mut body: impl Read,
    path: &Path,
    total: u64,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(Progress),
) -> Result<String, InstallError> {
    let mut file = File::create(path).map_err(InstallError::Download)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0; 64 * 1024];
    let mut done = 0;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(InstallError::Cancelled);
        }
        let read = match body.read(&mut buf) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(InstallError::Download(error)),
        };
        hasher.update(&buf[..read]);
        file.write_all(&buf[..read])
            .map_err(InstallError::Download)?;
        done += u64::try_from(read).expect("a read's length fits u64");
        progress(Progress { done, total });
    }
    Ok(to_hex(&hasher.finalize()))
}

/// `bytes` in lowercase hex, the way `SHA256SUMS.txt` writes a hash.
fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The unpacked bundle in `staging`: the archive's one folder, which holds the
/// program.
fn staged_bundle(staging: &Path) -> Result<StagedBundle, InstallError> {
    let entries = fs::read_dir(staging)
        .and_then(Iterator::collect::<Result<Vec<_>, _>>)
        .map_err(InstallError::Extract)?;
    let [entry] = entries.as_slice() else {
        return Err(InstallError::NotABundle);
    };
    let root = entry.path();
    if !root.join(exe_file_name()).is_file() {
        return Err(InstallError::NotABundle);
    }
    StagedBundle::list(root).map_err(InstallError::Extract)
}

/// "Update now": installs `release` into `bundle`, the running program's
/// folder. Downloads the running platform's archive through `open_url`,
/// reporting `progress` and stopping when `cancel` is set, checks it against
/// the release's `SHA256SUMS.txt`, unpacks it and swaps it in: the running
/// program moves into `old/` (replacing the one an earlier update left there),
/// the new one takes its place and every other file of the bundle goes over the
/// old one, `data/` aside. Runs on the caller's thread, the network included,
/// so the dialog calls it from a thread of its own.
///
/// On a failure the program's folder is left as it was (`NotRestored` aside).
pub fn install(
    release: &Release,
    bundle: &Path,
    open_url: &OpenUrl,
    progress: impl FnMut(Progress),
    cancel: &AtomicBool,
) -> Result<Installed, InstallError> {
    install_kind(
        ArchiveKind::RUNNING,
        release,
        bundle,
        open_url,
        progress,
        cancel,
    )
}

/// `install` with the archive kind given, so the tests run both kinds on
/// either platform.
fn install_kind(
    kind: ArchiveKind,
    release: &Release,
    bundle: &Path,
    open_url: &OpenUrl,
    mut progress: impl FnMut(Progress),
    cancel: &AtomicBool,
) -> Result<Installed, InstallError> {
    let (archive, checksums) = assets(release, kind)?;
    let temp = bundle.join(TEMP_FOLDER);
    let backup = temp.join(BACKUP_FOLDER);
    if fs::read_dir(&backup).is_ok_and(|mut entries| entries.next().is_some()) {
        return Err(InstallError::OldFilesLeft { folder: backup });
    }
    create_temp(&temp).map_err(|source| InstallError::NotWritable {
        folder: bundle.to_owned(),
        source,
    })?;
    // One closure, so every failure after the temp folder exists still reaches
    // its removal below.
    let outcome = (|| {
        let mut listed = String::new();
        open_url(&checksums.download_url)
            .map_err(InstallError::Request)?
            .read_to_string(&mut listed)
            .map_err(InstallError::Download)?;
        let expected = listed_hash(&listed, &archive.name)?;
        let path = temp.join(&archive.name);
        let body = open_url(&archive.download_url).map_err(InstallError::Request)?;
        let actual = download(body, &path, archive.size, cancel, &mut progress)?;
        if !actual.eq_ignore_ascii_case(&expected) {
            return Err(InstallError::ChecksumMismatch(archive.name.clone()));
        }
        let staging = temp.join("staging");
        fs::create_dir(&staging).map_err(InstallError::Extract)?;
        kind.extract(&path, &staging)
            .map_err(InstallError::Extract)?;
        swap(&staged_bundle(&staging)?, bundle, &backup)
    })();
    if !matches!(outcome, Err(InstallError::NotRestored { .. }))
        && let Err(error) = fs::remove_dir_all(&temp)
    {
        log::debug!("the update's temp folder {} stays: {error}", temp.display());
    }
    Ok(Installed {
        version: release.version.clone(),
        old_exe: outcome?,
    })
}

/// Creates the temp folder, first removing one an update that stopped midway
/// (the program closed during a download) left behind. The caller has made
/// sure it holds no backup.
fn create_temp(temp: &Path) -> io::Result<()> {
    if temp.exists() {
        fs::remove_dir_all(temp)?;
    }
    fs::create_dir(temp)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};
    use std::io::Cursor;

    use super::*;

    /// The new bundle's folder inside its archive.
    const TOP: &str = "unlimited-tool-works-0.0.2";

    /// The empty folder the new bundle adds.
    const NEW_FOLDER: &str = "exports";

    /// A file name and its text.
    type Files = Vec<(String, &'static str)>;

    /// The new bundle's files, relative to its folder: the program, the readme,
    /// the default settings, a license and a launcher.
    fn new_files() -> Files {
        vec![
            (exe_file_name(), "new program"),
            ("README.txt".to_owned(), "new readme"),
            ("data/settings.toml".to_owned(), "# the default settings"),
            ("licenses/LICENSE-MIT".to_owned(), "new license"),
            ("quick_compile.bat".to_owned(), "new launcher"),
        ]
    }

    /// The program's folder before the update: the same files with the old
    /// version's bytes, and the member's own settings.
    fn old_files() -> Files {
        vec![
            (exe_file_name(), "old program"),
            ("README.txt".to_owned(), "old readme"),
            ("data/settings.toml".to_owned(), "pes_version = 21"),
            ("licenses/LICENSE-MIT".to_owned(), "old license"),
            ("quick_compile.bat".to_owned(), "old launcher"),
        ]
    }

    /// A fresh `<temp>/studio_core_updater_<name>_<pid>` holding `files`.
    fn folder_with(name: &str, files: &Files) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("studio_core_updater_{name}_{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        for (relative, text) in files {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        root
    }

    /// Every folder (`None`) and file (its text) under `root`, by its path
    /// relative to `root` with `/` separators.
    fn snapshot(root: &Path) -> BTreeMap<String, Option<String>> {
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
    fn pack_entries(kind: ArchiveKind, files: &Files, folders: &[String]) -> Vec<u8> {
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
                let gzip =
                    flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
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

    /// `files` and the empty `NEW_FOLDER`, packed under `TOP` as an archive of
    /// `kind`: the shape a release's archive has.
    fn pack(kind: ArchiveKind, files: &Files) -> Vec<u8> {
        let under_top: Files = files
            .iter()
            .map(|(relative, text)| (format!("{TOP}/{relative}"), *text))
            .collect();
        pack_entries(kind, &under_top, &[format!("{TOP}/{NEW_FOLDER}")])
    }

    /// The archive asset's name for `kind`.
    fn archive_name(kind: ArchiveKind) -> String {
        format!("unlimited-tool-works-0.0.2{}", kind.suffix())
    }

    /// `SHA256SUMS.txt` as the release workflow writes it: the Windows line,
    /// then the Linux one; `kind`'s line carries `hash`, the other a hash of
    /// nothing in the test.
    fn checksums(kind: ArchiveKind, hash: &str) -> Vec<u8> {
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

    /// Release v0.0.2 listing `assets` (name, bytes), and the network seam
    /// serving each by its URL, every body in two reads so a download
    /// reports progress between them.
    fn release(assets: Vec<(String, Vec<u8>)>) -> (Release, Box<OpenUrl>) {
        let url = |name: &str| {
            format!(
                "https://github.com/the4chancup/unlimited-tool-works/releases/download/v0.0.2/{name}"
            )
        };
        let release = Release {
            version: Version::new(0, 0, 2),
            notes: String::new(),
            page_url: "https://github.com/the4chancup/unlimited-tool-works/releases/tag/v0.0.2"
                .to_owned(),
            assets: assets
                .iter()
                .map(|(name, bytes)| ReleaseAsset {
                    name: name.clone(),
                    download_url: url(name),
                    size: bytes.len() as u64,
                })
                .collect(),
        };
        let served: HashMap<String, Vec<u8>> = assets
            .into_iter()
            .map(|(name, bytes)| (url(&name), bytes))
            .collect();
        let open_url = move |requested: &str| -> Result<Box<dyn Read + Send>, ureq::Error> {
            let bytes = served.get(requested).ok_or(ureq::Error::StatusCode(404))?;
            let (first, rest) = bytes.split_at(bytes.len() / 2);
            Ok(Box::new(
                Cursor::new(first.to_vec()).chain(Cursor::new(rest.to_vec())),
            ))
        };
        (release, Box::new(open_url))
    }

    /// A release whose `kind` archive is `archive`, and a `SHA256SUMS.txt`
    /// listing `hash` for it (`None`: the archive's own).
    fn archive_release(
        kind: ArchiveKind,
        archive: Vec<u8>,
        hash: Option<String>,
    ) -> (Release, Box<OpenUrl>) {
        let hash = hash.unwrap_or_else(|| to_hex(&Sha256::digest(&archive)));
        release(vec![
            (archive_name(kind), archive),
            (CHECKSUMS_ASSET.to_owned(), checksums(kind, &hash)),
        ])
    }

    /// A release whose `kind` archive holds the new bundle, and a
    /// `SHA256SUMS.txt` listing `hash` for it (`None`: the archive's own).
    fn bundle_release(kind: ArchiveKind, hash: Option<String>) -> (Release, Box<OpenUrl>) {
        archive_release(kind, pack(kind, &new_files()), hash)
    }

    /// Both archive kinds, with a name for the test folders and messages.
    const KINDS: [(ArchiveKind, &str); 2] =
        [(ArchiveKind::SevenZ, "7z"), (ArchiveKind::TarGz, "tar")];

    #[test]
    fn an_update_puts_the_new_bundle_in_place_and_keeps_data() {
        for (kind, label) in KINDS {
            let bundle = folder_with(&format!("happy_{label}"), &old_files());
            let (release, open_url) = bundle_release(kind, None);
            let mut reports = Vec::new();
            let installed = install_kind(
                kind,
                &release,
                &bundle,
                &open_url,
                |progress| reports.push(progress),
                &AtomicBool::new(false),
            )
            .unwrap_or_else(|error| panic!("{label}: {error}"));

            let exe = exe_file_name();
            assert_eq!(installed.version, Version::new(0, 0, 2));
            assert_eq!(installed.old_exe, bundle.join("old").join(&exe));
            let total = release.assets[0].size;
            assert_eq!(
                reports.last(),
                Some(&Progress { done: total, total }),
                "{label}"
            );
            assert!(
                reports.len() >= 2,
                "{label}: progress between the two reads"
            );

            let expected: BTreeMap<String, Option<String>> = [
                (exe.clone(), Some("new program")),
                (format!("old/{exe}"), Some("old program")),
                ("old".to_owned(), None),
                ("README.txt".to_owned(), Some("new readme")),
                ("data".to_owned(), None),
                ("data/settings.toml".to_owned(), Some("pes_version = 21")),
                ("licenses".to_owned(), None),
                ("licenses/LICENSE-MIT".to_owned(), Some("new license")),
                ("quick_compile.bat".to_owned(), Some("new launcher")),
                (NEW_FOLDER.to_owned(), None),
            ]
            .into_iter()
            .map(|(path, text)| (path, text.map(str::to_owned)))
            .collect();
            assert_eq!(
                snapshot(&bundle),
                expected,
                "{label}: the temp folder gone too"
            );
            fs::remove_dir_all(bundle).unwrap();
        }
    }

    #[test]
    fn an_older_program_in_old_is_replaced_by_the_running_one() {
        for (kind, label) in KINDS {
            let mut files = old_files();
            files.push((format!("old/{}", exe_file_name()), "older program"));
            let bundle = folder_with(&format!("older_{label}"), &files);
            let (release, open_url) = bundle_release(kind, None);
            install_kind(
                kind,
                &release,
                &bundle,
                &open_url,
                |_| {},
                &AtomicBool::new(false),
            )
            .unwrap_or_else(|error| panic!("{label}: {error}"));
            assert_eq!(
                fs::read_to_string(bundle.join("old").join(exe_file_name())).unwrap(),
                "old program",
                "{label}"
            );
            assert!(!bundle.join(TEMP_FOLDER).exists(), "{label}");
            fs::remove_dir_all(bundle).unwrap();
        }
    }

    #[test]
    fn a_checksum_that_does_not_match_changes_nothing() {
        for (kind, label) in KINDS {
            let bundle = folder_with(&format!("mismatch_{label}"), &old_files());
            let before = snapshot(&bundle);
            let (release, open_url) = bundle_release(kind, Some(to_hex(&[0x11; 32])));
            let outcome = install_kind(
                kind,
                &release,
                &bundle,
                &open_url,
                |_| {},
                &AtomicBool::new(false),
            );
            assert!(
                matches!(&outcome, Err(InstallError::ChecksumMismatch(name)) if *name == archive_name(kind)),
                "{label}: {outcome:?}"
            );
            assert_eq!(snapshot(&bundle), before, "{label}");
            fs::remove_dir_all(bundle).unwrap();
        }
    }

    #[test]
    fn checksums_without_the_archives_line_change_nothing() {
        let kind = ArchiveKind::SevenZ;
        let bundle = folder_with("no_line", &old_files());
        let before = snapshot(&bundle);
        let archive = pack(kind, &new_files());
        let only_linux = format!(
            "{}  {}\n",
            to_hex(&Sha256::digest(&archive)),
            archive_name(ArchiveKind::TarGz)
        );
        let (release, open_url) = release(vec![
            (archive_name(kind), archive),
            (CHECKSUMS_ASSET.to_owned(), only_linux.into_bytes()),
        ]);
        let outcome = install_kind(
            kind,
            &release,
            &bundle,
            &open_url,
            |_| {},
            &AtomicBool::new(false),
        );
        assert!(
            matches!(&outcome, Err(InstallError::NoChecksumLine(name)) if *name == archive_name(kind)),
            "{outcome:?}"
        );
        assert_eq!(snapshot(&bundle), before);
        fs::remove_dir_all(bundle).unwrap();
    }

    #[test]
    fn a_download_cancelled_midway_changes_nothing() {
        for (kind, label) in KINDS {
            let bundle = folder_with(&format!("cancel_{label}"), &old_files());
            let before = snapshot(&bundle);
            let (release, open_url) = bundle_release(kind, None);
            let cancel = AtomicBool::new(false);
            let mut reports = Vec::new();
            let outcome = install_kind(
                kind,
                &release,
                &bundle,
                &open_url,
                |progress| {
                    reports.push(progress);
                    cancel.store(true, Ordering::Relaxed);
                },
                &cancel,
            );
            assert!(
                matches!(outcome, Err(InstallError::Cancelled)),
                "{label}: {outcome:?}"
            );
            let [report] = reports.as_slice() else {
                panic!("{label}: one read before the cancel, got {reports:?}");
            };
            assert!(report.done < report.total, "{label}: cancelled midway");
            assert_eq!(snapshot(&bundle), before, "{label}");
            fs::remove_dir_all(bundle).unwrap();
        }
    }

    #[test]
    fn a_swap_failing_midway_puts_every_file_back() {
        for (kind, label) in KINDS {
            for older in [false, true] {
                // A folder where the new launcher must go, which the launcher's move
                // fails on: renaming a file over a non-empty folder is refused on
                // Windows and Linux alike. The launcher comes last by name, so the
                // program, the readme and the license have moved by then.
                let mut files: Files = old_files()
                    .into_iter()
                    .filter(|(name, _)| name != "quick_compile.bat")
                    .collect();
                files.push(("quick_compile.bat/notes.txt".to_owned(), "a member's notes"));
                if older {
                    files.push((format!("old/{}", exe_file_name()), "older program"));
                }
                let bundle = folder_with(&format!("swap_{label}_{older}"), &files);
                let before = snapshot(&bundle);
                let (release, open_url) = bundle_release(kind, None);
                let outcome = install_kind(
                    kind,
                    &release,
                    &bundle,
                    &open_url,
                    |_| {},
                    &AtomicBool::new(false),
                );
                assert!(
                    matches!(
                        &outcome,
                        Err(InstallError::Swap { step: SwapStep::PlaceFile(path), .. })
                            if path == Path::new("quick_compile.bat")
                    ),
                    "{label}, older {older}: {outcome:?}"
                );
                assert_eq!(snapshot(&bundle), before, "{label}, older {older}");
                fs::remove_dir_all(bundle).unwrap();
            }
        }
    }

    #[test]
    fn a_release_without_this_platforms_archive_or_checksums_is_refused() {
        let other = if ArchiveKind::RUNNING == ArchiveKind::SevenZ {
            ArchiveKind::TarGz
        } else {
            ArchiveKind::SevenZ
        };
        let bundle = folder_with("assets", &old_files());
        let before = snapshot(&bundle);
        let checksums = (CHECKSUMS_ASSET.to_owned(), b"listed".to_vec());
        let (release_without_archive, open_url) =
            release(vec![(archive_name(other), b"archive".to_vec()), checksums]);
        let outcome = install(
            &release_without_archive,
            &bundle,
            &open_url,
            |_| {},
            &AtomicBool::new(false),
        );
        assert!(
            matches!(outcome, Err(InstallError::NoArchive(suffix)) if suffix == ArchiveKind::RUNNING.suffix()),
            "{outcome:?}"
        );
        let (release_without_checksums, open_url) = release(vec![(
            archive_name(ArchiveKind::RUNNING),
            b"archive".to_vec(),
        )]);
        let outcome = install(
            &release_without_checksums,
            &bundle,
            &open_url,
            |_| {},
            &AtomicBool::new(false),
        );
        assert!(
            matches!(outcome, Err(InstallError::NoChecksums)),
            "{outcome:?}"
        );
        assert_eq!(snapshot(&bundle), before);
        fs::remove_dir_all(bundle).unwrap();
    }

    #[test]
    fn the_running_platform_downloads_its_own_archive() {
        let (release, _) = release(vec![
            (archive_name(ArchiveKind::SevenZ), b"7z".to_vec()),
            (archive_name(ArchiveKind::TarGz), b"tar".to_vec()),
            (CHECKSUMS_ASSET.to_owned(), b"listed".to_vec()),
        ]);
        let (archive, checksums) = assets(&release, ArchiveKind::RUNNING).unwrap();
        let expected = if cfg!(windows) {
            "-windows.7z"
        } else {
            "-linux.tar.gz"
        };
        assert!(archive.name.ends_with(expected), "{}", archive.name);
        assert_eq!(checksums.name, CHECKSUMS_ASSET);
    }

    #[test]
    fn files_an_earlier_update_left_aside_stop_the_next_one() {
        let kind = ArchiveKind::SevenZ;
        let mut files = old_files();
        files.push((
            format!("{TEMP_FOLDER}/{BACKUP_FOLDER}/README.txt"),
            "the readme an earlier update moved aside",
        ));
        let bundle = folder_with("kept_backup", &files);
        let before = snapshot(&bundle);
        let (release, open_url) = bundle_release(kind, None);
        let outcome = install_kind(
            kind,
            &release,
            &bundle,
            &open_url,
            |_| {},
            &AtomicBool::new(false),
        );
        let backup = bundle.join(TEMP_FOLDER).join(BACKUP_FOLDER);
        assert!(
            matches!(&outcome, Err(InstallError::OldFilesLeft { folder }) if *folder == backup),
            "{outcome:?}"
        );
        assert_eq!(snapshot(&bundle), before, "the leftover kept too");
        fs::remove_dir_all(bundle).unwrap();

        let mut files = old_files();
        files.push((
            format!("{TEMP_FOLDER}/{}", archive_name(kind)),
            "the first half of an archive",
        ));
        let bundle = folder_with("cut_download", &files);
        install_kind(
            kind,
            &release,
            &bundle,
            &open_url,
            |_| {},
            &AtomicBool::new(false),
        )
        .unwrap_or_else(|error| panic!("a download cut short is cleared: {error}"));
        assert!(!bundle.join(TEMP_FOLDER).exists());
        assert_eq!(
            fs::read_to_string(bundle.join("README.txt")).unwrap(),
            "new readme"
        );
        fs::remove_dir_all(bundle).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn the_installed_linux_program_is_executable() {
        use std::os::unix::fs::PermissionsExt;

        let kind = ArchiveKind::TarGz;
        let bundle = folder_with("executable", &old_files());
        let exe = bundle.join(exe_file_name());
        let owner_may_run =
            |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o100 != 0;
        assert!(
            !owner_may_run(&exe),
            "the premise: the old program is not executable"
        );
        let (release, open_url) = bundle_release(kind, None);
        install_kind(
            kind,
            &release,
            &bundle,
            &open_url,
            |_| {},
            &AtomicBool::new(false),
        )
        .unwrap_or_else(|error| panic!("{error}"));
        assert!(owner_may_run(&exe), "the installed program is executable");
        fs::remove_dir_all(bundle).unwrap();
    }

    #[test]
    fn an_archive_that_is_not_one_program_folder_changes_nothing() {
        let kind = ArchiveKind::SevenZ;
        let exe = exe_file_name();
        // The program in both folders, so the folders' count alone refuses it,
        // whichever folder the listing returns first.
        let two_folders: Files = vec![
            (format!("{TOP}/{exe}"), "new program"),
            (format!("other/{exe}"), "another program"),
        ];
        let no_program: Files = vec![(format!("{TOP}/README.txt"), "new readme")];
        for (case, files) in [("two_folders", two_folders), ("no_program", no_program)] {
            let bundle = folder_with(&format!("not_a_bundle_{case}"), &old_files());
            let before = snapshot(&bundle);
            let (release, open_url) = archive_release(kind, pack_entries(kind, &files, &[]), None);
            let outcome = install_kind(
                kind,
                &release,
                &bundle,
                &open_url,
                |_| {},
                &AtomicBool::new(false),
            );
            assert!(
                matches!(outcome, Err(InstallError::NotABundle)),
                "{case}: {outcome:?}"
            );
            assert_eq!(snapshot(&bundle), before, "{case}");
            fs::remove_dir_all(bundle).unwrap();
        }
    }
}
