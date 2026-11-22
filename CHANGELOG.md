# Changelog

## [Unreleased]

### Important

- **Export upgrader and Compiler**: the legacy mode that kept an old export's boots and gloves
  IDs is gone, with the upgrader's `--legacy-ids` option, which 0.0.2's `quick_upgrade.bat`
  passed. The Compiler no longer reads the `legacy_ids.toml` that mode wrote into upgraded
  exports: it warns about the file as about any unknown one, and gives their boots and gloves
  its own IDs. Delete the file from your exports.

### Useful

- **Save editor**: typing the start of a nation's name in the open Nation list now picks it,
  as typing its ID does, and Escape in the open list now puts back the nation it opened on.
- **All tools**: UTW now keeps a record of the five times it was opened before the current
  one, with the saves opened and written each time. Report a problem's new box `Include earlier sessions` adds
  them to the file, for a problem that began before UTW was last opened, such as a crash.
- **All tools**: when UTW stops on an error of its own, its dialog now asks for the log file
  first, with the file's path on a line of its own, and gives the error's details last.

### New checks

- **Compiler**: a `Full` export with no logo at its root is now the error `logo_missing`, and
  on PES 2017 to 2021 a player with no portrait is now the warning `portrait_missing`. In a
  `Midcup` export he is reported only when the game's installed files hold no portrait for
  him either.

### Fixes

- **Compiler**: a referee with no face model, whose body model brings his head, showed the
  game's own referee head floating over it. He now gets an empty face, as a team player in the
  same case does.
- **Save editor**: a save holding no cup team (ids 701 to 920), such as one made from the stock
  game, opened to an empty list with no reason given. It now shows a warning saying so, and the
  command line's `check-aatf`, `export-toml --all-teams` and `export-teams-list` refuse it.
- **Save editor**: in a window larger than the default, the screens spread their columns and
  stretched their panels across it. They now keep their default layout, centred, while the
  player list shows more rows and the Live AATF panel takes the whole width.
- **Ruleset editor**: in a window larger than the default, the toolbar, the bottom bar and the
  form stretched across it. They now keep their default width, centred. The preview now
  stands at the window's right edge, wide enough for the data block's 100 columns, and can no
  longer be resized.

### Other stuff

- **All tools**: the sidebar now marks the active tool with a thin outline around its
  highlight, in place of the bar at its left.
- **Save editor**: a PES 2017 Texport now carries the 4cc clover as its thumbnail, in place of
  the game's own picture.
- **Export upgrader**: an export upgraded as `Full` with no logo in its `Logo/` folder is now
  the error `logo_missing`, and nothing is written.

## [0.0.2] - 2026-11-18

### Important

- **The Compiler**: added as a first preview, for the /vg/ League 27 on PES 2019.
  It handles a new format of aesthetics export, which includes the savefiles' aesthetics
  settings, and, in addition to CPKs, prepares a savefile patch file with all of the
  aesthetics settings, ready for applying to the official save. More details about the new
  format will be publicized later. This tool's window comes later: to use the compiler run
  `quick_compile.bat`, which keeps the whole report in `compile_log.txt`, or call the exe
  from the command line.
- **Export upgrader**: added as a first preview.
  It upgrades exports made in the old format to the one the Compiler reads, and leaves the
  old export as it was. Its window comes later too: drop an official save onto
  `quick_upgrade.bat` to upgrade every export in `old_exports`, or call the exe from the
  command line.
- **All tools**: added a "Report a problem" button, near the bottom of the sidebar, which
  saves a file to post with a report of something that went wrong. The report includes the
  log, your settings and your own words, and what you did in the Save editor with the files
  it opened and wrote. This helps with reproducing and fixing the issue.
  For now, information is lost when you close the program, so keeping it open while you work
  until you're finished is recommended, in case any issues arise in PES.
- **Save editor**: added a toggle, placed to the left of the player's or team's name, which
  switches between three screens: Stats, Appearance, and Gameplan.
  Appearance, previously a collapsed section under the stats, is now a screen of its own.
