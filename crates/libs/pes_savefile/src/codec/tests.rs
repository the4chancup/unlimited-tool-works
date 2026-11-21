//! The codec test suite: fixture round trips, literals and censuses.

use pes_version::PesVersion;

use super::team::tactics_runs;
use super::{
    CodecError, bits, read_player, read_player_into, read_roster_into, read_tactics_into,
    read_team, runs, write_player, write_roster, write_tactics, write_team,
};
use crate::model::ingame_face::IngameFace;
use crate::model::player::{PlayerEntry, STRIPPED_APPEARANCE_ID};
use crate::model::tactics::{CombinationSlot, Formation, FormationSlot, GamePlan};
use crate::model::team::{RosterSlot, TeamEntry};
use crate::schema::fields::{
    FormationPart, InstructionSide, PlayerField, PlayerText, PresetField, RosterField,
    TacticsField, TeamField, TeamText,
};
use crate::schema::ingame_face::IngameFaceField;
use crate::schema::{RecordSchema, TacticsSchema};
use crate::schema::{VersionSchema, schema_for};
use crate::test_support::{FIXTURES, appearance_for, count, find_player, payload, record};

/// Every player record of `schema`'s fixture as decoded entries.
fn players(payload: &[u8], schema: &VersionSchema) -> Vec<PlayerEntry> {
    (0..count(payload, &schema.players))
        .map(|i| {
            read_player(
                schema.version,
                record(payload, &schema.players, schema.player.size, i),
                schema.player,
            )
            .expect("player record decodes")
        })
        .collect()
}

#[test]
fn every_player_record_round_trips_byte_for_byte() {
    for version in FIXTURES {
        let payload = payload(version);
        let schema = schema_for(version);
        for i in 0..count(&payload, &schema.players) {
            let record = record(&payload, &schema.players, schema.player.size, i);
            let player = read_player(version, record, schema.player).expect("decode");
            let mut written = record.to_vec();
            write_player(version, &player, &mut written, schema.player).expect("encode");
            assert_eq!(written, record, "{version:?} player record {i}");
        }
        if let Some((section, appearance)) = &schema.appearance {
            // Every appearance record round-trips through a player patched
            // by it, and every player id keys exactly one record.
            let records: Vec<&[u8]> = (0..count(&payload, section))
                .map(|i| record(&payload, section, appearance.size, i))
                .collect();
            let mut seen = std::collections::HashMap::new();
            for rec in &records {
                let id = u32::from_le_bytes(rec[..4].try_into().expect("id"));
                *seen.entry(id).or_insert(0usize) += 1;
            }
            // Only versions holding a record per player key every id (PES
            // 2014's section holds edited and created players alone).
            if schema.appearance_for_every_player() {
                for i in 0..count(&payload, &schema.players) {
                    let player_rec = record(&payload, &schema.players, schema.player.size, i);
                    let player = read_player(version, player_rec, schema.player).expect("decode");
                    assert_eq!(
                        seen.get(&player.id),
                        Some(&1),
                        "{version:?} player {} keys one appearance record",
                        player.id
                    );
                }
            }
            for rec in records {
                let mut player = PlayerEntry::default();
                read_player_into(version, &mut player, rec, appearance).expect("decode appearance");
                let mut written = rec.to_vec();
                write_player(version, &player, &mut written, appearance)
                    .expect("encode appearance");
                assert_eq!(written, rec, "{version:?} appearance record");
            }
        }
    }
}

#[test]
fn player_70101_per_version() {
    // name, shirt_name, nationality, height, weight, attacking_prowess, age,
    // registered, speed.
    let expected = [
        (
            PesVersion::Pes15,
            "QUADS ONLY",
            "TRIS BTFO",
            231,
            189,
            80,
            74,
            20,
            0,
            74,
        ),
        (
            PesVersion::Pes16,
            "PLACEHOLDER",
            "PLACEHOLDER",
            231,
            180,
            80,
            77,
            20,
            1,
            77,
        ),
        (
            PesVersion::Pes17,
            "PLACEHOLDER",
            "PLACEHOLDER",
            231,
            180,
            80,
            77,
            20,
            1,
            77,
        ),
        (
            PesVersion::Pes18,
            "QUADS ONLY",
            "TRIS GTFO",
            238,
            194,
            70,
            77,
            17,
            1,
            77,
        ),
        (
            PesVersion::Pes19,
            "QUADS ONLY",
            "TRIS BTFO",
            231,
            189,
            80,
            77,
            20,
            0,
            77,
        ),
        (
            PesVersion::Pes21,
            "PLACEHOLDER",
            "PLACEHOLDER",
            231,
            180,
            80,
            77,
            20,
            1,
            77,
        ),
    ];
    for (version, name, shirt, nat, height, weight, att, age, reg, speed) in expected {
        let payload = payload(version);
        let player = find_player(&payload, schema_for(version), 70101);
        assert_eq!(
            (
                player.name.as_str(),
                player.shirt_name.as_str(),
                player.basic.nationality,
                player.basic.height,
                player.basic.weight,
                player.stats.attacking_prowess,
                player.basic.age,
                player.positions.registered,
                player.stats.speed,
            ),
            (name, shirt, nat, height, weight, att, age, reg, speed),
            "{version:?} player 70101"
        );
    }
    // Appearance literals on the two versions that keep a separate record.
    let p15 = find_player(
        &payload(PesVersion::Pes15),
        schema_for(PesVersion::Pes15),
        70101,
    );
    assert_eq!(
        (
            p15.appearance.boots_id,
            p15.appearance.gloves_id,
            p15.appearance
                .ingame_face
                .get(IngameFaceField::SkinColor)
                .expect("read run"),
            p15.appearance
                .ingame_face
                .get(IngameFaceField::IrisColor)
                .expect("read run"),
            p15.appearance.neck_length,
        ),
        (55, 11, 1, 1, 7),
        "PES 15 player 70101 appearance"
    );
    let p16 = find_player(
        &payload(PesVersion::Pes16),
        schema_for(PesVersion::Pes16),
        70101,
    );
    assert_eq!(
        (
            p16.appearance.boots_id,
            p16.appearance.gloves_id,
            p16.appearance
                .ingame_face
                .get(IngameFaceField::SkinColor)
                .expect("read run"),
            p16.appearance
                .ingame_face
                .get(IngameFaceField::IrisColor)
                .expect("read run"),
            p16.appearance.neck_length,
        ),
        (0, 0, 1, 1, 7),
        "PES 16 player 70101 appearance"
    );
}

/// The `PlayerField` variants that hold a 7-bit ability value in the model.
const ABILITIES: [PlayerField; 24] = [
    PlayerField::AttackingProwess,
    PlayerField::BallControl,
    PlayerField::Dribbling,
    PlayerField::LowPass,
    PlayerField::LoftedPass,
    PlayerField::Finishing,
    PlayerField::PlaceKicking,
    PlayerField::Swerve,
    PlayerField::Heading,
    PlayerField::DefensiveProwess,
    PlayerField::BallWinning,
    PlayerField::KickingPower,
    PlayerField::Speed,
    PlayerField::ExplosivePower,
    PlayerField::BodyControl,
    PlayerField::Jump,
    PlayerField::Goalkeeping,
    PlayerField::Stamina,
    PlayerField::Catching,
    PlayerField::Clearing,
    PlayerField::Reflexes,
    PlayerField::Coverage,
    PlayerField::PhysicalContact,
    PlayerField::TightPossession,
];

#[test]
fn the_player_census_matches_the_measured_ranges() {
    let counts = [11998usize, 5060, 5060, 5060, 4646, 4830, 5060, 5060];
    for (i, version) in FIXTURES.iter().enumerate() {
        let payload = payload(*version);
        let schema = schema_for(*version);
        assert_eq!(
            count(&payload, &schema.players),
            counts[i],
            "{version:?} player count"
        );
        // PES 18's magical-girl joke team stores ages 13/14: the one
        // measured exception set the 15..=50 census has.
        let age_exceptions: &[(u32, u8)] = if *version == PesVersion::Pes18 {
            &[
                (81401, 14),
                (81403, 14),
                (81408, 14),
                (81411, 14),
                (81413, 13),
                (81416, 14),
                (81417, 14),
                (81421, 13),
                (81423, 14),
            ]
        } else {
            &[]
        };
        let mut seen_exceptions = 0usize;
        for player in players(&payload, schema) {
            // The model checks read only fields the version's schema maps
            // (PES 2014's player record maps the id and texts alone).
            if schema.player.has(PlayerField::Age) {
                match age_exceptions.iter().find(|(id, _)| *id == player.id) {
                    Some((_, age)) => {
                        seen_exceptions += 1;
                        assert_eq!(player.basic.age, *age, "{version:?} age exception");
                    }
                    None => assert!(
                        (15..=50).contains(&player.basic.age),
                        "{version:?} player {} age {}",
                        player.id,
                        player.basic.age
                    ),
                }
            }
            if schema.player.has(PlayerField::RegisteredPosition) {
                assert!(
                    player.positions.registered <= 12,
                    "{version:?} player {} registered {}",
                    player.id,
                    player.positions.registered
                );
            }
            for spec in schema.player.fields {
                if spec.bit_width == 7 && ABILITIES.contains(&spec.field) {
                    let value = player.get(spec.field).expect("a stats field");
                    if !(*version == PesVersion::Pes16
                        && player.id == 71211
                        && spec.field == PlayerField::PlaceKicking)
                    {
                        assert!(
                            (40..=99).contains(&value),
                            "{version:?} player {} {:?} {value}",
                            player.id,
                            spec.field
                        );
                    } else {
                        assert_eq!(value, 100, "the one measured exception");
                    }
                }
            }
        }
        assert_eq!(
            seen_exceptions,
            age_exceptions.len(),
            "{version:?} every age exception seen exactly once"
        );
    }
    // The two measured non-ASCII shirt names.
    let p16 = find_player(
        &payload(PesVersion::Pes16),
        schema_for(PesVersion::Pes16),
        73903,
    );
    assert_eq!(p16.shirt_name, "N\u{fc}RBURG");
    let p21 = find_player(
        &payload(PesVersion::Pes21),
        schema_for(PesVersion::Pes21),
        83323,
    );
    assert_eq!(p21.shirt_name, "FUR\u{f0}USAGA");
}

