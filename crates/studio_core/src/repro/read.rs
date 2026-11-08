//! Reading a bundle back, the half `utw <tool-id> replay` runs on:
//! `journal.toml`'s steps and every kept file read from its row's zip
//! entry, each checked against the row's size and SHA-256 so a corrupted
//! or hand-edited bundle is refused, not replayed.

use std::io::Read;
use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

use super::bundle::sha256;

/// A repro bundle read back: the journal's version and steps, and every
/// kept input and output in the journal rows' order.
pub struct Bundle {
    /// The `version` line the bundle was written with
    /// (`crate::shell::version_label` of the recording build).
    pub version: String,
    /// Every `[[step]]` row, in journal order.
    pub steps: Vec<BundleStep>,
    /// Every `[[input]]`'s file, read from its row's zip entry.
    pub inputs: Vec<BundleFile>,
    /// Every `[[output]]`'s file, read from its row's zip entry.
    pub outputs: Vec<BundleFile>,
}

/// One `[[step]]` row: the tool that recorded it, its line, and the
/// typed change a replay applies when the step carried one.
pub struct BundleStep {
    /// The recording tool's id.
    pub tool: String,
    /// The step's line.
    pub what: String,
    /// The step's typed change (`[step.op]`); `None` on a note.
    pub op: Option<toml::Table>,
}

/// A kept file read back: its path as the journal wrote it (the home
/// folder as `~`) and its bytes from the zip entry.
pub struct BundleFile {
    /// The file's path, redacted.
    pub path: String,
    /// The entry's bytes.
    pub bytes: Vec<u8>,
}

/// `journal.toml` as parsed: only the parts replay needs.
#[derive(Deserialize)]
struct Doc {
    /// The `version` line.
    version: String,
    /// The `[[step]]` rows; absent on a journal of none.
    #[serde(default)]
    step: Vec<StepRow>,
    /// The `[[input]]` rows.
    #[serde(default)]
    input: Vec<FileRow>,
    /// The `[[output]]` rows.
    #[serde(default)]
    output: Vec<FileRow>,
}

/// One `[[step]]` row as parsed.
#[derive(Deserialize)]
struct StepRow {
    /// The recording tool's id.
    tool: String,
    /// The step's line.
    what: String,
    /// The `[step.op]` table when the row carries one.
    op: Option<toml::Table>,
}

/// One `[[input]]`/`[[output]]` row as parsed.
#[derive(Deserialize)]
struct FileRow {
    /// The file's path, redacted.
    path: String,
    /// The byte count the row declares.
    size: usize,
    /// The SHA-256 the row declares, lowercase hex.
    sha256: String,
    /// The file's name in the zip (`inputs/<i>-<name>`).
    entry: String,
}

/// `rows`' files read back from the zip, each checked: the entry must be
/// there, its byte count the row's `size`, its SHA-256 the row's. An
/// error names the bundle's path and the entry.
fn kept_files(
    archive: &mut zip::ZipArchive<std::fs::File>,
    path: &Path,
    rows: &[FileRow],
) -> anyhow::Result<Vec<BundleFile>> {
    let mut kept = Vec::with_capacity(rows.len());
    for row in rows {
        let mut entry = archive.by_name(&row.entry).with_context(|| {
            format!(
                "{}: the journal's {} is not in the bundle",
                path.display(),
                row.entry
            )
        })?;
        let mut bytes = Vec::with_capacity(row.size);
        entry
            .read_to_end(&mut bytes)
            .with_context(|| format!("{}: cannot read {}", path.display(), row.entry))?;
        if bytes.len() != row.size {
            anyhow::bail!(
                "{}: {} is {} bytes, the journal says {}",
                path.display(),
                row.entry,
                bytes.len(),
                row.size
            );
        }
        if sha256(&bytes) != row.sha256 {
            anyhow::bail!(
                "{}: {}'s bytes do not match the journal's SHA-256",
                path.display(),
                row.entry
            );
        }
        kept.push(BundleFile {
            path: row.path.clone(),
            bytes,
        });
    }
    Ok(kept)
}

