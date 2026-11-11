# Changelog

What changed for members, release by release, newest first.

## [Unreleased]

### New

- **Export upgrader and Team compiler** — VGL27 keeps the managers' own boots and gloves
  IDs: `quick_upgrade.bat` upgrades with `--legacy-ids`, which copies every boots or
  gloves folder into each player wearing it and records the ID its old name carried in
  `legacy_ids.toml`; the compiler packs those models under the old IDs, so a manager's
  Red test build and the official DLC agree.
- **Team compiler and Export upgrader** — in the bundle again through `quick_compile.bat`
  and `quick_upgrade.bat`, with the `exports/` and `old_exports/` folders, for the /vg/
  League 27 on PES 2019. An upgraded export names no team ID: its references into the
  team's Common read `team_common_folder/`, in its `.mtl` files and models alike, and a
  keeper's `u0XXXg1` textures become `gk_kit1`. The kit texture the modded exe swaps in is
  written `team_kit` (`team_gk_kit` for a keeper), which the compiler turns back into the
  `dummy_kit` the exe looks for; a model or `.mtl` still naming `dummy_kit` is refused, with
  a hint to run it through the Export upgrader. A team the current teams list no longer
  holds upgrades too, its roster found in the original save by the team's name.
- **Team compiler** — a model naming the team's Common by a team ID or the community's
  `000` placeholder is refused, with a hint to run it through the Export upgrader's
  `upgrade-models`.
- **Team compiler** — `check` and `compile` take `--timestamps`: each finding line starts
  with the seconds since the run started when the finding was made, for seeing where a slow
  compile spends its time.
- **Team compiler** — a file or folder name, or a path to one of the export's files, still
  spelled with the old kit token (`u0XXXp1`, `kit6_u0827p0.dds`, `Common/u0XXXp0/`) is
  refused, with a hint to run the export through the Export upgrader, which respells it
  `kit1`, `kitN` or `kitF`. Paths into the game's own folders keep the game's names.
- **Team compiler and Ruleset editor** — a texture can be a JPEG XL file (`.jxl`), which
  compiles exactly as the same picture saved as a PNG does, so no PNG copy is needed; the
  Ruleset editor's Event logo can be one too.
- **Save editor** — a team's Manager ID and Stadium ID can be seen and edited on PES 2017
  and 2018 too, not only from PES 2019 on, and a Team TOML carries them on those versions.

### Fixes

- **Save editor** — a PES 2017 Texport it wrote imported and listed its players in Edit, but
  the kick-off screen's "Starting Lineup" showed nobody. A Texport (PES 2017 to 2021) is now
  written from the save's own records, as the game writes one: the team keeps its manager and
  coach, and every value the editor does not show keeps the save's.

### Minor fixes

- **Export upgrader** — the progress line in a terminal counts one upgrade as one export
  (`0/1`, not `0/2`), and counts the files it writes, where only the clock moved.
- **Save editor** — an aesthetics patch's names now show in game on PES 2019: applying it
  marks each renamed player as edited, as typing the name does, where they showed
  PLACEHOLDER. `apply-patch` writing back over its own save with no player applied now
  leaves the save and its `.bak` as they were, instead of replacing the previous backup.
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
