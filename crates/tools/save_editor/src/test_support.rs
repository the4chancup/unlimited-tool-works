//! The unit tests' scratch folders, under the test binary's own build folder
//! rather than the system Temp.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub(crate) mod cup_save;
pub(crate) mod scratch;

/// A fresh, empty scratch folder for `test` under the test binary's build folder
/// (`target/<profile>/save_editor_tests/<test>`): per checkout, so a mutation run's copy
/// has its own. A passing test's folder is removed when its thread ends; a panicking test's
/// stays for reading and is emptied at the start of its next run.
pub(crate) fn scratch(test: &str) -> PathBuf {
    // A separator would nest the folder, and only the innermost one is removed.
    assert!(
        !test.contains(['/', '\\']),
        "a scratch name holds no path separator: {test}"
    );
    scratch::fresh(scratch_dir(test))
}

/// The path `scratch` gives `test`, without creating it.
fn scratch_dir(test: &str) -> PathBuf {
    // `CARGO_TARGET_TMPDIR` is set for integration tests only, so the build folder
    // comes from the test binary's path (`target/<profile>/deps/<exe>`).
    let deps = std::env::current_exe()
        .expect("current_exe is readable")
        .parent()
        .expect("the test binary sits under target/<profile>/deps")
        .to_path_buf();
    deps.parent()
        .expect("deps sits under target/<profile>")
        .join("save_editor_tests")
        .join(test)
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

/// `recorder`'s step lines so far, read back out of a written bundle's
/// `journal.toml`: the journal's only reader is the bundle. The bundle
/// lands in `dir` as `journal.zip`.
pub(crate) fn journal_lines(recorder: &studio_core::Recorder, dir: &Path) -> Vec<String> {
    let bundle = dir.join("journal.zip");
    recorder.write_bundle(&bundle, "", &[], &[], None).unwrap();
    let mut archive = zip::ZipArchive::new(std::fs::File::open(&bundle).unwrap()).unwrap();
    let mut text = String::new();
    archive
        .by_name("journal.toml")
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    let doc: toml::Table = toml::from_str(&text).unwrap();
    doc.get("step")
        .and_then(toml::Value::as_array)
        .map(|steps| {
            steps
                .iter()
                .map(|step| step["what"].as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// The lines starting `prefix` in `lines` (`journal_lines`' output read
/// for one step kind: `refused `, `could not read `, …).
pub(crate) fn steps_with<'a>(lines: &'a [String], prefix: &str) -> Vec<&'a String> {
    lines
        .iter()
        .filter(|line| line.starts_with(prefix))
        .collect()
}

/// Asserts `path`'s write is the journal's one `wrote` step and that the
/// bundle keeps `entry` byte-equal to the file written.
pub(crate) fn assert_wrote_and_kept(
    recorder: &studio_core::Recorder,
    dir: &Path,
    path: &Path,
    entry: &str,
) {
    let wrote = format!("wrote {}", path.display());
    let lines = journal_lines(recorder, dir);
    assert_eq!(
        lines.iter().filter(|line| **line == wrote).count(),
        1,
        "one wrote step: {lines:?}"
    );
    assert_eq!(
        zip_entry(&dir.join("journal.zip"), entry),
        fs::read(path).unwrap(),
        "the kept output is the file as written"
    );
}

/// Asserts `destination`'s refused write is the journal's one `could not
/// write` step and that the bundle keeps no output: a failed write's
/// bytes are not an output.
pub(crate) fn assert_write_refused(
    recorder: &studio_core::Recorder,
    dir: &Path,
    destination: &Path,
) {
    let lines = journal_lines(recorder, dir);
    let step = format!("could not write {}: ", destination.display());
    let failed = steps_with(&lines, &step);
    assert_eq!(failed.len(), 1, "the refused write records once: {lines:?}");
    let doc: toml::Table = toml::from_str(
        &String::from_utf8(zip_entry(&dir.join("journal.zip"), "journal.toml")).unwrap(),
    )
    .unwrap();
    assert!(
        doc["output"]
            .as_array()
            .expect("the journal's output list")
            .is_empty(),
        "no output kept: {doc:?}"
    );
}

/// `name`'s bytes inside the zip at `path`.
pub(crate) fn zip_entry(path: &Path, name: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut bytes = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
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

/// `name` written as a base CPK under `<pes>/Data/` holding `entries`, as the
/// install ships its `Data` CPKs.
pub(crate) fn base_cpk(pes: &Path, name: &str, entries: CpkEntries<'_>) {
    write_cpk(&pes.join("Data").join(name), entries);
}

/// A `player-dummy.png` as a CPK holds it: a 128 x 128 PNG encoded by
/// `dds_convert`, every pixel `colour`.
pub(crate) fn dummy_png(colour: [u8; 4]) -> Vec<u8> {
    let pixels = colour.repeat(128 * 128);
    dds_convert::encode_png(&pixels, 128, 128).expect("a PNG encodes")
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::thread;

    use super::scratch::fresh;
    use super::scratch_dir;

    fn write_file(folder: &Path, name: &str) -> PathBuf {
        let path = folder.join(name);
        fs::write(&path, name).unwrap();
        path
    }

    #[test]
    fn a_passing_thread_removes_its_folders() {
        let first = scratch_dir("scratch_passing_first");
        let second = scratch_dir("scratch_passing_second");
        let thread_first = first.clone();
        let thread_second = second.clone();
        thread::spawn(move || {
            let first = fresh(thread_first);
            write_file(&first, "first");
            let second = fresh(thread_second);
            write_file(&second, "second");
        })
        .join()
        .unwrap();
        assert!(
            !first.exists(),
            "a passing thread's first folder remains: {}",
            first.display()
        );
        assert!(
            !second.exists(),
            "a passing thread's second folder remains: {}",
            second.display()
        );
    }

    #[test]
    fn a_panicking_thread_keeps_its_folder() {
        let folder = scratch_dir("scratch_panicking_keeps");
        let file = folder.join("kept");
        let thread_folder = folder.clone();
        let thread_file = file.clone();
        let join = thread::spawn(move || {
            fresh(thread_folder);
            fs::write(&thread_file, "kept").unwrap();
            panic!("the panicking-thread test's intentional panic");
        })
        .join();
        assert!(join.is_err(), "the thread did not panic");
        assert!(
            folder.exists(),
            "a panicking thread's folder was removed: {}",
            folder.display()
        );
        assert!(
            file.exists(),
            "a panicking thread's file was removed: {}",
            file.display()
        );
        fs::remove_dir_all(&folder).unwrap();
    }

    /// The view tests' fixture helper in miniature: a fresh folder, one file written inside it,
    /// and the file's path returned.
    fn fixture_like(folder: PathBuf) -> PathBuf {
        let folder = fresh(folder);
        assert!(
            folder.exists(),
            "fresh returned no folder: {}",
            folder.display()
        );
        write_file(&folder, "fixture")
    }

    #[test]
    fn a_path_a_helper_returns_outlives_the_helper() {
        let folder = scratch_dir("scratch_helper_path");
        let thread_folder = folder.clone();
        thread::spawn(move || {
            let path = fixture_like(thread_folder);
            assert!(
                path.exists(),
                "the helper's file is gone before the thread ends: {}",
                path.display()
            );
        })
        .join()
        .unwrap();
        assert!(
            !folder.exists(),
            "the helper's folder remains after its thread ends: {}",
            folder.display()
        );
    }
}
