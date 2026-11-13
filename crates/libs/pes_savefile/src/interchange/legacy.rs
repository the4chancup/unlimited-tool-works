//! The `.4ccs` squad dumps and `.4cct` "nightly" tactics files, read-only.
//! Both decode into a [`TeamToml`] so that [`TeamToml::apply`] is the only
//! code that writes interchange data into a save.
//!
//! A `.4ccs` record is the MSVC memory layout of a foreign player struct,
//! not a save record, so its offsets live here as `const`s and not in
//! `schema/`. They are mirrored from the ctypes table in
//! `scripts/provenance/fixtures/interchange_fixtures.py`; the field names in
//! the comments are that struct's.
//!
//! The record carries no player id, so a record's player is only its
//! position: the file lists one record per rostered player in the order the
//! source save's player records held them. At import time that order is
//! known only through the target save, so [`read_squad`] takes the target
//! team and its players and gives record *k* to the roster slot of the
//! *k*-th rostered player in the target's record order.

use std::collections::{HashMap, HashSet};

use pes_version::PesVersion;

use crate::codec::CodecError;
use crate::interchange::team_toml::player_keys::PlayerKey;
use crate::interchange::team_toml::{
    AutoSection, InstructionsSection, PlayerSection, SetPiecesSection, SlidersSection,
    StyleSection, TacticsSection, TeamToml, TeamTomlError, labels,
};
use crate::model::instruction::Instruction;
use crate::model::player::PlayerEntry;
use crate::model::tactics::FormationSlot;
use crate::model::team::TeamEntry;
use crate::schema::fields::{PlayerField, PresetField, TacticsField};
use crate::schema::{playstyle, schema_for};
use crate::settings_toml::keys::{SettingKey, Source};
use crate::settings_toml::set_appearance;

/// What a legacy file cannot be read as.
#[derive(Debug, thiserror::Error)]
pub enum LegacyError {
    /// The file's tag is not the format's.
    #[error("not a 4ccEditor file: it starts {found:?}, not {expected:?}")]
    BadTag {
        /// The tag the format carries.
        expected: &'static str,
        /// What the file starts with.
        found: String,
    },
    /// The two version digits are not a supported PES version.
    #[error("{0:?} is not a PES version this app reads (15 to 21)")]
    BadVersion(String),
    /// The 8-byte team id field is not ASCII decimal.
    #[error("team id {0:?} is not a number")]
    BadTeamId(String),
    /// The file ends mid-structure.
    #[error("file is {len} bytes; {needed} are needed")]
    Truncated {
        /// The smallest size that would parse.
        needed: usize,
        /// The file's size.
        len: usize,
    },
    /// A stored playing-style byte the exporting version's list does not hold.
    #[error("playing style {value} is not in {version}'s list")]
    UnknownPlayingStyle {
        /// The exporting version.
        version: PesVersion,
        /// The stored byte.
        value: u8,
    },
    /// An instruction byte outside the canonical table.
    #[error("instruction byte {0} is not an instruction this app knows")]
    BadInstruction(u8),
    /// More player records than the 40-slot roster the format implies.
    #[error("{count} player records; the format holds at most 40")]
    TooManyPlayers {
        /// The records the file claims to carry.
        count: usize,
    },
    /// More player records than the target team has rostered players.
    #[error(
        "{records} players in the file, but the selected team has only {rostered}: select a team with at least {records}"
    )]
    MoreRecordsThanRoster {
        /// The player records the file carries.
        records: usize,
        /// The non-empty slots of the target team's roster.
        rostered: usize,
    },
    /// A field value the field cannot hold.
    #[error(transparent)]
    Value(#[from] TeamTomlError),
    /// A name/shirt-name field that does not decode.
    #[error("text does not decode: {0}")]
    Text(String),
}

/// Tag + two version digits.
const HEADER: usize = 5;
/// `sizeof(player_export)`.
const RECORD: usize = 356;
/// The `[u16; 40]` shirt-number block after the records.
const NUMBERS: usize = 80;
/// The tactics block both formats carry.
const TACTICS: usize = 405;

