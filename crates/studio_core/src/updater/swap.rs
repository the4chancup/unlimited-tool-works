//! Putting an unpacked release in place of the running one: the running
//! program moves into `old/`, the new one takes its place, and every other file
//! of the new bundle goes over the old one, `data/` aside. Every move is a
//! rename inside the program's folder, recorded as it happens, so a failure
//! midway renames everything back.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::install::InstallError;

/// The folder beside the program that keeps the previous version's program
/// after an update, one version deep.
pub(super) const OLD_FOLDER: &str = "old";

/// The bundle's folder of member data (the settings file, the teams list). An
/// update never reads, writes or removes anything under it, although the
/// archive carries a default settings file there.
const DATA_FOLDER: &str = "data";

/// The program's file name in the bundle: `UTW.exe` on Windows, `utw`
/// elsewhere.
pub(super) fn exe_file_name() -> String {
    format!("{}{}", crate::EXE_NAME, std::env::consts::EXE_SUFFIX)
}

/// The step of the swap that failed, for the member-readable error. Paths are
/// relative to the program's folder.
#[derive(Debug)]
pub(crate) enum SwapStep {
    /// Creating a folder the new bundle has and the old one lacks.
    MakeFolder(PathBuf),
    /// Moving an old file out of the way of its replacement.
    SetAside(PathBuf),
    /// Moving the running program into `old/`.
    RetireProgram,
    /// Moving a new file into its place.
    PlaceFile(PathBuf),
}

impl fmt::Display for SwapStep {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SwapStep::MakeFolder(path) => {
                write!(formatter, "creating the folder {}", path.display())
            }
            SwapStep::SetAside(path) => {
                write!(formatter, "moving the old {} aside", path.display())
            }
            SwapStep::RetireProgram => write!(
                formatter,
                "moving {} into the {OLD_FOLDER} folder",
                exe_file_name()
            ),
            SwapStep::PlaceFile(path) => {
                write!(formatter, "putting the new {} in place", path.display())
            }
        }
    }
}

/// What the swap lists of the unpacked bundle before anything moves: its
/// folder, its subfolders and its files other than the program, relative to
/// the folder, parents before children, `data/` left out.
pub(super) struct StagedBundle {
    /// The unpacked bundle's own folder.
    pub(super) root: PathBuf,
    /// Its subfolders, so an empty one (`exports/`) reaches the program's folder too.
    folders: Vec<PathBuf>,
    /// Its files, the program aside: the program moves first, through `old/`.
    files: Vec<PathBuf>,
}

impl StagedBundle {
    /// Lists the bundle unpacked at `root`, in name order.
    pub(super) fn list(root: PathBuf) -> io::Result<StagedBundle> {
        let mut staged = StagedBundle {
            root,
            folders: Vec::new(),
            files: Vec::new(),
        };
        staged.walk(Path::new(""))?;
        let exe = PathBuf::from(exe_file_name());
        staged.files.retain(|file| *file != exe);
        Ok(staged)
    }

    /// Adds the folders and files under `relative` to the lists.
    fn walk(&mut self, relative: &Path) -> io::Result<()> {
        let mut entries = fs::read_dir(self.root.join(relative))?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = relative.join(entry.file_name());
            if path == Path::new(DATA_FOLDER) {
                continue;
            }
            if entry.file_type()?.is_dir() {
                self.folders.push(path.clone());
                self.walk(&path)?;
            } else {
                self.files.push(path);
            }
        }
        Ok(())
    }
}

/// One move already made, as what undoes it.
enum Undo {
    /// `moved` goes back to `back_to`.
    Rename { moved: PathBuf, back_to: PathBuf },
    /// A folder the swap created, empty again once the later moves are undone.
    RemoveFolder(PathBuf),
}

/// The moves made so far, kept to undo them in reverse order.
struct Swap<'a> {
    /// The program's folder.
    bundle: &'a Path,
    /// Where replaced files wait until the swap is done, inside the temp folder.
    backup: &'a Path,
    /// What undoes each move made so far, in order.
    journal: Vec<Undo>,
}

