// AATF ruleset, a Rhai script. Open it in the Ruleset editor to change it.
const RULESET = #{
    schema: 1,
    name: "/vt/ League 10",
    page: "https://implyingrigged.info/wiki//vt/_League_10",
    squad_size: 23,

    stats: #{
        rating: ["clearing"],
        at_most: [],
    },

    tiers: [
        #{ id: "gold", label: "Gold", count: 2,
           when: #{ height_min: 194, height_max: 194, ranges: #{ swerve: [99, 99] } },
           rate: 99, form: 8, injury_resistance: 1,
           stats: #{ place_kicking: 96, defensive_prowess: 58 },
           weak_foot: #{ usage: 4, accuracy: 4 },
           cards: 6, min_cards: 3, free: #{ com: 2 }, a_positions: 2 },
        #{ id: "silver", label: "Silver", count: 3,
           when: #{ height_min: 194, height_max: 194, ranges: #{ swerve: [92, 92] } },
           rate: 92, form: 8, injury_resistance: 1,
           stats: #{ attacking_prowess: 95, place_kicking: 88, defensive_prowess: 56,
                    ball_winning: 88, stamina: 95, aggression: 88 },
           weak_foot: #{ usage: 4, accuracy: 4 },
           cards: 7, min_cards: 3, free: #{ com: 2 }, a_positions: 3 },
        #{ id: "regular", label: "Regular", count: 10,
           when: #{ any: [#{ height_min: 178, height_max: 178 },
                          #{ height_min: 180, height_max: 180 }] },
           rate: 77, form: 4, injury_resistance: 1,
           stats: #{ attacking_prowess: 87, ball_control: 87, dribbling: 87,
                    low_pass: 87, lofted_pass: 92, finishing: 97,
                    place_kicking: 70, swerve: 87, defensive_prowess: 73,
                    ball_winning: 68, kicking_power: 87, explosive_power: 73,
                    physical_contact: 68, jump: 67, goalkeeping: 75,
                    catching: 70, reflexes: 73, coverage: 73, stamina: 55,
                    tight_possession: 87, aggression: 92 },
           weak_foot: #{ usage: 4, accuracy: 4 },
           cards: 7, min_cards: 3, free: #{ com: 1 }, a_positions: 2 },
        #{ id: "buffed", label: "Buffed", count: 8,
           when: #{ height_min: 188, height_max: 188 },
           rate: 77, form: 4, injury_resistance: 1,
           stats: #{ attacking_prowess: 92, ball_control: 87, dribbling: 92,
                    low_pass: 87, lofted_pass: 87, finishing: 92,
                    place_kicking: 70, swerve: 87, defensive_prowess: 53,
                    ball_winning: 68, kicking_power: 92, speed: 88,
                    explosive_power: 88, body_control: 88, physical_contact: 88,
                    goalkeeping: 75, catching: 70, reflexes: 73, coverage: 73,
                    stamina: 60, tight_possession: 87, aggression: 87 },
           weak_foot: #{ usage: 4, accuracy: 4 },
           cards: 7, min_cards: 3, free: #{ com: 1 }, a_positions: 1 },
    ],

    universal: #{
        registered_position_a: true, no_b_ratings: true, gk_not_second_a: true,
        age: [15, 50], weight: #{ floor: 30, below: 129, above: 81 },
        min_goalkeepers: 2, max_skill_cards: 12, max_com_styles: 5,
    },

    specials: [
        #{ label: "The free skills", then: #{ free_cards: [
            "scissors_feint", "flip_flap", "marseille_turn", "sombrero",
            "cut_behind_and_turn", "scotch_move", "acrobatic_finishing",
            "heel_trick", "first_time_shot", "malicia", "double_touch",
            "crossover_turn", "step_on_skill"] } },
        #{ label: "The pass skills are free to non-medals",
           when: #{ tiers: ["regular", "buffed"] },
           then: #{ free_cards: ["one_touch_pass", "weighted_pass", "pinpoint_crossing"] } },
        #{ label: "The captain's Captaincy card is free",
           when: #{ captain: true }, then: #{ free_cards: ["captaincy"] } },
        #{ label: "Incisive Run and Long Ranger are free COM styles",
           then: #{ free_cards: ["incisive_run", "long_ranger"] } },
        #{ label: "Malicia is required", then: #{ require_cards: ["malicia"] } },
        #{ label: "Regulars cannot be captain",
           when: #{ tiers: ["regular", "buffed"], captain: true },
           then: #{ forbid: "Regulars can't be captain" } },
        #{ label: "Medals cannot be goalkeepers",
           when: #{ tiers: ["gold", "silver"], positions: ["GK"] },
           then: #{ forbid: "Medals cannot play as GK" } },
        #{ label: "188cm players have no A at CB or GK",
           when: #{ tiers: ["buffed"], rated_a_at: ["CB", "GK"] },
           then: #{ forbid: "188cm players may not have an A-position at CB or GK" } },
        #{ label: "A goalkeeper's form is 8",
           when: #{ tiers: ["regular"], positions: ["GK"] },
           then: #{ form: 4 } },
        #{ label: "Regular goalkeepers are 180cm",
           when: #{ tiers: ["regular"], positions: ["GK"],
                    height_min: 178, height_max: 178 },
           then: #{ forbid: "Regular goalkeepers must be 180cm" } },
        #{ label: "Outfield regulars are 178cm",
           when: #{ tiers: ["regular"], height_min: 180, height_max: 180,
                    not: #{ positions: ["GK"] } },
           then: #{ forbid: "Outfield regulars must be 178cm" } },
        #{ label: "Buffed wingbacks are 2/2 weak foot",
           when: #{ tiers: ["buffed"], rated_a_at: ["LB", "RB"] },
           then: #{ weak_foot: #{ usage: 2, accuracy: 2 } } },
    ],

    team_settings: [
        #{ label: "No man marking on PES 16", versions: [16], man_marking: false },
        #{ label: "PES 16 support settings", versions: [16],
           auto_substitution: false, auto_offside_trap: false, auto_preset_change: false },
    ],

    logo: "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",
};

// ==== CUSTOM CHECKS: optional, hand-written; the Ruleset editor keeps this section as it is ====
// What the schema cannot carry: the goalkeeper maximum (min_goalkeepers is a floor), and the
// 188cm players' registered positions — at most two, and no 178cm player may share one.
fn custom_checks(team) {
    let found = [];
    let goalkeepers = 0;
    for player in team.players {
        if player.position == "GK" { goalkeepers += 1; }
    }
    if goalkeepers > 2 {
        found.push(#{ kind: "Custom", text: "More than two goalkeepers" });
    }
    // The 188cm players' registered positions, in first-seen order.
    let positions = [];
    for player in team.players {
        if player.height == 188 && player.position != () && !(player.position in positions) {
            positions.push(player.position);
        }
    }
    if positions.len() > 2 {
        found.push(#{ kind: "Custom", text: "Too many registered positions with 188cm players. Maximum is 2" });
    }
    // The 178cm players at one of the first two of those positions.
    let theirs = [];
    for position in positions {
        if theirs.len() == 2 { break; }
        theirs.push(position);
    }
    for player in team.players {
        if player.height == 178 && player.position in theirs {
            found.push(#{ kind: "Custom",
                          text: "A 178cm player shares a 188cm player's registered position",
                          slot: player.slot });
        }
    }
    found
}