// `player_export` field offsets (MSVC, 4-byte alignment, `bool` one byte,
// `wchar_t` two; one padding byte at 271 — mirrored from the fixture script's
// ctypes table).
/// player_export.nation
const NATION: usize = 0;
/// player_export.height
const HEIGHT: usize = 4;
/// player_export.weight
const WEIGHT: usize = 5;
/// player_export.gc1
const GC1: usize = 6;
/// player_export.gc2
const GC2: usize = 7;
/// player_export.mo_fk
const MO_FK: usize = 12;
/// player_export.b_edit_player
const B_EDIT_PLAYER: usize = 18;
/// player_export.b_edit_basicset
const B_EDIT_BASICSET: usize = 24;
/// player_export.mo_armd
const MO_ARMD: usize = 29;
/// player_export.b_edit_regpos
const B_EDIT_REGPOS: usize = 30;
/// player_export.age
const AGE: usize = 31;
/// player_export.reg_pos
const REG_POS: usize = 32;
/// player_export.play_style
const PLAY_STYLE: usize = 33;
/// player_export.mo_armr
const MO_ARMR: usize = 38;
/// player_export.mo_ck
const MO_CK: usize = 39;
/// player_export.play_pos[13]
const PLAY_POS: usize = 42;
/// player_export.mo_hunchd
const MO_HUNCHD: usize = 55;
/// player_export.mo_hunchr
const MO_HUNCHR: usize = 56;
/// player_export.mo_pk
const MO_PK: usize = 57;
/// player_export.mo_drib
const MO_DRIB: usize = 60;
/// player_export.b_edit_playpos
const B_EDIT_PLAYPOS: usize = 64;
/// player_export.b_edit_ability
const B_EDIT_ABILITY: usize = 65;
/// player_export.b_edit_skill
const B_EDIT_SKILL: usize = 66;
/// player_export.b_edit_style
const B_EDIT_STYLE: usize = 69;
/// player_export.b_edit_com
const B_EDIT_COM: usize = 70;
/// player_export.b_edit_motion
const B_EDIT_MOTION: usize = 71;
/// player_export.b_base_copy
const B_BASE_COPY: usize = 72;
/// player_export.strong_foot
const STRONG_FOOT: usize = 73;
/// player_export.strong_hand
const STRONG_HAND: usize = 74;
/// player_export.com_style[7]
const COM_STYLE: usize = 75;
/// player_export.play_skill[41]
const PLAY_SKILL: usize = 82;
/// player_export.name — wchar_t[61], UTF-16LE, NUL-terminated
const NAME: usize = 124;
/// player_export.name's byte length
const NAME_LEN: usize = 122;
/// player_export.shirt_name — char[21], single-byte, NUL-terminated
const SHIRT_NAME: usize = 246;
/// player_export.shirt_name's byte length
const SHIRT_NAME_LEN: usize = 21;
/// player_export.b_edit_face
const B_EDIT_FACE: usize = 267;
/// player_export.b_edit_hair
const B_EDIT_HAIR: usize = 268;
/// player_export.b_edit_phys
const B_EDIT_PHYS: usize = 269;
/// player_export.b_edit_strip
const B_EDIT_STRIP: usize = 270;
/// player_export.boot_id
const BOOT_ID: usize = 272;
/// player_export.glove_id
const GLOVE_ID: usize = 276;
/// player_export.copy_id
const COPY_ID: usize = 280;
/// player_export.neck_len … player_export.head_dep — fourteen i32s
const PHYSIQUE: usize = 284;
/// player_export.wrist_col_l
const WRIST_COL_L: usize = 340;
/// player_export.wrist_col_r
const WRIST_COL_R: usize = 341;
/// player_export.wrist_tape
const WRIST_TAPE: usize = 342;
/// player_export.spec_col
const SPEC_COL: usize = 343;
/// player_export.spec_style
const SPEC_STYLE: usize = 344;
/// player_export.sleeve
const SLEEVE: usize = 345;
/// player_export.inners
const INNERS: usize = 346;
/// player_export.socks
const SOCKS: usize = 347;
/// player_export.undershorts
const UNDERSHORTS: usize = 348;
/// player_export.untucked
const UNTUCKED: usize = 349;
/// player_export.ankle_tape
const ANKLE_TAPE: usize = 350;
/// player_export.gloves
const GLOVES: usize = 351;
/// player_export.gloves_col
const GLOVES_COL: usize = 352;
/// player_export.skin_col
const SKIN_COL: usize = 353;
/// player_export.iris_col
const IRIS_COL: usize = 354;

/// The byte stats and positions scalars that map one-to-one onto `PlayerKey`s,
/// `(offset, key)`. Every one is gated on the exporting version storing the
/// field and range-checked by `PlayerKey::set_section`.
const U8_SCALARS: &[(usize, PlayerKey)] = &[
    (HEIGHT, PlayerKey::Height),            // player_export.height
    (WEIGHT, PlayerKey::Weight),            // player_export.weight
    (8, PlayerKey::AttackingProwess),       // player_export.atk
    (9, PlayerKey::DefensiveProwess),       // player_export.def
    (10, PlayerKey::Goalkeeping),           // player_export.gk
    (11, PlayerKey::Dribbling),             // player_export.drib
    (13, PlayerKey::Finishing),             // player_export.finish
    (14, PlayerKey::LowPass),               // player_export.lowpass
    (15, PlayerKey::LoftedPass),            // player_export.loftpass
    (16, PlayerKey::Heading),               // player_export.header
    (17, PlayerKey::Form),                  // player_export.form
    (19, PlayerKey::Swerve),                // player_export.swerve
    (20, PlayerKey::Catching),              // player_export.catching
    (21, PlayerKey::Clearing),              // player_export.clearing
    (22, PlayerKey::Reflexes),              // player_export.reflex
    (23, PlayerKey::InjuryResistance),      // player_export.injury
    (25, PlayerKey::BodyControl),           // player_export.body_ctrl
    (26, PlayerKey::PhysicalContact),       // player_export.phys_cont
    (27, PlayerKey::KickingPower),          // player_export.kick_pwr
    (28, PlayerKey::ExplosivePower),        // player_export.exp_pwr
    (AGE, PlayerKey::Age),                  // player_export.age
    (REG_POS, PlayerKey::Registered),       // player_export.reg_pos
    (34, PlayerKey::BallControl),           // player_export.ball_ctrl
    (35, PlayerKey::BallWinning),           // player_export.ball_win
    (36, PlayerKey::WeakFootAccuracy),      // player_export.weak_acc
    (37, PlayerKey::Jump),                  // player_export.jump
    (40, PlayerKey::Coverage),              // player_export.cover
    (41, PlayerKey::WeakFootUsage),         // player_export.weak_use
    (58, PlayerKey::PlaceKicking),          // player_export.place_kick
    (61, PlayerKey::TightPossession),       // player_export.tight_pos
    (62, PlayerKey::Aggression),            // player_export.aggres
    (67, PlayerKey::Stamina),               // player_export.stamina
    (68, PlayerKey::Speed),                 // player_export.speed
    (STRONG_FOOT, PlayerKey::StrongerFoot), // player_export.strong_foot
    (STRONG_HAND, PlayerKey::StrongerHand), // player_export.strong_hand
];

