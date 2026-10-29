# Unlimited Tool Works — Team compiler plan: Blue features to port

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Unimplemented Blue Features to Port

### Referee export processing (Red is the behavioral spec)

What exists of this is in `spec/team_compiler/blue_port.md` "Referee export processing (Red is the behavioral spec)".

**The referee body (`refkit`, PES 15-17).** On PES 18-21, until the Fox refkit is made, `fpc_off`
applies its settings preset alone there.

Drawn as a Fox collar, the marker's static painting: the referee step checks that the bundled
FMDL is painted that way (the pre-Fox prop is unskinned, no bone and no weight, and is the
game's own prop, so it is not checked).

**Preparing the slot mapping is a separate tool's job.** The slots are not drawn uniformly — each
PES version has measured slot appearance rates (flat per-slot chances on 17–21, a pattern table on
15/16 that demands matchday rotation) — and on PES 18–21 the community's Fox referee hook overrides
the draw from per-match lists. The [Refs arranger](../refs_arranger.md) edits `players.txt` inside the
refs export (against those tables, or as one slot per referee plus a weighted fallback fill when
the hook decides who appears) and, on Fox, the hook's `ref_lists.txt` beside it, then switches the
user to this tool for the compile — the arranger never compiles, and this tool's referee behavior
is unaffected by it: a saved `players.txt` arrives like any other file edit via the folder
watcher, and `ref_lists.txt` is not this tool's concern.

### Features that disappear in a compiled GUI app

What exists of this is in `spec/team_compiler/blue_port.md` "Features that disappear in a compiled GUI app".

| Feature | Red location | Why it disappears |
|---------|-------------|-------------------|
| Auto-update system | `updating.py` (22KB) | GUI "check for updates" dialog; Rust HTTP via `reqwest` |
| First-run wizard | `settings_management.py:67-128` | GUI settings dialog |
| Settings transfer between versions | `settings_management.py:130-211` | Settings versioning in the GUI |
| Interactive settings editing | `settings_management.py:23-66` | GUI form fields |
| Comment-preserving INI parser | `commentedconfigparser` dependency | Not needed; GUI manages settings |
| Log username cleaner | `log_username_clean.py` | `utw.log` writes the home directory as `~` (`spec/core/architecture.md` "Diagnostic logging") |
