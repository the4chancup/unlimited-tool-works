# Unlimited Tool Works — Core plan: Development plan

Part of the [Core plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Development Plan

Phases 1–2 build the platform skeleton and every lib crate that can be verified on its own (against
fixtures or reference outputs); the tools follow, each bringing the consumer-shaped libs it is the
first to need. Each phase's detailed specification lives in the corresponding plan document: phase
2 in [libs](../libs/README.md)/[model conversion](../model_conversion/README.md)/[savefile](../pes_savefile/README.md), phases
3–4 in [team compiler](../team_compiler/README.md), phase 5 in [savefile](../pes_savefile/README.md)/[save
editor](../save_editor.md)/[AATF rules](../aatf_rules.md), phase 6 in [export upgrader](../export_upgrader.md), phase 7 in [model
conversion](../model_conversion/README.md), phase 9 in [stadium compiler](../../plans/stadium_compiler.md), phase 10 in
[music player](../../plans/music_player.md)/[music export editor](../../plans/music_export_editor.md), phase 11 in [match
tracker](../../plans/match_tracker/README.md), phase 12 in [kit config editor](../../plans/kit_config_editor.md), phase 13 in [refs
arranger](../../plans/refs_arranger.md), phase 14 in [balls compiler](../../plans/balls_compiler.md), and phase 15 in
[player aesthetics editor](../../plans/player_aesthetics_editor.md); the post-release phases 17–20 name their
plans in their sections.

**Target versions.** The cup moves from PES 17 (pre-Fox) to a Fox version, not yet chosen, around
April 2027, and plays Fox versions for at least a year after that; Studio's first-class target is
therefore that Fox version, and PES 15–17 support covers what the plans already give them. A
change to the game's own behavior that Studio's output relies on (gloves on outfield players,
the referee hook, LiveCPK) is made in **FoxDen**
(`Tools_4cc/FoxDen`), the community's runtime for PES 18–21, rather than in a patched executable:
FoxDen applies the 4cc exe patches at runtime to the stock exe, so one installed file carries
them. FoxDen runs on the Fox games' Lua engine and cannot exist on pre-Fox versions; a pre-Fox
equivalent is built only if a pre-Fox version becomes the cup's game again, so a feature that
needs one is Fox-only until then.

### Phase 1: Workspace bootstrap + core skeleton

Done (2026-09-13). The guardrails above apply "from the first commit", so the first commit is the
one that makes them enforceable:

- The root `Cargo.toml` is a virtual workspace (`crates/studio`, `crates/studio_core`,
  `crates/libs/*`; `crates/tools/*` joins with the first tool crate, since cargo rejects a member
  glob that matches nothing). `[workspace.dependencies]` declares every shared crate once, and
  `[workspace.lints]` holds the policy every member inherits via `[lints] workspace = true`:
  `rust::missing_docs` (the `///`-on-every-`pub` rule) and `clippy::let_underscore_must_use =
  "deny"` (a discarded `Result` is a compile error unless handled or `#[expect]`ed with a reason,
  see the code style rule in `../../CONTRIBUTING.md`), plus `dbg_macro`, `print_stdout`,
  `print_stderr` and `todo` at `warn`, which `-D warnings` makes red; `studio`'s CLI result
  output and `extra_mutants`'s stderr carry the `#[expect]`s. `[profile.dev.package."*"] debug =
  false` builds dependencies without debug info while workspace crates keep theirs, since
  dependency debug info is the bulk of a dev `target/` (measured on a comparable project: 24 GB
  of `debug/deps/`) and is never stepped into. `rustfmt.toml` (`style_edition = "2024"`, LF);
  `rust-toolchain.toml` pins `1.98.1` with `clippy`, `rustfmt` and the `wasm32-unknown-unknown`
  target, so rustup installs all of them on the first `cargo` call.
- The root `justfile` is the single definition of the gates — `just gates` runs the fmt check,
  clippy with `-D warnings`, `cargo test --workspace`, and `scripts/wasm_check.py`, which derives
  the crate list for the `wasm32-unknown-unknown` check from `cargo metadata`: `studio_core` plus
  every crate under `crates/libs/` minus a named exclusion list, so a new lib is checked without
  anyone remembering to list it (the acceptance-ID scanner and the section-pointer check joined
  later) — and of `just deps-check` (guardrail 4 through
  `scripts/deps_check.py`, which walks each guarded crate's normal dependencies over every target
  and every feature; plus the `cargo deny` license allowlist in `deny.toml`). On Windows the
  recipes run under PowerShell (`set windows-shell`): Git for Windows' `sh` is not on PATH from a
  plain PowerShell, the fallback `../../CONTRIBUTING.md` named; a planted clippy warning was verified to
  turn `just gates` red there. The remaining recipes the plan names (`acceptance`, `parity`,
  `bindings`, `release`) arrive with the phases that give them something to run.
- CI (`.github/workflows/ci.yml`) runs `just gates` on Ubuntu and Windows and `just deps-check`
  on Ubuntu, installing `just` and `cargo-deny` as prebuilt binaries; the `python_bindings` build
  job is added in Phase 2 with the crate.
- `crates/libs/`, `crates/tools/`, `crates/studio/`, `crates/studio_core/` exist, so every later
  crate lands in its planned place.

The non-GUI parts of `studio_core` exist as the closed module tree above prescribes: `tool.rs`
(the `StudioTool` trait as specified under `architecture.md` "Tool plugin interface", `ToolContext`, and the
`ShellRequest` enum the context queues for the shell: `SwitchTool`, `Notify`, `SettingsChanged`),
`events.rs` (the `architecture.md` "Event system" types, with `Message` and `MessageCode { tool_id, code }` as the
Team compiler plan's "Message structure" defines them), `status.rs` (`ShellCondition`,
`ToolActivity`, `Notice` with at most one typed `NoticeAction`), `help/mod.rs` (the `HelpSection`,
`HelpTopic`, `HelpTarget` types only; search, rendering and export come with the GUI), `settings/`
(`Settings::load` / `parse` / `save` / `merge_defaults`, the `[common]` + per-tool-table layout
under `gui.md` "Settings menu", `CommonSettings` with the defaults the Team compiler plan lists and the
update-state keys; saving is atomic through a `.tmp` rename and the caller decides what a failed
save means) and `shell/launch.rs` (the three launch modes, `-v` verbosity, one clap subcommand per
registered tool named after its id, duplicate or reserved ids rejected at registration, and
`run_cli` dispatch). Two leaf libs came with it: `vtree` (`ScopePath`, `RelativeScopePath`,
`VirtualTree<T>`, see `../libs/README.md`) because `PipelineEvent` addresses by `vtree::ScopePath`, and
`pes_version` because `CommonSettings` holds a `PesVersion`. `crates/studio` came as a stub binary:
it parses the launch mode, installs the CLI `env_logger` sink, and dispatches to a registry that
was still empty; the GUI modes exited with a message (the window and the Team compiler's
registration are Phase 3's shell slice; the rest of the GUI is Phase 8's).

**Verification (done):** all four gates green on the workspace and `just deps-check` green; 38
unit tests, including settings round-trip and recursive default-merge, `ScopePath` rejection and
collision cases, and a stub tool registered through the trait and dispatched from
`UTW stub ping x`. CI's first green run and one deliberately red run are recorded in the
worklog when the first push happens.

### Phase 2: Library crates

Every lib crate that can be verified on its own — against real-file fixtures, the legacy tools'
reference outputs, or documented data — exists, built before any tool: leaf crates first, then
their dependents, `python_bindings` last. Libs whose API is shaped by a consumer
(`aesthetics_export`, `pipeline`, `aatf`, `music_export`, `audio_engine`, `match_feed`) wait for
the phase of the tool that first needs them. Each crate went through a converge audit (worklog
2.20): a whole-crate mutation run with every survivor triaged, a public-surface audit, a census
of every file of its format on the maintainer's machine for the format readers, and the
cross-family reviewer loop.

**Format codecs**, with `binrw` where the layout is declarative (`fpk`, `ftex`, `fmdl`,
`pes_model`) and hand-written readers elsewhere. Blue and the stadium compiler's parser variants
(`fmdl_file.py`, `cpk.py`, `ftex.py`) were format evidence checked against fixtures, not
implementations translated:
- `wezlib`
- `cpk` (read + write, CRILAYLA inside)
- `fpk`
- `ftex` + `dds_convert` (`image`, and `block_compression` for CPU BC1/BC3/BC7 encoding and for
  decoding): the CPU reference only. The desktop GPU BC7 backend (`libs/README.md` "First-release desktop
  GPU BC7") is built in Phase 16, after 0.1.0, on top of Phase 4's texture step, not here: every
  rule the plan gives it (bounded batches against the memory budget, cancellation, writer
  progress, one encoded result shared by all consumers, the fallback report) is pipeline
  integration, so a Phase 2 version would be shaped without its caller and reshaped once. The
  cold-start and upload/readback measurements are the first thing the GPU step does, before
  relying on it. BC7 output is PES 19–21 only; PES 15–18 use BC3 or eligible opaque-color BC1 — see
  [libs](../libs/README.md).
- `fmdl` — `format/` (read/write, byte-identical round-trip), `ops/` (mesh splitting, split vertex
  encoding, anti-blur, multi-FMDL merging, path-table editing), `check.rs`
- `pes_model` — `format/` (.model + sibling .mtl), `ops/` (mesh splitting, split vertex encoding,
  merging, texture stem rewriting), `check.rs`
- `uniparam`
- `fox2` (+ CityHash64, verified against fixtures)
- `archives` (.zip/.7z via `zip` and `sevenz-rust2`)

**Data and rule leaves:**
- `fpc` (Full Player Customization as data: kit values, player presets, interference rules —
  dependency-free, so it precedes `kit_config` and `pes_savefile`; spec in [libs](../libs/README.md))
- `teams_list` (`TeamName` fold, `TeamId` range, `teams_list.txt` parse/write/reconcile with the
  embedded upstream list — dependency-free; fixture: Red's current file; spec in [libs](../libs/README.md))
- `kit_config` (binary ↔ TOML kit config codec, applies `fpc::kit` values — format documentation
  in the [Kit config editor plan](../kit_config_editor.md))
- `color_tools` — dominant kit-color extraction only; the picker widget is Phase 8
- `elevation`

**`model_convert`** (spec: [Model conversion](../model_conversion/README.md)) — native formats only; glTF
is Phase 7:
- The `CanonicalModel` IR
- Importers `fmdl_to_ir`, `model_to_ir`; exporters `ir_to_fmdl`, `ir_to_model`, calling the format
  crates' `ops/` for splitting, encoding and anti-blur rather than reimplementing them
- Hand auto-split in the IR, in-process without Blender: select weighted hand vertices, grow once,
  separate — see [Model conversion](../model_conversion/hand_split.md#hand-auto-split)
- The games' skeleton files embedded and exposed per version (`resources/skeletons/`), the render
  hierarchy and fold table beside them, and the retargeting pass
- Material conversion logic
- Cross-version player save data (`convertPlayerSaveData`) belongs to `pes_savefile` below

**`pes_savefile`** (spec: [Savefile](../pes_savefile/README.md)) — the codec and its operations, no tool
wiring:
- Crypto for all versions 15–21 (MT19937 stream cipher + PES15 LCG/MD5 chunk format, from the
  Midcupping scripts and the converters' `save*.py`; PES 18/20 master keys from pesXdecrypter
  sources; replaces `libpesXcrypter.dll`)
- The schema-driven codec: per-version field tables transcribed from 4ccEditor's
  `pes15.cpp`–`pes20.cpp`, one generic engine
- The `PlayerEntry`/`TeamEntry` model, including the tactics structures (presets, formations,
  advanced instructions); `EditFile` load/save
- `settings.toml` (`PlayerSettings`) parsing and savefile merging; FPC presets via `fpc::player`
- Cross-version player conversion (`convertPlayerSaveData` bitfield surgery + playstyle/skill maps)
- Interchange formats: Team TOML (read/write), legacy `.4ccs`/`.4cct` readers, Texport read/write
- Save-to-save operations: aesthetics fingerprinting (Midcupping), the comparator (merged
  gameplay + aesthetics diff), FPC invisibility; the aesthetics transplant built here was
  removed at step 8.9j5c (aesthetics live in the exports)

**`python_bindings`** — PyO3 shim over `fmdl` and `pes_model`, built via `maturin`; proves
guardrail 4 with a real `cdylib` build and a smoke test from Blender's Python. Its Phase 2 surface
is the four codecs' entry points, enough to load the wheel from Python and run every fixture
through a byte-identical round trip; the Blender-facing accessors (vertices, faces, the `ops/`)
arrive with the `pes-models` extension's hot-path step (`model_conversion/gltf.md` "Blender
integration"), shaped by what that code calls, not guessed ahead of it.

```python
import pes_models_native as native      # the wheel; the `pes-models` extension's native module
native.fmdl.Fmdl.read(data: bytes) -> Fmdl;   Fmdl.write() -> bytes      # fmdl::FmdlFile
native.fmdl.Skl.read(data) -> Skl;            Skl.write() -> bytes       # fmdl::SklFile
native.pes_model.Model.read(data) -> Model;   Model.write() -> bytes     # pes_model::format::PreFoxModel
native.pes_model.MaterialSet.read(data) -> MaterialSet;  MaterialSet.write() -> bytes  # pes_model::format::mtl::MaterialSet
native.FormatError                             # every codec error, message = the Rust Display text
```

`write` returns what the Rust `write` returns: FMDL and SKL bytes as stored; `.model` and `.mtl`
unwrapped (the WESYS wrapping is the archive's concern, `wezlib`'s in Rust). The smoke test's
oracle is each format crate's own invariant: byte identity where the crate proves it (Konami
FMDLs, every SKL), write-idempotence (`write(read(write(read(b)))) == write(read(b))`) everywhere
else, `FormatError` on junk input, and no warning-level log line on a clean read (the wheel
installs `pyo3-log`, so the libs' `log` output lands in Blender's console).

Build rules: the crate is a workspace member (it inherits the lints and the dependency table, and
`cargo clippy --workspace` type-checks it at every PR), but its `[lib]` is a `cdylib` with `test =
false`/`doctest = false` and `pyo3/extension-module` on by default, so `cargo test --workspace`
never tries to link it against a `libpython` (a Python extension links against the interpreter
that loads it, not at build time); its one test is the wheel. `abi3-py311`: one wheel per platform
loads in every Python from 3.11 up, which covers Blender 5.0 (Python 3.11) and 5.2 (3.13) and a
developer's own interpreter; `scripts/bindings_check.py` refuses a wheel not tagged `cp311-abi3`,
because CI builds under one Python (3.13) and a wheel that lost the feature would pass there and
fail to load in Blender 5.0. `just bindings` builds the wheel with `maturin` and runs the smoke
test (`crates/libs/python_bindings/tests/smoke.py`) through `scripts/bindings_check.py`: the wheel
is unzipped onto `sys.path` of a subprocess rather than `pip install`ed, because Blender's bundled
Python has no pip and the test must run under it (`--python <interpreter>`). The wheel version is
the crate's own (`distribution.md` "One exception"). CI runs `just bindings` on both platforms.

**Verification (done):** per crate, per [libs](../libs/README.md) "Testing" and each crate's own
plan: format round-trips on real fixtures (byte-identical on the layout the game's own files use);
`color_tools` on synthetic textures painted into its region constants and on the coloured kit
template (managers' `colors.txt` proved no usable ground truth, `spec/libs/README.md` "Testing: format parser unit tests");
`model_convert` IR round-trips, with the
19to16 converter's own output as a reference fixture; `pes_savefile` round-trips on decrypted
sections (fixed test RNG), per-version field round-trips, and appearance-block, fingerprint and compare
parity with the Midcupping scripts' byte rules held as literal offsets, and field values equal to
4ccEditor's own `.4ccs` export of one team per version (15–21) taken from the saves the payload
fixtures hold ([Savefile verification](../pes_savefile/verification.md)); `python_bindings` import
and round-trip from Python and inside Blender 5.0 and 5.2. The converge censuses ran every
`.model`, FMDL, SKL, FPK, FTEX and savefile on the maintainer's machine through the readers.
At the close of converge (2026-09-30): `just gates` green, 1057 tests in 41 test runs, none
failed or ignored; `just bindings` green.

### Phase 3: Team compiler skeleton (`tools/team_compiler`)

The Team compiler checks and compiles the tracer bullet's content end to end, from the command
line and from a minimal window. What `compile` accepted at the phase's close is the Team
compiler plan's "Phase 3 scope" (in its "Acceptance"; Phase 4 withdrew the gate that refused
the rest with `content_not_yet_compiled`); the behavior contract is that section's TC
scenarios, every automated one cited by a test. Its
entry gates (normal-team/referee `ExportIdentity`, the sanitized validated-versus-eligible
projection, roster-entry scope and disposition semantics) are in the [Aesthetics
export](../aesthetics_export/object_model.md) plan's "Validation semantics".

- **Tracer bullet first.** Before the skeleton, the smallest real export was compiled end to
  end: /egg/'s one player folder (a Fox face) and one kit, read from disk and written as a CPK
  through the Phase 2 crates (`fmdl`, `ftex`, `fpk`, `cpk`, `kit_config`). Its test,
  `team_compiler/tests/parity.rs`, compares the result against Red's output for the same source
  export by the parity tiers that apply to it (tier 1 for textures, tier 2 for the FMDL through
  `fmdl`'s decoded model, tier 3 for the containers; "Testing: parity against Red" in the Team
  compiler plan), and now runs through `compile`; Phase 4 grew it. The fixture
  (`tests/fixtures/tracer/`) is a small old-layout export migrated to the Studio layout by hand,
  since the Export upgrader (Phase 6) does not exist yet; that same pair later becomes the
  upgrader's own input/expected fixture. The tracer's scaffolding (a hardcoded walk of the one
  folder) was replaced by `aesthetics_export` and the pipeline. We did this first, not as Phase
  4's verification, because the Phase 2 crates' `pub` APIs had had no consumer: a shape wrong
  across several crates (bulk-data ownership, finding codes, how a face's FPK is assembled)
  found there is a one-crate fix, found after the skeleton is built on it is shotgun surgery
  through the tool.
- The tool's settings struct and its two CLI subcommands, `check` and `compile`, through the
  `StudioTool` trait, with the exit codes 0/1/2/3 (`spec/team_compiler/settings.md` "CLI").
- `libs/aesthetics_export`: the source-neutral object model for the Studio export format
  (canonical `vtree::ScopePath` listing → `ParsedAestheticsExport` → sanitized
  `ValidatedAestheticsExport` → `ResolvedAestheticsExport`, with `ExportIdentity`, strong
  `ValidatedRoster`, `PlayerFolder`, `SharedModelFolder`, and `KitsFolder`) plus its folder
  conventions and the structure pass's validation rules, shared with the Export upgrader, Kit
  config editor, and Refs arranger. Tool-local `BuildManifest`, `PlanReport`, `plan_run`, and
  `process_task` stay in `team_compiler`.
- The pipeline orchestration (reader, coordinator, writer) on `rayon`, over `libs/pipeline`'s
  memory budget, thread count and `CpkStem`; the run driver's shapes are the Team compiler
  plan's "Run driver shapes" (`pipeline.md`).
- CLI execution with console output (`UTW team-compiler compile ...`), one line per
  finding (`spec/core/architecture.md` "Event system").
- **Shell slice, last.** The thinnest GUI path that runs: `studio_core`'s app shell reduced to a
  window, the sidebar listing the registered tools, and the selected tool's `view()`; the
  `studio` binary launching it with `team_compiler` registered; and the Team compiler's `view/`
  reduced to its settings section, a run button and a plain log of the `PipelineEvent`s the
  Phase 3 pipeline emits (no progress grid, no live validation). Everything else in the
  "Phase 8" list (common widgets, help window, status bar, watcher, cancellation, the other
  tools' views) stays in Phase 8, which completes this shell rather than starting it. We built
  it here, not in Phase 8, because the `StudioTool` trait, the event channels and the
  render-only `view/` rule would otherwise be designed against zero real tools until five tool
  crates had been written to them; one real tool on the shell early is what shows whether the
  seam holds, and it is what Phase 10 needs to run in parallel ("once the shell exists"). The
  settings section shows the effective settings read-only (editing them is the settings menu's,
  which needs the shell to save the file); the run button runs `compile` with no arguments,
  through the same parser and preflight as the CLI, on a thread of its own, and the log ends
  with the run's exit code (Phase 3 emits no `Complete`); the binary is a console-subsystem exe,
  so a console window opens beside the GUI, and `--gui` autorun is refused: both need what Phase
  8 builds (`AttachConsole`, the initial exports check).

**Verification (done):** the tracer bullet's parity case on its one-face fixture; the
`aesthetics_export` structure pass and `team_compiler`'s `check` and `compile` against the TC
scenarios, run in-process over Studio-format fixtures (folders, a `.zip` and a solid `.7z`),
with the binary's own seams (settings location, exit codes, the console printer) run through
the built `UTW`; `scripts/acceptance.py` proving every automated scenario cited by a
test; the shell slice's two manual scenarios (TC-GUI-01/02; the second automated at 8.0) checked by the maintainer on Windows
(Linux not yet, no machine with a display). Converge ran a whole-crate mutation run over each of
the phase's crates, every survivor triaged. At the close (2026-10-02): `just gates` green, 1402
tests in 50 test runs, none failed or ignored; acceptance 77 scenarios, 77 proven. Full
packed-output comparison was left to Phase 4.

### Phase 4: Processing logic

The Team compiler compiles every Studio-format export, normal teams and referees, into
game-facing output for every target version, PES 15 to 21: the CPK (or several, in multi-CPK
mode), the test or sideload tree, and the bins it updates, deployed into the PES install.
Cross-format model conversion goes through Phase 2's `model_convert`; savefile writing is Phase
5's, and a glTF model source is refused until Phase 7. The work is organized by the Team
compiler's stages, each specified in "Crate layout" and the "Per-export pipeline walkthrough" of
the [Team compiler plan](../team_compiler/pipeline.md); what `compile` builds and refuses is that
plan's "Phase 4 scope" (in its "Acceptance"), whose TC scenarios are the behavior contract,
every automated one cited by a test. Red module names in parentheses locate reference
*behavior*; they prescribe nothing about the Rust code.

**Planning before processing.** Planning enumerates every output and allocates its namespace,
folder-internal deep-derived names included, without a run phase or type of its own, and fixes
every model `BuildTask`'s planned model IDs before any task runs; these were the phase's entry
gates.

- **Deep validation** — the compiler's `deep/` pass: content-level rules on top of
  `aesthetics_export`'s structural ones (that crate reads no file content: the Aesthetics export
  plan's "Content findings"), and the `fmdl`/`pes_model` `check.rs` findings, all mapped through
  the Team compiler's message catalog (`export_check`, `xml_check`, `texture_check`). `check`
  and `compile` run the same pass. Live-grid scheduling in `check/` waits for Phase 8.
- **`plan/`** — `BuildManifest` from a `ResolvedAestheticsExport`, `PlannedModelIds` (40-ID team
  blocks, deterministic boots/gloves assignment — assignment only; the savefile write is Phase 5),
  duplicate-refs preflight, `overrides/` precedence (`team_id_get`).
- **`processing/`** — per-task work over `rayon` with memory permits: model format selection,
  cross-format conversion via `model_convert`, multi-model merging and SKL pairing
  (`fmdl_editing`); hand auto-split (`model_convert::ops::hand_split`) on every model carrying
  `skh_*_l`/`skh_*_r` weights before categorization, the split parts becoming the player's
  `glove_l`/`glove_r` parts (`spec/aesthetics_export/player_folders.md` "At compile time, the pipeline" step 0); model
  source selection before glTF exists: a folder whose selected representation would be glTF (no
  target-native source beside it) is refused with `model_gltf_unsupported` (folder-discarding,
  not pass-through-eligible) until Phase 7, never compiled from the opposite native format
  instead, since that output would change silently once glTF lands (`team_compiler/pipeline.md`
  "Per-export pipeline walkthrough"); texture conversion with the CPU converter, each worker
  converting one texture at a time, so the conversions in flight are bounded by the worker count
  (0.1.0 ships CPU-only; the desktop GPU BC7 backend, `libs/README.md` "First-release desktop GPU
  BC7", is built on this step in Phase 16; stopping a cancelled run between textures is Phase 8's
  cancellation, which brings the Cancel that triggers it) — plus WESYS wrapping and relocation
  to common (`textures`);
  MTL / face XML / `materials.toml` editing (`xml_editing`); kit folders: config
  generation/reconciliation/emission via `libs/kit_config` (TOML in exports, binary at compile
  time), FPC patching via `fpc`, kit colors from per-kit `colors.txt` (Team Note entry format) or
  derived from the kit texture via `libs/color_tools` when absent; portraits, logos, collars and
  Common (`portraits_move`, `name_editing`); referee processing per the Team compiler plan's
  "Referee export processing" with Red's observable output as the reference (`referee_tools`,
  referee code in `fmdl_editing`/`export_move`); `materialize.rs` as the single output-mode seam:
  relocation to game paths and Fox FPK packing (`contents_packing`, `export_move`).
- **`bins/`** — UniColor / TeamColor / UniformParameter accumulation via `uniparam`, DpFileList
  read/write and slot discovery (`bins_update`); on Fox the player appearance tables, read from
  the installed CPKs by the same walk and written whole: `BootsList.bin` and `GloveList.bin` get
  a row for every compiled player whose custom boots or gloves output committed (the planned ID),
  a failed output keeping the installed row, a compiled player the `Full` export gives no boots
  or no gloves losing that installed row, every other row kept; `PlayerAppearance.bin` passes
  through with no row changed. Phase 5 adds the stock and default-ID rows and the appearance rows
  from `settings.toml`: this split is the only one in which Phase 4's CPK is game-correct for what
  it compiled without reading `settings.toml`.
- **`output/`** — Phase 3's canonical-order writer, staging and promotion to `output/`, with
  the output folder's writability preflight (TC-OUT-*, TC-CLI-06), behind `OutputSink` (CPK, or
  a loose folder for test and sideload), the `.partial` copy into `download/` for deployment,
  and multi-CPK; access-denied output paths fail cleanly via `libs/elevation` (the GUI's
  elevated-relaunch prompt is Phase 8). `output/savefile.rs` is Phase 5.
- `dummy_kit_replace` is not ported — `dummy_kit*` stems are reserved game-substituted names, see
  the Unified model format plan's "Kit-dependent assets".

**Verification (done):** parity with Red on the tracer export (Fox) and the pre-Fox reference
(`team_compiler/testing.md`); `check` and `compile` against the TC scenarios, run in-process over
Studio-format fixtures, with the bins and deployment paths over synthetic installs; a census of
every export on the maintainer's machine; in-game checks through `scripts/ingame/` on PES 17, 19,
20 and 21 (the worklog's Phase 4 row). Converge ran a whole-crate mutation run and a Clef scan
over `aesthetics_export`, `pipeline` and `team_compiler`, every survivor and flag ruled. At the
close (2026-10-12): `just gates` green, 2820 tests in 52 test runs, none failed or ignored;
acceptance 421 scenarios, 366 proven (the 55 unproven are the save editor's and the Team
compiler's Phase 5 scenarios, written ahead). Deferred: the parity gate over upgraded old
exports, compiled and compared with Red's output (nested CPK contents, bins; auto-assigned IDs
aside), which needs Phase 5's savefile and ID integration and Phase 6's Export upgrader.

### Phase 5: Savefile integration (`tools/save_editor` logic, `libs/aatf`, Team compiler savefile writing)

`pes_savefile` exists since Phase 2; this phase wires it into its two consumers.

- Savefile writing in the Team compiler (`output/savefile.rs`), Fox first. The resolved settings
  become, on Fox, each compiled player's appearance table rows (Phase 4's `bins/`, whose boots
  and gloves rows are set only for committed outputs), the default-ID rows (`plan/item_rows.rs`,
  where a folder's `fpc_on`/`fpc_off` marker picks the hide preset's ID or 0), and an aesthetics
  patch of motions and a `write_names` export's names, which depend on no asset; the patch is
  written beside the output and applied to the local save. The pre-Fox patch ("Player settings
  in exports" in the Aesthetics export plan), the one whose boots and gloves IDs need
  commit-conditioned mutations, is deferred with pre-Fox's other gaps until the cup next plays a
  pre-Fox version (worklog issue). The allocation scheme is fixed (40-ID per-team blocks — see
  the Team compiler plan).
- `libs/aatf`: the ruleset schema and its load-time validation, the generic Rhai interpreter, the
  host and `apply_tier`, logic identity, and the Autumn 26, Autumn 25, Spring 26, VGL25, VGL26,
  VGL27, VTL10 and VTL11 rulesets as data with their fixtures ([AATF rules](../aatf_rules.md),
  "Development phase and verification").
- The Save editor tool crate's non-GUI substance that a command drives: settings, the CLI, and
  the `session/` operations its commands run over `pes_savefile` (Team TOML export and import,
  the aesthetics patch, AATF checks, comparator, the teams list export (strip and seed with
  its restored-id check went at step 8.36b); [Save editor](../save_editor.md), "CLI"). The
  wiring only the view drives (editing with its undo stack, selection, recent files, tactics,
  the FPC toggle, AATF quick actions) is Phase 8's, each piece in the slice of the view that
  uses it: built earlier, it would be a layer with no consumer, its shape guessed before the
  view that decides it.

**Verification (done):** the Team compiler's savefile stage against a whole fixture save
(`tests/fixture_save.rs`, through `pes_savefile`'s `test-support` feature); the AATF rulesets'
fixtures (known-legal and known-violating teams, `apply_tier` then check); the Save editor's CLI
on fixture saves, every SE scenario cited by a test. In game on PES 21 (worklog 5.5): a stripped
and seeded save shows the sampled players as before; a compiled team's rows show; an
`ingame_face` player whose file sets none of the face and hair keys renders the default face
(without the marker a folder with no face model gets the blank face, no head;
`aesthetics_export/settings_toml.md`, the ingame-face run's rules). The gloves half waits on a
FoxDen change that keeps the table's value (worklog issue). Converge ran a whole-crate mutation
run and a Clef scan over `pes_savefile`, `aatf`, `save_editor`, `studio_core`,
`aesthetics_export` and `team_compiler`, every survivor and flag ruled, and a census of every
save, `.ted` and editor file and every player table on the maintainer's machine. The second
whole-crate runs, at the close (extra kinds included): `pes_savefile` 1830 caught, 2 missed
(tested since); `aatf` 242 caught, 0 missed; `save_editor` 128 caught, 3 missed (two
placeholder views, tested with Phase 8's views, and an error arm only Linux reaches);
`studio_core` 76 caught, 5 missed (process-probe arms the half's OS compiles out);
`aesthetics_export` 603 caught, 0 missed; `team_compiler` 2474 caught, 1 missed (a placeholder
view). The citation audit (`CONTRIBUTING.md` "Citation audit") ruled every test it found not
proving its scenario. At the close (2026-10-17): `just gates` green, 3513 tests in 63 test
runs, none failed or ignored; acceptance 442 scenarios (4 manual, 9 withdrawn), all proven.

### Phase 6: Export upgrader (`tools/export_upgrader`)

The one-time migration of AET exports to the Studio format ([Export upgrader](../export_upgrader.md)),
the only place old-format knowledge lives, through the CLI: `upgrade` (steps 1 to 12) and
`upgrade-models` (model-folder mode). The glTF pass (step 13) is Phase 7's, the view Phase 8's.

- Old-format export loading (a folder, `.zip` or `.7z`), the team resolved from the Note or the
  name, the savefile-driven ID→player mapping (or loose mode), player folders merged from
  single-wearer model folders and shared folders with link files for the rest, kits
  restructured (binary configs → `config.toml`, `colors.txt` and the `icon_<N>` marker from the
  Team Note), portraits, logo, Common and the pre-Fox model renames, `settings.toml` generated
  from the save's records, referee exports flattened.
- The clean-output rule: an upgrade starts from a clean Red export and writes a clean Studio
  export or nothing, staged in `<output>.upgrading` and published by the verdict; content that
  never reached the game is a leftover, dropped with a Warning. The quirks old files carry (an
  `.mtl`'s stray text) are cleaned by the upgrader's own tolerant reads, the compiler staying
  strict.
- On the Team compiler's side, what the upgraded library showed it needed: a boots or gloves
  folder one player wears takes his exclusive ID, the shared IDs only for folders two or more
  wear; a Midcup kit keeps the installed config, textures and colors it does not resend; the
  `.mtl` search reads `oral_X_win32` as `X`.

**Verification (done):** every EU scenario but the GUI's cited by a test; the three tracers
(Fox, pre-Fox, referee) upgraded into committed goldens and compiled against Red's committed
output (`team_compiler/testing.md`); the upgrade census of every old export on the maintainer's
machine (639 inputs, 279 clean, every unclean run writing nothing), and the library parity
against released Red over the clean ones, every difference attributed
(`scripts/library_parity/`). In game: a hand-split face's gloves on PES 21 (worklog 6.16c).
The closing whole-crate mutation runs (2026-10-20, extra kinds included, split with the VPS):
`export_upgrader` 981 caught, 6 missed, 1 timeout, 102 unviable; `team_compiler` 2532 caught, 3
missed, 3 timeouts, 405 unviable; every survivor a test added, an equivalent pinned, or the
placeholder views Phase 8 replaces. At the close (2026-10-20): `just gates` green,
3987 tests passed in 67 test runs, 6 ignored (the GPU tests); acceptance 652 scenarios (10
manual, 9 withdrawn), 564 proven, the 88 unproven Phase 8's GUI scenarios.
