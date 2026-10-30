//! The writer's tests: every type's fields round-tripped, the layout pinned, its edges.

use std::collections::BTreeMap;

use pes_version::PesVersion;

use super::*;
use crate::ruleset::{Card, Foot, PlayStyle, PlayerValue, Position, Stat};

/// `name` as a `Stat`.
fn stat(name: &str) -> Stat {
    Stat::try_from(name.to_string()).expect("a stat name")
}

/// `name` as a `Position`.
fn position(name: &str) -> Position {
    Position::try_from(name.to_string()).expect("a position name")
}

/// `name` as a `PlayStyle`.
fn play_style(name: &str) -> PlayStyle {
    PlayStyle::try_from(name.to_string()).expect("a playing style")
}

/// `name` as a `Card`.
fn card(name: &str) -> Card {
    Card::try_from(name.to_string()).expect("a card name")
}

/// `map_key`'s bare-or-literal decision: an identifier shape with a
/// letter is bare when rhai's parser takes `#{ <key>: 0 }` (`in`,
/// `go` are reserved), everything else a string literal — `a: 0, b`
/// among them, which the parser alone would take as two keys.
#[test]
fn map_key_spells_identifiers_bare_and_the_rest_quoted() {
    for (key, expected) in [
        ("gold", "gold"),
        ("_x", "_x"),
        ("ball_winning", "ball_winning"),
        ("a_1", "a_1"),
        ("a: 0, b", "\"a: 0, b\""),
        ("_", "\"_\""),
        ("__", "\"__\""),
        ("1st", "\"1st\""),
        ("in", "\"in\""),
        ("go", "\"go\""),
        ("médaille", "\"médaille\""),
    ] {
        assert_eq!(map_key(key), expected, "{key}");
    }
}

/// `wrap_item`'s edge: a second field ending the line at the width
/// stays, one character more wraps it under the first field (the
/// indent plus three spaces).
#[test]
fn wrap_item_wraps_the_field_over_the_width() {
    // `        #{ ` (11) + `a: 1, ` (6) leaves 17; a 79-char field's
    // ` },` piece is 82, and 17 + 82 + 1 is exactly WIDTH.
    let lines = wrap_item("        ", vec!["a: 1".to_string(), "x".repeat(79)]);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0].len(), 99);
    assert_eq!(
        lines[0],
        format!("        #{{ a: 1, {} }},", "x".repeat(79))
    );
    let lines = wrap_item("        ", vec!["a: 1".to_string(), "x".repeat(80)]);
    assert_eq!(
        lines,
        vec![
            "        #{ a: 1,".to_string(),
            format!("           {} }},", "x".repeat(80)),
        ]
    );
}

/// `wrap_item`'s first field stays on the head line however long (a
/// field is never split): an overlong one leaves the `#{` line past
/// `WIDTH` and the next field wraps under it.
#[test]
fn wrap_item_keeps_an_overlong_first_field_on_the_head_line() {
    let lines = wrap_item("        ", vec!["x".repeat(120), "b: 2".to_string()]);
    assert_eq!(
        lines,
        vec![
            format!("        #{{ {},", "x".repeat(120)),
            "           b: 2 },".to_string(),
        ]
    );
}

/// `wrap_list`'s continuation is aligned under the first item
/// (`head`'s `[` and one more), and the first item never wraps: an
/// overlong one stays on the head's line.
#[test]
fn wrap_list_wraps_under_the_first_item() {
    // `        rating: [` is 17 columns: the continuation is 17
    // spaces.
    let lines = wrap_list(
        "        ",
        "rating: ",
        vec!["\"a\"".to_string(), "\"b\"".to_string(), "x".repeat(75)],
    );
    assert_eq!(
        lines,
        vec![
            "        rating: [\"a\", \"b\",".to_string(),
            format!("{}{}],", " ".repeat(17), "x".repeat(75)),
        ]
    );
    let lines = wrap_list("    ", "suggestions: ", vec!["x".repeat(90)]);
    assert_eq!(
        lines,
        vec![format!("    suggestions: [{}],", "x".repeat(90))],
        "the first item never wraps"
    );
}

