//! The unit tests' scratch folders, under the test binary's own build folder
//! rather than the system Temp.

use std::fs;
use std::path::{Path, PathBuf};

/// A fresh, empty scratch folder for `test` under the test binary's build folder
/// (`target/<profile>/save_editor_tests/<test>`): per checkout, so a mutation run's copy
/// has its own, and emptied at the start of each run, so no process id is needed.
pub(crate) fn scratch(test: &str) -> PathBuf {
    // `CARGO_TARGET_TMPDIR` is set for integration tests only, so the build folder
    // comes from the test binary's path (`target/<profile>/deps/<exe>`).
    let deps = std::env::current_exe()
        .expect("current_exe is readable")
        .parent()
        .expect("the test binary sits under target/<profile>/deps")
        .to_path_buf();
    let dir = deps
        .parent()
        .expect("deps sits under target/<profile>")
        .join("save_editor_tests")
        .join(test);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("the scratch folder is removable");
    }
    std::fs::create_dir_all(&dir).expect("the scratch folder is creatable");
    dir
}

/// A `ToolContext` for a `tool.view`/`settings_view`/CLI test, with the
/// receivers its sends land on: `settings` is the context's own except
/// `pes_folder_path`, set to a folder under `dir` (the test's scratch).
/// Its default names the machine's real `Pro Evolution Soccer 20**`
/// install, which `Tool::view` points the portraits worker at every frame
/// (CONTRIBUTING.md "Testing": a test never reaches the developer's own
/// files; `studio_core`'s `test_settings` does the same). `exe_dir` and
/// `data_dir` sit under `dir` too.
pub(crate) fn tool_context(
    dir: &Path,
    settings: studio_core::Settings,
) -> (
    studio_core::ToolContext,
    crossbeam_channel::Receiver<studio_core::PipelineEventEnvelope>,
    crossbeam_channel::Receiver<studio_core::ShellRequest>,
) {
    let (ctx, _settings, events_rx, requests_rx) = shared_tool_context(dir, settings);
    (ctx, events_rx, requests_rx)
}

/// `tool_context` with a handle on the context's settings, which the test writes between
/// calls as the shell does (the sidebar's selector writes `common.pes_version`).
pub(crate) fn shared_tool_context(
    dir: &Path,
    mut settings: studio_core::Settings,
) -> (
    studio_core::ToolContext,
    std::sync::Arc<std::sync::Mutex<studio_core::Settings>>,
    crossbeam_channel::Receiver<studio_core::PipelineEventEnvelope>,
    crossbeam_channel::Receiver<studio_core::ShellRequest>,
) {
    settings.common.pes_folder_path = dir.join("pes").display().to_string();
    let settings = std::sync::Arc::new(std::sync::Mutex::new(settings));
    let (events_tx, events_rx) = crossbeam_channel::unbounded();
    let (requests_tx, requests_rx) = crossbeam_channel::unbounded();
    let ctx = studio_core::ToolContext::new(
        std::sync::Arc::clone(&settings),
        studio_core::AppPaths {
            exe_dir: dir.join("exe"),
            data_dir: Some(dir.to_path_buf()),
        },
        events_tx,
        requests_tx,
    );
    (ctx, settings, events_rx, requests_rx)
}

/// The salt the written saves encrypt with (the CLI tests' own).
pub(crate) const TEST_SALT: [u8; 320] = [7; 320];

/// `save-editor <args>`'s stdout run in-process over `dir`: the run's
/// events as `EventLines` writes them, newline-separated. The context is
/// `tool_context`'s, so nothing reaches the developer's files.
pub(crate) fn cli_stdout(dir: &Path, args: &[&str]) -> String {
    let mut settings = studio_core::Settings::default();
    settings.merge_defaults(crate::messages::TOOL_ID, &crate::settings::default_table());
    let (ctx, events_rx, _requests_rx) = tool_context(dir, settings);
    let matches = crate::cli::command()
        .try_get_matches_from(std::iter::once("save-editor").chain(args.iter().copied()))
        .expect("the arguments parse");
    crate::cli::run(&matches, &ctx, crate::running_probe()).expect("the command runs");
    drop(ctx);
    let mut lines = studio_core::EventLines::new();
    let mut out = String::new();
    for envelope in events_rx.iter() {
        if let Some(line) = lines.line(&envelope.event) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

/// A `DpFileList.bin` listing `names` in load order: a 16-byte header whose second word is the
/// count, then one 48-byte NUL-padded record per CPK.
pub(crate) fn dp_file_list(names: &[&str]) -> Vec<u8> {
    let mut bytes = vec![0; 4];
    bytes.extend(u32::try_from(names.len()).unwrap().to_le_bytes());
    bytes.extend([0; 8]);
    for name in names {
        let mut record = [0; 48];
        record[..name.len()].copy_from_slice(name.as_bytes());
        bytes.extend(record);
    }
    bytes
}

/// A CPK's entries as the tests give them (`(entry path, bytes)`).
pub(crate) type CpkEntries<'a> = &'a [(&'a str, &'a [u8])];

/// `path` written as a CPK holding `entries` (`(entry path, bytes)`).
pub(crate) fn write_cpk(path: &Path, entries: CpkEntries<'_>) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut cpk = cpk::CpkWriter::new(fs::File::create(path).unwrap(), "test").unwrap();
    for (entry, bytes) in entries {
        cpk.add(entry, bytes, None).unwrap();
    }
    cpk.finish().unwrap();
}

/// A PES folder under `test`'s scratch holding `download/DpFileList.bin`, listing `cpks`' names
/// in load order, each written with its `entries` (`(entry path, bytes)`).
pub(crate) fn pes_folder(test: &str, cpks: &[(&str, CpkEntries<'_>)]) -> PathBuf {
    let download = scratch(test).join("PES/download");
    fs::create_dir_all(&download).unwrap();
    let names: Vec<&str> = cpks.iter().map(|(name, _)| *name).collect();
    fs::write(download.join("DpFileList.bin"), dp_file_list(&names)).unwrap();
    for (name, entries) in cpks {
        write_cpk(&download.join(name), entries);
    }
    download.parent().unwrap().to_path_buf()
}
