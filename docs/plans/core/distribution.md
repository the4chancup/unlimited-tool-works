# Unlimited Tool Works: Core plan: Distribution and updates

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
  (`development_plan.md` "The /vg/ League 27 (VGL)"). The launchers' sources are
  `resources/bundle/`, checked out with CRLF line ends (`.gitattributes`), because cmd misreads
  a batch file's labels with LF ones
- a basic readme, `README.txt`, pointing into the app: the full help and the changelog live in
  the help window (see `gui.md` "Help window" under GUI Design), and `UTW help --export` produces
  the same text as files for the release page. It is a `.txt`, not a `.md`, because a
  double-click opens a `.txt` in Notepad on any Windows, where a `.md` opens whatever app
  claimed it, or asks
- a `licenses/` folder:
  - the project's `LICENSE-MIT` and `LICENSE-APACHE`;
  - the licenses of the two third-party works the suite bundles itself rather than through a
    crate, Roboto Flex's `RobotoFlex-OFL.txt` (`resources/fonts/`) and `PHOSPHOR-LICENSE.txt`
    (`resources/icons/`, for the icon font's outlines copied from Phosphor), copied as they
    are, because `cargo-about` sees crates alone and its notices would leave both out;
  - `THIRD-PARTY-NOTICES.txt`, the license notices of every crate the exe links and of egui's
    bundled fonts, whose licenses ask that their notices travel with every copy.

  The licenses go in a folder, not beside the exe, because members open the exe and the readme
  and nothing else, so the top folder keeps to those. The repository keeps its two license
  files at its root, where the convention and GitHub's license detection look.

  The notices are generated at each release from `cargo-about`'s JSON (installed on the
  release machine, not a crate the exe links), with the licenses it accepts read from
  `deny.toml`'s allowlist, so the two cannot drift. A hand-kept list would miss a dependency
  `Cargo.lock` gained. Each license appears once: its text, then every crate under it with the
  copyright lines its own copy states, which are what the licenses ask to keep (cargo-about's
  own listing repeats MIT once per copyright line, 117 times for 0.0.1). The workspace's crates
  are left out, because the two license files cover them. About 70 KB, 18 KB of the archive

`just release <version>` (`scripts/release/release.py`, Windows only) builds the exe with
`--locked` and stages those files under one top folder, `unlimited-tool-works-<version>/`, so
"Extract here" makes one folder rather than scattering the files. It packs that folder with
7-Zip as `unlimited-tool-works-<version>-windows.7z` in the `dist/` folder of cargo's target
directory (asked of `cargo metadata`, because a developer may have moved it). Beside the
archive goes its checksum, `<archive>.sha256`, in `sha256sum`'s format
(`<hex>  <archive name>`), which `sha256sum -c` and PowerShell's `Get-FileHash` both check.

The recipe refuses:

- a dirty tree;
- a version other than the workspace's;
- an archive that already exists (a release is never overwritten);
- a readme whose first line names another version.

It ends by listing the archive and comparing the listing with the staged files, so a 7-Zip that
skipped the empty folders fails the recipe.

It takes the executable's name from the built exe's own `--help` usage line and checks that
each launcher it ships calls it (none in 0.0.1). So the executable's name lives in the code's
constant, and the recipe cannot drift from it. The package name `unlimited-tool-works` is the
recipe's own constant, because no built file states it.

Beside the archive it keeps the release's symbols, never in the bundle: the built exe and its
`.pdb`, packed as `unlimited-tool-works-<version>-symbols.7z`. `scripts/release/symbolize_log.py`
reads that archive to name a posted `utw.log`'s backtrace (`architecture.md` "Diagnostic
logging"). The release profile builds with `debug = "line-tables-only"`, so the kept `.pdb`
names each frame's file and line as well as its function. The line tables live in the `.pdb`,
which leaves the exe's size as it was.

