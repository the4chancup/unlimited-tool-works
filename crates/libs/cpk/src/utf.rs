//! The @UTF column/row table format every CPK structure uses.
//!
//! On disk a table is a 16-byte little-endian outer header (`4s` tag, u32
//! unused, u64 content length) wrapping XOR-encrypted content. The plaintext
//! content is a 32-byte big-endian @UTF header, a column list, the row data,
//! a NUL-terminated string pool, and an 8-aligned binary data pool. All
//! offsets in the @UTF header are relative to `content[8]` (the body).

use crate::CpkError;

/// A decoded @UTF table: its pool name, the column list, and one
/// `Vec<UtfValue>` per row (`rows[i][j]` is column `j`). Cells of
/// `Null`-storage columns read as [`UtfValue::Null`].
#[derive(Debug, Clone)]
pub(crate) struct UtfTable {
    /// The table's name from the string pool (`CpkHeader`, `CpkTocInfo`, ...).
    pub name: String,
    /// Columns in file order.
    pub columns: Vec<UtfColumn>,
    /// Rows in file order; each row has exactly `columns.len()` cells.
    pub rows: Vec<Vec<UtfValue>>,
}

/// One @UTF column: a name, the datum type, and how values are stored.
#[derive(Debug, Clone)]
pub(crate) struct UtfColumn {
    /// Column name from the string pool.
    pub name: String,
    /// The datum type (low nibble of the flags byte).
    pub kind: UtfKind,
    /// Where the values live (high nibble of the flags byte). On write this is
    /// recomputed from the data: `Null` when the single row's cell is null,
    /// `Variable` otherwise; `Constant` is never emitted.
    pub storage: UtfStorage,
}

/// @UTF datum type (low nibble of the column flags byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UtfKind {
    /// `>B`, 1 byte.
    U8,
    /// `>H`, 2 bytes.
    U16,
    /// `>I`, 4 bytes.
    U32,
    /// `>Q`, 8 bytes.
    U64,
    /// `>f`, 4 bytes.
    F32,
    /// `>I` offset into the string pool, 4 bytes.
    String,
    /// `>II` offset+length into the data pool, 8 bytes.
    Bytes,
}

/// @UTF column storage (high nibble of the column flags byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UtfStorage {
    /// No value stored; every cell reads as [`UtfValue::Null`].
    Null,
    /// The value follows the column definition inline and applies to all rows.
    Constant,
    /// The value is stored in each row.
    Variable,
}

/// One cell of an @UTF table.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UtfValue {
    /// A `Null`-storage column cell (no stored value).
    Null,
    /// `U8` cell.
    U8(u8),
    /// `U16` cell.
    U16(u16),
    /// `U32` cell.
    U32(u32),
    /// `U64` cell.
    U64(u64),
    /// `F32` cell.
    F32(f32),
    /// `String` cell.
    String(String),
    /// `Bytes` cell.
    Bytes(Vec<u8>),
}

impl UtfKind {
    fn from_nibble(nibble: u8) -> Result<UtfKind, CpkError> {
        match nibble {
            0 => Ok(UtfKind::U8),
            2 => Ok(UtfKind::U16),
            4 => Ok(UtfKind::U32),
            6 => Ok(UtfKind::U64),
            8 => Ok(UtfKind::F32),
            10 => Ok(UtfKind::String),
            11 => Ok(UtfKind::Bytes),
            other => Err(CpkError::UnsupportedUtfType(other)),
        }
    }

    fn nibble(self) -> u8 {
        match self {
            UtfKind::U8 => 0,
            UtfKind::U16 => 2,
            UtfKind::U32 => 4,
            UtfKind::U64 => 6,
            UtfKind::F32 => 8,
            UtfKind::String => 10,
            UtfKind::Bytes => 11,
        }
    }

    /// Bytes the value occupies inside a row or inline in the column list.
    fn datum_size(self) -> usize {
        match self {
            UtfKind::U8 => 1,
            UtfKind::U16 => 2,
            UtfKind::U32 | UtfKind::F32 | UtfKind::String => 4,
            UtfKind::U64 | UtfKind::Bytes => 8,
        }
    }
}

