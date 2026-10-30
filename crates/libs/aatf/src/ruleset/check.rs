//! The load-time rules: `check` and its helpers. The error types they
//! produce are in `errors.rs`.

use std::collections::HashSet;

use pes_savefile::interchange::team_toml::labels::ABILITY_STATS;

use super::*;

/// The seven `suggestions` words; reserved as `card_groups` keys too,
/// so a suggestion can never name two allowances.
const SUGGESTION_WORDS: [&str; 7] = [
    "cards",
    "a_positions",
    "weak_foot",
    "injury_resistance",
    "height",
    "free_cards",
    "choices",
];

impl Ruleset {
    /// The rules the types cannot carry, every one broken, in a fixed
    /// order: `Ruleset`'s fields in declaration order, list items in
    /// order, map entries in key order; the tier rules that need the
    /// medal systems' tiers come after `medal_systems`. The Ruleset
    /// editor calls it on a ruleset being edited, so the rail can mark
    /// the sections its errors sit in.
    pub fn check(&self) -> Vec<RuleError> {
        let mut out = Vec::new();

        // name: an emptied string names nothing.
        check_empty(&mut out, "name".to_string(), &self.name);

        // page and logo: refused rather than dropped, so an organizer
        // whose logo never shows learns why.
        if let Some(page) = &self.page
            && !is_web_address(page)
        {
            push(&mut out, "page".to_string(), LoadRule::PageNotWebAddress);
        }
        if let Some(rule) = self.logo.as_deref().and_then(logo_rule) {
            push(&mut out, "logo".to_string(), rule);
        }

        // stats.at_most: an ability stat, or one of the non-stat targets.
        for (i, name) in self.stats.at_most.iter().enumerate() {
            if !ABILITY_STATS.contains(&name.as_str()) && !NON_STAT_TARGETS.contains(&name.as_str())
            {
                push(
                    &mut out,
                    format!("stats.at_most[{i}]"),
                    LoadRule::UnknownTarget { name: name.clone() },
                );
            }
        }

        // tiers: DuplicateId (at the second), counts_as, free keys, the
        // when condition, count/counts_as.
        for (i, tier) in self.tiers.iter().enumerate() {
            let path = format!("tiers[{i}]");
            check_empty(&mut out, format!("{path}.id"), &tier.id);
            check_empty(&mut out, format!("{path}.label"), &tier.label);
            check_unique(
                &mut out,
                &path,
                IdKind::Tier,
                &tier.id,
                self.tiers[..i].iter().map(|earlier| earlier.id.as_str()),
            );
            if let Some(counts_as) = &tier.counts_as {
                if !self.id_known(IdKind::Tier, counts_as) {
                    push(
                        &mut out,
                        format!("{path}.counts_as"),
                        LoadRule::UnknownId {
                            kind: IdKind::Tier,
                            id: counts_as.clone(),
                        },
                    );
                } else if self
                    .tiers
                    .iter()
                    .any(|named| named.id == *counts_as && named.counts_as.is_some())
                {
                    // The tier `counts_as` names has a `counts_as` of its
                    // own, itself included.
                    push(
                        &mut out,
                        format!("{path}.counts_as"),
                        LoadRule::CountsAsChain,
                    );
                }
                if tier.count.is_some() {
                    push(&mut out, path.clone(), LoadRule::CountWithCountsAs);
                }
            }
            for key in tier.free.keys() {
                self.check_id(&mut out, format!("{path}.free.{key}"), IdKind::Group, key);
            }
            self.check_condition(&tier.when, &format!("{path}.when"), false, &mut out);
            if let Some(name_color) = &tier.name_color {
                check_color_code(&mut out, format!("{path}.name_color"), name_color);
            }
        }
        let total: u32 = self
            .tiers
            .iter()
            .filter_map(|tier| tier.count.map(u32::from))
            .sum();
        if total > u32::from(self.squad_size) {
            push(
                &mut out,
                "tiers".to_string(),
                LoadRule::CountsExceedSquad {
                    total,
                    squad_size: self.squad_size,
                },
            );
        }
        // The tiers the medal systems count, taken as a set: a tier one
        // names takes its count from the systems, not from the squad's
        // rest.
        let medal_tiers: HashSet<&str> = self
            .medal_systems
            .iter()
            .flat_map(|system| system.counts.keys().map(String::as_str))
            .collect();
        let rest_tiers = self
            .tiers
            .iter()
            .filter(|tier| {
                tier.count.is_none()
                    && tier.counts_as.is_none()
                    && !medal_tiers.contains(tier.id.as_str())
            })
            .count();
        // The counts equal `squad_size` when no tier is the rest of the
        // squad — under it, no team can be legal. With medal systems each
        // system's counts close the gap instead (checked with them below).
        if rest_tiers == 0 && medal_tiers.is_empty() && total < u32::from(self.squad_size) {
            push(
                &mut out,
                "tiers".to_string(),
                LoadRule::CountsShortOfSquad {
                    total,
                    squad_size: self.squad_size,
                },
            );
        }
        if rest_tiers > 1 {
            push(&mut out, "tiers".to_string(), LoadRule::MoreThanOneRestTier);
        }

        // card_groups: keys name a group; `com` and `suggestions`' seven
        // words are reserved (`com` for the COM styles, the words so a
        // suggestion can never name two allowances). The groups are
        // disjoint and hold no COM style: a card in two would be made
        // free by whichever group the allocation tries first.
        for (index, (key, cards)) in self.card_groups.iter().enumerate() {
            check_empty(&mut out, "card_groups".to_string(), key);
            if key == "com" || SUGGESTION_WORDS.contains(&key.as_str()) {
                push(
                    &mut out,
                    format!("card_groups.{key}"),
                    LoadRule::ReservedGroup,
                );
            }
            for card in cards {
                // `com` is the first group the allocation tries.
                if matches!(card, Card::ComStyle(_)) {
                    push(
                        &mut out,
                        format!("card_groups.{key}"),
                        LoadRule::GroupsOverlap {
                            card: card.label().to_string(),
                            other: "com".to_string(),
                        },
                    );
                }
                for (other, earlier) in self.card_groups.iter().take(index) {
                    if earlier.contains(card) {
                        push(
                            &mut out,
                            format!("card_groups.{key}"),
                            LoadRule::GroupsOverlap {
                                card: card.label().to_string(),
                                other: other.clone(),
                            },
                        );
                    }
                }
            }
        }

        // medal_systems: labels non-empty, `counts` naming existing
        // tiers. Each system's counts plus the fixed counts fit
        // `squad_size`, and equal it when no tier is the rest of the
        // squad — under it, no team can be legal.
        for (i, system) in self.medal_systems.iter().enumerate() {
            let path = format!("medal_systems[{i}]");
            check_empty(&mut out, format!("{path}.label"), &system.label);
            for id in system.counts.keys() {
                self.check_id(&mut out, format!("{path}.counts.{id}"), IdKind::Tier, id);
            }
            let counts = total + system.counts.values().map(|n| u32::from(*n)).sum::<u32>();
            if counts > u32::from(self.squad_size) {
                push(
                    &mut out,
                    path,
                    LoadRule::CountsExceedSquad {
                        total: counts,
                        squad_size: self.squad_size,
                    },
                );
            } else if rest_tiers == 0 && counts < u32::from(self.squad_size) {
                push(
                    &mut out,
                    path,
                    LoadRule::CountsShortOfSquad {
                        total: counts,
                        squad_size: self.squad_size,
                    },
                );
            }
        }
        // A tier a medal system counts takes its count from the systems:
        // no `count`, no `counts_as` of its own, and no `counts_as`
        // naming it.
        for (i, tier) in self.tiers.iter().enumerate() {
            if tier.count.is_some() && medal_tiers.contains(tier.id.as_str()) {
                push(&mut out, format!("tiers[{i}]"), LoadRule::MedalTierCounted);
            }
            if let Some(counts_as) = &tier.counts_as {
                if medal_tiers.contains(tier.id.as_str()) {
                    push(
                        &mut out,
                        format!("tiers[{i}].counts_as"),
                        LoadRule::MedalTierCountsAs,
                    );
                }
                if medal_tiers.contains(counts_as.as_str()) {
                    push(
                        &mut out,
                        format!("tiers[{i}].counts_as"),
                        LoadRule::CountsAsMedalTier,
                    );
                }
            }
        }

        // heights: brackets' and systems' ids unique and non-empty,
        // each bracket's heights above the brackets before it, quota
        // keys existing, a non-last system's `any_player` present and
        // non-empty and within heights and positions, the last system's
        // absent or empty.
        let mut previous_highest = 0u8;
        for (i, bracket) in self.heights.brackets.iter().enumerate() {
            let path = format!("heights.brackets[{i}]");
            check_empty(&mut out, format!("{path}.id"), &bracket.id);
            check_empty(&mut out, format!("{path}.label"), &bracket.label);
            check_unique(
                &mut out,
                &path,
                IdKind::Bracket,
                &bracket.id,
                self.heights.brackets[..i]
                    .iter()
                    .map(|earlier| earlier.id.as_str()),
            );
            // An `up_to: n` bracket takes the heights above the brackets
            // before it up to `n`, so it takes none unless `n` tops them;
            // an `exactly` list's minimum must top them.
            let (lowest, highest) = match &bracket.heights {
                BracketHeights::UpTo(limit) => (*limit, *limit),
                BracketHeights::Exactly(heights) => (
                    *heights
                        .iter()
                        .min()
                        .expect("`RawBracket` refuses an empty `exactly`"),
                    *heights
                        .iter()
                        .max()
                        .expect("`RawBracket` refuses an empty `exactly`"),
                ),
            };
            if lowest <= previous_highest {
                push(&mut out, path.clone(), LoadRule::BracketsNotAscending);
            }
            previous_highest = previous_highest.max(highest);
        }
        for (i, system) in self.heights.systems.iter().enumerate() {
            let path = format!("heights.systems[{i}]");
            check_empty(&mut out, format!("{path}.id"), &system.id);
            check_empty(&mut out, format!("{path}.label"), &system.label);
            check_unique(
                &mut out,
                &path,
                IdKind::System,
                &system.id,
                self.heights.systems[..i]
                    .iter()
                    .map(|earlier| earlier.id.as_str()),
            );
            // Every system but the last must have `any_player` or
            // `by_quotas` with quotas: with neither it holds for every team
            // (no quota fails), and the systems after it could never apply.
            if i + 1 != self.heights.systems.len()
                && system.any_player.as_ref().is_none_or(Condition::is_empty)
                && !(system.by_quotas && !system.quotas.is_empty())
            {
                push(&mut out, path.clone(), LoadRule::SystemWithoutCondition);
            }
            // Heights and positions only, so no id or colour code to check.
            if let Some(condition) = &system.any_player
                && condition_is_wide(condition)
            {
                push(
                    &mut out,
                    format!("{path}.any_player"),
                    LoadRule::SystemConditionTooWide,
                );
            }
            for key in system.quotas.keys() {
                let bracket = self
                    .heights
                    .brackets
                    .iter()
                    .find(|bracket| &bracket.id == key);
                match bracket {
                    None => push(
                        &mut out,
                        format!("{path}.quotas.{key}"),
                        LoadRule::UnknownId {
                            kind: IdKind::Bracket,
                            id: key.clone(),
                        },
                    ),
                    // A pool system's quotas name `exactly` brackets of
                    // one height: the draw needs the bracket's own height.
                    Some(bracket)
                        if system.extra_height.is_some()
                            && !matches!(&bracket.heights, BracketHeights::Exactly(h) if h.len() == 1) =>
                    {
                        push(
                            &mut out,
                            format!("{path}.quotas.{key}"),
                            LoadRule::ExtraHeightQuotaNotExact,
                        );
                    }
                    _ => {}
                }
            }
            // A pool system's quotas fill the squad exactly.
            if system.extra_height.is_some() {
                let quotas: u32 = system.quotas.values().map(|n| u32::from(*n)).sum();
                if quotas != u32::from(self.squad_size) {
                    push(
                        &mut out,
                        format!("{path}.quotas"),
                        LoadRule::ExtraHeightQuotasNotSquad {
                            total: quotas,
                            squad_size: self.squad_size,
                        },
                    );
                }
            }
        }
        if let Some(last) = self.heights.systems.last() {
            let last_path = format!("heights.systems[{}]", self.heights.systems.len() - 1);
            if last
                .any_player
                .as_ref()
                .is_some_and(|condition| !condition.is_empty())
            {
                push(
                    &mut out,
                    format!("{last_path}.any_player"),
                    LoadRule::LastSystemHasCondition,
                );
            }
            if last.by_quotas {
                push(
                    &mut out,
                    format!("{last_path}.by_quotas"),
                    LoadRule::LastSystemHasCondition,
                );
            }
        }

        // universal: `fielded_at_a` names presets 1 to 3.
        for (i, preset) in self.universal.fielded_at_a.iter().enumerate() {
            if !(1..=3).contains(preset) {
                push(
                    &mut out,
                    format!("universal.fielded_at_a[{i}]"),
                    LoadRule::PresetOutOfRange { preset: *preset },
                );
            }
        }

        // specials: condition ids exist; `height_allowance` (in `then` or
        // any `choice` option) names no bracket in `when`.
        for (i, special) in self.specials.iter().enumerate() {
            let path = format!("specials[{i}]");
            check_empty(&mut out, format!("{path}.label"), &special.label);
            self.check_condition(&special.when, &format!("{path}.when"), true, &mut out);
            if effects_have_allowance(&special.then) {
                allowance_with_bracket(&special.when, &format!("{path}.when"), &mut out);
            }
            self.check_effects(&special.then, &format!("{path}.then"), &mut out);
        }

        // team_settings: a forbidden shape's counts add up to 10.
        for (i, settings) in self.team_settings.iter().enumerate() {
            check_empty(
                &mut out,
                format!("team_settings[{i}].label"),
                &settings.label,
            );
            for (j, shape) in settings.forbidden_shapes.iter().enumerate() {
                if u32::from(shape[0]) + u32::from(shape[1]) + u32::from(shape[2]) != 10 {
                    push(
                        &mut out,
                        format!("team_settings[{i}].forbidden_shapes[{j}]"),
                        LoadRule::ShapeNotTen { shape: *shape },
                    );
                }
            }
        }

        // suggestions: one of the seven words or a group id (`com`
        // included).
        for (i, entry) in self.suggestions.iter().enumerate() {
            if !SUGGESTION_WORDS.contains(&entry.as_str()) && !self.id_known(IdKind::Group, entry) {
                push(
                    &mut out,
                    format!("suggestions[{i}]"),
                    LoadRule::UnknownSuggestion {
                        entry: entry.clone(),
                    },
                );
            }
        }

        out
    }

