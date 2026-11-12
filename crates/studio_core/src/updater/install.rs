//! "Update now" below the window: pick the running platform's archive of a
//! release, download it with progress and cancel, check it against the
//! release's `SHA256SUMS.txt`, unpack it and swap it into the program's
//! folder. Everything happens in a temp folder inside the program's folder, so
//! the swap's moves are renames on one volume, and the temp folder is removed
//! whatever the outcome, unless the old files could not be put back: it then
//! keeps them.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

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
pub(crate) type OpenUrl = dyn Fn(&str) -> Result<Box<dyn Read + Send>, ureq::Error> + Send + Sync;

/// How far the download is: `done` bytes of `total`, the archive's size as
/// the release lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Progress {
    /// Bytes downloaded so far.
    pub(crate) done: u64,
    /// The archive's size in bytes.
    pub(crate) total: u64,
}

/// The program file's size and modification date in the program's folder, as
/// a window notes them when it starts. Two windows may run from one folder:
/// once an update in one has replaced the program, the file is another one,
/// and an update from the other window would move the new program into `old/`
/// in place of the previous one. A rename keeps both values, so a swap that
/// was undone leaves the stamp as it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProgramStamp {
    /// The file's size in bytes.
    len: u64,
    /// When the file was last written.
    modified: SystemTime,
}

impl ProgramStamp {
    /// The stamp of the program in the program's folder `bundle`, `None` when
    /// its file cannot be read.
    pub(crate) fn of(bundle: &Path) -> Option<ProgramStamp> {
        let metadata = fs::metadata(super::program_path(bundle)).ok()?;
        Some(ProgramStamp {
            len: metadata.len(),
            modified: metadata.modified().ok()?,
        })
    }
}

/// Why an update failed. Each message is shown to the member after "Update
/// failed". Every failure but `NotRestored` leaves the program's folder as it
/// was.
#[derive(Debug, Error)]
pub(crate) enum InstallError {
    /// The program's file is no longer the one this window started from:
    /// another window's update most likely replaced it.
    #[error(
        "The program was already replaced while this window was open, most likely by an update \
         from another window. Close this window and start the program again to use the new \
         version."
    )]
    ProgramReplaced,
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
/// folder. Refuses at once when the program's file is no longer the one
/// `stamp` noted (`None` notes nothing). Downloads the running platform's
/// archive through `open_url`, reporting `progress` and stopping when `cancel`
/// is set, checks it against the release's `SHA256SUMS.txt`, unpacks it and
/// swaps it in: the running program moves into `old/` (replacing the one an
/// earlier update left there), the new one takes its place and every other
/// file of the bundle goes over the old one, `data/` aside. `cancel` is read
/// once more before the swap; a swap that has started runs whole. Runs on the
/// caller's thread, the network included, so the dialog calls it from a
/// thread of its own.
///
/// On a failure the program's folder is left as it was (`NotRestored` aside).
pub(crate) fn install(
    release: &Release,
    bundle: &Path,
    stamp: Option<ProgramStamp>,
    open_url: &OpenUrl,
    progress: impl FnMut(Progress),
    cancel: &AtomicBool,
) -> Result<(), InstallError> {
    install_kind(
        ArchiveKind::RUNNING,
        release,
        bundle,
        stamp,
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
    stamp: Option<ProgramStamp>,
    open_url: &OpenUrl,
    mut progress: impl FnMut(Progress),
    cancel: &AtomicBool,
) -> Result<(), InstallError> {
    if let Some(stamp) = stamp
        && ProgramStamp::of(bundle) != Some(stamp)
    {
        return Err(InstallError::ProgramReplaced);
    }
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
        let staged = staged_bundle(&staging)?;
        // The download reads `cancel` before each read, so a Cancel clicked
        // after the last byte would otherwise be missed.
        if cancel.load(Ordering::Relaxed) {
            return Err(InstallError::Cancelled);
        }
        swap(&staged, bundle, &backup)
    })();
    if !matches!(outcome, Err(InstallError::NotRestored { .. }))
        && let Err(error) = fs::remove_dir_all(&temp)
    {
        log::debug!("the update's temp folder {} stays: {error}", temp.display());
    }
    outcome
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

/// The program's folder before and after an update, the release archives that
/// carry the new one and the network seam that serves them: what the
/// install's tests and the update dialog's share.
#[cfg(test)]
pub(crate) mod test_bundle;

#[cfg(test)]
mod tests;
