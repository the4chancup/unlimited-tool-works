//! The repro journal's tests: every model change is a recorded step, the
//! open and write keep their files, an import writes only the rostered
//! players.

use super::*;
use crate::test_support::{journal_lines, zip_entry};

/// `op`'s effect on `session` moved `revision` and the journal's step
/// count together: both grew when `changed` holds, neither otherwise —
/// a model change that moves the revision without a step, or a step
/// with no model change, fails.
fn check_records(
    session: &mut Session,
    recorder: &Recorder,
    dir: &Path,
    name: &str,
    changed: bool,
    op: impl FnOnce(&mut Session),
) {
    let (revision, steps) = (session.revision(), journal_lines(recorder, dir).len());
    op(session);
    assert_eq!(
        session.revision() != revision,
        changed,
        "{name}: the revision"
    );
    assert_eq!(
        journal_lines(recorder, dir).len() > steps,
        changed,
        "{name}: the journal"
    );
}

// Every model change is a recorded step: the operations below move
// `revision` and the journal's step count together, and a no-change
// edit moves neither (a change cannot reach the model unrecorded).
#[test]
fn every_model_change_records_its_steps() {
    let dir = crate::test_support::scratch("journal_changes");
    let path = dir.join("EDIT00000000");
    let original = pes_savefile::fixture_save(PesVersion::Pes21);
    std::fs::write(&path, &original).unwrap();
    let recorder = Recorder::default();
    let mut session = Session::open(&path, recorder.clone()).unwrap();
    assert_eq!(
        journal_lines(&recorder, &dir),
        [format!(
            "opened {} (PES 2021, {} bytes)",
            path.display(),
            original.len()
        )],
        "the open is the journal's first step"
    );

    let height_before = session.file().player(70101).unwrap().basic.height;
    check_records(
        &mut session,
        &recorder,
        &dir,
        "edit_player",
        true,
        |session| {
            let mut edited = session.file().player(70101).unwrap().clone();
            edited.basic.height = edited.basic.height.wrapping_add(1);
            session.edit_player(1, edited);
        },
    );
    // A field's line names the player, the field and the two values.
    assert_eq!(
        journal_lines(&recorder, &dir).last().unwrap(),
        &format!(
            "player 70101: Height {} → {}",
            height_before,
            height_before + 1
        )
    );

    let batch_heights: Vec<u8> = [70101, 70102]
        .iter()
        .map(|id| session.file().player(*id).unwrap().basic.height)
        .collect();
    check_records(
        &mut session,
        &recorder,
        &dir,
        "edit_players",
        true,
        |session| {
            let mut edited = Vec::new();
            for id in [70101, 70102] {
                let mut player = session.file().player(id).unwrap().clone();
                player.basic.height = player.basic.height.wrapping_add(1);
                edited.push(player);
            }
            session.edit_players(edited);
        },
    );
    // The batch lands one line per changed player.
    let lines = journal_lines(&recorder, &dir);
    for (id, height) in [70101u32, 70102].iter().zip(&batch_heights) {
        assert!(
            lines.contains(&format!("player {id}: Height {} → {}", height, height + 1)),
            "{lines:?}"
        );
    }

    let short_before = session.file().team(701).unwrap().short_name.clone();
    check_records(
        &mut session,
        &recorder,
        &dir,
        "edit_team",
        true,
        |session| {
            let mut edited = session.file().team(701).unwrap().clone();
            edited.short_name = "XYZ".to_owned();
            session.edit_team(1, edited);
        },
    );
    assert!(
        journal_lines(&recorder, &dir)
            .contains(&format!("team 701: ShortName {short_before} → XYZ")),
        "the team's line"
    );

    // `edit_entries`' team half lands a `team …` line per changed team —
    // every numeric arm's own Debug spelling too — and undo/redo run a
    // team's change the way they run a player's.
    let flag_before = session.file().team(701).unwrap().edit_flags.name;
    let number_before = session.file().team(701).unwrap().roster[0].number;
    let support_before = session.file().team(701).unwrap().tactics.presets[0]
        .sliders
        .support_range;
    check_records(
        &mut session,
        &recorder,
        &dir,
        "edit_entries with a team",
        true,
        |session| {
            let mut edited = session.file().team(701).unwrap().clone();
            edited.short_name = "ABC".to_owned();
            edited.edit_flags.name = !edited.edit_flags.name;
            edited.roster[0].number = edited.roster[0].number.wrapping_add(1);
            edited.tactics.presets[0].sliders.support_range = edited.tactics.presets[0]
                .sliders
                .support_range
                .wrapping_add(1);
            session.edit_entries(Vec::new(), vec![edited]);
        },
    );
    let lines = journal_lines(&recorder, &dir);
    assert!(lines.contains(&"team 701: ShortName XYZ → ABC".to_owned()));
    assert!(
        lines.contains(&format!(
            "team 701: EditedName {} → {}",
            u32::from(flag_before),
            u32::from(!flag_before)
        )),
        "a record field's line: {lines:?}"
    );
    assert!(
        lines.contains(&format!(
            "team 701: Number(0) {} → {}",
            number_before,
            number_before + 1
        )),
        "a roster field's line: {lines:?}"
    );
    assert!(
        lines.contains(&format!(
            "team 701: Preset {{ preset: 0, field: SupportRange }} {} → {}",
            support_before,
            support_before + 1
        )),
        "a tactics field's line: {lines:?}"
    );
    check_records(&mut session, &recorder, &dir, "undo", true, |session| {
        session.undo();
    });
    let lines = journal_lines(&recorder, &dir);
    assert!(lines.contains(&"undo".to_owned()), "the undo header");
    assert!(
        lines.contains(&"team 701: ShortName ABC → XYZ".to_owned()),
        "the undone team field backwards: {lines:?}"
    );
    assert!(
        lines.contains(&format!(
            "team 701: Number(0) {} → {}",
            number_before + 1,
            number_before
        )),
        "the undone roster field backwards: {lines:?}"
    );
    // Only the redo's own appended lines: the journal already holds the
    // field's line both ways, so a redo recorded backwards would pass a
    // check over the whole list.
    let before = journal_lines(&recorder, &dir).len();
    check_records(&mut session, &recorder, &dir, "redo", true, |session| {
        session.redo();
    });
    let lines = journal_lines(&recorder, &dir);
    let appended = &lines[before..];
    assert!(appended.contains(&"redo".to_owned()), "the redo header");
    assert!(
        appended.contains(&"team 701: ShortName XYZ → ABC".to_owned()),
        "the team field forward again: {lines:?}"
    );
    assert!(
        !appended.contains(&"team 701: ShortName ABC → XYZ".to_owned()),
        "nothing backward in the redo: {lines:?}"
    );

    // A name the move's undo lines can hang on: the fixture's
    // placeholder records are identical, so 70101's is marked first
    // (direct writes, not model changes — the move's own lines show
    // the trade).
    session.file.player_mut(70101).unwrap().name = "DISTINCT".to_owned();
    let name_02 = session.file().player(70102).unwrap().name.clone();
    check_records(
        &mut session,
        &recorder,
        &dir,
        "move_player",
        true,
        |session| session.move_player(701, 1, 0),
    );
    check_records(&mut session, &recorder, &dir, "undo", true, |session| {
        session.undo();
    });
    // The undo's lines run the move backwards: `after → before`.
    let lines = journal_lines(&recorder, &dir);
    assert!(
        lines.contains(&format!("player 70101: Name {name_02} → DISTINCT")),
        "the undone trade read as it happened: {lines:?}"
    );
    // Same shape as the team's redo above: the appended lines only.
    let before = journal_lines(&recorder, &dir).len();
    check_records(&mut session, &recorder, &dir, "redo", true, |session| {
        session.redo();
    });
    let lines = journal_lines(&recorder, &dir);
    let appended = &lines[before..];
    assert!(
        appended.contains(&format!("player 70101: Name DISTINCT → {name_02}")),
        "the redone trade: {lines:?}"
    );
    assert!(
        !appended.contains(&format!("player 70101: Name {name_02} → DISTINCT")),
        "nothing backward in the redo: {lines:?}"
    );
    check_records(
        &mut session,
        &recorder,
        &dir,
        "import_team",
        true,
        |session| {
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
        },
    );
    let patch = |key: u8| {
        use std::collections::BTreeMap;

        use pes_savefile::interchange::aesthetics_patch::{
            ALLOCATION_SCHEME_VERSION, AestheticsPatch, PatchPlayer, PatchTeam,
        };
        AestheticsPatch {
            pes_version: PesVersion::Pes21,
            tool_version: "0.1.0".to_owned(),
            allocation_scheme_version: ALLOCATION_SCHEME_VERSION,
            cpks: vec![],
            compiled: "2026-10-13T00:00:00Z".parse().unwrap(),
            teams: vec![PatchTeam {
                id: 701,
                name: "PATCHED".to_owned(),
                players: BTreeMap::from([(
                    key,
                    PatchPlayer {
                        name: Some("PATCHED".to_owned()),
                        shirt_name: None,
                        boots_id: None,
                        gloves_id: None,
                        base_copy_id: None,
                        appearance: None,
                    },
                )]),
            }],
        }
    };
    check_records(
        &mut session,
        &recorder,
        &dir,
        "apply_patch",
        true,
        |session| {
            // Key 1 is the patch's first player: team 701's 70101.
            session.apply_patch(&patch(1)).unwrap();
        },
    );
    let lines = journal_lines(&recorder, &dir);
    assert!(lines.contains(&"applied the aesthetics patch".to_owned()));
    assert!(
        lines.contains(&"applied to team 701".to_owned()),
        "the team a player was written to: {lines:?}"
    );

    // A patch every player of which the save lacks writes nothing:
    // no step, no revision (key 0 is 70100, no such player).
    check_records(
        &mut session,
        &recorder,
        &dir,
        "a patch writing nothing",
        false,
        |session| {
            let summary = session.apply_patch(&patch(0)).unwrap();
            assert!(summary.applied_players.is_empty(), "premise");
        },
    );

    // An edit of nothing moves neither the revision nor the journal.
    check_records(
        &mut session,
        &recorder,
        &dir,
        "a no-change edit",
        false,
        |session| {
            session.edit_player(1, session.file().player(70101).unwrap().clone());
        },
    );
}

