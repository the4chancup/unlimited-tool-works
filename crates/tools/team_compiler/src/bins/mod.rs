//! The game's color bins as the compiler edits them
//! (`spec/team_compiler/pipeline.md` "Bins accumulation"). Each holds one fixed-size record per
//! team, in team ID order starting at team 100, so team `id`'s record starts at
//! `(id - 100) * record size`, and each record opens with a four-byte header naming its team.
//! A `TeamColor.bin` record is 16 bytes: the `u16` team ID, the `u16` color count (4 in every
//! record), then four colors of three bytes each (R, G, B). A `UniColor.bin` record is 85
//! bytes: the `u32` team ID, the `u8` kit count, then ten 8-byte kit entries (kit number, menu
//! icon number, two colors). The integers are little-endian.

pub(crate) mod installed;
pub(crate) mod kit_configs;
pub(crate) mod shared_ids;

use std::collections::BTreeMap;
use std::ops::Range;

use anyhow::{Context, ensure};
use kit_config::KitSlot;
use pes_version::PesVersion;
use uniparam::UniformParameter;

use crate::templates::Templates;

/// One color as the bins hold it: red, green, blue.
pub(crate) type Rgb = [u8; 3];

/// The colors a kit's `colors.txt` gives: the kit's two menu colors.
pub(crate) const KIT_COLORS: usize = 2;

/// The colors the root `colors.txt` gives: a `TeamColor.bin` record holds four.
pub(crate) const TEAM_COLORS: usize = 4;

/// The team of each bin's first record.
const FIRST_TEAM: u16 = 100;

/// A record's header size in both bins: the team ID, and in `TeamColor.bin` the color count.
const HEADER: usize = 4;

/// A `TeamColor.bin` record's size, 16 bytes: its header, then `TEAM_COLORS` colors.
const TEAM_COLOR_RECORD: usize = HEADER + TEAM_COLORS * 3;

/// The color count every `TeamColor.bin` record's header gives.
const TEAM_COLOR_COUNT: u16 = 4;

/// The kit entries a `UniColor.bin` record holds.
const KIT_ENTRIES: usize = 10;

/// A `UniColor.bin` kit entry's size, 8 bytes: the kit number, the icon number, then
/// `KIT_COLORS` colors.
const KIT_ENTRY: usize = 2 + KIT_COLORS * 3;

/// A `UniColor.bin` record's size, 85 bytes: its header, the kit count, then `KIT_ENTRIES`
/// entries.
const UNI_COLOR_RECORD: usize = HEADER + 1 + KIT_ENTRIES * KIT_ENTRY;

/// The entry a `UniColor.bin` record fills its unused places with: kit number `FF`, the rest
/// zero.
const UNUSED_KIT: [u8; KIT_ENTRY] = [0xff, 0, 0, 0, 0, 0, 0, 0];

/// The records of one color bin, which both bins' editing goes through: the length check, the
/// record of a team and the header loop are the same for both, only the record size and the
/// header's bytes differ.
#[derive(Debug, PartialEq, Eq)]
struct Records {
    /// The bin's file name, as its errors name it.
    name: &'static str,
    /// One record's size.
    size: usize,
    /// The file's bytes, a whole number of records.
    bytes: Vec<u8>,
}

impl Records {
    /// `bytes` as the bin `name` of `size`-byte records; a length that is not a whole number of
    /// records is an error, and so is a bin with more records than a `u16` has team IDs from
    /// 100.
    fn read(name: &'static str, size: usize, bytes: Vec<u8>) -> anyhow::Result<Records> {
        ensure!(
            bytes.len().is_multiple_of(size),
            "{name} is {} bytes, not a whole number of {size}-byte records",
            bytes.len()
        );
        let records = bytes.len() / size;
        ensure!(
            records <= usize::from(u16::MAX - FIRST_TEAM) + 1,
            "{name} holds {records} records, more than there are team IDs"
        );
        Ok(Records { name, size, bytes })
    }