/// `wrap_list`'s edge: a later item whose line, with the space after it,
/// reaches exactly `WIDTH` stays; one character more wraps it.
#[test]
fn wrap_list_wraps_an_item_past_the_width() {
    // `    suggestions: [` (18) + `"a",` and its space (5) leaves 23; a
    // 74-character item's `],` piece is 76, and 23 + 76 + 1 is WIDTH.
    let lines = wrap_list(
        "    ",
        "suggestions: ",
        vec!["\"a\"".to_string(), "x".repeat(74)],
    );
    assert_eq!(
        lines,
        vec![format!("    suggestions: [\"a\", {}],", "x".repeat(74))]
    );
    let lines = wrap_list(
        "    ",
        "suggestions: ",
        vec!["\"a\"".to_string(), "x".repeat(75)],
    );
    assert_eq!(
        lines,
        vec![
            "    suggestions: [\"a\",".to_string(),
            format!("{}{}],", " ".repeat(18), "x".repeat(75)),
        ]
    );
}

/// `universal`'s fields joined `, ` one indent in, each line ending
/// a comma: `max_skill_cards`'s piece lands the line on exactly
/// `WIDTH` and stays, `max_com_styles` wraps onto its own line.
#[test]
fn universal_wraps_onto_same_indent_lines() {
    let universal = Universal {
        registered_position_a: true,
        fielded_at_a: Vec::new(),
        no_b_ratings: false,
        b_uses_a_allowance: true,
        gk_not_second_a: false,
        age: Some((18, 135)),
        weight: None,
        captain_required: false,
        min_goalkeepers: 0,
        max_skill_cards: 10,
        max_com_styles: Some(2),
    };
    assert_eq!(
            universal_section(&universal),
            vec![
                "    universal: #{".to_string(),
                "        registered_position_a: true, b_uses_a_allowance: true, age: [18, 135], max_skill_cards: 10,".to_string(),
                "        max_com_styles: 2,".to_string(),
                "    },".to_string(),
            ]
        );
}

/// `universal`'s first field stays on the header's next line however
/// long: an overlong `fielded_at_a` sits alone (no blank line under
/// `universal: #{`), `max_skill_cards` wrapping onto the next.
#[test]
fn universal_keeps_an_overlong_first_field_on_its_line() {
    let universal = Universal {
        registered_position_a: false,
        fielded_at_a: vec![1; 40],
        no_b_ratings: false,
        b_uses_a_allowance: false,
        gk_not_second_a: false,
        age: None,
        weight: None,
        captain_required: false,
        min_goalkeepers: 0,
        max_skill_cards: 10,
        max_com_styles: None,
    };
    let lines = universal_section(&universal);
    assert!(
        lines.iter().all(|line| !line.trim().is_empty()),
        "{lines:?}"
    );
    assert_eq!(
        lines,
        vec![
            "    universal: #{".to_string(),
            format!("        fielded_at_a: [{}],", vec!["1"; 40].join(", ")),
            "        max_skill_cards: 10,".to_string(),
            "    },".to_string(),
        ]
    );
}

