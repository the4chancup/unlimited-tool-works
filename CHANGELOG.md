# Changelog

What changed for members, release by release, newest first.

## [Unreleased]

### New

- **Team compiler and Export upgrader** — in the bundle again through `quick_compile.bat`
  and `quick_upgrade.bat`, with the `exports/` and `old_exports/` folders, for the /vg/
  League 27 on PES 2019.

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
