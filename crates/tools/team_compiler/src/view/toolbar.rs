//! The toolbar under the grid, the draft's `.tc-bar`: the strip, which sums up the grid while
//! no run is in flight, and the Compile button at its right, the Upgrade DpFileList button
//! beside it when the check found the game's list not the official one.

use export_grid::{Cell, CellState, Row, cell_places};
use studio_core::widgets::{add_enabled, menu_above, menu_separator, resting_edge};
use studio_core::{LogFilter, LogPanel, Severity, ToolContext};

use crate::cli::{self, Mode};
use crate::gui_run::RunLog;
use crate::messages::TOOL_ID;
use crate::view::Intent;

/// The room between the strip and the button (the draft's `.tc-bar` gap).
const GAP: f32 = 8.0;
/// The room between the split button's main part and its arrow (the draft's `.tc-split`
/// gap): the two meet as one control.
const SPLIT_GAP: f32 = 1.0;
/// The main part's smallest width (the draft's `.tc-split button:first-child`).
const MAIN_MIN_WIDTH: f32 = 150.0;
/// The disabled tooltip of the sideload menu entry and of the main part while its picked
/// sideload mode has no runtime on the selected PES version.
const SIDELOAD_UNSERVED_TIP: &str = "No runtime serves sideloaded files on PES 2015 and 2016";

/// The grid summed up: its rows, how many of them a Compile skips, and how many of the others
/// have an error or a warning.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Summary {
    pub(crate) exports: usize,
    pub(crate) skipped: usize,
    pub(crate) with_errors: usize,
    pub(crate) with_warnings: usize,
}

/// `rows` summed up. A row is skipped when the check dropped its export whole: it has no
/// cell past its label (set aside before its files were listed, or showing a message in
/// place of the cells), or one of its cells is blocked. It then counts in no other way, since
/// a Compile reads none of it. The others count by the colour each cell shows, its worst
/// finding's (the draft's cell status): a row is with errors when its label or one of its
/// cells shows an Error or a Fatal, and with warnings when one shows a Warning, so a row can
/// count in both. A cell holding both an Error and a Warning shows the Error alone, and counts
/// only there.
pub(crate) fn summary(rows: &[Row]) -> Summary {
    let skipped = |row: &&Row| {
        cells(row).nth(1).is_none()
            || cells(row)
                .skip(1)
                .any(|cell| cell.state == CellState::Blocked)
    };
    let having = |wanted: fn(Severity) -> bool| {
        rows.iter()
            .filter(|row| !skipped(row))
            .filter(|row| {
                cells(row).any(|cell| {
                    cell.messages
                        .iter()
                        .map(|message| message.severity)
                        .max()
                        .is_some_and(wanted)
                })
            })
            .count()
    };
    Summary {
        exports: rows.len(),
        skipped: rows.iter().filter(skipped).count(),
        with_errors: having(|worst| worst >= Severity::Error),
        with_warnings: having(|worst| worst == Severity::Warning),
    }
}

/// `row`'s label, then every cell of its lines.
fn cells(row: &Row) -> impl Iterator<Item = &Cell> {
    cell_places(row).map(|(_, cell)| cell)
}

/// A run of the strip's text. A count that is a link carries the log filter a click on it
/// picks, and its tooltip.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Part {
    pub(crate) text: String,
    pub(crate) link: Option<(LogFilter, &'static str)>,
}

/// The strip's text for `summary` (the draft's `summary`): the export count, then the rows
/// skipped, with errors and with warnings, joined by ` · `. A count of zero is left out, so a grid
/// with no rows gives no text.
pub(crate) fn strip(summary: &Summary) -> Vec<Part> {
    let counts = [
        (summary.exports, "export", "exports", None),
        (summary.skipped, "skipped", "skipped", None),
        (
            summary.with_errors,
            "with errors",
            "with errors",
            Some((LogFilter::Errors, "Show only the errors in the log")),
        ),
        (
            summary.with_warnings,
            "with warnings",
            "with warnings",
            Some((
                LogFilter::Warnings,
                "Show only the warnings and errors in the log",
            )),
        ),
    ];
    let mut parts = Vec::new();
    for (count, one, many, link) in counts {
        if count == 0 {
            continue;
        }
        if !parts.is_empty() {
            parts.push(Part {
                text: " · ".to_owned(),
                link: None,
            });
        }
        let noun = if count == 1 { one } else { many };
        parts.push(Part {
            text: format!("{count} {noun}"),
            link,
        });
    }
    parts
}

