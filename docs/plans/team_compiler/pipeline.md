# Unlimited Tool Works — Team compiler plan: Pipeline

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Per-export pipeline walkthrough

### 1. Reader

What exists of this is in `spec/team_compiler/pipeline.md` "1. Reader".

In **Discovery** (step 1): In the GUI this scan is continuous (folder watcher, see
`plans/team_compiler/gui.md` "Live validation"); in the CLI it runs once. A disabled export is
a greyed row of the grid with **Enable**, never compiled (`plans/team_compiler/gui.md` "Row
actions").

Also in **Discovery**: a folder whose name ends in `.upgrading` (any letter case) is skipped
with a Warning, `export_upgrade_unfinished`, naming it.

Such a folder is the Export upgrader's staging folder (`spec/export_upgrader.md` "CLI"), which
an upgrade killed part way (the console closed, Ctrl+C) leaves beside the output it was
writing. Upgrading into `exports/`, as `quick_upgrade.bat` does, puts it where the compiler
looks, and its first word is the team's, so without the skip a half-migrated export would
compile, or refuse its team's finished export as a duplicate.

The skip is a Warning, not the Info `export_disabled` gets, because the folder is a failure the
member has to clear (remove it, then upgrade the old export again; the upgrader refuses while
it exists, EU-CLI-05), not a choice they made. The skipped folder takes no part in duplicate
detection or planning, as a disabled export does.

In **Load** (step 4): the eagerly loaded structure is also what the grid needs.

### 2. Per-export serial steps (coordinator)

What exists of this is in `spec/team_compiler/pipeline.md` "2. Per-export serial steps (coordinator)".

In **Validation, in two passes** (step 2), after the deep pass: Plain-folder full checks run both
passes. Packed archives run the structure pass during live checking, and this deferred deep
pass at compile start, before eligibility is decided.

Among the categories the two passes cover:

- **Shared folders** (`Faces/`, `Boots/`, `Gloves/`): same model checks, including glTF-only
  material files (`materials.toml` catch-all and `*.materials.toml` name-matched; see the
  [Unified model format plan](../model_format.md)'s "Material files").
- Among the **Root files**, `ref_lists.txt` is the Refs arranger's file for the Fox referee
  hook, and a missing referee roster's invalid `ParsedAestheticsExport` remains available to
  the Refs arranger for repair.
- Under **Portraits/Logo/Common/Collars**: Collar findings map to the team-name cell (collars
  have no cell of their own).

In **Export identity resolution** (step 3): The GUI also performs an early lookup after
structural validation so the live grid can show or edit the ID immediately. An unresolvable
normal team turns the grid's **team ID cell** red and editable: the user types an ID, confirms
with Enter, and it is written to the teams list (see "GUI: the live-validating team grid").

In **Player folder categorization** (step 6), assigned IDs reach the save through the
aesthetics patch, so a compiled team CPK and its savefile must be deployed together. The
aesthetics patch written with the CPK is how they travel together when the savefile is on
someone else's machine (see "Post-processing"). The deploy-together rule is distribution
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

- **Degraded run: promotion to `output/`** — the GUI offers **Retry** (for the locked case:
  the user closes PES and retries the deployment alone, no recompile) and **Open output
  folder** (a status-bar notice action after the run, and a permanent entry in the Compile
  dropdown).
- **DpFileList upgrade** — with any of the preflight's DpFileList findings the GUI shows
  **Upgrade DpFileList**, which asks before it writes. The dialog is honest about the override:
  it lists every installed entry that the official list does not contain and that is not
  renamed as "will no longer be loaded", and shows every rename before anything is done.

- **DpFileList upgrade** — for each entry the dialog lists as "will no longer be loaded" whose
  `.cpk` still sits in `download/`, it shows the size and offers deletion, since renaming a
  3.7 GB file as a backup is pointless. The user confirms per file, and nothing is deleted
  without that confirmation.
- **Destination writability preflight** — the two failure modes users actually hit are
  detectable long before Compile:
  - the download folder needs elevation because PES sits under `Program Files`;
  - the old CPK is locked because PES is still running.

  So they are checked **live**, like the version selector's exe check, not discovered at the
  end of a multi-minute run.

  The probe's failure kind is read from the OS error: `ERROR_ACCESS_DENIED` → elevation needed,
  `ERROR_SHARING_VIOLATION` → in use (cross-checked against the running-PES probe,
  `spec/core/architecture.md` "Crate structure", which names the culprit). The check runs at
  startup, on window focus, whenever a relevant setting changes (`pes_folder_path`,
  `pes_version`, CPK names, `multicpk_mode`), and whenever the PES process poll changes state.

  Results show on the Compile button as a warning badge with the reason in the tooltip and a
  line in the run strip: *"PES 2019 is running — close it before compiling"* / *"The download
  folder needs administrator rights"* with a **Relaunch as administrator** action (the
  `elevation` lib).

  The button stays enabled, since the user may close PES during the compile. The exception is
  the elevation case: compiling first would waste the run (relaunching loses it), so the click
  prompts for elevation up front.
- **The savefile in the preflight** — the save's header is read with the destination preflight
  (live in the GUI, before any export is read in the CLI), so a save of another version or a
  damaged one (`savefile_invalid`) is reported before minutes of compiling rather than rolling
  the deployment back at the end. The end-of-run read stays as it is, since the file can change
  between the two. With `savefile_path = auto` the preflight rarely finds anything, which costs
  one header read.
- **Run PES** — optional launch of `PES20{version}.exe`, only after the complete deployment
  transaction succeeds. Built in Phase 8 together with sideload mode's Launch PES button, as one
  launcher: both start the same exe the same way, and the GUI phase is where its only manual
  check lives.

### Legacy boots and gloves IDs (VGL27)

VGL27's managers keep the old export format and Red for their own test compiles, against the
official DLC's save. Red packs a boots or gloves folder under the ID its name carries
(`k0430 - Crocs` is boots 430), each team's IDs inside the 25-ID block the cup gave it once
for good. Studio's ID allocation ("2. Per-export serial steps", the 40-ID per-team blocks in
`spec/aesthetics_export/player_folders.md`) gives the same model another ID, so a manager's
test CPK and the official save would disagree on every custom boots and gloves ID.

