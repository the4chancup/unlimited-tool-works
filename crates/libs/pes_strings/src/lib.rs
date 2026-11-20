//! The games' text file, `all.str`: every menu text, description and
//! loading-screen tip a game shows, one file per language
//! (`eng/string/all.str` in the English archive).
//!
//! The file is a WESYS-wrapped zlib stream ([`wezlib`]), and may also come as
//! the bare inflated payload. The payload is little-endian throughout: a
//! `u32` section count and the `u32` offset of the section table, then one
//! 12-byte record per section (its data's offset, its length, its name's
//! offset). A section holds a `u32` string count, a `u32` group id and one
//! 12-byte record per string (its id, two `u16` lengths, its text's offset from
//! the section's start); names and texts are NUL-terminated, ASCII and UTF-8
//! respectively. The reader follows the offsets, so the two layouts the
//! versions use read alike: they differ only in where the names sit and what
//! pads to what.

/// `read` could not read the bytes as an `all.str`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file's WESYS wrapper is bad.
    #[error("WESYS wrapper: {0}")]
    Wrapper(#[from] wezlib::Error),
    /// The payload is shorter than the 8-byte header.
    #[error("the payload is {0} bytes, shorter than its 8-byte header")]
    ShorterThanHeader(usize),
    /// The payload is too short for the section table the header counts.
    #[error("the payload is {len} bytes, too short for the {count} sections its header counts")]
    TooShortForTable {
        /// The payload's length.
        len: usize,
        /// The header's section count.
        count: u32,
    },
    /// A section's name has no NUL ending it before the payload's end.
    #[error("section {0} has no NUL ending its name")]
    NameWithoutNul(usize),
    /// A section's name is not UTF-8.
    #[error("section {0}'s name is not UTF-8")]
    NameNotUtf8(usize),
    /// A section's header or its string records reach past the payload's end.
    #[error("section {name} needs {end} bytes for its strings; the payload is {len} bytes")]
    SectionTooShort {
        /// The section's name.
        name: String,
        /// The offset the section's bytes would have to reach.
        end: u64,
        /// The payload's length.
        len: usize,
    },
    /// A string's text has no NUL ending it before the payload's end.
    #[error("string {id} in section {name} has no NUL ending its text")]
    TextWithoutNul {
        /// The section's name.
        name: String,
        /// The string's id.
        id: u32,
    },
    /// A string's text is not UTF-8.
    #[error("string {id}'s text in section {name} is not UTF-8")]
    TextNotUtf8 {
        /// The section's name.
        name: String,
        /// The string's id.
        id: u32,
    },
}

/// A parsed `all.str`: its sections in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrFile {
    /// The file's sections, in the order the section table lists them.
    pub sections: Vec<Section>,
}

impl StrFile {
    /// The section named `name`, `None` when the file holds none.
    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|section| section.name == name)
    }
}

/// A named group of numbered strings: `Tip1C` holds the loading-screen tips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The section's name, an ASCII tag like `F8W`.
    pub name: String,
    /// The section's group id, a number of its own per section (`Tip1C` 126,
    /// `Tip2C` 127).
    pub group: u32,
    /// The section's strings, in file order.
    pub strings: Vec<Entry>,
}

impl Section {
    /// The text of the string numbered `id`, `None` when the section holds
    /// none.
    pub fn text(&self, id: u32) -> Option<&str> {
        self.strings
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.text.as_str())
    }
}

/// One string in a section: its id and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The string's number inside its section.
    pub id: u32,
    /// The text up to its NUL, with the game's inline codes as stored.
    pub text: String,
}

/// Reads `bytes` as an `all.str`, WESYS-wrapped or the bare payload.
pub fn read(bytes: &[u8]) -> Result<StrFile, Error> {
    let payload = wezlib::decompress_if_wrapped(bytes)?;
    let header = payload
        .get(..8)
        .ok_or(Error::ShorterThanHeader(payload.len()))?;
    let section_count = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let table_offset = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
    // Table records and offsets are u32s; the arithmetic stays in u64 so a
    // payload just under 4 GiB cannot overflow it.
    let table_end = u64::from(table_offset) + 12 * u64::from(section_count);
    if table_end > payload.len() as u64 {
        return Err(Error::TooShortForTable {
            len: payload.len(),
            count: section_count,
        });
    }
    let table = &payload[table_offset as usize..table_end as usize];
    let mut sections = Vec::with_capacity(section_count as usize);
    for (index, record) in table.as_chunks::<12>().0.iter().enumerate() {
        let data_offset = u32::from_le_bytes([record[0], record[1], record[2], record[3]]);
        // The record's middle u32, the section's length, is not read.
        let name_offset = u32::from_le_bytes([record[8], record[9], record[10], record[11]]);
        let name = name(&payload, index, name_offset as usize)?;
        sections.push(section(&payload, name, data_offset as usize)?);
    }
    Ok(StrFile { sections })
}

