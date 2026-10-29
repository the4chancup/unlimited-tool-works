# Unlimited Tool Works — AATF rules plan

The AATF rules (the 4cc player rules a team's savefile data must satisfy; the community calls the
check "Auto-ATF") are checked and applied by `libs/aatf`. A ruleset is **data**: one Rhai map
naming the tiers, the heights, the card economy and a list of conditional "specials". A **generic
interpreter**, written in Rhai and carried inside every rules file, evaluates any such map. The
official ruleset and every invitational's are the same kind of file, differing only in their data.

Consumers: the [Save editor](save_editor.md) (the `check-aatf` command today; the Make
Gold/Silver/… quick actions and the card pickers' remaining-count badges in `libs/team_widgets`
come with the Phase 8 view), the [Team creator](../plans/team_creator.md) (legal defaults, the
check before writing) and the [Ruleset editor](../plans/ruleset_editor.md) (which writes the
data). Platform context is in the [core plan](core/README.md).

---

## Background

4ccEditor's AATF tool (`aatf.cpp`) hardcodes both the parameters and the logic in C++, so a ruleset
change is a recompile. Its git history (46 commits, 2019–2026) shows what changes between cups:

- **Numbers**, most editions: weak-foot limits, bracket quotas, tier rates, height thresholds.
- **One to three "specials" per cup**, every one of the form *players matching X get Y*: "non-medals
  registered at CB, LB or RB get +5 defensive prowess" (Spring 24), "Malicia is mandatory on every
  non-medal player" (Summer 24), "GKs and medals cannot be giants" (Autumn 24), "Green silver
  194s at −3", "Red non-medal CBs fielded only at CB may be 189" (Autumn 26), "trade a card for
  4/4 weak foot" (FAG13), "medals get a free A position *or* a free COM style" (Autumn 26).
- **Engine changes**, rarely: B ratings checked, playing-style ranges per version, which stats a
  version has.

A ruleset usually serves two cups. 2026's earlier official rulesets, Autumn 25 (Autumn 25 and
Winter 26) and Spring 26 (Spring and Summer 26), are Autumn 26's structure with two medal tiers
and none of its new specials ("Autumn 25 and Spring 26").

Invitationals already run their own rulesets on forks of the editor. VGL26 (4ccEditor-VGL, branch
`vgl26`, `stats.h` + `aatf_single_vgl`) is a complete one of a different shape: five tiers
including a "buffed" one, per-stat targets per tier, one exact height per tier and no height
brackets, a different rating stat set, free A positions per tier, all trick cards free, and a free
extra card for the captain. It also reports *suggestions*: allowances a player does not use
(fewer cards, COM styles or A positions than allowed, weak foot or injury resistance below the
maximum, a captain without the free Captaincy card), shown only on request and never counted as
errors. 4ccEditor has the same idea behind its `eCheck` warnings, hardcoded off.

The two VGL rulesets before it (VGL24, VGL25) add one shape: an *optional* manlet bonus. A manlet
may carry the base rating or the base plus the bonus (a regular 77 or 82), and a suggestion names
the bonus not taken.

