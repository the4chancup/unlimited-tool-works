//! The Generate settings.toml / in-game face dialogs (the draft's
//! `dlgSettings`/`dlgIngameFace`): two Player-menu actions, one
//! per file a player folder holds, writing the player's look as
//! `PlayerSettings::from_player` emits it. A save cannot say which players
//! wear the in-game face (the cup keeps every "modified face" flag off), so
//! the member picks the action per player: `settings.toml` carries no face,
//! hair or iris key; `ingame_face.toml` is the whole in-game face.
//! The dialogs write no save, so the aesthetics lock does not gate them.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use pes_savefile::EditFile;
use pes_savefile::model::names::display_name;
use pes_savefile::settings_toml::PlayerSettings;
use studio_core::widgets::{add_enabled, modal, modal_buttons, text_box};
use studio_core::{Notice, NoticeKind};

use crate::messages::TOOL_ID;
use crate::session::journal;
use crate::view::SaveView;

/// The folder field's Pick dialog (`rfd` in the window, a stub in the
/// tests — the comparator's `OpenSaveDialog` pattern).
pub(crate) type PickFolderDialog = Box<dyn FnMut() -> Option<PathBuf>>;

/// The `rfd` Pick-a-folder dialog.
#[cfg(not(test))]
pub(crate) fn default_pick_folder() -> PickFolderDialog {
    Box::new(|| rfd::FileDialog::new().pick_folder())
}

/// `default_pick_folder` under `cargo test`: a test reaching a real file
/// dialog panics; tests stub `view.pick_folder_dialog`.
#[cfg(test)]
pub(crate) fn default_pick_folder() -> PickFolderDialog {
    Box::new(|| panic!("a test reached a real file dialog: stub pick_folder_dialog"))
}

/// Which of the two player-folder files the dialog writes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenFile {
    /// `settings.toml`: every setting but the in-game face's.
    Settings,
    /// `ingame_face.toml`: iris, face and hair.
    IngameFace,
}

impl GenFile {
    /// The file's name inside the player folder.
    fn file_name(self) -> &'static str {
        match self {
            GenFile::Settings => "settings.toml",
            GenFile::IngameFace => "ingame_face.toml",
        }
    }

    /// The dialog's heading (the menu item's label).
    fn title(self) -> &'static str {
        match self {
            GenFile::Settings => "Generate settings.toml",
            GenFile::IngameFace => "Generate in-game face",
        }
    }
}

/// The Generate dialog's state while it is open (`view.generate`): the
/// player the file is for, which of the two files, the folder field's text,
/// the previewed text and the last Write's failure, shown under the buttons
/// while the dialog stays open.
pub(crate) struct Generate {
    /// The player the file is generated from.
    player_id: u32,
    /// Which file Write puts down.
    file: GenFile,
    /// The player folder the file lands in, as typed or picked.
    folder: String,
    /// The text shown and written, `None` until the first frame builds it.
    preview: Option<Preview>,
    /// The failure of the last Write (or `None`).
    error: Option<String>,
    /// Whether the folder's own file's read or parse failure is already
    /// in the journal: one `could not read` step per folder picked or
    /// typed (a re-pick of the same folder reads and records again).
    failure_recorded: bool,
}

/// The file's text as the dialog last built it, with what it was built
/// from. Building it reads and parses the folder's own file, so it is
/// built again only when the folder text or the save changes, not on
/// every frame the dialog is up (egui draws one per pointer move).
struct Preview {
    /// The folder text it was built for.
    folder: String,
    /// The session's revision it was built at.
    revision: u64,
    /// Whether that folder is a directory, Write's gate.
    is_dir: bool,
    /// `text`'s answer for that folder (the shown line on `Err`: the
    /// file failure it carries is taken out before the preview is
    /// stored).
    text: Result<String, String>,
}

