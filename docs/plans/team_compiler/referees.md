# Unlimited Tool Works: Compiler plan: Referee exports

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Referee export processing

What exists of this is in `spec/team_compiler/referees.md` "Referee export processing".

**The referee body (`refkit`, PES 15-17).** On PES 18-21, until the Fox refkit is made, `fpc_off`
applies its settings preset alone there, with no body merged into the referee's face package.

The marker, drawn as a Fox collar, stays on the ground by its static painting: the referee step
checks that the bundled FMDL is painted that way (the pre-Fox prop is unskinned, no bone and no
weight, and is the game's own prop, so it is not checked).

**Preparing the slot mapping is a separate tool's job.** The slots are not drawn uniformly. Each
PES version has measured slot appearance rates: flat per-slot chances on 17–21, and on 15/16 a
pattern table that demands matchday rotation. On PES 18–21, the community's Fox referee hook
overrides the draw from per-match lists.

The [Refs arranger](../refs_arranger.md) edits `players.txt` inside the refs export, either
against those tables or, when the hook decides who appears, as one slot per referee plus a
even fallback fill. On Fox it also edits the hook's `ref_lists.txt` beside it. Then it
switches the user to this tool for the compile.

The arranger never compiles, and this tool's referee behavior is unaffected by it: a saved
`players.txt` arrives like any other file edit via the folder watcher, and `ref_lists.txt` is not
this tool's concern.
