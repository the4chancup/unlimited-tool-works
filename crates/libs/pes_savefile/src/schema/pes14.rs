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
    fields: &[FieldSpec {
        field: PlayerField::Id,
        bit_offset: 0,
        bit_width: 32,
    }],
    arrays: &[],
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
