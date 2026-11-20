//! The PES 2014 tables, written by hand: the other versions' are derived mechanically by
//! `scripts/derive_savefile_schema.py` from measurements PES 2014 does not have. Every entry was
//! pinned from saves changed in game or read against the Akko VII saves; a field not listed here
//! is kept as read.

use pes_version::PesVersion;

use super::fields::{PlayerField, PlayerText, RosterField, TacticsField, TeamField, TeamText};
use super::{
    ArraySpec, FieldSpec, RecordSchema, SectionLayout, TacticsSchema, TextSpec, VersionSchema,
};

/// The PES 2014 player record.
pub(crate) static PLAYER: RecordSchema<PlayerField, PlayerText> = RecordSchema {
    size: 108,
    fields: &[
        FieldSpec {
            field: PlayerField::Id,
            bit_offset: 0,
            bit_width: 32,
        },
        // The basics (run `player1`; `flags2` measured the age's six bits
        // and the height's stored value). The stats below store the shown
        // value less 40, the height less 132, the weight less 30
        // (`stored_offset`).
        FieldSpec {
            field: PlayerField::Nationality,
            bit_offset: 80,
            bit_width: 16,
        },
        FieldSpec {
            field: PlayerField::Height,
            bit_offset: 96,
            bit_width: 8,
        },
        FieldSpec {
            field: PlayerField::Weight,
            bit_offset: 104,
            bit_width: 8,
        },
        FieldSpec {
            field: PlayerField::GoalCelebration1,
            bit_offset: 112,
            bit_width: 8,
        },
        FieldSpec {
            field: PlayerField::GoalCelebration2,
            bit_offset: 120,
            bit_width: 8,
        },
        FieldSpec {
            field: PlayerField::Age,
            bit_offset: 128,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::Tenacity,
            bit_offset: 134,
            bit_width: 6,
        },
        // The stats, six bits each, and the position grades, two bits
        // each (C 0, B 1, A 2), interleaved in record order (run
        // `player1`).
        FieldSpec {
            field: PlayerField::AttackingProwess,
            bit_offset: 140,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::DefensiveProwess,
            bit_offset: 146,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::Goalkeeping,
            bit_offset: 152,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::PlayablePosition(0),
            bit_offset: 158,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::Dribbling,
            bit_offset: 160,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::BallControl,
            bit_offset: 166,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::Finishing,
            bit_offset: 172,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::LowPass,
            bit_offset: 178,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::LoftedPass,
            bit_offset: 184,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::PlayablePosition(1),
            bit_offset: 190,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::BallWinning,
            bit_offset: 192,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::PlaceKicking,
            bit_offset: 198,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::Swerve,
            bit_offset: 204,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::Catching,
            bit_offset: 210,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::Speed,
            bit_offset: 216,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::PlayablePosition(2),
            bit_offset: 222,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::KickingPower,
            bit_offset: 224,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::ExplosivePower,
            bit_offset: 230,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::Jump,
            bit_offset: 236,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::Stamina,
            bit_offset: 242,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::RegisteredPosition,
            bit_offset: 248,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::WeakFootAccuracy,
            bit_offset: 253,
            bit_width: 3,
        },
        // The motions and the 1-to-8 ratings, stored one below the shown
        // value (run `player1`; the dribbling motion from `flags1`).
        FieldSpec {
            field: PlayerField::FreeKickMotion,
            bit_offset: 256,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::PlayingStyle,
            bit_offset: 260,
            bit_width: 5,
        },
        FieldSpec {
            field: PlayerField::Form,
            bit_offset: 265,
            bit_width: 3,
        },
        FieldSpec {
            field: PlayerField::InjuryResistance,
            bit_offset: 268,
            bit_width: 3,
        },
        FieldSpec {
            field: PlayerField::WeakFootUsage,
            bit_offset: 271,
            bit_width: 3,
        },
        FieldSpec {
            field: PlayerField::PenaltyKickMotion,
            bit_offset: 277,
            bit_width: 3,
        },
        FieldSpec {
            field: PlayerField::PlayablePosition(3),
            bit_offset: 286,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::BodyControl,
            bit_offset: 288,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::Heading,
            bit_offset: 294,
            bit_width: 6,
        },
        FieldSpec {
            field: PlayerField::PlayablePosition(4),
            bit_offset: 300,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::PlayablePosition(5),
            bit_offset: 302,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::PlayablePosition(6),
            bit_offset: 304,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::PlayablePosition(7),
            bit_offset: 306,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::PlayablePosition(8),
            bit_offset: 308,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::PlayablePosition(9),
            bit_offset: 310,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::PlayablePosition(10),
            bit_offset: 312,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::PlayablePosition(11),
            bit_offset: 314,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::PlayablePosition(12),
            bit_offset: 316,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::DribblingMotion,
            bit_offset: 318,
            bit_width: 2,
        },
        // The punt-kick motion and the flag bits the game itself sets on
        // an edit (runs `player1`/`player2`; `flags1`/`flags2` for the
        // other two).
        FieldSpec {
            field: PlayerField::PuntKickMotion,
            bit_offset: 320,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::EditedName,
            bit_offset: 322,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::EditedBasicSettings,
            bit_offset: 326,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::EditedPlayer,
            bit_offset: 329,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::StrongerFoot,
            bit_offset: 336,
            bit_width: 1,
        },
        // The 20 skill cards, card `i` at bit 0x2b * 8 + i in the game's
        // screen order, each mapped to its canonical `Skill` slot (run
        // `player1`).
        FieldSpec {
            field: PlayerField::Skill(0),
            bit_offset: 344,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(1),
            bit_offset: 345,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(2),
            bit_offset: 346,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(3),
            bit_offset: 347,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(4),
            bit_offset: 348,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(7),
            bit_offset: 349,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(8),
            bit_offset: 350,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(9),
            bit_offset: 351,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(11),
            bit_offset: 352,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(12),
            bit_offset: 353,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(13),
            bit_offset: 354,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(14),
            bit_offset: 355,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(15),
            bit_offset: 356,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(18),
            bit_offset: 357,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(19),
            bit_offset: 358,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(20),
            bit_offset: 359,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(22),
            bit_offset: 360,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(23),
            bit_offset: 361,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(25),
            bit_offset: 362,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Skill(26),
            bit_offset: 363,
            bit_width: 1,
        },
    ],
    arrays: &[
        // The seven COM playing styles at 0x2A:1 to 0x2A:7 (run `player1`).
        ArraySpec {
            make: PlayerField::ComStyle,
            count: 7,
            base_bit: 337,
            stride_bits: 1,
            bit_width: 1,
        },
    ],
    texts: &[
        TextSpec {
            text: PlayerText::Name,
            byte_offset: 0x2e,
            len: 46,
        },
        // The Name Print text: set to `FLAG` in game, it landed here (run `flags1`).
        TextSpec {
            text: PlayerText::ShirtName,
            byte_offset: 0x5c,
            len: 16,
        },
    ],
    ingame_face: None,
};

