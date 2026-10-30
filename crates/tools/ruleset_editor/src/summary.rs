//! `ruleset` as readable rules text (Markdown): the Summary section's
//! text and `ruleset-editor summary`'s output. An organizer copies it to
//! post an invitational's rules, and two summaries diffed in any text tool
//! compare two rulesets. The wording is the draft's,
//! `docs/prototypes/tools/ruleset_editor.js` "Readable rules".

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Display;

use aatf::{
    BracketHeights, Condition, Effects, PerTier, Ruleset, Severity, StatOrAll, Tier, TierId,
};
use pes_savefile::interchange::team_toml::labels::card_name;

/// A section's opening lines: a blank line and its `##` head.
fn section(title: &str) -> Vec<String> {
    vec![String::new(), format!("## {title}")]
}

/// `ruleset` as readable rules text, Markdown: the draft's wording, ending
/// with one newline.
pub(crate) fn summary(ruleset: &Ruleset) -> String {
    let mut lines = header_lines(ruleset);
    lines.extend(tier_lines(ruleset));
    lines.extend(height_lines(ruleset));
    lines.extend(every_team_lines(ruleset));
    lines.extend(card_group_lines(ruleset));
    lines.extend(team_setting_lines(ruleset));
    lines.extend(special_lines(ruleset));
    lines.extend(suggestion_lines(ruleset));
    format!("{}\n", lines.join("\n"))
}

