use super::*;

// The TOML half: the template, the parser's errors, `update_toml`.

/// The `settings.toml` template the tests hold (`tests/settings_template.toml`):
/// the spec block minus the in-game face's 90 keys, `ingame_face.toml`'s.
/// `to_toml` of the settings it parses to must reproduce it byte for byte.
const PLAN_BLOCK: &str = include_str!("settings_template.toml");

#[test]
fn to_toml_of_the_parsed_plan_block_is_the_plan_block() {
    let settings = PlayerSettings::parse(PLAN_BLOCK).expect("the plan block parses");
    assert_eq!(
        settings.name,
        Some(NameSetting::Explicit("Snuffy".to_string()))
    );
    assert_eq!(settings.shirt_name, Some("SNUFFY".to_string()));
    assert_eq!(settings.to_toml().expect("emit"), PLAN_BLOCK);
}

/// Every value the spec block shows is the key's default: the block is the
/// spec, `KeySpec::default` its stored form (a physique `0` is stored 7, a
/// motion `1` stored 0, a label its index). `""` is no stored value, so the
/// two stock IDs are `None` like an absent key. The in-game face's keys are
/// `ingame_face.toml`'s, `None` here.
#[test]
fn the_plan_block_shows_every_keys_default() {
    let settings = PlayerSettings::parse(PLAN_BLOCK).expect("the plan block parses");
    for key in SettingKey::ALL {
        let expected = if key.is_ingame_face() {
            None
        } else {
            key.spec().default
        };
        assert_eq!(settings.get(key), expected, "{key:?}");
    }
}

#[test]
fn to_toml_of_a_default_is_a_fully_commented_template() {
    let text = PlayerSettings::default().to_toml().expect("emit");
    // The in-game face's keys are `ingame_face.toml`'s: no trace of them here.
    for absent in ["[appearance.face]", "[appearance.hair]", "iris_color"] {
        assert!(!text.contains(absent), "{absent} in {text}");
    }
    for key in SettingKey::ALL {
        let spec = key.spec();
        if key.is_ingame_face() {
            continue;
        }
        if spec.table == KeyTable::Top {
            // The stock-model IDs are never commented: `""` is the shown
            // default value.
            assert!(
                text.contains(&format!("{} = \"\"", spec.name)),
                "{key:?} is not an uncommented \"\" line"
            );
        } else {
            assert!(
                text.contains(&format!("# {} = ", spec.name)),
                "{key:?} is not a commented line"
            );
        }
        assert!(text.contains(spec.comment), "{key:?} comment absent");
    }
    assert!(text.contains("# name = true"), "name is commented");
    // Every key's line carries its default, the template's shown value.
    for (name, prefix) in [
        ("skin_color", "# skin_color = 1"),
        ("neck_length", "# neck_length = 0"),
        ("hunching_dribbling", "# hunching_dribbling = 1"),
        ("untucked", "# untucked = true"),
        ("sleeves", "# sleeves = \"short\""),
        ("name", "# name = true"),
        ("shirt_name", "# shirt_name = \"\""),
        ("boots_id", "boots_id = \"\""),
        ("gloves_id", "gloves_id = \"\""),
    ] {
        let line = text
            .lines()
            .find(|line| line.contains(&format!("{name} =")))
            .unwrap_or_else(|| panic!("a line for {name}"));
        assert!(line.starts_with(prefix), "{line:?}");
    }
    assert_eq!(
        PlayerSettings::parse(&text).expect("a commented file parses"),
        PlayerSettings::default(),
        "a commented file is an empty settings"
    );
    // No `[appearance.unknown]` table, not even commented: a template for a
    // new player folder has no player's bits to carry.
    assert!(
        !text.lines().any(|line| line.contains("unknown")),
        "an unknown line emitted: {text}"
    );
}

#[test]
fn name_true_parses_and_emits_true() {
    let settings = PlayerSettings::parse("name = true\n").expect("parses");
    assert_eq!(settings.name, Some(NameSetting::FromFolder));
    assert!(
        settings
            .to_toml()
            .expect("emit")
            .lines()
            .any(|line| line == "name = true"),
        "{}",
        settings.to_toml().expect("emit")
    );
}

