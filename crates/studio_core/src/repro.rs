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
use std::time::{Instant, SystemTime};

#[cfg(not(target_arch = "wasm32"))]
use crossbeam_channel::{Sender, TrySendError};

/// Reading a bundle back, for `utw <tool-id> replay`.
#[cfg(not(target_arch = "wasm32"))]
mod read;
#[cfg(not(target_arch = "wasm32"))]
pub use read::{Bundle, BundleFile, BundleStep, read_bundle};

/// The session folder: every window's record written to disk as it
/// happens, and a kept folder turned into a bundle.
#[cfg(not(target_arch = "wasm32"))]
mod session;
#[cfg(not(target_arch = "wasm32"))]
pub use session::{
    EarlierSession, SessionEnd, SessionFile, SessionFrame, SessionHeader, SessionMessage,
    SessionWriter, earlier_sessions, read_frame, session_bundle, session_folder_name, write_frame,
};

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
    /// Who the session's messages go to, set once by `Recorder::send_to`
    /// before the window opens; the queue's other end writes the session
    /// folder. Under the journal's own lock so a step's message is queued
    /// in the order the step landed, never later.
    #[cfg(not(target_arch = "wasm32"))]
    sink: Option<Sender<SessionMessage>>,
    /// The full-queue drop already logged its line: a queue that fills
    /// stays full for a while, and one line per drop would flood the log.
    #[cfg(not(target_arch = "wasm32"))]
    drop_logged: bool,
}

/// `started` needs a `now`, which no derive can give.
impl Journal {
    fn new() -> Journal {
        Journal {
            started: Instant::now(),
            steps: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            sink: None,
            #[cfg(not(target_arch = "wasm32"))]
            drop_logged: false,
        }
    }

    /// `message` queued for the session folder's writer, never waited
    /// on. A full queue drops it: the session's first drop logs one
    /// warning and the rest stay quiet, one line per drop flooding the
    /// log. A disconnected sink is silent — its thread already logged
    /// why it stopped.
    #[cfg(not(target_arch = "wasm32"))]
    fn emit(&mut self, message: SessionMessage) {
        let Some(sink) = &self.sink else {
            return;
        };
        match sink.try_send(message) {
            Ok(()) | Err(TrySendError::Disconnected(_)) => {}
            Err(TrySendError::Full(_)) => {
                if !self.drop_logged {
                    self.drop_logged = true;
                    log::warn!("the session's queue is full; session messages are being dropped");
                }
            }
        }
    }

    /// A keep's file message emitted: an `Input` when `input`, an
    /// `Output` else; `packed` (the tool's pack and the file's own
    /// SHA-256) riding as `SessionFile::Packed`, `bytes` as `Raw` when
    /// there is none.
    #[cfg(not(target_arch = "wasm32"))]
    fn emit_file(
        &mut self,
        input: bool,
        tool: &'static str,
        path: &Path,
        bytes: Arc<[u8]>,
        packed: Option<(Arc<[u8]>, String)>,
    ) {
        let file = match packed {
            Some((packed, sha256)) => SessionFile::Packed {
                packed,
                size: bytes.len(),
                sha256,
            },
            None => SessionFile::Raw(bytes),
        };
        self.emit(if input {
            SessionMessage::Input {
                tool: tool.to_owned(),
                path: path.to_path_buf(),
                file,
            }
        } else {
            SessionMessage::Output {
                tool: tool.to_owned(),
                path: path.to_path_buf(),
                file,
            }
        });
    }

