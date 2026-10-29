# Unlimited Tool Works — Team creator plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The Team creator (`crates/tools/team_creator`, tool id `team-creator`, sidebar label "Team
creator") is the one place where a team is made from nothing: a name and a list of players in,
a legal roster and a compilable aesthetics export out. It is written for the new manager who has
just inherited a dead team and for the veteran who needs a fake team for an invitational by
tonight — the same six screens, read slowly by the first and clicked through by the second. It
is deliberately **a wizard, not an editor**: it owns no file format and keeps no state of its
own, and when it finishes the per-domain tools take over. Platform context is in the [core
plan](core/README.md).

The tool consumes `libs/aesthetics_export` (export writing), `libs/pes_savefile` (the team
model and Team TOML), `libs/aatf` (legality), `libs/teams_list` (team identity),
`libs/color_tools` (picker), and `libs/team_widgets` (the tactics and card widgets it shares with
the Save editor — see "`libs/team_widgets`" in the [Save editor plan](save_editor.md)).

---

## Background

The wiki's newcomer pages (`resources/*.wikitext`: *New managers*, *New manager advice*, *Export
making advice*, *AET*, *FAQ*, *Blender tutorials*) spread the making of a team over half a dozen
pages and four programs. Reduced to what the games and the rules actually require:

| Side | Required | How it is done today |
|---|---|---|
| Savefile | 23 names, shirt names and numbers; a registered position each; the medal tiers (GGSS today — gold, gold, silver, silver; the rules file defines them); height-bracket quotas; base stats per tier; a legal formation; one captain, at least one GK; set-piece takers | 4ccEditor by hand, then Auto-ATF to find out what is wrong ("or you'll end up on the Fools page") |
| Export | team colors, a logo, at least one outfield kit and a GK kit (texture, config, menu colors), a portrait per player, and for every player either a face folder or an in-game-editor head | the AET template zip and its step-by-step guide; "download another team's export to see how it's laid out" |
| Models ("blenders") | optional; the wiki's own on-ramp is *replace one texture in a template*: cardhead, boxhead, the anime "starter kit" | Blender, the `Aesthetic_Templates` page |

Nothing in that table needs a format or a rule the suite does not already own. The savefile side
is `pes_savefile`'s team model and `libs/aatf`'s rules; the export side is the Studio export format
and the Team compiler's fill-in rules (a kit folder with no texture gets the placeholder, no
`config.toml` gets the template, no `colors.txt` gets derived colors); the model side is a player
folder with one texture swapped. The creator's job is to *ask the questions in the right order*
and write the answers where those tools expect them.

---

## Design principles

1. **The creator owns no format and keeps no state.** Its output is an ordinary Studio aesthetics
   export plus an ordinary Team TOML, written through `aesthetics_export` and `pes_savefile`; there
   is no `team_creator.toml`, no project file, no reopen. The export folder *is* the saved state,
   and every later change happens in the tool that owns that part — the Player aesthetics editor
   for a player folder, the Kit config editor for a kit, the Save editor for the roster and
   tactics. A reopenable creator was considered and rejected: it would have to read back files
   other tools may have edited since, tolerate every hand edit the format allows, and become a
   second editor for each of them — three tools' worth of surface for one tool's convenience.
2. **Legal by construction.** Every default the creator fills in comes from the same AATF rules
   file the Save editor's checker and its Make Gold/Silver buttons read (`libs/aatf`), and the
   Team TOML is checked by `libs/aatf` before it is written. A team that leaves the creator with no
   violations shown cannot fail Auto-ATF.
3. **Every non-blocking element is skippable.** The veteran's path is Team → Players → Next ×5. Only
   what the game or the rules cannot do without blocks Finish: a team name, at least one player,
   medal counts, a GK, a captain. Cards, instructions, models, kits, a logo — all optional, all
   filled with a legal or a placeholder default.
