//! The unsaved-changes prompt (RE-GUI-15): "Save changes to <name>?"
//! with Save, Discard and Cancel, met by a template pick, an Open, or a
//! window close on a dirty document.

use std::path::PathBuf;

use studio_core::ToolContext;
use studio_core::widgets::modal_buttons;

use crate::settings::RulesetEditorSettings;
use crate::view::{RulesetView, modal, run, save_now};

/// What the prompt does once answered.
#[derive(Debug)]
pub(crate) enum Pending {
    /// A start-page template button (`start::templates()`'s index).
    Template(usize),
    /// The start page's Blank button.
    Blank,
    /// "Open file…": the file dialog follows the answer.
    OpenFile,
    /// A recent-files button: the path is already known.
    OpenPath(PathBuf),
    /// The window's close (`Tool::hold_close`): Discard asks the shell
    /// to close again, and the document is dropped so the re-asked
    /// `hold_close` finds nothing to keep.
    Close,
}

/// The prompt's buttons.
#[derive(Debug)]
enum Answer {
    /// Writes first (Save as for a never-saved document), then goes on.
    Save,
    /// Goes on without writing.
    Discard,
    /// Keeps everything.
    Cancel,
}

/// The prompt while `view.pending_action` holds one: Save writes first
/// and goes on only if the save succeeded, Discard goes on, Cancel
/// keeps everything; a dismissal counts as Cancel.
pub(crate) fn show(
    ui: &mut egui::Ui,
    view: &mut RulesetView,
    ctx: &ToolContext,
    settings: &RulesetEditorSettings,
) {
    if view.pending_action.is_none() {
        return;
    }
    let mut answer = None;
    let name = view
        .document
        .as_ref()
        .map(|document| document.ruleset.name.clone())
        .unwrap_or_default();
    // The body returns false: a clicked answer decides below what
    // happens, and only a dismissal (Esc, the backdrop) counts as
    // Cancel.
    let dismissed = modal(
        ui,
        "ruleset_unsaved",
        format!("Save changes to {name}?"),
        |ui| {
            modal_buttons(ui, |ui| {
                if ui.button("Cancel").clicked() {
                    answer = Some(Answer::Cancel);
                }
                if ui.button("Discard").clicked() {
                    answer = Some(Answer::Discard);
                }
                if ui.button("Save").clicked() {
                    answer = Some(Answer::Save);
                }
            });
            false
        },
    );
    if dismissed {
        answer = Some(Answer::Cancel);
    }
    // A refused or cancelled save keeps the prompt (going on only if
    // the save succeeded).
    let go_on = match answer {
        Some(Answer::Cancel) => {
            view.pending_action = None;
            false
        }
        Some(Answer::Discard) => true,
        Some(Answer::Save) => save_now(view, ctx, settings),
        None => false,
    };
    if go_on && let Some(action) = view.pending_action.take() {
        run(view, ctx, action);
    }
}