/// The `bool` edit flags, `(offset, key)`.
const EDIT_FLAGS: &[(usize, PlayerKey)] = &[
    (B_EDIT_PLAYER, PlayerKey::EditedPlayer),
    (B_EDIT_BASICSET, PlayerKey::EditedBasicSettings),
    (B_EDIT_REGPOS, PlayerKey::EditedRegisteredPosition),
    (B_EDIT_PLAYPOS, PlayerKey::EditedPlayablePositions),
    (B_EDIT_ABILITY, PlayerKey::EditedAbilities),
    (B_EDIT_SKILL, PlayerKey::EditedSkills),
    (B_EDIT_STYLE, PlayerKey::EditedPlayingStyle),
    (B_EDIT_COM, PlayerKey::EditedComStyles),
    (B_EDIT_MOTION, PlayerKey::EditedMotion),
    (B_BASE_COPY, PlayerKey::EditedBaseCopy),
    (B_EDIT_FACE, PlayerKey::EditedFace),
    (B_EDIT_HAIR, PlayerKey::EditedHair),
    (B_EDIT_PHYS, PlayerKey::EditedPhysique),
    (B_EDIT_STRIP, PlayerKey::EditedStrip),
];

/// The `u32` ids, `(offset, key)`.
const U32_SCALARS: &[(usize, PlayerKey)] = &[
    (NATION, PlayerKey::Nationality),
    (BOOT_ID, PlayerKey::BootsId),
    (GLOVE_ID, PlayerKey::GlovesId),
    (COPY_ID, PlayerKey::BaseCopyId),
];

/// `u8` bytes that land in `appearance` (motion, strip and the face-run
/// colours), `(offset, key)`.
const APPEARANCE_U8: &[(usize, SettingKey)] = &[
    (GC1, SettingKey::GoalCelebration1),
    (GC2, SettingKey::GoalCelebration2),
    (MO_FK, SettingKey::FreeKick),
    (MO_ARMD, SettingKey::ArmMovementDribbling),
    (MO_ARMR, SettingKey::ArmMovementRunning),
    (MO_CK, SettingKey::CornerKick),
    (MO_HUNCHD, SettingKey::HunchingDribbling),
    (MO_HUNCHR, SettingKey::HunchingRunning),
    (MO_PK, SettingKey::PenaltyKick),
    (59, SettingKey::Star), // player_export.star
    (MO_DRIB, SettingKey::Dribbling),
    (63, SettingKey::PlayingAttitude), // player_export.play_attit
    (WRIST_COL_L, SettingKey::WristTapeColorLeft),
    (WRIST_COL_R, SettingKey::WristTapeColorRight),
    (WRIST_TAPE, SettingKey::WristTaping),
    (SPEC_COL, SettingKey::SpectaclesColor),
    (SPEC_STYLE, SettingKey::Spectacles),
    (SLEEVE, SettingKey::Sleeves),
    (INNERS, SettingKey::Inners),
    (SOCKS, SettingKey::Socks),
    (UNDERSHORTS, SettingKey::Undershorts),
    (GLOVES_COL, SettingKey::GlovesColor),
    (SKIN_COL, SettingKey::SkinColor),
    (IRIS_COL, SettingKey::IrisColor),
];

/// `bool` bytes that land in `appearance.strip`.
const APPEARANCE_BOOL: &[(usize, SettingKey)] = &[
    (UNTUCKED, SettingKey::Untucked),
    (ANKLE_TAPE, SettingKey::AnkleTaping),
    (GLOVES, SettingKey::Gloves),
];

/// The fourteen `i32` physique fields, in struct order from `PHYSIQUE`.
const PHYSIQUE_KEYS: &[SettingKey] = &[
    SettingKey::NeckLength,
    SettingKey::NeckSize,
    SettingKey::ShoulderHeight,
    SettingKey::ShoulderWidth,
    SettingKey::Chest,
    SettingKey::Waist,
    SettingKey::ArmSize,
    SettingKey::ArmLength,
    SettingKey::Thigh,
    SettingKey::Calf,
    SettingKey::LegLength,
    SettingKey::HeadLength,
    SettingKey::HeadWidth,
    SettingKey::HeadDepth,
];

fn u32_at(record: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(record[at..at + 4].try_into().expect("four bytes"))
}

fn i32_at(record: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(record[at..at + 4].try_into().expect("four bytes"))
}

/// `wchar_t[61]`, UTF-16LE, NUL-terminated.
fn wide_string(bytes: &[u8]) -> Result<String, LegacyError> {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_le_bytes(pair))
        .take_while(|&u| u != 0)
        .collect();
    String::from_utf16(&units).map_err(|e| LegacyError::Text(e.to_string()))
}

/// `char[21]`, the codec's single-byte text (byte b is char U+00b),
/// NUL-terminated.
fn single_byte_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    bytes[..end].iter().map(|&b| char::from(b)).collect()
}

/// The two version digits at `bytes[3..5]` → `PesVersion`; anything that is
/// not `15`..=`21` is `BadVersion`.
fn version_digits(bytes: &[u8]) -> Result<PesVersion, LegacyError> {
    let digits = bytes
        .get(3..5)
        .and_then(|d| std::str::from_utf8(d).ok())
        .unwrap_or("??");
    let number = digits
        .parse::<u16>()
        .map_err(|_| LegacyError::BadVersion(digits.to_string()))?;
    PesVersion::from_number(number).ok_or_else(|| LegacyError::BadVersion(digits.to_string()))
}

