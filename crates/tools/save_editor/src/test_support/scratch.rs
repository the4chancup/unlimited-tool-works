//! The test scratch folders' end-of-run cleanup.
//!
//! A guard returned by `fresh` cannot remove the folder: helpers hand their test a path inside
//! the folder, and the guard would drop when the helper returned. The folders are therefore
//! tied to the thread libtest gives each test, and a panicking thread keeps them for reading.

use std::cell::{Cell, RefCell};
use std::fs;
use std::path::PathBuf;
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

/// `dir` as a fresh, empty folder, recorded for removal when the current thread ends.
///
/// A folder that cannot be removed or created panics. When the thread panics, the folder is
/// kept so the test's files can be read.
pub(crate) fn fresh(dir: PathBuf) -> PathBuf {
    install_panic_hook();
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("the scratch folder is removable");
    }
    fs::create_dir_all(&dir).expect("the scratch folder is creatable");
    FOLDERS.with(|folders| folders.0.borrow_mut().push(dir.clone()));
    dir
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