VTL11 (the VTLEditor fork, `VTL7/stats.h` + `aatf.cpp` at `ab5f0e2`) adds further shapes: tiers
told apart by height and then by one stat (gold and silver share 195 cm and differ in swerve);
stat bonuses by playing style for golds; exact weak foot; banned and required cards; free cards
for some tiers, a free COM style (Long Ranger) and a cap of five COM styles; a minimum of three
paid skill cards, each extra A position counting as one; B ratings allowed only for 188 cm players
at CB, which also count toward the A-position allowance, and an A at CB or GK forbidden for them; medals'
names required to carry a gold or silver colour code and other players' forbidden to; at most two
registered positions among the 188 cm players, no 180 cm player sharing one (by user decision a
custom check, not schema; as the cup's checker does, "one" is the first two such positions in
roster order, since a third is already the too-many error); the skill card limit raised to 11 on every version (PES 21 loads
more than the official rules' 10); two warnings for a player rated A at CF, whose playing style the
league's exe treats differently there (a Goal Poacher's does not take effect, a Creative
Playmaker's does); and, on PES 16
only, team tactics settings (no man marking set up, auto substitution, offside trap and preset
change off).

VGL27 (4ccEditor-VGL, branch `vgl27`, tag `VGL27.B`) reshapes VGL26 toward the official layout:
no buffed tier and no per-stat targets, tiers recognized by rating alone, height brackets back
(one exact height each, one set of quotas) and the official manlet bonus.

The VGL26 and VGL27 rules pages (the VGL wiki's "Chapter III: Squad") state rules their checkers
do not test, or test loosely: every position a player is fielded at in the first preset must be
one of his A positions (VGL27's checker only asks that the registered position appear in the
first preset);
no man marking; every support setting off (the page's "switch preset tactic", which must be on,
is the tactics menu's option offering all three presets, not a team setting); injury
resistance exactly 2 for medals and 1 for non-medals (the checkers allow less, but for VGL27's
medals); and, in VGL26, no
formation of three defenders, four midfielders and three forwards (3-4-3 and its variants), fluid
formations included. A VGL ruleset here follows its rules page where the two disagree.

The schema below is the union of what these rulesets and the specials history need, and the
rulesets are its fixtures ("Development phase and verification").

---

## The rules file

One self-contained `.4ccrules` file is one ruleset, in three sections:

```rhai
// AATF ruleset, a Rhai script. Open it in the Ruleset editor to change it.
const RULESET = #{ ... };

// ==== LOGIC (ruleset-logic 1): the Ruleset editor replaces this section ====
fn check_team(team) { ... }
fn tier_values(player, tier, team) { ... }
fn card_limits(player, tier, team) { ... }
// ... the interpreter's own functions

// ==== CUSTOM CHECKS: optional, hand-written; the Ruleset editor keeps this section as it is ====
fn custom_checks(team) { [] }
```

- **`RULESET`** is the data ("The ruleset schema"). The Ruleset editor owns it and rewrites it
  whole; a hand edit is allowed and validated like any other.
- **The logic** is the generic interpreter ("The interpreter"), copied into every file so a
  ruleset behaves the same on every Studio build: a file made under one logic version keeps
  running it until someone updates it. Its banner names the version; "Loading and validation"
  describes how the host recognizes it.
- **Custom checks** are the escape hatch: a rule the schema cannot express yet, written in Rhai by
  someone who can, without a Studio release. `check_team` calls `custom_checks(team)` when the file
  defines it and appends what it returns.

A rules file a member or an organizer handles is named `<name>.4ccrules`, not `.rhai`: the
one ending every picker offers (the Save editor's Import rules file…, the Ruleset editor's
Open and Save), the name the Ruleset editor's Save suggests, the CLI's examples. A member only loads the file, and `.rhai` tells them nothing; `.4ccrules` says
it is the cup's rules and sits beside the suite's `.4ccm`. The content stays plain Rhai text,
so an organizer still edits it in a text editor (the CUSTOM CHECKS section is hand-written by
design): the file's first line says it is a Rhai script, for whoever wants their editor's Rhai
highlighting on it. The loader reads the content, never the ending, so `check-aatf --rules`
loads a file whatever it ends in. The repository's own sources keep `.rhai`
(`libs/aatf/logic/logic.rhai`, `rulesets/*.rhai`): they are code, edited with that highlighting,
and never handed to a member.

Rhai is a small scripting language written in pure Rust for embedding: scripts reach only what the
host registers, cannot touch the filesystem or network, and run under operation and call-depth
limits, so a broken file cannot hang the tool.

---

## The ruleset schema

`RULESET` deserializes into `Ruleset` (`rhai::serde::from_dynamic`; an unknown key is an error).
Tier, bracket, system and card-group ids are the ruleset's own strings. `Stat`, `Card`,
`Position` and `PlayStyle` are closed sets, spelled as `pes_savefile` spells them: stats are
`PlayerStats`' ability field names (`attacking_prowess` … `aggression`); a card is a skill or a
COM playing style, and cards, positions and playing styles are the Team TOML labels
(`interchange/team_toml/labels.rs`: `malicia`, `captaincy`, `long_ranger`, `GK`, `CB`,
`fox_in_the_box`, …).

```rust
pub struct Ruleset {
    pub schema: u32,
    pub name: String,
    pub squad_size: u8,
    pub stats: StatRules,
    /// Highest first: recognition takes the first tier that fits, and the quick-action
    /// buttons follow this order.
    pub tiers: Vec<Tier>,
    /// Named card sets a tier can make free (`Tier::free`); `com` is reserved for the
    /// seven COM playing styles, and no group id is one of `suggestions`' seven words. The
    /// groups are disjoint and hold no COM style.
    pub card_groups: BTreeMap<GroupId, Vec<Card>>,
    pub heights: Heights,
    pub universal: Universal,
    /// Applied in order: numeric effects add up, replacing effects take the last value.
    pub specials: Vec<Special>,
    pub team_settings: Vec<TeamSettings>,
    /// Allowances whose unused part is reported as a suggestion: `cards`, `a_positions`,
    /// `weak_foot`, `injury_resistance`, `height`, `free_cards`, `choices`, or a group id
    /// (`com` included).
    pub suggestions: Vec<String>,
}

pub struct StatRules {
    /// The stats whose maximum is a player's rating, compared with each tier's expected rating.
    pub rating: Vec<Stat>,
    /// Targets a player may sit below: ability stats, `form`, `injury_resistance`,
    /// `weak_foot_usage`, `weak_foot_accuracy`. Every other target must be met exactly.
    pub at_most: Vec<String>,
}

pub struct Tier {
    pub id: TierId,
    pub label: String,
    /// Exact number per team. `None` on a tier without `counts_as`: the rest of the squad
    /// (one such tier at most).
    pub count: Option<u8>,
    /// The tier whose count this tier's players fill (the goalkeeper tier counts as regular).
    pub counts_as: Option<TierId>,
    /// What a player must also meet to be this tier; may not name tiers.
    pub when: Condition,
    /// Target of every ability stat not in `stats`.
    pub rate: u8,
    pub stats: BTreeMap<Stat, u8>,
    pub form: u8,
    pub injury_resistance: u8,
    pub weak_foot: WeakFoot,
    /// Cards a player pays for: skills and COM styles that no free allowance covers.
    pub cards: u8,
    /// At least this many paid skill cards, each extra A position counting as one.
    pub min_cards: u8,
    /// Free cards per group.
    pub free: BTreeMap<GroupId, u8>,
    /// A positions included, the registered one counted; each extra A costs a card.
    pub a_positions: u8,
    /// The colour code (eight bytes, as `Condition::name_colors` spells them) this tier's
    /// names start with. When any tier sets one, a player of a tier that sets none carries
    /// none of the codes the tiers set (VTL11: medals gold or silver, the rest without).
    pub name_color: Option<String>,
}

pub struct WeakFoot {
    pub usage: u8,
    pub accuracy: u8,
}

pub struct Heights {
    /// Ascending. A player's bracket is the first that takes their height less any
    /// `height_allowance`; no bracket taking it is an illegal height. Empty: heights are
    /// constrained by tier conditions and specials alone.
    pub brackets: Vec<Bracket>,
    /// The first whose `any_player` holds sets the quotas; the last has none or an empty one.
    pub systems: Vec<HeightSystem>,
}

pub struct Bracket {
    pub id: BracketId,
    pub label: String,
    pub heights: BracketHeights,
}

/// Spelled `up_to: 175` or `exactly: [185]` directly in the bracket's map; `exactly` lists at
/// least one height.
pub enum BracketHeights {
    /// At most this, above the previous bracket.
    UpTo(u8),
    Exactly(Vec<u8>),
}

pub struct HeightSystem {
    pub id: SystemId,
    pub label: String,
    /// Holds when some player meets it; may name heights and positions only.
    pub any_player: Option<Condition>,
    /// Exact count per bracket; a bracket absent here is unconstrained.
    pub quotas: BTreeMap<BracketId, u8>,
}

pub struct Universal {
    pub registered_position_a: bool,
    /// Presets (1 to 3) in which a starting-eleven player may be fielded only at positions he is
    /// rated A at, in every formation the game can field from them (counted as for
    /// `Condition::fielded_only_at`). Empty: no such rule.
    pub fielded_at_a: Vec<u8>,
    pub no_b_ratings: bool,
    /// B ratings count toward the A-position allowance, like extra A positions.
    pub b_uses_a_allowance: bool,
    /// A GK rating is never a player's second A: it fires for any player holding GK A among
    /// several A positions, a goalkeeper with a second A included, as `aatf.cpp` tests it.
    pub gk_not_second_a: bool,
    /// Inclusive.
    pub age: Option<(u8, u8)>,
    pub weight: Option<WeightRule>,
    pub captain_required: bool,
    pub min_goalkeepers: u8,
    /// Skill cards a player may hold, free ones included: a ruleset's choice (the official
    /// rules' 10; VTL11 uses 11, on every version; DBC7 none, the game not holding a player to
    /// ten in play).
    pub max_skill_cards: u8,
    pub max_com_styles: Option<u8>,
}

/// Weight within `max(floor, height - below)..=height - above` (Autumn 26: 30, 129, 81).
pub struct WeightRule {
    pub floor: u8,
    pub below: u8,
    pub above: u8,
}

/// Every present field must hold; an empty list or `None` always holds.
pub struct Condition {
    pub tiers: Vec<TierId>,
    /// Registered position.
    pub positions: Vec<Position>,
    pub systems: Vec<SystemId>,
    pub brackets: Vec<BracketId>,
    /// Height in cm, inclusive, before any allowance.
    pub height_min: Option<u8>,
    pub height_max: Option<u8>,
    pub captain: Option<bool>,
    pub playing_styles: Vec<PlayStyle>,
    /// Holds any of these cards.
    pub holds: Vec<Card>,
    /// Rated A (or B) at any of these positions.
    pub rated_a_at: Vec<Position>,
    pub rated_b_at: Vec<Position>,
    /// The name starts with one of these colour codes (the eight bytes after `\x11c`,
    /// `cc9900ff`; see `pes_savefile`'s "Name colour codes").
    pub name_colors: Vec<String>,
    /// Fielded at this position in every formation the game can field: all three presets,
    /// the three formations of a fluid preset, the kick-off formation of the others. A player
    /// outside the starting eleven: registered there.
    pub fielded_only_at: Option<Position>,
    /// Each value within its inclusive range (`ranges: #{ speed: [90, 99] }`), on the game's
    /// scale; a stat the version lacks fails its range. Height has its own fields above.
    pub ranges: BTreeMap<PlayerValue, (u8, u8)>,
    pub strong_foot: Option<Foot>,
    /// At least one of these holds; empty always holds. With `not`, any combination of fields
    /// is one condition, so a rule with several cases is one special and its additive effects
    /// apply once to a player who meets two of them.
    pub any: Vec<Condition>,
    pub not: Option<Box<Condition>>,
}

/// A number `Condition::ranges` tests: an ability stat by its name, or `age`, `weight`,
/// `form`, `injury_resistance`, `weak_foot_usage`, `weak_foot_accuracy`.
pub enum PlayerValue {
    Stat(Stat),
    Age,
    Weight,
    Form,
    InjuryResistance,
    WeakFootUsage,
    WeakFootAccuracy,
}

/// Spelled `right` and `left`, the Team TOML labels.
pub enum Foot {
    Right,
    Left,
}

pub struct Special {
    /// Shown in the editor and with the violations it causes.
    pub label: String,
    pub when: Condition,
    pub then: Effects,
    /// Of the `forbid` and `require_cards` violations; default `error`.
    pub severity: Severity,
}

/// Every field optional. A `PerTier` value is one number for every tier or a map by tier
/// id, a tier absent from the map being unaffected.
pub struct Effects {
    /// Added to the target of every ability stat (`all`) or of one stat.
    pub stat_bonus: BTreeMap<StatOrAll, PerTier<i8>>,
    /// Replaces one stat's target.
    pub stat_target: BTreeMap<Stat, PerTier<u8>>,
    pub cards: Option<PerTier<i8>>,
    pub free: BTreeMap<GroupId, PerTier<i8>>,
    /// These cards cost nothing.
    pub free_cards: Vec<Card>,
    pub require_cards: Vec<Card>,
    pub a_positions: Option<PerTier<i8>>,
    pub max_a_positions: Option<u8>,
    /// Added to the tier's form target and injury-resistance limit.
    pub form: Option<PerTier<i8>>,
    pub injury_resistance: Option<PerTier<i8>>,
    /// Replaces the tier's limits.
    pub weak_foot: Option<WeakFoot>,
    /// Centimetres subtracted from the height before bracketing.
    pub height_allowance: Option<u8>,
    /// A violation with this text.
    pub forbid: Option<String>,
    /// The first option the player uses, or none: an option is used when, without it, the
    /// player exceeds a limit the option, applied, raises for him (a per-tier value for another
    /// tier raises nothing, nor does a replacement a later special overrides); a `stat_bonus`
    /// raises the stat targets, so it is used when, without it, a stat it raises is over its own
    /// target (a stat whose tier override sits below the rate counts, as VGL26's silver place
    /// kicking does; the overall rating alone would miss it); a `free_cards` option, when, without it, the paid cards are over
    /// the limit and, with it, fewer are paid (a card a group covers can still help, its
    /// group's allowance passing to a paid groupmate). A one-option
    /// choice is an optional effect (VGL25's manlet bonus). An option holds only effects that
    /// raise a limit (`stat_bonus`, `cards`, `free`, `free_cards`, `a_positions`,
    /// `max_a_positions`, `weak_foot`): any other could never be found used. Choices are
    /// resolved after every other effect, but a chosen option's replacing effect
    /// (`max_a_positions`, `weak_foot`) gives way to one a later special sets, as the order of
    /// `specials` says.
    pub choice: Vec<Effects>,
}

/// Team tactics settings required on the listed versions (every version when empty); each
/// present field must have this value.
pub struct TeamSettings {
    pub label: String,
    pub versions: Vec<PesVersion>,
    /// Man marking set up on any preset: any of a preset's eleven man-marking assignments set
    /// (PES 15 to 21; see "The host").
    pub man_marking: Option<bool>,
    /// Any setting but off (stored 0).
    pub auto_substitution: Option<bool>,
    pub auto_offside_trap: Option<bool>,
    pub auto_preset_change: Option<bool>,
    pub auto_attack_defence_levels: Option<bool>,
    /// Banned formation shapes, as defender, midfielder and forward counts (`[3, 4, 3]`), in
    /// every formation the game can field from any preset. Defenders are CB, LB and RB;
    /// midfielders DMF, CMF, LMF, RMF and AMF; forwards LWF, RWF, SS and CF.
    pub forbidden_shapes: Vec<[u8; 3]>,
}

pub enum PerTier<T> {
    All(T),
    ByTier(BTreeMap<TierId, T>),
}

pub enum StatOrAll {
    All,
    Stat(Stat),
}

pub enum Severity {
    Error,
    Warning,
    Suggestion,
}
```

A field left out of the file takes its empty value (0, `false`, an empty list or map, `None`),
except `schema`, `name`, `squad_size`, `universal` with its `max_skill_cards`, and each tier's,
bracket's and system's `id`, `label` and (tiers) `rate`, and each special's and team
settings entry's `label`, a present `weight`'s `floor`, `below` and `above`, and a present
`weak_foot`'s `usage` and `accuracy` (a tier's or an effect's), which are
required (an absent `universal` would otherwise default `max_skill_cards` to 0, a cap no file
means; a weight rule missing a bound would demand a weight equal to the height; a weak foot
missing a half would cap it at 0, below the game's 1; a violation
shows its special's label, which an empty one would leave blank).

Rules the types cannot carry, checked on load ("Loading and validation"): no string is empty
(an emptied name, id, label or `forbid` text names or says nothing), and a `name_colors` entry
or a tier's `name_color` is eight bytes (a partial paste matches no name); ids unique within their
kind, and every id a condition, effect, quota, `counts_as` or `suggestions` entry names exists; a
tier with `counts_as` has no `count`, and the tier it names has no `counts_as` of its own (itself
included); tier counts fit `squad_size`, and equal it when no tier is the rest of the squad (else
no team can be legal); each bracket's heights lie above every height of the brackets before it
(a reordered bracket would take no player); every height system but the last has a non-empty
`any_player` and the last has none or an empty one (an absent or empty condition holds for
every team, so the systems after it could never apply); a special with
`height_allowance` names no bracket in its condition, nested `not` and `any` included, since
brackets are computed from the allowance; a `choice` option holds only the effects its doc
lists; `fielded_at_a` names presets 1 to 3; a forbidden shape's counts add up to 10; no card is
in two `card_groups` and none holds a COM style (the built-in `com` group has them): a card in
two groups would be made free by whichever group the allocation tries first, so a team's
legality would hang on the groups' names (no shipped ruleset overlaps).

### How a player is read

The order is fixed, so every condition sees settled inputs:

1. **Height system** (team): the first system whose `any_player` holds.
2. **Tier** (player): for each tier in list order, the specials are evaluated as if the player
   were that tier: allowances first, then the bracket, then every other effect. The tier is the
   first whose `when` holds and whose *expected rating* (the highest target among `stats.rating`)
   is at most the player's rating. If none fits, the last tier whose `when` holds is taken; if no
   tier's `when` holds, the player gets `NoTier` and no further per-player check.
3. **Checks** (player), against the recognized tier's targets and limits.
4. **Team checks**: tier counts (`counts_as` folded in), the system's bracket quotas, captain,
   goalkeepers, team settings for the save's version, then `custom_checks`.

Recognition by expected rating is what lets a special move a tier's threshold: a Green silver
giant's target is 89, so a rating of 89 reads as silver, not bronze.

### The card economy

A player's **paid cards** are their skills plus COM styles, less the free ones held: per group,
up to that group's free count; every held card in `free_cards`. The **limit** is the tier's
`cards` plus every `cards` effect, less one per A position beyond the allowance (`a_positions`
plus effects; B ratings counted with the A positions under `b_uses_a_allowance`). Paid cards over
the limit is a violation, the limit falling below zero when the extra A positions outnumber the
cards (a regular with three cards and five A positions over one allowed is over it with no card at
all); so are paid skill cards plus extra A positions under `min_cards`, a
required card not held, more A positions than `max_a_positions`, more skill cards than
`max_skill_cards` and more COM styles than `max_com_styles`. The version's valid
registered-position and playing-style ranges are built in.

---

## Embedded rulesets

A shipped binary embeds every cup's current rulesets (`aatf::templates`, `aatf::builtin`), so a
member picks their cup's rules from the Save editor's ruleset dropdown (`spec/save_editor.md`
"Configurable AATF rules") rather than finding and importing a file, and a built-in ruleset is
never a copy a member can lose. They are the Ruleset editor's templates too. In the
dropdown's order, by description: `4cc Autumn 26`, `4cc Spring 26`, `/vg/ League 27`, `/vg/
League 26`, `/vt/ League 11`, `/vt/ League 10`, `Divegrass Before Christmas 7`; the first is
the default and the fallback. Each file's `name` is its description: for the 4cc's own cups
`4cc` then the season and its year's last two digits; for VGL and VTL the league's name then
its edition number, which counts editions, not years; for DBC7 the cup's name then its
edition. All seven are the ones the parity runs checked ("Development phase and
verification"; DBC7's, `plans/aatf_rules.md` "Development phase and verification").

`libs/aatf` keeps the latest two rulesets of each kind of cup with an organization of its own,
the 4cc's own (the official AATF), VGL (/vg/ League) and VTL (/vt/ League), and DBC7
(Divegrass Before Christmas). A cup's new edition joins the list, and its kind keeps its
latest two; the older one leaves the list and stays in `rulesets/` as a test ruleset with its
fixtures (`src/fixtures/`), built into the tests alone, as Autumn 25 and VGL25 do. Minor
invitationals (VTLA, UnityCup) get no built-in ruleset.

## The official ruleset

The default file, embedded in `libs/aatf`. Transcribed from `aatf.cpp` at 4ccEditor's
`Autumn_2026_AATF` tip `8f01926` (the file last changed in `f5e7b3e`), then corrected to the
cup's written rules where the two 4ccEditor editions read them differently (below). The card
groups take `aatf.cpp`'s skill indices as canonical slots, checked against `pes_savefile`'s
schema.

```rhai
const RULESET = #{
    schema: 1,
    name: "Autumn 26",
    squad_size: 23,

    stats: #{
        rating: ["dribbling", "goalkeeping", "finishing", "low_pass", "lofted_pass", "heading",
                 "swerve", "catching", "clearing", "reflexes", "coverage", "body_control",
                 "physical_contact", "kicking_power", "explosive_power", "ball_control",
                 "ball_winning", "jump", "place_kicking", "stamina", "speed", "aggression"],
        at_most: ["attacking_prowess", "defensive_prowess",
                  "injury_resistance", "weak_foot_usage", "weak_foot_accuracy"],
    },

    tiers: [
        #{ id: "gold", label: "Gold", count: 1, rate: 99, form: 8, injury_resistance: 3,
           weak_foot: #{ usage: 4, accuracy: 4 }, cards: 6, free: #{ tricks: 3 }, a_positions: 1 },
        #{ id: "silver", label: "Silver", count: 2, rate: 92, form: 8, injury_resistance: 3,
           weak_foot: #{ usage: 4, accuracy: 4 }, cards: 5, free: #{ tricks: 3 }, a_positions: 1 },
        #{ id: "bronze", label: "Bronze", count: 2, rate: 86, form: 8, injury_resistance: 3,
           weak_foot: #{ usage: 4, accuracy: 4 }, cards: 4, free: #{ tricks: 2 }, a_positions: 1 },
        #{ id: "goalkeeper", label: "Goalkeeper", counts_as: "regular", when: #{ positions: ["GK"] },
           rate: 77, form: 4, injury_resistance: 1,
           weak_foot: #{ usage: 2, accuracy: 2 }, cards: 2, free: #{}, a_positions: 1 },
        #{ id: "regular", label: "Regular", rate: 77, form: 4, injury_resistance: 1,
           weak_foot: #{ usage: 2, accuracy: 2 }, cards: 3, free: #{ tricks: 2 }, a_positions: 1 },
    ],

    card_groups: #{
        tricks: ["scissors_feint", "flip_flap", "marseille_turn", "sombrero", "cut_behind_and_turn",
                 "scotch_move", "rabona", "double_touch", "crossover_turn", "step_on_skill",
                 "no_look_pass"],
    },

    heights: #{
        brackets: [
            #{ id: "manlet", label: "Manlet", up_to: 175 },
            #{ id: "mid",    label: "Mid",    exactly: [180] },
            #{ id: "tall",   label: "Tall",   exactly: [185] },
            #{ id: "giant",  label: "Giant",  exactly: [194] },
            #{ id: "giga",   label: "Giga",   exactly: [199] },
        ],
        systems: [
            #{ id: "green", label: "Green", any_player: #{ height_min: 194 },
               quotas: #{ giga: 0, giant: 6, tall: 6, mid: 5, manlet: 6 } },
            #{ id: "red", label: "Red",
               quotas: #{ giga: 0, giant: 0, tall: 10, mid: 7, manlet: 6 } },
        ],
    },

    universal: #{
        registered_position_a: true, fielded_at_a: [1], no_b_ratings: true,
        gk_not_second_a: true,
        age: [15, 50], weight: #{ floor: 30, below: 129, above: 81 },
        captain_required: true, min_goalkeepers: 1, max_skill_cards: 10,
    },

    specials: [
        #{ label: "Malicia is a free card", then: #{ free_cards: ["malicia"] } },
        #{ label: "The captain's Captaincy card is free", when: #{ captain: true },
           then: #{ free_cards: ["captaincy"] } },
        #{ label: "Medals cannot be goalkeepers",
           when: #{ tiers: ["gold", "silver", "bronze"], positions: ["GK"] },
           then: #{ forbid: "Medals cannot play as GK" } },
        #{ label: "Goalkeepers at 189 count as tall",
           when: #{ positions: ["GK"], height_min: 189, height_max: 189 },
           then: #{ height_allowance: 4 } },
        #{ label: "Tall goalkeepers are 189",
           when: #{ positions: ["GK"], height_min: 181, height_max: 188 },
           then: #{ forbid: "GKs in this bracket must be 189cm" } },
        #{ label: "Goalkeepers below giant",
           when: #{ positions: ["GK"], height_min: 194 },
           then: #{ forbid: "GK heights cannot be 194cm or more" } },
        #{ label: "Gold below giant",
           when: #{ tiers: ["gold"], height_min: 194 },
           then: #{ forbid: "Gold heights cannot be 194cm or more" } },
        #{ label: "Red CBs at 189 count as tall",
           when: #{ tiers: ["regular"], systems: ["red"], positions: ["CB"], height_min: 189,
                    height_max: 189 },
           then: #{ height_allowance: 4 } },
        #{ label: "Manlets", when: #{ brackets: ["manlet"] },
           then: #{ cards: 1, a_positions: 1, weak_foot: #{ usage: 4, accuracy: 4 } } },
        #{ label: "Red manlet bonus",
           when: #{ tiers: ["regular", "goalkeeper", "bronze", "silver"], systems: ["red"],
                    height_max: 175 },
           then: #{ choice: [#{ stat_bonus: #{ all: #{ regular: 5, goalkeeper: 5, bronze: 3, silver: 2 } } }] } },
        #{ label: "Green silver giants", when: #{ tiers: ["silver"], systems: ["green"], height_min: 194 },
           then: #{ stat_bonus: #{ all: -3 } } },
        #{ label: "Medals trade a free COM style for a free A position",
           when: #{ tiers: ["gold", "silver", "bronze"] },
           then: #{ choice: [#{ free: #{ com: 1 } }, #{ a_positions: 1 }] } },
        #{ label: "Gold trades a second free COM style for a free A position",
           when: #{ tiers: ["gold"] },
           then: #{ choice: [#{ free: #{ com: 1 } }, #{ a_positions: 1 }] } },
    ],

    suggestions: ["cards", "tricks", "com", "weak_foot", "injury_resistance", "choices"],
};
```

Where the data reads differently from `aatf.cpp`, on purpose: `aatf.cpp` prints its two height
warnings for gold (above 194, and at 194) separately; here they are one. A player whose rating fits
no tier's target is placed by expected rating rather than by `aatf.cpp`'s thresholds, so an illegal
player can be reported against a different tier; both report the player.

Where the transcription's reading and the cup's written rules differ, the file follows the wiki's
Appendix:Rules as edited on 2026-10-06 (`DECISIONS.md`, 2026-10-21): a medal's free COM styles
are trades, each kept as a free COM style or spent on a free extra A position (gold has two to
trade, silver and bronze one, and an extra A at GK is already `gk_not_second_a`'s violation); a
bronze has two free trick cards, not three; the Red CB allowance asks only the registered
position, so a Red non-medal CB is tall at 189 and, like anyone, at 185, while 181 to 184 and 186
to 188 stand outside every bracket, as the goalkeepers' heights do; and the deadline export's
first preset fields every starter at one of his A positions. The 4cc organizer's answers
(`DECISIONS.md`, 2026-10-22) confirmed those and took the wiki's wording on three more points
where the official 4ccEditor reads otherwise: the middle bracket is exactly 180, so 176 to 179
stand in no bracket; a gold or a goalkeeper at 194 is an error; and the Red 175 cm bonus is
optional, a one-option `choice` for the tiers it names (a gold has none), so a manlet at his
tier's base rate is legal too and the untaken bonus is suggested.

## Autumn 25 and Spring 26

Autumn 25 is a test ruleset and Spring 26 a template ("Embedded rulesets"); both are there so
the official parity runs each 2026 cup save under the ruleset its teams were built for. Transcribed from `aatf.cpp` at 4ccEditor's `7e01541` (Autumn 25;
its Version B, `aae4835` and `bb3e54a`, changed no rule) and `c92d535` (Spring 26). Both are the
Autumn 26 file with:

- no bronze tier; gold `count` 2; silver `rate` 88 and `cards` 4; gold `cards` 5;
- the medals' COM styles free outright, no trade: `free: #{ tricks: 3, com: 2 }` for gold,
  `com: 1` for silver;
