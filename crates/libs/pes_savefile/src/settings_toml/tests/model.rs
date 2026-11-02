use super::*;

use crate::schema::limits::motion_top;

/// A key's field is the one it writes: a player-record key's in
/// `player_field`, an ingame-face key's in `face_field`, and PES 15's nose
/// angle, coded into `NoseType`, in neither.
#[test]
fn a_keys_field_is_the_one_it_writes() {
    assert_eq!(
        (
            SettingKey::BootsId.player_field(),
            SettingKey::BootsId.face_field()
        ),
        (Some(PlayerField::BootsId), None)
    );
    assert_eq!(
        (
            SettingKey::SkinColor.player_field(),
            SettingKey::SkinColor.face_field()
        ),
        (None, Some(IngameFaceField::SkinColor))
    );
    assert_eq!(
        (
            SettingKey::Pes15NoseAngle.player_field(),
            SettingKey::Pes15NoseAngle.face_field()
        ),
        (None, None)
    );
}

#[test]
fn every_settings_field_is_a_key_and_every_key_a_settings_field() {
    let mut fields = HashSet::new();
    for version in PesVersion::ALL {
        let schema = schema_for(version);
        fields.extend(runs(schema.player).map(|(f, _, _)| f));
        if let Some((_, appearance)) = &schema.appearance {
            fields.extend(runs(*appearance).map(|(f, _, _)| f));
        }
    }
    let mut settings = HashSet::new();
    for field in fields {
        if ownership(field) == Ownership::Settings {
            settings.insert(Source::Player(field));
        }
    }
    for field in IngameFaceField::ALL {
        if face_ownership(field) == Ownership::Settings {
            settings.insert(Source::Face(field));
        }
    }
    // PES 15's nose angle shares `NoseType`'s field with `nose_type`.
    let keys: HashSet<Source> = SettingKey::ALL
        .iter()
        .map(|k| k.source())
        .filter(|source| *source != Source::NoseAngle)
        .collect();
    let without_key: Vec<_> = settings.difference(&keys).collect();
    let without_field: Vec<_> = keys.difference(&settings).collect();
    assert!(
        without_key.is_empty() && without_field.is_empty(),
        "settings fields without a key: {without_key:?}; keys without a field: {without_field:?}"
    );
    // No key shares a (table, name) pair.
    let names: HashSet<(&str, &str)> = SettingKey::ALL
        .iter()
        .map(|k| (k.spec().table.path(), k.spec().name))
        .collect();
    assert_eq!(
        names.len(),
        SettingKey::ALL.len(),
        "a duplicate (table, name)"
    );
}

/// The completeness invariant, per bit and per version: the appearance
/// block — the whole appearance record on PES 15/16, the player record
/// from the `AppearanceId` row's byte to its end on PES 17+ — is carried
/// exactly once per bit: a `PlayerField` row of the block's record (a
/// settings key's or compiler-owned, never `Gameplay`; the PES 15/16
/// appearance record's own `Id`, the record's join key, is the one
/// exception), an `INGAME_FACE_FIELDS` row of the version's layout inside
/// the run, or an `UNUSED_FACE_BITS` row of that layout (measured unused,
/// written at the games' values). A bit no carrier covers fails the
/// exactly-one check, so no bit of the run is left undecoded.
#[test]
fn every_bit_of_the_appearance_block_has_one_carrier() {
    /// What carries one bit of the block.
    #[derive(Debug, Clone, Copy)]
    enum Carrier {
        /// A `PlayerField` row of the record's schema.
        Field(PlayerField),
        /// An `INGAME_FACE_FIELDS` row inside the run.
        Face(IngameFaceField),
        /// An `UNUSED_FACE_BITS` row inside the run, by its bit offset.
        Unused(u32),
    }
    /// The carrier's name in a failure message.
    fn name(carrier: &Carrier) -> String {
        match carrier {
            Carrier::Field(field) => format!("{field:?}"),
            Carrier::Face(field) => format!("ingame_face::{field:?}"),
            Carrier::Unused(offset) => format!("unused at {offset}"),
        }
    }
    for version in PesVersion::ALL {
        let schema = schema_for(version);
        let (record, block_start) = match &schema.appearance {
            Some((_, appearance)) => (*appearance, 0),
            None => {
                let (_, offset, _) = runs(schema.player)
                    .find(|(field, _, _)| *field == PlayerField::AppearanceId)
                    .expect("PES 17+ stores the appearance id");
                (schema.player, offset as usize / 8)
            }
        };
        let run = record
            .ingame_face
            .as_ref()
            .expect("the block's record carries the run");
        let run_first_bit = run.byte_offset as usize * 8;
        let mut carriers: Vec<Vec<Carrier>> = vec![Vec::new(); record.size * 8];
        for (field, offset, width) in runs(record) {
            for bit in offset..offset + width {
                carriers[bit as usize].push(Carrier::Field(field));
            }
        }
        let layout = FaceLayout::of(version);
        for spec in INGAME_FACE_FIELDS
            .iter()
            .filter(|spec| in_layout(spec.field.layout(), layout))
        {
            for bit in spec.bit_offset..spec.bit_offset + spec.bit_width {
                carriers[run_first_bit + bit as usize].push(Carrier::Face(spec.field));
            }
        }
        for row in UNUSED_FACE_BITS
            .iter()
            .filter(|row| in_layout(row.layout, layout))
        {
            for bit in row.bit_offset..row.bit_offset + row.bit_width {
                carriers[run_first_bit + bit as usize].push(Carrier::Unused(row.bit_offset));
            }
        }
        for (i, bit_carriers) in carriers[block_start * 8..].iter().enumerate() {
            assert_eq!(
                bit_carriers.len(),
                1,
                "{version:?} bit {}: carriers {:?}",
                block_start * 8 + i,
                bit_carriers.iter().map(name).collect::<Vec<_>>()
            );
            // A field carrier is a settings key's or compiler-owned, never
            // gameplay; the PES 15/16 appearance record's own `Id` is the
            // exception — its first u32 is the record's join key, not a
            // setting.
            if let [Carrier::Field(field)] = bit_carriers.as_slice() {
                assert!(
                    *field == PlayerField::Id || ownership(*field) != Ownership::Gameplay,
                    "{version:?} bit {}: gameplay field {field:?} carries the bit",
                    block_start * 8 + i,
                );
            }
        }
    }
}

