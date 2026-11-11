//! `Texport`: PES's own in-game team export/import format. Two eras:
//!
//! - 18-21 `.ted`: a 0x50 plaintext header, a 32-byte XOR key at 0x30, then
//!   the save's own team, roster, tactics and player records concatenated
//!   (sizes in `schema/texport.rs`, measured on real 18-21 files).
//! - 15-17 `TEXPORT00000000`: a whole savefile container; the team, roster,
//!   tactics and player records' payload offsets are
//!   `schema::texport::texport_old`'s, the team's and roster's derived from
//!   the save schema's record sizes (measured on every 15-17 texport of the
//!   reference machine).
//!
//! The module owns crypto and carry-through only: every record is decoded
//! and re-encoded by the schema codec, and unmodeled bytes ride along in the
//! retained plaintext/container.

use pes_version::PesVersion;

use crate::codec::{
    CodecError, read_player, read_player_into, read_roster_into, read_tactics_into, read_team,
    write_player, write_roster, write_tactics, write_team,
};
use crate::container::{ContainerError, MasterKey, SaveContainer, Scheme};
use crate::file::TeamRecords;
use crate::model::player::PlayerEntry;
#[cfg(test)]
use crate::model::player::STRIPPED_APPEARANCE_ID;
use crate::model::team::TeamEntry;
use crate::schema::texport::{PES17, TexportLayout, TexportOld, texport_layout, texport_old};
use crate::schema::{COACH_RECORD_SIZE, VersionSchema, schema_for};

/// The 18-21 key sits at 0x30; the body it encrypts starts at 0x50.
const KEY: usize = 0x30;
const HEADER: usize = 0x50;
/// The unmodeled block's byte length after tactics.
const BLOCK: usize = 660;
const TAIL: usize = 12;

/// An open texport: the retained ciphertext's plaintext (18-21) or container
/// (15-17) plus the decoded team and players.
#[derive(Debug)]
pub struct Texport {
    version: PesVersion,
    file: File,
    team: TeamEntry,
    /// Roster order, empty slots skipped.
    players: Vec<PlayerEntry>,
}

/// What a texport keeps for the write back.
#[derive(Debug)]
enum File {
    /// 18-21: the whole decrypted file (header, key and body).
    Ted(Vec<u8>),
    /// 15-17: the savefile container.
    Old(SaveContainer),
}

/// Why a buffer is not a texport, or the texport cannot be written.
#[derive(Debug, thiserror::Error)]
pub enum TexportError {
    /// The size or the record ids name no version.
    #[error("not a recognized texport")]
    UnknownVersion,
    /// `Some(version)` was passed a file of another size.
    #[error("a PES {version:?} texport is {expected} bytes, the input is {actual}")]
    WrongSize {
        /// The version asked for.
        version: PesVersion,
        /// That version's file size.
        expected: usize,
        /// The input's byte count.
        actual: usize,
    },
    /// The 15-17 container crypto failed.
    #[error(transparent)]
    Container(#[from] ContainerError),
    /// A savefile container that is a save, not a texport: the description
    /// is the file's own label, `Team Export Data NN` + the team name on a
    /// texport and `Edit Data` on a save. Checked before any record is read,
    /// so a save handed to `from_bytes` — of 15-17, or of 18-21 after its
    /// size matches no `.ted` layout — is refused rather than opening with
    /// a garbage team and no players.
    #[error("the container's description is not \"Team Export Data\" (a save's is \"Edit Data\")")]
    NotATexport,
    /// A record codec failed.
    #[error(transparent)]
    Codec(#[from] CodecError),
    /// A roster slot and its player record disagree.
    #[error("roster slot {slot} holds player {roster}, the record is {player}")]
    RosterMismatch {
        /// The roster slot.
        slot: usize,
        /// The roster's player id.
        roster: u32,
        /// The record's player id.
        player: u32,
    },
    /// `new` cannot synthesize a PES 2015 or 2016 file: there is no
    /// measured template.
    #[error("no texport template exists for {0:?}; write by editing a read file")]
    NoTemplate(PesVersion),
    /// A 15-17 file whose team, roster and tactics records disagree on the
    /// team id — the invariant every measured texport holds, so the payload
    /// is not laid out as one.
    #[error("the team, roster and tactics records disagree ({team}, {roster}, {tactics})")]
    IdsDisagree {
        /// The team record's id.
        team: u32,
        /// The roster record's id.
        roster: u32,
        /// The tactics record's id.
        tactics: u32,
    },
    /// `new` was handed no coach record: the team's manager id names none
    /// in the save. The game's own Texports always carry the team's coach
    /// record, so there is no stand-in to write.
    #[error("team {team}'s manager {manager} has no coach record in the save")]
    NoCoach {
        /// The team's id.
        team: u32,
        /// The manager id, the team record's u32 at +4.
        manager: u32,
    },
    /// A save record handed to `new` is not its record kind's size.
    #[error("the {record} record is {actual} bytes, not {expected}")]
    RecordSize {
        /// The record kind (`team`, `coach`, `roster`, `tactics`, `player`).
        record: &'static str,
        /// The version's size of that record.
        expected: usize,
        /// The slice's length.
        actual: usize,
    },
}

/// The little-endian u32 id at a record's start.
fn record_id(record: &[u8]) -> u32 {
    u32::from_le_bytes(record[..4].try_into().expect("an id field"))
}

/// The derived record offsets of an 18-21 file.
struct Offsets {
    team: usize,
    coach: usize,
    roster: usize,
    tactics: usize,
    players: usize,
    width: usize,
}

impl Offsets {
    /// Team at 0x50, then coach, roster, tactics, the 660-byte block, `width`
    /// player records and the tail; `width` must divide the remaining space
    /// exactly (the layouts are measured so it does).
    fn of(layout: &TexportLayout, schema: &VersionSchema) -> Offsets {
        let team = HEADER;
        let coach = team + schema.team.size;
        let roster = coach + COACH_RECORD_SIZE;
        let tactics = roster + schema.roster.size;
        let players = tactics + schema.tactic.size + BLOCK;
        let body = layout.size - TAIL - players;
        debug_assert_eq!(
            body % schema.player.size,
            0,
            "the player records fill the space between the block and the tail exactly"
        );
        let width = body / schema.player.size;
        Offsets {
            team,
            coach,
            roster,
            tactics,
            players,
            width,
        }
    }

    /// A PES 2017 payload's: `texport_old`'s offsets, the coach block right
    /// after the team record and one player record per roster slot, the
    /// roster `width` slots wide. PES 2017's player record has no separate
    /// appearance record, so the records sit a player record's size apart.
    fn pes17(old: &TexportOld, schema: &VersionSchema, width: usize) -> Offsets {
        Offsets {
            team: old.team_at,
            coach: old.team_at + schema.team.size,
            roster: old.roster_at,
            tactics: old.tactics_at,
            players: old.players_at,
            width,
        }
    }
}

/// The 18-21 body XOR'd with the 32-byte key at 0x30, starting at the
/// layout's key index and wrapping at 32. The same function encrypts: the
/// key bytes sit below 0x50 and pass through.
fn crypt(bytes: &[u8], layout: &TexportLayout) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let key = &bytes[KEY..HEADER];
    let mut k = layout.key_index;
    for b in out.iter_mut().skip(HEADER) {
        *b ^= key[k];
        k = (k + 1) % 0x20;
    }
    out
}

/// Whether the decrypted candidate's team, roster and tactics records carry
/// one id — the consistency check behind a size match: every version's file
/// size is distinct, so a size-matched buffer whose record ids disagree is
/// not that version's texport.
fn ids_agree(plaintext: &[u8], layout: &TexportLayout, version: PesVersion) -> bool {
    let schema = schema_for(version);
    let off = Offsets::of(layout, schema);
    let id = |at: usize| record_id(&plaintext[at..]);
    id(off.team) == id(off.roster) && id(off.roster) == id(off.tactics)
}

/// Pairs each filled roster slot below `width` with the next of `players`,
/// which must be the filled slots' players in roster order: a player whose id
/// is not the slot's, a slot left without one, or a player left over is
/// `RosterMismatch`.
fn slot_players<'a>(
    team: &TeamEntry,
    mut players: impl Iterator<Item = &'a PlayerEntry>,
    width: usize,
) -> Result<Vec<(usize, &'a PlayerEntry)>, TexportError> {
    let mut pairs = Vec::new();
    for (slot, roster) in team.roster.iter().enumerate().take(width) {
        if roster.player_id == 0 {
            continue;
        }
        let player = players.next().ok_or(TexportError::RosterMismatch {
            slot,
            roster: roster.player_id,
            player: 0,
        })?;
        if player.id != roster.player_id {
            return Err(TexportError::RosterMismatch {
                slot,
                roster: roster.player_id,
                player: player.id,
            });
        }
        pairs.push((slot, player));
    }
    if let Some(extra) = players.next() {
        return Err(TexportError::RosterMismatch {
            slot: width,
            roster: 0,
            player: extra.id,
        });
    }
    Ok(pairs)
}