/// `text`'s `Err`: the line shown in the preview's place, and — for the
/// folder's own file's read or parse error — the failure the journal
/// records as `could not read <path>` once per folder.
struct PreviewError {
    /// The line the dialog draws where the preview stands.
    shown: String,
    /// The folder's own file's read or parse failure, when `shown` is
    /// that file's.
    failed_read: Option<FailedRead>,
}

/// The folder's own file's read or parse failure, for
/// `journal::read_failed`'s `could not read <path>: <error>` step.
struct FailedRead {
    /// The file that failed.
    path: PathBuf,
    /// Its read or parse error, the step's reason.
    error: String,
    /// The file's bytes when the read reached them (a parse failure's):
    /// kept as an input, since the file that failed is the one a member
    /// reports.
    bytes: Option<Arc<[u8]>>,
}

impl PreviewError {
    /// An error of the generation, not of the folder's own file.
    fn other(error: impl fmt::Display) -> PreviewError {
        PreviewError {
            shown: error.to_string(),
            failed_read: None,
        }
    }

    /// `path`'s read or parse `error`: shown naming the file, and kept
    /// for the journal's `could not read <path>`.
    fn file(path: PathBuf, error: impl fmt::Display, bytes: Option<Arc<[u8]>>) -> PreviewError {
        PreviewError {
            shown: format!("{}: {error}", path.display()),
            failed_read: Some(FailedRead {
                path,
                error: error.to_string(),
                bytes,
            }),
        }
    }
}

impl Generate {
    /// The dialog as the menu item opens it: the folder left for the member
    /// to pick.
    pub(crate) fn new(player_id: u32, file: GenFile) -> Generate {
        Generate {
            player_id,
            file,
            folder: String::new(),
            preview: None,
            error: None,
            failure_recorded: false,
        }
    }
}

/// The file's text as Write would put it down: `PlayerSettings::from_player`
/// emitted into the file's own template — for `settings.toml` into the
/// folder's own file when one is there and parses (`update_toml`: its
/// comments and every other key kept), else the template; for
/// `ingame_face.toml` the record's face replacing whatever file is there.
/// `Err`'s `shown` is the text the dialog draws where the preview stands:
/// the first value `from_player` refuses, the existing file's read or
/// parse error.
fn text(
    file: &EditFile,
    player_id: u32,
    which: GenFile,
    folder: &Path,
) -> Result<String, PreviewError> {
    let player = file.player(player_id).expect("the dialog's player");
    let settings = PlayerSettings::from_player(player).map_err(PreviewError::other)?;
    match which {
        GenFile::IngameFace => settings
            .ingame_face()
            .to_toml()
            .map_err(PreviewError::other),
        GenFile::Settings => {
            let path = folder.join("settings.toml");
            let existing = if folder.is_dir() {
                match std::fs::read_to_string(&path) {
                    Ok(text) => Some(text),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(error) => return Err(PreviewError::file(path, error, None)),
                }
            } else {
                None
            };
            match existing {
                Some(existing) => {
                    let mut document =
                        existing
                            .parse::<toml_edit::DocumentMut>()
                            .map_err(|error| {
                                PreviewError::file(path, error, Some(existing.into_bytes().into()))
                            })?;
                    settings
                        .update_toml(&mut document)
                        .map_err(PreviewError::other)?;
                    Ok(document.to_string())
                }
                None => settings.to_toml().map_err(PreviewError::other),
            }
        }
    }
}

