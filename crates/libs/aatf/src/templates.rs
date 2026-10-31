//! The embedded rulesets: every cup's current ones, the latest two of the
//! 4cc, VGL and VTL, Autumn 26 first, then DBC7. The Ruleset editor's
//! start page offers them as templates; the Save editor's ruleset
//! dropdown lists them and loads the one picked (`builtin`).

use crate::file::file_sections;
use crate::rules::Rules;
use crate::ruleset::Ruleset;

/// One embedded ruleset offered on the Ruleset editor's start page.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    /// The embedded file's stem (`autumn26`): the name the Save editor's
    /// `aatf_ruleset` setting holds for it.
    pub stem: &'static str,
    /// The template's ruleset.
    pub ruleset: Ruleset,
    /// Its CUSTOM CHECKS section as the embedded file carries it, `None`
    /// when it has none (VTL11 and VTL10 alone carry checks).
    pub custom_checks: Option<String>,
}

/// The embedded blocks under their file stems, in the start page's order
/// (Autumn 26 is `rules.rs`' embedded official block).
const BLOCKS: &[(&str, &str)] = &[
    ("autumn26", crate::rules::AUTUMN26),
    ("spring26", include_str!("../rulesets/spring26.rhai")),
    ("vgl27", include_str!("../rulesets/vgl27.rhai")),
    ("vgl26", include_str!("../rulesets/vgl26.rhai")),
    ("vtl11", include_str!("../rulesets/vtl11.rhai")),
    ("vtl10", include_str!("../rulesets/vtl10.rhai")),
    ("dbc7", include_str!("../rulesets/dbc7.rhai")),
];

/// The start page's templates, in its order: Autumn 26, Spring 26, VGL27,
/// VGL26, VTL11, VTL10, DBC7.
///
/// Panics only if an embedded file does not load, which a test rules out.
pub fn templates() -> Vec<Template> {
    BLOCKS
        .iter()
        .map(|&(stem, block)| Template {
            stem,
            ruleset: Ruleset::from_data_block(block).expect("an embedded ruleset loads"),
            custom_checks: file_sections(block).custom_checks,
        })
        .collect()
}

/// The embedded ruleset whose file stem is `stem` (`vtl11`), assembled
/// with the current logic and loaded; `None` for a stem no embedded
/// ruleset has.
///
/// Panics only if an embedded file does not load, which a test rules out.
pub fn builtin(stem: &str) -> Option<Rules> {
    let (_, block) = BLOCKS.iter().find(|(name, _)| *name == stem)?;
    Some(Rules::from_data_block(block).expect("an embedded ruleset loads"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All seven templates load, in the start page's order; only the VTL
    /// ones carry custom checks.
    #[test]
    fn the_templates_load_in_the_start_pages_order() {
        let templates = templates();
        let names: Vec<&str> = templates
            .iter()
            .map(|template| template.ruleset.name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "4cc Autumn 26",
                "4cc Spring 26",
                "/vg/ League 27",
                "/vg/ League 26",
                "/vt/ League 11",
                "/vt/ League 10",
                "Divegrass Before Christmas 7"
            ]
        );
        let checks: Vec<bool> = templates
            .iter()
            .map(|template| template.custom_checks.is_some())
            .collect();
        assert_eq!(checks, [false, false, false, false, true, true, false]);
    }

    /// Every template's stem loads as a built-in ruleset carrying the
    /// template's name, so the dropdown's row and the ruleset it loads
    /// agree; a stem no embedded file has is `None`.
    #[test]
    fn every_templates_stem_loads_its_builtin_ruleset() {
        let stems: Vec<&str> = templates().iter().map(|template| template.stem).collect();
        assert_eq!(
            stems,
            [
                "autumn26", "spring26", "vgl27", "vgl26", "vtl11", "vtl10", "dbc7"
            ]
        );
        for template in templates() {
            let rules =
                builtin(template.stem).unwrap_or_else(|| panic!("{} is built in", template.stem));
            assert_eq!(
                rules.ruleset().name,
                template.ruleset.name,
                "{}",
                template.stem
            );
        }
        assert!(
            builtin("autumn25").is_none(),
            "a test ruleset is not built in"
        );
        assert!(builtin("cup.4ccrules").is_none());
    }

    /// Autumn 26 and DBC7, whose cups play a game that keeps any
    /// formation, allow illegal formations; every other built-in keeps
    /// the game's rule.
    #[test]
    fn only_autumn26_and_dbc7_allow_illegal_formations() {
        for &(stem, _) in BLOCKS {
            let rules = builtin(stem).unwrap_or_else(|| panic!("{stem} is built in"));
            assert_eq!(
                rules.ruleset().universal.illegal_formations,
                matches!(stem, "autumn26" | "dbc7"),
                "{stem}"
            );
        }
    }
}