4. **Teach on the way, then hand off.** Each step carries one paragraph of *why* (from the tool's
   `help/` chapter, sourced from the newcomer pages), and Finish writes a per-team `README.txt`
   that names the folders it just created and says what to do with each of them next.

---

## The wizard

A left rail lists the steps; any step can be revisited before Finish. Each page is a form with
its explanatory paragraph collapsed under a "Why?" disclosure, expanded by default until the user
collapses it once (a per-tool setting).

### 1. Team

- **Name** — the `/xx/` token, picked from the teams list (`libs/teams_list`) or typed. A typed
  name not in the list is allowed with a warning: a fake team for an invitational gets its ID from
  the host, who adds it to the list; the creator does not invent IDs. The export folder's first
  word is this token, which is how the compiler resolves the team (see "Team identity" in the
  [Aesthetics export plan](aesthetics_export/README.md)).
- **PES version** — defaults to the common setting; decides the Team TOML's version and which
  version-gated fields the later steps show.
- **Colors** — two, through the `color_tools` picker. They become the root `colors.txt` and the
  default menu colors of the kits.
- **Logo** — one image, any accepted format and size; the compiler squares and resizes it (root
  `logo.<ext>`, see "Logo" in the Aesthetics export plan). Optional.

### 2. Players

Type or paste names, or import a **roster file**. Both feed the same table: number, name, shirt
name, position, medal, captain. Up to 23.

Roster file grammar — plain text, one player per line, because a roster poll's ranked result *is*
one name per line:

```
# roster.txt — one player per line, in shirt-number order; blank and # lines ignored
Snuffy
Anon              CB
Mod               GK
Based Department  CF      gold
Janny             AMF     silver   captain
07  Sneed                          # a leading 1–23 sets the number explicitly
```

The name is the line up to the first run of two or more spaces or a tab; the remaining
whitespace-separated tokens are, in any order, a position code (`GK CB LB RB DMF CMF AMF LMF RMF
LWF RWF SS CF`), a medal (`gold`, `silver`), and `captain`. Numbers are assigned in file order
unless a line starts with one; a duplicate or out-of-range number is an error naming the line.
Anything the grammar cannot place is an error, never a guess.

**Shirt names** are derived from the name (`pes_savefile::shirt_name_from(name, version)`: the
version's field length, the game's character set — the lib owns the limits) and editable. A name
the derivation cannot reduce to a valid shirt name leaves the field empty and flagged.

### 3. Roles

The step that makes the team legal. On entering it, everything not yet set is filled by
`legal_defaults` from the AATF rules file; the user changes what they want, and an AATF panel
(the same rendering as the Save editor's) shows what is still wrong.

- **Formation** — a picker of **stock formations** (4-4-2, 4-4-2 diamond, 4-3-3, 4-2-3-1,
  3-5-2, 3-4-3, 5-3-2), their coordinates recorded as constants from a reference save whose
  provenance the constants file names. The pitch (from `team_widgets`) shows it and allows
  dragging, but nobody has to drag.
- **Positions** — the first eleven take the formation's slots in number order (1 = GK); 12 is a
  GK; 13–22 mirror the formation's outfield slots again; 23 is a third GK. Registered position
  rated A, everything else C (AATF: no B ratings). All editable per player.
- **Medals** — the tiers are the ones the rules file defines (gold and silver today; a bronze tier
  appears if a cup brings GSSBB back). None by default, since they are the manager's core choice,
  and a blocking violation until the counts match the rules file. **Suggest medals** assigns them in one click: gold to the
  most attacking starters (CF, SS, AMF, then wide forwards, then CMF), silver to the next in the
  same order, never a GK.
- **Heights** — from the active ruleset's height systems: under the official one the **Red**
  system by default (no giants; simpler quotas), switchable to Green. Bracket quotas are filled in
  position order — GKs and defenders take the tallest bracket first, forwards and attacking
  midfielders the shortest — each player at their bracket's midpoint, GKs at the system's GK rule,
  gold players below the giant threshold. A ruleset without brackets (VGL26) fixes heights through
  its tiers' conditions, and each player takes their tier's height. Weight defaults to the midpoint
  of the rules' allowed window for the height. Age is not set (the save's value stays).
