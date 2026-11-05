//! The Fox player tables (`spec/team_compiler/pipeline.md` "Bins accumulation"): `BootsList.bin` and
//! `GloveList.bin`, which give a player his custom boots or gloves ID, and
//! `PlayerAppearance.bin`, every player's look. The game reads the highest-priority copy of
//! each table whole, so a compile writes the installed table with its compiled players' rows
//! changed, never a table of its own rows alone. None has a bundled base: a table no installed
//! CPK holds is not written.
//!
//! The tables' codec is `pes_savefile::player_tables`; what is the tool's
//! stays here: applying the `ItemRow`s planning decided and the `player_table_missing`
//! finding.

use std::collections::BTreeSet;

use pes_savefile::player_tables::{ItemList, PlayerTableFile};
use studio_core::{Disposition, Message, Scope};

use crate::messages::{Code, tool_message};
use crate::plan::item_rows::ItemRow;

/// One of the two tables pointing a player at his own boots or gloves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemTable {
    /// `BootsList.bin`: (player id, boots ID).
    Boots,
    /// `GloveList.bin`: (player id, gloves ID).
    Gloves,
}

impl ItemTable {
    /// Both tables, in the order the CPK holds them.
    pub(crate) const ALL: [ItemTable; 2] = [ItemTable::Boots, ItemTable::Gloves];

    /// The table's `PlayerTableFile`, the codec's name for it.
    pub(crate) fn file(self) -> PlayerTableFile {
        match self {
            ItemTable::Boots => PlayerTableFile::BootsList,
            ItemTable::Gloves => PlayerTableFile::GloveList,
        }
    }
}

/// Applies `rows`' rows of `table` to `list`: each row that counts (`ItemRow::counts`) is
/// set, the installed pair staying when its task is not among `committed`.
pub(crate) fn apply(
    list: &mut ItemList,
    table: ItemTable,
    rows: &[ItemRow],
    committed: &BTreeSet<usize>,
) {
    for row in rows.iter().filter(|row| row.table == table) {
        if row.counts(committed) {
            list.set(row.player_id, row.id.to_le_bytes());
        }
    }
}

/// The rows of `table` a missing `BootsList.bin` or `GloveList.bin` leaves out: its `rows`
/// that count (`ItemRow::counts`).
pub(crate) fn left_out(table: ItemTable, rows: &[ItemRow], committed: &BTreeSet<usize>) -> usize {
    rows.iter()
        .filter(|row| row.table == table)
        .filter(|row| row.counts(committed))
        .count()
}

/// The `player_table_missing` finding for a table no CPK of the walk holds when the run has
/// `left_out` rows for it, naming the table and counting them; none when there are no such
/// rows.
pub(crate) fn table_missing(file: PlayerTableFile, left_out: usize) -> Option<Message> {
    if left_out == 0 {
        return None;
    }
    Some(tool_message(
        Code::PlayerTableMissing,
        Scope::Run,
        Disposition::Keep,
        vec![
            ("table", file.file_name().to_owned()),
            ("rows", left_out.to_string()),
        ],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pairs `pairs` (player id, item ID) as a table holds them, in the order given.
    fn bytes_of(pairs: &[(u32, u32)]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (player_id, item_id) in pairs {
            bytes.extend(player_id.to_le_bytes());
            bytes.extend(item_id.to_le_bytes());
        }
        bytes
    }

    /// The boots list holding `pairs`.
    fn boots(pairs: &[(u32, u32)]) -> ItemList {
        ItemList::read(ItemTable::Boots.file(), &bytes_of(pairs)).unwrap()
    }

    /// The row of `table` setting `player_id`'s item to `id` once `task` commits (`Some`),
    /// or unconditionally (`None`).
    fn row(table: ItemTable, player_id: u32, id: u32, task: Option<usize>) -> ItemRow {
        ItemRow {
            table,
            player_id,
            id,
            task,
        }
    }

    #[test]
    fn a_committed_row_is_set_an_uncommitted_one_kept_and_a_taskless_one_set() {
        let mut list = boots(&[(70201, 11), (71405, 7), (71406, 9), (71407, 8)]);
        let rows = [
            row(ItemTable::Boots, 71405, 625, Some(1)),
            // Its task failed: the installed pair stays.
            row(ItemTable::Boots, 71407, 627, Some(2)),
            // The resolved default, which no task emits.
            row(ItemTable::Boots, 71406, 0, None),
            row(ItemTable::Boots, 71408, 628, Some(3)),
            // Another table's rows change nothing here.
            row(ItemTable::Gloves, 70201, 0, None),
            row(ItemTable::Gloves, 71409, 629, Some(1)),
        ];
        apply(&mut list, ItemTable::Boots, &rows, &BTreeSet::from([1, 3]));
        assert_eq!(
            list.to_bytes(),
            bytes_of(&[
                (70201, 11),
                (71405, 625),
                (71406, 0),
                (71407, 8),
                (71408, 628)
            ])
        );
    }

    #[test]
    fn a_missing_table_counts_its_committed_and_taskless_rows_and_says_nothing_without_one() {
        let rows = [
            row(ItemTable::Boots, 71405, 625, Some(1)),
            row(ItemTable::Boots, 71407, 627, Some(2)),
            row(ItemTable::Boots, 71408, 628, Some(3)),
            row(ItemTable::Boots, 71406, 0, None),
            row(ItemTable::Gloves, 71406, 0, None),
        ];
        let committed = BTreeSet::from([1, 3]);
        assert_eq!(left_out(ItemTable::Boots, &rows, &committed), 3);
        assert_eq!(
            left_out(ItemTable::Boots, &rows, &BTreeSet::new()),
            1,
            "only the taskless row"
        );
        assert_eq!(
            table_missing(
                ItemTable::Boots.file(),
                left_out(ItemTable::Boots, &rows, &committed)
            ),
            Some(tool_message(
                Code::PlayerTableMissing,
                Scope::Run,
                Disposition::Keep,
                vec![
                    ("table", "BootsList.bin".to_owned()),
                    ("rows", "3".to_owned())
                ],
            ))
        );
        assert_eq!(left_out(ItemTable::Gloves, &rows, &committed), 1);
        assert_eq!(
            table_missing(
                ItemTable::Gloves.file(),
                left_out(ItemTable::Gloves, &rows, &committed)
            ),
            Some(tool_message(
                Code::PlayerTableMissing,
                Scope::Run,
                Disposition::Keep,
                vec![
                    ("table", "GloveList.bin".to_owned()),
                    ("rows", "1".to_owned())
                ],
            ))
        );
        assert_eq!(table_missing(ItemTable::Boots.file(), 0), None);
        let message = table_missing(
            ItemTable::Boots.file(),
            left_out(ItemTable::Boots, &rows, &committed),
        )
        .unwrap();
        assert_eq!(message.severity, studio_core::Severity::Warning);
    }
}
