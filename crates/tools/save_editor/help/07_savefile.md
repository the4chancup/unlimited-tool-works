The **Savefile** menu works on the whole save:

- **Comparator…** opens a window comparing the open save against a second
  save (see the window's chapter).
- **Apply aesthetics patch…** opens a panel for a patch the Team compiler
  wrote. **Open patch…** reads it; a patch for another PES version, or
  from a different version of the Team compiler, is refused and the save
  is left alone. The panel then lists every value the patch would change,
  player by player, with the old value and the new one; click a row to
  select that player.
  A team the save does not hold is named and skipped, and the rest
  applies. **Apply** writes the patch as one undoable step, unsaved until
  you save. A patch whose values the save already holds lists nothing,
  says so, and leaves Apply disabled. It works while the aesthetics lock
  is on: a patch is how the teams' exports reach the save.
- **Output rosters to TSV…** writes the roster sheet, the file members
  paste into a spreadsheet to check a cup's teams: every team of the
  save, hidden ones included, its name on a line of its own, then one
  tab-separated row per player (shirt number, name, position, Finishing,
  a `(C)` on the captain, height, playing style, skills and COM styles).
  A save dialog asks where to write it, with `EDIT.tsv` as the default
  name. It changes nothing in the save.
- **Multi-team AATF…** checks a range of teams under the AATF ruleset
  picked on the toolbar, for an organizer checking a cup's teams. Pick the
  first and the last team of the range (the save's first and last team to
  begin with); every team whose id lies between them is checked and listed
  under its id and name, its problems worded as Live AATF words them, or
  `All clean` when it has none. Suggestions are left out, since an unused
  allowance is the manager's business. **Copy to clipboard** copies the
  list as plain text for a cup's thread, each team's line after a `>` so
  the thread shows it in green. The dialog only reads the save.
- **Fix database…** is a maintenance repair behind a confirmation: it
  repairs base copies on PES 2015 to 2018 (a stray copy id is cleared, and
  a copy flag that points at nothing is cleared; later games' records have
  no base-copy flag, so their copy ids are kept), clears the face, hair,
  physique and strip edit flags on every player, and on PES 2017 clears
  every team's strip flag and resets the kit slots to their defaults. The
  whole repair is one undoable step. It stays disabled while the
  aesthetics lock is on (see the Appearance section), since it clears what
  the teams' exports own.
