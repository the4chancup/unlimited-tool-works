use super::*;
use crate::file::EditFile;
use crate::interchange::team_toml::{TeamToml, TeamTomlError};
use crate::model::ingame_face::IngameFace;
use crate::schema::ingame_face::IngameFaceField;
use crate::test_support::{FIXTURES, open};

/// A foreign stand-in for each of `team`'s rostered slots: a real player
/// of the same save that is not on this roster, re-id'd to the slot.
fn foreign_players(file: &EditFile, team: &TeamEntry) -> Vec<PlayerEntry> {
    let rostered: Vec<u32> = team.roster.iter().map(|slot| slot.player_id).collect();
    let mut others = file
        .players()
        .iter()
        .filter(|player| !rostered.contains(&player.id));
    team.roster
        .iter()
        .filter(|slot| slot.player_id != 0)
        .map(|slot| {
            let mut player = others.next().expect("a spare player").clone();
            player.id = slot.player_id;
            player
        })
        .collect()
}

/// (a) Every fixture version, every team with players: the document
/// reparses to itself, and applying onto foreign stand-ins reproduces
/// every source `PlayerEntry` field for field, plus the roster numbers.
#[test]
fn every_fixture_player_round_trips_through_the_document() {
    for version in FIXTURES {
        let (file, _) = open(version);
        let refs: Vec<&PlayerEntry> = file.players().iter().collect();
        // The text pass (to_toml -> parse == document) costs tens of ms a
        // team, mostly toml_edit; the model pass (from_team -> apply onto
        // foreign targets -> field-for-field) is ~1 ms and stays
        // exhaustive. Text runs on a sample: the first rostered team, the
        // last, and the team whose players carry the most `Some` keys.
        let rostered: Vec<&TeamEntry> = file
            .teams()
            .iter()
            .filter(|team| team.roster.iter().any(|slot| slot.player_id != 0))
            .collect();
        let docs: Vec<TeamToml> = rostered
            .iter()
            .map(|team| TeamToml::from_team(version, team, &refs).expect("from_team"))
            .collect();
        let fullest = docs
            .iter()
            .enumerate()
            .max_by_key(|(_, doc)| doc.players.values().map(some_keys).sum::<usize>())
            .map(|(n, _)| n)
            .expect("a rostered team exists");
        for (n, (team, doc)) in rostered.iter().zip(&docs).enumerate() {
            if n == 0 || n + 1 == rostered.len() || n == fullest {
                let text = doc.to_toml().expect("to_toml");
                let parsed = TeamToml::parse(&text)
                    .unwrap_or_else(|e| panic!("{version:?} team {} reparse: {e}", team.id));
                assert_eq!(parsed, *doc, "{version:?} team {}", team.id);
            }

            let mut target_team = (*team).clone();
            let mut targets = foreign_players(&file, team);
            let stand_in_ids: Vec<(u32, Option<u32>)> = targets
                .iter()
                .map(|player| (player.id, player.appearance_id))
                .collect();
            let notes = doc
                .apply(version, &mut target_team, &mut targets)
                .unwrap_or_else(|e| panic!("{version:?} team {} apply: {e}", team.id));
            assert!(notes.is_empty(), "{version:?} team {}: {notes:?}", team.id);
            assert_eq!(
                target_team.roster, team.roster,
                "{version:?} team {} roster",
                team.id
            );
            for slot in &team.roster {
                if slot.player_id == 0 {
                    continue;
                }
                let source = *refs
                    .iter()
                    .find(|player| player.id == slot.player_id)
                    .expect("the rostered player");
                let target = targets
                    .iter()
                    .find(|player| player.id == slot.player_id)
                    .expect("the stand-in");
                // `appearance_id` is not the document's to set: the
                // stand-in's own stands, everything else is the source's.
                let stand_in_id = stand_in_ids
                    .iter()
                    .find(|(id, _)| *id == slot.player_id)
                    .map(|(_, appearance_id)| *appearance_id)
                    .expect("the stand-in");
                let mut expected = PlayerEntry {
                    appearance_id: stand_in_id,
                    ..source.clone()
                };
                // The run's unused bits are no key: the stand-in keeps its
                // own (absent = untouched), so both sides compare at the
                // games' values.
                let mut target = target.clone();
                target.appearance.ingame_face.write_unused();
                expected.appearance.ingame_face.write_unused();
                assert_eq!(
                    target, expected,
                    "{version:?} team {} player {}",
                    team.id, slot.player_id
                );
            }
        }
    }
}

/// `height`/`weight` are `[players.NN]` player-level keys, right after
/// `age` and before the ids: the emitted text shows them there, and a
/// team's round trip through the document keeps them.
#[test]
fn height_and_weight_are_player_level_keys() {
    let (file, _) = open(PesVersion::Pes19);
    let (team, slot) = first_slot(&file);
    let key = u8::try_from(slot + 1).expect("a slot index");
    let source = file
        .players()
        .iter()
        .find(|p| p.id == team.roster[slot].player_id)
        .expect("the rostered player");
    let refs: Vec<&PlayerEntry> = file.players().iter().collect();
    let doc = TeamToml::from_team(PesVersion::Pes19, team, &refs).expect("from_team");
    let section = &doc.players[&key];
    assert_eq!(section.height, Some(source.basic.height));
    assert_eq!(section.weight, Some(source.basic.weight));

    // The emitted block holds `height`/`weight` right after `age`.
    let text = doc.to_toml().expect("to_toml");
    let block = text
        .split_once(&format!("[players.{key:02}]"))
        .and_then(|(_, rest)| rest.split_once("\n["))
        .map(|(block, _)| block)
        .unwrap_or_else(|| panic!("no [players.{key:02}] block in:\n{text}"));
    let lines: Vec<&str> = block.lines().collect();
    let age = lines
        .iter()
        .position(|line| line.starts_with("age ="))
        .unwrap_or_else(|| panic!("an age line in {block}"));
    assert!(
        lines[age + 1].starts_with(&format!("height = {}", source.basic.height)),
        "height right after age: {block}"
    );
    assert!(
        lines[age + 1].contains("# cm, the record's own field"),
        "the height comment: {}",
        lines[age + 1]
    );
    assert!(
        lines[age + 2].starts_with(&format!("weight = {}", source.basic.weight)),
        "weight after height: {block}"
    );
    assert!(
        lines[age + 2].contains("# kg"),
        "the weight comment: {}",
        lines[age + 2]
    );

    // Emit, parse, apply onto a copy scrambled to zero: the round trip
    // restores every rostered player's height and weight.
    let parsed = TeamToml::parse(&text).expect("the emitted text reparses");
    assert_eq!(parsed, doc);
    let mut target_team = team.clone();
    let mut players = file.players().to_vec();
    for player in players.iter_mut() {
        player.basic.height = 0;
        player.basic.weight = 0;
    }
    let notes = parsed
        .apply(PesVersion::Pes19, &mut target_team, &mut players)
        .expect("applies");
    assert!(notes.is_empty(), "{notes:?}");
    for rostered in team.roster.iter().filter(|s| s.player_id != 0) {
        let src = file
            .players()
            .iter()
            .find(|p| p.id == rostered.player_id)
            .expect("rostered");
        let dst = players
            .iter()
            .find(|p| p.id == rostered.player_id)
            .expect("applied");
        assert_eq!(
            (dst.basic.height, dst.basic.weight),
            (src.basic.height, src.basic.weight),
            "player {}",
            src.id
        );
    }
}

/// (b) A one-key player document changes exactly that stat of exactly
/// that player.
#[test]
fn a_one_key_player_document_changes_exactly_that_stat() {
    let (file, _) = open(PesVersion::Pes19);
    let team = file
        .teams()
        .iter()
        .find(|team| team.roster.iter().any(|slot| slot.player_id != 0))
        .expect("a team with players");
    let slot = team
        .roster
        .iter()
        .position(|slot| slot.player_id != 0)
        .expect("a rostered slot");
    let doc = TeamToml::parse(&format!(
        "[players.{:02}.stats]\nfinishing = 88\n",
        slot + 1
    ))
    .expect("parses");
    let mut target_team = team.clone();
    let mut players = file.players().to_vec();
    // The expectation is built from a pre-apply snapshot: deriving it
    // from `players` after `apply` would let any amount of clobbering
    // pass, since expected and actual would share the corruption.
    let mut expected = players.clone();
    let id = team.roster[slot].player_id;
    expected
        .iter_mut()
        .find(|player| player.id == id)
        .expect("the player")
        .stats
        .finishing = 88;
    let notes = doc
        .apply(PesVersion::Pes19, &mut target_team, &mut players)
        .expect("applies");
    assert!(notes.is_empty());
    assert_eq!(target_team.roster, team.roster);
    assert_eq!(players, expected);
}

