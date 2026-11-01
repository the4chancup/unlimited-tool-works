//! `apply-patch`: the Team compiler's aesthetics patch applied to a save.

use std::collections::BTreeMap;
use std::fs;

use pes_savefile::EditFile;
use pes_savefile::interchange::aesthetics_patch::{
    ALLOCATION_SCHEME_VERSION, AestheticsPatch, PatchPlayer, PatchTeam,
};
use pes_savefile::model::player::{PlayerEntry, STRIPPED_APPEARANCE_ID};
use pes_savefile::schema::ingame_face::IngameFaceField;
use pes_savefile::schema::schema_for;
use pes_savefile::settings_toml::{AppearanceSettings, PlayerSettings, SettingKey};
use pes_version::PesVersion;

use crate::{Sandbox, WritingCommand, payload, payload_after};

/// Team `/co/`, 714 in the PES 21 fixture, whose slot 3 is player 71403.
const CO: u32 = 714;
const CO_SLOT_3: u32 = 71403;

/// A patch text for `version` under `scheme`, holding `teams`.
fn patch_text(version: PesVersion, scheme: u32, teams: Vec<PatchTeam>) -> String {
    AestheticsPatch {
        pes_version: version,
        tool_version: "0.1.0".to_owned(),
        allocation_scheme_version: scheme,
        cpks: vec![],
        compiled: "2026-10-13T00:00:00Z".parse().unwrap(),
        teams,
    }
    .to_toml()
    .unwrap()
}

/// A `[teams]]` block.
fn team(id: u32, name: &str, players: BTreeMap<u8, PatchPlayer>) -> PatchTeam {
    PatchTeam {
        id,
        name: name.to_owned(),
        players,
    }
}

/// A patch entry holding nothing: every key `None`.
fn bare() -> PatchPlayer {
    PatchPlayer {
        name: None,
        shirt_name: None,
        boots_id: None,
        gloves_id: None,
        base_copy_id: None,
        appearance: None,
    }
}

/// `player`'s own appearance resolved to `version`, `changes` (stored
/// values, as the patch's table holds them) on top: a whole `[appearance]`
/// whose diff against the save is `changes` alone.
fn look_of(
    player: &PlayerEntry,
    version: PesVersion,
    changes: &[(SettingKey, u8)],
) -> AppearanceSettings {
    let mut settings = PlayerSettings::from_player(player).unwrap();
    for (key, value) in changes {
        settings.set(*key, *value).unwrap();
    }
    settings.appearance.resolved(version)
}

/// The template look of `version` — every key at its `KeySpec` default —
/// `changes` (stored values) on top.
fn template(version: PesVersion, changes: &[(SettingKey, u8)]) -> AppearanceSettings {
    let mut settings = PlayerSettings::default();
    for (key, value) in changes {
        settings.set(*key, *value).unwrap();
    }
    settings.appearance.resolved(version)
}

/// Team `/co/`'s block: slot 03 renamed, his own look with two motions
/// changed (hunching dribbling and corner kick, shown 2 and 3, stored one
/// under).
fn co_block(save: &EditFile, version: PesVersion) -> PatchTeam {
    let player = save.player(CO_SLOT_3).unwrap();
    team(
        CO,
        "/co/",
        BTreeMap::from([(
            3,
            PatchPlayer {
                name: Some("PATCHED".to_owned()),
                shirt_name: Some("PATCHED".to_owned()),
                appearance: Some(look_of(
                    player,
                    version,
                    &[
                        (SettingKey::HunchingDribbling, 1),
                        (SettingKey::CornerKick, 2),
                    ],
                )),
                ..bare()
            },
        )]),
    )
}

/// The payload byte ranges holding `id`'s records: the player record on
/// every version, and PES 15/16's separate appearance record (joined by
/// the player id at its head).
fn record_spans(file: &EditFile, payload: &[u8], id: u32) -> Vec<std::ops::Range<usize>> {
    let schema = schema_for(file.version());
    let index = file
        .players()
        .iter()
        .position(|player| player.id == id)
        .expect("the player");
    let mut spans = Vec::new();
    spans.push(
        schema.players.offset + index * schema.player.size
            ..schema.players.offset + (index + 1) * schema.player.size,
    );
    if let Some((section, appearance)) = &schema.appearance {
        let count = u16::from_le_bytes(
            payload[section.count_offset..section.count_offset + 2]
                .try_into()
                .unwrap(),
        );
        for i in 0..usize::from(count) {
            let at = section.offset + i * appearance.size;
            if u32::from_le_bytes(payload[at..at + 4].try_into().unwrap()) == id {
                spans.push(at..at + appearance.size);
            }
        }
    }
    spans
}

