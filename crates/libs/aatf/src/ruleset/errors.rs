//! Why a ruleset did not load: `LoadError`, `RuleError`, `LoadRule` and
//! `IdKind`, the findings `check` produces.

use std::fmt;

/// Why a ruleset did not load.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LoadError {
    /// The text does not compile, or its top level fails to run.
    #[error("{line}:{column}: {message}")]
    Parse {
        /// The line the error sits at.
        line: usize,
        /// The column the error sits at.
        column: usize,
        /// What the parser reported.
        message: String,
    },
    /// The text defines no constant `RULESET` map (a `let` is not one:
    /// a function cannot read it).
    #[error("the file defines no constant RULESET map")]
    NoRuleset,
    /// `RULESET` does not deserialize into `Ruleset`: an unknown or missing
    /// key, a wrong type, an unknown label.
    #[error("{path}: {message}")]
    Schema {
        /// The failing key's path (`tiers[2].free.tricks`): a wrong-typed
        /// value's or unknown key's own, a missing key's parent (`.` at the
        /// root); `message` carries rhai's text, which names the key or label.
        path: String,
        /// What the deserializer reported.
        message: String,
    },
    /// A `schema` this build does not read (it reads 1).
    #[error("schema {schema} is not one this build reads (1)")]
    UnknownSchema {
        /// The schema the file names.
        schema: u32,
    },
    /// A required function is not defined with its arity.
    #[error("the file defines no {name} with {arity} parameter(s)")]
    MissingFunction {
        /// The function.
        name: &'static str,
        /// The arity it must take.
        arity: usize,
    },
    /// `custom_checks` takes exactly one parameter (the team); another
    /// arity would be skipped without a word.
    #[error("custom_checks takes one parameter (the team), not {arity}")]
    CustomChecksArity {
        /// The arity the file defines it with.
        arity: usize,
    },
    /// `check_team` fails on the sample team (load step 3).
    #[error("check_team fails on the sample team: {error}")]
    SampleTeam {
        /// The `CheckError` it gave.
        error: crate::violation::CheckError,
    },
    /// The rules the types cannot carry, every one broken, in a fixed order.
    #[error("{}", fmt_rules(.0))]
    Rules(Vec<RuleError>),
}

/// One load-time rule broken at a key path.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleError {
    /// The key path of the value that breaks the rule
    /// (`tiers[2].free.tricks`, `specials[4].when.tiers[1]`; the list itself,
    /// `tiers`, for `CountsExceedSquad`, `CountsShortOfSquad` and
    /// `MoreThanOneRestTier`).
    pub path: String,
    /// Which rule broke.
    pub rule: LoadRule,
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.rule)
    }
}

/// Which of the four id kinds a `DuplicateId`/`UnknownId` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdKind {
    /// A tier.
    Tier,
    /// A height bracket.
    Bracket,
    /// A height system.
    System,
    /// A card group.
    Group,
}

impl fmt::Display for IdKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            IdKind::Tier => "tier",
            IdKind::Bracket => "bracket",
            IdKind::System => "system",
            IdKind::Group => "group",
        })
    }
}