/// The NUL-terminated name at `offset`; `index` goes only into the error.
fn name(payload: &[u8], index: usize, offset: usize) -> Result<String, Error> {
    let rest = payload.get(offset..).ok_or(Error::NameWithoutNul(index))?;
    let end = rest
        .iter()
        .position(|&byte| byte == 0)
        .ok_or(Error::NameWithoutNul(index))?;
    std::str::from_utf8(&rest[..end])
        .map(str::to_owned)
        .map_err(|_| Error::NameNotUtf8(index))
}

/// The section at `offset`: a `count`/`group` header, `count` 12-byte
/// records, each pointing at a text offset from the section's start.
fn section(payload: &[u8], name: String, offset: usize) -> Result<Section, Error> {
    let base = offset as u64;
    let too_short = |end: u64| Error::SectionTooShort {
        name: name.clone(),
        end,
        len: payload.len(),
    };
    if base + 8 > payload.len() as u64 {
        return Err(too_short(base + 8));
    }
    let count = u64::from(u32::from_le_bytes([
        payload[offset],
        payload[offset + 1],
        payload[offset + 2],
        payload[offset + 3],
    ]));
    let group = u32::from_le_bytes([
        payload[offset + 4],
        payload[offset + 5],
        payload[offset + 6],
        payload[offset + 7],
    ]);
    let records_end = base + 8 + 12 * count;
    if records_end > payload.len() as u64 {
        return Err(too_short(records_end));
    }
    let records = &payload[offset + 8..records_end as usize];
    let mut strings = Vec::with_capacity(count as usize);
    for record in records.as_chunks::<12>().0 {
        let id = u32::from_le_bytes([record[0], record[1], record[2], record[3]]);
        // The two u16 lengths are not read: files the game accepts carry a
        // wrong second one, and each text runs to its NUL anyway.
        let text_offset = base
            + u64::from(u32::from_le_bytes([
                record[8], record[9], record[10], record[11],
            ]));
        strings.push(Entry {
            id,
            text: text(payload, &name, id, text_offset)?,
        });
    }
    Ok(Section {
        name,
        group,
        strings,
    })
}