impl UtfStorage {
    fn from_nibble(nibble: u8) -> Result<UtfStorage, CpkError> {
        match nibble {
            1 => Ok(UtfStorage::Null),
            3 => Ok(UtfStorage::Constant),
            5 => Ok(UtfStorage::Variable),
            other => Err(CpkError::UnsupportedUtfStorage(other)),
        }
    }

    fn nibble(self) -> u8 {
        match self {
            UtfStorage::Null => 1,
            UtfStorage::Constant => 3,
            UtfStorage::Variable => 5,
        }
    }
}

/// The CRI table XOR keystream, applied symmetrically to encrypt and decrypt.
fn crypt(block: &mut [u8]) {
    let mut m: u32 = 0x5f;
    let t: u32 = 0x15;
    for byte in block.iter_mut() {
        *byte ^= (m & 0xff) as u8;
        m = m.wrapping_mul(t) & 0xff;
    }
}

/// Big-endian big-int reads over a slice with a moving cursor.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], pos: usize) -> Reader<'a> {
        Reader { bytes, pos }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], CpkError> {
        let end = self.pos.checked_add(len).ok_or(CpkError::Truncated)?;
        let slice = self.bytes.get(self.pos..end).ok_or(CpkError::Truncated)?;
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, CpkError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, CpkError> {
        Ok(u16::from_be_bytes(
            self.take(2)?.try_into().unwrap_or([0; 2]),
        ))
    }

    fn u32(&mut self) -> Result<u32, CpkError> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().unwrap_or([0; 4]),
        ))
    }

    fn u64(&mut self) -> Result<u64, CpkError> {
        Ok(u64::from_be_bytes(
            self.take(8)?.try_into().unwrap_or([0; 8]),
        ))
    }

    fn f32(&mut self) -> Result<f32, CpkError> {
        Ok(f32::from_be_bytes(
            self.take(4)?.try_into().unwrap_or([0; 4]),
        ))
    }
}

impl UtfTable {
    /// The cell at `row` in the column named `column`, if both exist.
    pub(crate) fn get(&self, row: usize, column: &str) -> Option<&UtfValue> {
        let index = self.columns.iter().position(|c| c.name == column)?;
        self.rows.get(row)?.get(index)
    }

    /// Reads one tagged @UTF table (`CPK `, `TOC `, `ETOC`) out of `bytes`,
    /// which must start at the table's outer header.
    pub(crate) fn read(bytes: &[u8], expected_tag: &[u8; 4]) -> Result<UtfTable, CpkError> {
        let outer = bytes.get(..16).ok_or(CpkError::Truncated)?;
        if outer[..4] != *expected_tag {
            return Err(CpkError::UnexpectedTag {
                expected: String::from_utf8_lossy(expected_tag).into_owned(),
                found: String::from_utf8_lossy(&outer[..4]).into_owned(),
            });
        }
        let length = usize::try_from(u64::from_le_bytes(
            outer[8..16].try_into().unwrap_or([0; 8]),
        ))
        .map_err(|_| CpkError::Truncated)?;
        let end = 16usize.checked_add(length).ok_or(CpkError::Truncated)?;
        let content = bytes.get(16..end).ok_or(CpkError::Truncated)?;
        let mut decrypted;
        let content = if content.starts_with(b"@UTF") {
            content
        } else {
            decrypted = content.to_vec();
            crypt(&mut decrypted);
            &decrypted
        };
        Self::read_content(content)
    }

