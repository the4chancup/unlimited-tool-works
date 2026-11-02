//! Which copy of the program this process is, for the files a running copy owns. Two
//! windows may run over one data location, each on its own save, so a copy claims a
//! numbered slot at start by locking the slot's file: the first holds `utw.lock`, a copy
//! that finds it held takes the first free of `utw.2.lock`, `utw.3.lock`…, and the slot
//! picks the log, old log and dump names so the second copy never rotates or reads the
//! first's live `utw.log` (ST-RUN-12). A lock, not a look at the running copies: the
//! system frees it however the process ends, a crash included, so the next start takes
//! slot 1 again.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;

/// The most copies that may hold a slot at once; a copy started past it logs nowhere,
/// which the log install already reports as "no log this run". Sixteen windows is far
/// past real use, and a bounded search keeps one copy from walking lock names without
/// end where something else holds `utw.*.lock` files open.
const MAX_INSTANCES: u32 = 16;

/// `utw.lock`'s name in the data location, slot 1's lock file.
pub(crate) const LOCK_FILE_NAME: &str = "utw.lock";

/// `name` (a data-location file of slot 1, `utw.log`, `utw.old.log`, `utw.dmp`,
/// `utw.lock`) as slot `slot`'s: slot 1's is `name` itself, slot n's has `.n` after the
/// first part (`utw.2.log`, `utw.2.old.log`, `utw.2.dmp`, `utw.2.lock`).
pub(crate) fn slot_file_name(name: &str, slot: u32) -> String {
    match slot {
        1 => name.to_string(),
        n => {
            // Every caller passes one of the four `utw.<rest>` names above, each of
            // which spells a `.`; a name without one cannot reach this point.
            let (stem, rest) = name
                .split_once('.')
                .expect("every caller passes a `utw.<rest>` name");
            format!("{stem}.{n}.{rest}")
        }
    }
}

/// A slot this copy holds until the process ends: the `File` stays open so the lock
/// stays taken, freeing only when the process closes it or dies.
pub(crate) struct Instance {
    slot: u32,
    _lock: File,
}

impl Instance {
    /// This copy's slot number: the first copy over a data location is 1.
    pub(crate) fn slot(&self) -> u32 {
        self.slot
    }
}

/// Takes the first free slot's lock in `data_dir`, `None` when every slot is held or
/// the folder will not open a lock file (a read-only location the log install already
/// reports as having no log). A file that cannot be opened is tried past, since the
/// failure can be that file's alone; a lock error that is not `WouldBlock` means
/// locking itself is not working and further slots would fail the same way.
pub(crate) fn claim(data_dir: &Path) -> Option<Instance> {
    for slot in 1..=MAX_INSTANCES {
        let Ok(file) = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(data_dir.join(slot_file_name(LOCK_FILE_NAME, slot)))
        else {
            continue;
        };
        match file.try_lock() {
            Ok(()) => {
                return Some(Instance { slot, _lock: file });
            }
            Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Error(_)) => return None,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_dir::TestDir;

    /// Holds `data_dir`'s `slot`'s lock the way another copy would, for as long as the
    /// returned `File` is alive.
    fn hold(data_dir: &Path, slot: u32) -> File {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(data_dir.join(slot_file_name(LOCK_FILE_NAME, slot)))
            .unwrap();
        file.try_lock().unwrap();
        file
    }

    // With no copy running, the claim takes slot 1's `utw.lock`.
    // ST-RUN-12
    #[test]
    fn the_first_copy_takes_slot_1() {
        let dir = TestDir::new("claim_first");
        let instance = claim(dir.path()).unwrap();
        assert_eq!(instance.slot(), 1);
        // The claim really holds the lock: a second file on `utw.lock` cannot lock it.
        let probe = OpenOptions::new()
            .read(true)
            .write(true)
            .open(dir.path().join(LOCK_FILE_NAME))
            .unwrap();
        assert!(matches!(probe.try_lock(), Err(TryLockError::WouldBlock)));
    }

    // A second copy skips held slots, taking the first free one.
    // ST-RUN-12
    #[test]
    fn a_second_copy_takes_the_first_free_slot() {
        let dir = TestDir::new("claim_second");
        let _first = hold(dir.path(), 1);
        let instance = claim(dir.path()).unwrap();
        assert_eq!(instance.slot(), 2);
        drop(instance);
        let _third = hold(dir.path(), 3);
        assert_eq!(claim(dir.path()).unwrap().slot(), 2);
    }

    // A lock the system freed when a copy ended is simply free, so the next
    // start takes slot 1 again.
    // ST-RUN-12
    #[test]
    fn a_freed_slot_is_taken_again() {
        let dir = TestDir::new("claim_freed");
        drop(claim(dir.path()).unwrap());
        let _first = hold(dir.path(), 1);
        assert_eq!(claim(dir.path()).unwrap().slot(), 2);
        drop(_first);
        assert_eq!(claim(dir.path()).unwrap().slot(), 1);
    }

    // The slot's `.n` goes after the name's first part, so `utw.old.log`
    // becomes `utw.2.old.log`, not `utw.old.2.log`.
    // ST-RUN-12
    #[test]
    fn slot_file_name_inserts_the_slot_after_the_first_part() {
        for name in [
            crate::gui_log::LOG_FILE_NAME,
            "utw.old.log",
            crate::crash_dump::DUMP_FILE_NAME,
            LOCK_FILE_NAME,
        ] {
            assert_eq!(slot_file_name(name, 1), name);
        }
        assert_eq!(slot_file_name("utw.log", 2), "utw.2.log");
        assert_eq!(slot_file_name("utw.old.log", 2), "utw.2.old.log");
        assert_eq!(slot_file_name("utw.dmp", 2), "utw.2.dmp");
        assert_eq!(slot_file_name("utw.lock", 2), "utw.2.lock");
        assert_eq!(slot_file_name("utw.log", 12), "utw.12.log");
    }

    // With every slot held the claim comes back empty, which the log
    // install reports as no log this run.
    // ST-RUN-12
    #[test]
    fn past_the_cap_no_slot_is_claimed() {
        let dir = TestDir::new("claim_cap");
        let held: Vec<File> = (1..=MAX_INSTANCES)
            .map(|slot| hold(dir.path(), slot))
            .collect();
        assert!(claim(dir.path()).is_none());
        drop(held);
        assert_eq!(claim(dir.path()).unwrap().slot(), 1);
    }
}
