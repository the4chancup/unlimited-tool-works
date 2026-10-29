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

### Aesthetics patch (new)

What exists of this is in `spec/pes_savefile/operations.md` "Aesthetics patch (new)" (the
format, the Fox half's keys and `apply`, `interchange/aesthetics_patch`'s types, the
compiler's production of the file).

Still open:

- **The pre-Fox half of the format.** On **pre-Fox** (PES 15–17) the patch holds everything
  below, as the only route for aesthetics, names likewise only from a `write_names` export: per
  player, the `PlayerSettings` schema plus `name`, `shirt_name` and the compiler-owned
  `edit_flags`, with `boots_id`/`gloves_id` over their full stored range — the resolved ID is
  the one the order in the [Aesthetics export plan](../aesthetics_export/settings_toml.md)'s
  "Player settings in exports" reaches (a committed custom output's assigned ID, else the
  authored stock ID 0 to 100, else the FPC marker's default), and a category whose requested
  output failed carries no key, so the save keeps its current ID — the same union Team TOML's
  player aesthetics section holds; nothing is invented for the patch. Every appearance field is
  written, with `settings.toml`'s defaults for keys its file lacks, so a player's appearance is
  his export's whatever the save held.

  ```toml
  [teams.players.03]           # slot 03 → player 70203; the Fox keys are the spec's
  name = "Snuffy"              # resolved: `name = true` became the string
  shirt_name = "SNUFFY"
  # pre-Fox only from here on
  boots_id = 143               # resolved: the assigned custom ID, the authored stock ID or the marker's default
  gloves_id = 0
  edit_flags = 12

  [teams.players.03.appearance]
  skin_color = 2
  # … the PlayerSettings schema, as the player's settings.toml and ingame_face.toml give it
  # together (the face, hair and iris from the latter, the defaults without it)
  ```

  The pre-Fox half also lifts the `PreFox` refusal and adds its keys to the spec's types.
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
and from Phase 5 the appearance block's player id, set to `id` as real saves hold it (a
stripped save gets its -1 from the Save editor's strip-and-seed, never from here);
`per_team` is capped at the version's roster width; the payload's player section grows to
`teams × per_team` records through `EditFile` — the one place player records are added, since
`to_bytes` otherwise keeps the file's own record count and order; on 15–18 the operation is
allowed but pointless (their fresh saves carry the players) and says so through a finding. The
PES 20 fixture is the day-0 save of a database these scripts generated, so populating a copy
with its player section stripped must reproduce it byte for byte.

---
