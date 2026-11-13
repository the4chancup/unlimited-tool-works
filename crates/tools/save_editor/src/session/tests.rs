mod journal;

use pes_savefile::model::ingame_face::IngameFace;
use pes_savefile::schema::fields::PlayerField;
use pes_savefile::schema::ingame_face::IngameFaceField;

use super::*;

/// Writes `version`'s fixture save under the test's scratch folder and
/// opens it as a `Session`.
fn open(version: PesVersion, test: &str) -> Session {
    let dir = crate::test_support::scratch(test);
    let path = dir.join("EDIT00000000");
    std::fs::write(&path, pes_savefile::fixture_save(version)).unwrap();
    Session::open(&path, Recorder::default()).unwrap()
}

/// Saves `session` over the file it was opened from, no game running.
fn save_in_place(session: &mut Session) {
    let path = session.path().to_path_buf();
    session.save_to(&path, &[], false).unwrap();
}

/// `entry` without what a roster slot owns (`id`, `appearance_id`,
/// `appearance.base_copy_id`, and the in-game face run apart from the
/// exported fields, which `a_move_carries_only_the_exported_face_fields`
/// checks on its own): the record a move carries.
fn record(entry: &PlayerEntry) -> PlayerEntry {
    let mut record = entry.clone();
    record.id = 0;
    record.appearance_id = None;
    record.appearance.base_copy_id = 0;
    record.appearance.ingame_face = IngameFace::default();
    record
}

// A player-list drop moves the players' records between the ids, every
// `id`, `appearance_id` and `base_copy_id` staying with its slot; the
// roster (and so the shirt numbers) is never written, and the whole
// move is one undo entry.
#[test]
fn a_move_trades_the_records_between_the_ids_one_undo_entry() {
    let mut session = open(PesVersion::Pes21, "move");
    // The fixture's placeholders are identical records: distinguish the
    // three the move rotates, so the trades are not vacuous; a save makes
    // them the baseline, so `is_dirty` measures the move alone.
    session.file.player_mut(70101).unwrap().name = "FIRST".to_owned();
    session.file.player_mut(70102).unwrap().stats.speed = 71;
    session.file.player_mut(70103).unwrap().appearance.boots_id = 13;
    save_in_place(&mut session);
    let before: Vec<PlayerEntry> = [70101, 70102, 70103]
        .iter()
        .map(|id| session.file().player(*id).unwrap().clone())
        .collect();
    let roster_before = session.file().team(701).unwrap().roster.clone();
    assert_ne!(
        record(&before[0]),
        record(&before[1]),
        "premise: the records differ"
    );
    assert_ne!(
        record(&before[1]),
        record(&before[2]),
        "premise: the records differ"
    );

    session.move_player(701, 2, 0);

    // Slot 0's id is still 70101 and holds the record 70103 had; 70102
    // holds 70101's, 70103 70102's.
    let after: Vec<PlayerEntry> = [70101, 70102, 70103]
        .iter()
        .map(|id| session.file().player(*id).unwrap().clone())
        .collect();
    for (moved, held) in after.iter().zip([2usize, 0, 1]) {
        assert_eq!(
            record(moved),
            record(&before[held]),
            "the record player {} had",
            before[held].id
        );
    }
    for (moved, id) in after.iter().zip([70101u32, 70102, 70103]) {
        assert_eq!(moved.id, id, "the id stays with the slot");
        assert_eq!(moved.appearance_id, Some(id), "the appearance id too");
    }
    assert_eq!(
        after[0].appearance.base_copy_id, before[0].appearance.base_copy_id,
        "the base copy stays with the slot"
    );
    assert_eq!(
        after[1].appearance.base_copy_id, before[1].appearance.base_copy_id,
        "the base copy stays with the slot"
    );
    assert_eq!(
        after[2].appearance.base_copy_id, before[2].appearance.base_copy_id,
        "the base copy stays with the slot"
    );
    assert_eq!(
        session.file().team(701).unwrap().roster,
        roster_before,
        "the roster's ids and shirt numbers are unchanged"
    );
    assert!(session.is_dirty());
    assert!(session.player_dirty(701, 0), "the moved players are marked");

    // One undo restores all three entries exactly; one redo reapplies.
    assert!(session.undo());
    for (id, entry) in [70101u32, 70102, 70103].iter().zip(&before) {
        assert_eq!(session.file().player(*id).unwrap(), entry);
    }
    assert!(!session.can_undo(), "the whole move is one undo entry");
    assert!(session.redo());
    for (id, entry) in [70101u32, 70102, 70103].iter().zip(&after) {
        assert_eq!(session.file().player(*id).unwrap(), entry);
    }
}