    /// Whether `id` names an entry of `kind` (a group id may also be
    /// `com`, the built-in COM-styles group).
    fn id_known(&self, kind: IdKind, id: &str) -> bool {
        match kind {
            IdKind::Tier => self.tiers.iter().any(|tier| tier.id == id),
            IdKind::Group => id == "com" || self.card_groups.contains_key(id),
            IdKind::System => self.heights.systems.iter().any(|system| system.id == id),
            IdKind::Bracket => self.heights.brackets.iter().any(|bracket| bracket.id == id),
        }
    }

    /// `UnknownId` at `path` when `id` names no entry of `kind`.
    fn check_id(&self, out: &mut Vec<RuleError>, path: String, kind: IdKind, id: &str) {
        if !self.id_known(kind, id) {
            push(
                out,
                path,
                LoadRule::UnknownId {
                    kind,
                    id: id.to_string(),
                },
            );
        }
    }

    /// `check_id` for each of `ids`, reported at `{path}[{i}]`.
    fn check_ids(&self, out: &mut Vec<RuleError>, path: &str, kind: IdKind, ids: &[String]) {
        for (i, id) in ids.iter().enumerate() {
            self.check_id(out, format!("{path}[{i}]"), kind, id);
        }
    }

    /// A condition's tiers, systems and brackets checked for existence —
    /// every name a condition may hold is an error (never a skip):
    /// `UnknownId` where the name exists nowhere. `allow_tiers` splits the
    /// two `tiers` rules: `true` for a special's `when` (tiers are
    /// legitimate names), `false` for a tier's own `when`
    /// (`TierConditionNamesTiers`). `not` is checked like the condition it
    /// sits in.
    fn check_condition(
        &self,
        condition: &Condition,
        path: &str,
        allow_tiers: bool,
        out: &mut Vec<RuleError>,
    ) {
        if allow_tiers {
            self.check_ids(
                out,
                &format!("{path}.tiers"),
                IdKind::Tier,
                &condition.tiers,
            );
        } else if !condition.tiers.is_empty() {
            push(
                out,
                format!("{path}.tiers"),
                LoadRule::TierConditionNamesTiers,
            );
        }
        self.check_ids(
            out,
            &format!("{path}.systems"),
            IdKind::System,
            &condition.systems,
        );
        self.check_ids(
            out,
            &format!("{path}.brackets"),
            IdKind::Bracket,
            &condition.brackets,
        );
        for (i, entry) in condition.name_colors.iter().enumerate() {
            check_color_code(out, format!("{path}.name_colors[{i}]"), entry);
        }
        for (i, entry) in condition.any.iter().enumerate() {
            self.check_condition(entry, &format!("{path}.any[{i}]"), allow_tiers, out);
        }
        if let Some(not) = &condition.not {
            self.check_condition(not, &format!("{path}.not"), allow_tiers, out);
        }
    }

