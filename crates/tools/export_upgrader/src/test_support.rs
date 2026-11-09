//! Helpers the unit tests of several modules share.

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use studio_core::{AppPaths, PipelineEventEnvelope, Settings, ToolContext};

/// A scratch folder under the temp dir (`CARGO_TARGET_TMPDIR` is set for
/// integration tests only).
pub(crate) fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("export_upgrader_{name}_{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(&root).unwrap();
    root
}

/// The unit tests' data directory, made once per process: it holds nothing
/// (the upgrader reads no list of its own).
fn data_dir() -> PathBuf {
    static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
    DATA_DIR.get_or_init(|| scratch("data_dir")).clone()
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