/// Every byte `after` differs from `before` in sits inside `spans`.
fn assert_only_spans_differ(before: &[u8], after: &[u8], spans: &[std::ops::Range<usize>]) {
    assert_eq!(before.len(), after.len());
    for i in 0..before.len() {
        if before[i] != after[i] {
            assert!(
                spans.iter().any(|span| span.contains(&i)),
                "byte {i:#x} changed outside the player's records"
            );
        }
    }
}

impl Sandbox {
    /// `apply-patch <file> --base <base> --out <out>`.
    fn apply(&self, file: &str, base: &str, out: &str) -> crate::common::Run {
        self.run(&[
            "apply-patch",
            &self.arg(file),
            "--base",
            &self.arg(base),
            "--out",
            &self.arg(out),
        ])
    }
}

/// The payload `base` has after `patch` is applied through the lib.
fn payload_with_patch(sandbox: &Sandbox, base: &str, patch: &str) -> Vec<u8> {
    let patch = AestheticsPatch::parse(patch).unwrap();
    payload_after(&sandbox.path(base), |file| {
        patch.apply(file).unwrap();
    })
}

// SE-PAT-01
#[test]
fn a_patch_writes_its_teams_motions_and_names_and_nothing_else() {
    let sandbox = Sandbox::new("patch_one_team");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    let before = EditFile::load(&sandbox.path("EDIT")).unwrap();
    assert_ne!(before.player(CO_SLOT_3).unwrap().name, "PATCHED");
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&before, PesVersion::Pes21)],
    );
    fs::write(sandbox.path("patch.toml"), &text).unwrap();

    let run = sandbox.apply("patch.toml", "EDIT", "OUT");
    assert_eq!(run.exit_code(), 0);
    assert_eq!(run.messages(), vec!["Info patch_team_applied (team=714)"]);
    let expected = payload_with_patch(&sandbox, "EDIT", &text);
    assert_ne!(
        expected,
        payload(&sandbox.path("EDIT")),
        "the patch changes the save"
    );
    assert_eq!(payload(&sandbox.path("OUT")), expected);
    let written = EditFile::load(&sandbox.path("OUT")).unwrap();
    let player = written.player(CO_SLOT_3).unwrap();
    assert_eq!(player.name, "PATCHED");
    assert_eq!(player.shirt_name, "PATCHED");
    assert_ne!(
        player.motion.hunching_dribbling,
        before.player(CO_SLOT_3).unwrap().motion.hunching_dribbling
    );
}

// SE-PAT-02
#[test]
fn a_team_the_save_lacks_is_an_error_and_the_rest_is_written() {
    let sandbox = Sandbox::new("patch_missing_team");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    let save = EditFile::load(&sandbox.path("EDIT")).unwrap();
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![
            co_block(&save, PesVersion::Pes21),
            team(
                999,
                "/zz/",
                BTreeMap::from([(
                    1,
                    PatchPlayer {
                        name: Some("NOBODY".to_owned()),
                        ..bare()
                    },
                )]),
            ),
        ],
    );
    fs::write(sandbox.path("patch.toml"), &text).unwrap();

    let run = sandbox.apply("patch.toml", "EDIT", "OUT");
    assert_eq!(run.exit_code(), 1);
    assert_eq!(
        run.messages(),
        vec![
            "Info patch_team_applied (team=714)",
            "Error patch_team_not_in_save (team=999)"
        ]
    );
    assert_eq!(
        payload(&sandbox.path("OUT")),
        payload_with_patch(&sandbox, "EDIT", &text)
    );
    let written = EditFile::load(&sandbox.path("OUT")).unwrap();
    assert_eq!(written.player(CO_SLOT_3).unwrap().name, "PATCHED");
}