    /// Sets every record's header to `header` of its team (`100 + index`) and returns the IDs
    /// of the teams whose header was not that already, ascending.
    fn repair_headers(&mut self, header: impl Fn(u16) -> [u8; HEADER]) -> Vec<u16> {
        let mut repaired = Vec::new();
        // `read` leaves no bytes past the last whole record.
        for (index, record) in self.bytes.chunks_exact_mut(self.size).enumerate() {
            let team_id = u16::try_from(index)
                .ok()
                .and_then(|index| FIRST_TEAM.checked_add(index))
                .expect("`read` refuses a bin with more records than team IDs");
            let header = header(team_id);
            if record[..HEADER] != header {
                record[..HEADER].copy_from_slice(&header);
                repaired.push(team_id);
            }
        }
        repaired
    }

    /// Where team `team_id`'s record sits in the bytes; a team with no record is an error.
    fn range(&self, team_id: u16) -> anyhow::Result<Range<usize>> {
        team_id
            .checked_sub(FIRST_TEAM)
            .map(|index| usize::from(index) * self.size)
            .map(|start| start..start + self.size)
            .filter(|range| range.end <= self.bytes.len())
            .with_context(|| format!("{} has no record for team {team_id}", self.name))
    }

    /// Team `team_id`'s record; a team with no record is an error.
    fn record(&self, team_id: u16) -> anyhow::Result<&[u8]> {
        let range = self.range(team_id)?;
        Ok(&self.bytes[range])
    }

    /// Team `team_id`'s record, to be edited; a team with no record is an error.
    fn record_mut(&mut self, team_id: u16) -> anyhow::Result<&mut [u8]> {
        let range = self.range(team_id)?;
        Ok(&mut self.bytes[range])
    }
}

/// A `TeamColor.bin` being edited.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TeamColorBin {
    records: Records,
}

impl TeamColorBin {
    /// `bytes` as a bin; a length that is not a whole number of 16-byte records is an error,
    /// and so is a bin with more records than a `u16` has team IDs from 100.
    pub(crate) fn read(bytes: Vec<u8>) -> anyhow::Result<TeamColorBin> {
        Ok(TeamColorBin {
            records: Records::read("TeamColor.bin", TEAM_COLOR_RECORD, bytes)?,
        })
    }

    /// Sets every record's header from its position (team ID `100 + index`, color count 4)
    /// and returns the IDs of the teams whose header was not that already, ascending.
    pub(crate) fn repair_headers(&mut self) -> Vec<u16> {
        self.records.repair_headers(|team_id| {
            let mut header = [0; HEADER];
            header[..2].copy_from_slice(&team_id.to_le_bytes());
            header[2..].copy_from_slice(&TEAM_COLOR_COUNT.to_le_bytes());
            header
        })
    }

    /// Writes `colors` over the first colors of `team_id`'s record; the colors it does not
    /// reach keep their bytes, and colors past the fourth have no slot and are not written. A
    /// team with no record is an error.
    pub(crate) fn set_colors(&mut self, team_id: u16, colors: &[Rgb]) -> anyhow::Result<()> {
        let record = self.records.record_mut(team_id)?;
        let (slots, _) = record[HEADER..].as_chunks_mut::<3>();
        for (slot, color) in slots.iter_mut().zip(colors) {
            *slot = *color;
        }
        Ok(())
    }

    /// The bin's bytes.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.records.bytes
    }
}

/// One kit's entry in its team's `UniColor.bin` record.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct KitColorEntry {
    /// The kit's number: 0 to 8 for `p1` to `p9`, 0x10 for `g1` (`uni_color_kit_number`).
    pub(crate) kit: u8,
    /// The menu icon's number.
    pub(crate) icon: u8,
    /// The kit's two menu colors.
    pub(crate) colors: [Rgb; KIT_COLORS],
}

impl KitColorEntry {
    /// The entry as the record holds it: kit number, icon number, then the colors.
    fn bytes(&self) -> [u8; KIT_ENTRY] {
        let [first, second] = self.colors;
        let mut bytes = [0; KIT_ENTRY];
        bytes[0] = self.kit;
        bytes[1] = self.icon;
        bytes[2..5].copy_from_slice(&first);
        bytes[5..].copy_from_slice(&second);
        bytes
    }
}

/// The game's ten kit slots, which `kit_slot` looks a kit number up in.
const KIT_SLOTS: [KitSlot; 10] = [
    KitSlot::P1,
    KitSlot::P2,
    KitSlot::P3,
    KitSlot::P4,
    KitSlot::P5,
    KitSlot::P6,
    KitSlot::P7,
    KitSlot::P8,
    KitSlot::P9,
    KitSlot::G1,
];

