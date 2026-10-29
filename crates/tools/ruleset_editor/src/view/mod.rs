//! The Ruleset editor's window: with no document the start page
//! (`start`); with one, the toolbar over
//! the section rail (`rail`), the section forms (`sections`) and the
//! optional data-block preview (`preview`), the bottom status bar, the
//! logic banners and the unsaved-changes prompt (`prompt`). The forms
//! edit the document's ruleset in place, the first change of a gesture
//! snapshots it (one undo entry per gesture), and opens and saves go
//! through `open_path`/`save_to`, the methods the tests drive (every
//! file-dialog call stays in a click handler).

mod preview;
pub(crate) mod prompt;
mod rail;
mod sections;
mod start;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use aatf::Ruleset;
use studio_core::ToolContext;
use studio_core::widgets::{menu_button, modal_buttons, modal_frame};

pub(crate) use prompt::Pending;
pub(crate) use sections::Section;

use crate::document::{Document, Logic, OpenError, SaveError, Snapshot};
use crate::messages::{TOOL_ID, finding_lines};
use crate::settings::{self, RulesetEditorSettings};

/// The window's state: the open document and what the frame draws of it.
pub(crate) struct RulesetView {
    /// The open document; `None` shows the start page.
    pub(crate) document: Option<Document>,
    /// The drawn section.
    pub(crate) section: Section,
    /// The **Show preview** toggle at the toolbar's right: a plain
    /// field, never written to settings, so off at every start. Open
    /// beside the form, the preview leaves the sections a third of the
    /// window, and it is a check on the form, not where the work happens.
    pub(crate) show_preview: bool,
    /// The document as the gesture in flight began, taken at its first
    /// write inside the section form; sealing a changed one pushes the
    /// undo snapshot.
    pending: Option<Snapshot>,
    /// The widget holding the keyboard at the last frame's end (the
    /// Save editor's `focused`): its loss seals the widget's gesture.
    focused: Option<egui::Id>,
    /// The data block's lines as the preview last showed them.
    previous_lines: Vec<String>,
    /// The preview's highlighted lines: the ones the last recorded edit
    /// changed (the draft's `refresh`).
    pub(crate) changed: BTreeSet<String>,
    /// The start page drawn over the document (File's "New from
    /// template…"); `false` while closed.
    pub(crate) start_open: bool,
    /// What the unsaved-changes prompt does once answered; `None` while
    /// the prompt is closed.
    pub(crate) pending_action: Option<Pending>,
    /// A dialog until dismissed (`Dialog`).
    pub(crate) dialog: Option<Dialog>,
    /// The start page's refused-open block (RE-FILE-03): the file's
    /// name and its findings' lines.
    open_refusal: Option<(String, Vec<String>)>,
    /// The file dialogs (`rfd`); stubbed in tests, a native dialog
    /// cannot be clicked headless (the Save editor's `randomize_draw`
    /// field is the precedent).
    pub(crate) open_dialog: FilePicker,
    /// `open_dialog` for Save as.
    pub(crate) save_as_dialog: SaveDialog,
    /// Which document this is: the id salt, so egui widget state never
    /// crosses into the next one (the Save editor's `generation`).
    generation: u64,
}

/// A file-dialog call: the `.4ccrules` dialog's start folder to the
/// picked path (`rfd`, or a test's stub).
type FilePicker = Box<dyn FnMut(&Path) -> Option<PathBuf>>;

/// `FilePicker` for Save as: the suggested file name comes second.
type SaveDialog = Box<dyn FnMut(&Path, &str) -> Option<PathBuf>>;

