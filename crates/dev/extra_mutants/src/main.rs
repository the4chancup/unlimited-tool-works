//! `extra_mutants list <crate dir>`: one mutant per line, tab-separated
//! (`<path>\t<start>\t<end>\t<kind>\t<line>\t<replacement>\t<description>`), for
//! `scripts/mutation/extra_mutants.py` to apply and test.

mod scan;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[expect(clippy::print_stderr, reason = "a dev tool's errors go to stderr")]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "list" {
        eprintln!("usage: extra_mutants list <crate dir>");
        return ExitCode::from(2);
    }
    match list(Path::new(&args[1])) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[expect(
    clippy::print_stdout,
    reason = "the lister's only output is the mutant list"
)]
fn list(crate_dir: &Path) -> Result<(), String> {
    let mut files = Vec::new();
    walk(&crate_dir.join("src"), &mut files)
        .map_err(|error| format!("{}: {error}", crate_dir.join("src").display()))?;
    files.sort();
    for path in files {
        if is_test_file(&path) {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let mutants =
            scan::scan_source(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        let rel = relative(&path, crate_dir);
        for mutant in &mutants {
            println!(
                "{rel}\t{}\t{}\t{}\t{}\t{}\t{}",
                mutant.start,
                mutant.end,
                mutant.kind,
                mutant.line,
                mutant.replacement,
                mutant.description
            );
        }
    }
    Ok(())
}

/// Every `.rs` file under `dir`, recursively.
fn walk(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            walk(&path, files)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

/// Whether `path` is a module some ancestor declares under `#[cfg(test)]`: `mod name;`
/// with the attribute, in the file Rust looks the module up in (`a/mod.rs` or `a.rs`).
/// The file walk sees a test module's file even though the parent's `#[cfg(test)]`
/// covers it, so it needs this check of its own; a file whose parent is not found
/// counts as production.
fn is_test_file(path: &Path) -> bool {
    let Some(dir) = path.parent() else {
        return false;
    };
    // A module's name is its file stem, or its directory's name for `mod.rs`; the
    // declaration `mod name;` sits in the parent's `mod.rs` or `<name>.rs`, or in the
    // crate root for a directory directly under `src/`.
    let (name, folder) = if path.file_name().is_some_and(|f| f == "mod.rs") {
        (dir.file_name(), dir.parent())
    } else {
        (path.file_stem(), Some(dir))
    };
    let (Some(name), Some(folder)) = (name, folder) else {
        return false;
    };
    let name = name.to_string_lossy();
    if folder.file_name().is_some_and(|f| f == "src")
        && path
            .file_name()
            .is_some_and(|f| f == "lib.rs" || f == "main.rs")
    {
        return false;
    }
    let parents = if folder.file_name().is_some_and(|f| f == "src") {
        [folder.join("lib.rs"), folder.join("main.rs")]
    } else {
        [folder.join("mod.rs"), folder.with_extension("rs")]
    };
    for parent in &parents {
        let Ok(text) = std::fs::read_to_string(parent) else {
            continue;
        };
        let Ok(file) = syn::parse_file(&text) else {
            continue;
        };
        let module = file.items.iter().find_map(|item| match item {
            syn::Item::Mod(module) if module.content.is_none() && module.ident == name => {
                Some(module)
            }
            _ => None,
        });
        let Some(module) = module else {
            continue;
        };
        return scan::has_test_attr(&module.attrs) || is_test_file(parent);
    }
    false
}

/// `path` relative to `crate_dir`, forward slashes for the tab-separated output.
fn relative(path: &Path, crate_dir: &Path) -> String {
    path.strip_prefix(crate_dir)
        .unwrap_or(path)
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("extra_mutants_{name}_{}", std::process::id()))
    }

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().expect("a file path")).expect("temp dir writable");
        std::fs::write(&path, text).expect("temp file writable");
    }

    #[test]
    fn a_cfg_test_declared_file_is_test_code() {
        let dir = temp_dir("gated");
        write(&dir, "src/lib.rs", "#[cfg(test)]\nmod tests;\nmod prod;\n");
        write(&dir, "src/tests.rs", "fn f() {}\n");
        write(&dir, "src/prod.rs", "fn f() {}\n");
        assert!(is_test_file(&dir.join("src/tests.rs")));
        assert!(!is_test_file(&dir.join("src/prod.rs")));
        assert!(!is_test_file(&dir.join("src/lib.rs")));
    }

    #[test]
    fn a_file_below_a_cfg_test_module_is_test_code() {
        let dir = temp_dir("nested");
        write(&dir, "src/lib.rs", "#[cfg(test)]\nmod helper;\n");
        write(&dir, "src/helper/mod.rs", "mod inside;\n");
        write(&dir, "src/helper/inside.rs", "fn f() {}\n");
        assert!(is_test_file(&dir.join("src/helper/mod.rs")));
        assert!(is_test_file(&dir.join("src/helper/inside.rs")));
    }
}
