//! The Savefile menu's "Save formation pictures for a range of teams…"
//! dialog (the draft's `dlgPicturesRange`): one picture per cup team of a
//! range picked as Multi-team AATF picks its range, written into one folder
//! with the one-team dialog's boxes. Save pictures turns the dialog into
//! the run's progress, with Stop, then into its result.
//!
//! The run writes one picture per frame on the UI thread and asks for the
//! next frame. A picture borrows the open save and paints in an egui
//! context of its own, and takes about 0.3 s, so a frame per picture keeps
//! the dialog repainting between pictures and lets Stop land between two of
//! them, with no worker thread, no copy of the save and no channel.
//!
//! egui shows a frame only once its code has run, so a frame stays on
//! screen while the next one writes its picture. Each frame therefore draws
//! the picture the next frame writes: the run's first frame draws `Writing
//! 1 of n` and writes nothing, and the frame that writes the last picture
//! draws `Writing n of n` again. Stop pressed while `Writing k of n` is on
//! screen so keeps pictures 1 to k, the one being written under that line,
//! and no more.
//!
//! The run and its result keep the width the picking state had when Save
//! pictures was pressed, so the dialog does not jump with each team's name.

use std::path::Path;

use pes_savefile::EditFile;
use pes_savefile::model::names::display_name;
use studio_core::widgets::{add_enabled, dialog_form, modal, modal_buttons, modal_sentence_gap};

use super::dialog::{folder_row, picture_boxes, save_picture};
use super::{PictureOptions, picture_file_name};
use crate::session::cup::cup_teams;
use crate::view::SaveView;
use crate::view::generate::PickFolderDialog;
use crate::view::multi_aatf::{counted, range_ids, team_combo};

/// The dialog's title, the menu entry's label without its ellipsis.
const TITLE: &str = "Save formation pictures for a range of teams";

/// The progress bar's height, the draft's `.pbar`.
const BAR_HEIGHT: f32 = 14.0;

/// The dialog's state while it is open (`view.pictures_range`).
pub(crate) struct RangeDialog {
    /// The range's first team id as picked.
    first: u32,
    /// The range's last team id as picked, never before `first`.
    last: u32,
    /// The folder the pictures go to, as typed or picked.
    folder: String,
    /// The Portraits box: this run's alone, never written back to the
    /// formation bar's setting.
    portraits: bool,
    /// The run's read of the install's `player-dummy.png`: `None` until a
    /// picture's starters lack a portrait and it is read, the answer
    /// since (`Some(None)` the install serving none), so a run reads it
    /// once however many of its pictures need it (`read_images`). Taken
    /// out for a picture's write and put back after (`write_next`).
    placeholder: Option<Option<egui::ColorImage>>,
    /// Where the dialog is: picking, writing or showing the result.
    stage: Stage,
}

/// The dialog's three states, one after the other.
enum Stage {
    /// The range, the folder and the boxes being picked.
    Picking,
    /// The run: `ids` the range's teams in id order, `written` how many of
    /// their pictures are written, `options` the boxes as Save pictures
    /// found them, `width` the picking state's content width then.
    /// `on_screen` is whether a frame of the run has been shown, so the
    /// next frame writes under a line the member already sees.
    Writing {
        ids: Vec<u32>,
        written: usize,
        options: PictureOptions,
        on_screen: bool,
        width: f32,
    },
    /// The run over: `written` of `total` pictures written, the write error
    /// that ended it early, if one did, and the run's `width`.
    Done {
        written: usize,
        total: usize,
        error: Option<String>,
        width: f32,
    },
}

/// What a frame of the dialog asks for once it is drawn.
enum Asked {
    /// Nothing pressed; the modal's own close (Escape, the backdrop) may
    /// still close the dialog.
    Nothing,
    /// Cancel or Close.
    Close,
    /// Save pictures: the run starts, at the picking state's content
    /// `width`.
    Save { width: f32 },
    /// The run's first frame drawn: its first picture is the next frame's.
    Shown,
    /// Stop: the run ends before its next picture.
    Stop,
    /// A writing frame without Stop: the run's next picture, team
    /// `team_id`'s.
    Write {
        team_id: u32,
        options: PictureOptions,
    },
    /// Open folder.
    OpenFolder,
}

