//! The repro journal: what a tool's model went through since the window
//! opened — every step that changed it, the files it read and the ones it
//! wrote — kept in memory so "Report a problem" can zip them, the member's
//! own words and the log into one bundle to post where problems are
//! reported. Only steps that changed the model are recorded, never input
//! events: every effect on what the member sees passes through the model,
//! so the steps replay the session anywhere.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// One tool's recorded steps, kept inputs and kept outputs so far.
/// `Recorder` shares it: every clone records into the same journal.
#[derive(Clone)]
struct Journal {
    /// When the recorder was made; a step's `seconds` measures from it.
    started: Instant,
    /// The recorded steps, in the order they happened.
    steps: Vec<Step>,
    /// Every file a tool kept as it was read.
    inputs: Vec<KeptFile>,
    /// The last file each tool wrote per path: a later write replaces it.
    outputs: Vec<KeptFile>,
}

/// `started` needs a `now`, which no derive can give.
impl Journal {
    fn new() -> Journal {
        Journal {
            started: Instant::now(),
            steps: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }
}

/// One recorded step: when it happened, which tool recorded it, what it did.
#[derive(Clone)]
struct Step {
    /// Seconds since the recorder was made.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(dead_code, reason = "read only by the native bundle writer")
    )]
    seconds: f64,
    /// The recording tool's id.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(dead_code, reason = "read only by the native bundle writer")
    )]
    tool: &'static str,
    /// What the step did, one line.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(dead_code, reason = "read only by the native bundle writer")
    )]
    what: String,
}

/// A file kept in memory as it was: the tool that kept it, its path and
/// its bytes.
#[derive(Clone)]
struct KeptFile {
    /// The recording tool's id.
    tool: &'static str,
    /// The path the file was read from or written to.
    path: PathBuf,
    /// The file's bytes.
    bytes: Arc<[u8]>,
}

/// A tool's repro journal: the model steps it records and the files it
/// keeps, shared by every clone and every `ToolContext`, so the bundle a
/// "Report a problem" writes covers the whole session.
#[derive(Clone)]
pub struct Recorder {
    journal: Arc<Mutex<Journal>>,
}

impl Default for Recorder {
    fn default() -> Recorder {
        Recorder {
            journal: Arc::new(Mutex::new(Journal::new())),
        }
    }
}

/// `Recorder(..)`: the journal's file bytes have no business in a debug line.
impl fmt::Debug for Recorder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Recorder(..)")
    }
}

impl Recorder {
    /// Appends one step `what`: `tool`'s line, timed from the recorder's
    /// start.
    pub fn step(&self, tool: &'static str, what: impl Into<String>) {
        let mut journal = self.journal.lock().unwrap();
        let seconds = journal.started.elapsed().as_secs_f64();
        journal.steps.push(Step {
            seconds,
            tool,
            what: what.into(),
        });
    }

    /// Keeps `bytes` as the file `tool` read at `path`. The bundle's
    /// inputs hold every one, same path or not: what the tool started
    /// from is the part a saved-over file would destroy.
    pub fn keep_input(&self, tool: &'static str, path: &Path, bytes: Arc<[u8]>) {
        self.journal.lock().unwrap().inputs.push(KeptFile {
            tool,
            path: path.to_path_buf(),
            bytes,
        });
    }

    /// Keeps `bytes` as the file `tool` wrote at `path`. The bundle
    /// holds the last write per path — the one the member has — so a
    /// kept write replaces an earlier one for the same tool and path.
    pub fn keep_output(&self, tool: &'static str, path: &Path, bytes: Arc<[u8]>) {
        let mut journal = self.journal.lock().unwrap();
        match journal
            .outputs
            .iter_mut()
            .find(|kept| kept.tool == tool && kept.path == path)
        {
            Some(kept) => kept.bytes = bytes,
            None => journal.outputs.push(KeptFile {
                tool,
                path: path.to_path_buf(),
                bytes,
            }),
        }
    }

