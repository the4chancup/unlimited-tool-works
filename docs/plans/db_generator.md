# Unlimited Tool Works — DB generator plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The DB generator (`crates/tools/db_generator`, tool id `db-generator`, sidebar label "DB
generator") is the successor to **pes-db-generator** (`Tools_4cc/pes-db-generator`, Python 3.12),
the scripts that build the cup's **game database**: the `common/etc/pesdb/*.bin` tables that
define which teams and players exist at all. Every other tool in the suite edits or dresses what
the database declares; this one declares it. It runs once per cup (and again when the teams list
changes) and its output is what a fresh game install turns into the day-0 savefile. Platform
context is in the [core plan](core/README.md); the table record formats live in a new lib crate,
`pesdb`, specified below.

As with the other legacy tools, the scripts define *what* to produce (the bytes the game accepts),
not *how* the Rust code is organized.

---

## Background

PES reads its static data from fixed-size-record tables under `common/etc/pesdb/` inside the
game's data CPKs (`dt10.cpk` on 15, `dt10_win.cpk` on 16/17, `dt00_x64.cpk` on 18–21). A DLC
CPK that carries the same paths overrides them. The 4cc database replaces Konami's teams and
players wholesale with the cup's own: one team per `teams_list.txt` row (ids 701–920), 23
placeholder players per team (ids `team*100 + 1..=23`), one manager per team, and the
competition entries that put every team into one of four competitions.

| Tool | Language | Status | Location |
|---|---|---|---|
| pes-db-generator | Python 3.12 (`struct`, byte templates) | Works; console prompts; hand steps for 19+ | `Tools_4cc/pes-db-generator` |

What the scripts produce, per version 15–21, into `database_NN/`:

| File | Records | Content |
|---|---|---|
| `common/etc/pesdb/Team.bin` | one per team | ids (manager id = team id), stadium 29, nationality 231, the licensing/anthem word, and the team name repeated in every language slot; the abbreviation in the database-name and licensed-abbreviation slots; `None` as the fake abbreviation |
| `common/etc/pesdb/Player.bin` | 23 per team | a per-version **base player** template (the `PLACEHOLDER` player: name, default abilities, appearance ids) with the player id patched in |
| `common/character0/model/character/appearance/PlayerAppearance.bin` | 23 per team | a base appearance template (56 bytes after the id; one template for 16–21, one for 15) with the id patched in |
| `common/etc/pesdb/PlayerAssignment.bin` | 23 per team | assignment index, player id, team id, order (0-based on 19+, 1-based before) |
| `common/etc/pesdb/Coach.bin` | one per team | manager id = team id, nationality 231, `マネージャー` / `MANAGER` |
| `common/etc/pesdb/CompetitionEntry.bin` | one per team | team id, entry index, competition id and the team's order within it; competition 9 = the cup, 10 = Invitational, 11 = VGL, 12 = Backup, classified by the team's name |
| blank files | — | every other table the version's game expects to find, as an empty file, so Konami's rows are cleared (the numbered `Team1.bin`..`Team6.bin` siblings included on 15–20; the myClub and Weekly tables where the version has them) |

The scripts stop short of a complete database on purpose: `Country*.bin`, `Competition*.bin`,
`CompetitionKind*.bin`, `CompetitionRegulation*.bin` are Konami's and must be copied from the
game's own CPK (the scripts print the list and the CPK path). On PES 15 the three `Competition*`
tables must *not* be shipped (the game crashes).

**The 19+ savefile quirk.** From PES 19 the game generates a fresh `EDIT00000000` from the
database *without* the player records (5060 players declared, none stored), so the game falls
back to the database for every player and 4ccEditor crashes on the empty section. The scripts'
answer is a manual procedure: decrypt the EDIT with pesXdecrypter, set the player count at
`0x60`, paste a generated `Player_Edit.bin` (one EDIT-layout player record per placeholder:
gameplay template + appearance template, id patched in three places) at `0x7C`, re-encrypt.
Measured 2026-09-20 on the PES 20 invitational save this database produced: player record
70101 of that EDIT is byte-identical to the record `player_edit.py` assembles for id 70101, and
every one of its 5060 records decodes under `pes_savefile`'s PES 20 tables. The EDIT-layout
record and the `Player.bin` record are **different layouts** of the same player (the gameplay
block is not byte-shared between them), so the two are two formats, owned by two crates.

## Relationship to the scripts

Replicate the game-facing bytes, not the mechanism:

- **Byte templates → typed records.** Each base `.bin` is a record of a known layout with a
  handful of fields patched; `pesdb` gives every table a per-version record layout (fields as
  data, the same discipline as `pes_savefile/schema`) and the tool sets fields by name. The
  templates' remaining bytes (the placeholder player's abilities, the appearance defaults) are
  kept as the tool's embedded **base records**, decoded once from the scripts' `generators/bin/`
  files and committed as fixtures with their provenance.
- **`team_list.txt` → `teams_list`.** The scripts carry their own copy of the teams list in a
  three-column form (`701 3   /3/`) and classify Backup/VGL/Invitational teams by name. The tool
  reads the suite's `teams_list.txt` through `libs/teams_list` (`Row::Team` for the cup's teams,
  `Row::Placeholder` for the Backup/VGL/Invitational rows, which are exactly the rows the
  scripts route to competitions 12/11/10) so the cup has one teams list, not two.
- **The console prompts → a tool view and a CLI.** Version, teams list, output folder, PES
  install (for the Konami tables), and the three switches below.
- **The Konami tables → extracted, not listed.** With a PES install path configured (the suite's
  common setting), the tool reads the version's data CPK through `libs/cpk` and copies
  `Country*`, `Competition*`, `CompetitionKind*`, `CompetitionRegulation*` into the output, so
  the database is complete; without one it reports the list with the CPK path, as the scripts do.
  The PES 15 `Competition*` exclusion is a rule, not a note.
- **The hex-editor procedure → `pes_savefile`.** The 19+ EDIT step becomes a `pes_savefile`
  operation (`ops::populate`, below) that writes the placeholder player records into a loaded
  EDIT through the codec and re-encrypts it. The tool offers it as its third output: point it
  at the fresh EDIT the game generated, get one with players.