/// The census the `PLAYABLE_POSITIONS` order encodes: every rated player —
/// one with at least one nonzero rating — stores 2 (A) at his registered
/// position, bar the four measured exceptions, named by id so a change in
/// which players fail is seen.
#[test]
fn a_rated_player_stores_a_at_his_registered_position() {
    const EXCEPTIONS: [(PesVersion, u32); 4] = [
        (PesVersion::Pes18, 70101),
        (PesVersion::Pes18, 78908),
        (PesVersion::Pes18, 82807),
        (PesVersion::Pes19, 84123),
    ];
    let mut seen = 0usize;
    for version in FIXTURES {
        let payload = payload(version);
        let schema = schema_for(version);
        for player in players(&payload, schema) {
            if !player.positions.playable.iter().any(|&rating| rating != 0) {
                continue;
            }
            let entry = crate::interchange::team_toml::labels::PLAYABLE_POSITIONS
                .iter()
                .position(|&p| p == usize::from(player.positions.registered))
                .expect("a permutation covers every position");
            let rating = player.positions.playable[entry];
            if EXCEPTIONS.contains(&(version, player.id)) {
                seen += 1;
                assert_ne!(
                    rating, 2,
                    "{version:?} player {} is a measured exception",
                    player.id
                );
            } else {
                assert_eq!(
                    rating, 2,
                    "{version:?} player {} registered {}",
                    player.id, player.positions.registered
                );
            }
        }
    }
    assert_eq!(seen, EXCEPTIONS.len(), "every exception seen exactly once");
}

#[test]
fn the_ingame_face_run_has_the_versions_length() {
    for version in FIXTURES {
        let payload = payload(version);
        let schema = schema_for(version);
        let mut player = read_player(
            version,
            record(&payload, &schema.players, schema.player.size, 0),
            schema.player,
        )
        .expect("decode");
        appearance_for(&payload, schema, &mut player);
        // PES 2014's schema maps no ingame-face run: 0 bytes. PES 15's is
        // 46, every other's 50.
        let expected = match version {
            PesVersion::Pes14 => 0,
            PesVersion::Pes15 => 46,
            PesVersion::Pes16
            | PesVersion::Pes17
            | PesVersion::Pes18
            | PesVersion::Pes19
            | PesVersion::Pes20
            | PesVersion::Pes21 => 50,
        };
        assert_eq!(
            player.appearance.ingame_face.bytes().len(),
            expected,
            "{version:?} run length"
        );
    }
}

#[test]
fn a_wrong_length_ingame_face_run_is_refused() {
    let payload = payload(PesVersion::Pes17);
    let schema = schema_for(PesVersion::Pes17);
    let mut player = find_player(&payload, schema, 70101);
    player.appearance.ingame_face = IngameFace::from_bytes(vec![0; 49]);
    let mut record = record(&payload, &schema.players, schema.player.size, 0).to_vec();
    match write_player(schema.version, &player, &mut record, schema.player) {
        Err(CodecError::RunSize { expected, got }) => {
            assert_eq!((expected, got), (50, 49));
        }
        other => panic!("expected CodecError::RunSize, got {other:?}"),
    }
    // A run written into a record whose schema has none is ignored.
    let schema15 = schema_for(PesVersion::Pes15);
    let mut record = vec![0u8; schema15.player.size];
    write_player(schema15.version, &player, &mut record, schema15.player)
        .expect("no run on the 15/16 player record");
}

#[test]
fn version_gated_fields_are_none_where_the_version_lacks_them() {
    let p15 = players(&payload(PesVersion::Pes15), schema_for(PesVersion::Pes15));
    for p in &p15 {
        assert!(p.stats.clearing.is_none() && p.stats.reflexes.is_none());
        assert!(p.stats.coverage.is_none() && p.stats.physical_contact.is_none());
        assert!(p.stats.star.is_none() && p.stats.catching.is_some());
    }
    let p17 = players(&payload(PesVersion::Pes17), schema_for(PesVersion::Pes17));
    for p in &p17 {
        assert!(p.stats.physical_contact.is_some() && p.stats.star.is_none());
    }
    let p19 = players(&payload(PesVersion::Pes19), schema_for(PesVersion::Pes19));
    for p in &p19 {
        assert!(p.stats.star.is_some() && p.stats.tight_possession.is_none());
    }
    let p21 = players(&payload(PesVersion::Pes21), schema_for(PesVersion::Pes21));
    for p in &p21 {
        assert!(
            p.stats.clearing.is_some()
                && p.stats.reflexes.is_some()
                && p.stats.coverage.is_some()
                && p.stats.physical_contact.is_some()
                && p.stats.star.is_some()
                && p.stats.tight_possession.is_some()
                && p.stats.aggression.is_some()
                && p.stats.playing_attitude.is_some()
                && p.positions.stronger_hand.is_some()
                && p.motion.dribbling.is_some()
                && p.stats.catching.is_some()
        );
    }
    // The base-copy flag: PES 15 to 18 store it, 19 to 21 do not.
    for (version, stored) in [
        (PesVersion::Pes15, true),
        (PesVersion::Pes16, true),
        (PesVersion::Pes17, true),
        (PesVersion::Pes18, true),
        (PesVersion::Pes19, false),
        (PesVersion::Pes20, false),
        (PesVersion::Pes21, false),
    ] {
        for p in players(&payload(version), schema_for(version)) {
            assert_eq!(
                p.edit_flags.base_copy.is_some(),
                stored,
                "{version:?} player {}",
                p.id
            );
        }
    }
}

/// The appearance block's first u32 on PES 17+: every fixture player carries
/// his own id (no fixture save holds a stripped save's -1).
#[test]
fn the_appearance_id_reads_the_players_own_on_every_fixture_player() {
    for version in [
        PesVersion::Pes17,
        PesVersion::Pes18,
        PesVersion::Pes19,
        PesVersion::Pes20,
        PesVersion::Pes21,
    ] {
        let payload = payload(version);
        let schema = schema_for(version);
        for p in players(&payload, schema) {
            assert_eq!(p.appearance_id, Some(p.id), "{version:?} player {}", p.id);
        }
    }
}

/// PES 15/16: the appearance record's first u32 is the join key itself, read
/// into `id`; `appearance_id` stays `None`, and a `Some` has no run to write
/// through — the codec leaves it alone, as it does every version-gated
/// `Option` the schema lacks.
#[test]
fn the_appearance_id_is_none_on_pes15_and_16() {
    for version in [PesVersion::Pes15, PesVersion::Pes16] {
        let payload = payload(version);
        let schema = schema_for(version);
        let mut player = find_player(&payload, schema, 70101);
        assert_eq!(player.appearance_id, None, "{version:?}");
        player.appearance_id = Some(STRIPPED_APPEARANCE_ID);
        let index = (0..count(&payload, &schema.players))
            .find(|&i| {
                read_player(
                    version,
                    record(&payload, &schema.players, schema.player.size, i),
                    schema.player,
                )
                .expect("decode")
                .id == 70101
            })
            .expect("70101's record");
        let rec = record(&payload, &schema.players, schema.player.size, index);
        let mut written = rec.to_vec();
        write_player(version, &player, &mut written, schema.player).expect("writes");
        assert_eq!(written, rec, "{version:?}: the Some has no run");
        let (section, appearance) = schema.appearance.as_ref().expect("a split record");
        let rec = (0..count(&payload, section))
            .map(|i| record(&payload, section, appearance.size, i))
            .find(|rec| u32::from_le_bytes(rec[..4].try_into().expect("id")) == 70101)
            .expect("70101's appearance record");
        let mut written = rec.to_vec();
        write_player(version, &player, &mut written, appearance).expect("writes");
        assert_eq!(written, rec, "{version:?}: the Some has no run");
    }
}

/// A `-1` (u32::MAX) writes back through the field and rereads as `u32::MAX`,
/// every other byte of the record untouched.
#[test]
fn a_stripped_appearance_id_writes_and_rereads() {
    let payload = payload(PesVersion::Pes21);
    let schema = schema_for(PesVersion::Pes21);
    let mut player = find_player(&payload, schema, 70101);
    player.appearance_id = Some(STRIPPED_APPEARANCE_ID);
    let index = (0..count(&payload, &schema.players))
        .find(|&i| {
            read_player(
                schema.version,
                record(&payload, &schema.players, schema.player.size, i),
                schema.player,
            )
            .expect("decode")
            .id == 70101
        })
        .expect("70101's record");
    let rec = record(&payload, &schema.players, schema.player.size, index);
    let mut written = rec.to_vec();
    write_player(schema.version, &player, &mut written, schema.player).expect("writes");
    let spec = schema
        .player
        .fields
        .iter()
        .find(|f| f.field == PlayerField::AppearanceId)
        .expect("PES 21 has the row");
    for i in 0..rec.len() {
        let inside = i * 8 >= spec.bit_offset as usize && i * 8 < (spec.bit_offset + 32) as usize;
        assert_eq!(
            written[i] != rec[i],
            inside,
            "byte {i} differs outside the appearance-id run"
        );
    }
    let back = read_player(schema.version, &written, schema.player).expect("redecode");
    assert_eq!(back.appearance_id, Some(STRIPPED_APPEARANCE_ID));
}