/// (c) A PES 21 document applied to PES 19: the 20/21-only keys are
/// `NotInThisVersion` notes, not errors, everything else applies, and a
/// face type over the 19 cap is `Capped`.
#[test]
fn a_pes21_document_on_a_pes19_target_notes_and_caps() {
    let (file21, _) = open(PesVersion::Pes21);
    let (file19, _) = open(PesVersion::Pes19);
    let team21 = file21
        .teams()
        .iter()
        .find(|team| team.roster.iter().any(|slot| slot.player_id != 0))
        .expect("a team with players");
    let refs: Vec<&PlayerEntry> = file21.players().iter().collect();
    let mut doc = TeamToml::from_team(PesVersion::Pes21, team21, &refs).expect("from_team");

    // Push the first player's facial hair type past the PES 19 cap — the
    // field's bits, an unmeasured column — so it resets to 0 there.
    let cap19 =
        face_type_cap(PesVersion::Pes19, IngameFaceField::FacialHairType).expect("a PES 19 cap");
    let over = cap19 + 1;
    doc.players
        .values_mut()
        .next()
        .expect("a player")
        .appearance
        .face
        .facial_hair_type = Some(over);

    let team19 = file19
        .teams()
        .iter()
        .find(|team| team.roster.iter().any(|slot| slot.player_id != 0))
        .expect("a team with players");
    let mut target_team = team19.clone();
    let mut targets: Vec<PlayerEntry> = team19
        .roster
        .iter()
        .filter(|slot| slot.player_id != 0)
        .filter_map(|slot| {
            file19
                .players()
                .iter()
                .find(|player| player.id == slot.player_id)
                .cloned()
        })
        .collect();
    let notes = doc
        .apply(PesVersion::Pes19, &mut target_team, &mut targets)
        .expect("foreign apply is notes, not errors");

    for suffix in [
        ".stats.tight_possession",
        ".stats.aggression",
        ".stats.playing_attitude",
        ".positions.stronger_hand",
        ".appearance.motion.dribbling",
    ] {
        assert!(
            notes.iter().any(|note| matches!(
                note,
                ImportNote::NotInThisVersion { path }
                    if path.starts_with("players.") && path.ends_with(suffix)
            )),
            "a NotInThisVersion note ending in {suffix}: {notes:?}"
        );
    }
    assert!(
        notes.iter().any(|note| matches!(
            note,
            ImportNote::Capped { path, from, to }
                if path.ends_with(".appearance.face.facial_hair_type")
                    && *from == over
                    && *to == 0
        )),
        "a Capped note for facial_hair_type, reset to 0: {notes:?}"
    );
    // A field that exists on both versions applied, on the roster slot
    // the section's key names (its first key is 01 = roster[0]).
    let (key, first) = doc.players.iter().next().expect("a player");
    let id = team19.roster[usize::from(*key - 1)].player_id;
    let target = targets
        .iter()
        .find(|player| player.id == id)
        .expect("the player the section lands on");
    let before_name = file19.player(id).expect("the target player").name.clone();
    assert_ne!(
        Some(&before_name),
        first.name.as_ref(),
        "his name differs, or the write is invisible"
    );
    assert_eq!(Some(&target.name), first.name.as_ref());
}

/// (d) A foreign document's hair key of the other layout is dropped with
/// a `NotInThisVersion` note, the run left as it was: PES 16's `style`
/// has no bits in PES 15's run.
#[test]
fn a_hair_key_of_the_other_layout_is_dropped_with_a_note() {
    let (file, _) = open(PesVersion::Pes15);
    let team = file
        .teams()
        .iter()
        .find(|team| team.roster.iter().any(|slot| slot.player_id != 0))
        .expect("a team with players");
    let slot = team
        .roster
        .iter()
        .position(|slot| slot.player_id != 0)
        .expect("a rostered slot");
    let doc = TeamToml::parse(&format!(
        "pes_version = 16\n\n[players.{:02}.appearance.hair]\nstyle = 4\n",
        slot + 1
    ))
    .expect("parses");
    let id = team.roster[slot].player_id;
    let before = file
        .player(id)
        .expect("the player")
        .appearance
        .ingame_face
        .clone();
    let mut target_team = team.clone();
    let mut players = file.players().to_vec();
    let notes = doc
        .apply(PesVersion::Pes15, &mut target_team, &mut players)
        .expect("a foreign key is a note, not an error");
    assert_eq!(
        notes,
        [ImportNote::NotInThisVersion {
            path: format!("players.{:02}.appearance.hair.style", slot + 1)
        }]
    );
    let target = players
        .iter()
        .find(|player| player.id == id)
        .expect("the player");
    assert_eq!(
        target.appearance.ingame_face, before,
        "the run is untouched"
    );
}

/// A document of the target's own version naming a key of the other
/// layout is an error, as `dribbling` is where the version lacks it: on
/// PES 15, `style` (PES 16 to 21's hair) is `NotInThisVersion`. The
/// document sets nothing else, so no later key's write could hide a
/// write of `style` into PES 15's bytes.
#[test]
fn a_same_version_key_of_the_other_layout_is_an_error() {
    let (file, _) = open(PesVersion::Pes15);
    let team = file
        .teams()
        .iter()
        .find(|team| team.roster.iter().any(|slot| slot.player_id != 0))
        .expect("a team with players");
    let slot = team
        .roster
        .iter()
        .position(|slot| slot.player_id != 0)
        .expect("a rostered slot");
    let doc = TeamToml::parse(&format!(
        "pes_version = 15\n\n[players.{:02}.appearance.hair]\nstyle = 4\n",
        slot + 1
    ))
    .expect("parses");
    let mut target_team = team.clone();
    let mut players = file.players().to_vec();
    let err = doc
        .apply(PesVersion::Pes15, &mut target_team, &mut players)
        .expect_err("a same-version key the version lacks");
    assert!(
        matches!(
            err,
            TeamTomlError::Settings(
                crate::settings_toml::SettingsError::NotInThisVersion { ref key }
            ) if key == "style"
        ),
        "{err:?}"
    );
}

/// An import writes the keys it holds and nothing else: the run's
/// measured-unused bits keep the target's own values (absent =
/// untouched), where a settings apply writes the games' values.
#[test]
fn an_import_leaves_the_runs_unused_bits_untouched() {
    let (file, _) = open(PesVersion::Pes16);
    let team = file
        .teams()
        .iter()
        .find(|team| team.roster.iter().any(|slot| slot.player_id != 0))
        .expect("a team with players");
    let slot = team
        .roster
        .iter()
        .position(|slot| slot.player_id != 0)
        .expect("a rostered slot");
    let id = team.roster[slot].player_id;
    let mut players = file.players().to_vec();
    let target_entry = players
        .iter_mut()
        .find(|player| player.id == id)
        .expect("the player");
    // Bytes 46 to 49 are unused on PES 16 to 21 and written 0 by the
    // games: 0xFF shows whether the import wrote them.
    let mut run = target_entry.appearance.ingame_face.bytes().to_vec();
    run[46..50].fill(0xFF);
    target_entry.appearance.ingame_face = IngameFace::from_bytes(run);
    let unused_before = target_entry.appearance.ingame_face.unused();

    let doc = TeamToml::parse(&format!(
        "pes_version = 16\n\n[players.{:02}.appearance.hair]\nstyle = 4\n",
        slot + 1
    ))
    .expect("parses");
    let mut target_team = team.clone();
    doc.apply(PesVersion::Pes16, &mut target_team, &mut players)
        .expect("applies");
    let face = &players
        .iter()
        .find(|player| player.id == id)
        .expect("the player")
        .appearance
        .ingame_face;
    assert_eq!(
        face.get(IngameFaceField::HairStyle).expect("reads"),
        4,
        "the key landed"
    );
    assert_eq!(
        face.unused(),
        unused_before,
        "the unused bits are untouched"
    );
    assert_eq!(face.bytes()[46..50], [0xFF; 4]);
}

/// (e) A slot with no player is an `EmptySlot` note; a slot past the
/// roster's width is `NoSuchSlot`.
#[test]
fn empty_and_missing_slots() {
    let (file, _) = open(PesVersion::Pes19);
    let team = file.teams().first().expect("a team");
    assert!(team.roster.len() >= 30, "the fixture roster has 30+ slots");
    let mut target_team = team.clone();
    target_team.roster[29].player_id = 0;
    let mut players = file.players().to_vec();
    let doc = TeamToml::parse("[players.30]\nname = \"GHOST\"\n").expect("parses");
    let notes = doc
        .apply(PesVersion::Pes19, &mut target_team, &mut players)
        .expect("an empty slot is a note");
    assert_eq!(notes, vec![ImportNote::EmptySlot { slot: 30 }]);

    let (file18, _) = open(PesVersion::Pes18);
    let team18 = file18.teams().first().expect("a team");
    assert_eq!(team18.roster.len(), 32, "PES 18 rosters are 32 wide");
    let doc = TeamToml::parse("[players.40]\nname = \"GHOST\"\n").expect("parses");
    let mut target18 = team18.clone();
    let mut players18 = file18.players().to_vec();
    let err = doc
        .apply(PesVersion::Pes18, &mut target18, &mut players18)
        .expect_err("slot 40 does not exist");
    assert!(matches!(err, TeamTomlError::NoSuchSlot { slot: 40 }));
}

