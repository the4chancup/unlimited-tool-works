//! The update dialog, as the draft's `openUpdate`, `updateNow` and
//! `openUpdateReady` draw it. It opens on the offer: the release's version
//! line and its notes rendered as the help renders Markdown in a framed box
//! that scrolls, then Update now, Open release page, Skip this version and
//! Remind me later. Update now installs the release on a worker thread of the
//! dialog's own (`updater::install`): the dialog shows the download's progress
//! with a Cancel, then `Update ready`, or `Update failed` with the reason and a
//! Retry. No dialog opens by itself: the status bar's offer text or its Details
//! button opens it. Native only: the browser build has no update check to
//! offer.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, TryRecvError, unbounded};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::shell::version_label_of;
use crate::updater::{self, InstallError, OpenUrl, ProgramStamp, Progress, Release};
use crate::widgets::{add_enabled, modal, modal_buttons, modal_sentence_gap};

/// The bytes in one MB, as the download's line counts them.
const MEGABYTE: f64 = 1_048_576.0;

/// What `Update failed` says when the worker ended without telling how the
/// update went: it may have stopped anywhere, so nothing promises the files
/// are as they were.
const WORKER_GONE: &str = "The update stopped unexpectedly before it could finish.";

/// The dialog's state: whether it is up, the stage it shows, and the cache its
/// notes' Markdown renders through.
#[derive(Default)]
pub(super) struct UpdateDialog {
    /// The modal is drawn while set.
    open: bool,
    /// What the dialog shows, and the install while one runs.
    stage: Stage,
    /// The notes' `CommonMarkCache`.
    cache: CommonMarkCache,
}

/// Where the update stands, each stage one look of the dialog.
#[derive(Default)]
enum Stage {
    /// `Update available`: the release's notes and the four answers.
    #[default]
    Offer,
    /// `Updating to`: the install runs on its worker, the dialog shows its
    /// progress and a Cancel.
    Downloading(Install),
    /// `Update failed`: why, and Retry, Open release page and Close.
    Failed(Failure),
    /// `Update ready`: the new version is in the program's folder.
    Ready,
}

/// An install running on its worker thread.
struct Install {
    /// The worker, joined when the window closes during the install.
    worker: JoinHandle<()>,
    /// The worker's reports: its progress, then its outcome.
    reports: Receiver<Report>,
    /// The last progress reported, `None` before the first.
    progress: Option<Progress>,
    /// Set by Cancel and by the window's close: the download stops at its next
    /// read.
    cancel: Arc<AtomicBool>,
}

/// One message from the install's worker.
enum Report {
    /// How far the download is.
    Progress(Progress),
    /// The install ended, the worker with it.
    Done(Result<(), InstallError>),
}

/// What `Update failed` shows.
struct Failure {
    /// The updater's own sentence for what went wrong.
    reason: String,
    /// Whether the program's folder is known to be as it was, which the
    /// dialog then says. Not so when the old files could not be put back, or
    /// when the worker never said how it ended.
    nothing_changed: bool,
}

/// What the member answered; the shell applies it, the dialog having closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UpdateAnswer {
    /// Skip this version: store it as `skipped_version` and drop the offer.
    SkipVersion,
    /// Remind me later: only close; the offer stays. Also the offer's answer
    /// to the backdrop and Escape, and `Update failed`'s Close.
    RemindLater,
    /// `Update ready`'s Restart now.
    RestartNow,
    /// `Update ready`'s Later, also its answer to the backdrop and Escape.
    Later,
}

/// A button of the dialog, or the backdrop or Escape standing for one.
#[derive(Clone, Copy)]
enum Click {
    /// Update now, and Retry: start the install.
    UpdateNow,
    /// Open release page: the dialog stays.
    OpenPage,
    /// Cancel: the download stops, and the offer comes back once the worker
    /// has answered.
    Cancel,
    /// `Update failed`'s Close: the offer stands again, the dialog closes.
    Close,
    /// An answer for the shell; the dialog closes.
    Answer(UpdateAnswer),
}

impl UpdateDialog {
    /// Opens the dialog on the stage it is in: the status bar's offer text or
    /// its Details button.
    pub(super) fn open(&mut self) {
        self.open = true;
    }

    /// Hides the dialog without an answer, as the shell closes its other
    /// modals (a held close, the help window opening). The stage stays, a
    /// running install too.
    pub(super) fn close(&mut self) {
        self.open = false;
    }

    /// Whether an install runs or has succeeded. No update check may run
    /// then: one could replace the release under the dialog.
    pub(super) fn holds_checks(&self) -> bool {
        matches!(self.stage, Stage::Downloading(_) | Stage::Ready)
    }

    /// Stops a running install and waits for its worker, so the window
    /// closing never cuts a swap midway.
    pub(super) fn stop_install(&mut self) {
        if let Stage::Downloading(install) = std::mem::take(&mut self.stage) {
            install.cancel.store(true, Ordering::Relaxed);
            if install.worker.join().is_err() {
                log::warn!("the update's worker panicked while the window closed");
            }
        }
    }