// SE-PAT-03
#[test]
fn a_version_or_scheme_mismatch_is_refused_with_nothing_written() {
    let sandbox = Sandbox::new("patch_mismatch");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    let save = EditFile::load(&sandbox.path("EDIT")).unwrap();
    fs::write(
        sandbox.path("version.toml"),
        patch_text(
            PesVersion::Pes19,
            ALLOCATION_SCHEME_VERSION,
            vec![co_block(&save, PesVersion::Pes19)],
        ),
    )
    .unwrap();
    sandbox
        .apply("version.toml", "EDIT", "OUT")
        .assert_refused(2, &["19", "21"]);
    fs::write(
        sandbox.path("scheme.toml"),
        patch_text(
            PesVersion::Pes21,
            ALLOCATION_SCHEME_VERSION + 1,
            vec![co_block(&save, PesVersion::Pes21)],
        ),
    )
    .unwrap();
    sandbox.apply("scheme.toml", "EDIT", "OUT").assert_refused(
        2,
        &[
            "allocation scheme",
            &(ALLOCATION_SCHEME_VERSION + 1).to_string(),
        ],
    );
    assert!(!sandbox.path("OUT").exists());
}

// SE-PAT-05
#[test]
fn a_team_whose_every_patch_player_the_save_lacks_is_not_listed_applied() {
    let sandbox = Sandbox::new("patch_no_players");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    let save = EditFile::load(&sandbox.path("EDIT")).unwrap();
    // Two slots of /co/ whose player ids the save lacks (a cup player id is the team's
    // id x 100 + slot).
    let empty: Vec<u8> = (1..=99u8)
        .filter(|slot| save.player(CO * 100 + u32::from(*slot)).is_none())
        .collect();
    let [first, second, ..] = empty[..] else {
        panic!("the fixture's /co/ lacks two slots' players: {empty:?}");
    };
    let players: BTreeMap<u8, PatchPlayer> = [first, second]
        .iter()
        .map(|slot| {
            (
                *slot,
                PatchPlayer {
                    name: Some("NOBODY".to_owned()),
                    ..bare()
                },
            )
        })
        .collect();
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![team(CO, "/co/", players)],
    );
    fs::write(sandbox.path("patch.toml"), &text).unwrap();

    let run = sandbox.apply("patch.toml", "EDIT", "OUT");
    assert_eq!(run.exit_code(), 1);
    assert_eq!(
        run.messages(),
        [first, second]
            .iter()
            .map(|slot| {
                format!(
                    "Error patch_player_not_in_save (player={})",
                    CO * 100 + u32::from(*slot)
                )
            })
            .collect::<Vec<_>>(),
        "each player skipped, the team not listed as applied"
    );
    assert_eq!(
        payload(&sandbox.path("OUT")),
        payload(&sandbox.path("EDIT")),
        "no player was written"
    );
}

// SE-PAT-06
#[test]
fn an_out_that_cannot_be_written_prints_only_the_failure() {
    let sandbox = Sandbox::new("patch_bad_out");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    let save = EditFile::load(&sandbox.path("EDIT")).unwrap();
    fs::write(
        sandbox.path("patch.toml"),
        patch_text(
            PesVersion::Pes21,
            ALLOCATION_SCHEME_VERSION,
            vec![co_block(&save, PesVersion::Pes21)],
        ),
    )
    .unwrap();
    // An --out that is a folder cannot be written; assert_refused asserts no event went
    // out, so no patch_team_applied or skip line.
    fs::create_dir(sandbox.path("OUT_DIR")).unwrap();

    sandbox
        .apply("patch.toml", "EDIT", "OUT_DIR")
        .assert_refused(3, &["OUT_DIR"]);
}

/// `apply-patch` writing in place a patch that renames `/co/`'s slot 3.
const APPLY_PATCH: WritingCommand = WritingCommand {
    name: "patch_running",
    setup: |sandbox| {
        sandbox.fixture(PesVersion::Pes21, "EDIT");
        let save = EditFile::load(&sandbox.path("EDIT")).unwrap();
        let text = patch_text(
            PesVersion::Pes21,
            ALLOCATION_SCHEME_VERSION,
            vec![co_block(&save, PesVersion::Pes21)],
        );
        fs::write(sandbox.path("patch.toml"), text).unwrap();
    },
    args: |sandbox| {
        let edit = sandbox.arg("EDIT");
        vec![
            "apply-patch".to_owned(),
            sandbox.arg("patch.toml"),
            "--base".to_owned(),
            edit.clone(),
            "--out".to_owned(),
            edit,
        ]
    },
    saves: &["EDIT"],
    files: &[],
};

