//! `utw-test-runner <cap> <exe> [args...]`: runs `<exe>` inside a Windows Job Object that
//! caps its memory at `<cap>`. `scripts/gates/test_runner.cmd` runs this binary when it
//! is installed and falls back to `test_runner.py` when it is not.
//!
//! Windows has no OOM killer: a runaway test binary (a mutation that pushes into a Vec
//! forever) pages until the commit limit runs out and whichever process allocates next,
//! a browser, a game or the IDE, fails instead. `JOB_OBJECT_LIMIT_PROCESS_MEMORY` makes
//! the child fail its own allocation and abort alone. `cargo test` and `cargo nextest`
//! run every binary they launch through the runner (`.cargo/config.toml`'s `runner`), and
//! nextest starts one process per test, so the runner's own startup cost is paid once per
//! test in the suite.
//!
//! The runner assigns its own process to the job before spawning, so the child inherits
//! it; `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` means a timeout kill of the runner takes the
//! child with it instead of orphaning it. A completion port attached to the job reports
//! `JOB_OBJECT_MSG_PROCESS_MEMORY_LIMIT` the moment a member attempts an over-cap
//! allocation: a peak-committed check would miss those, since a failed request never
//! commits.

#[cfg(windows)]
mod job;

#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::path::Path;

/// The runner's entry point: `run`'s exit code becomes the process's.
#[cfg(windows)]
fn main() {
    std::process::exit(run());
}

/// Reports `run_inner`'s error on stderr as exit 2; a clean run exits with the child's
/// code unchanged.
#[cfg(windows)]
#[expect(clippy::print_stderr, reason = "the runner's report goes to stderr")]
fn run() -> i32 {
    match run_inner() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("{message}");
            2
        }
    }
}

/// The whole job: parse the cap and the command, put this process in the job so the
/// child inherits it, run the child, then report a member that hit the cap.
#[cfg(windows)]
#[expect(clippy::print_stderr, reason = "the runner's report goes to stderr")]
fn run_inner() -> Result<i32, String> {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.len() < 2 {
        return Err("usage: utw-test-runner <cap> <exe> [args...]".to_string());
    }
    let cap_text = args[0].to_string_lossy();
    let Some(cap) = cap_bytes(&cap_text) else {
        return Err(format!(
            "test_runner: invalid cap '{cap_text}' (want e.g. 256M, 8G)"
        ));
    };
    let port = job::create_port()?;
    let job = job::create_job(cap, port)?;
    job::assign_self_to_job(job)?;
    let exe = &args[1];
    let status = std::process::Command::new(exe)
        .args(&args[2..])
        .status()
        .map_err(|error| {
            format!(
                "test_runner: cannot start {}: {error}",
                exe.to_string_lossy()
            )
        })?;
    if job::hit_memory_limit(port) {
        let name = Path::new(exe)
            .file_name()
            .unwrap_or(exe.as_os_str())
            .to_string_lossy();
        eprintln!(
            "test_runner: {name} hit the {cap_text} per-process memory cap (.cargo/config.toml)"
        );
    }
    // Windows reports a DWORD for every child exit, a kill included; the fallback
    // covers only the signal case `code` leaves open for other platforms.
    Ok(status.code().unwrap_or(1))
}

/// Stub for the platforms the runner does not run on, so the crate still builds there.
#[cfg(not(windows))]
#[expect(clippy::print_stderr, reason = "the runner's report goes to stderr")]
fn main() {
    eprintln!("test_runner: Windows only");
    std::process::exit(2);
}

/// `<n>M` or `<n>G` to bytes, `n` a positive decimal integer; anything else is `None`.
#[cfg(any(windows, test))]
fn cap_bytes(text: &str) -> Option<u64> {
    let (digits, shift) = if let Some(digits) = text.strip_suffix('M') {
        (digits, 20)
    } else {
        (text.strip_suffix('G')?, 30)
    };
    let n: u64 = digits.parse().ok()?;
    if n == 0 {
        return None;
    }
    n.checked_mul(1 << shift)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_bytes_takes_the_two_units_and_refuses_the_rest() {
        let cases = [
            ("256M", Some(256u64 << 20)),
            ("8G", Some(8u64 << 30)),
            ("6G", Some(6u64 << 30)),
            ("0G", None),
            ("6", None),
            ("6K", None),
            ("G", None),
            ("-1G", None),
            ("1.5G", None),
            ("", None),
        ];
        for (text, expected) in cases {
            assert_eq!(cap_bytes(text), expected, "{text}");
        }
    }
}
