//! ST-RUN-10 end to end: the real exe over a test data location, started with
//! `--crash-test stack`, overflows its stack once the crash monitor runs and before any
//! window, so no display is needed. The monitor writes `utw.dmp` beside `utw.log`
//! and appends the line the next start names the dump from. ST-REP-11 runs the same
//! exe with `--crash-test session`: two steps reach the session folder through the
//! monitor's pipe, then the window goes away with no closing message and `[end]`
//! reads `ended`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The monitor's last log line after writing the dump (`crash_dump::DUMP_WRITTEN_LINE`,
/// which a test cannot import from the binary).
const DUMP_WRITTEN_LINE: &str = "the crash dump was written to utw.dmp beside this log";

/// The releases list's address the run reads: no connection can be made to port 0, so an
/// update check the window starts fails at once and never reaches GitHub.
const NO_RELEASES_URL: &str = "http://127.0.0.1:0/";

/// The longest a wait on the window or the monitor may take: a bound that
/// stops a hang, never a measure of speed.
const WAIT: Duration = Duration::from_secs(120);

/// A fresh folder holding a hard link to the binary and an empty `data/settings.toml`,
/// so the portable data location is found beside the executable and no settings file
/// elsewhere on the machine is read (`team_compiler_cli.rs`'s shape).
struct Sandbox {
    root: PathBuf,
    exe: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Sandbox {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("crash_dump")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("data")).unwrap();
        let built = Path::new(env!("CARGO_BIN_EXE_utw"));
        let exe = root.join(built.file_name().unwrap());
        fs::hard_link(built, &exe).unwrap();
        fs::write(root.join("data/settings.toml"), "").unwrap();
        Sandbox { root, exe }
    }
}

// ST-RUN-10
#[test]
fn a_stack_overflow_leaves_a_dump_and_the_monitors_line() {
    let sandbox = Sandbox::new("stack");
    let mut child = Command::new(&sandbox.exe)
        .args(["--crash-test", "stack"])
        .env("UTW_RELEASES_URL", NO_RELEASES_URL)
        .current_dir(&sandbox.root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + WAIT;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            panic!("the crash test was still running after {WAIT:?}: the monitor never answered");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(!status.success(), "the crashed run exited with {status}");

    let log = fs::read_to_string(sandbox.root.join("data/utw.log")).unwrap();
    // Under load the dump has gone missing; the log says whether the monitor
    // ever answered (a Warn line when `start_monitor` gave up waiting).
    let dump = minidump::Minidump::read_path(sandbox.root.join("data/utw.dmp"))
        .unwrap_or_else(|error| panic!("no dump ({error}); the crashed run's log:\n{log}"));
    let threads: minidump::MinidumpThreadList = dump.get_stream().unwrap();
    assert!(!threads.threads.is_empty(), "the dump holds no thread");
    let exception: minidump::MinidumpException = dump.get_stream().unwrap();
    #[cfg(windows)]
    let expected = 0xC000_00FD; // STATUS_STACK_OVERFLOW
    #[cfg(not(windows))]
    let expected = 11; // SIGSEGV
    assert_eq!(exception.raw.exception_record.exception_code, expected);

    let last = log
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap();
    assert!(last.ends_with(DUMP_WRITTEN_LINE), "{last}");
}

// ST-REP-11
#[test]
fn a_window_gone_without_its_closing_message_leaves_an_ended_session() {
    let sandbox = Sandbox::new("session");
    let mut child = Command::new(&sandbox.exe)
        .args(["--crash-test", "session"])
        .env("UTW_RELEASES_URL", NO_RELEASES_URL)
        .current_dir(&sandbox.root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + WAIT;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            panic!("the session test was still running after {WAIT:?}: its steps never landed");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(status.success(), "the session run exited with {status}");

    // The monitor writes `[end]` once the window's pipe ends: it can trail
    // the window's exit by a moment, so the journal is polled for it.
    let sessions = sandbox.root.join("data/sessions");
    let deadline = Instant::now() + WAIT;
    let journal = loop {
        let found = fs::read_dir(&sessions).ok().and_then(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| fs::read_to_string(entry.path().join("journal.toml")).ok())
                .find(|journal| journal.contains("[end]"))
        });
        if let Some(journal) = found {
            break journal;
        }
        if Instant::now() > deadline {
            panic!("no session journal held [end] after {WAIT:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(
        journal.matches("[[step]]").count(),
        2,
        "the two steps reached the folder: {journal}"
    );
    assert!(
        journal.contains("the session test's first step")
            && journal.contains("the session test's second step"),
        "{journal}"
    );
    assert!(
        journal.contains("how = \"ended\""),
        "the window sent no closing message: {journal}"
    );
}