/// The header: the ruleset's name as the title, the rating and at-most
/// stats.
fn header_lines(ruleset: &Ruleset) -> Vec<String> {
    let mut lines = vec![
        format!("# {}", ruleset.name),
        String::new(),
        format!(
            "A player's rating is his best of: {}.",
            ruleset
                .stats
                .rating
                .iter()
                .map(|stat| words(stat.label()))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ];
    if !ruleset.stats.at_most.is_empty() {
        lines.push(format!(
            "May sit below their target: {}.",
            ruleset
                .stats
                .at_most
                .iter()
                .map(|stat| words(stat))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    lines
}

/// The `## Tiers` section: one line per tier, highest first, then the
/// medal-systems line when the ruleset has them.
fn tier_lines(ruleset: &Ruleset) -> Vec<String> {
    let mut lines = section("Tiers");
    let medal_tiers: BTreeSet<&str> = ruleset
        .medal_systems
        .iter()
        .flat_map(|system| system.counts.keys().map(String::as_str))
        .collect();
    for tier in &ruleset.tiers {
        lines.push(tier_line(ruleset, &medal_tiers, tier));
    }
    if !ruleset.medal_systems.is_empty() {
        let named: Vec<&Tier> = ruleset
            .tiers
            .iter()
            .filter(|tier| medal_tiers.contains(tier.id.as_str()))
            .collect();
        let systems: Vec<String> = ruleset
            .medal_systems
            .iter()
            .map(|system| {
                let counts: Vec<String> = named
                    .iter()
                    .map(|tier| {
                        format!(
                            "{} {}",
                            system.counts.get(&tier.id).copied().unwrap_or(0),
                            tier.label
                        )
                    })
                    .collect();
                format!("{} ({})", system.label, counts.join(", "))
            })
            .collect();
        lines.push(format!(
            "Medal systems: {}.",
            list(&systems, |text| text.clone(), "or")
        ));
    }
    lines
}

/// The `## Heights` section: the `Brackets:` line and one line per
/// system; empty when the ruleset names no brackets.
fn height_lines(ruleset: &Ruleset) -> Vec<String> {
    if ruleset.heights.brackets.is_empty() {
        return Vec::new();
    }
    let mut lines = section("Heights");
    lines.push(brackets_text(ruleset));
    for (index, system) in ruleset.heights.systems.iter().enumerate() {
        let when = system
            .any_player
            .as_ref()
            .map(|condition| cond_text(ruleset, condition))
            .unwrap_or_default();
        let mut parts = Vec::new();
        if !when.is_empty() {
            parts.push(format!("any player is {when}"));
        }
        if system.by_quotas {
            parts.push("the team meets its quotas".to_string());
        }
        let condition = if !parts.is_empty() {
            format!("(when {})", parts.join(" and "))
        } else if index > 0 {
            "(otherwise)".to_string()
        } else {
            String::new()
        };
        let quotas: Vec<String> = ruleset
            .heights
            .brackets
            .iter()
            .filter(|bracket| system.quotas.contains_key(&bracket.id))
            .map(|bracket| {
                format!(
                    "{} {}",
                    system.quotas[&bracket.id],
                    bracket.label.to_lowercase()
                )
            })
            .collect();
        let extra_height = match &system.extra_height {
            Some(extra) => format!(
                "; {} cm to share, no player above {}",
                extra.total, extra.max
            ),
            None => String::new(),
        };
        lines.push(format!(
            "- **{}** {}: {}{extra_height}.",
            system.label,
            condition,
            quotas.join(", ")
        ));
    }
    lines
}

/// The `Brackets:` line: `Label up to N`, `Label lo-N` and
/// `Label n or m` brackets in order; an empty `exactly` reads as the
/// label alone, and the next `up_to` starts after the last height an
/// earlier bracket named.
fn brackets_text(ruleset: &Ruleset) -> String {
    let mut previous = 0u16;
    let brackets: Vec<String> = ruleset
        .heights
        .brackets
        .iter()
        .map(|bracket| {
            let text = match &bracket.heights {
                BracketHeights::UpTo(up_to) if previous == 0 => {
                    format!("{} up to {up_to}", bracket.label)
                }
                BracketHeights::UpTo(up_to) => {
                    format!("{} {}-{up_to}", bracket.label, previous + 1)
                }
                BracketHeights::Exactly(exactly) if exactly.is_empty() => bracket.label.clone(),
                BracketHeights::Exactly(exactly) => format!(
                    "{} {}",
                    bracket.label,
                    exactly
                        .iter()
                        .map(u8::to_string)
                        .collect::<Vec<_>>()
                        .join(" or ")
                ),
            };
            // An empty `exactly` names no height: `previous` stands.
            previous = match &bracket.heights {
                BracketHeights::UpTo(up_to) => u16::from(*up_to),
                BracketHeights::Exactly(exactly) => {
                    exactly.iter().max().map_or(previous, |max| u16::from(*max))
                }
            };
            text
        })
        .collect();
    format!("Brackets: {} cm.", brackets.join(", "))
}

/// The `## Every team` section: the universal rules as one
/// semicolon-joined sentence.
fn every_team_lines(ruleset: &Ruleset) -> Vec<String> {
    let universal = &ruleset.universal;
    let mut every: Vec<String> = Vec::new();
    if universal.registered_position_a {
        every.push("every player is rated A at his registered position".to_string());
    }
    if !universal.fielded_at_a.is_empty() {
        every.push(format!(
            "starters play only where rated A in preset {}",
            universal
                .fielded_at_a
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if universal.no_b_ratings {
        every.push("no B ratings".to_string());
    }
    if universal.b_uses_a_allowance {
        every.push("B ratings count as A positions".to_string());
    }
    if universal.gk_not_second_a {
        every.push("GK is never a second A".to_string());
    }
    if let Some((low, high)) = universal.age {
        every.push(format!("age {low} to {high}"));
    }
    if let Some(weight) = &universal.weight {
        every.push(format!(
            "weight between height \u{2212} {} (at least {}) and height \u{2212} {}",
            weight.below, weight.floor, weight.above
        ));
    }
    if universal.captain_required {
        every.push("a captain".to_string());
    }
    if universal.min_goalkeepers > 0 {
        every.push(format!(
            "at least {} goalkeeper{}",
            universal.min_goalkeepers,
            if universal.min_goalkeepers > 1 {
                "s"
            } else {
                ""
            }
        ));
    }
    every.push(format!("at most {} skill cards", universal.max_skill_cards));
    if let Some(max) = universal.max_com_styles {
        every.push(format!("at most {max} COM styles"));
    }
    let every = every.join("; ");
    let mut lines = section("Every team");
    lines.push(format!("{}.", upper_first(&every)));
    lines
}

/// The `## Card groups` section, one line per group; empty without
/// groups.
fn card_group_lines(ruleset: &Ruleset) -> Vec<String> {
    if ruleset.card_groups.is_empty() {
        return Vec::new();
    }
    let mut lines = section("Card groups");
    for (group, cards) in &ruleset.card_groups {
        let cards: Vec<String> = cards.iter().map(|card| card_name(card.label())).collect();
        lines.push(format!("- **{}**: {}.", human(group), cards.join(", ")));
    }
    lines
}

/// The `## Team settings` section, one line per entry; empty without
/// entries.
fn team_setting_lines(ruleset: &Ruleset) -> Vec<String> {
    if ruleset.team_settings.is_empty() {
        return Vec::new();
    }
    let mut lines = section("Team settings");
    for settings in &ruleset.team_settings {
        let mut parts = Vec::new();
        for (value, name) in [
            (settings.man_marking, "man marking"),
            (settings.auto_substitution, "automatic substitution"),
            (settings.auto_offside_trap, "automatic offside trap"),
            (settings.auto_preset_change, "automatic preset change"),
            (
                settings.auto_attack_defence_levels,
                "automatic attack/defence levels",
            ),
        ] {
            if let Some(on) = value {
                parts.push(format!("{name} {}", if on { "on" } else { "off" }));
            }
        }
        if !settings.forbidden_shapes.is_empty() {
            let shapes: Vec<String> = settings
                .forbidden_shapes
                .iter()
                .map(|shape| {
                    shape
                        .iter()
                        .map(u8::to_string)
                        .collect::<Vec<_>>()
                        .join("-")
                })
                .collect();
            parts.push(format!("no {} formations", shapes.join(" or ")));
        }
        let versions = if settings.versions.is_empty() {
            String::new()
        } else {
            format!(
                " (PES {})",
                settings
                    .versions
                    .iter()
                    .map(|version| version.number().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        lines.push(format!(
            "- **{}**{}: {}.",
            settings.label,
            versions,
            parts.join(", ")
        ));
    }
    lines
}

/// The `## Special rules` section, one line per special in order; empty
/// without specials.
fn special_lines(ruleset: &Ruleset) -> Vec<String> {
    if ruleset.specials.is_empty() {
        return Vec::new();
    }
    let mut lines = section("Special rules");
    for special in &ruleset.specials {
        let when = cond_text(ruleset, &special.when);
        let when = if when.is_empty() {
            String::new()
        } else {
            format!("{}: ", upper_first(&when))
        };
        let severity = match special.severity {
            Severity::Error => "",
            Severity::Warning => " (warning)",
            Severity::Suggestion => " (suggestion)",
        };
        lines.push(format!(
            "- **{}**{}: {}{}.",
            special.label,
            severity,
            when,
            effects_text(ruleset, &special.then)
        ));
    }
    lines
}

/// The `## Suggestions` section: the allowances a team has not used up,
/// which the check lists as suggestions; empty without any.
fn suggestion_lines(ruleset: &Ruleset) -> Vec<String> {
    if ruleset.suggestions.is_empty() {
        return Vec::new();
    }
    let mut lines = section("Suggestions");
    lines.push(format!(
        "Allowances not used up, listed as suggestions: {}.",
        list(&ruleset.suggestions, |entry| suggestion_text(entry), "and")
    ));
    lines
}

/// A suggestions entry as words: the built-in allowances by name, any
/// other entry (a card group's id) as its group's free cards.
fn suggestion_text(entry: &str) -> String {
    match entry {
        "cards" => "paid cards".to_string(),
        "a_positions" => "A positions".to_string(),
        "weak_foot" => "weak foot".to_string(),
        "injury_resistance" => "injury resistance".to_string(),
        "height" => "height".to_string(),
        "free_cards" => "free cards".to_string(),
        "choices" => "choices not taken".to_string(),
        "com" => "free COM styles".to_string(),
        group => format!("free {}", words(group)),
    }
}

/// One tier's line (`- **Gold**: 1 per team; every stat 99; …`).
fn tier_line(ruleset: &Ruleset, medal_tiers: &BTreeSet<&str>, tier: &Tier) -> String {
    let count = if medal_tiers.contains(tier.id.as_str()) {
        "by medal system".to_string()
    } else if let Some(counts_as) = tier.counts_as.as_deref().filter(|id| !id.is_empty()) {
        format!("counts as {}", tier_label(ruleset, counts_as))
    } else {
        match tier.count {
            None => "the rest of the squad".to_string(),
            Some(count) => format!("{count} per team"),
        }
    };
    let when = cond_text(ruleset, &tier.when);
    let when = if when.is_empty() {
        String::new()
    } else {
        format!(", {when}")
    };
    let stats: Vec<String> = tier
        .stats
        .iter()
        .map(|(stat, target)| format!("{} {target}", words(stat.label())))
        .collect();
    let stats = if stats.is_empty() {
        String::new()
    } else {
        format!(" ({})", stats.join(", "))
    };
    let band = if tier.stat_warning == 0 {
        String::new()
    } else {
        format!(
            "; an ability stat that may sit below its target warns up to {} over it, past that it is not allowed",
            tier.stat_warning
        )
    };
    // "Those stats" names the band's; without a band the clause names
    // them itself, or it would read as every stat.
    let total = match (tier.total_within_targets, tier.stat_warning) {
        (false, _) => "",
        (true, 0) => {
            "; the ability stats that may sit below their target added up may not exceed their targets' sum"
        }
        (true, _) => "; those stats added up may not exceed their targets' sum",
    };
    let min_cards = if tier.min_cards == 0 {
        String::new()
    } else {
        format!(
            " (at least {} paid skill{}, an extra A position counting as one)",
            tier.min_cards,
            if tier.min_cards == 1 { "" } else { "s" }
        )
    };
    let free: Vec<String> = tier
        .free
        .iter()
        .map(|(group, count)| {
            // Only COM styles take a plural: a group's own name is the
            // organizer's word for its cards, as the draft spells it.
            let plural = if group == "com" && *count > 1 {
                "s"
            } else {
                ""
            };
            format!("{count} free {}{plural}", group_label(group))
        })
        .collect();
    let free = if free.is_empty() {
        String::new()
    } else {
        format!(", {}", free.join(", "))
    };
    let name_color = match tier.name_color.as_deref().filter(|color| !color.is_empty()) {
        Some(color) => format!("; name in colour {color}"),
        None => String::new(),
    };
    format!(
        "- **{}**: {}{when}; every stat {}{stats}{band}{total}; form {}, injury resistance {}, weak foot {}/{}; {} paid cards{}{}; {} A position{}{name_color}.",
        tier.label,
        count,
        tier.rate,
        tier.form,
        tier.injury_resistance,
        tier.weak_foot.usage,
        tier.weak_foot.accuracy,
        tier.cards,
        min_cards,
        free,
        tier.a_positions,
        if tier.a_positions == 1 { "" } else { "s" }
    )
}

/// `condition` as words (`registered GK, 181 to 188 cm`), empty when it
/// holds for every player.
fn cond_text(ruleset: &Ruleset, condition: &Condition) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.extend(listed(&condition.tiers, |id| tier_label(ruleset, id), "or"));
    parts.extend(
        listed(
            &condition.positions,
            |position| position.label().to_string(),
            "or",
        )
        .map(|text| format!("registered {text}")),
    );
    parts.extend(
        listed(
            &condition.systems,
            |id| {
                label_or_id(&ruleset.heights.systems, id, |system| {
                    (system.id.as_str(), system.label.as_str())
                })
            },
            "or",
        )
        .map(|text| format!("under {text}")),
    );
    parts.extend(
        listed(
            &condition.brackets,
            |id| {
                label_or_id(&ruleset.heights.brackets, id, |bracket| {
                    (bracket.id.as_str(), bracket.label.as_str())
                })
                .to_lowercase()
            },
            "or",
        )
        .map(|text| format!("{text}s")),
    );
    match (condition.height_min, condition.height_max) {
        (Some(min), Some(max)) if min == max => parts.push(format!("{min} cm")),
        (Some(min), Some(max)) => parts.push(format!("{min} to {max} cm")),
        (Some(min), None) => parts.push(format!("{min} cm or more")),
        (None, Some(max)) => parts.push(format!("{max} cm or less")),
        (None, None) => {}
    }
    match condition.captain {
        Some(true) => parts.push("the captain".to_string()),
        Some(false) => parts.push("not the captain".to_string()),
        None => {}
    }
    parts.extend(
        listed(
            &condition.playing_styles,
            |style| card_name(style.label()),
            "or",
        )
        .map(|text| format!("playing as {text}")),
    );
    parts.extend(
        listed(&condition.holds, |card| card_name(card.label()), "or")
            .map(|text| format!("holding {text}")),
    );
    parts.extend(
        listed(
            &condition.rated_a_at,
            |position| position.label().to_string(),
            "or",
        )
        .map(|text| format!("rated A at {text}")),
    );
    parts.extend(
        listed(
            &condition.rated_b_at,
            |position| position.label().to_string(),
            "or",
        )
        .map(|text| format!("rated B at {text}")),
    );
    parts.extend(
        listed(&condition.name_colors, |color| color.clone(), "or")
            .map(|text| format!("named in colour {text}")),
    );
    if let Some(position) = condition.fielded_only_at {
        parts.push(format!("fielded only at {}", position.label()));
    }
    for (value, (low, high)) in &condition.ranges {
        parts.push(format!("{} {low} to {high}", words(value.key())));
    }
    if let Some(foot) = condition.strong_foot {
        parts.push(format!("{}-footed", foot.label()));
    }
    if !condition.any.is_empty() {
        let cases: Vec<String> = condition
            .any
            .iter()
            .map(|case| cond_or_anyone(ruleset, case))
            .collect();
        parts.push(format!("any of ({})", cases.join("; ")));
    }
    if let Some(not) = &condition.not {
        parts.push(format!("except {}", cond_or_anyone(ruleset, not)));
    }
    parts.join(", ")
}

/// `condition` as words, `anyone` when it holds for every player (an
/// empty `any` case, an empty `not`).
fn cond_or_anyone(ruleset: &Ruleset, condition: &Condition) -> String {
    let text = cond_text(ruleset, condition);
    if text.is_empty() {
        "anyone".to_string()
    } else {
        text
    }
}

/// A `PerTier` value as the draft spells it: `+5` for one number, `+2
/// for Silver, +5 for Regular` by tier (the tiers' order, not the map's).
fn per_tier_text<T>(ruleset: &Ruleset, value: &PerTier<T>, unit: &str) -> String
where
    T: Copy + Into<i64> + Display,
{
    let sign = |n: T| if n.into() > 0 { "+" } else { "" };
    per_tier_with(ruleset, value, |n| format!("{}{n}{unit}", sign(*n)))
}

/// `value` as the draft spells it: `number(n)` for one number,
/// `number(n) for <tier>` joined `, ` in the tiers' order for `ByTier`.
fn per_tier_with<T>(
    ruleset: &Ruleset,
    value: &PerTier<T>,
    number: impl Fn(&T) -> String,
) -> String {
    match value {
        PerTier::All(n) => number(n),
        PerTier::ByTier(map) => by_tier(ruleset, map)
            .into_iter()
            .map(|(id, n)| format!("{} for {}", number(n), tier_label(ruleset, id)))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// A per-tier map's entries in the tiers' order, highest first (a tier
/// id the ruleset does not have sorts first, as the draft's
/// `findIndex` `-1` did).
fn by_tier<'a, T>(ruleset: &Ruleset, map: &'a BTreeMap<TierId, T>) -> Vec<(&'a TierId, &'a T)> {
    let mut entries: Vec<(&TierId, &T)> = map.iter().collect();
    entries.sort_by_key(|(id, _)| ruleset.tiers.iter().position(|tier| &tier.id == *id));
    entries
}

/// `effects` as the draft's `effectsText` writes them, one list entry's
/// worth.
fn effects_text(ruleset: &Ruleset, effects: &Effects) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (key, value) in &effects.stat_bonus {
        let stat = match key {
            StatOrAll::All => "every stat".to_string(),
            StatOrAll::Stat(stat) => words(stat.label()),
        };
        parts.push(format!("{} to {stat}", per_tier_text(ruleset, value, "")));
    }
    for (stat, value) in &effects.stat_target {
        let target = per_tier_with(ruleset, value, ToString::to_string);
        parts.push(format!("{} target {target}", words(stat.label())));
    }
    if let Some(cards) = &effects.cards {
        // The draft pluralizes on the value being the number 1 itself:
        // a per-tier map is always "cards".
        let plural = if matches!(cards, PerTier::All(1)) {
            ""
        } else {
            "s"
        };
        parts.push(format!(
            "{} paid card{plural}",
            per_tier_text(ruleset, cards, "")
        ));
    }
    for (group, value) in &effects.free {
        parts.push(format!(
            "{} free {}",
            per_tier_text(ruleset, value, ""),
            group_label(group)
        ));
    }
    parts.extend(
        listed(&effects.free_cards, |card| card_name(card.label()), "and")
            .map(|text| format!("{text} free")),
    );
    parts.extend(
        listed(
            &effects.require_cards,
            |card| card_name(card.label()),
            "and",
        )
        .map(|text| format!("must hold {text}")),
    );
    if let Some(a_positions) = &effects.a_positions {
        parts.push(format!(
            "{} A position",
            per_tier_text(ruleset, a_positions, "")
        ));
    }
    if let Some(max) = effects.max_a_positions {
        parts.push(format!(
            "at most {max} A position{}",
            if max == 1 { "" } else { "s" }
        ));
    }
    if let Some(form) = &effects.form {
        parts.push(format!("{} form", per_tier_text(ruleset, form, "")));
    }
    if let Some(resistance) = &effects.injury_resistance {
        parts.push(format!(
            "{} injury resistance",
            per_tier_text(ruleset, resistance, "")
        ));
    }
    if let Some(weak_foot) = &effects.weak_foot {
        parts.push(format!(
            "weak foot {}/{}",
            weak_foot.usage, weak_foot.accuracy
        ));
    }
    if let Some(allowance) = effects.height_allowance {
        parts.push(format!("count {allowance} cm shorter"));
    }
    if let Some(stat_warning) = &effects.stat_warning {
        let band = per_tier_with(ruleset, stat_warning, ToString::to_string);
        parts.push(format!("warning band {band}"));
    }
    if let Some(forbid) = &effects.forbid {
        parts.push(format!("not allowed (\"{forbid}\")"));
    }
    match effects.choice.as_slice() {
        [] => {}
        [option] => parts.push(format!("may get {}", effects_text(ruleset, option))),
        options => {
            let options: Vec<String> = options
                .iter()
                .map(|option| effects_text(ruleset, option))
                .collect();
            parts.push(format!(
                "gets the first needed of: {}",
                options.join("; or ")
            ));
        }
    }
    parts.join(", ")
}

/// `id`'s label on the item of `items` holding it (`id_label` gives an
/// item's `(id, label)`), or the id itself when none does.
fn label_or_id<'a, T>(
    items: &'a [T],
    id: &str,
    id_label: impl Fn(&'a T) -> (&'a str, &'a str),
) -> String {
    items
        .iter()
        .find(|item| id_label(*item).0 == id)
        .map(|item| id_label(item).1.to_string())
        .unwrap_or_else(|| id.to_string())
}

/// `id`'s tier label, or the id itself when no tier has it.
fn tier_label(ruleset: &Ruleset, id: &str) -> String {
    label_or_id(&ruleset.tiers, id, |tier| {
        (tier.id.as_str(), tier.label.as_str())
    })
}

/// `id` as titled words (`ball_winning` → `Ball Winning`, `gk` → `GK`,
/// `com` → `COM`).
pub(crate) fn human(id: &str) -> String {
    id.split('_')
        .map(|word| match word {
            "gk" | "com" => word.to_uppercase(),
            _ => upper_first(word),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A card group's name in a sentence: the built-in `com` group is
/// "COM style", any other its id as words.
fn group_label(group: &str) -> String {
    if group == "com" {
        "COM style".to_string()
    } else {
        words(group)
    }
}

/// `id` as lowercase words (`attacking_prowess` → `attacking prowess`).
fn words(id: &str) -> String {
    human(id).to_lowercase()
}

/// `text` with its first letter capitalized (the start of a sentence).
fn upper_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// `items` joined by `list` with `word`, `None` when it holds none (a
/// caller phrases the text around it only when it exists).
fn listed<T>(items: &[T], label: impl Fn(&T) -> String, word: &str) -> Option<String> {
    if items.is_empty() {
        None
    } else {
        Some(list(items, label, word))
    }
}

/// `items` joined as a list (`a`, `a or b`, `a, b or c` for `word`
/// "or"), `label` applied to each. Alternatives (a condition's
/// tiers, the medal systems) join with "or", things a player gets
/// all of (an effect's cards, the suggestions) with "and".
fn list<T>(items: &[T], label: impl Fn(&T) -> String, word: &str) -> String {
    match items.split_last() {
        None => String::new(),
        Some((last, [])) => label(last),
        Some((last, rest)) => format!(
            "{} {word} {}",
            rest.iter().map(&label).collect::<Vec<_>>().join(", "),
            label(last)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `human` titles each word and upper-cases `gk` and `com`.
    #[test]
    fn human_titles_words_and_uppercases_gk_and_com() {
        assert_eq!(human("ball_winning"), "Ball Winning");
        assert_eq!(human("gk"), "GK");
        assert_eq!(human("com"), "COM");
    }

    /// `list` joins none, one, two and three items with its word.
    #[test]
    fn list_joins_with_its_word() {
        let label = |s: &&str| s.to_string();
        assert_eq!(list::<&str>(&[], label, "or"), "");
        assert_eq!(list(&["a"], label, "or"), "a");
        assert_eq!(list(&["a", "b"], label, "or"), "a or b");
        assert_eq!(list(&["a", "b", "c"], label, "or"), "a, b or c");
        assert_eq!(list(&["a"], label, "and"), "a");
        assert_eq!(list(&["a", "b"], label, "and"), "a and b");
        assert_eq!(list(&["a", "b", "c"], label, "and"), "a, b and c");
    }

    /// An `exactly` bracket emptied mid-edit reads as its label alone.
    #[test]
    fn an_empty_exactly_bracket_reads_as_its_label() {
        let mut ruleset =
            Ruleset::from_data_block(include_str!("../../../libs/aatf/rulesets/autumn26.rhai"))
                .expect("the embedded ruleset loads");
        ruleset.heights.brackets[1].heights = BracketHeights::Exactly(Vec::new());
        let text = summary(&ruleset);
        let line = text
            .lines()
            .find(|line| line.starts_with("Brackets:"))
            .expect("the Brackets line");
        assert_eq!(
            line,
            "Brackets: Manlet up to 175, Mid, Tall 185, Giant 194, Giga 199 cm."
        );
    }

    /// A per-tier stat target is words in the tiers' order, not JSON.
    #[test]
    fn a_per_tier_stat_target_reads_as_words_in_tier_order() {
        let ruleset =
            Ruleset::from_data_block(include_str!("../../../libs/aatf/rulesets/autumn26.rhai"))
                .expect("the embedded ruleset loads");
        let mut effects = Effects::default();
        effects.stat_target.insert(
            aatf::Stat::try_from("finishing".to_string()).expect("a stat"),
            PerTier::ByTier(std::collections::BTreeMap::from([
                ("silver".to_string(), 90),
                ("gold".to_string(), 95),
            ])),
        );
        assert_eq!(
            effects_text(&ruleset, &effects),
            "finishing target 95 for Gold, 90 for Silver"
        );
    }

    /// A card a condition holds or an effect requires reads by its card
    /// name, `&` for and.
    #[test]
    fn a_held_or_required_card_reads_by_its_card_name() {
        let ruleset =
            Ruleset::from_data_block(include_str!("../../../libs/aatf/rulesets/autumn26.rhai"))
                .expect("the embedded ruleset loads");
        let card = aatf::Card::try_from("cut_behind_and_turn".to_string()).expect("a card");
        let condition = Condition {
            holds: vec![card],
            ..Condition::default()
        };
        assert_eq!(cond_text(&ruleset, &condition), "holding Cut Behind & Turn");
        let effects = Effects {
            require_cards: vec![card],
            ..Effects::default()
        };
        assert_eq!(
            effects_text(&ruleset, &effects),
            "must hold Cut Behind & Turn"
        );
    }

    /// A bracket a condition names reads by its label in lowercase, as a
    /// plural.
    #[test]
    fn a_condition_names_its_bracket_in_lowercase() {
        let ruleset =
            Ruleset::from_data_block(include_str!("../../../libs/aatf/rulesets/autumn26.rhai"))
                .expect("the embedded ruleset loads");
        assert!(
            ruleset
                .heights
                .brackets
                .iter()
                .any(|bracket| bracket.id == "giant" && bracket.label == "Giant"),
            "premise: Autumn 26's giant bracket is labeled Giant"
        );
        let condition = Condition {
            brackets: vec!["giant".to_string()],
            ..Condition::default()
        };
        assert_eq!(cond_text(&ruleset, &condition), "giants");
    }

    /// A per-tier number reads with its sign: `+` above zero, none at zero
    /// or below (the minus is the number's own).
    #[test]
    fn a_per_tier_number_reads_with_its_sign() {
        let ruleset =
            Ruleset::from_data_block(include_str!("../../../libs/aatf/rulesets/autumn26.rhai"))
                .expect("the embedded ruleset loads");
        assert_eq!(per_tier_text(&ruleset, &PerTier::All(1i8), ""), "+1");
        assert_eq!(per_tier_text(&ruleset, &PerTier::All(0i8), ""), "0");
        assert_eq!(per_tier_text(&ruleset, &PerTier::All(-3i8), ""), "-3");
    }

    /// The suggestions are part of the summary: emptying them changes it,
    /// and an empty list draws no `## Suggestions` section.
    #[test]
    fn the_suggestions_have_a_section_only_when_set() {
        let ruleset =
            Ruleset::from_data_block(include_str!("../../../libs/aatf/rulesets/autumn26.rhai"))
                .expect("the embedded ruleset loads");
        let mut without = ruleset.clone();
        without.suggestions.clear();
        let with_text = summary(&ruleset);
        let without_text = summary(&without);
        assert_ne!(
            with_text, without_text,
            "the suggestions change the summary"
        );
        assert!(with_text.contains("\n## Suggestions\n"), "{with_text}");
        assert!(!without_text.contains("## Suggestions"), "{without_text}");
    }

    /// The stat total without a band names the stats it covers itself,
    /// and a minimum of one reads in the singular.
    #[test]
    fn a_total_without_a_band_names_its_stats() {
        let mut ruleset =
            Ruleset::from_data_block(include_str!("../../../libs/aatf/rulesets/autumn26.rhai"))
                .expect("the embedded ruleset loads");
        let gold = &mut ruleset.tiers[0];
        assert_eq!(gold.stat_warning, 0, "premise: Gold has no band");
        gold.total_within_targets = true;
        gold.min_cards = 1;
        let line = tier_line(&ruleset, &BTreeSet::new(), &ruleset.tiers[0]);
        assert!(
            line.contains(
                "; the ability stats that may sit below their target added up may not exceed their \
                 targets' sum"
            ),
            "{line}"
        );
        assert!(
            line.contains("(at least 1 paid skill, an extra A position"),
            "{line}"
        );
    }

    /// Every embedded ruleset's summary is its golden
    /// (`tests/fixtures/summary/`), byte for byte.
    #[test]
    fn every_embedded_ruleset_summarizes_to_its_golden() {
        for (name, block, golden) in [
            (
                "autumn25",
                include_str!("../../../libs/aatf/rulesets/autumn25.rhai"),
                include_str!("../tests/fixtures/summary/autumn25.md"),
            ),
            (
                "autumn26",
                include_str!("../../../libs/aatf/rulesets/autumn26.rhai"),
                include_str!("../tests/fixtures/summary/autumn26.md"),
            ),
            (
                "dbc7",
                include_str!("../../../libs/aatf/rulesets/dbc7.rhai"),
                include_str!("../tests/fixtures/summary/dbc7.md"),
            ),
            (
                "spring26",
                include_str!("../../../libs/aatf/rulesets/spring26.rhai"),
                include_str!("../tests/fixtures/summary/spring26.md"),
            ),
            (
                "vgl25",
                include_str!("../../../libs/aatf/rulesets/vgl25.rhai"),
                include_str!("../tests/fixtures/summary/vgl25.md"),
            ),
            (
                "vgl26",
                include_str!("../../../libs/aatf/rulesets/vgl26.rhai"),
                include_str!("../tests/fixtures/summary/vgl26.md"),
            ),
            (
                "vgl27",
                include_str!("../../../libs/aatf/rulesets/vgl27.rhai"),
                include_str!("../tests/fixtures/summary/vgl27.md"),
            ),
            (
                "vtl10",
                include_str!("../../../libs/aatf/rulesets/vtl10.rhai"),
                include_str!("../tests/fixtures/summary/vtl10.md"),
            ),
            (
                "vtl11",
                include_str!("../../../libs/aatf/rulesets/vtl11.rhai"),
                include_str!("../tests/fixtures/summary/vtl11.md"),
            ),
        ] {
            let ruleset = Ruleset::from_data_block(block).expect("the embedded ruleset loads");
            let actual = summary(&ruleset);
            if actual == golden {
                continue;
            }
            let expected_lines: Vec<&str> = golden.lines().collect();
            let actual_lines: Vec<&str> = actual.lines().collect();
            for (index, (expected, actual)) in expected_lines.iter().zip(&actual_lines).enumerate()
            {
                assert_eq!(actual, expected, "{name}: line {} differs", index + 1);
            }
            panic!(
                "{name}: no shared line differs; {}/{} lines, {}/{} bytes",
                expected_lines.len(),
                actual_lines.len(),
                golden.len(),
                actual.len()
            );
        }
    }
}