- medals' `weak_foot` usage 2, accuracy 4;
- Green quotas giga 0, giant 5, tall 6, mid 6, manlet 6;
- the "Red manlet bonus" for regular and goalkeeper only (+5);
- no "Green silver giants", "Red CBs at 189 count as tall" or the medals' COM-style trades;
- no `fielded_at_a`, and no `"choices"` in `suggestions`.

They differ from each other in two values: Autumn 25 has 2 silvers and goalkeepers at 77, Spring
26 has 3 silvers and goalkeepers at 74.

## VGL25

A test ruleset (not a template), transcribed from 4ccEditor-VGL's `vgl25` branch (`2c2d26a`;
`aatf_single_vgl` and its `_vgl` settings in `aatf.cpp`). VGL24 (`vgl24`, released at `d4d7b15`)
is the same checker with no manlet bonus, giants at 194 with a quota of 0, and no free Acrobatic
Finishing; it adds nothing to test and has no file. What VGL25 exercises that the others do not:

| Feature | VGL25 |
|---|---|
| Tiers | gold 2 (99), silver 3 (88), regular (77, the rest); goalkeepers are regulars, with no tier of their own |
| Optional manlet bonus | `when: #{ brackets: ["manlet"] }`, `choice: [#{ stat_bonus: #{ all: #{ regular: 5, silver: 2, gold: 2 } } }]`; with `choices` in `suggestions` |
| Heights | manlet up to 175, then exactly 180, 185, 190, 210 under one system (quotas 6, 6, 5, 6, 0); a goalkeeper at 189 counts as 185; goalkeepers at 181–188 forbidden, medals and goalkeepers at 190 forbidden |
| Manlets | the official "Manlets" special (+1 card, +1 A position, weak foot 4/4) |
| Cards | `tricks` (Malicia included) all free; free COM 1, 1, 0; paid 5, 4, 3; `a_positions` 2; Captaincy for the captain and Acrobatic Finishing free; the captain one extra card |
| Universal | no captain or goalkeeper requirement; a B rating, itself a violation under `no_b_ratings`, still counts toward the A positions (`b_uses_a_allowance`); `max_skill_cards` 41, the most skills any version has, so no cap |
| Injury resistance | exact: 2 for medals, 1 for non-medals (not in `at_most`) |
| Suggestions | `cards`, `com`, `a_positions`, `weak_foot`, `height`, `free_cards`, `choices` |

