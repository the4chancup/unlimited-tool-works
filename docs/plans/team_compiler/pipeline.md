# Unlimited Tool Works — Team compiler plan: Pipeline

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Per-export pipeline walkthrough

### 1. Reader

What exists of this is in `spec/team_compiler/pipeline.md` "1. Reader".

In **Discovery** (step 1): In the GUI this scan is
continuous (folder watcher, see `plans/team_compiler/gui.md` "Live validation"); in the CLI it runs once.
A disabled export is a greyed row of the grid with **Enable**, never compiled
(`plans/team_compiler/gui.md` "Row actions").

Also in **Discovery**: a folder whose name ends in `.upgrading` (any letter case) is skipped
with a Warning, `export_upgrade_unfinished`, naming it: it is the Export upgrader's staging
folder (`spec/export_upgrader.md` "CLI"), which an upgrade killed part way (the console
closed, Ctrl+C) leaves beside the output it was writing. Upgrading into `exports/`, as
`quick_upgrade.bat` does, puts it where the compiler looks, and its first word is the team's,
so without the skip a half-migrated export would compile, or refuse its team's finished
export as a duplicate. A Warning, not the Info `export_disabled` gets: the folder is a
failure the member has to clear (remove it, then upgrade the old export again; the upgrader
refuses while it exists, EU-CLI-05), not a choice they made. The skipped folder takes no
part in duplicate detection or planning, as a disabled export does.

In **Load** (step 4): the eagerly loaded structure is also what the grid needs.

### 2. Per-export serial steps (coordinator)

What exists of this is in `spec/team_compiler/pipeline.md` "2. Per-export serial steps (coordinator)".

