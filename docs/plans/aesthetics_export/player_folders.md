# Unlimited Tool Works — Aesthetics export plan: Player folders

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Player folders (the Studio export format)

What exists of this is in `spec/aesthetics_export/player_folders.md` "Player folders (the Studio export format)".

The Export upgrader generates all the root files from the old Team Note txt and `Logo/` folder when
migrating.

At compile time, after the spec's steps 0 to 3, the pipeline:

4. **Plans the settings side** for assigned IDs and independent accepted settings — conditional
   savefile mutations on every engine (the compiler writes no player table rows). Producer
   commits activate mutations that reference compiled assets; after all outcomes are known,
   only activated asset mutations plus eligible independent settings are serialized, into the
   aesthetics patch (`plans/pes_savefile/operations.md` "Aesthetics patch (new)"). This
   makes the player wear successfully emitted boots/gloves without pointing at failed content — the
   coordination currently done manually with 4ccEditor.

## Complete export (an aesthetics export with its tactical file and its music)

A team's aesthetics export may carry its tactical half as a file at its root, `tactics.toml`:
a Team TOML (`plans/pes_savefile/operations.md` "Team TOML") written by the Save editor's
export, and its music export as a root folder, `Music/`, so one folder holds everything a team
sends to a cup, to be compiled for any version. Each part is optional: an aesthetics export
alone, a tactical export alone (a Texport or a Team TOML) and a music export alone keep working
as they do. The names follow every export's rule, folder names capitalized and file names
lower case (`Players/`, `Kits/`, `settings.toml`).

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
  loads in the Music player or in Rigdio unchanged. It is a known root folder of a team
  export: the Team compiler never reads its songs and packs nothing of it, the Export upgrader
  carries it over untouched, and the export checks parse its `.4ccm` and resolve its songs.
  What they find is a Warning, never an Error, because no compiled output depends on it (a
  missing goalhorn cannot spoil a DLC); a `Music/` holding no `.4ccm` or several, and a
  `.4ccm` whose `name` is not the export's team (the music of another team bundled by
  mistake), are Warnings of their own. A referee export has no music: `Music/` there is an
  unexpected root folder, as now.

**Who uses it.** An official cup keeps two exports, because it has two deadlines (aesthetics,
then tactics) and often two builders: the DLC builder never sees the tactics. A Complete export
is made at the end of a cup, to collect a team so it can be entered easily into an invitational,
which has one deadline and one organizer building both the DLC and the save. So the compiler
applies `tactics.toml` in its savefile stage, after the aesthetics patch, when a local save is
configured (the "Savefile update" stage, `spec/team_compiler/pipeline.md`); without one it is checked and
otherwise left alone, and the compiler carries nothing of it into its outputs. The streamer is
usually someone else, and the organizer often gathers every team's music into one download
for them: with the music inside each export, that download is the entered exports' `Music/`
folders, one per team, and the Music player's auto team loading finds each by its `.4ccm`'s
`name`, with a folder of whole Complete exports working as its library too
(`plans/music_player.md` "Autopilot").

**The two halves name the same players.** Team TOML addresses players by roster slot, the
aesthetics export by savefile identity (`team id × 100 + NN`); by 4cc rule a team's roster slot
`NN` holds player `team id × 100 + NN`, so slot and identity agree by definition and nothing
reconciles them. PES lets a roster be reordered, so the rule is checked rather than assumed:
the AATF check reports a team that breaks it (`plans/aatf_rules.md` "The host"), and the
compiler refuses to apply `tactics.toml` onto such a team in the local save, with an error
naming the slots, since its players would land on the wrong ids.
