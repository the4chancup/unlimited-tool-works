# Changelog

What changed for members, release by release, newest first.

## [Unreleased]

### New

- **Export upgrader and Team compiler**: VGL27 keeps the managers' own boots and gloves
  IDs: `quick_upgrade.bat` upgrades with `--legacy-ids`, which copies every boots or
  gloves folder into each player wearing it and records the ID its old name carried in
  `legacy_ids.toml`; the compiler packs those models under the old IDs, so a manager's
  Red test build and the official DLC agree.
- **Team compiler and Export upgrader**: in the bundle again through `quick_compile.bat`
  and `quick_upgrade.bat`, with the `exports/` and `old_exports/` folders, for the /vg/
  League 27 on PES 2019. An upgraded export names no team ID: its references into the
  team's Common read `team_common_folder/`, in its `.mtl` files and models alike, and a
  keeper's `u0XXXg1` textures become `gk_kit1`. The kit texture the modded exe swaps in is
  written `team_kit` (`team_gk_kit` for a keeper), which the compiler turns back into the
  `dummy_kit` the exe looks for; a model or `.mtl` still naming `dummy_kit` is refused, with
  a hint to run it through the Export upgrader. A team the current teams list no longer
  holds upgrades too, its roster found in the original save by the team's name.
- **Team compiler and Export upgrader**: `quick_compile.bat` and `quick_upgrade.bat` show
  warnings and errors only, and write the whole report, Info messages included, to
  `compile_log.txt` and `upgrade_log.txt` beside them. On the command line, `--log-file
  <FILE>` does the same for any tool's command: everything the run prints is added to the
  file, and Info messages are left out of the console. If the run crashes, the crash's
  message is in the file too.
- **Export upgrader**: `upgrade` takes several exports in one run, or every export of a
  folder with `--all <folder>`. Each is upgraded as if alone, under a line naming it; one
  that is refused says why and does not stop the others, and the run ends with how many
  were upgraded. `quick_upgrade.bat` runs this way, so its progress line counts all the
  exports in `old_exports` (`12/52 exports`) and not one at a time.
- **Team compiler**: a model naming the team's Common by a team ID or the community's
  `000` placeholder is refused, with a hint to run it through the Export upgrader's
  `upgrade-models`.
- **Team compiler**: `check` and `compile` take `--timestamps`: each finding line starts
  with the seconds since the run started when the finding was made, for seeing where a slow
  compile spends its time.
- **Team compiler**: a file or folder name, or a path to one of the export's files, still
  spelled with the old kit token (`u0XXXp1`, `kit6_u0827p0.dds`, `Common/u0XXXp0/`) is
  refused, with a hint to run the export through the Export upgrader, which respells it
  `kit1`, `kitN` or `kitF`. Paths into the game's own folders keep the game's names.
- **Team compiler and Ruleset editor**: a texture can be a JPEG XL file (`.jxl`), which
  compiles exactly as the same picture saved as a PNG does, so no PNG copy is needed; the
  Ruleset editor's Event logo can be one too.
- **Updates**: the dialog that offers a newer version has Update now: it downloads the
  version, checks it and puts it in the program's folder, showing its progress, with Cancel
  beside it. Restart now then starts the new version; Later leaves `Update ready` on the
  status bar until the next start. When an update fails, the dialog says whether anything
  was changed and where to download the version by hand. The first start of a new version
  shows `Updated to` and its number on the status bar, with What's new beside it. The
  launchers and the tools' commands typed in a terminal also tell of a newer version, in
  one line after their report, without waiting for the check. Cancel stops an update at
  any moment before the new files are put in place, and a window whose program another
  window has already updated says so instead of updating it a second time.
- **Help**: two new chapters: Updates explains the check, the offer and each step of an
  update, and What's new holds this list of changes, version by version.
- **Save editor**: the Comparator lists each difference in three columns, Item, Old value
  and New value, under one head row, as the unsaved changes list does; a value too long for
  its column is cut, and hovering the row shows it whole.
- **Save editor**: a player dragged in a team's player list moves with the pointer, as a
  dragged lineup row does.
- **Save editor**: a team's Stadium ID can be seen and edited on every version and its
  Manager ID from PES 2017 on, not only from PES 2019 on, and a Team TOML carries them
  wherever the save stores them.
- **Team compiler**: no texture is encoded as BC7 any more: it is BC1 or BC3 on every
  version, and a `.dds` or `.ftex` already in BC7 keeps its blocks. A `.dds` or `.ftex`
  stored uncompressed is written uncompressed now, except a kit's main texture on PES
  2015 to 2017 and a normal map, and on those versions such a texture compiles at any
  size.

### Fixes

- **Save editor**: a PES 2017 Texport it wrote imported and listed its players in Edit, but
  the kick-off screen's "Starting Lineup" showed nobody. A Texport (PES 2017 to 2021) is now
  written from the save's own records, as the game writes one: the team keeps its manager and
  coach (a Manager ID changed in the editor brings that manager's coach), and every value the
  editor does not show keeps the save's.
- **Settings**: with two windows of the program open, a setting changed in one is no longer
  undone when the other saves something else: each window writes only the settings it
  changed itself. A window shows what another one changed once it is started again.
- **Windows**: a Shift+click or a middle click on the program's taskbar button opens another
  window, as it does for most programs. It did nothing unless the program was pinned.

### Minor fixes

- **Export upgrader**: the progress line in a terminal counts one upgrade as one export
  (`0/1`, not `0/2`), and counts the files it writes, where only the clock moved.
- **Export upgrader**: the comments in a kit's `config.toml` give the ranges the game
  takes: a collar's for each PES version (up to 131 on PES 2021) and 0-15 for the chest
  number's `y`, where they said 1-107 and 0-7.
- **Save editor**: an aesthetics patch's names now show in game on PES 2019: applying it
  marks each renamed player as edited, as typing the name does, where they showed
  PLACEHOLDER. `apply-patch` writing back over its own save with no player applied now
  leaves the save and its `.bak` as they were, instead of replacing the previous backup.
- **Save editor**: Up and Down walk the player list again after a page key left a team's
  Tactics slot popover open behind a player: they did nothing until the team was selected
  again and the popover closed.
- **Save editor**: after opening another save, a team's medal outlines and a player's Cards
  used badge no longer show the previous save's for a moment: they wait for the new save's own
  answer.
- **Save editor**: the name of an ability stat the save's PES version does not have
  (Tight Possession and Aggression before PES 2020, for one) is dimmed like its `-`, so it
  no longer reads as a field left to fill.

## [0.0.1] - 2026-11-03

- **Save editor**: the first preview, for the DBC7's managers on PES 2017: players, teams and
  tactics, the aesthetics patch, the roster sheet, Live and Multi-team AATF, the Comparator,
  undo and redo, the unsaved-changes list, and a command line for batch work.
- **Ruleset editor**: a work-in-progress look at the new cup rules format (4ccrules).
