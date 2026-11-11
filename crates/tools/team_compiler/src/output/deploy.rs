//! Staging, promotion and deployment (`spec/team_compiler/pipeline.md` "6. Post-processing"): a CPK,
//! or a test or sideload run's loose tree, is written in a staging folder of its own run and only
//! then moved to its final path, so a failed run never leaves a half-written output where the
//! game or the user could pick it up. A run that deploys checks, before any export is read, that
//! it can install into the PES folder's `download/`, and installs its CPKs there by a copy and
//! renames, all of them or none.

use std::collections::BTreeSet;
use std::fs::{self, File, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use pes_version::PesVersion;
use pipeline::CpkStem;
use studio_core::{Disposition, Message, Scope};

use crate::bins::installed;
use crate::messages::{Code, tool_message};

/// The output subfolder that holds every run's staging folder.
const STAGING: &str = ".staging";

/// The game folder's subfolder a sideloading runtime serves to the running game (FoxDen on PES
/// 2018 to 2021, Sider on PES 2017), and the name of a sideload run's staged tree.
pub(crate) const LIVECPK: &str = "livecpk";

/// The output folder's subfolder a test run's tree replaces, and the name of the staged tree.
pub(crate) const TEST_OUTPUT: &str = "test_output";

/// The game folder's subfolder the game loads the CPKs `DpFileList.bin` lists from.
pub(crate) const DOWNLOAD: &str = "download";

/// The list of the CPKs the game loads, in `download/`.
pub(crate) const DPFILELIST: &str = "DpFileList.bin";

/// The subcommand that installs the official list, named under the context key `command` in
/// each finding it fixes (`dpfilelist_outdated`, `dpfilelist_not_official`,
/// `dpfilelist_cpk_missing`): the CLI renders a finding as its code and context alone, so the
/// key is how its line names the fix (the GUI offers a button instead). `upgrade-dpfl`'s own
/// `dpfilelist_upgrade_planned` names it with `--yes`.
pub(crate) const UPGRADE_COMMAND: &str =
    concat!(studio_core::exe_name!(), " team-compiler upgrade-dpfl");

/// This run's id, `<pid>-<unix ms>`: the process id keeps two runs at once apart, the time two
/// runs of a recycled process id.
fn run_id() -> String {
    // A clock set before 1970 leaves only the process id to tell runs apart, which is enough
    // for the runs alive at once.
    let millis = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(elapsed) => elapsed.as_millis(),
        Err(_) => 0,
    };
    format!("{}-{millis}", std::process::id())
}

/// The name of the lock file of the run `run_id`, beside its folder in `.staging/`.
fn lock_name(run_id: &str) -> String {
    format!("{run_id}.lock")
}

/// A run's staging: its folder `<output>/.staging/<run_id>`, which the run's sink creates with
/// the first file it writes, and the exclusive lock held on `<output>/.staging/<run_id>.lock`
/// for the run's whole life, which tells every other run that the folder is in use. The OS
/// releases the lock however the process ends, so a killed run's folder is known to be dead by
/// the next run, which removes it (`Staging::create`).
///
/// Dropping the value removes the folder with whatever is still in it, then releases the lock
/// and removes its file, then `.staging/` when no other run is in it: every way a run ends, a
/// panic included, leaves nothing in the output folder but what it promoted. A failure to
/// remove is logged, not returned: a run that failed already carries the error that matters,
/// and a promoted one has its output in place.
pub(crate) struct Staging {
    /// `<output>/.staging`.
    root: PathBuf,
    /// `<root>/<run_id>`, the run's own folder.
    folder: PathBuf,
    /// `<root>/<run_id>.lock`.
    lock_path: PathBuf,
    /// The handle holding the lock; taken when the lock is released.
    lock: Option<File>,
}

impl Staging {
    /// Creates `<output>/.staging/` and this run's lock file, and locks it, before anything is
    /// written in this run's folder; then removes there every other run's staging that is dead
    /// (a run killed before it cleaned up: `sweep`). Failing to create the folder or to lock the
    /// file is the error, naming the path.
    pub(crate) fn create(output: &Path) -> anyhow::Result<Staging> {
        let root = output.join(STAGING);
        fs::create_dir_all(&root)
            .with_context(|| format!("cannot create the folder {}", root.display()))?;
        let run_id = run_id();
        let lock_path = root.join(lock_name(&run_id));
        let cannot_lock = || format!("cannot lock {} for this run", lock_path.display());
        let lock = File::create_new(&lock_path).with_context(cannot_lock)?;
        lock.try_lock()
            .map_err(io::Error::from)
            .with_context(cannot_lock)?;
        // Swept with the lock held: a run ending meanwhile removes `.staging/` only while it is
        // empty, and the held lock is also what makes the sweep leave this run's own staging.
        sweep(&root);
        Ok(Staging {
            folder: root.join(&run_id),
            root,
            lock_path,
            lock: Some(lock),
        })
    }

    /// The run's own folder, where its output is staged.
    pub(crate) fn folder(&self) -> &Path {
        &self.folder
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.folder) {
            log::debug!("{}: kept: {error}", self.folder.display());
        }
        // Released only now: while it is held, a run starting meanwhile leaves the folder to
        // this one.
        drop(self.lock.take());
        if let Err(error) = fs::remove_file(&self.lock_path) {
            log::debug!("{}: kept: {error}", self.lock_path.display());
        }
        // Another run alive at once may still be using `.staging/`; it removes the folder when
        // it finishes, so a refusal here is expected.
        if let Err(error) = fs::remove_dir(&self.root) {
            log::debug!("{}: kept: {error}", self.root.display());
        }
    }
}

/// Removes from `root`, the `.staging/` folder, the staging of every run that is dead: each
/// folder whose lock file is missing or can be locked, with that file, and each lock file with
/// no folder that can be locked. A run still alive holds its lock, so its folder is left, and so
/// is the lock of a run that has not created its folder yet. A failure is logged, never the
/// sweeping run's.
fn sweep(root: &Path) {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            log::debug!("{}: not swept: {error}", root.display());
            return;
        }
    };
    let mut run_ids = BTreeSet::new();
    for entry in entries {
        match entry {
            Ok(entry) => {
                let name = entry.file_name().to_string_lossy().into_owned();
                let run_id = name.strip_suffix(".lock").unwrap_or(&name).to_owned();
                run_ids.insert(run_id);
            }
            Err(error) => log::debug!("{}: an entry not swept: {error}", root.display()),
        }
    }
    for run_id in run_ids {
        sweep_run(root, &run_id);
    }
}

/// Removes the staging of the run `run_id` from `root`, its folder and then its lock file,
/// unless its lock is held. Anything else of that name (a stray file) is left.
fn sweep_run(root: &Path, run_id: &str) {
    let folder = root.join(run_id);
    let lock_path = root.join(lock_name(run_id));
    // Held for the removal, so two runs sweeping at once do not both remove the folder.
    let lock = match File::open(&lock_path) {
        Ok(lock) => match lock.try_lock() {
            Ok(()) => Some(lock),
            Err(TryLockError::WouldBlock) => return,
            Err(TryLockError::Error(error)) => {
                log::debug!("{}: not swept: {error}", lock_path.display());
                return;
            }
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            log::debug!("{}: not swept: {error}", lock_path.display());
            return;
        }
    };
    if folder.is_dir()
        && let Err(error) = fs::remove_dir_all(&folder)
    {
        log::debug!("{}: a dead run's staging, kept: {error}", folder.display());
        return;
    }
    if let Some(lock) = lock {
        drop(lock);
        if let Err(error) = fs::remove_file(&lock_path) {
            log::debug!("{}: kept: {error}", lock_path.display());
        }
    }
}