#[test]
fn a_stored_value_the_kind_cannot_represent_is_out_of_range() {
    let mut player = find_player(
        &payload(PesVersion::Pes19),
        schema_for(PesVersion::Pes19),
        70101,
    );
    player.appearance.sleeves = 3;
    match PlayerSettings::from_player(&player) {
        Err(SettingsError::OutOfRange { key, value, .. }) => {
            assert_eq!(key, "appearance.strip.sleeves");
            assert_eq!(value, 3);
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }
}

#[test]
fn update_toml_refuses_a_table_position_holding_a_value() {
    let settings = PlayerSettings::parse(PLAN_BLOCK).expect("the plan block parses");
    for (text, key) in [
        ("appearance = 5\n", "appearance"),
        ("[appearance]\nstrip = 1\n", "appearance.strip"),
    ] {
        let mut document: DocumentMut = text.parse().expect("document parses");
        match settings.update_toml(&mut document) {
            Err(SettingsError::WrongType { key: got, expected }) => {
                assert_eq!(got, key, "{text:?}");
                assert_eq!(expected, "a table");
            }
            other => panic!("{text:?}: expected WrongType, got {other:?}"),
        }
    }
}

/// A name carrying the save's colour codes (`\x11c` + eight hex digits)
/// must survive the document text byte for byte: `to_toml` escapes the
/// control bytes in the TOML string and `parse` reads them back — a raw
/// 0x11 in the text would leave the file unparsable or the name corrupted.
/// The fixture is PES 19's player 71803, picked by scanning the fixture's
/// names for '\x11' (the PES 19 and 21 saves each hold about a hundred).
#[test]
fn a_colour_coded_name_survives_the_toml_round_trip() {
    let player = find_player(
        &payload(PesVersion::Pes19),
        schema_for(PesVersion::Pes19),
        71803,
    );
    assert!(
        player.name.contains('\x11'),
        "the fixture name carries a code: {:?}",
        player.name
    );
    let settings = PlayerSettings::from_player(&player).expect("from_player");
    let text = settings.to_toml().expect("emit");
    assert!(!text.contains('\x11'), "a raw control byte in {text}");
    let reparsed = PlayerSettings::parse(&text).expect("the emitted text parses");
    assert_eq!(
        reparsed.name,
        Some(NameSetting::Explicit(player.name.clone())),
        "the name byte for byte"
    );
}

#[test]
fn to_toml_round_trips_a_real_player_through_text() {
    for version in [PesVersion::Pes19, PesVersion::Pes15] {
        let player = find_player(&payload(version), schema_for(version), 70101);
        let settings = PlayerSettings::from_player(&player).expect("from_player");
        let text = settings.to_toml().expect("emit");
        let reparsed = PlayerSettings::parse(&text).expect("the emitted text parses");
        // `settings.toml` holds no in-game face key: the reparsed settings
        // are the same but for the face, which `ingame_face.toml` carries.
        let mut cleared = settings.clone();
        cleared.set_ingame_face(IngameFaceSettings::default());
        assert_eq!(reparsed, cleared, "{version:?} player 70101 through text");
        // The face survives its own file's round trip.
        let face_text = settings.ingame_face().to_toml().expect("emit");
        assert_eq!(
            IngameFaceSettings::parse(&face_text).expect("the emitted text parses"),
            settings.ingame_face(),
            "{version:?} player 70101's face through text"
        );
        assert_eq!(
            reparsed.shirt_name,
            Some(player.shirt_name.clone()),
            "{version:?} shirt name through text"
        );
        if version == PesVersion::Pes15 {
            assert!(
                text.lines().any(|line| line.starts_with("# dribbling =")),
                "dribbling stays commented on PES 15"
            );
        }
    }
}

/// `[appearance.unknown]` left the format once every bit of the run was a
/// key or measured unused: it is an unknown table like any other.
#[test]
fn appearance_unknown_is_an_unknown_table() {
    let err = PlayerSettings::parse(&format!(
        "[appearance.unknown]\nface = \"{}\"\n",
        "0".repeat(100)
    ))
    .expect_err("a retired table");
    assert!(
        matches!(err, SettingsError::UnknownKey { ref key } if key == "appearance.unknown"),
        "{err:?}"
    );
}

/// The three kinds the face run brought: a signed angle stored plus 1, the
/// wave level stored plus 1, the colour preset through its map; each
/// emitted back as written, each refused past its range.
#[test]
fn the_face_runs_kinds_convert_both_ways() {
    let text = "[face]\npes15_nose_angle = -1\n\n\
                [hair]\ncolor_preset = 1\nwave_level = 0\n";
    let face = IngameFaceSettings::parse(text).expect("parses");
    let appearance = ingame_face_as_appearance(&face);
    assert_eq!(
        get_appearance(&appearance, SettingKey::Pes15NoseAngle),
        Some(0)
    );
    assert_eq!(
        get_appearance(&appearance, SettingKey::HairColorPreset),
        Some(5),
        "the map's index 1"
    );
    assert_eq!(
        get_appearance(&appearance, SettingKey::HairWaveLevel),
        Some(1)
    );
    // Every editor preset and the id the game stores for it (measured in
    // the games), both ways.
    for (index, id) in [0u8, 5, 6, 7, 1, 3, 2, 8, 4].into_iter().enumerate() {
        let text = format!("[hair]\ncolor_preset = {index}\n");
        let preset = IngameFaceSettings::parse(&text).expect("parses");
        assert_eq!(preset.hair.color_preset, Some(id), "{text}");
        let emitted = preset.to_toml().expect("emits");
        assert!(
            emitted.contains(&format!("\ncolor_preset = {index} ")),
            "{text}{emitted}"
        );
    }
    let emitted = face.to_toml().expect("emits");
    for line in [
        "pes15_nose_angle = -1",
        "color_preset = 1",
        "wave_level = 0",
    ] {
        assert!(
            emitted.lines().any(|l| l.starts_with(line)),
            "{line} in {emitted}"
        );
    }
    for (text, key, range) in [
        (
            "[face]\npes15_nose_angle = 2\n",
            "face.pes15_nose_angle",
            "-1 to 1",
        ),
        ("[hair]\ncolor_preset = 9\n", "hair.color_preset", "0 to 8"),
        ("[hair]\nwave_level = 7\n", "hair.wave_level", "-1 to 6"),
        ("[hair]\nwave_level = -2\n", "hair.wave_level", "-1 to 6"),
    ] {
        match IngameFaceSettings::parse(text) {
            Err(SettingsError::OutOfRange {
                key: k, range: r, ..
            }) => {
                assert_eq!((k.as_str(), r.as_str()), (key, range), "{text}");
            }
            other => panic!("{text}: expected OutOfRange, got {other:?}"),
        }
    }
}

/// The hair keys' ranges, copied from the spec's `[hair]` block of
/// `ingame_face.toml`: each key's maximum parses, one past it is `OutOfRange`
/// with the spec's range, and so is one below a range that starts at 1.
/// (`wave_level` and `color_preset` are pinned above.)
#[test]
fn the_hair_keys_ranges_are_the_plans() {
    let out_of_range = |key: &str, value: i64, range: &str| {
        let text = format!("[hair]\n{key} = {value}\n");
        match IngameFaceSettings::parse(&text) {
            Err(SettingsError::OutOfRange {
                key: k, range: r, ..
            }) => assert_eq!(
                (k.as_str(), r.as_str()),
                (format!("hair.{key}").as_str(), range),
                "{text}"
            ),
            other => panic!("{text}: expected OutOfRange, got {other:?}"),
        }
    };
    for (key, min, max) in [
        ("color_red", 0, 63),
        ("color_green", 0, 63),
        ("color_blue", 0, 63),
        ("accessory_color", 0, 7),
        ("style", 0, 7),
        ("cropped_style", 0, 6),
        ("length", 0, 5),
        ("variation", 0, 29),
        ("front_style", 0, 3),
        ("parted", 0, 6),
        ("side_back_style", 0, 4),
        ("hairline", 0, 3),
        ("forehead_width", 0, 3),
        ("pes15_length", 1, 6),
        ("pes15_type", 0, 4),
        ("pes15_variation", 1, 82),
    ] {
        let range = format!("{min} to {max}");
        IngameFaceSettings::parse(&format!("[hair]\n{key} = {max}\n"))
            .unwrap_or_else(|e| panic!("{key} = {max}: {e}"));
        out_of_range(key, max + 1, &range);
        if min == 1 {
            out_of_range(key, 0, &range);
        }
    }
}

#[test]
fn parse_reports_the_first_error_with_its_key() {
    let cases: [(&str, SettingsError); 13] = [
        (
            "appearance.beard = 1",
            SettingsError::UnknownKey {
                key: "appearance.beard".to_string(),
            },
        ),
        // Height and weight left the key table: the record fields they were
        // are gameplay, so a settings.toml naming either is a stranger.
        (
            "[appearance.physique]\nheight = 180\n",
            SettingsError::UnknownKey {
                key: "appearance.physique.height".to_string(),
            },
        ),
        (
            "[appearance.physique]\nweight = 75\n",
            SettingsError::UnknownKey {
                key: "appearance.physique.weight".to_string(),
            },
        ),
        (
            "[appearance.strip]\nboots_id = 5\n",
            SettingsError::UnknownKey {
                key: "appearance.strip.boots_id".to_string(),
            },
        ),
        (
            "[appearance]\nskin_color = 8\n",
            SettingsError::OutOfRange {
                key: "appearance.skin_color".to_string(),
                value: 8,
                range: "0 to 7".to_string(),
            },
        ),
        (
            "[appearance.physique]\nneck_length = 8\n",
            SettingsError::OutOfRange {
                key: "appearance.physique.neck_length".to_string(),
                value: 8,
                range: "-7 to 7".to_string(),
            },
        ),
        (
            "[appearance.motion]\nhunching_dribbling = 0\n",
            SettingsError::OutOfRange {
                key: "appearance.motion.hunching_dribbling".to_string(),
                value: 0,
                range: "1 to 5".to_string(),
            },
        ),
        (
            "[appearance.strip]\nsleeves = \"medium\"\n",
            SettingsError::UnknownLabel {
                key: "appearance.strip.sleeves".to_string(),
                label: "medium".to_string(),
                allowed: "\"seasonal\", \"short\", \"long\"".to_string(),
            },
        ),
        (
            "[appearance.strip]\nuntucked = 1\n",
            SettingsError::WrongType {
                key: "appearance.strip.untucked".to_string(),
                expected: "true or false",
            },
        ),
        (
            "name = false",
            SettingsError::WrongType {
                key: "name".to_string(),
                expected: "true or a string",
            },
        ),
        (
            "[appearance.physique]\nneck_length = 1.5\n",
            SettingsError::WrongType {
                key: "appearance.physique.neck_length".to_string(),
                expected: "an integer",
            },
        ),
        (
            "stats = {}",
            SettingsError::UnknownKey {
                key: "stats".to_string(),
            },
        ),
        ("this is not toml", SettingsError::Toml(String::new())),
    ];
    for (text, expected) in cases {
        let error = PlayerSettings::parse(text).expect_err(text);
        match (&error, &expected) {
            (SettingsError::Toml(_), SettingsError::Toml(_)) => {}
            _ => assert_eq!(error.to_string(), expected.to_string(), "parsing {text:?}"),
        }
    }
}

#[test]
fn update_toml_rewrites_values_and_keeps_the_users_comments() {
    let mut document: DocumentMut = "[appearance.strip]\n\
# the sleeves we want\n\
sleeves = \"short\"   # keep me\n\
"
    .parse()
    .expect("document parses");
    let mut settings = PlayerSettings::default();
    settings.set(SettingKey::Sleeves, 2).expect("in range");
    settings.update_toml(&mut document).expect("update");
    assert_eq!(
        document.to_string(),
        "[appearance.strip]\n# the sleeves we want\nsleeves = \"long\"   # keep me\n"
    );

    // An in-game face key is never written: `ingame_face.toml` holds them.
    let mut settings = PlayerSettings::default();
    settings.set(SettingKey::CheekType, 2).expect("in range");
    settings.set(SettingKey::NeckLength, 2).expect("in range");
    settings.update_toml(&mut document).expect("update");
    let text = document.to_string();
    assert!(
        text.contains("[appearance.physique]"),
        "a missing table is created as a standard table: {text}"
    );
    assert!(
        !text.contains("cheek_type") && !text.contains("[appearance.face]"),
        "an in-game face key is never written: {text}"
    );
    assert_eq!(
        PlayerSettings::parse(&text)
            .expect("the updated document parses")
            .appearance
            .physique
            .neck_length,
        Some(2)
    );
}

#[test]
fn update_toml_keeps_an_inline_table_inline() {
    let mut document: DocumentMut = "appearance = { skin_color = 1 }\n"
        .parse()
        .expect("document parses");
    let mut settings = PlayerSettings::default();
    settings.set(SettingKey::Sleeves, 2).expect("in range");
    settings.update_toml(&mut document).expect("update");
    let text = document.to_string();
    assert!(text.starts_with("appearance = {"), "{text}");
    let parsed = PlayerSettings::parse(&text).expect("the updated document parses");
    assert_eq!(parsed.appearance.skin_color, Some(1));
    assert_eq!(parsed.appearance.strip.sleeves, Some(2));
}

#[test]
fn a_value_the_kind_cannot_represent_is_refused_everywhere() {
    // `set` refuses it.
    let mut settings = PlayerSettings::default();
    match settings.set(SettingKey::Sleeves, 3) {
        Err(SettingsError::OutOfRange { key, value, .. }) => {
            assert_eq!(key, "appearance.strip.sleeves");
            assert_eq!(value, 3);
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }
    // A public field written directly: `to_toml`, `update_toml` and `apply`
    // all refuse with the same error, and `apply` leaves the player alone.
    settings.appearance.strip.sleeves = Some(3);
    match settings.to_toml() {
        Err(SettingsError::OutOfRange { key, value, .. }) => {
            assert_eq!(key, "appearance.strip.sleeves");
            assert_eq!(value, 3);
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }
    let mut document: DocumentMut = "[appearance.strip]\nsleeves = \"short\"\n"
        .parse()
        .expect("document parses");
    match settings.update_toml(&mut document) {
        Err(SettingsError::OutOfRange { key, .. }) => {
            assert_eq!(key, "appearance.strip.sleeves")
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }
    let mut player = find_player(
        &payload(PesVersion::Pes19),
        schema_for(PesVersion::Pes19),
        70101,
    );
    let before = player.clone();
    match settings.apply(&mut player, PesVersion::Pes19) {
        Err(SettingsError::OutOfRange { key, .. }) => {
            assert_eq!(key, "appearance.strip.sleeves")
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }
    assert_eq!(player, before, "the refused write changed the player");
}

/// The wrist tape colours' range is the game's four (strip color, white,
/// black, beige): 3 compiles to 3, one past it is the `0 to 3` refusal.
#[test]
fn the_wrist_tape_colours_range_is_the_games_four() {
    let parsed =
        PlayerSettings::parse("[appearance.strip]\nwrist_tape_color_left = 3\n").expect("3 parses");
    assert_eq!(parsed.appearance.strip.wrist_tape_color_left, Some(3));
    let mut player = find_player(
        &payload(PesVersion::Pes19),
        schema_for(PesVersion::Pes19),
        70101,
    );
    parsed
        .apply(&mut player, PesVersion::Pes19)
        .expect("applies");
    assert_eq!(player.appearance.wrist_tape_color_left, 3);

    match PlayerSettings::parse("[appearance.strip]\nwrist_tape_color_left = 4\n") {
        Err(SettingsError::OutOfRange { key, value, range }) => {
            assert_eq!(key, "appearance.strip.wrist_tape_color_left");
            assert_eq!((value, range.as_str()), (4, "0 to 3"));
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }
}

#[test]
fn apply_writes_a_set_shirt_name_and_leaves_an_unset_one() {
    let payload = payload(PesVersion::Pes19);
    let schema = schema_for(PesVersion::Pes19);
    let mut player = find_player(&payload, schema, 70101);
    let before = player.shirt_name.clone();
    PlayerSettings {
        shirt_name: Some("SNUFFY".to_string()),
        ..PlayerSettings::default()
    }
    .apply(&mut player, PesVersion::Pes19)
    .expect("apply");
    assert_eq!(player.shirt_name, "SNUFFY");
    let mut player = find_player(&payload, schema, 70101);
    PlayerSettings::default()
        .apply(&mut player, PesVersion::Pes19)
        .expect("apply");
    assert_eq!(
        player.shirt_name, before,
        "an unset shirt_name is untouched"
    );
}

/// A `name`/`shirt_name` at its field's capacity applies; one over is
/// `TextTooLong` naming the key, and the player is unchanged. The two
/// versions' shirt-name fields differ (18 bytes on PES 19, 61 on PES 21).
#[test]
fn a_name_over_the_versions_field_is_refused_on_two_versions() {
    assert_ne!(
        text_max(
            schema_for(PesVersion::Pes19).player.texts,
            PlayerText::ShirtName
        ),
        text_max(
            schema_for(PesVersion::Pes21).player.texts,
            PlayerText::ShirtName
        ),
        "the test needs two different field lengths"
    );
    for version in [PesVersion::Pes19, PesVersion::Pes21] {
        let payload = payload(version);
        let schema = schema_for(version);
        let texts = schema.player.texts;
        let name_max = text_max(texts, PlayerText::Name);
        let shirt_max = text_max(texts, PlayerText::ShirtName);

        // At capacity applies. The shirt name's last char is `é` (U+00E9, two
        // UTF-8 bytes), so the string is `shirt_max` chars over `shirt_max`
        // bytes: it fits because the single-byte field counts chars.
        let full_shirt = format!("{}é", "S".repeat(shirt_max - 1));
        let mut player = find_player(&payload, schema, 70101);
        PlayerSettings {
            name: Some(NameSetting::Explicit("N".repeat(name_max))),
            shirt_name: Some(full_shirt.clone()),
            ..PlayerSettings::default()
        }
        .apply(&mut player, version)
        .expect("at capacity applies");
        assert_eq!(player.name, "N".repeat(name_max));
        assert_eq!(player.shirt_name, full_shirt);

        // `name` counts UTF-8 bytes, not chars: `é` is two bytes, so a
        // `name` of `é`s (plus one ASCII char when the capacity is odd)
        // fills the field exactly and applies; one `é` more still fits
        // in chars but its bytes do not.
        let full_name = if name_max.is_multiple_of(2) {
            "é".repeat(name_max / 2)
        } else {
            format!("N{}", "é".repeat(name_max / 2))
        };
        assert_eq!(
            full_name.len(),
            name_max,
            "the name fills the byte capacity exactly"
        );
        let mut player = find_player(&payload, schema, 70101);
        PlayerSettings {
            name: Some(NameSetting::Explicit(full_name.clone())),
            ..PlayerSettings::default()
        }
        .apply(&mut player, version)
        .expect("a name at the byte capacity applies");
        assert_eq!(player.name, full_name);
        let mut player = find_player(&payload, schema, 70101);
        let before = player.clone();
        match (PlayerSettings {
            name: Some(NameSetting::Explicit(format!("{full_name}é"))),
            ..PlayerSettings::default()
        })
        .apply(&mut player, version)
        {
            Err(SettingsError::TextTooLong {
                key: got,
                max: got_max,
            }) => assert_eq!((got.as_str(), got_max), ("name", name_max), "{version:?}"),
            other => panic!("{version:?} name: expected TextTooLong, got {other:?}"),
        }
        assert_eq!(player, before, "the refused write changed the player");

        for (settings, key, max) in [
            (
                PlayerSettings {
                    name: Some(NameSetting::Explicit("N".repeat(name_max + 1))),
                    ..PlayerSettings::default()
                },
                "name",
                name_max,
            ),
            (
                PlayerSettings {
                    shirt_name: Some("S".repeat(shirt_max + 1)),
                    ..PlayerSettings::default()
                },
                "shirt_name",
                shirt_max,
            ),
        ] {
            let mut player = find_player(&payload, schema, 70101);
            let before = player.clone();
            match settings.apply(&mut player, version) {
                Err(SettingsError::TextTooLong {
                    key: got,
                    max: got_max,
                }) => {
                    assert_eq!((got.as_str(), got_max), (key, max), "{version:?}");
                }
                other => panic!("{version:?} {key}: expected TextTooLong, got {other:?}"),
            }
            assert_eq!(player, before, "the refused write changed the player");
        }
    }
}

/// `NameField::check` is `apply`'s text check standalone: at the field's
/// capacity passes, one byte longer is `TextTooLong` naming the key.
#[test]
fn name_field_check_is_applys_check() {
    let texts = schema_for(PesVersion::Pes21).player.texts;
    for (field, max) in [
        (NameField::Name, text_max(texts, PlayerText::Name)),
        (NameField::ShirtName, text_max(texts, PlayerText::ShirtName)),
    ] {
        let key = field.key();
        field
            .check(&"N".repeat(max), PesVersion::Pes21)
            .expect("at capacity");
        match field.check(&"N".repeat(max + 1), PesVersion::Pes21) {
            Err(SettingsError::TextTooLong {
                key: got,
                max: got_max,
            }) => assert_eq!((got.as_str(), got_max), (key, max)),
            other => panic!("{key}: expected TextTooLong, got {other:?}"),
        }
    }
}

#[test]
fn a_shirt_name_that_is_not_a_plain_string_is_refused() {
    // A char above U+00FF does not fit the single-byte field: `WrongType` at
    // apply, and the player is unchanged.
    let mut player = find_player(
        &payload(PesVersion::Pes19),
        schema_for(PesVersion::Pes19),
        70101,
    );
    let before = player.clone();
    match (PlayerSettings {
        shirt_name: Some("Ł".to_string()),
        ..PlayerSettings::default()
    })
    .apply(&mut player, PesVersion::Pes19)
    {
        Err(SettingsError::WrongType { key, expected }) => {
            assert_eq!(key, "shirt_name");
            assert_eq!(expected, "a string of chars at most U+00FF");
        }
        other => panic!("expected WrongType, got {other:?}"),
    }
    assert_eq!(player, before, "the refused write changed the player");

    // At parse: a NUL is refused as `name`'s is, a non-string as `a string`.
    match PlayerSettings::parse("shirt_name = \"a\\u0000b\"\n") {
        Err(SettingsError::WrongType { key, expected }) => {
            assert_eq!(key, "shirt_name");
            assert_eq!(expected, "a string without NUL");
        }
        other => panic!("expected WrongType, got {other:?}"),
    }
    match PlayerSettings::parse("shirt_name = 5\n") {
        Err(SettingsError::WrongType { key, expected }) => {
            assert_eq!(key, "shirt_name");
            assert_eq!(expected, "a string");
        }
        other => panic!("expected WrongType, got {other:?}"),
    }
}

#[test]
fn update_toml_writes_a_set_shirt_name_and_keeps_the_comment() {
    let mut document: DocumentMut = "shirt_name = \"OLD\"   # keep me\n"
        .parse()
        .expect("document parses");
    PlayerSettings {
        shirt_name: Some("SNUFFY".to_string()),
        ..PlayerSettings::default()
    }
    .update_toml(&mut document)
    .expect("update");
    assert_eq!(
        document.to_string(),
        "shirt_name = \"SNUFFY\"   # keep me\n"
    );
}

#[test]
fn an_explicit_name_with_nul_is_refused() {
    match PlayerSettings::parse("name = \"a\\u0000b\"\n") {
        Err(SettingsError::WrongType { key, expected }) => {
            assert_eq!(key, "name");
            assert_eq!(expected, "a string without NUL");
        }
        other => panic!("expected WrongType, got {other:?}"),
    }
}

#[test]
fn accepts_stored_bounds_each_kind() {
    for (kind, accepted, refused) in [
        (Kind::Number { min: 0, max: 7 }, 7, 8),
        (Kind::Signed { limit: 7 }, 14, 15),
        (Kind::Signed { limit: 1 }, 2, 3),
        (Kind::OneBased { max: 5 }, 4, 5),
        (Kind::PlusOne { max: 6 }, 7, 8),
        (Kind::PlusOne { max: 6 }, 0, 8),
        (Kind::Mapped(&[0, 5, 6, 7, 1, 3, 2, 8, 4]), 8, 9),
        (Kind::Bool, 1, 2),
        (Kind::Labels(&["a", "b"]), 1, 2),
    ] {
        assert!(kind.accepts_stored(accepted), "{kind:?} {accepted}");
        assert!(!kind.accepts_stored(refused), "{kind:?} {refused}");
    }
}

#[test]
fn parse_maps_the_toml_values_to_the_stored_ones() {
    let settings = PlayerSettings::parse(
        "[appearance.physique]\nneck_length = -7\n\
[appearance.motion]\nfree_kick = 20\n\
[appearance.strip]\nwrist_taping = \"both\"\n",
    )
    .expect("parses");
    assert_eq!(settings.get(SettingKey::NeckLength), Some(0));
    assert_eq!(settings.get(SettingKey::FreeKick), Some(19));
    assert_eq!(settings.get(SettingKey::WristTaping), Some(3));
    let settings =
        PlayerSettings::parse("[appearance.physique]\nneck_length = 7\n").expect("parses");
    assert_eq!(settings.get(SettingKey::NeckLength), Some(14));
}

#[test]
fn stock_model_ids_parse_the_stock_band_and_the_default() {
    // 0 and 100 are the stock band's ends; `""` is default.
    let settings = PlayerSettings::parse("boots_id = 0\ngloves_id = 100\n").expect("parses");
    assert_eq!(settings.boots_id, Some(0));
    assert_eq!(settings.gloves_id, Some(100));
    let settings = PlayerSettings::parse("boots_id = \"\"\n").expect("parses");
    assert_eq!(settings.boots_id, None);
    // An authored stock ID emits back as the integer.
    let text = PlayerSettings::parse("boots_id = 42\n")
        .expect("parses")
        .to_toml()
        .expect("emit");
    assert!(text.lines().any(|line| line.starts_with("boots_id = 42")));
    // 101 is past the stock band: the compiler's first team block.
    match PlayerSettings::parse("boots_id = 101\n") {
        Err(SettingsError::OutOfRange { key, value, range }) => {
            assert_eq!(
                (key.as_str(), value, range.as_str()),
                ("boots_id", 101, "0 to 100")
            );
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }
    // A non-empty string is neither an integer nor the default marker.
    match PlayerSettings::parse("gloves_id = \"12\"\n") {
        Err(SettingsError::WrongType { key, expected }) => {
            assert_eq!(
                (key.as_str(), expected),
                ("gloves_id", "an integer or \"\"")
            );
        }
        other => panic!("expected WrongType, got {other:?}"),
    }
}

#[test]
fn a_stock_model_id_inside_a_table_is_an_unknown_key() {
    // The keys sit at the file's top level only: the same name inside
    // `[appearance]` or `[appearance.strip]` collides with Team TOML's
    // embedded appearance tables and is refused, naming the key.
    for (text, key) in [
        ("[appearance]\nboots_id = 5\n", "appearance.boots_id"),
        (
            "[appearance.strip]\ngloves_id = 5\n",
            "appearance.strip.gloves_id",
        ),
    ] {
        match PlayerSettings::parse(text) {
            Err(SettingsError::UnknownKey { key: got }) => assert_eq!(got, key),
            other => panic!("{text:?}: expected UnknownKey, got {other:?}"),
        }
    }
}

#[test]
fn from_player_emits_an_id_only_in_the_stock_band() {
    // 0 is the game's default.
    let p21 = find_player(
        &payload(PesVersion::Pes21),
        schema_for(PesVersion::Pes21),
        70101,
    );
    assert_eq!((p21.appearance.boots_id, p21.appearance.gloves_id), (0, 0));
    let s21 = PlayerSettings::from_player(&p21).expect("from_player");
    assert_eq!((s21.boots_id, s21.gloves_id), (None, None));
    // Above 100 is custom content a folder owns, not an authorable setting.
    let p19 = find_player(
        &payload(PesVersion::Pes19),
        schema_for(PesVersion::Pes19),
        70101,
    );
    assert_eq!(
        (p19.appearance.boots_id, p19.appearance.gloves_id),
        (101, 101)
    );
    let s19 = PlayerSettings::from_player(&p19).expect("from_player");
    assert_eq!((s19.boots_id, s19.gloves_id), (None, None));
    // A stock ID in 1 to 100 emits.
    let stock = find_player(
        &payload(PesVersion::Pes21),
        schema_for(PesVersion::Pes21),
        79503,
    );
    assert_eq!(stock.appearance.gloves_id, 12);
    assert!(stock.appearance.boots_id > 100);
    let emitted = PlayerSettings::from_player(&stock).expect("from_player");
    assert_eq!((emitted.boots_id, emitted.gloves_id), (None, Some(12)));
    // The band's own ends.
    let mut player = p21.clone();
    player.appearance.boots_id = 1;
    player.appearance.gloves_id = 100;
    let emitted = PlayerSettings::from_player(&player).expect("from_player");
    assert_eq!((emitted.boots_id, emitted.gloves_id), (Some(1), Some(100)));
}

#[test]
fn apply_writes_an_authored_id_and_leaves_none_alone() {
    let payload = payload(PesVersion::Pes19);
    let schema = schema_for(PesVersion::Pes19);
    let mut player = find_player(&payload, schema, 70101);
    PlayerSettings {
        boots_id: Some(42),
        ..PlayerSettings::default()
    }
    .apply(&mut player, PesVersion::Pes19)
    .expect("apply");
    assert_eq!(player.appearance.boots_id, 42);
    assert_eq!(player.appearance.gloves_id, 101, "an unset ID is untouched");
    // `None` leaves even the game's default alone.
    let mut player = find_player(&payload, schema, 70101);
    PlayerSettings::default()
        .apply(&mut player, PesVersion::Pes19)
        .expect("apply");
    assert_eq!(
        (player.appearance.boots_id, player.appearance.gloves_id),
        (101, 101)
    );
    // `""` parses to `None` and likewise leaves the field alone.
    PlayerSettings::parse("boots_id = \"\"\n")
        .expect("parses")
        .apply(&mut player, PesVersion::Pes19)
        .expect("apply");
    assert_eq!(player.appearance.boots_id, 101);
}

#[test]
fn update_toml_writes_an_authored_id_and_keeps_the_users_default() {
    let mut document: DocumentMut = "boots_id = \"\"  # the boots comment\n\
gloves_id = \"\"   # the gloves comment\n"
        .parse()
        .expect("document parses");
    PlayerSettings {
        boots_id: Some(7),
        ..PlayerSettings::default()
    }
    .update_toml(&mut document)
    .expect("update");
    assert_eq!(
        document.to_string(),
        "boots_id = 7  # the boots comment\ngloves_id = \"\"   # the gloves comment\n"
    );
}

#[test]
fn a_hand_set_id_past_the_stock_band_is_refused_on_write() {
    // The key's 0-100 range, not the field's stored width, bounds a
    // `boots_id`/`gloves_id` written directly into a public field: 150
    // fits the storage's 14 bits but is not a stock ID.
    for (settings, name) in [
        (
            PlayerSettings {
                boots_id: Some(150),
                ..PlayerSettings::default()
            },
            "boots_id",
        ),
        (
            PlayerSettings {
                gloves_id: Some(150),
                ..PlayerSettings::default()
            },
            "gloves_id",
        ),
    ] {
        for emit in [
            settings.to_toml().map(|_| ()),
            settings.update_toml(&mut DocumentMut::new()),
        ] {
            match emit {
                Err(SettingsError::OutOfRange { key, value, range }) => {
                    assert_eq!(
                        (key.as_str(), value, range.as_str()),
                        (name, 150, "0 to 100")
                    );
                }
                other => panic!("expected OutOfRange, got {other:?}"),
            }
        }
    }
}

// `ingame_face.toml`: the in-game face's own document — `iris_color` at the
// top level, `[face]` and `[hair]` — walked with the same key table, the
// `[appearance]` root dropped.

/// The template `docs/spec/aesthetics_export/settings_toml.md` "The document"
/// describes: `to_toml` of the settings the file parses to must reproduce it
/// byte for byte.
const INGAME_FACE_FIXTURE: &str = include_str!("ingame_face_template.toml");

/// `face` read through an `AppearanceSettings`, so a test can ask a
/// `SettingKey` of it.
fn ingame_face_as_appearance(face: &IngameFaceSettings) -> AppearanceSettings {
    AppearanceSettings {
        iris_color: face.iris_color,
        face: face.face.clone(),
        hair: face.hair.clone(),
        ..AppearanceSettings::default()
    }
}

/// `iris_color`, `[face]` and `[hair]` are 90 of the 130 keys.
#[test]
fn the_ingame_face_is_90_keys() {
    assert_eq!(
        SettingKey::ALL
            .iter()
            .filter(|key| key.is_ingame_face())
            .count(),
        90
    );
}

#[test]
fn ingame_face_parses_the_fixture_and_emits_it_verbatim() {
    let face = IngameFaceSettings::parse(INGAME_FACE_FIXTURE).expect("the fixture parses");
    let appearance = ingame_face_as_appearance(&face);
    for key in SettingKey::ALL {
        if key.is_ingame_face() {
            assert_eq!(
                get_appearance(&appearance, key),
                key.spec().default,
                "{key:?}: the fixture shows the key's default"
            );
        }
    }
    assert_eq!(face.to_toml().expect("emit"), INGAME_FACE_FIXTURE);
}

#[test]
fn ingame_face_default_emits_a_fully_commented_template() {
    let text = IngameFaceSettings::default().to_toml().expect("emit");
    for key in SettingKey::ALL {
        if key.is_ingame_face() {
            assert!(
                text.contains(&format!("# {} = ", key.spec().name)),
                "{key:?} is not a commented line"
            );
        }
    }
    assert_eq!(
        IngameFaceSettings::parse(&text).expect("a commented file parses"),
        IngameFaceSettings::default(),
        "a commented file is the default face"
    );
}

/// The two files split the 130 keys: `settings.toml`'s parse holds every
/// in-game face key `None` and every other its default, `ingame_face.toml`'s
/// the reverse.
#[test]
fn the_two_templates_split_the_keys() {
    let settings = PlayerSettings::parse(PLAN_BLOCK).expect("the plan block parses");
    let face = IngameFaceSettings::parse(INGAME_FACE_FIXTURE).expect("the fixture parses");
    let appearance = ingame_face_as_appearance(&face);
    for key in SettingKey::ALL {
        let (in_settings, in_face) = if key.is_ingame_face() {
            (None, key.spec().default)
        } else {
            (key.spec().default, None)
        };
        assert_eq!(settings.get(key), in_settings, "{key:?} in settings.toml");
        assert_eq!(
            get_appearance(&appearance, key),
            in_face,
            "{key:?} in ingame_face.toml"
        );
    }
}

#[test]
fn ingame_face_parse_of_an_empty_text_is_the_default() {
    assert_eq!(
        IngameFaceSettings::parse("").expect("an empty file parses"),
        IngameFaceSettings::default()
    );
}

#[test]
fn ingame_face_parse_sets_only_the_written_keys() {
    let face =
        IngameFaceSettings::parse("iris_color = 7\n[face]\ncheek_type = 5\n[hair]\nlength = 5\n")
            .expect("parses");
    let appearance = ingame_face_as_appearance(&face);
    for key in SettingKey::ALL {
        if key.is_ingame_face() {
            let expected = match key {
                SettingKey::IrisColor => Some(7),
                SettingKey::CheekType => Some(5),
                SettingKey::HairLength => Some(5),
                _ => None,
            };
            assert_eq!(get_appearance(&appearance, key), expected, "{key:?}");
        }
    }
}

#[test]
fn ingame_face_parse_reports_the_first_error_with_its_path() {
    let cases: [(&str, SettingsError); 7] = [
        (
            "skin_color = 4",
            SettingsError::UnknownKey {
                key: "skin_color".to_string(),
            },
        ),
        (
            "[appearance.face]\ncheek_type = 5",
            SettingsError::UnknownKey {
                key: "appearance".to_string(),
            },
        ),
        (
            "[face]\nbogus = 1",
            SettingsError::UnknownKey {
                key: "face.bogus".to_string(),
            },
        ),
        (
            "[physique]",
            SettingsError::UnknownKey {
                key: "physique".to_string(),
            },
        ),
        (
            "face = 3",
            SettingsError::WrongType {
                key: "face".to_string(),
                expected: "a table",
            },
        ),
        (
            "iris_color = 11",
            SettingsError::OutOfRange {
                key: "iris_color".to_string(),
                value: 11,
                range: "0 to 10".to_string(),
            },
        ),
        (
            "[hair]\nlength = 9",
            SettingsError::OutOfRange {
                key: "hair.length".to_string(),
                value: 9,
                range: "0 to 5".to_string(),
            },
        ),
    ];
    for (text, expected) in cases {
        let error = IngameFaceSettings::parse(text).expect_err(text);
        assert_eq!(error.to_string(), expected.to_string(), "parsing {text:?}");
    }
    let error = IngameFaceSettings::parse("iris_color = ").expect_err("not TOML");
    match &error {
        SettingsError::Toml(_) => {
            assert!(
                error.to_string().starts_with("not valid TOML: "),
                "the error names no file: {error}"
            );
        }
        other => panic!("expected Toml, got {other:?}"),
    }
}

/// The in-game face's keys are `ingame_face.toml`'s: one left in
/// `settings.toml`, at either spelling of its old path, is an unknown key
/// named as `settings.toml` spells it.
#[test]
fn a_settings_toml_refuses_the_in_game_face_s_keys() {
    for (text, key) in [
        ("[appearance.hair]\nlength = 5\n", "appearance.hair"),
        ("[appearance.face]\ncheek_type = 5\n", "appearance.face"),
        ("[appearance]\niris_color = 7\n", "appearance.iris_color"),
    ] {
        match PlayerSettings::parse(text) {
            Err(SettingsError::UnknownKey { key: got }) => assert_eq!(got, key, "{text:?}"),
            other => panic!("{text:?}: expected UnknownKey, got {other:?}"),
        }
    }
}

/// A moved key's reported path is its `ingame_face.toml` one — the `key` of
/// an `OutOfRange` (and so of a `RefusedValue` or `NarrowedValue`, which it
/// feeds) names the file the key lives in.
#[test]
fn a_moved_key_s_reported_path_is_ingame_face_toml_s() {
    for (key, path) in [
        (SettingKey::IrisColor, "iris_color"),
        (SettingKey::HairStyle, "hair.style"),
        (SettingKey::FacialHairType, "face.facial_hair_type"),
    ] {
        let mut settings = PlayerSettings::default();
        match settings.set(key, u8::MAX) {
            Err(SettingsError::OutOfRange { key: got, .. }) => assert_eq!(got, path),
            other => panic!("{key:?}: expected OutOfRange, got {other:?}"),
        }
    }
}
