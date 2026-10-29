# Unlimited Tool Works — Refs arranger plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The Refs arranger (`crates/tools/refs_arranger`, tool id `refs-arranger`, sidebar
label "Refs arranger") replaces the spreadsheet workflow the cup staff currently
uses to decide which referees appear in matches. It edits two files inside a refs
export: the slot allocation (`players.txt`), and — on Fox versions, where the
community's FoxDen runtime can override the game's referee draw — the **referee
lists** (`ref_lists.txt`) that the hook's script applies per match. Where the draw
cannot be overridden (PES 15–17, or Fox without the hook) it falls back to shaping
the draw statistically, against each PES version's measured slot appearance rates,
with drag-and-drop assignment and live appearance-chance feedback. It then hands the
compile itself off to the [Team compiler](team_compiler/README.md), which already owns
referee compilation. It is deliberately **not** a compiler — see "Why an arranger,
not a compiler". Platform context is in the [core plan](core/README.md).

The tool consumes `libs/aesthetics_export` (the export object model, folder conventions,
and `players.txt` slot mapping — see "Player numbering" and "Referee export
processing" in the Team compiler plan).

---

## Background

Every PES version has **35 referee slots** (`referee001`–`referee035`); each match
uses three referees (one main referee, two linesmen) drawn from those slots.
Filling all 35 with distinct models is overkill, so ~8 models are repeated across
the slots, with each model's repetition count determining how frequently that
referee appears. The mapping lives in the refs export's root `players.txt`: up to
35 slot entries, each naming a player folder, with the same folder free to appear
under any number of slots.

The catch is that **the slots are not drawn uniformly**, and the drawing mechanism
differs by engine generation:

- **PES 17–21**: measurements give per-slot marginal appearance rates, not independent
  draws or joint probabilities. Some slots look hot (16–18 appear in ~15–17% of measured
  matches each), some were rarely or never observed (11–15 and 26–30 at ~0%). FoxDen's
  reading of PES 2018's referee assigner explains the dead slots by code: in Exhibition-type
  modes the game shuffles the 25 ids 1–10, 16–25 and 31–35 and takes the first five, while
  competition, league and Master League modes draw from league-driven pools instead. Under
  that draw every live slot has the same chance, so the hot slots are likely sampling noise
  (the 125-match spread sits within about two standard deviations of 12%). A fixed weighted
  list is the intended workflow; the measured rates remain estimates until the cup's game mode
  and the other versions' assigners are known (Open questions).
- **PES 15/16**: the game picks a whole **pattern** — a fixed triple of slot numbers
  (upper linesman, main referee, lower linesman) — from a small set with very uneven
  probabilities. The dominant pattern (slots 17-8-5) covers almost a quarter of all
  matches, so a fixed assignment would show the same trio constantly. Referees must
  be **rotated across slots on a matchday basis** to spread appearances.

Until now this was managed in a spreadsheet: a pattern table where names are pasted
into cells (with bold marking the writable first occurrence of each slot and
background colors pairing the derived repeats), a storage list of available
referees, and a derived 35-entry column ready to be copied into the txt. The
arranger replaces that spreadsheet, keeps the math, and writes the file itself.

### The Fox referee hook

On PES 18–21 the draw can be overridden. FoxDen (`Tools_4cc/FoxDen`, formerly `fox_hook`: a `dinput8.dll`
that runs Lua scripts inside the Fox games' embedded engine, with a sider-compatible API)
ships `03_refmod.lua`, a trampoline on the game's **referee slot-writer** — the routine the
game calls once per match position, `slot 0..4`, with the referee id it drew (1..35). It runs
when the match menu opens, not at kickoff, so a force meant for a match is in place before its
menu opens. So a
Fox match has **five** referee positions, not three: main referee, two linesmen, and — to be
confirmed by forcing ids in-game — the fourth official and the substitution-board holder. The
hook offers two primitives, settable from Lua at any time and persistent across script reloads:

| Primitive | State | Effect |
|---|---|---|
| **Force** | five ids + an on/off flag | position *i* receives `force[i]` whatever the game drew (a forced 0 falls through to remap) |
| **Remap** | 36-entry table, identity by default | the drawn id is replaced by `remap[drawn]` |

FoxDen recognises PES 2018 to 2021, and the referee module carries a branch for each: PES
2020's uses 2019's writer pattern and 2021's match-context offsets. Runs have confirmed 2018,
2019 and 2021; the 2020 branch has not been run on a 2020 exe. The runtime's match context
exposes the home and away team ids, the stadium and a per-frame tick, so a companion script
can re-set the force table whenever a new match starts. (The module already binds referee
sets to stadiums; the arranger's lists are per match, which is what the staff schedule.) The
team ids read nil until a match is set up, so a script cannot match a `team=` selector at its
init. Nothing in the hook reads a text file today: its one file read loads a Lua chunk from
the game folder's `scripts/`. The file the arranger writes and the script that consumes it are
the contract defined below, the reading side being the hook author's to implement.

What this changes: with the hook, **which referee appears is decided in Lua, not by which of
35 slots the game happens to draw**. Each referee needs only one slot; the weighting-by-
repetition problem disappears on Fox and, with it, the "recompile the refs CPK before every
matchday" loop — rotation is a text file the script reads. The measured rates below remain the
tool's engine for PES 15–17, and the *fallback fill* for Fox viewers who get the game's own
draw.

**Who gets the file.** The referee module starts with forcing off and the remap table at
identity, then, as its boot default, forces five distinct ids drawn uniformly from 1..35 at
install and again after every match. So a Fox viewer sees one of three draws: the game's own
(no FoxDen), the module's uniform one (FoxDen, no lists file), or the lists (FoxDen and the
file). The lists file goes to the **streamers**, with one list per match of the matchday in
schedule order, so the broadcast shows the referees the staff chose. The fallback-filled
`players.txt` shapes the game's own draw; the same CPK serves all three. What the reading
script does when it finds no file, keep the module's uniform draw or turn forcing off for the
game's own, is open (Open questions): it decides whether the fallback fill also shapes what a
FoxDen viewer without the file sees. Force state persists across script reloads within a game
session, so the script re-establishes that no-file state whenever it finds no file; otherwise a
file loaded and then removed keeps forcing its last list until the game restarts.

## Slot appearance rates (reference data)

These tables are empirical measurements, embedded as per-version constants. They drive
the PES 15–17 editors and the Fox fallback fill. They can be refined as more matches
are logged; the sample sizes are given so future measurements can be merged sensibly.

### PES 17–21: flat per-slot rates

Measured over 125 matches (three referee appearances per match; rate = matches in
which the slot appeared / 125):

| Slot | Matches | Rate | | Slot | Matches | Rate | | Slot | Matches | Rate |
|------|---------|------|-|------|---------|------|-|------|---------|------|
| 1 | 8 | 6.4% | | 13 | 1 | 0.8% | | 25 | 16 | 12.8% |
| 2 | 18 | 14.4% | | 14 | 0 | 0.0% | | 26 | 0 | 0.0% |
| 3 | 11 | 8.8% | | 15 | 1 | 0.8% | | 27 | 0 | 0.0% |
| 4 | 14 | 11.2% | | 16 | 21 | 16.8% | | 28 | 0 | 0.0% |
| 5 | 11 | 8.8% | | 17 | 19 | 15.2% | | 29 | 0 | 0.0% |
| 6 | 14 | 11.2% | | 18 | 21 | 16.8% | | 30 | 1 | 0.8% |
| 7 | 15 | 12.0% | | 19 | 8 | 6.4% | | 31 | 13 | 10.4% |
| 8 | 12 | 9.6% | | 20 | 17 | 13.6% | | 32 | 12 | 9.6% |
| 9 | 17 | 13.6% | | 21 | 14 | 11.2% | | 33 | 13 | 10.4% |
| 10 | 15 | 12.0% | | 22 | 18 | 14.4% | | 34 | 16 | 12.8% |
| 11 | 0 | 0.0% | | 23 | 14 | 11.2% | | 35 | 14 | 11.2% |
| 12 | 0 | 0.0% | | 24 | 18* | 14.4%* | | | | |

\* Slot 24's count is marked uncertain in the source measurements (see Open
questions). The counts sum to **372**, not the **375** implied by 125 matches with
three appearances each. Preserve the supplied observations, but flag the table as
incomplete pending reconciliation; do not silently renormalize it or assign the
three missing appearances to slot 24.

Slots 11–15 and 26–30 are effectively dead (≤0.8%); assigning anything but the
filler to them is wasted weight.

### PES 15/16: pattern table

The game draws one of these (upper, main, lower) slot triples per match. Measured
over 64 matches:

| Upper | Main | Lower | Matches | Rate |
|-------|------|-------|---------|------|
| 17 | 8 | 5 | 15 | 23.44% |
| 5 | 22 | 25 | 14 | 21.88% |
| 7 | 25 | 17 | 12 | 18.75% |
| 3 | 7 | 4 | 8 | 12.50% |
| 20 | 2 | 33 | 8 | 12.50% |
| 25 | 22 | 5 | 2 | 3.13% |
| 9 | 24 | 7 | 2 | 3.13% |
| 25 | 7 | 17 | 2 | 3.13% |
| 19 | 16 | 18 | 1 | 1.56% |

Only **16 distinct slots** appear across the patterns (2, 3, 4, 5, 7, 8, 9, 16, 17,
18, 19, 20, 22, 24, 25, 33); the other 19 slots essentially never show and take the
filler. Note that a slot keeps one referee across all its pattern occurrences —
slot 25 appearing in four cells is still one assignment — and that slots are not
role-bound (slot 7 appears as upper, main, and lower in different patterns).

## Why an arranger, not a compiler

The Team compiler **already compiles refs exports**: a `refs`-prefixed export routes
through its pipeline as `ExportIdentity::Referees`, with numeric ID 999 rendered only
at game-format boundaries (999 is not a normal 701–920 `TeamId`) and slot-mapped
packing (see "Referee export processing" in the [Team compiler plan](team_compiler/blue_port.md)). A "Refs
compiler"
tool owning its own compile step would add no compilation behavior — it would only
need to *trigger* one, and since tool crates must not depend on tool crates (core
plan, workspace guardrail 1), that would force extracting the compilation engine
(~80% of the Team compiler crate: pipeline, processing, packing, message catalog,
settings ownership) into a lib crate with exactly one thin extra consumer. That is
guardrail 3's over-splitting warning sign, and the convenience it buys is one saved
click. Rejected; the engine split stays available along the blessed logic-vs-view
seam if a substantial second consumer ever appears (see "No per-tool companion libs"
in the core plan).

The handoff instead rides on machinery that already exists:

1. **Save** writes `players.txt` into the refs export under the exports folder.
2. The Team compiler's **folder watcher** picks the change up like any file edit and
   revalidates the refs row immediately.
3. The **"Save & open in Team compiler"** button saves (if unsaved) and asks the
   shell to switch the active tool (`ctx.switch_to_tool("team-compiler")` — an
   id-string request to `ToolContext`, no crate dependency; see "Tool plugin
   interface" in the core plan). The user presses Compile there.

A `match_feed`-style injected compile-request channel (for a true one-click
Save & Compile) was considered and deferred: it can be added later without moving
any code, and the manual switch costs a single click.

Hence the name: the tool arranges, the compiler compiles.

## The tool

The arranger reads the refs export from the common `exports_folder_path` (core plan). If multiple
non-disabled `refs`-prefixed exports are found, the arranger is inactive until the user resolves the
conflict (the Team compiler's Reader detects the conflict before validation and emits an
export-scoped `multiple_ref_exports` blocker for every conflicting row; the arranger shows a
notice pointing at them). `NO_USE`/`NO_USE.txt` sources are disabled and do not participate. Only
one plain-folder refs export can be edited at a time.

The walk finds refs exports by the compiler's rules, so the two tools never disagree on which
export is the refs one: the first word of the name folds to `refs` (`Refs - Summer` is one,
`referees 26` is a team export); a `.zip` or `.7z` counts, as it does for the compiler, and
shows the archive notice ("Saving"); a folder and an archive with the same stem are two refs
exports. The compiler's Reader keeps its disabled-marker test to itself today; it moves into
`aesthetics_export`, beside the identity parsing, and both tools call it.

### Storage box

- Lists the referees found in the refs export: the player folders under `Players/`.
  The arranger performs its plain-filesystem walk, supplies a `CanonicalListing` (each
  entry a path string with its kind and size) plus the bytes of every small metadata file
  the listing holds (`players.txt`, `refs.txt`, `notes.txt`: the parser reads each one listed,
  and one missing from the map is a failed read) to `aesthetics_export::parse_listing`, and
  receives a `ParsedAestheticsExport`; it never reads model or texture payloads. The arranger
  deliberately does not require a `ValidatedAestheticsExport`: the roster may be missing,
  invalid, or partial — creating and repairing it is this tool's job; valid entries from an
  existing `players.txt` pre-populate the assignments, while its scoped issues remain visible
  ("Loading an existing export").
- Each entry shows a version-appropriate **appearance estimate**:
  - **PES 15/16**: estimated chances of appearing as main referee and as linesman,
    separately — sum the observed rates of patterns in which the referee holds
    each kind of slot. Exact for the supplied sample, not a claim about the game's
    true probabilities.
  - **PES 17**: expected appearances per match — sum the assigned slots' marginal rates.
    Label this as an expected count, not a probability of appearing at least once; joint
    slot observations would be needed for the latter.
  - **PES 18–21**: appearances across the lists (how many lists, and in which positions),
    plus the fallback expected count from the filled `players.txt` in a secondary style,
    labelled "the game's own draw".
- One referee is designated the **Filler** (a dedicated drop target): on PES 15–17 it is
  auto-assigned to every slot not explicitly assigned, so all 35 slots are always covered
  and the written `players.txt` is always complete. On PES 18–21 there is no Filler: the
  leftover slots are filled by weighted repetition (see the Fox mode below).
- Entries are drag sources; dragging onto a slot row or a list position assigns (egui's
  built-in drag-and-drop: `dnd_drag_source` / `dnd_drop_zone`).

### Editor

The editor switches on the **global PES version selector** (sidebar), like every
version-dependent behavior in the suite:

- **PES 18–21 mode — lists.** The referee hook decides who appears, so the editor is
  about *lists*, and the slot mapping is derived:
  - **Home slots.** Every referee in the storage box holds exactly one slot, 1..N in
    storage-box order (reorderable by drag; the order is saved, so a referee's slot —
    which is its id in the lists — stays stable across sessions unless the user moves it).
    N ≤ 35; a 36th referee is refused with a notice. A move renumbers home slots, and the
    lists follow the referee: Save rewrites their ids, since a list names people, not slot
    numbers. A changed `players.txt` needs a recompiled CPK for everyone, so the Save
    confirmation says so whenever home slots moved.
  - **Lists.** A list is five positions (`0..4`, the order the game writes them; the role
    labels — main, linesmen, fourth official, board holder — are shown once the position
    roles are confirmed in-game, see Open questions), each holding one referee, **all five
    distinct** (the reason force lists are used rather than remap: collapsing 35 drawn ids
    onto N referees would regularly put the same model on the pitch twice). Lists are
    ordered; the script applies them per match in order (wrapping if the matchday runs
    longer than the file), so a file is **one matchday: list *k* is match *k* of the
    schedule**. A list may carry an optional `team=<id>` selector,
    which the script prefers for that team's matches when it can read the team ids; it is
    plain text the arranger round-trips and needs no UI beyond a field.
  - **Building lists**: by hand (drag referees into positions), or **Randomize** — *n*
    lists (*n* = the matchday's match count, asked for) such that every referee appears as
    evenly as possible per position across the set (round-robin over a shuffled order per
    position, then repaired for the five-distinct rule), which is what a person does with the
    spreadsheet by hand and gets wrong at list twelve. Undo covers it.
  - **Fallback fill.** Slots N+1..35 of `players.txt` are filled by **repeating the
    referees**, weighted by the PES 17–21 rate table so that each referee's expected
    appearances are as equal as the dead and hot slots allow — the previous engine, applied
    to the leftover slots only. This is what a viewer without FoxDen sees (and a FoxDen
    viewer without the file, if the reading script turns forcing off; see "Who gets the
    file"), so it is the ordinary viewer's experience rather than an edge case; with a file
    loaded the fill is invisible, since force never lets the drawn slot through. The fill is
    automatic and shown read-only under the lists; no Filler referee exists in this mode.
    Two referees whose expected counts tie are taken in storage-box order, so identical
    input gives identical output.
- **PES 17 mode**: all 35 slots as editable rows — slot number, measured rate,
  drop target with the assigned name. Dead slots (~0%) render grayed (filler by
  default) but stay assignable. The slot list *is* the mapping; no preview needed.
  (The table was measured on the Fox side and is assumed to hold on 17 — see Open
  questions; if it does not, this mode gets its own measurement, not a different UI.)
- **PES 15/16 mode**: **slot list + pattern preview.**
  - The editor is one row per distinct pattern slot (16 rows), sorted by total
    weight, each showing the slot number, its overall appearance rate, its
    main-vs-linesman split (e.g. slot 7: main 15.6%, lines 21.9%), and the drop
    target. The 19 never-appearing slots are not listed — they take the filler.
  - Below it, a **read-only pattern preview**: the familiar Upper/Main/Lower ×
    pattern-rows table with assigned names filled in live and each row's match %.
  - **Hover cross-linking**: hovering a slot row highlights all its cells in the
    preview, and hovering a preview cell highlights its slot row.
  - This replaces the spreadsheet's colors+bold convention deliberately: that
    convention existed only because a spreadsheet must designate one writable cell
    per repeated slot and visually pair the derived copies. With the mapping edited
    slot-per-row, every assignment has exactly one editing surface — no
    writable/derived distinction, no 16-color pairing to scan; identity is shown on
    demand by the hover link.
- A **list panel** always shows the resulting 35 entries (the `players.txt` view,
  filler and fallback-fill entries grayed).
- A **Shuffle** button (PES 15–17) randomly permutes the referees currently assigned to
  slots across those same slots — a one-click rotation for the PES 15/16 matchday
  workflow. Unassigned referees stay in the storage box and never appear, the Filler
  stays on every slot it holds, dead or live, and the slot set is preserved (so a
  deliberately dead slot keeps its Filler). Undo restores the pre-shuffle assignment.

### The lists file (`ref_lists.txt`)

Written beside `players.txt`, copied by the cup staff into the hook's folder together with
the compiled CPK. Plain text, because the consumer is Lua 5.1 with `string.gmatch` and no
parser for anything richer, the arranger must read it back to edit it, and a line should be
fixable in Notepad on a match night — one format, one home, a few lines of parsing on each
side:

```
# ref_lists.txt — written by Unlimited Tool Works Refs arranger
# one list per line: five referee slot ids, positions 0..4 as the game writes them
# optional selector after the ids: team=<id>; blank and # lines ignored
3 7 1 5 2
4 2 8 1 6
7 3 5 2 8 team=712
```

Grammar: five integers 1..35 separated by whitespace, then zero or more `key=value` tokens,
then an optional `#` comment. The arranger rejects a line with a repeated id or an id above
N with a scoped issue on load, and never writes one. The file's reader and writer live in the
arranger's crate, its only consumer: the Team compiler admits the file at a refs export's
root without reading or packing it.

The reading script is the hook author's; the arranger's responsibility ends at the file, and
this grammar is the interface to agree on. What the hook offers it today:

- `ref_mod.set` takes a table of five ids and turns forcing on.
- The module counts writer calls, five per match; the count grows by five *after* a match's
  draw is written. So the script sets list 1 at once when it starts, and list *k*+1 at the
  count's *k*-th jump, before the next match menu opens; set later, a match gets the
  module's boot draw.
- The hook reloads changed `scripts/*.lua` only, so a lists file placed or replaced
  mid-session reaches the script only if the script re-reads it itself.
- With no file, the script re-establishes the no-file draw ("Who gets the file").
- Reading plain text is the hook author's to add: the referee module runs in the game's own
  Lua state, where the standard `io` library is not to be assumed; a module in the hook's
  separate state, which opens the standard libraries, or a new DLL primitive can read it.

### Loading an existing export

One rule covers every line the arranger cannot use: it is listed with its issue in the tool's
issues panel and not loaded, and the next Save rewrites the file without it.

- `players.txt`: the roster rules are the compiler's. They run only inside
  `aesthetics_export`'s `validate` today, which keeps no slot map once an issue drops the whole
  export (a duplicate slot does); the crate gains a per-line roster check that `validate`
  itself calls, and the arranger takes each line's issue from it, so the rules have one copy.
  A line with no slot or no name, a slot outside 1..35, or a folder not in `Players/` is not
  loaded; of two lines on one slot, the first loads. A hand-written file (one-digit slots,
  TABs, CRLF, a BOM) reads as the compiler reads it, and Save rewrites it canonical.
- On PES 18–21 a referee's home slot is its lowest slot in the file; the storage box takes
  that order and numbers home slots 1..N from it (a file on slots 1, 2 and 5 loads as 1, 2
  and 3), the lists following as for a move. Referees in no slot go to the end.
- `ref_lists.txt`: a line with other than five ids, a repeated id or an id above N is not
  loaded. `key=value` tokens the arranger does not know and `#` comments are kept as written.

### Saving

Save writes `players.txt` into the refs export — and, on PES 18–21, `ref_lists.txt` beside
it — and nothing else: folders are never touched. `players.txt` uses the Team compiler
plan's canonical UTF-8/LF roster grammar, through a writer `aesthetics_export` gains for it;
the Export upgrader, which writes the same grammar from a `refs.txt` today with a function
of its own, moves onto it. Each write goes to a sibling temporary file renamed onto the
target, so folder watchers can never observe a half-written file. The Team compiler and the
settings file each do this with a private helper today; the Team compiler's moves into
`studio_core`, and settings, the compiler and the arranger share it. Overwriting an existing
`players.txt` asks for confirmation; creating one where none existed does not. The
confirmation names the lines the rewrite drops and says the file is rewritten canonical.
Save refuses with a notice while the storage box is empty (the compiler drops a roster with no
entry). On PES 18–21 with no list, Save writes no `ref_lists.txt` and removes an existing one,
with the same confirmation. A legacy `refs.txt`
(the read-only alias `aesthetics_export` accepts on referee exports — see "Player
numbering" in the Team compiler plan) pre-populates the assignments like a
`players.txt` would; on save it is removed after the new `players.txt` is in place,
with the same confirmation, so a successful save leaves only the canonical file. During the brief
transition both may exist; the compiler's documented `players.txt` precedence handles that state.
This is a deliberate, narrow exception to the "source exports are read-only" principle:
that rule binds the *compiler*
(auto-fixes stay in memory so submissions aren't altered); the arranger is an
export **editor**, like the Kit config editor editing kit folders inside exports —
editing the export is its purpose.

The refs export must be a **plain folder**: writing into `.zip`/`.7z` archives is
out of scope (an archived refs export shows a notice). In practice refs exports are
maintained by cup staff as folders.

### CLI

`UTW refs-arranger chances ./exports/refs_export` — prints the
per-referee appearance estimates (and per-slot assignments), labeled as probability for PES 15/16
or expected count for PES 17–21 (on 18–21 additionally the per-list, per-position counts from
`ref_lists.txt`), derived from the export's current files, using the active PES version's table.

`UTW refs-arranger randomize ./exports/refs_export --lists 12` — writes `ref_lists.txt`
with *n* randomized lists over the export's referees (same algorithm as the button; refuses
to overwrite without `--yes`), for the staff member who rotates from a script. Arrangement by
hand is GUI-only; drag-and-drop has no meaningful CLI analogue.

Both commands read the export as the GUI does: a `players.txt` beside a `refs.txt` wins with
the same warning, and an archived export is refused with the GUI's notice and a failing exit
code. The PES version is the saved common setting, as for the Team compiler's CLI; there is
no version flag.

### Settings

None initially — the tool contributes an empty settings section. The rate tables
are constants, not settings.

## Matchday rotation

- **PES 18–21**: none at compile time. The CPK is compiled once per referee roster change.
  Before a matchday the staff produce that matchday's lists file — one list per match in
  schedule order — and send it to the streamers, who drop it into the hook's folder; nobody
  else receives anything. No recompile, no Studio required at the venue.
- **PES 15/16**: before each matchday, open the arranger, reshuffle names across the slot rows
  (guided by the chances in the storage box), save, switch to the Team compiler,
  recompile the refs CPK. A possible future helper — tracking cumulative appearances
  across saved lists and suggesting the next assignment that equalizes them — is
  noted but not part of the initial scope.

## Development phase

Phase 13 in the core plan's Development Plan. Small: the export knowledge already
lives in `libs/aesthetics_export`; the tool adds the rate-table constants, the slot-list
and lists editors, the chance math, the fallback fill, the two writers, and the handoff
button. Needs `aesthetics_export` (Phase 3) and the app shell (Phase 8).

Verification covers canonical `players.txt` roundtrip and chance math; `ref_lists.txt`
round-trip including selectors and comments; the randomizer's invariants (five distinct per
list, per-position counts differing by at most one across the set, an N < 5 roster refused with
a notice); the fallback fill (every referee in slots 1..N exactly once and in that order, slots
N+1..35 repeating referees with expected counts within the table's resolution of each other,
identical output for identical input); plus observed atomic replacement of an existing file:
interrupted temporary writes leave the old target intact, watcher reads see either the complete
old or complete new file but never a partial one, and replacement is exercised explicitly on
Windows.

## Open questions

- **Position roles on Fox.** Which of the hook's positions 0..4 is the main referee, which are
  the linesmen, and what the remaining two are (fourth official and substitution-board holder,
  presumably). Settled in game by `ref_mod.set` with five distinct, recognizable models in one
  Exhibition match, then a second with the ids rotated, which shows the roles follow the
  position, not the id; until then the lists editor labels positions by number.
- **The reading script and the no-file draw.** How the script reads the file (above: the
  hook offers no text reader yet) is the hook author's side of the contract and does not change
  the file. What it does with no file is the staff's and the hook author's call, and decides
  whom the fallback fill serves (`QUESTIONS.md`).
- **PES 2020.** The referee module's 2020 branch has not been run on a 2020 exe; a run that
  logs the module installing and a forced match settles it.
- **Do PES 15 and 16 share one pattern table?** The measurements come from one of
  the two; assumed shared until measured on the other.
- **17–21 measurement reconciliation:** slot 24 is marked uncertain, and the supplied counts
  total 372 instead of 375. Confirm the missing observations and which game version(s) the
  125-match sample covers before presenting the table as verified across PES 17–21; on
  18–21 it now only shapes the fallback fill, so the stakes are lower than they were. The
  cup's game mode decides which draw applies (Exhibition's uniform shuffle over the live
  slots, or a competition mode's league pools; see Background), and with it whether the
  table's spread is noise.
- **Do 17 slots have role identity** (main vs linesman)? The flat measurements
  don't distinguish roles; if role placement turns out to matter to managers,
  the table needs re-measuring with roles logged. (On 18–21 roles are the list positions.)
