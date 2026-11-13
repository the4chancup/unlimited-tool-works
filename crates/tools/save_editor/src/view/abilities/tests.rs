use pes_savefile::schema::fields::ABILITY_FIELDS;
use pes_savefile::schema::fields::PlayerField;
use pes_savefile::schema::fields::PlayerText;
use pes_savefile::schema::{RecordSchema, schema_for};
use pes_version::PesVersion;

use super::{SMALL_STATS, aside_cards, stat_name};

/// The `2^bit_width - 1` bound `info_strip.rs`'s `max` reads, over the
/// appearance record too (a field may live there on PES 15/16); `None`
/// on a version that does not store `field`.
fn field_max(version: PesVersion, field: PlayerField) -> Option<u64> {
    fn in_record(
        record: &RecordSchema<PlayerField, PlayerText>,
        field: PlayerField,
    ) -> Option<u64> {
        record
            .fields
            .iter()
            .find(|spec| spec.field == field)
            .map(|spec| u64::from(spec.bit_width))
            .or_else(|| {
                record.arrays.iter().find_map(|array| {
                    (0..array.count)
                        .any(|i| (array.make)(i) == field)
                        .then_some(u64::from(array.bit_width))
                })
            })
    }
    let schema = schema_for(version);
    let width = in_record(schema.player, field).or_else(|| {
        schema
            .appearance
            .as_ref()
            .and_then(|(_, record)| in_record(record, field))
    })?;
    Some((1u64 << width) - 1)
}

/// `name`'s index in `labels::SKILLS` (a card's index is its
/// position there).
fn skill(name: &str) -> u8 {
    u8::try_from(
        pes_savefile::interchange::team_toml::labels::SKILLS
            .iter()
            .position(|&label| label == name)
            .expect("a skill"),
    )
    .expect("a byte index")
}

/// The smallest ruleset the crate's loader accepts, holding `body`
/// (the `card_groups`/`specials` lines the case varies): the check
/// asks the tiers' counts for the squad, so a rest tier is always
/// there.
fn ruleset(body: &str) -> aatf::Ruleset {
    let block = format!(
        "const RULESET = #{{
                schema: 1, name: \"x\",
                universal: #{{ max_skill_cards: 10 }},
                tiers: [#{{ id: \"regular\", label: \"Regular\", rate: 77, form: 4,
                            injury_resistance: 1, weak_foot: #{{ usage: 2, accuracy: 2 }},
                            cards: 3, a_positions: 1 }}],
                {body}
            }};"
    );
    aatf::Ruleset::from_data_block(&block).expect("the block loads")
}

