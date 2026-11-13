//! The session: the save that is open, and the operations over it that the commands run (and
//! the window will, through its undo stack). egui-free, so the command line and the window
//! cannot diverge (`spec/save_editor.md` "Crate layout"). `pes_savefile` owns every byte and
//! every operation on the model; the session resolves ids against the open save and hands
//! the lib the entries it asks for.

pub(crate) mod database;
pub(crate) mod import_sections;
pub(crate) mod journal;
pub(crate) mod player_ops;
pub(crate) mod rules;
pub(crate) mod selection;
pub(crate) mod tactics_ops;
pub(crate) mod team_ops;
pub(crate) mod undo;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, anyhow};
use pes_savefile::EditFile;
use pes_savefile::interchange::aesthetics_patch::{AestheticsPatch, PatchError, PatchSummary};
use pes_savefile::interchange::team_toml::{ImportNote, TeamToml};
use pes_savefile::model::{PlayerEntry, TeamEntry};
use pes_savefile::ops::compare::{PlayerDiff, TeamDiff, compare_players, compare_teams};
use pes_savefile::schema::ingame_face::IngameFaceField;
use pes_savefile::schema::schema_for;
use pes_version::PesVersion;
use studio_core::{Recorder, RunningPes};

use crate::messages::TOOL_ID;
use crate::session::journal::{
    Op, Step, change_lines, op_table, open_failed, player_lines, team_lines, write_failed,
};
use crate::session::undo::{Change, UndoStack};

/// The ticks `Session::revision` draws from, one per open and per change:
/// a revision never repeats, so a cache keyed on it (the view's Cards used badge) is
/// never stale across saves.
static NEXT_REVISION: AtomicU64 = AtomicU64::new(1);

/// The undo merge key every sealed operation's changes go under
/// (`edit_entries`, which `edit_players`, `fix_database` and the team
/// operations call, and `move_player`): each is one sealed entry, never
/// part of a widget's open gesture; a widget's key is its `egui::Id` value,
/// never `u64::MAX`.
const SEALED_KEY: u64 = u64::MAX;

/// The fields of the in-game face run `player_export` carries (its appearance
/// section ends at `iris_col`; the face structure and the hair are not in the
/// struct): a move trades them with the record, the rest of the run stays with
/// the slot's id (`move_player`, SE-GUI-08).
const EXPORT_FACE_FIELDS: [IngameFaceField; 4] = [
    IngameFaceField::PlayerGloves,
    IngameFaceField::PlayerGlovesColor,
    IngameFaceField::SkinColor,
    IngameFaceField::IrisColor,
];

/// The differences from the baselines `is_dirty` compares against, as plain
/// data for the unsaved-changes prompt (`Session::unsaved_changes`): every
/// player and team whose entry differs, with the diffs the compares list,
/// and the groups of roster places whose players traded records.
pub(crate) struct UnsavedChanges {
    /// `(player id, his diffs)`, in record order.
    pub(crate) players: Vec<(u32, Vec<PlayerDiff>)>,
    /// `(team id, the team's diffs)`, in record order.
    pub(crate) teams: Vec<(u32, Vec<TeamDiff>)>,
    /// `(team id, one group's places in roster order)`: the places whose
    /// players traded records among themselves, one entry per group (a
    /// drag from place 04 to 07 is a group of four, a second, separate
    /// swap a second group), teams in record order.
    pub(crate) moves: Vec<(u32, Vec<MovedPlace>)>,
}

/// One roster place of a group of moved players (`UnsavedChanges::moves`).
#[derive(Debug, PartialEq)]
pub(crate) struct MovedPlace {
    /// The roster slot, 0-based (place 04 is slot 3).
    pub(crate) slot: u8,
    /// The player id the slot holds, in the file and now: the file's
    /// player on the place is the baseline's record of this id.
    pub(crate) id: u32,
    /// The id whose record of the file the place holds now: the player now
    /// on the place is the file's player of this id, with what was edited
    /// on him since.
    pub(crate) origin: u32,
}

/// For every player id holding another id's record of the file as opened
/// or last saved, that other id (the record's origin); an id holding its
/// own record has no entry. The moves keep it, so the unsaved changes
/// compare each player with the record he came from.
pub(crate) type Origins = HashMap<u32, u32>;

/// The id whose record of the file `id` holds.
fn origin(origins: &Origins, id: u32) -> u32 {
    origins.get(&id).copied().unwrap_or(id)
}

