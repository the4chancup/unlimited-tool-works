# Unlimited Tool Works — Ruleset editor plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The Ruleset editor (`crates/tools/ruleset_editor`, tool id `ruleset-editor`, sidebar label
"Ruleset editor") lets anyone who can fill in a form build an AATF ruleset for their own
invitational: tiers, stat targets, heights, cards and conditional special rules, saved as a rules
file the Save editor and the Team creator use like the official one. The ruleset need not start
from the official rules or resemble them. The file format, the schema and the interpreter are
`libs/aatf`'s, specified in the [AATF rules plan](aatf_rules.md); this tool is a form over that
schema. Platform context is in the [core plan](core/README.md).

---

## Design principles

1. **Forms, not code.** Every field of the ruleset schema has a widget; nothing in it is reachable
   only by editing text. The user never sees Rhai unless the file has custom checks, which are
   shown read-only.
2. **The tool owns the data, Studio owns the logic, the author owns the custom checks.** Saving
   rewrites the `RULESET` block whole, writes the current logic, and copies the custom-checks
   section byte for byte, its trailing whitespace aside: the file's layout sets that (one
   newline ends each section), as it does for a kept logic section. Hand edits inside `RULESET`
   survive as values; their formatting and comments do not.
3. **An invalid ruleset is never written.** The load-time rules of "Loading and validation" run
   on every edit; Save stays available and, while the ruleset is invalid, lists what blocks it
   instead of writing.

---

## The editor

**Start page:** *New from template*, where the template is one of the embedded rulesets
(`spec/aatf_rules.md` "Embedded rulesets": the latest two of the 4cc, VGL and VTL, Autumn 26
first: Autumn 26, Spring 26, VGL27, VGL26, VTL11, VTL10), then DBC7, then **Blank**; *Random*,
in three flavors (see "Randomize"); and *Open file*. Recent files below, newest first.
DBC7 is a template while Divegrass Before Christmas is the Save editor's first test ground
(`core/development_plan.md` "Divegrass Before Christmas"): its organizers start from it (its
managers get the rules file in the 0.0.1 bundle, which ships before this tool). It leaves the
list when the cup ends, as a minor invitational's ruleset would never have joined it. **Blank** is a ruleset
named "New ruleset", squad 23, Autumn 26's rating and at-most stats, one tier (`regular`,
"Regular", the rest of the squad, rate 75, form 4, injury resistance 1, weak foot 2 and 2, 3
paid cards, 1 A position), no brackets, no specials, and the universal rules on: registered
position rated A, no B ratings, a GK A never a second A, a captain, at least one goalkeeper, at
most 10 skill cards.

A left rail lists the sections; each is a form, and a section with a validation error shows a
mark in the rail. The rules file's data block, as the editor would write it, shows in a pane
right of the form only while the **Show preview** toggle at the top right of the tool's bar is
on, off at every start: open beside the form it would leave the sections a third of Studio's
window, and the block is a check on the form, not where the work happens. Shown, its lines
the last edit changed are highlighted.

- **General**: name, event page, event logo, squad size, the rating stats and the at-most
  stats (stat checklists, with one line on what each list does). The event page is a text
  box (`https://` as its placeholder); the event logo shows the picture at the height the
  Save editor draws it (28, up to 84 wide), then `Choose PNG…` (a file dialog for PNGs) and
  `Remove`, or a weak `None` before them; a file that is no PNG or is over 64 KB is refused
  with a line beside the buttons saying which, the logo kept as it was. One weak line under
  the fields says where the logo shows and what a click on it does. The data block's text
  writes `page` after `name` and `logo` last; the preview shortens the logo's one long line
  to its first characters and its size, the file holding it whole. A page that is not an
  `http://` or `https://` address is one of General's problems.
