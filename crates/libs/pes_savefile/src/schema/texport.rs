//! The Texport layout tables. Two eras: the 18-21 `.ted` (a 0x50 header, a
//! 32-byte XOR key at 0x30, then the save's own records concatenated) and the
//! 15-17 `TEXPORT00000000` (a whole savefile container; the table names where
//! the team, roster, tactics and player records sit in its payload).
//!
//! Provenance: the 18-21 constants are measured on real files: four 18,
//! eleven 19, one 20 and five 21 texports. The 15-17 layout is measured on all 46 texports of the
//! reference machine (12 PES 16, 34 17; PES 15's is PES 16's, unverified), and PES
//! 2017's writer template on ten of its files.

use pes_version::PesVersion;

/// An 18-21 `.ted` layout. The record offsets are derived, not stored: the
/// team record sits at 0x50, the coach block after it, then roster, tactics,
/// a 660-byte block, `width` player records and the 12-byte tail.
pub struct TexportLayout {
    /// First key byte used.
    pub key_index: usize,
    /// Whole file.
    pub size: usize,
    /// The fixture's header, the writer's template.
    pub header: [u8; 0x30],
    /// The fixture's coach block minus the id, the writer's template.
    pub coach: [u8; 88],
    /// The file's last 12 bytes.
    pub tail: [u8; 12],
}

/// Where a 15-17 texport's records sit in the container's payload: the
/// tactics and player offsets measured, the team and roster offsets derived
/// from them and the save schema's record sizes.
pub(crate) struct TexportOld {
    /// Byte offset of the team record.
    pub team_at: usize,
    /// Byte offset of the roster record.
    pub roster_at: usize,
    /// Byte offset of the tactics record.
    pub tactics_at: usize,
    /// Byte offset of the first player record.
    pub players_at: usize,
}

/// `s` as a `[u8; N]`, so the measured hex strings stay verbatim.
const fn hex<const N: usize>(s: &str) -> [u8; N] {
    let bytes = s.as_bytes();
    assert!(bytes.len() == N * 2, "a hex pair per byte");
    let mut out = [0u8; N];
    let mut i = 0;
    while i < N {
        let hi = digit(bytes[2 * i]);
        let lo = digit(bytes[2 * i + 1]);
        out[i] = hi << 4 | lo;
        i += 1;
    }
    out
}

const fn digit(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => panic!("a lower-case hex digit"),
    }
}

/// The 88-byte coach block `team id (u32) | 4 constant bytes | flag |
/// "MANAGER" | zeros`, the id left zero (patched at write).
const fn coach(flag: u8) -> [u8; 88] {
    let mut out = [0u8; 88];
    out[4] = 0xe7;
    out[6] = 0xff;
    out[7] = 0xff;
    out[8] = flag;
    let mut i = 0;
    while i < 7 {
        out[9 + i] = b"MANAGER"[i];
        i += 1;
    }
    out
}

static PES18: TexportLayout = TexportLayout {
    key_index: 0x12,
    size: 0x1FC0,
    header: hex(
        "1700010050000000701f0000062900000000000000000000000000000000000038000000000000000000000000000000",
    ),
    coach: coach(0x61),
    tail: hex("000000000000000001000000"),
};

static PES19: TexportLayout = TexportLayout {
    key_index: 0x13,
    size: 0x25B0,
    header: hex(
        "170001005000000060250000682900000400000004000000000000000000000027000000000000000000000000000000",
    ),
    coach: coach(0x60),
    tail: hex("000000000400000001000000"),
};

/// PES 20, measured on one file (2026-10-12): its team record is 60 bytes
/// shorter than 21's, so its size is 0x39A8, not 21's 0x39E4, and its header's
/// u32 at 0x08 (the body length, size less 0x50, on every version) differs too.
static PES20: TexportLayout = TexportLayout {
    key_index: 0x14,
    size: 0x39A8,
    header: hex(
        "180001005000000058390000312a00000000000000000000000000000000000017000000000000000000000000000000",
    ),
    coach: coach(0x00),
    tail: hex("000000000000000001000000"),
};

