//! The Compiler's window, render-only (`spec/team_compiler/README.md` "Crate layout"): it
//! reads the tool's state (the run log, the grid's rows) and reports the user's intent
//! (Compile clicked, the DpFileList upgrade's dialog); it never calls into the pipeline
//! itself. It shows the export grid under its legend, the toolbar under the grid, and the
//! log panel.

mod pictures;
mod toolbar;
mod upgrade_dpfl;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use export_grid::{
    CellState, ColumnGroup, Columns, Detail, LegendEntry, Open, Outline, PictureFile, Row, Sources,
    Width,
};
use studio_core::files::{open_folder, show_in_folder};
use studio_core::{CatalogRow, HelpTarget, LogPanel, Severity, ToolContext};

use crate::cli::Mode;
use crate::gui_run::RunLog;
use crate::messages::{self, TOOL_ID};
use crate::view::pictures::Pictures;

pub(crate) use upgrade_dpfl::DpflDialog;
use upgrade_dpfl::Outcome;

/// A click's ask of the tool, reported for it to act on: the view is render-only, so a
/// click never starts a run itself.
pub(crate) enum Intent {
    /// Compile in `Mode`: the split button's main part, labelled by the same mode.
    Compile(Mode),
    /// Upgrade DpFileList: the upgrade's dialog opens on its preview, or the upgrade runs
    /// at once when the preview has nothing to do ([`crate::upgrade::Preview::up_to_date`]),
    /// with nothing deleted.
    UpgradeDpfl,
    /// The dialog's Upgrade: the upgrade applies in `download`, the folder the dialog's
    /// preview was made for, the ticked dropped CPKs deleted.
    Upgrade(PathBuf, BTreeSet<String>),
    /// Open output folder: the folder a compile writes into, opened in the file manager,
    /// created first when no run has made it yet.
    OpenOutputFolder,
}

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
/// findings from, built once rather than on every frame, the pictures of its top box, and
/// the Compile button's output mode, a pick for the session alone (the next launch starts
/// on Normal; it is no setting). It also keeps whether `show` has already decided if the
/// log starts collapsed, and the Upgrade DpFileList dialog while it is open.
pub(crate) struct GridDetail {
    state: Detail,
    catalog: Vec<CatalogRow>,
    pictures: Pictures,
    log_start_decided: bool,
    /// The output mode the Compile button's main part runs and reads: `Normal` until the
    /// arrow's menu picks another, never written to the settings.
    mode: Mode,
    /// The Upgrade DpFileList dialog's state while it is open: the preview it was
    /// opened on and the member's delete ticks.
    pub(crate) dpfl_dialog: Option<DpflDialog>,
}

impl GridDetail {
    /// No window open, the Compiler's catalog, no picture read yet, the normal output mode.
    pub(crate) fn new() -> GridDetail {
        GridDetail {
            state: Detail::default(),
            catalog: messages::catalog(),
            pictures: Pictures::default(),
            log_start_decided: false,
            mode: Mode::Normal,
            dpfl_dialog: None,
        }
    }

    /// The grid's rows were replaced by a new check's: the pictures are read again when next
    /// shown, since their files may have changed.
    pub(crate) fn rows_replaced(&mut self) {
        self.pictures.clear();
    }
}

/// Shows the window over `rows`, the grid's check's, drawn as being checked while `checking`,
/// with `detail`'s windows over the grid; the `Intent` a click asked for this frame, when
/// one was. A folder the grid asks for opens in the file manager. At the first frame with
/// rows the log is collapsed if, open, it left the grid too little room to show them all;
/// after that only the member opens and closes it.
pub(crate) fn show(
    ui: &mut egui::Ui,
    ctx: &ToolContext,
    log: &RunLog,
    panel: &mut LogPanel,
    rows: &[Row],
    checking: bool,
    detail: &mut GridDetail,
) -> Option<Intent> {
    // The dock claims the bottom first (`spec/core/gui.md` "Shell layout"), the
    // toolbar the row above it, and the legend and the grid take what is left. A
    // clicked message code is a shell request, not a pipeline call: the view
    // stays render-only.
    let on_log = panel.show(ui, log.lines());
    if let Some(code) = on_log.help {
        ctx.open_help(HelpTarget::message(&code));
    }
    // A check's line acts for its cell: resting on it opens the cell's window, and a click
    // on a Warning or Error line goes to the cell. An Info takes no click.
    let pointed = on_log.hovered.and_then(|line| log.cell_of(line));
    detail.state.point_at(pointed.cloned());
    if let Some(line) = on_log.clicked
        && log.lines()[line].severity >= Severity::Warning
        && let Some(cell) = log.cell_of(line)
    {
        detail.state.go_to(cell.clone());
    }
    let mut intent = toolbar::show(ui, ctx, panel, rows, log, &mut detail.mode);
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
            // The log and the toolbar were already drawn open this frame, so the collapse
            // asks for the next one itself: the grid's scroll bar, appearing on this same
            // frame, asks too today, but nothing ties the two together.
            ui.ctx().request_repaint();
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
    if let Some(open) = detail.dpfl_dialog.as_mut() {
        match open.show(ui) {
            Outcome::Open => {}
            Outcome::Closed => detail.dpfl_dialog = None,
            Outcome::Upgrade(download, delete) => {
                detail.dpfl_dialog = None;
                intent = Some(Intent::Upgrade(download, delete));
            }
        }
    }
    if log.is_running() || checking {
        // egui repaints only on input, and the run's events and the check's rows arrive on
        // their own: without this the window would advance only when the mouse moves.
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
    intent
}

#[cfg(test)]
mod tests;
