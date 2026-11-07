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
    /// suite's version, the OS, every step, every kept file's redacted
    /// path, size and SHA-256 beside its zip entry's name, and the
    /// `files` that could not be read under `missing`), the inputs under
    /// `inputs/` and the last outputs under `outputs/` (each named
    /// `<index>-<file name>` so two files of one name never collide), the
    /// member's `words` as `words.txt`, and each of `files` read from
    /// disk under its given name. Every path and step line is written
    /// with the home folder as `~`. The zip is written to a sibling temp
    /// and renamed over `out`, so a failed write never leaves a half zip.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write_bundle(
        &self,
        out: &Path,
        words: &str,
        files: &[(&str, &Path)],
        home: Option<&str>,
    ) -> anyhow::Result<()> {
        bundle::write(&self.journal.lock().unwrap(), out, words, files, home)
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
    /// extra files that could not be read under `missing`, then every
    /// step and every kept file as an array of tables.
    #[derive(Serialize)]
    struct Doc {
        /// The suite's version (`env!("CARGO_PKG_VERSION")`).
        version: &'static str,
        /// The OS the window ran on (`std::env::consts::OS`).
        os: &'static str,
        /// The `files` names that could not be read; absent when empty.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        missing: Vec<String>,
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

    /// Writes the bundle; see `Recorder::write_bundle`.
    pub(super) fn write(
        journal: &Journal,
        out: &Path,
        words: &str,
        files: &[(&str, &Path)],
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
            version: env!("CARGO_PKG_VERSION"),
            os: std::env::consts::OS,
            missing,
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
        if let Err(error) = write_zip(&tmp, &doc, journal, words, &extras) {
            // A half zip at `out`'s side is litter, not a half `out`.
            if let Err(remove) = fs::remove_file(&tmp) {
                log::debug!(
                    "{}: the failed bundle's temp file stays ({remove})",
                    tmp.display()
                );
            }
            return Err(error);
        }
        fs::rename(&tmp, out)
            .with_context(|| format!("{}: cannot move the bundle into place", tmp.display()))
    }

    /// The zip's contents into `tmp`: `journal.toml`, the kept inputs and
    /// outputs, `words.txt` and the extras.
    fn write_zip(
        tmp: &Path,
        doc: &str,
        journal: &Journal,
        words: &str,
        extras: &[(&str, Vec<u8>)],
    ) -> anyhow::Result<()> {
        let file = fs::File::create(tmp)
            .with_context(|| format!("{}: cannot write the bundle", tmp.display()))?;
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let mut put = |name: &str, bytes: &[u8]| -> anyhow::Result<()> {
            writer.start_file(name, options)?;
            writer.write_all(bytes)?;
            Ok(())
        };
        put("journal.toml", doc.as_bytes())?;
        for (index, kept) in journal.inputs.iter().enumerate() {
            put(&entry("inputs", index, kept), &kept.bytes)?;
        }
        for (index, kept) in journal.outputs.iter().enumerate() {
            put(&entry("outputs", index, kept), &kept.bytes)?;
        }
        put("words.txt", words.as_bytes())?;
        for (name, bytes) in extras {
            put(name, bytes)?;
        }
        writer.finish()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Read;

    use sha2::{Digest, Sha256};

    use super::*;

    /// A fresh scratch folder for `test` under the system temp.
    fn scratch(test: &str) -> PathBuf {
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
    fn zip_text(path: &Path, name: &str) -> String {
        String::from_utf8(zip_bytes(path, name)).unwrap()
    }

    /// `journal`'s `journal.toml` parsed.
    fn journal_doc(text: &str) -> toml::Table {
        toml::from_str(text).unwrap()
    }

    /// `bytes`' SHA-256, lowercase hex.
    fn sha256_hex(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
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
        recorder.step("save-editor", "player 70101: Height 183 → 184");
        recorder.keep_input(
            "save-editor",
            Path::new("/home/member/EDIT00000000"),
            Arc::from(&b"SAVE_BYTES"[..]),
        );
        recorder.keep_output(
            "save-editor",
            Path::new("out/EDIT00000000"),
            Arc::from(&b"OLD"[..]),
        );
        recorder.keep_output(
            "save-editor",
            Path::new("out/EDIT00000000"),
            Arc::from(&b"NEW_BYTES"[..]),
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
                Some("/home/member"),
            )
            .unwrap();

        assert_eq!(
            zip_names(&out),
            [
                "journal.toml",
                "inputs/0-EDIT00000000",
                "outputs/0-EDIT00000000",
                "words.txt",
                "settings.toml",
            ]
        );
        assert_eq!(zip_bytes(&out, "inputs/0-EDIT00000000"), b"SAVE_BYTES");
        assert_eq!(zip_bytes(&out, "outputs/0-EDIT00000000"), b"NEW_BYTES");
        assert_eq!(zip_text(&out, "words.txt"), "the member's words");
        assert_eq!(zip_text(&out, "settings.toml"), "theme = \"light\"\n");

        let journal = zip_text(&out, "journal.toml");
        let doc = journal_doc(&journal);
        assert_eq!(doc["version"].as_str(), Some(env!("CARGO_PKG_VERSION")));
        assert_eq!(doc["os"].as_str(), Some(std::env::consts::OS));
        assert_eq!(
            doc["missing"].as_array().unwrap()[0].as_str(),
            Some("utw.log"),
            "the unreadable extra is named under `missing`: {journal}"
        );
        assert!(doc.get("missing").is_some(), "missing: {journal}");

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
        assert!(steps[0]["seconds"].as_float().is_some_and(|s| s >= 0.0));

        let inputs = doc["input"].as_array().unwrap();
        assert_eq!(inputs.len(), 1, "{journal}");
        assert_eq!(inputs[0]["tool"].as_str(), Some("save-editor"));
        assert_eq!(inputs[0]["path"].as_str(), Some("~/EDIT00000000"));
        assert_eq!(inputs[0]["size"].as_integer(), Some(10));
        assert_eq!(
            inputs[0]["sha256"].as_str().unwrap(),
            sha256_hex(b"SAVE_BYTES")
        );
        assert_eq!(inputs[0]["entry"].as_str(), Some("inputs/0-EDIT00000000"));

        let outputs = doc["output"].as_array().unwrap();
        assert_eq!(
            outputs.len(),
            1,
            "the second write to one path replaces the first: {journal}"
        );
        assert_eq!(outputs[0]["size"].as_integer(), Some(9));
        assert_eq!(
            outputs[0]["sha256"].as_str().unwrap(),
            sha256_hex(b"NEW_BYTES")
        );
        assert_eq!(outputs[0]["entry"].as_str(), Some("outputs/0-EDIT00000000"));
    }
}