    /// The browser build has no session folder: nothing to emit to.
    #[cfg(target_arch = "wasm32")]
    fn emit_file(
        &mut self,
        _input: bool,
        _tool: &'static str,
        _path: &Path,
        _bytes: Arc<[u8]>,
        _packed: Option<(Arc<[u8]>, String)>,
    ) {
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
    /// The step's typed change beside the line, what a replay applies;
    /// `None` on a step that changed nothing (a pick, a refusal, a heading).
    #[cfg_attr(
        target_arch = "wasm32",
        expect(dead_code, reason = "read only by the native bundle writer")
    )]
    op: Option<toml::Table>,
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
    /// When the file was kept: the "wrote at" time the per-tool summary
    /// reads on outputs.
    written: SystemTime,
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
        self.push_step(tool, what, None);
    }

    /// `step` with the step's typed change beside its line: the table a
    /// replay applies where the line is only read. Steps that change the
    /// model record through here; notes (a pick, a refusal, a heading,
    /// a line whose change the ops do not cover) record through `step`.
    pub fn step_op(&self, tool: &'static str, what: impl Into<String>, op: toml::Table) {
        self.push_step(tool, what, Some(op));
    }

    /// `step` and `step_op`'s append.
    fn push_step(&self, tool: &'static str, what: impl Into<String>, op: Option<toml::Table>) {
        let mut journal = self.journal.lock().unwrap();
        let seconds = journal.started.elapsed().as_secs_f64();
        let what = what.into();
        journal.steps.push(Step {
            seconds,
            tool,
            what: what.clone(),
            op: op.clone(),
        });
        #[cfg(not(target_arch = "wasm32"))]
        journal.emit(SessionMessage::Step {
            seconds,
            tool: tool.to_owned(),
            what,
            op,
        });
    }

    /// Keeps `bytes` as the file `tool` read at `path`. The bundle's
    /// inputs hold every one, same path or not: what the tool started
    /// from is the part a saved-over file would destroy. Returns the
    /// kept file's index in the bundle's `[[input]]` list.
    pub fn keep_input(&self, tool: &'static str, path: &Path, bytes: Arc<[u8]>) -> usize {
        self.keep_input_impl(tool, path, bytes, None)
    }

    /// `keep_input` whose session message carries the file's `packed`
    /// form beside `bytes`' own size and SHA-256 — the tool's own pack
    /// of a file that does not compress on its own, which the folder's
    /// writer stores compressed and `read_bundle` rebuilds. The journal
    /// keeps `bytes` as `keep_input` does: a bundle of the running
    /// session still holds the file itself. With nobody listening
    /// (`listens` false) the keep is `keep_input`'s: the hash only the
    /// writer's packed row reads is skipped. Native only, as the
    /// session folder is.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn keep_input_packed(
        &self,
        tool: &'static str,
        path: &Path,
        bytes: Arc<[u8]>,
        packed: Vec<u8>,
    ) -> usize {
        if !self.listens() {
            return self.keep_input_impl(tool, path, bytes, None);
        }
        // The hash is computed before the lock: an 11 MB file's SHA-256
        // under it would stall every other recorder.
        let sha256 = bundle::sha256(&bytes);
        self.keep_input_impl(tool, path, bytes, Some((packed.into(), sha256)))
    }

    /// `keep_input` and `keep_input_packed`'s body: `packed` (the pack's
    /// bytes and the file's own SHA-256) when one was handed in.
    fn keep_input_impl(
        &self,
        tool: &'static str,
        path: &Path,
        bytes: Arc<[u8]>,
        packed: Option<(Arc<[u8]>, String)>,
    ) -> usize {
        let mut journal = self.journal.lock().unwrap();
        journal.inputs.push(KeptFile {
            tool,
            path: path.to_path_buf(),
            bytes: bytes.clone(),
            written: SystemTime::now(),
        });
        journal.emit_file(true, tool, path, bytes, packed);
        journal.inputs.len() - 1
    }

    /// Keeps `bytes` as the file `tool` wrote at `path`. The bundle
    /// holds the last write per path — the one the member has — so a
    /// kept write replaces an earlier one for the same tool and path.
    /// Returns the kept file's index in the bundle's `[[output]]` list
    /// (the replaced slot's on a replacement).
    pub fn keep_output(&self, tool: &'static str, path: &Path, bytes: Arc<[u8]>) -> usize {
        self.keep_output_impl(tool, path, bytes, None)
    }

    /// `keep_output` whose session message carries the file's `packed`
    /// form, as `keep_input_packed` does for inputs.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn keep_output_packed(
        &self,
        tool: &'static str,
        path: &Path,
        bytes: Arc<[u8]>,
        packed: Vec<u8>,
    ) -> usize {
        if !self.listens() {
            return self.keep_output_impl(tool, path, bytes, None);
        }
        // The hash is computed before the lock: an 11 MB file's SHA-256
        // under it would stall every other recorder.
        let sha256 = bundle::sha256(&bytes);
        self.keep_output_impl(tool, path, bytes, Some((packed.into(), sha256)))
    }

    /// `keep_output` and `keep_output_packed`'s body.
    fn keep_output_impl(
        &self,
        tool: &'static str,
        path: &Path,
        bytes: Arc<[u8]>,
        packed: Option<(Arc<[u8]>, String)>,
    ) -> usize {
        let mut journal = self.journal.lock().unwrap();
        let index = match journal
            .outputs
            .iter()
            .position(|kept| kept.tool == tool && kept.path == path)
        {
            Some(index) => {
                journal.outputs[index].bytes = bytes.clone();
                journal.outputs[index].written = SystemTime::now();
                index
            }
            None => {
                journal.outputs.push(KeptFile {
                    tool,
                    path: path.to_path_buf(),
                    bytes: bytes.clone(),
                    written: SystemTime::now(),
                });
                journal.outputs.len() - 1
            }
        };
        journal.emit_file(false, tool, path, bytes, packed);
        index
    }

    /// Whether `send_to` gave the recorder a session writer to feed.
    /// Only it reads a packed keep's session message, so a caller
    /// skips the pack and hash when this is false — every command-line
    /// run.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn listens(&self) -> bool {
        self.journal.lock().unwrap().sink.is_some()
    }

    /// Hands the session's messages to `sink`: every later `step`,
    /// `step_op`, `keep_input`, `keep_output`, `keep_input_packed` and
    /// `keep_output_packed` also queues its step or kept file — the
    /// file's `Arc` shared, never copied — for whoever writes the
    /// session folder at the queue's other end (the crash monitor,
    /// reached through its pipe; or the window's own writer thread).
    /// Set once, before the window opens. The queue is `try_` sent,
    /// never waited on: a full one drops the message and the first
    /// drop of the session logs one warning.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn send_to(&self, sink: Sender<SessionMessage>) {
        self.journal.lock().unwrap().sink = Some(sink);
    }

    /// Detaches the sink `send_to` set, ending the queue from the
    /// sender's side: the receiver sees the session's end and whoever
    /// writes the folder closes it out. Called once the window's run
    /// has returned.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn close_sink(&self) {
        self.journal.lock().unwrap().sink = None;
    }

    /// Writes the bundle to `out`: one zip holding `journal.toml` (the
    /// suite's version as the window shows it, the OS, every step, every
    /// kept file's redacted path, size and SHA-256 beside its zip entry's
    /// name, the names of the `files` the caller does not have under
    /// `missing`, and the `state` attachments' names), the inputs under
    /// `inputs/` and the last outputs under `outputs/` (each named
    /// `<index>-<file name>` so two files of one name never collide), the
    /// member's `words` as `words.txt`, each of `files` the caller has at
    /// the zip's root under its given name (the readme, the log, the
    /// settings, the installs found), and each of `state` under `state/<name>`: a
    /// tool's current state written as its format. Every path and step
    /// line, and every UTF-8 attachment, is written with the home folder
    /// as `~`. The zip is written to a sibling temp and renamed over
    /// `out`, so a failed write never leaves a half zip. The journal is
    /// cloned under its lock and written from the clone: the lock covers a
    /// cheap copy of `Arc`s, and a failed or panicking write can neither
    /// block nor poison the recorders.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write_bundle(
        &self,
        out: &Path,
        words: &str,
        files: &[(&str, Option<&[u8]>)],
        state: &[(&str, &[u8])],
        home: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut journal = self.journal.lock().unwrap().clone();
        // The clone is written, never listened on: a copied `Sender`
        // would keep the session's queue open past `close_sink`.
        journal.sink = None;
        bundle::write(&journal, out, words, files, state, home)
    }

    /// Every tool that recorded anything, as `ToolSummary`s — what the
    /// "Report a problem" dialog lists under "The file holds:", one line
    /// per tool, in the order they first recorded (the dialog sorts them
    /// into the sidebar's order). A tool that recorded nothing has no
    /// entry.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn tool_summaries(&self) -> Vec<ToolSummary> {
        let journal = self.journal.lock().unwrap();
        let mut summaries: Vec<ToolSummary> = Vec::new();
        for step in &journal.steps {
            let index = slot(&mut summaries, step.tool);
            summaries[index].steps += 1;
        }
        for kept in &journal.inputs {
            let index = slot(&mut summaries, kept.tool);
            let summary = &mut summaries[index];
            if summary.first_input.is_none() {
                summary.first_input = Some((kept.path.clone(), kept.bytes.len()));
            }
            summary.inputs += 1;
        }
        for kept in &journal.outputs {
            let index = slot(&mut summaries, kept.tool);
            let summary = &mut summaries[index];
            // `outputs` is ordered by first write, not last: a path written
            // again keeps its slot, so the last write is the newest `written`.
            if summary
                .last_output
                .as_ref()
                .is_none_or(|(_, written)| kept.written >= *written)
            {
                summary.last_output = Some((kept.path.clone(), kept.written));
            }
        }
        summaries
    }
}

