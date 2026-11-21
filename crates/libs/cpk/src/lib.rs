//! CRI CPK archive read and write, with CRILAYLA decompression.
//!
//! A CPK file is a `CPK ` header @UTF table (version fields, offsets to the
//! other tables), the file contents, a `TOC ` table of file entries, and
//! optionally an `ETOC` table of modification times. All @UTF content is
//! XOR-encrypted with the CRI keystream. A table string that is not UTF-8
//! (a name written in a Windows code page) reads as Latin-1, byte for byte.
//! The writer lays every archive out one fixed way (see `write.rs`), so the
//! same entries and `Tvers` tool-version string (the one `CpkWriter::new`
//! takes) always give the same bytes.

mod crilayla;
/// The installed `DpFileList.bin`: the CPKs `download/` loads, in order.
pub mod dp_file_list;
mod read;
mod utf;
mod write;

pub use dp_file_list::DpFileListError;
pub use read::{CpkArchive, CpkEntry, CpkTimestamp};
pub use write::CpkWriter;

/// Why a CPK read or write failed.
#[derive(Debug, thiserror::Error)]
pub enum CpkError {
    /// The underlying reader or writer failed.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    /// An outer table header carried a different tag than expected
    /// (`CPK `, `TOC `, `ETOC`).
    #[error("unexpected table tag: expected {expected}, found {found}")]
    UnexpectedTag {
        /// The four-byte tag the code was looking for.
        expected: String,
        /// The tag actually found.
        found: String,
    },
    /// The decrypted content does not start with `@UTF`.
    #[error("missing @UTF magic")]
    BadMagic,
    /// The @UTF body length does not match the outer header's length.
    #[error("@UTF body length does not match the outer header")]
    LengthMismatch,
    /// A column type nibble outside the known set (0,2,4,6,8,10,11).
    #[error("unsupported utf column type nibble {0}")]
    UnsupportedUtfType(u8),
    /// A column storage nibble outside the known set (1,3,5).
    #[error("unsupported utf column storage nibble {0}")]
    UnsupportedUtfStorage(u8),
    /// A required header field or TOC column is absent or holds no usable
    /// value.
    #[error("missing required utf column {0}")]
    MissingColumn(&'static str),
    /// The `CPK ` header table does not fit the 0x800 bytes before the first
    /// file.
    #[error("header table larger than the 0x800-byte reserved region")]
    HeaderTooLarge,
    /// An archive path holds a NUL byte, which the @UTF string pool cannot
    /// carry.
    #[error("path holds a NUL byte: {0:?}")]
    InvalidPath(String),
    /// A CRILAYLA buffer is malformed (bad magic, too short, or the bitstream
    /// ran out before the output was full).
    #[error("crilayla: {0}")]
    Crilayla(&'static str),
    /// The same archive path was added twice.
    #[error("duplicate path in archive: {0}")]
    DuplicatePath(String),
    /// The archive ends before a structure that points past it.
    #[error("archive is truncated")]
    Truncated,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    use crate::utf::{UtfKind, UtfValue};

    const PES17: &[u8] = include_bytes!("../tests/fixtures/konami_pes17_dt42_win.cpk");
    const PES21: &[u8] = include_bytes!("../tests/fixtures/konami_pes21_dt42_all.cpk");
    const RED: &[u8] = include_bytes!("../tests/fixtures/red_pes16_face_73113.cpk");
    const PLACEHOLDER: &[u8] = include_bytes!("../tests/fixtures/cpkmc136_placeholder.cpk");

    fn open(bytes: &[u8]) -> CpkArchive<Cursor<&[u8]>> {
        CpkArchive::open(Cursor::new(bytes)).unwrap()
    }

    fn table(
        tag: &[u8; 4],
        name: &str,
        columns: &[(&str, UtfKind)],
        rows: Vec<Vec<UtfValue>>,
    ) -> Vec<u8> {
        utf::UtfTable {
            name: name.to_owned(),
            columns: columns
                .iter()
                .map(|(name, kind)| utf::UtfColumn {
                    name: (*name).to_owned(),
                    kind: *kind,
                    storage: utf::UtfStorage::Variable,
                })
                .collect(),
            rows,
        }
        .write(tag)
    }

    /// The `CPK ` table at offset 0; `read` accepts a slice longer than the
    /// table.
    fn header_of(bytes: &[u8]) -> utf::UtfTable {
        utf::UtfTable::read(bytes, b"CPK ").unwrap()
    }

    /// Writes each byte run at its offset into a zero-filled buffer sized to
    /// the largest end.
    fn place(parts: &[(u64, &[u8])]) -> Vec<u8> {
        let len = parts
            .iter()
            .map(|(offset, bytes)| *offset as usize + bytes.len())
            .max()
            .unwrap_or(0);
        let mut buffer = vec![0u8; len];
        for (offset, bytes) in parts {
            buffer[*offset as usize..*offset as usize + bytes.len()].copy_from_slice(bytes);
        }
        buffer
    }

    const TOC_COLUMNS: &[(&str, UtfKind)] = &[
        ("DirName", UtfKind::String),
        ("FileName", UtfKind::String),
        ("FileSize", UtfKind::U32),
        ("ExtractSize", UtfKind::U32),
        ("FileOffset", UtfKind::U64),
    ];

    #[test]
    fn konami_pes17_header_and_entries() {
        let mut archive = open(PES17);
        assert_eq!(header_of(PES17).columns.len(), 35);
        assert_eq!(
            header_of(PES17).get(0, "Tvers"),
            Some(&UtfValue::String("CPKMC2.21.10, DLL3.00.00".into()))
        );
        let entries = archive.entries().to_vec();
        assert_eq!(entries.len(), 5);
        let first = &entries[0];
        assert_eq!(first.path, "common/sound/Announce/sa_config.bin");
        assert_eq!(first.size, 11279);
        assert_eq!(first.packed_size, 11279);
        assert_eq!(first.offset, 4096);
        assert_eq!(first.modified, None);
        let content = archive.read(first).unwrap();
        assert_eq!(content.len(), 11279);
        // WESYS-wrapped: this file carries the 00 10 11 prefix variant.
        assert_eq!(&content[3..8], b"WESYS");
    }

    #[test]
    fn konami_pes21_header_and_entries() {
        let archive = open(PES21);
        assert_eq!(header_of(PES21).columns.len(), 44);
        let paths: Vec<&str> = archive.entries().iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths.len(), 5);
        assert_eq!(paths[0], "common/sound/config/Announce/sa_config.bin");
        let offsets: Vec<u64> = archive.entries().iter().map(|e| e.offset).collect();
        assert_eq!(offsets, [4096, 18432, 26624, 34816, 36864]);
    }

    #[test]
    fn red_fixture_entries_carry_etoc_timestamps() {
        let archive = open(RED);
        let expected = CpkTimestamp {
            year: 2026,
            month: 2,
            day: 8,
            hour: 23,
            minute: 52,
            second: 11,
        };
        assert_eq!(archive.entries().len(), 3);
        for entry in archive.entries() {
            assert_eq!(entry.modified, Some(expected));
            assert!(
                entry
                    .path
                    .starts_with("common/character0/model/character/face/real/73113/")
            );
        }
        let sizes: Vec<u32> = archive.entries().iter().map(|e| e.size).collect();
        assert_eq!(sizes, [29, 1448, 320]);
    }

    #[test]
    fn placeholder_entry_keeps_leading_slash() {
        let archive = open(PLACEHOLDER);
        assert_eq!(header_of(PLACEHOLDER).columns.len(), 24);
        assert_eq!(archive.entries().len(), 1);
        assert_eq!(archive.entries()[0].path, "/placeholder");
        assert_eq!(archive.entries()[0].size, 11);
    }

    #[test]
    fn writer_output_is_byte_identical_to_pes_file_tools() {
        let mut archive = open(RED);
        let entries = archive.entries().to_vec();
        // Add in on-disk (offset) order: that is the order the original writer
        // appended the content, and byte parity needs the same layout.
        let mut by_offset = entries.clone();
        by_offset.sort_by_key(|e| e.offset);
        // The Tvers string the fixture's writer recorded; parity needs the same one.
        let mut writer = CpkWriter::new(Cursor::new(Vec::new()), "pes-file-tools").unwrap();
        for entry in &by_offset {
            let content = archive.read(entry).unwrap();
            writer.add(&entry.path, &content, entry.modified).unwrap();
        }
        let produced = writer.finish().unwrap().into_inner();
        assert_eq!(produced.len(), RED.len());
        assert_eq!(produced, RED);
    }

    #[test]
    fn written_archive_round_trips() {
        let stamp = CpkTimestamp {
            year: 2026,
            month: 11,
            day: 2,
            hour: 20,
            minute: 15,
            second: 30,
        };
        let big = vec![0xABu8; 5000];
        let mut writer = CpkWriter::new(Cursor::new(Vec::new()), "studio-test").unwrap();
        writer.add("top.txt", b"hello", Some(stamp)).unwrap();
        writer.add("a/b/deep.bin", &[1, 2, 3], Some(stamp)).unwrap();
        writer.add("big.bin", &big, Some(stamp)).unwrap();
        assert!(matches!(
            writer.add("top.txt", b"again", Some(stamp)),
            Err(CpkError::DuplicatePath(_))
        ));
        let bytes = writer.finish().unwrap().into_inner();

        let mut archive = CpkArchive::open(Cursor::new(&bytes[..])).unwrap();
        let entries = archive.entries().to_vec();
        assert_eq!(entries.len(), 3);
        for entry in &entries {
            assert_eq!(entry.modified, Some(stamp));
        }
        assert_eq!(archive.read(&entries[0]).unwrap(), [1, 2, 3]);
        assert_eq!(archive.read(&entries[1]).unwrap(), big);
        assert_eq!(archive.read(&entries[2]).unwrap(), b"hello");
        let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        // Top-level files get an empty DirName, which reads back with a
        // leading slash, same as the placeholder fixture.
        assert_eq!(paths, ["a/b/deep.bin", "/big.bin", "/top.txt"]);
    }

    #[test]
    fn a_tool_version_that_overflows_the_header_region_is_an_error() {
        let mut writer = CpkWriter::new(Cursor::new(Vec::new()), &"x".repeat(3000)).unwrap();
        writer.add("a.bin", b"x", None).unwrap();
        assert!(matches!(writer.finish(), Err(CpkError::HeaderTooLarge)));
    }

    #[test]
    fn a_header_exactly_one_block_still_fits() {
        let mut writer = CpkWriter::new(Cursor::new(Vec::new()), "").unwrap();
        writer.add("a.bin", b"x", None).unwrap();
        let produced = writer.finish().unwrap().into_inner();
        let base_len = 16 + u64::from_le_bytes(produced[8..16].try_into().unwrap()) as usize;

        // The header plaintext always ends 8-aligned (the string pool is
        // 8-padded and the data pool is empty), so a Tvers longer by a
        // multiple of 8 grows the header by exactly that.
        let pad = 0x800 - base_len;
        assert_eq!(pad % 8, 0);
        let mut writer = CpkWriter::new(Cursor::new(Vec::new()), &"x".repeat(pad)).unwrap();
        writer.add("a.bin", b"x", None).unwrap();
        let produced = writer.finish().unwrap().into_inner();
        let content_len = u64::from_le_bytes(produced[8..16].try_into().unwrap());
        assert_eq!(16 + content_len as usize, 0x800);

        // The boundary header fills the reserved region without touching the
        // first file.
        let mut archive = CpkArchive::open(Cursor::new(&produced[..])).unwrap();
        let entries = archive.entries().to_vec();
        assert_eq!(archive.read(&entries[0]).unwrap(), b"x");
    }

    #[test]
    fn a_path_with_a_nul_byte_is_rejected() {
        let mut writer = CpkWriter::new(Cursor::new(Vec::new()), "studio-test").unwrap();
        assert!(matches!(
            writer.add("a.bin\0x", b"x", None),
            Err(CpkError::InvalidPath(_))
        ));
        // Nothing was recorded: the same base name still goes in.
        writer.add("a.bin", b"x", None).unwrap();
    }

    #[test]
    fn a_u64_id_column_resolves_the_etoc_timestamp() {
        let stamp = CpkTimestamp {
            year: 2020,
            month: 5,
            day: 4,
            hour: 3,
            minute: 2,
            second: 1,
        };
        // Some archives store the TOC's ETOC row id as u64 rather than u32.
        let toc = table(
            b"TOC ",
            "CpkTocInfo",
            &[
                ("DirName", UtfKind::String),
                ("FileName", UtfKind::String),
                ("FileSize", UtfKind::U32),
                ("ExtractSize", UtfKind::U32),
                ("FileOffset", UtfKind::U64),
                ("ID", UtfKind::U64),
            ],
            vec![vec![
                UtfValue::String(String::new()),
                UtfValue::String("a.bin".into()),
                UtfValue::U32(4),
                UtfValue::U32(4),
                UtfValue::U64(0),
                UtfValue::U64(0),
            ]],
        );
        let etoc = table(
            b"ETOC",
            "CpkEtocInfo",
            &[("UpdateDateTime", UtfKind::U64)],
            vec![vec![UtfValue::U64(stamp.to_packed())]],
        );
        let toc_offset = 0x400u64;
        let etoc_offset = toc_offset + toc.len() as u64;
        let header = table(
            b"CPK ",
            "CpkHeader",
            &[
                ("ContentOffset", UtfKind::U64),
                ("TocOffset", UtfKind::U64),
                ("EtocOffset", UtfKind::U64),
            ],
            vec![vec![
                UtfValue::U64(0x800),
                UtfValue::U64(toc_offset),
                UtfValue::U64(etoc_offset),
            ]],
        );
        let bytes = place(&[(0, &header), (toc_offset, &toc), (etoc_offset, &etoc)]);

        let archive = CpkArchive::open(Cursor::new(&bytes[..])).unwrap();
        assert_eq!(archive.entries().len(), 1);
        assert_eq!(archive.entries()[0].modified, Some(stamp));
    }

    #[test]
    fn a_u32_etoc_offset_resolves_the_timestamp() {
        let stamp = CpkTimestamp {
            year: 2020,
            month: 5,
            day: 4,
            hour: 3,
            minute: 2,
            second: 1,
        };
        let toc = table(
            b"TOC ",
            "CpkTocInfo",
            &[
                ("DirName", UtfKind::String),
                ("FileName", UtfKind::String),
                ("FileSize", UtfKind::U32),
                ("ExtractSize", UtfKind::U32),
                ("FileOffset", UtfKind::U64),
                ("ID", UtfKind::U32),
            ],
            vec![vec![
                UtfValue::String(String::new()),
                UtfValue::String("a.bin".into()),
                UtfValue::U32(4),
                UtfValue::U32(4),
                UtfValue::U64(0),
                UtfValue::U32(0),
            ]],
        );
        let etoc = table(
            b"ETOC",
            "CpkEtocInfo",
            &[("UpdateDateTime", UtfKind::U64)],
            vec![vec![UtfValue::U64(stamp.to_packed())]],
        );
        let toc_offset = 0x400u64;
        let etoc_offset = toc_offset + toc.len() as u64;
        let header = table(
            b"CPK ",
            "CpkHeader",
            &[
                ("ContentOffset", UtfKind::U64),
                ("TocOffset", UtfKind::U64),
                ("EtocOffset", UtfKind::U32),
            ],
            vec![vec![
                UtfValue::U64(0x800),
                UtfValue::U64(toc_offset),
                UtfValue::U32(etoc_offset as u32),
            ]],
        );
        let bytes = place(&[(0, &header), (toc_offset, &toc), (etoc_offset, &etoc)]);

        let archive = CpkArchive::open(Cursor::new(&bytes[..])).unwrap();
        assert_eq!(archive.entries()[0].modified, Some(stamp));
    }

    #[test]
    fn crilayla_entry_decompresses_and_a_same_size_one_is_raw() {
        let packed: &[u8] = include_bytes!("../tests/fixtures/crilayla/settings_json.crilayla");
        let plain: &[u8] = include_bytes!("../tests/fixtures/crilayla/settings_json.bin");
        // Carries the CRILAYLA magic but is stored uncompressed.
        let mut raw = [0u8; 32];
        raw[..8].copy_from_slice(b"CRILAYLA");

        let header = table(
            b"CPK ",
            "CpkHeader",
            &[("ContentOffset", UtfKind::U64), ("TocOffset", UtfKind::U64)],
            vec![vec![UtfValue::U64(0x1000), UtfValue::U64(0x800)]],
        );
        let toc = table(
            b"TOC ",
            "CpkTocInfo",
            TOC_COLUMNS,
            vec![
                vec![
                    UtfValue::String(String::new()),
                    UtfValue::String("settings.json".into()),
                    UtfValue::U32(packed.len() as u32),
                    UtfValue::U32(plain.len() as u32),
                    UtfValue::U64(0x800),
                ],
                vec![
                    UtfValue::String(String::new()),
                    UtfValue::String("raw.bin".into()),
                    UtfValue::U32(raw.len() as u32),
                    UtfValue::U32(raw.len() as u32),
                    UtfValue::U64(0x1000),
                ],
            ],
        );
        let bytes = place(&[
            (0, &header),
            (0x800, &toc),
            (0x1000, packed),
            (0x1800, &raw),
        ]);

        let mut archive = open(&bytes);
        let entries = archive.entries().to_vec();
        assert_eq!(archive.read(&entries[0]).unwrap(), plain);
        assert_eq!(archive.read(&entries[1]).unwrap(), raw);
    }

    #[test]
    fn an_entry_ending_exactly_at_the_end_reads() {
        let content = b"tail bytes of the archive";
        let header = table(
            b"CPK ",
            "CpkHeader",
            &[("ContentOffset", UtfKind::U64), ("TocOffset", UtfKind::U64)],
            vec![vec![UtfValue::U64(0x1000), UtfValue::U64(0x800)]],
        );
        let toc = table(
            b"TOC ",
            "CpkTocInfo",
            TOC_COLUMNS,
            vec![vec![
                UtfValue::String(String::new()),
                UtfValue::String("tail.bin".into()),
                UtfValue::U32(content.len() as u32),
                UtfValue::U32(content.len() as u32),
                UtfValue::U64(0x800),
            ]],
        );
        let bytes = place(&[(0, &header), (0x800, &toc), (0x1000, content)]);

        let mut archive = open(&bytes);
        let entries = archive.entries().to_vec();
        assert_eq!(archive.read(&entries[0]).unwrap(), content);

        let mut archive = open(&bytes[..bytes.len() - 1]);
        assert!(matches!(
            archive.read(&entries[0]),
            Err(CpkError::Truncated)
        ));
    }

    #[test]
    fn a_table_offset_past_the_end_is_truncated() {
        let header = table(
            b"CPK ",
            "CpkHeader",
            &[("ContentOffset", UtfKind::U64), ("TocOffset", UtfKind::U64)],
            vec![vec![UtfValue::U64(0x800), UtfValue::U64(1 << 20)]],
        );
        let bytes = place(&[(0, &header)]);
        assert!(matches!(
            CpkArchive::open(Cursor::new(&bytes[..])),
            Err(CpkError::Truncated)
        ));
    }

    #[test]
    fn u32_header_offsets_and_u64_toc_sizes_are_accepted() {
        // Real files may carry these cells at other widths: read them by name
        // regardless of width.
        let header = table(
            b"CPK ",
            "CpkHeader",
            &[("ContentOffset", UtfKind::U32), ("TocOffset", UtfKind::U32)],
            vec![vec![UtfValue::U32(0x1000), UtfValue::U32(0x800)]],
        );
        let toc = table(
            b"TOC ",
            "CpkTocInfo",
            &[
                ("DirName", UtfKind::String),
                ("FileName", UtfKind::String),
                ("FileSize", UtfKind::U64),
                ("ExtractSize", UtfKind::U64),
                ("FileOffset", UtfKind::U32),
            ],
            vec![vec![
                UtfValue::String(String::new()),
                UtfValue::String("a.bin".into()),
                UtfValue::U64(4),
                UtfValue::U64(4),
                UtfValue::U32(0x800),
            ]],
        );
        let bytes = place(&[(0, &header), (0x800, &toc), (0x1000, b"data")]);

        let mut archive = open(&bytes);
        let entries = archive.entries().to_vec();
        assert_eq!(entries.len(), 1);
        assert_eq!((entries[0].size, entries[0].packed_size), (4, 4));
        assert_eq!(entries[0].offset, 0x1000);
        assert_eq!(archive.read(&entries[0]).unwrap(), b"data");
    }

    #[test]
    fn a_hostile_file_offset_is_truncated_not_a_panic() {
        let header = table(
            b"CPK ",
            "CpkHeader",
            &[("ContentOffset", UtfKind::U64), ("TocOffset", UtfKind::U64)],
            vec![vec![UtfValue::U64(0x1000), UtfValue::U64(0x800)]],
        );
        let toc = table(
            b"TOC ",
            "CpkTocInfo",
            TOC_COLUMNS,
            vec![vec![
                UtfValue::String(String::new()),
                UtfValue::String("a.bin".into()),
                UtfValue::U32(4),
                UtfValue::U32(4),
                UtfValue::U64(u64::MAX),
            ]],
        );
        let bytes = place(&[(0, &header), (0x800, &toc)]);
        assert!(matches!(
            CpkArchive::open(Cursor::new(&bytes[..])),
            Err(CpkError::Truncated)
        ));
    }

    #[test]
    fn a_toc_without_file_offset_is_missing_column() {
        let header = table(
            b"CPK ",
            "CpkHeader",
            &[("ContentOffset", UtfKind::U64), ("TocOffset", UtfKind::U64)],
            vec![vec![UtfValue::U64(0x1000), UtfValue::U64(0x800)]],
        );
        let toc = table(
            b"TOC ",
            "CpkTocInfo",
            &[
                ("DirName", UtfKind::String),
                ("FileName", UtfKind::String),
                ("FileSize", UtfKind::U32),
                ("ExtractSize", UtfKind::U32),
            ],
            vec![vec![
                UtfValue::String(String::new()),
                UtfValue::String("a.bin".into()),
                UtfValue::U32(4),
                UtfValue::U32(4),
            ]],
        );
        let bytes = place(&[(0, &header), (0x800, &toc)]);
        assert!(matches!(
            CpkArchive::open(Cursor::new(&bytes[..])),
            Err(CpkError::MissingColumn("FileOffset"))
        ));
    }

    #[test]
    fn an_archive_listing_files_by_number_only_opens_with_no_entries() {
        // A numbered archive's header has a TocOffset column holding no
        // value (null storage on disk) and an ItocOffset that has one.
        let header = table(
            b"CPK ",
            "CpkHeader",
            &[
                ("ContentOffset", UtfKind::U64),
                ("TocOffset", UtfKind::U64),
                ("ItocOffset", UtfKind::U64),
            ],
            vec![vec![
                UtfValue::U64(0x1000),
                UtfValue::Null,
                UtfValue::U64(0x800),
            ]],
        );
        let bytes = place(&[(0, &header)]);

        let archive = open(&bytes);
        assert!(archive.entries().is_empty());
    }

    #[test]
    fn a_header_with_neither_table_offset_is_still_refused() {
        let header = table(
            b"CPK ",
            "CpkHeader",
            &[
                ("ContentOffset", UtfKind::U64),
                ("TocOffset", UtfKind::U64),
                ("ItocOffset", UtfKind::U64),
            ],
            vec![vec![UtfValue::U64(0x1000), UtfValue::Null, UtfValue::Null]],
        );
        let bytes = place(&[(0, &header)]);
        assert!(matches!(
            CpkArchive::open(Cursor::new(&bytes[..])),
            Err(CpkError::MissingColumn("TocOffset"))
        ));
    }

    #[test]
    fn a_large_archive_round_trips() {
        let stamp = CpkTimestamp {
            year: 2026,
            month: 1,
            day: 15,
            hour: 8,
            minute: 30,
            second: 0,
        };
        let mut writer = CpkWriter::new(Cursor::new(Vec::new()), "studio-test").unwrap();
        for i in 0..120 {
            writer
                .add(
                    &format!("dir{i}/file{i}.bin"),
                    format!("content-{i}").as_bytes(),
                    Some(stamp),
                )
                .unwrap();
        }
        let bytes = writer.finish().unwrap().into_inner();

        let mut archive = CpkArchive::open(Cursor::new(&bytes[..])).unwrap();
        // The TOC must exceed one 0x800 block for this to exercise the ETOC
        // padding path.
        let toc_size = match header_of(&bytes).get(0, "TocSize") {
            Some(&UtfValue::U64(v)) => v,
            _ => panic!("TocSize missing"),
        };
        assert!(toc_size > 0x800);
        let entries = archive.entries().to_vec();
        assert_eq!(entries.len(), 120);
        for entry in &entries {
            assert_eq!(entry.modified, Some(stamp));
            let i = &entry.path[3..entry.path.find('/').unwrap()];
            assert_eq!(
                archive.read(entry).unwrap(),
                format!("content-{i}").as_bytes()
            );
        }
    }

    #[test]
    fn writer_without_timestamps_emits_no_etoc() {
        let mut writer = CpkWriter::new(Cursor::new(Vec::new()), "studio-test").unwrap();
        writer.add("a.txt", b"x", None).unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        let archive = CpkArchive::open(Cursor::new(&bytes[..])).unwrap();
        assert_eq!(
            header_of(&bytes).get(0, "EtocOffset"),
            Some(&UtfValue::Null)
        );
        assert_eq!(archive.entries()[0].modified, None);
    }

    /// Twenty entries of team 714 (/co/) as the Compiler writes them, with
    /// content lengths from 0 to 5000: short and long paths, one of 48
    /// characters (the boots package), several sharing a folder, and folders
    /// nested five deep and more.
    const TEAM_714: [(&str, usize); 20] = [
        ("common/etc/TeamColor.bin", 0),
        ("Asset/model/character/boots/k0714/#Win/boots.fpk", 5000),
        ("Asset/model/character/boots/k0714/#Win/boots.fpkd", 2048),
        ("Asset/model/character/face/real/71401/#Win/face.fpk", 2049),
        ("Asset/model/character/face/real/71401/#Win/face.fpkd", 320),
        ("Asset/model/character/face/real/71405/#Win/face.fpk", 1448),
        ("Asset/model/character/face/real/71405/#Win/face.fpkd", 29),
        (
            "Asset/model/character/uniform/texture/#windx11/u0714p1.ftex",
            4096,
        ),
        (
            "Asset/model/character/uniform/texture/#windx11/u0714p2.ftex",
            2047,
        ),
        (
            "Asset/model/character/uniform/texture/#windx11/u0714g1.ftex",
            4999,
        ),
        (
            "Asset/model/character/common/714/sourceimages/#windx11/hair_bsm.ftex",
            1,
        ),
        (
            "Asset/model/character/common/714/05 - A/sourceimages/#windx11/face_bsm.ftex",
            3000,
        ),
        ("common/render/symbol/player/71401.dds", 777),
        ("common/render/symbol/player/71405.dds", 4095),
        ("common/render/symbol/flag/e_000714_r_ll.png", 2500),
        ("common/render/symbol/flag/e_000714_r_l.png", 1024),
        ("common/render/symbol/flag/e_000714_r.png", 63),
        (
            "common/character0/model/character/uniform/team/714/714_DEF_GK1st_realUni.bin",
            100,
        ),
        (
            "common/character0/model/character/uniform/team/UniformParameter.bin",
            12,
        ),
        (
            "common/character0/model/character/face/real/71401/face.model",
            17,
        ),
    ];

    /// A writer holding `first`, each entry `modified` and filled with its
    /// length's worth of bytes.
    fn writer_with(
        first: &[(&str, usize)],
        modified: Option<CpkTimestamp>,
    ) -> CpkWriter<Cursor<Vec<u8>>> {
        let mut writer = CpkWriter::new(Cursor::new(Vec::new()), "studio-test").unwrap();
        for (path, len) in first {
            writer.add(path, &vec![0x5A; *len], modified).unwrap();
        }
        writer
    }

    /// `len_with(rest)` on a writer holding `first`, against the length of the
    /// file written by adding `rest` (without timestamps) and finishing.
    fn assert_len_with_is_finished_len(
        first: &[(&str, usize)],
        rest: &[(&str, usize)],
        modified: Option<CpkTimestamp>,
    ) {
        let mut writer = writer_with(first, modified);
        let predicted = writer.len_with(rest.iter().map(|(path, len)| (*path, *len as u64)));
        for (path, len) in rest {
            writer.add(path, &vec![0x5A; *len], None).unwrap();
        }
        let written = writer.finish().unwrap().into_inner().len() as u64;
        assert_eq!(
            predicted,
            written,
            "{} entries then {} more",
            first.len(),
            rest.len()
        );
    }

    #[test]
    fn len_with_is_the_finished_length_of_single_entries() {
        let path = "Asset/model/character/uniform/texture/#windx11/u0714p1.ftex";
        assert_len_with_is_finished_len(&[], &[], None);
        for len in [0, 0x800, 0x801] {
            assert_len_with_is_finished_len(&[], &[(path, len)], None);
            assert_len_with_is_finished_len(&[(path, len)], &[], None);
        }
    }

    #[test]
    fn len_with_is_the_finished_length_of_a_team_split_anywhere() {
        let (first, second) = TEAM_714.split_at(10);
        assert_len_with_is_finished_len(first, second, None);
        assert_len_with_is_finished_len(&[], &TEAM_714, None);
        assert_len_with_is_finished_len(&TEAM_714, &[], None);
    }

    #[test]
    fn len_with_counts_the_etoc_only_while_every_entry_has_a_timestamp() {
        let stamp = CpkTimestamp {
            year: 2026,
            month: 10,
            day: 7,
            hour: 12,
            minute: 0,
            second: 0,
        };
        // All timestamped: the ETOC is written and counted.
        assert_len_with_is_finished_len(&TEAM_714, &[], Some(stamp));
        // The added entries carry none, so neither file has an ETOC.
        let (first, second) = TEAM_714.split_at(10);
        assert_len_with_is_finished_len(first, second, Some(stamp));
    }

    #[test]
    fn len_with_leaves_the_writer_unchanged() {
        let (first, second) = TEAM_714.split_at(10);
        let more = || second.iter().map(|(path, len)| (*path, *len as u64));
        let finished = |mut writer: CpkWriter<Cursor<Vec<u8>>>| {
            for (path, len) in second {
                writer.add(path, &vec![0x5A; *len], None).unwrap();
            }
            writer.finish().unwrap().into_inner()
        };

        let asked = writer_with(first, None);
        let once = asked.len_with(more());
        assert_eq!(asked.len_with(more()), once);
        assert_eq!(finished(asked), finished(writer_with(first, None)));
    }
}
