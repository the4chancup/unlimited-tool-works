//! The toolbar's ruleset dropdown (the draft's `rulesetPicker`): the AATF
//! rulesets built into the binary by description, in the templates'
//! order, then under a separator the imported ones by description, each
//! with an × removing it, then under another `Import rules file…`, a
//! command rather than a ruleset. A pick writes `view.aatf_ruleset`,
//! which `Tool::view` writes back to the setting; the next frame's `show`
//! resolves it, and the caches keyed to the ruleset follow. The import and
//! the × do their file work in `session::rules` and show its outcome in
//! their dialogs (`dialog`).

use studio_core::widgets::{step_choice, wheel_steps};
use studio_core::{Notice, NoticeKind};

use crate::session::rules::{self, ImportCheck, builtins};
use crate::view::{Refusal, SaveView, comparator};

/// The dropdown's tooltip (the draft's).
const TOOLTIP: &str = "The AATF rules the quick actions, the card counts and Live AATF use";

/// The command row after the rulesets, and its tooltip (the draft's).
const IMPORT_ROW: &str = "Import rules file\u{2026}";
const IMPORT_TOOLTIP: &str = "Add a .4ccrules file to this list; a file with errors is refused";

/// An imported row's × tooltip (the draft's).
const REMOVE_TOOLTIP: &str = "Remove this imported ruleset";

/// The refusal's title when a clean file still cannot be imported.
const NOT_IMPORTED: &str = "The rules file was not imported";

/// The import's or the ×'s dialog, or the import's picker about to open.
pub(super) enum RulesDialog {
    /// Import rules file… was picked: the file picker opens after the
    /// frame's widgets, so an import's new ruleset is resolved on the next
    /// frame rather than halfway through this one.
    Pick,
    /// The picked file does not load: its file name and one line per
    /// finding.
    Errors { file: String, findings: Vec<String> },
    /// The picked file replaces earlier imports: its file name and the
    /// checked import, waiting for Replace.
    Replace { file: String, import: rules::Import },
    /// An imported row's × was clicked: the copy's file name and its
    /// description, waiting for Remove.
    Remove {
        file_name: String,
        description: String,
    },
}

/// The `rfd` picker for Import rules file…, on `.4ccrules` files.
#[cfg(not(test))]
pub(super) fn default_rules_dialog() -> comparator::OpenSaveDialog {
    Box::new(|| {
        rfd::FileDialog::new()
            .add_filter("AATF rules", &[aatf::RULES_FILE_EXTENSION])
            .pick_file()
    })
}

/// `default_rules_dialog` under `cargo test`: a test reaching a real file
/// dialog panics; tests stub `view.rules_file_dialog`.
#[cfg(test)]
pub(super) fn default_rules_dialog() -> comparator::OpenSaveDialog {
    Box::new(|| panic!("a test reached a real file dialog: stub rules_file_dialog"))
}

/// The import's and the ×'s dialogs for one frame, drawn after the
/// frame's widgets: an import or a removal changes the list and the
/// ruleset, which the next frame resolves.
pub(super) fn dialog(ui: &mut egui::Ui, view: &mut SaveView) {
    // The picker before the dialogs, so the dialog its file needs is drawn
    // on this frame too: a modal's first frame only measures it.
    if matches!(view.rules_dialog, Some(RulesDialog::Pick)) {
        view.rules_dialog = (view.rules_file_dialog)().and_then(|path| check(view, &path));
    }
    view.rules_dialog = match view.rules_dialog.take() {
        // A pick was answered just above.
        None | Some(RulesDialog::Pick) => None,
        Some(RulesDialog::Errors { file, findings }) => errors_dialog(ui, file, findings),
        Some(RulesDialog::Replace { file, import }) => replace_dialog(ui, view, file, import),
        Some(RulesDialog::Remove {
            file_name,
            description,
        }) => remove_dialog(ui, view, file_name, description),
    };
}

/// The picked file at `path` checked: its refusal shown, its import done,
/// or the dialog that comes first returned.
fn check(view: &mut SaveView, path: &std::path::Path) -> Option<RulesDialog> {
    let file = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    match rules::check_import(path, &view.imported) {
        ImportCheck::Errors(findings) => Some(RulesDialog::Errors { file, findings }),
        ImportCheck::BuiltinName(description) => {
            view.refusal = Some(Refusal {
                title: NOT_IMPORTED,
                message: format!(
                    "{file} is named {description}, as a built-in ruleset is. Rename it in its \
                     RULESET block, then import it again."
                ),
            });
            None
        }
        ImportCheck::Clean(import) if import.replaces.is_empty() => {
            finish_import(view, &file, &import);
            None
        }
        ImportCheck::Clean(import) => Some(RulesDialog::Replace { file, import }),
    }
}