- **Tiers**: a table, one row per tier, highest first, reordered by dragging, since the order is
  the recognition order and the quick-action buttons' order. Columns: label, id, count (a number
  or "the rest"), counts as, rate, form, injury resistance, weak foot, paid cards, minimum
  cards, A positions, name colour (none or a swatch), warning band (`stat_warning`: points
  above a stat's target that warn rather than err, 0 meaning none) and a "stats add up to at
  most the targets" checkbox (`total_within_targets`). Every number, here and in every
  section, draws its value as stored, never clamped to its control's range on sight (DBC7's 48
  paid cards, past a 0-20 range, were rewritten to 20 by drawing the section), and the paid and
  minimum cards run to 48, every card the game has. The id commits on Enter or focus loss, as a path row does, not per keystroke,
  and carries the tier's key in the medal systems from the id it had to the one committed, only
  while no other tier carries the old id and no system holds the new one: carried per
  keystroke, typing `gold2` over Silver passed through `gold`, the Gold tier's own id, and the
  next keystroke moved Gold's counts to `gold2`. Per row, an expander holds the per-stat
  targets (all ability stats, each an "own target" checkbox and its number: unticked, the stat
  follows the rate and has no entry; a number field cannot be left empty, which is how the
  draft says "the rate"), the free cards per group (one number per group, COM styles first: as
  columns they would widen the table past a laptop screen; 0 removes the group's entry, as an
  emptied draft field does) and the tier's conditions (the condition editor below). Below the table, the medal systems
  (`spec/aatf_rules.md` "The ruleset schema", `medal_systems`): one row per system, its label
  and a count per tier, a checkbox above each tier's column putting the tier in the systems; a
  tier in them shows "by medal system" as its count. The draft is
  `docs/prototypes/tools/ruleset_editor.js`.