/// Applies a trade of records to `origins`: every `(id, from)` of `took`
/// now holds what `from` held. Every origin is read before the first
/// write, since a trade's ids are its sources too.
pub(crate) fn trade(origins: &mut Origins, took: impl Iterator<Item = (u32, u32)>) {
    let traded: Vec<(u32, u32)> = took.map(|(id, from)| (id, origin(origins, from))).collect();
    for (id, from) in traded {
        if id == from {
            origins.remove(&id);
        } else {
            origins.insert(id, from);
        }
    }
}

/// The open save: the file, the path it came from, the undo stack and the
/// baseline the unsaved-changes marks compare against (the file's players
/// and teams as opened or last saved).
pub(crate) struct Session {
    file: EditFile,
    /// The path `open` read, or the last `save_to` wrote: where Save writes.
    path: PathBuf,
    /// Undo and redo, one entry per gesture and one per run of steps to
    /// one value.
    undo: UndoStack,
    /// Every player by id, as opened or last saved.
    baseline_players: HashMap<u32, PlayerEntry>,
    /// Every team by id, as opened or last saved.
    baseline_teams: HashMap<u32, TeamEntry>,
    /// Whose record of the baseline each player id holds, after the moves
    /// since the open or the last save.
    origins: Origins,
    /// Whether the model differs from the baseline, recomputed by each
    /// mutating call (the toolbar's marker reads the flag, not a rescan).
    dirty: bool,
    /// The tick of the last change to the open file (`revision()`; every
    /// change takes a fresh one, so it never repeats).
    revision: u64,
    /// The frame's clock (egui's `input.time`, seconds), set per frame by
    /// the view (`set_time`): the undo run's pause measures real seconds,
    /// not frames. 0.0 until a frame sets it — the command line's pushes,
    /// all sealed, keep no run either way.
    now: f64,
    /// The repro journal `changed` records every step into and `open`
    /// and `write` keep their files in: the context's shared one in the
    /// window, a default one on the command line, so a bundle covers the
    /// whole session.
    recorder: Recorder,
}

impl Session {
    /// Opens the save at `path`. The error names the path and what
    /// `pes_savefile` refused. `recorder` gets the open's step and keeps
    /// the file's bytes as read — what the tool started from is the part
    /// a saved-over file would destroy — and a refused open records its
    /// own step, keeping the bytes it read.
    pub(crate) fn open(path: &Path, recorder: Recorder) -> anyhow::Result<Session> {
        Self::open_step(path, recorder, "")
    }

    /// `open` for the comparator's second save: the step reads `opened
    /// <path> for comparison`, so the bundle tells the pick from the
    /// session's own open.
    pub(crate) fn open_for_comparison(path: &Path, recorder: Recorder) -> anyhow::Result<Session> {
        Self::open_step(path, recorder, " for comparison")
    }

    /// `open`'s body, `suffix` on the open step's path (`""` for the
    /// session's own open).
    fn open_step(path: &Path, recorder: Recorder, suffix: &str) -> anyhow::Result<Session> {
        let bytes: Arc<[u8]> = match std::fs::read(path) {
            Ok(bytes) => bytes.into(),
            Err(error) => return Err(open_failed(path, &recorder, error.into(), None)),
        };
        let file = match EditFile::from_bytes(&bytes) {
            Ok(file) => file,
            Err(error) => {
                return Err(open_failed(path, &recorder, error.into(), Some(bytes)));
            }
        };
        let (baseline_players, baseline_teams) = baselines(&file);
        let what = format!(
            "opened {}{} ({}, {} bytes)",
            path.display(),
            suffix,
            file.version(),
            bytes.len()
        );
        let input = recorder.keep_input(TOOL_ID, path, bytes);
        if suffix.is_empty() {
            // The session's own open names the kept input it read: replay
            // starts from it. The comparator's second open is a note.
            recorder.step_op(TOOL_ID, what, op_table(&Op::Open { input }));
        } else {
            recorder.step(TOOL_ID, what);
        }
        Ok(Session {
            file,
            path: path.to_path_buf(),
            undo: UndoStack::new(),
            baseline_players,
            baseline_teams,
            origins: Origins::new(),
            dirty: false,
            revision: NEXT_REVISION.fetch_add(1, Ordering::Relaxed),
            now: 0.0,
            recorder,
        })
    }

