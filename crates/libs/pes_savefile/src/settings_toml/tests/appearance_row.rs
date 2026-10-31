use super::*;

/// A save player's settings give back exactly his row: the id, then the
/// record's 56 bytes at the row offset (128 on PES 18/19, 252 on PES 20/21,
/// written literally so the check does not lean on `row_offset`). The
/// completeness test gives every bit of that span one carrier in
/// `PlayerSettings`, so nothing can be lost. `from_player` is the
/// settings' construction; where it refuses (a stored value the key table
/// cannot express) the same settings come through `appearance_from`, the
/// stored-ranges reader that carries such values verbatim. No fixture
/// player needs it since PES 15's caps joined the ranges, so one built
/// player does: the first one with `neck_length` stored 15, which its
/// field's 4 bits hold and its key's -7 to 7 (stored 0 to 14) does not. The
/// field is the row's first byte, so a row that dropped or reset the value
/// would differ from the record. The row's run differs from the record's
/// only in its unused bits, which take the games' values.
#[test]
fn a_save_players_settings_reproduce_his_appearance_row() {
    for (version, offset) in [
        (PesVersion::Pes18, 128usize),
        (PesVersion::Pes19, 128),
        (PesVersion::Pes20, 252),
        (PesVersion::Pes21, 252),
    ] {
        let payload = payload(version);
        let schema = schema_for(version);
        // The run inside the row: the record's run offset minus the row
        // offset, from the schema rather than a fixed byte 10; asserted below.
        let run = schema.player.ingame_face.as_ref().expect("Fox has a run");
        let run_in_row = run.byte_offset as usize - offset;
        assert_eq!(run_in_row, 10, "{version:?}: the run's row byte");
        let mut records: Vec<Vec<u8>> = (0..count(&payload, &schema.players))
            .map(|i| record(&payload, &schema.players, schema.player.size, i).to_vec())
            .filter(|rec| read_player(rec, schema.player).expect("decode").id != 0)
            .collect();
        assert!(!SettingKey::NeckLength.spec().kind.accepts_stored(15));
        let mut built = records[0].clone();
        let mut player = read_player(&built, schema.player).expect("decode");
        assert_ne!(player.appearance.neck_length, 15, "{version:?}");
        player.appearance.neck_length = 15;
        write_player(&player, &mut built, schema.player).expect("encode");
        assert_eq!(
            built[offset] & 0x0F,
            15,
            "{version:?}: the row's first byte"
        );
        records.push(built);
        let mut hair_seen = false;
        let mut out_of_range_players = 0usize;
        for rec in &records {
            let player = read_player(rec, schema.player).expect("decode");
            let settings = match PlayerSettings::from_player(&player) {
                Ok(settings) => settings,
                Err(SettingsError::OutOfRange { .. }) => {
                    // A stored value the key table cannot express: the
                    // stored-ranges reader carries it verbatim.
                    out_of_range_players += 1;
                    PlayerSettings {
                        appearance: appearance_from(&player).expect("stored-ranges read"),
                        ..PlayerSettings::default()
                    }
                }
                Err(err) => panic!("{version:?} player {}: {err:?}", player.id),
            };
            let row = settings.appearance_row(player.id, version).expect("a row");
            let mut expected = [0u8; 60];
            expected[..4].copy_from_slice(&player.id.to_le_bytes());
            expected[4..].copy_from_slice(&rec[offset..offset + 56]);
            let mut written = player.appearance.ingame_face.clone();
            written.write_unused();
            // The row ends inside the run: its last 46 bytes are the run's
            // first 46.
            let run_at = 4 + run_in_row;
            let in_row = expected.len() - run_at;
            expected[run_at..].copy_from_slice(&written.bytes()[..in_row]);
            if let Some(at) = row.iter().zip(&expected).position(|(a, b)| a != b) {
                panic!(
                    "{version:?} player {}: row byte {at} is {:#04x}, the record's is {:#04x}",
                    player.id, row[at], expected[at]
                );
            }
            hair_seen |= SettingKey::ALL.iter().any(|key| {
                key.spec().table == KeyTable::Hair
                    && held_on(*key, version)
                    && settings.get(*key) != key.spec().default
            });
        }
        assert!(
            hair_seen,
            "{version:?}: every player's hair is the default, the rows prove little"
        );
        // The built player's 15 is the reason `appearance_row` skips
        // `apply`'s checks: without it this test would stop proving it.
        assert!(
            out_of_range_players > 0,
            "{version:?}: no player's settings took the OutOfRange fallback"
        );
    }
}