/// The rules the types cannot carry, checked on load ("Loading and
/// validation").
#[derive(Debug, Clone, PartialEq)]
pub enum LoadRule {
    /// An id a file may not repeat within its kind.
    DuplicateId {
        /// Which kind of id repeated.
        kind: IdKind,
        /// The repeated id.
        id: String,
    },
    /// An id a condition, effect, quota, `counts_as` or suggestion names that
    /// exists nowhere.
    UnknownId {
        /// Which kind of id is missing.
        kind: IdKind,
        /// The unknown id.
        id: String,
    },
    /// A string is empty: a `name`, an id, a label, a `card_groups` key
    /// or a `forbid` text.
    Empty,
    /// `page` is not an `http://` or `https://` address: another
    /// beginning, nothing after it, or a space inside.
    PageNotWebAddress,
    /// `logo` is not base64 text (the standard alphabet, padded; no
    /// spaces, no `data:` prefix).
    LogoNotBase64,
    /// `logo` decodes to bytes that do not start with the PNG signature.
    LogoNotPng,
    /// `logo` decodes to more than the 64 KB a logo may hold.
    LogoTooLarge {
        /// The decoded size.
        bytes: usize,
    },
    /// A `name_colors` entry or a tier's `name_color` is not eight bytes
    /// long (any bytes — the game does not check they are hex).
    ColorCodeLength,
    /// A `suggestions` entry that is neither one of the seven words nor a
    /// group id.
    UnknownSuggestion {
        /// The entry.
        entry: String,
    },
    /// An `at_most` entry that is not an ability stat or one of `form`,
    /// `injury_resistance`, `weak_foot_usage`, `weak_foot_accuracy`.
    UnknownTarget {
        /// The entry.
        name: String,
    },
    /// A `card_groups` key `com` or one of `suggestions`' seven words:
    /// `com` is the COM styles' group, and a word named a group would let
    /// a suggestion mean two allowances.
    ReservedGroup,
    /// A card in two `card_groups`, or a COM style in any (the built-in
    /// `com` group holds the seven): the allocation would make the card
    /// free by whichever group it tries first, so a team's legality
    /// would hang on the groups' names.
    GroupsOverlap {
        /// The card.
        card: String,
        /// The earlier group holding it (`com` for a COM style).
        other: String,
    },
    /// A tier with `counts_as` also has `count`.
    CountWithCountsAs,
    /// The tier a `counts_as` names has a `counts_as` of its own (itself
    /// included).
    CountsAsChain,
    /// A tier a medal system counts has `count`.
    MedalTierCounted,
    /// A tier a medal system counts has `counts_as`.
    MedalTierCountsAs,
    /// A `counts_as` names a tier a medal system counts.
    CountsAsMedalTier,
    /// More than one tier has neither `count` nor `counts_as`.
    MoreThanOneRestTier,
    /// The counts' sum is over `squad_size`.
    CountsExceedSquad {
        /// The counts' sum.
        total: u32,
        /// The squad's size.
        squad_size: u8,
    },
    /// No tier is the rest of the squad, and the counts' sum is under
    /// `squad_size`.
    CountsShortOfSquad {
        /// The counts' sum.
        total: u32,
        /// The squad's size.
        squad_size: u8,
    },
    /// A height system other than the last has no `any_player`, or an
    /// empty one, and no `by_quotas` with quotas.
    SystemWithoutCondition,
    /// A bracket's lowest height is at most the highest height of the
    /// brackets before it: a reordered bracket takes no player.
    BracketsNotAscending,
    /// A tier's `when` names tiers.
    TierConditionNamesTiers,
    /// A system's `any_player` sets more than `height_min`, `height_max` and
    /// `positions`.
    SystemConditionTooWide,
    /// The last system has `any_player` or `by_quotas` set.
    LastSystemHasCondition,
    /// A system with `extra_height` quotas a bracket that is not an
    /// `exactly` bracket of one height: the draw needs the bracket's own
    /// height.
    ExtraHeightQuotaNotExact,
    /// A system with `extra_height`'s quotas do not add up to
    /// `squad_size`.
    ExtraHeightQuotasNotSquad {
        /// The quotas' sum.
        total: u32,
        /// The squad's size.
        squad_size: u8,
    },
    /// A special with `height_allowance` (in `then` or any `choice` option)
    /// names a bracket in its `when`, nested `not` and `any` included.
    AllowanceWithBracket,
    /// A `choice` option holding an effect its doc does not list: only
    /// `stat_bonus`, `cards`, `free`, `free_cards`, `a_positions`,
    /// `max_a_positions` and `weak_foot` can be found used.
    ChoiceOptionEffect,
    /// `fielded_at_a` holds a preset outside 1 to 3.
    PresetOutOfRange {
        /// The out-of-range preset.
        preset: u8,
    },
    /// A forbidden shape's counts do not add up to 10.
    ShapeNotTen {
        /// The shape (`[defenders, midfielders, forwards]`).
        shape: [u8; 3],
    },
}

