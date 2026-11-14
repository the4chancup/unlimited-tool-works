//! Writing a CPK archive in the layout PES loads:
//! files stored back to back at 0x800 alignment, then `TOC `, then `ETOC`
//! (only when every entry carries a timestamp), then the `CPK ` header table
//! written over the leading padding at offset 0.

use std::io::{Seek, SeekFrom, Write};

use crate::CpkError;
use crate::read::CpkTimestamp;
use crate::utf::{UtfColumn, UtfKind, UtfStorage, UtfTable, UtfValue};

const ALIGNMENT: u64 = 0x800;

/// One file queued for the archive.
#[derive(Debug)]
struct PendingEntry {
    path: String,
    size: u32,
    offset: u64,
    modified: Option<CpkTimestamp>,
}

/// Builds a CPK image in the wrapped writer. Alignment is fixed at 0x800.
#[derive(Debug)]
pub struct CpkWriter<W: Write + Seek> {
    writer: W,
    position: u64,
    files: Vec<PendingEntry>,
    paths: std::collections::HashSet<String>,
    tool_version: String,
}

impl<W: Write + Seek> CpkWriter<W> {
    /// Starts a new archive: 0x7FA zero bytes plus `(c)CRI`, so the first file
    /// lands at 0x800. `tool_version` becomes the header's `Tvers` string;
    /// passing the string an existing archive carries reproduces its bytes.
    pub fn new(mut writer: W, tool_version: &str) -> Result<Self, CpkError> {
        writer.write_all(&[0u8; 0x7FA])?;
        writer.write_all(b"(c)CRI")?;
        Ok(CpkWriter {
            writer,
            position: ALIGNMENT,
            files: Vec::new(),
            paths: std::collections::HashSet::new(),
            tool_version: tool_version.to_owned(),
        })
    }

    /// Stores `content` under `path`, padded to the 0x800 alignment. The same
    /// path twice is [`CpkError::DuplicatePath`]; the archive never replaces.
    pub fn add(
        &mut self,
        path: &str,
        content: &[u8],
        modified: Option<CpkTimestamp>,
    ) -> Result<(), CpkError> {
        if path.contains('\0') {
            return Err(CpkError::InvalidPath(path.to_owned()));
        }
        if !self.paths.insert(path.to_owned()) {
            return Err(CpkError::DuplicatePath(path.to_owned()));
        }
        let len = content.len() as u64;
        self.files.push(PendingEntry {
            path: path.to_owned(),
            size: content.len() as u32,
            offset: self.position,
            modified,
        });
        self.writer.write_all(content)?;
        let padding = padding(len);
        self.writer.write_all(&vec![0u8; padding as usize])?;
        self.position += len + padding;
        Ok(())
    }

