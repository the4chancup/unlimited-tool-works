//! Tests' scratch folders: [`fresh`] makes each one empty, and it belongs to the thread that
//! made it, removed when that thread ends without a panic and kept for reading when it
//! panics.
//!
//! A guard returned by `fresh` cannot remove the folder: helpers hand their test a path inside
//! it, and the guard would drop when the helper returned, deleting the folder before the test
//! used the path. The folders are therefore tied to the thread libtest gives each test.

use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Once;

std::thread_local! {
    static PANICKED: Cell<bool> = const { Cell::new(false) };
    static FOLDERS: ScratchFolders = const { ScratchFolders(RefCell::new(Vec::new())) };
}

struct ScratchFolders(RefCell<Vec<PathBuf>>);

impl Drop for ScratchFolders {
    fn drop(&mut self) {
        if PANICKED.with(Cell::get) {
            return;
        }
        for folder in self.0.get_mut().drain(..) {
            // Best effort: a panic in a thread-local destructor aborts the test process, so a
            // folder that cannot be removed (a handle still open on Windows) is only logged.
            if let Err(error) = fs::remove_dir_all(&folder) {
                log::debug!("{}: not removed: {error}", folder.display());
            }
        }
    }
}

/// `parent/name` as a fresh, empty folder, removed when the current thread ends without a
/// panic. When the thread panics, the folder is kept so the test's files can be read, and
/// the next `fresh` of the same path empties it.
///
/// `name` is one plain folder name, and must be unique among the tests that share `parent`:
/// two tests running at once under one name empty each other's folder.
///
/// Panics when the folder cannot be removed or created, or when `name` holds a path separator.
pub fn fresh(parent: &Path, name: &str) -> PathBuf {
    // A separator would nest the folder, and only the innermost one is removed.
    assert!(
        !name.contains(['/', '\\']),
        "a scratch name holds no path separator: {name}"
    );
    install_panic_hook();
    let dir = parent.join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir)
            .unwrap_or_else(|error| panic!("{}: not removable: {error}", dir.display()));
    }
    fs::create_dir_all(&dir)
        .unwrap_or_else(|error| panic!("{}: not creatable: {error}", dir.display()));
    FOLDERS.with(|folders| folders.0.borrow_mut().push(dir.clone()));
    dir
}

/// The running test binary's build folder, `target/<profile>`: per checkout, so a mutation
/// run's copy has its own.
pub fn build_folder() -> PathBuf {
    // `CARGO_TARGET_TMPDIR` is set for integration tests only, so the build folder comes from
    // the test binary's path (`target/<profile>/deps/<exe>`).
    let exe = std::env::current_exe().expect("current_exe is readable");
    let deps = exe
        .parent()
        .expect("the test binary sits under target/<profile>/deps");
    deps.parent()
        .expect("deps sits under target/<profile>")
        .to_path_buf()
}

