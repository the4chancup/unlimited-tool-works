# Unlimited Tool Works — Library crates plan

Covers the small shared lib crates: the binary format parsers (`cpk`, `fpk`, `fmdl`,
`pes_model`, `ftex`, `fox2`, `uniparam`, `wezlib`), `dds_convert`, `color_tools`,
`vtree`, `pes_version`, `archives`, `elevation`, `fpc`, and `teams_list`. `cpk` also reads an
install's `download/DpFileList.bin` (`cpk::dp_file_list`, for the Team compiler's working-bin
walk and the Save editor's portraits; the layout is in the Team compiler's
[pipeline](../team_compiler/pipeline.md) "DpFileList upgrade"). The aesthetics export format's object model, folder conventions,
and validation live in `aesthetics_export` (shared by the Team compiler, Export upgrader,
Kit config editor, Refs arranger, and Team creator), specified in the
[aesthetics export plan](../aesthetics_export/README.md). The two big libs have their own plans:
[Model conversion](../model_conversion/README.md) (`model_convert` + the mesh algorithms in
`fmdl`/`pes_model`) and [Savefile](../pes_savefile/README.md) (`pes_savefile`). The `pipeline`
crate (reader/writer scaffolding, memory budget, folder watcher, check cache, shared `CpkStem`
validation) has its contract in [pipeline](pipeline.md); the reasons are in the
[core plan](../core/README.md) (Parallelism) and the
[Team compiler plan](../team_compiler/README.md) (walkthrough, live validation). The music lib
crates (`music_export`, `audio_engine`) are specified in the
[Music player plan](../../plans/music_player.md), the live-match event feed (`match_feed`)
in the [Match tracker plan](../../plans/match_tracker/README.md), the kit config codec
(`kit_config` — binary/TOML, reverse-engineered format documentation) in the
[Kit config editor plan](../kit_config_editor.md), the AATF rules engine
(`aatf`) in the [AATF rules](../aatf_rules.md), the shared
tactics/card widgets (`team_widgets`) in the [Save editor
plan](../save_editor.md), the fox2 parser in
[fox2](fox2.md), and the Konami database tables (`pesdb`) in
the [DB generator plan](../../plans/db_generator.md). Platform context is in the
[core plan](../core/README.md).

---

The plan is split into parts; section headings are unchanged from the
single-file plan.

| Part | Covers |
|---|---|
| [Format crates](format_crates.md) | Recommended approach |
| [fox2](fox2.md) | `libs/fox2`: Fox Engine entity files |
| [archives](archives.md) | `libs/archives`: reading `.zip` and `.7z` exports |
| [dds_convert](dds_convert.md) | In-process DDS conversion (replaces texconv) |
| [color_tools](color_tools.md) | `libs/color_tools` |
| [elevation](elevation.md) | `libs/elevation` |
| [fpc](fpc.md) | `libs/fpc` |
| [teams_list](teams_list.md) | `libs/teams_list` |
| [pipeline](pipeline.md) | `libs/pipeline`: memory budget, thread count, `CpkStem` |

## Format references in Blue

| Format | Blue file | Size | Notes |
|--------|-----------|------|-------|
| CPK (CRI archive) | `utils/cpk.py` | 550 lines | UTF table with XOR encryption, read + write |
| FMDL (Fox Model) | `utils/FmdlFile.py` | 1700 lines | Largest parser; segmented binary format |
| SKL (Fox Skeleton) | — | ~150 lines | Bone list + bind-pose transforms; reference parser at `examples/skl.py`; lives in the `fmdl` crate (same Fox format family) |
| FPK/FPKD | `utils/fpk.py` | 137 lines | Fox package with MD5 checksums |
| FTEX | `utils/ftex.py` | 400 lines | PES texture with mipmap/BCn block math |
| UniformParameter | `utils/uniparam.py` | 91 lines | Kit config container |
| CRILAYLA | `utils/crilayla.py` | 72 lines | Bitstream LZ decompression |
| WESYS/zlib | `utils/zlib_plus.py` | 146 lines | zlib wrapper with custom header |
| VirtualTree / `ScopePath` | `utils/vtree.py` | 187 lines | In-memory file tree plus the canonical absolute-within-export path type (case-insensitive lookup with explicit collision checks) |

## `vtree` path types

`vtree::ScopePath` is the canonical export/event path. It normalizes separators and
rejects traversal, absolute paths, and empty/noncanonical segments. Case-folded and
Unicode-normalized collisions are detected before a path enters a tree.

## `pes_version`

`pes_version::PesVersion` is the closed set of supported versions (`Pes15`–`Pes21`) with
`engine()` (`PreFox` for 15–17, `Fox` for 18–21), `number()`/`year()`, `Display` (`PES 2021`),
`FromStr` (`21`, `2021`, `pes21`) and serde as the two-digit number. It is its own leaf crate,
not a type inside `pes_savefile`, because `studio_core`'s common settings, `model_convert`'s
skeleton tables, `kit_config` and the compilers all take it and none of them may depend on the
savefile crate (or on `studio_core`). Version-specific *facts* never live here: an offset, a key
or a path is data in the crate that owns the format, looked up by version, never a `match` on
`PesVersion` outside that crate.

## Format references in converters

| Format | Source | Size | Notes |
|--------|--------|------|-------|
| .model (PreFox) | `ModelFile.py` | ~1700 lines | Binary format, struct.unpack → binrw |

## External tool dependencies

| Tool | Current use | Rust replacement |
|------|-------------|-----------------|
| `7z.exe` | Extract `.7z` exports | `sevenz-rust2` crate (native, no external binary) |
| `texconv.exe` | DDS DXT5 conversion (rare) | **Dropped** — in-process conversion (see below) |
| `magick` (ImageMagick) | Texture conversion on Linux | **Dropped** — same in-process path on all platforms |

The `sevenz-rust2` crate eliminates the 7z subprocess and temp-folder extraction dance.

## Testing: format parser unit tests

Each binary format parser has unit tests with real file samples. Fixtures live in the crate's own
`tests/fixtures/` (modularity over a shared tree; a small sample needed by several crates is
copied, a large one is not duplicated without a decision). Real PES-derived files may be committed,
kept minimal so cloning stays fast: the smallest file that exercises the code path, one per format
variant rather than one per team, an FPK or FMDL rather than the CPK that contained it.
```rust
#[test]
fn cpk_roundtrip() {
    let original = parse_cpk("test_data/sample.cpk");
    let bytes = serialize_cpk(&original);
    let reparsed = parse_cpk_bytes(&bytes);
    assert_eq!(original, reparsed);
}
```

`color_tools` extraction is tested on synthetic textures painted into the region constants
(every pick-order step, the merge and distinctness boundaries, the buffer contract) and on
one real oracle independent of those constants: the community's colored template sheet,
subsampled to 128x128, whose shirt zone is two reds and whose shorts zones are two yellows,
counted by a script rather than by the crate. Managers' `colors.txt` entries are *not* a
usable ground truth (`color_tools.md` "Calibration evidence": only 55 of 133 declared shirt
colors appear in the shirt region at all), so no tolerance test against them exists; the
thresholds came from the visual swatch-sheet check the calibration harness produced.

---