// A move to itself, onto an empty slot, from an empty slot or past the
// roster, and a team the save lacks: nothing changes, no undo entry.
#[test]
fn a_move_that_names_no_two_filled_slots_does_nothing() {
    let mut session = open(PesVersion::Pes21, "move_noop");
    let empty = session
        .file()
        .team(701)
        .unwrap()
        .roster
        .iter()
        .position(|slot| slot.player_id == 0)
        .map(|i| u8::try_from(i).unwrap())
        .expect("the roster has an empty slot");

    session.move_player(701, 0, 0);
    session.move_player(701, 0, empty);
    session.move_player(701, empty, 0);
    session.move_player(701, 0, 250);
    session.move_player(99999, 0, 1);

    assert!(!session.is_dirty());
    assert!(!session.can_undo());
}

// The drop outline's rule is `move_player`'s: a release onto another
// filled slot writes; onto the dragged slot itself, an empty slot at
// either end, a slot past the roster or a team the save lacks writes
// nothing.
#[test]
fn moves_player_names_only_a_drop_that_writes() {
    let mut session = open(PesVersion::Pes21, "moves_player");
    // Slot 1 emptied (70102's record stays in the pool, unrostered).
    session.file.team_mut(701).unwrap().roster[1].player_id = 0;
    let len = u8::try_from(session.file().team(701).unwrap().roster.len())
        .expect("the roster has at most 40 slots");

    assert!(session.moves_player(701, 0, 2), "two filled slots");
    assert!(!session.moves_player(701, 0, 0), "the dragged slot itself");
    assert!(!session.moves_player(701, 0, 1), "an empty slot onto");
    assert!(!session.moves_player(701, 1, 0), "an empty slot dragged");
    assert!(!session.moves_player(701, 0, len), "a slot past the roster");
    assert!(!session.moves_player(99999, 0, 1), "a team the save lacks");
}

// The roster slot's player id, the selection's and `apply_tier`'s one
// lookup: a filled slot's id, `None` for an empty slot, a slot past the
// roster and a team the save lacks.
#[test]
fn slot_player_is_the_slots_filled_id_or_none() {
    let session = open(PesVersion::Pes21, "slot_player");
    assert_eq!(session.slot_player(701, 0), Some(70101));
    let empty = session
        .file()
        .team(701)
        .unwrap()
        .roster
        .iter()
        .position(|slot| slot.player_id == 0)
        .map(|index| u8::try_from(index).unwrap())
        .expect("the roster has an empty slot");
    assert_eq!(session.slot_player(701, empty), None, "an empty slot");
    let past = u8::try_from(session.file().team(701).unwrap().roster.len())
        .expect("the roster has at most 40 slots");
    assert_eq!(
        session.slot_player(701, past),
        None,
        "a slot past the roster"
    );
    assert_eq!(session.slot_player(99999, 0), None, "a team the save lacks");
}

