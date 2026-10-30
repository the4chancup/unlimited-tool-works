use super::*;

#[test]
fn a_duplicate_tier_id_is_refused() {
    // A second `regular` appended: every reference still resolves, so
    // the only error is the duplicate.
    let text = AUTUMN26.replacen(
        "#{ id: \"regular\", label: \"Regular\", rate: 77, form: 4, injury_resistance: 1,\n           weak_foot: #{ usage: 2, accuracy: 2 }, cards: 3, free: #{ tricks: 2 }, a_positions: 1 },",
        "#{ id: \"regular\", label: \"Regular\", rate: 77, form: 4, injury_resistance: 1,\n           weak_foot: #{ usage: 2, accuracy: 2 }, cards: 3, free: #{ tricks: 2 }, a_positions: 1 },\n        #{ id: \"regular\", label: \"Second\", count: 1, rate: 70, form: 4, injury_resistance: 1,\n           weak_foot: #{ usage: 2, accuracy: 2 }, cards: 1, free: #{}, a_positions: 1 },",
        1,
    );
    case(
        "duplicate tier",
        &text,
        LoadRule::DuplicateId {
            kind: IdKind::Tier,
            id: "regular".to_string(),
        },
        "tiers[5]",
    );
}

#[test]
fn a_duplicate_bracket_id_is_refused() {
    // A second `manlet` appended; `mid`'s quota keys still resolve.
    let text = AUTUMN26.replacen(
        "#{ id: \"giga\",   label: \"Giga\",   exactly: [199] },",
        "#{ id: \"giga\",   label: \"Giga\",   exactly: [199] },\n            #{ id: \"manlet\", label: \"Second\", up_to: 210 },",
        1,
    );
    case(
        "duplicate bracket",
        &text,
        LoadRule::DuplicateId {
            kind: IdKind::Bracket,
            id: "manlet".to_string(),
        },
        "heights.brackets[5]",
    );
}

#[test]
fn a_duplicate_system_id_is_refused() {
    // A second `green` inserted before `red` and carrying `any_player`:
    // `red` stays the last, the duplicate is the only error.
    let text = AUTUMN26.replacen(
        "#{ id: \"red\", label: \"Red\",",
        "#{ id: \"green\", label: \"Second\", any_player: #{ height_min: 180 } },\n            #{ id: \"red\", label: \"Red\",",
        1,
    );
    case(
        "duplicate system",
        &text,
        LoadRule::DuplicateId {
            kind: IdKind::System,
            id: "green".to_string(),
        },
        "heights.systems[1]",
    );
}

#[test]
fn an_unknown_counts_as_is_refused() {
    case(
        "unknown counts_as",
        &edit(
            AUTUMN26,
            "\"regular\", when: #{ positions: [\"GK\"] }",
            "\"gollum\", when: #{ positions: [\"GK\"] }",
        ),
        LoadRule::UnknownId {
            kind: IdKind::Tier,
            id: "gollum".to_string(),
        },
        "tiers[3].counts_as",
    );
}

#[test]
fn an_unknown_free_group_is_refused() {
    case(
        "unknown free group",
        &edit(AUTUMN26, "free: #{ tricks: 3 }", "free: #{ trickz: 3 }"),
        LoadRule::UnknownId {
            kind: IdKind::Group,
            id: "trickz".to_string(),
        },
        "tiers[0].free.trickz",
    );
}

#[test]
fn an_unknown_quota_bracket_is_refused() {
    case(
        "unknown quota bracket",
        &edit(
            AUTUMN26,
            "quotas: #{ giga: 0, giant: 6",
            "quotas: #{ giga: 0, gaint: 6",
        ),
        LoadRule::UnknownId {
            kind: IdKind::Bracket,
            id: "gaint".to_string(),
        },
        "heights.systems[0].quotas.gaint",
    );
}

#[test]
fn an_unknown_suggestion_is_refused() {
    case(
        "unknown suggestion",
        &edit(
            AUTUMN26,
            "\"weak_foot\", \"injury_resistance\"",
            "\"weak_foot\", \"injury_resistans\"",
        ),
        LoadRule::UnknownSuggestion {
            entry: "injury_resistans".to_string(),
        },
        "suggestions[4]",
    );
}

#[test]
fn an_unknown_at_most_target_is_refused() {
    case(
        "unknown at_most target",
        &edit(
            AUTUMN26,
            "\"attacking_prowess\", \"defensive_prowess\"",
            "\"attaking_prowess\", \"defensive_prowess\"",
        ),
        LoadRule::UnknownTarget {
            name: "attaking_prowess".to_string(),
        },
        "stats.at_most[0]",
    );
}