    /// Draws the dialog while it is up and returns the member's answer when
    /// one lands, the dialog then closed. Update now and Retry install
    /// `release` into `bundle`, the program's folder, downloading through
    /// `open_url`; `stamp` is the program's file as the window found it
    /// when it started.
    pub(super) fn show(
        &mut self,
        ctx: &egui::Context,
        release: &Arc<Release>,
        bundle: &Path,
        stamp: Option<ProgramStamp>,
        open_url: &Arc<OpenUrl>,
    ) -> Option<UpdateAnswer> {
        if !self.open {
            return None;
        }
        let click = match &self.stage {
            Stage::Offer => show_offer(ctx, release, &mut self.cache),
            Stage::Downloading(install) => show_downloading(
                ctx,
                release,
                install.progress,
                install.cancel.load(Ordering::Relaxed),
            ),
            Stage::Failed(failure) => show_failed(ctx, release, failure),
            Stage::Ready => show_ready(ctx, release),
        };
        let answer = match click? {
            Click::UpdateNow => {
                self.stage = Stage::Downloading(Install::start(
                    ctx,
                    Arc::clone(release),
                    bundle.to_owned(),
                    stamp,
                    Arc::clone(open_url),
                ));
                return None;
            }
            Click::OpenPage => {
                ctx.open_url(egui::OpenUrl::same_tab(&release.page_url));
                return None;
            }
            Click::Cancel => {
                if let Stage::Downloading(install) = &self.stage {
                    install.cancel.store(true, Ordering::Relaxed);
                }
                return None;
            }
            Click::Close => {
                self.stage = Stage::Offer;
                UpdateAnswer::RemindLater
            }
            Click::Answer(answer) => answer,
        };
        self.open = false;
        Some(answer)
    }

    /// Reads the running install's reports, the dialog hidden or not: the
    /// last progress stays for the line, and the outcome moves the dialog on.
    /// A cancelled install brings the offer back; a worker gone without an
    /// outcome is a failure. Returns whether the install succeeded on this
    /// call, once per install, so the shell turns the offer into `Update
    /// ready` then.
    pub(super) fn poll_install(&mut self) -> bool {
        let Stage::Downloading(install) = &mut self.stage else {
            return false;
        };
        let next = loop {
            match install.reports.try_recv() {
                Ok(Report::Progress(progress)) => install.progress = Some(progress),
                Ok(Report::Done(Ok(_))) => break Stage::Ready,
                Ok(Report::Done(Err(InstallError::Cancelled))) => break Stage::Offer,
                Ok(Report::Done(Err(error))) => break Stage::Failed(Failure::of(&error)),
                Err(TryRecvError::Empty) => return false,
                Err(TryRecvError::Disconnected) => {
                    log::warn!("the update's worker ended without an outcome");
                    break Stage::Failed(Failure {
                        reason: WORKER_GONE.to_owned(),
                        nothing_changed: false,
                    });
                }
            }
        };
        self.stage = next;
        matches!(self.stage, Stage::Ready)
    }
}

impl Failure {
    /// What `Update failed` says of `error`: the updater's sentence, and that
    /// nothing was changed unless the old files could not be put back.
    fn of(error: &InstallError) -> Failure {
        Failure {
            reason: error.to_string(),
            nothing_changed: !matches!(error, InstallError::NotRestored { .. }),
        }
    }
}

impl Install {
    /// Starts `updater::install` of `release` into `bundle` on a thread of its
    /// own. Each report wakes the window, so the progress and the outcome do
    /// not wait on input.
    fn start(
        ctx: &egui::Context,
        release: Arc<Release>,
        bundle: PathBuf,
        stamp: Option<ProgramStamp>,
        open_url: Arc<OpenUrl>,
    ) -> Install {
        let (sender, reports) = unbounded();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let repaint = ctx.clone();
        let worker = std::thread::spawn(move || {
            // The dialog gone (the window closed without waiting) leaves the
            // reports nobody to read.
            let send = |report| {
                if sender.send(report).is_ok() {
                    repaint.request_repaint();
                } else {
                    log::debug!("the update's report had no receiver left");
                }
            };
            let outcome = updater::install(
                &release,
                &bundle,
                stamp,
                &*open_url,
                |progress| send(Report::Progress(progress)),
                &worker_cancel,
            );
            send(Report::Done(outcome));
        });
        Install {
            worker,
            reports,
            progress: None,
            cancel,
        }
    }
}

/// The modal every stage draws in: `title` over `body`. `dismiss` is what the
/// backdrop and Escape stand for, `None` where they do nothing. Escape is
/// taken in every stage, so a press the dialog ignores does not reach what
/// lies under it.
fn stage_modal(
    ctx: &egui::Context,
    title: String,
    dismiss: Option<Click>,
    body: impl FnOnce(&mut egui::Ui) -> Option<Click>,
) -> Option<Click> {
    let modal = modal(ctx, egui::Id::unique("update-available"), title, body);
    let dismissed = modal.should_close();
    modal.inner.or(dismiss.filter(|_| dismissed))
}