// SE-CLI-03
#[test]
fn a_patch_is_refused_while_the_saves_own_pes_runs() {
    APPLY_PATCH.assert_refused_while_its_game_runs();
}

// SE-CLI-03
#[test]
fn a_patch_is_written_while_its_pes_runs_given_edit_mode_closed() {
    APPLY_PATCH.assert_written_given_edit_mode_closed();
}

// SE-CLI-03
#[test]
fn a_patch_is_written_while_only_another_versions_pes_runs() {
    APPLY_PATCH.assert_written_while_another_version_runs();
}

/// `base` rewritten through `edit` at its path, as the scenarios' GIVENs
/// set their fields.
fn set_up_save(sandbox: &Sandbox, base: &str, edit: impl FnOnce(&mut EditFile)) {
    let mut file = EditFile::load(&sandbox.path(base)).unwrap();
    edit(&mut file);
    fs::write(
        sandbox.path(base),
        file.to_bytes(&crate::TEST_SALT).unwrap(),
    )
    .unwrap();
}

// SE-PAT-07
#[test]
fn a_look_lands_whole_and_makes_the_record_his() {
    let sandbox = Sandbox::new("patch_whole_look");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    // 71403 stripped (appearance id -1) and copying player 7511.
    set_up_save(&sandbox, "EDIT", |file| {
        let player = file.player_mut(CO_SLOT_3).unwrap();
        player.appearance_id = Some(STRIPPED_APPEARANCE_ID);
        player.appearance.base_copy_id = 7511;
    });
    // The patch's entry: skin colour 4 and a free kick his record does not
    // hold, every other key at its default, boots 643 and gloves 0.
    let free_kick = {
        let file = EditFile::load(&sandbox.path("EDIT")).unwrap();
        (file.player(CO_SLOT_3).unwrap().motion.free_kick + 1) % 20
    };
    let appearance = template(
        PesVersion::Pes21,
        &[
            (SettingKey::SkinColor, 4),
            (SettingKey::FreeKick, free_kick),
        ],
    );
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![team(
            CO,
            "/co/",
            BTreeMap::from([(
                3,
                PatchPlayer {
                    boots_id: Some(643),
                    gloves_id: Some(0),
                    appearance: Some(appearance.clone()),
                    ..bare()
                },
            )]),
        )],
    );
    fs::write(sandbox.path("patch.toml"), &text).unwrap();
    let before_payload = payload(&sandbox.path("EDIT"));

    let run = sandbox.apply("patch.toml", "EDIT", "OUT");
    assert_eq!(run.exit_code(), 0, "{:?}", run.messages());

    let after = EditFile::load(&sandbox.path("OUT")).unwrap();
    let player = after.player(CO_SLOT_3).unwrap();
    // His appearance block is the entry's look: read back, every key is
    // the patch's.
    assert_eq!(
        PlayerSettings::from_player(player).unwrap().appearance,
        appearance
    );
    assert_eq!(
        player.motion.free_kick, free_kick,
        "the entry's motion landed"
    );
    assert_eq!(player.appearance.boots_id, 643);
    assert_eq!(player.appearance.gloves_id, 0);
    assert_eq!(player.appearance_id, Some(CO_SLOT_3));
    assert_eq!(player.appearance.base_copy_id, CO_SLOT_3);
    // Every other byte of the payload is unchanged: the differences all
    // sit inside his record.
    let spans = record_spans(&after, &payload(&sandbox.path("OUT")), CO_SLOT_3);
    assert_only_spans_differ(&before_payload, &payload(&sandbox.path("OUT")), &spans);
}

