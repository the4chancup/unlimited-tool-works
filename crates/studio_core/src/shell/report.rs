//! The "Report a problem" dialog the sidebar's `⚠` tail entry opens, as the
//! draft `docs/prototypes/index.html` `drawReport` draws it: the member's own
//! words and the lines saying what the bundle holds, then Save…, which zips,
//! to the path the member picks, those words, the session's journal and kept
//! files, every tool's state, the log, the session's settings, the installs
//! and a `README.txt` saying how to read them. The words stay for the window's
//! life, so a reopened dialog holds them; a written bundle leaves only
//! Close, and a failed write shows its error and keeps Save…. Native only:
//! bundles live on disk.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Context;
use directories::BaseDirs;
use pes_version::PesVersion;

use crate::redact::redact_home;
use crate::repro::{
    EarlierSession, Recorder, SessionEnd, ToolSummary, earlier_sessions, session_bundle,
};
use crate::tool::{ExeCheck, StudioTool, ToolContext};
use crate::widgets::{checkbox, modal, modal_buttons, modal_sentence_gap, text_box};

/// The body's width, the draft's `.window div` 540 points.
const WIDTH: f32 = 540.0;
/// The bytes in one MB, as the "opened" clause's size counts them (the update
/// dialog's download line writes them the same way).
const MEGABYTE: f64 = 1_048_576.0;
/// The bytes in one KB, the "opened" clause's unit under 0.1 MB.
const KILOBYTE: f64 = 1024.0;
/// The text box's height, the draft's `textarea` 84 points.
const WORDS_HEIGHT: f32 = 84.0;
/// `README.txt`'s core part: what each part of the bundle holds. The
/// tools' own `repro_readme` parts are appended under it.
const README_CORE: &str = include_str!("report_readme.txt");

