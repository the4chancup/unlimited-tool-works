use super::*;
use std::collections::BTreeMap;

#[test]
fn an_unknown_top_level_key_is_schema() {
    let err = schema("#{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, frob: 1 }");
    assert!(matches!(err, LoadError::Schema { .. }), "{err:?}");
}

#[test]
fn an_unknown_tier_key_is_schema() {
    let err = schema(
        "#{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"gold\", label: \"Gold\", rate: 99, frob: 1 }] }",
    );
    assert!(matches!(err, LoadError::Schema { .. }), "{err:?}");
}

#[test]
fn a_missing_rate_is_schema() {
    let err = schema(
        "#{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"gold\", label: \"Gold\" }] }",
    );
    assert!(matches!(err, LoadError::Schema { .. }), "{err:?}");
}

#[test]
fn an_unknown_position_label_is_schema() {
    let err = schema(
        "#{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"gold\", label: \"Gold\", rate: 99, when: #{ positions: [\"GKK\"] } }] }",
    );
    assert!(
        matches!(err, LoadError::Schema { ref message, .. } if message.contains("GKK")),
        "{err:?}"
    );
}

#[test]
fn unknown_keys_and_labels_inside_specials_are_schema() {
    for (from, to, name) in [
        (
            "then: #{ forbid: \"Medals cannot play as GK\" }",
            "then: #{ forbid: \"Medals cannot play as GK\", free_card: 1 }",
            "free_card",
        ),
        (
            "when: #{ positions: [\"GK\"], height_min: 189, height_max: 189 }",
            "when: #{ postions: [\"GK\"], height_min: 189, height_max: 189 }",
            "postions",
        ),
        (
            "free_cards: [\"malicia\"]",
            "free_cards: [\"malicias\"]",
            "malicias",
        ),
        (
            "when: #{ captain: true }",
            "when: #{ captain: true, playing_styles: [\"nope\"] }",
            "nope",
        ),
    ] {
        let err = Ruleset::from_data_block(&edit(AUTUMN26, from, to)).expect_err(name);
        assert!(
            matches!(err, LoadError::Schema { ref message, .. } if message.contains(name)),
            "{name}: {err:?}"
        );
    }
}

#[test]
fn a_bracket_with_both_or_neither_height_form_is_schema() {
    for heights in ["up_to: 175, exactly: [185]", ""] {
        let err = schema(&format!(
            "#{{ schema: 1, name: \"x\", universal: #{{ max_skill_cards: 10 }}, heights: #{{ brackets: [#{{ id: \"x\", label: \"x\", {heights} }}], systems: [] }} }}"
        ));
        // `RawBracket`'s error names both keys, whichever half is wrong.
        assert!(
            matches!(
                err,
                LoadError::Schema { ref message, .. }
                    if message.contains("up_to") && message.contains("exactly")
            ),
            "{heights}: {err:?}"
        );
    }
}

#[test]
fn an_empty_exactly_is_schema() {
    let err = schema(
        "#{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, heights: #{ brackets: [#{ id: \"x\", label: \"x\", exactly: [] }], systems: [] } }",
    );
    assert!(
        matches!(
            err,
            LoadError::Schema { ref message, .. } if message.contains("`exactly` lists no height")
        ),
        "{err:?}"
    );
}

#[test]
fn per_tier_deserializes_in_both_forms() {
    let ruleset = Ruleset::from_data_block(
        "const RULESET = #{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77 }], specials: [#{ label: \"x\", then: #{ stat_bonus: #{ all: -3, dribbling: #{ regular: 5 } } } }] };",
    )
    .expect("both forms parse");
    let Some(&PerTier::All(-3)) = ruleset.specials[0].then.stat_bonus.get(&StatOrAll::All) else {
        panic!("all is not All");
    };
    let Some(PerTier::ByTier(map)) = ruleset.specials[0].then.stat_bonus.get(&StatOrAll::Stat(
        Stat::try_from("dribbling".to_string()).expect("a stat"),
    )) else {
        panic!("the stat is not ByTier");
    };
    assert_eq!(map.get("regular"), Some(&5));
}

/// A tier's `stats` names ability stats only: `form` is the tier's own
/// `form` key, not a stat — `stats: #{ form: 8 }` is `Schema` naming
/// `"form"`.
#[test]
fn a_form_in_a_stats_map_is_schema() {
    let err = schema(
        "#{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"gold\", label: \"Gold\", rate: 99, stats: #{ form: 8 } }] }",
    );
    assert!(
        matches!(err, LoadError::Schema { ref message, .. } if message.contains("\"form\"")),
        "{err:?}"
    );
}

