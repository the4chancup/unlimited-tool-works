//! Tests' scratch folders: each test gets a fresh, empty folder, removed when the test passes
//! and kept for reading when it fails.
//!
//! A guard returned by `fresh` cannot remove the folder: helpers hand their test a path inside
//! the folder, and the guard would drop when the helper returned. The folders are therefore
//! tied to the thread libtest gives each test, and a panicking thread keeps them for reading.

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

/// `parent/name` as a fresh, empty folder, recorded for removal when the current thread ends.
///
/// Panics when the folder cannot be removed or created, or when `name` holds a path separator.
/// When the thread panics, the folder is kept so the test's files can be read.
pub fn fresh(parent: &Path, name: &str) -> PathBuf {
    // A separator would nest the folder, and only the innermost one is removed.
    assert!(
        !name.contains(['/', '\\']),
        "a scratch name holds no path separator: {name}"
    );
    install_panic_hook();
    let dir = parent.join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("the scratch folder is removable");
    }
    fs::create_dir_all(&dir).expect("the scratch folder is creatable");
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

/// Installs the panic hook that sets `PANICKED` on the panicking thread. `std::thread::panicking`
/// is false when `ScratchFolders::drop` runs: libtest catches the test's panic before the thread
/// ends. The previous hook still runs, so the panic prints.
fn install_panic_hook() {
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
    use std::path::{Path, PathBuf};
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