The exe carries no path of the machine that built it. A panic's location names its source
file, so a plain release build held the builder's home folder in about 2,300 strings (every
dependency under the cargo registry, the standard library under rustup) and the repository's
absolute path in one more (a build script's generated file).

The recipe builds with rustc's `--remap-path-prefix`, in this order, because the last prefix
that matches wins (measured on rustc 1.98.1):

1. the home folder becomes `~`;
2. `CARGO_HOME` and `RUSTUP_HOME`, when set, become `~cargo` and `~rustup`;
3. the repository becomes `.`;
4. cargo's target directory, when outside the repository, becomes `target`.

The flags go to cargo through `--config` on `target.x86_64-pc-windows-msvc.rustflags`, which
joins the array `.cargo/config.toml` sets there. `RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS`
would replace that array, and with it `+crt-static`, leaving an exe that needs the VC++
redistributable (`spec/core/distribution.md` "Platform floor"). After the build the recipe
refuses an exe holding any of those prefixes, or naming `VCRUNTIME140.dll`. Cargo's own
`trim-paths` profile setting does the same with no paths to name, but it is unstable in cargo
1.98.

The `.pdb` keeps its paths (the linker's command line, the object files'), because it stays
with the maintainer. Its source paths are remapped like the exe's, which `symbolize_log.py`
prints as they are.

**Linux's bundle** holds the same files with `utw` for `UTW.exe` (the name its `--help` usage
line prints), packed as `unlimited-tool-works-<version>-linux.tar.gz`, not a `.7z`. A tar
keeps the binary's executable bit, which `.7z` extraction tools often drop, and a Linux member
unpacks a `.tar.gz` with what the system already has.

`release.py` packs it on Linux with Python's `tarfile`: the binary `0755`, every other file
`0644`, each entry owned by uid and gid 0 with no user or group name, so the archive names
nobody. It lists the archive back and compares the listing with the staged set, as the `.7z`
path does.

