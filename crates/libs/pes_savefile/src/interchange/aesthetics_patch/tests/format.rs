use super::*;

/// The five required header keys at valid values; a refusal case that
/// does not mutate the header starts from them.
const KEYS: &str = "\
pes_version = 21
tool_version = \"0.1.0\"
allocation_scheme_version = 1
cpks = [\"4cc_69.cpk\"]
compiled = 2026-09-12T14:03:00Z
";

/// The header keys plus `tail`.
fn doc(tail: &str) -> String {
    format!("{KEYS}{tail}")
}

/// The sample patch `golden()` builds: the header, `/co/` 714 with slot
/// 05's names, resolved model IDs and whole `[appearance]` — every key
/// PES 21 holds, no comments — and slot 06's `base_copy_id` alone.
const GOLDEN: &str = "\
# aesthetics_patch.toml, written by the Team compiler: apply it with the Save editor's Savefile > Apply aesthetics patch
pes_version = 21
tool_version = \"0.1.0\"
allocation_scheme_version = 1
cpks = [\"4cc_69.cpk\"]
compiled = 2026-09-12T14:03:00Z

[[teams]]
id = 714
name = \"/co/\"                   # for the reader; the id is what is applied

[teams.players.05]              # player 71405
name = \"Snuffy\"
shirt_name = \"SNUFFY\"
boots_id = 143
gloves_id = 0

[teams.players.05.appearance]
skin_color = 4
iris_color = 0

[teams.players.05.appearance.physique]
neck_length = 0
neck_size = 0
shoulder_height = 0
shoulder_width = 0
chest = 0
waist = 0
arm_size = 0
arm_length = 0
thigh = 0
calf = 0
leg_length = 0
head_length = 0
head_width = 0
head_depth = 0

[teams.players.05.appearance.strip]
sleeves = \"long\"
inners = \"off\"
socks = \"standard\"
undershorts = \"off\"
untucked = true
ankle_taping = false
wrist_taping = \"off\"
wrist_tape_color_left = 0
wrist_tape_color_right = 0
spectacles = 0
spectacles_color = 0
gloves = false
gloves_color = 0

[teams.players.05.appearance.motion]
hunching_dribbling = 2
hunching_running = 1
arm_movement_dribbling = 1
arm_movement_running = 1
corner_kick = 1
free_kick = 4
penalty_kick = 1
dribbling = 0
goal_celebration_1 = 0
goal_celebration_2 = 0

[teams.players.05.appearance.presentation]
star = 0
playing_attitude = 0

[teams.players.05.appearance.face]
jawline = 0
jaw_height = 0
cheekbones = 0
cheekbone_height = 0
chin_width = 0
chin_height = 0
chin_depth = 0
face_size = 0
face_height = 0
brow_width = 0
inner_eyebrow_height = 0
eyebrow_depth = 0
temple_width = 0
outer_eyebrow_height = 0
pupil_size = 0
eye_depth = 0
inner_eye_position = 0
inner_eye_height = 0
outer_eye_position = 0
eye_corner_height = 0
upper_eyelid_width_inner = 0
upper_eyelid_height_inner = 0
upper_eyelid_width_outer = 0
upper_eyelid_height_outer = 0
bottom_eyelid_height = 0
horizontal_eye_position = 0
eye_height = 0
nose_height = 0
nose_tip_depth = 0
nostril_width = 0
nose_depth = 0
nose_width = 0
mouth_height = 0
mouth_depth = 0
lip_size = 0
mouth_corner_height = 0
lip_width = 0
ear_length = 0
ear_width = 0
ear_angle = 0
pes15_jaw_depth = 0
pes15_cheekbone_depth = 0
pes15_mouth_edge_position = 0
pes15_eyebrow_depth = 0
pes15_bottom_eyelid_position = 0
pes15_nasal_bridge_position = 0
pes15_nasal_root_height = 0
cheek_type = 0
forehead_type = 0
facial_hair_type = 0
laughter_lines_type = 0
upper_eyelid_type = 0
lower_eyelid_type = 0
eyebrow_type = 0
neck_line_type = 0
nose_type = 0
upper_lip_type = 0
lower_lip_type = 0
eyebrow_color_red = 14
eyebrow_color_green = 10
eyebrow_color_blue = 7
eyebrow_style = 0
eyebrow_density = 3
eyebrow_thickness = 1
facial_hair_color_red = 14
facial_hair_color_green = 10
facial_hair_color_blue = 7
facial_hair_thickness = 3
pes15_eye_white = 0

[teams.players.05.appearance.hair]
color_red = 14
color_green = 10
color_blue = 7
color_preset = 0
accessory = false
accessory_color = 0
style = 1
cropped_style = 1
length = 2
wave_level = 0
variation = 0
front_style = 1
parted = 1
side_back_style = 4
hairline = 1
forehead_width = 2

[teams.players.06]              # player 71406
base_copy_id = 101334
";

/// The patch `GOLDEN` reads and writes: slot 05's appearance is PES 21's
/// resolved template with a changed skin colour, sleeves, hunching and
/// free kick beside the names and IDs; slot 06 carries a copy link alone.
fn golden() -> AestheticsPatch {
    let mut appearance = AppearanceSettings::default().resolved(PesVersion::Pes21);
    appearance.skin_color = Some(4);
    appearance.strip.sleeves = Some(2);
    appearance.motion.hunching_dribbling = Some(1);
    appearance.motion.free_kick = Some(3);
    AestheticsPatch {
        pes_version: PesVersion::Pes21,
        tool_version: "0.1.0".to_string(),
        allocation_scheme_version: 1,
        cpks: vec!["4cc_69.cpk".to_string()],
        compiled: "2026-09-12T14:03:00Z".parse().expect("a datetime"),
        teams: vec![PatchTeam {
            id: 714,
            name: "/co/".to_string(),
            players: BTreeMap::from([
                (
                    5,
                    PatchPlayer {
                        name: Some("Snuffy".to_string()),
                        shirt_name: Some("SNUFFY".to_string()),
                        boots_id: Some(143),
                        gloves_id: Some(0),
                        base_copy_id: None,
                        appearance: Some(appearance),
                    },
                ),
                (
                    6,
                    PatchPlayer {
                        name: None,
                        shirt_name: None,
                        boots_id: None,
                        gloves_id: None,
                        base_copy_id: Some(101334),
                        appearance: None,
                    },
                ),
            ]),
        }],
    }
}

#[test]
fn golden_text() {
    let patch = golden();
    assert_eq!(patch.to_toml().expect("emit"), GOLDEN);
    assert_eq!(
        AestheticsPatch::parse(GOLDEN).expect("the golden parses"),
        patch
    );
}

/// The presentation table carries each key the patch's version holds:
/// both on PES 21, `star` alone on PES 19, which stores no playing
/// attitude.
#[test]
fn the_presentation_table_carries_the_keys_the_patched_version_holds() {
    let table_of = |version: PesVersion| {
        let patch = patch_at(
            version,
            vec![team(
                701,
                BTreeMap::from([(5, look(AppearanceSettings::default().resolved(version)))]),
            )],
        );
        let text = patch.to_toml().expect("emits");
        text.split_once("[teams.players.05.appearance.presentation]\n")
            .and_then(|(_, rest)| rest.split_once("\n["))
            .map(|(block, _)| block.to_owned())
            .unwrap_or_else(|| panic!("a presentation table in:\n{text}"))
    };
    assert_eq!(
        table_of(PesVersion::Pes21),
        "star = 0\nplaying_attitude = 0\n"
    );
    assert_eq!(table_of(PesVersion::Pes19), "star = 0\n");
}

/// A hand-built patch can hold a stored value no key's range expresses;
/// `to_toml` refuses it rather than writing a file `parse` would not
/// read back. `free_kick` stored 20 would emit `free_kick = 21`, one
/// past its 1 to 20.
#[test]
fn to_toml_refuses_a_value_parse_would_refuse_back() {
    let mut patch = golden();
    patch.teams[0]
        .players
        .get_mut(&5)
        .expect("player 05")
        .appearance
        .as_mut()
        .expect("the look")
        .motion
        .free_kick = Some(20);
    let err = patch.to_toml().expect_err("free_kick = 21 is out of range");
    match &err {
        PatchError::Settings(SettingsError::OutOfRange { key, .. }) => {
            assert_eq!(key, "teams[0].players.05.appearance.motion.free_kick")
        }
        other => panic!("{other:?}"),
    }
}

/// A hand-built patch can hold an `[appearance]` a key of its version is
/// `None` in: `to_toml` refuses it (`MissingKey`) rather than writing a
/// partial look `parse` would refuse.
#[test]
fn to_toml_refuses_an_appearance_missing_a_version_key() {
    let mut patch = golden();
    patch.teams[0]
        .players
        .get_mut(&5)
        .expect("player 05")
        .appearance
        .as_mut()
        .expect("the look")
        .iris_color = None;
    let err = patch.to_toml().expect_err("a partial look is not written");
    match &err {
        PatchError::MissingKey { key } => {
            assert_eq!(key, "teams[0].players.05.appearance.iris_color")
        }
        other => panic!("{other:?}"),
    }
}

/// A hand-built patch can hold a `Some` of a key its version does not:
/// `to_toml` refuses it (`NotInThisVersion`) rather than dropping it into
/// the file, where `parse` would refuse the same key.
#[test]
fn to_toml_refuses_a_key_the_version_does_not_hold() {
    let mut appearance = AppearanceSettings::default().resolved(PesVersion::Pes18);
    appearance.motion.dribbling = Some(2);
    let patch = patch_at(
        PesVersion::Pes18,
        vec![team(701, BTreeMap::from([(5, look(appearance))]))],
    );
    let err = patch.to_toml().expect_err("a foreign key is not written");
    match &err {
        PatchError::Settings(SettingsError::NotInThisVersion { key }) => {
            assert_eq!(key, "teams[0].players.05.appearance.motion.dribbling")
        }
        other => panic!("{other:?}"),
    }
}

/// A hand-built `boots_id` past the field's 14 bits is refused by
/// `to_toml` at the same path `parse` refuses the line at.
#[test]
fn to_toml_refuses_a_boots_id_past_the_field() {
    let mut patch = golden();
    patch.teams[0]
        .players
        .get_mut(&5)
        .expect("player 05")
        .boots_id = Some(16384);
    let err = patch
        .to_toml()
        .expect_err("a 14-bit field holds 0 to 16383");
    match &err {
        PatchError::Settings(SettingsError::OutOfRange { key, value, range }) => {
            assert_eq!(key, "teams[0].players.05.boots_id");
            assert_eq!(*value, 16384);
            assert_eq!(range, "0 to 16383");
        }
        other => panic!("{other:?}"),
    }
}

/// The field's top is in range: a boots or gloves ID of `MODEL_ID_MAX`
/// writes; only one past it is refused.
#[test]
fn to_toml_writes_a_model_id_at_the_field_top() {
    let mut patch = golden();
    let player = patch.teams[0].players.get_mut(&5).expect("player 05");
    player.boots_id = Some(MODEL_ID_MAX);
    player.gloves_id = Some(MODEL_ID_MAX);
    let text = patch.to_toml().expect("the field's top is in range");
    assert!(text.contains("boots_id = 16383\n"), "{text}");
    assert!(text.contains("gloves_id = 16383\n"), "{text}");
}

/// A hand-built `players` map can hold a slot outside `01` to `99`;
/// `to_toml` refuses it at the key `parse` would name.
#[test]
fn to_toml_refuses_a_player_key_outside_01_to_99() {
    for (slot, key) in [
        (0u8, "teams[0].players.00"),
        (100u8, "teams[0].players.100"),
    ] {
        let patch = patch_at(
            PesVersion::Pes21,
            vec![team(701, BTreeMap::from([(slot, named("X"))]))],
        );
        let err = patch
            .to_toml()
            .expect_err("a bad player key is not written");
        match &err {
            PatchError::Settings(SettingsError::UnknownKey { key: got }) => {
                assert_eq!(got, key, "slot {slot}")
            }
            other => panic!("slot {slot}: {other:?}"),
        }
    }
}

/// A resolved look on every version round-trips: a `[teams.players.NN]`
/// built whole (`name`, the IDs, every key the version holds changed off
/// its default where the test sets one) writes and parses back equal.
/// PES 15 holds the `pes15_` keys the later versions drop; PES 21 alone
/// holds `dribbling`.
#[test]
fn every_version_round_trips() {
    for version in FIXTURES {
        let mut appearance = AppearanceSettings::default().resolved(version);
        appearance.skin_color = Some(4);
        appearance.strip.sleeves = Some(2);
        appearance.motion.free_kick = Some(3);
        appearance.face.cheekbones = Some(5);
        appearance.hair.color_red = Some(9);
        let patch = AestheticsPatch {
            pes_version: version,
            tool_version: "0.1.0".to_string(),
            allocation_scheme_version: ALLOCATION_SCHEME_VERSION,
            cpks: vec!["4cc_69.cpk".to_string()],
            compiled: "2026-09-12T14:03:00Z".parse().expect("a datetime"),
            teams: vec![
                PatchTeam {
                    id: 714,
                    name: "/co/".to_string(),
                    players: BTreeMap::from([(
                        5,
                        PatchPlayer {
                            name: Some("Snuffy".to_string()),
                            shirt_name: Some("SNUFFY".to_string()),
                            boots_id: Some(643),
                            gloves_id: Some(0),
                            base_copy_id: None,
                            appearance: Some(appearance),
                        },
                    )]),
                },
                PatchTeam {
                    id: 702,
                    name: "/a/".to_string(),
                    players: BTreeMap::from([(
                        3,
                        PatchPlayer {
                            name: None,
                            shirt_name: None,
                            boots_id: None,
                            gloves_id: None,
                            base_copy_id: Some(101334),
                            appearance: None,
                        },
                    )]),
                },
            ],
        };
        let text = patch.to_toml().expect("emit");
        let look_at = text.find("[teams.players.05.appearance]").expect("a look");
        let end = text[look_at..]
            .find("\n[[teams]]")
            .map(|i| look_at + i)
            .unwrap_or(text.len());
        assert!(
            !text[look_at..end].lines().any(|line| line.contains('#')),
            "{version:?}: no comments in a patch's [appearance]"
        );
        assert_eq!(
            AestheticsPatch::parse(&text).expect("round trip"),
            patch,
            "{version:?}"
        );
    }
}

#[test]
fn refusals() {
    // (patch text, the error it must produce); `doc` supplies the valid
    // header keys where a case does not mutate them.
    let cases: Vec<(String, PatchError)> = vec![
        (
            doc("").replace("pes_version = 21", "pes_version = 22"),
            SettingsError::WrongType {
                key: "pes_version".to_string(),
                expected: "a PES version, 15 to 21",
            }
            .into(),
        ),
        (
            doc("").replace("pes_version = 21", "pes_version = \"21\""),
            SettingsError::WrongType {
                key: "pes_version".to_string(),
                expected: "a PES version, 15 to 21",
            }
            .into(),
        ),
        (
            doc("").replace("tool_version = \"0.1.0\"\n", ""),
            PatchError::MissingKey {
                key: "tool_version".to_string(),
            },
        ),
        (
            doc("foo = 1\n"),
            SettingsError::UnknownKey {
                key: "foo".to_string(),
            }
            .into(),
        ),
        (
            doc("[[teams]]\nname = \"/a/\"\n"),
            PatchError::MissingKey {
                key: "teams[0].id".to_string(),
            },
        ),
        (
            doc("[[teams]]\nid = 714\nname = \"/co/\"\ncolor = 1\n"),
            SettingsError::UnknownKey {
                key: "teams[0].color".to_string(),
            }
            .into(),
        ),
        (
            doc("[[teams]]\nid = 702\nname = \"/a/\"\n\n[teams.players.5]\n"),
            SettingsError::UnknownKey {
                key: "teams[0].players.5".to_string(),
            }
            .into(),
        ),
        (
            doc("[[teams]]\nid = 702\nname = \"/a/\"\n\n[teams.players.00]\n"),
            SettingsError::UnknownKey {
                key: "teams[0].players.00".to_string(),
            }
            .into(),
        ),
        // The model IDs take the field's whole stored range: a string is
        // `WrongType`, a value past it `OutOfRange` — both at parse, the
        // patch being the compiler's output, not an author's band.
        (
            doc("[[teams]]\nid = 714\nname = \"/co/\"\n\n[teams.players.05]\nboots_id = \"x\"\n"),
            SettingsError::WrongType {
                key: "teams[0].players.05.boots_id".to_string(),
                expected: "an integer",
            }
            .into(),
        ),
        (
            doc("[[teams]]\nid = 714\nname = \"/co/\"\n\n[teams.players.05]\nboots_id = 16384\n"),
            SettingsError::OutOfRange {
                key: "teams[0].players.05.boots_id".to_string(),
                value: 16384,
                range: "0 to 16383".to_string(),
            }
            .into(),
        ),
        (
            doc("[[teams]]\nid = 714\nname = \"/co/\"\n\n[teams.players.05]\ngloves_id = -1\n"),
            SettingsError::OutOfRange {
                key: "teams[0].players.05.gloves_id".to_string(),
                value: -1,
                range: "0 to 16383".to_string(),
            }
            .into(),
        ),
        (
            doc("[[teams]]\nid = 714\nname = \"/co/\"\n\n[teams.players.05]\nbase_copy_id = -1\n"),
            SettingsError::OutOfRange {
                key: "teams[0].players.05.base_copy_id".to_string(),
                value: -1,
                range: "0 to 4294967295".to_string(),
            }
            .into(),
        ),
        // An `[appearance]` is a whole look or absent: one missing a key
        // the version holds is `MissingKey` at the absent key's path, not
        // a partial look applied at defaults.
        (
            doc(
                "[[teams]]\nid = 714\nname = \"/co/\"\n\n[teams.players.05.appearance.face]\ncheek_type = 1\n",
            ),
            PatchError::MissingKey {
                key: "teams[0].players.05.appearance.skin_color".to_string(),
            },
        ),
        (
            doc(
                "[[teams]]\nid = 714\nname = \"/co/\"\n\n[teams.players.05.appearance.motion]\nfree_kick = 21\n",
            ),
            SettingsError::OutOfRange {
                key: "teams[0].players.05.appearance.motion.free_kick".to_string(),
                value: 21,
                range: "1 to 20".to_string(),
            }
            .into(),
        ),
        (
            doc("").replace("compiled = 2026-09-12T14:03:00Z", "compiled = \"x\""),
            SettingsError::WrongType {
                key: "compiled".to_string(),
                expected: "a datetime",
            }
            .into(),
        ),
        // A motion table takes the motion keys alone: a stranger, and a
        // key of another table, are both `UnknownKey`.
        (
            doc(
                "[[teams]]\nid = 714\nname = \"/co/\"\n\n[teams.players.05.appearance.motion]\nfoo = 1\n",
            ),
            SettingsError::UnknownKey {
                key: "teams[0].players.05.appearance.motion.foo".to_string(),
            }
            .into(),
        ),
        (
            doc(
                "[[teams]]\nid = 714\nname = \"/co/\"\n\n[teams.players.05.appearance.motion]\nskin_color = 1\n",
            ),
            SettingsError::UnknownKey {
                key: "teams[0].players.05.appearance.motion.skin_color".to_string(),
            }
            .into(),
        ),
    ];
    for (text, expected) in cases {
        let error = AestheticsPatch::parse(&text).expect_err(&text);
        assert_eq!(error.to_string(), expected.to_string(), "{text}");
    }
}

/// A patch missing one key deep in the schema is refused at the absent
/// key's dotted path, not applied at its default: `golden()`'s text minus
/// its `iris_color` line is `MissingKey` at `appearance.iris_color`.
#[test]
fn a_look_missing_one_key_of_its_version_is_refused() {
    let text = golden().to_toml().expect("the golden emits");
    let gone = text.replace("iris_color = 0\n", "");
    assert_ne!(gone, text, "premise: the line was there");
    let err = AestheticsPatch::parse(&gone).expect_err("a partial look is refused");
    match &err {
        PatchError::MissingKey { key } => {
            assert_eq!(key, "teams[0].players.05.appearance.iris_color")
        }
        other => panic!("{other:?}"),
    }
}

/// A key the patch's version does not store is `NotInThisVersion`, not a
/// stranger's `UnknownKey`: `dribbling` is PES 20 and 21's, so a PES 18
/// patch holding it is refused naming it.
#[test]
fn a_key_the_patchs_version_lacks_is_not_in_this_version() {
    let text = "pes_version = 18\ntool_version = \"0.1.0\"\nallocation_scheme_version = 1\ncpks = []\ncompiled = 2026-09-12T14:03:00Z\n\n[[teams]]\nid = 714\nname = \"/co/\"\n\n[teams.players.05.appearance.motion]\ndribbling = 2\n";
    let err = AestheticsPatch::parse(text).expect_err("a foreign key is refused");
    match &err {
        PatchError::Settings(SettingsError::NotInThisVersion { key }) => {
            assert_eq!(key, "teams[0].players.05.appearance.motion.dribbling")
        }
        other => panic!("{other:?}"),
    }
}

/// A pre-Fox patch parses on the same rules: PES 17's look holds the
/// pre-Fox layout's keys, a key of PES 15's own layout on PES 21
/// `NotInThisVersion`.
#[test]
fn a_pre_fox_patch_parses_and_refuses_a_key_of_another_era() {
    let appearance = AppearanceSettings::default().resolved(PesVersion::Pes17);
    let text = patch_at(
        PesVersion::Pes17,
        vec![team(714, BTreeMap::from([(5, look(appearance.clone()))]))],
    )
    .to_toml()
    .expect("a pre-Fox patch emits");
    assert!(
        text.contains("[teams.players.05.appearance]"),
        "the look is in the file"
    );
    let patch = AestheticsPatch::parse(&text).expect("a pre-Fox patch parses");
    assert_eq!(
        patch.teams[0].players[&5].appearance.as_ref(),
        Some(&appearance),
        "the look round-trips"
    );
    let mut appearance = AppearanceSettings::default().resolved(PesVersion::Pes21);
    appearance.face.pes15_nose_angle = Some(2);
    let patch = patch_at(
        PesVersion::Pes21,
        vec![team(714, BTreeMap::from([(5, look(appearance))]))],
    );
    let err = patch.to_toml().expect_err("a pes15-only key on PES 21");
    match &err {
        PatchError::Settings(SettingsError::NotInThisVersion { key }) => {
            assert_eq!(key, "teams[0].players.05.appearance.face.pes15_nose_angle")
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn text_that_is_not_toml_is_toml_error() {
    let error = AestheticsPatch::parse("this is not toml").expect_err("not TOML");
    assert!(matches!(error, PatchError::Toml(_)), "{error:?}");
}

/// The `compiled` stamp's form: UTC, whole seconds, offset `Z`.
#[test]
fn utc_datetime_stamps_whole_seconds_utc() {
    for (unix, expected) in [
        (0u64, "1970-01-01T00:00:00Z"),
        (1709164800, "2024-02-29T00:00:00Z"),
        (4102444799, "2099-12-31T23:59:59Z"),
    ] {
        assert_eq!(utc_datetime(unix).to_string(), expected, "{unix}");
    }
}

/// Every day from 1970 past 2400 against a day-by-day calendar walk, so the century and
/// 400-year leap corrections are reached (2000 and 2400 leap, 2100 and 2200 not), and a time
/// whose hour, minute and second all differ.
#[test]
fn utc_datetime_matches_a_day_by_day_calendar() {
    let (mut year, mut month, mut day) = (1970u16, 1u8, 1u8);
    for days in 0..160_000u64 {
        let date = utc_datetime(days * 86_400).date.expect("a date");
        assert_eq!(
            (date.year, date.month, date.day),
            (year, month, day),
            "day {days}"
        );
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let length = match month {
            2 if leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        day += 1;
        if day > length {
            day = 1;
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
    }
    assert_eq!(utc_datetime(45_296).to_string(), "1970-01-01T12:34:56Z");
}
