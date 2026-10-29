//! The toolbar's Open menu (the draft's `openMenu`): the saves discovery
//! finds under Documents for every version, one click each, then the files
//! opened earlier in this run of the program, then the file picker, which
//! starts in the selected version's save folder.

use std::path::{Component, Path, PathBuf};

use pes_savefile::discovery::SavefileCandidate;
use pes_version::PesVersion;
use studio_core::widgets::menu_button;

use crate::view::SaveView;
use crate::view::prompt::{self, Pending};

/// Discovery for one version: its saves under Documents, newest first
/// (`discover_savefiles` in the window, a folder of the test's own in the
/// tests, which must never list the developer's saves).
pub(crate) type Discover = Box<dyn Fn(PesVersion) -> Vec<SavefileCandidate>>;

/// The menu's file picker: the folder it starts in, if any, to the picked
/// file (`rfd` in the window, a stub in the tests: a native dialog cannot
/// be driven headless).
pub(crate) type OpenDialog = Box<dyn FnMut(Option<&Path>) -> Option<PathBuf>>;

/// `Choose file…`'s tooltip.
const CHOOSE_FILE_TIP: &str = "Opens in the selected version's save folder when it exists";

/// Discovery under the shell's Documents folder.
#[cfg(not(test))]
pub(crate) fn default_discover() -> Discover {
    Box::new(pes_savefile::discovery::discover_savefiles)
}

/// `default_discover` under `cargo test`: a test reaching the real
/// Documents folder panics; tests stub `view.discover`.
#[cfg(test)]
pub(crate) fn default_discover() -> Discover {
    Box::new(|_| panic!("a test reached the real Documents folder: stub discover"))
}

/// The `rfd` Open dialog, in `start` when given.
#[cfg(not(test))]
pub(crate) fn default_open_dialog() -> OpenDialog {
    Box::new(|start| {
        let mut dialog = rfd::FileDialog::new();
        if let Some(folder) = start {
            dialog = dialog.set_directory(folder);
        }
        dialog.pick_file()
    })
}

/// `default_open_dialog` under `cargo test`: a test reaching a real file
/// dialog panics; tests stub `view.open_dialog`.
#[cfg(test)]
pub(crate) fn default_open_dialog() -> OpenDialog {
    Box::new(|_| panic!("a test reached a real file dialog: stub open_dialog"))
}

/// Puts `path` at the top of `recents`, taking it out of where it was.
pub(super) fn push_recent(recents: &mut Vec<PathBuf>, path: &Path) {
    recents.retain(|recent| recent != path);
    recents.insert(0, path.to_path_buf());
}

/// The Open menu button. Discovery lists folders on disk, so it runs once
/// when the menu opens and its list is kept until the menu closes. An
/// entry clicked with unsaved changes asks first (the unsaved-changes
/// prompt), as the picker does.
pub(super) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let menu = menu_button(ui, "Open", |ui| {
        let discovered = view.discovered.get_or_insert_with(|| {
            PesVersion::ALL
                .into_iter()
                .flat_map(|version| {
                    (view.discover)(version)
                        .into_iter()
                        .map(move |candidate| (version, candidate))
                })
                .collect()
        });
        let mut chosen = None;
        for (version, candidate) in discovered.iter() {
            if ui
                .button(discovered_label(*version, candidate))
                .on_hover_text(candidate.path.display().to_string())
                .clicked()
            {
                chosen = Some(Pending::Open(candidate.path.clone()));
            }
        }
        if !discovered.is_empty() && !view.recents.is_empty() {
            ui.separator();
        }
        for path in &view.recents {
            if ui
                .button(recent_label(path))
                .on_hover_text(path.display().to_string())
                .clicked()
            {
                chosen = Some(Pending::Open(path.clone()));
            }
        }
        if !discovered.is_empty() || !view.recents.is_empty() {
            ui.separator();
        }
        if ui
            .button("Choose file\u{2026}")
            .on_hover_text(CHOOSE_FILE_TIP)
            .clicked()
        {
            let start = discovered
                .iter()
                .find(|(version, _)| *version == view.pes_version)
                .and_then(|(_, candidate)| candidate.path.parent())
                .map(Path::to_path_buf);
            chosen = Some(Pending::OpenPicker(start));
        }
        if chosen.is_some() {
            ui.close();
        }
        chosen
    });
    match menu.inner {
        None => view.discovered = None,
        Some(None) => {}
        Some(Some(pending)) if view.is_dirty() => prompt::open(view, pending),
        Some(Some(pending)) => prompt::go_on(view, pending),
    }
}

/// A discovered save's entry: `PES 2019`, then its account folder after
/// two spaces when it has one.
fn discovered_label(version: PesVersion, candidate: &SavefileCandidate) -> String {
    match &candidate.account {
        Some(account) => format!("{version}  ({account})"),
        None => version.to_string(),
    }
}

/// A recent file's entry: the last three parts of its path (two folders
/// and the file name), joined by `\` as Windows writes a path.
fn recent_label(path: &Path) -> String {
    let parts: Vec<String> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    parts[parts.len().saturating_sub(3)..].join("\\")
}
