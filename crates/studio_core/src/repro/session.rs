//! The session folder: `<data location>/sessions/<yyyy>-<mm>-<dd>-<hhmmss>-<pid>/`,
//! one per window, written as the session happens so a member who closed
//! Studio before finding a problem in game still has what that session
//! did. `journal.toml` opens with the build, the OS, the start time and
//! the process id, then takes one `[[step]]` block per step, one
//! `[[input]]` or `[[output]]` row per kept file, and an `[end]` table
//! last saying how the session ended. `inputs/` holds every file read;
//! `outputs/` the last file written per path, replaced at each write.
//! A file kept packed is `<index>-<name>.packed`, compressed.
//!
//! The crash monitor writes the folder from the window's session
//! messages (`SessionWriter`), which reach it as `SessionFrame`s
//! through its standard input; a window whose monitor did not start
//! runs the same writer on a thread of its own. `session_bundle` turns
//! a kept folder into the bytes of a repro bundle `read_bundle` reads.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use super::bundle::{FileDoc, StepDoc, entry, sha256};
use crate::files::replace_file;
use crate::redact::redact_home;

/// How many session folders the data location keeps besides the one a
/// new session is writing: the newest few are enough to report from.
const KEPT_SESSIONS: usize = 5;

/// What a session's `journal.toml` opens with, and what names its
/// folder: the build that recorded it, the OS it ran on, when the
/// session started and its process id.
#[derive(Clone)]
pub struct SessionHeader {
    /// The suite's version as the window shows it
    /// (`crate::shell::version_label`).
    pub version: String,
    /// The OS the window ran on (`std::env::consts::OS`).
    pub os: String,
    /// When the session started, in the window's own time zone; the
    /// folder is named from it.
    pub started: jiff::Zoned,
    /// The window's process id.
    pub pid: u32,
}

impl SessionHeader {
    /// The header of this window's session: the suite's version as the
    /// window shows it, this OS, now in this process's time zone and
    /// its process id.
    pub fn for_this_window() -> SessionHeader {
        SessionHeader {
            version: crate::shell::version_label(),
            os: std::env::consts::OS.to_owned(),
            started: jiff::Zoned::now(),
            pid: std::process::id(),
        }
    }
}

/// A kept file's bytes as the session's writer gets them: the file
/// itself, or its packed form — what a tool's own pack produced, which
/// compresses where the file does not — beside the file's own size and
/// SHA-256. The writer stores a packed form compressed; `read_bundle`
/// rebuilds the file and checks it against `size` and `sha256`, which
/// are the file's, never the packed form's.
#[derive(Debug, PartialEq)]
pub enum SessionFile {
    /// The file's bytes, as the recorder keeps them.
    Raw(Arc<[u8]>),
    /// The file's packed form beside its own size and SHA-256.
    Packed {
        /// The packed form's bytes.
        packed: Arc<[u8]>,
        /// The file's byte length.
        size: usize,
        /// The file's SHA-256, lowercase hex.
        sha256: String,
    },
}

/// One thing a window's session did, sent to whoever writes the folder:
/// the recorder's step, kept input or kept output as data.
pub enum SessionMessage {
    /// A step, as `Recorder::step` or `step_op` records it.
    Step {
        /// Seconds since the recorder's start.
        seconds: f64,
        /// The recording tool's id.
        tool: String,
        /// The step's line.
        what: String,
        /// The step's typed change (`[step.op]`); `None` on a note.
        op: Option<toml::Table>,
    },
    /// A file kept as it was read, as `Recorder::keep_input` or
    /// `keep_input_packed` records it.
    Input {
        /// The recording tool's id.
        tool: String,
        /// The path the file was read from.
        path: PathBuf,
        /// The file, or its packed form for the folder.
        file: SessionFile,
    },
    /// A file kept as it was written, as `Recorder::keep_output` or
    /// `keep_output_packed` records it: a later write to one tool and
    /// path replaces it.
    Output {
        /// The recording tool's id.
        tool: String,
        /// The path the file was written to.
        path: PathBuf,
        /// The file, or its packed form for the folder.
        file: SessionFile,
    },
}

/// How the session ended, written as `[end]`'s `how`: `closed` (the
/// window sent its closing message), `crashed` (the monitor handled the
/// crash, or the window caught a panic of its own and said so) or
/// `ended` (the window went away without either, as when it is ended
/// from Task Manager). A journal with no `[end]` is a session still
/// open, or one whose monitor died too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionEnd {
    /// The window sent its closing message.
    Closed,
    /// The window crashed: the monitor handled it, or the window caught
    /// a panic and told the monitor so.
    Crashed,
    /// The window went away without a closing message or a handled
    /// crash.
    Ended,
}

impl SessionEnd {
    /// The `[end]` table's `how` value.
    fn how(&self) -> &'static str {
        match self {
            SessionEnd::Closed => "closed",
            SessionEnd::Crashed => "crashed",
            SessionEnd::Ended => "ended",
        }
    }

    /// `how`'s `SessionEnd`; `None` on a string `how()` never writes.
    fn from_how(how: &str) -> Option<SessionEnd> {
        match how {
            "closed" => Some(SessionEnd::Closed),
            "crashed" => Some(SessionEnd::Crashed),
            "ended" => Some(SessionEnd::Ended),
            _ => None,
        }
    }
}

/// One thing the window sends through its crash monitor's standard
/// input: the session's header first, each `SessionMessage` as it
/// happens, and `Closed` — or `Crashed`, when the window caught a panic
/// of its own — when the window goes. The pipe's own end is the other
/// way the monitor learns the window is gone.
pub enum SessionFrame {
    /// The session's `SessionHeader`; the first frame and the only one
    /// of its kind.
    Header(SessionHeader),
    /// One recorded step, kept input or kept output.
    Message(SessionMessage),
    /// The window's closing message: the session's `[end]` is `closed`,
    /// not `ended`.
    Closed,
    /// The window's closing message after a panic it caught: the
    /// session's `[end]` is `crashed`.
    Crashed,
}

/// The frame kinds on the pipe: one byte naming the body that follows.
const KIND_HEADER: u8 = 1;
const KIND_STEP: u8 = 2;
const KIND_INPUT: u8 = 3;
const KIND_OUTPUT: u8 = 4;
const KIND_CLOSED: u8 = 5;
const KIND_CRASHED: u8 = 6;

/// A header's fields as a frame body's TOML; `started` is `Zoned`'s own
/// text, which keeps the time zone.
#[derive(Serialize, Deserialize)]
struct HeaderMeta {
    /// The `version` line.
    version: String,
    /// The `os` line.
    os: String,
    /// `started` printed by `Zoned`'s `Display` and parsed back by its
    /// `FromStr`, so the offset and the zone come along.
    started: String,
    /// The `pid` line.
    pid: u32,
}

/// A kept file's fields as a frame body's TOML; the file's bytes follow
/// the TOML raw — a save is far too large to travel inside it.
#[derive(Serialize, Deserialize)]
struct FileMeta {
    /// The recording tool's id.
    tool: String,
    /// The file's path, spelled as the journal spells it.
    path: String,
    /// Present when the raw part is the file's packed form: the file's
    /// own size and SHA-256 ride beside it. A frame with no `packed`
    /// table decodes as `SessionFile::Raw`.
    #[serde(skip_serializing_if = "Option::is_none")]
    packed: Option<PackedMeta>,
}

/// The `[packed]` table of a `FileMeta`: the file's own facts, since the
/// raw part is then the packed form's bytes.
#[derive(Serialize, Deserialize)]
struct PackedMeta {
    /// The file's byte length.
    size: usize,
    /// The file's SHA-256, lowercase hex.
    sha256: String,
}

/// `frame` written to `out`: a kind byte, then the body's length
/// (little-endian) and the body — the frame's fields as TOML, then a
/// kept file's bytes raw.
pub fn write_frame(out: &mut (impl Write + ?Sized), frame: &SessionFrame) -> io::Result<()> {
    let (kind, body) = frame_body(frame)?;
    let length = u32::try_from(body.len()).expect("a frame body is under 4 GB");
    out.write_all(&[kind])?;
    out.write_all(&length.to_le_bytes())?;
    out.write_all(&body)
}

/// One frame read from `input`; `None` at its end, reached between
/// frames or inside one — a window that died mid-send drops its partial
/// frame with it. A body that does not decode is `InvalidData`.
pub fn read_frame(input: &mut impl Read) -> io::Result<Option<SessionFrame>> {
    let mut kind = [0_u8; 1];
    if !read_full(input, &mut kind)? {
        return Ok(None);
    }
    let mut length = [0_u8; 4];
    if !read_full(input, &mut length)? {
        return Ok(None);
    }
    let mut body = vec![0; u32::from_le_bytes(length) as usize];
    if !read_full(input, &mut body)? {
        return Ok(None);
    }
    decode_frame(kind[0], &body).map(Some)
}

/// `buf` read whole; `false` is the input's end, whether it comes before
/// any of `buf` or inside it — a frame cut mid-send is dropped, not an
/// error.
fn read_full(input: &mut impl Read, buf: &mut [u8]) -> io::Result<bool> {
    match input.read_exact(buf) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
        Err(error) => Err(error),
    }
}

/// `frame`'s kind byte and body.
fn frame_body(frame: &SessionFrame) -> io::Result<(u8, Vec<u8>)> {
    match frame {
        SessionFrame::Header(header) => Ok((
            KIND_HEADER,
            encode(
                &HeaderMeta {
                    version: header.version.clone(),
                    os: header.os.clone(),
                    started: header.started.to_string(),
                    pid: header.pid,
                },
                &[],
            )?,
        )),
        SessionFrame::Message(SessionMessage::Step {
            seconds,
            tool,
            what,
            op,
        }) => Ok((
            KIND_STEP,
            encode(
                &StepDoc {
                    seconds: *seconds,
                    tool: tool.clone(),
                    what: what.clone(),
                    op: op.clone(),
                },
                &[],
            )?,
        )),
        SessionFrame::Message(SessionMessage::Input { tool, path, file }) => {
            Ok((KIND_INPUT, file_body(tool, path, file)?))
        }
        SessionFrame::Message(SessionMessage::Output { tool, path, file }) => {
            Ok((KIND_OUTPUT, file_body(tool, path, file)?))
        }
        SessionFrame::Closed => Ok((KIND_CLOSED, Vec::new())),
        SessionFrame::Crashed => Ok((KIND_CRASHED, Vec::new())),
    }
}