/// `import` copied into the data folder and picked, with the success
/// notice; a refusal when there is no data folder or the copy fails.
fn finish_import(view: &mut SaveView, file: &str, import: &rules::Import) {
    let Some(data_dir) = view.data_dir.as_deref() else {
        view.refusal = Some(Refusal {
            title: NOT_IMPORTED,
            message: format!(
                "{file} was not imported because there is no data folder to keep its copy in."
            ),
        });
        return;
    };
    match rules::import(data_dir, import) {
        Ok(()) => {
            view.aatf_ruleset = import.file_name.clone();
            view.notice = Some(Notice {
                text: format!("Imported {}.", import.description),
                kind: NoticeKind::Success,
                action: None,
            });
        }
        Err(error) => {
            view.refusal = Some(Refusal {
                title: NOT_IMPORTED,
                message: format!("{file} could not be copied into the data folder: {error}"),
            });
        }
    }
    // Also after a failure: a replaced copy may be gone by then.
    reread_imports(view);
}

/// The imported copy `file_name` deleted; when it was the ruleset in use,
/// the first built-in one is picked, with no notice, since the member
/// chose to remove it.
fn remove_import(view: &mut SaveView, file_name: &str) {
    let data_dir = view
        .data_dir
        .as_deref()
        .expect("an imported row is listed only from a data folder");
    match rules::remove(data_dir, file_name) {
        Ok(()) => {
            if view.aatf_ruleset == file_name {
                view.aatf_ruleset = rules::FIRST_BUILTIN.to_owned();
            }
        }
        Err(error) => {
            view.refusal = Some(Refusal {
                title: "The ruleset was not removed",
                message: format!(
                    "Its copy in the data folder ({file_name}) could not be deleted: {error}"
                ),
            });
        }
    }
    reread_imports(view);
}

/// The imported list read again from the data folder, and the ruleset
/// resolved afresh on the next frame along with what was computed under
/// it: a replaced copy keeps its file name, the key the Cards left badge,
/// the medals and the AATF checks are kept under.
fn reread_imports(view: &mut SaveView) {
    view.imported = rules::imported(view.data_dir.as_deref());
    view.rules = None;
    view.cards = None;
    view.medals = None;
    view.live.check.reset();
    view.multi_aatf.check.reset();
}

/// The refusal for a file that does not load: its findings, one a line
/// in the monospace font, then OK. Returns the dialog while it stays up.
fn errors_dialog(ui: &egui::Ui, file: String, findings: Vec<String>) -> Option<RulesDialog> {
    let mut close = false;
    let response =
        egui::Modal::new(egui::Id::unique("save_editor_rules_errors")).show(ui.ctx(), |ui| {
            studio_core::theme::window_title(ui, "The rules file has errors");
            ui.label(format!(
                "{file} was not imported. Fix these errors, then import it again:"
            ));
            for finding in &findings {
                ui.label(egui::RichText::new(finding).monospace());
            }
            close = ui.button("OK").clicked();
        });
    (!close && !response.should_close()).then_some(RulesDialog::Errors { file, findings })
}

/// The confirmation before an import replaces earlier ones, naming them;
/// Replace imports, Cancel changes nothing. Returns the dialog while it
/// stays up.
fn replace_dialog(
    ui: &egui::Ui,
    view: &mut SaveView,
    file: String,
    import: rules::Import,
) -> Option<RulesDialog> {
    let replaced: Vec<String> = import
        .replaces
        .iter()
        .map(|(description, file_name)| format!("{description} ({file_name})"))
        .collect();
    let text = format!("Importing {file} replaces {}.", replaced.join(" and "));
    let answer = confirm(
        ui,
        "save_editor_rules_replace",
        "Replace an imported ruleset?",
        &text,
        "Replace",
    );
    match answer {
        Some(true) => {
            finish_import(view, &file, &import);
            None
        }
        Some(false) => None,
        None => Some(RulesDialog::Replace { file, import }),
    }
}

/// The ×'s confirmation, naming the ruleset; Remove deletes its copy,
/// Cancel changes nothing. Returns the dialog while it stays up.
fn remove_dialog(
    ui: &egui::Ui,
    view: &mut SaveView,
    file_name: String,
    description: String,
) -> Option<RulesDialog> {
    let text = format!(
        "Remove {description} from the list? Its copy in the data folder is deleted; the file you \
         imported it from ({file_name}) stays."
    );
    let answer = confirm(
        ui,
        "save_editor_rules_remove",
        "Remove an imported ruleset?",
        &text,
        "Remove",
    );
    match answer {
        Some(true) => {
            remove_import(view, &file_name);
            None
        }
        Some(false) => None,
        None => Some(RulesDialog::Remove {
            file_name,
            description,
        }),
    }
}

