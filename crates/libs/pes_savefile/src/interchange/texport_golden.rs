//! Parity of `interchange::texport` with the reference editor's Texport read
//! walk, held as the literals measured on the real files (four PES 18, eleven
//! 19, one 20 and five 21 texports; the fixtures are one of each, whole): the key
//! index, where the tactics and player records start, the roster width, and
//! what the records say. The 15-17 half opens through the container: the
//! PES 17 `TEXPORT00000000`'s payload is the fixture, sliced as a save's is.

use std::io::Read;

use pes_version::PesVersion;

use crate::codec::{read_player, read_player_into, read_tactics_into};
use crate::container::{MasterKey, SaveContainer, Scheme};
use crate::fixtures::{pes17_texport, pes17_texport_container, pes17_texport_payload};
use crate::interchange::texport::{Texport, TexportError};
use crate::model::names::display_name;
use crate::model::player::PlayerEntry;
use crate::model::team::TeamEntry;
use crate::schema::schema_for;
use crate::schema::texport::texport_old;
use crate::test_support::test_salt;

/// One measured file.
struct Layout {
    version: PesVersion,
    bytes: &'static [u8],
    /// First key byte the reference XORs with.
    key_index: usize,
    /// The reference's `tacticsPos`.
    tactics_at: usize,
    /// The reference's `playersPos`.
    players_at: usize,
    /// Player record slots the file has room for (the roster width).
    width: usize,
    team: u32,
    team_name: &'static str,
    short_name: &'static str,
    /// Players actually present (roster slots filled).
    filled: usize,
    first_player: &'static str,
    first_boots: u32,
}

const LAYOUTS: [Layout; 4] = [
    Layout {
        version: PesVersion::Pes18,
        bytes: include_bytes!("../../tests/fixtures/pes18_texport.ted"),
        key_index: 0x12,
        tactics_at: 0x32C,
        players_at: 0x834,
        width: 32,
        team: 736,
        team_name: "/o/",
        short_name: "O",
        filled: 23,
        first_player: "BLACK ICE TREE",
        first_boots: 38,
    },
    Layout {
        version: PesVersion::Pes19,
        bytes: include_bytes!("../../tests/fixtures/pes19_texport.ted"),
        key_index: 0x13,
        tactics_at: 0x33C,
        players_at: 0x844,
        width: 40,
        team: 841,
        team_name: "/98hu/",
        short_name: "I1",
        filled: 23,
        first_player: "YOUR SKILL: A FAILURE",
        first_boots: 3601,
    },
    // Exported by the game for this fixture (team /3/ of the cup's base DLC,
    // every player a placeholder); `first_boots` is the codec's read, the
    // record being PES 21's layout.
    Layout {
        version: PesVersion::Pes20,
        bytes: include_bytes!("../../tests/fixtures/pes20_texport.ted"),
        key_index: 0x14,
        tactics_at: 0x3D4,
        players_at: 0x8DC,
        width: 40,
        team: 701,
        team_name: "/3/",
        short_name: "3",
        filled: 23,
        first_player: "PLACEHOLDER",
        first_boots: 0,
    },
    Layout {
        version: PesVersion::Pes21,
        bytes: include_bytes!("../../tests/fixtures/pes21_texport.ted"),
        key_index: 0x15,
        tactics_at: 0x410,
        players_at: 0x918,
        width: 40,
        team: 841,
        team_name: "/98hu/",
        short_name: "2HU",
        filled: 23,
        first_player: "YOUR SKILL: A FAILURE",
        first_boots: 3601,
    },
];