The organizer upgrades every VGL27 export in the upgrader's legacy mode (`--legacy-ids`,
`plans/export_upgrader.md` "What it does"), midcups included, and the compiler packs and
patches the old IDs of an export so upgraded. Managers change nothing: they keep sending Red
exports, so the experiment is invisible to them.

VGL28 runs on Studio's IDs, and the mode, the file and this section are then removed. The mode
is temporary, so it changes as little as it can: faces and kits need nothing (both tools name
them by team ID and slot), stock-band IDs (1 to 100) are untouched, and nothing else in the
compile reads the file.

- **The old IDs** come from the export's root file `legacy_ids.toml`, which only a legacy
  upgrade writes: per player slot, the custom boots and gloves IDs (above 100) his folders
  were packed under in Red.
  - The file's presence is the switch, with no compiler setting: only the organizer's legacy
    upgrades carry it, and a second switch would be one more step per compile to forget.
  - It is a known root file, so validation does not warn about it.
  - A file that does not parse is an Error (`legacy_ids_invalid`), the file ignored and the
    export compiled on Studio's IDs.
  - It is one table per team rather than keys in each `settings.toml`, so the file members
    edit carries nothing that leaves with VGL27.
- **Packing.** A legacy upgrade merges each worn boots or gloves folder into its wearers'
  folders, so mostly a player folder's own packages change: a slot whose table records an ID
  of a kind has that package emitted under the old ID instead of his exclusive one, and his
  patch row carries it.

  Slots that shared a Red folder recorded its one ID and hold byte-identical copies. So each
  old ID is packed once per team, by the first slot in roster order recording it, and every
  other slot recording it takes its row from that package (gated on its task, as a linked
  shared folder's row is).

  A slot whose own package of a kind has no recorded ID (a model under a stock ID, a
  hand-made folder) is not packed for that kind, and the patch leaves his ID of that kind as
  the save holds it. There is no finding, because every VGL27 export goes through the
  upgrader, so a missing ID is no manager's mistake, and a slot ID could land in another
  team's old block.

  A shared folder the upgrade kept (its merge clashed with a wearer's file) packs under the
  old ID its first plain linker in roster order records for its kind, whether it would take a
  shared ID or that slot's exclusive one. Its wearers' rows find that package as above. A
  shared folder no linker records an ID for packs on Studio's IDs as usual.

  FoxDen's per-kit boots and gloves packages are not written for these IDs: old exports have
  none.
- **Out of block** (poc13e, in the PoC: the VGL27 Friendlies' /hanny/ export packs /gfg/'s
  IDs, and the two `k2351` paths stopped the compile on a Fatal). An old ID outside its
  team's block in `legacy_boots_blocks.txt`, or any old ID of a team the table does not
  list, is an Error (`legacy_id_out_of_block`, naming the ID, the folder and the block, or
  "none"): a folder packed under another team's ID puts its model over that team's.

  The check is on what the export packs, not on what players wear: that folder is not packed
  under that ID, but its wearers' patch entries still carry it (a player may wear another
  team's boots, as a manager's save sets it), and the folder's other IDs are packed. The
  Error names the folder that would have packed the ID, once: a player folder that only
  links a kept-shared folder packs nothing of that kind, so the shared folder alone is
  named, not each wearer.

  The table lives in the data directory (beside `teams_list.txt`, created from the embedded
  copy the first time a legacy export is compiled). It has two tab-separated columns, `Name`
  and `FirstBootsID`, one row per VGL team. A block is `FirstBootsID` to
  `FirstBootsID + 24`, for boots and gloves alike. The embedded copy is the wiki's PES 2019
  boots table from /2hug/ on, with /feg/'s and /uma/'s rows from the organizer. A team is
  found by its teams-list name, letter case aside. The real GBFG VGL26 export's `g2522` and
  `g2523` are such IDs (its block is 2301 to 2325, /ksg/'s 2501 to 2525).

### Recovery manifest

A team's aesthetics export is sometimes lost, and the cup's DLC is then the only copy of its
look: the files carry the team's ID, but the export's player, model and kit folder names are
gone (a player folder's name survives in the save only sometimes).

So the writer packs a folder per compiled export into the CPK,
`4cc/exports/<team id>/<export folder name>/`, a path no game version reads: every version
ignores files it does not look up, as the 7 KB placeholder CPKs holding only `placeholder.txt`
show. The Team decompiler (`plans/team_decompiler.md`) reads it back.

- **`manifest.toml`**: the export's folder name, the Studio version and the PES version it
  was compiled for; every file the export packed, as its CPK path, the source path relative
  to the export folder, and the conversion it took (passthrough, rewrap, a texture encoded
  with a named codec, a model converted, a portrait converted).
- **The export's TOML files, as files**: `settings.toml`, each kit's `config.toml`, each model
  folder's `materials.toml`, at their paths relative to the export folder, beside the
  manifest. They are packed as files, not embedded in the manifest as strings, because their
  bytes then go in and come out as they are (comments, blank lines, line endings), where an
  embedded copy needs escaping on the way in and out, and a member can take one out with any
  CPK tool.

  The `settings.toml` matters beyond the folder names: the savefile settings reach the game
  only through the aesthetics patch, so without it the save is the only place they survive.
- **The replica it allows is the compiled one, not the sources**: a decompiled export
  recompiles to the same CPK. A file packed as it was comes back exact; a converted one comes
  back in its compiled form under its original name, which the next compile passes through.
  We do not embed sources (an uncompressed texture before encoding, a glTF before conversion):
  they would grow every member's DLC download for a file only an organizer recovering a team
  ever needs. Leftovers the upgrader dropped are not recorded either: they never reached a
  player.
- **No hashes, no sizes, no times, no absolute paths**: the decompiler hashes the CPK's own
  bytes if it needs to, and has no source to check a hash against otherwise. A time or a path
  would make two compiles of one export differ and could show a member's user name.
- Built early, ahead of the decompiler, because only DLCs compiled after it ships carry it, so
  every cup compiled without it is one more that can only be recovered by the decompiler's
  heuristic.

### Resolved decisions and open questions

What exists of this is in `spec/team_compiler/pipeline.md` "Resolved decisions and open questions".

Resolved decisions:

- **Teams list: embedded upstream, one working copy in the data directory.** `data/teams_list.txt`
  is edited by the grid's ID cell and by the updater's merge (see the core plan's
  "Self-update"). We do this, not Red's loose file beside the exe, because the grid writes into
  the list, which makes it user data: it must follow the settings file into the user config
  directory when that mode is chosen, or a replaced program folder loses the user's ID
  assignments. And a single on-disk file leaves nothing for a user to edit by mistake.

  **Writes are best-effort, never elevated**: when the data directory is not writable (the
  typical case is a portable install unpacked under Program Files), the ID cell stays
  read-only with the reason in its tooltip and the updater's merge is reported as skipped
  (`teams_list_read_only`). Studio is a portable app, and elevating for a one-line edit is
  handling a corner case nobody should be in. Detection is by attempting the write, not by
  inspecting the path: there is no list of protected directories to keep right.
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
- **`teams_list.txt` contract.** Still open: atomic writes and concurrency between the grid's
  ID-cell write, the updater merge and the savefile import. Callers own I/O. The one writer so
  far, `compile`, only creates a missing file, so the question opens with the grid's ID-cell
  write in Phase 8 and the update merge of Release 0.1.0.
- **Run-result semantics.** Define separate compile and deployment outcome types so scoped errors,
  `pass_through`, failed moves, and failed savefile writes map predictably to GUI state and CLI exit
  codes. The exit codes are fixed (`spec/team_compiler/settings.md` "CLI"): deployment adds
  none, a degraded run exits 1 and an aborted one 3. What remains is the outcome types for GUI
  state.

  Deployment is transactional across generated CPKs and the savefile, with backups and
  rollback. PES launches only after the complete transaction succeeds. (No system CPK is part
  of it: the Fox referee marker is a reserved collar inside the refs CPK,
  `spec/team_compiler/blue_port.md` "Referee export processing".)
- **Superseded installed outputs.** Within the teams slot run this is solved: every slot is written
  each multi-CPK run (content or the empty placeholder), so stale parts cannot survive. What
  remains:
  - the single-CPK-versus-multi-CPK crossover: a `4cc_99_test` left from a test compile is not
    touched by a DLC compile, and vice versa (decide whether that needs a message);
  - the rule that an unrecognized user file is never deleted.

  Retired official names are handled by the DpFileList upgrade's confirmed per-file deletion.
