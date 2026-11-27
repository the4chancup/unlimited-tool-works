//! Helpers the unit tests of several modules share.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use studio_core::{AppPaths, PipelineEvent, PipelineEventEnvelope, Settings, ToolContext};

/// A fresh, empty scratch folder for `name` under the test binary's build
/// folder (`target/<profile>/export_upgrader_tests/<name>`): per checkout, so
/// a mutation run's copy has its own. A passing test's folder is removed when
/// its thread ends; a panicking test's stays for reading and is emptied at the
/// start of its next run. `name` must be unique among the tests that call it:
/// two tests running at once must not share a folder.
pub(crate) fn scratch(name: &str) -> PathBuf {
    test_scratch::fresh(
        &test_scratch::build_folder().join("export_upgrader_tests"),
        name,
    )
}

/// Writes into the old export at `export` the files it still lacks for its
/// upgrade to be clean (`test_exports::complete_old`). A test writes the files
/// it is about first.
pub(crate) fn complete_old(export: &Path) {
    crate::test_exports::complete_old(export, &[]);
}

/// The files below `folder`, counted; a folder itself is no file.
pub(crate) fn file_count(folder: &Path) -> usize {
    fs::read_dir(folder)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() { file_count(&path) } else { 1 }
        })
        .sum()
}

/// The `(processed, total)` of the run's `Progress` events, in order.
pub(crate) fn progress_events(
    events_rx: &crossbeam_channel::Receiver<PipelineEventEnvelope>,
) -> Vec<(usize, usize)> {
    events_rx
        .iter()
        .filter_map(|envelope| match envelope.event {
            PipelineEvent::Progress { processed, total } => Some((processed, total)),
            _ => None,
        })
        .collect()
}

/// The unit tests' data directory, under the test binary's build folder
/// (`target/<profile>/export_upgrader_tests/data_dir`), made if missing and
/// never removed. It holds nothing (the upgrader reads no list of its own), so
/// every test, and every test process, shares it. Not a `scratch` folder: a
/// `scratch` folder goes when the test that made it ends.
fn data_dir() -> PathBuf {
    static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
    DATA_DIR
        .get_or_init(|| {
            let dir = test_scratch::build_folder()
                .join("export_upgrader_tests")
                .join("data_dir");
            fs::create_dir_all(&dir).unwrap();
            dir
        })
        .clone()
}

/// A `ToolContext` whose events land on the returned receiver, its data
/// directory [`data_dir`].
pub(crate) fn test_ctx() -> (
    ToolContext,
    crossbeam_channel::Receiver<PipelineEventEnvelope>,
) {
    let (events_tx, events_rx) = crossbeam_channel::unbounded();
    let (requests_tx, _requests_rx) = crossbeam_channel::unbounded();
    (
        ToolContext::new(
            Arc::new(Mutex::new(Settings::default())),
            AppPaths {
                exe_dir: PathBuf::new(),
                data_dir: Some(data_dir()),
            },
            events_tx,
            requests_tx,
        ),
        events_rx,
    )
}