- **Heights**: a "use height brackets" switch. On: the brackets in ascending order, each "up to
  N cm" or "exactly N, M cm"; the height systems in order, reordered by dragging (the first
  that holds is the team's), each "when any player is …" (the
  condition editor, limited to height and position, with "any of" and "except when" over
  them) and a "when the team meets its quotas"
  checkbox (`by_quotas`), with a quota per bracket and an optional extra-height pool (the
  team's total centimetres and the tallest a player may be), the last one "otherwise". Off:
  heights are left to tier conditions and specials, as in VGL26.
- **Cards**: the card groups, each a name and a checkbox grid of the 41 skills and 7 COM styles
  (labels as the Save editor shows them); all COM styles are also the built-in `com` group.
- **Universal rules**: checkboxes and number fields for `universal`, and below them the team
  settings: one row per entry, its PES versions, the value each tactics setting must have and
  the formation shapes it bans (defenders-midfielders-forwards, `3-4-3`).
- **Specials**: a list of cards, reordered by dragging (order matters for replacing effects), each
  with a label, a **When** row and a **Then** row:
  - *When* is the condition editor: tiers, registered positions, playing styles, height systems
    and brackets as multi-select chips; a height range; captain yes / no / either; "holds any of"
    (cards); "rated A at" and "rated B at" (positions); "name colour" (swatches of the codes);
    "fielded only at"; value ranges (a stat, age, weight, form, injury resistance or weak foot,
    each with a from-to pair); stronger foot either / right / left; "any of" (a list of
    sub-conditions, one of which must hold); and an "except when" sub-condition (`not`).
  - *Then* is a list of effects, added from a menu that names them in words, one entry per
    effect of the schema ("Add to every stat" and "Add to one stat", `stat_bonus`'s two keys;
    "Set one stat's target"; "Extra paid cards"; "Free cards of a group", `free`; "Free card",
    `free_cards`; "Required card"; "Extra A positions"; "At most N A positions"; "Change form";
    "Change injury resistance"; "Weak-foot limits"; "Height allowance"; "Warning band";
    "Forbid"). A
    numeric effect the schema gives per tier (`PerTier`: the stat bonuses and targets, extra
    paid cards, free cards of a group, extra A positions, form, injury resistance, the warning
    band) has a "same for every tier / per tier" switch; per tier shows one field per tier. "At
    most N A positions" and "Height allowance" are one number for every player they apply to.
    *One of* adds a choice: ordered options, each a list of effects, with the rule spelled out
    under it ("the first option the player needs is given"); a choice of one option reads *May
    get* ("players matching this may take it").
  - A severity selector appears when the effects include Forbid or Required card.
- **Suggestions**: a checkbox per allowance (paid cards, A positions, weak foot, injury
  resistance, height, free cards, choices not taken, and one per card group, COM included).
- **Custom checks**: present only when the file has them (the empty stub `fn custom_checks(team)
  { [] }` that every assembled file without checks carries counts as none, and is kept as it
  is on save); the code, read-only, with a line saying
  it runs after the ruleset and is kept as is.
- **Summary**: the ruleset as readable rules text ("Gold: 1 per team, every stat 99, form 8, …";
  the medal systems, heights and universal rules; each card group with its cards; each team
  settings entry; each special as a sentence from its When and Then), generated from the data. **Copy** puts it
  on the clipboard as Markdown, for posting an invitational's rules; two summaries diffed in any
  text tool are the comparison between two rulesets.

Undo and redo work on whole snapshots of the ruleset and its logic status (a ruleset is a few
kilobytes): **Update logic** is an edit like a form's, and a snapshot of the ruleset alone would
make its undo restore an identical ruleset and leave the new logic in place. Closing with
unsaved changes prompts.

## Randomize

**Randomize ▾** (toolbar, and *Random* on the start page) generates a whole new ruleset in one
of three presets, for an organizer who wants a starting point or a cup with a twist:

| | Sensible | Weird | Crazy |
|---|---|---|---|
| Reads like | a real cup's rules with different numbers | a themed invitational | anything the schema allows |
| Tiers | 3–5, top rate 95–99, each step down 4–8, regular 72–80, named Gold / Silver / Bronze / Regular / Goalkeeper | 2–6, per-stat targets on 2–6 stats (an attacking tier with finishing 99 and defensive prowess 50), named from a word list | 2–8, any stat target 40–99, per-stat targets anywhere |
| Counts | 1–3 per medal tier, the rest regular | 0–8 per tier | any that fit the squad |
| Heights | the official bracket layout or one height per tier (VGL26), thresholds moved by up to 5 cm | brackets at random cut points, unusual quotas (five giants, no tall players) | any heights in the game's range, one to three systems |
| Rating and at-most stats | the official sets | the official rating set less 0–3 stats | random non-empty sets |
| Specials | 0–2 from the familiar kinds (manlet bonus, giant penalty, a free card, the captain's card) | 2–5, every effect kind except choices, conditions on positions and heights | 4–10, every effect kind, choices, `not` conditions, forbids, required cards |

All three keep the universal rules on, except that Crazy may switch off any rule but the captain
and the goalkeeper; the squad is always 23.

**Every generated ruleset admits a legal team.** The generator draws a **witness** team first:
23 players with registered positions (at least the goalkeepers the ruleset requires) and a tier
each, then one set of heights per height system, chosen so that system is the one picked. The
tier counts and each system's bracket quotas are then *read off* the witnesses rather than
drawn. The specials come next. Each witness is then completed through the ruleset itself:
`apply_tier` for stats, form and limits, the required cards and nothing else, A only in the
registered position, no tactics (so `fielded_only_at` reads the registered position and
`fielded_at_a` and `forbidden_shapes` find nothing to check). Finally
`check_team` runs on it. A witness with an error redraws the specials; after a bounded number of
attempts the whole ruleset is redrawn, continuing the same random sequence, so a seed still names
exactly one result. A ruleset that no team could satisfy is never offered.

**Seeds.** Every result has a seed, shown beside the preset (`Weird #48213`, also the ruleset's
name until the user changes it); typing a seed reproduces the ruleset exactly, and **Again** draws
a new seed with the same preset. The generator uses its own SplitMix64 rather than a crate's
generator, so a seed gives the same ruleset in every Studio release (`rand`'s `StdRng` does not
promise that). A press draws its seed from the clock; on `wasm32`, where `SystemTime` is
unavailable, from eframe's input time.

A random ruleset opens as a new, unsaved document (the current one prompting for unsaved changes,
as New does); it has no custom checks.

## Opening and saving

**Open** runs `libs/aatf`'s validation, then reads the logic section's identity:

- *Current*: opens normally.
- *Older official version*: opens, with a banner offering **Update logic**, which migrates
  `RULESET` to the current schema if its version is older and replaces the logic section. The
  file keeps working unchanged until the user accepts.
- *Modified*: opens, with a banner saying the file's logic is not Studio's, so its specials may
  not behave as the forms describe; saving keeps that logic rather than replacing it.
- *Unreadable* (a parse or schema error, or a broken load rule): not opened in the forms; the
  findings are listed with their line or key path, for fixing in a text editor. A file that
  reads but breaks a load rule (two tiers sharing an id, say) is refused like the other two,
  not opened with rail marks: it is the file the Save editor refuses too, so both tools agree
  on which files are rules files, and the forms never start from a ruleset no tool can load.

**Save** and **Save as** write the file: the header comment, the `RULESET` block from
`libs/aatf`'s writer, the logic section (current, or the file's own when modified), and the
custom-checks section as read. Before writing, the sample team is checked (validation step 3),
so a file that would fail in the Save editor is never written. The file is written to a
temporary sibling and renamed into place.

The Save editor and the Team creator pick a ruleset through their own settings (the rules path);
this tool does not write other tools' settings.

---

## CLI

```
UTW ruleset-editor validate rules.4ccrules      # load-time validation; findings to stdout, exit 1 if any
UTW ruleset-editor update-logic rules.4ccrules  # migrate RULESET and replace an older official logic
UTW ruleset-editor summary rules.4ccrules [-o rules.md]
UTW ruleset-editor random --preset sensible|weird|crazy [--seed N] -o rules.4ccrules
```

`update-logic` refuses a file whose logic is modified (it would discard someone's code) and leaves
a current file untouched. `random` prints the seed it used. The writing commands keep a `.bak` of
a file they replace.

## Settings

- `last_folder`: where Open and Save as start.
- `recent_files`: the start page's recent files, newest first, at most 10; a file opened or
  saved moves to the front, and one that no longer exists is dropped when the list is shown.

---

## Crate layout

Standard tool-crate skeleton (core plan, "Tool crates"), elaborated below it:

```
crates/tools/ruleset_editor/src/
├── lib.rs              # Tool (StudioTool impl) — wiring only
├── settings.rs
├── cli.rs              # validate / update-logic / summary / random
├── messages.rs         # validation findings, open/save failures, logic-status banners
├── document.rs         # Document: path, Ruleset, custom-checks text, logic status, undo
│                       #   snapshots, dirty flag; egui-free, what the CLI drives
├── summary.rs          # Ruleset → rules text (Markdown)
├── random/             # Randomize: SplitMix64, the three presets' parameters, witness teams,
│                       #   draw → verify → redraw; egui-free
└── view/
    ├── mod.rs          # start page, section rail, open/save, banners
    ├── condition.rs    # the condition editor (tier conditions, specials, systems)
    ├── effects.rs      # the effects editor: per-tier values, choices
    └── sections/       # one file per section
```

Placement rules: reading, validating, writing and migrating a rules file are `libs/aatf`'s
(`file.rs`, `ruleset.rs`, `writer/`); this crate never parses Rhai or formats a `RULESET` block.
`document.rs`, `summary.rs` and `random/` are egui-free, so the CLI and the GUI share them.

---

## Acceptance

Format and rules: `CONTRIBUTING.md` "Testing". IDs are never reused; later phases append.

**Phase 20 scope.** Phase 20 delivers the whole tool: the view, opening and saving, the summary,
Randomize and the CLI. What a scenario observes is what a frame draws, through the headless
harness the Save editor's view tests use (the frame's AccessKit nodes, clicks, drags and key
events as input frames; a highlight or a mark's colour, which no AccessKit node carries, read
back from the view state the frame paints from; the clipboard from the frame's platform
output), the file written, and the CLI's stdout, exit code and files; `manual` marks only what
needs a display or a native dialog. Rules files are the embedded rulesets (`libs/aatf`'s
`rulesets/`, assembled with the current logic), copied under the test's own folder; "a team
passes" or "fails" means `Rules::check_team` on a fixture team of `libs/aatf`'s fixtures (or
`check-aatf` on the save it comes from), what a player pays for `Rules::card_limits` on him,
and a random ruleset's witness teams are the ones the generator returns with it, one per
height system (one in all for a ruleset without height systems). The writer's
round trips and the load rules are `libs/aatf`'s own tests, not restated here.