/// PES 18's `ArmMovementDribbling` is 3 bits at offset 295, ending where
/// `ArmMovementRunning` begins at 298 — read as 2 bits it could never reach
/// 4, but real saves carry 4 to 7.
#[test]
fn pes18_arm_movement_dribbling_reads_its_third_bit() {
    let players = players(&payload(PesVersion::Pes18), schema_for(PesVersion::Pes18));
    let four_or_more = players
        .iter()
        .filter(|p| p.motion.arm_movement_dribbling >= 4)
        .count();
    assert_eq!(four_or_more, 16);
}

#[test]
fn text_writes_patch_the_field_and_keep_the_old_tail() {
    let payload = payload(PesVersion::Pes17);
    let schema = schema_for(PesVersion::Pes17);
    let record = record(&payload, &schema.players, schema.player.size, 0);
    let mut player = read_player(schema.version, record, schema.player).expect("decode");

    // A shorter name leaves the old tail bytes after the new NUL.
    player.name = "AB".to_string();
    let mut written = record.to_vec();
    write_player(schema.version, &player, &mut written, schema.player).expect("encode");
    let name = schema
        .player
        .texts
        .iter()
        .find(|t| t.text == PlayerText::Name)
        .expect("name spec");
    let at = name.byte_offset as usize;
    assert_eq!(&written[at..at + 3], b"AB\0");
    assert_eq!(
        written[at + 3..at + name.len as usize],
        record[at + 3..at + name.len as usize]
    );

    // A name one byte longer than its field is refused.
    player.name = "x".repeat(name.len as usize + 1);
    assert!(matches!(
        write_player(schema.version, &player, &mut written, schema.player),
        Err(CodecError::Text { .. })
    ));

    // A single-byte shirt char writes as its byte and reads back equal.
    let shirt = schema
        .player
        .texts
        .iter()
        .find(|t| t.text == PlayerText::ShirtName)
        .expect("shirt spec");
    player.name = "AB".to_string();
    player.shirt_name = "MéXICO".to_string();
    write_player(schema.version, &player, &mut written, schema.player).expect("encode");
    assert_eq!(written[shirt.byte_offset as usize + 1], 0xE9);
    let reread = read_player(schema.version, &written, schema.player).expect("redecode");
    assert_eq!(reread.shirt_name, "MéXICO");

    // A char above U+00FF is refused.
    player.shirt_name = "€URO".to_string();
    assert!(matches!(
        write_player(schema.version, &player, &mut written, schema.player),
        Err(CodecError::Text { .. })
    ));
}

#[test]
fn a_text_filling_its_field_writes_with_no_nul() {
    let payload = payload(PesVersion::Pes17);
    let schema = schema_for(PesVersion::Pes17);
    let record = record(&payload, &schema.players, schema.player.size, 0);
    let mut player = read_player(schema.version, record, schema.player).expect("decode");
    let mut written = record.to_vec();

    // A Name of exactly the field's length writes unterminated and reads back.
    let name = schema
        .player
        .texts
        .iter()
        .find(|t| t.text == PlayerText::Name)
        .expect("name spec");
    player.name = "n".repeat(name.len as usize);
    write_player(schema.version, &player, &mut written, schema.player).expect("encode");
    let at = name.byte_offset as usize;
    assert_eq!(&written[at..at + name.len as usize], player.name.as_bytes());
    let reread = read_player(schema.version, &written, schema.player).expect("redecode");
    assert_eq!(reread.name, player.name);

    // One byte more is refused.
    player.name = "n".repeat(name.len as usize + 1);
    assert!(matches!(
        write_player(schema.version, &player, &mut written, schema.player),
        Err(CodecError::Text { .. })
    ));

    // The same boundary holds for the single-byte ShirtName.
    let shirt = schema
        .player
        .texts
        .iter()
        .find(|t| t.text == PlayerText::ShirtName)
        .expect("shirt spec");
    player.name = "AB".to_string();
    player.shirt_name = "s".repeat(shirt.len as usize);
    write_player(schema.version, &player, &mut written, schema.player).expect("encode");
    let at = shirt.byte_offset as usize;
    assert_eq!(
        &written[at..at + shirt.len as usize],
        player.shirt_name.as_bytes()
    );
    let reread = read_player(schema.version, &written, schema.player).expect("redecode");
    assert_eq!(reread.shirt_name, player.shirt_name);
    player.shirt_name = "s".repeat(shirt.len as usize + 1);
    assert!(matches!(
        write_player(schema.version, &player, &mut written, schema.player),
        Err(CodecError::Text { .. })
    ));
}

#[test]
fn a_full_field_name_round_trips_byte_for_byte() {
    // The census's PES 17 shape: a Name of 46 bytes with no NUL.
    let payload = payload(PesVersion::Pes17);
    let schema = schema_for(PesVersion::Pes17);
    let mut record = record(&payload, &schema.players, schema.player.size, 0).to_vec();
    let name = schema
        .player
        .texts
        .iter()
        .find(|t| t.text == PlayerText::Name)
        .expect("name spec");
    let at = name.byte_offset as usize;
    record[at..at + name.len as usize].fill(b'A');
    let player = read_player(schema.version, &record, schema.player).expect("decode");
    assert_eq!(player.name, "A".repeat(name.len as usize));
    let mut written = record.clone();
    write_player(schema.version, &player, &mut written, schema.player).expect("encode");
    assert_eq!(written, record);
}

#[test]
fn a_gated_field_left_none_is_an_error_not_a_zero() {
    let payload = payload(PesVersion::Pes21);
    let schema = schema_for(PesVersion::Pes21);
    let mut player = find_player(&payload, schema, 70101);
    player.stats.star = None;
    let mut record = vec![0u8; schema.player.size];
    match write_player(schema.version, &player, &mut record, schema.player) {
        Err(CodecError::Missing { field }) => assert_eq!(field, "Star"),
        other => panic!("expected CodecError::Missing, got {other:?}"),
    }
}

/// PES 2014 stores each stat less 40, the height less 132 and the weight
/// less 30: the codec adds the offset on read and takes it off on write,
/// refusing a value below its offset rather than wrapping it. The
/// version, not the schema, decides: PES 2014's table read as another
/// version adds nothing.
#[test]
fn pes14_stored_offsets_apply_on_read_and_write() {
    let schema = schema_for(PesVersion::Pes14);
    let mut record = vec![0u8; schema.player.size];
    for (wanted, stored) in [
        (PlayerField::AttackingProwess, 40),
        (PlayerField::Height, 48),
        (PlayerField::Weight, 50),
    ] {
        let (_, offset, width) = runs(schema.player)
            .find(|(field, _, _)| *field == wanted)
            .expect("the schema maps the field");
        bits::write_bits(&mut record, offset, width, stored);
    }
    let player = read_player(PesVersion::Pes14, &record, schema.player).expect("decode");
    assert_eq!(
        (
            player.stats.attacking_prowess,
            player.basic.height,
            player.basic.weight
        ),
        (80, 180, 80),
        "stored + the offset"
    );
    let player = read_player(PesVersion::Pes15, &record, schema.player).expect("decode");
    assert_eq!(
        (
            player.stats.attacking_prowess,
            player.basic.height,
            player.basic.weight
        ),
        (40, 48, 50),
        "another version adds no offset through the same table"
    );
    // Below the offset is refused rather than wrapped; the six-bit run's
    // top stored value is 63.
    let mut player = PlayerEntry::bare(PesVersion::Pes14);
    player.stats.attacking_prowess = 39;
    let mut record = vec![0u8; schema.player.size];
    match write_player(PesVersion::Pes14, &player, &mut record, schema.player) {
        Err(CodecError::BelowStoredOffset {
            field,
            value,
            offset,
        }) => {
            assert_eq!(
                (field.as_str(), value, offset),
                ("AttackingProwess", 39, 40)
            );
        }
        other => panic!("expected CodecError::BelowStoredOffset, got {other:?}"),
    }
    player.stats.attacking_prowess = 103;
    write_player(PesVersion::Pes14, &player, &mut record, schema.player).expect("encode");
    let (_, offset, width) = runs(schema.player)
        .find(|(field, _, _)| *field == PlayerField::AttackingProwess)
        .expect("the schema maps the field");
    assert_eq!(bits::read_bits(&record, offset, width), 63, "shown - 40");
    player.stats.attacking_prowess = 104;
    assert!(matches!(
        write_player(PesVersion::Pes14, &player, &mut record, schema.player),
        Err(CodecError::ValueTooWide { .. })
    ));
}