#[test]
fn from_player_reads_every_key_and_the_gated_dribbling() {
    let p19 = find_player(
        &payload(PesVersion::Pes19),
        schema_for(PesVersion::Pes19),
        70101,
    );
    let s19 = PlayerSettings::from_player(&p19).expect("from_player");
    let face = &p19.appearance.ingame_face;
    assert_eq!(
        s19.appearance.skin_color,
        Some(face.get(IngameFaceField::SkinColor).expect("read run"))
    );
    assert_eq!(
        s19.appearance.iris_color,
        Some(face.get(IngameFaceField::IrisColor).expect("read run"))
    );
    assert_eq!(
        s19.appearance.physique.neck_length,
        Some(p19.appearance.neck_length)
    );
    assert_eq!(s19.appearance.strip.sleeves, Some(p19.appearance.sleeves));
    assert_eq!(s19.appearance.motion.dribbling, None);
    assert_eq!(s19.name, Some(NameSetting::Explicit(p19.name.clone())));
    let p21 = find_player(
        &payload(PesVersion::Pes21),
        schema_for(PesVersion::Pes21),
        70101,
    );
    let s21 = PlayerSettings::from_player(&p21).expect("from_player");
    assert_eq!(s21.appearance.motion.dribbling, p21.motion.dribbling);
    assert!(s21.appearance.motion.dribbling.is_some());
}

#[test]
fn apply_copies_the_settings_and_leaves_the_rest() {
    let payload = payload(PesVersion::Pes19);
    let schema = schema_for(PesVersion::Pes19);
    let p = find_player(&payload, schema, 70101);
    let mut q = find_player(&payload, schema, 70102);

    // Bytes 46 to 49 are measured unused on PES 16 to 21, the games write
    // 0: 0xFF in q's makes the check below discriminate, `apply` writing
    // the games' values there.
    let mut run = q.appearance.ingame_face.bytes().to_vec();
    run[46..50].fill(0xFF);
    q.appearance.ingame_face = IngameFace::from_bytes(run);
    let q_before = q.clone();

    PlayerSettings::from_player(&p)
        .expect("from_player")
        .apply(&mut q, PesVersion::Pes19)
        .expect("apply");

    // Every scalar appearance field and every face field is p's, and the
    // unused bits are the games' values.
    let mut run = p.appearance.ingame_face.clone();
    run.write_unused();
    assert_eq!(run.bytes()[46..50], [0; 4], "the games' values");
    assert_eq!(
        q.appearance,
        PlayerAppearance {
            boots_id: q_before.appearance.boots_id,
            gloves_id: q_before.appearance.gloves_id,
            base_copy_id: q_before.appearance.base_copy_id,
            ingame_face: run,
            ..p.appearance.clone()
        }
    );
    assert_eq!(q.motion, p.motion);
    assert_eq!(q.name, p.name);
    assert_eq!(q.shirt_name, p.shirt_name);
    assert_eq!(q.id, q_before.id);
    assert_eq!(q.stats, q_before.stats);
    assert_eq!(q.positions, q_before.positions);
    assert_eq!(q.skills, q_before.skills);
    assert_eq!(q.edit_flags, q_before.edit_flags);
    // q's basics (nationality, age, height, weight) are gameplay, untouched.
    assert_eq!(q.basic, q_before.basic);
}

#[test]
fn height_and_weight_are_not_settings() {
    let payload = payload(PesVersion::Pes19);
    let schema = schema_for(PesVersion::Pes19);
    let p = find_player(&payload, schema, 70101);
    let mut q = find_player(&payload, schema, 70102);
    assert_ne!(
        (p.basic.height, p.basic.weight),
        (q.basic.height, q.basic.weight),
        "the test needs the two players to differ"
    );
    let (height, weight) = (q.basic.height, q.basic.weight);
    let settings = PlayerSettings::from_player(&p).expect("from_player");
    settings.apply(&mut q, PesVersion::Pes19).expect("apply");
    assert_eq!(
        (q.basic.height, q.basic.weight),
        (height, weight),
        "a settings file does not carry them"
    );
    // Nothing named `height` or `weight` emits, set or commented.
    let text = settings.to_toml().expect("emit");
    assert!(
        !text.lines().any(|line| {
            let line = line.trim_start_matches('#').trim_start();
            line.starts_with("height =") || line.starts_with("weight =")
        }),
        "a height/weight line emitted: {text}"
    );
}

/// Each key the version holds, set to another value its kind accepts,
/// changes its own bits and no other: the baseline is the same settings
/// unflipped, applied the same way (`apply` also writes the run's unused
/// bits, the same in both).
#[test]
fn every_key_writes_only_its_own_run() {
    let payload = payload(PesVersion::Pes21);
    let schema = schema_for(PesVersion::Pes21);
    let index = player_record_index(&payload, schema, 70101);
    let rec = record(&payload, &schema.players, schema.player.size, index);
    let run = schema.player.ingame_face.as_ref().expect("a run on PES 21");
    let player = find_player(&payload, schema, 70101);
    // `from_player` fills every key, so only the flipped one's bits can
    // differ from the baseline (absent keys would write their defaults).
    let settings = PlayerSettings::from_player(&player).expect("from_player");
    let written_with = |settings: &PlayerSettings| {
        let mut modified = player.clone();
        settings
            .apply(&mut modified, PesVersion::Pes21)
            .expect("apply");
        let mut written = rec.to_vec();
        write_player(&modified, &mut written, schema.player).expect("encode");
        (modified, written)
    };
    let (_, baseline) = written_with(&settings);
    for key in SettingKey::ALL {
        if !held_on(key, PesVersion::Pes21) {
            continue;
        }
        let current = settings.get(key);
        // 1 first: an unset stock ID reads `None` over a stored 0, which a 0
        // would leave as it is.
        let new_value = (1..=u8::MAX)
            .chain([0])
            .find(|value| Some(*value) != current && key.spec().kind.accepts_stored(*value))
            .expect("a kind accepts two values");
        let mut flipped = settings.clone();
        flipped
            .set(key, new_value)
            .expect("the flipped value is in range");
        let (modified, written) = written_with(&flipped);

        let (start, end) = match key.source() {
            Source::Player(field) => {
                let (_, offset, width) = runs(schema.player)
                    .find(|(f, _, _)| *f == field)
                    .unwrap_or_else(|| panic!("{key:?} source has no run"));
                (offset, offset + width)
            }
            Source::Face(field) => {
                let row = INGAME_FACE_FIELDS
                    .iter()
                    .find(|f| f.field == field)
                    .unwrap_or_else(|| panic!("{key:?} source has no row"));
                (
                    run.byte_offset * 8 + row.bit_offset,
                    run.byte_offset * 8 + row.bit_offset + row.bit_width,
                )
            }
            Source::NoseAngle => unreachable!("PES 21 holds no nose angle"),
        };
        let mut diffs = 0usize;
        for (byte, (was, now)) in baseline.iter().zip(&written).enumerate() {
            for bit in 0..8 {
                if (was >> bit) & 1 != (now >> bit) & 1 {
                    let pos = byte as u32 * 8 + bit;
                    diffs += 1;
                    assert!(
                        start <= pos && pos < end,
                        "{key:?} changed bit {pos} outside its run {start}..{end}"
                    );
                }
            }
        }
        assert_ne!(diffs, 0, "{key:?} changed nothing");
        assert_eq!(
            PlayerSettings::from_player(&modified)
                .expect("from_player")
                .get(key),
            Some(new_value),
            "{key:?} reads back the new value"
        );
    }
}

