//! `Ruleset` written as its `RULESET` data block: the Ruleset editor's
//! save and preview text. The block reads back as an equal `Ruleset`,
//! and writing twice gives identical bytes.

use std::fmt;

use crate::ruleset::{
    Bracket, BracketHeights, Condition, Effects, ExtraHeight, HeightSystem, Heights, MedalSystem,
    PerTier, Ruleset, Severity, Special, StatOrAll, StatRules, TeamSettings, Tier, Universal,
    WeakFoot, WeightRule,
};

/// The first line of every rules file, as `rulesets/` carries it.
const HEADER: &str = "// AATF ruleset, a Rhai script. Open it in the Ruleset editor to change it.";

/// The indent unit, four spaces.
const INDENT: &str = "    ";
/// The list items' and map entries' indent.
const I2: &str = "        ";
/// The height lists' indent, one deeper.
const I3: &str = "            ";
/// The wrap width, in columns.
const WIDTH: usize = 100;

/// `key: value`.
fn field(key: impl fmt::Display, value: impl fmt::Display) -> String {
    format!("{key}: {value}")
}

/// `text` as a Rhai string literal: `"` and `\` escaped, every
/// character below U+0020 escaped (`\n`, `\t`, `\r`, else `\xHH`, the
/// escapes rhai's tokenizer reads), everything else written as it is.
fn string_literal(text: &str) -> String {
    let mut literal = String::with_capacity(text.len() + 2);
    literal.push('"');
    for ch in text.chars() {
        match ch {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\t' => literal.push_str("\\t"),
            '\r' => literal.push_str("\\r"),
            ch if ch < ' ' => literal.push_str(&format!("\\x{:02x}", u32::from(ch))),
            ch => literal.push(ch),
        }
    }
    literal.push('"');
    literal
}

/// `key` as a map-literal key: bare when Rhai's parser takes it as an
/// identifier, a string literal else (an id is any non-empty string).
/// Bare needs both (a) the identifier shape with at least one letter
/// (the parser alone would take a key like `a: 0, b`) and (b) the
/// parser accepting `#{ <key>: 0 }`, which refuses keywords and
/// reserved words (`in`, `go`, `match`): rhai's identifier check and
/// keyword table are behind its unstable `internals` feature, and a
/// copied list would drift with rhai upgrades.
fn map_key(key: &str) -> String {
    let shaped = key.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && key.chars().any(|ch| ch.is_ascii_alphabetic());
    if shaped
        && rhai::Engine::new_raw()
            .compile_expression(format!("#{{ {key}: 0 }}"))
            .is_ok()
    {
        key.to_string()
    } else {
        string_literal(key)
    }
}

/// `#{ a: 1, b: 2 }` of the `key: value` pairs, `#{}` when empty.
fn rhai_map(fields: impl IntoIterator<Item = String>) -> String {
    let fields: Vec<String> = fields.into_iter().collect();
    if fields.is_empty() {
        "#{}".to_string()
    } else {
        format!("#{{ {} }}", fields.join(", "))
    }
}

/// `[a, b]` of the items, `[]` when empty.
fn rhai_list(items: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", items.into_iter().collect::<Vec<_>>().join(", "))
}

/// `[1, 2]` of the numbers.
fn number_list(numbers: &[u8]) -> String {
    rhai_list(numbers.iter().map(u8::to_string))
}

/// `#{ id: n }` of the counts: a tier's free cards, a medal system's counts, a height
/// system's quotas, a per-tier value's `ByTier` map.
fn id_counts<T: fmt::Display>(counts: &std::collections::BTreeMap<String, T>) -> String {
    rhai_map(counts.iter().map(|(id, count)| field(map_key(id), count)))
}

/// `["a", "b"]` of the items as string literals.
fn quoted_list<'a>(items: impl IntoIterator<Item = &'a str>) -> String {
    rhai_list(items.into_iter().map(string_literal))
}

/// `key: render(value)` pushed unless `value` equals its default: a
/// field the reader defaults is left out, so a block lists only what
/// the ruleset sets.
fn push_set<T: Default + PartialEq>(
    fields: &mut Vec<String>,
    key: &str,
    value: &T,
    render: impl FnOnce(&T) -> String,
) {
    if *value != T::default() {
        fields.push(field(key, render(value)));
    }
}

