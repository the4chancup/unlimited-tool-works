//! ST-RUN-10 end to end: the real exe over a test data location, started with
//! `--crash-test stack`, overflows its stack once the crash monitor runs and before any
//! window, so no display is needed. The monitor writes `utw.dmp` beside `utw.log`
//! and appends the line the next start names the dump from. ST-REP-11 runs the same
//! exe with `--crash-test session`: two steps reach the session folder through the
//! monitor's pipe, then the window goes away with no closing message and `[end]`
//! reads `ended`. ST-REP-10 runs it with `--crash-test monitor`: a step lands, the
//! monitor is ended, a second step lands in the same folder through the monitor the
//! link starts in its place, and the session closes `closed`.

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

/// `child`'s exit status, waited on up to `WAIT` and killed past it —
/// `what` names what never happened in the panic.
fn await_exit(child: &mut std::process::Child, what: &str) -> std::process::ExitStatus {
    let deadline = Instant::now() + WAIT;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            panic!("{what} after {WAIT:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The session journal under `sessions` holding `[end]`, waited on up to
/// `WAIT`: the monitor writes it once the window's pipe ends, which can
/// trail the window's exit by a moment.
fn await_journal_end(sessions: &Path) -> String {
    let deadline = Instant::now() + WAIT;
    loop {
        let found = fs::read_dir(sessions).ok().and_then(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| fs::read_to_string(entry.path().join("journal.toml")).ok())
                .find(|journal| journal.contains("[end]"))
        });
        if let Some(journal) = found {
            return journal;
        }
        if Instant::now() > deadline {
            panic!("no session journal held [end] after {WAIT:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

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
    let status = await_exit(
        &mut child,
        "the crash test was still running: the monitor never answered",
    );
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
    let status = await_exit(
        &mut child,
        "the session test was still running: its steps never landed",
    );
    assert!(status.success(), "the session run exited with {status}");

    let journal = await_journal_end(&sandbox.root.join("data/sessions"));
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

// ST-REP-10
#[test]
fn a_dead_monitor_is_replaced_and_the_session_stays_in_one_folder() {
    let sandbox = Sandbox::new("monitor");
    let mut child = Command::new(&sandbox.exe)
        .args(["--crash-test", "monitor"])
        .env("UTW_RELEASES_URL", NO_RELEASES_URL)
        .current_dir(&sandbox.root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let status = await_exit(&mut child, "the monitor test was still running");
    assert!(status.success(), "the monitor run exited with {status}");

    let sessions = sandbox.root.join("data/sessions");
    let journal = await_journal_end(&sessions);
    let folders: Vec<_> = fs::read_dir(&sessions)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .collect();
    assert_eq!(
        folders.len(),
        1,
        "one session folder for the whole run: the new monitor adopted the dead one's"
    );
    assert_eq!(
        journal.matches("[[step]]").count(),
        2,
        "both steps reached the one folder: {journal}"
    );
    assert!(
        journal.contains("the monitor test's first step")
            && journal.contains("the monitor test's second step"),
        "{journal}"
    );
    assert!(
        journal.contains("how = \"closed\""),
        "the session was closed, not ended: {journal}"
    );

    let log = fs::read_to_string(sandbox.root.join("data/utw.log")).unwrap();
    let broke = log
        .lines()
        .position(|line| line.contains("link to the crash monitor broke"));
    assert!(
        broke.is_some(),
        "the break's line is in the slot's log: {log}"
    );
    // The steps reaching the folder would read the same through the window's
    // own writer: the new monitor's start line is what tells them apart,
    // and a failed one would have logged "did not start" after the break.
    assert!(
        log.lines()
            .any(|line| line.contains("a new crash monitor was started")),
        "the new monitor's line is in the slot's log: {log}"
    );
    let rest = &log.lines().collect::<Vec<_>>()[broke.unwrap() + 1..];
    assert!(
        !rest.iter().any(|line| line.contains("did not start")),
        "no restart failed after the break: {log}"
    );
}
