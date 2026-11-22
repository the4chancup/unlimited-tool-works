//! The export grid: a row per export, its label and its ID, then lines of cells at fixed
//! positions, one cell per folder or file, each painted in its state's colour. The Compiler
//! draws its check this way and the Export upgrader its run's folders. The grid reads no
//! export and runs no check: a tool maps its own results to [`Row`]s and hands them over,
//! with the [`Columns`] its lines share and the [`LegendEntry`]s of its states. A cell's
//! detail window opens on hover and stays open on a click; the tool keeps its [`Detail`] and
//! carries out what [`show`] returns (a folder to open, the help at a code).

mod cells;
mod detail;
mod grid;
mod legend;
mod rows;
#[cfg(test)]
mod testing;

pub use cells::{Cell, CellState, Open, Outline, Position, Width};
pub use detail::{Detail, GridResponse};
pub use grid::show;
pub use legend::{LegendEntry, legend};
pub use rows::{ColumnGroup, Columns, IdCell, Line, Row, RowBody, Tone};
