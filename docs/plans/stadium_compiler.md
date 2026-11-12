# Unlimited Tool Works: Stadium compiler plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The stadium compiler (`crates/tools/stadium_compiler`) is the successor to the PES
Stadium Compiler. Platform context is in the [core plan](core/README.md). The format crates
it needs are built: `fox2` ([spec](../spec/libs/fox2.md)) and the in-process DDS conversion,
`dds_convert` ([spec](../spec/libs/dds_convert.md)).

---

## Background

The PES Stadium Compiler (`Tools_4cc/pes-stadium-compiler-v1.0.0`) is a Python tool that compiles
stadium export folders into CPK files, using a three-stage pipeline like Red:

1. **exports_to_extracted**: validate stadium exports, copy components to
   `extracted_exports/{Component}/st{sid}/`, convert DDS textures to FTEX, remap stadium IDs in all
   files (FMDL, XML, fox2.xml, FPK/FPKD)
2. **extracted_to_contents**: generate fox2 files, pack FPK/FPKD archives, assemble the directory
   structure (stadium content and billboard content separately)
3. **contents_to_patches**: generate stadium database bin files and pack everything into a CRIWARE
   CPK

It becomes the `stadium_compiler` tool crate in the suite.

## Stadium and billboard exports

Stadiums and billboards are compiled as export kinds of the suite's one Compiler, beside team,
refs and balls exports, not by a tool of their own (DECISIONS 2026-11-09, "one Compiler for
every export kind"). The crate layout is settled when Phase 9 opens.

Each kind goes into its own CPK in the run, so the old tool's "compile mode" (stadium CPK,
billboard CPK, or both) becomes the Compiler's choice of what to compile.

- **A stadium export** is one folder per stadium in `exports/`, its name starting with the
  marker word `stadium`. A stadium belongs to a team, but council members rather than
  managers usually provide it, so a team export never has to carry one.
- **Its stadium ID comes from a list**, as the old tool's `stadium_list.txt` (`63
  4ccg_spankneseum`: the ID, then the folder). Later, maybe: the IDs ordered by team name,
  as the balls list's order should be.
- **Billboards** come two ways, as in the old tool: a **billboards export** (marker word
  `billboards`) holds global patterns (`4cc`, `hdg`: the old `billboards_to_add/`), and a
  stadium export's own `Billboards/` holds its stadium's (the old
  `stadiums_to_add/<folder>/Billboards/`). The billboards export is required. A stadium's
  own `Billboards/` is supported too, unless Phase 9's planning finds a cost against it.
- **One stadium CPK and one billboard CPK** hold every stadium and every billboard (the old
  defaults: `4cc_30_stadiums0`, `4cc_15_billboard`). A stadium is independent as a folder,
  never as a CPK: a CPK per stadium would put forty CPKs in the DLC.

## Recovery manifest

The stadiums' CPK and the billboards' CPK carry their exports' recovery manifests, as a team's
CPK carries each team export's (`team_compiler/pipeline.md` "Recovery manifest", which sets
the scheme and its reasons): `4cc/exports/stadium/<export folder name>/` per stadium export in
the stadiums' CPK, and `4cc/exports/billboards/<export folder name>/` in the billboards' CPK.

Each holds a `manifest.toml` (the kind, the export's folder name, the Studio and PES versions,
every file packed with its source path and conversion, and for a stadium the stadium ID the
list gave it) and, as files, the export's files that the CPK holds under no path of their own.
A stadium's own `Billboards/` is packed into the billboards' CPK, while its manifest stays with
the stadium: the Decompiler looks a manifest's paths up in every CPK it has open.

Which files those are is settled with the export's format, when Phase 9 opens, and so is
where a decompiled stadium export carries its ID for the next compile, the list being no part
of the export. The Decompiler (`decompiler.md`) rebuilds the exports from them.

## New formats to port

| Format | Source | Size | Placement |
|--------|--------|------|-----------|
| **Stadium DB bins** (Stadium.bin per version) | `Engines/stages/lib/st_bin_gen.py` | 9KB | record layout in `libs/pesdb` ([DB generator plan](db_generator.md)), the stadium-specific content rules in `tools/stadium_compiler` |

## Tool-specific logic to port

| Component | Source | Size | Notes |
|-----------|--------|------|-------|
| Stage orchestration | `stadium_compiler.py`, `stages/*.py` | 61KB | Maps to the same reader/coordinator/writer pattern as the Team compiler |
| Components handling | `util/components.py` | 27KB | Stadium component validation and copying |
| Billboards | `util/billboards.py` | 30KB | Billboard content processing |
| Packing | `util/packing.py` | 16KB | FPK/FPKD + directory assembly |
| ID remapping | `util/remap.py` | 2KB | Stadium ID rewriting across file types |
| Stadium configs | `util/stadium_configs.py` | 7KB | Per-stadium configuration |

The stadium pipeline emits the same `PipelineEvent` stream as the Team compiler, so the GUI progress
grid works for stadium compilation without modification (rows = stadiums, cells = components). Its
warnings/errors likewise use the shared `Message` structure with a tool-owned `MessageCode` catalog,
to be specified when the tool is planned in detail (see "Message catalog" in the [Team compiler
plan](team_compiler/messages.md)).
