//! The data-block preview: the ruleset's data block (the logo's line
//! shortened) in monospace, read-only, the lines the last edit changed
//! highlighted (the draft's `refresh`).

use std::collections::BTreeSet;

use aatf::Ruleset;

use crate::view::RulesetView;

/// The start of the data block's logo line: the key at the block's
/// indent.
const LOGO_KEY: &str = "    logo: ";

/// The right `egui::Panel`, drawn only while **Show preview** is on.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut RulesetView) {
    let Some(document) = view.document.as_ref() else {
        return;
    };
    ui.horizontal(|ui| {
        ui.strong("Rules file: data block");
        ui.weak("(read-only)");
    });
    let short_logo = short_logo_line(&document.ruleset);
    egui::ScrollArea::both()
        .id_salt("ruleset_preview")
        .auto_shrink(false)
        .show(ui, |ui| {
            for line in document.ruleset.to_data_block().lines() {
                block_line(ui, line, short_logo.as_deref(), &view.changed);
            }
        });
}

/// One whole line of the data block, drawn: the logo's as `short_logo`,
/// since its base64 runs to tens of thousands of characters, which would
/// make the scroll area that wide and its highlight span all of it (the
/// file keeps the line whole). The line is highlighted when `changed`
/// holds it whole: every PNG's base64 starts with the same characters, so
/// two logos of one size in KB shorten to one line, and comparing the
/// shortened lines would show no change when a logo replaces another.
fn block_line(ui: &mut egui::Ui, line: &str, short_logo: Option<&str>, changed: &BTreeSet<String>) {
    let shown = match short_logo {
        Some(short) if line.starts_with(LOGO_KEY) => short,
        _ => line,
    };
    let text = egui::RichText::new(shown).monospace();
    let text = if changed.contains(line) {
        text.background_color(ui.visuals().selection.bg_fill)
    } else {
        text
    };
    ui.add(egui::Label::new(text).selectable(false));
}

/// The preview's logo line: the logo's first 24 base64 characters and
/// the PNG's size in KB, rounded up; `None` with no logo.
fn short_logo_line(ruleset: &Ruleset) -> Option<String> {
    let logo = ruleset.logo.as_deref()?;
    let kb = ruleset.logo_png()?.len().div_ceil(1024);
    let start: String = logo.chars().take(24).collect();
    Some(format!(
        "{LOGO_KEY}\"{start}…\",  // a {kb} KB PNG, shortened in this view"
    ))
}
