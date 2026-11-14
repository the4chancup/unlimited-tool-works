//! The toolbar's File menu (the draft's `fileMenu`): first the opens, the
//! saves discovery finds under Documents for every version, one click
//! each, then the files opened earlier in this run of the program, then
//! the file picker, which starts in the folder the last save was opened
//! from, or the newest discovered save's; then Save and Save as. The opens'
//! shortcuts (the draft's `OPEN_KEYS`)
//! open a version's newest save, or the picker, with the menu closed.
//! The welcome panel (the draft's `welcomePanel`), the view with no save
//! open, draws the same opens in the middle of the view.

use std::path::{Component, Path, PathBuf};

use pes_savefile::discovery::SavefileCandidate;
use pes_version::PesVersion;
use studio_core::widgets::{add_enabled, menu_button, menu_separator};

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
const CHOOSE_FILE_TIP: &str = "Pick a save file, starting in the folder of the save opened last (or else of the \
     newest save found)";

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

/// The File menu's two writes, which `show` hands back to the toolbar: a
/// write needs the running-game probe, which the menu does not hold.
pub(super) enum Write {
    /// Save: the open file.
    Save,
    /// Save as: the picker, then the picked path.
    SaveAs,
}

/// One of the File menu's opens: a discovered save, a recent file, or the
/// picker. The menu and the welcome panel draw the one list `opens`
/// builds, so the two can never disagree.
struct Open {
    /// The entry's label (the version's name, the recent's path tail,
    /// "Choose file…").
    label: String,
    /// The shortcut the entry carries at its right, empty with none (the
    /// first of each version's saves, and the picker's, have one).
    shortcut: String,
    /// The entry's tooltip (the save's or the file's path, the picker's
    /// start rule).
    tip: String,
    /// What the click asks for.
    pending: Pending,
}

impl Open {
    /// The entry's button: its label, with its shortcut at the right in
    /// weak when it has one.
    fn button(&self) -> egui::Button<'_> {
        let button = egui::Button::new(&self.label);
        if self.shortcut.is_empty() {
            button
        } else {
            button.shortcut_text(self.shortcut.as_str())
        }
    }
}

/// The opens' entries, in the order the File menu draws them.
struct Opens {
    /// The discovered saves, each labelled by its version, the first of
    /// each version's with its shortcut.
    saves: Vec<Open>,
    /// The files opened earlier in this run.
    recents: Vec<Open>,
    /// The file picker, `Choose file…` with Ctrl+O.
    picker: Open,
}

/// The opens `discovered` and `view`'s recents make. Discovery lists a
/// version's saves newest first, and the newest is the one its shortcut
/// opens.
fn opens(view: &SaveView, discovered: &[(PesVersion, SavefileCandidate)]) -> Opens {
    let mut saves = Vec::new();
    let mut previous = None;
    for (version, candidate) in discovered {
        let shortcut = if previous != Some(*version) {
            shortcut_text(*version)
        } else {
            String::new()
        };
        previous = Some(*version);
        saves.push(Open {
            label: version.to_string(),
            shortcut,
            tip: candidate.path.display().to_string(),
            pending: Pending::Open(candidate.path.clone()),
        });
    }
    let recents = view
        .recents
        .iter()
        .map(|path| Open {
            label: recent_label(path),
            shortcut: String::new(),
            tip: path.display().to_string(),
            pending: Pending::Open(path.clone()),
        })
        .collect();
    let picker = Open {
        label: "Choose file\u{2026}".to_owned(),
        shortcut: "Ctrl+O".to_owned(),
        tip: CHOOSE_FILE_TIP.to_owned(),
        pending: choose_file(view.last_folder.as_deref(), discovered),
    };
    Opens {
        saves,
        recents,
        picker,
    }
}

