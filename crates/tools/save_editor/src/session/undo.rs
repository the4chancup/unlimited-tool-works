//! The undo and redo stacks: an undo entry is the before and after of each
//! `PlayerEntry` or `TeamEntry` it changed, one entry per gesture — a widget's changes merge
//! into the open entry while it stays unsealed — and a run of steps to one
//! value (wheel notches, arrow presses, a stepper's clicks) is one entry
//! across its gestures' ends, until a second's pause, a step under another
//! key, or the keyboard moving off a widget.

use pes_savefile::EditFile;
use pes_savefile::model::{PlayerEntry, TeamEntry};

/// One model entry an edit changed, before and after. The entries are boxed: a
/// `TeamEntry` (its tactics included) is more than twice a `PlayerEntry`'s size, and
/// unboxed every change would carry the larger variant's room.
pub(crate) enum Change {
    /// A player's entry.
    Player {
        /// The entry before the edit.
        before: Box<PlayerEntry>,
        /// The entry after.
        after: Box<PlayerEntry>,
    },
    /// A team's entry.
    Team {
        /// The entry before the edit.
        before: Box<TeamEntry>,
        /// The entry after.
        after: Box<TeamEntry>,
    },
}

/// How long, in seconds, a run of steps to one value stays joinable after
/// its last push: a wheel spun in one go stays one undo entry, while a
/// value nudged again a while later is a new one.
const RUN_PAUSE: f64 = 1.0;

/// One undo step: the widget whose gesture made it (the merge key) and the
/// model entries it changed.
struct Entry {
    /// The host's widget id; only a change pushed under the same key merges
    /// into the open entry, or into the newest entry through a kept run.
    key: u64,
    /// The entries the gesture changed so far.
    changes: Vec<Change>,
}

impl Entry {
    /// `change` merged into the entry: one already holding that player or
    /// team keeps its `before` and takes the new `after`; another's joins
    /// the list.
    fn merge(&mut self, change: Change) {
        match self
            .changes
            .iter_mut()
            .find(|held| held.same_entry(&change))
        {
            Some(held) => held.merge_after(change),
            None => self.changes.push(change),
        }
    }
}

/// The session's undo and redo stacks. `undo.last()` is the open entry while
/// `open` holds: pushes under its key merge into it; `seal`, `settle` or a
/// push under another key closes it. `seal` ends the run as well; `settle`
/// keeps it, so a push under the run's key within `RUN_PAUSE` of the last
/// one still joins the newest entry, which is always the run's own (a push
/// records the run after merging or pushing, and an undo or redo clears it).
pub(crate) struct UndoStack {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    open: bool,
    /// The last push's merge key and when it landed (`now`, egui's clock).
    run: Option<(u64, f64)>,
}

impl Change {
    /// Whether the change does nothing.
    fn is_noop(&self) -> bool {
        match self {
            Change::Player { before, after } => before == after,
            Change::Team { before, after } => before == after,
        }
    }

    /// The same model entry another `Change` names: same kind (player or
    /// team), same id.
    fn same_entry(&self, other: &Change) -> bool {
        match (self, other) {
            (Change::Player { before: a, .. }, Change::Player { before: b, .. }) => a.id == b.id,
            (Change::Team { before: a, .. }, Change::Team { before: b, .. }) => a.id == b.id,
            (Change::Player { .. }, Change::Team { .. })
            | (Change::Team { .. }, Change::Player { .. }) => false,
        }
    }

    /// `self` retargeted to `other`'s after, keeping its own before (the
    /// merge's rule for a second change to one entry).
    fn merge_after(&mut self, other: Change) {
        match (self, other) {
            (Change::Player { after, .. }, Change::Player { after: new, .. }) => *after = new,
            (Change::Team { after, .. }, Change::Team { after: new, .. }) => *after = new,
            (Change::Player { .. }, Change::Team { .. })
            | (Change::Team { .. }, Change::Player { .. }) => {
                unreachable!("merge_after is called only under `same_entry`")
            }
        }
    }

    /// Writes `before`'s entry back into the file (`before` set) or
    /// `after`'s (cleared): undo and redo's halves of a restore. An entry
    /// the file lacks is left (the model never removes one).
    fn restore(&self, file: &mut EditFile, before: bool) {
        match self {
            Change::Player {
                before: old,
                after: new,
            } => {
                let value = if before { old } else { new };
                if let Some(player) = file.player_mut(value.id) {
                    *player = PlayerEntry::clone(value);
                }
            }
            Change::Team {
                before: old,
                after: new,
            } => {
                let value = if before { old } else { new };
                if let Some(team) = file.team_mut(value.id) {
                    *team = TeamEntry::clone(value);
                }
            }
        }
    }
}

impl UndoStack {
    /// An empty stack.
    pub(crate) fn new() -> UndoStack {
        UndoStack {
            undo: Vec::new(),
            redo: Vec::new(),
            open: false,
            run: None,
        }
    }

