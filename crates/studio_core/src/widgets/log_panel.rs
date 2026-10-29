//! The log panel, the draft `docs/prototypes/index.html`'s `logPanel`: a `BottomDock`
//! titled `Log` holding a run's newest `LOG_CAP` lines. They are colored by severity,
//! filterable, and stuck to the bottom until the member scrolls up; each finding's
//! message code links to its row in the help. The other half of the draft's
//! cross-linking, a line click highlighting its grid cell, waits for the grid itself:
//! only the code is clickable here.

use std::cell::Cell;

use super::BottomDock;
use crate::events::{EventLines, MessageCode, PipelineEvent, Severity};

/// The most lines the body draws (the draft's `LOG_CAP`): egui's text rendering is
/// not made for thousands of widgets, so only the newest show; full logs are the
/// tool's own files on disk.
const LOG_CAP: usize = 300;

/// The jump button's margin from the body's bottom-right corner, in points.
const JUMP_MARGIN: f32 = 10.0;

/// One drawn line: its severity (color and filter), the message code its code part
/// links to the help for, and the text as `EventLines` prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// Color and filter bucket.
    pub severity: Severity,
    /// The finding's code, when the line carries one: drawn as the help link.
    pub code: Option<MessageCode>,
    /// The whole line, monospace, exactly as the console prints it.
    pub text: String,
}

impl LogLine {
    /// The line `event` shows in the log, or `None` for events the log does not
    /// show. A `Message` keeps its severity and code (the help link); any other
    /// shown event is `Info` and codeless. `lines` is the run's formatter: events
    /// it does not show (an `ExportStarted`) still update it.
    pub fn from_event(lines: &mut EventLines, event: &PipelineEvent) -> Option<LogLine> {
        let text = lines.line(event)?;
        let (severity, code) = if let PipelineEvent::Message(message) = event {
            (message.severity, Some(message.code.clone()))
        } else {
            (Severity::Info, None)
        };
        Some(LogLine {
            severity,
            code,
            text,
        })
    }

    /// A line no event carries (the run's exit line).
    pub fn plain(severity: Severity, text: impl Into<String>) -> LogLine {
        LogLine {
            severity,
            code: None,
            text: text.into(),
        }
    }
}

/// The header's filter chips (the draft's `LOG_FILTERS`): which severities the body
/// shows. A filter hides lines; it never drops them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Filter {
    /// Every line.
    All,
    /// Warning, error and fatal.
    Warnings,
    /// Error and fatal.
    Errors,
}

impl Filter {
    /// `(filter, chip label)` in header order.
    const CHIPS: [(Filter, &'static str); 3] = [
        (Filter::All, "All"),
        (Filter::Warnings, "Warnings"),
        (Filter::Errors, "Errors"),
    ];

    /// Whether a line of `severity` is drawn under this filter.
    fn shows(self, severity: Severity) -> bool {
        match self {
            Filter::All => true,
            Filter::Warnings => severity >= Severity::Warning,
            Filter::Errors => severity >= Severity::Error,
        }
    }
}

/// The body's auto-scroll state (the draft's `logStick`/`logNew`): whether the view
/// follows its newest line, the line count when it left the bottom (the `↓ N new`
/// button's base), the count last seen (a shorter one is a new run's cleared log),
/// and the pending snap the jump button asked for: `stick_to_bottom` only holds a
/// bottom the view never left, it cannot pull back to one.
struct Follow {
    stick: bool,
    left_at: usize,
    seen: usize,
    snap: bool,
}

/// The docked log: the `BottomDock` frame plus the view's own state — the severity
/// filter and the follow state. The lines live with the tool.
pub struct LogPanel {
    dock: BottomDock,
    /// The body scroll area's salt, derived from the dock's, so two tools' panels keep
    /// their scroll and dock heights apart: the Export upgrader has a log of its own.
    salt: egui::Id,
    filter: Filter,
    follow: Follow,
}

impl LogPanel {
    /// A panel titled `Log` whose egui state (the dock height, the scroll offset)
    /// is keyed under `id_salt`: pass the tool's own id, or a salt of its own. Its
    /// body opens 180 points high (the drafts' `logHeight`).
    pub fn new(id_salt: impl egui::AsIdSalt) -> LogPanel {
        let id = egui::Id::unique(id_salt);
        LogPanel {
            dock: BottomDock::new(id, "Log", "the log", 180.0),
            salt: id.with("body"),
            filter: Filter::All,
            follow: Follow {
                stick: true,
                left_at: 0,
                seen: 0,
                snap: false,
            },
        }
    }