    fn read_content(content: &[u8]) -> Result<UtfTable, CpkError> {
        let mut header = Reader::new(content, 0);
        if header.take(4)? != b"@UTF" {
            return Err(CpkError::BadMagic);
        }
        let body_length = header.u32()? as usize;
        let rows_offset = header.u32()? as usize;
        let strings_offset = header.u32()? as usize;
        let data_offset = header.u32()? as usize;
        let table_name_offset = header.u32()? as usize;
        let column_count = header.u16()? as usize;
        let row_length = header.u16()? as usize;
        let row_count = header.u32()? as usize;
        if body_length.checked_add(8) != Some(content.len()) {
            return Err(CpkError::LengthMismatch);
        }

        // Offsets are relative to the body (content minus the 8-byte magic and
        // length fields the header occupies at its start).
        let body = &content[8..];
        let strings = body.get(strings_offset..).ok_or(CpkError::Truncated)?;
        let data = body.get(data_offset..).ok_or(CpkError::Truncated)?;
        let rows_area = body.get(rows_offset..).ok_or(CpkError::Truncated)?;

        let read_string = |offset: usize| -> Result<String, CpkError> {
            let rest = strings.get(offset..).ok_or(CpkError::Truncated)?;
            let end = rest.iter().position(|b| *b == 0).unwrap_or(rest.len());
            let bytes = &rest[..end];
            // A name written in a Windows code page (a cup's stadium pack names
            // a file `au_seat cópia.dds` with byte 0xF3) would otherwise refuse
            // the whole archive; Latin-1 maps each byte to its own character,
            // so two such names never fold into one path.
            Ok(match std::str::from_utf8(bytes) {
                Ok(text) => text.to_owned(),
                Err(_) => bytes.iter().map(|byte| char::from(*byte)).collect(),
            })
        };

        let read_value = |reader: &mut Reader<'_>, kind: UtfKind| -> Result<UtfValue, CpkError> {
            Ok(match kind {
                UtfKind::U8 => UtfValue::U8(reader.u8()?),
                UtfKind::U16 => UtfValue::U16(reader.u16()?),
                UtfKind::U32 => UtfValue::U32(reader.u32()?),
                UtfKind::U64 => UtfValue::U64(reader.u64()?),
                UtfKind::F32 => UtfValue::F32(reader.f32()?),
                UtfKind::String => UtfValue::String(read_string(reader.u32()? as usize)?),
                UtfKind::Bytes => {
                    let offset = reader.u32()? as usize;
                    let len = reader.u32()? as usize;
                    let end = offset.checked_add(len).ok_or(CpkError::Truncated)?;
                    let slice = data.get(offset..end).ok_or(CpkError::Truncated)?;
                    UtfValue::Bytes(slice.to_vec())
                }
            })
        };

        let name = read_string(table_name_offset)?;

        // Column definitions follow the 32-byte header; a Constant column's
        // value sits inline right after its 5-byte definition.
        let mut columns = Vec::with_capacity(column_count);
        let mut constants = Vec::with_capacity(column_count);
        let mut column_reader = Reader::new(content, 32);
        for _ in 0..column_count {
            let flags = column_reader.u8()?;
            let name_offset = column_reader.u32()? as usize;
            let kind = UtfKind::from_nibble(flags & 0x0f)?;
            let storage = UtfStorage::from_nibble(flags >> 4)?;
            let constant = match storage {
                UtfStorage::Constant => Some(read_value(&mut column_reader, kind)?),
                UtfStorage::Null | UtfStorage::Variable => None,
            };
            columns.push(UtfColumn {
                name: read_string(name_offset)?,
                kind,
                storage,
            });
            constants.push(constant);
        }

        // A table cannot declare more rows than it has bytes; at a zero
        // row_length this is the only bound the loop has.
        if row_count > body.len() {
            return Err(CpkError::Truncated);
        }
        // The declared row area must fit before any row allocation happens.
        let rows_len = row_count
            .checked_mul(row_length)
            .filter(|total| *total <= rows_area.len())
            .ok_or(CpkError::Truncated)?;
        let rows_area = &rows_area[..rows_len];

        let mut rows = Vec::new();
        for i in 0..row_count {
            let start = i.checked_mul(row_length).ok_or(CpkError::Truncated)?;
            let end = start.checked_add(row_length).ok_or(CpkError::Truncated)?;
            let row_bytes = rows_area.get(start..end).ok_or(CpkError::Truncated)?;
            let mut row_reader = Reader::new(row_bytes, 0);
            let mut row = Vec::with_capacity(column_count);
            for (column, constant) in columns.iter().zip(constants.iter()) {
                row.push(match column.storage {
                    UtfStorage::Null => UtfValue::Null,
                    UtfStorage::Constant => constant.clone().unwrap_or(UtfValue::Null),
                    UtfStorage::Variable => read_value(&mut row_reader, column.kind)?,
                });
            }
            rows.push(row);
        }

