//! Drives the built runner binary on the probe (`utw-runner-probe <mb> <code>`,
//! `src/bin/probe.rs`) the way the test gate does: cap first, then the child and its
//! arguments.
#![cfg(windows)]

use std::path::{Path, PathBuf};
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

/// `scripts/gates/test_runner.cmd`, the shim cargo runs every test binary through.
fn shim() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scripts/gates/test_runner.cmd")
}

/// The shim's exit code for `args`, run as cargo runs it, with `CARGO_HOME` at `home`.
fn run_shim(home: &Path, args: &[&str]) -> Option<i32> {
    Command::new(shim())
        .args(args)
        .env("CARGO_HOME", home)
        .output()
        .expect("the shim starts")
        .status
        .code()
}

// A failing test must fail through the shim on both branches: a parenthesized
// `exit /b` once returned 0 for every child on the compiled branch.
#[test]
fn the_shim_passes_the_childs_exit_code_through_on_both_branches() {
    let dir = test_scratch::fresh(
        &Path::new(env!("CARGO_TARGET_TMPDIR")).join("test_runner_runner"),
        "utw-shim",
    );
    let compiled = dir.join("compiled");
    let python = dir.join("python");
    std::fs::create_dir_all(compiled.join("bin")).expect("the compiled home");
    std::fs::create_dir_all(&python).expect("the python home");
    std::fs::copy(RUNNER, compiled.join("bin").join("utw-test-runner.exe"))
        .expect("the runner copied into the compiled home");
    for home in [&compiled, &python] {
        for code in [7, 0] {
            let code_text = code.to_string();
            assert_eq!(
                run_shim(home, &["64M", PROBE, "1", &code_text]),
                Some(code),
                "{home:?}, child exit {code}"
            );
        }
    }
}
