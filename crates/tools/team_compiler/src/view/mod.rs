//! The Compiler's window, render-only (`spec/team_compiler/README.md` "Crate layout"): it
//! reads the tool's state (the run log, the grid's rows) and reports the user's intent
//! (Compile clicked); it never calls into the pipeline itself. It shows the export grid under
//! its legend, the toolbar under the grid, and the log panel.

mod pictures;
mod toolbar;

use std::time::Duration;

use export_grid::{
    CellState, ColumnGroup, Columns, Detail, LegendEntry, Open, Outline, PictureFile, Row, Sources,
    Width,
};
use studio_core::files::{open_folder, show_in_folder};
use studio_core::{CatalogRow, HelpTarget, LogPanel, ToolContext};

use crate::gui_run::RunLog;
use crate::messages::{self, TOOL_ID};
use crate::view::pictures::Pictures;

/// The grid's column groups: a player's 23 positions, the portraits, the 10 kits.
const GROUPS: [ColumnGroup; 3] = [
    ColumnGroup {
        title: "Players",
        columns: 23,
        width: Width::Square,
    },
    ColumnGroup {
        title: "Portraits",
        columns: 1,
        width: Width::Wide,
    },
    ColumnGroup {
        title: "Kits",
        columns: 10,
        width: Width::Square,
    },
];

/// The grid's columns: the team's ID beside its label, the groups, and the row actions'
/// column, empty until the row actions are built.
const COLUMNS: Columns = Columns {
    id_title: Some("ID"),
    line_names: false,
    groups: &GROUPS,
    last_title: "Actions",
};

/// A legend entry for `state` with `outline`, named `name`.
const fn entry(state: CellState, outline: Option<Outline>, name: &'static str) -> LegendEntry {
    LegendEntry {
        state,
        outline,
        name,
    }
}

/// The legend's columns: pairs of related states, and the two states with no pair alone.
const LEGEND: [&[LegendEntry]; 6] = [
    &[
        entry(CellState::Unchecked, None, "Unchecked"),
        entry(CellState::Checking, None, "Checking"),
    ],
    &[entry(
        CellState::PartialOk,
        None,
        "Shallow check done\n(archive)",
    )],
    &[
        entry(CellState::FullOk, Some(Outline::Warning), "Warning"),
        entry(CellState::FullOk, Some(Outline::Error), "Error"),
    ],
    &[
        entry(CellState::DoneWithWarnings, None, "Done with warnings"),
        entry(CellState::DoneWithErrors, None, "Done with errors"),
    ],
    &[
        entry(CellState::FullOk, None, "Checked"),
        entry(CellState::Done, None, "Done"),
    ],
    &[entry(CellState::Blocked, None, "Blocked")],
];

/// The grid's detail window: what it keeps from frame to frame, the catalog it explains
/// findings from, built once rather than on every frame, and the pictures of its top box.
/// It also keeps whether `show` has already decided if the log starts collapsed.
pub(crate) struct GridDetail {
    state: Detail,
    catalog: Vec<CatalogRow>,
    pictures: Pictures,
    log_start_decided: bool,
}

impl GridDetail {
    /// No window open, the Compiler's catalog, no picture read yet.
    pub(crate) fn new() -> GridDetail {
        GridDetail {
            state: Detail::default(),
            catalog: messages::catalog(),
            pictures: Pictures::default(),
            log_start_decided: false,
        }
    }

    /// The grid's rows were replaced by a new check's: the pictures are read again when next
    /// shown, since their files may have changed.
    pub(crate) fn rows_replaced(&mut self) {
        self.pictures.clear();
    }
}

/// Shows the window over `rows`, the grid's check's, drawn as being checked while `checking`,
/// with `detail`'s window over the grid; `true` when Compile was clicked this frame. A folder
/// the grid asks for opens in the file manager. At the first frame with rows the log is
/// collapsed if, open, it left the grid too little room to show them all; after that only
/// the member opens and closes it.
pub(crate) fn show(
    ui: &mut egui::Ui,
    ctx: &ToolContext,
    log: &RunLog,
    panel: &mut LogPanel,
    rows: &[Row],
    checking: bool,
    detail: &mut GridDetail,
) -> bool {
    // The dock claims the bottom first (`spec/core/gui.md` "Shell layout"), the
    // toolbar the row above it, and the legend and the grid take what is left. A
    // clicked message code is a shell request, not a pipeline call: the view
    // stays render-only.
    if let Some(code) = panel.show(ui, log.lines()) {
        ctx.open_help(HelpTarget::message(&code));
    }
    let clicked = toolbar::show(ui, panel, rows, log.is_running());
    export_grid::legend(ui, (TOOL_ID, "legend"), &LEGEND);
    let egui_ctx = ui.ctx().clone();
    detail.pictures.receive(&egui_ctx);
    let pictures = &mut detail.pictures;
    let mut sources = Sources {
        catalog: &detail.catalog,
        pictures: &mut |file: &PictureFile| pictures.texture(&egui_ctx, file),
    };
    let grid = export_grid::show(
        ui,
        (TOOL_ID, "grid"),
        &COLUMNS,
        rows,
        checking,
        &mut detail.state,
        &mut sources,
    );
    if !detail.log_start_decided && !rows.is_empty() {
        detail.log_start_decided = true;
        if grid.overflows {
            panel.collapse();
        }
    }
    if let Some(code) = grid.help {
        ctx.open_help(HelpTarget::message(&code));
    }
    match grid.open {
        Some(Open::Folder(folder)) => open_folder(&folder),
        Some(Open::Select(file)) => show_in_folder(&file),
        None => {}
    }
    if log.is_running() || checking {
        // egui repaints only on input, and the run's events and the check's rows arrive on
        // their own: without this the window would advance only when the mouse moves.
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
    clicked
}

#[cfg(test)]
mod tests;