/// Installs, once per process, the panic hook that tells [`fresh`] a thread panicked; the
/// hook it replaces still runs, so the panic prints. `fresh` calls it, and so does a test
/// that swaps the process's panic hook, before its swap: installed inside the swap, the hook
/// would be lost when the swap restores the old one, and every later panicking test's folder
/// removed.
pub fn install_panic_hook() {
    // `std::thread::panicking` is false when `ScratchFolders::drop` runs: libtest catches the
    // test's panic before the thread ends, so the hook marks the thread instead.
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            PANICKED.with(|panicked| panicked.set(true));
            previous(info);
        }));
    });
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::panic;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Barrier, mpsc};
    use std::thread;

    use super::{build_folder, fresh};

    fn tests_folder() -> PathBuf {
        build_folder().join("test_scratch_tests")
    }

    fn write_file(folder: &Path, name: &str) -> PathBuf {
        let path = folder.join(name);
        fs::write(&path, name).unwrap();
        path
    }

    #[test]
    fn a_passing_thread_removes_its_folders() {
        let first = tests_folder().join("passing_first");
        let second = tests_folder().join("passing_second");
        thread::spawn(|| {
            let first = fresh(&tests_folder(), "passing_first");
            write_file(&first, "first");
            let second = fresh(&tests_folder(), "passing_second");
            write_file(&second, "second");
        })
        .join()
        .unwrap();
        assert!(
            !first.exists(),
            "a passing thread's first folder remains: {}",
            first.display()
        );
        assert!(
            !second.exists(),
            "a passing thread's second folder remains: {}",
            second.display()
        );
    }

    #[test]
    fn a_panicking_thread_keeps_its_folder() {
        let folder = tests_folder().join("panicking_keeps");
        let file = folder.join("kept");
        let thread_file = file.clone();
        let join = thread::spawn(move || {
            fresh(&tests_folder(), "panicking_keeps");
            fs::write(&thread_file, "kept").unwrap();
            panic!("the panicking-thread test's intentional panic");
        })
        .join();
        assert!(join.is_err(), "the thread did not panic");
        assert!(
            folder.exists(),
            "a panicking thread's folder was removed: {}",
            folder.display()
        );
        assert!(
            file.exists(),
            "a panicking thread's file was removed: {}",
            file.display()
        );
        fs::remove_dir_all(&folder).unwrap();
    }

    /// The fixture helper of a tool's view tests in miniature: a fresh folder, one file written
    /// inside it, and the file's path returned.
    fn fixture_like(name: &str) -> PathBuf {
        let folder = fresh(&tests_folder(), name);
        assert!(
            folder.exists(),
            "fresh returned no folder: {}",
            folder.display()
        );
        write_file(&folder, "fixture")
    }

    #[test]
    fn a_path_a_helper_returns_outlives_the_helper() {
        let folder = tests_folder().join("helper_path");
        thread::spawn(|| {
            let path = fixture_like("helper_path");
            assert!(
                path.exists(),
                "the helper's file is gone before the thread ends: {}",
                path.display()
            );
        })
        .join()
        .unwrap();
        assert!(
            !folder.exists(),
            "the helper's folder remains after its thread ends: {}",
            folder.display()
        );
    }

    #[test]
    fn a_reused_folder_starts_empty() {
        let folder = tests_folder().join("reused");
        fs::create_dir_all(&folder).unwrap();
        let stale = write_file(&folder, "stale");
        thread::spawn(move || {
            let folder = fresh(&tests_folder(), "reused");
            assert!(
                folder.exists(),
                "fresh returned no folder: {}",
                folder.display()
            );
            assert!(
                !stale.exists(),
                "a file an earlier run left survives fresh: {}",
                stale.display()
            );
        })
        .join()
        .unwrap();
    }

    #[test]
    fn a_passing_thread_beside_a_panicking_one_keeps_only_the_panicking_folder() {
        let passing_folder = tests_folder().join("beside_passing");
        let panicking_folder = tests_folder().join("beside_panicking");
        let panicking_file = panicking_folder.join("kept");
        // A channel, not a barrier, says the folder is made: a barrier would wait forever for
        // a thread whose setup panicked, where a receive fails once the sender is dropped.
        let (made, made_receiver) = mpsc::channel();
        let release = Arc::new(Barrier::new(2));
        let thread_release = release.clone();
        let passing = thread::spawn(move || {
            let folder = fresh(&tests_folder(), "beside_passing");
            write_file(&folder, "file");
            made.send(()).unwrap();
            thread_release.wait();
        });
        if made_receiver.recv().is_err() {
            panic::resume_unwind(passing.join().unwrap_err());
        }
        assert!(
            passing_folder.exists(),
            "the passing thread's folder is not there yet: {}",
            passing_folder.display()
        );
        let panicking = thread::spawn(|| {
            let folder = fresh(&tests_folder(), "beside_panicking");
            write_file(&folder, "kept");
            panic!("the beside-panicking test's intentional panic");
        });
        assert!(
            panicking.join().is_err(),
            "the panicking thread did not panic"
        );
        release.wait();
        passing.join().unwrap();
        assert!(
            !passing_folder.exists(),
            "a passing thread's folder stays beside a panicking one: {}",
            passing_folder.display()
        );
        assert!(
            panicking_folder.exists(),
            "a panicking thread's folder was removed: {}",
            panicking_folder.display()
        );
        assert!(
            panicking_file.exists(),
            "a panicking thread's file was removed: {}",
            panicking_file.display()
        );
        fs::remove_dir_all(&panicking_folder).unwrap();
    }

    #[test]
    fn build_folder_is_the_test_binary_s_profile_folder() {
        assert_eq!(
            build_folder().join("deps"),
            std::env::current_exe().unwrap().parent().unwrap()
        );
    }

    #[test]
    #[should_panic(expected = "a scratch name holds no path separator: nested/name")]
    fn a_name_with_a_slash_is_refused() {
        fresh(&tests_folder(), "nested/name");
    }

    #[test]
    #[should_panic(expected = r"a scratch name holds no path separator: nested\name")]
    fn a_name_with_a_backslash_is_refused() {
        fresh(&tests_folder(), r"nested\name");
    }
}