Where the data reads differently from `aatf_single_vgl`, on purpose: it lets a gold carry injury
resistance below 2, where the VGL rules set it to 2 for medals (maintainer, from the VGL wiki); it
repeats the bonus suggestion at team level for manlets under 175, here it is reported once; it
checks no registered-position or playing-style range, which `libs/aatf` builds in; and two
suggestions read differently: a goalkeeper rated A at GK is still told of an unused A position,
and a captain without the Captaincy card is told through the `free_cards` suggestion rather than
a team-level line.

## VGL26

A template ("Embedded rulesets"), transcribed
from the VGL26 rules page and 4ccEditor-VGL's `vgl26` branch (tip `ad378d2`; `stats.h`, and
`aatf_single_vgl` in `aatf.cpp`). What it exercises that the official file does not:

| Feature | VGL26 |
|---|---|
| Tiers | gold 1, silver 2, bronze 2, buffed 8, regular 10; goalkeeper `counts_as` regular |
| Per-stat targets | each tier's `rate` (99, 95, 91, 85, 80; 72 for goalkeepers) plus `stats` overrides (silver: finishing, heading and attacking prowess 99, defensive prowess 60, …) |
| Tier recognition | each outfield tier's `when` fixes one height (gold and silver 195, bronze 190, buffed 185 and not a goalkeeper, regular 180); the goalkeeper's `when` is `positions: ["GK"]` alone, a `forbid` special holding him to 185; no brackets, no systems |
| Rating stats | the official set less finishing, heading and aggression |
| Cards | `tricks` (the rules page's twelve, Malicia included) free; Heading free for everyone (`free_cards`); paid 9, 8, 8, 7, 6, goalkeeper 5; free COM 2 for medals, 1 for the rest; `a_positions` 2 (goalkeeper 1); the captain's Captaincy free and one extra card (`cards: 1` for `captain: true`) |
| Injury resistance | exact: 2 for medals, 1 for the rest |
| Universal | `fielded_at_a: [1]`; `max_skill_cards` 41, so no cap |
| Team settings | no man marking; auto substitution, offside trap and preset change off; `forbidden_shapes: [[3, 4, 3]]` |
| Suggestions | `cards`, `com`, `a_positions`, `weak_foot`, `free_cards` |

Where the data reads differently from `aatf_single_vgl`, on purpose (the rules page is stricter):
injury resistance is exact, where the checker allows less; the first-preset A-position rule,
the team settings and the formation ban are checked, where the checker tests none of them; a
registered goalkeeper must be 185 cm (a special), where the checker reads one at 180 as a
regular; and Heading is free beside the twelve tricks, where the checker counts it among them
(a player holding all thirteen pays one card there). Goalkeepers' form is 8, as both say. The
page's "medal and buffed players cannot be played as a goalkeeper at any time" is checked in
preset 1 only (an outfielder cannot be rated A at GK, and preset 1 fields A positions alone):
no condition reads the positions fielded in presets 2 and 3. The
rules page's "support settings" are read as substitution, offside trap and preset change: its
next rule allows auto attack/defence levels at fixed values, so that setting is left free.
The page's "switch preset tactic", which must be on, is not one of them: it is the tactics
menu's option offering all three presets instead of one, a manager's own setting the save does
not hold per team, so no ruleset checks it. `auto_preset_change` is the team's automatic preset
change during a match (the 4cc wiki's Edit-file page calls the byte "Auto change Preset
Tactics"), a support setting, so it is off, as in VTL10 and VTL11; every team of the three PES 21
cup saves holds it off.

## VGL27

A template ("Embedded rulesets"), transcribed from the VGL27 rules page and 4ccEditor-VGL's `vgl27`
branch (tip `e2add10`, tag `VGL27.B`; the ruleset is commit `4447996`). What it exercises that the
others do not:

| Feature | VGL27 |
|---|---|
| Tiers | gold 1, silver 2, bronze 2 (rates 99, 94, 89), goalkeeper (77) `counts_as` regular (77, the rest); no per-stat targets |
| Tier recognition | rating alone; every other tier's `when` excludes GK (the regular's too, or a GK rated under 77 would fall back to it), so a registered GK is a goalkeeper whatever his rating |
| Heights | five brackets of one exact height each (175, 180, 185, 190, 210) under one system with no `any_player` (quotas 6, 7, 10, 0, 0); a goalkeeper at 189 counts as 185 (`height_allowance: 4`); the official "Manlets" special |
| Cards | VGL26's shape with smaller numbers and no free Heading: paid 6, 5, 5, 4, goalkeeper 4; free COM 1, 1, 1, 0, 0 |
| Injury resistance, universal, team settings | as VGL26, without the formation ban |
| Rating stats, suggestions | as VGL26 |

Where the data reads differently from `aatf_single_vgl`, on purpose: the rules page is followed
where it is stricter (exact injury resistance for non-medals, which the checker lets go below 1;
the team settings are checked; a goalkeeper at 185 is forbidden, since the page's tall bracket
takes goalkeepers at 189 only, where the checker counts him as tall; a goalkeeper at 175 or 180
is in those brackets, which name no position); the captain's extra allowance is the page's one
of a card, a COM style or an A position (a `choice`), where the checker always grants a card;
the checker's positional test (the registered position appears
in the first preset) gives way to the page's first-preset A-position rule, which judges every
position a player is fielded at there. A rating between two tiers' targets (96) is placed by
expected rating, as in the official file, where `aatf_single_vgl` reports no tier. `VGL27.B`
brought the checker to the page on the rest: Heading is a paid card, a medal's injury
resistance is exact, a height in no bracket is a violation for that player, and each quota
message prints its own bracket's count (before it, the 185 cm count for every bracket).