- **Save editor**: a Team TOML export now opens by saying what the file is, and that a key
  left out of it changes nothing on import. A player's shirt number is now `shirt_number`,
  since `number` read as his ID. The file's numbers now read as the game and the Save
  editor show them. Form, injury resistance and weak foot are one higher than before. The
  starting eleven, the bench, the set-piece takers, the captain and an instruction's target
  count roster slots from 1, as the `[players.NN]` tables do, and a marked opponent is his
  starter, 1 to 11; 0 means nobody. The tactics' style keys now use the game's names. A
  file exported by 0.0.1 reads these values wrongly: export it again.
- **Save editor**: the values in General info and Other stats have been rearranged so that
  General info only holds non-tactical values (name, age, etc.) and the new Main stats panel,
  which replaces Other stats, holds the rest. This was required to optimize the upcoming
  Squad view, a grid showing every player in a team.

### Useful

- **Save editor and Ruleset editor**: the sidebar's PES version box is now empty and disabled
  while either tool is shown, since it's not used by either of them.
- **All tools**: wheel down now moves a slider or a toggle to the right, and a dropdown to
  the item below, with the inverted scrolling setting on or off. That setting, renamed to
  Invert number scrolling, now applies to number boxes only, the box beside a slider
  included. If you had Invert value scrolling ticked, tick the new one in Settings.
- **Save editor**: the recent saves are kept between runs, the last 10, each named by
  its version, folder and file name (`[PES 2021] backups\EDIT00000000 - VGL26`).
- **Save editor and Ruleset editor**: the toolbar is now a menu bar, with `Open…`, `Save` and
  `Save as…` in the File menu, and Undo and Redo in the Edit menu.
  In the Save editor, the File menu starts with `Open…` (Ctrl+O, until now `Choose file…`),
  `Save` and `Save as…`, which now has Ctrl+Shift+S. The saves found in Documents and the
  recent saves follow, each under its own heading. Below them are `Close save` (Ctrl+W), which
  closes the save and brings back the list of saves, and `Exit` (Ctrl+Q), which closes the
  window. Its Edit menu also has Copy and Paste, which do the same thing as pressing Ctrl+C
  and Ctrl+V.
- **Updates**: the "new version available" dialog now has an Update now button, which
  downloads and installs the update. The program mentions newer versions when run in the
  command line as well. The window checks each time it starts and every 5 minutes while it
  stays open, so the setting for the time between checks is gone.
- **Save editor**: radio buttons at the bottom of the team dropdown narrow the list to the
  4cc, VGL or Invitational teams.
- **Save editor**: Copy stats and Swap stats are now one window, `Copy/swap stats`
  (`Copy/swap stats…` in the Player menu), which stays open: pick the source once, then
  click each player in the list to copy onto.
- **Save editor**: Ctrl+V now pastes what Ctrl+C copied (a player's stats, a Gameplan, a
  tactics preset or a formation) into another Save editor window too, and even into a save of
  another PES version opened in the same window.
- **Save editor**: the status bar now says what Save, Save as, Output rosters to TSV, Ctrl+C
  and Ctrl+V did after you use them.
- **Save editor**: each row of the unsaved changes list and of the close prompt now has an ×,
  which puts that one change back. In the list, a click on a row's item now switches the
  screen to its player or team. A player dragged in the player list now shows there as a
  single `Player order` row, instead of a bunch of individual stat changes.
- **Save editor**: added the button `Save a picture of the formations…` to the Team menu:
  it saves a PNG of the team's formations, a row per preset, to check its shapes or share
  them, optionally with each preset's settings beside its row; the notice that follows has an
  Open folder button.
- **Save editor**: added the button `Save formation pictures for a range of teams…` to the
  Savefile menu: it saves a formation picture for every cup team between the picked two, into
  one folder. Clicking Stop ends the run and keeps the pictures already saved.
- **Save editor**: added a Portraits checkbox above the pitch image: unticking it hides
  the portraits so the formation's shape is easier to see.