/// The four targets besides the ability stats that a player may sit
/// below, each alone, so no name is pinned only by a ruleset listing it.
#[test]
fn every_non_stat_at_most_target_loads() {
    for name in [
        "form",
        "injury_resistance",
        "weak_foot_usage",
        "weak_foot_accuracy",
    ] {
        let ruleset = Ruleset::from_data_block(&edit(
            AUTUMN26,
            "at_most: [\"attacking_prowess\", \"defensive_prowess\",\n                  \"injury_resistance\", \"weak_foot_usage\", \"weak_foot_accuracy\"],",
            &format!("at_most: [\"{name}\"],"),
        ))
        .expect(name);
        assert_eq!(ruleset.stats.at_most, [name], "{name}");
    }
}

#[test]
fn the_com_card_group_is_reserved() {
    // `com` inside `card_groups` next to `tricks`: `free: com` and the
    // `com` suggestion still resolve, so only ReservedGroup fires.
    case(
        "reserved com group",
        &edit(
            AUTUMN26,
            "card_groups: #{\n",
            "card_groups: #{\n        com: [\"captaincy\"],\n",
        ),
        LoadRule::ReservedGroup,
        "card_groups.com",
    );
}

#[test]
fn a_count_with_counts_as_is_refused() {
    case(
        "count with counts_as",
        &edit(
            AUTUMN26,
            "#{ id: \"goalkeeper\", label: \"Goalkeeper\", counts_as",
            "#{ id: \"goalkeeper\", label: \"Goalkeeper\", count: 1, counts_as",
        ),
        LoadRule::CountWithCountsAs,
        "tiers[3]",
    );
}

#[test]
fn two_rest_tiers_are_refused() {
    // Silver loses its `count`: silver and regular are both rest tiers.
    case(
        "two rest tiers",
        &edit(
            AUTUMN26,
            "#{ id: \"silver\", label: \"Silver\", count: 2, rate",
            "#{ id: \"silver\", label: \"Silver\", rate",
        ),
        LoadRule::MoreThanOneRestTier,
        "tiers",
    );
}

#[test]
fn a_counts_as_chain_is_refused() {
    // `goalkeeper` naming itself: the tier `counts_as` has a `counts_as`
    // of its own.
    case(
        "counts_as chain",
        &edit(
            AUTUMN26,
            "counts_as: \"regular\"",
            "counts_as: \"goalkeeper\"",
        ),
        LoadRule::CountsAsChain,
        "tiers[3].counts_as",
    );
}

#[test]
fn counts_short_of_the_squad_are_refused() {
    // `regular` gets a `count`: no rest tier, and the sum is under
    // `squad_size`.
    case(
        "counts short of squad",
        &edit(
            AUTUMN26,
            "#{ id: \"regular\", label: \"Regular\", rate: 77",
            "#{ id: \"regular\", label: \"Regular\", count: 3, rate: 77",
        ),
        LoadRule::CountsShortOfSquad {
            total: 8,
            squad_size: 23,
        },
        "tiers",
    );
}

#[test]
fn counts_equal_to_the_squad_with_no_rest_tier_loads() {
    // `regular` given `count: 18` brings the counts to `squad_size` with
    // no rest tier: 1 + 2 + 2 + 18 = 23.
    Ruleset::from_data_block(&edit(
        AUTUMN26,
        "#{ id: \"regular\", label: \"Regular\", rate: 77",
        "#{ id: \"regular\", label: \"Regular\", count: 18, rate: 77",
    ))
    .expect("counts equal to squad_size load");
}

#[test]
fn a_system_other_than_the_last_without_a_condition_is_refused() {
    // Green (system 0 of 2) loses its `any_player`.
    case(
        "system without condition",
        &edit(AUTUMN26, "any_player: #{ height_min: 194 },\n", ""),
        LoadRule::SystemWithoutCondition,
        "heights.systems[0]",
    );
}

#[test]
fn empty_strings_are_refused() {
    for (name, broken, path) in [
        (
            "empty name",
            edit(AUTUMN26, "name: \"4cc Autumn 26\"", "name: \"\""),
            "name",
        ),
        (
            "empty tier label",
            edit(AUTUMN26, "label: \"Silver\"", "label: \"\""),
            "tiers[1].label",
        ),
        (
            "empty special label",
            edit(AUTUMN26, "label: \"Malicia is a free card\"", "label: \"\""),
            "specials[0].label",
        ),
    ] {
        case(name, &broken, LoadRule::Empty, path);
    }
}