    /// The save's file: the one it was opened from, or the last Save as
    /// wrote. Save writes there.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// The open save's game version.
    pub(crate) fn version(&self) -> PesVersion {
        self.file.version()
    }

    /// The open save, for the read-only operations over two saves (the comparator).
    pub(crate) fn file(&self) -> &EditFile {
        &self.file
    }

    /// The session's repro journal: the sites that keep a file the member
    /// picked (an export, the roster sheet) record through it — the same
    /// journal `changed`, `open` and `write` record to.
    pub(crate) fn recorder(&self) -> &Recorder {
        &self.recorder
    }

    /// The tick of the last change to the open file, which keys the Cards
    /// left badge's cache: a fresh number every change — an edit pushed, an
    /// undo, a redo, an import, a patch, a successful open — and
    /// never twice the same.
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    /// Marks the model changed: `steps` recorded in the journal (one per
    /// field or event, each carrying its typed change where the ops
    /// cover it, so a bundle replays the change), then the next
    /// `revision`.
    fn changed(&mut self, steps: Vec<Step>) {
        for step in steps {
            match step.op {
                Some(op) => self.recorder.step_op(TOOL_ID, step.line, op_table(&op)),
                None => self.recorder.step(TOOL_ID, step.line),
            }
        }
        self.revision = NEXT_REVISION.fetch_add(1, Ordering::Relaxed);
    }

    /// The ids of the save's teams, in record order.
    pub(crate) fn team_ids(&self) -> impl Iterator<Item = u32> {
        self.file.teams().iter().map(|team| team.id)
    }

    /// The player id `team`'s roster `slot` (0-based) holds, `None` for a
    /// team the save lacks, a slot past the roster, an empty slot (id 0)
    /// or an id no player record carries. The selection's player and
    /// `apply_tier`'s are both read this way.
    pub(crate) fn slot_player(&self, team_id: u32, slot: u8) -> Option<u32> {
        let player_id = self
            .file
            .team(team_id)?
            .roster
            .get(usize::from(slot))?
            .player_id;
        if player_id == 0 {
            return None;
        }
        self.file.player(player_id)?;
        Some(player_id)
    }

    /// Replaces the player `edited.id` names with `edited` and pushes the
    /// change under `key` (the widget's `egui::Id` value, the undo stack's
    /// merge key); `edited` equal to the stored entry pushes nothing.
    pub(crate) fn edit_player(&mut self, key: u64, edited: PlayerEntry) {
        let Some(entry) = self.file.player_mut(edited.id) else {
            return;
        };
        if *entry == edited {
            return;
        }
        let before = std::mem::replace(entry, edited.clone());
        let steps = player_lines(&before, &edited, schema_for(self.version()));
        self.undo.push(
            key,
            Change::Player {
                before: Box::new(before),
                after: Box::new(edited),
            },
            self.now,
        );
        self.changed(steps);
        self.dirty = self.compute_dirty();
    }

    /// Replaces each player `edited` names (by `id`) with its entry, as one sealed undo entry; an
    /// entry equal to the stored one, or naming a player the save lacks, is skipped; nothing
    /// changed pushes nothing.
    pub(crate) fn edit_players(&mut self, edited: Vec<PlayerEntry>) {
        self.edit_entries(edited, Vec::new());
    }

    /// `edit_players` widened to teams (Fix database changes both in one
    /// step): every player of `players` and every team of `teams` replaced
    /// as one sealed undo entry, the same skip rules on both.
    pub(crate) fn edit_entries(&mut self, players: Vec<PlayerEntry>, teams: Vec<TeamEntry>) {
        let Some(steps) = self.push_entries(players, teams) else {
            return;
        };
        // One entry for the whole batch, sealed: the next edit does not merge.
        self.undo.seal();
        self.changed(steps);
        self.dirty = self.compute_dirty();
    }