/// The CPK file name for `cpk_stem`.
pub(crate) fn cpk_file_name(cpk_stem: &CpkStem) -> String {
    format!("{}.cpk", cpk_stem.as_str())
}

/// Creates the output folder `output` if it is missing and probes it for writing. Each failure
/// names the folder, not the probe.
pub(crate) fn prepare_output_folder(output: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(output)
        .with_context(|| format!("cannot create the output folder {}", output.display()))?;
    probe_folder(output)
        .with_context(|| format!("cannot write in the output folder {}", output.display()))
}

/// Probes `folder` for writing: a file created in it and removed.
fn probe_folder(folder: &Path) -> io::Result<()> {
    let probe = folder.join(format!(".write-probe-{}", std::process::id()));
    fs::write(&probe, b"")?;
    fs::remove_file(&probe)
}

/// The PES folder's `download/` a run installs into, and whether the game will load it.
#[derive(Debug)]
pub(crate) struct Download {
    pub(crate) folder: PathBuf,
    /// `false` when the installed DpFileList names a CPK the folder lacks
    /// (`dpfilelist_cpk_missing`): the game then rejects the whole folder and runs vanilla.
    pub(crate) loaded: bool,
}

/// The checks a run that deploys makes before any export is read, in order: the PES folder
/// `pes_folder` is a folder (`pes_folder_not_found`), it holds `version`'s exe
/// (`pes_version_mismatch`, a Warning: the checks go on), its `download/DpFileList.bin` exists
/// and lists every CPK of the run, `cpks` (those it lacks that the `official` list names are
/// one `dpfilelist_outdated`, the others one `cpk_name_unlisted`, each joining their names in
/// `cpks`' order), the list is the official one (`dpfilelist_not_official`, a Warning), every
/// CPK it lists but the run's has its file in `download/` (`dpfilelist_cpk_missing`, a
/// Warning), and each CPK can be written in `download/` (`deploy_target_unwritable`, the first
/// that cannot). The first failing check ends them (both list findings when the list lacks both
/// kinds). `dpfilelist_outdated` and `cpk_name_unlisted` are Fatal (`AbortRun`): the member has to
/// fix the list or the CPK name and compile again anyway, so the run ends there rather than spend
/// minutes on CPKs nobody installs; the other Errors end the checks but not the run. A missing or
/// unreadable list adds no finding: the working-bin walk, which reads it next, reports it
/// (`dpfilelist_missing`, `installed_bin_unreadable`). Returns the `download/` to install into,
/// `loaded` `false` when the game will run vanilla over it, or `None` when a check failed, and the
/// findings. `promoted` (the one CPK's path in the output folder, or the folder) is the CPKs'
/// destination when the run goes on without deploying, which each remaining Error names; the list
/// findings abort the run, so they name none.
pub(crate) fn preflight(
    pes_folder: &Path,
    version: PesVersion,
    cpks: &[CpkStem],
    promoted: &Path,
    official: &[String],
) -> (Option<Download>, Vec<Message>) {
    let output = || ("output", promoted.display().to_string());
    let mut messages = Vec::new();
    if !pes_folder.is_dir() {
        messages.push(tool_message(
            Code::PesFolderNotFound,
            Scope::Run,
            Disposition::Keep,
            vec![("path", pes_folder.display().to_string()), output()],
        ));
        return (None, messages);
    }
    let exe = format!("PES{}.exe", version.year());
    if !pes_folder.join(&exe).is_file() {
        messages.push(tool_message(
            Code::PesVersionMismatch,
            Scope::Run,
            Disposition::Keep,
            vec![("path", pes_folder.display().to_string()), ("exe", exe)],
        ));
    }
    let download = pes_folder.join(DOWNLOAD);
    let list_path = download.join(DPFILELIST);
    let list = match installed::read_list(&list_path) {
        Ok(Some(list)) => list,
        Ok(None) => return (None, messages),
        Err(unreadable) => {
            log::debug!(
                "{}: not deployed: {:#}",
                unreadable.path.display(),
                unreadable.error
            );
            return (None, messages);
        }
    };
    let names: Vec<String> = cpks.iter().map(cpk_file_name).collect();
    let (outdated, unlisted): (Vec<&str>, Vec<&str>) = names
        .iter()
        .map(String::as_str)
        .filter(|name| !list.iter().any(|entry| entry == name))
        .partition(|name| official.iter().any(|entry| entry == name));
    if !outdated.is_empty() || !unlisted.is_empty() {
        let path = || ("path", list_path.display().to_string());
        if !outdated.is_empty() {
            messages.push(tool_message(
                Code::DpfilelistOutdated,
                Scope::Run,
                Disposition::AbortRun,
                vec![("cpk", outdated.join(", ")), path(), upgrade_command()],
            ));
        }
        if !unlisted.is_empty() {
            messages.push(tool_message(
                Code::CpkNameUnlisted,
                Scope::Run,
                Disposition::AbortRun,
                vec![("cpk", unlisted.join(", ")), path()],
            ));
        }
        return (None, messages);
    }
    messages.extend(not_official(&list_path, &list, official));
    let missing = cpks_missing(&download, &list, &names);
    let loaded = missing.is_none();
    messages.extend(missing);
    for name in &names {
        if let Err((path, error)) = probe_download(&download, name) {
            messages.push(tool_message(
                Code::DeployTargetUnwritable,
                Scope::Run,
                Disposition::Keep,
                vec![
                    ("path", path.display().to_string()),
                    ("error", error.to_string()),
                    output(),
                ],
            ));
            return (None, messages);
        }
    }
    (
        Some(Download {
            folder: download,
            loaded,
        }),
        messages,
    )
}

/// The `command` context entry of a finding about the installed list.
fn upgrade_command() -> (&'static str, String) {
    ("command", UPGRADE_COMMAND.to_owned())
}

/// `dpfilelist_not_official` when the `installed` list, read from `list_path`, is not the
/// `official` one entry for entry; its context says how they differ: `missing`, the official
/// entries it lacks, in the official order; `unofficial`, its entries the official list lacks,
/// in its order; `order` = `differs`, when the entries both lists hold, each taken in its own
/// list's order, are not the same sequence. Each of the three only when it applies.
fn not_official(list_path: &Path, installed: &[String], official: &[String]) -> Option<Message> {
    if installed == official {
        return None;
    }
    let missing: Vec<&str> = official
        .iter()
        .filter(|entry| !installed.contains(entry))
        .map(String::as_str)
        .collect();
    let unofficial: Vec<&str> = installed
        .iter()
        .filter(|entry| !official.contains(entry))
        .map(String::as_str)
        .collect();
    let shared_in_installed = installed.iter().filter(|entry| official.contains(entry));
    let shared_in_official = official.iter().filter(|entry| installed.contains(entry));
    let mut context = vec![("path", list_path.display().to_string())];
    if !missing.is_empty() {
        context.push(("missing", missing.join(", ")));
    }
    if !unofficial.is_empty() {
        context.push(("unofficial", unofficial.join(", ")));
    }
    if !shared_in_installed.eq(shared_in_official) {
        context.push(("order", "differs".to_owned()));
    }
    context.push(upgrade_command());
    Some(tool_message(
        Code::DpfilelistNotOfficial,
        Scope::Run,
        Disposition::Keep,
        context,
    ))
}

