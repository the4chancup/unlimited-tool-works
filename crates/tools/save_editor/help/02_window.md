The Save editor's window opens a save (`EDIT00000000`, or a copy of it) and edits it.

**Opening a save**

The **File** menu lists, from the top:

- the saves found in your Documents folder for every PES version, each named by its version,
  so your game's save opens in one click (hover an entry to see its full path). Ctrl+5 to
  Ctrl+9 open the save of PES 2015 to PES 2019, Ctrl+0 PES 2020's and Ctrl+1 PES 2021's, from
  anywhere in the editor; with no save found for that version, they open the file picker;
- the files you opened since the program started, the most recent first;
- **Choose file…** (Ctrl+O), which picks any file and starts in the folder the last save
  was opened from when it exists, or in the newest found save's folder;
- under a line, **Save** (Ctrl+S) and **Save as**, which write the open save (see **Saving**
  below) and stay greyed out until a save is open.

Once a save is open, the window's title bar shows its path, in square brackets after the
editor's name, and the toolbar shows `• unsaved changes` while edits wait unsaved.

**A save from another PES version**

A save opens and edits as its own version, whatever the sidebar's selector shows: the
editor reads the game from the open save, so its portraits and the PES folder they are
read from are the save's version's.

**Teams and players**

The dropdown picks a team, or ALL for every team of the save; picking a team selects its
row, so the right side shows the team's fields and tactics (ALL keeps what you had
selected).

The list below it holds a row per team and, under it, one per player: the team's row shows
its id, its two colours and its name followed by `[click for gameplan]`, a player's row his
shirt number, position and name as the game draws it, colors included. A `●` on a row marks
a player or the team itself with unsaved changes.

The filter box above the list finds a team, or a player of any team, by part of a name,
shirt name or id, so you can find someone without knowing their team first. A team whose name
matches is listed with its whole squad. Double-click a team's row to show that team alone: the
team selector picks it and the filter box empties. Up and Down move through the list's rows,
team rows included, while no field has the keyboard and no menu or dialog is open.

**Reordering a team**

With the filter box empty, drag a player row onto another row of his team to move him to
that place; a drop on another team's row does nothing. The shirt number and the in-game face
and hairstyle belong to the place, so they stay where they were.

**Editing**

Click a player's row to select him: the right side shows his id and name, then three columns
of his fields, and the collapsed **Appearance** section underneath. Click a team's row to
select the team: the right side shows the team's fields and its tactics.

A player's Name and Shirt labels count what the field holds (`(12/61)` reads as 12 used
of 61) and turn amber when it is longer than every version can hold. A `\x11c` plus 8 hex
digits inside a name is a color code, and the field keeps it as typed.

**Undo and redo**

Every edit is undoable, and the **Edit** menu's **Undo** and **Redo** step through them
(Ctrl+Z, Ctrl+Y or Ctrl+Shift+Z); a drag or a run of typing on one field is one undo step.

**Quick actions**

Under the list, one **Make \<tier\>** button per tier of the AATF rules in use gives the
player that tier's values (stats, form, injury resistance, weak foot, name color) as one
undoable edit. **Set stats** sets every ability stat to the number beside it (40 to 99).
They act on the selected player, so a team row leaves them greyed out.

**AATF ruleset**

The dropdown on the toolbar, left of **Live AATF**, picks the AATF rules the quick actions,
the Cards used badge and Live AATF use. Every cup's current rules are built in, listed by
name, and the copies in the data folder's `rules` folder follow them. The pick is
remembered. The ruleset's event logo shows at the window's top right, and when the ruleset
names its event's page, a click on the logo opens it. When the copy picked last is gone from
the data folder, 4cc Autumn 26 is used instead and a notice says so.

**Import rules file…** at the bottom of the list adds a cup's `.4ccrules` file: it is copied
into the data folder and picked, while a file with errors is refused with its errors listed,
so you can fix it and import it again. The **×** beside an imported ruleset removes it from
the list after asking, deleting only the data folder's copy, never the file you imported.

**Comparator**

**Savefile** in the toolbar, **Comparator…**, opens a floating window comparing the open
save, edits included, against a second save: **Pick…** chooses it and refuses one of another
version, naming both. The **Show** row narrows the list to All, Gameplay or Aesthetics, and
the team dropdown to one team.

Rows group by team and by player: each row shows a field's name and its value in the picked
save and in the open save. Clicking a row selects that player in the editor.

A comparison takes a moment on a big save, and the window says so until the rows land.
**Export…** writes the picked scope's differences over the whole save as a text file.

**Live AATF**

The toolbar's **Live AATF** toggle checks the open team after every edit, in a panel docked
at the bottom of the window: the violations left, grouped per player and the team's own
under `Team-wide issues`, under `N issues · <ruleset>`, or `<team> passes <ruleset> with no
issues` once the team passes. Clicking a row selects that player; a Fielded position row,
about a position he is fielded at, selects his team's row instead, on the named preset's
first formation that crosses his portrait.

The icon near the right end of the panel's header opens the list in a window of its own, which
you can move and resize to keep it beside what you are fixing. The window lists the players
one under the other, its icon beside the close button puts the list back in the panel, and
closing the window turns Live AATF off. The panel's own close button, beside its icon, turns
Live AATF off too. The editor remembers where you left the list, so turning Live AATF on
again opens it there.

While it is on, the editor itself is marked too:

- each player of the team gets a mark at the end of his row in the list: a red `✖` when he
  breaks a rule, otherwise a `✔`, orange when he has a warning, blue when he has only
  suggestions and green when there is nothing to fix (hover over it to see how many of each
  he has);
- a field of the selected player that a row is about gets a mark just left of its label: a
  red `✖` for an error, an orange `⚠` for a warning and a blue `ⓘ` for a suggestion (hover
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
from (a file it replaces is kept once as a `.bak` copy). **Save as** writes them to a file
you pick instead, and the editor then carries on with that file, so the next Save writes
there too. A file it replaces is kept as a `.bak` copy as well.

While the save's own PES version is running, saving first shows how to keep the save:

- if the game's Edit mode is open, use its Data Management > Load after saving, before you
  leave Edit mode, since leaving it after changing anything there writes the game's own copy
  over your save;
- if Edit mode is closed, the changes show when you open it.

A PES of another version asks nothing.

**Unsaved changes**

Closing the window or opening another save with unsaved edits asks first, over a table of
each changed field with its value in the file and in the editor. Save writes the edits,
Discard loses them, and Cancel keeps everything as it was.

A player you dragged to another place in the list shows as one `Player order` row of his
team, which lists the places he and the players he shifted held in the file beside the
places they hold now, one place per line. Each of those players lists only what you
edited on him, since the drag itself is that one row.

Clicking `• unsaved changes` on the toolbar shows the same list. Each row there ends with an ×, which puts that one change back as the file has it and
keeps your other edits, so the list stays open a row shorter, and closes when none is left. It is a step like any edit,
so Undo brings the change back. On a `Player order` row, the × puts every player of the
row back on his place in the file, and each keeps what you edited on him.
