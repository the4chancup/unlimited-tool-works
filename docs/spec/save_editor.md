# Unlimited Tool Works — Save editor plan

## Background: 4ccEditor

4ccEditor (`Tools_4cc/4ccEditor`, the official repository's `origin/master`; its Autumn 26
edition, 2026-10-20, merged the Tactics tab and Texport import from the fork in
`Tools_4cc/4ccEditor-1`) is a raw Win32 C++ application that edits PES save files
(`EDIT00000000`) for PES 2015–2021. The local `master` carries two of the maintainer's own
experiments upstream does not have (the "Make everyone else Messi" team action, out of
scope); read the official code through `origin/master`. Its codebase breaks down as:

| Component | Files | Size | Fate in the Rust port |
|-----------|-------|------|----------------------|
| Per-version save codecs | `pes15.cpp` … `pes20.cpp` | ~190KB | **Collapsed** into schema tables + one codec engine (`pes_savefile`) |
| Data model | `editor.h` (`player_entry` ~100 fields, `team_entry` incl. tactics) | ~30KB | Ported to `pes_savefile`; derives replace manual boilerplate |
| AATF rules tool | `aatf.cpp` | ~28KB | Ported as configurable rules (see below) |
| Comparator | `comparator.cpp` | ~22KB | Ported, merged with Midcupping's aesthetics diff |
| Misc tools | `fpc.cpp`, `data_util.cpp` | ~12KB | Ported (FPC invisibility; the bit engine goes into `pes_savefile`) |
| Crypto interface | `crypt.h/cpp` + `libpesXcrypter.dll` | external | **Replaced** by native Rust crypto in `pes_savefile` |

Portability is good: half the C++ is Win32 UI that gets replaced rather than
ported, and portable crypto reference implementations already exist. The codecs, the data
model, the bit engine and the crypto are built in `pes_savefile`
([spec](pes_savefile/README.md)); the editor's own parts are the work below.

## Background: Midcupping

Midcupping (`Tools_4cc/Midcupping`) is a set of six standalone Python scripts
(PES 15/16/17 variants of two tools) used mid-cup, when re-running full exports
is too heavy:

- **`transplant-aesthetics-*.py`** — copies player appearance data (boots/gloves
  IDs, physique, strip style, ingame face) from a donor save into a base save,
  by player ID, ID pair, team, or team range; optionally in-place.
- **`compare-saves-*.py`** — diffs two saves aesthetically: per player, decodes
  boots/gloves/face IDs, taping, glasses, sleeves, inners, socks, undershorts,
  shirttail, winter gloves, skin color, plus a normalized hash of the ingame-face
  block, and prints what changed.

The second is absorbed into the save editor as the `diff` command and the comparator panel (the
operation lives in `pes_savefile`, "Save-to-save operations" in the [Savefile
plan](pes_savefile/operations.md)). The first is not: aesthetics live in the exports now, so a
look copied into a save is overwritten by the next compile, and on a stripped Fox save never
read at all. Its successor copies a donor save's looks into the team's export instead (after
Release 0.1.0, "Aesthetics transplant" in the [Save editor plan](../plans/save_editor.md)).

---

## Feature inventory (the parity contract)

Everything 4ccEditor can do, with its fate in the port. This is the checklist the
implementation is measured against.

### Team operations

| 4ccEditor | Fate |
|-----------|------|
| Save squad (`.4ccs`) | Replaced by **Team TOML export** (full-fidelity, human-readable — see the [Savefile plan](pes_savefile/README.md)) |

### Database operations

| 4ccEditor | Fate |
|-----------|------|
| AATF: current team / select teams | Kept (rules become configurable — see below) |
| Compare EDITs | Kept, merged with Midcupping's aesthetics diff (see "Comparator") |
| — | **Export teams list** (new): the open save's team names and IDs for 701–920 (in-game names are the `/xx/` names) merged into `data/teams_list.txt` through the same reconciliation the updater uses — added / kept / overridden shown for review, savefile wins conflicts, a change behind an unresolved conflict is dropped and the rest applied, never a blind overwrite (see the [Team compiler plan](team_compiler/pipeline.md), "Resolved decisions", "Teams list"). Lives here because this tool owns the open savefile; the compiler only consumes the list |
| — | **Strip and seed** (new, Fox saves): every not-yet-stripped player's appearance into `4cc_08_bins.cpk`'s `PlayerAppearance.bin`/`BootsList.bin`/`GloveList.bin` rows, then every appearance id to -1; plus the restored-id check. See "Stripped save" |

### New (from Midcupping)

| Feature | Notes |
|---------|-------|
| Aesthetics transplant | Not carried over as a save-to-save copy; its successor writes the team's export, after Release 0.1.0 ("Aesthetics transplant" in the [Save editor plan](../plans/save_editor.md)) |
| Aesthetics diff | Part of the merged comparator |

### New (from the Team compiler)

| Feature | Notes |
|---------|-------|
| Apply aesthetics patch | The compiler's `aesthetics_patch.toml` → current save: on pre-Fox the default route by which teams' aesthetics reach the official save, on Fox the compiled players' motions and the resolved `name`/`shirt_name` entries (only from exports carrying the `write_names` marker). See "Read-only aesthetics" in the [Save editor plan](../plans/save_editor.md) |

---

## Crate layout

The Save editor follows the standard tool-crate skeleton (core plan, "Tool crates") and elaborates
below it. The organizing principle: **`pes_savefile` owns every byte and every operation on the
model; this crate owns the session (the file that is open) and the command line over it.** If a
function here could be called without an egui context and without a `Session`, it belongs in the
lib.

