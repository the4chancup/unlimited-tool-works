//! Live AATF's marks on the editor itself, so a manager sees what to fix
//! where they fix it (the draft's `rowMark`, `violationField` and
//! `fieldMark`). While the toggle is on and the dock's check holds one
//! team's result, the list's rows carry a `✖` or `✔` and the selected
//! player's fields a row is about carry a `✖`, `⚠` or `ⓘ` left of their
//! labels. `held_violations` is the one read of the held check every mark
//! goes through.

pub(super) use team_widgets::violations::{CROSS, MARK_GAP};
use team_widgets::violations::{Wording, violation_line};

use crate::view::aatf_check::TeamsCheck;

/// The passing glyph (`✔`, U+2714), the `✖`'s pair (`violations::CROSS`):
/// egui's bundled emoji fonts hold them, and the theme keeps those fonts
/// in the fallback chain. The icon font's subset holds neither.
const TICK: char = '\u{2714}';

/// A field's mark for a Warning, the warning sign (`⚠`, U+26A0).
const WARNING_SIGN: char = '\u{26a0}';

/// A field's mark for suggestions alone, the circled i (`ⓘ`, U+24D8): a
/// plain `i` was too slight to see.
const CIRCLED_I: char = '\u{24d8}';

/// A field mark's box width: the widest of its glyphs' (the warning sign,
/// 10.6 at the body size), each glyph centred in it, so a column's marks
/// line up by their centres whatever their glyphs.
const MARK_BOX: f32 = 11.0;

/// The violations the marks draw: `check`'s held result while `live`
/// (the `live_aatf` toggle) is on, when the result is one team's
/// successful check — the team id and its violations, `None` otherwise.
/// While a check is in flight the held result stands, as the dock's list
/// keeps it. A failed check marks nothing.
pub(super) fn held_violations(live: bool, check: &TeamsCheck) -> Option<(u32, &[aatf::Violation])> {
    if !live {
        return None;
    }
    let (key, checks) = check.result.as_ref()?;
    let [team_id] = key.ids.as_slice() else {
        return None;
    };
    let [Ok(violations)] = checks.as_slice() else {
        return None;
    };
    Some((*team_id, violations))
}

/// The field a violation's mark stands before: `marked_field` maps a kind
/// to one, `None` for the kinds about no single field and the team's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum MarkedField {
    /// The Height box (`Height`).
    Height,
    /// The Age box (`Age`).
    Age,
    /// The Weight box (`Weight`).
    Weight,
    /// The Form box (`Form`).
    Form,
    /// The Injury Resistance box (`InjuryResistance`).
    InjuryResistance,
    /// The Weak Foot Usage box (`WeakFootUsage`).
    WeakFootUsage,
    /// The Weak Foot Accuracy box (`WeakFootAccuracy`).
    WeakFootAccuracy,
    /// The ability stat's box (`Stat`).
    Stat(aatf::Stat),
    /// The Name field (`NameColor`).
    Name,
    /// The Registered position combo (`RegisteredNotA`, `PositionRange`).
    RegisteredPosition,
    /// The Playable positions grid (`BRating`, `GkSecondA`, `APositions`).
    PlayablePositions,
    /// The player's spot on the named preset's Tactics pitch
    /// (`FieldedNotA`; the pitch draws it through the roster row's
    /// `fielded` marks, `tactics::show`'s fill of `held_violations`).
    Pitch,
    /// The Playing style combo (`PlayingStyleRange`).
    PlayingStyle,
    /// The Skills header: the skills (`SkillCardCap`, a required skill
    /// card) and the card budget its Cards left badge counts (`Cards`,
    /// `MinCards`), in one mark, since a mark of its own before the badge
    /// would shift the badge whenever it showed.
    Skills,
    /// The COM playing styles (`ComStyleCap`, a required COM style).
    ComStyles,
    /// The Tricks or Free cards panel's header (a suggestion of its
    /// cards).
    AsideCards,
}

