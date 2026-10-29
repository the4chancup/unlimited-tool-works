# Unlimited Tool Works — Core plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Overview

What exists of this is in `spec/core/README.md` "Overview".

2. **The save editor** (successor to 4ccEditor, absorbing the Midcupping scripts) — edits PES save
   files (`EDIT00000000`): player stats, appearance, teams, tactics, AATF rule enforcement, save
   diffing, aesthetics transplant

The tight integration between the two is a primary design driver: aesthetic settings that currently
live only in the savefile (boots IDs, gloves IDs, physique, strip style, etc.) move into the
aesthetic exports as text files (TOML) and reach the game at compile time (Fox: rows of the output
CPK's database tables; pre-Fox: **written to the savefile**). This
eliminates the manual coordination between the save editor and the compiler, and enables automatic
boots/gloves ID assignment.

The remaining tools are listed in the plan table above; each plan carries its own scope and
rationale. Structural points worth noting here: the **Stadium compiler**'s codebase contributes the
in-process DDS conversion that eliminates the texconv.exe dependency; the **Export upgrader** exists
because the Team compiler only supports the Studio player-folder format; the three stream-side tools
(Music player, Music export editor, Match tracker) operate on audio and game memory rather than game
files, sharing only the platform and their own lib crates — the tracker feeds live match events to
the Music player, letting it run the match soundboard autonomously; the **Refs arranger** prepares
referee exports' `players.txt` and hands the compile off to the Team compiler, which owns referee
compilation; the **balls export** is the suite's third export kind, compiled by the Balls compiler
into its own CPK; and the **Player aesthetics editor** hands model viewing to Blender (launched with
a manifest consumed by the community's all-in-one `pes-models` extension) instead of embedding a 3D
preview — it only gains a labeled approximate in-app preview if the Team compiler's future
3D-preview feature creates the shared `model_viewport` lib; and the **Team creator** is a wizard
that takes a name and a list of players to a legal Team TOML plus a compilable aesthetics export
built from embedded starter heads — it owns no format and keeps no state, writing only files the
other tools already read, and shares its tactics and card widgets with the Save editor through
`libs/team_widgets`; and the **Ruleset editor** builds AATF rulesets through forms, as data that
a generic interpreter in the rules file evaluates, so an invitational's organizer can make rules
that need not resemble the official ones. Additional tools (CPK toolkit/decompiler; other
utilities) will build on the same lib crates and be specified later.

### Project context

What exists of this is in `spec/core/README.md` "Project context".

A standalone stadium compiler exists (Python, three-stage pipeline like Red):

| Tool | Role | Location |
|------|------|----------|
| PES Stadium Compiler | Compiles stadium exports into CPKs (fox2, stadium DB bins, in-process DDS conversion) | `Tools_4cc/pes-stadium-compiler-v1.0.0` |

A stream-side music player/editor pair exists (Python/tkinter, playback via libmpv, loudness
analysis via a bundled ffmpeg.exe):

| Tool | Role | Location |
|------|------|----------|
| Rigdio | Match-day goalhorn/anthem/chant player for streamers (.4ccm music exports) | `Tools_4cc/rigdio` |
| RigDJ | GUI editor for .4ccm files (same codebase) | `Tools_4cc/rigdio` |

A stream-side match stat extractor exists (C++/Qt, dormant but the only map of the game's in-memory
match structures):

| Tool | Role | Location |
|------|------|----------|
| SEN:P-AI | Live PES17 match stats/events via process-memory reading; fed Rigdio's event clips until Rigdio dropped the link in v1.11 | `Tools_4cc/SEN_P-AI` |

### GUI framework choice: egui

What exists of this is in `spec/core/README.md` "GUI framework choice: egui".

The suite also uses wgpu for first-release desktop GPU BC7 and the future 3D preview's
`egui_wgpu` render callback — see the [Team compiler plan](../team_compiler/README.md).

Trade-offs accepted:
- **No CSS transitions**: cell color changes are instant rather than smoothly fading. egui's
  built-in animation helpers (`ctx.animate_value_with_time` / `animate_bool`) drive the fade in
  the update loop; `egui_animation` adds easing helpers if the built-ins are not enough.
- **Styling is code, not CSS**: egui's default look is plain, so the suite ships a curated theme
  instead of defaults. One curated dark `Visuals` preset in `studio_core` (accent color,
  `CornerRadius`, `Shadow`, per-widget style overrides), the default and the theme every view
  is designed and reviewed in, applied by every tool. Light is a second-class alternative for
  the few members who want it, chosen by the common `theme` setting and switched live: egui's
  light `Visuals` with the same accent, rounding and status colors applied, checked once for
  legibility rather than per view. Views take their colors from the preset's semantic palette
  (status, warning, accent), never a hard-coded color, which is what keeps light readable at no
  per-view cost (the Team compiler's grid derives its label contrast from the cell for the same
  reason). Icon glyphs for sidebar/toolbar/status icons from the suite's own single-colour
  icon font, registered through `FontDefinitions` like any font: one SVG per icon, each tool's
  drawn to say what the tool does (the Team compiler's: the maintainer's running stickman
  dropping into an open box drawn from the front, flaps out; a box in perspective reads as a
  pedestal at 24 px, and an arrow does not show there; the stickman is a filled outline traced from the maintainer's drawing, which the build
  takes as it is). A tool's kind shows as one mark on its icon:
  an editor's carries a pencil (Phosphor's note-and-pencil pencil, redrawn smaller), a
  creator's or generator's a plus, each one shape at one size and place, the bottom-right
  corner, and a compiler's the open box of the Team compiler's, across the icon's foot; a mark
  hides what lies under it
  with a 12-unit gap cut around it, as Phosphor's note-and-pencil does, so a member reads the
  kind before the subject and no line runs through the mark. Each icon is one filled outline
  (the strokes outlined, the cut subtracted, the mark added), the form the font takes, kept
  as `resources/icons/<tool id>.svg` (its README says how they were drawn), the font build's
  input. The icons: the Save editor a floppy disk, the Export upgrader (c)'s two chevrons on an upright
  folder, its tab at the bottom right, the Music player Phosphor's music notes and the Music
  export editor the same with the pencil, the Match tracker a live feed on a monitor, the Kit
  config editor a shirt and the Refs arranger the same shirt striped its full length with a
  refresh sign (two arrows bending along half a circle each) as its mark, hiding the disc
  inside its ring too, the Balls compiler a ball dropping
  into the box, the Player aesthetics editor the stickman, the Team creator Phosphor's three
  figures, the Ruleset editor a clipboard of ticks, the DB generator a database; the Stadium
  compiler keeps Phosphor's buildings for now. They are built into `resources/fonts/` by a
  fontTools script, the
  general ones (help, settings, dismiss, warning, download) copied from Phosphor (MIT). A glyph
  takes the text colour, so hover and selection need no code of their own; multi-colour icons
  (SVG drawn at run time, `egui_extras`' `svg` feature) wait until after 1.0.0. Not the
  `egui-phosphor` crate: its one tie to egui, a font-registration helper, made every egui
  upgrade wait on its release (0.37's by weeks), and its stock glyphs say little about a
  tool. Custom fonts via `FontDefinitions`, and rounded,
  shadowed `Frame` chrome for panels and cards. A 2026 re-evaluation of the Rust GUI landscape
  against this suite's requirements (pure-Rust desktop rendering, same-codebase WASM, embedded
  wgpu viewport, ecosystem maturity for LLM-assisted coding) reconfirmed egui; the visual gap is
  closed with this theme work, not a framework change.
- **3D preview requires `wgpu`**: the future model preview viewport (see Future Features in the
  [Team compiler plan](../team_compiler/README.md)) would use `wgpu` rendering alongside egui rather than
  Three.js in a webview. More work, but it's a post-MVP feature: three.js would bring the glTF
  loader, the orbit camera and the skeleton overlay for free, where egui needs a small
  renderer of our own (shaders, a depth target, the camera), a one-off cost on one widget
  kept small by the preview's glance-only scope; PES's own shaders or posing would be custom
  work under either.
- **Log viewer performance**: egui's text rendering can be slow with thousands of lines. Mitigated
  by capping displayed lines to the last few hundred (full logs go to disk regardless).

## External Dependencies (Rust crates)

What exists of this is in `spec/core/README.md` "External Dependencies (Rust crates)".

| Crate | Purpose | Maturity |
|-------|---------|----------|
| `gltf` | glTF parsing | Production-ready |
| `egui_commonmark` | Markdown rendering for the help window (CommonMark subset: headings, lists, tables, code blocks, links) | `=0.26.0`, the release on egui 0.37 (each release follows one egui minor), with `default-features = false` and `pulldown_cmark` alone: the help shows no images, so `egui_extras`' image and file loaders stay out. Its link hooks carry the in-doc links. Chosen over a hand-rolled renderer (see `gui.md` "Help window") |
| `egui_animation` | Easing helpers for cell-color fades and panel transitions (on top of egui's built-in `animate_*`) | Release not yet reviewed; dropped if the built-in helpers suffice |
| `egui_extras` (`default-features = false`) | `TableBuilder` (resizable columns, sticky header, rows drawn only when visible; sorting is a clickable header over our own data) for the Save editor's player and team lists, the comparator and the AATF violations list; `StripBuilder` for proportional splits | Released with egui from its own repository (0.37.0 in the lockfile, 2026-10-23), so it moves in lock-step with the pinned egui and never holds an upgrade back. Third-party egui crates (`egui_dnd`, `egui_plot`) wait for the tool that needs them, since each can lag an egui release |
| `rfd` | Native file/folder dialogs | Production-ready |
| `indicatif` (`default-features = false`) | The CLI's progress line on stderr in `studio`, drawn only in a terminal (`architecture.md` "Event system") | Production-ready (0.18.6, published 2026-07-01, verified 2026-10-16: adds `console`, `encode_unicode`, `unit-prefix` and itself, all MIT or MIT/Apache, and about 145 KiB to a release binary) |
| `ureq` (3.x; features `rustls` [default], `platform-verifier`, `json`, `win-system-proxy`) | HTTP for the desktop updater: one Releases API GET and one streamed asset download per release | Production-ready (3.4.1 verified 2026-09-10). Chosen over `reqwest` because `reqwest` starts a Tokio runtime internally even in blocking mode, which breaks the "no async runtime" rule; the updater needs nothing `ureq` lacks. `platform-verifier` trusts the OS certificate store; `win-system-proxy` honors Windows proxy settings, env vars cover Linux |
| `sha2` | SHA256 verification of downloaded updates | Production-ready |
| `backtrace` | The panic hook's frames as module bases and offsets, unresolved, which `scripts/symbolize_log.py` names against the release's kept symbols (`architecture.md` "Diagnostic logging"); the standard library's `Backtrace` gives names or nothing | Production-ready (0.3.76, 2025-09-26; the crate the standard library's own backtrace is built from) |
| `crash-handler` | The crash dump's hook in the window's process (`architecture.md` "Diagnostic logging", the crash dump bullet): an unhandled exception on Windows, a signal on Linux, attached through its one `unsafe fn` | Production-ready (0.8.1, Embark Studios; measured at 0.0.1k4 with `minidumper` and the `minidump-writer` under it: 221 KiB on a probe exe, 140.5 KiB on ours at 0.0.1k4b) |
| `minidumper` | The link from the hook to the crash monitor, and the monitor's dump, written through `minidump-writer` (not a dependency of ours) | Production-ready (0.11.0, Embark Studios) |
| `minidump` (dev) | Reads the crash test's dump back in `studio`'s tests (ST-RUN-10) | Production-ready (0.27.0, the `rust-minidump` project) |
| `winresource` (build, Windows hosts) | `studio`'s build script: compiles `resources/icon/icon.ico` and the version details into the exe through the Windows SDK's `rc.exe` (`development_plan.md` "The icon"); `default-features = false` drops its `toml` feature, which reads package metadata we do not use | Production-ready (0.1.34, 2026-10-12, MIT; adds `version_check` alone, a build-time crate) |
| `raw-window-handle` (Windows targets), and `windows`' `Win32_UI_Controls` feature | `studio_core` reads the native window's handle from eframe's creation context and hides the title bar's icon with `SetWindowThemeAttribute` (`development_plan.md` "The icon", `small.png`'s bullet) | 0.6.2, the version eframe and winit already resolve, so the build holds one; no crate added to the build |
| `rhai` | AATF rules file (the ruleset data and its interpreter; sandboxed, see the AATF rules plan) | Production-ready (1.26.0, 2026-08; the language server `rhaiscript/lsp` is abandoned, which nothing here depends on) |
| `notify` + `notify-debouncer-full` | Exports folder watching for live validation | Production-ready |
| `kira` | Audio playback (mixer, tweens/fades, loop regions, effects; native + WASM) | Production-ready |
| `symphonia` | Audio decoding (MP3, Vorbis, FLAC, AAC/M4A, WAV) | Production-ready |
| pure-Rust Opus decoder (e.g. `opus-decoder`) | Opus decoding, registered into symphonia (its own Opus support is unfinished) | Verify maturity (fallback: statically linked `audiopus`) |
| `ebur128` | EBU R128 loudness measurement (replaces ffmpeg volumedetect) | Production-ready |
| `qrcode` | QR timestamp widget (match tracker stream-sync aid) | Production-ready |

## Key Decisions Summary

What exists of this is in `spec/core/README.md` "Key Decisions Summary".

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Universal model source | glTF (`.glb`, single file; `.gltf`+`.bin` accepted) with *optional* PES extensions (PES_bone/PES_mesh) + materials.toml — own plan: [Unified model format](../model_format.md) | Blender exports natively, mature Rust parser, extensible; material properties in comment-friendly TOML, not uncommented JSON. Design goal: one authored model converts to `.model` and FMDL with no loss of customizability in the simplest possible way, or users keep the native formats. Stock-Blender exports must compile (extensions have fallbacks, PBR material data is ignored), which is why a custom `.pes` extension was rejected — it would make the plugin mandatory |
| glTF material files | `materials.toml` (catch-all) + `*.materials.toml` (name-matched) + `.common` links to Common's files, mirroring `.model`/`.mtl` and Red's `.mtl.common` | Materials never embedded in glTF JSON — glTF files need textures anyway, JSON has no comments, and tomls-as-siblings make sharing unambiguous; name-matching (startsWith/endsWith) ports Red's `.mtl` logic for the same modular copy-paste workflow; Common-linked models resolve materials in Common first with local overrides layered on top (Red's cascade); missing material = hard error (no safe defaults without texture paths) |
| Stadium compiler | `tools/stadium_compiler` crate | Ports the Python stadium compiler; fox2 parser goes in `libs/fox2` |
| GUI validation | Live, no Check/Refresh buttons | Folder watcher + deep checks for extracted folders, shallow (blue) for archives |
| GUI actions | Single Compile button (becomes Cancel) | Staged operations superseded by the unified pipeline |
| Export upgrader | Dedicated tool crate | Merges single-user model folders into player folders; shared folders + link files; savefile-driven ID mapping |
| Refs tooling | Arranger + handoff, no engine split | The Refs arranger edits the refs export's `players.txt` (via `aesthetics_export`) and switches to the Team compiler for the compile — a compile-engine lib crate would have one thin extra consumer |
| Balls tooling | Standalone compiler + balls export | Ball content shares nothing with aesthetics exports (own paths, bins, CPK), so the Balls compiler owns its compile on the shared libs; balls become the third export kind in `exports/` (first word `balls`, skipped by the Team compiler) |
| Distribution | Portable `.7z` (binary + `quick_compile.bat` + exports/ + teams list + readme), no installer | Extract anywhere and run; no elevation, no registry. Extract-and-run with no external runtime is the requirement, not file count — Red's many-file bundle causes users no trouble |
| Quick-compile launcher | `quick_compile.bat` → `UTW --gui team-compiler compile`; GUI-subsystem exe + `AttachConsole` for CLI output; `exports/` beside the exe in both data modes | Red's `0_all_in_one.bat` is the one script users actually run; the GUI autorun keeps that double-click habit while showing the grid/log instead of a console. `gui_run` trait hook reuses the tool's CLI grammar, so no second command surface |
| Studio Web | Post-release lite browser edition, not a first-release gate; lib crates `wasm32`-checked in CI from Phase 1 | Convenience for compiling/editing one's own team from another PC via a bookmark, never full-cup compilation (4 GB `wasm32` ceiling); the cheap file-picking tier ships first, the pipeline tier needs adapters + COOP/COEP hosting on the community server |
| Self-update | In-place binary swap (move-running-exe pattern) + teams-list merge; previous binary kept in `old/` with a rollback button | Inverts Red's sibling-folder update: the folder is a few files, user data stays put; settings need no transfer (default-merge handles new keys); the `old/` subfolder keeps the previous binary out of misclick range |
| Save editor scope | Full 4ccEditor parity + direct-manipulation tactics UI | Every 4ccEditor feature carried over; drag-and-drop replaces spinner/dropdown friction |
| Midcupping tools | Absorbed into the save editor | The aesthetics diff as the comparator panel, its operation in `pes_savefile`; the save-to-save transplant is not ported, its successor writes the team's export (`save_editor.md` "Aesthetics transplant") |
| Boots/gloves IDs | Automatic deterministic assignment for custom models; stock models (IDs 0–100) named in `settings.toml` | Player-exclusive: from (team_id, player_number); shared: per-team pool; a player wearing a game model has no folder, so `boots_id`/`gloves_id` name it; Fox: rows of the output CPK's `BootsList`/`GloveList`; pre-Fox: written to the savefile at compile time |
| Player settings | TOML in exports (`settings.toml`), merged at compile time; all fields optional; schema covers every savefile aesthetic field (tested) | Replaces manual savefile editing; editor can generate TOML from saves; name writing is opt-in (`name = true`/string — absent leaves the savefile name untouched), so shared folders act as generic model folders |
| Aesthetics patch | `aesthetics_patch.toml` written to the output folder with the CPK (on Fox only motions and names, appearance travelling in the CPK's database tables over a stripped save); the compiler's only savefile write path (the local save is updated by applying it); applied to other saves by the save editor, whose own aesthetics fields are read-only by default (session unlock) | Separates the DLC builder from the savefile builder — the official save holds the teams' custom tactics, which the DLC builder must not receive; one write path means no drift between the two machines |
| FPC toggle | Empty `fpc_on`/`fpc_off` marker files in the player folder | Folder-level like link files; apply `libs/fpc`'s enable/disable presets (same as the editor's toggle); absent = no preset applied, the keys taking their defaults |
| AATF rules | One self-contained Rhai file: the ruleset as data (`RULESET`), a generic interpreter carried in the file, optional hand-written custom checks; built without code in the Ruleset editor | Anyone can make an invitational's ruleset, and it need not resemble the official one; the interpreter in the file keeps a ruleset's meaning fixed across Studio builds; TOML+CEL, a two-file split, hand-written logic per ruleset and a Rust-interpreted TOML rejected ([AATF rules plan](../aatf_rules.md)) |
| Music tools | Two tool crates (`music_player`, `music_export_editor`) + `music_export`/`audio_engine` libs | Rigdio/RigDJ successors; descriptive crate names, old names kept in the user-facing labels ("Music player (Rigdio)") and old icons in the icon rail |
| .4ccm format | Canonical, unchanged, read + write | Interop with legacy Rigdio during transition; no new format, no migration for managers |
| Audio backend | Pure Rust: `kira` + `symphonia` (+ Rust Opus decoder) + `ebur128` | Drops libmpv-2.dll and ffmpeg.exe; single binary; in-process loudness; WASM-compatible |
| Match tracker | `tools/match_tracker` crate; per-version memory signatures (PES17 initially); Windows-only backend behind a memory-source trait | SEN:P-AI successor; offsets are data, not code; Win32 is the only backend for now |
| Match events interface | In-process `libs/match_feed` (typed pub/sub + snapshot replay); no pipe/socket, `watch` CLI for external scripts | Tracker and music player share one binary; the old pipe protocol has no surviving consumers |
| Match records | New JSON format (`.match.json`); binary `.sen` dropped | Readable and schema-stable; old archives keep old SEN:P-AI |
| Music autopilot | Tracker-driven team loading/scoring/goalhorns/event clips; Full, Co-pilot (highlights + automatic buttonless clips), Assist (highlight-only), or Off (suppresses tracker-triggered playback/highlights but does not transfer score authority); manual override always live | The suite's end goal: the music player runs the match unattended, with Co-pilot/Assist/Off as trust-building middle steps |
| Player aesthetics viewing | Blender launch via a versioned JSON manifest consumed by the `pes-models` extension's loader; the in-app quick preview waits for `libs/model_viewport`, created only with the Team compiler's future 3D-preview feature | Blender's viewport/Outliner beat any bespoke wgpu pass and stay the accurate view (the future embedded preview is labeled approximate); export-format knowledge stays in Rust (the loader is manifest-driven); glTF conversion stays Studio-side in `model_convert` (`ir_to_gltf` + superset merge) |