/// `tool`'s entry in `summaries`, created on first sight.
#[cfg(not(target_arch = "wasm32"))]
fn slot(summaries: &mut Vec<ToolSummary>, tool: &'static str) -> usize {
    match summaries.iter().position(|summary| summary.tool == tool) {
        Some(index) => index,
        None => {
            summaries.push(ToolSummary {
                tool,
                steps: 0,
                inputs: 0,
                first_input: None,
                last_output: None,
            });
            summaries.len() - 1
        }
    }
}

/// One tool's recorded activity, read back for the "Report a problem"
/// dialog's "The file holds:" line: its step count, the first file it
/// opened with its size and how many more it opened, and the last file
/// it wrote with the write's time.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub(crate) struct ToolSummary {
    /// The recording tool's id.
    pub(crate) tool: &'static str,
    /// How many steps it recorded.
    pub(crate) steps: usize,
    /// How many files it kept as read (the bundle's `inputs/`).
    pub(crate) inputs: usize,
    /// Its first kept input: the file's path and byte length.
    pub(crate) first_input: Option<(PathBuf, usize)>,
    /// Its last kept output: the file's path and when the write was kept.
    pub(crate) last_output: Option<(PathBuf, SystemTime)>,
}

#[cfg(not(target_arch = "wasm32"))]
mod bundle {
    //! The bundle writer's file: `journal.toml`'s shape and the zip.

