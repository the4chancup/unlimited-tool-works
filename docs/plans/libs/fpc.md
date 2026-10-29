# Unlimited Tool Works — Library crates plan: fpc

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## `libs/fpc`

What exists of this is in `spec/libs/fpc.md` "`libs/fpc`".

The hide preset's boots and gloves IDs become an argument: `preset(kind, ids: HideIds)`, where
`HideIds { boots: u16, gloves: u16 }` carries the common setting (`plans/save_editor.md` "Full
Player Customization (FPC)"), `HideIds::CONVENTION` is 4cc's 55 and 11 (the settings'
defaults), and `HideIds::BOOTS` and `HideIds::GLOVES` are the valid ranges (1 to 100, 11 to
100). They replace `NONEXISTENT_BOOTS_ID` and `NONEXISTENT_GLOVES_ID`: a constant beside the
setting would be a second source every caller could reach for.

Consumers through `kit_config` and `pes_savefile` still to come: the Save editor (FPC toggle,
Appearance-tab warnings), the Kit config editor (FPC indicator), and the Player aesthetics
editor (marker toggles).