/// The dialog's state: open or not, the member's words (kept for the window's
/// life, so a reopened dialog holds them), the lines under "The file holds:"
/// as built at the opening, the earlier sessions the "Include earlier
/// sessions" box lists — built with them — and its tick, the bundle last
/// written or the write's error — both cleared by a reopening — the member's
/// home folder for the `~` rewrite, and the Save… file picker, which the
/// tests replace (a native dialog cannot be clicked headless; the tools'
/// pickers are stubbed the same way).
pub(super) struct Report {
    /// The modal is drawn while set.
    open: bool,
    /// The member's own words, the text box's content.
    words: String,
    /// The lines under "The file holds:", built when the dialog opens:
    /// reading the journal and the system's time zone is work no frame
    /// should repeat. The dialog is modal, so no tool records while it is
    /// up.
    lines: Vec<String>,
    /// The kept sessions but the window's own, the "Include earlier
    /// sessions" box's list, built when the dialog opens: the listing
    /// reads each session's journal and `outputs/` folder — work no frame
    /// should repeat either. Newest first, as the box lists them and the
    /// bundle packs them.
    sessions: Vec<EarlierSession>,
    /// The "Include earlier sessions" box's tick: unticked at every
    /// opening.
    include_earlier: bool,
    /// The "Saved to …" line of the last write, shown under the list; `None`
    /// until the first one.
    saved_to: Option<String>,
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
            lines: Vec::new(),
            sessions: Vec::new(),
            include_earlier: false,
            saved_to: None,
            error: None,
            home: BaseDirs::new().map(|dirs| dirs.home_dir().display().to_string()),
            save_dialog: default_save_dialog(),
        }
    }

    /// Opens the dialog, fresh for a new report: the lines are built from
    /// `tools` and what `ctx`'s recorder holds now, and the earlier
    /// sessions listed from `<data location>/sessions` — the window's
    /// own, named `own_session`, left out. The words stay (a member may
    /// reopen mid-writing), the last write's mark and error do not, and
    /// the box starts unticked.
    pub(super) fn open(
        &mut self,
        tools: &[Box<dyn StudioTool>],
        ctx: &ToolContext,
        own_session: Option<&str>,
    ) {
        self.open = true;
        self.lines = lines(tools, ctx.recorder());
        self.sessions = ctx
            .paths()
            .data_dir
            .as_deref()
            .map(|dir| earlier_sessions(&dir.join("sessions"), own_session))
            .unwrap_or_default();
        self.include_earlier = false;
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
        let modal = modal(
            egui_ctx,
            egui::Id::unique("report-a-problem"),
            crate::shell::sidebar::REPORT_LABEL,
            |ui| {
                ui.set_width(WIDTH);
                ui.scope(|ui| {
                    // The draft's text rows stand 15 apart, one under the
                    // other, so egui's 3-point widget gap is dropped for
                    // them; the sentence gaps stay `modal_sentence_gap`'s.
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(
                        "Saves what UTW did since it opened, so the problem can be reproduced.",
                    );
                    ui.label(
                        "The file is not sent automatically, please post it wherever you usually \
                         report problems.",
                    );
                    modal_sentence_gap(ui);
                    ui.label("What happened?");
                    ui.add(text_box(
                        egui::TextEdit::multiline(&mut self.words)
                            .hint_text(
                                "What you did, what you expected and what you got; also anything \
                                 done outside UTW, in the game or another tool",
                            )
                            .desired_width(f32::INFINITY)
                            .min_size(egui::vec2(0.0, WORDS_HEIGHT)),
                    ));
                    modal_sentence_gap(ui);
                    ui.label("The file holds:");
                    // The draft's `ul` keeps its first line 4 under the
                    // heading.
                    ui.add_space(4.0);
                    for line in &self.lines {
                        // The draft's `<li>`: the bullet at the first line's top,
                        // a wrapped line hanging under the text.
                        ui.horizontal_top(|ui| {
                            ui.label("•");
                            ui.add(egui::Label::new(line.as_str()).wrap());
                        });
                    }
                    modal_sentence_gap(ui);
                    if !self.sessions.is_empty() {
                        checkbox(ui, &mut self.include_earlier, "Include earlier sessions")
                            .on_hover_text(
                                "Tick it when the problem began before UTW was last \
                                 opened: a session that crashed, or a save you tried in \
                                 the game after closing UTW.",
                            );
                        // The draft's sessions `ul` keeps its first line 4
                        // under the box, its rows in the weak colour.
                        ui.add_space(4.0);
                        for session in &self.sessions {
                            ui.horizontal_top(|ui| {
                                ui.weak("•");
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(session_line(session)).weak(),
                                    )
                                    .wrap(),
                                );
                            });
                        }
                        modal_sentence_gap(ui);
                    }
                    ui.label("Your user folder is written as ~ in the file's paths.");
                    if let Some(saved_to) = &self.saved_to {
                        modal_sentence_gap(ui);
                        ui.label(saved_to.as_str());
                    }
                });
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

    /// Save…'s work: the picker answers a path (a cancel writes nothing),
    /// then `write_report` writes the bundle there. The outcome lands in
    /// `saved_to` or `error`.
    fn save(&mut self, tools: &[Box<dyn StudioTool>], ctx: &ToolContext, log_file: Option<&Path>) {
        let Some(out) = (self.save_dialog)(&bundle_name(&jiff::Zoned::now().datetime())) else {
            return;
        };
        let home = self.home.as_deref();
        let sessions = if self.include_earlier {
            self.sessions.as_slice()
        } else {
            &[]
        };
        match write_report(&out, tools, ctx, log_file, &self.words, home, sessions) {
            Ok(()) => {
                self.saved_to = Some(format!(
                    "Saved to {}",
                    redact_home(&out.display().to_string(), home)
                ));
                self.error = None;
            }
            Err(error) => self.error = Some(format!("{error:#}")),
        }
    }
}

