//! A settings row, the draft `docs/prototypes/index.html`'s `sRow`: the label cell, its
//! tooltip leading with the settings key, and the widget cell. `path_row` is the one row
//! every section's path setting draws, the common folders' and a tool's file alike, so
//! no tool's path commits differently from the common ones. The text edit commits on
//! Enter or focus loss, never per keystroke, and
//! the Browse button opens `rfd`'s native picker of the row's kind, a pick committing
//! at once.

use std::path::PathBuf;

use super::text_box::text_box;

/// What `path_row`'s Browse button opens: the native picker
/// of the setting's kind, a folder or a file filtered to the type the tool reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    /// A folder (`rfd`'s `pick_folder`).
    Folder,
    /// A file of `filter`'s `extensions` (`rfd`'s `pick_file` behind an `add_filter`).
    File {
        /// The filter's name as the dialog shows it.
        filter: &'static str,
        /// The file endings the picker accepts, without the dot (`4ccrules`).
        extensions: &'static [&'static str],
    },
}

/// A row's left cell: the plain label, its tooltip the settings key then `tip`, the row's
/// own sentence for a member (the draft's `sRow`), so a member who edits `settings.toml`
/// by hand still finds it. The tip is written for the member, not copied from the field's
/// doc comment, which is written for the developer.
pub(crate) fn row_label(ui: &mut egui::Ui, label: &str, key: &str, tip: &str) -> egui::Response {
    ui.label(label).on_hover_text(format!("{key}: {tip}"))
}

/// A path row: the edit binds `buffer` and commits it to `current` when it loses focus
/// or Enter is pressed, returning whether the commit changed the setting; the Browse
/// button after the edit opens `kind`'s native picker and a pick sets the buffer and
/// the setting at once (a commit). While the row has no focus the buffer follows
/// `current`, so a change made outside the menu (the notice's "Fix PES path") still
/// shows.
pub fn path_row(
    ui: &mut egui::Ui,
    label: &str,
    key: &str,
    tip: &str,
    buffer: &mut String,
    current: &mut String,
    kind: PathKind,
) -> bool {
    path_row_commit(ui, label, key, tip, buffer, current, kind).is_some()
}

/// How a path row's setting changed this frame: typed into the edit, or picked with
/// Browse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathCommit {
    /// The edit committed on Enter or focus loss, its text stored as typed.
    Typed,
    /// Browse's picker returned a folder or file, stored as `Path::display` writes it.
    Picked,
}

/// The path edit's width: the one that shows Steam's PES 2019 install path
/// whole (456 points at the 13-point body font, plus the edit's 8-point
/// margin on each side); the menu's width test measures it.
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
    _kind: PathKind,
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
            // alone (`_kind`: `browse` exists only off wasm).
            #[cfg(not(target_arch = "wasm32"))]
            let picked = browse(ui, _kind);
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

/// The Browse button and the native picker it opens, `kind`'s: `rfd`'s blocking
/// folder dialog or file dialog filtered to `extensions`; `Some` only on a pick.
#[cfg(not(target_arch = "wasm32"))]
fn browse(ui: &mut egui::Ui, kind: PathKind) -> Option<PathBuf> {
    if !ui.button("Browse").clicked() {
        return None;
    }
    match kind {
        PathKind::Folder => rfd::FileDialog::new().pick_folder(),
        PathKind::File { filter, extensions } => rfd::FileDialog::new()
            .add_filter(filter, extensions)
            .pick_file(),
    }
}