/// The reference's loop: the body from 0x50 XOR'd with the 32-byte key at
/// 0x30, starting at `key_index` and wrapping.
fn reference_decrypt(bytes: &[u8], key_index: usize) -> Vec<u8> {
    let key = &bytes[0x30..0x50];
    let mut out = bytes.to_vec();
    let mut k = key_index;
    for b in out.iter_mut().skip(0x50) {
        *b ^= key[k];
        k = (k + 1) % 0x20;
    }
    out
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

#[test]
fn the_reference_walk_finds_the_records_at_the_literal_offsets() {
    for l in &LAYOUTS {
        let dec = reference_decrypt(l.bytes, l.key_index);
        assert_eq!(
            u32_at(&dec, l.tactics_at),
            l.team,
            "{:?} tactics id",
            l.version
        );
        let stride = (dec.len() - 12 - l.players_at) / l.width;
        assert_eq!(
            l.players_at + l.width * stride + 12,
            dec.len(),
            "{:?} width",
            l.version
        );
        let mut ids: Vec<u32> = (0..l.width)
            .map(|i| u32_at(&dec, l.players_at + i * stride))
            .collect();
        let empty = ids.split_off(l.filled);
        assert!(
            empty.iter().all(|&id| id == 0),
            "{:?} empty slots",
            l.version
        );
        let expected: Vec<u32> = (0..l.filled as u32).map(|i| l.team * 100 + 1 + i).collect();
        assert_eq!(ids, expected, "{:?} player ids", l.version);
    }
}

#[test]
fn from_bytes_decodes_what_the_reference_reads_and_detects_the_version() {
    for l in &LAYOUTS {
        let t = Texport::from_bytes(l.bytes, None).expect("opens");
        assert_eq!(t.version(), l.version);
        let team = t.team();
        assert_eq!(team.id, l.team);
        assert_eq!(display_name(&team.name), l.team_name);
        assert_eq!(team.short_name, l.short_name);
        assert_eq!(
            team.roster.iter().filter(|s| s.player_id != 0).count(),
            l.filled,
            "{:?} roster",
            l.version
        );
        let players = t.players();
        assert_eq!(players.len(), l.filled);
        for (i, p) in players.iter().enumerate() {
            assert_eq!(
                p.id,
                l.team * 100 + 1 + i as u32,
                "{:?} slot {i}",
                l.version
            );
            assert_eq!(
                p.id, team.roster[i].player_id,
                "{:?} roster slot {i}",
                l.version
            );
        }
        assert_eq!(display_name(&players[0].name), l.first_player);
        assert_eq!(players[0].appearance.boots_id, l.first_boots);
        // Every stat of every player is a real ability, not a misaligned read.
        for p in players {
            for v in [
                p.stats.attacking_prowess,
                p.stats.ball_control,
                p.stats.speed,
                p.stats.stamina,
                p.stats.goalkeeping,
            ] {
                assert!(
                    (40..=99).contains(&v),
                    "{:?} player {} stat {v}",
                    l.version,
                    p.id
                );
            }
        }
        let explicit = Texport::from_bytes(l.bytes, Some(l.version)).expect("opens");
        assert_eq!(explicit.team(), t.team());
        assert_eq!(explicit.players(), t.players());
    }
}

/// The schema codec applied at literal offsets: the oracle for what a
/// `Texport` must have decoded. `tactics_at` is the tactics record;
/// `players_at` the first player record, `stride` apart (player record, then
/// the appearance record on the versions that split it).
fn codec_reads(
    plain: &[u8],
    version: PesVersion,
    tactics_at: usize,
    players_at: usize,
    stride: usize,
    count: usize,
) -> (TeamEntry, Vec<PlayerEntry>) {
    let schema = schema_for(version);
    let mut team = TeamEntry {
        id: u32_at(plain, tactics_at),
        ..TeamEntry::default()
    };
    read_tactics_into(
        &mut team,
        &plain[tactics_at..tactics_at + schema.tactic.size],
        schema.tactic,
    )
    .expect("tactics decode");
    let players = (0..count)
        .map(|i| {
            let at = players_at + i * stride;
            let mut player =
                read_player(&plain[at..at + schema.player.size], schema.player).expect("decodes");
            if let Some((_, appearance)) = &schema.appearance {
                let at = at + schema.player.size;
                read_player_into(&mut player, &plain[at..at + appearance.size], appearance)
                    .expect("appearance decodes");
            }
            player
        })
        .collect();
    (team, players)
}

#[test]
fn every_record_decodes_to_what_the_codec_reads_at_the_literal_offsets() {
    for l in &LAYOUTS {
        let dec = reference_decrypt(l.bytes, l.key_index);
        let stride = (dec.len() - 12 - l.players_at) / l.width;
        let (team, players) = codec_reads(
            &dec,
            l.version,
            l.tactics_at,
            l.players_at,
            stride,
            l.filled,
        );
        let t = Texport::from_bytes(l.bytes, None).expect("opens");
        assert_eq!(t.team().tactics, team.tactics, "{:?} tactics", l.version);
        assert_eq!(t.players(), &players[..], "{:?} players", l.version);
        // Not vacuous: a real formation and a real ability behind the equality.
        assert!(
            team.tactics.presets[0].formations[0]
                .players
                .iter()
                .any(|s| s.x != 0 || s.y != 0),
            "{:?} formation",
            l.version
        );
        assert_eq!(players[0].appearance.boots_id, l.first_boots);
    }
}

#[test]
fn a_file_of_another_size_is_refused_for_the_named_version() {
    let err = Texport::from_bytes(LAYOUTS[1].bytes, Some(PesVersion::Pes18)).expect_err("refused");
    assert!(
        matches!(
            err,
            TexportError::WrongSize {
                version: PesVersion::Pes18,
                ..
            }
        ),
        "{err:?}"
    );
    let err = Texport::from_bytes(&LAYOUTS[0].bytes[..0x50], None).expect_err("refused");
    assert!(matches!(err, TexportError::UnknownVersion), "{err:?}");
}

#[test]
fn round_trip_is_byte_identical_on_every_fixture() {
    for l in &LAYOUTS {
        let t = Texport::from_bytes(l.bytes, None).expect("opens");
        let out = t.to_bytes(&test_salt()).expect("encodes");
        assert_eq!(out, l.bytes, "{:?} round trip", l.version);
    }
}

#[test]
fn pes17_texport_opens_through_the_container_and_round_trips() {
    let bytes = pes17_texport();
    let t = Texport::from_bytes(&bytes, None).expect("opens");
    assert_eq!(t.version(), PesVersion::Pes17);
    // The reference reads the tactics record at 0x10330 and the players at 0x510840.
    assert_eq!(t.team().id, 736);
    // The team record carries its metadata and the roster record the shirt
    // numbers, both read at the measured offsets.
    assert!(!t.team().name.is_empty(), "the team record's name");
    let doc = t.to_team_toml().expect("to_team_toml");
    for (key, section) in &doc.players {
        let index = usize::from(*key - 1);
        assert_eq!(
            section.number,
            Some(t.team().roster[index].number),
            "the roster's shirt number at slot {key}"
        );
    }
    let players = t.players();
    assert_eq!(players.len(), 23);
    let schema = schema_for(PesVersion::Pes17);
    let stride = schema
        .appearance
        .as_ref()
        .map_or(schema.player.size, |(_, a)| schema.player.size + a.size);
    let (team, expected) = codec_reads(
        &pes17_texport_payload(),
        PesVersion::Pes17,
        0x10330,
        0x510840,
        stride,
        23,
    );
    assert_eq!(t.team().tactics, team.tactics);
    assert_eq!(players, &expected[..]);
    assert!(
        team.tactics.presets[0].formations[0]
            .players
            .iter()
            .any(|s| s.x != 0 || s.y != 0),
        "a real formation"
    );
    assert_eq!(display_name(&players[0].name), "BLACK ICE TREE");
    assert_eq!(display_name(&players[1].name), "JAMES MAY");
    for (i, p) in players.iter().enumerate() {
        assert_eq!(p.id, 736 * 100 + 1 + i as u32);
    }
    assert_eq!(t.to_bytes(&test_salt()).expect("encodes"), bytes);
}

/// The 15-17 open's premise: the team, roster and tactics records carry
/// one team id in every measured file, so a file whose team record — or
/// roster record — alone names another is not laid out as a texport and
/// is refused (`IdsDisagree` reporting the three ids the file holds).
#[test]
fn a_pes17_texport_whose_team_or_roster_id_alone_disagrees_is_refused() {
    let old = texport_old(PesVersion::Pes17).expect("a 15-17 layout");
    for (at, team, roster) in [(old.team_at, 737, 736), (old.roster_at, 736, 737)] {
        let mut payload = pes17_texport_payload();
        assert_eq!(
            u32_at(&payload, at),
            736,
            "the record's own id is the team's"
        );
        payload[at..at + 4].copy_from_slice(&737u32.to_le_bytes());
        let err =
            Texport::from_bytes(&pes17_texport_container(payload), None).expect_err("refused");
        assert!(
            matches!(
                err,
                TexportError::IdsDisagree {
                    team: t,
                    roster: r,
                    tactics: 736,
                } if t == team && r == roster
            ),
            "id changed at {at:#x}: {err:?}"
        );
    }
}

/// The PES 16 texport's decrypted payload, sliced as a save fixture's is.
fn pes16_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    flate2::read::ZlibDecoder::new(
        &include_bytes!("../../tests/fixtures/pes16_texport_payload.bin.zz")[..],
    )
    .read_to_end(&mut payload)
    .expect("inflates");
    payload
}