/// `dpfilelist_cpk_missing` when a CPK the `installed` list names, other than the run's own
/// `own` (which the run writes), has no file in `download`, naming those CPKs in list order:
/// the game then loads none of the folder's CPKs.
fn cpks_missing(download: &Path, installed: &[String], own: &[String]) -> Option<Message> {
    let missing: Vec<&str> = installed
        .iter()
        .filter(|entry| !own.contains(entry) && !download.join(entry).is_file())
        .map(String::as_str)
        .collect();
    if missing.is_empty() {
        return None;
    }
    Some(tool_message(
        Code::DpfilelistCpkMissing,
        Scope::Run,
        Disposition::Keep,
        vec![
            ("path", download.display().to_string()),
            ("files", missing.join(", ")),
            upgrade_command(),
        ],
    ))
}

/// Whether the CPK `name` can be written in `download`: the old CPK opened for writing, not
/// truncated, when there is one, else a probe file created and removed in the folder. The
/// failure is the path that cannot be written and the error. An old CPK that cannot be opened
/// for another reason than access denied is in use, by PES most likely, which may be closed
/// before the run ends: the deployment meets the lock if it is not, so the probe passes.
fn probe_download(download: &Path, name: &str) -> Result<(), (PathBuf, io::Error)> {
    let old = download.join(name);
    match fs::OpenOptions::new().write(true).open(&old) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            probe_folder(download).map_err(|error| (download.to_owned(), error))
        }
        Err(error) if elevation::is_access_denied(&error) => Err((old, error)),
        Err(error) => {
            log::debug!("{}: in use, not probed: {error}", old.display());
            Ok(())
        }
    }
}

/// Why installing the run's CPKs into `download/` failed, named by the step that failed and
/// not by the OS error: Windows reports a file held open without delete sharing as access
/// denied, the error of a folder that needs elevation, while only the renames need the old
/// CPKs free.
#[derive(Debug)]
pub(crate) enum DeployFailure {
    /// A copy to `{name}.cpk.partial` failed: the folder denies writes
    /// (`deploy_target_unwritable`).
    Copy(io::Error),
    /// A rename failed, of `cpk`'s `.partial` over its old CPK, or (when the run writes
    /// several CPKs) of its old CPK aside to `.cpk.old` or of its `.partial` into place: the
    /// old CPK is in use, by PES most likely (`old_cpk_locked`).
    Rename { cpk: CpkStem, error: io::Error },
}

/// A deployment installed and not yet committed: the run's CPKs in place in `download/`,
/// the old ones they replaced kept aside, until the patch and the savefile are written.
#[derive(Debug)]
#[must_use]
pub(crate) struct Installed {
    /// The run's CPKs now in `download/`.
    placed: Vec<PathBuf>,
    /// Each old CPK moved aside: its `.old` path, then its own.
    moved: Vec<(PathBuf, PathBuf)>,
}

impl Installed {
    /// Removes the old CPKs kept aside (a failure is logged: the run is installed).
    pub(crate) fn commit(self) {
        for (aside, _) in self.moved {
            remove_logged(&aside);
        }
    }

    /// Puts `download/` back as the run found it: `undo` over `placed` and `moved`.
    pub(crate) fn roll_back(self) {
        undo(&self.placed, &self.moved, &[]);
    }
}

/// Installs the CPKs `cpks` staged in `staging` into `download`, each as `<name>.cpk`, all of
/// them or none: each is copied to `<name>.cpk.partial` first (a copy: the output folder is
/// usually on another volume than the game), then `install_all` moves each old CPK aside and
/// renames the `.partial`s into place: the old CPK moved aside first, so a run can be undone
/// after it is installed. A failure removes the `.partial`s (a failure to remove one is
/// logged) and leaves the old CPKs as they were; the staged CPKs stay, for the output folder.
/// The returned `Installed` commits or rolls the deployment back.
pub(crate) fn deploy(
    staging: &Staging,
    download: &Path,
    cpks: &[CpkStem],
) -> Result<Installed, DeployFailure> {
    let names: Vec<String> = cpks.iter().map(cpk_file_name).collect();
    let partials: Vec<PathBuf> = names
        .iter()
        .map(|name| download.join(format!("{name}.partial")))
        .collect();
    for (index, name) in names.iter().enumerate() {
        if let Err(error) = fs::copy(staging.folder.join(name), &partials[index]) {
            // The failed copy's own `.partial` too: it may hold the copy's first bytes.
            remove_all(&partials[..=index]);
            return Err(DeployFailure::Copy(error));
        }
    }
    install_all(download, cpks, &names, &partials)
}

/// Renames the copied `partials` of the CPKs `cpks` (file `names`) into place in `download`,
/// all of them or none: each old CPK there is moved aside to `<name>.cpk.old`, then each
/// `.partial` renamed to its name. The old CPKs stay aside in the
/// returned `Installed` until the run commits or rolls back. A rename that fails undoes the
/// ones made (`undo`) and is the `Rename` failure naming its CPK.
fn install_all(
    download: &Path,
    cpks: &[CpkStem],
    names: &[String],
    partials: &[PathBuf],
) -> Result<Installed, DeployFailure> {
    let installed: Vec<PathBuf> = names.iter().map(|name| download.join(name)).collect();
    let asides: Vec<PathBuf> = names
        .iter()
        .map(|name| download.join(format!("{name}.old")))
        .collect();
    // A `.old` a killed run left needs no removal: the rename aside replaces it (std's
    // Windows rename replaces an existing file), and this run's commit removes the new one.
    // The old CPKs move aside before any new one goes in: renaming each `.partial` over its
    // old CPK could not be undone once a later rename failed, the old CPKs being gone, while
    // a CPK moved aside can still be moved back.
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    for (index, cpk) in cpks.iter().enumerate() {
        match fs::rename(&installed[index], &asides[index]) {
            Ok(()) => moved.push((asides[index].clone(), installed[index].clone())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                undo(&[], &moved, partials);
                let cpk = cpk.clone();
                return Err(DeployFailure::Rename { cpk, error });
            }
        }
    }
    for (index, cpk) in cpks.iter().enumerate() {
        if let Err(error) = fs::rename(&partials[index], &installed[index]) {
            undo(&installed[..index], &moved, &partials[index..]);
            let cpk = cpk.clone();
            return Err(DeployFailure::Rename { cpk, error });
        }
    }
    Ok(Installed {
        placed: installed,
        moved,
    })
}

/// Puts `download/` back as `install_all` found it, after a rename failed or the run rolled
/// its deployment back: the new CPKs `placed` removed, then each old CPK `moved` aside (its
/// `.old` path, its own) moved back, then the `partials` left removed. A failure is logged,
/// not returned: the failure reported is the one that started the undo.
fn undo(placed: &[PathBuf], moved: &[(PathBuf, PathBuf)], partials: &[PathBuf]) {
    remove_all(placed);
    for (aside, installed) in moved {
        if let Err(error) = fs::rename(aside, installed) {
            log::debug!("{}: not moved back: {error}", aside.display());
        }
    }
    remove_all(partials);
}