/// The slot of the kit `UniColor.bin` numbers `number` (`uni_color_kit_number`'s inverse);
/// `None` for a number no slot has, such as 0x11, a second goalkeeper kit.
pub(crate) fn kit_slot(number: u8) -> Option<KitSlot> {
    KIT_SLOTS
        .into_iter()
        .find(|slot| uni_color_kit_number(*slot) == number)
}

/// The number `UniColor.bin` gives the kit in `slot`: player kits count from 0, the goalkeeper
/// kit is 0x10.
pub(crate) fn uni_color_kit_number(slot: KitSlot) -> u8 {
    match slot {
        KitSlot::P1 => 0,
        KitSlot::P2 => 1,
        KitSlot::P3 => 2,
        KitSlot::P4 => 3,
        KitSlot::P5 => 4,
        KitSlot::P6 => 5,
        KitSlot::P7 => 6,
        KitSlot::P8 => 7,
        KitSlot::P9 => 8,
        KitSlot::G1 => 0x10,
    }
}

/// A `UniColor.bin` being edited.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct UniColorBin {
    records: Records,
}

impl UniColorBin {
    /// `bytes` as a bin; a length that is not a whole number of 85-byte records is an error,
    /// and so is a bin with more records than a `u16` has team IDs from 100.
    pub(crate) fn read(bytes: Vec<u8>) -> anyhow::Result<UniColorBin> {
        Ok(UniColorBin {
            records: Records::read("UniColor.bin", UNI_COLOR_RECORD, bytes)?,
        })
    }

    /// Sets every record's team ID from its position (`100 + index`) and returns the IDs of
    /// the teams whose ID was not that already, ascending. The kit count is not part of the
    /// header: it varies by record.
    pub(crate) fn repair_headers(&mut self) -> Vec<u16> {
        self.records
            .repair_headers(|team_id| u32::from(team_id).to_le_bytes())
    }

    /// Merges `entry` into `team_id`'s record: the record's counted entries are its kits,
    /// keyed by kit number, and `entry` replaces the one of its number or joins them. The
    /// record is written again with its count, its kits in ascending kit number, then unused
    /// entries; past ten kits the highest-numbered are left out. A record whose counted
    /// entries repeat a kit number holds no kit (the base game's placeholder, ten white
    /// entries numbered 0). A team with no record is an error.
    pub(crate) fn set_kit(&mut self, team_id: u16, entry: &KitColorEntry) -> anyhow::Result<()> {
        self.edit_kits(team_id, |kits| {
            kits.retain(|kit| kit[0] != entry.kit);
            kits.push(entry.bytes());
        })
    }

    /// Keeps, in `team_id`'s record, only the kits whose number is one of `numbers`: a `Full`
    /// export's team holds its export's kits and nothing a past cup left. The record is
    /// written again as `set_kit` writes it; a placeholder record holds no kit, so it keeps
    /// none. A team with no record is an error.
    pub(crate) fn keep_kits(&mut self, team_id: u16, numbers: &[u8]) -> anyhow::Result<()> {
        self.edit_kits(team_id, |kits| {
            kits.retain(|kit| numbers.contains(&kit[0]));
        })
    }

    /// The numbers of the kits `team_id`'s record holds (`held_kits` of its counted entries),
    /// ascending: the kits the game offers the team. A placeholder record holds none. A team
    /// with no record is an error.
    pub(crate) fn kits(&self, team_id: u16) -> anyhow::Result<Vec<u8>> {
        let record = self.records.record(team_id)?;
        let (count, entries) = record[HEADER..]
            .split_first()
            .expect("a record holds its kit count after its header");
        // The rest of the record is exactly `KIT_ENTRIES` entries.
        let (entries, _) = entries.as_chunks::<KIT_ENTRY>();
        let counted = usize::from(*count).min(KIT_ENTRIES);
        let mut numbers: Vec<u8> = held_kits(&entries[..counted])
            .iter()
            .map(|kit| kit[0])
            .collect();
        numbers.sort_unstable();
        Ok(numbers)
    }

