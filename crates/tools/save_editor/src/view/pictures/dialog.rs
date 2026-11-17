//! The Team menu's "Save a picture of the formations…" dialog (the draft's
//! `dlgPictureSave`): the folder and the file name on one grid, the
//! settings, Portraits and grain boxes under it, and Save, which asks before it
//! replaces a picture already in the folder, then writes the PNG and raises
//! a notice with an Open folder action. Its folder row, its boxes and its
//! write are the range dialog's too (`range`).

use std::io;
use std::path::Path;

use pes_savefile::model::TeamEntry;
use studio_core::widgets::{
    CHECKBOX_SPACING, checkbox, dialog_form, modal, modal_buttons, text_box,
};
use studio_core::{ActionEffect, Notice, NoticeAction, NoticeKind};

use super::{PictureOptions, PictureTeam, formations_picture, picture_file_name};
use crate::messages::TOOL_ID;
use crate::session::journal;
use crate::view::SaveView;
use crate::view::generate::PickFolderDialog;
use crate::view::prompt::confirm;

/// The dialog's title, the menu entry's label without its ellipsis.
const TITLE: &str = "Save a picture of the formations";

/// The Portraits box's hover.
const PORTRAITS_TIP: &str =
    "Draw the players' portraits in the picture. Off, the formation's shape is easier to see.";

/// The grain box's hover.
const FLAT_TIP: &str = "Paint the pitch in flat bands, without its grain. The picture looks \
     plainer and its file is smaller, which helps when sharing many.";

/// The folder and file name boxes' width, the draft's.
const FIELD_WIDTH: f32 = 380.0;

/// The dialog's state while it is open (`view.picture`).
pub(crate) struct PictureDialog {
    /// The team the picture is of.
    team_id: u32,
    /// The folder the picture goes to, as typed or picked.
    folder: String,
    /// The picture's file name, as filled in or typed.
    file_name: String,
    /// The Portraits box: this picture's alone, never written back to the
    /// formation bar's setting.
    portraits: bool,
    /// Whether the replace question is up, over the dialog.
    asking_replace: bool,
}

impl PictureDialog {
    /// The dialog as the Team menu opens it on `team`: the folder of the
    /// `formation_pictures_folder` setting (`folder_setting`), a relative
    /// one starting from `exe_dir`; the file name from the team's name;
    /// Portraits as the formation bar's box (`portraits`) is.
    pub(crate) fn open(
        team: &TeamEntry,
        folder_setting: &str,
        exe_dir: &Path,
        portraits: bool,
    ) -> PictureDialog {
        PictureDialog {
            team_id: team.id,
            // An absolute setting replaces `exe_dir` in the join.
            folder: exe_dir.join(folder_setting).display().to_string(),
            file_name: picture_file_name(team.id, &team.name),
            portraits,
            asking_replace: false,
        }
    }
}

/// The dialog while `view.picture` holds one. Save writes the picture
/// when its file is not in the folder yet, else asks first; a written
/// picture closes the dialog, keeps its folder in
/// `view.formation_pictures_folder` and raises the success notice, and a
/// failed one keeps the dialog open behind an error notice.
pub(crate) fn dialog(ui: &mut egui::Ui, view: &mut SaveView) {
    if view.session.is_none() {
        return;
    }
    let SaveView {
        picture,
        pick_folder_dialog,
        formation_pictures_flat,
        formation_pictures_settings,
        ..
    } = view;
    let Some(dialog) = picture.as_mut() else {
        return;
    };
    let mut close = false;
    let mut save = false;
    let response = modal(
        ui.ctx(),
        egui::Id::unique("save_editor_picture"),
        TITLE,
        |ui| {
            dialog_form(ui, "save_editor_picture_grid", |ui| {
                folder_row(ui, &mut dialog.folder, pick_folder_dialog);
                let label = ui.label("File name");
                ui.add(text_box(
                    egui::TextEdit::singleline(&mut dialog.file_name).desired_width(FIELD_WIDTH),
                ))
                .labelled_by(label.id);
                ui.end_row();
            });
            picture_boxes(
                ui,
                formation_pictures_settings,
                &mut dialog.portraits,
                formation_pictures_flat,
            );
            modal_buttons(ui, |ui| {
                if ui.button("Cancel").clicked() {
                    close = true;
                }
                if ui.button("Save").clicked() {
                    save = true;
                }
            });
        },
    );
    let mut write = false;
    if save {
        let exists = Path::new(&dialog.folder).join(&dialog.file_name).is_file();
        dialog.asking_replace = exists;
        write = !exists;
    }
    if dialog.asking_replace {
        let text = format!(
            "{} is already in {}. Replace it?",
            dialog.file_name, dialog.folder
        );
        let answer = confirm(
            ui,
            "save_editor_picture_replace",
            "Replace the picture?",
            &text,
            "Replace",
        );
        if let Some(replace) = answer {
            dialog.asking_replace = false;
            write = replace;
        }
    } else {
        close |= response.should_close();
    }
    if write {
        let options = PictureOptions {
            portraits: dialog.portraits,
            flat: *formation_pictures_flat,
            settings: *formation_pictures_settings,
        };
        let (team_id, folder, file_name) = (
            dialog.team_id,
            dialog.folder.clone(),
            dialog.file_name.clone(),
        );
        let written = save_picture(ui.visuals(), view, team_id, options, &folder, &file_name);
        view.notice = Some(match &written {
            Ok(()) => Notice {
                text: format!("{file_name} written to {folder}"),
                kind: NoticeKind::Success,
                action: Some(NoticeAction {
                    label: "Open folder".to_owned(),
                    effect: ActionEffect::OpenFolder(folder.into()),
                }),
            },
            Err(error) => Notice {
                text: format!("Could not write {file_name}: {error}"),
                kind: NoticeKind::Error,
                action: None,
            },
        });
        close |= written.is_ok();
    }
    if close {
        view.picture = None;
    }
}