impl Swap<'_> {
    /// Creates `relative` in the program's folder unless it is already there.
    /// A file in its place fails the step.
    fn make_folder(&mut self, relative: &Path) -> Result<(), (SwapStep, io::Error)> {
        let path = self.bundle.join(relative);
        if path.is_dir() {
            return Ok(());
        }
        fs::create_dir(&path)
            .map_err(|error| (SwapStep::MakeFolder(relative.to_owned()), error))?;
        self.journal.push(Undo::RemoveFolder(path));
        Ok(())
    }

    /// Renames `from` to `to`, recording how to rename it back.
    fn rename(
        &mut self,
        from: PathBuf,
        to: PathBuf,
        step: SwapStep,
    ) -> Result<(), (SwapStep, io::Error)> {
        fs::rename(&from, &to).map_err(|error| (step, error))?;
        self.journal.push(Undo::Rename {
            moved: to,
            back_to: from,
        });
        Ok(())
    }

    /// Moves the file at `relative` in the program's folder into the backup, so
    /// its replacement can take the name. Only a file is moved: a folder in the
    /// way stays, and the placing that follows fails on it.
    fn set_aside(&mut self, relative: &Path) -> Result<(), (SwapStep, io::Error)> {
        let path = self.bundle.join(relative);
        if !path.is_file() {
            return Ok(());
        }
        let aside = self.backup.join(relative);
        let step = || SwapStep::SetAside(relative.to_owned());
        if let Some(parent) = aside.parent() {
            fs::create_dir_all(parent).map_err(|error| (step(), error))?;
        }
        self.rename(path, aside, step())
    }

    /// Every move of the swap, in order, stopping at the first that fails.
    fn apply(&mut self, staged: &StagedBundle) -> Result<(), (SwapStep, io::Error)> {
        let exe = PathBuf::from(exe_file_name());
        let old_exe = Path::new(OLD_FOLDER).join(&exe);
        self.make_folder(Path::new(OLD_FOLDER))?;
        self.set_aside(&old_exe)?;
        self.rename(
            self.bundle.join(&exe),
            self.bundle.join(&old_exe),
            SwapStep::RetireProgram,
        )?;
        self.rename(
            staged.root.join(&exe),
            self.bundle.join(&exe),
            SwapStep::PlaceFile(exe),
        )?;
        for folder in &staged.folders {
            self.make_folder(folder)?;
        }
        for file in &staged.files {
            self.set_aside(file)?;
            self.rename(
                staged.root.join(file),
                self.bundle.join(file),
                SwapStep::PlaceFile(file.clone()),
            )?;
        }
        Ok(())
    }

    /// Undoes every recorded move, newest first. A move that cannot be undone
    /// does not stop the others; the first such failure is returned.
    fn undo(self) -> io::Result<()> {
        let mut first_failure = None;
        for undo in self.journal.into_iter().rev() {
            let result = match &undo {
                Undo::Rename { moved, back_to } => fs::rename(moved, back_to),
                Undo::RemoveFolder(path) => fs::remove_dir(path),
            };
            if let Err(error) = result {
                log::debug!("an update's undo failed: {error}");
                first_failure.get_or_insert(error);
            }
        }
        first_failure.map_or(Ok(()), Err)
    }

    /// Ends the swap on `applied`, what `apply` returned: done, or every move
    /// undone and the failed step named. When an undo fails too, the files set
    /// aside are still in the backup folder, and the error names it.
    fn finish(self, applied: Result<(), (SwapStep, io::Error)>) -> Result<(), InstallError> {
        let (step, source) = match applied {
            Ok(()) => return Ok(()),
            Err(failure) => failure,
        };
        let kept = self.backup.to_owned();
        match self.undo() {
            Ok(()) => Err(InstallError::Swap { step, source }),
            Err(restore) => Err(InstallError::NotRestored {
                step,
                source,
                restore,
                kept,
            }),
        }
    }
}

