//! A settings row, the draft `docs/prototypes/index.html`'s `sRow`: the label cell, its
//! tooltip leading with the settings key, and the widget cell. `path_row` is the one row
//! every section's path setting draws, the common folders' and a tool's alike, so
//! no tool's path commits differently from the common ones. The text edit commits on
//! Enter or focus loss, never per keystroke, and the Browse button opens `rfd`'s
//! native folder picker, a pick committing at once.

use std::path::PathBuf;

use super::text_box::text_box;

/// A row's left cell: the plain label, its tooltip the settings key then `tip`, the row's
/// own sentence for a member (the draft's `sRow`), so a member who edits `settings.toml`
/// by hand still finds it. The tip is written for the member, not copied from the field's
/// doc comment, which is written for the developer.
pub(crate) fn row_label(ui: &mut egui::Ui, label: &str, key: &str, tip: &str) -> egui::Response {
    ui.label(label).on_hover_text(format!("{key}: {tip}"))
}

/// A path row: the edit binds `buffer` and commits it to `current` when it loses focus
/// or Enter is pressed, returning whether the commit changed the setting; the Browse
/// button after the edit opens the native folder picker and a pick sets the buffer and
/// the setting at once (a commit). While the row has no focus the buffer follows
/// `current`, so a change made outside the menu (the notice's "Fix PES path") still
/// shows.
pub(crate) fn path_row(
    ui: &mut egui::Ui,
    label: &str,
    key: &str,
    tip: &str,
    buffer: &mut String,
    current: &mut String,
) -> bool {
    path_row_commit(ui, label, key, tip, buffer, current).is_some()
}

/// How a path row's setting changed this frame: typed into the edit, or picked with
/// Browse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathCommit {
    /// The edit committed on Enter or focus loss, its text stored as typed.
    Typed,
    /// Browse's picker returned a folder, stored as `Path::display` writes it.
    Picked,
}

/// The path edit's width: the one that shows Steam's PES 2019 install path
/// whole. Its galley is 434 points wide at the 12.5-point body font, inside
/// the 456 points the edit's 8-point margin on each side leaves; the menu's
/// width test measures it.
const PATH_EDIT_WIDTH: f32 = 472.0;

/// `path_row`, saying which way the setting changed: the PES folder row stores a
/// picked folder through its own rule (the selected version's year written `20**`),
/// a typed one as typed.
pub(crate) fn path_row_commit(
    ui: &mut egui::Ui,
    label: &str,
    key: &str,
    tip: &str,
    buffer: &mut String,
    current: &mut String,
) -> Option<PathCommit> {
    let label = row_label(ui, label, key, tip);
    let (response, picked) = ui
        .horizontal(|ui| {
            let response = ui
                .add(text_box(
                    egui::TextEdit::singleline(buffer).desired_width(PATH_EDIT_WIDTH),
                ))
                .labelled_by(label.id);
            // `rfd`'s pickers are native-only, so the wasm row is the edit
            // alone.
            #[cfg(not(target_arch = "wasm32"))]
            let picked = browse(ui);
            #[cfg(target_arch = "wasm32")]
            let picked: Option<PathBuf> = None;
            (response, picked)
        })
        .inner;
    ui.end_row();
    if let Some(path) = picked {
        *buffer = path.display().to_string();
        *current = buffer.clone();
        return Some(PathCommit::Picked);
    }
    if response.lost_focus() {
        if buffer == current {
            return None;
        }
        *current = buffer.clone();
        return Some(PathCommit::Typed);
    }
    // The buffer follows `current` only while the edit lacks focus in
    // egui's memory: `Response::has_focus` is also false while the
    // window is unfocused, so resetting on it would drop a half-typed
    // path on an Alt+Tab.
    if !ui.ctx().memory(|memory| memory.has_focus(response.id)) {
        *buffer = current.clone();
    }
    None
}

/// The Browse button and the native picker it opens, `rfd`'s blocking folder
/// dialog; `Some` only on a pick.
#[cfg(not(target_arch = "wasm32"))]
fn browse(ui: &mut egui::Ui) -> Option<PathBuf> {
    if !ui.button("Browse").clicked() {
        return None;
    }
    rfd::FileDialog::new().pick_folder()
}
