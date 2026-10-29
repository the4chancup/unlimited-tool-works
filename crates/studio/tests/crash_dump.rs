//! ST-RUN-10 end to end: the real exe over a test data location, started with
//! `--crash-test stack`, overflows its stack once the crash monitor runs and before any
//! window, so no display is needed. The monitor writes `utw.dmp` beside `utw.log`
//! and appends the line the next start names the dump from.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The monitor's last log line after writing the dump (`crash_dump::DUMP_WRITTEN_LINE`,
/// which a test cannot import from the binary).
const DUMP_WRITTEN_LINE: &str = "the crash dump was written to utw.dmp beside this log";

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
        .current_dir(&sandbox.root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            panic!("the crash test was still running after 60 s: the monitor never answered");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(!status.success(), "the crashed run exited with {status}");

    let dump = minidump::Minidump::read_path(sandbox.root.join("data/utw.dmp")).unwrap();
    let threads: minidump::MinidumpThreadList = dump.get_stream().unwrap();
    assert!(!threads.threads.is_empty(), "the dump holds no thread");
    let exception: minidump::MinidumpException = dump.get_stream().unwrap();
    #[cfg(windows)]
    let expected = 0xC000_00FD; // STATUS_STACK_OVERFLOW
    #[cfg(not(windows))]
    let expected = 11; // SIGSEGV
    assert_eq!(exception.raw.exception_record.exception_code, expected);

    let log = fs::read_to_string(sandbox.root.join("data/utw.log")).unwrap();
    let last = log
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap();
    assert!(last.ends_with(DUMP_WRITTEN_LINE), "{last}");
}
