The Export upgrader migrates an export in the old (AET) layout to the new
format the Team compiler reads. Its commands run from a terminal opened in the
folder that holds `UTW.exe`.

```text
UTW export-upgrader upgrade <export> [--savefile <EDIT> | --loose] [--coverage full|midcup] [-o <dir>]
UTW export-upgrader upgrade-models <folder> [-o <dir>]
```

`upgrade` takes one old export: a folder, a `.zip` or a `.7z`. A team export
takes `--savefile`, naming the save (`EDIT00000000`) that says which player
wears which boots and gloves, or `--loose` to guess the wearer of each boots
or gloves folder from its ID alone. A referee export (a name whose first word
is `refs` in any letter case) takes neither.

With `--loose` each boots or gloves folder goes to the player at its ID's
position in the team's old block of 25 IDs, the block its kind's lowest ID
falls in (`k0128` is player 03 of the 126 block). Each pairing is reported as
`wearer_assumed`, so it can be checked. An ID on the block's last two
positions or past it belongs to no player and is reported as
`unmigrated_content`. Without a save no look is written. A player folder with a name still
gets a `settings.toml` whose only setting is `name = true`.

The output is a new folder beside the input, or inside the folder `-o` names
(created when missing). Its name is the team's key with the coverage tag
inserted (`co - Spring` becomes `co Full - Spring`, an export whose name holds
`midcup` or `additions` in any letter case, and no tag as its second word,
gets `Midcup`), or the tag
`--coverage full` or `--coverage midcup` sets instead. A name whose second
word is already a tag in any letter case keeps it and refuses a flag naming
the other, and a referee export's name takes no tag and refuses `--coverage`.
The tag used is always reported (`coverage_tag_chosen`, naming what decided
it), because an export that was only a mid-cup update gets `Full` when its
name says neither, and a `Full` export's first compile builds placeholder
kits over the team's installed ones. When that name is already the input's
own, letter case aside, ` upgraded` is appended so the original is never written over. An output
folder that already exists stops the run and is left untouched.

A referee export is upgraded from its `refs.txt`. The output gets a
`players.txt` holding the same slots in the new spelling (two-digit slot
numbers, one entry per line) and no `refs.txt`, and its `ref_marker.dds` is
carried as it is. A roster file that is not UTF-8 is reported, and no
`players.txt` is written. The roster may also be the older `Refs list.txt`
when there is no `refs.txt`. With no `Players/` folder, the referee folders
stand directly in the export. The migration is the same. The old `face/`, `boots/`,
`gloves/` and `common/` subfolders of each referee's folder are folded into
that folder, and a file held twice with the same bytes is kept once. Two
files landing on one path with different bytes, two textures landing on one
stem from different places (a `hair.dds` and a `hair.ftex`), or a subfolder
of one of those four names inside them, leave the referee's folder as it was
and are reported. In a referee folder that folds, a `common/` model no
`face.xml` of the referee names is left out and reported. A referee export
with neither `refs.txt` nor `Refs list.txt` has no roster to migrate, so it
is reported (`refs_txt_missing`, or `no_old_export` when it holds nothing
else the migration reads) and nothing is written.

`upgrade-models` takes a folder of old model folders instead: the face, boots
and gloves folders lying directly in it, one level deep, plus an optional
`Common` folder. Each becomes one folder to drop into an export. A face
folder becomes a player folder `NN - Name` (its number and name from its own
name, or its own name alone when it has no number), a boots or gloves folder
becomes a folder named by its name part (`k0430 - Crocs` becomes `Crocs`) or
by its ID when it has none, and `Common` is upgraded as an export's would be.
The report names each output folder and where it goes: a player folder drops
into `Players/`, `Common` at the export's root, and a boots or gloves folder
goes into `Boots/` or `Gloves/`, where you add a link file to the folder of
each player who wears it, or move its files into a player folder with the
model renamed `boots.fmdl` or `glove_*.fmdl`. A top-level folder holding no
file directly in it (such as a folder of player folders) is reported as
`unmigrated_content` and left out. No team
is known, so a `???` placeholder and the goalkeeper `u0???g` token stay as
written. The output is the folder `<folder> upgraded` beside the input, or
the folder `-o` names, which must not exist yet.

The original export is never modified. A file, a folder or a Team Note entry the migration
cannot place yet is listed in the report as `unmigrated_content`, the line naming the cause
and, where another file or folder already took the place, that one's path.
Content that never reached a player in game (a stray file, a folder nobody
wore) is dropped and listed as `leftover_dropped`, a warning that does not
stop the migration.
The upgraded folder is checked against the same rules the Team compiler
applies. Every finding prints as one line naming the export it is about, when
there is one, how serious it is, its code, where in the export it is, when it
is about a file or folder, and its details in parentheses, when it has any.
A run that gets as far as its verdict ends the report with `migration_clean`
or `migration_not_clean` (nothing is written in the second case); a finding
that stops the run at once is its last line instead. An incomplete `<name>.upgrading` folder an
earlier run left behind is removed and reported (`staging_folder_removed`)
before this run writes its own. One that cannot be removed still aborts the
run.

The exit code is 0 when the migration is clean. It is 1 when the migration is
not clean, after an error finding or after a finding that drops anything but
a leftover (a warning that drops a folder counts, because the compile would
lose it). It is 2 when the command was refused before anything was written
(a bad argument, an input that is not there or not of the kind the command
takes, a savefile that does not read
as a save or does not hold the team). It is 3 when the run aborted, either
before anything is written (the output folder exists, a leftover `.upgrading`
folder cannot be removed, the teams list cannot be read or parsed, or the output cannot
be created) or when the output cannot be written, moved into place or removed
and the incomplete output is left.
