//! The unit tests' scratch folders and contexts, under the test binary's own
//! build folder rather than the system Temp.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use studio_core::{AppPaths, PipelineEventEnvelope, Settings, ShellRequest, ToolContext};

/// A fresh, empty scratch folder for `test` under the test binary's build
/// folder (`target/<profile>/ruleset_editor_tests/<test>`): per checkout, so
/// a mutation run's copy has its own. A passing test's folder is removed when
/// its thread ends; a panicking test's stays for reading and is emptied at the
/// start of its next run. `test` must be unique among the tests that call it:
/// two tests running at once must not share a folder.
pub(crate) fn scratch(test: &str) -> PathBuf {
    test_scratch::fresh(
        &test_scratch::build_folder().join("ruleset_editor_tests"),
        test,
    )
}

/// A `ToolContext` over fresh settings and channels, and the events receiver
/// the context emits to: how a unit test runs a command's report.
pub(crate) fn context() -> (
    ToolContext,
    crossbeam_channel::Receiver<PipelineEventEnvelope>,
) {
    let (ctx, events, _requests) = context_requests();
    (ctx, events)
}

/// `context()` with the shell-requests receiver kept (the close prompt's
/// `request_close` lands there).
pub(crate) fn context_requests() -> (
    ToolContext,
    crossbeam_channel::Receiver<PipelineEventEnvelope>,
    crossbeam_channel::Receiver<ShellRequest>,
) {
    let (events_tx, events_rx) = crossbeam_channel::unbounded();
    let (requests_tx, requests_rx) = crossbeam_channel::unbounded();
    let ctx = ToolContext::new(
        Arc::new(Mutex::new(Settings::default())),
        AppPaths {
            exe_dir: PathBuf::from("exe"),
            data_dir: None,
        },
        events_tx,
        requests_tx,
    );
    (ctx, events_rx, requests_rx)
}

/// `text` written to `dir/name`; the path.
pub(crate) fn write_file(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path
}

/// A valid assembled rules file: the embedded official ruleset written
/// as the tool's save writes it.
pub(crate) fn valid_file() -> String {
    aatf::assemble_file(aatf::Rules::official().ruleset(), None, None)
}

/// The modified-logic file: `valid_file` with one logic comment changed
/// (the file still loads, its logic is `Modified`).
pub(crate) fn modified_logic_file() -> String {
    valid_file().replacen(
        "// The generic interpreter",
        "// The interpreter, its comment edited",
        1,
    )
}

/// The CUSTOM CHECKS stub every assembled file without checks carries.
pub(crate) const CUSTOM_CHECKS_STUB: &str = "// ==== CUSTOM CHECKS: optional, hand-written; the Ruleset editor keeps this section as it is ====\nfn custom_checks(team) { [] }";

/// The first RE-FILE-03 file: a parse error at line 12 (the comma
/// between `at_most`'s `attacking_prowess` and `defensive_prowess`
/// dropped).
pub(crate) fn parse_error_file() -> String {
    let text = valid_file().replacen(
        "\"attacking_prowess\", \"defensive_prowess\"",
        "\"attacking_prowess\" \"defensive_prowess\"",
        1,
    );
    assert!(
        text.lines()
            .nth(11)
            .unwrap_or_default()
            .contains("\"attacking_prowess\" \"defensive_prowess\""),
        "premise: the edit sits on line 12"
    );
    text
}

/// The second RE-FILE-03 file: `tiers[2].free.tricks` is the string
/// "two".
pub(crate) fn schema_error_file() -> String {
    let mut text = valid_file();
    let at = text.find("id: \"bronze\"").expect("the bronze tier");
    let pos = at + text[at..].find("tricks: 2").expect("its free cards");
    text.replace_range(pos..pos + "tricks: 2".len(), "tricks: \"two\"");
    text
}

/// The third RE-FILE-03 file: the second tier (`silver`) carries the
/// first tier's id (`gold`).
pub(crate) fn duplicate_id_file() -> String {
    let text = valid_file();
    let gold = text.find("id: \"gold\"").expect("the gold tier");
    let silver = text.find("id: \"silver\"").expect("the silver tier");
    assert!(gold < silver, "premise: gold is the first tier");
    text.replacen("id: \"silver\"", "id: \"gold\"", 1)
}