    use std::fs;
    use std::io::{self, Write};
    use std::path::Path;

    use anyhow::Context;
    use serde::{Deserialize, Serialize};
    use sha2::{Digest, Sha256};

    use super::{Journal, KeptFile};
    use crate::redact::redact_home;

    /// `journal.toml` as serialized: the suite's version, the OS, the
    /// root files the caller did not have under `missing`, the `state`
    /// attachments' names, then every step and every kept file as an
    /// array of tables.
    #[derive(Serialize)]
    struct Doc {
        /// The suite's version as the window shows it
        /// (`crate::shell::version_label` — the stage and a debug build's
        /// mark included). First, so it is the file's first line: the
        /// bundle's `README.txt` sends its reader there.
        version: String,
        /// The OS the window ran on (`std::env::consts::OS`).
        os: &'static str,
        /// The `files` names the caller did not have; absent when empty.
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

    /// One `[[step]]` row: seconds since the record's start, the tool
    /// and its line, and the typed change a replay applies when the step
    /// carried one. `Deserialize` for the session folder's journal,
    /// which holds the same rows.
    #[derive(Serialize, Deserialize)]
    pub(super) struct StepDoc {
        /// Seconds since the recorder was made.
        pub(super) seconds: f64,
        /// The recording tool's id.
        pub(super) tool: String,
        /// The step's line; in a bundle, the home folder written as `~`.
        pub(super) what: String,
        /// The step's typed change; absent on a note (an op holds no
        /// path, so it is never redacted).
        #[serde(skip_serializing_if = "Option::is_none")]
        pub(super) op: Option<toml::Table>,
    }

    /// One `[[input]]`/`[[output]]` row: what the kept file is and where
    /// it sits in the bundle or session folder.
    #[derive(Serialize, Deserialize)]
    pub(super) struct FileDoc {
        /// The tool that kept it.
        pub(super) tool: String,
        /// Its path; in a bundle, the home folder written as `~`.
        pub(super) path: String,
        /// Its byte length (the file's own, even on a packed entry).
        pub(super) size: usize,
        /// Its SHA-256, lowercase hex (the file's own).
        pub(super) sha256: String,
        /// Its name in the zip or folder (`<dir>/<index>-<file name>`).
        pub(super) entry: String,
        /// Whether the entry holds the file's packed form, compressed:
        /// `read_bundle` then inflates and rebuilds it before the same
        /// checks. Written only when true, so a raw row and an older
        /// journal read as before.
        #[serde(default, skip_serializing_if = "is_false")]
        pub(super) packed: bool,
    }