    /// Applies `edit` to the kits `team_id`'s record holds (`held_kits` of its counted
    /// entries), then writes the record again: its count, its kits in ascending kit number, then
    /// unused entries; past ten kits the highest-numbered are left out. A team with no record
    /// is an error.
    fn edit_kits(
        &mut self,
        team_id: u16,
        edit: impl FnOnce(&mut Vec<[u8; KIT_ENTRY]>),
    ) -> anyhow::Result<()> {
        let record = self.records.record_mut(team_id)?;
        let (count, entries) = record[HEADER..]
            .split_first_mut()
            .expect("a record holds its kit count after its header");
        // The rest of the record is exactly `KIT_ENTRIES` entries.
        let (entries, _) = entries.as_chunks_mut::<KIT_ENTRY>();
        let counted = usize::from(*count).min(KIT_ENTRIES);
        let mut kits = held_kits(&entries[..counted]);
        edit(&mut kits);
        kits.sort_by_key(|kit| kit[0]);
        kits.truncate(KIT_ENTRIES);
        *count = u8::try_from(kits.len()).expect("a record holds at most ten kits");
        let written = kits.into_iter().chain(std::iter::repeat(UNUSED_KIT));
        for (slot, kit) in entries.iter_mut().zip(written) {
            *slot = kit;
        }
        Ok(())
    }

    /// The bin's bytes.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.records.bytes
    }
}

/// The kits a record's `counted` entries hold: the entries themselves, or none when two of them
/// share a kit number, which is the base game's placeholder for a team with no kit colors.
fn held_kits(counted: &[[u8; KIT_ENTRY]]) -> Vec<[u8; KIT_ENTRY]> {
    let repeats = counted
        .iter()
        .enumerate()
        .any(|(index, kit)| counted[..index].iter().any(|earlier| earlier[0] == kit[0]));
    if repeats {
        Vec::new()
    } else {
        counted.to_vec()
    }
}

/// The bins a run builds on, parsed.
pub(crate) struct WorkingBins {
    /// `TeamColor.bin`.
    pub(crate) team_color: TeamColorBin,
    /// `UniColor.bin`.
    pub(crate) uni_color: UniColorBin,
    /// `UniformParameter.bin`: `Some` on the Fox versions, `None` on PES 15-17, which have no
    /// such bin.
    pub(crate) uniform_parameter: Option<UniformParameter>,
    /// PES 15-17's loose kit configs (`kit_configs::loose_kit_configs`), by entry name
    /// (`714_DEF_1st_realUni.bin`), unwrapped: every `.bin` in a team's folder under
    /// `uniform/team/` of the installed CPKs, the nearest CPK's copy of each. A `Midcup`
    /// export edits those of the kits it does not resend, which are known only once the run is
    /// planned, after the walk; the walk gathers them as it passes, since a second walk would
    /// reopen every CPK for a few 120-byte files. A `Full` export never uses them. Empty on Fox,
    /// whose kit configs are `uniform_parameter`'s entries, and when no walk is made.
    pub(crate) loose_kit_configs: BTreeMap<String, Vec<u8>>,
}

