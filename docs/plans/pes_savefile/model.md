# Unlimited Tool Works — Savefile plan: Data model

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Player settings model (settings.toml)

What exists of this is in `spec/pes_savefile/model.md` "Player settings model (settings.toml)".

- Generation's callers skip a record whose appearance id is -1 (`settings_toml.md`, the end
  of "Player settings in exports"); `from_player` itself reads whatever record it is given.
