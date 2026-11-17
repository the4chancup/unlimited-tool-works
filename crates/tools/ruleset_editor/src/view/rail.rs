//! The section rail (the draft's left column; the help window's
//! `help-tree` is the `Panel` precedent): one entry per section in
//! `Section::RAIL`'s order, Custom checks only when the document has them,
//! a "⚠" on a section holding a `check()` error.

use aatf::RuleError;

use crate::view::RulesetView;
use crate::view::sections::Section;

/// The rail, in a left `egui::Panel`.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut RulesetView) {
    let Some(document) = view.document.as_ref() else {
        return;
    };
    // A Custom checks entry makes no sense without checks (a replaced
    // document may carry none where the old one did).
    if view.section == Section::CustomChecks && document.custom_checks.is_none() {
        view.section = Section::General;
    }
    let problems = document.ruleset.check();
    egui::ScrollArea::vertical()
        .id_salt("ruleset_rail")
        .auto_shrink(false)
        .show(ui, |ui| {
            // The draft's `.rail .sel` `padding: 3px 6px`: three points
            // of padding above and below each row set their tops 24
            // apart (the 18-point row and the shared six); egui's own
            // spacing would put them 21.
            ui.spacing_mut().item_spacing.y = 6.0;
            for &section in Section::RAIL {
                if section == Section::CustomChecks && document.custom_checks.is_none() {
                    continue;
                }
                let bad = problems.iter().any(|error| section_of(error) == section);
                // The draft's `.rail .sel`: the row fills the rail's
                // width, its mark at the right end. A row is not a control
                // (`row`), so the selected entry's blue draws no edge.
                let mut entry = egui::Button::selectable(view.section == section, section.label())
                    .min_size(egui::vec2(ui.available_width(), 0.0));
                if bad {
                    entry = entry
                        .right_text(egui::RichText::new("⚠").color(ui.visuals().error_fg_color));
                }
                if studio_core::widgets::row(ui, entry).clicked() {
                    view.section = section;
                }
            }
        });
}

/// The rail entry a `RuleError` lands in: its path's first segment names
/// the ruleset's field, the fields group into sections.
pub(crate) fn section_of(error: &RuleError) -> Section {
    let head = error.path.split(['.', '[']).next().unwrap_or_default();
    match head {
        "tiers" | "medal_systems" => Section::Tiers,
        "heights" => Section::Heights,
        "card_groups" => Section::Cards,
        "universal" | "team_settings" => Section::Universal,
        "specials" => Section::Specials,
        "suggestions" => Section::Suggestions,
        // `name`, `stats` and anything unmapped: the first
        // section is where an unmappable problem lands.
        _ => Section::General,
    }
}

#[cfg(test)]
mod tests {
    use aatf::{LoadRule, RuleError};

    use super::*;

    fn error(path: &str) -> RuleError {
        RuleError {
            path: path.to_string(),
            rule: LoadRule::Empty,
        }
    }

    /// Every first segment of a `check()` path lands on its section.
    #[test]
    fn a_problems_first_path_segment_names_its_section() {
        for (path, section) in [
            ("name", Section::General),
            ("stats.at_most[0]", Section::General),
            ("tiers", Section::Tiers),
            ("tiers[2].free.tricks", Section::Tiers),
            ("medal_systems[0].counts", Section::Tiers),
            ("heights.brackets[1].id", Section::Heights),
            ("card_groups", Section::Cards),
            ("universal.fielded_at_a[0]", Section::Universal),
            ("team_settings[1].label", Section::Universal),
            ("specials[4].when.tiers[1]", Section::Specials),
            ("suggestions[2]", Section::Suggestions),
        ] {
            assert_eq!(section_of(&error(path)), section, "{path}");
        }
    }
}
