# Unlimited Tool Works — Aesthetics export plan: settings.toml

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

Part of the [Aesthetics export plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Player settings in exports (settings.toml)

What exists of this is in `spec/aesthetics_export/settings_toml.md` "Player settings in exports (settings.toml)".

The spec's engine split has a second half:

- **Pre-Fox (PES 15–17): in the savefile**, through the aesthetics patch the compiler writes and
  the save editor applies, as before. The database route is untested there.

In the spec's compile-time flow, which ends on the per-engine split, the pre-Fox half:

- **Pre-Fox:** everything resolved goes into the aesthetics patch.

**An invalid `settings.toml` keeps the player's installed look.** Today `settings_toml_invalid`
ignores the file and the player compiles with the defaults in its keys' place, so on Fox his
appearance row is written from the defaults and his look resets in game while his models
compile: one mistyped value loses every setting he had. Instead, the keys the invalid file
would have set are left out of the compile, as a failed boots output leaves the installed ID:
on Fox his installed appearance row is carried into the output, read where a `Midcup`
export's defaulted categories read theirs (spec step 3), and the defaults stand only when no
installed row exists; on pre-Fox the patch holds no appearance entry for him, so
the save keeps his record. `ingame_face.toml` and his models still apply as today. This is a
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
On a stripped Fox save a record whose appearance id is -1 is not the player's look (the tables
are, see "Stripped save" in the [Save editor plan](../save_editor.md)), so the save editor's
generation skips that player with a note; the face-editor route restores his id before
generating. The Export upgrader generates from the record all the same: for a team the
compiler has never compiled, the record still is the look its rows were seeded from (its step 8).

**Edits made in PES come back to the export.** PES is the previewer for what only the game
shows well (the motions, played in context; the in-game face, built with its sliders), so
members keep tuning those in Edit mode, and Studio brings the result into the export's files
instead of the member copying it by hand. Without it an Edit-mode edit is lost twice over:
the edit restores the player's appearance id on a stripped Fox save (`spec/save_editor.md`
"Stripped save", Test 1), so the game shows the frozen record instead of new compiles, and the
next compile writes his motions from `settings.toml`, an absent key as its default, over the
edit.

- **Found from state, not from watching.** PES writes the save only when the member leaves
  Edit mode, and Studio may not have been running then, so nothing depends on seeing the write
  happen. Each compile leaves a **compile record** beside its aesthetics patch in the output
  folder: per compiled player, his save id, his export folder, and every value of his files'
  keys as the compile resolved them, plus the save's fingerprint (size, mtime, hash) once the
  compile's save step ran or was skipped. Studio compares the save against the record whenever
  the fingerprint differs: at start, when the Team compiler's or the Save editor's view opens,
  before every compile, and on the save file's change while it runs (the watch only makes it
  sooner). A key whose save value differs from the record's resolved value is an edit made in
  PES: for the motions on every compiled player (the save holds them), for the appearance
  block's keys (the face sliders, skin, physique, strip) only on a player whose appearance id
  is restored, since a stripped record is not his look. The frozen record's boots and gloves
  IDs (written 0 by the freeze) and his name, unless the export carries `write_names`, are not
  edits and are not compared.
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
  Edit-mode exit, so any write Studio makes while it runs is lost at the next one. The re-strip
  of the restored ids (`check-stripped --strip-again`'s operation) and a compile's pending patch
  (`savefile_skipped_pes_running`) wait until the running-PES probe sees the game closed; the
  shell then offers **Apply to the save** (at once, when Studio starts with PES not running).
- **The restart.** A restored player shows his frozen record for the rest of that game's run:
  PES holds the restored id in memory, and only a restart reads the re-stripped save. The
  notice says so in one line ("his compiled look returns after PES restarts"), so a member
  iterating models by sideloading sees why one player stopped updating. Before this is built,
  an in-game check measures it, with which keys Edit mode writes into a frozen record (the
  comparison's list above is read from Test 1, not measured key by key).

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