// An empty slot between the dragged and the target is skipped over — the records
// rotate over the filled slots.
#[test]
fn a_move_skips_over_an_empty_slot() {
    let mut session = open(PesVersion::Pes21, "move_empty");
    // Distinguish the records (the fixture's placeholders are identical),
    // then empty out slot 1 (70102's record stays in the pool, unrostered).
    session.file.player_mut(70101).unwrap().name = "FIRST".to_owned();
    session.file.player_mut(70103).unwrap().name = "THIRD".to_owned();
    session.file.team_mut(701).unwrap().roster[1].player_id = 0;
    let before: Vec<PlayerEntry> = [70101, 70102, 70103]
        .iter()
        .map(|id| session.file().player(*id).unwrap().clone())
        .collect();
    assert_ne!(
        record(&before[0]),
        record(&before[2]),
        "premise: the records differ"
    );

    session.move_player(701, 0, 2);

    // Slot 0 (70101) holds 70103's record, slot 2 (70103) 70101's; the
    // skipped slot stays empty and 70102's record is untouched.
    assert_eq!(
        record(session.file().player(70101).unwrap()),
        record(&before[2])
    );
    assert_eq!(
        record(session.file().player(70103).unwrap()),
        record(&before[0])
    );
    assert_eq!(
        record(session.file().player(70102).unwrap()),
        record(&before[1])
    );
    assert_eq!(session.file().team(701).unwrap().roster[1].player_id, 0);
    assert_eq!(session.file().team(701).unwrap().roster[0].player_id, 70101);
    assert_eq!(session.file().team(701).unwrap().roster[2].player_id, 70103);
}

/// Writes `value` into `field` of `id`'s in-game face run.
fn set_face(session: &mut Session, id: u32, field: IngameFaceField, value: u8) {
    session
        .file
        .player_mut(id)
        .unwrap()
        .appearance
        .ingame_face
        .set(field, value)
        .unwrap();
}

/// `field` of `id`'s in-game face run.
fn get_face(session: &Session, id: u32, field: IngameFaceField) -> u8 {
    session
        .file()
        .player(id)
        .unwrap()
        .appearance
        .ingame_face
        .get(field)
        .unwrap()
}

// The team row's dot covers the team's own entry —
// a player's edit does not set it, and an edit undone back to the
// saved state clears it.
#[test]
fn team_dirty_covers_the_teams_own_fields_alone() {
    let mut session = open(PesVersion::Pes17, "team_dirty");
    assert!(!session.team_dirty(701), "a fresh save is clean");

    // A short-name edit marks the team; one undo clears it again.
    let mut edited = session.file().team(701).unwrap().clone();
    edited.short_name = "XYZ".to_owned();
    session.edit_team(0, edited);
    assert!(session.team_dirty(701), "the short-name edit marks it");
    assert!(session.undo());
    assert!(!session.team_dirty(701), "undone back clears it");

    // A roster slot's shirt number stays the player's dot: the team's
    // is not set.
    let mut numbered = session.file().team(701).unwrap().clone();
    numbered.roster[0].number += 1;
    session.edit_team(0, numbered);
    assert!(!session.team_dirty(701), "a shirt number is not the team's");
    assert!(session.player_dirty(701, 0), "it is the player's");
    assert!(session.undo());

    // A player's edit does not mark his team.
    let mut player = session.file().player(70101).unwrap().clone();
    player.name = "RENAMED".to_owned();
    session.edit_player(0, player);
    assert!(session.player_dirty(701, 0), "the edit is the player's");
    assert!(!session.team_dirty(701), "the team's stays clear");
}

// A player-list move carries the fields `player_export` carries — the
// player-gloves, skin and iris fields inside the face run — and leaves
// the rest of the run (the face structure and the hair) with the slot.
#[test]
fn a_move_carries_only_the_exported_face_fields() {
    face_fields_move(PesVersion::Pes21, "move_face21");
}

// The same on a version whose appearance block is a separate record
// joined by id (PES 15/16, `appearance_id` `None`): it stays joined.
#[test]
fn a_move_carries_only_the_exported_face_fields_on_a_split_record() {
    face_fields_move(PesVersion::Pes16, "move_face16");
}

