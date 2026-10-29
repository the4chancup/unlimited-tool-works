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

/// The sample patch of `spec/pes_savefile/operations.md` "Aesthetics patch
/// (new)", the motion lines as `settings.toml`'s emitter writes them
/// (`padded` puts `#` at column 33).
const GOLDEN: &str = "\
# aesthetics_patch.toml, written by the Team compiler to its output folder
pes_version = 21
tool_version = \"0.1.0\"
allocation_scheme_version = 1
cpks = [\"4cc_69.cpk\"]
compiled = 2026-09-12T14:03:00Z

[[teams]]
id = 714
name = \"/co/\"                   # for the reader; the id is what is applied

[teams.players.05]
name = \"Snuffy\"
shirt_name = \"SNUFFY\"

[teams.players.05.appearance.motion]
hunching_dribbling = 2          # 1 to 3 (PES 20 and 21: 1 to 5)
free_kick = 4                   # 1 to 16 (PES 20 and 21: 1 to 20)

[teams.players.06]

[teams.players.06.appearance.motion]
dribbling = 2                   # 0 to 3, PES 20 and 21 only
";

/// The patch `GOLDEN` reads and writes.
fn golden() -> AestheticsPatch {
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
                        motion: MotionSettings {
                            hunching_dribbling: Some(1),
                            free_kick: Some(3),
                            ..MotionSettings::default()
                        },
                    },
                ),
                (
                    6,
                    PatchPlayer {
                        name: None,
                        shirt_name: None,
                        motion: MotionSettings {
                            dribbling: Some(2),
                            ..MotionSettings::default()
                        },
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
        .motion
        .free_kick = Some(20);
    let err = patch.to_toml().expect_err("free_kick = 21 is out of range");
    assert!(
        matches!(err, PatchError::Settings(SettingsError::OutOfRange { .. })),
        "{err:?}"
    );
}

/// A hand-built patch can hold a pre-Fox `pes_version`; `to_toml`
/// refuses it rather than writing a file `parse` would reject.
#[test]
fn to_toml_refuses_a_pre_fox_version() {
    let patch = patch_at(PesVersion::Pes17, vec![team(701, BTreeMap::new())]);
    let err = patch.to_toml().expect_err("a pre-Fox patch is not written");
    assert!(
        matches!(err, PatchError::PreFox { version } if version == PesVersion::Pes17),
        "{err:?}"
    );
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

/// Every motion key `Some` at a non-default value, on PES 21 (with
/// `dribbling`) and PES 18 (without), two teams.
#[test]
fn every_motion_key_round_trips() {
    for (pes_version, dribbling) in [(PesVersion::Pes21, Some(2)), (PesVersion::Pes18, None)] {
        let motion = |goal_celebration_2| MotionSettings {
            hunching_dribbling: Some(2),
            hunching_running: Some(3),
            arm_movement_dribbling: Some(5),
            arm_movement_running: Some(6),
            corner_kick: Some(4),
            free_kick: Some(15),
            penalty_kick: Some(3),
            dribbling,
            goal_celebration_1: Some(40),
            goal_celebration_2: Some(goal_celebration_2),
        };
        let patch = AestheticsPatch {
            pes_version,
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
                            motion: motion(120),
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
                            motion: motion(150),
                        },
                    )]),
                },
            ],
        };
        assert_eq!(
            AestheticsPatch::parse(&patch.to_toml().expect("emit")).expect("round trip"),
            patch
        );
    }
}