    /// The length in bytes of the file [`finish`](Self::finish) would write if
    /// an entry were first added for each `(path, content length)` of `more`, in
    /// that order, without a modification time (as the Compiler adds its
    /// entries). `more` empty gives the length of the file finished now. The
    /// Compiler asks this before it puts a whole team into a teams part, to
    /// keep the part under its size cap, table of contents included.
    ///
    /// The answer is exact: the tables are laid out by the same code `finish`
    /// writes them with. It depends on more than the lengths: on the distinct
    /// folder and file names (the TOC stores each once) and on whether every
    /// entry carries a modification time (only then is the `ETOC` written, so
    /// an entry without one drops it). The paths are taken to be ones
    /// [`add`](Self::add) accepts. Nothing is written and the writer is
    /// unchanged.
    pub fn len_with<'a>(&self, more: impl IntoIterator<Item = (&'a str, u64)>) -> u64 {
        let mut position = self.position;
        let mut added = Vec::new();
        for (path, len) in more {
            added.push(PendingEntry {
                path: path.to_owned(),
                // Truncated as `add` truncates a content length.
                size: len as u32,
                offset: position,
                modified: None,
            });
            position += len + padding(len);
        }
        let entries = self.files.iter().chain(&added).collect();
        tables(entries, position).end()
    }

    /// Writes the `TOC ` table, the `ETOC` when every entry has a timestamp,
    /// then the `CPK ` header table at offset 0, and returns the writer.
    pub fn finish(mut self) -> Result<W, CpkError> {
        let tables = tables(self.files.iter().collect(), self.position);
        self.writer.write_all(&tables.toc)?;
        if let Some((offset, bytes)) = &tables.etoc {
            let toc_end = tables.toc_offset + tables.toc.len() as u64;
            self.writer
                .write_all(&vec![0u8; (offset - toc_end) as usize])?;
            self.writer.write_all(bytes)?;
        }

        let header = self.header_table(
            tables.toc_offset,
            tables.toc.len() as u64,
            tables.etoc.as_ref().map(|(offset, _)| *offset),
            tables.etoc.as_ref().map(|(_, bytes)| bytes.len() as u64),
            tables.content_size,
        );
        let header_bytes = header.write(b"CPK ");
        if header_bytes.len() as u64 > ALIGNMENT {
            return Err(CpkError::HeaderTooLarge);
        }
        self.writer.seek(SeekFrom::Start(0))?;
        self.writer.write_all(&header_bytes)?;
        Ok(self.writer)
    }

    /// The 44-column `CPK ` header row, in the fixed order the layout uses.
    fn header_table(
        &self,
        toc_offset: u64,
        toc_size: u64,
        etoc_offset: Option<u64>,
        etoc_size: Option<u64>,
        total_size: u64,
    ) -> UtfTable {
        let mut header = UtfTable {
            name: "CpkHeader".to_owned(),
            columns: Vec::new(),
            rows: vec![Vec::new()],
        };
        let mut add = |name: &'static str, kind: UtfKind, value: UtfValue| {
            header.columns.push(UtfColumn {
                name: name.to_owned(),
                kind,
                storage: UtfStorage::Variable,
            });
            header.rows[0].push(value);
        };
        let null = || UtfValue::Null;
        add("UpdateDateTime", UtfKind::U64, UtfValue::U64(1));
        add("FileSize", UtfKind::U64, null());
        add("ContentOffset", UtfKind::U64, UtfValue::U64(ALIGNMENT));
        add(
            "ContentSize",
            UtfKind::U64,
            UtfValue::U64(toc_offset - ALIGNMENT),
        );
        add("TocOffset", UtfKind::U64, UtfValue::U64(toc_offset));
        add("TocSize", UtfKind::U64, UtfValue::U64(toc_size));
        add("TocCrc", UtfKind::U32, null());
        add("HtocOffset", UtfKind::U64, null());
        add("HtocSize", UtfKind::U64, null());
        add(
            "EtocOffset",
            UtfKind::U64,
            etoc_offset.map_or_else(null, UtfValue::U64),
        );
        add(
            "EtocSize",
            UtfKind::U64,
            etoc_size.map_or_else(null, UtfValue::U64),
        );
        add("ItocOffset", UtfKind::U64, null());
        add("ItocSize", UtfKind::U64, null());
        add("ItocCrc", UtfKind::U32, null());
        add("GtocOffset", UtfKind::U64, null());
        add("GtocSize", UtfKind::U64, null());
        add("GtocCrc", UtfKind::U32, null());
        add("HgtocOffset", UtfKind::U64, null());
        add("HgtocSize", UtfKind::U64, null());
        add("EnabledPackedSize", UtfKind::U64, UtfValue::U64(total_size));
        add("EnabledDataSize", UtfKind::U64, UtfValue::U64(total_size));
        add("TotalDataSize", UtfKind::U64, null());
        add("Tocs", UtfKind::U32, null());
        add(
            "Files",
            UtfKind::U32,
            UtfValue::U32(self.files.len() as u32),
        );
        add("Groups", UtfKind::U32, UtfValue::U32(0));
        add("Attrs", UtfKind::U32, UtfValue::U32(0));
        add("TotalFiles", UtfKind::U32, null());
        add("Directories", UtfKind::U32, null());
        add("Updates", UtfKind::U32, null());
        add("Version", UtfKind::U16, UtfValue::U16(7));
        add("Revision", UtfKind::U16, UtfValue::U16(14));
        add("Align", UtfKind::U16, UtfValue::U16(ALIGNMENT as u16));
        add("Sorted", UtfKind::U16, UtfValue::U16(1));
        add("EnableFileName", UtfKind::U16, UtfValue::U16(1));
        add("EID", UtfKind::U16, null());
        add("CpkMode", UtfKind::U32, UtfValue::U32(1));
        add(
            "Tvers",
            UtfKind::String,
            UtfValue::String(self.tool_version.clone()),
        );
        add("Comment", UtfKind::String, UtfValue::String(String::new()));
        add("Codec", UtfKind::U32, UtfValue::U32(0));
        add("DpkItoc", UtfKind::U32, UtfValue::U32(0));
        add("EnableTocCrc", UtfKind::U16, UtfValue::U16(0));
        add("EnableFileCrc", UtfKind::U16, UtfValue::U16(0));
        add("CrcMode", UtfKind::U32, UtfValue::U32(0));
        add("CrcTable", UtfKind::Bytes, UtfValue::Bytes(Vec::new()));
        header
    }
}

