//! The session folder: `<data location>/sessions/<yyyy>-<mm>-<dd>-<hhmmss>-<pid>/`,
//! one per window, written as the session happens so a member who closed
//! Studio before finding a problem in game still has what that session
//! did. `journal.toml` opens with the build, the OS, the start time and
//! the process id, then takes one `[[step]]` block per step, one
//! `[[input]]` or `[[output]]` row per kept file, and an `[end]` table
//! last saying how the session ended. `inputs/` holds every file read;
//! `outputs/` the last file written per path, replaced at each write.
//!
//! The crash monitor writes the folder from the window's session
//! messages (`SessionWriter`); a window whose monitor did not start runs
//! the same writer on a thread of its own. `session_bundle` turns a kept
//! folder into the bytes of a repro bundle `read_bundle` reads.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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
    /// A file kept as it was read, as `Recorder::keep_input` records it.
    Input {
        /// The recording tool's id.
        tool: String,
        /// The path the file was read from.
        path: PathBuf,
        /// The file's bytes.
        bytes: Vec<u8>,
    },
    /// A file kept as it was written, as `Recorder::keep_output`
    /// records it: a later write to one tool and path replaces it.
    Output {
        /// The recording tool's id.
        tool: String,
        /// The path the file was written to.
        path: PathBuf,
        /// The file's bytes.
        bytes: Vec<u8>,
    },
}

/// How the session ended, written as `[end]`'s `how`: `closed` (the
/// window sent its closing message), `crashed` (the monitor handled the
/// crash) or `ended` (the window went away without either, as when it
/// is ended from Task Manager). A journal with no `[end]` is a session
/// still open, or one whose monitor died too.
pub enum SessionEnd {
    /// The window sent its closing message.
    Closed,
    /// The monitor handled the window's crash.
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
/// nothing leaves none. Blocks are appended whole, one write each, so a
/// crash can cut at most the block in flight — which `session_bundle`
/// drops on reading.
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
            SessionMessage::Input { tool, path, bytes } => {
                let index = self.inputs;
                self.inputs += 1;
                let row = keep_file(&folder, "inputs", index, tool, path, bytes)?;
                append_block(&journal, &InputBlock { input: vec![row] })?;
            }
            SessionMessage::Output { tool, path, bytes } => {
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
                let row = keep_file(&folder, "outputs", index, tool, path, bytes)?;
                append_block(&journal, &OutputBlock { output: vec![row] })?;
            }
        }
        Ok(())
    }

    /// Appends `[end]` with `how` the session ended. Only the first
    /// `end` counts — a later one writes nothing — and an `end` before
    /// the first `write` writes nothing: no folder was made.
    pub fn end(&mut self, end: SessionEnd) -> anyhow::Result<()> {
        if self.ended {
            return Ok(());
        }
        let Some(folder) = &self.folder else {
            return Ok(());
        };
        let block = EndBlock {
            end: EndRow {
                how: end.how().to_owned(),
            },
        };
        append_block(&folder.join("journal.toml"), &block)?;
        self.ended = true;
        Ok(())
    }

    /// The session's folder, made when this is the first `write`:
    /// `<root>/<yyyy>-<mm>-<dd>-<hhmmss>-<pid>/` from the header's local
    /// time and pid, holding `journal.toml`'s header lines and the empty
    /// `inputs/` and `outputs/`. Making it prunes `root` to the kept
    /// newest folders.
    fn ensure_folder(&mut self) -> anyhow::Result<PathBuf> {
        if self.folder.is_none() {
            let name = folder_name(&self.header);
            let folder = self.root.join(&name);
            for dir in ["inputs", "outputs"] {
                fs::create_dir_all(folder.join(dir)).with_context(|| {
                    format!("{}: cannot make the session folder", folder.display())
                })?;
            }
            let text = toml_block(&SessionDoc::header(&self.header)?)?;
            let journal = folder.join("journal.toml");
            replace_file(&journal, |tmp| fs::write(tmp, &text))
                .with_context(|| format!("{}: cannot write the journal", journal.display()))?;
            prune(&self.root, &name);
            self.folder = Some(folder);
        }
        Ok(self.folder.clone().expect("the folder was just made"))
    }
}

/// `bytes` written under `folder` as `<dir>/<index>-<file name>`
/// through `replace_file` — the row then appended is for a file
/// already whole in place.
fn keep_file(
    folder: &Path,
    dir: &str,
    index: usize,
    tool: &str,
    path: &Path,
    bytes: &[u8],
) -> anyhow::Result<FileDoc> {
    let entry = entry(dir, index, path);
    let file = folder.join(&entry);
    replace_file(&file, |tmp| fs::write(tmp, bytes))
        .with_context(|| format!("{}: cannot keep the file", file.display()))?;
    Ok(FileDoc {
        tool: tool.to_owned(),
        path: path.display().to_string(),
        size: bytes.len(),
        sha256: sha256(bytes),
        entry,
    })
}

