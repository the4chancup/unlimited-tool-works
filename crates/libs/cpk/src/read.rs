//! Reading a CPK archive: the `CPK ` header table, the `TOC ` entry table, and
//! the optional `ETOC` timestamp table.

use std::io::{Read, Seek, SeekFrom};

use crate::CpkError;
use crate::crilayla;
use crate::utf::{UtfTable, UtfValue};

/// One file stored in the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpkEntry {
    /// `DirName/FileName` with backslashes normalized to `/`. An empty
    /// `DirName` yields a leading `/` (e.g. `/placeholder`), matching the
    /// legacy reader.
    pub path: String,
    /// Uncompressed size (`ExtractSize`).
    pub size: u32,
    /// Stored size (`FileSize`); smaller than `size` when CRILAYLA-packed.
    pub packed_size: u32,
    /// Absolute offset in the archive: `FileOffset` plus the CRI base, which
    /// is `min(ContentOffset, TocOffset)` — 0x800 in every PES file on hand.
    pub offset: u64,
    /// The ETOC timestamp, when the archive carries one for this entry.
    pub modified: Option<CpkTimestamp>,
}

/// A file modification time from an `ETOC` `UpdateDateTime` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpkTimestamp {
    /// Year, e.g. 2026.
    pub year: u16,
    /// Month, 1-12.
    pub month: u8,
    /// Day of month.
    pub day: u8,
    /// Hour.
    pub hour: u8,
    /// Minute.
    pub minute: u8,
    /// Second.
    pub second: u8,
}

impl CpkTimestamp {
    /// Unpacks the `UpdateDateTime` bitfield
    /// (`year<<48 | month<<40 | day<<32 | hour<<24 | minute<<16 | second<<8`).
    pub(crate) fn from_packed(value: u64) -> Self {
        CpkTimestamp {
            year: (value >> 48) as u16,
            month: (value >> 40) as u8,
            day: (value >> 32) as u8,
            hour: (value >> 24) as u8,
            minute: (value >> 16) as u8,
            second: (value >> 8) as u8,
        }
    }

    /// Packs the timestamp back into `UpdateDateTime` form.
    pub(crate) fn to_packed(self) -> u64 {
        (u64::from(self.year) << 48)
            | (u64::from(self.month) << 40)
            | (u64::from(self.day) << 32)
            | (u64::from(self.hour) << 24)
            | (u64::from(self.minute) << 16)
            | (u64::from(self.second) << 8)
    }
}

/// An open CPK archive. Reads lazily through the reader it wraps.
#[derive(Debug)]
pub struct CpkArchive<R: Read + Seek> {
    reader: R,
    file_len: u64,
    entries: Vec<CpkEntry>,
}

impl<R: Read + Seek> CpkArchive<R> {
    /// Reads the `CPK ` header, `TOC `, and `ETOC` (when present) tables and
    /// builds the entry list. Does not load any file content. An archive
    /// that lists its files by number only (an `ITOC` table, no `TOC `)
    /// opens with no entries: it carries no file names, so there is nothing
    /// to look up by path.
    pub fn open(mut reader: R) -> Result<Self, CpkError> {
        let file_len = reader.seek(SeekFrom::End(0))?;
        let header = read_table(&mut reader, file_len, 0, b"CPK ")?;

        let content_offset = cell_u64(&header, 0, "ContentOffset")?;
        let toc_offset = match as_u64(header.get(0, "TocOffset")) {
            Some(offset) => offset,
            // The `ITOC` table is not read: nothing in the suite looks files up by
            // number, and nothing says which number is which file. A tool that extracts
            // or repacks such archives would read it.
            None if as_u64(header.get(0, "ItocOffset")).is_some() => {
                return Ok(CpkArchive {
                    reader,
                    file_len,
                    entries: Vec::new(),
                });
            }
            None => return Err(CpkError::MissingColumn("TocOffset")),
        };
        // The real content base libcpk uses is the earlier of the two header
        // offsets; every PES archive on hand gives 0x800.
        let base = content_offset.min(toc_offset);

        let toc = read_table(&mut reader, file_len, toc_offset, b"TOC ")?;

        let etoc = match as_u64(header.get(0, "EtocOffset")) {
            Some(offset) => {
                let table = read_table(&mut reader, file_len, offset, b"ETOC")?;
                if table.columns.iter().any(|c| c.name == "UpdateDateTime") {
                    Some(table)
                } else {
                    None
                }
            }
            None => None,
        };

        let mut entries = Vec::with_capacity(toc.rows.len());
        for row in 0..toc.rows.len() {
            let dir = cell_string(&toc, row, "DirName")?;
            let file = cell_string(&toc, row, "FileName")?;
            let path = format!(
                "{}/{}",
                dir.replace('\\', "/").trim_end_matches('/'),
                file.replace('\\', "/").trim_start_matches('/')
            );
            let size = cell_u32(&toc, row, "ExtractSize")?;
            let packed_size = cell_u32(&toc, row, "FileSize")?;
            let offset = cell_u64(&toc, row, "FileOffset")?
                .checked_add(base)
                .ok_or(CpkError::Truncated)?;
            let modified = match toc.get(row, "ID") {
                Some(UtfValue::U32(id)) => etoc_timestamp(etoc.as_ref(), *id as usize),
                Some(UtfValue::U64(id)) => {
                    // An ETOC row index; one past the index range matches nothing.
                    etoc_timestamp(etoc.as_ref(), usize::try_from(*id).unwrap_or(usize::MAX))
                }
                _ => None,
            };
            entries.push(CpkEntry {
                path,
                size,
                packed_size,
                offset,
                modified,
            });
        }

        Ok(CpkArchive {
            reader,
            file_len,
            entries,
        })
    }

