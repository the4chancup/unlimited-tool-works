//! The rail's section forms: General and Tiers are forms, Custom checks
//! and Summary read only,
//! and Heights, Cards, Universal rules, Specials and Suggestions have no
//! form yet: each draws one weak line sending the member to Summary.

pub(crate) mod custom_checks;
pub(crate) mod general;
pub(crate) mod summary;
pub(crate) mod tiers;
pub(crate) mod widgets;

use aatf::Ruleset;

/// A rail section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    /// Name, event page and logo, the stat lists (`general`).
    General,
    /// The tiers table and the medal systems.
    Tiers,
    /// The height brackets and systems.
    Heights,
    /// The card groups.
    Cards,
    /// The universal rules and the team settings.
    Universal,
    /// The conditional special rules.
    Specials,
    /// The allowance suggestions.
    Suggestions,
    /// The file's hand-written checks, read-only (`custom_checks`);
    /// offered only when the document has them.
    CustomChecks,
    /// The ruleset as readable rules text (`summary`).
    Summary,
}

impl Section {
    /// The rail's order (RE-GUI-02); Custom checks joins only when the
    /// document has them.
    pub(crate) const RAIL: &[Section] = &[
        Section::General,
        Section::Tiers,
        Section::Heights,
        Section::Cards,
        Section::Universal,
        Section::Specials,
        Section::Suggestions,
        Section::CustomChecks,
        Section::Summary,
    ];

    /// The rail's label.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Section::General => "General",
            Section::Tiers => "Tiers",
            Section::Heights => "Heights",
            Section::Cards => "Cards",
            Section::Universal => "Universal rules",
            Section::Specials => "Specials",
            Section::Suggestions => "Suggestions",
            Section::CustomChecks => "Custom checks",
            Section::Summary => "Summary",
        }
    }
}

/// The drawn section over `ruleset` (the forms edit it in place;
/// `custom_checks` is the section's read-only text, `logo` General's
/// Event logo field); the sections with no form draw one weak line.
pub(crate) fn show(
    ui: &mut egui::Ui,
    section: Section,
    ruleset: &mut Ruleset,
    custom_checks: Option<&str>,
    logo: &mut general::LogoField,
) {
    match section {
        Section::General => general::show(ui, ruleset, logo),
        Section::Tiers => tiers::show(ui, ruleset),
        Section::CustomChecks => custom_checks::show(ui, custom_checks),
        Section::Summary => summary::show(ui, ruleset),
        Section::Heights
        | Section::Cards
        | Section::Universal
        | Section::Specials
        | Section::Suggestions => {
            ui.heading(section.label());
            ui.weak(
                "You can't edit this section yet. Summary shows what's in it, and saving leaves it unchanged.",
            );
        }
    }
}