    /// A `ByTier` map's tier keys checked for existence at
    /// `prefix.{id}`; `All` maps nothing.
    fn check_per_tier<T>(&self, per_tier: &PerTier<T>, prefix: &str, out: &mut Vec<RuleError>) {
        let PerTier::ByTier(map) = per_tier else {
            return;
        };
        for id in map.keys() {
            self.check_id(out, format!("{prefix}.{id}"), IdKind::Tier, id);
        }
    }

    /// An effects block's PerTier map keys (tier ids) and `free` keys
    /// (group ids) checked for existence, recursively into `choice`.
    fn check_effects(&self, effects: &Effects, path: &str, out: &mut Vec<RuleError>) {
        for (key, per_tier) in &effects.stat_bonus {
            let label = match key {
                StatOrAll::All => "all",
                StatOrAll::Stat(stat) => stat.label(),
            };
            self.check_per_tier(per_tier, &format!("{path}.stat_bonus.{label}"), out);
        }
        for (stat, per_tier) in &effects.stat_target {
            self.check_per_tier(
                per_tier,
                &format!("{path}.stat_target.{}", stat.label()),
                out,
            );
        }
        for (name, per_tier) in [
            ("cards", &effects.cards),
            ("a_positions", &effects.a_positions),
            ("form", &effects.form),
            ("injury_resistance", &effects.injury_resistance),
        ] {
            if let Some(per_tier) = per_tier {
                self.check_per_tier(per_tier, &format!("{path}.{name}"), out);
            }
        }
        if let Some(per_tier) = &effects.stat_warning {
            self.check_per_tier(per_tier, &format!("{path}.stat_warning"), out);
        }
        for (group, per_tier) in &effects.free {
            self.check_id(out, format!("{path}.free.{group}"), IdKind::Group, group);
            self.check_per_tier(per_tier, &format!("{path}.free.{group}"), out);
        }
        // `forbid`: an emptied text would show a blank violation.
        if let Some(forbid) = &effects.forbid {
            check_empty(out, format!("{path}.forbid"), forbid);
        }
        for (i, option) in effects.choice.iter().enumerate() {
            let option_path = format!("{path}.choice[{i}]");
            // An option holds only the effects `Effects::choice`'s doc
            // lists (`LoadRule::ChoiceOptionEffect`): an option is found
            // used through a limit it raises, which the others never do.
            for (field, present) in [
                ("stat_target", !option.stat_target.is_empty()),
                ("require_cards", !option.require_cards.is_empty()),
                ("form", option.form.is_some()),
                ("injury_resistance", option.injury_resistance.is_some()),
                ("height_allowance", option.height_allowance.is_some()),
                ("forbid", option.forbid.is_some()),
                ("choice", !option.choice.is_empty()),
                ("stat_warning", option.stat_warning.is_some()),
            ] {
                if present {
                    push(
                        out,
                        format!("{option_path}.{field}"),
                        LoadRule::ChoiceOptionEffect,
                    );
                }
            }
            self.check_effects(option, &option_path, out);
        }
    }
}