/// Every required key absent is `Schema` naming it: a violation shows
/// the special's label, and an absent `universal` would default
/// `max_skill_cards` to 0.
#[test]
fn a_required_key_absent_is_schema() {
    for (name, text, key) in [
        (
            "schema",
            "const RULESET = #{ name: \"x\", universal: #{ max_skill_cards: 10 } };"
                .to_string(),
            "schema",
        ),
        (
            "name",
            "const RULESET = #{ schema: 1, universal: #{ max_skill_cards: 10 } };"
                .to_string(),
            "name",
        ),
        ("a tier's id", edit(AUTUMN26, "id: \"gold\", ", ""), "id"),
        ("a tier's label", edit(AUTUMN26, "label: \"Gold\", ", ""), "label"),
        ("a bracket's id", edit(AUTUMN26, "id: \"manlet\", ", ""), "id"),
        ("a bracket's label", edit(AUTUMN26, "label: \"Manlet\", ", ""), "label"),
        ("a system's id", edit(AUTUMN26, "id: \"green\", ", ""), "id"),
        ("a system's label", edit(AUTUMN26, "label: \"Green\", ", ""), "label"),
        (
            "universal",
            "const RULESET = #{ schema: 1, name: \"x\" };".to_string(),
            "universal",
        ),
        (
            "universal.max_skill_cards",
            "const RULESET = #{ schema: 1, name: \"x\", universal: #{ } };"
                .to_string(),
            "max_skill_cards",
        ),
        (
            "a special's label",
            edit(AUTUMN26, "label: \"Malicia is a free card\", ", ""),
            "label",
        ),
        (
            "a team settings entry's label",
            "const RULESET = #{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, team_settings: [#{ }] };"
                .to_string(),
            "label",
        ),
        (
            "weight.floor",
            edit(AUTUMN26, "floor: 30, ", ""),
            "floor",
        ),
        (
            "weight.below",
            edit(AUTUMN26, "below: 129, ", ""),
            "below",
        ),
        (
            "weight.above",
            edit(AUTUMN26, ", above: 81", ""),
            "above",
        ),
        (
            "a medal system's counts",
            edit(
                AUTUMN26,
                "    card_groups:",
                "    medal_systems: [#{ label: \"GGG\" }],\n    card_groups:",
            ),
            "counts",
        ),
        (
            "extra_height.total",
            edit(
                AUTUMN26,
                "#{ id: \"red\", label: \"Red\",",
                "#{ id: \"red\", label: \"Red\", extra_height: #{ max: 210 },",
            ),
            "total",
        ),
        (
            "extra_height.max",
            edit(
                AUTUMN26,
                "#{ id: \"red\", label: \"Red\",",
                "#{ id: \"red\", label: \"Red\", extra_height: #{ total: 50 },",
            ),
            "max",
        ),
    ] {
        let err = Ruleset::from_data_block(&text).expect_err(name);
        assert!(
            matches!(err, LoadError::Schema { ref message, .. } if message.contains(key)),
            "{name}: {err:?}"
        );
    }
}

/// `Schema`'s `path` is the failing key's path, spelled
/// `tiers[2].free.tricks`: a wrong-typed value's own key, an unknown
/// key itself, and a missing key's parent (the root is `.`).
#[test]
fn a_schema_error_carries_its_key_path() {
    for (name, text, path) in [
        (
            "a wrong-typed value",
            "#{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"gold\", label: \"Gold\", rate: 99 }, #{ id: \"silver\", label: \"Silver\", rate: 88 }, #{ id: \"bronze\", label: \"Bronze\", rate: 77, free: #{ tricks: \"two\" } }] }",
            "tiers[2].free.tricks",
        ),
        (
            "an unknown top-level key",
            "#{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, frob: 1 }",
            "frob",
        ),
        (
            "an unknown tier key",
            "#{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"gold\", label: \"Gold\", rate: 99, frob: 1 }] }",
            "tiers[0].frob",
        ),
        (
            "a missing tier key",
            "#{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"gold\", label: \"Gold\" }] }",
            "tiers[0]",
        ),
        (
            "a missing top-level key",
            "#{ name: \"x\", universal: #{ max_skill_cards: 10 } }",
            ".",
        ),
    ] {
        match schema(text) {
            LoadError::Schema { path: got, .. } => assert_eq!(got, path, "{name}"),
            other => panic!("{name}: {other:?}"),
        }
    }
}

/// Every squad is 23, so the squad's size is no key of the schema: a
/// file that still sets it is refused like any key the schema lacks,
/// with the key as its path.
#[test]
fn a_squad_key_is_refused_as_unknown() {
    match schema("#{ schema: 1, name: \"x\", squad_size: 23, universal: #{ max_skill_cards: 10 } }")
    {
        LoadError::Schema { path, .. } => assert_eq!(path, "squad_size"),
        other => panic!("{other:?}"),
    }
}