// The aside panel's set: a `tricks` group names it, else the skills
// the `free_cards` effects name — a free card inside a `choice`
// option counts — in `SKILLS` order with none twice.
#[test]
fn the_aside_holds_the_tricks_else_the_free_skills() {
    // A `tricks` group: its skills in SKILLS order however the file
    // lists them (its `free_cards` specials are not read then).
    let rs = ruleset(
        "card_groups: #{ tricks: [\"malicia\", \"scissors_feint\"] },
             specials: [#{ label: \"s\", then: #{ free_cards: [\"heading\"] } }],",
    );
    let (title, indices) = aside_cards(&rs).expect("the group names cards");
    assert_eq!(title, "Tricks");
    assert_eq!(indices, [skill("scissors_feint"), skill("malicia")]);

    // No `tricks` group: every skill a `free_cards` effect names,
    // a conditional special's and a choice option's included, the
    // COM styles left out.
    let rs = ruleset(
        "specials: [
                #{ label: \"free\", then: #{ free_cards: [\"malicia\", \"long_ranger\"] } },
                #{ label: \"conditional\", when: #{ captain: true },
                   then: #{ free_cards: [\"captaincy\"] } },
                #{ label: \"pick\",
                   then: #{ choice: [#{ free_cards: [\"heading\"] }, #{ cards: 1 }] } },
            ],",
    );
    let (title, indices) = aside_cards(&rs).expect("the free cards name skills");
    assert_eq!(title, "Free cards");
    assert_eq!(
        indices,
        [skill("heading"), skill("malicia"), skill("captaincy")],
        "the choice's, the conditional's and the plain effect's skills, in SKILLS order"
    );

    // A ruleset whose only free cards sit in a choice's options:
    // they count too.
    let rs = ruleset(
        "specials: [#{ label: \"pick\",
                then: #{ choice: [#{ free_cards: [\"heading\"] }, #{ cards: 1 }] } }],",
    );
    assert_eq!(
        aside_cards(&rs),
        Some(("Free cards", vec![skill("heading")]))
    );

    // A skill named twice lists once: twice in the `tricks` group,
    // or once under each of two specials.
    let rs = ruleset("card_groups: #{ tricks: [\"malicia\", \"malicia\"] },");
    assert_eq!(aside_cards(&rs), Some(("Tricks", vec![skill("malicia")])));
    let rs = ruleset(
        "specials: [
                #{ label: \"a\", then: #{ free_cards: [\"malicia\"] } },
                #{ label: \"b\", when: #{ captain: true },
                   then: #{ free_cards: [\"malicia\"] } },
            ],",
    );
    assert_eq!(
        aside_cards(&rs),
        Some(("Free cards", vec![skill("malicia")]))
    );

    // Neither: no aside panel.
    assert_eq!(aside_cards(&ruleset("")), None);
}

// Every range's top fits the field's bit width on every version
// storing it (after the stored-one-lower shift). The Appearance
// section's Cutscenes pair is a `SmallStat` too.
#[test]
fn every_ranges_top_fits_the_fields_bit_width() {
    for (field, top, shift) in SMALL_STATS
        .iter()
        .chain(crate::view::appearance::fields::CUTSCENE_STATS.iter())
        .map(|small| (small.field, small.max, small.shift))
        .chain(ABILITY_FIELDS.iter().map(|&field| (field, 99u8, 0u8)))
    {
        for version in PesVersion::ALL {
            let Some(max) = field_max(version, field) else {
                continue;
            };
            assert!(
                u64::from(top - shift) <= max,
                "{version:?} {field:?}: shown top {top} stores {} past {max}",
                top - shift
            );
        }
    }
}

// The Other stats' labels are fixed strings, not looked up per version:
// they are PES 2017's names, the one version whose names are read off
// its game. A version found to name them otherwise needs them looked up
// through `stat_name` like the ability column.
#[test]
fn the_small_stats_labels_are_the_games_names() {
    for (field, stat) in [
        (PlayerField::Form, "form"),
        (PlayerField::InjuryResistance, "injury_resistance"),
        (PlayerField::WeakFootUsage, "weak_foot_usage"),
        (PlayerField::WeakFootAccuracy, "weak_foot_accuracy"),
    ] {
        let small = SMALL_STATS
            .iter()
            .find(|small| small.field == field)
            .expect("an Other stats field");
        assert_eq!(small.label, stat_name(PesVersion::Pes17, stat), "{field:?}");
    }
}

/// An empty roster slot fails the badge's check, naming the slot as
/// the player list numbers it: from 01, not the 0-based slot.
#[test]
fn an_empty_slot_is_named_as_the_list_numbers_it() {
    let file = pes_savefile::EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21))
        .expect("fixture save");
    let team = aatf::TeamView::new(&file, 701).expect("team 701");
    let slot = (0..=u8::MAX)
        .find(|&slot| team.player(slot).is_none())
        .expect("an empty slot");
    assert_eq!(slot, 23, "premise: the first slot past a full roster");
    let badge = super::own_badge(&aatf::Rules::official(), &team, slot);
    let super::OwnBadge::Failed(error) = badge else {
        panic!("an empty slot fails the check");
    };
    assert_eq!(error, "slot 24 of the roster holds no player");
}