/// A confirmation modal: `title`, `text`, then `accept` and Cancel. The
/// answer, `None` while it waits; closing it is Cancel.
fn confirm(ui: &egui::Ui, id: &str, title: &str, text: &str, accept: &str) -> Option<bool> {
    let mut answer = None;
    let response = egui::Modal::new(egui::Id::unique(id)).show(ui.ctx(), |ui| {
        studio_core::theme::window_title(ui, title);
        ui.label(text);
        ui.horizontal(|ui| {
            if ui.button(accept).clicked() {
                answer = Some(true);
            }
            if ui.button("Cancel").clicked() {
                answer = Some(false);
            }
        });
    });
    if answer.is_none() && response.should_close() {
        answer = Some(false);
    }
    answer
}

/// The dropdown for one frame: the active ruleset's description closed,
/// as wide as its widest row so a pick never resizes the toolbar.
pub(super) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let active = view
        .rules
        .as_ref()
        .expect("`show` resolves the ruleset before the toolbar draws");
    let key = active.key.clone();
    let selected = active.rules.ruleset().name.clone();
    let font = egui::TextStyle::Button.resolve(ui.style());
    let text_width = |text: &str| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
            .size()
            .x
    };
    // An imported row's × sits inside the row at its right end, a square
    // as tall as the row: its room counts in the row's width, so the text
    // never runs under it.
    let remove_width = ui.spacing().interact_size.y;
    let widest = builtins()
        .iter()
        .map(|(_, name)| text_width(name))
        .chain(
            view.imported
                .iter()
                .map(|import| text_width(&import.rules.ruleset().name) + remove_width),
        )
        .chain([text_width(IMPORT_ROW)])
        .fold(0.0_f32, f32::max);
    // The closed combo's chrome around its text: the button's padding on
    // both sides, the gap and the arrow.
    let chrome = {
        let spacing = ui.spacing();
        2.0 * spacing.button_padding.x + spacing.icon_spacing + spacing.icon_width
    };
    let mut picked = None;
    let mut asked = None;
    let combo = egui::ComboBox::from_id_salt("save_editor_ruleset")
        .selected_text(selected)
        .width(widest + chrome)
        .show_ui(ui, |ui| {
            for (stem, name) in builtins() {
                if ui.selectable_label(key == *stem, name).clicked() {
                    picked = Some((*stem).to_owned());
                }
            }
            if !view.imported.is_empty() {
                ui.separator();
            }
            for import in &view.imported {
                let name = &import.rules.ruleset().name;
                let row = ui.selectable_label(key == import.file_name, name);
                if row.clicked() {
                    picked = Some(import.file_name.clone());
                }
                // Added after the row, over its right end, so the clicks
                // there are the ×'s and the row's hover still spans the
                // popup's width.
                let rect = egui::Rect::from_min_max(
                    egui::pos2(row.rect.max.x - remove_width, row.rect.min.y),
                    row.rect.max,
                );
                let remove = ui.put(
                    rect,
                    egui::Button::new(studio_core::icons::X)
                        .frame(false)
                        .small(),
                );
                remove.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Remove {name}"))
                });
                if remove.on_hover_text(REMOVE_TOOLTIP).clicked() {
                    asked = Some(RulesDialog::Remove {
                        file_name: import.file_name.clone(),
                        description: name.clone(),
                    });
                }
            }
            ui.separator();
            if ui
                .selectable_label(false, IMPORT_ROW)
                .on_hover_text(IMPORT_TOOLTIP)
                .clicked()
            {
                asked = Some(RulesDialog::Pick);
            }
        })
        .response
        .on_hover_text(TOOLTIP);
    // The wheel steps among the rulesets alone: Import rules file… is a
    // command, not a value.
    let rulesets = builtins()
        .iter()
        .map(|(stem, _)| (*stem).to_owned())
        .chain(view.imported.iter().map(|import| import.file_name.clone()));
    if let Some(next) = step_choice(rulesets, &key, wheel_steps(ui, &combo)) {
        picked = Some(next);
    }
    if let Some(picked) = picked {
        view.aatf_ruleset = picked;
    }
    if let Some(asked) = asked {
        view.rules_dialog = Some(asked);
    }
}
