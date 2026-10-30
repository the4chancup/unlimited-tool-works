//! The label tables `team.toml` writes where the record stores a number or a
//! bit: `(key, ["false" label, "true" label])` pairs for the style switches,
//! index tables for positions, instructions, ratings, styles, skills and COM
//! styles. Pure data.

use pes_version::PesVersion;

/// `[tactics.preset_N.style]`: `(key, [false label, true label])` in emit order.
pub const STYLE_SWITCHES: [(&str, [&str; 2]); 7] = [
    ("attacking_style", ["counter_attack", "possession"]),
    ("attacking_zone", ["centre", "wide"]),
    ("buildup", ["long_pass", "short_pass"]),
    ("positioning", ["maintain", "flexible"]),
    ("defensive_style", ["frontline_pressure", "all_out_defence"]),
    ("containment_area", ["middle", "wide"]),
    ("pressure", ["aggressive", "conservative"]),
];

/// Formation/registered-position names; index = the stored position byte.
pub const POSITIONS: [&str; 13] = [
    "GK", "CB", "LB", "RB", "DMF", "CMF", "LMF", "RMF", "AMF", "LWF", "RWF", "SS", "CF",
];

/// Advanced-instruction names in canonical order (`model::instruction::ALL`'s
/// index): `wing_back`, not a stored value.
pub const INSTRUCTIONS: [&str; 17] = [
    "off",
    "hug_the_touchline",
    "false_no_9",
    "false_full_backs",
    "attacking_full_backs",
    "wing_rotation",
    "tiki_taka",
    "centering_targets",
    "swarm_the_box",
    "deep_defensive_line",
    "gegenpress",
    "tight_marking",
    "counter_target",
    "defensive",
    "false_winger",
    "wing_back",
    "anchoring",
];

/// Playable-position ratings; index = the stored 2-bit value (0 C, 1 B,
/// 2 A). 3 is past the game's scale, held by no save, spelled as its
/// number so a dump still carries it.
pub const RATINGS: [&str; 4] = ["C", "B", "A", "3"];

/// The index into `POSITIONS` of `playable` entry `i`: the save's order
/// CF, SS, LWF, RWF, AMF, DMF, CMF, LMF, RMF, CB, LB, RB, GK — the position
/// groups in reverse order, each group in the registered order. Measured:
/// 11 795 of 11 799 rated fixture players store A at their registered
/// position under this order (a plain reversal reads only 5 683).
pub const PLAYABLE_POSITIONS: [usize; 13] = [12, 11, 9, 10, 8, 4, 5, 6, 7, 1, 2, 3, 0];

/// `PlayStyle` names in canonical order (`PlayStyle::ALL`'s index).
pub const PLAY_STYLES: [&str; 22] = [
    "none",
    "goal_poacher",
    "dummy_runner",
    "fox_in_the_box",
    "target_man",
    "creative_playmaker",
    "prolific_winger",
    "roaming_flank",
    "crossing_specialist",
    "classic_no_10",
    "hole_player",
    "box_to_box",
    "the_destroyer",
    "orchestrator",
    "anchor_man",
    "offensive_fullback",
    "fullback_finisher",
    "defensive_fullback",
    "build_up",
    "extra_frontman",
    "offensive_goalkeeper",
    "defensive_goalkeeper",
];

/// Player-skill names; index = the canonical skill slot (0 Scissors Feint to
/// 40 Through Passing).
pub const SKILLS: [&str; 41] = [
    "scissors_feint",
    "flip_flap",
    "marseille_turn",
    "sombrero",
    "cut_behind_and_turn",
    "scotch_move",
    "heading",
    "long_range_drive",
    "knuckle_shot",
    "acrobatic_finishing",
    "heel_trick",
    "first_time_shot",
    "one_touch_pass",
    "weighted_pass",
    "pinpoint_crossing",
    "outside_curler",
    "rabona",
    "low_lofted_pass",
    "low_punt_trajectory",
    "long_throw",
    "gk_long_throw",
    "malicia",
    "man_marking",
    "track_back",
    "acrobatic_clear",
    "captaincy",
    "super_sub",
    "fighting_spirit",
    "double_touch",
    "crossover_turn",
    "step_on_skill",
    "chip_shot",
    "dipping_shots",
    "rising_shots",
    "no_look_pass",
    "gk_high_punt",
    "penalty_specialist",
    "gk_penalty_specialist",
    "interception",
    "long_range_shooting",
    "through_passing",
];