/// The sections' presence: `stats` writes only the list it holds
/// (either alone), `heights` either list it holds — and neither
/// writes an empty section. The editor's preview hands the writer
/// rulesets the load rules would refuse, so `brackets` without
/// `systems` still writes.
#[test]
fn sections_appear_only_with_content() {
    // VGL26 has no heights: no `heights:` section.
    let vgl26 =
        Ruleset::from_data_block(include_str!("../../rulesets/vgl26.rhai")).expect("VGL26 loads");
    assert!(!vgl26.to_data_block().contains("heights:"));

    let mut ruleset = small_ruleset();
    // `small_ruleset` has `rating` and `at_most` empty: no `stats:`.
    assert!(!ruleset.to_data_block().contains("stats:"));

    // Brackets but no systems: `brackets:` alone.
    ruleset.heights.systems.clear();
    let written = ruleset.to_data_block();
    assert!(written.contains("brackets: ["), "{written}");
    assert!(!written.contains("systems:"), "{written}");

    // `stats` with `rating` only, then `at_most` only.
    ruleset.stats = StatRules {
        rating: vec![stat("speed")],
        at_most: Vec::new(),
    };
    let written = ruleset.to_data_block();
    assert!(written.contains("stats: #{"), "{written}");
    assert!(written.contains("rating: ["), "{written}");
    assert!(!written.contains("at_most:"), "{written}");
    ruleset.stats = StatRules {
        rating: Vec::new(),
        at_most: vec!["form".to_string()],
    };
    let written = ruleset.to_data_block();
    assert!(written.contains("at_most: ["), "{written}");
    assert!(!written.contains("rating:"), "{written}");
}

/// Every embedded ruleset: written, read back equal, written again
/// byte-identical. `vtl10` and `vtl11` end in a CUSTOM CHECKS
/// section the writer does not carry, so the comparison is of the
/// `Ruleset`s, not of the written text to the file.
#[test]
fn every_embedded_ruleset_round_trips() {
    for (name, text) in [
        ("autumn25", include_str!("../../rulesets/autumn25.rhai")),
        ("autumn26", include_str!("../../rulesets/autumn26.rhai")),
        ("dbc7", include_str!("../../rulesets/dbc7.rhai")),
        ("spring26", include_str!("../../rulesets/spring26.rhai")),
        ("vgl25", include_str!("../../rulesets/vgl25.rhai")),
        ("vgl26", include_str!("../../rulesets/vgl26.rhai")),
        ("vgl27", include_str!("../../rulesets/vgl27.rhai")),
        ("vtl10", include_str!("../../rulesets/vtl10.rhai")),
        ("vtl11", include_str!("../../rulesets/vtl11.rhai")),
    ] {
        let ruleset =
            Ruleset::from_data_block(text).unwrap_or_else(|error| panic!("{name}: {error}"));
        let written = ruleset.to_data_block();
        let reread = Ruleset::from_data_block(&written)
            .unwrap_or_else(|error| panic!("{name}: {error}\n{written}"));
        assert_eq!(reread, ruleset, "{name}");
        assert_eq!(reread.to_data_block(), written, "{name}");
    }
}

/// A `Condition` holding `height_min`/`height_max` and nothing else
/// (every field named, the same compile guard as the writer's
/// destructuring, throughout this module).
fn height_condition(low: u8, high: u8) -> Condition {
    Condition {
        tiers: Vec::new(),
        positions: Vec::new(),
        systems: Vec::new(),
        brackets: Vec::new(),
        height_min: Some(low),
        height_max: Some(high),
        captain: None,
        playing_styles: Vec::new(),
        holds: Vec::new(),
        rated_a_at: Vec::new(),
        rated_b_at: Vec::new(),
        name_colors: Vec::new(),
        fielded_only_at: None,
        ranges: BTreeMap::new(),
        strong_foot: None,
        any: Vec::new(),
        not: None,
    }
}

/// A `Condition` holding only `strong_foot`.
fn strong_foot_condition(strong_foot: Option<Foot>) -> Condition {
    Condition {
        tiers: Vec::new(),
        positions: Vec::new(),
        systems: Vec::new(),
        brackets: Vec::new(),
        height_min: None,
        height_max: None,
        captain: None,
        playing_styles: Vec::new(),
        holds: Vec::new(),
        rated_a_at: Vec::new(),
        rated_b_at: Vec::new(),
        name_colors: Vec::new(),
        fielded_only_at: None,
        ranges: BTreeMap::new(),
        strong_foot,
        any: Vec::new(),
        not: None,
    }
}