/// A dialog until dismissed.
#[derive(Debug)]
pub(crate) enum Dialog {
    /// A refused open over a document (the start page's block is the
    /// no-document form): the file's name and its findings' lines.
    OpenRefused {
        /// The file's name.
        name: String,
        /// Its load findings, one line each.
        findings: Vec<String>,
    },
    /// Save wrote nothing: the assembled file failed load-time
    /// validation; the findings name every rule broken (the sample
    /// team's script error included).
    SaveRefused {
        /// Its findings, one line each.
        findings: Vec<String>,
    },
    /// A save's write failed: the file and the error, the document
    /// staying dirty.
    SaveWrite {
        /// `<path>: <error>`.
        text: String,
    },
}

impl Default for RulesetView {
    fn default() -> RulesetView {
        RulesetView::new()
    }
}

impl RulesetView {
    /// The window as it starts: no document open, the preview off.
    pub(crate) fn new() -> RulesetView {
        RulesetView {
            document: None,
            section: Section::General,
            show_preview: false,
            pending: None,
            focused: None,
            previous_lines: Vec::new(),
            changed: BTreeSet::new(),
            start_open: false,
            pending_action: None,
            dialog: None,
            open_refusal: None,
            open_dialog: default_open_dialog(),
            save_as_dialog: default_save_dialog(),
            generation: 0,
        }
    }

    /// `document` open: the view lands on General, the preview's diff
    /// restarts and egui widget state resets (`generation` bumps the
    /// salt); the start page closes.
    fn set_document(&mut self, document: Document) {
        self.section = Section::General;
        self.previous_lines = block_lines(&document.ruleset);
        self.changed = BTreeSet::new();
        self.pending = None;
        self.focused = None;
        self.start_open = false;
        self.open_refusal = None;
        self.document = Some(document);
        self.generation += 1;
    }
}

/// The `.4ccrules` file dialog, started in `start` when it names a
/// folder (only the real dialogs call it; tests stub the pickers).
#[cfg(not(test))]
fn ruleset_dialog(start: &Path) -> rfd::FileDialog {
    let dialog = rfd::FileDialog::new().add_filter("Rules file", &[aatf::RULES_FILE_EXTENSION]);
    if start.as_os_str().is_empty() {
        dialog
    } else {
        dialog.set_directory(start)
    }
}

/// What a test reaching a real file dialog hits instead of a native
/// window (the pickers are stubbed per test).
#[cfg(test)]
const REAL_DIALOG_PANIC: &str =
    "a test reached a real file dialog: stub open_dialog/save_as_dialog";

/// The default Open dialog: `rfd` (reaching a native dialog in a test
/// panics; tests stub `open_dialog`/`save_as_dialog`).
#[cfg(not(test))]
fn default_open_dialog() -> FilePicker {
    Box::new(|start| ruleset_dialog(start).pick_file())
}

/// `default_open_dialog` under `cargo test`.
#[cfg(test)]
fn default_open_dialog() -> FilePicker {
    Box::new(|_| panic!("{REAL_DIALOG_PANIC}"))
}

/// The default Save as dialog: `rfd`, the file name prefilled.
#[cfg(not(test))]
fn default_save_dialog() -> SaveDialog {
    Box::new(|start, suggested| ruleset_dialog(start).set_file_name(suggested).save_file())
}

/// `default_save_dialog` under `cargo test`.
#[cfg(test)]
fn default_save_dialog() -> SaveDialog {
    Box::new(|_, _| panic!("{REAL_DIALOG_PANIC}"))
}

/// `name` as the suggested file name: lowercase, its alphanumeric runs
/// joined by `_`, then `.4ccrules`. Every other character is dropped,
/// not only whitespace, since a name's slash or colon would be read as a
/// folder or refused by the Save dialog (`/vg/ League 27` suggests
/// `vg_league_27.4ccrules`).
fn suggested_name(name: &str) -> String {
    format!(
        "{}.{}",
        name.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|piece| !piece.is_empty())
            .collect::<Vec<_>>()
            .join("_"),
        aatf::RULES_FILE_EXTENSION
    )
}

