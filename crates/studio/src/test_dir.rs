//! The binary's unit-test folder helper, which each test module uses.

use std::path::{Path, PathBuf};

/// A fresh, empty folder for a test under the test binary's build folder
/// (`target/<profile>/studio_tests/<name>`): per checkout, so a mutation run's copy has its
/// own. [`test_scratch::fresh`] says when the folder is removed and what `name` must be.
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