/// `RuleError { path, rule }` pushed onto `out`: the one shape every
/// rule reports through.
fn push(out: &mut Vec<RuleError>, path: String, rule: LoadRule) {
    out.push(RuleError { path, rule });
}

/// `ColorCodeLength` at `path` unless `code` is eight bytes: a tier's
/// `name_color` and a condition's `name_colors` entry alike (any bytes:
/// a partial paste matches no name).
fn check_color_code(out: &mut Vec<RuleError>, path: String, code: &str) {
    if code.len() != 8 {
        push(out, path, LoadRule::ColorCodeLength);
    }
}

/// The first eight bytes of every PNG file.
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// The most bytes a decoded logo may hold: 64 KB.
const MAX_LOGO_BYTES: usize = 65536;

/// Whether `page` is `http://` or `https://` followed by at least one
/// character and no whitespace anywhere.
fn is_web_address(page: &str) -> bool {
    page.strip_prefix("https://")
        .or_else(|| page.strip_prefix("http://"))
        .is_some_and(|rest| !rest.is_empty() && !rest.contains(char::is_whitespace))
}

impl Ruleset {
    /// The event logo's PNG bytes; `None` when the ruleset carries no
    /// logo or its text is not base64. A ruleset loaded through
    /// `Rules::load` was checked, so its logo decodes.
    pub fn logo_png(&self) -> Option<Vec<u8>> {
        use base64::Engine as _;
        let logo = self.logo.as_deref()?;
        base64::engine::general_purpose::STANDARD.decode(logo).ok()
    }