/// `key: render(value)` pushed when `value` is `Some`.
fn push_some<T>(
    fields: &mut Vec<String>,
    key: &str,
    value: &Option<T>,
    render: impl FnOnce(&T) -> String,
) {
    if let Some(value) = value {
        fields.push(field(key, render(value)));
    }
}

/// `All` as its number, `ByTier` as a `#{ id: n }` map.
fn per_tier<T: fmt::Display>(per_tier: &PerTier<T>) -> String {
    match per_tier {
        PerTier::All(value) => value.to_string(),
        PerTier::ByTier(by_tier) => id_counts(by_tier),
    }
}

/// `weak_foot` as `#{ usage: u, accuracy: a }` (its two fields required).
fn weak_foot_map(weak_foot: &WeakFoot) -> String {
    let WeakFoot { usage, accuracy } = weak_foot;
    rhai_map([field("usage", usage), field("accuracy", accuracy)])
}

/// `extra_height` as `#{ total: t, max: m }` (its two fields required).
fn extra_height_map(extra_height: &ExtraHeight) -> String {
    let ExtraHeight { total, max } = extra_height;
    rhai_map([field("total", total), field("max", max)])
}

/// `weight` as `#{ floor: f, below: b, above: a }` (its fields required).
fn weight_map(weight: &WeightRule) -> String {
    let WeightRule {
        floor,
        below,
        above,
    } = weight;
    rhai_map([
        field("floor", floor),
        field("below", below),
        field("above", above),
    ])
}

/// A `stat_bonus` map key: `all` or the stat's label.
fn stat_or_all_key(key: &StatOrAll) -> String {
    match key {
        StatOrAll::All => "all".to_string(),
        StatOrAll::Stat(stat) => map_key(stat.label()),
    }
}

/// `severity` as its lowercase spelling.
fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Suggestion => "suggestion",
    }
}

/// `condition` as a `#{ ... }` map literal, every non-empty field in
/// declaration order.
fn condition_map(condition: &Condition) -> String {
    let Condition {
        tiers,
        positions,
        systems,
        brackets,
        height_min,
        height_max,
        captain,
        playing_styles,
        holds,
        rated_a_at,
        rated_b_at,
        name_colors,
        fielded_only_at,
        ranges,
        strong_foot,
        any,
        not,
    } = condition;
    let mut fields: Vec<String> = Vec::new();
    push_set(&mut fields, "tiers", tiers, |ids| {
        quoted_list(ids.iter().map(String::as_str))
    });
    push_set(&mut fields, "positions", positions, |positions| {
        quoted_list(positions.iter().map(|pos| pos.label()))
    });
    push_set(&mut fields, "systems", systems, |ids| {
        quoted_list(ids.iter().map(String::as_str))
    });
    push_set(&mut fields, "brackets", brackets, |ids| {
        quoted_list(ids.iter().map(String::as_str))
    });
    push_some(&mut fields, "height_min", height_min, ToString::to_string);
    push_some(&mut fields, "height_max", height_max, ToString::to_string);
    push_some(&mut fields, "captain", captain, ToString::to_string);
    push_set(&mut fields, "playing_styles", playing_styles, |styles| {
        quoted_list(styles.iter().map(|style| style.label()))
    });
    push_set(&mut fields, "holds", holds, |cards| {
        quoted_list(cards.iter().map(|card| card.label()))
    });
    push_set(&mut fields, "rated_a_at", rated_a_at, |positions| {
        quoted_list(positions.iter().map(|pos| pos.label()))
    });
    push_set(&mut fields, "rated_b_at", rated_b_at, |positions| {
        quoted_list(positions.iter().map(|pos| pos.label()))
    });
    push_set(&mut fields, "name_colors", name_colors, |codes| {
        quoted_list(codes.iter().map(String::as_str))
    });
    push_some(&mut fields, "fielded_only_at", fielded_only_at, |pos| {
        string_literal(pos.label())
    });
    push_set(&mut fields, "ranges", ranges, |ranges| {
        rhai_map(
            ranges.iter().map(|(value, (low, high))| {
                field(map_key(value.key()), format!("[{low}, {high}]"))
            }),
        )
    });
    push_some(&mut fields, "strong_foot", strong_foot, |foot| {
        string_literal(foot.label())
    });
    push_set(&mut fields, "any", any, |cases| {
        rhai_list(cases.iter().map(condition_map))
    });
    push_some(&mut fields, "not", not, |condition| {
        condition_map(condition)
    });
    rhai_map(fields)
}

