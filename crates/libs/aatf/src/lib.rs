//! `aatf`: the 4cc player rules (the community's "Auto-ATF" check). A
//! ruleset is **data** — one Rhai map naming the tiers, the card economy,
//! the heights and a list of conditional "specials" — read here into
//! `Ruleset` and checked against the rules the types cannot carry. A
//! `Rules` loads a whole rules file (steps 1 to 3 of "Loading and
//! validation": shape, `RULESET`, `check_team` on the sample team) and
//! runs its `check_team` on a `TeamView`, which a save or a Team TOML
//! document builds; a file knows its `logic()` identity. The shipped
//! logic (`logic/logic.rhai`) is the generic interpreter: `check_team`,
//! `tier_values` (behind `apply_tier`, the one writing operation),
//! `card_limits` and `player_tier` (the tier a player is recognized
//! as).
//!
//! The host reads a team as a `TeamView` of `PlayerView`s on the game's
//! own scale, and every script runs on the one engine of `engine.rs`.
//! The script API a ruleset sees, on `Player`: `slot`, `age`, `height`,
//! `weight`; `form`, `injury_resistance`, `weak_foot_usage`,
//! `weak_foot_accuracy` (the game's scale); `position`, `playing_style`,
//! `name_color` (label or `()`), `strong_foot` (`"right"` or `"left"`,
//! `()` past them); `cards` (the held skills and COM styles'
//! labels); `stat(name)` (the ability stat's value, `()` where the
//! version lacks it; an unknown name is a script error);
//! `rating_at(pos)` (`"A"`, `"B"` or `"C"`; a stored 3, which no save
//! holds, reads `"3"`; an unknown position is a script error).
//! On `Team`: `players` (the roster in slot order), `captain` (the slot,
//! `()` for none), `version` (the short number);
//! `fielded_positions(player, preset)` (labels, once each, in the
//! position table's order, empty for a player outside the starting
//! eleven; a preset outside 1 to 3 is a script error),
//! `fielded_only_at(player, pos)` (whether every formation the game can
//! field from the three presets fields the player at `pos`; for a player
//! outside the starting eleven, whether `pos` is his registered
//! position; an unknown position is a script error),
//! `formation(preset, formation)` (the eleven slots as
//! `#{ slot, position, x, y }`, `()` for a formation the preset cannot
//! field), `formation_shapes()` (`#{ preset, formation,
//! shape: [defenders, midfielders, forwards] }`); `man_marking` (whether
//! any preset has a man-marking assignment set), `auto_substitution`
//! (the stored setting, 0 off), `auto_offside_trap`,
//! `auto_preset_change`, `auto_attack_defence_levels` (the flag), each
//! auto value `()` on a version without it.

/// The one engine every script runs on: packages, accessors, limits.
mod engine;
/// Assembling a rules file: data block, current logic, custom-checks
/// stub, and reading a file's sections back.
mod file;
/// The per-ruleset fixtures: known-legal and known-violating teams.
#[cfg(test)]
mod fixtures;
/// `Rules`: a loaded file, steps 1 to 3, `check_team`.
mod rules;
/// `Ruleset` and its types, deserialization and the load-time rules.
mod ruleset;
/// `TeamView`/`PlayerView` from `pes_savefile`'s model, the tactics answers.
mod team;
/// The embedded rulesets: the Ruleset editor's start page templates
/// (`templates()`) and the Save editor's built-in rulesets (`builtin`).
mod templates;
/// `TierValues`/`CardLimits` from the script's maps, and the write into a record.
mod values;
/// `Violation` and the script-to-`Violation` conversion.
mod violation;
/// `Ruleset` written as its `RULESET` data block (`to_data_block`).
mod writer;

pub use file::{FileSections, LogicIdentity, assemble_file, file_sections};
pub use rules::{RULES_FILE_EXTENSION, Rules, official_file};
pub use team::{PlayerView, TeamView, TeamViewError};
pub use templates::{Template, builtin, templates};
pub use values::CardLimits;
pub use violation::{CheckError, Violation, ViolationKind};

pub use ruleset::{
    Bracket, BracketHeights, BracketId, Card, Condition, Effects, ExtraHeight, Foot, GroupId,
    HeightSystem, Heights, IdKind, LoadError, LoadRule, LogoRefusal, MedalSystem, NON_STAT_TARGETS,
    PerTier, PlayStyle, PlayerValue, Position, RuleError, Ruleset, Severity, Special, Stat,
    StatOrAll, StatRules, SystemId, TeamSettings, Tier, TierId, Universal, WeakFoot, WeightRule,
};
