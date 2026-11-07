# Unlimited Tool Works — Team decompiler plan

> **Plan:** work not built yet. What the suite does now is described in `docs/spec/`.

A stub: the tool's purpose and its two paths, written when the Team compiler's recovery
manifest was planned, so the manifest records what this tool needs. The rest of the plan
(GUI, CLI, findings, Acceptance) is written just in time for its phase.

## Purpose

A team's aesthetics export is sometimes lost, and the DLC of a cup it played in is then the
only copy of its look. The Team decompiler rebuilds a Studio aesthetics export from such a
DLC's CPK, for one team, so the team can be compiled again (for a later cup, or another PES
version through the compiler's own conversions).

## From a recovery manifest

A CPK the Team compiler wrote since the manifest shipped holds one per export
(`plans/team_compiler/pipeline.md` "Recovery manifest"). The decompiler recreates the export
folder under its recorded name: each packed file back at its recorded source path, a file
packed as it was taken as it is, a converted one in its compiled form (an FTEX unwrapped to
its DDS, a converted model kept as the game's format), and the TOML files copied out as packed.
The result recompiles to the same CPK; the report says, per file, whether it is the source
itself or its compiled form.

## Without one: the heuristic

A CPK compiled by Red, or by Studio before the manifest, has no manifest. The decompiler then
gathers every file carrying the team's ID at the time (the organizer gives the ID, or picks it
from the DLC's list), and names the folders as best it can: player folders from a save of that
cup when its players' names match, numbered names otherwise; kit and model folders by their
number. The settings the export carried are lost, apart from what that save still holds. The
report says which names were recovered and which invented.