/// One 356-byte `player_export` record → a `PlayerSection`. Fields the
/// exporting version does not store stay `None` whatever the bytes hold — the
/// writer fills some of them with stale values.
fn read_player(
    version: PesVersion,
    fields: &HashSet<PlayerField>,
    record: &[u8],
    number: Option<u16>,
) -> Result<PlayerSection, LegacyError> {
    debug_assert_eq!(record.len(), RECORD);
    let mut section = PlayerSection {
        name: Some(wide_string(&record[NAME..NAME + NAME_LEN])?),
        shirt_name: Some(single_byte_string(
            &record[SHIRT_NAME..SHIRT_NAME + SHIRT_NAME_LEN],
        )),
        number,
        ..PlayerSection::default()
    };
    for &(offset, key) in U8_SCALARS.iter().chain(EDIT_FLAGS) {
        if fields.contains(&key.spec().field) {
            key.set_section(&mut section, u32::from(record[offset]))?;
        }
    }
    for &(offset, key) in U32_SCALARS {
        if fields.contains(&key.spec().field) {
            key.set_section(&mut section, u32_at(record, offset))?;
        }
    }
    // The editor's `play_pos` is the save's playable array verbatim, in the
    // model's order (CF at 0 to GK at 12), no remap.
    section.positions.playable = Some(
        record[PLAY_POS..PLAY_POS + 13]
            .try_into()
            .expect("thirteen bytes"),
    );
    section.positions.playing_style = Some(
        playstyle::decode(version, record[PLAY_STYLE]).map_err(|e| match e {
            CodecError::UnknownPlayingStyle { version, value } => {
                LegacyError::UnknownPlayingStyle { version, value }
            }
            other => LegacyError::from(TeamTomlError::from(other)),
        })?,
    );
    let mut skills = [false; 41];
    for (set, &byte) in skills.iter_mut().zip(&record[PLAY_SKILL..PLAY_SKILL + 41]) {
        *set = byte != 0;
    }
    section.skills.skills = Some(skills);
    let mut com_styles = [false; 7];
    for (set, &byte) in com_styles.iter_mut().zip(&record[COM_STYLE..COM_STYLE + 7]) {
        *set = byte != 0;
    }
    section.skills.com_styles = Some(com_styles);
    for &(offset, key) in APPEARANCE_U8 {
        put_appearance(&mut section, fields, key, u32::from(record[offset]))?;
    }
    for &(offset, key) in APPEARANCE_BOOL {
        put_appearance(&mut section, fields, key, u32::from(record[offset] != 0))?;
    }
    for (i, &key) in PHYSIQUE_KEYS.iter().enumerate() {
        let value = i32_at(record, PHYSIQUE + 4 * i);
        let value = u8::try_from(value).map_err(|_| TeamTomlError::OutOfRange {
            key: format!("appearance key {}", key.spec().name),
            value: i64::from(value),
            range: "0 to 255".to_string(),
        })?;
        put_appearance(&mut section, fields, key, u32::from(value))?;
    }
    Ok(section)
}

/// Write `value` into `section.appearance` under `key` when the exporting
/// version stores the key's field (`Source::Face` fields live in the ingame
/// face run, which every version has).
fn put_appearance(
    section: &mut PlayerSection,
    fields: &HashSet<PlayerField>,
    key: SettingKey,
    value: u32,
) -> Result<(), LegacyError> {
    let stored = match key.source() {
        Source::Player(field) => fields.contains(&field),
        // A legacy export carries none of the layout-only keys.
        Source::Face(_) | Source::NoseAngle => true,
    };
    if stored {
        let value = u8::try_from(value).map_err(|_| TeamTomlError::OutOfRange {
            key: key.spec().name.to_string(),
            value: i64::from(value),
            range: "0 to 255".to_string(),
        })?;
        set_appearance(&mut section.appearance, key, Some(value));
    }
    Ok(())
}

/// The 405-byte tactics block both formats carry, into `tactics`, read in
/// file order. Fields the exporting version's tactics schema does not store
/// are left `None` — the block carries their bytes anyway on every version.
fn tactics_block(block: &[u8], version: PesVersion) -> Result<TacticsSection, LegacyError> {
    debug_assert_eq!(block.len(), TACTICS);
    let tactic = schema_for(version).tactic;
    let mut pos = 0;
    let mut tactics = TacticsSection::default();
    for preset in &mut tactics.presets {
        for formation in &mut preset.formations {
            let positions = &block[pos..pos + 11];
            let mut slots = [FormationSlot::default(); 11];
            pos += 11;
            for (k, slot) in slots.iter_mut().enumerate() {
                // The block is y, then x.
                *slot = FormationSlot {
                    position: positions[k],
                    y: block[pos],
                    x: block[pos + 1],
                };
                pos += 2;
            }
            *formation = Some(slots);
        }
        let style = &block[pos..pos + 7];
        preset.style = StyleSection {
            attacking_style: Some(style[0] != 0),
            buildup: Some(style[1] != 0),
            attacking_zone: Some(style[2] != 0),
            positioning: Some(style[3] != 0),
            defensive_style: Some(style[4] != 0),
            containment_area: Some(style[5] != 0),
            pressure: Some(style[6] != 0),
            ..StyleSection::default()
        };
        pos += 7;
        // Two attack then two defence instructions, canonical byte + player.
        let mut instructions = InstructionsSection::default();
        for entry in instructions
            .attack
            .iter_mut()
            .chain(&mut instructions.defence)
        {
            let byte = block[pos];
            entry.instruction = *Instruction::ALL
                .get(usize::from(byte))
                .ok_or(LegacyError::BadInstruction(byte))?;
            entry.player = block[pos + 1];
            pos += 2;
        }
        preset.instructions = tactic.instructions.is_some().then_some(instructions);
        let sliders = &block[pos..pos + 5];
        preset.sliders = SlidersSection {
            support_range: Some(sliders[0]),
            numbers_in_attack: Some(sliders[1]),
            defensive_line: Some(sliders[2]),
            compactness: Some(sliders[3]),
            numbers_in_defence: Some(sliders[4]),
        };
        pos += 5;
        preset.style.fluid = tactic
            .has_preset(PresetField::FluidFormation)
            .then(|| block[pos] != 0);
        pos += 1;
    }
    tactics.starting_eleven = Some(block[pos..pos + 11].try_into().expect("eleven"));
    pos += 11;
    // The document's bench is the first twelve of the block's
    // twenty-one; `apply` fills the other nine as the game does.
    tactics.bench_order = Some(block[pos..pos + 12].try_into().expect("twelve"));
    pos += 21;
    // FK long, FK short, FK 2, CK left, CK right, PK; no captain in the block.
    let takers = &block[pos..pos + 6];
    tactics.set_pieces = SetPiecesSection {
        free_kick_long: Some(takers[0]),
        free_kick_short: Some(takers[1]),
        free_kick_second: Some(takers[2]),
        corner_left: Some(takers[3]),
        corner_right: Some(takers[4]),
        penalty: Some(takers[5]),
        captain: None,
    };
    pos += 6;
    tactics.players_to_join_attack = Some(block[pos..pos + 3].try_into().expect("three"));
    pos += 3;
    tactics.auto = AutoSection {
        substitution: tactic
            .has(TacticsField::AutoSubstitution)
            .then_some(block[pos]),
        offside_trap: tactic
            .has(TacticsField::AutoOffsideTrap)
            .then(|| block[pos + 1] != 0),
        preset_change: tactic
            .has(TacticsField::AutoPresetChange)
            .then(|| block[pos + 2] != 0),
        attack_defence_levels: tactic
            .has(TacticsField::AutoAttackDefenceLevels)
            .then(|| block[pos + 3] != 0),
    };
    pos += 4;
    debug_assert_eq!(pos, TACTICS);
    Ok(tactics)
}

