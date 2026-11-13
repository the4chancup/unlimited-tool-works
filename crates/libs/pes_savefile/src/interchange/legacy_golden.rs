//! Parity of `interchange::legacy` with the reference editor's struct dumps.
//! The `.4ccs` record is `player_export` in MSVC layout; its offsets are held
//! here as literals (from the ctypes mirror in
//! `scripts/provenance/fixtures/interchange_fixtures.py`) and read raw from
//! the real fixture, then compared with what `read_squad` decodes. The
//! `.4cct` fixture is synthesized from the reference's write walk over the
//! PES 19 fixture save's team 713, so its evidence is agreement with the
//! schema codec's tactics for that team.

use pes_version::PesVersion;

use crate::interchange::legacy::{LegacyError, read_squad, read_tactics};
use crate::interchange::team_toml::TeamToml;
use crate::interchange::team_toml::player_keys::PlayerKey;
use crate::model::instruction::Instruction;
use crate::model::player::PlayerEntry;
use crate::model::team::TeamEntry;
use crate::schema::playstyle;
use crate::settings_toml::keys::SettingKey;
use crate::settings_toml::{FaceSettings, HairSettings, get_appearance};
use crate::test_support::open;

const SQUAD: &[u8] = include_bytes!("../../tests/fixtures/pes19_squad.4ccs");
const NIGHTLY: &[u8] = include_bytes!("../../tests/fixtures/pes19_tactics.4cct");

/// The `.4ccs` each version's fixture save exports for one of its teams —
/// `(version, bytes, team id)`: the parity evidence that this crate's field
/// values match the reference editor's for the same save.
const PARITY: &[(PesVersion, &[u8], u32)] = &[
    (
        PesVersion::Pes15,
        include_bytes!("../../tests/fixtures/pes15_parity.4ccs"),
        702,
    ),
    (
        PesVersion::Pes16,
        include_bytes!("../../tests/fixtures/pes16_parity.4ccs"),
        702,
    ),
    (
        PesVersion::Pes17,
        include_bytes!("../../tests/fixtures/pes17_parity.4ccs"),
        702,
    ),
    (
        PesVersion::Pes18,
        include_bytes!("../../tests/fixtures/pes18_parity.4ccs"),
        702,
    ),
    (
        PesVersion::Pes19,
        include_bytes!("../../tests/fixtures/pes19_parity.4ccs"),
        702,
    ),
    (
        PesVersion::Pes20,
        include_bytes!("../../tests/fixtures/pes20_parity.4ccs"),
        857,
    ),
    (
        PesVersion::Pes21,
        include_bytes!("../../tests/fixtures/pes21_parity.4ccs"),
        782,
    ),
];

/// The `.4ccs` `SQUAD` was exported from team 713 of the PES 19 fixture
/// save; the read is against that save's team and players.
fn squad_target(file: &crate::file::EditFile) -> (&TeamEntry, &[PlayerEntry]) {
    (
        file.team(713).expect("team 713 is in the PES 19 fixture"),
        file.players(),
    )
}

/// `"21a"`/`"20a"` + two version digits.
const HEADER: usize = 5;
/// `sizeof(player_export)`.
const RECORD: usize = 356;
/// `uint16_t numbers[40]`.
const NUMBERS: usize = 80;

// `player_export` field offsets (MSVC, 4-byte alignment, one padding byte at 271).
const NATION: usize = 0;
const HEIGHT: usize = 4;
const ATK: usize = 8;
const GK: usize = 10;
const B_EDIT_PLAYER: usize = 18;
const REG_POS: usize = 32;
const PLAY_STYLE: usize = 33;
const PLAY_SKILL: usize = 82;
const NAME: usize = 124;
const SHIRT_NAME: usize = 246;
const BOOT_ID: usize = 272;
const COPY_ID: usize = 280;
const NECK_LEN: usize = 284;
const SKIN_COL: usize = 353;
const IRIS_COL: usize = 354;

