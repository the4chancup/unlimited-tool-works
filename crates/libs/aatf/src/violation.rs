//! `Violation`: what `check_team` reports — a finding a team's player
//! or the team itself breaks, converted from the script's list of maps.

use rhai::serde::from_dynamic;
use rhai::{Dynamic, Map};
use serde::Deserialize;

use crate::ruleset::{BracketId, Card, Position, Severity, Stat, SystemId, TierId};

/// One finding: a rule a player or the team breaks (`slot` `None` for
/// the team).
#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    /// The roster slot the violation is on; `None`: the team.
    pub slot: Option<u8>,
    /// Error, warning or suggestion.
    pub severity: Severity,
    /// Which rule.
    pub kind: ViolationKind,
}

/// Which rule a violation reports.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ViolationKind {
    /// The registered position is not rated A (under
    /// `registered_position_a`).
    RegisteredNotA {
        /// Every position the player is rated A at.
        rated_a: Vec<Position>,
    },
    /// Fielded at `position` in preset `preset`, which it is not rated A
    /// at (under `fielded_at_a`).
    FieldedNotA {
        /// The preset (1 to 3).
        preset: u8,
        /// The position.
        position: Position,
        /// Every position the player is rated A at.
        rated_a: Vec<Position>,
    },
    /// A B rating where `no_b_ratings` refuses one.
    BRating {
        /// Every position the player is rated B at.
        rated_b: Vec<Position>,
    },
    /// A second A rating that is GK's (under `gk_not_second_a`).
    GkSecondA {
        /// The positions other than GK the player is rated A at.
        rated_a: Vec<Position>,
    },
    /// Age outside `universal.age`.
    Age {
        /// The player's age.
        age: u8,
        /// The minimum.
        min: u8,
        /// The maximum.
        max: u8,
    },
    /// Weight outside the version's weight rule.
    Weight {
        /// The player's weight.
        weight: u8,
        /// The bound's minimum.
        min: u8,
        /// The bound's maximum.
        max: u8,
    },
    /// The registered-position byte is past the version's position
    /// labels.
    PositionRange,
    /// A playing style outside its allowed set.
    PlayingStyleRange,
    /// No tier's `when` held (the player gets no tier and no further
    /// per-player check).
    NoTier {
        /// The heights the tiers' `when`s name (`height_min`/`height_max`, an `any` list's entries
        /// included), each an inclusive range; empty when the player's height is among them, the
        /// cause then being another condition.
        heights: Vec<(u8, u8)>,
    },
    /// An ability stat off its target (over it, for an `at_most`
    /// target).
    Stat {
        /// The stat.
        stat: Stat,
        /// The player's value.
        value: u8,
        /// The tier's or the effect's target.
        target: u8,
        /// The target is an `at_most` entry: a value under it is legal,
        /// one over it is the violation.
        at_most: bool,
    },
    /// The `at_most` ability stats' sum over the sum of their targets (under
    /// a tier's `total_within_targets`).
    StatTotal {
        /// The player's stats added up.
        total: u16,
        /// The targets' sum.
        limit: u16,
    },
    /// Form off the tier's target (over it, for an `at_most` target).
    Form {
        /// The player's form.
        value: u8,
        /// The target.
        target: u8,
    },
    /// Injury resistance off its target (over it, for an `at_most`
    /// target).
    InjuryResistance {
        /// The player's value.
        value: u8,
        /// The target.
        max: u8,
    },
    /// Weak foot usage off its target (over it, for an `at_most`
    /// target).
    WeakFootUsage {
        /// The player's value.
        value: u8,
        /// The target.
        max: u8,
    },
    /// Weak foot accuracy off its target (over it, for an `at_most`
    /// target).
    WeakFootAccuracy {
        /// The player's value.
        value: u8,
        /// The target.
        max: u8,
    },
    /// The name's leading colour code against the tier's; `expected: None`
    /// is a tier that sets none, met by a name with no colour code the
    /// tiers set.
    NameColor {
        /// The name's leading colour code (`None` for a name without one).
        color: Option<String>,
        /// The tier's code (`None` on a tier that sets none).
        expected: Option<String>,
    },
    /// Paid cards over the limit.
    Cards {
        /// The paid count.
        paid: u8,
        /// The limit; below zero, the extra A positions outnumber the
        /// cards.
        limit: i8,
    },
    /// Paid skill cards plus extra A positions under `min_cards`.
    MinCards {
        /// The paid count.
        paid: u8,
        /// The minimum.
        min: u8,
    },
    /// More skill cards than `max_skill_cards`.
    SkillCardCap {
        /// The count.
        count: u8,
        /// The cap.
        max: u8,
    },
    /// More COM styles than `max_com_styles`.
    ComStyleCap {
        /// The count.
        count: u8,
        /// The cap.
        max: u8,
    },
    /// A `require_cards` card not held.
    RequiredCard {
        /// The card.
        card: Card,
    },
    /// More A-rated positions than the allowance.
    APositions {
        /// The count.
        count: u8,
        /// The allowance.
        max: u8,
        /// Every position the player is rated A at.
        rated_a: Vec<Position>,
    },
    /// A height no bracket of the team's system takes.
    Height {
        /// The player's height in cm.
        height: u8,
        /// The heights the player could take, each an inclusive range, his height allowance
        /// added: the ranges of the team's height system's brackets whose quota is not met
        /// yet; when none has room, of every bracket whose quota is above 0; under an
        /// `extra_height` system, one range from the lowest quota bracket to `max`.
        fits: Vec<(u8, u8)>,
        /// Whether `fits` are brackets with room (false when none has, and under a pool).
        room: bool,
    },
    /// A tier's count off its expected one (`counts_as` folded in).
    TierCount {
        /// The tier.
        tier: TierId,
        /// The players in it.
        count: u8,
        /// Its `count` or the rest of the squad.
        expected: u8,
    },
    /// A bracket's quota off its system's.
    BracketQuota {
        /// The system.
        system: SystemId,
        /// The bracket.
        bracket: BracketId,
        /// The players in it.
        count: u8,
        /// The quota.
        quota: u8,
    },
    /// The team's counts of the medal tiers equal no `medal_systems` entry's.
    MedalSystem {
        /// Every system's label, in the ruleset's order: the counts the team may have.
        systems: Vec<String>,
    },
    /// The pool of centimetres an `extra_height` system gives the team, overdrawn.
    ExtraHeight {
        /// The centimetres drawn.
        used: u16,
        /// The pool's size.
        total: u16,
    },
    /// No captain where `captain_required`.
    Captain,
    /// Fewer goalkeepers than `min_goalkeepers`.
    Goalkeepers {
        /// The count.
        count: u8,
        /// The minimum.
        min: u8,
    },
    /// A team settings entry whose value does not hold on this save.
    TeamSetting {
        /// The entry's label.
        label: String,
        /// Which setting fails (`man_marking`, an auto flag).
        setting: String,
        /// The value the entry sets for the setting.
        wanted: bool,
    },
    /// A formation matching a `forbidden_shapes` shape.
    FormationShape {
        /// The preset (1 to 3).
        preset: u8,
        /// The formation within it (1 to 3).
        formation: u8,
        /// `[defenders, midfielders, forwards]`.
        shape: [u8; 3],
    },
    /// A special's `forbid` fired: `text` is the ruleset author's.
    Special {
        /// The special's label.
        label: String,
        /// Its `forbid` text.
        text: String,
    },
    /// A `custom_checks` violation: `text` is the check author's.
    Custom {
        /// The violation text.
        text: String,
    },
    /// A suggestion: an allowance listed in `suggestions` the player does
    /// not use up.
    Unused {
        /// Which allowance: a `suggestions` word or group, or
        /// `weak_foot_usage` / `weak_foot_accuracy`, the `weak_foot` word's
        /// two halves, each reported on its own.
        allowance: String,
        /// How much is used.
        used: u8,
        /// How much is allowed.
        allowed: u8,
    },
    /// A suggestion under `choices`: a special's choice the player takes
    /// no option of.
    UnusedChoice {
        /// The special's label.
        special: String,
    },
    /// The roster slot's player is not `team id x 100 + slot + 1` (4cc's
    /// roster order, `TeamEntry::roster_order_breaks`): the aesthetics
    /// export names players by id, a Team TOML by slot, and this rule is
    /// their only link. Raised by the host before the script runs; the
    /// script does not see it, and the shipped logic emits none.
    RosterOrder {
        /// The player id the slot holds.
        player: u32,
    },
}