/// Shows the window for one frame: the settings are read each frame
/// (the recent files follow a file written elsewhere).
pub(crate) fn show(ui: &mut egui::Ui, view: &mut RulesetView, ctx: &ToolContext) {
    set_surrender_on_press(ui.ctx());
    let settings = read_settings(ctx);
    if view.document.is_some() {
        toolbar(ui, view, ctx, &settings);
        egui::Panel::bottom("ruleset_bottom").show(ui, |ui| bottom_bar(ui, view, ctx, &settings));
        if view.show_preview {
            egui::Panel::right("ruleset_preview")
                .resizable(true)
                .default_size(560.0)
                .show(ui, |ui| preview::show(ui, view));
        }
        egui::Panel::left("ruleset_rail")
            .resizable(false)
            .exact_size(140.0)
            .show(ui, |ui| rail::show(ui, view));
        egui::CentralPanel::default().show(ui, |ui| {
            banners(ui, view);
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                ui.push_id(view.generation, |ui| {
                    let document = view.document.as_mut().expect("a document is open");
                    // The forms edit the ruleset in place; a changed
                    // one inside this call is the gesture's first
                    // write, so the pre-call state becomes the
                    // pending snapshot. Nothing else reaches this
                    // call, so undo, redo, a new document and Update
                    // logic are never taken for an edit.
                    let before = document.ruleset.clone();
                    sections::show(
                        ui,
                        view.section,
                        &mut document.ruleset,
                        document.custom_checks.as_deref(),
                    );
                    if view.pending.is_none() && document.ruleset != before {
                        view.pending = Some(Snapshot {
                            ruleset: before,
                            logic: document.logic.clone(),
                        });
                    }
                });
            });
        });
    } else {
        egui::CentralPanel::default().show(ui, |ui| {
            start::page(ui, view, ctx, &settings);
        });
    }
    if view.start_open && view.document.is_some() {
        start::modal(ui, view, ctx, &settings);
    }
    prompt::show(ui, view, ctx, &settings);
    dialog(ui, view);
    seal_gesture(ui, view);
}

/// The editor's focus mode (the Save editor's `set_surrender_on_press`):
/// a press on another widget ends the focused widget's keyboard gesture
/// on the press frame.
fn set_surrender_on_press(ctx: &egui::Context) {
    ctx.memory_mut(|memory| {
        memory.options.input_options.surrender_focus_on = egui::SurrenderFocusOn::Presses;
    });
}

/// `action` on a dirty document goes through the unsaved-changes prompt
/// first; clean, it runs at once.
pub(crate) fn guard(view: &mut RulesetView, ctx: &ToolContext, action: Pending) {
    if view.document.as_ref().is_some_and(Document::is_dirty) {
        view.pending_action = Some(action);
    } else {
        run(view, ctx, action);
    }
}

/// `action` performed: the prompt's Discard and saved Save land here,
/// and the unguarded picks on a clean document.
pub(crate) fn run(view: &mut RulesetView, ctx: &ToolContext, action: Pending) {
    match action {
        Pending::Template(index) => {
            let template = &start::templates()[index];
            let mut document = Document::new(template.ruleset.clone());
            document.custom_checks = template.custom_checks.clone();
            view.set_document(document);
        }
        Pending::Blank => view.set_document(Document::new(crate::document::blank())),
        Pending::OpenFile => {
            // Read at the pick: the prompt's own save may have moved
            // `last_folder` since `show`'s snapshot at the frame's top.
            let start = PathBuf::from(&read_settings(ctx).last_folder);
            if let Some(path) = (view.open_dialog)(&start) {
                open_path(view, ctx, &path);
            }
        }
        Pending::OpenPath(path) => open_path(view, ctx, &path),
        Pending::Close => {
            view.document = None;
            view.start_open = false;
            ctx.request_close();
        }
    }
}