/// PES 2014 stores Sleeves and the shirttail in another order than the
/// model's (`stored_order`): Sleeves' stored 0 to 3 read as the model's
/// 1, 2, 0, 3 (Short, Long, Auto, the kept 3) and write back as stored,
/// and the shirttail's stored 0 reads as untucked. On PES 2015 the same
/// fields read as stored. A model value the order does not list,
/// Sleeves' 4, refuses the write.
#[test]
fn pes14_sleeves_and_shirttail_translate_through_the_stored_order() {
    let (_, appearance) = schema_for(PesVersion::Pes14)
        .appearance
        .as_ref()
        .expect("PES 2014 has an appearance record");
    let run_of = |wanted: PlayerField| {
        runs(appearance)
            .find(|(field, _, _)| *field == wanted)
            .map(|(_, offset, width)| (offset, width))
            .expect("the schema maps the field")
    };
    let (sleeves_at, sleeves_w) = run_of(PlayerField::Sleeves);
    for (stored, model) in [(0u32, 1u8), (1, 2), (2, 0), (3, 3)] {
        let mut record = vec![0u8; appearance.size];
        bits::write_bits(&mut record, sleeves_at, sleeves_w, stored);
        let mut player = PlayerEntry::default();
        read_player_into(PesVersion::Pes14, &mut player, &record, appearance).expect("decode");
        assert_eq!(player.appearance.sleeves, model, "stored {stored}");
        let mut written = vec![0u8; appearance.size];
        write_player(PesVersion::Pes14, &player, &mut written, appearance).expect("encode");
        assert_eq!(
            bits::read_bits(&written, sleeves_at, sleeves_w),
            stored,
            "model {model} writes stored {stored}"
        );
    }
    let (untucked_at, untucked_w) = run_of(PlayerField::Untucked);
    for (stored, model) in [(0u32, true), (1, false)] {
        let mut record = vec![0u8; appearance.size];
        bits::write_bits(&mut record, untucked_at, untucked_w, stored);
        let mut player = PlayerEntry::default();
        read_player_into(PesVersion::Pes14, &mut player, &record, appearance).expect("decode");
        assert_eq!(player.appearance.untucked, model, "stored {stored}");
        let mut written = vec![0u8; appearance.size];
        write_player(PesVersion::Pes14, &player, &mut written, appearance).expect("encode");
        assert_eq!(
            bits::read_bits(&written, untucked_at, untucked_w),
            stored,
            "model {model} writes stored {stored}"
        );
    }
    // The version decides, not the field: on PES 2015 Sleeves reads as
    // stored.
    let (_, appearance15) = schema_for(PesVersion::Pes15)
        .appearance
        .as_ref()
        .expect("PES 2015 has an appearance record");
    let (_, sleeves15_at, sleeves15_w) = runs(appearance15)
        .find(|(field, _, _)| *field == PlayerField::Sleeves)
        .expect("PES 2015 maps Sleeves");
    for stored in 0u32..=2 {
        let mut record = vec![0u8; appearance15.size];
        bits::write_bits(&mut record, sleeves15_at, sleeves15_w, stored);
        let mut player = PlayerEntry::default();
        read_player_into(PesVersion::Pes15, &mut player, &record, appearance15).expect("decode");
        assert_eq!(
            player.appearance.sleeves, stored as u8,
            "PES 2015 stored {stored} reads as stored"
        );
    }
    // A model value the order does not list refuses the write.
    let mut player = PlayerEntry::default();
    player.appearance.sleeves = 4;
    let mut record = vec![0u8; appearance.size];
    match write_player(PesVersion::Pes14, &player, &mut record, appearance) {
        Err(CodecError::OutsideStoredOrder { field, value }) => {
            assert_eq!((field.as_str(), value), ("Sleeves", 4));
        }
        other => panic!("expected CodecError::OutsideStoredOrder, got {other:?}"),
    }
}

/// PES 2014's gloves id, which the base game does not store: the Akko DLC
/// reads it from the hair accessory's colour, one six-bit run per channel —
/// red at 0x13:0, blue at 0x14:0, green at 0x50:3, its bits running into
/// 0x51:0 — the id's lowest bits first, 18 bits in all. A write changes
/// those runs alone and a read composes them back.
#[test]
fn pes14_gloves_id_splits_over_the_hair_accessory_colour_runs() {
    let (_, appearance) = schema_for(PesVersion::Pes14)
        .appearance
        .as_ref()
        .expect("PES 2014 has an appearance record");
    // The hide pair an FPC player carries, boots 55 and gloves 11, written
    // into a bare record first, reads back as the pair.
    let mut player = PlayerEntry::default();
    player.appearance.boots_id = 55;
    player.appearance.gloves_id = 11;
    let mut record = vec![0u8; appearance.size];
    write_player(PesVersion::Pes14, &player, &mut record, appearance).expect("encode");
    let mut back = PlayerEntry::default();
    read_player_into(PesVersion::Pes14, &mut back, &record, appearance).expect("decode");
    assert_eq!(
        (back.appearance.boots_id, back.appearance.gloves_id),
        (55, 11),
        "the hide pair reads back"
    );
    assert_eq!(
        (
            bits::read_bits(&record, 0x13 * 8, 6),
            bits::read_bits(&record, 0x14 * 8, 6),
            bits::read_bits(&record, 0x50 * 8 + 3, 6)
        ),
        (11, 0, 0),
        "id 11 sits in the red run alone"
    );

    // An id with all three runs non-zero: the spectacles' colour just
    // under the green run (0x50:0, three bits, which the schema does not
    // map) is set first, and a write must leave it — and every bit
    // outside the three runs — alone.
    bits::write_bits(&mut record, 0x50 * 8, 3, 5);
    let before = record.clone();
    player.appearance.gloves_id = 0x2A5A5;
    write_player(PesVersion::Pes14, &player, &mut record, appearance).expect("encode");
    for (byte, (&was, &now)) in before.iter().zip(&record).enumerate() {
        let run_mask = match byte {
            0x13 | 0x14 => 0x3F,
            0x50 => 0xF8,
            0x51 => 0x01,
            _ => 0x00,
        };
        assert_eq!(now & !run_mask, was & !run_mask, "byte {byte:#04x}");
    }
    assert_eq!(
        (
            bits::read_bits(&record, 0x13 * 8, 6),
            bits::read_bits(&record, 0x14 * 8, 6),
            bits::read_bits(&record, 0x50 * 8 + 3, 6)
        ),
        (
            0x2A5A5 & 0x3F,
            (0x2A5A5 >> 6) & 0x3F,
            (0x2A5A5 >> 12) & 0x3F
        ),
        "the id's bits, lowest first"
    );
    let mut reread = PlayerEntry::default();
    read_player_into(PesVersion::Pes14, &mut reread, &record, appearance).expect("redecode");
    assert_eq!(reread.appearance.gloves_id, 0x2A5A5);
}

#[test]
fn a_value_too_wide_for_its_run_is_refused() {
    let schema = crate::schema::RecordSchema {
        size: 2,
        fields: &[crate::schema::FieldSpec {
            field: PlayerField::Form,
            bit_offset: 0,
            bit_width: 3,
        }],
        split: &[],
        arrays: &[],
        texts: &[],
        ingame_face: None,
    };
    let mut player = PlayerEntry::default();
    player.stats.form = 8;
    let mut record = [0u8; 2];
    assert!(matches!(
        write_player(PesVersion::Pes21, &player, &mut record, &schema),
        Err(CodecError::ValueTooWide {
            value: 8,
            width: 3,
            ..
        })
    ));
}

/// `write_run`'s width check covers the roster writer too: a roster number
/// above the 8-bit run is `ValueTooWide`.
#[test]
fn a_roster_number_too_wide_for_its_run_is_refused() {
    let payload = payload(PesVersion::Pes17);
    let schema = schema_for(PesVersion::Pes17);
    let mut team = find_team(&payload, schema, 701);
    team.roster[0].number = 300;
    let mut rec = vec![0u8; schema.roster.size];
    assert!(matches!(
        write_roster(&team, &mut rec, schema.roster),
        Err(CodecError::ValueTooWide {
            value: 300,
            width: 8,
            ..
        })
    ));
}

/// A roster longer than the schema's slot count is refused rather than
/// silently cut; the same roster at the count still writes.
#[test]
fn a_roster_longer_than_the_schema_is_refused() {
    let payload = payload(PesVersion::Pes17);
    let schema = schema_for(PesVersion::Pes17);
    // PES 17's roster record holds 32 player slots.
    let slots = 32;
    let mut team = find_team(&payload, schema, 701);
    assert_eq!(team.roster.len(), slots, "the fixture fills every slot");
    team.roster.push(RosterSlot::default());
    let mut rec = vec![0u8; schema.roster.size];
    assert!(matches!(
        write_roster(&team, &mut rec, schema.roster),
        Err(CodecError::RosterTooLong { slots: s, got })
            if s == slots && got == slots + 1
    ));
    team.roster.pop();
    write_roster(&team, &mut rec, schema.roster).expect("the full roster writes");
}

/// The same check through `write_team`: a colour channel above its 6-bit
/// run is `ValueTooWide`.
#[test]
fn a_team_colour_too_wide_for_its_run_is_refused() {
    let payload = payload(PesVersion::Pes19);
    let schema = schema_for(PesVersion::Pes19);
    let mut team = find_team(&payload, schema, 100);
    team.colors.as_mut().expect("PES 19 teams have colours")[0].red = 200;
    let mut rec = vec![0u8; schema.team.size];
    assert!(matches!(
        write_team(&team, &mut rec, schema.team),
        Err(CodecError::ValueTooWide {
            value: 200,
            width: 6,
            ..
        })
    ));
}

/// A playing style or instruction the version cannot hold is refused
/// naming the version as the game does.
#[test]
fn a_style_or_instruction_refusal_names_the_version() {
    let err = crate::schema::playstyle::decode(PesVersion::Pes16, 16)
        .expect_err("16 is a hole in PES 16's list");
    assert_eq!(
        err.to_string(),
        "PES 2016: playing-style index 16 is not a value of its list"
    );
    let err = crate::schema::instruction::decode(PesVersion::Pes17, 0x0D)
        .expect_err("13 is past PES 17's table");
    assert_eq!(
        err.to_string(),
        "PES 2017: advanced-instruction value 13 is not a value of its table"
    );
}

