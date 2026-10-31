# Unlimited Tool Works — Savefile plan: Operations

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Interchange formats

### Team TOML (new, replaces `.4ccs`/`.4cct`)

What exists of this is in `spec/pes_savefile/operations.md` "Team TOML". Planned on top of it:

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
format, the Fox half's keys and `apply`, `interchange/aesthetics_patch`'s types, the
compiler's production of the file).

Still open:

- **The whole look, on every engine** (worklog 8.36c). The patch is the one route by which a
  compiled player's look reaches the game, on Fox as on pre-Fox: the Team compiler writes no
  database table rows (`PlayerAppearance.bin`, `BootsList.bin`, `GloveList.bin`) and no save
  is stripped. A compile writes a patch anyway for the motions, which live only in the save,
  so the tables never made a team's looks independent of the save; one route removes the
  stripping before every cup and the gloves clash between `GloveList.bin` and the 4cc gloves
  patch. Per compiled player, beside the motions and the `write_names` names the spec's
  format already holds, the patch holds:
  - the **`[appearance]` tables**, the `PlayerSettings` schema as his `settings.toml` and
    `ingame_face.toml` give it together (the face, hair and iris from the latter), every key
    written, with the defaults for keys his files lack, so his look is his export's whatever
    the save held. The motion keys join them there.
  - **`boots_id` and `gloves_id` over the field's whole stored range** (14 bits on every
    version), not `settings.toml`'s stock band: the resolved ID is the one the order in the
    [Aesthetics export plan](../aesthetics_export/settings_toml.md)'s "Player settings in
    exports" reaches (a committed custom output's assigned ID, else the authored stock ID 0
    to 100, else the FPC marker's default), and a category whose requested output failed, or
    a `Midcup` export's markerless default, carries no key, so the save keeps its current ID.
    The compiler plans the patch before its tasks run and fills these two keys once each
    task's outcome is known, then writes the file once: the file never holds an ID
    conditioned on a task.
  - **No look for a player whose `settings.toml` was invalid**: no `[appearance]` and no ID
    keys from that file, so the save keeps his record (the Aesthetics export plan's "An
    invalid `settings.toml` keeps the player's installed look").

  The file stays readable as the export's own files are: the same key names, tables and
  values as `settings.toml` and `ingame_face.toml` (`sleeves = "short"`, a motion as the game
  shows it), the face, hair and iris under the player's `appearance` as the joined schema
  holds them, so a reader compares a patch with a player's files key by key. What differs is
  that every value is resolved (no `""` default, no `name = true`, the FPC preset applied) and
  that there are no per-key range comments: the templates carry those.

  ```toml
  [teams.players.03]           # slot 03 → player 70203
  name = "Snuffy"              # a write_names export's only, as today
  shirt_name = "SNUFFY"
  boots_id = 143               # the assigned custom ID, the authored stock ID or the marker's default
  gloves_id = 0

  [teams.players.03.appearance]
  skin_color = 2
  # … the PlayerSettings schema, the motion keys included
  ```

  **Applying a look also makes the record his.** For a player whose entry holds
  `[appearance]`, `apply` writes it through `PlayerSettings::apply` (every key, the unused
  bits at the games' values), sets the appearance block's player id to his own id, as real
  saves and `populate_players` hold it, and clears his base copy: the base-copy id set to his
  own id (the record's "no copy") and the base-copy flag, on the versions that hold one,
  cleared. The id, because a save still at -1 from the old stripped route (the maintainer's
  test saves) would otherwise ignore the written look. The base copy, because a record whose
  base-copy id names another player shows that player's face and hair over his own: in PES
  21's Edit mode (2026-10-15) a cup save's "MESSI" copying player 7511 and a teammate copying
  114624 showed the copied faces; the game shows a copied face with the face edit flag clear as
  with it set (the same save, that flag alone cleared), and over the player's own `face.fpk`
  too (two players with one each, flag set and clear, given a base copy). A census of 125 Fox
  saves on the maintainer's machine (2026-10-14, `.tmp/5_4/base_copy_census.txt`) found such
  an id on 0 to 26 players of each real PES 21 cup save, up to 117 on PES 19 and 988 on PES
  18; counted are ids neither 0 nor the player's own. A compiled player whose export describes
  no face of his own (no face model, no `ingame_face.toml`) then shows the Placeholder face,
  as his export says, where the copy showed someone else's. The edit flags (the record's `Edited*`
  bits) are left as the save holds them: 8.36d checks in game that a compiled look shows on a
  player whose flags are clear, and if one is needed the patch sets it, as a compiler-owned
  value.

  With it the `PreFox` refusal goes, `pes_savefile`'s player-table code goes with the
  compiler's table writer (`portrait_path` moving to a module of its own: the Save editor's
  portraits and the compiler's writer use it), and the lossless guard the appearance-row
  round trip gave (a save player's settings reproduce his record) is kept as
  `from_player` → `apply` → the record's bytes.
- **The save editor's apply path.** Applying is the only aesthetics write path the save
  editor leaves open by default (see "Read-only aesthetics" in the
  [Save editor plan](../save_editor.md)); it goes through the editor's undo pipeline and the
  comparator's preview like any other bulk edit.

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