/// `marks`' (field, value on 70101, value on 70102) as the move leaves
/// them in `session` (a `face_fields_move` assert, on the open session
/// and again on the reopened save): the exported fields hold the
/// source's value, the rest the slot's own.
fn assert_face_move(session: &Session, marks: &[(IngameFaceField, u8, u8)]) {
    use IngameFaceField as F;
    for &(field, a, b) in marks {
        let carried = matches!(
            field,
            F::PlayerGloves | F::PlayerGlovesColor | F::SkinColor | F::IrisColor
        );
        // Slot 01 took 70102's record, slot 02 70101's: a carried field
        // holds the source's value, a kept one the slot's own.
        let (on_01, on_02) = if carried { (b, a) } else { (a, b) };
        assert_eq!(get_face(session, 70101, field), on_01, "{field:?}");
        assert_eq!(get_face(session, 70102, field), on_02, "{field:?}");
    }
}

/// The face-field half of `a_move_trades_the_records_between_the_ids` on
/// `version`: 70102's record moved onto slot 01 trades the exported
/// fields (gloves, skin, iris) and keeps each slot's own face and hair,
/// in memory and as the file written by Save holds it.
fn face_fields_move(version: PesVersion, test: &str) {
    use IngameFaceField as F;
    let mut session = open(version, test);
    // The fixture's runs are identical: mark each of the seven fields the
    // split decides, four carried by `player_export` and three that stay
    // (a hair field among them).
    let marks: [(F, u8, u8); 7] = [
        (F::Jawline, 5, 2),
        (F::NoseType, 7, 3),
        (F::HairStyle, 2, 5),
        (F::PlayerGloves, 1, 0),
        (F::PlayerGlovesColor, 5, 2),
        (F::SkinColor, 6, 2),
        (F::IrisColor, 9, 4),
    ];
    for &(field, a, b) in &marks {
        set_face(&mut session, 70101, field, a);
        set_face(&mut session, 70102, field, b);
    }
    for &(field, a, b) in &marks {
        assert_eq!(get_face(&session, 70101, field), a, "premise");
        assert_eq!(get_face(&session, 70102, field), b, "premise");
    }

    session.move_player(701, 1, 0);
    assert_face_move(&session, &marks);

    // Saved and reopened: the moved fields and the slots' faces are what
    // the file holds (SE-GUI-08's THEN after Save).
    save_in_place(&mut session);
    let reopened = Session::open(session.path(), Recorder::default()).unwrap();
    assert_face_move(&reopened, &marks);
}

// One undo entry per gesture: a second drag is its own entry, not a
// merge into the first's.
#[test]
fn two_moves_are_two_undo_steps() {
    let mut session = open(PesVersion::Pes21, "two_moves");
    session.file.player_mut(70101).unwrap().name = "FIRST".to_owned();
    session.file.player_mut(70102).unwrap().name = "SECOND".to_owned();
    session.file.player_mut(70103).unwrap().name = "THIRD".to_owned();

    session.move_player(701, 1, 0);
    session.move_player(701, 2, 1);
    assert_eq!(session.file().player(70101).unwrap().name, "SECOND");
    assert_eq!(session.file().player(70102).unwrap().name, "THIRD");
    assert_eq!(session.file().player(70103).unwrap().name, "FIRST");

    // One undo reverts only the second move; the next undoes the first.
    assert!(session.undo());
    assert_eq!(session.file().player(70101).unwrap().name, "SECOND");
    assert_eq!(session.file().player(70102).unwrap().name, "FIRST");
    assert_eq!(session.file().player(70103).unwrap().name, "THIRD");
    assert!(session.undo());
    assert_eq!(session.file().player(70101).unwrap().name, "FIRST");
    assert_eq!(session.file().player(70102).unwrap().name, "SECOND");
    assert_eq!(session.file().player(70103).unwrap().name, "THIRD");
    assert!(!session.can_undo());
}