/// A default apply writes every appearance key's bits (set ones at their
/// value, absent ones at their default) plus the run's unused bits (at the
/// games' values), and no other bit of the record (stats, edit flags, the appearance
/// id).
#[test]
fn an_apply_then_write_touches_only_the_appearance_keys_bits() {
    let payload = payload(PesVersion::Pes19);
    let schema = schema_for(PesVersion::Pes19);
    let mut player = find_player(&payload, schema, 70101);
    PlayerSettings::default()
        .apply(&mut player, PesVersion::Pes19)
        .expect("apply");

    let index = player_record_index(&payload, schema, 70101);
    let rec = record(&payload, &schema.players, schema.player.size, index);
    let mut written = rec.to_vec();
    write_player(&player, &mut written, schema.player).expect("encode");

    // The bit runs of every settings key the version holds (the stock IDs
    // included: unset they are left alone, but their bits are the file's to
    // write), plus the whole ingame-face run: `apply` writes its unused
    // bits. A key the version has no field for (`dribbling` here) writes
    // nothing, so it contributes no run.
    let run = schema.player.ingame_face.as_ref().expect("a run on PES 19");
    let mut ranges = Vec::new();
    for key in SettingKey::ALL {
        match key.source() {
            Source::Player(field) => {
                if let Some((_, offset, width)) = runs(schema.player).find(|(f, _, _)| *f == field)
                {
                    ranges.push((offset, offset + width));
                }
            }
            Source::Face(field) => {
                let row = INGAME_FACE_FIELDS
                    .iter()
                    .find(|f| f.field == field)
                    .unwrap_or_else(|| panic!("{key:?} source has no row"));
                ranges.push((
                    run.byte_offset * 8 + row.bit_offset,
                    run.byte_offset * 8 + row.bit_offset + row.bit_width,
                ));
            }
            // Inside the run, which is pushed whole below.
            Source::NoseAngle => {}
        }
    }
    ranges.push((run.byte_offset * 8, (run.byte_offset + run.len) * 8));
    let mut diffs = 0usize;
    for (byte, (was, now)) in rec.iter().zip(&written).enumerate() {
        for bit in 0..8 {
            if (was >> bit) & 1 != (now >> bit) & 1 {
                let pos = byte as u32 * 8 + bit;
                diffs += 1;
                assert!(
                    ranges.iter().any(|(s, e)| pos >= *s && pos < *e),
                    "bit {pos} changed outside the settings keys' runs"
                );
            }
        }
    }
    assert_ne!(
        diffs, 0,
        "the defaults differ from the fixture's appearance"
    );
}

/// `dribbling` exists only on PES 20 and 21: on an older target the key is
/// skipped, set or not (an export written for a newer version compiles for
/// an older one as it is), and the rest of the settings still applies. On
/// PES 21 the same settings writes it.
#[test]
fn a_setting_the_version_lacks_is_skipped_not_refused() {
    let mut settings = PlayerSettings {
        name: Some(NameSetting::Explicit("RENAMED".to_string())),
        ..PlayerSettings::default()
    };
    settings.set(SettingKey::Sleeves, 2).expect("in range");
    settings.set(SettingKey::Dribbling, 2).expect("in range");

    let payload19 = payload(PesVersion::Pes19);
    let mut player = find_player(&payload19, schema_for(PesVersion::Pes19), 70101);
    settings
        .apply(&mut player, PesVersion::Pes19)
        .expect("a key the version lacks is skipped, not an error");
    assert_eq!(player.motion.dribbling, None, "no field, no write");
    assert_eq!(player.name, "RENAMED");
    assert_eq!(player.appearance.sleeves, 2);

    let payload21 = payload(PesVersion::Pes21);
    let mut player = find_player(&payload21, schema_for(PesVersion::Pes21), 70101);
    settings
        .apply(&mut player, PesVersion::Pes21)
        .expect("apply");
    assert_eq!(player.motion.dribbling, Some(2));
    assert_eq!(player.appearance.sleeves, 2);
}

/// `resolved` is the aesthetics patch's view of a settings file's
/// appearance tables: every key the version holds at its set value or its
/// `KeySpec` default, a key the version lacks `None`.
#[test]
fn resolved_fills_every_appearance_key_the_version_holds() {
    for version in [PesVersion::Pes18, PesVersion::Pes21] {
        let resolved = AppearanceSettings::default().resolved(version);
        for key in SettingKey::ALL {
            let spec = key.spec();
            if spec.table == KeyTable::Top {
                continue;
            }
            let expected = if held_on(key, version) {
                Some(spec.default.expect("every non-top key has a default"))
            } else {
                None
            };
            assert_eq!(
                get_appearance(&resolved, key),
                expected,
                "{version:?} {key:?}"
            );
        }
    }
    // `dribbling` is PES 20+: `None` resolved on 18, its default on 21.
    assert_eq!(
        AppearanceSettings::default()
            .resolved(PesVersion::Pes18)
            .motion
            .dribbling,
        None
    );
    assert_eq!(
        AppearanceSettings::default()
            .resolved(PesVersion::Pes21)
            .motion
            .dribbling,
        SettingKey::Dribbling.spec().default
    );
    // A set key keeps its stored value.
    let mut set = AppearanceSettings::default();
    set.motion.free_kick = Some(5);
    assert_eq!(set.resolved(PesVersion::Pes21).motion.free_kick, Some(5));
}

