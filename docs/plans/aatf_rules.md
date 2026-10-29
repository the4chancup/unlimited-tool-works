# Unlimited Tool Works — AATF rules plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

What exists of this is in `spec/aatf_rules.md` "Unlimited Tool Works — AATF rules plan".

## The rules file

What exists of this is in `spec/aatf_rules.md` "The rules file".

## The ruleset schema

What exists of this is in `spec/aatf_rules.md` "The ruleset schema".

DBC7 ("DBC7" below) adds five things the schema cannot say (worklog 20.0d, 20.0e): a choice
between several medal counts, height systems told apart by their counts, a pool of centimetres
shared by the team, a band above a stat's target that warns rather than errs, and a cap on a
player's stats added up.

```rust
pub struct Ruleset {
    // ... every field in the spec, and:
    /// Alternative counts for some tiers, of which the team's must equal one (DBC7: GGG,
    /// GGSS, GSSSS, SSSSSS). Empty: each tier's own `count`.
    pub medal_systems: Vec<MedalSystem>,
}

pub struct Tier {
    // ... every field in the spec, and:
    /// Points an `at_most` ability stat may sit above its target with a warning, past which it
    /// is an error (DBC7: 7 for a regular, 5 for a silver). 0: any point above is an error.
    pub stat_warning: u8,
    /// The player's `at_most` ability stats, added up over the stats the version has, are at
    /// most the sum of their targets (DBC7's regulars and silvers), an error.
    pub total_within_targets: bool,
}

pub struct Effects {
    // ... every field in the spec, and:
    /// Replaces the tier's `stat_warning` (DBC7: 5 for a Manlet regular at 175, so his
    /// warning stops short of a silver's 88). Not a `choice` option's: it raises no limit.
    pub stat_warning: Option<PerTier<u8>>,
}

pub struct MedalSystem {
    pub label: String,
    /// A tier another system names and this one does not has 0 here.
    pub counts: BTreeMap<TierId, u8>,
}

pub struct HeightSystem {
    // ... every field in the spec, and:
    /// The system holds only when the team, read under it, meets its quotas (the pool's
    /// checks included), as well as `any_player`.
    pub by_quotas: bool,
    pub extra_height: Option<ExtraHeight>,
}

/// Centimetres the whole team may add above the bracket heights (DBC7's giga system: 50,
/// no player above 210).
pub struct ExtraHeight {
    pub total: u8,
    /// No player is taller, before any allowance.
    pub max: u8,
}
```

**Medal systems.** A tier a medal system names has no `count` and no `counts_as`, and no tier
`counts_as` it; the tier that is the rest of the squad takes what the team's system leaves.
The team's system is the first whose counts equal the team's counts of the named tiers (a tier a
system leaves out counting 0 in it, so a system with no counts is "no medal players"); when
none does, the team gets one `MedalSystem` violation and the rest of the squad is what the
team's named-tier players leave, so a wrong medal count is reported once, not again as the
regulars' `TierCount`. We do not give each tier a list of allowed counts instead: DBC7's
systems tie the counts together (two golds go with two silvers, never with four), which lists
per tier cannot say.

**Height systems by quotas.** "How a player is read" step 1 becomes: the first system whose
`any_player` holds and, with `by_quotas`, under which the team's quota and pool checks (step
4's part for heights) find nothing, every player read under that system (step 2). Reading the
team under each candidate is what lets an allowance or a tier that a special gives under one
system count toward that system's quotas. DBC7's organizer and its editor tell the systems
apart by how many players stand at each height; no condition on one player says that (a giga
team can hold a 195 cm player, so "any player at 195" would misread it as the Height bracket).

**The pool.** Under a system with `extra_height`, the quota brackets are filled tallest first:
players taken by height less allowance, highest first, each into the tallest quota bracket with
room whose height is at most his, the centimetres he stands above that height drawn from the
pool. A player above `max`, or below every quota bracket's height, is a `Height` violation and
takes no bracket, as a height no bracket takes does under any system; the pool overdrawn is one `ExtraHeight { used, total }` on the team; a bracket left short or
overfilled is `BracketQuota`, as elsewhere. Tallest first draws no more than any other order
that fills the quotas (what is drawn is the players' heights less the quotas' heights, the
same for every such order); the order decides only whether the quotas fill. A condition naming
a bracket still sees the bracket the player's own height takes (none, between two heights): the
assignment is the team's, made after every player is read, and DBC7 names no bracket in a
condition. We do not make the pool a custom check: a custom check runs after every player is
read, so a team whose giga players stand between bracket heights would already have failed the
built-in height checks, and the Manlet system's stat bonus needs the system that the pool
decides.

