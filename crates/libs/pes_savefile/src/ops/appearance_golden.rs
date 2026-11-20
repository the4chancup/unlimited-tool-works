//! Parity of the appearance block and the fingerprint with the reference
//! scripts' byte rules, transcribed here as literal offsets into the
//! appearance block (bytes relative to the block's first byte, the id), over
//! every player of every fixture. The rules are the whole semantic content of
//! the scripts; their container crypto is `container`'s parity test.

use std::collections::BTreeSet;

use pes_version::PesVersion;
use sha2::{Digest, Sha256};

use crate::codec::{read_player, write_player};
use crate::model::player::PlayerEntry;
use crate::ops::fingerprint::face_hash;
use crate::schema::ingame_face::IngameFaceField;
use crate::schema::{VersionSchema, schema_for};
use crate::test_support::{FIXTURES, appearance_for, copy_appearance, count, payload, record};

/// The script's slice: bytes 4..68 of the block are the donor's.
const SLICE: std::ops::Range<usize> = 4..68;

/// Where the appearance block starts inside the record that carries it: the
/// ingame-face run is the block's byte 22 on every version.
fn block_start(schema: &VersionSchema) -> usize {
    let run = schema.ingame_face_run();
    run.expect("the record carrying the appearance block has a run")
        .byte_offset as usize
        - 22
}

/// The record that carries player `i`'s appearance block, as a copy: the
/// appearance record on 15/16, the player record from 17.
fn block_record(payload: &[u8], schema: &VersionSchema, player: &PlayerEntry, i: usize) -> Vec<u8> {
    match &schema.appearance {
        Some((section, appearance)) => {
            for j in 0..count(payload, section) {
                let rec = record(payload, section, appearance.size, j);
                if u32::from_le_bytes(rec[..4].try_into().expect("id")) == player.id {
                    return rec.to_vec();
                }
            }
            panic!("no appearance record for {}", player.id);
        }
        None => record(payload, &schema.players, schema.player.size, i).to_vec(),
    }
}

/// Every decoded player with its record index.
fn players(payload: &[u8], schema: &VersionSchema) -> Vec<(usize, PlayerEntry)> {
    (0..count(payload, &schema.players))
        .map(|i| {
            let rec = record(payload, &schema.players, schema.player.size, i);
            let mut p = read_player(rec, schema.player).expect("decode");
            appearance_for(payload, schema, &mut p);
            (i, p)
        })
        .collect()
}

#[test]
fn the_appearance_copy_matches_the_scripts_slice_rule_on_every_fixture_player() {
    for version in FIXTURES {
        let payload = payload(version);
        let schema = schema_for(version);
        // PES 2014 maps no appearance block: the slice rule has nothing to copy.
        if schema.ingame_face_run().is_none() {
            continue;
        }
        let start = block_start(schema);
        let all = players(&payload, schema);
        assert!(all.len() > 100, "{version:?}: fixture has players");
        let mut changed = 0usize;
        for (k, (i, target)) in all.iter().enumerate() {
            let (j, donor) = &all[(k + 1) % all.len()];
            // The script's output: the donor's slice over the target's block.
            let target_rec = block_record(&payload, schema, target, *i);
            let donor_rec = block_record(&payload, schema, donor, *j);
            let mut expected = target_rec.clone();
            let slice = start + SLICE.start..start + SLICE.end;
            expected[slice.clone()].copy_from_slice(&donor_rec[slice]);
            if expected != target_rec {
                changed += 1;
            }
            // Ours: the model copy, written back into the same record.
            let mut moved = target.clone();
            copy_appearance(&mut moved, donor);
            assert_eq!(moved.id, target.id, "{version:?}: the target keeps its id");
            let mut actual = target_rec.clone();
            match &schema.appearance {
                Some((_, appearance)) => {
                    write_player(&moved, &mut actual, appearance).expect("write");
                    // The player record is not part of the slice: untouched.
                    let mut player_rec =
                        record(&payload, &schema.players, schema.player.size, *i).to_vec();
                    let before = player_rec.clone();
                    write_player(&moved, &mut player_rec, schema.player).expect("write");
                    assert_eq!(
                        player_rec, before,
                        "{version:?} {}: player record",
                        target.id
                    );
                }
                None => write_player(&moved, &mut actual, schema.player).expect("write"),
            }
            assert_eq!(
                actual, expected,
                "{version:?}: player {} <- {}",
                target.id, donor.id
            );
        }
        // Non-vacuity: most fixture players are placeholders sharing one block
        // (PES 15: 1049 of 5060 pairs differ), so a floor, not a majority.
        assert!(
            changed > 100,
            "{version:?}: the donors differ from the targets ({changed})"
        );
    }
}