    /// `edit_entries`' writes and pushes, the entry left open for a sealed
    /// operation to add to (`move_player`'s trade): the journal's steps, or
    /// `None` when nothing changed and nothing was pushed.
    fn push_entries(
        &mut self,
        players: Vec<PlayerEntry>,
        teams: Vec<TeamEntry>,
    ) -> Option<Vec<Step>> {
        // The before/after pairs are computed before the first write, as in
        // `move_player`: the write and the undo entry are the same data.
        let mut changed_players = Vec::new();
        for after in players {
            let Some(entry) = self.file.player(after.id) else {
                continue;
            };
            if *entry == after {
                continue;
            }
            changed_players.push((entry.clone(), after));
        }
        let mut changed_teams = Vec::new();
        for after in teams {
            let Some(entry) = self.file.team(after.id) else {
                continue;
            };
            if *entry == after {
                continue;
            }
            changed_teams.push((entry.clone(), after));
        }
        if changed_players.is_empty() && changed_teams.is_empty() {
            return None;
        }
        // The journal's lines are read off the same before/after pairs the
        // undo entry takes: a change the model took is a change recorded.
        let schema = schema_for(self.version());
        let mut steps = Vec::new();
        for (before, after) in &changed_players {
            steps.extend(player_lines(before, after, schema));
        }
        for (before, after) in &changed_teams {
            steps.extend(team_lines(before, after, schema));
        }
        for (before, after) in changed_players {
            *self.file.player_mut(after.id).expect("resolved above") = after.clone();
            self.undo.push(
                SEALED_KEY,
                Change::Player {
                    before: Box::new(before),
                    after: Box::new(after),
                },
                self.now,
            );
        }
        for (before, after) in changed_teams {
            *self.file.team_mut(after.id).expect("resolved above") = after.clone();
            self.undo.push(
                SEALED_KEY,
                Change::Team {
                    before: Box::new(before),
                    after: Box::new(after),
                },
                self.now,
            );
        }
        Some(steps)
    }

    /// `edit_player` for a team entry (a roster slot's number, the
    /// captain, the Tactics editor's edited clone).
    pub(crate) fn edit_team(&mut self, key: u64, edited: TeamEntry) {
        let Some(entry) = self.file.team_mut(edited.id) else {
            return;
        };
        if *entry == edited {
            return;
        }
        let before = std::mem::replace(entry, edited.clone());
        let steps = team_lines(&before, &edited, schema_for(self.version()));
        self.undo.push(
            key,
            Change::Team {
                before: Box::new(before),
                after: Box::new(edited),
            },
            self.now,
        );
        self.changed(steps);
        self.dirty = self.compute_dirty();
    }

    /// Whether the record on roster slot `from` moved onto `to` would write
    /// anything (`move_player`'s own rule, shared with the row's drop
    /// outline): `from` another slot than `to`, the team in the save, both
    /// slots inside its roster, and neither slot empty (player id 0).
    pub(crate) fn moves_player(&self, team_id: u32, from: u8, to: u8) -> bool {
        if from == to {
            return false;
        }
        let Some(team) = self.file.team(team_id) else {
            return false;
        };
        let (from, to) = (usize::from(from), usize::from(to));
        let Some(range) = team.roster.get(from.min(to)..=from.max(to)) else {
            return false;
        };
        let lo = from.min(to);
        range[from - lo].player_id != 0 && range[to - lo].player_id != 0
    }

    /// The player-list drop: the record on roster slot `from` lands on `to`'s player id
    /// and the records between shift one filled slot along, each slot keeping what
    /// belongs to its id (`id`, `appearance_id`, `appearance.base_copy_id`, and the face
    /// and hair in the in-game face run: `moved_record`). The roster is never written,
    /// so its ids, the shirt numbers and the captain stay put (slot `NN` holds player
    /// `team id x 100 + NN` by 4cc rule). The whole move is one sealed undo entry.
    /// `from == to`, a team or player the save lacks, or an empty `from` or `to` slot
    /// does nothing (`moves_player`); an empty slot between them is skipped over.
    pub(crate) fn move_player(&mut self, team_id: u32, from: u8, to: u8) {
        if !self.moves_player(team_id, from, to) {
            return;
        }
        let (from, to) = (usize::from(from), usize::from(to));
        let lo = from.min(to);
        let range = &self
            .file
            .team(team_id)
            .expect("`moves_player` passed: the team is in the save")
            .roster[lo..=from.max(to)];
        // The filled slots' ids in slot order: the sequence the move rotates.
        let filled: Vec<u32> = range
            .iter()
            .filter(|slot| slot.player_id != 0)
            .map(|slot| slot.player_id)
            .collect();
        let mut order = filled.clone();
        let taken = order.remove(
            filled
                .iter()
                .position(|id| *id == range[from - lo].player_id)
                .expect("`from` is filled"),
        );
        order.insert(
            filled
                .iter()
                .position(|id| *id == range[to - lo].player_id)
                .expect("`to` is filled"),
            taken,
        );
        // Each filled slot's player takes `order`'s record for it, keeping the
        // slot's own part of it (`moved_record`). Every entry is computed
        // before the first write: a rostered id the save lacks leaves it
        // untouched, aborting the whole move.
        let mut edited = Vec::new();
        let mut took = Vec::new();
        for (&slot_id, &record_id) in filled.iter().zip(&order) {
            if slot_id == record_id {
                continue;
            }
            let (Some(dest), Some(source)) =
                (self.file.player(slot_id), self.file.player(record_id))
            else {
                return;
            };
            edited.push(moved_record(source, dest));
            took.push((slot_id, record_id));
        }
        // Records equal on every traded id write nothing, so the trade is
        // not kept either: the unsaved changes stay as empty as the save.
        let Some(steps) = self.push_entries(edited, Vec::new()) else {
            return;
        };
        trade(&mut self.origins, took.iter().copied());
        // Into the entry the players' changes opened: one undo takes both.
        self.undo.push(SEALED_KEY, Change::Move { took }, self.now);
        self.undo.seal();
        self.changed(steps);
        self.dirty = self.compute_dirty();
    }