In **Validation, in two passes** (step 2), after the deep pass: Plain-folder full checks run both
passes; packed archives run the structure pass during live
checking and this deferred deep pass at compile start before eligibility is decided. Among the
categories the two passes cover, **Shared folders** (`Faces/`, `Boots/`, `Gloves/`): same model
checks, including glTF-only
material files (`materials.toml` catch-all and `*.materials.toml` name-matched; see the
[Unified model format plan](../model_format.md)'s "Material files"). Among the **Root files**,
`ref_lists.txt` is the Refs arranger's file for the Fox referee hook, and a missing referee
roster's invalid `ParsedAestheticsExport` remains available to the Refs arranger for repair.
Under **Portraits/Logo/Common/Collars**: Collar findings map to the team-name cell (collars
have no cell of their own).

In **Export identity resolution** (step 3): The GUI also performs an early lookup after
structural validation so the live grid can show or edit the ID immediately. An unresolvable
normal team turns the grid's **team ID cell** red and editable — the user types an ID, confirms
with Enter, and it is written to the teams list (see "GUI: the live-validating team grid").

In **Player folder categorization** (step 6), assigned IDs are written on pre-Fox into the
savefile, so a compiled team CPK and its savefile must be deployed
together, and the aesthetics patch written with the CPK is how they travel together when the
savefile is on someone else's machine (see "Post-processing"). The deploy-together rule is distribution
guidance, not a compile blocker.

### 3. Per-model-folder parallel steps (rayon)

What exists of this is in `spec/team_compiler/pipeline.md` "3. Per-model-folder parallel steps (rayon)".

In **Format conversion** (step 1): duplicate sources within the selected representation are
ambiguous and drop the folder (`model_source_ambiguous`).

### 5. Writer (single-threaded)

What exists of this is in `spec/team_compiler/pipeline.md` "5. Writer (single-threaded)".

Under **Output modes and targets** (step 5), sideload mode: for PES 15/16, where no sideloading
runtime exists, the GUI disables the mode.

### 6. Post-processing

What exists of this is in `spec/team_compiler/pipeline.md` "6. Post-processing".

- **Degraded run: promotion to `output/`** — the GUI offers
  **Retry** (for the locked case — the user closes PES and retries the deployment alone, no
  recompile) and **Open output folder** (a status-bar notice action after the run, and a
  permanent entry in the Compile dropdown).
- **DpFileList upgrade** — with any of the preflight's DpFileList findings the GUI shows
  **Upgrade DpFileList**, which asks before it writes. The dialog is honest about the override:
  it lists every installed entry that the official list does not contain and that is not
  renamed as "will no longer be loaded", and shows every rename before anything is done.

- **DpFileList upgrade** — for each entry the dialog lists as "will no longer be loaded" whose
  `.cpk` still sits in `download/`, it shows the size and offers deletion (renaming a 3.7 GB file
  as a backup is pointless; the user confirms per file, and nothing is deleted without that
  confirmation).
- **Destination writability preflight** — the two failure modes users actually hit (the download
  folder needs elevation because PES sits under `Program Files`; the old CPK is locked because PES
  is still running) are detectable long before Compile, so they are checked **live**, like the
  version selector's exe check, not discovered at the end of a multi-minute run. The probe's failure
  kind is read from the OS error — `ERROR_ACCESS_DENIED` → elevation needed, `ERROR_SHARING_VIOLATION`
  → in use (cross-checked against the running-PES probe, `spec/core/architecture.md` "Crate
  structure", which names the culprit). The
  check runs at startup, on window focus, whenever a relevant setting changes (`pes_folder_path`,
  `pes_version`, CPK names, `multicpk_mode`), and whenever the PES process poll changes
  state. Results show on the Compile button as a warning badge with the reason in the tooltip and a
  line in the run strip: *"PES 2019 is running — close it before compiling"* / *"The download
  folder needs administrator rights"* with a **Relaunch as administrator** action (the `elevation`
  lib). The button stays enabled — the user may close PES during the compile — except for the
  elevation case, where compiling first would waste the run (relaunching loses it), so the click
  prompts for elevation up front.
- **The savefile in the preflight** — the save's header is read with the destination preflight
  (live in the GUI, before any export is read in the CLI), so a save of another version or a
  damaged one (`savefile_invalid`) is reported before minutes of compiling rather than rolling
  the deployment back at the end. The end-of-run read stays as it is, since the file can change
  between the two; with `savefile_path = auto` the preflight rarely finds anything, which costs
  one header read.
- **Aesthetics patch** — on **pre-Fox** it holds everything (names only from a `write_names`
  export): settings.toml settings with
  `name = true` and FPC markers resolved to concrete values, the authored stock boots/gloves IDs
  (else the marker's default) for categories requesting no standalone output, and the
  auto-assigned boots/gloves IDs **only for content that was actually packed**: if a boots/gloves task failed, the affected players
  keep their existing savefile IDs rather than pointing at absent CPK content, while their
  independent valid settings still apply.
- **Run PES** — optional launch of `PES20{version}.exe`, only after the complete deployment
  transaction succeeds. Built in Phase 8 together with sideload mode's Launch PES button, as one
  launcher: both start the same exe the same way, and the GUI phase is where its only manual
  check lives.

### Resolved decisions and open questions

What exists of this is in `spec/team_compiler/pipeline.md` "Resolved decisions and open questions".

Resolved decisions:

- **Teams list: embedded upstream, one working copy in the data directory.** `data/teams_list.txt`
  is edited by the grid's ID cell and by the updater's merge (see the core plan's "Self-update"). We do this,
  not Red's loose file beside the exe, because the grid writes into the list, which makes it user
  data: it must follow the settings file into the user config directory when that mode is chosen,
  or a replaced program folder loses the user's ID assignments; and a single on-disk file leaves
  nothing for a user to edit by mistake. **Writes are best-effort, never
  elevated**: when the data directory is not writable (the typical case is a portable install
  unpacked under Program Files), the ID cell stays read-only with the reason in its tooltip and
  the updater's merge is reported as skipped (`teams_list_read_only`) — Studio is a portable app,
  and elevating for a one-line edit is handling a corner case nobody should be in. Detection is by
  attempting the write, not by inspecting the path: there is no list of protected directories to
  keep right.
- **Check-cache identity**: recursive manifest fingerprint combined with teams-list and
  validation-settings revisions; the watcher only triggers re-fingerprinting (see "Live
  validation").
- **Shallow archive checks never decompress solid data**: `.zip` per-entry access may read small
  metadata; solid-`.7z` content checks defer to the compile-start deep check, with the roster
  recorded as unread.
- **Model source selection**: in-representation duplicates are rejected (`model_source_ambiguous`).
- **Source snapshot**: The GUI watcher (Phase 8) keys its stale-result envelopes on
  the same value.

Open questions — **implementation/spec work** (no user preference involved; resolved during the
phase that owns them):

- **Shared/Common dependency graph.** Define cache ownership when a player task needs shared or
  Common models for a Fox merge while those folders also have independent output work (each
  player task reads and converts its parts itself today; the ordering of Fox's Common textures is
  settled above).
- **`teams_list.txt` contract.** Still open: atomic writes
  and concurrency between the grid's ID-cell write, the updater merge and the savefile import
  (callers own I/O; the one writer so far, `compile`, only creates a missing file, so the
  question opens with the grid's ID-cell write in Phase 8 and the update merge of Release 0.1.0).
- **Run-result semantics.** Define separate compile and deployment outcome types so scoped errors,
  `pass_through`, failed moves, and failed savefile writes map predictably to GUI state and CLI exit
  codes. The exit codes are fixed (`spec/team_compiler/settings.md` "CLI"): deployment adds none, a degraded run
  exits 1 and an aborted one 3; what remains is the outcome types for GUI state. Deployment is
  transactional across generated CPKs and the savefile, with backups and rollback; PES launches
  only after the complete transaction succeeds. (No system CPK is part of it: the Fox referee
  marker is a reserved collar inside the refs CPK, `spec/team_compiler/blue_port.md` "Referee export processing".)
- **Superseded installed outputs.** Within the teams slot run this is solved: every slot is written
  each multi-CPK run (content or the empty placeholder), so stale parts cannot survive. What remains
  is the single-CPK-versus-multi-CPK crossover (a `4cc_99_test` left from a test compile is not
  touched by a DLC compile, and vice versa — decide whether that needs a message) and the rule that
  an unrecognized user file is never deleted; retired official names are handled by the DpFileList
  upgrade's confirmed per-file deletion.