/// The PES 2014 appearance record: only a player whose look was edited, or who was created in
/// the save, has one.
pub(crate) static APPEARANCE: RecordSchema<PlayerField, PlayerText> = RecordSchema {
    size: 88,
    fields: &[FieldSpec {
        field: PlayerField::Id,
        bit_offset: 0,
        bit_width: 32,
    }],
    arrays: &[],
    texts: &[],
    ingame_face: None,
};

/// The PES 2014 team record.
pub(crate) static TEAM: RecordSchema<TeamField, TeamText> = RecordSchema {
    size: 316,
    // The id is the low 16 bits of the u32 at +0; the high 16 are 0xffff.
    fields: &[FieldSpec {
        field: TeamField::Id,
        bit_offset: 0,
        bit_width: 16,
    }],
    arrays: &[],
    texts: &[
        TextSpec {
            text: TeamText::Name,
            byte_offset: 0x78,
            len: 70,
        },
        TextSpec {
            text: TeamText::ShortName,
            byte_offset: 0xbe,
            len: 4,
        },
    ],
    ingame_face: None,
};

/// The PES 2014 roster record, laid out as PES 2015's.
pub(crate) static ROSTER: RecordSchema<RosterField, TeamText> = RecordSchema {
    size: 164,
    fields: &[FieldSpec {
        field: RosterField::TeamId,
        bit_offset: 0,
        bit_width: 32,
    }],
    arrays: &[
        ArraySpec {
            make: RosterField::Player,
            count: 32,
            base_bit: 32,
            stride_bits: 32,
            bit_width: 32,
        },
        ArraySpec {
            make: RosterField::Number,
            count: 32,
            base_bit: 1056,
            stride_bits: 8,
            bit_width: 8,
        },
    ],
    texts: &[],
    ingame_face: None,
};

/// The PES 2014 tactics record. Only the team id and the captain are mapped: the record keeps
/// one formation per preset and a lineup of roster slots, which PES 2015's layouts do not
/// describe, so the rest is kept as read.
pub(crate) static TACTICS: TacticsSchema = TacticsSchema {
    size: 502,
    fields: &[
        FieldSpec {
            field: TacticsField::TeamId,
            bit_offset: 0,
            bit_width: 32,
        },
        // Byte +0x1cc, the captain's roster slot: moving the captain from #1 to #2 wrote 0 to 1
        // (run `tactics1`), and the captain stayed with his player when he swapped places on
        // the pitch (run `tactics2`).
        FieldSpec {
            field: TacticsField::Captain,
            bit_offset: 3680,
            bit_width: 8,
        },
    ],
    arrays: &[],
    // The preset blocks are 0x6c bytes apart; no preset setting is mapped yet.
    preset_stride_bits: 864,
    man_marking_bit: None,
    presets: &[],
    formations: None,
    instructions: None,
};

/// Where every section sits in the PES 2014 payload.
pub(crate) static PES14: VersionSchema = VersionSchema {
    version: PesVersion::Pes14,
    players: SectionLayout {
        offset: 0x4c,
        count_offset: 0x34,
    },
    player: &PLAYER,
    appearance: Some((
        SectionLayout {
            offset: 0x29332c,
            count_offset: 0x36,
        },
        &APPEARANCE,
    )),
    teams: SectionLayout {
        offset: 0x36a0ac,
        count_offset: 0x38,
    },
    team: &TEAM,
    coaches: None,
    // Rosters and tactics hold one record per team, in the team section's order.
    rosters: SectionLayout {
        offset: 0x3e5cd4,
        count_offset: 0x38,
    },
    roster: &ROSTER,
    tactics: SectionLayout {
        offset: 0x4081a0,
        count_offset: 0x38,
    },
    tactic: &TACTICS,
};
