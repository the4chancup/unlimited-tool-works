use super::*;

/// `player`'s own appearance for the patch, every key `version` holds set,
/// `changes` applied on top: a whole `[appearance]` whose diff against his
/// record is `changes` alone. `changes` are stored values, as the patch's
/// table holds them.
fn look_of(
    player: &PlayerEntry,
    version: PesVersion,
    changes: &[(SettingKey, u8)],
) -> AppearanceSettings {
    let mut settings = PlayerSettings::from_player(player).expect("the fixture's settings");
    for (key, value) in changes {
        settings.set(*key, *value).expect("inside the key's range");
    }
    settings.appearance.resolved(version)
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

/// `before` with the patch's names, `Some` IDs, base copy and
/// `[appearance]` written — what `apply` must produce of `before`: the
/// look through `PlayerSettings::apply`, then the record made his (his own
/// appearance id and base copy where the version has them, the flag
/// cleared) unless `base_copy_id` links him elsewhere.
fn patched(before: &PlayerEntry, patch: &PatchPlayer, version: PesVersion) -> PlayerEntry {
    let mut out = before.clone();
    if let Some(name) = &patch.name {
        out.name = name.clone();
        out.edit_flags.player = true;
    }
    if let Some(shirt_name) = &patch.shirt_name {
        out.shirt_name = shirt_name.clone();
        out.edit_flags.player = true;
    }
    if let Some(appearance) = &patch.appearance {
        PlayerSettings {
            appearance: appearance.clone(),
            ..PlayerSettings::default()
        }
        .apply(&mut out, version)
        .expect("the fixture's own values apply");
        if out.appearance_id.is_some() {
            out.appearance_id = Some(out.id);
        }
        if patch.base_copy_id.is_none() {
            out.appearance.base_copy_id = out.id;
            if out.edit_flags.base_copy.is_some() {
                out.edit_flags.base_copy = Some(false);
            }
        }
    }
    if let Some(id) = patch.base_copy_id {
        out.appearance.base_copy_id = id;
    }
    if let Some(id) = patch.boots_id {
        out.appearance.boots_id = u32::from(id);
    }
    if let Some(id) = patch.gloves_id {
        out.appearance.gloves_id = u32::from(id);
    }
    out
}

/// The patch's look, names and IDs apply, nothing else writes, and the
/// bytes persist through a save write.
#[test]
fn apply_writes_the_look_names_and_ids_and_nothing_else() {
    let (mut save, _) = open(PesVersion::Pes21);
    let slots = [5u8, 6, 7];
    let patched_ids: Vec<u32> = slots.iter().map(|s| 70100 + u32::from(*s)).collect();
    let before: Vec<PlayerEntry> = save.players().to_vec();
    let teams = save.teams().to_vec();
    let mut players = BTreeMap::new();
    for &id in &patched_ids {
        let player = save.player(id).expect("the fixture has the player");
        let slot = u8::try_from(id % 100).expect("an NN");
        let mut appearance = PlayerSettings::from_player(player)
            .expect("the fixture's settings")
            .appearance;
        appearance.motion = shifted(&player.motion);
        let patch_player = PatchPlayer {
            name: Some(format!("PATCH{slot:02}")),
            shirt_name: Some(format!("SHIRT{slot:02}")),
            boots_id: Some(600 + u16::from(slot)),
            gloves_id: Some(0),
            base_copy_id: None,
            appearance: Some(appearance),
        };
        // The patch's values differ from the save's, or the write
        // proves nothing.
        assert_ne!(
            patched(player, &patch_player, PesVersion::Pes21),
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
            &patched(before, patch_player, PesVersion::Pes21),
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
            &patched(before, patch_player, PesVersion::Pes21),
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

/// The presentation keys land in the save as the other appearance keys
/// do: a patch carrying `star` and `playing_attitude` writes both on PES
/// 21.
#[test]
fn a_patchs_presentation_keys_land_in_the_save() {
    let (mut save, _) = open(PesVersion::Pes21);
    let id = 70105;
    let player = save.player(id).expect("the fixture has the player");
    assert_ne!(player.stats.star, Some(7), "premise");
    assert_ne!(player.stats.playing_attitude, Some(3), "premise");
    let patch = patch_at(
        PesVersion::Pes21,
        vec![team(
            701,
            BTreeMap::from([(
                5,
                look(look_of(
                    player,
                    PesVersion::Pes21,
                    &[(SettingKey::Star, 7), (SettingKey::PlayingAttitude, 3)],
                )),
            )]),
        )],
    );
    patch.apply(&mut save).expect("applies");
    let player = save.player(id).expect("the player");
    assert_eq!(player.stats.star, Some(7));
    assert_eq!(player.stats.playing_attitude, Some(3));
}

/// A team or player the save lacks is skipped and reported, the rest
/// applied.
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
                appearance: Some(look_of(
                    save.player(70105).expect("present"),
                    PesVersion::Pes21,
                    &[(SettingKey::FreeKick, 3)],
                )),
                ..bare()
            },
        ),
        (
            24,
            PatchPlayer {
                name: Some("GHOST".to_string()),
                shirt_name: Some("GHOST".to_string()),
                ..bare()
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
        &patched(
            before,
            patch.teams[0].players.get(&5).expect("player 05"),
            PesVersion::Pes21
        )
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

/// A version or allocation-scheme mismatch is refused naming both values,
/// before any write.
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
            PesVersion::Pes15,
            ALLOCATION_SCHEME_VERSION,
            PatchError::VersionMismatch {
                patch: PesVersion::Pes15,
                save: PesVersion::Pes21,
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

/// A `base_copy_id` in the patch links the player to that id and leaves
/// the flag as the save holds it; without one, an `[appearance]` clears
/// the copy to his own id and the flag where the version has one.
#[test]
fn a_base_copy_id_links_him_while_a_look_clears_the_copy() {
    let (mut save, _) = open(PesVersion::Pes18);
    let (linked, cleared) = (70105u32, 70106u32);
    for id in [linked, cleared] {
        let player = save.player_mut(id).expect("present");
        player.appearance.base_copy_id = 7511;
        player.edit_flags.base_copy = Some(true);
    }
    let before: Vec<PlayerEntry> = [linked, cleared]
        .iter()
        .map(|id| save.player(*id).expect("present").clone())
        .collect();
    let patch = patch_at(
        PesVersion::Pes18,
        vec![team(
            701,
            BTreeMap::from([
                (
                    5,
                    PatchPlayer {
                        base_copy_id: Some(101334),
                        appearance: Some(look_of(&before[0], PesVersion::Pes18, &[])),
                        ..bare()
                    },
                ),
                (
                    6,
                    PatchPlayer {
                        appearance: Some(look_of(&before[1], PesVersion::Pes18, &[])),
                        ..bare()
                    },
                ),
            ]),
        )],
    );
    patch.apply(&mut save).expect("applies");
    let linked_after = save.player(linked).expect("the player");
    assert_eq!(
        linked_after.appearance.base_copy_id, 101334,
        "the link's id"
    );
    assert_eq!(
        linked_after.edit_flags.base_copy,
        Some(true),
        "the flag left as the save held it"
    );
    let cleared_after = save.player(cleared).expect("the player");
    assert_eq!(
        cleared_after.appearance.base_copy_id, cleared,
        "no link: his own id"
    );
    assert_eq!(
        cleared_after.edit_flags.base_copy,
        Some(false),
        "no link: the flag cleared"
    );
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

    // `dribbling` on a PES 18 save: the version has no such field. A
    // hand-built patch can carry one, where the text form refuses it.
    let (mut save, _) = open(PesVersion::Pes18);
    let before = save.players().to_vec();
    let teams = save.teams().to_vec();
    let patch = patch_at(
        PesVersion::Pes18,
        vec![team(
            701,
            BTreeMap::from([(
                5,
                look(AppearanceSettings {
                    motion: MotionSettings {
                        dribbling: Some(2),
                        ..MotionSettings::default()
                    },
                    ..AppearanceSettings::default()
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

    // `corner_kick` stored 8 (shown 9): inside the key's widest 1-to-10
    // range, past PES 18's own top (shown 1 to 6), refused at the key's
    // per-version range before the field's width is reached, reporting the
    // value as the file writes it.
    let (mut save, _) = open(PesVersion::Pes18);
    let before = save.players().to_vec();
    let teams = save.teams().to_vec();
    let patch = patch_at(
        PesVersion::Pes18,
        vec![team(
            701,
            BTreeMap::from([(
                5,
                look(AppearanceSettings {
                    motion: MotionSettings {
                        corner_kick: Some(8),
                        ..MotionSettings::default()
                    },
                    ..AppearanceSettings::default()
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
            assert_eq!(*value, 9);
            assert_eq!(range, "1 to 6");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(save.players(), &before[..]);
    assert_eq!(save.teams(), &teams[..]);
}

/// A motion past the patch's own version's top is refused at the key's
/// per-version range, not written: PES 18's celebration top is 127
/// stored, its hunching shown 1 to 3 (stored 0 to 2), so a patch's
/// `hunching_dribbling = 4` (stored 3) reports the 4 it wrote.
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
            4,
            "1 to 3",
        ),
    ] {
        let patch = patch_at(
            PesVersion::Pes18,
            vec![team(
                701,
                BTreeMap::from([(
                    5,
                    look(AppearanceSettings {
                        motion,
                        ..AppearanceSettings::default()
                    }),
                )]),
            )],
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
                look(AppearanceSettings {
                    motion: MotionSettings {
                        free_kick: Some(stored),
                        ..MotionSettings::default()
                    },
                    ..AppearanceSettings::default()
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

/// A written name, and a written shirt name, each set the edited-player
/// flag of a PES 2019 player whose flag is clear: without it the game
/// shows PLACEHOLDER for the new name (in game, 2026-11-08). A look alone
/// leaves the flag clear.
#[test]
fn a_written_name_sets_the_edited_player_flag() {
    let (mut save, _) = open(PesVersion::Pes19);
    let clear: Vec<u32> = save
        .players()
        .iter()
        .filter(|player| player.id / 100 == 711 && !player.edit_flags.player)
        .map(|player| player.id)
        .take(3)
        .collect();
    let [name_id, shirt_id, look_id] = clear[..] else {
        panic!("premise: three of team 711's players have the flag clear, got {clear:?}")
    };
    let slot = |id: u32| u8::try_from(id % 100).expect("an NN");
    let appearance = look_of(
        save.player(look_id).expect("present"),
        PesVersion::Pes19,
        &[],
    );
    let patch = patch_at(
        PesVersion::Pes19,
        vec![team(
            711,
            BTreeMap::from([
                (slot(name_id), named("NAMED")),
                (
                    slot(shirt_id),
                    PatchPlayer {
                        shirt_name: Some("SHIRT".to_string()),
                        ..bare()
                    },
                ),
                (slot(look_id), look(appearance)),
            ]),
        )],
    );
    patch.apply(&mut save).expect("applies");
    let flag = |id: u32| save.player(id).expect("the player").edit_flags.player;
    assert!(flag(name_id), "the name's player {name_id}");
    assert!(flag(shirt_id), "the shirt name's player {shirt_id}");
    assert!(!flag(look_id), "the look's player {look_id}");
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
                    shirt_name: Some("本田".to_string()),
                    ..bare()
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

/// A patch carrying an `[appearance]` alone leaves both names as they
/// were: `None` is "not written", never an empty string.
#[test]
fn an_appearance_only_patch_leaves_the_names() {
    let (mut save, _) = open(PesVersion::Pes21);
    let before = save.player(70105).expect("present").clone();
    assert!(
        !before.name.is_empty() && !before.shirt_name.is_empty(),
        "the test needs nonempty names"
    );
    let mut appearance = look_of(&before, PesVersion::Pes21, &[]);
    appearance.motion = shifted(&before.motion);
    let patch = patch_at(
        PesVersion::Pes21,
        vec![team(701, BTreeMap::from([(5, look(appearance))]))],
    );
    patch.apply(&mut save).expect("applies");
    let after = save.player(70105).expect("the player");
    // The look applied too, or the name assertions prove nothing.
    assert_eq!(
        after,
        &patched(
            &before,
            patch.teams[0].players.get(&5).expect("player 05"),
            PesVersion::Pes21
        ),
        "the look applied and the entry is otherwise his own"
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
    let mut appearance = look_of(save.player(70106).expect("present"), PesVersion::Pes21, &[]);
    appearance.motion.free_kick =
        Some((appearance.motion.free_kick.expect("held on PES 21") + 1) % 20);
    let patch = patch_at(
        PesVersion::Pes21,
        vec![team(701, BTreeMap::from([(6, look(appearance.clone()))]))],
    );
    patch.apply(&mut save).expect("applies");
    let moved = save.player(70106).expect("the player");
    let other = save.player(70110).expect("the player");
    assert_eq!(
        moved.motion.free_kick,
        appearance.motion.free_kick.expect("set"),
        "the patch lands by id, wherever he sits"
    );
    assert_eq!(other, &before, "the swapped-in player is untouched");
}

/// The lossless guard over the patch's `[appearance]` write: every
/// fixture player's own settings, applied back to him through the patch's
/// `[appearance]`, leave his record as it was but for the ids `apply`
/// makes his own — his appearance id where the version has one and his
/// base copy — and a player already carrying those ids keeps his record's
/// bytes whole, on every version's fixture save. A real save can hold a
/// value past the version's editor range (a celebration, a face type):
/// a patch cannot carry it, so its apply is refused and counted, the
/// byte-level premise standing on the players the patch does carry.
#[test]
fn a_players_own_settings_applied_by_the_patch_change_only_his_ids() {
    for version in FIXTURES {
        let (mut save, _) = open(version);
        let schema = schema_for(version);
        let before: Vec<PlayerEntry> = save.players().to_vec();
        let before_payload = crate::container::SaveContainer::decrypt(
            &save.to_bytes(&test_salt()).expect("the save writes"),
        )
        .expect("decrypts")
        .payload;
        // One patch a player, so a look a patch cannot carry — a stored
        // value past the version's editor range — is a per-player refusal,
        // not the whole save's.
        let mut applied: Vec<bool> = Vec::new();
        let mut unexpressible = 0usize;
        for before in &before {
            // `from_player` is the settings' construction; where it
            // refuses (a stored value the key table cannot express) the
            // stored-ranges reader carries it verbatim, as the
            // appearance-row guard does. Either way the value goes into a
            // patch entry of his own.
            let appearance = match PlayerSettings::from_player(before) {
                Ok(settings) => settings.appearance,
                Err(SettingsError::OutOfRange { .. }) => {
                    crate::settings_toml::appearance_from(before).expect("stored-ranges read")
                }
                Err(err) => panic!("{version:?} player {}: {err:?}", before.id),
            };
            let patch = patch_at(
                version,
                vec![PatchTeam {
                    id: before.id / 100,
                    name: String::new(),
                    players: BTreeMap::from([(
                        u8::try_from(before.id % 100).expect("a slot fits u8"),
                        look(appearance),
                    )]),
                }],
            );
            match patch.apply(&mut save) {
                Ok(summary) => applied.push(summary.applied_players.contains(&before.id)),
                Err(PatchError::Settings(
                    SettingsError::OutOfRange { .. } | SettingsError::NotInThisVersion { .. },
                )) => {
                    unexpressible += 1;
                    applied.push(false);
                }
                Err(err) => panic!("{version:?} player {}: {err:?}", before.id),
            }
        }
        let after_payload = crate::container::SaveContainer::decrypt(
            &save.to_bytes(&test_salt()).expect("the save writes"),
        )
        .expect("decrypts")
        .payload;
        assert_eq!(before_payload.len(), after_payload.len());
        let mut byte_identical = 0usize;
        for (index, (before, applied)) in before.iter().zip(&applied).enumerate() {
            let after = save.player(before.id).expect("still held");
            let mut expected = before.clone();
            if *applied {
                // The games' values for the run's unused bits, as
                // `PlayerSettings::apply` writes them.
                expected.appearance.ingame_face.write_unused();
                if expected.appearance_id.is_some() {
                    expected.appearance_id = Some(expected.id);
                }
                expected.appearance.base_copy_id = expected.id;
                if expected.edit_flags.base_copy.is_some() {
                    expected.edit_flags.base_copy = Some(false);
                }
            }
            assert_eq!(after, &expected, "{version:?} player {}", before.id);
            if expected != *before || !applied {
                continue;
            }
            // An applied player his own ids already named: every byte of
            // his record(s) is the fixture's, the premise the guard
            // stands on.
            byte_identical += 1;
            assert_eq!(
                record(&after_payload, &schema.players, schema.player.size, index),
                record(&before_payload, &schema.players, schema.player.size, index),
                "{version:?} player {}'s record",
                before.id
            );
            if let Some((section, appearance)) = &schema.appearance {
                let at = (0..count(&after_payload, section))
                    .map(|i| section.offset + i * appearance.size)
                    .find(|at| {
                        u32::from_le_bytes(after_payload[*at..at + 4].try_into().expect("u32"))
                            == before.id
                    })
                    .expect("his appearance record");
                assert_eq!(
                    &after_payload[at..at + appearance.size],
                    &before_payload[at..at + appearance.size],
                    "{version:?} player {}'s appearance record",
                    before.id
                );
            }
        }
        assert!(
            byte_identical > 0,
            "{version:?}: no player carried his own ids ({unexpressible} \
             unexpressible in the patch), the guard proves little"
        );
    }
}