/// The NUL-terminated UTF-8 text at `offset`; `name` and `id` go only into
/// the error.
fn text(payload: &[u8], name: &str, id: u32, offset: u64) -> Result<String, Error> {
    let Some(rest) = usize::try_from(offset)
        .ok()
        .and_then(|offset| payload.get(offset..))
    else {
        return Err(Error::TextWithoutNul {
            name: name.to_owned(),
            id,
        });
    };
    let Some(end) = rest.iter().position(|&byte| byte == 0) else {
        return Err(Error::TextWithoutNul {
            name: name.to_owned(),
            id,
        });
    };
    std::str::from_utf8(&rest[..end])
        .map(str::to_owned)
        .map_err(|_| Error::TextNotUtf8 {
            name: name.to_owned(),
            id,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PES14: &[u8] = include_bytes!("../tests/fixtures/pes14_all.str");
    const PES17: &[u8] = include_bytes!("../tests/fixtures/pes17_all.str");

    #[test]
    fn the_pes_2014_fixture_reads() {
        let file = read(PES14).unwrap();
        assert_eq!(file.sections.len(), 185);
        assert_eq!(file.sections[0].name, "Cmt1T");
        assert_eq!(file.sections[184].name, "Xbl8W");
        let tip1c = file.section("Tip1C").unwrap();
        assert_eq!(tip1c.group, 126);
        assert_eq!(tip1c.strings.len(), 336);
        let f8w = file.section("F8W").unwrap();
        assert_eq!(f8w.group, 61);
        assert_eq!(f8w.strings.len(), 70);
        assert_eq!(f8w.text(84), Some("Long Ball"));
        assert_eq!(
            file.section("F8T").unwrap().text(46),
            Some(
                "Play a tight backline that can win back possession and play long balls to launch counter attacks."
            )
        );
        assert_eq!(file.section("DefW").unwrap().text(65), Some("Français"));
        assert_eq!(
            file.section("E10W").unwrap().text(62),
            Some("\u{11}jh Rotate")
        );
    }

    #[test]
    fn the_pes_2017_fixture_reads() {
        let file = read(PES17).unwrap();
        assert_eq!(file.sections.len(), 202);
        assert_eq!(file.sections[0].name, "All1C");
        assert_eq!(file.sections[201].name, "Xbl8W");
        let tip1c = file.section("Tip1C").unwrap();
        assert_eq!(tip1c.group, 126);
        assert_eq!(tip1c.strings.len(), 343);
        let f8w = file.section("F8W").unwrap();
        assert_eq!(f8w.group, 61);
        assert_eq!(f8w.strings.len(), 64);
        assert_eq!(f8w.text(84), Some("Long Ball"));
        assert_eq!(
            file.section("F8T").unwrap().text(46),
            Some(
                "Play a tight backline that can win back possession and play long balls to launch counter attacks."
            )
        );
        assert_eq!(
            file.section("Cmt3W").unwrap().text(245),
            Some("Florianópolis")
        );
        assert_eq!(
            file.section("All1C").unwrap().text(327),
            Some("\u{11}jc Log in to \u{11}ri Server")
        );
    }

    #[test]
    fn wrapped_and_bare_payloads_read_the_same() {
        for fixture in [PES14, PES17] {
            let payload = wezlib::decompress(fixture).unwrap();
            assert_eq!(read(fixture).unwrap(), read(&payload).unwrap());
        }
    }

    #[test]
    fn an_absent_section_or_string_is_none() {
        let file = read(PES17).unwrap();
        assert!(file.section("Nope").is_none());
        assert!(file.section("F8W").unwrap().text(u32::MAX).is_none());
    }

    /// A minimal one-section payload: `name` (group 7) holding `texts`, each
    /// `(id, bytes up to its NUL)`. Layout: header, table, name, section.
    fn one_section(name: &[u8], texts: &[(u32, &[u8])]) -> Vec<u8> {
        let name_offset = 20usize;
        let data_offset = name_offset + name.len() + 1;
        let texts_offset = 8 + 12 * texts.len();
        let mut section = Vec::new();
        section.extend_from_slice(&(texts.len() as u32).to_le_bytes());
        section.extend_from_slice(&7u32.to_le_bytes());
        let mut texts_bytes = Vec::new();
        for &(id, text) in texts {
            section.extend_from_slice(&id.to_le_bytes());
            section.extend_from_slice(&((text.len() + 1) as u16).to_le_bytes());
            section.extend_from_slice(&(text.len() as u16).to_le_bytes());
            section.extend_from_slice(&((texts_offset + texts_bytes.len()) as u32).to_le_bytes());
            texts_bytes.extend_from_slice(text);
            texts_bytes.push(0);
        }
        section.extend_from_slice(&texts_bytes);
        let mut payload = Vec::new();
        payload.extend_from_slice(&1u32.to_le_bytes());
        payload.extend_from_slice(&8u32.to_le_bytes());
        payload.extend_from_slice(&(data_offset as u32).to_le_bytes());
        payload.extend_from_slice(&(section.len() as u32).to_le_bytes());
        payload.extend_from_slice(&(name_offset as u32).to_le_bytes());
        payload.extend_from_slice(name);
        payload.push(0);
        payload.extend_from_slice(&section);
        payload
    }

    #[test]
    fn a_built_payload_reads_back() {
        let file = read(&one_section(b"Alpha", &[(5, b"hi"), (9, b"there")])).unwrap();
        let section = file.section("Alpha").unwrap();
        assert_eq!(section.group, 7);
        assert_eq!(section.strings.len(), 2);
        assert_eq!(section.text(5), Some("hi"));
        assert_eq!(section.text(9), Some("there"));
    }

    #[test]
    fn a_header_or_table_past_the_end_is_an_error() {
        assert!(matches!(read(&[0; 7]), Err(Error::ShorterThanHeader(7))));
        // The header counts one section; the table record is cut at 16.
        assert!(matches!(
            read(&one_section(b"Alpha", &[])[..16]),
            Err(Error::TooShortForTable { .. })
        ));
    }

    #[test]
    fn a_name_past_the_end_or_not_utf8_is_an_error() {
        let mut payload = one_section(b"Alpha", &[(1, b"hi")]);
        // The record's name offset, at 16, pointed past the payload.
        payload[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(read(&payload), Err(Error::NameWithoutNul(0))));
        assert!(matches!(
            read(&one_section(b"\xff", &[(1, b"hi")])),
            Err(Error::NameNotUtf8(0))
        ));
    }

    #[test]
    fn a_section_past_the_end_is_an_error() {
        let mut payload = one_section(b"Alpha", &[(1, b"hi")]);
        // The section's string count, at its start (26), grown to 255.
        payload[26..30].copy_from_slice(&255u32.to_le_bytes());
        assert!(matches!(read(&payload), Err(Error::SectionTooShort { .. })));
    }

    #[test]
    fn a_text_offset_past_the_end_is_an_error() {
        let mut payload = one_section(b"Alpha", &[(1, b"hi")]);
        // The entry's text offset, at 26 + 8 + 8, pointed past the payload.
        payload[42..46].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(read(&payload), Err(Error::TextWithoutNul { .. })));
        // The text's own NUL removed.
        let mut payload = one_section(b"Alpha", &[(1, b"hi")]);
        payload.pop();
        assert!(matches!(read(&payload), Err(Error::TextWithoutNul { .. })));
    }

    #[test]
    fn a_text_that_is_not_utf8_is_an_error() {
        assert!(matches!(
            read(&one_section(b"Alpha", &[(9, &[0xff])])),
            Err(Error::TextNotUtf8 { .. })
        ));
    }

    #[test]
    fn a_bad_wrapper_is_the_wrapper_error() {
        // WESYS header, then bytes that are no zlib stream.
        let mut bytes = b"\x00\x10\x01WESYS\x00\x00\x00\x00\x00\x00\x00\x00".to_vec();
        bytes.extend_from_slice(&[1, 2, 3]);
        assert!(matches!(read(&bytes), Err(Error::Wrapper(_))));
    }
}