    /// `value` is `false` (`skip_serializing_if` wants a `&T` predicate).
    fn is_false(value: &bool) -> bool {
        !*value
    }

    /// `path`'s entry under `dir` (`inputs`/`outputs`): the index and
    /// the file name, so two files of one name never collide.
    pub(super) fn entry(dir: &str, index: usize, path: &Path) -> String {
        let name = path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy();
        format!("{dir}/{index}-{name}")
    }

    /// `bytes`' SHA-256, lowercase hex.
    pub(super) fn sha256(bytes: &[u8]) -> String {
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
                tool: kept.tool.to_owned(),
                path: redact_home(&kept.path.display().to_string(), home),
                size: kept.bytes.len(),
                sha256: sha256(&kept.bytes),
                entry: entry(dir, index, &kept.path),
                // A bundle of the running session keeps each file as it was.
                packed: false,
            })
            .collect()
    }

    /// Writes the bundle; see `Recorder::write_bundle`.
    pub(super) fn write(
        journal: &Journal,
        out: &Path,
        words: &str,
        files: &[(&str, Option<&[u8]>)],
        state: &[(&str, &[u8])],
        home: Option<&str>,
    ) -> anyhow::Result<()> {
        let doc = toml::to_string_pretty(&Doc {
            version: crate::shell::version_label(),
            os: std::env::consts::OS,
            missing: files
                .iter()
                .filter(|(_, bytes)| bytes.is_none())
                .map(|(name, _)| (*name).to_owned())
                .collect(),
            state: state.iter().map(|(name, _)| (*name).to_owned()).collect(),
            steps: journal
                .steps
                .iter()
                .map(|step| StepDoc {
                    seconds: step.seconds,
                    tool: step.tool.to_owned(),
                    what: redact_home(&step.what, home),
                    op: step.op.clone(),
                })
                .collect(),
            inputs: file_docs("inputs", &journal.inputs, home),
            outputs: file_docs("outputs", &journal.outputs, home),
        })
        .context("cannot serialize the journal")?;
        // A half zip at `out`'s side is litter, not a half `out`.
        let mut zipped = false;
        let written = crate::files::replace_file(out, |tmp| {
            let wrote = write_zip(tmp, &doc, journal, words, files, state, home);
            zipped = wrote.is_ok();
            wrote.map_err(io::Error::other)
        });
        if let Err(error) = written {
            return if zipped {
                // The zip was written whole; the move onto `out` is what failed.
                Err(error).with_context(|| {
                    format!("{}.tmp: cannot move the bundle into place", out.display())
                })
            } else {
                Err(error.into())
            };
        }
        Ok(())
    }

    /// The zip's contents into `tmp`: `journal.toml`, the kept inputs and
    /// outputs, `words.txt`, the root files the caller has and the `state`
    /// attachments — every attachment's bytes redacted when they are UTF-8
    /// text.
    fn write_zip(
        tmp: &Path,
        doc: &str,
        journal: &Journal,
        words: &str,
        files: &[(&str, Option<&[u8]>)],
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
            put(&entry("inputs", index, &kept.path), &kept.bytes)?;
        }
        for (index, kept) in journal.outputs.iter().enumerate() {
            put(&entry("outputs", index, &kept.path), &kept.bytes)?;
        }
        put("words.txt", redact_home(words, home).as_bytes())?;
        for (name, bytes) in files {
            if let Some(bytes) = bytes {
                put(name, &redacted(bytes, home))?;
            }
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
    use crate::shell::test_harness::test_dir;

    /// A fresh scratch folder for `test`, `test_dir("repro_<test>")`.
    pub(crate) fn scratch(test: &str) -> PathBuf {
        test_dir(&format!("repro_{test}"))
    }

    /// The zip's entry names at `path`, in write order.
    pub(crate) fn zip_names(path: &Path) -> Vec<String> {
        let file = fs::File::open(path).unwrap();
        let archive = zip::ZipArchive::new(file).unwrap();
        archive
            .file_names()
            .map(|name| name.unwrap().into_owned())
            .collect()
    }

    /// `name`'s bytes inside the zip at `path`.
    pub(crate) fn zip_bytes(path: &Path, name: &str) -> Vec<u8> {
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
    // words and the root files the caller has, one it has not listed under
    // `missing`.
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
            "compiler",
            Path::new("/home/member/out/EDIT00000000"),
            Arc::from(&b"TEAM"[..]),
        );
        recorder.keep_output(
            "save-editor",
            Path::new("out/EDIT00000000"),
            Arc::from(&b"REL"[..]),
        );
        let out = dir.join("repro.zip");

        recorder
            .write_bundle(
                &out,
                "the member's words",
                &[
                    ("settings.toml", Some(&b"theme = \"light\"\n"[..])),
                    ("utw.log", None),
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
            "the root file the caller did not have is named under `missing`: {journal}"
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
        assert_eq!(outputs[1]["tool"].as_str(), Some("compiler"));
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
        let settings =
            "pes_folder_path = 'C:/Users/Alice/PES'\ndir = \"C:\\\\Users\\\\Alice\\\\Games\"\n";
        let out = dir.join("repro.zip");
        recorder
            .write_bundle(
                &out,
                "I copied the save to C:/Users/Alice/Desktop",
                &[("settings.toml", Some(settings.as_bytes()))],
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

    // `keep_input` and `keep_output` return the kept file's index in its
    // list, a second write to one output's path the replaced slot's — a
    // caller storing the index would read the wrong file were it off.
    #[test]
    fn a_rewrite_returns_the_replaced_slots_index() {
        let dir = scratch("kept_index");
        let recorder = Recorder::default();
        assert_eq!(
            recorder.keep_input(
                "save-editor",
                Path::new("a/EDIT00000000"),
                Arc::from(&b"IN_A"[..])
            ),
            0
        );
        assert_eq!(
            recorder.keep_input(
                "save-editor",
                Path::new("b/EDIT00000000"),
                Arc::from(&b"IN_B"[..])
            ),
            1
        );
        assert_eq!(
            recorder.keep_output(
                "save-editor",
                Path::new("out/a.png"),
                Arc::from(&b"OUT_A"[..])
            ),
            0
        );
        assert_eq!(
            recorder.keep_output(
                "save-editor",
                Path::new("out/b.png"),
                Arc::from(&b"FIRST"[..])
            ),
            1
        );
        assert_eq!(
            recorder.keep_output(
                "save-editor",
                Path::new("out/b.png"),
                Arc::from(&b"SECOND"[..])
            ),
            1,
            "the second write to one path returns the replaced slot"
        );
        let out = dir.join("repro.zip");
        recorder.write_bundle(&out, "", &[], &[], None).unwrap();

        assert_eq!(
            zip_bytes(&out, "outputs/1-b.png"),
            b"SECOND",
            "the replaced slot holds the second write's bytes"
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

    // A zip write's failure names its context once: the write's error
    // travels to `replace_file` as an `io::Error` and back, and `{:#}` on
    // the result must not read "cannot write the bundle" twice.
    #[test]
    fn a_failed_bundle_write_names_its_context_once() {
        let dir = scratch("write_context_once");
        let recorder = Recorder::default();
        let out = dir.join("missing/repro.zip");
        let error = recorder
            .write_bundle(&out, "", &[], &[], None)
            .expect_err("a folder that does not exist");
        let os_error = fs::File::create(dir.join("missing/x.tmp"))
            .expect_err("the same failure")
            .to_string();

        let text = format!("{error:#}");
        assert_eq!(
            text.matches("cannot write the bundle").count(),
            1,
            "the context is named once: {text}"
        );
        assert!(
            text.ends_with(&os_error),
            "ending with the OS error: {text}"
        );
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

    // `close_sink` ends the queue from the sender's side: the receiver
    // takes the recorded step, then reads the queue's end.
    #[test]
    fn close_sink_disconnects_the_queue() {
        let recorder = Recorder::default();
        let (tx, rx) = crossbeam_channel::unbounded();
        recorder.send_to(tx);
        recorder.step("stub", "a step");
        recorder.close_sink();

        assert!(
            matches!(rx.try_recv(), Ok(SessionMessage::Step { .. })),
            "the step was queued"
        );
        assert!(
            matches!(
                rx.try_recv(),
                Err(crossbeam_channel::TryRecvError::Disconnected)
            ),
            "the queue ended"
        );
    }

    // A packed keep queues the file's packed form — the raw bytes' own
    // size and SHA-256 beside it — while the bundle keeps the raw file
    // and writes no `packed` key: `write_bundle` is as it was.
    #[test]
    fn a_packed_keep_queues_the_form_and_bundles_the_file() {
        let dir = scratch("keep_packed");
        let recorder = Recorder::default();
        let (tx, rx) = crossbeam_channel::unbounded();
        recorder.send_to(tx);
        recorder.keep_input_packed(
            "save-editor",
            Path::new("a/EDIT00000000"),
            Arc::from(&b"SAVE_BYTES"[..]),
            b"PACKED_FORM".to_vec(),
        );
        recorder.keep_output_packed(
            "save-editor",
            Path::new("out/EDIT00000000"),
            Arc::from(&b"WRITTEN"[..]),
            b"PACKED_WRITTEN".to_vec(),
        );
        let out = dir.join("repro.zip");
        recorder.write_bundle(&out, "", &[], &[], None).unwrap();

        let input = rx.try_recv().expect("the input message");
        let SessionMessage::Input { file, .. } = input else {
            panic!("an input message was sent")
        };
        let SessionFile::Packed {
            packed,
            size,
            sha256,
        } = file
        else {
            panic!("the input's file is packed")
        };
        assert_eq!(&*packed, b"PACKED_FORM");
        assert_eq!(size, 10, "the raw bytes' size");
        assert_eq!(
            sha256, "d3fb2163b52a43f0ef3f73b889c4f6bc925950da789e4be415ac49311d86bb66",
            "the raw bytes' SHA-256 (b\"SAVE_BYTES\")"
        );
        let output = rx.try_recv().expect("the output message");
        let SessionMessage::Output { file, .. } = output else {
            panic!("an output message was sent")
        };
        assert!(
            matches!(
                file,
                SessionFile::Packed { size: 7, sha256, .. }
                    if sha256 == "8f1c7519c3731203605ac4043e1231c0418561f1f3b00d48b320d2ddeea1b6d8"
            ),
            "the output's file is packed with the raw bytes' size and SHA-256"
        );

        let journal = zip_text(&out, "journal.toml");
        assert_eq!(zip_bytes(&out, "inputs/0-EDIT00000000"), b"SAVE_BYTES");
        assert_eq!(zip_bytes(&out, "outputs/0-EDIT00000000"), b"WRITTEN");
        let doc = journal_doc(&journal);
        let input_row = &doc["input"].as_array().unwrap()[0];
        assert_eq!(input_row["size"].as_integer(), Some(10), "{journal}");
        assert!(
            input_row.get("packed").is_none(),
            "no packed key on a bundle row: {journal}"
        );
        let output_row = &doc["output"].as_array().unwrap()[0];
        assert!(
            output_row.get("packed").is_none(),
            "no packed key on a bundle row: {journal}"
        );
    }

    // `listens` reports the sink `send_to` gave, and a packed keep with
    // nobody listening keeps the file as `keep_input` would — the pack
    // and hash ran for nothing, so they are skipped.
    #[test]
    fn a_packed_keep_without_a_listener_keeps_the_file() {
        let dir = scratch("packed_quiet");
        let recorder = Recorder::default();
        assert!(!recorder.listens(), "no sink was given");
        recorder.keep_input_packed(
            "save-editor",
            Path::new("a/EDIT00000000"),
            Arc::from(&b"SAVE_BYTES"[..]),
            b"PACKED_FORM".to_vec(),
        );
        let out = dir.join("repro.zip");
        recorder.write_bundle(&out, "", &[], &[], None).unwrap();

        assert_eq!(zip_bytes(&out, "inputs/0-EDIT00000000"), b"SAVE_BYTES");

        let (tx, _rx) = crossbeam_channel::unbounded();
        recorder.send_to(tx);
        assert!(recorder.listens(), "a sink was given");
    }

    // A tool's summary reads back its steps, its first input with its size
    // and the count of them, and the output it wrote last (the newest
    // `written`, not the last path first kept); a tool that recorded
    // nothing has none.
    #[test]
    fn a_tools_summary_reads_back_its_record() {
        let recorder = Recorder::default();
        recorder.step("save-editor", "opened a/EDIT00000000");
        recorder.step("save-editor", "player 1: Height 1 → 2");
        recorder.keep_input(
            "save-editor",
            Path::new("a/EDIT00000000"),
            Arc::from(&b"SAVE"[..]),
        );
        recorder.keep_input(
            "save-editor",
            Path::new("b/EDIT00000000"),
            Arc::from(&b"OTHER"[..]),
        );
        recorder.keep_output(
            "save-editor",
            Path::new("out/EDIT00000000"),
            Arc::from(&b"OLD"[..]),
        );
        recorder.keep_output(
            "save-editor",
            Path::new("out2/EDIT00000000"),
            Arc::from(&b"OTHER"[..]),
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
        recorder.keep_output(
            "save-editor",
            Path::new("out/EDIT00000000"),
            Arc::from(&b"NEW"[..]),
        );

        let summaries = recorder.tool_summaries();
        let [summary] = summaries.as_slice() else {
            panic!("one tool recorded: {summaries:?}")
        };
        assert_eq!(summary.tool, "save-editor");
        assert_eq!(summary.steps, 2);
        assert_eq!(summary.inputs, 2);
        assert_eq!(
            summary
                .first_input
                .as_ref()
                .map(|(path, size)| (path.as_path(), *size)),
            Some((Path::new("a/EDIT00000000"), 4)),
            "the first input, with its size"
        );
        // A path written again keeps its slot but refreshes its time: the
        // last write is `out`, the newest `written`, not `out2`, the last
        // path first kept.
        let (path, written) = summary.last_output.as_ref().expect("it wrote");
        assert_eq!(path, Path::new("out/EDIT00000000"), "the last write's path");
        assert!(
            written.elapsed().is_ok(),
            "the write's time is in the past: {written:?}"
        );
    }

    // An attachment whose bytes are not UTF-8 lands byte-equal: a binary
    // (a save's current state) is never rewritten by the redaction.
    #[test]
    fn a_binary_attachment_lands_byte_equal() {
        let dir = scratch("binary_extra");
        let recorder = Recorder::default();
        let bytes: &[u8] = b"\xFF\x00\xFE\x80abc";
        let out = dir.join("repro.zip");
        recorder
            .write_bundle(
                &out,
                "",
                &[("packed.bin", Some(bytes))],
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

    /// A `log::Log` capturing this module's records (`level|text`), for
    /// the drop warning's count; installed once, a process holding one
    /// logger.
    struct CaptureLog;

    /// `CaptureLog`'s records so far.
    static CAPTURED: Mutex<Vec<String>> = Mutex::new(Vec::new());

    static CAPTURE_LOG: CaptureLog = CaptureLog;

    impl log::Log for CaptureLog {
        fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            if record.target() == "studio_core::repro" {
                CAPTURED
                    .lock()
                    .unwrap()
                    .push(format!("{}|{}", record.level(), record.args()));
            }
        }

        fn flush(&self) {}
    }

    /// `CAPTURED`'s lines naming `what`.
    fn captured(what: &str) -> Vec<String> {
        CAPTURED
            .lock()
            .unwrap()
            .iter()
            .filter(|line| line.contains(what))
            .cloned()
            .collect()
    }

    // A disconnected sink is quiet — ten steps on a sink whose receiver
    // is already gone log no drop line, its thread having logged why it
    // stopped — and a full session queue drops the message rather than
    // block the caller: ten steps on a two-deep queue nobody drains all
    // return, and the drops log one warning, not one each.
    // ST-REP-10
    #[test]
    fn a_full_session_queue_drops_messages_and_logs_once() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            log::set_logger(&CAPTURE_LOG).unwrap();
            log::set_max_level(log::LevelFilter::Trace);
        });
        let dropped = "session messages are being dropped";

        let recorder = Recorder::default();
        let (tx, rx) = crossbeam_channel::bounded(2);
        drop(rx);
        recorder.send_to(tx);
        for index in 0..10 {
            recorder.step("stub", format!("step {index}"));
        }
        assert!(
            captured(dropped).is_empty(),
            "a disconnected sink stays quiet"
        );

        let recorder = Recorder::default();
        let (tx, _rx) = crossbeam_channel::bounded(2);
        recorder.send_to(tx);
        for index in 0..10 {
            recorder.step("stub", format!("step {index}"));
        }
        recorder.keep_input("stub", Path::new("a/EDIT00000000"), Arc::from(&b"IN"[..]));

        let warned = captured(dropped);
        assert_eq!(
            warned.len(),
            1,
            "one line for the session's drops: {warned:?}"
        );
        assert!(warned[0].starts_with("WARN|"), "{warned:?}");
    }
}