/// `header`'s session folder name, `<yyyy>-<mm>-<dd>-<hhmmss>-<pid>`
/// from its start's local time, so folders sort by when they were made.
fn folder_name(header: &SessionHeader) -> String {
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

/// `value`'s TOML, ending with a newline so the next appended block
/// starts on its own line.
fn toml_block(value: &impl Serialize) -> anyhow::Result<String> {
    let mut text = toml::to_string_pretty(value).context("cannot serialize the journal block")?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

/// `text`'s byte offset where its last block begins — the last line
/// opening a `[[step]]`, `[[input]]`, `[[output]]` or `[end]` — for the
/// cut-write retry. `None` when no line opens one.
fn last_block_offset(text: &str) -> Option<usize> {
    let mut last = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if matches!(
            line.trim(),
            "[[step]]" | "[[input]]" | "[[output]]" | "[end]"
        ) {
            last = Some(offset);
        }
        offset += line.len();
    }
    last
}

/// `folder`, a session folder, as a bundle's zip bytes: `journal.toml`
/// rewritten as `write_bundle` writes it — every path and step line
/// with `home` as `~`, and of a path's `[[output]]` rows the last kept
/// at the index of its first — plus each row's file from `inputs/` or
/// `outputs/`, so `read_bundle` reads it like any other. A journal that
/// does not parse is tried once more without its last block, so a block
/// cut off mid-write is lost and nothing else; a second failure, or a
/// first with no block to drop, is an error naming the folder.
pub fn session_bundle(folder: &Path, home: Option<&str>) -> anyhow::Result<Vec<u8>> {
    let journal = folder.join("journal.toml");
    let bytes = fs::read(&journal)
        .with_context(|| format!("{}: cannot read the journal", journal.display()))?;
    // Lossy, not strict: a cut inside a multi-byte character ("→" in
    // every field line) must reach the cut-block retry, not fail here.
    let text = String::from_utf8_lossy(&bytes);
    let mut doc: SessionDoc = match toml::from_str(&text) {
        Ok(doc) => doc,
        Err(first) => match last_block_offset(&text) {
            Some(cut) => toml::from_str(&text[..cut])
                .with_context(|| format!("{}: cannot parse the journal", folder.display()))?,
            None => {
                return Err(first)
                    .with_context(|| format!("{}: cannot parse the journal", folder.display()));
            }
        },
    };
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

    let text = toml_block(&doc)?;
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
                bytes: b"INPUT_BYTES".to_vec(),
            },
            SessionMessage::Output {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/out/EDIT00000000").to_path_buf(),
                bytes: b"WRITE_A".to_vec(),
            },
            SessionMessage::Output {
                tool: "stub".to_owned(),
                path: Path::new("C:/Data/out/EDIT00000000").to_path_buf(),
                bytes: b"WRITE_B_BYTES".to_vec(),
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
    /// zip `read_bundle` then reads.
    fn folder_bundle(dir: &Path, folder: &Path, home: Option<&str>) -> Bundle {
        let out = dir.join("session.zip");
        fs::write(&out, session_bundle(folder, home).unwrap()).unwrap();
        read_bundle(&out).unwrap()
    }

    // A session folder holds the header, the step and its op, the input
    // row, one row per write to the output path and the closing `[end]`;
    // `outputs/` holds the second write only. Read back as a bundle it
    // gives the step, the input and the one output.
    // ST-REP-06
    #[test]
    fn a_session_folder_holds_the_record_and_bundles_it() {
        let dir = TestDir::new("studio_core_session");
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
        let dir = TestDir::new("studio_core_session_end");
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

    // A `journal.toml` whose last `[[step]]` block is cut in the middle
    // of its `what` value still gives the earlier steps: the bundle's
    // journal drops the cut block and nothing else.
    // ST-REP-07
    #[test]
    fn a_journal_block_cut_mid_write_is_dropped() {
        let dir = TestDir::new("studio_core_session_cut");
        let folder = dir.path().join("2026-11-21-093015-4242");
        fs::create_dir(&folder).unwrap();
        let text = "version = \"v-test\"\nos = \"test-os\"\n\
            started = 2026-11-21T09:30:15+00:00\npid = 4242\n\
            [[step]]\nseconds = 0.5\ntool = \"stub\"\nwhat = \"opened x\"\n\
            [[step]]\nseconds = 1.0\ntool = \"stub\"\nwhat = \"cut in the mid";
        fs::write(folder.join("journal.toml"), text).unwrap();

        let bundle = folder_bundle(dir.path(), &folder, None);
        assert_eq!(bundle.steps.len(), 1, "the cut block is dropped");
        assert_eq!(bundle.steps[0].what, "opened x");

        // Cut inside "→", two of its three bytes written: still the cut
        // block alone is dropped.
        let mut cut = text.replace("cut in the mid", "Height 183 ").into_bytes();
        cut.extend_from_slice(&"→".as_bytes()[..2]);
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
        let dir = TestDir::new("studio_core_session_two_tools");
        let root = dir.path().join("sessions");
        let mut writer = SessionWriter::new(&root, header(4242));
        for tool in ["stub", "other"] {
            writer
                .write(&SessionMessage::Output {
                    tool: tool.to_owned(),
                    path: Path::new("C:/Data/out/EDIT00000000").to_path_buf(),
                    bytes: tool.as_bytes().to_vec(),
                })
                .unwrap();
        }
        let folder = root.join("2026-11-21-093015-4242");
        let bundle = folder_bundle(dir.path(), &folder, None);
        let bytes: Vec<&[u8]> = bundle.outputs.iter().map(|out| &out.bytes[..]).collect();
        assert_eq!(bytes, [&b"stub"[..], &b"other"[..]]);
    }

    // A new session's first write makes its folder and leaves the five
    // newest of the seven already there; a folder not named like a
    // session and a plain file are left alone.
    // ST-REP-08
    #[test]
    fn a_new_session_keeps_only_the_five_newest_session_folders() {
        let dir = TestDir::new("studio_core_session_prune");
        let root = dir.path().join("sessions");
        for day in 10..=16 {
            fs::create_dir_all(root.join(format!("2026-11-{day}-120000-1"))).unwrap();
        }
        fs::create_dir_all(root.join("notes")).unwrap();
        fs::write(root.join("stray.txt"), "not a session").unwrap();

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
        let dir = TestDir::new("studio_core_session_empty");
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
        let dir = TestDir::new("studio_core_session_redact");
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
                bytes: b"SAVE".to_vec(),
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
}