/// What the three sampled records hold (values printed by the fixture script).
struct Sample {
    index: usize,
    name: &'static str,
    shirt: &'static str,
    height: u8,
    atk: u8,
    gk: u8,
    reg_pos: u8,
    play_style: u8,
    skills: &'static [usize],
    boots: u32,
    copy: u32,
    skin: u8,
    iris: u8,
}

const SAMPLES: [Sample; 3] = [
    Sample {
        index: 0,
        name: "SHOTABOT 9S",
        shirt: "NINE ES",
        height: 189,
        atk: 77,
        gk: 77,
        reg_pos: 0,
        play_style: 20,
        skills: &[20],
        boots: 403,
        copy: 71301,
        skin: 1,
        iris: 1,
    },
    Sample {
        index: 7,
        name: "LONELY GAY NEET",
        shirt: "ALL \"BI\" MYSELF",
        height: 185,
        atk: 88,
        gk: 88,
        reg_pos: 5,
        play_style: 12,
        skills: &[7, 11, 12, 13, 16],
        boots: 0,
        copy: 71308,
        skin: 1,
        iris: 1,
    },
    Sample {
        index: 22,
        name: "wtf im gransexual now??",
        shirt: "(YOU)SEXUAL",
        height: 180,
        atk: 77,
        gk: 77,
        reg_pos: 5,
        play_style: 11,
        skills: &[12, 13],
        boots: 407,
        copy: 71323,
        skin: 2,
        iris: 3,
    },
];

fn raw(i: usize) -> &'static [u8] {
    &SQUAD[HEADER + i * RECORD..HEADER + (i + 1) * RECORD]
}

fn u32_at(rec: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(rec[at..at + 4].try_into().expect("four bytes"))
}

fn wide_string(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| u16::from_le_bytes(c))
        .take_while(|&u| u != 0)
        .collect();
    String::from_utf16(&units).expect("UTF-16")
}

fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8(bytes[..end].to_vec()).expect("ASCII")
}

#[test]
fn the_struct_dump_reads_at_the_literal_offsets() {
    assert_eq!(&SQUAD[..3], b"20a");
    assert_eq!(&SQUAD[3..5], b"19");
    assert_eq!(
        SQUAD.len(),
        HEADER + 23 * RECORD + NUMBERS,
        "no tactics block"
    );
    for s in &SAMPLES {
        let rec = raw(s.index);
        assert_eq!(u32_at(rec, NATION), 231, "[{}] nation", s.index);
        assert_eq!(rec[HEIGHT], s.height, "[{}] height", s.index);
        assert_eq!(rec[ATK], s.atk, "[{}] atk", s.index);
        assert_eq!(rec[GK], s.gk, "[{}] gk", s.index);
        assert_eq!(rec[REG_POS], s.reg_pos, "[{}] reg_pos", s.index);
        assert_eq!(rec[PLAY_STYLE], s.play_style, "[{}] play_style", s.index);
        let skills: Vec<usize> = (0..41).filter(|&k| rec[PLAY_SKILL + k] != 0).collect();
        assert_eq!(skills, s.skills, "[{}] skills", s.index);
        assert_eq!(
            wide_string(&rec[NAME..NAME + 122]),
            s.name,
            "[{}] name",
            s.index
        );
        assert_eq!(
            c_string(&rec[SHIRT_NAME..SHIRT_NAME + 21]),
            s.shirt,
            "[{}] shirt",
            s.index
        );
        assert_eq!(u32_at(rec, BOOT_ID), s.boots, "[{}] boots", s.index);
        assert_eq!(u32_at(rec, COPY_ID), s.copy, "[{}] copy", s.index);
        assert_eq!(u32_at(rec, NECK_LEN), 7, "[{}] neck_len", s.index);
        assert_eq!(rec[SKIN_COL], s.skin, "[{}] skin", s.index);
        assert_eq!(rec[IRIS_COL], s.iris, "[{}] iris", s.index);
    }
    let numbers = &SQUAD[HEADER + 23 * RECORD..];
    for i in 0..40 {
        let n = u16::from_le_bytes([numbers[2 * i], numbers[2 * i + 1]]);
        assert_eq!(n, if i < 23 { i as u16 + 1 } else { 0 }, "number {i}");
    }
}