/// Swaps the bundle unpacked at `staged` into the program's folder `bundle`,
/// replaced files waiting in `backup` (on the same volume, so every move is a
/// rename) until the swap is done. On a failure every move already made is
/// undone, and the error names the step that failed.
pub(super) fn swap(
    staged: &StagedBundle,
    bundle: &Path,
    backup: &Path,
) -> Result<(), InstallError> {
    let mut swap = Swap {
        bundle,
        backup,
        journal: Vec::new(),
    };
    let applied = swap.apply(staged);
    swap.finish(applied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::test_harness::TestDir;
    use crate::updater::test_bundle::folder_with;

    /// A program folder `name` holding a readme and a launcher, and the
    /// backup folder a swap there sets files aside in.
    fn scene(name: &str) -> (TestDir, PathBuf) {
        let dir = folder_with(
            name,
            &vec![
                ("README.txt".to_owned(), "old readme"),
                ("quick_compile.bat".to_owned(), "old launcher"),
            ],
        );
        let backup = dir.path().join(".update").join("backup");
        (dir, backup)
    }

    /// A swap of `bundle` that has set the readme, then the launcher, aside.
    fn both_set_aside<'a>(bundle: &'a Path, backup: &'a Path) -> Swap<'a> {
        let mut swap = Swap {
            bundle,
            backup,
            journal: Vec::new(),
        };
        swap.set_aside(Path::new("README.txt")).unwrap();
        swap.set_aside(Path::new("quick_compile.bat")).unwrap();
        swap
    }

    /// The text of the file `name` in `bundle`, `None` when it is not there.
    fn text(bundle: &Path, name: &str) -> Option<String> {
        fs::read_to_string(bundle.join(name)).ok()
    }

    /// A failure of the step putting the new launcher in place.
    fn launcher_failed() -> Result<(), (SwapStep, io::Error)> {
        Err((
            SwapStep::PlaceFile(PathBuf::from("quick_compile.bat")),
            io::Error::other("the launcher's move failed"),
        ))
    }

    #[test]
    fn an_undo_that_fails_still_undoes_the_older_moves() {
        let (dir, backup) = scene("undo_fails");
        let bundle = dir.path();
        let swap = both_set_aside(bundle, &backup);
        // The newest move's file is gone, so its undo fails.
        fs::remove_file(backup.join("quick_compile.bat")).unwrap();

        let outcome = swap.undo();
        assert!(
            outcome
                .as_ref()
                .is_err_and(|error| error.kind() == io::ErrorKind::NotFound),
            "{outcome:?}"
        );
        assert_eq!(
            text(bundle, "README.txt").as_deref(),
            Some("old readme"),
            "the older move is undone"
        );
    }

    #[test]
    fn a_failed_step_whose_undo_fails_too_names_the_backup_folder() {
        let (dir, backup) = scene("not_restored");
        let bundle = dir.path();
        let swap = both_set_aside(bundle, &backup);
        fs::remove_file(backup.join("quick_compile.bat")).unwrap();

        let outcome = swap.finish(launcher_failed());
        assert!(
            matches!(
                &outcome,
                Err(InstallError::NotRestored { step: SwapStep::PlaceFile(path), kept, .. })
                    if *path == Path::new("quick_compile.bat") && *kept == backup
            ),
            "{outcome:?}"
        );
        assert_eq!(
            text(bundle, "README.txt").as_deref(),
            Some("old readme"),
            "the older move is undone"
        );
    }

    #[test]
    fn a_failed_step_whose_undo_succeeds_puts_every_file_back() {
        let (dir, backup) = scene("restored");
        let bundle = dir.path();
        let swap = both_set_aside(bundle, &backup);

        let outcome = swap.finish(launcher_failed());
        assert!(
            matches!(
                &outcome,
                Err(InstallError::Swap { step: SwapStep::PlaceFile(path), .. })
                    if *path == Path::new("quick_compile.bat")
            ),
            "{outcome:?}"
        );
        assert_eq!(text(bundle, "README.txt").as_deref(), Some("old readme"));
        assert_eq!(
            text(bundle, "quick_compile.bat").as_deref(),
            Some("old launcher")
        );
    }
}
