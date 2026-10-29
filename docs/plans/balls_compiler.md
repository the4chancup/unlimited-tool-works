# Unlimited Tool Works — Balls compiler plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The Balls compiler (`crates/tools/balls_compiler`, tool id `balls-compiler`, sidebar
label "Balls compiler") is the successor to the **4cc balls compiler 2.03**, a batch
file from the same era as the original AET compiler batch. It compiles a **balls
export** — the ball models, thumbnails, and menu list for PES's ball selection
screen — into its own CPK. Unlike the [Refs arranger](refs_arranger.md), this is a
real compiler: ball content has nothing to do with aesthetics exports (own game paths, own
`pesdb` bins, own CPK), so the tool owns its compile end to end, built on the
existing lib crates. Platform context is in the [core plan](core/README.md).

As with the AET compilers, the old tool defines *what* to produce, not *how* (see
"Relationship to the old compiler").

---

## Background

PES has a ball selection menu with up to **50 slots**. Each slot is a ball id
(`ball001`–`ball050`, assigned by list position) with:

- a **model**: Fox versions (PES 18+) load `ball.fpk` (an FPK containing `ball.fmdl`)
  plus FTEX textures from `Asset/model/ball/ballXXX/`; pre-Fox versions (PES ≤17)
  load loose `ball.model` + `ball.mtl` + DDS textures from
  `common/render/model/ball/ballXXX/`;
- a **thumbnail**: `common/render/thumbnail/ball/ball_XXX.dds`, shown in the menu;
- a **menu entry**: a record in `common/etc/pesdb/Ball.bin` (name, number, menu
  order), alongside a `BallCondition.bin`.

| Tool | Language | Status | Location |
|------|----------|--------|----------|
| 4cc balls compiler 2.03 | Batch file (+ GzsTool, hexed.exe, cpkmakec, pes-file-tools Python scripts) | Works, ancient | `Tools_Mine/4cc balls compiler/4cc balls compiler 2.03` |

The old tool keeps two parallel ball libraries (`Balls_fmdl/` for Fox, `Balls_mtl/`
for pre-Fox), a `balls_list.txt` of folder names (spaces forbidden), and per-ball
`name.txt` + `thumbnail\*.dds`. The community library is **100+ balls and sparse**:
many balls exist in only one of the two trees.

## Relationship to the old compiler

Replicate the game-facing output, not the mechanism. What disappears with the Rust
implementation:

- **External binaries**: GzsTool (FPK packing) → `fpk` crate; cpkmakec → `cpk`
  crate; hexed.exe (byte-poking `Ball.bin` together) → a typed `Ball.bin` writer;
  the pes-file-tools Python scripts (DDS↔FTEX) → `ftex`/`dds_convert`;
  `fmdl_id_change.py` → the `fmdl` crate's texture-path rewriting (the same
  mechanism the Team compiler uses); the admin batch dance → the `elevation` lib.
- **Temp folders** → in-memory processing (`vtree`), like every suite compiler.
- **The DpFileList appender** — dropped deliberately. Red removed its own appender
  ("Please never modify your dpfl") and the suite's policy is **check-only**. The
  Balls compiler reports equivalent tool-owned codes
  (`balls_dpfilelist_missing` / `balls_cpk_name_unlisted`) rather than reusing
  Team-compiler catalog identities.
- **`ball.fpk.xml`** — pure GzsTool boilerplate (a one-entry manifest), generated
  internally now; no longer part of the format.
- **`name.txt` shipped inside the CPK** — a 2.03 artifact (the batch moved it into
  the packed tree); the new output omits it.

## The balls export (the format)

Balls move from a tool-private library to an **export**, the suite's third export
kind after team and referee exports. It lives in `exports/` with its
siblings; the export name's first word being `balls` is the discriminant, exactly
as `refs` is for referee exports (the Team compiler skips `balls` exports with an
info message pointing here — see "Compilation walkthrough" step 2 in the
[Team compiler plan](team_compiler/README.md)). **One balls export at a time**: the cup has
one ball list, so finding several `balls*` exports is an error, like with referees.

```
balls 4cc/
├── balls.txt                  ← the list: one ball folder name per line;
│                                 position = menu order = ballXXX id
└── Balls/
    ├── 4CC Clover/
    │   ├── ball.fmdl          ← Fox model, and/or:
    │   ├── ball.model         ← pre-Fox model
    │   ├── ball.mtl           ←   + its material XML
    │   ├── materials.toml     ← glTF material properties (glTF folders only; see Unified model format plan)
    │   ├── ball_col.ftex      ← textures: any image format (DDS, FTEX, PNG, JPEG, BMP,
    │   ├── *.png / *.jpg      ←   WebP, TGA, TIFF) — referenced by stem, interchangeable;
    │   │                         see Unified model format plan
    │   ├── name.txt           ← optional display name (UTF-8, ≤135 bytes, one line);
    │   │                         falls back to the folder name
    │   └── thumbnail.dds      ← menu thumbnail, fixed root-level name
    └── ...
```