**The editor**

```
RE-GUI-01  GIVEN the tool opened with no document
           WHEN the start page is drawn
           THEN it offers New from template with Autumn 26, Spring 26, VGL27, VGL26, VTL11,
                VTL10, DBC7 and Blank in that order; Random with Sensible, Weird and Crazy; Open
                file; and the recent files, newest first; GIVEN two files opened in turn, or
                one opened and one saved, and the tool restarted THEN the later heads the
                recent files and the earlier follows; GIVEN a listed file deleted THEN it is
                no longer listed
RE-GUI-02  GIVEN the start page
           WHEN a template is picked (each of the seven, and Blank)
           THEN an unsaved document of the template's name (Blank's is "New ruleset") opens on
                General, the rail lists General, Tiers, Heights, Cards, Universal rules,
                Specials, Suggestions, Custom checks (VTL11 and VTL10 alone: the others' files
                carry only the empty stub) and Summary in that order, no rail entry carries a
                problem mark, and Save as writes a file that check-aatf loads and whose ruleset
                equals the template's
RE-GUI-03  GIVEN Autumn 26 open, Show preview on
           WHEN the Silver tier's rate is changed from 92 to 90 in the Tiers table
           THEN the preview's silver line reads rate 90 and is highlighted, the lines no edit
                changed are not, and the document is marked unsaved; GIVEN the tool restarted
                THEN Show preview is off
RE-GUI-04  GIVEN Autumn 26 open
           WHEN the Bronze row is dragged above the Silver row
           THEN the Tiers table lists bronze before silver, and the saved file's tiers are
                gold, bronze, silver, goalkeeper, regular; GIVEN DBC7 WHEN its Manlet height
                system is dragged above Height THEN the saved file's systems are manlet,
                height, universal, giga
RE-GUI-05  GIVEN Autumn 26 open
           WHEN the Silver tier's id is changed to "gold" (two tiers sharing an id)
           THEN the Tiers rail entry carries a problem mark; Save writes no file and lists what
                blocks it, the shared id among it; Undo clears the mark
RE-GUI-06  GIVEN DBC7 open
           WHEN the Gold tier's medal-systems checkbox is cleared
           THEN Gold's count shows "the rest" with its checkbox instead of "by medal system",
                the systems' Gold cells show "—" under the cleared checkbox, no system counts
                gold (GGG now counts none: a team with no silver), and the Tiers rail entry
                carries a problem mark (two tiers are the rest of the squad); WHEN the checkbox
                is ticked again THEN Gold
                is by medal system once more and each system counts gold 0, the marks gone;
                Undo instead restores the counts it had
RE-GUI-07  GIVEN Autumn 26 open
           WHEN "use height brackets" is switched off
           THEN the Heights section shows no bracket or system rows, every condition naming
                them is listed as a problem until removed (the saved file's heights then empty),
                and Undo restores them
RE-GUI-08  GIVEN Blank open
           WHEN a card group "tricks" is added in Cards with Double Touch and Heel Trick checked,
                and the Regular tier is given 1 free card of it in its expander
           THEN the saved file's card_groups holds tricks with those two skills and the regular
                tier's free cards hold tricks 1; a regular holding both skills and no other
                card pays for one; the Suggestions section gains a checkbox for tricks
RE-GUI-09  GIVEN Blank open
           WHEN a special is added with When "height up to 175" and Then "Add to every stat" 5
           THEN the saved file's special has that condition and a stat bonus of 5 on every
                stat for every tier; WHEN its "same for every tier" switch is set to per tier
                and only Regular is given 5 THEN the bonus is by tier, Regular 5
RE-GUI-10  GIVEN a special in Blank
           WHEN "One of" is added with two options (one extra paid card; one extra A position)
           THEN the rule under it reads that the first option the player needs is given, and
                the saved file's special holds a choice of those two options in that order;
                with one option only, the choice reads "May get"
RE-GUI-11  GIVEN a special whose effects are "Add to every stat" only
           WHEN Forbid, or Required card, is added to its effects
           THEN a severity selector appears (error, warning, suggestion), and is gone again
                once that effect is removed
RE-GUI-12  GIVEN Autumn 26 open
           WHEN the special "Medals cannot be goalkeepers" is dragged to the end of the list
           THEN the saved file's specials end with it, the others in their order
RE-GUI-13  GIVEN VTL11 open
           WHEN the Custom checks section is drawn
           THEN it shows the file's custom-checks code read-only, with the line saying it runs
                after the ruleset and is kept as it is
RE-GUI-14  GIVEN two edits made in order
           WHEN Undo is invoked (button or Ctrl+Z), then Redo (button or Ctrl+Y)
           THEN the second edit is undone and the first kept, then the second is redone
RE-GUI-15  GIVEN a document with unsaved changes
           WHEN the window is closed, another document opened or a template picked
           THEN a prompt offers to save, discard or cancel; cancel keeps the document as it
                was; save writes it first (Save as for one never saved); discard goes on
RE-GUI-16  manual  GIVEN Blank open
                   WHEN VGL26's data is rebuilt through the forms alone (no text edited)
                   THEN the saved file validates, its ruleset equals the embedded VGL26's, and
                        VGL26's fixture teams pass and fail as under the embedded file
RE-GUI-17  GIVEN a ruleset setting every field of the schema but `schema` itself, which has no
           widget (DBC7's warning band, stat total, quota systems and GigaHeight pool among
           them, plus team settings with a banned shape, suggestions, the rating and at-most
           stats, which DBC7 does not set)
           WHEN each field is changed through its widget, one field at a time, and saved; and,
                one at a time, an item added to and one removed from each list and map the
                schema holds (tiers, medal systems, brackets, height systems, card groups and
                their cards, specials, a special's effects, a choice's options, a condition's
                any-cases and chips, quotas, team settings and their shapes, among them)
           THEN each saved file's ruleset differs from the opened one in that field alone, or by
                that one item, apart from the references to an id the tool renames with it (a
                tier's in the medal systems, a card group's in free cards and suggestions); a
                field the load rules tie to others (the squad size to a pool system's quotas, a
                quota to its system's sum) is changed with the companion edits that keep the
                ruleset valid, and the file differs by those alone
RE-GUI-18  GIVEN Autumn 26 open
           WHEN the condition editor's add menu is opened in a tier's conditions, in a height
                system's "when any player is …", and in a special's When
           THEN the tier's menu offers no tiers; the height system's offers heights,
                positions, any of and except when, the last two's own menus the same four;
                the special's offers every kind of condition; the last height
                system (Red) reads "otherwise" and offers no condition; WHEN a special's Then
                menu is opened THEN it offers the fifteen effects "The editor" lists and One
                of, and
                inside a choice's option only those an option may hold (the stat bonuses,
                extra paid cards, free cards of a group, a free card, extra A positions, at
                most N A positions, weak-foot limits)
RE-GUI-19  GIVEN a ruleset open with no logo
           WHEN a PNG of 2 KB is chosen in General's Event logo
           THEN the field shows it 28 points high with Remove beside Choose PNG…; WHEN a
                JPEG, or a PNG over 64 KB, is chosen THEN a line beside the buttons says
                why and the logo is unchanged; WHEN the Event page holds `www.example`
                THEN General lists the problem
```