/// `version`'s fixture with team 701's first eight players told apart
/// (slot `i` holds 70101 + `i`, named `P01` to `P08`, 171 to 178 cm
/// tall), saved so the file holds them: the fixture's placeholders are
/// identical records, which a move would trade without a trace.
fn told_apart(test: &str) -> Session {
    let mut session = open(PesVersion::Pes21, test);
    for n in 1..=8u8 {
        let id = 70100 + u32::from(n);
        assert_eq!(
            session.file().team(701).unwrap().roster[usize::from(n - 1)].player_id,
            id,
            "premise: slot {} holds {id}",
            n - 1
        );
        let player = session.file.player_mut(id).unwrap();
        player.name = format!("P{n:02}");
        player.basic.height = 170 + n;
    }
    save_in_place(&mut session);
    session
}

/// A `MovedPlace` of `slot`, its `id` and the `origin` whose record it holds.
fn place(slot: u8, id: u32, origin: u32) -> MovedPlace {
    MovedPlace { slot, id, origin }
}

/// The group `move_player(701, 3, 6)` makes of `told_apart`'s players:
/// place 04's record lands on 07, those of 05 to 07 shift one up.
fn drag_04_to_07() -> (u32, Vec<MovedPlace>) {
    (
        701,
        vec![
            place(3, 70104, 70105),
            place(4, 70105, 70106),
            place(5, 70106, 70107),
            place(6, 70107, 70104),
        ],
    )
}

// A drag is one group of the places it shifted, and no player lists a
// diff: each is compared with the record he came from.
#[test]
fn a_move_is_one_group_and_no_players_diffs() {
    let mut session = told_apart("move_group");
    session.move_player(701, 3, 6);
    let changes = session.unsaved_changes().unwrap();
    assert_eq!(
        changes
            .players
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>(),
        Vec::<u32>::new(),
        "no player lists a diff"
    );
    assert!(changes.teams.is_empty(), "the roster is not written");
    assert_eq!(changes.moves, [drag_04_to_07()]);
}

// An edit after the move lists that edit alone, its old value the
// origin's; undoing the edit and the move empties the list, a redo
// brings the group back.
#[test]
fn a_moved_players_edit_is_his_one_diff_through_undo_and_redo() {
    let mut session = told_apart("move_edit_undo");
    session.move_player(701, 3, 6);
    // The dragged player, the file's 70104, now holds 70107.
    let origin_height = session.baseline_player(70104).unwrap().basic.height;
    assert_ne!(
        origin_height,
        session.baseline_player(70107).unwrap().basic.height,
        "premise: the file's heights on the two ids differ"
    );
    let mut edited = session.file().player(70107).unwrap().clone();
    assert_eq!(edited.basic.height, origin_height, "premise: he moved");
    edited.basic.height = 190;
    session.edit_player(1, edited);

    let changes = session.unsaved_changes().unwrap();
    assert_eq!(
        changes.players,
        [(
            70107,
            vec![PlayerDiff::Field {
                field: PlayerField::Height,
                old: u32::from(origin_height),
                new: 190,
            }]
        )],
        "his one Height diff, the old value his own"
    );
    assert_eq!(changes.moves, [drag_04_to_07()]);

    assert!(session.undo(), "the edit undone");
    assert!(session.undo(), "the move undone");
    let changes = session.unsaved_changes().unwrap();
    assert!(changes.players.is_empty(), "no diffs");
    assert!(changes.moves.is_empty(), "no group");

    assert!(session.redo(), "the move redone");
    let changes = session.unsaved_changes().unwrap();
    assert!(changes.players.is_empty(), "no diffs");
    assert_eq!(changes.moves, [drag_04_to_07()], "the group back");
}

