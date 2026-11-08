//! Replaying a repro bundle's journal: the `op` beside each
//! model-changing step applied to the kept input in order, each kept
//! output compared byte for byte under its own salt. A match puts the
//! member's problem in what the editor writes or in the game; a
//! `Stale` step or a `Differs` write puts it in a change the journal
//! never saw or reads differently.

use pes_savefile::container::file_salt;
use pes_savefile::file::EditFile;
use pes_savefile::ops::apply::{ApplyError, apply_player};
use pes_savefile::ops::compare::{PlayerDiff, compare};
use studio_core::repro::{Bundle, BundleFile, BundleStep};

use crate::messages::TOOL_ID;
use crate::session::journal::Op;

/// What replaying found, one row per reportable step (and the version
/// banner first).
#[derive(Debug, PartialEq)]
pub(crate) enum Outcome {
    /// The bundle was recorded by another Studio version; replay ran
    /// anyway.
    Version {
        /// The `version` line the bundle was written with.
        bundle: String,
        /// This build's.
        studio: String,
    },
    /// The replayed save's bytes equal the kept output's.
    Matches {
        /// The wrote step's 1-based position in the journal.
        step: usize,
        /// Its line.
        line: String,
    },
    /// A later write to the same kept output replaced this write's
    /// bytes; the member has the later one, so this one is not
    /// compared.
    Replaced {
        /// The earlier wrote step's position.
        step: usize,
        /// Its line.
        line: String,
    },
    /// The written file was not kept (the journal kept no output for
    /// the write).
    NotKept {
        /// The wrote step's position.
        step: usize,
        /// Its line.
        line: String,
    },
    /// The replayed save's bytes differ from the kept output's.
    Differs {
        /// The wrote step's position.
        step: usize,
        /// Its line.
        line: String,
        /// The `compare_players` rows from the replayed save to the
        /// written one, per id in both saves with any.
        players: Vec<(u32, Vec<PlayerDiff>)>,
        /// Ids the replayed save alone has.
        only_replayed: Vec<u32>,
        /// Ids the written save alone has.
        only_written: Vec<u32>,
        /// The first byte the two files differ at, when `players` is
        /// empty.
        first_byte: Option<usize>,
    },
    /// A step's old value is not what the replayed entry holds: the
    /// walk stops there, at the unrecorded model change.
    Stale {
        /// The stale step's position.
        step: usize,
        /// Its line.
        line: String,
        /// What the entry holds, spelled as the line spells the value.
        holds: String,
    },
}

/// A bundle the replay cannot walk: the journal step that stopped it
/// and why.
#[derive(Debug, thiserror::Error)]
#[error("step {step} ({line}): {reason}")]
pub(crate) struct ReplayError {
    /// The step's 1-based position in the journal.
    pub(crate) step: usize,
    /// Its line.
    pub(crate) line: String,
    /// Why the walk could not go on.
    pub(crate) reason: String,
}

impl ReplayError {
    /// The error at journal step `at` (its 1-based position), with
    /// `step`'s line.
    fn at(at: usize, step: &BundleStep, reason: impl Into<String>) -> ReplayError {
        ReplayError {
            step: at,
            line: step.what.clone(),
            reason: reason.into(),
        }
    }
}

/// `list[index]` or an error naming the step: an op's index past the
/// bundle's list.
fn kept_file<'a>(
    list: &'a [BundleFile],
    index: usize,
    side: &'static str,
    at: usize,
    step: &BundleStep,
) -> Result<&'a BundleFile, ReplayError> {
    list.get(index).ok_or_else(|| {
        ReplayError::at(
            at,
            step,
            format!(
                "the op names {side} {index}, the bundle holds {}",
                list.len()
            ),
        )
    })
}

/// Whether a `wrote` after `at` names the same kept output `output`:
/// the last write to one path is the only one the member has. `ops` is
/// the walk's parsed ops.
fn replaced(ops: &[Option<Op>], at: usize, output: usize) -> bool {
    ops[at + 1..]
        .iter()
        .flatten()
        .any(|op| matches!(op, Op::Wrote { output: Some(later) } if *later == output))
}