## VTL10

A template, transcribed from the VTLEditor fork at `2cf222f` (2026-03-01), the last commit before
VTL10's first matchday on 2026-03-13: `5b78e5b` ("small aatf changes", 2026-04-12) and `4333325`
("post vtl10 update") came during the cup and are not its ruleset. It differs from VTL11 in:

- heights: medals at 194 (told apart by swerve, gold 99 and silver 92), outfield regulars at 178
  and their goalkeepers at 180 (a special for each wrong one), the buffed at 188;
- targets: its own per tier (silver's rate 92; the file stores absolute values); no
  playing-style bonuses or warnings;
- cards: gold's paid 6 and free COM 2; its own thirteen free skills; Incisive Run free beside Long
  Ranger; no banned card; the skill card cap at 12;
- no B rating anywhere (`no_b_ratings`); a buffed player rated A at LB or RB has weak foot 2/2;
- no name colours.

Its custom checks are VTL11's, with 178 cm for the regulars who may not share a 188 cm player's
registered position. The VTL wiki's VTL10 page found in 2026-10 describes another ruleset entirely
(medals rated 89, 94 and 99, eighteen regulars, heights 185, 180 and 175); the checker is
followed, and the cup save bears it out: the 43 teams the checker passes are the ones the data
block passes, and every other team fails both.

---

## The interpreter

The logic section implements the three functions the host calls, over `global::RULESET` and the
host's registered accessors:

- `check_team(team) -> [violation]`: steps 1–4 of "How a player is read" for every player.
- `tier_values(player, tier, team) -> map`: the targets a player of `tier` must carry (per stat,
  form, injury resistance, weak foot), specials applied. Behind `apply_tier`.
- `card_limits(player, tier, team) -> map`: the paid cards the player holds now, paid limit
  (below zero when the extra A positions outnumber the cards), free count per group, free and
  required cards, A positions allowed, and the `max_a_positions` cap when a special sets one.
  Behind the card pickers' remaining-count badges (the Save editor's, Phase 8), each `limit -
  paid`: the logic counts the paid cards, so a widget never recounts them with a second card
  economy that could drift from the checks.

A violation crosses back as a map the host converts into:

```rust
pub struct Violation {
    /// Roster slot; `None`: the team.
    pub slot: Option<u8>,
    pub severity: Severity,
    pub kind: ViolationKind,
}

pub enum ViolationKind {
    /// `rated_a`, here and below: every position the player is rated A at.
    RegisteredNotA { rated_a: Vec<Position> },
    FieldedNotA { preset: u8, position: Position, rated_a: Vec<Position> },
    /// Every position the player is rated B at.
    BRating { rated_b: Vec<Position> },
    /// The positions other than GK the player is rated A at.
    GkSecondA { rated_a: Vec<Position> },
    Age { age: u8, min: u8, max: u8 },
    Weight { weight: u8, min: u8, max: u8 },
    PositionRange,
    PlayingStyleRange,
    /// The heights the tiers' `when`s name (`height_min`/`height_max`, an `any` list's
    /// entries included), each an inclusive range; empty when the player's height is among
    /// them, the cause then being another condition.
    NoTier { heights: Vec<(u8, u8)> },
    Stat { stat: Stat, value: u8, target: u8, at_most: bool },
    Form { value: u8, target: u8 },
    InjuryResistance { value: u8, max: u8 },
    WeakFootUsage { value: u8, max: u8 },
    WeakFootAccuracy { value: u8, max: u8 },
    /// The name's leading colour code against the tier's; `expected: None` is a tier that
    /// sets none, met by a name with no colour code the tiers set.
    NameColor { color: Option<String>, expected: Option<String> },
    /// `limit` below zero: more extra A positions than cards (see "The card economy").
    Cards { paid: u8, limit: i8 },
    MinCards { paid: u8, min: u8 },
    SkillCardCap { count: u8, max: u8 },
    ComStyleCap { count: u8, max: u8 },
    RequiredCard { card: Card },
    APositions { count: u8, max: u8, rated_a: Vec<Position> },
    /// `fits`: the heights the player could take, each an inclusive range, his height
    /// allowance added: the brackets of the team's height system whose quota is not met yet
    /// (`room: true`); when none has room, every bracket whose quota is above 0 (a quota of
    /// 0 takes no player); under an `extra_height` system, one range from the lowest quota
    /// bracket to `max` (`room: false` both).
    Height { height: u8, fits: Vec<(u8, u8)>, room: bool },
    TierCount { tier: TierId, count: u8, expected: u8 },
    BracketQuota { system: SystemId, bracket: BracketId, count: u8, quota: u8 },
    Captain,
    Goalkeepers { count: u8, min: u8 },
    /// `wanted`: the value the entry sets for the setting.
    TeamSetting { label: String, setting: String, wanted: bool },
    FormationShape { preset: u8, formation: u8, shape: [u8; 3] },
    Special { label: String, text: String },
    Custom { text: String },
    /// A suggestion: an allowance listed in `suggestions` the player does not use up.
    Unused { allowance: String, used: u8, allowed: u8 },
    /// A suggestion under `choices`: a special's choice the player takes no option of.
    UnusedChoice { special: String },
}
```

