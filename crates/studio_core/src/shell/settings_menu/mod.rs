//! The settings menu, as the draft `docs/prototypes/index.html` draws it: an `egui::Modal`
//! the sidebar's gear opens, as does a notice's
//! `ActionEffect::OpenSettings`. It holds the common grid every tool shares — one row
//! per `CommonSettings` field a member edits, in the draft's order and wording — then
//! one `CollapsingHeader` per registered tool that has settings, its body the tool's
//! `settings_view`. A tool has settings when its `default_settings()` is not empty and
//! the build does not mark it CLI-only: such a tool's window is not built, so its keys
//! are edited in the settings file. `last_tool`, `last_update_check` and
//! `skipped_version`, the state the shell writes itself, have no row.
//!
//! There is no Save or Apply: a widget's change writes the setting under the settings
//! lock and asks the shell to save at the end of the frame. A text row commits when its
//! edit loses focus or Enter is pressed, not per keystroke: a half-typed path would
//! otherwise reach every tool reading it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::thread::available_parallelism;

use pes_version::PesVersion;

use crate::settings::{CommonSettings, Theme, pes_folder_pattern};
use crate::tool::{ShellRequest, StudioTool, ToolContext};
use crate::widgets::{
    PathCommit, PathKind, checkbox, path_row, path_row_commit, row_label, step_choice, step_number,
    wheel_steps,
};

use super::sidebar::{Sidebar, exe_tooltip, version_blocker, version_label};

/// The modal's width, the draft's `.settings-body`, which the common grid's
/// widest row (the label column, the path edit and Browse) fits.
const WIDTH: f32 = 660.0;

/// The menu's state across frames: whether it is open, the tool the next drawn frame
/// expands, the "A folder per version" toggle's member-set half, and the path rows'
/// edit buffers.
#[derive(Default)]
pub(super) struct SettingsMenu {
    /// The modal is drawn while set.
    open: bool,
    /// The index of the tool whose section the next drawn frame expands, every other
    /// collapsed: the open set is fixed at each opening, then the member's toggles hold
    /// while the menu stays open (ST-SET-01, the draft's `openSettings`).
    pending: Option<usize>,
    /// `pes_folder_path` while its row is edited.
    pes_folder: String,
    /// `exports_folder_path` while its row is edited.
    exports_folder: String,
    /// The "A folder per version" toggle: view-only, never a setting. Any filled
    /// `[common.pes_folders]` entry sets it and holds it on until every row is emptied.
    per_version: bool,
    /// The per-version rows' edit buffers, one per `PesVersion::ALL` entry.
    per_version_folders: BTreeMap<PesVersion, String>,
}

impl SettingsMenu {
    /// Opens the menu on `tools[active]`'s section when it has one, the buffers filled
    /// from the settings and the toggle off (the first frame switches it on when any
    /// version has a folder of its own).
    pub(super) fn open(&mut self, active: usize, ctx: &ToolContext) {
        self.open = true;
        self.pending = Some(active);
        let common = ctx.common();
        self.pes_folder = common.pes_folder_path;
        self.exports_folder = common.exports_folder_path.display().to_string();
        self.per_version = false;
        self.per_version_folders = PesVersion::ALL
            .into_iter()
            .map(|version| {
                (
                    version,
                    common
                        .pes_folders
                        .get(&version)
                        .cloned()
                        .unwrap_or_default(),
                )
            })
            .collect();
    }

    /// Closes the menu without a click: a held window close makes the holder's prompt the
    /// view, and the menu's modal must not cover it.
    pub(super) fn close(&mut self) {
        self.open = false;
    }

    /// Draws the modal while open: the common grid, then one `CollapsingHeader` per tool
    /// with settings in sidebar order, none for the tools `cli_only` names. `sidebar` is
    /// the PES version row's: a pick goes through the selector's one function, and the
    /// row's tooltips are the selector's blocker reason and exe check.
    pub(super) fn show(
        &mut self,
        egui_ctx: &egui::Context,
        tools: &mut [Box<dyn StudioTool>],
        cli_only: &[&str],
        ctx: &ToolContext,
        sidebar: &mut Sidebar,
    ) {
        if !self.open {
            return;
        }
        let id = egui::Id::unique("settings-menu");
        let modal = crate::widgets::modal(egui_ctx, id, "Settings", |ui| {
            ui.set_min_width(WIDTH);
            self.common_grid(ui, tools, ctx, sidebar);
            let pending = self.pending.take();
            for (index, tool) in tools.iter_mut().enumerate() {
                if tool.default_settings().is_empty() || cli_only.contains(&tool.id()) {
                    continue;
                }
                let mut header =
                    egui::CollapsingHeader::new(tool.label()).id_salt(("settings", tool.id()));
                // Each opening expands the active tool's section and collapses the rest;
                // without a pending one the stored state stands, so the member's toggles
                // hold while the menu stays open.
                if let Some(active) = pending {
                    header = header.open(Some(index == active));
                }
                header.show(ui, |ui| tool.settings_view(ui, ctx));
            }
            crate::widgets::modal_buttons(ui, |ui| ui.button("Close").clicked())
        });
        if modal.inner || modal.should_close() {
            self.open = false;
        }
    }