Improvements over the 2.03 folder format:

- **One unified folder per ball** instead of parallel `Balls_fmdl`/`Balls_mtl`
  trees. A folder may hold the Fox model, the pre-Fox model, or both; glTF
  (`ball.gltf`/`.glb`, with the PES_bone/PES_mesh extensions) is accepted as a third source, like
  player models. Ball folders containing glTF also support the `materials.toml`
  material files from the [Unified model format plan](model_format.md) (catch-all + name-matched
  `*.materials.toml`; no `.common` links — a balls export has no Common folder). The compiler picks the
  target version's native format and **converts through `model_convert`'s IR only
  when it is missing** — important for this library, where most balls exist in a
  single format. Textures convert
  per-target the same way (DDS→FTEX for Fox, FTEX→DDS for pre-Fox).
- **Folder names may contain spaces** (a batch tokenization limit, not a format
  need).
- **`name.txt` is optional** — the folder name is the fallback display name. The
  name is the file's text with a UTF-8 byte-order mark dropped and trimmed at both
  ends; a line break left inside it is an error naming the file. 2.03 kept the last
  line and passed a byte-order mark into `Ball.bin` as name bytes; no shipped
  `name.txt` has a second line or a mark, so nobody relies on either.
- **`thumbnail.dds` at the folder root** replaces the `thumbnail/` subfolder with
  its any-name `.dds`; a legacy `thumbnail/*.dds` is still accepted with a warning
  (used in memory, never rewritten — the message suggests renaming). More than one
  candidate (a root `thumbnail.dds` beside a legacy one, or two `.dds` in `thumbnail/`)
  is an error naming them: 2.03 silently took the last one its directory listing gave.

`balls.txt` keeps the 2.03 shape: one folder name per line, order defining both the
menu order and the `ballXXX` numbering, at most 50 lines. Blank lines are skipped and
each line is trimmed, as 2.03's tokenizer did. There is no comment syntax: a line
starting with `;`, which 2.03 dropped silently, is a folder name like any other, so a
commented-out ball shows as a missing folder instead of vanishing. Duplicates are
rejected: a ball listed twice spends a second of the 50 slots on the same model and is
a typo far more often than intent.

The *export* format knowledge (`balls.txt`, the folder layout) lives **inside the tool crate**:
unlike `aesthetics_export` it has exactly one consumer, so it earns no lib crate (core plan,
workspace guardrail 3). The `Ball.bin`/`BallCondition.bin` *record layouts* are the exception:
they are two of the Konami database tables the suite writes, so they live in `libs/pesdb`
([DB generator plan](db_generator.md)) with the other tables; the format reference below stays
here and the crate transcribes it.

## Compilation pipeline

Processed with rayon per ball, source export read-only (the one file this tool
writes, `balls.txt`, is its *editor* function — see "Saving"). Ordinary ball folders
are only a few MB, but archives, glTF data URIs, decoded images, and malformed inputs
make an unbounded all-in-memory assumption unsafe. The tool therefore uses the shared
`pipeline::MemoryBudget` at ball-folder granularity, charging source bytes, glTF
dependencies, decoded/converted intermediates, and packed output until the writer
drains them. For each listed ball at 1-based position N (id `ball%03d`):

1. **Model selection and dependency resolution** — pick the target version's format;
   convert via the IR if only another format is present; error if none is. A pre-Fox
   representation is an atomic `ball.model` + `ball.mtl` bundle: a missing or invalid
   MTL makes that representation unavailable, and FMDL/glTF→pre-Fox produces and
   commits both outputs together. glTF uses
   `model_convert`'s caller-supplied resolver contract exactly as the Team compiler
   does: external buffer/image URIs are canonicalized and restricted to the source
   ball folder, absolute/traversing/remote paths are rejected, and buffers/images are
   exactly-once, permit-charged dependencies rather than ancillary output files.
   All accepted image formats are valid material texture sources for native models and glTF alike,
   resolved by stem under the shared image contract. glTF URI dependencies are additionally
   resolved through the asset resolver; source-only raster images and buffers are never copied
   through as game files. Texture encoding follows the shared PES-version policy: BC7 is valid
   only on PES 19–21; PES18 and older use BC3 or eligible fully opaque color BC1 for BC7/raster
   sources. The [library crates plan](libs/README.md) owns alpha and normal-map exceptions.