/// The `players` pool contract: an empty slice is a tactics-only dump; a
/// non-empty pool missing a rostered id is `PlayerMissing`, never a
/// silent drop.
#[test]
fn the_player_pool_is_all_or_nothing() {
    let (file, _) = open(PesVersion::Pes19);
    let team = file
        .teams()
        .iter()
        .find(|team| team.roster.iter().any(|slot| slot.player_id != 0))
        .expect("a rostered team");
    let tactics_only = TeamToml::from_team(PesVersion::Pes19, team, &[])
        .expect("an empty pool is a tactics-only dump");
    assert!(tactics_only.players.is_empty());

    let missing_id = team
        .roster
        .iter()
        .find(|slot| slot.player_id != 0)
        .expect("a rostered slot")
        .player_id;
    let pool: Vec<&PlayerEntry> = file
        .players()
        .iter()
        .filter(|player| player.id != missing_id)
        .collect();
    let err = TeamToml::from_team(PesVersion::Pes19, team, &pool)
        .expect_err("a partial pool is a caller bug");
    assert!(
        matches!(err, TeamTomlError::PlayerMissing { id } if id == missing_id),
        "{err:?}"
    );
}

/// (f) `base_copy_id` equal to `[team] id` x 100 + slot writes the
/// target's own id; any other value — or no `[team] id` — applies as
/// written.
#[test]
fn the_base_copy_own_id_convention() {
    let (file, _) = open(PesVersion::Pes19);
    let team = file
        .teams()
        .iter()
        .find(|team| team.roster.iter().any(|slot| slot.player_id != 0))
        .expect("a team with players");
    let slot = team
        .roster
        .iter()
        .position(|slot| slot.player_id != 0)
        .expect("a rostered slot");
    let id = team.roster[slot].player_id;
    let path = format!("players.{:02}", slot + 1);
    let convention = 5 * 100 + u32::try_from(slot + 1).expect("a small slot");

    let doc = TeamToml::parse(&format!(
        "[team]\nid = 5\n\n[{path}]\nbase_copy_id = {convention}\n"
    ))
    .expect("parses");
    let mut target_team = team.clone();
    let mut players = file.players().to_vec();
    doc.apply(PesVersion::Pes19, &mut target_team, &mut players)
        .expect("applies");
    let target = players
        .iter()
        .find(|player| player.id == id)
        .expect("the player");
    assert_eq!(target.appearance.base_copy_id, id);

    for (text, expected) in [
        (
            format!(
                "[team]\nid = 5\n\n[{path}]\nbase_copy_id = {}\n",
                convention + 1
            ),
            convention + 1,
        ),
        (
            format!("[{path}]\nbase_copy_id = {convention}\n"),
            convention,
        ),
    ] {
        let doc = TeamToml::parse(&text).expect("parses");
        let mut target_team = team.clone();
        let mut players = file.players().to_vec();
        doc.apply(PesVersion::Pes19, &mut target_team, &mut players)
            .expect("applies");
        let target = players
            .iter()
            .find(|player| player.id == id)
            .expect("the player");
        assert_eq!(target.appearance.base_copy_id, expected, "{text:?}");
    }
}

/// (g) Player parse errors.
#[test]
fn malformed_player_keys_are_errors() {
    let err = TeamToml::parse("[players.7]\nname = \"X\"\n").expect_err("players.7");
    assert!(matches!(
        err,
        TeamTomlError::UnknownKey { ref key } if key == "players.7"
    ));
    let err = TeamToml::parse("[players.41]\nname = \"X\"\n").expect_err("players.41");
    assert!(matches!(
        err,
        TeamTomlError::UnknownKey { ref key } if key == "players.41"
    ));
    let err = TeamToml::parse("[players.01.skills]\nskills = [\"heading\", \"heading\"]\n")
        .expect_err("a duplicate skill");
    assert!(matches!(
        err,
        TeamTomlError::UnknownLabel {
            ref key,
            ref label,
            ..
        } if key == "players.01.skills.skills" && label == "heading"
    ));
    let err = TeamToml::parse("[players.01.positions]\nplayable = { CF = \"A\" }\n")
        .expect_err("a missing position");
    assert!(matches!(
        err,
        TeamTomlError::MissingKey { ref key }
            if key == "players.01.positions.playable.GK"
    ));
    // `ingame_face` left the format when its bits moved to
    // `[appearance.unknown] face`: the key is a stranger like any other.
    let err = TeamToml::parse("[players.01]\ningame_face = \"zz\"\n").expect_err("a retired key");
    assert!(matches!(
        err,
        TeamTomlError::UnknownKey { ref key } if key == "players.01.ingame_face"
    ));
    // `[appearance.unknown]` left it too, once every bit of the run was a
    // key or measured unused.
    let err = TeamToml::parse("[players.01.appearance.unknown]\nface = \"00\"\n")
        .expect_err("a retired table");
    assert!(
        matches!(
            err,
            TeamTomlError::Settings(crate::settings_toml::SettingsError::UnknownKey { ref key })
                if key == "players.01.appearance.unknown"
        ),
        "{err:?}"
    );
}

/// The stored-ranges deviation, kept deliberate: a stored value past every
/// version's editor cap rides the document unchanged (and `settings.toml`'s
/// strict `from_player` still refuses it). `goal_celebration_2` 200: its 8
/// bits hold it, the widest range of 162 does not (a face type's range is
/// its field's bits now, so the bump is a motion's).
#[test]
fn a_stored_value_above_the_widest_range_round_trips() {
    use crate::settings_toml::{PlayerSettings, SettingsError};

    let (file, _) = open(PesVersion::Pes15);
    let (team, slot) = first_slot(&file);
    let id = team.roster[slot].player_id;
    let mut players = file.players().to_vec();
    let player = players
        .iter_mut()
        .find(|p| p.id == id)
        .expect("the rostered player");
    player.motion.goal_celebration_2 = 200;
    let player = player.clone();

    // settings.toml's contract is intact: it refuses the same player.
    assert!(matches!(
        PlayerSettings::from_player(&player),
        Err(SettingsError::OutOfRange { .. })
    ));

    let refs: Vec<&PlayerEntry> = players.iter().collect();
    let doc = TeamToml::from_team(PesVersion::Pes15, team, &refs).expect("from_team");
    let text = doc.to_toml().expect("to_toml");
    let parsed = TeamToml::parse(&text).expect("stored ranges reparse");
    assert_eq!(parsed, doc);
    let mut target_team = team.clone();
    // The save's own players: the one built above is back to his
    // stored celebration there.
    let mut targets = file.players().to_vec();
    parsed
        .apply(PesVersion::Pes15, &mut target_team, &mut targets)
        .expect("applies");
    let target = targets.iter().find(|p| p.id == id).expect("rostered");
    assert_eq!(target, &player);
}

/// The first rostered team of `file` and its first rostered slot.
fn first_slot(file: &EditFile) -> (&TeamEntry, usize) {
    let team = file
        .teams()
        .iter()
        .find(|team| team.roster.iter().any(|slot| slot.player_id != 0))
        .expect("a team with players");
    let slot = team
        .roster
        .iter()
        .position(|slot| slot.player_id != 0)
        .expect("a rostered slot");
    (team, slot)
}

/// A `[players.NN]`-only document carrying `section` at `slot` (0-based).
fn one_section(from: PesVersion, slot: usize, section: PlayerSection) -> TeamToml {
    TeamToml {
        pes_version: Some(from),
        players: BTreeMap::from([(u8::try_from(slot + 1).expect("a slot index"), section)]),
        ..TeamToml::default()
    }
}

/// A `players` map key of 0 (only a built document carries one) is
/// `NoSuchSlot`, not an underflowing index — `slot - 1` on `u8` would
/// panic in a debug build.
#[test]
fn a_players_key_zero_is_no_such_slot() {
    let (file, _) = open(PesVersion::Pes18);
    let (team, _) = first_slot(&file);
    let document = TeamToml {
        pes_version: Some(PesVersion::Pes18),
        players: BTreeMap::from([(0, PlayerSection::default())]),
        ..TeamToml::default()
    };
    let mut target = team.clone();
    let mut players = file.players().to_vec();
    assert!(
        matches!(
            document
                .apply(PesVersion::Pes18, &mut target, &mut players)
                .expect_err("a key 0"),
            TeamTomlError::NoSuchSlot { slot: 0 }
        ),
        "a key 0"
    );
}

/// A skill the document's source version cannot store is not written at
/// all — the target's value stands, note-free (a pair-wide rule). The
/// same `false` under the target's own version clears it.
#[test]
fn a_skill_the_source_version_lacks_is_left_untouched() {
    let (file, _) = open(PesVersion::Pes19);
    let (team, slot) = first_slot(&file);
    let id = team.roster[slot].player_id;
    // Double Touch: stored on PES 19+, absent on 15/16 (28 skills there).
    const DOUBLE_TOUCH: usize = 28;
    assert!(!schema_for(PesVersion::Pes16).player_has(PlayerField::Skill(28)));
    assert!(schema_for(PesVersion::Pes19).player_has(PlayerField::Skill(28)));

    let mut section = PlayerSection::default();
    section.skills.skills = Some([false; 41]);
    let mut players = file.players().to_vec();
    players
        .iter_mut()
        .find(|player| player.id == id)
        .expect("the player")
        .skills
        .skills[DOUBLE_TOUCH] = true;

    let notes = one_section(PesVersion::Pes16, slot, section.clone())
        .apply(PesVersion::Pes19, &mut team.clone(), &mut players)
        .expect("applies");
    assert!(notes.is_empty(), "{notes:?}");
    assert!(
        players
            .iter()
            .find(|player| player.id == id)
            .expect("the player")
            .skills
            .skills[DOUBLE_TOUCH],
        "a skill PES 16 cannot store is left alone"
    );

    let notes = one_section(PesVersion::Pes19, slot, section)
        .apply(PesVersion::Pes19, &mut team.clone(), &mut players)
        .expect("applies");
    assert!(notes.is_empty(), "{notes:?}");
    assert!(
        !players
            .iter()
            .find(|player| player.id == id)
            .expect("the player")
            .skills
            .skills[DOUBLE_TOUCH],
        "the same false under the target's own version clears"
    );
}