/// Opens `path` (`Document::open`): what the file dialogs and the
/// recent-files buttons land on, and the tests drive. On success the
/// document replaces the open one and the file is remembered; on
/// refusal the findings list shows (the start page's block with no
/// document, a dialog over one).
pub(crate) fn open_path(view: &mut RulesetView, ctx: &ToolContext, path: &Path) {
    match Document::open(path) {
        Ok(document) => {
            view.set_document(document);
            view.open_refusal = None;
            view.dialog = None;
            remember_path(ctx, path);
        }
        Err(OpenError::Invalid(error)) => {
            refuse_open(view, path, finding_lines(&error));
        }
        Err(OpenError::Read { error, .. }) => {
            refuse_open(view, path, vec![error.to_string()]);
        }
    }
}

/// A refused open's landing: the file's name and its findings' lines.
fn refuse_open(view: &mut RulesetView, path: &Path, lines: Vec<String>) {
    let name = file_label(path);
    if view.document.is_none() {
        view.open_refusal = Some((name, lines));
    } else {
        view.dialog = Some(Dialog::OpenRefused {
            name,
            findings: lines,
        });
    }
}

/// The toolbar's and bottom bar's Save and Ctrl+S: no path is Save as.
/// Returns whether the file was written.
pub(crate) fn save_now(
    view: &mut RulesetView,
    ctx: &ToolContext,
    settings: &RulesetEditorSettings,
) -> bool {
    match view
        .document
        .as_ref()
        .and_then(|document| document.path.clone())
    {
        Some(path) => save_to(view, ctx, &path),
        None => save_as(view, ctx, settings),
    }
}

/// Save as…: the `.4ccrules` dialog starting in `last_folder`,
/// suggesting the ruleset's name slugged, then the same path as Save.
fn save_as(view: &mut RulesetView, ctx: &ToolContext, settings: &RulesetEditorSettings) -> bool {
    let start = PathBuf::from(&settings.last_folder);
    let suggested = view
        .document
        .as_ref()
        .map(|document| suggested_name(&document.ruleset.name))
        .unwrap_or_default();
    match (view.save_as_dialog)(&start, &suggested) {
        Some(path) => save_to(view, ctx, &path),
        None => false,
    }
}

/// The write the dialogs and Save land on (the method tests drive): on
/// success the path is remembered and `true`; a save refused by the
/// load-time validation lists its findings, a failed write names the
/// file and the error, each as a dialog and `false`, the document
/// staying dirty.
pub(crate) fn save_to(view: &mut RulesetView, ctx: &ToolContext, path: &Path) -> bool {
    let Some(document) = view.document.as_mut() else {
        return false;
    };
    match document.save(path) {
        Ok(()) => {
            remember_path(ctx, path);
            true
        }
        Err(SaveError::Invalid(error)) => {
            view.dialog = Some(Dialog::SaveRefused {
                findings: finding_lines(&error),
            });
            false
        }
        Err(SaveError::Write { path, error }) => {
            view.dialog = Some(Dialog::SaveWrite {
                text: format!("{}: {error}", path.display()),
            });
            false
        }
    }
}

/// `path` as shown to a member: its file name, the whole path when it
/// has none.
pub(crate) fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// The tool's settings as the table holds them now; a table that fails
/// to read counts as the defaults (the start page then lists no recent
/// files).
fn read_settings(ctx: &ToolContext) -> RulesetEditorSettings {
    settings::from_table(&ctx.tool_settings(TOOL_ID)).unwrap_or_default()
}

/// `path` into the recent files and `last_folder`, written back through
/// `set_tool_settings` after every open and save. The two keys are written
/// into the table as read, so a
/// key this version does not know survives (`spec/core/gui.md`
/// "Settings menu"). A table that fails to read is not overwritten: the
/// recent files it may hold would be emptied, so the write is skipped
/// and the error logged.
fn remember_path(ctx: &ToolContext, path: &Path) {
    let mut table = ctx.tool_settings(TOOL_ID);
    let mut settings = match settings::from_table(&table) {
        Ok(settings) => settings,
        Err(error) => {
            log::warn!(
                "the {TOOL_ID} settings table does not read, the recent files are not updated: {error}"
            );
            return;
        }
    };
    settings.remember(path);
    table.extend(settings.to_table());
    ctx.set_tool_settings(TOOL_ID, table);
}

