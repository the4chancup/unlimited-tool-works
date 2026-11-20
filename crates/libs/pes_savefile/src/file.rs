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
use crate::schema::{COACH_RECORD_SIZE, SectionLayout, VersionSchema, schema_for};

/// An open save: the retained container plus the decoded players and teams.
#[derive(Clone)]
pub struct EditFile {
    container: SaveContainer,
    schema: &'static VersionSchema,
    players: Vec<PlayerEntry>,
    /// The appearance record index of each player on versions with a
    /// separate appearance section (PES 14/15/16); `None` for a player the
    /// section holds no record for (PES 2014's edited-look players only).
    appearance_records: Vec<Option<usize>>,
    teams: Vec<TeamEntry>,
}

/// A team's own record bytes in the open save, as loaded: slices of the
/// retained payload, so an edit made through `team_mut` is not in them (the
/// Texport writer writes the entry over them).
pub struct TeamRecords<'a> {
    /// The team record.
    pub team: &'a [u8],
    /// The coach record the team's manager id names, as edited (the id the
    /// Texport writer writes over the team record); `None` when the version
    /// has no coach section or no record carries that id.
    pub coach: Option<&'a [u8]>,
    /// The roster record.
    pub roster: &'a [u8],
    /// The tactics record.
    pub tactics: &'a [u8],
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
            players.push(read_player(schema.version, rec, schema.player)?);
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
                let index = index_of.get(&player.id).copied();
                // PES 2014 holds a record only for an edited or created
                // player; the others refuse a missing one.
                let Some(index) = index else {
                    if schema.appearance_for_every_player() {
                        return Err(SaveError::RecordId {
                            section: "appearance",
                            id: player.id,
                        });
                    }
                    appearance_records.push(None);
                    continue;
                };
                read_player_into(schema.version, player, records[index], appearance)?;
                appearance_records.push(Some(index));
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
    /// 2014 and 2015). A PES 2017 Texport the editor writes carries the
    /// open save's.
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

    /// Team `id`'s own record bytes as loaded, and the coach record its
    /// manager id (as edited) names; `None` when the save has no team `id`. The records
    /// sit where `to_bytes` writes the team: its section's offset plus the
    /// team's index times the record size.
    pub fn team_records(&self, id: u32) -> Option<TeamRecords<'_>> {
        let schema = self.schema;
        let index = self.teams.iter().position(|t| t.id == id)?;
        let payload = &self.container.payload;
        let record = |section: &SectionLayout, size: usize| {
            let at = section.offset + index * size;
            &payload[at..at + size]
        };
        let team = record(&schema.teams, schema.team.size);
        Some(TeamRecords {
            team,
            coach: self.teams[index]
                .manager_id
                .and_then(|manager| self.coach_record(manager)),
            roster: record(&schema.rosters, schema.roster.size),
            tactics: record(&schema.tactics, schema.tactic.size),
        })
    }

    /// The coach record whose coach id is `manager`, by a scan of the coach
    /// section's counted records; `None` when the version has no coach
    /// section or no counted record inside the payload carries that id.
    fn coach_record(&self, manager: u32) -> Option<&[u8]> {
        let section = self.schema.coaches.as_ref()?;
        let payload = &self.container.payload;
        let count = payload.get(section.count_offset..section.count_offset + 2)?;
        let count = u16::from_le_bytes(count.try_into().expect("two bytes"));
        (0..usize::from(count))
            .map_while(|i| {
                let at = section.offset + i * COACH_RECORD_SIZE;
                payload.get(at..at + COACH_RECORD_SIZE)
            })
            .find(|coach| record_id(coach) == manager)
    }

    /// Player `id`'s own record bytes as loaded (the player record alone, on
    /// PES 14 to 16 too); `None` when the save has no player `id`. The
    /// record sits where `to_bytes` writes the player.
    pub fn player_record(&self, id: u32) -> Option<&[u8]> {
        let schema = self.schema;
        let index = self.players.iter().position(|p| p.id == id)?;
        let at = schema.players.offset + index * schema.player.size;
        Some(&self.container.payload[at..at + schema.player.size])
    }

    /// Whether player `id` has an appearance record: `true` for every
    /// player on a version that holds the look for all (the block inside
    /// the player record on PES 17 to 21, a record per player in the
    /// section on PES 15 and 16); on PES 2014 only for a player the
    /// appearance section holds a record for, one whose look was edited
    /// or who was created in the save. `false` for an id the save lacks.
    pub fn has_appearance_record(&self, id: u32) -> bool {
        let Some(index) = self.players.iter().position(|p| p.id == id) else {
            return false;
        };
        // On a version without a separate appearance section the vector
        // is empty: the player record holds the look itself.
        match self.appearance_records.get(index) {
            Some(&record) => record.is_some(),
            None => true,
        }
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
                schema.version,
                player,
                &mut payload[at..at + schema.player.size],
                schema.player,
            )?;
            // PES 2014's game reads a rostered player's record only under
            // the flags it sets itself on an edit, so a changed record is
            // marked as the game marks it.
            if schema.version == PesVersion::Pes14 {
                crate::codec::mark_edit_flags(
                    &self.container.payload[at..at + schema.player.size],
                    &mut payload[at..at + schema.player.size],
                    schema.player,
                );
            }
        }
        if let Some((section, appearance)) = &schema.appearance {
            for (i, player) in self.players.iter().enumerate() {
                if let Some(index) = self.appearance_records[i] {
                    let at = section.offset + index * appearance.size;
                    write_player(
                        schema.version,
                        player,
                        &mut payload[at..at + appearance.size],
                        appearance,
                    )?;
                }
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
        let player_counts = [11998usize, 5060, 5060, 5060, 4646, 4830, 5060, 5060];
        let team_counts = [478usize, 220, 220, 220, 202, 346, 220, 220];
        for (i, version) in FIXTURES.iter().enumerate() {
            let (file, bytes) = open(*version);
            assert_eq!(file.version(), *version);
            assert_eq!(
                file.players().len(),
                player_counts[i],
                "{version:?} players"
            );
            assert_eq!(file.teams().len(), team_counts[i], "{version:?} teams");
            // The ids of the player and team each check reads: the vanilla
            // PES 2014 save's first records (no /3/ team exists on it),
            // team 100 on PES 19 (no 701 either), /3/ elsewhere.
            let (player_id, team_id) = match *version {
                PesVersion::Pes14 => (450, 1),
                PesVersion::Pes19 => (70101, 100),
                PesVersion::Pes15
                | PesVersion::Pes16
                | PesVersion::Pes17
                | PesVersion::Pes18
                | PesVersion::Pes20
                | PesVersion::Pes21 => (70101, 701),
            };
            assert!(
                file.player(player_id).is_some(),
                "{version:?} player {player_id}"
            );
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

    /// PES 2014's appearance section holds a record only for a player whose
    /// look was edited or who was created in the save: the vanilla fixture
    /// holds none for its 11998 players and opens with every player
    /// record-less. On PES 15 a player's missing record is still the
    /// `RecordId` refusal.
    #[test]
    fn a_player_without_an_appearance_record_is_an_error_only_where_one_is_due() {
        let (file, _) = open(PesVersion::Pes14);
        assert_eq!(file.players().len(), 11998);
        let empty = crate::model::player::PlayerAppearance::default();
        assert!(
            file.players().iter().all(|p| p.appearance == empty),
            "no appearance record was applied to any player"
        );
        // Rename PES 15's first appearance record's id: its player is
        // orphaned, and a version holding a record per player refuses.
        let mut container = crate::fixtures::fixture_container(PesVersion::Pes15);
        let (section, _) = schema_for(PesVersion::Pes15)
            .appearance
            .as_ref()
            .expect("PES 15 has an appearance section");
        let orphan = record_id(&container.payload[section.offset..]);
        container.payload[section.offset..section.offset + 4]
            .copy_from_slice(&u32::MAX.to_le_bytes());
        let bytes = container.to_bytes(&test_salt()).expect("container encodes");
        let err = EditFile::from_bytes(&bytes)
            .err()
            .expect("the orphaned player refuses");
        assert!(
            matches!(
                err,
                SaveError::RecordId {
                    section: "appearance",
                    id
                } if id == orphan
            ),
            "{err:?}"
        );
    }

    /// `has_appearance_record` answers per player on PES 2014 — the
    /// vanilla fixture's first player has none, the section holding a
    /// record for edited or created players alone — and per version
    /// elsewhere: the PES 2015 fixture's first player has one. An id the
    /// save lacks reads false.
    #[test]
    fn has_appearance_record_answers_per_player_only_on_pes14() {
        let (file14, _) = open(PesVersion::Pes14);
        let first14 = file14.players()[0].id;
        assert!(!file14.has_appearance_record(first14));
        let (file15, _) = open(PesVersion::Pes15);
        let first15 = file15.players()[0].id;
        assert!(file15.has_appearance_record(first15));
        assert!(
            !file15.has_appearance_record(u32::MAX),
            "an id the save lacks"
        );
    }

    /// Every team's records are its own (each carries the team's id), and
    /// on 17-21 its manager id names a coach record of the save, the one
    /// handed over; every player's record carries his id. PES 14, 15 and 16
    /// have no coach section.
    #[test]
    fn every_teams_manager_id_names_a_coach_record() {
        for version in FIXTURES {
            let (file, _) = open(version);
            for team in file.teams() {
                let records = file.team_records(team.id).expect("the team's records");
                // PES 2014's team record holds the id in its u32's low 16
                // bits, the high 16 `0xffff`; the other versions' and the
                // roster and tactics records hold the plain id.
                let team_record_id = if version == PesVersion::Pes14 {
                    0xFFFF_0000 | team.id
                } else {
                    team.id
                };
                assert_eq!(
                    record_id(records.team),
                    team_record_id,
                    "{version:?} team {}",
                    team.id
                );
                for record in [records.roster, records.tactics] {
                    assert_eq!(record_id(record), team.id, "{version:?} team {}", team.id);
                }
                let manager = record_id(&records.team[4..]);
                match version {
                    PesVersion::Pes14 | PesVersion::Pes15 | PesVersion::Pes16 => {
                        assert!(records.coach.is_none(), "{version:?} team {}", team.id);
                    }
                    PesVersion::Pes17
                    | PesVersion::Pes18
                    | PesVersion::Pes19
                    | PesVersion::Pes20
                    | PesVersion::Pes21 => {
                        let coach = records.coach.unwrap_or_else(|| {
                            panic!("{version:?} team {}: no coach {manager}", team.id)
                        });
                        assert_eq!(coach.len(), COACH_RECORD_SIZE);
                        assert_eq!(record_id(coach), manager, "{version:?} team {}", team.id);
                    }
                }
            }
            for player in file.players() {
                let record = file.player_record(player.id).expect("the player's record");
                assert_eq!(record_id(record), player.id, "{version:?}");
            }
            assert!(file.team_records(0).is_none(), "{version:?}: no team 0");
            assert!(file.player_record(0).is_none(), "{version:?}: no player 0");
        }
    }

    /// PES 17 and 18 keep the manager id (the team record's u32 at +4) and
    /// the home stadium id (its u16 at +10) where PES 19 does; PES 15 and
    /// 16, whose record lacks the manager id's four bytes, keep the stadium
    /// id at +6. Every team reads them there.
    #[test]
    fn pes15_to_18_teams_read_their_manager_and_stadium_ids() {
        let mut stadiums = std::collections::BTreeSet::new();
        for (version, manager_at, stadium_at) in [
            (PesVersion::Pes15, None, 6),
            (PesVersion::Pes16, None, 6),
            (PesVersion::Pes17, Some(4), 10),
            (PesVersion::Pes18, Some(4), 10),
        ] {
            let (file, _) = open(version);
            for team in file.teams() {
                let record = file.team_records(team.id).expect("the team's records").team;
                let manager = manager_at.map(|at| record_id(&record[at..]));
                let stadium = u16::from_le_bytes(
                    record[stadium_at..stadium_at + 2]
                        .try_into()
                        .expect("a u16"),
                );
                stadiums.insert(stadium);
                assert_eq!(
                    (team.manager_id, team.stadium_id),
                    (manager, Some(stadium)),
                    "{version:?} team {}",
                    team.id
                );
            }
        }
        // Every PES 15, 16 and 17 fixture team has stadium 29; the PES 18
        // ones vary. None is the record's `ff ff` filler.
        assert!(stadiums.len() > 1, "premise: the stadiums differ");
        assert!(!stadiums.contains(&0xFFFF), "premise: no filler read");
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

    /// A PES 2014 write marks each changed player record the way the game
    /// marks its own edits: `EditedPlayer` on any change, `EditedName` on
    /// a name change and `EditedBasicSettings` on a nationality, age or
    /// stronger-foot change; an untouched record keeps its flags as read.
    #[test]
    fn a_pes14_player_edit_sets_the_flags_the_game_sets() {
        let (mut file, bytes) = open(PesVersion::Pes14);
        // An unchanged save still writes back byte for byte.
        assert_eq!(file.to_bytes(&test_salt()).expect("the save writes"), bytes);
        // Three players with every flag clear, and a fourth left alone.
        let ids: Vec<u32> = file
            .players()
            .iter()
            .filter(|player| {
                let flags = &player.edit_flags;
                !(flags.player || flags.name || flags.basic_settings)
            })
            .take(4)
            .map(|player| player.id)
            .collect();
        assert_eq!(ids.len(), 4, "four players with every flag clear");
        file.player_mut(ids[0]).expect("a player").basic.height += 1;
        file.player_mut(ids[1]).expect("a player").name = "RENAMED".to_string();
        file.player_mut(ids[2]).expect("a player").basic.age += 1;

        let out = file.to_bytes(&test_salt()).expect("the save writes");
        let back = EditFile::from_bytes(&out).expect("the save reopens");
        let flags = |id: u32| {
            let flags = &back.player(id).expect("the player").edit_flags;
            (flags.player, flags.name, flags.basic_settings)
        };
        assert_eq!(flags(ids[0]), (true, false, false), "a height edit");
        assert_eq!(flags(ids[1]), (true, true, false), "a name edit");
        assert_eq!(flags(ids[2]), (true, false, true), "an age edit");
        assert_eq!(flags(ids[3]), (false, false, false), "untouched");
    }

    /// A PES 17 team given team 702's manager and stadium 46 (the PES 19
    /// fixture's team 100 plays there) writes them at +4 (u32) and +10
    /// (u16); a PES 16 team given stadium 15 (where 38 teams of a real
    /// PES 16 save play: every fixture team plays in 29) writes it at +6.
    /// Each reads back, and no other byte of its record changes.
    #[test]
    fn a_manager_and_stadium_edit_writes_only_their_bytes() {
        for (version, manager, stadium, stadium_at) in [
            (PesVersion::Pes17, Some(702u32), 46u16, 10),
            (PesVersion::Pes16, None, 15, 6),
        ] {
            let (mut file, _) = open(version);
            let before = file.team_records(701).expect("team 701").team.to_vec();
            if let Some(manager) = manager {
                assert_ne!(
                    before[4..8],
                    manager.to_le_bytes(),
                    "{version:?} premise: another manager"
                );
            }
            assert_ne!(
                before[stadium_at..stadium_at + 2],
                stadium.to_le_bytes(),
                "{version:?} premise: another stadium"
            );
            let team = file.team_mut(701).expect("team 701");
            team.manager_id = manager;
            team.stadium_id = Some(stadium);

            let out = file.to_bytes(&test_salt()).expect("the save writes");
            let back = EditFile::from_bytes(&out).expect("the save reopens");
            let team = back.team(701).expect("team 701");
            assert_eq!(
                (team.manager_id, team.stadium_id),
                (manager, Some(stadium)),
                "{version:?}"
            );
            let mut expected = before;
            if let Some(manager) = manager {
                expected[4..8].copy_from_slice(&manager.to_le_bytes());
            }
            expected[stadium_at..stadium_at + 2].copy_from_slice(&stadium.to_le_bytes());
            assert_eq!(
                back.team_records(701).expect("team 701").team,
                expected,
                "{version:?}"
            );
        }
    }

    /// A team's coach record follows its manager id as edited, unsaved: the
    /// Texport writer writes the edited id over the team record, so the coach
    /// block beside it must be the one that id names, and an id naming no
    /// coach record gives none.
    #[test]
    fn a_teams_coach_record_follows_its_edited_manager_id() {
        let (mut file, _) = open(PesVersion::Pes17);
        let coach_id = |file: &EditFile| {
            file.team_records(701)
                .expect("team 701")
                .coach
                .map(record_id)
        };
        assert_eq!(coach_id(&file), Some(701), "premise: 701's own coach");

        file.team_mut(701).expect("team 701").manager_id = Some(702);
        assert_eq!(coach_id(&file), Some(702));

        file.team_mut(701).expect("team 701").manager_id = Some(999_999);
        assert_eq!(coach_id(&file), None, "no coach record carries 999999");
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

    /// The maintainer's PES 2014 saves open and write back byte for byte.
    /// In `.akko7v2`, Akko League VII's `/4ccg/` is team 857 (in a lower
    /// league; team 784, its /vg/ League id, is a blank 4cc team on this
    /// save) and its 23 players are the ones `.tmp/akko7/4ccg_names.txt`
    /// names, in roster order.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    #[ignore = "reads the maintainer's PES 2014 saves"]
    fn the_real_pes14_saves_open_and_rewrite_byte_for_byte() {
        let folder =
            std::path::Path::new("C:/Data/Documents/KONAMI/Pro Evolution Soccer 2014/save");
        for name in [
            "EDIT.bin.orig",
            "EDIT.bin.akko",
            "EDIT.bin.akko7v2",
            "EDIT.bin.akko7v2_flags",
        ] {
            let bytes = std::fs::read(folder.join(name)).expect("the save reads");
            let file = EditFile::from_bytes(&bytes).expect("the save opens");
            assert_eq!(file.version(), PesVersion::Pes14, "{name}");
            assert_eq!(
                file.to_bytes(&crate::container::file_salt(&bytes).expect("the salt"))
                    .expect("the save writes"),
                bytes,
                "{name} lossless"
            );
            if name == "EDIT.bin.akko7v2" {
                let names_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../.tmp/akko7/4ccg_names.txt");
                let names: Vec<String> = std::fs::read_to_string(&names_file)
                    .expect("the names file reads")
                    .lines()
                    .map(str::to_owned)
                    .collect();
                assert_eq!(names.len(), 23);
                let team = file.team(857).expect("team 857");
                assert_eq!(team.name, "/4ccg/");
                let got: Vec<&str> = team.roster[..names.len()]
                    .iter()
                    .map(|slot| {
                        file.player(slot.player_id)
                            .expect("a rostered player")
                            .name
                            .as_str()
                    })
                    .collect();
                assert_eq!(got, names, "team 857's roster by name");
                let records = file.appearance_records.iter().flatten().count();
                assert_eq!(
                    records,
                    file.players().len(),
                    "every player has an appearance record"
                );
                assert_eq!(records, 5060, "the appearance-record count");
            }
        }
    }

    /// `player2`, the PES 2014 `EDIT.bin` saved by the game after player
    /// 85723 was edited field by field: every measured field reads the
    /// value the game stored.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    #[ignore = "reads the maintainer's PES 2014 saves"]
    fn the_ingame_player2_save_reads_the_measured_fields() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../.tmp/ingame_saves/saves_14/player2");
        let bytes = std::fs::read(&path).expect("the save reads");
        let file = EditFile::from_bytes(&bytes).expect("the save opens");
        let player = file.player(85723).expect("player 85723");
        assert_eq!(player.name, "STEPNAME");
        // The game's screen order, each stat set to 60 plus its index.
        let stats = [
            player.stats.attacking_prowess,
            player.stats.ball_control,
            player.stats.dribbling,
            player.stats.low_pass,
            player.stats.lofted_pass,
            player.stats.finishing,
            player.stats.place_kicking,
            player.stats.swerve,
            player.stats.heading,
            player.stats.defensive_prowess,
            player.stats.ball_winning,
            player.stats.kicking_power,
            player.stats.speed,
            player.stats.explosive_power,
            player.stats.body_control,
            player.stats.jump,
            player.stats.goalkeeping,
            player.stats.catching.expect("PES 2014 stores Saving"),
            player.stats.tenacity.expect("PES 2014 stores Tenacity"),
            player.stats.stamina,
        ];
        assert_eq!(stats.to_vec(), (60..=79).collect::<Vec<u8>>());
        assert_eq!(player.stats.form, 0);
        assert_eq!(player.stats.injury_resistance, 1);
        assert_eq!(player.stats.weak_foot_usage, 2);
        assert_eq!(player.stats.weak_foot_accuracy, 4);
        assert_eq!(
            player.positions.playable,
            [0, 1, 2, 0, 1, 2, 1, 2, 0, 2, 0, 1, 0]
        );
        assert_eq!(player.motion.dribbling, Some(3));
        assert_eq!(player.motion.free_kick, 11);
        assert_eq!(player.motion.penalty_kick, 3);
        assert_eq!(player.motion.punt_kick, Some(3));
        assert!(player.edit_flags.player && player.edit_flags.name);
    }

    /// `.akko7v2`'s `/4ccg/` players: the basics, Tenacity, playing style,
    /// stronger foot, skills and COM styles as the game stores them.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    #[ignore = "reads the maintainer's PES 2014 saves"]
    fn the_akko7v2_save_reads_the_measured_player_fields() {
        use crate::model::playstyle::PlayStyle;

        let folder =
            std::path::Path::new("C:/Data/Documents/KONAMI/Pro Evolution Soccer 2014/save");
        let bytes = std::fs::read(folder.join("EDIT.bin.akko7v2")).expect("the save reads");
        let file = EditFile::from_bytes(&bytes).expect("the save opens");
        for (id, height, weight, age, tenacity, style, foot) in [
            (85702u32, 189, 85, 15, 77, PlayStyle::OffensiveGoalkeeper, 0),
            (85708, 189, 79, 20, 77, PlayStyle::AnchorMan, 0),
            (85720, 185, 74, 20, 92, PlayStyle::HolePlayer, 0),
            (85722, 194, 77, 20, 95, PlayStyle::DummyRunner, 0),
            (85723, 194, 85, 20, 95, PlayStyle::GoalPoacher, 1),
        ] {
            let player = file.player(id).expect("the player");
            assert_eq!(
                (
                    player.basic.height,
                    player.basic.weight,
                    player.basic.age,
                    player.stats.tenacity.expect("PES 2014 stores Tenacity"),
                    player.positions.stronger_foot,
                    player.basic.nationality,
                ),
                (height, weight, age, tenacity, foot, 146),
                "player {id}"
            );
            assert_eq!(
                crate::schema::playstyle::decode(PesVersion::Pes14, player.positions.playing_style)
                    .expect("a listed style"),
                style,
                "player {id}"
            );
        }
        // 85702's skills are GK Long Throw and Super-sub alone; 85723's
        // COM styles Speeding Bullet and Long Ranger alone.
        let skills = &file.player(85702).expect("the player").skills.skills;
        assert_eq!(skills.iter().filter(|held| **held).count(), 2);
        assert!(skills[20] && skills[26]);
        let com_styles = &file.player(85723).expect("the player").skills.com_styles;
        assert_eq!(com_styles.iter().filter(|held| **held).count(), 2);
        assert!(com_styles[2] && com_styles[6]);
    }

    /// `appear1`, the PES 2014 `EDIT.bin` pair the game saved with player
    /// 85720's every look item at its lowest (`appear1_min`) and highest
    /// (`appear1_max`): every mapped appearance field reads the model
    /// value the game stored, Sleeves and the shirttail through
    /// `stored_order` and each slider as its stored 0 to 14.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    #[ignore = "reads the maintainer's PES 2014 saves"]
    fn the_ingame_appear1_saves_read_the_measured_look_fields() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../.tmp/ingame_saves/saves_14");
        // (save name, boots id, (face, hair), strip (wristband,
        // undershorts, sleeves, socks, inners), (spectacles, ankle tape,
        // shirttail out)); each slider 0 in min, 14 in max.
        for (name, boots, flags, strip, extras) in [
            (
                "appear1_min",
                1011u32,
                (true, false),
                (0u8, 0, 1, 0, 0),
                (0u8, false, true),
            ),
            (
                "appear1_max",
                83,
                (true, true),
                (3, 1, 0, 2, 2),
                (7, true, false),
            ),
        ] {
            let bytes = std::fs::read(dir.join(name)).expect("the save reads");
            let file = EditFile::from_bytes(&bytes).expect("the save opens");
            let player = file.player(85720).expect("player 85720");
            assert!(file.has_appearance_record(85720), "{name}: the record");
            let a = &player.appearance;
            assert_eq!(a.boots_id, boots, "{name}: boots");
            assert_eq!(
                (player.edit_flags.face, player.edit_flags.hair),
                flags,
                "{name}: edit flags"
            );
            let want = if name == "appear1_min" { 0 } else { 14 };
            let sliders = [
                a.neck_length,
                a.neck_size,
                a.shoulder_height,
                a.shoulder_width,
                a.chest,
                a.waist,
                a.arm_length,
                a.arm_size,
                a.thigh,
                a.calf,
                a.leg_length,
                a.head_length,
                a.head_width,
                a.head_depth,
            ];
            assert_eq!(sliders, [want; 14], "{name}: the 14 sliders");
            let (wrist, under, sleeves, socks, inners) = strip;
            assert_eq!(a.wrist_taping, wrist, "{name}: wristband");
            assert_eq!(a.undershorts, under, "{name}: undershorts");
            assert_eq!(a.sleeves, sleeves, "{name}: sleeves");
            assert_eq!(a.socks, socks, "{name}: socks");
            assert_eq!(a.inners, inners, "{name}: inners");
            let (spectacles, ankle, untucked) = extras;
            assert_eq!(a.spectacles_style, spectacles, "{name}: spectacles");
            assert_eq!(a.ankle_taping, ankle, "{name}: ankle tape");
            assert_eq!(a.untucked, untucked, "{name}: shirttail");
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