/// The main part's and the menu's label for `mode` (the draft's `MODES`).
fn mode_label(mode: Mode) -> &'static str {
    match mode {
        Mode::Normal => "Compile",
        Mode::Sideload => "Compile for sideloading",
        Mode::Test => "Test output",
    }
}

/// The arrow's menu (the draft's `modeMenu`): the three output modes, a pick writing
/// `mode` and so relabelling the main part for the session, then Open output folder
/// and Copy report. `served` is whether a sideloading runtime exists for the selected
/// PES version, worked out once in `show`. The `Intent` a non-mode entry asked for,
/// when one was picked.
fn mode_menu(ui: &mut egui::Ui, log: &RunLog, mode: &mut Mode, served: bool) -> Option<Intent> {
    if ui.button(mode_label(Mode::Normal)).clicked() {
        *mode = Mode::Normal;
        ui.close();
    }
    let sideload = add_enabled(ui, served, egui::Button::new(mode_label(Mode::Sideload)));
    let sideload = if served {
        sideload.on_hover_text("Unpacked into the game folder's livecpk")
    } else {
        sideload.on_disabled_hover_text(SIDELOAD_UNSERVED_TIP)
    };
    if sideload.clicked() {
        *mode = Mode::Sideload;
        ui.close();
    }
    if ui
        .button(mode_label(Mode::Test))
        .on_hover_text("The processed exports, unpacked in their own layout")
        .clicked()
    {
        *mode = Mode::Test;
        ui.close();
    }
    menu_separator(ui);
    let mut intent = None;
    if ui.button("Open output folder").clicked() {
        intent = Some(Intent::OpenOutputFolder);
        ui.close();
    }
    menu_separator(ui);
    if ui.button("Copy report").clicked() {
        ui.ctx().copy_text(log.report());
        ui.close();
    }
    intent
}