        Ok(UtfTable {
            name,
            columns,
            rows,
        })
    }

    /// Serializes the table under `tag`: columns in order, `Null` storage
    /// only when the table has
    /// exactly one row and that cell is null (`Constant` is never written),
    /// strings deduplicated in first-use order with the table name first, data
    /// pool entries 8-padded, the string pool 8-padded before the data pool,
    /// body-relative header offsets, and the whole plaintext encrypted.
    pub(crate) fn write(&self, tag: &[u8; 4]) -> Vec<u8> {
        let mut column_bytes = Vec::new();
        let mut row_bytes = Vec::new();
        let mut string_pool = StringPool::new();
        let mut data_pool: Vec<u8> = Vec::new();

        let table_name_id = string_pool.add(&self.name) as u32;

        // Storage is decided per column from the data: Null only for a
        // single-row table whose one cell is null, Variable otherwise.
        let single_row = self.rows.len() == 1;
        let mut variable = Vec::with_capacity(self.columns.len());
        for (index, column) in self.columns.iter().enumerate() {
            let is_null = single_row
                && matches!(
                    self.rows.first().and_then(|row| row.get(index)),
                    Some(UtfValue::Null) | None
                );
            let storage = if is_null {
                UtfStorage::Null
            } else {
                UtfStorage::Variable
            };
            variable.push(matches!(storage, UtfStorage::Variable));
            column_bytes.push(storage.nibble() << 4 | column.kind.nibble());
            column_bytes.extend_from_slice(&(string_pool.add(&column.name) as u32).to_be_bytes());
        }

        for row in &self.rows {
            for (index, column) in self.columns.iter().enumerate() {
                if !variable[index] {
                    continue;
                }
                write_cell(
                    &mut row_bytes,
                    &mut string_pool,
                    &mut data_pool,
                    column.kind,
                    row.get(index).unwrap_or(&UtfValue::Null),
                );
            }
        }

        let column_offset = 32usize;
        let row_offset = column_offset + column_bytes.len();
        let string_offset = row_offset + row_bytes.len();
        let string_padding_offset = string_offset + string_pool.bytes.len();
        let string_padding = (8 - string_padding_offset % 8) % 8;
        let data_offset = string_padding_offset + string_padding;
        let data_end = data_offset + data_pool.len();

        let mut plaintext = Vec::with_capacity(data_end);
        plaintext.extend_from_slice(b"@UTF");
        plaintext.extend_from_slice(&((data_end - 8) as u32).to_be_bytes());
        plaintext.extend_from_slice(&((row_offset - 8) as u32).to_be_bytes());
        plaintext.extend_from_slice(&((string_offset - 8) as u32).to_be_bytes());
        plaintext.extend_from_slice(&((data_offset - 8) as u32).to_be_bytes());
        plaintext.extend_from_slice(&table_name_id.to_be_bytes());
        plaintext.extend_from_slice(&(self.columns.len() as u16).to_be_bytes());
        let row_length: usize = self
            .columns
            .iter()
            .zip(variable.iter())
            .map(|(c, v)| if *v { c.kind.datum_size() } else { 0 })
            .sum();
        plaintext.extend_from_slice(&(row_length as u16).to_be_bytes());
        plaintext.extend_from_slice(&(self.rows.len() as u32).to_be_bytes());
        plaintext.extend_from_slice(&column_bytes);
        plaintext.extend_from_slice(&row_bytes);
        plaintext.extend_from_slice(&string_pool.bytes);
        plaintext.resize(data_offset, 0);
        plaintext.extend_from_slice(&data_pool);

        let mut output = Vec::with_capacity(16 + plaintext.len());
        output.extend_from_slice(tag);
        output.extend_from_slice(&0u32.to_le_bytes());
        output.extend_from_slice(&(plaintext.len() as u64).to_le_bytes());
        crypt(&mut plaintext);
        output.extend_from_slice(&plaintext);
        output
    }
}

/// The deduplicating string pool: offsets are relative to the pool start.
struct StringPool {
    bytes: Vec<u8>,
    offsets: std::collections::HashMap<String, usize>,
}

impl StringPool {
    fn new() -> StringPool {
        StringPool {
            bytes: Vec::new(),
            offsets: std::collections::HashMap::new(),
        }
    }

    fn add(&mut self, text: &str) -> usize {
        if let Some(offset) = self.offsets.get(text) {
            return *offset;
        }
        let offset = self.bytes.len();
        self.bytes.extend_from_slice(text.as_bytes());
        self.bytes.push(0);
        self.offsets.insert(text.to_owned(), offset);
        offset
    }
}

