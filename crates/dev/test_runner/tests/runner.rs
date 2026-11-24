//! Drives the built runner binary on the probe (`utw-runner-probe <mb> <code>`,
//! `src/bin/probe.rs`) the way the test gate does: cap first, then the child and its
//! arguments.
#![cfg(windows)]

use std::process::Command;

const RUNNER: &str = env!("CARGO_BIN_EXE_utw-test-runner");
const PROBE: &str = env!("CARGO_BIN_EXE_utw-runner-probe");

/// The runner's exit code, stdout and stderr when called with `args`, its stdio piped.
fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let output = Command::new(RUNNER)
        .args(args)
        .output()
        .expect("the runner binary starts");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn the_childs_exit_code_passes_through() {
    let (code, stdout, stderr) = run(&["64M", PROBE, "16", "7"]);
    assert_eq!(code, Some(7));
    assert!(
        stdout.contains("utw-runner-probe: 16 MB touched"),
        "the child's stdout reaches the caller: {stdout:?}"
    );
    assert!(!stderr.contains("memory cap"), "stderr: {stderr}");
}

#[test]
fn an_over_cap_child_aborts_and_is_reported() {
    let (code, _, stderr) = run(&["64M", PROBE, "256", "0"]);
    assert_eq!(code, Some(0xC0000409u32 as i32));
    assert!(
        stderr.contains(
            "test_runner: utw-runner-probe.exe hit the 64M per-process memory cap (.cargo/config.toml)"
        ),
        "stderr: {stderr}"
    );
}

#[test]
fn fewer_than_two_arguments_gets_the_usage() {
    for args in [&[][..], &["64M"][..]] {
        let (code, _, stderr) = run(args);
        assert_eq!(code, Some(2), "args: {args:?}");
        assert_eq!(
            stderr.trim(),
            "usage: utw-test-runner <cap> <exe> [args...]",
            "args: {args:?}"
        );
    }
}

#[test]
fn an_invalid_cap_is_refused() {
    let (code, _, stderr) = run(&["6K", PROBE, "0", "0"]);
    assert_eq!(code, Some(2));
    assert_eq!(
        stderr.trim(),
        "test_runner: invalid cap '6K' (want e.g. 256M, 8G)"
    );
}

#[test]
fn a_child_that_cannot_start_is_reported() {
    let missing = "C:/nonexistent/utw-no-such-child.exe";
    let (code, _, stderr) = run(&["64M", missing]);
    assert_eq!(code, Some(2));
    assert!(
        stderr.starts_with(&format!("test_runner: cannot start {missing}: ")),
        "stderr: {stderr}"
    );
}