/// Removes each of `files`, logging a failure.
fn remove_all(files: &[PathBuf]) {
    for file in files {
        remove_logged(file);
    }
}

/// Removes a file a deployment made and no longer needs; a failure is logged.
fn remove_logged(file: &Path) {
    if let Err(error) = fs::remove_file(file) {
        log::debug!("{}: kept: {error}", file.display());
    }
}

/// Moves the CPK staged in `staging` to `<output>/<cpk_stem>.cpk`, replacing a previous one.
/// The rename stays on one volume, so the final path holds either the previous CPK or the
/// whole new one, never part of it. Returns the promoted path.
pub(crate) fn promote(
    staging: &Staging,
    output: &Path,
    cpk_stem: &CpkStem,
) -> anyhow::Result<PathBuf> {
    let name = cpk_file_name(cpk_stem);
    let promoted = output.join(&name);
    fs::rename(staging.folder.join(&name), &promoted)
        .with_context(|| format!("cannot replace {} with the new CPK", promoted.display()))?;
    Ok(promoted)
}

/// Replaces the folder `target` (the PES folder's `livecpk/`, the output folder's
/// `test_output/`) with the tree staged at `<staging folder>/<tree>`. The previous tree goes
/// first, whatever it holds: Studio is the folder's only writer, and a file the export no longer
/// has must not linger there. The staged tree is renamed into place, or copied when the rename
/// fails (the output folder on another drive than the game). Nothing outside `target` is
/// deleted. Failing to remove the previous tree or to move the new one is the error.
pub(crate) fn promote_tree(staging: &Staging, tree: &str, target: &Path) -> anyhow::Result<()> {
    let cannot_replace = || format!("cannot replace {} with the new files", target.display());
    match fs::remove_dir_all(target) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).with_context(cannot_replace),
    }
    let staged = staging.folder.join(tree);
    if let Err(error) = fs::rename(&staged, target) {
        log::debug!("{}: not renamed ({error}), copied", staged.display());
        copy_tree(&staged, target).with_context(cannot_replace)?;
    }
    Ok(())
}