**Opening and saving**

```
RE-FILE-01  GIVEN a rules file Save wrote with the current logic
            WHEN it is opened
            THEN it opens in the forms with no banner
RE-FILE-02  GIVEN an Autumn 26 file whose logic section has one comment's text changed by hand
            (the file still loading)
            WHEN it is opened, an edit made and Save pressed
            THEN a banner says the file's logic is not Studio's and its specials may not behave
                 as the forms describe; the saved file's logic section is the file's own, byte
                 for byte (its trailing whitespace, which is layout, aside), and its RULESET
                 holds the edit; the saved file reopened shows the banner again
RE-FILE-03  GIVEN a file with a parse error at line 12, one whose tiers[2].free.tricks is the
            string "two", and one whose second tier has the first tier's id
            WHEN any of them is opened
            THEN it is not opened in the forms; the findings are listed, the first file's with its
                 line and column, the second's and the third's with their key paths
RE-FILE-04  GIVEN VTL11 assembled as a file, with its custom checks
            WHEN it is opened, its Silver tier's label changed, and saved
            THEN the saved file is the header line, the RULESET block holding the new label, the
                 current logic, and the custom-checks section, that section byte for byte as it
                 was read (its trailing whitespace, which is layout, aside); VTL11's fixture
                 teams pass and fail as under the original file
RE-FILE-05  GIVEN dbc7.4ccrules assembled as a file, with the comments inside its RULESET
            WHEN it is opened and saved with no edit (Save is available on an unedited
                 document)
            THEN the saved RULESET holds the same values (the ruleset reads back equal) and none
                 of the comments
RE-FILE-06  GIVEN a file of five tiers whose custom check reads the fifth tier
            (`global::RULESET.tiers[4]`), a tier whose deletion breaks no load rule (it has a
            count, and no condition or system names it)
            WHEN it is opened, the fifth tier deleted and Save pressed
            THEN nothing is written: the sample team's script error is listed as what blocks it
RE-FILE-07  GIVEN an existing rules file
            WHEN Save replaces it
            THEN no other file is left beside it (the temporary sibling renamed into place);
                 GIVEN the existing file read-only THEN Save reports the failure, the file is
                 byte for byte as it was, no other file is left beside it, and the document
                 stays unsaved
RE-FILE-08  manual  GIVEN a file opened from one folder
                    WHEN Open or Save as is used again, also after a restart
                    THEN the dialog starts in that folder
RE-FILE-09  GIVEN a ruleset with an event page and a logo
            WHEN it is saved and opened again
            THEN both are as they were, `page` written after `name` and `logo` as the data
                 block's last key, and the logic's identity is the one it had without them
```