/// `effects` as a `#{ ... }` map literal, every non-empty field in
/// declaration order.
fn effects_map(effects: &Effects) -> String {
    let Effects {
        stat_bonus,
        stat_target,
        cards,
        free,
        free_cards,
        require_cards,
        a_positions,
        max_a_positions,
        form,
        injury_resistance,
        weak_foot,
        height_allowance,
        forbid,
        choice,
        stat_warning,
    } = effects;
    let mut fields: Vec<String> = Vec::new();
    push_set(&mut fields, "stat_bonus", stat_bonus, |stat_bonus| {
        rhai_map(
            stat_bonus
                .iter()
                .map(|(key, value)| field(stat_or_all_key(key), per_tier(value))),
        )
    });
    push_set(&mut fields, "stat_target", stat_target, |stat_target| {
        rhai_map(
            stat_target
                .iter()
                .map(|(stat, value)| field(map_key(stat.label()), per_tier(value))),
        )
    });
    push_some(&mut fields, "cards", cards, per_tier);
    push_set(&mut fields, "free", free, |free| {
        rhai_map(
            free.iter()
                .map(|(group, value)| field(map_key(group), per_tier(value))),
        )
    });
    push_set(&mut fields, "free_cards", free_cards, |cards| {
        quoted_list(cards.iter().map(|card| card.label()))
    });
    push_set(&mut fields, "require_cards", require_cards, |cards| {
        quoted_list(cards.iter().map(|card| card.label()))
    });
    push_some(&mut fields, "a_positions", a_positions, per_tier);
    push_some(
        &mut fields,
        "max_a_positions",
        max_a_positions,
        ToString::to_string,
    );
    push_some(&mut fields, "form", form, per_tier);
    push_some(
        &mut fields,
        "injury_resistance",
        injury_resistance,
        per_tier,
    );
    push_some(&mut fields, "weak_foot", weak_foot, weak_foot_map);
    push_some(
        &mut fields,
        "height_allowance",
        height_allowance,
        ToString::to_string,
    );
    push_some(&mut fields, "forbid", forbid, |text| string_literal(text));
    push_set(&mut fields, "choice", choice, |options| {
        rhai_list(options.iter().map(effects_map))
    });
    push_some(&mut fields, "stat_warning", stat_warning, per_tier);
    rhai_map(fields)
}

/// `tier`'s fields as `key: value` strings, in declaration order: a
/// field without `#[serde(default)]` always written, a field with one
/// left out when it equals its default.
fn tier_fields(tier: &Tier) -> Vec<String> {
    let Tier {
        id,
        label,
        count,
        counts_as,
        when,
        rate,
        stats,
        form,
        injury_resistance,
        weak_foot,
        cards,
        min_cards,
        free,
        a_positions,
        name_color,
        stat_warning,
        total_within_targets,
    } = tier;
    let mut fields = vec![
        field("id", string_literal(id)),
        field("label", string_literal(label)),
    ];
    push_some(&mut fields, "count", count, ToString::to_string);
    push_some(&mut fields, "counts_as", counts_as, |id| string_literal(id));
    push_set(&mut fields, "when", when, condition_map);
    fields.push(field("rate", rate));
    push_set(&mut fields, "stats", stats, |stats| {
        rhai_map(
            stats
                .iter()
                .map(|(stat, target)| field(map_key(stat.label()), target)),
        )
    });
    push_set(&mut fields, "form", form, ToString::to_string);
    push_set(
        &mut fields,
        "injury_resistance",
        injury_resistance,
        ToString::to_string,
    );
    push_set(&mut fields, "weak_foot", weak_foot, weak_foot_map);
    push_set(&mut fields, "cards", cards, ToString::to_string);
    push_set(&mut fields, "min_cards", min_cards, ToString::to_string);
    push_set(&mut fields, "free", free, id_counts);
    push_set(&mut fields, "a_positions", a_positions, ToString::to_string);
    push_some(&mut fields, "name_color", name_color, |color| {
        string_literal(color)
    });
    push_set(
        &mut fields,
        "stat_warning",
        stat_warning,
        ToString::to_string,
    );
    push_set(
        &mut fields,
        "total_within_targets",
        total_within_targets,
        ToString::to_string,
    );
    fields
}