    /// `png` stored as the event logo, base64. Refused, the logo left as
    /// it was, when the bytes lack the PNG signature (`NotPng`) or hold
    /// more than 64 KB (`TooLarge`): the rules `check` applies to a loaded
    /// logo.
    pub fn set_logo(&mut self, png: &[u8]) -> Result<(), LogoRefusal> {
        use base64::Engine as _;
        if let Some(refusal) = png_rule(png) {
            return Err(refusal);
        }
        self.logo = Some(base64::engine::general_purpose::STANDARD.encode(png));
        Ok(())
    }
}

/// The rule `logo` breaks, the first of: not base64, not a PNG, over
/// 64 KB. The picture itself is never decoded here: the hosts that draw
/// it do.
fn logo_rule(logo: &str) -> Option<LoadRule> {
    use base64::Engine as _;
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(logo) else {
        return Some(LoadRule::LogoNotBase64);
    };
    png_rule(&bytes).map(LoadRule::from)
}

/// Why `Ruleset::set_logo` refused a picture: the two rules a logo's
/// bytes must meet, which a loaded logo breaks as `LoadRule::LogoNotPng`
/// and `LoadRule::LogoTooLarge`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoRefusal {
    /// The bytes do not start with the PNG signature.
    NotPng,
    /// The bytes are more than the 64 KB a logo may hold.
    TooLarge {
        /// How many bytes the picture holds.
        bytes: usize,
    },
}