#[test]
fn empty_strings_at_the_untested_sites_are_refused() {
    for (name, broken, path) in [
        (
            "empty bracket label",
            edit(AUTUMN26, "label: \"Tall\"", "label: \"\""),
            "heights.brackets[2].label",
        ),
        (
            "empty system label",
            edit(AUTUMN26, "label: \"Green\"", "label: \"\""),
            "heights.systems[0].label",
        ),
        (
            "empty team settings label",
            edit(
                AUTUMN26,
                "    suggestions:",
                "    team_settings: [#{ label: \"\" }],\n    suggestions:",
            ),
            "team_settings[0].label",
        ),
        (
            "empty forbid in then",
            edit(
                AUTUMN26,
                "forbid: \"Medals cannot play as GK\"",
                "forbid: \"\"",
            ),
            "specials[2].then.forbid",
        ),
    ] {
        case(name, &broken, LoadRule::Empty, path);
    }
    // An emptied `forbid` inside a `choice` option reports the option's
    // disallowed effect as well as the empty string.
    let err = Ruleset::from_data_block(&edit(
        AUTUMN26,
        "#{ free: #{ com: 1 } }, #{ a_positions: 1 }",
        "#{ free: #{ com: 1 }, forbid: \"\" }, #{ a_positions: 1 }",
    ))
    .expect_err("empty forbid in a choice option");
    assert_eq!(
        err,
        LoadError::Rules(vec![
            RuleError {
                path: "specials[11].then.choice[0].forbid".to_string(),
                rule: LoadRule::ChoiceOptionEffect,
            },
            RuleError {
                path: "specials[11].then.choice[0].forbid".to_string(),
                rule: LoadRule::Empty,
            },
        ]),
        "{err:?}"
    );
}

#[test]
fn an_empty_tier_id_makes_its_names_unknown() {
    // `gold` emptied: `Empty` at the tier, then `UnknownId` where every
    // `when` names it.
    let err = Ruleset::from_data_block(&edit(AUTUMN26, "id: \"gold\",", "id: \"\","))
        .expect_err("an emptied tier id");
    let unknown = || RuleError {
        path: String::new(),
        rule: LoadRule::UnknownId {
            kind: IdKind::Tier,
            id: "gold".to_string(),
        },
    };
    let mut e2 = unknown();
    e2.path = "specials[2].when.tiers[0]".to_string();
    let mut e6 = unknown();
    e6.path = "specials[6].when.tiers[0]".to_string();
    let mut e11 = unknown();
    e11.path = "specials[11].when.tiers[0]".to_string();
    let mut e12 = unknown();
    e12.path = "specials[12].when.tiers[0]".to_string();
    assert_eq!(
        err,
        LoadError::Rules(vec![
            RuleError {
                path: "tiers[0].id".to_string(),
                rule: LoadRule::Empty,
            },
            e2,
            e6,
            e11,
            e12,
        ]),
        "{err:?}"
    );
}

#[test]
fn an_empty_system_id_makes_its_names_unknown() {
    // `green` emptied: `Empty` at the system, then `UnknownId` where
    // `when` names it.
    let err = Ruleset::from_data_block(&edit(AUTUMN26, "id: \"green\",", "id: \"\","))
        .expect_err("an emptied system id");
    assert_eq!(
        err,
        LoadError::Rules(vec![
            RuleError {
                path: "heights.systems[0].id".to_string(),
                rule: LoadRule::Empty,
            },
            RuleError {
                path: "specials[10].when.systems[0]".to_string(),
                rule: LoadRule::UnknownId {
                    kind: IdKind::System,
                    id: "green".to_string(),
                },
            },
        ]),
        "{err:?}"
    );
}

#[test]
fn an_empty_card_groups_key_makes_its_names_unknown() {
    // `tricks` emptied: the tiers' `free` keys name it, the group is
    // gone, and `suggestions`' entry is unknown — every error in order.
    let err = Ruleset::from_data_block(&edit(
        AUTUMN26,
        "        tricks: [\"scissors_feint\"",
        "        \"\": [\"scissors_feint\"",
    ))
    .expect_err("an emptied group key");
    let unknown = |path: &str| RuleError {
        path: path.to_string(),
        rule: LoadRule::UnknownId {
            kind: IdKind::Group,
            id: "tricks".to_string(),
        },
    };
    assert_eq!(
        err,
        LoadError::Rules(vec![
            unknown("tiers[0].free.tricks"),
            unknown("tiers[1].free.tricks"),
            unknown("tiers[2].free.tricks"),
            unknown("tiers[4].free.tricks"),
            RuleError {
                path: "card_groups".to_string(),
                rule: LoadRule::Empty,
            },
            RuleError {
                path: "suggestions[1]".to_string(),
                rule: LoadRule::UnknownSuggestion {
                    entry: "tricks".to_string(),
                },
            },
        ]),
        "{err:?}"
    );
}

#[test]
fn an_empty_bracket_id_makes_its_quotas_unknown() {
    // `tall` emptied: `Empty` at its bracket, then every system's quota
    // naming it is `UnknownId`.
    let err = Ruleset::from_data_block(&edit(
        AUTUMN26,
        "#{ id: \"tall\",   label: \"Tall\",   exactly: [185] },",
        "#{ id: \"\",   label: \"Tall\",   exactly: [185] },",
    ))
    .expect_err("an emptied bracket id");
    assert_eq!(
        err,
        LoadError::Rules(vec![
            RuleError {
                path: "heights.brackets[2].id".to_string(),
                rule: LoadRule::Empty,
            },
            RuleError {
                path: "heights.systems[0].quotas.tall".to_string(),
                rule: LoadRule::UnknownId {
                    kind: IdKind::Bracket,
                    id: "tall".to_string(),
                },
            },
            RuleError {
                path: "heights.systems[1].quotas.tall".to_string(),
                rule: LoadRule::UnknownId {
                    kind: IdKind::Bracket,
                    id: "tall".to_string(),
                },
            },
        ]),
        "{err:?}"
    );
}