/// `system`'s fields as `key: value` strings, in declaration order
/// (`label` and `counts` required).
fn medal_system_fields(system: &MedalSystem) -> Vec<String> {
    let MedalSystem { label, counts } = system;
    vec![
        field("label", string_literal(label)),
        field("counts", id_counts(counts)),
    ]
}

/// `bracket`'s fields as `key: value` strings, `up_to` or `exactly`
/// flattened into the bracket's map as it is read.
fn bracket_fields(bracket: &Bracket) -> Vec<String> {
    let Bracket { id, label, heights } = bracket;
    let mut fields = vec![
        field("id", string_literal(id)),
        field("label", string_literal(label)),
    ];
    match heights {
        BracketHeights::UpTo(up_to) => fields.push(field("up_to", up_to)),
        BracketHeights::Exactly(exactly) => fields.push(field("exactly", number_list(exactly))),
    }
    fields
}

/// `system`'s fields as `key: value` strings: `by_quotas` before
/// `quotas`, as the hand-written blocks spell it.
fn system_fields(system: &HeightSystem) -> Vec<String> {
    let HeightSystem {
        id,
        label,
        any_player,
        quotas,
        by_quotas,
        extra_height,
    } = system;
    let mut fields = vec![
        field("id", string_literal(id)),
        field("label", string_literal(label)),
    ];
    push_some(&mut fields, "any_player", any_player, condition_map);
    push_set(&mut fields, "by_quotas", by_quotas, ToString::to_string);
    push_set(&mut fields, "quotas", quotas, id_counts);
    push_some(&mut fields, "extra_height", extra_height, extra_height_map);
    fields
}

/// `special`'s fields as `key: value` strings: `severity` right after
/// `label`, as the hand-written blocks spell it.
fn special_fields(special: &Special) -> Vec<String> {
    let Special {
        label,
        when,
        then,
        severity,
    } = special;
    let mut fields = vec![field("label", string_literal(label))];
    push_set(&mut fields, "severity", severity, |severity| {
        string_literal(severity_label(*severity))
    });
    push_set(&mut fields, "when", when, condition_map);
    push_set(&mut fields, "then", then, effects_map);
    fields
}

/// `settings`' fields as `key: value` strings, in declaration order.
fn team_settings_fields(settings: &TeamSettings) -> Vec<String> {
    let TeamSettings {
        label,
        versions,
        man_marking,
        auto_substitution,
        auto_offside_trap,
        auto_preset_change,
        auto_attack_defence_levels,
        forbidden_shapes,
    } = settings;
    let mut fields = vec![field("label", string_literal(label))];
    push_set(&mut fields, "versions", versions, |versions| {
        rhai_list(versions.iter().map(|version| version.number().to_string()))
    });
    for (key, setting) in [
        ("man_marking", man_marking),
        ("auto_substitution", auto_substitution),
        ("auto_offside_trap", auto_offside_trap),
        ("auto_preset_change", auto_preset_change),
        ("auto_attack_defence_levels", auto_attack_defence_levels),
    ] {
        push_some(&mut fields, key, setting, ToString::to_string);
    }
    push_set(
        &mut fields,
        "forbidden_shapes",
        forbidden_shapes,
        |shapes| rhai_list(shapes.iter().map(|shape| number_list(shape))),
    );
    fields
}

