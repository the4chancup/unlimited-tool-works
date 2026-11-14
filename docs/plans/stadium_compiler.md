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
- **Its models sit in `Models/`**, where the old tool's exports have `#Win/`: `#Win` is the
  game's own name for its Windows asset folder, which tells a member nothing, while
  `Models/` says what goes in it. The Compiler writes the files under `#Win/` in the CPK,
  where the game looks. It refuses a stadium export's `#Win/` (the compiler stays strict),
  and the Export upgrader renames it once, as it migrates every old layout
  (`AGENTS.md` "Fundamental concepts", the upgrader absorbing the old format's slack).
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

## Stadium export format

A stadium export keeps the old tool's layout (`Engines/stages/extracted_from_exports.py`,
`util/components.py`), with three changes:
- `#Win/` is `Models/`;
- each file has one name, where the old tool took three for the picture (`Thumbnail.dds`,
  `Thumb.dds`, `st<ID>.dds`) and two for the audience folder (`Audience/`, `Audiarea/`);
- names match in any letter case, as a team export's folders do
  (`aesthetics_export/player_folders.md`), where the old tool compared them exactly and read
  the lowercase `pitch/` and `name.txt` of real exports only because Windows ignores case.

```text
stadium Club Room/        the export; its name starts with the marker word
├── Name.txt              the name the game lists the stadium under (required)
├── Thumbnail.dds         its picture on the stadium select screen (required)
├── Models/               the stadium's FMDLs (required); df/ dr/ nf/ nr/ hold one weather's own
├── Textures/             DDS (converted to FTEX) or FTEX, for every model of the stadium
├── Pitch/                the pitch's FMDL
├── Audience/             audiarea.bin, and the crowd's PNGs
├── Cheer/                a1/ and a2/, the two cheering crowds' FMDLs
├── Scarecrow/  Staff/  TV/  Big Flag/  Stands Flag/  Ad/
├── Corner Flag/          models/ and motion/ (PES 2019 on)
├── Lights/  Effects/  Draw Parameter/    per weather: lights, effects, brightness
├── Billboards/           the stadium's own billboard pattern
└── Other/                files written as they are, at the same path in the CPK
```

The weathers are `df`, `dr`, `nf` and `nr`: day fine, day rain, night fine, night rain
(`util/settings.py`).

- **Only `Models/`, `Name.txt` and `Thumbnail.dds` are required.** The Compiler fills every
  missing folder as the old tool does (a stock stadium's lights, empty archives, generated
  brightness and TV settings), so a first stadium needs nothing more.
- **A folder holds loose files or the old tool's premade FPK and FPKD**, as it accepts them,
  never both: the Compiler cannot tell which of the two the member meant to ship.
- **Every `stNNN` the export's files and names carry becomes the list's ID**, as
  `util/remap.py` rewrites them: an export made from another stadium names its donor's ID
  throughout. The Compiler rewrites every name and file the old tool's rewrite reaches,
  always: the old tool rewrote nothing when its scan of FMDL, FPK and fox2 files found no
  other ID, so a `.json` or `.xml` naming one shipped with it.
- **The CPK's paths are the game's**, `Asset/model/bg/st<ID>/#Win/` for the models and
  `.../sourceimages/tga/#windx11/` for the textures: `Models/` is a name in the export only.
- **PES 2018** has no corner flag (the Compiler warns and skips `Corner Flag/`), shorter
  name fields in `Stadium.bin` (121 bytes, against 181 from PES 2019), and its textures
  in DXT5, to which the old tool converts a BC7 or BC5 one.

### Pre-Fox stadiums

No pre-Fox stadium compile is planned, pre-Fox support being second-class, but the format
above stays open to one, should a cup go back to PES 2015-2017.

The last pre-Fox tool (`Tools_4cc/pre-fox-stadiums/`, `compile.py`, VGL23's edition) copies
an export's component folders, `audi`, `bigflag`, `cheer`, `effect`, `pitch`, `scarecrow`,
`sky`, `stadium`, `staff` and `tv`, as they are into `common/bg/model/bg/`. Each holds the
game's own tree for that component, with the stadium's `stNNN` folders inside, its models
`.model` files with an `.mtl` and an `.xml` per weather, and its DDS textures in the
component's own `texture/` folders, which the `.mtl` files name by path. A `thumb/` folder
holds the picture and `Note.txt` the slot and the name.

The same export holds one:
- **The engine follows from `Models/`**: `.fmdl` files make it a Fox stadium, `.model`
  files a pre-Fox one, and both an error.
- **A pre-Fox component folder holds the game's tree for it, as it is**: `Models/` the
  `stadium` tree (its `etc/` folder holds the lights and effects), `Pitch/` the `pitch`
  tree, `Audience/` `audi`, `Cheer/`, `Scarecrow/`, `Staff/`, `TV/`, `Big Flag/` `bigflag`,
  `Effects/` `effect`, and `Sky/`, a pre-Fox folder only. `Textures/` stays unused, since
  each tree keeps the texture folders its `.mtl` files name.
- **`Lights/`, `Draw Parameter/`, `Stands Flag/`, `Corner Flag/` and `Billboards/` are
  an error** in a pre-Fox export: the engine has no such component.
- **`Name.txt` and `Thumbnail.dds` are the Fox ones**, and the ID comes from the list, so
  `Note.txt` has no counterpart (the upgrader would turn its `Name:` line into `Name.txt`).
- **Only the stadium's own ID is rewritten**, the one its trees' `stNNN` folders are named
  (two different ones are an error), in names and in `.mtl` and `.xml` text, as the old
  tool's `ChangeSlot` did. A pre-Fox `.mtl` may name another stadium's textures
  (`model/bg/stadium/st016/...`), which the game reads from that stadium's own files.
- **The bins are the same four files at the same paths**, and `Stadium.bin`'s 272-byte record
  is PES 2018's. The two order bins differ between the two pre-Fox tools (`compile.py` and
  `stadium17.py`, beside it): which the game needs is an in-game test, made when a pre-Fox
  stadium compile is planned.

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
