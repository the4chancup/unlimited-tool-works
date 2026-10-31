//! `EditFile`: the whole save, open in memory. The one module that composes
//! `container` (bytes ↔ decrypted sections) and `codec` (sections ↔ fields):
//! it keeps the retained container and every unmodeled payload byte, so a
//! `to_bytes` patches only the modeled records and re-encrypts.

use std::collections::{HashMap, HashSet};

use pes_version::PesVersion;

use crate::codec::{
    CodecError, read_player, read_player_into, read_roster_into, read_tactics_into, read_team,
    write_player, write_roster, write_tactics, write_team,
};
use crate::container::{ContainerError, SaveContainer};
use crate::model::player::PlayerEntry;
use crate::model::team::TeamEntry;
use crate::schema::{SectionLayout, VersionSchema, schema_for};

/// An open save: the retained container plus the decoded players and teams.
#[derive(Clone)]
pub struct EditFile {
    container: SaveContainer,
    schema: &'static VersionSchema,
    players: Vec<PlayerEntry>,
    /// The appearance record index of each player (PES 15/16 only).
    appearance_records: Vec<usize>,
    teams: Vec<TeamEntry>,
}

/// Why a buffer is not a loadable save, or the save cannot be written.
#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    /// The container crypto failed.
    #[error(transparent)]
    Container(#[from] ContainerError),
    /// A record codec failed.
    #[error(transparent)]
    Codec(#[from] CodecError),
    /// File I/O failed (`load`/`save`, native only).
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A section's count or records run past the payload.
    #[error("{section}: {detail}")]
    Layout {
        /// The malformed section.
        section: &'static str,
        /// What is wrong with it.
        detail: String,
    },
    /// A record names an id the owning section lacks.
    #[error("{section}: record id {id} has no matching record")]
    RecordId {
        /// The section the stray record sits in.
        section: &'static str,
        /// The unmatched id.
        id: u32,
    },
}

/// The record count of a section and its bounds-checked record slices.
fn section_records<'a>(
    payload: &'a [u8],
    section: &SectionLayout,
    size: usize,
    name: &'static str,
) -> Result<Vec<&'a [u8]>, SaveError> {
    if section.count_offset + 2 > payload.len() {
        return Err(SaveError::Layout {
            section: name,
            detail: format!(
                "count offset {} is past the {}-byte payload",
                section.count_offset,
                payload.len()
            ),
        });
    }
    let count = u16::from_le_bytes(
        payload[section.count_offset..section.count_offset + 2]
            .try_into()
            .expect("two bytes"),
    ) as usize;
    let end = section
        .offset
        .checked_add(count.saturating_mul(size))
        .ok_or_else(|| SaveError::Layout {
            section: name,
            detail: format!("{count} records of {size} bytes overflow"),
        })?;
    if end > payload.len() {
        return Err(SaveError::Layout {
            section: name,
            detail: format!(
                "{count} records of {size} bytes need {end}, the payload has {}",
                payload.len()
            ),
        });
    }
    Ok((0..count)
        .map(|i| &payload[section.offset + i * size..section.offset + (i + 1) * size])
        .collect())
}

/// The little-endian u32 id at a record's start.
fn record_id(record: &[u8]) -> u32 {
    u32::from_le_bytes(record[..4].try_into().expect("an id field"))
}

/// A roster or tactics section's records, checked against the team section:
/// same count, and the same ids in the same order (`Layout` on a count or
/// order break, `RecordId` on an id the team section lacks).
fn side_section<'a>(
    payload: &'a [u8],
    section: &SectionLayout,
    size: usize,
    name: &'static str,
    teams: &[TeamEntry],
    team_ids: &HashSet<u32>,
) -> Result<Vec<&'a [u8]>, SaveError> {
    let records = section_records(payload, section, size, name)?;
    if records.len() != teams.len() {
        return Err(SaveError::Layout {
            section: name,
            detail: format!(
                "{} records, the team section has {}",
                records.len(),
                teams.len()
            ),
        });
    }
    for (i, rec) in records.iter().enumerate() {
        let id = record_id(rec);
        if id != teams[i].id {
            if team_ids.contains(&id) {
                return Err(SaveError::Layout {
                    section: name,
                    detail: format!(
                        "record {i} is team {id}, the team section's {i} is {}",
                        teams[i].id
                    ),
                });
            }
            return Err(SaveError::RecordId { section: name, id });
        }
    }
    Ok(records)
}