    /// Undoes the newest gesture; `false` when the stack is empty.
    pub(crate) fn undo(&mut self) -> bool {
        // The journal's lines come off the entry before `undo` moves it:
        // the fields' values run from the edit's `after` back to its
        // `before`.
        let steps = self.undo.undo_changes().map(|changes| {
            let schema = schema_for(self.version());
            let mut steps = vec![Step::note("undo")];
            for change in changes {
                steps.extend(change_lines(change, schema, true));
            }
            steps
        });
        let undone = self.undo.undo(&mut self.file, &mut self.origins);
        if undone {
            self.changed(steps.expect("`undo` returned true"));
            self.dirty = self.compute_dirty();
        }
        undone
    }

    /// Reapplies the newest undone gesture; `false` when the redo stack is
    /// empty.
    pub(crate) fn redo(&mut self) -> bool {
        let steps = self.undo.redo_changes().map(|changes| {
            let schema = schema_for(self.version());
            let mut steps = vec![Step::note("redo")];
            for change in changes {
                steps.extend(change_lines(change, schema, false));
            }
            steps
        });
        let redone = self.undo.redo(&mut self.file, &mut self.origins);
        if redone {
            self.changed(steps.expect("`redo` returned true"));
            self.dirty = self.compute_dirty();
        }
        redone
    }

    /// Whether `undo` would do anything (the Undo button's enabled state).
    pub(crate) fn can_undo(&self) -> bool {
        self.undo.can_undo()
    }

    /// Whether `redo` would do anything.
    pub(crate) fn can_redo(&self) -> bool {
        self.undo.can_redo()
    }

    /// Ends the open gesture and any run of steps under way: the next
    /// change starts a fresh undo entry (the keyboard moved off a
    /// widget, or a sealed operation ran).
    pub(crate) fn seal(&mut self) {
        self.undo.seal();
    }

    /// Ends only the open gesture, keeping a run of steps to one value:
    /// a push under the run's key within its pause still joins the
    /// newest entry (the frame's soft end — the pointer up with no text
    /// edit holding the keyboard).
    pub(crate) fn settle(&mut self) {
        self.undo.settle();
    }

    /// The frame's clock, in seconds: the undo run's pause is measured
    /// against it at each push. The view sets it once a frame from
    /// egui's `input.time` before any widget draws.
    pub(crate) fn set_time(&mut self, now: f64) {
        self.now = now;
    }

    /// Whether the save differs from the file as opened or last saved, the
    /// toolbar's marker; undoing back to the saved state clears it. The flag
    /// is kept by `compute_dirty` at every mutating call, so a frame can ask
    /// it without rescanning the model.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// The deep compare behind `dirty`: any player or team unlike its
    /// baseline entry.
    fn compute_dirty(&self) -> bool {
        self.file
            .players()
            .iter()
            .any(|player| self.baseline_players.get(&player.id) != Some(player))
            || self
                .file
                .teams()
                .iter()
                .any(|team| self.baseline_teams.get(&team.id) != Some(team))
    }