/// A face type over the target's cap resets to 0 with a `Capped` note,
/// never a clamp; a value at the cap is not capped.
#[test]
fn a_face_type_over_the_cap_resets_and_at_the_cap_is_untouched() {
    // Facial hair's PES 19 cap is the field's bits, an unmeasured column:
    // the over-cap value is a document-side stored 32, the at-cap one 31.
    let (file, _) = open(PesVersion::Pes19);
    let cap =
        face_type_cap(PesVersion::Pes19, IngameFaceField::FacialHairType).expect("a PES 19 cap");
    let (team, slot) = first_slot(&file);
    let id = team.roster[slot].player_id;
    let face_value = |players: &[PlayerEntry]| {
        players
            .iter()
            .find(|player| player.id == id)
            .expect("the player")
            .appearance
            .ingame_face
            .get(IngameFaceField::FacialHairType)
            .expect("a stored type")
    };
    // The fixture carries 0 for him: set his stored type to 5 first,
    // or the reset and the write would be invisible.
    let set_face = |players: &mut [PlayerEntry]| {
        players
            .iter_mut()
            .find(|player| player.id == id)
            .expect("the player")
            .appearance
            .ingame_face
            .set(IngameFaceField::FacialHairType, 5)
            .expect("writes");
    };

    let mut section = PlayerSection::default();
    section.appearance.face.facial_hair_type = Some(cap + 1);
    let mut target = team.clone();
    let mut players = file.players().to_vec();
    set_face(&mut players);
    let notes = one_section(PesVersion::Pes21, slot, section)
        .apply(PesVersion::Pes19, &mut target, &mut players)
        .expect("applies");
    assert_eq!(
        notes,
        vec![ImportNote::Capped {
            path: format!("players.{:02}.appearance.face.facial_hair_type", slot + 1),
            from: cap + 1,
            to: 0,
        }]
    );
    assert_eq!(face_value(&players), 0);

    let mut section = PlayerSection::default();
    section.appearance.face.facial_hair_type = Some(cap);
    let mut target = team.clone();
    let mut players = file.players().to_vec();
    set_face(&mut players);
    let notes = one_section(PesVersion::Pes21, slot, section)
        .apply(PesVersion::Pes19, &mut target, &mut players)
        .expect("applies");
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(face_value(&players), cap);
}

/// Skin 7 resets to 1 with a `Capped` note where the target has no
/// custom skin (PES 18+), stands on PES 17, and an ordinary preset skin
/// never notes.
#[test]
fn skin_seven_resets_only_where_the_target_has_no_custom_skin() {
    let apply = |from: PesVersion, to: PesVersion, skin: u8| {
        let (file, _) = open(to);
        let (team, slot) = first_slot(&file);
        let id = team.roster[slot].player_id;
        let mut players = file.players().to_vec();
        // The fixture carries skin 1 for him: set his stored skin to 3
        // first, or the reset and the writes would be invisible.
        let player = players
            .iter_mut()
            .find(|player| player.id == id)
            .expect("the player");
        player
            .appearance
            .ingame_face
            .set(IngameFaceField::SkinColor, 3)
            .expect("writes");
        let before = player
            .appearance
            .ingame_face
            .get(IngameFaceField::SkinColor)
            .expect("a stored skin");
        let mut section = PlayerSection::default();
        section.appearance.skin_color = Some(skin);
        let mut target = team.clone();
        let notes = one_section(from, slot, section)
            .apply(to, &mut target, &mut players)
            .expect("applies");
        let stored = players
            .iter()
            .find(|player| player.id == id)
            .expect("the player")
            .appearance
            .ingame_face
            .get(IngameFaceField::SkinColor)
            .expect("a stored skin");
        (slot, notes, stored, before)
    };

    let (slot, notes, stored, before) = apply(PesVersion::Pes21, PesVersion::Pes19, 7);
    assert_eq!(
        notes,
        vec![ImportNote::Capped {
            path: format!("players.{:02}.appearance.skin_color", slot + 1),
            from: 7,
            to: 1,
        }]
    );
    assert_ne!(before, 1, "a reset to 1 would be invisible");
    assert_eq!(stored, 1);

    let (_, notes, stored, before) = apply(PesVersion::Pes19, PesVersion::Pes17, 7);
    assert!(notes.is_empty(), "{notes:?}");
    assert_ne!(before, 7, "a write of 7 would be invisible");
    assert_eq!(stored, 7);

    let (_, notes, stored, before) = apply(PesVersion::Pes21, PesVersion::Pes18, 2);
    assert!(notes.is_empty(), "{notes:?}");
    assert_ne!(before, 2, "a write of 2 would be invisible");
    assert_eq!(stored, 2);
}

/// A name longer than the target field writes its `crop_player_text`
/// result — cut at whole spans — with a `TextCropped` note naming the
/// player's slot and the text as cut (`spec/pes_savefile/operations.md`
/// "Team TOML").
#[test]
fn a_name_longer_than_the_field_crops_to_it_with_a_note() {
    let (file, _) = open(PesVersion::Pes16);
    let (team, slot) = first_slot(&file);
    let max = usize::try_from(
        schema_for(PesVersion::Pes16)
            .player
            .texts
            .iter()
            .find(|spec| spec.text == PlayerText::Name)
            .expect("a name field")
            .len,
    )
    .expect("fits usize");
    assert_eq!(max, 46, "the field holds 46 bytes");

    // A section that writes cleanly at a lower slot, and an overlong one:
    // both write, the long one as its crop.
    assert!(
        team.roster[slot + 1].player_id != 0,
        "the next slot is rostered"
    );
    let mut doc = TeamToml {
        pes_version: Some(PesVersion::Pes19),
        ..TeamToml::default()
    };
    doc.players.insert(
        u8::try_from(slot + 1).expect("a slot index"),
        PlayerSection {
            name: Some("FINE".to_string()),
            ..PlayerSection::default()
        },
    );
    doc.players.insert(
        u8::try_from(slot + 2).expect("a slot index"),
        PlayerSection {
            name: Some("N".repeat(60)),
            ..PlayerSection::default()
        },
    );
    let mut target = team.clone();
    let mut players = file.players().to_vec();
    let notes = doc
        .apply(PesVersion::Pes16, &mut target, &mut players)
        .expect("a long name crops, never refuses");
    assert_eq!(
        notes,
        [ImportNote::TextCropped {
            path: format!("players.{:02}.name", slot + 2),
            cropped: "N".repeat(46),
        }]
    );
    assert_eq!(
        players
            .iter()
            .find(|p| p.id == team.roster[slot + 1].player_id)
            .expect("the cropped player")
            .name,
        "N".repeat(46),
        "the name cut to the field"
    );
    assert_eq!(
        players
            .iter()
            .find(|p| p.id == team.roster[slot].player_id)
            .expect("the earlier player")
            .name,
        "FINE",
        "the earlier slot wrote cleanly"
    );
}

/// A name or shirt name of exactly the field's capacity applies; one
/// over crops to the field with a `TextCropped` note (`apply` crops,
/// never refuses, a longer name or shirt name).
#[test]
fn a_player_text_at_the_fields_capacity_applies_and_one_more_crops() {
    let (file, _) = open(PesVersion::Pes16);
    let (team, slot) = first_slot(&file);
    let id = team.roster[slot].player_id;
    let texts = schema_for(PesVersion::Pes16).player.texts;
    let name_max = text_max(texts, PlayerText::Name);
    let shirt_max = text_max(texts, PlayerText::ShirtName);

    let section = PlayerSection {
        name: Some("N".repeat(name_max)),
        shirt_name: Some("S".repeat(shirt_max)),
        ..PlayerSection::default()
    };
    let mut target = team.clone();
    let mut players = file.players().to_vec();
    one_section(PesVersion::Pes16, slot, section)
        .apply(PesVersion::Pes16, &mut target, &mut players)
        .expect("at capacity applies");
    let applied = players
        .iter()
        .find(|player| player.id == id)
        .expect("the player");
    assert_eq!(applied.name, "N".repeat(name_max));
    assert_eq!(applied.shirt_name, "S".repeat(shirt_max));

    for (name_over, shirt_over, leaf, cropped) in [
        (true, false, "name", "N".repeat(name_max)),
        (false, true, "shirt_name", "S".repeat(shirt_max)),
    ] {
        let section = PlayerSection {
            name: Some("N".repeat(name_max + usize::from(name_over))),
            shirt_name: Some("S".repeat(shirt_max + usize::from(shirt_over))),
            ..PlayerSection::default()
        };
        let mut target = team.clone();
        let mut players = file.players().to_vec();
        let notes = one_section(PesVersion::Pes16, slot, section)
            .apply(PesVersion::Pes16, &mut target, &mut players)
            .expect("one over crops");
        assert_eq!(
            notes,
            [ImportNote::TextCropped {
                path: format!("players.{:02}.{leaf}", slot + 1),
                cropped,
            }],
            "{leaf}"
        );
        let applied = players
            .iter()
            .find(|player| player.id == id)
            .expect("the player");
        if leaf == "name" {
            assert_eq!(applied.name, "N".repeat(name_max));
        } else {
            assert_eq!(applied.shirt_name, "S".repeat(shirt_max));
        }
    }
}