#[test]
fn refusals() {
    // (patch text, the error it must produce); `doc` supplies the valid
    // header keys where a case does not mutate them.
    let cases: Vec<(String, PatchError)> = vec![
        (
            "pes_version = 17\ntool_version = \"0.1.0\"\nallocation_scheme_version = 1\ncpks = []\ncompiled = 2026-09-12T14:03:00Z\n".to_string(),
            PatchError::PreFox {
                version: PesVersion::Pes17,
            },
        ),
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
        (
            doc("[[teams]]\nid = 714\nname = \"/co/\"\n\n[teams.players.05]\nboots_id = 143\n"),
            SettingsError::UnknownKey {
                key: "teams[0].players.05.boots_id".to_string(),
            }
            .into(),
        ),
        (
            doc("[[teams]]\nid = 714\nname = \"/co/\"\n\n[teams.players.05.appearance.face]\ncheek_type = 1\n"),
            SettingsError::UnknownKey {
                key: "teams[0].players.05.appearance.face".to_string(),
            }
            .into(),
        ),
        (
            doc("[[teams]]\nid = 714\nname = \"/co/\"\n\n[teams.players.05.appearance.motion]\nfree_kick = 21\n"),
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
            doc("[[teams]]\nid = 714\nname = \"/co/\"\n\n[teams.players.05.appearance.motion]\nfoo = 1\n"),
            SettingsError::UnknownKey {
                key: "teams[0].players.05.appearance.motion.foo".to_string(),
            }
            .into(),
        ),
        (
            doc("[[teams]]\nid = 714\nname = \"/co/\"\n\n[teams.players.05.appearance.motion]\nskin_color = 1\n"),
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

// -------------------------------------------------------------------
// Apply

use crate::file::EditFile;
use crate::model::player::{PlayerMotion, STRIPPED_APPEARANCE_ID};
use crate::schema::fields::PlayerField;
use crate::test_support::{open, test_salt};

/// A patch at `version` over `teams`.
fn patch_at(version: PesVersion, teams: Vec<PatchTeam>) -> AestheticsPatch {
    AestheticsPatch {
        pes_version: version,
        tool_version: "0.1.0".to_string(),
        allocation_scheme_version: ALLOCATION_SCHEME_VERSION,
        cpks: vec!["4cc_69.cpk".to_string()],
        compiled: "2026-09-12T14:03:00Z".parse().expect("a datetime"),
        teams,
    }
}

/// A player with `name` alone.
fn named(name: &str) -> PatchPlayer {
    PatchPlayer {
        name: Some(name.to_string()),
        shirt_name: None,
        motion: MotionSettings::default(),
    }
}

/// A player with `motion` alone.
fn motion_only(motion: MotionSettings) -> PatchPlayer {
    PatchPlayer {
        name: None,
        shirt_name: None,
        motion,
    }
}

/// A `[[teams]]` block; `name` is a reader aid `apply` never uses.
fn team(id: u32, players: BTreeMap<u8, PatchPlayer>) -> PatchTeam {
    PatchTeam {
        id,
        name: String::new(),
        players,
    }
}

/// Every motion key `Some` at a value differing from `motion`'s —
/// `value + 1` inside the key's stored range, wrapping.
fn shifted(motion: &PlayerMotion) -> MotionSettings {
    let next = |value: u8, range: u8| (value + 1) % range;
    MotionSettings {
        hunching_dribbling: Some(next(motion.hunching_dribbling, 5)),
        hunching_running: Some(next(motion.hunching_running, 5)),
        arm_movement_dribbling: Some(next(motion.arm_movement_dribbling, 10)),
        arm_movement_running: Some(next(motion.arm_movement_running, 10)),
        corner_kick: Some(next(motion.corner_kick, 10)),
        free_kick: Some(next(motion.free_kick, 20)),
        penalty_kick: Some(next(motion.penalty_kick, 7)),
        dribbling: Some(next(motion.dribbling.expect("PES 21 has the field"), 4)),
        goal_celebration_1: Some(next(motion.goal_celebration_1, 163)),
        goal_celebration_2: Some(next(motion.goal_celebration_2, 163)),
    }
}

/// `before` with the patch's names and `Some` motion keys written —
/// what `apply` must produce of `before`.
fn patched(before: &PlayerEntry, patch: &PatchPlayer) -> PlayerEntry {
    let mut out = before.clone();
    if let Some(name) = &patch.name {
        out.name = name.clone();
    }
    if let Some(shirt_name) = &patch.shirt_name {
        out.shirt_name = shirt_name.clone();
    }
    let m = &patch.motion;
    if let Some(v) = m.hunching_dribbling {
        out.motion.hunching_dribbling = v;
    }
    if let Some(v) = m.hunching_running {
        out.motion.hunching_running = v;
    }
    if let Some(v) = m.arm_movement_dribbling {
        out.motion.arm_movement_dribbling = v;
    }
    if let Some(v) = m.arm_movement_running {
        out.motion.arm_movement_running = v;
    }
    if let Some(v) = m.corner_kick {
        out.motion.corner_kick = v;
    }
    if let Some(v) = m.free_kick {
        out.motion.free_kick = v;
    }
    if let Some(v) = m.penalty_kick {
        out.motion.penalty_kick = v;
    }
    if let Some(v) = m.dribbling {
        out.motion.dribbling = Some(v);
    }
    if let Some(v) = m.goal_celebration_1 {
        out.motion.goal_celebration_1 = v;
    }
    if let Some(v) = m.goal_celebration_2 {
        out.motion.goal_celebration_2 = v;
    }
    out
}

/// SE-PAT-01's lib half: the patch's motions and names apply, nothing else
/// writes, and the bytes persist through a save write.
#[test]
fn apply_writes_the_motions_and_names_and_nothing_else() {
    let (mut save, _) = open(PesVersion::Pes21);
    let slots = [5u8, 6, 7];
    let patched_ids: Vec<u32> = slots.iter().map(|s| 70100 + u32::from(*s)).collect();
    let before: Vec<PlayerEntry> = save.players().to_vec();
    let teams = save.teams().to_vec();
    let mut players = BTreeMap::new();
    for &id in &patched_ids {
        let player = save.player(id).expect("the fixture has the player");
        let slot = u8::try_from(id % 100).expect("an NN");
        let patch_player = PatchPlayer {
            name: Some(format!("PATCH{slot:02}")),
            shirt_name: Some(format!("SHIRT{slot:02}")),
            motion: shifted(&player.motion),
        };
        // The patch's values differ from the save's, or the write
        // proves nothing.
        assert_ne!(
            patched(player, &patch_player),
            *player,
            "player {id}: patch values must differ"
        );
        players.insert(slot, patch_player);
    }
    let patch = patch_at(PesVersion::Pes21, vec![team(701, players)]);
    let summary = patch.apply(&mut save).expect("applies");
    assert_eq!(summary.applied_teams, [701]);
    let written: Vec<u32> = patch.teams[0]
        .players
        .keys()
        .map(|slot| 70100 + u32::from(*slot))
        .collect();
    assert_eq!(summary.applied_players, written);
    assert!(summary.skipped.is_empty());
    for (slot, patch_player) in &patch.teams[0].players {
        let id = 70100 + u32::from(*slot);
        let before = before.iter().find(|p| p.id == id).expect("in before");
        assert_eq!(
            save.player(id).expect("the player"),
            &patched(before, patch_player),
            "player {id}"
        );
    }
    for (after, before) in save.players().iter().zip(&before) {
        if !patched_ids.contains(&after.id) {
            assert_eq!(after, before, "player {}", after.id);
        }
    }
    assert_eq!(save.teams(), &teams[..], "no team was written");
    // And it persists through the write: a reopened save holds the same.
    let back = EditFile::from_bytes(&save.to_bytes(&test_salt()).expect("the save writes"))
        .expect("reopens");
    for (slot, patch_player) in &patch.teams[0].players {
        let id = 70100 + u32::from(*slot);
        let before = before.iter().find(|p| p.id == id).expect("in before");
        assert_eq!(
            back.player(id).expect("the player"),
            &patched(before, patch_player),
            "player {id} persisted"
        );
    }
    for (after, before) in back.players().iter().zip(&before) {
        if !patched_ids.contains(&after.id) {
            assert_eq!(after, before, "player {}", after.id);
        }
    }
    assert_eq!(back.teams(), &teams[..]);
}

/// SE-PAT-02's lib half: a team or player the save lacks is skipped and
/// reported, the rest applied.
#[test]
fn an_absent_team_or_player_is_skipped_and_the_rest_applied() {
    let (mut save, _) = open(PesVersion::Pes21);
    let before: Vec<PlayerEntry> = save.players().to_vec();
    // Player 70124: team 701's first slot past its 23-man roster; the
    // save has no such player. Team 999 (the referee id) is a real cup
    // team id the fixture's all-701..=920 list does not hold.
    assert!(save.player(70124).is_none());
    assert!(save.team(999).is_none());
    let players = BTreeMap::from([
        (
            5,
            PatchPlayer {
                name: Some("PATCH05".to_string()),
                shirt_name: None,
                motion: shifted(&save.player(70105).expect("present").motion),
            },
        ),
        (
            24,
            PatchPlayer {
                name: Some("GHOST".to_string()),
                shirt_name: Some("GHOST".to_string()),
                motion: MotionSettings {
                    free_kick: Some(3),
                    ..MotionSettings::default()
                },
            },
        ),
    ]);
    let absent = PatchTeam {
        id: 999,
        name: "refs".to_string(),
        // An absent team's players are not looked at: this one produces
        // no `PlayerNotInSave`.
        players: BTreeMap::from([(5, named("REF"))]),
    };
    let patch = patch_at(PesVersion::Pes21, vec![team(701, players), absent]);
    let summary = patch.apply(&mut save).expect("applies");
    assert_eq!(summary.applied_teams, [701]);
    assert_eq!(summary.applied_players, [70105], "70124 is not in the save");
    assert_eq!(
        summary.skipped,
        [
            PatchSkip::PlayerNotInSave { player: 70124 },
            PatchSkip::TeamNotInSave { team: 999 },
        ]
    );
    let before = before.iter().find(|p| p.id == 70105).expect("in before");
    assert_eq!(
        save.player(70105).expect("the player"),
        &patched(before, patch.teams[0].players.get(&5).expect("player 05"))
    );
}

/// A hand-edited patch's `id` is any u32: past 42,949,672 `id * 100 + NN`
/// overflows u32, and unchecked arithmetic wraps onto a different, real
/// player's id — written with no error. The player is reported skipped
/// instead, and the save untouched.
#[test]
fn a_team_id_too_large_for_its_player_ids_skips_the_players() {
    let (mut save, _) = open(PesVersion::Pes21);
    // The fixture's teams run 701..=920; give one the overflowing id so
    // the team is found and the player loop is reached.
    save.teams_mut()[0].id = 42_949_673;
    let patch = patch_at(
        PesVersion::Pes21,
        vec![team(42_949_673, BTreeMap::from([(1, named("OVER"))]))],
    );
    let before = save.to_bytes(&test_salt()).expect("encodes");
    let summary = patch.apply(&mut save).expect("applies");
    assert_eq!(summary.applied_teams, [42_949_673]);
    assert!(summary.applied_players.is_empty());
    assert_eq!(
        summary.skipped,
        [PatchSkip::PlayerNotInSave { player: u32::MAX }]
    );
    assert_eq!(
        save.to_bytes(&test_salt()).expect("encodes"),
        before,
        "nothing was written"
    );
}

/// SE-PAT-03's lib half: a version or allocation-scheme mismatch is
/// refused naming both values, before any write.
#[test]
fn a_version_or_scheme_mismatch_refuses_before_any_write() {
    for (pes_version, scheme, expected) in [
        (
            PesVersion::Pes19,
            ALLOCATION_SCHEME_VERSION,
            PatchError::VersionMismatch {
                patch: PesVersion::Pes19,
                save: PesVersion::Pes21,
            },
        ),
        (
            PesVersion::Pes21,
            2,
            PatchError::SchemeMismatch {
                patch: 2,
                suite: ALLOCATION_SCHEME_VERSION,
            },
        ),
        (
            PesVersion::Pes17,
            ALLOCATION_SCHEME_VERSION,
            PatchError::PreFox {
                version: PesVersion::Pes17,
            },
        ),
    ] {
        let (mut save, _) = open(PesVersion::Pes21);
        let before = save.players().to_vec();
        let teams = save.teams().to_vec();
        // A valid player whose patch would change him: the refusal must
        // leave him as he was.
        let mut patch = patch_at(
            pes_version,
            vec![team(701, BTreeMap::from([(5, named("CHANGED"))]))],
        );
        patch.allocation_scheme_version = scheme;
        let err = patch.apply(&mut save).expect_err("refused");
        assert_eq!(
            err.to_string(),
            expected.to_string(),
            "{pes_version:?} {scheme}"
        );
        assert_eq!(save.players(), &before[..], "{pes_version:?}");
        assert_eq!(save.teams(), &teams[..], "{pes_version:?}");
    }
}

/// SE-PAT-04's lib half: a stripped player's names and motions apply, his
/// appearance id and block untouched, persisted through a save write.
#[test]
fn a_stripped_player_stays_stripped() {
    let (mut save, _) = open(PesVersion::Pes21);
    let slots = [5u8, 6];
    for &slot in &slots {
        save.player_mut(70100 + u32::from(slot))
            .expect("present")
            .appearance_id = Some(STRIPPED_APPEARANCE_ID);
    }
    let before: Vec<PlayerEntry> = slots
        .iter()
        .map(|slot| {
            save.player(70100 + u32::from(*slot))
                .expect("present")
                .clone()
        })
        .collect();
    let players: BTreeMap<u8, PatchPlayer> = slots
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            (
                *slot,
                PatchPlayer {
                    name: Some(format!("PATCH{slot:02}")),
                    shirt_name: Some(format!("SHIRT{slot:02}")),
                    motion: shifted(&before[i].motion),
                },
            )
        })
        .collect();
    let patch = patch_at(PesVersion::Pes21, vec![team(701, players)]);
    patch.apply(&mut save).expect("applies");
    for (before, (slot, patch_player)) in before.iter().zip(&patch.teams[0].players) {
        let id = 70100 + u32::from(*slot);
        let after = save.player(id).expect("the player");
        assert_eq!(
            after.appearance_id,
            Some(STRIPPED_APPEARANCE_ID),
            "{id} stays stripped"
        );
        assert_eq!(
            after.appearance, before.appearance,
            "{id}: the appearance block is untouched"
        );
        assert_eq!(after, &patched(before, patch_player), "{id}");
    }
    // And the stripped ids persist through the write.
    let back = EditFile::from_bytes(&save.to_bytes(&test_salt()).expect("the save writes"))
        .expect("reopens");
    for (before, (slot, patch_player)) in before.iter().zip(&patch.teams[0].players) {
        let id = 70100 + u32::from(*slot);
        let after = back.player(id).expect("the player");
        assert_eq!(
            after.appearance_id,
            Some(STRIPPED_APPEARANCE_ID),
            "{id} persisted stripped"
        );
        assert_eq!(after, &patched(before, patch_player), "{id} persisted");
    }
}

/// A failure anywhere in pass 1 leaves the whole save untouched: a
/// name past the version's field, a motion the version lacks, a stored
/// value past the field's width.
#[test]
fn a_broken_patch_is_refused_whole() {
    // A name longer than PES 21's 61-byte field on the second team's
    // player: the first team's player is not written either.
    let (mut save, _) = open(PesVersion::Pes21);
    let before = save.players().to_vec();
    let teams = save.teams().to_vec();
    let patch = patch_at(
        PesVersion::Pes21,
        vec![
            team(701, BTreeMap::from([(5, named("FINE"))])),
            team(702, BTreeMap::from([(5, named(&"N".repeat(100)))])),
        ],
    );
    let err = patch.apply(&mut save).expect_err("refused");
    match &err {
        PatchError::Settings(SettingsError::TextTooLong { key, .. }) => {
            assert_eq!(key, "teams[1].players.05.name")
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(save.players(), &before[..]);
    assert_eq!(save.teams(), &teams[..]);

    // `dribbling` on a PES 18 save: the version has no such field.
    let (mut save, _) = open(PesVersion::Pes18);
    let before = save.players().to_vec();
    let teams = save.teams().to_vec();
    let patch = patch_at(
        PesVersion::Pes18,
        vec![team(
            701,
            BTreeMap::from([(
                5,
                motion_only(MotionSettings {
                    dribbling: Some(2),
                    ..MotionSettings::default()
                }),
            )]),
        )],
    );
    let err = patch.apply(&mut save).expect_err("refused");
    match &err {
        PatchError::Settings(SettingsError::NotInThisVersion { key }) => {
            assert_eq!(key, "teams[0].players.05.appearance.motion.dribbling")
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(save.players(), &before[..]);
    assert_eq!(save.teams(), &teams[..]);

    // `corner_kick` stored 8: inside the key's widest 1-to-10 range, past
    // PES 18's own top (shown 1 to 6), refused at the key's per-version
    // range before the field's width is reached.
    let (mut save, _) = open(PesVersion::Pes18);
    let before = save.players().to_vec();
    let teams = save.teams().to_vec();
    let patch = patch_at(
        PesVersion::Pes18,
        vec![team(
            701,
            BTreeMap::from([(
                5,
                motion_only(MotionSettings {
                    corner_kick: Some(8),
                    ..MotionSettings::default()
                }),
            )]),
        )],
    );
    let err = patch.apply(&mut save).expect_err("refused");
    match &err {
        PatchError::Settings(SettingsError::OutOfRange {
            key, value, range, ..
        }) => {
            assert_eq!(key, "teams[0].players.05.appearance.motion.corner_kick");
            assert_eq!(*value, 8);
            assert_eq!(range, "1 to 6");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(save.players(), &before[..]);
    assert_eq!(save.teams(), &teams[..]);
}

/// A motion past the patch's own version's top is refused at the key's
/// per-version range, not written: PES 18's celebration top is 127
/// stored, its hunching shown 1 to 3 (stored 0 to 2).
#[test]
fn a_motion_past_the_patch_versions_top_is_refused() {
    let (mut save, _) = open(PesVersion::Pes18);
    let before = save.players().to_vec();
    let teams = save.teams().to_vec();
    for (motion, path, value, range) in [
        (
            MotionSettings {
                goal_celebration_1: Some(162),
                ..MotionSettings::default()
            },
            "teams[0].players.05.appearance.motion.goal_celebration_1",
            162i64,
            "0 to 127",
        ),
        (
            MotionSettings {
                hunching_dribbling: Some(3),
                ..MotionSettings::default()
            },
            "teams[0].players.05.appearance.motion.hunching_dribbling",
            3,
            "1 to 3",
        ),
    ] {
        let patch = patch_at(
            PesVersion::Pes18,
            vec![team(701, BTreeMap::from([(5, motion_only(motion))]))],
        );
        let err = patch.apply(&mut save).expect_err("refused");
        match &err {
            PatchError::Settings(SettingsError::OutOfRange {
                key,
                value: got,
                range: got_range,
                ..
            }) => {
                assert_eq!(key, path);
                assert_eq!(*got, value);
                assert_eq!(got_range, range);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(save.players(), &before[..], "{path}");
        assert_eq!(save.teams(), &teams[..], "{path}");
    }
}

/// The kind's range check the field's width does not see: `free_kick`
/// stored 20 is past the key's `1 to 20` (`OneBased`, stored < 20) but
/// inside PES 20/21's 5-bit `FreeKickMotion` field (holds 0 to 31).
/// Both halves of that premise are checked against the key table and
/// the schema, so the test cannot go green on a narrowed field.
#[test]
fn a_motion_past_its_keys_range_but_inside_the_field_is_refused() {
    let stored = 20;
    assert!(
        !SettingKey::FreeKick.spec().kind.accepts_stored(stored),
        "outside the kind's range"
    );
    let width = schema_for(PesVersion::Pes21)
        .player
        .fields
        .iter()
        .find(|spec| spec.field == PlayerField::FreeKickMotion)
        .expect("the schema has the field")
        .bit_width;
    assert!(u32::from(stored) < (1 << width), "inside the field's width");
    let (mut save, _) = open(PesVersion::Pes21);
    let before = save.players().to_vec();
    let teams = save.teams().to_vec();
    let patch = patch_at(
        PesVersion::Pes21,
        vec![team(
            701,
            BTreeMap::from([(
                5,
                motion_only(MotionSettings {
                    free_kick: Some(stored),
                    ..MotionSettings::default()
                }),
            )]),
        )],
    );
    let err = patch.apply(&mut save).expect_err("refused");
    match &err {
        PatchError::Settings(SettingsError::OutOfRange { key, value, .. }) => {
            assert_eq!(key, "teams[0].players.05.appearance.motion.free_kick");
            assert_eq!(*value, i64::from(stored));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(save.players(), &before[..]);
    assert_eq!(save.teams(), &teams[..]);
}

/// `name` is a UTF-8 field: a real name outside Latin-1 (`"本田"`, 6
/// bytes) applies and is written.
#[test]
fn a_name_past_latin1_applies() {
    let (mut save, _) = open(PesVersion::Pes21);
    let patch = patch_at(
        PesVersion::Pes21,
        vec![team(701, BTreeMap::from([(5, named("本田"))]))],
    );
    patch.apply(&mut save).expect("applies");
    assert_eq!(save.player(70105).expect("the player").name, "本田");
}

/// `shirt_name` is single-byte text (chars at most U+00FF): the same
/// text is `WrongType` at the patch path, the save untouched.
#[test]
fn a_shirt_name_past_latin1_is_refused() {
    let (mut save, _) = open(PesVersion::Pes21);
    let before = save.players().to_vec();
    let teams = save.teams().to_vec();
    let patch = patch_at(
        PesVersion::Pes21,
        vec![team(
            701,
            BTreeMap::from([(
                5,
                PatchPlayer {
                    name: None,
                    shirt_name: Some("本田".to_string()),
                    motion: MotionSettings::default(),
                },
            )]),
        )],
    );
    let err = patch.apply(&mut save).expect_err("refused");
    match &err {
        PatchError::Settings(SettingsError::WrongType { key, .. }) => {
            assert_eq!(key, "teams[0].players.05.shirt_name")
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(save.players(), &before[..]);
    assert_eq!(save.teams(), &teams[..]);
}

/// A patch carrying motions alone leaves both names as they were: `None`
/// is "not written", never an empty string.
#[test]
fn a_motion_only_patch_leaves_the_names() {
    let (mut save, _) = open(PesVersion::Pes21);
    let before = save.player(70105).expect("present").clone();
    assert!(
        !before.name.is_empty() && !before.shirt_name.is_empty(),
        "the test needs nonempty names"
    );
    let patch = patch_at(
        PesVersion::Pes21,
        vec![team(
            701,
            BTreeMap::from([(5, motion_only(shifted(&before.motion)))]),
        )],
    );
    patch.apply(&mut save).expect("applies");
    let after = save.player(70105).expect("the player");
    // The motion applied too, or the name assertions prove nothing.
    assert_eq!(
        after,
        &patched(&before, patch.teams[0].players.get(&5).expect("player 05")),
        "the motion applied and the entry is otherwise his own"
    );
    assert_eq!(after.name, before.name);
    assert_eq!(after.shirt_name, before.shirt_name);
}

/// `NN` names the player id `team × 100 + NN`, not the roster slot: after
/// the roster is reordered, the player with that id takes the patch
/// wherever he now sits.
#[test]
fn the_patch_lands_on_the_player_id_not_the_slot() {
    let (mut save, _) = open(PesVersion::Pes21);
    // Slots 06 and 10 swap: 70106 sits at slot 10 now, 70110 at slot 06.
    save.team_mut(701).expect("the team").roster.swap(5, 9);
    assert_eq!(
        save.team(701).expect("the team").roster[9].player_id,
        70106,
        "the patched player moved"
    );
    assert_eq!(
        save.team(701).expect("the team").roster[5].player_id,
        70110,
        "the swapped-in one stands in his old slot"
    );
    let before = save.player(70110).expect("present").clone();
    let patch = patch_at(
        PesVersion::Pes21,
        vec![team(
            701,
            BTreeMap::from([(
                6,
                motion_only(shifted(&save.player(70106).expect("present").motion)),
            )]),
        )],
    );
    patch.apply(&mut save).expect("applies");
    let moved = save.player(70106).expect("the player");
    let other = save.player(70110).expect("the player");
    assert_eq!(
        moved.motion.free_kick,
        patch.teams[0]
            .players
            .get(&6)
            .expect("player 06")
            .motion
            .free_kick
            .expect("set"),
        "the patch lands by id, wherever he sits"
    );
    assert_eq!(other, &before, "the swapped-in player is untouched");
}