- **Stats** — per tier through `aatf::apply_tier` (the function behind the Save editor's Make
  Gold/Silver/Regular buttons): rates, form, injury resistance, weak foot, height bonuses.
- **Captain** — the first gold unless set. **Set-piece takers** by the wiki's rule of thumb: PK
  and long FK to a gold, short FK to a silver attacking midfielder, corners to the silvers.

### 4. Cards and instructions

Optional, and legal when skipped: no cards is a valid team ("styleless is legitimate", says the
wiki), and advanced instructions default to none. The page is the Save editor's tactics tab and
its skill/COM/playstyle pickers, rendered by `team_widgets` over the draft `TeamEntry`, with the
Save editor's one remaining-count badge per player: the cards his own tier leaves him, as the AATF
rules read him (`player_tier`, then `card_limits`). Preset 1 only; presets 2 and 3 are the
Save editor's business.

### 5. Looks

Per player, a **starter head** and one image. The list shows each player with a thumbnail of the
chosen starter; a drop zone takes the image; **Use for everyone** applies one starter to all
players without one. The default starter is **In-game head** — no image needed, no Blender,
exactly what the wiki tells beginners to start with.

Applying a starter to a player writes their folder `Players/NN - Name/`: the starter's files, the
image saved under the starter's input stem, a `portrait.png` cut from the same image (largest
centered square, scaled to 128²), and a `settings.toml` holding the starter's settings plus
`name = true`. Image handling is one rule: **fit onto a power-of-two square canvas, transparent
margins, PNG out** — the compiler discards a Fox texture with neither side a power of two
(`texture_not_pow2`), and the starters' UVs map the whole square, so a portrait-shaped picture keeps
its proportions with the margins invisible. Any format the `image` crate decodes is accepted.

### 6. Kits

`p1` and `g1` by default; **+ kit** adds up to `p9`. Per kit: two menu colors (defaults: the team
colors for outfield kits, swapped for the GK kit), a drop zone for the main texture, and a
**Drawn for** selector (this version / PES 15–17 / PES 18–21) that writes the kit layout marker
(`pre-fox` / `fox`, see "Kit layout marker" in the Aesthetics export plan) when it names the
other engine than the target.

No texture means the compiler's placeholder — a checkerboard kit, loud on purpose. The creator
does **not** generate kit textures: a flat two-color fill was considered and dropped, because a
button does the job better — **Design a kit online** opens PES Master's kit creator
(`https://www.pesmaster.com/kit-creator/`), a template-and-pattern kit designer whose download
is a 2048² PNG drawn for PES 2018–2021. Pressing the button sets that kit's **Drawn for** to
PES 18–21, so a pre-Fox compile re-lays the socks out automatically. PNG kit
textures need no conversion: every texture in a Studio export may be any accepted image format,
kits included.

No `config.toml` is written — absent means the template, which is what a new team wants; the
Kit config editor is one click away for anyone who wants a collar.

### 7. Finish

Shows what will be written and where, then writes it:

- The export folder, into the common `exports_folder_path` by default, named after the team
  token (editable). **Never over an existing folder**: the creator makes new teams; an existing
  export is edited with the other tools. The folder is assembled in a temporary sibling and
  renamed into place, so a failure leaves nothing half-written.
- The Team TOML **beside** the export, `<token> - team.toml`, not inside it — the two files have
  different recipients (the aesthetics helper gets the export, the savefile builder gets the
  roster; the same split the aesthetics patch exists for), and a Team TOML inside an export would
  be an unknown root file to the compiler. The file carries only what the creator decided; keys it
  does not write leave the save's values untouched on import.