/// The proposal's fields deserialize: a `ranges` key is a stat label or
/// one of the six `PlayerValue` names, `strong_foot` the Team TOML foot
/// labels, `any` a list of conditions, `form`/`injury_resistance` the
/// `PerTier` spellings.
#[test]
fn the_proposal_fields_deserialize() {
    let ruleset = Ruleset::from_data_block(
        "const RULESET = #{
            schema: 1, name: \"x\",
            universal: #{ max_skill_cards: 10 },
            tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77, name_color: \"cc9900ff\" }],
            specials: [#{ label: \"x\",
                when: #{ ranges: #{ speed: [90, 99], age: [15, 20] }, strong_foot: \"left\",
                         any: [#{ captain: true }, #{ positions: [\"GK\"] }] },
                then: #{ form: #{ regular: 1 }, injury_resistance: -1 } }],
        };",
    )
    .expect("the fields parse");
    let tier = &ruleset.tiers[0];
    assert_eq!(tier.name_color.as_deref(), Some("cc9900ff"));
    let when = &ruleset.specials[0].when;
    assert_eq!(
        when.ranges.get(&PlayerValue::Stat(
            Stat::try_from("speed".to_string()).expect("a stat")
        )),
        Some(&(90, 99))
    );
    assert_eq!(when.ranges.get(&PlayerValue::Age), Some(&(15, 20)));
    assert_eq!(when.strong_foot, Some(Foot::Left));
    assert_eq!(when.any.len(), 2);
    assert!(when.any[0].captain == Some(true) && when.any[1].positions.len() == 1);
    let then = &ruleset.specials[0].then;
    assert_eq!(
        then.form,
        Some(PerTier::ByTier(BTreeMap::from([(
            "regular".to_string(),
            1
        )])))
    );
    assert_eq!(then.injury_resistance, Some(PerTier::All(-1)));
}

/// A `ranges` key that is no `PlayerValue` name, a foot that is not
/// `right`/`left` and a `name_color` that is not a string are `Schema`.
#[test]
fn bad_proposal_field_values_are_schema() {
    for (name, snippet) in [
        ("ranges key", "when: #{ ranges: #{ pace: [90, 99] } }"),
        ("strong_foot", "when: #{ strong_foot: \"both\" }"),
        ("name_color", "name_color: 5"),
    ] {
        let err = Ruleset::from_data_block(&format!(
            "const RULESET = #{{ schema: 1, name: \"x\", universal: #{{ max_skill_cards: 10 }}, tiers: [#{{ id: \"regular\", label: \"Regular\", rate: 77, {snippet} }}] }};"
        ))
        .expect_err(name);
        assert!(matches!(err, LoadError::Schema { .. }), "{name}: {err:?}");
    }
}

/// `key` spells the name `TryFrom` reads back: the six non-stat values
/// and one stat round-trip, and `NON_STAT_TARGETS` is the four
/// below-target names' `key()`s (`key` cannot be `const` —
/// `Stat::label` goes through `usize::from` — so the array is pinned
/// by this test instead).
#[test]
fn a_player_value_round_trips_through_its_ranges_key() {
    let stat = Stat::try_from(ABILITY_STATS[0].to_string()).expect("an ability stat");
    for value in [
        PlayerValue::Age,
        PlayerValue::Weight,
        PlayerValue::Form,
        PlayerValue::InjuryResistance,
        PlayerValue::WeakFootUsage,
        PlayerValue::WeakFootAccuracy,
        PlayerValue::Stat(stat),
    ] {
        assert_eq!(PlayerValue::try_from(value.key().to_string()), Ok(value));
    }
    assert_eq!(
        NON_STAT_TARGETS,
        [
            PlayerValue::Form.key(),
            PlayerValue::InjuryResistance.key(),
            PlayerValue::WeakFootUsage.key(),
            PlayerValue::WeakFootAccuracy.key()
        ]
    );
}

/// A present `weak_foot` names both halves (`#{ usage: 4 }` alone would
/// cap accuracy at 0, below the game's 1): `Schema` on a tier, in a
/// special's `then` and in a `choice` option. A tier with no `weak_foot`
/// key at all still loads (the field's own default).
#[test]
fn a_partial_weak_foot_is_schema() {
    for (name, text) in [
        (
            "a tier",
            "const RULESET = #{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77, weak_foot: #{ usage: 4 } }] };",
        ),
        (
            "a special's then",
            "const RULESET = #{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77 }], specials: [#{ label: \"x\", then: #{ weak_foot: #{ usage: 4 } } }] };",
        ),
        (
            "a choice option",
            "const RULESET = #{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77 }], specials: [#{ label: \"x\", then: #{ choice: [#{ weak_foot: #{ usage: 4 } }] } }] };",
        ),
    ] {
        let err = Ruleset::from_data_block(text).expect_err(name);
        assert!(matches!(err, LoadError::Schema { .. }), "{name}: {err:?}");
    }
    Ruleset::from_data_block(
        "const RULESET = #{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77 }] };",
    )
    .expect("a tier without a weak_foot key loads");
}

#[test]
fn schema_two_is_unknown_schema() {
    let err = schema("#{ schema: 2, name: \"x\", universal: #{ max_skill_cards: 10 } }");
    assert_eq!(err, LoadError::UnknownSchema { schema: 2 }, "{err:?}");
}

/// A `let` is not a constant: a function could not read
/// `global::RULESET`, so the entry is `NoRuleset` too.
#[test]
fn a_let_ruleset_is_no_ruleset() {
    assert_eq!(
        Ruleset::from_data_block(
            "let RULESET = #{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77 }] };"
        )
        .expect_err("a let"),
        LoadError::NoRuleset
    );
}

/// The scope keeps the last write of `RULESET`, the one `global::RULESET`
/// reads: a second `const` wins, and a `let` above a `const` loads.
#[test]
fn the_last_constant_ruleset_wins() {
    let two = format!(
        "{}\n{}",
        AUTUMN26.replacen("name: \"4cc Autumn 26\"", "name: \"a\"", 1),
        AUTUMN26.replacen("name: \"4cc Autumn 26\"", "name: \"b\"", 1)
    );
    assert_eq!(
        Ruleset::from_data_block(&two).expect("the second const"),
        Ruleset::from_data_block(&AUTUMN26.replacen("name: \"4cc Autumn 26\"", "name: \"b\"", 1))
            .expect("b loads")
    );
    // `let` above `const`: the `const` wins and loads.
    let let_then_const = format!("let RULESET = #{{}};\n{}", AUTUMN26);
    assert!(
        Ruleset::from_data_block(&let_then_const).is_ok(),
        "a let above a const loads"
    );
}

#[test]
fn no_ruleset_is_no_ruleset() {
    assert_eq!(
        Ruleset::from_data_block("const OTHER = 1;").expect_err("no map"),
        LoadError::NoRuleset
    );
    assert_eq!(
        Ruleset::from_data_block("const RULESET = 1;").expect_err("not a map"),
        LoadError::NoRuleset
    );
}

/// `import` has no resolver on the bare engine; `print` exists under the
/// standard package and a data block containing it loads.
#[test]
fn import_is_refused_but_print_runs() {
    let err = Ruleset::from_data_block("import \"x\" as m;\nconst RULESET = #{ schema: 1 };")
        .expect_err("import has no resolver");
    assert!(matches!(err, LoadError::Parse { .. }), "{err:?}");
    Ruleset::from_data_block(
        "print(\"x\");\nconst RULESET = #{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77 }] };",
    )
    .expect("the standard package's print runs before the data block");
}

#[test]
fn a_stray_token_is_parse_with_its_position() {
    let err = Ruleset::from_data_block("const A = 1;\nconst B = 2;\nhello\nconst RULESET = #{};")
        .expect_err("a stray token is a parse error");
    match err {
        LoadError::Parse { line, column, .. } => {
            // rhai reports the error at the `const` following the stray
            // `hello`: line 4, column 1.
            assert_eq!((line, column), (4, 1), "line 4 column 1");
        }
        other => panic!("{other:?}"),
    }
}

/// `byte()` is the stored position byte: the value's index in
/// `POSITIONS`, what a formation slot's `position` holds.
#[test]
fn a_positions_byte_is_its_index_in_the_table() {
    for (name, expected) in [("GK", 0), ("CF", 12)] {
        assert_eq!(
            POSITIONS
                .iter()
                .position(|label| *label == name)
                .map(|i| i as u8),
            Some(expected),
            "{name}'s index"
        );
        assert_eq!(
            Position::try_from(name.to_string()).expect(name).byte(),
            expected,
            "{name}'s byte"
        );
    }
}

/// Every entry of each closed-set table deserializes to a value whose
/// `label()` is that entry — killing the `label` and `==` mutants the
/// first-entry tests missed.
#[test]
fn every_label_round_trips() {
    for name in ABILITY_STATS {
        assert_eq!(
            Stat::try_from(name.to_string()).expect(name).label(),
            name,
            "{name}"
        );
    }
    for name in POSITIONS {
        assert_eq!(
            Position::try_from(name.to_string()).expect(name).label(),
            name,
            "{name}"
        );
    }
    for name in PLAY_STYLES {
        assert_eq!(
            PlayStyle::try_from(name.to_string()).expect(name).label(),
            name,
            "{name}"
        );
    }
    for name in SKILLS {
        assert_eq!(
            Card::try_from(name.to_string()).expect(name).label(),
            name,
            "{name}"
        );
    }
    for name in COM_STYLES {
        assert_eq!(
            Card::try_from(name.to_string()).expect(name).label(),
            name,
            "{name}"
        );
    }
}
