//! The installed `DpFileList.bin`, the list of CPKs the game loads from `download/`, read for
//! the Compiler's working-bin walk (compared there with the official list, `templates`)
//! and the Save editor's portrait source. Its layout is the one
//! measured in `spec/team_compiler/pipeline.md` "DpFileList upgrade" (the sub-bullet starting "The
//! DPFL binary format is small"): a 16-byte header whose
//! second little-endian `u32` is the entry count, then that many 48-byte records, each a CPK file
//! name ended by a NUL, then a tail of zeros. The header's other words, each record's bytes after
//! its name and the tail are ignored: some lists carry a number or a stray byte there.

/// The header's size: a word the reader ignores, the entry count, 8 more bytes.
const HEADER: usize = 16;

/// One record's size.
const RECORD: usize = 48;

/// `read` could not read the bytes as a `DpFileList.bin`.
#[derive(Debug, thiserror::Error)]
pub enum DpFileListError {
    /// The bytes are shorter than the header.
    #[error("the list is {0} bytes, shorter than its {HEADER}-byte header")]
    ShorterThanHeader(usize),
    /// The bytes hold fewer records than the header's count.
    #[error("the list is {len} bytes, too short for the {count} records its header counts")]
    TooShortForRecords {
        /// The bytes' length.
        len: usize,
        /// The header's record count.
        count: u32,
    },
    /// A record's name has no NUL ending it.
    #[error("record {0} has no NUL ending its name")]
    NameWithoutNul(usize),
    /// A record's name is not UTF-8.
    #[error("record {0}'s name is not UTF-8")]
    NameNotUtf8(usize),
    /// A record's name is empty.
    #[error("record {0}'s name is empty")]
    NameEmpty(usize),
}

/// The CPK file names an installed `DpFileList.bin` lists, in load order: a later CPK's files
/// win over an earlier one's.
pub fn read(bytes: &[u8]) -> Result<Vec<String>, DpFileListError> {
    let Some((header, body)) = bytes.split_at_checked(HEADER) else {
        return Err(DpFileListError::ShorterThanHeader(bytes.len()));
    };
    let count = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
    let (records, _) = body.as_chunks::<RECORD>();
    let records = usize::try_from(count)
        .ok()
        .and_then(|count| records.get(..count))
        .ok_or(DpFileListError::TooShortForRecords {
            len: bytes.len(),
            count,
        })?;
    records
        .iter()
        .enumerate()
        .map(|(index, record)| name(index, record))
        .collect()
}

/// The CPK file name the record at `index` holds: its bytes before the first NUL.
fn name(index: usize, record: &[u8; RECORD]) -> Result<String, DpFileListError> {
    let end = record
        .iter()
        .position(|&byte| byte == 0)
        .ok_or(DpFileListError::NameWithoutNul(index))?;
    let name =
        std::str::from_utf8(&record[..end]).map_err(|_| DpFileListError::NameNotUtf8(index))?;
    if name.is_empty() {
        return Err(DpFileListError::NameEmpty(index));
    }
    Ok(name.to_owned())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;

    /// The bytes of `tests/fixtures/dpfl/<name>`.
    fn fixture(name: &str) -> Vec<u8> {
        fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/dpfl")
                .join(name),
        )
        .unwrap()
    }

    #[test]
    fn the_walk_fixture_lists_its_three_cpks_in_order() {
        assert_eq!(
            read(&fixture("walk.bin")).unwrap(),
            ["4cc_08_bins.cpk", "4cc_61_midcup.cpk", "4cc_99_test.cpk"]
        );
    }

    #[test]
    fn a_pes_19_list_s_record_numbers_and_stray_byte_are_ignored() {
        let names = read(&fixture("pes19_official.bin")).unwrap();
        assert_eq!(names.len(), 29);
        assert_eq!(names[0], "4cc_01_db.cpk");
        assert_eq!(names[4], "4cc_08_bins.cpk", "the record with a stray byte");
        assert_eq!(names[28], "4cc_69_midcup.cpk");
    }

    #[test]
    fn a_pes_20_list_s_header_word_of_100_is_ignored() {
        let names = read(&fixture("pes20_header_100.bin")).unwrap();
        assert_eq!(names.len(), 45);
        assert_eq!(names[0], "4cc_01_db.cpk");
        assert_eq!(names[44], "4cc_90_test.cpk");
    }

    #[test]
    fn a_pes_17_list_is_read() {
        let names = read(&fixture("pes17_official.bin")).unwrap();
        assert_eq!(names.len(), 39);
        assert_eq!(names[0], "4cc_01_db.cpk");
        assert_eq!(names[38], "4cc_90_test.cpk");
    }

    #[test]
    fn a_list_shorter_than_its_counted_records_is_an_error() {
        // 1,364 bytes hold the header and 28 records; 40 are counted.
        let mut bytes = fixture("walk.bin");
        bytes[4] = 40;
        let error = read(&bytes).unwrap_err();
        assert_eq!(
            error.to_string(),
            "the list is 1364 bytes, too short for the 40 records its header counts"
        );
    }

    #[test]
    fn a_list_shorter_than_its_header_is_an_error() {
        let error = read(&[0; 15]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "the list is 15 bytes, shorter than its 16-byte header"
        );
    }

    /// A list of one record, `record`.
    fn one_record(record: [u8; RECORD]) -> Vec<u8> {
        let mut bytes = vec![0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        bytes.extend(record);
        bytes
    }

    #[test]
    fn a_record_with_no_nul_is_an_error() {
        let error = read(&one_record([b'a'; RECORD])).unwrap_err();
        assert_eq!(error.to_string(), "record 0 has no NUL ending its name");
    }

    #[test]
    fn a_record_whose_name_is_empty_or_not_utf8_is_an_error() {
        let error = read(&one_record([0; RECORD])).unwrap_err();
        assert_eq!(error.to_string(), "record 0's name is empty");
        let mut record = [0; RECORD];
        record[..2].copy_from_slice(&[0xff, 0xfe]);
        let error = read(&one_record(record)).unwrap_err();
        assert_eq!(error.to_string(), "record 0's name is not UTF-8");
    }

    #[test]
    fn the_bytes_after_a_name_s_nul_are_ignored() {
        let mut record = [0; RECORD];
        record[..6].copy_from_slice(b"a.cpk\0");
        record[6..].fill(0xab);
        assert_eq!(read(&one_record(record)).unwrap(), ["a.cpk"]);
    }
}
