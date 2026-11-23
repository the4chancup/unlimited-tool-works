//! The Upgrade DpFileList dialog (the draft's `upgradeDpfl`): what the upgrade will do in
//! `download/` (the renames, the placeholders, the CPKs the game stops loading, each with a
//! file of its own given an unticked delete box) before anything is written.

use std::collections::BTreeSet;
use std::path::PathBuf;

use studio_core::widgets::{modal_buttons, modal_sentence_gap};

use crate::messages::size_text;
use crate::upgrade::Preview;

/// The dialog's content width (the draft's `width:460px`).
const WIDTH: f32 = 460.0;
/// A list's most height before it scrolls (the draft's `.list` `max-height:120px`).
const LIST_HEIGHT: f32 = 120.0;
/// The room between the renames table's columns (the draft's `td` padding).
const COLUMN_GAP: f32 = 8.0;

/// The dialog's state: the upgrade's `preview` shown, and the dropped CPKs the member
/// ticked for deletion.
pub(crate) struct DpflDialog {
    preview: Preview,
    /// The `download/` the preview was made for, which the upgrade acts in: kept, since
    /// the settings it was resolved from may have changed while the dialog is open.
    download: PathBuf,
    delete: BTreeSet<String>,
}

/// What a frame of the dialog decided.
pub(crate) enum Outcome {
    /// Still open.
    Open,
    /// Closed, nothing written (Cancel or a dismissal).
    Closed,
    /// Upgrade: apply the upgrade in the folder the preview was made for, deleting the
    /// ticked CPKs.
    Upgrade(PathBuf, BTreeSet<String>),
}

impl DpflDialog {
    /// The dialog over `preview`, made for `download`, every delete box unticked.
    pub(crate) fn new(preview: Preview, download: PathBuf) -> DpflDialog {
        DpflDialog {
            preview,
            download,
            delete: BTreeSet::new(),
        }
    }

    /// Draws the dialog for this frame: the member's decision when one was made. A part
    /// of the preview whose count is zero is left out with its sentence, and the
    /// first paragraph shows only when the preview replaces the list.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> Outcome {
        let mut outcome = Outcome::Open;
        let response = studio_core::widgets::modal(
            ui.ctx(),
            egui::Id::unique("upgrade_dpfl"),
            "Upgrade DpFileList",
            |ui| {
                ui.set_width(WIDTH);
                let preview = &self.preview;
                // The gap goes between two parts shown, none above the first: the first
                // paragraph is left out when the list already is the official one.
                let mut shown = 0;
                let mut gap = |ui: &mut egui::Ui| {
                    if shown > 0 {
                        modal_sentence_gap(ui);
                    }
                    shown += 1;
                };
                if preview.replaces_list {
                    gap(ui);
                    ui.label(
                        "Upgrade replaces the game's DpFileList.bin with the cup's official \
                         list, keeping the old one beside it as DpFileList.bin.bak.",
                    );
                }
                if !preview.renames.is_empty() {
                    gap(ui);
                    if preview.renames.len() == 1 {
                        ui.label(
                            "Renamed to its official name, so the game keeps loading \
                             it (1):",
                        );
                    } else {
                        ui.label(format!(
                            "Renamed to their official names, so the game keeps loading \
                             them ({}):",
                            preview.renames.len()
                        ));
                    }
                    renames_list(ui, &preview.renames);
                }
                if preview.placeholders > 0 {
                    gap(ui);
                    if preview.placeholders == 1 {
                        ui.label(
                            "An empty placeholder is written for the official CPK with \
                             no file, because the game loads nothing from download/ \
                             while a CPK its list names is missing.",
                        );
                    } else {
                        ui.label(format!(
                            "An empty placeholder is written for each of the {} official \
                             CPKs with no file, because the game loads nothing from \
                             download/ while a CPK its list names is missing.",
                            preview.placeholders
                        ));
                    }
                }
                if !preview.dropped.is_empty() {
                    gap(ui);
                    // The tick is asked for only over boxes: a dropped entry without a
                    // file in download/ gets none.
                    if preview.dropped.iter().any(|(_, size)| size.is_some()) {
                        ui.label(format!(
                            "No longer loaded by the game ({}). Tick any you want deleted:",
                            preview.dropped.len()
                        ));
                    } else {
                        ui.label(format!(
                            "No longer loaded by the game ({}):",
                            preview.dropped.len()
                        ));
                    }
                    dropped_list(ui, &preview.dropped, &mut self.delete);
                }
                let buttons = modal_buttons(ui, |ui| {
                    (
                        ui.button("Upgrade").clicked(),
                        ui.button("Cancel").clicked(),
                    )
                });
                match buttons {
                    (true, _) => {
                        outcome = Outcome::Upgrade(
                            self.download.clone(),
                            std::mem::take(&mut self.delete),
                        );
                    }
                    (_, true) => outcome = Outcome::Closed,
                    (false, false) => {}
                }
            },
        );
        if response.should_close() {
            outcome = Outcome::Closed;
        }
        outcome
    }
}

