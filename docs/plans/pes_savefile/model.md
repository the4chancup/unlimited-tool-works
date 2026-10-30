# Unlimited Tool Works — Savefile plan: Data model

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Player settings model (settings.toml)

What exists of this is in `spec/pes_savefile/model.md` "Player settings model (settings.toml)".

- No save is stripped (`operations.md` "Aesthetics patch (new)"), so the spec's
  `PlayerSettings::appearance_row` and the Fox player tables go with the compiler's table
  writer (worklog 8.36c).
