# Unlimited Tool Works — Core plan: Distribution and updates

Part of the [Core plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Distribution and updates

### License

**`MIT OR Apache-2.0`** — the Rust ecosystem's dual license, declared once as
`license = "MIT OR Apache-2.0"` in the workspace `Cargo.toml` and inherited by every crate;
`LICENSE-MIT` and `LICENSE-APACHE` at the repository root; contributions are accepted under the
same terms (the README says so). Red stays GPL-3.0; Studio is an independent reimplementation of
its ideas, not a derivative of its code (see "Relationship to legacy tools"), and Red's author is
this project's author, so nothing in Red's license binds Studio.

Permissive rather than copyleft because the suite is *designed as libraries with tools on top*,
for a community that writes its own tools in whatever language the author knows: a GPL
`pes_savefile` or `cpk` could go into a GPL Blender add-on and nowhere else, which would cancel
half the reason the crate boundaries exist. The copyleft threat — a closed fork — has never
materialized in this community's tooling and would do it no harm if it did; the open version
stays. Apache alongside MIT for the explicit patent grant and because it is what every dependency
uses. The door MIT closes (absorbing GPL code) leads nowhere: Red is the author's own, 4ccEditor
is zlib, and the other legacy tools carry no license at all — unusable verbatim under any license.

What the license does **not** cover: game-derived data kept for interoperability — the skeletons
in `resources/skeletons/`, `DpFileList.bin` and the other embedded `.bin` templates, the kit-icon
reference sheet. They are Konami's; the README states it. `just deps-check` enforces the
dependency side with a `cargo deny` license allowlist (`MIT`, `Apache-2.0`, `Apache-2.0 WITH
LLVM-exception`, `BSD-2-Clause`,
`BSD-3-Clause`, `ISC`, `Zlib`, `BSL-1.0`, `Unicode-*`, `CC0-1.0`, `MPL-2.0` for file-level
copyleft crates only, plus the font licenses `OFL-1.1` and `Ubuntu-font-1.0` that egui's bundled default fonts
carry), so a copyleft crate cannot enter the dependency graph unnoticed. The list lives in
`deny.toml`. A license compatible with `MIT OR Apache-2.0` (it puts no copyleft obligation on
Studio's source or binary) joins the list when a dependency needs it, without asking; any other
license is a question for the maintainer.

### Distribution: portable .7z bundle

The current teams list is not a bundle file: it is embedded in the binary and materialized as
`data/teams_list.txt` by the first `compile` that has a data directory (a `check` never writes it;
see the [Team compiler plan](../team_compiler/README.md), "Resolved
decisions", "Teams list").

**Size budget:** the release binary stays under about 30 MB, and up to
50 MB is acceptable where a feature truly needs it. A new dependency is weighed against this,
measured on an optimized build, not a debug one. Measured on 2026-10-15 (default release profile,
static C runtime, Phase 5's code): 30.8 MiB, up from 17.1 MiB on 2026-10-02 (Phase 3), and the
growth is features, not an accident (`cargo bloat`, crate by crate): `rhai` and the `aatf` crate
2.6 MiB of code, the Phase 4 and 5 crates (the Team compiler's later steps, `pes_savefile`,
`pes_model`, `model_convert`, the Save editor, the image formats) about 5 MiB, and read-only data
from 5.4 to 10.9 MiB (embedded files about 4.2 MiB of it: the referee templates about 3 MiB, the kit
bins, the help chapters and the AATF sample team). Measured again on 2026-10-27 for the 0.0.1
preview (the build `just release` makes, at commit `745ea9f4`): 37.6 MiB (39,380,992 bytes),
past the "about 30 MB" and inside the 50 MB ceiling, and again features: code grew from 19.1 to
24.5 MiB, nearly all of it the tools and the GUI the preview brings (the Save editor's view
1.1 MiB, up 0.9; the Export upgrader and the Ruleset editor, 0.55 and 0.42 MiB, newly linked;
`studio_core`'s shell up 0.36; `egui` up 0.57 as the views use more of it; `team_widgets`, `egui_extras`, the
help window's Markdown and its text shaping about 0.6 together), with `aatf` up 0.28 and `std`
up 0.4; read-only data is 12.0 MiB, embedded files 5.3 MiB of it (the body font, 0.34 MiB,
the largest new one). The tables are `cargo bloat --release -p studio --crates`, whose
per-crate split is an estimate, as the tool itself says. The release profile stays the default:
`panic = "abort"` would cut the unwinding tables, but the GUI reports a compile thread's panic
instead of closing, and LTO trades longer builds for size. The one cut taken is `rhai`'s
`only_i64` (0.6 MiB; [External Dependencies](README.md#external-dependencies-rust-crates));
storing the referee templates compressed would save an estimated 1.5 to 2 MiB more, if the
budget ever needs it.

**Platform floor:** Windows 10 x64 and 64-bit Linux. The Windows exe cannot load on Windows 7,
and Windows 8.1 is untested; supporting either is not a goal: Rust's `x86_64-pc-windows-msvc` target has required
Windows 10 since Rust 1.78 (its standard library imports `WaitOnAddress` and
`GetSystemTimePreciseAsFileTime`, which Windows 7 lacks), and the GUI stack imports
`combase.dll` (through `windows-core`, used by `wgpu-hal` and `accesskit_windows`), which
Windows 7 does not have under that name. The cup's last Windows 7 users moved to Linux, which
the Linux build serves. The Windows exe links the C runtime statically (`.cargo/config.toml`), so
it runs on a clean Windows 10 without the Visual C++ redistributable installed.

### Data location: asked on first run

Settings and data live in one of two locations:

- **Next to the executable** — a `data/` subfolder beside the binary. Fully portable: the folder
  carries the program and everything it persists; deleting it removes the program completely, moving
  it moves everything.
- **In the user config directory** — `%APPDATA%\unlimited-tool-works` on Windows, `~/.config/unlimited-tool-works` on
  Linux (via the `directories` crate). Survives deleting or replacing the program folder.

Resolution at startup is **presence-based, no marker files** (EGG's mechanism): if
`data/settings.toml` exists next to the binary → portable mode; otherwise if the config-dir settings
file exists → config-dir mode; if neither → first run.

The CLI resolves the location the same way but never asks and never writes the settings file: a
noninteractive run cannot make the choice, and a `check` that created files would not be a
check. With neither settings file present it runs on the defaults in memory, with no data
directory (`AppPaths::data_dir` is `None`): data-dir cargo is read from its embedded default
(the teams list) and nothing is persisted, so the first-run choice stays open for the first GUI
start (the dialog is Phase 8's).

`data/teams_list.txt` *is* data-dir cargo (member-edited; the grid's inline ID writes are the
[Team compiler plan](../team_compiler/pipeline.md)'s, "Resolved decisions", "Teams list"), and writes to it are best-effort:
an unwritable data directory makes the list read-only, never triggers elevation. The following
things are *not* data-dir cargo:

- The `exports/` folder (the default `exports_folder_path`) lives **next to the binary in both
  modes**, beside `quick_compile.bat`. Copying exports into `%APPDATA%` is impractical and would
  break the "drop it in the folder, double-click" habit Red established; users who want it elsewhere
  point the setting at another path.
- The `output/` folder (the default `output_folder_path`) likewise lives next to the binary — it is
  Red's `patches_output/`, where users look for a CPK when a deployment could not happen or
  `--no-deploy` was given; it also hosts the run-scoped staging folder (see "Post-processing" in
  the Team compiler plan).

### Update state

Stored with the settings (common settings, `studio_core`):

| Key | Replaces (Red) | Default |
|-----|----------------|---------|
| `check_for_updates` | `updates_check` ini key | `true` |
| `check_interval_hours` | hardcoded 60 minutes | `24` (a resident GUI app, not a per-compile CLI run) |
| `last_update_check` | `state/update_check_last.txt` | — |
| `skipped_version` | `state/update_skip_last.txt` | — |

This closes out Red's entire `Engines/state/` folder — the remaining five files either became
settings or lost their reason to exist:

| Red state file | Fate |
|---|---|
| `dt00_write_allowed.txt` | Dropped — the Team compiler never writes `dt00_x64.cpk`: the Fox referee marker is a reserved collar inside the refs CPK (`spec/team_compiler/blue_port.md` "Referee export processing") |
| `first_run_done.txt` | Presence-based data-location resolution: the settings file's existence *is* the first-run marker (see "Data location") |
| `admin_warned.txt` | Dropped — the one-time console explanation before Red's UAC relaunch becomes the `elevation` lib's GUI prompt, which explains itself every time it appears |
| `sideload_warned.txt` | Dropped — the one-time sideload explanation becomes the per-run `overrides_active` Info message (Red's `sideload/` folder is the compiler's `overrides/`) (team compiler catalog) |
| `ver_mismatch_warned.txt` | Dropped — the suppressible console notice becomes the plain per-run `pes_version_mismatch` warning (team compiler catalog) plus the version selector's live red/yellow exe check (see `plans/core/gui.md` "Sidebar") |

### Versioning: one workspace version, one exception

The suite uses a **single workspace version** for every crate that ships in the `studio` binary. Set
once in the root `Cargo.toml`:

```toml
[workspace.package]
version = "0.0.1"
```

Every tool and lib crate inherits it via `version.workspace = true`. Bumping is one line, applied to
all crates at once.

Rationale:

- **The binary is the release unit.** Users download `UTW.exe`; the self-updater compares
  `CARGO_PKG_VERSION` against a GitHub Release tag. The version they see *is* the suite version.
  Independent crate versions would be invisible to every actual consumer.
- **Nothing is published to crates.io.** Per-crate semver exists to signal breaking changes to
  downstream *crate* consumers; there are none; every internal crate's only consumer is `studio` (or
  another internal crate), all compiled together in one binary.
- **Tools are compile-time plugins**, statically registered in `studio`. A "team_compiler 2.0 vs.
  studio 1.5" compatibility question is impossible by construction — a version mismatch can't ship.
- **Changelog and rollback are per-Studio-release.** Release notes are "Unlimited Tool Works 1.4 adds the
  Balls compiler"; the rollback button swaps two binaries in `old/`. Both only make sense on one
  version axis.

A release is cut (tag, GitHub Release, `.7z` asset) when there's something to distribute; not every
commit is a release. The workspace version is the next-release version; the tag is what the updater
compares against. The suite ships `0.x` releases from the 0.0.1 preview (`plans/core/development_plan.md`
"Divegrass Before Christmas") and 0.1.0 (end of Phase 8) until 1.0.0 (end of Phase 16; the same
plan's "Releases"), so the workspace says 0.0.1 until that preview is cut. The compatibility rule for released
surfaces binds from 1.0.0: the `0.x` releases are test releases for a restricted group of
testers, told in person of every change, so everything can still change before then.

The version a member sees is `v{CARGO_PKG_VERSION}`: in the status bar, and from `--version`
beside the executable's name (`UTW v0.0.1`), the line a bug report quotes.

**One exception: `python_bindings`.** It is built via maturin into a wheel that Blender users
install independently of the Studio binary, and its consumers are not Studio users. Its version is
meaningful to them (which wheel matches their Blender / which `fmdl`+`pes_model` API), so it keeps
its own `version = "..."` and its own release cadence — shipping on demand when the
`fmdl`/`pes_model` API surface it exposes changes. This is also why its wheel is built by
`maturin` (`just bindings`) rather than as part of the Studio binary's build.