/// `file`'s frame body: the `FileMeta` (its `packed` table set on a
/// `Packed` file) as TOML, then the file's bytes raw — the packed form's
/// on a `Packed`.
fn file_body(tool: &str, path: &Path, file: &SessionFile) -> io::Result<Vec<u8>> {
    let (packed, raw) = match file {
        SessionFile::Raw(bytes) => (None, &bytes[..]),
        SessionFile::Packed {
            packed,
            size,
            sha256,
        } => (
            Some(PackedMeta {
                size: *size,
                sha256: sha256.clone(),
            }),
            &packed[..],
        ),
    };
    encode(
        &FileMeta {
            tool: tool.to_owned(),
            path: path.display().to_string(),
            packed,
        },
        raw,
    )
}

/// A frame body: `meta` as TOML behind its own little-endian length,
/// then `raw` (a kept file's bytes).
fn encode(meta: &impl Serialize, raw: &[u8]) -> io::Result<Vec<u8>> {
    let meta = toml::to_string(meta).map_err(io::Error::other)?;
    let meta_len = u32::try_from(meta.len()).expect("a frame's TOML is under 4 GB");
    let mut body = Vec::with_capacity(4 + meta.len() + raw.len());
    body.extend_from_slice(&meta_len.to_le_bytes());
    body.extend_from_slice(meta.as_bytes());
    body.extend_from_slice(raw);
    Ok(body)
}

/// `body` decoded by its `kind`; a body that does not decode is
/// `InvalidData`.
fn decode_frame(kind: u8, body: &[u8]) -> io::Result<SessionFrame> {
    match kind {
        KIND_HEADER => {
            let (meta, _) = split_body(body)?;
            let meta: HeaderMeta = toml::from_str(meta).map_err(invalid)?;
            Ok(SessionFrame::Header(SessionHeader {
                version: meta.version,
                os: meta.os,
                started: meta.started.parse().map_err(invalid)?,
                pid: meta.pid,
            }))
        }
        KIND_STEP => {
            let (meta, _) = split_body(body)?;
            let meta: StepDoc = toml::from_str(meta).map_err(invalid)?;
            Ok(SessionFrame::Message(SessionMessage::Step {
                seconds: meta.seconds,
                tool: meta.tool,
                what: meta.what,
                op: meta.op,
            }))
        }
        KIND_INPUT | KIND_OUTPUT => {
            let (meta, raw) = split_body(body)?;
            let meta: FileMeta = toml::from_str(meta).map_err(invalid)?;
            let file = match meta.packed {
                Some(packed) => SessionFile::Packed {
                    packed: Arc::from(raw),
                    size: packed.size,
                    sha256: packed.sha256,
                },
                None => SessionFile::Raw(Arc::from(raw)),
            };
            let (tool, path) = (meta.tool, PathBuf::from(meta.path));
            Ok(SessionFrame::Message(if kind == KIND_INPUT {
                SessionMessage::Input { tool, path, file }
            } else {
                SessionMessage::Output { tool, path, file }
            }))
        }
        KIND_CLOSED => Ok(SessionFrame::Closed),
        KIND_CRASHED => Ok(SessionFrame::Crashed),
        _ => Err(invalid(format!("an unknown frame kind {kind}"))),
    }
}

/// `body` split into its TOML's text and the raw bytes after it: the
/// TOML's own length opens the body.
fn split_body(body: &[u8]) -> io::Result<(&str, &[u8])> {
    let Some((length, rest)) = body.split_at_checked(4) else {
        return Err(invalid("a frame body without its length"));
    };
    let meta_len = u32::from_le_bytes(
        length
            .try_into()
            .expect("split_at_checked(4) gives four bytes"),
    ) as usize;
    let Some((meta, raw)) = rest.split_at_checked(meta_len) else {
        return Err(invalid("a frame body cut inside its TOML"));
    };
    let meta = std::str::from_utf8(meta).map_err(|_| invalid("a frame's TOML is not UTF-8"))?;
    Ok((meta, raw))
}

/// An undecodable frame is an `InvalidData` read error.
fn invalid(error: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

/// A session folder's `journal.toml`, as written and as read back: the
/// header's scalars, then the `[[step]]`, `[[input]]` and `[[output]]`
/// rows, then `[end]`. `Deserialize` covers `session_bundle`; `Serialize`
/// the header write and the bundle's own journal. The row types are the
/// bundle writer's: the shapes are one.
#[derive(Serialize, Deserialize)]
struct SessionDoc {
    /// The `version` line.
    version: String,
    /// The `os` line.
    os: String,
    /// The session's start, RFC 3339 with the offset.
    started: toml::value::Datetime,
    /// The window's process id.
    pid: u32,
    /// Every `[[step]]` row.
    #[serde(rename = "step", default, skip_serializing_if = "Vec::is_empty")]
    steps: Vec<StepDoc>,
    /// Every `[[input]]` row.
    #[serde(rename = "input", default, skip_serializing_if = "Vec::is_empty")]
    inputs: Vec<FileDoc>,
    /// Every `[[output]]` row: one per write to a path, so a path
    /// written twice has two rows and the last counts.
    #[serde(rename = "output", default, skip_serializing_if = "Vec::is_empty")]
    outputs: Vec<FileDoc>,
    /// The `[end]` table; absent while the session is open (or its
    /// monitor died before writing it).
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<EndRow>,
}

impl SessionDoc {
    /// `header`'s `journal.toml` start: the four scalar lines and no
    /// rows yet.
    fn header(header: &SessionHeader) -> anyhow::Result<SessionDoc> {
        let started = header
            .started
            .strftime("%Y-%m-%dT%H:%M:%S%:z")
            .to_string()
            .parse::<toml::value::Datetime>()
            .context("the start time is not RFC 3339")?;
        Ok(SessionDoc {
            version: header.version.clone(),
            os: header.os.clone(),
            started,
            pid: header.pid,
            steps: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            end: None,
        })
    }
}

/// The `[end]` table: `how` the session ended.
#[derive(Serialize, Deserialize)]
struct EndRow {
    /// `closed`, `crashed` or `ended`.
    how: String,
}

/// One `[[step]]` block as `toml` spells an append.
#[derive(Serialize)]
struct StepBlock {
    step: Vec<StepDoc>,
}

/// One `[[input]]` row as `toml` spells an append.
#[derive(Serialize)]
struct InputBlock {
    input: Vec<FileDoc>,
}

/// One `[[output]]` row as `toml` spells an append.
#[derive(Serialize)]
struct OutputBlock {
    output: Vec<FileDoc>,
}

/// The `[end]` table as `toml` spells an append.
#[derive(Serialize)]
struct EndBlock {
    end: EndRow,
}

/// Writes a window's session to its folder as it happens: `write` each
/// `SessionMessage` as it arrives, `end` the one closing state. The
/// folder is made at the first `write`, so a session that records
/// nothing leaves none. Blocks are appended whole, one write each
/// ending with the `# ok` marker, so a crash can cut at most the block
/// in flight — which `session_bundle` drops on reading.
pub struct SessionWriter {
    /// The data location's `sessions` folder.
    root: PathBuf,
    /// The header `journal.toml` opens with, which also names the folder.
    header: SessionHeader,
    /// The session's folder, made at the first `write`.
    folder: Option<PathBuf>,
    /// The next input's index (every input is a new entry).
    inputs: usize,
    /// Each output path's index at its first write, as `keep_output`
    /// gives it: a later write to one tool and path replaces the file
    /// under that index and appends a row.
    outputs: Vec<(String, PathBuf)>,
    /// `[end]` was appended: a later `end` writes nothing.
    ended: bool,
}

impl SessionWriter {
    /// A writer for the session `header` describes, its folder under
    /// `root` (the data location's `sessions`). Nothing is touched until
    /// the first `write`.
    pub fn new(root: &Path, header: SessionHeader) -> SessionWriter {
        SessionWriter {
            root: root.to_path_buf(),
            header,
            folder: None,
            inputs: 0,
            outputs: Vec::new(),
            ended: false,
        }
    }

    /// Writes `message` into the session's folder, making the folder
    /// first when needed: a step appends its `[[step]]` block, an input
    /// is written under `inputs/` then gets its `[[input]]` row, an
    /// output the same under `outputs/` — the row appended after its
    /// file is in place.
    pub fn write(&mut self, message: &SessionMessage) -> anyhow::Result<()> {
        let folder = self.ensure_folder()?;
        let journal = folder.join("journal.toml");
        match message {
            SessionMessage::Step {
                seconds,
                tool,
                what,
                op,
            } => {
                let block = StepBlock {
                    step: vec![StepDoc {
                        seconds: *seconds,
                        tool: tool.clone(),
                        what: what.clone(),
                        op: op.clone(),
                    }],
                };
                append_block(&journal, &block)?;
            }
            SessionMessage::Input { tool, path, file } => {
                let index = self.inputs;
                self.inputs += 1;
                let row = keep_file(&folder, "inputs", index, tool, path, file)?;
                append_block(&journal, &InputBlock { input: vec![row] })?;
            }
            SessionMessage::Output { tool, path, file } => {
                let index = match self
                    .outputs
                    .iter()
                    .position(|(kept_tool, kept_path)| kept_tool == tool && kept_path == path)
                {
                    Some(index) => index,
                    None => {
                        self.outputs.push((tool.clone(), path.clone()));
                        self.outputs.len() - 1
                    }
                };
                let row = keep_file(&folder, "outputs", index, tool, path, file)?;
                append_block(&journal, &OutputBlock { output: vec![row] })?;
            }
        }
        Ok(())
    }

    /// Appends `[end]` with `how` the session ended. Only the first
    /// `end` counts — a later one writes nothing — and an `end` before
    /// the first `write` writes nothing unless a folder is there to
    /// adopt: a writer that took the session over still owes the folder
    /// its `[end]`.
    pub fn end(&mut self, end: SessionEnd) -> anyhow::Result<()> {
        if self.folder.is_none() && !self.adopt_folder()? {
            return Ok(());
        }
        // Only the first `end` counts — `ended` set by an earlier call or
        // by the journal just adopted, whose `[end]` stands.
        if self.ended {
            return Ok(());
        }
        let folder = self.folder.as_ref().expect("the folder is set");
        let block = EndBlock {
            end: EndRow {
                how: end.how().to_owned(),
            },
        };
        append_block(&folder.join("journal.toml"), &block)?;
        self.ended = true;
        Ok(())
    }

    /// The session's folder: `<root>/<yyyy>-<mm>-<dd>-<hhmmss>-<pid>/`
    /// from the header's local time and pid. A `journal.toml` already
    /// there is adopted, anything else is made fresh.
    fn ensure_folder(&mut self) -> anyhow::Result<PathBuf> {
        if self.folder.is_none() && !self.adopt_folder()? {
            self.make_folder()?;
        }
        Ok(self.folder.clone().expect("the folder is set"))
    }

    /// `self.folder` adopted from the folder the header names, `true`
    /// when its `journal.toml` was already there: the file is cut back
    /// to just after its last `# ok` line — a block the earlier writer
    /// left half-written sits past it, where it would read as a row —
    /// and the rows up to the cut give the next input's index and each
    /// output's tool and path, so the writer writes on where they left
    /// off. Nothing before the cut is rewritten, and `root` is not
    /// pruned: the folder is no new session. `false` when the journal
    /// is not there, or holds no marker at all — the earlier writer
    /// then died inside the header's own block and the folder is made
    /// fresh.
    fn adopt_folder(&mut self) -> anyhow::Result<bool> {
        let name = session_folder_name(&self.header);
        let folder = self.root.join(&name);
        let journal = folder.join("journal.toml");
        let bytes = match fs::read(&journal) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("{}: cannot read the journal", journal.display()));
            }
        };
        // Lossy, as `session_bundle` reads it: a block cut inside a
        // multi-byte character sits past the last marker anyway, and the
        // marker search must reach it still.
        let text = String::from_utf8_lossy(&bytes);
        let Some(end) = last_marker_end(&text) else {
            return Ok(false);
        };
        for dir in ["inputs", "outputs"] {
            fs::create_dir_all(folder.join(dir))
                .with_context(|| format!("{}: cannot make the session folder", folder.display()))?;
        }
        fs::OpenOptions::new()
            .write(true)
            .open(&journal)
            .and_then(|file| file.set_len(end as u64))
            .with_context(|| format!("{}: cannot cut the journal", journal.display()))?;
        // The same parse `session_bundle` runs on the kept rows.
        let doc: SessionDoc = toml::from_str(&text[..end])
            .with_context(|| format!("{}: cannot parse the journal", folder.display()))?;
        self.inputs = doc.inputs.len();
        for row in doc.outputs {
            let path = PathBuf::from(&row.path);
            if !self
                .outputs
                .iter()
                .any(|(tool, kept)| tool == &row.tool && kept == &path)
            {
                self.outputs.push((row.tool, path));
            }
        }
        // An `[end]` already written stands: only the first one counts.
        self.ended = doc.end.is_some();
        self.folder = Some(folder);
        Ok(true)
    }

    /// `self.folder` made fresh: `journal.toml`'s header lines and the
    /// empty `inputs/` and `outputs/`, then `root` pruned to the kept
    /// newest folders.
    fn make_folder(&mut self) -> anyhow::Result<()> {
        let name = session_folder_name(&self.header);
        let folder = self.root.join(&name);
        for dir in ["inputs", "outputs"] {
            fs::create_dir_all(folder.join(dir))
                .with_context(|| format!("{}: cannot make the session folder", folder.display()))?;
        }
        let text = toml_block(&SessionDoc::header(&self.header)?)?;
        let journal = folder.join("journal.toml");
        replace_file(&journal, |tmp| fs::write(tmp, &text))
            .with_context(|| format!("{}: cannot write the journal", journal.display()))?;
        prune(&self.root, &name);
        self.folder = Some(folder);
        Ok(())
    }
}

