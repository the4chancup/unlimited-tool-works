//! The data-block preview: the ruleset's data block (the logo's line
//! shortened) in monospace, read-only, the lines the last edit changed
//! highlighted (the draft's `refresh`).

use std::collections::BTreeSet;

use aatf::Ruleset;

use crate::view::RulesetView;

/// The start of the data block's logo line: the key at the block's
/// indent.
const LOGO_KEY: &str = "    logo: ";

/// The data block's columns (the draft's `100ch`): the writer's wrap
/// width.
const COLUMNS: usize = 100;

/// The font the block is drawn in: `TextStyle::Monospace` resolved from
/// the style (the theme's 12-point Hack). `block_line` draws with it and
/// `width` measures the columns with it, so the two cannot drift.
fn block_font(ui: &egui::Ui) -> egui::FontId {
    egui::TextStyle::Monospace.resolve(ui.style())
}

/// The separator line's width at the panel's resizable edge: the panel
/// reserves it inside its `exact_size` (the frame's outer margin grows by
/// it), so the data block's columns need it beside them too.
fn separator(ui: &egui::Ui) -> f32 {
    f32::from(ui.visuals().widgets.noninteractive.bg_stroke.width.round() as i8)
}

/// The preview panel's exact width (the draft's `pre` at `100ch` plus its
/// `scrollbar-gutter`): `COLUMNS` of `block_font`, measured through
/// egui's fonts rather than written down, beside the scroll bar's strip
/// and inside the panel frame's margins.
pub(crate) fn width(ui: &egui::Ui) -> f32 {
    let column = ui
        .ctx()
        .fonts_mut(|fonts| fonts.glyph_width(&block_font(ui), '0'));
    egui::Frame::side_top_panel(ui.style()).inner_margin.sum().x
        + COLUMNS as f32 * column
        + scroll_bars().allocated_width()
        + separator(ui)
}

/// The preview's scroll bars (the draft's `scrollbar-gutter: stable`): a
/// solid style keeps each bar's strip beside the content and
/// `AlwaysVisible` keeps it drawn when the block fits, so the text area
/// is the same width whether or not the block scrolls — a column's end
/// never sits under a drawn bar.
fn scroll_bars() -> egui::style::ScrollStyle {
    egui::style::ScrollStyle::solid()
}

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
    ui.spacing_mut().scroll = scroll_bars();
    egui::ScrollArea::both()
        .id_salt("ruleset_preview")
        .auto_shrink(false)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
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
    let text = egui::RichText::new(shown).font(block_font(ui));
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