The older-official path (the banner offering **Update logic**, and `update-logic` migrating a
file) has no scenario in Phase 20: logic version 1 is the only one, so no real file of an older
official logic exists. Its first is a file the 0.0.1 preview saves, once a later logic change
makes version 2 (`aatf_rules.md` "Loading and validation"); the scenarios are written then,
with such a file as their fixture. Phase 20 builds the path; `libs/aatf`'s own tests prove the
identity and the section's replacement against a versions table holding a second entry.

**Summary**

```
RE-SUM-01  GIVEN Autumn 26 open
           WHEN the Summary section is drawn
           THEN it has one line per tier, highest first, naming its count, rate and the targets
                it sets (the draft's "- **Gold**: 1 per team; every stat 99; form 8, …"), the
                height brackets and systems, the universal rules, and one sentence per special
                built from its When and Then; Copy puts the same text on the clipboard as
                Markdown; GIVEN DBC7 THEN it also has the medal systems with each one's counts,
                Gold and Silver counted "by medal system", the regular's and silver's warning
                bands and stat totals, each height system's quotas, and GigaHeight's pool;
                GIVEN VGL26 THEN it also has its card group tricks with its twelve cards, and
                its team settings: no man marking, the support settings off, no 3-4-3
RE-SUM-02  GIVEN two rulesets differing in one tier's rate (Autumn 26, and Autumn 26 with
           Silver at 90)
           WHEN their summaries are compared line by line
           THEN exactly the Silver line differs
```