#[test]
fn read_squad_decodes_the_struct_dump() {
    let (file, _) = open(PesVersion::Pes19);
    let (team, players) = squad_target(&file);
    // The record-i-to-slot-i+1 comparisons below stand only while team
    // 713's roster order is its record order.
    let positions: Vec<usize> = team
        .roster
        .iter()
        .filter(|slot| slot.player_id != 0)
        .map(|slot| {
            players
                .iter()
                .position(|p| p.id == slot.player_id)
                .expect("a rostered id is in the save")
        })
        .collect();
    assert!(
        positions.windows(2).all(|w| w[0] < w[1]),
        "team 713's roster order is its record order"
    );
    let doc = read_squad(SQUAD, team, players).expect("a valid .4ccs");
    assert_eq!(doc.pes_version, Some(PesVersion::Pes19));
    assert_eq!(doc.players.len(), 23);
    assert_eq!(
        doc.players.keys().copied().collect::<Vec<u8>>(),
        (1..=23).collect::<Vec<u8>>()
    );
    assert!(
        doc.tactics.starting_eleven.is_none(),
        "no tactics block in this file"
    );
    assert!(doc.team.name.is_none(), "a .4ccs carries no team entry");
    for s in &SAMPLES {
        let p = &doc.players[&(s.index as u8 + 1)];
        let rec = raw(s.index);
        assert_eq!(p.name.as_deref(), Some(s.name));
        assert_eq!(p.shirt_name.as_deref(), Some(s.shirt));
        assert_eq!(p.number, Some(s.index as u16 + 1));
        assert_eq!(p.nationality, Some(231));
        assert_eq!(p.stats.attacking_prowess, Some(s.atk));
        assert_eq!(p.stats.goalkeeping, Some(s.gk));
        assert_eq!(p.positions.registered, Some(s.reg_pos));
        assert_eq!(
            p.positions.playing_style,
            Some(playstyle::decode(PesVersion::Pes19, s.play_style).expect("listed"))
        );
        let mut skills = [false; 41];
        for &k in s.skills {
            skills[k] = true;
        }
        assert_eq!(p.skills.skills, Some(skills));
        assert_eq!(p.edit_flags.player, Some(rec[B_EDIT_PLAYER] != 0));
        assert_eq!(p.boots_id, Some(s.boots));
        assert_eq!(p.base_copy_id, Some(s.copy));
        assert_eq!(p.appearance.skin_color, Some(s.skin));
        assert_eq!(p.appearance.iris_color, Some(s.iris));
        assert_eq!(p.appearance.physique.neck_length, Some(7));
        assert_eq!(p.height, Some(s.height));
        assert_eq!(
            p.appearance.hair,
            HairSettings::default(),
            "the legacy record has no ingame-face run"
        );
        // Fields PES 19 lacks are absent whatever the bytes say.
        assert!(p.stats.tight_possession.is_none());
        assert!(p.positions.stronger_hand.is_none());
    }
}