impl ViolationKind {
    /// The kind as a rules file writes it (`FieldedNotA`) and its fields by their field
    /// names, in declaration order, each value spelled as the file spells it.
    /// One arm per kind, so a new one does not compile past this match.
    pub fn rule(&self) -> (&'static str, Vec<(&'static str, String)>) {
        match self {
            ViolationKind::RegisteredNotA { rated_a } => (
                "RegisteredNotA",
                vec![("rated_a", positions_field(rated_a))],
            ),
            ViolationKind::FieldedNotA {
                preset,
                position,
                rated_a,
            } => (
                "FieldedNotA",
                vec![
                    ("preset", preset.to_string()),
                    ("position", position.label().to_owned()),
                    ("rated_a", positions_field(rated_a)),
                ],
            ),
            ViolationKind::BRating { rated_b } => {
                ("BRating", vec![("rated_b", positions_field(rated_b))])
            }
            ViolationKind::GkSecondA { rated_a } => {
                ("GkSecondA", vec![("rated_a", positions_field(rated_a))])
            }
            ViolationKind::Age { age, min, max } => (
                "Age",
                vec![
                    ("age", age.to_string()),
                    ("min", min.to_string()),
                    ("max", max.to_string()),
                ],
            ),
            ViolationKind::Weight { weight, min, max } => (
                "Weight",
                vec![
                    ("weight", weight.to_string()),
                    ("min", min.to_string()),
                    ("max", max.to_string()),
                ],
            ),
            ViolationKind::PositionRange => ("PositionRange", vec![]),
            ViolationKind::PlayingStyleRange => ("PlayingStyleRange", vec![]),
            ViolationKind::NoTier { heights } => {
                ("NoTier", vec![("heights", ranges_field(heights))])
            }
            ViolationKind::Stat {
                stat,
                value,
                target,
                at_most,
            } => (
                "Stat",
                vec![
                    ("stat", stat.label().to_owned()),
                    ("value", value.to_string()),
                    ("target", target.to_string()),
                    ("at_most", at_most.to_string()),
                ],
            ),
            ViolationKind::StatTotal { total, limit } => (
                "StatTotal",
                vec![("total", total.to_string()), ("limit", limit.to_string())],
            ),
            ViolationKind::Form { value, target } => (
                "Form",
                vec![("value", value.to_string()), ("target", target.to_string())],
            ),
            ViolationKind::InjuryResistance { value, max } => (
                "InjuryResistance",
                vec![("value", value.to_string()), ("max", max.to_string())],
            ),
            ViolationKind::WeakFootUsage { value, max } => (
                "WeakFootUsage",
                vec![("value", value.to_string()), ("max", max.to_string())],
            ),
            ViolationKind::WeakFootAccuracy { value, max } => (
                "WeakFootAccuracy",
                vec![("value", value.to_string()), ("max", max.to_string())],
            ),
            ViolationKind::NameColor { color, expected } => (
                "NameColor",
                [
                    color.as_ref().map(|color| ("color", color.clone())),
                    expected
                        .as_ref()
                        .map(|expected| ("expected", expected.clone())),
                ]
                .into_iter()
                .flatten()
                .collect(),
            ),
            ViolationKind::Cards { paid, limit } => (
                "Cards",
                vec![("paid", paid.to_string()), ("limit", limit.to_string())],
            ),
            ViolationKind::MinCards { paid, min } => (
                "MinCards",
                vec![("paid", paid.to_string()), ("min", min.to_string())],
            ),
            ViolationKind::SkillCardCap { count, max } => (
                "SkillCardCap",
                vec![("count", count.to_string()), ("max", max.to_string())],
            ),
            ViolationKind::ComStyleCap { count, max } => (
                "ComStyleCap",
                vec![("count", count.to_string()), ("max", max.to_string())],
            ),
            ViolationKind::RequiredCard { card } => {
                ("RequiredCard", vec![("card", card.label().to_owned())])
            }
            ViolationKind::APositions {
                count,
                max,
                rated_a,
            } => (
                "APositions",
                vec![
                    ("count", count.to_string()),
                    ("max", max.to_string()),
                    ("rated_a", positions_field(rated_a)),
                ],
            ),
            ViolationKind::Height { height, fits, room } => (
                "Height",
                vec![
                    ("height", height.to_string()),
                    ("fits", ranges_field(fits)),
                    ("room", room.to_string()),
                ],
            ),
            ViolationKind::TierCount {
                tier,
                count,
                expected,
            } => (
                "TierCount",
                vec![
                    ("tier", tier.clone()),
                    ("count", count.to_string()),
                    ("expected", expected.to_string()),
                ],
            ),
            ViolationKind::BracketQuota {
                system,
                bracket,
                count,
                quota,
            } => (
                "BracketQuota",
                vec![
                    ("system", system.clone()),
                    ("bracket", bracket.clone()),
                    ("count", count.to_string()),
                    ("quota", quota.to_string()),
                ],
            ),
            ViolationKind::MedalSystem { systems } => (
                "MedalSystem",
                vec![("systems", format!("[{}]", systems.join(", ")))],
            ),
            ViolationKind::ExtraHeight { used, total } => (
                "ExtraHeight",
                vec![("used", used.to_string()), ("total", total.to_string())],
            ),
            ViolationKind::Captain => ("Captain", vec![]),
            ViolationKind::Goalkeepers { count, min } => (
                "Goalkeepers",
                vec![("count", count.to_string()), ("min", min.to_string())],
            ),
            ViolationKind::TeamSetting {
                label,
                setting,
                wanted,
            } => (
                "TeamSetting",
                vec![
                    ("label", label.clone()),
                    ("setting", setting.clone()),
                    ("wanted", wanted.to_string()),
                ],
            ),
            ViolationKind::FormationShape {
                preset,
                formation,
                shape,
            } => (
                "FormationShape",
                vec![
                    ("preset", preset.to_string()),
                    ("formation", formation.to_string()),
                    ("shape", format!("{shape:?}")),
                ],
            ),
            ViolationKind::Special { label, text } => (
                "Special",
                vec![("label", label.clone()), ("text", text.clone())],
            ),
            ViolationKind::Custom { text } => ("Custom", vec![("text", text.clone())]),
            ViolationKind::Unused {
                allowance,
                used,
                allowed,
            } => (
                "Unused",
                vec![
                    ("allowance", allowance.clone()),
                    ("used", used.to_string()),
                    ("allowed", allowed.to_string()),
                ],
            ),
            ViolationKind::UnusedChoice { special } => {
                ("UnusedChoice", vec![("special", special.clone())])
            }
            ViolationKind::RosterOrder { player } => {
                ("RosterOrder", vec![("player", player.to_string())])
            }
        }
    }
}