// The bundle's input is the file as read and its output the file as
// written: `open` keeps the exact bytes, `write` the written ones.
#[test]
fn open_and_write_keep_their_bytes_in_the_bundle() {
    let dir = crate::test_support::scratch("journal_kept");
    let path = dir.join("EDIT00000000");
    let original = pes_savefile::fixture_save(PesVersion::Pes21);
    std::fs::write(&path, &original).unwrap();
    let recorder = Recorder::default();
    let mut session = Session::open(&path, recorder.clone()).unwrap();

    let mut edited = session.file().player(70101).unwrap().clone();
    edited.basic.height = edited.basic.height.wrapping_add(1);
    session.edit_player(1, edited);
    let out = dir.join("EDIT00000000.out");
    session.write(&out).unwrap();

    let bundle = dir.join("bundle.zip");
    recorder.write_bundle(&bundle, "", &[], &[], None).unwrap();
    assert!(
        journal_lines(&recorder, &dir).contains(&format!("wrote {}", out.display())),
        "the write is a step"
    );
    assert_eq!(
        zip_entry(&bundle, "inputs/0-EDIT00000000"),
        original,
        "the kept input is the file as read"
    );
    assert_eq!(
        zip_entry(&bundle, "outputs/0-EDIT00000000.out"),
        std::fs::read(&out).unwrap(),
        "the kept output is the file as written"
    );
}