/// `file` written under `folder` through `replace_file` — the row then
/// appended is for a file already whole in place. A raw file lands as
/// `<dir>/<index>-<file name>`; a packed form as
/// `<dir>/<index>-<file name>.packed`, zlib-compressed (the pack exists
/// because the file itself does not compress, and the compression runs
/// here, off the window's thread). The row's `size`/`sha256` are the
/// kept file's own — a packed row's come from the `SessionFile`, never
/// recomputed from the packed form.
fn keep_file(
    folder: &Path,
    dir: &str,
    index: usize,
    tool: &str,
    path: &Path,
    file: &SessionFile,
) -> anyhow::Result<FileDoc> {
    let (entry, size, hash, packed, written) = match file {
        SessionFile::Raw(bytes) => {
            let entry = entry(dir, index, path);
            let written = replace_file(&folder.join(&entry), |tmp| fs::write(tmp, bytes));
            (entry, bytes.len(), sha256(bytes), false, written)
        }
        SessionFile::Packed {
            packed,
            size,
            sha256,
        } => {
            let entry = format!("{}.packed", entry(dir, index, path));
            let written = replace_file(&folder.join(&entry), |tmp| compressed(tmp, packed));
            (entry, *size, sha256.clone(), true, written)
        }
    };
    written.with_context(|| format!("{}: cannot keep the file", folder.join(&entry).display()))?;
    Ok(FileDoc {
        tool: tool.to_owned(),
        path: path.display().to_string(),
        size,
        sha256: hash,
        entry,
        packed,
    })
}

/// `packed` zlib-compressed into `tmp` (`keep_file`'s packed write).
fn compressed(tmp: &Path, packed: &[u8]) -> io::Result<()> {
    let mut encoder =
        flate2::write::ZlibEncoder::new(fs::File::create(tmp)?, flate2::Compression::default());
    encoder.write_all(packed)?;
    encoder.finish()?;
    Ok(())
}

/// `header`'s session folder name, `<yyyy>-<mm>-<dd>-<hhmmss>-<pid>`
/// from its start's local time, so folders sort by when they were made.
pub fn session_folder_name(header: &SessionHeader) -> String {
    let when = header.started.datetime();
    format!(
        "{:04}-{:02}-{:02}-{:02}{:02}{:02}-{}",
        when.year(),
        when.month(),
        when.day(),
        when.hour(),
        when.minute(),
        when.second(),
        header.pid
    )
}

/// `name` spells `<yyyy>-<mm>-<dd>-<hhmmss>-<pid>`: four, two, two and
/// six digits, then the pid — what `prune` may delete and nothing else
/// in `root` may be.
fn is_session_name(name: &str) -> bool {
    let parts: Vec<&str> = name.split('-').collect();
    let [year, month, day, time, pid] = parts[..] else {
        return false;
    };
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    year.len() == 4
        && month.len() == 2
        && day.len() == 2
        && time.len() == 6
        && [year, month, day, time, pid]
            .iter()
            .all(|part| digits(part))
}

/// `root` keeps the `KEPT_SESSIONS` newest session folders besides
/// `own`: every older one is deleted. Anything in `root` not named like
/// a session folder is left, and a failed delete is a warning in the
/// log — the session's own write never fails on old folders.
fn prune(root: &Path, own: &str) {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            log::warn!(
                "{}: cannot list the sessions folder: {error}",
                root.display()
            );
            return;
        }
    };
    let mut folders: Vec<PathBuf> = entries
        .filter_map(|dir_entry| match dir_entry {
            Ok(dir_entry) => Some(dir_entry),
            Err(error) => {
                log::warn!("{}: cannot list a session: {error}", root.display());
                None
            }
        })
        .map(|dir_entry| dir_entry.path())
        .filter(|path| {
            path.is_dir()
                && path.file_name().is_some_and(|name| {
                    name != std::ffi::OsStr::new(own) && is_session_name(&name.to_string_lossy())
                })
        })
        .collect();
    // Names start with the date, so the sort order is the age order.
    folders.sort();
    for old in folders
        .iter()
        .take(folders.len().saturating_sub(KEPT_SESSIONS))
    {
        if let Err(error) = fs::remove_dir_all(old) {
            log::warn!(
                "{}: cannot delete the old session folder: {error}",
                old.display()
            );
        }
    }
}

/// `block` appended to `journal` in one write: a crash can cut at most
/// this block, which `session_bundle` then drops.
fn append_block(journal: &Path, block: &impl Serialize) -> anyhow::Result<()> {
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(journal)
        .with_context(|| format!("{}: cannot append to the journal", journal.display()))?;
    file.write_all(toml_block(block)?.as_bytes())
        .with_context(|| format!("{}: cannot append to the journal", journal.display()))
}

/// `value`'s TOML ending with the `# ok` marker line: `session_bundle`
/// reads a journal up to its last marker, so a block cut anywhere is
/// dropped whole.
fn toml_block(value: &impl Serialize) -> anyhow::Result<String> {
    let mut text = toml::to_string_pretty(value).context("cannot serialize the journal block")?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str("# ok\n");
    Ok(text)
}