/// Every `u8` field the reader maps to a `PlayerKey`, `(offset, key)`; the
/// offsets are the ctypes layout's, not `legacy.rs`'s constants, so a mapping
/// dropped or moved there fails here.
const U8_KEYS: &[(usize, PlayerKey)] = &[
    (4, PlayerKey::Height),
    (5, PlayerKey::Weight),
    (8, PlayerKey::AttackingProwess),
    (9, PlayerKey::DefensiveProwess),
    (10, PlayerKey::Goalkeeping),
    (11, PlayerKey::Dribbling),
    (13, PlayerKey::Finishing),
    (14, PlayerKey::LowPass),
    (15, PlayerKey::LoftedPass),
    (16, PlayerKey::Heading),
    (17, PlayerKey::Form),
    (19, PlayerKey::Swerve),
    (20, PlayerKey::Catching),
    (21, PlayerKey::Clearing),
    (22, PlayerKey::Reflexes),
    (23, PlayerKey::InjuryResistance),
    (25, PlayerKey::BodyControl),
    (26, PlayerKey::PhysicalContact),
    (27, PlayerKey::KickingPower),
    (28, PlayerKey::ExplosivePower),
    (31, PlayerKey::Age),
    (32, PlayerKey::Registered),
    (34, PlayerKey::BallControl),
    (35, PlayerKey::BallWinning),
    (36, PlayerKey::WeakFootAccuracy),
    (37, PlayerKey::Jump),
    (40, PlayerKey::Coverage),
    (41, PlayerKey::WeakFootUsage),
    (58, PlayerKey::PlaceKicking),
    (61, PlayerKey::TightPossession),
    (62, PlayerKey::Aggression),
    (67, PlayerKey::Stamina),
    (68, PlayerKey::Speed),
    (73, PlayerKey::StrongerFoot),
    (74, PlayerKey::StrongerHand),
];

/// The `bool` edit flags, `(offset, key)`.
const FLAG_KEYS: &[(usize, PlayerKey)] = &[
    (18, PlayerKey::EditedPlayer),
    (24, PlayerKey::EditedBasicSettings),
    (30, PlayerKey::EditedRegisteredPosition),
    (64, PlayerKey::EditedPlayablePositions),
    (65, PlayerKey::EditedAbilities),
    (66, PlayerKey::EditedSkills),
    (69, PlayerKey::EditedPlayingStyle),
    (70, PlayerKey::EditedComStyles),
    (71, PlayerKey::EditedMotion),
    (72, PlayerKey::EditedBaseCopy),
    (267, PlayerKey::EditedFace),
    (268, PlayerKey::EditedHair),
    (269, PlayerKey::EditedPhysique),
    (270, PlayerKey::EditedStrip),
];

/// The `u32` ids, `(offset, key)`.
const U32_KEYS: &[(usize, PlayerKey)] = &[
    (0, PlayerKey::Nationality),
    (272, PlayerKey::BootsId),
    (276, PlayerKey::GlovesId),
    (280, PlayerKey::BaseCopyId),
];

/// `u8` bytes that land in `appearance`, `(offset, key)`.
const APPEARANCE_U8_KEYS: &[(usize, SettingKey)] = &[
    (6, SettingKey::GoalCelebration1),
    (7, SettingKey::GoalCelebration2),
    (12, SettingKey::FreeKick),
    (29, SettingKey::ArmMovementDribbling),
    (38, SettingKey::ArmMovementRunning),
    (39, SettingKey::CornerKick),
    (55, SettingKey::HunchingDribbling),
    (56, SettingKey::HunchingRunning),
    (57, SettingKey::PenaltyKick),
    (59, SettingKey::Star),
    (60, SettingKey::Dribbling),
    (63, SettingKey::PlayingAttitude),
    (340, SettingKey::WristTapeColorLeft),
    (341, SettingKey::WristTapeColorRight),
    (342, SettingKey::WristTaping),
    (343, SettingKey::SpectaclesColor),
    (344, SettingKey::Spectacles),
    (345, SettingKey::Sleeves),
    (346, SettingKey::Inners),
    (347, SettingKey::Socks),
    (348, SettingKey::Undershorts),
    (352, SettingKey::GlovesColor),
    (353, SettingKey::SkinColor),
    (354, SettingKey::IrisColor),
];

/// `bool` bytes that land in `appearance.strip`, `(offset, key)`.
const APPEARANCE_BOOL_KEYS: &[(usize, SettingKey)] = &[
    (349, SettingKey::Untucked),
    (350, SettingKey::AnkleTaping),
    (351, SettingKey::Gloves),
];

/// The fourteen `i32` physique fields from offset 284, in struct order.
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

