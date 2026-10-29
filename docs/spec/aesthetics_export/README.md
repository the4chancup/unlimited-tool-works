# Unlimited Tool Works — Aesthetics export plan

Covers the `aesthetics_export` lib crate and the **Studio aesthetics export format** it describes: the object
model and its validation progression, the player-folder format, `settings.toml` in exports, and the
FPC marker files. The format is shared knowledge — the [Team compiler](../team_compiler/README.md) compiles
it, the [Export upgrader](../export_upgrader.md) writes it, the [Kit config editor](../kit_config_editor.md)
edits kit folders inside it, the [Refs arranger](../../plans/refs_arranger.md) edits referee slot allocations
inside it, the [Player aesthetics editor](../../plans/player_aesthetics_editor.md) browses and converts player
folders in it, and the Team creator generates it — so one plan and one crate own it.
Compile-time behavior (what the compiler *does* with an export) stays in the Team compiler plan;
model-level format details (glTF + PES extensions, `materials.toml`) are in the [Unified model format
plan](../model_format.md). Platform context is in the [core plan](../core/README.md).

---

The plan is split into parts; section headings are unchanged from the
single-file plan.

| Part | Covers |
|---|---|
| [Object model](object_model.md) | Object Model |
| [Player folders](player_folders.md) | Player folders (the Studio export format) |
| [settings.toml](settings_toml.md) | Player settings in exports (settings.toml) |
| [FPC toggle](fpc_toggle.md) | FPC toggle (`fpc_on` / `fpc_off` marker files) |