    /// Whether the player of `team`'s roster `slot` (0-based) differs from
    /// the baseline: his `PlayerEntry` or his slot's shirt number, which his
    /// row's dot covers.
    pub(crate) fn player_dirty(&self, team_id: u32, slot: u8) -> bool {
        let Some(slot_entry) = self
            .file
            .team(team_id)
            .and_then(|team| team.roster.get(usize::from(slot)))
        else {
            return false;
        };
        let baseline_slot = self
            .baseline_teams
            .get(&team_id)
            .and_then(|team| team.roster.get(usize::from(slot)));
        if baseline_slot.is_none_or(|base| base != slot_entry) {
            return true;
        }
        self.file
            .player(slot_entry.player_id)
            .is_some_and(|player| self.baseline_players.get(&player.id) != Some(player))
    }

    /// Whether the team `team_id`'s own entry differs from the baseline:
    /// its fields, tactics and kit slots, but not the roster slots' shirt
    /// numbers, which the player's dot covers.
    pub(crate) fn team_dirty(&self, team_id: u32) -> bool {
        let (Some(team), Some(baseline)) =
            (self.file.team(team_id), self.baseline_teams.get(&team_id))
        else {
            return false;
        };
        let mut masked = team.clone();
        for (slot, baseline_slot) in masked.roster.iter_mut().zip(&baseline.roster) {
            slot.number = baseline_slot.number;
        }
        masked != *baseline
    }

    /// The baseline entry of the player `id` (the file as opened or last
    /// saved), `None` when the baseline holds none: the old side of the
    /// unsaved-changes table's roster rows, whose names are the baseline's.
    pub(crate) fn baseline_player(&self, id: u32) -> Option<&PlayerEntry> {
        self.baseline_players.get(&id)
    }

    /// `baseline_player` for a team's entry: the old side's tactics an
    /// instruction-target row words its target byte under.
    pub(crate) fn baseline_team(&self, id: u32) -> Option<&TeamEntry> {
        self.baseline_teams.get(&id)
    }

    /// The differences from the baselines `is_dirty` compares against (the
    /// unsaved-changes table's rows): every player whose entry differs from
    /// the record he came from, `compare_players` on that record and his
    /// current one; every team that differs, `compare_teams`; and the
    /// groups of places whose players traded records (`moves`). A codec
    /// error is named by the player or team it hit (a gated field `None` on
    /// an entry never read from this save).
    pub(crate) fn unsaved_changes(&self) -> anyhow::Result<UnsavedChanges> {
        let schema = schema_for(self.version());
        let mut players = Vec::new();
        for player in self.file.players() {
            let Some(own) = self.baseline_players.get(&player.id) else {
                continue;
            };
            // What the file would hold on his id had the moves been made
            // in it: the record he came from, with his slot's own part.
            let moved;
            let baseline = match self.origins.get(&player.id) {
                Some(from) => {
                    let source = self
                        .baseline_players
                        .get(from)
                        .expect("an origin is an id of the baseline");
                    moved = moved_record(source, own);
                    &moved
                }
                None => own,
            };
            if baseline == player {
                continue;
            }
            let diffs = compare_players(baseline, player, schema)
                .with_context(|| format!("player {}", player.id))?;
            players.push((player.id, diffs));
        }
        let mut teams = Vec::new();
        for team in self.file.teams() {
            let Some(baseline) = self.baseline_teams.get(&team.id) else {
                continue;
            };
            if baseline == team {
                continue;
            }
            let diffs = compare_teams(baseline, team, schema)
                .with_context(|| format!("team {}", team.id))?;
            teams.push((team.id, diffs));
        }
        Ok(UnsavedChanges {
            players,
            teams,
            moves: self.moves(),
        })
    }

    /// The groups of roster places whose players traded records, one per
    /// cycle of `origins` (each id holds its origin's record, the origin
    /// holds its own origin's, and so on back to the first), teams in
    /// record order, each team's groups in the order of their first place.
    /// A player rostered on two teams is grouped under the first.
    fn moves(&self) -> Vec<(u32, Vec<MovedPlace>)> {
        let mut grouped: HashSet<u32> = HashSet::new();
        let mut moves = Vec::new();
        for team in self.file.teams() {
            let slot_of = |id: u32| {
                team.roster
                    .iter()
                    .position(|slot| slot.player_id == id)
                    .and_then(|slot| u8::try_from(slot).ok())
            };
            for slot in &team.roster {
                let start = slot.player_id;
                if !self.origins.contains_key(&start) || grouped.contains(&start) {
                    continue;
                }
                let mut places = Vec::new();
                let mut id = start;
                while grouped.insert(id) {
                    let from = origin(&self.origins, id);
                    if let Some(slot) = slot_of(id) {
                        places.push(MovedPlace {
                            slot,
                            id,
                            origin: from,
                        });
                    }
                    id = from;
                }
                places.sort_by_key(|place| place.slot);
                moves.push((team.id, places));
            }
        }
        moves
    }