/// A `Condition` holding only `positions` (the `not` below).
fn positions_condition(positions: Vec<Position>) -> Condition {
    Condition {
        tiers: Vec::new(),
        positions,
        systems: Vec::new(),
        brackets: Vec::new(),
        height_min: None,
        height_max: None,
        captain: None,
        playing_styles: Vec::new(),
        holds: Vec::new(),
        rated_a_at: Vec::new(),
        rated_b_at: Vec::new(),
        name_colors: Vec::new(),
        fielded_only_at: None,
        ranges: BTreeMap::new(),
        strong_foot: None,
        any: Vec::new(),
        not: None,
    }
}

/// A `Condition` holding only `captain`.
fn captain_condition(captain: Option<bool>) -> Condition {
    Condition {
        tiers: Vec::new(),
        positions: Vec::new(),
        systems: Vec::new(),
        brackets: Vec::new(),
        height_min: None,
        height_max: None,
        captain,
        playing_styles: Vec::new(),
        holds: Vec::new(),
        rated_a_at: Vec::new(),
        rated_b_at: Vec::new(),
        name_colors: Vec::new(),
        fielded_only_at: None,
        ranges: BTreeMap::new(),
        strong_foot: None,
        any: Vec::new(),
        not: None,
    }
}

/// A `Condition` with every field set non-default: `any` and `not`
/// nested, every `PlayerValue` in `ranges`, both `Foot`s across the
/// condition and its `any` entry.
fn full_condition() -> Condition {
    Condition {
        tiers: vec!["mid".to_string()],
        positions: vec![position("GK")],
        systems: vec!["green".to_string()],
        brackets: vec!["tall".to_string()],
        height_min: Some(180),
        height_max: Some(190),
        captain: Some(true),
        playing_styles: vec![play_style("goal_poacher")],
        holds: vec![card("scissors_feint"), card("long_ranger")],
        rated_a_at: vec![position("CF")],
        rated_b_at: vec![position("CB")],
        name_colors: vec!["cc9900ff".to_string()],
        fielded_only_at: Some(position("SS")),
        ranges: [
            (PlayerValue::Stat(stat("speed")), (90, 99)),
            (PlayerValue::Age, (18, 35)),
            (PlayerValue::Weight, (70, 90)),
            (PlayerValue::Form, (4, 8)),
            (PlayerValue::InjuryResistance, (1, 3)),
            (PlayerValue::WeakFootUsage, (1, 4)),
            (PlayerValue::WeakFootAccuracy, (1, 4)),
        ]
        .into_iter()
        .collect(),
        strong_foot: Some(Foot::Right),
        any: vec![
            strong_foot_condition(Some(Foot::Left)),
            captain_condition(Some(false)),
        ],
        not: Some(Box::new(positions_condition(vec![position("SS")]))),
    }
}

/// `(&str, n)` pairs as a `BTreeMap<String, _>` (tier ids in
/// `PerTier::ByTier`, group ids in `free`, counts in `quotas`).
fn by_id<T: Copy>(pairs: &[(&str, T)]) -> BTreeMap<String, T> {
    pairs
        .iter()
        .map(|&(id, value)| (id.to_string(), value))
        .collect()
}

/// An `Effects` with every field set non-default but
/// `height_allowance` (`full_condition` names brackets, which an
/// allowance forbids): both `PerTier` forms and both `StatOrAll`s.
fn full_effects() -> Effects {
    Effects {
        stat_bonus: [
            (
                StatOrAll::All,
                PerTier::ByTier(by_id(&[("mid", 5), ("gold", 3)])),
            ),
            (StatOrAll::Stat(stat("speed")), PerTier::All(-3)),
        ]
        .into_iter()
        .collect(),
        stat_target: [(
            stat("finishing"),
            PerTier::ByTier(by_id(&[("regular", 90)])),
        )]
        .into_iter()
        .collect(),
        cards: Some(PerTier::All(2)),
        free: [
            ("com".to_string(), PerTier::All(1)),
            (
                "gold-medal".to_string(),
                PerTier::ByTier(by_id(&[("mid", 2)])),
            ),
        ]
        .into_iter()
        .collect(),
        free_cards: vec![card("malicia"), card("long_ranger")],
        require_cards: vec![card("captaincy")],
        a_positions: Some(PerTier::All(1)),
        max_a_positions: Some(4),
        form: Some(PerTier::ByTier(by_id(&[("gold", 2)]))),
        injury_resistance: Some(PerTier::All(-1)),
        weak_foot: Some(WeakFoot {
            usage: 4,
            accuracy: 4,
        }),
        height_allowance: None,
        forbid: Some("The special forbids this".to_string()),
        choice: vec![choice_effects()],
        stat_warning: Some(PerTier::ByTier(by_id(&[("regular", 5)]))),
    }
}