#[test]
fn brackets_out_of_order_are_refused() {
    // `manlet` and `mid` swapped: `up_to: 175` now sits under 180. The
    // quotas still resolve (`manlet` and `mid` both exist), so the only
    // error is the order.
    case(
        "brackets swapped",
        &AUTUMN26.replacen(
            "#{ id: \"manlet\", label: \"Manlet\", up_to: 175 },\n            #{ id: \"mid\",    label: \"Mid\",    exactly: [180] },",
            "#{ id: \"mid\",    label: \"Mid\",    exactly: [180] },\n            #{ id: \"manlet\", label: \"Manlet\", up_to: 175 },",
            1,
        ),
        LoadRule::BracketsNotAscending,
        "heights.brackets[1]",
    );
    // `tall` at 180, not above `mid`'s 180.
    case(
        "exactly equal to the previous",
        &edit(AUTUMN26, "exactly: [185]", "exactly: [180]"),
        LoadRule::BracketsNotAscending,
        "heights.brackets[2]",
    );
    // Unsorted but all above 180 and below 194: loads.
    Ruleset::from_data_block(&edit(AUTUMN26, "exactly: [185]", "exactly: [186, 184]"))
        .expect("an unsorted exactly in range loads");
}

#[test]
fn color_code_lengths_are_refused() {
    // Seven bytes: not a code.
    case(
        "name_colors short",
        &edit(
            AUTUMN26,
            "when: #{ tiers: [\"gold\", \"silver\", \"bronze\"], positions: [\"GK\"] }",
            "when: #{ tiers: [\"gold\", \"silver\", \"bronze\"], positions: [\"GK\"], name_colors: [\"cc9900f\"] }",
        ),
        LoadRule::ColorCodeLength,
        "specials[2].when.name_colors[0]",
    );
    // An empty entry inside `not` is this rule, not `Empty`.
    case(
        "empty entry inside not",
        &edit(
            AUTUMN26,
            "when: #{ captain: true }",
            "when: #{ not: #{ name_colors: [\"\"] } }",
        ),
        LoadRule::ColorCodeLength,
        "specials[1].when.not.name_colors[0]",
    );
    // Nine bytes: over is refused too; eight loads.
    case(
        "name_colors long",
        &edit(
            AUTUMN26,
            "when: #{ captain: true }",
            "when: #{ captain: true, name_colors: [\"cc9900ff0\"] }",
        ),
        LoadRule::ColorCodeLength,
        "specials[1].when.name_colors[0]",
    );
    Ruleset::from_data_block(&edit(
        AUTUMN26,
        "when: #{ captain: true }",
        "when: #{ captain: true, name_colors: [\"cc9900ff\"] }",
    ))
    .expect("an eight-byte code loads");
}

#[test]
fn a_tiers_name_color_is_eight_bytes() {
    // The same rule as a `name_colors` entry, at the tier's key.
    case(
        "tier name_color short",
        &edit(
            AUTUMN26,
            "#{ id: \"gold\", label: \"Gold\", count: 1, rate: 99",
            "#{ id: \"gold\", label: \"Gold\", count: 1, rate: 99, name_color: \"cc9900f\"",
        ),
        LoadRule::ColorCodeLength,
        "tiers[0].name_color",
    );
    Ruleset::from_data_block(&edit(
        AUTUMN26,
        "#{ id: \"gold\", label: \"Gold\", count: 1, rate: 99",
        "#{ id: \"gold\", label: \"Gold\", count: 1, rate: 99, name_color: \"cc9900ff\"",
    ))
    .expect("an eight-byte name_color loads");
}

#[test]
fn a_suggestion_word_as_a_group_is_reserved() {
    for word in [
        "cards",
        "a_positions",
        "weak_foot",
        "injury_resistance",
        "height",
        "free_cards",
        "choices",
    ] {
        case(
            word,
            &edit(
                AUTUMN26,
                "    card_groups: #{\n",
                &format!("    card_groups: #{{\n        {word}: [\"malicia\"],\n"),
            ),
            LoadRule::ReservedGroup,
            &format!("card_groups.{word}"),
        );
    }
}

