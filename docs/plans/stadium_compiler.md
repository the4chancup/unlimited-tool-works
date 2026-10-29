# Unlimited Tool Works — Stadium compiler plan

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
plan](team_compiler/README.md)).
