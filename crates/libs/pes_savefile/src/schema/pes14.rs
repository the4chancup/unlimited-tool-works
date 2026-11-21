//! The PES 2014 tables, written by hand: the other versions' are derived mechanically by
//! `scripts/derive_savefile_schema.py` from measurements PES 2014 does not have. Every entry was
//! pinned from saves changed in game or read against the Akko VII saves; a field not listed here
//! is kept as read.

use pes_version::PesVersion;

use super::fields::{
    PlayerField, PlayerText, PresetField, RosterField, TacticsField, TeamField, TeamText,
};
use super::{
    ArraySpec, FieldSpec, FormationLayout, PresetSpec, RecordSchema, SectionLayout, SplitSpec,
    TacticsSchema, TextSpec, VersionSchema,
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
    split: &[],
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
    fields: &[
        FieldSpec {
            field: PlayerField::Id,
            bit_offset: 0,
            bit_width: 32,
        },
        // The boot code (run `appear1`).
        FieldSpec {
            field: PlayerField::BootsId,
            bit_offset: 64,
            bit_width: 14,
        },
        // The face's and hairstyle's edited flags (run `appear1`).
        FieldSpec {
            field: PlayerField::EditedFace,
            bit_offset: 95,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::EditedHair,
            bit_offset: 127,
            bit_width: 1,
        },
        // The 14 physique sliders, each the shown -7..+7 plus 7 (run
        // `appear1`), Arm Length before Arm Size unlike the other
        // versions' order.
        FieldSpec {
            field: PlayerField::NeckLength,
            bit_offset: 552,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::NeckSize,
            bit_offset: 556,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::ShoulderHeight,
            bit_offset: 560,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::ShoulderWidth,
            bit_offset: 564,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::Chest,
            bit_offset: 568,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::Waist,
            bit_offset: 572,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::ArmLength,
            bit_offset: 576,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::ArmSize,
            bit_offset: 580,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::Thigh,
            bit_offset: 584,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::Calf,
            bit_offset: 588,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::LegLength,
            bit_offset: 592,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::HeadLength,
            bit_offset: 596,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::HeadWidth,
            bit_offset: 600,
            bit_width: 4,
        },
        FieldSpec {
            field: PlayerField::HeadDepth,
            bit_offset: 604,
            bit_width: 4,
        },
        // The strip items (run `appear1`; the inners `appear2`'s): the
        // wristband, undershorts, sleeves, sock length and inners, the
        // spectacles' style, the outfielder gloves, the ankle tape and
        // the shirttail. Sleeves and the shirttail store in another order
        // than the model's (`stored_order` translates).
        FieldSpec {
            field: PlayerField::WristTaping,
            bit_offset: 661,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::Undershorts,
            bit_offset: 663,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Sleeves,
            bit_offset: 665,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::Socks,
            bit_offset: 667,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::Inners,
            bit_offset: 669,
            bit_width: 2,
        },
        FieldSpec {
            field: PlayerField::SpectaclesStyle,
            bit_offset: 678,
            bit_width: 3,
        },
        // The outfielder gloves' Off/Auto at 0x56:2 (run `appear1`).
        FieldSpec {
            field: PlayerField::PlayerGloves,
            bit_offset: 690,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::AnkleTaping,
            bit_offset: 691,
            bit_width: 1,
        },
        FieldSpec {
            field: PlayerField::Untucked,
            bit_offset: 692,
            bit_width: 1,
        },
    ],
    // The gloves id the Akko DLC reads from the hair accessory's colour
    // (the base game stores none): six bits each from the lowest, red
    // 0x13:0, blue 0x14:0, green 0x50:3 running into 0x51:0 — an 18-bit id
    // where the other versions hold 14.
    split: &[SplitSpec {
        field: PlayerField::GlovesId,
        runs: &[(0x13 * 8, 6), (0x14 * 8, 6), (0x50 * 8 + 3, 6)],
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
    split: &[],
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
    split: &[],
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

/// The PES 2014 tactics record (runs `tactics1`/`tactics2`, `tactics3` and `formations1`):
/// the set-piece takers, the captain, the players to join the attack, the auto switches,
/// the 32-entry lineup of roster slots, Combination Play's slots, and per preset the Game
/// Plan record's Strategy and eight sliders and one formation. The rest is kept as read.
pub(crate) static TACTICS: TacticsSchema = TacticsSchema {
    size: 502,
    fields: &[
        FieldSpec {
            field: TacticsField::TeamId,
            bit_offset: 0,
            bit_width: 32,
        },
        // The set-piece takers at +0x1C6 to +0x1CB, each a roster slot (run `tactics3`).
        FieldSpec {
            field: TacticsField::FreeKickTakerLong,
            bit_offset: 0x1C6 * 8,
            bit_width: 8,
        },
        FieldSpec {
            field: TacticsField::FreeKickTakerShort,
            bit_offset: 0x1C7 * 8,
            bit_width: 8,
        },
        FieldSpec {
            field: TacticsField::FreeKickTakerSecond,
            bit_offset: 0x1C8 * 8,
            bit_width: 8,
        },
        FieldSpec {
            field: TacticsField::CornerTakerLeft,
            bit_offset: 0x1C9 * 8,
            bit_width: 8,
        },
        FieldSpec {
            field: TacticsField::CornerTakerRight,
            bit_offset: 0x1CA * 8,
            bit_width: 8,
        },
        FieldSpec {
            field: TacticsField::PenaltyTaker,
            bit_offset: 0x1CB * 8,
            bit_width: 8,
        },
        // The captain at +0x1CC, a roster slot: moving the captain from #1 to #2 wrote 0 to 1
        // (run `tactics1`), and the captain stayed with his player when he swapped places on
        // the pitch (run `tactics2`).
        FieldSpec {
            field: TacticsField::Captain,
            bit_offset: 0x1CC * 8,
            bit_width: 8,
        },
        // The auto switches: Substitutions (OFF 0 to Very Early 3), then OFF 0 and ON 1 the
        // Offside Trap, Change Preset Tactics and Combination Play (run `tactics3`).
        FieldSpec {
            field: TacticsField::AutoSubstitution,
            bit_offset: 0x1D0 * 8,
            bit_width: 8,
        },
        FieldSpec {
            field: TacticsField::AutoOffsideTrap,
            bit_offset: 0x1D1 * 8,
            bit_width: 8,
        },
        FieldSpec {
            field: TacticsField::AutoPresetChange,
            bit_offset: 0x1D2 * 8,
            bit_width: 8,
        },
        FieldSpec {
            field: TacticsField::AutoCombinationPlay,
            bit_offset: 0x1D3 * 8,
            bit_width: 8,
        },
    ],
    arrays: &[
        // The lineup at +0x146: 32 u32s, entry k the roster slot playing position k (run
        // `tactics1`). The first 11 are the starting eleven, the next 21 the bench; each
        // field reads the u32's low byte.
        ArraySpec {
            make: TacticsField::Starting,
            count: 11,
            base_bit: 0x146 * 8,
            stride_bits: 32,
            bit_width: 8,
        },
        ArraySpec {
            make: TacticsField::Bench,
            count: 21,
            base_bit: 0x172 * 8,
            stride_bits: 32,
            bit_width: 8,
        },
        // The players to join the attack at +0x1CD, a roster slot each, 0xFF empty (run
        // `tactics3`).
        ArraySpec {
            make: TacticsField::PlayerToJoinAttack,
            count: 3,
            base_bit: 0x1CD * 8,
            stride_bits: 8,
            bit_width: 8,
        },
        // Combination Play's three slots from +0x1D6, two bytes each: the area, then the
        // strategy's id (run `tactics3`).
        ArraySpec {
            make: TacticsField::CombinationArea,
            count: 3,
            base_bit: 0x1D6 * 8,
            stride_bits: 16,
            bit_width: 8,
        },
        ArraySpec {
            make: TacticsField::CombinationStrategy,
            count: 3,
            base_bit: 0x1D7 * 8,
            stride_bits: 16,
            bit_width: 8,
        },
    ],
    // The preset blocks are 0x6c bytes apart (run `tactics3`).
    preset_stride_bits: 864,
    man_marking_bit: None,
    presets: &[
        // The Strategy dropdown's index at +0x5A (Custom 0 to All-out Attack 5), then the
        // eight Game Plan sliders at +0x5B in the panel's order (runs `tactics3`,
        // `formations1`).
        PresetSpec {
            field: PresetField::Strategy,
            bit_offset: 0x5A * 8,
        },
        PresetSpec {
            field: PresetField::GamePlanSlider(0),
            bit_offset: 0x5B * 8,
        },
        PresetSpec {
            field: PresetField::GamePlanSlider(1),
            bit_offset: 0x5C * 8,
        },
        PresetSpec {
            field: PresetField::GamePlanSlider(2),
            bit_offset: 0x5D * 8,
        },
        PresetSpec {
            field: PresetField::GamePlanSlider(3),
            bit_offset: 0x5E * 8,
        },
        PresetSpec {
            field: PresetField::GamePlanSlider(4),
            bit_offset: 0x5F * 8,
        },
        PresetSpec {
            field: PresetField::GamePlanSlider(5),
            bit_offset: 0x60 * 8,
        },
        PresetSpec {
            field: PresetField::GamePlanSlider(6),
            bit_offset: 0x61 * 8,
        },
        PresetSpec {
            field: PresetField::GamePlanSlider(7),
            bit_offset: 0x62 * 8,
        },
    ],
    // One formation per preset: the role bytes at +0x06, one every four, for slots 1 to 10
    // (the keeper, slot 0, has a place but no role byte), then the eleven place pairs from
    // +0x2E, each the depth's low byte then the width's (run `formations1`).
    formations: Some(FormationLayout {
        count: 1,
        first_role_slot: 1,
        base_bit: 0x02 * 8,
        slot_stride_bits: 32,
        y_offset_bits: 0x2C * 8,
        x_offset_bits: 0x2E * 8,
        pair_stride_bits: 32,
        formation_stride_bits: 0,
    }),
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
