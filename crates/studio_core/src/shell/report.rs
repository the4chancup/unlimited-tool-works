//! The "Report a problem" dialog the sidebar's `⚠` tail entry opens, as the
//! draft `docs/prototypes/index.html` `drawReport` draws it: the member's own
//! words and the lines saying what the bundle holds, then Save…, which zips the
//! session's journal and kept files, every tool's state, the log, the settings
//! and the installs to the path the member picks. The words stay for the
//! window's life, so a reopened dialog holds them; a written bundle leaves
//! only Close, and a failed write shows its error and keeps Save…. Native
//! only: bundles live on disk.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use directories::BaseDirs;
use pes_version::PesVersion;

use crate::redact::redact_home;
use crate::repro::{Recorder, ToolSummary};
use crate::settings::SETTINGS_FILE_NAME;
use crate::tool::{ExeCheck, StudioTool, ToolContext};
use crate::widgets::{modal, modal_buttons, modal_sentence_gap, text_box};

/// The body's width, the draft's `.window div` 540 points.
const WIDTH: f32 = 540.0;
/// The bytes in one MB, as the "opened" clause's size counts them (the update
/// dialog's download line writes them the same way).
const MEGABYTE: f64 = 1_048_576.0;
/// The text box's height in rows, the draft's textarea 84 points.
const WORDS_ROWS: usize = 4;

/// The dialog's state: open or not, the member's words (kept for the window's
/// life, so a reopened dialog holds them), the bundle last written or the
/// write's error — both cleared by a reopening — the member's home folder for
/// the `~` rewrite, and the Save… file picker, which the tests replace (a
/// native dialog cannot be clicked headless; the tools' pickers are stubbed
/// the same way).
pub(super) struct Report {
    /// The modal is drawn while set.
    open: bool,
    /// The member's own words, the text box's content.
    words: String,
    /// The path the last write went to, shown under the list; `None` until the
    /// first one.
    saved_to: Option<PathBuf>,
    /// The failed write's error, shown under the buttons; `None` while there
    /// is none to show.
    error: Option<String>,
    /// The member's home folder, written `~` in the bundle and in the "Saved
    /// to" line: `BaseDirs`' in the window, a test's fake.
    home: Option<String>,
    /// The Save… picker: `rfd`'s save dialog in the window, a test's stub.
    save_dialog: SaveDialog,
}

/// The Save… file dialog: `rfd`'s save box (the suggested file name in, the
/// picked path or `None` out — `None` is the member's cancel), stubbed in
/// tests, as the tools' pickers are.
type SaveDialog = Box<dyn FnMut(&str) -> Option<PathBuf>>;

/// The real Save… dialog: `rfd` on `zip` files, the suggested name prefilled.
#[cfg(not(test))]
fn default_save_dialog() -> SaveDialog {
    Box::new(|suggested| {
        rfd::FileDialog::new()
            .add_filter("zip", &["zip"])
            .set_file_name(suggested)
            .save_file()
    })
}

/// `default_save_dialog` under `cargo test`: a test that reaches it forgot to
/// stub `report.save_dialog`.
#[cfg(test)]
fn default_save_dialog() -> SaveDialog {
    Box::new(|_| panic!("a test reached a real file dialog: stub report.save_dialog"))
}

impl Report {
    /// The dialog's starting state: closed, no words, the real picker and the
    /// member's real home.
    fn new() -> Report {
        Report {
            open: false,
            words: String::new(),
            saved_to: None,
            error: None,
            home: BaseDirs::new().map(|dirs| dirs.home_dir().display().to_string()),
            save_dialog: default_save_dialog(),
        }
    }

    /// Opens the dialog, fresh for a new report: the words stay (a member may
    /// reopen mid-writing), the last write's mark and error do not.
    pub(super) fn open(&mut self) {
        self.open = true;
        self.saved_to = None;
        self.error = None;
    }