/// TC-BIN-28's "the template's defaults": a default settings' row, written
/// into a PES 21 record of 0xFF bytes at the row offset and decoded, reads
/// back every appearance key the version has at its `KeySpec` default and
/// the run's unused bits at the games' values. The 0xFF record holds no
/// default of its own, so a key the row left out reads as its field's
/// all-ones. The motion keys sit outside the row and are not its to set.
/// 71405 is /co/'s slot 05.
#[test]
fn a_default_row_carries_the_template_defaults() {
    let version = PesVersion::Pes21;
    let schema = schema_for(version);
    let row = PlayerSettings::default()
        .appearance_row(71405, version)
        .expect("a row");
    assert_eq!(row[..4], 71405u32.to_le_bytes(), "the row names the player");
    let mut record = vec![0xFFu8; schema.player.size];
    // The names are no key, and 0xFF bytes are no text: they read empty.
    for text in schema.player.texts {
        let at = usize::try_from(text.byte_offset).expect("an offset");
        let len = usize::try_from(text.len).expect("a length");
        record[at..at + len].fill(0);
    }
    record[252..252 + 56].copy_from_slice(&row[4..]);
    let player = read_player(&record, schema.player).expect("decode");
    let read = PlayerSettings {
        appearance: appearance_from(&player).expect("stored-ranges read"),
        ..PlayerSettings::default()
    };
    for key in SettingKey::ALL {
        if key.spec().table == KeyTable::Motion
            || matches!(key, SettingKey::BootsId | SettingKey::GlovesId)
        {
            continue;
        }
        let expected = if held_on(key, version) {
            key.spec().default
        } else {
            None
        };
        assert_eq!(read.get(key), expected, "{key:?}");
    }
    // The row holds the run's first 46 bytes, the run starting at the
    // row's byte 10; the rest of the run is the record's.
    let mut written = player.appearance.ingame_face.clone();
    written.write_unused();
    assert_eq!(
        player.appearance.ingame_face.bytes()[..46],
        written.bytes()[..46],
        "the run's unused bits are the games' values"
    );
}

/// The row carries neither name: an overlong one is not this function's
/// error, and the names' bytes never reach the row.
#[test]
fn the_names_are_not_the_rows() {
    let named = PlayerSettings {
        name: Some(NameSetting::Explicit("N".repeat(100))),
        shirt_name: Some("S".repeat(100)),
        ..PlayerSettings::default()
    };
    for version in [
        PesVersion::Pes18,
        PesVersion::Pes19,
        PesVersion::Pes20,
        PesVersion::Pes21,
    ] {
        let plain = PlayerSettings::default()
            .appearance_row(70101, version)
            .expect("a row");
        let row = named.appearance_row(70101, version).expect("a row");
        assert_eq!(row, plain, "{version:?}: the names reached the row");
    }
}

/// Every key at its `version_kind` extreme fits the version's fields: the
/// narrowing is what the writer relies on when it `expect`s the row to
/// build. Both ends of each held range are tried, on every Fox version.
#[test]
fn a_row_builds_from_every_key_at_its_version_range_extremes() {
    let stored_max = |kind: Kind| match kind {
        Kind::Number { max, .. } => max,
        Kind::Signed { limit } => 2 * limit,
        Kind::OneBased { max } => max - 1,
        Kind::PlusOne { max } => max + 1,
        Kind::Mapped(map) => *map.iter().max().expect("a nonempty map"),
        Kind::Bool => 1,
        Kind::Labels(labels) => labels.len() as u8 - 1,
    };
    let stored_min = |kind: Kind| match kind {
        Kind::Number { min, .. } => min,
        Kind::Signed { .. } | Kind::OneBased { .. } | Kind::PlusOne { .. } => 0,
        Kind::Mapped(map) => *map.iter().min().expect("a nonempty map"),
        Kind::Bool | Kind::Labels(_) => 0,
    };
    for version in [
        PesVersion::Pes18,
        PesVersion::Pes19,
        PesVersion::Pes20,
        PesVersion::Pes21,
    ] {
        for stored in [stored_min, stored_max] {
            let mut settings = PlayerSettings::default();
            for key in SettingKey::ALL {
                let Some(kind) = key.version_kind(version) else {
                    continue;
                };
                settings
                    .set(key, stored(kind))
                    .expect("inside the widest range");
            }
            settings
                .appearance_row(71405, version)
                .unwrap_or_else(|err| panic!("{version:?}: {err}"));
        }
    }
}

/// A pre-Fox game has no row: PES 17 reads one, but its compile writes the
/// savefile instead.
#[test]
fn a_pre_fox_version_has_no_row() {
    for version in [PesVersion::Pes15, PesVersion::Pes16, PesVersion::Pes17] {
        let err = PlayerSettings::default()
            .appearance_row(70101, version)
            .expect_err("no row before Fox");
        assert!(
            matches!(err, SettingsError::NoAppearanceRow { version: v } if v == version),
            "{version:?}: {err}"
        );
    }
    let err = PlayerSettings::default()
        .appearance_row(70101, PesVersion::Pes17)
        .expect_err("no row before Fox");
    assert_eq!(
        err.to_string(),
        "PES 2017: a pre-Fox appearance travels in the savefile, not in PlayerAppearance.bin"
    );
}