2. **Fox path** (PES 18+): convert target-compatible DDS textures to FTEX; rewrite the FMDL's texture
   path table to the `ballXXX` id (paths of the form
   `/Assets/pes16/model/ball/ballXXX/...` — segment 5, `fmdl` crate); a directory of
   any other form is left as authored, since it may name a texture the game itself
   ships; pack `ball.fmdl` into `ball.fpk` (`fpk` crate) as its one entry, named
   `ball.fmdl` as 2.03's manifest named it, paired with the embedded generic
   `ball.fpkd` template; emit to `Asset/model/ball/ballXXX/#Win/` with textures
   under `Asset/model/ball/ballXXX/#windx11/`. Only the textures the rewritten FMDL
   names ship; any other file in the folder is `file_not_used` (W), as in the Team
   compiler. 2.03 shipped every `.ftex`, which the game never loads; the warning is
   also how a model that never names its own textures shows.
3. **Pre-Fox path** (PES ≤17): convert FTEX textures to DDS; emit `ball.model`,
   `ball.mtl`, and the textures the `.mtl` names, loose to
   `common/render/model/ball/ballXXX/`. No id rewriting — the `.mtl` uses relative
   `./` texture paths. Any other file in the folder is `file_not_used` (W): 2.03 copied
   the whole folder, `name.txt` and stray files included.