/// A `choice` option: only the effects that raise a limit.
fn choice_effects() -> Effects {
    Effects {
        stat_bonus: [(StatOrAll::Stat(stat("speed")), PerTier::All(5))]
            .into_iter()
            .collect(),
        stat_target: BTreeMap::new(),
        cards: Some(PerTier::ByTier(by_id(&[("mid", 1)]))),
        free: [("in".to_string(), PerTier::All(1))].into_iter().collect(),
        free_cards: vec![card("malicia")],
        require_cards: Vec::new(),
        a_positions: Some(PerTier::All(1)),
        max_a_positions: Some(3),
        form: None,
        injury_resistance: None,
        weak_foot: Some(WeakFoot {
            usage: 3,
            accuracy: 3,
        }),
        height_allowance: None,
        forbid: None,
        choice: Vec::new(),
        stat_warning: None,
    }
}

/// An `Effects` holding only `height_allowance`.
fn allowance_effects() -> Effects {
    Effects {
        stat_bonus: BTreeMap::new(),
        stat_target: BTreeMap::new(),
        cards: None,
        free: BTreeMap::new(),
        free_cards: Vec::new(),
        require_cards: Vec::new(),
        a_positions: None,
        max_a_positions: None,
        form: None,
        injury_resistance: None,
        weak_foot: None,
        height_allowance: Some(4),
        forbid: None,
        choice: Vec::new(),
        stat_warning: None,
    }
}

/// An `Effects` with nothing set.
fn empty_effects() -> Effects {
    Effects {
        stat_bonus: BTreeMap::new(),
        stat_target: BTreeMap::new(),
        cards: None,
        free: BTreeMap::new(),
        free_cards: Vec::new(),
        require_cards: Vec::new(),
        a_positions: None,
        max_a_positions: None,
        form: None,
        injury_resistance: None,
        weak_foot: None,
        height_allowance: None,
        forbid: None,
        choice: Vec::new(),
        stat_warning: None,
    }
}

/// A `Tier` with every field set non-default but `count` and
/// `counts_as` (the two cannot coexist): `when` holds heights only
/// since a tier's condition may not name tiers.
fn rich_tier(id: &str, label: &str, count: Option<u8>, counts_as: Option<&str>) -> Tier {
    Tier {
        id: id.to_string(),
        label: label.to_string(),
        count,
        counts_as: counts_as.map(str::to_string),
        when: height_condition(180, 200),
        rate: 90,
        stats: [(stat("speed"), 95)].into_iter().collect(),
        form: 8,
        injury_resistance: 3,
        weak_foot: WeakFoot {
            usage: 4,
            accuracy: 4,
        },
        cards: 6,
        min_cards: 2,
        free: by_id(&[
            ("1st", 1),
            ("com", 1),
            ("go", 1),
            ("gold-medal", 2),
            ("in", 2),
        ]),
        a_positions: 1,
        name_color: Some("cc9900ff".to_string()),
        stat_warning: 5,
        total_within_targets: true,
    }
}

