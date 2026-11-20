//! The toolbar's File menu (the draft's `fileMenu`): `Open…` (the file
//! picker, which starts in the folder the last save was opened from, or
//! the newest discovered save's), Save and Save as, then the weak heading
//! `Open from Documents` over the saves discovery finds under Documents
//! for every version (one click each), the `Recent files` heading over the
//! last saves opened or saved, and Close save and Exit. The opens'
//! shortcuts (the draft's `OPEN_KEYS`)
//! open a version's newest save, or the picker, with the menu closed.
//! The welcome panel (the draft's `welcomePanel`), the view with no save
//! open, draws the same opens in the menu's order in the middle of the view.

use std::path::{Path, PathBuf};

use pes_savefile::discovery::SavefileCandidate;
use pes_version::PesVersion;
use studio_core::widgets::{add_enabled, consume_ctrl, menu_button, menu_separator};

use crate::settings::RecentSave;
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

/// `Open…`'s tooltip.
const OPEN_TIP: &str = "Pick a save file, starting in the folder of the last opened save (or else of the \
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

/// The recent saves' cap, the `recent_files` setting's.
const RECENT_MAX: usize = 10;

/// Puts `path` at the top of `recents` under `version`, taking it out of
/// where it was, the list then cut to `RECENT_MAX`.
pub(super) fn push_recent(recents: &mut Vec<RecentSave>, path: &Path, version: PesVersion) {
    recents.retain(|recent| recent.path != path);
    recents.insert(
        0,
        RecentSave {
            path: path.to_path_buf(),
            version,
        },
    );
    recents.truncate(RECENT_MAX);
}

/// The File menu's two writes, which `show` hands back to the toolbar: a
/// write needs the running-game probe, which the menu does not hold.
pub(super) enum Write {
    /// Save: the open file.
    Save,
    /// Save as: the picker, then the picked path.
    SaveAs,
}

/// One of the File menu's opens: the picker, a discovered save or a recent
/// file. The menu draws the whole list `opens` builds, and the welcome
/// panel the same opens in the same order, so the two can never disagree.
struct Open {
    /// The entry's label ("Open…", the version's name, the recent's
    /// `[version] folder\file`).
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
    /// weak when it has one. The label keeps one line (`Extend`): the
    /// menu and the welcome panel widen by the scroll bar's strip rather
    /// than let a wide label wrap under it.
    fn button(&self) -> egui::Button<'_> {
        let button = egui::Button::new(&self.label).wrap_mode(egui::TextWrapMode::Extend);
        if self.shortcut.is_empty() {
            button
        } else {
            button.shortcut_text(self.shortcut.as_str())
        }
    }
}

/// The opens' entries, in the order the File menu draws them.
struct Opens {
    /// The file picker, `Open…` with Ctrl+O.
    picker: Open,
    /// The discovered saves, each labelled by its version, the first of
    /// each version's with its shortcut.
    saves: Vec<Open>,
    /// The recent saves still on disk, `[<version>] <folder>\<file>` each;
    /// a path that is gone is skipped but stays in the setting.
    recents: Vec<Open>,
}

/// What one look at the disk found for the File menu or the welcome
/// panel, taken when the menu opens or the panel appears and kept while
/// it stays up, never a frame: a recent save on a network share that has
/// gone can take seconds to answer whether it exists.
#[derive(Clone)]
pub(super) struct Found {
    /// The discovered saves, each with its version, oldest first
    /// (PES 2014, then `PesVersion::ALL`'s order).
    saves: Vec<(PesVersion, SavefileCandidate)>,
    /// `view`'s recent saves still on disk; one that is gone is skipped but
    /// stays in the setting.
    recents: Vec<RecentSave>,
    /// `Open…`'s start folder, `picker_folder`'s rule resolved once with
    /// the look.
    picker_start: Option<PathBuf>,
}

/// The look `Found` keeps: discovery under Documents, which recent
/// saves are still on disk, and `Open…`'s start folder.
fn look(view: &SaveView) -> Found {
    let saves = discover_all(view);
    Found {
        picker_start: picker_folder(view.last_folder.as_deref(), &saves),
        saves,
        recents: view
            .recents
            .iter()
            .filter(|recent| recent.path.exists())
            .cloned()
            .collect(),
    }
}