/// COM playing-style names; index = the canonical slot (0 Trickster to 6 Long
/// Ranger).
pub const COM_STYLES: [&str; 7] = [
    "trickster",
    "mazing_run",
    "speeding_bullet",
    "incisive_run",
    "long_ball_expert",
    "early_cross",
    "long_ranger",
];

/// Stronger foot/hand labels; index = the stored bool.
pub const FOOT: [&str; 2] = ["right", "left"];

/// Ability-stat names: the `PlayerStats` fields that are 0-99 abilities, in
/// `PlayerStats`'s field order, the version-dependent ones (tight possession,
/// aggression) included. Weak foot, form, injury resistance, star and playing
/// attitude are stats but not abilities. `team.toml` spells them as
/// `stats.<name>`; other crates share the table for their own closed sets.
pub const ABILITY_STATS: [&str; 25] = [
    "attacking_prowess",
    "ball_control",
    "dribbling",
    "low_pass",
    "lofted_pass",
    "finishing",
    "place_kicking",
    "swerve",
    "heading",
    "defensive_prowess",
    "ball_winning",
    "kicking_power",
    "speed",
    "explosive_power",
    "body_control",
    "physical_contact",
    "jump",
    "goalkeeping",
    "catching",
    "clearing",
    "reflexes",
    "coverage",
    "stamina",
    "tight_possession",
    "aggression",
];