/// A ruleset holding every value the schema can spell: every field
/// of every type named and set non-default (where the load rules
/// allow), every enum variant present, ids needing string-literal
/// map keys (`"in"`, `"go"`, `"1st"`, `"gold-medal"`, `"médaille"`)
/// and labels needing escapes. One ruleset suffices: `any` and
/// `not` leave room for both `Foot`s and the second `PerTier` form.
fn every_field_ruleset() -> Ruleset {
    Ruleset {
        schema: 1,
        name: "Every field".to_string(),
        page: Some("https://implyingrigged.info/wiki/2026_4chan_Autumn_Babby_Cup".to_string()),
        // The PNG signature alone, base64: the load checks go no further.
        logo: Some("iVBORw0KGgo=".to_string()),
        stats: StatRules {
            rating: vec![stat("speed"), stat("dribbling")],
            at_most: vec!["attacking_prowess".to_string(), "form".to_string()],
        },
        tiers: vec![
            // A `"` `\`, a tab and `é` in `label` (written `\"` `\\`
            // `\t` and as it is).
            rich_tier("gold", "Gold \"médaille\" \\\té", None, None),
            rich_tier("silver", "Silver", None, None),
            rich_tier("mid", "Mid", Some(5), None),
            rich_tier("médaille", "Médaille", None, Some("regular")),
            rich_tier("regular", "Regular", None, None),
        ],
        card_groups: [
            ("1st".to_string(), vec![card("scissors_feint")]),
            ("go".to_string(), vec![card("flip_flap")]),
            ("gold-medal".to_string(), vec![card("marseille_turn")]),
            ("in".to_string(), vec![card("sombrero")]),
        ]
        .into_iter()
        .collect(),
        medal_systems: vec![
            MedalSystem {
                label: "GGG".to_string(),
                counts: by_id(&[("gold", 3)]),
            },
            MedalSystem {
                label: "GGSS".to_string(),
                counts: by_id(&[("gold", 2), ("silver", 2)]),
            },
        ],
        heights: Heights {
            brackets: vec![
                Bracket {
                    id: "manlet".to_string(),
                    label: "Manlet".to_string(),
                    heights: BracketHeights::UpTo(175),
                },
                Bracket {
                    id: "tall".to_string(),
                    label: "Tall".to_string(),
                    heights: BracketHeights::Exactly(vec![185]),
                },
                Bracket {
                    id: "giga".to_string(),
                    label: "Giga".to_string(),
                    heights: BracketHeights::Exactly(vec![210]),
                },
            ],
            systems: vec![
                HeightSystem {
                    id: "green".to_string(),
                    label: "Green".to_string(),
                    any_player: Some(height_condition(190, 255)),
                    quotas: by_id(&[("manlet", 5), ("tall", 18)]),
                    by_quotas: true,
                    extra_height: None,
                },
                HeightSystem {
                    id: "giga".to_string(),
                    label: "Giga".to_string(),
                    any_player: None,
                    quotas: by_id(&[("tall", 17), ("giga", 6)]),
                    by_quotas: false,
                    extra_height: Some(ExtraHeight {
                        total: 50,
                        max: 210,
                    }),
                },
            ],
        },
        universal: Universal {
            registered_position_a: true,
            fielded_at_a: vec![1, 2],
            no_b_ratings: true,
            b_uses_a_allowance: true,
            gk_not_second_a: true,
            age: Some((18, 35)),
            weight: Some(WeightRule {
                floor: 30,
                below: 129,
                above: 81,
            }),
            captain_required: true,
            min_goalkeepers: 1,
            max_skill_cards: 10,
            max_com_styles: Some(2),
        },
        specials: vec![
            Special {
                label: "Every field".to_string(),
                when: full_condition(),
                then: full_effects(),
                severity: Severity::Error,
            },
            Special {
                label: "Goalkeepers at 189 count as 185".to_string(),
                when: height_condition(189, 189),
                then: allowance_effects(),
                severity: Severity::Warning,
            },
            Special {
                // U+0001, written `\x01`.
                label: "A\u{1}suggestion".to_string(),
                when: strong_foot_condition(None),
                then: empty_effects(),
                severity: Severity::Suggestion,
            },
        ],
        team_settings: vec![TeamSettings {
            label: "Support settings".to_string(),
            versions: vec![PesVersion::Pes16, PesVersion::Pes21],
            man_marking: Some(false),
            auto_substitution: Some(true),
            auto_offside_trap: Some(false),
            auto_preset_change: Some(true),
            auto_attack_defence_levels: Some(false),
            forbidden_shapes: vec![[3, 4, 3]],
        }],
        suggestions: vec![
            "cards".to_string(),
            "in".to_string(),
            "go".to_string(),
            "choices".to_string(),
        ],
    }
}