```
crates/tools/save_editor/src/
├── lib.rs              # Tool (StudioTool impl) — wiring only
├── settings.rs         # editor settings (aatf_ruleset, live_aatf, aesthetics_from_exports,
│                       #   scroll_names)
├── cli.rs              # the subcommands of "CLI" below; runs libs/aatf's rules over the
│                       #   session's teams and maps the results to messages
├── messages.rs         # message catalog (the codes of "Messages" below)
├── session/mod.rs      # Session: the open EditFile and the operations the commands run
├── diff.rs             # the comparator's report (finding lines, and one JSON document for --json)
├── events.rs           # a command's findings stream and the worst severity among them
└── teams_list.rs       # the teams list as export-teams-list sees it
```

Placement rules:

- **No savefile bytes, offsets, or version `match`es in this crate.** Version gating in widgets asks
  the schema (`schema.has(PlayerField::Star)`), never `version >= 19`.
- **`session/` is egui-free.** It is the unit-testable heart of the tool and is what the CLI
  subcommands drive (and the window will, through its undo stack), so the CLI and GUI cannot
  diverge.
- The AATF *engine* (rules-file loading, Rhai evaluation) lives in `libs/aatf`; `check-aatf` here
  only feeds it the session's teams and renders the results.

---

## Stripped save

On Fox the game takes a player's appearance, boots and gloves from the database tables
(`PlayerAppearance.bin`, `BootsList.bin`, `GloveList.bin`) when his appearance record's player id
is -1, and from the record when the id is set (in-game Test 1, PES 2021). The cup's official save
is therefore **stripped**: every appearance id at -1, so the tables the Team compiler writes decide
every player's look. It is prepared once per game version and distributed as the blank save
managers receive at every version change; no compile and no manager writes appearance into it
afterwards.

- **Strip and seed** is one operation (Database operations, Fox saves only): it writes every
  player's current appearance (the record's appearance bytes and his
  boots and gloves IDs) into rows of the three tables inside `4cc_08_bins.cpk`, the official
  DpFileList entry that holds the cup's team-related bins (`TeamColor.bin`, `UniColor.bin`,
  `UniformParameter.bin`), which the DLC builder installs at lower priority than the
  compiler's output, then sets every appearance id to -1. A player already at -1 is left out,
  his record not being his look: his rows stand (a table created whole holds none for him). The seed is what a
  player no export describes looks like afterwards, so stripping changes nobody's look; compiled
  players' rows replace their seed rows. Appearance holds no tactics, so the seed can come from
  any save with the cup's appearances, keeping the DLC builder and the savefile builder apart.
  Each row is taken from the player's record as the save holds it (`pes_savefile`'s
  `EditFile::appearance_row`), not rebuilt through his `PlayerSettings`: real cup saves hold
  values past a key's range inside its field's bits (79 players of the PES 18 fixture save, 26
  of the PES 19 one), which the settings refuse and the row must carry as they are.
- **A base-copied face is reported, not carried.** A player whose appearance block copies
  another player's face (a face copied from a Konami player or from a teammate in Edit mode) is
  stripped and seeded like the others, but the row has no base-copy id, so once stripped he
  loses the copied face and hair and shows the face his row describes, Placeholder for a player
  who never had one of his own (PES 21 Edit mode, 2026-10-15: a cup save's "MESSI" copying
  player 7511, and a teammate copying 114624). Strip and seed reports each such player
  (`strip_base_copied_face`, a warning naming him and the player copied); his export describing
  his face settles it, the compiled rows replacing his seed. Leaving them unstripped would keep
  their look but break "every appearance id at -1", so every later compile and `check-stripped`
  would flag them. A player counts when his base-copy id is neither 0 nor his own id, whatever
  his face edit flag says, and whether or not a face model of his own is installed: the game
  shows the copied face with the flag clear as with it set (the same save, that one flag
  cleared, showed the same face), and over his own `face.fpk` too (two players with one each,
  flag set and clear, given a base copy, both showed the copied face), which stripping
  reveals, as it does for a player without a base copy. A census of 125 Fox saves on the
  maintainer's machine (2026-10-14, `.tmp/5_4/base_copy_census.txt`) found such an id on 0 to
  26 players of each real PES 21 cup save, up to 117 on PES 19 and 988 on PES 18.
- **Nothing in the editor sets an id back.** Imports (Team TOML) and
  name writes, the aesthetics patch included, leave the appearance block's id as it is.
- **A record at -1 is not the player's look.** Everything that reads a record's appearance
  (the comparator and `diff`, a Team TOML export) marks a
  player at -1 as stripped beside what it shows or writes.
- **The game's own Edit mode does.** Test 1 found that any change to a stripped player in Edit
  mode restores his id: a rename brings back his pre-strip look, a look edit freezes the database
  look shown into the save (with boots 0). Managers therefore rename players in this editor and
  change looks only through the export (the Player aesthetics editor, then a recompile).
- **A check reports restored ids**: `check-stripped` lists the players whose appearance id is
  set while others are stripped, and `--strip-again` sets their ids to -1 (their record is
  ignored from then on; the tables decide).

Pre-Fox saves are not stripped: the database route is untested there, and the aesthetics patch
stays their route.

---

## Comparator (gameplay + aesthetics)

The `diff` command is the merge of 4ccEditor's `comparator.cpp` with Midcupping's
`compare-saves-*.py` (the panel is Phase 8's):

- It takes a second save (the same version) and lists the per-player differences.
- The two scopes are `PlayerDiff::scope()`'s (`DiffScope`, "Save-to-save operations" in the
  [Savefile plan](pes_savefile/operations.md)); the lists below are the two references' rows,
  each a subset of ours.