#[test]
fn giga_first_fails_every_later_bracket() {
    // `giga` first: every bracket after it sits under its 199 — a check
    // against only the immediate predecessor would pass `[2]` to `[4]`.
    let err = Ruleset::from_data_block(&AUTUMN26.replacen(
        "#{ id: \"manlet\", label: \"Manlet\", up_to: 175 },\n            #{ id: \"mid\",    label: \"Mid\",    exactly: [180] },\n            #{ id: \"tall\",   label: \"Tall\",   exactly: [185] },\n            #{ id: \"giant\",  label: \"Giant\",  exactly: [194] },\n            #{ id: \"giga\",   label: \"Giga\",   exactly: [199] },",
        "#{ id: \"giga\",   label: \"Giga\",   exactly: [199] },\n            #{ id: \"manlet\", label: \"Manlet\", up_to: 175 },\n            #{ id: \"mid\",    label: \"Mid\",    exactly: [180] },\n            #{ id: \"tall\",   label: \"Tall\",   exactly: [185] },\n            #{ id: \"giant\",  label: \"Giant\",  exactly: [194] },",
        1,
    ))
    .expect_err("giga first");
    assert_eq!(
        err,
        LoadError::Rules(
            (1..=4)
                .map(|i| RuleError {
                    path: format!("heights.brackets[{i}]"),
                    rule: LoadRule::BracketsNotAscending,
                })
                .collect()
        ),
        "{err:?}"
    );
}

#[test]
fn counts_over_the_squad_are_refused() {
    case(
        "counts exceed squad",
        &edit(AUTUMN26, "squad_size: 23", "squad_size: 4"),
        LoadRule::CountsExceedSquad {
            total: 5,
            squad_size: 4,
        },
        "tiers",
    );
}

#[test]
fn a_preset_out_of_range_is_refused() {
    case(
        "preset out of range",
        &edit(AUTUMN26, "fielded_at_a: [1]", "fielded_at_a: [4]"),
        LoadRule::PresetOutOfRange { preset: 4 },
        "universal.fielded_at_a[0]",
    );
}

#[test]
fn a_shape_not_ten_is_refused() {
    case(
        "shape not ten",
        &edit(
            AUTUMN26,
            "suggestions:",
            "team_settings: [#{ label: \"no 3-4-4\", forbidden_shapes: [[3, 4, 4]] }],\n    suggestions:",
        ),
        LoadRule::ShapeNotTen { shape: [3, 4, 4] },
        "team_settings[0].forbidden_shapes[0]",
    );
}

/// Counts summing to exactly `squad_size` load — `>` at the boundary.
#[test]
fn counts_at_the_squad_size_load() {
    let ruleset = Ruleset::from_data_block(&edit(AUTUMN26, "squad_size: 23", "squad_size: 5"))
        .expect("counts at the boundary load");
    assert_eq!(ruleset.squad_size, 5);
}

/// A `forbidden_shapes` entry summing to 10 loads — the `+` mutants would
/// refuse it.
#[test]
fn a_forbidden_shape_of_ten_loads() {
    Ruleset::from_data_block(&edit(
        AUTUMN26,
        "suggestions:",
        "team_settings: [#{ label: \"no 3-4-3\", forbidden_shapes: [[3, 4, 3]] }],\n    suggestions:",
    ))
    .expect("a shape adding to ten loads");
}

/// `LoadError::Rules` displays each `RuleError` as `path: message`,
/// joined by `"; "`.
#[test]
fn rules_error_texts_name_path_and_message() {
    let err = LoadError::Rules(vec![
        RuleError {
            path: "tiers[1]".to_string(),
            rule: LoadRule::DuplicateId {
                kind: IdKind::Tier,
                id: "gold".to_string(),
            },
        },
        RuleError {
            path: "team_settings[0].forbidden_shapes[0]".to_string(),
            rule: LoadRule::ShapeNotTen { shape: [3, 4, 4] },
        },
    ]);
    assert_eq!(
        err.to_string(),
        "tiers[1]: tier id \"gold\" is used twice; \
         team_settings[0].forbidden_shapes[0]: shape [3, 4, 4] does not add up to 10"
    );
    for (kind, expected) in [
        (IdKind::Tier, "tier id"),
        (IdKind::Bracket, "bracket id"),
        (IdKind::System, "system id"),
        (IdKind::Group, "group id"),
    ] {
        let err = LoadRule::UnknownId {
            kind,
            id: "x".to_string(),
        }
        .to_string();
        assert!(err.contains(expected), "{expected}: {err}");
    }
    for (rule, expected) in [
        (LoadRule::CountsAsChain, "counts_as"),
        (LoadRule::SystemWithoutCondition, "any_player"),
        (LoadRule::Empty, "empty"),
        (LoadRule::BracketsNotAscending, "bracket"),
        (LoadRule::ColorCodeLength, "eight"),
    ] {
        let text = rule.to_string();
        assert!(text.contains(expected), "{rule:?}: {text}");
    }
    // `ReservedGroup`'s one text names both reasons.
    let text = LoadRule::ReservedGroup.to_string();
    assert!(
        text.contains("com") && text.contains("suggestions"),
        "{text}"
    );
    let text = LoadRule::CountsShortOfSquad {
        total: 8,
        squad_size: 23,
    }
    .to_string();
    let total_at = text.find('8').expect("the total in the text");
    let size_at = text.find("23").expect("the squad size in the text");
    assert!(total_at < size_at, "{text}");
}