    /// Whether `undo` would do anything (the button's enabled state).
    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether `redo` would do anything.
    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Pushes `change` under `key` at `now` (the frame's clock): a change
    /// equal to its own before is not pushed; a change under the open
    /// entry's key merges into it (an entry already holding that player or
    /// team keeps its `before` and takes the new `after`); a change under
    /// the settled run's key within `RUN_PAUSE` of its last push merges
    /// into the newest entry when it holds a change to the same player or
    /// team, and reopens it; any other push starts an entry. Every push
    /// records its key and `now` as the run; a push clears the redo stack.
    pub(crate) fn push(&mut self, key: u64, change: Change, now: f64) {
        if change.is_noop() {
            return;
        }
        self.redo.clear();
        match self.undo.last_mut() {
            Some(entry) if self.open && entry.key == key => entry.merge(change),
            Some(entry)
                if !self.open
                    && self
                        .run
                        .is_some_and(|(run_key, at)| run_key == key && now - at <= RUN_PAUSE)
                    && entry.changes.iter().any(|held| held.same_entry(&change)) =>
            {
                entry.merge(change);
                self.open = true;
            }
            _ => {
                self.undo.push(Entry {
                    key,
                    changes: vec![change],
                });
                self.open = true;
            }
        }
        self.run = Some((key, now));
    }

    /// Closes the open entry and ends the run: the next push starts a
    /// fresh entry even under the same key within the pause (the gesture
    /// ended, and so did a run of steps to one value — the keyboard moved
    /// off the widget, or a sealed operation ran).
    pub(crate) fn seal(&mut self) {
        self.open = false;
        self.run = None;
    }

    /// Closes the open entry but keeps the run: a push under the run's key
    /// within `RUN_PAUSE` of its last still merges into the newest entry
    /// (the gesture ended; the run of steps to one value did not).
    pub(crate) fn settle(&mut self) {
        self.open = false;
    }

    /// Restores each change's `before` into the file and moves the newest
    /// entry to redo; `false` when the stack is empty.
    pub(crate) fn undo(&mut self, file: &mut EditFile) -> bool {
        self.seal();
        let Some(entry) = self.undo.pop() else {
            return false;
        };
        for change in &entry.changes {
            change.restore(file, true);
        }
        self.redo.push(entry);
        true
    }

    /// Reapplies each change's `after` and moves the newest redo entry back;
    /// `false` when the redo stack is empty.
    pub(crate) fn redo(&mut self, file: &mut EditFile) -> bool {
        self.seal();
        let Some(entry) = self.redo.pop() else {
            return false;
        };
        for change in &entry.changes {
            change.restore(file, false);
        }
        self.undo.push(entry);
        self.open = false;
        true
    }
}

#[cfg(test)]
mod tests {
    use pes_savefile::fixture_save;
    use pes_savefile::model::player::PlayerBasics;
    use pes_version::PesVersion;

    use super::*;

    fn file() -> EditFile {
        EditFile::from_bytes(&fixture_save(PesVersion::Pes17)).unwrap()
    }

    /// A `Change::Player` for `id` whose height goes `from` to `to`.
    fn change(id: u32, from: u8, to: u8) -> Change {
        let basic = |height| PlayerBasics {
            height,
            ..Default::default()
        };
        Change::Player {
            before: Box::new(PlayerEntry {
                id,
                basic: basic(from),
                ..Default::default()
            }),
            after: Box::new(PlayerEntry {
                id,
                basic: basic(to),
                ..Default::default()
            }),
        }
    }

    /// The height the file's player `id` holds.
    fn height(file: &EditFile, id: u32) -> u8 {
        file.player(id).unwrap().basic.height
    }

    #[test]
    fn pushes_under_one_key_merge_until_sealed() {
        let mut file = file();
        let mut stack = UndoStack::new();
        stack.push(1, change(70101, 10, 11), 0.0);
        stack.push(1, change(70101, 11, 12), 0.0);
        // One entry: undo restores the first push's before, not the last's.
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 10);
        assert!(!stack.can_undo());

