# Unlimited Tool Works: Export upgrader plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The Export upgrader is built through the CLI (`spec/export_upgrader.md`, Phase 6). What is
left is step 8's base copy (worklog 8.36c), step 13, the optional glTF pass (Phase 7), and the
tool's view (Phase 8).

---

## What it does

What exists of this is in `spec/export_upgrader.md` "What it does", steps 1 to 12.

Step 8 gains the base copy (with worklog 8.36c). A player whose record in the savefile copies
another player (a base-copy id neither 0 nor his own) gets that id's `<id>.copy` link in his
folder (`plans/aesthetics_export/player_folders.md` "Player folders (the Studio export
format)"), because the copy lives only in the save and the `.ted` and would otherwise be
cleared by the first patch.

When his folder also holds a face of his own, the face stays, the copy is not written and a
Warning names both: `base_copy_beside_face` (context: the folder, the copied id). The game
showed the copy over that face, so the manager's face was not what the cup saw, and the member
is told so rather than having it chosen for them silently.

A face of his own is a face folder's model. An in-game face the upgrader would generate from
his record (the `ingame_face.toml` of a player with no face folder, or of a folder holding the
old `ingame_face` marker) is not one. The game drew the copy over that record, so the copy is
what the cup saw, and the folder gets the link in place of the `ingame_face.toml`, with no
Warning.

A copying player the upgrade gives no folder (a stock-booted player with no face folder) needs
no link, because no patch entry names him, so his record, copy included, stays as the save
holds it. Loose mode, which has no save, writes none.

The shared validation's `logo_missing` (`plans/aesthetics_export/object_model.md`
"Validation semantics") reaches the upgrader too. An export upgraded as `Full` (from its name,
from `--coverage full`, or by default) whose `Logo/` folder gives no logo writes nothing. On
the maintainer's machine, 19 of about 100 old exports that would be `Full` have no `Logo/`
folder. Most are from 2016 to 2020, and at least one holds only faces, likely a mid-cup
export named neither way, which the Error now stops before a compile resets its team. The
text names the two
ways out: put the logo in the old export's `Logo/` folder, or upgrade it with
`--coverage midcup` when it only adds to what is installed.

The shared validation's `portrait_missing`, a Warning for a player with no portrait on PES
2017 to 2021, reaches the upgrader the same way. Being a Warning that drops nothing, it
leaves the run clean.

What worklog 0.0.2-poc1, poc4 and poc6 built is in `spec/export_upgrader.md` "What it
does": step 3 ("When the save and the folders disagree"), step 8 (the hide ID of a
custom ID no folder claims) and step 10 (a `.dds` beside a `.ftex` of one stem).

What worklog 0.0.2-poc5 and poc7 built is in `spec/export_upgrader.md` "What it does",
step 7 (a model hidden by distance, anti-blur duplicates dropped). The last step:

13. **Optional glTF conversion** (`--convert-to-gltf`, off by default): after the structural
    migration is complete and verified (step 10's report is clean), a second pass converts all
    model files to glTF in place. Every `.fmdl` (Fox exports) or `.model`+`.mtl` (pre-Fox
    exports) in player folders, shared folders, and Common is imported to the IR and exported as
    one `.glb` per model via `model_convert`'s `ir_to_gltf`.

    Old exports are always single-engine (Fox-only or pre-Fox-only; Red cannot handle mixed
    states), so there is no dual-set superset merge to automate: each model converts plainly
    through one path. Skeleton transforms come from the FMDL's companion SKL or the embedded
    template skeleton, or from a `.model`'s inline bone table (see "Skeleton reconstruction from
    FMDL" and "Skeleton in `.model`" in the [Model conversion
    spec](../spec/model_conversion/gltf.md)).

    Textures stay in place: existing DDS/FTEX referenced by stem from the generated
    `materials.toml`, whose entries carry the `family` inferred from the native shader plus the
    native engine table (see "Emission" in the [Unified model format plan](model_format.md)).

    - A `.mtl.common` link converts to the matching `.materials.toml.common` link, and the
      Common `.mtl` it points at is converted in place like any other.
    - Texture links (`hair.dds.common`, created by step 7's path normalization or already
      present) are format-neutral and stay as they are.
    - For pre-Fox exports, `.mtl` texture paths are stripped of extensions (normalized to
      stem-based references, matching the Studio format).

    Replacement is transactional per model folder: all parts convert first, and the
    structurally-migrated originals remain recoverable until publication completes. Multiple
    writes/deletes are not a single atomic filesystem operation. Conversion or publication
    failure restores the native folder, or retains a clearly identified recoverable copy if
    rollback itself fails. Success is not reported for a partial migration.

    The two-step design keeps the structural migration lossless and parity-testable: the test
    suite validates the structural step against Red's reference output, and the glTF conversion
    has its own roundtrip tests in `model_convert`.