These are findings (`AGENTS.md` "Fundamental concepts"): each tool maps a kind to its message
catalog; only `Special` and `Custom` carry text, the ruleset author's.

**Hints at a valid value.** A violation whose sentence would not otherwise name a value to
reach carries one, computed by the logic so every surface (Live AATF's rows, the marks'
tooltips, Multi-team AATF's copy, `check-aatf`'s fields) shows the same: most kinds already
name theirs (a range, a target, a limit, a count); `Height`, `NoTier`, `RegisteredNotA`,
`FieldedNotA`, `APositions`, `GkSecondA`, `BRating` and `TeamSetting` carry the fields above.
Each hint reads what the check has already read, never a search for a value that passes every
rule (each candidate height run through the whole check would be the expensive part); a
special can therefore still forbid a height the hint names (Autumn 26's goalkeepers between
181 and 188 must be 189). `check_team` reads every player before checking any, so the
brackets' room is counted once and handed to the per-player checks; `card_summary`, which
walks every position's rating, collects the A and B lists. The sentences (`team_widgets`'
`violations.rs` and the drafts' `violationSentence`): `height 183 takes no bracket (180 or 185
have room)`, `(up to 175, 180 or 185)` without room, `(175 to 210)` under a pool; `fits no
tier (the tiers take 180, 185, 190 or 195)`; `the registered position is not rated A (rated A
at CF and SS)`, `(no position rated A)` when none; the same list after FieldedNotA's and
APositions' sentences; `a second A rating at GK (also A at CB)`; `a B rating where the ruleset
refuses one (B at LB and RB)`; `team setting No man marking: man marking should be off`, the
setting in words. A range of one height prints once, one from the bottom `up to` its top. No
hint for PositionRange and PlayingStyleRange (the editor's controls offer only valid values),
Captain, FormationShape (any other shape is valid), Special and Custom (the ruleset author's
words) and UnusedChoice (naming a choice's options would need a sentence for every kind of
effect). `check-aatf` prints a list field as it prints `MedalSystem`'s systems, `[a, b]`, a
height range as `low-high` (`fits=[0-175, 180-180]`, 0 being an `up to` range's bottom), so
its line keeps one comma-separated field per key and a script reading it needs no second rule.

The map names the variant in `kind` (`"Stat"`) and carries its fields under their names: closed
sets as the labels the file uses (`stat: "speed"`, `position: "CB"`, `card: "malicia"`), ids as
their strings, `shape` as an array of three. `slot` is the roster slot, absent or `()` for the
team; `severity` is `"error"`, `"warning"` or `"suggestion"`, absent meaning an error, so a
custom check can return `#{ kind: "Custom", text: "…" }`. A map the host cannot convert (an
unknown kind, a missing or extra key, a number past its field's range, an unknown label) fails
the check with the map's index in the returned list. It is not dropped: it is a bug in the logic
or a custom check, and a check that skipped it would pass a team the file means to flag.

**Suggestions** need no authoring. Every allowance named in `suggestions` that a player does not
use up yields an `Unused` suggestion: paid cards under the limit, a group's free cards not all
held, a free card not held, fewer A positions than allowed, weak foot or injury resistance below
the maximum, a height below an `up_to` bracket's maximum. Under `choices`, a choice the player
takes no option of yields an `UnusedChoice` (a manlet at 77 who may carry 82). An ability stat
below its target is otherwise a manager's choice, not an allowance, and is never suggested.

## The host

`libs/aatf` compiles a file once per load, reads `RULESET` from the global scope, and calls the
three functions on demand. What a script sees is registered by the host and nothing else: a team
as a `TeamView` built from `pes_savefile`'s model (`TeamView::new(&EditFile, team_id)`) and its
roster as `PlayerView`s, every value on the game's own scale (form 1 to 8, injury resistance 1 to
3, weak foot 1 to 4, where the save stores each less one). They are views, not `PlayerEntry` and
`TeamEntry`, which are `pes_savefile`'s records. A team also comes from a Team TOML document
(`TeamView::from_team_toml`, for the Team creator's check before it writes and for the sample
team), of the version its `pes_version` names: the document is applied onto bare records, one
roster slot per `players` key, with each player's `appearance` table left out, because no
ruleset reads it and it applies only over a real player's face data. A bare record carries every
field its version has, at zero (decoded from a zeroed record of the version's layout), so a key
the document omits reads 0 like any other, never as a field the version lacks. The fields that
name a roster slot (set-piece takers, man marking, the starting eleven, the bench order) are
0xFF instead, the record's none, so an omitted taker reads as nobody rather than slot 0. With
`PlayerEntry::default()`'s absent fields, an omitted `catching` would switch its check off while
an omitted `speed` is reported.

The engine's setup, in outline (`engine.rs` holds every registration, most of them returning a
`Result` so a bad argument is a script error):

```rust
let mut engine = rhai::Engine::new_raw();        // no module resolver; print and debug go nowhere
engine.register_global_module(rhai::packages::StandardPackage::new().as_shared_module());
engine.register_type_with_name::<PlayerView>("Player")
      .register_get("height", |p: &mut PlayerView| i64::from(p.height))
      .register_fn("stat", |p: &mut PlayerView, name: &str| p.stat_by_name(name))
      .register_fn("rating_at", |p: &mut PlayerView, pos: &str| p.rating_at(pos));
engine.register_type_with_name::<TeamView>("Team")
      .register_get("players", |t: &mut TeamView| t.players.clone())
      .register_fn("fielded_only_at", |t: &mut TeamView, p: PlayerView, pos: &str| t.fielded_only_at(&p, pos));

engine.set_max_operations(MAX_OPERATIONS);
engine.set_max_call_levels(32);
engine.set_max_expr_depths(64, 64);              // rhai's debug and release defaults differ, and the interpreter nests past the debug limit
```

