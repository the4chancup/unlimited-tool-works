# Changelog

What changed for members, release by release, newest first.

## [Unreleased]

### Changes

- **Team compiler** — on every PES version, a compiled player's look now reaches the game
  through `aesthetics_patch.toml` and the save it is applied to: the patch carries his
  appearance, motions, boots and gloves IDs, and the CPK carries no player tables, so the
  save needs no stripping. PES 2015 to 2017 get the patch too.
- **Export upgrader** — a player whose save wears a custom boots or gloves ID that no folder
  of the export holds no longer stops the upgrade: his `settings.toml` gets the hide ID
  (boots 55, gloves 11), which shows no model as the game did, and a warning names him.
- **Team compiler** — on PES 2018 to 2021, a model naming a texture nobody supplies is kept,
  with a warning per missing texture, instead of being left out of the CPK.
- **Export upgrader** — a boots or gloves folder nobody wears in the save is given to the
  player it is named after, when the save gives him a custom ID no folder holds, with a
  warning naming the pairing, instead of being dropped.
- **Export upgrader** — a folder holding one texture as both a `.dds` and a `.ftex` no longer
  stops the upgrade: the `.ftex` is dropped with a warning, or both files when no model uses
  the texture.

### Minor fixes

- **Save editor** — Up and Down walk the player list again after a page key left a team's
  Tactics slot popover open behind a player: they did nothing until the team was selected
  again and the popover closed.
- **Save editor** — after opening another save, a team's medal outlines and a player's Cards
  used badge no longer show the previous save's for a moment: they wait for the new save's own
  answer.

## [0.0.1] — 2026-11-03

- **Save editor** — the first preview, for the DBC7's managers on PES 2017: players, teams and
  tactics, the aesthetics patch, the roster sheet, Live and Multi-team AATF, the Comparator,
  undo and redo, the unsaved-changes list, and a command line for batch work.
- **Ruleset editor** — a work-in-progress look at the new cup rules format (4ccrules).