/// Every field of every type round-trips, written and read back.
#[test]
fn every_field_round_trips() {
    let ruleset = every_field_ruleset();
    let written = ruleset.to_data_block();
    let reread =
        Ruleset::from_data_block(&written).unwrap_or_else(|error| panic!("{error}\n{written}"));
    assert_eq!(reread, ruleset);
    assert_eq!(reread.to_data_block(), written);
    // Map keys: `in`, `go`, `1st`, `gold-medal` and `médaille` need
    // string literals; `gold` and `com` are bare.
    for key in [
        "\"gold-medal\":",
        "\"in\":",
        "\"go\":",
        "\"1st\":",
        "\"médaille\"",
    ] {
        assert!(written.contains(key), "{key} missing from\n{written}");
    }
    assert!(written.contains("gold:"), "a bare key, from\n{written}");
    assert!(written.contains(" com:"), "a bare key, from\n{written}");
    // The escapes: `\"`, `\\`, `\t`, `\x01`; `é` written as it is.
    for escaped in ["\\\"", "\\\\", "\\t", "\\x01", "é"] {
        assert!(
            written.contains(escaped),
            "{escaped} missing from\n{written}"
        );
    }
    // A `severity: "error"` is never written: it is the default.
    assert!(!written.contains("error"), "Severity::Error omitted");
}

/// `page` is written on the line after `name`, `logo` as the data
/// block's last key, after `suggestions`.
#[test]
fn page_follows_name_and_logo_comes_last() {
    let written = every_field_ruleset().to_data_block();
    let lines: Vec<&str> = written.lines().collect();
    let name = lines
        .iter()
        .position(|line| *line == "    name: \"Every field\",")
        .expect("the name line");
    assert_eq!(
        lines[name + 1],
        "    page: \"https://implyingrigged.info/wiki/2026_4chan_Autumn_Babby_Cup\",",
        "{written}"
    );
    let end = lines
        .iter()
        .position(|line| *line == "};")
        .expect("the end");
    assert_eq!(lines[end - 1], "    logo: \"iVBORw0KGgo=\",", "{written}");
}