/// Mapped fields PES 19 does not store: three PES 20 added, and the base-copy
/// edit flag, which the 15-18 schemas hold and 19's does not. Absent whatever
/// the bytes hold. Dribbling motion and playing attitude (PES 20 too) are
/// `APPEARANCE_ABSENT_ON_19`.
const ABSENT_ON_19: &[PlayerKey] = &[
    PlayerKey::TightPossession,
    PlayerKey::StrongerHand,
    PlayerKey::Aggression,
    PlayerKey::EditedBaseCopy,
];
const APPEARANCE_ABSENT_ON_19: &[SettingKey] =
    &[SettingKey::Dribbling, SettingKey::PlayingAttitude];

#[test]
fn read_squad_decodes_every_record_and_every_mapped_field() {
    let (file, _) = open(PesVersion::Pes19);
    let (team, players) = squad_target(&file);
    let doc = read_squad(SQUAD, team, players).expect("a valid .4ccs");
    let numbers = &SQUAD[HEADER + 23 * RECORD..];
    for i in 0..23 {
        let p = &doc.players[&(i as u8 + 1)];
        let rec = raw(i);
        assert_eq!(
            p.name.as_deref(),
            Some(wide_string(&rec[NAME..NAME + 122]).as_str()),
            "[{i}] name"
        );
        assert_eq!(
            p.shirt_name.as_deref(),
            Some(c_string(&rec[SHIRT_NAME..SHIRT_NAME + 21]).as_str()),
            "[{i}] shirt"
        );
        assert_eq!(
            p.number,
            Some(u16::from_le_bytes([numbers[2 * i], numbers[2 * i + 1]])),
            "[{i}] number"
        );
        for &(at, key) in U8_KEYS {
            let expected = (!ABSENT_ON_19.contains(&key)).then_some(u32::from(rec[at]));
            assert_eq!(key.get_section(p), expected, "[{i}] {key:?} at {at}");
        }
        for &(at, key) in FLAG_KEYS {
            let expected = (!ABSENT_ON_19.contains(&key)).then_some(u32::from(rec[at] != 0));
            assert_eq!(key.get_section(p), expected, "[{i}] {key:?} at {at}");
        }
        for &(at, key) in U32_KEYS {
            assert_eq!(
                key.get_section(p),
                Some(u32_at(rec, at)),
                "[{i}] {key:?} at {at}"
            );
        }
        for &(at, key) in APPEARANCE_U8_KEYS {
            let expected = (!APPEARANCE_ABSENT_ON_19.contains(&key)).then_some(rec[at]);
            assert_eq!(
                get_appearance(&p.appearance, key),
                expected,
                "[{i}] {key:?} at {at}"
            );
        }
        for &(at, key) in APPEARANCE_BOOL_KEYS {
            assert_eq!(
                get_appearance(&p.appearance, key),
                Some(u8::from(rec[at] != 0)),
                "[{i}] {key:?} at {at}"
            );
        }
        for (k, &key) in PHYSIQUE_KEYS.iter().enumerate() {
            let at = 284 + 4 * k;
            let value = i32::from_le_bytes(rec[at..at + 4].try_into().expect("four bytes"));
            assert_eq!(
                get_appearance(&p.appearance, key),
                Some(u8::try_from(value).expect("the fixture's physique fits u8")),
                "[{i}] {key:?} at {at}"
            );
        }
        assert_eq!(
            p.positions.playable,
            Some(rec[42..55].try_into().expect("thirteen bytes")),
            "[{i}] playable"
        );
        assert_eq!(
            p.positions.playing_style,
            Some(playstyle::decode(PesVersion::Pes19, rec[33]).expect("listed")),
            "[{i}] style"
        );
        let mut skills = [false; 41];
        for (k, set) in skills.iter_mut().enumerate() {
            *set = rec[82 + k] != 0;
        }
        assert_eq!(p.skills.skills, Some(skills), "[{i}] skills");
        let mut com = [false; 7];
        for (k, set) in com.iter_mut().enumerate() {
            *set = rec[75 + k] != 0;
        }
        assert_eq!(p.skills.com_styles, Some(com), "[{i}] com styles");
        assert_eq!(
            p.appearance.hair,
            HairSettings::default(),
            "[{i}] no ingame-face run"
        );
    }
}