/// The `wrote` compare: the replayed `file` re-encrypted under the kept
/// output's own salt and compared byte for byte (every write salts
/// afresh, so no other salt could reproduce it). A mismatch lists the
/// comparator's rows, or the first differing byte when every row
/// matches.
fn compare_write(
    bundle: &Bundle,
    file: &EditFile,
    output: usize,
    at: usize,
    step: &BundleStep,
) -> Result<Outcome, ReplayError> {
    let kept = kept_file(&bundle.outputs, output, "output", at, step)?;
    let not_a_save = |error: &dyn std::fmt::Display| {
        ReplayError::at(
            at,
            step,
            format!("the kept output {output} does not read as a save: {error}"),
        )
    };
    let salt = file_salt(&kept.bytes).map_err(|error| not_a_save(&error))?;
    let replayed = file
        .to_bytes(&salt)
        .map_err(|error| ReplayError::at(at, step, error.to_string()))?;
    if replayed == kept.bytes {
        return Ok(Outcome::Matches {
            step: at,
            line: step.what.clone(),
        });
    }
    let written = EditFile::from_bytes(&kept.bytes).map_err(|error| not_a_save(&error))?;
    let diff =
        compare(file, &written).map_err(|error| ReplayError::at(at, step, error.to_string()))?;
    let first_byte = diff.players.is_empty().then(|| {
        replayed
            .iter()
            .zip(&kept.bytes)
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| replayed.len().min(kept.bytes.len()))
    });
    Ok(Outcome::Differs {
        step: at,
        line: step.what.clone(),
        players: diff.players,
        only_replayed: diff.only_in_a,
        only_written: diff.only_in_b,
        first_byte,
    })
}