impl RangeDialog {
    /// The dialog as the Savefile menu opens it on `file`: the range from
    /// the save's first cup team to its last; the folder of the
    /// `formation_pictures_folder` setting (`folder_setting`), a relative
    /// one starting from `exe_dir`; Portraits as the formation bar's box
    /// (`portraits`) is. `None` when the save holds no cup team.
    pub(crate) fn open(
        file: &EditFile,
        folder_setting: &str,
        exe_dir: &Path,
        portraits: bool,
    ) -> Option<RangeDialog> {
        let ids = || cup_teams(file).map(|team| team.id);
        Some(RangeDialog {
            first: ids().min()?,
            last: ids().max()?,
            // An absolute setting replaces `exe_dir` in the join.
            folder: exe_dir.join(folder_setting).display().to_string(),
            portraits,
            placeholder: None,
            stage: Stage::Picking,
        })
    }

    /// Save pictures: the run over the range's teams, with the boxes as
    /// they are now (`flat` the grain box, `settings` the settings box),
    /// at `width`.
    fn start(&mut self, file: &EditFile, flat: bool, settings: bool, width: f32) {
        self.placeholder = None;
        self.stage = Stage::Writing {
            ids: range_ids(file, self.first, self.last),
            written: 0,
            options: PictureOptions {
                portraits: self.portraits,
                flat,
                settings,
            },
            on_screen: false,
            width,
        };
    }

    /// The run's first frame drawn.
    fn shown(&mut self) {
        if let Stage::Writing { on_screen, .. } = &mut self.stage {
            *on_screen = true;
        }
    }

    /// Stop: the run ends with the pictures written so far.
    fn stop(&mut self) {
        if let Stage::Writing {
            ids,
            written,
            width,
            ..
        } = &self.stage
        {
            self.stage = Stage::Done {
                written: *written,
                total: ids.len(),
                error: None,
                width: *width,
            };
        }
    }

    /// The run after its next picture's write ended in `result`: on to the
    /// next team, done after the last, or ended where it stands by an
    /// error.
    fn wrote(&mut self, result: Result<(), String>) {
        let Stage::Writing {
            ids,
            written,
            width,
            ..
        } = &mut self.stage
        else {
            return;
        };
        let (total, width) = (ids.len(), *width);
        match result {
            Ok(()) => {
                *written += 1;
                if *written == total {
                    self.stage = Stage::Done {
                        written: total,
                        total,
                        error: None,
                        width,
                    };
                }
            }
            Err(error) => {
                self.stage = Stage::Done {
                    written: *written,
                    total,
                    error: Some(error),
                    width,
                };
            }
        }
    }
}

/// The dialog while `view.pictures_range` holds one: the picking state, a
/// frame of the run (its line, its bar and Stop, then the next picture
/// written unless Stop was pressed), or the result. The modal's own close
/// is refused while the run writes: Stop is the way out.
pub(crate) fn range_dialog(ui: &mut egui::Ui, view: &mut SaveView) {
    let SaveView {
        pictures_range,
        session,
        pick_folder_dialog,
        formation_pictures_flat,
        formation_pictures_settings,
        ..
    } = view;
    let (Some(dialog), Some(session)) = (pictures_range.as_mut(), session.as_ref()) else {
        return;
    };
    let file = session.file();
    let response = modal(
        ui.ctx(),
        egui::Id::unique("save_editor_pictures_range"),
        TITLE,
        |ui| match dialog.stage {
            Stage::Picking => picking(
                ui,
                dialog,
                file,
                pick_folder_dialog,
                formation_pictures_settings,
                formation_pictures_flat,
            ),
            Stage::Writing {
                ref ids,
                written,
                options,
                on_screen,
                width,
            } => {
                ui.set_width(width);
                writing(ui, file, ids, written, options, on_screen, width)
            }
            Stage::Done {
                written,
                total,
                ref error,
                width,
            } => {
                ui.set_width(width);
                result(ui, &dialog.folder, written, total, error.as_deref(), width)
            }
        },
    );
    let close = match response.inner {
        Asked::Nothing => response.should_close(),
        Asked::Close => true,
        Asked::Save { width } => {
            dialog.start(
                file,
                *formation_pictures_flat,
                *formation_pictures_settings,
                width,
            );
            ui.ctx().request_repaint();
            false
        }
        Asked::Shown => {
            dialog.shown();
            ui.ctx().request_repaint();
            false
        }
        Asked::Stop => {
            dialog.stop();
            false
        }
        Asked::Write { team_id, options } => {
            write_next(ui, view, team_id, options);
            false
        }
        Asked::OpenFolder => {
            studio_core::files::open_folder(Path::new(&dialog.folder));
            false
        }
    };
    if close {
        view.pictures_range = None;
    }
}