**Randomize**

```
RE-RND-01  GIVEN the start page
           WHEN Weird is drawn, then drawn again by typing the seed the first draw showed
           THEN both are the same ruleset, written to identical bytes, named "Weird #<seed>";
                GIVEN Sensible, Weird and Crazy at seed 48213 THEN each written ruleset equals
                the file pinned for it when the generator shipped (a seed names the same ruleset
                in every release)
RE-RND-02  GIVEN each preset and seeds 1 to 1000
           WHEN a ruleset is drawn for each
           THEN each validates, and for each of its height systems the witness team the
                generator returns for that system is read under it and passes
                `Rules::check_team` with no error; a ruleset with no height system (Sensible's
                one height per tier) has one witness, which passes the same way
RE-RND-03  GIVEN Sensible and seeds 1 to 1000
           WHEN a ruleset is drawn for each
           THEN each stays inside every bound of the Randomize table's Sensible column: 3 to 5
                tiers named from Gold, Silver, Bronze, Regular and Goalkeeper, a top rate of 95
                to 99, steps of 4 to 8 between rates, a regular rate of 72 to 80, 1 to 3 players
                per medal tier and the rest regular, the official bracket layout or one height
                per tier with each threshold within 5 cm of its source, Autumn 26's rating and
                at-most stats, 0 to 2 specials of the familiar kinds, squad 23, and the
                universal rules on (as Blank's)
RE-RND-04  GIVEN a document with unsaved changes
           WHEN Randomize ▾ Weird is chosen
           THEN the unsaved-changes prompt comes first; once it is answered with discard (or
                save), a new unsaved document opens, with no custom checks, and cancel keeps
                the old one; Again opens another with a new seed and the same preset, with no
                prompt while the drawn document is unedited
```

