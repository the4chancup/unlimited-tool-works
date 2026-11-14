//! The start page (the draft's `startPage`): with no document the
//! dialog's frame and title centred on the page, the draft's
//! `.start-page` (`page`); over a document, the modal File's "New from
//! template…" opens (`modal`). The two share `body`.

use std::path::PathBuf;
use std::sync::OnceLock;

use aatf::Template;
use studio_core::ToolContext;
use studio_core::widgets::{add_enabled, dialog_page, modal_buttons, modal_frame};

use crate::settings::RulesetEditorSettings;
use crate::view::{Pending, RulesetView, file_label, guard, refused_block};

/// The templates, parsed once (`aatf::templates` compiles each embedded
/// block; the list never changes).
pub(crate) fn templates() -> &'static [Template] {
    static TEMPLATES: OnceLock<Vec<Template>> = OnceLock::new();
    TEMPLATES.get_or_init(aatf::templates)
}

/// The whole view when no document is open: `body` in the modal's
/// frame and title, centred on `ui`'s rect. Not a real dialog, which
/// would hold the whole window — the sidebar's tool switch included —
/// until a ruleset is picked.
pub(crate) fn page(
    ui: &mut egui::Ui,
    view: &mut RulesetView,
    ctx: &ToolContext,
    settings: &RulesetEditorSettings,
) {
    let rect = ui.max_rect();
    dialog_page(
        ui,
        egui::Id::unique("ruleset_start"),
        "Ruleset editor",
        |ui| {
            // The body scrolls when the window is too short, the bound
            // `view::modal` gives the dialog form's: the frame's margins
            // and the title's row off the rect's height.
            let frame = modal_frame(ui.style()).total_margin().sum().y;
            let title = ui.cursor().top() - ui.min_rect().top();
            egui::ScrollArea::vertical()
                .max_height(rect.height() - frame - title)
                .show(ui, |ui| body(ui, view, ctx, settings));
        },
    );
}

/// The start page over the document (File's "New from template…"), with
/// a Close button back to it.
pub(crate) fn modal(
    ui: &mut egui::Ui,
    view: &mut RulesetView,
    ctx: &ToolContext,
    settings: &RulesetEditorSettings,
) {
    if super::modal(ui, "ruleset_start", "Ruleset editor", |ui| {
        body(ui, view, ctx, settings);
        modal_buttons(ui, |ui| ui.button("Close").clicked())
    }) {
        view.start_open = false;
    }
}

/// The start page's entries (RE-GUI-01): New from template in the
/// embedded rulesets' order then Blank; Random, drawn disabled until
/// its build; Open file…; the recent files, newest first, the existing
/// ones only; and the last open's refusal when there was one.
fn body(
    ui: &mut egui::Ui,
    view: &mut RulesetView,
    ctx: &ToolContext,
    settings: &RulesetEditorSettings,
) {
    ui.strong("New from template");
    for (index, template) in templates().iter().enumerate() {
        if ui.button(button_label(template)).clicked() {
            guard(view, ctx, Pending::Template(index));
        }
    }
    if ui
        .button("Blank: one tier, no height brackets, universal rules on")
        .clicked()
    {
        guard(view, ctx, Pending::Blank);
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.strong("Random");
        for preset in ["Sensible", "Weird", "Crazy"] {
            add_enabled(ui, false, egui::Button::new(preset))
                .on_disabled_hover_text("The generator comes in a later build.");
        }
    });
    ui.separator();
    if ui.button("Open file…").clicked() {
        guard(view, ctx, Pending::OpenFile);
    }
    ui.separator();
    ui.strong("Recent files");
    let recent: Vec<PathBuf> = settings
        .recent_files
        .iter()
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .collect();
    if recent.is_empty() {
        ui.weak("none");
    }
    for path in recent {
        if ui
            .button(file_label(&path))
            .on_hover_text(path.display().to_string())
            .clicked()
        {
            guard(view, ctx, Pending::OpenPath(path));
        }
    }
    // The last open's refusal (RE-FILE-03).
    if let Some((name, lines)) = &view.open_refusal {
        ui.separator();
        refused_block(ui, name, lines);
    }
}

/// A template button's label (the draft's): the official ruleset says
/// so, the others are their description alone.
fn button_label(template: &Template) -> String {
    let name = &template.ruleset.name;
    match template.stem {
        "autumn26" => format!("{name} (the official rules)"),
        _ => name.clone(),
    }
}
