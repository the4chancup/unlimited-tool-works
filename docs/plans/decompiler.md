# Unlimited Tool Works: Decompiler plan

> **Plan:** work not built yet. What the suite does now is described in `docs/spec/`.

A stub: the tool's purpose, the kinds it covers and its two paths, written so that each
kind's compile records what this tool needs before the first DLC that should carry it. The
rest of the plan (GUI, CLI, findings, Acceptance) is written just in time for its phase, which
comes after 0.0.2.

## Purpose

What went into a cup's DLC is sometimes lost: a team's aesthetics export, the referee export,
the ball list, a stadium, the billboards. The DLC is then the only copy. The Decompiler
rebuilds Studio exports from a DLC, whole or in parts, so they can be compiled again (for a
later cup, or for another PES version through the Compiler's own conversions).

It is one tool for every export kind, as the Compiler is (`core/README.md`, the "One Compiler"
row): an organizer recovering a cup opens its DLC once and finds every export in it.

| Kind | What comes out | Its manifest is planned in |
|---|---|---|
| Team aesthetics export | the export folder: player, kit and model folders, logos, its settings and list files | `team_compiler/pipeline.md` "Recovery manifest" |
| Referee export | the same, with its `players.txt` | the same section |
| Balls export | `balls.txt` and each ball's folder | `balls_compiler.md` "Recovery manifest" |
| Stadium export | one folder per stadium, with its own `Billboards/` | `stadium_compiler.md` "Recovery manifest" |
| Billboards export | the global billboard patterns | the same section |

## Opening a DLC

The organizer opens one CPK, or the folder that holds a DLC's CPKs. A DLC is several CPKs (the
teams' one or its parts in multi-CPK mode, the referees', the balls', the stadiums' and the
billboards'), and a file a manifest names is looked up in every CPK opened, so a folder is the
usual choice.

## Picking what to extract

A DLC holds many exports, and the organizer rarely needs a whole one: often one player's
folder, one kit, a logo or one ball. So the tool shows a tree of every export it can recreate
from what is opened: the kind, then the export, then the export's own folders (player folders,
kit folders, model folders, ball folders, a stadium's components and the rest), down to single
files. Each node has a tick box.

The organizer ticks a whole export, a folder, a subfolder or single files, and extracts the
selection into a folder of their choice, laid out as in the export (a player folder comes out
as a player folder, ready to drop into another export).

The tree is the same for both paths below: from a manifest it holds the exports' real names,
from the heuristic the names it could recover or invent, marked as such.

## From a recovery manifest

A CPK the Compiler wrote since a kind's manifest shipped holds one per export of that kind
(the sections the table above names). The Decompiler recreates the export folder under its
recorded name:

- each packed file back at its recorded source path;
- a file packed as it was, taken as it is;
- a converted one in its compiled form (an FTEX unwrapped to its DDS, a converted model kept as
  the game's format);
- the files packed beside the manifest (the export's settings, lists, colours and notes)
  copied out as packed;
- its link files recreated from the manifest's list of them.

The result recompiles to the same CPK. The report says, per file, whether it is the source
itself or its compiled form.

## Without one: the heuristic

A CPK compiled by the old tools (Red, the balls compiler's batch, the PES Stadium Compiler),
or by Studio before a kind's manifest, has none. The Decompiler then works from what the
game's own files say, kind by kind. The report says which names were recovered and which
invented.

- **A team**: every file carrying the team's ID at the time (the organizer gives the ID, or
  picks it from the DLC's list). Player folders are named from a save of that cup when its
  players' names match, by number otherwise; kit and model folders by their number. The
  settings the export carried are lost, apart from what that save still holds.
- **The referees**: the files of the referee slots, as a refs export with a `players.txt`
  naming a folder per slot found.
- **The balls**: `Ball.bin` gives each ball's slot, menu order and name, and the slot gives
  its model, textures and thumbnail, so the balls export comes back whole, its folders named by
  the balls' names.
- **Stadiums and billboards**: the stadium bins give each stadium's ID and name, and the ID
  gives its files. What a stadium export's folders are is settled with the kind's own plan
  (`stadium_compiler.md`, before Phase 9), and this path with it.

## Open questions

- Which kinds need the heuristic at all is asked of the maintainer (`docs/QUESTIONS.md`,
  "After the release"): it is the larger half of the tool, and a kind whose old DLCs nobody
  needs to recover can leave it out.