/// `text`'s byte offset just after the last `# ok` marker line — the
/// end of the last whole block, every appended block ending with one.
/// `None` when no marker line is present.
fn last_marker_end(text: &str) -> Option<usize> {
    let mut last = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        offset += line.len();
        if line.trim() == "# ok" {
            last = Some(offset);
        }
    }
    last
}

/// `folder`'s `journal.toml` as a `SessionDoc`. Only the text up to the
/// last `# ok` marker line is parsed, so a block cut off mid-write is
/// dropped whole — on a line boundary too, where its fragment would
/// still parse as a note or a different op; a journal that does not
/// parse is an error naming the folder.
fn read_session_doc(folder: &Path) -> anyhow::Result<SessionDoc> {
    let journal = folder.join("journal.toml");
    let bytes = fs::read(&journal)
        .with_context(|| format!("{}: cannot read the journal", journal.display()))?;
    // Lossy, not strict: a cut inside a multi-byte character ("→" in
    // every field line) must reach the marker drop, not fail here.
    let text = String::from_utf8_lossy(&bytes);
    let end = last_marker_end(&text).unwrap_or(0);
    toml::from_str(&text[..end])
        .with_context(|| format!("{}: cannot parse the journal", folder.display()))
}

/// `folder`, a session folder, as a bundle's zip bytes: `journal.toml`
/// rewritten as `write_bundle` writes it — every path and step line
/// with `home` as `~`, and of a path's `[[output]]` rows the last kept
/// at the index of its first — plus each row's file from `inputs/` or
/// `outputs/`, so `read_bundle` reads it like any other. The journal is
/// read as `read_session_doc` reads it.
pub fn session_bundle(folder: &Path, home: Option<&str>) -> anyhow::Result<Vec<u8>> {
    let mut doc = read_session_doc(folder)?;
    for step in &mut doc.steps {
        step.what = redact_home(&step.what, home);
    }
    // Of a path's [[output]] rows the last counts, at the index of its
    // first: each row after the first replaces the row at its slot, as
    // the session's file was replaced under its first write's name.
    let mut outputs: Vec<FileDoc> = Vec::with_capacity(doc.outputs.len());
    for row in doc.outputs {
        match outputs
            .iter_mut()
            .find(|kept| kept.tool == row.tool && kept.path == row.path)
        {
            Some(kept) => *kept = row,
            None => outputs.push(row),
        }
    }
    doc.outputs = outputs;
    for row in doc.inputs.iter_mut().chain(&mut doc.outputs) {
        row.path = redact_home(&row.path, home);
    }

    // No marker lines in the bundle: they only mark the session folder's
    // appends, which a bundle is never appended to.
    let text = toml::to_string_pretty(&doc).context("cannot serialize the journal")?;
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
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
    put("journal.toml", text.as_bytes())?;
    for row in doc.inputs.iter().chain(&doc.outputs) {
        let file = folder.join(&row.entry);
        let bytes = fs::read(&file)
            .with_context(|| format!("{}: cannot read the kept file", file.display()))?;
        put(&row.entry, &bytes)?;
    }
    let cursor = writer
        .finish()
        .with_context(|| format!("{}: cannot finish the bundle", folder.display()))?;
    Ok(cursor.into_inner())
}

/// One earlier session kept on disk, as Report a problem's "Include
/// earlier sessions" box lists it: the folder, when it started, the first
/// file it opened, when it last wrote one and how it ended.
#[derive(Debug)]
pub struct EarlierSession {
    /// The session's folder under the data location's `sessions`.
    pub folder: PathBuf,
    /// When the session started, the local time its folder's name holds.
    pub started: jiff::civil::DateTime,
    /// The file name the first `[[input]]` row's path ends in; `None`
    /// when the session opened nothing.
    pub opened: Option<String>,
    /// When the session last wrote a file: the newest `outputs/` file's
    /// modification time — the rows carry no time, and a kept output is
    /// written at the moment of its write. `None` when it wrote nothing.
    pub wrote: Option<SystemTime>,
    /// How the session ended, the `[end]` table's `how`; `None` on a
    /// journal without one — a session still open in another window, or
    /// one whose writer died too.
    pub ended: Option<SessionEnd>,
}

/// Every session folder under `root` (the data location's `sessions`)
/// but the one named `own` — the window's own — newest first: the names
/// start with the start, so the sort order is the age order. A folder
/// not named like a session is left out, and one whose journal cannot
/// be read or parsed is left out with a debug line.
pub fn earlier_sessions(root: &Path, own: Option<&str>) -> Vec<EarlierSession> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            log::debug!(
                "{}: cannot list the sessions folder: {error}",
                root.display()
            );
            return Vec::new();
        }
    };
    let mut folders: Vec<PathBuf> = entries
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry.path()),
            Err(error) => {
                log::debug!("{}: cannot list a session: {error}", root.display());
                None
            }
        })
        .filter(|path| {
            path.is_dir()
                && path.file_name().is_some_and(|name| {
                    Some(name) != own.map(std::ffi::OsStr::new)
                        && is_session_name(&name.to_string_lossy())
                })
        })
        .collect();
    // Names start with the date, so the sort order is the age order.
    folders.sort();
    folders
        .iter()
        .rev()
        .filter_map(|folder| earlier_session(folder))
        .collect()
}

/// The session kept in `folder`, a folder whose name `is_session_name`
/// passed; `None` when its journal cannot be read or parsed — such a
/// folder is left out, not a failure the report should die on.
fn earlier_session(folder: &Path) -> Option<EarlierSession> {
    let name = folder
        .file_name()
        .expect("a session folder has a name")
        .to_string_lossy();
    let doc = match read_session_doc(folder) {
        Ok(doc) => doc,
        Err(error) => {
            log::debug!("{error:#}");
            return None;
        }
    };
    Some(EarlierSession {
        folder: folder.to_path_buf(),
        started: session_started(&name)?,
        opened: doc.inputs.first().map(|row| file_name(&row.path)),
        wrote: last_output_time(folder),
        ended: doc.end.and_then(|end| SessionEnd::from_how(&end.how)),
    })
}

/// `name`'s start: the `yyyy-mm-dd-hhmmss` the folder name opens with,
/// as a civil DateTime. `None` on digits that make no date or time —
/// `is_session_name` checks only the shape.
fn session_started(name: &str) -> Option<jiff::civil::DateTime> {
    let [year, month, day, time, _pid] = name.split('-').collect::<Vec<_>>()[..] else {
        return None;
    };
    format!(
        "{year}-{month}-{day}T{}:{}:{}",
        &time[..2],
        &time[2..4],
        &time[4..]
    )
    .parse()
    .ok()
}