// An edit made before the move stays his through it: the moved player
// lists that edit alone, and so does he once the move is undone.
#[test]
fn an_edit_before_the_move_stays_the_moved_players() {
    let mut session = told_apart("move_after_edit");
    let mut edited = session.file().player(70104).unwrap().clone();
    edited.basic.height = 190;
    session.edit_player(1, edited);
    session.move_player(701, 3, 6);
    let height = |old: u32| {
        vec![PlayerDiff::Field {
            field: PlayerField::Height,
            old,
            new: 190,
        }]
    };

    let changes = session.unsaved_changes().unwrap();
    assert_eq!(changes.players, [(70107, height(174))], "on his new id");
    assert_eq!(changes.moves, [drag_04_to_07()]);

    assert!(session.undo(), "the move undone");
    let changes = session.unsaved_changes().unwrap();
    assert_eq!(changes.players, [(70104, height(174))], "back on his own");
    assert!(changes.moves.is_empty(), "no group");
}

// Two separate swaps are two groups; a drag overlapping an earlier one
// gives the groups the records' cycles make.
#[test]
fn separate_swaps_are_separate_groups() {
    let mut session = told_apart("move_two_groups");
    session.move_player(701, 0, 1);
    session.move_player(701, 3, 6);
    let changes = session.unsaved_changes().unwrap();
    assert!(changes.players.is_empty(), "no diffs");
    assert_eq!(
        changes.moves,
        [
            (701, vec![place(0, 70101, 70102), place(1, 70102, 70101)]),
            drag_04_to_07(),
        ]
    );

    // Places 04 to 07 hold P05, P06, P07, P04; place 02's P02 dragged onto
    // 05 leaves places 02 to 07 holding P03, P05, P06, P02, P07, P04: two
    // cycles, 02, 03, 05 (P03, P05, P02) and 04, 06, 07 (P06, P07, P04).
    let mut session = told_apart("move_overlapping");
    session.move_player(701, 3, 6);
    session.move_player(701, 1, 4);
    let names: Vec<String> = (70102..=70107)
        .map(|id| session.file().player(id).unwrap().name.clone())
        .collect();
    assert_eq!(names, ["P03", "P05", "P06", "P02", "P07", "P04"], "premise");
    let changes = session.unsaved_changes().unwrap();
    assert!(changes.players.is_empty(), "no diffs");
    assert_eq!(
        changes.moves,
        [
            (
                701,
                vec![
                    place(1, 70102, 70103),
                    place(2, 70103, 70105),
                    place(4, 70105, 70102)
                ]
            ),
            (
                701,
                vec![
                    place(3, 70104, 70106),
                    place(5, 70106, 70107),
                    place(6, 70107, 70104)
                ]
            ),
        ]
    );
}

// A save makes the moved order the file's: no group, and a later move is
// measured against the written order, the undone first move included.
#[test]
fn a_save_makes_the_moved_order_the_files() {
    let mut session = told_apart("move_saved");
    session.move_player(701, 3, 6);
    save_in_place(&mut session);
    let changes = session.unsaved_changes().unwrap();
    assert!(changes.players.is_empty(), "no diffs");
    assert!(changes.moves.is_empty(), "no group");

    // Dragged back from 07 to 04: against the written order a move of
    // its own, not the first one undone.
    session.move_player(701, 6, 3);
    let changes = session.unsaved_changes().unwrap();
    assert!(changes.players.is_empty(), "no diffs");
    assert_eq!(
        changes.moves,
        [(
            701,
            vec![
                place(3, 70104, 70107),
                place(4, 70105, 70104),
                place(5, 70106, 70105),
                place(6, 70107, 70106)
            ]
        )]
    );

    // Both moves undone: the order before the save, which the file no
    // longer holds, is a move against it.
    assert!(session.undo());
    assert!(session.undo());
    let changes = session.unsaved_changes().unwrap();
    assert!(changes.players.is_empty(), "no diffs");
    assert_eq!(
        changes.moves,
        [(
            701,
            vec![
                place(3, 70104, 70107),
                place(4, 70105, 70104),
                place(5, 70106, 70105),
                place(6, 70107, 70106)
            ]
        )],
        "the undone move against the written order"
    );
}