/// `universal`'s fields as `key: value` strings, in declaration order
/// (`max_skill_cards` required).
fn universal_fields(universal: &Universal) -> Vec<String> {
    let Universal {
        registered_position_a,
        fielded_at_a,
        no_b_ratings,
        b_uses_a_allowance,
        gk_not_second_a,
        age,
        weight,
        captain_required,
        min_goalkeepers,
        max_skill_cards,
        max_com_styles,
    } = universal;
    let mut fields: Vec<String> = Vec::new();
    push_set(
        &mut fields,
        "registered_position_a",
        registered_position_a,
        ToString::to_string,
    );
    push_set(&mut fields, "fielded_at_a", fielded_at_a, |fielded_at_a| {
        number_list(fielded_at_a)
    });
    push_set(
        &mut fields,
        "no_b_ratings",
        no_b_ratings,
        ToString::to_string,
    );
    push_set(
        &mut fields,
        "b_uses_a_allowance",
        b_uses_a_allowance,
        ToString::to_string,
    );
    push_set(
        &mut fields,
        "gk_not_second_a",
        gk_not_second_a,
        ToString::to_string,
    );
    push_some(&mut fields, "age", age, |(low, high)| {
        format!("[{low}, {high}]")
    });
    push_some(&mut fields, "weight", weight, weight_map);
    push_set(
        &mut fields,
        "captain_required",
        captain_required,
        ToString::to_string,
    );
    push_set(
        &mut fields,
        "min_goalkeepers",
        min_goalkeepers,
        ToString::to_string,
    );
    fields.push(field("max_skill_cards", max_skill_cards));
    push_some(
        &mut fields,
        "max_com_styles",
        max_com_styles,
        ToString::to_string,
    );
    fields
}

/// One `#{ field: value, ... },` item under `indent`: the fields joined
/// `, `, wrapping onto continuation lines indented three more spaces
/// (under the first field) when the line would pass `WIDTH`. A field is
/// never split across lines: a longer one sits alone on its line,
/// overlong.
fn wrap_item(indent: &str, fields: Vec<String>) -> Vec<String> {
    if fields.is_empty() {
        return vec![format!("{indent}#{{}},")];
    }
    let mut lines = Vec::new();
    let mut line = format!("{indent}#{{ ");
    let last = fields.len() - 1;
    for (index, entry) in fields.iter().enumerate() {
        let piece = if index == last {
            format!("{entry} }},")
        } else {
            format!("{entry},")
        };
        if line.trim() != "#{" && line.len() + piece.len() + 1 > WIDTH {
            lines.push(line.trim_end().to_string());
            line = format!("{indent}   ");
        }
        line.push_str(&piece);
        line.push(' ');
    }
    lines.push(line.trim_end().to_string());
    lines
}

/// `head`'s `[a, b, c]` list under `indent`, wrapping onto continuation
/// lines aligned under the first item when the line would pass `WIDTH`.
fn wrap_list(indent: &str, head: &str, items: impl IntoIterator<Item = String>) -> Vec<String> {
    let items: Vec<String> = items.into_iter().collect();
    let continuation = " ".repeat(indent.len() + head.len() + 1);
    let mut lines = Vec::new();
    let mut line = format!("{indent}{head}[");
    let last = items.len().saturating_sub(1);
    for (index, item) in items.iter().enumerate() {
        let piece = if index == last {
            format!("{item}],")
        } else {
            format!("{item},")
        };
        if index > 0 && line.len() + piece.len() + 1 > WIDTH {
            lines.push(line.trim_end().to_string());
            line = continuation.clone();
        }
        line.push_str(&piece);
        line.push(' ');
    }
    if items.is_empty() {
        line.push_str("],");
    }
    lines.push(line.trim_end().to_string());
    lines
}

/// The `universal: #{ ... }` section: the fields joined `, ` on lines
/// one indent in, wrapped onto same-indent lines when the line would
/// pass `WIDTH`, a trailing comma per line.
fn universal_section(universal: &Universal) -> Vec<String> {
    let mut lines = vec![format!("{INDENT}universal: #{{")];
    // `entry, ` after each field but the last (`entry,`): a wrapped
    // line ends with the field's comma.
    let mut pieces: Vec<String> = universal_fields(universal)
        .iter()
        .map(|entry| format!("{entry}, "))
        .collect();
    if let Some(last) = pieces.last_mut() {
        last.pop();
    }
    let mut line = I2.to_string();
    for piece in &pieces {
        if !line.trim().is_empty() && line.len() + piece.len() > WIDTH {
            lines.push(line.trim_end().to_string());
            line = I2.to_string();
        }
        line.push_str(piece);
    }
    lines.push(line);
    lines.push(format!("{INDENT}}},"));
    lines
}

/// One top-level section of `#[ field ]` items under `I2`.
fn item_section(head: &str, items: Vec<Vec<String>>) -> Vec<String> {
    let mut lines = vec![format!("{INDENT}{head}: [")];
    for fields in items {
        lines.extend(wrap_item(I2, fields));
    }
    lines.push(format!("{INDENT}],"));
    lines
}