/// A list of positions as `rule` prints it: `[CF, SS]`, `[]` when empty.
fn positions_field(positions: &[Position]) -> String {
    let labels: Vec<&str> = positions.iter().map(|position| position.label()).collect();
    format!("[{}]", labels.join(", "))
}

/// A list of inclusive height ranges as `rule` prints it: `[0-175, 180-180]`, `[]` when empty.
fn ranges_field(ranges: &[(u8, u8)]) -> String {
    let spans: Vec<String> = ranges
        .iter()
        .map(|(low, high)| format!("{low}-{high}"))
        .collect();
    format!("[{}]", spans.join(", "))
}

/// Why a check could not produce its violations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CheckError {
    /// The script failed: a runtime error or a limit reached; `message`
    /// is rhai's, with its position.
    #[error("the check's script failed: {message}")]
    Script {
        /// rhai's error text.
        message: String,
    },
    /// `check_team` returned something other than a list (`type_name`:
    /// rhai's name for it).
    #[error("check_team did not return a list (got {type_name})")]
    NotAList {
        /// rhai's name for the type.
        type_name: String,
    },
    /// Entry `index` of the returned list does not convert into a
    /// `Violation`.
    #[error("violation {index}: {message}")]
    Violation {
        /// The entry's index.
        index: usize,
        /// What did not convert.
        message: String,
    },
    /// A function's result does not convert (`function` names it): a
    /// `tier_values` or `card_limits` map that is not its values, or a
    /// `player_tier` answer that is neither `()` nor a tier's id.
    #[error("{function} did not return its values: {message}")]
    Values {
        /// The script function.
        function: &'static str,
        /// What did not convert.
        message: String,
    },
    /// No tier of the file has this id.
    #[error("unknown tier {tier}")]
    UnknownTier {
        /// The id asked for.
        tier: String,
    },
    /// `apply_tier`'s values still change after three writes: a special
    /// conditioned on a value the tier sets keeps switching with what it
    /// writes. The player is left as it was.
    #[error("tier {tier} writes values that never settle")]
    Unsettled {
        /// The tier applied.
        tier: String,
    },
}