/// The File menu button. Discovery lists folders on disk, so it runs once
/// when the menu opens and its list is kept until the menu closes. An
/// open clicked with unsaved changes asks first (the unsaved-changes
/// prompt), as the picker does. Returns the write chosen this frame, if
/// any; Save and Save as are disabled while no save is open.
pub(super) fn show(ui: &mut egui::Ui, view: &mut SaveView) -> Option<Write> {
    let save_open = view.session.is_some();
    let menu = menu_button(ui, "File", |ui| {
        if view.discovered.is_none() {
            view.discovered = Some(discover_all(view));
        }
        let opens = opens(view, view.discovered.as_ref().expect("filled above"));
        let mut chosen = None;
        let mut entry = |ui: &mut egui::Ui, open: &Open| {
            if ui.add(open.button()).on_hover_text(&open.tip).clicked() {
                chosen = Some(open.pending.clone());
            }
        };
        for open in &opens.saves {
            entry(ui, open);
        }
        if !opens.saves.is_empty() && !opens.recents.is_empty() {
            menu_separator(ui);
        }
        for open in &opens.recents {
            entry(ui, open);
        }
        if !opens.saves.is_empty() || !opens.recents.is_empty() {
            menu_separator(ui);
        }
        entry(ui, &opens.picker);
        menu_separator(ui);
        let mut write = None;
        if add_enabled(
            ui,
            save_open,
            egui::Button::new("Save").shortcut_text("Ctrl+S"),
        )
        .clicked()
        {
            write = Some(Write::Save);
        }
        if add_enabled(ui, save_open, egui::Button::new("Save as\u{2026}")).clicked() {
            write = Some(Write::SaveAs);
        }
        if chosen.is_some() || write.is_some() {
            ui.close();
        }
        (chosen, write)
    });
    let Some((chosen, write)) = menu.inner else {
        view.discovered = None;
        return None;
    };
    if let Some(pending) = chosen {
        request(view, pending);
    }
    write
}

/// The welcome panel (the draft's `welcomePanel`), drawn in place of the
/// panes with no save open: the File menu's opens in the dialog's frame
/// and title in the middle of the view, but a page, not a modal — nothing
/// under it is blocked, the sidebar's tool switch included. Discovery
/// runs once when the panel appears and the list is kept while it stays
/// up, never a scan a frame.
pub(super) fn welcome_panel(ui: &mut egui::Ui, view: &mut SaveView) {
    if view.welcome_discovered.is_none() {
        view.welcome_discovered = Some(discover_all(view));
    }
    let opens = opens(
        view,
        view.welcome_discovered.as_ref().expect("filled above"),
    );
    let chosen = studio_core::widgets::dialog_page(
        ui,
        egui::Id::unique("save_editor_welcome"),
        "Save editor",
        |ui| welcome_body(ui, &opens),
    )
    .inner;
    if let Some(pending) = chosen {
        request(view, pending);
    }
}

/// The welcome panel's body: the heading "Saves in Documents", each save
/// as a button filling the frame's width with its shortcut at the right
/// and its path as the tooltip (the weak line with none), then a
/// separator and "Recent" when any opened earlier, a separator and
/// `Choose file…` with Ctrl+O. Returns the clicked entry's `Pending`.
fn welcome_body(ui: &mut egui::Ui, opens: &Opens) -> Option<Pending> {
    // The widest entry's row — its label, the draft's 24-point gap and
    // its shortcut, in the button's padding — is the width every entry
    // fills, the shortcut ending at the frame's edge.
    let font = egui::TextStyle::Button.resolve(ui.style());
    let text_width = |text: &str| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
            .size()
            .x
    };
    let row = |open: &Open| {
        text_width(&open.label)
            + if open.shortcut.is_empty() {
                0.0
            } else {
                24.0 + text_width(&open.shortcut)
            }
    };
    let content = opens
        .saves
        .iter()
        .chain(&opens.recents)
        .map(row)
        .fold(0.0, f32::max)
        + 2.0 * ui.spacing().button_padding.x;
    let entry = |ui: &mut egui::Ui, open: &Open| {
        ui.add(open.button().min_size(egui::vec2(content, 0.0)))
            .on_hover_text(&open.tip)
            .clicked()
            .then(|| open.pending.clone())
    };
    ui.strong("Saves in Documents");
    let mut chosen = None;
    for open in &opens.saves {
        chosen = entry(ui, open).or(chosen);
    }
    if opens.saves.is_empty() {
        ui.weak("No PES saves were found in Documents.");
    }
    if !opens.recents.is_empty() {
        ui.separator();
        ui.strong("Recent");
        for open in &opens.recents {
            chosen = entry(ui, open).or(chosen);
        }
    }
    ui.separator();
    // The picker's entry is the draft's compact button, not a full row.
    if ui
        .add(opens.picker.button())
        .on_hover_text(&opens.picker.tip)
        .clicked()
    {
        chosen = Some(opens.picker.pending.clone());
    }
    chosen
}

/// What an Open shortcut asks for.
pub(super) enum Shortcut {
    /// Ctrl+O: the file picker, as `Choose file…`.
    Picker,
    /// Ctrl and the version's digit: its newest discovered save.
    Save(PesVersion),
}

