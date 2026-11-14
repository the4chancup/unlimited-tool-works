# Changelog

## [Unreleased]

### Important

- **Team compiler**: the first preview, for /vg/ League 27 on PES 2019. It checks the teams'
  aesthetics exports, compiles them into the CPKs the game loads, and writes the aesthetics
  patch that carries their settings into the official save. Its window comes later: run
  `quick_compile.bat`, which keeps the whole report in `compile_log.txt`, or its command line.
- **Export upgrader**: the first preview. It upgrades exports made in the old format to the
  one the Team compiler reads, and leaves the old export as it was. Its window comes later
  too: drop the official save onto `quick_upgrade.bat` to upgrade every export in
  `old_exports`, or use its command line.
- **Save editor**: lists only the cup's teams (IDs 701 to 920) and their players, and Fix
  database repairs only those; the game's own clubs and national teams stay in the save
  untouched. On the command line, `diff` and `export-toml --all-teams` do the same.
- **Save editor and Ruleset editor**: the toolbar is a menu bar. Open, Save and Save as are
  in the **File** menu, Undo and Redo in the **Edit** menu, with the same shortcuts.
- **Save editor**: a toggle left of the player's or team's name switches the right side
  between three screens: Stats, Appearance, and Gameplan for the team's fields and tactics.
  Appearance, until now a collapsed section under the stats, is a screen of its own.
- **Updates**: the offer of a newer version has an Update now button, which downloads and
  installs it, and a command typed in a terminal mentions a newer version too. The Help
  window's new Updates chapter explains each step.

### Useful

- **Save editor**: the team dropdown above the list reads All teams in place of ALL, and
  radio buttons at the bottom of the open dropdown narrow it and the list to the 4cc, VGL or
  Invitational teams.
- **Save editor**: Copy stats and Swap stats are one window, `Copy/swap stats…`, which stays
  open: pick the source once, then select in the list each player to copy onto.
- **Save editor**: Ctrl+V pastes what Ctrl+C copied (a player's stats, a Gameplan, a tactics
  preset or a formation) in another Save editor window too, even one holding another PES
  version's save.
- **Save editor**: the status bar says what Save, Save as, Output rosters to TSV, Ctrl+C and
  Ctrl+V did, in semibold so it is easier to see.
- **Save editor**: each row of the unsaved changes list has an ×, which puts that one change
  back. A player dragged in the player list shows there as one `Player order` row.
- **Save editor**: a player dragged in the player list moves with the pointer, and a line
  shows where he will land.
- **Save editor**: the Tactics formation bar has a Portraits checkbox, which hides the
  portraits so the formation's shape is easier to see. `Scroll names` beside it is now
  `Scroll`.
- **Save editor**: the Player menu's FPC entry says what Ctrl+I will do: `Hide default body
  (Enable FPC)` or `Show default body (Disable FPC)`.
- **Save editor**: a team's Stadium ID can be edited on every version and its Manager ID from
  PES 2017 on, and Team TOML files carry them.
- **Save editor**: exporting a Team TOML suggests the team's name for the file (`f.toml` for
  /f/).
- **Save editor**: the Comparator shows each difference in three columns: Item, Picked save
  and Open save.
- **Save editor**: on a PES 2017 to 2021 save, Numbers in attack and Numbers in defence are
  disabled unless the ruleset allows illegal formations, since the game no longer lets them
  be set.
- **Command line**: `--log-file <FILE>` keeps everything a run prints in a file, a crash's
  message included, and leaves Info messages out of the console.
- **Ruleset editor**: the Event logo can be a JPEG XL file (`.jxl`).
- **Help**: a What's new chapter holds this list.
- **Save editor**: the Appearance screen fits the window without scrolling, its settings in
  titled groups. Full Player Customization comes first, with the skin and iris colors beside
  it, then the strip style in a grid of three columns, then Physique beside Motion and the
  edit flags. Star and Playing attitude close the screen in a Presentation group and, like
  the rest of the look, are locked while the team's export owns it. The strip's colors are
  dropdowns, and the Player gloves checkbox is an Outfielder gloves dropdown, Off or Worn in
  winter.
- **Save editor**: Star and Playing attitude count as part of a player's look: his team's
  export carries them, Copy/swap stats leaves them as they were, and the Comparator lists
  them, and every motion, under Aesthetics. A Team TOML keeps them under
  `appearance.presentation`, and one written before still opens.

### New checks

- **Save editor**: on a PES 2017 or PES 2019 save, Live AATF and `check-aatf` warn about a
  formation the game's own screens cannot make, naming the preset, the formation and the
  players, unless the ruleset allows illegal formations.

### Fixes

- **Save editor**: a PES 2017 Texport it wrote showed nobody in the kick-off screen's
  Starting Lineup. Texports are now written from the save's own records, as the game writes
  them.
- **Save editor**: `apply-patch` writing over its own save with no player to apply no longer
  replaces the previous `.bak`.
- **Settings**: with two windows open, a setting changed in one is no longer undone when the
  other changes one of its own.
- **Save editor**: in the Tactics tab's bench list, a player dragged onto another swaps
  places with him, as in the starting eleven, instead of shifting the players between.
- **Save editor**: a starter moved on the Tactics pitch stays inside the area the game's
  Formation screen allows, so an edit can no longer make a formation Live AATF warns about.

### Minor fixes

- **All tools**: warnings are amber instead of orange and errors a softer red, so the two
  stand apart. The success green and the link blue changed with them, and on the light
  theme links are easier to read.
- **Save editor**: a save is edited as its own version, whatever the sidebar shows: the
  portraits and Set PES path follow it, and no warning says it is not the sidebar's version.
  The file picker starts in the folder of the last save opened.
- **Save editor and Ruleset editor**: the sidebar's PES version box is empty and disabled
  while either is shown.
- **Save editor**: Up and Down walk the player list again after Page Up or Page Down left a
  Tactics slot popover open.
- **Save editor**: after opening another save, medal outlines and Cards used badges no longer
  show the previous save's for a moment.
- **Save editor**: an ability stat the save's version lacks is dimmed like its `-`.
- **Sidebar**: in a short window, the tool entries scroll, and Help and Settings stay pinned
  below them.
- **Status bar**: on a small screen, the Reset window button no longer stays after a click
  that changed nothing.
- **Windows**: a Shift+click or a middle click on the program's taskbar button opens another
  window.
- **General**: a click between two entries of a menu or dropdown list no longer closes it
  with nothing picked.
- **General**: menus, dialogs, dropdown lists, text fields and the update's progress bar
  share one corner rounding again.

## [0.0.1] - 2026-11-03

- **Save editor**: the first preview, for the DBC7's managers on PES 2017: players, teams and
  tactics, the aesthetics patch, the roster sheet, Live and Multi-team AATF, the Comparator,
  undo and redo, the unsaved-changes list, and a command line for batch work.
- **Ruleset editor**: a work-in-progress look at the new cup rules format (4ccrules).
