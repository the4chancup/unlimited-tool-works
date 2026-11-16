# Changelog

## [Unreleased]

### Important

- **The Compiler**: the first preview, for /vg/ League 27 on PES 2019. It checks the teams'
  aesthetics exports, compiles them into the CPKs the game loads, and writes the aesthetics
  patch that carries their settings into the official save. Its window comes later: run
  `quick_compile.bat`, which keeps the whole report in `compile_log.txt`, or its command line.
- **Export upgrader**: the first preview. It upgrades exports made in the old format to the
  one the Compiler reads, and leaves the old export as it was. Its window comes later
  too: drop the official save onto `quick_upgrade.bat` to upgrade every export in
  `old_exports`, or use its command line.
- **Compiler**: its command line is `UTW.exe compiler`, and its settings sit under `[compiler]`
  in `settings.toml`. A `[team-compiler]` table left there by an earlier build is not read:
  move its settings under `[compiler]`.
- **Save editor**: lists only the cup's teams (IDs 701 to 920) and their players, and Fix
  database repairs only those; the game's own clubs and national teams stay in the save
  untouched. On the command line, `diff` and `export-toml --all-teams` do the same.
- **Save editor and Ruleset editor**: the toolbar is a menu bar. Open, Save and Save as are
  in the **File** menu, Undo and Redo in the **Edit** menu, and their existing shortcuts
  still work.
- **Save editor**: a toggle left of the player's or team's name switches the right side
  between three screens: Stats, Appearance, and Gameplan for the team's fields and tactics.
  Appearance, until now a collapsed section under the stats, is a screen of its own.
- **Save editor**: in a team file, a player's shirt number is now `shirt_number`, since
  `number` read as his ID. A team file exported by 0.0.1 is refused: export the team again, or
  rename each `number` under a `[players.NN]` heading to `shirt_number` (a kit slot's `number`
  stays as it is).
- **Save editor**: a team file exported by 0.0.1 from a PES 2019, 2020 or 2021 save is
  refused, because it holds the star rating (and from PES 2020 the playing attitude) beside
  the stats, where a team file no longer has them. Export the team again, or delete the
  `star` and `playing_attitude` lines under each `[players.NN.stats]` heading, which leaves
  those two values as they are in the save.
- **Updates**: the offer of a newer version has an Update now button, which downloads and
  installs it, and a run of `quick_compile.bat`, `quick_upgrade.bat` or another command
  mentions a newer version too. The Help
  window's new Updates chapter explains each step. The window checks each time it starts and
  every 5 minutes while it stays open, so the setting for the time between checks is gone: a
  `check_interval_minutes` line left in `settings.toml` does nothing.
- **All tools**: wheel down now moves a slider or a toggle to the right, and a dropdown to
  the item below, with the scrolling setting on or off. That setting, renamed Invert number
  scrolling, now turns number boxes only, the box beside a slider included: if you had
  Invert value scrolling ticked, tick the new one in Settings (an `invert_value_scrolling`
  line left in `settings.toml` does nothing).

### Useful

- **All tools**: an open dropdown list shows half as many items again before it scrolls.
- **Save editor**: the team dropdown above the list reads All teams in place of ALL, and
  radio buttons at the bottom of the open dropdown narrow it and the list to the 4cc, VGL or
  Invitational teams.
- **Save editor**: Copy stats and Swap stats are one window, `Copy/swap stats`
  (`Copy/swap stats…` in the Player menu), which stays open: pick the source once, then
  click each player in the list to copy onto.
- **Save editor**: Ctrl+V pastes what Ctrl+C copied (a player's stats, a Gameplan, a tactics
  preset or a formation) in another Save editor window too, and after another save is
  opened, even a save of another PES version.
- **Save editor**: under Undo and Redo, the Edit menu has Copy and Paste, which do what Ctrl+C
  and Ctrl+V do.
- **Save editor**: the status bar says what Save, Save as, Output rosters to TSV, Ctrl+C and
  Ctrl+V did, in semibold so it is easier to see.
- **Save editor**: each row of the unsaved changes list and of the close prompt has an ×,
  which puts that one change back. In the list, a click on a row's item goes to its player
  or team. A player dragged in the player list shows there as one `Player order` row.
- **Save editor**: a player dragged in the player list moves with the pointer, and a line
  shows where he will land.
- **Save editor**: the Team menu's `Save a picture of the formations…` saves a PNG of the
  team's formations, a row per preset, to check its shapes or share them, optionally with
  each preset's settings beside its row; the notice that follows has an Open folder button.