    /// Writes the save to `path` (Save as; Save passes the session's own
    /// path), then makes `path` the session's file, where the next Save
    /// writes, and the current model the baseline. `edit_mode_closed` lets
    /// the write go ahead while the save's own PES runs (the command line's
    /// `--edit-mode-closed`, the window's running-game dialog's Save):
    /// refused only when that PES runs and it is `false` (the game writes
    /// its save back over ours when the member leaves Edit mode after
    /// changing anything there); a game of another version does not block
    /// it.
    pub(crate) fn save_to(
        &mut self,
        path: &Path,
        running: &[RunningPes],
        edit_mode_closed: bool,
    ) -> anyhow::Result<()> {
        refuse_while_running(running, self.version(), edit_mode_closed)?;
        self.write(path)?;
        self.path = path.to_path_buf();
        (self.baseline_players, self.baseline_teams) = baselines(&self.file);
        self.origins.clear();
        self.dirty = false;
        Ok(())
    }

    /// The Team TOML text of the team with `id`: its entry, tactics and every rostered
    /// player with every field. An id the save lacks is an error naming it.
    pub(crate) fn export_team(&self, id: u32) -> anyhow::Result<String> {
        let team = self
            .file
            .team(id)
            .ok_or_else(|| anyhow!("the save has no team {id}"))?;
        // The whole pool, borrowed: `from_team` picks the roster's players out of it.
        let players: Vec<&PlayerEntry> = self.file.players().iter().collect();
        let document = TeamToml::from_team(self.version(), team, &players)
            .with_context(|| format!("team {id} cannot be exported"))?;
        document
            .to_toml()
            .with_context(|| format!("team {id} cannot be written as Team TOML"))
    }

    /// Imports `document` onto the team its `[team] id` names, with the lib's import rules
    /// (absent = untouched, cross-version conversion when the document's version differs),
    /// all or nothing. The notes are what was converted or dropped. A document with no id,
    /// one naming a team the save lacks, or one the lib refuses (a team name or short name
    /// that does not fit the version's field; a player's name or shirt name is cut to it with
    /// a warning instead) is an error and the save is untouched.
    pub(crate) fn import_team(&mut self, document: &TeamToml) -> anyhow::Result<Vec<ImportNote>> {
        let id = document
            .team
            .id
            .ok_or_else(|| anyhow!("the Team TOML names no team: [team] id is required"))?;
        // Applied to a copy of the team, put back once every key applied: the lib takes the
        // team and the player pool as two borrows, which one `EditFile` cannot hand out at once.
        let mut team = self
            .file
            .team(id)
            .ok_or_else(|| anyhow!("the save has no team {id}"))?
            .clone();
        // For the journal's field lines: the import writes only the team
        // and its rostered players, so those are the entries compared.
        let before_team = team.clone();
        let before_players: Vec<PlayerEntry> = team
            .roster
            .iter()
            .filter(|slot| slot.player_id != 0)
            .filter_map(|slot| self.file.player(slot.player_id))
            .cloned()
            .collect();
        let version = self.version();
        let notes = document.apply(version, &mut team, self.file.players_mut())?;
        *self.file.team_mut(id).expect("found above") = team;
        let schema = schema_for(version);
        let mut steps = vec![Step::note(format!("imported Team TOML onto team {id}"))];
        steps.extend(team_lines(
            &before_team,
            self.file.team(id).expect("found above"),
            schema,
        ));
        for before in &before_players {
            if let Some(after) = self.file.player(before.id) {
                steps.extend(player_lines(before, after, schema));
            }
        }
        self.changed(steps);
        self.dirty = self.compute_dirty();
        Ok(notes)
    }