/// A fully unset settings writes every appearance key's default, so no
/// save value shows through: the stock IDs and the names keep the save's
/// own (the compiler resolves the IDs; absent names are never written),
/// and nothing outside the appearance changes.
#[test]
fn a_default_settings_applies_the_template_values() {
    for version in [
        PesVersion::Pes15,
        PesVersion::Pes16,
        PesVersion::Pes19,
        PesVersion::Pes21,
    ] {
        let payload = payload(version);
        let schema = schema_for(version);
        let mut player = find_player(&payload, schema, 70101);
        let before = player.clone();
        // The fixture's appearance must differ from the defaults, or the
        // assertions below prove nothing.
        let current = PlayerSettings::from_player(&before).expect("from_player");
        assert!(
            SettingKey::ALL
                .iter()
                .any(|&key| { held_on(key, version) && current.get(key) != key.spec().default }),
            "{version:?} player 70101 already sits at the defaults"
        );

        PlayerSettings::default()
            .apply(&mut player, version)
            .expect("apply");

        // Write through the codec and reread: on PES 15/16 the appearance
        // lives in its own record, so both records round-trip.
        let mut written = payload.clone();
        let index = player_record_index(&payload, schema, 70101);
        let start = schema.players.offset + index * schema.player.size;
        write_player(
            &player,
            &mut written[start..start + schema.player.size],
            schema.player,
        )
        .expect("encode");
        if let Some((section, appearance)) = &schema.appearance {
            let index = (0..count(&payload, section))
                .find(|&i| {
                    u32::from_le_bytes(
                        record(&payload, section, appearance.size, i)[..4]
                            .try_into()
                            .expect("id"),
                    ) == player.id
                })
                .expect("the player's appearance record");
            let start = section.offset + index * appearance.size;
            write_player(
                &player,
                &mut written[start..start + appearance.size],
                appearance,
            )
            .expect("encode");
        }
        let read = PlayerSettings::from_player(&find_player(&written, schema, 70101))
            .expect("from_player");
        for key in SettingKey::ALL {
            let expected = if held_on(key, version) {
                key.spec().default
            } else {
                None
            };
            match key {
                SettingKey::BootsId | SettingKey::GlovesId => {}
                _ => assert_eq!(read.get(key), expected, "{version:?} {key:?}"),
            }
        }
        // On PES 21 the default `dribbling = 0` is written too.
        assert_eq!(
            player.motion.dribbling,
            if version == PesVersion::Pes21 {
                Some(0)
            } else {
                None
            }
        );
        // The compiler-resolved IDs, the names and the rest of the record.
        assert_eq!(player.appearance.boots_id, before.appearance.boots_id);
        assert_eq!(player.appearance.gloves_id, before.appearance.gloves_id);
        assert_eq!(
            player.appearance.base_copy_id,
            before.appearance.base_copy_id
        );
        assert_eq!(player.name, before.name);
        assert_eq!(player.shirt_name, before.shirt_name);
        assert_eq!(player.id, before.id);
        assert_eq!(player.appearance_id, before.appearance_id);
        assert_eq!(player.basic, before.basic);
        assert_eq!(player.stats, before.stats);
        assert_eq!(player.positions, before.positions);
        assert_eq!(player.skills, before.skills);
        assert_eq!(player.edit_flags, before.edit_flags);
    }
}

/// The point of "absent = default": the same partial file applied to two
/// different players gives them the same appearance, nothing of what the
/// save held shows through.
#[test]
fn the_same_partial_settings_gives_two_players_the_same_appearance() {
    let payload = payload(PesVersion::Pes19);
    let schema = schema_for(PesVersion::Pes19);
    let mut a = find_player(&payload, schema, 70101);
    let mut b = find_player(&payload, schema, 70102);
    assert_ne!(
        PlayerSettings::from_player(&a)
            .expect("from_player")
            .appearance,
        PlayerSettings::from_player(&b)
            .expect("from_player")
            .appearance,
        "the test needs two players whose appearance differs"
    );
    let mut settings = PlayerSettings::default();
    settings.set(SettingKey::SkinColor, 3).expect("in range");
    settings.set(SettingKey::FreeKick, 19).expect("in range");
    settings.set(SettingKey::CheekType, 2).expect("in range");
    settings.apply(&mut a, PesVersion::Pes19).expect("apply");
    settings.apply(&mut b, PesVersion::Pes19).expect("apply");
    assert_eq!(
        PlayerSettings::from_player(&a)
            .expect("from_player")
            .appearance,
        PlayerSettings::from_player(&b)
            .expect("from_player")
            .appearance
    );
}

/// A run from its hex form, as the tests below quote measured runs.
fn run_of(hex: &str) -> IngameFace {
    IngameFace::from_bytes(
        hex.as_bytes()
            .chunks(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).expect("ascii"), 16))
            .collect::<Result<_, _>>()
            .expect("hex"),
    )
}

/// PES 17's facial reset (measured 2026-10-13): the game's default face.
const PES17_RESET: &str = "7079777777777777777777777777777777777777777777010000707700774089a31c110eca470ae028c70100092300000000";

/// The game's reset run decodes to its default face, and its unused bits
/// are the values `write_unused` writes.
#[test]
fn the_pes17_reset_run_decodes_to_the_default_face() {
    use IngameFaceField as F;
    let face = run_of(PES17_RESET);
    for (field, want) in [
        (F::HairColorRed, 14),
        (F::HairColorGreen, 10),
        (F::HairColorBlue, 7),
        (F::HairColorPreset, 0),
        (F::HairStyle, 1),
        (F::HairCroppedStyle, 1),
        (F::HairLength, 2),
        (F::HairWaveLevel, 1),
        (F::HairVariation, 0),
        (F::HairFrontStyle, 1),
        (F::HairParted, 1),
        (F::HairSideBackStyle, 4),
        (F::Hairline, 1),
        (F::HairForeheadWidth, 2),
        (F::EyebrowColorRed, 14),
        (F::EyebrowColorGreen, 10),
        (F::EyebrowColorBlue, 7),
        (F::EyebrowDensity, 3),
        (F::EyebrowThickness, 1),
        (F::FacialHairColorRed, 14),
        (F::FacialHairColorGreen, 10),
        (F::FacialHairColorBlue, 7),
        (F::FacialHairThickness, 3),
        (F::JawHeight, 9),
        (F::UpperLipType, 1),
        (F::LowerLipType, 1),
    ] {
        assert_eq!(
            face.get(field).expect("PES 17's run holds it"),
            want,
            "{field:?}"
        );
    }
    assert_eq!(face.nose().expect("reads"), (0, 0));
    assert!(matches!(
        face.get(F::Pes15HairLength),
        Err(CodecError::FieldNotInLayout { .. })
    ));
    assert_write_unused_restores(&face);
}

/// `run` with every measured-unused bit of its layout flipped, so a
/// `write_unused` that skips a row, or writes another value, leaves a
/// difference.
fn with_unused_flipped(run: &IngameFace) -> IngameFace {
    let mut bytes = run.bytes().to_vec();
    for row in UNUSED_FACE_BITS
        .iter()
        .filter(|row| in_layout(row.layout, run.layout()))
    {
        for bit in row.bit_offset..row.bit_offset + row.bit_width {
            bytes[(bit / 8) as usize] ^= 1 << (bit % 8);
        }
    }
    IngameFace::from_bytes(bytes)
}

/// A game's own run holds its unused bits at the values `write_unused`
/// writes: with those bits flipped, `write_unused` gives back the game's
/// run exactly, so it overwrites every row and touches no field beside one.
fn assert_write_unused_restores(run: &IngameFace) {
    let mut written = with_unused_flipped(run);
    assert_ne!(&written, run, "the flip changes the run");
    written.write_unused();
    assert_eq!(&written, run, "write_unused writes the games' values");
}

