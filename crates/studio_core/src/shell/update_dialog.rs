//! The update offer's Details dialog, as the draft's `openUpdate` draws it: the
//! release's version line and its notes rendered as the help renders Markdown
//! in a framed box that scrolls, then Open release page, Skip this version and
//! Remind me later. Update now and its download come with 0.0.2; until then the
//! release page is the way to the new version. No dialog opens by itself: the
//! status bar's offer text or its Details button opens it. Native only: the
//! browser build has no update check to offer.

use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::updater;
use crate::widgets::{modal, modal_buttons, modal_sentence_gap};

/// The dialog's state: whether it is up, and the cache its notes' Markdown
/// renders through.
#[derive(Default)]
pub(super) struct UpdateDialog {
    /// The modal is drawn while set.
    open: bool,
    /// The notes' `CommonMarkCache`.
    cache: CommonMarkCache,
}

/// What the member answered; the shell applies it. `RemindLater` is also the
/// backdrop's and Escape's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UpdateAnswer {
    /// Skip this version: store it as `skipped_version` and drop the offer.
    SkipVersion,
    /// Remind me later: only close; the offer stays.
    RemindLater,
}

impl UpdateDialog {
    /// Opens the dialog: the status bar's offer text or its Details button.
    pub(super) fn open(&mut self) {
        self.open = true;
    }

    /// Closes the dialog without an answer, as the shell closes its other
    /// modals (a held close, the help window opening).
    pub(super) fn close(&mut self) {
        self.open = false;
    }

    /// Draws the dialog while it is up and clears `open` when it closes (an
    /// answer, the backdrop or Escape), returning the member's answer when one
    /// lands.
    pub(super) fn show(
        &mut self,
        ctx: &egui::Context,
        release: &updater::Release,
    ) -> Option<UpdateAnswer> {
        if !self.open {
            return None;
        }
        let cache = &mut self.cache;
        let modal = modal(
            ctx,
            egui::Id::unique("update-available"),
            "Update available",
            |ui| {
                ui.label(format!(
                    "{} {} is out. You have {}.",
                    crate::APP_NAME,
                    crate::shell::version_label_of(&release.version.to_string(), false),
                    crate::shell::version_label_of(env!("CARGO_PKG_VERSION"), false),
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
                // The buttons right-aligned, the last in reading order first.
                modal_buttons(ui, |ui| {
                    let later = ui.button("Remind me later").clicked();
                    let skip = ui.button("Skip this version").clicked();
                    let page = ui.button("Open release page").clicked();
                    (page, skip, later)
                })
            },
        );
        let (page, skip, later) = modal.inner;
        if page {
            ctx.open_url(egui::OpenUrl::same_tab(&release.page_url));
        }
        let answer = if skip {
            Some(UpdateAnswer::SkipVersion)
        } else if later || modal.should_close() {
            Some(UpdateAnswer::RemindLater)
        } else {
            None
        };
        if answer.is_some() {
            self.open = false;
        }
        answer
    }
}

#[cfg(test)]
mod tests;