/// PES 16's shirt field is the one outlier the portable limit leaves
/// out: a shirt name past its 16 chars crops to 16 with a `TextCropped`
/// note, never a refusal (`spec/pes_savefile/operations.md` "Team TOML").
#[test]
fn a_pes16_shirt_name_past_the_field_crops_to_16_with_a_note() {
    let (file, _) = open(PesVersion::Pes16);
    let (team, slot) = first_slot(&file);
    let id = team.roster[slot].player_id;
    let text = "ABCDEFGHIJKLMNOPQRST"; // 20 chars over PES 16's 16-char field.

    let section = PlayerSection {
        shirt_name: Some(text.to_string()),
        ..PlayerSection::default()
    };
    let mut target = team.clone();
    let mut players = file.players().to_vec();
    let notes = one_section(PesVersion::Pes16, slot, section)
        .apply(PesVersion::Pes16, &mut target, &mut players)
        .expect("a PES 16 shirt name crops, never refuses");
    assert_eq!(
        notes,
        [ImportNote::TextCropped {
            path: format!("players.{:02}.shirt_name", slot + 1),
            cropped: "ABCDEFGHIJKLMNOP".to_string(),
        }]
    );
    let applied = players
        .iter()
        .find(|player| player.id == id)
        .expect("the player");
    assert_eq!(applied.shirt_name, "ABCDEFGHIJKLMNOP");
}

/// A PES 21 document's 52-byte name, applied to the PES 17 save (46-byte
/// field), writes the 46-byte crop with one note — the cross-version case
/// the portable limits exist for.
#[test]
fn a_pes21_name_applied_to_pes17_writes_the_46_byte_crop() {
    let (file, _) = open(PesVersion::Pes17);
    let (team, slot) = first_slot(&file);
    let id = team.roster[slot].player_id;
    let section = PlayerSection {
        name: Some("N".repeat(52)),
        ..PlayerSection::default()
    };
    let mut target = team.clone();
    let mut players = file.players().to_vec();
    let notes = one_section(PesVersion::Pes21, slot, section)
        .apply(PesVersion::Pes17, &mut target, &mut players)
        .expect("the longer name crops, never refuses");
    assert_eq!(
        notes,
        [ImportNote::TextCropped {
            path: format!("players.{:02}.name", slot + 1),
            cropped: "N".repeat(46),
        }]
    );
    let applied = players
        .iter()
        .find(|player| player.id == id)
        .expect("the player");
    assert_eq!(applied.name, "N".repeat(46));
}

/// A `shirt_name` of exactly the field's capacity applies; one more char
/// crops to the field with a `TextCropped` note — on every version, not
/// only PES 16 (`spec/pes_savefile/operations.md` "Team TOML").
#[test]
fn a_shirt_name_at_capacity_applies_and_one_more_crops() {
    let (file, _) = open(PesVersion::Pes19);
    let (team, slot) = first_slot(&file);
    let id = team.roster[slot].player_id;
    let max = text_max(
        schema_for(PesVersion::Pes19).player.texts,
        PlayerText::ShirtName,
    );

    let section = PlayerSection {
        shirt_name: Some("a".repeat(max)),
        ..PlayerSection::default()
    };
    let mut target = team.clone();
    let mut players = file.players().to_vec();
    one_section(PesVersion::Pes19, slot, section)
        .apply(PesVersion::Pes19, &mut target, &mut players)
        .expect("at capacity applies");
    let applied = players
        .iter()
        .find(|player| player.id == id)
        .expect("the player");
    assert_eq!(applied.shirt_name, "a".repeat(max));

    let section = PlayerSection {
        shirt_name: Some("a".repeat(max + 1)),
        ..PlayerSection::default()
    };
    let mut target = team.clone();
    let mut players = file.players().to_vec();
    let notes = one_section(PesVersion::Pes19, slot, section)
        .apply(PesVersion::Pes19, &mut target, &mut players)
        .expect("one char over crops");
    assert_eq!(
        notes,
        [ImportNote::TextCropped {
            path: format!("players.{:02}.shirt_name", slot + 1),
            cropped: "a".repeat(max),
        }]
    );
    let applied = players
        .iter()
        .find(|player| player.id == id)
        .expect("the player");
    assert_eq!(applied.shirt_name, "a".repeat(max));
}

/// The `NotEncodable` note names the label of the style the target
/// cannot store — not some other style's.
#[test]
fn an_unencodable_playing_style_notes_its_own_label() {
    // Roaming Flank is a PES 19+ style; 17/18 cannot store it.
    let (file, _) = open(PesVersion::Pes18);
    let (team, slot) = first_slot(&file);
    let id = team.roster[slot].player_id;
    let mut section = PlayerSection::default();
    section.positions.playing_style = Some(PlayStyle::RoamingFlank);
    let mut target = team.clone();
    let mut players = file.players().to_vec();
    let notes = one_section(PesVersion::Pes19, slot, section)
        .apply(PesVersion::Pes18, &mut target, &mut players)
        .expect("applies");
    assert_eq!(
        notes,
        vec![ImportNote::NotEncodable {
            path: format!("players.{:02}.positions.playing_style", slot + 1),
            label: "roaming_flank".to_string(),
        }]
    );
    let before = file.player(id).expect("the player").positions.playing_style;
    let none =
        playstyle::encode(PesVersion::Pes18, PlayStyle::None).expect("every list holds none");
    assert_ne!(before, none, "the fallback would be invisible");
    let stored = players
        .iter()
        .find(|player| player.id == id)
        .expect("the player")
        .positions
        .playing_style;
    assert_eq!(stored, none, "the stored style falls back to none");
}

/// `from_team` carries every rostered slot's shirt number into
/// `players[NN].number`.
#[test]
fn from_team_carries_every_rostered_slots_shirt_number() {
    for version in FIXTURES {
        let (file, _) = open(version);
        let refs: Vec<&PlayerEntry> = file.players().iter().collect();
        for team in file
            .teams()
            .iter()
            .filter(|team| team.roster.iter().any(|slot| slot.player_id != 0))
        {
            let doc = TeamToml::from_team(version, team, &refs).expect("from_team");
            for (i, slot) in team.roster.iter().enumerate() {
                if slot.player_id == 0 {
                    continue;
                }
                let key = u8::try_from(i + 1).expect("a slot index");
                assert_eq!(
                    doc.players.get(&key).and_then(|section| section.number),
                    Some(slot.number),
                    "{version:?} team {} slot {key}",
                    team.id
                );
            }
        }
    }
}

/// A name or shirt name past the portable limit emits a note on its own
/// line, in the file the dump writes (`spec/pes_savefile/operations.md`
/// "Team TOML", the portable text limits).
#[test]
fn a_name_or_shirt_name_past_the_portable_limits_carries_a_note() {
    let (mut file, _) = open(PesVersion::Pes21);
    let (team_id, slot, player_id) = {
        let (team, slot) = first_slot(&file);
        (team.id, slot, team.roster[slot].player_id)
    };
    let player = file.player_mut(player_id).expect("the player");
    player.name = "N".repeat(59);
    player.shirt_name = "S".repeat(19);
    let refs: Vec<&PlayerEntry> = file.players().iter().collect();
    let doc = TeamToml::from_team(
        PesVersion::Pes21,
        file.team(team_id).expect("the team"),
        &refs,
    )
    .expect("from_team");
    let text = doc.to_toml().expect("to_toml");
    let block = text
        .split_once(&format!("[players.{:02}]", slot + 1))
        .and_then(|(_, rest)| rest.split_once("\n["))
        .map(|(block, _)| block)
        .expect("the player's block");
    let name_line = block
        .lines()
        .find(|line| line.starts_with("name ="))
        .expect("the name line");
    assert!(
        name_line.contains("# name past 46 bytes: cut on PES 2015 to 2019"),
        "{name_line}"
    );
    let shirt_line = block
        .lines()
        .find(|line| line.starts_with("shirt_name ="))
        .expect("the shirt_name line");
    assert!(
        shirt_line
            .contains("# shirt name past 18 letters: cut on PES 2015 to 2019 (PES 2016: past 16)"),
        "{shirt_line}"
    );
    // A text inside the limits gets no note.
    let line = text
        .lines()
        .find(|line| line.starts_with("name =") && !line.contains("past 46"))
        .expect("an unnoted name line");
    assert!(line.contains("name = "), "{line}");
    // Exactly the limit gets none either (46 bytes of name, 18 letters of
    // shirt): a `>=` would stamp a "cut" note nothing makes.
    for (which, text) in [
        (PlayerText::Name, "n".repeat(46)),
        (PlayerText::ShirtName, "s".repeat(18)),
    ] {
        assert!(
            portable_note(which, Some(&text)).is_empty(),
            "{which:?} at the limit noted: {:?}",
            portable_note(which, Some(&text))
        );
    }
}

