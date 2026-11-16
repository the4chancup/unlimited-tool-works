The Save editor's window opens a save (`EDIT00000000`, or a copy of it) and edits it.

**Opening a save**

The **File** menu lists, from the top:

- **Open…** (Ctrl+O), which picks any file and starts in the folder the last save was opened
  from when it exists, or in the newest found save's folder;
- **Save** (Ctrl+S) and **Save as…** (Ctrl+Shift+S), which write the open save (see **Saving**
  below) and stay greyed out until a save is open;
- under **Open from Documents**, the saves found in your Documents folder for every PES version,
  each named by its version, so your game's save opens in one click (hover an entry to see
  its full path);
- under **Recent files**, the last 10 saves you opened or saved, the most recent first, kept
  for the next time the program opens; each is named by its version, folder and file name,
  so two saves of the same name stand apart;
- **Close save** (Ctrl+W), which closes the save and brings back the list of saves in the
  middle of the window, and
  **Exit** (Ctrl+Q), which closes the window as its close button does.

With no save open, the middle of the window offers the same saves, the recent saves and
**Open…**.

Ctrl+5 to Ctrl+9 open the newest save found for PES 2015 to PES 2019, Ctrl+0 PES 2020's and
Ctrl+1 PES 2021's, or the file picker when none was found. They do nothing while you type in
a field or a dialog is open.

Once a save is open, the window's title bar shows its path, in square brackets after the
editor's name, and the toolbar shows `• unsaved changes` while edits wait unsaved.

**The save's PES version**

The editor reads a save's PES version from the file itself and edits it as that version, so
the sidebar's version box stays empty here. The players' portraits come from that version's
PES folder.

**Teams and players**

The dropdown above the list picks a team, or All teams for every cup team of the save.
Picking a team selects its row, so the right side shows the team's fields and tactics;
All teams keeps what you had selected.

The radio buttons at the bottom of the open dropdown narrow the dropdown's teams and the
list to one competition's teams: 4cc, VGL or Invitational; All is every cup team again.
If the picked team is not in the category, the dropdown shows All teams. The pick is
remembered for next time.

Only the cup's teams are listed (ids 701 to 920), with the players on their rosters. A save
made from the game's own data also holds its clubs and national teams, and players on no cup
team: they stay in the save exactly as they are, and the editor never lists, checks or
changes them.

The list holds a row per team and, under each team, a row per player: the team's row shows
its id, its two colors and its name, and a player's row his shirt number, position and name
as the game draws it, colors included. A `●` on a row marks
a player or the team itself with unsaved changes.

The filter box above the list finds a team, or a player of any listed team, by part of a
name or shirt name, or by the start of an id, so you can find someone without knowing their
team first. A team whose name or id matches is listed with its whole squad. Double-click a
team's row to show that team alone: the team selector picks it and the filter box empties.
While no field has the keyboard and no menu or dialog is open, Up and Down move through
the list's rows, team rows included, and Page Up and Page Down move to the previous or
next team's row (seven rows at a time when only one team is listed).

**Reordering a team**

With the filter box empty, drag a player row onto another row of his team to move him to
that slot; a drop on another team's row does nothing. The shirt number and the in-game face
and hairstyle belong to the slot, so they stay where they were.

**Editing**

Click a player's row to select him: the right side shows his id and name, and under them his
fields. The toggle left of the name switches between **Stats**, three columns of his fields,
and **Appearance**, his look. Click a team's row, or **Gameplan** on the toggle, to see the
team's fields and its tactics.

A player's Name and Shirt labels count what the field holds (`(12/61)` reads as 12 used
of 61). When it is longer than an older version can hold, the field takes an amber tint and
its count turns red. A `\x11c` plus 8 hex digits inside a name is a color code, and the field
keeps it as typed.

**Undo and redo**

Every edit can be undone: the **Edit** menu's **Undo** (Ctrl+Z) and **Redo** (Ctrl+Y or
Ctrl+Shift+Z) step through your edits. A drag, or a run of typing in one field, is one undo
step.

**Copy and paste**

Ctrl+C copies the selected row, and Ctrl+V pastes it onto another selected row of the same
kind: a player's row copies his stats (the fields the **Player** menu's **Copy/swap stats…**
moves), and a team's row copies its Gameplan. Each paste is one undo step. The **Edit** menu's
**Copy** and **Paste** do the same as the two keys.

On a team's **Gameplan** screen, a click on a **Preset** button (1, 2 or 3) or a formation
button (**Kick-off**, **In-possession** or **Out-of-possession**) makes Ctrl+C copy that
preset or formation, and Ctrl+V paste over the one shown. That lasts while the team's row
stays selected: select another row, or click the team's row, and the keys copy and paste
whole rows again.

The copy is text on the system clipboard, so it also pastes into another Save editor window,
or after another save is opened, even a save of another PES version. When the target's
version cannot hold a value as it was, the status bar says how many were left out or
changed.

While you type in a text field, Ctrl+C and Ctrl+V copy and paste its text, and Ctrl+C with
text selected on screen copies that text.

**Quick actions**

Under the list, one **Make \<tier\>** button per tier of the AATF rules in use gives the
player that tier's values (stats, form, injury resistance, weak foot, name color) as one
undoable edit. **Set stats** sets every ability stat to the number beside it (40 to 99).
These buttons act on the selected player, so they stay greyed out while a team row is
selected.