impl EditFile {
    /// Decrypts and decodes; the version is the container's.
    pub fn from_bytes(bytes: &[u8]) -> Result<EditFile, SaveError> {
        let container = SaveContainer::decrypt(bytes)?;
        let schema = schema_for(container.version());
        let payload = &container.payload;

        let player_records =
            section_records(payload, &schema.players, schema.player.size, "players")?;
        let mut players = Vec::with_capacity(player_records.len());
        for rec in &player_records {
            players.push(read_player(rec, schema.player)?);
        }

        let mut appearance_records = Vec::with_capacity(players.len());
        if let Some((section, appearance)) = &schema.appearance {
            let records = section_records(payload, section, appearance.size, "appearance")?;
            let index_of: HashMap<u32, usize> = records
                .iter()
                .enumerate()
                .map(|(i, rec)| (record_id(rec), i))
                .collect();
            for player in players.iter_mut() {
                let index = *index_of.get(&player.id).ok_or(SaveError::RecordId {
                    section: "appearance",
                    id: player.id,
                })?;
                read_player_into(player, records[index], appearance)?;
                appearance_records.push(index);
            }
        }

        let team_records = section_records(payload, &schema.teams, schema.team.size, "teams")?;
        let mut teams = Vec::with_capacity(team_records.len());
        for rec in &team_records {
            teams.push(read_team(rec, schema.team)?);
        }
        let team_ids: HashSet<u32> = teams.iter().map(|t| t.id).collect();

        // The three team sections must list the same ids in the same order.
        let rosters = side_section(
            payload,
            &schema.rosters,
            schema.roster.size,
            "rosters",
            &teams,
            &team_ids,
        )?;
        for (team, rec) in teams.iter_mut().zip(rosters) {
            read_roster_into(team, rec, schema.roster)?;
        }
        let tactics = side_section(
            payload,
            &schema.tactics,
            schema.tactic.size,
            "tactics",
            &teams,
            &team_ids,
        )?;
        for (team, rec) in teams.iter_mut().zip(tactics) {
            read_tactics_into(team, rec, schema.tactic)?;
        }

        Ok(EditFile {
            container,
            schema,
            players,
            appearance_records,
            teams,
        })
    }

    /// `from_bytes` over a file (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(path: &std::path::Path) -> Result<EditFile, SaveError> {
        Self::from_bytes(&std::fs::read(path)?)
    }

    /// The game version this save belongs to.
    pub fn version(&self) -> PesVersion {
        self.container.version()
    }

    /// The container's writer serial: the UTF-16LE text a PES 2016 to 2021
    /// save carries as its writer's serial, kept as read (empty on PES
    /// 2015). A PES 2017 Texport the editor writes carries the open save's.
    pub fn serial(&self) -> &[u8] {
        &self.container.serial
    }

    /// Every player, in record order.
    pub fn players(&self) -> &[PlayerEntry] {
        &self.players
    }

    /// Every player for editing, in record order.
    pub fn players_mut(&mut self) -> &mut [PlayerEntry] {
        &mut self.players
    }

    /// The player with `id`, if the save has one.
    pub fn player(&self, id: u32) -> Option<&PlayerEntry> {
        self.players.iter().find(|p| p.id == id)
    }

    /// The player with `id` for editing, if the save has one.
    pub fn player_mut(&mut self, id: u32) -> Option<&mut PlayerEntry> {
        self.players.iter_mut().find(|p| p.id == id)
    }

    /// Every team (team, roster and tactics joined), in record order.
    pub fn teams(&self) -> &[TeamEntry] {
        &self.teams
    }

    /// Every team for editing, in record order.
    pub fn teams_mut(&mut self) -> &mut [TeamEntry] {
        &mut self.teams
    }

    /// The team with `id`, if the save has one.
    pub fn team(&self, id: u32) -> Option<&TeamEntry> {
        self.teams.iter().find(|t| t.id == id)
    }

    /// The team with `id` for editing, if the save has one.
    pub fn team_mut(&mut self, id: u32) -> Option<&mut TeamEntry> {
        self.teams.iter_mut().find(|t| t.id == id)
    }

    /// Every player and team written back into the retained payload, then
    /// re-encrypted under `salt`. The record order and count are the file's
    /// own (no player or team is added or removed through this API).
    pub fn to_bytes(&self, salt: &[u8; 320]) -> Result<Vec<u8>, SaveError> {
        let schema = self.schema;
        let mut payload = self.container.payload.clone();
        for (i, player) in self.players.iter().enumerate() {
            let at = schema.players.offset + i * schema.player.size;
            write_player(
                player,
                &mut payload[at..at + schema.player.size],
                schema.player,
            )?;
        }
        if let Some((section, appearance)) = &schema.appearance {
            for (i, player) in self.players.iter().enumerate() {
                let at = section.offset + self.appearance_records[i] * appearance.size;
                write_player(player, &mut payload[at..at + appearance.size], appearance)?;
            }
        }
        for (i, team) in self.teams.iter().enumerate() {
            let team_at = schema.teams.offset + i * schema.team.size;
            write_team(
                team,
                &mut payload[team_at..team_at + schema.team.size],
                schema.team,
            )?;
            let roster_at = schema.rosters.offset + i * schema.roster.size;
            write_roster(
                team,
                &mut payload[roster_at..roster_at + schema.roster.size],
                schema.roster,
            )?;
            let tactics_at = schema.tactics.offset + i * schema.tactic.size;
            write_tactics(
                team,
                &mut payload[tactics_at..tactics_at + schema.tactic.size],
                schema.tactic,
            )?;
        }
        let mut container = self.container.clone();
        container.payload = payload;
        Ok(container.to_bytes(salt)?)
    }

    /// `to_bytes` with a fresh salt, written next to `path` and renamed over
    /// it after the existing file was copied to `<path>.bak` (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&self, path: &std::path::Path) -> Result<(), SaveError> {
        use std::ffi::OsString;
        let bytes = self.to_bytes(&fresh_salt(&self.container.payload))?;
        let sibling = |ext: &str| {
            let mut name: OsString = path.as_os_str().to_owned();
            name.push(ext);
            std::path::PathBuf::from(name)
        };
        let tmp = sibling(".tmp");
        let bak = sibling(".bak");
        let write_and_swap = || -> std::io::Result<()> {
            std::fs::write(&tmp, bytes)?;
            if path.exists() {
                // The `.bak` holds the previous version and is overwritten on every
                // save on purpose: one level of backup.
                std::fs::copy(path, &bak)?;
            }
            std::fs::rename(&tmp, path)
        };
        if let Err(error) = write_and_swap() {
            // Best effort: the original error is the one to report.
            if let Err(cleanup) = std::fs::remove_file(&tmp) {
                log::debug!("could not remove {}: {cleanup}", tmp.display());
            }
            return Err(error.into());
        }
        Ok(())
    }
}