/// `name`'s label as the draft's `titleCase` writes it: underscores as
/// spaces, each word capitalized (the stat labels the save editor draws
/// from these tables; a card's name is `card_name`'s).
pub fn title_case(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let (first, rest) = word.split_at(1);
            format!("{}{rest}", first.to_uppercase())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A skill's, COM style's or playing style's name as the views draw it: its
/// `SKILLS`, `COM_STYLES` or `PLAY_STYLES` spelling read as an English title.
/// `gk` is GK, `and` is &, `in`, `on`, `the` and `to` stay lower case after
/// the first word, and `no` before `10` is No. (`GK Long Throw`, `Cut Behind
/// & Turn`, `Fox in the Box`, `Classic No. 10`), where the bare title case
/// would draw `Gk Long Throw` and `Classic No 10`.
pub fn card_name(id: &str) -> String {
    let words: Vec<&str> = id.split('_').collect();
    words
        .iter()
        .enumerate()
        .map(|(i, &word)| match word {
            "gk" => "GK".to_owned(),
            "and" => "&".to_owned(),
            "no" if words.get(i + 1) == Some(&"10") => "No.".to_owned(),
            "in" | "on" | "the" | "to" if i > 0 => word.to_owned(),
            _ => title_case(word),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The name the game of `version` shows for the stat spelled `stat` (an
/// `ABILITY_STATS` entry, or `form`, `injury_resistance`, `weak_foot_usage`,
/// `weak_foot_accuracy`): the place a stat's shown name comes from wherever
/// it depends on the version, so the player view, the changes lists and the
/// AATF rows' sentences read the names the member's game shows. PES 2017's
/// Edit screen calls `heading` Header and every other stat by its spelling
/// title-cased. The other versions' names are not read off their games yet,
/// so they fall back to the title case. The AATF rows' rule column and the
/// Other stats' labels name Form, Injury Resistance and the weak foot's
/// halves with fixed strings instead, equal to the names here on every
/// version so far.
pub fn stat_name(version: PesVersion, stat: &str) -> String {
    // Each version's stats whose name is not their spelling title-cased.
    let renamed: &[(&str, &str)] = match version {
        PesVersion::Pes17 => &[("heading", "Header")],
        PesVersion::Pes15
        | PesVersion::Pes16
        | PesVersion::Pes18
        | PesVersion::Pes19
        | PesVersion::Pes20
        | PesVersion::Pes21 => &[],
    };
    renamed
        .iter()
        .find(|(spelling, _)| *spelling == stat)
        .map_or_else(|| title_case(stat), |(_, name)| (*name).to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interchange::team_toml::player_keys::PlayerKey;
    use crate::model::instruction::Instruction;
    use crate::model::playstyle::PlayStyle;

    /// `ABILITY_STATS`, sorted, is exactly the `PlayerKey` names of the
    /// `Kind::Ability` keys, `stats.` prefix stripped: membership,
    /// completeness and no duplicates in one assertion.
    #[test]
    fn ability_stats_are_the_ability_keys() {
        use crate::interchange::team_toml::player_keys::Kind;

        let mut from_keys: Vec<&str> = PlayerKey::ALL
            .iter()
            .filter(|key| matches!(key.spec().kind, Kind::Ability))
            .map(|key| key.name())
            .collect();
        from_keys.sort_unstable();
        let mut table = ABILITY_STATS.to_vec();
        table.sort_unstable();
        assert_eq!(table, from_keys);
    }

    /// `PlayerStats::abilities` entry `i` is the `stats.<ABILITY_STATS[i]>`
    /// key's value on every fixture version's first named player.
    #[test]
    fn abilities_agree_with_the_stats_keys() {
        use crate::test_support;
        use pes_version::PesVersion;

        for version in PesVersion::ALL {
            let (file, _) = test_support::open(version);
            let player = file
                .players()
                .iter()
                .find(|p| !p.name.is_empty())
                .expect("a named player");
            let abilities = player.stats.abilities();
            for (i, name) in ABILITY_STATS.iter().enumerate() {
                let key = PlayerKey::ALL
                    .iter()
                    .find(|key| key.table() == "stats" && key.name() == *name)
                    .expect("every ability stat is a stats key");
                assert_eq!(
                    abilities[i].map(u32::from),
                    key.get(player),
                    "{version:?} stats.{name}"
                );
            }
        }
    }

    /// `PLAYABLE_POSITIONS` is a permutation of 0..13; its inverse maps a
    /// `POSITIONS` index to its `playable` entry.
    #[test]
    fn playable_positions_is_a_permutation() {
        let mut sorted = PLAYABLE_POSITIONS;
        sorted.sort_unstable();
        assert_eq!(sorted, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        let mut inverse = [0usize; 13];
        for (i, &position) in PLAYABLE_POSITIONS.iter().enumerate() {
            inverse[position] = i;
        }
        assert_eq!(inverse, [12, 9, 10, 11, 5, 6, 7, 8, 4, 2, 3, 1, 0]);
    }

    /// Lowercase ASCII, digits and underscores only.
    fn is_snake_case(label: &str) -> bool {
        !label.is_empty()
            && label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    }

    fn unique(table: &[&str]) -> bool {
        let mut sorted = table.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        sorted.len() == table.len()
    }

    #[test]
    fn every_label_table_is_sized_and_unique() {
        assert_eq!(INSTRUCTIONS.len(), Instruction::ALL.len());
        assert_eq!(PLAY_STYLES.len(), PlayStyle::ALL.len());
        assert_eq!(POSITIONS.len(), 13);
        assert_eq!(RATINGS.len(), 4);
        assert_eq!(SKILLS.len(), 41);
        assert_eq!(COM_STYLES.len(), 7);
        assert_eq!(FOOT.len(), 2);
        for table in [
            POSITIONS.as_slice(),
            INSTRUCTIONS.as_slice(),
            RATINGS.as_slice(),
            PLAY_STYLES.as_slice(),
            SKILLS.as_slice(),
            COM_STYLES.as_slice(),
            FOOT.as_slice(),
        ] {
            assert!(unique(table), "duplicates in {table:?}");
        }
        for (_, pair) in STYLE_SWITCHES {
            assert_ne!(pair[0], pair[1]);
        }
    }

    /// `title_case` writes a table name the way the labels read it:
    /// underscores as spaces, each word capitalized — on a COM style, a
    /// play style and a one-word skill.
    #[test]
    fn title_case_writes_the_labels_as_drawn() {
        assert_eq!(title_case("speeding_bullet"), "Speeding Bullet");
        assert_eq!(title_case("goal_poacher"), "Goal Poacher");
        assert_eq!(title_case("sombrero"), "Sombrero");
    }

    /// Every skill, COM style and playing style, in table order, under the
    /// name the views draw for it.
    #[test]
    fn card_name_reads_every_card_as_an_english_title() {
        let skills = [
            "Scissors Feint",
            "Flip Flap",
            "Marseille Turn",
            "Sombrero",
            "Cut Behind & Turn",
            "Scotch Move",
            "Heading",
            "Long Range Drive",
            "Knuckle Shot",
            "Acrobatic Finishing",
            "Heel Trick",
            "First Time Shot",
            "One Touch Pass",
            "Weighted Pass",
            "Pinpoint Crossing",
            "Outside Curler",
            "Rabona",
            "Low Lofted Pass",
            "Low Punt Trajectory",
            "Long Throw",
            "GK Long Throw",
            "Malicia",
            "Man Marking",
            "Track Back",
            "Acrobatic Clear",
            "Captaincy",
            "Super Sub",
            "Fighting Spirit",
            "Double Touch",
            "Crossover Turn",
            "Step on Skill",
            "Chip Shot",
            "Dipping Shots",
            "Rising Shots",
            "No Look Pass",
            "GK High Punt",
            "Penalty Specialist",
            "GK Penalty Specialist",
            "Interception",
            "Long Range Shooting",
            "Through Passing",
        ];
        let com_styles = [
            "Trickster",
            "Mazing Run",
            "Speeding Bullet",
            "Incisive Run",
            "Long Ball Expert",
            "Early Cross",
            "Long Ranger",
        ];
        let play_styles = [
            "None",
            "Goal Poacher",
            "Dummy Runner",
            "Fox in the Box",
            "Target Man",
            "Creative Playmaker",
            "Prolific Winger",
            "Roaming Flank",
            "Crossing Specialist",
            "Classic No. 10",
            "Hole Player",
            "Box to Box",
            "The Destroyer",
            "Orchestrator",
            "Anchor Man",
            "Offensive Fullback",
            "Fullback Finisher",
            "Defensive Fullback",
            "Build Up",
            "Extra Frontman",
            "Offensive Goalkeeper",
            "Defensive Goalkeeper",
        ];
        for (ids, names) in [
            (SKILLS.as_slice(), skills.as_slice()),
            (COM_STYLES.as_slice(), com_styles.as_slice()),
            (PLAY_STYLES.as_slice(), play_styles.as_slice()),
        ] {
            assert_eq!(ids.len(), names.len());
            for (id, name) in ids.iter().zip(names) {
                assert_eq!(card_name(id), *name, "{id}");
            }
        }
    }

    /// PES 2017's Edit screen lists its 23 abilities, then the weak foot's
    /// two, form and injury resistance, in this order and under these
    /// names. Another version's `heading` keeps the title case until its
    /// names are read off its game.
    #[test]
    fn pes_2017_names_its_stats_as_its_edit_screen_does() {
        let edit_screen = [
            ("attacking_prowess", "Attacking Prowess"),
            ("ball_control", "Ball Control"),
            ("dribbling", "Dribbling"),
            ("low_pass", "Low Pass"),
            ("lofted_pass", "Lofted Pass"),
            ("finishing", "Finishing"),
            ("place_kicking", "Place Kicking"),
            ("swerve", "Swerve"),
            ("heading", "Header"),
            ("defensive_prowess", "Defensive Prowess"),
            ("ball_winning", "Ball Winning"),
            ("kicking_power", "Kicking Power"),
            ("speed", "Speed"),
            ("explosive_power", "Explosive Power"),
            ("body_control", "Body Control"),
            ("physical_contact", "Physical Contact"),
            ("jump", "Jump"),
            ("goalkeeping", "Goalkeeping"),
            ("catching", "Catching"),
            ("clearing", "Clearing"),
            ("reflexes", "Reflexes"),
            ("coverage", "Coverage"),
            ("stamina", "Stamina"),
            ("weak_foot_usage", "Weak Foot Usage"),
            ("weak_foot_accuracy", "Weak Foot Accuracy"),
            ("form", "Form"),
            ("injury_resistance", "Injury Resistance"),
        ];
        for (stat, name) in edit_screen {
            assert_eq!(stat_name(PesVersion::Pes17, stat), name, "{stat}");
        }
        assert_eq!(stat_name(PesVersion::Pes21, "heading"), "Heading");
    }

    #[test]
    fn labels_are_snake_case_or_the_uppercase_short_forms() {
        // Positions ("GK") and the ratings' letter grades are uppercase in the
        // TOML block of `spec/pes_savefile/operations.md` "Team TOML";
        // everything else is snake_case.
        for label in INSTRUCTIONS
            .iter()
            .chain(PLAY_STYLES.iter())
            .chain(SKILLS.iter())
            .chain(COM_STYLES.iter())
            .chain(FOOT.iter())
            .chain(RATINGS.iter().filter(|l| **l == "3"))
            .chain(STYLE_SWITCHES.iter().flat_map(|(key, pair)| {
                assert!(is_snake_case(key), "{key}");
                pair.iter()
            }))
        {
            assert!(is_snake_case(label), "{label}");
        }
    }
}
