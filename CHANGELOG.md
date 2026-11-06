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
- **Export upgrader** — when the save gives a player a custom boots or gloves ID no folder
  holds, a folder nobody wears is given to him instead of being dropped, with a warning
  naming the pairing: the folder named after him, the folder numbered like the other kind's
  folder he was given, or, with as many such folders as such IDs, the folders in order. An
  ID numbered like his folder of the other kind means he wears no custom item of the kind,
  as before (an info line now). A folder nobody wears left beside an ID nothing pairs stops
  the upgrade, since nothing says which folder is his.
- **Export upgrader** — a folder holding one texture as both a `.dds` and a `.ftex` no longer
  stops the upgrade: the `.ftex` is dropped with a warning, or both files when no model uses
  the texture.
- **Export upgrader** — a model whose every vertex sits more than 5000 units from the
  origin, the way old templates hid a default face, hair or glove, is dropped with a warning
  instead of costing the player his folder when the export is compiled. A gloves or boots
  folder left with no model hides the hands or boots, as an empty one always did.
- **Export upgrader** — the anti-blur copies older Fox models carry (each mesh drawn a
  second time, which FoxDen's TAA removal made pointless) are removed from every `.fmdl`, with
  an info line naming the file.
- **Team compiler** — on PES 2018 to 2021, a model converted from a PES 2015-2017 `.model`
  gets no anti-blur copies any more, and model parts merged into one `.fmdl` are no longer
  refused when some carry anti-blur copies and others do not.

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