**AATF ruleset**

The dropdown on the toolbar, left of **Live AATF**, picks the AATF rules the quick actions,
the Cards used badge and Live AATF use. Every cup's current rules are built in, listed by
name, and below them come the rules files in the data folder's `rules` folder, where
**Import rules file…** puts them. The pick is remembered. The ruleset's event logo shows at
the window's top right, and when the ruleset names its event's page, a click on the logo
opens it. When the file picked last is gone from the data folder, 4cc Autumn 26 is used
instead and a notice says so.

**Import rules file…** at the bottom of the list adds a cup's `.4ccrules` file: it is copied
into the data folder and picked, while a file with errors is refused with its errors listed,
so you can fix it and import it again. The **×** beside an imported ruleset removes it from
the list after asking, deleting only the data folder's copy, never the file you imported.

**Comparator**

**Comparator…**, in the toolbar's **Savefile** menu, opens a floating window comparing the
open save, edits included, against a second save: **Pick…** chooses it, and refuses a save of
another PES version, naming both versions. The **Show** row narrows the list to All, Gameplay or Aesthetics, and
the team dropdown to one team.

Rows group by team and by player: each row shows a field's name and its value in the picked
save and in the open save. Clicking a row selects that player in the editor. Only the players
on a cup team's roster, in either save, are compared: the game's own teams are left out.

A comparison takes a moment on a big save, and the window says so until the rows land.
**Export…** writes the differences the Show row picks to a text file, for every team,
whatever the team dropdown shows.

**Live AATF**

The toolbar's **Live AATF** toggle checks the open team after every edit and lists, in a
panel docked at the bottom of the window, the violations left: the header reads
`N issues · <ruleset>`, and the rows are grouped per player, with the team's own under
`Team-wide issues`. Once the team passes, the header reads `<team> passes <ruleset> with no
issues`. Clicking a row selects that player. A Fielded position row, about a position he is
fielded at, selects his team's row instead and shows the first formation of the named preset
on which his portrait is crossed out (see below).

The icon near the right end of the panel's header opens the list in a window of its own, which
you can move and resize to keep it beside what you are fixing. The window lists the players
one under the other, its icon beside the close button puts the list back in the panel, and
closing the window turns Live AATF off. The panel's own close button, beside its icon, turns
Live AATF off too. The editor remembers where you left the list, so turning Live AATF on
again opens it there.

**Copy to clipboard**, left of that icon in the panel and under the close button in the
window, copies the list as it shows, as plain text you can paste into a message: a line
naming the ruleset, the team with its error and warning counts, then the same groups.

While it is on, the editor itself is marked too:

- each player of the team gets a mark at the end of his row in the list: a red `✖` when he
  breaks a rule, otherwise a `✔`, amber when he has a warning, blue when he has only
  suggestions and green when there is nothing to fix (hover over it to see how many of each
  he has);
- a field of the selected player that a row is about gets a mark just left of its label: a
  red `✖` for an error, an amber `⚠` for a warning and a blue `ⓘ` for a suggestion (hover
  over it to read its rows);
- on the tactics pitch, a player fielded at a position he is not rated A at has his portrait
  crossed out, on every formation the preset fields him at that position.

**Show suggestions**, beside the toggle while it is on, also lists each player's unused
allowances (cards he could still take, free tricks, weak foot) under `Suggestions`, after
the violations. It is off at first, because most players of a cup team leave some allowance
unused. The toggle and the checkbox are remembered, so they are as you left them the next
time the editor is opened.

**Saving**

The **File** menu's **Save**, or Ctrl+S, writes the edits to the file the save was opened
from (a file it replaces is kept once as a `.bak` copy). **Save as…**, or Ctrl+Shift+S,
writes them to a file you pick instead, and the editor then carries on with that file, so
the next Save writes there too. A file it replaces is kept as a `.bak` copy as well.

While the save's own PES version is running, saving first shows how to keep the save:

- if the game's Edit mode is open, use its Data Management > Load after saving, before you
  leave Edit mode. Leaving Edit mode after changing anything in it writes the game's own
  copy over your save;
- if Edit mode is closed, the changes show when you open it.

While only a PES of another version is running, saving asks nothing.

**Unsaved changes**

Closing the save or the window, or opening another save, with unsaved edits asks first,
over a table of each changed field with its value in the file and in the editor. Save
writes the edits, Discard loses them, and Cancel keeps everything as it was.

Each row ends with an ×, which puts that one change back as the file has it and keeps
your other edits, so you can drop a change you did not mean and save the rest. The × is
an undo step like any edit, so Undo brings the change back. Once no row is left there is
nothing to save, and the window closes or the other save opens.

A player you dragged to another slot in the list shows as one `Player order` row of his
team, which lists the slots he and the players he shifted held in the file beside the
slots they hold now, one slot per line. Each of those players lists only what you
edited on him, since the drag itself is that one row. On that row the × puts every
player of the row back in his slot in the file, and each keeps what you edited on him.

Clicking `• unsaved changes` on the toolbar shows the same list, ×'s included. It stays
open a row shorter after each ×, and closes when no row is left.