- `README.txt` in the export root (a known root file the compiler ignores): the players and which
  starter each got; how to replace a starter with a real model — a "blender", in the community's
  word — through the Player aesthetics editor's Blender path and the wiki's Blender tutorials;
  how to make a real kit (the online designer, the wiki's kits pages, the Kit config editor); how
  to compile and test (Team compiler); how to get the roster into a save (Save editor,
  `import-toml`); how to submit (zip the export folder; the AET rules). Generated per team, so it
  names the folders that actually exist.

Then the hand-offs, each a `ctx.switch_to_tool(...)` request (see "Tool plugin interface" in the
core plan): **Compile now** (Team compiler — its watcher has already seen the new folder), **Edit
a player** (Player aesthetics editor), **Edit kits** (Kit config editor), **Import roster into a
save** (Save editor).

---

## Starter heads

A starter head is a Studio player folder plus a manifest:

```
cardhead/
├── starter.toml
├── thumbnail.png
├── head.glb                 # the model, glTF — one file serves PES 15–21 (the compiler converts)
├── materials.toml
└── settings.toml            # the settings the starter needs, if any
```

```toml
# starter.toml
name = "Cardhead"
description = "A flat picture where the head is. Mirrored on the back."
input = { stem = "card", hint = "Any picture; a square one fills the card." }   # absent: no image
```

The starters are **embedded in the binary** (`include_dir!`, as the compiler's templates are) and
nothing else: no user folder, no download. Adding one is a Studio release. A user-extensible
folder was considered and rejected for now: the format is the ordinary player folder, so anyone
who can make a starter can make a player folder, and the Player aesthetics editor already edits
those; a second discovery mechanism would exist to save that person a copy. If the community
produces a set worth shipping, it ships.

Starter set:

| Starter | Input | Notes |
|---|---|---|
| In-game head | none | portrait + an empty `ingame_face.toml`, nothing else — the game's face editor supplies the head (the ingame-face workflow in the Player aesthetics editor plan) |
| Cardhead (mirrored) | image | one plane weighted to `sk_head`, two-sided material |
| Cardhead (two-sided) | image | the same, with the back face duplicated so text reads correctly from both sides |
| Boxhead | image | a cube, the image on every face |

All four are model files the wiki's *Blender tutorials* page describes step by step (Cardhead,
Boxhead); their Studio-format versions are a deliverable of the tool's phase, converted once from
the community's `Aesthetic_Templates` models or rebuilt from the tutorial, and verified in-game on
both engines (transparency in the cardhead material on PES 15–17 in particular).

---

## Output

What a finished run leaves behind, for a team `xx` with three players and two kits:

```
exports/
├── xx - team.toml                (Team TOML: roster, roles, tactics preset 1 — for the save editor)
└── xx/
    ├── README.txt
    ├── colors.txt
    ├── logo.png
    ├── Kits/
    │   ├── p1/
    │   │   ├── colors.txt
    │   │   ├── kit.png           (if dropped in)
    │   │   └── fox               (if "Drawn for" named the other engine)
    │   └── g1/
    │       └── colors.txt
    └── Players/
        ├── 01 - Snuffy/          (In-game head)
        │   ├── portrait.png
        │   ├── ingame_face.toml
        │   └── settings.toml
        ├── 02 - Anon/            (Cardhead)
        │   ├── head.glb
        │   ├── materials.toml
        │   ├── card.png
        │   ├── portrait.png
        │   └── settings.toml
        └── ...
```

Every file is one the compiler already reads under its existing rules; nothing here is
creator-specific.

---

## CLI

```
studio team-creator create --roster roster.txt --team xx --out ./exports
    [--version 21] [--colors "#RRGGBB,#RRGGBB"] [--logo logo.png] [--formation 4-4-2]
    [--heights red|green] [--starter ingame-head|cardhead|cardhead-two-sided|boxhead]
    [--images ./pics] [--suggest-medals]
```

The scripted fake-team path: the wizard's defaults, with `--images` naming a folder whose files
are matched to players by number (`07.png`) or by name (`Sneed.png`); a player without an image
gets the In-game head starter regardless of `--starter`. An existing target folder fails the
command — the CLI never overwrites, exactly like Finish. Medal counts that do not satisfy the
rules fail the command unless `--suggest-medals` is given; the CLI never writes an illegal Team
TOML.

## Settings

- `show_explanations` (bool, default true) — the "Why?" disclosures start expanded.
- `heights_system` (`red` | `green`, default `red`).

---

## Crate layout

Standard tool-crate skeleton (core plan, "Tool crates"), elaborated below it:

```
crates/tools/team_creator/src/
├── lib.rs              # Tool (StudioTool impl) — wiring only
├── settings.rs
├── cli.rs              # create
├── messages.rs         # roster-file errors, starter/image findings, write failures
├── draft.rs            # Draft: the wizard's in-memory team (TeamEntry + per-player starter/image),
│                       #   egui-free; what the CLI builds directly
├── roster.rs           # roster file grammar → Draft players
├── legal.rs            # legal_defaults: positions from formation, heights to quota, weights,
│                       #   captain and set pieces; calls aatf::apply_tier for stats
├── formations.rs       # stock formation constants (+ provenance)
├── starters.rs         # embedded starter heads: manifest parsing, listing, applying to a folder
├── images.rs           # fit-to-pow2 square, portrait crop
├── writer.rs           # export folder (via aesthetics_export) + Team TOML (via pes_savefile)
│                       #   + README, in a temp sibling, renamed into place
└── view/
    ├── mod.rs          # step rail, navigation, Finish
    └── steps/          # one file per step (team, players, roles, cards, looks, kits, finish)
```

Placement rules: `draft.rs`, `roster.rs`, `legal.rs`, `writer.rs` are egui-free and are what the
CLI drives, so GUI and CLI cannot diverge. Anything about *how a team is legal* lives in
`libs/aatf` (parameters, checks, `apply_tier`), never here; `legal.rs` only decides *which* legal
value to pick where several would do (which bracket a CB gets). Formation constants stay in this
crate as long as it is their only consumer (core plan, guardrail 3); if the Save editor gains an
"apply stock formation" action they move to `team_widgets`.

---

## Development phase

Phase 17 in the core plan's Development Plan — after the first release: its value is for the
*next* cup's new managers, and it depends on the widest set of finished pieces (the Save editor's
tactics widgets in `team_widgets`, `libs/aatf` with `apply_tier`, the compiler's kit and marker
rules, the Player aesthetics editor to hand off to). Deliverables: the wizard, the roster grammar,
`legal_defaults`, the formation constants, the four embedded starter heads in Studio format, the writer,
the CLI, the help chapter written from the newcomer pages.

Verification: roster grammar round-trip and error cases (duplicate number, unknown token, 24
lines); `legal_defaults` output passes `libs/aatf` with the default rules for every stock
formation, both height systems, and rosters of 11, 18 and 23; image fitting (non-square in → pow2
square out, aspect preserved, margins transparent; portrait 128² centered); a created export
compiles on both engines with zero errors and the expected placeholders (`kit_placeholder`,
`kit_config_generated`); the Team TOML imports into a fixture save and the imported team passes
the Save editor's AATF check; each starter verified in-game on PES 17 and PES 21 (head replaced,
picture upright and not mirrored where it must not be, cardhead transparency); the CLI produces
the same files as the wizard for the same inputs.

## Open questions

- **Starter assets.** Whether the community's cardhead/boxhead templates convert cleanly to Studio
  format or are rebuilt from the tutorial; settled when the phase starts, by trying.
- **Cardhead transparency on PES 15–17.** The wiki describes cardheads for both engines, but
  whether the pre-Fox material handles an alpha-masked plane the way the Fox one does is
  confirmed in-game, not assumed; a fallback is a starter without margins (image stretched).
- **Formation coordinates.** Which reference save supplies the stock formations, and whether they
  differ between versions enough to need per-version constants.