impl From<LogoRefusal> for LoadRule {
    fn from(refusal: LogoRefusal) -> LoadRule {
        match refusal {
            LogoRefusal::NotPng => LoadRule::LogoNotPng,
            LogoRefusal::TooLarge { bytes } => LoadRule::LogoTooLarge { bytes },
        }
    }
}

/// The refusal a logo's decoded bytes earn, the first of: not a PNG,
/// over 64 KB. A loaded logo and one `set_logo` stores pass the same
/// check.
fn png_rule(bytes: &[u8]) -> Option<LogoRefusal> {
    if !bytes.starts_with(&PNG_SIGNATURE) {
        return Some(LogoRefusal::NotPng);
    }
    if bytes.len() > MAX_LOGO_BYTES {
        return Some(LogoRefusal::TooLarge { bytes: bytes.len() });
    }
    None
}

/// `Empty` at `path` when `value` is empty.
fn check_empty(out: &mut Vec<RuleError>, path: String, value: &str) {
    if value.is_empty() {
        push(out, path, LoadRule::Empty);
    }
}

/// `DuplicateId` at `path` when `earlier` already holds `id`.
fn check_unique<'a>(
    out: &mut Vec<RuleError>,
    path: &str,
    kind: IdKind,
    id: &str,
    mut earlier: impl Iterator<Item = &'a str>,
) {
    if earlier.any(|earlier_id| earlier_id == id) {
        push(
            out,
            path.to_string(),
            LoadRule::DuplicateId {
                kind,
                id: id.to_string(),
            },
        );
    }
}