        stack.seal();
        stack.push(1, change(70101, 10, 11), 0.0);
        stack.seal();
        stack.push(1, change(70101, 11, 12), 0.0);
        // Two entries: the sealed first gesture is its own undo step.
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 11);
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 10);
    }

    #[test]
    fn a_push_under_another_key_starts_an_entry() {
        let mut file = file();
        let mut stack = UndoStack::new();
        stack.push(1, change(70101, 10, 11), 0.0);
        stack.push(2, change(70102, 20, 21), 0.0);
        stack.undo(&mut file);
        assert_eq!(height(&file, 70102), 20);
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 10);
    }

    #[test]
    fn an_entry_merges_changes_to_different_players_under_one_key() {
        let mut file = file();
        let mut stack = UndoStack::new();
        stack.push(1, change(70101, 10, 11), 0.0);
        stack.push(1, change(70102, 20, 21), 0.0);
        // One entry holding both: undo restores both players at once.
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 10);
        assert_eq!(height(&file, 70102), 20);
        assert!(!stack.can_undo());
    }

    #[test]
    fn a_push_clears_redo() {
        let mut file = file();
        let mut stack = UndoStack::new();
        stack.push(1, change(70101, 10, 11), 0.0);
        stack.undo(&mut file);
        assert!(stack.can_redo());
        stack.push(2, change(70102, 20, 21), 0.0);
        assert!(!stack.can_redo());
    }

    #[test]
    fn an_equal_after_pushes_nothing() {
        let mut stack = UndoStack::new();
        stack.push(1, change(70101, 10, 10), 0.0);
        assert!(!stack.can_undo());
    }

    #[test]
    fn a_merged_entry_redoes_to_the_latest_after() {
        let mut file = file();
        let mut stack = UndoStack::new();
        stack.push(1, change(70101, 10, 11), 0.0);
        // Merged under the same key: one entry holding before=10 after=12.
        stack.push(1, change(70101, 11, 12), 0.0);
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 10);
        stack.redo(&mut file);
        assert_eq!(
            height(&file, 70101),
            12,
            "the latest after, not the first's"
        );
    }

    #[test]
    fn undo_then_redo_round_trips() {
        let mut file = file();
        let mut stack = UndoStack::new();
        let before = file.player(70101).unwrap().clone();
        file.player_mut(70101).unwrap().basic.height = 99;
        let after = file.player(70101).unwrap().clone();
        stack.push(
            1,
            Change::Player {
                before: Box::new(before),
                after: Box::new(after),
            },
            0.0,
        );
        stack.undo(&mut file);
        assert_ne!(height(&file, 70101), 99);
        stack.redo(&mut file);
        assert_eq!(height(&file, 70101), 99);
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }

    // A run of steps to one value: a push under the settled run's key
    // within the pause merges into the newest entry and reopens it (one
    // undo restores the first before, not the last's).
    #[test]
    fn a_push_within_the_pause_joins_the_settled_run() {
        let mut file = file();
        let mut stack = UndoStack::new();
        stack.push(1, change(70101, 10, 11), 0.0);
        stack.settle();
        stack.push(1, change(70101, 11, 12), RUN_PAUSE);
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 10, "the two pushes were one entry");
        assert!(!stack.can_undo());
    }

    // Past the pause the run has ended: the same key's push is its own
    // entry (a value nudged again a while later is a new one).
    #[test]
    fn a_push_past_the_pause_is_its_own_entry() {
        let mut file = file();
        let mut stack = UndoStack::new();
        stack.push(1, change(70101, 10, 11), 0.0);
        stack.settle();
        stack.push(1, change(70101, 11, 12), RUN_PAUSE + 0.5);
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 11);
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 10);
        assert!(!stack.can_undo());
    }

    // `seal` ends the run as the gesture ends, where `settle` keeps it:
    // a push after a seal is its own entry even under the run's key
    // inside the pause.
    #[test]
    fn a_seal_between_pushes_ends_the_run() {
        let mut file = file();
        let mut stack = UndoStack::new();
        stack.push(1, change(70101, 10, 11), 0.0);
        stack.seal();
        stack.push(1, change(70101, 11, 12), 0.5);
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 11);
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 10);
        assert!(!stack.can_undo());
    }

    // A push under another key ends the run: the first key's next push,
    // inside the pause, joins neither of the two before it.
    #[test]
    fn a_push_under_another_key_ends_the_run() {
        let mut file = file();
        let mut stack = UndoStack::new();
        stack.push(1, change(70101, 10, 11), 0.0);
        stack.settle();
        stack.push(2, change(70101, 11, 12), 0.3);
        stack.settle();
        stack.push(1, change(70101, 12, 13), 0.5);
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 12, "the last push its own entry");
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 11);
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 10);
        assert!(!stack.can_undo(), "three entries");
    }

    // The run's key is not enough on its own: a push under it on a
    // player the newest entry does not hold is its own entry (a run is
    // of steps to one value, one player's or team's). The first undo
    // takes back only the second push: a merge into the run's entry
    // would write the first player's `before` back with it.
    #[test]
    fn the_runs_key_on_another_player_is_its_own_entry() {
        let mut file = file();
        let mut stack = UndoStack::new();
        let fixture_height = height(&file, 70101);
        stack.push(1, change(70101, 10, 11), 0.0);
        stack.settle();
        stack.push(1, change(70102, 20, 21), 0.5);
        stack.undo(&mut file);
        assert_eq!(height(&file, 70102), 20);
        assert_eq!(
            height(&file, 70101),
            fixture_height,
            "the first undo touched only the second push"
        );
        stack.undo(&mut file);
        assert_eq!(height(&file, 70101), 10);
        assert!(!stack.can_undo(), "two entries");
    }
}