/// The draft's `.list` box: a frame on the extreme background holding `contents`.
fn list_box(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(ui.visuals().widgets.noninteractive.corner_radius)
        .inner_margin(egui::Margin {
            left: 6,
            right: 6,
            top: 0,
            bottom: 4,
        })
        .show(ui, contents);
}

/// `text`'s drawn width in `ui`, to give the renames table's first column its widest
/// cell's.
fn text_width(ui: &egui::Ui, text: egui::RichText) -> f32 {
    egui::WidgetText::from(text)
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::FontSelection::Default,
        )
        .size()
        .x
}

/// The renames table, the draft's `.list` box holding a `table.grid`: the header row
/// kept at the top while the names scroll under it, at most `LIST_HEIGHT` tall in all.
fn renames_list(ui: &mut egui::Ui, renames: &[(String, String)]) {
    // The first column is as wide as its widest cell, the header's own included, so the
    // header's columns line up with the rows' that scroll under them.
    let mut column = text_width(ui, egui::RichText::new("Current name").strong());
    for (from, _) in renames {
        column = column.max(text_width(ui, egui::RichText::new(from).monospace()));
    }
    let column = column + COLUMN_GAP;
    let header = ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().item_spacing.y;
    list_box(ui, |ui| {
        renames_row(ui, column, "Current name", "Official name", false);
        egui::ScrollArea::vertical()
            .id_salt("renames")
            .max_height(LIST_HEIGHT - header)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (from, to) in renames {
                    renames_row(ui, column, from, to, true);
                }
            });
    });
}

/// A row of the renames table: `left` in a column `column` wide, `right` after it; the
/// header row strong (the draft's `th`), the name rows monospace (its `td.mono`).
fn renames_row(ui: &mut egui::Ui, column: f32, left: &str, right: &str, names: bool) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let (left, right) = if names {
            (
                egui::RichText::new(left).monospace(),
                egui::RichText::new(right).monospace(),
            )
        } else {
            (
                egui::RichText::new(left).strong(),
                egui::RichText::new(right).strong(),
            )
        };
        let taken = ui.label(left).rect.width();
        ui.add_space((column - taken).max(0.0));
        ui.label(right);
    });
}

/// The dropped CPKs' list, the same `.list` box with the draft's `mono`
/// class: `name (size)` with an unticked delete box for each whose file is
/// there, the name alone for one without, all in the monospace font.
fn dropped_list(
    ui: &mut egui::Ui,
    dropped: &[(String, Option<u64>)],
    delete: &mut BTreeSet<String>,
) {
    list_box(ui, |ui| {
        egui::ScrollArea::vertical()
            .id_salt("dropped")
            .max_height(LIST_HEIGHT)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (name, size) in dropped {
                    match size {
                        Some(size) => {
                            let mut ticked = delete.contains(name);
                            if studio_core::widgets::checkbox(
                                ui,
                                &mut ticked,
                                egui::RichText::new(format!("{name} ({})", size_text(*size)))
                                    .monospace(),
                            )
                            .changed()
                            {
                                if ticked {
                                    delete.insert(name.clone());
                                } else {
                                    delete.remove(name);
                                }
                            }
                        }
                        None => {
                            ui.label(egui::RichText::new(name).monospace());
                        }
                    }
                }
            });
    });
}