    /// Draws the panel over `lines` (oldest first) and returns the message code
    /// whose help link was clicked this frame.
    pub fn show(&mut self, ui: &mut egui::Ui, lines: &[LogLine]) -> Option<MessageCode> {
        // A shorter `lines` than last frame is a new run's cleared log: the jump
        // counter starts over.
        if lines.len() < self.follow.seen {
            self.follow.left_at = lines.len();
        }
        self.follow.seen = lines.len();
        // The header's chips and the body both read the filter this frame, but the
        // chips set it: it crosses through a cell so the count and the body see the
        // click's result in the same frame.
        let filter = Cell::new(self.filter);
        let mut clicked = None;
        self.dock.show(
            ui,
            |ui| {
                for (choice, label) in Filter::CHIPS {
                    if ui.selectable_label(filter.get() == choice, label).clicked() {
                        filter.set(choice);
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(count_text(filter.get(), lines));
                });
            },
            |ui| {
                body(
                    &mut self.follow,
                    self.salt,
                    ui,
                    lines,
                    filter.get(),
                    &mut clicked,
                )
            },
        );
        self.filter = filter.get();
        clicked
    }
}

/// The body: the capped tail of the filtered lines in a both-ways scroll area (the
/// lines do not wrap — a long one scrolls sideways), and the jump button over its
/// bottom-right while new lines wait unseen.
fn body(
    follow: &mut Follow,
    salt: egui::Id,
    ui: &mut egui::Ui,
    lines: &[LogLine],
    filter: Filter,
    clicked: &mut Option<MessageCode>,
) {
    // A paused view draws only the lines present when the pause began
    // (`lines[..follow.left_at]`): later arrivals raise the `↓ N new` count
    // without moving the text under a member reading scrolled-up lines.
    let pool = if follow.stick {
        lines
    } else {
        &lines[..follow.left_at]
    };
    let shown = pool
        .iter()
        .filter(|line| filter.shows(line.severity))
        .count();
    let skip = shown.saturating_sub(LOG_CAP);
    let output = well_frame(ui.visuals())
        .show(ui, |ui| {
            egui::ScrollArea::both()
                .id_salt(salt)
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                // The jump button's scroll lands at once; the member's wheel input
                // keeps egui's own smoothing, which is a different path.
                .animated(false)
                .show(ui, |ui| {
                    // Labels in a vertical layout wrap by default
                    // (`Ui::wrap_mode`); a long line scrolls sideways instead.
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    for line in pool
                        .iter()
                        .filter(|line| filter.shows(line.severity))
                        .skip(skip)
                    {
                        if let Some(code) = show_line(ui, line) {
                            *clicked = Some(code);
                        }
                    }
                    if follow.snap {
                        ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    }
                })
        })
        .inner;
    follow.snap = false;
    // egui's own criterion for `scroll_stuck_to_end` (scroll_area.rs): the offset
    // at exactly the clamped end, or content fitting the view. A wheel back down
    // that stops a fraction short leaves the bottom too — the button must show.
    let max_offset = output.content_size.y - output.inner_rect.height();
    let at_bottom = output.state.clamped_offset().y == max_offset || max_offset < 0.0;
    if follow.stick && !at_bottom {
        follow.left_at = lines.len();
    }
    follow.stick = at_bottom;
    let new = lines.len().saturating_sub(follow.left_at);
    if !follow.stick && new > 0 {
        // A bottom-up, right-aligned child over the body's bottom-right sizes and
        // places the button itself.
        let button = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .max_rect(output.inner_rect.shrink(JUMP_MARGIN))
                    .layout(egui::Layout::bottom_up(egui::Align::Max)),
                |ui| ui.button(format!("↓ {new} new")),
            )
            .inner;
        if button.on_hover_text("Jump to the newest line").clicked() {
            follow.stick = true;
            follow.left_at = lines.len();
            follow.snap = true;
        }
    }
}

/// The log panel's well, the drafts' `.log-body` and `.live-body`: the extreme fill
/// and the window stroke, 6 points of margin beside the rows and 2 above and below.
/// A dock's body draws its rows inside it.
pub fn well_frame(visuals: &egui::Visuals) -> egui::Frame {
    egui::Frame::new()
        .fill(visuals.extreme_bg_color)
        .stroke(visuals.window_stroke())
        .inner_margin(egui::Margin::symmetric(6, 2))
}

/// The header's count line (the draft's `log-count`): `last 300 of N` when the
/// shown lines overflow the cap, `N of M lines` when a filter hides some, `N lines`
/// when everything shows.
fn count_text(filter: Filter, lines: &[LogLine]) -> String {
    let shown = lines
        .iter()
        .filter(|line| filter.shows(line.severity))
        .count();
    if shown > LOG_CAP {
        format!("last {LOG_CAP} of {shown}")
    } else if shown < lines.len() {
        format!("{shown} of {} lines", lines.len())
    } else {
        format!("{shown} lines")
    }
}

/// A line's color (the draft's `warn`/`err` classes): warning amber, error and
/// fatal red, info the normal text color.
fn line_color(severity: Severity, visuals: &egui::Visuals) -> Option<egui::Color32> {
    match severity {
        Severity::Info => None,
        Severity::Warning => Some(visuals.warn_fg_color),
        Severity::Error | Severity::Fatal => Some(visuals.error_fg_color),
    }
}

/// One line of the body: monospace, severity-colored, one row. A line with a code
/// draws the code as a link to its catalog row in the help (ST-HELP-03), found as
/// the first `"<severity name> <code>"` in the `EventLines` text; the rest of the
/// text is inert while the grid cross-link is unbuilt. Returns the clicked code.
fn show_line(ui: &mut egui::Ui, line: &LogLine) -> Option<MessageCode> {
    let color = line_color(line.severity, ui.visuals());
    let rich = |text: &str| {
        let text = egui::RichText::new(text).monospace();
        match color {
            Some(color) => text.color(color),
            None => text,
        }
    };
    let Some(code) = &line.code else {
        ui.label(rich(&line.text));
        return None;
    };
    let needle = format!("{} {}", line.severity.name(), code.code);
    let Some(start) = line.text.find(&needle) else {
        ui.label(rich(&line.text));
        return None;
    };
    let code_start = start + line.severity.name().len() + 1;
    let code_end = code_start + code.code.len();
    let mut clicked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label(rich(&line.text[..code_start]));
        if ui
            .link(egui::RichText::new(&line.text[code_start..code_end]).monospace())
            .on_hover_text("Open the help at this message")
            .clicked()
        {
            clicked = Some(code.clone());
        }
        ui.label(rich(&line.text[code_end..]));
    });
    clicked
}

#[cfg(test)]
mod tests;