/// A card in two `card_groups` is one `GroupsOverlap` error per pair,
/// at the later group, naming the earlier.
#[test]
fn a_card_in_two_groups_is_refused() {
    // `scissors_feint` in `broad` and `narrow`; `flip_flap` is in
    // `broad` alone, so the only error is the pair's.
    case(
        "overlapping groups",
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77 }],
            card_groups: #{
                broad: [\"scissors_feint\", \"flip_flap\"],
                narrow: [\"scissors_feint\"],
            },
        };",
        LoadRule::GroupsOverlap {
            card: "scissors_feint".to_string(),
            other: "broad".to_string(),
        },
        "card_groups.narrow",
    );
}

/// A COM style in a `card_groups` entry overlaps the built-in `com`
/// group, which holds the seven.
#[test]
fn a_com_style_in_a_group_is_refused() {
    case(
        "com style in a group",
        "const RULESET = #{
            schema: 1, name: \"x\", squad_size: 23,
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77 }],
            card_groups: #{ shots: [\"long_ranger\"] },
        };",
        LoadRule::GroupsOverlap {
            card: "long_ranger".to_string(),
            other: "com".to_string(),
        },
        "card_groups.shots",
    );
}

/// The smallest ruleset with a medal system: `gold` counted by the
/// systems, `regular` the rest of the squad.
const MEDAL: &str = "const RULESET = #{
    schema: 1, name: \"x\", squad_size: 23,
    universal: #{ max_skill_cards: 10 },
    tiers: [
        #{ id: \"gold\", label: \"Gold\", rate: 99 },
        #{ id: \"regular\", label: \"Regular\", rate: 77 },
    ],
    medal_systems: [#{ label: \"GGG\", counts: #{ gold: 3 } }],
};";

/// A tier a medal system counts is not the rest of the squad: `gold`
/// counted by GGG and `regular` the rest loads, where counting `gold`
/// too would be `MoreThanOneRestTier`.
#[test]
fn a_medal_tier_is_not_the_rest_of_the_squad() {
    Ruleset::from_data_block(MEDAL).expect("the medal file loads");
}

#[test]
fn a_medal_systems_empty_label_is_refused() {
    case(
        "empty medal label",
        &edit(MEDAL, "label: \"GGG\"", "label: \"\""),
        LoadRule::Empty,
        "medal_systems[0].label",
    );
}

/// A medal system counting no tier loads: a tier a system does not
/// name counts 0 in it, so an empty system is "no medal players", not
/// one every team matches (the Ruleset editor can leave one so by
/// clearing a tier from the systems). `gold` counted at 3, since
/// uncounted by any system it would be a second rest tier.
#[test]
fn a_medal_system_counting_no_tier_loads() {
    let counted = edit(
        MEDAL,
        "id: \"gold\", label: \"Gold\", rate",
        "id: \"gold\", label: \"Gold\", count: 3, rate",
    );
    Ruleset::from_data_block(&edit(&counted, "counts: #{ gold: 3 }", "counts: #{}"))
        .expect("an empty medal system loads");
}

#[test]
fn a_medal_system_naming_an_unknown_tier_is_refused() {
    case(
        "unknown medal tier",
        &edit(
            MEDAL,
            "counts: #{ gold: 3 }",
            "counts: #{ gold: 3, gollum: 1 }",
        ),
        LoadRule::UnknownId {
            kind: IdKind::Tier,
            id: "gollum".to_string(),
        },
        "medal_systems[0].counts.gollum",
    );
}

#[test]
fn a_medal_tier_with_a_count_is_refused() {
    // `gold` gets its own `count`: the systems decide it, not the tier.
    case(
        "medal tier with count",
        &edit(
            MEDAL,
            "id: \"gold\", label: \"Gold\", rate",
            "id: \"gold\", label: \"Gold\", count: 3, rate",
        ),
        LoadRule::MedalTierCounted,
        "tiers[0]",
    );
}

#[test]
fn a_medal_tier_with_counts_as_is_refused() {
    case(
        "medal tier with counts_as",
        &edit(
            MEDAL,
            "id: \"gold\", label: \"Gold\", rate",
            "id: \"gold\", label: \"Gold\", counts_as: \"regular\", rate",
        ),
        LoadRule::MedalTierCountsAs,
        "tiers[0].counts_as",
    );
}

#[test]
fn a_counts_as_naming_a_medal_tier_is_refused() {
    // A third tier `counts_as` the medal-counted `gold`.
    case(
        "counts_as a medal tier",
        &edit(
            MEDAL,
            "#{ id: \"regular\", label: \"Regular\", rate: 77 },",
            "#{ id: \"keeper\", label: \"Keeper\", counts_as: \"gold\", rate: 77 },\n        #{ id: \"regular\", label: \"Regular\", rate: 77 },",
        ),
        LoadRule::CountsAsMedalTier,
        "tiers[1].counts_as",
    );
}

#[test]
fn a_medal_systems_counts_over_the_squad_are_refused() {
    case(
        "medal counts exceed squad",
        &edit(MEDAL, "counts: #{ gold: 3 }", "counts: #{ gold: 30 }"),
        LoadRule::CountsExceedSquad {
            total: 30,
            squad_size: 23,
        },
        "medal_systems[0]",
    );
}

