//! The toolbar under the grid, the draft's `.tc-bar`: the strip, which sums up the grid while
//! no Compile runs, and the Compile button at its right.

use export_grid::{Cell, CellState, Position, Row, RowBody};
use studio_core::widgets::add_enabled;
use studio_core::{LogFilter, LogPanel, Severity};

use crate::messages::TOOL_ID;

/// The room between the strip and the button (the draft's `.tc-bar` gap).
const GAP: f32 = 8.0;

/// The grid summed up: its rows, how many of them a Compile skips, and how many of the others
/// have an error or a warning.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Summary {
    pub(crate) exports: usize,
    pub(crate) skipped: usize,
    pub(crate) with_errors: usize,
    pub(crate) with_warnings: usize,
}

/// `rows` summed up. A row is skipped when the check dropped its export whole, its cells
/// blocked; it then counts in no other way, since a Compile reads none of it. The others count
/// by the colour each cell shows, its worst finding's (the draft's cell status): a row is with
/// errors when its label or one of its cells shows an Error or a Fatal, and with warnings when
/// one shows a Warning, so a row can count in both. A cell holding both an Error and a Warning
/// shows the Error alone, and counts only there.
pub(crate) fn summary(rows: &[Row]) -> Summary {
    let skipped = |row: &&Row| {
        cells(row)
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
    let lines = match &row.body {
        RowBody::Lines(lines) => lines.as_slice(),
        RowBody::Message { .. } => &[],
    };
    let positions = lines.iter().flat_map(|line| line.groups.iter().flatten());
    std::iter::once(&row.label).chain(positions.filter_map(|position| match position {
        Position::Cell(cell) => Some(cell.as_ref()),
        Position::Empty => None,
    }))
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

/// Draws the toolbar as a bottom panel of `ui`: the strip summing up `rows` while no Compile
/// is `running`, and the Compile button, disabled while one is. A click on a count shows only
/// its lines in `panel`. `true` when Compile was clicked.
pub(crate) fn show(ui: &mut egui::Ui, panel: &mut LogPanel, rows: &[Row], running: bool) -> bool {
    let frame = egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::symmetric(8, 4));
    egui::Panel::bottom((TOOL_ID, "toolbar"))
        .resizable(false)
        .frame(frame)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let clicked = add_enabled(ui, !running, egui::Button::new("Compile")).clicked();
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
                clicked
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