    /// Writes the bundle to `out`: one zip holding `journal.toml` (the
    /// suite's version as the window shows it, the OS, every step, every
    /// kept file's redacted path, size and SHA-256 beside its zip entry's
    /// name, the `files` that could not be read under `missing`, and the
    /// `state` attachments' names), the inputs under `inputs/` and the
    /// last outputs under `outputs/` (each named `<index>-<file name>`
    /// so two files of one name never collide), the member's `words` as
    /// `words.txt`, each of `files` read from disk under its given name,
    /// and each of `state` under `state/<name>` — bytes the caller holds,
    /// such as a tool's current state written as its format. Every path
    /// and step line, and every UTF-8 attachment, is written with the
    /// home folder as `~`. The zip is written to a sibling temp and
    /// renamed over `out`, so a failed write never leaves a half zip.
    /// The journal is cloned under its lock and written from the clone:
    /// the lock covers a cheap copy of `Arc`s, and a failed or panicking
    /// write can neither block nor poison the recorders.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write_bundle(
        &self,
        out: &Path,
        words: &str,
        files: &[(&str, &Path)],
        state: &[(&str, &[u8])],
        home: Option<&str>,
    ) -> anyhow::Result<()> {
        let journal = self.journal.lock().unwrap().clone();
        bundle::write(&journal, out, words, files, state, home)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod bundle {
    //! The bundle writer's file: `journal.toml`'s shape and the zip.

    use std::fs;
    use std::io::Write;
    use std::path::Path;

    use anyhow::Context;
    use serde::Serialize;
    use sha2::{Digest, Sha256};

    use super::{Journal, KeptFile};
    use crate::redact::redact_home;

    /// `journal.toml` as serialized: the suite's version, the OS, the
    /// extra files that could not be read under `missing`, the `state`
    /// attachments' names, then every step and every kept file as an
    /// array of tables.
    #[derive(Serialize)]
    struct Doc {
        /// The suite's version as the window shows it
        /// (`crate::shell::version_label` — the stage and a debug build's
        /// mark included).
        version: String,
        /// The OS the window ran on (`std::env::consts::OS`).
        os: &'static str,
        /// The `files` names that could not be read; absent when empty.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        missing: Vec<String>,
        /// The `state` attachments' names, as given; absent when empty.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        state: Vec<String>,
        /// Every recorded step as `[[step]]`.
        #[serde(rename = "step")]
        steps: Vec<StepDoc>,
        /// Every kept input as `[[input]]`.
        #[serde(rename = "input")]
        inputs: Vec<FileDoc>,
        /// Every kept output as `[[output]]`.
        #[serde(rename = "output")]
        outputs: Vec<FileDoc>,
    }

    /// One `[[step]]` row: seconds since the recorder's start, the tool
    /// and its line.
    #[derive(Serialize)]
    struct StepDoc {
        /// Seconds since the recorder was made.
        seconds: f64,
        /// The recording tool's id.
        tool: &'static str,
        /// The step's line, the home folder written as `~`.
        what: String,
    }

    /// One `[[input]]`/`[[output]]` row: what the kept file is and where
    /// it sits in the zip.
    #[derive(Serialize)]
    struct FileDoc {
        /// The tool that kept it.
        tool: &'static str,
        /// Its path, the home folder written as `~`.
        path: String,
        /// Its byte length.
        size: usize,
        /// Its SHA-256, lowercase hex.
        sha256: String,
        /// Its name in the zip (`<dir>/<index>-<file name>`).
        entry: String,
    }

    /// `kept`'s zip entry under `dir` (`inputs`/`outputs`): the index and
    /// the file name, so two files of one name never collide.
    fn entry(dir: &str, index: usize, kept: &KeptFile) -> String {
        let name = kept
            .path
            .file_name()
            .unwrap_or_else(|| kept.path.as_os_str())
            .to_string_lossy();
        format!("{dir}/{index}-{name}")
    }

    /// `bytes`' SHA-256, lowercase hex.
    fn sha256(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// `bytes` with the home folder written as `~` when they are UTF-8
    /// text; the bytes as they are otherwise (a binary carries no
    /// readable path and must not be rewritten).
    fn redacted(bytes: &[u8], home: Option<&str>) -> Vec<u8> {
        match std::str::from_utf8(bytes) {
            Ok(text) => redact_home(text, home).into_bytes(),
            Err(_) => bytes.to_vec(),
        }
    }

    /// `kept_files`' `journal.toml` rows; `dir` names their entries.
    fn file_docs(dir: &str, kept_files: &[KeptFile], home: Option<&str>) -> Vec<FileDoc> {
        kept_files
            .iter()
            .enumerate()
            .map(|(index, kept)| FileDoc {
                tool: kept.tool,
                path: redact_home(&kept.path.display().to_string(), home),
                size: kept.bytes.len(),
                sha256: sha256(&kept.bytes),
                entry: entry(dir, index, kept),
            })
            .collect()
    }

    /// Drops the leftover temp zip of a failed write: the failure, not
    /// the litter, is what the caller sees. Nothing logged when `tmp`
    /// never got made — nothing stays either way.
    fn remove_tmp(tmp: &Path) {
        if tmp.exists()
            && let Err(error) = fs::remove_file(tmp)
        {
            log::debug!(
                "{}: the failed bundle's temp file stays ({error})",
                tmp.display()
            );
        }
    }

    /// Writes the bundle; see `Recorder::write_bundle`.
    pub(super) fn write(
        journal: &Journal,
        out: &Path,
        words: &str,
        files: &[(&str, &Path)],
        state: &[(&str, &[u8])],
        home: Option<&str>,
    ) -> anyhow::Result<()> {
        // The extras are read before `journal.toml` is built, because an
        // unreadable one lands in its `missing` list.
        let mut extras = Vec::new();
        let mut missing = Vec::new();
        for (name, path) in files {
            match fs::read(path) {
                Ok(bytes) => extras.push((*name, bytes)),
                Err(_) => missing.push((*name).to_owned()),
            }
        }
        let doc = toml::to_string_pretty(&Doc {
            version: crate::shell::version_label(),
            os: std::env::consts::OS,
            missing,
            state: state.iter().map(|(name, _)| (*name).to_owned()).collect(),
            steps: journal
                .steps
                .iter()
                .map(|step| StepDoc {
                    seconds: step.seconds,
                    tool: step.tool,
                    what: redact_home(&step.what, home),
                })
                .collect(),
            inputs: file_docs("inputs", &journal.inputs, home),
            outputs: file_docs("outputs", &journal.outputs, home),
        })
        .context("cannot serialize the journal")?;
        let mut tmp = out.to_path_buf();
        tmp.as_mut_os_string().push(".tmp");
        if let Err(error) = write_zip(&tmp, &doc, journal, words, &extras, state, home) {
            // A half zip at `out`'s side is litter, not a half `out`.
            remove_tmp(&tmp);
            return Err(error);
        }
        if let Err(error) = fs::rename(&tmp, out) {
            remove_tmp(&tmp);
            return Err(error)
                .with_context(|| format!("{}: cannot move the bundle into place", tmp.display()));
        }
        Ok(())
    }

    /// The zip's contents into `tmp`: `journal.toml`, the kept inputs and
    /// outputs, `words.txt`, the extras and the `state` attachments —
    /// every attachment's bytes redacted when they are UTF-8 text.
    fn write_zip(
        tmp: &Path,
        doc: &str,
        journal: &Journal,
        words: &str,
        extras: &[(&str, Vec<u8>)],
        state: &[(&str, &[u8])],
        home: Option<&str>,
    ) -> anyhow::Result<()> {
        let file = fs::File::create(tmp)
            .with_context(|| format!("{}: cannot write the bundle", tmp.display()))?;
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let mut put = |name: &str, bytes: &[u8]| -> anyhow::Result<()> {
            writer
                .start_file(name, options)
                .with_context(|| format!("the bundle's {name}"))?;
            writer
                .write_all(bytes)
                .with_context(|| format!("the bundle's {name}"))?;
            Ok(())
        };
        put("journal.toml", doc.as_bytes())?;
        for (index, kept) in journal.inputs.iter().enumerate() {
            put(&entry("inputs", index, kept), &kept.bytes)?;
        }
        for (index, kept) in journal.outputs.iter().enumerate() {
            put(&entry("outputs", index, kept), &kept.bytes)?;
        }
        put("words.txt", redact_home(words, home).as_bytes())?;
        for (name, bytes) in extras {
            put(name, &redacted(bytes, home))?;
        }
        for (name, bytes) in state {
            put(&format!("state/{name}"), &redacted(bytes, home))?;
        }
        writer
            .finish()
            .with_context(|| format!("{}: cannot finish the bundle", tmp.display()))?;
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::fs;
    use std::io::Read;

    use super::*;

    /// A fresh scratch folder for `test` under the system temp.
    pub(crate) fn scratch(test: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("studio_core_repro_{test}_{}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The zip's entry names at `path`, in write order.
    fn zip_names(path: &Path) -> Vec<String> {
        let file = fs::File::open(path).unwrap();
        let archive = zip::ZipArchive::new(file).unwrap();
        archive
            .file_names()
            .map(|name| name.unwrap().into_owned())
            .collect()
    }

    /// `name`'s bytes inside the zip at `path`.
    fn zip_bytes(path: &Path, name: &str) -> Vec<u8> {
        let file = fs::File::open(path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut bytes = Vec::new();
        archive
            .by_name(name)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        bytes
    }

    /// `name`'s text inside the zip at `path`.
    pub(crate) fn zip_text(path: &Path, name: &str) -> String {
        String::from_utf8(zip_bytes(path, name)).unwrap()
    }

    /// `journal`'s `journal.toml` parsed.
    pub(crate) fn journal_doc(text: &str) -> toml::Table {
        toml::from_str(text).unwrap()
    }

    // A bundle holds the journal (version, OS, the steps in order and the
    // kept files' redacted paths, sizes, SHA-256s and entry names), the
    // inputs and the last outputs under `inputs/`/`outputs/`, the member's
    // words and the extras that could be read, an unreadable one listed
    // under `missing`.
    #[test]
    fn a_bundle_holds_the_journal_the_kept_files_the_words_and_the_extras() {
        let dir = scratch("bundle");
        let recorder = Recorder::default();
        recorder.step(
            "save-editor",
            "opened /home/member/EDIT00000000 (PES 2021, 10 bytes)",
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
        recorder.step("save-editor", "player 70101: Height 183 → 184");
        recorder.keep_input(
            "save-editor",
            Path::new("/home/member/EDIT00000000"),
            Arc::from(&b"SAVE_BYTES"[..]),
        );
        recorder.keep_output(
            "save-editor",
            Path::new("/home/member/out/EDIT00000000"),
            Arc::from(&b"OLD"[..]),
        );
        recorder.keep_output(
            "save-editor",
            Path::new("/home/member/out/EDIT00000000"),
            Arc::from(&b"NEW_BYTES"[..]),
        );
        // The same path written by another tool is a second output, not a
        // replacement.
        recorder.keep_output(
            "team-compiler",
            Path::new("/home/member/out/EDIT00000000"),
            Arc::from(&b"TEAM"[..]),
        );
        recorder.keep_output(
            "save-editor",
            Path::new("out/EDIT00000000"),
            Arc::from(&b"REL"[..]),
        );
        let settings = dir.join("settings.toml");
        fs::write(&settings, "theme = \"light\"\n").unwrap();
        let out = dir.join("repro.zip");

        recorder
            .write_bundle(
                &out,
                "the member's words",
                &[
                    ("settings.toml", settings.as_path()),
                    ("utw.log", dir.join("no-such.log").as_path()),
                ],
                &[("current.save", &b"CURRENT"[..])],
                Some("/home/member"),
            )
            .unwrap();

        assert_eq!(
            zip_names(&out),
            [
                "journal.toml",
                "inputs/0-EDIT00000000",
                "outputs/0-EDIT00000000",
                "outputs/1-EDIT00000000",
                "outputs/2-EDIT00000000",
                "words.txt",
                "settings.toml",
                "state/current.save",
            ]
        );
        assert_eq!(zip_bytes(&out, "inputs/0-EDIT00000000"), b"SAVE_BYTES");
        assert_eq!(zip_bytes(&out, "outputs/0-EDIT00000000"), b"NEW_BYTES");
        assert_eq!(zip_bytes(&out, "outputs/1-EDIT00000000"), b"TEAM");
        assert_eq!(zip_bytes(&out, "outputs/2-EDIT00000000"), b"REL");
        assert_eq!(zip_bytes(&out, "state/current.save"), b"CURRENT");
        assert_eq!(zip_text(&out, "words.txt"), "the member's words");
        assert_eq!(zip_text(&out, "settings.toml"), "theme = \"light\"\n");

        let journal = zip_text(&out, "journal.toml");
        let doc = journal_doc(&journal);
        assert_eq!(
            doc["version"].as_str().unwrap(),
            crate::shell::version_label()
        );
        assert_eq!(doc["os"].as_str(), Some(std::env::consts::OS));
        let missing = doc["missing"].as_array().unwrap();
        assert_eq!(missing.len(), 1, "{journal}");
        assert_eq!(
            missing[0].as_str(),
            Some("utw.log"),
            "the unreadable extra is named under `missing`: {journal}"
        );
        let state = doc["state"].as_array().unwrap();
        assert_eq!(state.len(), 1, "{journal}");
        assert_eq!(
            state[0].as_str(),
            Some("current.save"),
            "the attachment is named under `state`: {journal}"
        );

        let steps = doc["step"].as_array().unwrap();
        assert_eq!(steps.len(), 2, "{journal}");
        assert_eq!(steps[0]["tool"].as_str(), Some("save-editor"));
        assert_eq!(
            steps[0]["what"].as_str(),
            Some("opened ~/EDIT00000000 (PES 2021, 10 bytes)"),
            "the home folder is written as ~: {journal}"
        );
        assert_eq!(
            steps[1]["what"].as_str(),
            Some("player 70101: Height 183 → 184")
        );
        assert!(
            steps[1]["seconds"].as_float().unwrap() > steps[0]["seconds"].as_float().unwrap(),
            "the later step's seconds exceed the first's"
        );

        let inputs = doc["input"].as_array().unwrap();
        assert_eq!(inputs.len(), 1, "{journal}");
        assert_eq!(inputs[0]["tool"].as_str(), Some("save-editor"));
        assert_eq!(inputs[0]["path"].as_str(), Some("~/EDIT00000000"));
        assert_eq!(inputs[0]["size"].as_integer(), Some(10));
        assert_eq!(
            inputs[0]["sha256"].as_str().unwrap(),
            "d3fb2163b52a43f0ef3f73b889c4f6bc925950da789e4be415ac49311d86bb66",
            "SHA-256 of b\"SAVE_BYTES\""
        );
        assert_eq!(inputs[0]["entry"].as_str(), Some("inputs/0-EDIT00000000"));

        let outputs = doc["output"].as_array().unwrap();
        assert_eq!(
            outputs.len(),
            3,
            "the second write to one path replaces the first; another tool's is its own: {journal}"
        );
        assert_eq!(outputs[0]["tool"].as_str(), Some("save-editor"));
        assert_eq!(
            outputs[0]["path"].as_str(),
            Some("~/out/EDIT00000000"),
            "the output's path is redacted too: {journal}"
        );
        assert_eq!(outputs[0]["size"].as_integer(), Some(9));
        assert_eq!(
            outputs[0]["sha256"].as_str().unwrap(),
            "bcb9189612cc6778b450aa557242d7cf09f556ba215ee875b3e4a16f6c681bf4",
            "SHA-256 of b\"NEW_BYTES\""
        );
        assert_eq!(outputs[0]["entry"].as_str(), Some("outputs/0-EDIT00000000"));
        assert_eq!(outputs[1]["tool"].as_str(), Some("team-compiler"));
        assert_eq!(
            outputs[1]["path"].as_str(),
            Some("~/out/EDIT00000000"),
            "the other tool's same path is its own row: {journal}"
        );
        assert_eq!(outputs[1]["size"].as_integer(), Some(4));
        assert_eq!(
            outputs[1]["sha256"].as_str().unwrap(),
            "f404d4ef53431418a111b361b24b6d805798314477493738d95eb6ca347ea17b",
            "SHA-256 of b\"TEAM\""
        );
        assert_eq!(outputs[1]["entry"].as_str(), Some("outputs/1-EDIT00000000"));
        assert_eq!(outputs[2]["tool"].as_str(), Some("save-editor"));
        assert_eq!(
            outputs[2]["path"].as_str(),
            Some("out/EDIT00000000"),
            "a relative path is written as it is"
        );
        assert_eq!(outputs[2]["entry"].as_str(), Some("outputs/2-EDIT00000000"));
    }

    // The member's words, an extra and an attachment whose bytes are
    // UTF-8 are written with the home folder as `~`; a binary goes
    // verbatim.
    #[test]
    fn a_utf8_extra_is_written_with_the_home_folder_as_a_tilde() {
        let dir = scratch("redact_extra");
        let recorder = Recorder::default();
        let settings = dir.join("settings.toml");
        fs::write(
            &settings,
            "pes_folder_path = 'C:/Users/Alice/PES'\ndir = \"C:\\\\Users\\\\Alice\\\\Games\"\n",
        )
        .unwrap();
        let out = dir.join("repro.zip");
        recorder
            .write_bundle(
                &out,
                "I copied the save to C:/Users/Alice/Desktop",
                &[("settings.toml", settings.as_path())],
                &[("note.txt", &b"the save is at C:/Users/Alice/saves"[..])],
                Some("C:/Users/Alice"),
            )
            .unwrap();

        assert_eq!(
            zip_text(&out, "words.txt"),
            "I copied the save to ~/Desktop",
            "the member's own text is redacted too"
        );
        // The doubled backslashes of a basic string's escaped separator
        // spell the home folder only where paths can be written that way.
        let quoted = if cfg!(windows) {
            "dir = \"~\\\\Games\""
        } else {
            "dir = \"C:\\\\Users\\\\Alice\\\\Games\""
        };
        assert_eq!(
            zip_text(&out, "settings.toml"),
            format!("pes_folder_path = '~/PES'\n{quoted}\n"),
            "the extra's text is redacted"
        );
        assert_eq!(
            zip_text(&out, "state/note.txt"),
            "the save is at ~/saves",
            "the attachment's too"
        );
    }

    // A file kept as an input stays kept: two reads of one path are two
    // inputs — the re-open's first read is the part the bundle exists to
    // keep, unlike an output, where the last write is the one kept.
    #[test]
    fn two_reads_of_one_path_are_two_inputs() {
        let dir = scratch("reinput");
        let recorder = Recorder::default();
        recorder.keep_input(
            "save-editor",
            Path::new("a/EDIT00000000"),
            Arc::from(&b"FIRST"[..]),
        );
        recorder.keep_input(
            "save-editor",
            Path::new("a/EDIT00000000"),
            Arc::from(&b"SECOND"[..]),
        );
        let out = dir.join("repro.zip");
        recorder.write_bundle(&out, "", &[], &[], None).unwrap();

        assert_eq!(zip_bytes(&out, "inputs/0-EDIT00000000"), b"FIRST");
        assert_eq!(zip_bytes(&out, "inputs/1-EDIT00000000"), b"SECOND");
        let journal = zip_text(&out, "journal.toml");
        assert_eq!(
            journal_doc(&journal)["input"].as_array().unwrap().len(),
            2,
            "{journal}"
        );
    }

    // The journal's file bytes have no business in a debug line: the
    // impl is the `Recorder(..)` placeholder whatever the recorder holds.
    #[test]
    fn a_recorders_debug_is_its_placeholder() {
        let recorder = Recorder::default();
        recorder.step("save-editor", "a step");
        recorder.keep_input(
            "save-editor",
            Path::new("a/EDIT00000000"),
            Arc::from(&b"BYTES"[..]),
        );
        assert_eq!(format!("{recorder:?}"), "Recorder(..)");
    }

    // `out` a directory: the write is an error and the temp sibling is
    // gone, never a half bundle or a misleading success.
    #[test]
    fn a_bundle_write_refuses_a_directory_and_cleans_up() {
        let dir = scratch("out_is_dir");
        let recorder = Recorder::default();
        recorder.step("save-editor", "a step");

        let error = recorder
            .write_bundle(&dir, "", &[], &[], None)
            .expect_err("a directory is not a bundle");
        assert!(
            format!("{error:#}").contains("cannot move the bundle into place"),
            "{error:#}"
        );
        let tmp = dir.with_file_name(format!(
            "{}.tmp",
            dir.file_name().unwrap().to_string_lossy()
        ));
        assert!(!tmp.exists(), "the temp sibling is removed");
    }

    // An attachment whose bytes are not UTF-8 lands byte-equal: a binary
    // (a save's current state) is never rewritten by the redaction.
    #[test]
    fn a_binary_attachment_lands_byte_equal() {
        let dir = scratch("binary_extra");
        let recorder = Recorder::default();
        let packed = dir.join("packed.bin");
        let bytes: &[u8] = b"\xFF\x00\xFE\x80abc";
        fs::write(&packed, bytes).unwrap();
        let out = dir.join("repro.zip");
        recorder
            .write_bundle(
                &out,
                "",
                &[("packed.bin", packed.as_path())],
                &[("state.bin", &b"\xFF\x00\xFE"[..])],
                Some("C:/Users/Alice"),
            )
            .unwrap();

        assert_eq!(zip_bytes(&out, "packed.bin"), bytes, "verbatim");
        assert_eq!(
            zip_bytes(&out, "state/state.bin"),
            b"\xFF\x00\xFE",
            "verbatim"
        );
    }
}