#[test]
fn a_medal_systems_counts_short_of_the_squad_are_refused() {
    // `regular` counted at 20: no tier is the rest, so every system's
    // counts must add to `squad_size`.
    let counted = edit(
        MEDAL,
        "id: \"regular\", label: \"Regular\", rate",
        "id: \"regular\", label: \"Regular\", count: 20, rate",
    );
    case(
        "medal counts short of squad",
        &edit(&counted, "counts: #{ gold: 3 }", "counts: #{ gold: 2 }"),
        LoadRule::CountsShortOfSquad {
            total: 22,
            squad_size: 23,
        },
        "medal_systems[0]",
    );
    // 20 + 3 = 23 loads.
    Ruleset::from_data_block(&counted).expect("counts equal to squad_size load");
}

#[test]
fn by_quotas_counts_as_a_systems_condition() {
    // Green loses `any_player` but is `by_quotas`: the quota check
    // itself decides whether the system holds.
    Ruleset::from_data_block(&edit(
        AUTUMN26,
        "any_player: #{ height_min: 194 },\n",
        "by_quotas: true,\n",
    ))
    .expect("a non-last system with by_quotas needs no condition");
    // With no quotas there is nothing to fail: Green would hold for every
    // team and Red never apply.
    case(
        "by_quotas without quotas",
        &edit(
            AUTUMN26,
            "any_player: #{ height_min: 194 },\n               quotas: #{ giga: 0, giant: 6, tall: 6, mid: 5, manlet: 6 } }",
            "by_quotas: true }",
        ),
        LoadRule::SystemWithoutCondition,
        "heights.systems[0]",
    );
}

#[test]
fn a_last_system_with_by_quotas_is_refused() {
    case(
        "last system by_quotas",
        &edit(
            AUTUMN26,
            "#{ id: \"red\", label: \"Red\",",
            "#{ id: \"red\", label: \"Red\", by_quotas: true,",
        ),
        LoadRule::LastSystemHasCondition,
        "heights.systems[1].by_quotas",
    );
}

/// A `choice` option may not hold `stat_warning`: it raises no limit, so no
/// player could ever be found to use it.
#[test]
fn a_stat_warning_in_a_choice_option_is_refused() {
    case(
        "stat_warning option",
        &edit(
            AUTUMN26,
            "#{ free: #{ com: 1 } }, #{ a_positions: 1 }",
            "#{ free: #{ com: 1 }, stat_warning: 3 }, #{ a_positions: 1 }",
        ),
        LoadRule::ChoiceOptionEffect,
        "specials[11].then.choice[0].stat_warning",
    );
}

/// A `stat_warning` per-tier map names existing tiers, as every other
/// per-tier effect's does.
#[test]
fn a_stat_warning_naming_an_unknown_tier_is_refused() {
    case(
        "stat_warning unknown tier",
        &edit(
            AUTUMN26,
            "then: #{ free_cards: [\"malicia\"] }",
            "then: #{ free_cards: [\"malicia\"], stat_warning: #{ gollum: 3 } }",
        ),
        LoadRule::UnknownId {
            kind: IdKind::Tier,
            id: "gollum".to_string(),
        },
        "specials[0].then.stat_warning.gollum",
    );
}

/// The smallest ruleset with an `extra_height` system: two `exactly`
/// brackets, quotas adding to `squad_size`.
const POOL: &str = "const RULESET = #{
    schema: 1, name: \"x\", squad_size: 23,
    universal: #{ max_skill_cards: 10 },
    tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77 }],
    heights: #{
        brackets: [
            #{ id: \"low\", label: \"Low\", exactly: [175] },
            #{ id: \"high\", label: \"High\", exactly: [185] },
        ],
        systems: [#{ id: \"a\", label: \"A\", quotas: #{ high: 7, low: 16 },
                      extra_height: #{ total: 50, max: 210 } }],
    },
};";

#[test]
fn an_extra_height_quotas_a_bracket_of_no_one_height_is_refused() {
    for (name, heights) in [
        ("up_to", "up_to: 185"),
        ("two heights", "exactly: [185, 190]"),
    ] {
        case(
            name,
            &edit(
                POOL,
                "label: \"High\", exactly: [185]",
                &format!("label: \"High\", {heights}"),
            ),
            LoadRule::ExtraHeightQuotaNotExact,
            "heights.systems[0].quotas.high",
        );
    }
}

#[test]
fn an_extra_height_quotas_off_the_squad_are_refused() {
    for (name, quotas, total) in [
        ("short", "high: 7, low: 15", 22),
        ("over", "high: 8, low: 16", 24),
    ] {
        case(
            name,
            &edit(
                POOL,
                "quotas: #{ high: 7, low: 16 }",
                &format!("quotas: #{{ {quotas} }}"),
            ),
            LoadRule::ExtraHeightQuotasNotSquad {
                total,
                squad_size: 23,
            },
            "heights.systems[0].quotas",
        );
    }
    Ruleset::from_data_block(POOL).expect("quotas at squad_size load");
}