## Model-folder mode

Model-folder mode is removed in 0.0.2: the `upgrade-models` command, its module, its help,
its spec section and its scenarios (EU-MF-*, `spec/withdrawn_ids.md`) go, and the view
has no second input. An old model that arrives after a team's upgrade comes in a Midcup
export, a team export holding only what it adds, so the upgrader knows the team and reads
the save's settings for its players as for any export. A bare folder of model folders names
no team, so its upgrade could carry nothing of the save, and the mode existed only for it. No release presented the command (the
upgrader's first preview is 0.0.2's), so it goes without a deprecation.

## CLI

What exists of this is in `spec/export_upgrader.md` "CLI". Step 13 adds `--convert-to-gltf` to
`upgrade` (off by default):

```text
UTW export-upgrader upgrade [<export>...] [--all <DIR>] [--savefile <EDIT> | --loose] [--coverage full|midcup] [--convert-to-gltf] [-o <dir>]
```

Two additions to the built commands (the coverage tag and the staging folder's sweep are
built: `spec/export_upgrader.md` "CLI"):

- **Several exports at once.** `upgrade` takes one or more exports, with one `--savefile` or
  `--loose` for all of them. It is for an organizer moving a cup's library (about 50 teams)
  to Studio once per cup.

  **Which exports.** The ones named, then, with `--all <DIR>`, every export directly inside
  DIR: its folders and its `.zip` and `.7z` files, sorted by file name without regard to
  letter case. Any other file in DIR is passed over without a line, and so is a folder
  whose name ends in `.upgrading`, the staging folder an interrupted run left: it is no
  export, and the export's own turn removes it. At least one of the two
  is given. `--all` exists for the launcher: a `cmd` line holds 8,191 characters, which 50
  full paths can pass, and listing a folder is logic we keep in the exe, not in a bat.

  **Each as if run alone.** Each export has its own verdict and its output written or not,
  every output goes inside `-o` or beside its input, and nothing an export does stops the
  run. The exit code is the highest any export's run gave.

  **What refuses the whole run**, before any export is read: an argument error that is the
  same for all (a flag, a savefile that does not read as a save), and an `--all` folder that
  is missing, is no folder or holds no export.

  **What refuses one export alone**, its block saying so:
  - a missing export path;
  - a save lacking one export's team;
  - a flag that export's kind refuses (`--savefile`, `--loose` or `--coverage` given with a
    referee export among team exports);
  - a flag that export's kind requires and the run lacks (neither `--savefile` nor `--loose`
    with a team export among referee exports);
  - an abort of its own (its output folder exists, one of its files cannot be written).

  **What is printed.** A run of one named export, without `--all`, prints as today. Every
  other run, `--all` over a folder of one export included, prints:
  - before each export's findings, the line `Upgrading <the input's file name>`, with an
    empty line before it except before the first;
  - a refused or aborted export's reason as the line `error: <reason>`, the last of its
    block;
  - after the last export, an empty line and the tally `Upgraded: <n>. Not upgraded: <m>.`,
    an export counting as upgraded when its own exit code is 0.

  These lines go through the event stream as the findings do, so they keep their order and
  reach a run's log file. We do not make them findings: a finding line names its export by
  itself, and these lines exist for the two that do not (the verdict, whose scope is the
  run, and a refusal).

  **The launcher** starts the exe once, with `--all` over `old_exports`, so the tally and
  the naming lines are the exe's and the launcher counts nothing. An export named twice, or
  named and also inside the `--all` folder, gets no special case: once its first turn has
  written the output, its second is refused because the output folder exists.
- **The progress line.** On a terminal, `upgrade` draws the console's
  progress line as `compile` does, counting the input exports and the files written
  (`plans/core/architecture.md` "The console's progress line and run summary"). A run of
  several exports counts them all from its first frame (`n/N exports`): the command
  announces how many it will run before the first starts, because it starts each when its
  turn comes, and every export counts as done once, whatever its outcome. The parts counter
  is the current export's: it leaves the line when the next export's turn starts, and a
  refused export, which writes no file, shows none.

## GUI

The tool view upgrades every old export in the `old_exports` folder beside the app into the
`exports` folder the Compiler reads, as `quick_upgrade.bat` does. Its draft is
`docs/prototypes/tools/export_upgrader.js` (`core/gui.md` "Prototypes before
implementation"), whose second version the maintainer accepted. It reuses the Compiler's
widgets: the grid, its cells and colours, the legend, the toolbar, the detail window and the
log panel are one shared widget, so the two tools read alike, and an upgrade's result is
checked in the next tool exactly as it was shown here.

The view takes no path of its own (no drop target to fill, no Output into field), because
the bundle's folders already say where old and new exports live, and the one launcher
members use (`quick_upgrade.bat`) reads the same two.

- **The folder.** On opening, the view reads `old_exports` and lists every export in it (a
  folder, `.zip` or `.7z`), each read as the upgrade reads it up to the write: its cells
  show what the run would do and what stops it, before anything is written. The folder is
  watched as the Compiler watches `exports`, so an export copied in appears and is read.
  A folder, `.zip` or `.7z` dropped on the view is copied into `old_exports`, as the
  Compiler's drop copies into `exports`, and refused with a notice when `old_exports`
  already holds that name. An empty folder shows `No old exports found`, the folder's path
  and the drop's invitation.
- **The arguments**, one row under the title bar:
  - **Save**, prefilled with the newest save `pes_savefile::discovery` finds for the
    selected PES version (empty when it finds none), with a Browse button. It follows the
    selected version's discovered save until the member edits or picks one. One save serves
    every export, since a cup's teams come from one save. The field turns red while any
    listed team is missing from it, its tooltip naming those teams, and each such export's
    row says so (below).
  - **Upgrade without the save**, the `--loose` switch, its tooltip saying what loose does
    (each boots or gloves folder goes to the player at its ID's position in the team's old
    block of 25, and no player's looks are read). Ticked, it disables the save field, keeping
    its value for when it is unticked, because a member trying both ways would pick the save
    again, and an export refused for its team is read again with no save.
  - `into` and the `exports` folder's path, as text: where every upgraded export goes.
  - At the row's right end, the **look toggle**, `Files | Grid` (below).
- **Two looks, one set of cells.**
  - **Grid**, the default from five exports on: the Compiler's grid (`libs/export_grid`,
    `plans/team_compiler/gui.md` "GUI: the live-validating team grid"), three lines an export.
    The **Face** line holds a cell per player slot (his face folder, to `Players\NN -
    Name`), then the portraits (`n/N`) and a cell per kit (its two colours as the
    Compiler's swatch).
  - The **Boots** and **Gloves** lines hold, under each player, the boots or gloves
    folder the save has him wear, by its four-digit ID. A folder one player wears moves into
    his own; one two or more wear stays shared, with a link in each wearer's folder, and its
    cell stands under each wearer.
  - Right of the portraits, the team's files: `Note` (its colours and notes), `Logo`,
    `Common` and `Collar` on the Boots line; on the Gloves line one cell counting what the
    upgrade leaves out (`3 out`), in place of a list.
  - The team's label cell, at the row's left, carries the export's own state; its detail
    window gives the old name and the upgraded one (`exports\<name>`, EU-ROOT's naming).
  - **Files**, the default under five: the explanatory page's map (`docs/web/
    upgrade_explained.html`) drawn for each export. A box stands per folder or file of the
    old export on the left and per folder or file of the upgraded one on the right, with a
    line from each old box to every place it goes, in the colour of what it carries:
    players, boots and gloves, kits, team files, and dashed for what is not carried or for
    information read (the Note's kit colours into each kit).
  - A box under the pointer keeps its own lines and the boxes at their other ends, and dims
    the rest. The dimming ends 120 ms after the pointer leaves, as on that page, so going
    from box to box does not flicker.
  - Each export's block opens with its two names, each under its column: on the left
    its label and its old name (`old_exports\<name>`), on the right its upgraded name
    (`exports\<name>`) and its `Full`/`Midcup` combo. An export that cannot be read
    writes nothing, so its right side holds the reason and its action instead. The labels
    `Old export` and `Upgraded export` are one row that stays above the exports as they
    scroll, and a legend of the lines' colours stands above the map.
  - **The toggle** picks either look whatever the count; until the member picks one, the
    count chooses. The pick lasts the session, because the right look depends on the day's
    folder.
  - A box's state is its cells' (the worst while any is unread or wrong, the run's once
    every one is through), so the two looks never disagree.
- **Cell states** are the Compiler's colours with the upgrader's words: `Not read yet`,
  `Reading`, `Ready`, `Warning` and `Error` (outlines), `Upgraded`, `Upgraded with warnings`,
  and `Not written` (the dashed red outline). The legend, above the grid, lists them as the
  Compiler's does.
- **An export with an Error is not written.** Its cells turn `Not written` once the run
  passes it, the Error's cell keeping its red outline so the eye finds what to fix, and the
  log gives `migration_not_clean` for it. The others are written. This is the upgrader's
  rule, a clean export or nothing, applied per export.
- **An export that cannot be read** is one line in place of its cells, in the error
  colour, with the action that resolves it beside the message, never in the rightmost
  column:
  - a team the save does not hold: **Browse…**, the Save field's own, since picking another
    save resolves it (ticking the loose switch does too);
  - an export already in the Studio format: **Move to exports**, which moves it into
    `exports` as it is;
  - an upgraded export already in `exports`: **Open the existing folder**, so the member
    sees what the earlier run made and renames or removes it. The path comes from the
    error's field, never parsed from its text, which a Windows path's `:` would break.
- **Full or Midcup** is a combo at each row's right (in Files, at the block's head, where
  no column title names it, so its entries read Full export and Midcup export),
  prefilled with the name's guess, its tooltip naming what decided it
  (`coverage_tag_chosen`'s `source`). It is held on the name's tag when the name carries
  one, since the CLI refuses the other there. The run passes `--coverage` for an export
  only when the combo differs from the guess, so the report's `source` stays the name's
  whenever the member left it alone. A referee export has no combo. The grid's column for it
  is titled **Export type** in the title row like the others, so it does not read as unused
  space on a row without the combo.
- **The detail window** is the Compiler's (`plans/team_compiler/gui.md` "GUI: the
  live-validating team grid", the detail window), opened and pinned by its rules. It opens
  when the pointer rests on a cell, a box, a label or a log line, a click pins it, and Esc
  closes it. A log line's click also outlines its cell or row. The cells, boxes and labels
  carry no tooltip of their own.
  - It shows the old paths a cell is read from and the path it is written to, its state, its
    findings, who wears a boots or gloves folder (and so whether it moves or stays shared),
    the list of what is left out for the count's cell, and **Open the old folder**.
  - It has no top box, since the upgrader has no 3D preview to keep the text below.
  - A label's window also offers **Copy messages**, the export's log lines.
  - A double-click opens the old path the cell is read from first: a folder, or a file's
    folder with the file selected. A label opens the old export's folder, and an archive
    export opens the archive's folder with the archive selected. The IDs and the left-out
    count's cells stand for no one path, so a double-click on them does nothing.
  - A box of the Files look stands for its cells as a cell does. Where a left box and a
    right box stand for the same cell, both show its pin.
- **A log line** is the Compiler's, `Severity - /team/ - code at <the old path>`, and acts
  for its cell as there; the finding's own text is the window's.
- **The toolbar** is the Compiler's:
  - **The strip.** Before any run it gives the counts
    (`13 old exports · 8 ready · 1 with errors · 3 cannot be read · 2 with warnings`).
    After a run it gives the run's line and only the counts that line leaves out
    (`Upgrade finished in 00:18 · 7 written to exports · 2 with errors · 3 cannot be read ·
    2 with warnings`): the exports written are the ones that were ready, and those with
    errors are the ones not written, so naming both would say each twice. During a run it
    reads `Upgrading · mm:ss · n of N folders written`. As in the Compiler,
    a click on `with errors` or `cannot be read` sets the log to Errors and one on
    `with warnings` sets it to Warnings. They are in the link blue only while the log is
    collapsed.
  - **The log** starts collapsed when, open, it would push some cells or boxes out of view,
    as the Compiler's does.
  - **Switch to the Compiler** stands left of Upgrade, always enabled but during a run. Once an
    upgrade has finished it takes the accent colour, the view's one blue button, until the
    next run starts, because the upgraded exports are already in the Compiler's folder and
    it is the next step.
  - **Upgrade** is a split button. Its arrow opens Open old_exports folder, Open exports
    folder and **Copy report** (the run's lines as the console prints them, for a member
    asking for help in the cup's thread). Upgrade is enabled once any export is ready,
    and during the run it reads Cancel.
- **The run** upgrades every ready export, in parallel, through the command's own code, so
  the view upgrades nothing the command line would refuse. The run takes the exports in the
  grid's order, a few folders at a time, so the cells change down the grid as their folders
  are written. The run ends with the notice `Upgraded N exports`, carrying Open exports
  folder.
  - **Cancel** stops the run. Exports already written stay in `exports`, and one being
    written leaves nothing, by the upgrade's own staging (never a half-migrated output).
  - The window's close during a run asks, as the Compiler's does.
- **Upgrading without the save asks first.** With the loose switch ticked, Upgrade opens a
  dialog before the run. The save is where the players' looks live (skin colour, build,
  strip style, the in-game faces) and the old exports hold none of it, so an upgrade
  without it carries none, and a compile of the result gives those players the default
  look. The dialog says that and the remedy (cancel, and pick the save the teams were made
  in). Its Upgrade stays disabled until `I understand the players' looks will be lost` is
  ticked, and Cancel returns to the view, the switch still ticked. It is asked on every
  such run, because each run's loss is its own. The CLI asks nothing: `--loose` is already
  the opt-in step 11 requires, and a prompt would stall a script.