/// The toolbar (the draft's top bar): the File, Edit and Randomize
/// menus, the document's name with the unsaved dot, and right-aligned
/// the **Show preview** toggle.
fn toolbar(
    ui: &mut egui::Ui,
    view: &mut RulesetView,
    ctx: &ToolContext,
    settings: &RulesetEditorSettings,
) {
    // Ctrl+S saves even while a widget holds the keyboard; the other
    // shortcuts do nothing then (a text field's own undo is egui's).
    // None of them act behind a modal (the prompt, a dialog, the modal
    // start page): an undo under the prompt would write a state it
    // never showed.
    let keys_free = !modal_open(view);
    let save_key =
        keys_free && ui.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::S));
    let typing = ui.ctx().egui_wants_keyboard_input();
    // Ctrl+Shift+Z first: `consume_key(Ctrl, Z)` matches it too, so the
    // more specific shortcut has to be tested before the plainer one.
    let redo_key = keys_free
        && !typing
        && ui.input_mut(|input| {
            input.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z)
                || input.consume_key(egui::Modifiers::CTRL, egui::Key::Y)
        });
    let undo_key = keys_free
        && !typing
        && !redo_key
        && ui.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Z));
    egui::Panel::top("ruleset_toolbar").show(ui, |ui| {
        ui.horizontal(|ui| {
            menu_button(ui, "File", |ui| {
                // "New from template…" opens the start page over the
                // document at once: it loses nothing (Close returns to
                // it), so no prompt; the picks inside are guarded.
                if menu_item(ui, true, egui::Button::new("New from template…")) {
                    view.start_open = true;
                }
                if menu_item(ui, true, egui::Button::new("Open…")) {
                    guard(view, ctx, Pending::OpenFile);
                }
                if menu_item(ui, true, egui::Button::new("Save")) {
                    save_now(view, ctx, settings);
                }
                if menu_item(ui, true, egui::Button::new("Save as…")) {
                    save_as(view, ctx, settings);
                }
            });
            let (can_undo, can_redo) = view
                .document
                .as_ref()
                .map(|document| (document.can_undo(), document.can_redo()))
                .unwrap_or_default();
            menu_button(ui, "Edit", |ui| {
                if menu_item(
                    ui,
                    can_undo,
                    egui::Button::new("Undo").shortcut_text("Ctrl+Z"),
                ) {
                    history_step(view, Document::undo);
                }
                if menu_item(
                    ui,
                    can_redo,
                    egui::Button::new("Redo").shortcut_text("Ctrl+Y"),
                ) {
                    history_step(view, Document::redo);
                }
            });
            // The generator is a later build's (20.7).
            ui.add_enabled_ui(false, |ui| {
                menu_button(ui, "Randomize", |_| ());
            });
            ui.separator();
            if let Some(document) = view.document.as_ref() {
                let marker = if document.is_dirty() { " •" } else { "" };
                ui.strong(format!("{}{marker}", document.ruleset.name));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(egui::Button::selectable(view.show_preview, "Show preview"))
                    .on_hover_text("Show the rules file's data block as the editor writes it")
                    .clicked()
                {
                    view.show_preview = !view.show_preview;
                }
            });
        });
    });
    if save_key {
        save_now(view, ctx, settings);
    }
    if undo_key {
        history_step(view, Document::undo);
    }
    if redo_key {
        history_step(view, Document::redo);
    }
}

/// One menu item: added enabled or not, the menu closed on a click, the
/// click returned.
fn menu_item(ui: &mut egui::Ui, enabled: bool, button: egui::Button) -> bool {
    let clicked = ui.add_enabled(enabled, button).clicked();
    if clicked {
        ui.close();
    }
    clicked
}