Its notices come from the Linux binary's own dependency graph (`cargo-about` run for
`x86_64-unknown-linux-gnu`), because the two platforms link different crates. The remap flags
go on `target.x86_64-unknown-linux-gnu.rustflags`, and the static C runtime check is Windows'
alone (a Linux binary links the system's glibc).

The built binary is 269 MB, 207 of it debug info, because `line-tables-only` still links in the
standard library's full debug info, which a Windows exe leaves in its `.pdb` (measured on the
first dry run's binary). So the recipe moves the debug info out as Windows' symbols are:

1. `objcopy --only-keep-debug` writes `utw.debug`;
2. `objcopy --strip-debug --add-gnu-debuglink=utw.debug` strips the shipped binary and names
   `utw.debug` in it, keeping the symbol table;
3. `utw.debug` is packed as `unlimited-tool-works-<version>-linux-symbols.tar.gz`, never in the
   bundle.

A crash's backtrace then names its functions but not its lines, which `utw.debug` gives
(`addr2line`). The binary drops to 60 MB, the bundle from 66.6 MB to 21.9 MB. Stripping the
symbols too saves 1.7 MB more and loses the names, and `.tar.xz` would pack 15.4 MB but needs
an xz decoder in the updater.

The readme is the one file, with a Linux line beside its `UTW.exe`. The binary is built on
Ubuntu 22.04 (glibc 2.35), the oldest GitHub runner image, so it runs on that release and any
newer distribution.

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
    against ("Self-update" step 3).

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
whose version it outranks (the check reads pre-releases). Update now is therefore tested end to
end (ST-UPD-04) against releases no member's check reads. The environment variable
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

What exists of this is in `spec/core/distribution.md` "Data location: asked on first run".

Some users want a fully portable install, others want their data to survive replacing or moving the
program folder. Following EGG-Translator's first-start wizard, the first run asks where to store
settings and data, with two options (each with a one-line hint, like EGG's buttons).

On a first run the GUI shows the dialog and writes the settings file to the chosen location.
The file's existence is what makes the choice stick. The dialog has no Cancel, because the
choice is needed to start.

A location that cannot be written (a program folder unpacked under `Program Files`, picked as
"Next to the program") keeps the dialog open with the error under the buttons, so the member
picks the other. It stays open because a choice that never reached the disk would be asked
again at every start, and the session's settings would vanish with it.

The portable file shadowing a config-dir choice, which EGG renames to `.bak`, cannot happen
here: the dialog shows only when neither file exists, and the program offers no later move of
the data location (moving the folder is the move).

**The settings file is written whole, commented.** The bundle ships `data/settings.toml`
(`resources/bundle/data/settings.toml`) with every setting at its default, the bundle's
presets aside (DBC's `pes_version` and `aatf_ruleset`). A comment above each setting says what
it does and which values it takes, so a member sets the PES folder in a text editor and runs
`quick_compile.bat` without opening the window.

The binary embeds the same text. A settings file written fresh (the first-run dialog's choice,
or a file that does not parse) starts from it, its values replaced by the session's, so it
carries the comments too. A save edits the file in place, so the comments survive every later
save.

The comments are written for members, not taken from the code's doc comments, which speak to
developers. A test of the `studio` crate keeps the file's keys equal to the registered tools'
defaults and its values equal to them but for the presets.

**The first run also finds the PES version and folder.** Below the location choice, the dialog
offers the versions found on the machine:

- those whose save folder exists (`Documents\KONAMI\Pro Evolution Soccer 20xx`, the folders the
  save discovery already walks);
- those whose `PES20xx.exe` sits in the default install path
  (`C:\Program Files (x86)\Pro Evolution Soccer 20xx`).

The newest is preselected for `pes_version`, and for `pes_folder_path` the default install path
when the exe is found there. The member confirms or changes the pick. With nothing found, the
defaults stand (PES 19, the default path), as they do today.

Without this search a new member's first compile goes to PES 19 and a folder that may not
exist, and only "Fix PES path" from a running PES corrects it. Steam's library folders are not
probed: a Steam install is not the cup's (its launch is incompatible with the mods,
`plans/team_compiler/gui.md` "Toolbar"), so a found one would be the wrong default.

One more thing is not data-dir cargo:

- The `old/` rollback binary is tied to the program folder, not the data.

The question is desktop-only: the browser build has no exe directory and persists to browser
storage.

### Self-update: in-place binary swap

Red updates by downloading the new release's 7z, extracting it as a **sibling folder**, and
migrating user data into it (settings via a transfer table, `teams_list.txt` via an interactive
diff, `exports/` contents, state files). Red needs this because its install folder is a pile of
engine files that must be replaced wholesale, with user data interleaved. Unlimited Tool Works'
folder is a few files, so the update inverts: **the folder stays, the binary is swapped in
place**, and there is almost nothing to migrate.

The desktop flow is coordinated by `studio` (the binary crate). `studio_core::updater` provides
generic release checking, download/verification, and binary replacement. `studio` separately
calls `teams_list`'s parse/reconcile logic for teams-list reconciliation. Core knows neither
teams-list/team-ID semantics nor team-format types. No plugin hook or new updater framework is
needed. This is desktop-only (`#[cfg(not(target_arch = "wasm32"))]`); the browser build updates
server-side.

0.0.1 ships steps 1 and 2, in the window only, its dialog without **Update now** (a disabled
one reads as an update that is not there), so a 0.0.1 member downloads 0.0.2 from the release
page, once. 0.0.2 adds steps 3, 4 and 6 (ST-UPD-03, ST-UPD-04) and the command line's notice.
Step 5 and the `update` subcommand come with the Team compiler.

The HTTP client is `ureq` pinned to `=3.5.0`, with default features off, `rustls-no-provider`
and `platform-verifier`, `rustls` on `ring` handed to it as the crypto provider, the system's
certificate store, no gzip and a `User-Agent`. The pin is exact because `rustls-no-provider`
may change in a minor release.

1. **Check** (at startup, and again while the window stays open, on a plain background
   thread): if `check_for_updates` is on and `last_update_check` is older than
   `check_interval_minutes` (120 by default, at least 1), read the repository's list of
   GitHub releases and take the highest `v<version>` tag by semver, pre-releases included.
   Then compare it with the built-in `CARGO_PKG_VERSION`.

   Pre-releases are included: `releases/latest` skips a release marked Pre-release, which a
   0.0.x may be. A tag that is not `v` and a semver is passed over. The release's
   `body` (its notes) comes in the same response.

   A completed check stores `last_update_check`. A failed one (offline, rate-limited, a body
   that does not parse) shows nothing and writes one line to the log.
2. **Offer**: a newer version that isn't `skipped_version` becomes the status bar's
   `update_available` condition, `Update available: v<version>` in the link blue, its text and
   its **Details** button both opening the dialog. Nothing opens by itself.

   The dialog says "Unlimited Tool Works v<new> is out. You have v<own>." and shows the notes as
   Markdown in a scrolling box (`egui_commonmark`, as the help window). It offers:
   - **Update now**;
   - **Open release page**;
   - **Skip this version**, which stores `skipped_version` and drops the condition;
   - **Remind me later**, which closes the dialog; the next check offers it again.

   Turning checks off is the settings' `check_for_updates`, kept out of the dialog (Red's
   `fuckoff`). A `UTW update` subcommand that checks and applies non-interactively comes later.

   **The command line's notice.** From 0.0.2 a tool command (`UTW <tool> <command>`, the
   launchers' runs among them) looks for a newer release too, when `check_for_updates` is
   on: a member who only double-clicks the launchers never sees the status bar.

   - The check starts on a thread of its own as the command starts, and the command never
     waits for it. A compile or an upgrade outlasts the fetch; a quick `check` may not, and
     then prints nothing.
   - When the command ends, a reply that has arrived and offers a release (newer than the
     program's own version and not `skipped_version`) prints one line on stderr, after every
     other line of the run: `Unlimited Tool Works v<new> is out (you have v<own>). Start the
     program and click Update available to get it.` The line goes to the run's log file too
     (`architecture.md` "A run's log file").
   - No reply yet, a failed check and a release that is not newer print nothing. A failed
     check writes one debug log line.
   - The check stores nothing. The command line writes no settings, so `last_update_check`
     stays the window's, and every command checks, whatever the interval.
   - `UTW help` and a command line refused before its tool runs check nothing. The exit code
     is the tool's, whatever the check did.
3. **Download + verify**:
   1. fetch the release's archive for the running platform (the Windows `.7z`, the Linux
      `.tar.gz`: the artifacts members download by hand, one serving both paths) into a temp
      folder on the executable's volume, with a progress dialog (`Downloading v<new>…
      <done> of <total> MB`, a bar, Cancel);
   2. verify its SHA-256 against the release's `SHA256SUMS.txt` asset (CI writes it);
   3. extract it (`sevenz-rust2` for the `.7z`; `flate2` and the `tar` crate for the `.tar.gz`).

   Any failure (the network, the checksum, the archive, a folder that cannot be written to)
   changes nothing. It shows `Update failed` with the reason (the updater's own sentence for
   it), then "Nothing was changed. You can download v<new> from the release page instead.", and
   **Retry**, **Open release page** and **Close**. Cancel is no failure: it returns to the
   offer.

   While the download runs the dialog stays: the backdrop and Escape do nothing, because a
   stray click must neither cancel a download nor hide one that is running. A window closed
   then cancels the download and waits for the updater's thread, so a swap is never cut midway.

   The download **streams** the response body (`ureq`'s `into_reader()`) into the temp file,
   hashing and counting bytes as it goes. That is where the dialog's progress comes from. It
   also sidesteps `ureq`'s 10 MB default in-memory body limit, which the convenience readers
   (`read_to_vec`/`read_to_string`/`read_json`) enforce and a release asset exceeds
   (`into_reader()` is unbounded; `into_with_config().limit()` sets the limit of either kind of
   reader). Both HTTP calls run on a plain background thread.
4. **Swap**: first stage and verify the new bundle on the executable's volume. Move the running
   binary into `old/`, then put the new binary in place, and every other file of the bundle
   but `data/` (the readme, the licenses, the launchers a later bundle adds) over the old ones.
   If replacement fails, restore the original.

   This is a recoverable sequence, not one atomic swap. When putting the old files back fails
   too:
   - the files set aside stay in `.update/backup` inside the program's folder;
   - the failure names that folder (and does not say nothing was changed);
   - no later update starts until the member has put them back or removed the folder.

   A folder that isn't writable (e.g. Program Files) is found before the download starts and is
   a failure like the others, the release page its way out. There is no elevated relaunch,
   because the bundle is a folder the member unpacks wherever they can write.

   Restart only after active work is stopped and unsaved edits are handled by the ordinary
   close prompts.
5. **Teams list reconciliation** (on the first start of the new binary, not during the swap): the
   new binary carries the new upstream list embedded. The working `data/teams_list.txt` may
   contain user-added ID assignments (the grid's inline ID-cell writes — see the [Team compiler
   plan](../team_compiler/README.md)), so it can't just be overwritten.

   The `studio` coordinator calls `teams_list`'s reconciliation, not the generic core updater,
   to **merge** instead of showing Red's pick-one diff:
   - rows only in the embedded list are added;
   - rows only in the working list (user additions) are kept;
   - conflicting rows (same team, different ID) take the embedded list's value.

   The result is presented as a reviewable summary (added / kept / overridden) before being
   written. Validate uniqueness of both team names and IDs before publication, because a new
   team's ID may collide with a retained local team's ID even when their keys differ. A change
   behind an unresolved conflict is dropped and reported for review, the rest applied (the
   noninteractive CLI does not guess).

   An unwritable data directory skips the merge with `teams_list_read_only`. The embedded list
   is not used in place of the working one, so the user's assignments keep winning until they
   move the install.

   The readme and `quick_compile.bat` are overwritten without ceremony (not user-edited).
6. **Restart**: `Update ready` says "v<new> is installed. Restart to start using it, the version
   you are running now will stay in the old folder." It offers **Restart now** (the ordinary
   close prompts, then the new exe starts and shows `Updated to v<new>` with What's new) and
   **Later** (the next start runs the new exe).

   Once the update is installed the status bar's offer reads `Update ready: v<new>`, its
   tooltip "<app> v<new> is installed. Restart to start using it.", and its text and Details
   open `Update ready` again. No check runs for the rest of the session, and Update now is not
   offered twice, because a second one would move the new program into `old/`, over the version
   to roll back to.

   Restart now asks the window to close as its close button does. When the close goes through,
   the program at the bundle's own path, now the new one, is started. A close that a tool's
   prompt holds drops the restart.

   A start whose version is newer than the one that last ran shows `Updated to v<new>`. The
   version that last ran is `last_run_version`, state the shell writes as it does `last_tool`;
   it is absent on a fresh install and after 0.0.1, which then show no notice.

   The settings file needs no transfer: it stays where it is, and the existing merge of
   `default_settings()` for missing keys (see `spec/core/gui.md` "Settings menu") absorbs
   new-version settings changes, which is what retired Red's transfer-table mechanism.

### Rollback

`old/UTW.exe` is kept (one version deep — each update replaces it). This covers the "new
version is broken" case that Red handles by preserving the entire old folder: the member
closes the program and copies the program file from `old/` over the one in the bundle's
folder.

A settings menu button that does the swap and relaunches is a proposal, not planned work
(`PROPOSALS.md` P51), and is not built while it is unruled.

### Changelog and version display

**One changelog, three readers.** `CHANGELOG.md` at the repository root, in the Keep a Changelog
shape: `## [Unreleased]` on top, then one `## [1.4.0] — 2026-10-02` section per release. Inside a
section, bullets are grouped by tool (`**Team compiler** — …`, `**Save editor** — …`,
`**General** — …` for what every tool shares), because that is how a member reads it even
though the version axis is one.

A fix for a corner case few members ever reach (a rare sequence of steps, such as a popover
left open while a page key moves the selection elsewhere) goes in the section's closing
`### Minor fixes` list, in the same per-tool bullets, not among the headline bullets. A member
reads the top of a section for what changed in their own use, and a list led by corner cases
buries it. A fix for a crash, a loss of data or a wrong file written stays among the headline
bullets however rare its case, because the member it reached needs to find it. Every reader
below shows the `###` heading as written, so the help window's renderer handles it.

The changelog is written in the step that changes the behavior, like the help topic (see
`spec/core/gui.md` "Help window"), from the first release on. An entry goes under
`[Unreleased]` in the commit that changes something a member notices against the last release:

- a fix for a bug a release shipped;
- a new or changed behavior;
- a speed-up worth telling.

A fix for something no release has shipped gets none, so the file itself tells the two kinds of
fix apart and commit types do not have to.

A tool no release has presented (named in its notes or in the `README.md`) gets no entry either,
even when its code ships in the bundle. The Team compiler and the Export upgrader were in
0.0.1's binary but in neither, so no member uses them yet, and a list of changes to a tool
nobody has been told about reads as noise. The release that first presents a tool describes it
whole, in one bullet, as 0.0.1 did the Save editor.

Nothing else describes releases; the three places a user meets release notes all read this
file:

1. **The help window's "What's new" chapter**: the file embedded at build time, newest first,
   without its title line: the chapter's one topic is titled "Changelog" already, and the
   window would draw the same heading twice.
2. **The GitHub release body**: the release process copies the released version's section
   verbatim into the release notes. The updater dialog shows that body inline (see "Distribution
   and updates" step 2), so the dialog, the release page and the in-app chapter are the same text
   by construction rather than by discipline.
3. **The post-update notice**: `Updated to v1.4.0 · [What's new]` in the status bar on the first
   start of a new version, opening reader 1 (see `spec/core/gui.md` "Status bar").

**One version string, shown in one place.** `v{CARGO_PKG_VERSION}`, then the release stage
while there is one:

- ` pre-alpha` for 0.0.x (the previews: a few tools, for one cup's members);
- ` alpha` from 0.1.0 (the test releases for a restricted group of testers);
- nothing from 1.0.0.

Then comes ` (debug)` when `cfg!(debug_assertions)`, enough to tell a local build from a
release, which is the only distinction that has ever mattered in a bug report.

The stage is derived from the version's major and minor numbers in one function, not written
beside it, so a version bump can never leave a stale word. The semver itself carries no
pre-release suffix (`0.0.1`, not `0.0.1-pre-alpha`), because a suffix would change the release
tags and the updater's version comparison for a label. A beta stage, if the maintainer opens
one, is a ruling added here.

No git hash: it would need a `build.rs` or `vergen` for information the tag already carries,
since releases are built from tags.

Where the version string appears:

- **Status bar empty state**: the one always-visible place, and the line to paste into a report.
- **About** (logo click) — the detailed block with Copy (see `gui.md` "Sidebar" item 1).
- **`UTW --version`**: the same one-liner on stdout (CLI equivalence); built,
  `spec/core/distribution.md` "Versioning: one workspace version, one exception".
- **Not the title bar**: the title is what the taskbar and alt-tab show, the surface for what
  the Studio is *doing* while the window is not in front (see `gui.md` "Window title"), and it
  is outside every screenshot a user crops to the window content.
- **Not beside the logo**: duplicate of the status bar, and homeless on the icon rail, which is
  all of the sidebar most of the time.

---