/// The template's defaults are the game's default face: every face and
/// hair key PES 17 holds reads its `KeySpec` default from the reset run.
/// Aside: skin, iris and the player gloves (the reset keeps the player's
/// own), and the reset's jaw height +2 and lips 1, which the template does
/// not take (a slider at 0 is the editor's centre; the spec's ingame-face
/// rules).
#[test]
fn the_face_and_hair_defaults_are_the_reset_run() {
    let mut player = find_player(
        &payload(PesVersion::Pes17),
        schema_for(PesVersion::Pes17),
        70101,
    );
    player.appearance.ingame_face = run_of(PES17_RESET);
    let appearance = appearance_from(&player).expect("reads");
    let mut differ = Vec::new();
    for key in SettingKey::ALL {
        let keep = matches!(
            key,
            SettingKey::SkinColor
                | SettingKey::IrisColor
                | SettingKey::Gloves
                | SettingKey::GlovesColor
                | SettingKey::JawHeight
                | SettingKey::UpperLipType
                | SettingKey::LowerLipType
        );
        if keep || !matches!(key.source(), Source::Face(_)) || !held_on(key, PesVersion::Pes17) {
            continue;
        }
        let reset = get_appearance(&appearance, key);
        if reset != key.spec().default {
            differ.push(format!(
                "{}: reset {reset:?}, default {:?}",
                key.spec().name,
                key.spec().default
            ));
        }
    }
    assert!(differ.is_empty(), "{differ:#?}");
}

/// PES 15 runs measured in round 3 (`/3/` of the PES 15 cup save): the
/// nose angle coded into the nose field, the hair's PES 15 fields, and the
/// PES 16 to 21 fields refused.
#[test]
fn pes15_runs_decode_and_the_nose_round_trips() {
    use IngameFaceField as F;
    // Slot 00: variation 63 in the game, Medium/Straight, a headband.
    let slot00 = run_of(
        "72777777777777477777777777777777777777777d777761e806707700a74200a7513e003f1f0c0dfc1f7a470037",
    );
    assert_eq!(slot00.nose().expect("reads"), (0, 1));
    assert_eq!(slot00.get(F::HairAccessory).expect("reads"), 1);
    assert_eq!(slot00.get(F::Pes15HairVariation).expect("reads"), 62);
    assert_eq!(slot00.get(F::Pes15HairLength).expect("reads"), 4);
    assert_eq!(slot00.get(F::Pes15HairType).expect("reads"), 1);
    let slot09 = run_of(
        "727777777777777777777777777777777777777777a777016048767409775bc0cb86440b09030cb2244379070037",
    );
    assert_eq!(
        [
            F::HairColorRed,
            F::HairColorGreen,
            F::HairColorBlue,
            F::HairColorPreset
        ]
        .map(|field| slot09.get(field).expect("reads")),
        [47, 44, 33, 0]
    );
    assert_eq!(slot09.nose().expect("reads"), (9, 0));
    let slot13 = run_of(
        "72777777a777777777777777777777777774777777777701506872770d774100a751290b09430da3fe1570270037",
    );
    assert_eq!(slot13.nose().expect("reads"), (0, -1));
    for run in [slot00, slot09, slot13] {
        let (nose_type, angle) = run.nose().expect("reads");
        let mut written = run.clone();
        written.set_nose(nose_type, angle).expect("fits");
        assert_eq!(written, run, "set_nose writes back the same bits");
        assert_write_unused_restores(&run);
        assert!(matches!(
            run.get(F::HairStyle),
            Err(CodecError::FieldNotInLayout { .. })
        ));
    }
}

/// `set_nose`'s refusals, each `ValueTooWide` on the nose field, but on an
/// empty run (an entry never read), which is `NoIngameFaceRun` as for
/// every other accessor.
#[test]
fn set_nose_refuses_what_the_field_cannot_hold() {
    let mut empty = IngameFace::from_bytes(Vec::new());
    for angle in [0, 1] {
        assert!(
            matches!(
                empty.set_nose(0, angle),
                Err(CodecError::NoIngameFaceRun { ref field }) if field == "NoseType"
            ),
            "angle {angle}"
        );
    }
    let mut pes15 = IngameFace::from_bytes(vec![0; 46]);
    let mut pes21 = IngameFace::from_bytes(vec![0; 50]);
    let too_wide = |result: Result<(), CodecError>, value: u32| {
        assert!(
            matches!(
                result,
                Err(CodecError::ValueTooWide { ref field, value: v, width: 6 })
                    if field == "NoseType" && v == value
            ),
            "{result:?}"
        );
    };
    // 3 × 21 + 2 = 65 does not fit 6 bits; 3 × 21 + 0 = 63 does.
    too_wide(pes15.set_nose(21, 1), 65);
    pes15.set_nose(21, 0).expect("63 fits");
    too_wide(pes15.set_nose(0, 2), 2);
    too_wide(pes15.set_nose(0, -2), 2);
    too_wide(pes21.set_nose(0, 1), 1);
    too_wide(pes21.set_nose(64, 0), 64);
    pes21.set_nose(63, 0).expect("63 fits");
}

/// Every player of every fixture save, through `from_player` and `apply`
/// onto a clone, comes back identical but for the run's unused bits,
/// which take the games' values. A stored value outside a key's range is
/// reported, version, player, key and value, not widened.
#[test]
fn every_fixture_player_round_trips_through_his_settings() {
    // (version, key, stored value) -> players, for a stored value the key's
    // kind refuses: a range to report. A face type cannot be one, its widest
    // range being its field's bits (an unmeasured cap).
    let mut refused = std::collections::BTreeMap::<String, usize>::new();
    for version in PesVersion::ALL {
        let (file, _) = open(version);
        let mut checked = 0usize;
        for player in file.players() {
            let stored = appearance_from(player).expect("stored-ranges read");
            let mut outside = false;
            for key in SettingKey::ALL {
                if let Some(value) = get_appearance(&stored, key)
                    && !key.spec().kind.accepts_stored(value)
                {
                    outside = true;
                    *refused
                        .entry(format!("{version:?} {} = {value}", key.spec().name))
                        .or_default() += 1;
                }
            }
            if outside {
                continue;
            }
            let settings = PlayerSettings::from_player(player).expect("from_player");
            let mut applied = player.clone();
            settings.apply(&mut applied, version).expect("apply");
            let mut expected = player.clone();
            expected.appearance.ingame_face.write_unused();
            assert_eq!(applied, expected, "{version:?} player {}", player.id);
            checked += 1;
        }
        assert!(checked > 0, "{version:?}: no player checked");
    }
    // The one celebration past every range (PES 16's 183) stays refused: it is
    // past every version's top.
    let unexpected: Vec<_> = refused
        .iter()
        .filter(|(line, _)| line.as_str() != "Pes16 goal_celebration_2 = 183")
        .collect();
    assert!(
        unexpected.is_empty(),
        "stored values outside a key's range (players): {unexpected:#?}"
    );
}