/// Read and write share `tactics_runs`, so a round trip cannot see a wrong
/// offset in it. The fixture bytes at the layout's own offsets decode to
/// what `read_tactics_into` produced.
#[test]
fn tactics_bytes_sit_at_the_layouts_own_offsets() {
    // A team whose two same-preset formations differ at some slot of
    // preset 1 or 2: the preset and `formation` strides multiply into the
    // offset, so a wrong stride shows only where the bytes it would read
    // differ, and preset 0's offset has no preset stride in it. PES 17
    // then PES 19, the first difference found.
    let mut found = None;
    'versions: for version in [PesVersion::Pes17, PesVersion::Pes19] {
        let payload = payload(version);
        let schema = schema_for(version);
        for i in 0..count(&payload, &schema.tactics) {
            let rec = record(&payload, &schema.tactics, schema.tactic.size, i);
            let mut team = TeamEntry {
                id: tactics_id(rec, schema.tactic),
                ..TeamEntry::default()
            };
            read_tactics_into(&mut team, rec, schema.tactic).expect("tactics record decodes");
            for preset in 1..3 {
                for slot in 0..11 {
                    let a = team.tactics.presets[preset].formations[1].players[slot];
                    let b = team.tactics.presets[preset].formations[2].players[slot];
                    if a != b {
                        found = Some((version, schema, team, rec.to_vec(), preset, slot));
                        break 'versions;
                    }
                }
            }
        }
    }
    let (version, schema, team, rec, preset, slot) =
        found.expect("a fixture team whose formations 1 and 2 differ");
    let tactic = schema.tactic;
    let f = tactic
        .formations
        .as_ref()
        .expect("a version that maps formations");
    for (formation, want) in [
        (
            1u8,
            team.tactics.presets[preset].formations[1].players[slot],
        ),
        (2, team.tactics.presets[preset].formations[2].players[slot]),
    ] {
        // The eleven positions are a contiguous run; the y/x pairs follow
        // at the layout's own offsets.
        let base = f.base_bit
            + preset as u32 * tactic.preset_stride_bits
            + u32::from(formation) * f.formation_stride_bits;
        assert_eq!(
            bits::read_bits(&rec, base + slot as u32 * f.slot_stride_bits, 8) as u8,
            want.position,
            "{version:?} preset {preset} formation {formation} slot {slot} position"
        );
        assert_eq!(
            bits::read_bits(
                &rec,
                base + f.y_offset_bits + slot as u32 * f.pair_stride_bits,
                8
            ) as u8,
            want.y,
            "{version:?} preset {preset} formation {formation} slot {slot} y"
        );
        assert_eq!(
            bits::read_bits(
                &rec,
                base + f.x_offset_bits + slot as u32 * f.pair_stride_bits,
                8
            ) as u8,
            want.x,
            "{version:?} preset {preset} formation {formation} slot {slot} x"
        );
    }
    // The advanced instructions sit at the instruction layout's offsets,
    // per preset, side and index.
    let ins = tactic
        .instructions
        .as_ref()
        .expect("PES 17 stores instructions");
    // A wrong preset or side stride reads another preset's or side's
    // bytes, which only shows where those bytes differ.
    let presets = &team.tactics.presets;
    assert_ne!(
        presets[0].attack_instructions, presets[1].attack_instructions,
        "{version:?} team {}: presets 0 and 1 attack alike",
        team.id
    );
    assert_ne!(
        presets[0].attack_instructions, presets[0].defence_instructions,
        "{version:?} team {}: attack and defence alike",
        team.id
    );
    for preset in 0..3u8 {
        let preset_base = u32::from(preset) * tactic.preset_stride_bits;
        for (side, s, entries) in [
            (
                InstructionSide::Attack,
                0u32,
                team.tactics.presets[usize::from(preset)]
                    .attack_instructions
                    .expect("attack instructions"),
            ),
            (
                InstructionSide::Defence,
                1u32,
                team.tactics.presets[usize::from(preset)]
                    .defence_instructions
                    .expect("defence instructions"),
            ),
        ] {
            for (index, entry) in entries.iter().enumerate() {
                let at = preset_base
                    + ins.base_bit
                    + s * ins.side_stride_bits
                    + index as u32 * ins.index_stride_bits;
                assert_eq!(
                    bits::read_bits(&rec, at, 8) as u8,
                    entry.instruction,
                    "preset {preset} {side:?} instruction {index}"
                );
                assert_eq!(
                    bits::read_bits(&rec, at + ins.part_stride_bits, 8) as u8,
                    entry.player,
                    "preset {preset} {side:?} instruction {index} player"
                );
            }
        }
    }
}

/// The id a roster record carries, read through its schema.
fn roster_id(record: &[u8], schema: &RecordSchema<RosterField, TeamText>) -> u32 {
    let (_, offset, width) = runs(schema)
        .find(|(f, _, _)| *f == RosterField::TeamId)
        .expect("the roster schema has a TeamId run");
    bits::read_bits(record, offset, width)
}

/// The id a tactics record carries, read through its schema.
fn tactics_id(record: &[u8], schema: &TacticsSchema) -> u32 {
    let (_, offset, width) = schema
        .fields
        .iter()
        .map(|spec| (spec.field, spec.bit_offset, spec.bit_width))
        .find(|(f, _, _)| *f == TacticsField::TeamId)
        .expect("the tactics schema has a TeamId run");
    bits::read_bits(record, offset, width)
}

/// The id a team record carries.
fn team_id(record: &[u8], schema: &RecordSchema<TeamField, TeamText>) -> u32 {
    let (_, offset, width) = runs(schema)
        .find(|(f, _, _)| *f == TeamField::Id)
        .expect("the team schema has an Id run");
    bits::read_bits(record, offset, width)
}

/// The joined `TeamEntry` for `id`: team record plus its roster and
/// tactics records.
fn find_team(payload: &[u8], schema: &VersionSchema, id: u32) -> TeamEntry {
    for i in 0..count(payload, &schema.teams) {
        let rec = record(payload, &schema.teams, schema.team.size, i);
        let mut team = read_team(rec, schema.team).expect("team record decodes");
        if team.id != id {
            continue;
        }
        for j in 0..count(payload, &schema.rosters) {
            let rec = record(payload, &schema.rosters, schema.roster.size, j);
            if roster_id(rec, schema.roster) == id {
                read_roster_into(&mut team, rec, schema.roster).expect("roster record decodes");
                break;
            }
        }
        for j in 0..count(payload, &schema.tactics) {
            let rec = record(payload, &schema.tactics, schema.tactic.size, j);
            if tactics_id(rec, schema.tactic) == id {
                read_tactics_into(&mut team, rec, schema.tactic).expect("tactics record decodes");
                break;
            }
        }
        return team;
    }
    panic!("team {id} not in the save");
}

#[test]
fn every_team_roster_and_tactics_record_round_trips_byte_for_byte() {
    for version in FIXTURES {
        let payload = payload(version);
        let schema = schema_for(version);
        for i in 0..count(&payload, &schema.teams) {
            let rec = record(&payload, &schema.teams, schema.team.size, i);
            let team = read_team(rec, schema.team).expect("decode team");
            let mut written = rec.to_vec();
            write_team(&team, &mut written, schema.team).expect("encode team");
            assert_eq!(written, rec, "{version:?} team record {i}");
        }
        for i in 0..count(&payload, &schema.rosters) {
            let rec = record(&payload, &schema.rosters, schema.roster.size, i);
            let mut team = TeamEntry {
                id: roster_id(rec, schema.roster),
                ..TeamEntry::default()
            };
            read_roster_into(&mut team, rec, schema.roster).expect("decode roster");
            let mut written = rec.to_vec();
            write_roster(&team, &mut written, schema.roster).expect("encode roster");
            assert_eq!(written, rec, "{version:?} roster record {i}");
        }
        for i in 0..count(&payload, &schema.tactics) {
            let rec = record(&payload, &schema.tactics, schema.tactic.size, i);
            let mut team = TeamEntry {
                id: tactics_id(rec, schema.tactic),
                ..TeamEntry::default()
            };
            read_tactics_into(&mut team, rec, schema.tactic).expect("decode tactics");
            let mut written = rec.to_vec();
            write_tactics(&team, &mut written, schema.tactic).expect("encode tactics");
            assert_eq!(written, rec, "{version:?} tactics record {i}");
        }
    }
}

#[test]
fn team_roster_and_tactics_sections_list_the_same_teams() {
    let counts = [478usize, 220, 220, 220, 202, 346, 220, 220];
    for (i, version) in FIXTURES.iter().enumerate() {
        let payload = payload(*version);
        let schema = schema_for(*version);
        assert_eq!(
            count(&payload, &schema.teams),
            counts[i],
            "{version:?} team count"
        );
        assert_eq!(count(&payload, &schema.rosters), counts[i]);
        assert_eq!(count(&payload, &schema.tactics), counts[i]);
        // The three sections must list the same ids in the same order.
        for j in 0..counts[i] {
            let team_rec = record(&payload, &schema.teams, schema.team.size, j);
            let roster_rec = record(&payload, &schema.rosters, schema.roster.size, j);
            let tactics_rec = record(&payload, &schema.tactics, schema.tactic.size, j);
            let id = team_id(team_rec, schema.team);
            assert_eq!(
                roster_id(roster_rec, schema.roster),
                id,
                "{version:?} slot {j}"
            );
            assert_eq!(
                tactics_id(tactics_rec, schema.tactic),
                id,
                "{version:?} slot {j}"
            );
            // The joined entry round-trips through all three writers.
            let mut team = read_team(team_rec, schema.team).expect("decode team");
            read_roster_into(&mut team, roster_rec, schema.roster).expect("decode roster");
            read_tactics_into(&mut team, tactics_rec, schema.tactic).expect("decode tactics");
            let mut w_team = team_rec.to_vec();
            let mut w_roster = roster_rec.to_vec();
            let mut w_tactics = tactics_rec.to_vec();
            write_team(&team, &mut w_team, schema.team).expect("encode team");
            write_roster(&team, &mut w_roster, schema.roster).expect("encode roster");
            write_tactics(&team, &mut w_tactics, schema.tactic).expect("encode tactics");
            assert_eq!(w_team, team_rec, "{version:?} team record {j}");
            assert_eq!(w_roster, roster_rec, "{version:?} roster record {j}");
            assert_eq!(w_tactics, tactics_rec, "{version:?} tactics record {j}");
        }
        // A roster or tactics record against the wrong team is refused.
        let rec = record(&payload, &schema.rosters, schema.roster.size, 0);
        let mut team = TeamEntry {
            id: roster_id(rec, schema.roster) + 1,
            ..TeamEntry::default()
        };
        assert!(matches!(
            read_roster_into(&mut team, rec, schema.roster),
            Err(CodecError::TeamIdMismatch { .. })
        ));
        let rec = record(&payload, &schema.tactics, schema.tactic.size, 0);
        assert!(matches!(
            read_tactics_into(&mut team, rec, schema.tactic),
            Err(CodecError::TeamIdMismatch { .. })
        ));
    }
}