    /// The common grid, always shown (no header): one row per field a member edits. A
    /// widget's change writes the common setting under the lock and asks the shell to
    /// save, as every settings change does.
    fn common_grid(
        &mut self,
        ui: &mut egui::Ui,
        tools: &[Box<dyn StudioTool>],
        ctx: &ToolContext,
        sidebar: &mut Sidebar,
    ) {
        let mut common = ctx.common();
        let mut changed = false;
        egui::Grid::new("settings-common")
            .num_columns(2)
            .show(ui, |ui| {
                version_row(ui, tools, &common, ctx, sidebar);
                let commit = path_row_commit(
                    ui,
                    "PES folder",
                    "pes_folder_path",
                    "The folder PES is installed in, where ** stands for the version's \
                     two digits (20** becomes 2019 for PES 2019). When the folder you \
                     browse to has the selected version's year in its name, the year \
                     is stored as 20** so that switching versions switches folders \
                     too. Any other folder is stored as you picked it.",
                    &mut self.pes_folder,
                    &mut common.pes_folder_path,
                    PathKind::Folder,
                );
                if commit == Some(PathCommit::Picked) {
                    common.pes_folder_path =
                        pes_folder_pattern(Path::new(&common.pes_folder_path), common.pes_version);
                    self.pes_folder.clone_from(&common.pes_folder_path);
                }
                if commit.is_some() {
                    changed = true;
                }
                // The toggle is the view's alone, not a setting: it is on while any
                // override exists and cannot be switched off until the rows are
                // emptied, since a hidden override would still apply and hide why a
                // folder is used. Once seen on it stays on, so emptying the last row
                // does not take the rows away under the member's edit.
                let overrides = !common.pes_folders.is_empty();
                self.per_version |= overrides;
                ui.label("");
                ui.add_enabled(!overrides, |ui: &mut egui::Ui| {
                    checkbox(ui, &mut self.per_version, "A folder per version")
                })
                .on_hover_text(
                    "Gives each PES version a folder of its own, for installs \
                         whose folders do not follow one pattern.",
                )
                .on_disabled_hover_text(
                    "Empty every version's folder to switch this off, because a \
                         filled one is still used while it is hidden.",
                );
                ui.end_row();
                if self.per_version {
                    for version in PesVersion::ALL {
                        let label = format!("PES {} folder", version.year());
                        let key = format!("pes_folders.{}", version.number());
                        let tip = format!(
                            "PES {}'s own folder. When it is filled, it is used as \
                             written instead of the PES folder above, and when it is \
                             empty, the PES folder above is used.",
                            version.year()
                        );
                        let mut current = common
                            .pes_folders
                            .get(&version)
                            .cloned()
                            .unwrap_or_default();
                        if path_row(
                            ui,
                            &label,
                            &key,
                            &tip,
                            self.per_version_folders.entry(version).or_default(),
                            &mut current,
                            PathKind::Folder,
                        ) {
                            if current.is_empty() {
                                common.pes_folders.remove(&version);
                            } else {
                                common.pes_folders.insert(version, current);
                            }
                            changed = true;
                        }
                    }
                }
                let mut exports = common.exports_folder_path.display().to_string();
                if path_row(
                    ui,
                    "Exports folder",
                    "exports_folder_path",
                    "Where exports are picked up.",
                    &mut self.exports_folder,
                    &mut exports,
                    PathKind::Folder,
                ) {
                    common.exports_folder_path = PathBuf::from(exports);
                    changed = true;
                }
                let threads = row_label(
                    ui,
                    "Threads",
                    "thread_count",
                    "The number of worker threads a tool's run uses, where 0 means \
                     automatic (the logical cores minus one).",
                );
                let max = available_parallelism()
                    .map(|count| count.get())
                    .unwrap_or(1);
                let mut thread_count = common.thread_count;
                // Every `DragValue` here clamps only what the member enters: egui's default
                // clamps the stored value on sight and reports it changed, so merely opening
                // the menu would rewrite a hand-set value (a portable folder's thread count
                // from a bigger PC).
                let mut response = ui
                    .add(
                        egui::DragValue::new(&mut thread_count)
                            .range(0..=max)
                            .clamp_existing_to_range(false),
                    )
                    .labelled_by(threads.id);
                if step_number(&mut thread_count, wheel_steps(ui, &response), 0..=max) {
                    response.mark_changed();
                }
                if response.changed() {
                    common.thread_count = thread_count;
                    changed = true;
                }
                ui.end_row();

                let memory = row_label(
                    ui,
                    "Memory cap",
                    "memory_cap_percent",
                    "The most memory a tool's run may budget for, as a percentage \
                     of physical memory.",
                );
                let mut memory_cap = common.memory_cap_percent;
                let mut response = ui
                    .add(
                        egui::DragValue::new(&mut memory_cap)
                            .range(0.1..=100.0)
                            .clamp_existing_to_range(false)
                            .suffix(" %"),
                    )
                    .labelled_by(memory.id);
                if step_number(&mut memory_cap, wheel_steps(ui, &response), 0.1..=100.0) {
                    response.mark_changed();
                }
                if response.changed() {
                    common.memory_cap_percent = memory_cap;
                    changed = true;
                }
                ui.end_row();

                let hide_boots = row_label(
                    ui,
                    "FPC hide boots ID",
                    "fpc_hide_boots_id",
                    "The boots ID that Hide body and an fpc_on marker write. The \
                     game has no boots with this ID, and 4cc uses 55.",
                );
                let mut hide_boots_id = common.fpc_hide_boots_id;
                let mut response = ui
                    .add(
                        egui::DragValue::new(&mut hide_boots_id)
                            .range(fpc::HideIds::BOOTS)
                            .clamp_existing_to_range(false),
                    )
                    .labelled_by(hide_boots.id);
                let steps = wheel_steps(ui, &response);
                if step_number(&mut hide_boots_id, steps, fpc::HideIds::BOOTS) {
                    response.mark_changed();
                }
                if response.changed() {
                    common.fpc_hide_boots_id = hide_boots_id;
                    changed = true;
                }
                ui.end_row();

                let hide_gloves = row_label(
                    ui,
                    "FPC hide gloves ID",
                    "fpc_hide_gloves_id",
                    "The gloves ID that Hide body and an fpc_on marker write. The \
                     game has no gloves with this ID, and 4cc uses 11.",
                );
                let mut hide_gloves_id = common.fpc_hide_gloves_id;
                let mut response = ui
                    .add(
                        egui::DragValue::new(&mut hide_gloves_id)
                            .range(fpc::HideIds::GLOVES)
                            .clamp_existing_to_range(false),
                    )
                    .labelled_by(hide_gloves.id);
                let steps = wheel_steps(ui, &response);
                if step_number(&mut hide_gloves_id, steps, fpc::HideIds::GLOVES) {
                    response.mark_changed();
                }
                if response.changed() {
                    common.fpc_hide_gloves_id = hide_gloves_id;
                    changed = true;
                }
                ui.end_row();

                let theme = row_label(ui, "Theme", "theme", "The window's color theme.");
                let mut picked = common.theme;
                let combo = egui::ComboBox::from_id_salt("settings-theme")
                    .selected_text(theme_label(picked))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut picked, Theme::Dark, "Dark");
                        ui.selectable_value(&mut picked, Theme::Light, "Light");
                    })
                    .response
                    .labelled_by(theme.id);
                let steps = wheel_steps(ui, &combo);
                if let Some(next) = step_choice([Theme::Dark, Theme::Light], &picked, steps) {
                    picked = next;
                }
                if picked != common.theme {
                    common.theme = picked;
                    changed = true;
                }
                ui.end_row();

