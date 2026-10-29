// The Ruleset editor's draft (docs/plans/ruleset_editor.md "The editor"), a view of the drafts'
// host page, index.html. Drawn only with what egui can draw (docs/plans/core/gui.md "Prototypes
// before implementation"): its behaviour is faked with sample data and nothing is written. The
// top bar's "Show preview" toggle opens a right-hand pane with the ruleset file's data block as
// the editor would write it, rebuilt on every edit.
// Widget mapping: <input type=number> = DragValue, <select> = ComboBox, .sel = SelectableLabel,
// <details> = CollapsingHeader, table.grid = Grid, .menu = menu_button, .window = Window/Modal,
// the colour swatch = color_edit_button, title="" = on_hover_text.
// The wrapper keeps the view's names out of the host's scope; its body is left unindented so
// the provenance script that reads the summary's blocks finds them at column 0.
// Names this script uses without declaring them (render, structural, h, row, weak, strong,
// button, combo, selectable, collapsing, menu, closeMenus, window_, closeWindow,
// named, registerTool) are the host's, index.html.
(() => {
"use strict";

// The view's own rules the shared stylesheet does not already hold (the host adds the string
// once, at registration): bleed makes the central panel a column the skeleton fills, and the
// host's 24px status-bar rule would shrink the view's bottom bar under it.
const RULESET_CSS = `
.center.bleed { padding: 0; overflow: hidden; display: flex; flex-direction: column; }
#rail { width: 150px; }
.data.hidden { display: none; }
#rs-bottom { height: auto; min-height: 26px; padding: 3px 8px; overflow: visible; }
#rs-bottom > span { display: inline; flex: 0 1 auto; gap: normal; white-space: normal; overflow: visible; min-width: auto; }
`;


// ---- Closed sets: pes_savefile's Team TOML labels ----
const STATS = ["attacking_prowess", "ball_control", "dribbling", "low_pass", "lofted_pass",
  "finishing", "place_kicking", "swerve", "heading", "defensive_prowess", "ball_winning",
  "kicking_power", "speed", "explosive_power", "body_control", "physical_contact", "jump",
  "goalkeeping", "catching", "clearing", "reflexes", "coverage", "stamina", "tight_possession",
  "aggression"];
const AT_MOST_EXTRA = ["form", "injury_resistance", "weak_foot_usage", "weak_foot_accuracy"];
const RANGE_VALUES = ["age", "weight", "form", "injury_resistance", "weak_foot_usage",
  "weak_foot_accuracy"].concat(STATS);
const SKILLS = ["scissors_feint", "flip_flap", "marseille_turn", "sombrero",
  "cut_behind_and_turn", "scotch_move", "heading", "long_range_drive", "knuckle_shot",
  "acrobatic_finishing", "heel_trick", "first_time_shot", "one_touch_pass", "weighted_pass",
  "pinpoint_crossing", "outside_curler", "rabona", "low_lofted_pass", "low_punt_trajectory",
  "long_throw", "gk_long_throw", "malicia", "man_marking", "track_back", "acrobatic_clear",
  "captaincy", "super_sub", "fighting_spirit", "double_touch", "crossover_turn",
  "step_on_skill", "chip_shot", "dipping_shots", "rising_shots", "no_look_pass", "gk_high_punt",
  "penalty_specialist", "gk_penalty_specialist", "interception", "long_range_shooting",
  "through_passing"];
const COM = ["trickster", "mazing_run", "speeding_bullet", "incisive_run", "long_ball_expert",
  "early_cross", "long_ranger"];
const CARDS = SKILLS.concat(COM);
const POSITIONS = ["GK", "CB", "LB", "RB", "DMF", "CMF", "LMF", "RMF", "AMF", "LWF", "RWF", "SS", "CF"];
const STYLES = ["goal_poacher", "dummy_runner", "fox_in_the_box", "target_man",
  "creative_playmaker", "prolific_winger", "roaming_flank", "crossing_specialist",
  "classic_no_10", "hole_player", "box_to_box", "the_destroyer", "orchestrator", "anchor_man",
  "offensive_fullback", "fullback_finisher", "defensive_fullback", "build_up",
  "extra_frontman", "offensive_goalkeeper", "defensive_goalkeeper"];
const VERSIONS = [15, 16, 17, 18, 19, 20, 21];
const CHOICE_EFFECTS = ["stat_bonus", "cards", "free", "free_cards", "a_positions",
  "max_a_positions", "weak_foot"];
const SECTIONS = [["general", "General"], ["tiers", "Tiers"], ["heights", "Heights"],
  ["cards", "Cards"], ["universal", "Universal rules"], ["specials", "Specials"],
  ["suggestions", "Suggestions"], ["custom", "Custom checks"], ["summary", "Summary"]];

// ---- The official file, crates/libs/aatf/rulesets/autumn26.rhai ----
const AUTUMN26 = {
  schema: 1, name: "4cc Autumn 26", squad_size: 23,
  stats: {
    rating: ["dribbling", "goalkeeping", "finishing", "low_pass", "lofted_pass", "heading",
      "swerve", "catching", "clearing", "reflexes", "coverage", "body_control",
      "physical_contact", "kicking_power", "explosive_power", "ball_control", "ball_winning",
      "jump", "place_kicking", "stamina", "speed", "aggression"],
    at_most: ["attacking_prowess", "defensive_prowess", "injury_resistance", "weak_foot_usage",
      "weak_foot_accuracy"],
  },
  tiers: [
    tier("gold", "Gold", 1, 99, 8, 3, 4, 6, { tricks: 3, com: 2 }),
    tier("silver", "Silver", 2, 92, 8, 3, 4, 5, { tricks: 3, com: 1 }),
    tier("bronze", "Bronze", 2, 86, 8, 3, 4, 4, { tricks: 3, com: 1 }),
    Object.assign(tier("goalkeeper", "Goalkeeper", null, 77, 4, 1, 2, 2, {}),
      { counts_as: "regular", when: { positions: ["GK"] } }),
    tier("regular", "Regular", null, 77, 4, 1, 2, 3, { tricks: 2 }),
  ],
  card_groups: {
    tricks: ["scissors_feint", "flip_flap", "marseille_turn", "sombrero", "cut_behind_and_turn",
      "scotch_move", "rabona", "double_touch", "crossover_turn", "step_on_skill", "no_look_pass"],
  },
  heights: {
    brackets: [
      { id: "manlet", label: "Manlet", up_to: 175 },
      { id: "mid", label: "Mid", up_to: 180 },
      { id: "tall", label: "Tall", exactly: [185] },
      { id: "giant", label: "Giant", exactly: [194] },
      { id: "giga", label: "Giga", exactly: [199] },
    ],
    systems: [
      { id: "green", label: "Green", any_player: { height_min: 194 },
        quotas: { giga: 0, giant: 6, tall: 6, mid: 5, manlet: 6 } },
      { id: "red", label: "Red", any_player: null,
        quotas: { giga: 0, giant: 0, tall: 10, mid: 7, manlet: 6 } },
    ],
  },
  universal: {
    registered_position_a: true, fielded_at_a: [], no_b_ratings: true, b_uses_a_allowance: false,
    gk_not_second_a: true, age: [15, 50], weight: { floor: 30, below: 129, above: 81 },
    captain_required: true, min_goalkeepers: 1, max_skill_cards: 10, max_com_styles: null,
  },
  specials: [
    special("Malicia is a free card", {}, { free_cards: ["malicia"] }),
    special("The captain's Captaincy card is free", { captain: true }, { free_cards: ["captaincy"] }),
    special("Medals cannot be goalkeepers", { tiers: ["gold", "silver", "bronze"], positions: ["GK"] },
      { forbid: "Medals cannot play as GK" }),
    special("Goalkeepers at 189 count as tall", { positions: ["GK"], height_min: 189, height_max: 189 },
      { height_allowance: 4 }),
    special("Tall goalkeepers are 189", { positions: ["GK"], height_min: 181, height_max: 188 },
      { forbid: "GKs in this bracket must be 189cm" }),
    special("Goalkeepers below giant", { positions: ["GK"], height_min: 194 },
      { forbid: "GK heights cannot be 194cm or more" }, "warning"),
    special("Gold below giant", { tiers: ["gold"], height_min: 194 },
      { forbid: "Gold heights cannot be 194cm or more" }, "warning"),
    special("Red CBs up to 189",
      { tiers: ["regular"], systems: ["red"], positions: ["CB"], fielded_only_at: "CB" },
      { height_allowance: 4 }),
    special("Manlets", { brackets: ["manlet"] },
      { cards: 1, a_positions: 1, weak_foot: { usage: 4, accuracy: 4 } }),
    special("Red manlet bonus", { systems: ["red"], height_max: 175 },
      { stat_bonus: { all: { regular: 5, goalkeeper: 5, bronze: 3, silver: 2 } } }),
    special("Green silver giants", { tiers: ["silver"], systems: ["green"], height_min: 194 },
      { stat_bonus: { all: -3 } }),
    special("Medals: a free A position or a free COM style", { tiers: ["gold", "silver", "bronze"] },
      { choice: [{ free: { com: 1 } }, { a_positions: 1 }] }),
  ],
  team_settings: [],
  suggestions: ["cards", "tricks", "com", "weak_foot", "injury_resistance"],
  custom_checks: null,
};

const CUSTOM_SAMPLE = `fn custom_checks(team) {
    // At most two registered positions among the 188 cm players.
    let seen = [];
    for player in team.players {
        if player.height == 188 && !(player.position in seen) { seen.push(player.position); }
    }
    if seen.len() > 2 {
        return [#{ kind: "Custom", text: "188 cm players share more than two positions" }];
    }
    []
}`;

// ---- The embedded templates: crates/libs/aatf/rulesets/*.rhai, "Embedded rulesets" order ----
// crates/libs/aatf/rulesets/spring26.rhai
const SPRING26 = {
  schema: 1, name: "4cc Spring 26", squad_size: 23,
  stats: clone(AUTUMN26.stats),
  tiers: [
    tier("gold", "Gold", 2, 99, 8, 3, { usage: 2, accuracy: 4 }, 5, { tricks: 3, com: 2 }),
    tier("silver", "Silver", 3, 88, 8, 3, { usage: 2, accuracy: 4 }, 4, { tricks: 3, com: 1 }),
    tier("goalkeeper", "Goalkeeper", null, 74, 4, 1, 2, 2, {},
      { counts_as: "regular", when: { positions: ["GK"] } }),
    tier("regular", "Regular", null, 77, 4, 1, 2, 3, { tricks: 2 }),
  ],
  card_groups: clone(AUTUMN26.card_groups),
  heights: {
    brackets: clone(AUTUMN26.heights.brackets),
    systems: [
      { id: "green", label: "Green", any_player: { height_min: 194 },
        quotas: { giga: 0, giant: 5, tall: 6, mid: 6, manlet: 6 } },
      { id: "red", label: "Red", any_player: null,
        quotas: { giga: 0, giant: 0, tall: 10, mid: 7, manlet: 6 } },
    ],
  },
  universal: clone(AUTUMN26.universal),
  specials: [
    special("Malicia is a free card", {}, { free_cards: ["malicia"] }),
    special("The captain's Captaincy card is free", { captain: true }, { free_cards: ["captaincy"] }),
    special("Medals cannot be goalkeepers", { tiers: ["gold", "silver"], positions: ["GK"] },
      { forbid: "Medals cannot play as GK" }),
    special("Goalkeepers at 189 count as tall", { positions: ["GK"], height_min: 189, height_max: 189 },
      { height_allowance: 4 }),
    special("Tall goalkeepers are 189", { positions: ["GK"], height_min: 181, height_max: 188 },
      { forbid: "GKs in this bracket must be 189cm" }),
    special("Goalkeepers below giant", { positions: ["GK"], height_min: 194 },
      { forbid: "GK heights cannot be 194cm or more" }, "warning"),
    special("Gold below giant", { tiers: ["gold"], height_min: 194 },
      { forbid: "Gold heights cannot be 194cm or more" }, "warning"),
    special("Manlets", { brackets: ["manlet"] },
      { cards: 1, a_positions: 1, weak_foot: { usage: 4, accuracy: 4 } }),
    special("Red manlet bonus", { systems: ["red"], height_max: 175 },
      { stat_bonus: { all: { regular: 5, goalkeeper: 5 } } }),
  ],
  team_settings: [],
  suggestions: ["cards", "tricks", "com", "weak_foot", "injury_resistance"],
  custom_checks: null,
};

// crates/libs/aatf/rulesets/vgl27.rhai
const VGL27 = {
  schema: 1, name: "/vg/ League 27", squad_size: 23,
  stats: {
    rating: ["dribbling", "goalkeeping", "low_pass", "lofted_pass", "swerve",
      "catching", "clearing", "reflexes", "coverage", "body_control",
      "physical_contact", "kicking_power", "explosive_power", "ball_control",
      "ball_winning", "jump", "place_kicking", "stamina", "speed"],
    at_most: ["attacking_prowess", "defensive_prowess", "weak_foot_usage",
      "weak_foot_accuracy"],
  },
  tiers: [
    tier("gold", "Gold", 1, 99, 8, 2, 4, 6, { tricks: 12, com: 1 },
      { when: { not: { positions: ["GK"] } }, a_positions: 2 }),
    tier("silver", "Silver", 2, 94, 8, 2, 4, 5, { tricks: 12, com: 1 },
      { when: { not: { positions: ["GK"] } }, a_positions: 2 }),
    tier("bronze", "Bronze", 2, 89, 8, 2, 4, 5, { tricks: 12, com: 1 },
      { when: { not: { positions: ["GK"] } }, a_positions: 2 }),
    tier("goalkeeper", "Goalkeeper", null, 77, 4, 1, 2, 4, { tricks: 12 },
      { counts_as: "regular", when: { positions: ["GK"] } }),
    tier("regular", "Regular", null, 77, 4, 1, 2, 4, { tricks: 12 },
      { when: { not: { positions: ["GK"] } }, a_positions: 2 }),
  ],
  card_groups: {
    tricks: ["scissors_feint", "double_touch", "flip_flap", "marseille_turn",
      "sombrero", "crossover_turn", "cut_behind_and_turn", "scotch_move",
      "step_on_skill", "rabona", "no_look_pass", "malicia"],
  },
  heights: {
    brackets: [
      { id: "manlet", label: "Manlet", exactly: [175] },
      { id: "mid", label: "Mid", exactly: [180] },
      { id: "tall", label: "Tall", exactly: [185] },
      { id: "giant", label: "Giant", exactly: [190] },
      { id: "colossal", label: "Colossal", exactly: [210] },
    ],
    systems: [
      { id: "purple", label: "Purple", any_player: null,
        quotas: { colossal: 0, giant: 0, tall: 10, mid: 7, manlet: 6 } },
    ],
  },
  universal: {
    registered_position_a: true, fielded_at_a: [1], no_b_ratings: true,
    b_uses_a_allowance: false, gk_not_second_a: true, age: [15, 50],
    weight: { floor: 30, below: 129, above: 81 }, captain_required: true,
    min_goalkeepers: 1, max_skill_cards: 41, max_com_styles: null,
  },
  specials: [
    special("The captain's Captaincy card is free, plus one card, COM style or A position",
      { captain: true },
      { free_cards: ["captaincy"], choice: [{ cards: 1 }, { free: { com: 1 } }, { a_positions: 1 }] }),
    special("Goalkeepers at 189 count as tall", { positions: ["GK"], height_min: 189, height_max: 189 },
      { height_allowance: 4 }),
    special("Tall goalkeepers are 189", { positions: ["GK"], height_min: 185, height_max: 185 },
      { forbid: "GKs in this bracket must be 189cm" }),
    special("Manlets", { brackets: ["manlet"] },
      { cards: 1, a_positions: 1, weak_foot: { usage: 4, accuracy: 4 } }),
  ],
  team_settings: [
    tset("No man marking", { man_marking: false }),
    tset("Support settings", { auto_substitution: false, auto_offside_trap: false,
      auto_preset_change: false }),
  ],
  suggestions: ["cards", "com", "a_positions", "weak_foot", "free_cards"],
  custom_checks: null,
};

// crates/libs/aatf/rulesets/vgl26.rhai
const VGL26 = {
  schema: 1, name: "/vg/ League 26", squad_size: 23,
  stats: {
    rating: ["dribbling", "goalkeeping", "low_pass", "lofted_pass", "swerve",
      "catching", "clearing", "reflexes", "coverage", "body_control",
      "physical_contact", "kicking_power", "explosive_power", "ball_control",
      "ball_winning", "jump", "place_kicking", "stamina", "speed"],
    at_most: ["attacking_prowess", "defensive_prowess", "weak_foot_usage",
      "weak_foot_accuracy"],
  },
  tiers: [
    tier("gold", "Gold", 1, 99, 8, 2, 4, 9, { tricks: 12, com: 2 },
      { when: { height_min: 195, height_max: 195 },
        stats: { place_kicking: 94, ball_winning: 94, defensive_prowess: 60 },
        a_positions: 2 }),
    tier("silver", "Silver", 2, 95, 8, 2, 4, 8, { tricks: 12, com: 2 },
      { when: { height_min: 195, height_max: 195 },
        stats: { attacking_prowess: 99, finishing: 99, heading: 99,
          place_kicking: 90, ball_winning: 90, defensive_prowess: 60 },
        a_positions: 2 }),
    tier("bronze", "Bronze", 2, 91, 8, 2, 4, 8, { tricks: 12, com: 2 },
      { when: { height_min: 190, height_max: 190 },
        stats: { attacking_prowess: 96, finishing: 96, heading: 96,
          place_kicking: 86, ball_winning: 86, defensive_prowess: 60 },
        a_positions: 2 }),
    tier("buffed", "Buffed", 8, 85, 4, 1, 4, 7, { tricks: 12, com: 1 },
      { when: { height_min: 185, height_max: 185, not: { positions: ["GK"] } },
        stats: { attacking_prowess: 90, aggression: 90, jump: 75,
          stamina: 70, ball_winning: 65, defensive_prowess: 60 },
        a_positions: 2 }),
    tier("goalkeeper", "Goalkeeper", null, 72, 8, 1, 4, 5, { tricks: 12, com: 1 },
      { counts_as: "regular", when: { positions: ["GK"] } }),
    tier("regular", "Regular", 10, 80, 4, 1, 4, 6, { tricks: 12, com: 1 },
      { when: { height_min: 180, height_max: 180 },
        stats: { aggression: 90, jump: 80, defensive_prowess: 70, ball_winning: 70,
          explosive_power: 70, goalkeeping: 72, catching: 72, clearing: 72,
          reflexes: 72, coverage: 72, stamina: 65 },
        a_positions: 2 }),
  ],
  card_groups: {
    tricks: ["scissors_feint", "double_touch", "flip_flap", "marseille_turn",
      "sombrero", "crossover_turn", "cut_behind_and_turn", "scotch_move",
      "step_on_skill", "rabona", "no_look_pass", "malicia"],
  },
  heights: { brackets: [], systems: [] },
  universal: {
    registered_position_a: true, fielded_at_a: [1], no_b_ratings: true,
    b_uses_a_allowance: false, gk_not_second_a: true, age: [15, 50],
    weight: { floor: 30, below: 129, above: 81 }, captain_required: true,
    min_goalkeepers: 1, max_skill_cards: 41, max_com_styles: null,
  },
  specials: [
    special("Heading is a free card", {}, { free_cards: ["heading"] }),
    special("The captain's Captaincy card and one more are free", { captain: true },
      { free_cards: ["captaincy"], cards: 1 }),
    special("Medals cannot be goalkeepers", { tiers: ["gold", "silver", "bronze"], positions: ["GK"] },
      { forbid: "Medals cannot play as GK" }),
    special("Goalkeepers are 185", { positions: ["GK"], not: { height_min: 185, height_max: 185 } },
      { forbid: "GKs must be 185cm" }),
  ],
  team_settings: [
    tset("No man marking", { man_marking: false }),
    tset("Support settings", { auto_substitution: false, auto_offside_trap: false,
      auto_preset_change: false }),
    tset("No 3-4-3 formations", { forbidden_shapes: [[3, 4, 3]] }),
  ],
  suggestions: ["cards", "com", "a_positions", "weak_foot", "free_cards"],
  custom_checks: null,
};

// crates/libs/aatf/rulesets/vtl11.rhai
const VTL11 = {
  schema: 1, name: "/vt/ League 11", squad_size: 23,
  stats: { rating: ["clearing"], at_most: [] },
  tiers: [
    tier("gold", "Gold", 2, 99, 8, 1, 4, 7, { com: 3 },
      { when: { height_min: 195, height_max: 195, ranges: { swerve: [99, 99] } },
        stats: { place_kicking: 94, defensive_prowess: 65, ball_winning: 94,
          speed: 94, physical_contact: 94, stamina: 94 },
        min_cards: 3, a_positions: 2, name_color: "cc9900ff" }),
    tier("silver", "Silver", 3, 89, 8, 1, 4, 7, { com: 2 },
      { when: { height_min: 195, height_max: 195, ranges: { swerve: [89, 89] } },
        stats: { attacking_prowess: 94, heading: 94, defensive_prowess: 57,
          tight_possession: 94, aggression: 94 },
        min_cards: 3, a_positions: 3, name_color: "51bbc4ff" }),
    tier("regular", "Regular", 10, 77, 4, 1, 4, 7, { com: 1 },
      { when: { height_min: 180, height_max: 180 },
        stats: { attacking_prowess: 87, ball_control: 87, dribbling: 87,
          low_pass: 87, lofted_pass: 92, finishing: 92,
          place_kicking: 70, swerve: 87, defensive_prowess: 73,
          ball_winning: 70, kicking_power: 87, physical_contact: 70,
          jump: 70, catching: 70, reflexes: 73, coverage: 73,
          stamina: 62, tight_possession: 87, aggression: 92 },
        min_cards: 3, a_positions: 2 }),
    tier("buffed", "Buffed", 8, 77, 4, 1, 4, 7, { com: 1 },
      { when: { height_min: 188, height_max: 188 },
        stats: { attacking_prowess: 92, ball_control: 87, dribbling: 92,
          low_pass: 87, lofted_pass: 87, finishing: 92,
          place_kicking: 70, swerve: 87, defensive_prowess: 47,
          ball_winning: 70, kicking_power: 92, speed: 87,
          explosive_power: 87, body_control: 87, physical_contact: 82,
          catching: 70, reflexes: 73, coverage: 73, stamina: 62,
          tight_possession: 87, aggression: 87 },
        min_cards: 3, a_positions: 1 }),
  ],
  card_groups: {},
  heights: { brackets: [], systems: [] },
  universal: {
    registered_position_a: true, fielded_at_a: [], no_b_ratings: false,
    b_uses_a_allowance: true, gk_not_second_a: true, age: [15, 50],
    weight: { floor: 30, below: 129, above: 81 }, captain_required: false,
    min_goalkeepers: 2, max_skill_cards: 11, max_com_styles: 5,
  },
  specials: [
    special("The free skills", {}, { free_cards: [
      "no_look_pass", "double_touch", "rabona", "acrobatic_finishing",
      "first_time_shot", "dipping_shots", "rising_shots", "heading",
      "malicia", "flip_flap", "marseille_turn", "scotch_move",
      "crossover_turn"] }),
    special("The pass skills are free to non-medals", { tiers: ["regular", "buffed"] },
      { free_cards: ["one_touch_pass", "weighted_pass", "pinpoint_crossing"] }),
    special("The captain's Captaincy card is free", { captain: true },
      { free_cards: ["captaincy"] }),
    special("Long Ranger is a free COM style", {}, { free_cards: ["long_ranger"] }),
    special("Malicia is required", {}, { require_cards: ["malicia"] }),
    special("The GK Penalty Specialist card is banned", { holds: ["gk_penalty_specialist"] },
      { forbid: "The GK Penalty Specialist card is banned" }),
    special("Regulars cannot be captain", { tiers: ["regular", "buffed"], captain: true },
      { forbid: "Regulars can't be captain" }),
    special("Medals cannot be goalkeepers", { tiers: ["gold", "silver"], positions: ["GK"] },
      { forbid: "Medals cannot play as GK" }),
    special("B positions only for 188cm players at CB", { any: [
      { tiers: ["gold", "silver", "regular"],
        rated_b_at: ["GK", "CB", "LB", "RB", "DMF", "CMF", "LMF", "RMF",
          "AMF", "LWF", "RWF", "SS", "CF"] },
      { tiers: ["buffed"],
        rated_b_at: ["GK", "LB", "RB", "DMF", "CMF", "LMF", "RMF",
          "AMF", "LWF", "RWF", "SS", "CF"] }] },
      { forbid: "Illegal B positions, only allowed on 188cm players at CB" }),
    special("188cm players have no A at CB or GK", { tiers: ["buffed"], rated_a_at: ["CB", "GK"] },
      { forbid: "188cm players may not have an A-position at CB or GK" }),
    special("A goalkeeper's form is 8", { tiers: ["regular"], positions: ["GK"] },
      { form: 4 }),
    special("A gold Fox in the Box", { tiers: ["gold"], playing_styles: ["fox_in_the_box"] },
      { stat_bonus: { ball_winning: 5, defensive_prowess: -5 } }),
    special("A gold Target Man", { tiers: ["gold"], playing_styles: ["target_man"] },
      { stat_bonus: { physical_contact: 5, defensive_prowess: -5 } }),
    special("A gold Creative Playmaker", { tiers: ["gold"], playing_styles: ["creative_playmaker"] },
      { stat_bonus: { stamina: 5, defensive_prowess: -5 } }),
    special("Goal Poacher at CF", { playing_styles: ["goal_poacher"], rated_a_at: ["CF"] },
      { forbid: "Goal Poacher does not activate at CF with the league's exe" }, "warning"),
    special("Creative Playmaker at CF", { playing_styles: ["creative_playmaker"], rated_a_at: ["CF"] },
      { forbid: "Creative Playmaker activates at CF with the league's exe" }, "warning"),
  ],
  team_settings: [
    tset("No man marking on PES 16", { versions: [16], man_marking: false }),
    tset("PES 16 support settings", { versions: [16], auto_substitution: false,
      auto_offside_trap: false, auto_preset_change: false }),
  ],
  suggestions: [],
  custom_checks: `// What the schema cannot carry: the goalkeeper maximum (min_goalkeepers is a floor), and the
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
}`,
};

// crates/libs/aatf/rulesets/vtl10.rhai
const VTL10 = {
  schema: 1, name: "/vt/ League 10", squad_size: 23,
  stats: { rating: ["clearing"], at_most: [] },
  tiers: [
    tier("gold", "Gold", 2, 99, 8, 1, 4, 6, { com: 2 },
      { when: { height_min: 194, height_max: 194, ranges: { swerve: [99, 99] } },
        stats: { place_kicking: 96, defensive_prowess: 58 },
        min_cards: 3, a_positions: 2 }),
    tier("silver", "Silver", 3, 92, 8, 1, 4, 7, { com: 2 },
      { when: { height_min: 194, height_max: 194, ranges: { swerve: [92, 92] } },
        stats: { attacking_prowess: 95, place_kicking: 88, defensive_prowess: 56,
          ball_winning: 88, stamina: 95, aggression: 88 },
        min_cards: 3, a_positions: 3 }),
    tier("regular", "Regular", 10, 77, 4, 1, 4, 7, { com: 1 },
      { when: { any: [{ height_min: 178, height_max: 178 },
                      { height_min: 180, height_max: 180 }] },
        stats: { attacking_prowess: 87, ball_control: 87, dribbling: 87,
          low_pass: 87, lofted_pass: 92, finishing: 97,
          place_kicking: 70, swerve: 87, defensive_prowess: 73,
          ball_winning: 68, kicking_power: 87, explosive_power: 73,
          physical_contact: 68, jump: 67, goalkeeping: 75,
          catching: 70, reflexes: 73, coverage: 73, stamina: 55,
          tight_possession: 87, aggression: 92 },
        min_cards: 3, a_positions: 2 }),
    tier("buffed", "Buffed", 8, 77, 4, 1, 4, 7, { com: 1 },
      { when: { height_min: 188, height_max: 188 },
        stats: { attacking_prowess: 92, ball_control: 87, dribbling: 92,
          low_pass: 87, lofted_pass: 87, finishing: 92,
          place_kicking: 70, swerve: 87, defensive_prowess: 53,
          ball_winning: 68, kicking_power: 92, speed: 88,
          explosive_power: 88, body_control: 88, physical_contact: 88,
          goalkeeping: 75, catching: 70, reflexes: 73, coverage: 73,
          stamina: 60, tight_possession: 87, aggression: 87 },
        min_cards: 3, a_positions: 1 }),
  ],
  card_groups: {},
  heights: { brackets: [], systems: [] },
  universal: {
    registered_position_a: true, fielded_at_a: [], no_b_ratings: true,
    b_uses_a_allowance: false, gk_not_second_a: true, age: [15, 50],
    weight: { floor: 30, below: 129, above: 81 }, captain_required: false,
    min_goalkeepers: 2, max_skill_cards: 12, max_com_styles: 5,
  },
  specials: [
    special("The free skills", {}, { free_cards: [
      "scissors_feint", "flip_flap", "marseille_turn", "sombrero",
      "cut_behind_and_turn", "scotch_move", "acrobatic_finishing",
      "heel_trick", "first_time_shot", "malicia", "double_touch",
      "crossover_turn", "step_on_skill"] }),
    special("The pass skills are free to non-medals", { tiers: ["regular", "buffed"] },
      { free_cards: ["one_touch_pass", "weighted_pass", "pinpoint_crossing"] }),
    special("The captain's Captaincy card is free", { captain: true },
      { free_cards: ["captaincy"] }),
    special("Incisive Run and Long Ranger are free COM styles", {},
      { free_cards: ["incisive_run", "long_ranger"] }),
    special("Malicia is required", {}, { require_cards: ["malicia"] }),
    special("Regulars cannot be captain", { tiers: ["regular", "buffed"], captain: true },
      { forbid: "Regulars can't be captain" }),
    special("Medals cannot be goalkeepers", { tiers: ["gold", "silver"], positions: ["GK"] },
      { forbid: "Medals cannot play as GK" }),
    special("188cm players have no A at CB or GK", { tiers: ["buffed"], rated_a_at: ["CB", "GK"] },
      { forbid: "188cm players may not have an A-position at CB or GK" }),
    special("A goalkeeper's form is 8", { tiers: ["regular"], positions: ["GK"] },
      { form: 4 }),
    special("Regular goalkeepers are 180cm",
      { tiers: ["regular"], positions: ["GK"], height_min: 178, height_max: 178 },
      { forbid: "Regular goalkeepers must be 180cm" }),
    special("Outfield regulars are 178cm",
      { tiers: ["regular"], height_min: 180, height_max: 180, not: { positions: ["GK"] } },
      { forbid: "Outfield regulars must be 178cm" }),
    special("Buffed wingbacks are 2/2 weak foot", { tiers: ["buffed"], rated_a_at: ["LB", "RB"] },
      { weak_foot: { usage: 2, accuracy: 2 } }),
  ],
  team_settings: [
    tset("No man marking on PES 16", { versions: [16], man_marking: false }),
    tset("PES 16 support settings", { versions: [16], auto_substitution: false,
      auto_offside_trap: false, auto_preset_change: false }),
  ],
  suggestions: [],
  custom_checks: `// What the schema cannot carry: the goalkeeper maximum (min_goalkeepers is a floor), and the
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
}`,
};

// crates/libs/aatf/rulesets/dbc7.rhai
const DBC7 = {
  schema: 1, name: "Divegrass Before Christmas 7", squad_size: 23,
  stats: {
    rating: ["dribbling", "goalkeeping", "finishing", "low_pass", "lofted_pass", "heading",
      "swerve", "catching", "clearing", "reflexes", "coverage", "body_control",
      "physical_contact", "kicking_power", "explosive_power", "ball_control",
      "ball_winning", "jump", "place_kicking", "stamina", "speed", "aggression"],
    at_most: ["attacking_prowess", "ball_control", "dribbling", "low_pass", "lofted_pass",
      "finishing", "place_kicking", "swerve", "heading", "defensive_prowess",
      "ball_winning", "kicking_power", "speed", "explosive_power", "body_control",
      "physical_contact", "jump", "goalkeeping", "catching", "clearing",
      "reflexes", "coverage", "stamina", "tight_possession", "aggression",
      "form", "injury_resistance", "weak_foot_usage", "weak_foot_accuracy"],
  },
  tiers: [
    tier("gold", "Gold", null, 99, 8, 3, 4, 48, {}, { a_positions: 13 }),
    tier("silver", "Silver", null, 88, 8, 3, 4, 48, {}, { a_positions: 13, stat_warning: 5,
      total_within_targets: true }),
    tier("regular", "Regular", null, 77, 8, 3, 4, 48, {}, { a_positions: 13, stat_warning: 7,
      total_within_targets: true }),
  ],
  medal_systems: [
    { label: "GGG", counts: { gold: 3 } },
    { label: "GGSS", counts: { gold: 2, silver: 2 } },
    { label: "GSSSS", counts: { gold: 1, silver: 4 } },
    { label: "SSSSSS", counts: { silver: 6 } },
  ],
  card_groups: {},
  heights: {
    brackets: [
      { id: "manlet", label: "Manlet", exactly: [175] },
      { id: "mid", label: "Mid", exactly: [180] },
      { id: "tall", label: "Tall", exactly: [185] },
      { id: "giant", label: "Giant", exactly: [195] },
    ],
    systems: [
      { id: "height", label: "Height", any_player: null, by_quotas: true,
        quotas: { giant: 6, tall: 6, mid: 5, manlet: 6 } },
      { id: "manlet", label: "Manlet", any_player: null, by_quotas: true,
        quotas: { giant: 0, tall: 10, mid: 7, manlet: 6 } },
      { id: "universal", label: "Universal", any_player: null, by_quotas: true,
        quotas: { giant: 0, tall: 23, mid: 0, manlet: 0 } },
      { id: "giga", label: "GigaHeight", any_player: null,
        quotas: { giant: 0, tall: 7, mid: 10, manlet: 6 },
        extra_height: { total: 50, max: 210 } },
    ],
  },
  universal: { registered_position_a: false, fielded_at_a: [], no_b_ratings: false,
    b_uses_a_allowance: false, gk_not_second_a: false, age: null, weight: null,
    captain_required: false, min_goalkeepers: 0, max_skill_cards: 41, max_com_styles: null },
  specials: [
    special("Goalkeepers at 189 count as 185", { positions: ["GK"], height_min: 189, height_max: 189 },
      { height_allowance: 4 }),
    special("Manlet bonus", { systems: ["manlet"], height_max: 175 },
      { choice: [{ stat_bonus: { all: { regular: 5, silver: 5 } } }] }),
    special("Manlet regulars: warning band 5", { systems: ["manlet"], height_max: 175 },
      { stat_warning: { regular: 5 } }),
    special("Goalkeepers at 189 have no second A position", { positions: ["GK"], height_min: 189, height_max: 189 },
      { max_a_positions: 1 }),
    special("The registered position must be rated A",
      { any: POSITIONS.map(p => ({ positions: [p], not: { rated_a_at: [p] } })) },
      { forbid: "The registered position is not rated A" }, "warning"),
  ],
  team_settings: [],
  suggestions: [],
  custom_checks: null,
};

// VGL26's block extended so that every item of the plain-language proposal
// (aatf_rules.md "Coverage of a plain-language proposal") the schema can
// carry is used at least once; each added special's label names its item.
const CLOUDS = {
  schema: 1, name: "Clouds' proposal, every item", squad_size: 23,
  stats: clone(VGL26.stats),
  tiers: clone(VGL26.tiers),
  card_groups: clone(VGL26.card_groups),
  heights: {
    brackets: [
      { id: "manlet", label: "Manlet", up_to: 175 },
      { id: "mid", label: "Mid", up_to: 180 },
      { id: "tall", label: "Tall", up_to: 185 },
      { id: "giant", label: "Giant", up_to: 190 },
      { id: "giga", label: "Giga", up_to: 210 },
    ],
    systems: [
      { id: "standard", label: "Standard", any_player: null,
        quotas: { giga: 3, giant: 2, manlet: 0 } },
    ],
  },
  universal: clone(VGL26.universal),
  specials: [
    special("Heading is a free card", {}, { free_cards: ["heading"] }),
    special("The captain's Captaincy card is free, plus one card, COM style or A position (choice)",
      { captain: true },
      { free_cards: ["captaincy"], choice: [{ cards: 1 }, { free: { com: 1 } }, { a_positions: 1 }] }),
    special("Medals cannot be goalkeepers", { tiers: ["gold", "silver", "bronze"], positions: ["GK"] },
      { forbid: "Medals cannot play as GK" }),
    special("Goalkeepers are 185", { positions: ["GK"], not: { height_min: 185, height_max: 185 } },
      { forbid: "GKs must be 185cm" }),
    special("Buffed players at tall: +2 to every stat (brackets, stats_global_mod)",
      { tiers: ["buffed"], brackets: ["tall"] },
      { stat_bonus: { all: 2 } }),
    special("Medals at giga: +1 A position (brackets)",
      { tiers: ["gold", "silver", "bronze"], brackets: ["giga"] },
      { a_positions: 1 }),
    special("Medals below giga: +1 card (ELSE, the rule above under not)",
      { tiers: ["gold", "silver", "bronze"], not: { brackets: ["giga"] } },
      { cards: 1 }),
    special("Regular CBs fielded only at CB count 4 cm shorter (fielded_only_at, allowed_height_mod)",
      { tiers: ["regular"], positions: ["CB"], fielded_only_at: "CB" },
      { height_allowance: 4 }),
    special("A Fox in the Box: +3 attacking prowess at most (playing_style, stats_<stat>_mod)",
      { playing_styles: ["fox_in_the_box"] },
      { stat_bonus: { attacking_prowess: 3 } }),
    special("Left-footed wingers: +1 paid card (strong_foot)",
      { strong_foot: "left", positions: ["LWF", "LMF"] },
      { cards: 1 }),
    special("Speed 95 or more: a free COM style (stats_<stat> in a condition)",
      { ranges: { speed: [95, 99] } },
      { free: { com: 1 } }),
    special("Weight 80 kg or more: +1 injury resistance (ranges)",
      { ranges: { weight: [80, 130] } },
      { injury_resistance: 1 }),
    special("Form 8: +1 A position (form in a condition)",
      { ranges: { form: [8, 8] } },
      { a_positions: 1 }),
    special("Holds Fighting Spirit: +1 paid card (cards_<card> in a condition)",
      { holds: ["fighting_spirit"] },
      { cards: 1 }),
    special("195 cm or a Target Man: +1 card (any)",
      { any: [{ height_min: 195 }, { playing_styles: ["target_man"] }] },
      { cards: 1 }),
    special("Buffed wingers: weak foot 2/2 (allowed_weak_foot_acc/use_mod, =)",
      { tiers: ["buffed"], positions: ["LWF", "RWF"] },
      { weak_foot: { usage: 2, accuracy: 2 } }),
    special("Buffed and regulars play on form 8 (allowed_form_mod)",
      { tiers: ["buffed", "regular"] },
      { form: 4 }),
    special("Goalkeepers' goalkeeping is 85 (stat_target)",
      { tiers: ["goalkeeper"] },
      { stat_target: { goalkeeping: 85 } }),
  ],
  team_settings: [
    tset("No man marking (PES 21)", { versions: [21], man_marking: false }),
    tset("Support settings", { auto_substitution: false, auto_offside_trap: false,
      auto_preset_change: false, auto_attack_defence_levels: false }),
    tset("No 3-4-3 formations", { forbidden_shapes: [[3, 4, 3]] }),
  ],
  suggestions: clone(VGL26.suggestions),
  custom_checks: `// Two example rules (not from the proposal) reading what the schema cannot carry: the
// proposal's preset<N>_<formation>_player_pos and _x/_y variables, a formation's positions
// and coordinates rather than a player's fields.
fn custom_checks(team) {
    let found = [];
    // No non-goalkeeper is fielded at GK in presets 2 and 3 (preset 1 is the schema's).
    for player in team.players {
        if player.position == "GK" { continue; }
        for preset in [2, 3] {
            if "GK" in team.fielded_positions(player, preset) {
                found.push(#{ kind: "Custom",
                              text: "A non-goalkeeper is fielded at GK after preset 1",
                              slot: player.slot });
            }
        }
    }
    // No two players stand on the same spot in a formation.
    for meta in team.formation_shapes() {
        let formation = team.formation(meta.preset, meta.formation);
        if formation == () { continue; }
        for i in 0..formation.len() {
            for j in (i + 1)..formation.len() {
                if formation[i].x == formation[j].x && formation[i].y == formation[j].y {
                    found.push(#{ kind: "Custom",
                                  text: "Two players share a spot in a formation" });
                }
            }
        }
    }
    found
}`,
};

function tier(id, label, count, rate, form, injury, weakFoot, cards, free, extra) {
  const wf = typeof weakFoot === "number" ? { usage: weakFoot, accuracy: weakFoot } : weakFoot;
  return Object.assign({ id, label, count, counts_as: null, when: {}, rate, stats: {}, form,
    injury_resistance: injury, weak_foot: wf, cards,
    min_cards: 0, free, a_positions: 1, name_color: null, stat_warning: 0,
    total_within_targets: false }, extra);
}
function special(label, when, then, severity) {
  return { label, when, then, severity: severity || "error" };
}
function tset(label, fields) {
  return Object.assign({ label, versions: [], man_marking: null, auto_substitution: null,
    auto_offside_trap: null, auto_preset_change: null, auto_attack_defence_levels: null,
    forbidden_shapes: [] }, fields);
}
function blank() {
  return {
    schema: 1, name: "New ruleset", squad_size: 23,
    stats: clone(AUTUMN26.stats),
    tiers: [tier("regular", "Regular", null, 75, 4, 1, 2, 3, {})],
    card_groups: {},
    heights: { brackets: [], systems: [] },
    universal: { registered_position_a: true, fielded_at_a: [], no_b_ratings: true,
      b_uses_a_allowance: false, gk_not_second_a: true, age: null, weight: null,
      captain_required: true, min_goalkeepers: 1, max_skill_cards: 10, max_com_styles: null },
    specials: [], team_settings: [], suggestions: [], custom_checks: null,
  };
}

// ---- State ----
let R = null;               // the ruleset being edited
// The loaded template's link key (R's name is editable, so it cannot name it).
let loadedKey = "autumn26";
let section = "general";
let undo = [], redo = [], last = "";
let saved = "";
let previousLines = [];
// The link's `ruleset` values, one per loadable template; each entry builds
// the object to load (the blank and the custom-check sample are made on ask).
const TEMPLATES = { autumn26: () => AUTUMN26, spring26: () => SPRING26,
  vgl27: () => VGL27, vgl26: () => VGL26, vtl11: () => VTL11, vtl10: () => VTL10,
  dbc7: () => DBC7, clouds: () => CLOUDS, blank: () => blank(),
  "custom-check": () => Object.assign(clone(AUTUMN26),
    { name: "4cc Autumn 26 + custom check", custom_checks: CUSTOM_SAMPLE }) };

function clone(v) { return JSON.parse(JSON.stringify(v)); }
function human(id) {
  return id.split("_").map(w => w === "gk" || w === "com" ? w.toUpperCase()
    : w[0].toUpperCase() + w.slice(1)).join(" ");
}

// ---- Widgets (each one an egui widget) ----
function checkbox(text, value, set, title) {
  return h("label", { class: "cb", title }, h("input", { type: "checkbox", checked: value,
    onchange: e => { set(e.target.checked); structural(); } }), text);
}
function drag(value, set, opts = {}) {
  return h("input", { type: "number", value: value ?? "", min: opts.min ?? 0, max: opts.max ?? 255,
    placeholder: opts.placeholder, title: opts.title, style: opts.width ? `width:${opts.width}px` : "",
    oninput: e => { const v = e.target.value === "" ? null : Number(e.target.value); set(v); changed(); },
    onchange: commit });
}
function text(value, set, opts = {}) {
  return h("input", { type: "text", value: value ?? "", class: opts.mono ? "mono" : "",
    style: `width:${opts.width || 140}px`, placeholder: opts.placeholder, title: opts.title,
    oninput: e => { set(e.target.value); changed(); }, onchange: commit });
}
function chips(all, list, set, label = human) {
  return h("span", { class: "row", style: "margin:0" }, all.map(v => selectable(label(v), list.includes(v),
    () => set(list.includes(v) ? list.filter(x => x !== v) : list.concat([v])))));
}
function colour(hex8, set) {
  const rgb = /^[0-9a-f]{6}/i.test(hex8 || "") ? "#" + hex8.slice(0, 6) : "#000000";
  return h("span", { class: "row", style: "margin:0; gap:3px" },
    h("input", { type: "color", value: rgb, title: "Colour (the code's last two digits are the alpha)",
      oninput: e => { set(e.target.value.slice(1) + ((hex8 || "").slice(6, 8) || "ff")); changed(); },
      onchange: structural }),
    text(hex8, set, { mono: true, width: 76, title: "The eight bytes after \\x11c in the name" }));
}
function remove(list, i) { return button("✕", () => { list.splice(i, 1); structural(); }, { frame0: true, title: "Remove" }); }

// Drag and drop between rows of one list (egui's dnd_drag_source / dnd_drop_zone).
function reorderable(el, list, i) {
  el.addEventListener("dragover", e => { e.preventDefault(); el.classList.add("dragover"); });
  el.addEventListener("dragleave", () => el.classList.remove("dragover"));
  el.addEventListener("drop", e => {
    e.preventDefault(); el.classList.remove("dragover");
    const from = Number(e.dataTransfer.getData("text/plain"));
    if (Number.isNaN(from) || from === i) return;
    const [moved] = list.splice(from, 1);
    list.splice(i, 0, moved);
    structural();
  });
  return el;
}
function handle(i) {
  return h("span", { class: "handle", draggable: "true", title: "Drag to reorder",
    ondragstart: e => e.dataTransfer.setData("text/plain", String(i)) }, "⠇");
}

// ---- Edits, undo, outputs ----
function commit() {
  const now = JSON.stringify(R);
  if (now === last) return;
  undo.push(last); redo = []; last = now;
  refresh();
}
function changed() { refresh(); }
function load(ruleset, key, noRender) {
  R = clone(ruleset); R.medal_systems = R.medal_systems || []; loadedKey = key;
  undo = []; redo = []; last = JSON.stringify(R); saved = last; previousLines = [];
  if (!noRender) render();
}
function loadTemplate(key) { load(TEMPLATES[key](), key); closeWindow(); }
function doUndo() { if (!undo.length) return; redo.push(last); last = undo.pop(); R = JSON.parse(last); render(); }
function doRedo() { if (!redo.length) return; undo.push(last); last = redo.pop(); R = JSON.parse(last); render(); }
function onKey(e) {
  if (!(e.ctrlKey || e.metaKey) || e.target.tagName === "INPUT") return;
  if (e.key === "z") { e.preventDefault(); doUndo(); }
  if (e.key === "y") { e.preventDefault(); doRedo(); }
}

// ---- The data block, as the editor writes it (aatf_rules.md "The rules file") ----
// A map key as Rhai reads it: bare when an identifier, else a string literal (an id is any
// non-empty string).
const rk = k => /^[A-Za-z_][A-Za-z0-9_]*$/.test(k) ? k : JSON.stringify(k);
function rv(v) {
  if (v === null || v === undefined) return "()";
  if (typeof v === "string") return JSON.stringify(v);
  if (typeof v !== "object") return String(v);
  if (Array.isArray(v)) return "[" + v.map(rv).join(", ") + "]";
  const keys = Object.keys(v);
  return keys.length ? "#{ " + keys.map(k => rk(k) + ": " + rv(v[k])).join(", ") + " }" : "#{}";
}
const isEmpty = v => v === null || v === undefined || v === "" || v === false || v === 0 ||
  (Array.isArray(v) && !v.length) || (typeof v === "object" && !Array.isArray(v) && v && !Object.keys(v).length);
// `fields`: [key, mode], mode "req" (always), "opt" (left out only when absent), else left out
// when empty: a field left out takes its empty value.
function struct(obj, fields, conv = {}) {
  const out = {};
  for (const [k, mode] of fields) {
    let v = obj[k];
    if (conv[k] && v !== null && v !== undefined) v = conv[k](v);
    if (mode === "req" || (mode === "opt" ? v !== null && v !== undefined : !isEmpty(v))) out[k] = v;
  }
  return out;
}
function condData(c) {
  if (!c) return {};
  return struct(c, [["tiers"], ["positions"], ["systems"], ["brackets"], ["height_min", "opt"],
    ["height_max", "opt"], ["captain", "opt"], ["playing_styles"], ["holds"], ["rated_a_at"],
    ["rated_b_at"], ["name_colors"], ["fielded_only_at", "opt"], ["ranges"], ["strong_foot", "opt"],
    ["any"], ["not", "opt"]], { any: l => l.map(condData), not: condData });
}
function effectsData(e) {
  return struct(e, [["stat_bonus"], ["stat_target"], ["cards", "opt"], ["free"], ["free_cards"],
    ["require_cards"], ["a_positions", "opt"], ["max_a_positions", "opt"], ["form", "opt"],
    ["injury_resistance", "opt"], ["weak_foot", "opt"], ["height_allowance", "opt"], ["forbid", "opt"],
    ["choice"], ["stat_warning", "opt"]], { choice: l => l.map(effectsData) });
}
function rulesetData(r) {
  return {
    schema: r.schema, name: r.name, squad_size: r.squad_size,
    stats: struct(r.stats, [["rating"], ["at_most"]]),
    tiers: r.tiers.map(t => struct(t, [["id", "req"], ["label", "req"], ["count", "opt"],
      ["counts_as", "opt"], ["when"], ["rate", "req"], ["stats"], ["form"], ["injury_resistance"],
      ["weak_foot"], ["cards"], ["min_cards"], ["free", "req"], ["a_positions"], ["name_color", "opt"],
      ["stat_warning"], ["total_within_targets"]],
      { when: condData })),
    card_groups: r.card_groups,
    medal_systems: (r.medal_systems || []).map(m => struct(m, [["label", "req"], ["counts", "req"]])),
    heights: { brackets: r.heights.brackets.map(b => struct(b, [["id", "req"], ["label", "req"],
      ["up_to", "opt"], ["exactly", "opt"]])),
      systems: r.heights.systems.map(s => struct(s, [["id", "req"], ["label", "req"],
        ["any_player", "opt"], ["by_quotas"], ["quotas", "req"], ["extra_height", "opt"]],
        { any_player: c => isEmpty(condData(c)) ? null : condData(c),
          extra_height: e => e.total == null || e.max == null ? null : e })) },
    universal: struct(r.universal, [["registered_position_a"], ["fielded_at_a"], ["no_b_ratings"],
      ["b_uses_a_allowance"], ["gk_not_second_a"], ["age", "opt"], ["weight", "opt"],
      ["captain_required"], ["min_goalkeepers"], ["max_skill_cards", "req"], ["max_com_styles", "opt"]]),
    specials: r.specials.map(s => struct(Object.assign({}, s, { severity: s.severity === "error" ? null : s.severity }),
      [["label", "req"], ["severity", "opt"], ["when"], ["then"]], { when: condData, then: effectsData })),
    team_settings: r.team_settings.map(t => struct(t, [["label", "req"], ["versions"],
      ["man_marking", "opt"], ["auto_substitution", "opt"], ["auto_offside_trap", "opt"],
      ["auto_preset_change", "opt"], ["auto_attack_defence_levels", "opt"], ["forbidden_shapes"]])),
    suggestions: r.suggestions,
  };
}
// Wraps one item's fields over lines of at most `width` characters.
function wrapItem(obj, indent, width) {
  const parts = Object.keys(obj).map(k => k + ": " + rv(obj[k]));
  const lines = [];
  let line = indent + "#{ ";
  parts.forEach((p, i) => {
    const piece = p + (i < parts.length - 1 ? "," : " },");
    if (line.trim() !== "#{" && line.length + piece.length + 1 > width) { lines.push(line.trimEnd()); line = indent + "   "; }
    line += piece + " ";
  });
  if (!parts.length) line = indent + "#{},";
  lines.push(line.trimEnd());
  return lines;
}
function dataBlock(r) {
  const d = rulesetData(r);
  const L = ["// AATF ruleset, a Rhai script. Open it in the Ruleset editor to change it.", "const RULESET = #{"];
  const I = "    ", W = 76;
  // `head` then the list's items, wrapped under it when the line would pass W.
  const listLines = (indent, head, items) => {
    const out = [];
    let line = indent + head + "[";
    items.forEach((v, i) => {
      const piece = rv(v) + (i < items.length - 1 ? "," : "],");
      if (i && line.length + piece.length + 1 > W) { out.push(line.trimEnd()); line = indent + I; }
      line += piece + " ";
    });
    if (!items.length) line += "],";
    out.push(line.trimEnd());
    return out;
  };
  L.push(`${I}schema: ${d.schema},`, `${I}name: ${rv(d.name)},`, `${I}squad_size: ${d.squad_size},`, "");
  L.push(`${I}stats: #{`);
  for (const k of Object.keys(d.stats)) L.push(...listLines(I + I, k + ": ", d.stats[k]));
  L.push(`${I}},`, "", `${I}tiers: [`);
  d.tiers.forEach(t => L.push(...wrapItem(t, I + I, W)));
  L.push(`${I}],`, "");
  if (!isEmpty(d.card_groups)) {
    L.push(`${I}card_groups: #{`);
    for (const [k, v] of Object.entries(d.card_groups)) L.push(...listLines(I + I, rk(k) + ": ", v));
    L.push(`${I}},`, "");
  }
  if (d.medal_systems.length) {
    L.push(`${I}medal_systems: [`);
    d.medal_systems.forEach(m => L.push(...wrapItem(m, I + I, W)));
    L.push(`${I}],`, "");
  }
  if (d.heights.brackets.length || d.heights.systems.length) {
    L.push(`${I}heights: #{`, `${I}${I}brackets: [`);
    d.heights.brackets.forEach(b => L.push(...wrapItem(b, I + I + I, W)));
    L.push(`${I}${I}],`, `${I}${I}systems: [`);
    d.heights.systems.forEach(s => L.push(...wrapItem(s, I + I + I, W)));
    L.push(`${I}${I}],`, `${I}},`, "");
  }
  L.push(`${I}universal: #{`);
  const u = Object.entries(d.universal).map(([k, v]) => `${k}: ${rv(v)}`);
  let line = I + I;
  u.forEach((p, i) => { if (line.trim() && line.length + p.length > W) { L.push(line.trimEnd()); line = I + I; } line += p + (i < u.length - 1 ? ", " : ","); });
  L.push(line.trimEnd(), `${I}},`, "");
  if (d.specials.length) {
    L.push(`${I}specials: [`);
    d.specials.forEach(s => L.push(...wrapItem(s, I + I, W)));
    L.push(`${I}],`, "");
  }
  if (d.team_settings.length) {
    L.push(`${I}team_settings: [`);
    d.team_settings.forEach(s => L.push(...wrapItem(s, I + I, W)));
    L.push(`${I}],`, "");
  }
  if (d.suggestions.length) L.push(...listLines(I, "suggestions: ", d.suggestions));
  L.push("};", "", "// ==== LOGIC (ruleset-logic 1): the Ruleset editor replaces this section ====",
    "// (the interpreter, a few hundred lines, not shown in this draft)");
  if (r.custom_checks) {
    L.push("", "// ==== CUSTOM CHECKS: optional, hand-written; the Ruleset editor keeps this section as it is ====");
    L.push(...r.custom_checks.split("\n"));
  }
  return L;
}

// ---- Validation: the load rules of aatf_rules.md "The ruleset schema" ----
function walkConds(c, fn, path) {
  if (!c) return;
  fn(c, path);
  (c.any || []).forEach((a, i) => walkConds(a, fn, `${path}.any[${i}]`));
  if (c.not) walkConds(c.not, fn, path + ".not");
}
// `any_player` may name heights and positions only (check.rs `condition_is_wide`).
function condIsWide(c) {
  return (c.tiers || []).length || (c.systems || []).length || (c.brackets || []).length ||
    c.captain != null || (c.playing_styles || []).length || (c.holds || []).length ||
    (c.rated_a_at || []).length || (c.rated_b_at || []).length || (c.name_colors || []).length ||
    c.fielded_only_at != null || Object.keys(c.ranges || {}).length || c.strong_foot != null ||
    (c.any || []).some(condIsWide) || (c.not != null && condIsWide(c.not));
}
// `height_allowance` in `then` or any `choice` option (check.rs `effects_have_allowance`).
function hasAllowance(e) {
  return e.height_allowance != null || (e.choice || []).some(hasAllowance);
}
function problems(r) {
  const out = [];
  const add = (sec, msg) => out.push({ section: sec, text: msg });
  const tierIds = r.tiers.map(t => t.id), brackets = r.heights.brackets.map(b => b.id),
    systems = r.heights.systems.map(s => s.id), groups = Object.keys(r.card_groups),
    suggestionWords = ["cards", "a_positions", "weak_foot", "injury_resistance", "height",
      "free_cards", "choices"],
    bytes8 = s => new TextEncoder().encode(s).length === 8;
  const dup = (ids, sec, kind) => ids.forEach((id, i) => { if (ids.indexOf(id) !== i) add(sec, `two ${kind} share the id "${id}"`); });
  if (r.name === "") add("general", "the ruleset has no name");
  r.stats.at_most.forEach(n => { if (!STATS.concat(AT_MOST_EXTRA).includes(n)) add("general", `"${n}" is no at-most target`); });
  dup(tierIds, "tiers", "tiers"); dup(brackets, "heights", "brackets"); dup(systems, "heights", "height systems");
  const checkCond = (c, sec, where, opts = {}) => walkConds(c, (k, path) => {
    if (!opts.noTiers) (k.tiers || []).forEach(t => { if (!tierIds.includes(t)) add(sec, `${where}${path}: no tier "${t}"`); });
    (k.systems || []).forEach(s => { if (!systems.includes(s)) add(sec, `${where}${path}: no height system "${s}"`); });
    (k.brackets || []).forEach(b => { if (!brackets.includes(b)) add(sec, `${where}${path}: no bracket "${b}"`); });
    (k.name_colors || []).forEach(n => { if (!bytes8(n)) add(sec, `${where}${path}: a name colour code is eight bytes`); });
    if (opts.noTiers && (k.tiers || []).length) add(sec, `${where}${path}: a tier's condition cannot name tiers`);
    if (opts.allowance && (k.brackets || []).length) add(sec, `${where}${path}: a special with a height allowance cannot name brackets, which are computed from it`);
  }, "");
  r.tiers.forEach((t, i) => {
    const where = `"${t.label || "tier " + (i + 1)}"`;
    if (t.id === "" || t.label === "") add("tiers", `${where}: an empty id or label`);
    if (t.counts_as) {
      if (t.count != null) add("tiers", `${where}: a tier that counts as another has no count of its own`);
      const target = r.tiers.find(x => x.id === t.counts_as);
      if (!target) add("tiers", `${where}: counts as an unknown tier`);
      else if (target.counts_as) add("tiers", `${where}: counts as a tier that itself counts as another`);
    }
    if (t.name_color != null && !bytes8(t.name_color)) add("tiers", `${where}: a name colour code is eight bytes`);
    Object.keys(t.free).forEach(g => { if (g !== "com" && !groups.includes(g)) add("tiers", `${where}: free cards of an unknown group "${g}"`); });
    checkCond(t.when, "tiers", where + " when", { noTiers: true });
  });
  const medalTiers = new Set((r.medal_systems || []).flatMap(s => Object.keys(s.counts)));
  const rest = r.tiers.filter(t => t.count == null && t.counts_as == null && !medalTiers.has(t.id));
  if (rest.length > 1) add("tiers", "only one tier can be the rest of the squad");
  const counted = r.tiers.reduce((n, t) => n + (t.count || 0), 0);
  if (counted > r.squad_size) add("tiers", `the tier counts add up to ${counted}, over the squad's ${r.squad_size}`);
  if (!rest.length && !medalTiers.size && counted < r.squad_size) add("tiers", `the tier counts add up to ${counted}, not the squad's ${r.squad_size}, and no tier is the rest`);
  (r.medal_systems || []).forEach((s, i) => {
    if (s.label === "") add("tiers", `medal system ${i + 1}: an empty label`);
    Object.keys(s.counts).forEach(id => { if (!tierIds.includes(id)) add("tiers", `medal system "${s.label}": counts an unknown tier "${id}"`); });
    const sc = counted + Object.values(s.counts).reduce((n, c) => n + c, 0);
    if (sc > r.squad_size) add("tiers", `medal system "${s.label}": its counts and the tier counts add up to ${sc}, over the squad's ${r.squad_size}`);
    else if (!rest.length && sc !== r.squad_size) add("tiers", `medal system "${s.label}": its counts and the tier counts add up to ${sc}, not the squad's ${r.squad_size}, and no tier is the rest`);
  });
  r.tiers.forEach((t, i) => {
    const where = `"${t.label || "tier " + (i + 1)}"`;
    if (t.count != null && medalTiers.has(t.id)) add("tiers", `${where}: a tier the medal systems count has no count of its own`);
    if (t.counts_as) {
      if (medalTiers.has(t.id)) add("tiers", `${where}: a tier the medal systems count does not count as another`);
      if (medalTiers.has(t.counts_as)) add("tiers", `${where}: counts as a tier the medal systems count`);
    }
  });
  let top = 0;
  r.heights.brackets.forEach(b => {
    const hs = b.up_to != null ? [b.up_to] : (b.exactly || []);
    if (!hs.length) add("heights", `bracket "${b.label}": no height`);
    if (Math.min(...hs) <= top) add("heights", `bracket "${b.label}": its heights must lie above the brackets before it`);
    top = Math.max(top, ...hs);
    if (b.id === "" || b.label === "") add("heights", "a bracket with an empty id or label");
  });
  r.heights.systems.forEach((s, i) => {
    const last = i === r.heights.systems.length - 1, has = !isEmpty(condData(s.any_player));
    if (s.id === "" || s.label === "") add("heights", "a system with an empty id or label");
    if (!last && !has && !(s.by_quotas && Object.keys(s.quotas).length)) add("heights", `system "${s.label}": every system but the last needs a "when any player is" condition or "when the team meets its quotas" with quotas`);
    if (last && (has || s.by_quotas)) add("heights", `system "${s.label}": the last system applies otherwise and has no condition`);
    if (condIsWide(s.any_player || {})) add("heights", `system "${s.label}": "when any player is" may name only heights and positions`);
    const pool = s.extra_height && s.extra_height.total != null && s.extra_height.max != null;
    Object.keys(s.quotas).forEach(bid => {
      const b = r.heights.brackets.find(x => x.id === bid);
      if (!b) add("heights", `system "${s.label}": a quota for an unknown bracket`);
      else if (pool && !(b.exactly && b.exactly.length === 1))
        add("heights", `system "${s.label}": a height pool's quotas name only brackets of exactly one height`);
    });
    if (s.extra_height && (s.extra_height.total == null || s.extra_height.max == null)) add("heights", `system "${s.label}": the extra height pool needs both its total and the tallest a player may be`);
    if (pool) {
      const quotas = Object.values(s.quotas).reduce((n, q) => n + q, 0);
      if (quotas !== r.squad_size) add("heights", `system "${s.label}": the pool's quotas add up to ${quotas}, not the squad's ${r.squad_size}`);
    }
  });
  Object.entries(r.card_groups).forEach(([gid, cards], gi) => {
    if (gid === "") add("cards", "a card group with an empty id");
    if (gid === "com" || suggestionWords.includes(gid))
      add("cards", `the group name "${gid}" is reserved: "com" is the built-in COM styles group, the others are suggestion words`);
    cards.forEach(card => {
      if (COM.includes(card)) add("cards", `COM style "${human(card)}" is in group "${gid}"; the built-in "com" group holds the seven`);
      Object.keys(r.card_groups).slice(0, gi).forEach(other => {
        if (r.card_groups[other].includes(card)) add("cards", `"${human(card)}" is in two card groups ("${other}", "${gid}")`);
      });
    });
  });
  const perTierIds = (v, sec, where) => { if (v && typeof v === "object") Object.keys(v).forEach(id => {
    if (!tierIds.includes(id)) add(sec, `${where}: a per-tier value names no tier "${id}"`); }); };
  const checkEffects = (e, sec, where) => {
    Object.values(e.stat_bonus || {}).concat(Object.values(e.stat_target || {}),
      [e.cards, e.a_positions, e.form, e.injury_resistance, e.stat_warning]).forEach(v => perTierIds(v, sec, where));
    Object.entries(e.free || {}).forEach(([g, v]) => {
      if (g !== "com" && !groups.includes(g)) add(sec, `${where}: free cards of an unknown group "${g}"`);
      perTierIds(v, sec, where);
    });
    if (e.forbid === "") add(sec, `${where}: a forbid with no text`);
    (e.choice || []).forEach(o => checkEffects(o, sec, where));
  };
  r.specials.forEach((s, i) => {
    const where = `special "${s.label || i + 1}"`;
    if (s.label === "") add("specials", `special ${i + 1}: no label`);
    checkCond(s.when, "specials", where + " when", { allowance: hasAllowance(s.then) });
    (s.then.choice || []).forEach((o, j) => Object.keys(o).forEach(k => {
      if (!CHOICE_EFFECTS.includes(k)) add("specials", `${where}: option ${j + 1} holds "${effectName(k)}", which an option cannot (it could never be found used)`);
    }));
    checkEffects(s.then, "specials", where);
  });
  r.team_settings.forEach((t, i) => {
    if (t.label === "") add("universal", `team settings ${i + 1}: no label`);
    (t.forbidden_shapes || []).forEach(sh => {
      if (sh.reduce((a, b) => a + b, 0) !== 10) add("universal", `team settings "${t.label}": the shape ${sh.join("-")} does not add up to 10 outfield players`);
    });
  });
  r.universal.fielded_at_a.forEach(p => { if (p < 1 || p > 3) add("universal", "presets are 1 to 3"); });
  r.suggestions.forEach(sg => {
    if (!suggestionWords.includes(sg) && sg !== "com" && !groups.includes(sg))
      add("suggestions", `"${sg}" is no allowance or card group`);
  });
  return out;
}

// ---- Readable rules (the Summary section) ----
const list = (xs, f = human) => xs.length < 2 ? xs.map(f).join("") : xs.slice(0, -1).map(f).join(", ") + " or " + f(xs[xs.length - 1]);
function tierLabel(id) { const t = R.tiers.find(x => x.id === id); return t ? t.label : id; }
function condText(c) {
  if (!c) return "";
  const p = [];
  if ((c.tiers || []).length) p.push(list(c.tiers, tierLabel));
  if ((c.positions || []).length) p.push("registered " + list(c.positions, x => x));
  if ((c.systems || []).length) p.push("under " + list(c.systems, id => (R.heights.systems.find(s => s.id === id) || { label: id }).label));
  if ((c.brackets || []).length) p.push(list(c.brackets, id => (R.heights.brackets.find(b => b.id === id) || { label: id }).label.toLowerCase()) + "s");
  if (c.height_min != null && c.height_max != null) p.push(c.height_min === c.height_max ? `${c.height_min} cm` : `${c.height_min} to ${c.height_max} cm`);
  else if (c.height_min != null) p.push(`${c.height_min} cm or more`);
  else if (c.height_max != null) p.push(`${c.height_max} cm or less`);
  if (c.captain === true) p.push("the captain");
  if (c.captain === false) p.push("not the captain");
  if ((c.playing_styles || []).length) p.push("playing as " + list(c.playing_styles));
  if ((c.holds || []).length) p.push("holding " + list(c.holds));
  if ((c.rated_a_at || []).length) p.push("rated A at " + list(c.rated_a_at, x => x));
  if ((c.rated_b_at || []).length) p.push("rated B at " + list(c.rated_b_at, x => x));
  if ((c.name_colors || []).length) p.push("named in colour " + list(c.name_colors, x => x));
  if (c.fielded_only_at) p.push("fielded only at " + c.fielded_only_at);
  for (const [k, [lo, hi]] of Object.entries(c.ranges || {})) p.push(`${human(k).toLowerCase()} ${lo} to ${hi}`);
  if (c.strong_foot) p.push(c.strong_foot + "-footed");
  if ((c.any || []).length) p.push("any of (" + c.any.map(a => condText(a) || "anyone").join("; ") + ")");
  if (c.not) p.push("except " + (condText(c.not) || "anyone"));
  return p.join(", ");
}
// A per-tier map's entries in the tiers' order, highest first, whatever order the file has.
const byTier = v => Object.entries(v).sort(([a], [b]) => R.tiers.findIndex(t => t.id === a) - R.tiers.findIndex(t => t.id === b));
function perTierText(v, unit) {
  if (typeof v === "number") return (v > 0 ? "+" : "") + v + unit;
  return byTier(v).map(([t, n]) => `${(n > 0 ? "+" : "") + n}${unit} for ${tierLabel(t)}`).join(", ");
}
function effectsText(e) {
  const p = [];
  for (const [k, v] of Object.entries(e.stat_bonus || {})) p.push(`${perTierText(v, "")} to ${k === "all" ? "every stat" : human(k).toLowerCase()}`);
  for (const [k, v] of Object.entries(e.stat_target || {})) p.push(`${human(k).toLowerCase()} target ${typeof v === "number" ? v
    : byTier(v).map(([t, n]) => `${n} for ${tierLabel(t)}`).join(", ")}`);
  if (e.cards != null) p.push(`${perTierText(e.cards, "")} paid card${e.cards === 1 ? "" : "s"}`);
  for (const [k, v] of Object.entries(e.free || {})) p.push(`${perTierText(v, "")} free ${k === "com" ? "COM style" : human(k).toLowerCase()}`);
  if ((e.free_cards || []).length) p.push(list(e.free_cards) + " free");
  if ((e.require_cards || []).length) p.push("must hold " + list(e.require_cards));
  if (e.a_positions != null) p.push(`${perTierText(e.a_positions, "")} A position`);
  if (e.max_a_positions != null) p.push(`at most ${e.max_a_positions} A position${e.max_a_positions === 1 ? "" : "s"}`);
  if (e.form != null) p.push(`${perTierText(e.form, "")} form`);
  if (e.injury_resistance != null) p.push(`${perTierText(e.injury_resistance, "")} injury resistance`);
  if (e.weak_foot) p.push(`weak foot up to ${e.weak_foot.usage}/${e.weak_foot.accuracy}`);
  if (e.height_allowance != null) p.push(`count ${e.height_allowance} cm shorter`);
  if (e.stat_warning != null) p.push("warning band " + (typeof e.stat_warning === "number" ? e.stat_warning
    : byTier(e.stat_warning).map(([t, n]) => `${n} for ${tierLabel(t)}`).join(", ")));
  if (e.forbid != null) p.push(`not allowed ("${e.forbid}")`);
  if ((e.choice || []).length === 1) p.push("may get " + effectsText(e.choice[0]));
  else if ((e.choice || []).length) p.push("gets the first needed of: " + e.choice.map(effectsText).join("; or "));
  return p.join(", ");
}
function summary(r) {
  const L = [`# ${r.name}`, "", `Squad of ${r.squad_size}. A player's rating is his best of: ${r.stats.rating.map(s => human(s).toLowerCase()).join(", ")}.`];
  if (r.stats.at_most.length) L.push(`May sit below their target: ${r.stats.at_most.map(s => human(s).toLowerCase()).join(", ")}.`);
  L.push("", "## Tiers");
  const medalTiers = new Set((r.medal_systems || []).flatMap(s => Object.keys(s.counts)));
  r.tiers.forEach(t => {
    const count = medalTiers.has(t.id) ? "by medal system" : t.counts_as ? `counts as ${tierLabel(t.counts_as)}` : t.count === null ? "the rest of the squad" : `${t.count} per team`;
    const when = condText(t.when);
    const free = Object.entries(t.free).map(([g, n]) => `${n} free ${g === "com" ? "COM style" + (n > 1 ? "s" : "") : human(g).toLowerCase()}`);
    const stats = Object.entries(t.stats).map(([s, v]) => `${human(s).toLowerCase()} ${v}`);
    const band = t.stat_warning ? `; a stat up to ${t.stat_warning} over its target warns, past that it is not allowed` : "";
    const total = t.total_within_targets ? "; his stats added up may not exceed the targets' sum" : "";
    L.push(`- **${t.label}**: ${count}${when ? ", " + when : ""}; every stat ${t.rate}${stats.length ? " (" + stats.join(", ") + ")" : ""}${band}${total}; form ${t.form}, injury resistance ${t.injury_resistance}, weak foot ${t.weak_foot.usage}/${t.weak_foot.accuracy}; ${t.cards} paid cards${t.min_cards ? ` (at least ${t.min_cards})` : ""}${free.length ? ", " + free.join(", ") : ""}; ${t.a_positions} A position${t.a_positions === 1 ? "" : "s"}${t.name_color ? `; name in colour ${t.name_color}` : ""}.`);
  });
  if ((r.medal_systems || []).length) {
    const named = r.tiers.filter(t => medalTiers.has(t.id));
    L.push("Medal systems: " + list(r.medal_systems.map(s => `${s.label} (${named.map(t => `${s.counts[t.id] || 0} ${t.label}`).join(", ")})`), x => x) + ".");
  }
  if (r.heights.brackets.length) {
    L.push("", "## Heights");
    let prev = 0;
    L.push("Brackets: " + r.heights.brackets.map(b => { const s = b.up_to != null ? (prev ? `${b.label} ${prev + 1}-${b.up_to}` : `${b.label} up to ${b.up_to}`) : b.exactly.length ? `${b.label} ${b.exactly.join(" or ")}` : b.label; prev = b.up_to ?? (b.exactly.length ? Math.max(...b.exactly) : prev); return s; }).join(", ") + " cm.");
    r.heights.systems.forEach((s, i) => {
      const when = condText(s.any_player), parts = [];
      if (when) parts.push("any player is " + when);
      if (s.by_quotas) parts.push("the team meets its quotas");
      const eh = s.extra_height;
      L.push(`- **${s.label}** ${parts.length ? "(when " + parts.join(" and ") + ")" : i ? "(otherwise)" : ""}: ` +
        r.heights.brackets.filter(b => b.id in s.quotas).map(b => `${s.quotas[b.id]} ${b.label.toLowerCase()}`).join(", ") +
        (eh && eh.total != null && eh.max != null ? `; ${eh.total} cm to share, no player above ${eh.max}` : "") + ".");
    });
  }
  const u = r.universal, U = [];
  if (u.registered_position_a) U.push("every player is rated A at his registered position");
  if (u.fielded_at_a.length) U.push(`starters play only where rated A in preset ${u.fielded_at_a.join(", ")}`);
  if (u.no_b_ratings) U.push("no B ratings");
  if (u.b_uses_a_allowance) U.push("B ratings count as A positions");
  if (u.gk_not_second_a) U.push("GK is never a second A");
  if (u.age) U.push(`age ${u.age[0]} to ${u.age[1]}`);
  if (u.weight) U.push(`weight between height − ${u.weight.below} (at least ${u.weight.floor}) and height − ${u.weight.above}`);
  if (u.captain_required) U.push("a captain");
  if (u.min_goalkeepers) U.push(`at least ${u.min_goalkeepers} goalkeeper${u.min_goalkeepers > 1 ? "s" : ""}`);
  U.push(`at most ${u.max_skill_cards} skill cards`);
  if (u.max_com_styles != null) U.push(`at most ${u.max_com_styles} COM styles`);
  const every = U.join("; ");
  L.push("", "## Every team", every[0].toUpperCase() + every.slice(1) + ".");
  const groups = Object.entries(r.card_groups);
  if (groups.length) {
    L.push("", "## Card groups");
    groups.forEach(([g, cards]) => L.push(`- **${human(g)}**: ${cards.map(c => human(c).toLowerCase()).join(", ")}.`));
  }
  if (r.team_settings.length) {
    L.push("", "## Team settings");
    const SETTING = { man_marking: "man marking", auto_substitution: "automatic substitution",
      auto_offside_trap: "automatic offside trap", auto_preset_change: "automatic preset change",
      auto_attack_defence_levels: "automatic attack/defence levels" };
    r.team_settings.forEach(t => {
      const parts = Object.keys(SETTING).filter(k => t[k] != null).map(k => `${SETTING[k]} ${t[k] ? "on" : "off"}`);
      if (t.forbidden_shapes.length) parts.push("no " + t.forbidden_shapes.map(s => s.join("-")).join(" or ") + " formations");
      L.push(`- **${t.label}**${t.versions.length ? ` (PES ${t.versions.join(", ")})` : ""}: ${parts.join(", ")}.`);
    });
  }
  if (r.specials.length) {
    L.push("", "## Special rules");
    r.specials.forEach(s => { const w = condText(s.when); L.push(`- **${s.label}**${s.severity !== "error" ? ` (${s.severity})` : ""}: ${w ? w.charAt(0).toUpperCase() + w.slice(1) + ": " : ""}${effectsText(s.then)}.`); });
  }
  return L.join("\n");
}

// ---- Layout ----
// The top bar's "Show preview" toggle; off at every start.
let showPreview = false;
function drawView(center) {
  if (!R) return;
  closeMenus();
  const probs = problems(R);
  center.classList.add("bleed");
  // The page's own <body> skeleton, built into the host's central panel on the
  // first draw and after another tool held it.
  if (!document.getElementById("rs-content")) {
    center.innerHTML = "";
    center.append(
      h("div", { class: "top", id: "top", "data-panel": "top-bar" }),
      h("div", { class: "main" },
        h("div", { class: "rail", id: "rail", "data-panel": "rail" }),
        h("div", { class: "center", id: "rs-content", "data-panel": "content" }),
        h("div", { class: "data hidden", id: "data", "data-panel": "data-preview" },
          h("div", { class: "row", style: "padding:4px 8px; margin:0" },
            h("span", { class: "strong" }, "Rules file: data block"),
            h("span", { class: "weak", title: "The editor writes this block; you can also edit it by hand. Changed lines are highlighted." }, "(read-only)")),
          h("pre", { id: "block" }))),
      h("div", { class: "bottom", id: "rs-bottom", "data-panel": "bottom-bar" }));
  }
  const top = document.getElementById("top");
  top.innerHTML = "";
  top.append(
    menu("File", [["New from template…", startPage], ["Open…", () => {}, true, "Not in this draft"],
      ["Save", save], ["Save as…", save], ["-"],
      ["Open the sample with a custom check", () => load(TEMPLATES["custom-check"](), "custom-check")]], { frame0: true }),
    menu("Edit", [["Undo  (Ctrl+Z)", doUndo, !undo.length], ["Redo  (Ctrl+Y)", doRedo, !redo.length]], { frame0: true }),
    menu("Randomize", [["Sensible", () => {}, true, "The generator is not faked in this draft"],
      ["Weird", () => {}, true, "The generator is not faked in this draft"],
      ["Crazy", () => {}, true, "The generator is not faked in this draft"]], { frame0: true }),
    h("span", { class: "sep", style: "border-top:0; border-left:1px solid var(--stroke); height:14px; margin:0 4px" }),
    strong(R.name + (JSON.stringify(R) !== saved ? " •" : "")),
    h("span", { style: "flex:1" }),
    weak("Draft: sample data, nothing is written"),
    // A toggle_value: the data block pane right of the form, off at every start
    // (plans/ruleset_editor.md "The editor").
    h("span", { class: "sel" + (showPreview ? " on" : ""), title: "Show the rules file's data block as the editor writes it",
      onclick: () => { showPreview = !showPreview; render(); } }, "Show preview"));
  document.getElementById("data").classList.toggle("hidden", !showPreview);
  const rail = document.getElementById("rail");
  rail.innerHTML = "";
  for (const [id, label] of SECTIONS) {
    if (id === "custom" && !R.custom_checks) continue;
    const bad = probs.some(p => p.section === id);
    rail.append(h("span", { class: "sel" + (section === id ? " on" : ""), onclick: () => { section = id; render(); } },
      label, bad ? h("span", { class: "err", title: "This section has a problem" }, "⚠") : ""));
  }
  if (section === "custom" && !R.custom_checks) section = "general";
  const content = document.getElementById("rs-content");
  const scroll = content.scrollTop;
  content.innerHTML = "";
  content.append(VIEWS[section]());
  content.scrollTop = scroll;
  refresh();
}
function refresh() {
  const probs = problems(R);
  const lines = dataBlock(R);
  const pre = document.getElementById("block");
  pre.innerHTML = "";
  const before = new Set(previousLines);
  lines.forEach(l => pre.append(h("div", { class: previousLines.length && !before.has(l) ? "changed" : "" }, l)));
  previousLines = lines;
  const bottom = document.getElementById("rs-bottom");
  bottom.innerHTML = "";
  bottom.append(probs.length
    ? h("span", { class: "err", title: probs.map(p => p.text).join("\n") }, `✖ ${probs.length} problem${probs.length > 1 ? "s" : ""}: ${probs[0].text}${probs.length > 1 ? " (hover for all)" : ""}`)
    : h("span", { style: "color:#7fbf7f" }, "✔ Valid: the Save editor and the Team creator can use it"),
    h("span", { style: "flex:1" }), button("Save", save));
  document.querySelectorAll("#rail .sel").forEach((el, i) => {
    const id = SECTIONS.filter(([s]) => s !== "custom" || R.custom_checks)[i][0];
    const bad = probs.some(p => p.section === id);
    const mark = el.querySelector(".err");
    if (bad && !mark) el.append(h("span", { class: "err" }, "⚠"));
    if (!bad && mark) mark.remove();
  });
  if (section === "summary") { const s = document.getElementById("summary-text"); if (s) s.textContent = summary(R); }
}
function save() {
  const probs = problems(R);
  if (probs.length) {
    window_("Not saved: the ruleset has problems", h("div", { class: "col" }, probs.map(p => h("span", { class: "err" }, "✖ " + p.text))),
      [button("OK", closeWindow)], "not-saved");
  } else {
    saved = JSON.stringify(R);
    window_("Saved", h("div", {}, `In the app this writes ${R.name.toLowerCase().split(/[^\p{L}\p{N}]+/u).filter(Boolean).join("_")}.4ccrules: the data block, the current logic, and the custom checks unchanged. This draft writes nothing.`), [button("OK", () => { closeWindow(); render(); })], "saved");
  }
  render(); // the link carries the open window's name
}
function startPage() {
  // The `start` window's name is the link's mark for the start page open.
  window_("Ruleset editor - Unlimited Tool Works", h("div", { class: "col" },
    strong("New from template"),
    button("4cc Autumn 26 (the official rules)", () => loadTemplate("autumn26")),
    button("4cc Spring 26", () => loadTemplate("spring26")),
    button("/vg/ League 27", () => loadTemplate("vgl27")),
    button("/vg/ League 26", () => loadTemplate("vgl26")),
    button("/vt/ League 11", () => loadTemplate("vtl11")),
    button("/vt/ League 10", () => loadTemplate("vtl10")),
    button("Divegrass Before Christmas 7", () => loadTemplate("dbc7")),
    button("Clouds' proposal, every item (from VGL26)", () => loadTemplate("clouds")),
    button("Blank: one tier, no brackets, the universal rules on", () => loadTemplate("blank")),
    h("div", { class: "sep" }),
    strong("Random"),
    row(["Sensible", "Weird", "Crazy"].map(p => button(p, () => {}, { disabled: true, title: "The generator is not faked in this draft" }))),
    h("div", { class: "sep" }),
    row(button("Open file…", () => {}, { disabled: true, title: "Not in this draft" }), weak("Recent: none"))),
    R ? [button("Close", closeWindow)] : [], "start");
  render(); // the link carries the start page's mark off its keys
}

// ---- Sections ----
const VIEWS = {
  general() {
    const d = h("div", {}, h("h2", {}, "General"));
    d.append(named(h("table", { class: "grid" },
      h("tr", {}, h("td", {}, "Name"), h("td", {}, text(R.name, v => R.name = v, { width: 220 }))),
      h("tr", {}, h("td", {}, "Squad size"), h("td", {}, drag(R.squad_size, v => R.squad_size = v ?? 0, { min: 1, max: 32 })))), "general-fields"));
    d.append(h("div", { class: "sep" }), strong("Rating stats"),
      h("div", { class: "weak" }, "A player's rating is the highest of these; it picks his tier. What a stat may sit at is the target, the warning band and the stat total, not the rating."),
      named(statGrid(STATS, R.stats.rating, v => R.stats.rating = v), "rating-stats"));
    d.append(h("div", { class: "sep" }), strong("May sit below the target"),
      h("div", { class: "weak" }, "Every other stat must equal its target exactly."),
      named(statGrid(STATS.concat(AT_MOST_EXTRA), R.stats.at_most, v => R.stats.at_most = v), "at-most-stats"));
    return d;
  },
  tiers() {
    const d = h("div", {}, h("h2", {}, "Tiers"), h("div", { class: "weak" },
      "Highest first: a player gets the first tier whose conditions he meets and whose rating he reaches. The Save editor's tier buttons follow this order."));
    const ids = R.tiers.map(t => [t.id, t.label]);
    const medalTiers = new Set(R.medal_systems.flatMap(s => Object.keys(s.counts)));
    const g = named(h("table", { class: "grid" }, h("tr", {}, ["", "Label", "Id", "Count", "Counts as", "Rate", "Form", "Injury res.", "Weak foot", "Paid cards", "Min. cards", "A positions", "Name colour", "Warn band", "Stats ≤ targets", ""].map(x => h("th", {}, x)))), "tiers-table");
    R.tiers.forEach((t, i) => {
      g.append(reorderable(h("tr", {},
        h("td", {}, handle(i)),
        h("td", {}, text(t.label, v => t.label = v, { width: 100 })),
        h("td", {}, text(t.id, v => {
          R.medal_systems.forEach(s => { if (t.id in s.counts && !(v in s.counts)) { s.counts[v] = s.counts[t.id]; delete s.counts[t.id]; } });
          t.id = v;
        }, { width: 80, mono: true, title: "What conditions and other tiers refer to" })),
        h("td", {}, medalTiers.has(t.id) ? weak("by medal system") : t.counts_as ? weak("—") : h("span", { class: "row", style: "margin:0" },
          t.count === null ? weak("the rest") : drag(t.count, v => t.count = v ?? 0, { max: 32 }),
          checkbox("rest", t.count === null, v => t.count = v ? null : 1, "The rest of the squad: one tier at most"))),
        h("td", {}, combo([[null, "—"]].concat(ids.filter(([id]) => id !== t.id)), t.counts_as, v => { t.counts_as = v; if (v) t.count = null; },
          { title: "The tier whose count this tier's players fill" })),
        h("td", {}, drag(t.rate, v => t.rate = v ?? 0, { max: 99 })),
        h("td", {}, drag(t.form, v => t.form = v ?? 0, { max: 8 })),
        h("td", {}, drag(t.injury_resistance, v => t.injury_resistance = v ?? 0, { max: 3 })),
        h("td", {}, h("span", { class: "row", style: "margin:0" }, drag(t.weak_foot.usage, v => t.weak_foot.usage = v ?? 0, { max: 4, width: 38, title: "Usage" }),
          drag(t.weak_foot.accuracy, v => t.weak_foot.accuracy = v ?? 0, { max: 4, width: 38, title: "Accuracy" }))),
        h("td", {}, drag(t.cards, v => t.cards = v ?? 0, { max: 20 })),
        h("td", {}, drag(t.min_cards, v => t.min_cards = v ?? 0, { max: 20 })),
        h("td", {}, drag(t.a_positions, v => t.a_positions = v ?? 0, { max: 13 })),
        h("td", {}, h("span", { class: "row", style: "margin:0" }, checkbox("", t.name_color !== null, v => t.name_color = v ? "cc9900ff" : null, "This tier's names carry a colour code"),
          t.name_color !== null ? colour(t.name_color, v => t.name_color = v) : weak("none"))),
        h("td", {}, drag(t.stat_warning, v => t.stat_warning = v ?? 0, { max: 99, width: 40,
          title: "Points above a stat's target that warn rather than err (0: every point errs)" })),
        h("td", {}, checkbox("", t.total_within_targets, v => t.total_within_targets = v,
          "The stats add up to at most the targets, an error past them")),
        h("td", {}, R.tiers.length > 1 ? button("✕", () => { R.medal_systems.forEach(s => { delete s.counts[t.id]; }); R.tiers.splice(i, 1); structural(); }, { frame0: true, title: "Remove" }) : "")), R.tiers, i));
      g.append(h("tr", {}, h("td", {}), h("td", { colspan: 15 }, collapsing(`tier:${i}`, "Stat targets, free cards and conditions", () => h("div", {},
        strong("Per-stat targets"), h("span", { class: "weak" }, " (empty: the rate)"),
        perStatGrid(t),
        h("div", { class: "sep" }), strong("Free cards"),
        row(["com"].concat(Object.keys(R.card_groups)).map(gid => h("span", { class: "row", style: "margin:0 8px 0 0" },
          gid === "com" ? "COM styles" : human(gid), drag(t.free[gid] ?? null, v => { if (v === null) delete t.free[gid]; else t.free[gid] = v; }, { max: 20, placeholder: "0" })))),
        h("div", { class: "sep" }), strong("Conditions"), h("span", { class: "weak" }, " (what a player must also meet to be this tier)"),
        condEditor(t.when, `tier:${i}:when`, { noTiers: true }))))));
    });
    d.append(g, row(button("+ Add tier", () => { const n = R.tiers.length + 1; R.tiers.push(tier("tier" + n, "Tier " + n, 1, 80, 4, 1, 2, 3, {})); structural(); })));
    d.append(h("div", { class: "sep" }), strong("Medal systems"), h("div", { class: "weak" },
      "Alternative medal counts: the team's counts of the ticked tiers must equal one system's. Unticked tiers keep their own count, one of them the rest of the squad."));
    const ms = R.medal_systems;
    const mg = named(h("table", { class: "grid" }, h("tr", {}, h("th", {}), R.tiers.map(t => h("th", {},
      h("label", { class: "cb", title: ms.length ? "The medal systems count this tier" : "Add a medal system first" },
        h("input", { type: "checkbox", checked: medalTiers.has(t.id), disabled: !ms.length, onchange: e => {
          if (e.target.checked) { ms.forEach(s => { if (!(t.id in s.counts)) s.counts[t.id] = 0; }); t.count = null; }
          else ms.forEach(s => { delete s.counts[t.id]; });
          structural();
        } }), t.label))))), "medal-systems");
    ms.forEach((s, i) => mg.append(h("tr", {},
      h("td", {}, h("span", { class: "row", style: "margin:0" }, text(s.label, v => s.label = v, { width: 90 }), remove(ms, i))),
      R.tiers.map(t => h("td", {}, medalTiers.has(t.id)
        ? drag(s.counts[t.id] ?? null, v => { if (v === null) delete s.counts[t.id]; else s.counts[t.id] = v; }, { max: 32, width: 40, placeholder: "0" })
        : weak("—"))))));
    d.append(mg, row(button("+ Add medal system", () => { ms.push({ label: "System " + (ms.length + 1), counts: {} }); structural(); })));
    return d;
  },
  heights() {
    const d = h("div", {}, h("h2", {}, "Heights"));
    const on = R.heights.brackets.length > 0 || R.heights.systems.length > 0 || R._bracketsOn;
    d.append(row(checkbox("Use height brackets", on, v => {
      if (v) { R._bracketsOn = true; R.heights = R._savedHeights || { brackets: [{ id: "short", label: "Short", up_to: 175 }], systems: [{ id: "default", label: "Default", any_player: null, quotas: {} }] }; }
      else { R._savedHeights = clone(R.heights); R._bracketsOn = false; R.heights = { brackets: [], systems: [] }; }
    })));
    if (!on) { d.append(h("div", { class: "weak" }, "Off: heights are left to tier conditions and special rules.")); return d; }
    d.append(strong("Brackets"), h("div", { class: "weak" }, "Ascending: a player's bracket is the first that takes his height (less any height allowance)."));
    const g = named(h("table", { class: "grid" }, h("tr", {}, ["", "Label", "Id", "Heights", "", ""].map(x => h("th", {}, x)))), "brackets");
    R.heights.brackets.forEach((b, i) => g.append(reorderable(h("tr", {},
      h("td", {}, handle(i)),
      h("td", {}, text(b.label, v => b.label = v, { width: 90 })),
      h("td", {}, text(b.id, v => b.id = v, { width: 70, mono: true })),
      h("td", {}, combo([["up_to", "up to"], ["exactly", "exactly"]], b.up_to != null ? "up_to" : "exactly", v => {
        if (v === "up_to") { b.up_to = (b.exactly || [180])[0]; delete b.exactly; } else { b.exactly = [b.up_to]; delete b.up_to; } })),
      h("td", {}, b.up_to != null ? h("span", {}, drag(b.up_to, v => b.up_to = v ?? 0, { max: 210 }), " cm")
        : h("span", {}, text(b.exactly.join(", "), v => b.exactly = v.split(",").map(x => Number(x.trim())).filter(x => x > 0), { width: 90, title: "Heights, comma-separated" }), " cm")),
      h("td", {}, remove(R.heights.brackets, i))), R.heights.brackets, i)));
    d.append(g, row(button("+ Add bracket", () => { const top = R.heights.brackets.reduce((m, b) => Math.max(m, b.up_to ?? Math.max(...b.exactly)), 170); R.heights.brackets.push({ id: "b" + (top + 5), label: "Up to " + (top + 5), up_to: top + 5 }); structural(); })));
    d.append(h("div", { class: "sep" }), strong("Height systems"), h("div", { class: "weak" }, "The first whose condition some player meets sets the quotas; the last applies otherwise."));
    const systems = h("div", { "data-panel": "height-systems" });
    R.heights.systems.forEach((s, i) => {
      const last = i === R.heights.systems.length - 1;
      const box = reorderable(h("div", { class: "group" },
        row(handle(i), text(s.label, v => s.label = v, { width: 100 }), text(s.id, v => s.id = v, { width: 70, mono: true }), h("span", { style: "flex:1" }), remove(R.heights.systems, i)),
        last && isEmpty(condData(s.any_player)) ? row(weak("Otherwise (no condition): the last system takes every team the others do not"))
          : h("div", {}, weak("When any player is:"), condEditor(s.any_player || (s.any_player = {}), `sys:${i}`, { heightsPositions: true })),
        row(checkbox("when the team meets its quotas", !!s.by_quotas, v => s.by_quotas = v,
          "The system holds only when the team, read under it, meets the quotas below")),
        (() => { const eh = k => drag(s.extra_height ? s.extra_height[k] : null, v => {
            const e = s.extra_height || (s.extra_height = { total: null, max: null });
            e[k] = v;
            if (e.total == null && e.max == null) s.extra_height = null;
          }, { max: 255, width: 44, placeholder: "-", title: k === "total" ? "Centimetres the team may add above the bracket heights" : "The tallest a player may be" });
          return row(weak("Extra height pool:"), eh("total"), "cm to share, no player above", eh("max"), "cm", weak("(both empty: none)")); })(),
        h("table", { class: "grid" }, h("tr", {}, h("th", {}, "Quota"), R.heights.brackets.map(b => h("th", {}, b.label))),
          h("tr", {}, h("td", {}, weak("per team")), R.heights.brackets.map(b => h("td", {}, h("span", { class: "row", style: "margin:0" },
            checkbox("", b.id in s.quotas, v => { if (v) s.quotas[b.id] = 0; else delete s.quotas[b.id]; }, "Constrained (unticked: any number)"),
            b.id in s.quotas ? drag(s.quotas[b.id], v => s.quotas[b.id] = v ?? 0, { max: 23, width: 40 }) : weak("any"))))))),
        R.heights.systems, i);
      systems.append(box);
    });
    d.append(systems);
    d.append(row(button("+ Add system", () => { R.heights.systems.push({ id: "system" + (R.heights.systems.length + 1), label: "System " + (R.heights.systems.length + 1), any_player: null, quotas: {} }); structural(); })));
    return d;
  },
  cards() {
    const d = h("div", {}, h("h2", {}, "Cards"), h("div", { class: "weak" },
      "Named card sets a tier or a special rule can make free. Every COM style is also in the built-in group \"COM styles\"."));
    const groups = h("div", { "data-panel": "card-groups" });
    Object.keys(R.card_groups).forEach(gid => {
      groups.append(h("div", { class: "group" }, row(strong("Group"), text(gid, v => {
        if (!v || v === gid || v in R.card_groups) return;
        const cards = R.card_groups[gid];
        const groups = {};
        for (const [k, c] of Object.entries(R.card_groups)) groups[k === gid ? v : k] = c;
        R.card_groups = groups;
        R.tiers.forEach(t => { if (gid in t.free) { t.free[v] = t.free[gid]; delete t.free[gid]; } });
        R.suggestions = R.suggestions.map(s => s === gid ? v : s);
        gid = v;
      }, { mono: true, width: 120, title: "The group's id; renaming it updates the tiers and suggestions" }),
        h("span", { style: "flex:1" }), button("✕", () => { delete R.card_groups[gid]; structural(); }, { frame0: true })),
        cardGrid(R.card_groups[gid], v => R.card_groups[gid] = v)));
    });
    d.append(groups);
    d.append(row(button("+ Add group", () => { let n = 1; while ("group" + n in R.card_groups) n++; R.card_groups["group" + n] = []; structural(); })));
    return d;
  },
  universal() {
    const u = R.universal;
    const d = h("div", {}, h("h2", {}, "Universal rules"));
    d.append(h("div", { class: "col", "data-panel": "universal-rules" },
      checkbox("Every player is rated A at his registered position", u.registered_position_a, v => u.registered_position_a = v),
      row(weak("Starters play only where rated A, in preset:"), [1, 2, 3].map(p => checkbox(String(p), u.fielded_at_a.includes(p),
        v => u.fielded_at_a = v ? u.fielded_at_a.concat([p]).sort() : u.fielded_at_a.filter(x => x !== p)))),
      checkbox("No B ratings", u.no_b_ratings, v => u.no_b_ratings = v),
      checkbox("B ratings count toward the A-position allowance", u.b_uses_a_allowance, v => u.b_uses_a_allowance = v),
      checkbox("GK is never an outfield player's second A", u.gk_not_second_a, v => u.gk_not_second_a = v),
      row(checkbox("Age", u.age !== null, v => u.age = v ? [15, 50] : null), u.age ? [drag(u.age[0], v => u.age[0] = v ?? 0, { max: 50 }), "to", drag(u.age[1], v => u.age[1] = v ?? 0, { max: 50 })] : weak("any")),
      row(checkbox("Weight", u.weight !== null, v => u.weight = v ? { floor: 30, below: 129, above: 81 } : null),
        u.weight ? ["between height −", drag(u.weight.below, v => u.weight.below = v ?? 0), "(at least", drag(u.weight.floor, v => u.weight.floor = v ?? 0), "kg) and height −", drag(u.weight.above, v => u.weight.above = v ?? 0)] : weak("any")),
      checkbox("A captain is required", u.captain_required, v => u.captain_required = v),
      row("At least", drag(u.min_goalkeepers, v => u.min_goalkeepers = v ?? 0, { max: 5 }), "goalkeepers"),
      row("At most", drag(u.max_skill_cards, v => u.max_skill_cards = v ?? 0, { max: 41 }), "skill cards per player", weak("(a ruleset's choice; the official rules use 10)")),
      row(checkbox("At most", u.max_com_styles !== null, v => u.max_com_styles = v ? 5 : null), u.max_com_styles !== null ? drag(u.max_com_styles, v => u.max_com_styles = v ?? 0, { max: 7 }) : "", "COM styles per player")));
    d.append(h("div", { class: "sep" }), strong("Team settings"), h("div", { class: "weak" }, "Tactics settings a team must have, on the versions listed (every version when none is ticked)."));
    const tri = (t, k, label) => row(h("span", { style: "width:220px" }, label), combo([[null, "any"], ["true", "must be on"], ["false", "must be off"]],
      t[k] === null || t[k] === undefined ? null : String(t[k]), v => t[k] = v === null ? null : v === "true"));
    const settings = h("div", { "data-panel": "team-settings" });
    R.team_settings.forEach((t, i) => settings.append(h("div", { class: "group" },
      row(text(t.label, v => t.label = v, { width: 200 }), h("span", { style: "flex:1" }), remove(R.team_settings, i)),
      row(weak("PES"), VERSIONS.map(v => checkbox(String(v), t.versions.includes(v), on => t.versions = on ? t.versions.concat([v]).sort() : t.versions.filter(x => x !== v)))),
      tri(t, "man_marking", "Man marking set up"), tri(t, "auto_substitution", "Automatic substitution"),
      tri(t, "auto_offside_trap", "Automatic offside trap"), tri(t, "auto_preset_change", "Automatic preset change"),
      tri(t, "auto_attack_defence_levels", "Automatic attack/defence levels"),
      row(weak("Banned formation shapes:"), t.forbidden_shapes.map((s, j) => h("span", { class: "chip mono" }, s.join("-"), button("✕", () => { t.forbidden_shapes.splice(j, 1); structural(); }, { frame0: true }))),
        text("", v => t._shape = v, { width: 60, placeholder: "3-4-3", mono: true }),
        button("Add", () => { const s = (t._shape || "").split("-").map(Number); if (s.length === 3 && s.every(n => n >= 0)) { t.forbidden_shapes.push(s); delete t._shape; structural(); } })))));
    d.append(settings);
    d.append(row(button("+ Add team settings", () => { R.team_settings.push({ label: "Team settings " + (R.team_settings.length + 1), versions: [], man_marking: null,
      auto_substitution: null, auto_offside_trap: null, auto_preset_change: null, auto_attack_defence_levels: null, forbidden_shapes: [] }); structural(); })));
    return d;
  },
  specials() {
    const d = h("div", {}, h("h2", {}, "Special rules"), h("div", { class: "weak" },
      "Applied in order: numbers add up, settings take the last value. Drag to reorder."));
    const rules = h("div", { "data-panel": "special-rules" });
    R.specials.forEach((s, i) => {
      const severityShown = s.then.forbid != null || (s.then.require_cards || []).length;
      rules.append(reorderable(h("div", { class: "group" },
        row(handle(i), text(s.label, v => s.label = v, { width: 330 }),
          severityShown ? combo([["error", "Error"], ["warning", "Warning"], ["suggestion", "Suggestion"]], s.severity, v => s.severity = v, { title: "How a violation of this rule is reported" }) : "",
          h("span", { style: "flex:1" }), remove(R.specials, i)),
        h("div", { class: "row", style: "align-items:flex-start" }, h("span", { class: "strong", style: "width:40px" }, "When"), condEditor(s.when, `sp:${i}:when`, {})),
        h("div", { class: "row", style: "align-items:flex-start" }, h("span", { class: "strong", style: "width:40px" }, "Then"), effectsEditor(s.then, `sp:${i}:then`, false))), R.specials, i));
    });
    d.append(rules);
    d.append(row(button("+ Add special rule", () => { R.specials.push(special("New rule", {}, {})); structural(); })));
    return d;
  },
  suggestions() {
    const d = h("div", {}, h("h2", {}, "Suggestions"), h("div", { class: "weak" },
      "Allowances a player does not use up are listed as suggestions (off by default where results are shown)."));
    const all = [["cards", "Paid cards"], ["a_positions", "A positions"], ["weak_foot", "Weak foot"], ["injury_resistance", "Injury resistance"],
      ["height", "Height"], ["free_cards", "Free cards"], ["choices", "Choices not taken"], ["com", "Free COM styles"]]
      .concat(Object.keys(R.card_groups).map(g => [g, "Free " + human(g).toLowerCase()]));
    d.append(h("div", { class: "col", "data-panel": "suggestions-list" }, all.map(([id, label]) => checkbox(label, R.suggestions.includes(id),
      v => R.suggestions = v ? R.suggestions.concat([id]) : R.suggestions.filter(x => x !== id)))));
    return d;
  },
  custom() {
    return h("div", {}, h("h2", {}, "Custom checks"), h("div", { class: "weak" },
      "Hand-written Rhai, run after the ruleset's own checks; the editor keeps it as it is."),
      h("pre", { class: "mono", style: "background:var(--extreme); padding:6px; color:#b4b4b4", "data-panel": "custom-checks" }, R.custom_checks));
  },
  summary() {
    return h("div", {}, h("h2", {}, "Summary"), row(button("Copy", () => navigator.clipboard && navigator.clipboard.writeText(summary(R)), { title: "Copies the rules as Markdown, for posting them" }),
      weak("The ruleset as rules text, written from the data.")),
      h("pre", { id: "summary-text", class: "mono", style: "white-space:pre-wrap; background:var(--extreme); padding:8px; color:#c8c8c8; max-width:900px", "data-panel": "summary-text" }, summary(R)));
  },
};

function statGrid(all, selected, set) {
  const g = h("table", { class: "grid" });
  for (let i = 0; i < all.length; i += 5)
    g.append(h("tr", {}, all.slice(i, i + 5).map(s => h("td", {}, checkbox(human(s), selected.includes(s),
      v => set(v ? selected.concat([s]) : selected.filter(x => x !== s)))))));
  return g;
}
function perStatGrid(t) {
  const g = h("table", { class: "grid" });
  for (let i = 0; i < STATS.length; i += 5)
    g.append(h("tr", {}, STATS.slice(i, i + 5).map(s => h("td", {}, h("span", { class: "row", style: "margin:0; justify-content:space-between; width:180px" },
      human(s), drag(t.stats[s] ?? null, v => { if (v === null) delete t.stats[s]; else t.stats[s] = v; }, { max: 99, placeholder: String(t.rate) }))))));
  return g;
}
function cardGrid(selected, set) {
  const g = h("table", { class: "grid" });
  const all = CARDS;
  for (let i = 0; i < all.length; i += 6)
    g.append(h("tr", {}, all.slice(i, i + 6).map(c => h("td", {}, checkbox(human(c), selected.includes(c),
      v => set(v ? selected.concat([c]) : selected.filter(x => x !== c)), COM.includes(c) ? "COM playing style" : "Skill")))));
  return g;
}
function cardPicker(list, set) {
  return h("span", { class: "row", style: "margin:0" },
    list.map((c, j) => h("span", { class: "chip" }, human(c), button("✕", () => { list.splice(j, 1); structural(); }, { frame0: true }))),
    combo([[null, "+ card…"]].concat(CARDS.filter(c => !list.includes(c)).map(c => [c, human(c) + (COM.includes(c) ? " (COM)" : "")])), null, v => { if (v) set(list.concat([v])); }));
}

// ---- The condition editor ----
const COND_FIELDS = [
  ["tiers", "Tiers", c => c.tiers = []],
  ["positions", "Registered positions", c => c.positions = []],
  ["systems", "Height systems", c => c.systems = []],
  ["brackets", "Height brackets", c => c.brackets = []],
  ["height", "Height range", c => { c.height_min = 175; c.height_max = 185; }],
  ["captain", "Captain", c => c.captain = true],
  ["playing_styles", "Playing styles", c => c.playing_styles = []],
  ["holds", "Holds any of (cards)", c => c.holds = []],
  ["rated_a_at", "Rated A at", c => c.rated_a_at = []],
  ["rated_b_at", "Rated B at", c => c.rated_b_at = []],
  ["name_colors", "Name colour", c => c.name_colors = ["cc9900ff"]],
  ["fielded_only_at", "Fielded only at", c => c.fielded_only_at = "CB"],
  ["ranges", "Value range (a stat, age, weight, form, …)", c => c.ranges = Object.assign(c.ranges || {}, { [RANGE_VALUES.find(v => !(v in (c.ranges || {})))]: [0, 99] })],
  ["strong_foot", "Stronger foot", c => c.strong_foot = "left"],
  ["any", "Any of (several cases)", c => c.any = (c.any || []).concat([{}, {}])],
  ["not", "Except when", c => c.not = {}],
];
const has = (c, k) => k === "height" ? c.height_min != null || c.height_max != null
  : k === "captain" || k === "fielded_only_at" || k === "strong_foot" || k === "not" ? c[k] !== undefined && c[k] !== null
  : k === "ranges" ? false : c[k] !== undefined;
function condEditor(c, path, opts) {
  const allowed = COND_FIELDS.filter(([k]) => opts.heightsPositions ? ["positions", "height", "any", "not"].includes(k) : !(opts.noTiers && k === "tiers"));
  const box = h("div", { class: "col", style: "flex:1" });
  const drop = (keys) => button("✕", () => { keys.forEach(k => delete c[k]); structural(); }, { frame0: true, title: "Remove this condition" });
  const line = (label, widget, keys) => box.append(row(h("span", { style: "width:150px" }, label), widget, drop(keys)));
  const ids = (xs) => xs.map(x => x.id);
  const labelOf = (xs) => id => (xs.find(x => x.id === id) || { label: id }).label;
  if (c.tiers !== undefined) line("Tier is", chips(ids(R.tiers), c.tiers, v => c.tiers = v, labelOf(R.tiers)), ["tiers"]);
  if (c.positions !== undefined) line("Registered at", chips(POSITIONS, c.positions, v => c.positions = v, x => x), ["positions"]);
  if (c.systems !== undefined) line("Height system is", chips(ids(R.heights.systems), c.systems, v => c.systems = v, labelOf(R.heights.systems)), ["systems"]);
  if (c.brackets !== undefined) line("Bracket is", chips(ids(R.heights.brackets), c.brackets, v => c.brackets = v, labelOf(R.heights.brackets)), ["brackets"]);
  if (has(c, "height")) line("Height", h("span", { class: "row", style: "margin:0" },
    checkbox("from", c.height_min != null, v => c.height_min = v ? 175 : null), c.height_min != null ? drag(c.height_min, v => c.height_min = v) : "",
    checkbox("to", c.height_max != null, v => c.height_max = v ? 185 : null), c.height_max != null ? drag(c.height_max, v => c.height_max = v) : "", "cm",
    weak("(before any allowance)")), ["height_min", "height_max"]);
  if (has(c, "captain")) line("Captain", h("span", { class: "row", style: "margin:0" },
    [["yes", true], ["no", false]].map(([l, v]) => h("label", { class: "cb" }, h("input", { type: "radio", checked: c.captain === v, onchange: () => { c.captain = v; structural(); } }), l))), ["captain"]);
  if (c.playing_styles !== undefined) line("Playing style is", chips(STYLES, c.playing_styles, v => c.playing_styles = v), ["playing_styles"]);
  if (c.holds !== undefined) line("Holds any of", cardPicker(c.holds, v => c.holds = v), ["holds"]);
  if (c.rated_a_at !== undefined) line("Rated A at", chips(POSITIONS, c.rated_a_at, v => c.rated_a_at = v, x => x), ["rated_a_at"]);
  if (c.rated_b_at !== undefined) line("Rated B at", chips(POSITIONS, c.rated_b_at, v => c.rated_b_at = v, x => x), ["rated_b_at"]);
  if (c.name_colors !== undefined) line("Name colour is", h("span", { class: "row", style: "margin:0" },
    c.name_colors.map((n, j) => h("span", { class: "row", style: "margin:0" }, colour(n, v => c.name_colors[j] = v), remove(c.name_colors, j))),
    button("+", () => { c.name_colors.push("ffffffff"); structural(); }, { frame0: true })), ["name_colors"]);
  if (has(c, "fielded_only_at")) line("Fielded only at", h("span", { class: "row", style: "margin:0" }, combo(POSITIONS.map(p => [p, p]), c.fielded_only_at, v => c.fielded_only_at = v),
    weak("in every formation of the three presets")), ["fielded_only_at"]);
  for (const k of Object.keys(c.ranges || {})) {
    const range = c.ranges[k];
    box.append(row(h("span", { style: "width:150px" }, "Value"),
      combo(RANGE_VALUES.filter(v => v === k || !(v in c.ranges)).map(v => [v, human(v)]), k, v => {
        const next = {};
        for (const [kk, vv] of Object.entries(c.ranges)) next[kk === k ? v : kk] = vv;
        c.ranges = next;
      }),
      "from", drag(range[0], v => range[0] = v ?? 0, { max: 99 }), "to", drag(range[1], v => range[1] = v ?? 0, { max: 99 }),
      button("✕", () => { delete c.ranges[k]; if (!Object.keys(c.ranges).length) delete c.ranges; structural(); }, { frame0: true })));
  }
  if (has(c, "strong_foot")) line("Stronger foot", h("span", { class: "row", style: "margin:0" },
    ["right", "left"].map(f => h("label", { class: "cb" }, h("input", { type: "radio", checked: c.strong_foot === f, onchange: () => { c.strong_foot = f; structural(); } }), f))), ["strong_foot"]);
  if (c.any !== undefined) {
    const any = h("div", { class: "col", style: "flex:1" });
    c.any.forEach((a, j) => {
      if (j) any.append(weak("or"));
      any.append(h("div", { class: "group", style: "margin:0" }, row(weak(`Case ${j + 1}`), h("span", { style: "flex:1" }), c.any.length > 1 ? remove(c.any, j) : ""), condEditor(a, `${path}.any${j}`, opts)));
    });
    any.append(row(button("+ Add case", () => { c.any.push({}); structural(); }, { frame0: true })));
    box.append(h("div", { class: "row", style: "align-items:flex-start" }, h("span", { style: "width:150px" }, "Any of", h("div", { class: "weak" }, "(one case must hold)")), any, drop(["any"])));
  }
  if (has(c, "not")) box.append(h("div", { class: "row", style: "align-items:flex-start" }, h("span", { style: "width:150px" }, "Except when"),
    h("div", { class: "group", style: "margin:0; flex:1" }, condEditor(c.not, path + ".not", opts)), drop(["not"])));
  if (!box.childNodes.length) box.append(weak(opts.heightsPositions ? "Any player" : "Always (every player)"));
  box.append(row(menu("+ Condition", allowed.filter(([k]) => !has(c, k) && !(["tiers", "positions", "systems", "brackets", "playing_styles", "holds", "rated_a_at", "rated_b_at", "name_colors", "any"].includes(k) && c[k] !== undefined))
    .map(([k, label, add]) => [label, () => { add(c); structural(); }]), { frame0: true })));
  return box;
}

// ---- The effects editor ----
const EFFECTS = [
  ["stat_bonus_all", "Add to every stat", e => e.stat_bonus = Object.assign(e.stat_bonus || {}, { all: 1 })],
  ["stat_bonus_one", "Add to one stat", e => e.stat_bonus = Object.assign(e.stat_bonus || {}, { [STATS.find(s => !((e.stat_bonus || {})[s] !== undefined))]: 1 })],
  ["stat_target", "Set one stat's target", e => e.stat_target = Object.assign(e.stat_target || {}, { [STATS.find(s => !((e.stat_target || {})[s] !== undefined))]: 80 })],
  ["cards", "Extra paid cards", e => e.cards = 1],
  ["free", "Free cards of a group", e => e.free = Object.assign(e.free || {}, { com: 1 })],
  ["free_cards", "Free card", e => e.free_cards = (e.free_cards || []).concat([CARDS.find(c => !(e.free_cards || []).includes(c))])],
  ["require_cards", "Required card", e => e.require_cards = (e.require_cards || []).concat([CARDS.find(c => !(e.require_cards || []).includes(c))])],
  ["a_positions", "Extra A positions", e => e.a_positions = 1],
  ["max_a_positions", "At most N A positions", e => e.max_a_positions = 2],
  ["form", "Change form", e => e.form = 1],
  ["injury_resistance", "Change injury resistance", e => e.injury_resistance = 1],
  ["weak_foot", "Weak-foot limits", e => e.weak_foot = { usage: 4, accuracy: 4 }],
  ["height_allowance", "Height allowance", e => e.height_allowance = 4],
  ["stat_warning", "Warning band", e => e.stat_warning = 5],
  ["forbid", "Forbid", e => e.forbid = "Not allowed"],
  ["choice", "One of (a choice)", e => e.choice = [{ cards: 1 }, { a_positions: 1 }]],
];
function effectName(k) { const f = EFFECTS.find(([id]) => id === k || id.startsWith(k + "_")); return f ? f[1] : k; }
// A number for every tier, or one per tier (a tier left unticked is unaffected).
function perTier(get, set) {
  const v = get();
  const byTier = typeof v === "object";
  return h("span", { class: "row", style: "margin:0" },
    byTier ? R.tiers.map(t => h("span", { class: "row", style: "margin:0 6px 0 0" },
      checkbox(t.label, t.id in v, on => { if (on) v[t.id] = 0; else delete v[t.id]; }),
      t.id in v ? drag(v[t.id], n => v[t.id] = n ?? 0, { min: -99, max: 99, width: 44 }) : ""))
      : drag(v, n => set(n ?? 0), { min: -99, max: 99 }),
    combo([["all", "same for every tier"], ["tier", "per tier"]], byTier ? "tier" : "all", m => {
      if (m === "tier" && !byTier) { const o = {}; R.tiers.forEach(t => o[t.id] = v); set(o); }
      if (m === "all" && byTier) set(Object.values(v)[0] ?? 0);
    }));
}
function effectsEditor(e, path, inChoice) {
  const box = h("div", { class: "col", style: "flex:1" });
  const line = (label, widget, del) => box.append(row(h("span", { style: "width:170px" }, label), widget,
    button("✕", () => { del(); structural(); }, { frame0: true, title: "Remove this effect" })));
  const statCombo = (map, k) => combo(STATS.filter(s => s === k || !(s in map)).map(s => [s, human(s)]), k, v => {
    const next = {};
    for (const [kk, vv] of Object.entries(map)) next[kk === k ? v : kk] = vv;
    Object.keys(map).forEach(kk => delete map[kk]); Object.assign(map, next);
  });
  for (const k of Object.keys(e.stat_bonus || {})) {
    const del = () => { delete e.stat_bonus[k]; if (!Object.keys(e.stat_bonus).length) delete e.stat_bonus; };
    if (k === "all") line("Add to every stat", perTier(() => e.stat_bonus.all, v => e.stat_bonus.all = v), del);
    else line("Add to one stat", h("span", { class: "row", style: "margin:0" }, statCombo(e.stat_bonus, k), perTier(() => e.stat_bonus[k], v => e.stat_bonus[k] = v)), del);
  }
  for (const k of Object.keys(e.stat_target || {}))
    line("Set the target of", h("span", { class: "row", style: "margin:0" }, statCombo(e.stat_target, k), "to", perTier(() => e.stat_target[k], v => e.stat_target[k] = v)),
      () => { delete e.stat_target[k]; if (!Object.keys(e.stat_target).length) delete e.stat_target; });
  if (e.cards != null) line("Extra paid cards", perTier(() => e.cards, v => e.cards = v), () => delete e.cards);
  for (const g of Object.keys(e.free || {}))
    line("Free cards of a group", h("span", { class: "row", style: "margin:0" },
      combo(["com"].concat(Object.keys(R.card_groups)).filter(x => x === g || !(x in e.free)).map(x => [x, x === "com" ? "COM styles" : human(x)]), g, v => {
        const next = {}; for (const [kk, vv] of Object.entries(e.free)) next[kk === g ? v : kk] = vv; e.free = next; }),
      perTier(() => e.free[g], v => e.free[g] = v)), () => { delete e.free[g]; if (!Object.keys(e.free).length) delete e.free; });
  if (e.free_cards) line("These cards are free", cardPicker(e.free_cards, v => e.free_cards = v), () => delete e.free_cards);
  if (e.require_cards) line("Must hold", cardPicker(e.require_cards, v => e.require_cards = v), () => delete e.require_cards);
  if (e.a_positions != null) line("Extra A positions", perTier(() => e.a_positions, v => e.a_positions = v), () => delete e.a_positions);
  if (e.max_a_positions != null) line("At most", h("span", {}, drag(e.max_a_positions, v => e.max_a_positions = v ?? 0, { max: 13 }), " A positions"), () => delete e.max_a_positions);
  if (e.form != null) line("Change form by", perTier(() => e.form, v => e.form = v), () => delete e.form);
  if (e.injury_resistance != null) line("Change injury resistance by", perTier(() => e.injury_resistance, v => e.injury_resistance = v), () => delete e.injury_resistance);
  if (e.weak_foot) line("Weak foot up to", h("span", { class: "row", style: "margin:0" }, "usage", drag(e.weak_foot.usage, v => e.weak_foot.usage = v ?? 0, { max: 4 }),
    "accuracy", drag(e.weak_foot.accuracy, v => e.weak_foot.accuracy = v ?? 0, { max: 4 }), weak("(replaces the tier's)")), () => delete e.weak_foot);
  if (e.height_allowance != null) line("Counts", h("span", {}, drag(e.height_allowance, v => e.height_allowance = v ?? 0, { max: 20 }), " cm shorter for brackets"), () => delete e.height_allowance);
  if (e.stat_warning != null) line("Warning band", perTier(() => e.stat_warning, v => e.stat_warning = v), () => delete e.stat_warning);
  if (e.forbid != null) line("Forbid, saying", text(e.forbid, v => e.forbid = v, { width: 300 }), () => delete e.forbid);
  if (e.choice) {
    const c = h("div", { class: "col", style: "flex:1" },
      weak(e.choice.length === 1 ? "May get: players matching this may take it." : "The first option the player needs is given."));
    e.choice.forEach((o, j) => c.append(h("div", { class: "group", style: "margin:0" },
      row(weak(e.choice.length === 1 ? "Option" : `Option ${j + 1}`), h("span", { style: "flex:1" }), remove(e.choice, j)), effectsEditor(o, `${path}.choice${j}`, true))));
    c.append(row(button("+ Add option", () => { e.choice.push({}); structural(); }, { frame0: true })));
    box.append(h("div", { class: "row", style: "align-items:flex-start" }, h("span", { style: "width:170px" }, e.choice.length === 1 ? "May get" : "One of"), c,
      button("✕", () => { delete e.choice; structural(); }, { frame0: true })));
  }
  if (!box.childNodes.length) box.append(weak(inChoice ? "Nothing yet" : "No effect yet"));
  const offered = EFFECTS.filter(([k]) => !inChoice || CHOICE_EFFECTS.includes(k.replace(/_(all|one)$/, "")))
    .filter(([k]) => ["stat_bonus_one", "stat_target", "free", "free_cards", "require_cards"].includes(k) || (k === "stat_bonus_all" ? !(e.stat_bonus && "all" in e.stat_bonus) : e[k] == null));
  box.append(row(menu("+ Effect", offered.map(([k, label, add]) => [label, () => { add(e); structural(); }]), { frame0: true })));
  return box;
}

function firstShow() { load(AUTUMN26, "autumn26", true); startPage(); }
// The view's screen is the open section (the host's layout edit keys its
// tweaks by it).
const screen = () => section;
// The link's keys (plans/core/gui.md "Links"): the loaded template, the
// preview pane, the start page open or not (the view's start has it open, so
// a link that reached a section carries start=0).
const startMarked = () => !!document.querySelector('#overlay .window[data-dlg="start"]');
function linkKeys() {
  const k = {};
  if (loadedKey !== "autumn26") k.ruleset = loadedKey;
  if (showPreview) k.preview = "1";
  // Only a name of this view's own windows ("start" is the `start` key, and
  // the host's named windows are the host's `win` key).
  const w = document.querySelector("#overlay .window");
  if (w && DLGS[w.dataset.dlg]) k.dialog = w.dataset.dlg;
  if (!startMarked()) k.start = "0";
  return k;
}
// A `dialog` value to its opener: save() picks Saved or Not saved by the
// ruleset's problems, so both names open the same way.
const DLGS = { saved: save, "not-saved": save };
const linkWarn = what => console.warn("ruleset-editor link: " + what);
function linkOpen(scr, keys) {
  if (keys.ruleset !== undefined) {
    if (TEMPLATES[keys.ruleset]) load(TEMPLATES[keys.ruleset](), keys.ruleset, true);
    else linkWarn("ruleset=" + keys.ruleset);
  }
  if (keys.preview !== undefined) {
    if (keys.preview === "0" || keys.preview === "1") showPreview = keys.preview === "1";
    else linkWarn("preview=" + keys.preview);
  }
  if (SECTIONS.some(([id]) => id === scr)) section = scr;
  else linkWarn("unknown screen " + scr);
  if (keys.start !== undefined) {
    if (keys.start === "0") closeWindow();
    else if (keys.start === "1") { if (!startMarked()) startPage(); }
    else linkWarn("start=" + keys.start);
  }
  if (keys.dialog !== undefined) {
    if (DLGS[keys.dialog]) DLGS[keys.dialog]();
    else linkWarn("dialog=" + keys.dialog);
  }
}
registerTool({ id: "ruleset-editor", css: RULESET_CSS, draw: drawView, firstShow, onKey, commit, screen, linkKeys, linkOpen });
})();