impl WorkingBins {
    /// The bundled bases for `version` (`resources/bins/`, or the run's `templates/` files
    /// replacing them), for a run with no installed bin to build on.
    pub(crate) fn bundled(version: PesVersion, templates: &Templates) -> WorkingBins {
        // An embedded base is tested to parse, and an override was parsed when it was read
        // (`Templates::read`).
        let parses = "a bundled base or a templates/ file read by the run parses";
        WorkingBins {
            team_color: TeamColorBin::read(templates.team_color().to_vec()).expect(parses),
            uni_color: UniColorBin::read(templates.uni_color().to_vec()).expect(parses),
            uniform_parameter: templates
                .uniform_parameter_base(version)
                .map(|bytes| UniformParameter::read(bytes).expect(parses)),
            loose_kit_configs: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two sound records, teams 100 and 101, each with its own four colors.
    fn two_records() -> Vec<u8> {
        let mut bytes = vec![0x64, 0x00, 0x04, 0x00];
        bytes.extend(1..=12);
        bytes.extend([0x65, 0x00, 0x04, 0x00]);
        bytes.extend(21..=32);
        bytes
    }

    #[test]
    fn a_bin_is_a_whole_number_of_records() {
        assert!(TeamColorBin::read(vec![0; 17]).is_err());
        assert!(TeamColorBin::read(vec![0; 32]).is_ok());
    }

    #[test]
    fn a_bin_holds_no_more_records_than_there_are_team_ids_from_100() {
        // Teams 100 to 65535: 65436 records.
        let most = 65_436 * TEAM_COLOR_RECORD;
        assert!(TeamColorBin::read(vec![0; most]).is_ok());
        let error =
            TeamColorBin::read(vec![0; most + TEAM_COLOR_RECORD]).expect_err("one record too many");
        assert_eq!(
            error.to_string(),
            "TeamColor.bin holds 65437 records, more than there are team IDs"
        );
    }

    #[test]
    fn sound_headers_are_left_as_they_are() {
        let mut bin = TeamColorBin::read(two_records()).unwrap();
        assert_eq!(bin.repair_headers(), Vec::<u16>::new());
        assert_eq!(bin.into_bytes(), two_records());
    }

    #[test]
    fn a_header_overwritten_by_colors_is_set_from_its_position() {
        let mut bytes = two_records();
        bytes[16..20].copy_from_slice(&[0xc1, 0x12, 0x00, 0x41]);
        let mut bin = TeamColorBin::read(bytes.clone()).unwrap();
        assert_eq!(bin.repair_headers(), [101]);
        let repaired = bin.into_bytes();
        assert_eq!(repaired[16..20], [0x65, 0x00, 0x04, 0x00]);
        assert_eq!(
            repaired[20..],
            bytes[20..],
            "the record's other bytes are kept"
        );
        assert_eq!(repaired[..16], bytes[..16], "the sound record is unchanged");
    }

    #[test]
    fn a_header_whose_count_is_not_four_is_repaired() {
        let mut bytes = two_records();
        bytes[2] = 3;
        let mut bin = TeamColorBin::read(bytes).unwrap();
        assert_eq!(bin.repair_headers(), [100]);
        assert_eq!(bin.into_bytes(), two_records());
    }

    #[test]
    fn two_colors_are_written_after_the_header_and_the_rest_of_the_record_is_kept() {
        let mut bin = TeamColorBin::read(two_records()).unwrap();
        bin.set_colors(101, &[[0xc1, 0x12, 0x00], [0x41, 0x41, 0x41]])
            .unwrap();
        let mut expected = two_records();
        expected[20..26].copy_from_slice(&[0xc1, 0x12, 0x00, 0x41, 0x41, 0x41]);
        assert_eq!(bin.into_bytes(), expected);
    }

    #[test]
    fn four_colors_fill_the_record() {
        let mut bin = TeamColorBin::read(two_records()).unwrap();
        bin.set_colors(100, &[[0xa1; 3], [0xa2; 3], [0xa3; 3], [0xa4; 3]])
            .unwrap();
        let bytes = bin.into_bytes();
        let mut record = vec![0x64, 0x00, 0x04, 0x00];
        for color in [0xa1, 0xa2, 0xa3, 0xa4] {
            record.extend([color; 3]);
        }
        assert_eq!(bytes[..16], record);
        assert_eq!(bytes[16..], two_records()[16..]);
    }

    #[test]
    fn a_team_with_no_record_is_an_error() {
        let mut bin = TeamColorBin::read(two_records()).unwrap();
        for team_id in [102, 99] {
            let error = bin.set_colors(team_id, &[[1, 2, 3]]).unwrap_err();
            assert_eq!(
                error.to_string(),
                format!("TeamColor.bin has no record for team {team_id}")
            );
        }
        assert_eq!(bin.into_bytes(), two_records());
    }

    #[test]
    fn the_bundled_base_holds_teams_100_to_920_with_sound_headers() {
        let mut bin = WorkingBins::bundled(PesVersion::Pes21, &Templates::embedded()).team_color;
        assert_eq!(bin.repair_headers(), Vec::<u16>::new());
        assert_eq!(bin.into_bytes().len(), 821 * TEAM_COLOR_RECORD);
    }

    /// The bundled `UniColor.bin`'s record of team `team_id`, 85 bytes.
    fn base_record(team_id: usize) -> Vec<u8> {
        let start = (team_id - 100) * UNI_COLOR_RECORD;
        Templates::embedded().uni_color()[start..start + UNI_COLOR_RECORD].to_vec()
    }

    /// A record of team `team_id` (as its four ID bytes) with the kit count `count` and the
    /// `entries`, padded to ten with unused entries.
    fn uni_record(team_id: u16, count: u8, entries: &[[u8; KIT_ENTRY]]) -> Vec<u8> {
        let mut record = u32::from(team_id).to_le_bytes().to_vec();
        record.push(count);
        for index in 0..KIT_ENTRIES {
            record.extend(entries.get(index).unwrap_or(&UNUSED_KIT));
        }
        record
    }

    /// The bin holding the one record `record`, of team 100.
    fn one_record_bin(record: Vec<u8>) -> UniColorBin {
        UniColorBin::read(record).unwrap()
    }

    /// The entry of kit `kit`, its icon and both colors filled with `kit`, so each entry is
    /// told apart by its bytes.
    fn kit(kit: u8) -> [u8; KIT_ENTRY] {
        [kit, 3, kit, kit, kit, kit, kit, kit]
    }

    /// The tracer's goalkeeper kit: icon 11, `#c11200` and `#414141`.
    fn tracer_g1() -> KitColorEntry {
        KitColorEntry {
            kit: 0x10,
            icon: 11,
            colors: [[0xc1, 0x12, 0x00], [0x41, 0x41, 0x41]],
        }
    }

    /// Team 714's record in the bundled base, a past cup's eight kits, as its entries.
    fn team_714_entries() -> Vec<[u8; KIT_ENTRY]> {
        vec![
            [0x00, 0x03, 0xaa, 0x00, 0x00, 0xc6, 0x93, 0x16],
            [0x01, 0x03, 0x08, 0x2d, 0xa2, 0xec, 0x00, 0x00],
            [0x02, 0x03, 0x13, 0x2d, 0x3f, 0x8c, 0x09, 0x08],
            [0x03, 0x03, 0x12, 0x09, 0x0a, 0xf4, 0x3c, 0x6e],
            [0x04, 0x03, 0x8c, 0x8a, 0x60, 0x4a, 0x34, 0x23],
            [0x05, 0x03, 0x00, 0x00, 0x00, 0xe6, 0xb4, 0x00],
            [0x06, 0x03, 0xe6, 0xc3, 0x00, 0x00, 0x00, 0x23],
            [0x10, 0x03, 0x99, 0x00, 0x00, 0x00, 0x00, 0x00],
        ]
    }

    /// The bundled base, read, with team `team_id`'s record after `set_kit` of each of
    /// `entries` in turn.
    fn base_record_after(team_id: u16, entries: &[KitColorEntry]) -> Vec<u8> {
        let mut bin = UniColorBin::read(Templates::embedded().uni_color().to_vec()).unwrap();
        for entry in entries {
            bin.set_kit(team_id, entry).unwrap();
        }
        let start = usize::from(team_id - 100) * UNI_COLOR_RECORD;
        bin.into_bytes()[start..start + UNI_COLOR_RECORD].to_vec()
    }

    #[test]
    fn a_uni_color_bin_is_a_whole_number_of_85_byte_records() {
        let error = UniColorBin::read(vec![0; 86]).expect_err("86 bytes");
        assert_eq!(
            error.to_string(),
            "UniColor.bin is 86 bytes, not a whole number of 85-byte records"
        );
        assert!(UniColorBin::read(vec![0; 170]).is_ok());
    }

    #[test]
    fn a_uni_color_record_whose_id_bytes_are_wrong_gets_them_back_and_keeps_the_rest() {
        let sound = [
            uni_record(100, 2, &[kit(0), kit(1)]),
            uni_record(101, 8, &[kit(5)]),
        ]
        .concat();
        let mut bin = UniColorBin::read(sound.clone()).unwrap();
        assert_eq!(
            bin.repair_headers(),
            Vec::<u16>::new(),
            "a count of 2 or 8 is no header"
        );
        assert_eq!(bin.into_bytes(), sound);

        let mut broken = sound.clone();
        broken[85..89].copy_from_slice(&[0xc1, 0x12, 0x00, 0x41]);
        let mut bin = UniColorBin::read(broken.clone()).unwrap();
        assert_eq!(bin.repair_headers(), [101]);
        let repaired = bin.into_bytes();
        assert_eq!(repaired[85..89], [0x65, 0x00, 0x00, 0x00]);
        assert_eq!(repaired[89..], broken[89..], "the other 81 bytes are kept");
        assert_eq!(repaired[..85], sound[..85], "the sound record is unchanged");
    }

    #[test]
    fn a_kit_set_on_a_placeholder_record_is_its_only_kit_and_a_second_joins_it_in_order() {
        let mut expected = base_record(790);
        assert_eq!(
            expected[..13],
            [
                0x16, 0x03, 0x00, 0x00, 0x02, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff
            ],
            "the base's placeholder"
        );
        let g1 = [0x10, 0x0b, 0xc1, 0x12, 0x00, 0x41, 0x41, 0x41];
        expected = uni_record(790, 1, &[g1]);
        assert_eq!(base_record_after(790, &[tracer_g1()]), expected);

        let p1 = KitColorEntry {
            kit: 0,
            icon: 3,
            colors: [[1, 2, 3], [4, 5, 6]],
        };
        assert_eq!(
            base_record_after(790, &[tracer_g1(), p1]),
            uni_record(790, 2, &[[0, 3, 1, 2, 3, 4, 5, 6], g1])
        );
    }

    #[test]
    fn a_kit_replaces_the_entry_of_its_number_or_joins_the_others_in_order() {
        let entries = team_714_entries();
        assert_eq!(
            base_record(714),
            uni_record(714, 8, &entries),
            "the base's record"
        );

        let p2 = KitColorEntry {
            kit: 1,
            icon: 7,
            colors: [[0x11; 3], [0x22; 3]],
        };
        let mut replaced = entries.clone();
        replaced[1] = [0x01, 0x07, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22];
        assert_eq!(base_record_after(714, &[p2]), uni_record(714, 8, &replaced));

        let p9 = KitColorEntry {
            kit: 8,
            icon: 3,
            colors: [[0x33; 3], [0x44; 3]],
        };
        let mut joined = entries.clone();
        joined.insert(7, [0x08, 0x03, 0x33, 0x33, 0x33, 0x44, 0x44, 0x44]);
        assert_eq!(base_record_after(714, &[p9]), uni_record(714, 9, &joined));
    }

    #[test]
    fn past_ten_kits_the_highest_numbered_are_left_out() {
        let held: Vec<[u8; KIT_ENTRY]> = (0..8).chain([0x10, 0x11]).map(kit).collect();
        let mut bin = one_record_bin(uni_record(100, 10, &held));
        let p9 = KitColorEntry {
            kit: 8,
            icon: 3,
            colors: [[8; 3], [8; 3]],
        };
        bin.set_kit(100, &p9).unwrap();
        let expected: Vec<[u8; KIT_ENTRY]> = (0..9).chain([0x10]).map(kit).collect();
        assert_eq!(bin.into_bytes(), uni_record(100, 10, &expected));
    }

    #[test]
    fn a_count_over_ten_is_read_as_the_record_s_ten_entries() {
        let held: Vec<[u8; KIT_ENTRY]> = (0..10).map(kit).collect();
        let mut bin = one_record_bin(uni_record(100, 0xff, &held));
        let replaced = KitColorEntry {
            kit: 9,
            icon: 1,
            colors: [[0xab; 3], [0xcd; 3]],
        };
        bin.set_kit(100, &replaced).unwrap();
        let mut expected = held;
        expected[9] = [9, 1, 0xab, 0xab, 0xab, 0xcd, 0xcd, 0xcd];
        assert_eq!(bin.into_bytes(), uni_record(100, 10, &expected));
    }

    #[test]
    fn keeping_kits_leaves_only_those_numbers_in_ascending_order() {
        let held = [kit(0), kit(1), kit(2), kit(0x10)];
        let mut bin = one_record_bin(uni_record(100, 4, &held));
        bin.keep_kits(100, &[0x10, 1]).unwrap();
        assert_eq!(
            bin.into_bytes(),
            uni_record(100, 2, &[kit(1), kit(0x10)]),
            "count 2, kit 1 then 0x10, eight unused entries"
        );
    }

    #[test]
    fn keeping_kits_of_a_placeholder_record_leaves_none() {
        assert_eq!(
            base_record(100)[..13],
            [
                0x64, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff
            ],
            "the base's placeholder"
        );
        let mut bin = UniColorBin::read(Templates::embedded().uni_color().to_vec()).unwrap();
        bin.keep_kits(100, &[0, 0x10]).unwrap();
        assert_eq!(
            bin.into_bytes()[..UNI_COLOR_RECORD],
            uni_record(100, 0, &[]),
            "count 0 and ten unused entries"
        );
    }

    #[test]
    fn keeping_kits_of_a_team_with_no_uni_color_record_is_an_error() {
        let mut bin = one_record_bin(uni_record(100, 1, &[kit(0)]));
        let error = bin.keep_kits(101, &[0]).unwrap_err();
        assert_eq!(error.to_string(), "UniColor.bin has no record for team 101");
    }

    #[test]
    fn a_kit_for_a_team_with_no_uni_color_record_is_an_error() {
        let mut bin = UniColorBin::read(Templates::embedded().uni_color().to_vec()).unwrap();
        for team_id in [99, 921] {
            let error = bin.set_kit(team_id, &tracer_g1()).unwrap_err();
            assert_eq!(
                error.to_string(),
                format!("UniColor.bin has no record for team {team_id}")
            );
        }
        assert_eq!(bin.into_bytes(), Templates::embedded().uni_color());
    }

    #[test]
    fn the_bundled_uni_color_base_holds_teams_100_to_920_with_sound_headers() {
        let mut bin = WorkingBins::bundled(PesVersion::Pes21, &Templates::embedded()).uni_color;
        assert_eq!(bin.repair_headers(), Vec::<u16>::new());
        assert_eq!(bin.into_bytes().len(), 821 * UNI_COLOR_RECORD);
    }

    #[test]
    fn every_version_s_embedded_bases_parse() {
        // `bundled` expects them to: a base that did not parse would panic here first.
        for version in PesVersion::ALL {
            let bins = WorkingBins::bundled(version, &Templates::embedded());
            let configs = bins.uniform_parameter.map(|bin| bin.len());
            match version.engine() {
                pes_version::Engine::Fox => {
                    assert!(configs.is_some_and(|len| len > 0), "{version}")
                }
                pes_version::Engine::PreFox => assert_eq!(configs, None, "{version}"),
            }
        }
    }

    #[test]
    fn a_record_s_kits_are_its_held_kit_numbers_ascending() {
        // Out of order, as an installed record may be, and a second goalkeeper kit.
        let held = [kit(0x10), kit(2), kit(0), kit(0x11)];
        let bin = one_record_bin(uni_record(100, 4, &held));
        assert_eq!(bin.kits(100).unwrap(), [0, 2, 0x10, 0x11]);
        // Entries past the count are not held.
        let bin = one_record_bin(uni_record(100, 2, &held));
        assert_eq!(bin.kits(100).unwrap(), [2, 0x10]);
        let error = bin.kits(101).unwrap_err();
        assert_eq!(error.to_string(), "UniColor.bin has no record for team 101");
    }

    #[test]
    fn a_placeholder_record_holds_no_kit_and_team_714_s_base_record_eight() {
        let bin = WorkingBins::bundled(PesVersion::Pes21, &Templates::embedded()).uni_color;
        assert_eq!(bin.kits(100).unwrap(), Vec::<u8>::new(), "the placeholder");
        assert_eq!(bin.kits(714).unwrap(), [0, 1, 2, 3, 4, 5, 6, 0x10]);
    }

    #[test]
    fn each_kit_slot_has_its_uni_color_number() {
        let numbers: Vec<u8> = [
            KitSlot::P1,
            KitSlot::P2,
            KitSlot::P3,
            KitSlot::P4,
            KitSlot::P5,
            KitSlot::P6,
            KitSlot::P7,
            KitSlot::P8,
            KitSlot::P9,
            KitSlot::G1,
        ]
        .into_iter()
        .map(uni_color_kit_number)
        .collect();
        assert_eq!(numbers, [0, 1, 2, 3, 4, 5, 6, 7, 8, 0x10]);
    }

    #[test]
    fn each_kit_number_of_a_slot_gives_the_slot_back_and_another_none() {
        for slot in KIT_SLOTS {
            assert_eq!(kit_slot(uni_color_kit_number(slot)), Some(slot));
        }
        assert_eq!(kit_slot(0), Some(KitSlot::P1));
        assert_eq!(kit_slot(0x10), Some(KitSlot::G1));
        for number in [9, 0x0f, 0x11, 0xff] {
            assert_eq!(kit_slot(number), None, "{number:#x}");
        }
    }
}