**CLI**

```
RE-CLI-01  GIVEN a valid rules file, and the RE-FILE-03 files
           WHEN `ruleset-editor validate` is run on each
           THEN the valid one prints no finding, exit code 0; each invalid one prints its
                findings with line and column or key path, exit code 1; `update-logic` on an
                invalid one refuses it with a nonzero exit code, writing no file
RE-CLI-02  GIVEN a current file, and a file with modified logic
           WHEN `ruleset-editor update-logic` is run on each
           THEN the current one is left byte for byte, no .bak written; the modified one is
                refused naming the reason, unchanged, with a nonzero exit code
RE-CLI-03  GIVEN Autumn 26 as a file
           WHEN `ruleset-editor summary` is run, then with -o over an existing file
           THEN stdout holds the RE-SUM-01 text; with -o the file holds it and <file>.bak the
                file replaced; GIVEN an RE-FILE-03 file THEN it is refused with a nonzero exit
                code, writing no file; GIVEN -o naming the rules file itself THEN it is refused
                with a nonzero exit code, the rules file unchanged and no .bak written
RE-CLI-04  GIVEN `ruleset-editor random --preset weird --seed 48213 -o a.4ccrules`, and the same
           without --seed
           WHEN each is run twice
           THEN each prints the seed it used; the seeded runs write identical bytes that
                validate, the second leaving a.4ccrules.bak holding the first; rerunning the
                unseeded one with its printed seed reproduces its file
```

---

## Development phase

Phase 20 in the core plan's Development Plan, after the first release: it needs `libs/aatf` with
the schema and the interpreter (Phase 5) and the shell (Phase 8), and its users are the organizers
of invitationals, who use 4ccEditor forks until then. Deliverables: `libs/aatf`'s writer and the
migration hook, the tool's sections, opening and saving with logic identity, the summary,
Randomize, the CLI, the help chapter. The tool is file-picking only, so it joins Studio Web's
first tier (Phase 18) once both exist.

Verification: the writer round-trips every embedded ruleset (read, write, read gives an equal
`Ruleset`; writing twice gives identical bytes); VGL26's data rebuilt from Blank through the forms
alone (manual, recorded at converge) validates and passes VGL26's fixtures; a file with an older
logic hash updates and still passes its fixtures, a modified one is refused by `update-logic`; an
invalid ruleset is never written by Save or by the CLI. Randomize, over 1,000 seeds per preset:
every ruleset validates and its witnesses pass `check_team` with no error; a seed written twice
gives identical bytes; Sensible results stay inside their table's bounds; the share of seeds that
needed a redraw is measured and reported, since a generator that mostly redraws is drawing badly.