/// A key the document's version lacks emits the commented neutral line,
/// comment included.
#[test]
fn a_key_the_version_lacks_emits_its_commented_neutral_line() {
    // star is PES 19+: on a PES 18 document it is `# star = 0`.
    let (file, _) = open(PesVersion::Pes18);
    let (team, _) = first_slot(&file);
    let refs: Vec<&PlayerEntry> = file.players().iter().collect();
    let doc = TeamToml::from_team(PesVersion::Pes18, team, &refs).expect("from_team");
    let text = doc.to_toml().expect("to_toml");
    let line = text
        .lines()
        .find(|line| line.contains("star ="))
        .expect("a star line, commented or not");
    assert!(line.starts_with("# star = 0"), "{line}");
    assert!(line.contains("# PES 19+, 0-7 stored"), "{line}");
}

/// Stored-ranges mode is bounded by the field's widest bit width: neck
/// length's 4 bits refuse 20 (stored 27) with the width's range.
#[test]
fn a_stored_value_past_the_fields_widest_width_is_out_of_range() {
    use crate::settings_toml::SettingsError;

    let err = TeamToml::parse("[players.01.appearance.physique]\nneck_length = 20\n")
        .expect_err("out of range");
    assert!(
        matches!(
            err,
            TeamTomlError::Settings(SettingsError::OutOfRange { ref range, .. })
                if range == "0 to 15"
        ),
        "{err:?}"
    );
}

/// A stored value a `Kind::Labels` list does not name emits as the
/// integer and reparses to the same document.
#[test]
fn a_width_valid_stored_label_without_a_label_round_trips_as_an_integer() {
    let (file, _) = open(PesVersion::Pes19);
    let (team, _) = first_slot(&file);
    // sleeves: labels 0..=2, a 2-bit stored field — 3 fits the width.
    let mut players: Vec<PlayerEntry> = file.players().to_vec();
    let id = team
        .roster
        .iter()
        .find(|slot| slot.player_id != 0)
        .expect("a rostered slot")
        .player_id;
    players
        .iter_mut()
        .find(|player| player.id == id)
        .expect("the player")
        .appearance
        .sleeves = 3;
    let refs: Vec<&PlayerEntry> = players.iter().collect();
    let doc = TeamToml::from_team(PesVersion::Pes19, team, &refs).expect("from_team");
    let text = doc.to_toml().expect("to_toml");
    assert!(
        text.lines().any(|line| line.starts_with("sleeves = 3")),
        "the stored integer emits, not a panic:\n{}",
        text.lines()
            .find(|line| line.contains("sleeves"))
            .unwrap_or("(no sleeves line)")
    );
    let parsed = TeamToml::parse(&text).expect("reparses");
    assert_eq!(parsed, doc);
}

/// A shirt number wider than the target's roster field is refused at
/// apply (`write_roster` would fail after `apply` returned Ok), never
/// clamped. PES 18's field is 8 bits; PES 19's is 16.
#[test]
fn a_number_past_the_target_fields_width_is_refused() {
    let doc = TeamToml::parse("pes_version = 19\n\n[players.01]\nnumber = 999\n").expect("parses");

    let (file18, _) = open(PesVersion::Pes18);
    let team18 = file18
        .teams()
        .iter()
        .find(|team| team.roster.first().is_some_and(|s| s.player_id != 0))
        .expect("a team with slot 01 rostered");
    let mut players18 = file18.players().to_vec();
    let mut target = team18.clone();
    let err = doc
        .apply(PesVersion::Pes18, &mut target, &mut players18)
        .expect_err("999 does not fit an 8-bit number field");
    assert!(
        matches!(
            err,
            TeamTomlError::OutOfRange {
                ref key,
                value: 999,
                ref range,
            } if key == "players.01.number" && range == "0 to 255"
        ),
        "{err:?}"
    );
    assert_eq!(&target, team18, "nothing was written");
    let edge = TeamToml::parse("pes_version = 19\n\n[players.01]\nnumber = 255\n").expect("parses");
    edge.apply(PesVersion::Pes18, &mut target, &mut players18)
        .expect("255 is the 8-bit field's last value");
    assert_eq!(target.roster[0].number, 255);

    let (file19, _) = open(PesVersion::Pes19);
    let team19 = file19
        .teams()
        .iter()
        .find(|team| team.roster.first().is_some_and(|s| s.player_id != 0))
        .expect("a team with slot 01 rostered");
    let mut players19 = file19.players().to_vec();
    let mut target = team19.clone();
    doc.apply(PesVersion::Pes19, &mut target, &mut players19)
        .expect("PES 19's 16-bit field holds 999");
    assert_eq!(target.roster[0].number, 999);
}

/// Hostile integers reach the stored-width check through checked math:
/// the `value + 7` / `value - 1` shifts of Signed7 and OneBased keys do
/// not overflow-panic.
#[test]
fn extreme_integers_are_out_of_range_not_a_panic() {
    use crate::settings_toml::SettingsError;

    for (table, key, value) in [
        ("physique", "neck_length", "9223372036854775807"),
        ("motion", "hunching_dribbling", "-9223372036854775808"),
    ] {
        let doc = format!("[players.01.appearance.{table}]\n{key} = {value}\n");
        let err = TeamToml::parse(&doc).expect_err("out of range");
        assert!(
            matches!(
                err,
                TeamTomlError::Settings(SettingsError::OutOfRange { .. })
            ),
            "{key} = {value}: {err:?}"
        );
    }
}

/// The own-id convention compares with checked math: a team id near
/// `u32::MAX` cannot be `id * 100 + slot`, so it compares false and the
/// document's value lands, rather than panicking in debug builds.
#[test]
fn a_maximal_team_id_does_not_overflow_the_own_id_check() {
    let (file, _) = open(PesVersion::Pes19);
    let (team, slot) = first_slot(&file);
    let mut players = file.players().to_vec();
    let mut target = team.clone();
    let doc = TeamToml::parse(&format!(
        "pes_version = 19\n\n[team]\nid = 4294967295\n\n[players.{:02}]\nbase_copy_id = 7\n",
        slot + 1
    ))
    .expect("parses");
    doc.apply(PesVersion::Pes19, &mut target, &mut players)
        .expect("applies");
    let id = team.roster[slot].player_id;
    let player = players.iter().find(|p| p.id == id).expect("rostered");
    assert_eq!(player.appearance.base_copy_id, 7);
}

/// A stored value the target version's field cannot hold is refused at
/// apply — `write_player` would reject it afterwards, breaking the
/// all-or-nothing contract. `free_kick` is stored 4 bits on 15-19 and
/// 5 bits on 20/21, so `free_kick = 17` (stored 16) is a real width
/// difference between versions.
#[test]
fn a_value_past_the_target_fields_width_is_refused() {
    let (file19, _) = open(PesVersion::Pes19);
    let (team19, slot) = first_slot(&file19);
    let doc = TeamToml::parse(&format!(
        "pes_version = 19\n\n[players.{:02}.appearance.motion]\nfree_kick = 17\n",
        slot + 1
    ))
    .expect("parses");

    let mut players19 = file19.players().to_vec();
    let mut target19 = team19.clone();
    let err = doc
        .apply(PesVersion::Pes19, &mut target19, &mut players19)
        .expect_err("16 does not fit a 4-bit field");
    assert!(
        matches!(
            err,
            TeamTomlError::OutOfRange {
                ref key,
                value: 16,
                ref range,
            } if key.ends_with(".appearance.motion.free_kick") && range == "0 to 15"
        ),
        "{err:?}"
    );
    assert_eq!(&target19, team19, "the team changed");
    assert_eq!(players19, file19.players(), "a player changed");

    let (file21, _) = open(PesVersion::Pes21);
    let (team21, slot21) = first_slot(&file21);
    let doc21 = TeamToml::parse(&format!(
        "pes_version = 21\n\n[players.{:02}.appearance.motion]\nfree_kick = 17\n",
        slot21 + 1
    ))
    .expect("parses");
    let mut players21 = file21.players().to_vec();
    let mut target21 = team21.clone();
    doc21
        .apply(PesVersion::Pes21, &mut target21, &mut players21)
        .expect("PES 21's 5-bit field holds it");
}