/// The bundle at `out`: `write_bundle` with the window's log, the session's
/// settings, `installs.txt` and `README.txt` at the zip's root, and every
/// tool's `repro_state` under its id in `state`; each of `sessions` — the
/// ones the member ticked — goes in as `sessions/<folder name>.zip`, a
/// bundle of its own, named under `missing` when it fails. The settings are
/// the ones the session runs under, not the file on disk, which a failed
/// settings save leaves out of date.
fn write_report(
    out: &Path,
    tools: &[Box<dyn StudioTool>],
    ctx: &ToolContext,
    log_file: Option<&Path>,
    words: &str,
    home: Option<&str>,
    sessions: &[EarlierSession],
) -> anyhow::Result<()> {
    // `utw.log` is named even when the run has no log or it cannot be
    // read, so the journal's `missing` says why the bundle lacks it. A
    // session whose bundle fails is named the same way: the rest of the
    // report is still written.
    let log = log_file.and_then(|path| fs::read(path).ok());
    let settings = ctx.settings().lock().unwrap().to_table();
    let settings = settings
        .and_then(|table| toml::to_string_pretty(&table))
        .context("cannot write the settings for the bundle")?;
    let installs = installs(ctx);
    let readme = readme(tools);
    let session_files: Vec<(String, Option<Vec<u8>>)> = sessions
        .iter()
        .map(|session| {
            let name = format!("sessions/{}.zip", file_name(&session.folder));
            let bytes = match session_bundle(&session.folder, home) {
                Ok(bytes) => Some(bytes),
                Err(error) => {
                    log::warn!(
                        "{}: the earlier session's bundle could not be written: {error:#}",
                        session.folder.display()
                    );
                    None
                }
            };
            (name, bytes)
        })
        .collect();
    let mut files: Vec<(&str, Option<&[u8]>)> = vec![
        ("utw.log", log.as_deref()),
        ("settings.toml", Some(settings.as_bytes())),
        ("installs.txt", Some(installs.as_bytes())),
        ("README.txt", Some(readme.as_bytes())),
    ];
    for (name, bytes) in &session_files {
        files.push((name.as_str(), bytes.as_deref()));
    }
    let mut state: Vec<(String, Vec<u8>)> = Vec::new();
    for tool in tools {
        for (name, bytes) in tool.repro_state() {
            state.push((format!("{}/{}", tool.id(), name), bytes));
        }
    }
    let state: Vec<(&str, &[u8])> = state
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
        .collect();
    ctx.recorder()
        .write_bundle(out, words, &files, &state, home)
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

/// One tool's line after its label: `<n> steps` (`1 step`), then `; opened
/// <file name> (<size>)` — `, and <k> more` when it opened others — and
/// `; wrote <file name> at <hh:mm>`. A clause with nothing to name is left
/// out.
fn summary_text(summary: &ToolSummary) -> String {
    let mut text = match summary.steps {
        1 => "1 step".to_owned(),
        steps => format!("{steps} steps"),
    };
    if let Some((path, size)) = &summary.first_input {
        text += &format!("; opened {} ({})", file_name(path), size_text(*size));
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

/// One session's line under the "Include earlier sessions" box, as the
/// draft's: `<d Mon hh:mm>: opened <name>; wrote at <hh:mm>; <ending>` —
/// "opened nothing" and "wrote nothing" for the missing parts. The ending
/// is `closed`, `crashed` or `stopped without closing` (`ended`), or
/// `ending unknown` on a journal with no `[end]`, which may be a session
/// still open in another window.
fn session_line(session: &EarlierSession) -> String {
    let started = session.started.strftime("%-d %b %H:%M");
    let opened = match &session.opened {
        Some(name) => format!("opened {name}"),
        None => "opened nothing".to_owned(),
    };
    let wrote = match session.wrote {
        Some(when) => format!("wrote at {}", clock_text(local_clock(when))),
        None => "wrote nothing".to_owned(),
    };
    let ended = match session.ended {
        Some(SessionEnd::Closed) => "closed",
        Some(SessionEnd::Crashed) => "crashed",
        Some(SessionEnd::Ended) => "stopped without closing",
        None => "ending unknown",
    };
    format!("{started}: {opened}; {wrote}; {ended}")
}

/// `size` bytes as the "opened" clause writes them: `7.4 MB`, or rounded KB
/// (`25 KB`) under 0.1 MB, where MB would read `0.0` for a rules file.
fn size_text(size: usize) -> String {
    let size = size as f64;
    if size < MEGABYTE / 10.0 {
        format!("{:.0} KB", size / KILOBYTE)
    } else {
        format!("{:.1} MB", size / MEGABYTE)
    }
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

/// `README.txt`: the core part, then each tool's `repro_readme` part in
/// `tools`' order, a blank line before it.
fn readme(tools: &[Box<dyn StudioTool>]) -> String {
    let mut readme = README_CORE.to_owned();
    for tool in tools {
        if let Some(part) = tool.repro_readme() {
            readme.push('\n');
            readme.push_str(part);
        }
    }
    readme
}

#[cfg(test)]
mod tests;