rhai is built with its `sync` feature, so a loaded `Rules` is `Send + Sync`: the whole-save
check runs its teams in parallel on one `Rules`, and a worker checks a team on the ruleset the
window loaded rather than loading its own. Measured 2026-10-22 (release, the official ruleset
on a PES 2017 save's 220 cup teams): one team's check takes about 56 ms, of which building the
team's view, copying it and calling into the script take under 1%; the rest is the script's
own work, `read_player`'s profile of each tier the most of it. `sync` costs about 3% per team;
the parallel check takes 1.8 s on 16 threads where one thread took 12.8 s. Since Rhai copies
every map a call passes, the logic computes once what does not change between the calls that
would repeat it: a choice's card summary, targets and weak-foot limits once per choice rather
than per option, and a special's `when` that names no tier and no bracket once per player
rather than per tier (`normalized` marks it `tier_free_when`). Measured 2026-10-25 (release,
DBC7 on the 12 census saves with the most findings, the two builds alternated): `check-aatf`
takes 0.85 of its earlier time, process start and the save's load included, and its output on
all 424 census saves is identical.

The script API, which `libs/aatf`'s crate doc lists too:

| Accessor | Returns |
|---|---|
| `player.slot`, `.age`, `.height`, `.weight` | the roster slot; age in years, height in cm, weight in kg |
| `player.form`, `.injury_resistance`, `.weak_foot_usage`, `.weak_foot_accuracy` | the game's scale |
| `player.strong_foot` | `"right"` or `"left"`, `()` for a byte past them |
| `player.position` | the registered position's label (`"CB"`); `()` past the labels |
| `player.playing_style` | the style's label; `()` when the version's table has no such style |
| `player.name_color` | the eight bytes after a leading `\x11c` (`"cc9900ff"`); `()` for a name without one |
| `player.cards` | the labels of the skills and COM styles held, skills first, each in table order |
| `player.stat(name)` | the ability stat's value; `()` on a version without it (`physical_contact` before PES 17); an unknown name is a script error |
| `player.rating_at(pos)` | `"A"`, `"B"` or `"C"` (`labels::RATINGS`; a stored 3, which no save holds, reads `"3"`); an unknown position is a script error |
| `team.players` | the roster's players in slot order, empty slots left out |
| `team.captain` | the captain's slot; `()` for none |
| `team.version` | the short number (`21`), as `TeamSettings::versions` spells it |
| `team.fielded_positions(player, preset)` | the labels of the positions the player is fielded at in preset 1 to 3's formations (a fluid preset's three, another's kick-off one), once each, in the position table's order (GK first, CF last); empty for a player outside the starting eleven; a preset outside 1 to 3 is a script error |
| `team.fielded_only_at(player, pos)` | whether every formation the game can field from the three presets fields the player at `pos`; for a player outside the starting eleven, whether `pos` is his registered position |
| `team.formation(preset, formation)` | the formation's eleven slots in order, each `#{ slot, position, x, y }`: the starting-eleven player's roster slot, the position's label and the coordinates as stored; presets and formations numbered as in `formation_shapes`, `()` for a formation the preset cannot field; a preset outside 1 to 3 is a script error |
| `team.formation_shapes()` | one map per formation the game can field, `#{ preset, formation, shape: [defenders, midfielders, forwards] }`, presets and formations numbered from 1 |
| `team.man_marking` | whether any of the three presets has a man-marking assignment set |
| `team.auto_substitution` | the stored setting (0 off); `()` on a version without it |
| `team.auto_offside_trap`, `.auto_preset_change`, `.auto_attack_defence_levels` | the flag; `()` on a version without it (attack/defence levels before PES 17) |

`MAX_OPERATIONS` is set from the measured cost of the heaviest embedded ruleset on a full team,
with at least a tenfold margin (10 million against about 790,000 measured for DBC7 on the sample
team, about 730,000 for the official ruleset; the test runs every embedded ruleset, holds each
under a tenth of the limit and the limit under 30 times the heaviest). Closed sets cross the
boundary as the labels the file uses (`"GK"`, `"A"`, `"malicia"`); the Rust side keeps its
enums and converts at the registration functions, nowhere else. Tactics questions are answered in Rust from `pes_savefile`'s model, so the starting-eleven
lookup and the formation count, both wrong upstream until `f5e7b3e`, live in one tested place: a
formation's slot *i* holds the starting eleven's *i*th player (`TeamTactics::starting_eleven`
holds roster slots; `aatf.cpp`'s `aatf_check_player_in_pos` reads them the same way), and a preset fields
its three formations when fluid and its kick-off formation otherwise (PES 15 has no fluid
formation). A data block alone (`Ruleset::from_data_block`) runs on the same engine. Adding an
accessor is an edit to this section, never a silent addition.

The team settings (`man_marking` and the auto flags) read `pes_savefile`'s team model, which
carries man marking. Edit mode offers no man marking on any version: a member sets it
in a match's pre-game game plan, saves that as a copy preset and loads the preset into the team
in Edit mode ("pre-made man marking", frowned upon, and the reason VTLEditor checks it). It lands
as eleven bytes per preset, one per own formation slot, each the marked opponent's formation slot
(its starting-eleven index, read against whichever team is played) or `0xFF` for none: in the
tactics record at +0x73, +0xED and +0x167 on PES 15 (read back in-game from a match's Mark
Settings, four assignments of a cup team matching its bytes), at +0x73, +0xEE and +0x169 on
PES 16 (VTLEditor's `pes16.cpp` reads them; measured on the 2024 Summer Cup save, whose team 710
has two set), at +0x95, +0x135 and +0x1D5 on PES 17 to 21 (the same shape after each preset's
modeled fields, set on a few teams of the cup saves of 17, 18, 19 and 21; read back in-game on
PES 21, a cup team's three preset-3 assignments matching its bytes). It is not an advanced instruction: Tight Marking is a stronger,
separate instruction. Auto substitution is off at 0 on PES 15, 16 and 21 (measured in-game: Very
Late 1, Flexible 2, and on PES 21 Very Early 3); on PES 15 it is the tactics record's +0x1FE,
followed by the offside trap (+0x1FF) and the automatic preset change (+0x200), each 0 or 1.
482 of the 483 teams of the PES 16 and 21 saves measured hold 0.

`apply_tier(player, tier, rules)` is the lib's one *writing* operation: it calls `tier_values`
and writes the result into the player (stats; form; injury resistance and weak foot at their
maximum), and the tier's `name_color` over the name's leading colour code; under a tier that
sets none, it removes a leading code that another tier sets and keeps any other. A name the
code pushes past its field fails the save's write, as any name too long for it does. The values
are resolved against what it writes, not what the player held: a special conditioned on a value
the tier sets (form, a stat, the name colour) would otherwise see the old one, so `apply_tier`
writes the name colour onto a copy first, then the values,
recomputes `tier_values` on it and repeats until they stop changing (three passes at most) before
committing; values still changing are an error and leave the player untouched. It backs the Save editor's quick actions (one button per tier, in list order; Phase 8) and
the Team creator's default stats, so neither tool can produce a player the checker then rejects.
AATF reports violations and never fixes them.

## Loading and validation

The embedded rulesets ("Embedded rulesets") load from the binary; the Save editor's
`aatf_ruleset` setting picks one of them or a rules file imported into the data folder, and
`check-aatf --rules` names a file. A file is validated on load (`Rules::load`, so `check-aatf`
refuses one that fails, and the Save editor refuses to import one):

1. It parses, and defines `RULESET`, `check_team`, `tier_values` and `card_limits` with their
   arities, and `custom_checks`, if defined, with one parameter (with another, `check_team`
   would skip it without a word).
2. `RULESET` deserializes into `Ruleset` of a known `schema`, and passes the rules under "The
   ruleset schema".
3. `check_team` runs without a script error on the sample team embedded in `libs/aatf` (a real,
   anonymized cup team: `sample/team.toml`, a PES 15 Team TOML document from a 2026 cup save,
   every player renamed and no appearance table). What it reports does not matter here; the
   fixtures check that.

A failure is a finding with the file's line and column for parse errors and the key path
(`tiers[2].free.tricks`) for schema errors, before any real check runs. The schema error's path
comes from `serde_path_to_error` wrapping the deserializer, not a wrapper of our own: its whole
job, and it depends only on `serde` and `itoa`.

**Logic identity.** `libs/aatf` embeds the current logic and the SHA-256 of every logic section
it has ever shipped, by version (`LOGIC_VERSIONS`), hashed with line endings normalized to LF
and trailing whitespace trimmed (the blank lines before the next banner are layout, not logic).
The logic section runs from the start of the LOGIC banner line to the start of the CUSTOM CHECKS
banner line, or to the end of a file without one; in an assembled file it is `logic/logic.rhai`
whole. A file without the LOGIC banner has a modified logic. The current logic is the table's
last entry: until the first release, its hash follows every edit of the logic; after it, a
changed logic is a new version, in the banner and the table, since a file in the wild carries
the released one. A
file's logic section is *current*, an *older official* version (the Ruleset editor offers to
update it), or *modified*: it still runs, and the Ruleset editor edits its data but says the
specials depend on that logic.

## Crate layout

```
crates/libs/aatf/
├── src/
│   ├── lib.rs          # the crate doc (the script API) and the re-exports
│   ├── rules.rs        # Rules (a loaded file): load steps 1-3, the official file, the host
│   │                   #   API: check_team, card_limits, apply_tier (over the file's
│   │                   #   tier_values, which the host keeps crate-private)
│   ├── violation.rs    # Violation, ViolationKind, CheckError; the conversion from the map
│   ├── values.rs       # TierValues and CardLimits from the script's maps, and the write of
│   │                   #   the tier values into a player record
│   ├── ruleset/        # Ruleset and its types, deserialization (mod.rs), the load-time
│   │                   #   rules (check.rs), their tests (tests/)
│   ├── team.rs         # TeamView and PlayerView from pes_savefile's model, the tactics answers
│   ├── engine.rs       # rhai::Engine setup: registered types and accessors, limits
│   ├── file.rs         # section split, logic identity (LOGIC_VERSIONS), assembling a file
│   │                   #   from a data block and the current logic
│   └── fixtures/       # (test-only) the known-legal and known-violating teams per ruleset
├── logic/logic.rhai    # the interpreter, current version
├── rulesets/           # data blocks only; a shipped file is data + logic.rhai
│   ├── autumn26.rhai
│   ├── autumn25.rhai
│   ├── spring26.rhai
│   ├── vgl25.rhai
│   ├── vgl26.rhai
│   ├── vgl27.rhai
│   ├── vtl11.rhai
│   └── vtl10.rhai
└── sample/             # team.toml, the sample team load step 3 runs (a Team TOML document),
                        # and README.md
```

The data blocks hold only `const RULESET = …;`, followed, in a ruleset with custom checks (VTL10
and VTL11), by its CUSTOM CHECKS section, banner included; a file is assembled from the data, the
current logic and that section (or the empty stub), so the logic exists once in the repository and
a custom check is kept beside the data it belongs to, not in a test.

---

## Why data plus an interpreter in the file

- **Not TOML parameters plus a logic file** (the first design). The two files version-skew: an
  invitational's parameters without a `bronze` key against logic that reads one. Tiers as serde
  fields resisted the fourth tier Autumn 26 added.
- **Not CEL** (one non-Turing-complete expression per rule). Autumn 26's specials need sequencing
  (the A-or-COM choice), a preset × formation traversal and a starting-eleven lookup; under CEL
  each is a new host helper, so a ruleset change would still need a Rust release.
- **Not hand-written Rhai logic per ruleset** (the 2026-09-19 design). Only a programmer could
  make a new ruleset, and invitationals are organized by people who are not.
- **Not a TOML ruleset interpreted by Rust.** A file's meaning would depend on the Studio build
  that runs it: the official logic changes most cups, so an invitational's rules would change
  under it when members update. Every new kind of rule would also need a Rust release.
- **Not visual programming** (blocks or a node graph generating Rhai). Checked in 2026-10:
  node-graph widgets exist for egui 0.36 (`nodez`, `egui-snarl`) but block editors do not, and
  reading hand-edited Rhai back into a graph needs a lossless Rhai parser nobody maintains.
  Every rule in seven years of history is a condition and an effect, which forms express. A graph
  can come later as a `GRAPH` block beside `RULESET`, without changing the format.

The costs accepted: `RULESET`'s syntax is denser than TOML (the Ruleset editor hides it, and
schema errors carry a key path); each file carries a copy of the logic (a few hundred lines);
and a rule the schema cannot express needs either the custom-checks hook or a schema extension.

## Coverage of a plain-language proposal

A member proposed a plain-language rules format without knowing Studio: tier and bracket
groups, player variables, `IF`/`ELSE`/`AND`/`OR`/`NOT` blocks with comparisons, and additive
`_mod` modifiers (the maintainer's copy is `.tmp/Clouds_proposal.txt`). The schema does not
adopt it, since the Ruleset editor's forms are simpler to fill in than any text to write, but
it expresses everything the proposal does. This table is that check, item by item; the fields
it found missing (`ranges`, `strong_foot`, `any`, the `form` and `injury_resistance` effects)
are in the schema above, and the two items left to custom checks are positional rules no
ruleset has had, whose shape (which slot, which bound) varies too much for one form.

| Proposal | Schema |
|---|---|
| Groups `regular`, `bronze`, `silver`, `gold` | tier ids, in `Condition::tiers` |
| `medal`, every medal tier | `tiers: ["gold", "silver", "bronze"]`: tiers are each ruleset's own, so there is no built-in group |
| `manlet` to `giga`; the number of brackets and of players in each | `Heights::brackets` and `HeightSystem::quotas`; `Condition::brackets` |
| `stats_<stat>` as a target | `Tier::rate`, `Tier::stats`, `Effects::stat_target` |
| `stats_<stat>` in a condition | `Condition::ranges` |
| `cards_<card>`, `com_<style>` in a condition | `Condition::holds` |
| `allowed_a_pos` | `Tier::a_positions` |
| `height` | `height_min` and `height_max`, inclusive, which write `<`, `<=`, `=`, `>=` and `>` |
| `weight`, `age` | `Universal::weight`, `Universal::age`; in a condition, `ranges` |
| `strong_foot` | `Condition::strong_foot` |
| `weak_foot_acc`, `weak_foot_use`, `injury_resist`, `form` | `Tier::weak_foot`, `injury_resistance`, `form`; in a condition, `ranges` |
| `registered_pos` | `Condition::positions` |
| `playing_style` | `Condition::playing_styles` |
| `preset<N>_<formation>_player_pos` | `Condition::fielded_only_at`, `Universal::fielded_at_a`; one preset's positions, a custom check over `team.fielded_positions` |
| `preset<N>_<formation>_player_x`, `_y` | a custom check over `team.formation` |
| the other tactics settings | `TeamSettings`; a setting it lacks, a custom check |
| `card_limit`, `card_limit_mod` | `Tier::cards`, `Effects::cards` |
| `stats_<stat>_mod`, `stats_global_mod` | `Effects::stat_bonus` by stat, and `all` |
| `allowed_a_pos_mod` | `Effects::a_positions` |
| `allowed_height_mod`, capped below the next bracket | `Effects::height_allowance`: subtracted before bracketing, it decides the bracket the player counts in, so no cap is needed |
| `allowed_weak_foot_acc_mod`, `allowed_weak_foot_use_mod`, `=` on them | `Effects::weak_foot`, which sets the limits; per-tier values by a `when` on tiers |
| `allowed_injury_resist_mod`, `allowed_form_mod` | `Effects::injury_resistance`, `Effects::form` |
| `IF … END` | a special's `when` and `then` |
| `AND` | the fields of one condition |
| `OR` within a group (`gold OR silver`) | a list (`tiers: ["gold", "silver"]`) |
| `OR` across fields | `Condition::any` |
| `NOT` | `Condition::not` |
| `ELSE` | a second special whose `when` is the first one's under `not` |
| `OR` between modifiers, tried as needed before the card limits | `Effects::choice` (the first option the player uses; "The card economy" follows it) |
| custom functions | the custom-checks section ("The rules file") |

---

## Development phase and verification

`libs/aatf` holds the `Ruleset` schema and the load-time rules, the interpreter, the host and its
accessors, `apply_tier`, logic identity, the Autumn 26, Autumn 25, Spring 26, VGL25, VGL26, VGL27,
VTL11 and VTL10 data blocks, the sample team and the fixtures (delivered in Phase 5, with the Save
editor's logic).

Verification:

- **Parity, official.** Each official file flags the same players with the same violation kinds
  as `aatf.cpp` at its own commit, on the cup saves played under it; each difference is one of the
  deliberate ones above or a fix. In `C:\Data\4cc\Saves` on the maintainer's machine (matched to
  rulesets by date): Autumn 25 on `2025 Autumn Babby Cup V2 Save.zip` and `2026 Winter Cup V2
  Save.rar`, Spring 26 on `2026 Spring Babby Cup Save V3.rar` and `2026 4chan Summer Cup Save
  V3.rar`; Autumn 26 on the same saves (findings compared, legality aside) until its own cup
  saves exist. Most teams there were legal under their own ruleset, so this is also the check that
  a real legal team reports nothing.
- **Parity, VGL26.** The same against `aatf_single_vgl` on the VGL26 cup save
  (`C:\Data\4cc\Saves\EDIT00000000_VGL26` on the maintainer's machine, 2026-10-06), every team;
  each difference is one recorded under "VGL26" (the rules page's stricter rules) or a fix.
- **Parity, VTL11.** Transcribed into a ruleset from the VTLEditor fork (its positional rule
  for 188 cm players as a custom check); the same against VTLEditor's `aatf_single` on the VTL11
  cup save (`C:\Data\4cc\Saves\EDIT00000000_VTL11`, 2026-10-06), every team. The PES 16 team
  settings get fixture teams of their own: by its size (11 MB) the save is a PES 21 one, where
  VTLEditor skips them.
- **Parity, VTL10.** The same against VTLEditor's `aatf_single` at `2cf222f` on the VTL10 cup save
  (`C:\Data\4cc\Saves\EDIT00000000_VTL10`, 2026-10-12, the size of VTL11's), every team.
- **Parity, VGL27.** The same against `vgl27`'s `aatf_single_vgl` (`VGL27.B`) on the VGL26 cup save's teams:
  built for VGL26, most fail, and what is compared is the findings, not legality. A VGL27 cup
  save replaces it when one exists.
- **Differences every legacy checker shows, on purpose.** A player of no tier is reported once,
  as `NoTier`, where the checkers go on to report his tier-free checks too (a weight, a required
  card): the member fixes the player's type first and the next check reports the rest, one more
  run rather than a missed error, since a team with an error gets a person's attention anyway.
  The checkers' "Illegal Ability scores" line (the overall rating off the tier's) has no kind:
  every stat off its target is reported itself. A tier over its count is reported on the team
  (`TierCount`), where the official checker also names one of the excess players, an arbitrary
  choice among them.
- Per ruleset, known-legal teams report nothing; each known-violating fixture reports exactly its
  violation. Every special of the official file is exercised by at least one fixture. VGL26's
  and VGL27's include a starting-eleven player fielded in preset 1 at a position he is not rated
  A at, one whose only such position is in preset 2 (legal), and a fluid preset 1 whose
  out-of-possession formation alone breaks the rule; VGL26's add a 3-4-3 shape in one formation
  of a fluid preset 3, and a 3-5-2 (legal). VGL25 has no cup save, so its
  fixtures are hand-built: manlets at each legal rating (77 and 82), one between them, and one at
  77 with `choices` suggested; a medal at 190; a gold at injury resistance 1.
- Suggestions: a fixture per allowance kind, and none for an allowance not listed in
  `suggestions`.
- Validation: one failing file per load-time rule, each reporting its key path or line.
- `apply_tier` then `check_team`: no violation of the applied tier's targets, for every tier of
  every ruleset above.
- A script that loops forever stops at the operation limit with a finding, not a hang.
