# Changelog

What changed for members, release by release, newest first.

## [Unreleased]

### New

- **Command line**: `--log-file <FILE>` on any tool's command adds everything the run
  prints to the file and leaves Info messages out of the console. If the run crashes,
  the crash's message is in the file too.
- **Ruleset editor**: the Event logo can be a JPEG XL file (`.jxl`).
- **Updates**: the dialog that offers a newer version has an Update now button: it
  downloads the version, checks it and puts it in the program's folder, showing its
  progress, with Cancel beside it. Cancel stops the update at any moment before the new
  files are put in place. Once they are, Restart now starts the new version; Later leaves
  `Update ready` on the status bar until the next start. When an update fails, the dialog
  says whether anything was changed and where to download the version by hand. The first
  start of a new version shows `Updated to` and its number on the status bar, with What's
  new beside it. With two windows open, once one has updated the program, the other says
  so when asked to update instead of updating it a second time. A tool's command
  typed in a terminal also mentions a newer version, in one line after its report,
  without waiting for the check.
- **Help**: two new chapters: Updates explains the check, the offer and each step of an
  update, and What's new holds this list of changes, version by version.
- **Save editor**: the Comparator lists each difference in three columns, Item, Old value
  and New value, under one head row, as the unsaved changes list does; a value too long for
  its column is cut, and hovering the row shows it whole.
- **Save editor**: a player dragged in a team's player list moves with the pointer, as a
  dragged lineup row does, and the list shows a line where he will land instead of an
  outline round a row.
- **Save editor**: a team's Stadium ID can be seen and edited on every version and its
  Manager ID from PES 2017 on, not only from PES 2019 on, and a Team TOML carries them
  wherever the save stores them.
- **Save editor and Ruleset editor**: the toolbar is a menu bar. In the Save editor the
  Open menu, Save and Save as are now in a **File** menu, and Undo and Redo in an **Edit**
  menu; every shortcut works as before. The event logo is always shown: the bar now ends
  short of it.
- **Save editor**: on a PES 2017 to 2021 save, the Tactics tab's Numbers in attack and
  Numbers in defence are disabled unless the active ruleset allows illegal formations,
  since the game no longer lets them be set: a cup playing a patched game keeps them.
- **Save editor**: what Ctrl+C copies (a player's stats, a team's Gameplan, a tactics
  preset or a formation) can be pasted with Ctrl+V in another Save editor window, even one
  holding a save of another PES version, and still after opening another save.
- **Save editor**: a player dragged in a team's player list shows as one `Player order`
  row in the unsaved changes, listing the places he and the players he shifted held
  before and after. Each player then lists only what was edited on him, rather than every
  field of the record the drag put in his place.
- **Save editor**: each row of the toolbar's unsaved changes list ends with an ×, which
  puts that one change back as the file has it, keeping the other edits, and Undo brings it
  back again. On a `Player order` row it puts the players back on their places, each
  keeping what was edited on him.
- **Save editor**: on a PES 2017 or PES 2019 save, Live AATF and `check-aatf` warn about a
  formation the game's own screens cannot make (a player beyond the area a starter can be
  moved in, at a position the game does not give at his spot, or on top of a teammate, a
  wide position held twice, too many or too few in a line), naming the preset, the
  formation and the players, unless the ruleset allows illegal formations. Moving the
  player on the Tactics pitch puts it right.
- **Save editor**: lists only the cup's teams (IDs 701 to 920) and the players on their
  rosters, in the team selector, the filter, Multi-team AATF, the Comparator and the roster
  sheet, and Fix database repairs only those. The game's own clubs and national teams stay
  in the save untouched. On the command line, `diff` compares the cup's players alone and
  `export-toml --all-teams` writes the cup's teams alone, as `check-aatf` already checked
  them, while a team named by its ID is still read.

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
  window, as it does for most programs. It did nothing unless Windows
  had a shortcut for the program, such as a taskbar pin.
- **Save editor**: a save is edited as its own version, whatever the sidebar's selector
  shows: the portraits and the Set PES path button read the PES folder of the save's
  version, and the editor no longer warns that the save is not the sidebar's version.
  The file picker starts in the folder the last save was opened from.
- **Save editor and Ruleset editor**: the sidebar's PES version box stands empty and
  disabled while the editor is shown, instead of naming a version the editor does not
  use.
- **Save editor**: in the Tactics tab's bench list, a player dragged onto another swaps
  places with him, as in the starting eleven's list. He used to be moved into that place,
  shifting the players between.
- **Save editor**: a starter moved on the Tactics pitch, dragged or edited in the slot
  popover's X and Y, now stays inside the area the game's own Formation screen can put
  him in, so an edit can no longer make a formation Live AATF warns about, unless the
  active ruleset allows illegal formations.
- **Save editor**: a PES 2018 save's Tactics pitch is drawn as that game's own, which
  is PES 2017's; PES 2015's and PES 2016's draw PES 2017's too, the nearest to theirs
  that has been measured.
- **Sidebar**: in a window too short for every entry, the tool entries now scroll with
  the wheel in the room between the version selector and the bottom, where Help and
  Settings stay pinned instead of being drawn over the last entries.

### Minor fixes

- On a small screen, the status bar's **Reset window** button no longer stays after a
  click that left the window as it was: it goes, and comes back when the window is
  resized. The pointer turns into a hand over it.
- **Save editor**: an aesthetics patch's names now show in game on PES 2019, where they
  showed PLACEHOLDER: applying it marks each renamed player as edited, as typing the
  name does. `apply-patch` writing back over its own save with no player applied now
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
- A click between two neighbouring entries of a menu or of a dropdown list no longer closes it with
  nothing picked: the entries now touch, so the click lands on one of them.
- The corners of menus, dialogs, dropdown lists, text fields and the update download's
  bar all carry the same slight rounding again: an update of the toolkit the window is
  drawn with had left the menus and dialogs rounder than the rest.

## [0.0.1] - 2026-11-03

- **Save editor**: the first preview, for the DBC7's managers on PES 2017: players, teams and
  tactics, the aesthetics patch, the roster sheet, Live and Multi-team AATF, the Comparator,
  undo and redo, the unsaved-changes list, and a command line for batch work.
- **Ruleset editor**: a work-in-progress look at the new cup rules format (4ccrules).
