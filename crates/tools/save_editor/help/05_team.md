Clicking a team's row in the list opens the team view: the team's id and name as
its title, one row holding the name, three-letter short name and two colour
fields, the manager and stadium IDs and the **Kit slots…** button, and the
Tactics below. A field the save's version does not hold stays in its place,
disabled, with a note naming the first version that has it.

**Names and colours**

The name and short name fields keep only what the save can hold: the name up to
the save's byte limit (an accented letter counts more than one), the short name
up to three letters, shown upper case.

A colour opens the same picker as elsewhere. The save keeps 64 steps per
channel, so the colour written is the nearest one the save can hold, and the hex
beside the field shows what the save actually stores.

**Kit slots**

**Kit slots…** opens the kit slot assignments in a dialog (PES 2017 only) and
closes on OK. A bound kit slot is shown as the team id it points at, and the
extra data the game keeps alongside is left as it was.

**The Team menu**

The **Team** menu works on the selected row's team, a player row's own or a team
row's, and writes every filled roster slot in one undoable step:

- **Set edited flag** marks every player of the team as edited.
- **Set boots and gloves IDs for everyone…** opens a small dialog with a box for
  the boot IDs and one for the glove IDs.
- **Set player names to positions** renames every player of the team to his
  registered position's full name.
- **Clear visual flags** (Ctrl+F) clears the face, hair, physique and strip edit
  flags on every player of the team.
- **Set FPC invisibility** writes the hide preset on every player of the team:
  long sleeves, a tucked shirt and short socks, and a boots or gloves ID of 0
  takes the hide ID from Settings (55 and 11 unless changed, while a model's own
  ID is kept, as in the Appearance section).
- **Remove FPC invisibility** writes the show preset on them all, and on PES
  2015 to 2017 both FPC items also turn a Custom skin into Light.
- **Import / export team file…** opens a panel beside the views.

The boots and gloves item, Clear visual flags and the FPC items stay disabled
while the aesthetics lock is on (see the Appearance section), because they write
what the team's export owns and a hand edit would be overwritten the next time
the team is recompiled or patched. Ctrl+F does nothing while the lock is on.

**The boots and gloves dialog**

The boots can:

- stay **Unchanged**;
- count up from a start ID with **Set incremental value:** (slot 1 gets the
  start, slot 2 the next, and so on; the grey box beside the start shows slot
  23's ID and follows it);
- take one ID with **Set all to the same ID:**.

The gloves can stay **Unchanged**, take each player's boot ID with **Set like
boot IDs**, or take one ID for all.

**Set** writes the IDs and closes the dialog, **Close** closes it without
writing. An ID the save cannot hold is refused with a line under the boxes, and
nothing is written.

**Exporting a team file**

The import / export panel's top row exports the team:

- **Team TOML…** writes the team, its tactics and its players with every value,
  the same file the command line's `export-toml` writes;
- **Texport…** writes a `.ted` file the game's own team import reads (PES 2018
  to 2021; PES 2015 to 2017 have no Texport template, so the button stays
  disabled).

Neither will write over the save that is open.

**Importing a team file**

Below the panel's top row, **Pick…** opens a team file to import into the selected team: a
Team TOML (`.toml`), a squad file (`.4ccs`), a tactics file (`.4cct`) or a
Texport. A squad file lists which record goes to which player.

Tick the sections to bring in:

- **gameplay** (abilities, skills, height and the like, and the team's name,
  short name, manager and stadium);
- **aesthetics** (names, shirt numbers, the look);
- **tactics**.

Only the sections the file holds are offered, and aesthetics stays off while the
aesthetics lock is on.

A file from another PES version is converted, and the values that could not
carry over are listed as notes, grouped, each group opening to its lines. If a
ticked section holds a value the save cannot take, the reason stands under the
choices and Apply is disabled; untick that section to import the rest.

**Apply** writes the ticked sections as one undoable step, unsaved until you
save. When the team already holds every value the ticked sections would write
(its own file, or a file applied already), Apply stays disabled and the panel
says so.