- **Output as folder or CPK.** The scripts write a folder tree. The tool writes the same tree,
  or packs it into a CPK through `libs/cpk` (the DLC's database CPK), the default the release
  build ships.

## Inputs and outputs

Inputs: the PES version; a `teams_list.txt` (the suite's); the output folder; optionally the
PES install path (Konami tables) and, for the savefile step, the path of a fresh EDIT.

Outputs (each an independent action, all three in one run by default):

1. **Database tree** — `database_NN/common/etc/pesdb/*.bin` and
   `database_NN/common/character0/model/character/appearance/PlayerAppearance.bin`, byte-identical
   to the scripts' output for the same teams list (see "Verification"), plus the Konami tables
   when an install is available.
2. **Database CPK** — the tree packed with `libs/cpk`; file name is an Open question.
3. **Populated EDIT** (19–21) — the given EDIT with the player section filled and the count set,
   written next to the input with the suite's `.bak` rule (`pes_savefile::EditFile::save`).

Switches: `--no-konami` (skip the table copy even with an install configured), `--cpk` /
`--tree`, `--players-per-team N` (23 by default; the scripts hard-code it and the roster width
is 32 or 40 by version, so the tool caps at the version's roster width and reports).

## The `pesdb` lib crate

`crates/libs/pesdb`: the Konami database tables the suite writes, as typed fixed-size records
with per-version layouts. Consumers: this tool (`Team`, `Player`, `PlayerAppearance`,
`PlayerAssignment`, `Coach`, `CompetitionEntry`), the [Balls compiler](balls_compiler.md)
(`Ball`, `BallCondition` — the plan's "format knowledge lives inside the tool crate" sentence is
superseded by this crate; the Ball.bin format reference stays in that plan and the record layout
is transcribed from it), and the [Stadium compiler](stadium_compiler.md) (`Stadium`). Three
writers of one family of tables is what earns the crate (core plan, workspace guardrail 3).

Shape (future tense; the implementing step copies these blocks):

```rust
/// One table: fixed-size little-endian records, no header.
pub struct Table<R: Record> { pub records: Vec<R> }

pub trait Record: Sized {
    /// The record size for `version`; `None` when the version has no such table.
    fn size(version: PesVersion) -> Option<usize>;
    fn read(version: PesVersion, bytes: &[u8]) -> Result<Self, PesdbError>;
    fn write(&self, version: PesVersion, out: &mut Vec<u8>) -> Result<(), PesdbError>;
}

pub struct Team { pub id: u32, pub manager_id: u32, pub stadium_id: u16, pub nationality: u16,
                  pub name: String, pub abbreviation: String, /* per-version licensing word */ }
pub struct Player { pub id: u32, pub name: String, /* the base record's other fields as bytes
                    the tool never sets: carried, not modeled, until a consumer needs one */ }
pub struct PlayerAppearance { pub id: u32, pub body: [u8; 56] }
pub struct PlayerAssignment { pub index: u32, pub player_id: u32, pub team_id: u32, pub order: u32 }
pub struct Coach { pub id: u32, pub nationality: u16, pub name_jp: String, pub name_en: String }
pub struct CompetitionEntry { pub team_id: u32, pub entry: u16, pub competition_id: u16, pub order: u16 }

/// The tables a version's game expects under pesdb/, blank when the cup does not fill them.
pub fn expected_tables(version: PesVersion) -> &'static [&'static str];
/// The Konami-owned tables the database must carry from the game's own CPK, and that CPK's path.
pub fn konami_tables(version: PesVersion) -> (&'static [&'static str], &'static str);
```

Rules: a byte offset appears in the crate's per-version layout tables or nowhere; `Player` and
`PlayerAppearance` carry their unmodeled bytes verbatim (the scripts' templates are the only
source for them, and the game's own tables are the parity check when a real one is read); the
crate reads as well as writes so a real Konami table round-trips byte for byte (the fixture for
each table is the smallest real one the game ships); no tool identity, `wasm32`-checkable.

## `pes_savefile::ops::populate`

The savefile side of the 19+ quirk, specified with the other operations in
[`pes_savefile/operations.md`](pes_savefile/operations.md) "Player section population": given a
loaded `EditFile` whose player section is empty (or shorter than the teams × players the list
implies), write one placeholder `PlayerEntry` per (team, slot) in id order and set the count.
The placeholder is the version's **base player** (`PlayerEntry` decoded from the scripts'
`Player_Edit_Base_NN.bin` + `PlayerAppearance_Base_16.bin` assembly, committed as a fixture),
with `id`, `base_copy_id` and, from Phase 5, the appearance-block id patched. `EditFile` gains the one thing its API refuses today —
adding player records — behind this operation only; the record order and count rules of
`to_bytes` otherwise stand.

## Tool crate layout

```
crates/tools/db_generator/
├── src/
│   ├── lib.rs          # StudioTool impl, tool id, CLI dispatch
│   ├── plan.rs         # teams list → the team/player/manager/competition-entry sets (pure)
│   ├── tables.rs       # the sets → pesdb::Table values (base records patched)
│   ├── konami.rs       # the Konami tables: read from the install's CPK or report the list
│   ├── output.rs       # tree or CPK writing; the populated-EDIT step through pes_savefile
│   ├── messages.rs     # finding codes → user text (missing install, roster width cap, PES 15 rule)
│   └── view.rs         # the egui view (Phase 8 shell)
└── tests/
    └── parity/         # the scripts' output for the three-team list, per version
```

CLI: `UTW db-generator generate --version 21 --teams-list teams_list.txt --out ./database_21
[--pes-install <dir>] [--cpk] [--edit <EDIT00000000>]`.

## Verification

- **Byte parity with the scripts**: run pes-db-generator once per version on a three-team list
  (one cup team, one Invitational, one Backup row) and commit its seven output trees as
  fixtures (a few kilobytes each); the tool's tree for the same list must be byte-identical,
  file for file, blank files included. The full 220-row list is checked as sizes and a sampled
  record per table (a full-list Player.bin is 1.5 MB per version, too large to commit).
- **`pesdb` round trips**: each real Konami table fixture reads and writes back byte for byte.
- **Populated EDIT**: populating a copy of the PES 20 fixture's stripped player section
  reproduces the fixture's player section byte for byte (the fixture *is* this database's
  product); the result opens in `pes_savefile` and its players compare equal to the base player.
- **Konami table copy**: with a PES 21 install on the reference machine, the copied tables equal
  the CPK's entries by hash.
- **In game** (manual, per version): a generated database + populated EDIT starts the game,
  shows the teams in the cup competition, and the day-0 EDIT the game writes decodes.

## Open questions

- **Database CPK file name and DpFileList position** in the 4cc DLC — take from the current
  cup's DLC layout when the Team compiler's deployment stage is specified (Phase 4); until
  then the CPK is written under the name the user gives.
- **Whether PES 20/21 also need the populated EDIT**: the scripts' README says the quirk starts
  at 19 and was seen to persist on 20+; the invitational save on the reference machine has
  its players, but whether the game or the manual procedure put them there is not recorded.
  Ask the maintainer; the operation is harmless on a save that already has them (it refuses
  a non-empty section unless told to overwrite).
- **Competition ids 9–12** are the scripts' constants; what the game shows for each (menu
  names, which one hosts the group stage) is unverified here.
- **The licensing/anthem words** per version (`0x0660`, `0x0C60`, `0x0C` + nationality 2 on
  20/21) are transcribed from the scripts; their bit meanings are unknown and kept as words.
