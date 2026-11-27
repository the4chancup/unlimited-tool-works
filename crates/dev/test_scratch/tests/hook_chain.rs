//! Alone in its own binary: `fresh` chains whichever panic hook is installed when it first
//! runs in a process, so the hook this test installs must be in place before that first call.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

static PREVIOUS_HOOK_RAN: AtomicBool = AtomicBool::new(false);

#[test]
fn the_hook_fresh_chains_still_runs_on_a_panic() {
    // The hook it replaces still runs, so a failing assertion below prints.
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        PREVIOUS_HOOK_RAN.store(true, Ordering::SeqCst);
        default(info);
    }));
    let parent = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hook_chain");
    let folder = parent.join("kept");
    let thread_parent = parent.clone();
    let join = thread::spawn(move || {
        test_scratch::fresh(&thread_parent, "kept");
        panic!("the hook-chain test's intentional panic");
    })
    .join();
    assert!(join.is_err(), "the thread did not panic");
    assert!(
        PREVIOUS_HOOK_RAN.load(Ordering::SeqCst),
        "the hook installed before fresh did not run"
    );
    fs::remove_dir_all(&folder).unwrap();
}