/// The boundary, not just a miss: `boots_id`'s 14-bit field takes 16383
/// and refuses 16384.
#[test]
fn a_value_at_the_width_boundary_applies_and_one_over_is_refused() {
    let (file, _) = open(PesVersion::Pes19);
    let (team, slot) = first_slot(&file);
    let doc = |value| {
        TeamToml::parse(&format!(
            "pes_version = 19\n\n[players.{:02}]\nboots_id = {value}\n",
            slot + 1
        ))
        .expect("parses")
    };

    let mut players = file.players().to_vec();
    let mut target = team.clone();
    doc(16383)
        .apply(PesVersion::Pes19, &mut target, &mut players)
        .expect("16383 fits 14 bits");

    let mut players = file.players().to_vec();
    let mut target = team.clone();
    let err = doc(16384)
        .apply(PesVersion::Pes19, &mut target, &mut players)
        .expect_err("16384 does not fit 14 bits");
    assert!(
        matches!(
            err,
            TeamTomlError::OutOfRange {
                ref key,
                value: 16384,
                ref range,
            } if key.ends_with(".boots_id") && range == "0 to 16383"
        ),
        "{err:?}"
    );
    assert_eq!(&target, team);
    assert_eq!(players, file.players());
}

/// The same check on a caller-built document (no parse): an appearance
/// value wider than its field — `sleeves` is 2 bits — is refused.
#[test]
fn a_caller_built_appearance_value_past_the_field_width_is_refused() {
    let (file, _) = open(PesVersion::Pes19);
    let (team, slot) = first_slot(&file);
    let mut section = PlayerSection::default();
    section.appearance.strip.sleeves = Some(5);
    let mut doc = TeamToml::default();
    doc.players
        .insert(u8::try_from(slot + 1).expect("a slot"), section);

    let mut players = file.players().to_vec();
    let mut target = team.clone();
    let err = doc
        .apply(PesVersion::Pes19, &mut target, &mut players)
        .expect_err("5 does not fit a 2-bit field");
    assert!(
        matches!(
            err,
            TeamTomlError::OutOfRange {
                ref key,
                value: 5,
                ref range,
            } if key.ends_with(".appearance.strip.sleeves") && range == "0 to 3"
        ),
        "{err:?}"
    );
    assert_eq!(&target, team);
    assert_eq!(players, file.players());
}

/// A char above U+00FF cannot encode into the single-byte fields:
/// `shirt_name = "Ā"` is refused at apply with nothing written, and a
/// caller-built document with a NUL in `name` is refused the same way.
#[test]
fn unencodable_text_is_refused_at_apply() {
    let (file, _) = open(PesVersion::Pes19);
    let (team, slot) = first_slot(&file);
    let key = u8::try_from(slot + 1).expect("a slot");

    let doc = TeamToml::parse(&format!(
        "pes_version = 19\n\n[players.{key:02}]\nshirt_name = \"Ā\"\n"
    ))
    .expect("parses");
    let mut players = file.players().to_vec();
    let mut target = team.clone();
    let err = doc
        .apply(PesVersion::Pes19, &mut target, &mut players)
        .expect_err("Ā is above U+00FF");
    assert!(
        matches!(err, TeamTomlError::WrongType { ref key, .. } if key.ends_with(".shirt_name")),
        "{err:?}"
    );
    assert_eq!(&target, team);
    assert_eq!(players, file.players());

    let section = PlayerSection {
        name: Some("A\0B".to_string()),
        ..PlayerSection::default()
    };
    let mut doc = TeamToml::default();
    doc.players.insert(key, section);
    let mut players = file.players().to_vec();
    let mut target = team.clone();
    let err = doc
        .apply(PesVersion::Pes19, &mut target, &mut players)
        .expect_err("a NUL is refused");
    assert!(
        matches!(err, TeamTomlError::WrongType { ref key, .. } if key.ends_with(".name")),
        "{err:?}"
    );
    assert_eq!(&target, team);
    assert_eq!(players, file.players());
}

/// `é` and `ÿ` (U+00FF, the last single-byte char) are one byte each in
/// the single-byte encoding: they apply and survive a
/// `write_player`/`read_player` round trip on the PES 19 fixture.
#[test]
fn a_latin1_shirt_name_round_trips_through_the_codec() {
    let (file, _) = open(PesVersion::Pes19);
    let (team, slot) = first_slot(&file);
    let doc = TeamToml::parse(&format!(
        "pes_version = 19\n\n[players.{:02}]\nshirt_name = \"éÿ\"\n",
        slot + 1
    ))
    .expect("parses");
    let mut players = file.players().to_vec();
    let mut target = team.clone();
    doc.apply(PesVersion::Pes19, &mut target, &mut players)
        .expect("applies");

    let id = team.roster[slot].player_id;
    let player = players.iter().find(|p| p.id == id).expect("rostered");
    assert_eq!(player.shirt_name, "éÿ");
    let schema = schema_for(PesVersion::Pes19);
    let mut record = vec![0u8; schema.player.size];
    crate::codec::write_player(player, &mut record, schema.player).expect("write_player");
    let back = crate::codec::read_player(&record, schema.player).expect("read_player");
    assert_eq!(back.shirt_name, "éÿ");
}

/// An export carries the run as keys, `[appearance.hair]` included, the
/// text holds no `ingame_face` or `[appearance.unknown]`, and the
/// document applied back onto the save it came from leaves every player
/// identical — the claim
/// `every_fixture_player_round_trips_through_the_document` makes on
/// every fixture, witnessed here on one.
#[test]
fn an_exported_player_carries_his_hair_and_the_document_imports_identical() {
    let (file, _) = open(PesVersion::Pes21);
    let refs: Vec<&PlayerEntry> = file.players().iter().collect();
    let (team, slot) = first_slot(&file);
    let id = team.roster[slot].player_id;
    let player = file.player(id).expect("rostered");
    let doc = TeamToml::from_team(PesVersion::Pes21, team, &refs).expect("from_team");
    let key = u8::try_from(slot + 1).expect("a slot index");
    let face = &player.appearance.ingame_face;
    let hair = &doc.players[&key].appearance.hair;
    assert_eq!(
        hair.style,
        Some(face.get(IngameFaceField::HairStyle).expect("reads"))
    );
    assert_eq!(
        hair.color_red,
        Some(face.get(IngameFaceField::HairColorRed).expect("reads"))
    );
    assert_eq!(hair.pes15_length, None, "PES 15's keys are not PES 21's");
    let text = doc.to_toml().expect("to_toml");
    assert!(text.contains(&format!("[players.{key:02}.appearance.hair]")));
    assert!(
        !text.contains("ingame_face") && !text.contains("unknown"),
        "the retired keys are out of the text"
    );
    let parsed = TeamToml::parse(&text).expect("the emitted text reparses");
    assert_eq!(parsed, doc);
    let mut target_team = team.clone();
    let mut players = file.players().to_vec();
    parsed
        .apply(PesVersion::Pes21, &mut target_team, &mut players)
        .expect("applies");
    assert_eq!(&target_team, team);
    assert_eq!(players, file.players(), "every player identical");
}

/// `stripped` is no key: an export writes none, not even as a comment, and
/// a player block holding one is refused as any stranger is.
#[test]
fn stripped_is_an_unknown_player_key() {
    let (file, _) = open(PesVersion::Pes21);
    let refs: Vec<&PlayerEntry> = file.players().iter().collect();
    let (team, slot) = first_slot(&file);
    let text = TeamToml::from_team(PesVersion::Pes21, team, &refs)
        .expect("from_team")
        .to_toml()
        .expect("to_toml");
    let header = format!("[players.{:02}]", slot + 1);
    assert!(text.contains(&header), "a player block is written");
    assert!(!text.contains("stripped"), "{text}");

    let err = TeamToml::parse("[players.01]\nstripped = false\n").expect_err("a retired key");
    assert!(
        matches!(
            err,
            TeamTomlError::UnknownKey { ref key } if key == "players.01.stripped"
        ),
        "{err:?}"
    );
}

/// SE-TOML-04's lib half: a document of `[team] id`, one name and one
/// physique key writes exactly those onto slot 3's player — every
/// other field of his, appearance fields included, and every other
/// player and team, is unchanged.
#[test]
fn a_minimal_document_writes_only_its_keys() {
    let (file, _) = open(PesVersion::Pes21);
    let team = file
        .teams()
        .iter()
        .find(|team| team.roster.len() >= 3 && team.roster[2].player_id != 0)
        .expect("a team whose slot 3 is rostered");
    let id = team.roster[2].player_id;
    let doc = TeamToml::parse(&format!(
        "[team]\nid = {}\n\n[players.03]\nname = \"PATCH\"\n\n[players.03.appearance.physique]\nneck_length = 2\n",
        team.id
    ))
    .expect("parses");
    let mut target = team.clone();
    let mut players = file.players().to_vec();
    let notes = doc
        .apply(PesVersion::Pes21, &mut target, &mut players)
        .expect("applies");
    assert!(notes.is_empty(), "{notes:?}");
    let mut expected: Vec<PlayerEntry> = file.players().to_vec();
    {
        let patched = expected
            .iter_mut()
            .find(|p| p.id == id)
            .expect("the player");
        patched.name = "PATCH".to_string();
        // `neck_length = 2` is the TOML number: stored + 7.
        patched.appearance.neck_length = 9;
    }
    assert_eq!(players, expected, "only slot 3's two keys changed");
    assert_eq!(&target, team, "no team field was written");
}