    /// Hides the dialog without an answer, as the shell closes its other
    /// modals (a held close, the help window opening).
    pub(super) fn close(&mut self) {
        self.open = false;
    }

    /// Draws the dialog while it is up. `log_file` is the window's log's path,
    /// `None` when the run has none (a first run before the location's
    /// choice): the bundle then carries no `utw.log`.
    pub(super) fn show(
        &mut self,
        egui_ctx: &egui::Context,
        tools: &[Box<dyn StudioTool>],
        ctx: &ToolContext,
        log_file: Option<&Path>,
    ) {
        if !self.open {
            return;
        }
        let lines = lines(tools, ctx.recorder());
        let modal = modal(
            egui_ctx,
            egui::Id::unique("report-a-problem"),
            crate::shell::sidebar::REPORT_LABEL,
            |ui| {
                ui.set_width(WIDTH);
                ui.label(format!(
                    "Saves what {} did since it opened, so the problem can be reproduced.",
                    crate::EXE_NAME
                ));
                ui.label(
                    "The file is not sent automatically, please post it wherever you usually \
                     report problems.",
                );
                modal_sentence_gap(ui);
                ui.label("What happened?");
                ui.add(text_box(
                    egui::TextEdit::multiline(&mut self.words)
                        .hint_text(format!(
                            "What you did, what you expected and what you got; also anything \
                             done outside {}, in the game or another tool",
                            crate::EXE_NAME
                        ))
                        .desired_width(f32::INFINITY)
                        .desired_rows(WORDS_ROWS),
                ));
                modal_sentence_gap(ui);
                ui.label("The file holds:");
                for line in &lines {
                    // The draft's `<li>`: the bullet carries the row, a
                    // wrapped line hangs under the text.
                    ui.horizontal(|ui| {
                        ui.label("•");
                        ui.label(line);
                    });
                }
                modal_sentence_gap(ui);
                ui.label("Your username is rewritten as ~.");
                if let Some(path) = &self.saved_to {
                    modal_sentence_gap(ui);
                    ui.label(format!(
                        "Saved to {}",
                        redact_home(&path.display().to_string(), self.home.as_deref())
                    ));
                }
                // Reading order Save…, Cancel; `modal_buttons` adds them from
                // the right, so the last one comes first.
                let (save, close) = if self.saved_to.is_some() {
                    (false, modal_buttons(ui, |ui| ui.button("Close").clicked()))
                } else {
                    modal_buttons(ui, |ui| {
                        let cancel = ui.button("Cancel").clicked();
                        let save = ui.button("Save…").clicked();
                        (save, cancel)
                    })
                };
                if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                }
                (save, close)
            },
        );
        let (save, close) = modal.inner;
        if modal.should_close() || close {
            self.open = false;
        } else if save {
            self.save(tools, ctx, log_file);
        }
    }

    /// Save…'s work: the picker answers a path (a cancel writes nothing), then
    /// `write_bundle` with the window's log and the settings file in `files`,
    /// every tool's `repro_state` prefixed by its id and `installs.txt` in
    /// `state`. The outcome lands in `saved_to` or `error`.
    fn save(&mut self, tools: &[Box<dyn StudioTool>], ctx: &ToolContext, log_file: Option<&Path>) {
        let Some(out) = (self.save_dialog)(&bundle_name(&jiff::Zoned::now().datetime())) else {
            return;
        };
        let mut files: Vec<(&str, &Path)> = Vec::new();
        if let Some(path) = log_file {
            files.push(("utw.log", path));
        }
        let settings = ctx
            .paths()
            .data_dir
            .as_ref()
            .map(|dir| dir.join(SETTINGS_FILE_NAME));
        if let Some(path) = &settings {
            files.push(("settings.toml", path));
        }
        let mut state: Vec<(String, Vec<u8>)> = Vec::new();
        for tool in tools {
            for (name, bytes) in tool.repro_state() {
                state.push((format!("{}/{}", tool.id(), name), bytes));
            }
        }
        state.push(("installs.txt".to_owned(), installs(ctx).into_bytes()));
        let state: Vec<(&str, &[u8])> = state
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
            .collect();
        match ctx
            .recorder()
            .write_bundle(&out, &self.words, &files, &state, self.home.as_deref())
        {
            Ok(()) => {
                self.saved_to = Some(out);
                self.error = None;
            }
            Err(error) => self.error = Some(format!("{error:#}")),
        }
    }
}

