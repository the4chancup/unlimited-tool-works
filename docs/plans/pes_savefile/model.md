# Unlimited Tool Works — Savefile plan: Data model

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Player settings model (settings.toml)

What exists of this is in `spec/pes_savefile/model.md` "Player settings model (settings.toml)".

- No save is stripped (`operations.md` "Aesthetics patch (new)"), so the spec's parts for
  that route go: `EditFile::appearance_row` and the rule that only the strip writes the
  appearance id's -1 with the Save editor's strip (worklog 8.36b), `PlayerSettings::appearance_row`
  and the Fox player tables with the compiler's table writer (8.36c). Generation reads every
  record as the player's look, with no skip for an id at -1.
