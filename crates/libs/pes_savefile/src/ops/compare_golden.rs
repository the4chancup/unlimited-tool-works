//! Parity of `ops::compare` with the compare script's diff outcome: for every
//! fixture player `a` and its twin `a'` carrying the next player's appearance
//! block (same id, so the script's `face == playerID → 0` rule reads the same
//! on both sides), each key the script prints differs exactly when our
//! corresponding row exists, with the script's numbers.

use sha2::{Digest, Sha256};

use crate::codec::{read_player, write_player};
use crate::model::player::PlayerEntry;
use crate::ops::compare::{DiffScope, PlayerDiff, compare_players};
use crate::schema::fields::PlayerField;
use crate::schema::ingame_face::IngameFaceField;
use crate::schema::{VersionSchema, schema_for};
use crate::test_support::{FIXTURES, appearance_for, copy_appearance, count, payload, record};

/// The appearance block of `p`, as the record bytes our writer produces.
fn block(p: &PlayerEntry, schema: &VersionSchema) -> Vec<u8> {
    let (record_schema, run) = match &schema.appearance {
        Some((_, appearance)) => (*appearance, appearance.ingame_face.as_ref()),
        None => (schema.player, schema.player.ingame_face.as_ref()),
    };
    let start = run.expect("run").byte_offset as usize - 22;
    let mut rec = vec![0u8; record_schema.size];
    write_player(p, &mut rec, record_schema).expect("write");
    rec[start..].to_vec()
}

fn bits(block: &[u8], byte: usize, bit: u32, n: u32) -> u32 {
    let word = u32::from_le_bytes(block[byte..byte + 4].try_into().expect("u32"));
    (word >> bit) & ((1 << n) - 1)
}

/// The script's per-player dictionary, keys in its order; `face` is 0 when it
/// equals the player's id.
/// `id` is passed in: the `face == playerID` rule compares against the
/// entry's own id, not the block's copy.
fn script_dict(block: &[u8], id: u32) -> Vec<(&'static str, u32)> {
    let boots_gloves = u32::from_le_bytes(block[4..8].try_into().unwrap());
    let mut face = u32::from_le_bytes(block[8..12].try_into().unwrap());
    if face == id {
        face = 0;
    }
    let mut run = block[22..].to_vec();
    run[0] &= !15;
    run[23] &= !7;
    let hash = u32::from_be_bytes(Sha256::digest(&run)[..4].try_into().unwrap());
    vec![
        ("face", face),
        ("boots", (boots_gloves >> 4) & 0x3fff),
        ("gloves", (boots_gloves >> 18) & 0x3fff),
        ("rightWristTaping", bits(block, 19, 6, 1)),
        ("leftWristTaping", bits(block, 19, 7, 1)),
        ("glasses", bits(block, 20, 3, 3)),
        ("sleeves", bits(block, 20, 6, 2)),
        ("inners", bits(block, 20, 8, 2)),
        ("socks", bits(block, 20, 10, 2)),
        ("undershorts", bits(block, 20, 12, 2)),
        ("shirt", bits(block, 20, 14, 1)),
        ("ankleTaping", bits(block, 20, 15, 1)),
        ("winterGlovesOn", bits(block, 22, 0, 1)),
        ("winterGlovesColor", bits(block, 22, 1, 3)),
        ("skin", bits(block, 44, 8, 3)),
        ("ingameFace", hash),
    ]
}

/// Our row for a script key, with its old/new numbers.
fn row(diffs: &[PlayerDiff], key: &str, id: u32) -> Option<(u32, u32)> {
    let field = |f: PlayerField| {
        diffs.iter().find_map(|d| match d {
            PlayerDiff::Field { field, old, new } if *field == f => Some((*old, *new)),
            _ => None,
        })
    };
    let face = |f: IngameFaceField| {
        diffs.iter().find_map(|d| match d {
            PlayerDiff::Face { field, old, new } if *field == f => {
                Some((u32::from(*old), u32::from(*new)))
            }
            _ => None,
        })
    };
    match key {
        // The script prints 0 for a base-copy id equal to the player's own.
        "face" => field(PlayerField::BaseCopyId)
            .map(|(o, n)| (if o == id { 0 } else { o }, if n == id { 0 } else { n })),
        "boots" => field(PlayerField::BootsId),
        "gloves" => field(PlayerField::GlovesId),
        // The script splits the two taping bits; ours is one 2-bit field.
        "rightWristTaping" => field(PlayerField::WristTaping).map(|(o, n)| (o & 1, n & 1)),
        "leftWristTaping" => field(PlayerField::WristTaping).map(|(o, n)| (o >> 1, n >> 1)),
        "glasses" => field(PlayerField::SpectaclesStyle),
        "sleeves" => field(PlayerField::Sleeves),
        "inners" => field(PlayerField::Inners),
        "socks" => field(PlayerField::Socks),
        "undershorts" => field(PlayerField::Undershorts),
        "shirt" => field(PlayerField::Untucked),
        "ankleTaping" => field(PlayerField::AnkleTaping),
        "winterGlovesOn" => face(IngameFaceField::PlayerGloves),
        "winterGlovesColor" => face(IngameFaceField::PlayerGlovesColor),
        "skin" => face(IngameFaceField::SkinColor),
        _ => unreachable!("ingameFace is checked separately"),
    }
}