// A document's `[players.NN]` names a roster slot, resolved against the
// roster as opened, and the import never writes the roster itself
// (`team_toml`'s `apply`): a field of a player the roster does not name
// cannot change, so the journal holds no line for him. An entry on an
// empty slot is noted and writes nothing.
#[test]
fn an_import_writes_only_the_players_the_roster_names() {
    let dir = crate::test_support::scratch("journal_import_roster");
    let path = dir.join("EDIT00000000");
    std::fs::write(&path, pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
    let recorder = Recorder::default();
    let mut session = Session::open(&path, recorder.clone()).unwrap();
    let empty = session
        .file()
        .team(701)
        .unwrap()
        .roster
        .iter()
        .position(|slot| slot.player_id == 0)
        .map(|i| u8::try_from(i).unwrap())
        .expect("the roster has an empty slot");
    let name_before = session.file().player(70103).unwrap().name.clone();

    use pes_savefile::interchange::team_toml::{PlayerSection, TeamSection};
    let mut document = TeamToml {
        team: TeamSection {
            id: Some(701),
            ..Default::default()
        },
        ..Default::default()
    };
    // `[players.03]` is 70103's roster slot; `[players.NN]` on the
    // empty slot has no player to write to.
    document.players.insert(
        3,
        PlayerSection {
            name: Some("NEWNAME".to_owned()),
            ..Default::default()
        },
    );
    document.players.insert(
        empty + 1,
        PlayerSection {
            name: Some("NOBODY".to_owned()),
            number: Some(9),
            ..Default::default()
        },
    );
    let notes = session.import_team(&document).unwrap();

    assert!(
        notes
            .iter()
            .any(|note| matches!(note, ImportNote::EmptySlot { .. })),
        "the empty slot's entry is noted skipped: {notes:?}"
    );
    let lines = journal_lines(&recorder, &dir);
    assert!(
        lines.contains(&"imported Team TOML onto team 701".to_owned()),
        "the import's header: {lines:?}"
    );
    let lines = journal_lines(&recorder, &dir);
    let players: Vec<&String> = lines
        .iter()
        .filter(|line| line.starts_with("player "))
        .collect();
    assert_eq!(
        players,
        [&format!("player 70103: Name {name_before} → NEWNAME")],
        "only the rostered player's line: {lines:?}"
    );
    assert_eq!(
        session.file().player(70103).unwrap().name,
        "NEWNAME",
        "the rostered player imported"
    );
}

// A write the member asked for that fails records `could not write
// <path>` (the error as the window shows it) and keeps no output.
#[test]
fn a_failed_write_records_the_step_and_keeps_nothing() {
    let dir = crate::test_support::scratch("journal_write_fails");
    let path = dir.join("EDIT00000000");
    std::fs::write(&path, pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
    let recorder = Recorder::default();
    let session = Session::open(&path, recorder.clone()).unwrap();
    let missing = dir.join("missing").join("EDIT00000000");

    let error = session.write(&missing).unwrap_err();
    assert!(
        format!("{error:#}").contains("cannot write the save"),
        "premise: the write failed: {error:#}"
    );

    let lines = journal_lines(&recorder, &dir);
    let failed = lines
        .iter()
        .find(|line| line.starts_with("could not write "))
        .expect("the failed write records its own step");
    assert!(failed.contains("EDIT00000000"), "{failed}");
    let bundle = dir.join("journal.zip");
    let doc: toml::Table =
        toml::from_str(&String::from_utf8(zip_entry(&bundle, "journal.toml")).unwrap()).unwrap();
    assert!(
        doc.get("output")
            .is_none_or(|outputs| outputs.as_array().unwrap().is_empty()),
        "no output kept: {doc:?}"
    );
}
