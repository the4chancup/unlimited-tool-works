# 4cc Studio — Core plan: Distribution and updates

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Distribution and updates

### Distribution: portable .7z bundle

What exists of this is in `spec/core/distribution.md` "Distribution: portable .7z bundle".

There is no installer. Each release is a `.7z` archive containing:

- `UTW.exe` (or the `utw` Linux binary) — the single self-contained binary
  (`development_plan.md` "The project's name: Unlimited Tool Works")
- `quick_compile.bat` (`quick_compile.sh` on Linux) — the double-click launcher: opens the GUI on
  the Team compiler and starts a compile (see `architecture.md` "Launch modes" under Architecture)
- in the 0.0.1 preview only, until the two tools' views land: `quick_compile.bat` in its CLI
  form, `quick_upgrade.bat` and an empty `old_exports/` (`development_plan.md` "The /vg/
  League 27 (VGL)"); the launchers' sources are `resources/bundle/`, checked out with CRLF
  line ends (`.gitattributes`), since cmd misreads a batch file's labels with LF ones
- a basic readme (pointing into the app: the full help and the changelog live in the help window
  — see `gui.md` "Help window" under GUI Design — and `4cc-studio help --export` produces the same text as
  files for the release page), with the two license files beside it; `README.txt`, not
  `.md`, because a double-click opens a `.txt` in Notepad on any Windows, where a `.md` opens
  whatever app claimed it, or asks

`just release <version>` (`scripts/release.py`, Windows only) builds the exe with `--locked`,
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
checks that both launchers call it, so the executable's name lives in the code's constant
and the recipe cannot drift from it; the package name `unlimited-tool-works` is the
recipe's own constant, since no built file states it. Beside the archive it keeps the release's symbols, never in the
bundle: the built exe and its `.pdb`, packed as `unlimited-tool-works-<version>-symbols.7z`,
which `scripts/symbolize_log.py` reads to name a posted `utw.log`'s backtrace
(`architecture.md` "Diagnostic logging"). The release profile builds with
`debug = "line-tables-only"`, so the kept `.pdb` names each frame's file and line as well as
its function; the line tables live in the `.pdb`, leaving the exe's size as it was.

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
engine files that must be replaced wholesale, with user data interleaved. 4cc Studio's folder is
a few files, so the update inverts: **the folder stays, the binary is swapped in place**, and there
is almost nothing to migrate.

The desktop flow is coordinated by `studio` (the binary crate). `studio_core::updater` provides generic
release checking, download/verification, and binary replacement; `studio` separately calls
`teams_list`'s parse/reconcile logic for teams-list reconciliation. Core knows neither
teams-list/team-ID semantics nor team-format types. No plugin hook or new updater framework is
needed. This is desktop-only (`#[cfg(not(target_arch = "wasm32"))]`); the browser build updates
server-side. Release 0.2.0 ships step 1, step 2 as a notice (release notes and a link to the
release page, no *Update now*) and step 5; steps 3, 4, 6, the `update` subcommand and rollback
arrive in Phase 16:

1. **Check** (startup, non-blocking background task): if `check_for_updates` is on and the last
   check is older than the check interval, query the GitHub Releases API (`releases/latest`),
   compare the `tag_name` semver against the built-in `CARGO_PKG_VERSION`. The release `body`
   (release notes) comes back in the same response.
2. **Offer**: if a newer version exists and it isn't the `skipped_version`, show a dialog with the
   release notes inline and the options *Update now* / *Skip this version* / *Remind me later*; a
   *Disable update checks* option lives in settings (kept out of the dialog — Red's `fuckoff`). The
   CLI equivalent is a one-line notice plus a `4cc-studio update` subcommand that performs the
   check+apply non-interactively.
3. **Download + verify**: fetch the release's `.7z` asset (the same artifact users download manually
   — one artifact serves both paths) into a temp folder, verify its SHA256 against a checksum asset,
   and extract it with the already-in-workspace `sevenz-rust2`. The download **streams** the
   response body (`ureq`'s `into_reader()`) into the temp file, hashing and counting bytes as it
   goes — that is where the dialog's progress comes from, and it sidesteps `ureq`'s 10 MB default
   in-memory body limit, which the convenience readers (`read_to_vec`/`read_to_string`/`read_json`)
   enforce and a release asset exceeds (`into_reader()` is unbounded; `into_with_config().limit()`
   changes either). Both HTTP calls run on a plain background thread.
4. **Swap**: first stage and verify the new binary on the executable's volume. Move the running
   binary into `old/`, then put the new binary in place; if replacement fails, restore the original.
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
6. **Restart**: offer to relaunch. The settings file needs no transfer — it stays where it is, and
   the existing merge of `default_settings()` for missing keys (see `spec/core/gui.md` "Settings menu") absorbs
   new-version settings changes, which is what retired Red's transfer-table mechanism.

### Rollback

`old/4cc-studio.exe` is kept (one version deep — each update replaces it), and the settings menu gets a
**"Roll back to previous version"** button that swaps the two binaries back and relaunches. This
covers the "new version is broken" case that Red handles by preserving the entire old folder.

### Changelog and version display

**One changelog, three readers.** `CHANGELOG.md` at the repository root, in the Keep a Changelog
shape: `## [Unreleased]` on top, then one `## [1.4.0] — 2026-10-02` section per release; inside a
section, bullets grouped by tool (`**Team compiler** — …`, `**Save editor** — …`, `**Studio** —
…`), because that is how a member reads it even though the version axis is one. It is written in
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
- **`4cc-studio --version`** — the same one-liner on stdout (CLI equivalence); built,
  `spec/core/distribution.md` "Versioning: one workspace version, one exception".
- **Not the title bar**: the title is what the taskbar and alt-tab show — the surface for what
  the Studio is *doing* while the window is not in front (see `gui.md` "Window title"), and it is outside
  every screenshot a user crops to the window content. **Not beside the logo**: duplicate of the
  status bar, and homeless on the icon rail, which is all of the sidebar most of the time.

---
