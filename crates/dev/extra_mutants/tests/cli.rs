//! The binary end to end: `extra_mutants.py` parses this output, so a lister
//! that silently skips files or prints nothing reads as "no mutants, nothing
//! missed".

use std::path::{Path, PathBuf};
use std::process::Command;

/// A fresh, empty folder for a test's crate tree, under the integration tests' build
/// folder (`target/tmp/extra_mutants_cli/cli_<name>`). A passing test's folder is removed
/// when its thread ends; a panicking test's stays for reading and is emptied at the start
/// of its next run.
fn temp_dir(name: &str) -> PathBuf {
    test_scratch::fresh(
        &Path::new(env!("CARGO_TARGET_TMPDIR")).join("extra_mutants_cli"),
        &format!("cli_{name}"),
    )
}

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("a file path")).expect("temp dir writable");
    std::fs::write(&path, text).expect("temp file writable");
}

#[test]
fn lists_the_mutants_of_a_small_crate() {
    let dir = temp_dir("list");
    // `mod lib` names the root file's own stem: it must never reach the module
    // lookup `is_test_file` runs for a file, or lib.rs reads as its own
    // `#[cfg(test)]` declaration. `src/a/lib.rs` exercises the same check one
    // folder down, where the early return must not fire.
    write(
        &dir,
        "src/lib.rs",
        "fn top(x: u32) {\n    if x > 0 {\n        return;\n    }\n}\n\nmod a;\n\n#[cfg(test)]\nmod lib;\n\n#[cfg(test)]\nmod tests;\n",
    );
    write(&dir, "src/a/mod.rs", "mod b;\n\n#[cfg(test)]\nmod lib;\n");
    write(
        &dir,
        "src/a/b.rs",
        "fn f(x: u32) {\n    if x > 1 {\n        return;\n    }\n}\n",
    );
    write(
        &dir,
        "src/a/lib.rs",
        "fn g(x: u32) {\n    if x > 2 {\n        return;\n    }\n}\n",
    );
    write(
        &dir,
        "src/tests.rs",
        "fn t(x: u32) {\n    if x > 9 {\n        return;\n    }\n}\n",
    );
    write(&dir, "src/notes.txt", "not rust\n");

    let output = Command::new(env!("CARGO_BIN_EXE_extra_mutants"))
        .arg("list")
        .arg(&dir)
        .output()
        .expect("the binary runs");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 output");
    let mut paths = Vec::new();
    for line in stdout.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 7, "line: {line}");
        paths.push(fields[0]);
        assert_eq!(fields[3], "guard");
        assert_eq!(fields[5], "false");
    }
    assert_eq!(paths, ["src/a/b.rs", "src/lib.rs"]);
    for path in &paths {
        assert!(!path.contains('\\'), "path: {path}");
    }

    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn wrong_arguments_print_the_usage_and_exit_2() {
    for args in [["bogus"].as_slice(), ["list"].as_slice()] {
        let output = Command::new(env!("CARGO_BIN_EXE_extra_mutants"))
            .args(args)
            .output()
            .expect("the binary runs");
        assert_eq!(output.status.code(), Some(2), "args: {args:?}");
        let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
        assert!(
            stderr.contains("usage: extra_mutants list <crate dir>"),
            "args: {args:?}, stderr: {stderr}"
        );
    }
}