- **Gameplay scope** (from 4ccEditor): names, IDs, basics (age/height/weight),
  every ability stat, playstyle, positions and ratings, COM styles, all skills —
  with old → new values.
- **Aesthetics scope** (from Midcupping): boots/gloves/face IDs, taping, glasses,
  sleeves, inners, socks, undershorts, shirttail, winter gloves, skin color, and
  the normalized ingame-face fingerprint (catches "the face was edited" without
  decoding every facial parameter). It compares the records, so a player at -1 is
  marked ("Stripped save").
- `--aesthetics` and `--gameplay` keep one scope; the report prints as one finding line
  per row, or as one JSON document with `--json`.

---

## Configurable AATF rules

4ccEditor's AATF tool hardcodes both its parameters and its logic in C++; a ruleset change is a
recompile. This editor checks a **rules file** instead: a ruleset as data plus a
generic interpreter, specified in the [AATF rules plan](aatf_rules.md) (`libs/aatf`), and
built without code in the [Ruleset editor](../plans/ruleset_editor.md). Every cup's current
ruleset ships embedded, the official one the default; the `aatf_ruleset` setting names the one
in use, an embedded ruleset's file stem (`vgl27`) or the file name of a copy in the data
folder's `rules/`.

What this editor adds on top of the lib:

- **The ruleset dropdown.** A combo box on the toolbar, left of the Live AATF toggle, drawn
  whether Live AATF is on or off: the ruleset drives the quick actions, the Cards left badge
  and which skills the Tricks or Free cards panel holds, not only the check. It lists the
  rulesets built into the binary, every cup's current ones in `aatf_rules.md`
  "Embedded rulesets"'s order, each by its description alone, the file's `name` (`aatf_rules.md`
  "The ruleset schema": `/vg/ League 27`, not a file name or a code), then under a separator the imported ones the same way, each row with an ×
  at its right end (a frameless small button: egui's combo popup holds any widget), then
  under another `Import rules file…`, a command rather than a value. The choice
  is the `aatf_ruleset` setting (an embedded ruleset's file stem, `autumn26`, or an imported
  copy's file name), remembered across restarts, and `check-aatf` without `--rules` reads it.
  It replaces the `aatf_rules` path setting and its settings-menu row: one place to pick the
  rules, the place a member switching between cups' rules is already looking (switching
  through the settings row was awkward). The settings menu's Save editor section keeps the
  "Aesthetics from exports" checkbox ("Read-only aesthetics").
  - **Import rules file…** opens the file dialog on `.4ccrules` files and loads the pick as
    `Rules::load` does (`aatf_rules.md` "Loading and validation"). A file with findings is
    refused, a dialog titled `The rules file has errors` listing them, each with its line and
    column or key path, and nothing is copied: no file that does not load can become the
    ruleset, so the quick actions and the badge never have a load error to show (the file is
    fixed, in the Ruleset editor or by hand, and imported again). A clean file is copied into
    the data folder's `rules/` under its file name and becomes the ruleset. The imported
    rulesets are the `.4ccrules` files in that folder, read at start, no list kept beside
    them, so moving or editing the original changes nothing until it is imported again. A
    file whose description a built-in ruleset has is refused naming the clash (two rows would
    read the same, and a built-in is not replaced); one whose file name or description an
    earlier import has replaces that import once a confirmation naming it is accepted (a cup's
    corrected rules keep their name). A file picked under another ending is copied as
    `<stem>.4ccrules`, and one whose file name differs from an earlier import's in case alone
    takes that import's name: Windows reads the two as one file, and the replace would
    otherwise delete the copy it had just written. With no data folder (a portable copy
    beside a read-only exe), the import is refused saying so. No Save a copy: writing a
    ruleset out is the Ruleset editor's.
  - **The ×** asks first (`Remove an imported ruleset?`, naming it, saying its copy is
    deleted and the original stays), then deletes the copy; removing the ruleset in use falls
    back to the first built-in one.
  - **A choice that is gone**: a copy missing from `rules/` at start, or one that no longer
    loads (edited by hand there), is left out of the list, and a setting naming it falls back
    to the first built-in ruleset with a warning notice, `The AATF ruleset picked last is gone
    from the data folder: <ruleset> is used instead.`
- **No AATF panel.** Live AATF is the window's check: there is no floating "AATF
  violations" window, no menu entry for one, no Validate rules and no Save a copy. A
  member checks the team they are making legal, which Live AATF does after every edit;
  importing validates a rules file; the Ruleset editor writes a ruleset out. An organizer's
  check of several teams or the whole save is the Savefile menu's Multi-team AATF (below), or
  `check-aatf --teams` on the command line.
- **Multi-team AATF.** The Savefile menu's `Multi-team AATF…`, after Output rosters to TSV,
  opens a dialog, `Multi-team AATF (<ruleset>)`, for an organizer checking a cup's
  teams, who does not use a command line. On one row `Teams`, the range's first team, `to`, its
  last: two combo boxes of the save's teams by id and name, the save's first and last by
  default, the pick kept while the save stays open. Under it a line, `N teams checked, M
  with issues` (`The first team comes after the last` in the warning colour when reversed, and
  Copy to clipboard off); then every team of the range under its id and name, its rows
  grouped and worded as Live AATF's dock words them, errors first, or `All clean` under a team
  with none, so a clean team reads as checked rather than missed, the list as wide as the
  dialog's content; then `Copy to clipboard` and `Close`. The teams are checked on a worker, in parallel as `check-aatf` runs them (220 teams
  of a PES 2017 save in 2.0 s on 16 threads, 13.2 s on one), when the dialog opens and
  whenever the range changes, the previous list kept and marked as checking until the new one
  arrives; a team whose check fails is a line naming it and the error, the others still
  listed. Suggestions are not asked for (an unused allowance is the manager's business, not
  the organizer's), and a row's click does nothing. A dialog, not a docked panel or a
  floating window: it is a report read once and copied out, while Live AATF is where a team
  gets fixed. Copy to clipboard copies the list as plain text, a line per row: first the
  ruleset and range with the error and warning counts, then per team its id and name after a
  `>`, which a cup's thread draws as green text so each team stands out, per player his id and
  name indented two spaces (the team's own rows under `Team-wide issues`; `All clean` there
  under a clean team), per row its severity, rule and sentence indented four, so it pastes
  readably into the thread (a failed team's error under its line); a notice says `Copied the
  results of N teams.`, a count of 1 reading in the singular. The list is 614 by 360 points
  and scrolls. `violations_list` draws both Live AATF's dock and this list (`ListUse`), and
  the copied text walks the same grouping (`issue_groups`), so the two read alike.
- **Validate rules.** `check-aatf` refuses a rules file that fails the lib's load-time
  validation with its findings (line or key path), so the command doubles as a rules file's
  check.
- AATF reports and never fixes, as in 4ccEditor.

---

## CLI

The batch surface for the operations that came from scripts — Midcupping existed as
Python scripts precisely because GUIs can't be looped:

```
UTW save-editor export-toml ./EDIT00000000 --team 701 -o ./aaa.toml   # single-team full-fidelity Team TOML dump
UTW save-editor export-toml ./EDIT00000000 --all-teams --out-dir ./teams
UTW save-editor import-toml ./aaa.toml --base ./EDIT00000000 --out ./EDIT_new   # patch the base save
UTW save-editor apply-patch ./aesthetics_patch.toml --base ./EDIT00000000 --out ./EDIT_new   # the compiler's patch
UTW save-editor diff ./a_save ./b_save [--aesthetics | --gameplay] [--json]
UTW save-editor check-aatf ./EDIT00000000 [--teams 701,705-710] [--rules ./vgl27.4ccrules]
UTW save-editor strip-and-seed --base ./EDIT00000000 --bins ./4cc_08_bins.cpk --out ./EDIT_new --bins-out ./4cc_08_bins_new.cpk
UTW save-editor check-stripped ./EDIT00000000 [--strip-again --out ./EDIT_new]
UTW save-editor export-teams-list ./EDIT00000000 [--yes]     # merge the save's team names/IDs into data/teams_list.txt
```

Rules every command shares:

- **Exit codes**, the Team compiler's meanings: 0 done; 1 done with an error reported (a patch's
  team the save lacks, an AATF error, a restored id listed); 2 refused before anything was
  written (a bad argument, a version mismatch, an unreadable file, a team or player the save
  lacks); 3 a write failed (what was already written stands).
- **Writing a save.** A command that writes a save reads its source save (`--base`, or
  `check-stripped`'s one save) and writes `--out`, which may be the same path; an existing
  output save or CPK (`--out`, `--bins-out`) is first copied to `<file>.bak` (one level, as every
  save does). There is no implicit base save and no command creates a whole EDIT. While the PES
  of that save's version is running (the source save's version, read before anything is
  written), a command that writes a save writes only when given `--edit-mode-closed`, the
  member's word that the game's Edit mode is closed: the game writes the save it holds over the
  file when Edit mode closes, so a write made while it is open is lost then, while one made with
  it closed stands and Edit mode's Load reads it in without restarting the game. Without the
  flag the command writes nothing and exits 2, its message naming the game and the flag. Not a
  warning followed by the write: a write made while Edit mode is open is lost without a word,
  and the warning would come after it. A running PES of another version needs no flag, because
  a game only ever writes its own version's save; refusing on any running game would refuse
  every edit of another version's save, and a copy of one, for no protection. The compiler's
  savefile step follows the same rule (`team_compiler/pipeline.md`,
  `savefile_skipped_pes_running`).
- **A stripped player is marked.** On a Fox save, `diff` marks a player at -1 as stripped
  ("Stripped save") beside his rows, `export-toml` with `stripped = true` in his table
  (informational; import ignores it).

The commands:

- `export-toml` requires an explicit source EDIT and either `--team <id>` with `-o`/`--out <file>`,
  or `--all-teams` with `--out-dir <directory>` for one file per team of the save, named
  `<id>.toml`. These forms are mutually exclusive; there is no implicit current-team selection.
  An existing TOML is replaced, with no backup; a `--team` the save lacks is refused, and so
  is an `--out` naming the input save itself (exit 2, nothing written): the no-backup rule
  is for a Team TOML, and replacing the save with one would lose it.
- `import-toml` requires the Team TOML, `--base <EDIT>`, and `--out <EDIT>`. It imports onto
  the team the document's `id` names, preserving unmodeled
  save data; it prints each import note (a converted or dropped value) on its own line. The
  file's sections are its selection: a section left out of the file is untouched, so the CLI has
  no section flags. A document with no `id`, or one the save lacks, is refused.
- `apply-patch` takes the patch, `--base <EDIT>` and `--out <EDIT>` exactly like `import-toml`;
  it refuses a version or allocation-scheme mismatch before touching anything, prints the per-team
  summary (applied / skipped: not in save), and is the batch form of the GUI action — the savefile
  builder can apply a night's patches in one script. A team is listed as applied only when at
  least one of its players was written: a team the save holds whose patch players it all lacks
  prints only their skips. A skipped team or player is an error (exit
  1) and the rest is written. Like every command that writes a save (`import-toml`'s notes,
  `check-stripped --strip-again`'s fixed players), its lines are printed
  once `--out` is written: a write that fails prints only its failure (exit 3), never a line
  saying what was written. A pre-Fox patch is refused until the compiler writes one (deferred
  with pre-Fox's other gaps). The read-only lock is a GUI guard; a CLI command that writes
  aesthetics (an `import-toml` whose file holds aesthetics) is explicit by nature and is not
  gated.
- `diff` prints the comparator's report to stdout, players paired by id, each changed field as
  old → new (the appearance id is no row: a stripped player is marked instead), then the
  players only in one save (human-readable by default, `--json` for
  tooling), replacing the Midcupping compare scripts; `--aesthetics` and `--gameplay` keep one
  scope. Saves of two versions are refused. Differences are its output, not an error: exit 0.
- `check-aatf` runs the active ruleset (`--rules`, else the one the `aatf_ruleset` setting
  names) over the cup teams the
  save holds (701-920, or `--teams`) and prints each violation with its team and, for a player's,
  the player; an error-severity violation makes the exit code 1, a warning alone leaves it 0. A rules file that fails load-time validation is
  refused with its findings (line or key path), so `check-aatf` doubles as the CLI's rules
  validation. Each violation is one finding line, `aatf_violation`, at the violation's severity:
  the team, the player's id for a player's violation, the rule (the violation's kind as a rules
  file names it, `FieldedNotA`) and the kind's own fields in their rules-file spelling
  (`position=CF`, `stat=dribbling`), teams in id order, each team's lines in the order its check
  returns them. Suggestions are not printed: the CLI has no "Show suggestions" toggle, and an
  unused allowance is not something to fix. A team whose check fails (a bug in the logic or in a
  custom check) is an `aatf_check_failed` error line and the other teams are still checked.
  `--teams` takes ids and inclusive ranges (`701,705-710`); an id the save lacks is refused
  (exit 2). The setting resolves as the window resolves it: an embedded ruleset's stem, else a
  copy of that file name in the data directory's `rules/` that loads, else the first embedded
  ruleset, Autumn 26, with the warning line `aatf_ruleset_gone` (a copy deleted or edited
  until it no longer loads; the check still runs). `--rules`, a command-line path, is relative
  to the current directory.
- `strip-and-seed` is "Stripped save"'s operation on a Fox save: it reads `--bins` (the cup's
  `4cc_08_bins.cpk`) and writes the seeded tables into `--bins-out`, then the stripped save into
  `--out`: when `--bins-out` cannot be written, the save is not (exit 3); a `--bins-out` written
  beside a save left unstripped changes no look, the records still deciding. A table `--bins`
  lacks is created from the seed rows. A pre-Fox save is refused. Each base-copied face is a
  `strip_base_copied_face` warning line; warnings leave the exit code 0.
- `check-stripped` lists the players whose appearance id is set while others are stripped, one
  `appearance_id_restored` error each (exit 1 when it lists any; a save no player of which is
  stripped lists nobody); `--strip-again` sets their ids to -1, leaving the tables as they are, and
  writes `--out`, which it requires (exit 0), reporting the players it set once `--out` is
  written, so a failed write reports none fixed. A pre-Fox save is refused.
- `export-teams-list` prints the merge summary (added / kept / overridden / unresolved) and stops
  unless `--yes` is given; a change behind an unresolved conflict is dropped, as for the updater
  merge (`libs/teams_list.md`). An unwritable data directory reports `teams_list_read_only` and
  writes nothing (exit 1). The save's list is its slash-wrapped names with a cup id (701-920),
  colour codes stripped; a later team folding to an earlier one's name (the fold is
  case-insensitive; a real PES 21 cup save holds `/4ccg/` twice) is left out and reported as
  `teams_list_name_duplicate` (exit 1), since the list holds a name once and the save, not the
  list, is where the clash is fixed.

AATF quick actions (`apply_tier`), the FPC toggle, `settings.toml` generation, Texport and the
legacy imports have no command: they are GUI actions (Phase 8), and generation's batch form is
the Export upgrader's (Phase 6).

---

## Messages

Every finding a command reports is a `Message` carrying one of these codes; an `Error` finding
makes the command's exit code 1. The catalog is `crates/tools/save_editor/src/messages.rs`,
whose order this table keeps; the console prints the code with its context fields (the
parenthesized names). All findings are run-wide and kept: a command works on one save, and a
finding that drops something says so with its own code.

| Code | Severity | When | Effect |
|------|----------|------|--------|
| `team_exported` | Info | `export-toml` wrote one team's file (`team`, `path`) | informational; goes on |
| `import_key_not_in_version` | Warning | `import-toml`: a key the document's version stores and the save's does not; dropped (`path`) | warns and goes on |
| `import_value_not_encodable` | Warning | `import-toml`: a value the save's version cannot store (an instruction it lacks); written as off (`path`, `label`) | warns and goes on |
| `import_value_capped` | Warning | `import-toml`: a face type above the save's version's cap, or a skin the version has no custom slot for; written as `to` (`path`, `from`, `to`) | warns and goes on |
| `import_slot_empty` | Warning | `import-toml`: a `[players.NN]` whose roster slot is empty; skipped (`slot`) | warns and goes on |
| `import_text_cropped` | Warning | `import-toml`: a name or shirt name longer than the save's field; cut to it at whole characters and colour codes (`path`; `cropped`, the cut text as the game shows it, colour codes left out) | warns and goes on |
| `patch_team_applied` | Info | `apply-patch`: the save holds the team and at least one of its patch players was written (`team`); a team whose players the save all lacks is not listed | informational; goes on |
| `patch_team_not_in_save` | Error | `apply-patch`: the save has no team with this id; its players were not looked at (`team`) | the rest is written; exit 1 |
| `patch_player_not_in_save` | Error | `apply-patch`: the save has no player with this id (`player`) | the rest is written; exit 1 |
| `player_diff` | Info | `diff`: one field of a player in both saves, old to new (`player`, `field`, `old`, `new`) | the report's rows; differences are the output, exit 0 |
| `player_only_in` | Info | `diff`: a player in one save alone (`player`, `save`: `first` or `second`) | informational; goes on |
| `player_stripped` | Info | `diff`: the player's appearance id is -1 in `save` (`first`, `second` or `both`); his record is compared, not the look the game shows (`player`, `save`) | marks the player beside his rows; exit 0 |
| `teams_list_added` | Info | `export-teams-list`: a team of the save the list lacked, appended or put into a placeholder's slot (`team`, `name`) | merged; written only with `--yes` |
| `teams_list_overridden` | Info | `export-teams-list`: a name in both whose id becomes the save's (`name`, `old`, `new`) | merged; written only with `--yes` |
| `teams_list_unresolved` | Error | `export-teams-list`: a change dropped because its id collided with another row of the merged list (`team`, `name`) | the change is dropped, the other changes written with `--yes`; exit 1 |
| `teams_list_name_duplicate` | Error | `export-teams-list`: a team of the save left out because an earlier team folds to the same name, which the list can hold once (`team`, `name`, `kept`) | the team is left out; exit 1 |
| `teams_list_kept` | Info | `export-teams-list`: the working rows that ended unchanged (`count`) | informational; goes on |
| `teams_list_written` | Info | `export-teams-list`: the merged list was written (`path`) | informational; goes on |
| `teams_list_not_written` | Info | `export-teams-list` without `--yes`: the summary only, the file untouched (`path`) | nothing is written |
| `teams_list_read_only` | Error | `export-teams-list --yes`: the data directory cannot take the file (`path`, `error`) | nothing is written; exit 1 |
| `aatf_violation` | the violation's (Error or Warning) | `check-aatf`: one rule the team breaks under the ruleset (`team`; `player` for a player's violation; `rule`, then the rule's own fields) | printed; an error-severity violation makes the exit code 1 |
| `aatf_check_failed` | Error | `check-aatf`: the ruleset's check itself failed on the team, which the other teams' results do not cover (`team`, `error`) | the other teams are still checked; exit 1 |
| `aatf_ruleset_gone` | Warning | `check-aatf` without `--rules`: the `aatf_ruleset` setting names a ruleset that is gone from the data folder or no longer loads (`ruleset`, `used`) | the first embedded ruleset, which the line names, is checked instead |
| `appearance_id_restored` | Error, or Info once re-stripped | `check-stripped`: the player's appearance id is set again while another player's is stripped, so the game shows his save record, not the database rows (`player`, `name`) | exit 1 while he stands listed; reported at Info and exit 0 once `--strip-again` has stripped him |
| `strip_base_copied_face` | Warning | `strip-and-seed`: the player's record copied another player's face, which the seed rows cannot carry, so once stripped he shows the face his row describes; his export describing his face settles it (`player`, `name`, `copied`) | warns and goes on |

---

## Acceptance

Format and rules: `CONTRIBUTING.md` "Testing". IDs are never reused; later phases append.

**Phase 5 scope.** Phase 5 delivers the tool crate's settings, CLI and operations wiring, and
`libs/aatf`; the view is Phase 8's, so these scenarios are what the CLI shows: stdout, the exit
code, and the files written (a save's decrypted payload, a Team TOML, a CPK's tables read back
with the `cpk` crate). The operations with no command (editing, tactics, the FPC toggle, AATF
quick actions, generation, Texport, the legacy imports) get their scenarios with the view.
`libs/aatf` is a lib: its rulesets' fixtures are its spec (AATF rules plan, "Development phase
and verification"), and `check-aatf` is its one scenario surface here. Test data: the fixture
saves are `pes_savefile`'s (`tests/fixtures/`, one real save per version, rebuilt from the
slices held there), a team or player
named by its id in that save; a stripped save is a fixture save after `strip-and-seed`. The
cup saves with known violations and the rules files are `libs/aatf`'s fixtures. The stripped-save
scenarios run on PES 21; PES 18 and 19 join them within Phase 5, once a step has measured those
versions' appearance offsets.

**CLI**

```
SE-CLI-01  GIVEN the PES 21 fixture save, a Team TOML of one of its teams, and an existing file
           at --out
           WHEN import-toml writes --out, then again with --out equal to --base
           THEN each time <out>.bak holds the file the write replaced, and the exit code is 0
SE-CLI-02  withdrawn: restated as SE-CLI-03, the save written while its game runs
           when the command is given --edit-mode-closed (step 8.9l5a)
SE-CLI-03  GIVEN a running PES of the save's version (the tests' running-PES probe)
           WHEN import-toml, apply-patch, strip-and-seed or check-stripped --strip-again is
                run on that save
           THEN without --edit-mode-closed nothing is written, the exit code is 2 and the
                message names the game and the flag; with it, --out is written as with no
                game running; with only a PES of another version running, the command writes
                without the flag
```

**Team TOML**

```
SE-TOML-01 GIVEN the PES 21 fixture save
           WHEN a team is exported with export-toml --team and the file imported back with
                import-toml into a new --out
           THEN the --out save's decrypted payload equals the fixture's
SE-TOML-02 GIVEN the PES 21 fixture save
           WHEN export-toml --all-teams --out-dir is run into a folder holding an older <id>.toml
                of a team the save holds
           THEN the folder holds one <id>.toml per team of the save, the older one replaced, each
                headed by the save's version and its team's id; the first, the replaced one and
                the last parse as a Team TOML of their team (all 220 is a minute of parsing in a
                test build, and the format's own tests cover the parse)
SE-TOML-03 GIVEN the PES 21 fixture save
           WHEN export-toml names a --team the save lacks, or both --team and --all-teams
           THEN it is refused with exit code 2 and no file is written
SE-TOML-04 GIVEN a Team TOML whose only content is [team] id, [players.03] name and one key of
                [players.03.appearance.physique], for a team whose slot 3 is rostered
           WHEN it is imported into the PES 21 fixture save
           THEN slot 3 of that team carries the name and the key's value, and every other byte
                of the payload, the player's other appearance fields included, is unchanged
SE-TOML-05 GIVEN a Team TOML, exported from the PES 21 fixture save, of a team both fixture
                saves hold, one of whose players has a goal celebration above PES 18's top
           WHEN it is imported into the PES 18 fixture save
           THEN the --out save holds the team as converted for PES 18 (that celebration
                written at the top) with every other byte unchanged, each converted or dropped
                value is printed as a note line, and the exit code is 0
SE-TOML-06 GIVEN a Team TOML with a player name longer than the target version's name field
           WHEN it is imported
           THEN it exits 0, --out holds the name cut to the field at whole characters and
                colour codes, and an import_text_cropped warning names the player's key and
                the cut name as the game shows it (colour codes left out)
SE-TOML-07 GIVEN a stripped PES 21 save and the unstripped PES 21 fixture save
           WHEN a team whose slot 3 is rostered is exported from the stripped one with
                export-toml, given an appearance table for its slot 3, and imported into each
           THEN every player of the exported file carries stripped = true; after each import
                slot 3's appearance fields are the file's, his appearance id still -1 in the
                stripped save and still set in the other
SE-TOML-08 GIVEN a Team TOML whose [team] id the PES 21 fixture save lacks
           WHEN it is imported
           THEN it is refused naming the team, with exit code 2, and --out is not written
SE-TOML-09 GIVEN a copy of the PES 21 fixture save
           WHEN export-toml --team 714 is run with --out naming that copy
           THEN it is refused with exit code 2, and the copy is unchanged
```

**Aesthetics patch**

```
SE-PAT-01  GIVEN the PES 21 fixture save and a PES 21 aesthetics patch with motions and names
           for one of its teams
           WHEN apply-patch is run
           THEN that team's players carry the patch's motions and names, every other
                byte of the payload is unchanged, the summary lists the team as applied, and the
                exit code is 0
SE-PAT-02  GIVEN a PES 21 patch for two teams, the second absent from the save
           WHEN apply-patch is run
           THEN the first team is applied, the second listed as skipped: not in save, --out is
                written, and the exit code is 1
SE-PAT-03  GIVEN a patch whose pes_version differs from the save's, and one whose
                allocation_scheme_version differs from the suite's
           WHEN apply-patch is run with each
           THEN each is refused naming the mismatch, with exit code 2, and --out is not written
SE-PAT-04  GIVEN a stripped PES 21 save and a PES 21 patch with names and motions for one team
           WHEN apply-patch is run
           THEN the team's players carry the patch's names and motions and their appearance ids
                are still -1
SE-PAT-05  GIVEN the PES 21 fixture save and a PES 21 patch for one of its teams whose every
                player id the save lacks
           WHEN apply-patch is run
           THEN each player is listed as skipped: not in save, the team is not listed as
                applied, and the exit code is 1
SE-PAT-06  GIVEN SE-PAT-01's save and patch, and an --out that cannot be written (a folder of
                that name)
           WHEN apply-patch is run
           THEN it fails naming --out, with exit code 3, and no team is listed as applied
```

**Transplant**

```
SE-TRN-01  withdrawn: the save-writing transplant is removed (8.9j5c); its successor writes
           the team's export, SE-GUI-14 (`plans/save_editor.md`)
SE-TRN-02  withdrawn: the save-writing transplant is removed (8.9j5c)
SE-TRN-03  withdrawn: the save-writing transplant is removed (8.9j5c)
```

**Diff**

```
SE-DIF-01  GIVEN two PES 21 saves where one player's boots ID and one ability stat differ, and a
                player is only in the second
           WHEN diff is run, then with --aesthetics, then with --gameplay
           THEN the first lists both fields as old → new, the second only the boots ID, the
                third only the stat, each run also listing the player only in the second;
                every exit code is 0
SE-DIF-02  GIVEN the same saves
           WHEN diff --json is run
           THEN stdout parses as JSON carrying the same rows and the player only in one save
SE-DIF-03  GIVEN a PES 21 and a PES 19 save
           WHEN diff is run
           THEN it is refused naming both versions, with exit code 2
SE-DIF-04  GIVEN a stripped PES 21 save and the fixture it was stripped from
           WHEN diff is run
           THEN each player at -1 is marked stripped, with no row for his appearance id
```

**AATF**

```
SE-AATF-01 GIVEN a cup save with known error-severity violations, a team-level one (a tier
                count) among them, and a clean one, with no --rules and no rules file
                configured (the official ruleset); and one whose only violations are
                warning-severity under a rules file that has warnings (the official file with
                one special's severity a warning: the official ruleset itself has none since
                2026-10-22)
           WHEN check-aatf is run on each
           THEN the first prints each violation with its team and, for a player's, the
                player, and exits 1; the second prints its warnings and exits 0; the third
                prints none and exits 0
SE-AATF-02 GIVEN the save with error-severity violations in two teams
           WHEN check-aatf --teams names one of them, and again naming a team id the save
                lacks, alone or beside one it holds
           THEN the first prints only that team's violations, and the exit code is 1; the
                others are refused naming the missing id, with exit code 2, printing nothing
SE-AATF-03 GIVEN a rules file that fails load-time validation, and the VGL27 ruleset with a save
                whose team breaks a VGL27 rule the official ruleset does not have
           WHEN check-aatf --rules is run with each, and with no --rules and the settings'
                aatf_ruleset naming the embedded vgl27
           THEN the first is refused with its validation findings (line or key path) and exit
                code 2; the second and the third print that violation, which a run with neither
                does not
SE-AATF-04 GIVEN the VGL27 ruleset and save of SE-AATF-03, a copy of the ruleset in the data
                directory's rules/, and the command started from another directory
           WHEN check-aatf is run with aatf_ruleset naming the copy's file name, and again
                naming a copy that is not there
           THEN the first prints SE-AATF-03's VGL27 violation; the second prints the warning
                line aatf_ruleset_gone naming the official ruleset and checks against it
SE-AATF-05 GIVEN SE-AATF-02's save, with error-severity violations in two cup teams and a team
                outside 701 to 920 that would break the same rules
           WHEN check-aatf is run without --teams
           THEN both cup teams' violations are printed, in team id order, none of the other
                team's, and the exit code is 1
SE-AATF-06 GIVEN a rules file that passes load-time validation but whose check fails on one
                cup team, and a later cup team with a violation
           WHEN check-aatf is run with it
           THEN the first team is the error line aatf_check_failed, the later team's violation
                is still printed, and the exit code is 1
SE-AATF-07 GIVEN the settings' aatf_ruleset naming the embedded vgl27 and the official ruleset
                written to a file given by --rules
           WHEN check-aatf is run on SE-AATF-03's save
           THEN the --rules file is the one checked: the VGL27 violation is not printed
```

**Stripped save**

```
SE-STR-01  GIVEN the PES 21 fixture save with one player's appearance id at -1, and a
                4cc_08_bins.cpk whose three player tables hold rows for him
           WHEN strip-and-seed is run
           THEN --bins-out's PlayerAppearance.bin holds a row per other player equal to his
                record's appearance bytes, and BootsList.bin and GloveList.bin his boots and
                gloves IDs; the stripped player's three rows are unchanged; the CPK's other files are
                unchanged; in --out every appearance id is -1 and every other byte is unchanged
SE-STR-02  GIVEN a PES 17 save
           WHEN strip-and-seed is run, and check-stripped
           THEN each is refused as a pre-Fox save, with exit code 2, and nothing is written
SE-STR-03  GIVEN a stripped PES 21 save in which two players' appearance ids are set again
           WHEN check-stripped is run, then with --strip-again
           THEN the first lists the two players and exits 1; the second sets both ids to -1 in
                --out, writes no table, and exits 0
SE-STR-04  GIVEN the PES 21 fixture save, and a fully stripped one
           WHEN check-stripped is run on each
           THEN neither lists a player, and both exit 0
SE-STR-05  GIVEN a --bins-out that cannot be written
           WHEN strip-and-seed is run
           THEN it fails naming --bins-out, with exit code 3, and --out is not written
SE-STR-06  GIVEN the PES 21 fixture save with one player's appearance id at -1, and a
                4cc_08_bins.cpk holding none of the three player tables
           WHEN strip-and-seed is run
           THEN --bins-out holds PlayerAppearance.bin, BootsList.bin and GloveList.bin with a
                row per other player built from his record, as in SE-STR-01, and none for the
                stripped one; the CPK's other files are unchanged
SE-STR-07  GIVEN the PES 21 fixture save, whose players copy player 7511 with the face edit
                flag clear but for team 795's, which copy their own ids, with one copying
                player's id set to 0
           WHEN strip-and-seed is run
           THEN every player is stripped and seeded as in SE-STR-01; each player copying 7511
                gets one strip_base_copied_face warning naming him and player 7511, team 795's
                and the one at 0 none; the exit code is 0
SE-STR-08  GIVEN a stripped PES 21 save in which one player's appearance id is set again, and
                an --out that cannot be written (a folder of that name)
           WHEN check-stripped --strip-again is run
           THEN it fails naming --out, with exit code 3, and no player is reported fixed
```

**Teams list**

```
SE-TL-01   GIVEN a data/teams_list.txt and the PES 21 fixture save with a team name missing from
                the list and one listed under another ID
           WHEN export-teams-list is run, then with --yes
           THEN the first prints the added and overridden rows and leaves the file unchanged;
                the second writes them, the save's ID winning; both exit 0
SE-TL-02   GIVEN a save whose team names would give two teams one ID in the merged list
           WHEN export-teams-list --yes is run
           THEN the colliding change is listed as unresolved and not written, the other
                changes are, and the exit code is 1
SE-TL-03   GIVEN an unwritable data directory
           WHEN export-teams-list --yes is run
           THEN teams_list_read_only is reported, nothing is written, and the exit code is 1
SE-TL-04   GIVEN a save holding two teams whose names fold to one (the PES 21 fixture's two
                /4ccg/)
           WHEN export-teams-list --yes is run
           THEN the first in record order is merged, the other is listed as
                teams_list_name_duplicate naming the team kept and is not written, and the
                exit code is 1
SE-TL-05   GIVEN a data directory whose teams_list.txt cannot be read (a folder of that name)
           WHEN export-teams-list --yes is run
           THEN it is refused naming teams_list.txt, nothing is merged or written, and the exit
                code is 2
```