/// The opens `found` makes. Discovery lists a version's saves newest
/// first, and the newest is the one its shortcut opens.
fn opens(found: &Found) -> Opens {
    let mut saves = Vec::new();
    let mut previous = None;
    for (version, candidate) in &found.saves {
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
    let recents = found
        .recents
        .iter()
        .map(|recent| Open {
            label: recent_label(recent),
            shortcut: String::new(),
            tip: recent.path.display().to_string(),
            pending: Pending::Open(recent.path.clone()),
        })
        .collect();
    let picker = Open {
        label: "Open\u{2026}".to_owned(),
        shortcut: "Ctrl+O".to_owned(),
        tip: OPEN_TIP.to_owned(),
        pending: Pending::OpenPicker(found.picker_start.clone()),
    };
    Opens {
        picker,
        saves,
        recents,
    }
}

/// A group's heading, the draft's `.menu-heading`: a label, not a button —
/// `text` carries its colour (weak in the File menu, strong on the welcome
/// panel) — its text starting where the entries' text does, the button
/// padding in from the row's left edge.
fn group_heading(ui: &mut egui::Ui, text: egui::RichText) {
    ui.horizontal(|ui| {
        ui.add_space(ui.spacing().button_padding.x);
        ui.label(text);
    });
}

/// The File menu button. Discovery lists folders on disk, so it runs once
/// when the menu opens and its list is kept until the menu closes; while
/// no save is open the welcome panel's list takes the same scan, so the
/// two never disagree. An open clicked with unsaved changes asks first
/// (the unsaved-changes prompt), as the picker does. Returns the write
/// chosen this frame, if any; Save, Save as and Close save are disabled
/// while no save is open.
pub(super) fn show(ui: &mut egui::Ui, view: &mut SaveView) -> Option<Write> {
    let save_open = view.session.is_some();
    let menu = menu_button(ui, "File", |ui| {
        if view.discovered.is_none() {
            view.discovered = Some(look(view));
            if !save_open {
                view.welcome_discovered = view.discovered.clone();
            }
        }
        let opens = opens(view.discovered.as_ref().expect("filled above"));
        let mut chosen = None;
        let mut write = None;
        if ui
            .add(opens.picker.button())
            .on_hover_text(&opens.picker.tip)
            .clicked()
        {
            chosen = Some(opens.picker.pending.clone());
        }
        if add_enabled(
            ui,
            save_open,
            egui::Button::new("Save").shortcut_text("Ctrl+S"),
        )
        .clicked()
        {
            write = Some(Write::Save);
        }
        if add_enabled(
            ui,
            save_open,
            egui::Button::new("Save as\u{2026}").shortcut_text("Ctrl+Shift+S"),
        )
        .clicked()
        {
            write = Some(Write::SaveAs);
        }
        let top_separator = menu_separator(ui);
        // The saves' and recents' part scrolls past the window's bottom:
        // its room is the window's bottom less the bottom rows' and the
        // popup frame's bottom margin.
        let below = top_separator.rect.height()
            + 2.0 * ui.spacing().interact_size.y
            + egui::Frame::popup(ui.style()).total_margin().bottom;
        let max_height = (ui.ctx().content_rect().bottom() - ui.cursor().top() - below).max(0.0);
        // The solid bar takes its strip out of the scroll's width, which
        // the popup's sizing pass measured without it: a scrolling menu
        // asks for the widest entry plus the strip, so no entry wraps or
        // clips under the bar. A menu that fits keeps its width.
        let (widest, height) = measure(ui, |ui| menu_middle(ui, &opens).0);
        let scroll = egui::style::ScrollStyle::solid();
        if height > max_height {
            ui.set_min_width(widest + scroll.allocated_width());
        }
        ui.spacing_mut().scroll = scroll;
        egui::ScrollArea::vertical()
            .max_height(max_height)
            .show(ui, |ui| {
                if let Some(pending) = menu_middle(ui, &opens).1 {
                    chosen = Some(pending);
                }
            });
        menu_separator(ui);
        if add_enabled(
            ui,
            save_open,
            egui::Button::new("Close save").shortcut_text("Ctrl+W"),
        )
        .clicked()
        {
            chosen = Some(Pending::CloseSave);
        }
        if ui
            .add(egui::Button::new("Exit").shortcut_text("Ctrl+Q"))
            .clicked()
        {
            view.request_close = true;
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

/// The File menu's middle, the part that scrolls when the menu runs past
/// the window's bottom: the `Open from Documents` heading over the
/// discovered saves (the weak line with none found), then the `Recent
/// files` heading over the recents. Returns the widest entry's
/// intrinsic width, which `show` sizes a scrolling menu by, and the
/// clicked entry's `Pending`.
fn menu_middle(ui: &mut egui::Ui, opens: &Opens) -> (f32, Option<Pending>) {
    let mut widest = 0.0_f32;
    let mut entry = |ui: &mut egui::Ui, open: &Open| {
        let response = ui.add(open.button());
        widest = widest.max(response.intrinsic_size().unwrap_or(response.rect.size()).x);
        response
            .on_hover_text(&open.tip)
            .clicked()
            .then(|| open.pending.clone())
    };
    group_heading(ui, egui::RichText::new("Open from Documents").weak());
    if opens.saves.is_empty() {
        group_heading(
            ui,
            egui::RichText::new("No PES saves were found in Documents.").weak(),
        );
    }
    let mut chosen = None;
    for open in &opens.saves {
        chosen = entry(ui, open).or(chosen);
    }
    if !opens.recents.is_empty() {
        menu_separator(ui);
        group_heading(ui, egui::RichText::new("Recent files").weak());
        for open in &opens.recents {
            chosen = entry(ui, open).or(chosen);
        }
    }
    (widest, chosen)
}

/// Lays `body` out in an invisible child of `ui`, its widgets placed but
/// never painted or hit, and returns what `body` returns (both callers'
/// bodies return their widest entry's width) with the height the layout
/// settles at.
fn measure(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui) -> f32) -> (f32, f32) {
    let mut probe = ui.new_child(egui::UiBuilder::new().invisible());
    let widest = body(&mut probe);
    (widest, probe.min_rect().height())
}

/// The welcome panel (the draft's `welcomePanel`), drawn in place of the
/// panes with no save open: the File menu's opens in the dialog's frame
/// and title in the middle of the view, but a page, not
/// a modal — nothing under it is blocked, the sidebar's tool switch
/// included. Discovery runs once when the panel appears and the list is
/// kept while it stays up, never a scan a frame.
pub(super) fn welcome_panel(ui: &mut egui::Ui, view: &mut SaveView) {
    if view.welcome_discovered.is_none() {
        view.welcome_discovered = Some(look(view));
    }
    let opens = opens(view.welcome_discovered.as_ref().expect("filled above"));
    let rect = ui.max_rect();
    let chosen = studio_core::widgets::dialog_page(
        ui,
        egui::Id::unique("save_editor_welcome"),
        "Save editor",
        |ui| {
            // The body scrolls when the window is too short for the
            // list. Its room is the page's height less the frame's
            // margins and the title's row, handed over as the child ui's
            // max_rect: a scroll area that read the frame's own available
            // height would get the height the body settled at, shrink to
            // it and keep its bar drawn.
            let chrome = studio_core::widgets::modal_frame(ui.style())
                .total_margin()
                .sum();
            let title = ui.cursor().top() - ui.min_rect().top();
            let max_height = rect.height() - chrome.y - title;
            // The body's width is its widest entry's — never under the
            // frame's floor (`min_rect` after `dialog_page` set it) — the
            // solid bar's strip beside it when the list scrolls.
            let (widest, height) = measure(ui, |ui| welcome_body(ui, &opens).0);
            let scroll = egui::style::ScrollStyle::solid();
            let bar = if height > max_height {
                scroll.allocated_width()
            } else {
                0.0
            };
            let width = widest.max(ui.min_rect().width());
            let room = egui::Rect::from_min_max(
                ui.cursor().min,
                egui::pos2(
                    ui.cursor().min.x + width + bar,
                    ui.cursor().top() + max_height,
                ),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(room), |ui| {
                ui.spacing_mut().scroll = scroll;
                egui::ScrollArea::vertical()
                    .max_height(max_height)
                    .show(ui, |ui| welcome_body(ui, &opens).1)
                    .inner
            })
            .inner
        },
    )
    .inner;
    if let Some(pending) = chosen {
        request(view, pending);
    }
}

/// The welcome panel's body, the File menu's opens in the menu's order:
/// `Open…` as wide as the entries under it; the `Open from Documents`
/// heading over the discovered saves (the weak line with none found); the
/// `Recent files` heading over the recent saves when there are any.
/// Returns the widest entry's intrinsic width, which `welcome_panel`
/// sizes the body by, and the clicked entry's `Pending`.
fn welcome_body(ui: &mut egui::Ui, opens: &Opens) -> (f32, Option<Pending>) {
    // Justified: every entry fills the body's width — `welcome_panel`
    // sets it to the widest entry's, which `Open::button`'s `Extend`
    // keeps to one line.
    let mut widest = 0.0_f32;
    let mut entry = |ui: &mut egui::Ui, open: &Open| {
        let response = ui.add(open.button());
        widest = widest.max(response.intrinsic_size().unwrap_or(response.rect.size()).x);
        response
            .on_hover_text(&open.tip)
            .clicked()
            .then(|| open.pending.clone())
    };
    let layout = egui::Layout::top_down_justified(egui::Align::Min);
    let chosen = ui
        .with_layout(layout, |ui| {
            let mut chosen = entry(ui, &opens.picker);
            ui.separator();
            group_heading(ui, egui::RichText::new("Open from Documents").strong());
            if opens.saves.is_empty() {
                group_heading(
                    ui,
                    egui::RichText::new("No PES saves were found in Documents.").weak(),
                );
            }
            for open in &opens.saves {
                chosen = entry(ui, open).or(chosen);
            }
            if !opens.recents.is_empty() {
                ui.separator();
                group_heading(ui, egui::RichText::new("Recent files").strong());
                for open in &opens.recents {
                    chosen = entry(ui, open).or(chosen);
                }
            }
            chosen
        })
        .inner;
    (widest, chosen)
}

/// What an Open shortcut asks for.
pub(super) enum Shortcut {
    /// Ctrl+O: the file picker, as `Open…`.
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
    discovered_versions()
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
            picker_request(view.last_folder.as_deref(), &discover_all(view))
        }
    };
    request(view, pending);
}

/// The digit that, with Ctrl, opens `version`'s newest discovered save:
/// 4 to 9 for PES 2014 to PES 2019, then 0 for PES 2020 and 1 for PES 2021.
fn shortcut_key(version: PesVersion) -> egui::Key {
    match version {
        PesVersion::Pes14 => egui::Key::Num4,
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

/// The versions whose saves the File menu lists and its Open shortcuts
/// open, oldest first: `PesVersion::ALL` with PES 2014 before it, the
/// editor being the one tool a PES 2014 save reaches.
pub(super) fn discovered_versions() -> impl Iterator<Item = PesVersion> {
    [PesVersion::Pes14].into_iter().chain(PesVersion::ALL)
}

/// Every discovered version's saves under Documents, each with its
/// version, oldest first (`view.discover`, stubbed in the tests).
fn discover_all(view: &SaveView) -> Vec<(PesVersion, SavefileCandidate)> {
    discovered_versions()
        .flat_map(|version| {
            (view.discover)(version)
                .into_iter()
                .map(move |candidate| (version, candidate))
        })
        .collect()
}

/// `Open…`'s start folder: `last_folder`, the folder the last save was
/// opened from, while it is a folder, else `discovered`'s newest save's
/// folder, else none. `look` resolves it once for its `Found`; the Open
/// shortcuts' fallback applies it per press.
fn picker_folder(
    last_folder: Option<&Path>,
    discovered: &[(PesVersion, SavefileCandidate)],
) -> Option<PathBuf> {
    last_folder
        .filter(|folder| folder.is_dir())
        .or_else(|| {
            discovered
                .iter()
                .max_by_key(|(_, candidate)| candidate.modified)
                .and_then(|(_, candidate)| candidate.path.parent())
        })
        .map(Path::to_path_buf)
}

/// The Open shortcuts' picker request when no look is kept:
/// `picker_folder`'s rule.
fn picker_request(
    last_folder: Option<&Path>,
    discovered: &[(PesVersion, SavefileCandidate)],
) -> Pending {
    Pending::OpenPicker(picker_folder(last_folder, discovered))
}

/// Goes on with `pending`, the unsaved-changes prompt first when there
/// are any.
pub(super) fn request(view: &mut SaveView, pending: Pending) {
    if view.is_dirty() {
        prompt::open(view, pending);
    } else {
        prompt::go_on(view, pending);
    }
}

/// A recent save's entry: its version in brackets, then its parent
/// folder's name and its file name joined by the platform's separator
/// (`[PES 2021] backups\EDIT00000000 - VGL26`); a path with no parent
/// folder name reads `[<version>] <file name>`.
fn recent_label(recent: &RecentSave) -> String {
    let file = recent.path.file_name().map(|name| name.to_string_lossy());
    let folder = recent
        .path
        .parent()
        .and_then(Path::file_name)
        .map(|folder| folder.to_string_lossy());
    let tail = match (folder, file) {
        (Some(folder), Some(file)) => {
            format!("{folder}{}{file}", std::path::MAIN_SEPARATOR_STR)
        }
        (_, Some(file)) => file.into_owned(),
        _ => recent.path.display().to_string(),
    };
    format!("[{}] {tail}", recent.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recent_list(entries: &[(&str, PesVersion)]) -> Vec<RecentSave> {
        entries
            .iter()
            .map(|(path, version)| RecentSave {
                path: PathBuf::from(path),
                version: *version,
            })
            .collect()
    }

    // Discovery covers PES 2014 beside `PesVersion::ALL`, oldest first:
    // its save stands first, then the other versions' in `ALL`'s order.
    #[test]
    fn discover_all_lists_pes14_first_then_all_in_order() {
        let mut view = SaveView::new();
        view.discover = Box::new(|version| {
            vec![SavefileCandidate {
                path: PathBuf::from(format!("{version}/save")),
                account: None,
                modified: std::time::SystemTime::UNIX_EPOCH,
            }]
        });
        let saves = discover_all(&view);
        let versions: Vec<PesVersion> = saves.iter().map(|(version, _)| *version).collect();
        assert_eq!(
            versions,
            [PesVersion::Pes14]
                .into_iter()
                .chain(PesVersion::ALL)
                .collect::<Vec<PesVersion>>(),
            "PES 2014 first, then `PesVersion::ALL`'s order"
        );
        assert_eq!(
            saves[0].1.path,
            PathBuf::from("PES 2014/save"),
            "the PES 2014 entry carries its candidate"
        );
    }

    // Ctrl+4 asks for PES 2014's newest save, as 5 to 9, 0 and 1 do the
    // other versions'.
    #[test]
    fn ctrl_plus_4_consumes_to_pes14s_open_shortcut() {
        let mut input = egui::InputState::default();
        input.events = vec![egui::Event::Key {
            key: egui::Key::Num4,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::CTRL,
        }];
        assert!(matches!(
            consume_shortcut(&mut input),
            Some(Shortcut::Save(PesVersion::Pes14))
        ));
        assert!(input.events.is_empty(), "the press is consumed");
    }

    // Every version's Open shortcut is the digit the File menu's entries
    // show: 5 to 9 for PES 2015 to PES 2019, then 0 and 1.
    #[test]
    fn each_versions_shortcut_reads_its_digit() {
        assert_eq!(
            PesVersion::ALL.map(shortcut_text),
            [
                "Ctrl+5", "Ctrl+6", "Ctrl+7", "Ctrl+8", "Ctrl+9", "Ctrl+0", "Ctrl+1",
            ]
        );
    }

    // The last opened or saved file stands first: a path already in the
    // list moves there once with its new version, a new one prepends.
    #[test]
    fn a_pushed_path_stands_first_once_with_its_new_version() {
        let mut recents = recent_list(&[
            ("a", PesVersion::Pes19),
            ("b", PesVersion::Pes21),
            ("c", PesVersion::Pes17),
        ]);
        push_recent(&mut recents, Path::new("b"), PesVersion::Pes15);
        assert_eq!(
            recents,
            recent_list(&[
                ("b", PesVersion::Pes15),
                ("a", PesVersion::Pes19),
                ("c", PesVersion::Pes17),
            ])
        );
        push_recent(&mut recents, Path::new("d"), PesVersion::Pes21);
        assert_eq!(
            recents,
            recent_list(&[
                ("d", PesVersion::Pes21),
                ("b", PesVersion::Pes15),
                ("a", PesVersion::Pes19),
                ("c", PesVersion::Pes17),
            ])
        );
    }

    // The list is the last 10: the eleventh push drops the oldest.
    #[test]
    fn the_list_is_cut_to_ten() {
        let mut recents = Vec::new();
        for index in 0..15 {
            push_recent(
                &mut recents,
                &PathBuf::from(format!("f/{index}")),
                PesVersion::Pes21,
            );
        }
        assert_eq!(recents.len(), RECENT_MAX);
        assert_eq!(recents[0].path, PathBuf::from("f/14"));
        assert_eq!(recents[9].path, PathBuf::from("f/5"));
    }

    // A recent save's entry reads its version in brackets, then its folder
    // and its file name (`[PES 2021] backups\EDIT00000000 - VGL26`).
    #[test]
    fn a_recents_label_reads_its_version_folder_and_file() {
        let sep = std::path::MAIN_SEPARATOR;
        let save = RecentSave {
            path: PathBuf::from(format!(
                "C:{sep}Documents{sep}backups{sep}EDIT00000000 - VGL26"
            )),
            version: PesVersion::Pes21,
        };
        assert_eq!(
            recent_label(&save),
            format!("[PES 2021] backups{sep}EDIT00000000 - VGL26")
        );
        // A path with no parent folder name reads the file name alone.
        let bare = RecentSave {
            path: PathBuf::from("EDIT00000000"),
            version: PesVersion::Pes19,
        };
        assert_eq!(recent_label(&bare), "[PES 2019] EDIT00000000");
    }
}