/// Replays `bundle`'s journal: the `save-editor` steps' ops applied to
/// the kept input in journal order (op-less steps are notes and are
/// skipped), each kept output's write compared at its step.
pub(crate) fn replay(bundle: &Bundle) -> Result<Vec<Outcome>, ReplayError> {
    let mut outcomes = Vec::new();
    let studio = studio_core::shell::version_label();
    if bundle.version != studio {
        outcomes.push(Outcome::Version {
            bundle: bundle.version.clone(),
            studio,
        });
    }
    // Every op parsed once, before anything is applied: an op this build
    // cannot read is the error naming its step, and the `Replaced`
    // look-ahead reads the parsed list.
    let ops = bundle
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            if step.tool != TOOL_ID {
                return Ok(None);
            }
            step.op
                .clone()
                .map(|table| {
                    table.try_into().map_err(|error| {
                        ReplayError::at(index + 1, step, format!("cannot read the op: {error:#}"))
                    })
                })
                .transpose()
        })
        .collect::<Result<Vec<Option<Op>>, ReplayError>>()?;
    let mut file: Option<EditFile> = None;
    for (index, (step, op)) in bundle.steps.iter().zip(&ops).enumerate() {
        let at = index + 1;
        let Some(op) = op else { continue };
        match op {
            Op::Open { input } => {
                let kept = kept_file(&bundle.inputs, *input, "input", at, step)?;
                file = Some(EditFile::from_bytes(&kept.bytes).map_err(|error| {
                    ReplayError::at(
                        at,
                        step,
                        format!("the kept input {input} does not read as a save: {error}"),
                    )
                })?);
            }
            Op::Player { id, row } => {
                let file = file
                    .as_mut()
                    .ok_or_else(|| ReplayError::at(at, step, "no save is open yet"))?;
                let entry = file.player_mut(*id).ok_or_else(|| {
                    ReplayError::at(at, step, format!("the save has no player {id}"))
                })?;
                match apply_player(entry, row) {
                    Ok(()) => {}
                    Err(ApplyError::Stale { holds }) => {
                        outcomes.push(Outcome::Stale {
                            step: at,
                            line: step.what.clone(),
                            holds,
                        });
                        return Ok(outcomes);
                    }
                    Err(ApplyError::Codec(error)) => {
                        return Err(ReplayError::at(at, step, error.to_string()));
                    }
                }
            }
            Op::Wrote { output } => {
                let file = file
                    .as_ref()
                    .ok_or_else(|| ReplayError::at(at, step, "no save is open yet"))?;
                match output {
                    None => outcomes.push(Outcome::NotKept {
                        step: at,
                        line: step.what.clone(),
                    }),
                    Some(output) if replaced(&ops, index, *output) => {
                        outcomes.push(Outcome::Replaced {
                            step: at,
                            line: step.what.clone(),
                        });
                    }
                    Some(output) => {
                        outcomes.push(compare_write(bundle, file, *output, at, step)?);
                    }
                }
            }
        }
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use pes_savefile::container::SaveContainer;
    use pes_savefile::model::ingame_face::IngameFace;
    use pes_savefile::ops::fingerprint::face_hash;
    use pes_savefile::schema::fields::PlayerField;
    use pes_savefile::schema::ingame_face::IngameFaceField;
    use pes_version::PesVersion;
    use studio_core::Recorder;
    use studio_core::repro::read_bundle;

    use super::*;
    use crate::session::Session;
    use crate::test_support::scratch;

    /// The PES 21 fixture opened as a session under `dir`.
    fn session(dir: &Path) -> (Session, Recorder) {
        let path = dir.join("EDIT00000000");
        std::fs::write(&path, pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        let recorder = Recorder::default();
        let session = Session::open(&path, recorder.clone()).unwrap();
        (session, recorder)
    }

    /// `recorder`'s bundle written to `dir` and read back.
    fn bundle_of(recorder: &Recorder, dir: &Path) -> Bundle {
        let zip = dir.join("repro.zip");
        recorder.write_bundle(&zip, "", &[], &[], None).unwrap();
        read_bundle(&zip).unwrap()
    }

    /// `player`'s Height nudged up, pushed as one edit under `key`.
    fn edit_height(session: &mut Session, id: u32, key: u64) {
        let mut edited = session.file().player(id).unwrap().clone();
        edited.basic.height = edited.basic.height.wrapping_add(1);
        session.edit_player(key, edited);
    }

    /// `face` with every field of its layout zeroed: its measured-unused
    /// bits recomputed, `IngameFace::unused` being the lib's own.
    fn unused_bits(face: &IngameFace) -> Vec<u8> {
        let mut zeroed = face.clone();
        for field in IngameFaceField::ALL {
            if zeroed.get(field).is_ok() {
                zeroed.set(field, 0).expect("0 fits every field");
            }
        }
        zeroed.bytes().to_vec()
    }

    /// `player`'s ingame-face run's measured-unused bits flipped, pushed
    /// as one edit under `key`. `set_unused` is the lib's own door to
    /// them, so the edit goes through the row a replay applies; the row's
    /// fingerprints are the journal line's, unread on apply, and the
    /// recorded op is `compare_players`' own either way.
    fn edit_face_run(session: &mut Session, id: u32, key: u64) {
        let mut edited = session.file().player(id).unwrap().clone();
        let old_bits = unused_bits(&edited.appearance.ingame_face);
        let new_bits: Vec<u8> = old_bits.iter().map(|byte| !byte).collect();
        let old = face_hash(&edited.appearance.ingame_face);
        let row = PlayerDiff::FaceRun {
            old,
            new: old,
            old_bits,
            new_bits,
        };
        apply_player(&mut edited, &row).expect("the run's own bits apply");
        assert_ne!(
            Some(&edited),
            session.file().player(id),
            "premise: the unused bits flipped"
        );
        session.edit_player(key, edited);
    }

    /// The op of `bundle`'s step whose line holds `needle`.
    fn op_of<'a>(bundle: &'a Bundle, needle: &str) -> &'a toml::Table {
        bundle
            .steps
            .iter()
            .find(|step| step.what.contains(needle))
            .and_then(|step| step.op.as_ref())
            .unwrap_or_else(|| panic!("no op on the step holding {needle:?}"))
    }

    /// `session` saved to `out`, its folder made first.
    fn save_written(session: &mut Session, out: &Path) {
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        session.save_to(out, &[], false).unwrap();
    }

    /// `bundle`'s kept output `index` replaced by `f`'s re-encryption of
    /// it under the file's own salt: a change the journal never saw.
    fn edit_output(bundle: &mut Bundle, index: usize, f: impl Fn(&[u8], &[u8; 320]) -> Vec<u8>) {
        let bytes = &bundle.outputs[index].bytes;
        let salt = file_salt(bytes).unwrap();
        bundle.outputs[index].bytes = f(bytes, &salt);
    }

    // The journal's own steps replay to the kept write: the open's
    // input, every player op in order (the undo's backwards and the
    // redo's forwards again, a row of each of the op's four kinds), the
    // wrote's output compared and equal.
    #[test]
    fn a_recorded_session_replays_to_one_matching_write() {
        let dir = scratch("replay_matches");
        let (mut session, recorder) = session(&dir);
        edit_height(&mut session, 70101, 1);
        assert!(session.undo());
        assert!(session.redo());
        let mut edited = session.file().player(70101).unwrap().clone();
        edited.shirt_name.push('Z');
        session.edit_player(2, edited);
        let mut edited = session.file().player(70101).unwrap().clone();
        edited.name.push('Z');
        session.edit_player(3, edited);
        let mut edited = session.file().player(70101).unwrap().clone();
        let nose = edited
            .appearance
            .ingame_face
            .get(IngameFaceField::NoseType)
            .expect("covered");
        edited
            .appearance
            .ingame_face
            .set(IngameFaceField::NoseType, nose ^ 1)
            .expect("fits");
        session.edit_player(4, edited);
        edit_face_run(&mut session, 70101, 5);
        save_written(&mut session, &dir.join("written/EDIT00000000"));

        let bundle = bundle_of(&recorder, &dir);
        assert!(
            bundle.steps.iter().any(|step| {
                step.op
                    .as_ref()
                    .is_some_and(|op| op.get("row").and_then(|row| row.get("FaceRun")).is_some())
            }),
            "premise: the face-run op is in the bundle"
        );
        let outcomes = replay(&bundle).unwrap();

        assert_eq!(
            outcomes.len(),
            1,
            "the wrote is the only reported step: {outcomes:?}"
        );
        assert!(
            matches!(
                &outcomes[0],
                Outcome::Matches { step: 11, line } if line.starts_with("wrote ")
            ),
            "the wrote step's position and line: {outcomes:?}"
        );
    }

    // Two opens and three writes over two paths, the first path written
    // twice: the second open's step names kept input 1 and the second
    // path's write names kept output 1 — replay reads the files they
    // name, so the write to the second path is compared, the first
    // path's earlier write `Replaced`, its last `Matches`. The second
    // save's bytes differ so a wrong index could never compare equal.
    #[test]
    fn a_second_open_and_path_replay_by_their_kept_indexes() {
        let dir = scratch("replay_indexes");
        let (mut session, recorder) = session(&dir);
        edit_height(&mut session, 70101, 1);
        let p = dir.join("p").join("EDIT00000000");
        save_written(&mut session, &p);

        let other = dir.join("EDIT00000001");
        let mut second_save = EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21))
            .expect("the fixture reads");
        second_save.player_mut(70101).unwrap().basic.weight = second_save
            .player(70101)
            .unwrap()
            .basic
            .weight
            .wrapping_add(1);
        second_save.save(&other).expect("the second save writes");
        let mut session_b = Session::open(&other, recorder.clone()).unwrap();
        edit_height(&mut session_b, 70101, 2);
        let q = dir.join("q").join("EDIT00000000");
        save_written(&mut session_b, &q);
        edit_height(&mut session_b, 70101, 3);
        save_written(&mut session_b, &p);

        let bundle = bundle_of(&recorder, &dir);
        assert_eq!(
            op_of(&bundle, "EDIT00000001")
                .get("input")
                .and_then(|input| input.as_integer()),
            Some(1),
            "the second save's open names kept input 1"
        );
        assert_eq!(
            op_of(&bundle, &q.display().to_string())
                .get("output")
                .and_then(|output| output.as_integer()),
            Some(1),
            "the second path's write names kept output 1"
        );

        let outcomes = replay(&bundle).unwrap();

        let [first, second, third] = outcomes.as_slice() else {
            panic!("expected three reported writes, got {outcomes:?}")
        };
        assert!(
            matches!(first, Outcome::Replaced { step: 3, .. }),
            "the first path's earlier write: {first:?}"
        );
        assert!(
            matches!(second, Outcome::Matches { step: 6, .. }),
            "the second path's write: {second:?}"
        );
        assert!(
            matches!(third, Outcome::Matches { step: 8, .. }),
            "the first path's last write: {third:?}"
        );
    }

    // A write after the undo and another after the redo are each
    // compared at their own step: both ops replay, the undoed file's
    // write matching as the redone one's does.
    #[test]
    fn a_write_after_undo_and_after_redo_each_compare() {
        let dir = scratch("replay_undo_redo");
        let (mut session, recorder) = session(&dir);
        edit_height(&mut session, 70101, 1);
        save_written(&mut session, &dir.join("p/EDIT00000000"));
        assert!(session.undo());
        save_written(&mut session, &dir.join("q/EDIT00000000"));
        assert!(session.redo());
        save_written(&mut session, &dir.join("r/EDIT00000000"));

        let outcomes = replay(&bundle_of(&recorder, &dir)).unwrap();

        let [first, second, third] = outcomes.as_slice() else {
            panic!("expected three compared writes, got {outcomes:?}")
        };
        assert!(
            matches!(first, Outcome::Matches { step: 3, .. }),
            "the edit's write: {first:?}"
        );
        assert!(
            matches!(second, Outcome::Matches { step: 6, .. }),
            "the undo's write: {second:?}"
        );
        assert!(
            matches!(third, Outcome::Matches { step: 9, .. }),
            "the redo's write: {third:?}"
        );
    }

    // A change the journal never saw: the kept output's save decoded,
    // 70102's Weight nudged and the file re-encrypted under its own
    // salt. The write's compare then lists 70102's Weight row with the
    // replayed and the written value.
    #[test]
    fn an_unrecorded_change_is_the_differs_row() {
        let dir = scratch("replay_differs");
        let (mut session, recorder) = session(&dir);
        edit_height(&mut session, 70101, 1);
        let weight = session.file().player(70102).unwrap().basic.weight;
        save_written(&mut session, &dir.join("written/EDIT00000000"));
        let mut bundle = bundle_of(&recorder, &dir);

        edit_output(&mut bundle, 0, |bytes, salt| {
            let mut file = EditFile::from_bytes(bytes).unwrap();
            file.player_mut(70102).unwrap().basic.weight = weight.wrapping_add(1);
            file.to_bytes(salt).unwrap()
        });

        let outcomes = replay(&bundle).unwrap();

        let [
            Outcome::Differs {
                step: 3,
                players,
                first_byte,
                only_replayed,
                only_written,
                ..
            },
        ] = outcomes.as_slice()
        else {
            panic!("expected one Differs, got {outcomes:?}")
        };
        let rows = &players
            .iter()
            .find(|(id, _)| *id == 70102)
            .expect("70102's rows")
            .1;
        let Some(PlayerDiff::Field {
            field: PlayerField::Weight,
            old,
            new,
        }) = rows.first()
        else {
            panic!("expected the Weight row, got {rows:?}")
        };
        assert_eq!(*old, u32::from(weight), "the replayed value");
        assert_eq!(*new, u32::from(weight.wrapping_add(1)), "the written one");
        assert!(only_replayed.is_empty() && only_written.is_empty());
        assert_eq!(*first_byte, None, "the rows carry the difference");
    }

    // A byte changed outside every entry: the container's logo gains a
    // byte (the fixtures carry none). No player row differs, so `Differs`
    // falls to the first differing byte, computed here from the kept
    // output before and after the edit.
    #[test]
    fn a_byte_change_outside_the_entries_is_the_first_byte() {
        let dir = scratch("replay_first_byte");
        let (mut session, recorder) = session(&dir);
        edit_height(&mut session, 70101, 1);
        save_written(&mut session, &dir.join("written/EDIT00000000"));
        let mut bundle = bundle_of(&recorder, &dir);

        let original = bundle.outputs[0].bytes.clone();
        edit_output(&mut bundle, 0, |bytes, salt| {
            let mut container = SaveContainer::decrypt(bytes).unwrap();
            container.logo.push(0xab);
            container.to_bytes(salt).unwrap()
        });
        let expected = original
            .iter()
            .zip(&bundle.outputs[0].bytes)
            .position(|(a, b)| a != b);
        assert!(expected.is_some(), "premise: the files differ");

        let outcomes = replay(&bundle).unwrap();

        let [
            Outcome::Differs {
                players,
                only_replayed,
                only_written,
                first_byte,
                ..
            },
        ] = outcomes.as_slice()
        else {
            panic!("expected one Differs, got {outcomes:?}")
        };
        assert!(players.is_empty(), "no entry row: {players:?}");
        assert!(only_replayed.is_empty() && only_written.is_empty());
        assert_eq!(*first_byte, expected);
    }

    // A player renumbered to an id no save holds: the replayed save alone
    // has 70102, the written one alone has the new id — `only_replayed`
    // and `only_written`, not a row.
    #[test]
    fn a_player_in_one_save_alone_is_the_only_lists() {
        let dir = scratch("replay_only_lists");
        let (mut session, recorder) = session(&dir);
        edit_height(&mut session, 70101, 1);
        save_written(&mut session, &dir.join("written/EDIT00000000"));
        let mut bundle = bundle_of(&recorder, &dir);

        edit_output(&mut bundle, 0, |bytes, salt| {
            let mut file = EditFile::from_bytes(bytes).unwrap();
            file.player_mut(70102).unwrap().id = 9_999_999;
            file.to_bytes(salt).unwrap()
        });

        let outcomes = replay(&bundle).unwrap();

        let [
            Outcome::Differs {
                only_replayed,
                only_written,
                ..
            },
        ] = outcomes.as_slice()
        else {
            panic!("expected one Differs, got {outcomes:?}")
        };
        assert_eq!(only_replayed, &[70102]);
        assert_eq!(only_written, &[9_999_999]);
    }

    // The Height step's op with its `old` raised by one is `Stale` at
    // that step — `holds` is the fixture's height — and the write after
    // it is never compared.
    #[test]
    fn a_step_whose_old_value_is_not_held_stops_the_walk() {
        let dir = scratch("replay_stale");
        let (mut session, recorder) = session(&dir);
        let height = session.file().player(70101).unwrap().basic.height;
        edit_height(&mut session, 70101, 1);
        save_written(&mut session, &dir.join("written/EDIT00000000"));
        let mut bundle = bundle_of(&recorder, &dir);

        let step = bundle
            .steps
            .iter_mut()
            .find(|step| step.what.contains("Height"))
            .expect("the Height step");
        let field = step.op.as_mut().expect("a player step carries an op")["row"]["Field"]
            .as_table_mut()
            .expect("a Field row");
        let old = field["old"].as_integer().expect("an integer");
        field.insert("old".to_owned(), toml::Value::Integer(old + 1));

        let outcomes = replay(&bundle).unwrap();

        assert_eq!(
            outcomes.len(),
            1,
            "the walk stops at the stale step: {outcomes:?}"
        );
        assert!(
            matches!(
                &outcomes[0],
                Outcome::Stale { step: 2, holds, .. } if holds == &height.to_string()
            ),
            "{outcomes:?}"
        );
    }

    // Two writes to one path keep the second: the first's step is
    // `Replaced` (it is not the file the member has), the second's
    // `Matches`. A bundle another version recorded replays the same
    // with the two versions named first.
    #[test]
    fn a_second_write_to_one_path_replaces_the_first() {
        let dir = scratch("replay_replaced");
        let (mut session, recorder) = session(&dir);
        save_written(&mut session, &dir.join("written/EDIT00000000"));
        edit_height(&mut session, 70101, 1);
        save_written(&mut session, &dir.join("written/EDIT00000000"));
        let mut bundle = bundle_of(&recorder, &dir);
        bundle.version = "v9.9.9 another build".to_owned();

        let outcomes = replay(&bundle).unwrap();

        assert_eq!(outcomes.len(), 3, "{outcomes:?}");
        assert!(
            matches!(
                &outcomes[0],
                Outcome::Version { bundle, studio }
                    if bundle == "v9.9.9 another build"
                        && studio == &studio_core::shell::version_label()
            ),
            "{outcomes:?}"
        );
        assert!(
            matches!(outcomes[1], Outcome::Replaced { step: 2, .. }),
            "{outcomes:?}"
        );
        assert!(
            matches!(outcomes[2], Outcome::Matches { step: 4, .. }),
            "{outcomes:?}"
        );
    }

    // An op this build cannot read is a `ReplayError` naming its step.
    #[test]
    fn an_op_of_an_unknown_kind_is_a_replay_error_naming_the_step() {
        let dir = scratch("replay_bad_op");
        let (mut session, recorder) = session(&dir);
        edit_height(&mut session, 70101, 1);
        save_written(&mut session, &dir.join("written/EDIT00000000"));
        let mut bundle = bundle_of(&recorder, &dir);
        let bogus: toml::Table = toml::from_str("kind = \"bogus\"\nx = 1\n").unwrap();
        bundle.steps[1].op = Some(bogus);

        let error = replay(&bundle).unwrap_err();
        assert_eq!(error.step, 2, "the mutated step's position");
        assert!(
            error.reason.contains("cannot read the op"),
            "{}",
            error.reason
        );
    }
}