    /// The file entries in `TOC ` order.
    pub fn entries(&self) -> &[CpkEntry] {
        &self.entries
    }

    /// Reads one entry's content: CRILAYLA-decompressed when `packed_size`
    /// differs from `size` and the data starts with the CRILAYLA magic, the
    /// raw stored bytes otherwise.
    pub fn read(&mut self, entry: &CpkEntry) -> Result<Vec<u8>, CpkError> {
        let content = self.read_at(entry.offset, entry.packed_size as usize)?;
        if entry.size != entry.packed_size && content.starts_with(b"CRILAYLA") {
            return crilayla::decompress(&content, entry.size as usize);
        }
        Ok(content)
    }

    fn read_at(&mut self, offset: u64, len: usize) -> Result<Vec<u8>, CpkError> {
        if offset.checked_add(len as u64).ok_or(CpkError::Truncated)? > self.file_len {
            return Err(CpkError::Truncated);
        }
        self.reader.seek(SeekFrom::Start(offset))?;
        let mut buffer = vec![0u8; len];
        read_exact(&mut self.reader, &mut buffer)?;
        Ok(buffer)
    }
}

/// `read_exact` that reports end-of-input as [`CpkError::Truncated`].
fn read_exact<R: Read>(reader: &mut R, buf: &mut [u8]) -> Result<(), CpkError> {
    reader.read_exact(buf).map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            CpkError::Truncated
        } else {
            CpkError::Io(e)
        }
    })
}

/// Reads one tagged @UTF table starting at `offset` inside `reader`.
fn read_table<R: Read + Seek>(
    reader: &mut R,
    file_len: u64,
    offset: u64,
    tag: &[u8; 4],
) -> Result<UtfTable, CpkError> {
    reader.seek(SeekFrom::Start(offset))?;
    let mut outer = [0u8; 16];
    read_exact(reader, &mut outer)?;
    let length = u64::from_le_bytes(outer[8..16].try_into().unwrap_or([0; 8]));
    if offset
        .checked_add(16)
        .and_then(|v| v.checked_add(length))
        .ok_or(CpkError::Truncated)?
        > file_len
    {
        return Err(CpkError::Truncated);
    }
    let length = usize::try_from(length).map_err(|_| CpkError::Truncated)?;
    let mut table_bytes = Vec::with_capacity(16 + length);
    table_bytes.extend_from_slice(&outer);
    let mut content = vec![0u8; length];
    read_exact(reader, &mut content)?;
    table_bytes.extend_from_slice(&content);
    UtfTable::read(&table_bytes, tag)
}

fn etoc_timestamp(etoc: Option<&UtfTable>, id: usize) -> Option<CpkTimestamp> {
    match etoc?.get(id, "UpdateDateTime") {
        Some(&UtfValue::U64(v)) => Some(CpkTimestamp::from_packed(v)),
        _ => None,
    }
}

fn cell_string(table: &UtfTable, row: usize, column: &'static str) -> Result<String, CpkError> {
    match table.get(row, column) {
        Some(UtfValue::String(s)) => Ok(s.clone()),
        _ => Err(CpkError::MissingColumn(column)),
    }
}

fn cell_u32(table: &UtfTable, row: usize, column: &'static str) -> Result<u32, CpkError> {
    match table.get(row, column) {
        Some(&UtfValue::U32(v)) => Ok(v),
        Some(&UtfValue::U64(v)) => u32::try_from(v).map_err(|_| CpkError::MissingColumn(column)),
        _ => Err(CpkError::MissingColumn(column)),
    }
}

fn as_u64(value: Option<&UtfValue>) -> Option<u64> {
    match value {
        Some(&UtfValue::U64(v)) => Some(v),
        Some(&UtfValue::U32(v)) => Some(u64::from(v)),
        _ => None,
    }
}

fn cell_u64(table: &UtfTable, row: usize, column: &'static str) -> Result<u64, CpkError> {
    as_u64(table.get(row, column)).ok_or(CpkError::MissingColumn(column))
}