/// One modal's scaffolding (the prompt's, the start page's and the
/// dialogs' share it): `studio_core`'s modal under `egui::Id::unique(id)`
/// with `title`, `body` drawn inside a scrolling area under it and
/// returning whether it asks to close; the return is whether the modal
/// closes — the body's ask or a dismissal.
pub(crate) fn modal(
    ui: &mut egui::Ui,
    id: &str,
    title: impl Into<egui::RichText>,
    body: impl FnOnce(&mut egui::Ui) -> bool,
) -> bool {
    let mut close = false;
    let response = studio_core::widgets::modal(ui.ctx(), egui::Id::unique(id), title, |ui| {
        // The bound keeps the modal on the screen: an `Area` taller
        // than it is centered with both ends off it, so an unbounded
        // body's first and last rows are unreachable (a start page's
        // recent files, a long findings list). egui bounds a new
        // modal's first frame itself — a new `Area` opens at
        // `spacing.default_area_size`, at most the screen — so the cap
        // binds only once the window shrinks under an open modal (the
        // shell sets no minimum). The frame's margins and stroke, and
        // the title above the area, are the rest of the modal's height.
        let frame = modal_frame(ui.style()).total_margin().sum().y;
        let title = ui.cursor().top() - ui.min_rect().top();
        // The body is at least as wide as the modal, so the button row
        // it ends in reaches the modal's right edge.
        let width = ui.min_rect().width();
        egui::ScrollArea::vertical()
            .max_height(ui.ctx().content_rect().height() - frame - title)
            .show(ui, |ui| {
                ui.set_min_width(width);
                close = body(ui);
            });
    });
    close || response.should_close()
}

/// Whether a modal is up over the document: the unsaved-changes prompt,
/// a dialog, or the modal start page (the keyboard shortcuts' gate).
fn modal_open(view: &RulesetView) -> bool {
    view.pending_action.is_some() || view.dialog.is_some() || view.start_open
}

/// `view`'s pending gesture recorded when its snapshot differs from the
/// document's (the `seal_gesture` test, shared with `history_step`:
/// undo seals a gesture in flight rather than lose its start).
fn seal_pending(view: &mut RulesetView) {
    if let Some(before) = view.pending.take()
        && let Some(document) = view.document.as_mut()
        && document.snapshot() != before
    {
        document.record(before);
        remember_changed(view);
    }
}

/// Undo or redo, by button or key: the gesture in flight seals first
/// (its snapshot is the state the step restores), then steps; a
/// restored state is a change the preview shows.
fn history_step(view: &mut RulesetView, step: fn(&mut Document) -> bool) {
    seal_pending(view);
    if view.document.as_mut().is_some_and(step) {
        remember_changed(view);
    }
}

/// The logic banners under the toolbar: the file's own code kept, or an
/// older official one with
/// **Update logic**.
fn banners(ui: &mut egui::Ui, view: &mut RulesetView) {
    let Some(document) = view.document.as_mut() else {
        return;
    };
    let mut update = false;
    if matches!(document.logic, Logic::Modified { .. }) {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            "This file's checking code was changed by hand, so its special rules may not \
             behave as the forms describe. Saving keeps that code.",
        );
    } else if matches!(document.logic, Logic::Older { .. }) {
        ui.horizontal(|ui| {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "This file's checking code is an older official version. It keeps working as \
                 it is until you update it.",
            );
            update = ui.button("Update logic").clicked();
        });
    }
    if update {
        // Update logic is an edit like a form's: a gesture in flight
        // seals first, then the pre-click snapshot records (as
        // `history_step` does for undo and redo).
        seal_pending(view);
        let document = view.document.as_mut().expect("a document is open");
        document.record(document.snapshot());
        document.update_logic();
    }
}

