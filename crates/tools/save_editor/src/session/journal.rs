//! The session's journal lines: what each mutating call records through
//! `Session::changed` (`edit_*` field lines, `undo`/`redo`'s reversed
//! lines), the `open` refusal's step, and the writes a member picks
//! outside `Session::write`.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use anyhow::anyhow;
use pes_savefile::model::{PlayerEntry, TeamEntry};
use pes_savefile::ops::compare::{compare_players, compare_teams};
use pes_savefile::schema::VersionSchema;
use studio_core::Recorder;

use crate::messages::{TOOL_ID, diff_fields, team_diff_fields};
use crate::session::undo::Change;

/// The journal's `player <id>: <field> <old> → <new>` lines for a change
/// to `after`'s entry from `before`, one per field `compare_players`
/// names.
pub(super) fn player_lines(
    before: &PlayerEntry,
    after: &PlayerEntry,
    schema: &VersionSchema,
) -> Vec<String> {
    compare_players(before, after, schema)
        .expect("a session's entries match its version's schema")
        .iter()
        .map(|diff| {
            let (field, old, new) = diff_fields(diff);
            format!("player {}: {field} {old} → {new}", after.id)
        })
        .collect()
}

/// `player_lines` for a team's entry.
pub(super) fn team_lines(
    before: &TeamEntry,
    after: &TeamEntry,
    schema: &VersionSchema,
) -> Vec<String> {
    compare_teams(before, after, schema)
        .expect("a session's entries match its version's schema")
        .iter()
        .map(|diff| {
            let (field, old, new) = team_diff_fields(diff);
            format!("team {}: {field} {old} → {new}", after.id)
        })
        .collect()
}

/// A pushed change's journal lines: the fields' values as they move
/// (`undone`: an undo's lines run from the edit's `after` back to its
/// `before`).
pub(super) fn change_lines(change: &Change, schema: &VersionSchema, undone: bool) -> Vec<String> {
    match change {
        Change::Player { before, after } => {
            let (from, to) = if undone {
                (after, before)
            } else {
                (before, after)
            };
            player_lines(from, to, schema)
        }
        Change::Team { before, after } => {
            let (from, to) = if undone {
                (after, before)
            } else {
                (before, after)
            };
            team_lines(from, to, schema)
        }
    }
}

/// The `open` refusal recorded — `could not open <path>: <the error's
/// cause>` — and `bytes` kept as an input, when the read reached them (a
/// file that could not be read keeps nothing): the file a member reports
/// is the one that failed. The error is contexted after the step so the
/// line does not repeat it; returned for `open` to hand back.
pub(super) fn open_failed(
    path: &Path,
    recorder: &Recorder,
    error: anyhow::Error,
    bytes: Option<Arc<[u8]>>,
) -> anyhow::Error {
    recorder.step(
        TOOL_ID,
        format!("could not open {}: {error}", path.display()),
    );
    if let Some(bytes) = bytes {
        recorder.keep_input(TOOL_ID, path, bytes);
    }
    error.context(format!("{}: cannot open the save", path.display()))
}

/// `open_failed` for a file the member picked to apply or open in a panel
/// (a team file, a patch): `could not read <path>: <error>`, and `bytes`
/// kept as an input when the read reached them (a file that could not be
/// read keeps nothing).
pub(crate) fn read_failed(
    path: &Path,
    recorder: &Recorder,
    error: impl fmt::Display,
    bytes: Option<Arc<[u8]>>,
) {
    recorder.step(
        TOOL_ID,
        format!("could not read {}: {error}", path.display()),
    );
    if let Some(bytes) = bytes {
        recorder.keep_input(TOOL_ID, path, bytes);
    }
}

/// The `picked <path>` step for a file the member picked that reads, its
/// `bytes` kept as an input: what the tool started from is the part a
/// saved-over file would destroy.
pub(crate) fn picked(path: &Path, recorder: &Recorder, bytes: Arc<[u8]>) {
    recorder.step(TOOL_ID, format!("picked {}", path.display()));
    recorder.keep_input(TOOL_ID, path, bytes);
}

/// `read_failed` for a file the member picked that a panel refuses after
/// a clean read (a staged import, a patch's preview, the comparator's
/// version mismatch): `refused <path>: <reason>` — recorded once by each
/// site's own gate (`refusal_recorded`, or the one arm it runs through).
pub(crate) fn refused(path: &Path, recorder: &Recorder, reason: impl fmt::Display) {
    recorder.step(TOOL_ID, format!("refused {}: {reason}", path.display()));
}

/// The `imported <picked> as <copy>` step for a file a member's action
/// copied into the app's own folders (the ruleset import's data-folder
/// copy): a step, not a kept file — the app's own copies stay out of the
/// bundle, and the pick is already its input.
pub(crate) fn imported(picked: &Path, copy: &Path, recorder: &Recorder) {
    recorder.step(
        TOOL_ID,
        format!("imported {} as {}", picked.display(), copy.display()),
    );
}

/// The `removed <path>` step for a file a member's action deleted: an
/// import's replaced copies, the ×'s own copy.
pub(crate) fn removed(path: &Path, recorder: &Recorder) {
    recorder.step(TOOL_ID, format!("removed {}", path.display()));
}

/// `write_failed` for a deletion the member asked for that fails:
/// `could not remove <path>: <error>`; nothing was a kept output either
/// way — a delete keeps nothing.
pub(crate) fn remove_failed(path: &Path, recorder: &Recorder, error: impl fmt::Display) {
    recorder.step(
        TOOL_ID,
        format!("could not remove {}: {error}", path.display()),
    );
}

/// `read_failed` for a write the member asked for that fails: `could not
/// write <path>: <error>`; nothing is kept — a failed write's bytes are
/// not an output.
pub(crate) fn write_failed(path: &Path, recorder: &Recorder, error: impl fmt::Display) {
    recorder.step(
        TOOL_ID,
        format!("could not write {}: {error}", path.display()),
    );
}

/// `error` handed back after `write_failed` records it, for a write the
/// member asked for that never reaches `write_and_record` (its
/// destination refused, its text failing to build): the write is a step
/// whether or not a byte landed.
pub(crate) fn write_refused(
    path: &Path,
    recorder: &Recorder,
    error: impl Into<anyhow::Error>,
) -> anyhow::Error {
    let error = error.into();
    write_failed(path, recorder, &error);
    error
}

/// `fs::write` to `path` plus the journal's `wrote <path>` step and the
/// bytes kept as an output: a file the member saved, which a bundle
/// holds. A failed write records `could not write <path>` and keeps
/// nothing.
pub(crate) fn write_and_record(
    recorder: &Recorder,
    path: &Path,
    bytes: &[u8],
) -> anyhow::Result<()> {
    if let Err(io) = std::fs::write(path, bytes) {
        write_failed(path, recorder, &io);
        return Err(anyhow!("{}: cannot write the file ({io})", path.display()));
    }
    recorder.step(TOOL_ID, format!("wrote {}", path.display()));
    recorder.keep_output(TOOL_ID, path, bytes.into());
    Ok(())
}
