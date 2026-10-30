// AATF ruleset, a Rhai script. Open it in the Ruleset editor to change it.
const RULESET = #{
    schema: 1,
    name: "/vt/ League 11",
    page: "https://implyingrigged.info/wiki//vt/_League_11",

    stats: #{
        rating: ["clearing"],
        at_most: [],
    },

    tiers: [
        #{ id: "gold", label: "Gold", count: 2,
           when: #{ height_min: 195, height_max: 195, ranges: #{ swerve: [99, 99] } },
           rate: 99, form: 8, injury_resistance: 1,
           stats: #{ place_kicking: 94, defensive_prowess: 65, ball_winning: 94,
                     speed: 94, physical_contact: 94, stamina: 94 },
           weak_foot: #{ usage: 4, accuracy: 4 },
           cards: 7, min_cards: 3, free: #{ com: 3 }, a_positions: 2,
           name_color: "cc9900ff" },
        #{ id: "silver", label: "Silver", count: 3,
           when: #{ height_min: 195, height_max: 195, ranges: #{ swerve: [89, 89] } },
           rate: 89, form: 8, injury_resistance: 1,
           stats: #{ attacking_prowess: 94, heading: 94, defensive_prowess: 57,
                     tight_possession: 94, aggression: 94 },
           weak_foot: #{ usage: 4, accuracy: 4 },
           cards: 7, min_cards: 3, free: #{ com: 2 }, a_positions: 3,
           name_color: "51bbc4ff" },
        #{ id: "regular", label: "Regular", count: 10,
           when: #{ height_min: 180, height_max: 180 },
           rate: 77, form: 4, injury_resistance: 1,
           stats: #{ attacking_prowess: 87, ball_control: 87, dribbling: 87,
                    low_pass: 87, lofted_pass: 92, finishing: 92,
                    place_kicking: 70, swerve: 87, defensive_prowess: 73,
                    ball_winning: 70, kicking_power: 87, physical_contact: 70,
                    jump: 70, catching: 70, reflexes: 73, coverage: 73,
                    stamina: 62, tight_possession: 87, aggression: 92 },
           weak_foot: #{ usage: 4, accuracy: 4 },
           cards: 7, min_cards: 3, free: #{ com: 1 }, a_positions: 2 },
        #{ id: "buffed", label: "Buffed", count: 8,
           when: #{ height_min: 188, height_max: 188 },
           rate: 77, form: 4, injury_resistance: 1,
           stats: #{ attacking_prowess: 92, ball_control: 87, dribbling: 92,
                    low_pass: 87, lofted_pass: 87, finishing: 92,
                    place_kicking: 70, swerve: 87, defensive_prowess: 47,
                    ball_winning: 70, kicking_power: 92, speed: 87,
                    explosive_power: 87, body_control: 87, physical_contact: 82,
                    catching: 70, reflexes: 73, coverage: 73, stamina: 62,
                    tight_possession: 87, aggression: 87 },
           weak_foot: #{ usage: 4, accuracy: 4 },
           cards: 7, min_cards: 3, free: #{ com: 1 }, a_positions: 1 },
    ],

    universal: #{
        registered_position_a: true, b_uses_a_allowance: true, gk_not_second_a: true,
        age: [15, 50], weight: #{ floor: 30, below: 129, above: 81 },
        min_goalkeepers: 2, max_skill_cards: 11, max_com_styles: 5,
    },

    specials: [
        #{ label: "The free skills", then: #{ free_cards: [
            "no_look_pass", "double_touch", "rabona", "acrobatic_finishing",
            "first_time_shot", "dipping_shots", "rising_shots", "heading",
            "malicia", "flip_flap", "marseille_turn", "scotch_move",
            "crossover_turn"] } },
        #{ label: "The pass skills are free to non-medals",
           when: #{ tiers: ["regular", "buffed"] },
           then: #{ free_cards: ["one_touch_pass", "weighted_pass", "pinpoint_crossing"] } },
        #{ label: "The captain's Captaincy card is free",
           when: #{ captain: true }, then: #{ free_cards: ["captaincy"] } },
        #{ label: "Long Ranger is a free COM style",
           then: #{ free_cards: ["long_ranger"] } },
        #{ label: "Malicia is required", then: #{ require_cards: ["malicia"] } },
        #{ label: "The GK Penalty Specialist card is banned",
           when: #{ holds: ["gk_penalty_specialist"] },
           then: #{ forbid: "The GK Penalty Specialist card is banned" } },
        #{ label: "Regulars cannot be captain",
           when: #{ tiers: ["regular", "buffed"], captain: true },
           then: #{ forbid: "Regulars can't be captain" } },
        #{ label: "Medals cannot be goalkeepers",
           when: #{ tiers: ["gold", "silver"], positions: ["GK"] },
           then: #{ forbid: "Medals cannot play as GK" } },
        #{ label: "B positions only for 188cm players at CB",
           when: #{ any: [
               #{ tiers: ["gold", "silver", "regular"],
                  rated_b_at: ["GK", "CB", "LB", "RB", "DMF", "CMF", "LMF", "RMF",
                               "AMF", "LWF", "RWF", "SS", "CF"] },
               #{ tiers: ["buffed"],
                  rated_b_at: ["GK", "LB", "RB", "DMF", "CMF", "LMF", "RMF",
                               "AMF", "LWF", "RWF", "SS", "CF"] },
           ] },
           then: #{ forbid: "Illegal B positions, only allowed on 188cm players at CB" } },
        #{ label: "188cm players have no A at CB or GK",
           when: #{ tiers: ["buffed"], rated_a_at: ["CB", "GK"] },
           then: #{ forbid: "188cm players may not have an A-position at CB or GK" } },
        #{ label: "A goalkeeper's form is 8",
           when: #{ tiers: ["regular"], positions: ["GK"] },
           then: #{ form: 4 } },
        #{ label: "A gold Fox in the Box",
           when: #{ tiers: ["gold"], playing_styles: ["fox_in_the_box"] },
           then: #{ stat_bonus: #{ ball_winning: 5, defensive_prowess: -5 } } },
        #{ label: "A gold Target Man",
           when: #{ tiers: ["gold"], playing_styles: ["target_man"] },
           then: #{ stat_bonus: #{ physical_contact: 5, defensive_prowess: -5 } } },
        #{ label: "A gold Creative Playmaker",
           when: #{ tiers: ["gold"], playing_styles: ["creative_playmaker"] },
           then: #{ stat_bonus: #{ stamina: 5, defensive_prowess: -5 } } },
        #{ label: "Goal Poacher at CF", severity: "warning",
           when: #{ playing_styles: ["goal_poacher"], rated_a_at: ["CF"] },
           then: #{ forbid: "Goal Poacher does not activate at CF with the league's exe" } },
        #{ label: "Creative Playmaker at CF", severity: "warning",
           when: #{ playing_styles: ["creative_playmaker"], rated_a_at: ["CF"] },
           then: #{ forbid: "Creative Playmaker activates at CF with the league's exe" } },
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
// 188cm players' registered positions — at most two, and no 180cm player may share one.
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
    // The 180cm players at one of the first two of those positions.
    let theirs = [];
    for position in positions {
        if theirs.len() == 2 { break; }
        theirs.push(position);
    }
    for player in team.players {
        if player.height == 180 && player.position in theirs {
            found.push(#{ kind: "Custom",
                          text: "A 180cm player shares a 188cm player's registered position",
                          slot: player.slot });
        }
    }
    found
}