#[test]
fn team_701_and_team_100_literals() {
    // Team 701's name and short name on the five versions that carry it.
    for version in [
        PesVersion::Pes15,
        PesVersion::Pes16,
        PesVersion::Pes17,
        PesVersion::Pes18,
        PesVersion::Pes21,
    ] {
        let payload = payload(version);
        let team = find_team(&payload, schema_for(version), 701);
        assert_eq!(team.name, "/3/", "{version:?} team 701 name");
        assert_eq!(team.short_name, "3", "{version:?} team 701 short name");
    }
    // PES 17/18 team 701 colours.
    for version in [PesVersion::Pes17, PesVersion::Pes18] {
        let payload = payload(version);
        let team = find_team(&payload, schema_for(version), 701);
        let colors = team.colors.expect("PES 17/18 teams have colours");
        assert_eq!(
            [
                (colors[0].red, colors[0].green, colors[0].blue),
                (colors[1].red, colors[1].green, colors[1].blue)
            ],
            [(63, 42, 16), (0, 0, 0)],
            "{version:?} team 701 colours"
        );
    }
    // PES 19 team 100.
    let team = find_team(
        &payload(PesVersion::Pes19),
        schema_for(PesVersion::Pes19),
        100,
    );
    assert_eq!(team.name, "MAN RED");
    assert_eq!(team.short_name, "MAN");
    assert_eq!(team.manager_id, Some(362381));
    assert_eq!(team.stadium_id, Some(46));
    let colors = team.colors.expect("PES 19 teams have colours");
    assert_eq!(
        [
            (colors[0].red, colors[0].green, colors[0].blue),
            (colors[1].red, colors[1].green, colors[1].blue)
        ],
        [(10, 10, 10), (10, 10, 10)]
    );
    assert_eq!(team.roster.len(), 40);
    assert_eq!(
        team.roster[..4]
            .iter()
            .map(|s| s.player_id)
            .collect::<Vec<_>>(),
        [0, 0, 0, 0]
    );
    // PES 21 team 701 ids.
    let team = find_team(
        &payload(PesVersion::Pes21),
        schema_for(PesVersion::Pes21),
        701,
    );
    assert_eq!(team.manager_id, Some(701));
    assert_eq!(team.stadium_id, Some(29));
    assert_eq!(team.roster.len(), 40);
    assert_eq!(
        team.roster[..4]
            .iter()
            .map(|s| (s.player_id, s.number))
            .collect::<Vec<_>>(),
        [(70101, 1), (70102, 2), (70103, 3), (70104, 4)]
    );
    // PES 15 team 701 roster.
    let team = find_team(
        &payload(PesVersion::Pes15),
        schema_for(PesVersion::Pes15),
        701,
    );
    assert_eq!(team.roster.len(), 32);
    assert_eq!(
        team.roster[..4]
            .iter()
            .map(|s| (s.player_id, s.number))
            .collect::<Vec<_>>(),
        [(70101, 1), (70102, 2), (70103, 3), (70104, 4)]
    );
    // PES 17 team 701 kit slot 0.
    let team = find_team(
        &payload(PesVersion::Pes17),
        schema_for(PesVersion::Pes17),
        701,
    );
    let kit = team.kit_slots.expect("PES 17 teams have kit slots")[0];
    assert_eq!(kit.number, 0);
    assert_eq!(kit.binding, 701 * 0x40);
}

#[test]
fn a_team_text_filling_its_field_writes_with_no_nul() {
    let payload = payload(PesVersion::Pes19);
    let schema = schema_for(PesVersion::Pes19);
    let record = record(&payload, &schema.teams, schema.team.size, 0);
    let mut team = read_team(record, schema.team).expect("decode team");
    let mut written = record.to_vec();

    // A team name of exactly the field's length writes unterminated and
    // reads back; one byte more is refused.
    let name = schema
        .team
        .texts
        .iter()
        .find(|t| t.text == TeamText::Name)
        .expect("name spec");
    team.name = "n".repeat(name.len as usize);
    write_team(&team, &mut written, schema.team).expect("encode");
    let at = name.byte_offset as usize;
    assert_eq!(&written[at..at + name.len as usize], team.name.as_bytes());
    let reread = read_team(&written, schema.team).expect("redecode");
    assert_eq!(reread.name, team.name);
    team.name = "n".repeat(name.len as usize + 1);
    assert!(matches!(
        write_team(&team, &mut written, schema.team),
        Err(CodecError::Text { .. })
    ));
}

#[test]
fn tactics_literals() {
    // PES 17 team 701.
    let team = find_team(
        &payload(PesVersion::Pes17),
        schema_for(PesVersion::Pes17),
        701,
    );
    let formation = &team.tactics.presets[0].formations[0];
    assert_eq!(
        formation
            .players
            .iter()
            .map(|s| s.position)
            .collect::<Vec<_>>(),
        [0, 1, 1, 3, 2, 4, 4, 7, 6, 8, 12]
    );
    assert_eq!(
        formation.players.iter().map(|s| s.x).collect::<Vec<_>>(),
        [52, 63, 41, 89, 15, 64, 40, 86, 18, 52, 52]
    );
    assert_eq!(team.tactics.presets[0].sliders.support_range, 5);
    assert_eq!(team.tactics.set_pieces.captain, 0);
    assert_eq!(
        team.tactics.starting_eleven,
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    );
    // PES 15 team 701.
    let team = find_team(
        &payload(PesVersion::Pes15),
        schema_for(PesVersion::Pes15),
        701,
    );
    assert_eq!(
        team.tactics.presets[0].formations[0]
            .players
            .iter()
            .map(|s| s.position)
            .collect::<Vec<_>>(),
        [0, 1, 1, 3, 2, 4, 5, 8, 11, 11, 12]
    );
    assert_eq!(team.tactics.presets[0].sliders.support_range, 3);
    assert_eq!(team.tactics.set_pieces.captain, 10);
    assert_eq!(
        team.tactics.starting_eleven,
        [0, 3, 2, 4, 1, 5, 6, 7, 8, 9, 10]
    );
    assert!(team.tactics.presets[0].attack_instructions.is_none());
    assert!(team.tactics.presets[0].defence_instructions.is_none());
    assert_eq!(team.tactics.auto.substitution, Some(0));
    assert_eq!(team.tactics.auto.offside_trap, Some(false));
    assert_eq!(team.tactics.auto.preset_change, Some(false));
    assert!(team.tactics.auto.attack_defence_levels.is_none());
    // PES 19 team 100: every taker and starter unset.
    let team = find_team(
        &payload(PesVersion::Pes19),
        schema_for(PesVersion::Pes19),
        100,
    );
    assert_eq!(team.tactics.set_pieces.captain, 255);
    assert_eq!(team.tactics.starting_eleven, [255; 11]);
}