/// The picking state: one grid holding the `Teams` row (the range's two
/// combos and its count) and the folder row, then the three boxes, the
/// file-name line, Save pictures and Cancel.
fn picking(
    ui: &mut egui::Ui,
    dialog: &mut RangeDialog,
    file: &EditFile,
    pick_folder_dialog: &mut PickFolderDialog,
    settings: &mut bool,
    flat: &mut bool,
) -> Asked {
    let grid = dialog_form(ui, "save_editor_pictures_range_grid", |ui| {
        ui.label("Teams");
        let (count, room) = ui
            .horizontal(|ui| {
                team_combo(
                    ui,
                    "save_editor_pictures_first",
                    file,
                    u32::MIN,
                    &mut dialog.first,
                );
                // The last end follows a first end picked past it, before
                // it is drawn, so the range is never reversed.
                dialog.last = dialog.last.max(dialog.first);
                ui.weak("to");
                team_combo(
                    ui,
                    "save_editor_pictures_last",
                    file,
                    dialog.first,
                    &mut dialog.last,
                );
                let count = range_ids(file, dialog.first, dialog.last).len();
                // Room for the count, which is drawn after the grid, so
                // a row of long team names still widens the column.
                let font = egui::TextStyle::Body.resolve(ui.style());
                let size = ui
                    .painter()
                    .layout_no_wrap(counted(count, "team"), font, egui::Color32::PLACEHOLDER)
                    .size();
                (count, ui.allocate_exact_size(size, egui::Sense::hover()).0)
            })
            .inner;
        ui.end_row();
        folder_row(ui, &mut dialog.folder, pick_folder_dialog);
        (count, room)
    });
    let (count, room) = grid.inner;
    // The count ends where the grid's field column ends, which is where
    // the folder row's Browse ends. Only the drawn grid knows that end, so
    // the count is drawn after the grid, in a child that takes no room of
    // its own: the Teams row already kept room for it.
    let at = room.translate(egui::vec2(grid.response.rect.right() - room.right(), 0.0));
    ui.new_child(egui::UiBuilder::new().max_rect(at))
        .add(egui::Label::new(egui::RichText::new(counted(count, "team")).weak()).extend());
    picture_boxes(ui, settings, &mut dialog.portraits, flat);
    modal_sentence_gap(ui);
    file_name_line(ui, file, dialog.first);
    // The button row takes the content's width and never widens it.
    let width = ui.min_rect().width();
    modal_buttons(ui, |ui| {
        let cancel = ui.button("Cancel").clicked();
        let save = add_enabled(ui, count > 0, egui::Button::new("Save pictures")).clicked();
        if cancel {
            Asked::Close
        } else if save {
            Asked::Save { width }
        } else {
            Asked::Nothing
        }
    })
}

/// The line naming the file of the range's first team, `first`.
fn file_name_line(ui: &mut egui::Ui, file: &EditFile, first: u32) {
    let name = &file.team(first).expect("the range's first team").name;
    let line = format!(
        "One picture per team, named after it ({}), overwriting files with the same names.",
        picture_file_name(first, name)
    );
    ui.add(egui::Label::new(egui::RichText::new(line).weak()).extend());
}

/// `text` as a `Label` wrapping under `width` (`truncate` instead cuts
/// the line to one row), for the run's states, which pin the dialog to
/// it. A `Label`'s own wrap asks egui for the ui's available width, and
/// egui rounds a wrap's width to a whole point, so a `width` of 496.7
/// would wrap at 497: a row landing in the fraction between reports wider
/// than the `set_width` and pushes the dialog wider. `width`'s floor is a
/// whole point already: rows wrap at and under it, and the label reports
/// no wider.
fn pinned_label(ui: &mut egui::Ui, text: &str, width: f32, truncate: bool) -> egui::Response {
    let mut job = std::sync::Arc::unwrap_or_clone(egui::WidgetText::from(text).into_layout_job(
        ui.style(),
        egui::FontSelection::Default,
        ui.text_valign(),
    ));
    job.wrap = egui::text::TextWrapping::from_wrap_mode_and_width(
        if truncate {
            egui::TextWrapMode::Truncate
        } else {
            egui::TextWrapMode::Wrap
        },
        width.floor(),
    );
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    ui.add(egui::Label::new(galley))
}

