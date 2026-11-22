//! The strip-style settings that interact with FPC, as findings.

use crate::player::SkinColor;

/// The strip-style settings that interact with FPC, as booleans the savefile
/// crate derives from its fields (`FPC.wikitext` lines 50–57).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StripStyle {
    /// Inners set to anything other than None (line 55).
    pub inners_worn: bool,
    /// Undershorts set to anything other than Off/Off (line 56).
    pub undershorts_worn: bool,
    /// Wrist taping set to anything other than None (line 53).
    pub wrist_taping: bool,
    /// Ankle taping set to anything other than None (line 53).
    pub ankle_taping: bool,
    /// The "Gloves?" checkbox (line 54).
    pub gloves_checkbox: bool,
    /// The gloves ID (line 54).
    pub gloves_id: u32,
    /// Skin color preset/custom (line 57).
    pub skin_color: SkinColor,
}

/// How bad a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Deliberate in custom setups; worth mentioning.
    Info,
    /// Valid but surprising.
    Warning,
    /// Breaks the feature.
    Error,
}

/// One troubleshooting rule that fired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Finding {
    /// The stable finding code.
    pub code: &'static str,
    /// Its severity.
    pub severity: Severity,
}

/// The troubleshooting list as findings, in a fixed order: inners,
/// undershorts, taping, gloves checkbox, skin color (FPC.wikitext
/// lines 53–57). `is_fpc_player` marks a player on a hide or partial-hide
/// preset.
pub fn check(style: &StripStyle, is_fpc_player: bool) -> Vec<Finding> {
    let mut findings = Vec::new();
    if style.inners_worn && is_fpc_player {
        findings.push(Finding {
            code: "fpc_inners_break_hiding",
            severity: Severity::Error,
        });
    }
    if style.undershorts_worn && is_fpc_player {
        findings.push(Finding {
            code: "fpc_undershorts_break_hiding",
            severity: Severity::Error,
        });
    }
    if (style.wrist_taping || style.ankle_taping) && is_fpc_player {
        findings.push(Finding {
            code: "fpc_taping_shows_pieces",
            severity: Severity::Info,
        });
    }
    if style.gloves_checkbox && style.gloves_id == 0 {
        findings.push(Finding {
            code: "fpc_gloves_checkbox_winter_gloves",
            severity: Severity::Warning,
        });
    }
    if style.skin_color == SkinColor::Custom && !is_fpc_player {
        findings.push(Finding {
            code: "fpc_custom_skin_on_non_fpc",
            severity: Severity::Warning,
        });
    }
    findings
}