    /// Applies the compiler's patch: each listed player's names, whole look and base copy
    /// as his entry holds them, an entry's `[appearance]` making the record his
    /// (`AestheticsPatch::apply`). The summary lists the teams applied
    /// and the items the save lacked; a version or scheme mismatch is an error with the
    /// save untouched.
    pub(crate) fn apply_patch(
        &mut self,
        patch: &AestheticsPatch,
    ) -> Result<PatchSummary, PatchError> {
        let summary = patch.apply(&mut self.file)?;
        if summary.applied_players.is_empty() {
            return Ok(summary);
        }
        // A team is named only when one of its players was written (a cup
        // player id is its team's id x 100 + slot): `applied_teams` alone
        // would name a team whose every patched player the save lacked.
        let mut steps = vec![Step::note("applied the aesthetics patch")];
        let mut teams: Vec<u32> = Vec::new();
        for id in &summary.applied_players {
            let team = id / 100;
            if !teams.contains(&team) {
                teams.push(team);
                steps.push(Step::note(format!("applied to team {team}")));
            }
        }
        self.changed(steps);
        self.dirty = self.compute_dirty();
        Ok(summary)
    }

    /// Writes the save to `out`, an existing file there first copied to `<out>.bak` (one
    /// level, as every save does); `out` may be the path the save was opened from.
    /// A failed write records `could not write <path>` and keeps nothing.
    pub(crate) fn write(&self, out: &Path) -> anyhow::Result<()> {
        if let Err(error) = self.file.save(out) {
            write_failed(out, &self.recorder, &error);
            return Err(anyhow::Error::new(error)
                .context(format!("{}: cannot write the save", out.display())));
        }
        let output = match std::fs::read(out) {
            Ok(written) => Some(self.recorder.keep_output(TOOL_ID, out, written.into())),
            // The save was written; a journal that cannot keep it loses the
            // bundle's output, never the member's write.
            Err(error) => {
                log::debug!(
                    "{}: the written save cannot be kept ({error})",
                    out.display()
                );
                None
            }
        };
        self.recorder.step_op(
            TOOL_ID,
            format!("wrote {}", out.display()),
            op_table(&Op::Wrote { output }),
        );
        Ok(())
    }
}

/// `source`'s record retargeted onto `dest`'s slot (`move_player`): the fields
/// `player_export` carries move with the record; the fields it does not carry
/// are `dest`'s — `id`, `appearance_id`, `base_copy_id`, and the in-game face
/// run apart from `EXPORT_FACE_FIELDS`, whose four the export holds and so
/// moves (its appearance section ends at `iris_col`; the face structure and
/// the hair are not in the struct).
fn moved_record(source: &PlayerEntry, dest: &PlayerEntry) -> PlayerEntry {
    let mut record = source.clone();
    record.id = dest.id;
    record.appearance_id = dest.appearance_id;
    record.appearance.base_copy_id = dest.appearance.base_copy_id;
    record.appearance.ingame_face = dest.appearance.ingame_face.clone();
    // A loaded save reads every player's whole run, and both layouts hold
    // the four fields, so neither call can fail on a record of the save.
    for field in EXPORT_FACE_FIELDS {
        let value = source
            .appearance
            .ingame_face
            .get(field)
            .expect("a loaded player's face run holds the exported fields");
        record
            .appearance
            .ingame_face
            .set(field, value)
            .expect("two players of one save share a face layout");
    }
    record
}

/// The save's players and teams by id: the baseline `is_dirty` and
/// `player_dirty` compare against, taken at `open` and after `save`.
fn baselines(file: &EditFile) -> (HashMap<u32, PlayerEntry>, HashMap<u32, TeamEntry>) {
    (
        file.players()
            .iter()
            .map(|player| (player.id, player.clone()))
            .collect(),
        file.teams()
            .iter()
            .map(|team| (team.id, team.clone()))
            .collect(),
    )
}

/// The refusal a writing operation makes while the PES of `version` runs and
/// `edit_mode_closed` is not given (the command line's word that the game's
/// Edit mode is closed, or the window's running-game dialog's Save): the game
/// writes the save it holds over the file when the member leaves Edit mode
/// after changing anything there, so a write made while it is open is lost
/// unless Edit mode's Load reads it in first. A game of another version never
/// writes this save, so it does not refuse (`spec/save_editor.md` "CLI").
pub(crate) fn refuse_while_running(
    running: &[RunningPes],
    version: PesVersion,
    edit_mode_closed: bool,
) -> anyhow::Result<()> {
    if edit_mode_closed {
        return Ok(());
    }
    match running.iter().find(|game| game.version == version) {
        Some(game) => Err(anyhow!(
            "{} is running: the game writes the save it holds over the file when you leave \
             its Edit mode after changing anything there, so nothing was written. Close the \
             game's Edit mode, then run the command again with `--edit-mode-closed`. The save \
             is then written, and opening Edit mode reads it in without restarting the game.",
            game.version
        )),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests;