/// SE-TOML-05's lib half: a PES 21 document on a PES 18 target converts
/// — a face type over the older cap is written 0 with a `Capped` note,
/// a key the older version lacks is a `NotInThisVersion` note, and a
/// shared field applies as stored. PES 18's facial-hair cap is the
/// field's bits (31, an unmeasured column), so a document value past it
/// is made by hand.
#[test]
fn a_pes21_document_on_a_pes18_target_converts() {
    let (file21, _) = open(PesVersion::Pes21);
    let (file18, _) = open(PesVersion::Pes18);
    let team21 = file21.team(701).expect("team 701 is in both fixtures");
    let team18 = file18.team(701).expect("team 701 is in both fixtures");
    let refs: Vec<&PlayerEntry> = file21.players().iter().collect();
    let mut doc = TeamToml::from_team(PesVersion::Pes21, team21, &refs).expect("from_team");
    let cap18 =
        face_type_cap(PesVersion::Pes18, IngameFaceField::FacialHairType).expect("a PES 18 cap");
    let over = cap18 + 1;
    let first = doc.players.values_mut().next().expect("a player");
    first.appearance.face.facial_hair_type = Some(over);
    // A celebration past PES 18's shown top (127) and a free kick past its
    // 4-bit field: both write at the target's top, each with its note.
    first.appearance.motion.goal_celebration_1 = Some(162);
    first.appearance.motion.free_kick = Some(19);
    let mut target_team = team18.clone();
    let mut targets: Vec<PlayerEntry> = file18.players().to_vec();
    let notes = doc
        .apply(PesVersion::Pes18, &mut target_team, &mut targets)
        .expect("a foreign apply is notes, not errors");
    assert!(
        notes.iter().any(|note| matches!(
            note,
            ImportNote::Capped { path, from, to }
                if path.ends_with(".appearance.face.facial_hair_type")
                    && *from == over
                    && *to == 0
        )),
        "a Capped note for facial_hair_type, reset to 0: {notes:?}"
    );
    for (suffix, from, to) in [
        (".appearance.motion.goal_celebration_1", 162u8, 127u8),
        (".appearance.motion.free_kick", 20, 16),
    ] {
        assert!(
            notes.iter().any(|note| matches!(
                note,
                ImportNote::Capped { path, from: f, to: t }
                    if path.ends_with(suffix) && *f == from && *t == to
            )),
            "a Capped note for {suffix}, {from} to {to} as the document shows them: {notes:?}"
        );
    }
    assert!(
        notes.iter().any(|note| matches!(
            note,
            ImportNote::NotInThisVersion { path }
                if path.ends_with(".appearance.motion.dribbling")
        )),
        "a 20/21-only key is a note: {notes:?}"
    );
    let first_slot = team18
        .roster
        .iter()
        .position(|slot| slot.player_id != 0)
        .expect("a rostered slot");
    let target = targets
        .iter()
        .find(|player| player.id == team18.roster[first_slot].player_id)
        .expect("the player");
    assert_eq!(
        target
            .appearance
            .ingame_face
            .get(IngameFaceField::FacialHairType)
            .expect("a stored type"),
        0,
        "the over-cap face type is written 0"
    );
    assert_eq!(
        target.motion.goal_celebration_1, 127,
        "the over-top celebration is written at PES 18's top"
    );
    assert_eq!(
        target.motion.free_kick, 15,
        "the over-wide free kick is written at PES 18's top"
    );
    let source = file21
        .player(team18.roster[first_slot].player_id)
        .expect("the source player");
    // The carry witness is `playable` (his only field that differs
    // between the fixtures: `[0; 13]` on 21 against a CF rating on 18);
    // the stats are identical for him, which is why it is not `finishing`.
    let before_playable = file18
        .player(team18.roster[first_slot].player_id)
        .expect("the target player")
        .positions
        .playable;
    assert_ne!(
        before_playable, source.positions.playable,
        "his playable ratings differ, or the carry is invisible"
    );
    assert_eq!(
        target.positions.playable, source.positions.playable,
        "a field both versions hold carries its value"
    );
    // And nobody outside the roster was touched: every other player of
    // the PES 18 save is his before.
    let rostered: Vec<u32> = team18.roster.iter().map(|slot| slot.player_id).collect();
    for player in targets.iter().filter(|p| !rostered.contains(&p.id)) {
        assert_eq!(
            player,
            file18.player(player.id).expect("in the save"),
            "player {} outside the roster",
            player.id
        );
    }
}

/// A foreign motion stored at the target's top is not over it: the value
/// writes as stored and no `Capped` note names the key. 127 and 15 are
/// PES 18's tops in `motion_top`'s stored units (a celebration is stored
/// as shown, a 1-based key one below what the editor shows).
#[test]
fn a_foreign_motion_at_the_targets_top_converts_with_no_note() {
    let (file21, _) = open(PesVersion::Pes21);
    let (file18, _) = open(PesVersion::Pes18);
    let team21 = file21.team(701).expect("team 701 is in both fixtures");
    let team18 = file18.team(701).expect("team 701 is in both fixtures");
    let refs: Vec<&PlayerEntry> = file21.players().iter().collect();
    let mut doc = TeamToml::from_team(PesVersion::Pes21, team21, &refs).expect("from_team");
    let first = doc.players.values_mut().next().expect("a player");
    first.appearance.motion.goal_celebration_1 = Some(127);
    first.appearance.motion.free_kick = Some(15);
    let mut target_team = team18.clone();
    let mut targets: Vec<PlayerEntry> = file18.players().to_vec();
    let notes = doc
        .apply(PesVersion::Pes18, &mut target_team, &mut targets)
        .expect("a foreign apply is notes, not errors");
    for suffix in [
        ".appearance.motion.goal_celebration_1",
        ".appearance.motion.free_kick",
    ] {
        assert!(
            !notes.iter().any(|note| matches!(
                note,
                ImportNote::Capped { path, .. } if path.ends_with(suffix)
            )),
            "no Capped note for {suffix}: {notes:?}"
        );
    }
    let first_slot = team18
        .roster
        .iter()
        .position(|slot| slot.player_id != 0)
        .expect("a rostered slot");
    let target = targets
        .iter()
        .find(|player| player.id == team18.roster[first_slot].player_id)
        .expect("the player");
    assert_eq!(
        target.motion.goal_celebration_1, 127,
        "the celebration at PES 18's top is stored as is"
    );
    assert_eq!(
        target.motion.free_kick, 15,
        "the free kick at PES 18's top is stored as is"
    );
}

/// The `[players.NN]` header lines of `text`, in slot order (the `.stats`
/// et al. sub-headers differ at byte 11).
fn player_headers(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.starts_with("[players.") && line.as_bytes().get(11) == Some(&b']'))
        .collect()
}

/// The roster-order note: team 701 of the PES 21 fixture with slots 0 and 1's player
/// ids swapped carries the note on those headers alone.
#[test]
fn a_broken_roster_order_notes_the_held_ids_on_the_headers() {
    let (mut file, _) = open(PesVersion::Pes21);
    let team = file.team_mut(701).expect("team 701");
    assert_eq!(team.roster[0].player_id, 70101, "premise: slot 0's player");
    assert_eq!(team.roster[1].player_id, 70102, "premise: slot 1's player");
    (team.roster[0].player_id, team.roster[1].player_id) = (70102, 70101);
    let team = file.team(701).expect("team 701");
    let refs: Vec<&PlayerEntry> = file.players().iter().collect();
    let doc = TeamToml::from_team(PesVersion::Pes21, team, &refs).expect("from_team");
    assert_eq!(doc.roster_breaks, vec![(0, 70102), (1, 70101)]);
    let text = doc.to_toml().expect("to_toml");
    let headers = player_headers(&text);
    assert_eq!(
        headers[0],
        "[players.01]                    # roster slot 01 .. 40 (32 on PES 15-18); holds player 70102, not 70101: applied, this record lands on slot 01, apart from 70102's aesthetics export"
    );
    assert_eq!(
        headers[1],
        "[players.02]                    # roster slot 01 .. 40 (32 on PES 15-18); holds player 70101, not 70102: applied, this record lands on slot 02, apart from 70101's aesthetics export"
    );
    assert_eq!(
        headers
            .iter()
            .filter(|line| line.contains("holds player"))
            .count(),
        2,
        "no other header notes"
    );
}

/// An export whose roster keeps the order emits the headers it always
/// did: no note, the range comment as it is.
#[test]
fn an_ordered_roster_emits_no_note() {
    let (file, _) = open(PesVersion::Pes21);
    let team = file.team(701).expect("team 701");
    let refs: Vec<&PlayerEntry> = file.players().iter().collect();
    let doc = TeamToml::from_team(PesVersion::Pes21, team, &refs).expect("from_team");
    let text = doc.to_toml().expect("to_toml");
    assert!(!text.contains("holds player"), "no note anywhere");
    let headers = player_headers(&text);
    assert_eq!(
        headers[0],
        "[players.01]                    # roster slot 01 .. 40 (32 on PES 15-18)"
    );
}