/// Draws the toolbar as a bottom panel of `ui`: the strip summing up `rows` while `log` has
/// no run in flight, the Compile split button (its main part running a `mode` compile,
/// disabled while one runs, in the installing mode alone while `log`'s findings say the
/// game's list is the cup's outdated one, or in the sideload mode while the selected PES
/// version has no runtime to serve it; its arrow opening the mode menu, disabled
/// while a run is in flight), and the Upgrade DpFileList button beside it while the
/// findings say the list is not the official one. A click on a count shows only its
/// lines in `panel`. The `Intent` a click asked for, when one was made this frame.
pub(crate) fn show(
    ui: &mut egui::Ui,
    ctx: &ToolContext,
    panel: &mut LogPanel,
    rows: &[Row],
    log: &RunLog,
    mode: &mut Mode,
) -> Option<Intent> {
    let running = log.is_running();
    let served = cli::sideload_served(ctx.common().pes_version);
    let frame = egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::symmetric(8, 4));
    egui::Panel::bottom((TOOL_ID, "toolbar"))
        .resizable(false)
        .frame(frame)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut intent = None;
                // The draft's `.tc-split`: the arrow at the row's right edge, the main
                // part meeting it at its left, their shared corners square. The gap
                // between them is the arrow's own advance, so SPLIT_GAP is set first.
                let radius = ui.visuals().widgets.inactive.corner_radius;
                ui.spacing_mut().item_spacing.x = SPLIT_GAP;
                let arrow = add_enabled(
                    ui,
                    !running,
                    egui::Button::new("\u{23f7}").corner_radius(egui::CornerRadius {
                        nw: 0,
                        sw: 0,
                        ..radius
                    }),
                )
                .on_hover_text("The output modes");
                ui.spacing_mut().item_spacing.x = GAP;
                // A Normal-mode run's preflight would stop it before anything is
                // compiled, so the outdated list is why the installing mode is off; the
                // loose-file modes install no CPK, and the two Warnings leave it enabled.
                // A sideload pick the version can no longer serve is off too, since its
                // click would only reach the preflight's refusal.
                let normal_off = *mode == Mode::Normal && log.dpfl_outdated();
                let sideload_off = *mode == Mode::Sideload && !served;
                let compile = add_enabled(
                    ui,
                    !running && !normal_off && !sideload_off,
                    egui::Button::new(mode_label(*mode))
                        .min_size(egui::vec2(MAIN_MIN_WIDTH, 0.0))
                        .corner_radius(egui::CornerRadius {
                            ne: 0,
                            se: 0,
                            ..radius
                        }),
                );
                let compile = if !running && normal_off {
                    compile.on_disabled_hover_text(
                        "The game's DpFileList.bin does not list this compile's CPK, so the \
                         game would not load it. Upgrade DpFileList first",
                    )
                } else if !running && sideload_off {
                    compile.on_disabled_hover_text(SIDELOAD_UNSERVED_TIP)
                } else {
                    compile
                };
                if compile.clicked() {
                    intent = Some(Intent::Compile(*mode));
                }
                if let Some(asked) =
                    menu_above(&arrow, |ui| mode_menu(ui, log, mode, served)).flatten()
                {
                    intent = Some(asked);
                }
                // Left of Compile in the right-to-left row, beside the button it
                // unblocks; gone while a run is in flight. Filled in the selection blue
                // with its edge, as a held control is, so it reads as the run's remedy
                // and not as one more button.
                if !running && log.dpfl_found() {
                    let upgrade = ui
                        .add(egui::Button::new("Upgrade DpFileList").selected(true))
                        .on_hover_text(
                            "Replace the game's DpFileList.bin with the cup's official list",
                        );
                    resting_edge(
                        ui,
                        &upgrade,
                        studio_core::theme::edge_color(
                            ui.visuals(),
                            ui.visuals().selection.bg_fill,
                        ),
                    );
                    if upgrade.clicked() {
                        intent = Some(Intent::UpgradeDpfl);
                    }
                }
                let parts = if running {
                    Vec::new()
                } else {
                    strip(&summary(rows))
                };
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for part in &parts {
                        let (response, filter) = show_part(ui, part, panel.is_expanded());
                        if let Some(filter) = filter {
                            panel.show_only(filter);
                        }
                        // A part the ellipsis cut short has filled the room: the parts
                        // after it would only add stray scraps past the cut.
                        if response
                            .intrinsic_size()
                            .is_some_and(|size| size.x > response.rect.width())
                        {
                            break;
                        }
                    }
                });
                intent
            })
            .inner
        })
        .inner
}

/// Draws `part` monospace on one line, cut short with an ellipsis where the room runs out. A
/// count that is a link is in the link blue while the log is collapsed and in the strip's own
/// colour while it is open (the log's filter chips then sit right below it), never
/// underlined. Returns the part's response and, when it was clicked, the filter it picks.
fn show_part(
    ui: &mut egui::Ui,
    part: &Part,
    log_open: bool,
) -> (egui::Response, Option<LogFilter>) {
    let text = egui::RichText::new(&part.text).monospace();
    let Some((filter, tooltip)) = part.link else {
        return (ui.add(egui::Label::new(text).truncate()), None);
    };
    let colour = if log_open {
        ui.visuals().text_color()
    } else {
        ui.visuals().hyperlink_color
    };
    let response = ui
        .add(
            egui::Label::new(text.color(colour))
                .sense(egui::Sense::click())
                .truncate(),
        )
        .on_hover_text(tooltip)
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let clicked = response.clicked().then_some(filter);
    (response, clicked)
}