/// A `.4ccs` squad file: `"20a"`/`"21a"`, two version digits, `n` player
/// records, the shirt-number block, an optional tactics block.
///
/// Read against `team` (the roster to land on) and `players` (its save's
/// player records, in record order), as the module doc explains: record *k*
/// goes to the slot of the *k*-th rostered player in `players` order, shirt
/// number *i* to slot *i*. A rostered id absent from `players` is
/// `PlayerMissing`; more records than rostered players is
/// `MoreRecordsThanRoster`.
pub fn read_squad(
    bytes: &[u8],
    team: &TeamEntry,
    players: &[PlayerEntry],
) -> Result<TeamToml, LegacyError> {
    if bytes.len() < HEADER + NUMBERS {
        return Err(LegacyError::Truncated {
            needed: HEADER + NUMBERS,
            len: bytes.len(),
        });
    }
    let tag = &bytes[..3];
    if tag != b"21a" && tag != b"20a" {
        return Err(LegacyError::BadTag {
            expected: "21a",
            found: String::from_utf8_lossy(tag).into_owned(),
        });
    }
    let version = version_digits(bytes)?;
    // Layout: header | n * RECORD | numbers | optional TACTICS. The block is
    // longer than a record, so test it by subtraction, not by remainder.
    let body = bytes.len() - HEADER - NUMBERS;
    let (records, block) = if body.is_multiple_of(RECORD) {
        (body / RECORD, None)
    } else if body >= TACTICS && (body - TACTICS).is_multiple_of(RECORD) {
        (
            (body - TACTICS) / RECORD,
            Some(&bytes[bytes.len() - TACTICS..]),
        )
    } else {
        // The tail is shorter than a record, so shorter than the block too:
        // report it as a truncated tactics block.
        let needed = bytes.len() - body % RECORD + TACTICS;
        return Err(LegacyError::Truncated {
            needed,
            len: bytes.len(),
        });
    };
    // The numbers block holds 40 slots; more records have no shirt numbers
    // and no roster slot to land in, so the file is refused outright.
    if records > 40 {
        return Err(LegacyError::TooManyPlayers { count: records });
    }
    // The target's non-empty slots as (1-based slot, record index of its
    // player), sorted by that index: record k lands on `rostered[k]`'s slot.
    let position_of: HashMap<u32, usize> = players
        .iter()
        .enumerate()
        .map(|(i, player)| (player.id, i))
        .collect();
    let mut rostered: Vec<(u8, usize)> = Vec::new();
    for (slot, entry) in team.roster.iter().enumerate() {
        if entry.player_id == 0 {
            continue;
        }
        let slot = u8::try_from(slot + 1).expect("a roster slot index fits u8");
        let &index = position_of
            .get(&entry.player_id)
            .ok_or(TeamTomlError::PlayerMissing {
                id: entry.player_id,
            })?;
        rostered.push((slot, index));
    }
    rostered.sort_by_key(|&(_, index)| index);
    if records > rostered.len() {
        return Err(LegacyError::MoreRecordsThanRoster {
            records,
            rostered: rostered.len(),
        });
    }
    let fields = schema_for(version).player_field_set();
    let numbers = &bytes[HEADER + records * RECORD..HEADER + records * RECORD + NUMBERS];
    let mut out = TeamToml {
        pes_version: Some(version),
        ..TeamToml::default()
    };
    for (i, &(slot, _)) in rostered.iter().enumerate().take(records) {
        let record = &bytes[HEADER + i * RECORD..HEADER + (i + 1) * RECORD];
        // A playable rating the label table does not hold would index-panic
        // when the document is emitted; refuse it at ingestion.
        for (j, &byte) in record[PLAY_POS..PLAY_POS + 13].iter().enumerate() {
            if usize::from(byte) >= labels::RATINGS.len() {
                return Err(TeamTomlError::OutOfRange {
                    key: format!("record {} offset {}", i + 1, PLAY_POS + j),
                    value: i64::from(byte),
                    range: format!("0 to {}", labels::RATINGS.len() - 1),
                }
                .into());
            }
        }
        // The numbers block stays in roster order: entry `slot - 1`.
        let at = usize::from(slot - 1);
        let number = Some(u16::from_le_bytes([numbers[2 * at], numbers[2 * at + 1]]));
        let section = read_player(version, &fields, record, number)?;
        out.players.insert(slot, section);
    }
    if let Some(block) = block {
        out.tactics = tactics_block(block, version)?;
    }
    Ok(out)
}