/// The buttons under a stage's body, given in reading order and drawn
/// right-aligned, the last first. Returns the clicked one's `Click`.
fn stage_buttons(ui: &mut egui::Ui, buttons: &[(&str, Click)]) -> Option<Click> {
    modal_buttons(ui, |ui| {
        let mut clicked = None;
        for &(label, click) in buttons.iter().rev() {
            if ui.button(label).clicked() {
                clicked = Some(click);
            }
        }
        clicked
    })
}

/// `release`'s version as the dialog's sentences name it (`v0.0.2 pre-alpha`).
fn version_label(release: &Release) -> String {
    version_label_of(&release.version.to_string(), false)
}

/// `Update available`: the version sentence, the notes and the four answers.
fn show_offer(
    ctx: &egui::Context,
    release: &Release,
    cache: &mut CommonMarkCache,
) -> Option<Click> {
    let dismiss = Click::Answer(UpdateAnswer::RemindLater);
    stage_modal(ctx, "Update available".to_owned(), Some(dismiss), |ui| {
        ui.label(format!(
            "{} {} is out. You have {}.",
            crate::APP_NAME,
            version_label(release),
            version_label_of(env!("CARGO_PKG_VERSION"), false),
        ));
        modal_sentence_gap(ui);
        // The notes' box, the draft's `.update-notes`: a stroke frame,
        // 8 points of padding either side, at most 240 tall, scrolling.
        egui::Frame::new()
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .corner_radius(2)
            .inner_margin(egui::Margin::symmetric(8, 0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        CommonMarkViewer::new().show(ui, cache, &release.notes);
                    });
            });
        stage_buttons(
            ui,
            &[
                ("Update now", Click::UpdateNow),
                ("Open release page", Click::OpenPage),
                (
                    "Skip this version",
                    Click::Answer(UpdateAnswer::SkipVersion),
                ),
                ("Remind me later", dismiss),
            ],
        )
    })
}

/// How much of the download's bar is filled, from 0 to 1: nothing while the
/// release lists no size to divide by.
fn bar_fraction(progress: Progress) -> f32 {
    if progress.total > 0 {
        (progress.done as f64 / progress.total as f64) as f32
    } else {
        0.0
    }
}

/// `Updating to`: the download's line, its bar and Cancel, disabled once
/// `cancelled` until the worker has answered. The backdrop and Escape do
/// nothing: a stray click must neither cancel a download nor hide one that
/// runs.
fn show_downloading(
    ctx: &egui::Context,
    release: &Release,
    progress: Option<Progress>,
    cancelled: bool,
) -> Option<Click> {
    let title = format!("Updating to v{}", release.version);
    stage_modal(ctx, title, None, |ui| {
        let mut line = format!("Downloading {}…", version_label(release));
        if let Some(Progress { done, total }) = progress {
            let (done, total) = (done as f64, total as f64);
            line += &format!(" {:.1} of {:.1} MB", done / MEGABYTE, total / MEGABYTE);
        }
        let fraction = progress.map_or(0.0, bar_fraction);
        ui.label(line);
        modal_sentence_gap(ui);
        // The draft's `.update-bar`: the stroke colour under a fill in the
        // selection's text colour.
        ui.scope(|ui| {
            let visuals = ui.visuals_mut();
            visuals.extreme_bg_color = visuals.widgets.noninteractive.bg_stroke.color;
            let fill = visuals.selection.stroke.color;
            ui.add(
                egui::ProgressBar::new(fraction)
                    .desired_width(360.0)
                    .desired_height(8.0)
                    .fill(fill),
            );
        });
        modal_buttons(ui, |ui| {
            let cancel = add_enabled(ui, !cancelled, egui::Button::new("Cancel"));
            cancel.clicked().then_some(Click::Cancel)
        })
    })
}

/// `Update failed`: the updater's sentence, the way out through the release
/// page, and Retry, Open release page and Close.
fn show_failed(ctx: &egui::Context, release: &Release, failure: &Failure) -> Option<Click> {
    stage_modal(ctx, "Update failed".to_owned(), Some(Click::Close), |ui| {
        ui.label(&failure.reason);
        modal_sentence_gap(ui);
        let way_out = format!(
            "You can download v{} from the release page instead.",
            release.version
        );
        ui.label(if failure.nothing_changed {
            format!("Nothing was changed. {way_out}")
        } else {
            way_out
        });
        stage_buttons(
            ui,
            &[
                ("Retry", Click::UpdateNow),
                ("Open release page", Click::OpenPage),
                ("Close", Click::Close),
            ],
        )
    })
}

/// `Update ready`: the new version is installed, and Restart now and Later.
fn show_ready(ctx: &egui::Context, release: &Release) -> Option<Click> {
    let later = Click::Answer(UpdateAnswer::Later);
    stage_modal(ctx, "Update ready".to_owned(), Some(later), |ui| {
        ui.label(format!(
            "{} is installed. Restart to start using it, the version you are running now \
             will stay in the old folder.",
            version_label(release)
        ));
        stage_buttons(
            ui,
            &[
                ("Restart now", Click::Answer(UpdateAnswer::RestartNow)),
                ("Later", later),
            ],
        )
    })
}

#[cfg(test)]
mod tests;