/// `bits(byteOffset, bitOffset, bitCount)` of the compare script: a little-endian
/// u32 read at the block byte, shifted and masked.
fn bits(block: &[u8], byte: usize, bit: u32, n: u32) -> u32 {
    let word = u32::from_le_bytes(block[byte..byte + 4].try_into().expect("u32"));
    (word >> bit) & ((1 << n) - 1)
}

#[test]
fn fingerprint_fields_and_hash_match_the_compare_script_on_every_fixture_player() {
    for version in FIXTURES {
        let payload = payload(version);
        let schema = schema_for(version);
        // PES 2014 maps no appearance block: the fingerprint has nothing to read.
        if schema.ingame_face_run().is_none() {
            continue;
        }
        let start = block_start(schema);
        let mut hashes = BTreeSet::new();
        for (i, p) in players(&payload, schema) {
            let rec = block_record(&payload, schema, &p, i);
            let block = &rec[start..];
            let face = &p.appearance.ingame_face;
            let id = p.id;
            // (playerID, bootsGlovesData, faceID) = unpack('< III', appearance[0:12])
            assert_eq!(
                u32::from_le_bytes(block[0..4].try_into().unwrap()),
                p.appearance_id.unwrap_or(id),
                "{version:?} {id} appearance id"
            );
            let boots_gloves = u32::from_le_bytes(block[4..8].try_into().unwrap());
            assert_eq!(
                p.appearance.boots_id,
                (boots_gloves >> 4) & 0x3fff,
                "{version:?} {id} boots"
            );
            assert_eq!(
                p.appearance.gloves_id,
                (boots_gloves >> 18) & 0x3fff,
                "{version:?} {id} gloves"
            );
            assert_eq!(
                p.appearance.base_copy_id,
                u32::from_le_bytes(block[8..12].try_into().unwrap()),
                "{version:?} {id} face"
            );
            assert_eq!(
                u32::from(p.appearance.wrist_taping & 1),
                bits(block, 19, 6, 1),
                "{version:?} {id} right wrist"
            );
            assert_eq!(
                u32::from(p.appearance.wrist_taping >> 1),
                bits(block, 19, 7, 1),
                "{version:?} {id} left wrist"
            );
            assert_eq!(
                u32::from(p.appearance.spectacles_style),
                bits(block, 20, 3, 3),
                "{version:?} {id} glasses"
            );
            assert_eq!(
                u32::from(p.appearance.sleeves),
                bits(block, 20, 6, 2),
                "{version:?} {id} sleeves"
            );
            assert_eq!(
                u32::from(p.appearance.inners),
                bits(block, 20, 8, 2),
                "{version:?} {id} inners"
            );
            assert_eq!(
                u32::from(p.appearance.socks),
                bits(block, 20, 10, 2),
                "{version:?} {id} socks"
            );
            assert_eq!(
                u32::from(p.appearance.undershorts),
                bits(block, 20, 12, 2),
                "{version:?} {id} undershorts"
            );
            assert_eq!(
                u32::from(p.appearance.untucked),
                bits(block, 20, 14, 1),
                "{version:?} {id} shirt"
            );
            assert_eq!(
                u32::from(p.appearance.ankle_taping),
                bits(block, 20, 15, 1),
                "{version:?} {id} ankle"
            );
            assert_eq!(
                u32::from(face.get(IngameFaceField::PlayerGloves).unwrap()),
                bits(block, 22, 0, 1),
                "{version:?} {id} winter gloves"
            );
            assert_eq!(
                u32::from(face.get(IngameFaceField::PlayerGlovesColor).unwrap()),
                bits(block, 22, 1, 3),
                "{version:?} {id} winter gloves colour"
            );
            assert_eq!(
                u32::from(face.get(IngameFaceField::SkinColor).unwrap()),
                bits(block, 44, 8, 3),
                "{version:?} {id} skin"
            );
            // ingameFace = appearance[22:]; [0] &= ~15; [23] &= ~7; sha256 hex [0:8]
            let mut run = block[22..].to_vec();
            run[0] &= !15;
            run[23] &= !7;
            let digest = Sha256::digest(&run);
            let expected: String = digest[..4].iter().map(|b| format!("{b:02x}")).collect();
            let actual = face_hash(face);
            assert_eq!(actual.to_string(), expected, "{version:?} {id} hash");
            assert_eq!(actual.0, digest[..4], "{version:?} {id} hash bytes");
            hashes.insert(expected);
        }
        // The PES 20 fixture's 414 named players share one face once gloves and skin are masked.
        assert_eq!(
            hashes.len() > 1,
            version != PesVersion::Pes20,
            "{version:?}: {} distinct faces",
            hashes.len()
        );
    }
}