- **Save editor**: the Savefile menu's `Save formation pictures for a range of teams…` saves
  that picture for every cup team between two you pick, into one folder. Stop ends the run
  and keeps the pictures already saved.
- **Save editor**: the Tactics formation bar has a Portraits checkbox: unticking it hides
  the portraits so the formation's shape is easier to see. `Scroll names` beside it is
  now `Scroll`.
- **Save editor**: the Player menu's FPC entry says what Ctrl+I will do: `Hide default body
  (Enable FPC)` or `Show default body (Disable FPC)`, and the Team menu's two FPC entries
  use the same names.
- **Save editor**: a team's Stadium ID can be edited on every version and its Manager ID from
  PES 2017 on, and Team TOML files carry them.
- **Save editor**: exporting a Team TOML suggests the team's name for the file (`f.toml` for
  /f/).
- **Save editor**: the Comparator shows each difference in three columns: Item, Picked save
  and Open save.
- **Save editor**: on a PES 2017 to 2021 save, Numbers in attack and Numbers in defence are
  named Auto attack level and Auto defence level, since the game is said to use them only
  with the automatic attack and defence levels. They can be changed only when the ruleset
  allows illegal formations and the team's Attack/defence levels, in Automatic settings,
  is on.
- **Command line**: `--log-file <FILE>` keeps everything a run prints in a file, a crash's
  message included, and leaves Info messages out of the console.
- **Ruleset editor**: the Event logo can be a JPEG XL file (`.jxl`).
- **Help**: a What's new chapter holds this list.
- **Save editor**: the Appearance screen fits the window without scrolling, its settings in
  titled groups. Full Player Customization comes first, with the skin and iris colors beside
  it, then the strip style in a grid of three columns, then Physique, its values on sliders,
  beside Motion and the edit flags. Star rating and Playing attitude close the screen in a
  Presentation group and, like the rest of the look, are locked while the team's export owns
  it. The strip's colors are dropdowns, and the Player gloves checkbox is an Outfielder gloves
  dropdown, Off or Worn in winter.
- **Save editor**: Star rating and Playing attitude count as part of a player's look: his
  team's export carries them, Copy/swap stats leaves them as they were, and the Comparator
  lists them, and every motion, under Aesthetics. A team file keeps them under
  `appearance.presentation`, as `star_rating` and `playing_attitude`.
- **Save editor**: a Pitch | Left | List toggle under the Playable positions heading shows the
  13 positions as they stand on the pitch, as the same rows from one left edge, or one
  position a row, and the choice is remembered. The letters of the ratings not chosen are
  darker, so the chosen one stands out.
- **Save editor**: a Team TOML now opens by saying that a key left out of it changes nothing
  on import, and its stats table says the game shows weak foot, form and injury resistance
  one higher than the file.
- **Export upgrader**: the comments of a kit's `config.toml` start at one column, so a
  two-digit value no longer pushes its comment out of line.
- **Save editor**: with no save open, the middle of the window offers `Open…` (Ctrl+O),
  the saves found in Documents (the newest of each version with its shortcut) and the
  recent saves.
- **Ruleset editor**: the toolbar stays above the start page, so its File menu's Open… is
  there before a ruleset is open.
- **Save editor**: while a row of the starting eleven or the bench is dragged over a
  player on the Tactics pitch, that player is marked as a pitch drag marks its target, so
  you see who the drop swaps out.
- **Save editor**: in `Team file interchange` (the Team menu's `Import / export team
  file…`), a button under `Import into` selects the team the picked file was exported
  from, so you can import back onto it without looking for it in the list.
- **All tools**: Report a problem, in the sidebar above Help, saves a file to post with a
  report of something that went wrong: the log, your settings and your own words, and what
  you did in the Save editor with the files it opened and wrote (the other tools join
  later). Nothing is sent: you post the file.
- **Save editor**: the File menu starts with `Open…` (Ctrl+O, until now `Choose
  file…`), `Save` and `Save as…`, which now has Ctrl+Shift+S. The saves found in
  Documents and the recent saves follow, each under its own heading. Below them are
  `Close save` (Ctrl+W), which closes the save and brings back the list of saves, and
  `Exit` (Ctrl+Q).
- **Save editor**: the recent saves are kept between runs, the last 10, each named by
  its version, folder and file name (`[PES 2021] backups\EDIT00000000 - VGL26`).
- **All tools**: Ctrl+Q closes the window, asking first about unsaved changes, as its
  close button does.

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
- **Save editor**: on the Gameplan screen, a player dragged onto another in the bench list
  swaps places with him, as in the starting eleven, instead of shifting the players in
  between.
- **Save editor**: a starter moved on the Tactics pitch stays inside the area the game's
  Formation screen allows, so an edit can no longer make a formation Live AATF warns about.
- **Save editor**: the Wrist tape left and right dropdowns list the game's four colors,
  Strip color, White, Black and Beige, where they offered the numbers 0 to 7. An export's
  `settings.toml` names the four in its comment, and the Compiler refuses 4 to 7 there,
  which the game never offers.
- **Save editor**: the unsaved changes list and the close prompt no longer scroll slowly
  after several teams are imported.
- **Save editor**: on PES 2015 and PES 2016, the Substitution dropdown also offers Very
  early, as those games' own screens do.
- **Save editor**: a Team TOML imported onto another team keeps that team's own manager,
  where it gave it the manager of the team the file came from. A file naming any other
  manager still sets him.

### Minor fixes

- **All tools**: warnings are amber instead of orange and errors a softer red, so the two
  stand apart. The success green and the link blue changed with them, and on the light
  theme links are easier to read.
- **Save editor**: the portraits and Set PES path follow the open save's PES version, not
  the sidebar's, and no warning says the two differ. The file picker starts in the folder of
  the last save opened.
- **Save editor and Ruleset editor**: the sidebar's PES version box is empty and disabled
  while either is shown.
- **Save editor**: Up and Down walk the player list again after Page Up or Page Down left a
  Tactics slot popover open.
- **Save editor**: after opening another save, medal outlines and Cards used badges no longer
  show the previous save's for a moment.
- **Save editor**: an ability stat the save's version lacks is dimmed like its `-`.
- **Sidebar**: in a short window, the tool entries scroll, and Report a problem, Help and
  Settings stay pinned below them.
- **Status bar**: on a small screen, the Reset window button no longer stays after a click
  that changed nothing.
- **Windows**: a Shift+click or a middle click on the program's taskbar button opens another
  window.
- **General**: a click between two entries of a menu or dropdown list no longer closes it
  with nothing picked.
- **General**: menus, dialogs, dropdown lists, text fields and the update's progress bar
  share one corner rounding again.
- **Save editor**: an event logo that opens the event's page is framed like a button while
  the pointer is on it, as the 4cc logo is.
- **Save editor**: a Team TOML export opens with a line saying what the file is, and its
  stats and instructions tables each carry a short note.
- **Export upgrader**: a kit `config.toml`'s comments sit at one column whatever the
  value's width.
- **Save editor**: the note about wrist or ankle taping under a hidden body says to take
  the taping off unless it is meant to show.
- **Save editor**: in a Team TOML export, the comment beside `skills` says how a skill's
  name is spelt.
- **Save editor and Ruleset editor**: the name color findings, the ruleset summary and the
  hair color comment in `settings.toml` spell "color", as the rest of the program does.
- **Save editor**: on the light theme, a white or pale color code in a team's or player's
  name is drawn darker, so the name stays readable on the light background.
- **Save editor**: the column heads of the unsaved changes list are no longer dimmed.
- **Save editor**: Undo, Redo and the × of a `Player order` row keep a moved player
  selected, instead of whoever took his place in the list.
- **Save editor**: the unsaved changes list grows as wide as its longest row, up to the
  window's width, instead of stopping at 640 points.
- **Save editor**: the filter box matches an ID from its start, so `820` lists team 820 and
  its squad, no longer every player whose ID holds 820 somewhere.
- **Save editor**: Page Up and Page Down bring the team row they select to the top of the
  list, with its players under it.
- **Save editor**: the player list's team rows stand out more from its player rows, on both
  themes.
- **Save editor**: on the Gameplan screen, a held player rated A or B at no position
  darkens the whole pitch, as in the game, where nothing was darkened.
- **Save editor**: on the Gameplan screen, the panels and the two lineup lists stand 8
  points apart, as on the Appearance screen.
- **Ruleset editor**: the section list on the left has a little more room between its
  rows.

## [0.0.1] - 2026-11-03

- **Save editor**: the first preview, for the DBC7's managers on PES 2017: players, teams and
  tactics, the aesthetics patch, the roster sheet, Live and Multi-team AATF, the Comparator,
  undo and redo, the unsaved-changes list, and a command line for batch work.
- **Ruleset editor**: a work-in-progress look at the new cup rules format (4ccrules).