/// A frame of the run, `written` pictures written before it: `Writing k of
/// n: <name> (<id>)` for the team the next frame writes, the bar at the
/// share written before that one, and Stop. Unless Stop is clicked, the
/// first frame (`on_screen` false) asks for nothing more and every later
/// one for team `ids[written]`'s picture.
fn writing(
    ui: &mut egui::Ui,
    file: &EditFile,
    ids: &[u32],
    written: usize,
    options: PictureOptions,
    on_screen: bool,
    width: f32,
) -> Asked {
    let shown = if on_screen {
        (written + 1).min(ids.len() - 1)
    } else {
        written
    };
    let team_id = ids[shown];
    let name = &file.team(team_id).expect("a team of the range").name;
    let line = format!(
        "Writing {} of {}: {} ({team_id})",
        shown + 1,
        ids.len(),
        display_name(name)
    );
    pinned_label(ui, &line, width, true);
    progress_bar(ui, shown as f32 / ids.len() as f32);
    let stop = modal_buttons(ui, |ui| ui.button("Stop").clicked());
    if stop {
        Asked::Stop
    } else if on_screen {
        Asked::Write {
            team_id: ids[written],
            options,
        }
    } else {
        Asked::Shown
    }
}

/// The draft's `.pbar`: the dialog's content width, `BAR_HEIGHT` high, the
/// theme's darkest fill under the selection colour up to `fraction`, in
/// the panel's 1-point stroke at the theme's radius.
fn progress_bar(ui: &mut egui::Ui, fraction: f32) {
    let width = ui.min_rect().width();
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, BAR_HEIGHT), egui::Sense::hover());
    response.widget_info(|| {
        let mut info = egui::WidgetInfo::new(egui::accesskit::Role::ProgressIndicator);
        info.value = Some((f64::from(fraction) * 100.0).floor());
        info
    });
    let visuals = ui.visuals();
    let radius = visuals.widgets.noninteractive.corner_radius;
    let stroke = egui::Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, visuals.extreme_bg_color);
    let filled =
        egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * fraction, rect.height()));
    // Square on the right, as the draft's bar clips its fill.
    let fill_radius = egui::CornerRadius {
        ne: 0,
        se: 0,
        ..radius
    };
    painter.rect_filled(filled, fill_radius, visuals.selection.bg_fill);
    painter.rect_stroke(rect, radius, stroke, egui::StrokeKind::Inside);
}

/// The result: the sentence saying what was written, with the folder on
/// the line under it when a picture was; the error that ended the run, if
/// one did, on its own line; then Open folder (when a picture was written)
/// and Close. The folder has a line of its own because the dialog keeps
/// its width, so a long folder would otherwise break the sentence wherever
/// it wrapped.
fn result(
    ui: &mut egui::Ui,
    folder: &str,
    written: usize,
    total: usize,
    error: Option<&str>,
    width: f32,
) -> Asked {
    let line = if written == total {
        format!("{} written, they are here:", counted(written, "picture"))
    } else if written == 0 {
        "Stopped before any picture was written".to_owned()
    } else {
        format!("Stopped after {written} of {total} pictures, they are here:")
    };
    pinned_label(ui, &line, width, false);
    if written > 0 {
        pinned_label(ui, folder, width, false);
    }
    if let Some(error) = error {
        pinned_label(ui, error, width, false);
    }
    modal_buttons(ui, |ui| {
        let close = ui.button("Close").clicked();
        let open = written > 0 && ui.button("Open folder").clicked();
        if close {
            Asked::Close
        } else if open {
            Asked::OpenFolder
        } else {
            Asked::Nothing
        }
    })
}