/// PES 2014's first tactics record (team 1, IRELAND): the takers, the
/// captain, the auto switches, the lineup, Combination Play's slots and,
/// per preset, the Game Plan record's Strategy and sliders and the one
/// formation. Support Range reads as the game shows it (stored 12 is
/// shown 8), and formation slot 0 has a place but no position byte.
#[test]
fn pes14_tactics_literals() {
    let payload = payload(PesVersion::Pes14);
    let schema = schema_for(PesVersion::Pes14);
    let rec = record(&payload, &schema.tactics, schema.tactic.size, 0);
    assert_eq!(tactics_id(rec, schema.tactic), 1, "team 1's record first");
    let mut team = TeamEntry {
        id: 1,
        ..TeamEntry::default()
    };
    read_tactics_into(&mut team, rec, schema.tactic).expect("decode tactics");
    let tactics = &team.tactics;
    assert_eq!(tactics.set_pieces.captain, 10);
    assert_eq!(
        [
            tactics.set_pieces.free_kick_long,
            tactics.set_pieces.free_kick_short,
            tactics.set_pieces.free_kick_second,
            tactics.set_pieces.corner_left,
            tactics.set_pieces.corner_right,
            tactics.set_pieces.penalty,
        ],
        [6, 6, 7, 6, 6, 8]
    );
    assert_eq!(tactics.players_to_join_attack, [255, 255, 255]);
    assert_eq!(tactics.auto.substitution, Some(0));
    assert_eq!(tactics.auto.offside_trap, Some(false));
    assert_eq!(tactics.auto.preset_change, Some(false));
    assert_eq!(tactics.auto.combination_play, Some(false));
    assert_eq!(
        tactics.combination,
        Some([
            CombinationSlot {
                area: 2,
                strategy: 10
            },
            CombinationSlot {
                area: 6,
                strategy: 1
            },
            CombinationSlot {
                area: 8,
                strategy: 5
            },
        ])
    );
    assert_eq!(tactics.starting_eleven, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(
        tactics.bench_order,
        [
            11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31
        ]
    );
    for (index, strategy) in [(0usize, 1u8), (1, 3), (2, 4)] {
        let preset = &tactics.presets[index];
        assert_eq!(
            preset.game_plan,
            Some(GamePlan {
                strategy,
                sliders: [13, 8, 9, 9, 8, 12, 10, 10],
            }),
            "preset {}'s Game Plan",
            index + 1
        );
        let formation = &preset.formations[0];
        assert_eq!(
            formation.players[0],
            FormationSlot {
                position: 0,
                x: 52,
                y: 4
            },
            "the keeper's place, his role byte not stored"
        );
        assert_eq!(
            formation.players[1..]
                .iter()
                .map(|s| (s.position, s.x, s.y))
                .collect::<Vec<_>>(),
            [
                (1, 63, 9),
                (1, 41, 9),
                (3, 86, 11),
                (2, 17, 11),
                (5, 63, 25),
                (5, 44, 24),
                (7, 84, 29),
                (6, 20, 30),
                (11, 43, 38),
                (12, 63, 40),
            ],
            "preset {}'s outfield roles and places",
            index + 1
        );
        assert_eq!(preset.formations[1], Formation::default());
        assert_eq!(preset.formations[2], Formation::default());
    }
}

/// A PES 2014 tactics record written with every mapped field set to a
/// value the record does not hold: each lands at its offset — a preset-2
/// Strategy and a formation slot prove the preset stride — Support Range
/// stores 20 less the value shown, and every other byte stays as read.
#[test]
fn pes14_tactics_write_lands_each_field_at_its_offset() {
    let payload = payload(PesVersion::Pes14);
    let schema = schema_for(PesVersion::Pes14);
    let rec = record(&payload, &schema.tactics, schema.tactic.size, 0);
    let mut team = TeamEntry {
        id: 1,
        ..TeamEntry::default()
    };
    read_tactics_into(&mut team, rec, schema.tactic).expect("decode tactics");
    let tactics = &mut team.tactics;
    tactics.set_pieces.free_kick_long = 21;
    tactics.set_pieces.free_kick_short = 22;
    tactics.set_pieces.free_kick_second = 23;
    tactics.set_pieces.corner_left = 24;
    tactics.set_pieces.corner_right = 25;
    tactics.set_pieces.penalty = 26;
    tactics.players_to_join_attack = [3, 7, 12];
    tactics.auto.substitution = Some(2);
    tactics.auto.offside_trap = Some(true);
    tactics.auto.preset_change = Some(true);
    tactics.auto.combination_play = Some(true);
    tactics.combination = Some([
        CombinationSlot {
            area: 1,
            strategy: 2,
        },
        CombinationSlot {
            area: 3,
            strategy: 4,
        },
        CombinationSlot {
            area: 5,
            strategy: 6,
        },
    ]);
    tactics.starting_eleven = [30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40];
    for (i, slot) in tactics.bench_order.iter_mut().enumerate() {
        *slot = 50 + i as u8;
    }
    let plan = tactics.presets[0].game_plan.get_or_insert_default();
    assert_ne!(plan.strategy, 0, "premise: another strategy");
    plan.strategy = 0;
    assert_ne!(
        plan.sliders,
        [0, 5, 1, 2, 3, 4, 6, 7],
        "premise: other sliders"
    );
    plan.sliders = [0, 5, 1, 2, 3, 4, 6, 7];
    // Preset 2 proves the stride: another Strategy, and formation slot 5
    // at another place. Slot 0's position is set in the model too: its
    // byte must not move, the keeper's role being unwritten.
    let second = &mut tactics.presets[1];
    second.game_plan.get_or_insert_default().strategy = 2;
    second.formations[0].players[5] = FormationSlot {
        position: 8,
        x: 70,
        y: 30,
    };
    tactics.presets[0].formations[0].players[0].position = 7;
    let mut written = rec.to_vec();
    write_tactics(&team, &mut written, schema.tactic).expect("encode tactics");
    let changed: Vec<usize> = (0..rec.len()).filter(|&i| written[i] != rec[i]).collect();
    let mut expected: Vec<usize> = vec![0x5A, 0xC6];
    expected.extend(0x5B..=0x62);
    expected.extend([0x82, 0xAE, 0xB0]);
    expected.extend((0..11).map(|i| 0x146 + 4 * i));
    expected.extend((0..21).map(|i| 0x172 + 4 * i));
    expected.extend(0x1C6..=0x1CB);
    expected.extend(0x1CD..=0x1D3);
    expected.extend(0x1D6..=0x1DB);
    expected.sort_unstable();
    assert_eq!(changed, expected);
    // Each landed at its offset; Support Range stored 20 less the shown 5.
    assert_eq!(written[0x5A], 0, "preset 1's Strategy");
    assert_eq!(written[0x5C], 15, "Support Range: shown 5 stored as 15");
    assert_eq!(written[0xC6], 2, "preset 2's Strategy");
    assert_eq!(
        (written[0x82], written[0xAE], written[0xB0]),
        (8, 30, 70),
        "preset 2's formation slot 5"
    );
    assert_eq!(written[0x1C6], 21, "the long free kick taker");
    assert_eq!(written[0x1D0], 2, "the substitution setting");
    assert_eq!(written[0x1D3], 1, "Combination Play's switch");
    assert_eq!(
        (0..6).map(|i| written[0x1D6 + i]).collect::<Vec<_>>(),
        [1, 2, 3, 4, 5, 6],
        "the combination slots' area then strategy bytes"
    );
    assert_eq!(written[0x146], 30, "the lineup's first entry");
    assert_eq!(written[0x172], 50, "the bench order's first entry");
}

/// `has` follows the formation layout's `count` and `first_role_slot`:
/// PES 2014 stores one formation whose slot 0 has a place but no role
/// byte; PES 2015 stores three with every slot's role, as before.
#[test]
fn tactics_has_follows_the_formation_layouts_bounds() {
    let field = |formation, slot, part| TacticsField::Formation {
        preset: 0,
        formation,
        slot,
        part,
    };
    let pes14 = schema_for(PesVersion::Pes14).tactic;
    assert!(
        !pes14.has(field(1, 1, FormationPart::Position)),
        "PES 2014 stores one formation"
    );
    assert!(
        !pes14.has(field(0, 0, FormationPart::Position)),
        "the keeper's role is not stored"
    );
    for part in [FormationPart::X, FormationPart::Y] {
        assert!(pes14.has(field(0, 0, part)), "the keeper's place is stored");
    }
    assert!(pes14.has(field(0, 1, FormationPart::Position)));
    let pes15 = schema_for(PesVersion::Pes15).tactic;
    for formation in 0..3 {
        for slot in 0..11 {
            for part in [FormationPart::Position, FormationPart::X, FormationPart::Y] {
                assert!(
                    pes15.has(field(formation, slot, part)),
                    "PES 2015 maps formation {formation} slot {slot} {part:?}"
                );
            }
        }
    }
}

/// A Game Plan Support Range over 20 has no stored value: the write is
/// refused rather than wrapped.
#[test]
fn a_game_plan_support_range_over_20_is_refused() {
    let payload = payload(PesVersion::Pes14);
    let schema = schema_for(PesVersion::Pes14);
    let rec = record(&payload, &schema.tactics, schema.tactic.size, 0);
    let mut team = TeamEntry {
        id: 1,
        ..TeamEntry::default()
    };
    read_tactics_into(&mut team, rec, schema.tactic).expect("decode tactics");
    team.tactics.presets[0].game_plan = Some(GamePlan {
        strategy: 0,
        sliders: [0, 21, 0, 0, 0, 0, 0, 0],
    });
    let mut written = rec.to_vec();
    match write_tactics(&team, &mut written, schema.tactic) {
        Err(CodecError::OutsideStoredOrder { field, value }) => {
            assert!(field.contains("GamePlanSlider(1)"), "{field}");
            assert_eq!(value, 21);
        }
        other => panic!("expected CodecError::OutsideStoredOrder, got {other:?}"),
    }
}

/// The man-marking runs, measured in-game: each preset's
/// eleven own-formation-slot bytes, each the marked opponent's formation
/// slot or 0xFF for none.
#[test]
fn man_marking_literals() {
    let team = find_team(
        &payload(PesVersion::Pes15),
        schema_for(PesVersion::Pes15),
        737,
    );
    assert_eq!(
        team.tactics.presets[0].man_marking,
        [
            0xFF, 0xFF, 0xFF, 0xFF, 0x0A, 0xFF, 0xFF, 0x02, 0x04, 0xFF, 0x05
        ]
    );
    assert_eq!(
        team.tactics.presets[1].man_marking,
        [
            0xFF, 0xFF, 0xFF, 0x08, 0x07, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF
        ]
    );
    assert_eq!(team.tactics.presets[2].man_marking, [0xFF; 11]);
    let team = find_team(
        &payload(PesVersion::Pes16),
        schema_for(PesVersion::Pes16),
        710,
    );
    assert_eq!(
        team.tactics.presets[0].man_marking,
        [
            0xFF, 0x09, 0xFF, 0xFF, 0x08, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF
        ]
    );
    assert_eq!(team.tactics.presets[1].man_marking, [0xFF; 11]);
    assert_eq!(team.tactics.presets[2].man_marking, [0xFF; 11]);
    let team = find_team(
        &payload(PesVersion::Pes17),
        schema_for(PesVersion::Pes17),
        737,
    );
    for preset in &team.tactics.presets[..2] {
        assert_eq!(
            preset.man_marking,
            [
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0A, 0xFF, 0xFF, 0xFF, 0xFF
            ]
        );
    }
    assert_eq!(team.tactics.presets[2].man_marking, [0xFF; 11]);
    let team = find_team(
        &payload(PesVersion::Pes17),
        schema_for(PesVersion::Pes17),
        771,
    );
    for preset in [&team.tactics.presets[0], &team.tactics.presets[2]] {
        assert_eq!(
            preset.man_marking,
            [
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0A, 0xFF, 0xFF, 0xFF
            ]
        );
    }
    assert_eq!(team.tactics.presets[1].man_marking, [0xFF; 11]);
    let team = find_team(
        &payload(PesVersion::Pes18),
        schema_for(PesVersion::Pes18),
        764,
    );
    for preset in &team.tactics.presets {
        assert_eq!(
            preset.man_marking,
            [
                0xFF, 0x0A, 0xFF, 0xFF, 0xFF, 0xFF, 0x07, 0xFF, 0xFF, 0xFF, 0xFF
            ]
        );
    }
    let team = find_team(
        &payload(PesVersion::Pes19),
        schema_for(PesVersion::Pes19),
        706,
    );
    for preset in &team.tactics.presets {
        assert_eq!(
            preset.man_marking,
            [
                0xFF, 0xFF, 0xFF, 0xFF, 0x09, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF
            ]
        );
    }
    // PES 20 and 21: no fixture team marks anyone.
    for version in [PesVersion::Pes20, PesVersion::Pes21] {
        let payload = payload(version);
        let schema = schema_for(version);
        for i in 0..count(&payload, &schema.tactics) {
            let rec = record(&payload, &schema.tactics, schema.tactic.size, i);
            let mut team = TeamEntry {
                id: tactics_id(rec, schema.tactic),
                ..TeamEntry::default()
            };
            read_tactics_into(&mut team, rec, schema.tactic).expect("decode tactics");
            for preset in &team.tactics.presets {
                assert_eq!(preset.man_marking, [0xFF; 11], "{version:?} tactics {i}");
            }
        }
    }
}

/// PES 15's three auto flags, measured in-game at the record's +0x1FE to
/// +0x200. The attack/defence-level flag stays PES 17+.
#[test]
fn pes15_auto_flags_literals() {
    let team = find_team(
        &payload(PesVersion::Pes15),
        schema_for(PesVersion::Pes15),
        754,
    );
    assert_eq!(team.tactics.auto.substitution, Some(0));
    assert_eq!(team.tactics.auto.offside_trap, Some(false));
    assert_eq!(team.tactics.auto.preset_change, Some(true));
    let team = find_team(
        &payload(PesVersion::Pes15),
        schema_for(PesVersion::Pes15),
        771,
    );
    assert_eq!(team.tactics.auto.offside_trap, Some(true));
    assert_eq!(team.tactics.auto.preset_change, Some(false));
    let team = find_team(
        &payload(PesVersion::Pes15),
        schema_for(PesVersion::Pes15),
        701,
    );
    assert_eq!(team.tactics.auto.substitution, Some(0));
    assert_eq!(team.tactics.auto.offside_trap, Some(false));
    assert_eq!(team.tactics.auto.preset_change, Some(false));
    assert!(team.tactics.auto.attack_defence_levels.is_none());
}

/// A man-marking byte (and a PES 15 auto flag) writes into its measured run
/// and reads back, no other record byte touched.
#[test]
fn man_marking_and_pes15_auto_flags_write_and_reread() {
    // Team 701's tactics record, preset 2's man-marking slot 3 -> 5; the
    // differing bytes are exactly the run's byte 3 (PES 15 +0xF0, PES 21
    // +0x138) plus the PES 15 offside-trap byte at +0x1FF.
    for (version, expected) in [
        (PesVersion::Pes15, vec![0xF0, 0x1FF]),
        (PesVersion::Pes21, vec![0x138]),
    ] {
        let payload = payload(version);
        let schema = schema_for(version);
        let rec = (0..count(&payload, &schema.tactics))
            .map(|i| record(&payload, &schema.tactics, schema.tactic.size, i))
            .find(|rec| tactics_id(rec, schema.tactic) == 701)
            .expect("team 701's tactics record");
        let mut team = TeamEntry {
            id: 701,
            ..TeamEntry::default()
        };
        read_tactics_into(&mut team, rec, schema.tactic).expect("decode tactics");
        assert_ne!(team.tactics.presets[1].man_marking[3], 5);
        team.tactics.presets[1].man_marking[3] = 5;
        if version == PesVersion::Pes15 {
            assert_eq!(team.tactics.auto.offside_trap, Some(false));
            team.tactics.auto.offside_trap = Some(true);
        }
        let mut written = rec.to_vec();
        write_tactics(&team, &mut written, schema.tactic).expect("encode tactics");
        let changed: Vec<usize> = (0..rec.len()).filter(|&i| written[i] != rec[i]).collect();
        assert_eq!(changed, expected, "{version:?} differing bytes");
        let mut back = TeamEntry {
            id: 701,
            ..TeamEntry::default()
        };
        read_tactics_into(&mut back, &written, schema.tactic).expect("redecode tactics");
        assert_eq!(back.tactics.presets[1].man_marking[3], 5, "{version:?}");
        if version == PesVersion::Pes15 {
            assert_eq!(back.tactics.auto.offside_trap, Some(true));
        }
    }
}

/// A tactics record written through a schema with no man-marking layout
/// keeps every byte outside the fields it maps: PES 2014's, which maps
/// the Game Plan but no man marking, changes only the captain's byte.
#[test]
fn a_schema_without_man_marking_keeps_its_bytes() {
    let payload = payload(PesVersion::Pes14);
    let schema = schema_for(PesVersion::Pes14);
    assert!(
        !schema
            .tactic
            .has(TacticsField::ManMarking { preset: 0, slot: 0 }),
        "PES 2014 maps no man marking"
    );
    // The shared `tactics_runs` emits no run for a `None` layout, so the
    // unmapped bytes are neither read nor written.
    assert!(
        tactics_runs(schema.tactic)
            .iter()
            .all(|(field, _, _)| !matches!(field, TacticsField::ManMarking { .. })),
        "no man-marking run is emitted"
    );
    let rec = record(&payload, &schema.tactics, schema.tactic.size, 0);
    let mut team = TeamEntry {
        id: tactics_id(rec, schema.tactic),
        ..TeamEntry::default()
    };
    read_tactics_into(&mut team, rec, schema.tactic).expect("decode tactics");
    assert_ne!(
        team.tactics.set_pieces.captain, 7,
        "premise: another captain"
    );
    team.tactics.set_pieces.captain = 7;
    let mut written = rec.to_vec();
    write_tactics(&team, &mut written, schema.tactic).expect("encode tactics");
    let changed: Vec<usize> = (0..rec.len()).filter(|&i| written[i] != rec[i]).collect();
    // The captain is byte +0x1CC of the 502-byte record.
    assert_eq!(changed, [0x1CC], "only the captain's byte changed");
}

#[test]
fn the_tactics_census_matches_the_measured_ranges() {
    for version in FIXTURES {
        let payload = payload(version);
        let schema = schema_for(version);
        for i in 0..count(&payload, &schema.tactics) {
            let rec = record(&payload, &schema.tactics, schema.tactic.size, i);
            let mut team = TeamEntry {
                id: tactics_id(rec, schema.tactic),
                ..TeamEntry::default()
            };
            // Every boolean byte decodes (0 or 1) or the read fails.
            read_tactics_into(&mut team, rec, schema.tactic).expect("decode tactics");
            for preset in &team.tactics.presets {
                // The model checks read only fields the version's preset
                // table maps (PES 2014's maps none of these).
                for (field, value) in [
                    (PresetField::SupportRange, preset.sliders.support_range),
                    (PresetField::DefensiveLine, preset.sliders.defensive_line),
                    (PresetField::Compactness, preset.sliders.compactness),
                ] {
                    if schema.tactic.has_preset(field) {
                        assert!(
                            (1..=10).contains(&value),
                            "{version:?} tactics {i} slider {value}"
                        );
                    }
                }
                for (field, value) in [
                    (
                        PresetField::NumbersInAttack,
                        preset.sliders.numbers_in_attack,
                    ),
                    (
                        PresetField::NumbersInDefence,
                        preset.sliders.numbers_in_defence,
                    ),
                ] {
                    if schema.tactic.has_preset(field) {
                        assert!(
                            (1..=3).contains(&value),
                            "{version:?} tactics {i} numbers {value}"
                        );
                    }
                }
            }
        }
    }
    // A synthetic boolean byte of 2 is exactly one NotBoolean.
    let payload = payload(PesVersion::Pes17);
    let schema = schema_for(PesVersion::Pes17);
    let mut rec = record(&payload, &schema.tactics, schema.tactic.size, 0).to_vec();
    let (field, offset, _) = tactics_runs(schema.tactic)
        .into_iter()
        .find(|(f, _, _)| {
            matches!(
                f,
                TacticsField::Preset {
                    preset: 0,
                    field: crate::schema::fields::PresetField::AttackingStyle,
                }
            )
        })
        .expect("a style run");
    assert_eq!(offset % 8, 0, "style runs are byte-aligned");
    rec[offset as usize / 8] = 2;
    let mut team = TeamEntry {
        id: tactics_id(&rec, schema.tactic),
        ..TeamEntry::default()
    };
    match read_tactics_into(&mut team, &rec, schema.tactic) {
        Err(CodecError::NotBoolean { field: name, value }) => {
            assert!(name.contains("AttackingStyle"), "{name}");
            assert_eq!(value, 2);
        }
        other => panic!("expected CodecError::NotBoolean for {field:?}, got {other:?}"),
    }
}
