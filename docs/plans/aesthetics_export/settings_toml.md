# Unlimited Tool Works — Aesthetics export plan: settings.toml

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

Part of the [Aesthetics export plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Player settings in exports (settings.toml)

What exists of this is in `spec/aesthetics_export/settings_toml.md` "Player settings in exports (settings.toml)".

**One route on every engine: the savefile.** The spec's Fox half, the database tables the
output CPK carries over a stripped save, goes (worklog 8.36c): on every engine everything
resolved goes into the aesthetics patch, which the compiler writes and the save editor
applies, and the patch carries the whole look (`plans/pes_savefile/operations.md` "Aesthetics
patch (new)"). A compile writes a patch anyway for the motions, which live only in the save,
so the tables never made a team's looks independent of the save. Where the spec's steps say
"his row of the installed table" or "his installed row (Fox)", they read "the save's current
value" on every engine, as pre-Fox already does. Three consequences: on Fox a DLC whose patch
was not applied shows none of its compiled looks (the CPK alone carried them before); a
sideloaded look change reaches the game only through the save step; and a manager's Edit-mode
change to a look stays in the save until the next compile's patch writes over it, as it did
before the tables.

**An invalid `settings.toml` keeps the player's installed look.** Today `settings_toml_invalid`
ignores the file and the player compiles with the defaults in its keys' place, so on Fox his
appearance row is written from the defaults and his look resets in game while his models
compile: one mistyped value loses every setting he had. Instead, the keys the invalid file
would have set are left out of the compile, as a failed boots output leaves the installed ID:
the patch holds no appearance entry for him, so the save keeps his record.
`ingame_face.toml` and his models still apply as today. This is a
third exception to the spec's "his export and nothing else", beside a failed boots or gloves
output and a `Midcup` export's defaulted categories, of the same kind: content the export
failed to state keeps what is installed rather than a default nobody chose. The finding's
consequence text changes with it (`spec/team_compiler/messages.md`'s row).

The save editor view can also **generate** these TOML files from an existing savefile, giving teams
a migration path from the current workflow. Generation (`PlayerSettings::from_player`, shared with
the Export upgrader) writes every key the save's version holds,
so a generated file reproduces the player as he is, whatever the defaults; it emits
`boots_id`/`gloves_id` only for a stored ID from 1 to 100: 0 (the default boots, normal hands)
stays unset, which is its default (the upgrader writes it in a Midcup output, where unset keeps
the installed ID; the save editor, not knowing the export's coverage, cannot), and an ID above 100 is custom content, which the upgrader
migrates into a folder or link (the old per-team blocks start at 101 too) and the save editor
leaves out. It writes `name` per the Export upgrader's rule (`true` when the folder's name part
equals the save's name, the explicit string otherwise; the save editor, generating without a
folder, writes the string), `shirt_name` as the save's string, and no `height` or `weight`.
With no save stripped, a record is always the player's look: the save editor's generation
and the Export upgrader generate from it for every player.

**Edits made in PES come back to the export.** PES is the previewer for what only the game
shows well (the motions, played in context; the in-game face, built with its sliders), so
members keep tuning those in Edit mode, and Studio brings the result into the export's files
instead of the member copying it by hand. Without it an Edit-mode edit is lost: the next
compile's patch writes his motions and his look from his files, an absent key as its
default, over the edit.

- **Found from state, not from watching.** PES writes the save only when the member leaves
  Edit mode, and Studio may not have been running then, so nothing depends on seeing the write
  happen. Each compile leaves a **compile record** beside its aesthetics patch in the output
  folder: per compiled player, his save id, his export folder, and every value of his files'
  keys as the compile resolved them, plus the save's fingerprint (size, mtime, hash) once the
  compile's save step ran or was skipped. Studio compares the save against the record whenever
  the fingerprint differs: at start, when the Team compiler's or the Save editor's view opens,
  before every compile, and on the save file's change while it runs (the watch only makes it
  sooner). A key whose save value differs from the record's resolved value is an edit made in
  PES, for the motions and the appearance block's keys (the face sliders, skin, physique,
  strip) on every compiled player: the save holds both. The boots and gloves IDs are compared
  as generation writes them: a stock ID (0 to 100) that differs comes back as the key, a
  custom one is folder content and is named in the notice, not brought. His name, unless the
  export carries `write_names`, is not compared.
- **Into the export at once.** A notice lists the players and keys ("3 players changed in PES:
  the motions of 07 - Snuffy, the face of 10 - Pepe") with **Bring into the exports**, which
  writes the keys through `PlayerSettings::from_player`'s conversion with `toml_edit`, keeping
  the files' comments: the motions and the appearance keys into `settings.toml`, the face, hair
  and iris keys into `ingame_face.toml`. A player whose folder holds a face model is asked
  first (he does not wear the in-game face). A key the member also changed in the file since
  the compile shows both values, and the member picks one; nothing is overwritten unasked. The
  record is updated to the brought values, so the notice does not return. Declined, it returns
  at the next check, and a compile over an unbrought edit says so before it writes the save
  (a Warning naming the players), never overwriting it silently.
- **Into the save only when PES is closed.** PES writes the save from its own memory at every
  Edit-mode exit, so any write Studio makes while it runs is lost at the next one. A compile's
  pending patch (`savefile_skipped_pes_running`) waits until the running-PES probe sees the
  game closed; the shell then offers **Apply to the save** (at once, when Studio starts with
  PES not running).

Before this is built, an in-game check measures which keys an Edit-mode edit writes (the
comparison's list above is read from Test 1 of the old stripped route, not measured key by
key).

The record-to-keys conversion is `pes_savefile`'s `PlayerSettings::from_player`, a lib both
tools already reach, never a copy in a tool crate: the Team compiler finds and brings the
edits, and one tool crate never depends on another.

The in-game hair ranges of PES 16 to 20, which no game measured, are assumed from PES 21's: the
spec's rule for an unmeasured range (its field's bits, for generation and the compiler's
narrowing alike, built for the face types: `spec/pes_savefile/operations.md` "Cross-version
player conversion") applies to them too. They have no per-version table yet, every version
sharing PES 21's measured range, and no real save holds a hair value past it (the fixture
saves' round trip, `every_fixture_player_round_trips_through_his_settings`), so the table is
built when such a value is found.

---