/// Takes the Open shortcut pressed this frame out of `input`, if any.
/// Ctrl alone: egui's `consume_key(Ctrl, …)` would also take the key with
/// Shift or Alt held as well.
pub(super) fn consume_shortcut(input: &mut egui::InputState) -> Option<Shortcut> {
    if consume_ctrl(input, egui::Key::O) {
        return Some(Shortcut::Picker);
    }
    PesVersion::ALL
        .into_iter()
        .find(|version| consume_ctrl(input, shortcut_key(*version)))
        .map(Shortcut::Save)
}

/// Opens what `shortcut` asks for, as the menu would with its entry
/// clicked: the version's newest discovered save, or the picker when
/// there is none. Discovery runs now, since the menu's list exists only
/// while the menu is open.
pub(super) fn open_shortcut(view: &mut SaveView, shortcut: Shortcut) {
    let newest = match shortcut {
        Shortcut::Save(version) => (view.discover)(version).into_iter().next(),
        Shortcut::Picker => None,
    };
    let pending = match newest {
        Some(candidate) => Pending::Open(candidate.path),
        None => {
            // The picker's start folder asks the newest discovered save of
            // whatever version.
            choose_file(view.last_folder.as_deref(), &discover_all(view))
        }
    };
    request(view, pending);
}

/// Whether `key` was pressed with Ctrl and no other modifier, taking it
/// out of `input` if so.
fn consume_ctrl(input: &mut egui::InputState, key: egui::Key) -> bool {
    let before = input.events.len();
    input.events.retain(|event| {
        !matches!(
            event,
            egui::Event::Key { key: pressed, pressed: true, modifiers, .. }
                if *pressed == key && modifiers.matches_exact(egui::Modifiers::CTRL)
        )
    });
    input.events.len() < before
}

/// The digit that, with Ctrl, opens `version`'s newest discovered save:
/// 5 to 9 for PES 2015 to PES 2019, then 0 for PES 2020 and 1 for PES 2021.
fn shortcut_key(version: PesVersion) -> egui::Key {
    match version {
        PesVersion::Pes15 => egui::Key::Num5,
        PesVersion::Pes16 => egui::Key::Num6,
        PesVersion::Pes17 => egui::Key::Num7,
        PesVersion::Pes18 => egui::Key::Num8,
        PesVersion::Pes19 => egui::Key::Num9,
        PesVersion::Pes20 => egui::Key::Num0,
        PesVersion::Pes21 => egui::Key::Num1,
    }
}

/// `version`'s shortcut as its menu entry shows it (`Ctrl+9`).
fn shortcut_text(version: PesVersion) -> String {
    format!("Ctrl+{}", shortcut_key(version).name())
}

/// Every version's saves under Documents, each with its version, in
/// `PesVersion::ALL`'s order (`view.discover`, stubbed in the tests).
fn discover_all(view: &SaveView) -> Vec<(PesVersion, SavefileCandidate)> {
    PesVersion::ALL
        .into_iter()
        .flat_map(|version| {
            (view.discover)(version)
                .into_iter()
                .map(move |candidate| (version, candidate))
        })
        .collect()
}

/// `Choose file…`'s request, the picker the Open shortcuts fall back to too:
/// starting in `last_folder`, the folder the last save was opened from, while
/// it exists, else in `discovered`'s newest save's folder, else in no folder.
fn choose_file(
    last_folder: Option<&Path>,
    discovered: &[(PesVersion, SavefileCandidate)],
) -> Pending {
    Pending::OpenPicker(
        last_folder
            .filter(|folder| folder.is_dir())
            .or_else(|| {
                discovered
                    .iter()
                    .max_by_key(|(_, candidate)| candidate.modified)
                    .and_then(|(_, candidate)| candidate.path.parent())
            })
            .map(Path::to_path_buf),
    )
}

/// Goes on with `pending`, the unsaved-changes prompt first when there
/// are any.
fn request(view: &mut SaveView, pending: Pending) {
    if view.is_dirty() {
        prompt::open(view, pending);
    } else {
        prompt::go_on(view, pending);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    // The last opened file stands first: a path already in the list moves
    // there once, a new one prepends. The list is uncapped — it holds
    // this run of the program's opens.
    #[test]
    fn a_pushed_path_stands_first_once() {
        let mut recents = paths(&["a", "b", "c"]);
        push_recent(&mut recents, Path::new("b"));
        assert_eq!(recents, paths(&["b", "a", "c"]));
        push_recent(&mut recents, Path::new("d"));
        assert_eq!(recents, paths(&["d", "b", "a", "c"]));
    }
}