/// The field `kind`'s row is about, one arm per kind so a new kind does
/// not compile past it. `None` for the kinds about no single field
/// (`NoTier`, `StatTotal`, `Special`, `Custom`, `RosterOrder`,
/// `UnusedChoice`), which mark the player's row alone, and the team's own
/// kinds, which mark the team's.
pub(super) fn marked_field(kind: &aatf::ViolationKind) -> Option<MarkedField> {
    use aatf::ViolationKind as K;
    match kind {
        K::Height { .. } => Some(MarkedField::Height),
        K::Age { .. } => Some(MarkedField::Age),
        K::Weight { .. } => Some(MarkedField::Weight),
        K::Form { .. } => Some(MarkedField::Form),
        K::InjuryResistance { .. } => Some(MarkedField::InjuryResistance),
        K::WeakFootUsage { .. } => Some(MarkedField::WeakFootUsage),
        K::WeakFootAccuracy { .. } => Some(MarkedField::WeakFootAccuracy),
        K::Stat { stat, .. } => Some(MarkedField::Stat(*stat)),
        K::NameColor { .. } => Some(MarkedField::Name),
        K::RegisteredNotA { .. } | K::PositionRange => Some(MarkedField::RegisteredPosition),
        K::BRating { .. } | K::GkSecondA { .. } | K::APositions { .. } => {
            Some(MarkedField::PlayablePositions)
        }
        K::FieldedNotA { .. } => Some(MarkedField::Pitch),
        K::PlayingStyleRange => Some(MarkedField::PlayingStyle),
        K::SkillCardCap { .. } | K::Cards { .. } | K::MinCards { .. } => Some(MarkedField::Skills),
        K::ComStyleCap { .. } => Some(MarkedField::ComStyles),
        K::RequiredCard { card } => Some(match card {
            aatf::Card::Skill(_) => MarkedField::Skills,
            aatf::Card::ComStyle(_) => MarkedField::ComStyles,
        }),
        K::Unused { allowance, .. } => Some(unused_field(allowance)),
        K::NoTier { .. }
        | K::StatTotal { .. }
        | K::Special { .. }
        | K::Custom { .. }
        | K::RosterOrder { .. }
        | K::UnusedChoice { .. }
        | K::TierCount { .. }
        | K::BracketQuota { .. }
        | K::MedalSystem { .. }
        | K::ExtraHeight { .. }
        | K::Captain
        | K::Goalkeepers { .. }
        | K::TeamSetting { .. }
        | K::FormationShape { .. } => None,
    }
}

/// The field an `Unused` suggestion's allowance is about: a `suggestions`
/// word's own field, a card group's panel (`com` the COM styles, `tricks`
/// the Tricks panel), the skills for any other group.
fn unused_field(allowance: &str) -> MarkedField {
    match allowance {
        "cards" => MarkedField::Skills,
        "a_positions" => MarkedField::PlayablePositions,
        "weak_foot_usage" => MarkedField::WeakFootUsage,
        "weak_foot_accuracy" => MarkedField::WeakFootAccuracy,
        "injury_resistance" => MarkedField::InjuryResistance,
        "height" => MarkedField::Height,
        "com" => MarkedField::ComStyles,
        "free_cards" | "tricks" => MarkedField::AsideCards,
        _ => MarkedField::Skills,
    }
}

/// The highest of `severities`: an Error over a Warning over a
/// Suggestion; `None` for none.
fn highest(severities: impl IntoIterator<Item = aatf::Severity>) -> Option<aatf::Severity> {
    severities
        .into_iter()
        .max_by_key(|severity| match severity {
            aatf::Severity::Error => 2,
            aatf::Severity::Warning => 1,
            aatf::Severity::Suggestion => 0,
        })
}

/// A severity's colour, the list's and the fields' marks alike: the error
/// red, the warning orange, the link blue for a suggestion.
fn severity_color(severity: aatf::Severity, visuals: &egui::Visuals) -> egui::Color32 {
    match severity {
        aatf::Severity::Error => visuals.error_fg_color,
        aatf::Severity::Warning => visuals.warn_fg_color,
        aatf::Severity::Suggestion => visuals.hyperlink_color,
    }
}