/// The one fixture player whose stored value no key's range can hold (the
/// PES 16 save's `goal_celebration_2` 183, found as the round trip above
/// finds him): `from_player_skipping` leaves the key unset and reports the
/// value, the rest of his settings filled; `from_player` still fails him.
#[test]
fn from_player_skipping_leaves_an_unrepresentable_value_out() {
    let (file, _) = open(PesVersion::Pes16);
    let player = file
        .players()
        .iter()
        .find(|player| {
            player
                .get(PlayerField::GoalCelebration2)
                .is_ok_and(|value| value == 183)
        })
        .expect("the fixture save's player storing 183 in goal_celebration_2");
    let (settings, refused) =
        PlayerSettings::from_player_skipping(player).expect("from_player_skipping");
    assert_eq!(
        refused,
        vec![RefusedValue {
            key: "appearance.motion.goal_celebration_2".to_string(),
            value: 183,
            range: "0 to 162".to_string(),
        }]
    );
    assert_eq!(settings.appearance.motion.goal_celebration_2, None);
    assert!(settings.appearance.motion.goal_celebration_1.is_some());
    assert!(
        matches!(
            PlayerSettings::from_player(player),
            Err(SettingsError::OutOfRange { .. })
        ),
        "from_player still refuses him"
    );
}

/// A motion at its version's measured top reads into a settings file and
/// applies back: the parser's widest range takes every version's top.
#[test]
fn a_motion_at_the_versions_top_round_trips() {
    let fields = [
        PlayerField::HunchingDribbling,
        PlayerField::HunchingRunning,
        PlayerField::ArmMovementDribbling,
        PlayerField::ArmMovementRunning,
        PlayerField::CornerKickMotion,
        PlayerField::FreeKickMotion,
        PlayerField::PenaltyKickMotion,
        PlayerField::GoalCelebration1,
        PlayerField::GoalCelebration2,
    ];
    for version in PesVersion::ALL {
        let payload = payload(version);
        let schema = schema_for(version);
        let mut player = find_player(&payload, schema, 70101);
        for field in fields {
            let top = motion_top(version, field).expect("a motion field has a top");
            player
                .set(field, u32::from(top))
                .expect("the field holds the top");
        }
        let settings = PlayerSettings::from_player(&player).expect("from_player");
        let mut applied = player.clone();
        settings.apply(&mut applied, version).expect("apply");
        let mut expected = player.clone();
        expected.appearance.ingame_face.write_unused();
        assert_eq!(applied, expected, "{version:?} player {}", player.id);
    }
}

/// Keys of one layout are skipped on the other, with no finding or error:
/// one file compiles for PES 15 and PES 21.
#[test]
fn a_family_only_key_is_skipped_on_the_other_layout() {
    use IngameFaceField as F;
    let mut settings = PlayerSettings::default();
    settings.set_ingame_face(
        IngameFaceSettings::parse(
            "[face]\nnose_type = 9\npes15_nose_angle = -1\n\n\
             [hair]\nstyle = 4\npes15_length = 5\n",
        )
        .expect("parses"),
    );
    let mut p15 = find_player(
        &payload(PesVersion::Pes15),
        schema_for(PesVersion::Pes15),
        70101,
    );
    settings.apply(&mut p15, PesVersion::Pes15).expect("apply");
    let face = &p15.appearance.ingame_face;
    assert_eq!(face.get(F::Pes15HairLength).expect("PES 15's"), 5);
    assert_eq!(
        Some(face.get(F::Pes15HairVariation).expect("PES 15's")),
        SettingKey::Pes15HairVariation.spec().default,
        "byte 34 is the default variation's, not style 4"
    );
    // 3 × 9 + the angle -1's code 1.
    assert_eq!(face.bytes()[30] & 0x3F, 28, "the nose field holds 28");
    assert_eq!(face.nose().expect("reads"), (9, -1));

    let mut p21 = find_player(
        &payload(PesVersion::Pes21),
        schema_for(PesVersion::Pes21),
        70101,
    );
    settings.apply(&mut p21, PesVersion::Pes21).expect("apply");
    let face = &p21.appearance.ingame_face;
    assert_eq!(face.get(F::HairStyle).expect("PES 21's"), 4);
    assert_eq!(
        Some(face.get(F::HairLength).expect("PES 21's")),
        SettingKey::HairLength.spec().default
    );
    assert_eq!(face.get(F::NoseType).expect("PES 21's"), 9);
}

