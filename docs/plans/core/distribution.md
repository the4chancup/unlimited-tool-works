# Unlimited Tool Works — Core plan: Distribution and updates

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Distribution and updates

### Distribution: portable .7z bundle

What exists of this is in `spec/core/distribution.md` "Distribution: portable .7z bundle".

There is no installer. Each release is a `.7z` archive containing:

- `UTW.exe` (or the `utw` Linux binary) — the single self-contained binary
  (`development_plan.md` "The project's name: Unlimited Tool Works")
- `quick_compile.bat` (`quick_compile.sh` on Linux) — the double-click launcher: opens the GUI on
  the Team compiler and starts a compile (see `architecture.md` "Launch modes" under Architecture)
- in the 0.0.2 preview only (0.0.1 is the Save editor's alone), until the two tools' views
  land: `quick_compile.bat` in its CLI form, `quick_upgrade.bat` and an empty `old_exports/`
  (`development_plan.md` "The /vg/ League 27 (VGL)"); the launchers' sources are `resources/bundle/`, checked out with CRLF
  line ends (`.gitattributes`), since cmd misreads a batch file's labels with LF ones
- a basic readme (pointing into the app: the full help and the changelog live in the help window
  — see `gui.md` "Help window" under GUI Design — and `UTW help --export` produces the same text as
  files for the release page); `README.txt`, not
  `.md`, because a double-click opens a `.txt` in Notepad on any Windows, where a `.md` opens
  whatever app claimed it, or asks
- a `licenses/` folder: the project's `LICENSE-MIT` and `LICENSE-APACHE`; the licenses of the
  two third-party works the suite bundles itself rather than through a crate, Roboto Flex's
  `RobotoFlex-OFL.txt` (`resources/fonts/`) and `PHOSPHOR-LICENSE.txt` (`resources/icons/`,
  for the icon font's outlines copied from Phosphor), copied as they are, since `cargo-about`
  sees crates alone and its notices would leave both out; and
  `THIRD-PARTY-NOTICES.txt`, the license notices of every crate the exe links and of egui's
  bundled fonts, whose licenses ask that their notices travel with every copy. In a folder,
  not beside the exe, because members open the exe and the readme and nothing else, so the
  top folder keeps to those; the repository keeps its two license files at its root, where the
  convention and GitHub's license detection look. The notices are generated at each release
  from `cargo-about`'s JSON (installed on the release machine, not a crate the exe links),
  with the licenses it accepts read from `deny.toml`'s allowlist, so the two cannot drift; a
  hand-kept list would miss a dependency `Cargo.lock` gained. Each license appears once: its
  text, then every crate under it with the copyright lines its own copy states, which are
  what the licenses ask to keep (cargo-about's own listing repeats MIT once per copyright
  line, 117 times for 0.0.1). The workspace's crates are left out, the two license files
  covering them. About 70 KB, 18 KB of the archive

`just release <version>` (`scripts/release/release.py`, Windows only) builds the exe with `--locked`,
stages those files under one top folder, `unlimited-tool-works-<version>/`, so "Extract here"
makes one folder rather than scattering the files, and packs it with 7-Zip as
`unlimited-tool-works-<version>-windows.7z` in the `dist/` folder of cargo's target directory
(asked of `cargo metadata`: a developer may have moved it), beside its checksum,
`<archive>.sha256`, in `sha256sum`'s format (`<hex>  <archive name>`), which
`sha256sum -c` and PowerShell's `Get-FileHash` both check. It refuses a dirty tree, a version
other than the workspace's, an archive that already exists (a release is never overwritten)
and a readme whose first line names another version, and it ends by listing the archive and
comparing the listing with the staged files, so a 7-Zip that skipped the empty folders fails
the recipe. It takes the executable's name from the built exe's own `--help` usage line and
checks that each launcher it ships calls it (none in 0.0.1), so the executable's name lives in the code's constant
and the recipe cannot drift from it; the package name `unlimited-tool-works` is the
recipe's own constant, since no built file states it. Beside the archive it keeps the release's symbols, never in the
bundle: the built exe and its `.pdb`, packed as `unlimited-tool-works-<version>-symbols.7z`,
which `scripts/release/symbolize_log.py` reads to name a posted `utw.log`'s backtrace
(`architecture.md` "Diagnostic logging"). The release profile builds with
`debug = "line-tables-only"`, so the kept `.pdb` names each frame's file and line as well as
its function; the line tables live in the `.pdb`, leaving the exe's size as it was.

The exe carries no path of the machine that built it. A panic's location names its source
file, so a plain release build held the builder's home folder in about 2,300 strings (every
dependency under the cargo registry, the standard library under rustup) and the repository's
absolute path in one more (a build script's generated file). The recipe builds with rustc's
`--remap-path-prefix`: the home folder becomes `~`, `CARGO_HOME` and `RUSTUP_HOME`, when set,
`~cargo` and `~rustup`, the repository `.`, and cargo's target directory, when outside it,
`target`, in that order, since the last prefix that matches wins (measured on rustc 1.98.1).
The flags go to cargo through `--config` on `target.x86_64-pc-windows-msvc.rustflags`, which
joins the array `.cargo/config.toml` sets there; `RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS`
would replace it, and with it `+crt-static`, leaving an exe that needs the VC++
redistributable (`spec/core/distribution.md` "Platform floor"). After the build the recipe refuses an exe holding any of
those prefixes, or naming `VCRUNTIME140.dll`. Cargo's own `trim-paths` profile setting does
the same with no paths to name, but it is unstable in cargo 1.98. The `.pdb` keeps its paths
(the linker's command line, the object files'): it stays with the maintainer. Its source paths
are remapped like the exe's, which `symbolize_log.py` prints as they are.

The user extracts the archive anywhere and runs the binary — or, for the common case, drops an
export into `exports/` (shipped empty beside the binary) and double-clicks
`quick_compile.bat`, which is the whole Red workflow carried over. Where the suite's persisted data goes
(the settings file, state like the skipped-update version, logs) is the user's choice, asked once on
first run — see "Data location" below. This is the same folder model Red uses, minus the ~80MB
embedded Python runtime that made Red's updates a folder-replacement operation.

### Data location: asked on first run

What exists of this is in `spec/core/distribution.md` "Data location: asked on first run".

Some users want a fully portable install, others want their data to survive replacing or moving the
program folder. Following EGG-Translator's first-start wizard, the first run asks where to store
settings and data, with two options (each with a one-line hint, like EGG's buttons).

On a first run the GUI shows the dialog and writes the settings file to the chosen location — the
file's existence is what makes the choice stick. The dialog has no Cancel: the choice is needed
to start. A location that cannot be written (a program folder unpacked under `Program Files`,
picked as "Next to the program") keeps the dialog open with the error under the buttons, so the
member picks the other: a choice that never reached the disk would be asked again at every
start, and the session's settings would vanish with it. The portable file shadowing a
config-dir choice, which EGG renames to `.bak`, cannot happen here: the dialog shows only when
neither file exists, and the program offers no later move of the data location (moving the
folder is the move).

**The first run also finds the PES version and folder.** Below the location choice, the dialog
offers the versions found on the machine: those whose save folder exists
(`Documents\KONAMI\Pro Evolution Soccer 20xx`, the folders the save discovery already walks)
and those whose `PES20xx.exe` sits in the default install path
(`C:\Program Files (x86)\Pro Evolution Soccer 20xx`), the newest preselected for
`pes_version`, and for `pes_folder_path` the default `**` path when the exe is found there.
The member confirms or changes the pick; with nothing found, the defaults stand (PES 19, the
default path), as they do today. Without it a new member's first compile goes to PES 19 and a
folder that may not exist, and only "Fix PES path" from a running PES corrects it. Steam's
library folders are not probed: a Steam install is not the cup's (its launch is incompatible
with the mods, `plans/team_compiler/gui.md` "Toolbar"), so a found one would be the wrong
default.

One more thing is not data-dir cargo:

- The `old/` rollback binary is tied to the program folder, not the data.

The question is desktop-only: the browser build has no exe directory and persists to browser
storage.

### Self-update: in-place binary swap

Red updates by downloading the new release's 7z, extracting it as a **sibling folder**, and
migrating user data into it (settings via a transfer table, `teams_list.txt` via an interactive
diff, `exports/` contents, state files) — necessary because Red's install folder is a pile of
engine files that must be replaced wholesale, with user data interleaved. Unlimited Tool Works' folder is
a few files, so the update inverts: **the folder stays, the binary is swapped in place**, and there
is almost nothing to migrate.

The desktop flow is coordinated by `studio` (the binary crate). `studio_core::updater` provides generic
release checking, download/verification, and binary replacement; `studio` separately calls
`teams_list`'s parse/reconcile logic for teams-list reconciliation. Core knows neither
teams-list/team-ID semantics nor team-format types. No plugin hook or new updater framework is
needed. This is desktop-only (`#[cfg(not(target_arch = "wasm32"))]`); the browser build updates
server-side. 0.0.1 ships steps 1, 2, 3, 4 and 6, in the window only: *Update now* has to be in
the version updated from, or a 0.0.1 never updates itself. 0.0.2 adds the command line's notice
and rollback (the rollback lives in the newer exe); step 5 and the `update` subcommand come with
the Team compiler. The HTTP client is `ureq` pinned to `=3.5.0` (default features off,
`rustls-no-provider` and `platform-verifier`, `rustls` on `ring` handed to it as the crypto
provider, the system's certificate store, no gzip, a `User-Agent`): `rustls-no-provider` may
change in a minor release, hence the exact pin.

1. **Check** (startup, a plain background thread): if `check_for_updates` is on and
   `last_update_check` is older than `check_interval_hours`, read the repository's list of
   GitHub releases and take the highest `v<version>` tag by semver, pre-releases included
   (`releases/latest` skips a release marked Pre-release, which a 0.0.x may be; a tag that is
   not `v` and a semver is passed over), and compare it with the built-in
   `CARGO_PKG_VERSION`. The release's `body` (its notes) comes in the same response. A
   completed check stores `last_update_check`; a failed one (offline, rate-limited, a body
   that does not parse) shows nothing and writes one line to the log.
2. **Offer**: a newer version that isn't `skipped_version` becomes the status bar's
   `update_available` condition, `Update available: v<version>` in the link blue, its text and
   its **Details** button both opening the dialog; nothing opens by itself. The dialog says
   "Unlimited Tool Works v<new> is out. You have v<own>.", shows the notes as Markdown in a
   scrolling box (`egui_commonmark`, as the help window), and offers **Update now**, **Open
   release page**, **Skip this version** (stores `skipped_version`, drops the condition) and
   **Remind me later** (closes; the next check offers it again). Turning checks off is the
   settings' `check_for_updates`, kept out of the dialog (Red's `fuckoff`). From 0.0.2 the
   command line prints a one-line notice when a newer release exists; a `UTW update`
   subcommand that checks and applies non-interactively comes later.
3. **Download + verify**: fetch the release's archive for the running platform (the Windows
   `.7z`, the Linux `.tar.gz`: the artifacts members download by hand, one serving both paths)
   into a temp folder on the executable's volume, with a progress dialog (`Downloading v<new>…
   <done> of <total> MB`, a bar, Cancel), verify its SHA-256 against the release's
   `SHA256SUMS.txt` asset (CI writes it), and extract it (`sevenz-rust2` for the `.7z`;
   `flate2` and the `tar` crate for the `.tar.gz`). Any failure (the network, the checksum, the
   archive) changes nothing and shows `Update failed` with the reason, **Retry**, **Open release
   page** and **Close**. The download **streams** the
   response body (`ureq`'s `into_reader()`) into the temp file, hashing and counting bytes as it
   goes — that is where the dialog's progress comes from, and it sidesteps `ureq`'s 10 MB default
   in-memory body limit, which the convenience readers (`read_to_vec`/`read_to_string`/`read_json`)
   enforce and a release asset exceeds (`into_reader()` is unbounded; `into_with_config().limit()`
   changes either). Both HTTP calls run on a plain background thread.
4. **Swap**: first stage and verify the new bundle on the executable's volume. Move the running
   binary into `old/`, then put the new binary in place, and every other file of the bundle
   but `data/` (the readme, the licenses, the launchers a later bundle adds) over the old ones;
   if replacement fails, restore the original.
   This is a recoverable sequence, not one atomic swap; handle file locks and permissions explicitly.
   If the folder isn't writable (e.g. Program Files), use the `elevation` lib's elevated-relaunch
   path or download-and-instruct. Restart only after active work is stopped and unsaved edits are
   handled by the ordinary close prompts.
5. **Teams list reconciliation** (on the first start of the new binary, not during the swap): the
   new binary carries the new upstream list embedded; the working `data/teams_list.txt` may contain
   user-added ID assignments (the grid's inline ID-cell writes — see the [Team compiler
   plan](../team_compiler/README.md)), so it can't just be overwritten. The `studio` coordinator calls
   `teams_list`'s reconciliation, not the generic core updater, to **merge** instead
   of showing Red's pick-one diff: rows only in the embedded list are added, rows only in the
   working list (user additions) are kept, and conflicting rows (same team, different ID) take the
   embedded list's value. The result is presented as a reviewable summary (added / kept /
   overridden) before being written. Validate uniqueness of both team names and IDs before
   publication: a new team's ID may collide with a retained local team's ID even when their keys
   differ. A change behind an unresolved conflict is dropped and reported for review, the rest
   applied (the
   noninteractive CLI does not guess). An unwritable data directory skips the merge with
   `teams_list_read_only`; the embedded list is not used in place of the working one, so the
   user's assignments keep winning until they move the install. The readme and
   `quick_compile.bat` are overwritten without ceremony (not user-edited).
6. **Restart**: `Update ready` says "v<new> is installed. Restart to start using it, the version
   you are running now will stay in the old folder." with **Restart now** (the ordinary close
   prompts, then the new exe starts and shows `Updated to v<new>` with What's new) and **Later**
   (the next start runs the new exe). The settings file needs no transfer — it stays where it is, and
   the existing merge of `default_settings()` for missing keys (see `spec/core/gui.md` "Settings menu") absorbs
   new-version settings changes, which is what retired Red's transfer-table mechanism.

### Rollback

`old/UTW.exe` is kept (one version deep — each update replaces it), and the settings menu gets a
**"Roll back to previous version"** button that swaps the two binaries back and relaunches. This
covers the "new version is broken" case that Red handles by preserving the entire old folder.

### Changelog and version display

**One changelog, three readers.** `CHANGELOG.md` at the repository root, in the Keep a Changelog
shape: `## [Unreleased]` on top, then one `## [1.4.0] — 2026-10-02` section per release; inside a
section, bullets grouped by tool (`**Team compiler** — …`, `**Save editor** — …`, `**General** —
…` for what every tool shares), because that is how a member reads it even though the version axis is one. It is written in
the step that changes the behavior, like the help topic (see `spec/core/gui.md` "Help window"), from the
first release on: an entry goes under `[Unreleased]` in the commit that changes something a
member notices against the last release (a fix for a bug a release shipped, a new or changed
behavior, a speed-up worth telling). A fix for something no release has shipped gets none, so
the file itself tells the two kinds of fix apart and commit types do not have to. Nothing else
describes releases; the three places a user meets release notes all read this file:

1. **The help window's "What's new" chapter** — the file embedded at build time, newest first.
2. **The GitHub release body** — the release process copies the released version's section
   verbatim into the release notes. The updater dialog shows that body inline (see "Distribution
   and updates" step 2), so the dialog, the release page and the in-app chapter are the same text
   by construction rather than by discipline.
3. **The post-update notice** — `Updated to v1.4.0 · [What's new]` in the status bar on the first
   start of a new version, opening reader 1 (see `spec/core/gui.md` "Status bar").

**One version string, shown in one place.** `v{CARGO_PKG_VERSION}`, then the release stage
while there is one: ` pre-alpha` for 0.0.x (the previews: a few tools, for one cup's members),
` alpha` from 0.1.0 (the test releases for a restricted group of testers), nothing from 1.0.0;
then ` (debug)` when `cfg!(debug_assertions)`, enough to tell a local build from a release,
which is the only distinction that has ever mattered in a bug report. The stage is derived from
the version's major and minor numbers in one function, not written beside it, so a version bump
can never leave a stale word; the semver itself carries no pre-release suffix (`0.0.1`, not
`0.0.1-pre-alpha`), which would change the release tags and the updater's version comparison
for a label. A beta stage, if the maintainer opens one, is a ruling added here. No git hash: it would need a `build.rs` or
`vergen` for information the tag already carries, since releases are built from tags. Where it
appears:

- **Status bar empty state** — the one always-visible place, and the line to paste into a report.
- **About** (logo click) — the detailed block with Copy (see `gui.md` "Sidebar" item 1).
- **`UTW --version`** — the same one-liner on stdout (CLI equivalence); built,
  `spec/core/distribution.md` "Versioning: one workspace version, one exception".
- **Not the title bar**: the title is what the taskbar and alt-tab show — the surface for what
  the Studio is *doing* while the window is not in front (see `gui.md` "Window title"), and it is outside
  every screenshot a user crops to the window content. **Not beside the logo**: duplicate of the
  status bar, and homeless on the icon rail, which is all of the sidebar most of the time.

---