/// Each filled roster slot below `width` with its player record, which
/// `record(slot)` returns: a record whose id is not the slot's is
/// `RosterMismatch`.
fn slot_records<'a>(
    team: &TeamEntry,
    width: usize,
    record: impl Fn(usize) -> Result<&'a [u8], TexportError>,
) -> Result<Vec<(usize, &'a [u8])>, TexportError> {
    let mut pairs = Vec::new();
    for (slot, roster) in team.roster.iter().enumerate().take(width) {
        if roster.player_id == 0 {
            continue;
        }
        let rec = record(slot)?;
        let id = record_id(rec);
        if id != roster.player_id {
            return Err(TexportError::RosterMismatch {
                slot,
                roster: roster.player_id,
                player: id,
            });
        }
        pairs.push((slot, rec));
    }
    Ok(pairs)
}

/// The record writer of `new` and of `to_bytes`, PES 2017 to 2021 both:
/// team, roster, tactics and each filled roster slot's player record into
/// `plain`; every other byte, the coach block's included, is kept as it
/// was. `players` must be the filled slots' records in roster order
/// (`RosterMismatch` on an id or count disagreement).
fn write_records<'a>(
    plain: &mut [u8],
    off: &Offsets,
    schema: &VersionSchema,
    team: &TeamEntry,
    players: impl Iterator<Item = &'a PlayerEntry>,
) -> Result<(), TexportError> {
    write_team(
        team,
        &mut plain[off.team..off.team + schema.team.size],
        schema.team,
    )?;
    write_roster(
        team,
        &mut plain[off.roster..off.roster + schema.roster.size],
        schema.roster,
    )?;
    write_tactics(
        team,
        &mut plain[off.tactics..off.tactics + schema.tactic.size],
        schema.tactic,
    )?;
    for (slot, player) in slot_players(team, players, off.width)? {
        let at = off.players + slot * schema.player.size;
        // The game's own exports hold each player's own id at the appearance-id slot
        // (every PES 17-21 Texport measured), whatever the save's entry holds: a -1 there
        // is the save's state, not the export's. PES 15/16 records have no
        // such slot, and `write_player` leaves a field the schema lacks unwritten.
        let player = PlayerEntry {
            appearance_id: Some(player.id),
            ..player.clone()
        };
        write_player(
            &player,
            &mut plain[at..at + schema.player.size],
            schema.player,
        )?;
    }
    Ok(())
}

/// `bytes`, a save record of kind `record`, if it is `expected` bytes long.
fn sized<'a>(
    record: &'static str,
    bytes: &'a [u8],
    expected: usize,
) -> Result<&'a [u8], TexportError> {
    if bytes.len() != expected {
        return Err(TexportError::RecordSize {
            record,
            expected,
            actual: bytes.len(),
        });
    }
    Ok(bytes)
}

/// A fresh file's records, `new`'s on PES 2017 to 2021: the open save's
/// own record bytes copied into their regions (the team, the coach record
/// its manager id names, roster, tactics, and each filled roster slot's
/// player), then the entries written over them by `write_records`. The
/// game writes a Texport by copying the save's records whole, and they
/// hold bytes the codec does not model, so a byte no field covers keeps
/// the save's value. `players` pairs each entry, in roster order, with its save
/// record, whose id must be the entry's (`RosterMismatch`).
fn lay_records(
    plain: &mut [u8],
    off: &Offsets,
    schema: &VersionSchema,
    team: &TeamEntry,
    records: &TeamRecords<'_>,
    players: &[(&PlayerEntry, &[u8])],
) -> Result<(), TexportError> {
    let team_record = sized("team", records.team, schema.team.size)?;
    let coach = records.coach.ok_or(TexportError::NoCoach {
        team: team.id,
        manager: team
            .manager_id
            .unwrap_or_else(|| record_id(&team_record[4..])),
    })?;
    let bases = [
        (off.team, team_record),
        (off.coach, sized("coach", coach, COACH_RECORD_SIZE)?),
        (
            off.roster,
            sized("roster", records.roster, schema.roster.size)?,
        ),
        (
            off.tactics,
            sized("tactics", records.tactics, schema.tactic.size)?,
        ),
    ];
    for (at, bytes) in bases {
        plain[at..at + bytes.len()].copy_from_slice(bytes);
    }
    let entries = players.iter().map(|(player, _)| *player);
    // `slot_players` pairs every entry in order or refuses, so its pairs
    // line up with `players`.
    for ((slot, player), (_, record)) in slot_players(team, entries, off.width)?
        .into_iter()
        .zip(players)
    {
        let record = sized("player", record, schema.player.size)?;
        let id = record_id(record);
        if id != player.id {
            return Err(TexportError::RosterMismatch {
                slot,
                roster: player.id,
                player: id,
            });
        }
        let at = off.players + slot * schema.player.size;
        plain[at..at + record.len()].copy_from_slice(record);
    }
    write_records(
        plain,
        off,
        schema,
        team,
        players.iter().map(|(player, _)| *player),
    )
}

/// A fresh 18-21 plaintext: the layout's header, the key `salt[..32]`, the
/// records (`lay_records`), a zero 660-byte block and the tail.
fn ted_plaintext(
    version: PesVersion,
    team: &TeamEntry,
    records: &TeamRecords<'_>,
    players: &[(&PlayerEntry, &[u8])],
    salt: &[u8; 320],
) -> Result<Vec<u8>, TexportError> {
    let schema = schema_for(version);
    let layout = texport_layout(version).ok_or(TexportError::NoTemplate(version))?;
    let off = Offsets::of(layout, schema);
    let mut plain = vec![0u8; layout.size];
    plain[..KEY].copy_from_slice(&layout.header);
    plain[KEY..HEADER].copy_from_slice(&salt[..32]);
    plain[layout.size - TAIL..].copy_from_slice(&layout.tail);
    lay_records(&mut plain, &off, schema, team, records, players)?;
    Ok(plain)
}

/// The game's own thumbnail, the same in every PES 2017 Texport it writes.
const PES17_LOGO: &[u8] = include_bytes!("pes17_texport_logo.png");

/// A fresh PES 2017 container: a payload zero but for `PES17`'s template
/// and the records (`lay_records`) at `texport_old`'s offsets, under the
/// PES 2017 key. Its description is `Team Export Data 01` with the team's
/// name at 0x80, the name the game lists; its identifier is zero but for
/// `TEXPORT` at 64, the 64 bytes before it differing per file and
/// unchecked by the game.
fn pes17_container(
    team: &TeamEntry,
    records: &TeamRecords<'_>,
    players: &[(&PlayerEntry, &[u8])],
    serial: &[u8],
) -> Result<SaveContainer, TexportError> {
    let schema = schema_for(PesVersion::Pes17);
    let old = texport_old(PesVersion::Pes17).expect("PES 2017 has payload offsets");
    let off = Offsets::pes17(&old, schema, team.roster.len());
    let mut payload = vec![0u8; PES17.size];
    payload[..PES17.header.len()].copy_from_slice(&PES17.header);
    let marker = PES17.marker_at..PES17.marker_at + PES17.marker.len();
    payload[marker].copy_from_slice(&PES17.marker);
    payload[PES17.size - PES17.tail.len()..].copy_from_slice(&PES17.tail);
    lay_records(&mut payload, &off, schema, team, records, players)?;

    const LABEL: &[u8] = b"Team Export Data 01";
    let mut description = vec![0u8; 384];
    description[..LABEL.len()].copy_from_slice(LABEL);
    describe(&mut description, &team.name);
    let mut identifier = vec![0u8; MasterKey::Pes17.header_size() - 80];
    identifier[64..71].copy_from_slice(b"TEXPORT");
    Ok(SaveContainer {
        scheme: Scheme::Keyed(MasterKey::Pes17),
        description,
        logo: PES17_LOGO.to_vec(),
        payload,
        identifier,
        serial: serial.to_vec(),
    })
}

