An `unmigrated_content` line names content the old export was meant to show in
game, which the migration could not place. While any such line stands, the run
writes nothing: fix the cause in the old export and upgrade again.

The path after `at` is the file or folder the line is about, then `reason=`
names the cause, one of the values below. Where two files or folders claim one
place, `other=` names the one that has it. Some causes add `line=` for the
Team Note entry, `error=` for why a file does not read, or `name=` for a
folder.

**Face folders and portraits**

- `slot_invalid`: the face folder's number, the two digits after its name's
  first three characters (`15` in `XXX15 - Snuffy`), is not a player number
  from 01 to 23 (`XXX24`, `ab15x`). Rename the folder so it is.
- `slot_taken`: a second face folder claims a player's number; `other=` names
  the folder that has it. Merge their files into one, or rename the one meant
  for a different player.
- `portrait_taken`: two portraits claim one player, a second `Portraits/` file
  of one number or two files ending `portrait.dds` inside one face folder.
  Keep one of them.

**Boots and gloves folders**

- `id_invalid`: the folder's name is not a boots folder's `k` or a gloves
  folder's `g` and four digits, then the name part (`K0430`, `k430`). Rename
  it to that form, the letter lowercase, like `k0430 - Crocs`.
- `id_taken`: a second boots or gloves folder carries one ID; `other=` names
  the folder that has it. Keep one of the two, or renumber it.
- `loose_unplaceable`: under `--loose` the folder's ID sits on a spare position
  of its kind's block of 25 IDs, or past the block, so it is no player's.
  Rename it to an ID of the block, or upgrade with `--savefile` so the save
  says who wears it.

**Kit configs, kit textures and the Team Note**

- `config_name_invalid`: the file's name is no config name
  (`XXX_DEF_1st_realUni.bin`, `_GK1st_` for the goalkeeper's). Rename it to
  that form, or remove it if it is no kit config.
- `config_too_deep`: the file sits deeper than the one `Kit Configs` subfolder
  the old layout read. Move it up into `Kit Configs/` or its subfolder.
- `config_taken`: a second config of one kit, different in content; `other=`
  names the one kept. Keep one of the two.
- `config_undecodable`: the file does not read as a kit config (`error=` says
  why). Replace it or remove it; a kit left with no config still gets its
  folder, a placeholder the compiler fills.
- `kit_entry_past_ninth`: a player kit entry of the Team Note past the ninth
  (`line=` the entry): there is no kit number for it. Remove it, or mark it
  `gk:` when it was meant as the goalkeeper's.
- `colors_unreadable`: a Team Note entry's colors do not read, a team-color
  entry's or a kit entry's (`line=` the entry). Fix the line: a color is
  `#RRGGBB` or three numbers (`211 74 79`), and a kit entry's two colors split
  at ` - `.
- `texture_undecodable`: an image file does not read, a kit `.ftex` that
  fails its conversion or a PNG in `Logo/` (`error=` says why). Save it again
  in a readable form, or replace it.
- `texture_name_taken`: two kit textures would land on the same name in the
  upgraded kit (`u0XXXp1.dds` beside `u0XXXp1.ftex`); `other=` names the one
  kept. Keep one of the two.

**The `Logo` folder**

- `logo_member_invalid`: the file is not a logo image the old format reads:
  it sits in a subfolder, or its name ends in none of `_r_ll.png`, `_r_l.png`,
  `_r.png`. Rename it to the ending it was meant as, or move it out of
  `Logo/`.
- `logo_role_taken`: a second file claims one of the three endings, two
  ending `_r.png` for example; `other=` names the one kept. Keep one file per
  ending.

**Models and other files**

- `mtl_unparsable`: the `.mtl` cannot be read even after stray text inside it
  is dropped (`error=` says why). Fix the file in a text editor, or remove it
  from the folder.
- `path_taken`: two files would land on the same path in the upgraded export
  once renames apply (`A.dds` beside `a.dds`, or numbered copies of one kit
  texture, `u0731p2.dds` beside `u0799p2.dds`); `other=` names the one kept.
  Keep one of the two.
  A copy spelled with letters (`u0XXXp2.dds`) beside numbered ones is no
  conflict: it is the one the game loaded, so it is kept and each numbered
  copy is dropped as a leftover.
- `note_taken`: a second `.txt` at the export's root whose name holds `note`;
  `other=` names the file read as the Team Note. Keep one of the two.
- `unclaimed`: the file sits where nothing reads it: directly in `Players/` or
  inside a folder the export does not use (a `Players/` folder in a team
  export, `Kit Configs/` in a referee export). Move it into a folder the
  export reads, or remove it.

**Referee exports**

- `model_unreferenced`: a model in the referee's `common/` that no `face.xml`
  in his folder names. Remove it, or name it in a `face.xml`.
- `name_taken`: two files from the referee's `face/`, `boots/`, `gloves/` or
  `common/` subfolders would land on the same path in his folder, with
  different content (`other=` the other file). Keep one of the two.
- `stem_taken`: two textures from different subfolders would land on the same
  stem in the referee's folder, a `hair.dds` and a `hair.ftex` (`other=` the
  other file). Keep one of the two.
- `nested_layout`: one of the referee's `face/`, `boots/`, `gloves/` or
  `common/` subfolders holds another folder of the same four names inside it
  (`face/face/`). Move the inner folder's files up into the outer one.
- `roster_not_utf8`: `refs.txt` is not UTF-8, so no `players.txt` can be
  written. Save it with UTF-8 encoding.
