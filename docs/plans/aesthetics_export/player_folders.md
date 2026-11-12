# Unlimited Tool Works — Aesthetics export plan: Player folders

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Player folders (the Studio export format)

What exists of this is in `spec/aesthetics_export/player_folders.md` "Player folders (the Studio export format)".

The Export upgrader generates all the root files from the old Team Note txt and `Logo/` folder when
migrating.

**A base copy: the `<id>.copy` link file.** A player folder holding an empty file named after a
player id with the `.copy` extension (`5223.copy`; a stray `.txt` suffix is accepted, as for every
link file) wears that player's face and hair the game's own way: the aesthetics patch writes the
id as his base copy (Team TOML's `base_copy_id`) instead of clearing it.

Members use copies on purpose, for a player with no face of his own: a stock player's face,
another board's player, a teammate cloned (a census of PES 2019 cup saves, 2026-10-31: /sp/'s
NGUBU copying 101334, /4ccseeks/'s nineteen MR. MEESEEKS copying their #02, VGL 15's Backup 1
copying all of /akg/).

A copy is the face category's link: at most one in a folder, and refused beside any face of
the player's own (a face model, a `.face` link, an `ingame_face.toml`), since the game shows the
copy over his own face and the two contradict each other. The id is a number other than 0 and
the player's own id (both are the record's "no copy"). Whether a stock id exists in the game's
database is not checked, the compiler having no database to look in.

Each refusal is an Error that drops the player folder, as the other link rules do
(`shared_link_duplicate`, `ingame_face_explicit_face_model`):

- `copy_link_duplicate` for a second copy;
- `copy_link_with_face` for a copy beside a face of his own;
- `copy_link_id_invalid` for a name that is not a number, is 0, or is the id of a player the
  folder maps to (a folder that `players.txt` maps to several players is checked against each;
  a teammate's id is a valid copy).

A file rather than a `settings.toml` key: the folder listing shows that a player wears a copy,
where a key would leave a folder that looks empty until its settings are opened, and the export
already names what a player wears by file (models, `.face`, `.boots` and `.gloves` links) while
Team TOML holds the ids they become (`boots_id`, `gloves_id`, `base_copy_id`).

## Complete export (an aesthetics export with its tactical file and its music)

A team's aesthetics export may carry:

- its tactical half as a file at its root, `tactics.toml`: a Team TOML
  (`plans/pes_savefile/operations.md` "Team TOML") written by the Save editor's export;
- its music export as a root folder, `Music/`.

So one folder holds everything a team sends to a cup, to be compiled for any version. Each part
is optional: an aesthetics export alone, a tactical export alone (a Texport or a Team TOML) and
a music export alone keep working as they do. The names follow every export's rule, folder
names capitalized and file names lower case (`Players/`, `Kits/`, `settings.toml`).

- **Each half owns its fields, so no field has two sources.** The aesthetics half owns the
  looks (the player folders, `settings.toml`, kits, the team's root files); `tactics.toml` owns
  names, gameplay and tactics, and carries no looks: an appearance table or a kit or colour key
  in it is an export-check error naming the table, and the Save editor's export offers the
  tactical file without them (Team TOML's "absent = untouched" makes such a file valid).
- **Names come from the tactical file when there is one.** The aesthetics patch writes names
  only for an export without `tactics.toml`, which keeps the autopilot teams' route (the
  `write_names` marker) as it is; a marker beside `tactics.toml` is a warning saying the
  tactical file's names are used.
- **One fixed name, recognized everywhere.** `tactics.toml` at the root is a known root file
  for the Team compiler, the Export upgrader (which carries it over untouched) and the export
  checks (which parse it as a Team TOML, its texts against the portable limits); any other
  `.toml` at the root stays `root_file_unexpected`, as now.
- **`Music/` is the music export as it stands alone**: one `.4ccm` directly inside (its file
  name free, as music exports' are) and the songs it names, resolved against that folder as a
  `.4ccm`'s always are (`plans/music_player.md` "File resolution"), so the folder copied out
  loads in the Music player or in Rigdio unchanged.

  It is a known root folder of a team export: the Team compiler never reads its songs and packs
  nothing of it, the Export upgrader carries it over untouched, and the export checks parse its
  `.4ccm` and resolve its songs. What they find is a Warning, never an Error, because no
  compiled output depends on it (a missing goalhorn cannot spoil a DLC). A `Music/` holding no
  `.4ccm` or several, and a `.4ccm` whose `name` is not the export's team (the music of another
  team bundled by mistake), are Warnings of their own.

  A referee export has no music: `Music/` there is an unexpected root folder, as now.

**Who uses it.** An official cup keeps two exports, because it has two deadlines (aesthetics,
then tactics) and often two builders: the DLC builder never sees the tactics. A Complete export
is made at the end of a cup, to collect a team so it can be entered easily into an invitational,
which has one deadline and one organizer building both the DLC and the save.

So the compiler applies `tactics.toml` in its savefile stage, after the aesthetics patch, when a
local save is configured (the "Savefile update" stage, `spec/team_compiler/pipeline.md`).
Without one it is checked and otherwise left alone, and the compiler carries nothing of it into
its outputs.

The streamer is usually someone else, and the organizer often gathers every team's music into
one download for them. With the music inside each export, that download is the entered exports'
`Music/` folders, one per team, and the Music player's auto team loading finds each by its
`.4ccm`'s `name`, with a folder of whole Complete exports working as its library too
(`plans/music_player.md` "Autopilot").

**The two halves name the same players.** Team TOML addresses players by roster slot, the
aesthetics export by savefile identity (`team id × 100 + NN`). By 4cc rule a team's roster slot
`NN` holds player `team id × 100 + NN`, so slot and identity agree by definition and nothing
reconciles them.

PES lets a roster be reordered, so the rule is checked rather than assumed:

- the AATF check reports a team that breaks it (`plans/aatf_rules.md` "The host");
- the compiler refuses to apply `tactics.toml` onto such a team in the local save, with an
  error naming the slots, since its players would land on the wrong ids.