/// The Generate dialog while `view.generate` holds one (the draft's
/// `dlgSettings`/`dlgIngameFace` over one body): the folder field with its
/// Pick button, the file's text in a monospace scroll area (the read, parse
/// or `from_player` error in its place), Write — enabled only for an existing
/// directory and a text that built — and Cancel. Write puts down exactly
/// the previewed text and raises the shell notice; a failed write keeps
/// the dialog open with the error under the buttons.
pub(crate) fn dialog(ui: &mut egui::Ui, view: &mut SaveView) {
    let SaveView {
        generate,
        session,
        pick_folder_dialog,
        notice,
        ..
    } = view;
    let Some(generate) = generate.as_mut() else {
        return;
    };
    let Some(session) = session.as_ref() else {
        return;
    };
    let file = session.file();
    let player = file
        .player(generate.player_id)
        .expect("the dialog's player");
    let name = display_name(&player.name);
    let revision = session.revision();
    if generate
        .preview
        .as_ref()
        .is_none_or(|preview| preview.folder != generate.folder)
    {
        // A new folder's file is a new possible failure.
        generate.failure_recorded = false;
    }
    if generate
        .preview
        .as_ref()
        .is_none_or(|preview| preview.folder != generate.folder || preview.revision != revision)
    {
        let folder = Path::new(&generate.folder);
        let built = text(file, generate.player_id, generate.file, folder);
        // The preview stores the line shown; the file failure the built
        // `Err` carries is recorded once per folder picked or typed.
        let (previewed, failed) = match built {
            Ok(text) => (Ok(text), None),
            Err(error) => (Err(error.shown), error.failed_read),
        };
        if !generate.failure_recorded
            && let Some(failed) = failed
        {
            generate.failure_recorded = true;
            journal::read_failed(
                &failed.path,
                session.recorder(),
                &failed.error,
                failed.bytes,
            );
        }
        generate.preview = Some(Preview {
            folder: generate.folder.clone(),
            revision,
            is_dir: folder.is_dir(),
            text: previewed,
        });
    }
    let mut close = false;
    let mut picked = false;
    let id = egui::Id::unique("save_editor_generate");
    let response = modal(ui.ctx(), id, generate.file.title(), |ui| {
        let preview = generate.preview.as_ref().expect("built above");
        ui.horizontal(|ui| {
            ui.label("Folder");
            ui.add(text_box(
                egui::TextEdit::singleline(&mut generate.folder).desired_width(320.0),
            ));
            if ui.button("Pick\u{2026}").clicked()
                && let Some(path) = pick_folder_dialog()
            {
                generate.folder = path.display().to_string();
                picked = true;
            }
        });
        ui.weak(generate.file.file_name());
        egui::ScrollArea::both()
            .id_salt("save_editor_generate_text")
            .max_width(600.0)
            .max_height(300.0)
            .show(ui, |ui| match &preview.text {
                Ok(text) => {
                    ui.add(
                        egui::Label::new(egui::RichText::new(text.as_str()).monospace())
                            .selectable(false),
                    );
                }
                Err(error) => {
                    ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                }
            });
        modal_buttons(ui, |ui| {
            if ui.button("Cancel").clicked() {
                close = true;
            }
            let writeable = preview.is_dir && preview.text.is_ok();
            if add_enabled(ui, writeable, egui::Button::new("Write")).clicked()
                && let Ok(text) = &preview.text
            {
                let path = Path::new(&preview.folder).join(generate.file.file_name());
                // Write destroys the file it replaces: its bytes are kept
                // as an input first, read now — the member may have
                // touched it since the preview was built. A failed read is
                // a `could not read` step; the write proceeds either way.
                match std::fs::read(&path) {
                    Ok(bytes) => {
                        session.recorder().keep_input(TOOL_ID, &path, bytes.into());
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => journal::read_failed(&path, session.recorder(), &error, None),
                }
                match journal::write_and_record(session.recorder(), &path, text.as_bytes()) {
                    Ok(()) => {
                        *notice = Some(Notice {
                            text: format!(
                                "{} for {name} written into {}",
                                generate.file.file_name(),
                                preview.folder
                            ),
                            kind: NoticeKind::Success,
                            action: None,
                        });
                        close = true;
                    }
                    Err(error) => {
                        generate.error = Some(format!("{error:#}"));
                    }
                }
            }
        });
        if let Some(error) = &generate.error {
            ui.colored_label(ui.visuals().error_fg_color, error.as_str());
        }
    });
    // A pick reads the folder again, even the one already shown: its file
    // may have been fixed outside since the preview was built.
    if picked {
        generate.preview = None;
    }
    if close || response.should_close() {
        view.generate = None;
    }
}