fn write_cell(
    row: &mut Vec<u8>,
    strings: &mut StringPool,
    data: &mut Vec<u8>,
    kind: UtfKind,
    value: &UtfValue,
) {
    match (kind, value) {
        (UtfKind::U8, UtfValue::U8(v)) => row.push(*v),
        (UtfKind::U16, UtfValue::U16(v)) => row.extend_from_slice(&v.to_be_bytes()),
        (UtfKind::U32, UtfValue::U32(v)) => row.extend_from_slice(&v.to_be_bytes()),
        (UtfKind::U64, UtfValue::U64(v)) => row.extend_from_slice(&v.to_be_bytes()),
        (UtfKind::F32, UtfValue::F32(v)) => row.extend_from_slice(&v.to_be_bytes()),
        (UtfKind::String, UtfValue::String(s)) => {
            row.extend_from_slice(&(strings.add(s) as u32).to_be_bytes());
        }
        (UtfKind::Bytes, UtfValue::Bytes(b)) => {
            let offset = data.len();
            data.extend_from_slice(b);
            data.resize(data.len() + (8 - data.len() % 8) % 8, 0);
            row.extend_from_slice(&(offset as u32).to_be_bytes());
            row.extend_from_slice(&(b.len() as u32).to_be_bytes());
        }
        // A null or type-mismatched cell in a variable column: emit the
        // zero-equivalent so the row keeps its declared length. A written
        // table never carries one.
        (UtfKind::U8, _) => row.push(0),
        (UtfKind::U16, _) => row.extend_from_slice(&0u16.to_be_bytes()),
        (UtfKind::U32, _) => row.extend_from_slice(&0u32.to_be_bytes()),
        (UtfKind::U64, _) => row.extend_from_slice(&0u64.to_be_bytes()),
        (UtfKind::F32, _) => row.extend_from_slice(&0f32.to_be_bytes()),
        (UtfKind::String, _) => {
            row.extend_from_slice(&(strings.add("") as u32).to_be_bytes());
        }
        (UtfKind::Bytes, _) => {
            row.extend_from_slice(&(data.len() as u32).to_be_bytes());
            row.extend_from_slice(&0u32.to_be_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_table() -> UtfTable {
        UtfTable {
            name: "Sample".to_owned(),
            columns: vec![
                UtfColumn {
                    name: "Id".to_owned(),
                    kind: UtfKind::U32,
                    storage: UtfStorage::Variable,
                },
                UtfColumn {
                    name: "Label".to_owned(),
                    kind: UtfKind::String,
                    storage: UtfStorage::Variable,
                },
            ],
            rows: vec![
                vec![UtfValue::U32(7), UtfValue::String("one".to_owned())],
                vec![UtfValue::U32(8), UtfValue::String("two".to_owned())],
            ],
        }
    }

    #[test]
    fn written_table_reads_back() {
        let bytes = sample_table().write(b"TEST");
        let table = UtfTable::read(&bytes, b"TEST").unwrap();
        assert_eq!(table.name, "Sample");
        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.get(0, "Label"), Some(&UtfValue::String("one".into())));
        assert_eq!(table.get(1, "Id"), Some(&UtfValue::U32(8)));
    }

    #[test]
    fn f32_and_bytes_columns_round_trip_and_the_data_pool_is_8_padded() {
        let table = UtfTable {
            name: "Floats".to_owned(),
            columns: vec![
                UtfColumn {
                    name: "F".to_owned(),
                    kind: UtfKind::F32,
                    storage: UtfStorage::Variable,
                },
                UtfColumn {
                    name: "B".to_owned(),
                    kind: UtfKind::Bytes,
                    storage: UtfStorage::Variable,
                },
            ],
            rows: vec![
                vec![UtfValue::F32(1.5), UtfValue::Bytes(vec![1, 2, 3])],
                vec![UtfValue::F32(-2.25), UtfValue::Bytes(vec![4, 5, 6, 7, 8])],
            ],
        };
        let bytes = table.write(b"TEST");
        let back = UtfTable::read(&bytes, b"TEST").unwrap();
        assert_eq!(back.get(0, "F"), Some(&UtfValue::F32(1.5)));
        assert_eq!(back.get(1, "F"), Some(&UtfValue::F32(-2.25)));
        assert_eq!(back.get(0, "B"), Some(&UtfValue::Bytes(vec![1, 2, 3])));
        assert_eq!(
            back.get(1, "B"),
            Some(&UtfValue::Bytes(vec![4, 5, 6, 7, 8]))
        );

        // Each data-pool datum pads to 8 bytes: the two cells must sit at
        // data-pool offsets 0 and 8, and the pool is exactly 16 bytes.
        let mut plain = bytes;
        let length = usize::try_from(u64::from_le_bytes(plain[8..16].try_into().unwrap())).unwrap();
        crypt(&mut plain[16..16 + length]);
        let content = &plain[16..16 + length];
        let body_length = u32::from_be_bytes(content[4..8].try_into().unwrap()) as usize;
        let rows_offset = u32::from_be_bytes(content[8..12].try_into().unwrap()) as usize;
        let data_offset = u32::from_be_bytes(content[16..20].try_into().unwrap()) as usize;
        let row_length = u16::from_be_bytes(content[26..28].try_into().unwrap()) as usize;
        // F32 then Bytes(offset,length) in each row; the data-pool offset is
        // the row's second cell's first field.
        let row0 = &content[8 + rows_offset..8 + rows_offset + row_length];
        let offset0 = u32::from_be_bytes(row0[4..8].try_into().unwrap());
        let row1 = &content[8 + rows_offset + row_length..8 + rows_offset + 2 * row_length];
        let offset1 = u32::from_be_bytes(row1[4..8].try_into().unwrap());
        assert_eq!((offset0, offset1), (0, 8));
        assert_eq!(body_length - data_offset, 16);
    }

    #[test]
    fn a_null_cell_in_a_variable_column_writes_the_zero_equivalent() {
        // A `Null` cell (or one missing off a short row) in a Variable
        // column stores the kind's zero: "" for String, nothing for Bytes.
        let table = UtfTable {
            name: "Sparse".to_owned(),
            columns: vec![
                UtfColumn {
                    name: "Label".to_owned(),
                    kind: UtfKind::String,
                    storage: UtfStorage::Variable,
                },
                UtfColumn {
                    name: "Data".to_owned(),
                    kind: UtfKind::Bytes,
                    storage: UtfStorage::Variable,
                },
            ],
            rows: vec![
                vec![
                    UtfValue::String("one".to_owned()),
                    UtfValue::Bytes(vec![1, 2, 3]),
                ],
                vec![UtfValue::Null],
            ],
        };
        let bytes = table.write(b"TEST");
        let back = UtfTable::read(&bytes, b"TEST").unwrap();
        assert_eq!(back.get(0, "Label"), Some(&UtfValue::String("one".into())));
        assert_eq!(back.get(0, "Data"), Some(&UtfValue::Bytes(vec![1, 2, 3])));
        assert_eq!(back.get(1, "Label"), Some(&UtfValue::String(String::new())));
        assert_eq!(back.get(1, "Data"), Some(&UtfValue::Bytes(Vec::new())));
    }

    #[test]
    fn a_non_utf8_pool_string_reads_as_latin1() {
        let mut bytes = sample_table().write(b"TEST");
        let length = usize::try_from(u64::from_le_bytes(bytes[8..16].try_into().unwrap())).unwrap();
        crypt(&mut bytes[16..16 + length]);
        let content = &mut bytes[16..16 + length];
        // Corrupt the first byte of "one" in the string pool; pool offsets are
        // relative to the body (content[8..]).
        let strings = u32::from_be_bytes(content[12..16].try_into().unwrap()) as usize;
        let at = content[8 + strings..]
            .windows(4)
            .position(|w| w == b"one\0")
            .unwrap();
        content[8 + strings + at] = 0xFF;
        let table = UtfTable::read(&bytes, b"TEST").unwrap();
        assert_eq!(
            table.get(0, "Label"),
            Some(&UtfValue::String("\u{FF}ne".into()))
        );
    }

    #[test]
    fn a_huge_row_count_is_truncated_not_an_allocation() {
        let mut bytes = sample_table().write(b"TEST");
        let length = usize::try_from(u64::from_le_bytes(bytes[8..16].try_into().unwrap())).unwrap();
        crypt(&mut bytes[16..16 + length]);
        // Row count is the last big-endian u32 of the @UTF header.
        bytes[16 + 28..16 + 32].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(matches!(
            UtfTable::read(&bytes, b"TEST"),
            Err(CpkError::Truncated)
        ));
    }

    /// A plaintext-`@UTF` `TEST` table with one Constant `U8` column ("C")
    /// holding 0xAB and `row_count` zero-length rows.
    fn constant_u8_table(row_count: u32) -> Vec<u8> {
        let mut content = Vec::new();
        content.extend_from_slice(b"@UTF");
        content.extend_from_slice(&34u32.to_be_bytes()); // body length
        content.extend_from_slice(&30u32.to_be_bytes()); // rows offset
        content.extend_from_slice(&30u32.to_be_bytes()); // strings offset
        content.extend_from_slice(&34u32.to_be_bytes()); // data offset
        content.extend_from_slice(&0u32.to_be_bytes()); // table name in pool
        content.extend_from_slice(&1u16.to_be_bytes()); // column count
        content.extend_from_slice(&0u16.to_be_bytes()); // row length
        content.extend_from_slice(&row_count.to_be_bytes());
        content.push(0x30); // Constant storage, U8 kind
        content.extend_from_slice(&2u32.to_be_bytes()); // column name in pool
        content.push(0xAB); // the constant value
        content.extend_from_slice(b"T\0C\0");

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"TEST");
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&(content.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&content);
        bytes
    }

    #[test]
    fn a_constant_storage_column_applies_to_every_row() {
        let bytes = constant_u8_table(2);
        let table = UtfTable::read(&bytes, b"TEST").unwrap();
        assert_eq!(table.name, "T");
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.get(0, "C"), Some(&UtfValue::U8(0xAB)));
        assert_eq!(table.get(1, "C"), Some(&UtfValue::U8(0xAB)));
    }

    #[test]
    fn a_huge_row_count_with_zero_row_length_is_truncated() {
        // row_length is 0, so the area check is no bound; a table still
        // cannot declare more rows than it has bytes.
        let bytes = constant_u8_table(u32::MAX);
        assert!(matches!(
            UtfTable::read(&bytes, b"TEST"),
            Err(CpkError::Truncated)
        ));
        // The boundary is allowed: 34 rows in a 34-byte body.
        let bytes = constant_u8_table(34);
        assert_eq!(UtfTable::read(&bytes, b"TEST").unwrap().rows.len(), 34);
    }

    #[test]
    fn plaintext_utf_content_reads_without_decryption() {
        let mut bytes = sample_table().write(b"TEST");
        let length = usize::try_from(u64::from_le_bytes(bytes[8..16].try_into().unwrap())).unwrap();
        crypt(&mut bytes[16..16 + length]);
        assert!(bytes[16..20] == *b"@UTF");
        let table = UtfTable::read(&bytes, b"TEST").unwrap();
        assert_eq!(table.get(1, "Label"), Some(&UtfValue::String("two".into())));
    }

    #[test]
    fn unknown_type_nibble_is_an_error() {
        let mut bytes = sample_table().write(b"TEST");
        let length = usize::try_from(u64::from_le_bytes(bytes[8..16].try_into().unwrap())).unwrap();
        crypt(&mut bytes[16..16 + length]);
        // First column's flags byte sits at plaintext offset 32.
        bytes[16 + 32] = 0x50 | 1;
        assert!(matches!(
            UtfTable::read(&bytes, b"TEST"),
            Err(CpkError::UnsupportedUtfType(1))
        ));
    }

    #[test]
    fn unexpected_tag_is_reported() {
        let bytes = sample_table().write(b"TEST");
        assert!(matches!(
            UtfTable::read(&bytes, b"TOCx"),
            Err(CpkError::UnexpectedTag { .. })
        ));
    }

    #[test]
    fn a_length_past_usize_is_truncated() {
        let mut bytes = sample_table().write(b"TEST");
        bytes[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(matches!(
            UtfTable::read(&bytes, b"TEST"),
            Err(CpkError::Truncated)
        ));
    }
}