                // The update check's two rows wait for the updater that reads the
                // keys. Drawn now, a checkbox a member switches off would change
                // nothing.
            });
        if changed {
            ctx.settings().lock().unwrap().common = common;
            ctx.request(ShellRequest::SettingsChanged);
        }
    }
}

/// The `PES version` row: the sidebar selector's twin — the same entries, the same
/// blocker disabling it with `<label>: <reason>` on the disabled hover, and a pick
/// through `Sidebar::pick_version`, which spends the once-per-life auto-switch.
/// Its enabled tooltip is the shared exe check's, as the draft draws it.
fn version_row(
    ui: &mut egui::Ui,
    tools: &[Box<dyn StudioTool>],
    common: &CommonSettings,
    ctx: &ToolContext,
    sidebar: &mut Sidebar,
) {
    let label = row_label(
        ui,
        "PES version",
        "pes_version",
        "The PES version every tool works with, the same setting the sidebar's selector \
         changes.",
    );
    let blocker = version_blocker(tools);
    let mut picked = common.pes_version;
    let combo = ui
        .add_enabled_ui(blocker.is_none(), |ui| {
            egui::ComboBox::from_id_salt("settings-pes-version")
                .selected_text(version_label(picked, false))
                .show_ui(ui, |ui| {
                    for version in PesVersion::ALL {
                        ui.selectable_value(&mut picked, version, version_label(version, false));
                    }
                })
                .response
        })
        .inner
        .labelled_by(label.id);
    if let Some(next) = step_choice(PesVersion::ALL, &picked, wheel_steps(ui, &combo)) {
        picked = next;
    }
    match blocker {
        Some((blocking, reason)) => {
            combo.on_disabled_hover_text(format!("{blocking}: {reason}"));
        }
        None => {
            let check = sidebar.exe_check(common, ui.input(|input| input.focused));
            combo.on_hover_text(exe_tooltip(check, &common.pes_folder()));
        }
    }
    ui.end_row();
    sidebar.pick_version(picked, ctx);
}

/// The theme combo's item label.
fn theme_label(theme: Theme) -> &'static str {
    match theme {
        Theme::Dark => "Dark",
        Theme::Light => "Light",
    }
}

#[cfg(test)]
mod tests;