// SE-PAT-08
#[test]
fn a_base_copy_id_links_him_and_the_flag_stands_or_clears() {
    let sandbox = Sandbox::new("patch_base_copy_link");
    sandbox.fixture(PesVersion::Pes18, "EDIT");
    // 71403 copying player 7511, the base-copy flag set.
    set_up_save(&sandbox, "EDIT", |file| {
        let player = file.player_mut(CO_SLOT_3).unwrap();
        player.appearance.base_copy_id = 7511;
        player.edit_flags.base_copy = Some(true);
    });
    let patch_at = |base_copy_id: Option<u32>| {
        patch_text(
            PesVersion::Pes18,
            ALLOCATION_SCHEME_VERSION,
            vec![team(
                CO,
                "/co/",
                BTreeMap::from([(
                    3,
                    PatchPlayer {
                        base_copy_id,
                        appearance: Some(template(PesVersion::Pes18, &[])),
                        ..bare()
                    },
                )]),
            )],
        )
    };
    // With the link: his base copy is the link's id, the flag still set.
    fs::write(sandbox.path("linked.toml"), patch_at(Some(101334))).unwrap();
    let run = sandbox.apply("linked.toml", "EDIT", "OUT_A");
    assert_eq!(run.exit_code(), 0, "{:?}", run.messages());
    let after = EditFile::load(&sandbox.path("OUT_A")).unwrap();
    let player = after.player(CO_SLOT_3).unwrap();
    assert_eq!(player.appearance.base_copy_id, 101334);
    assert_eq!(
        player.edit_flags.base_copy,
        Some(true),
        "the flag left as the save held it"
    );

    // And apart, the same entry with no base_copy_id: his own id, the
    // flag clear.
    fs::write(sandbox.path("cleared.toml"), patch_at(None)).unwrap();
    let run = sandbox.apply("cleared.toml", "EDIT", "OUT_B");
    assert_eq!(run.exit_code(), 0, "{:?}", run.messages());
    let after = EditFile::load(&sandbox.path("OUT_B")).unwrap();
    let player = after.player(CO_SLOT_3).unwrap();
    assert_eq!(player.appearance.base_copy_id, CO_SLOT_3);
    assert_eq!(player.edit_flags.base_copy, Some(false));
}

// SE-PAT-09
#[test]
fn a_boots_only_patch_writes_the_id_and_nothing_else() {
    let sandbox = Sandbox::new("patch_boots_only");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    // 71403 copying 7511, with a look no template default matches.
    set_up_save(&sandbox, "EDIT", |file| {
        let player = file.player_mut(CO_SLOT_3).unwrap();
        player.appearance.base_copy_id = 7511;
        player.appearance.neck_length = 3;
        player
            .appearance
            .ingame_face
            .set(IngameFaceField::SkinColor, 4)
            .unwrap();
        player
            .appearance
            .ingame_face
            .set(IngameFaceField::HairStyle, 5)
            .unwrap();
    });
    let before = EditFile::load(&sandbox.path("EDIT")).unwrap();
    assert_ne!(
        before.player(CO_SLOT_3).unwrap().appearance.neck_length,
        7,
        "premise: the look is not the template's"
    );
    // The entry: boots_id 643, no [appearance].
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![team(
            CO,
            "/co/",
            BTreeMap::from([(
                3,
                PatchPlayer {
                    boots_id: Some(643),
                    ..bare()
                },
            )]),
        )],
    );
    fs::write(sandbox.path("patch.toml"), &text).unwrap();

    let run = sandbox.apply("patch.toml", "EDIT", "OUT");
    assert_eq!(run.exit_code(), 0, "{:?}", run.messages());

    let after = EditFile::load(&sandbox.path("OUT")).unwrap();
    let player = after.player(CO_SLOT_3).unwrap();
    assert_eq!(player.appearance.boots_id, 643);
    // The rest of his record — base copy and appearance id included — is
    // as it was.
    let mut expected = before.player(CO_SLOT_3).unwrap().clone();
    expected.appearance.boots_id = 643;
    assert_eq!(*player, expected);
}

// SE-PAT-10
#[test]
fn the_look_and_boots_land_on_pre_fox_and_the_flag_clears() {
    for version in [PesVersion::Pes16, PesVersion::Pes17] {
        let sandbox = Sandbox::new(&format!("patch_prefox_{}", version.number()));
        sandbox.fixture(version, "EDIT");
        // 71403 copying 7511, the base-copy flag set; on PES 17 also
        // stripped (appearance id -1).
        set_up_save(&sandbox, "EDIT", |file| {
            let player = file.player_mut(CO_SLOT_3).unwrap();
            player.appearance.base_copy_id = 7511;
            player.edit_flags.base_copy = Some(true);
            if player.appearance_id.is_some() {
                player.appearance_id = Some(STRIPPED_APPEARANCE_ID);
            }
        });
        // The patch's entry for him: skin colour 4 and boots 625.
        let appearance = template(version, &[(SettingKey::SkinColor, 4)]);
        let text = patch_text(
            version,
            ALLOCATION_SCHEME_VERSION,
            vec![team(
                CO,
                "/co/",
                BTreeMap::from([(
                    3,
                    PatchPlayer {
                        boots_id: Some(625),
                        appearance: Some(appearance.clone()),
                        ..bare()
                    },
                )]),
            )],
        );
        fs::write(sandbox.path("patch.toml"), &text).unwrap();

        let run = sandbox.apply("patch.toml", "EDIT", "OUT");
        assert_eq!(run.exit_code(), 0, "{version:?}: {:?}", run.messages());

        let after = EditFile::load(&sandbox.path("OUT")).unwrap();
        let player = after.player(CO_SLOT_3).unwrap();
        assert_eq!(
            PlayerSettings::from_player(player).unwrap().appearance,
            appearance,
            "{version:?}: the entry's look"
        );
        assert_eq!(player.appearance.boots_id, 625, "{version:?}");
        assert_eq!(player.appearance.base_copy_id, CO_SLOT_3, "{version:?}");
        assert_eq!(
            player.edit_flags.base_copy,
            Some(false),
            "{version:?}: the flag clear"
        );
        if version == PesVersion::Pes17 {
            assert_eq!(player.appearance_id, Some(CO_SLOT_3));
        }
    }
}