/// The bottom bar (the draft's): the problem count or the valid line,
/// and a Save button.
fn bottom_bar(
    ui: &mut egui::Ui,
    view: &mut RulesetView,
    ctx: &ToolContext,
    settings: &RulesetEditorSettings,
) {
    ui.horizontal(|ui| {
        let problems = view
            .document
            .as_ref()
            .map(|document| document.ruleset.check())
            .unwrap_or_default();
        match problems.first() {
            None => {
                ui.label("✔ Valid: the Save editor and the Team creator can use it");
            }
            Some(first) => {
                let all = problems
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n");
                let plural = if problems.len() > 1 { "s" } else { "" };
                let more = if problems.len() > 1 {
                    " (hover for all)"
                } else {
                    ""
                };
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    format!("✖ {} problem{plural}: {first}{more}", problems.len()),
                )
                .on_hover_text(all);
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Save").clicked() {
                save_now(view, ctx, settings);
            }
        });
    });
}

/// A refused open's listing (RE-FILE-03): the file's name, then one
/// line per finding as `validate` prints them — the start page's block
/// and the dialog share it.
pub(crate) fn refused_block(ui: &mut egui::Ui, name: &str, lines: &[String]) {
    ui.colored_label(
        ui.visuals().error_fg_color,
        format!("Could not open {name}:"),
    );
    for line in lines {
        ui.colored_label(ui.visuals().error_fg_color, line);
    }
}

/// The dialogs (`Dialog`): a refused open over a document, a refused
/// save, a failed write.
fn dialog(ui: &mut egui::Ui, view: &mut RulesetView) {
    let Some(kind) = &view.dialog else {
        return;
    };
    let title = match kind {
        Dialog::OpenRefused { .. } => "Not opened",
        Dialog::SaveRefused { .. } => "Not saved: the ruleset has problems",
        Dialog::SaveWrite { .. } => "Not saved",
    };
    let close = modal(ui, "ruleset_dialog", title, |ui| {
        match kind {
            Dialog::OpenRefused { name, findings } => refused_block(ui, name, findings),
            Dialog::SaveRefused { findings } => {
                for line in findings {
                    ui.colored_label(ui.visuals().error_fg_color, line);
                }
            }
            Dialog::SaveWrite { text } => {
                ui.colored_label(ui.visuals().error_fg_color, text);
            }
        }
        modal_buttons(ui, |ui| ui.button("OK").clicked())
    });
    if close {
        view.dialog = None;
    }
}

/// One undo entry per gesture, sealed as the Save editor seals its own:
/// the pre-gesture snapshot `edit` took is recorded when the gesture ends,
/// on the pointer up with no widget holding the keyboard, or when the
/// focused widget loses it. A `None` to `Some` focus
/// move never seals.
fn seal_gesture(ui: &mut egui::Ui, view: &mut RulesetView) {
    let focused = ui.ctx().memory(|memory| memory.focused());
    let sealed = (!ui.input(|input| input.pointer.any_down()) && focused.is_none())
        || (view.focused.is_some() && view.focused != focused);
    if sealed {
        seal_pending(view);
    }
    view.focused = focused;
}

/// `changed` recomputed after a recorded edit or an undo/redo: the data
/// block's lines the block before did not hold (the draft's `refresh`).
fn remember_changed(view: &mut RulesetView) {
    let Some(document) = view.document.as_ref() else {
        return;
    };
    let lines = block_lines(&document.ruleset);
    let before: BTreeSet<&str> = view.previous_lines.iter().map(String::as_str).collect();
    view.changed = lines
        .iter()
        .filter(|line| !before.contains(line.as_str()))
        .cloned()
        .collect();
    view.previous_lines = lines;
}

/// `ruleset`'s data block as the preview draws it, one string per line.
fn block_lines(ruleset: &Ruleset) -> Vec<String> {
    ruleset.to_data_block().lines().map(str::to_owned).collect()
}