/// The small ruleset `the_layout_is_pinned` pins: two tiers, one
/// bracket, one height system, one special, `universal` with two
/// fields.
fn small_ruleset() -> Ruleset {
    Ruleset {
        schema: 1,
        name: "Small".to_string(),
        page: None,
        logo: None,
        stats: StatRules {
            rating: Vec::new(),
            at_most: Vec::new(),
        },
        tiers: vec![
            Tier {
                id: "gold".to_string(),
                label: "Gold".to_string(),
                count: Some(3),
                counts_as: None,
                when: strong_foot_condition(None),
                rate: 99,
                stats: BTreeMap::new(),
                form: 0,
                injury_resistance: 3,
                weak_foot: WeakFoot {
                    usage: 4,
                    accuracy: 4,
                },
                cards: 6,
                min_cards: 0,
                free: by_id(&[("com", 1)]),
                a_positions: 0,
                name_color: None,
                stat_warning: 0,
                total_within_targets: false,
            },
            Tier {
                id: "regular".to_string(),
                label: "Regular".to_string(),
                count: None,
                counts_as: None,
                when: strong_foot_condition(None),
                rate: 77,
                stats: BTreeMap::new(),
                form: 0,
                injury_resistance: 0,
                weak_foot: WeakFoot {
                    usage: 0,
                    accuracy: 0,
                },
                cards: 0,
                min_cards: 0,
                free: BTreeMap::new(),
                a_positions: 0,
                name_color: None,
                stat_warning: 0,
                total_within_targets: false,
            },
        ],
        card_groups: BTreeMap::new(),
        medal_systems: Vec::new(),
        heights: Heights {
            brackets: vec![Bracket {
                id: "tall".to_string(),
                label: "Tall".to_string(),
                heights: BracketHeights::Exactly(vec![185]),
            }],
            systems: vec![
                // A `by_quotas` system may not be the last, so
                // `rest` applies otherwise after it.
                HeightSystem {
                    id: "picked".to_string(),
                    label: "Picked".to_string(),
                    any_player: None,
                    quotas: by_id(&[("tall", 3)]),
                    by_quotas: true,
                    extra_height: None,
                },
                HeightSystem {
                    id: "rest".to_string(),
                    label: "Rest".to_string(),
                    any_player: None,
                    quotas: by_id(&[("tall", 8)]),
                    by_quotas: false,
                    extra_height: None,
                },
            ],
        },
        universal: Universal {
            registered_position_a: false,
            fielded_at_a: Vec::new(),
            no_b_ratings: false,
            b_uses_a_allowance: false,
            gk_not_second_a: false,
            age: None,
            weight: None,
            captain_required: true,
            min_goalkeepers: 0,
            max_skill_cards: 10,
            max_com_styles: None,
        },
        specials: vec![Special {
            label: "A special".to_string(),
            when: captain_condition(Some(true)),
            then: Effects {
                stat_bonus: BTreeMap::new(),
                stat_target: BTreeMap::new(),
                cards: None,
                free: BTreeMap::new(),
                free_cards: vec![card("captaincy")],
                require_cards: Vec::new(),
                a_positions: None,
                max_a_positions: None,
                form: None,
                injury_resistance: None,
                weak_foot: None,
                height_allowance: None,
                forbid: None,
                choice: Vec::new(),
                stat_warning: None,
            },
            severity: Severity::Warning,
        }],
        team_settings: Vec::new(),
        suggestions: vec!["cards".to_string()],
    }
}

/// The written block, pinned to the byte: the sections and their
/// blank lines, one `#{ ... },` item wrapped under its first field,
/// `universal` inside `#{ ... }`, the lists and the `};` ending.
/// The two departures from declaration order show too: `severity`
/// right after `label`, `by_quotas` before `quotas`. `Tier::form` 0
/// is a default, so no `form:` is written; the block reads back
/// equal.
#[test]
fn the_layout_is_pinned() {
    let ruleset = small_ruleset();
    const EXPECTED: &str = r##"// AATF ruleset, a Rhai script. Open it in the Ruleset editor to change it.
const RULESET = #{
    schema: 1,
    name: "Small",

    tiers: [
        #{ id: "gold", label: "Gold", count: 3, rate: 99, injury_resistance: 3,
           weak_foot: #{ usage: 4, accuracy: 4 }, cards: 6, free: #{ com: 1 } },
        #{ id: "regular", label: "Regular", rate: 77 },
    ],

    heights: #{
        brackets: [
            #{ id: "tall", label: "Tall", exactly: [185] },
        ],
        systems: [
            #{ id: "picked", label: "Picked", by_quotas: true, quotas: #{ tall: 3 } },
            #{ id: "rest", label: "Rest", quotas: #{ tall: 8 } },
        ],
    },

    universal: #{
        captain_required: true, max_skill_cards: 10,
    },

    specials: [
        #{ label: "A special", severity: "warning", when: #{ captain: true },
           then: #{ free_cards: ["captaincy"] } },
    ],

    suggestions: ["cards"],
};
"##;
    let written = ruleset.to_data_block();
    assert_eq!(written, EXPECTED);
    assert!(!written.contains("form:"), "a tier's `form` 0 is left out");
    assert!(
        !written.contains("error"),
        "`Severity::Error` is never spelled"
    );
    let reread =
        Ruleset::from_data_block(&written).unwrap_or_else(|error| panic!("{error}\n{written}"));
    assert_eq!(reread, ruleset);
    assert_eq!(reread.to_data_block(), written);
}
