The **Savefile** menu works on the whole save:

- **Comparator…** opens a window comparing the open save against a second save
  (see **The window**).
- **Apply aesthetics patch…** opens a panel for a patch the Team compiler wrote.
- **Output rosters to TSV…** writes the roster sheet, the file members paste
  into a spreadsheet to check a cup's teams.
- **Multi-team AATF…** checks a range of teams under the AATF ruleset picked on
  the toolbar, for an organizer checking a cup's teams.
- **Fix database…** is a maintenance repair behind a confirmation.

**Applying an aesthetics patch**

The panel's **Open patch…** reads the patch; a patch made for another PES
version, or by a Team compiler that numbers custom boots and gloves differently
from the one in this program, is refused and the save is left alone. The panel
then lists every value the patch would change, player by player, with
the old value and the new one; click a row to select that player.

A team the save does not hold is named and skipped, and the rest applies.
**Apply** writes the patch as one undoable step, unsaved until you save. A patch
whose values the save already holds lists nothing, says so, and leaves Apply
disabled. It works while the aesthetics lock is on: a patch is how the teams'
exports reach the save.

**The roster sheet**

The sheet holds every cup team of the save: its name on a line of its own,
then one tab-separated row per player (shirt number, name, position,
Finishing, a `(C)` on the captain, height, playing style, skills and COM
styles). A save dialog asks where to write it, with `EDIT.tsv` as the default
name. Writing it changes nothing in the save.

**Multi-team AATF**

Pick the first and the last team of the range (both start on the save's first cup
team). The last team's list starts at the first team, and moving the first past the last
brings the last along, so the range is never backwards. Every cup team whose id lies
between them is checked and listed under its id and name, with its problems worded as
Live AATF words them, or `All clean` when it has none. The check starts once both ends
have stayed put for half a second, so the mouse wheel can run through the teams without
checking each one on the way; until then the previous list stays, dimmed and marked
`checking…`. Suggestions are left out, since an unused allowance is the manager's
business.

**Copy to clipboard** copies the list as plain text for a cup's thread, each
team's line after a `>` so the thread shows it in green. The dialog only reads
the save.

**Fix database**

**Fix database…** does the following, as one undoable step:

- on PES 2015 to 2018, it repairs base copies, clearing a stray copy id and a copy
  flag that points at nothing (later games' records have no base-copy flag, so
  their copy ids are kept);
- it clears the face, hair, physique and strip edit flags on every cup player;
- on PES 2017, it also clears every team's strip flag and resets the kit slots to
  their defaults.

It repairs the cup's teams and their players alone, and leaves the game's own
teams as they are.

It stays disabled while the aesthetics lock is on (see the Appearance section),
since it clears what the teams' exports own.
