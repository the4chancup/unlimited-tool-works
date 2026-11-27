//! The binary's unit-test folder helper, which each test module uses.

use std::path::{Path, PathBuf};

/// A fresh, empty folder for a test under the test binary's build folder
/// (`target/<profile>/studio_tests/<name>`): per checkout, so a mutation run's copy has its
/// own. A passing test's folder is removed when its thread ends; a panicking test's stays
/// for reading and is emptied at the start of its next run. `name` must be unique among
/// the binary's tests: two tests running at once must not share a folder.
pub(crate) struct TestDir(PathBuf);

impl TestDir {
    pub(crate) fn new(name: &str) -> TestDir {
        TestDir(test_scratch::fresh(
            &test_scratch::build_folder().join("studio_tests"),
            name,
        ))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}