/// 320 salt bytes that differ per write: ten SHA-256 blocks over the payload,
/// the clock and a counter; the salt only diversifies the file key (the
/// master keys are public). The save's writer seeds it with the save's
/// payload; a Texport's writer passes it to `Texport::new` and `to_bytes`,
/// whose file key is its first 32 bytes.
#[cfg(not(target_arch = "wasm32"))]
pub fn fresh_salt(payload: &[u8]) -> [u8; 320] {
    use sha2::{Digest, Sha256};
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut salt = [0u8; 320];
    let mut block: Vec<u8> = {
        let mut hasher = Sha256::new();
        hasher.update(payload);
        hasher.update(nanos.to_be_bytes());
        hasher.update(0u64.to_be_bytes());
        hasher.finalize().to_vec()
    };
    for (n, chunk) in salt.chunks_mut(32).enumerate() {
        if n > 0 {
            let mut hasher = Sha256::new();
            hasher.update(&block);
            hasher.update((n as u64).to_be_bytes());
            block = hasher.finalize().to_vec();
        }
        chunk.copy_from_slice(&block);
    }
    salt
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;

    use super::*;
    use crate::test_support::{FIXTURES, open, test_salt};

    #[test]
    fn every_fixture_opens_and_rewrites_byte_for_byte() {
        let player_counts = [5060usize, 5060, 5060, 4646, 4830, 5060, 5060];
        let team_counts = [220usize, 220, 220, 202, 346, 220, 220];
        for (i, version) in FIXTURES.iter().enumerate() {
            let (file, bytes) = open(*version);
            assert_eq!(file.version(), *version);
            assert_eq!(
                file.players().len(),
                player_counts[i],
                "{version:?} players"
            );
            assert_eq!(file.teams().len(), team_counts[i], "{version:?} teams");
            assert!(file.player(70101).is_some(), "{version:?} player 70101");
            let team_id = if *version == PesVersion::Pes19 {
                100
            } else {
                701
            };
            let team = file.team(team_id).expect("the team exists");
            assert_eq!(team.id, team_id);
            assert_eq!(
                file.to_bytes(&test_salt()).expect("the save writes"),
                bytes,
                "{version:?} lossless"
            );
        }
        // The joined roster literals.
        let (file, _) = open(PesVersion::Pes15);
        assert_eq!(
            file.team(701).expect("team 701").roster[..4]
                .iter()
                .map(|s| s.player_id)
                .collect::<Vec<_>>(),
            [70101, 70102, 70103, 70104]
        );
        let (file, _) = open(PesVersion::Pes21);
        assert_eq!(
            file.team(701).expect("team 701").roster[..4]
                .iter()
                .map(|s| s.player_id)
                .collect::<Vec<_>>(),
            [70101, 70102, 70103, 70104]
        );
    }

    /// A save reports its container's serial, the one a PES 2017 Texport
    /// written from it carries.
    #[test]
    fn a_save_reports_its_containers_serial() {
        let serial: Vec<u8> = "S-1-5-21-0-0-0-1000"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert!(
            serial.len() > 1,
            "premise: the serial is neither empty nor a single byte"
        );
        let mut container = crate::fixtures::fixture_container(PesVersion::Pes17);
        container.serial = serial.clone();
        let bytes = container.to_bytes(&test_salt()).expect("container encodes");
        let file = EditFile::from_bytes(&bytes).expect("the save opens");
        assert_eq!(file.serial(), serial);
    }

    #[test]
    fn an_edit_round_trip_touches_only_the_edited_records() {
        let (mut file, bytes) = open(PesVersion::Pes17);
        let schema = schema_for(PesVersion::Pes17);
        let player_at = file
            .players()
            .iter()
            .position(|p| p.id == 70101)
            .expect("player 70101");
        let team_at = file
            .teams()
            .iter()
            .position(|t| t.id == 701)
            .expect("team 701");
        file.player_mut(70101).expect("player").basic.age = 33;
        file.player_mut(70101).expect("player").name = "EDITED".to_string();
        file.team_mut(701).expect("team").tactics.set_pieces.captain = 4;

        let out = file.to_bytes(&test_salt()).expect("the save writes");
        let back = EditFile::from_bytes(&out).expect("the save reopens");
        let player = back.player(70101).expect("player");
        assert_eq!(player.basic.age, 33);
        assert_eq!(player.name, "EDITED");
        assert_eq!(back.team(701).expect("team").tactics.set_pieces.captain, 4);
        // Every other entry is unchanged.
        for (a, b) in file.players().iter().zip(back.players()) {
            if a.id != 70101 {
                assert_eq!(a, b, "player {}", a.id);
            }
        }
        for (a, b) in file.teams().iter().zip(back.teams()) {
            if a.id != 701 {
                assert_eq!(a, b, "team {}", a.id);
            }
        }
        // The two payloads differ only inside the edited records.
        let old_payload = SaveContainer::decrypt(&bytes).expect("decrypt").payload;
        let new_payload = SaveContainer::decrypt(&out).expect("decrypt").payload;
        let ranges = [
            (
                schema.players.offset + player_at * schema.player.size,
                schema.player.size,
            ),
            (
                schema.teams.offset + team_at * schema.team.size,
                schema.team.size,
            ),
            (
                schema.rosters.offset + team_at * schema.roster.size,
                schema.roster.size,
            ),
            (
                schema.tactics.offset + team_at * schema.tactic.size,
                schema.tactic.size,
            ),
        ];
        assert_eq!(old_payload.len(), new_payload.len());
        let diffs: Vec<usize> = (0..old_payload.len())
            .filter(|&i| old_payload[i] != new_payload[i])
            .collect();
        assert!(!diffs.is_empty(), "the edit changed something");
        for i in diffs {
            assert!(
                ranges.iter().any(|(at, size)| i >= *at && i < at + size),
                "payload byte {i} changed outside the edited records"
            );
        }
    }

    /// A payload ending exactly at a section's count field still reads the
    /// count (and hits the record-end check); one byte shorter is the
    /// count-offset `Layout` error.
    #[test]
    fn a_payload_ending_at_the_count_fields_end_reads_it() {
        let schema = schema_for(PesVersion::Pes19);
        let section = &schema.players;
        let exact = section.count_offset + 2;
        let mut payload = vec![0u8; exact];
        // count 0: the count itself reads; the section's first record offset
        // lands past the payload, which is the other Layout error.
        let err = section_records(&payload, section, schema.player.size, "players")
            .expect_err("a payload past the section's end");
        match err {
            SaveError::Layout { detail, .. } => assert!(detail.contains("need"), "{detail}"),
            other => panic!("expected SaveError::Layout, got {other:?}"),
        }
        payload.pop();
        let err = section_records(&payload, section, schema.player.size, "players")
            .expect_err("a payload short of the count field");
        match err {
            SaveError::Layout { detail, .. } => {
                assert!(detail.contains("count offset"), "{detail}")
            }
            other => panic!("expected SaveError::Layout, got {other:?}"),
        }
        // A payload ending exactly at the section's end is fine: count 0,
        // `end == len` is inside bounds.
        let payload = vec![0u8; section.offset];
        assert!(
            section_records(&payload, section, schema.player.size, "players")
                .expect("count 0 fills the payload exactly")
                .is_empty()
        );
    }

    /// An edit through `players_mut`/`teams_mut` lands on the entry and
    /// survives `to_bytes` -> `from_bytes`.
    #[test]
    fn edits_through_the_mut_slices_are_written_back() {
        let (mut file, _) = open(PesVersion::Pes19);
        let player_id = file.players()[0].id;
        let team_id = file.teams()[0].id;
        file.players_mut().first_mut().expect("a player").name = "MUTATED".to_string();
        file.teams_mut().first_mut().expect("a team").short_name = "MTD".to_string();
        assert_eq!(file.player(player_id).expect("the player").name, "MUTATED");
        assert_eq!(file.team(team_id).expect("the team").short_name, "MTD");
        let bytes = file.to_bytes(&test_salt()).expect("to_bytes");
        let back = EditFile::from_bytes(&bytes).expect("the save reopens");
        assert_eq!(back.player(player_id).expect("the player").name, "MUTATED");
        assert_eq!(back.team(team_id).expect("the team").short_name, "MTD");
    }

    /// The write salt is 320 fresh bytes: nonzero, and its ten 32-byte
    /// chunks pairwise different (they chain-hash the payload and clock).
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn fresh_salt_is_diverse() {
        let salt = fresh_salt(b"a payload");
        assert!(salt.iter().any(|&b| b != 0), "an all-zero salt");
        let chunks: Vec<&[u8]> = salt.chunks(32).collect();
        for (i, a) in chunks.iter().enumerate() {
            for b in &chunks[i + 1..] {
                assert_ne!(a, b, "two salt chunks are equal");
            }
        }
    }

    #[test]
    fn the_decorated_name_census_matches_the_measurement() {
        use crate::model::names::display_name;
        // (version, players whose display name differs from the raw name).
        let decorated = [
            (PesVersion::Pes16, 60usize),
            (PesVersion::Pes19, 103),
            (PesVersion::Pes21, 183),
        ];
        for (version, expected) in decorated {
            let (file, _) = open(version);
            let mut seen = 0;
            for player in file.players() {
                let shown = display_name(&player.name);
                if shown != player.name {
                    seen += 1;
                }
                assert!(
                    !shown.contains('\x11'),
                    "{version:?} player {} display name {shown:?}",
                    player.id
                );
            }
            assert_eq!(seen, expected, "{version:?} decorated names");
        }
    }

    /// A unique temp folder for the native save/load test.
    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pes_savefile_test_{}_{}", name, std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("cleanup");
        }
        dir
    }

    #[test]
    fn save_and_load_round_trip_through_the_filesystem() {
        // A keyed version: PES 15's one-byte seed collides with the test salt's
        // once in 256 saves, which made the `assert_ne` below flaky.
        let (file, bytes) = open(PesVersion::Pes16);
        let dir = temp_dir("save");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("EDIT00000000");
        std::fs::write(&path, &bytes).expect("seed file");

        file.save(&path).expect("first save");
        let first = std::fs::read(&path).expect("read");
        assert_ne!(first, bytes, "a fresh salt rewrites the bytes");
        file.save(&path).expect("second save");
        let bak = dir.join("EDIT00000000.bak");
        assert_eq!(std::fs::read(&bak).expect("the .bak"), first);
        let loaded = EditFile::load(&path).expect("load");
        assert_eq!(loaded.players(), file.players());
        assert_eq!(loaded.teams(), file.teams());

        // A save whose folder does not exist is an io::Error.
        let missing = dir.join("nope").join("EDIT00000000");
        assert!(matches!(file.save(&missing), Err(SaveError::Io(_))));

        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn a_failed_save_leaves_no_tmp_sibling() {
        let (file, _bytes) = open(PesVersion::Pes15);
        let dir = temp_dir("save_fail");
        std::fs::create_dir_all(&dir).expect("temp dir");
        // `path` is a directory: the copy to `.bak` fails after `.tmp` was written.
        let path = dir.join("EDIT.bin");
        std::fs::create_dir(&path).expect("directory in the way");

        assert!(matches!(file.save(&path), Err(SaveError::Io(_))));
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        assert!(
            !std::path::Path::new(&tmp).exists(),
            "the .tmp sibling was cleaned up"
        );

        std::fs::remove_dir_all(&dir).expect("cleanup");
    }
}