- **A referee export** (its name's first word `refs`) takes no save: its row is read and
  written whatever the save field holds, and has no Full or Midcup combo. The draft does
  not show one yet (worklog 8.17b).
- No settings section: the tool has no settings (step 6: it reads no teams list).

## Dependencies

What exists of this is in `spec/export_upgrader.md` "Dependencies". The optional glTF
conversion (step 13) uses `model_convert`'s `ir_to_gltf` path, which makes `model_convert` a
build dependency; the runtime `--convert-to-gltf` flag only determines whether that path is
executed.

## Acceptance

What exists of this is in `spec/export_upgrader.md` "Acceptance", whose rules (format, the
defaults a scenario naming no version or export takes, the test data) the scenarios below
follow. Step 13's scenarios are written with Phase 7.

**Several exports at once** ("CLI").

```
EU-CLI-11  GIVEN two of EU-CLI-01's /co/ exports, a third export of another team, and a save
                 holding /co/ but not that team
           WHEN `upgrade` names all three with that save
           THEN the two are written, each block under a line `Upgrading <its file name>`, the
                third's block ends with an `error:` line refusing it for its team, the last
                line is `Upgraded: 2. Not upgraded: 1.` and the exit code is 2; WHEN one export
                is named alone THEN no naming line and no tally are printed; GIVEN an unknown
                flag THEN exit 2 before any export is read; GIVEN a path that does not exist
                named between two exports THEN its block ends with its `error:` line, the two
                are written and the exit code is 2; GIVEN a clean export, one whose upgrade
                ends with an Error and a missing path THEN the tally reads `Upgraded: 1. Not
                upgraded: 2.` and the exit code is 2
EU-CLI-13  GIVEN a folder holding two old export folders, one named in capitals, an old
                 export `.zip` and a file that is neither
           WHEN `upgrade --all <the folder>` runs
           THEN the three exports are upgraded in the order of their names, letter case set
                aside, each under its naming line, the other file is passed over without a
                line and the tally counts three; GIVEN a folder holding one export THEN its
                naming line and the tally are still printed; GIVEN the folder also
                holding the staging folder an interrupted run left, `co Full -
                Spring.upgrading`, THEN it is passed over without a line and `co -
                Spring` is upgraded; GIVEN one of the folder's
                exports also named THEN it runs first, and its second turn is refused, its
                output folder existing; GIVEN a folder that is missing, is a file or holds no
                export, neither an export nor `--all`, or two exports named with a savefile
                that is no save THEN exit 2 and no export is read
EU-CLI-14  GIVEN two exports named, the first's output folder already there
           WHEN `upgrade` runs
           THEN the first's block ends with its `error:` line, the second is upgraded, the
                tally reads `Upgraded: 1. Not upgraded: 1.` and the exit code is 3
EU-CLI-15  GIVEN two exports named, the second of a team the save lacks
           WHEN `upgrade` runs with the progress line drawn
           THEN the line's exports counter reads 0/2 from its first frame and 2/2 at the end,
                the refused export counted as done; the first export's parts counter is
                drawn while it runs and gone from the last frame, the refused one having none
```

**Phase 8 scope.** Phase 8 delivers the view ("GUI"). What a scenario observes is what a frame
draws, through the headless harness (the frame's AccessKit nodes and painted shapes; a dropped
path as egui's dropped-files input; a click as press and release frames), the output written,
and the notice published. `manual` marks only what needs a native dialog.

The inputs are Phase 6's fixtures, copied into an `old_exports` folder under the test's own
folder, with an `exports` folder beside it. The discovered save is one placed under a
Documents folder there.

What the upgrade writes is the Phase 6 scenarios'. These scenarios observe that the view
reaches the same code with the arguments shown and shows its report.

```
EU-GUI-01  GIVEN the selector at PES 21, EU-CLI-01's `co - Spring` export in old_exports and a PES 21
                 save holding /co/ under the test's Documents folder
           WHEN the view is drawn
           THEN the Save field holds the discovered save, the loose switch is off, the look is
                Files (one export), /co/'s block names its old and upgraded names, and once its
                reading ends every box is Ready and Upgrade is enabled; GIVEN no discovered
                save THEN the field is empty and Upgrade disabled until a save is picked or
                the loose switch ticked
EU-GUI-02  GIVEN EU-GUI-01's folder and save, the export's Other/ holding one file
           WHEN Upgrade is clicked
           THEN exports holds the folder `upgrade --savefile` writes (EU-ROOT's naming), the
                count cell reads `1 out` and its detail lists the Other/ file with
                leftover_dropped, the log shows the report's lines as the console prints them,
                the notice `Upgraded 1 export` with Open exports folder is published, and Open
                the Compiler takes the accent colour; WHEN that is clicked THEN the Compiler's
                view is active
EU-GUI-03  GIVEN five old exports, one of them /co/ whose Note holds EU-ROOT-06's unreadable colour entry
           WHEN the view is drawn and Upgrade clicked
           THEN the look is Grid, /co/'s Note cell is outlined red before the run, the other
                four are written, /co/ is not (its cells Not written, the Note's outline kept)
                and the log gives migration_not_clean for it; WHEN Files is picked THEN the same
                five show as maps and stay so with a sixth export added
EU-GUI-04  GIVEN two exports, one of a team the save does not hold, one whose upgraded folder
                 already exists in exports
           WHEN the view is drawn
           THEN each is one line in the error colour: the first's names the team and the Save
                field is red, its tooltip naming the team, and Browse… beside its message
                opens the same file picker as the Save field's; the second's carries Open
                the existing folder beside its message; Upgrade is disabled; WHEN the loose switch
                is ticked THEN the first is read and Upgrade is enabled
EU-GUI-05  GIVEN EU-GUI-01's folder, the loose switch ticked
           WHEN Upgrade is clicked
           THEN the loose dialog opens and no run starts, its Upgrade disabled until its box is
                ticked; WHEN Cancel is clicked THEN the view is back, the switch still ticked;
                WHEN the box is ticked and the dialog's Upgrade clicked THEN the output is the
                one `upgrade --loose` writes for that export
EU-GUI-06  GIVEN EU-GUI-01's folder, its name carrying no tag
           WHEN Midcup is picked in its combo and Upgrade clicked
           THEN the output is `upgrade --coverage midcup`'s and the report's source is the
                member's choice; GIVEN the combo left alone THEN no --coverage is passed and the
                source is the name's
EU-GUI-07  GIVEN a referee export in old_exports and no save picked
           WHEN the view is drawn and Upgrade clicked
           THEN its row has no Full or Midcup combo, Upgrade is enabled, and the output is
                EU-REF-01's (its players.txt from refs.txt)
EU-GUI-08  GIVEN EU-GUI-03's five exports and the upgrade in flight
           WHEN Cancel is clicked
           THEN the run stops, every export exports holds is whole (as its own `upgrade` writes
                it) and none is partial; GIVEN another tool's view active and the window's
                close requested during a run THEN the Export upgrader's view is active with
                the Compiler's prompt
EU-GUI-09  GIVEN EU-GUI-01's folder
           WHEN a folder holding an old export is dropped on the view
           THEN it is copied into old_exports, listed and read; WHEN a folder of the same name
                is dropped THEN nothing is copied and a notice names it
EU-GUI-10  manual  GIVEN the view open
                   WHEN the Save field's Browse button is clicked
                   THEN the native picker opens in the selected version's save folder and the
                        picked file fills the field
EU-GUI-11  GIVEN EU-GUI-03's exports, one with an Error on its Note
           WHEN the pointer rests on a face cell, on a Files box and on the Error's log line
           THEN the detail window opens on each as the Compiler's does (TC-GUI-09, TC-GUI-15),
                and a click pins it as there (TC-GUI-24); WHEN a face cell is double-clicked
                THEN the face folder of the old export opens in the file manager; WHEN Esc
                is pressed THEN the window closes; GIVEN the label's window of the export with
                the Error THEN it offers Copy messages, which puts its log lines on the
                clipboard
EU-GUI-12  GIVEN the strip's counts with the log collapsed
           WHEN `cannot be read` is clicked
           THEN the log opens on Errors; the counts are in the link blue while the log is
                collapsed and in the strip's grey while it is open
```

**0.0.2-ux1 scope.** The progress line ("CLI"). Observed through the console printer's
counters with the line drawn (the console's own test harness), on EU-CLI-01's export.

```
EU-CLI-12  GIVEN EU-CLI-01's /co/ export and its save
           WHEN `upgrade` runs with the progress line drawn
           THEN the line's exports counter reads 0/1 from the first event and 1/1 after the
                output's validation, never a total of 2; its parts counter appears once the
                files are planned, its total the number of files the output holds, and
                reaches it; the finding lines on stdout are those of a run without the line
```

**8.36c scope.** Step 8's base copy (above, "What it does"): a record copying another
player becomes the folder's `<id>.copy` link. 101334 and 7511 are copied ids real cup saves
hold.

```
EU-SET-12  GIVEN a save whose records copy another player: slot 05's 101334, with no face
                 folder and wearing custom boots 430 from "k0430 - Crocs" (EU-PLY-02's); slot
                 06's 7511, whose face folder holds a face model; slot 07's 101334, whose face
                 folder holds the old ingame_face marker; and slot 08's his own id, with a face
                 folder
           WHEN the export is upgraded with that save
           THEN Players/05 holds 101334.copy and boots.fmdl and no ingame_face.toml;
                Players/06 holds its face model and no .copy, and base_copy_beside_face is
                reported once, naming that folder and 7511; Players/07 holds 101334.copy and
                no ingame_face.toml; Players/08 holds no .copy; and the shared validation
                ("Verification") reports no copy_link_ finding, the run ending
                migration_clean
EU-SET-13  GIVEN EU-SET-12's export
           WHEN it is upgraded with --loose
           THEN the output is written, Players/06 holding its face model as in EU-SET-12,
                no player folder holds a .copy, and no base_copy_beside_face is reported
```

**0.0.2-logo scope.** `logo_missing` from the shared validation (above, "What it does").

```
EU-MDL-43  GIVEN an old export "co - Spring" with faces, kits and no Logo folder, on a PES 21
                save, one of its players with no portrait
           WHEN it is upgraded, then upgraded again with --coverage midcup
           THEN the first run reports logo_missing (Error) on "co Full - Spring", writes
                nothing and ends migration_not_clean with exit code 1; the second writes
                "co Midcup - Spring" with no logo file, reports no portrait_missing and ends
                migration_clean; GIVEN a Logo folder added THEN a run as Full reports
                portrait_missing (Warning) on that player, writes "co Full - Spring" and ends
                migration_clean
```
