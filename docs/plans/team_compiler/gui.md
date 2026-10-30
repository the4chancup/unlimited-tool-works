# Unlimited Tool Works — Team compiler plan: GUI grid

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

Part of the [Team compiler plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## GUI: the live-validating team grid

The primary view is a grid where **each aesthetics export is a row**, keyed internally by stable
`ExportId` rather than display name, with the team name label on the left, the team ID cell next to
it, one cell per player at a fixed position by number, plus one cell per kit:

```
/aa/   [714]   [01][02][03][04][  ][  ][  ][  ]   [GK][K1][K2]
/bb/   [???]   [01][02][03][04][05][06][07][08]   [GK][K1]
/cc/   [809]   [01][  ][03][  ][05][  ][  ][  ]   [GK]
```

- **Team name label**: fixed-width left column with the canonical `/xx/` `team_name`; the tooltip
  shows the complete `export_display_name` and resolved identity. The label doubles as the **cell
  for export-scoped and non-slot task messages**: shared model folders, Logo, Common, Collars,
  RefMarker, and root-artifact scopes have no cells of their own, so their status and messages roll
  up here. For example, an orphaned shared folder (`shared_folder_orphaned`) turns the label yellow,
  with the details in its tooltip.
- **Team ID cell**: between the team name and the first player column, showing the ID resolved from
  `teams_list.txt`. When the team is not on the teams list (`team_name_unknown`), the cell turns
  **red and editable**: the user types an ID and confirms with Enter, and the ID is **written to the
  teams list** — the successor to Red's interactive prompt. The entry is validated on confirm:
  numeric and in the 701–920 range (violations keep the cell red with the reason in the tooltip). An
  ID **already assigned to another team** asks for confirmation on the cell ("in use by {team};
  reassign?") before overwriting the old entry — the same overwrite flow Red's prompt offered, kept
  because its purpose is correcting a stale teams list. The teams-list write is picked up like any
  file change: every export re-resolves and revalidates, so a reassignment's effect on other rows
  (the team that lost the ID turning red) is immediately visible. If the write fails because the
  data directory is not writable, the cell reverts and stays red with `teams_list_read_only` in
  its tooltip; there is no elevation path (see `spec/team_compiler/pipeline.md` "Resolved decisions", "Teams list").
- **Untagged export**: a team export whose name has neither `Full` nor `Midcup`
  (`export_tag_missing`) is not compiled and has no player to show, so its row holds two
  buttons, **Full** and **Midcup**, where its player cells would be: no space is reserved for
  them on other rows. A click renames the export's folder or archive, putting the tag right
  after the team name, and the row revalidates like after any rename.
- **Player cells**: one per player, labeled with the player number (`01`–`23`, from the folder name
  or the root `players.txt`). The columns are **fixed positions 01–23**: a player without a folder
  in the export leaves its column completely empty (no cell, no number), so columns always align
  across rows by number — missing players are visible at a glance.
- **Referee rows**: the fixed positions extend to `01`–`35` (one per referee slot); each slot listed
  in `players.txt` gets a cell, and slots backed by the same model folder share its status (the
  tooltip names the folder, making the repetition visible).
- **Kit cells**: one per kit — the `Kits/` folder has one subfolder per kit (containing
  `config.toml`, `colors.txt`, and that kit's textures), which gives the per-kit cell granularity
  naturally; `all/` is not a kit and gets no cell — its effect shows in the kit cells' tooltips,
  which name the folder (`p1 - Lakers`) and the stems inherited from `all/`. Kit cells use a
  separate fixed-slot loop (`GK`, then `K1`–`K9`) following the same
  reserve-every-position rule as player cells; they use the same square shape and are separated from
  the player group by a small gap. Color carries status; the label carries identity. Once a kit's
  menu colors are known — its `colors.txt` parses at check time, or the compile derives them from
  the texture — the cell additionally shows its **menu icon's pattern** (the `icon_<N>` marker's pattern
  rendered by `color_tools` in pattern-only mode — a flat two-color swatch, no shirt silhouette,
  which wouldn't read at cell size) drawn centered under the label on the status background. A
  failed kit keeps its error state and shows the existing base-`UniColor.bin` colors rather than the
  unwritten pair. This is the kit colors' proof-of-life: a row of identical all-black miniatures
  exposes a lazy set of `colors.txt` files at a glance, and a derived-color miniature shows exactly
  what the extraction fallback picked before anything ships.
- **Task-to-cell mapping**: player model and portrait tasks update every mapped player/referee slot
  cell; kit tasks update their kit cell; Logo/Common/Collar/RefMarker/root-artifact work updates the
  team/ref row label. Run/output failures appear in the run strip and log, with run-abort state
  propagated to pending cells as described below.
- **Hover**: tooltip with folder name + warning/error text. **Click**: opens a detail panel (and
  later, the 3D preview from Future Features).
- One row per team bounds the row count (~50), so no virtualization is needed; direct egui rendering
  suffices.

### Row actions

What a member does about a row, from the row itself. Every action that writes into the
exports folder is a rename, a marker or a copy the member asks for by a click; nothing is
changed on its own.

- **Open folder.** The detail panel and a log line's context menu carry **Open folder**,
  opening the scope's folder in the file manager, a file's folder with the file selected
  (`explorer /select,` on Windows, `xdg-open` on the folder elsewhere); for an archive export,
  the archive's folder with the archive selected. The fix for a finding is nearly always a
  rename, a link or a removal in the file manager, so the click saves finding the folder by
  hand.
- **One-click fixes for the mechanical cases.** A finding with one obvious fix gets a button
  in the detail panel that applies it, the member still choosing: a known file with a hidden
  `.txt` (`settings.toml.txt` → `settings.toml`; kept disallowed, so the mistake is named), an
  old `Portraits/player_05.dds` (→ the player folder's portrait name), a player folder
  `15 Snuffy` or `15_Snuffy` (→ `15 - Snuffy`). Each is a rename refused when the target name
  exists (the refusal says so), and the row revalidates as after any rename. A folder export
  only: inside an archive nothing is renamed, so an archive's finding has no button. It widens
  the untagged row's **Full**/**Midcup** buttons, the precedent of the GUI renaming in an
  export.
- **An old export offers its upgrade.** A row red with `export_layout_old` or
  `player_layout_proto` carries an **Upgrade** button: the shell switches to the Export
  upgrader with the export as its input (a tool switch with an argument, as the Refs arranger's
  handoff). After a clean upgrade, the upgrader asks once whether to move the old export into a
  sibling `old exports/` folder, so its red row goes; declined, the old export stays where it
  is. The upgrader's "Move to exports and compile" (`plans/export_upgrader.md` "GUI") is the
  handoff back.
- **The empty grid.** An exports folder with no export (`no_exports_found`) shows, in the
  grid's place, the folder's path, an **Open exports folder** button and a line saying an
  export can be dropped here. A folder, `.zip` or `.7z` dropped anywhere on the grid is copied
  into the exports folder (refused, naming it, when that name exists) and checked like any new
  export; the drop works on a filled grid too.
- **Compile one export, or a chosen few.** A row's context menu has **Compile only this
  export**, the CLI's `--export` from the grid, in the current output mode. A **compile
  column** of checkboxes at the start of each row, ticked by default, picks the exports the
  next Compile takes: it is this run's selection only, written nowhere, every row ticked again
  on the next start, and an unticked row is greyed and skipped as if absent. The column is
  hidden by default, shown and hidden by a toggle in the Compile button's dropdown; hidden, every
  row is compiled. Not the `NO_USE` marker: a checkbox that wrote into the export would change
  files on a click meant for one run.
- **Save findings open the Save editor at their fix.** `savefile_missing` (the patch written,
  no save to write it into) carries **Apply in Save editor**: the shell switches to the Save
  editor with that patch loaded and its apply action ready, the member picking the save. It is
  the tool switch with an argument the Upgrade button uses; the save is written by the Save
  editor's own apply, the one save-writing path (`savefile_player_not_stripped` goes with the
  stripped route, worklog 8.36c). `savefile_skipped_pes_running` gets no button:
  the view asks the Save editor's Edit mode question (`plans/save_editor.md` "Writing a save
  while its game runs") in place of the command line's `--edit-mode-closed`
  (`spec/team_compiler/pipeline.md` "6. Post-processing"), its Save running the savefile step
  as the flag does; the question is drafted before it is built. Until then the window's
  Compile runs without the flag, and the skip's catalog text, which names the flag, is the
  command line's.
- **Disabled exports stay visible.** An export with its `NO_USE` marker is a greyed row with
  **Enable** (removing the marker) rather than left out of the grid, so what was disabled can
  be seen and undone. Where two exports clash (`duplicate_aesthetics_export`,
  `multiple_ref_exports`, which skip every export involved), each clashing folder export's row
  offers **Disable** (writing its `NO_USE` marker), and an archive offers a move into
  `old exports/` instead, since no marker can be written inside it; disabling one clears the
  clash and the other compiles.

---

## Live validation (no Check or Refresh buttons)

The exports folder is monitored continuously; checking requires no user input. The watcher and
the cache below are `libs/pipeline`'s, shared with the Balls compiler's list view; this crate's
`check/` decides what to check when and maps the results to rows (`README.md` "Crate layout"):

- **Folder watching** via the `notify` crate (`ReadDirectoryChangesW` on Windows, inotify on Linux),
  with `notify-debouncer-full` for event debouncing. New exports trigger checks after a ~1–2s quiet
  period plus a file-size stability check (so half-copied archives aren't parsed). Deletions and
  renames update the grid immediately.
- **Parallel checking**: each triggered check is an independent task on the rayon pool — never on
  the UI thread — and multiple exports check concurrently (e.g. on first launch over a full exports
  folder, or when several exports are dropped in at once). Within an export, per-scope checks fan
  out in parallel too. Results arrive as `Message`/`FolderStatus` events mapped to each scope's
  slot, kit, or row-label cell; cells flip from `checking` as each scope finishes, not per export.
  Events carry the run and export revision they were computed from, and the receiver rejects results
  older than the row's current revision, so a stale check can never overwrite newer state.
- **Deep check (plain-folder exports)**: full parse + validation of every file. Rust makes this
  near-instant for folders. When compiling a full DLC, users normally submit plain folders, so the
  main workflow gets full validation automatically. **Deep means every check whose inputs exist
  before a compile**: the export's own files, the teams list, the settings and the installed
  CPKs the compile would read. A finding the compile reports from those same inputs is a gap
  in the deep check, not a compile-time finding: a cell that looks fully checked and turns red
  only at Compile tells the member nothing until minutes later. `fmdl_texture_not_found` is
  the first such case: the deep check runs the same texture lookup for each source model's own
  texture names (one function both callers use, never a copy, with a test pair keeping them in
  agreement). Only a finding about something the compile itself produces (a merged model's
  combined texture list, a retargeted model, a converted model over its limit) stays with the
  compile, and its catalog row says why.
- **Shallow check (packed archives)**: archive extraction (LZMA) is the real bottleneck, not CPU —
  so `.zip`/`.7z` exports get a shallow check only: the archive's directory listing is read (file
  names, sizes, structure) without extraction, catching naming/nesting/file-type issues in
  milliseconds. Shallow checks never decompress solid data: a `.zip`'s per-entry random access lets
  the structure pass also read small root metadata (`players.txt`, kit `config.toml`) cheaply, while
  a solid `.7z` yields only its entry table — its content-dependent checks (roster syntax, config
  parsing) defer to the compile-start deep check, and the parse result records the roster as
  *unread* rather than missing. **Shallow-passed cells are blue instead of purple**, indicating the
  export may still contain issues that only a deep check would find.
- **Compile-start deep check for archives**: compilation must materialize archive contents, so at
  Compile their deferred deep check runs before the process-or-skip decision. ZIP entries are
  materialized lazily as needed; solid 7z data is decompressed into an archive-owned buffer. Neither
  format is extracted into a mutable filesystem tree. Errors missed by the shallow check surface
  here with complete messages, so one compile round reports everything wrong with an archive instead
  of leaking them across successive attempts.
- **Check cache**: unchanged exports are not re-checked, so the grid can be warm on launch. A root
  `(path, size, mtime)` tuple is insufficient (it misses child-file edits); the cache identity is a
  **recursive manifest fingerprint** — a hash over the export's sorted relative paths, sizes, and
  mtimes (an archive hashes its file's identity) — combined with the validation-context revisions:
  the teams-list revision and the validation-affecting-settings revision, since both change results
  without changing export files. A watcher-driven revision counter was rejected as the identity
  because it cannot validate cold state across restarts — warm-on-launch is the point of the cache;
  the watcher only triggers re-fingerprinting.
- **Discard-and-re-read**: checked exports are not kept in memory; the compile re-reads from disk
  (predictable memory use, consistent with the memory budget). The check cache makes re-validation
  free. Deep checks acquire from the same memory budget as compilation, so a first launch over a
  large exports folder cannot OOM any more than a compile can.

---

## Cell states

```
        (folder watcher)
unchecked (background) → checking (inverse background) → partial-ok (blue)/full-ok (purple)/warning (faint yellow)/partial-error (faint red)/error (red)
        (Compile pressed — eligible cells are pending; effective DropFolder/DropExport scopes are blocked)
processing (bold number, color kept) → done (green)/done with warning (yellow)/done with errors (orange-red, pass-through or DropFile only)
```

Processing eligibility follows the effective disposition, not severity alone. `DropFile` discards
only the affected file and processes the remaining folder, which ends as `DoneWithErrors`;
`DropSlot` removes only the affected roster assignment (and therefore its cell) while other slots
remain eligible; `DropFolder` skips only that folder while sibling tasks continue; `DropExport`
skips everything belonging to that export; `AbortRun` terminates the run. An effective `DropExport`
gives every player and kit cell in that row a derived blocked state—not pending or processing—and
each tooltip references the export-level cause. A skipped scope otherwise keeps its error status
when Compile is pressed.

Planning messages are applied before manifest tasks become pending: rows dropped by planning become
derived blocked rows, valid tasks proceed, and a run-fatal planning failure turns all
otherwise-pending cells into a run-aborted outcome. A plan with no eligible tasks is a no-op
compile: it writes no artifacts and never replaces installed CPKs.

The debugging invariant **a red final compile outcome means nothing from that scope was written**
applies to final outcomes, not check-phase colors. During checking, a deep Error with `DropFile`
disposition is red because the offending file will be dropped; after compilation, the folder is
`DoneWithErrors` because its remaining content was written. Under `pass_through`, eligible
content-level `DropFile`/`DropFolder` findings are overridden to keep, so written errored content
likewise ends as `DoneWithErrors`; `studio_core::FolderStatus` includes that distinct state, and the
completion summary reports it separately from skipped folders.

Packed archives receive their deferred deep check at compile start (see "Live validation"), so by
the time the process-or-skip decision is made their error list is complete rather than
first-error-only.

egui renders cells with `painter.rect_filled` — direct GPU drawing, no DOM:

```rust
// Check-phase and compile-outcome colors. Processing reuses the retained
// check-phase status; the label turning bold is its only extra presentation.
fn cell_color(
    status: FolderStatus,
    check_status: FolderStatus,
    bg: Color32,
    fg: Color32,
) -> Color32 {
    let displayed = if status == FolderStatus::Processing {
        check_status
    } else {
        status
    };
    match displayed {
        FolderStatus::Unchecked       => bg,
        FolderStatus::Checking        => fg,
        FolderStatus::PartialOk       => Color32::from_rgb(0x32, 0x78, 0xDC),
        FolderStatus::FullOk          => Color32::from_rgb(0x9B, 0x59, 0xB6),
        FolderStatus::Warning         => Color32::from_rgb(0x8F, 0x82, 0x21),
        FolderStatus::PartialError    => Color32::from_rgb(0x8C, 0x2A, 0x2A),
        FolderStatus::Error           => Color32::from_rgb(0xDC, 0x3C, 0x3C),
        FolderStatus::Done            => Color32::from_rgb(0x32, 0xB4, 0x50),
        FolderStatus::DoneWithWarning => Color32::from_rgb(0xDC, 0xC8, 0x32),
        FolderStatus::DoneWithErrors  => Color32::from_rgb(0xB4, 0x64, 0x32),
        FolderStatus::Processing      => unreachable!("check status is never Processing"),
    }
}

let bg = ui.visuals().extreme_bg_color;
let fg = ui.visuals().extreme_fg_color;
let slot_count = if row.is_referee { 35 } else { 23 };

// Allocate every slot position even when it is empty, so 01-23/35 remain
// fixed columns across rows. Empty slots reserve space but paint no cell.
for slot in 1..=slot_count {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(26.0, 26.0),
        egui::Sense::click(),
    );
    let Some(cell) = row.cell_for_slot(slot) else { continue };

    let actual_color = cell_color(cell.status, cell.check_status, bg, fg);
    ui.painter().rect_filled(rect, 3.0, actual_color);

    let font = if cell.status == FolderStatus::Processing {
        egui::FontId::new(11.0, egui::FontFamily::Name("bold_mono".into()))
    } else {
        egui::FontId::monospace(11.0)
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{slot:02}"),
        font,
        label_color(actual_color), // contrast derives from the painted color
    );

    if response.clicked() {
        row.open_detail_for_slot(slot);
    }
    response.on_hover_ui(|ui| {
        ui.label(&cell.folder_name);
        // Messages are pre-sorted by severity (Fatal > Error > Warning > Info),
        // then by code for stable ordering.
        for msg in &cell.messages {
            ui.colored_label(actual_color, msg);
        }
    });
}
```

**Label contrast is derived from the cell, not the theme.** The number/kit label color is chosen by
the luminance of the cell's background color at all times: dark backgrounds get a light-grey label,
light backgrounds a dark-grey label — soft extremes rather than pure white/black, to avoid excessive
contrast. This is independent of the dark/light theme, which matters most for `Checking` cells
(inverse background flips with the theme) and keeps labels readable on every status color.

Color transitions (blue→green fade) are not built-in as with CSS; if desired, interpolate between
the old and new color over a few frames in the update loop (~10 lines).

---

## Toolbar

A single primary action:

- **Compile** — the only button in the default view. During a run it becomes **Cancel**. It is a
  **split button**: the main part runs the compile in the *current output mode*, the arrow part opens
  the mode list — **Compile** (normal, CPK), **Compile for sideloading** (unpacked PES folder structure
  into the game folder's `livecpk\`, Red's steps 1+2; disabled on PES 15/16, where no runtime
  serves it) and **Test output** (processed exports written unpacked in their original layout,
  Red's step 1) — see "Output modes and targets" in the Writer section. Picking a mode **sticks for
  the session** and relabels the main part, so the prototyping loop the sideload mode exists for
  (compile → alt-tab to PES → adjust → compile again) is one click per iteration after the first.
  The mode resets to normal on the next launch (output mode is deliberately not a persisted
  setting), and a `--gui` autorun always uses its own `--mode`, so `quick_compile.bat` cannot be
  hijacked by a leftover session mode. No separate buttons: the three are the same compile with a
  different sink, so they belong to the same control. With the sideload mode selected, a **Launch
  PES** button sits beside it: it starts the selected version's `PES20{version}.exe` from the
  game folder (`pes_folder_path`; never through Steam, whose launch is incompatible with the cup's
  mods, so every version runs from its install folder) when that PES is not running, and
  is disabled while it runs (the running-PES probe, `spec/core/architecture.md` "Crate
  structure", polled by the sidebar's selector); it
  starts the game and does nothing else. Red's staged operations as separate runs
  ("extracted → contents", "contents → CPK") are superseded by the unified pipeline and are not
  exposed. GUI autorun (`quick_compile.bat`, see
  `spec/team_compiler/settings.md` "CLI") presses this button programmatically once the initial check completes; a run started that
  way is indistinguishable in the grid and log from a manual click, except that
  `quick_compile_close_on_success` may close the window when it ends. The button carries the
  **destination writability badge** (PES running / elevation needed — see `spec/team_compiler/pipeline.md` "Post-processing") with the
  reason and remedy in its tooltip, and its dropdown always contains **Open output folder**.
- **Run strip** — a one-line bar next to the button: current stage, elapsed time, files written,
  memory in use. (The one name for it across the plans: "strip" alone means a kit here, and
  "status strip" would be confused with the shell's status bar.) The core plan's current
  `Progress { processed, total }` payload is insufficient; extend it with these metrics or add
  separate `StageChanged`/`MetricsUpdated` events before implementing this view
  (`core/architecture.md` "Event system").

---