impl Default for Report {
    fn default() -> Report {
        Report::new()
    }
}

/// The list under "The file holds:": one line per tool that recorded
/// anything, in sidebar order, then the fixed lines. A tool that recorded
/// nothing is not listed; the exports' line is not drawn yet — no tool opens
/// exports.
fn lines(tools: &[Box<dyn StudioTool>], recorder: &Recorder) -> Vec<String> {
    let summaries = recorder.tool_summaries();
    let mut lines = Vec::new();
    for tool in tools {
        let Some(summary) = summaries.iter().find(|summary| summary.tool == tool.id()) else {
            continue;
        };
        lines.push(format!("{}: {}", tool.label(), summary_text(summary)));
    }
    lines.push("the files as they were opened, before any change".to_owned());
    lines.push("utw.log, the version, your settings and the PES installs found".to_owned());
    lines
}

/// One tool's line after its label: `<n> steps`, then `; opened <file name>
/// (<size> MB)` — `, and <k> more` when it opened others — and `; wrote <file
/// name> at <hh:mm>`. A clause with nothing to name is left out.
fn summary_text(summary: &ToolSummary) -> String {
    let mut text = format!("{} steps", summary.steps);
    if let Some((path, size)) = &summary.first_input {
        text += &format!(
            "; opened {} ({:.1} MB)",
            file_name(path),
            *size as f64 / MEGABYTE
        );
        if summary.inputs > 1 {
            text += &format!(", and {} more", summary.inputs - 1);
        }
    }
    if let Some((path, written)) = &summary.last_output {
        text += &format!(
            "; wrote {} at {}",
            file_name(path),
            clock_text(local_clock(*written))
        );
    }
    text
}

/// `path`'s file name, the whole path where it has none (a drive root).
fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// `when`'s clock time, `hh:mm` in the local zone (`02:41`).
fn local_clock(when: SystemTime) -> jiff::civil::Time {
    jiff::Timestamp::try_from(when)
        .expect("a SystemTime the program made is inside jiff's timestamp range")
        .to_zoned(jiff::tz::TimeZone::system())
        .time()
}

/// `when` as `hh:mm`, 24-hour (`02:41`).
fn clock_text(when: jiff::civil::Time) -> String {
    format!("{:02}:{:02}", when.hour(), when.minute())
}

/// The suggested file name at `when` (`utw-report-2026-11-07-0245.zip`), so
/// reports sort by when they were made.
fn bundle_name(when: &jiff::civil::DateTime) -> String {
    format!(
        "utw-report-{:04}-{:02}-{:02}-{:02}{:02}.zip",
        when.year(),
        when.month(),
        when.day(),
        when.hour(),
        when.minute()
    )
}

/// `installs.txt`'s text: one line per PES version the selector offers — the
/// folder the settings resolve for it and what the exe check found there,
/// or `no folder set` alone while none is.
fn installs(ctx: &ToolContext) -> String {
    let common = ctx.common();
    PesVersion::ALL
        .iter()
        .map(|version| {
            let folder = common.pes_folder_for(*version);
            if folder.as_os_str().is_empty() {
                return format!("{version}: no folder set");
            }
            let found = match ctx.pes_exe_check_for(*version) {
                ExeCheck::Found => "exe found".to_owned(),
                ExeCheck::OtherVersion(other) => format!("{other}'s exe"),
                ExeCheck::NoExe => "no PES exe".to_owned(),
                ExeCheck::NoFolder => "folder not found".to_owned(),
            };
            format!("{version}: {} ({found})", folder.display())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests;