/// The grid row `Folder`, which both picture dialogs hold: `folder`'s box
/// and Browse…, whose pick replaces it.
pub(super) fn folder_row(
    ui: &mut egui::Ui,
    folder: &mut String,
    pick_folder_dialog: &mut PickFolderDialog,
) {
    let label = ui.label("Folder");
    ui.horizontal(|ui| {
        ui.add(text_box(
            egui::TextEdit::singleline(folder).desired_width(FIELD_WIDTH),
        ))
        .labelled_by(label.id);
        if ui.button("Browse\u{2026}").clicked()
            && let Some(picked) = pick_folder_dialog()
        {
            *folder = picked.display().to_string();
        }
    });
    ui.end_row();
}

/// The three boxes under both picture dialogs' grids, one under the other
/// and `CHECKBOX_SPACING` apart: the settings box, Portraits and the grain
/// box.
pub(super) fn picture_boxes(
    ui: &mut egui::Ui,
    settings: &mut bool,
    portraits: &mut bool,
    flat: &mut bool,
) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = CHECKBOX_SPACING;
        checkbox(ui, settings, "With each preset's settings beside its row");
        checkbox(ui, portraits, "Portraits").on_hover_text(PORTRAITS_TIP);
        checkbox(ui, flat, "Without the pitch's grain (smaller file)").on_hover_text(FLAT_TIP);
    });
}

/// Writes team `team_id`'s picture as `file_name` in `folder`
/// (`write_picture`), for either picture dialog: on success the folder is
/// kept as the `formation_pictures_folder` setting, the journal gets a
/// `wrote` step and the PNG is kept as an output, as every file the member
/// saves is; a failure is a `could not write` step.
pub(super) fn save_picture(
    visuals: &egui::Visuals,
    view: &mut SaveView,
    team_id: u32,
    options: PictureOptions,
    folder: &str,
    file_name: &str,
) -> io::Result<()> {
    let session = view.session.as_ref().expect("the dialog's save");
    let file = session.file();
    let team = PictureTeam {
        file,
        team: file.team(team_id).expect("the dialog's team"),
        rules: view.rules.as_ref().map(|active| active.rules.as_ref()),
        pes_folder: view.pes_folder.as_deref(),
    };
    let path = Path::new(folder).join(file_name);
    match write_picture(&team, visuals, options, Path::new(folder), file_name) {
        Ok(png) => {
            let recorder = session.recorder();
            recorder.step(TOOL_ID, format!("wrote {}", path.display()));
            recorder.keep_output(TOOL_ID, &path, png.into());
            view.formation_pictures_folder = folder.to_owned();
            Ok(())
        }
        Err(error) => {
            journal::write_failed(&path, session.recorder(), &error);
            Err(error)
        }
    }
}

/// `team`'s picture painted in `visuals` and written as a PNG named
/// `file_name` in `folder`, the folder created when missing; returns the
/// PNG's bytes. The PNG is written to a temporary file beside it, then
/// renamed onto its name, so a failed write leaves the picture that was
/// there before whole.
fn write_picture(
    team: &PictureTeam<'_>,
    visuals: &egui::Visuals,
    options: PictureOptions,
    folder: &Path,
    file_name: &str,
) -> io::Result<Vec<u8>> {
    std::fs::create_dir_all(folder)?;
    let image = formations_picture(team, visuals, options);
    let mut png = io::Cursor::new(Vec::new());
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(io::Error::other)?;
    studio_core::files::replace_file(&folder.join(file_name), |temporary| {
        std::fs::write(temporary, png.get_ref())
    })?;
    Ok(png.into_inner())
}