impl fmt::Display for LoadRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadRule::DuplicateId { kind, id } => write!(f, "{kind} id {id:?} is used twice"),
            LoadRule::UnknownId { kind, id } => write!(f, "unknown {kind} id {id:?}"),
            LoadRule::UnknownSuggestion { entry } => {
                write!(f, "unknown suggestion {entry:?}")
            }
            LoadRule::UnknownTarget { name } => write!(f, "unknown at_most target {name:?}"),
            LoadRule::ReservedGroup => {
                f.write_str("`com` or a `suggestions` word is a reserved card group")
            }
            LoadRule::GroupsOverlap { card, other } => {
                write!(f, "card {card:?} is also in group {other:?}")
            }
            LoadRule::CountWithCountsAs => f.write_str("a tier with `counts_as` also has `count`"),
            LoadRule::CountsAsChain => {
                f.write_str("the tier a `counts_as` names has a `counts_as` of its own")
            }
            LoadRule::MedalTierCounted => {
                f.write_str("a tier a medal system counts has `count`")
            }
            LoadRule::MedalTierCountsAs => {
                f.write_str("a tier a medal system counts has `counts_as`")
            }
            LoadRule::CountsAsMedalTier => {
                f.write_str("a `counts_as` names a tier a medal system counts")
            }
            LoadRule::MoreThanOneRestTier => {
                f.write_str("more than one tier has neither `count` nor `counts_as`")
            }
            LoadRule::CountsExceedSquad { total, squad_size } => {
                write!(f, "counts total {total} over squad_size {squad_size}")
            }
            LoadRule::CountsShortOfSquad { total, squad_size } => {
                write!(f, "counts total {total} under squad_size {squad_size}")
            }
            LoadRule::SystemWithoutCondition => {
                f.write_str("a height system other than the last has no or an empty `any_player` and no `by_quotas` with quotas")
            }
            LoadRule::Empty => f.write_str("empty string"),
            LoadRule::PageNotWebAddress => {
                f.write_str("not a web address (http:// or https://)")
            }
            LoadRule::LogoNotBase64 => f.write_str("not base64 text"),
            LoadRule::LogoNotPng => f.write_str("not a PNG picture"),
            LoadRule::LogoTooLarge { bytes } => {
                write!(f, "{bytes} bytes, over the 64 KB a logo may hold")
            }
            LoadRule::ColorCodeLength => f.write_str("a name colour code is not eight bytes"),
            LoadRule::BracketsNotAscending => {
                f.write_str("bracket heights not above the brackets before it")
            }
            LoadRule::TierConditionNamesTiers => f.write_str("a tier's `when` names tiers"),
            LoadRule::SystemConditionTooWide => {
                f.write_str("`any_player` sets more than heights and positions")
            }
            LoadRule::LastSystemHasCondition => {
                f.write_str("the last system has `any_player` or `by_quotas`")
            }
            LoadRule::ExtraHeightQuotaNotExact => {
                f.write_str("an `extra_height` system quotas a bracket that is not `exactly` one height")
            }
            LoadRule::ExtraHeightQuotasNotSquad { total, squad_size } => {
                write!(f, "an `extra_height` system's quotas add up to {total}, not squad_size {squad_size}")
            }
            LoadRule::AllowanceWithBracket => {
                f.write_str("a special with `height_allowance` names a bracket")
            }
            LoadRule::ChoiceOptionEffect => {
                f.write_str("a `choice` option holds an effect it cannot be used through")
            }
            LoadRule::PresetOutOfRange { preset } => {
                write!(f, "preset {preset} is outside 1 to 3")
            }
            LoadRule::ShapeNotTen { shape } => {
                write!(f, "shape {shape:?} does not add up to 10")
            }
        }
    }
}

/// The `Rules` display's join of `RuleError`s.
fn fmt_rules(rules: &[RuleError]) -> String {
    rules
        .iter()
        .map(RuleError::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}
