# Unlimited Tool Works — Savefile plan: Operations

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Interchange formats

### Team TOML (new, replaces `.4ccs`/`.4cct`)

What exists of this is in `spec/pes_savefile/operations.md` "Team TOML". Planned on top of it:

**For 0.0.1: the bench is twelve, and the comments read as sentences.** A 4cc team has 23
players, and players past 23 are not supported (the Save editor's bench list shows 12 for a 4cc
team), so `bench_order` lists the first 12 of the record's 21 entries: the bench. The other nine
name no player of a 4cc team, and a member reading the file cannot tell them from the bench;
the game itself writes them as the roster slots the starting eleven and the bench leave out, in
ascending order (team 893 of a PES 2017 cup save: 23 to 31; some saves hold 255 there instead).
An import of a 12-entry `bench_order` writes those 12 and fills the nine as the game does: the
roster slots from 0 that neither `starting_eleven` nor the 12 name, ascending, as many as fit.
An array of another length is refused (`bench_order` holds 12 roster slots). Before
`[tactics.preset_1.formation_1]` a comment says how a formation's players are read: the first
entry is the player `starting_eleven` names first, and so on (`starting_eleven` and the other
roster-slot arrays count from 0, so slot 0 is `[players.01]`). Every comment the export writes
reads as a plain description for a member editing the file: no stored-byte jargon ("0-3
stored" is "0 to 3"), no inverted phrasing ("shirt number, the roster's" is "shirt number"),
versions spelled as the appearance comments spell them ("PES 17 and later", "PES 17 only"), and
nothing a member cannot act on ("the record's own field"); where a comment cannot say what a
value means in plain words, it says where the value comes from ("as the game stores it").

**Roster order.** Export from a team that breaks 4cc's roster order (slot `NN` holding a player
other than `team id × 100 + NN`: `roster_order_breaks`, `plans/aatf_rules.md` "The host")
writes the slot as the save holds it, with a note on that slot's header line naming the player
it holds and the one the rule puts there ("holds player 80423, not 80403: applied, this record
lands on slot 03, apart from 80423's aesthetics export"). A Team TOML lands players by slot, so the record leaves
its id behind, and the aesthetics export, which names players by id, no longer matches it; the
note is how a member learns that before applying the file. `apply` adds no note: the file holds
no ids. Measured 2026-10-22 (worklog 8.9): no current cup save breaks the rule; five older PES
16 saves break it in 13 to 25 teams each.

### Texport (read + write)

What exists of this is in `spec/pes_savefile/operations.md` "Texport (read + write)". Planned
on top of it:

**Writing PES 2017.** `Texport::new` writes a PES 2017 file from templates and the open save,
as it writes 18-21, rather than by editing a real 5.6 MB file the app would have to ship. Ten
real files measured (2026-10-30) put everything but the team's records and emblem in a fixed
layout. The payload, 5,578,704 bytes, is zero but:

- `0x00..0x30`, the header template (`17`, `0x50`, `0x551f80`, `0x27d8`, `4`, `4`, then `0x0c`
  at `0x24`), then 32 bytes written zero: they differ in every real file, no digest of the
  payload tried matches them, and the game imports a file with them zeroed and its records
  changed;
- the emblem block, `0x50..0x10050`, written zero: the save holds no emblem image (a cup
  team's comes from the CPK), and a real export of a team without one is zero there;
- `01 00 00 00` at `0x10050`;
- the records where the reader finds them: the team record at `0x10054`, the coach block at
  `0x10234` (the template `d7 00 ff ff 61 "Manager"`, the team id patched in front: the variant
  real files pair with `0x27d8`; the other pairs `e7`, `0x60`, `"MANAGER"` with `0x2774`), the
  roster at `0x1028C`, the tactics at `0x10330`, and each filled roster slot's player from
  `0x510840` at the player record's stride, each as the codec writes it, its bytes no field
  models zero, as on 18-21 (real files hold copies of the team's and the player's ids and
  text past a name's terminator there, which the game does not need: a written file imports
  to the same team and players);
- the 8-byte tail `04 00 00 00 01 00 00 00`.

The container: the PES 2017 key; the description `Team Export Data 01` with the team's name
at `0x80` (the number differs between real files, and the game lists the name); the game's
own thumbnail, `logo.png` (31,590 bytes, equal in all ten files), bundled with the crate; the
identifier's 64 per-file bytes zero, then `TEXPORT` (the game imports a file with them
zeroed); and the writer's serial, which `new` takes from the caller (the open save's,
`EditFile::serial`) and ignores on 18-21. PES 2015 and 2016 stay `NoTemplate`: no file of
theirs was studied for a writer, and no cup asks for one.

### Aesthetics patch (new)

What exists of this is in `spec/pes_savefile/operations.md` "Aesthetics patch (new)" (the
format and `apply` on every engine, `interchange/aesthetics_patch`'s types, the compiler's
production of the file).

Still open:

- **The `<id>.copy` link into `base_copy_id`** (worklog 8.36c3). A player folder's
  `<id>.copy` link
  (`plans/aesthetics_export/player_folders.md` "Player folders (the Studio export format)",
  its base copy paragraph) fills the entry's `base_copy_id`: the key exists already, `apply`
  writing it as his base copy with the flag left as the save holds it, where an
  `[appearance]` entry without one gets its base copy cleared to his own id. 8.36d checks a
  copy shows in game with the flag clear.
- **The in-game checks of 8.36d**: that a compiled look shows on a player whose edit flags
  are clear — the patch leaves them as the save holds them, so if one is needed it is set,
  as a compiler-owned value.

## Player section population (Phase 19, DB generator)

The savefile half of the [DB generator](../db_generator.md), specified here and implemented in
that phase: from PES 19 the game writes a fresh `EDIT00000000` from the database with the player count declared and **no player
records**, so the day-0 save has to be given its placeholder players before anyone (the game
included) can edit them. The scripts do it with a hex editor; here it is one operation:

```rust
/// Writes one base player per (team, slot) into the player section, ids `team*100 + 1..=n`
/// in team order, and sets the count. `overwrite = false` refuses a section that already holds
/// records (the day-0 save of a cup that was hand-populated is not to be flattened by accident).
pub fn populate_players(
    file: &mut EditFile,
    teams: &[TeamId],
    per_team: u8,
    base: &PlayerEntry,
    overwrite: bool,
) -> Result<usize, PopulateError>;
```

Rules: `base` is the version's **base player**, the `PlayerEntry` the scripts' EDIT-layout
templates decode to (`Player_Edit_Base_NN.bin` + `PlayerAppearance_Base_16.bin` assembled as
`player_edit.py` does; committed as a fixture with its provenance), and the operation patches
only `id` and `base_copy_id` (the record's own id copy, `base_copy_id == id` meaning "unset"),
and from Phase 5 the appearance block's player id, set to `id` as real saves hold it;
`per_team` is capped at the version's roster width; the payload's player section grows to
`teams × per_team` records through `EditFile` — the one place player records are added, since
`to_bytes` otherwise keeps the file's own record count and order; on 15–18 the operation is
allowed but pointless (their fresh saves carry the players) and says so through a finding. The
PES 20 fixture is the day-0 save of a database these scripts generated, so populating a copy
with its player section stripped must reproduce it byte for byte.

---