impl Ruleset {
    /// The data block that reads back as this ruleset: the header
    /// comment line and `const RULESET = #{ ... };`, ending in a
    /// newline.
    pub fn to_data_block(&self) -> String {
        // Destructuring names every field on purpose: a field added to
        // the schema fails to compile here until the writer handles it.
        // The same holds of the field functions below. The fields come
        // in declaration order but three, spelled as the hand-written
        // blocks and the editor's preview read them: a special's
        // `severity` follows `label`, a height system's `by_quotas`
        // precedes `quotas`, and `logo` comes last, its long base64 line
        // after every key a person reads.
        let Ruleset {
            schema,
            name,
            page,
            logo,
            stats,
            tiers,
            card_groups,
            medal_systems,
            heights,
            universal,
            specials,
            team_settings,
            suggestions,
        } = self;
        let mut sections: Vec<Vec<String>> = Vec::new();

        let StatRules { rating, at_most } = stats;
        if !rating.is_empty() || !at_most.is_empty() {
            let mut lines = vec![format!("{INDENT}stats: #{{")];
            if !rating.is_empty() {
                lines.extend(wrap_list(
                    I2,
                    "rating: ",
                    rating.iter().map(|stat| string_literal(stat.label())),
                ));
            }
            if !at_most.is_empty() {
                lines.extend(wrap_list(
                    I2,
                    "at_most: ",
                    at_most.iter().map(|name| string_literal(name)),
                ));
            }
            lines.push(format!("{INDENT}}},"));
            sections.push(lines);
        }

        if !tiers.is_empty() {
            sections.push(item_section(
                "tiers",
                tiers.iter().map(tier_fields).collect(),
            ));
        }

        if !card_groups.is_empty() {
            let mut lines = vec![format!("{INDENT}card_groups: #{{")];
            for (group, cards) in card_groups {
                lines.extend(wrap_list(
                    I2,
                    &format!("{}: ", map_key(group)),
                    cards.iter().map(|card| string_literal(card.label())),
                ));
            }
            lines.push(format!("{INDENT}}},"));
            sections.push(lines);
        }

        if !medal_systems.is_empty() {
            sections.push(item_section(
                "medal_systems",
                medal_systems.iter().map(medal_system_fields).collect(),
            ));
        }

        let Heights { brackets, systems } = heights;
        if !brackets.is_empty() || !systems.is_empty() {
            let mut lines = vec![format!("{INDENT}heights: #{{")];
            if !brackets.is_empty() {
                lines.push(format!("{I2}brackets: ["));
                for bracket in brackets {
                    lines.extend(wrap_item(I3, bracket_fields(bracket)));
                }
                lines.push(format!("{I2}],"));
            }
            if !systems.is_empty() {
                lines.push(format!("{I2}systems: ["));
                for system in systems {
                    lines.extend(wrap_item(I3, system_fields(system)));
                }
                lines.push(format!("{I2}],"));
            }
            lines.push(format!("{INDENT}}},"));
            sections.push(lines);
        }

        sections.push(universal_section(universal));

        if !specials.is_empty() {
            sections.push(item_section(
                "specials",
                specials.iter().map(special_fields).collect(),
            ));
        }

        if !team_settings.is_empty() {
            sections.push(item_section(
                "team_settings",
                team_settings.iter().map(team_settings_fields).collect(),
            ));
        }

        if !suggestions.is_empty() {
            sections.push(wrap_list(
                INDENT,
                "suggestions: ",
                suggestions.iter().map(|entry| string_literal(entry)),
            ));
        }

        if let Some(logo) = logo {
            sections.push(vec![format!("{INDENT}logo: {},", string_literal(logo))]);
        }

        let mut block = format!(
            "{HEADER}\nconst RULESET = #{{\n{INDENT}schema: {schema},\n{INDENT}name: {},\n",
            string_literal(name)
        );
        if let Some(page) = page {
            block.push_str(&format!("{INDENT}page: {},\n", string_literal(page)));
        }
        for section in &sections {
            block.push('\n');
            for line in section {
                block.push_str(line);
                block.push('\n');
            }
        }
        block.push_str("};\n");
        block
    }
}

#[cfg(test)]
mod tests;