/// Every `brackets` entry of `condition` reported at its own path,
/// `not` nesting included: a special with `height_allowance` names no
/// bracket, brackets being computed from the allowance.
fn allowance_with_bracket(condition: &Condition, path: &str, out: &mut Vec<RuleError>) {
    for (i, _) in condition.brackets.iter().enumerate() {
        push(
            out,
            format!("{path}.brackets[{i}]"),
            LoadRule::AllowanceWithBracket,
        );
    }
    for (i, entry) in condition.any.iter().enumerate() {
        allowance_with_bracket(entry, &format!("{path}.any[{i}]"), out);
    }
    if let Some(not) = &condition.not {
        allowance_with_bracket(not, &format!("{path}.not"), out);
    }
}

/// `any_player` may name heights and positions only: whether any other
/// field is set, `not` and `any` entries included.
fn condition_is_wide(condition: &Condition) -> bool {
    !condition.tiers.is_empty()
        || !condition.systems.is_empty()
        || !condition.brackets.is_empty()
        || condition.captain.is_some()
        || !condition.playing_styles.is_empty()
        || !condition.holds.is_empty()
        || !condition.rated_a_at.is_empty()
        || !condition.rated_b_at.is_empty()
        || !condition.name_colors.is_empty()
        || condition.fielded_only_at.is_some()
        || !condition.ranges.is_empty()
        || condition.strong_foot.is_some()
        || condition.any.iter().any(condition_is_wide)
        || condition.not.as_deref().is_some_and(condition_is_wide)
}

/// Whether a special's `then` or any `choice` option holds
/// `height_allowance`.
fn effects_have_allowance(effects: &Effects) -> bool {
    effects.height_allowance.is_some() || effects.choice.iter().any(effects_have_allowance)
}