4. **Thumbnail** — emit as `common/render/thumbnail/ball/ball_%03d.dds` (missing:
   warning, ball kept, slot has no menu image — 2.03's behavior).
5. **Bins** — build `Ball.bin` (see the format reference below) with number = menu
   order = N and the display name; copy the embedded `BallCondition.bin` template
   verbatim; both to `common/etc/pesdb/`.
6. **CPK** — validate the configured name as shared `pipeline::CpkStem`, pack everything into one
   CPK (`cpk` crate; default stem
   `4cc_38_balls`), deploy it to the download folder (`elevation` lib when needed),
   and **check** the name is listed in `DpFileList.bin` — never write to it. Deployment follows the
   Team compiler's model: staged under `output/.staging/`, always attempted, degraded to
   `output/{name}.cpk` with the reason when it cannot happen, CLI-only `--no-deploy` for the
   deliberate case (see "Post-processing" in the Team compiler plan) — the shared `pipeline` crate
   carries the staging/promotion scaffolding so both compilers behave identically.

## Ball.bin (recovered format reference)

Verified against the binary shipped with 2.03 (there is no other documentation):

- Fixed **140-byte records**, one per ball, no header, little-endian:

| Offset | Type | Field |
|--------|------|-------|
| 0x00 | u16 | Ball number (1-based; must match the `ballXXX` asset id) |
| 0x02 | u16 | Menu order (1-based) |
| 0x04 | 136 bytes | Display name, UTF-8, null-padded (≤135 bytes + terminator) |

The tool keeps number = menu order = list position, as 2.03 did (the fields *can*
diverge; nothing uses that). On the name field's encoding: 2.03's hex writer
(CharLib's `str2hex`) turns out to be **byte-transparent** — its character map
covers the full 0x01–0xFF range, so `Ball.bin` received `name.txt`'s raw bytes
unchanged, whatever encoding they were saved in, with the 135 limit counting
bytes. A UTF-8 `name.txt` therefore produced UTF-8 in `Ball.bin`, but what
encoding PES actually renders is unverified (see Open questions).

**`BallCondition.bin`** is a static 1800-byte file the old tool never generates or
edits — only copies. It is carried as an embedded template verbatim. Its structure:
225 little-endian 8-byte records `u16 ball, u16 index, u32 code`, where `ball` runs
1-50, `index` counts the records from 1, ball 1 has 29 records with codes 0-28 and
balls 2-50 have four each with codes 0, 4, 5 and 13. What the codes mean is undecoded;
see Open questions.

## GUI

Same layout family as the [Refs arranger](refs_arranger.md): a storage box and an
editable list, with live validation via the `pipeline` lib's folder watcher.

- **Storage box**: every ball folder found in `Balls/`, showing its **decoded
  thumbnail** (`dds_convert` → egui texture) and display name; entries already on
  the list are marked. Entries are drag sources.
- **The list**: the `balls.txt` entries in order, each showing position number,
  thumbnail, display name, and a validation status color. Drag from storage to
  add/insert, drag within to reorder. The 50-cap and duplicate rejection are
  enforced at drop time.
- **Utility buttons**: **Add all** (append every unlisted ball), **Fill with
  random** (append randomly chosen unlisted balls until the list hits the 50-cap or
  the library runs out), **Sort A→Z** (by display name — ball names are usually
  prefixed by the owning team's name, so this groups by team), **Reverse**,
  **Remove all** (clear the list); per-entry: remove, move to top/bottom.
- **Validation** (per-ball statuses on both panels): no model for the target
  version and none convertible (E), pre-Fox `.model` without a valid
  `.mtl` pair (E for that representation), invalid `materials.toml` or glTF image
  dependency (E), model converted from another format (I), thumbnail missing (W),
  legacy `thumbnail/` location (W), name over
  135 bytes (E), listed folder missing (E), duplicate list entry (E), over 50
  balls (E), multiple `balls*` exports found (E, tool inactive until resolved).
  An error on any listed ball, or on the list, means the compile writes no CPK. A ball's
  id is its list position, so skipping one would either shift every later ball off the
  number the organizer sees (2.03 did that, with a pause and a message) or leave a gap
  in `Ball.bin` that PES may not accept; the statuses show the error before Compile, and
  the fix is to the list or the folder.
- **Save**: writes `balls.txt` into the export. Same convention as the Refs
  arranger, for consistency: write a sibling temporary file, flush it, and
  atomically replace the target so watchers never observe a half-written list;
  overwriting an existing `balls.txt` asks for confirmation, creating a new one
  doesn't. Plain-folder exports only (archives can't be written into; a notice is
  shown).
- **Compile**: saves first (same confirmation rules), then runs the pipeline
  in-tool, reporting through the common log panel/run strip widgets.

## CLI

```
studio balls-compiler compile            # find the balls export in the exports folder and compile
studio balls-compiler check              # validate without compiling
studio balls-compiler import-legacy "<path to old compiler folder>"
```

**`import-legacy`** migrates a 2.03 installation into a balls export: for each ball
name in the union of `Balls_fmdl/` and `Balls_mtl/`, merge both folders' contents
into one `Balls/<name>/` (the sparse library makes the union essential — most balls
exist in only one tree), move `thumbnail\*.dds` to `thumbnail.dds`, drop
`ball.fpk.xml`, and convert `balls_list.txt` to `balls.txt`, each line's first
whitespace-delimited token becoming the line (2.03 read only that token as the folder
name). Worth a subcommand rather than a manual merge at 100+ balls. Migration builds a
separate output and leaves the old library untouched.

When merging produces same-stem DDS and FTEX alternatives, **keep the DDS** and omit the
FTEX from the migrated folder: every 2.03 compile converted the DDS over its same-stem FTEX,
so the DDS is what the game showed and the FTEX never reached a CPK. Keep a lone DDS or
FTEX source as-is. This rule is confined to legacy import; ordinary compiler validation still
rejects ambiguous same-stem sources. Identical duplicate files coalesce; other differing duplicates
(including two DDS files, names, or thumbnails) are reported for user selection, not silently
overwritten.

## Settings

| Setting | Default | Purpose |
|---------|---------|---------|
| `cpk_name` | `4cc_38_balls` | Output `pipeline::CpkStem`, using the same filename-stem contract as the Team compiler |

`CpkStem` accepts 1–28 characters from ASCII alphanumeric, `_`, `-`, and `.` only (no spaces),
rejects separators/control characters/trailing dot/Windows reserved-device names or a
case-insensitive user-supplied `.cpk` suffix, and appends `.cpk` internally. PES version, PES folder
path, and exports folder path are common settings (core plan).

## Testing

In addition to model/bin/CPK fixtures, save tests observe repeated replacement of an existing
`balls.txt`: interrupted temporary writes never alter the old target, watcher reads see either the
complete old or complete new list but never a partial file, and atomic replacement is exercised
explicitly on Windows. Model fixtures cover a missing/invalid pre-Fox MTL making that representation
unavailable, atomic FMDL/glTF→`.model`+`.mtl` output, `materials.toml` parsing, native-model raster
texture sources, and resolved glTF image dependencies, all producing target PES textures. `CpkStem` boundary
tests are shared with the Team compiler, including empty/29-character, invalid-character,
reserved-name, explicit-extension, and case-collision cases. Legacy-import fixtures cover the
DDS winning a same-stem pair, single-format content surviving, a list line with trailing text
migrating as its first token, other conflicts remaining reported, and the original library
staying unchanged.

## Development phase

Phase 14 in the core plan's Development Plan. Needs the format/archive libs and
`model_convert` for native conversion (Phase 2), glTF support (Phase 7), and the shell
(Phase 8).

## Open questions

- **`Ball.bin` name encoding on the game side** — 2.03 passed `name.txt`'s raw
  bytes through, so the field's encoding was whatever the file was saved in; the
  shipped sample names are all ASCII. Whether PES decodes non-ASCII names as UTF-8
  (or a legacy codepage) needs an in-game test with a known non-ASCII name before
  the validator's byte-count limit is finalized.
- **`BallCondition.bin` semantics** — the records' codes are undecoded (the format
  reference above gives the layout). Carrying the template verbatim matches 2.03 and works
  in-game; decoding is only needed if a future feature wants to edit it.
- **Pre-Fox version quirks** — 2.03 treated PES 14–17 as one path and never
  zlib-compressed ball DDS textures (unlike the Team compiler's optional PES ≤17
  compression). Assumed fine as-is until a version-specific issue surfaces.