/// A `.4cct` "nightly" tactics file: `"001"`, two version digits, an 8-byte
/// ASCII team id, the 405-byte block.
pub fn read_tactics(bytes: &[u8]) -> Result<TeamToml, LegacyError> {
    const LENGTH: usize = 13 + TACTICS;
    if bytes.get(..3) != Some(b"001".as_slice()) {
        return Err(LegacyError::BadTag {
            expected: "001",
            found: String::from_utf8_lossy(bytes.get(..3).unwrap_or(b"")).into_owned(),
        });
    }
    if bytes.len() != LENGTH {
        return Err(LegacyError::Truncated {
            needed: LENGTH,
            len: bytes.len(),
        });
    }
    let version = version_digits(bytes)?;
    let id_bytes = &bytes[5..13];
    let end = id_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(id_bytes.len());
    let id_text = std::str::from_utf8(&id_bytes[..end]).unwrap_or("?");
    let team_id = id_text
        .parse::<u32>()
        .map_err(|_| LegacyError::BadTeamId(id_text.to_string()))?;
    let mut out = TeamToml {
        pes_version: Some(version),
        ..TeamToml::default()
    };
    out.team.id = Some(team_id);
    out.tactics = tactics_block(&bytes[13..], version)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::team::RosterSlot;
    use crate::test_support::open;

    const SQUAD: &[u8] = include_bytes!("../../tests/fixtures/pes19_squad.4ccs");
    const NIGHTLY: &[u8] = include_bytes!("../../tests/fixtures/pes19_tactics.4cct");
    const PARITY16: &[u8] = include_bytes!("../../tests/fixtures/pes16_parity.4ccs");

    /// The fixture `.4ccs` was exported from team 713 of the PES 19 fixture
    /// save; the read is against that save's team and players.
    fn squad_target(file: &crate::file::EditFile) -> (&TeamEntry, &[PlayerEntry]) {
        (
            file.team(713).expect("team 713 is in the PES 19 fixture"),
            file.players(),
        )
    }

    /// Team 702 of the PES 16 fixture, whose `.4ccs` is `PARITY16`.
    fn pes16_target(file: &crate::file::EditFile) -> (&TeamEntry, &[PlayerEntry]) {
        (
            file.team(702).expect("team 702 is in the PES 16 fixture"),
            file.players(),
        )
    }

    /// A `.4ccs` with the tactics block appended carries both halves.
    #[test]
    fn a_squad_with_a_tactics_block_decodes_both() {
        let (file, _) = open(PesVersion::Pes19);
        let (team, players) = squad_target(&file);
        let mut combined = SQUAD.to_vec();
        combined.extend_from_slice(&NIGHTLY[13..]);
        let doc = read_squad(&combined, team, players).expect("a .4ccs with tactics");
        assert_eq!(
            doc.players,
            read_squad(SQUAD, team, players).expect("squad").players
        );
        assert_eq!(doc.tactics, read_tactics(NIGHTLY).expect("tactics").tactics);
    }

    /// `"20a"` and `"21a"` are the same record; the tag is the only difference.
    #[test]
    fn both_squad_tags_decode_identically() {
        let (file, _) = open(PesVersion::Pes19);
        let (team, players) = squad_target(&file);
        let mut tagged = SQUAD.to_vec();
        tagged[..3].copy_from_slice(b"21a");
        assert_eq!(
            read_squad(&tagged, team, players).expect("21a"),
            read_squad(SQUAD, team, players).expect("20a")
        );
    }

    /// A PES 19 squad relabelled `16` holds style bytes 16's list does not
    /// have (16 is its hole, and it has only 19 entries), so the playing
    /// style is decoded with 16's list and the file is refused.
    #[test]
    fn a_16_label_on_a_file_with_an_unlisted_style_is_unknown_playing_style() {
        let (file, _) = open(PesVersion::Pes19);
        let (team, players) = squad_target(&file);
        let mut v16 = SQUAD.to_vec();
        v16[3..5].copy_from_slice(b"16");
        assert!(
            (0..23).any(|i| {
                playstyle::decode(PesVersion::Pes16, v16[HEADER + i * RECORD + PLAY_STYLE]).is_err()
            }),
            "premise: a style byte 16's list lacks"
        );
        let err = read_squad(&v16, team, players).expect_err("an unlisted style");
        assert!(
            matches!(
                err,
                LegacyError::UnknownPlayingStyle {
                    version: PesVersion::Pes16,
                    ..
                }
            ),
            "{err:?}"
        );
    }

    /// A PES 16 squad leaves the fields 16 does not store `None`, whatever
    /// its record bytes hold there, and reads the ones it does.
    #[test]
    fn a_pes16_squad_gates_the_fields_16_does_not_store() {
        let (file, _) = open(PesVersion::Pes16);
        let (team, players) = pes16_target(&file);
        let doc = read_squad(PARITY16, team, players).expect("a valid .4ccs");
        assert!(!doc.players.is_empty(), "premise: the squad has players");
        for (slot, section) in &doc.players {
            assert!(section.stats.physical_contact.is_none(), "slot {slot}");
            assert!(
                section.appearance.presentation.star.is_none(),
                "slot {slot}"
            );
            assert!(section.stats.tight_possession.is_none(), "slot {slot}");
            assert!(section.positions.stronger_hand.is_none(), "slot {slot}");
            assert!(section.appearance.motion.dribbling.is_none(), "slot {slot}");
            assert!(section.stats.attacking_prowess.is_some(), "slot {slot}");
        }
    }

    /// A 404-byte block is `Truncated`, not a short parse.
    #[test]
    fn a_short_tactics_block_is_truncated() {
        let err = read_tactics(&NIGHTLY[..NIGHTLY.len() - 1]).expect_err("one byte short");
        assert!(matches!(err, LegacyError::Truncated { .. }), "{err:?}");
    }

    /// An instruction's `player` is the byte after its `instruction` byte —
    /// `block[pos + 1]`, not the neighbouring byte at `pos - 1`.
    #[test]
    fn an_instructions_player_byte_follows_its_instruction() {
        // Preset 2's first defence instruction sits at block offsets
        // 120 + 106 + 4 (instruction) and +5 (player). NIGHTLY's player
        // bytes are all 0 — a `pos - 1` read lands on 0 there — so set the
        // player byte to a value the neighbouring bytes do not hold.
        let mut bytes = NIGHTLY.to_vec();
        bytes[13 + 120 + 106 + 5] = 7;
        let doc = read_tactics(&bytes).expect("NIGHTLY with a player byte set");
        let entry = doc.tactics.presets[1]
            .instructions
            .expect("instructions")
            .defence[0];
        assert_eq!(entry.player, 7);
    }

    /// A squad shorter than header + numbers (5 + 80) is `Truncated` to
    /// exactly that minimum; at the minimum the tag check is what refuses
    /// it.
    #[test]
    fn a_squad_shorter_than_the_minimum_reports_the_minimum() {
        let (file, _) = open(PesVersion::Pes19);
        let (team, players) = squad_target(&file);
        assert!(matches!(
            read_squad(&[0u8; 84], team, players),
            Err(LegacyError::Truncated {
                needed: 85,
                len: 84
            })
        ));
        assert!(matches!(
            read_squad(&[0u8; 85], team, players),
            Err(LegacyError::BadTag { .. })
        ));
    }

    /// A record count that leaves a partial tail is `Truncated` to the
    /// tactics block's size: `needed = len - rem + TACTICS`.
    #[test]
    fn a_short_squad_reports_the_bytes_it_needs() {
        let (file, _) = open(PesVersion::Pes19);
        let (team, players) = squad_target(&file);
        let cut = &SQUAD[..SQUAD.len() - 1];
        // 8187 body bytes = 23 * 356 + 355: the 355-byte tail is read as a
        // truncated tactics block, so 405 - 355 = 50 more bytes are needed.
        let err = read_squad(cut, team, players).expect_err("a byte short of a record");
        assert!(
            matches!(
                err,
                LegacyError::Truncated { needed, len } if needed == 8322 && len == 8272
            ),
            "{err:?}"
        );
    }

    /// An instruction byte the canonical table does not hold is BadInstruction.
    #[test]
    fn an_out_of_table_instruction_is_an_error() {
        // First instruction byte of preset 1: 3 formations * 33 + 7 style.
        let mut bad = NIGHTLY.to_vec();
        bad[13 + 106] = 0x20;
        assert!(matches!(
            read_tactics(&bad),
            Err(LegacyError::BadInstruction(0x20))
        ));
    }

    /// The end-to-end path: a `.4ccs` applies onto the fixture save's own team
    /// through `TeamToml::apply` and writes what the file says.
    #[test]
    fn a_squad_applies_to_the_fixture_team() {
        let (file, _) = open(PesVersion::Pes19);
        let (team, players_ref) = squad_target(&file);
        let doc = read_squad(SQUAD, team, players_ref).expect("a valid .4ccs");
        let mut target = team.clone();
        let mut players = file.players().to_vec();
        let notes = doc
            .apply(PesVersion::Pes19, &mut target, &mut players)
            .expect("same-version apply");
        assert!(notes.is_empty(), "{notes:?}");
        // Player 1's literals from the golden's sample table.
        let first = players.iter().find(|p| p.id == 71301).expect("applied");
        assert_eq!(first.name, "SHOTABOT 9S");
        assert_eq!(first.shirt_name, "NINE ES");
        assert_eq!(first.stats.attacking_prowess, 77);
        assert_eq!(first.stats.goalkeeping, 77);
        assert_eq!(first.positions.registered, 0);
        assert_eq!(first.basic.height, 189);
        assert_eq!(
            first
                .appearance
                .ingame_face
                .get(crate::schema::ingame_face::IngameFaceField::SkinColor)
                .ok(),
            Some(1)
        );
        let last = players.iter().find(|p| p.id == 71323).expect("applied");
        assert_eq!(last.name, "wtf im gransexual now??");
        assert_eq!(last.appearance.boots_id, 407);
        assert!(last.skills.skills[12] && last.skills.skills[13]);
    }

    /// The block's bytes exist on every version, but only fields the
    /// exporting version's tactics schema stores make the document.
    #[test]
    fn a_tactics_block_reports_only_what_the_exporting_version_stores() {
        // The 19 file first: instructions and the auto flags are all Some.
        let doc = read_tactics(NIGHTLY).expect("a .4cct");
        assert!(
            doc.tactics
                .presets
                .iter()
                .all(|preset| preset.instructions.is_some())
        );
        assert!(doc.tactics.auto.attack_defence_levels.is_some());

        let mut bytes = NIGHTLY.to_vec();
        bytes[3..5].copy_from_slice(b"16");
        let doc = read_tactics(&bytes).expect("a .4cct labelled 16");
        assert_eq!(doc.pes_version, Some(PesVersion::Pes16));
        for preset in &doc.tactics.presets {
            assert!(preset.instructions.is_none(), "16 has no instruction block");
            assert!(preset.style.fluid.is_some(), "16 has fluid");
        }
        assert!(doc.tactics.auto.attack_defence_levels.is_none());

        // And the gated document applies to a PES 16 team without error.
        let (file, _) = open(PesVersion::Pes16);
        let team = file
            .teams()
            .iter()
            .find(|team| team.roster.iter().any(|slot| slot.player_id != 0))
            .expect("a team");
        let mut target = team.clone();
        doc.apply(PesVersion::Pes16, &mut target, &mut [])
            .expect("applies to a PES 16 team");
    }

    /// Each of the block's last four bytes lands on its own auto field:
    /// substitution, offside trap, preset change, attack/defence levels.
    #[test]
    fn each_auto_byte_lands_on_its_own_field() {
        let mut bytes = NIGHTLY.to_vec();
        bytes[13 + TACTICS - 4..13 + TACTICS].copy_from_slice(&[0, 1, 0, 1]);
        let doc = read_tactics(&bytes).expect("a .4cct");
        assert_eq!(doc.tactics.auto.substitution, Some(0));
        assert_eq!(doc.tactics.auto.offside_trap, Some(true));
        assert_eq!(doc.tactics.auto.preset_change, Some(false));
        assert_eq!(doc.tactics.auto.attack_defence_levels, Some(true));
    }

    /// Exactly forty records — the roster's width — parse; the numbers
    /// block covers them all.
    #[test]
    fn a_squad_of_exactly_forty_players_parses() {
        let (file, _) = open(PesVersion::Pes19);
        let players = file.players();
        let mut target = file.team(713).expect("team 713").clone();
        target.roster = players
            .iter()
            .take(40)
            .map(|p| RosterSlot {
                player_id: p.id,
                number: 0,
            })
            .collect();
        let record = &SQUAD[HEADER..HEADER + RECORD];
        let mut bytes = b"21a19".to_vec();
        for _ in 0..40 {
            bytes.extend_from_slice(record);
        }
        bytes.extend_from_slice(&[0u8; NUMBERS]);
        let doc = read_squad(&bytes, &target, players).expect("forty players parse");
        assert_eq!(doc.players.len(), 40);
    }

    /// More records than the 40-slot roster is `TooManyPlayers`, never an
    /// index panic on the shirt-number block.
    #[test]
    fn a_squad_of_more_than_forty_players_is_refused() {
        let (file, _) = open(PesVersion::Pes19);
        let (team, players) = squad_target(&file);
        let record = &SQUAD[HEADER..HEADER + RECORD];
        let mut bytes = b"21a19".to_vec();
        for _ in 0..41 {
            bytes.extend_from_slice(record);
        }
        bytes.extend_from_slice(&[0u8; NUMBERS]);
        let err = read_squad(&bytes, team, players).expect_err("41 players");
        assert!(
            matches!(err, LegacyError::TooManyPlayers { count: 41 }),
            "{err:?}"
        );
    }

    /// A playable rating past the four labels would index-panic when the
    /// document is emitted; the reader refuses it.
    #[test]
    fn a_playable_rating_past_the_label_table_is_refused() {
        let (file, _) = open(PesVersion::Pes19);
        let (team, players) = squad_target(&file);
        let mut bytes = SQUAD.to_vec();
        bytes[HEADER + PLAY_POS] = 4;
        let err = read_squad(&bytes, team, players).expect_err("a rating of 4 has no label");
        assert!(
            matches!(
                err,
                LegacyError::Value(TeamTomlError::OutOfRange {
                    ref key,
                    value: 4,
                    ..
                }) if key.starts_with("record 1 offset")
            ),
            "{err:?}"
        );
    }

    /// A record lands on its own player's slot when the target's roster
    /// order differs from its record order; shirt numbers stay in roster
    /// order.
    #[test]
    fn records_follow_the_targets_record_order_not_its_roster_order() {
        let (file, _) = open(PesVersion::Pes16);
        let players = file.players();
        let mut target = file.team(702).expect("team 702").clone();
        // Slots 1 and 2 now hold each other's players; the records must
        // still land on their own player's slot.
        target.roster.swap(0, 1);
        let name_of = |id: u32| {
            players
                .iter()
                .find(|p| p.id == id)
                .expect("a rostered id is in the save")
                .name
                .clone()
        };
        let doc = read_squad(PARITY16, &target, players).expect("a valid .4ccs");
        assert_eq!(
            doc.players[&2].name.as_deref(),
            Some(name_of(70201).as_str())
        );
        assert_eq!(
            doc.players[&1].name.as_deref(),
            Some(name_of(70202).as_str())
        );
        for slot in 3..=23u8 {
            let id = target.roster[usize::from(slot) - 1].player_id;
            assert_eq!(
                doc.players[&slot].name.as_deref(),
                Some(name_of(id).as_str()),
                "slot {slot}"
            );
        }
        // The numbers block is roster order: slot 1 keeps its own entry.
        let numbers = &PARITY16[HEADER + 23 * RECORD..];
        let first = u16::from_le_bytes([numbers[0], numbers[1]]);
        assert_eq!(doc.players[&1].number, Some(first));
    }

    /// More records than the target roster holds is `MoreRecordsThanRoster`.
    #[test]
    fn more_records_than_the_roster_holds_is_refused() {
        let (file, _) = open(PesVersion::Pes16);
        let (team, players) = pes16_target(&file);
        let mut target = team.clone();
        target.roster[22].player_id = 0;
        let err = read_squad(PARITY16, &target, players).expect_err("23 records, 22 rostered");
        assert!(
            matches!(
                err,
                LegacyError::MoreRecordsThanRoster {
                    records: 23,
                    rostered: 22
                }
            ),
            "{err:?}"
        );
    }

    /// A rostered id the target save does not hold is `PlayerMissing`.
    #[test]
    fn a_rostered_id_absent_from_the_save_is_player_missing() {
        let (file, _) = open(PesVersion::Pes16);
        let (team, players) = pes16_target(&file);
        let mut target = team.clone();
        target.roster[0].player_id = u32::MAX;
        let err = read_squad(PARITY16, &target, players).expect_err("an id no player has");
        assert!(
            matches!(
                err,
                LegacyError::Value(TeamTomlError::PlayerMissing { id: u32::MAX })
            ),
            "{err:?}"
        );
    }

    /// Fewer records than rostered players fill only the first rostered
    /// players in record order, not in roster order; the rest get no section.
    #[test]
    fn fewer_records_fill_only_the_first_rostered_players() {
        let (file, _) = open(PesVersion::Pes16);
        let (team, players) = pes16_target(&file);
        // Slots 22 and 23 hold each other's players, so record order puts
        // slot 23's player 22nd and slot 22's player last.
        let mut target = team.clone();
        target.roster.swap(21, 22);
        let cut: Vec<u8> = PARITY16[..HEADER + 22 * RECORD]
            .iter()
            .chain(&PARITY16[HEADER + 23 * RECORD..HEADER + 23 * RECORD + NUMBERS])
            .copied()
            .collect();
        let doc = read_squad(&cut, &target, players).expect("22 records");
        assert_eq!(doc.players.len(), 22);
        assert!((1..=21u8).all(|slot| doc.players.contains_key(&slot)));
        assert!(doc.players.contains_key(&23));
        assert!(!doc.players.contains_key(&22));
    }
}
