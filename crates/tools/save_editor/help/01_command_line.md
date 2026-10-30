The Save editor's batch commands run from a terminal opened in the folder that holds `UTW.exe`.
Every command names the save it reads (the `EDIT00000000` file of your PES install's save
folder, or a copy of it), and no command picks one for you.

```text
UTW save-editor export-toml <save> --team <id> --out <file>
UTW save-editor export-toml <save> --all-teams --out-dir <folder>
UTW save-editor import-toml <team.toml> --base <save> --out <save> [--edit-mode-closed]
UTW save-editor apply-patch <aesthetics_patch.toml> --base <save> --out <save> [--edit-mode-closed]
UTW save-editor diff <first save> <second save> [--aesthetics | --gameplay] [--json]
UTW save-editor check-aatf <save> [--teams <list>] [--rules <file>]
UTW save-editor export-teams-list <save> [--yes]
```

**Writing a save**

A command that writes a save reads the one `--base` names and writes the one `--out` names,
which may be the same file. A file already at `--out` is first copied to `<file>.bak` (one
level deep: the next write replaces that copy). No command creates a save from nothing.

**While the game is running**

While the PES of the save's version is running, the game writes the save it holds over the file
when you leave its Edit mode after changing anything there, so a save written while Edit mode
is open is lost then unless Edit mode's Load reads it in first. A command that writes a save
therefore writes it while the game runs only when you add `--edit-mode-closed`, which says you
have closed Edit mode.

Without it, the command writes nothing and names the running game: close Edit mode, then run
the command again with `--edit-mode-closed`. Opening Edit mode then reads the save in without
restarting the game. A PES of another version running does not count.

**Exporting teams**

`export-toml` writes a team of the save as a Team TOML file: the team's names, colors, manager
and stadium, its tactics, and every rostered player with every field, the look included.

- With `--team` one team goes to the file `--out` names, replacing it when it exists.
- With `--all-teams` every team of the save goes to its own `<id>.toml` in the `--out-dir`
  folder, which is created when missing and whose older files are replaced.

A team id the save does not hold, or an `--out` naming the save itself, is refused and nothing
is written.

**Importing a team**

`import-toml` writes a Team TOML file onto the team its `[team] id` names. The file's keys
decide what it writes: a key, a player or a whole section missing from the file stays as the
save has it, so a file holding only `[team] id` and `[players.03] name = "Snuffy"` renames one
player and nothing else.

A file written from another PES version is converted on the way in, and each value that could
not be taken as written is one line:

- a key the save's version has no field for (`import_key_not_in_version`);
- a value it cannot store (`import_value_not_encodable`);
- a face type above the target's cap or a custom skin (7) on a version that has none
  (`import_value_capped`, which writes the face type as 0 and the skin as 1);
- a player table for an empty roster slot (`import_slot_empty`).

Those lines do not change the exit code. A file with no `[team] id`, one naming a team the save
does not hold, or one with a name longer than the save's version has room for is refused,
naming the team or the player, and nothing is written.

**Applying an aesthetics patch**

`apply-patch` applies the `aesthetics_patch.toml` the Team compiler writes beside its CPKs: the
compiled players' motions and, for exports carrying the `write_names` marker, their names. The
patch must be for the save's PES version and the suite's boots and gloves allocation scheme, or
it is refused with nothing written.

Each team of the patch is one line, `patch_team_applied`, listed only when at least one of its
players was written: a team whose patch players the save lacks entirely is not listed. A team
or a player the save does not hold is the error line `patch_team_not_in_save` or
`patch_player_not_in_save`. The rest of the patch is applied and written, and the command exits
1.

Like every command that writes a save, its lines come once `--out` is written, so a write that
fails prints only its failure.

**Comparing two saves**

`diff` lists what differs between two saves of the same version, player by player, paired by
player id:

- one line per changed field, `player_diff`, with the field, the first save's value and the
  second's;
- a player who is in one save alone is `player_only_in`.

`--aesthetics` keeps the look's fields alone (appearance id, boots, gloves, base copy, physique,
strip, face), and `--gameplay` keeps everything else (names, basics, abilities, positions,
skills, motion).
With `--json` the report is one JSON document on stdout instead, for tooling. It lists the
players with their changed fields, then the players only one save holds.
Nothing else is printed.

Differences are the output, not errors, so the exit code is 0. Two saves of different versions
are refused.

**Checking teams against AATF rules**

`check-aatf` checks the save's cup teams against an AATF ruleset: every team whose id is in the
cup range, 701 to 920, or the ones `--teams` names as a comma-separated list of ids and
inclusive ranges (`701,705-710`).

The ruleset is the file `--rules` names, read from where the command runs, else the one picked
in the dropdown on the window's toolbar (the `aatf_ruleset` setting): a built-in ruleset or a
copy in the data folder's `rules` folder. When that copy is gone, the warning line
`aatf_ruleset_gone` says so and the official ruleset built into Unlimited Tool Works, 4cc
Autumn 26, is used instead.

Each violation prints as one `aatf_violation` line with the team, the player's id for a
player's violation, the rule and what is wrong, and the teams print in id order. A team
whose check cannot run is the error line `aatf_check_failed`, and the rest are still checked.
An error-severity violation makes the command exit 1, while warnings alone leave it at 0.

A rules file that does not load is refused, naming the line and column or the key path it
failed at, so the command also validates a rules file. A `--teams` id the save does not hold is
refused as well.

**Updating the teams list**

`export-teams-list` merges the save's teams into the `teams_list.txt` of the data folder (the
list the Team compiler resolves export names against, or the list shipped with Unlimited Tool
Works when there is no file yet): every team whose name is slash-wrapped (`/co/`) and whose id
is in the cup range, 701 to 920.

It prints what the merge does, one line each:

- a team the list lacked (`teams_list_added`);
- a name already listed under another id, which takes the save's id instead
  (`teams_list_overridden`);
- a change dropped because two teams would share one id (`teams_list_unresolved`, an error
  line);
- a team left out because an earlier team of the save has the same name, upper and lower case
  aside (`teams_list_name_duplicate`, an error line naming the team kept, so fix the save);
- how many rows stay as they were (`teams_list_kept`).

Without `--yes` it stops there (`teams_list_not_written`); with `--yes` it writes the merged
list (`teams_list_written`), with the dropped changes left out. A data folder that cannot take
the file, or none chosen yet, is the error line `teams_list_read_only` and nothing is written.
The exit code is 1 when an error line was printed, and 0 otherwise.

**Exit codes**

Each command ends with an exit code scripts can test:

- 0 when it did its work;
- 1 when it did its work but reported an error;
- 2 when it was refused before writing anything (a bad argument, a file it cannot read, a team
  it cannot find);
- 3 when a write failed (what was written before it is kept).