/// Every face and hair key's field at the byte:bit and width, and every
/// measured-unused row at the bits and value, that
/// `spec/pes_savefile/model.md` "The ingame-face run" gives. The table is
/// copied from that section, not derived from the schema, so a row
/// of the schema that moved, changed width or swapped with another fails
/// here even where both fields hold the same value in every fixture.
#[test]
fn every_face_key_sits_at_the_plans_bits() {
    // (key, byte, bit, width): a slider 4 bits, a colour channel 6,
    // `gloves` 1, others as the spec writes them.
    const PLAN: [(&str, u32, u32, u32); 92] = [
        ("jawline", 0, 4, 4),
        ("jaw_height", 1, 0, 4),
        ("cheekbones", 2, 0, 4),
        ("cheekbone_height", 2, 4, 4),
        ("chin_width", 21, 4, 4),
        ("chin_height", 22, 0, 4),
        ("chin_depth", 22, 4, 4),
        ("face_size", 29, 0, 4),
        ("face_height", 29, 4, 4),
        ("brow_width", 4, 4, 4),
        ("inner_eyebrow_height", 5, 0, 4),
        ("eyebrow_depth", 5, 4, 4),
        ("temple_width", 6, 0, 4),
        ("outer_eyebrow_height", 6, 4, 4),
        ("pupil_size", 7, 4, 4),
        ("eye_depth", 8, 0, 4),
        ("inner_eye_position", 8, 4, 4),
        ("inner_eye_height", 9, 0, 4),
        ("outer_eye_position", 9, 4, 4),
        ("eye_corner_height", 10, 0, 4),
        ("upper_eyelid_width_inner", 10, 4, 4),
        ("upper_eyelid_height_inner", 11, 0, 4),
        ("upper_eyelid_width_outer", 11, 4, 4),
        ("upper_eyelid_height_outer", 12, 0, 4),
        ("bottom_eyelid_height", 13, 0, 4),
        ("horizontal_eye_position", 13, 4, 4),
        ("eye_height", 14, 0, 4),
        ("nose_height", 14, 4, 4),
        ("nose_tip_depth", 15, 0, 4),
        ("nostril_width", 15, 4, 4),
        ("nose_depth", 16, 4, 4),
        ("nose_width", 17, 0, 4),
        ("mouth_height", 18, 0, 4),
        ("mouth_depth", 18, 4, 4),
        ("lip_size", 19, 0, 4),
        ("mouth_corner_height", 20, 0, 4),
        ("lip_width", 20, 4, 4),
        ("ear_length", 26, 4, 4),
        ("ear_width", 27, 0, 4),
        ("ear_angle", 27, 4, 4),
        ("pes15_jaw_depth", 1, 4, 4),
        ("pes15_cheekbone_depth", 3, 0, 4),
        ("pes15_mouth_edge_position", 21, 0, 4),
        ("pes15_eyebrow_depth", 7, 0, 4),
        ("pes15_bottom_eyelid_position", 12, 4, 4),
        ("pes15_nasal_bridge_position", 16, 0, 4),
        ("pes15_nasal_root_height", 17, 4, 4),
        ("cheek_type", 23, 3, 5),
        ("forehead_type", 24, 0, 3),
        ("facial_hair_type", 24, 3, 5),
        ("laughter_lines_type", 25, 0, 3),
        ("upper_eyelid_type", 25, 3, 5),
        ("lower_eyelid_type", 26, 0, 4),
        ("eyebrow_type", 28, 0, 5),
        ("neck_line_type", 28, 5, 3),
        ("nose_type", 30, 0, 6),
        ("upper_lip_type", 31, 0, 3),
        ("lower_lip_type", 31, 3, 3),
        ("eyebrow_color_red", 35, 0, 6),
        ("eyebrow_color_green", 36, 0, 6),
        ("eyebrow_color_blue", 37, 0, 6),
        ("eyebrow_style", 35, 6, 2),
        ("eyebrow_density", 36, 6, 2),
        ("eyebrow_thickness", 37, 6, 2),
        ("facial_hair_color_red", 39, 4, 6),
        ("facial_hair_color_green", 40, 2, 6),
        ("facial_hair_color_blue", 41, 0, 6),
        ("facial_hair_thickness", 41, 6, 2),
        ("pes15_eye_white", 38, 6, 2),
        ("color_red", 31, 6, 6),
        ("color_green", 32, 4, 6),
        ("color_blue", 33, 2, 6),
        ("accessory", 39, 0, 1),
        ("accessory_color", 39, 1, 3),
        ("color_preset", 43, 4, 4),
        ("style", 34, 0, 4),
        ("cropped_style", 34, 4, 4),
        ("length", 38, 0, 3),
        ("wave_level", 38, 3, 3),
        ("variation", 42, 4, 8),
        ("front_style", 44, 0, 3),
        ("parted", 44, 3, 3),
        ("side_back_style", 44, 6, 3),
        ("hairline", 45, 1, 3),
        ("forehead_width", 45, 4, 4),
        ("pes15_variation", 34, 0, 8),
        ("pes15_length", 38, 0, 3),
        ("pes15_type", 38, 3, 3),
        ("gloves", 0, 0, 1),
        ("gloves_color", 0, 1, 3),
        ("skin_color", 23, 0, 3),
        ("iris_color", 42, 0, 4),
    ];
    let mut seen = HashSet::new();
    for (name, byte, bit, width) in PLAN {
        let key = SettingKey::ALL
            .iter()
            .find(|key| key.spec().name == name)
            .unwrap_or_else(|| panic!("{name}: no such key"));
        let Source::Face(field) = key.source() else {
            panic!("{name}: not an ingame-face key");
        };
        let row = INGAME_FACE_FIELDS
            .iter()
            .find(|row| row.field == field)
            .unwrap_or_else(|| panic!("{field:?}: no row"));
        assert_eq!(
            (row.bit_offset, row.bit_width),
            (byte * 8 + bit, width),
            "{name} at {byte}:{bit} w{width}"
        );
        seen.insert(field);
    }
    let unplanned: Vec<_> = INGAME_FACE_FIELDS
        .iter()
        .map(|row| row.field)
        .filter(|field| !seen.contains(field))
        .collect();
    assert!(
        unplanned.is_empty(),
        "rows the plan's list lacks: {unplanned:?}"
    );

    // (layout, byte, bit, width, value): "Measured unused".
    let planned_unused = [
        (None, 3, 4, 4, 7),
        (None, 4, 0, 4, 7),
        (None, 19, 4, 4, 7),
        (None, 30, 6, 2, 1),
        (Some(FaceLayout::Pes16To21), 46, 0, 8, 0),
        (Some(FaceLayout::Pes16To21), 47, 0, 8, 0),
        (Some(FaceLayout::Pes16To21), 48, 0, 8, 0),
        (Some(FaceLayout::Pes16To21), 49, 0, 8, 0),
        (Some(FaceLayout::Pes15), 42, 4, 8, 0x77),
        (Some(FaceLayout::Pes15), 44, 0, 8, 0),
        (Some(FaceLayout::Pes15), 45, 0, 8, 0x37),
    ]
    .map(|(layout, byte, bit, width, value)| (layout, byte * 8 + bit, width, value));
    let table: Vec<_> = UNUSED_FACE_BITS
        .iter()
        .map(|row| (row.layout, row.bit_offset, row.bit_width, row.value))
        .collect();
    assert_eq!(table, planned_unused, "the measured-unused rows");
}

#[test]
fn get_after_set_round_trips_every_key() {
    for key in SettingKey::ALL {
        let mut settings = PlayerSettings::default();
        settings.set(key, 1).expect("in range");
        assert_eq!(settings.get(key), Some(1), "{key:?}");
    }
}