static PES21: TexportLayout = TexportLayout {
    key_index: 0x15,
    size: 0x39E4,
    header: hex(
        "180001005000000094390000742700000000000000000000000000000000000019000000000000000000000000000000",
    ),
    coach: coach(0x00),
    tail: hex("000000000000000001000000"),
};

/// The PES 2017 `TEXPORT00000000` writer's template: what its payload holds
/// beside the team's records. Every other byte is written zero: the 32
/// bytes after the header differ in every real file and the game imports a
/// file with them zeroed, and the emblem block `0x50..0x10050` stays empty
/// because a save holds no emblem image.
pub(crate) struct TexportPes17 {
    /// The whole payload.
    pub size: usize,
    /// The payload's first 0x30 bytes.
    pub header: [u8; 0x30],
    /// Byte offset of the marker, right after the emblem block.
    pub marker_at: usize,
    /// The marker.
    pub marker: [u8; 4],
    /// The coach block, the team id at 0..4 left zero (patched at write).
    pub coach: [u8; 88],
    /// The payload's last 8 bytes.
    pub tail: [u8; 8],
}

/// Measured on ten real PES 2017 files. The coach block is the
/// variant real files pair with the header's `0x27d8` at 0x0C.
pub(crate) static PES17: TexportPes17 = TexportPes17 {
    size: 0x551FD0,
    header: hex(
        "1700000050000000801f5500d8270000040000000400000000000000000000000c000000000000000000000000000000",
    ),
    marker_at: 0x10050,
    marker: hex("01000000"),
    coach: pes17_coach(),
    tail: hex("0400000001000000"),
};

/// PES 2017's coach block: `00000000d700ffff614d616e61676572` (the id,
/// 4 constant bytes, the flag `0x61`, `Manager`), then 72 zero bytes.
const fn pes17_coach() -> [u8; 88] {
    let head: [u8; 16] = hex("00000000d700ffff614d616e61676572");
    let mut out = [0u8; 88];
    let mut i = 0;
    while i < head.len() {
        out[i] = head[i];
        i += 1;
    }
    out
}

/// Whether `Texport::new` writes a `version` file: PES 2017 from its
/// template, 2018 to 2021 from their layouts. PES 2015 and 2016 have no
/// template.
pub fn texport_writes(version: PesVersion) -> bool {
    match version {
        PesVersion::Pes17
        | PesVersion::Pes18
        | PesVersion::Pes19
        | PesVersion::Pes20
        | PesVersion::Pes21 => true,
        PesVersion::Pes15 | PesVersion::Pes16 => false,
    }
}

/// The 18-21 `.ted` layout; `None` on 15-17.
pub fn texport_layout(version: PesVersion) -> Option<&'static TexportLayout> {
    match version {
        PesVersion::Pes18 => Some(&PES18),
        PesVersion::Pes19 => Some(&PES19),
        PesVersion::Pes20 => Some(&PES20),
        PesVersion::Pes21 => Some(&PES21),
        PesVersion::Pes15 | PesVersion::Pes16 | PesVersion::Pes17 => None,
    }
}

/// The 15-17 payload offsets; `None` on 18-21. The team and roster offsets
/// are derived: the roster ends where the tactics record begins, the team
/// record ends where the roster begins — on 17 with the 88-byte coach
/// block between them.
pub(crate) fn texport_old(version: PesVersion) -> Option<TexportOld> {
    let (tactics_at, players_at, coach) = match version {
        PesVersion::Pes15 => (0x10298, 0x51065C, 0),
        PesVersion::Pes16 => (0x10298, 0x510660, 0),
        PesVersion::Pes17 => (0x10330, 0x510840, 88),
        PesVersion::Pes18 | PesVersion::Pes19 | PesVersion::Pes20 | PesVersion::Pes21 => {
            return None;
        }
    };
    let schema = super::schema_for(version);
    let roster_at = tactics_at - schema.roster.size;
    let team_at = roster_at - coach - schema.team.size;
    Some(TexportOld {
        team_at,
        roster_at,
        tactics_at,
        players_at,
    })
}