/// Whether the run differs outside the gloves and skin bits: a `Face` row for
/// any other field, or a `FaceRun` row.
fn run_changed(diffs: &[PlayerDiff]) -> Option<(u32, u32)> {
    diffs.iter().find_map(|d| match d {
        PlayerDiff::FaceRun { old, new, .. } => {
            Some((u32::from_be_bytes(old.0), u32::from_be_bytes(new.0)))
        }
        _ => None,
    })
}

#[test]
fn the_comparators_aesthetics_rows_match_the_compare_scripts_diff_on_every_fixture_player() {
    for version in FIXTURES {
        let payload = payload(version);
        let schema = schema_for(version);
        let all: Vec<PlayerEntry> = (0..count(&payload, &schema.players))
            .map(|i| {
                let rec = record(&payload, &schema.players, schema.player.size, i);
                let mut p = read_player(rec, schema.player).expect("decode");
                appearance_for(&payload, schema, &mut p);
                p
            })
            .collect();
        let mut pairs_with_rows = 0usize;
        for (k, a) in all.iter().enumerate() {
            assert!(
                compare_players(a, a, schema).expect("compare").is_empty(),
                "{version:?} {}: a player equals its own copy",
                a.id
            );
            let mut twin = a.clone();
            copy_appearance(&mut twin, &all[(k + 1) % all.len()]);
            let diffs = compare_players(a, &twin, schema).expect("compare");
            if !diffs.is_empty() {
                pairs_with_rows += 1;
            }
            for d in &diffs {
                assert_eq!(
                    d.scope(),
                    DiffScope::Aesthetics,
                    "{version:?} {}: {d:?}",
                    a.id
                );
                assert!(
                    !matches!(d, PlayerDiff::Text { .. }),
                    "{version:?} {}: {d:?}",
                    a.id
                );
            }
            let (old, new) = (
                script_dict(&block(a, schema), a.id),
                script_dict(&block(&twin, schema), twin.id),
            );
            for ((key, o), (_, n)) in old.iter().zip(&new) {
                let ours = if *key == "ingameFace" {
                    // The hash row carries the script's fingerprints when the
                    // unused bits changed; a change inside a named field is
                    // that field's row instead. Either way, the hash differs
                    // exactly when something outside gloves and skin changed.
                    match run_changed(&diffs) {
                        Some(hashes) => Some(hashes),
                        None => diffs
                            .iter()
                            .any(|d| {
                                matches!(d, PlayerDiff::Face { field, .. }
                                if !matches!(field, IngameFaceField::PlayerGloves
                                    | IngameFaceField::PlayerGlovesColor
                                    | IngameFaceField::SkinColor))
                            })
                            .then_some((*o, *n)),
                    }
                } else {
                    row(&diffs, key, a.id)
                };
                if o == n {
                    assert!(
                        ours.is_none() || ours == Some((*o, *n)),
                        "{version:?} {}: {key} unchanged ({o}) but our row says {ours:?}",
                        a.id
                    );
                } else {
                    assert_eq!(
                        ours,
                        Some((*o, *n)),
                        "{version:?} {}: {key} {o} -> {n} has no matching row in {diffs:?}",
                        a.id
                    );
                }
            }
        }
        // Non-vacuity floor: most fixture players share one placeholder block.
        assert!(
            pairs_with_rows > 100,
            "{version:?}: {pairs_with_rows} pairs differ"
        );
    }
}