/// What a field's mark draws from: the held team's violations, the
/// selected player's slot and team (`violation_line`'s team id) and the
/// wording its lines take (the active ruleset's tiers and brackets, the
/// open save's stat names), bundled so the view's field sites take one
/// parameter. `None` at a site while
/// `held_violations` names another team or no result — the view's
/// players are not checked then.
#[derive(Clone, Copy)]
pub(super) struct Marks<'a> {
    /// The held team's violations, in the check's order.
    pub(super) violations: &'a [aatf::Violation],
    /// The selected player's roster slot.
    pub(super) slot: u8,
    /// His team.
    pub(super) team_id: u32,
    /// The active ruleset and the open save's version, for `violation_line`.
    pub(super) wording: Wording<'a>,
}

impl Marks<'_> {
    /// The mark of `field`'s rows on the selected player, painted before
    /// `label` (the field's label or its panel's header, its text drawn
    /// from the rect's left edge): a `MARK_BOX`-wide box ending
    /// `MARK_GAP` before the text, its glyph centred in it and on the
    /// label's height, its tooltip the rows' lines. Painted over the
    /// margin before the label, so no label moves or wraps and a column
    /// of labels has its marks on one centre whatever the labels' lengths.
    /// Draws nothing, and returns `None`, when no row of the slot is about
    /// `field`.
    pub(super) fn paint(
        &self,
        ui: &mut egui::Ui,
        label: egui::Rect,
        field: MarkedField,
    ) -> Option<egui::Response> {
        let mark = field_mark(self, field)?;
        let (glyph, color) = mark.glyph(ui.visuals());
        let galley = ui.painter().layout_no_wrap(
            glyph.to_string(),
            egui::TextStyle::Body.resolve(ui.style()),
            color,
        );
        let mark_box = egui::Rect::from_center_size(
            egui::pos2(label.min.x - MARK_GAP - MARK_BOX / 2.0, label.center().y),
            egui::vec2(MARK_BOX, galley.size().y),
        );
        let pos = mark_box.center() - galley.size() / 2.0;
        ui.painter().galley(pos, galley, color);
        Some(
            ui.interact(
                mark_box,
                ui.scope_id().with(("live aatf field mark", field)),
                egui::Sense::hover(),
            )
            .on_hover_text(mark.tooltip),
        )
    }
}

/// What a field's mark draws (from `field_mark`): the highest severity
/// among its rows and the tooltip.
struct FieldMark {
    /// The highest severity among the rows.
    severity: aatf::Severity,
    /// The rows' `violation_line`s, one per line, in the check's order.
    tooltip: String,
}

impl FieldMark {
    /// The glyph and its colour: a `✖` for an Error, a `⚠` for a
    /// Warning, a `ⓘ` for suggestions alone. Not the list's check mark:
    /// on a field it would read as "this field passes".
    fn glyph(&self, visuals: &egui::Visuals) -> (char, egui::Color32) {
        let glyph = match self.severity {
            aatf::Severity::Error => CROSS,
            aatf::Severity::Warning => WARNING_SIGN,
            aatf::Severity::Suggestion => CIRCLED_I,
        };
        (glyph, severity_color(self.severity, visuals))
    }
}

/// The rows on `marks`' slot whose kind maps to `field`, as the mark
/// before it draws them (`None`: no row, nothing is drawn).
fn field_mark(marks: &Marks<'_>, field: MarkedField) -> Option<FieldMark> {
    let marked: Vec<&aatf::Violation> = marks
        .violations
        .iter()
        .filter(|violation| {
            violation.slot == Some(marks.slot) && marked_field(&violation.kind) == Some(field)
        })
        .collect();
    Some(FieldMark {
        severity: highest(marked.iter().map(|violation| violation.severity))?,
        tooltip: marked
            .iter()
            .map(|violation| violation_line(violation, marks.team_id, marks.wording))
            .collect::<Vec<_>>()
            .join("\n"),
    })
}