/// A small ruleset that loads, `extra` (`key: value, `) among its keys.
fn small_with(extra: &str) -> String {
    format!(
        "const RULESET = #{{ schema: 1, name: \"x\", {extra}squad_size: 23, universal: #{{ max_skill_cards: 10 }}, tiers: [#{{ id: \"regular\", label: \"Regular\", rate: 77 }}] }};"
    )
}

/// `bytes` as a `logo` key, base64.
fn logo_key(bytes: &[u8]) -> String {
    use base64::Engine as _;
    format!(
        "logo: \"{}\", ",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

/// The PNG signature followed by zeros up to `len` bytes.
fn png_of_len(len: usize) -> Vec<u8> {
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.resize(len, 0);
    bytes
}

#[test]
fn a_logo_that_is_not_base64_is_refused() {
    for logo in ["data:image/png;base64,iVBORw0KGgo=", "iVBORw0K Ggo="] {
        case(
            logo,
            &small_with(&format!("logo: \"{logo}\", ")),
            LoadRule::LogoNotBase64,
            "logo",
        );
    }
}

#[test]
fn a_logo_that_is_not_a_png_is_refused() {
    case(
        "not a png",
        &small_with(&logo_key(b"GIF89a, not a PNG")),
        LoadRule::LogoNotPng,
        "logo",
    );
}

#[test]
fn a_logo_over_64_kb_is_refused() {
    case(
        "over 64 KB",
        &small_with(&logo_key(&png_of_len(65537))),
        LoadRule::LogoTooLarge { bytes: 65537 },
        "logo",
    );
}

#[test]
fn a_page_that_is_not_a_web_address_is_refused() {
    for page in ["www.example", "ftp://x", "https://", "https://a b"] {
        case(
            page,
            &small_with(&format!("page: \"{page}\", ")),
            LoadRule::PageNotWebAddress,
            "page",
        );
    }
}

/// The limits' edges load: a logo of exactly 64 KB, a site's bare
/// address.
#[test]
fn a_64_kb_logo_and_a_bare_web_address_load() {
    let page = "https://implyingrigged.info/";
    let text = small_with(&format!(
        "page: \"{page}\", {}",
        logo_key(&png_of_len(65536))
    ));
    let ruleset = Ruleset::from_data_block(&text).expect("the edges load");
    assert_eq!(ruleset.page.as_deref(), Some(page));
    assert!(ruleset.logo.is_some());
}

#[test]
fn an_http_page_loads() {
    let page = "http://4cc.example/dbc7";
    let ruleset = Ruleset::from_data_block(&small_with(&format!("page: \"{page}\", ")))
        .expect("an http:// page loads");
    assert_eq!(ruleset.page.as_deref(), Some(page));
}

/// A real event logo: DBC7's, as the drafts draw it.
const DBC7_LOGO: &[u8] = include_bytes!("../../../../../../docs/prototypes/assets/logos/dbc7.png");

#[test]
fn set_logo_stores_a_png_that_logo_png_gives_back() {
    let mut ruleset = Ruleset::from_data_block(&small_with("")).expect("the small ruleset loads");
    assert_eq!(ruleset.logo, None);
    assert_eq!(ruleset.logo_png(), None, "no logo, no bytes");

    ruleset.set_logo(DBC7_LOGO).expect("a real PNG is taken");
    assert_eq!(ruleset.logo_png().as_deref(), Some(DBC7_LOGO));
    assert_eq!(
        ruleset.check(),
        Vec::new(),
        "the stored logo passes the load rules"
    );
}

#[test]
fn logo_png_is_none_for_text_that_is_not_base64() {
    let mut ruleset = Ruleset::from_data_block(&small_with("")).expect("the small ruleset loads");
    ruleset.logo = Some("iVBORw0K Ggo=".to_string());
    assert_eq!(ruleset.logo_png(), None);
}

#[test]
fn set_logo_refuses_a_non_png_and_an_oversize_png_keeping_the_logo() {
    use base64::Engine as _;
    let mut ruleset = Ruleset::from_data_block(&small_with("")).expect("the small ruleset loads");
    let kept = base64::engine::general_purpose::STANDARD.encode(DBC7_LOGO);
    ruleset.logo = Some(kept.clone());

    assert_eq!(
        ruleset.set_logo(b"GIF89a, not a PNG"),
        Err(LogoRefusal::NotPng)
    );
    assert_eq!(ruleset.logo.as_deref(), Some(kept.as_str()));

    assert_eq!(
        ruleset.set_logo(&png_of_len(65537)),
        Err(LogoRefusal::TooLarge { bytes: 65537 })
    );
    assert_eq!(ruleset.logo.as_deref(), Some(kept.as_str()));

    // The limit's edge is taken, as a loaded logo's is.
    ruleset
        .set_logo(&png_of_len(65536))
        .expect("exactly 64 KB is taken");
    assert_eq!(ruleset.logo_png().map(|png| png.len()), Some(65536));
}