- **Save editor**: the Player menu's FPC button now says what Ctrl+I will do: `Hide default
  body (Enable FPC)` or `Show default body (Disable FPC)`.
- **Save editor**: a team's Stadium ID can now be edited on every version and its Manager ID
  from PES 2017 on, and Team TOML files now carry them.
- **Save editor**: on a PES 2017 to 2021 save, Numbers in attack and Numbers in defence are
  now named Auto attack level and Auto defence level, since the game is said to use them only
  with the automatic attack and defence levels. They can now be changed only when the ruleset
  allows illegal formations and the team's Attack/defence levels setting, in Automatic
  settings, is enabled.
- **Ruleset editor**: the Event logo can now be a JPEG XL file (`.jxl`).
- **Help**: a What's new chapter now holds this list.
- **Save editor**: the Appearance screen now fits the window without scrolling, and its
  settings are divided into titled groups.
- **Save editor**: Star rating and Playing attitude now count as part of a player's look: his
  team's export carries them, Copy/swap stats leaves them as they were, and the Comparator
  lists them, and every motion, under Aesthetics. A team TOML file now keeps them under
  `appearance.presentation`, as `star_rating` and `playing_attitude`.
- **Save editor**: Nation is now a dropdown of the nations the save's PES version has, each
  written after its ID; typing an ID in the open list picks it.
- **Save editor**: a Pitch | Left | List toggle under the Playable positions heading shows the
  13 positions as they stand on the pitch, as the same rows from one left edge, or one
  position a row, and the choice is remembered.
- **Save editor**: with no save open, a new welcome popup shows up, which offers `Open…`,
  the saves found in Documents (the newest of each version with its shortcut) and the
  recent saves.
- **Save editor**: in `Team file interchange` (the Team menu's `Import / export team
  file…`), a new button under `Import into` selects the team the picked file was exported
  from, so you can import back onto it without looking for it in the list.

### New checks

- **Save editor**: on a PES 2017 or PES 2019 save, Live AATF and `check-aatf` now warn about a
  formation the game's own screens cannot make, naming the preset, the formation and the
  players, unless the ruleset allows illegal formations.

### Fixes

- **Save editor**: the Attacking zone setting, now named Attacking area, showed Centre and
  Wide the wrong way round, in the editor and in Team TOML files: a team set to Wide played
  Centre in the game. It now matches the game.
- **Save editor**: a PES 2017 Texport written and imported into an invitational slot showed
  nobody in the kick-off screen's Starting Lineup, because the manager ID wasn't being
  written. Texports now include the manager ID, as the game writes them.
- **Settings**: with two windows open, a setting changed in one is no longer undone when the
  other changes one of its own.
- **Save editor**: the unsaved changes list and the close prompt no longer scroll slowly
  after several teams are imported.
- **Save editor**: a starter's overall rating on the Tactics pitch could be up to 7 off PES
  2017's when his Kicking Power and Swerve, or his Body Control and Physical Contact, were far
  apart. It now matches.

### Minor fixes

- **Save editor**: the Wrist tape left and right dropdowns now list the game's four colors,
  Strip color, White, Black and Beige, where they offered the numbers 0 to 7. An export's
  `settings.toml` names the four in its comment, and the Compiler refuses 4 to 7 there,
  which the game never offers.
- **Save editor**: `apply-patch` writing over its own save with no player to apply no longer
  replaces the previous `.bak`.
- **Save editor**: on the Gameplan screen, a player dragged onto another in the bench list
  swaps places with him, as in the starting eleven, instead of shifting the players in
  between.
- **All tools**: resizing the window no longer makes its contents jump on every step of
  the drag.
- **Save editor**: Up and Down now walk the player list again after Page Up or Page Down left
  a Tactics slot popover open.
- **Save editor**: after opening another save, medal outlines and Cards used badges no longer
  show the previous save's for a moment.
- **Sidebar**: in a short window, the tool entries now scroll, and Report a problem, Help and
  Settings now stay pinned below them.
- **All tools**: on Windows, a Shift+click or a middle click on the program's taskbar button
  now opens another window properly.
- **All tools**: a click between two entries of a menu or dropdown list no longer closes it
  with nothing picked.
- **All tools**: menus, dialogs, dropdown lists, text fields and the update's progress bar
  now share one corner rounding again.
- **Save editor**: an event logo that opens the event's page is now framed like a button while
  the pointer is on it, as the 4cc logo is.
- **Save editor**: Undo, Redo and the × of a `Player order` row now keep a moved player
  selected, instead of switching to whoever took his place in the list.
- **Save editor**: the unsaved changes list now grows as wide as its longest row, up to the
  window's width, instead of stopping at 640 points.
- **Save editor**: the filter box now matches an ID from its start, so `820` lists team 820
  and its squad, instead of every player whose ID holds 820 somewhere.
- **Save editor**: Page Up and Page Down now bring the team row they select to the top of the
  list, with its players under it.
- **Save editor**: on the Gameplan screen, a held player rated A or B at no position now
  darkens the whole pitch, as in the game.

### Other stuff

- **Command line**: `--log-file <FILE>` now keeps everything a run prints in a file, a crash's
  message included, and leaves Info messages out of the console.
- **All tools**: an open dropdown list can now hold more items before it scrolls.
- **All tools**: switched to the GitHub color palette: warnings are now amber instead of
  orange and errors a softer red, so the two stand apart. The success green and the link blue
  changed with them, and on the light theme links are easier to read.
- **All tools**: checkboxes, radio buttons, number boxes, dropdowns, toggles and buttons now
  have a thin outline, so an empty box or the unpicked side of a toggle no longer blends into
  the background.
- **All tools**: on the light theme the background is now a shade darker, so boxes and buttons
  stand off it, and a number box now looks like a text box.
- **Save editor**: a Team TOML imported onto another team now keeps that team's own manager,
  instead of giving it the manager of the team the file came from. A file naming any other
  manager still sets him.
- **Save editor**: a starter moved on the Tactics pitch now stays inside the area the game's
  Formation screen allows, so an edit can no longer make a formation Live AATF warns about.
- **Save editor**: a player dragged in the player list now moves with the pointer, and a line
  shows where he will land.
- **Save editor**: on PES 2015 and PES 2016, the Substitution dropdown now also offers Very
  early, as those games' own screens do.
- **Save editor**: exporting a Team TOML now suggests the team's name for the file (`f.toml`
  for /f/) instead of the team's ID.
- **Save editor**: the Comparator now shows each difference in three columns: Item, Picked
  save and Open save.
- **Save editor**: the portraits and Set PES path now follow the open save's PES version, not
  the sidebar's, and no warning says the two differ. The file picker starts in the folder of
  the last save opened.
- **Save editor**: while a row of the starting eleven or the bench is dragged over a
  player on the Tactics pitch, that player is now marked as a target, so you see who the drop
  swaps out.
- **Save editor**: picking the selected player's own team in the team dropdown now keeps him
  selected, instead of selecting the team's row.
- **Save editor**: in General info, Shirt now reads Shirt name, and in the team view Short,
  Col. 1 and Col. 2 now read Short name, Color 1 and Color 2.
- **Save editor**: the formation settings now use the game's names.
- **Save editor**: the player list's team rows now stand out more from its player rows.
- **Save editor**: the column heads of the unsaved changes list are no longer dimmed.
- **Save editor**: the text of `Team file interchange` is no longer dimmed.
- **Save editor**: the name of a field which the save's PES version lacks is now dimmed with
  its control, and hovering it names the first version that has the field.
- **Save editor**: Live AATF's mark on a panel of the Stats screen now stands after its title,
  and on Skills after the Cards used badge.
- **Save editor**: in a Team TOML export, the comment beside `skills` now says how a skill's
  name is spelt.
- **Save editor**: the note about wrist or ankle taping under a hidden body now says to take
  the taping off unless it is meant to show up.
- **Save editor and Ruleset editor**: the name color findings, the ruleset summary and the
  hair color comment in `settings.toml` now spell "color", as the rest of the program does.


## [0.0.1] - 2026-11-03

- **Save editor**: the first preview, for the DBC7's managers on PES 2017: players, teams and
  tactics, the aesthetics patch, the roster sheet, Live and Multi-team AATF, the Comparator,
  undo and redo, the unsaved-changes list, and a command line for batch work.
- **Ruleset editor**: a work-in-progress look at the new cup rules format (4ccrules).