/// What `row_mark` computes for a list row: the highest severity among
/// its rows and the tooltip's counts.
pub(super) struct RowMark {
    /// The highest severity among the row's violations; `None` when it
    /// has none.
    severity: Option<aatf::Severity>,
    /// `N error(s), M warning(s), K suggestion(s)` — a zero count left
    /// out, `No errors` when there are none.
    pub(super) tooltip: String,
}

impl RowMark {
    /// The glyph and its colour: `✖` in `error_fg_color` on an Error,
    /// else `✔` (warnings and suggestions are not failures, as
    /// `check-aatf`'s exit code counts them) in the highest severity's
    /// colour, or in the theme's success green for text (`notice_color`
    /// keeps it readable on both themes) when there is none.
    pub(super) fn glyph(&self, visuals: &egui::Visuals) -> (char, egui::Color32) {
        match self.severity {
            Some(aatf::Severity::Error) => (CROSS, visuals.error_fg_color),
            Some(severity @ (aatf::Severity::Warning | aatf::Severity::Suggestion)) => {
                (TICK, severity_color(severity, visuals))
            }
            None => (
                TICK,
                studio_core::theme::notice_color(studio_core::NoticeKind::Success, visuals),
            ),
        }
    }

    /// The glyph's galley width at the body font: a row leaves it room
    /// at its right end before truncating its text.
    pub(super) fn width(&self, ui: &egui::Ui) -> f32 {
        let (glyph, _) = self.glyph(ui.visuals());
        ui.painter()
            .layout_no_wrap(
                glyph.to_string(),
                egui::TextStyle::Body.resolve(ui.style()),
                egui::Color32::PLACEHOLDER,
            )
            .size()
            .x
    }
}

/// A list row's mark over the violations on `slot` (`None`: the team's
/// own).
pub(super) fn row_mark(violations: &[aatf::Violation], slot: Option<u8>) -> RowMark {
    let on_slot: Vec<aatf::Severity> = violations
        .iter()
        .filter(|violation| violation.slot == slot)
        .map(|violation| violation.severity)
        .collect();
    let mut parts = Vec::new();
    for (severity, word) in [
        (aatf::Severity::Error, "error"),
        (aatf::Severity::Warning, "warning"),
        (aatf::Severity::Suggestion, "suggestion"),
    ] {
        let count = on_slot.iter().filter(|held| **held == severity).count();
        if count > 0 {
            parts.push(format!(
                "{count} {word}{}",
                if count == 1 { "" } else { "s" }
            ));
        }
    }
    RowMark {
        severity: highest(on_slot),
        tooltip: if parts.is_empty() {
            "No errors".to_owned()
        } else {
            parts.join(", ")
        },
    }
}

/// `mark` painted at `rect`'s right end vertically centred; the glyph's
/// own hover rect shows the tooltip (the row's own hover, the name,
/// stays on the rest of the row — and a click on the glyph still selects
/// the row, the mark sensing hover only). `key` keeps the interact id
/// unique per row.
pub(super) fn paint_row_mark(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    mark: &RowMark,
    key: impl std::hash::Hash + std::fmt::Debug,
) -> egui::Response {
    let (glyph, color) = mark.glyph(ui.visuals());
    let galley = ui.painter().layout_no_wrap(
        glyph.to_string(),
        egui::TextStyle::Body.resolve(ui.style()),
        color,
    );
    let pos = egui::pos2(
        rect.max.x - galley.size().x,
        rect.center().y - galley.size().y / 2.0,
    );
    let glyph_rect = egui::Rect::from_min_size(pos, galley.size());
    ui.painter().galley(pos, galley, color);
    ui.interact(
        glyph_rect,
        ui.scope_id().with(("live aatf row mark", key)),
        egui::Sense::hover(),
    )
    .on_hover_text(mark.tooltip.as_str())
}