#[test]
fn read_tactics_equals_the_codec_for_the_same_team() {
    let doc = read_tactics(NIGHTLY).expect("a valid .4cct");
    assert_eq!(doc.pes_version, Some(PesVersion::Pes19));
    assert_eq!(doc.team.id, Some(713));
    assert!(doc.players.is_empty());

    let (file, _) = open(PesVersion::Pes19);
    let team = file.team(713).expect("team 713 is in the fixture save");
    let mut expected = TeamToml::from_team(PesVersion::Pes19, team, &[]).expect("dumps");
    // The nightly block has no captain and no man marking.
    expected.tactics.set_pieces.captain = None;
    for preset in &mut expected.tactics.presets {
        preset.man_marking = None;
    }
    assert_eq!(doc.tactics, expected.tactics);

    // Literals from the fixture script, so the equality above is not vacuous.
    assert_eq!(
        doc.tactics.starting_eleven,
        Some([0, 3, 2, 4, 1, 5, 6, 7, 8, 9, 10])
    );
    assert_eq!(doc.tactics.set_pieces.penalty, Some(10));
    assert_eq!(doc.tactics.set_pieces.captain, None);
    let preset = &doc.tactics.presets[2];
    let instructions = preset
        .instructions
        .as_ref()
        .expect("PES 19 has instructions");
    assert_eq!(
        instructions.defence[0].instruction,
        Instruction::SwarmTheBox
    );
    assert_eq!(instructions.defence[1].instruction, Instruction::Gegenpress);
    assert_eq!(instructions.attack[0].instruction, Instruction::Off);
    assert_eq!(preset.sliders.support_range, Some(3));
    assert_eq!(preset.style.buildup, Some(false));
    assert_eq!(preset.style.attacking_zone, Some(true));
}

#[test]
fn a_wrong_tag_or_version_is_refused() {
    let (file, _) = open(PesVersion::Pes19);
    let (team, players) = squad_target(&file);
    let err = read_squad(NIGHTLY, team, players).expect_err("a .4cct is not a .4ccs");
    assert!(matches!(err, LegacyError::BadTag { .. }), "{err:?}");
    let err = read_tactics(SQUAD).expect_err("a .4ccs is not a .4cct");
    assert!(matches!(err, LegacyError::BadTag { .. }), "{err:?}");
    let mut bad = SQUAD.to_vec();
    bad[3..5].copy_from_slice(b"1x");
    let err = read_squad(&bad, team, players).expect_err("version digits");
    assert!(matches!(err, LegacyError::BadVersion(_)), "{err:?}");
    let err = read_squad(&SQUAD[..HEADER + RECORD], team, players).expect_err("truncated");
    assert!(matches!(err, LegacyError::Truncated { .. }), "{err:?}");
}

/// Every value a `.4ccs` carries equals the codec's read of the same team:
/// each file was exported by 4ccEditor from the save the version's payload
/// fixture holds.
#[test]
fn each_versions_4ccs_equals_the_codec_read_of_its_team() {
    for &(version, bytes, team_id) in PARITY {
        let (file, _) = open(version);
        let team = file
            .team(team_id)
            .expect("the parity team is in the fixture save");
        let players = file.players();
        let doc = read_squad(bytes, team, players).expect("the .4ccs decodes");
        let pool: Vec<&PlayerEntry> = players.iter().collect();
        let mut ours = TeamToml::from_team(version, team, &pool).expect("the codec dumps the team");
        // The record carries no ingame-face run: no face or hair keys.
        for section in ours.players.values_mut() {
            section.appearance.face = FaceSettings::default();
            section.appearance.hair = HairSettings::default();
        }
        assert_eq!(doc.players.len(), 23, "{version:?}");
        assert_eq!(doc.players, ours.players, "{version:?}");
        assert_eq!(doc.pes_version, Some(version), "{version:?}");
    }
}