#[test]
fn refuse_while_running_refuses_only_the_saves_own_version() {
    assert!(refuse_while_running(&[], PesVersion::Pes21, false).is_ok());
    let game = |version| RunningPes { version, exe: None };
    // Another version's game never writes this save.
    assert!(refuse_while_running(&[game(PesVersion::Pes15)], PesVersion::Pes21, false).is_ok());
    let running = [game(PesVersion::Pes15), game(PesVersion::Pes21)];
    let error = refuse_while_running(&running, PesVersion::Pes21, false).unwrap_err();
    assert!(error.to_string().contains("PES 2021"), "{error}");
    assert!(error.to_string().contains("--edit-mode-closed"), "{error}");
    // The member's word that Edit mode is closed lets the write through.
    assert!(refuse_while_running(&running, PesVersion::Pes21, true).is_ok());
}

#[test]
fn a_players_dot_covers_his_entry_and_his_roster_number() {
    let dir = crate::test_support::scratch("dirty");
    let path = dir.join("EDIT00000000");
    std::fs::write(&path, pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
    let mut session = Session::open(&path, Recorder::default()).unwrap();
    assert!(!session.is_dirty());
    assert!(!session.player_dirty(701, 0));

    // A player edit marks him.
    let mut edited = session.file().player(70101).unwrap().clone();
    edited.basic.height = edited.basic.height.wrapping_add(1);
    session.edit_player(1, edited);
    assert!(session.player_dirty(701, 0));
    assert!(session.is_dirty());

    // Undo clears his mark; editing his roster number marks him too.
    assert!(session.undo());
    assert!(!session.player_dirty(701, 0));
    let mut team = session.file().team(701).unwrap().clone();
    team.roster[0].number = team.roster[0].number.wrapping_add(1);
    session.edit_team(1, team);
    assert!(session.player_dirty(701, 0));

    // A roster number elsewhere marks nobody here.
    assert!(!session.player_dirty(701, 1));

    // Save makes the current model the baseline: clean again.
    save_in_place(&mut session);
    assert!(!session.is_dirty());
    assert!(!session.player_dirty(701, 0));
}

#[test]
fn the_dirty_flag_and_the_redo_stack_track_edits_undos_redos_and_save() {
    let dir = crate::test_support::scratch("dirty_flag");
    let path = dir.join("EDIT00000000");
    std::fs::write(&path, pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
    let mut session = Session::open(&path, Recorder::default()).unwrap();
    assert!(!session.can_redo(), "nothing to redo on an open save");

    let mut edited = session.file().player(70101).unwrap().clone();
    edited.basic.height = edited.basic.height.wrapping_add(1);
    session.edit_player(1, edited);
    assert!(session.is_dirty(), "an edit marks the save");
    assert!(!session.can_redo(), "a new edit clears redo");

    assert!(session.undo());
    assert!(
        !session.is_dirty(),
        "undoing back to the baseline clears it"
    );
    assert!(session.can_redo(), "the undone edit is redoable");

    assert!(session.redo());
    assert!(session.is_dirty(), "redo marks it again");
    save_in_place(&mut session);
    assert!(!session.is_dirty(), "a save clears it");
}

// The mutators keep `dirty` beside `revision`: an import that changes a
// player marks the save too (the window's unsaved-changes marker reads
// `is_dirty`, and the interchange commands are the view's write paths).
#[test]
fn an_import_marks_the_save_dirty() {
    let dir = crate::test_support::scratch("import_dirty");
    let path = dir.join("EDIT00000000");
    std::fs::write(&path, pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
    let mut session = Session::open(&path, Recorder::default()).unwrap();
    assert!(!session.is_dirty());

    use pes_savefile::interchange::team_toml::{PlayerSection, TeamSection};
    let mut document = TeamToml {
        team: TeamSection {
            id: Some(701),
            ..Default::default()
        },
        ..Default::default()
    };
    document.players.insert(
        3,
        PlayerSection {
            name: Some("NEWNAME".to_string()),
            ..Default::default()
        },
    );
    session.import_team(&document).unwrap();
    assert!(session.is_dirty(), "the import changed a player");
}

// The Cards used badge's cache key: the revision moves on every change
// to the open file (an edit pushed, an undo, a redo) and not on a
// frame's seal.
#[test]
fn the_revision_moves_on_each_change_and_stays_put_on_a_frame() {
    let dir = crate::test_support::scratch("revision");
    let path = dir.join("EDIT00000000");
    std::fs::write(&path, pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
    let mut session = Session::open(&path, Recorder::default()).unwrap();
    let opened = session.revision();

    // The per-frame seal is not a change.
    session.seal();
    assert_eq!(session.revision(), opened, "a frame's seal moves nothing");

    let mut edited = session.file().player(70101).unwrap().clone();
    edited.basic.height = edited.basic.height.wrapping_add(1);
    session.edit_player(1, edited);
    let after_edit = session.revision();
    assert_ne!(after_edit, opened, "an edit takes the next tick");
    assert!(session.undo());
    let after_undo = session.revision();
    assert_ne!(after_undo, after_edit, "an undo takes the next tick");
    assert!(session.redo());
    assert_ne!(session.revision(), after_undo, "a redo takes the next tick");
}

// `edit_players` is the batch write the team operations go through: the
// changed entries land as one sealed undo entry (an equal entry, and an
// id the save lacks, are skipped), and the gesture after it — under the
// same batch key or a widget's — is its own entry.
#[test]
fn a_batch_edit_is_one_sealed_entry_of_the_changed_players() {
    let mut session = open(PesVersion::Pes21, "edit_players");
    let mut a = session.file().player(70101).unwrap().clone();
    a.basic.height = a.basic.height.wrapping_add(1);
    let unchanged = session.file().player(70102).unwrap().clone();
    let mut missing = session.file().player(70103).unwrap().clone();
    missing.id = 99999;
    let before = a.basic.height - 1;

    session.edit_players(vec![a.clone(), unchanged, missing]);

    assert_eq!(
        session.file().player(70101).unwrap().basic.height,
        before + 1
    );
    // One undo entry restores the one change; nothing else was pushed.
    assert!(session.undo());
    assert_eq!(session.file().player(70101).unwrap().basic.height, before);
    assert!(!session.can_undo(), "the batch was one entry");
    assert!(session.redo());

    // A widget's edit under another key is a new entry, not a merge into
    // the batch's.
    let mut b = session.file().player(70102).unwrap().clone();
    b.basic.height = b.basic.height.wrapping_add(1);
    let b_before = b.basic.height - 1;
    session.edit_player(1, b);
    assert!(session.undo());
    assert_eq!(session.file().player(70102).unwrap().basic.height, b_before);
    assert!(session.can_undo(), "the batch's entry is still there");

    // Back-to-back batches are their own entries: the first's seal keeps
    // the second from merging into it.
    let mut c = session.file().player(70101).unwrap().clone();
    c.basic.height = before + 2;
    session.edit_players(vec![c.clone()]);
    c.basic.height = before + 3;
    session.edit_players(vec![c]);
    assert_eq!(
        session.file().player(70101).unwrap().basic.height,
        before + 3
    );
    assert!(session.undo());
    assert_eq!(
        session.file().player(70101).unwrap().basic.height,
        before + 2,
        "the second batch is its own entry"
    );
    session.undo();
    assert_eq!(
        session.file().player(70101).unwrap().basic.height,
        before + 1
    );
    session.undo();
    assert_eq!(session.file().player(70101).unwrap().basic.height, before);
    assert!(!session.can_undo(), "each push was its own entry");
}
