//! The parity tests' upgraded case, its own module so that only the two parity targets declare
//! it (the cli suite shares `common`, and compiles no upgraded export).

use std::fs;
use std::path::{Path, PathBuf};

/// The exports root for the parity tests' upgraded case: `tests/fixtures/<tracer>/upgraded/`,
/// compiled in place, which holds the one golden export.
///
/// The golden is a `Midcup` export, as the hand-made twin is: the tracers are cut down to one
/// player and kit `g1` and hold no logo, so upgraded as `Full` they would be refused for the
/// missing logo, and compiled as `Full` they would get the compiler's placeholder kit `p1`,
/// which the reference tree has no counterpart for. The golden's own name is pinned by
/// `export_upgrader`'s `upgraded_golden` test.
pub fn upgraded_golden(tracer: &str) -> PathBuf {
    let upgraded = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(tracer)
        .join("upgraded");
    let exports: Vec<PathBuf> = fs::read_dir(&upgraded)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(exports.len(), 1, "upgraded/ holds the one golden export");
    let name = exports[0].file_name().unwrap().to_str().unwrap();
    assert!(
        name.contains(" Midcup "),
        "{name}: the golden is a Midcup export"
    );
    upgraded
}