/// Writes the run's next picture, team `team_id`'s, through the one-team
/// dialog's path (`save_picture`), under the name `picture_file_name`
/// gives it, then moves the run on and asks for the next frame.
fn write_next(ui: &egui::Ui, view: &mut SaveView, team_id: u32, options: PictureOptions) {
    // The run's shared dummy read is taken out for the write: it cannot be
    // lent to `save_picture` through the view it is a field of.
    let (folder, mut placeholder) = {
        let dialog = view.pictures_range.as_mut().expect("the open dialog");
        (
            dialog.folder.clone(),
            std::mem::take(&mut dialog.placeholder),
        )
    };
    let session = view.session.as_ref().expect("the dialog's save");
    let name = &session
        .file()
        .team(team_id)
        .expect("a team of the range")
        .name;
    let file_name = picture_file_name(team_id, name);
    let written = save_picture(
        ui.visuals(),
        view,
        team_id,
        options,
        &folder,
        &file_name,
        &mut placeholder,
    )
    .map_err(|error| format!("Could not write {file_name}: {error}"));
    let dialog = view.pictures_range.as_mut().expect("the open dialog");
    dialog.placeholder = placeholder;
    dialog.wrote(written);
    ui.ctx().request_repaint();
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;

    use super::*;

    /// `pinned_label`'s labels never report wider than `width`: the sweep
    /// pads `x`s into a folder path's line to land rows in the sub-pixel
    /// slot between `width` and the point egui rounds it up to, at the
    /// run's measured 496.72 and at widths that round the other way.
    /// Reverting the call to a plain wrapped `Label` fails this test.
    #[test]
    fn pinned_label_reports_no_wider_than_its_width() {
        let ctx = egui::Context::default();
        studio_core::theme::install(&ctx);
        for width in [496.71875f32, 496.3, 496.7] {
            let mut rects = Vec::new();
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1024.0, 768.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    ui.set_width(width);
                    for length in 0..72usize {
                        let text = format!(
                            "formation {} pictures of every team of the cup, from the first to \
                             the last",
                            "x".repeat(length)
                        );
                        rects.push((length, pinned_label(ui, &text, width, false).rect));
                    }
                },
            )
            .drop_without_applying_deltas();
            for (length, rect) in &rects {
                assert!(
                    rect.width() <= width,
                    "width {width}, length {length}: the label reports {} wide",
                    rect.width()
                );
            }
        }
    }

    // A run over two teams whose starters no listed CPK holds a portrait
    // of reads the game's `player-dummy.png` once: the first picture's
    // read is kept in the run's `placeholder` for the second, here the
    // dummy a listed `download` CPK holds. The run is driven frame by
    // frame through `range_dialog`, as the window does.
    #[test]
    fn a_range_run_reads_the_games_dummy_once() {
        let ctx = egui::Context::default();
        studio_core::theme::install(&ctx);
        let dir = crate::test_support::scratch("range_dummy_once");
        let save = dir.join("EDIT00000000");
        std::fs::write(&save, pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        let dummy = crate::test_support::dummy_png([255, 0, 255, 255]);
        let entries: &[(&str, &[u8])] = &[(
            "common/render/symbol/blank/player-dummy.png",
            dummy.as_slice(),
        )];
        let pes = crate::test_support::pes_folder(
            "range_dummy_once_pes",
            &[("4cc_40_faces.cpk", entries)],
        );

        // The run over cup teams 716 and 717 of the PES 2021 fixture.
        let mut view = SaveView::new();
        view.open(&save);
        view.pes_folder = Some(pes);
        let file = view.session.as_ref().expect("the open save").file();
        let mut dialog = RangeDialog::open(file, "", &dir, true).expect("cup teams");
        dialog.first = 716;
        dialog.last = 717;
        dialog.start(file, false, false, 400.0);
        view.pictures_range = Some(dialog);
        crate::view::portraits::reset_placeholder_reads();
        for _ in 0..10 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1024.0, 768.0),
                )),
                ..Default::default()
            };
            ctx.run_ui(input, |ui| range_dialog(ui, &mut view))
                .drop_without_applying_deltas();
            if matches!(
                view.pictures_range.as_ref().expect("the dialog").stage,
                Stage::Done { .. }
            ) {
                break;
            }
        }
        assert!(
            matches!(
                view.pictures_range.as_ref().expect("the dialog").stage,
                Stage::Done {
                    written: 2,
                    error: None,
                    ..
                }
            ),
            "both pictures written"
        );
        assert_eq!(
            crate::view::portraits::placeholder_reads(),
            1,
            "the run's two pictures share one read"
        );
    }
}
