The Save editor's window opens a save (`EDIT00000000`, or a copy of it) and edits it.

**Opening a save**

The **Open** menu lists, from the top:

- the saves found in your Documents folder for every PES version, each named by its version
  and, from PES 2019 on, by the account folder it sits in, so your game's save opens in one
  click (hover an entry to see its full path);
- the files you opened since the program started, the most recent first;
- **Choose file…**, which picks any file and starts in the save folder of the PES version
  selected in the sidebar when there is one.

Once a save is open, the window's title bar shows its path, in square brackets after the
editor's name, and the toolbar shows `• unsaved changes` while edits wait unsaved.

**A save from another PES version**

A save from another PES version than the one selected in the sidebar opens as its own
version, with a warning in the status bar until you dismiss it or switch. Its **Switch to
PES 20xx** button sets the sidebar to the save's version, which also changes the game the
other tools work with.

**Teams and players**

The dropdown picks a team, or ALL for every team of the save; picking a team selects its
row, so the right side shows the team's fields and tactics (ALL keeps what you had
selected).

The list below it holds a row per team and, under it, one per player: the team's row shows
its id, its two colours and its name followed by `[click for tactics]`, a player's row his
shirt number, position and name as the game draws it, colors included. A `●` on a row marks
a player or the team itself with unsaved changes.

The filter box above the list finds a team, or a player of any team, by part of a name,
shirt name or id, so you can find someone without knowing their team first.

**Reordering a team**

With the filter empty, drag a player row onto another row of his team to move him to that
place; a drop on another team's row does nothing. The shirt number and the in-game face and
hairstyle belong to the place, so they stay where they were.

**Editing**

Click a player's row to select him: the right side shows his id and name, then three columns
of his fields, and the collapsed **Appearance** section underneath. Click a team's row to
select the team: the right side shows the team's fields and its tactics.

A player's Name and Shirt labels count what the field holds (`(12/61)` reads as 12 used
of 61) and turn amber when it is longer than every version can hold. A `\x11c` plus 8 hex
digits inside a name is a color code, and the field keeps it as typed.

**Undo and redo**

Every edit is undoable, and **Undo** and **Redo** step through them (Ctrl+Z, Ctrl+Y or
Ctrl+Shift+Z); a drag or a run of typing on one field is one undo step.

**Quick actions**

Under the list, one **Make \<tier\>** button per tier of the AATF rules in use gives the
player that tier's values (stats, form, injury resistance, weak foot, name color) as one
undoable edit. **Set stats** sets every ability stat to the number beside it (40 to 99).
They act on the selected player, so a team row leaves them greyed out.

**AATF ruleset**

The dropdown on the toolbar, left of **Live AATF**, picks the AATF rules the quick actions,
the Cards left badge and Live AATF use. Every cup's current rules are built in, listed by
name, and the copies in the data folder's `rules` folder follow them. The pick is
remembered. When the copy picked last is gone from the data folder, 4cc Autumn 26 is used
instead and a notice says so.

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
save and in the open save, and a mark sits beside a player stripped in one save or both.
Clicking a row selects that player in the editor.

A comparison takes a moment on a big save, and the window says so until the rows land.
**Export…** writes the picked scope's differences over the whole save as a text file.

**Live AATF**

The toolbar's **Live AATF** toggle checks the open team after every edit, in a panel docked
at the bottom of the window: the violations left, grouped per player and the team's own
under `Team-wide issues`, under `N issues · <ruleset>`, or `<team> passes <ruleset> with no
issues` once the team passes. Clicking a row selects that player; a `FieldedNotA` row, about
a position he is fielded at, selects his team's row instead, on the named preset's first
formation that crosses his portrait.

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

**Save** or Ctrl+S writes the edits to the file the save was opened from (a file it replaces
is kept once as a `.bak` copy). **Save as** writes them to a file you pick instead, and the
editor then carries on with that file, so the next Save writes there too. A file it replaces
is kept as a `.bak` copy as well.

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
