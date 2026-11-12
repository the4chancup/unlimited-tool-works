# Unlimited Tool Works: Core spec: Distribution and updates

Part of the [Core spec](README.md).

## Distribution and updates

### License

**`MIT OR Apache-2.0`** (the Rust ecosystem's dual license):

- declared once as `license = "MIT OR Apache-2.0"` in the workspace `Cargo.toml` and inherited
  by every crate;
- `LICENSE-MIT` and `LICENSE-APACHE` at the repository root;
- contributions are accepted under the same terms (the README says so).

Red stays GPL-3.0. Studio is an independent reimplementation of its ideas, not a derivative of
its code (see "Relationship to legacy tools"), and Red's author is this project's author, so
nothing in Red's license binds Studio.

Permissive rather than copyleft because the suite is *designed as libraries with tools on top*,
for a community that writes its own tools in whatever language the author knows. A GPL
`pes_savefile` or `cpk` could go into a GPL Blender add-on and nowhere else, which would cancel
half the reason the crate boundaries exist. The copyleft threat, a closed fork, has never
materialized in this community's tooling and would do it no harm if it did; the open version
stays.

Apache alongside MIT for the explicit patent grant and because it is what every dependency uses.
The door MIT closes (absorbing GPL code) leads nowhere: Red is the author's own, 4ccEditor is
zlib, and the other legacy tools carry no license at all, unusable verbatim under any license.

What the license does **not** cover: game-derived data kept for interoperability (the skeletons
in `resources/skeletons/`, `DpFileList.bin` and the other embedded `.bin` templates, the kit-icon
reference sheet). They are Konami's; the README states it.

`just deps-check` enforces the dependency side with a `cargo deny` license allowlist, so a
copyleft crate cannot enter the dependency graph unnoticed. The list lives in `deny.toml`:

- `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`,
  `Zlib`, `BSL-1.0`, `Unicode-*`, `CC0-1.0`;
- `MPL-2.0` for file-level copyleft crates only;
- the font licenses `OFL-1.1` and `Ubuntu-font-1.0` that egui's bundled default fonts carry;
- the data license `CDLA-Permissive-2.0` of the root-certificate list ureq's certificate
  verifier pulls on wasm32.

A license compatible with `MIT OR Apache-2.0` (it puts no copyleft obligation on Studio's source
or binary) joins the list when a dependency needs it, without asking; any other license is a
question for the maintainer.

### Distribution: portable .7z bundle

The current teams list is not a bundle file: it is embedded in the binary and materialized as
`data/teams_list.txt` by the first `compile` that has a data directory (a `check` never writes it;
see the [Team compiler spec](../team_compiler/README.md), "Resolved
decisions", "Teams list").

**Size budget:** the release binary stays under about 30 MB, and up to 50 MB is acceptable where
a feature truly needs it. A new dependency is weighed against this, measured on an optimized
build, not a debug one.

The measurements (the tables are `cargo bloat --release -p studio --crates`, whose
per-crate split is an estimate, as the tool itself says):

- **2026-10-15** (default release profile, static C runtime, Phase 5's code): 30.8 MiB, up from
  17.1 MiB on 2026-10-02 (Phase 3). The growth is features, not an accident (`cargo bloat`,
  crate by crate):
  - `rhai` and the `aatf` crate 2.6 MiB of code;
  - the Phase 4 and 5 crates (the Team compiler's later steps, `pes_savefile`, `pes_model`,
    `model_convert`, the Save editor, the image formats) about 5 MiB;
  - read-only data from 5.4 to 10.9 MiB (embedded files about 4.2 MiB of it: the referee
    templates about 3 MiB, the kit bins, the help chapters and the AATF sample team).
- **2026-10-27, the 0.0.1 preview** (the build `just release` makes, at commit `745ea9f4`):
  37.6 MiB (39,380,992 bytes), past the "about 30 MB" and inside the 50 MB ceiling, and again
  features. Code grew from 19.1 to 24.5 MiB, nearly all of it the tools and the GUI the preview
  brings:
  - the Save editor's view 1.1 MiB, up 0.9;
  - the Export upgrader and the Ruleset editor, 0.55 and 0.42 MiB, newly linked;
  - `studio_core`'s shell up 0.36;
  - `egui` up 0.57 as the views use more of it;
  - `team_widgets`, `egui_extras`, the help window's Markdown and its text shaping about 0.6
    together.

  Besides these, `aatf` is up 0.28 and `std` up 0.4. Read-only data is 12.0 MiB, embedded files
  5.3 MiB of it (the body font, 0.34 MiB, the largest new one).
- **The release itself** (0.0.1, commit `c97ce914`): 41.6 MiB (43,661,824 bytes here; the
  published exe, built by CI, 43,813,376), code 26.9 MiB:
  - the update check's HTTPS client newly linked (`ureq`, `rustls` and `ring`, 0.83 MiB);
  - the Save editor up 0.4 to 1.5 MiB;
  - `std` up 0.3, `studio_core` up 0.25.

  Read-only data is 13.5 MiB.

The release profile stays the default: `panic = "abort"` would cut the unwinding tables, but the
GUI reports a compile thread's panic instead of closing, and LTO trades longer builds for size.
The one cut taken is `rhai`'s `only_i64` (0.6 MiB; [External
Dependencies](README.md#external-dependencies-rust-crates)). Storing the referee templates
compressed would save an estimated 1.5 to 2 MiB more, if the budget ever needs it.

**Platform floor:** Windows 10 x64 and 64-bit Linux. The Windows exe cannot load on Windows 7,
and Windows 8.1 is untested; supporting either is not a goal:

- Rust's `x86_64-pc-windows-msvc` target has required Windows 10 since Rust 1.78 (its standard
  library imports `WaitOnAddress` and `GetSystemTimePreciseAsFileTime`, which Windows 7 lacks);
- the GUI stack imports `combase.dll` (through `windows-core`, used by `wgpu-hal` and
  `accesskit_windows`), which Windows 7 does not have under that name.

The cup's last Windows 7 users moved to Linux, which the Linux build serves. The Windows exe links
the C runtime statically (`.cargo/config.toml`), so it runs on a clean Windows 10 without the
Visual C++ redistributable installed.

### Release builds in CI

A release is built by GitHub Actions, never on a developer's machine, so the binary the
maintainer checks before publishing is the one members download (a local build and CI's
differ at least in the linker's timestamps and the toolchain's machine).

The workflow, `.github/workflows/release.yml`, runs `release.py` on `windows-latest` and on
`ubuntu-22.04`. Each job checks its own binary with the recipe's checks (no build-machine path
in it, the static C runtime on Windows, the listing equal to the staged set), then:

- **A tag `v<version>`** (a semver: `v0.0.1`, `v0.0.1-rc.1`) whose version is the workspace's
  creates a **draft** GitHub release named for the tag, marked pre-release when the version has
  a pre-release part. It holds three assets:
  - the Windows `.7z`;
  - the Linux `.tar.gz`;
  - `SHA256SUMS.txt`, both archives' `sha256sum` lines, the file the updater checks a download
    against (`plans/core/distribution.md` "Self-update" step 3).

  A draft is visible to the repository's maintainers alone. The maintainer downloads the
  Windows asset, gives it the look before the cut, writes the notes and publishes it. The notes
  are the release's body, the text the updater's dialog shows, taken from `CHANGELOG.md`'s
  section, for every release after 0.0.1 (the file came the day after it).
- **A push to `ci/release-dry-run`**, or a run started by hand once the workflow is on the
  default branch (GitHub offers its **Run workflow** button only for a workflow there), builds
  and checks the same bundles and uploads them as the run's artifacts only: no tag, no release.
  That is how the workflow itself is tested before a tag relies on it.

Each job also uploads its symbols archive (Windows' `-symbols.7z`, the exe and its `.pdb`;
Linux's `-linux-symbols.tar.gz`, `utw.debug`) as a run artifact, never a release asset. The
archive stays with the maintainer, who keeps a copy beside the release's other files, because
artifacts expire after 90 days and `symbolize_log.py` needs it for as long as the release is in
use.

Before 0.0.1's tag, one test tag `v0.0.1-rc.1`, pushed from a branch whose workspace version is
bumped to it, runs the release job once. Its draft pre-release's three assets and their
checksums are checked, then the draft and the tag are deleted.

No release candidate is published, because a published pre-release is offered to every member
whose version it outranks (the check reads pre-releases). The environment variable
`UTW_RELEASES_URL`, when set, replaces the address the check reads the list of releases from,
so a test build reads a list served from this PC or a test repository's, whose assets are two
test bundles. It is a test hook, in no help topic and no settings file, because whoever can set
a program's environment can already replace the program.

The Linux bundle's look runs in WSLg (Ubuntu 22.04 on the maintainer's PC) before a release is
published.

The user extracts the archive anywhere and runs the binary. In the common case the user drops
an export into `exports/` (shipped empty beside the binary) and double-clicks
`quick_compile.bat`, which is the whole Red workflow carried over. Where the suite's persisted
data goes (the settings file, state like the skipped-update version, logs) is the user's
choice, asked once on first run (see "Data location" below). This is the same folder model Red
uses, minus the ~80MB embedded Python runtime that made Red's updates a folder-replacement
operation.

### Data location: asked on first run

Settings and data live in one of two locations:

- **Next to the executable**: a `data/` subfolder beside the binary. Fully portable: the folder
  carries the program and everything it persists; deleting it removes the program completely, moving
  it moves everything.
- **In the user config directory**: `%APPDATA%\unlimited-tool-works` on Windows, `~/.config/unlimited-tool-works` on
  Linux (via the `directories` crate). Survives deleting or replacing the program folder.

Resolution at startup is **presence-based, no marker files** (EGG's mechanism):

1. if `data/settings.toml` exists next to the binary → portable mode;
2. otherwise if the config-dir settings file exists → config-dir mode;
3. if neither → first run.

The CLI resolves the location the same way but never asks and never writes the settings file,
because a noninteractive run cannot make the choice, and a `check` that created files would not
be a check. With neither settings file present it runs on the defaults in memory, with no data
directory (`AppPaths::data_dir` is `None`). Data-dir cargo is then read from its embedded default
(the teams list) and nothing is persisted, so the first-run choice stays open for the first GUI
start.

`data/teams_list.txt` *is* data-dir cargo (member-edited; the grid's inline ID writes are the
[Team compiler spec](../team_compiler/pipeline.md)'s, "Resolved decisions", "Teams list"). Writes
to it are best-effort: an unwritable data directory makes the list read-only, never triggers
elevation.

The following things are *not* data-dir cargo:

- The `exports/` folder (the default `exports_folder_path`) lives **next to the binary in both
  modes**, beside `quick_compile.bat`. Copying exports into `%APPDATA%` is impractical and would
  break the "drop it in the folder, double-click" habit Red established; users who want it elsewhere
  point the setting at another path.
- The `output/` folder (the default `output_folder_path`) likewise lives next to the binary. It is
  Red's `patches_output/`, where users look for a CPK when a deployment could not happen or
  `--no-deploy` was given. It also hosts the run-scoped staging folder (see "Post-processing" in
  the Team compiler spec).

### Update state

Stored with the settings (common settings, `studio_core`):

| Key | Replaces (Red) | Default |
|-----|----------------|---------|
| `check_for_updates` | `updates_check` ini key | `true` |
| `check_interval_minutes` | hardcoded 60 minutes | `120`: minutes between release checks |
| `last_update_check` | `state/update_check_last.txt` | - |
| `skipped_version` | `state/update_skip_last.txt` | - |

This closes out Red's entire `Engines/state/` folder. The remaining five files either became
settings or lost their reason to exist:

| Red state file | Fate |
|---|---|
| `dt00_write_allowed.txt` | Dropped. The Team compiler never writes `dt00_x64.cpk`: the Fox referee marker is a reserved collar inside the refs CPK (`spec/team_compiler/blue_port.md` "Referee export processing") |
| `first_run_done.txt` | Presence-based data-location resolution: the settings file's existence *is* the first-run marker (see "Data location") |
| `admin_warned.txt` | Dropped: the one-time console explanation before Red's UAC relaunch becomes the `elevation` lib's GUI prompt, which explains itself every time it appears |
| `sideload_warned.txt` | Dropped: the one-time sideload explanation becomes the per-run `overrides_active` Info message (Red's `sideload/` folder is the compiler's `overrides/`) (team compiler catalog) |
| `ver_mismatch_warned.txt` | Dropped: the suppressible console notice becomes the plain per-run `pes_version_mismatch` warning (team compiler catalog) plus the version selector's live red/yellow exe check (see `plans/core/gui.md` "Sidebar") |

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
  downstream *crate* consumers, and there are none: every internal crate's only consumer is
  `studio` (or another internal crate), all compiled together in one binary.
- **Tools are compile-time plugins**, statically registered in `studio`. A "team_compiler 2.0 vs.
  studio 1.5" compatibility question is impossible by construction: a version mismatch can't ship.
- **Changelog and rollback are per-Studio-release.** Release notes are "Unlimited Tool Works 1.4 adds the
  Balls compiler"; a rollback puts back the one binary kept in `old/`. Both only make sense on
  one version axis.

A release is cut (tag, GitHub Release, `.7z` asset) when there's something to distribute; not every
commit is a release. The workspace version is the next-release version; the tag is what the updater
compares against.

The suite ships `0.x` releases from the 0.0.1 preview (`plans/core/development_plan.md`
"Divegrass Before Christmas") and 0.1.0 (end of Phase 8) until 1.0.0 (end of Phase 16; the same
plan's "Releases"), so the workspace says 0.0.1 until that preview is cut. The compatibility rule
for released surfaces binds from 1.0.0. The `0.x` releases are test releases for a restricted
group of testers, told in person of every change, so everything can still change before then.

The version a member sees is `v{CARGO_PKG_VERSION}`: in the status bar, and from `--version`
beside the executable's name (`UTW v0.0.1`), the line a bug report quotes.

**One exception: `python_bindings`.** It is built via maturin into a wheel that Blender users
install independently of the Studio binary, and its consumers are not Studio users. Its version is
meaningful to them (which wheel matches their Blender / which `fmdl`+`pes_model` API), so it keeps
its own `version = "..."` and its own release cadence, shipping on demand when the
`fmdl`/`pes_model` API surface it exposes changes. This is also why its wheel is built by
`maturin` (`just bindings`) rather than as part of the Studio binary's build.
