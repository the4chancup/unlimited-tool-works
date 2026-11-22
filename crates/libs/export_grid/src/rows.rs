//! The rows a tool hands the grid, and the columns their lines share.

use crate::cells::{Cell, Position, Width};

/// One export's row: its label, its ID, and its lines or a message in their place.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// The label at the row's left. Its text is the team's name; its state and findings are
    /// the export's own: about the export as a whole, and about what has no cell of its own.
    pub label: Cell,
    /// The ID cell right of the label; `None` in a grid with no ID column.
    pub id: Option<IdCell>,
    /// What stands right of the first vertical line.
    pub body: RowBody,
}

/// An ID cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdCell {
    /// The ID, as the cell shows it.
    Known(String),
    /// An empty cell: an export with no ID (the referees').
    Blank,
    /// `???` on the error red: a team the teams list lacks.
    Missing,
}

/// What stands right of a row's first vertical line.
#[derive(Debug, Clone, PartialEq)]
pub enum RowBody {
    /// One line of cells or more, one under the other; none leaves the row its label alone.
    Lines(Vec<Line>),
    /// A message in place of the lines, in a colour of its own.
    Message {
        /// What the row says.
        text: String,
        /// The colour it says it in.
        tone: Tone,
    },
}

/// The colour of a row's message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// The warning amber.
    Warning,
    /// The error red.
    Error,
}

/// One line of a row: an optional name in its own column, then its positions, group by
/// group as [`Columns::groups`] lists them.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The line's name (`Face`, `Boots`), drawn when the grid has a line-name column.
    pub name: Option<&'static str>,
    /// Each column group's positions, left to right. A wide cell in a group of square columns
    /// takes two of them.
    pub groups: Vec<Vec<Position>>,
}

/// The columns every line of a grid shares, and their titles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Columns {
    /// The ID column's title; `None` for a grid with no ID column.
    pub id_title: Option<&'static str>,
    /// Whether a column for the lines' names stands between the ID and the first vertical line.
    pub line_names: bool,
    /// The groups of positions between the two vertical lines, left to right, a small gap
    /// between two groups.
    pub groups: &'static [ColumnGroup],
    /// The title of the column right of the second vertical line.
    pub last_title: &'static str,
}

/// A group of columns between the vertical lines (`Players`, `Portraits`, `Kits`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnGroup {
    /// The title over the group: left-aligned over square columns, centred over a wide one.
    pub title: &'static str,
    /// How many columns it holds.
    pub columns: usize,
    /// How wide each column is.
    pub width: Width,
}