// SE-PAT-11
#[test]
fn a_value_past_the_fields_range_refuses_the_patch_naming_it() {
    let sandbox = Sandbox::new("patch_out_of_range");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    // SE-PAT-07's patch, mutated in place of one line at a time.
    let text = |from: &str, to: &str| {
        patch_text(
            PesVersion::Pes21,
            ALLOCATION_SCHEME_VERSION,
            vec![team(
                CO,
                "/co/",
                BTreeMap::from([(
                    3,
                    PatchPlayer {
                        boots_id: Some(643),
                        gloves_id: Some(0),
                        appearance: Some(template(
                            PesVersion::Pes21,
                            &[(SettingKey::SkinColor, 4)],
                        )),
                        ..bare()
                    },
                )]),
            )],
        )
        .replace(from, to)
    };
    // boots_id past the field's 14 bits.
    fs::write(
        sandbox.path("bad_boots.toml"),
        text("boots_id = 643", "boots_id = 16384"),
    )
    .unwrap();
    sandbox
        .apply("bad_boots.toml", "EDIT", "OUT")
        .assert_refused(2, &["teams[0].players.03.boots_id", "16384"]);
    assert!(!sandbox.path("OUT").exists());
    // skin_color outside its range.
    fs::write(
        sandbox.path("bad_skin.toml"),
        text("skin_color = 4", "skin_color = 9"),
    )
    .unwrap();
    sandbox
        .apply("bad_skin.toml", "EDIT", "OUT")
        .assert_refused(2, &["teams[0].players.03.appearance.skin_color", "outside"]);
    assert!(!sandbox.path("OUT").exists());
    // skin_color removed: a key the version holds, missing.
    fs::write(
        sandbox.path("missing_skin.toml"),
        text("skin_color = 4\n", ""),
    )
    .unwrap();
    sandbox
        .apply("missing_skin.toml", "EDIT", "OUT")
        .assert_refused(
            2,
            &["teams[0].players.03.appearance.skin_color", "required"],
        );
    assert!(!sandbox.path("OUT").exists());
}

// SE-PAT-11
#[test]
fn a_key_of_another_version_refuses_the_patch_naming_it() {
    let sandbox = Sandbox::new("patch_foreign_key");
    sandbox.fixture(PesVersion::Pes19, "EDIT");
    // A PES 19 patch holding `dribbling`, a PES 20 and 21 key.
    let text = patch_text(
        PesVersion::Pes19,
        ALLOCATION_SCHEME_VERSION,
        vec![team(
            CO,
            "/co/",
            BTreeMap::from([(
                3,
                PatchPlayer {
                    appearance: Some(template(PesVersion::Pes19, &[])),
                    ..bare()
                },
            )]),
        )],
    )
    .replace(
        "goal_celebration_2 = 0\n",
        "goal_celebration_2 = 0\ndribbling = 2\n",
    );
    assert!(
        text.contains("dribbling = 2"),
        "premise: the key is in the file"
    );
    fs::write(sandbox.path("foreign.toml"), &text).unwrap();
    sandbox.apply("foreign.toml", "EDIT", "OUT").assert_refused(
        2,
        &[
            "teams[0].players.03.appearance.motion.dribbling",
            "no such setting",
        ],
    );
    assert!(!sandbox.path("OUT").exists());
}