/// Writes `name` into a 15-17 container's description at 0x80, the name
/// the game lists, clearing whatever an older name left after it. Called
/// after the team record is written, which refuses a name longer than its
/// 70-byte field, so it fits the description's 256 bytes there.
fn describe(description: &mut [u8], name: &str) {
    let field = &mut description[0x80..];
    field.fill(0);
    field[..name.len()].copy_from_slice(name.as_bytes());
}

impl Texport {
    /// `version = None` detects: the container's version for 15-17; by size
    /// for 18-21 (every version's file size is distinct since PES 20's was
    /// measured), the size-matched layout's team, roster and tactics record
    /// ids checked for agreement so a same-sized non-texport stays
    /// `UnknownVersion`. An explicit `version` for 15-17 defers to the
    /// container's own.
    pub fn from_bytes(bytes: &[u8], version: Option<PesVersion>) -> Result<Texport, TexportError> {
        match version {
            Some(version) => match texport_layout(version) {
                Some(layout) => {
                    if bytes.len() != layout.size {
                        return Err(TexportError::WrongSize {
                            version,
                            expected: layout.size,
                            actual: bytes.len(),
                        });
                    }
                    Self::open_ted(bytes, version, layout)
                }
                None => Self::open_old(SaveContainer::decrypt(bytes)?),
            },
            None => {
                let versions: Vec<PesVersion> = PesVersion::ALL
                    .iter()
                    .copied()
                    .filter(|v| {
                        texport_layout(*v).is_some_and(|layout| {
                            bytes.len() == layout.size
                                && ids_agree(&crypt(bytes, layout), layout, *v)
                        })
                    })
                    .collect();
                match versions[..] {
                    [version] => Self::open_ted(
                        bytes,
                        version,
                        texport_layout(version).expect("filtered on Some"),
                    ),
                    _ => match SaveContainer::decrypt(bytes) {
                        Ok(container) => Self::open_old(container),
                        Err(ContainerError::Unrecognized) => Err(TexportError::UnknownVersion),
                        Err(error) => Err(TexportError::Container(error)),
                    },
                }
            }
        }
    }

    /// An 18-21 file: decrypt, then read the records at the derived offsets.
    fn open_ted(
        bytes: &[u8],
        version: PesVersion,
        layout: &TexportLayout,
    ) -> Result<Texport, TexportError> {
        let schema = schema_for(version);
        let off = Offsets::of(layout, schema);
        let plain = crypt(bytes, layout);
        let mut team = read_team(&plain[off.team..off.team + schema.team.size], schema.team)?;
        read_roster_into(
            &mut team,
            &plain[off.roster..off.roster + schema.roster.size],
            schema.roster,
        )?;
        read_tactics_into(
            &mut team,
            &plain[off.tactics..off.tactics + schema.tactic.size],
            schema.tactic,
        )?;
        let records = slot_records(&team, off.width, |slot| {
            let at = off.players + slot * schema.player.size;
            Ok(&plain[at..at + schema.player.size])
        })?;
        let players = records
            .into_iter()
            .map(|(_, record)| read_player(record, schema.player))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Texport {
            version,
            file: File::Ted(plain),
            team,
            players,
        })
    }

    /// A 15-17 file: the container's payload holds the team, roster and
    /// tactics records at `texport_old`'s offsets, then the player records
    /// in the roster's order — one record per filled roster slot, its id
    /// checked against the slot's (15/16: a player record then its
    /// appearance record per player).
    fn open_old(container: SaveContainer) -> Result<Texport, TexportError> {
        // The description is the file's own label, checked before any record
        // is read: without it a save opens silently with a garbage team and
        // no players.
        if !container.description.starts_with(b"Team Export Data") {
            return Err(TexportError::NotATexport);
        }
        let version = container.version();
        let schema = schema_for(version);
        let old = texport_old(version).ok_or(TexportError::UnknownVersion)?;
        let payload = &container.payload;
        let record = |at: usize, size: usize| -> Result<&[u8], TexportError> {
            payload.get(at..at + size).ok_or_else(|| {
                CodecError::RecordSize {
                    expected: size,
                    got: payload.len().saturating_sub(at),
                }
                .into()
            })
        };
        // The three records' ids agree in every measured file.
        let team_rec = record(old.team_at, schema.team.size)?;
        let roster_rec = record(old.roster_at, schema.roster.size)?;
        let tactics_rec = record(old.tactics_at, schema.tactic.size)?;
        let (team_id, roster_id, tactics_id) = (
            record_id(team_rec),
            record_id(roster_rec),
            record_id(tactics_rec),
        );
        if team_id != tactics_id || roster_id != tactics_id {
            return Err(TexportError::IdsDisagree {
                team: team_id,
                roster: roster_id,
                tactics: tactics_id,
            });
        }
        let mut team = read_team(team_rec, schema.team)?;
        read_roster_into(&mut team, roster_rec, schema.roster)?;
        read_tactics_into(&mut team, tactics_rec, schema.tactic)?;
        let stride = schema
            .appearance
            .as_ref()
            .map_or(schema.player.size, |(_, a)| schema.player.size + a.size);
        let at = |slot: usize| old.players_at + slot * stride;
        let records = slot_records(&team, team.roster.len(), |slot| {
            record(at(slot), schema.player.size)
        })?;
        let mut players = Vec::new();
        for (slot, rec) in records {
            let mut player = read_player(rec, schema.player)?;
            if let Some((_, appearance)) = &schema.appearance {
                let rec = record(at(slot) + schema.player.size, appearance.size)?;
                read_player_into(&mut player, rec, appearance)?;
            }
            players.push(player);
        }
        Ok(Texport {
            version,
            file: File::Old(container),
            team,
            players,
        })
    }

    /// A fresh file written onto the open save's own records: `team` and
    /// `players` (in roster order, each paired with its save record, ids
    /// checked against the roster and the record: `RosterMismatch`) are
    /// written over copies of `records` and the player records, so a byte
    /// the codec does not model keeps the save's value. The coach block is
    /// `records.coach` as it is (`NoCoach` when it is `None`); a record of
    /// the wrong size is `RecordSize`. Around the records, on 18-21 the
    /// layout's header, a zero 660-byte block and the tail, with a key from
    /// `salt[..32]`; on 17 a PES 2017 container around `PES17`'s payload
    /// template. `serial` is the writer's serial a 15-17 container carries
    /// (the open save's, `EditFile::serial`); it is unused on 18-21, as
    /// `to_bytes` leaves `salt` unused there. 15 and 16 are `NoTemplate`:
    /// write by reading a real file and replacing its team and players
    /// (`team_mut`/`players_mut`).
    pub fn new(
        version: PesVersion,
        team: &TeamEntry,
        records: &TeamRecords<'_>,
        players: &[(&PlayerEntry, &[u8])],
        salt: &[u8; 320],
        serial: &[u8],
    ) -> Result<Texport, TexportError> {
        let file = match version {
            PesVersion::Pes18 | PesVersion::Pes19 | PesVersion::Pes20 | PesVersion::Pes21 => {
                File::Ted(ted_plaintext(version, team, records, players, salt)?)
            }
            PesVersion::Pes17 => File::Old(pes17_container(team, records, players, serial)?),
            PesVersion::Pes15 | PesVersion::Pes16 => {
                return Err(TexportError::NoTemplate(version));
            }
        };
        Ok(Texport {
            version,
            file,
            team: team.clone(),
            // `players` are what the file holds, so each entry keeps the id
            // `write_records` put at the appearance-id slot: his own, not the
            // save entry's.
            players: players
                .iter()
                .map(|(p, _)| PlayerEntry {
                    appearance_id: Some(p.id),
                    ..(*p).clone()
                })
                .collect(),
        })
    }

    /// The game version this texport belongs to.
    pub fn version(&self) -> PesVersion {
        self.version
    }

    /// The team (team, roster and tactics joined).
    pub fn team(&self) -> &TeamEntry {
        &self.team
    }

    /// Every player, in roster order, empty slots skipped.
    pub fn players(&self) -> &[PlayerEntry] {
        &self.players
    }

    /// The team for editing.
    pub fn team_mut(&mut self) -> &mut TeamEntry {
        &mut self.team
    }

    /// Every player for editing, in roster order.
    pub fn players_mut(&mut self) -> &mut [PlayerEntry] {
        &mut self.players
    }

    /// The export as a Team TOML document: team, tactics and the rostered
    /// players, ready to `TeamToml::apply` onto a save. On every version
    /// the `[team]` section and the roster's shirt numbers are the file's.
    pub fn to_team_toml(
        &self,
    ) -> Result<crate::interchange::team_toml::TeamToml, crate::interchange::team_toml::TeamTomlError>
    {
        let players: Vec<&PlayerEntry> = self.players.iter().collect();
        crate::interchange::team_toml::TeamToml::from_team(self.version, &self.team, &players)
    }

    /// The records re-encoded into the plaintext (unmodeled bytes as read),
    /// then encrypted: XOR with the file's own key on 18-21 — `salt` is
    /// unused on that path — the container with `salt` on 15-17, whose
    /// description takes the team's name at 0x80, the name the game lists.
    pub fn to_bytes(&self, salt: &[u8; 320]) -> Result<Vec<u8>, TexportError> {
        match &self.file {
            File::Ted(plaintext) => {
                let layout = texport_layout(self.version).expect("a ted holds a layout");
                let schema = schema_for(self.version);
                let off = Offsets::of(layout, schema);
                let mut plain = plaintext.clone();
                write_records(&mut plain, &off, schema, &self.team, self.players.iter())?;
                Ok(crypt(&plain, layout))
            }
            File::Old(container) => {
                let mut container = container.clone();
                match self.version {
                    PesVersion::Pes17 => self.write_pes17(&mut container)?,
                    PesVersion::Pes15 | PesVersion::Pes16 => {
                        self.write_pes15_16(&mut container.payload)?;
                    }
                    PesVersion::Pes18
                    | PesVersion::Pes19
                    | PesVersion::Pes20
                    | PesVersion::Pes21 => {
                        unreachable!("an 18-21 texport is a `.ted`")
                    }
                }
                describe(&mut container.description, &self.team.name);
                Ok(container.to_bytes(salt)?)
            }
        }
    }

    /// A PES 2017 container's records, written as `new` writes them
    /// (`write_records`: each player's own id at the appearance-id slot).
    fn write_pes17(&self, container: &mut SaveContainer) -> Result<(), TexportError> {
        let schema = schema_for(PesVersion::Pes17);
        let old = texport_old(PesVersion::Pes17).expect("PES 2017 has payload offsets");
        let off = Offsets::pes17(&old, schema, self.team.roster.len());
        // `team_mut` can fill a slot past the records the payload holds: a
        // bounds refusal, not a slice panic.
        let end = off.players + off.width * schema.player.size;
        if container.payload.len() < end {
            return Err(CodecError::RecordSize {
                expected: end,
                got: container.payload.len(),
            }
            .into());
        }
        write_records(
            &mut container.payload,
            &off,
            schema,
            &self.team,
            self.players.iter(),
        )?;
        Ok(())
    }

    /// A PES 2015 or 2016 payload's records: team, roster, tactics, then
    /// each filled roster slot's player record and its appearance record.
    /// These records hold no appearance-id slot and no coach block.
    fn write_pes15_16(&self, payload: &mut [u8]) -> Result<(), TexportError> {
        let schema = schema_for(self.version);
        let old = texport_old(self.version).expect("an old holds offsets");
        write_team(
            &self.team,
            &mut payload[old.team_at..old.team_at + schema.team.size],
            schema.team,
        )?;
        write_roster(
            &self.team,
            &mut payload[old.roster_at..old.roster_at + schema.roster.size],
            schema.roster,
        )?;
        write_tactics(
            &self.team,
            &mut payload[old.tactics_at..old.tactics_at + schema.tactic.size],
            schema.tactic,
        )?;
        let stride = schema
            .appearance
            .as_ref()
            .map_or(schema.player.size, |(_, a)| schema.player.size + a.size);
        let width = self.team.roster.len();
        for (slot, player) in slot_players(&self.team, self.players.iter(), width)? {
            let at = old.players_at + slot * stride;
            // `team_mut` can fill a slot past the records the payload
            // holds: a bounds refusal, not a slice panic.
            let got = payload.len().saturating_sub(at);
            let record = payload
                .get_mut(at..at + stride)
                .ok_or(CodecError::RecordSize {
                    expected: stride,
                    got,
                })?;
            write_player(player, &mut record[..schema.player.size], schema.player)?;
            if let Some((_, appearance)) = &schema.appearance {
                write_player(player, &mut record[schema.player.size..], appearance)?;
            }
        }
        Ok(())
    }
}

