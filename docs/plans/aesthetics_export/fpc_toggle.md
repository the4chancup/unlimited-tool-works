# Unlimited Tool Works — Aesthetics export plan: FPC toggle

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## FPC toggle (`fpc_on` / `fpc_off` marker files)

What exists of this is in `spec/aesthetics_export/fpc_toggle.md` "FPC toggle (`fpc_on` / `fpc_off` marker files)".

- **`fpc_on`** — an empty marker file in a player folder. When present, the compile applies the
  **FPC enable preset** from `libs/fpc` to the player's settings (the same
  preset the save editor's FPC toggle uses — one implementation, so the two tools can never drift);
  they reach the game like every other setting (on Fox in his table rows, on pre-Fox
  through the aesthetics patch).
- **`fpc_off`** — applies the disable preset (visible defaults) instead. Since an absent key takes
  its default, it differs from no marker only where an authored strip key, or an authored custom
  skin (7, PES 15 to 17, which the preset resets to 1), would otherwise apply, and in a
  `Midcup` export, where it writes the preset's IDs and no marker keeps the installed row.
- **Absent = no FPC preset applied**: the player's strip keys and boots/gloves IDs are what his
  `settings.toml` and its defaults say. No earlier FPC state survives a compile without the
  marker (except in a `Midcup` export, where a boots or gloves ID the folder does not set keeps
  its installed row, so `fpc_off` is what resets a hidden player's IDs there), since a compiled player's appearance is his file and nothing else (see "Player settings
  in exports"). Model-derived ID assignments still apply.
- Either preset **overrides** the keys it sets (the strip keys and `skin_color`) in the folder's
  `settings.toml`; explicitly-set keys that get overridden raise `fpc_strip_conflict`.
- In a `players.txt` multi-mapped folder, the preset applies to all mapped players (like the rest of
  the folder's settings).

How a folder's models, an authored stock `boots_id`/`gloves_id` and the marker's preset IDs
resolve, independently per category and for both markers, is the numbered resolution of "Player
settings in exports" in [settings.toml](settings_toml.md). Other preset fields still apply
normally.
