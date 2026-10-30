use super::*;

mod conditions;
mod rules;
mod schema;

/// The official data block, embedded.
const AUTUMN26: &str = include_str!("../../../rulesets/autumn26.rhai");

/// `text` with `from` replaced once by `to` (asserted present first).
fn edit(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "the snippet is in the file: {from}");
    text.replacen(from, to, 1)
}

#[test]
fn the_official_ruleset_loads() {
    let ruleset = Ruleset::from_data_block(AUTUMN26).expect("the official file loads");
    let tiers: Vec<&str> = ruleset.tiers.iter().map(|tier| tier.id.as_str()).collect();
    assert_eq!(tiers, ["gold", "silver", "bronze", "goalkeeper", "regular"]);
    assert_eq!(ruleset.tiers[4].count, None, "regular is the rest tier");
    assert_eq!(ruleset.tiers[3].counts_as.as_deref(), Some("regular"));
    // Green's quotas, in bracket order as written.
    let green = ruleset
        .heights
        .systems
        .iter()
        .find(|system| system.id == "green")
        .expect("the green system");
    let quotas: Vec<(&str, u8)> = green
        .quotas
        .iter()
        .map(|(id, count)| (id.as_str(), *count))
        .collect();
    // `BTreeMap` order is alphabetical, not the file's bracket order.
    assert_eq!(
        quotas,
        [
            ("giant", 6),
            ("giga", 0),
            ("manlet", 6),
            ("mid", 5),
            ("tall", 6)
        ]
    );
    assert_eq!(ruleset.specials.len(), 13);
    let last = ruleset.specials.last().expect("a last special");
    assert_eq!(last.then.choice.len(), 2);
    // "Red manlet bonus": a one-option `choice` whose `stat_bonus` is
    // `all` keyed by four tiers.
    let bonus = ruleset
        .specials
        .iter()
        .find(|special| special.label == "Red manlet bonus")
        .expect("the manlet bonus");
    assert_eq!(bonus.then.choice.len(), 1);
    let option = bonus.then.choice.first().expect("the bonus's option");
    let Some(PerTier::ByTier(by_tier)) = option.stat_bonus.get(&StatOrAll::All) else {
        panic!("the bonus is not All-by-tier: {:?}", option.stat_bonus);
    };
    assert_eq!(by_tier.len(), 4);
    for (id, amount) in [
        ("regular", 5),
        ("goalkeeper", 5),
        ("bronze", 3),
        ("silver", 2),
    ] {
        assert_eq!(by_tier.get(id), Some(&amount), "{id}");
    }
}

/// The operation limit binds the top level, not only `check_team`
/// (`rules.rs`'s loop test): a `loop {}` ahead of `RULESET` fails the
/// load with rhai's operations error, never a hang.
#[test]
fn a_loop_before_the_data_block_stops_at_the_operation_limit() {
    let err = Ruleset::from_data_block(
        "loop {}\nconst RULESET = #{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77 }] };",
    )
    .expect_err("the limit stops it");
    match err {
        LoadError::Parse { message, .. } => {
            assert!(message.contains("operations"), "{message}");
        }
        other => panic!("expected Parse, got {other:?}"),
    }
}

// One failing file per load-time rule: `edit` of a unique snippet of
// the official text → `Rules` with exactly the expected error.
fn case(name: &str, broken: &str, rule: LoadRule, path: &str) {
    let err = Ruleset::from_data_block(broken).expect_err(name);
    match err {
        LoadError::Rules(errors) => {
            assert_eq!(
                errors,
                vec![RuleError {
                    path: path.to_string(),
                    rule,
                }],
                "{name}"
            );
        }
        other => panic!("{name}: expected Rules, got {other:?}"),
    }
}
/// `Ruleset` deserialization errors land as `Schema` with rhai's text.
fn schema(text: &str) -> LoadError {
    Ruleset::from_data_block(&format!("const RULESET = {text};")).expect_err("a schema error")
}