/// Reads the bundle at `path`. Errors name the path and what is wrong:
/// not a zip, no `journal.toml`, an unparseable journal, a row's entry
/// missing from the zip, an entry whose size or SHA-256 differs from
/// its row's.
pub fn read_bundle(path: &Path) -> anyhow::Result<Bundle> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("{}: cannot open the bundle", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("{}: not a bundle (not a zip)", path.display()))?;
    let mut text = String::new();
    archive
        .by_name("journal.toml")
        .with_context(|| format!("{}: not a bundle (no journal.toml)", path.display()))?
        .read_to_string(&mut text)
        .with_context(|| format!("{}: cannot read the journal", path.display()))?;
    let doc: Doc = toml::from_str(&text)
        .with_context(|| format!("{}: cannot parse the journal", path.display()))?;
    Ok(Bundle {
        version: doc.version,
        steps: doc
            .step
            .into_iter()
            .map(|row| BundleStep {
                tool: row.tool,
                what: row.what,
                op: row.op,
            })
            .collect(),
        inputs: kept_files(&mut archive, path, &doc.input)?,
        outputs: kept_files(&mut archive, path, &doc.output)?,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::sync::Arc;

    use super::*;
    use crate::repro::Recorder;
    use crate::repro::tests::scratch;

    /// A zip at `path` holding `entries` in order.
    fn zip_of(path: &Path, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        for (name, bytes) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }

    /// `read_bundle`'s error on `path` (a `Bundle` holds the files' bytes
    /// and prints no `Debug`, so `unwrap_err` cannot be asked).
    fn bundle_err(path: &Path) -> anyhow::Error {
        match read_bundle(path) {
            Ok(_) => panic!("{} read as a bundle", path.display()),
            Err(error) => error,
        }
    }

    /// A `journal.toml` naming one input of `size`/`sha256` at `entry`.
    fn journal_with_input(entry: &str, size: usize, sha256: &str) -> String {
        format!(
            "version = \"v0.0.1 pre-alpha (debug)\"\nos = \"windows\"\n\
             [[step]]\nseconds = 0.0\ntool = \"save-editor\"\nwhat = \"opened x\"\n\
             [[input]]\ntool = \"save-editor\"\npath = \"a/EDIT00000000\"\n\
             size = {size}\nsha256 = \"{sha256}\"\nentry = \"{entry}\"\n"
        )
    }

    // A bundle written by `write_bundle` reads back: the version, the
    // op-less step and the one carrying its op table, the input and the
    // output, all equal to what was recorded.
    #[test]
    fn a_written_bundle_reads_back() {
        let dir = scratch("read_bundle");
        let recorder = Recorder::default();
        recorder.step("save-editor", "opened ~/EDIT00000000 (PES 2021, 10 bytes)");
        let op: toml::Table = toml::from_str("kind = \"player\"\nid = 70101\n").unwrap();
        recorder.step_op("save-editor", "player 70101: Height 183 → 184", op.clone());
        recorder.keep_input(
            "save-editor",
            Path::new("a/EDIT00000000"),
            Arc::from(&b"SAVE_BYTES"[..]),
        );
        recorder.keep_output(
            "save-editor",
            Path::new("out/EDIT00000000"),
            Arc::from(&b"WRITTEN"[..]),
        );
        let out = dir.join("repro.zip");
        recorder.write_bundle(&out, "", &[], &[], None).unwrap();

        let bundle = read_bundle(&out).unwrap();

        assert_eq!(bundle.version, crate::shell::version_label());
        assert_eq!(bundle.steps.len(), 2);
        assert_eq!(bundle.steps[0].tool, "save-editor");
        assert_eq!(
            bundle.steps[0].what,
            "opened ~/EDIT00000000 (PES 2021, 10 bytes)"
        );
        assert!(bundle.steps[0].op.is_none(), "an op-less step reads none");
        assert_eq!(bundle.steps[1].op, Some(op), "the op table as recorded");
        assert_eq!(bundle.inputs.len(), 1);
        assert_eq!(bundle.inputs[0].path, "a/EDIT00000000");
        assert_eq!(bundle.inputs[0].bytes, b"SAVE_BYTES");
        assert_eq!(bundle.outputs.len(), 1);
        assert_eq!(bundle.outputs[0].path, "out/EDIT00000000");
        assert_eq!(bundle.outputs[0].bytes, b"WRITTEN");
    }

    // One byte of an input's entry changed is refused naming the entry:
    // the row's SHA-256 is of the bytes the journal saw.
    #[test]
    fn a_changed_input_byte_is_refused_naming_the_entry() {
        let dir = scratch("read_changed");
        let path = dir.join("repro.zip");
        zip_of(
            &path,
            &[
                (
                    "journal.toml",
                    journal_with_input("inputs/0-EDIT00000000", 4, &sha256(b"SAVE")).as_bytes(),
                ),
                ("inputs/0-EDIT00000000", &b"SAVX"[..]),
            ],
        );

        let error = bundle_err(&path);
        assert!(
            format!("{error:#}").contains("inputs/0-EDIT00000000"),
            "the refusal names the entry: {error:#}"
        );
        assert!(
            format!("{error:#}").contains("SHA-256"),
            "the refusal says why: {error:#}"
        );
        assert!(
            format!("{error:#}").contains(path.to_string_lossy().as_ref()),
            "the refusal names the bundle: {error:#}"
        );
    }

    // A file that is not a zip is refused as not a bundle, naming the path.
    #[test]
    fn a_file_that_is_not_a_zip_is_refused() {
        let dir = scratch("read_not_zip");
        let path = dir.join("repro.zip");
        fs::write(&path, b"not a zip at all").unwrap();

        let error = bundle_err(&path);
        assert!(format!("{error:#}").contains("not a zip"), "{error:#}");
        assert!(
            format!("{error:#}").contains(path.to_string_lossy().as_ref()),
            "names the path: {error:#}"
        );
    }
}
