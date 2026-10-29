# Unlimited Tool Works — Core plan

This is the core architectural plan for Unlimited Tool Works: the goals, the platform (crate
structure, tool plugin system, GUI shell, parallelism), and the cross-cutting
decisions. The tools and the major library crates have their own documents, listed in the
[index](../README.md).

---

The plan is split into parts; section headings are unchanged from the
single-file plan.

| Part | Covers |
|---|---|
| [Architecture](architecture.md) | Architecture |
| [GUI design](gui.md) | GUI Design |
| [Parallelism](parallelism.md) | Parallelism |
| [Distribution and updates](distribution.md) | Distribution and updates |
| [Development plan](development_plan.md) | Development Plan |

## Overview

Unlimited Tool Works is a Rust-based **suite of all the tools used by the 4cc community**, built around two
tightly integrated main tools, the first of which exists:

1. **The Team compiler** (successor to the AET Compiler) — compiles aesthetics exports into CPK archives,
   with compile-time model format conversion and compile-time aesthetics (on Fox the output CPK's
   database tables, on pre-Fox savefile writing)

Model format conversion (pre-Fox ↔ Fox) is **not** offered as a standalone tool — it is integrated
into the Team compiler, which converts models to the target PES version's format at compile time.

The plan documents form a standalone architectural plan derived from a feasibility analysis of the
existing codebases.

**Relationship to legacy tools:** Unlimited Tool Works is an independent reimplementation of their useful
ideas and required observable results, not a translation of their source. Its implementation,
internal data layout, module organization, and UI are designed anew around this plan. Legacy file
and function names identify behavioral/format evidence, not code or application structure to copy.
Required PES binary formats and game-facing output conventions remain compatible; a new internal
layout does not mean changing formats the game must read. Normal third-party library reuse remains
appropriate, with its own dependency/license checks.

### Project context

The 4cc AET Compiler is a modding tool for the Pro Evolution Soccer (PES) game series (PES 15–21).
It takes user-submitted "exports" (folders, `.zip`, or `.7z` archives containing team assets —
faces, kits, boots, gloves, portraits, logos) and compiles them into CPK archive files that the game
can load.

**PES 2020** (a recurring question): no official 4cc cup has used it. PES
2021 is in theory the same game with a roster update, which does not matter to the 4cc, so 2020
was skipped. It was used once, for a recent invitational, partly as a test, and Red gained a small
addition to support it. Studio supports it like any version; where its game files have not been
checked, it is taken to match 2021.

Three prior compiler implementations exist:

| Version | Language | Status | Location |
|---------|----------|--------|----------|
| Original | Batch file | Obsolete, rudimentary | — |
| Red | Python | Mature, feature-complete; direct batch translation, works by moving files on storage | `4cc-aet-compiler-red` |
| Blue | Python (free-threaded 3.14t) | LLM-made prototype of an in-memory parallel pipeline; missing referee support | `4cc-aet-compiler-blue` |