/// A read PES 2017 to 2021 Texport's own records, sliced at its `Offsets`:
/// the team, coach, roster and tactics records, and each filled roster
/// slot's player record in roster order. The tests build `new`'s input from
/// a fixture Texport with it, as the save editor builds it from a save.
#[cfg(test)]
pub(crate) fn file_records(texport: &Texport) -> (TeamRecords<'_>, Vec<&[u8]>) {
    let schema = schema_for(texport.version);
    let (plain, off) = match &texport.file {
        File::Ted(plain) => {
            let layout = texport_layout(texport.version).expect("a ted holds a layout");
            (plain.as_slice(), Offsets::of(layout, schema))
        }
        File::Old(container) => {
            let old = texport_old(texport.version).expect("an old texport has offsets");
            let width = texport.team.roster.len();
            (
                container.payload.as_slice(),
                Offsets::pes17(&old, schema, width),
            )
        }
    };
    let slice = |at: usize, size: usize| &plain[at..at + size];
    let records = TeamRecords {
        team: slice(off.team, schema.team.size),
        coach: Some(slice(off.coach, COACH_RECORD_SIZE)),
        roster: slice(off.roster, schema.roster.size),
        tactics: slice(off.tactics, schema.tactic.size),
    };
    let players = texport
        .team
        .roster
        .iter()
        .enumerate()
        .take(off.width)
        .filter(|(_, slot)| slot.player_id != 0)
        .map(|(slot, _)| slice(off.players + slot * schema.player.size, schema.player.size))
        .collect();
    (records, players)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::EditFile;
    use crate::test_support::{open, test_salt};

    /// `players` each paired with the record at its position in `records`.
    fn paired<'a>(
        players: &[&'a PlayerEntry],
        records: &[&'a [u8]],
    ) -> Vec<(&'a PlayerEntry, &'a [u8])> {
        players
            .iter()
            .copied()
            .zip(records.iter().copied())
            .collect()
    }

    /// The first team of `file` with a rostered player.
    fn first_rostered_team(file: &EditFile) -> &TeamEntry {
        file.teams()
            .iter()
            .find(|team| team.roster.iter().any(|slot| slot.player_id != 0))
            .expect("a team with players")
    }

    /// `team`'s rostered players in roster order, each with his save
    /// record, as the save editor hands them to `new`.
    fn save_players<'a>(file: &'a EditFile, team: &TeamEntry) -> Vec<(&'a PlayerEntry, &'a [u8])> {
        team.roster
            .iter()
            .filter(|slot| slot.player_id != 0)
            .map(|slot| {
                let id = slot.player_id;
                let player = file.player(id).expect("the player is in the save");
                (player, file.player_record(id).expect("his record"))
            })
            .collect()
    }

    /// A written PES 2017 to 2021 file's plaintext (17: the container's
    /// payload; 18-21: the decrypted file) and its record offsets, for a
    /// roster `width` slots wide on 17.
    fn written_plain(version: PesVersion, bytes: &[u8], width: usize) -> (Vec<u8>, Offsets) {
        let schema = schema_for(version);
        match texport_layout(version) {
            Some(layout) => (crypt(bytes, layout), Offsets::of(layout, schema)),
            None => {
                let old = texport_old(version).expect("a 15-17 layout");
                let payload = SaveContainer::decrypt(bytes).expect("decrypts").payload;
                (payload, Offsets::pes17(&old, schema, width))
            }
        }
    }

    /// The first offset where `written` and `expected` differ, `None` when
    /// they are equal: an assertion on it names the byte, not two whole
    /// records.
    fn first_difference(written: &[u8], expected: &[u8]) -> Option<usize> {
        written
            .iter()
            .zip(expected)
            .position(|(a, b)| a != b)
            .or((written.len() != expected.len()).then(|| written.len().min(expected.len())))
    }

    /// The fixture .ted files, one per version.
    const TEDS: [(PesVersion, &[u8]); 4] = [
        (
            PesVersion::Pes18,
            include_bytes!("../../tests/fixtures/pes18_texport.ted"),
        ),
        (
            PesVersion::Pes19,
            include_bytes!("../../tests/fixtures/pes19_texport.ted"),
        ),
        (
            PesVersion::Pes21,
            include_bytes!("../../tests/fixtures/pes21_texport.ted"),
        ),
        (
            PesVersion::Pes20,
            include_bytes!("../../tests/fixtures/pes20_texport.ted"),
        ),
    ];

    #[test]
    fn every_layouts_width_divides_exactly() {
        for version in PesVersion::ALL {
            let Some(layout) = texport_layout(version) else {
                continue;
            };
            let off = Offsets::of(layout, schema_for(version));
            assert_eq!(
                off.players + off.width * schema_for(version).player.size + TAIL,
                layout.size,
                "{version:?}"
            );
        }
    }

    #[test]
    fn a_stripped_player_is_exported_with_his_own_id_as_the_game_exports_one() {
        for (version, bytes) in TEDS {
            let fixture = Texport::from_bytes(bytes, None).expect("opens");
            let mut stripped: Vec<PlayerEntry> = fixture.players().to_vec();
            for player in &mut stripped {
                player.appearance_id = Some(STRIPPED_APPEARANCE_ID);
            }
            let players: Vec<&PlayerEntry> = stripped.iter().collect();
            let (records, player_records) = file_records(&fixture);
            let made = Texport::new(
                version,
                fixture.team(),
                &records,
                &paired(&players, &player_records),
                &test_salt(),
                &[],
            )
            .expect("synthesizes");
            // The stored players are what the file holds: each one's own id,
            // as the game's exports carry it.
            for player in made.players() {
                assert_eq!(player.appearance_id, Some(player.id), "{version:?}");
            }
            let out = made.to_bytes(&test_salt()).expect("encodes");
            let back = Texport::from_bytes(&out, Some(version)).expect("reopens");
            for player in back.players() {
                assert_eq!(player.appearance_id, Some(player.id), "{version:?}");
            }
        }
    }

    /// `new` already stores each player with his own id; the own-id override
    /// in `write_records` is reached only by an opened texport edited through
    /// `players_mut`, where `new`'s own-id copy never ran. PES 17 to 21.
    #[test]
    fn a_stripped_player_edited_into_an_opened_texport_is_written_with_his_own_id() {
        let pes17 = crate::fixtures::pes17_texport();
        let files = TEDS
            .into_iter()
            .chain([(PesVersion::Pes17, pes17.as_slice())]);
        for (version, bytes) in files {
            let mut texport = Texport::from_bytes(bytes, None).expect("opens");
            for player in texport.players_mut() {
                // The fixture holds each player's own id, not -1: the edit
                // below is a real change.
                assert_ne!(
                    player.appearance_id,
                    Some(STRIPPED_APPEARANCE_ID),
                    "{version:?}"
                );
                player.appearance_id = Some(STRIPPED_APPEARANCE_ID);
            }
            let out = texport.to_bytes(&test_salt()).expect("encodes");
            let back = Texport::from_bytes(&out, Some(version)).expect("reopens");
            for player in back.players() {
                assert_eq!(player.appearance_id, Some(player.id), "{version:?}");
            }
        }
    }

    #[test]
    fn new_synthesizes_a_file_the_records_round_trip_through() {
        for (version, bytes) in TEDS {
            let fixture = Texport::from_bytes(bytes, None).expect("opens");
            let players: Vec<&PlayerEntry> = fixture.players().iter().collect();
            let (records, player_records) = file_records(&fixture);
            // The fixture's own key, so the plaintexts compare byte for byte.
            let mut salt = test_salt();
            salt[..32].copy_from_slice(&bytes[KEY..HEADER]);
            let made = Texport::new(
                version,
                fixture.team(),
                &records,
                &paired(&players, &player_records),
                &salt,
                &[],
            )
            .expect("synthesizes");
            let out = made.to_bytes(&test_salt()).expect("encodes");
            let layout = texport_layout(version).expect("a layout");
            assert_eq!(out.len(), layout.size, "{version:?}");
            let back = Texport::from_bytes(&out, Some(version)).expect("reopens");
            assert_eq!(back.team(), fixture.team(), "{version:?} team");
            // The real files hold each player's own id at the appearance-id
            // slot, as `new` writes it.
            assert_eq!(back.players(), fixture.players(), "{version:?} players");

            // The non-record regions equal the real file's: the header and
            // key, and the tail. The records, written over the fixture's
            // own, equal them byte for byte, the coach block included.
            let made_plain = crypt(&out, layout);
            let fixture_plain = crypt(bytes, layout);
            let schema = schema_for(version);
            let off = Offsets::of(layout, schema);
            assert_eq!(
                &made_plain[..HEADER],
                &fixture_plain[..HEADER],
                "{version:?} header and key"
            );
            let mut regions = vec![
                (off.team, schema.team.size),
                (off.coach, COACH_RECORD_SIZE),
                (off.roster, schema.roster.size),
                (off.tactics, schema.tactic.size),
            ];
            regions.extend(
                fixture
                    .team()
                    .roster
                    .iter()
                    .enumerate()
                    .filter(|(_, slot)| slot.player_id != 0)
                    .map(|(slot, _)| (off.players + slot * schema.player.size, schema.player.size)),
            );
            for (at, size) in regions {
                assert_eq!(
                    first_difference(&made_plain[at..at + size], &fixture_plain[at..at + size]),
                    None,
                    "{version:?}: the record at {at:#x}"
                );
            }
            // The 660-byte block is written zero, as the game writes it for
            // the 18, 19 and 21 fixtures' teams; the PES 20 fixture's holds
            // five 125-byte runs, so the fixture's block is not compared.
            let block = off.tactics + schema_for(version).tactic.size;
            assert!(
                made_plain[block..block + BLOCK].iter().all(|&b| b == 0),
                "{version:?} made block"
            );
            assert_eq!(
                &made_plain[layout.size - TAIL..],
                &fixture_plain[layout.size - TAIL..],
                "{version:?} tail"
            );
        }
    }

    #[test]
    fn new_refuses_players_out_of_roster_order() {
        let (_, bytes) = TEDS[1];
        let fixture = Texport::from_bytes(bytes, None).expect("opens");
        let mut players: Vec<&PlayerEntry> = fixture.players().iter().collect();
        players.swap(0, 1);
        let (records, player_records) = file_records(&fixture);
        let err = Texport::new(
            PesVersion::Pes19,
            fixture.team(),
            &records,
            &paired(&players, &player_records),
            &test_salt(),
            &[],
        )
        .expect_err("order is the roster's");
        assert!(
            matches!(err, TexportError::RosterMismatch { slot: 0, .. }),
            "{err:?}"
        );
    }

    /// `slot_records` refuses a roster slot whose record holds another
    /// id: here the first two filled slots' player records trade places
    /// in the file itself, so `from_bytes` is the one refusing — on the
    /// 18-21 path (the `.ted` plaintext's record space) and on the 15-17
    /// one (the container's payload).
    #[test]
    fn slot_records_refuses_a_record_of_another_player() {
        for (version, bytes) in TEDS {
            let fixture = Texport::from_bytes(bytes, None).expect("opens");
            let layout = texport_layout(version).expect("a layout");
            let off = Offsets::of(layout, schema_for(version));
            let filled: Vec<usize> = fixture
                .team()
                .roster
                .iter()
                .enumerate()
                .filter(|(_, slot)| slot.player_id != 0)
                .map(|(i, _)| i)
                .collect();
            let (s0, s1) = (filled[0], filled[1]);
            let mut plain = crypt(bytes, layout);
            let size = schema_for(version).player.size;
            let (a, b) = (off.players + s0 * size, off.players + s1 * size);
            let (first, second) = (record_id(&plain[a..]), record_id(&plain[b..]));
            assert_eq!(first, fixture.team().roster[s0].player_id, "{version:?}");
            assert_eq!(second, fixture.team().roster[s1].player_id, "{version:?}");
            assert_ne!(first, second, "{version:?}: a real swap");
            let tmp = plain[a..a + size].to_vec();
            plain.copy_within(b..b + size, a);
            plain[b..b + size].copy_from_slice(&tmp);
            let corrupt = crypt(&plain, layout);
            let err = Texport::from_bytes(&corrupt, None).expect_err("refused");
            assert!(
                matches!(
                    err,
                    TexportError::RosterMismatch {
                        slot,
                        roster,
                        player
                    } if slot == s0 && roster == first && player == second
                ),
                "{version:?}: {err:?}"
            );
        }

        // The 15-17 reader: the same check, the records read out of the
        // container's payload a stride apart.
        let bytes = crate::fixtures::pes17_texport();
        let fixture = Texport::from_bytes(&bytes, None).expect("opens");
        let mut container = SaveContainer::decrypt(&bytes).expect("decrypts");
        let schema = schema_for(PesVersion::Pes17);
        let old = texport_old(PesVersion::Pes17).expect("a 15-17 layout");
        let stride = schema
            .appearance
            .as_ref()
            .map_or(schema.player.size, |(_, a)| schema.player.size + a.size);
        let filled: Vec<usize> = fixture
            .team()
            .roster
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.player_id != 0)
            .map(|(i, _)| i)
            .collect();
        let (s0, s1) = (filled[0], filled[1]);
        let (a, b) = (old.players_at + s0 * stride, old.players_at + s1 * stride);
        let (first, second) = (
            record_id(&container.payload[a..]),
            record_id(&container.payload[b..]),
        );
        assert_eq!(first, fixture.team().roster[s0].player_id);
        assert_eq!(second, fixture.team().roster[s1].player_id);
        assert_ne!(first, second, "a real swap");
        let tmp = container.payload[a..a + stride].to_vec();
        container.payload.copy_within(b..b + stride, a);
        container.payload[b..b + stride].copy_from_slice(&tmp);
        let corrupt = container.to_bytes(&test_salt()).expect("encodes");
        let err = Texport::from_bytes(&corrupt, None).expect_err("refused");
        assert!(
            matches!(
                err,
                TexportError::RosterMismatch {
                    slot,
                    roster,
                    player
                } if slot == s0 && roster == first && player == second
            ),
            "{err:?}"
        );
    }

    /// A filled roster slot with no player to pair is refused: the
    /// fixture's players less the last leaves its slot unmatched.
    #[test]
    fn new_refuses_a_missing_player() {
        for (version, bytes) in TEDS {
            let fixture = Texport::from_bytes(bytes, None).expect("opens");
            let slot = fixture
                .team()
                .roster
                .iter()
                .rposition(|slot| slot.player_id != 0)
                .expect("a filled slot");
            let roster = fixture.team().roster[slot].player_id;
            let players: Vec<&PlayerEntry> = fixture.players().iter().collect();
            assert_eq!(players.last().expect("nonempty").id, roster, "{version:?}");
            let (records, player_records) = file_records(&fixture);
            let err = Texport::new(
                version,
                fixture.team(),
                &records,
                &paired(&players[..players.len() - 1], &player_records),
                &test_salt(),
                &[],
            )
            .expect_err("a filled slot keeps its player");
            assert!(
                matches!(
                    err,
                    TexportError::RosterMismatch {
                        slot: s,
                        roster: r,
                        player: 0
                    } if s == slot && r == roster
                ),
                "{version:?}: {err:?}"
            );
        }
    }

    /// A player no filled slot pairs with is refused: the fixture's
    /// players plus the first again leaves one over at `width`.
    #[test]
    fn new_refuses_a_left_over_player() {
        for (version, bytes) in TEDS {
            let fixture = Texport::from_bytes(bytes, None).expect("opens");
            let mut players: Vec<&PlayerEntry> = fixture.players().iter().collect();
            players.push(players[0]);
            let (records, mut player_records) = file_records(&fixture);
            player_records.push(player_records[0]);
            let err = Texport::new(
                version,
                fixture.team(),
                &records,
                &paired(&players, &player_records),
                &test_salt(),
                &[],
            )
            .expect_err("a player has no slot");
            // `new` derives the roster width from the layout the same
            // way: the left-over refusal reports it as the slot.
            let off = Offsets::of(
                texport_layout(version).expect("a layout"),
                schema_for(version),
            );
            assert!(
                matches!(
                    err,
                    TexportError::RosterMismatch {
                        slot,
                        roster: 0,
                        player
                    } if slot == off.width && player == fixture.players()[0].id
                ),
                "{version:?}: {err:?}"
            );
        }
    }

    /// The 15-17 write path pairs the same way: an opened texport whose
    /// players lose the last is refused at his slot. (`players_mut` hands
    /// out a slice, which cannot shrink; the field edit is the same one
    /// the borrow would make.)
    #[test]
    fn to_bytes_refuses_a_missing_player() {
        let bytes = crate::fixtures::pes17_texport();
        let mut texport = Texport::from_bytes(&bytes, None).expect("opens");
        let slot = texport
            .team()
            .roster
            .iter()
            .rposition(|slot| slot.player_id != 0)
            .expect("a filled slot");
        let roster = texport.team().roster[slot].player_id;
        assert_eq!(texport.players().last().expect("nonempty").id, roster);
        texport.players.pop();
        let err = texport
            .to_bytes(&test_salt())
            .expect_err("a filled slot keeps its player");
        assert!(
            matches!(
                err,
                TexportError::RosterMismatch {
                    slot: s,
                    roster: r,
                    player: 0
                } if s == slot && r == roster
            ),
            "{err:?}"
        );
    }

    /// And a player more than the roster's filled slots is refused at the
    /// roster width.
    #[test]
    fn to_bytes_refuses_a_left_over_player() {
        let bytes = crate::fixtures::pes17_texport();
        let mut texport = Texport::from_bytes(&bytes, None).expect("opens");
        let extra = texport.players()[0].clone();
        let width = texport.team().roster.len();
        texport.players.push(extra.clone());
        let err = texport
            .to_bytes(&test_salt())
            .expect_err("a player has no slot");
        assert!(
            matches!(
                err,
                TexportError::RosterMismatch {
                    slot,
                    roster: 0,
                    player
                } if slot == width && player == extra.id
            ),
            "{err:?}"
        );
    }

    /// PES 2015 and 2016 are `NoTemplate` before any record is looked at,
    /// and `texport_writes` says so of them alone.
    #[test]
    fn new_has_no_template_for_15_and_16() {
        let bytes = crate::fixtures::pes17_texport();
        let fixture = Texport::from_bytes(&bytes, None).expect("opens");
        let players: Vec<&PlayerEntry> = fixture.players().iter().collect();
        let (records, player_records) = file_records(&fixture);
        let players = paired(&players, &player_records);
        for version in [PesVersion::Pes15, PesVersion::Pes16] {
            let made = Texport::new(
                version,
                fixture.team(),
                &records,
                &players,
                &test_salt(),
                &[],
            );
            assert!(
                matches!(made, Err(TexportError::NoTemplate(v)) if v == version),
                "{version:?}: {made:?}"
            );
        }
        let writes: Vec<PesVersion> = PesVersion::ALL
            .iter()
            .copied()
            .filter(|version| crate::schema::texport::texport_writes(*version))
            .collect();
        assert_eq!(
            writes,
            [
                PesVersion::Pes17,
                PesVersion::Pes18,
                PesVersion::Pes19,
                PesVersion::Pes20,
                PesVersion::Pes21
            ]
        );
    }

    /// A 15-17 *save* is refused at its description before a record is read;
    /// a texport-labelled container whose payload ends before the tactics
    /// record errors, never slices past the payload.
    #[test]
    fn a_save_shorter_than_the_old_offsets_is_an_error_not_a_panic() {
        let container = crate::test_support::fixture_container(PesVersion::Pes17);
        let bytes = container.to_bytes(&test_salt()).expect("encodes");
        let err = Texport::from_bytes(&bytes, None).expect_err("not a texport");
        assert!(matches!(err, TexportError::NotATexport), "{err:?}");

        // A valid container whose payload ends before the tactics record:
        // the same refusal, never a slice panic.
        let mut container = crate::test_support::fixture_container(PesVersion::Pes17);
        container.description[..16].copy_from_slice(b"Team Export Data");
        container.payload.truncate(0x10330);
        let bytes = container.to_bytes(&test_salt()).expect("encodes");
        let err = Texport::from_bytes(&bytes, None).expect_err("not a texport");
        assert!(
            matches!(err, TexportError::Codec(CodecError::RecordSize { .. })),
            "{err:?}"
        );
    }

    /// The PES 17 writer needs the payload to reach the last roster slot's
    /// player record: a payload ending exactly there writes, one byte
    /// shorter is `RecordSize` naming that end, not a slice panic.
    #[test]
    fn a_pes17_payload_must_reach_the_last_slots_player_record() {
        // Players at 0x510840, 32 roster slots of 188-byte player records:
        // 0x510840 + 32 * 188 = 0x510840 + 0x1780.
        const END: usize = 0x511FC0;
        let texport = Texport::from_bytes(&crate::fixtures::pes17_texport(), None).expect("opens");
        let File::Old(container) = &texport.file else {
            panic!("a PES 17 texport is a container");
        };
        assert!(
            container.payload.len() > END,
            "premise: the fixture is longer"
        );

        let mut exact = container.clone();
        exact.payload.truncate(END);
        texport
            .write_pes17(&mut exact)
            .expect("a payload that reaches the last record writes");

        let mut short = container.clone();
        short.payload.truncate(END - 1);
        let err = texport
            .write_pes17(&mut short)
            .expect_err("one byte short of the last record");
        assert!(
            matches!(
                err,
                TexportError::Codec(CodecError::RecordSize {
                    expected: END,
                    got,
                }) if got == END - 1
            ),
            "{err:?}"
        );
    }

    /// A save's description reads `Edit Data`, not `Team Export Data`: the
    /// PES 17 fixture save refuses on both entry paths (detected, and asked
    /// for by name).
    #[test]
    fn a_save_is_not_a_texport() {
        let container = crate::test_support::fixture_container(PesVersion::Pes17);
        let bytes = container.to_bytes(&test_salt()).expect("encodes");
        for version in [None, Some(PesVersion::Pes17)] {
            let err = Texport::from_bytes(&bytes, version).expect_err("a save");
            assert!(
                matches!(err, TexportError::NotATexport),
                "{version:?}: {err:?}"
            );
        }
    }

    /// PES 19's `.ted` and PES 17's container.
    #[test]
    fn edits_through_team_mut_and_players_mut_survive_the_round_trip() {
        let pes17 = crate::fixtures::pes17_texport();
        for bytes in [TEDS[1].1, pes17.as_slice()] {
            let original = Texport::from_bytes(bytes, None).expect("opens");
            let version = original.version();
            let mut t = Texport::from_bytes(bytes, None).expect("opens");
            assert_ne!(t.team().name, "EDITED", "{version:?}");
            assert_ne!(t.players()[0].stats.attacking_prowess, 99, "{version:?}");
            t.team_mut().name = "EDITED".to_string();
            t.players_mut()[0].stats.attacking_prowess = 99;
            let out = t.to_bytes(&test_salt()).expect("encodes");
            let back = Texport::from_bytes(&out, None).expect("reopens");
            assert_eq!(back.team().name, "EDITED", "{version:?}");
            assert_eq!(back.players()[0].stats.attacking_prowess, 99, "{version:?}");
            let mut expected_team = original.team().clone();
            expected_team.name = "EDITED".to_string();
            assert_eq!(back.team(), &expected_team, "{version:?}");
            let mut expected_players = original.players().to_vec();
            expected_players[0].stats.attacking_prowess = 99;
            assert_eq!(back.players(), &expected_players[..], "{version:?}");
        }
    }

    /// An opened PES 17 texport whose team is re-id'd and renamed through
    /// `team_mut` is written as `new` writes one: the description, the name
    /// the game lists, takes the new name, an old name's longer tail
    /// cleared, and the coach block stays the file's, the coach record the
    /// team record's manager id (not edited, so kept) still names. The
    /// fixture is team 736 `/o/`; /bant/ is team 708.
    #[test]
    fn a_pes17_team_edit_reaches_the_description_and_keeps_the_coach_block() {
        // The 480-byte team record at 0x10054, its manager id at +4, and
        // the coach block right after it.
        let manager_at = 0x10054 + 4;
        let coach = 0x10234..0x10234 + COACH_RECORD_SIZE;
        // The container's label, then `name` at 0x80, zeros to 384 bytes.
        let described = |name: &[u8]| {
            let mut description = b"Team Export Data 03".to_vec();
            description.resize(0x80, 0);
            description.extend_from_slice(name);
            description.resize(384, 0);
            description
        };
        let mut bytes = crate::fixtures::pes17_texport();
        let container = SaveContainer::decrypt(&bytes).expect("decrypts");
        let fixture_coach = container.payload[coach.clone()].to_vec();
        assert_eq!(record_id(&fixture_coach), 736, "the fixture's coach");
        assert_eq!(
            record_id(&container.payload[manager_at..]),
            736,
            "the fixture's manager id names its coach"
        );
        assert_eq!(container.description, described(b"/o/"), "the fixture's");
        // To /bant/ (708), then back to /o/ (736), three bytes shorter: a
        // NUL after the new name alone would leave "t/" of /bant/ behind.
        for (id, name) in [(708, "/bant/"), (736, "/o/")] {
            let mut texport = Texport::from_bytes(&bytes, None).expect("opens");
            assert_ne!(texport.team().id, id);
            texport.team_mut().id = id;
            texport.team_mut().name = name.to_string();
            bytes = texport.to_bytes(&test_salt()).expect("encodes");
            let container = SaveContainer::decrypt(&bytes).expect("decrypts");
            assert_eq!(
                container.payload[coach.clone()],
                fixture_coach[..],
                "{name}: the coach"
            );
            assert_eq!(
                record_id(&container.payload[manager_at..]),
                736,
                "{name}: the manager id"
            );
            assert_eq!(container.description, described(name.as_bytes()), "{name}");
            let back = Texport::from_bytes(&bytes, None).expect("reopens");
            assert_eq!((back.team().id, back.team().name.as_str()), (id, name));
        }
    }

    /// The size match alone does not make a texport: a 21-sized buffer whose
    /// decrypted record ids disagree is `UnknownVersion`.
    #[test]
    fn a_size_match_with_disagreeing_ids_is_unknown() {
        let (_, bytes) = TEDS[2];
        let layout = texport_layout(PesVersion::Pes21).expect("a layout");
        let mut plain = crypt(bytes, layout);
        // Corrupt the roster record's id so the three records disagree, then
        // re-encrypt under the 21 index.
        let off = Offsets::of(layout, schema_for(PesVersion::Pes21));
        plain[off.roster] ^= 0xff;
        let corrupt = crypt(&plain, layout);
        assert!(matches!(
            Texport::from_bytes(&corrupt, None),
            Err(TexportError::UnknownVersion)
        ));

        // Corrupting only the team record's id still refuses: roster and
        // tactics agree with each other, so agreement must be of all three.
        let mut plain = crypt(bytes, layout);
        plain[off.team] ^= 0xff;
        let corrupt = crypt(&plain, layout);
        assert!(matches!(
            Texport::from_bytes(&corrupt, None),
            Err(TexportError::UnknownVersion)
        ));
    }

    /// PES 20's layout is derived from its own record sizes (a 528-byte team
    /// record, not 21's 588), so a 21-sized file is `WrongSize` for it — the
    /// untested key index never gets to decode anything.
    #[test]
    fn pes20s_derived_size_refuses_the_21_fixture() {
        let (_, bytes) = TEDS[2];
        let err = Texport::from_bytes(bytes, Some(PesVersion::Pes20)).expect_err("refused");
        assert!(
            matches!(
                err,
                TexportError::WrongSize {
                    version: PesVersion::Pes20,
                    ..
                }
            ),
            "{err:?}"
        );
    }

    /// A 15-17 texport through `to_team_toml`: the `[team]` section and
    /// the roster's shirt numbers are the file's, and the document applies
    /// onto another team of the same version.
    #[test]
    fn a_pes17_texport_applies_through_team_toml() {
        let bytes = crate::fixtures::pes17_texport();
        let texport = Texport::from_bytes(&bytes, None).expect("opens");
        let doc = texport.to_team_toml().expect("to_team_toml");
        assert!(!texport.team().name.is_empty());
        assert_eq!(
            doc.team.name.as_deref(),
            Some(texport.team().name.as_str()),
            "the file carries its team metadata"
        );
        assert_eq!(doc.players.len(), texport.players().len());
        // The shirt numbers are the roster record's.
        for (key, section) in &doc.players {
            let index = usize::from(*key - 1);
            assert_eq!(
                section.number,
                Some(texport.team().roster[index].number),
                "slot {key}"
            );
        }

        // Another team of the PES 17 save with at least as many rostered
        // slots: the texport's 23 players land on its first 23.
        let (file, _) = open(PesVersion::Pes17);
        let team = file
            .teams()
            .iter()
            .find(|team| {
                team.id != texport.team().id
                    && team
                        .roster
                        .iter()
                        .filter(|slot| slot.player_id != 0)
                        .count()
                        >= doc.players.len()
            })
            .expect("a team with enough slots");
        let mut target = team.clone();
        let mut players = file.players().to_vec();
        let notes = doc
            .apply(PesVersion::Pes17, &mut target, &mut players)
            .expect("applies");
        assert!(notes.is_empty(), "{notes:?}");
        // The file's [team] applies as any other document's does.
        assert_eq!(target.name, texport.team().name);
        for key in doc.players.keys() {
            let index = usize::from(*key - 1);
            let id = target.roster[index].player_id;
            let applied = players.iter().find(|p| p.id == id).expect("rostered");
            let mut expected = texport.players()[index].clone();
            expected.id = id;
            // An import never sets the appearance id: the target's stands
            // (the save entry's own, not the texport player's).
            expected.appearance_id = file.player(id).expect("in the save").appearance_id;
            if expected.appearance.base_copy_id == texport.team().id * 100 + u32::from(*key) {
                // The own-id convention reads as "unset": the target's id.
                expected.appearance.base_copy_id = id;
            }
            // The run's unused bits are no key: the target keeps its own
            // (absent = untouched), so both sides compare at the games'
            // values.
            let mut applied = applied.clone();
            applied.appearance.ingame_face.write_unused();
            expected.appearance.ingame_face.write_unused();
            assert_eq!(applied, expected, "slot {key}");
            assert_eq!(
                target.roster[index].number,
                texport.team().roster[index].number,
                "shirt numbers are the file's"
            );
        }
    }

    /// The 15-17 writer lays each filled roster slot's player at its
    /// position and the reader checks the record's id against the slot's:
    /// a swapped order would write fine and reopen mismatched, so it is
    /// refused.
    #[test]
    fn an_old_layout_refuses_player_ids_that_do_not_match_the_slots() {
        let bytes = crate::fixtures::pes17_texport();
        let mut texport = Texport::from_bytes(&bytes, None).expect("opens");
        texport.players_mut().swap(0, 1);
        let err = texport
            .to_bytes(&test_salt())
            .expect_err("swapped players cannot be written positionally");
        assert!(
            matches!(err, TexportError::RosterMismatch { slot: 0, .. }),
            "{err:?}"
        );
    }

    /// A Texport of a team as the save holds it carries the save's own
    /// records byte for byte: the team record (its manager id at +4
    /// included), the coach record that id names, the roster, the tactics
    /// and every rostered player's record.
    #[test]
    fn an_unedited_teams_texport_carries_the_saves_records_byte_for_byte() {
        for version in [
            PesVersion::Pes17,
            PesVersion::Pes18,
            PesVersion::Pes19,
            PesVersion::Pes20,
            PesVersion::Pes21,
        ] {
            let (file, _) = open(version);
            let team = first_rostered_team(&file);
            let records = file.team_records(team.id).expect("the team's records");
            let coach = records.coach.expect("the team's coach record");
            let players = save_players(&file, team);
            let made = Texport::new(
                version,
                team,
                &records,
                &players,
                &test_salt(),
                file.serial(),
            )
            .expect("writes");
            let out = made.to_bytes(&test_salt()).expect("encodes");
            let (plain, off) = written_plain(version, &out, team.roster.len());
            let schema = schema_for(version);
            let region = |at: usize, size: usize| &plain[at..at + size];
            for (what, written, saved) in [
                ("team", region(off.team, schema.team.size), records.team),
                ("coach", region(off.coach, COACH_RECORD_SIZE), coach),
                (
                    "roster",
                    region(off.roster, schema.roster.size),
                    records.roster,
                ),
                (
                    "tactics",
                    region(off.tactics, schema.tactic.size),
                    records.tactics,
                ),
            ] {
                assert_eq!(
                    first_difference(written, saved),
                    None,
                    "{version:?} team {}: the {what} record",
                    team.id
                );
            }
            let slots = team
                .roster
                .iter()
                .enumerate()
                .filter(|(_, slot)| slot.player_id != 0);
            for ((slot, _), (player, saved)) in slots.zip(&players) {
                let written = region(off.players + slot * schema.player.size, schema.player.size);
                assert_eq!(
                    first_difference(written, saved),
                    None,
                    "{version:?} player {}",
                    player.id
                );
            }
        }
    }

    /// An edit to the team and to a player is written over the save's
    /// bytes: the file reads back with both, and the team record still
    /// carries the save's manager id.
    #[test]
    fn an_edit_is_written_over_the_saves_bytes() {
        for version in [PesVersion::Pes17, PesVersion::Pes21] {
            let (file, _) = open(version);
            let saved = first_rostered_team(&file);
            let records = file.team_records(saved.id).expect("the team's records");
            let manager = record_id(&records.team[4..]);
            assert_ne!(manager, 0, "{version:?}: premise, the save names a manager");
            let mut players = save_players(&file, saved);
            let mut team = saved.clone();
            let mut player = players[0].0.clone();
            assert_ne!(team.name, "EDITED", "{version:?}");
            assert_ne!(player.name, "EDITED PLAYER", "{version:?}");
            team.name = "EDITED".to_string();
            player.name = "EDITED PLAYER".to_string();
            players[0].0 = &player;
            let made = Texport::new(
                version,
                &team,
                &records,
                &players,
                &test_salt(),
                file.serial(),
            )
            .expect("writes");
            let out = made.to_bytes(&test_salt()).expect("encodes");
            let back = Texport::from_bytes(&out, None).expect("reopens");
            assert_eq!(back.team().name, "EDITED", "{version:?}");
            assert_eq!(back.players()[0].name, "EDITED PLAYER", "{version:?}");
            let (plain, off) = written_plain(version, &out, team.roster.len());
            assert_eq!(record_id(&plain[off.team + 4..]), manager, "{version:?}");
        }
    }

    /// A team whose manager id names no coach record of the save has no
    /// coach block to carry: `NoCoach` with the team and the manager id. A
    /// team record one byte short is `RecordSize`.
    #[test]
    fn a_team_whose_manager_names_no_coach_is_no_coach() {
        for version in [PesVersion::Pes17, PesVersion::Pes21] {
            let (file, _) = open(version);
            let team = first_rostered_team(&file);
            let records = file.team_records(team.id).expect("the team's records");
            let players = save_players(&file, team);
            // An edited manager id, which the error names, not the save's.
            let manager = 999_999;
            assert_ne!(record_id(&records.team[4..]), manager, "premise");
            let mut edited = team.clone();
            edited.manager_id = Some(manager);
            let no_coach = TeamRecords {
                coach: None,
                ..records
            };
            let err = Texport::new(version, &edited, &no_coach, &players, &test_salt(), &[])
                .expect_err("no coach record");
            assert!(
                matches!(
                    err,
                    TexportError::NoCoach { team: t, manager: m } if t == team.id && m == manager
                ),
                "{version:?}: {err:?}"
            );
            let size = schema_for(version).team.size;
            let short = TeamRecords {
                team: &records.team[..size - 1],
                ..records
            };
            let err = Texport::new(version, team, &short, &players, &test_salt(), &[])
                .expect_err("a short team record");
            assert!(
                matches!(
                    err,
                    TexportError::RecordSize {
                        record: "team",
                        expected,
                        actual,
                    } if expected == size && actual == size - 1
                ),
                "{version:?}: {err:?}"
            );
        }
    }

    /// A rostered player handed another player's save record is refused at
    /// his slot, naming the record's id.
    #[test]
    fn a_player_with_another_players_record_is_a_roster_mismatch() {
        for version in [PesVersion::Pes17, PesVersion::Pes21] {
            let (file, _) = open(version);
            let team = first_rostered_team(&file);
            let records = file.team_records(team.id).expect("the team's records");
            let mut players = save_players(&file, team);
            let (first, other) = (players[0].0.id, record_id(players[1].1));
            assert_ne!(first, other, "{version:?}: premise, two players");
            players[0].1 = players[1].1;
            let slot = team
                .roster
                .iter()
                .position(|slot| slot.player_id != 0)
                .expect("a filled slot");
            let err = Texport::new(version, team, &records, &players, &test_salt(), &[])
                .expect_err("another player's record");
            assert!(
                matches!(
                    err,
                    TexportError::RosterMismatch { slot: s, roster, player }
                        if s == slot && roster == first && player == other
                ),
                "{version:?}: {err:?}"
            );
        }
    }
}