/// The PES 16 texport as a container around the sliced payload (logo and
/// serial dropped, as the save fixtures are).
fn pes16_texport_bytes() -> Vec<u8> {
    let payload = pes16_payload();
    let mut description = b"Team Export Data 02".to_vec();
    description.resize(384, 0);
    SaveContainer {
        scheme: Scheme::Keyed(MasterKey::Pes16),
        description,
        logo: Vec::new(),
        payload,
        identifier: vec![0; MasterKey::Pes16.header_size() - 80],
        serial: Vec::new(),
    }
    .to_bytes(&test_salt())
    .expect("container encodes")
}

/// The one real file whose roster is not id-sorted (team 804): the
/// records follow the roster's order, which the consecutive-id scan
/// read two players of before dropping the other 21 without an error.
#[test]
fn pes16_texport_reads_players_in_the_rosters_order() {
    let bytes = pes16_texport_bytes();
    let t = Texport::from_bytes(&bytes, None).expect("opens");
    assert_eq!(t.version(), PesVersion::Pes16);
    assert_eq!(t.team().id, 804);
    assert!(!t.team().name.is_empty(), "the team record's name");
    let expected: Vec<u32> = vec![
        80401, 80402, 80423, 80404, 80405, 80407, 80422, 80408, 80410, 80411, 80409, 80413, 80414,
        80403, 80415, 80421, 80418, 80419, 80412, 80417, 80416, 80420, 80406,
    ];
    // The roster record itself holds the ids in the file's order.
    assert_eq!(
        t.team()
            .roster
            .iter()
            .filter(|slot| slot.player_id != 0)
            .map(|slot| slot.player_id)
            .collect::<Vec<_>>(),
        expected,
        "the roster's order"
    );
    let players = t.players();
    assert_eq!(
        players.iter().map(|p| p.id).collect::<Vec<_>>(),
        expected,
        "players in the roster's order"
    );
    // The shirt numbers are the roster's, carried into the document.
    let doc = t.to_team_toml().expect("to_team_toml");
    for (key, section) in &doc.players {
        let index = usize::from(*key - 1);
        assert_eq!(
            section.number,
            Some(t.team().roster[index].number),
            "slot {key}"
        );
    }
    assert_eq!(t.to_bytes(&test_salt()).expect("encodes"), bytes);
}