/// The violations of `check_team`'s returned list, in its order: every
/// entry converts or the first bad one fails the whole check (a bug in
/// the logic or a custom check, never skipped).
pub(crate) fn violations(list: Dynamic) -> Result<Vec<Violation>, CheckError> {
    let entries = list
        .into_array()
        .map_err(|type_name| CheckError::NotAList {
            type_name: type_name.to_string(),
        })?;
    let mut out = Vec::with_capacity(entries.len());
    for (index, entry) in entries.into_iter().enumerate() {
        let mut map = entry
            .try_cast::<Map>()
            .ok_or_else(|| CheckError::Violation {
                index,
                message: "not a map".to_string(),
            })?;
        let slot = match map.remove("slot") {
            Some(d) if d.is_unit() => None,
            Some(d) => Some(
                u8::try_from(d.as_int().map_err(|e| CheckError::Violation {
                    index,
                    message: format!("slot is not a number: {e}"),
                })?)
                .map_err(|_| CheckError::Violation {
                    index,
                    message: format!("slot {d:?}: past u8"),
                })?,
            ),
            None => None,
        };
        let severity = match map.remove("severity") {
            None => Severity::Error,
            Some(d) => from_dynamic::<Severity>(&d).map_err(|e| CheckError::Violation {
                index,
                message: format!("severity: {e}"),
            })?,
        };
        let kind_dynamic = Dynamic::from_map(map.clone());
        let kind =
            from_dynamic::<ViolationKind>(&kind_dynamic).map_err(|e| CheckError::Violation {
                index,
                message: e.to_string(),
            })?;
        // `deny_unknown_fields` does not see a unit variant's extra key
        // (serde's internally-tagged read lets it through): rule it out
        // here — a unit variant takes `kind` alone. Every variant placed
        // on purpose: a new one lands in a match arm, not a `_`.
        match kind {
            ViolationKind::PositionRange
            | ViolationKind::PlayingStyleRange
            | ViolationKind::Captain => {
                if let Some(extra) = map.keys().find(|key| key.as_str() != "kind") {
                    return Err(CheckError::Violation {
                        index,
                        message: format!("`{extra}`: unknown key on a unit variant"),
                    });
                }
            }
            ViolationKind::RegisteredNotA { .. }
            | ViolationKind::FieldedNotA { .. }
            | ViolationKind::BRating { .. }
            | ViolationKind::GkSecondA { .. }
            | ViolationKind::Age { .. }
            | ViolationKind::Weight { .. }
            | ViolationKind::NoTier { .. }
            | ViolationKind::Stat { .. }
            | ViolationKind::StatTotal { .. }
            | ViolationKind::Form { .. }
            | ViolationKind::InjuryResistance { .. }
            | ViolationKind::WeakFootUsage { .. }
            | ViolationKind::WeakFootAccuracy { .. }
            | ViolationKind::NameColor { .. }
            | ViolationKind::Cards { .. }
            | ViolationKind::MinCards { .. }
            | ViolationKind::SkillCardCap { .. }
            | ViolationKind::ComStyleCap { .. }
            | ViolationKind::RequiredCard { .. }
            | ViolationKind::APositions { .. }
            | ViolationKind::Height { .. }
            | ViolationKind::TierCount { .. }
            | ViolationKind::BracketQuota { .. }
            | ViolationKind::MedalSystem { .. }
            | ViolationKind::ExtraHeight { .. }
            | ViolationKind::Goalkeepers { .. }
            | ViolationKind::TeamSetting { .. }
            | ViolationKind::FormationShape { .. }
            | ViolationKind::Special { .. }
            | ViolationKind::Custom { .. }
            | ViolationKind::Unused { .. }
            | ViolationKind::UnusedChoice { .. }
            | ViolationKind::RosterOrder { .. } => {}
        }
        out.push(Violation {
            slot,
            severity,
            kind,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::engine;
    use crate::ruleset::Severity;

    /// `text` evaluated as a `check_team`-shaped list of one map.
    fn one(text: &str) -> Dynamic {
        engine()
            .eval::<Dynamic>(&format!("[{text}]"))
            .expect("a list of one map")
    }

    /// `text` → the one `Violation` its map converts into.
    fn one_violation(text: &str) -> Violation {
        violations(one(text))
            .expect(text)
            .into_iter()
            .next()
            .expect("one")
    }

    /// Every `ViolationKind` variant converts; `slot` and `severity`
    /// vary across rows (`warning` and `suggestion` each once).
    #[test]
    fn every_kind_converts() {
        let cases: &[(&str, Option<u8>, Severity, ViolationKind)] = &[
            (
                "#{ kind: \"RegisteredNotA\", rated_a: [\"CF\", \"SS\"] }",
                None,
                Severity::Error,
                ViolationKind::RegisteredNotA {
                    rated_a: vec![Position(12), Position(11)],
                },
            ),
            (
                "#{ kind: \"FieldedNotA\", preset: 1, position: \"CB\", rated_a: [\"DMF\"], slot: 5, severity: \"warning\" }",
                Some(5),
                Severity::Warning,
                ViolationKind::FieldedNotA {
                    preset: 1,
                    position: Position(1),
                    rated_a: vec![Position(4)],
                },
            ),
            (
                "#{ kind: \"BRating\", rated_b: [\"LB\", \"RB\"] }",
                None,
                Severity::Error,
                ViolationKind::BRating {
                    rated_b: vec![Position(2), Position(3)],
                },
            ),
            (
                "#{ kind: \"GkSecondA\", rated_a: [] }",
                None,
                Severity::Error,
                ViolationKind::GkSecondA { rated_a: vec![] },
            ),
            (
                "#{ kind: \"Age\", age: 15, min: 16, max: 40 }",
                None,
                Severity::Error,
                ViolationKind::Age {
                    age: 15,
                    min: 16,
                    max: 40,
                },
            ),
            (
                "#{ kind: \"Weight\", weight: 80, min: 30, max: 50 }",
                None,
                Severity::Error,
                ViolationKind::Weight {
                    weight: 80,
                    min: 30,
                    max: 50,
                },
            ),
            (
                "#{ kind: \"PositionRange\" }",
                None,
                Severity::Error,
                ViolationKind::PositionRange,
            ),
            (
                "#{ kind: \"PlayingStyleRange\" }",
                None,
                Severity::Error,
                ViolationKind::PlayingStyleRange,
            ),
            (
                "#{ kind: \"NoTier\", heights: [[180, 180], [185, 255]] }",
                None,
                Severity::Error,
                ViolationKind::NoTier {
                    heights: vec![(180, 180), (185, 255)],
                },
            ),
            (
                "#{ kind: \"Stat\", stat: \"speed\", value: 80, target: 92, at_most: true }",
                None,
                Severity::Error,
                ViolationKind::Stat {
                    stat: Stat(12),
                    value: 80,
                    target: 92,
                    at_most: true,
                },
            ),
            (
                "#{ kind: \"StatTotal\", total: 1772, limit: 1771, slot: 3 }",
                Some(3),
                Severity::Error,
                ViolationKind::StatTotal {
                    total: 1772,
                    limit: 1771,
                },
            ),
            (
                "#{ kind: \"Form\", value: 2, target: 4 }",
                None,
                Severity::Error,
                ViolationKind::Form {
                    value: 2,
                    target: 4,
                },
            ),
            (
                "#{ kind: \"InjuryResistance\", value: 0, max: 2 }",
                None,
                Severity::Error,
                ViolationKind::InjuryResistance { value: 0, max: 2 },
            ),
            (
                "#{ kind: \"WeakFootUsage\", value: 2, max: 4 }",
                None,
                Severity::Error,
                ViolationKind::WeakFootUsage { value: 2, max: 4 },
            ),
            (
                "#{ kind: \"WeakFootAccuracy\", value: 1, max: 3 }",
                None,
                Severity::Error,
                ViolationKind::WeakFootAccuracy { value: 1, max: 3 },
            ),
            (
                "#{ kind: \"NameColor\", color: \"c0c0c0ff\", expected: () }",
                None,
                Severity::Error,
                ViolationKind::NameColor {
                    color: Some("c0c0c0ff".to_string()),
                    expected: None,
                },
            ),
            (
                "#{ kind: \"Cards\", paid: 10, limit: 8 }",
                None,
                Severity::Error,
                ViolationKind::Cards { paid: 10, limit: 8 },
            ),
            (
                "#{ kind: \"MinCards\", paid: 3, min: 5 }",
                None,
                Severity::Error,
                ViolationKind::MinCards { paid: 3, min: 5 },
            ),
            (
                "#{ kind: \"SkillCardCap\", count: 11, max: 10 }",
                None,
                Severity::Error,
                ViolationKind::SkillCardCap { count: 11, max: 10 },
            ),
            (
                "#{ kind: \"ComStyleCap\", count: 8, max: 7 }",
                None,
                Severity::Error,
                ViolationKind::ComStyleCap { count: 8, max: 7 },
            ),
            (
                "#{ kind: \"RequiredCard\", card: \"malicia\" }",
                None,
                Severity::Error,
                ViolationKind::RequiredCard {
                    card: Card::Skill(21),
                },
            ),
            (
                "#{ kind: \"APositions\", count: 3, max: 2, rated_a: [\"CF\", \"SS\", \"AMF\"] }",
                None,
                Severity::Error,
                ViolationKind::APositions {
                    count: 3,
                    max: 2,
                    rated_a: vec![Position(12), Position(11), Position(8)],
                },
            ),
            (
                "#{ kind: \"Height\", height: 191, fits: [[0, 175], [180, 180]], room: true }",
                None,
                Severity::Error,
                ViolationKind::Height {
                    height: 191,
                    fits: vec![(0, 175), (180, 180)],
                    room: true,
                },
            ),
            (
                "#{ kind: \"TierCount\", tier: \"gold\", count: 0, expected: 1 }",
                None,
                Severity::Error,
                ViolationKind::TierCount {
                    tier: "gold".to_string(),
                    count: 0,
                    expected: 1,
                },
            ),
            (
                "#{ kind: \"BracketQuota\", system: \"red\", bracket: \"tall\", count: 3, quota: 6 }",
                None,
                Severity::Error,
                ViolationKind::BracketQuota {
                    system: "red".to_string(),
                    bracket: "tall".to_string(),
                    count: 3,
                    quota: 6,
                },
            ),
            (
                "#{ kind: \"MedalSystem\", systems: [\"GGG\", \"GGSS\", \"GSSSS\", \"SSSSSS\"] }",
                None,
                Severity::Error,
                ViolationKind::MedalSystem {
                    systems: vec![
                        "GGG".to_string(),
                        "GGSS".to_string(),
                        "GSSSS".to_string(),
                        "SSSSSS".to_string(),
                    ],
                },
            ),
            (
                "#{ kind: \"ExtraHeight\", used: 51, total: 50 }",
                None,
                Severity::Error,
                ViolationKind::ExtraHeight {
                    used: 51,
                    total: 50,
                },
            ),
            (
                "#{ kind: \"Captain\" }",
                None,
                Severity::Error,
                ViolationKind::Captain,
            ),
            (
                "#{ kind: \"Goalkeepers\", count: 0, min: 1 }",
                None,
                Severity::Error,
                ViolationKind::Goalkeepers { count: 0, min: 1 },
            ),
            (
                "#{ kind: \"TeamSetting\", label: \"no offside trap\", setting: \"offside_trap\", wanted: false }",
                None,
                Severity::Error,
                ViolationKind::TeamSetting {
                    label: "no offside trap".to_string(),
                    setting: "offside_trap".to_string(),
                    wanted: false,
                },
            ),
            (
                "#{ kind: \"FormationShape\", preset: 2, formation: 3, shape: [3, 4, 3] }",
                None,
                Severity::Error,
                ViolationKind::FormationShape {
                    preset: 2,
                    formation: 3,
                    shape: [3, 4, 3],
                },
            ),
            (
                "#{ kind: \"Special\", label: \"Red CBs\", text: \"CBs up to 189\" }",
                None,
                Severity::Error,
                ViolationKind::Special {
                    label: "Red CBs".to_string(),
                    text: "CBs up to 189".to_string(),
                },
            ),
            (
                "#{ kind: \"Custom\", text: \"x\", slot: 0, severity: \"suggestion\" }",
                Some(0),
                Severity::Suggestion,
                ViolationKind::Custom {
                    text: "x".to_string(),
                },
            ),
            (
                "#{ kind: \"Unused\", allowance: \"cards\", used: 1, allowed: 3 }",
                None,
                Severity::Error,
                ViolationKind::Unused {
                    allowance: "cards".to_string(),
                    used: 1,
                    allowed: 3,
                },
            ),
            (
                "#{ kind: \"UnusedChoice\", special: \"Medals\" }",
                None,
                Severity::Error,
                ViolationKind::UnusedChoice {
                    special: "Medals".to_string(),
                },
            ),
            (
                "#{ kind: \"RosterOrder\", player: 70102, slot: 0 }",
                Some(0),
                Severity::Error,
                ViolationKind::RosterOrder { player: 70102 },
            ),
        ];
        assert_eq!(cases.len(), 36);
        for (text, slot, severity, kind) in cases {
            let got = violations(one(text)).expect(text).remove(0);
            assert_eq!(
                got,
                Violation {
                    slot: *slot,
                    severity: *severity,
                    kind: kind.clone(),
                },
                "{text}"
            );
        }
    }

    #[test]
    fn slot_and_severity_default() {
        // No `slot`, no `severity`: `None` and `Error`.
        assert_eq!(one_violation("#{ kind: \"Captain\" }").slot, None);
        assert_eq!(
            one_violation("#{ kind: \"Captain\" }").severity,
            Severity::Error
        );
        assert_eq!(one_violation("#{ kind: \"Captain\", slot: () }").slot, None);
    }

    /// `bad` second in a good-bad-good list is `Violation { index: 1 }`
    /// naming `culprit`.
    fn bad_entry(bad: &str, culprit: &str) {
        let err = violations(
            engine()
                .eval::<Dynamic>(&format!(
                    "[#{{ kind: \"Captain\" }}, {bad}, #{{ kind: \"Custom\", text: \"x\" }}]"
                ))
                .expect("a list"),
        )
        .expect_err("the bad entry");
        match err {
            CheckError::Violation { index, message } => {
                assert_eq!(index, 1);
                assert!(message.contains(culprit), "{culprit}: {message}");
            }
            other => panic!("expected Violation, got {other:?}"),
        }
    }

    #[test]
    fn bad_entries_fail_at_their_index() {
        bad_entry("#{ kind: \"Nope\" }", "Nope");
        bad_entry("#{ kind: \"Age\", age: 15, min: 16 }", "max");
        bad_entry("#{ kind: \"ExtraHeight\", total: 50 }", "used");
        bad_entry("#{ kind: \"StatTotal\", total: 1772 }", "limit");
        bad_entry(
            "#{ kind: \"Age\", age: 15, min: 16, max: 40, extra: 1 }",
            "extra",
        );
        bad_entry("#{ kind: \"Captain\", extra: 1 }", "extra");
        // The hints' fields are required, and a range is a pair.
        bad_entry("#{ kind: \"Height\", height: 183, room: true }", "fits");
        bad_entry("#{ kind: \"NoTier\" }", "heights");
        bad_entry("#{ kind: \"NoTier\", heights: [[180, 180, 185]] }", "2");
        bad_entry("#{ kind: \"BRating\", rated_b: [\"XX\"] }", "XX");
        bad_entry("#{ kind: \"Age\", age: 300, min: 16, max: 40 }", "u8");
        bad_entry("#{ kind: \"Age\", age: -1, min: 16, max: 40 }", "u8");
        bad_entry(
            "#{ kind: \"Stat\", stat: \"pace\", value: 80, target: 92, at_most: true }",
            "pace",
        );
        bad_entry("#{ kind: \"Captain\", severity: \"fatal\" }", "fatal");
        bad_entry("#{ kind: \"Captain\", slot: 300 }", "300");
        bad_entry("5", "not a map");
    }

    #[test]
    fn a_map_is_not_a_list() {
        let err =
            violations(engine().eval::<Dynamic>("#{}").expect("a map")).expect_err("not a list");
        assert!(matches!(err, CheckError::NotAList { .. }), "{err:?}");
    }

    /// `rule` spells the kind and its fields as the CLI prints them (the
    /// spellings of `save_editor`'s
    /// `every_violation_kind_prints_its_rules_file_fields`).
    #[test]
    fn rule_spells_the_kind_and_its_fields() {
        assert_eq!(
            ViolationKind::FieldedNotA {
                preset: 2,
                position: Position(12),
                rated_a: vec![Position(12), Position(11)],
            }
            .rule(),
            (
                "FieldedNotA",
                vec![
                    ("preset", "2".to_string()),
                    ("position", "CF".to_string()),
                    ("rated_a", "[CF, SS]".to_string())
                ]
            )
        );
        // A list of ranges as `low-high`, an empty list as `[]`, a bool as
        // `true`/`false`.
        assert_eq!(
            ViolationKind::Height {
                height: 183,
                fits: vec![(0, 175), (180, 180), (185, 185)],
                room: false,
            }
            .rule(),
            (
                "Height",
                vec![
                    ("height", "183".to_string()),
                    ("fits", "[0-175, 180-180, 185-185]".to_string()),
                    ("room", "false".to_string())
                ]
            )
        );
        assert_eq!(
            ViolationKind::NoTier { heights: vec![] }.rule(),
            ("NoTier", vec![("heights", "[]".to_string())])
        );
        // An `Option` field prints only when `Some`.
        assert_eq!(
            ViolationKind::NameColor {
                color: None,
                expected: Some("cc9900ff".to_string())
            }
            .rule(),
            ("NameColor", vec![("expected", "cc9900ff".to_string())])
        );
        assert_eq!(
            ViolationKind::MedalSystem {
                systems: vec!["GGG".to_string(), "GGSS".to_string()]
            }
            .rule(),
            ("MedalSystem", vec![("systems", "[GGG, GGSS]".to_string())])
        );
    }
}
