//! The session's journal lines: what each mutating call records through
//! `Session::changed` (`edit_*` field lines, `undo`/`redo`'s reversed
//! lines), the `open` refusal's step, and the writes a member picks
//! outside `Session::write`. A step's error or reason is formatted
//! `{:#}`, so the step carries the whole chain the dialog shows; for a
//! plain `io::Error` or a string the alternate form is the plain one.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use anyhow::anyhow;
use pes_savefile::model::{PlayerEntry, TeamEntry};
use pes_savefile::ops::compare::{PlayerDiff, compare_players, compare_teams};
use pes_savefile::schema::VersionSchema;
use serde::{Deserialize, Serialize};
use studio_core::Recorder;

use crate::messages::{TOOL_ID, diff_fields, team_diff_fields};
use crate::session::undo::Change;

/// A step's typed change, written beside its line as `journal.toml`'s
/// `[step.op]` table: what a replay applies where the line is only read.
/// Internally tagged (`kind`); the fields serialize as the table's own
/// keys.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Op {
    /// The session's own open: `input` is the kept input's index in the
    /// bundle's `[[input]]` list.
    Open {
        /// The kept input the save was read from.
        input: usize,
    },
    /// One comparator row of a player edit, the entry's id beside it.
    Player {
        /// The player the row applies to.
        id: u32,
        /// The comparator row the line was printed from.
        row: PlayerDiff,
    },
    /// A save write: `output` is the kept output's index in the bundle's
    /// `[[output]]` list, `None` when the written file could not be read
    /// back to keep.
    Wrote {
        /// The kept output the member has.
        output: Option<usize>,
    },
}

/// A journal step for `Session::changed`: its line and, when the change
/// is one replay applies, the `Op` the line was read off.
pub(super) struct Step {
    /// The step's line.
    pub(super) line: String,
    /// The step's typed change; `None` on a heading or a line the ops do
    /// not cover (team, roster and tactics rows are not replayed).
    pub(super) op: Option<Op>,
}

impl Step {
    /// A step replay skips: a heading, or a line the ops do not cover.
    pub(super) fn note(line: impl Into<String>) -> Step {
        Step {
            line: line.into(),
            op: None,
        }
    }
}

/// `op` as the `[step.op]` table `journal.toml` writes beside the line.
/// Every variant's fields are serde data, so the table always comes out.
pub(super) fn op_table(op: &Op) -> toml::Table {
    toml::Table::try_from(op).expect("every Op variant's fields are serde data")
}

/// The journal's `player <id>: <field> <old> → <new>` steps for a change
/// to `after`'s entry from `before`, one per field `compare_players`
/// names, each carrying the `PlayerDiff` the line was printed from.
pub(super) fn player_lines(
    before: &PlayerEntry,
    after: &PlayerEntry,
    schema: &VersionSchema,
) -> Vec<Step> {
    compare_players(before, after, schema)
        .expect("a session's entries match its version's schema")
        .iter()
        .map(|diff| {
            let (field, old, new) = diff_fields(diff);
            Step {
                line: format!("player {}: {field} {old} → {new}", after.id),
                op: Some(Op::Player {
                    id: after.id,
                    row: diff.clone(),
                }),
            }
        })
        .collect()
}

/// `player_lines` for a team's entry: the ops do not cover team, roster
/// or tactics rows yet, so the lines record as notes.
pub(super) fn team_lines(
    before: &TeamEntry,
    after: &TeamEntry,
    schema: &VersionSchema,
) -> Vec<Step> {
    compare_teams(before, after, schema)
        .expect("a session's entries match its version's schema")
        .iter()
        .map(|diff| {
            let (field, old, new) = team_diff_fields(diff);
            Step::note(format!("team {}: {field} {old} → {new}", after.id))
        })
        .collect()
}

/// A pushed change's journal steps: the fields' values as they move
/// (`undone`: an undo's lines run from the edit's `after` back to its
/// `before`).
pub(super) fn change_lines(change: &Change, schema: &VersionSchema, undone: bool) -> Vec<Step> {
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
        // The move's players' own changes, in the same entry, carry its lines.
        Change::Move { .. } => Vec::new(),
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
        format!("could not open {}: {error:#}", path.display()),
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
        format!("could not read {}: {error:#}", path.display()),
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
    recorder.step(TOOL_ID, format!("refused {}: {reason:#}", path.display()));
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
        format!("could not remove {}: {error:#}", path.display()),
    );
}

/// `read_failed` for a write the member asked for that fails: `could not
/// write <path>: <error>`; nothing is kept — a failed write's bytes are
/// not an output.
pub(crate) fn write_failed(path: &Path, recorder: &Recorder, error: impl fmt::Display) {
    recorder.step(
        TOOL_ID,
        format!("could not write {}: {error:#}", path.display()),
    );
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
