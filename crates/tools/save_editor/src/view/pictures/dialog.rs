//! The Team menu's "Save a picture of the formations…" dialog (the draft's
//! `dlgPictureSave`): the folder and the file name on one grid, the
//! Portraits and grain boxes under it, and Save, which asks before it
//! replaces a picture already in the folder, then writes the PNG and raises
//! a notice with an Open folder action.

use std::io;
use std::path::Path;

use pes_savefile::model::TeamEntry;
use studio_core::widgets::{FORM_ROW_SPACING, checkbox, modal, modal_buttons, text_box};
use studio_core::{ActionEffect, Notice, NoticeAction, NoticeKind};

use super::{PictureOptions, PictureTeam, formations_picture, picture_file_name};
use crate::messages::TOOL_ID;
use crate::session::journal;
use crate::view::SaveView;
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
            file_name: picture_file_name(team.id, &team.name, false),
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
            egui::Grid::new("save_editor_picture_grid")
                .num_columns(2)
                .spacing(egui::vec2(ui.spacing().item_spacing.x, FORM_ROW_SPACING))
                .show(ui, |ui| {
                    let label = ui.label("Folder");
                    ui.horizontal(|ui| {
                        ui.add(text_box(
                            egui::TextEdit::singleline(&mut dialog.folder)
                                .desired_width(FIELD_WIDTH),
                        ))
                        .labelled_by(label.id);
                        if ui.button("Browse\u{2026}").clicked()
                            && let Some(folder) = pick_folder_dialog()
                        {
                            dialog.folder = folder.display().to_string();
                        }
                    });
                    ui.end_row();
                    let label = ui.label("File name");
                    ui.add(text_box(
                        egui::TextEdit::singleline(&mut dialog.file_name)
                            .desired_width(FIELD_WIDTH),
                    ))
                    .labelled_by(label.id);
                    ui.end_row();
                });
            checkbox(ui, &mut dialog.portraits, "Portraits").on_hover_text(PORTRAITS_TIP);
            checkbox(
                ui,
                formation_pictures_flat,
                "Without the pitch's grain (smaller file)",
            )
            .on_hover_text(FLAT_TIP);
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
        let written = save_picture(ui.visuals(), view);
        let dialog = view.picture.as_ref().expect("the open dialog");
        view.notice = Some(match &written {
            Ok(()) => Notice {
                text: format!("{} written to {}", dialog.file_name, dialog.folder),
                kind: NoticeKind::Success,
                action: Some(NoticeAction {
                    label: "Open folder".to_owned(),
                    effect: ActionEffect::OpenFolder(dialog.folder.clone().into()),
                }),
            },
            Err(error) => Notice {
                text: format!("Could not write {}: {error}", dialog.file_name),
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

/// Writes the open dialog's picture (`write_picture`): on success its
/// folder is kept as the `formation_pictures_folder` setting and the
/// journal gets a `wrote` step; a failure is a `could not write` step.
fn save_picture(visuals: &egui::Visuals, view: &mut SaveView) -> io::Result<()> {
    let dialog = view.picture.as_ref().expect("the open dialog");
    let session = view.session.as_ref().expect("the dialog's save");
    let file = session.file();
    let team = PictureTeam {
        file,
        team: file.team(dialog.team_id).expect("the dialog's team"),
        rules: view.rules.as_ref().map(|active| active.rules.as_ref()),
        pes_folder: view.pes_folder.as_deref(),
    };
    let options = PictureOptions {
        portraits: dialog.portraits,
        flat: view.formation_pictures_flat,
    };
    let folder = Path::new(&dialog.folder);
    let path = folder.join(&dialog.file_name);
    let written = write_picture(&team, visuals, options, folder, &dialog.file_name);
    match &written {
        Ok(()) => {
            session
                .recorder()
                .step(TOOL_ID, format!("wrote {}", path.display()));
            view.formation_pictures_folder = dialog.folder.clone();
        }
        Err(error) => journal::write_failed(&path, session.recorder(), error),
    }
    written
}

/// `team`'s picture painted in `visuals` and written as a PNG named
/// `file_name` in `folder`, the folder created when missing. The PNG is
/// written to a temporary file beside it, then renamed onto its name, so a
/// failed write leaves the picture that was there before whole.
fn write_picture(
    team: &PictureTeam<'_>,
    visuals: &egui::Visuals,
    options: PictureOptions,
    folder: &Path,
    file_name: &str,
) -> io::Result<()> {
    std::fs::create_dir_all(folder)?;
    let image = formations_picture(team, visuals, options);
    let mut png = io::Cursor::new(Vec::new());
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(io::Error::other)?;
    let temporary = folder.join(format!("{file_name}.tmp"));
    let written = std::fs::write(&temporary, png.get_ref())
        .and_then(|()| std::fs::rename(&temporary, folder.join(file_name)));
    if let Err(error) = written {
        // The write's error is the one to report; a temporary file left
        // behind is a diagnostic.
        if let Err(removal) = std::fs::remove_file(&temporary) {
            log::debug!("{}: kept: {removal}", temporary.display());
        }
        return Err(error);
    }
    Ok(())
}