/// `path`'s last part, the path whole when it has none (a drive root).
fn file_name(path: &str) -> String {
    Path::new(path).file_name().map_or_else(
        || path.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// The newest file's modification time under `folder`'s `outputs/`; the
/// rows carry no time, and a kept output lands at the moment of its
/// write. `None` when `outputs/` holds none or cannot be listed.
fn last_output_time(folder: &Path) -> Option<SystemTime> {
    fs::read_dir(folder.join("outputs"))
        .ok()?
        .filter_map(|entry| entry.ok()?.metadata().ok()?.modified().ok())
        .max()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::repro::{Bundle, read_bundle, tests::zip_text};
    use crate::shell::test_harness::TestDir;

    /// A fixed header: `started` at 2026-11-21 09:30:15 UTC, so the
    /// session's folder is `2026-11-21-093015-<pid>`.
    fn header(pid: u32) -> SessionHeader {
        SessionHeader {
            version: "v-test".to_owned(),
            os: "test-os".to_owned(),
            started: jiff::civil::date(2026, 11, 21)
                .at(9, 30, 15, 0)
                .to_zoned(jiff::tz::TimeZone::UTC)
                .unwrap(),
            pid,
        }
    }

    /// The step's op table, as recorded and as the bundle hands it back.
    fn op() -> toml::Table {
        toml::from_str("kind = \"player\"\nid = 70101\n").unwrap()
    }

    /// A stub tool's messages in order: a step with its op, a kept
    /// input, and a kept output written twice to one path.
    fn messages() -> Vec<SessionMessage> {
        vec![
            SessionMessage::Step {
                seconds: 0.5,
                tool: "stub".to_owned(),
                what: "opened C:/Data/in/EDIT00000000 (10 bytes)".to_owned(),
                op: Some(op()),
            },
            SessionMessage::Input {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/in/EDIT00000000").to_path_buf(),
                file: SessionFile::Raw(Arc::from(&b"INPUT_BYTES"[..])),
            },
            SessionMessage::Output {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/out/EDIT00000000").to_path_buf(),
                file: SessionFile::Raw(Arc::from(&b"WRITE_A"[..])),
            },
            SessionMessage::Output {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/out/EDIT00000000").to_path_buf(),
                file: SessionFile::Raw(Arc::from(&b"WRITE_B_BYTES"[..])),
            },
        ]
    }

    /// `folder`'s `journal.toml` as written text and as a parsed table.
    fn journal(folder: &Path) -> (String, toml::Table) {
        let text = fs::read_to_string(folder.join("journal.toml")).unwrap();
        let doc = toml::from_str(&text).unwrap();
        (text, doc)
    }

    /// `folder` read as a bundle: `session_bundle`'s bytes written to a
    /// zip `read_bundle` then reads. These folders hold no packed rows.
    fn folder_bundle(dir: &Path, folder: &Path, home: Option<&str>) -> Bundle {
        let out = dir.join("session.zip");
        fs::write(&out, session_bundle(folder, home).unwrap()).unwrap();
        read_bundle(&out, |_| panic!("the folder holds no packed rows")).unwrap()
    }

    /// A session's frames in order: the header, a step with its op and
    /// one without, a 300 KB kept input, a kept output and both closing
    /// frames (a codec round trip, not a real sequence).
    fn frames() -> Vec<SessionFrame> {
        vec![
            SessionFrame::Header(header(4242)),
            SessionFrame::Message(SessionMessage::Step {
                seconds: 0.5,
                tool: "stub".to_owned(),
                what: "opened C:/Data/in/EDIT00000000 (10 bytes)".to_owned(),
                op: Some(op()),
            }),
            SessionFrame::Message(SessionMessage::Step {
                seconds: 1.25,
                tool: "stub".to_owned(),
                what: "picked the next row".to_owned(),
                op: None,
            }),
            SessionFrame::Message(SessionMessage::Input {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/in/EDIT00000000").to_path_buf(),
                file: SessionFile::Raw(Arc::from(vec![7_u8; 300 * 1024].as_slice())),
            }),
            SessionFrame::Message(SessionMessage::Output {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/out/EDIT00000000").to_path_buf(),
                file: SessionFile::Raw(Arc::from(&b"WRITE_A"[..])),
            }),
            SessionFrame::Closed,
            SessionFrame::Crashed,
        ]
    }

    /// `read` is `written`, field by field; a header's `started`
    /// compares as text so a dropped time zone fails too.
    fn assert_frame_eq(read: &SessionFrame, written: &SessionFrame) {
        match (read, written) {
            (SessionFrame::Header(read), SessionFrame::Header(written)) => {
                assert_eq!(read.version, written.version);
                assert_eq!(read.os, written.os);
                assert_eq!(read.started.to_string(), written.started.to_string());
                assert_eq!(read.pid, written.pid);
            }
            (SessionFrame::Message(read), SessionFrame::Message(written)) => {
                assert_message_eq(read, written)
            }
            (SessionFrame::Closed, SessionFrame::Closed) => {}
            (SessionFrame::Crashed, SessionFrame::Crashed) => {}
            _ => panic!("the frames' kinds differ"),
        }
    }

    /// `read` is `written`, field by field.
    fn assert_message_eq(read: &SessionMessage, written: &SessionMessage) {
        match (read, written) {
            (
                SessionMessage::Step {
                    seconds: r_seconds,
                    tool: r_tool,
                    what: r_what,
                    op: r_op,
                },
                SessionMessage::Step {
                    seconds: w_seconds,
                    tool: w_tool,
                    what: w_what,
                    op: w_op,
                },
            ) => {
                assert_eq!(
                    (r_seconds, r_tool, r_what, r_op),
                    (w_seconds, w_tool, w_what, w_op)
                );
            }
            (
                SessionMessage::Input {
                    tool: r_tool,
                    path: r_path,
                    file: r_file,
                },
                SessionMessage::Input {
                    tool: w_tool,
                    path: w_path,
                    file: w_file,
                },
            )
            | (
                SessionMessage::Output {
                    tool: r_tool,
                    path: r_path,
                    file: r_file,
                },
                SessionMessage::Output {
                    tool: w_tool,
                    path: w_path,
                    file: w_file,
                },
            ) => {
                assert_eq!((r_tool, r_path, r_file), (w_tool, w_path, w_file));
            }
            _ => panic!("the messages' kinds differ"),
        }
    }

    // A session folder holds the header, the step and its op, the input
    // row, one row per write to the output path and the closing `[end]`;
    // `outputs/` holds the second write only. Read back as a bundle it
    // gives the step, the input and the one output.
    // ST-REP-06
    #[test]
    fn a_session_folder_holds_the_record_and_bundles_it() {
        let dir = TestDir::new("session");
        let root = dir.path().join("sessions");
        let mut writer = SessionWriter::new(&root, header(4242));
        for message in messages() {
            writer.write(&message).unwrap();
        }
        writer.end(SessionEnd::Closed).unwrap();

        let folder = root.join("2026-11-21-093015-4242");
        let (text, doc) = journal(&folder);
        assert_eq!(doc["version"].as_str(), Some("v-test"));
        assert_eq!(doc["os"].as_str(), Some("test-os"));
        assert_eq!(
            doc["started"].as_datetime().unwrap().to_string(),
            "2026-11-21T09:30:15+00:00",
            "{text}"
        );
        assert_eq!(doc["pid"].as_integer(), Some(4242));
        let steps = doc["step"].as_array().unwrap();
        assert_eq!(steps.len(), 1, "{text}");
        assert_eq!(steps[0]["seconds"].as_float(), Some(0.5));
        assert_eq!(steps[0]["tool"].as_str(), Some("stub"));
        assert_eq!(
            steps[0]["what"].as_str(),
            Some("opened C:/Data/in/EDIT00000000 (10 bytes)")
        );
        assert_eq!(steps[0]["op"]["kind"].as_str(), Some("player"));
        assert_eq!(steps[0]["op"]["id"].as_integer(), Some(70101));
        let inputs = doc["input"].as_array().unwrap();
        assert_eq!(inputs.len(), 1, "{text}");
        assert_eq!(inputs[0]["tool"].as_str(), Some("stub"));
        assert_eq!(inputs[0]["path"].as_str(), Some("C:/Data/in/EDIT00000000"));
        assert_eq!(inputs[0]["size"].as_integer(), Some(11));
        assert_eq!(inputs[0]["entry"].as_str(), Some("inputs/0-EDIT00000000"));
        let outputs = doc["output"].as_array().unwrap();
        assert_eq!(outputs.len(), 2, "one row per write to the path: {text}");
        assert_eq!(outputs[0]["size"].as_integer(), Some(7));
        assert_eq!(outputs[0]["entry"].as_str(), Some("outputs/0-EDIT00000000"));
        assert_eq!(outputs[1]["size"].as_integer(), Some(13));
        assert_eq!(outputs[1]["entry"].as_str(), Some("outputs/0-EDIT00000000"));
        assert_eq!(doc["end"]["how"].as_str(), Some("closed"), "{text}");
        assert!(
            !text.contains("\n\n# ok"),
            "no blank line before a block's marker: {text}"
        );

        assert_eq!(
            fs::read(folder.join("inputs/0-EDIT00000000")).unwrap(),
            b"INPUT_BYTES"
        );
        let names: Vec<String> = fs::read_dir(folder.join("outputs"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(
            names,
            ["0-EDIT00000000"],
            "the path's one file, replaced at each write"
        );
        assert_eq!(
            fs::read(folder.join("outputs/0-EDIT00000000")).unwrap(),
            b"WRITE_B_BYTES"
        );

        let bundle = folder_bundle(dir.path(), &folder, None);
        assert_eq!(bundle.steps.len(), 1);
        assert_eq!(bundle.steps[0].op, Some(op()));
        assert_eq!(bundle.inputs.len(), 1);
        assert_eq!(bundle.inputs[0].bytes, b"INPUT_BYTES");
        assert_eq!(
            bundle.outputs.len(),
            1,
            "of the path's rows the last counts"
        );
        assert_eq!(bundle.outputs[0].bytes, b"WRITE_B_BYTES");
    }

    // `[end]` says how the session ended — `ended` when the link
    // dropped, `crashed` on a handled crash — and only the first `end`
    // counts: a second writes nothing.
    // ST-REP-07
    #[test]
    fn the_first_end_block_says_how_the_session_ended() {
        let dir = TestDir::new("session_end");
        let root = dir.path().join("sessions");
        let mut writer = SessionWriter::new(&root, header(4242));
        for message in messages() {
            writer.write(&message).unwrap();
        }
        writer.end(SessionEnd::Ended).unwrap();
        writer.end(SessionEnd::Crashed).unwrap();

        let (text, doc) = journal(&root.join("2026-11-21-093015-4242"));
        assert_eq!(
            doc["end"]["how"].as_str(),
            Some("ended"),
            "the first end counts: {text}"
        );
        assert_eq!(
            text.matches("\n[end]\n").count(),
            1,
            "the second end wrote nothing: {text}"
        );

        let mut writer = SessionWriter::new(&root, header(4243));
        for message in messages() {
            writer.write(&message).unwrap();
        }
        writer.end(SessionEnd::Crashed).unwrap();
        let (text, doc) = journal(&root.join("2026-11-21-093015-4243"));
        assert_eq!(doc["end"]["how"].as_str(), Some("crashed"), "{text}");
    }

    // A `journal.toml` whose last `[[step]]` block is cut anywhere — inside a
    // string, on a line boundary or inside an op's number — still gives the
    // earlier steps: the bundle reads up to the last `# ok` marker and the
    // cut block is dropped whole. (The file is written as the writer leaves
    // it: every block ends with its marker line.)
    // ST-REP-07
    #[test]
    fn a_journal_block_cut_mid_write_is_dropped() {
        let dir = TestDir::new("session_cut");
        let folder = dir.path().join("2026-11-21-093015-4242");
        fs::create_dir(&folder).unwrap();
        let head = "version = \"v-test\"\nos = \"test-os\"\n\
            started = 2026-11-21T09:30:15+00:00\npid = 4242\n# ok\n\
            [[step]]\nseconds = 0.5\ntool = \"stub\"\nwhat = \"opened x\"\n# ok\n";
        let last_block = "[[step]]\nseconds = 1.0\ntool = \"stub\"\n\
            what = \"edited x\"\n[step.op]\nkind = \"player\"\nid = 70101\n# ok\n";
        for cut in [
            // On the line boundary after `what`, before `[step.op]`.
            last_block.find("[step.op]").unwrap(),
            // Inside the op's number (`id = 7010` parses as a different op).
            last_block.find("70101").unwrap() + 4,
            // Inside the `what` string.
            last_block.find("edited").unwrap() + 4,
        ] {
            fs::write(
                folder.join("journal.toml"),
                format!("{head}{}", &last_block[..cut]),
            )
            .unwrap();
            let bundle = folder_bundle(dir.path(), &folder, None);
            assert_eq!(
                bundle.steps.len(),
                1,
                "the cut at {cut} leaves only the first step"
            );
            assert_eq!(bundle.steps[0].what, "opened x");
        }
        // Cut inside "→", two of its three bytes written: still the cut
        // block alone is dropped.
        let arrow_block = last_block.replace("edited x", "Height 183 →");
        let mut cut = head.as_bytes().to_vec();
        let arrow = arrow_block.find('→').unwrap();
        cut.extend_from_slice(&arrow_block.as_bytes()[..arrow + 2]);
        fs::write(folder.join("journal.toml"), cut).unwrap();
        let bundle = folder_bundle(dir.path(), &folder, None);
        assert_eq!(
            bundle.steps.len(),
            1,
            "the block cut mid-character is dropped"
        );
    }

    // Two tools writing one path keep two outputs, as `keep_output` does:
    // the session keys an output by its tool and path.
    // ST-REP-06
    #[test]
    fn two_tools_writing_one_path_keep_two_outputs() {
        let dir = TestDir::new("session_two_tools");
        let root = dir.path().join("sessions");
        let mut writer = SessionWriter::new(&root, header(4242));
        for tool in ["stub", "other"] {
            writer
                .write(&SessionMessage::Output {
                    tool: tool.to_owned(),
                    path: Path::new("C:/Data/out/EDIT00000000").to_path_buf(),
                    file: SessionFile::Raw(Arc::from(tool.as_bytes())),
                })
                .unwrap();
        }
        let folder = root.join("2026-11-21-093015-4242");
        let bundle = folder_bundle(dir.path(), &folder, None);
        let bytes: Vec<&[u8]> = bundle.outputs.iter().map(|out| &out.bytes[..]).collect();
        assert_eq!(bytes, [&b"stub"[..], &b"other"[..]]);
    }

    // One session's files in order: two inputs (one file name), two
    // output paths of one tool, then the first path written again. The
    // bundle's outputs keep first-write order, the first holding its
    // second write.
    // ST-REP-06
    #[test]
    fn the_session_keeps_every_input_and_the_outputs_in_write_order() {
        let dir = TestDir::new("session_files");
        let root = dir.path().join("sessions");
        let mut writer = SessionWriter::new(&root, header(4242));
        for (path, bytes) in [
            ("C:/Data/a/EDIT00000000", &b"INPUT_A"[..]),
            ("D:/Data/b/EDIT00000000", &b"INPUT_B"[..]),
        ] {
            writer
                .write(&SessionMessage::Input {
                    tool: "stub".to_owned(),
                    path: Path::new(path).to_path_buf(),
                    file: SessionFile::Raw(Arc::from(bytes)),
                })
                .unwrap();
        }
        for (path, bytes) in [
            ("C:/Data/out/one.bin", &b"OUT1_A"[..]),
            ("C:/Data/out/two.bin", &b"OUT2"[..]),
            ("C:/Data/out/one.bin", &b"OUT1_B"[..]),
        ] {
            writer
                .write(&SessionMessage::Output {
                    tool: "stub".to_owned(),
                    path: Path::new(path).to_path_buf(),
                    file: SessionFile::Raw(Arc::from(bytes)),
                })
                .unwrap();
        }

        let folder = root.join("2026-11-21-093015-4242");
        assert_eq!(
            fs::read(folder.join("inputs/0-EDIT00000000")).unwrap(),
            b"INPUT_A"
        );
        assert_eq!(
            fs::read(folder.join("inputs/1-EDIT00000000")).unwrap(),
            b"INPUT_B"
        );

        let bundle = folder_bundle(dir.path(), &folder, None);
        assert_eq!(bundle.inputs.len(), 2);
        let paths: Vec<&str> = bundle.outputs.iter().map(|out| out.path.as_str()).collect();
        assert_eq!(paths, ["C:/Data/out/one.bin", "C:/Data/out/two.bin"]);
        assert_eq!(
            bundle.outputs[0].bytes, b"OUT1_B",
            "the path's second write counts"
        );
        assert_eq!(bundle.outputs[1].bytes, b"OUT2");
    }

    // A new session's first write makes its folder and leaves the five
    // newest of the seven already there; a folder not named like a
    // session and a plain file are left alone.
    // ST-REP-08
    #[test]
    fn a_new_session_keeps_only_the_five_newest_session_folders() {
        let dir = TestDir::new("session_prune");
        let root = dir.path().join("sessions");
        for day in 10..=16 {
            fs::create_dir_all(root.join(format!("2026-11-{day}-120000-1"))).unwrap();
        }
        fs::create_dir_all(root.join("notes")).unwrap();
        fs::write(root.join("stray.txt"), "not a session").unwrap();
        // Session-shaped but not session names: a letter inside an
        // otherwise well-shaped name, and a three-digit year. Both sort
        // before the dated folders, so a lax name check would prune them.
        for name in ["0000-0a-00-000000-0", "202-11-10-120000-1"] {
            fs::create_dir_all(root.join(name)).unwrap();
        }

        let mut writer = SessionWriter::new(&root, header(4242));
        writer
            .write(&SessionMessage::Step {
                seconds: 0.0,
                tool: "stub".to_owned(),
                what: "a step".to_owned(),
                op: None,
            })
            .unwrap();

        let mut names: Vec<String> = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "0000-0a-00-000000-0",
                "202-11-10-120000-1",
                "2026-11-12-120000-1",
                "2026-11-13-120000-1",
                "2026-11-14-120000-1",
                "2026-11-15-120000-1",
                "2026-11-16-120000-1",
                "2026-11-21-093015-4242",
                "notes",
                "stray.txt",
            ]
        );
    }

    // A window that records nothing makes no folder: `end` alone
    // writes nothing.
    // ST-REP-08
    #[test]
    fn a_session_that_records_nothing_leaves_no_folder() {
        let dir = TestDir::new("session_empty");
        let root = dir.path().join("sessions");
        let mut writer = SessionWriter::new(&root, header(4242));
        writer.end(SessionEnd::Closed).unwrap();
        assert!(!root.exists(), "no folder was ever made");
    }

    // The folder's journal keeps paths whole — it stays on the member's
    // own disk — while the bundle it becomes writes the home folder as
    // `~` in step lines and file paths, as `write_bundle` does.
    #[test]
    fn the_bundle_redacts_the_home_folder_the_journal_keeps_it_whole() {
        let dir = TestDir::new("session_redact");
        let root = dir.path().join("sessions");
        let mut writer = SessionWriter::new(&root, header(4242));
        writer
            .write(&SessionMessage::Step {
                seconds: 0.5,
                tool: "stub".to_owned(),
                what: "opened C:/Users/Alice/EDIT00000000".to_owned(),
                op: None,
            })
            .unwrap();
        writer
            .write(&SessionMessage::Input {
                tool: "stub".to_owned(),
                path: Path::new("C:/Users/Alice/in/EDIT00000000").to_path_buf(),
                file: SessionFile::Raw(Arc::from(&b"SAVE"[..])),
            })
            .unwrap();

        let folder = root.join("2026-11-21-093015-4242");
        let (text, _) = journal(&folder);
        assert!(
            text.contains("C:/Users/Alice/EDIT00000000"),
            "the folder's journal keeps the path whole: {text}"
        );
        assert!(text.contains("C:/Users/Alice/in/EDIT00000000"), "{text}");

        let out = dir.path().join("session.zip");
        fs::write(
            &out,
            session_bundle(&folder, Some("C:/Users/Alice")).unwrap(),
        )
        .unwrap();
        let journal = zip_text(&out, "journal.toml");
        assert!(
            journal.contains("what = \"opened ~/EDIT00000000\""),
            "the step line is redacted: {journal}"
        );
        assert!(
            journal.contains("path = \"~/in/EDIT00000000\""),
            "the input's path is redacted: {journal}"
        );
    }

    // Every frame kind written to the pipe's bytes reads back equal —
    // a kept file's bytes included — and the input's end reads `None`.
    #[test]
    fn frames_written_to_the_pipe_read_back() {
        let mut bytes = Vec::new();
        for frame in frames() {
            write_frame(&mut bytes, &frame).unwrap();
        }

        let mut input = &bytes[..];
        for written in frames() {
            let read = read_frame(&mut input)
                .unwrap()
                .expect("a frame was written");
            assert_frame_eq(&read, &written);
        }
        assert!(
            read_frame(&mut input).unwrap().is_none(),
            "the frames' end is `None`"
        );
    }

    // The pipe cut inside the input's 300 KB body reads the frames
    // before it, then `None`: the window died mid-send and the partial
    // frame is dropped.
    #[test]
    fn a_frame_cut_mid_send_is_dropped() {
        let frames = frames();
        let mut bytes = Vec::new();
        for frame in &frames {
            write_frame(&mut bytes, frame).unwrap();
        }
        // The input is the fourth frame; cut it inside its body.
        let mut head = Vec::new();
        for frame in &frames[..4] {
            write_frame(&mut head, frame).unwrap();
        }
        let cut = head.len() - 150 * 1024;

        let mut input = &bytes[..cut];
        for written in &frames[..3] {
            let read = read_frame(&mut input)
                .unwrap()
                .expect("a frame was written");
            assert_frame_eq(&read, written);
        }
        assert!(
            read_frame(&mut input).unwrap().is_none(),
            "the cut frame is dropped, `None` like the pipe's end"
        );
    }

    /// A `Read` whose every `read` fails with `kind`.
    struct FailingRead(io::ErrorKind);

    impl Read for FailingRead {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }
    }

    // An error of another kind surfaces: only `UnexpectedEof` — a window
    // dying mid-send — reads as the pipe's end.
    #[test]
    fn a_read_error_that_is_not_eof_is_an_error() {
        let mut input = FailingRead(io::ErrorKind::BrokenPipe);
        let error = match read_frame(&mut input) {
            Ok(_) => panic!("the read did not fail"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    }

    // A second writer with the same header adopts the folder the first
    // made: the journal's bytes up to the first writer's end stand, the
    // next input lands at the row count's index, and a write to the
    // output's tool and path replaces `outputs/0-…`.
    // ST-REP-10
    #[test]
    fn a_second_writer_adopts_the_folder_and_writes_on() {
        let dir = TestDir::new("session_adopt");
        let root = dir.path().join("sessions");
        let mut first = SessionWriter::new(&root, header(4242));
        for (path, bytes) in [
            ("C:/Data/a/EDIT00000000", &b"INPUT_A"[..]),
            ("D:/Data/b/EDIT00000000", &b"INPUT_B"[..]),
        ] {
            first
                .write(&SessionMessage::Input {
                    tool: "stub".to_owned(),
                    path: Path::new(path).to_path_buf(),
                    file: SessionFile::Raw(Arc::from(bytes)),
                })
                .unwrap();
        }
        for (path, bytes) in [
            ("C:/Data/out/EDIT00000000", &b"WRITE_A"[..]),
            ("C:/Data/out/other.bin", &b"WRITE_OTHER"[..]),
        ] {
            first
                .write(&SessionMessage::Output {
                    tool: "stub".to_owned(),
                    path: Path::new(path).to_path_buf(),
                    file: SessionFile::Raw(Arc::from(bytes)),
                })
                .unwrap();
        }
        let folder = root.join("2026-11-21-093015-4242");
        let written = fs::read(folder.join("journal.toml")).unwrap();

        let mut second = SessionWriter::new(&root, header(4242));
        second
            .write(&SessionMessage::Input {
                tool: "stub".to_owned(),
                path: Path::new("E:/Data/c/EDIT00000000").to_path_buf(),
                file: SessionFile::Raw(Arc::from(&b"INPUT_C"[..])),
            })
            .unwrap();
        second
            .write(&SessionMessage::Output {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/out/other.bin").to_path_buf(),
                file: SessionFile::Raw(Arc::from(&b"WRITE_B"[..])),
            })
            .unwrap();

        assert_eq!(
            fs::read(folder.join("inputs/2-EDIT00000000")).unwrap(),
            b"INPUT_C",
            "the third input, not a second inputs/0"
        );
        assert_eq!(
            fs::read(folder.join("outputs/1-other.bin")).unwrap(),
            b"WRITE_B",
            "the adopted output path keeps its first index"
        );
        let journal = fs::read(folder.join("journal.toml")).unwrap();
        assert_eq!(
            &journal[..written.len()],
            &written[..],
            "the first writer's bytes are unchanged"
        );
    }

    // A `journal.toml` that cannot be read — a directory here, not a
    // missing file — is the first write's error naming it, never a fresh
    // folder made over it.
    // ST-REP-10
    #[test]
    fn an_unreadable_journal_is_an_error_not_a_fresh_folder() {
        let dir = TestDir::new("session_journal_dir");
        let root = dir.path().join("sessions");
        let folder = root.join("2026-11-21-093015-4242");
        fs::create_dir_all(folder.join("journal.toml")).unwrap();

        let mut writer = SessionWriter::new(&root, header(4242));
        let error = writer
            .write(&SessionMessage::Step {
                seconds: 0.5,
                tool: "stub".to_owned(),
                what: "a step".to_owned(),
                op: None,
            })
            .expect_err("the unreadable journal is the write's error");
        assert!(
            format!("{error:#}").contains("journal.toml"),
            "the error names the journal: {error:#}"
        );
        assert!(
            !folder.join("inputs").exists() && !folder.join("outputs").exists(),
            "no fresh folder was made"
        );
    }

    // The adopted journal's output rows give the writer's index one entry
    // each: a path's second row (its rewritten output) does not take a
    // second index, and a row sharing only the tool or only the path is a
    // new entry.
    // ST-REP-10
    #[test]
    fn adoption_keeps_one_index_per_tool_and_path() {
        let dir = TestDir::new("session_adopt_dedupe");
        let root = dir.path().join("sessions");
        let mut first = SessionWriter::new(&root, header(4242));
        for (tool, path, bytes) in [
            ("stub", "C:/Data/out/a.bin", &b"A_FIRST"[..]),
            ("stub", "C:/Data/out/a.bin", &b"A_SECOND"[..]),
            ("stub", "C:/Data/out/b.bin", &b"B"[..]),
            ("other", "C:/Data/out/a.bin", &b"OTHER_A"[..]),
        ] {
            first
                .write(&SessionMessage::Output {
                    tool: tool.to_owned(),
                    path: Path::new(path).to_path_buf(),
                    file: SessionFile::Raw(Arc::from(bytes)),
                })
                .unwrap();
        }

        let mut second = SessionWriter::new(&root, header(4242));
        second.end(SessionEnd::Closed).unwrap();
        assert_eq!(
            second.outputs,
            [
                (
                    "stub".to_owned(),
                    Path::new("C:/Data/out/a.bin").to_path_buf()
                ),
                (
                    "stub".to_owned(),
                    Path::new("C:/Data/out/b.bin").to_path_buf()
                ),
                (
                    "other".to_owned(),
                    Path::new("C:/Data/out/a.bin").to_path_buf()
                ),
            ],
            "the path's second row keeps its one index"
        );
    }

    // A journal left with half a block past its last marker is cut back
    // there on adoption: the second writer's step lands where the
    // fragment sat and the bundle reads every whole block and it.
    // ST-REP-10
    #[test]
    fn adoption_cuts_the_journal_back_to_its_last_marker() {
        let dir = TestDir::new("session_adopt_cut");
        let root = dir.path().join("sessions");
        let mut first = SessionWriter::new(&root, header(4242));
        first
            .write(&SessionMessage::Step {
                seconds: 0.5,
                tool: "stub".to_owned(),
                what: "the first writer's step".to_owned(),
                op: None,
            })
            .unwrap();
        let folder = root.join("2026-11-21-093015-4242");
        // The fragment a monitor dying mid-block would leave: a `[[step]]`
        // and a cut `what` line with no marker after them.
        fs::OpenOptions::new()
            .append(true)
            .open(folder.join("journal.toml"))
            .unwrap()
            .write_all(b"[[step]]\nwhat = \"cut")
            .unwrap();

        let mut second = SessionWriter::new(&root, header(4242));
        second
            .write(&SessionMessage::Step {
                seconds: 1.0,
                tool: "stub".to_owned(),
                what: "the second writer's step".to_owned(),
                op: None,
            })
            .unwrap();

        let bundle = folder_bundle(dir.path(), &folder, None);
        let whats: Vec<&str> = bundle.steps.iter().map(|step| step.what.as_str()).collect();
        assert_eq!(
            whats,
            ["the first writer's step", "the second writer's step"],
            "the whole blocks and the new step, the fragment gone"
        );
    }

    // A writer whose first call is `end` adopts the folder another
    // writer left: its `[end]` lands in it.
    // ST-REP-10
    #[test]
    fn an_end_before_any_write_adopts_the_folder_and_ends_it() {
        let dir = TestDir::new("session_end_adopt");
        let root = dir.path().join("sessions");
        let mut first = SessionWriter::new(&root, header(4242));
        first
            .write(&SessionMessage::Step {
                seconds: 0.5,
                tool: "stub".to_owned(),
                what: "the first writer's step".to_owned(),
                op: None,
            })
            .unwrap();

        let mut second = SessionWriter::new(&root, header(4242));
        second.end(SessionEnd::Closed).unwrap();

        let folder = root.join("2026-11-21-093015-4242");
        let (text, doc) = journal(&folder);
        assert_eq!(doc["end"]["how"].as_str(), Some("closed"), "{text}");
        assert_eq!(doc["step"].as_array().unwrap().len(), 1, "{text}");
    }

    // A journal already holding `[end]` keeps it on adoption: `end`
    // writes no second one — two `[end]` tables would not parse — and
    // the first `how` stands.
    // ST-REP-10
    #[test]
    fn an_adopted_end_counts_as_the_first() {
        let dir = TestDir::new("session_end_twice");
        let root = dir.path().join("sessions");
        let mut first = SessionWriter::new(&root, header(4242));
        first
            .write(&SessionMessage::Step {
                seconds: 0.5,
                tool: "stub".to_owned(),
                what: "the first writer's step".to_owned(),
                op: None,
            })
            .unwrap();
        first.end(SessionEnd::Closed).unwrap();

        let mut second = SessionWriter::new(&root, header(4242));
        second.end(SessionEnd::Ended).unwrap();

        let folder = root.join("2026-11-21-093015-4242");
        let (text, doc) = journal(&folder);
        assert_eq!(
            text.matches("\n[end]\n").count(),
            1,
            "the adopted [end] stands alone: {text}"
        );
        assert_eq!(doc["end"]["how"].as_str(), Some("closed"), "{text}");
        let bundle = folder_bundle(dir.path(), &folder, None);
        assert_eq!(bundle.steps.len(), 1, "the journal still parses");
    }

    // `started` keeps its own time zone through the pipe: a +01:00
    // header reads back at +01:00, not at the reader's zone.
    #[test]
    fn the_headers_started_keeps_its_time_zone() {
        let mut written = header(4242);
        written.started = jiff::civil::date(2026, 11, 21)
            .at(9, 30, 15, 0)
            .to_zoned(jiff::tz::TimeZone::fixed(jiff::tz::offset(1)))
            .unwrap();
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &SessionFrame::Header(written)).unwrap();

        let Some(SessionFrame::Header(read)) = read_frame(&mut &bytes[..]).unwrap() else {
            panic!("the header frame did not read back")
        };
        assert_eq!(
            read.started.to_string(),
            "2026-11-21T09:30:15+01:00[+01:00]"
        );
    }

    // A packed input and output survive the pipe: the `packed` table in
    // the frame's TOML, the packed form as the body's raw part, both
    // kinds reading back with the file's own size and SHA-256.
    #[test]
    fn a_packed_input_and_output_survive_the_pipe() {
        let file = |packed: &'static [u8]| SessionFile::Packed {
            packed: Arc::from(packed),
            size: 14,
            sha256: sha256(b"THE_FILE_BYTES"),
        };
        let frames = [
            SessionFrame::Message(SessionMessage::Input {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/in/EDIT00000000").to_path_buf(),
                file: file(b"PACKED_IN"),
            }),
            SessionFrame::Message(SessionMessage::Output {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/out/EDIT00000000").to_path_buf(),
                file: file(b"PACKED_OUT"),
            }),
        ];
        let mut bytes = Vec::new();
        for frame in &frames {
            write_frame(&mut bytes, frame).unwrap();
        }

        let mut input = &bytes[..];
        for written in &frames {
            let read = read_frame(&mut input)
                .unwrap()
                .expect("a frame was written");
            assert_frame_eq(&read, written);
        }
        assert!(
            read_frame(&mut input).unwrap().is_none(),
            "the frames' end is `None`"
        );
    }

    /// `header`'s `started` moved to `hh:mm:ss` of the same day, so two
    /// tests' folders sort by it.
    fn header_at(pid: u32, hour: i8, minute: i8, second: i8) -> SessionHeader {
        SessionHeader {
            started: jiff::civil::date(2026, 11, 21)
                .at(hour, minute, second, 0)
                .to_zoned(jiff::tz::TimeZone::UTC)
                .unwrap(),
            ..header(pid)
        }
    }

    /// `session`'s folder's name.
    fn session_name(session: &EarlierSession) -> String {
        session
            .folder
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    // Every session-named folder is listed newest first, each field from
    // the folder: the start from the name, the first `[[input]]` row's
    // file name, the newest `outputs/` file's modification time and
    // `[end]`. A folder not named like a session, a session-named file
    // and a journal that does not parse are left out, and `own` names
    // the window's own, which the list never holds.
    #[test]
    fn earlier_sessions_lists_each_kept_session_newest_first() {
        let dir = TestDir::new("earlier_sessions");
        let root = dir.path().join("sessions");
        let input = |tool: &str, path: &str, bytes: &'static [u8]| SessionMessage::Input {
            tool: tool.to_owned(),
            path: Path::new(path).to_path_buf(),
            file: SessionFile::Raw(Arc::from(bytes)),
        };

        // 09:30:15 — `closed`, an input and an output.
        let mut writer = SessionWriter::new(&root, header_at(4242, 9, 30, 15));
        for message in [
            input("stub", "C:/Data/in/EDIT00000000", b"INPUT"),
            SessionMessage::Output {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/out/EDIT00000000").to_path_buf(),
                file: SessionFile::Raw(Arc::from(&b"OUTPUT"[..])),
            },
        ] {
            writer.write(&message).unwrap();
        }
        writer.end(SessionEnd::Closed).unwrap();

        // 09:31:15 — `crashed`, an input and no output.
        let mut writer = SessionWriter::new(&root, header_at(4243, 9, 31, 15));
        writer
            .write(&input("stub", "C:/Data/in/EDIT00000001", b"INPUT_B"))
            .unwrap();
        writer.end(SessionEnd::Crashed).unwrap();

        // 09:32:15 — no `[end]`, as a session still open or a writer that
        // died with the window; a step alone, so no input or output.
        let mut writer = SessionWriter::new(&root, header_at(4244, 9, 32, 15));
        writer
            .write(&SessionMessage::Step {
                seconds: 0.0,
                tool: "stub".to_owned(),
                what: "a step".to_owned(),
                op: None,
            })
            .unwrap();

        // Left out: a folder not named like a session, a session-named
        // plain file, and a session name whose journal does not parse.
        fs::create_dir_all(root.join("notes")).unwrap();
        fs::write(root.join("2026-11-21-095015-4246"), "a file").unwrap();
        fs::create_dir_all(root.join("2026-11-21-094015-4245")).unwrap();
        fs::write(
            root.join("2026-11-21-094015-4245/journal.toml"),
            "not a journal [[[",
        )
        .unwrap();

        let sessions = earlier_sessions(&root, None);
        assert_eq!(sessions.len(), 3, "the three whole folders only");
        let names: Vec<String> = sessions.iter().map(session_name).collect();
        assert_eq!(
            names,
            [
                "2026-11-21-093215-4244",
                "2026-11-21-093115-4243",
                "2026-11-21-093015-4242"
            ],
            "newest first"
        );
        let [unknown, crashed, closed] = &sessions[..] else {
            panic!("three sessions")
        };
        assert_eq!(
            unknown.started,
            jiff::civil::date(2026, 11, 21).at(9, 32, 15, 0)
        );
        assert_eq!(unknown.opened, None, "no input row");
        assert_eq!(unknown.wrote, None, "no file under outputs/");
        assert_eq!(unknown.ended, None, "no [end]");
        assert_eq!(crashed.opened.as_deref(), Some("EDIT00000001"));
        assert_eq!(crashed.wrote, None);
        assert_eq!(crashed.ended, Some(SessionEnd::Crashed));
        assert_eq!(closed.opened.as_deref(), Some("EDIT00000000"));
        let wrote = fs::metadata(root.join("2026-11-21-093015-4242/outputs/0-EDIT00000000"))
            .unwrap()
            .modified()
            .unwrap();
        assert_eq!(
            closed.wrote,
            Some(wrote),
            "the newest output's modification time"
        );
        assert_eq!(closed.ended, Some(SessionEnd::Closed));

        // `own` names the window's own: the list holds the other two.
        let sessions = earlier_sessions(&root, Some("2026-11-21-093115-4243"));
        let names: Vec<String> = sessions.iter().map(session_name).collect();
        assert_eq!(names, ["2026-11-21-093215-4244", "2026-11-21-093015-4242"]);
    }

    /// A session folder under `dir` holding one packed input of `file`,
    /// ended `Closed`.
    fn packed_folder(dir: &TestDir, file: SessionFile) -> PathBuf {
        let root = dir.path().join("sessions");
        let mut writer = SessionWriter::new(&root, header(4242));
        writer
            .write(&SessionMessage::Input {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/in/EDIT00000000").to_path_buf(),
                file,
            })
            .unwrap();
        writer.end(SessionEnd::Closed).unwrap();
        root.join("2026-11-21-093015-4242")
    }

    // A packed input is kept as `inputs/0-<name>.packed`, compressed to
    // the packed form, its row holding `packed = true` and the file's
    // own size and SHA-256. The bundle the folder makes reads it
    // through `rebuild` (here the toy pack, the file's first half) and
    // hands back the file.
    // ST-REP-06
    #[test]
    fn a_packed_input_is_kept_compressed_and_rebuilt() {
        let dir = TestDir::new("session_packed");
        // The toy pack keeps the file's first half; `rebuild` doubles
        // it back, the halves being equal. Shorter than the file, so a
        // row's `size` holding the packed form's length stands out.
        let raw = b"THE_SAMETHE_SAME".to_vec();
        let form: Vec<u8> = raw[..raw.len() / 2].to_vec();
        let folder = packed_folder(
            &dir,
            SessionFile::Packed {
                packed: form.clone().into(),
                size: raw.len(),
                sha256: sha256(&raw),
            },
        );

        let entry = folder.join("inputs/0-EDIT00000000.packed");
        let mut inflated = Vec::new();
        flate2::read::ZlibDecoder::new(fs::File::open(&entry).unwrap())
            .read_to_end(&mut inflated)
            .unwrap();
        assert_eq!(inflated, form, "the entry inflates to the packed form");

        let (text, doc) = journal(&folder);
        let inputs = doc["input"].as_array().unwrap();
        assert_eq!(inputs[0]["packed"].as_bool(), Some(true), "{text}");
        assert_eq!(
            inputs[0]["size"].as_integer(),
            Some(16),
            "the file's size, not the packed form's 8: {text}"
        );
        assert_eq!(
            inputs[0]["sha256"].as_str(),
            Some(sha256(&raw).as_str()),
            "{text}"
        );
        assert_eq!(
            inputs[0]["entry"].as_str(),
            Some("inputs/0-EDIT00000000.packed"),
            "{text}"
        );

        // `session_bundle` copies the entry as it is; the row's
        // `packed` comes with it.
        let out = dir.path().join("session.zip");
        fs::write(&out, session_bundle(&folder, None).unwrap()).unwrap();
        let journal_text = zip_text(&out, "journal.toml");
        assert!(journal_text.contains("packed = true"), "{journal_text}");

        let bundle = read_bundle(&out, |form: &[u8]| -> anyhow::Result<Vec<u8>> {
            Ok(form.repeat(2))
        })
        .unwrap();
        assert_eq!(bundle.inputs[0].bytes, raw, "the rebuilt file");
    }

    // A `rebuild` whose output is not the kept file is refused naming
    // the entry — the rebuilt bytes take the row's size and SHA-256
    // checks like a raw entry's do.
    #[test]
    fn a_rebuilt_entry_mismatching_its_row_is_refused() {
        let dir = TestDir::new("session_bad_rebuild");
        let raw = b"THE_FILE_BYTES".to_vec();
        let form: Vec<u8> = raw.iter().rev().copied().collect();
        let folder = packed_folder(
            &dir,
            SessionFile::Packed {
                packed: form.into(),
                size: raw.len(),
                sha256: sha256(&raw),
            },
        );
        let out = dir.path().join("session.zip");
        fs::write(&out, session_bundle(&folder, None).unwrap()).unwrap();

        // The same size as the row's, so the SHA-256 is what differs.
        let wrong = |_: &[u8]| -> anyhow::Result<Vec<u8>> { Ok(vec![0; 14]) };
        let error = match read_bundle(&out, wrong) {
            Ok(_) => panic!("the rebuilt bundle read"),
            Err(error) => error,
        };
        let text = format!("{error:#}");
        assert!(text.contains("rebuilt"), "{text}");
        assert!(text.contains("SHA-256"), "{text}");
        assert!(
            text.contains("inputs/0-EDIT00000000.packed"),
            "the refusal names the entry: {text}"
        );
    }
}