#[test]
fn the_key_specs_match_the_plan_table() {
    let comments: [&str; 130] = [
        "\"\" = default (fpc_on: hidden, fpc_off 0, no marker 0 or in a Midcup export the installed ID); 0 to 100, a stock boots model; ignored when the folder's boots models or a boots link get an ID (pre-Fox face-XML boots do not)",
        "\"\" = default (fpc_on: hidden, fpc_off 0, no marker 0 or in a Midcup export the installed ID); 0 to 100, a stock gloves model (0 = normal hands); ignored when the folder's gloves models or a gloves link get an ID (pre-Fox face-XML gloves do not)",
        "0 white, 1 light, 2 fair, 3 medium, 4 olive, 5 brown, 6 black, 7 custom (invisible body, PES 15 to 17 only)",
        "0 black, 1 dark brown, 2 brown, 3 sable, 4 navy blue, 5 charcoal, 6 gray, 7 blue, 8 sienna, 9 green, 10 violet",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "\"seasonal\", \"short\", \"long\"",
        "\"off\", \"normal\", \"turtleneck\"",
        "\"standard\", \"long\", \"short\"",
        "\"off\", \"short\", \"winter_long\" (none in summer, long in winter), \"short_winter_long\" (short in summer, long in winter)",
        "true, false",
        "true, false",
        "\"off\", \"right\", \"left\", \"both\"",
        "0 to 7",
        "0 to 7",
        "0 none, 1 rectangle rimless, 2 rectangle half frame, 3 rectangle full frame, 4 oval rimless, 5 oval half frame, 6 oval full frame, 7 round full frame",
        "0 white, 1 black, 2 red, 3 blue, 4 yellow, 5 green, 6 pink, 7 turquoise",
        "true, false (outfield player gloves)",
        "0 to 7",
        "1 to 3 (PES 20 and 21: 1 to 5)",
        "1 to 3 (PES 20 and 21: 1 to 5)",
        "1 to 8 (PES 15: 1 to 5; PES 20 and 21: 1 to 10)",
        "1 to 8 (PES 15: 1 to 5; PES 20 and 21: 1 to 10)",
        "1 to 6 (PES 20 and 21: 1 to 10)",
        "1 to 16 (PES 20 and 21: 1 to 20)",
        "1 to 4 (PES 20 and 21: 1 to 7)",
        "0 to 3, PES 20 and 21 only",
        "0 none, then PES 15: 1 to 112, 16 and 17: 1 to 122, 18: 1 to 127, 19: 1 to 140, 20: 1 to 152, 21: 1 to 162",
        "0 none, then PES 15: 1 to 112, 16 and 17: 1 to 122, 18: 1 to 127, 19: 1 to 140, 20: 1 to 152, 21: 1 to 162",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7 (PES 16 to 21: no editor slider)",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7",
        "-7 to 7, drawn by PES 15 only",
        "-7 to 7, drawn by PES 15 only",
        "-7 to 7, drawn by PES 15 only",
        "-7 to 7, drawn by PES 15 only",
        "-7 to 7, drawn by PES 15 only",
        "-7 to 7, drawn by PES 15 only",
        "-7 to 7, drawn by PES 15 only",
        "0 to 3 (PES 15: 0 to 14; PES 19: 0 to 5)",
        "0 to 5 (PES 15 and 19: 0 to 6)",
        "0 to 12 (PES 15: 0 to 29; PES 20 and 21: 0 to 19)",
        "0 to 4 (PES 15: 0 to 6)",
        "0 to 6 (PES 15: 0 to 14; PES 20 and 21: 0 to 7)",
        "0 to 2 (PES 15: 0 to 14; PES 20 and 21: 0 to 6)",
        "0 to 5 (PES 15: 0 to 17; PES 20 and 21: 0 to 7)",
        "0 to 2 (PES 15: 0 to 5; PES 20 and 21: 0 to 3)",
        "0 to 6 (PES 15: 0 to 11; PES 20 and 21: 0 to 7)",
        "-1 to 1; PES 15 only",
        "0 to 3 (PES 15: 0 to 7; PES 20 and 21: 0 to 4)",
        "0 to 2 (PES 15: 0 to 7; PES 20 and 21: 0 to 4)",
        "0 to 63",
        "0 to 63",
        "0 to 63",
        "0 normal, 1 thick, 2 thin",
        "0 to 3",
        "0 to 3",
        "0 to 63",
        "0 to 63",
        "0 to 63",
        "0 to 3",
        "0 to 3, brighter eye whites, drawn by PES 15 only",
        "0 to 63",
        "0 to 63",
        "0 to 63",
        "0 custom (the colour above), 1 to 8 the editor's presets",
        "true, false (PES 15: the game sets it with the variation)",
        "0 white, 1 dark grey, 2 red, 3 blue, 4 yellow, 5 green, 6 magenta, 7 cyan",
        "1 normal, 2 dry, 3 mohican, 4 afro, 5 dreadlocks, 6 braided, 7 special, 0 in some saves; PES 16 to 21",
        "1 to 6, 0 in some saves; PES 16 to 21",
        "1 shaven, 2 very short, 3 short, 4 medium, 5 long, 0 in some saves; PES 16 to 21",
        "0 to 6, -1 in some saves; PES 16 to 21",
        "0 to 29; PES 16 to 21",
        "1 up, 2 down, 3 combed back, 0 in some saves; PES 16 to 21",
        "1 off, 2 left 2, 3 left 1, 4 centre, 5 right 1, 6 right 2, 0 in some saves; PES 16 to 21",
        "1 normal, 2 reduce body, 3 shorter sides, 4 cropped, 0 in some saves; PES 16 to 21",
        "1 to 3, 0 in some saves; PES 16 to 21",
        "1 narrow, 2 normal, 3 wide, 0 in some saves; PES 16 to 21",
        "1 skinhead, 2 very short, 3 short, 4 medium, 5 long, 6 special; PES 15 only",
        "0 none (skinhead, special), 1 straight, 2 wavy, 3 permed, 4 dreadlocks; PES 15 only",
        "1 to 82, the editor's grid position (each length and type has its own grid); PES 15 only",
    ];
    let tables = [
        "appearance",
        "appearance.physique",
        "appearance.strip",
        "appearance.motion",
        "appearance.face",
        "appearance.hair",
    ];
    // `zip` stops at the shorter side: a key added to `ALL` alone would
    // otherwise go unchecked.
    assert_eq!(comments.len(), SettingKey::ALL.len(), "one comment per key");
    for (key, comment) in SettingKey::ALL.iter().zip(comments) {
        let spec = key.spec();
        assert!(!spec.comment.is_empty(), "{key:?} comment");
        assert_eq!(spec.comment, comment, "{key:?} comment");
        assert!(
            spec.table == KeyTable::Top || tables.contains(&spec.table.path()),
            "{key:?} table {:?}",
            spec.table
        );
        match spec.kind {
            Kind::Labels(labels) => assert!(!labels.is_empty(), "{key:?} labels"),
            Kind::OneBased { max } => assert!(max >= 1, "{key:?} one-based max"),
            Kind::Number { min, max } => assert!(min <= max, "{key:?} number range"),
            Kind::Signed { limit } => assert!(limit >= 1, "{key:?} signed limit"),
            Kind::PlusOne { max } => assert!(max >= 1, "{key:?} plus-one max"),
            Kind::Mapped(map) => assert!(map.len() >= 2, "{key:?} map"),
            Kind::Bool => {}
        }
    }
}

#[test]
fn an_unset_settings_is_empty_and_default() {
    let settings = PlayerSettings::default();
    for key in SettingKey::ALL {
        assert_eq!(settings.get(key), None, "{key:?}");
    }
    assert_eq!(settings.appearance, AppearanceSettings::default());
}

/// `ingame_face` reads the settings' face keys as `ingame_face.toml` holds
/// them; `set_ingame_face` writes every one of the file's keys back over
/// them, a `None` included, and touches nothing else of the settings.
#[test]
fn set_ingame_face_moves_the_face_keys_and_leaves_the_rest() {
    let mut settings = PlayerSettings::parse("[appearance]\nskin_color = 4\n").expect("parses");
    settings.set_ingame_face(IngameFaceSettings::parse("[hair]\nlength = 5\n").expect("parses"));
    let face = IngameFaceSettings::parse("iris_color = 7\n").expect("parses");
    settings.set_ingame_face(face.clone());
    assert_eq!(settings.appearance.skin_color, Some(4));
    assert_eq!(settings.appearance.iris_color, Some(7));
    assert_eq!(
        settings.appearance.hair.length, None,
        "a key the file does not set is unset"
    );
    assert_eq!(settings.ingame_face(), face);
}