/// The zero bytes that follow `len` bytes to reach the next 0x800 boundary.
fn padding(len: u64) -> u64 {
    (ALIGNMENT - len % ALIGNMENT) % ALIGNMENT
}

/// The tables written after the file contents, serialized, and where each
/// starts in the file.
struct Tables {
    /// Where the `TOC ` starts: the end of the padded contents.
    toc_offset: u64,
    /// The `TOC ` table's bytes.
    toc: Vec<u8>,
    /// The `ETOC`'s offset and bytes, present only when every entry carries a
    /// modification time. It starts at the 0x800 boundary after the `TOC `.
    etoc: Option<(u64, Vec<u8>)>,
    /// The sum of the entries' sizes, without padding.
    content_size: u64,
}

impl Tables {
    /// The file's length: the end of its last table.
    fn end(&self) -> u64 {
        match &self.etoc {
            Some((offset, bytes)) => offset + bytes.len() as u64,
            None => self.toc_offset + self.toc.len() as u64,
        }
    }
}

/// Lays out the `TOC ` and the `ETOC` of `entries`, the `TOC ` at
/// `toc_offset`. `finish` writes these bytes and `len_with` measures them, so
/// the predicted length cannot drift from the written one.
fn tables(mut entries: Vec<&PendingEntry>, toc_offset: u64) -> Tables {
    entries.sort_by_key(|f| f.path.to_uppercase());

    let mut toc = table(
        "CpkTocInfo",
        &[
            ("DirName", UtfKind::String),
            ("FileName", UtfKind::String),
            ("FileSize", UtfKind::U32),
            ("ExtractSize", UtfKind::U32),
            ("FileOffset", UtfKind::U64),
            ("ID", UtfKind::U32),
            ("UserString", UtfKind::String),
        ],
    );
    let mut etoc = table(
        "CpkEtocInfo",
        &[
            ("UpdateDateTime", UtfKind::U64),
            ("LocalDir", UtfKind::String),
        ],
    );

    let mut content_size = 0u64;
    for entry in &entries {
        let (dir, file) = split_path(&entry.path);
        toc.rows.push(vec![
            UtfValue::String(dir.clone()),
            UtfValue::String(file),
            UtfValue::U32(entry.size),
            UtfValue::U32(entry.size),
            UtfValue::U64(entry.offset - ALIGNMENT),
            UtfValue::U32(toc.rows.len() as u32),
            UtfValue::String(String::new()),
        ]);
        if let Some(modified) = entry.modified {
            etoc.rows.push(vec![
                UtfValue::U64(modified.to_packed()),
                UtfValue::String(dir),
            ]);
        }
        content_size += u64::from(entry.size);
    }

    let toc_bytes = toc.write(b"TOC ");
    let etoc = if etoc.rows.len() == toc.rows.len() {
        let toc_size = toc_bytes.len() as u64;
        // The trailing all-empty row the layout ends with.
        etoc.rows
            .push(vec![UtfValue::U64(0), UtfValue::String(String::new())]);
        Some((
            toc_offset + toc_size + padding(toc_size),
            etoc.write(b"ETOC"),
        ))
    } else {
        None
    };
    Tables {
        toc_offset,
        toc: toc_bytes,
        etoc,
        content_size,
    }
}

fn table(name: &str, columns: &[(&'static str, UtfKind)]) -> UtfTable {
    UtfTable {
        name: name.to_owned(),
        columns: columns
            .iter()
            .map(|(name, kind)| UtfColumn {
                name: (*name).to_owned(),
                kind: *kind,
                // Recomputed from the data on write.
                storage: UtfStorage::Variable,
            })
            .collect(),
        rows: Vec::new(),
    }
}

/// Splits `dir/name` at the last `/`; no slash means an empty dir.
fn split_path(path: &str) -> (String, String) {
    match path.rfind('/') {
        Some(pos) => (path[..pos].to_owned(), path[pos + 1..].to_owned()),
        None => (String::new(), path.to_owned()),
    }
}