All three are stopgaps towards Unlimited Tool Works: they define *what* the compiler must produce (Red's CPK
output is the golden standard for parity), not *how* the new implementation works (see "Relationship
to the older compilers" in the [Team compiler plan](../team_compiler/README.md)).

The save editor exists as a Win32 C++ application, accompanied by a set of standalone savefile
scripts:

| Tool | Language | Status | Location |
|------|----------|--------|----------|
| 4ccEditor | C++ (raw Win32) | Mature; PES 2015–2021 | `Tools_4cc/4ccEditor-1` |
| Midcupping | Python | Aesthetics transplant + aesthetics diff scripts (PES 15/16/17); reference container-crypto implementations | `Tools_4cc/Midcupping` |

Some standalone format converters also exist:

| Converter | Direction | Location |
|-----------|-----------|----------|
| 4cc-aet-converter-19to16 | FMDL (PES 19) → .model (PES 16) | `Tools_4cc/4cc-aet-converter-19to16` |
| aes_converter_16to21 | .model (PES 16) → FMDL (PES 21) | `Tools_4cc/aes_converter_16to21` |
| 4cc-model-simplifier-15 | .model (PES 16/17) → .model (PES 15): folds weights of bones PES15 lacks or poses differently (absorbed by the Team compiler's per-version skeleton retargeting) | `Tools_4cc/4cc-model-simplifier-15` |

Two Blender addons provide import/export for the community:

| Addon | Format | Location |
|-------|--------|----------|
| pes-fmdl | FMDL | `Blender/2.79/scripts/addons/pes-fmdl` |
| pes-model | .model | `Blender/2.79/scripts/addons/pes-model` |

Unlimited Tool Works consolidates all projects into a single Rust application with a GUI.

### Language choice: Rust

Rust was chosen over C# and Python after evaluating:

- **Binary format parsing** (the bulk of the codebase): Rust's `binrw` derive macros provide typed,
  declarative readers and writers. The compiler checks their types, not whether offsets, widths,
  or endianness match PES files; fixtures and independent field checks establish that.
- **Performance**: The mesh splitting algorithm in the converters is O(V×F) graph partitioning with
  set operations in the inner loop. Python takes 1–5 seconds per complex model; Rust estimates
  10–50ms. This is essential for compile-time format conversion.
- **Single binary distribution**: eliminates the embedded Python runtime and separate conversion
  executables. Final binary size and platform system-library requirements are measured at release;
  optional Blender viewing still requires Blender and its extension.
- **LLM-assisted development**: The compiler strictness + LLM iteration loop is maximally productive
  for binary format parsing, which is the project's core workload. The object model friction (parent
  references, ownership) is a one-time cost that LLM assistance reduces.
- **True parallelism**: No GIL, no free-threaded Python workarounds. `rayon` work-stealing replaces
  `ThreadPoolExecutor` + `console_lock`.
- **Linux support**: Native.

### GUI framework choice: egui

egui was chosen over Tauri after evaluation. egui is a pure-Rust immediate-mode GUI that renders
directly to a GPU surface via `wgpu` (pinned over the alternative `glow`/OpenGL backend). No
webview, no HTML/CSS/JS.

Key reasons:
- **Proportionate to the UI scope**: the application is a utility tool (grid of colored squares, log
  viewer, settings forms, sidebar). A browser engine for that is overkill.
- **Zero external dependencies**: no WebView2 runtime on Windows, no WebKitGTK on Linux. One binary,
  works everywhere, instant startup.
- **Single language**: the entire codebase is Rust. No second technology stack (web frontend) to
  maintain alongside Rust.
- **Resource goal**: avoid a separate webview process alongside PES. Measure actual idle and
  processing memory on the intended machines; egui/wgpu does not guarantee a particular RAM budget.
- **Cross-platform consistency**: identical GPU rendering on all platforms. No webview rendering
  differences between Windows and Linux.
- **Native HiDPI support**: `egui-winit` applies the OS scale factor automatically (live updates
  when dragged between monitors of different pixel densities; `devicePixelRatio` on the web). The
  zoom factor on top of it is the shell's alone, set from the window's width (`gui.md` "Shell
  layout", "Scaling with the width"), with egui's keyboard zoom off. No custom DPI code is
  needed.
- **Browser deployment path**: the same Rust codebase compiles to a browser-served WASM build,
  Studio Web (see `gui.md` "Browser deployment: Studio Web"). Tauri cannot serve to browsers — its web
  frontend is coupled to the Tauri runtime.

---

## External Dependencies (Rust crates)

| Crate | Purpose | Maturity |
|-------|---------|----------|
| `binrw` | Binary format parsing with derive macros | Production-ready |
| `rayon` | Parallel iteration (work-stealing thread pool) | Production-ready |
| `crossbeam-channel` | Tool↔shell event and request channels; the write queue between coordinator and writer | Production-ready |
| `flate2` | zlib compression (WESYS format, FTEX's compressed chunks) | Production-ready |
| `sevenz-rust2` | Native .7z reading (replaces 7z.exe); the maintained fork of `sevenz-rust`, which RUSTSEC-2026-0246 marks unmaintained | 0.22.2 in use, no default features (decode only: LZMA, LZMA2, BCJ) |
| `zip` | Native .zip reading | 8.6.0 in use, no default features plus `deflate` |
| `serde` | Serialization: `PesVersion`'s two-digit number form, app settings and `aatf`'s ruleset data today; `serde_json` writes `save-editor diff --json`'s report today and joins with the Blender manifest (`player_aesthetics_editor.md`) and the match-event wire format (`match_feed.md`) in their phases | Production-ready |
| `clap` | CLI argument parsing | Production-ready |
| `thiserror` | Ergonomic error enums for the lib crates' error types | Production-ready |
| `anyhow` | Error propagation in tool crates and the binary (`StudioTool::cli_run`'s `CliError` wraps an `anyhow::Error`) | Production-ready |
| `log` | Diagnostic logging facade — the one logging front-end a lib or tool crate uses (see `architecture.md` "Diagnostic logging") | Production-ready |
| `env_logger` | CLI-mode log sink in `studio` (`default-features = false`) | Production-ready |
| `pyo3` | The Python extension module of `python_bindings` (`abi3-py311`, `extension-module`); built into a wheel by `maturin`, a developer tool installed with pip like `just` is with cargo | Production-ready |
| `pyo3-log` | Forwards `log` lines to Python's `logging` in `python_bindings` | Production-ready |
| `eframe` (egui) | GUI framework (pure Rust, immediate-mode, native + WASM) | Production-ready |
| `md-5` | FPK checksums, PES15 save integrity hashes | Production-ready |
| `unicode-normalization` | NFC normalization for `vtree`'s collision detection (two spellings of `é` are one file name on disk) | Production-ready (0.1.25 in use) |
| `indexmap` | `music_export`'s slot map, the `.4ccm` slots in file order (the plan's `Model` names it); already in the build through `toml_edit` | Production-ready (2.14.2 in use) |
| `roxmltree` | Read-only XML tree for `.mtl` material sets, `face.xml`, `face_diff.xml` (`pes_model`, the Team compiler) and `fox2`'s XML form; writing those small fixed shapes is done by hand so Konami's formatting is reproducible | Production-ready (0.21.1 in use) |
| `base64` | Decoding the vertex blocks of a Fox `face_diff.xml` (the Team compiler); chosen over a hand-written decoder because it leaves no bit-twiddling to own and mutation-test | Production-ready |
| `directories` | User config dir resolution (data-location choice) and `pes_savefile`'s save discovery under the user's Documents folder | Production-ready |
| `toml` | Export text formats (`settings.toml`, `config.toml`; `materials.toml` joins with glTF), app settings | Production-ready |
| `toml_edit` | Comment/formatting-preserving edits to the auto-generated tomls (`settings.toml`, `config.toml`; `materials.toml` joins with glTF); comments are app-injected (predefined per-field documentation) and preserved across edits | Production-ready (the `toml` crate's own foundation) |
| `image` | Raster source decoding (PNG, JPEG, BMP, WebP, TGA, TIFF) for texture conversion — all formats interchangeable as sources for any model format | Production-ready (pure-Rust default formats) |
| `tiff` | TIFF tag access `image` does not reach (the premultiplied-alpha metadata for `dds_convert`) | 0.11.3 in use |
| `block_compression` | CPU BC1/BC3/BC7 encoding and BC1–BC7 decoding (one crate for both directions); first-release desktop GPU BC7 | 0.10.0 in use (features `bc15`, `bc7`; `wgpu` feature native-only, for `dds_convert`'s GPU encoder); decode verified exact against texconv; GPU cold start, throughput and fallback measured on one AMD GPU (`plans/libs/README.md` "First-release desktop GPU BC7") |
| `wgpu` | The compute device of `dds_convert`'s GPU BC7 encoder (Vulkan, Metal on macOS); already in the build through eframe's `egui-wgpu` | 30.0.1 in use, the version `egui-wgpu` resolves, so the build holds one; native targets only |
| (none: `fox2` ports CityHash64 1.0.3 directly) | fox2 string hashes must match the C# tool's embedded 1.0.3 variant exactly; no crate pins that variant, and the port is 150 lines verified by a reference golden and every fixture's string table | Not a dependency |
| `windows` | Win32 APIs: `elevation` (token query, `ShellExecuteExW`) and `pipeline` (the available-memory read); later the match tracker's memory reading | 0.62.2 in use, `Win32_Foundation`, `Win32_Security`, `Win32_System_Registry` (a `SHELLEXECUTEINFOW` field), `Win32_System_SystemInformation` (`pipeline`'s `GlobalMemoryStatusEx`), `Win32_System_Threading`, `Win32_UI_Shell`, `Win32_UI_WindowsAndMessaging`; Windows targets only |
| `libc` | `geteuid` for `elevation` on POSIX | 0.2.189 in use; Unix targets only |
| `sha2` | SHA-256 hashing: `dds_convert`'s conversion-cache keys, `pes_savefile`'s write salt and aesthetics fingerprint; the updater's download verification joins it (`plans/core/README.md`) | Production-ready |
| `rhai` | `aatf`'s ruleset files: the `RULESET` data block and the engine every script runs on (sandboxed; [AATF rules](../aatf_rules.md)) | 1.26.1 in use, no default features (`std`, `serde`; its RNG does not build for `wasm32`), plus `only_i64`: every value the host registers is an `i64`, so the operators for the other integer types are code no script reaches (627 KiB of the release exe) |
| `syn` (3.x; `full`, `visit`, `parsing`, `printing`, no default features) + `proc-macro2` (`span-locations`) | The dev tool `crates/dev/extra_mutants` alone (step 5.0b), linked into no release binary: it parses a crate's sources to list the mutants `cargo-mutants` lacks, and `span-locations` gives each node's position in the file for the text patch | Both already in the build with these features (`binrw_derive`, `serde_derive`), so nothing new compiles |

No frontend dependencies — the entire GUI is Rust, compiled to both native and WASM from the same
codebase.

The Rust code will be more verbose per line than the Python/C++ it replaces across the source
projects, but dramatically less duplicated — especially with the IR eliminating the N×N converter
pattern and the schema-driven save codec collapsing 4ccEditor's six per-version files.

---

## Key Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language | Rust | Binary parsing ergonomics, performance for mesh splitting, single binary, LLM + compiler strictness loop |
| GUI framework | egui (`eframe`) | Pure Rust, zero dependencies, compiles to native + WASM; proportionate to a utility tool's UI scope |
| Object model | Typed structs with enums | Compile-time exhaustiveness, no string-extension dispatch |
| Parent references | Pass resolved identity/context, not parent references | Avoid ownership coupling; validated identity is not an untyped team-ID string |
| Model conversion | All cross-format through IR | One decode/encode per format, not N×N; lossless roundtrip tests |
| Same-format paths | Skip IR unless an explicitly planned IR operation is required | Hand auto-split and pre-Fox `ingame_face` merging are exceptions; ordinary native processing stays IR-free |
| Material schema | Shader `family` + engine-neutral keys (booleans, canonical texture roles, parameters) + optional `[fox]`/`[prefox]` native tables | `[face]` alone is a valid material (family `shaded`, textures auto-detected by name); families are named as laymen know them (`shaded`/`shadeless`, not `blin`/`constant`) and encode the converters' shader mapping tables so both engines get a working material from one word; native tables keep same-format round-trips lossless and expose every knob; unknown keys warn (forward-compatible), invalid values error |
| Blender integration | Two complementary paths, packaged as one all-in-one extension (`pes-models`) | Path A: vendored `pes-fmdl`/`pes-model` 5.2 port codecs, hot paths accelerated by the `python_bindings` PyO3 wheel (lossless same-format round-trips, pure-Python fallback). Path B: PES glTF codec (native glTF I/O + `materials.toml` read/write + `PES_bone`/`PES_mesh` extension handling) for new authoring targeting both engines. Plus the Player aesthetics editor's loader module; replaces the standalone addons at release (plan lives with the Blender project) |
| Mesh splitting | CPU only (Rust) | GPU unsuitable for branch-heavy graph partitioning; Rust is fast enough |
| Texture conversion | In-process (`image` + `block_compression`, which decodes as well as encodes, CPU/GPU backends + bounded cache) | BC7 supported only on PES 19–21, with first-release desktop GPU acceleration in Auto mode and CPU fallback/explicit CPU mode. BC7/raster sources for 15–18 use CPU BC3 or eligible opaque-color BC1. Codec, alpha, cache, and fallback policy live in [libs](../libs/README.md) |
| Texture references | Stem-based (input); `.dds` extensions on output `.mtl` | All image formats (DDS, FTEX, PNG, JPEG, BMP, WebP, TGA, TIFF) are interchangeable as input sources for any model format; FMDL path tables were already stem-based; Studio-format `.mtl` and `.materials.toml` write stems; output `.mtl` (game format) keeps `.dds` extensions; two image files with the same stem but different extensions is `texture_stem_conflict` |
| Model conversion exposure | Shared `libs/model_convert`, used by compilers and glTF migration actions | No separate conversion-only tool; consumers include Team/Balls compilers, Export upgrader, and Player aesthetics editor |
| Old export format | Not supported by the compiler | Migrated once via the Export upgrader tool; keeps the compiler simple |
| Shared model folders | Name-based, no embedded IDs | Player folders are the reference point; link files (`Name.boots`); IDs auto-assigned at compile time |
| Kits format | `Kits/` folder, one subfolder per kit (`<slot>[ - <label>]`) plus an optional `all/` of textures every kit inherits per stem; an empty kit folder is a placeholder kit (bundled magenta/black checkerboard, template config) for full-body-model teams; an optional `pre-fox` / `fox` marker file declares which engine's kit layout the main texture is drawn for, and the compiler re-lays out the sock islands when the target is the other engine | `config.toml` + optional `colors.txt` (Team Note color format; derived from the kit texture when absent, else the loud magenta/black pair) + optional `icon_<N>` menu icon marker + textures per kit; drives per-kit grid cells |
| 7z extraction | Native `sevenz-rust2` crate | Eliminates subprocess and temp-folder dance |
| Compression | Archive-level only | Exports already submitted as .zip/.7z; no format-level compression needed |
| Testing | Upgrader+compile parity + IR roundtrips; CPU reproducibility + GPU quality checks | CPU mode is the byte-reproducible reference. GPU texture bytes and containing CPKs may differ across devices/drivers; decoded quality and all non-texture identities/ordering remain constrained |
| Suite scope | Team compiler + save editor first | Shared `libs/pes_savefile` crate; other tools build on the same libs later |
| Crate architecture | Minimal core + tool crates + lib crates | `studio_core` = shell/trait/settings only; every tool its own crate; all shared functionality in lib crates |
| Tool crate granularity | One crate per tool, no companion libs | Even the Team compiler (15–25k lines) stays one crate with internal modules; only genuinely shared parts move down — hence `libs/aesthetics_export` (compiler + upgrader + kit config editor + refs arranger + future tools), not a single-consumer `team_compiler_lib` |
| Tool modularity | Tools own their GUI, settings, CLI, and help | `StudioTool` trait: view, settings section (injected into settings menu), clap subcommand, help chapter (injected into the help window, Messages topic generated from the catalog) |
| CLI shape | `UTW <tool-id> <command>` subcommands | Standard clap subcommands; better help/completion than flag-style dispatch |
| License | `MIT OR Apache-2.0`; game-derived data excluded and named as Konami's | The suite is libraries first, for a community that writes tools in every language; copyleft would confine the libs to GPL consumers for a protection this community has never needed. See `distribution.md` "License" |
| Data location | First-run choice: `data/` next to the exe (fully portable) or the user config dir; presence-based resolution, no marker files | EGG-Translator's wizard mechanism; portable users get delete/move-the-folder semantics, config-dir users survive program-folder replacement |
| Save codec | Schema tables + one engine | Collapses 4ccEditor's six per-version files; mechanically verifiable |
| Save crypto | Native Rust (MT19937 stream cipher; PES15 LCG + MD5) | Not AES (earlier analysis was wrong); replaces libpesXcrypter.dll; proven portable via the Midcupping scripts and save*.py |
| Team interchange | New full-fidelity Team TOML; `.4ccs`/`.4cct` read-only; Texport read+write | Text format editable without the Studio; player settings tables shared with settings.toml; Texport is PES's own format so both directions matter |
| Player folders | The primary export unit | Face+boots+gloves in one folder; primary motivation for the project |
| Auto-generated toml comments | App-injected (predefined per-field documentation), preserved by `toml_edit` | Users never write comments from scratch; the comments are the simplified documentation, present in every auto-generated `settings.toml`/`config.toml`/`materials.toml` |
| Kit configs | `config.toml` in exports, game binary emitted at compile time; `libs/kit_config` shared by compiler/upgrader/editor | Format recovered by disassembling the abandoned closed-source Kit Manager (documented in the Kit config editor plan); text in exports matches the settings.toml philosophy |