/// Copies every file under `source` to the same path under `target`, folders created as
/// needed.
fn copy_tree(source: &Path, target: &Path) -> io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let into = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &into)?;
        } else {
            fs::copy(entry.path(), into)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use studio_core::Severity;

    use super::*;
    use crate::testing::{dpfilelist, scratch};

    /// The official list the comparison tests use: three midcups.
    const OFFICIAL: [&str; 3] = [
        "4cc_61_midcup.cpk",
        "4cc_62_midcup.cpk",
        "4cc_63_midcup.cpk",
    ];

    /// Makes `root/PES` a PES 2021 folder whose `download/` holds a `DpFileList.bin` listing
    /// `installed` and a file for each of `files`; returns the PES folder.
    fn pes_with_list(root: &Path, installed: &[&str], files: &[&str]) -> PathBuf {
        let pes = root.join("PES");
        fs::create_dir_all(pes.join(DOWNLOAD)).unwrap();
        fs::write(pes.join("PES2021.exe"), "the game").unwrap();
        fs::write(pes.join(DOWNLOAD).join(DPFILELIST), dpfilelist(installed)).unwrap();
        for file in files {
            fs::write(pes.join(DOWNLOAD).join(file), "a CPK").unwrap();
        }
        pes
    }

    /// `preflight` for a PES 2021 run compiling `cpks` into the PES folder `pes`, against
    /// `OFFICIAL`, promoted to `<pes>/promoted` when it cannot deploy.
    fn preflight_of(pes: &Path, cpks: &[&str]) -> (Option<Download>, Vec<Message>) {
        let official = OFFICIAL.map(str::to_owned);
        let cpks: Vec<CpkStem> = cpks.iter().map(|cpk| CpkStem::new(cpk).unwrap()).collect();
        preflight(
            pes,
            PesVersion::Pes21,
            &cpks,
            &pes.join("promoted"),
            &official,
        )
    }

    /// `preflight` for a PES 2021 run compiling `4cc_61_midcup.cpk` into the PES folder `pes`,
    /// against `OFFICIAL`; it deploys, the findings are only Warnings.
    fn preflight_61(pes: &Path) -> Vec<Message> {
        let (download, messages) = preflight_of(pes, &["4cc_61_midcup"]);
        assert_eq!(
            download.map(|download| download.folder),
            Some(pes.join(DOWNLOAD)),
            "{messages:#?}"
        );
        messages
    }

    /// The `dpfilelist_not_official` for `pes`'s list with `differences`, the context keys
    /// between `path` and `command`.
    fn not_official_finding(pes: &Path, differences: &[(&'static str, &str)]) -> Message {
        let mut context = vec![(
            "path",
            pes.join(DOWNLOAD).join(DPFILELIST).display().to_string(),
        )];
        context.extend(
            differences
                .iter()
                .map(|(key, value)| (*key, (*value).to_owned())),
        );
        context.push(("command", UPGRADE_COMMAND.to_owned()));
        tool_message(
            Code::DpfilelistNotOfficial,
            Scope::Run,
            Disposition::Keep,
            context,
        )
    }

    #[test]
    fn the_official_entries_in_another_order_are_not_official_by_their_order_alone() {
        let temp = scratch("preflight_order");
        let installed = [
            "4cc_61_midcup.cpk",
            "4cc_63_midcup.cpk",
            "4cc_62_midcup.cpk",
        ];
        let pes = pes_with_list(temp.path(), &installed, &installed[1..]);

        let messages = preflight_61(&pes);

        assert_eq!(
            messages,
            [not_official_finding(&pes, &[("order", "differs")])]
        );
        assert_eq!(messages[0].code.code, "dpfilelist_not_official");
        assert_eq!(messages[0].severity, Severity::Warning);
    }

    #[test]
    fn a_list_lacking_an_official_entry_names_it_as_missing_alone() {
        let temp = scratch("preflight_missing");
        let installed = ["4cc_61_midcup.cpk", "4cc_62_midcup.cpk"];
        let pes = pes_with_list(temp.path(), &installed, &installed[1..]);

        let messages = preflight_61(&pes);

        assert_eq!(
            messages,
            [not_official_finding(
                &pes,
                &[("missing", "4cc_63_midcup.cpk")]
            )]
        );
    }

    #[test]
    fn an_entry_the_official_list_lacks_is_unofficial_and_the_shared_entries_order_is_judged_alone()
    {
        let temp = scratch("preflight_unofficial");
        let installed = [
            "4cc_62_midcup.cpk",
            "4cc_61_midcup.cpk",
            "4cc_63_midcup.cpk",
            "4cc_80_mine.cpk",
        ];
        let pes = pes_with_list(
            temp.path(),
            &installed,
            &["4cc_62_midcup.cpk", "4cc_63_midcup.cpk", "4cc_80_mine.cpk"],
        );

        let messages = preflight_61(&pes);

        assert_eq!(
            messages,
            [not_official_finding(
                &pes,
                &[("unofficial", "4cc_80_mine.cpk"), ("order", "differs")]
            )]
        );
    }

    #[test]
    fn an_entry_appended_to_the_official_list_keeps_the_shared_order() {
        let temp = scratch("preflight_appended");
        let installed = [
            "4cc_61_midcup.cpk",
            "4cc_62_midcup.cpk",
            "4cc_63_midcup.cpk",
            "4cc_80_mine.cpk",
        ];
        let pes = pes_with_list(temp.path(), &installed, &installed[1..]);

        let messages = preflight_61(&pes);

        assert_eq!(
            messages,
            [not_official_finding(
                &pes,
                &[("unofficial", "4cc_80_mine.cpk")]
            )]
        );
    }

    #[test]
    fn the_listed_cpks_missing_from_download_are_named_but_not_the_run_s_own() {
        let temp = scratch("preflight_cpk_missing");
        // The run's own `4cc_61_midcup.cpk` is absent too; `4cc_62_midcup.cpk` is there.
        let pes = pes_with_list(temp.path(), &OFFICIAL, &["4cc_62_midcup.cpk"]);

        let messages = preflight_61(&pes);

        assert_eq!(
            messages,
            [tool_message(
                Code::DpfilelistCpkMissing,
                Scope::Run,
                Disposition::Keep,
                vec![
                    ("path", pes.join(DOWNLOAD).display().to_string()),
                    ("files", "4cc_63_midcup.cpk".to_owned()),
                    ("command", UPGRADE_COMMAND.to_owned()),
                ],
            )]
        );
        assert_eq!(messages[0].code.code, "dpfilelist_cpk_missing");
        assert_eq!(messages[0].severity, Severity::Warning);
    }

    #[test]
    fn the_official_list_with_every_other_cpk_present_is_no_finding() {
        let temp = scratch("preflight_official");
        let pes = pes_with_list(temp.path(), &OFFICIAL, &OFFICIAL[1..]);

        assert_eq!(preflight_61(&pes), []);
    }

    /// `dpfilelist_outdated` (`official`) or `cpk_name_unlisted` naming `cpks` for the list of
    /// the PES folder `pes`, as `preflight_of` reports it.
    fn list_error(pes: &Path, official: bool, cpks: &str) -> Message {
        let mut context = vec![
            ("cpk", cpks.to_owned()),
            (
                "path",
                pes.join(DOWNLOAD).join(DPFILELIST).display().to_string(),
            ),
        ];
        let code = if official {
            context.push(upgrade_command());
            Code::DpfilelistOutdated
        } else {
            Code::CpkNameUnlisted
        };
        tool_message(code, Scope::Run, Disposition::AbortRun, context)
    }

    #[test]
    fn the_official_cpks_a_list_lacks_are_one_dpfilelist_outdated_in_the_run_s_order() {
        let temp = scratch("preflight_outdated_joined");
        let pes = pes_with_list(temp.path(), &["4cc_61_midcup.cpk"], &[]);

        let (download, messages) =
            preflight_of(&pes, &["4cc_63_midcup", "4cc_61_midcup", "4cc_62_midcup"]);

        assert!(download.is_none());
        assert_eq!(
            messages,
            [list_error(
                &pes,
                true,
                "4cc_63_midcup.cpk, 4cc_62_midcup.cpk"
            )]
        );
    }

    #[test]
    fn an_official_cpk_and_an_unknown_one_the_list_lacks_are_one_finding_each() {
        let temp = scratch("preflight_outdated_and_unlisted");
        let pes = pes_with_list(temp.path(), &["4cc_61_midcup.cpk"], &[]);

        let (download, messages) =
            preflight_of(&pes, &["4cc_80_mine", "4cc_62_midcup", "4cc_81_mine"]);

        assert!(download.is_none());
        assert_eq!(
            messages,
            [
                list_error(&pes, true, "4cc_62_midcup.cpk"),
                list_error(&pes, false, "4cc_80_mine.cpk, 4cc_81_mine.cpk"),
            ]
        );
    }

    #[test]
    fn no_cpk_of_the_run_is_named_missing_from_download() {
        let temp = scratch("preflight_cpks_missing_several");
        // No CPK in `download/`: the run writes 61 and 62, 63 is missing.
        let pes = pes_with_list(temp.path(), &OFFICIAL, &[]);

        let (download, messages) = preflight_of(&pes, &["4cc_61_midcup", "4cc_62_midcup"]);

        let Some(download) = download else {
            panic!("deploys: {messages:#?}")
        };
        assert_eq!(download.folder, pes.join(DOWNLOAD));
        assert!(
            !download.loaded,
            "a listed CPK absent: the game runs vanilla"
        );
        assert_eq!(
            messages,
            [tool_message(
                Code::DpfilelistCpkMissing,
                Scope::Run,
                Disposition::Keep,
                vec![
                    ("path", pes.join(DOWNLOAD).display().to_string()),
                    ("files", "4cc_63_midcup.cpk".to_owned()),
                    upgrade_command(),
                ],
            )]
        );
    }

    /// A staging under `output` whose folder holds the CPK `cup.cpk` with `bytes`.
    fn staged_cpk(output: &Path, bytes: &[u8]) -> Staging {
        let staging = Staging::create(output).unwrap();
        fs::create_dir_all(staging.folder()).unwrap();
        fs::write(staging.folder().join("cup.cpk"), bytes).unwrap();
        staging
    }

    /// Whether the lock file at `path` is held: a second handle cannot take it.
    fn is_held(path: &Path) -> bool {
        match File::open(path).unwrap().try_lock() {
            Ok(()) => false,
            Err(TryLockError::WouldBlock) => true,
            Err(TryLockError::Error(error)) => panic!("{}: {error}", path.display()),
        }
    }

    #[test]
    fn the_staging_folder_is_per_process_under_the_output_folder() {
        let temp = scratch("staging_folder");
        let output = temp.path();

        let staging = Staging::create(output).unwrap();

        let folder = staging.folder();
        assert_eq!(folder.parent(), Some(output.join(STAGING).as_path()));
        let name = folder.file_name().unwrap().to_str().unwrap();
        let (pid, millis) = name.split_once('-').unwrap();
        assert_eq!(pid, std::process::id().to_string());
        assert!(millis.parse::<u128>().unwrap() > 0, "{name}");
    }

    #[test]
    fn a_run_s_own_lock_is_held_until_its_folder_is_gone() {
        let temp = scratch("staging_own_lock");
        let output = temp.path();
        let staging = staged_cpk(output, b"partial");
        let folder = staging.folder().to_owned();
        let lock = output.join(STAGING).join(format!(
            "{}.lock",
            folder.file_name().unwrap().to_str().unwrap()
        ));
        assert!(is_held(&lock), "held while the run lives");

        drop(staging);

        assert!(!folder.exists());
        assert!(!lock.exists());
        assert!(!output.join(STAGING).exists(), "an emptied .staging goes");
    }

    #[test]
    fn a_staging_that_cannot_be_created_is_an_error_naming_the_folder() {
        let temp = scratch("staging_blocked");
        let output = temp.path();
        fs::write(output.join(STAGING), "in the way").unwrap();

        let Err(error) = Staging::create(output) else {
            panic!("created over a file");
        };

        assert!(
            error.to_string().starts_with(&format!(
                "cannot create the folder {}",
                output.join(STAGING).display()
            )),
            "{error}"
        );
    }

    #[test]
    fn starting_a_run_removes_dead_runs_staging_and_keeps_live_runs() {
        let temp = scratch("staging_sweep");
        let output = temp.path();
        let root = output.join(STAGING);
        for folder in ["1-1", "2-2", "3-3"] {
            fs::create_dir_all(root.join(folder)).unwrap();
            fs::write(root.join(folder).join("x.cpk"), "half a CPK").unwrap();
        }
        // 1-1 has no lock; 2-2's and the lone 4-4's are free; 3-3's and the lone 5-5's are held,
        // the latter by a run that has not created its folder yet.
        for lock in ["2-2.lock", "3-3.lock", "4-4.lock", "5-5.lock"] {
            fs::write(root.join(lock), "").unwrap();
        }
        let held: Vec<File> = ["3-3.lock", "5-5.lock"]
            .iter()
            .map(|lock| {
                let file = File::open(root.join(lock)).unwrap();
                file.try_lock().unwrap();
                file
            })
            .collect();

        let staging = Staging::create(output).unwrap();

        let mut left: Vec<String> = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        let own = format!(
            "{}.lock",
            staging.folder().file_name().unwrap().to_str().unwrap()
        );
        let mut expected = vec![
            "3-3".to_owned(),
            "3-3.lock".to_owned(),
            "5-5.lock".to_owned(),
        ];
        expected.push(own);
        expected.sort();
        assert_eq!(left, expected);
        assert!(root.join("3-3/x.cpk").is_file());
        drop(held);
    }

    #[test]
    fn probing_a_download_folder_that_does_not_exist_fails_naming_the_folder() {
        let temp = scratch("probe_missing_download");
        let download = temp.path().join("download");

        let Err((path, error)) = probe_download(&download, "cup.cpk") else {
            panic!("a folder that does not exist was probed as writable");
        };

        assert_eq!(path, download);
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    // Assumes a user that is not root, as CI's runner and the remote mutation unit are: root
    // opens a read-only file for writing.
    #[test]
    fn a_read_only_old_cpk_fails_the_probe_naming_the_cpk() {
        let temp = scratch("probe_read_only_cpk");
        let download = temp.path().join("download");
        fs::create_dir_all(&download).unwrap();
        let old = download.join("4cc_61_midcup.cpk");
        fs::write(&old, "the old CPK").unwrap();
        let writable = fs::metadata(&old).unwrap().permissions();
        let mut read_only = writable.clone();
        read_only.set_readonly(true);
        fs::set_permissions(&old, read_only).unwrap();

        let probed = probe_download(&download, "4cc_61_midcup.cpk");

        // Writable again, so the scratch folder can be removed.
        fs::set_permissions(&old, writable).unwrap();
        let Err((path, error)) = probed else {
            panic!("a read-only old CPK was probed as writable");
        };
        assert_eq!(path, old);
        assert!(elevation::is_access_denied(&error), "{error}");
    }

    // Windows only: there a folder cannot be opened as a file, the one portable way to make the
    // lock exist and not open; Linux opens and locks a folder.
    #[cfg(windows)]
    #[test]
    fn a_staging_whose_lock_cannot_be_opened_is_left() {
        let temp = scratch("staging_lock_unopenable");
        let output = temp.path();
        let root = output.join(STAGING);
        fs::create_dir_all(root.join("9-9")).unwrap();
        fs::write(root.join("9-9/x.cpk"), "half a CPK").unwrap();
        fs::create_dir_all(root.join("9-9.lock")).unwrap();

        let _staging = Staging::create(output).unwrap();

        assert!(root.join("9-9/x.cpk").is_file());
    }

    #[test]
    fn dropping_a_staging_keeps_the_staging_folder_another_run_still_uses() {
        let temp = scratch("staging_beside_another_run");
        let output = temp.path();
        let staging = staged_cpk(output, b"partial");
        let other_run = output.join(STAGING).join("1-1");
        fs::create_dir_all(&other_run).unwrap();
        let folder = staging.folder().to_owned();

        drop(staging);

        assert!(!folder.exists());
        assert!(other_run.is_dir());
    }

    #[test]
    fn the_output_folder_is_created_and_probed_leaving_nothing_in_it() {
        let temp = scratch("prepare_output");
        let root = temp.path();
        let output = root.join("new").join("output");

        prepare_output_folder(&output).unwrap();

        assert!(output.is_dir());
        assert_eq!(
            fs::read_dir(&output).unwrap().count(),
            0,
            "no probe file is left"
        );
    }

    #[test]
    fn an_output_folder_under_a_file_is_refused_naming_the_folder() {
        let temp = scratch("prepare_output_blocked");
        let root = temp.path();
        fs::write(root.join("blocker"), "").unwrap();
        let output = root.join("blocker").join("out");

        let error = prepare_output_folder(&output).unwrap_err();

        let text = error.to_string();
        assert!(
            text.starts_with(&format!(
                "cannot create the output folder {}",
                output.display()
            )),
            "{text}"
        );
    }

    #[test]
    fn deploying_copies_the_staged_cpk_over_the_old_one_kept_aside_until_commit() {
        let temp = scratch("deploy");
        let output = temp.path().join("output");
        let download = temp.path().join("download");
        fs::create_dir_all(&download).unwrap();
        fs::write(download.join("cup.cpk"), "old").unwrap();
        let staging = staged_cpk(&output, b"new");

        let installed = deploy(&staging, &download, &[CpkStem::new("cup").unwrap()]).unwrap();

        assert_eq!(fs::read(download.join("cup.cpk")).unwrap(), b"new");
        assert_eq!(fs::read(download.join("cup.cpk.old")).unwrap(), b"old");
        assert_eq!(
            fs::read(staging.folder().join("cup.cpk")).unwrap(),
            b"new",
            "a copy: the staged CPK stays until the staging goes"
        );
        installed.commit();
        assert_eq!(
            texts(&download),
            BTreeMap::from([("cup.cpk".to_owned(), "new".to_owned())]),
            "committed: the new CPK alone, no .partial or .old"
        );
    }

    #[test]
    fn a_failing_copy_is_the_unwritable_kind_and_leaves_no_partial() {
        let temp = scratch("deploy_copy_failed");
        let output = temp.path().join("output");
        // No `download/` folder: the copy cannot create the `.partial`.
        let download = temp.path().join("download");
        let staging = staged_cpk(&output, b"new");

        let failure = deploy(&staging, &download, &[CpkStem::new("cup").unwrap()]).unwrap_err();

        assert!(matches!(failure, DeployFailure::Copy(_)), "{failure:?}");
        assert!(!download.exists(), "nothing is written: {failure:?}");
        assert!(staging.folder().join("cup.cpk").is_file());
    }

    #[test]
    fn a_failing_rename_is_the_locked_kind_and_leaves_no_partial() {
        let temp = scratch("deploy_rename_failed");
        let output = temp.path().join("output");
        let download = temp.path().join("download");
        fs::create_dir_all(&download).unwrap();
        fs::write(download.join("cup.cpk"), "old").unwrap();
        // A non-empty folder where the old CPK would be moved aside: nothing can be renamed
        // over it, as nothing can over a `.old` PES holds.
        fs::create_dir_all(download.join("cup.cpk.old")).unwrap();
        fs::write(download.join("cup.cpk.old/kept.txt"), "kept").unwrap();
        let staging = staged_cpk(&output, b"new");

        let failure = deploy(&staging, &download, &[CpkStem::new("cup").unwrap()]).unwrap_err();

        assert!(
            matches!(&failure, DeployFailure::Rename { cpk, .. } if cpk.as_str() == "cup"),
            "{failure:?}"
        );
        assert_eq!(fs::read(download.join("cup.cpk")).unwrap(), b"old");
        assert!(!download.join("cup.cpk.partial").exists());
        assert_eq!(
            fs::read(download.join("cup.cpk.old/kept.txt")).unwrap(),
            b"kept"
        );
    }

    /// A staging under `output` whose folder holds the CPK `<name>.cpk` of each of `names`,
    /// its bytes `new <name>`.
    fn staged_cpks(output: &Path, names: &[&str]) -> Staging {
        let staging = Staging::create(output).unwrap();
        fs::create_dir_all(staging.folder()).unwrap();
        for name in names {
            fs::write(
                staging.folder().join(format!("{name}.cpk")),
                format!("new {name}"),
            )
            .unwrap();
        }
        staging
    }

    /// The CPK stems `names`.
    fn stems(names: &[&str]) -> Vec<CpkStem> {
        names
            .iter()
            .map(|name| CpkStem::new(name).unwrap())
            .collect()
    }

    /// Every file and folder in `folder`, by name, with a file's bytes (a folder's are empty).
    fn contents(folder: &Path) -> BTreeMap<String, Vec<u8>> {
        fs::read_dir(folder)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let bytes = if entry.file_type().unwrap().is_dir() {
                    Vec::new()
                } else {
                    fs::read(entry.path()).unwrap()
                };
                (entry.file_name().into_string().unwrap(), bytes)
            })
            .collect()
    }

    /// `contents` with each file's bytes as text.
    fn texts(folder: &Path) -> BTreeMap<String, String> {
        contents(folder)
            .into_iter()
            .map(|(name, bytes)| (name, String::from_utf8(bytes).unwrap()))
            .collect()
    }

    #[test]
    fn deploying_several_cpks_installs_each_and_leaves_no_partial_or_old() {
        let temp = scratch("deploy_several");
        let output = temp.path().join("output");
        let download = temp.path().join("download");
        fs::create_dir_all(&download).unwrap();
        fs::write(download.join("bins.cpk"), "old bins").unwrap();
        fs::write(download.join("other.cpk"), "not the run's").unwrap();
        let staging = staged_cpks(&output, &["bins", "part"]);

        deploy(&staging, &download, &stems(&["bins", "part"]))
            .unwrap()
            .commit();

        assert_eq!(
            texts(&download),
            BTreeMap::from([
                ("bins.cpk".to_owned(), "new bins".to_owned()),
                ("other.cpk".to_owned(), "not the run's".to_owned()),
                ("part.cpk".to_owned(), "new part".to_owned()),
            ])
        );
    }

    #[test]
    fn a_rolled_back_install_restores_the_old_cpk_and_removes_the_new_one() {
        let temp = scratch("deploy_rollback");
        let output = temp.path().join("output");
        let download = temp.path().join("download");
        fs::create_dir_all(&download).unwrap();
        fs::write(download.join("cup.cpk"), "old").unwrap();
        let staging = staged_cpk(&output, b"new");

        deploy(&staging, &download, &[CpkStem::new("cup").unwrap()])
            .unwrap()
            .roll_back();

        assert_eq!(
            texts(&download),
            BTreeMap::from([("cup.cpk".to_owned(), "old".to_owned())]),
            "the old CPK back, alone"
        );
    }

    #[test]
    fn a_rolled_back_install_of_several_restores_every_old_cpk() {
        let temp = scratch("deploy_rollback_several");
        let output = temp.path().join("output");
        let download = temp.path().join("download");
        fs::create_dir_all(&download).unwrap();
        fs::write(download.join("bins.cpk"), "old bins").unwrap();
        fs::write(download.join("part.cpk"), "old part").unwrap();
        fs::write(download.join("other.cpk"), "not the run's").unwrap();
        let staging = staged_cpks(&output, &["bins", "fresh", "part"]);

        deploy(&staging, &download, &stems(&["bins", "fresh", "part"]))
            .unwrap()
            .roll_back();

        // `fresh` had no old CPK: it is removed, the two moved ones put back.
        assert_eq!(
            texts(&download),
            BTreeMap::from([
                ("bins.cpk".to_owned(), "old bins".to_owned()),
                ("other.cpk".to_owned(), "not the run's".to_owned()),
                ("part.cpk".to_owned(), "old part".to_owned()),
            ])
        );
    }

    #[test]
    fn a_committed_install_removes_the_old_cpks_kept_aside() {
        let temp = scratch("deploy_commit_several");
        let output = temp.path().join("output");
        let download = temp.path().join("download");
        fs::create_dir_all(&download).unwrap();
        fs::write(download.join("bins.cpk"), "old bins").unwrap();
        fs::write(download.join("part.cpk"), "old part").unwrap();
        let staging = staged_cpks(&output, &["bins", "part"]);

        deploy(&staging, &download, &stems(&["bins", "part"]))
            .unwrap()
            .commit();

        assert_eq!(
            texts(&download),
            BTreeMap::from([
                ("bins.cpk".to_owned(), "new bins".to_owned()),
                ("part.cpk".to_owned(), "new part".to_owned()),
            ])
        );
    }

    #[test]
    fn an_old_cpk_that_cannot_be_moved_aside_moves_back_those_moved_and_installs_none() {
        let temp = scratch("deploy_several_aside_failed");
        let output = temp.path().join("output");
        let download = temp.path().join("download");
        fs::create_dir_all(&download).unwrap();
        for name in ["bins", "part", "last"] {
            fs::write(download.join(format!("{name}.cpk")), format!("old {name}")).unwrap();
        }
        // A non-empty folder where `part.cpk` would be moved aside: nothing can be renamed
        // over it, as nothing can over a CPK PES holds.
        fs::create_dir_all(download.join("part.cpk.old")).unwrap();
        fs::write(download.join("part.cpk.old/kept.txt"), "kept").unwrap();
        let before = contents(&download);
        let staging = staged_cpks(&output, &["bins", "part", "last"]);

        let failure = deploy(&staging, &download, &stems(&["bins", "part", "last"])).unwrap_err();

        assert!(
            matches!(&failure, DeployFailure::Rename { cpk, .. } if cpk.as_str() == "part"),
            "{failure:?}"
        );
        assert_eq!(contents(&download), before);
        assert_eq!(
            fs::read(download.join("part.cpk.old/kept.txt")).unwrap(),
            b"kept"
        );
    }

    // The rename into place fails only when something takes a name step 2 freed, which no
    // file-system state can do between two steps of one run. A run naming one CPK twice
    // reaches it with real files: the second copy replaces the first `.partial`, the second
    // move aside finds no old CPK, and the second rename into place finds no `.partial`.
    #[test]
    fn a_partial_that_cannot_be_renamed_into_place_undoes_the_install_whole() {
        let temp = scratch("deploy_several_into_place_failed");
        let output = temp.path().join("output");
        let download = temp.path().join("download");
        fs::create_dir_all(&download).unwrap();
        fs::write(download.join("twice.cpk"), "old twice").unwrap();
        let before = contents(&download);
        let staging = staged_cpks(&output, &["fresh", "twice"]);

        // `fresh` has no old CPK: only removing it puts the folder back.
        let failure =
            deploy(&staging, &download, &stems(&["fresh", "twice", "twice"])).unwrap_err();

        assert!(
            matches!(&failure, DeployFailure::Rename { cpk, error }
                if cpk.as_str() == "twice" && error.kind() == io::ErrorKind::NotFound),
            "{failure:?}"
        );
        assert_eq!(contents(&download), before);
    }

    #[test]
    fn a_failing_copy_of_one_of_several_cpks_removes_the_partials_made() {
        let temp = scratch("deploy_several_copy_failed");
        let output = temp.path().join("output");
        let download = temp.path().join("download");
        fs::create_dir_all(&download).unwrap();
        fs::write(download.join("bins.cpk"), "old bins").unwrap();
        let before = contents(&download);
        // `part` is not staged: its copy fails after `bins.cpk.partial` is made.
        let staging = staged_cpks(&output, &["bins", "last"]);

        let failure = deploy(&staging, &download, &stems(&["bins", "part", "last"])).unwrap_err();

        assert!(matches!(failure, DeployFailure::Copy(_)), "{failure:?}");
        assert_eq!(contents(&download), before);
    }

    #[test]
    fn promotion_replaces_the_previous_cpk_and_the_staging_goes_with_the_run() {
        let temp = scratch("promote");
        let output = temp.path();
        let stem = CpkStem::new("cup").unwrap();
        let staging = staged_cpk(output, b"new");
        fs::write(output.join("cup.cpk"), "previous").unwrap();

        let promoted = promote(&staging, output, &stem).unwrap();
        drop(staging);

        assert_eq!(promoted, output.join("cup.cpk"));
        assert_eq!(fs::read(&promoted).unwrap(), b"new");
        assert!(!output.join(STAGING).exists());
    }

    /// A staging under `output` holding a staged `livecpk/` tree of two files, one nested.
    fn staged_tree(output: &Path) -> Staging {
        let staging = Staging::create(output).unwrap();
        let staged = staging.folder().join(LIVECPK);
        fs::create_dir_all(staged.join("common/etc")).unwrap();
        fs::write(staged.join("common/etc/TeamColor.bin"), "colors").unwrap();
        fs::write(staged.join("top.bin"), "top").unwrap();
        staging
    }

    /// Asserts `livecpk` holds exactly `staged_tree`'s two files.
    fn assert_staged_tree(livecpk: &Path) {
        assert_eq!(
            fs::read(livecpk.join("common/etc/TeamColor.bin")).unwrap(),
            b"colors"
        );
        assert_eq!(fs::read(livecpk.join("top.bin")).unwrap(), b"top");
        assert_eq!(fs::read_dir(livecpk).unwrap().count(), 2);
    }

    #[test]
    fn the_staged_tree_replaces_livecpk_s_whole_contents_and_the_run_folder_goes() {
        let temp = scratch("promote_livecpk");
        let output = temp.path().join("output");
        let pes_folder = temp.path().join("PES");
        fs::create_dir_all(pes_folder.join("livecpk/old")).unwrap();
        fs::write(pes_folder.join("livecpk/old/old.txt"), "an earlier run").unwrap();
        fs::write(pes_folder.join("livecpk/top.bin"), "an earlier top").unwrap();
        fs::write(pes_folder.join("beside.txt"), "not ours").unwrap();
        let staging = staged_tree(&output);

        promote_tree(&staging, LIVECPK, &pes_folder.join(LIVECPK)).unwrap();
        drop(staging);

        assert_staged_tree(&pes_folder.join("livecpk"));
        assert_eq!(
            fs::read(pes_folder.join("beside.txt")).unwrap(),
            b"not ours"
        );
        assert!(!output.join(STAGING).exists());
    }

    #[test]
    fn a_missing_livecpk_is_created_from_the_staged_tree() {
        let temp = scratch("promote_livecpk_new");
        let output = temp.path().join("output");
        let pes_folder = temp.path().join("PES");
        fs::create_dir_all(&pes_folder).unwrap();
        let staging = staged_tree(&output);

        promote_tree(&staging, LIVECPK, &pes_folder.join(LIVECPK)).unwrap();

        assert_staged_tree(&pes_folder.join("livecpk"));
    }

    #[test]
    fn a_livecpk_that_cannot_be_removed_fails_the_promotion_naming_it() {
        let temp = scratch("promote_livecpk_blocked");
        let output = temp.path().join("output");
        let pes_folder = temp.path().join("PES");
        fs::create_dir_all(&pes_folder).unwrap();
        // A file named `livecpk` is no folder to remove.
        fs::write(pes_folder.join("livecpk"), "a file").unwrap();
        let staging = staged_tree(&output);

        let error = promote_tree(&staging, LIVECPK, &pes_folder.join(LIVECPK)).unwrap_err();

        assert_eq!(
            error.to_string(),
            format!(
                "cannot replace {} with the new files",
                pes_folder.join("livecpk").display()
            )
        );
        assert_eq!(fs::read(pes_folder.join("livecpk")).unwrap(), b"a file");
    }

    #[cfg(windows)]
    #[test]
    fn a_staged_tree_that_cannot_be_renamed_is_copied_into_place() {
        use std::os::windows::fs::OpenOptionsExt;
        /// `FILE_SHARE_READ` alone: the file may be read, so copied, but its folder not moved.
        const FILE_SHARE_READ: u32 = 1;

        let temp = scratch("promote_livecpk_copied");
        let output = temp.path().join("output");
        let pes_folder = temp.path().join("PES");
        fs::create_dir_all(&pes_folder).unwrap();
        let staging = staged_tree(&output);
        let held_open = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(staging.folder().join("livecpk/top.bin"))
            .unwrap();

        promote_tree(&staging, LIVECPK, &pes_folder.join(LIVECPK)).unwrap();

        drop(held_open);
        assert_staged_tree(&pes_folder.join("livecpk"));
    }

    #[cfg(windows)]
    #[test]
    fn a_previous_tree_that_cannot_be_removed_fails_the_promotion_and_is_not_overlaid() {
        use std::os::windows::fs::OpenOptionsExt;
        /// `FILE_SHARE_READ` alone: the file may be read but not deleted while it is open.
        const FILE_SHARE_READ: u32 = 1;

        let temp = scratch("promote_livecpk_held");
        let output = temp.path().join("output");
        let pes_folder = temp.path().join("PES");
        fs::create_dir_all(pes_folder.join("livecpk")).unwrap();
        fs::write(pes_folder.join("livecpk/stale.txt"), "an earlier run").unwrap();
        let staging = staged_tree(&output);
        // A runtime reading the previous tree holds one of its files open.
        let held_open = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(pes_folder.join("livecpk/stale.txt"))
            .unwrap();

        let error = promote_tree(&staging, LIVECPK, &pes_folder.join(LIVECPK)).unwrap_err();

        drop(held_open);
        assert_eq!(
            error.to_string(),
            format!(
                "cannot replace {} with the new files",
                pes_folder.join("livecpk").display()
            )
        );
        // The new tree was not copied over what is left of the old one.
        assert!(!pes_folder.join("livecpk/top.bin").exists());
    }

    #[test]
    fn copying_a_tree_copies_every_file_at_its_path() {
        let temp = scratch("copy_tree");
        let staging = staged_tree(temp.path());
        let target = temp.path().join("copy");

        copy_tree(&staging.folder().join(LIVECPK), &target).unwrap();

        assert_staged_tree(&target);
    }
}
