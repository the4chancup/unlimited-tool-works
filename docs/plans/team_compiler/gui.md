# Unlimited Tool Works: Compiler plan: GUI grid

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

Part of the [Compiler plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## GUI: the live-validating team grid

The primary view is a grid where **each aesthetics export is a row**, keyed internally by stable
`ExportId` rather than display name, with the team name label on the left, the team ID cell next to
it, one cell per player at a fixed position by number, plus one cell per kit:

```
/aa/   [714]   [01][02][03][04][  ][  ][  ][  ]   [GK][K1][K2]
/bb/   [???]   [01][02][03][04][05][06][07][08]   [GK][K1]
/cc/   [809]   [01][  ][03][  ][05][  ][  ][  ]   [GK]
```

The grid is drawn by `libs/export_grid`, which the Export upgrader's Grid look draws too
(`plans/export_upgrader.md` "GUI"). The lib draws the rows a tool hands it: labels, lines of
cells at fixed positions, their states and messages, the legend and the detail window. It reads no export and runs no check: each tool maps its own results to
rows, the Compiler its check's findings and the upgrader its run's folders. It is a lib, not
part of this crate, because a tool crate never depends on another, and two copies of one grid
would drift apart.

- **Team name label**: fixed-width left column with the canonical `/xx/` `team_name`; its detail
  window shows the complete `export_display_name` and resolved identity. The label doubles as the **cell
  for export-scoped and non-slot task messages**: shared model folders, Logo, Common, Collars,
  RefMarker, and root-artifact scopes have no cells of their own, and neither has a player folder
  whose name claims no slot (`15_Snuffy`, `player_folder_number_invalid`), so their status and
  messages roll up here. For example, an orphaned shared folder (`shared_folder_orphaned`) turns
  the label amber, with the details in its window.
- **Team ID cell**: between the team name and the first player column, showing the ID resolved from
  `teams_list.txt`. When the team is not on the teams list (`team_name_unknown`), the cell turns
  **red and editable**: the user types an ID and confirms with Enter, and the ID is **written to the
  teams list**, the successor to Red's interactive prompt.

  The entry is validated on confirm: numeric and in the 701–920 range (violations keep the cell
  red with the reason in the tooltip). An ID **already assigned to another team** asks for
  confirmation on the cell ("in use by {team}; reassign?") before overwriting the old entry.
  This is the same overwrite flow Red's prompt offered, kept because its purpose is correcting
  a stale teams list.

  The teams-list write is picked up like any file change: every export re-resolves and
  revalidates, so a reassignment's effect on other rows (the team that lost the ID turning red)
  is immediately visible. If the write fails because the data directory is not writable, the
  cell reverts and stays red with `teams_list_read_only` in its tooltip. There is no elevation
  path (see `spec/team_compiler/pipeline.md` "Resolved decisions", "Teams list").
- **Untagged export**: a team export whose name has neither `Full` nor `Midcup`
  (`export_tag_missing`) is not compiled and has no player to show, so its row holds two
  buttons, **Full** and **Midcup**, where its player cells would be: no space is reserved for
  them on other rows. A click renames the export's folder or archive, putting the tag right
  after the team name, and the row revalidates like after any rename.
- **Player cells**: one per player, labeled with the player number (`01`–`23`, from the folder name
  or the root `players.txt`). The columns are **fixed positions 01–23**: a player without a folder
  in the export leaves its column completely empty (no cell, no number), so columns always align
  across rows by number: missing players are visible at a glance.
- **Referee rows**: the fixed positions extend to `01`–`35` (one per referee slot). Each slot listed
  in `players.txt` gets a cell, and slots backed by the same model folder share its status (the
  detail window names the folder, making the repetition visible).
- **Kit cells**: one per kit. The `Kits/` folder has one subfolder per kit (containing
  `config.toml`, `colors.txt`, and that kit's textures), which gives the per-kit cell
  granularity naturally. `all/` is not a kit and gets no cell: its effect shows in the kit
  cells' detail windows, which name the folder (`p1 - Lakers`) and the stems inherited from `all/`.

  Kit cells use a separate fixed-slot loop (`GK`, then `K1`–`K9`) following the same
  reserve-every-position rule as player cells. They use the same square shape and are
  separated from the player group by a small gap. Color carries status; the label carries
  identity.

  Once a kit's menu colors are known (its `colors.txt` parses at check time, or the compile
  derives them from the texture), the cell additionally shows its **menu icon's pattern**,
  drawn centered under the label on the status background. That is the `icon_<N>` marker's
  pattern rendered by `color_tools` in pattern-only mode: a flat two-color swatch, no shirt
  silhouette, which wouldn't read at cell size. A failed kit keeps its error state and shows
  the existing base-`UniColor.bin` colors rather than the unwritten pair.

  This is the kit colors' proof-of-life: a row of identical all-black miniatures exposes a lazy
  set of `colors.txt` files at a glance, and a derived-color miniature shows exactly what the
  extraction fallback picked before anything ships.
- **Task-to-cell mapping**: player model and portrait tasks update every mapped player/referee
  slot cell. Kit tasks update their kit cell. Logo/Common/Collar/RefMarker/root-artifact work
  updates the team/ref row label. Run/output failures appear in the run strip and log, with
  run-abort state propagated to pending cells as described below.
- **The detail window** shows one cell (a player, the portraits, a kit, or a team label for its
  export's own messages): its folder, its state, and each message with the file it is about
  and the catalog's When and Effect texts, then Open folder (when the cell has a folder to
  open, below) and, for a finding with one obvious fix, the rename that fixes it. The cells
  carry no tooltip of their own; the ID cell and the buttons keep theirs.

  A team label's window also offers **Copy messages** when the export has a Warning or an
  Error. It puts the export's log lines on the clipboard, as the log prints them (its Infos
  too), and reads "Copied" for a moment. Some organizers tell a member what is wrong with
  their export rather than fixing it themselves, and this gives them the text to send.
  **Copy report** would put the whole run's lines on the clipboard, which they would have to
  cut down to the one export. An export with Infos alone does not offer it: every export the
  check identifies carries the Info `export_identified`, so a clean export would copy that
  line alone, with nothing in it for a member to fix.

  The window opens on a box the 3D preview's size. The box is there on every cell, so the
  text under it stays in place as the window moves between cells. What it holds depends on
  the cell:
  - a player: his 3D preview (Future Features), with his portrait in the bottom-left corner
    when he has one (in his folder or in `Portraits/`, the one the compile packs) and the
    export's logo in the bottom-right one, each the size of a portraits tile; until the
    preview is built, the box holds those two alone;
  - a kit: its main texture (`kit.dds`), fitted inside the box;
  - the portraits: every portrait as a 6 × 4 grid in the players' order, the export's logo
    in the last tile, at the bottom right, so 23 portraits and the logo fill the 24 tiles; a
    player with no portrait leaves his tile empty;
  - a team label: the export's logo (its root `logo*` texture), fitted inside the box.
  A picture fitted inside the box is shrunk to fit, never enlarged: a small logo stands at
  its own size, as it would blur if enlarged. A tile's picture (a portrait, or the logo in a
  tile or a corner) fills its tile either way, its shape kept.
  Portraits and logos keep their transparency: each is drawn straight on the box's
  background, with no tile or frame behind it, so a crest or a cut-out portrait shows its
  own shape rather than a square. An export with no logo shows "No logo" where the logo
  would be. In a referee export, the
  marker texture (`ref_marker.dds`), when there is one, takes the logo's place in the window,
  because a referee export's logo reaches no output while the marker is the emblem the
  referees wear in game. The pictures are the files the compile packs, taken from the export's
  listing when the check dropped the whole export, so a member still sees what it holds. A
  `.zip` export's pictures are read from the archive, each from its own entry, since a zip
  unpacks one entry without the rest. A `.7z` export's boxes stay empty until the check cache
  (below) keeps its pictures: reading one out of a solid `.7z` means decompressing it whole,
  which only its deep check does, in its turn.
  - **Hover** opens it on the cell under the pointer, and leaving the cell closes it. It opens
    after 0.15 s, and then follows the pointer from cell to cell with no delay. That is
    longer than the sidebar's 0.1 s, so a pointer crossing the grid does not flash a window
    the size of the 3D preview over every cell it passes.
  - **A click pins it**: a pin shows on the cell and in the window's title bar, and the window
    stays open on that cell, with its buttons usable, and dragging its title bar moves it.
    Scrolled under the grid's title row, the cell's pin is covered with the cell.
    While a cell is pinned, hovering another opens nothing, so the pointer can cross the grid
    to the window. A click on the pinned cell, on the window's pin or on the grid's empty
    space unpins it; a click on another cell pins that one. After a click that unpins, the
    cell opens no window on hover until the pointer leaves it: egui reports it hovered on the
    next frame, which would open the window the click just closed.
  - **Esc** closes the window, pinned or not, as a click that unpins does.
  - **A double-click** opens the cell's folder in the file manager (for an archive export, the
    folder holding it, with the archive selected), and leaves the pin as it was before it.
    The portraits cell's folder is the export's `Portraits/` when it has one. Most exports have
    none and keep each portrait in its player's folder, so the cell has no folder: a
    double-click does nothing, and the window shows no Open folder. A finding about such a
    portrait is on its player's cell, which opens his folder.
  - **Where it opens**: where the member last left it (at first, at the grid's right), not
    beside the cell, because a window sized for the 3D preview is too big to follow the pointer.
    While unpinned it lets the pointer through, so a cell under it can still be hovered.
- **A message's log line** is `Severity - /team/ - code`, then ` at ` and the file or folder it
  is about, then its context fields, as the console prints them; the catalog's text, a
  paragraph, is the window's and the help's, which the code links to.

  A line acts for its cell (the team label for an export's own message). Hovering it opens
  the cell's window as hovering the cell does, under the same rules. A click outlines the cell,
  scrolls it into view and pins its window; a second click on the line unpins the window and
  keeps the outline.
- One row per team bounds the row count (~50), so no virtualization is needed; direct egui rendering
  suffices.

### Sizes and sorting

The legend's row ends, at its right, with what the cells show, the unit and the rows' order,
whether the legend is folded or not:

- **Status | Sizes.** Status is the cell states above. Sizes colours each cell by its size,
  green to red, so an organizer sees which export or player is heavy before the DLC grows.
  A cell not read yet keeps the unchecked colour, and in Sizes the warning and error rings and
  the dashed blocked look are not drawn: red means large and nothing else, and Status is a
  click away.
- **What a cell measures.** A player cell measures his folder plus the shared `Faces/`,
  `Boots/` and `Gloves/` folders he links to, since the compile copies those into his files:
  a player split over several folders shows his whole weight. A kit cell measures its folder
  plus what it takes from `all/`, the portraits cell its portraits, and a team label its whole
  export.
- **The scale.** Each kind has three fixed thresholds: green up to the first, amber at the
  second, red from the third, blended on a log scale between. They are players 10, 20 and
  32 MB, kits 6, 10 and 16 MB, portraits 1, 2 and 4 MB, and exports 150, 300 and 500 MB.
  In Sizes the legend shows them in place of the states.

  The thresholds come from a census of the exports on the maintainer's PC (2026-11-22): 828
  player totals in 36 upgraded VGL exports had a median of 7.8 MB, p75 12.4 and p90 18.9; their
  164 kits clustered at 5.3 MB; 117 exports had a median of 123 MB and p75 235 MB. An
  export's green ends at 150 MB, near the median, because 250 MB is already large for an
  export. A player's thresholds keep the same proportions to his red end, 32 MB, as an
  export's keep to its 500 MB. The red ends are
  the 4cc rules page's own (a model folder over 32 MB, an export over 500 MB, "unless it
  is absolutely required"). Fixed thresholds, not a scale relative to the folder's largest,
  because a relative scale paints someone red in a cup where every export is fine.
- **Disk | VRAM**, always shown and enabled only in Sizes: the controls are right-aligned,
  so a pair appearing on a click on Sizes would push the toggle out from under the pointer.
  Disabled, it is drawn as the Save editor's disabled formation picker: the chosen unit as a
  disabled button, not on the selection's blue, the other in the disabled grey.
  Disk is the files' size. VRAM is what the textures and
  models take in the game's video memory, read from headers at check time with no compile:
  - a DDS takes its payload;
  - a WESYS-wrapped file takes its unpacked length (the wrapper's header holds it);
  - a PNG takes the DDS the compile makes of it, with its mips: DXT1 when its header has no
    alpha (no alpha channel, no `tRNS`) and it is not a normal map, else DXT5 until a compile;
  - any other file takes its size on disk.

  The compile, without BC7, picks DXT1 for a PNG whose pixels are all opaque (`dds_convert`'s
  codec choice), which only a full decode tells, too slow for a check. So before a compile
  such a PNG counts as DXT5, the most it can take, and after one its real size is shown. An
  `.7z` export, read shallowly, has no VRAM size until its deep check. One set of thresholds
  serves both units, because today's exports ship DDS textures, whose VRAM size equals their
  payload on disk (the census: VRAM over disk 1.00 to 1.01 on 75 exports).
- **Sort**, in either colouring: Folder order, Errors first (an Error, then a Warning, then
  the rest), Largest export and Largest player (the export with the heaviest player first),
  in the unit chosen. Rows equal under the sort keep the folder's order.
- **The window in Sizes** adds the cell's size on disk and in VRAM (noting the PNGs counted
  as DXT5 until a compile), the shared folders a player's total takes in, and the five
  largest files. A label's window lists the five largest players instead, since that is what
  to shrink first.
- **The strip in Sizes** adds the exports' total in the chosen unit (`2.5 GB on disk`),
  `at least` while any export is not measured.

### Row actions

What a member does about a row, from the row itself. Every action that writes into the
exports folder is a rename, a marker or a copy the member asks for by a click; nothing is
changed on its own. The buttons a row holds itself (**Enable**, **Disable**, **Move to unused**)
sit in the grid's last column, right of the kits, titled **Actions** in the title row like the
other columns, so on the rows with none the column reads as theirs, not as unused space.

- **Open folder.** The detail panel and a log line's context menu carry **Open folder**,
  opening the scope's folder in the file manager, a file's folder with the file selected
  (`explorer /select,` on Windows, `xdg-open` on the folder elsewhere); for an archive export,
  the archive's folder with the archive selected. The fix for a finding is nearly always a
  rename, a link or a removal in the file manager, so the click saves finding the folder by
  hand.
- **One-click fixes for the mechanical cases.** A finding with one obvious fix gets a button
  in the detail panel that applies it, the member still choosing:
  - a known file with a hidden `.txt` (`settings.toml.txt` → `settings.toml`; kept disallowed,
    so the mistake is named);
  - an old `Portraits/player_05.dds` (→ the player folder's portrait name);
  - a player folder `15_Snuffy` (→ `15 - Snuffy`; `15 Snuffy` is a valid name,
    `plans/aesthetics_export/player_folders.md` "Player numbering without the hyphen").

  Each is a rename refused when the target name exists (the refusal says so), and the row
  revalidates as after any rename. The buttons are for a folder export only: inside an archive
  nothing is renamed, so an archive's finding has no button. These fixes widen the precedent
  of the GUI renaming in an export, which the untagged row's **Full**/**Midcup** buttons set.
- **An old export offers its upgrade.** A row red with `export_layout_old` or
  `player_layout_proto` carries an **Upgrade** button: the shell switches to the Export
  upgrader with the export as its input (a tool switch with an argument, as the Refs arranger's
  handoff). After a clean upgrade, the upgrader asks once whether to move the old export into a
  sibling `old exports/` folder, so its red row goes. If that is declined, the old export stays
  where it is. The upgrader's "Move to exports and compile" (`plans/export_upgrader.md` "GUI")
  is the handoff back.
- **The empty grid.** An exports folder with no export (`no_exports_found`) shows, in the
  grid's place, the folder's path, an **Open exports folder** button and a line saying an
  export can be dropped here. A folder, `.zip` or `.7z` dropped anywhere on the grid is copied
  into the exports folder (refused, naming it, when that name exists) and checked like any new
  export; the drop works on a filled grid too.
- **Compile one export, or a chosen few.** A row's context menu has **Compile only this
  export**, the CLI's `--export` from the grid, in the current output mode.

  A **compile column** of checkboxes at the start of each row, ticked by default, picks the
  exports the next Compile takes. It is this run's selection only: it is written nowhere, and
  every row is ticked again on the next start. An unticked row is greyed and skipped as if
  absent. The column is hidden by default, shown and hidden by a toggle in the Compile
  button's dropdown. While it is hidden, every row is compiled. It is not the `NO_USE` marker,
  because a checkbox that wrote into the export would change files on a click meant for one
  run.
- **Save findings open the Save editor at their fix.** `savefile_missing` (the patch written,
  no save to write it into) carries **Apply in Save editor**: the shell switches to the Save
  editor with that patch loaded and its apply action ready, the member picking the save. It is
  the tool switch with an argument the Upgrade button uses. The save is written by the Save
  editor's own apply, the one save-writing path (`savefile_player_not_stripped` goes with the
  stripped route, worklog 8.36c).

  `savefile_skipped_pes_running` gets no button: the view asks the Save editor's Edit mode
  question (`plans/save_editor.md` "Writing a save while its game runs") in place of the
  command line's `--edit-mode-closed` (`spec/team_compiler/pipeline.md` "6. Post-processing"),
  its **Update save** running the savefile step as the flag does and its **Skip** leaving the
  save as it was. Its title asks whether to update the game's save, and its text opens on what
  the step writes, the compiled teams' aesthetics settings into that save, before the Edit
  mode advice: the Save editor asks after the member pressed Save, where here the member
  pressed Compile, and the advice alone would leave them wondering which save it means. The question is drafted before it is
  built. Until then the window's Compile runs without the flag, and the skip's catalog text,
  which names the flag, is the command line's.
- **Disabled exports stay visible.** An export with its `NO_USE` marker is a greyed row with
  **Enable** (removing the marker) rather than left out of the grid, so what was disabled can
  be seen and undone.

  Where two exports clash (`duplicate_aesthetics_export`, `multiple_ref_exports`, which skip
  every export involved), each clashing folder export's row offers **Disable** (writing its
  `NO_USE` marker). An archive offers a move into `old exports/` instead, since no marker can
  be written inside it. Disabling one clears the clash and the other compiles.

---

## Live validation (no Check or Refresh buttons)

The exports folder is monitored continuously; checking requires no user input. The watcher and
the cache below are `libs/pipeline`'s, shared with the Balls compiler's list view; this crate's
`check/` decides what to check when and maps the results to rows (`README.md` "Crate layout"):

- **Folder watching** via the `notify` crate (`ReadDirectoryChangesW` on Windows, inotify on Linux),
  with `notify-debouncer-full` for event debouncing. New exports trigger checks after a ~1–2s quiet
  period plus a file-size stability check (so half-copied archives aren't parsed). Deletions and
  renames update the grid immediately.
- **Parallel checking**: each triggered check is an independent task on the rayon pool (never on
  the UI thread), and multiple exports check concurrently (e.g. on first launch over a full
  exports folder, or when several exports are dropped in at once). Within an export, per-scope
  checks fan out in parallel too.

  Results arrive as `Message`/`FolderStatus` events mapped to each scope's slot, kit, or
  row-label cell. Cells flip from `checking` as each scope finishes, not per export. Events
  carry the run and export revision they were computed from, and the receiver rejects results
  older than the row's current revision, so a stale check can never overwrite newer state.
- **Deep check (plain-folder and `.zip` exports)**: full parse + validation of every file. Rust
  makes this near-instant for folders. A `.zip` is read the same way, one entry at a time from
  the file on disk, so it costs no more memory than a folder: a zip has no solid mode, and the
  zipped exports on record are deflated or stored, which unpacks fast. When compiling a full DLC, users normally
  submit plain folders, so the main workflow gets full validation automatically.

  **Deep means every check whose inputs exist before a compile**: the export's own files, the
  teams list, the settings and the installed CPKs the compile would read. A finding the
  compile reports from those same inputs is a gap in the deep check, not a compile-time
  finding: a cell that looks fully checked and turns red only at Compile tells the member
  nothing until minutes later.

  `fmdl_texture_not_found` is the first such case: the deep check runs the same texture lookup
  for each source model's own texture names (one function both callers use, never a copy, with
  a test pair keeping them in agreement). Only a finding about something the compile itself
  produces (a merged model's combined texture list, a retargeted model, a converted model over
  its limit) stays with the compile, and its catalog row says why.
- **Shallow check, then a deep one in its turn (`.7z` exports)**: a solid `.7z` is read by
  decompressing it whole into one buffer, so its deep check costs its whole unpacked size in
  memory. A `.7z` therefore gets a shallow check first: its entry table alone (file names,
  sizes, structure), read without decompressing, which catches naming, nesting and file-type
  issues in milliseconds. Its content-dependent checks (roster syntax, config parsing) wait,
  and the parse result records the roster as *unread* rather than missing.

  Its deep check comes once every folder and `.zip` export has been checked, one `.7z` at a
  time, each when its unpacked size fits the memory budget the compile uses (the compile's own
  check orders them the same way). The buffer is freed when the check ends, and the compile
  decompresses the archive again: at a real `.7z` export's size (about 2 MB) that costs well
  under a second, against holding every archive's contents for as long as the view is open.

  **Shallow-passed cells are blue instead of purple** until the deep check replaces their
  result, indicating the export may still contain issues that only a deep check would find.
- **Compile-start deep check for `.7z` exports**: a `.7z` whose deep check has not run yet when
  Compile is pressed gets it then, before the process-or-skip decision. Its solid data is
  decompressed into an archive-owned buffer, never extracted into a mutable filesystem tree.
  Errors missed by the shallow check surface here with complete messages, so one compile round
  reports everything wrong with an archive instead of leaking them across successive
  attempts.
- **Check cache**: unchanged exports are not re-checked, so the grid can be warm on launch. A root
  `(path, size, mtime)` tuple is insufficient (it misses child-file edits). The cache identity
  is a **recursive manifest fingerprint**, a hash over the export's sorted relative paths,
  sizes, and mtimes (an archive hashes its file's identity). It is combined with the
  validation-context revisions: the teams-list revision and the validation-affecting-settings
  revision, since both change results without changing export files.

  A watcher-driven revision counter was rejected as the identity because it cannot validate
  cold state across restarts, and warm-on-launch is the point of the cache. The watcher only
  triggers re-fingerprinting.
- **Discard-and-re-read**: checked exports are not kept in memory; the compile re-reads from disk
  (predictable memory use, consistent with the memory budget). The check cache makes re-validation
  free. Deep checks acquire from the same memory budget as compilation, so a first launch over a
  large exports folder cannot OOM any more than a compile can.

---

## Cell states

```
        (folder watcher)
unchecked (background) → checking (inverse background) → partial-ok (blue)/full-ok (purple), outlined in amber for a warning, in red for an error
        (Compile pressed — eligible cells are pending; effective DropFolder/DropExport scopes are blocked)
processing (color kept, a light bar sweeping the cell's bottom edge) → done (green)/done with warning (amber)/done with errors (red, pass-through or DropFile only)
```

Processing eligibility follows the effective disposition, not severity alone:

- `DropFile` discards only the affected file and processes the remaining folder, which ends as
  `DoneWithErrors`;
- `DropSlot` removes only the affected roster assignment (and therefore its cell) while other
  slots remain eligible;
- `DropFolder` skips only that folder while sibling tasks continue;
- `DropExport` skips everything belonging to that export;
- `AbortRun` terminates the run.

An effective `DropExport` gives every player and kit cell in that row a derived blocked state
(not pending or processing), and each one's detail window names the export-level cause. A skipped
scope otherwise keeps its error status when Compile is pressed.

Planning messages are applied before manifest tasks become pending: rows dropped by planning become
derived blocked rows, valid tasks proceed, and a run-fatal planning failure turns all
otherwise-pending cells into a run-aborted outcome. A plan with no eligible tasks is a no-op
compile: it writes no artifacts and never replaces installed CPKs.

The debugging invariant **a red final compile outcome means nothing from that scope was written**
applies to final outcomes, not check-phase colors. During checking, a deep Error with `DropFile`
disposition is red because the offending file will be dropped; after compilation, the folder is
`DoneWithErrors` because its remaining content was written. Under `pass_through`, eligible
content-level `DropFile`/`DropFolder` findings are overridden to keep, so written errored content
likewise ends as `DoneWithErrors`. `studio_core::FolderStatus` includes that distinct state, and
the completion summary reports it separately from skipped folders.

Packed archives receive their deferred deep check at compile start (see "Live validation"), so by
the time the process-or-skip decision is made their error list is complete rather than
first-error-only.

egui renders cells with `painter.rect_filled` (direct GPU drawing, no DOM):

```rust
// Check-phase and compile-outcome colors. Processing reuses the retained
// check-phase status; the sweeping bar (below) is its only extra presentation.
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

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{slot:02}"),
        egui::FontId::monospace(11.0),
        label_color(actual_color), // contrast derives from the painted color
    );

    // The detail window (above): hover shows it, a click pins it, a
    // double-click opens the folder. Its messages are sorted by severity
    // (Fatal > Error > Warning > Info), then by code for stable ordering.
    detail.track(row.scope_for_slot(slot), &response);
}
```

**Label contrast is derived from the cell, not the theme.** The number/kit label color is chosen by
the luminance of the cell's background color at all times: dark backgrounds get a light-grey label,
light backgrounds a dark-grey label. These are soft extremes rather than pure white/black, to
avoid excessive contrast. This is independent of the dark/light theme, which matters most for
`Checking` cells (inverse background flips with the theme) and keeps labels readable on every
status color.

**A processing cell moves**, and nothing else in the grid does, so the eye finds the few
cells being compiled without a colour of their own (a bold label alone was too hard to see). A
light bar, 2 points tall and two fifths of the cell's width, sweeps the cell's bottom edge from
left to right, entering and leaving it, once every 0.9 seconds, in the strong text colour. Its
phase is counted from that cell's own start (`ctx.input(|i| i.time)` less the time it turned
Processing), so a short task still shows the bar setting off and the cells do not move in
step. It is clipped to the cell and painted after the Warning or Error ring, so it shows over
them. While any cell processes the view requests a repaint each frame; it stops when none
does.

Color transitions (blue→green fade) are not built-in as with CSS. If desired, interpolate
between the old and new color over a few frames in the update loop (~10 lines).

---

## Toolbar

A single primary action:

- **Compile**: the only button in the default view. During a run it becomes **Cancel**.

  It is a **split button**: the main part runs the compile in the *current output mode*, the
  arrow part opens the mode list (see "Output modes and targets" in the Writer section):
  - **Compile** (normal, CPK);
  - **Compile for sideloading** (unpacked PES folder structure into the game folder's
    `livecpk\`, Red's steps 1+2; disabled on PES 15/16, where no runtime serves it);
  - **Test output** (processed exports written unpacked in their original layout, Red's
    step 1).

  Picking a mode **sticks for the session** and relabels the main part, so the prototyping loop
  the sideload mode exists for (compile → alt-tab to PES → adjust → compile again) is one click
  per iteration after the first. The mode resets to normal on the next launch (output mode is
  deliberately not a persisted setting), and a `--gui` autorun always uses its own `--mode`, so
  `quick_compile.bat` cannot be hijacked by a leftover session mode. The modes get no separate
  buttons, because the three are the same compile with a different sink, so they belong to the
  same control.

  With the sideload mode selected, a **Launch PES** button sits beside it. When that PES is not
  running, the button starts the selected version's `PES20{version}.exe` from the game folder
  (`pes_folder_path`). It never launches through Steam, whose launch is incompatible with the
  cup's mods, so every version runs from its install folder. It is disabled while that PES
  runs (the running-PES probe, `spec/core/architecture.md` "Crate structure", polled by the
  sidebar's selector). It starts the game and does nothing else.

  When the last run's preflight found the game's `DpFileList.bin` not the official one
  (`dpfilelist_outdated`, `dpfilelist_not_official` or `dpfilelist_cpk_missing`), an **Upgrade
  DpFileList** button sits beside Compile. It opens the upgrade's dialog
  (`pipeline.md` "DpFileList upgrade"). Once the member confirms, the upgrade does what
  `upgrade-dpfl --yes` does and deletes the CPKs ticked in the dialog, its lines going to the
  log. The button stays until an upgrade
  replaces the list or a later run's preflight finds it official. It is in the toolbar, not
  only in the finding's log line, because an outdated list stops every compile until it is
  replaced: the remedy sits beside the button it unblocks. It shows after a run, not from a
  live check, because the preflight stops a run before anything is compiled, so the member
  loses one click and no time.

  Red's staged operations as separate runs ("extracted → contents", "contents → CPK") are
  superseded by the unified pipeline and are not exposed.

  GUI autorun (`quick_compile.bat`, see `spec/team_compiler/settings.md` "CLI") presses this
  button programmatically once the initial check completes. A run started that way is
  indistinguishable in the grid and log from a manual click, except that
  `quick_compile_close_on_success` may close the window when it ends.

  The button carries the **destination writability badge** (PES running / elevation needed;
  see `spec/team_compiler/pipeline.md` "Post-processing") with the reason and remedy in its
  tooltip, and its dropdown always contains **Open output folder** and **Copy report** (the
  run's lines as the console prints them, for a member asking for help in the cup's thread,
  as the Export upgrader's Upgrade menu offers).
- **Run strip**: a one-line bar next to the button: current stage, elapsed time, files written
  with their total size so far (`592 files written (306 MB)`, summed as each file is written),
  memory in use. Once a run ends it gives the run's line, `Compile finished in 00:42 · 1184
  files written (612 MB)`, so a member sees what the run put on disk. (The one name for it across the plans: "strip" alone means a kit here, and
  "status strip" would be confused with the shell's status bar.) The core plan's current
  `Progress { processed, total }` payload is insufficient; extend it with these metrics or add
  separate `StageChanged`/`MetricsUpdated` events before implementing this view
  (`core/architecture.md` "Event system").

  While nothing runs, the strip also sums up the grid after the last run's line: `14 exports ·
  6 skipped · 3 with errors · 1 with warnings`. Each count is of exports, not of messages, and
  a count of zero is left out. This answers an organizer's first question, whether anything is wrong at all. A click
  on the error or warning count sets the log's filter to Errors or Warnings, and expands the
  log if it is collapsed. The two counts are in the link blue, never underlined, only while
  the log is collapsed. While it is open they are in the strip's own grey, still clickable,
  because the log's own filter buttons sit right below them and two blue rows of the same
  choice are one too many.

  The log starts collapsed when, open, it would push some of the grid's cells out of view
  (the grid scrolling to reach them): the grid is
  what the view is opened for, and the strip's counts still say whether anything is wrong.
  This is decided once per launch, at the view's first frame with rows. After that, the
  chevron and the counts open and close the log.

---