**Recognition before choices.** "How a player is read" step 2 compares the player's rating
with the tier's expected rating *before* its choices are resolved: an option only raises a
target, so a player who rates the tier without it is that tier. Read after them, DBC7's optional
Manlet bonus would make a silver at 92 (taking 4 of the 5) fall to regular, since the option,
used, raises his expected rating to 93; three Manlet teams of the maintainer's PES 2017 save
carry such a silver, and VGL25's optional manlet bonus had the same gap with no cup save to show
it. A mandatory effect still moves the threshold, as the Green silver giant's does.

New violation kinds: `MedalSystem { systems: Vec<String> }` (the team; every system's label, so
the message lists the counts the team may have),
`ExtraHeight { used: u16, total: u16 }` and `StatTotal { slot, total: u16, limit: u16 }`.

**The stat band.** A `Stat` violation carries a severity: a warning while the stat is at most
`stat_warning` above its target, an error past it, so a regular's 84 against 77 is one warning
and his 85 one error. The band is the tier's, or the last `stat_warning` effect's; it applies to
`at_most` ability stats only (a target that must be met exactly has no band), and recognition
does not see it (a regular's 84 still rates under a silver's 88). We do not give the band to
`at_most`'s list, per stat: DBC7's band differs by tier and by the Manlet bonus, not by stat.
The stat total compares the player's values with his targets after every effect and chosen
option, so a Manlet regular who takes the bonus is held to an all-82 sum.

New load-time rules: a medal system's label is not empty, it names existing tiers, and those
tiers have no `count` and no `counts_as` and no tier `counts_as` them; every system's counts plus
the fixed counts fit `squad_size`, and equal it when no tier is the rest of the squad. Every
height system but the last has a non-empty `any_player` or `by_quotas` with quotas (with none, no
quota fails); the last has neither
(else a team could fall through every system). A system with `extra_height` gives quotas only to
`exactly` brackets of one height, and its quotas add up to `squad_size`.

## Embedded rulesets

What exists of this is in `spec/aatf_rules.md` "Embedded rulesets".

## DBC7

A built-in ruleset in `rulesets/dbc7.rhai` (`spec/aatf_rules.md` "Embedded rulesets"), for Divegrass Before Christmas 7 (PES 2017, exports due 2026-11-18). Its
source of truth is the wiki page
`https://implyingrigged.info/wiki/Divegrass_Before_Christmas_7:_Melty_Under_The_Christmas_Tree`,
"Tactical export Rules" (read through a browser, since a plain fetch gets HTTP 403; unchanged
between 2026-10-19 and 2026-10-20); how its rules are checked is in the organizer's fork of the
official editor, DBC7-Editor (`RockAnon/DBC7-Editor`, `aatf.cpp` at `ee73917`), a reference where
the two differ.

| Feature | DBC7 |
|---|---|
| Tiers | gold 99 and silver 88 by the medal systems GGG, GGSS, GSSSS, SSSSSS (`medal_systems`); regular 77, the rest; no per-stat targets; every ability stat at most its target, a regular's up to 7 above it and a silver's up to 5 with a warning (`stat_warning`), and a regular's or silver's ability stats adding up to at most their targets' sum (`total_within_targets`) |
| Tier recognition | rating alone (the official rating stats) |
| Heights | four brackets of one height each (175, 180, 185, 195); three systems `by_quotas` in the organizer's order, Height (6, 6, 5, 6 from 195 down), Manlet (0, 10, 7, 6) and Universal (0, 23, 0, 0), then GigaHeight (0, 7, 10, 6 with `extra_height` 50, at most 210) for every other team; a registered goalkeeper at 189 counts as 185 (`height_allowance: 4`) in every system, after the pool under GigaHeight, as the page says |
| Specials | the GK allowance; a goalkeeper at 189 holds at most one A position (`max_a_positions: 1`, an error; that it is the registered one is the registered-position warning's); under Manlet, players at 175 may take +5 on every stat, regulars and silvers (the page's "silvers can also be buffed"; a gold is at 99 already), a one-option `choice` since the page allows the bonus rather than requiring it, and a Manlet regular's band narrowed to 5 (`stat_warning`), so his warning stops short of a silver's 88; the registered position not rated A, a warning (a `forbid` special over the 13 positions) |
| Cards, positions, form, weak foot, injury resistance | none: "No tactical export rules for anything else … go crazy". Paid cards 48 (the 41 skills of the version with the most, and 7 COM styles), 13 A positions, B ratings allowed, form, injury resistance and weak foot at most the game's maxima |
| Universal | none of the official file's checks; `max_skill_cards` 41, so no cap |

The page and the fork differ on seven points; the organizer chose one side for each
(2026-10-21), and the data block follows the choice:

- The fork's warning band is kept: a stat above its target by up to 7 (a regular) or 5 (a
  silver, a Manlet regular) is a warning, past that an error, where the page's "Non-Medals are
  77 rated, Silvers are 88 rated" reads as a cap. The organizer adds what the fork lacks: a
  regular's or a silver's ability stats, added up, may not exceed the sum of their targets (an
  all-77 or all-88 player's), an error, so the band trades a few points between stats and never
  raises a player overall. The sum runs over every ability stat the version has, a goalkeeping
  stat of an outfielder included, against each stat's own target, a Manlet bonus taken
  included: a Manlet regular who takes the +5 is held to an all-82 player's sum, not an
  all-77 one, which would leave his bonus nothing but points to trade; and no list of the
  stats that matter per role is needed.
- The registered position not rated A, which the fork errs on, is a warning: the page leaves
  it free, and the organizer wants the member alerted.
- A 189 cm goalkeeper with a second A position, which the page leaves free, is an error as in
  the fork: a team of registered goalkeepers at 189 would otherwise reach a full squad of
  "185s" and play its other positions through A ratings.
- What the page leaves free and the fork errs on stays free: an age outside 15 to 50, a weight
  outside the official range, no captain, no registered goalkeeper.

No card is capped: the game does not hold a player to ten skill cards in play (the organizer's
finding of 2026-10-21; the fork checks no card count either), so the block caps neither skill
cards nor paid cards below every card the game has.

Where the data still reads differently from the fork: the fork assigns GigaHeight's brackets
in roster order and puts a player with no room left in the lowest bracket his height reaches,
past its quota, drawing for him too; the pool places no one past a quota. On a team that fills
the quotas the two place the same counts and draw the same total; on one that cannot (a
stripped save's 23 placeholders at 180) the fork also reports the draw of the players it
overfilled, the pool only the short bracket.

## Coverage of the DBC7 rules

Each rule of the page's "Tactical export Rules", with the fork's `aatf_single` for how it is
checked:

| Rule (page; fork) | Schema |
|---|---|
| Height bracket: 6x 195, 6x 185, 5x 180, 6x 175 (`usingGreen`, counts equal) | `Heights::brackets` of one exact height each; a `HeightSystem` with `by_quotas` and `quotas` |
| Manlet bracket: 10x 185, 7x 180, 6x 175 (`usingRed`) | the same, the second system |
| Universal: 23x 185 (`usingCommie`) | the same, the third system |
| GigaHeight: 7x 185, 10x 180, 6x 175 and 50 cm to add to any player, at most 210 (`usingGiga`, every other team) | the last system, `quotas` and `extra_height` |
| The two-210 variant: 2x 210, 5x 185, 10x 180, 6x 175 | GigaHeight: two 185s raised 25 each |
| Manlet: stats for 175 cm players increased by 5, silvers too (`silverManletBonus`, gold 0) | a special: `when: #{ systems: ["manlet"], height_max: 175 }`, `choice: [#{ stat_bonus: … }]`, 5 for regular and silver |
| 185 cm goalkeepers may be 189, after GigaHeight's 50 cm (`heightTallGK`, a GK at 189 counted as 185 with no pool drawn) | a special: a registered GK at 189 gets `height_allowance: 4` |
| Non-medals 77, silvers 88, golds 99 (the rating's thresholds, each stat at most the target) | `Tier::rate`; recognition by rating; `StatRules::at_most` holding every ability stat |
| The organizer's choices (2026-10-21): a stat up to 7 above a regular's target or 5 above a silver's or a Manlet regular's is a warning (the fork's `warningRateBonus`) | `Tier::stat_warning`; a special's `stat_warning` for the Manlet regular |
| A regular's or silver's stats added up at most an all-77 or all-88 player's (the organizer's; the fork has none) | `Tier::total_within_targets` |
| The registered position rated A, a warning (the fork's error) | a special: `forbid` with `severity: "warning"`, over `any` of 13 positions each `not` `rated_a_at` itself |
| A goalkeeper at 189 has no second A position (`heightTallGK` with `countA > 1`) | a special: `max_a_positions: 1` |
| Medal systems GGG, GGSS, GSSSS, SSSSSS (the four count pairs) | `Ruleset::medal_systems` |
| No rule for anything else (cards, footing, formations) | tiers at the game's maxima, `universal` off, no card cap (the game holds none in play) |

No row is a custom check: the five new fields are the gaps, each one a form ("The editor" in
`plans/ruleset_editor.md`).

## The interpreter

What exists of this is in `spec/aatf_rules.md` "The interpreter".

Every surface that shows results (the Save editor's Live AATF, the Team creator's) has a "Show
suggestions" toggle, off by default, and lists them apart from errors and warnings, uncounted.

**The player's own tier.** The logic implements a fourth function the host calls,
`player_tier(player, team) -> tier id or ()`: the tier `check_team` reads the player as (step
1's height system for the team, then step 2's recognition), `()` when no tier's `when` holds
(`check_team`'s `NoTier`). The host's `Rules::player_tier(team, player)` returns the ruleset's
`Tier`, or none. It is behind the Save editor's one cards badge (`plans/save_editor.md`, the
Cards left bullet: "the tier his stats, registered position and height match under that
ruleset, as `check-aatf` recognizes it"), which then asks `card_limits` for that tier. It sits
in the file's logic, beside `check_team`'s own reading, for the reason `card_limits` does: a
widget that recognized tiers in Rust would be a second recognition that drifts from the
checks (recognition before choices, a height system told apart by its quotas, a special that
moves a threshold), and from a file whose logic is not the build's. Loading step 1 requires it
with its two parameters, like the other three. It lands before the 0.0.1 cut freezes logic
version 1, so no released file lacks it.

## The host

What exists of this is in `spec/aatf_rules.md` "The host".

**The roster order, checked for every ruleset.** By 4cc rule a team's roster slot `NN` holds
player `team id × 100 + NN`: the aesthetics export names players by that id and a Team TOML by
slot, and the two meet only through this rule. PES lets a roster be reordered (a real PES 16
texport of team 804 lists 80401, 80402, 80423, 80404, …, `spec/pes_savefile/operations.md`
"Texport"), so a manager can break it by mistake. The host checks it itself, in Rust before the
script runs, so no ruleset can leave it out and `logic.rhai` is untouched: each filled slot
whose player is not `team id × 100 + slot` is an error, `ViolationKind::RosterOrder { player }`
on that slot. The rule itself is `pes_savefile`'s (`TeamEntry::roster_order_breaks(&self) ->
Vec<(u8, u32)>`, the roster slots, 0-based as stored, and the player ids they hold; a method on
the team record rather than a function of the whole file, since every caller already holds
the team's entry and the rule reads nothing else), so the tools that guard the same
thing call one function: this check, Team TOML's export (a note per slot, since the file lands
its players by slot) and the Team compiler applying a Complete export's `tactics.toml`, which
refuses onto a local save whose team breaks the rule
(`plans/aesthetics_export/player_folders.md` "Complete export").

## Loading and validation

What exists of this is in `spec/aatf_rules.md` "Loading and validation".

The Ruleset editor validates a file before saving it (`plans/ruleset_editor.md`). A logic
change that needs a new data shape bumps `schema`; `libs/aatf` keeps a migration from every
earlier schema, run by the update.

**The first release, for logic identity, is the 0.0.1 preview** (`core/development_plan.md`
"Divegrass Before Christmas"): its Ruleset editor saves files that DBC's organizers and
managers keep, so from the cut on a changed logic is version 2, in the banner and in
`LOGIC_VERSIONS` beside version 1's frozen hash. Waiting for 0.1.0 instead would turn every
file saved with the preview into a *modified* one at the next logic edit: the editor would no
longer offer to update it, and would keep its old logic on every save. The hash is frozen only
on the maintainer's final confirmation, given once the UI is polished, never earlier: until
then a logic edit is still version 1.

## Crate layout

What exists of this is in `spec/aatf_rules.md` "Crate layout".

`src/writer/` (Phase 20): `Ruleset` → `RULESET` map literal, deterministic. A map key that
is not an identifier (an id is any non-empty string: `gold-medal`), or is one of Rhai's
keywords or reserved words (`in`, `for`), is written as a string literal, which Rhai's map
literal takes.

## Development phase and verification

What exists of this is in `spec/aatf_rules.md` "Development phase and verification".

The writer (`writer/`) and schema migrations arrive with the Ruleset editor in Phase 20.

DBC7's fields and data block (worklog 20.0d) are verified as the others are:

- **Parity, DBC7.** The data block against the fork's `aatf_single` (built headless, as for
  5.2f) on the maintainer's PES 2017 saves (`Documents\KONAMI\Pro Evolution Soccer 2017\save`
  and `C:\Data\4cc\Cups\Autumn24Q`), every team; built for other rules, most teams fail, and
  what is compared is the findings, each difference one listed under "DBC7" or a fix. Run at
  20.0d (2026-10-20, 220 teams each): every difference is one of those classes; the one fix it
  found is "Recognition before choices". The fork passes 29 and 36 teams, and the data block
  fails each of them on stats inside the fork's warning band alone.
- Fixtures: a legal team under each height system and each medal system; a Manlet regular and a
  Manlet silver at their raised targets, and the silver at 92 (still a silver); a goalkeeper at 189 under Universal and under GigaHeight;
  GigaHeight teams at exactly 50 cm, at 51, with a player at 211, with one at 174, with a 185
  cm player taking a 180 slot, and the page's two-210 variant; a medal count no system has; a
  stat one above its target.
- Validation: one failing file per new load-time rule.
