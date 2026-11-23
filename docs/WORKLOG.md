# 4cc Studio — Worklog

Where the implementation is and what comes next. The plan (`docs/plans/`) holds the *why*;
`DECISIONS.md` holds choices made where the plan was silent; this file holds *where we are*.
Never duplicate rationale here — link to the plan section instead. Procedure for using this file
is in `AGENTS.md` ("Working documents").

---

## Current status

**Release 0.0.2, the VGL preview, is out** (`plans/core/development_plan.md` "The /vg/ League
27 (VGL)"): tagged `v0.0.2` on `aab124dc` and published by the maintainer 2026-11-18, once CI
and the Release run had passed. It went out without Select all similar, the Carbon rounds and
the PES 2019 check (DECISIONS 2026-11-16 "0.0.2 is cut without Select all similar, the Carbon
rounds and the PES 2019 check"), and the no-stretch slices moved to after it (2026-11-16 "The
no-stretch slices move to after 0.0.2"). Card `poc5-in-game` stays open.

**Buildable drafts**: the Save editor's, the Compiler's and the Export upgrader's, so their
built UI follows them. Every other tool's draft is not buildable (the Ruleset editor's), and its
built UI is not worked on (`RUNBOOK.md` "How the work runs"). Only the maintainer moves a tool
here.

**Now, a detour: PES 2014 saves for Akko League VII.** The Save editor learns PES 2014 on the
long-lived branch `pes14`, which never merges into main (DECISIONS 2026-11-19 "PES 2014 support
lives on its own branch"). The branch's `docs/plans/pes14.md` holds its plan, decisions and
steps. Its slice 2c reads the game's own texts for its tooltips (`string/all.str`) through
the lib crate `pes_strings` (`plans/libs/pes_strings.md`), whose reader is built on main
(2026-11-20) and merged into `pes14`; its writer waits for the editors below. Two tools will edit those files, both low priority and plan-only for now: the String
editor (replacing the PES2021 STR Editor) and the Gametips editor (replacing the gametips
scripts in `C:/Data/4cc/0Tools/Tips/`). Their plans are stubs, to be finished from a lookup
run's findings (`4cc-studio-temp/str_re/out/findings.md`).

**The detour is over**: Build 2, `0.0.2-pes14.2`, was published 2026-11-21. **The main goal
now is item 6, the Compiler's view for the VGL organizer** (the maintainer, 2026-11-21), its
draft looked at again first; the Export upgrader's view may follow in a later build. The
other items run beside it. **Ahead of item 6:** the `pes14` branch's step 12, a whole PES
2014 team exported and imported as Team TOML, which Build 3 carries (the maintainer,
2026-11-22): `0.0.2-pes14.3`, tagged 2026-11-23 on `48ab28bc`, its Release run passed and its
draft checked, for the maintainer to publish; running sidekicks and jobs are not stopped for it. **After the detour**, in this order:
1. The Nation list: done (merges `467790ab`, `1b9c5f27`; SWE-2 round S1n). It opens on the
   held nation, a typed name picks as an ID does, Escape puts the old nation back, and one
   opening is one undo entry; the General info review's fixes (rulings "geninfo A1") with it.
   Seen in the release build on the PES 2021 fixture (2026-11-21:
   opened on 231, `ital` picked 215, Escape put 231 back with nothing left to undo).
2. 0.0.2-nostretch's slices: done for every built view (the Save editor, the Ruleset
   editor); the Compiler's and the Export upgrader's views take the rule in step 8.10.
3. Earlier sessions in Report a problem: the repro bundles issue's slice (5), in slices:
   - [x] the crash monitor's start tried once more (ST-RUN-13): merged 2026-11-21, SWE-2
     round S1rt; Astra round A-s5b r1 with 5b's (below);
   - [x] 5a the session folder's writer and reader in `studio_core::repro`, called in
     process (ST-REP-06 to 08): merged `08d65aa6`, SWE-2 round S1s5a; Astra round A-s5a r1,
     2 accepted (a marker line ending each journal block, and a test of several files),
     fixed with 5b-i;
   - [x] 5b-i the monitor's half: it writes the session from the frames on its standard
     input and ends it (ST-REP-06, 07): merged `d496aebd`, SWE-2 round S1s5b1;
   - [x] 5b-ii the window's half: the recorder's queue, the thread sending each message down
     the pipe, the window's own writer when no monitor started, a caught panic ending
     `crashed` (ST-REP-10's first cases, ST-REP-11): merged `e27d9f2b`, one rework, SWE-2
     round S1s5b2 (3 accepted, the loop ended); with it the tests for the checkpoint's seven
     mutants in 5a's writer;
   - [x] 5b2 a new monitor after the pipe breaks, writing on into the same folder (ST-REP-10's
     last cases): its writer adopts the folder, cut back to its last marker, the next
     indexes from its rows (DECISIONS 2026-11-21 "a new monitor's writer adopts the session
     folder"): merged `51643014`, one rework, SWE-2 round S1s5b2b (3 accepted, the loop
     ended);
   - [x] the retry's and 5b's Astra round A-s5b r1 (4 of 7 accepted, the loop ended: a close
     read as a crash at the next start after an idle pipe break, a warning left by a monitor
     that served on its second start, two test gaps): the fixes merged `ba254801`, SWE-2
     round S5b r1 over them (the loop ended);
   - [x] 5c the Save editor's saves kept decrypted and rebuilt on reading (SE-REP-07): packed
     and compressed by the writer, rebuilt through a function the Save editor gives the
     bundle reader (DECISIONS 2026-11-21 "a session keeps each save packed and
     compressed"): merged `53398fdc`, one rework, with the tests for the checkpoint's two
     missed mutants in `repro.rs` and `session.rs`. SWE-2 round S5c r1's fixes merged
     `9d9e1786` (that loop ended). Astra round A-s5c r1 ran first, Carbon's login being spent
     until its reset (RUNBOOK "Review rounds": the next loop runs meanwhile): 3 accepted,
     which ends Astra's loop (rulings "A-s5c r1"). Then:
     - [x] `save-editor unpack-session <session.zip> <folder>` writes an earlier session's
       saves back as files, since no shipped command read a `.packed` save (SE-REP-08):
       merged `3dba53b7`, SWE-2's slice; the report readme names it;
     Owed, not yet briefed:
     - the packed round trip tested with a non-empty logo, identifier and serial;
     - `frames_until` back to CONTRIBUTING's 120 s, and the comparator test that ran out at
       120 s in the 2026-11-21 checkpoint made not to wait on a loaded worker;
     both SWE-2 slice N2, one rework (the overlap test's comparator screen), merged
     `7082df7f`; then a SWE-2 subloop over the fixes, and Carbon's loop (queued);
   - [x] 5d the dialog's "Include earlier sessions" box and the help page's line (ST-REP-09,
     12), the draft's ending line settled (DECISIONS 2026-11-21 "an earlier session with no
     `[end]` reads "ending unknown""): merged `db2900cd`. Seen in the release build beside
     the draft, box clear and ticked (2026-11-21). SWE-2 round S5d r1 (4 accepted, the loop
     ended): its fixes and the maintainer's ruling on the list's colours (DECISIONS
     2026-11-21 "Report a problem's earlier sessions in the regular colour") merged
     `5583611a`, and seen in the release build beside the draft, box clear and ticked
     (2026-11-21): the earlier sessions in the regular colour, the `~` line weak.
4. 0.0.2-similar, Select all similar players: slices 1a and 1b merged (the group, the button
   and its window), 1c and slice 2 to build (code map `.tmp/similar/code_map.md`). Review rounds owed: every Carbon round a 0.0.2 step still
   owes (Report a problem has had its whole review).
5. 0.0.2-pes19, and with it the readme's PES 2019 claim.
6. The Compiler's view in the build, unpolished, for the VGL organizer (the maintainer,
   2026-11-20; `plans/core/development_plan.md` "Phase 8: GUI"): step 8.10's grid first,
   showing which of an export's components has an error or a warning and what it says;
   the row actions and the finish in later builds. The draft was looked at again on 2026-11-22:
   a cell's detail window now opens on hover and pins on a click, and long log lines wrap; a
   log line acts for its cell (`plans/team_compiler/gui.md`). The maintainer stamped that
   draft on 2026-11-22, and the Export upgrader's draft took the same window, log and counts.
   The maintainer approved both drafts on 2026-11-22, the Compiler's with the logo and
   portrait findings, the upgrader's once each export's two names stood under their columns.
   The Sizes colouring and sorting were drafted and stamped the same day
   (`plans/team_compiler/gui.md` "Sizes and sorting"). The view is built in ordered slices (Opus), each landing and reviewed before
   the next is briefed:
   - [x] 1a (2026-11-22, merge `304822e4`, Opus, two reworks) the grid, read-only, in the
     new lib `export_grid`, filled by `check`'s validation when the view opens and after each
     Compile: rows, label, ID, player, portraits and kit cells, check states, the legend
     (TC-GUI-04 without its watcher part, 20, 21). Its look: checked by the lead beside the
     draft (the portraits count left out a dropped player's portrait, fixed in `bbe765a6`),
     now the maintainer's on the tasks page (card "The Compiler's grid, its first slice"). The
     shipped build still greys the Compiler and the Export upgrader (`CLI_ONLY`,
     `crates/studio/src/main.rs`); only the VGL organizer's hand-given build opens them
     (merge `e92af4a0`, `plans/core/gui.md` ST-SIDE-11). When they open for every member is
     the maintainer's call
   - [x] The VGL organizer's build `0.0.3-vgl27.1` (2026-11-22, branch `vgl27`, `c2db362c`):
     the version opens the two tools, and the update check's tag prefix `vgl27-v` keeps main's
     releases from being offered to it (`RUNBOOK.md` "The VGL organizer's build"); packed in
     the `vgl27` worktree's `target/dist/` and checked in its built window (both tools
     clickable, the Compiler's grid opening, the upgrader on its placeholder line), handed over
     by the maintainer as it is
   - [ ] The next VGL build, `0.0.3-vgl27.2`, once the Compiler's slices 1b and 1c have landed
     (the maintainer, 2026-11-22), 1c-iii's Upgrade DpFileList button included (2026-11-23):
     `vgl27` merges main, raises the version, is packed. Built 2026-11-23: `vgl27` merged the
     gated `88040bcc` (`76e46277`, its legacy-ID tests moved onto main's test helpers), raised
     to `0.0.3-vgl27.2` (`de9c9ccb`), packed in the worktree's `target/dist/`, and checked in
     its built window (the version on the sidebar, the four tools, the Compiler's grid and
     split Compile button, the upgrader's placeholder line). It is handed over once item 8 is
     answered (`maintainer/questions.html` "Upgrade DpFileList in the organizer's PES 2019
     build"), since that answer decides what the button does on PES 2019
   - [x] VGL27's legacy mode off main, onto `vgl27` alone (2026-11-22, DECISIONS 2026-11-22):
     main's merge `8cc425a3` (Opus; the lead dropped three tests of the old names and the
     draft's `IDs` cell), TC-LEG-01 to 05 and EU-SET-15 to 17 withdrawn, and a CHANGELOG
     `### Important` line telling members to delete the `legacy_ids.toml` 0.0.2 wrote. `vgl27`
     merged main and reverted the removal (`a0c24750`), its gates green with the mode back
   - [x] 1b: the detail window, in two slices, since its pictures need decoding and a cache
     of their own:
     - [x] 1b-i (2026-11-22, merge `63ecaae8`, Opus): hover, pin, Esc, double-click, Open
       folder and Copy messages in `export_grid`, the top box drawn empty (TC-GUI-09, 24, 25).
       The lead dropped the portraits cell's one-finding fallback, which no export reaches,
       and offers Copy messages only on an export with a Warning or an Error (DECISIONS); the
       built window beside the draft found the Effect line weak, fixed (`4fb5d8ec`)
     - [x] 1b-ii (2026-11-23, merge `e3ca6a56`, Opus, rework SWE-2): the top box's pictures
       (TC-GUI-31, 32), read off the UI thread, at most 256 texels; a box picture never
       enlarged (kept by the maintainer: every real logo is 512 or 256). Release build beside
       the draft (scene `.tmp/look_1b2/app`): kit, player and portraits boxes match; three
       decisions went to `maintainer/ui_decisions.html`, all kept, the empty archive boxes
       for `.7z` only. A `.7z`'s pictures wait for the watcher-and-cache slice
     - [x] 1b-iii (2026-11-23, merge `2eeccf60`, SWE-2): a `.zip` export's pictures read from
       the zip, each from its own entry; a `.7z`'s boxes stay empty (TC-GUI-31's zip clause).
       Its review rounds go with 1c-ii's
   - [x] 1c: the toolbar's strip and its counts, the log's lines linked to cells, the log
     collapsed when it would hide rows, the Upgrade DpFileList button → verify: TC-GUI-15,
     26, 27, 33. The pure move of the
     view's tests came first (`3aa75bac` on `sk/grid1c`).
     - [x] 1c-i (2026-11-23, merge `185d9e01`, Opus): the strip, its counts and the log's
       start (TC-GUI-26, 27). The lead counts a row's warnings by the colour its cells show,
       as the draft does. The release build beside the draft (scene `.tmp/look_1b2/app`,
       built in `look_grid`) found two differences, fixed in `99d51f81`: an export the check
       drops whole counts as skipped, and a collapsed log shows no filter chips. The strip
       cut short in a narrow window cannot be captured (a narrower window scales the whole
       UI); its unit test covers it
     - [x] 1c-ii (2026-11-23, merge `9b7f7007`, Opus): the log opens with the check's
       findings, each line acting for its cell: hover opens the cell's window, a click on a
       Warning or an Error outlines the cell, scrolls to it and pins it (TC-GUI-15). The lead
       made `cell_places` allocate nothing, since the strip walks every cell each frame. Its
       built-window look was taken with 1c-iii's (below)
     - 1c-iii: the Upgrade DpFileList button beside Compile and its dialog (TC-GUI-33;
       `plans/team_compiler/gui.md` "Toolbar", `plans/team_compiler/pipeline.md` "DpFileList
       upgrade"; the draft's `compiler/main?dpfl=outdated`), over the CLI's `upgrade-dpfl`,
       with the new `dpfilelist_cpk_deleted`. On PES 2019 it waits for item 8. SWE-2, in two
       parts:
       - [x] 1c-iii-a (2026-11-23, merge `7a488e81`, SWE-2): `upgrade.rs`'s preview as data,
         the deletions the member ticked (`dpfilelist_cpk_deleted`), the catalog's texts
       - [x] 1c-iii-b (2026-11-23, merge `bdfccc40`, SWE-2): the button and the dialog over
         them (TC-GUI-33), the button shown after a run's preflight, as briefed before
         1c-iii-c's decision
       - [x] Review round S1's fixes (2026-11-23, merge `3c47d77a`, SWE-2): a failed delete
         no longer stops the upgrade, the dialog names a replacement only when it makes one,
         a `.zip`'s cells are deep-checked, and tests that passed on wrong code were tightened.
         Round S2 (SWE-2) reviews 1c-iii-b and these fixes
       - [x] 1c-iii-c (2026-11-23, merge `393312d2`, SWE-2, one rework with round S2's
         Compiler fixes): the list checked with each check of the exports, its findings in
         the log before any Compile; Compile disabled while the list is outdated; the button
         in the selection blue, following the check, so an upgrade that only writes
         placeholders also takes it away
       - Round S3 (SWE-2) over 1c-iii-c, the selection edge and round S2's fixes: 7 accepted,
         one of reach (a), among them the help window's search hit drawn in the drafts' green
         (DECISIONS 2026-11-23)
       - [x] Review round S3's fixes (2026-11-23, merge `35184122`, SWE-2, one rework for a
         shared test helper): a check of the exports that ends after another was asked for no
         longer shows its rows, and the help window's search hits are filled in green. Round S4 (SWE-2)
         over them: 2 accepted, both doc comments that state a condition wrongly, so the
         rounds end here; the lead fixes them once `sk/looks_s3` merges
     - [x] The selection's edge on every blue fill (2026-11-23, merge `749dd2ae`, SWE-2, one
       rework with round S2's test fixes): a picked row drawn through `widgets::row` and the
       Save editor's selected player row draw the `#1e7a9e` edge at rest
     - [x] Report a problem keeps its "Saved to" line's room while hidden, so its buttons do
       not move when the line shows (2026-11-23, merge `cb4b3fbd`, SWE-2, landed first time);
       no changelog line, a spacing change
     - [x] The built window beside the drafts (2026-11-23, merges `b174f18b` and `29bad046`,
       SWE-2, three reworks; scene `.tmp/look_s3`, its game folder a fake PES 2017 install
       holding the old list): the log's lines, the Upgrade DpFileList button, the picked rows'
       edge, Report a problem's kept room and the help search's highlight green matched. The
       differences found are fixed and recaptured: the upgrade dialog's content is 460 wide
       and its delete list monospace; a help jump scrolls the hit's whole row or paragraph to
       the pane's top; the help tree's entries and search results span their pane, a long
       result cut with an ellipsis; the Save editor's selected row shows its edge on all four
       sides. Found in passing and fixed: a catalog row's search result showed its Markdown marks
   - The kit cells' swatch (TC-GUI-10), a strip along the cell's foot (DECISIONS 2026-11-23
     "a kit's menu icon pattern is a strip along its cell's foot"), in slices:
     - [x] a `color_tools::icon`: the 24 menu icon patterns as shapes over any rectangle
       (2026-11-23, merge `0e9ba0e0`, SWE-2, landed first time; reviewed with c)
     - [x] b a kit cell carries its menu icon and colours when the check kept the kit and its
       `colors.txt` lists two (2026-11-23, merge `057c55db`, SWE-2, landed first time;
       reviewed with c)
     - [x] c the grid paints the strip under the label (2026-11-23, merge `a19e622d`, SWE-2,
       landed first time). Captured beside the draft (scene `.tmp/look_sw`, ten kits on ten
       icons): the strip's place and size match; the label's glyphs stand 1 pixel lower.
       The stretched pattern is a `ui_decisions.html` card
     - SWE-2 round SW1 over a, b and c (two parts): 5 accepted, none of reach (a), so the
       rounds end here; among them, a referee export's kits no longer carry an icon (the
       compile writes none for them). The fixes merged `356e1591` (SWE-2, the lead moving
       two test samples off the edge they were to catch)
     - [ ] d a compile's colours worked out from the texture, and the base `UniColor.bin`
       colours for a kit the check dropped or whose compile failed (a failed kit keeps its
       listed pair today, though its entry is never written). Its compile half waits on a
       run updating the cells (TC-GUI-12)
   - [ ] One paid round (tier 2, Carbon) over slices 1a to 1c and the swatch, whose SWE-2
     loops have ended. Round 1 ran on `5baf8835`: 17 accepted, 10 of them a real input
     whose outcome changes. Its fixes are three SWE-2 slices: A (the check's rows) merged
     `0fc94490`, B (the detail window and title row) merged `76bcad35`, and C (the settings re-check and the Upgrade
     dialog, one rework) merged `d3e7be78`. Carbon round 2, over the grid and the Compile
     menu together (at `ca8559a1`): 11 accepted, two of them a real input whose outcome
     changes (the dialog's two lists sharing one scroll, a game started while the window
     is idle leaving the old state drawn), so a round 3 is owed. Their fixes are three
     SWE-2 slices, merged 2026-11-23: G1 (the dialog, the settings re-check, the Compile
     menu's tests) `4f5cbe58`, G2 (the layout of duplicate kit folders and referee rows)
     `fd84b216`, G3 (the archive tooltip, the menu's 2 px gap, the shared widgets' tests)
     `4ee3aa5b`. SWE-2's round over the three, 2026-11-23 (rulings "S-g1 r1" to "S-g3 r1"):
     4 accepted, none of reach (a), which ends the loop; their fixes, slice G4, merged
     `2ce16798`. Carbon round 3 over the same surface, at `2ce16798` (rulings "C-grid r3"):
     11 accepted, four of them a real input whose outcome changes (Esc under a dialog unpins
     the hidden detail window, a double-click under the window opens nothing, Help opens
     behind it, a refused check's line stays in the log), so a round 4 is owed. The lead's
     draft text: the dialog's counts at one and the archive tooltip. Its fixes are two SWE-2
     slices, both merged: G5a (`export_grid`'s detail window, the upward menu's test),
     `daab1034`, and G5b (`team_compiler`: the log, the dialog, the help links, the
     citations), `51cab105`. Due: their SWE-2 loop, then Carbon round 4
   - [ ] The Compile button's mode list, Open output folder and Copy report (TC-GUI-11, and
     TC-GUI-33's clause that Compile for sideloading stays enabled while the DpFileList is
     outdated; DECISIONS 2026-11-23 "the Compile menu's Copy report copies the whole log").
     Merged `997c2e09` (SWE-2, one rework for three untested paths; the lead shared a test
     closure). Its SWE-2 round found 3 to fix, which ends the loop: a picked sideload mode
     stays runnable after a switch to PES 15 or 16, and two tests miss a likely wrong
     version rule and the menu's place. Their fixes merged `f63bf2aa` (SWE-2, no rework). Its
     Carbon round is part of the grid's round 2, whose surface holds the toolbar. Its look,
     2026-11-23 (release build at `ca8559a1` beside the draft, `.tmp/look_menu/`): the menu,
     the closed arrow and the sideload pick match, but for one difference: the menu sat
     flush on the arrow, where the draft leaves 2 px (SB1-11). G3 gave it the gap
     (`4ee3aa5b`), seen 2026-11-23 in the built window beside the draft: 2 px on both,
     the right edges aligned. The draft's menu had stood 3.5 px past the arrow, its width
     measured while the window's edge squeezed it; fixed there and in the Export upgrader's
     More menu.
     The column toggle the draft lists comes with its later slice
   - Then the row actions, the ID cell's editing and its tooltips (each variant's, as the
     draft gives them), the
     watcher and cache, Sizes and sorting, and the rest of the toolbar
     (`plans/team_compiler/gui.md` "Toolbar"): Launch PES, the writability badge and the
     compile column's toggle. The log's lines also owe the plan's
     `Severity - /team/ - code` form (`plans/team_compiler/gui.md` "A message's log line"),
     their wrapping under that head (ST-LOG-05), and Open folder in a line's context menu.
     The watcher and cache's slice also owes TC-GUI-04's `.7z` order: 1a's check reads a
     `.7z` whole at once, where the scenario wants its entry table first and its deep check
     after the folders and zips, within the memory budget (a `.zip`'s deep check, as built,
     is right).
   - Slices of their own beside the view: 0.0.2-logo, `logo_missing`, a
     `Full` team export with no logo is an Error, and `portrait_missing`, on PES 2017 to 2021
     a player with no portrait is a Warning (`plans/aesthetics_export/object_model.md`
     "Validation semantics"; `plans/team_compiler/messages.md`; `plans/export_upgrader.md`
     "What it does"):
     - [x] a — 2026-11-22, `ee108bbf`: both findings in `aesthetics_export`'s validation and
       both catalogs, `installed_portrait_slots` `None` at every caller; 172 tests and five
       fixture archives given the logo and portraits they now need
     - [x] The shared export builder (the open issue "Every new check on what an export must
       hold breaks every test"). `aesthetics_export::test_exports::missing` lists the files an
       export lacks to be clean, one rule per requirement: some content, a `Full` export's
       logo, a portrait per player from PES 2017. A test of one check turns its rule off with
       `without`. In memory it runs through `TestExport` and the `report`/`resolved` wrappers,
       on disk through a sandbox's `complete`. The real bytes come from
       `team_compiler::test_files`, and the old Red export's rules live in
       `export_upgrader::test_exports`. Each slice proves itself with a dummy requirement: a
       new check alone breaks the slice's tests, and with its rule in `missing` only the
       builder's own tests fail:
       - [x] 1 (2026-11-22, merge `4382ae18`, Opus) the builder, and `team_compiler`'s unit
         tests through `resolved*` and the deep helpers, no call site edited. The dummy
         requirement's `notes.txt` holds a single newline, since any text in it reports
         `notes_found`. SWE-2 round S1: 6 accepted, one a real test's input (rulings
         `.tmp/duck_rulings_builder.md`), reworked (SWE-2, merge `cdd64037`); the second
         SWE-2 round, S2, on the rework: 3 accepted, none of reach (a), which ends the loop,
         fixed by the lead (a team's `refs.txt` is no roster; the empty-folder loop's test).
         The round's part over the grid's position tests and `commit.py`: 4 accepted, which
         ends its loop, all fixed (merge `50bed6bc`, `7b00659c`)
       - [x] 2 (2026-11-22, merge `5a46d101`, SWE-2) `aesthetics_export`'s validator tests
         through `report*`, nine tests of a requirement's own check on `without`
       - [x] 3 (2026-11-22, merge `beeb2094`, Opus) the Compiler's CLI tests the logo slice
         touched (147 tests), through `complete`, whose folder walk is
         `team_compiler::test_files::complete`, shared with the unit tests; the CONTRIBUTING
         rule
       - [x] 4 (2026-11-22, merge `3efc8619`, Opus) the Compiler's other CLI tests, `deploy` and
         `prefox_kit_variants` included: 455 tests in 17 files through `complete`. A Run B over
         every export kind found 10 referee tests still on their own files, since the dummy
         check fired on team exports only
       - [x] 5 (2026-11-22, merge `807e376b`, Opus) `export_upgrader`: 109 tests (105 CLI, 4
         unit) through `complete_old`, whose `old_missing` counts a `Full` export's added logo
         as its content. The dummy run uses a `Collars/` file (Run B'), since an upgraded
         `notes.txt` always reports `notes_found`; its 13 failures are the builder's own
         tests, the tracer goldens, 5 tests reading the `co Spring` fixture as it is, and 4
         archive tests for slice 7
       - [x] 6 (2026-11-22, merge `86928756`, SWE-2) `studio`: the six Compiler CLI tests
         through `complete`, `console.rs`'s old export through `complete_old`; `launchers.rs`
         reads the upgrader's fixtures as they are. Its dummy runs come in two pairs, since
         both checks sit in the validator the upgrader also runs on its output
       - [x] 7 (2026-11-22, `65341792`, lead) the unit tests' tracer copy (`testing.rs`'s
         `sandbox`) completed, as the CLI
         tests' copy is since slice 4, so the `gui_run` tests that compile it pass a dummy run.
         The archive fixtures all stay stored. Packing a `.zip` when the tests run would save
         no step, because its `.7z` twin is stored (writing one needs `sevenz-rust2`'s
         `compress` feature, which the workspace leaves off) and is rebuilt by script whenever
         a requirement adds a file. When the next requirement comes,
         `tracer_archives_team_folder.py`'s `ADDED` grows to the `co Midcup Spring` archives
         and the upgrader's `co Spring` ones, which have no script in the repository yet
     - b's tests are written on `complete` (builder slice 3 landed)
     - [x] b (2026-11-22, merge `e0c24349`, Opus) `installed_portrait_slots` filled from the
       Compiler's walk over the installed CPKs (TC-ROOT-17). The builder's `Portraits` fills
       every team export, so 64 tests gained portrait entries; a portrait is a task of its own,
       so a folder a failing task drops still ships it, as the spec's group commit says
     - [x] Tier 2 review of slices a and b (2026-11-22): SWE-2's round S1 (its fixes
       `dc5e32ea` to `33c9a675`), then Carbon's round C1, 7 accepted, none of reach (a), which
       ends the loop. Its fixes: the texts `d70e56ea`, and a player folder's `portrait.*`
       counted whatever its kind, EU-MDL-43 widened to `portrait_missing` (SWE-2,
       `f4a8ac22`)
   - Not urgent, a small slice of its own (SWE-2), after the view's or beside it: player
     numbering without the hyphen (`plans/aesthetics_export/player_folders.md`, the section of
     that name): `aesthetics_export`'s `conventions` (`folder_slot`, `folder_player_name`), the
     `Portraits/` names and the kit folders (`validate/kits.rs`), their tests, and the catalog
     texts of `player_folder_number_invalid`, `portrait_name_invalid` and `kit_folder_invalid`
     in `messages/catalog.rs`.
7. The drafts in two copies (the maintainer, 2026-11-17: a spec draft of what the build holds
   and a plan draft for prototyping, which the app's unbuilt sections open with `demo=1`. Git
   keeps the two: the spec draft is `docs/prototypes/` on main, which `draft_compare`, the
   looks and the tasks cards read; the plan draft a long-lived branch in its own worktree, a
   step's draft commits merged into main when its build merges, and main merged back into the
   branch, never rebased; then the plan and spec text keeps the reasons, the behaviour and what
   no draft shows, such as the light theme, and points at the draft for measurements rather
   than copying them, a copy that drifted once: rows "22 apart" against the draft's 24).
8. The DpFileList upgrade on a PES 2019 DLC (the VGL organizer's poc.5 report, 2026-11-17:
   after `upgrade-dpfl` the game ran vanilla and made a new save of the real teams; the
   official list has no stem for the PES 2019 list's `03_base`, `10_scoreboard`,
   `11_interface`, `12_cutscenes`, `13_competition`, so they drop out, and PES 2019's records
   carry a countdown at byte 36 ours lack; the maintainer is settling it with the organizer).
9. The rest of Team TOML's numbers as the game shows them, as 0.0.2 did for form, injury
   resistance and weak foot: team colours in 8 bits as on screen (the file holds the save's 0
   to 63), `[tactics.auto] substitution` and the appearance colours by name. The Save editor
   draft's sample Team TOML (`save_editor.js`, `"pes17/718"`) is the CLI's export of
   2026-11-21, so it is exported again once these land.
10. Reset window's first click on Linux (the maintainer's check of 0.0.2-rc.3, 2026-11-17,
    "good enough for 0.0.2": the first click resets the window but the button stays, a second
    click changes nothing and the button goes, its pointer a hand; likely the window manager
    leaving the reset window off the fitted size, so the button waits for a click the window
    does not follow).
11. The mutation run the 0.0.2 window never had: `abbae531..aab124dc` (2026-11-14 to the
    0.0.2 cut; 166 commits touching code, about 28,000 lines added). The pipelines of those
    days ran `--no-mutants` while mutfast was awaited, and `checkpoint.py run` moves
    `refs/checkpoints/gated` with or without the mutation run, so the first run after mutfast
    (2026-11-20) diffed from `aab124dc` and nothing came back for the range. The cause is
    fixed (2026-11-22): the mutation run starts from `refs/checkpoints/mutated`, which only a
    run whose mutants all ran moves (RUNBOOK "How the work runs"). Owed:
    - the catch-up: `mutants-diff abbae531` over the range, sized first with its own
      estimate; per crate past about an hour a side, the crate's run goes to the VPS (RUNBOOK
      "Mutation runs and the VPS"), the rest here; then its triage as the checkpoints' are.
12. The suite's own images (DECISIONS 2026-11-20 "the placeholder portraits and the Team
    Export thumbnail are the suite's own"): the three placeholder portraits drawn as SVGs and
    the 4cc clover thumbnail, both rendered to the PNGs the binary embeds by one script; a
    look of each beside the game's. PES 2017 lists and imports a Team Export carrying the
    clover (checked in game 2026-11-20); its list draws the game's stock picture for every
    file, so the clover shows nowhere in game. A starter with no portrait now shows the
    install's own `player-dummy.png` on the pitch and in the team picture, the drawing only
    without one (merged `cc113dc2`); its SWE-2 round S1dm (2 accepted, the loop ended): a
    range of pictures reads the game's copy once, and two test gaps, merged `c20681a1`.
    Owed: the paid review rounds of both commits (Tier 1).
    Can run beside the detour: it touches no PES 2014 file.
13. The pitch from the game's own texture, as the placeholder portrait is (the maintainer,
    2026-11-21), the painted pitch staying for a member without the PES folder (DECISIONS
    2026-11-21 "the pitch is the game's own texture when the PES folder is set"). The texture
    is in Konami's apk menu packs; the `apk` crate's plan (`plans/libs/apk.md`, `c8daca62`)
    holds the format, from two research passes over every PES 2014-2021 copy (SWE-2,
    `.tmp/apk_re/findings.md`, `findings_write.md`; the Python prototype `apk_write.py`
    beside them), checked against CPGE V0.7.1's reader and the maintainer's
    `pesafp_texturetool.py`. Next: the code, in two slices: the crate's reader with the
    Tactics screen's pitch (`save_editor.md` "Tactics editor"), then the writer, ending in
    the in-game check its plan's "Tests" names.
14. The PES file explorer (`plans/core/development_plan.md` "Unscheduled tools"), medium
    priority, from two tools: Jenkey's PES File Explorer for PES 2014
    (`C:/Data/4cc/0Tools/S_File_Explorer_2014`; native and BoxedApp-packed, so evidence by
    what it does, not by its code; it browses CPKs and edits DDS and apk textures) and CPGE
    V0.7.1 (above; besides apk it handles `.str`, FTEX, FPK, Fox files, FMDL and CPKs). Its
    plan comes after the apk crate's (item 13), which it builds on. A survey of every tool
    in `C:/Data/4cc/0Tools/` (SWE-2, 2026-11-21, `.tmp/tools_survey/findings_a.md` and
    `_b.md`, each tool's evidence) gave the explorer's file list, now in its plan entry. It
    found no other tool for the suite: the rest are covered, a runtime piece (sider, DXVK),
    general-purpose, or for another game. Two Blender add-ons, Joshua502's stadium camera
    editor (`.fdc`) and MFG's kk2pes (other games' rigs onto PES bones), are in the Blender
    project's `pes_models_plan.md` ("Community add-ons to take in"). The survey of
    `C:/Data/4cc/Tools_4cc/` (SWE-2, `findings_c.md`) added two tools to "Unscheduled tools",
    a Video encoder for the cup's USM videos (the format in `plans/libs/usm.md`) and an Export
    viewer (MFG's FMDL-Viewer), two file kinds to the explorer (`.mtar`, `.fsop`), and a
    third add-on, EssexDotDev's 4cc-FMDL-Toolkit, to the Blender plan.
15. The checkpoints of 2026-11-22. The one on `f51c570a` left eight mutants alive, whose
    tests landed (merge `36458a20`). The one on `625e7147` passed its gates and ran every
    mutant, yet left the gate at `fc6e8cf0`, as `f51c570a`'s had. Neither was
    stopped by its survivors: PowerShell, the shell `just` runs recipes under on Windows,
    turned `mutants-diff`'s exit 2 (missed) and 3 (timed out) into 1, which the checkpoint
    reads as untested mutants. Fixed in `e3cff7c6`, the checkpoint now running
    `mutants_diff.py` directly; `refs/checkpoints/gated` and `mutated` were then moved to
    `625e7147` by hand, as the fixed script would have. Its 107 survivors (`.tmp/lead/mutants_625e7147/missed_all.txt`),
    triaged:
    - the two `session_link.rs` timeouts are caught: each mutant makes `give`'s send loop
      endless, and a timeout an infinite loop explains counts as caught (RUNBOOK "Mutation
      runs");
    - `save_editor`: four are equivalent, now in `.cargo/mutants.toml` (`toggle_in_group`'s
      `>=`, the Similar dialog's three `|` to `^`); two need tests: `alike`'s
      `skills && com_styles` (a player matching on skills alone must not match on Cards)
      and the dialog's `!team_category.contains` (Every team keeps a category that holds
      every match);
    - `export_grid` and `team_compiler`, rerun at HEAD since the grid's position tests landed
      after `625e7147`: 6 and 11 left. Two timeouts are caught (endless loops); `text_colour`'s
      `>=` and `export_row`'s message guard are equivalent (`.cargo/mutants.toml`); the rest,
      with `save_editor`'s two, got tests (Opus, one rework, merge `881c9a32`), and the
      survivor that was unreachable code, a validated export's own portrait count, is gone;
    - [ ] then `just pipeline` at the new HEAD.
16. The VPS disk guard refuses only at the mutation step, after the checkpoint's whole test
    pass (2026-11-23, on `e9b0a3d0`: 11 minutes of tests, then 42 GiB free against the 48 GiB
    floor; the build cache's cleanup freed 30 GB and the pipeline ran again from the start).
    Fixed: `just pipeline` now asks the remote whether it can take the run (not busy, not
    short of disk) before the checkpoint starts, so a refusal comes first.
17. Owed paid rounds went unseen (census of 2026-11-23): the Compiler grid's slices never
    named their tier, so their Carbon round was never queued, and four reworks of Carbon's
    round 1 merged without their worklog lines saying so, which left two round 2s looking
    blocked. The first broke RUNBOOK's rule that a step's worklog line names its tier when
    it lands. The rounds had fallen due while the reviewer's usage was short, and a round
    held for that was noted nowhere the next session reads first. Fixed as three habits of
    the lead's: the rulings line that ends a loop names the next review owed, a rework's
    merge updates its worklog line in the commit right after, and a run held for usage is a
    line of the lead's handover until it runs.

Most open steps below wait only on a check in the game or the built window that the lead's
harness makes ("Pending looks" below), not on the maintainer.

**Done for 0.0.2 since 2026-11-16:** 0.0.2-geninfo (both slices; its SWE-2 loop ended),
-frames, -paneshift, -keepsel, -posheight and -lookfix; the light theme's control colours and
the Save editor's margins (`2d1d41fe`, `69f3a70f`, `d9fc23bf`); the overall's paired abilities,
settled by three in-game checks (`overall_points.tsv` gained a physical contact column and 12
rows); Team TOML's numbers as the game shows them (`c48437e5`), the dribbling motion counted
from 1; Attacking area's Wide as 0 and the formation settings in the game's names; Report a
problem (the maintainer ran ST-REP-05; Carbon's loop and Astra's round done, fixes at
`b3e8f007`); the answers' recheck; `gui.py`'s review round.

**The PoC for VGL's organizer:** 0.0.2-poc.5 is built (`4cc-studio-temp/poc/target/dist`,
archive SHA-256 `cac0447a…`, branch `vgl27` at `61d224c4`: main of 2026-11-12 at
`cfd15a27`, so with the launchers' logs, the readme's backup and list steps, Update now's
Restart and the Save editor's three-column comparator), beside poc.4 and the older ones.
Both launchers were run from the unpacked archive under a pseudo-console, over a fake PES 21
install and the tests' fixture exports. Its in-game check and its hand-over are the
maintainer's. A Linux bundle, 0.0.2-poc.6, stands beside it (archive SHA-256 `0a68c459…`,
branch `vgl27` at `a611951c`: main at `177657a3`, so also the settings' four-digit folder
key, the taskbar fix and Update now's fixes of Carbon's round). It was built on the VPS
(`just remote just release`), so it asks for glibc 2.39 and serves a test only; the
maintainer started it once on the Linux virtual machine 2026-11-17: fine.
**Blocked on:** nothing.
**Behind 0.0.2:** Phase 8's other views and Release 0.1.0, then the phase order 1–6, 8,
0.1.0, 7, 9–16 (`plans/core/development_plan.md` "Releases"; the first-class target is the
Fox version the cup moves to around April 2027, "Target versions"). Phases 1 to 6 are
closed; 0.0.1 was released 2026-11-03 (tag `v0.0.1` on `c97ce914`). Built ahead of their
phases: 10.0a-g (`music_export`), 16.w (the platform floor), 16.x a (`dds_convert`'s GPU
BC7 half, unwired), 20.0 to 20.6a2a (the Ruleset editor, resuming after 0.1.0).
**Planned for after 0.0.2** (the maintainer, 2026-11-12): the Team decompiler is now the
Decompiler, for every export kind (`plans/decompiler.md`, a stub: team, referee, balls,
stadium and billboards exports from a DLC), and each kind's compile packs a recovery manifest
(`plans/team_compiler/pipeline.md`, `plans/balls_compiler.md` and
`plans/stadium_compiler.md`, "Recovery manifest"). The one Compiler those kinds are compiled
by was already decided (DECISIONS 2026-11-09, `plans/core/README.md` "One Compiler"; its
respelling of the plans is the issue of 2026-11-09 below). Reviewed in one SWE-2 round (tier
3): twelve concerns accepted and in the plan. Every kind is also recovered from a DLC that
has no manifest (DECISIONS 2026-11-12 "every export kind is recovered from an old DLC").

**Working mode.** `RUNBOOK.md` "How the work runs": the lead plans, briefs, reviews and
writes what is correctness-critical; sidekicks implement, two or three at once; a review's
depth follows its surface's tier. A review that cannot run is queued in the lead's handover,
and no phase closes with one outstanding.
- [ ] A periodic review of the dev cycle itself (`PREVENTION.md`, DECISIONS 2026-11-23 "a
  periodic review of the cycle", and "a look-ahead every two or three phases"): designed,
  waiting on the maintainer's start. It begins with a backfill of two weeks of prevention
  lines into the log, then the first review, then the first look-ahead. At the
  start, `AGENTS.md` gains the file's row (it is at its 16 KB cap: something is cut first)
  and the dev cycle chart its loop.

---

## How to resume

| Thing | Where |
|---|---|
| Plan index | `docs/plans/README.md` → `docs/plans/core/README.md` |
| Development phases | `docs/plans/core/development_plan.md` "Development Plan" |
| Legacy tools (format evidence) | `docs/spec/core/README.md` "Project context" (paths are per-machine) |
| Skeleton data | `resources/skeletons/` (see its README) |
| FPC source text (for the `fpc` crate) | `resources/FPC.wikitext` |
| `.model` format as measured (for the plugin author; the crate's notes are in `libs/README.md`) | `resources/prefox_model_format.md` |
| Coding rules and verification gates | `docs/CONTRIBUTING.md` |
| Domain terms | `docs/GLOSSARY.md` |

### Current-state gotchas

Things that are true *right now* and would cost the next session time to rediscover (a broken
fixture, a flaky test, a crate that doesn't build on one platform). Remove an entry when it stops
being true. Permanent toolchain facts go in `docs/CONTRIBUTING.md` "Toolchain notes" instead.

- `just` needs PowerShell as its Windows shell (`set windows-shell` in the justfile): Git for
  Windows' `sh` is not on PATH from a plain PowerShell. Recipes still stop at the first failure.
- `cargo install just cargo-deny` is required once per machine (both take a few minutes to build);
  the pinned toolchain and the wasm32 target install themselves on the first `cargo` call.
- `dds_convert`'s CPU BC1 core (`block_compression`, PCA plus one refinement) trails DirectXTex
  by about 2.5 mean error on an 8x4 grey-ramp mip while matching it within 1.0 at the top mip;
  the encode tests compare the chain-wide mean for that reason. Revisit with representative kit
  and face textures when the Compiler can produce them (plan: "tune against representative
  textures").
- `just mutants-diff` reuses `mutants.out/`: it wipes a whole-crate run's results. Keep the
  whole-crate run's summary aside (to `.tmp/mutants_<step>_whole/`, only `*.txt` and
  `outcomes.json`: `RUNBOOK.md` "The scratch folder") before the first diff run.
- `block_compression` 0.10's BC3/BC4/BC5 decoder truncates the alpha-ramp interpolation where
  DirectX rounds; `dds_convert::dds::fix_interpolated_channels` re-derives those channels. Drop
  it if a later release rounds (the exact-match test will say).
- `studio_core`'s `pes_process.rs` is split by `cfg` per OS, so a mutation run reports the other
  OS's mutants as missed: a Windows-only function's mutant tested on the VPS's Linux half (and a
  Linux one on this machine) compiles out. Triage those by OS, not as test gaps. Windows gates
  never compile a `target_os = "linux"` branch either (5.4a6's did not build): `cfg`-split code
  gets `just gates-remote` before it lands.
- A compile killed between moving a CPK aside and renaming its `.partial` in leaves that CPK
  missing, and one killed between the install and the savefile write leaves the new CPKs with
  the old save; the next compile repairs both (0.0.1c, `.tmp/release_0_0_1/0_0_1/interrupt/`).
- The VGL27 Friendlies set (39 teams, the organizer's save) is no longer on this machine: its
  sorted copy in `.tmp/vgl27f/sorted/` was deleted 2026-11-14 to free disk, and the maintainer
  holds the exports elsewhere; ask for them before a step needs the set.

---

## Phases

Status: `todo` · `in progress` · `done` · `blocked`. A phase is done only after its last two
steps, which every phase has: **converge** (the lead's own audit first, then the cross-family
reviewer's, both against the phase's plan sections and acceptance IDs; each gap becomes a new
step above it, and the phase waits for them) and **rewrite**
(what the phase delivered moved from `docs/plans/` to `docs/spec/`, corrected where it was built
differently; `RUNBOOK.md` "Phases and releases"). Then collapse its step list
below to this one row, keeping its `manual: checked` lines (the step-level detail stays in git
history), and bring the root `README.md` to the closed phase. Tool phases (3–6, 8–15, 17, 19, 20) also
open with an **acceptance** step: the tool plan's "Acceptance" section for that phase, written
before any code (GUI scenarios that no automated test can prove are marked `manual` and proven by
a recorded check at converge — `CONTRIBUTING.md` "Testing"). Procedure: `RUNBOOK.md` "Phases and
releases".

| Phase | Scope | Crates | Status |
|---|---|---|---|
| 1 | Workspace bootstrap + core skeleton | workspace, CI, non-GUI `studio_core`, `vtree`, `pes_version` | done |
| 2 | Library crates (standalone-verifiable) | `wezlib` `cpk` `fpk` `ftex` `dds_convert` `fmdl` `pes_model` `uniparam` `fox2` `archives` `fpc` `teams_list` `kit_config` `color_tools` `elevation` `model_convert` (native) `pes_savefile` `python_bindings` | done |
| 3 | Compiler skeleton | `team_compiler`, `aesthetics_export`, `pipeline` | done |
| 4 | Processing logic | `team_compiler` (`plan/` `processing/` `bins/` `output/`), `aesthetics_export` deep validation | done |
| 5 | Savefile integration | `save_editor` logic, `aatf`, `team_compiler` `output/savefile.rs` | done |
| 6 | Export upgrader | `export_upgrader` | done |
| 7 | glTF support (runs after Release 0.1.0) | `model_convert` (glTF half) | todo |
| 8 | GUI | `studio_core` shell, `studio`, tool views, `color_tools` widget | todo |
| 0.0.1 | Preview build: the Save editor for DBC alone, with DBC7's rules file and PES 2017 picked (`core/development_plan.md` "Divegrass Before Christmas") | `just release` | todo |
| 0.0.2 | Preview build: 0.0.1 plus the Compiler and Export upgrader through their CLI for VGL 27, with the launchers (`core/development_plan.md` "The /vg/ League 27 (VGL)") | `just release`, the launchers (the console's progress line done in 0.0.1a) | todo |
| 0.1.0 | Release 0.1.0, after Phase 8, a test release (`core/development_plan.md` "Release 0.1.0") | `just release`, `CHANGELOG.md` | todo |
| 0.2.0 | Release 0.2.0 or a later `0.x` (`core/development_plan.md` "Release 0.2.0") | help chapters, update check + notice, teams-list merge | todo |
| 9 | Stadium compiler | `stadium_compiler` | todo |
| 10 | Music tools | `music_player`, `music_export_editor`, `music_export`, `audio_engine` | todo |
| 11 | Match tracker | `match_tracker`, `match_feed` | todo |
| 12 | Kit config editor | `kit_config_editor` | todo |
| 13 | Refs arranger | `refs_arranger` | todo |
| 14 | Balls compiler | `balls_compiler` | todo |
| 15 | Player aesthetics editor | `player_aesthetics_editor` | todo |
| 16 | Polish and distribution | — | todo |
| 17 | Team creator (post-release) | `team_creator` | todo |
| 18 | Studio Web (post-release) | — | todo |
| 19 | DB generator (scheduled by need, after 2 and 8) | `db_generator`, `pesdb`, `pes_savefile` `ops/populate.rs` | todo |
| 20 | Ruleset editor (post-release) | `ruleset_editor`, `aatf` writer | todo |

---

## Steps

`[ ]` todo · `[~]` in progress · `[x]` done · `[!]` blocked / needs a decision

A step is done when its own `→ verify:` check has actually been run and passed, on top of the
gates. Every step carries one when itemized; if a step is listed without one, the agent writes it
before starting the step (a specific check, not "test it") and puts it in the step text. Mark a
done step with a one-line summary and the files or crates touched. One `[~]` per agent at a time.

### Phase 1 — Workspace bootstrap + core skeleton

Done 2026-09-13 (spec now describes what exists: `docs/spec/core/development_plan.md` "Phase 1"). Step detail
in git history up to commit `2f158a9`. CI proof: green run on `5e6ffc6`, deliberately red run on
`eccf0f3` (both `gates` jobs failed at `just gates`, `deps-check` unaffected), reverted in
`2f158a9`.

### Phase 2 — Library crates

Done 2026-09-30 (spec now describes what exists: `docs/spec/core/development_plan.md`
"Phase 2", `libs/README.md`, `model_conversion/README.md`, `pes_savefile/README.md`). Step
detail (2.1-2.21: each crate's build, review rounds A-C, the 2.20a-k converge with its
mutation runs, censuses and reviewer rulings) in git history up to commit `7de61f7`. Carried
forward: 2.5b GPU BC7 is step 16.x, the `dds_convert` cache bound is step 4.y, and the
phase's open questions are under "Issues".

### Phase 3 — Compiler skeleton

Done 2026-10-02, its cross-family reviews still queued (the maintainer's directive; "Handover"
lists them, 3.7's and 3.8's among them; what they find becomes steps under this heading). Spec
now describes what exists: `docs/spec/core/development_plan.md` "Phase 3",
`team_compiler/README.md` ("Acceptance": 77 scenarios, 75 cited by tests, 2 manual),
`spec/team_compiler/pipeline.md` "Run driver shapes", `aesthetics_export/object_model.md`,
`libs/pipeline.md`. Step detail (3.1-3.9f, the 3.w `.cargo/mutants.toml` audit, the 3.y
converge with its whole-crate mutation runs, the 3.z shell slice, 3.x this rewrite) in git
history up to commit `32261dd`. Carried forward: the phase's open issues under "Issues".

TC-GUI-01 manual: checked 2026-10-02 by the maintainer on Windows, the window, the sidebar's
Team compiler entry, its Settings, Compile button and log.
TC-GUI-02 was checked by hand the same day (the /egg/ tracer export compiled from the window,
the log showing `egg Tracer: Info export_identified (team=/egg/, id=792)` then `Run finished:
exit code 0`); since 8.0 it and TC-GUI-03 are automated, cited by the headless view and shell
tests, and TC-GUI-01 is narrowed to the window opening. Linux: not yet checked, no machine
with a display.

### Phase 4 — Processing logic

Done 2026-10-12. Spec now describes what exists: `docs/spec/core/development_plan.md` "Phase 4",
`team_compiler/README.md` ("Phase 4 scope"; "Acceptance": 363 scenarios, 2 manual, 5
withdrawn, all proven, every CLI test citing one), `team_compiler/pipeline.md` (the walkthrough
and "Run driver shapes"), `messages.md`, `settings.md`, `aesthetics_export/object_model.md`.
Step detail (4.0-4.34; the 4.c Clef passes; the 4.y converge: the lead's audit and the export
census, 4.y-fix1 to 4.y-fix23, 4.y-sub, 4.y-7z, 4.y-kitwarn, 4.y-mattable, 4.y-simp,
4.y-scen, the S7-S20 review rounds, the in-game harness and checks; 4.z this rewrite) in git
history up to commit `c93f025`. The converge's closing whole-crate mutation runs, all on bonfire
over `6e13fae`: `aesthetics_export` 519 caught, 0 missed; `pipeline` 54 caught, 6 missed
(`available_memory`'s arms for other platforms, accepted in `.cargo/mutants.toml`), 8
timeouts (`MemoryBudget::acquire` waits a mutant makes endless, caught); `team_compiler`
2205 caught, 10 missed, 3 timeouts, 384 unviable: the five sites filing a texture as the folder's own or a combined folder's got tests (`c93f025`), `own_package`'s
`ingame_face` guard is excluded as read by no consumer, the three `deploy.rs` error-kind
guards (tests Windows-only; the run was Linux) and the timeouts (two endless loops, one
64-fold texture) were caught when rerun on Windows, `settings_view` is the placeholder
label. Carried forward: the Fox cube map check (4.y-ingame2 (d), waiting on a hand-made FMDL; its
`maintainer/questions.html` entry), the phase's open issues under "Issues" (the pre-Fox
ones deferred until the cup next plays a pre-Fox version), the in-game questions in
`docs/maintainer/questions.html`.

The phase's in-game checks (TC-REF-04's at 4.19d; 4.y-ingame; 4.y-ingame2):

4.19d:
manual: checked 2026-10-09 by the lead on PES 17 through the harness (`.tmp/phase_04/4_19/ingame/`, results in `.tmp/phase_04/4_0/apptest/results.txt`): the referee present with the pair installed, nothing drawn; a `.mtl` beside a nocloth `.model` is read and stops the model drawing (runs E and F against 2026-10-08's run I), so the pre-Fox marker goes by Red's route instead: step 4.19f, DECISIONS 2026-10-09; the scene where the pre-Fox marker shows is a maintainer question

4.y-ingame:
manual: checked 2026-10-10 (a): PES 21's stock `collar_107.fmdl` as `Collars/collar_039.fmdl`
of an `/a/` Midcup export, converted for PES 17 (`uni_collar` then `uni_shirts`, two
meshes), drawn on the keeper WIDEFACE in Edit mode as the laced polo it is, in the GK
kit's black (`q03_neck.png`); the baseline, PES 17's own `collar_039.model` packed as
it is, draws a plain stand collar (`r02_neck.png`).
manual: checked 2026-10-10 (b): the tracer's `fcl_hair.fmdl` converted for slot 01 and
02, its body material naming `common/702/dummy_kit.dds`, draws kit 2's texture on
both (the keeper wears the GK kit, slot 02 kit 1), and a green checker shipped at that
path is not read (`p20_appearance.png`, `q04_slot02.png`): the plan's "active kit"
corrected, DECISIONS 2026-10-10; no compiler change.

4.y-ingame2:
manual: checked 2026-10-10, the portrait header variants on PES 21: the four files
of `C:/Data/4cc/Tools_Mine/temp/variants/` as the portraits of `/out/` bench players
(`.tmp/phase_04/4_y/ingame/build_portraits_cpk.py`), each hovered alone in Kick Off's team
sheet after its neighbour (the maintainer's method): A (DX10 header, BC3, sRGB id,
one level) and B (the same with the UNORM id) crash the game on hover; C (the
legacy `DXT5` header) and D (BC7 under DX10, full chain) draw
(`v21_portraits_sheet.png`). So the DX10 header itself, not the colour-space id,
crashes the Fox games' BC3 portrait reader, and the compiler's re-headering (4.y-fix6)
is the right fix; BC7 stays under DX10, which draws. The cup's eight BC7 portraits (ESG
XXX07-11, FGOG XXX13/21/23) are 128x128 with eight levels under DXGI 98, D's shape, so
nothing of the question is left (its QUESTIONS entry removed).
manual: checked 2026-10-10, collars beyond the stock set on PES 21: Test 3's CPK
(`.tmp/phase_04/4_0/apptest/out_test3/pes21/`, team `/out/`: kit 1's collar 200, a copy of the
game's `collar_107.fmdl` under that ID; the GK kit's collar 201, no model anywhere)
in the test slot, Edit mode's Appearance view: the outfielder draws a folded collar
(`p21_19_collar_front.png`), the keeper a plain neckline, no crash
(`p21_22_gk_front.png`): an ID beyond the stock set loads when its model exists.
manual: checked 2026-10-10, the sock table on PES 21, in part: sock design A
(`socks_design_prefox.png`, converted) on `/out/`'s kit 1: the thick white stripe at
the front, the thin red one and the yellow disc on the outer side, both socks alike,
the stripes straight, no seam at the visible part (`p21_legs_sheet.png`); the
diagonal and the hoop sit under the team's gaiter boots, and the PES 17 reference
view is still to take (open item (a)).
manual: checked 2026-10-10, the sock table's PES 17 reference and the lower sock:
Test 3's kit (design A, laid out for PES 15-17, so no conversion) compiled for
`/out/` on PES 17 with the cup's own heads, whose PES 17 players wear the game's
boots: the thick white stripe at the front, the thin red one at the back, the
yellow disc on the outer side, the green diagonal band on the lower calf and the
orange hoop at the ankle, as drawn (`s17_34_out_legs.png`, `s17_35_out_legs_side.png`,
`s17_36_out_legs_back.png`); PES 21's converted upper part matches it feature for
feature. The lower part on PES 21 still waits for a player without the gaiters: a
`settings.toml` with `boots_id = 0` writes no table before Phase 5, and a blank
`face.fpk` in the test slot did not replace `/out/`'s cup heads in PES 21's Edit
mode (`w21_24_roster.png`), an observation for Phase 5's own checks. Method:
`/a/`'s save hides its players' standard bodies (billboard models), so any face put
on an `/a/` slot draws with no body on PES 17; the Face view draws the face package
alone, and the whole body shows in the Appearance view zoomed out or in Strip Style
(`scripts/ingame/README.md`).
manual: checked 2026-10-10, the stock collar set: the template config's collar and
the version's last stock ID (116 on PES 17, 131 on PES 21) each draw a collar on
`/out/`'s standard bodies, a crew neck with the kit's own V at the back, 116 a
V-neck under the long-sleeve toggle (`s17_38_collar_front.png`,
`s17_42_c116_collar.png`, `s17_47_sleeves_long.png`, `w21_17_row5.png`); the
QUESTIONS entry closed.
manual: checked 2026-10-10, `kit_config::validate`'s two Infos: shirt model 150
(outside the documented 144, 160 and 176) draws a sleeveless shirt on PES 17 and
PES 21, no crash (`s17_40_model150.png`, `w21_17_row5.png`), a model the game has,
so `kit_shirt_model_unknown` names a documentation gap, not a bad value;
`long_sleeves = 3`, an undocumented raw value, leaves the short-sleeve view as it
is and draws the long-sleeve variant without sleeves on PES 17
(`s17_47_sleeves_long.png`), no crash. Both stay unreported by the compiler
(DECISIONS 2026-10-10); open item (e) closed, (a) closed but for PES 21's lower
sock.
manual: checked 2026-10-10, the 192x512 mipped texture (c, first half): galosengen's
`Common/scroll.dds` (192x512, DXT5, ten levels) compiles for PES 17 with no finding
(the pre-Fox rule has been `texture_not_div4` alone since 2026-10-07; the question's
"both engines" was stale) and, as the `face.dds` of the Fumos head on eight `/out/`
players, draws its content (black with brick-red squares, `scroll_source.png`) on
their heads in Edit mode's roster preview (`t17_rows_sheet2.png`, rows 16-23, against
the fixture's red texture on rows 1-15): PES 17 reads a mipped texture whose sides are
not powers of two, so `texture_not_pow2` stays Fox's. Export built by
`.tmp/phase_04/4_y/ingame/build_tagtex_export.py`.
manual: checked 2026-10-10, the `.model` mesh tag (c, second half): the stock
`konami_modD_cap.model` is PES 17's captain armband (its one mesh tagged
`DCaptainmark`, material `modD_cap_phone` with no texture), put as `cap.model` on
`/out/` players with a blank face: the tagged one and a copy with the tag renamed
`DZaptainmark` both draw the white band on the left arm in Edit mode's Appearance view
(`t17c_face_01.png`, `t17c_face_08.png`; none on the control, `t17c_face_16.png`), and
in an exhibition match (/acccc/ against /wsg/, the tagged band on /wsg/'s slots 01-11
with blank faces, the untagged one on 12-23 under the Fumos head) a headless
non-captain wears the band in the replay's ground camera (`t17m_free_left.png`; the
captain is Ricardo Milos, red-headed in `t17m_rt_22.png`). The game draws a tagged mesh
as an untagged one, so the IR keeps dropping tags (`ir.md`, DECISIONS 2026-10-10); the
QUESTIONS entry removed. (c) closed.
manual: checked 2026-10-10, a `.model` collar converted for PES 21 (4.y-fix12's open
question): PES 17's stock `collar_001.model` and `collar_050.model` (the probe copies in
`.tmp/phase_04/4_y/uniform_mtl/`) each as `Collars/collar_012.model` of an `/out/` export with
Test 3's kit 1 (`.tmp/phase_04/4_y/ingame/build_collar21_export.py [050]`, the release build of
`104d8c8`; 200 is `collar_id_invalid`), the CPK's `collar_012.fmdl` in the test slot: row
19's standard body draws the same plain neckline for both (`c21_14_row19_neck.png`,
`c21_24_row19_neck.png`), the one a collar ID with no model draws (`p21_22_gk_front.png`),
where the stock `collar_107.fmdl` copy drew its folded collar (`p21_19_collar_front.png`).
The converted FMDL (the converter's `fox3ddf_blin`, every sampler dropped) is not drawn:
step 4.y-fix18 gives it the stock collars' Fox material set. The PES 21 Edit path drifts
when the title needs one `a` instead of two (Kick Off's submenu opens: `b` backs out).
manual: checked 2026-10-10, the same `collar_050.model` under ID 12 with 4.y-fix18's
release build (the stock collars' Fox material set): row 19's standard body draws a
folded collar in the kit's colour at the neck (`f18_05_row19_neck.png`, against
`c21_24_row19_neck.png`'s plain neckline), so the material set was the whole cause;
the missing vertex colour stream and the four unweighted bones the converted FMDL lacks
(4.y-fix18's comparison) change nothing visible. Compiled with the CLI from the test
folder (`team-compiler compile --no-deploy --export`; the version is the settings').
(d) answered by the maintainer 2026-10-10, no run needed: a Fox referee export's `env.dds`
is a pre-Fox leftover, and no Fox material samples a cube map
(`resources/Materials.wikitext`), so the 0xD form is drawn by nothing and stays Red's
harmless output (`pipeline.md` step 3); the QUESTIONS entry removed. The earlier note:
manual: not run 2026-10-10, the Fox cube map of type 0xD (d): no material of the refs
export (robocopclassic's `apc.mtl`, `face.mtl`) and no bundled `.mtl` names `env.dds`,
and the 4cc Fox CPKs hold no cube map (4.y-fix3's recon), so a Fox material pointed at a
converted `env.ftex` exists in no export: the check needs a hand-made FMDL, which no
reachability rule puts ahead of the queue. The QUESTIONS entry stands as written.
manual: checked 2026-10-10, the pre-Fox referee marker (b), no answer yet: run D (the
checker as `parts/referee/incom_bsm.dds`) and run G (the template tree's
`referee_prop.model` and `.mtl` with it, what the compiler writes) in PES 17's test
slot, an exhibition match of `/4cccc/` against `/wsg/` driven by the pad: no magenta
and yellow square anywhere in the walkout, the kickoff's wide camera or the replay's
top-down and free cameras at 0:00 (`r17_walk_*`, `r17g_kick_*`, `r17g_top_03.png`),
and no referee figure at all in any of them (a player leads the walkout; the only
figures on the pitch wear the two kits). The cup's own `4cc_35_referees.cpk` holds the
same prop pair and its clover `incom_bsm.dds` (5.6 MB), 35 referee face CPKs and the
referee kit textures, and Sider's LiveCPK root (`C:/Data/4cc/0Livecpk`) holds no referee
file, so the install's referee is drawn by nothing the lead can see: whether the
referee is hidden on purpose on this install, and which scene draws the prop, stays
the maintainer's (QUESTIONS, narrowed). The replay's free camera (mode 3 of the A
button) is tethered to the target player and the pad holds one input at a time, so
no close view of an official was reachable; the pad's `hold:left` rewinds there.
manual: checked 2026-10-10, a Fox subfolder texture before `sourceimages` (f): /@/'s
`hair_high.fmdl` (a whole head naming `face_bsm`) on /out/'s slots 01-03 in PES 21's
test slot (`.tmp/phase_04/4_y/ingame_f/build_subtex_export.py`, compiled by the release build of
`565c802`): slot 01, the head and its own texture in the root, draws as /@/'s player;
slot 02, the head and a magenta `face_bsm.png` in `jessie/`, (b1)'s layout as compiled
(`.../02 - Old/sourceimages/jessie/#windx11/`), draws magenta; slot 03, the same in
green with the CPK patched to the chosen layout (`.../03 - New/jessie/sourceimages/
#windx11/`, the FMDL directory moved alike, `patch_new_layout.py`), draws green
(`s21_rows.png`, Edit Player's roster preview). The game reads a texture wherever the
FMDL's directory says, `#windx11/` inserted before the name, so slice (b3)'s layout move
is safe; (f) closed, its QUESTIONS entry removed.
manual: checked 2026-10-10 by the maintainer, the pre-Fox referee marker (b): PES 17 with
the cup's current `4cc_35_referees.cpk` (the template's prop pair and clover, what the
compiler writes, step 4.19f) draws the clover on the ground under each of the three
officials in every scene (a walkout in the black Void stadium, the maintainer's
screenshot). That CPK's clover is a plain 5.6 MB DDS, the form the compiler writes there
(`team_compiler/referees.md`), so the plain-DDS half is answered too. Why the lead's runs D
and G (a plain checker through the test slot) showed no marker is not known: the marker does not
scale with a small referee (the maintainer), so a missed official does not explain it.
(b) closed, its QUESTIONS entry removed.
manual: checked 2026-10-12, the first PES 20 `.ted` (g): team /3/ exported from
`F:/Games/PES2020` (the cup's base DLC; `game.py`'s new `20` entry, no Sider; the file
name typed by posting `WM_CHAR` to the window, so no focus taken; frames
`.tmp/phase_04/4_y/ingame/frames/p20/`) is 14760 bytes (`0x39A8`, the derived size) with key
index `0x14`, the only one that decodes the coach block's `MANAGER` and the team id.
The writer's PES 20 header template was PES 21's, whose u32 at `0x08` (the body length)
is 21's: replaced by the file's header, and the file is the PES 20 fixture of
`pes_savefile`'s Texport tests (`pes20_texport.ted`). Its 660-byte block holds five
125-byte runs where the other fixtures' is zero (`operations.md` Texport). (g) closed.
manual: checked 2026-10-12, the lower sock on PES 21 (a's rest): Test 3's kit (design A,
`pre-fox`, converted) compiled for `/3/` (701, `build_socks_export.py socks team=3
runner=21 noplayers`, the release build of `5a28e37`), whose second row is a standard
body in the game's boots: the green diagonal band on the lower calf crossing the white
stripe, each leg's in the PES 17 reference's direction, and the orange hoop at the
ankle, above the boot (`.tmp/phase_04/4_y/ingame/frames/p21sock/s06_front.png`,
`s07_side.png`, `s08_back.png`, against `s17_34_out_legs.png`); with the upper part
checked 2026-10-10, the converted sock matches PES 17 feature for feature. (a) closed.
manual: checked 2026-10-12, QUESTIONS "Non-power-of-two mipped textures and kits over 2048
on Fox": Red (`e12aa01`, a scratch copy under `.tmp/phase_05/5_0e/red_np2/`, `setup.py` and
`retarget.py`) compiled EGG VGL26's `Faces/XXX14` and AKG VGL26's kits as `/3/` (the
save's /vg/ teams are placeholders that draw no custom kit, `s06_vgl5.png`); its FTEX keep
the chains (`screen` 512x461 and `text` 797x64 ten levels, `u0701p1` 4096 thirteen). In
Edit mode row 14 (70114) draws the Steam page and the caption as the sources show
(`.tmp/phase_04/4_y/ingame/frames/p21np2/s09_row14_face.png` against `screen_source.png`), and row
2 wears AKG's kit (`s07_kit4096.png`). Red's rules adopted (DECISIONS, step 5.0f), the
entry removed. The gloves-subfolder entry (PES 17) cannot run yet: a shared gloves folder
compiles to the team's first shared ID, drawn only on a player the save points at it, which
is the deferred pre-Fox patch's write; the entry says so.

### Phase 5 — Savefile integration

Done 2026-10-17. Spec now describes what exists: `docs/spec/core/development_plan.md` "Phase 5",
`pes_savefile/README.md` (and `operations.md`, `model.md`), `aatf_rules.md`, `save_editor.md`
(its CLI and 25-code message catalog), `team_compiler/README.md` ("Acceptance": the SE and TC
scenarios, 442 proven under `just acceptance strict`), `aesthetics_export/settings_toml.md`.
Step detail (5.0-5.0m4: the 4ccEditor fork survey, the Red inventory and its Fox gaps, the
plans/spec split; 5.1a-d the schema widening to PES 18-19; 5.2a-i `aatf`: schema, interpreter,
host, the eight rulesets, their review loops; 5.3a-b the Compiler's rows, patch and
savefile transaction; 5.4a-c the Save editor's CLI; 5.5 the in-game check; 5.y-gloves; the
5.6a-f converge: the lead's audit, the design-health pass, the `pes_savefile` census, the
cross-family reviews, the citation audit, 5.6e this rewrite) in git history up to commit
`27a94bb`. The converge's closing whole-crate mutation runs (extra kinds included, split with
bonfire, triage in `.tmp/phase_05/5_6/close/mutants_<crate>/triage.md`): `pes_savefile` 1830 caught, 2
missed (both tested, `27a94bb`), 2 timeouts (endless loops, caught); `aatf` 242 caught, 0 missed, 1 timeout (an engine with no operation limit hangs the endless-loop
test, caught);
`save_editor` 128 caught, 3 missed (the placeholder `view` and `settings_view`, and a
`NotADirectory` arm covered on Linux); `studio_core` 76 caught, 5 missed (`pes_process.rs` arms
the half's OS compiles out); `aesthetics_export` 603 caught, 0 missed; `team_compiler` 2474
caught, 1 missed (the placeholder `settings_view`), 3 timeouts (caught). Carried forward: the
gloves half of the in-game check (issue "FoxDen's PES 2021 gloves trampoline"), the pre-Fox
patch and the Red inventory's pre-Fox gaps (deferred until the cup next plays a pre-Fox
version), the placeholder views (issue of 2026-10-16), the phase's other open issues under
"Issues", the in-game questions in `docs/maintainer/questions.html`.

The phase's manual checks:

TC-SAV-10 (withdrawn at 8.9l5b for TC-SAV-17, which tests prove), checked by hand 2026-10-15, against a stripped copy of the maintainer's save
and a sandbox PES folder (`.tmp/phase_05/5_5/studio_sl/`): with PES 21 running, sideload compile
exit 0, the patch's `cpks = []`, `savefile_skipped_pes_running`, the copy byte-identical;
with only PES 17 running, `savefile_written` and the copy's `.bak` kept

SE-CLI-02 (withdrawn at 8.9l5 for SE-CLI-03, which tests prove), checked by hand 2026-10-15, with PES 21 running: `import-toml`, `apply-patch`,
`transplant`, `strip-and-seed` and `check-stripped --strip-again` each refused with exit 2
naming PES 2021, no `--out` written; with only PES 17 running each wrote its `--out`
(`.tmp/phase_05/5_5/cli02/`)

5.4c2 (`strip_base_copied_face`):
manual: checked 2026-10-15, PES 21 Edit mode (`.tmp/ingame/face_check/`): a stripped
base-copied player shows Placeholder; the copied face shows with the face flag clear too

5.5 (the in-game check; runs, shots and saves in `.tmp/phase_05/5_5/`, by a305229's release exe; the
maintainer's save swapped with their approval and restored, SHA-256 checked):
manual: checked 2026-10-15, PES 21 Edit mode, the maintainer's cup save strip-and-seeded
(`4cc_08_bins.cpk` seeded into the test slot; /d/ 0 of 23 players stripped before, 23 after):
/d/ rows 1-3 and VGL 22 (team 792) rows 5-6 look the same as on the unstripped save
(`pair_d_0N.png`, `pair_vgl_0N.png`), a sample, not every player
manual: checked 2026-10-15, PES 21 Edit mode, stripped save, the /egg/ tracer export compiled
into the test slot: 792's slot 05 shows the compiled full-body model with its boots
(`s3_05_body.png`); the CPK holds `4cc_studio/shared_ids/792.toml` and loads as any other
manual: checked 2026-10-15, same session: slot 06, an `ingame_face` folder whose
`settings.toml` sets only a motion, shows the default face (short dark-brown hair, Face and
Hairstyle editable; `pair4_vgl_06.png`). Without the marker the folder gets the blank face
and no head, as designed (DECISIONS 2026-10-04; `s3_06_body.png`)

5.y-gloves (every `.mtl` of a pre-Fox shared gloves output packed at its root):
manual: checked 2026-10-15, PES 17 Edit mode, /out/ slots 01-03 on shared gloves 1684-1686
(`.tmp/ingame/gloves_check/`, `r3_slot0N.png`, `r4_slot03.png`): `glove.xml`'s
`./grip/glove_r.model` loads; a `.mtl` texture path `./x.dds` and `./sub/x.dds` loads,
`../x.dds` and the game-rooted `model/character/glove/g1684/x.dds` do not (white glove);
a `grip/` model whose `.mtl` sits at the root naming `./x.dds` loads (`r5_slot01.png`)
manual: checked 2026-10-15, PES 17 Edit mode, end to end: run 2's export compiled by
this step's exe (`.tmp/ingame/gloves_check/studio6/`), unpatched: slots 01 and 02 draw their
right gloves magenta (`r6_slot01.png`, `r6_slot02.png`)

### Phase 6 — Export upgrader

Done 2026-10-20. Spec now describes what exists: `docs/spec/export_upgrader.md` (steps 1 to 12,
model-folder mode, the CLI, the message catalog, the Phase 6 acceptance),
`docs/spec/core/development_plan.md` "Phase 6", `team_compiler/testing.md` (the tracers' upgraded cases and the
library parity), `aesthetics_export/object_model.md` (Portraits named `NN`); the plan keeps
step 13 (Phase 7) and the view (Phase 8). Step detail (6.0a-c the open items, the acceptance
section and the Red inventory on `main`; 6.1-6.8 the build, step by step of the plan; 6.9-6.11
the tracers' goldens and parity cases, Red's references recompiled with released Red; 6.12 the
library parity, its census, Red runs and attribution, kept as `scripts/library_parity/`;
6.13-6.16 the defects and the maintainer's rulings it raised: an upgrade writes a clean export
or nothing, leftovers dropped with a Warning, one-wearer item folders' exclusive IDs, Midcup
kits keeping what they do not resend, a pre-Fox Common segment of any name; the converge: the
lead's audit, the design-health passes, Astra and SWE-2 rounds, the citation audits) in git
history up to commit `cf78a8e1`. The close (2026-10-20): the census of every old export on the
maintainer's machine (639 inputs, 279 clean, every unclean run writing nothing, none lost
against 6.16's run), the library parity over the 279 against released Red (every line
attributed but two `red_parity` pairing ties, `.tmp/phase_06/6_close2/`), the design sweep over the code
since the first close run (`upgrade.rs` past a thousand lines and the staging claim written
twice: `staging.rs`), the citation audit (one flag, EU-REF-12), and the closing whole-crate
mutation runs (extra kinds included, split with bonfire, `.tmp/mutants_6_close2_<crate>/`):
`export_upgrader` 981 caught, 6 missed (the two placeholder views; three equivalents whose
`exclude_re` pins had drifted, re-pinned; a game path's letter case, a test added in
`players.rs`), `team_compiler` 2532 caught, 3 missed (the placeholder settings view;
`match_stem`'s empty-inner guard, a test added; the shared folders' packing sort, an equivalent
pinned in `.cargo/extra_mutants.toml`, since the CPK sorts its entries and the tables their
rows). Carried forward: the placeholder views (8.3), the face transformation for a face made for
the other engine (issue "a face diff is engine-specific", after the DBC editors), the gloves
half of the in-game checks (issue "FoxDen's PES 2021 gloves trampoline"), the `save_editor`
tests a running PES game fails (issue), the in-game questions in `docs/maintainer/questions.html`.

The phase's manual checks:

6.16c (a one-sided hand split, HDG VGL26 slot 22 `EAT`):
    manual: checked 2026-10-19, PES 21 Edit mode (`.tmp/phase_06/6_16/ingame/`, the export upgraded
    `--loose` and compiled by dc30c45; /hdg/ is the maintainer's save's VGL 33, team 803; save
    backed up and restored): the compile's `GloveList.bin` rows never reach an outfield player
    on the cup's exe (issue "FoxDen's PES 2021 gloves trampoline"; a stripped save, then the
    stock exe, showed stock hands, slot 06's two-sided crossbow gloves included). With the ids
    in the save records (80306 gloves 4186, 80322 gloves 4202, the route the cup uses today):
    slot 06 wears its crossbow gloves (`w01.png`); EAT shows no stock hands, and the split's
    patch floats as a speck beside the model (`w02.png`, `w02_speck.png`)

### Phase 8 — GUI

The phase's steps are itemized when it opens. Before that, its plans get the fixes a survey
found (2026-10-15; the prototype rule and the launcher were fixed that day, DECISIONS):

- [x] 8.0 plan fixes before itemizing (side-lead C, branch `side/8.0`). Done 2026-10-19: the
  sketches matched to the spec, Phase 8's acceptance sections written (`ST`, TC-GUI, SE-GUI,
  EU-GUI), `eframe` and `egui` pinned `=0.36.2` (DECISIONS 2026-10-19)
- [ ] 8.1 egui's bundled fonts, once the theme settles its icon font (the shell's own, 8.14;
  egui 0.37 also drops `default_fonts` from egui's default features, 8.11): the
  four default fonts are 1.37 MiB of the release exe's `.rdata` (2026-10-16,
  `.tmp/phase_16/16w/rdata_parts.py`), the two emoji fonts (`NotoEmoji-Regular` 409 KiB,
  `emoji-icon-font` 317 KiB) about 0.7 MiB of it. If the GUI shows no emoji, egui's
  `default_fonts` feature goes off and `Ubuntu-Light` and `Hack` are embedded through
  `FontDefinitions` → verify: every view drawn with the emoji fonts removed shows no missing
  glyph (egui's own widgets draw a few symbols from them), and the exe shrinks by about 0.7 MiB
  (`spec/core/distribution.md` "Size budget"). That font setup is the shell's first context
  setup: `surrender_focus_on = Presses`, set today by the Save editor's view every frame
  (`save_editor` `view::set_surrender_on_press`, 8.9e rework 5: a press elsewhere ends a
  field's keyboard gesture, which the one-entry-per-gesture undo needs), moves there, set once
  for every view, the Save editor's test harness setting it the same way
- [ ] 8.3 carried into the itemizing (2026-10-19): the placeholder `view` and `settings_view`
  of `export_upgrader` and `save_editor` (`lib.rs`, one label each) are the mutants their
  whole-crate runs left missed, deferred because no headless test exists for a label; each
  tool's view brings its own tests (EU-GUI, SE-GUI) and the placeholders go, so the next
  whole-crate run of each crate measures them
- [x] 8.4 the usability pass before the tools' drafts. Done 2026-10-20: `maintainer/proposals.html` P1-P22
- [x] 8.5 the Save editor's draft, `docs/prototypes/save_editor.html`, in five slices (8.5a-e).
  Done 2026-10-20 (SWE-2): every SE-GUI scenario but the manual ones walked in a headless-Edge
  harness on real save data
- Phase 8 opens (itemized 2026-10-20). The shell is Phase 3's slice (a label list and the
  active view, `studio_core/src/shell/mod.rs`); no `ST` or `SE-GUI` scenario is proven, and
  neither `libs/team_widgets` nor `color_tools`' widgets exist. The steps run in DBC's order
  (`plans/core/development_plan.md` "Divegrass Before Christmas"): what the 0.0.1 preview needs
  first, the Compiler's and the Export upgrader's views after it. Each step builds against
  its draft (the host page `docs/prototypes/index.html` and its `tools/*.js`) and its plan, and cites its scenarios in tests
  through the shell's headless harness (`shell/mod.rs`'s tests). The drafts wait for the
  maintainer's remarks, so the steps that draw least of what a remark would change come first
  - [ ] 8.6 the shell (`plans/core/gui.md`, `spec/core/gui.md`; draft `index.html`), sliced
    - [x] 8.6a the sidebar: the planned-tool registry, the About block, the PES selector, the
      icon rail, `last_tool`. Done 2026-10-20 (SWE-2)
    - [x] 8.6b the status bar, the window title and the tools' wake callback (ST-BAR-01 to 03,
      ST-TTL-01 to 03, ST-SET-03, ST-CLI-01, ST-SIDE-05). Done 2026-10-20 (SWE-2):
      `Settings::save` edits the file in place (`toml_edit`)
    - [x] 8.6c the settings menu (8.6c1), the first-run dialog and the close hook (8.6c2).
      Done 2026-10-20 (SWE-2)
    - [ ] 8.6d the help window (and the About popup's help link, completing ST-SIDE-06), its
      generated Messages topics and search, `4cc-studio help`
      (ST-HELP-01 to 04), and the log panel widget (ST-LOG-01 to 04); a notice's `OpenHelp`
      action opens the window at its target (8.6b leaves it a dismiss)
      - [x] 8.6d1 the catalogs' member texts, the core chapters, `help` and `--export`
        (ST-HELP-04). Done 2026-10-20 (SWE-2)
      - [x] 8.6d2 the help window (`egui_commonmark` `=0.25.0`; ST-HELP-01, 02, ST-SIDE-06).
        Done 2026-10-21 (SWE-2)
      - [x] 8.6d3 the log panel widget (ST-LOG-01 to 04) and its code links (ST-HELP-03).
        Done (SWE-2): `widgets/dock.rs` `BottomDock`, `widgets/log_panel.rs`; the grid
        cross-link waits for the grid (8.10)
      - [ ] when the first topic gains an in-doc link `[text](<id>#<slug>)`, `--export`
        rewrites it to `<id>.md#<slug>` and writes an anchor per topic slug (review A3-7: no
        shipped topic holds one yet, so the exported files have nothing to break)
      - [x] 8.6d4 a member wording pass over the three catalogs' texts (8.6d4a the compiler's
        234 rows, 8.6d4b the upgrader's 69 and the Save editor's 23). Done 2026-10-21 (SWE-2):
        `studio`'s `every_catalog_row_is_written_for_a_member` guards them
    - [x] 8.6e the window opens at 1440 × 900 points, centered, fitted to a smaller monitor;
      the help window at 90% of it (ST-WIN-01, ST-HELP-05). Done 2026-10-23 (SWE-2)
  - [ ] 8.7 `color_tools`' colour-picker popup (`plans/libs/color_tools.md` Phase 8), the
    Save editor's team colours; the kit menu icons wait for the Compiler's grid
    - [x] 8.7a the picker popup drawn in the Save editor's draft. Done 2026-10-21 (SWE-2)
    - [x] 8.7b `color_tools`' picker widget against the draft, tested headless. Done
      2026-10-21 (SWE-2, seven reworks over the S1-S3 and A1-A3 rounds): `picker::color_field`
    - [x] 8.7c a click on a suggestion or recent swatch commits and closes the popup (P23).
      Code done 2026-10-26 (SWE-2)
  - [ ] 8.8 `libs/team_widgets` (`plans/save_editor.md` "`libs/team_widgets`"): the card
    pickers and the violations list first, the tactics editor and its pitch with the view's
    slice 3
    - [x] 8.8a the Save editor's draft follows DBC7 as 20.0e left it. Done 2026-10-21 (SWE-2)
    - [x] 8.8b the crate and its `cards` module, and `paid` in `aatf`'s `card_limits`. Done
      2026-10-21 (SWE-2)
    - [x] 8.8c `team_widgets::violations`: the AATF rows grouped per player. Done 2026-10-21
      (SWE-2)
    - [x] 8.8 review: Astra over 8.8b-c and the SWE-2 loops, ended. Done 2026-10-21
  - [ ] 8.9 the Save editor's view in the five slices of `plans/save_editor.md` "Build order
    and verification", with the `session/` wiring each needs (undo, selection, recent files,
    the FPC toggle, AATF quick actions) and the placeholders of 8.3 gone; each slice's SE-GUI
    scenarios cited
    - [x] 8.9a slice 1's skeleton: Open, the team selector and list, the info strip, undo
      (SE-GUI-01, 02, 04). Done 2026-10-21 (SWE-2); reviewed (S1, A1)
    - [x] 8.9b1 the Abilities & Skills tab, with the 8.9a review's fixes. Done 2026-10-21
      (SWE-2); review closed 2026-10-22
    - [x] 8.9b2 the Appearance tab, slice 1's last part (SE-GUI-07, 20, 22's Appearance part).
      Done 2026-10-22 (SWE-2)
    - [x] the portable name limits (`plans/pes_savefile/operations.md` "Team TOML", Portable
      text limits; DECISIONS 2026-10-21): `portable_text_max` and `crop_player_text`
    - [x] the roster-order rule (`plans/aatf_rules.md` "The host"; DECISIONS 2026-10-21) and
      drag reorder (SE-GUI-08), in two slices (8.9c1, 8.9c2). Done 2026-10-22 (SWE-2)
    - [x] 8.9d slice 2 (Team tab, batch operations, FPC; 8.9d1-d4). Done 2026-10-22 (SWE-2):
      parity with 4ccEditor ruled (DECISIONS 2026-10-22 "Slice 2's parity")
    - [x] the FPC hide IDs (55, 11) as a suite-common setting the editor and the Team compiler
      both read (SE-GUI-31, TC-BIN-43). Done 2026-10-22 (SWE-2): `fpc::HideIds`
    - [x] 8.9e slice 3, the Tactics tab (8.9e1 settings, 8.9e2 lists, 8.9e3 pitch; SE-GUI-12,
      25). Done 2026-10-23 (SWE-2), reviewed
    - [ ] 8.9f the draft's second arrangement (the maintainer's request of 2026-10-23: several
      usability issues, mostly badly arranged elements; their own remarks to follow). A new
      page, `docs/prototypes/save_editor_v2.html`, beside the first until the maintainer picks,
      arranged on the lead's judgment, weighing a collaborator's remarks on the first draft
      (not yet ruled by the maintainer): the team's name and abbreviation always in view, not
      on a tab (invitational members edit them often); fonts too thin (seen on the draft only:
      check the built view too); the card count derivable from stats, registered position and
      height rather than listed; star rating and playing attitude toward Appearance (they act
      only in cutscenes); the stats and the playable positions aligned, the positions in the
      grid CF SS LWF RWF / AMF LMF RMF CMF DMF / CB LB RB GK. Each remark adopted or not, with
      why, written in the page beside its draft controls. Slice 4 waits for the maintainer's
      ruling; what the ruling changes in slices 1-3's built tabs becomes a step of its own →
      verify: the harness walks the v2 page as it walks the first, every SE-GUI scenario
      the first draft shows drawn in v2 too
      - [x] slice 1, the frame: body text at weight 400, the team bar above every tab
      - [x] slice 2, Abilities: Profile, the positions grid, one cards badge
      - [x] slice 3: star and playing attitude to Appearance, Tactics in three columns; the v2
        harness 53 of 53
      - [ ] slice 4: v2 is the base (the maintainer, 2026-10-24; DECISIONS "The second draft
        arrangement is the base"): the built tabs of slices 1-3 ported to `save_editor_v2.html`'s
        arrangement, after 8.9j4 (the comparator, panels the same in both drafts) and before
        8.9j5; the maintainer's fine tweaks follow on v2
        - [x] 4a the frame: the team bar, the tab strip in groups, the player header
          (SE-GUI-01, 10, 32). Done 2026-10-24 (SWE-2), reviewed
        - [x] 4b Abilities: Profile, the positions grid, stats in four columns (SE-GUI-06, 07).
          Done 2026-10-24 (SWE-2), reviewed
        - [x] 4c Appearance's groups in aligned cells, Tactics in three columns. Done
          2026-10-25 (SWE-2), reviewed (`.tmp/phase_08/8_9f/duck_rulings_4c.md`); reworks r3
          and r4 landed 2026-10-25 (r4: the view harness draws inside a `CentralPanel`, the
          default-pane tests at `DEFAULT_PANE` 1224 x 776; post-Astra round S2 over r2-r4 ended
          the 4c loop)
          - [ ] rework r2 with 8.15 (2026-10-25, SWE-2; the lead finished it): the window at
            1280 x 800 (ST-WIN-01, ST-HELP-05), the lists at 200, the instruction combos
            capped, the survivors' tests (DECISIONS "The Tactics settings column scrolls with
            the tab at 1280 × 800"). Open: the post-Astra SWE-2 round over r2-r4
        - [x] 4d the one cards badge, the player's own tier, through `Rules::player_tier`. Done
          2026-10-25 (SWE-2), reviewed
        - [x] 4d's post-Astra SWE-2 round S2: a failed tier check shows its error in the
          badge's place (DECISIONS)
        - [x] 4e the regular-weight body font, `resources/fonts/Ubuntu-Regular.ttf` (DECISIONS
          "The body font is Ubuntu Regular, bundled; every view test draws with the app's
          fonts"). Done 2026-10-25 (SWE-2), reviewed
    - [ ] 8.9j slice 4 (`plans/save_editor.md` "Configurable AATF rules", "Comparator",
      "Aesthetics transplant"), the AATF parts first (the maintainer's direction of 2026-10-24),
      each part its own commit and review:
      - [x] 8.9j1 the rules file's settings row and the quick actions row (SE-GUI-09, 32).
        Done 2026-10-24 (SWE-2), reviewed; Browse checked by hand in the built window
      - [x] 8.9j2 the AATF panel (SE-GUI-15). Done 2026-10-24 (SWE-2), reviewed
      - [x] 8.9j3 Live AATF (SE-GUI-30). Done 2026-10-24 (SWE-2), reviewed
      - [x] 8.9j4 the comparator (SE-GUI-13; 8.9j4a the tabs' terms, 8.9j4b the panel). Done
        2026-10-24 (SWE-2), reviewed
      - [ ] 8.9j5 the transplant (SE-GUI-14), redirected 2026-10-25 (the maintainer; DECISIONS
        "The aesthetics transplant writes the team's export, after 0.1.0; the save-to-save
        transplant goes"): the export-writing panel is built after 0.1.0 (its plan section);
        8.9j5c removed the save-writing transplant (8.9j5a's panel, the CLI command,
        `pes_savefile`'s operation, their codes, help, spec), keeping the comparator's r2 fixes
        (the one-line pick row, the focus given up); 8.9j5b is dropped:
        - [x] 8.9j5a the save-to-save transplant panel. Done 2026-10-25 (SWE-2); removed by
          8.9j5c
        - [-] 8.9j5b the two-pane browser: dropped with the save-to-save transplant
        - [x] 8.9j5c remove the save-writing transplant; SE-TRN-01 to 03 withdrawn. Done
          2026-10-25 (SWE-2)
    - [x] 8.9h the editor's tabs fit a narrow window: ruled a 960 × 600 window minimum
      (DECISIONS "The window has a 960 × 600 minimum"). Done 2026-10-25 (SWE-2)
    - [x] 8.9i a starter whose portrait the pitch cannot show draws the game's placeholder
      portrait (`resources/portraits/`). Done 2026-10-23 (SWE-2)
    - [x] 8.9g the Tactics pitch as PES 2021's own: the reference and the draft, slice 1 the
      look, slice 2 the portraits from `download/` CPKs. Done 2026-10-23, seen in the built
      window on PES 2021
    - [x] before slice 4, the draft draws Live AATF (SE-GUI-30). Done 2026-10-22 (lead)
    - [x] before slice 4, one team's AATF check measured: `rhai` built with `sync`,
      `check-aatf` in parallel (DECISIONS 2026-10-22). Done 2026-10-22 (lead)
    - [x] 8.9m the script's own work: `logic.rhai`'s `read_player` rewritten to copy less,
      logic 1's hash `ea75b5fb…` (DECISIONS 2026-10-25). Done 2026-10-25 (SWE-2)
    - [ ] 8.9k slice 5, the interchange dialogs (`plans/save_editor.md` "Build order and
      verification" item 5): Team TOML export and import with its sections (SE-GUI-16), the
      import dialog for `.4ccs`, `.4cct` and Texport (SE-GUI-17, with SE-GUI-22's and 35's
      aesthetics section under both settings), Apply aesthetics patch with its preview
      (SE-GUI-18), the Texport export checked in game per version (SE-GUI-29, manual). The
      libs hold every operation (`pes_savefile::interchange`); the draft has every panel
      (`docs/prototypes/tools/save_editor.js`). Slices, each one review (draft check against
      the app's theme first, brief, sidekick, review, merge, release-build check):
      - [x] k5a 2026-10-29 Apply aesthetics patch, the Savefile menu's floating panel
        (SE-GUI-18): `view/patch.rs`; release build checked on the real PES 21 save
      - [ ] k5b the interchange panel's import half (SE-GUI-16's import, SE-GUI-17's import,
        SE-GUI-22's and SE-GUI-35's import-dialog parts): the file picked decides the format,
        the sections the file holds offered, the legacy formats read-only, the version
        mismatch's warning list, the aesthetics section under the ownership setting. The
        lib applies a Team TOML whole (no section choice), so two sub-slices:
        - [x] k5b1 2026-10-29 the section filter: `session/import_sections.rs`
        - [ ] k5b2 the panel's import half on it (`legacy::read_squad`/`read_tactics`,
          `Texport::from_bytes` + `to_team_toml`, the notes list, the `.4ccs` pairing), the
          file pick row as the Comparator's, the floating panels' open flags folded into one.
          Built and merged 2026-10-29 (`ffd74ec9`): `view/interchange.rs`; release build
          checked on the real PES 21 save. The notes' scroll area (rulings L1) landed with
          k5b2r
      - [x] k5c its export half (SE-GUI-16's export, SE-GUI-17's export). Merged 2026-10-29
        (`ae9fd2a2`); release build checked 2026-10-30
      - [x] k5b2r the rework of SWE-2 round 1 on k5b2 and round 2 on k5a, and of k5c's round 1.
        Merged 2026-10-29 (`e194f974`, tests `17cfd6d8`)
      - [ ] k5d after 0.0.2, PES 2019's case in 0.0.2-pes19 (the maintainer, 2026-10-30: 0.0.1
        targets PES 2017 only):
        SE-GUI-29 manual, each Texport the panel exports imported by the game's own team import
        in PES 18, 19, 20 and 21 (the installs on G:; saves backed up first). Look: Pending looks
      - [x] k5e PES 2015-2017 Texports in the panel (no extension; on PES 2015-2017 the dialog
        opens on All files). Landed `81daabdb` (Opus sidekick); checked in PES 2017 2026-10-30
      - [x] k5f Texport export on PES 2017, from the open save (DECISIONS "PES 2017 Texports
        written from templates and the open save"). Landed `740388f4` (Opus sidekick);
        SE-GUI-29's PES 17 case checked in game 2026-10-30; the zero emblem's effect unseen
        (look: Pending looks)
      - [ ] k5g after 0.0.2, PES 2019's case in 0.0.2-pes19 (the maintainer, 2026-10-30: "the behavior applies to all
        versions, but we'll have to confirm it"): on PES 2015, 2016 and 2018-2021, import a
        team file in game and check whether the import writes the team name the DLC's team
        bin gives the target's id into the save, as on PES 2017 (`plans/save_editor.md` "Interchange
        formats in the UI", the Texport bullet); the bin's editing is future work, no plan yet.
        Look: Pending looks
      - [x] k5rev the cross-family review loop over k5a-k5c, ended 2026-10-30 (rulings
        `.tmp/phase_08/8_9k5/rulings_k5.md`; fixes merged `83fea5f2`)
      - [x] k5mut the loop's `mutants-diff` over k5a-k5c, ran 2026-10-30: 9 missed, all test
        gaps, tested in cv3 and the brief after it
    - [x] 8.9m the maintainer's pass over the 0.0.1 candidate (`30b14680`), 2026-10-29 (m1-m5:
      the medals off the UI thread, rounded corners, the light theme's greys, the running-game
      texts; m2 `draft_compare.py` compares fills and text colours). Done 2026-10-30; m4's
      pitch warning cross unseen (look: Pending looks)
    - [x] 8.9l 2026-10-29 the open/save scenarios no slice built: the unsaved-changes prompt
      (SE-GUI-05), the Open menu's discovered saves and Save as (SE-GUI-26), the version
      warning as a standing notice (SE-GUI-28), SE-GUI-21 in the built window, and
      `--edit-mode-closed` for the CLI and the compiler (SE-CLI-03, TC-SAV-17); with it the
      drafts take egui's spacing (l3d, DECISIONS 2026-10-29 "The drafts take egui's spacing")
      SE-GUI-21 manual: checked 2026-10-29 (lead, release build of `7be7ecb5`, PES 2021 from
        `E:/PES2021` driven by the pad, the maintainer's live save backed up first and restored
        by hash after, `.tmp/release_0_0_1/se21b/`): with the game running, Save on a renamed player (/3/'s
        QUADS ONLY to QUADS ONLYX) opened the dialog naming PES 2021 and asking to close Edit
        mode; Cancel left the live EDIT00000000's hash unchanged; with Edit mode closed in game,
        the dialog's Save wrote it (hash changed); back in Edit mode, Load ("Rigging Edit Data
        will erase current data") then Edit Player showed QUADS ONLYX, no restart
        (`.tmp/release_0_0_1/se21b/shots/12_dialog.png`, `11_after_load_squad.png`)
  - [ ] 8.10 after 0.0.2: the progress grid (with the log panel's cross-link: a warning or
    error line's click highlights its grid cell, `plans/core/gui.md` "Log panel", which 8.6d3 leaves for
    the grid), the run strip, the exports folder watcher and check cache, cancellation, the launcher (`quick_compile.bat` to its `--gui` form, `quick_upgrade.bat` dropped once the upgrader's view lands), the
    Compiler's and the Export upgrader's views (their drafts first), each with the larger
    window's rule and its `view/tests/stretch.rs` as 0.0.2-nostretch's slices; the notice action
    `OpenFolder` opens the system file browser for the compile's "Open output" (8.6b leaves it
    a dismiss; eframe's `open_url` opens a browser even for a local path, so not through it),
    and the Compiler's view stops requesting repaints itself (`ToolContext::emit` wakes
    the GUI since 8.6b); the first tool that publishes a notice during a CLI command gets a
    binary test of ST-CLI-01's notice half (8.6b proves it through the printer's loop only);
    the grid's row actions (`plans/team_compiler/gui.md` "Row actions": Open folder, one-click
    fixes, Upgrade on an old export, the empty grid's drop, Compile only this export and the
    hidden compile column, disabled exports greyed with Enable and Disable on a clash, the save
    findings opening the Save editor; accepted 2026-10-25 as P3-P5, P7-P9, P15's Part A)
    - [ ] the Team compiler's draft, approved 2026-11-14 (the maintainer) but not final
      (2026-11-20): one more look with the maintainer, for what could make it more useful,
      before the view is briefed. Its text already names it the Compiler; `docs/prototypes/tools/team_compiler.js`: a first
      version 2026-11-13 (the lead; renders `.tmp/0_0_2/tc_draft/` by `shot_tc.py`): the
      grid on sample rows in every cell state, the row actions, the ID cell's entry, the
      detail panel, the split Compile button with its modes, the run strip, a fake run and
      its cancel, the log's cross-link, the draft controls and the scenario walk. To
      refine with the maintainer before `view/grid.rs` is briefed. Ruled 2026-11-13 (the
      maintainer): the referee row's 24-35 on a second line is fine (FoxDen's referee
      script needs one slot per referee, so a row will rarely pass 23); the legend stays,
      above the grid with a toolbar's line under it; every position is drawn, a faint
      square where the export has nothing (it counts as the plan's empty column), so a
      row's button is easy to match to its row; a vertical line after the ID column and
      after the last kit column, each the panel's whole height, clear of the cells on
      both sides and not dimmed with a disabled row; a portraits cell between the
      players and the kits (how many of the row's players have a portrait, in the colour
      of the worst). A portrait's finding colours the portraits cell, and the player's
      own cell too only when the portrait is in the player's folder, not in
      `Portraits/`: the compiler needs no change for it, a portrait finding's scope is
      the file's own path (`deep/portrait.rs`, `IssueScope::File`), which names the
      folder; `portrait_conflict` is the export's (it skips the export) and stays on the
      row's label. `plans/team_compiler/gui.md` "Task-to-cell mapping" changes with it. Also ruled: checked
      is `#800080` and done `#008000`; every margin of the grid is 8; a title row (Team,
      ID, Players, Portraits, Kits) that stays in view, solid, with no tick-all checkbox
      in the compile column; the detail panel squeezes no column (the grid scrolls
      sideways); an untagged row reads "Full or Midcup?" in the warning colour and an
      old one "Old export format" in the error colour; a clashing archive is moved to
      the exports folder's `unused/`, not `old exports/`, which is for exports in the
      old format (`plans/team_compiler/gui.md` "Row actions" says `old exports/` today). Later the same
      day: a check's warning or error is a thick yellow or red outline on the cell's own
      colour (checked, or shallow-checked, so the two archive states become one), solid
      yellow or red once the compile has gone through the cell (`plans/team_compiler/gui.md` "Cell states"
      and its colour table change); a skipped export's cells are blocked from the check
      on, not from Compile; a row's message and its answer buttons stand over the
      Players columns (the reassign question hides the row's cells while asked), its one
      small button right of the last line; the legend is two rows of related pairs under
      a "Legend" title and folds, staying folded across launches; the run strip sums the
      grid up at rest (exports, skipped, with errors, with warnings); a kit's two colours
      fill the cell's foot edge to edge; the portraits column is 50 wide; a disabled or
      unticked row is flat grey; the detail panel is a window the member places and
      finds again where it was left, with room for a player's 3D preview; cells change
      scattered over the grid as parallel tasks end; a team's label takes the export's
      outcome once every cell of its row is through the run (done, done with warnings or
      done with errors, the worst of its cells and its own findings, a skipped cell
      counting as an error); the legend's archive entry reads "Shallow check done
      (archive)" on two lines. All go into
      `gui.md` when the draft is accepted. Ruled 2026-11-13 night (the maintainer): a blocked
      cell's dotted red outline and the kit swatch as two flat halves stand; the detail
      panel is the placed window above, not a panel at the grid's right. Drawn the same night
      (the lead; shots by `.tmp/0_0_2/tc_draft/shot_tc4.py`), for the maintainer's look: the
      file drop on the grid (outlined while dragged; a name already there refused, naming it),
      the Edit mode question at the save's step, the autorun (close only on a run with no
      error), a check arriving live. Looked at the same night (the maintainer): the rest
      stands; fixed (`shot_tc5.py`): a live check starts every cell Unchecked, a few at a
      time turn Checking and each its result a few ticks later, and the drop's refusal is a
      Warning notice (orange). 2026-11-14: a compiling cell's bold label gave way to a 2-point
      bar sweeping its bottom edge over any ring (the maintainer's pick over a segment
      going around the edge; DECISIONS "a compiling cell sweeps"). The host page's compiler stub (`startCompile`, its three-row table) is now
      unreachable and goes when the draft is accepted
  - [ ] 8.23 after 0.0.2: edits made in PES come back to the export
    (`plans/aesthetics_export/settings_toml.md` "Edits made in PES come back to the export";
    DECISIONS of 2026-10-25; the maintainer's "even if Studio was not running" first):
    - [ ] 8.23a the in-game check first (PES 21, a save with a compiled team's patch applied,
      a backup before; after 8.36c): a motion and a face edit in Edit mode, which keys the
      record then holds → verify: `manual:` lines with captures; the plan's key list corrected
      to what it found. Look: Pending looks
    - [ ] 8.23b the compile record beside the patch (its format, written by every compile with
      a save step, run or skipped) → verify: a compile test reading it back
    - [ ] 8.23c the comparison and the notice (at start, on the views opening, before a
      compile, on the save's change), Bring into the exports, the conflict choice, the
      compile's Warning over an unbrought edit → verify: tests over a fixture save edited as
      PES would (a motion on one player, a face on another), with Studio "not running"
      between the compile and the edit (a fresh shell over the record)
    - [ ] 8.23d Apply to the save once PES is closed (a pending patch) → verify:
      the running-PES probe faked; nothing written while it reports PES running
  - [ ] 8.24 after 8.23, research: previewers outside PES for the in-game face and the motions
    (the maintainer wants it looked into, 2026-10-25), in Blender or in Studio's planned
    `libs/model_viewport` (`plans/core/README.md`'s "Player aesthetics viewing" row): first
    what the game's files and Marco's RE notes give (the base head and the sliders' shape
    rules; the motion archives and what selects a clip), then a plan section, not code
  - [x] 8.25 the sidebar as a rail that opens on hover over the view (DECISIONS "The sidebar
    is a rail that opens over the view on hover"; ST-SIDE-02): the drafts' half merged
    `7b5ed0ef`, the code half 2026-10-25 (SWE-2); seen in the built window at 1280 × 800
  - [ ] 8.22 after 0.0.2: the accepted usability items outside the grid (2026-10-25, P1, P2,
    P6, P10-P14; DECISIONS "Usability proposals P1-P14 accepted"; 2026-10-26, P18-P20, P22,
    P32-P36, h to j), each its own commit:
    - [ ] 8.22a the finding line's disposition phrase (`plans/core/architecture.md` "Every
      finding line says what was lost") → verify: a CLI test per phrase, the pinned lines
      updated
    - [ ] 8.22b the `closest` context field on five codes (`plans/team_compiler/messages.md`
      "A failed name lookup names the nearest existing name") → verify: a test per code with a
      real misspelling (`fpc-on`, a shared folder one letter off), and one with nothing near
    - [ ] 8.22c the deep check runs every check whose inputs exist before a compile
      (`plans/team_compiler/gui.md` "Deep check": a gap, not a proposal): first an audit of
      every code the catalog marks "reported by `compile`", each kept with its reason or
      moved; then `fmdl_texture_not_found`'s lookup shared by both callers → verify: the
      audit's list; a deep check reporting a missing texture with no compile, a test pair
      keeping the two callers in agreement
    - [ ] 8.22d `model_name_type_misplaced` and `kits_folder_missing`
      (`plans/team_compiler/messages.md`) → verify: a test each, and the full-body team not
      warned
    - [ ] 8.22e an invalid `settings.toml` keeps the installed look
      (`plans/aesthetics_export/settings_toml.md`) → verify: a Fox compile over an installed
      row keeps it; the defaults with none installed; the pre-Fox patch holds no entry
    - [ ] 8.22f the savefile in the preflight (`plans/team_compiler/pipeline.md` "6.
      Post-processing") → verify: another version's save reported before any export is read
    - [ ] 8.22g the first run finds the PES version and folder (`plans/core/distribution.md`
      "Data location") → verify: a dialog test over a fake Documents and install tree (no real
      install reached); nothing found keeps the defaults
    - [ ] 8.22h the context the findings' texts need (P22, accepted 2026-10-26;
      `plans/team_compiler/messages.md` "A finding carries the context its text needs"),
      before the texts that read it → verify: a test per code reading the new field
    - [x] 8.22i 2026-11-12 (`88c91de1`): `upgrade <export>...` (P20), built as 0.0.2-upgall
    - [ ] 8.22j the upgrader's view items (P18's pair, P19's dialog, P32-P36;
      `plans/export_upgrader.md` "GUI"), with the view (8.10), drafted first (8.17b) → verify:
      EU-GUI scenarios for each, written with the view's Acceptance before it is built
  - [x] 8.11 egui 0.36.2 to 0.37 (`spec/core/gui.md` "egui version discipline"). Done
    2026-10-23 (SWE-2, with 8.13): `egui-phosphor` dropped for
    `resources/fonts/phosphor-subset.ttf` (since replaced by 8.14's `icons.ttf`)
  - [ ] 8.12 after DBC: the Texport coverage audit, the measurement that decides whether a
    Team TOML can stand for a tactical export (`plans/aesthetics_export/player_folders.md`
    "Complete export"; agreed 2026-10-21). Every byte of the real texports of each version
    (`spec/pes_savefile/operations.md` "Texport") classified: carried by Team TOML, worth
    adding to the half that owns it (the candidates, set in game: the team record's emblem, stadium settings, rivals,
    callname, banners, goal-net colours, manager name, PES 18's kit slots; the tactics'
    lineup slots 32 to 39 on PES 19-21, Switch Preset Tactics, Auto Lineup Select; a
    player's commentary name and PES 20/21's print name), or not a tactic (the 660-byte block,
    the tail); the meanings checked in game, and for PES 2021 against Marco's reverse
    engineering of the game's executable (`C:/Data/4cc/Tools_Marco/GodotFootball/docs/re/`;
    the repository is in active evolution, so before the audit starts its new commits are
    pulled, `git pull --ff-only` in that clone, a pull that cannot fast-forward reported to
    the maintainer rather than forced;
    index `README.md`, every claim tagged observed or inferred: `pes-re-matchlogic.md` for the
    player and team records, `pes-kit-assembly.md` §8.1 for the EDIT save's layout); each
    "worth adding" a Team TOML key; then a test
    per version taking a real texport to Team TOML and back, byte for byte apart from the
    bytes classified as not a tactic
  - [x] 8.13 the release stage after the version: `v0.0.1 pre-alpha` from
    `shell::version_label`. Done 2026-10-23 (with 8.11)
  - [x] 8.14 the shell's own icon font: `scripts/fonts/make_icons.py` builds
    `resources/fonts/icons.ttf` from the 13 tool SVGs and the Phosphor glyphs (DECISIONS "The
    final icons approved; …"). Done (`581aef24`, merged `a87ae977`; Opus sidekick); the rail
    seen in the built window 2026-10-30
  - [ ] 8.15 (code done 2026-10-25 with 8.9f4c's rework r2; the built window's check open) the window's default to 1280 × 800 (the maintainer, 2026-10-25; `spec/core/gui.md`
    "The window's size", ST-WIN-01, ST-HELP-05; DECISIONS "Studio opens at 1280 × 800
    points"): `studio_core`'s `DEFAULT_SIZE` and its tests, the help window's; the Save
    editor's Tactics lists at 200 points, and its tests named for 1440 × 900 → verify: the
    scenarios' tests; the built window at 1280 × 800 on the FHD monitor, the Tactics tab in
    three columns. Look: Pending looks
  - [x] 8.16 the drafts as one host and a script per tool (DECISIONS "The drafts are one host
    page and a script per tool"): `docs/prototypes/index.html`, `tools/*.js`,
    `scripts/drafts/bundle_drafts.py`. Done 2026-10-25 (side-lead A, SWE-2)
  - [ ] 8.17 the Export upgrader's draft, `tools/export_upgrader.js` (`plans/export_upgrader.md`;
    `core/gui.md` "Prototypes before implementation"; side-lead track A, after 8.16): every
    screen the plan describes, with sample data; the lead's usability review first, ideas to
    `maintainer/proposals.html`; then the maintainer's remarks
    Drawn 2026-10-25 (side-lead A, SWE-2): every screen on real CLI runs' samples
    (`.tmp/8_17/runs.py`); DECISIONS "The view's arguments and report, where the plan was
    silent"; the usability review's ideas `maintainer/proposals.html` P32-P36. Waits for the maintainer's
    remarks
    - [x] 8.17c a second draft, replacing the first (the maintainer, 2026-11-13: two looks by
      the number of old exports, a grid as the Team compiler's from five on with three
      lines an export for the face, boots and gloves folders, and under five the source
      and destination files in two columns, turning as the run goes, "feel free to
      improvise"; then a toggle to pick either look whatever the count). Drawn 2026-11-13
      (the lead; renders `.tmp/0_0_2/eu_draft/` by `shot_eu.py`; the first draft's file
      kept beside them as `export_upgrader_single.js`). Improvised, for the maintainer to
      rule on: the view reads `old_exports` beside the app and writes into `exports`
      (no drop targets, no Output into field); one save for all the exports, the field
      red while any team is missing from it; boots and gloves cells stand under the
      player the save has wear them, by their four-digit ID, a shared folder under each
      wearer; the team's files as cells (Note, Logo, Common, Collar, IDs) and one cell
      counting what is left out, in place of the left-behind list; an export with an
      Error is not written and its cells turn to "Not written"; an export that cannot be
      read (team not in the save, already upgraded, output exists) is one line with its
      action; Full or Midcup is a combo at each row's right; the detail window is fixed
      at the right; after a run, Open the Team compiler in place of Move to exports and
      compile. Ruled the same day: the grid is good; the button that opens the Team
      compiler is always there and turns blue once an upgrade has finished; an action
      that follows a row's message stands beside the message, never in the rightmost
      column; the files look is the explanatory page's map (`docs/web/
      upgrade_explained.html`), a box per old folder left, a box per new one right and a
      line from each to where it goes, in the colour of what it carries, a hovered box
      keeping its lines and dimming the rest (the dimming ends 120 ms after the pointer
      leaves, as on that page, so going from box to box does not flicker; the files look
      accepted by the maintainer 2026-11-13; the labels `Old export` and `Upgraded export` are one row that stays above the exports as they scroll), a destination lighting as it is written
      (renders `flow_*.png` by `shot_flow.py`). Accepted as drawn 2026-11-13 night (the
      maintainer); `plans/export_upgrader.md` "GUI" and its EU-GUI scenarios rewritten
      from it the same night, Copy report drawn in the Upgrade button's menu, model-folder
      mode removed (0.0.2-nomodels). Not drawn: a referee export's row (8.17b). Approved
      2026-11-14 (the maintainer, after the Export type title, Files' Full export and
      Midcup export, and the missing team's Browse…, `726ea87f`)
    - [ ] 8.17b a referee export's row in the second draft (no save, no Full or Midcup
      combo; `plans/export_upgrader.md` "GUI"); the first draft's items (P18's pair, P19's
      dialog, P32-P36) are in 8.17c's draft or ruled out with it → verify: the bundled
      drafts build; the row reached in the draft
  - [x] 8.18 the host's layout edit toggle (DECISIONS "The drafts' host gets a layout edit
    toggle"). Done 2026-10-25 (side-lead A, merged `139355d9`)
  - [ ] 8.19 the host's remarks, links and longest values (`core/gui.md` "The host's other
    review aids"; DECISIONS "The drafts' host gets remarks, links, …"; after 8.18, with it on
    track A or the top lead's): the URL fragment helper and every tool script's state keys,
    the Remark toggle and "Copy remarks", the extraction script and `assets/longest.js`
    (real values only) → verify: Playwright opens a link to a dialog's state and the capture
    shows it; a remark's copied line names its panel and control and carries a link that
    reopens the state; with Longest values on, each tool's longest text is drawn (a capture per
    tool, looked at)
    - [x] 8.19a links (side-lead A, merged `144fedf1`): `#<tool>/<screen>?keys`
    - [x] 8.19b remarks (Opus, 2026-10-27): the Layout group's "Remark" toggle, "Copy remarks"
    - [ ] 8.19c longest values, briefed: `.tmp/phase_08/8_19/brief_8_19c.md`
  - [ ] 8.20 the host's overlay and draft shots (`core/gui.md` "The host's other review aids";
    after 8.19, whose links it walks; the overlay moved into 0.0.1 as 0.0.1ov): the Overlay control and the capture naming, the
    Save editor's first captures from the built window; `scripts/drafts/draft_shots.py` → verify: the
    overlay of the built Tactics tab on its draft at 1280 × 800, looked at; a one-line host
    stylesheet edit lists every tool's changed screens, reverted by hand, then lists none.
    Briefed: `.tmp/phase_08/8_20/brief_8_20.md` (the overlay needs built-window captures, which take
    the UI automation until `--gui` opens a save)
  - [ ] 8.26 the drafts' text drift side-lead A found (2026-10-25): the drafts show `v0.1.0`
    where the build shows `v0.0.1 pre-alpha`; the host's help chapters still say `4cc-studio`
    where the help files say `UnlimitedToolWorks`; with the preview pane on at 1280, the
    Ruleset editor's form clips its stat grid (the Ruleset editor waits for 0.1.0: fix with
    20.6) → verify: a capture of each, looked at
  - [x] 8.27 the notice in the bar's middle, in its kind's colour (DECISIONS "A notice sits in
    the bar's middle, in its kind's bright colour"; ST-BAR-02 to 04). Done 2026-10-25 (SWE-2)
  - [x] 8.28 the aesthetics lock behind the "Aesthetics from exports" setting, off by default
    (DECISIONS "The aesthetics lock is a setting, …"; SE-GUI-35). Done 2026-10-25 (SWE-2)
  - [x] 8.30 WITHDRAWN (the maintainer, 2026-10-28: "keep the Nation input as it is, with no
    label"); its draft stays on `side/drafts2`, unmerged
  - [x] 8.31 a click on the toolbar's `• unsaved changes` opens the list of the changes in a
    popup (DECISIONS "The changes list is a popup under the unsaved-changes label"; SE-GUI-43).
    Done (`832caa83`, merged `0bf68fc4`); seen in the built window 2026-10-30
  - [x] 8.32 the pitch draws the formation's name in its corner: built as 0.0.1x, approved
    2026-10-28
  - [ ] 8.33 in 0.0.2-flatpitch (the maintainer, 2026-10-27): a menu entry that saves a picture of
    the team's formations to a folder: a grid, one row per preset, one column per formation.
    Drafted and approved 2026-10-28 (DECISIONS 2026-10-27 "Formation pictures, the
    formation's name and the nation names: the maintainer's answers", "The drafts' looks of
    2026-10-28"; revised 2026-10-29, DECISIONS "Formation pictures draw hidden formations,
    label Fluid formation, and put a target on its own line"); the plan written
    (`plans/save_editor.md` "Tactics editor", "Formation pictures"); left: the code, with
    8.34, its scenarios written with it
  - [ ] 8.34 with 8.33 in 0.0.2-flatpitch (the maintainer, 2026-10-27): the same for a range of teams, one
    picture per team, in one go. Drafted and approved with 8.33 (above); left: the code
  - [x] 8.35 a ruleset carries its event's logo and page (P45; DECISIONS "A ruleset carries its
    event's logo and page"; RE-FILE-09, SE-GUI-44, RE-GUI-19): a `aatf`'s keys, b the Save
    editor's logo and the Ruleset editor's fields, c every ruleset file's logo and page. Done
    2026-10-30 (Opus sidekick, merged `d4cdcdf4`, `a61ee964`, `eb699752`)
  - [ ] 8.36 every engine's looks through the savefile patch, no stripped saves (the
    maintainer, 2026-10-30; DECISIONS "Every engine carries a player's looks in the savefile
    patch; no stripped saves")
    - [x] a the plans: stripped saves out, the full-look patch designed
      (`plans/pes_savefile/operations.md` "Aesthetics patch (new)"; DECISIONS "The full-look
      patch writes the player's own appearance id…")
    - [x] b before 0.0.1, the Save editor: `strip-and-seed`, `check-stripped` and every
      stripped mark removed, `diff` gains the appearance id (SE-DIF-05), SE-GUI-20 withdrawn.
      Done 2026-10-30 (Opus sidekick, branch `s36b`)
    - [ ] c after 0.0.1 and before 0.0.2 (VGL 27 plays on PES 2019, a Fox version, with no
      stripped save; `plans/core/development_plan.md` "The /vg/ League 27"), the full-look
      patch and the Compiler: the aesthetics patch on
      every engine (it is Fox-only today) gains the appearance block, the boots and gloves IDs
      over the whole allocation, filled after their tasks commit, and clears a compiled
      player's base copy, or writes the one his `<id>.copy` link names (the link's validation
      in the compiler, its generation in the Export upgrader's step 8:
      `plans/aesthetics_export/player_folders.md`, `plans/export_upgrader.md` "What it does");
      `apply-patch` writes them; the Fox player-table writer and its
      messages go; the spec with it (the inventory's S22-S54, S56-S57's table parts, S61,
      S64-S66, S68, and GLOSSARY's D1 and D2), its Acceptance written first → verify: the
      gates; TC-SAV-12 (a PES 17 compile writes no patch) rewritten; the library parity's
      difference classes re-diagnosed
      - [x] c0 the Acceptance (2026-11-01): `plans/team_compiler/README.md` "8.36c scope",
        SE-PAT-07 to 11, EU-SET-12, 13; four DECISIONS entries; reviewed
        (`.tmp/0_0_2/s36c/rulings_c0.md`)
      - [x] c1 the patch's look: `pes_savefile` and `apply-patch` (`[appearance]`, the two IDs
        over 14 bits, `base_copy_id`). Done 2026-11-01 (SWE-2, merged `81301b9e`)
      - [x] c2a the compiler's IDs filled once the tasks' outcomes are known, the patch on
        pre-Fox (TC-SAV-19, TC-BIN-50 to 59, TC-MOD-95 to 99). Done 2026-11-05 (SWE-2)
      - [x] c2b the player tables go (about 2,400 lines; TC-BIN-37 and TC-SAV-07 removed).
        Done 2026-11-05 (SWE-2)
      - [ ] c3 the `<id>.copy` link: the compiler's three checks and the patch's
        `base_copy_id`; the upgrader's step 8 → verify: TC-STR-21, TC-SAV-20, EU-SET-12, 13.
        Settle first (review S-36c r1, A1): an entry holding `base_copy_id` alone (an
        invalid-settings player whose folder keeps its link) leaves a record at appearance id
        -1 reading its look from the game's table, since `apply` writes the own id only with
        an `[appearance]`; the spec is silent on whether a link alone makes the record his
      - [ ] c4 the spec moved (the inventory's S22-S54 …), help and catalog texts (among them
        `messages/catalog.rs`'s "motions and names on PES 2018 to 2021, aesthetics on PES 2015
        to 2017", wrong since c1), the drafts' message rows, the library parity re-run and its
        classes re-diagnosed; then 8.36c's review: Astra over its code (checkpoint (b)) and
        one `mutants-diff` over c1-c4
        - the texts done 2026-11-05 (two SWE-2 sidekicks): the built scenarios moved to the
          spec, TC-SAV-20 and TC-STR-21 staying for c3; the 8.36d checks stay in the plan
        - the library parity done 2026-11-05 at `46105675` (tag `c4` against `close6`): one
          new intentional class, I35
        - the review done 2026-11-06 (the duck rulings' "S-36c r1" to "r3", "A-36c part 1"
          and "part 2")
        - `mutants-diff 8bcbf720` over 8.36c and 0.0.2-poc1 to poc11, run 2026-11-08 and
          2026-11-09: 1 missed, equivalent, no missing test
          (`.tmp/0_0_2/poc11/mutants_diff_8bcbf720/triage.md`)
        - the library parity p11 (release build of `ee80b0ac`): every change traced, five new
          intentional classes I36 to I40 (`.tmp/library_parity/library/diagnosis_p11.md`)
    - [ ] d in game, before 0.0.2 (PES 2019 first, VGL 27's game and the likeliest for the
      4cc's spring cup, then PES 2021 and PES 2017; the maintainer, 2026-11-04): a
      patch applied to an
      unstripped save shows the compiled look in Edit mode, boots and gloves included, on a
      player whose base copy named another and on one whose edit flags were clear
      (`scripts/ingame/`, saves backed up); with it, the FoxDen gloves issue rechecked, and
      SE-GUI-34 proved again in its patch form (a `SE-GUI-34 manual: checked` line). Look:
      Pending looks
  - [x] 8.37 a dialog's title centred over the dialog (DECISIONS "A dialog's title is centred,
    as the windows' titles are"), with 8.37b Set all and Bump all stats' number on the
    sentence's row. Built (Opus sidekick, merged `72df1947`); seen in the built window
    2026-11-01
  - [x] 8.38 (closed by 0.0.1's cut: the maintainer, 2026-11-05) the built-window check of
    2026-10-30 (`.tmp/phase_08/8_30c/findings.md`): a the number dialogs' 52-point box (merged
    `bd940680`), b a disabled widget's look, `studio_core::widgets::add_enabled` (merged
    `6cb1a234`). The Fox ownership line's Apply link went unchecked (look: Pending looks)
  - [ ] 8.39 after 0.0.2 (the maintainer, 2026-10-30): rename `dds_convert` to `image_convert`.
    The crate decodes every picture the suite reads (PNG, DDS, FTEX), resizes, writes PNGs and
    encodes the games' textures; the Save editor, the Ruleset editor and `team_widgets` use it
    for logos and portraits that are never a DDS, so the name tells a reader the wrong scope.
    `image_convert`, not a split into a raster crate and a texture crate: the decode is shared
    by both halves (CONTRIBUTING guardrail 3, no over-splitting), and the name pairs with
    `model_convert`. Mechanical: the package and folder, every `dds_convert::` path and manifest
    (76 files), `plans/libs/dds_convert.md` and `spec/libs/dds_convert.md` renamed with their
    pointers, GLOSSARY, AGENTS.md's read-order row, `just crate-map`; DECISIONS and LOG keep
    the old name. Only with no branch open and no mutation run queued (it touches every tool)
    → verify: `git grep dds_convert` lists only DECISIONS and LOG; `just gates`; `just
    doc-pointers`
  - [x] 8.40 a keyboard edit settles without leaving the control: a focus held by a widget
    with no text cursor counts as settled. Done 2026-11-02 as `4f37cab7` (with 8.42), on `f1e`
  - [x] 8.41 the shirt name and the team's short name take upper case only. Done 2026-11-02
    as `091c725b`, on `f1e`
  - [x] 8.42 the undo history merges a run of steps to one value within a second. Done
    2026-11-02 as `4f37cab7`, on `f1e`
  - [x] 8.43 PgUp/PgDn step a page of the rows the list shows in full. Done 2026-11-02 as
    `091c725b`, on `f1e`
  - [x] 8.44 the wheel steps the Tactics preset and formation selectors, wrapping round
    (DECISIONS "The wheel steps the preset and formation selectors, wrapping round"). Done
    2026-11-02 as `cbbb727e`, on `f1e`
  - [x] 8.45 Ctrl+C and Ctrl+V on the player list copy a player's stats (DECISIONS "Ctrl+C and
    Ctrl+V on the player list do what Copy stats does"). Done 2026-11-02 as `a4545968`, on
    `f1e`
  - [x] 8.46 one Up or Down press in an edited number field steps it one (egui 0.37's
    `DragValue` steps by its drag speed, 0.25). Done 2026-11-02 as `091c725b`, on `f1e`
  - [ ] 8.29 the text pass: every line a member reads reread against CONTRIBUTING's "That text
    flows naturally" (the maintainer, 2026-10-25; DECISIONS "Text a member reads flows
    naturally, and every existing line is swept once"), rewritten where it reads dry, its meaning
    kept; the tests that pin a string follow it. One slice per surface, the 0.0.1 tools first;
    each slice's diff read line by line by the lead before it lands (a rewrite that changes what
    a line says is a finding) → verify: the gates; every rewritten line read in its place (the
    draft or a headless frame)
    - [ ] a the Save editor, in three slices:
      - [x] a1 the window's strings, tooltips and notices under `view/`: 34 lines rewritten
      - [x] a2 the message catalog and the CLI's output and `--help`: 76 lines rewritten; the
        spec's Messages table keeps its developers' wording
      - [x] a3 the help chapters and the draft `docs/prototypes/tools/save_editor.js`: 93 lines
    - [x] b the shell (`studio_core`) and the drafts host `docs/prototypes/index.html`: 35 lines
      rewritten (Opus, worktree `wt-829b`)
    - [ ] c the Compiler and the Export upgrader: their message catalogs, help chapters,
      CLI output and the comments injected into `settings.toml`/`config.toml`/`materials.toml`
      (side-lead U, branch `side/text`), in six slices, the last outside its locks. Left by
      c4-c5 for whoever holds the files: `libs/fpc`'s hide-ID refusals keep a "key = value:
      must be" shape; the Save editor's `view/portraits.rs` keeps "not a CPK the reader
      accepts"; `spec/aesthetics_export/player_folders.md` (about line 720) says a shared
      folder with no model of its kind takes no ID without the `.common` link case; the drafts
      host's mock catalog (`docs/prototypes/index.html`) and `tools/export_upgrader.js` quote
      the upgrader's old texts:
      - [x] c1 the Team compiler's catalog, rows 1-80: 89 texts rewritten (SWE-2 S1-S3)
      - [x] c2 the catalog's rows 81-161: 115 texts in all (SWE-2 S1-S5)
      - [x] c3 the catalog's format-check and structure-pass rows (162-237): 58 texts, the
        upgrader's 13 copies following
      - [x] c4 the Team compiler's CLI output, `--help`, window strings and help chapter
        (c4a-c4d), and c4e the fact audit of both tools' help chapters against the catalog and
        the spec (c4e1-c4e3: 60 facts made true)
      - [x] c5 the Export upgrader: c5a its catalog (45 texts), c5b its `cli.rs` and help
      - [ ] c6 the injected comments: they live in `pes_savefile` (`settings_toml/`, the
        `settings.toml` and `ingame_face.toml` headers and key comments) and `kit_config`
        (`toml_form.rs`, `config.toml`), outside side-lead U's locks: for whoever holds those
        crates (no code writes comments into `materials.toml` today)
    - [ ] d `libs/aatf`'s violation texts and the other tools' catalogs and help
      - [x] d1 before 0.0.1: no GUI or CLI text prints an internal identifier (inventory
        `.tmp/phase_08/8_29d/inventory.md`): d1a the AATF rows in words (DECISIONS "Each
        violation kind's name in words, …"), d1b below. Done 2026-10-30
        - [x] d1b the rest of the inventory: d1b1 the fields no view draws (merged `5aaf3b12`),
          d1b2 below
          - [x] d1b2 2026-10-30 the team file notes, the badge's and quick actions' refusals,
            the card and playing-style labels, the man-marking row (DECISIONS "card names, the
            team file notes, man marking and the tier refusals in words"). Merged `5da2edf0`
      - [ ] d3 the stats named as each version names them (P43, accepted 2026-10-29; DECISIONS
        "Stats are named as the save's PES version names them", "Before 0.0.1, only PES
        2017's stat names are checked"):
        - [x] d3a PES 2017's stat names (`pes_savefile` `labels::stat_name`). Done 2026-10-30
          (Opus sidekick, merged `34639d07`)
        - [ ] d3b after 0.0.2, PES 2019's in 0.0.2-pes19: every other version's names, gathered from each game (the
          player.rs doc comments name no versions; Marco's RE reports for PES 2021, the
          games on G: for the rest) → verify: a test per version against the gathered names.
          Look: Pending looks
      - [ ] d2 after 0.1.0, with Phase 20 (the maintainer, 2026-10-29): the Ruleset editor
        shows the game's names (Attacking Prowess) and writes the rules-file ids
        (`attacking_prowess`)
    - [ ] e the help chapters' shape (the maintainer, 2026-10-29: "each paragraph is a single
      extremely long line, which explains multiple facts one after the other"; DECISIONS "Help
      topics in short paragraphs under subheadings, lists for parallel items", CONTRIBUTING's
      bullet): every topic restructured, its facts kept, each change read by the lead line by
      line (a fact lost or changed is a finding)
      - [x] e1 before 0.0.1: the Save editor's eight chapters and the two core ones. Done
        2026-10-30 (Opus sidekick, merged `db1b149d`); e1r the search's labels as plain text
        (merged `1c1fb403`). The read in the built window was left open (look: Pending looks)
      - [ ] e2 after 0.0.2: the Compiler's, the Export upgrader's and the other tools'
        chapters
    - [ ] f a second read of every string a member reads, for how a person would say it
      (the maintainer, 2026-11-12, after two lines of the Save editor's import panel that
      a1 had passed or that came after it: "and the file decides the format", unnecessary,
      and "one undoable edit; the changes stay unsaved until Save.", shorthand; both
      reworded that day). A sidekick per surface, as a to c ran, each diff read line by
      line by the lead; the Save editor's `view/` last, after 0.0.2-clip, -line and -undo,
      which edit the same files. To settle in it: the `Applied. The changes stay unsaved
      until Save.` notices beside the reworded line, which now says "until you click Save"
      - [ ] f1 the shell (`studio_core`) and the drafts host
      - [ ] f2 the Compiler and the Export upgrader (window, CLI, catalogs, help)
      - [ ] f3 the Save editor (window, CLI, catalog, help, draft)
  - [ ] 8.21 the app's font in the drafts (`core/gui.md` "The host's other review aids"; after
    8.9f4e picks the bundled font): `@font-face` rules at egui's sizes → verify: one width
    constant's row (`TOP_ROW_ONE_LINE`) measured in the draft and in the built window agree
    within a few points
- [x] 8.2 the shell's draft, `docs/prototypes/shell.html` (2026-10-19, side-lead E, SWE-2 in
  seven slices): every `core/gui.md` shell screen, walked by a 29-check headless harness
- [ ] 8.47 for 0.1.0 (the maintainer, 2026-11-13): the Compiler takes hand-made mip
  levels beside a raster texture, so a model can show or hide with the camera's distance:
  `body_mip2.png` gives `body`'s mip 2, `body_mip4-6.png` mips 4 to 6, `body_mip8+.png` mip 8
  and below, and `_mips_alpha255-255-127-0` on the texture's name fades it. Plan written
  (`plans/team_compiler/pipeline.md` "3. Per-model-folder parallel steps", DECISIONS
  "hand-made mip levels"); left: its acceptance scenarios, then the code
- [ ] 8.48 for 0.1.0 (the maintainer, 2026-11-13): the compiler writes star 0 on a player with
  FPC on and no kit-dependent textures, so the pre-match screen shows a teammate whose kit
  shows; `star = -7` keeps the value, "only for special occasions"
  (`plans/aesthetics_export/settings_toml.md` "From 0.1.0, a pre-match screen that shows the
  kit"); after 0.0.2-presentation. Left: the scenarios, then the code
- [ ] 8.49 the Save editor's Squad screen (the maintainer, 2026-11-14): one player per row,
  one stat or card per column, edited in place; it takes a larger window's extra width and
  height (`plans/save_editor.md` "The Squad screen"). Drafted with the maintainer
  2026-11-14 (`docs/prototypes/tools/save_editor.js` `squadView`); scenarios SE-GUI-94 to
  101. Left: the code, in four slices (the read-only table and the selector; sorting, the
  Show row and Live AATF's marks; keys and block edits; the pick lists); no release named
  yet. The grid: `egui_extras` is already linked (through
  `egui_commonmark`, the help window's), so using it costs no download and little size; its
  `TableBuilder` keeps a heading row in place and sizes columns to their content, but cannot
  freeze columns at the left. The first slice tries it (with the frozen columns painted at
  its scroll offset) beside a grid of our own, and keeps the simpler; the maintainer allows
  the direct dependency (2026-11-14)

### Releases 0.0.1 and 0.0.2 — the preview builds

- [x] 0.0.1 the preview build for DBC: the Save editor, the Ruleset editor as WIP
  (`plans/core/development_plan.md` "Divegrass Before Christmas"). Shipped 2026-11-03: tag
  `v0.0.1` on `c97ce914`, built by CI (run 41564671137), published as "Unlimited Tool Works
  v0.0.1 pre-alpha". Its steps (0.0.1a to 0.0.1z, la, cv, ov, rn, the cut f) were collapsed
  2026-11-16; their record is this file at `647da0fb`. Their open items moved: the
  launchers' check to 0.0.2's bundle (Pending looks), the rest under "after 0.0.2" below
- [ ] 0.0.2 the VGL preview (`plans/core/development_plan.md` "The /vg/ League 27 (VGL)").
  Prerequisites: 8.36c and 8.36d (the full-look aesthetics patch, checked in game on PES
  2019), before the PES 2019 rehearsal; the readme's VGL section gains the patch's place in
  the organizer's order and the AATF check of the official save (the plan's paragraph)
  - [ ] 0.0.2-pes19 the Save editor's writes on PES 2019 checked in game before the readme
    calls PES 2019 supported (the maintainer, 2026-11-16: VGL 27's managers use it there).
    Already seen on PES 2019: an unchanged import leaves the save byte for byte, and the
    edited-player flag gates a name (0.0.2-se19; `spec/pes_savefile/model.md` "What the
    player's edit flags gate in game"); the formation looks (2026-11-13); the compiled looks
    of the proof of concept's rehearsal. Left:
    - [x] a census first, done 2026-11-16 (SWE-2, `.tmp/0_0_2/pes19/census.md`): 107 rows,
      about 160 stored fields, nearly none seen in game on PES 2019. PES 2019's player
      record (188 bytes) is laid out apart from PES 2021's (312), so almost every player
      field's offset differs; the tactics record is PES 2021's, its instruction values
      PES 2018's table. Nothing writes man marking but Copy preset and the Team TOML, and
      PES 2019's kit slots, measured in the team record, have no schema rows
    - each field not yet seen, read off PES 2019's Edit mode (pad and OBS, no focus; no
      cargo while PES runs), every other team intact (Invitational 1 above all, the case of
      the dropped tester's report). The save is a copy of the maintainer's own PES 2019
      save (`.tmp/ingame_saves/backup_19`), not the VGL 27 one, which this install refuses
      ("Create Edit Data?", 2026-11-13). The Team TOML reaches nearly every field the
      editor writes, and the GUI's controls write through the same codec, so one Team TOML
      with every value stepped apart, imported by the CLI into team 701, shows each
      field's offset:
      - [x] part 1, 2026-11-16 (SWE-2, `.tmp/0_0_2/pes19/rb/`): the stepped Team TOML
        imported with no note, exported back unchanged, and only team 701 differs among
        the save's 210 cup teams; `expected.md` says what each Edit-mode screen should show
      - [ ] part 2, the read-back in game, four players, the team and its three presets
        (SWE-2, `.tmp/0_0_2/pes19/brief_rb2.md`), once no cargo runs
    - a `.ted` import (the organizer's first step) loaded in game: only the window imports
      a `.ted` (the CLI takes a Team TOML), so it waits for a built-window run with the
      maintainer's yes
    - PES 2019's case of three per-version looks: its stat names (8.29d's d3b), a Texport
      the panel exports imported by the game (8.9k's k5d), and whether a team-file import
      writes the DLC bin's team name (k5g)
    -> verify: a `manual: checked` line per field group; a wrong field becomes a test, red
    first, and its fix
  - [x] 0.0.2-left01 2026-11-16 (merge `9cfdf0ad`) `run_tests.py`'s timings in cargo's target
    folder, `make_candidate.py`'s folder beside the main tree, each tested; the tier handle
    was no defect (Issues)
  - [x] 0.0.2-arrows 2026-11-13 Up and Down step every whole-number box by one (found in
    0.0.2-range's rework; tried in the built window 2026-11-13: the Save editor's Height
    steps, through its own `number_arrows`; the Tactics popover's X stays put after four
    presses. DECISIONS 2026-11-13; `plans/core/gui.md` "The wheel over a value", the arrow
    keys' paragraph, ST-WHEEL-07): the helper moves to `studio_core`'s widgets and the
    popover, the settings menu, the Ruleset editor and the colour picker draw their
    whole-number boxes through it -> verify: headless, one Up adds one at each site; the
    built window, the popover's X. Brief `.tmp/arrows/brief.md` (SWE-2)
    - [x] the slice 2026-11-13 (`ab7c70de`, SWE-2 and the lead's rerun: `studio_core` 476,
      `team_widgets` 261, `color_tools` 74, `ruleset_editor` 149 + 13, `save_editor`
      852 + 57, clippy, fmt): one test a site, each red first. In the built window the
      popover's X went 27 to 28 on one Up and to 26 on two Downs
    - [x] its SWE-2 review 2026-11-13 (S-arrows r1, 1 of 2 accepted, the loop ends): the
      popover's end-of-range test asserts its focus premise. Rejected: a box disabled
      while focused stepping on, since egui drops a disabled widget's focus (shown by a
      red run)
  - [x] 0.0.2-cgu 2026-11-13 (the lead): `codegen-units = 1` timed on the AATF check of a
    220-team PES 2017 save and not taken, 2.230 s against 2.280 s, inside either build's
    spread (DECISIONS 2026-11-13; `.tmp/0_0_2/cgu/`)
  - [ ] 0.0.2-text the member-facing text reread, merged 2026-11-13 (`9ed7606d`; open: the
    slices below still unticked) (the maintainer, 2026-11-13: Opus agents
    apply `~/.claude/rules/shakespeare.md`'s three readings to the readmes, the help, the UI
    text, the web pages and the comments of the TOML files a member edits; plans and specs
    later). Branch `text/shakespeare` (worktree `4cc-studio-temp/shakespeare`), merged only
    on the maintainer's word; briefs and reports in its `.tmp/shakespeare/` -> verify: each
    crate's tests and `just changelog` on the branch, then `just pipeline` after the merge
    - [x] wave 1 2026-11-13 (`08cc737e` and the two commits after it, on the branch): the
      readmes, the Save editor and core help, three web pages; the changelog cleaned of
      entries for tools no release had presented, 0.0.2 presenting the Team compiler and
      the Export upgrader in one bullet each, and the gate `just changelog`
    - [x] wave 2 2026-11-13 (`642b046d`, on the branch): the command-line help pages (the
      Team compiler's gains 45 subheadings), the two generated developer pages through
      their scripts, the bundled `settings.toml`'s comments
    - [x] wave 3 2026-11-13 (`f0b68506`, on the branch; `team_compiler` 912 + 669, `export_upgrader` 308 + 159, `pes_savefile` 419, clippy, fmt): the ruled proposals for the Team compiler's and the Export upgrader's
      message catalogs and an export's `settings.toml` comments, applied by three SWE-2
      sidekicks (briefs `brief_ap_tc.md`, `brief_ap_eu.md`, `brief_ap_toml.md`: the
      rulings are in them). The Team compiler's catalog then names its settings by key;
      the step that builds its settings form (Phase 8) gives the catalog the form's labels
      back (DECISIONS 2026-11-13, on the branch)
    - [x] done with wave 4 (`f32b3ab5`) and the merge `9ed7606d`: the maintainer's three answers of 2026-11-13, applied on the branch once wave 3's
      sidekicks have left the files (one SWE-2 brief; a DECISIONS entry with them):
      (1) where a member posts a log or a migration they cannot fix: "on the /aesco/ server
      or the divegrass thread", Red's crash handler's words (`Engines/compiler_run.bat`); the
      bundle readme says so already (uncommitted on the branch); still to do: the crash and
      unclean-close dialogs' "where you report problems" (`crates/studio/src/gui_log.rs`,
      ST-RUN-09/10 and their tests) and the Export upgrader's `migration_not_clean` row;
      (2) the gloves colours, looked at in PES 2019's Edit mode: white, black, red, blue,
      yellow, green, pink, turquoise, the spectacles' eight in the same order, so
      `gloves_color`'s comment names them (`sh_toml_a.proposals.md` P4; the wrist tape's
      stay `0 to 7`, not looked at) and the Save editor's code comment calling them
      approximations says they were looked at;
      (3) Player gloves in PES 2019's Edit mode is greyed out on a goalkeeper; an outfield
      player has "OFF" and "Worn in Winter", the colour below it: the `gloves` key's
      "outfield player gloves" is right and stays
    - [x] 2026-11-14 the wrist tape's colours are the game's four (0 strip color, 1 white, 2
      black, 3 beige; PES 2019's Edit mode): the keys' comments and range, the Save editor's
      two dropdowns, the changes list; an export's 4 to 7 is refused, a Team TOML's kept as
      stored, an old export's upgraded to a warning. The other games' lists: maintainer/questions.html
    - [x] wave 4's rulings, applied in `f32b3ab5` on the branch with their DECISIONS entry
      (the maintainer, 2026-11-13, on the twelve choices of
      `.tmp/shakespeare_choices.html`; every other proposal of the four lists is the lead's
      to rule, as wave 3's were), to apply on the branch with a DECISIONS entry:
      (1) the bar holding the tools' icons is "sidebar" in every member text (the readme,
      `settings.toml`'s comments and the install help say "left bar" today);
      (2) British spelling in member-facing prose (as the labels the game itself shows,
      such as "Numbers in defence"), with one exception: "color" is always spelt "color",
      because the files and keys members know carry it (`colors.txt`, `skin_color`). So the
      Save editor's and the Ruleset editor's "colour" becomes "color" (the `Colours` group,
      "colour code", the tooltips, the help lines that write it), and nothing else changes
      spelling;
      (3) a roster position is a "slot", counted from 1 (not "place", "Player 04" or the
      copied AATF line's 0-based "Slot 6");
      (4) the FPC action is "Hide default body (FPC)" / "Show default body (FPC)" in the
      Team menu and the Appearance section, and the settings tooltips' stale "Hide body"
      follows; the Player menu's toggle is 0.0.2-fpcmenu below;
      (5) the Comparator's columns are "Picked save" / "Open save";
      (6) the boots and gloves dialog's option reads "Set values starting from:";
      (7) "Update logic" stays; (8) "Save as…", the dialog's "Cancel", the tick's "No issues";
      (9) Fix database's "every player's" stays: the cups' database files hold cup teams
      only, so every player is a cup player (the help's "cup player" wording may follow);
      (10) Update ready says the program you are running will be moved to a folder named old;
      (11) `fpc_custom_skin_on_non_fpc` leaves the Save editor (the `Code` variant, its
      arm, the catalog row, `help_cli.rs`'s last-row pin): the editor reads an export only
      to preview portraits, so it never knows a marker;
      (12) the status bar's "Data folder is read-only" says the settings could not be
      saved and names the cause when the error gives one
    - [x] wave 4's proposals 2026-11-13: four Opus agents, proposals only
      (`sh_ui_msg`, `sh_ui_view`, `sh_ui_shell`, `sh_toml_b` `.proposals.md`)
    - [x] wave 4 applied 2026-11-13 (`f32b3ab5` on the branch; four SWE-2 sidekicks, each
      diff read whole, the six crates' tests, clippy, fmt, changelog and doc-pointers rerun
      by the lead). The lead's own in it: the terminal's start-up line reads "The settings
      file cannot be written: <error>" (it comes from a probe, so the window's "Settings
      could not be saved" would name a save that never ran); a roster slot no player holds
      reads "Slot NN" from 1 in `team_widgets`' violations list and the Multi-team AATF
      copy. A side effect to see in the built window: the Gloves color dropdown lists the
      eight color names, since `gloves_color`'s comment now names them
    - [x] 2026-11-13 the drafts brought in step with wave 4 (`docs/prototypes/`, the lead's): the four
      reports' "Drafts to follow" lists (`4cc-studio-temp/shakespeare/.tmp/shakespeare/ap_*.report.md`),
      on main, where the drafts moved meanwhile (the Team menu's FPC entries already read
      "(Enable FPC)" and "(Disable FPC)" there: keep those)
    - [x] 2026-11-13 (`7baef557` on the branch) the Export upgrader's help page on what each `unmigrated_content` cause means, and
      the stale `upgrade-models` sentence about `u0731p2.dds` beside `u0XXXp2.dds`
      (`brief_eu_reasons.md`, SWE-2, running on the branch since 2026-11-13)
    - [x] labels that grew, looked at in the built window 2026-11-16 at `1df4a139` (each read
      whole, none cut; `.tmp/lead/looks0002/`) (`ap_view.report.md` section 5:
      the Comparator's Export tooltip, the import Pick tooltip, the Team TOML tooltip, the
      boots and gloves dialog's option column)
    - [x] 2026-11-15 (SWE-2, merged `63bb0696`) the member texts left spell "color" (violations, the
      ruleset summary, the rules load error, the hair color comment, the FPC error); Red's
      `Team Colours:` note heading stays
    - [x] 2026-11-15 (lead) the two texts the maintainer picked (DECISIONS 2026-11-15 "the
      taping advice and the skills comment"): `fpc_taping_shows_pieces`'s advice in the
      Save editor and the Compiler, and the Team TOML's `skills` comment; the third, the
      Ruleset editor's findings, waits until after 0.0.2
    - [x] 2026-11-14 (`f90a92a0`) the Team TOML, patch and kit config spec blocks show what
      their writers write, each guarded by a test that reads the block; the Team TOML gains
      its first line, the instructions' note and the stats table's stored-values note; the
      kit config's comments start at one column
  - [x] 0.0.2-copywin (2026-11-13, `30b1109c`) `Copy/swap stats…` opens a non-modal window (Source and Destination by team and player; the list's selection sets the Destination; Copy and Swap leave it open); help page 06. Left: a look at the built window
  - [x] 0.0.2-fpcmenu (2026-11-13, `bba7eee1`) the Player menu's FPC entry reads `Hide default body (Enable FPC)` or `Show default body (Disable FPC)`, whichever Ctrl+I will do; the Team menu's two carry the same brackets; help pages 05 and 06
  - [x] 0.0.2-clipmsg (2026-11-13, `bba7eee1`) Ctrl+C and Ctrl+V publish a notice naming the copy and the target, a cross-version paste's with the count of values left out or changed; the clipboard text's first line holds the notice's words
  - [x] 0.0.2-notice (2026-11-13, `bba7eee1`) every result notice of the Save editor names the thing first (`f.toml written`), and the status bar draws a notice in the label family's semibold
  - [x] 0.0.2-saved (2026-11-13, `bba7eee1`) Save, Save as, Output rosters to TSV and the Comparator's Export publish their notice
  - [x] 0.0.2-tomlname (2026-11-13, `bba7eee1`) a Team TOML export suggests the team's name less its slashes, then the short name, then the ID
  - [x] 0.0.2-pitchbar (2026-11-13, `bba7eee1`) the formation bar's `Scroll` and `Portraits` checkboxes with their hovers, the `portraits` setting, the captain's mark and Live AATF's cross on the badge with portraits off; help page 08. Left: a look at the built window
  - [x] 0.0.2-pagetoggle (2026-11-13, `923607d9`) the title's row carries a `Stats | Appearance | Gameplan` toggle at its left, on the player's screens and the team view; Appearance is a screen of its own; a player segment pressed on a team row selects the team's last shown player, else its first; `[click for gameplan]` gone; help pages 02 and 04. Left: a look at the built window
  - [ ] 0.0.2-applayout (code done 2026-11-14, Opus sidekick after SWE-2 hit its output limit twice; the built window's check open, Pending looks) the Appearance screen's layout: framed groups, Strip style's grid and lines, the colour dropdowns, `Outfielder gloves`, the Presentation group last. The built screen has 17 points to spare with the lock's lines showing, where the plan's 38 is the draft's -> verify: SE-GUI-07 and 84, the built window
  - [x] 0.0.2-welcome (the maintainer, 2026-11-14: in 0.0.2 if simple) the Save editor shows a
    welcome panel when no save is open, in place of "No save is open.": the File menu's opens,
    framed in the middle of the view (`plans/save_editor.md` "Editing UI", the toolbar's
    bullet; SE-GUI-90; the draft's `welcomePanel`). Code done 2026-11-14 (`6fb65133`, SWE-2):
    the File menu and the panel draw one list of opens; the panel discovers once per stay.
    The built window, checked 2026-11-14 (`.tmp/welcome/look/notes.md`): the panel is about
    150 points wide where the draft's is 360, since `dialog_page` takes no minimum width, and
    its rows are as wide as the widest entry where the draft's fill the frame. Left: after
    0.0.2-chpop (it owns save_editor `src/view/`), `dialog_page` takes `modal`'s 360-point
    minimum and the entries fill the frame, in this panel and in the Ruleset start page
    (whose template buttons are as wide as their text). Done 2026-11-14 (`852b5f91`, the
    lead, tests red first); the built window matches both drafts. The maintainer's look
    2026-11-17: fine
  - [x] 0.0.2-chpop (the maintainer, 2026-11-14) the toolbar's unsaved changes list: a row's
    item switches to its player or team, the list staying open; the X's are the error red,
    centred under an `Undo` head; a thin line parts each row of the changes table, prompt and
    patch panel too
    (`plans/save_editor.md` "Editing UI", SE-GUI-43, SE-GUI-74; the draft's `changesTable`).
    Code done 2026-11-14 (`c8ba1be7`, SWE-2): the rows' click targets computed with the rows,
    a player no roster holds offering none. The built window matches the draft (the lead,
    2026-11-14). The maintainer's look 2026-11-17: fine, the X's to be highlighted on hover
    (0.0.2-xhover)
  - [x] 0.0.2-startdlg (the maintainer, 2026-11-14, in the built window; DECISIONS "a page that
    looks like a dialog") the Ruleset editor draws its start page as a plain full page where the
    draft shows a dialog; it becomes a page under the toolbar with its entries in the middle,
    framed and titled like the dialog, with no backdrop, so the sidebar still switches tools
    (RE-GUI-01); the draft follows. The page landed 2026-11-14 (SWE-2): `studio_core`'s
    `dialog_page` draws the modal's frame and title centred, with no modal layer; the
    toolbar stands over it, its document items disabled (`6fb65133`). The built window,
    checked 2026-11-14: the toolbar and File menu match; the draft now draws Random's buttons
    beside its label and "Recent files" under Open file…, as the editor and the plan do. The
    maintainer's look 2026-11-17: fine
  - [x] 0.0.2-rebar (2026-11-14, SWE-2) with a document open the Ruleset editor runs to the window's edges as its draft does, toolbar, rail, bottom bar and preview; the rail is the draft's 150 wide, each entry highlighted across its row with a problem's mark at the right end; its headless tests draw inside the shell's panel. Left: the built window, with the start dialog's look
  - [x] 0.0.2-sidebaredge (2026-11-22, SWE-2, merge `5e326745`) the active sidebar tool takes the selection's 1-point edge at rest, `#1e7a9e` on the dark theme, in place of the bar at its left, and every icon centres in its whole cell; the built window checked beside the draft the same day: the same edge and fill, the icon at the cell's centre on the rail and opened
  - [x] 0.0.2-sideopen (2026-11-14, `5f82ef3f`) a sidebar opened in a tall window stays inside the window when it shrinks, its entries scrolling between the logo and the Help and Settings rows; it used to keep the height it measured in the tall window
  - [x] 0.0.2-poslayout (2026-11-14, `2ef1bfe4`) a `Pitch | Left | List` toggle under Playable positions, kept as the `positions_layout` setting; the grid where the draft puts it under the toggle (63.7 under the title, List 45.9); the unchosen rating letters in the normal text colour; SE-GUI-85 headless, the maintainer's look at the built window 2026-11-17: fine
  - [x] 0.0.2-teamdrop (2026-11-13, `923607d9`) `ALL` reads `All teams` in every team dropdown; the list's dropdown ends with All, 4cc, VGL and Invitational radio buttons, which narrow it and the list, remembered as the `team_category` setting; help page 02
  - [x] 0.0.2-presentation (2026-11-14, `7b2e9ea2` merging `3667be9b`, and `b222d710`) star and playing attitude are `settings.toml` keys under `appearance.presentation`, which the patch and the upgrader carry; the Comparator counts them and every motion as Aesthetics, Copy/swap stats leaves them to the target, a 0.0.1 Team TOML still imports; the Save editor's Presentation group under the lock
  - [ ] 0.0.2-compilername (the maintainer, 2026-11-13; DECISIONS "named Compiler from 0.0.2")
    the Team compiler becomes the Compiler: the sidebar, title, help, messages and
    `quick_compile.bat`, the command-line id `compiler` and the settings table `[compiler]`,
    no alias; the crate keeps its name (`plans/core/README.md`, the Compiler bullet); then
    the docs respelled by the lead (spec, plans, drafts; DECISIONS and LOG stay; the drafts'
    text is done, 2026-11-14, their tool id and page links still `team-compiler`). After
    0.0.2-applayout and -presentation a land, since it touches their files -> verify:
    `UTW compiler compile` on the tracer exports, `git grep team-compiler` empty outside
    history, the built window's sidebar and title
    Code half merged 2026-11-14 (`31f44cda`, SWE-2, one comment fixed by the lead; workspace
    gates green but for the two overlap tests below, green alone). Docs half 2026-11-14 (SWE-2,
    84 files; the lead rejoined five links the swap had split). The sidebar and title in
    the built window checked 2026-11-16 at `1df4a139`: "The Compiler" grayed on the sidebar
    and as the help's chapter, as the draft names it
  - [x] 0.0.2-noticeerr (2026-11-14, `7ac5df69`) `NoticeKind::Error` in the theme's error red; both themes' error, warning and link colours and the success green are GitHub's pairs; the pitch's crosses take the dark red and amber in both themes; ST-BAR-04 with four kinds
  - [x] 0.0.2-nomodels (2026-11-14, `8ac5658c`) the Export upgrader's model-folder mode removed: `upgrade-models` is refused as an unknown command (EU-CLI-16), EU-MF archived in `spec/withdrawn_ids.md`, the compiler's remedies point at a Midcup export
  - [ ] 0.0.2-determ (the maintainer, 2026-11-14: "we really can't keep having tests that fail
    just because of some load on the computer") no test depends on a worker thread's timing or
    on geometry read from an earlier frame (`CONTRIBUTING.md` "Testing and verification"):
    a SWE-2 census of the tests that do, then one slice making them deterministic (a test
    controls when a background reply lands) -> verify: the census's tests pass under
    `just pipeline`'s load, two clean pipelines retire the watch entries
    Census done 2026-11-14 (SWE-2, `.tmp/determ/census.findings.md`): 1 certain race (the copy
    test), 3 clicks on a dialog a reply can move, 4 suspects, and 5-10 s hang bounds; the
    slice merged 2026-11-14 (`c4eee271`): a held-run seam (`Pending::held`), the copy test's
    check held, range picks after the dialog's check, every hang bound 120 s; mutation runs'
    test timeout at least 150 s to match (`.cargo/mutants.toml`). Open: two clean pipelines;
    the first ran 2026-11-17 on `eb72467a` (the checkpoint and the Linux gates), beside a
    sidekick's and an in-game run. But GitHub's CI on the pushed `32649d79` failed one test,
    `shell::tests::an_idle_window_wakes_for_the_moment_a_check_falls_due` ("the repaint is
    asked for the check's due moment, got 0 s", `shell/mod.rs:1977`; run 43983909058),
    which passed in both local runs. Fixed 2026-11-17: the check's worker wakes the window
    right after sending its reply, and on a loaded runner that wake landed during the
    test's idle frames, merging their delay to 0 s (reproduced by sending it from the
    test). The test now reads each request through egui's repaint observer and sends
    such a wake itself every run.
    The watch list's two, both seen failing under load on 2026-11-16, fixed: the staging
    test's lock is released by the retried step itself, and the crash monitor's connect
    bound is 10 s (the crashed run's log showed the monitor missing 2 s)
    Seen 2026-11-14 under the pipeline, a sidekick's run and two workspace test runs at once:
    `save_editor` `overlap::no_two_widgets_overlap_on_the_{player_and_team_views,menus_dialogs}`
    reached their 120 s bound ("a view worker never replied", `view/tests.rs:1511`) twice,
    in the sidekick's run and the lead's; both pass alone in 29 s. Fixed 2026-11-14
    (`6fb65133`): `open_team_view` waits for the Stats screen's card count before its click
    (red with a 3 s sleep in `spawn_badge`, green with the fix)
  - [ ] 0.0.2-similar (asked and answered by the maintainer 2026-11-13; after everything else of
    0.0.2, and only if time is left) Select all similar players: planned and drafted
    2026-11-14 (`plans/save_editor.md` "Editing UI", the bullets "Select all similar players"
    and "Several players selected"; SE-GUI-91 to 93; DECISIONS 2026-11-14; the draft's
    `dlgSimilar`, `toggleInGroup`, `lockGroupView`; captures `.tmp/similar/draft_*.png`).
    The maintainer's look 2026-11-17: agreed but for one call, so a card only some of the
    selected players hold shows the mixed mark and stays live, a click taking it from all
    of them and the next giving it to all (the draft's `lockGroupView`; DECISIONS
    2026-11-17). Next: two slices, slice 2's scenarios gaining that click (/gd/'s 10 and 13
    differ in Interception in the draft's PES 2021 sample) -> verify: headless tests, the
    built window
    - [ ] slice 1: several rows selected (the window, Ctrl+click, the filled rows, the view
      titled `N players` with its locks), split in three, since nothing of it exists (code map
      `.tmp/similar/code_map.md`: no group, no Ctrl+click, no blank locked field, no mixed
      checkbox). Until 1c lands, a group's view edits its first player alone
      - [x] 1a (2026-11-21, merge `ac0d6dbb`, SWE-2) the group: Ctrl+click, the filled rows,
        every other selecting ending it, the `N players` heading, Appearance disabled
        (SE-GUI-93 but its tooltip clause)
      - [x] 1b (2026-11-22, merge `ffa45247`, SWE-2, one rework) the button over the quick
        actions and its window (SE-GUI-91, SE-GUI-93's tooltip); Select leaves the list's
        scroll alone, as the draft does (proposals P59 asks otherwise). Owed: its look beside
        the draft, then one paid round over 1a and 1b
      - [ ] 1c the locks: what the players do not share empty and locked, the cards' mixed
        mark, no Live AATF marks or Cards used badge, the Player menu's one-player entries
        disabled (SE-GUI-92 but its edits)
    - [ ] slice 2: the edits written to every selected player as one undo step (the fields,
      the quick actions, Set and Bump all stats, Ctrl+V)
  - [x] 0.0.2-flatpitch the two formation picture dialogs (8.33, 8.34; `plans/save_editor.md`
    "Formation pictures", with the maintainer's additions of 2026-11-13; SE-GUI-86 to 89;
    DECISIONS 2026-11-14 "formation pictures are painted off screen by the view's own
    painters"; code map `.tmp/flatpitch/code_map.md` and `part2.md`) -> verify: the
    scenarios headless, the pictures looked at
    - [x] slice 1: the one-team dialog and the picture without the settings (SE-GUI-86,
      87); card `look-picture-dialog`; the maintainer's look 2026-11-17: fine
      - [x] 1a (2026-11-14, `349ca307`) the picture's painter: the pitch's painting split
        from the widget, the off-screen frame and its rasterizer, the shape rule, the file
        name; nothing calls it outside its tests yet
      - [x] 1b (2026-11-14, `b0c2f105`, Opus) the Team menu's dialog, the folder (default
        `formations`, read from the program's folder) and grain settings, the confirmation
        shared with the ruleset picker, Open folder opening the folder; help page 05. Save
        takes 273 ms on /gbpen/ (release, warm, median of five)
    - [x] slice 2 (2026-11-14, `0b71f879`, Opus) the settings box, the 240-point column and
      the Automatic settings line (SE-GUI-88); the words come from the Tactics editor's
      tables through `team_widgets::tactics::summary`, the instruction and Substitution
      wording each one shared function; help page 05. Picture looked at beside the draft
      (`.tmp/flatpitch/look2/`)
    - [x] slice 3 (2026-11-14, `3c9b7baf`, Opus) the Savefile menu's range dialog: one picture
      per team of a range, the progress line naming the picture being written, Stop keeping
      what was written, the dialog's width held through the run (SE-GUI-89); help page 07.
      Looked at beside the draft by the lead; the maintainer's look 2026-11-17: fine
    - [x] slice 3c (2026-11-14, `513b72bf`, Opus) after the maintainer's look: the range
      dialog's count ends the Teams row where Browse ends, and both results put the folder on
      a line of their own; the maintainer's look 2026-11-17: fine
  - [x] 0.0.2-dlgroom (2026-11-14, merged `887e2ae2`) every dialog's spacing as ruled
    (`plans/core/gui.md` the Modals and Form grids rows): the title 12 above the body, a
    dialog's own form's rows 27 apart and 6 above what follows (`widgets::dialog_form`),
    stacked checkboxes 21, in the drafts and the build. The built window beside fresh
    draft renders (`.tmp/dlgroom/look/compare_*.log`, `pair_*.png`): the two picture
    dialogs, Set all stats and Kit slot assignments line up row for row, every row within
    1 point but the forms' row labels at 1.3 (a label centred beside its input). The
    settings window stays off its draft (the issue of 2026-11-14 below)
  - [ ] 0.0.2-gaps stacked controls spaced by their gaps on the Stats screen and in every
    dialog: fields 6 apart, checkboxes' squares 8 (`plans/core/gui.md` the Form grids and
    Modals rows; DECISIONS 2026-11-14 "stacked controls are spaced by their gaps") -> verify:
    `draft_compare.py player`, `picture_save` and `pictures_range` in the built window, every
    row within 1 point; the tests that pin dialog rows at 27 and 21 pinning 24 and 22
    - [x] the drafts (lead, 2026-11-14): General info, Other stats and the Ability stats'
      boxes 6 apart, the Skills, Tricks and COM styles squares 8; the dialogs' `formGrid`
      fields 6 apart and their stacked checkboxes 8. Before and after in
      `.tmp/statsroom/pair8.png` and `pair_dlg.png` (before on the left)
    - [x] the build (Opus sidekick, 2026-11-14, merged `81e6166a`): `FIELD_SPACING` (6) and
      `CHECKBOX_SPACING` (4) in `studio_core::widgets`; the Stats screen, the Skills lists
      (`team_widgets::cards`) and the picture dialogs; General info's block gap gone
    - [x] the grids' drafts (lead, 2026-11-14; DECISIONS 2026-11-14 "every form takes the
      gaps"): every `table.grid` row holding a field 6 from the next, a row of checkboxes
      alone 8 between squares, text rows 19. Pairs in `.tmp/gridgaps/` (before on the left)
    - [x] the grids' build (Opus sidekick, 2026-11-15, merged `08d2ec6e`): every form grid
      but the Stats screen's and the dialogs' matched to its draft; a checkbox row among
      fields 16 tall (`checkbox_row`), its square 7 under a field and 8 under another;
      `TEXT_ROW_SPACING` (19) for the two text grids. The draft's bare grid checkboxes
      centred in their rows (`display: block`), as egui draws them
    - [x] the built window beside the drafts (lead, 2026-11-15, release at `e0a54793`,
      dark; captures in `.tmp/lead/bw/`): the two picture dialogs, Settings, Appearance,
      Copy/swap stats, Kit slots and the Ruleset editor's General and Tiers grids stand as
      their drafts (fields 24, checkbox rows 22, Physique 34; tier rows 48 against 49, as
      ruled at the review)
    - [ ] fix what that check found past 2 points:
      - the Player screen: the view's content 1 higher than the draft, each section's first
        row 0.5 nearer its heading and the Pitch/Left/List selector 1.5 nearer "Playable
        positions", adding up to 2.5 at the columns' foot (`draft_compare.py player`)
      - [x] the Ruleset editor's General: each checklist 4 lower under its help line (21
        against the draft's 17): fixed (SWE-2, 2026-11-16, merge `30cca13d`), the heading,
        help and grid stacked flush as the draft's
      - the Ruleset editor's Tiers help line over its table, and Medal systems' heading, help
        and grid, stand 3 to 4 lower than their drafts for the same cause (`item_spacing`
        between the stacked lines, which the draft has none of): moved to Phase 20 (the
        Ruleset editor waits for 0.1.0), whose built window is off its draft page-wide
        (lead, 2026-11-17, `.tmp/lead/ans2/cmp_rs_*.txt`): General's page title in the weak
        colour and 4 higher, its form 4 left of the draft's indent and 12.5 higher, the
        checklists 12 and 17 higher; Tiers' table columns 2 to 106 right of the draft's,
        its headers wrapped otherwise, its rows 19 to 32 higher
    - [x] the built window (lead, 2026-11-17, the release build of `a9ec4919`): the two
      picture dialogs within 1.3 (the forms' row labels, accepted); the range dialog's team
      pickers read `701 /3/` where the draft's read `/3/`, kept as built (the maintainer,
      DECISIONS 2026-11-17; the draft follows): the build took
      Multi-team AATF's pickers (`b213ee6a`), as the plan's "as Multi-team AATF picks its
      range" says, while the draft's are older; `draft_compare.py` pairs by text, so it
      listed the two labels as unmatched and flagged nothing. The Player screen: see the
      answers slice's recheck (the Stats screen's sections)
    - [x] 2026-11-15 (lead) each difference the check met outside these slices traced to its
      cause, and the looks asked in QUESTIONS ("The 0.0.2 Save editor report"): the Ruleset
      editor's page list, the Appearance screen's Physique and Colors boxes, Randomize, and
      the texts wave 4 reworded that the drafts never took. Three of them were never
      measured because `draft_compare.py` lists no screen of theirs
    - [x] the Tiers' Label and Id fields are drawn about 40 points wide, where `tier_row` asks
      100 and 80 as the draft does, so "Bronze" and "silver" are cut (SWE-2, 2026-11-16,
      merge `30cca13d`): a `TextEdit` in a grid cell is capped at the column's drawn width,
      so the fields, and the hex box with them, are `add_sized`
    - [x] 2026-11-15 (lead) `draft_compare.py` lists the Appearance screen (the draft's link
      takes `page=appearance`) and the Ruleset editor's General and Tiers pages (`rs_general`,
      `rs_tiers`), so a slice there is measured, not looked at
    - [x] 2026-11-15 (lead) `draft_compare.py kit_slots` opens the PES 17 sample's team 718,
      whose ten kit slots the draft shows (the build side: `EDIT17`)
  - [ ] 0.0.2-mdocs (the maintainer, 2026-11-15: "Markdown is a fairly limited format") the
    maintainer's documents as HTML pages in `docs/maintainer/`: `questions.html` and
    `proposals.html` replace the two Markdown files, `tasks.html` holds what waits on the
    maintainer's hands; shared `maintainer.css` and `maintainer.js` (picks kept in the browser,
    "Copy my answers" for the chat); captures in `img/`, ignored by git (the maintainer,
    2026-11-15); `just doc-pointers` reads the pages
    - [x] 2026-11-15 the three pages: every entry of the two files, and the maintainer's lines
      of "Pending looks" as cards; every pointer moved, the two files removed
    - [ ] the lead checks each look of `tasks.html`'s "Coming once the lead has checked them"
      beside its draft (`scripts/built_window/`), each then a card with its capture pair
    - [x] a Linux bundle built from main for the Reset window card 2026-11-17: 0.0.2-rc.3 of
      `333ec0cb` (branch `linux-rc3` at `7febf590`, never merged), built on the VPS, in
      `4cc-studio-temp/tasks_build/`, named on the card (SHA-256 `519d93f0…`)
  - [x] 0.0.2-tplclean 2026-11-15 (merge `c20e6b29`): a template or Blank opens in the Ruleset
    editor with no unsaved changes (RE-GUI-02, 15)
  - [x] 0.0.2-teamscroll 2026-11-15 (merge `c20e6b29`): every dropdown's list 300 points tall
    (`theme.rs` `combo_height`); the teams dropdown keeps the whole 300 with the radios under
    it, in a popup the selector opens itself (an egui ComboBox's popup stops at 400)
    - [x] the built window beside the draft: checked 2026-11-15 at `c20e6b29`, card
      `look-team-dropdown`; the maintainer's look 2026-11-17: fine
  - [x] 0.0.2-picsettings 2026-11-15 (merge `b13767aa`): the picture dialogs' settings box is
    the `formation_pictures_settings` setting, shared by both; a picture with the settings
    keeps its plain name
  - [ ] 0.0.2-report (the maintainer, 2026-11-15, who expected it in 0.0.2) the "Report a
    problem" button and dialog: the repro bundles issue's slice (4) (`plans/core/architecture.md`
    "Repro bundles"; the draft's `drawReport`), writing the Save editor's bundle from the journal
    slice (2) built; replay's comparison of the tool's state waits with slices (3b) and (3c),
    bundles read with `save-editor diff` until then (DECISIONS 2026-11-08); the dialog's
    rulings and `jiff` (DECISIONS 2026-11-15 "Report a problem: the dialog's rulings")
    -> verify: ST-REP-01 to 04 (`plans/core/gui.md`), the built window beside the draft
    - [x] built (2026-11-15, SWE-2 and the lead; merge `ced67b83`): ST-REP-01 to 04 pass, a
      help chapter of its own ("Reporting a problem") and the changelog line
    - [ ] (after 0.0.2: no headless shell other crates can drive exists yet; for 0.0.2 the
      chain is run by hand in the built window below) the whole chain once, as a member runs
      it: the tests cover it in pieces (the
      dialog writing a stub tool's bundle, ST-REP-02; the Save editor's state, ST-REP-04; a
      recorded `Session`'s bundle replaying to a matching write, `replay.rs`), never through
      the shell with the real Save editor -> verify: a test with the Save editor registered in
      the shell opens a fixture save, edits a player, saves, clicks Report a problem and
      Save…, then reads the zip back (`repro::read_bundle`): replay's one `Matches` on the
      write, `state/` the edited save, the input and output kept; red with the tool's
      journal not handed to the dialog (SWE-2, after barrw3)
    - [x] a blind drill (the maintainer, 2026-11-15): on a throwaway branch the lead plants a
      behaviour bug that shows no error, the kind found in game (an edit that skips the
      edited-player flag PES 2019 needs for a name to show, as the aesthetics patch once
      did), records a bundle through the dialog, and hands a fresh agent, not told the
      bug, only the bundle and a member's words ("I renamed him, the game shows the old
      name"): SWE-2 first, and an Opus agent only if SWE-2 does not find it (the
      maintainer, 2026-11-15). Measured: whether the journal (what the member did) and `save-editor diff`
      of the kept input against the written save lead it to the cause, and what replay
      adds: on the planted build it reproduces the write (`Matches`), and on the fixed
      build it reports `Differs` naming the field, so it confirms a fix on the member's own
      data but cannot flag a bug the replaying build shares. What the agent missed becomes
      the bundle's next addition (after the chain test)
      - [x] run 2026-11-17: the Name field's edit stripped of its flag line, a bundle
        recorded in the built window on a copy of `EDIT00000000_VGL27_Friendlies` (PES 2019,
        player 70101 renamed), SWE-2 given only the bundle, the member's words and a copy of
        the source without history (`.tmp/lead/drill/`; its findings in
        `4cc-studio-temp/drill_case/work/findings.md`). Found in 12 minutes: the file and line,
        the one-line fix, and the existing test that fails on it. The journal and `diff`
        showed a name written and nothing else; the flag's state it read by porting the
        container's decryption to Python, since nothing in the bundle shows a stored field
        that did not change (`export-toml` would have, and it did not think of it). Replay
        was not measured: no command runs it. The bundle's next additions (DECISIONS
        2026-11-17 "a Report a problem bundle carries a readme"): its `README.txt` now,
        the edited teams' Team TOMLs before and after with the chain test
    - [x] the bundle's `README.txt` (`plans/core/architecture.md` "Repro bundles"): what
      each part holds, then each tool's `repro_readme`, the Save editor's naming `diff` and
      `export-toml` (SWE-2) -> verify: a written bundle's readme holds the core part and
      the Save editor's, red without the method's override; a SWE-2 review round
    - [x] the built window beside the draft (`.tmp/lead/looks/report/draft_open.png`), run
      through as above on a copy of a save, the zip then read with `save-editor diff`; then
      card ST-REP-05 in `maintainer/tasks.html`; review rounds (tier 2)
      - [x] run through (lead, 2026-11-17, the release build of `97cc9757`, PES 2021's
        sandbox save): a skill ticked, the save written, Report a problem, Save…. The zip
        holds the journal's four steps, both inputs, the output, `state/` and the log;
        `save-editor diff` of the kept input against the output names the one edit
        (player 70101, Heading 0 → 1), and `state/` reads the same as the output
      - [x] two differences from the draft, over 2 points each: the intro's lines and the
        list's rows 18 apart (the draft's 15), and the text box 61 tall (the draft's 84,
        right under its label). Fixed (SWE-2, merge `32649d79`), looked at in the release
        build of `32649d79`: rows 15 apart, the box 84 right under its label
      - [x] the lower half 8 higher than the draft's from the title (the draft's 10-point
        sentence gaps against the dialogs' 8, the room a browser keeps under an inline box):
        the draft follows the build (the maintainer, DECISIONS 2026-11-17); every row now
        stands where the build's does
      - [x] card ST-REP-05 (2026-11-17): the maintainer's run from a save to the zip works
  - [x] 0.0.2-namelight 2026-11-15 (SWE-2, merge `40ce82da`): in the light theme a name's
    colour code reading under 4.5 on the panel is darkened until it does, hue kept
    (`theme::darkened_to_read`, which reproduces the lineup lists' hand-made colours); the
    pitch's turf and the dark theme draw codes as they are. The changes table's heads are
    strong and the paste notice's tails end with a full stop. Look checked in the built
    window; the maintainer keeps the game's colours instead (2026-11-17): 0.0.2-namegame
  - [ ] 0.0.2-namegame the light theme draws a name's colour codes as they are, the
    darkening above taken out (the maintainer, 2026-11-17; `plans/save_editor.md` "The
    player list"; DECISIONS 2026-11-17); `theme::darkened_to_read` stays for the palette's
    colours. SWE-2, after 0.0.2-frames B1 and A2a merge (it touches `team_widgets` and
    `save_editor`'s name callers) -> verify: the light theme's /f/ list and lineup lists
    - [x] landed 2026-11-17 (SWE-2, merge `29991c8b`, with 0.0.2-xhover; the lead's rerun:
      save_editor 1047, studio_core 538, team_widgets 282): `append_name` draws every code as
      it is, `name_background` gone; the darkening's changelog line removed (never released)
    - [x] the built window (lead, 2026-11-17, the release build of `17bda172`): on the light theme every code drawn
      as stored, so a white or pale part nearly vanishes on the white list; card
      `look-light-names`; the maintainer's look 2026-11-17: fine as stored
  - [x] 0.0.2-present 2026-11-17 (SWE-2, merge `0cbe7637`; the lead's rerun of `studio_core`, 535)
    the window presents through a
    DirectComposition swap chain, so a resize no longer stretches the last frame and the UI
    stops jumping (`plans/core/gui.md` "The graphics backend"; DECISIONS 2026-11-17). SWE-2,
    `studio_core` `shell/window_size/mod.rs` `wgpu_options` -> verify: a unit test of the
    pick and its variable, `.tmp/lead/flicker/variants2.py` on the built copy
    - [x] `variants2.py` on the release build of `17bda172` (lead, 2026-11-17): 121 grabs per variant, no
      feature off its trend in either (the default, now DirectComposition, and the
      variable's)
  - [ ] 0.0.2-xhover the X ending each row of the unsaved changes list and of the close
    prompt highlighted on hover, as the draft's frameless buttons are (the maintainer,
    2026-11-17; `plans/save_editor.md` "An X on each row"). SWE-2, beside
    0.0.2-namegame -> verify: a headless hover test, the built window
    - [x] landed 2026-11-17 (merge `29991c8b`): `frame_when_inactive(false)`;
      `a_rows_x_lights_on_hover`; the changelog's × line says it lights up
    - [x] the built window (lead, 2026-11-17, the release build of `17bda172`): framed under the pointer as the
      draft's; card `look-x-hover`; the maintainer's look 2026-11-17: fine
  - [x] 0.0.2-allcat under 4cc, VGL or Invitational the team selector's All teams entry reads
    `4cc teams`, `VGL teams`, `Invitational teams`, open and closed (the maintainer, 2026-11-17;
    `plans/save_editor.md` the team selector's category paragraph, SE-GUI-83; DECISIONS
    2026-11-17; draft `teamSelector`). SWE-2 on `.tmp/brief_allcat.md`, after 0.0.2-frames B2
    and 0.0.2-namegame merge (all three touch `view/player_list.rs`) -> verify: the category
    test turned around, the built window under VGL
    - [x] landed 2026-11-17 (SWE-2, merge `df9833a8`, with 0.0.2-ixtext; the lead's rerun:
      save_editor 1048): `format!("{} teams", shown.label())`; help page 02 says so
    - [x] the built window under VGL (lead, 2026-11-17, the release build of `17bda172`): `VGL teams` open and
      closed, as the draft's; card `look-category-teams`; the maintainer's look 2026-11-17: fine
  - [x] 0.0.2-ixtext the Team file interchange window's sentences and labels in the normal
    text colour, the Skills labels', not the weak grey (the maintainer, 2026-11-17;
    `plans/save_editor.md` "Interchange formats in the UI"; DECISIONS 2026-11-17; draft
    `interchangePanel`). In the same SWE-2 run as 0.0.2-allcat (`interchange.rs` is one of
    0.0.2-namegame's name callers) -> verify: a painted-colour test, the built window beside
    the draft
    - [x] landed 2026-11-17 (merge `df9833a8`): `the_panels_sentences_paint_the_normal_text_colour`;
      changelog under the new Other stuff heading
    - [x] the built window beside the draft (lead, 2026-11-17, the release build of `17bda172`): the text colours
      match; the import row is the draft's fake file list against the build's file
      dialog, by design; card `look-interchange-text`; the maintainer's look 2026-11-17: fine
  - [ ] 0.0.2-gatelbl (P53, accepted for 0.0.2 by the maintainer, 2026-11-17) every
    version-gated control's label dimmed with it and carrying its `PES NN+` tooltip, as the
    Ability stats' are (`plans/save_editor.md` the Stats screen's paragraph on field labels;
    DECISIONS 2026-11-17). First a SWE-2 map of every such control and its label, build and
    draft; then the draft; then SWE-2 on a brief, after 0.0.2-frames B2 merges (General
    info and the team view) -> verify: a test per place reads the label's painted colour,
    red first; the built window beside the draft on a PES 2016 and a PES 2019 save
    - [x] landed 2026-11-17 (SWE-2, `a0d7afa7`, merge `1e317853`): `abilities::gated_label`
      for Stronger hand, Star rating, Playing attitude, Dribbling motion and a PES 2015 or
      2016 team's Color 1, Color 2 and Manager ID; the changelog's ability-stat line names
      them all; help page 03's Greyed controls. The lead's rerun: save_editor 1055 + 57,
      clippy, fmt
    - [x] SWE-2 round 1 2026-11-17 (`.tmp/0_0_2/frames_rev/swe_gatelbl.findings.md`): 0
      accepted (two unreachable arms, a nit), the loop ended; the paid round's Stats
      part follows
    - [x] the built window beside the draft (lead, 2026-11-17, the release build of `17bda172`): PES 2017's
      Stats and Appearance and PES 2016's team row dimmed as the draft's (the drafts
      hold no PES 2016 save); the build's gated boxes read `-`, the draft's `0`, a
      difference older than this step, asked on the card `look-gated-labels`; the
      maintainer's look 2026-11-17: fine, the build's `-` kept
  - [x] 0.0.2-undosel 2026-11-15 (SWE-2, merge `e0ebd784`): Undo, Redo and a `Player order`
    row's X keep the moved player selected (`view/mod.rs` `following_selection`,
    `Session::record_of`). Look checked, card `look-undo-selection`; the maintainer's look 2026-11-17: fine
  - [x] 0.0.2-leeway 2026-11-15 (SWE-2, merge `addf0000`): `SHAPE_TOLERANCE` 3; /kfg/'s
    preset 1 draws Kick-off alone. Look checked, card `look-kfg-leeway`; the maintainer's look 2026-11-17: fine
  - [x] 0.0.2-idfilter 2026-11-15 (SWE-2, merge `addf0000`): IDs match from their start, a
    team's ID listing its squad (SE-GUI-102)
  - [x] 0.0.2-pgtop 2026-11-15 (SWE-2, merge `addf0000`): a team row the page keys select
    scrolls to the list's top (SE-GUI-103). Look checked, card `look-pgdn-team-top`; the maintainer's look 2026-11-17: fine
  - [x] 0.0.2-ownteam 2026-11-15 (SWE-2, merge `addf0000`): the import window's
    `Select <name> (<id>)` (SE-GUI-104). Look checked beside the draft, card
    `look-own-team-button`; the maintainer's look 2026-11-17: fine (the button on its own row under the line, as built)
  - [x] 0.0.2-changesperf 2026-11-15 (SWE-2, merge `a6bba169`): the changes table lays out
    only the rows in view, each cell measured once beside its rows (`prompt::TableLayout`);
    a release frame of the list with four teams' 2,494 rows 17.45 ms -> 1.01 ms, the close
    prompt 8.73 -> 0.97. Scrolled in the built window with two imports; card
    `test-changes-scroll`: the maintainer's try 2026-11-17 scrolls well
    - [x] no row pops in during a fast scroll (2026-11-16, SWE-2, merge `8db25bd4`):
      `a_fast_scroll_keeps_the_box_covered`, red with the drawn range one row short at
      either edge and with the wheel off the list
  - [ ] 0.0.2-frames (a member's request; DECISIONS 2026-11-17 "the Stats screen's panels are
    framed" and "every live box, dropdown, toggle and button draws a 1-point edge") -> verify:
    the built window beside the draft on the Stats, Appearance and Gameplan screens; a tasks
    card; review rounds (a SWE-2 loop and one paid round)
    - [ ] the review rounds: SWE-2 round 1 on A1, A2a, B1 and B2 (2026-11-17, from a worktree
      at `74d85558`, `.tmp/0_0_2/frames_rev/`; the rulings file's "frames r1"):
      2, 2, 2 and 4 accepted, every loop ended; the fixes landed 2026-11-17 (SWE-2 rw1, merge
      `de984144`; the lead's rerun of the five crates, clippy, fmt)
      - [x] C's SWE-2 round 2026-11-17: 2 accepted (one already fixed), the loop ended; the
        slider's value box drawn as a number box on the light theme is rw2 (SWE-2)
      - [x] rw2 2026-11-17 (SWE-2, `9d3195c1`, merge `17bda172`): a slider's value box drawn
        as a number box on the light theme, its rail and handle unchanged; the widgets'
        tests share `test_support`. The lead's rerun: studio_core 545, team_widgets 282,
        save_editor 1055 + 57, clippy, fmt, the colour gate
      - [ ] one paid round (Carbon) at `17bda172`, two parts by concern: the edges and the
        light theme (A1, A2a, C, rw1, rw2), then the Stats screen (B1, B2, rw1, gatelbl)
        after gatelbl's SWE-2 round
        - [x] the Stats part, 2026-11-17: 8 of 11 accepted ("frames C1 — Stats"), 3 of reach
          (a): a panel title's Live AATF mark fell on its frame line, or past the pane's edge
          in column 1 (the draft's too: the lead moved it right of the title, DECISIONS
          2026-11-17, `ce0ae2b2`, with the plan's stale lines, help 03 and the changelog
          line); a PES 2015/2016 team colour's hex left undimmed; a third pass on every new
          player. Tier 2: a second paid round on the Stats part after the rework
        - [x] rw3 2026-11-17 (SWE-2, `fd1d7eca`, merge `d1594018`; the lead's rerun 1057 +
          57, clippy, fmt, changelog): a panel title's mark right of the title (on Skills after
          the badge), the gated hex dimmed, two passes for a new player (the widths stored
          clamped), `total_margin`, the grid 1.5 higher; the layout tests' figures set to the
          draft's as the build measures it (Pitch 88, List 46: the lead, by the draft)
        - [x] its SWE-2 round 2026-11-17: 4 accepted, none of reach (a), the loop ends after
          their fixes: the Tier badge untested, two comments, the Skills mark a row's gap (8)
          after the badge as the draft's (plan edited, `e3fc5b93`)
        - [x] rw3 r1 2026-11-17 (SWE-2, `e109b2aa`, merge `9091411a`; the lead's rerun 1060 +
          57, team_widgets 283, clippy, fmt): the Skills mark the row's gap after the badge,
          the Tier badge tested by the chip's painted edge, three comments
        - [x] Carbon's round 2 (Stats) 2026-11-17 at `9091411a`: 4 accepted, none of reach (a),
          the loop ends after their fixes (help 02, the plan's marks paragraph and the
          changelog line by the lead, `01f8b455`; the rest in rw5)
        - [x] the edges part, 2026-11-17: 7 of 10 accepted ("frames C1 — edges"), 1 of reach
          (a): on the light theme a version-gated number's `-` stand-in draws the button grey
          beside white number boxes; the rest a readout grey whose contrast drifted with the
          240 panel, two docstrings, duplicated light-look code and four untested looks. The
          changelog's edges line reworded (the lead). Tier 2: a second paid round on the edges
        - [x] rw4 2026-11-17 (SWE-2, `bd552eb2`, merge `6de96354`; the lead's rerun of
          studio_core 548, team_widgets 283, color_tools 78, boglo, clippy, fmt, the colour
          gate): `widgets::number_box_look` shared by the number box and the slider, the
          readout `0x5b`, the docstrings, the tests
        - [x] its SWE-2 round 2026-11-17: three nits fixed (`e3fc5b93`), the loop ended
        - [x] the gated stand-in 2026-11-17 (SWE-2, `b4efbe98`, merge `1b40ddc8`; the lead's
          rerun 1060 + 57, clippy, fmt; the two tests' setup shared): on the light theme a
          version-gated number's `-` draws as a disabled number box
        - [x] Carbon's round 2 (edges) 2026-11-17 at `e3fc5b93`: 4 accepted, none of reach (a),
          the loop ends after their fixes (the plan's readout grey and two docstrings by the
          lead, `4526244e`; the rest in rw5)
        - [x] rw5 2026-11-17 (SWE-2, `f2e0c127`, merge `11be9f8f`; the lead's rerun 1059 +
          57, studio_core 548, clippy, fmt): both parts' round-2 code fixes (the disabled and
          hovered boxes' tests, one `gated_number`, one pass-count test, three comments); the
          frames' review loops end here
        - [x] the pipeline on `11be9f8f` 2026-11-17: the checkpoint and the Linux gates passed
        - [x] the pipeline on `6de96354` 2026-11-17: the checkpoint and the Linux gates passed
    - [ ] the control edges (`plans/core/gui.md` "Other UI components", "Control edges")
      - [x] A1 2026-11-17 `studio_core` (`b98986f7`, merge `53bc3805`): the resting stroke,
        `theme::edge_color`, the checked box's and radio's edge, none on disabled widgets or
        rows (the combo's, the log chips', the help tree's, the sidebar's)
      - [x] A2a 2026-11-17 (SWE-2, one rework; merge on `e09fe74d`; the lead's rerun of the
        five crates) `widgets::row` and `widgets::resting_edge`, `theme::unheld_edge`: no edge on
        the rows outside `studio_core` (the comparator's chips, the player list, the ruleset
        rail, Show preview) and the picker's swatches; the toggle edges of `team_widgets`'
        selectors and Tactics toggles; the armed role chip (SWE-2, `sk/edges2`)
      - [x] C 2026-11-17 (SWE-2; merge `9abed142`; the lead's rerun of the five crates): the
        light panel 240, text boxes 253, `widgets::number_box` at every `DragValue` (no clippy
        rule: it would flag the call inside the wrap at every site); the derived fills, amber,
        green and lineup colours as DECISIONS lists them, and the quiet greys (count, C
        rating) at 153, keeping the dark 2.5 as the plan's rule says (the lead's, at review)
        - [ ] the built window on the light theme: captured (lead, 2026-11-17, the release build of `17bda172`;
          the drafts have no light theme), the card after the Stats rework
    - [ ] the Stats screen's frames (`plans/save_editor.md` "Editing UI", the player view)
      - [x] B1 2026-11-17 (SWE-2, one rework; merge `a663b4e9`; the lead's rerun 1044 + 57,
        clippy, fmt): the panels framed, spanning the pane as Appearance's frames, 8 apart,
        tops on Appearance's, columns ending on one line, the positions grid at List's height;
        the shared frame helpers in `view/group_frame.rs`. Column 1 is the tallest in every
        bundled ruleset on PES 2015, 2017 and 2021 (measured), so the grid never stretches
      - [x] B2 2026-11-17 (SWE-2; merge `74d85558`; the lead's rerun on the merge): General
        info's fields end 12 inside the frame at any width, every label 20 before its field,
        the halves at least 18 apart; the labels `Shirt name`, `Short name`, `Color 1`/`Color 2`
        (DECISIONS 2026-11-17, a changelog line under Other stuff; Playing attitude's note
        needs none, its screen is new in 0.0.2); the Save editor's toggle and rating edges
        through `resting_edge`, the lit Live AATF button's in the selection blue's
        - [ ] the built window beside the draft, with the batch release build
  - [ ] 0.0.2-paneshift (the maintainer's request, 2026-11-17): every screen right of the
    player list 8 points left, 4 after the list's bar (as the bar stands 4 after the list's
    frame; 14 after the frame with no bar) and 16 from the window's right edge, where 12/22
    against 8 made the left look much wider -> verify: the built window beside the draft on
    every screen
    - [x] the draft 2026-11-17 (lead): `.master` gap 8, the pane's right padding 12, the
      Squad header's gap 8 so its tabs stay put; measured: every screen's frames 4 left,
      their widths kept. Then (the maintainer: Firefox's bar made the gap 29) the pane draws
      the list's 6-point bar (`vScrollBar`, shared) in its right margin, and the harness
      drops the window's scroll-bar strip for a view drawing its own bars (`own-bars`,
      `#win.nostrip`); the Squad screen keeps the strip. Measured in Chromium and Firefox:
      window 1280, gap 12, the bar 4 after the frames. Then (the maintainer: 4 more, so the
      list's bar has 4 on each side) `.master` gap 4, the pane's padding 16, Squad's gap 4,
      the pane's bar 4 after the frames; measured: frames 323 to 1264, tabs at 323 on every
      screen. Then (the maintainer) the frames' tops on the list's frame: the screens and the
      line above them 1.5 lower, the tabs row 0.75 (viewTitle's margins, Squad's alike);
      measured: frames and list at 71, the line at 58, the tabs at 34.75 on every screen
    - [x] the frames' tops on the list's (71), the line at 59, the tabs at 34.75 (the
      maintainer's, 2026-11-17; drafts to `a083b48c`); the draft approved by the maintainer
      2026-11-17
    - [x] the plan (`save_editor.md` "Editing UI" and "The player view") and DECISIONS
    - [x] the build 2026-11-17, with 0.0.2-keepsel's and 0.0.2-posheight's (SWE-2, one
      rework; `aeed1e34`, merge `541f88a6`; the lead's rerun 1063 + 57, clippy, fmt): its
      contradiction found the draft's Gameplan 1 past the pane (the lead: 8 under the
      fields, `58cb2b94`); r1 set the 8 and measured every Tactics block on the draft's
      within 0.5 (the first run's "2 taller" was its own arithmetic). For the look: under
      DBC7 the build's Pitch ends at 700 and List at 725, the draft's 703 and 726 (Ability
      stats' and the positions' own heights, before this step)
    - [x] the pipeline on `541f88a6` 2026-11-17: passed, with the Linux gates
    - [x] the changelog 2026-11-17: keepsel's line under Other stuff; the margins and the
      positions height refine lines already there (the maintainer's own pass runs on branch
      `docs/changelog`, worktree `4cc-studio-temp/changelog`)
  - [ ] 0.0.2-keepsel (the maintainer's ruling, 2026-11-17): picking in the team dropdown
    the team the selection is already on keeps the selection (a player stays selected)
    -> verify: a test per case (his team keeps him, another team selects its row)
    - [x] the draft (`teamSelector`, `a083b48c`), approved by the maintainer
    - [x] the plan (`save_editor.md`, "Editing UI", picking a team) and DECISIONS
    - [x] the build 2026-11-17, with 0.0.2-paneshift's (`541f88a6`)
  - [ ] 0.0.2-geninfo (the maintainer's request, 2026-11-17): General info (Name, Shirt
    name, Shirt number | Age, Nation as an `ID - NAME` dropdown) and Main stats in place of
    Other stats (Captain in its title's row, Height | Weight, Injury Resistance | Form, Weak
    Foot: Usage | Accuracy, the sides, Registered position, Playing style), the panels'
    widths unchanged -> verify: the built window beside the draft; the nation lists' test
    - [x] the draft, evaluated on `eval/geninfo` and approved for 0.0.2 by the maintainer;
      merged; the nations then taken from the games' own `Country.bin` per version (lead,
      `.tmp/lead/nations/`: `unwesys.py`, `table.py`, the seven `pes<NN>.tsv`)
    - [x] the plan ("The player view") and DECISIONS
    - [x] the nation lists (lead): `scripts/derive_nations.py G:/Games` writes
      `crates/tools/save_editor/data/nations.tsv` (220 IDs; 189, 191, 220, then 214 a
      version)
    - [x] the build (SWE-2 on `sk/geninfo`, worktree `4cc-studio-temp/geninfo2`, from
      `73641390`; brief `.tmp/brief_geninfo.md`): the two panels as the draft's
      `generalPanel`, the nation combo from `nations.tsv` (loaded once, not a frame's work;
      the open list scrolled to the held nation; `ID - ?` for an ID the version lacks), a
      test holding the fixture saves' nation IDs to their lists, help 03; then the
      changelog line (under Useful). Built 2026-11-17 (SWE-2, 73 min; `388e5fdf`, merge
      `5023e59a`; the lead's rerun 1070 + 57, pes_savefile 433, clippy, fmt; its pipeline
      passed, with the Linux gates): the fixture saves carry one ID
      their version lacks (player 75114, nation 23, on PES 2015 and 2016), so the test
      pins those two, the case `ID - ?` exists for
    - [x] slice 2 2026-11-17 (SWE-2, one rework: the entries' texts cached at load, the
      maintainer's; `1804f30e`; the lead's rerun 1073 + 57, clippy, fmt): `view/terms.rs` words a nation change as the
      combo does (`135 - USA`; today numbers, against its own rule "its old and new values
      as that view shows them": the changes list, the close prompt, the Comparator), and
      typed digits pick a nation while the list is open (plan and DECISIONS 2026-11-17);
      its pipeline passed, with the Linux gates
    - [ ] the review: a SWE-2 loop over both slices, then one paid round (Astra's). Round 1
      2026-11-17 at `1804f30e`, five parts (`.tmp/0_0_2/geninfo_rev/`): 15 accepted, among
      them a typed `2` unreachable while an unlisted nation is held, two rows selected on a
      typed pick's frame, test gaps; the lead's share done (the draft's Squad table grouped
      under Main stats, SE-GUI-95, `gui.md`); the rest by SWE-2 (19 min, landed first time;
      `527cfcf2`, merge `d2eadc87`; the lead's rerun 1077 + 57, wheel 19, clippy, fmt).
      Round 2 over the fixes (`.tmp/0_0_2/geninfo_rev2/`): none accepted, which ends the
      loop; the pipeline on `d2eadc87` passed, with the Linux gates. Astra's round
      2026-11-20 over `73641390..be99921c` (rulings "geninfo A1"): 4 accepted, which ends
      its loop. The plan's two (SE-GUI-106, the nationality's 16 bits) are done; the code's
      three go to the Nation list step (status list): Enter after Down and typed digits
      picks the focused row, not the typed one; no test clicks a non-captain's Captain
      radio; one nation lookup a draw, not two. Their SWE-2 loop runs with that step's
      review
    - [x] the 4cc patch's own nation table (different on PES 2015 to 2017) is not used: the
      base game's lists stand (the maintainer, 2026-11-17; plan and DECISIONS)
  - [x] 0.0.2-lightfill 2026-11-17 (the maintainer's change; the lead): the light theme's
    control fills 8 lighter, 208, 194 and 215 (plan `core/gui.md`, DECISIONS 2026-11-17);
    gates studio_core 552, save_editor 1077 + 57, ruleset_editor 174 + 13, clippy, fmt; the
    release build reads 208 on a button, a dropdown and a checkbox; the pipeline on
    `daa6c926` passed, with the Linux gates
  - [ ] 0.0.2-posheight (the maintainer's change, 2026-11-17): the playable positions grid
    takes its own layout's height, so Pitch and Left leave the panel shorter than List
    (under DBC7 the screen ends on Ability stats' frame, 703 against List's 726 in the
    draft) -> verify: SE-GUI-85's DBC7 clause; the built window beside the draft
    - [x] the draft (`#pos-grid` loses its `min-height`), the plan and DECISIONS
    - [x] the build 2026-11-17, with 0.0.2-paneshift's (`541f88a6`; `GRID_HEIGHT` gone)
  - [x] 0.0.2-taskbuild 2026-11-17: the tasks page names the build its cards run on -> verify: the next
    release build (the Report a problem look's) copied to `4cc-studio-temp/tasks_build/`,
    the page's intro naming that path and the commit it was built from; refreshed whenever
    a card is added from a later commit (RUNBOOK "Looks owed have one home each")
  - [ ] 0.0.2-lookfix (the lead's look on the release build of `1804f30e`, 2026-11-17): the
    player screen beside its draft, dark: every frame edge equal under the default ruleset,
    General info and Main stats within 1, Gameplan's frames equal, DBC7's Pitch ending on
    Ability stats' frame above List's; light looked at (no draft). Two mismatches, both the
    build's -> verify: the built window beside the draft again
    - the status bar's left and middle slots stand at the bar's top, where the draft centres
      every slot (`v0.0.1 pre-alpha` 2.9 high on every screen)
    - Ability stats drops the draft's 3 of cell padding under its last row, so under DBC7 the
      columns end at 700, the draft's 703
    - [x] the fix 2026-11-17 (SWE-2, 22 min, landed first time; `254262b2`, merge `333ec0cb`;
      the lead's rerun: status bar 20, save_editor 1074 + 57, clippy, fmt): the left and
      middle slots `horizontal_centered`; Ability stats' frame pads 3 under its last row
    - [x] the built window beside the draft again 2026-11-17 (release build of `be99921c`): the
      version label centred in its bar, DBC7's Pitch ending at the draft's 703; the card
      `look-stats-screen` on the tasks page, the build in `tasks_build/`
    - Not a mismatch: the quick actions stand 23 lower than the draft's, which also draws
      0.0.2-similar's button, built after 0.0.2. Not checked in the window: typed digits in
      the Nation list (the capture tool's posted keys type no text; the headless tests hold
      them)
  - [x] 0.0.2-shirtno the Team TOML player key `number` is `shirt_number` (`fb42b800`, merge
    `5e4cb83a`): an error naming `players.03.number` read as the player's ID; a 0.0.1 file
    holding `number` is refused as any unknown key, and the changelog's Important line says so
  - [x] 0.0.2-pgframe a held PgDn flickered the selected row, the list's scroll landing a frame
    after the selection (`35902abc`, merge `f4c1d2db`): the list gives its scroll area the
    reveal's offset before layout (egui lays the area out from its stored offset first);
    SE-GUI-103's same-frame clauses tested on the key's own frame. Built list checked at
    `9d536eb4` (the draft has no page keys), card `test-held-pgdn`: the maintainer's try 2026-11-17 steady
  - [x] 0.0.2-promptx the close prompt's table gets the toolbar list's X column, its last
    row's X letting the close or open go on (`55f398a9`, merge `29299fe3`)
  - [x] 0.0.2-popwidth the toolbar's changes list is as wide as its widest row up to the
    window less 8 points a side, growing left of the label (`55f398a9`, merge `29299fe3`,
    SE-GUI-43's width clauses tested): tried by the maintainer in the built window,
    2026-11-15, a long renamed player's row growing the list left past its label
  - [ ] 0.0.2-revdebt the reviews owed by behavior changes no reviewer read (DECISIONS
    2026-11-15 "A change to behavior is reviewed at its tier, however small"; audit of the
    closed 0.0.2 items against `.tmp/duck_rulings.md`). One review per surface, each SWE-2's
    loop then one Carbon round (tier 2), or the three loops (tier 1), in this order:
    - [x] list and keys (tier 2): `d700b3e5`, `35902abc`, `e0ebd784`, `96aef711`, `457f30c8`.
      SWE-2's loop (2 of 4 accepted) and Carbon's seven rounds (9, 13, 6, 7, 5, 5 and 4
      accepted, "C-list r1" to "r7"), each reworked: a selection moved, kept or picked
      under a filter keeps its row on screen; the loop ended with round 7's rework (merge
      `4fff9ae9`)
    - [x] notices and files (tier 2): `bba7eee1`, `7ac5df69`, `30b1109c`. SWE-2's loop (12 then 1
      accepted) and Carbon's three rounds (15, 7 and 6 accepted, "C-notices r1" to "r3"), each
      reworked; the loop ended with round 3's rework (merge `93f945ee`)
    - [ ] screens and layout (tier 2): `923607d9` (pagetoggle, teamdrop), `2ef1bfe4`
      (poslayout), `5f82ef3f` (sideopen), `887e2ae2` (dlgroom), `40ce82da` (namelight),
      `b13767aa` (picsettings), `be3913bf` (picwidth), `c20e6b29` (tplclean, teamscroll)
      - [x] SWE-2's loop over parts A to C ("S-scr", "S-scrc"); part C's last fixes (a team
        row's name codes, the range run's settings box) merged (`08b518ba`)
      - [ ] one Carbon round, after 0.0.2
    - [ ] the changes list and the close prompt (tier 2): `a6bba169` (changesperf),
      `55f398a9` (promptx)
    - [ ] interchange and export settings (tier 1): `fb42b800` (shirtno), `7b2e9ea2`,
      `3667be9b`, `b222d710` (presentation)
    - [ ] the Export upgrader (tier 1): `8ac5658c` (nomodels), `6fed2e3e` (cli)
    - [ ] the Export upgrader's poc fixes (tier 1): `2b028e34`'s `.dds` beside a `.ftex`
      (poc4; its pairing by name was read in S-poc6c r1), `975e68a7` (poc9), `6268280c`
      (poc10, the teams list), `14241e1d` (poc11: its SWE-2 and Carbon loops; Astra's round is
      `.tmp/0_0_2/poc11/rulings_poc11.md`). poc1, poc2, poc5 and poc7 were reviewed (S-poc12,
      A-poc12, S-poc57u, A-poc57u, S-poc7, A-poc7mc)
  - [ ] 0.0.2-wheelside sliders, toggles and row selectors move right on wheel down and a
    closed combo to the item below, whatever the setting, now Invert number scrolling
    (`invert_number_scrolling`), which turns number boxes only (the maintainer, 2026-11-15;
    DECISIONS 2026-11-15). Landed (SWE-2, `bdb78173`, merge `4bc6bd55`): ST-WHEEL-05 and 06
    red first; a preset's wheel step writes no undo entry, which ST-WHEEL-05 now says
    - [ ] its review (tier 2): SWE-2's loop ended (S-wheel r1, 2 accepted); one Carbon round
      is due
  - [x] 0.0.2-recents the Save editor keeps its last 10 saves opened or saved between runs
    (`recent_files`, each with its version) and lists them under Recent on the welcome panel
    and in the File menu, each `[PES 2021] backups\EDIT00000000 - VGL26` (P58, the
    maintainer, 2026-11-15; plan and DECISIONS edited) -> verify: SE-GUI-26 and SE-GUI-90 red
    first; a start with the setting lists the earlier run's saves; a deleted one is dropped.
    With it, the File menu's new layout (the maintainer, 2026-11-15; DECISIONS 2026-11-15
    "the File menu: the picker and the writes first"): Open…, Save, Save as… with
    Ctrl+Shift+S, the headings Open from Documents and Recent files, Close save with Ctrl+W,
    Exit with Ctrl+Q (the shell's key); the welcome panel in the menu's order. Draft and
    plan edited (SE-GUI-26, 90, 105; ST-RUN-05). Landed (SWE-2, `10a66ec0`, merge
    `9f2b8bb0`), with two lead fixes: whether each recent save exists is asked once per look
    of the menu or panel, not every frame (`80acbb1e`), and the bundled settings file names
    `recent_files` (`b9686f0b`)
    - [x] the built window beside the draft, 2026-11-16: it matches, but for the welcome
      panel's gap above each heading, about 4 to 5 points tighter than the draft's
    - [ ] its review (tier 2): SWE-2's round 1 accepted 6 ("S-recents r1"); the rework
      landed (SWE-2, merge `6680c856`); SWE-2's round 2 accepted 4 ("S-recents r2"), which
      ends the SWE-2 loop: its rework landed (SWE-2, merge `2923f863`, with Choose file's
      tooltip from 0.0.2-answers)
      - [x] Carbon's round 1, 2026-11-16 (at `d43550fe`): 12 accepted, one in part
        ("C-recents r1"): opening a save rewrote `settings.toml` without its comments, Ctrl+Q
        left an open menu over the prompt, a long recent entry wrapped once the panel
        scrolled; five test gaps, two citations, the changelog, help 02 and stale docs
      - [x] the rework's part A (SWE-2, merge `9ee8e180`): `studio_core`'s settings edit
        keeps the comment above a key whose item changes kind; a held close closes popups
        (ST-RUN-05 widened)
      - [x] part B, the rest (SWE-2, merge `1db0dbe5`): the welcome panel's entries never
        wrap, the prompt closes the changes list and the Tactics popover
      - [x] Carbon's round 2, 2026-11-16 (at `1db0dbe5`): 9 accepted, two in part
        ("C-recents r2"): the File menu cut off in a short window (now scrolls: plan, draft,
        DECISIONS), a key the settings save inserts lands after the last section, five tests
        that pass on wrong code, a stale plan line and mutants exclusion (lead, `0ac94593`)
      - [x] the rework: A2, the settings save (SWE-2, merge `dbcc5358`); B2, the Save editor
        (SWE-2, merge `3ae96fe5`)
      - [x] Carbon's round 3, 2026-11-16 (at `3ae96fe5`): 8 accepted, one in part
        ("C-recents r3"): the scrolled File menu wraps its widest entry, an inserted
        `[save-editor]` writes its recent files above its header, four tests that pass on
        wrong code, SE-GUI-26 at a size that never overflows (lead, `b2f3ecc9`)
      - [x] the rework: A3, the settings save (SWE-2, merge `e5889d60`); B3, the Save editor
        (SWE-2, merge `9a607575`)
      - [x] Carbon's round 4, 2026-11-16 (at `9a607575`): 6 accepted, none of reach (a)
        ("C-recents r4"), which ends the loop: four tests that pass on wrong code, two width
        caps that cap nothing, two docstrings, the plan's 1366 × 768 window height (lead,
        `5dadddeb`)
      - [x] the rework C (SWE-2, merge `82a86c32`)
      - [x] the built window at 1280 × 691 with thirteen saves found and ten recent,
        2026-11-16: the File menu stands at its full height 1.5 s after the click with no
        pointer move (headless, the popup grows over four frames; the real window runs them
        unasked), its middle scrolls, and no entry wraps under the bar
  - [ ] 0.0.2-answers the maintainer's answers of 2026-11-16 that change the build (plans and
    DECISIONS edited that day):
    - [x] code, in two slices -> verify: each a test red first. SWE-2 (merge `498ef8e4`):
      the lit area, the manager id, the rail, Substitution, the Gameplan's gaps. Opus (merge
      `9c07f067`): the Physique box, the indent, `Star rating` and its key, the Kit slots'
      widths (51 points, the draft's too):
      - a held starter rated A or B nowhere darkens the whole pitch; an empty slot or no
        ratings still darkens nothing (`team_widgets` `pitch/lit_area.rs`)
      - a Team TOML import writes a `manager_id` equal to the file's team id as the target's
        id, any other as it is (`pes_savefile` "Team TOML")
      - the Kit slots dialog's boxes one width, six digits wide (`view/team.rs` `kit_slots`)
      - the Ruleset editor's rail rows 24 apart (3 points of padding)
      - the Appearance page's Physique box as wide as the draft's (Motion 24 points right),
        its labels 6 points in from its title, and the Colors squares 2 points apart
      - every box's content on the Stats and Appearance screens 6 points in from its title,
        as the draft draws it (`tools/save_editor.js`; the maintainer agreed 2026-11-16)
      - the Presentation group's Star labelled `Star rating` (`appearance/fields.rs`, help
        04, the tests that name it), and its key in `[appearance.presentation]` renamed
        `star_rating` in `settings.toml` and the Team TOML alike (`settings_toml/keys.rs`,
        the tests and the `aatf` sample that spell it, the `[Unreleased]` line naming the
        table); the Team TOML's old place (`stats.star`, `stats.playing_attitude`) no
        longer read (`MOVED_TO_PRESENTATION` and its tests go), with an `### Important`
        changelog line saying how a tester recovers. The layout slice was briefed to keep the
        old place: its review sends this as a rework
      - the Gameplan screen's panels 8 points apart, Automatic settings ending level with
        the lineup lists' bottom (2 points shorter)
      - Substitution offers Very early on PES 15 and 16 too (`tactics/mod.rs`
        `substitution_choices`, help 08, the two comments)
      - [x] Choose file's tooltip reads "starting in the folder of the last opened save":
        landed with the recents round 2 rework (merge `2923f863`)
    - [x] the built window beside the drafts: the Stats indent, Appearance with Motion's 24
      points less room, the Gameplan's 8-point gaps, Kit slots at 51. Looked at (lead,
      2026-11-16, the release build of `5e086fbe`): the indent, Motion's place, the 8-point
      gaps, Automatic settings level with the lists and the boxes' one width match the draft.
      - [x] five differences fixed (SWE-2, merge `a34bfa97`): Kit slots' label columns no
        longer 51 wide, the Gameplan unscrolled at the default window, Colors 230 wide, the
        Motion, Presentation and Copy from ID boxes 52, the title row's line at 56.5 (the
        draft's 57). Rechecked (lead, 2026-11-17, the release build of `a9ec4919`, captures
        in `.tmp/lead/ans2/`): each within 1 point of the draft
      - [x] two more differences the recheck met, each to a fix:
        - Kit slots: each label and its box in two grid columns, 8 apart, where the draft
          has them in one cell a space apart (3): every box 5 right of the draft's. Fixed
          (SWE-2, `4017c877`): one cell each, `SPACE_GAP` apart. Looked at (lead,
          2026-11-17, the release build of `97cc9757`): each box 3 after its label, the
          columns within 1 point of the draft's. Left over: the whole grid stands about 4
          points left of and above the draft's, since the draft's `table.grid` pads its
          cells 4 left and 3 above where egui's Grid has no outer margin: left as is, the
          dialog redesigned after 0.1.0 (16.k)
        - the Stats screen: its sections 9 under the title row's line, the draft's 12, so
          the screen stands 4 to 5.5 higher than the draft once the line moved up. Fixed
          (SWE-2, merge `f8e4d661`; it takes in 0.0.2-gaps' Player screen item), looked at
          in the release build of `f8e4d661`: headings 1 higher than the draft's (the line
          at 56.5 against 57, and the text's pixel snapping), each first row within 0.5 of
          the draft's under its heading. The positions grid stood 2.5 higher (1.5 nearer
          its heading): the draft follows the build (the maintainer, DECISIONS 2026-11-17),
          the screen now within 1 point everywhere (Gameplan rechecked too). The layout
          tests still cite the old draft's numbers in their comments (the toggle's 22.5,
          the grid's 63.7 under its title, `view/tests/abilities/layout.rs`); their values
          pin the build, which did not move, though headless they measure the grid 1.3
          lower than the real window does: respell when next touched
    - [x] the drafts (the lead), each captured and critiqued: Kit slots shows `Bound team
      ID` as the id in boxes of one width (72 points, six digits); Randomize greyed whole;
      the three texts of DECISIONS 2026-11-16 "the drafts take three texts"; `Star rating`;
      the Gameplan screen's 8-point gaps
    - [x] in-game, PES 2017, 2026-11-17 (SWE-2, `.tmp/0_0_2/ingame/coach17/findings.md`):
      two teams naming one coach record show no fault, but the first lines up with the
      second's starting eleven; in the savefile plan's "manager id" paragraph
    - [ ] PES 2019's playing attitude (the maintainer: not in Edit mode on 2019 or 2021, a
      hidden stat; star ratings neither): look for it in
      the PES 2019 save folder's `EDIT00000000 - defaultdb` (Documents, `KONAMI\PRO
      EVOLUTION SOCCER 2019\<account folder>\save`), a default-database save, against the same players' PES 2020 values -> verify: a field
      whose values match theirs, then the editor shows it on 2019; none found, the matter
      is dropped
  - [x] 0.0.2-tacgap the Tactics grid's two column gaps were 10 and 8 (the draft's
    `column-gap: 10px`, its pitch's border overflowing a 320 track): 6 each, as between every
    pair of panels, the pitch's track 322 (the maintainer, 2026-11-15; draft and plan edited)
    -> verify: a test measures both gaps from the drawn frames, red first (10 and 8 before);
    landed (SWE-2, merge `cb074624`); the pitch's turf corners rounded with it (0.0.2-pitchround,
    merge `b700d3fa`). Built window measured at `b700d3fa` (both gaps 6), card
    `look-tactics-gaps`: the gaps fine (the maintainer, 2026-11-17); the grey ring inside
    the rounded corners is to take the turf's green, after 0.0.2 (0.1-pitchring)
  - [x] 0.0.2-picwidth (`be3913bf`, merge `5dfad4e2`) the pictures dialog kept its width only
    for some folder paths: egui rounds a wrap's width to a whole point, so a line landing in
    the fraction reported wider than the pinned width; the run's labels now wrap at its floor,
    a unit test sweeping line lengths at three fractional widths
  - [x] 0.0.2-teamfill (2026-11-15, merge `457f30c8`) the player list's team rows take the
    draft's `#202020`: the palette's `team_row_fill`, mid grey at 48 of 255, one value for both
    themes (grey 231 on the light list); the built list beside the draft, both themes, sampled
    at `#202020` and grey 231 (`.tmp/0_0_2/teamfill/pair.png`)
  - [x] 0.0.2-nostretch a larger window grows or moves no panel but the named lists
    (`plans/core/gui.md` "Shell layout", the paragraph "A window larger than the default";
    DECISIONS 2026-11-14 "no panel stretches unless it is allowed to") -> verify: per view, a
    test drawing it at 1280 and at 1920 wide, every panel that cannot scroll at the default size the same size,
    moved right by half the extra width;
    the drafts and the built window at the whole monitor
    - [x] the census (2026-11-14, SWE-2): `.tmp/stretch/census_findings.md`
    - [x] the drafts (2026-11-14, lead): `--calib-w` (1208, the view's width at the default
      window) caps each view, centred; the Squad table, the Files look, the Compiler's grid
      (height), the Ruleset editor's rail and shown preview and the player list take the room;
      measured at default (unchanged) and at 1920 x 1080 with
      `.tmp/stretch/drafts/measure.py`, captures `.tmp/stretch/drafts/*_monitor.png`
    - [x] a slice per tool (after 0.0.2: DECISIONS 2026-11-16)
      - [x] the Save editor: merged `b0703afa` (2026-11-21, SWE-2, one rework), tests
        `view/tests/stretch.rs`; seen in the release build at 1280 and maximized beside the
        drafts (Stats, Gameplan, Live AATF's dock spanning the pane); SWE-2 round S1w's
        fixes `cbb13f6a`. The Squad screen takes its rule when it is built
      - [x] the Ruleset editor: merged `172e302a` (2026-11-21, SWE-2, two reworks), tests
        `view/tests/stretch.rs`, the default pane's width one `studio_core` constant
        (DECISIONS 2026-11-21); seen in the release build at 1280 and maximized beside the
        draft, preview hidden and shown (the draft's preview fixed to fit its 100 columns);
        SWE-2 round S1re's fixes in the merge
      - [x] the Compiler and the Export upgrader: nothing to slice (2026-11-21): both windows
        are placeholders (a settings list and a Compile button; one line of text), so step 8.10 gives each
        the rule as it builds them
  - [x] 0.0.2-lib the library rerun against checkpoint `p11`, 2026-11-13: checkpoint `002`
    (422 clean inputs; `.tmp/library_parity/checkpoints/diff_p11_002.txt`), every change
    traced to a commit since `3c512628`: VGL27's team IDs (`6268280c`), the short Common
    forms in an `.fmdl` (`a2b948c6`), `texture_not_div4` gone (`33e5d5a1`), and two
    upgrader rules (`e6c672e4`). The respelled `.fmdl` keeps the long Common path as a
    string no texture names (checked 2026-11-14; the `p11` file holds one such string too)
  - [ ] 0.0.2-upd Update now (`plans/core/distribution.md` "Self-update" steps 3, 4 and 6;
    DECISIONS "Update now waits for 0.0.2", "Update now's dialog: what the plan left open"),
    postponed from 0.0.1's f1g9 → verify: ST-UPD-03, 05 and 06 headless, ST-UPD-04 by hand
    - [x] slice 1, 2026-11-11 (`b6954512`, Opus): the updater's core, `updater::install`
      (download, SHA-256, unpack, the swap with its undo)
    - [x] slice 2a, 2026-11-11 (`7e37bb5c`, Opus): the dialog's stages, Update now, the
      download's progress and Cancel, `Update failed`, `Update ready` (ST-UPD-03)
    - [x] slice 2b, 2026-11-12 (`62b92662`, Opus): `Update ready` on the status bar, Restart
      now, `UTW_RELEASES_URL` (ST-UPD-05); the tests for slice 1's five mutation gaps
    - [x] the dialogs and a real restart looked at in the built exe, 2026-11-12, against a
      releases list served from this PC over plain http (`UTW_RELEASES_URL`): every stage
      drawn (the offer with a release's real notes, the download, `Update failed` on a
      checksum that does not match and on a program another window replaced, Cancel,
      `Update ready`, Later and the next start, What's new and the help's Updates chapter).
      The restarted program takes log slot 2 (`utw.2.log`) while the old one is still
      closing, and slot 1 again at the next start alone; the swap leaves no temp folder
      and keeps `old/UTW.exe`. Found and fixed (`a19cc7b5`): the offer back after a Cancel
      or a failure was wrapped to the download's narrower dialog. Captures:
      `4cc-studio-temp/st-upd-04{,b,c}/shot_*.png`; the scripts: `.tmp/0_0_2/upd/rc/`
    - [x] slice 3, 2026-11-12 (`d617bf3a`, Opus): the `Updated to` notice
      (`last_run_version`), the help's Updates and What's new chapters (ST-UPD-06), the
      offer's tooltip naming the app and the version
    - [x] review round 1 (tier 1, SWE-2), 2026-11-12 (`caf88b0f`, Opus): 23 of 43 concerns
      accepted: Update now refused when another window replaced the program, Cancel read
      once more before the swap and disabled once clicked, a check interval of 0 read as a
      minute, the name lookup's timeout, and the endings and dialog moves no test reached
    - [x] review round 2 (SWE-2), 2026-11-12 (`314e2551`, Opus): 7 of 12 concerns accepted:
      the program's stamp compared again before the swap (a second window clicking Update
      now during the first one's download), each field of the stamp and the "old files not
      put back" ending tested, a member's file inside a folder the bundle ships
    - [x] review round 3 (SWE-2), 2026-11-12: one concern, rejected, which ends SWE-2's loop
    - [x] Astra's round, 2026-11-12 (`deafb774`, the lead), ahead of Carbon's loop: 3 of 4
      concerns accepted, which ends Astra's loop: a Cancel over a stalled connection brings
      the offer back when the wait times out, the help's Restart now says the restart is
      dropped whenever a tool asks about unsaved work, and a test reads the command line's
      notice after the command's own lines (ST-CLI-05)
    - [x] SWE-2's round over Astra's fixes, 2026-11-12: 2 of 4 concerns accepted, both in
      the new tests (the notice test read the machine's own settings; no test kept a
      failure a failure after Cancel), which ends the subloop
    - [x] Carbon's round 1, 2026-11-12 (`a1b233ea`, Opus; merge
      `067f5222`): 7 of 11 concerns accepted: Restart now started the new program before
      the closing one released its lock, so it opened as a second instance (seen in the
      three manual runs' `utw.2.lock`); a succeeded update whose temp folder could not be
      deleted refused every later one; Cancel read before each request; the offer tests
      survive the version bump of the release cut; ST-CLI-05's stderr half asserted
      (DECISIONS 2026-11-12 "An update that succeeded marks its leftovers")
    - [x] the SWE-2 subloop over `a1b233ea`, 2026-11-12 (`96757ebb`, SWE-2): 4 of 12
      concerns accepted, all tests that passed on wrong code (a kept file below the
      backup's top level, no mark after a failed update, a version that is not the
      build's, a cancel set before the call), which ends SWE-2's loop
    - [x] Carbon's round 2, 2026-11-15 (at `2f9b9c44`): 8 of 8 concerns accepted: the bar
      kept `Update available` on an idle window after a hidden install ended; a failed
      Restart now was only logged, past the log's closing line, so the next start reported
      a crash; `skipped_version` untested through the window's check and a tool command;
      Escape on `Update failed` untested; two stale doc comments; the lead's text: the
      settings file's comment, the help's Restart now sentence, the changelog's launchers
      (`4705095d`)
    - [x] the rework (SWE-2, 2026-11-15; merge `6d5a9ab6`)
    - [x] Carbon's round 3, 2026-11-15 (at `766ccf6c`): 7 of 7 accepted: an idle window
      never wakes for its next check or retry (the drain frame schedules nothing); four
      tests that pass on wrong code; the help offered deleting the only copies of the old
      files after a failed restore (`ef105c80`, the lead)
    - [x] the rework (SWE-2, merge `eeb4fa6f`), then Carbon's round 4 (6 of 6 accepted: a
      click on the bar's update entry dismissed an unrelated notice; four tests that passed
      on wrong code) and its rework (merge `8b40f887`)
    - [x] Carbon's round 5, 2026-11-15 (at `8b40f887`): 5 of 5 accepted: no test sees the
      workers stop waking an idle window; four items wider than their users; two stale
      docs; the help names the offer's window "Details" and puts Cancel on the bar
    - [x] the rework (SWE-2 and the lead's help text, merge `88b24e95`)
    - [x] Carbon's round 6, 2026-11-15 (at `88b24e95`): no concerns, which ends Carbon's
      loop. Astra rerunning round 5's brief (the maintainer's comparison) found two
      Carbon had not: the command line's notice pointing at an offer a reopened window
      lacks, and a test racing its worker on a 300 ms sleep ("A-upd r5x")
    - [x] every start checks and the open window rechecks every 5 minutes, the two keys
      gone (merge `ff033e52`; the spec's tables and the changelog's Important line after
      it). No test or script that starts the real program reaches GitHub (`1ae7e171`)
    - the settings menu's rollback button is not a step: nobody asked for it, and it waits
      for a ruling as `maintainer/proposals.html` P51
    - [x] ST-UPD-04 by hand, on Windows and on Linux. Windows 2026-11-12: the bundle of
      `0.0.2-rc.1` updated itself to `0.0.2-rc.2` and reopened on `Updated to v0.0.2-rc.2
      pre-alpha`, the settings, a file inside `exports/` and a root log file as they were
      (both bundles built by `just release` on the throwaway branch `test/upd-rc`, worktree
      `4cc-studio-temp/rc`, never merged or pushed).
      manual: checked 2026-11-12 by the maintainer on Linux, in a virtual machine: the
      update passed. It surfaced two things: the Reset window button did nothing on the
      machine's 1280 x 800 monitor, where the fit asks for more than the desktop gives
      a window (fixed the same day: the button goes after a click the window did not
      follow, and is not drawn at the minimum a smaller monitor holds the window at;
      not seen again on that machine yet),
      and the update notice's tooltip, reworded. Its kit,
      `4cc-studio-temp/st-upd-04-linux/` (README, the two bundles built on the VPS by
      `just remote just release <version>` from the same branch with main's fixed release
      recipe copied in, a releases list served from the machine's own 127.0.0.1): the
      layout and server scripts were tried on this PC; its README also says how to
      serve the folder to the machine from the host
  - [x] 0.0.2-tls 2026-11-12: the update check's TLS keeps `ring`, as the first listed
    exception to "the shipped build is pure Rust by default" (DECISIONS 2026-11-12, the
    lead's ruling for the maintainer to reverse; measurements by an Opus sidekick on a
    bench worktree, `.tmp/0_0_2/tls/out/report.md`): `rustls-rustcrypto` is a pre-release
    its readme bars from production, `rustls-graviola` panics on a processor without AVX2
    and ADX, the system's TLS means OpenSSL on Linux. `CONTRIBUTING.md` "Dependencies"
    states the rule, and `just deps-check` runs its gate (`scripts/gates/c_crates.py`,
    seen red with `ring` unlisted). Not chosen and open to the maintainer: SChannel on
    Windows alone, `ring` kept on Linux
  - [x] 0.0.2-cmp 2026-11-12 (`5412b447`): the comparator's field rows in three columns,
    Item, Old value and New value, under one head row (SE-GUI-13)
  - [x] 0.0.2-drag 2026-11-12 (`5412b447`): the player list's dragged row floats under the
    pointer, its grab point kept, its place left blank (SE-GUI-69)
    - [x] review rounds over both (tier 2). SWE-2 S-se1 r1, 2026-11-12 (two parts): 4 of
      10 concerns accepted (`a6174e84`, Opus; merge `78212109`): the New value cut with an
      ellipsis as the other two columns are, and three tests that passed on wrong code
      (the cut's width bounded from below, the whole dragged row watched, the float gone
      after the release); its loop ended. One paid round is owed (Carbon)
      - [x] Carbon's round 1, 2026-11-15 (at `2f9b9c44`): 9 of 10 concerns accepted: a
        Comparator panel dragged narrow cut its values with no ellipsis; a held row slid
        away from the pointer and vanished while the wheel scrolled the list; Esc's cancel
        of a drop untested (a test first); two tests that could not fail; the lead's text:
        the plan's drag and Comparator paragraphs (`10a7b2d3`)
      - [x] the rework merged (`9e757880`); due: a SWE-2 loop over it, then Carbon's round 2
        (round 1 had two of reach (a)). SWE-2 round "S-cmp c1", 2026-11-23: 6 accepted, one
        a group member's float drawn unselected; its fixes, with S-sel c1's, merged
        `044955c1` (SWE-2). Round c2 over them found nothing, which ends the SWE-2 loop
      - [x] Carbon's round 2, 2026-11-23 (at `bb2ccfb4`, rulings "C-cmp r2"): 9 accepted. Its
        fixes, slice C1 (SWE-2), merged `1774139b`: the float gone on the release frame, its
        name cut where the row's is, the Comparator's narrowest width tested against where its
        last heading ends.
        Its one concern of reach (a), a held row's id passed to another row while the wheel
        scrolled, does not happen (egui's `show_rows` keeps the ids; rulings "C1 review"), so
        the round holds none and the loop over the drag and the Comparator ends
  - [ ] 0.0.2-bar the toolbar as a menu bar (the maintainer, 2026-11-07 and 2026-11-12): the
    Ruleset editor draft's look, unframed menus on a band edge to edge, on the Save editor
    and the Ruleset editor; in the Save editor `File` (the opens, then Save and Save as) and
    `Edit` (Undo, Redo) take the buttons' place, and with an event logo drawn the band ends
    as far left of it as the logo stands from the window's edge (`plans/save_editor.md`
    "The toolbar", "The event logo")
    - [x] 2026-11-12: the Save editor's draft and plan, copied by hand from the branch
      `ui/save-editor-polish` (cut afterwards), the band's stop corrected for a window
      drawn scaled down; the Ruleset editor's draft already has the look
    - [x] the two apps, 2026-11-12 (`35224d61`, Opus; tier 2): `studio_core`'s `toolbar`
      draws the menu bar's style and `toolbar_framed` keeps a control framed; SE-GUI-03, 04,
      26, 43 and 44 restated and their tests re-aimed at the menus. The row is one line
      that never wraps and the logo is always drawn (DECISIONS 2026-11-12); a view's title
      is measured from the band's line, and everything under the bar stands 1 point higher
    - [x] the look in the built window, both tools, both themes: checked 2026-11-15 at
      `4b9f2258`; the maintainer's look 2026-11-17: fine, the three menus greyed with no save
      open as built (the draft follows since)
    - [x] the Ruleset editor's start page draws no separator after Randomize (merge
      `c20e6b29`)
    - [x] its SWE-2 loop, 2026-11-12 (`b1f3c85e`; two rounds, 6 of 11 then 1 of 2): the
      logo's place is read once a frame and handed to the band and the logo both, and
      tests pin the framed dropdown and Live AATF, the Ruleset editor's row 3 points
      inside its panel, the team view's last row at 739 and no logo on the frame a save
      opens
    - [x] Carbon's round 1, 2026-11-15 (at `2f9b9c44`): 14 of 15 concerns accepted: a Ctrl
      shortcut ran with Shift or Alt held too, so Ctrl+Shift+S, Save as elsewhere, saved
      over the open file; an undo step reached back past a save; the Player menu differed
      with a team row selected; recent-file labels used backslashes on Linux; the welcome
      list kept a stale entry; tests and citations; the lead's text: the help's Undo
      paragraph and a changelog clause (`10a7b2d3`)
    - [x] its rework (merge `2e23f811`), then Carbon's round 2: 8 of 9 accepted; the
      rework (merge `27afa199`): the Ruleset editor's Undo keys work on a focused
      checkbox, no Player menu without a selected team, the welcome panel over the panes
    - [x] Carbon's round 3, 2026-11-15 (at `069eb2ea`): 11 of 14 accepted
    - [x] the rework (SWE-2 and the lead's help text, merge `b44c8e1e`)
    - [x] Carbon's round 4, 2026-11-15 (at `b44c8e1e`): 5 of 6 accepted
    - [x] the rework (SWE-2, merge `fa472c3b`)
    - [x] Carbon's round 5, 2026-11-15 (at `fa472c3b`): 6 of 7 accepted: three tests that
      pass on wrong code, a dead width branch, stale comments, SE-GUI-79's Save as notice
      (the lead's plan edit)
    - [x] the rework (SWE-2, merge `9d536eb4`)
    - [x] Carbon's round 6, 2026-11-15 (at `81c539d7`): 9 of 10 accepted: the welcome panel's
      Recent never seen (dropped, then restored by 0.0.2-recents), untested clauses, a
      surviving mutant, stale citations and comments
    - [x] the rework (SWE-2, merge `5b5546d8`)
    - [x] round 7: Astra in Carbon's place (the maintainer, 2026-11-15: a cross-check on a
      loop that has not converged; Astra's round over round 3's surface, "A-bar r3", found one
      gap Carbon never did); it carries A-bar r3-4 (the cancelled Save as test). Done
      2026-11-16 at `2923f863`: 4 of 4 accepted ("A-bar r7"), under five, so the loop ends
      with its rework: the welcome panel scrolls once ten kept recents outgrow a 600-point
      window, a changelog line's "shortcuts are unchanged", Ctrl+Shift+S and Ctrl+W in the
      close-the-menu test, and the plan's File row (fixed by the lead)
    - [x] round 7's rework (SWE-2, merge `d43550fe`): the bars' review loop ends
  - [x] 0.0.2-dragmark (2026-11-15, merge `96aef711`) a Starting eleven or Bench row held over a
    Tactics pitch player marks the player its drop swaps out, as a pitch drag marks its target;
    one hit test (`marker_at`) for the drop and the mark, the keeper and an illegal formation
    included; SE-GUI-59. The built window beside the draft (`.tmp/lead/bw/pair_dragmark.png`)
  - [ ] 0.0.2-num Numbers in attack and Numbers in defence disabled on a PES 17 to 21 save
    unless the active ruleset allows illegal formations (the maintainer, 2026-11-12;
    `plans/save_editor.md` "Tactics editor", SE-GUI-71; DECISIONS 2026-11-12)
    - [x] 2026-11-12 (`33423b13`, SWE-2; tier 3): the draft, `formation_settings` taking
      the version and the flag, the tooltip, the CHANGELOG line; the choice a disabled
      toggle holds stands on `locked_choice_fill` (`#2c2c2c`, the maintainer's pick; grey
      225 in the light theme by the contrast rule), and the formation selector locked on
      Kick-off marks it the same way
    - [x] its SWE-2 round, 2026-11-12 ("S-num r1", 1 of 2): the help chapter on the
      tactics says when the two numbers are disabled
    - [x] the auto levels (Opus sidekick, 2026-11-15, merged `62e2d0b5`; DECISIONS
      2026-11-14 "on PES 17 to 21 the two numbers are the auto attack and defence levels";
      SE-GUI-71): labelled Auto attack level and Auto defence level on PES 17 to 21, and
      also disabled while Attack/defence levels is off, with its own tooltip
    - [x] the lead's check in the built window (2026-11-15, dark, the PES 17 save): the two
      labels; under 4cc Autumn 26 locked with "Used only while Attack/defence levels is on,
      in Automatic settings" until the flag is ticked, then editable; under /vg/ League 27
      locked with "Not editable in the game from PES 2017 on" with the flag ticked
    - [x] the look in the built window, both themes: checked 2026-11-15 at `4b9f2258`; the
      maintainer's look 2026-11-17: fine
  - [x] 0.0.2-num17 2026-11-14 (`b518585a`; lead, Opus sidekick): who reads the two numbers
    in PES 2017, inconclusive. A live memory watch (`.tmp/num17/`) and a static search of the
    dump in Ghidra (`4cc-studio-temp/pes17_re`, report `out/report_re.md`) found only copiers,
    the game plan menu and a serialiser reading them, and no match-side reader of any slider;
    the spec's paragraph says so. The live caller trace moves after 0.0.2 (below), and 0.0.2
    follows the managers' reading (0.0.2-num's auto levels item)
  - [ ] 0.0.2-sel (tier 2) the PES selector drawn only for a tool that targets the version, its place
    kept; the Save editor follows the open save (the maintainer, 2026-11-12; DECISIONS
    2026-11-12; `plans/core/gui.md` "Sidebar" item 2, ST-SIDE-09; `plans/save_editor.md`
    "Set PES path", SE-GUI-26, 28 and 50). Tier 2
    - [x] a (2026-11-12, `58892900`, merge `9ffffa8c`; SWE-2 and the lead): the Save
      editor's portraits, Set PES path and its pick by the save's version, through
      per-version reads on the common settings and the tool context; the version warning
      gone (SE-GUI-28 withdrawn); the picker's start folder (`last_folder`); draft and help
    - [x] b (2026-11-12, `de01a410`, merge `af786a5e`; SWE-2): the shell:
      `StudioTool::targets_pes_version` (the Team compiler and the Export upgrader), the
      selector disabled and empty under a tool that does not use it (ST-SIDE-09), the
      standing notice and `SwitchVersion` removed (ST-BAR-07, 08 withdrawn)
    - [x] its SWE-2 loop (2026-11-12; rulings "S-sel r1" and "S-sel r2", five then two
      accepted; fixes `1c765193` and the commit after it). Carbon owed
    - [x] Carbon's round 1, 2026-11-15 (at `2f9b9c44`): 11 of 13 concerns accepted:
      portraits stayed blank after the PES folder was repaired and picked again; an unused
      context API (`pes_exe_check`) beside the sidebar's own check; six tests that could not
      fail; the lead's text: the help's PES version paragraph and Set PES path trigger, the
      architecture section, a changelog line (`cef04d9c`)
    - [ ] the rework merged (`97d073e4`); due: a SWE-2 loop over it, then Carbon's round 2
      (round 1 had one of reach (a)). SWE-2 round "S-sel c1", 2026-11-23: 2 accepted (a test
      copying a helper's loop, a doc), which ends the loop; one concern waits on
      `maintainer/questions.html` "Portraits after a compile in the same session". Its fixes
      merged with S-cmp c1's (`044955c1`)
    - [x] Carbon's round 2, 2026-11-23 (at `bb2ccfb4`, rulings "C-sel r2"): 7 accepted, none of
      reach (a), which ends Carbon's loop: texts still saying every tool uses the version,
      and three guards meant to act once per change, each tested on one frame. The lead's
      text: SE-GUI-50 and the drafts' tooltips (`72bfe950`). Its fixes, slice C2 (SWE-2),
      merged `b96b64b7`; SWE-2's round over them ("S-c2 r1"): 1 accepted, the label
      tooltip's test (`34b0253f`). The review loops are done
    - [x] the look in the built window, both themes: checked 2026-11-15 at `4b9f2258`; the
      maintainer's look 2026-11-17: fine
  - [x] 0.0.2-undo an X on each row of the unsaved changes list undoing that change alone,
    if a design for changes spanning many fields holds (the maintainer, 2026-11-03). It
    holds once a move is one row (the maintainer, 2026-11-12; DECISIONS 2026-11-12;
    `plans/save_editor.md` "Editing UI", the two bullets after the changes table's;
    SE-GUI-73, 74): a move trades whole records between ids and filled the list with every
    field of the players it shifted. Tier 2. After 0.0.2-clip merges (the same crate)
    - [x] a 2026-11-13 (`4090e2b4`, Opus with one rework and the lead; merge `969e6f3f`;
      the lead's rerun of `save_editor`, clippy, fmt; SE-GUI-73, reworded: undo is
      linear): the session remembers whose record each id holds (`Origins`), a move's
      undo entry keeps the trade itself so an undo past a save still reads right, the
      list compares a player with the record he came from, and each group of places
      that traded players is one `Player order` row. Two overlapping drags give groups
      that match neither drag (the cycles). Look: checked 2026-11-15 at `4b9f2258`; the
      maintainer's look 2026-11-17: fine
    - [x] a's SWE-2 review 2026-11-13 (S-undo-a r1, 1 of 5 accepted, the loop ends; its
      fix, a moved player renamed after the move, landed with b)
    - [x] the draft 2026-11-12 (the lead): `changesTable` draws the `Player order` row,
      one place per line (approved), and the X ending each row in the link blue, its
      column fixed at the list's right edge while a wide row scrolls under it (the
      maintainer, 2026-11-12: first in the row it read as an error mark); renders
      `.tmp/0_0_2/undo/x_list.png`, `x_wide.png`, `x_wide_scrolled.png` by `shot_x.py`
      and `shot_x_wide.py` beside them. The maintainer's look 2026-11-17: fine
    - [x] b 2026-11-13 (Opus, the same session, and the lead; merge `f682295e`; the
      lead's rerun of `pes_savefile`, `aatf`, `team_widgets` and `save_editor` on the
      merged tree, clippy, fmt; SE-GUI-74): an X ends each row of the toolbar's list and
      puts that change back as one sealed undo step, the list open a row shorter; a row
      is put back by reversing its diff and applying it (`PlayerDiff::reversed`,
      `TeamDiff::reversed`, `apply_team`), a `Player order` row through the drag's own
      trade; the close prompt has no X. Look: checked 2026-11-15 at `4b9f2258`; the
      maintainer's look 2026-11-17: fine, the X to be highlighted on hover (0.0.2-xhover). Open: the table's heads (Item, Old value, New value, Undo) are drawn
      weak, grey 95 measured, where the draft's `table.grid th` draws them in the strong
      colour, 220; the close prompt's share them (`view/prompt.rs` `changes_table`); fixed
      by 0.0.2-namelight (`40ce82da`)
    - [x] b's SWE-2 review 2026-11-13 (S-undo-b r1, 3 of 5 accepted, the loop ends): a
      doc comment and a help sentence fixed
  - [x] 0.0.2-line the player list's drop place as a line between two rows, not an outline
    round one (the maintainer, 2026-11-12: a drop inserts and shifts, it swaps nothing;
    `plans/save_editor.md` "Editing UI", the drag-and-drop paragraphs). Drafted 2026-11-12
    (`.dropabove`, `.dropbelow`; renders of a drag each way, `.tmp/0_0_2/undo/dropline_down.png`
    and `dropline_up.png`, by `shot_dropline.py` beside them); the maintainer looked at
    the draft and its light approximation and approved both, serifs included (2026-11-12).
    SE-GUI-75. A light theme in the drafts was weighed and left out: 175 colours of
    `index.html` and `save_editor.js` are written as literals beside the variables, so it is
    no cheap addition (`shot_dropline_light.py` overrides the variables for a one-off look)
    - [x] the slice 2026-11-13 (`da849d2a`, SWE-2 and the lead's rerun: 823 and 54 tests,
      clippy, fmt): three painter calls in `player_row`, the SE-GUI-75 test reading the
      line's and serifs' segments. Measured by the sidekick: the list's clip cuts the half
      of the line outside a row flush with the view's edge. Look: checked 2026-11-15 at
      `4b9f2258`; the maintainer's look 2026-11-17: fine
    - [x] its SWE-2 review: "S-line r1" 2026-11-13, 5 of 7 accepted (`.tmp/duck_rulings.md`):
      the next row's fill covered the line's lower half (a team row under ALL, a selected
      row), and a pointer on the edge two rows share drew two lines; the mark is painted
      once after the rows; SE-GUI-75 loses its empty slot's row (there is none) and gains
      both clauses. Rework landed 2026-11-13 (`b7b67cc5`, SWE-2 and the lead's rerun: 825
      and 54 tests, clippy, fmt). "S-line r2": 4 of 5 accepted, the loop ends; its rework
      landed 2026-11-13 (`57954310`; 826 and 54 tests, clippy, fmt): only a row that
      accepts a drop takes it (on the edge under the dragged row the line showed a drop
      that did nothing), and the line's ends stop inside the list's border, which was
      stroked over each serif's outer point
  - [x] 0.0.2-cup only the cup's teams are shown (the maintainer, 2026-11-13, on a review
    concern about players shared between rosters: "there's no need to show the game's own
    clubs and national teams. We should filter them out, even"; asked, they chose the
    filter everywhere with no switch, and their players hidden too; DECISIONS 2026-11-13;
    `plans/save_editor.md` "Editing UI", the bullet "Only the cup's teams are shown")
    - [x] its Acceptance scenarios 2026-11-13 (the lead): SE-GUI-77 (the selector, ALL,
      the filter, Multi-team AATF, Fix database, and a write keeping the hidden
      records' bytes), SE-AATF-10 (`check-aatf` with and without `--teams`)
    - [x] the slice 2026-11-13 (Opus, two runs, and the lead; merge `b60cec7e`; the lead's
      rerun of `save_editor`, 852 + 57, clippy, fmt on the branch; SE-GUI-77, SE-AATF-10,
      SE-DIF-07, SE-TOML-10): one rule, `session/cup.rs` on `teams_list`'s range, feeds
      the selector, ALL, the filter, Multi-team AATF, Fix database, the roster sheet, the
      copy and swap dialog, the comparator and `diff` (a player a cup roster holds in
      either save), `check-aatf` without `--teams`, `export-toml --all-teams` and
      `export-teams-list`; the baselines, the unsaved changes, undo and the write stay
      whole. The tests' save is the fixture with /aco/ renumbered 300 and one player off
      every roster (`src/test_support/cup_save.rs`, shared with the CLI suite by `#[path]`).
      PES 2019's fixture already holds teams outside the cup (346 teams, ids 100 to 420)
    - [x] its SWE-2 review 2026-11-13 (S-cup r1, 3 of 6 accepted, the loop ends; its
      sweep of the unchanged walks placed none differently): two help sentences and the
      test save's doc (`4d3cda84`)
  - [x] 0.0.2-side a window too short for the sidebar's entries (the maintainer,
    2026-11-13: the tail's separator and Help drawn over the last tool entries)
    - [x] the slice 2026-11-13 (SWE-2 and the lead's rerun of `studio_core`, clippy,
      fmt; ST-SIDE-10): the tool entries scroll by the wheel in the room between the
      head and the tail, which keep their places; no scroll bar; one offset for the
      rail and the opened sidebar; an entry focused by Tab is scrolled into view (egui
      does not do it by itself). egui's fade at a scroll area's cut edge is left on.
      Its test starts `sidebar/tests/`; moving `tests.rs`'s other tests there is not
      done. Look: Pending looks
    - [x] its SWE-2 review 2026-11-13 (S-side r1: 2 of 8 accepted, the loop ends)
  - [x] 0.0.2-rad 2026-11-13 (`4bdfc5cf`, merge `b1274e74`, review fixes `842f41e3`): one
    corner radius, 2 points, everywhere: one constant in `theme.rs` sets all seven of
    egui's radii in both themes and the suite's own painting reads the style
    (ST-CORNER-01); the captain's mark keeps the game's radii, radio buttons stay circles.
    The update download's bar is painted in no test. Look: the corners checked 2026-11-15 at
    `4b9f2258`, the maintainer's look 2026-11-17: fine; the bar waits for the first real update (no hook fakes one)
  - [x] 0.0.2-gap no gap between the choices of a menu or a list (the maintainer,
    2026-11-12: a click in the thin gap between two entries of a toolbar menu picks
    nothing, as the team dropdown did before `699a77ab`; "We should try to solve this at
    the root, there is no real reason why there should be gaps between the choices in a
    menu"). Read 2026-11-12: egui lays a menu's and a combo list's rows 18 points high and
    3 apart (`item_spacing.y`); the earlier fix left the gap and gave each combo row a
    click strip half the gap high above and below it (`studio_core`'s `widgets/combo.rs`,
    ST-COMBO-01), which the toolbar menus, drawn through `widgets/menu_button.rs`, never
    got. The root: the rows meet, each 21 high with no spacing, its text where it stood,
    so no click can land between two; in the menus through `menu_button`'s one place, in
    the combo lists through `combo_label` itself, whose strips then go; a separator keeps
    its room. The highlight grows from 18 to 21 points, in the drafts' `.combo-pop` and
    menus too (`docs/prototypes/index.html`, the comment "A menu as egui 0.37 draws one")
    - [x] the draft and the plan 2026-11-12 (the lead): `.menu` and `.combo-pop` rows 21
      high with no gap, separators keeping their room; `plans/core/gui.md` "Other UI
      components" (the row on a menu's and a combo's rows), ST-COMBO-01 reworded,
      ST-COMBO-02 for a toolbar menu. Measured in the draft before and after
      (`.tmp/0_0_2/gap/shot_gap.py`: rows 18 and 3 apart, then 21 and 0; a menu's
      separator 12 both times). The maintainer approved the draft on 2026-11-12
    - [x] the slice 2026-11-13 (`c6aaa19c`, Opus; merge `0d225c19`; the lead's rerun of
      `studio_core`, `save_editor`, `team_widgets` and `ruleset_editor`, clippy, fmt): one
      `rows_meet` (a control's height plus the gap, the gap gone) for a menu's style and
      a list's rows; `menu_separator` (12) and `combo_separator` (7); the ruleset picker's
      list is 5 points shorter with an import, so its second separator shows and its test
      expects both. Look: Pending looks
    - [x] its SWE-2 review 2026-11-13: "S-gap r1", 3 of 5 accepted, the loop ends
      (`.tmp/duck_rulings.md`): the menu sweep asserts which entry acted, the dropdown
      test sees the list close, the changelog's "always" is gone (the lead)
  - [ ] 0.0.2-inst a Shift+click on the app's taskbar button opens a new instance, as Windows
    does for most apps (the maintainer, 2026-11-03). Found 2026-11-12 with the maintainer
    at the taskbar: Explorer starts a new instance on those clicks only through a shortcut,
    and a portable bundle has none (pinning the button made them work). The fix
    (`91f32ded`, SWE-2 and the lead; merge `6242c2ec`): the window sets its own relaunch
    command, name, icon and an application ID per copy, at every start
    (`plans/core/gui.md` "A new instance from the taskbar button", ST-WIN-05; DECISIONS
    2026-11-12 "the window tells the Windows taskbar how to start it again" and "set at
    every start, pinned or not"). The first build ended at every start with a corrupted
    heap (the `windows` crate's `PROPVARIANT` has a destructor, so the borrowed string was
    freed twice); fixed in `33a6275c`, with a Windows test that sets the values on a real
    hidden window and reads them back
    - [x] 2026-11-12: the four properties read back off the built window, and ST-WIN-06's
      clicks by the maintainer: a Shift+click and a middle click each open another window.
      Not looked at, by the maintainer's choice: two copies as two groups, and a pin made
      by an earlier version beside the new window (the changelog says nothing of old pins)
    - [ ] review rounds (tier 2). SWE-2 "S-inst r1", 2026-11-12: 3 of 4 concerns accepted,
      none of them the crash, which the look found. "S-inst r2" over the fix: 1 of 3,
      its loop ended; the checkpoint's survivor (`set_relaunch` replaced by nothing) got
      its test (`9d12e100`). Owed: one paid round (Carbon)
      - [x] Carbon's round 1, 2026-11-15 (at `2f9b9c44`): 5 of 7 concerns accepted:
        `taskbar.rs` missing from the mutation run's Windows-only list, so its mutants
        counted as missed on the VPS; a test helper that accepted a value of the wrong
        type; two hidden-window tests where one covers both; the lead's text: the GUI
        plan's taskbar section, the dependency row and the `Cargo.toml` note (`10a7b2d3`)
      - [x] the rework merged (`43a7bbdc`); round 1 had none of reach (a), so Carbon's loop
        ended
  - [x] 0.0.2-drift 2026-11-12 (`c069deab`, SWE-2 and the lead; merge `69368443`; DECISIONS
    2026-11-12 "A settings save writes only what its session changed"): today a save wrote
    the window's whole copy, so the last of two windows to save undid every value the other
    had changed; a save now writes only what its own session changed, a default not being
    a change (ST-SET-05). Not looked at in two built windows
    - [x] review rounds (tier 2). SWE-2 "S-drift r1", 2026-11-12: 1 of 3 concerns
      accepted (two windows each filling a per-version PES folder row lost one, the
      table being absent from the file while empty: a table a side lacks is merged as an
      empty one); its loop ended. The checkpoint's three survivors got two tests (the
      settings' equality; the read-back check, now one helper). Carbon "C-drift r1",
      2026-11-12: 3 of 7 concerns accepted (`5f1f8488`, SWE-2; merge
      `1e3fdf67`): a folder row written by hand under the four-digit year kept its old key
      beside a changed row and won the next start, and two tests for what none pinned;
      the paid round is done. SWE-2 "S-drift c1" over the fixes: one concern, a branch
      no test reached removed (`9d12e100`). The review rounds are done
  - [x] 0.0.2-clip Ctrl+C/Ctrl+V across instances, if possible: players, formations,
    presets, gameplans copied in one instance and pasted in another (the maintainer,
    2026-11-03). Possible, as text: Ctrl+C puts a Team TOML document of the copy on the
    system clipboard, which Ctrl+V parses (DECISIONS 2026-11-12; `plans/save_editor.md`
    "Editing UI", SE-GUI-72; SE-GUI-56 lost its "another save empties the clipboard" case)
    - [x] the slice 2026-11-12 (Opus, `ec8fe910`, merge `ef487589`): the copy as text, the
      paste through the import panel's path, the in-memory clipboard and its three session
      helpers gone; six tests for SE-GUI-72, one from PES 2019 to PES 2021. Both of the
      brief's doubts held: a stats document writes what Copy stats writes, a preset and a
      formation survive the document. Tried in two real windows 2026-11-15 at `4b9f2258`, card
      `look-clipboard`; the maintainer's look 2026-11-17: fine. Open: a paste notice with a tail ends without a full stop, unlike
      every other notice (DECISIONS 2026-11-15 "Every copy and paste notice ends with a full
      stop"); fixed by 0.0.2-namelight (`40ce82da`)
    - [x] its review rounds (tier 2): SWE-2, then Carbon. SWE-2 "S-clip
      r1" 2026-11-12 (three parts): 14 of 18 accepted (`.tmp/duck_rulings.md`): a copy
      of a Gameplan, preset or formation no longer builds the players' tables, whose odd
      byte could stop it; eight on the tests (a wrong-kind test green without the kind
      check, the source preset's index unproven, the Gameplan's expected value built by
      the code under test, lost premises, helpers copied); SE-GUI-56 and 72 reworded.
      Rejected: a note after a cross-version paste (`maintainer/proposals.html` P54). Rework 1
      2026-11-12 (`55b61937`, Opus, its session resumed; merge `af03939f`; the lead's
      rerun: 823 and 54 tests, clippy, fmt): all ten done; a Gameplan's whole record
      through the document comes back but for the bench's last nine places, which the
      format fills as the game does (the test pins that rule); a stats copy still needs
      every teammate's record. "S-clip r2" 2026-11-13 (two parts): 6 of 10 accepted, five
      asserts that could not fail and SE-GUI-72's clause for a tactics copy beside a
      player the export refuses (the plan, the lead); rework 2 (`f03528fe`, SWE-2; merge
      `8aa9c94e`). "S-clip r3": 1 of 2 accepted (all four auto flags set apart in the
      Gameplan test; the lead's edit, uncommitted until the Save editor's tests are rerun
      with the drop line's rework), which ends the SWE-2 loop; Carbon is left
      - [x] Carbon's round 1, 2026-11-15 (at `2f9b9c44`): 11 of 13 concerns accepted: a
        preset or formation press stayed the target after Down moved to a player, so Ctrl+V
        wrote a hidden preset; a row reached with no click took no paste; a team's own
        Gameplan pasted back dirtied a save holding 255 in the bench's last nine (DECISIONS
        2026-11-15 "refills the bench's last nine only when"); Ctrl+C over text selected in
        a label copied the save; three tests that could not fail (CRLF, formations off
        preset 1, a mixed notes tail); the lead's text: the help's Copy and paste section,
        the plan bullet's stale lines, two changelog lines (`4705095d`)
      - [x] the rework (SWE-2 and the lead, 2026-11-15; merge `58e2aa12`): all six; a label
        test's copy assertion narrowed to text that is not the save's own
      - [x] Carbon's round 2, 2026-11-15 (at `766ccf6c`): 10 of 11 accepted: a preset or
        formation press stays the target after the selection reaches another team without
        a list press, and comes back with its team row; five tests that cannot fail; the
        help named the whole bars where only their buttons pick (`d95ac237`, the lead)
      - [x] the rework (SWE-2 and the lead, 2026-11-15; merge `3598dc7e`): a selector's
        target ends with its team row; the Edit menu's Copy and Paste
      - [x] Carbon's round 3, 2026-11-15 (at `3598dc7e`): 6 of 9 accepted: three tests that
        pass on wrong code; a hover gate egui already applies; the help's Copy and paste
        section, SE-GUI-56 and the changelog's Edit menu line (the lead's)
      - [x] the rework (SWE-2, 2026-11-15; `9307edd8`, merge `0434295b`): the Edit menu's
        hovers tested for every target, the notices' counts exact from the source save
      - [x] Carbon's round 4 (at `0434295b`): 3 of 3 accepted, which ends the loop: a hover
        test that passed on wrong code, the lapse test stopping short of 701, two comments
        naming the wrong row; the rework (SWE-2, `d84dda67`, merge `917a202a`)
  - [x] 0.0.2-statlbl 2026-11-12 (`e4bf0a92`, SWE-2 and the lead; merge `13fa465c`; plan
    and draft `43770f93`): the name of an ability stat the save's version does not store is
    dimmed with its `-`, in the colour of a version-gated card's name (SE-GUI-70). Not
    looked at in the built window (look: Pending looks). The same rule for the editor's
    other gated fields is `maintainer/proposals.html` P53, unruled
    - [x] review round (tier 3), SWE-2 "S-statlbl r1", 2026-11-12: 1 of 3 concerns
      accepted (the dimmed name had no `PES NN+` tooltip, where its `-` and a gated
      card's name have one)
  - [x] 0.0.2-log 2026-11-12 (`d61750cb`, Opus; tier 2, reviewed with 0.0.2-upgall
    below): `--log-file <FILE>` appends a run's whole output to the file and leaves Info
    findings out of stdout, and the two launchers pass it (ST-CLI-04); the upgrade launcher
    run under a pseudo-console showed the warnings, a refusal's `error:` line and the
    progress line in the window and every line in the log. The bundle's readme names the
    logs, and the organizer's backup of `download/` and the list's upgrade as steps
  - [x] 0.0.2-upgall 2026-11-12 (`88c91de1`, Opus; tier 2): `upgrade`
    takes several exports or `--all <DIR>` in one run (8.22i), each as if alone under its
    naming line, with the tally at the end, and `quick_upgrade.bat` starts the exe once, so
    the progress line counts every export (EU-CLI-11, 13 to 15). The launcher run under a
    pseudo-console over two exports, one refused, drew `[ 0/2 exports ]` first and showed
    both blocks, the `error:` line and `Upgraded: 1. Not upgraded: 1.`
    - [x] review round 1 of both (SWE-2), 2026-11-12 (`be23aec1`, Opus): 12 of 19 concerns
      accepted: a crash's text in the log file, the launcher's last report kept when no run
      follows, the progress line's parts counter per export, and the cases of a run of
      several exports no test reached
    - [x] review round 2 of both (SWE-2), 2026-11-12: 3 concerns, two comments corrected,
      which ends SWE-2's loop
    - [x] the paid round (Carbon), 2026-11-12 (`27f7b59a`, SWE-2): 8 of 10 concerns
      accepted: `--all` skips an interrupted run's `.upgrading` folder, `--savefile` with
      `--loose` refuses the whole run before reading anything, the usage lines, the
      readme's Linux note, and three test gaps (the launchers' quoting, `--legacy-ids`, a
      referee export beside a team export)
    - [x] the SWE-2 round over `27f7b59a`, 2026-11-12: 2 of 4 concerns accepted, both
      wording (the help and the plan name the `.upgrading` folder `--all` passes over),
      which ends SWE-2's loop
    - [x] Carbon's round 2, 2026-11-15 (at `2f9b9c44`): 5 of 7 concerns accepted: no test
      ran a clean export after an unclean one; the panic hook's call of the hook before it
      untested; `FIRST_RUN`'s doc named removed commands; the lead's text: the help's
      "Team compiler" and its several-exports paragraph split in three (`4705095d`)
    - [x] the rework (SWE-2, 2026-11-15; merge `218ecaaf`): the three test and doc findings
    - [x] Carbon's round 3, 2026-11-15 (at `766ccf6c`): 6 of 7 accepted: the printers'
      mutant excluded under its old function's name; `upgrade --help` placing the output in
      the input's folder (it goes beside it); the plan's list of what the log file takes;
      seven `RunEvents` methods wider than their callers; two docs (`fb177097`, the lead).
      Rejected: `quick_upgrade.bat` passing the exe's exit code through
    - [ ] the `--help` text and the methods' visibility (the lead, needs a build), then
      Carbon's round 4
  - [x] 0.0.2-cli 2026-11-12 (`6fed2e3e`, Opus; tier 1, its review loops owed): with
    `check_for_updates` on, a tool command starts the check on its own thread and, when a
    newer release has answered by the run's end, prints one line last on stderr and in the
    log file; it never waits and stores nothing (ST-CLI-05,
    `plans/core/distribution.md` "The command line's notice"). Not run: a command against a
    served releases list printing a real offer (the tests cover the line in process)
  - [x] 0.0.2-se19 dropped 2026-11-11 (the maintainer: the tester's save is lost; DECISIONS
    "0.0.2 holds every open 0.0.2 step"): the writer was found sound, and what the edit
    flags gate in game is in `spec/pes_savefile/model.md` "What the player's edit flags gate
    in game"
  - [ ] 0.0.2-poc the first work after 0.0.1's cut (the maintainer, 2026-10-31): 8.36c and
    8.36d, then a proof-of-concept exe for VGL's organizer that
    upgrades and compiles a couple of example teams on PES 2019. 8.36c3 (the `<id>.copy`
    link) waits until after it: no player of the two example teams uses a copy id (the
    maintainer, 2026-11-04). A bundle is fine (the maintainer, 2026-11-04), else the exe
    with the three commands; the gates and the in-game check suffice, 8.36c4's Astra
    review and `mutants-diff` running beside it, their fixes in 0.0.2 → verify: the example teams'
    players show their compiled faces, boots and gloves in Edit mode on PES 2019. The example
    teams (the maintainer): /dbg/ and /4ccg/, the old exports `DBG Aesthetics Export VGL26`
    and `4CCG Aesthetics Export VGL26` in `C:/Data/4cc/Lab/VGL26/`, upgraded
    against the save `C:/Data/4cc/Saves/EDIT00000000_VGL26`. Dry runs 2026-11-05 over the 53
    VGL26 exports (`.tmp/0_0_2/poc/dry_run.py`, `all_run.py`, `tally.py`): their refusals
    and Errors answered by poc1 to poc7
    - [x] 0.0.2-poc1 a custom ID no folder claims becomes the hide ID, with a Warning
      (EU-SET-14, `custom_model_hidden`)
    - [x] 0.0.2-poc2 a texture nobody supplies is a Warning on Fox too (TC-MOD-100,
      TC-TEX-21)
    - [x] 0.0.2-poc3 the 53 exports again with poc1 and poc2's binary: every Error left
      diagnosed (2026-11-05, `4e69334c`, `.tmp/0_0_2/poc/all/`)
    - [x] 0.0.2-poc4 the maintainer's two cleanups: a worn ID's folder found by name, a `.dds`
      beside a `.ftex` resolved (EU-PLY-23, EU-MDL-23)
    - [x] 0.0.2-poc6 what no name pairs: `wearer_by_number`, `wearer_by_order`,
      `custom_model_unpaired` (EU-PLY-24 to 26)
    - [x] 0.0.2-poc5 a model hidden by distance dropped (`model_far_dropped`; EU-MDL-24)
    - [x] 0.0.2-poc7 anti-blur duplicates dropped (EU-MDL-25; TC-MOD-34 withdrawn, restated as
      TC-MOD-101): the 53 VGL26 exports' compile ends clean (2026-11-06)
    - [ ] 0.0.2-poc8 the bundle for VGL's organizer (found 2026-11-08: no step had it): the
      two launchers, `exports/` and `old_exports/` back in `release.py`'s `LAUNCHERS` and
      `EMPTY_FOLDERS`, the readme's VGL section and "pick your PES" step
      restored from `3d7fdc33^`'s `resources/bundle/README.txt`, the section gaining the
      plan's order (`core/development_plan.md` "The /vg/ League 27 (VGL)": the patch the last
      write, the official save's AATF check); `release_test.py`'s
      `test_the_bundle_ships_no_launcher_and_no_export_folder` rewritten, not deleted → verify:
      the staged bundle's file list; the PES 2019 rehearsal (/dbg/ and /4ccg/ through the
      launchers from the extracted bundle, checked in Edit mode). Landed:
      - main's half 2026-11-08 (SWE-2, brief `.tmp/0_0_2/poc/brief_poc8.md`): the two tuples,
        the readme, the CHANGELOG's `### New`; the version 0.0.2-poc.1 (DECISIONS "the PoC
        bundle is 0.0.2-poc.1"); Astra A-poc8 r1 and r2 over the readme, the loop ended
      - branch `vgl27` (named `poc/0.0.2-poc.1` until 2026-11-11), built 2026-11-08 and
        rebuilt with each later fix (earlier builds in `target/dist_prev_*`)
      - the rehearsal on PES 2019 2026-11-09 (`.tmp/0_0_2/ingame/rehearsal/findings.md`): both
        teams upgrade, the compile deploys and patches the save, 8 players show their own
        faces, bodies, boots and gloves
      - the official save's write path (A-savepath r1 and r2, S-savepath r1 and r2, loops
        ended): a written name sets the edited-player flag (SE-PAT-13); an in-place
        `apply-patch` that applies no player writes nothing (SE-PAT-12)
      - the VGL27 Friendlies through the bundle 2026-11-09 (39 teams): 37 upgrade and compile
        clean for PES 2019; /wuwa/'s note colour `##8C96C5` refuses its upgrade, which stays
        (DECISIONS 2026-11-09)
      Handed over: 0.0.2-poc.5, by the maintainer; the organizer has not tried it yet. The Linux tarball
      carries no `.bat` file (`release.py` `PLATFORMS`: Linux's launchers are none)
  - [x] 0.0.2-poc9 a legacy kit folder's file names lose their Studio tokens (DECISIONS
    2026-11-09, two entries; EU-MDL-28): an author's token written `kit_tex` (`975e68a7`)
  - [x] 0.0.2-poc10 the embedded teams list carries VGL27's allocation (DECISIONS "the
    embedded list carries VGL27's allocation"; done 2026-11-09, `6268280c`): 781–834
  - [x] 0.0.2-poc11 the upgrader writes no team ID and needs no teams list (DECISIONS
    2026-11-09 "the Studio format names no team ID"; done 2026-11-09): the save's team found
    by name, `gk_kitN` for a keeper's `u0XXXgN`
  - [ ] 0.0.2-poc12 (tier 1) no ambiguous Common spelling (the maintainer, 2026-11-09, ruling on
    poc11's *lead* choices after Astra's round). Rulings in DECISIONS 2026-11-09 "no ambiguous
    Common spelling" and "the kit stand-in is written `team_kit`", 2026-11-11 for slice 4: the
    team's Common is `team_common_folder/…` (long form
    `model/character/uniform/common/team_folder/…`), a game file below the uniform Common
    `common_folder/<name>`, a reference starting `common/` refused; an FMDL texture directory
    takes the same two short forms; the kit stand-in is `team_kit`/`team_gk_kit` (the
    compiler writes `dummy_kit…` in every output); every legacy spelling is refused by the
    compiler and respelled only by the upgrader
    - [x] spec: DECISIONS 2026-11-09 "no ambiguous Common spelling"; TC-XML-21, 22,
      TC-MOD-103, EU-MDL-34 to 39, EU-REF-14, EU-MF-10
    - [x] fixtures (lead, `cf80cf48`): the studio tracer's four FMDLs, `deep/boots_far.fmdl`
      and the three archives built from them
    - [x] the code (slice 1): `team_common_folder/`, SWE-2, `.tmp/0_0_2/poc12/brief.md` and
      `rework1.md`
    - [x] slice 2: the FMDL short forms (TC-MOD-104, 105, EU-MDL-40), `brief2.md`
    - [x] slice 3: `team_kit` (TC-MOD-106, TC-CMN-36, EU-MDL-41, 42), `brief3.md`, `brief3b.md`
    - [x] slice 3b: FoxDen's `02_kitswappers.lua` gets back the `dummy_kit`/`dummy_gk_kit`
      swapper; done, uncommitted in FoxDen's repository (the official scripts are the
      maintainer's)
    - [x] the PoC rebuilt 2026-11-09 (`f815f1b1`, again `19a7fb3f`): 36 of 39 Friendlies
      upgrade, no old spelling left
    - [x] slice 4: the legacy kit token refused in an export's file names and paths, naming
      the upgrader (`kit_token_old`; TC-STR-22, TC-MOD-107; `9382b068`, merged `abe8e0ff`)
      - [ ] review rounds. Astra A-poc12s4: 2 accepted (names below a player or shared folder
        and in `Common/` only; a bare `.mtl` name is the folder's own; DECISIONS 2026-11-11),
        its loop ended. SWE-2 S-poc12s4 r1 (two parts): part a 2 nits (`aa38922c`); part b 5
        accepted, the check wider than the upgrader's respelling (`Faces/`, collars,
        game-supplied names; the catalog row's text): rework by Opus sidekick `sk_kto`
        (`4cded58e`, merge `46fc9622`; DECISIONS 2026-11-11), then SWE-2 r2: 0 accepted (the
        paths it raised are in none of the 15,199 FMDLs under `C:/Data/4cc`; a doc nit
        fixed), its loop ended; Carbon owed
        - [x] Carbon's round 1, 2026-11-15 (at `2f9b9c44`): 10 of 11 concerns accepted: a
          player, Boots or Gloves FMDL naming the token in a game-folder path, which the
          upgrader rewrites to `./` (`/Assets/.../face/real/00000/sourceimages/u0XXXp1.dds`,
          as Blender's exporter writes it), compiled with no finding; five checks no test
          could fail; the catalog row's text, the help and two spec passages (`64d1981f`,
          the rest with the rework)
        - [x] the rework merged (`b03c533d`); round 1 had none of reach (a), so Carbon's
          loop ended
  - [ ] 0.0.2-poc13 VGL27 keeps the managers' boots and gloves IDs (the maintainer,
    2026-11-09, after the PoC): managers keep the old format and Red for their own tests, so
    an official DLC whose boots/gloves IDs differ from what they sent (Studio's slot-based
    40-per-team block, 3745 for team 792's slot 05, against the old exports' own `k`/`g`
    IDs in a 25-per-team block) breaks their test CPKs against the official save.
    - [x] poc13a spec (2026-11-10): the legacy IDs' plan section and the block table
      `legacy_boots_blocks.txt`, on the branch `vgl27` alone since 2026-11-22 (DECISIONS
      2026-11-22 "VGL27's legacy mode leaves main")
    - [x] poc13b the upgrader's `legacy_ids.toml` (2026-11-10, SWE-2, landed first time)
    - [x] poc13a2 re-spec (2026-11-10 evening; DECISIONS 2026-11-10): legacy mode is the
      upgrader's `--legacy-ids`, the compiler reads the file's presence
    - [x] poc13c the upgrader's `--legacy-ids` (2026-11-10, SWE-2, landed first time;
      EU-SET-15 to 17)
    - [x] poc13d the compiler's legacy packing (2026-11-10, SWE-2, landed first time;
      TC-LEG-01/02)
    - [x] poc13e the block check, brought into the PoC (2026-11-11, SWE-2, landed first time;
      TC-LEG-03/04): /utg/'s wrong IDs refused (DECISIONS 2026-11-11)
    - [x] poc13f a legacy merge that clashes keeps the folder shared, packed under its
      old ID (2026-11-11, SWE-2, landed first time; TC-LEG-05, EU-SET-17)
    - [x] poc13g an out-of-block old ID reported once, on the folder that would pack it
      (2026-11-11, SWE-2, landed first time; DECISIONS 2026-11-11)
    - [ ] the maintainer's asks for the PoC if they fit (2026-11-10), each checked first.
      Bats keep their `quick_` names (the maintainer, 11-10)
      - [x] why /nepgen/'s `SWITCH EXCLUSIVE` became a shared gloves folder with one wearer
        (answered 2026-11-11, no change: its textures differ from the boots folder's)
      - [x] poc14 (SWE-2, two reworks): `dpfilelist_outdated` and `cpk_name_unlisted` Fatal
        at the preflight (TC-DEP-05, 08); the bundle's commented `data/settings.toml`
      - [x] 0.0.2-poc.3 built (2026-11-11; branch `1a731a94`, archive SHA-256 `991cc670…`,
        the exe `v0.0.2-poc.3`) in `4cc-studio-temp/poc/target/dist`. Look: Pending looks
    - [x] the embedded `dpfl`'s `4cc_99_test` renamed `4cc_90_test` on the PoC branch alone
      (`301c8b46`); main keeps `4cc_99_test`
    - [x] 0.0.2-poc.2 rebuilt with poc13e/f (2026-11-11; branch `9eafbfad`, archive SHA-256
      `1cabaa13…`): the Friendlies with `--legacy-ids` upgrade 36 of 39 and compile with only
      the expected Errors. Not checked in game (look: Pending looks)
  - [x] 0.0.2-ux1 (tier 2) the CLI's progress line and finding times (the maintainer, 2026-11-10;
    DECISIONS 2026-11-10): ux1a and ux1b below
    - [x] ux1a the upgrader's line (EU-CLI-12): `PipelineEvent::ExportNamed` names the output
      half, the parts are the files written into staging (no test for a failed read's part)
    - [x] ux1b `--timestamps` on `check` and `compile` (TC-CLI-13): the time on the event
      envelope (`PipelineEventEnvelope.made_at`)
    - [x] review rounds over ux1a + ux1b (`03ce3612..2ea6e27c`): Astra's round is tier 2's one
      paid round, so no Carbon loop. Astra A-ux1: 2 accepted (a test of `write_batches`' finish
      times, the plan's `duplicate_path` timing), its loop ended. SWE-2 S-ux1 r1 (three
      parts): 1 accepted (TC-CLI-13's `check` test asserts no stderr), its loop ended
  - [x] 0.0.2-chk 2026-11-12 (the lead): the library's compile is measured against our own
    checkpoints, not Red (DECISIONS 2026-11-12; `spec/team_compiler/testing.md`, the
    library paragraphs): `scripts/library_parity/checkpoint.py` (`write`, `diff`), the
    README's rerun order, RUNBOOK and AGENTS. Checkpoint `p11` written with the p11 run's
    frozen binary (`.tmp/library_parity/checkpoints/p11/`): 419 of its 469 inputs are
    still on disk (45 of the 50 gone were under `C:/Data/4cc/Lab/VGL26`), every CPK hashing as in
    that run. `library.py`, split for it, gives the p11 run's rows again on two inputs,
    against a clean clone of Red (the maintainer's checkout has an uncommitted
    `teams_list.txt`, which the script refuses). No review round (scripts and docs)
  - [x] 0.0.2-acc withdrawn scenarios out of the way (the maintainer, 2026-11-09): whole
    withdrawn areas in `docs/spec/withdrawn_ids.md`, runs of withdrawn IDs as range lines
  - [x] 0.0.2a JPEG XL sources (DECISIONS 2026-10-30; TC-TEX-22): `jxl` =0.8.0 decoded
    directly, built without AVX-512 (DECISIONS 2026-11-11); tier 1: Carbon, then Astra
  - [x] 0.0.2-ids the team's manager id (+4, u32) and home stadium id (+10, u16) on PES 17
    and 18 (DECISIONS 2026-11-11; `8cb837d6`, merge `395e734c`); tier 1: Carbon, then
    Astra
  - [x] 0.0.2-st15 the home stadium id on PES 2015 and 2016 (DECISIONS 2026-11-11;
    `9975abe6`, merge `9ae8e390`): the u16 at +6, shown live. Not checked in game (look:
    Pending looks); tier 1: Carbon, then Astra
  - [ ] 0.0.2b (tier 1) no BC7 encoding until the GPU encoder (`spec/libs/dds_convert.md` "Target codecs
    and CPU encoder"; DECISIONS 2026-10-30): PES 19–21 encode as
    PES 15–18, BC7 sources kept; the Compiler's tests and goldens that expect BC7
    follow, and the library parity against Red re-run (Red ships BC3). Pulled into the
    0.0.2-poc bundle (the maintainer, 2026-11-06). With it, uncompressed sources stay
    uncompressed (`spec/libs/dds_convert.md` "In-process DDS conversion (replaces texconv)";
    DECISIONS 2026-11-06), a pre-Fox main kit texture excepted. → verify also: the
    maintainer's PES 21 test compile (`.tmp/release_0_0_1/0_0_1/manual_run2`, about 1:30
    with BC7) re-timed; its 45 DXT5 sources pass through either way, and its three
    uncompressed 4096 × 4096 sources (`kit.dds` twice, one `shirt.dds`) were the only BC7
    encodes and now encode nothing, so a run still slow after this step sends the profiling
    of 0.0.2d elsewhere. Landed:
    - the code (SWE-2, the commit "no BC7 encoding; uncompressed sources stay uncompressed"),
      the spec moved (the deferred GPU part stays in the plan); `just gpu-tests` not run
    - re-timed 2026-11-06 (`.tmp/0_0_2/poc/retime.py`): 0.8 s wall where BC7 took about 1:30
    - in game on PES 2019 2026-11-07 (`.tmp/0_0_2/ingame/poc/findings.md`, `s42`-`s44`): the
      4096 `kit.dds` kept uncompressed loads and Edit mode draws the shirt; the seasonal
      logos not seen, which Edit mode never places (look: Pending looks)
    - Astra A-nobc7 r1 (3 accepted) and its rework 2026-11-08; SWE-2 S-nobc7 r1 2026-11-11
      (3 accepted, among them `texture_not_div4` skipping a pre-Fox texture written
      uncompressed, DECISIONS 2026-11-11; look: Pending looks), its loop ended
    - [x] Carbon's round, 2026-11-23 (at `ca8559a1`, rulings "C-nobc7 r1"): 6 accepted, none of
      reach (a), which ends Carbon's loop. The lead's texts `f05659ef`; the tests for each
      place that keeps or encodes an uncompressed source, SWE-2 slice N1, merged `6b5ace55`.
      SWE-2's round over both (rulings "S-n1 r1"): 3 accepted, none of reach (a); the lead
      widened the scenarios and the BC7 texts, and the two test helpers now check that the
      texture each writes is stored uncompressed (`64a79e9c`). Astra's round 2 through Devin
      (rulings "A-nobc7 r2"): 2 accepted, none of reach (a), a help line and a test comment
      (`65d2ee4d`); SWE-2's round over them ("S-a2 r1"): 2 accepted, none of reach (a)
      (`2544b106`). The review loops are done
    - the same run's PoC look passed: /dbg/ and /4ccg/ upgrade and compile clean (about 2 s
      for both), and the 8 players checked show their own faces and bodies
  - [x] 0.0.2c the illegal formation check (`plans/aatf_rules.md` "The illegal formation
    check (0.0.2)"; DECISIONS 2026-10-31, P46 accepted; 2026-11-12 "An illegal formation is
    one only a tool outside PES can make"): what the stock screens can produce, read from
    PES 2017's exe, then looked at in game on `PES2017.exe.old5`; then the plan's rule, its
    Acceptance, and the slice
    - [x] the first reading, 2026-11-12 (Opus, `.tmp/0_0_2/formation/research/report.md`):
      the Whack Patch's 25 runs read; no area refusal among them, a match-time rewrite of
      the line counts and the drag's re-pick switched off
    - [x] the second reading, 2026-11-12 (Opus, `.tmp/0_0_2/formation/research/report2.md`):
      the drop's offers per spot (two area tables, PES 2017's first equal to
      `team_widgets`'), the menu, the swap, the per-set positions; the test's four clauses,
      no line counts; a formation breaking each clause
    - [x] six defenders and seven midfielders tried in the stock screen (the maintainer,
      2026-11-12, two captures): the drop is refused with a no-entry sign, so the screens
      do count lines and the second reading's "no line count" is wrong
    - [x] the limits, from the maintainer's looks, 2026-11-12: forwards 1 to 5, midfielders
      1 to 6, defenders 2 to 5, each a refused drop; where the exe decides it is unread
      and not needed for the check
    - [x] the sixth clause (the maintainer, 2026-11-12: overlapping starters are refused,
      at the distance the editor's pitch already swaps at): two outfield starters no
      more than 8 units apart across and 2 up or down, each inside the other's marker box
    - [x] the game's own formations and the cup saves under the six clauses, 2026-11-12
      (Opus, `.tmp/0_0_2/formation/census/report.md`, the tally rerun by the lead): none
      of PES 2017's 3,198 own formations and none of the cup's in ten PES 2017 and ten
      PES 2021 saves breaks a clause; PES 2016 and 2019 saves hold 35 distinct breaks
      under PES 2017's tables; the preset list was not found in the game's data, and
      clause 6's box catches nothing anywhere (the plan's "Still open")
    - [x] each clause looked at in game under the stock exe, 2026-11-12 (SWE-2, pad and
      OBS; `.tmp/0_0_2/formation/ingame/findings.md`, the kept saves under the clauses in
      `.tmp/0_0_2/formation/census/looks/`): the game draws and saves a broken formation
      as stored (the keeper drawn on his own spot); a drop repairs the starter it moves;
      the fifteen presets pass; the screen swaps up to 11 across and 3 or 4 up or down,
      and clause 6 keeps 8 by 2 because the game's own formations hold pairs inside that
      box. The Whack Patch's exe is back as `E:/PES2017/PES2017.exe`. Ten other teams of
      the live cup save break clause 2 or 3 (made under the patch or a tool: what the
      warning is for)
    - [x] the plan's rest and its Acceptance 2026-11-12 (the lead; DECISIONS 2026-11-12):
      one warning per fielded set, the rule in `pes_savefile`, the fixtures, SE-AATF-09,
      SE-GUI-76
    - [x] the same looks on PES 2019, 2026-11-13 (SWE-2, pad and OBS;
      `.tmp/0_0_2/formation/ingame19/findings.md`, the kept saves under the clauses in
      `.tmp/0_0_2/formation/census/looks19/`): the same rule on every point, the fifteen
      presets equal to PES 2017's spot for spot, every set the game wrote passing under
      PES 2017's tables; the check runs on PES 2017 and PES 2019 (DECISIONS 2026-11-13).
      PES 2019 writes all three sets of a preset alike. The cup's PES 2019 save the looks
      ran on holds six other teams breaking clause 3 or 4 (/b/, /co/, /d/, /tg/, /toy/,
      /x/: a tool's work, what the warning is for). The live PES 2019 save (VGL27's
      Friendlies save, another profile's) is refused by this install with "Create Edit
      Data?", so the looks ran on a test save built on the maintainer's own save of
      2026-10-13; the live save is back in place, byte for byte
    - [x] slice a 2026-11-13 (`eb6b96e0`, Opus and the lead; merge `8c528582`; the lead's
      rerun of `pes_savefile`, `team_widgets` and `save_editor`, clippy, fmt):
      `Formation::illegal_breaks` and its `FormationBreak`s in `pes_savefile`'s
      `model/formation_rule.rs`, the area tables and the wide positions moved there from
      `team_widgets`' drop rule; tested on what PES 2017 and PES 2019 made themselves
      (938 and 506 formations in `tests/fixtures/game_formations_pes17.txt` and
      `_pes19.txt`, built by `.tmp/0_0_2/formation/fixture/make_fixture*.py`) and one
      formation per clause; `team_widgets` pins the rule's reaches to its marker box and
      its row reading to what a drop offers
    - [x] a's SWE-2 review 2026-11-13 (S-rule r1, 4 of 5 accepted, the loop ends;
      `dd28365b`)
    - [x] slice b 2026-11-13 (Opus, the same session, and the lead; merge `299551ad`; the
      lead's rerun of `pes_savefile`, `aatf`, `team_widgets` and `save_editor`, clippy,
      fmt; SE-AATF-09, SE-GUI-76): the host raises one `IllegalFormation` warning per
      fielded set that breaks the rule, on PES 2017 and PES 2019 saves, unless the
      ruleset allows illegal formations; it carries the eleven starters' names and
      positions for its words; it lands after the players' rows and before the script's
      own team rows. A team viewed from a Team TOML with no tactics would hold all-zero
      formations and warn: nothing reaches that today, the Team creator's check will
      (to settle in its phase). Look: checked 2026-11-15 at `4b9f2258`, card
      `look-illegal-formation`; the maintainer's look 2026-11-17: fine
    - [x] b's SWE-2 review 2026-11-13 (S-warn r1, 1 of 2 accepted, the loop ends): the
      pitch's own range let a member make what the check warns about, which is 0.0.2-range
  - [x] 0.0.2-range with illegal formations off, the pitch holds an outfielder in the game's
    movable range, x 5 to 99 and y 4 to 47, by a drop's snap and the popover's X and Y, on
    every version (S-warn-1; DECISIONS 2026-11-13; `plans/save_editor.md` "Editing UI", the
    popover's paragraph on X and Y; today `snap` and the steppers of
    `team_widgets/src/pitch/` use `GRID_X` 104 and `GRID_Y` 48)
    - [x] its Acceptance clauses 2026-11-13 (the lead): SE-GUI-42's last six, the range's
      four ends by a held marker and by the steppers, and a stored x 2 stepping in to 5
    - [x] the slice 2026-11-13 (`cb1ee954`, SWE-2 and the lead's rerun: `team_widgets`
      258, `save_editor` 852 + 57, clippy, fmt): `outfield_range` gives a drop's snap and
      the popover's steppers their ends; the same commit moves PES 2015, PES 2016 and
      PES 2018 to PES 2017's pitch (the maintainer's pictures, 2026-11-13, measured:
      `plans/save_editor.md`, "Each version draws its own screen's pitch"). Nothing
      else of the pitch treats PES 2017's screen apart: the keeper's marker and the
      spot mapping are one set of constants for every version
    - [x] its SWE-2 review 2026-11-13 (S-range r1, 3 of 6 accepted, the loop ends with
      its rework, SWE-2 and the lead's rerun: `pes_savefile` 419, `team_widgets` 260,
      `save_editor` 852 + 57, clippy, fmt): the arrow keys on the popover's focused X or
      Y stepped past the range's ends, so the number is clamped when it reports a
      change; the range is read from `formation_rule` (`MOVABLE_X`, `MOVABLE_Y`, now
      `pub`) instead of written twice; two tests' position asserted. Issues: the keys'
      step itself
  - [x] 0.0.2d 2026-11-13 (the lead): a compile timed against Red, after 0.0.2b. Ours
    6.16 s (5.90 to 6.34), Red 57.06 s (52.74 to 58.14): three runs each, wall time, the
    median, PES 2019, no deploy, ten of the VGL27 Friendlies' Day 1 exports (three more
    name teams no list holds), ours the release exe at `cb1ee954` on the exports our
    upgrader wrote from them (18.5 s once for the thirteen, archives unpacked), Red at
    `1a48fa5` in a copy of its folder on the old ones, its run unpacking the archives
    too. No stage of ours costs what Red's whole run does, so no step follows and no
    sampling profiler was run. Not the export the maintainer timed at 1:30 (`egg Midcup
    Big`): it predates today's format and no longer passes the checks; that run encoded
    BC7, which 0.0.2b took out. Not looked into: our CPK is 946 MB to Red's 1,286 MB.
    Scripts and logs: `.tmp/0_0_2/prof/`, `4cc-studio-temp/prof/`
- [ ] with Release 0.2.0 (the maintainer, 2026-11-12): the response-time gate at the
  checkpoint (`plans/core/development_plan.md` "Release 0.2.0"); its first step only
  reports the medians
- [ ] later, once enough tools ship for it to filter anything (the maintainer's idea,
  2026-11-12, to design then, not ruled): roles in the sidebar. Above the tool list, with
  a separator between, a member's role (Manager, Aesthetics maker, Aesthetics helper,
  Tactician, Organizer, Streamer), and the list then shows only that role's tools. Open
  points from the first discussion: a filtered list moves the icons when the role changes,
  against the rule that they never move (0.0.2-sel), so a role is a setting made once more
  than a switch; several roles at once, and an "every tool" choice as the default so no
  tool is hidden from a member who never picks; six rows above the list against one
  control; the role to tools table itself, the Save editor falling in nearly every role.
  The maintainer's additions: Manager is Aesthetics maker and Tactician together, those
  two being for members of a team's council who hold one part of it; and the program asks
  "So who are you?" at startup, the roles on a ring with an icon each and a "Don't ask
  again" checkbox in a corner (the lead's points on it: ask on the first start and while
  no role is set rather than at every start, the same choice reachable later from the
  sidebar or Settings, the ring usable from the keyboard). Settled with the maintainer:
  the sidebar shows the current role as one compact control, not the six listed, the
  ring being where a role is chosen
- [ ] after 0.2.0 (the maintainer, 2026-11-12: "no real need for it, but it's a nice to
  have"): a shorter text for the Save editor's Ctrl+C. The copy is a Team TOML document
  (0.0.2-clip), and the writer lists every unset key as a commented line: about 305 lines
  for a player's stats, 143 for a preset. A form without the unset keys needs a writer
  option in `pes_savefile`'s `team_toml`
- [ ] for Release 0.1.0 (the maintainer, 2026-11-17; DECISIONS 2026-11-17 "a tactical patch
  from the exports' Team TOMLs"): the Compiler's tactical patch, a team export's Team TOML's
  tactics gathered into one patch in the download folder and applied to the save only on
  the organizer's yes to the run's closing question (`plans/team_compiler/pipeline.md` "6.
  Post-processing"), from `team.toml` at the export's root (DECISIONS 2026-11-17); its
  scenarios first
  - [ ] before it: the maintainer's "universal export" (the Team TOML and music in one
    export), remembered as devised but written nowhere; to be talked through with the
    maintainer when its time comes (2026-11-17)
- [ ] after 0.0.2 (the maintainer, 2026-11-17, card `look-tactics-gaps`): 0.1-pitchring,
  the Tactics pitch's turf reaches its border's inner curve, as `plans/save_editor.md` (the
  Tactics screen's item 2) says: 0.0.2-pitchround (`1f958e81`) rounds the turf by the
  border's whole radius, so grey shows between the border's inner curve and the turf in
  each corner (the maintainer's capture on the 4K screen, at 200%)
  -> verify: a test reads the painted turf's corner radius against the border's inner one,
  red first; the corners looked at at 100% and 200%
- [ ] after 0.0.2 (the maintainer, 2026-11-15; DECISIONS 2026-11-15 "the File menu: the
  picker and the writes first"): the Ruleset editor's File menu takes the Save editor's
  layout: its picker, Save, Save as… with Ctrl+Shift+S, Recent files, Close file with
  Ctrl+W, Exit with Ctrl+Q (`plans/ruleset_editor.md` to edit first)
- [ ] after 0.0.2 (the maintainer, 2026-11-15; DECISIONS 2026-11-15 "the Ruleset editor's
  findings read as text in its window"): the window's refused-open and refused-save lists show
  each finding as `Line 12, column 39: …` or `tiers[1]: …` in place of `key=value` fields,
  dropping the parser's own trailing "(line 12, position 39)"; the command line keeps every
  tool's shared form (`studio_core` `events.rs` `message_line`). `ruleset_editor`
  `messages.rs` `finding_text`, pinned by `view/tests/start.rs`. Examples and the options
  weighed: `.tmp/text_choices/rules_lines.html`
- [ ] after 0.0.2 (the maintainer, 2026-11-15; DECISIONS 2026-11-15 "Each screen is measured
  against its draft in the tests (P57)"): each built screen compared with its draft by a
  headless test (`plans/core/gui.md` "Prototypes before implementation", the "Measured against
  the draft" bullet): the recording script (from `draft_compare.py`'s draft side) and its
  fixtures with the draft's hash; `draft_compare.py`'s matching rules ported to Rust test
  support; one test per screen of its list. Then, if still worth it: `egui_kittest`
  (dev-dependency) for the colour and weight checks
- [ ] after 0.0.2 (the maintainer, 2026-11-15; DECISIONS 2026-11-15 "a shirt number past
  the target's range is clamped"): a team file import writes a shirt number past the target's
  range as the range's end, listed among its notes, in place of refusing the file
  (`plans/save_editor.md`, the paragraph starting "A shirt number past the target's range")
  -> verify: `C:/Data/4cc/Lab/VGL27 Qualies/ted/gbfg_vgl27.ted` into a PES 2017 save
  imports, player 03's number 255 and the note naming it (it refuses today with
  `players.03.number: 420 is out of range (0 to 255)`)
- [ ] after 0.0.2 (the maintainer, 2026-11-15): docstrings: every doc comment of the codebase reread under the Shakespeare rule's
  docstring part (`~/.claude/rules/shakespeare.md`: readings 1 and 2, reading 3 as
  reference text). The maintainer decides whether SWE-2 or Opus runs it. The trial,
  2026-11-13: SWE-2 on `fpc`
  and `vtree` (branch `text/docstrings`, worktree `4cc-studio-temp/docstrings`, one commit,
  not merged; brief and report in its `.tmp/docstrings/`): 104 blocks read, 15 edited,
  mostly chained sentences split, three pointers given their file, one stale claim found
  and reported (`StripStyle` "as booleans"); one slip in fifteen (a source pointer moved
  onto a sentence it does not support), corrected by the lead. For the full pass: one
  crate per brief, the trial's brief as the model, plus "a pointer or citation moves only
  with the sentence it documents"; the lead reads each edit against the code
- [ ] after 0.0.2 (from 0.0.1's la, b-3): `aatf`'s nine fixture modules' repeated tests
  become helpers (`.tmp/topics/lib_audit/rulings.md`)
- [ ] after 0.0.2 (from 0.0.1's cv13; the maintainer, 2026-11-01: "It can wait"): the
  window's first frame under load. The target is under one second, two at most: met at rest
  (528 ms as is, 419 with DX12 alone), missed under CPU contention (2.5 s, 1.8 s with one
  backend), which is not accepted where disk contention is
  (`.tmp/release_0_0_1/0_0_1cv/cv13_times.txt`, `.tmp/0_0_2/cv13/cv13_load.txt`). Next, on a
  quiet machine: more rounds of `cv13_load.py cpu` and `cv13_who.py` to find the process the
  start waits on (a WPR trace needs an elevated prompt, so it is the maintainer's); then
  whether eframe's wgpu runs on one backend (`studio_core` `shell/window_size/mod.rs`
  `native_options`; about 130 ms at rest)
- [ ] after 0.0.2 (from 0.0.1's cv23b and cv23c): Multi-team AATF's cost. A listing frame
  takes 23 ms, 21.3 of them in `violations_list`, because every team's rows word their
  sentences and lay out three texts each frame, in view or not: lay out only the rows in
  view, or keep the worded rows per check key (`.tmp/0_0_2/cv23/run4.log`). A team check
  takes about 160 ms of CPU, the team read again by each host call (`height_system` in
  `player_tier`, `card_limits`, `tier_values` and `check_team`)
- [ ] after 0.0.2 (from 0.0.1's cv7): split the modules past a thousand lines, with the tests'
  pass below: `save_editor` `view/mod.rs` (1,672 lines), `view/interchange.rs` (1,484),
  `team_widgets` `pitch/mod.rs` (1,600) and `tactics/mod.rs` (1,292)
- [ ] after 0.0.2 (from 0.0.1's f6): the Ruleset editor's undo merges a run of steps to one
  value as the Save editor's does (`plans/ruleset_editor.md`, the undo paragraph; DECISIONS
  "A run of undo steps is keyed by the widget call") -> verify: ten wheel notches on a
  tier's count, one Undo
- [x] mutfast (2026-11-19, `7feea20c`, `aa9928dd`): mutation runs test each mutant with
  cargo-nextest, which stops at its first failing test: a caught save_editor mutant's tests
  took 28-162 s against 303-771 s under `cargo test`, and the whole suite about the same
  (444 s against 418 s). The VPS has cargo-nextest 0.9.146
  - [x] Carbon's rounds on the change (`7feea20c..aa0f62ee`), 2026-11-20: three rounds, 8, 8
    and 5 accepted, the last none changing a run's outcome; reworked in `246a988c`,
    `9a66b2b8` and `7fc3d374` (rulings `.tmp/duck_rulings_nextest.md`)
- [ ] after 0.0.2: a crate whose mutation-run estimate passes a budget (about an hour a
  side) runs detached from the checkpoint, which closes on the rest (save_editor's held one
  ~12.5 h before mutfast)
- [ ] after 0.0.2 (from 0.0.1's f6, the whole-crate mutation runs' two design findings):
  - `team_widgets` `pitch/popover.rs` `slot_popover`'s outside-press focus surrender is
    unobservable under egui 0.37 (`Focus::end_pass` drops an undrawn widget's focus; the
    DragValue commits as typed), so the block and its "cannot end on its own" comment look
    removable: check against an egui upgrade first
  - `save_editor` `abilities.rs` `playable_positions`' `column_width` measured 350 against
    an available 350 over the whole suite (9,008 frames), so its `max` and `+ CELL_PADDING`
    decide nothing where the tests look; whether a wider window or font ever makes them
    bind is unmeasured, its `+` -> `*` mutant a survivor until that is settled
- [ ] after 0.0.2 (from 0.0.1's f7; the maintainer, 2026-11-04: tests are code,
  `CONTRIBUTING.md` "Tests are code"): the tests' design-health pass. The ranking is
  `.tmp/after_cut/f7/files_w8.txt` (`clones.py --files --window 8`); `aesthetics_export`'s
  `validate/tests.rs` is done (split into eleven files, 15 of 29 clusters into tables and
  helpers). The window-8 ranking also counts a file's scenario idiom (one call over a
  listing, then one assert), which is not copied setup, so briefs target runs of ten shaped
  lines or more. The handoff's rules: `.tmp/after_cut/f7/handoff.md`.
  - the ranking's other top files, each split by concern as a pure move
    (`pure_move_check.py`, its own commit) before its helpers are extracted
  - every other test module past a thousand lines split the same way, in due time (117
    files at `69687460`); meanwhile a diff that grows one splits it first
  - for 0.1.0 (the maintainer, 2026-11-15: "That would never happen in real life, please
    be mindful of overengineering"): every test module read for tests of inputs no member
    makes, and the production branches that exist only for them (two inputs in one frame, a
    key in the frame between a state change and its dialog, bytes no game or tool writes,
    crafted files); each removal ruled by the lead with the reach it lacks, in the same
    brief as the module's split -> verify: per module the tests and branches removed with
    their rulings, the suite green after each
- [ ] after 0.0.2 (the maintainer, 2026-11-16): the Tactics editor's arrangement setting
  (`plans/save_editor.md`, the "one grid" paragraph's "arrangement"). Waits on the
  maintainer, who provides 4ccEditor's layout; then the draft, then one slice.
- [ ] after 0.0.2 (the maintainer, 2026-11-16): a Snap audit of stale comments, beside the
  citation audit: per function, its doc and inline comments, its body and the same file's
  functions it calls; Snap answers whether a comment states something the code does not do.
  Measure first: the comments the review rounds accepted as stale (`.tmp/duck_rulings.md`,
  reach (d)), each at the commit its reviewer read, against the same comment after its fix;
  build the recipe only if Snap separates them about as well as it does citations (AUC 0.995).
  It shares the Snap client (keys, cache, request) with `citation_audit.py`. It cannot see a
  comment wrong about another file (a doc listing its callers).
- [ ] after 0.0.2 (the maintainer, 2026-11-12: "it can wait"): the drafts draw from a palette as the app does (the maintainer,
  2026-11-12: "The draft should ideally use a palette like the app does"): every colour
  that follows the theme is a variable of the host page named as `studio_core`'s
  `theme.rs` names it, with a light value beside the dark one, and the settings' Theme
  choice switches them; the colours that follow no theme (the pitch, the medals, a
  name's own codes, the sample data) stay literals, marked as such. Measured 2026-11-12
  (`.tmp/0_0_2/undo/colour_census.py`): 160 hex literals outside the 17 variables, 75
  distinct, 116 of them in `save_editor.js`. A pure refactor first, proven by
  `draft_shots.py` showing no changed shot on the dark theme; then the light values.
  Opus, when no draft edit is in flight
- [ ] after 0.0.2 (the maintainer, 2026-11-13: "something we will have to fix"): the
  unsaved changes list's sideways scroll bar at the list's bottom edge. Since the X
  (0.0.2-undo b) the table sits in a sideways scroll inside the vertical one, so the X
  column can stay outside it (`save_editor` `view/changes_popup.rs` `table_with_xs`), and
  the bar is under the table's last row: on a list both taller than its 240 points and
  wider than its room it shows only once scrolled to the end (S-undo-b-4; a mock of both,
  `.tmp/0_0_2/undo/bar_edge.png` and `bar_under_top.png`, by `shot_bar.py`). The wheel
  still scrolls sideways
- [ ] after 0.0.2 (the maintainer, 2026-11-14): PES 2017's two numbers, the live caller
  trace. The maintainer plays one exhibition with the automatic levels on while
  `.tmp/num17/watch.py`, extended to log the return address and ECX at the field copier's
  hit (`0x0311e929`), names the match code that copies the presets; that destination's
  readers are the match's converted form (`4cc-studio-temp/pes17_re/out/report_re.md`
  section 4). If it shows the two numbers unused, SE-GUI-71's labels and second lock are
  revisited
- [ ] after 0.0.2 (the maintainer, 2026-11-14), each version's turn as its support is stated:
  whether PES 18 to 21 read the two numbers. Marco's PES 21 finding (both ignored,
  `spec/pes_savefile/model.md` "Player/team/tactics model") is a claim to verify; PES 20 is
  expected to match PES 21, and PES 18 and 19 may not use the bytes either. Until then
  SE-GUI-71's labels and locks hold on PES 17 to 21 alike
- [ ] after 0.0.2: the Tactics pitch of the versions not measured yet (`plans/save_editor.md`
  "Each version draws its own screen's pitch"; until then PES 2015 and PES 2016 draw PES
  2017's, the maintainer, 2026-11-13: "more similar to 17", moved by 0.0.2-range). Settled
  2026-11-13: PES 2019 is PES 2021's, PES 2018 is PES 2017's (0.0.2-range moves it), and
  PES 2020 is taken as PES 2021's unmeasured (the maintainer). PES 2015's is a third
  screen, not a third `PitchLook`: on the maintainer's two pictures
  (`C:/Data/Pictures/ShareX/2026-11/PES2015-1.1.9.2-wip_2026-11-13_06-18-22.png` and
  `_06-19-09.png`) the pitch is 348 by 454 where ours is 320 by 384 (0.77 wide to tall
  against 0.83), with touchlines, solid white lines, a centre spot, about eighteen bands
  of 25 rows under a brightness gradient (dark at both ends, `#055e01`, to `#157808` at
  the halfway line), and its markers are dots with the name above, no portrait and no
  badge. So it needs its own canvas shape, its own mapping of the stored spots and its
  own markers, each to measure. PES 2016's is between the two (three pictures,
  `PES2016_2026-11-13_06-23-03.png`, `-18` and `-24` in the same folder): PES 2017's
  seven bands at PES 2017's row scale (the inner five 53.4 tall, edges on rows 46, 100,
  153, 207, 260, 313 of 360) and its penalty box (133 wide) and goal area (60), but the
  pitch is 275 wide by 360 (0.76, PES 2015's shape), the turf olive (`#426522`,
  `#4d712a`), the lines stronger, with touchlines and a centre spot, and a marker is the
  name over a position badge over the overall, no portrait. So it too needs its own
  canvas width, mapping across and markers
- [ ] after 0.0.2: Live AATF in a native window of its own (the maintainer, 2026-11-01; the
  embedded one is cv28): an egui viewport (`show_viewport_immediate`, the embedded window as
  its fallback where the platform has no second window), not always-on-top (desktop-wide, so
  it would cover other programs; a member moves it off Studio's window instead) → first
  measure what a second viewport costs per frame on the maintainer's machine, and what focus
  and the keyboard shortcuts do across the two windows; then the plan's paragraph, its
  scenario and a slice
- [ ] after 0.0.2: the pitch's name scroll delays as the game's (the maintainer, 2026-11-01;
  `plans/save_editor.md` "Tactics editor", the marker bullet's scrolling name: the editor
  starts every name together, the game staggers them by a rule not established) → first measure in PES
  2017 on captures of a whole lineup whose names overflow, over several 13.5 s periods: each
  name's delay from the team screen opening to its first scroll, against its slot, its
  position and its width; PES 2019 the same if 2017's rule is unclear; then the plan's
  sentence, a scenario and a slice
- [ ] after 0.0.2: the pitch's portraits from the team's aesthetics export first (the
  maintainer, 2026-11-01): before the PES folder's `download/` CPKs (`plans/save_editor.md`
  "Tactics editor", the Portraits bullet), the Save editor looks in the exports folder
  (`exports_folder_path`) for the shown team's aesthetics export and takes every portrait it
  holds from there; `download/` stays the fallback for each player the export has none for,
  and the placeholder after both → first read how the Compiler finds an export's team
  (`aesthetics_export` `resolve_identity` over the teams list) and which player a portrait
  belongs to (`spec/aesthetics_export/player_folders.md` "Savefile identity" and
  "Portraits": a player folder's `portrait.*` or the standalone `Portraits/NN.*`, any
  accepted format, a conflict between the two), and reuse both rather than a second reading.
  While any shown portrait came from the export, a Refresh button, a symbol button (the
  suite's icon set, a tooltip naming it), stands at the pitch's top right and reads the
  export's portraits again, so a member iterating on a portrait sees each change without
  reopening the save (the maintainer, 2026-11-01). Then the draft (the button's place
  against the Set PES path button's bottom right), the plan's paragraph (what a malformed
  or ambiguous export does, off the UI thread as the CPK read is), its scenarios, and a slice

### Phase 10 — Music tools

The phase's steps are itemized when Phase 9 closes; its format crate was built ahead of it:

- [x] 10.0 `libs/music_export`: the `.4ccm` format crate (`music_player.md` "The `.4ccm` format")
  (10.0a-10.0e: model, parser, writer, evaluation, selection, resolution; `just mutants
  music_export` 0 missed; DECISIONS 2026-10-18)
- [x] 10.0f the legacy-behavior audit against Rigdio v2.2.0 (`RUNBOOK.md` "Before a tool
  phase"; `.tmp/phase_10/10_0f/rigdio_audit.md`): done 2026-10-21 (DECISIONS 2026-10-21, "A
  `.4ccm` reads as Rigdio reads it")
- [x] 10.0g the crate to the audit's rulings: in-order conditions, `once` spent when reached,
  `TeamLists` with one shared `goal` fallback per team. Done 2026-10-21 (`dff1548f`,
  `5cf02863`)
- [x] 10.0h the Music export editor's legacy-behavior audit against RigDJ v1.16.0
  (`.tmp/phase_10/10_0h/rigdj_audit.md`): 42 rows ruled into the plan and four DECISIONS
  entries; PROPOSALS P40
- [x] 10.0i 10.0f's part 2, Rigdio's player controls against `plans/music_player.md`. Done
  2026-10-27 (DECISIONS "The Music player's controls, ruled against Rigdio v2.2.0")

### Phase 12 — Kit config editor

The phase's steps are itemized when it opens; prepared ahead:

- [x] 12.0a the tool's GUI plan against Kit Manager's recovered behavior. Done 2026-10-27
  (`.tmp/phase_12/12_0a/kitmanager_audit.md`; DECISIONS "The Kit config editor saves on Save,
  and the rest of Kit Manager's audit"; PROPOSALS P41-P42)
- [x] 12.0b in game (PES 2016): a long-sleeve badge on undershirt-only long sleeves (0x02 =
  0xBB) draws no badge on the forearm; stopped 2026-10-27, the editor grays the four fields

### Phase 13 — Refs arranger

The phase's steps are itemized when it opens; prepared ahead:

- [x] 13.0a the plan's readiness audit: `plans/refs_arranger.md` against what it relies on
  (`.tmp/phase_13/13_0a/refs_audit.md`; DECISIONS "The Refs arranger's plan checked against
  FoxDen and the export library")
- [ ] 13.0b in game, when the phase opens (PES 2021 with FoxDen copied in for the test and
  removed after, pad and OBS): which of positions 0..4 is the main referee, the linesmen and
  the other two (`plans/refs_arranger.md` "Open questions"); FoxDen's files are not in any
  game folder today

### Phase 14 — Balls compiler

The phase's steps are itemized when it opens; prepared ahead:

- [x] 14.0a the legacy-behavior audit of the 4cc balls compiler 2.03
  (`.tmp/phase_14/14_0a/balls_audit.md`): 49 rows ruled into the plan, five DECISIONS entries

### Phase 16 — Polish and distribution

Steps are itemized when Phase 15 closes; four are fixed already:

- [ ] 16.k the Kit slot assignments dialog's columns further apart and its grid centred across
  the dialog (DECISIONS 2026-11-17; `plans/save_editor.md` "Editing UI"): draft first, then
  the build -> verify: the built window beside the draft

- [x] 16.w the release exe's platform floor and size (maintainer, 2026-10-15; DECISIONS
  2026-10-15): Windows 10 x64 and Linux, `+crt-static`, `rhai` `only_i64` (16.w1-16.w3)

- [ ] 16.x `dds_convert` GPU BC7 (deferred from 2.5b to Phase 4, decision entry 2026-09-21, then
  past Release 0.1.0, decision entry 2026-09-28; spec `libs/README.md` "First-release desktop GPU
  BC7"): `block_compression`'s wgpu backend on a Vulkan/Metal device, CPU fallback with the
  fallback reason reported, cold pipeline creation and upload/readback measured first, then the
  texture step's bounded batches → verify: GPU and CPU outputs decode within the same tolerance
  on the `dds_convert` fixtures; the fallback path exercised by forcing no adapter
  - [x] 16.x a `dds_convert`'s half, built ahead and unwired (2026-10-19, DECISIONS "the GPU
    BC7 encoder is built ahead of 0.1.0"): `gpu::GpuEncoder`, `just gpu-tests`
  - [ ] 16.x b the Compiler's texture step on the encoder, after 0.1.0: the CPU starts at
    once and the GPU joins when its device is ready, started when the run has BC7 work
    (`plans/libs/README.md` "First-release desktop GPU BC7"); start it after the window's
    renderer is up, a failed start this run's CPU fallback with its reason in the report
    (`.tmp/phase_16/16x/findings.md`, the transient adapter failure seen under load). The system log
    (checked 2026-10-21) has no driver reset at any of those failures (04:07, 04:15 and later
    on 2026-10-19); that day's one reset, 03:16:41, which Adrenalin reported twice (two
    `LiveKernelEvent` reports), was the probe's single-submission DX12 run of an 8192 chain.
    So the adapter blip is not a timeout: most likely the Vulkan loader with several processes
    opening it at once, unexplained
  - [ ] 16.x c the BC7 speed probe, after all of DBC's work, the GPU encoding study with it once
    0.0.1 is out (the maintainer, 2026-10-30, after a 2048² face's CPU BC7 chain took 31.7 s of
    CPU against 128 ms for DXT5, `.tmp/release_0_0_2/jxl_bench/findings.md`; VGL 27's PES 2019 compiles are
    BC7 throughout) (the maintainer, 2026-10-21;
    `plans/libs/README.md` "The workload the speed is judged on"): on the RX 9060 XT and its
    CPU, real kits (4096² and 8192²) and faces, time and PSNR for `block_compression`'s BC7
    presets (`ultra_fast` to `basic`) on GPU and CPU, `intel_tex_2`'s on CPU, BC3 as the
    quality floor; a team's ~30 chains in flight on the GPU at once against one at a time; the
    share of flat 4×4 blocks in real kits and faces, and the time a direct write of them saves
    → verify: a table per encoder and preset, and a recommendation: the preset and CPU encoder
    to ship, or the BC3 fallback, with a decision entry; our own encoder only if every lever
    falls short

- [ ] 16.y a settings panel where a member customizes the palette (the maintainer,
  2026-10-29: after Release 0.2.0), on ov11a's `studio_core::theme::Palette` (`plans/core/gui.md`,
  the Dark/light theme row): each named colour editable with the suite's colour picker,
  saved in the settings, reset per colour and whole; `theme::palette` then returns the
  member's edited copy, the built-in dark and light instances its defaults. To plan first:
  whether egui's own `Visuals` colours (text, widget fills, the accent) are editable too,
  or the palette grows fields that set them; a draft, then the plan section

### Phase 20 — Ruleset editor (post-release)

Opened 2026-10-23, pulled forward for DBC (`plans/core/development_plan.md` "Divegrass Before
Christmas"); itemized as 20.3 onward below. Its GUI was drafted ahead of it, at the maintainer's
request (2026-10-14), to show the members who offered to help shape the ruleset format:

- [x] 20.0 The Ruleset editor's GUI draft (`docs/prototypes/ruleset_editor.html`), with the
  coverage tables of Clouds' proposal and the DBC7 rules in `aatf_rules.md` (20.0a-20.0e;
  DECISIONS 2026-10-20, 2026-10-21)
  - [x] 20.0c the draft brought up to date (maintainer's request, 2026-10-15): the five other
    embedded rulesets as working templates and a sixth, "Clouds' proposal, every item (from
    VGL26)". Done 2026-10-15 (SWE-2)
- [x] 20.1 Autumn 26 against the official 4ccEditor's Autumn 26 edition: six differences
  ruled against the wiki's `Appendix:Rules` (DECISIONS "Autumn 26 follows the wiki's written
  rules…"). Done 2026-10-21
- [x] 20.2 Autumn 26 takes the 4cc organizer's answers (DECISIONS 2026-10-22). Done
  2026-10-22
- [x] 20.3 the Acceptance section (lead, 2026-10-23): `plans/ruleset_editor.md` "Acceptance",
  logic version 1 frozen at the 0.0.1 cut (DECISIONS 2026-10-23). Done 2026-10-23
- [x] 20.4 `libs/aatf`'s writer (`src/writer/`, `Ruleset::to_data_block`): every ruleset
  reads back equal from what it writes. Done 2026-10-23
- [x] 20.5 the tool crate's egui-free core (`plans/ruleset_editor.md` "Crate layout"):
  `document.rs`, `summary.rs`, the CLI's `validate`, `update-logic`, `summary`. Done
  2026-10-23
- Set aside on 2026-10-24 (the maintainer: not in 0.0.1, after Release 0.1.0;
  `plans/core/development_plan.md` "Phase 20"): 20.6a2a, in flight then, is reviewed and
  landed, and the steps below wait until after 0.1.0.
- [ ] 20.6 the view (`plans/ruleset_editor.md` "The editor", the draft
  `docs/prototypes/tools/ruleset_editor.js`), as slices:
  - 20.6a the window's frame and the plain forms, as subslices:
    - [x] 20.6a1 `settings.rs`, the templates and Blank, undo, the start page, the toolbar,
      the rail, the preview, General, Custom checks and Summary (RE-GUI-01, 02, 13 to 15,
      RE-SUM-01, RE-FILE-01 to 03, 05, 07). Done 2026-10-23, its review reworks to 2026-10-24
    - 20.6a2 Tiers with the medal systems, Cards, Universal rules with the team settings,
      Suggestions (RE-GUI-03 to 06, 08, RE-FILE-04, 06), as two subslices (one is past a
      review's size):
      - [x] 20.6a2a Tiers and the medal systems, the tier expander's per-stat targets and free
        cards (RE-GUI-03, RE-GUI-04's tiers half, 05, 06, RE-FILE-04, 06). Done 2026-10-24,
        reviewed (`74dcf473`)
      - [ ] 20.6a2b Cards, the team settings, Suggestions (RE-GUI-08)
      - [x] 20.6a2c Universal rules without the team settings (RE-GUI-20), for the illegal
        formations checkbox (cv14; the maintainer, 2026-10-31: "a very simple page", the
        team settings may wait): `sections/universal.rs`, after cv14a lands
  - 20.6b the condition editor, the effects editor, Heights and Specials (RE-GUI-07, 09 to 12,
    and RE-GUI-04's height-systems half, whose test joins the tiers half's)
- [ ] 20.7 Randomize (`plans/ruleset_editor.md` "Randomize"): `random/`, the start page's and
  toolbar's entries, the CLI's `random` (RE-RND, RE-CLI-04) → verify: 1,000 seeds per preset,
  and the share of seeds that needed a redraw measured and reported
- [ ] 20.8 the help chapter, and the Ruleset editor active on the shell's sidebar
- [ ] 20.8a the built window beside the drafts, page by page (`draft_compare.py rs_general`,
  `rs_tiers`, and a screen for each page they do not list): every row within 1 point.
  General and Tiers were off page-wide on 2026-11-17 (the numbers in 0.0.2-gaps' Tiers item)
- [ ] 20.9 converge (RUNBOOK.md "Closing a phase"), RE-GUI-16 checked by hand

---

## Pending looks

Checks only the game or the built window can make, which the lead's in-game or window-capture
harness makes, one line each: what to look at, the PES version, the step or decision it
belongs to. Ordered by the cost of a wrong result, game-facing writes first. A look leaves this
list when its finding or `manual:` line lands in its step. Those only the maintainer can make
are cards in `docs/maintainer/tasks.html` (since 2026-11-15; a step line's "Pending looks" for
one of them means that page).

- The aesthetics patch on an unstripped save, the compiled look in Edit mode (boots and gloves,
  a base-copied player, one with clear flags), the FoxDen gloves issue, SE-GUI-34 in patch
  form; PES 2019, then 2021 and 2017; 8.36d; harness
- The Friendlies compiled with `--legacy-ids` (`4cc_90_test.cpk` and the patched save), boots
  and gloves under their old IDs in Edit mode; PES 2019; 0.0.2-poc13's poc.2; harness
- A team's home stadium id (the u16 at +6) edited by the Save editor shows as that stadium in
  game; PES 2015 and 2016; 0.0.2-st15 (DECISIONS 2026-11-11); harness
- A pre-Fox texture written uncompressed with a side no multiple of 4 (a 1030 × 513 hair part)
  loads and draws; PES 2017; 0.0.2b (DECISIONS 2026-11-11 "texture_not_div4 holds only a
  pre-Fox texture the compile block-compresses"); harness
- The Ability stats column on a PES 17 save in the built window: Tight Possession's and
  Aggression's names as dim as Through Passing's beside them; no PES; 0.0.2-statlbl; harness
- A motion and a face edit in Edit mode on a save with a compiled team's patch: which keys the
  record then holds; PES 2021; 8.23a; harness
- SE-GUI-29: each Texport the panel exports, imported by the game's team import; PES 2018 to
  2021; 8.9k's k5d; harness
- Whether a team-file import writes the DLC team bin's name into the save, as on PES 2017; PES
  2015, 2016, 2018 to 2021; 8.9k's k5g; harness
- The seasonal logos on an uncompressed 4096 main kit, looked at in the pre-match menu (its
  two demonstration players, when they wear the default kit models) or in a match: Edit
  mode never places them, no competition type being chosen there. With it, whether the
  player still draws see-through under an opaque texture (the first capture's test pattern
  had alpha about 68, `.tmp/0_0_2/ingame/poc/s42_egg_body.png`); PES 2019; 0.0.2b; harness
- Each version's stat names, read off its Edit screen; PES 2015, 2016, 2018 to 2021; 8.29d's
  d3b; harness
- What a zero emblem in a Texport the panel wrote does in Select Team; PES 2017; 8.9k's k5f;
  harness
- Each launcher prints in its console and its `pause` waits, on 0.0.2's bundle; any PES;
  0.0.2 (from 0.0.1l); harness
- The built window at 1280 × 800 on the FHD monitor, the Tactics tab in three columns; any PES;
  8.15; harness
- The Fox ownership line's Apply link in the built window; PES 2018 to 2021; 8.38; harness
- The Save editor's eight help chapters and the two core ones read in the built window; any
  PES; 8.29e's e1; harness
- The Appearance screen in the built window at 1280 × 800: the seven groups whole with no
  scroll bar, lock off and on, the strip grid's lines; any PES; 0.0.2-applayout; harness
- The Live AATF pitch's warning cross (`#D29922` from 0.0.2-noticeerr), drawn for a Warning-severity position mark (no
  real save under 4cc Autumn 26 raises one); any PES; 8.9m's m4; harness
- ov7b's Import and × dialogs, ov10's pitch drag and ov12's popover steppers in the built
  window; any PES; 0.0.1ov; harness

---

## Issues

Bugs, unexpected behavior, things to revisit. `open` / `resolved (date)`. Resolved issues are
pruned when their phase closes; they stay in git history. A question only the maintainer
can answer is not an issue: it goes to `docs/maintainer/questions.html`, and an issue that waits on one
points there.

- open (2026-11-23) — the grid's detail window grows for a few frames after it opens: when it
  replaces a smaller pinned window, it grows about 27 points a frame for four frames (387 to
  485 points tall) before it settles, measured in `export_grid`'s tests by G5a. The findings'
  `ScrollArea` sizes itself to the space the `Area` had last frame, so the window reaches its
  height a step at a time. It predates G5a (the same growth with G5a's change undone). Due: a
  look in the built window to see whether a member notices it, then a fix if one does.
- open (2026-11-23) — `just corner-radius` reads only `CornerRadius::same(` and
  `.corner_radius(`, so a painter's `rect_filled(rect, 2.0, …)` passed (`export_grid`'s detail
  top box, Carbon "C-grid3-11"). Widening it to a numeric radius of `rect_filled` and
  `rect_stroke` (0 allowed: a square rectangle) also flags the dock's grip,
  `studio_core/src/widgets/dock.rs` `paint_grip`, painted with 1.0: that grip's radius wants a
  look against its draft first, then the gate's change.
- resolved (2026-11-23) — `just acceptance` counted a citation only on a line holding
  scenario IDs alone, and said nothing of `// TC-GUI-10: prose` directly above `#[test]`
  (Carbon "C-grid3-7"). Since `51cab105` the gate fails a line of the comment block above a
  `#[test]` that starts with IDs and a colon. The 41 such lines (G5b's six, and 35 more the
  gate found across the workspace) now carry their IDs on a line of their own, which proved
  TC-GUI-10 and SE-GUI-91.
- resolved (2026-11-23) — the checkpoint on `6c8efed2` failed in `studio_core`'s
  `no_source_file_passes_a_number_as_a_corner_radius` (ST-CORNER-01): 1c-iii-b's dialog
  (`team_compiler/src/view/upgrade_dpfl.rs`, `list_box`) writes `CornerRadius::same(2)`. The
  scan is a test of `studio_core` that reads every crate's sources, so a slice whose Verify
  tests only its own crate never runs it, and the lead's rerun of that slice's gates missed it
  too. The line now takes the theme's radius (`3c47d77a`), and the scan is now the gate `just
  corner-radius`, which reads every crate whichever one changed.
- open (2026-11-23) — `team_compiler`'s test sandbox (`src/testing.rs` `sandbox`) writes a
  teams list holding `792 /egg/`, a pairing no user sees: the real list
  (`crates/libs/teams_list/data/teams_list.txt`) has 792 as `/END/`, and `/egg/` is not in
  it. Every test through `sandbox` inherits it, and fixtures name `egg` exports. Fix: a real
  row from the list in `sandbox`, and the tests asserting `/egg/` moved to it. Found by the
  1c-ii sidekick.
- open (2026-11-23) — the GUI tests' shape helpers are copied crate by crate: a function
  collecting the strokes painted over a widget's bounds (`strokes_over`) is in
  `team_widgets`' `test_support.rs` and `save_editor`'s `view/tests.rs`, and the 1c-iii-c
  slice adds one to `team_compiler`'s `view/tests/upgrade_dpfl.rs`; the selection-edge slice
  writes the same filter out inline in `studio_core`'s help window and log panel tests and in
  the Ruleset editor's preview and problems tests. `rect_of_fill_at` and the `Headless` test
  window are copied the same way.
  Each crate stays under `CONTRIBUTING.md`'s third-copy line, but each new look test copies
  them again. Fix: one home for them that every crate's tests can use, which `studio_core`
  does not offer today (no test-support module or feature). That is a crate-boundary choice,
  to be ruled before it is built.
- fixed on `pes14` (2026-11-22, `17463113`), in Build 3 (`0.0.2-pes14.3`) — PES 2014 Build 2
  (`0.0.2-pes14.2`) crashed on the Appearance page, and on Hide default body, of a player the
  game created on an in-game edit (the Akko7 organizer's /hi3/ players,
  `.tmp/pes14_crash/EDIT.bin.newer`): such a record stores the gloves ID with all 18 bits set
  (262143), which the code narrowed to `u16` behind an `expect`. Nopanic-1 keeps the IDs at
  `u32` and shows an error line in place of a panic (merged into `pes14` at `33995510`); the
  gloves slice reads 262143 as no gloves model, as the game draws it (the `pes14` branch's
  plan, "Gloves 262143 is no gloves model"). Build 3 passed its gates, and its release build
  was checked on copies of both saves: HAGSEO's Appearance page and Hide default body, team
  127 in the newer save, /gd/ at 136 in `EDIT.bin.autumn14`. It is left unpushed. Build 3
  now also carries the `pes14` plan's step 12 (Team TOML on PES 2014). That step landed on
  `pes14` 2026-11-22 (slices a to f, then `91d84e69`), its data check passed on the rebuilt
  release. Build 3 was rebuilt at `be6f9ed5` (`pes14`): full gates passed, the /gd/ check
  and HAGSEO's Appearance page and Hide default body checked again on its release build. Its
  `tasks.html` card is back: the maintainer approves the `[Unreleased]` section.
  Step 12's in-game look (2026-11-22, SWE-2, `.tmp/step12/ingame/findings.md`): the imported
  /hi3/ shows /gd/'s names, positions, numbers and Game Plan, its one appearance record
  travels, and gloves 262143 draws plain hands. The team's name showed as "Invitational 3":
  the import wrote /gd/'s `edit_flags.name = false`, and the game shows a stored team name
  only with it set. Fixed on `pes14` (`042e60b8`): an import that renames a team sets the
  flag (`plans/pes_savefile/operations.md` "A written team name shows", DECISIONS). Build 3
  rebuilt there (`tasks_build/UTW-pes14-042e60b8.exe`), its scoped gates passed (4,820
  tests), and a second in-game look (`.tmp/pes14_ingame/teamname/findings.md` in its
  worktree) shows "/gd/" in the Italian League's list. The maintainer
  said yes 2026-11-23: Build 3 cut as `0.0.2-pes14.3` (`48ab28bc`). Main has the same rule
  (2026-11-23, merge `ac123e50`, SWE-2), with its CHANGELOG line.
  The Compiler's view (item 6) goes on without waiting for it (the maintainer, 2026-11-22).
- open (2026-11-22) — The Save editor's tests leave their scratch behind:
  `test_support::scratch` empties `target/<profile>/save_editor_tests/<test>` when a test
  starts and never removes it after, so every checkout keeps 1,064 folders, many holding a
  save copy: 12.8 GB in main's `target/`, 13.2 GB in the gate's, and the same in each
  sidekick's tree until its `target/` goes. Fix (SWE-2): `scratch` returns a guard that
  removes the folder when it drops and the test did not panic (a failed test's folder stays
  to be read); check the other crates' scratch helpers for the same: main's `target/tmp/`
  held 5.6 GB of other suites' leftovers on 2026-11-22 (`export_upgrader_in_process`,
  `team_compiler_cli`, `save_editor_in_process`, `team_compiler_in_process`, ...). Until the
  fix, these folders are what to delete when the disk runs low, in a tree no job runs in
  (28.5 GB from main and `pes14` on 2026-11-22).
- open (2026-11-22) — Panics on file content (DECISIONS 2026-11-22 "What a file holds is never
  an invariant an `expect` rests on"; `CONTRIBUTING.md` "Code style", the Errors bullet). A
  census of the 885 `expect`, `unwrap`, `panic!`, `unreachable!` and `assert!` calls in
  production code at `ffa45247` (lists `.tmp/nopanic/part_{a..d}.txt`, made by
  `.tmp/nopanic/surface.py`), each classed as resting on a file, on our own code, on the
  environment, in a developer tool, or unsure (SWE-2, `.tmp/nopanic/census_*.findings.md`).
  Result: one call rests on a file (`team_widgets` `tactics_term`'s byte, PES 2015 and 2016
  storing a lineup slot in 32 bits), 13 on the environment (poisoned locks, impossible input
  counts), the rest on our own code, three of those on a schema's field width
  (DECISIONS 2026-11-22 "A decoded value keeps the type it was decoded into"). Slice
  nopanic-1 (SWE-2, `sk/nopanic1`) fixes the four: boots and gloves IDs carried at `u32`
  through `fpc`, the Appearance page's check showing an error line, `settings_toml`'s byte
  read returning an error, the tactics wording showing the stored number. The `pes14`
  branch's own code gets the same census after it merges.
- fixed (2026-11-22, builder slices 1 to 7, the last `65341792`) — Every new check on what an
  export must hold broke every test that builds one, and more checks are coming. Slice 0.0.2-logo a's `logo_missing` broke 172 tests:
  each test writes its own export file by file, directly or through one of about a dozen
  helpers spread over the tests of `team_compiler`, `export_upgrader` and `studio`, and the
  fixture archives are stored binaries. Each had to learn the check by hand: about 130 added
  logo calls and five rebuilt archives. Fix, after logo slice a merges and before slice b, so
  b's tests are written on it: one builder of the smallest clean export of each kind (`Full`,
  `Midcup`, an old Red export) in `aesthetics_export`, behind a `test-support` feature as
  `pes_savefile`'s fixtures are. Every test starts from it and removes or breaks only what it
  tests; the archive fixtures stay stored and gain the new file by script. A new check then
  changes the builder and the tests of what it catches, not every test. Slices: step 6's
  "The shared export builder".
- fixed (2026-11-22) — The citation audit's 24 unruled flags: 20 accepted, their 12 scenarios
  widened (SE-GUI-69, 74, 82, 83, 85, 106; ST-REP-06, 09, 10; ST-UPD-01; ST-BAR-02;
  TC-REF-26), 4 rejected, and SE-GUI-82's new flag after its widening rejected too;
  `scripts/gates/citation_rulings.md`.
- fixed (2026-11-23, merge `eeca7652`, Opus) — The code said "paid cards" where the docs and
  drafts say "non-free cards": the findings, the Tiers column, the summary, the logic's `paid`
  key and the code's names now say non-free, the logic's hash following (version 1 until
  1.0.0), with its CHANGELOG line under Other stuff.
- open (2026-11-22) — The Compiler's test teams lists pair ids with names the bundled list
  does not: `team_compiler/tests/common/mod.rs:18-19` and `src/testing.rs:277` hold `790 /dbg/`,
  `792 /egg/` and `793 /esg/`, where `crates/libs/teams_list/data/teams_list.txt` holds
  `789 /dbg/`, `790 /domg/`, `792 /END/`, `793 /feg/`, and no `/egg/` or `/esg/`; the comment at
  `common/mod.rs:15-17` says each row is the bundled list's own. `/egg/` is the tracer's team,
  which the bundled list lacks, so its row stays and the comment says why; the others take
  their real ids (the test-data rule). Small, SWE-2, beside the builder's slice 3.

- resolved (2026-11-21) — The maintainer's PES 2021 save opens with an empty team list. The
  release build at `1b9c5f27`, File > PES 2021, opened
  `Documents\KONAMI\eFootball PES 2021 SEASON UPDATE\...\save\EDIT00000000` (11,011,039 bytes,
  2026-11-18): the title named it, "All teams" listed no row, the Player and Team menus stayed
  grey, and `check-aatf` on the same file printed nothing. The test fixture (10,996,712 bytes)
  opened in the same window listed every team. Cause: the save holds no cup team. On a copy,
  `export-toml --all-teams` wrote no file, as it does for the `EDIT00000000_vanilla` beside
  it, while its June copy (`EDIT00000000 - Copy (4)`) gives 220. The editor lists cup teams
  alone by design, so the bug is the silence. Fixed in `e8c1561c` and `b22b191c`: a notice in
  the window, and a refusal from `check-aatf`, `export-toml --all-teams` and
  `export-teams-list` (plan "Only the cup's teams are shown"; SE-GUI-107, SE-AATF-11,
  SE-TOML-11, SE-TL-06). Seen in the release build at `b22b191c` on a copy of that save,
  beside the draft's stock PES 21 sample: the same warning in the status bar.
- resolved (2026-11-21) — The draft's Playing style pickers (the Stats page and the list's
  column, `save_editor.js`) listed one order for every version, where the build lists each
  version's styles in its stored order, the game's own list (DECISIONS 2026-10-21 "The
  playing-style combo lists the version's stored order"); the draft had not followed that
  decision. Found while giving the `pes14` draft PES 2014's own list. The pickers now read the
  version's list, PES 2015 and 2016's added (with them, those versions' changes rows name the
  right style).
- resolved 2026-11-23 (dropped) — The Ruleset editor's stat grids clip where the draft wraps. With the
  preview shown at the default window, the General section is about 300 points wide: the
  build keeps its five checkbox columns on one line each and cuts them at the section's edge
  (a sideways scroll reaches them), where the draft (`ruleset_editor/general?preview=1`) wraps
  long names onto two lines ("Attacking / Prowess") and shows four columns. It was already so
  with the old 560-point preview; seen in the release build of the larger-window slice
  (`.tmp/lead/re_look/`). Dropped 2026-11-23: the Ruleset editor's draft is not buildable, so
  its built UI is not compared with it (DECISIONS).
- resolved 2026-11-22 (dropped) — The trial of whether SubQ Snap can tell a flaky test from a
  steady one, as it tells a test that proves its scenario from one that does not. Asked per
  test whether its outcome depends on anything outside its inputs; the state was the test and
  the helpers of its file it calls, as the citation audit builds it. Five known flakes at the
  commit before each fix, against 100 tests sampled at random from HEAD's 6631:
  - AUC 0.790, against the citation audit's 0.995. Every score is low (the highest is 0.227),
    and a steady test tops the ranking.
  - The flakes ranked 2 (the PES 15 save/load test's one-byte seed), 5 (the pictures dialog's
    width), 18 (the kept-.7z test's free-memory budget), and 44 and 51 (the Multi-team list
    test, twice). The two frame races score as steady as the median test: their cause is when
    the headless harness delivers a check's reply, which neither the test nor its file shows,
    and one sat in a helper of another file.

  The project's flakes are mostly races of that kind, so the check does not join the
  checkpoint, and no variant of the question was tried. The script and its answers are in the
  gitignored `.tmp/snap_flaky/` and `snap.out/flaky_cache.json`.
- open (2026-11-22) — The Compiler's tests that write their own teams list
  (`team_compiler/src/testing.rs`, `output/savefile.rs`, `tests/cli/deep.rs` and `logo.rs`) use
  the bundled list's rows from before its VGL27 allocation (`6268280c`, 2026-11-09): `790 /dbg/`
  and `792 /egg/`, where the list now reads `789 /dbg/` and `790 /domg/` and holds no `/egg/`.
  Nothing fails, the lists being the tests' own, but test data copies today's real rows: update
  them when those files are next touched.
- open (2026-11-20) — A checkpoint's diff mutation run pays each crate's fixed costs however
  few mutants it holds. On `969e1cb6`, `team_widgets`' one mutant took 567 s wall:
  - a baseline of 133 s build and 199 s test, though the gates had just passed the same tree;
  - a split with `bonfire` that sent it no mutant.

  Two changes to weigh in `scripts/mutation/`:
  - skip the baseline's test when the gates passed (`cargo mutants --baseline=skip` needs an
    explicit `--timeout`, which the baseline now sets);
  - run a crate of one or two mutants on this machine alone.
- resolved 2026-11-20 (Carbon, then SWE-2) — Carbon's review of the referee blank face ran
  on `work` after all (one run takes 1-2% of its week). Its five accepted concerns, none
  changing a real export's output, are fixed in the commit that resolves this, which ends the
  referee fix's review rounds.
- open (2026-11-20) — `spec/team_compiler/red_divergences.md` holds 52 differences from Red's
  output marked Not checked, and 3 where two records disagree (RD-17, RD-23, RD-53). Each
  needs the check the file's head describes: which real exports reach it, what Red's output
  did for each, that ours is right. Its Status then becomes Checked, or the behavior changes
  where ours is wrong. Also owed: the parity rows' reasons, the spec's "unlike Red" sentences
  and `scripts/library_parity/classes.md`'s I-classes point at the entries, not restate them.
- resolved 2026-11-21 (SWE-2, `68d933e5`): `team_compiler`'s test
  `a_7z_export_s_source_keeps_its_decompressed_content_into_the_planned_run` failed whenever
  memory was short (232 MB free on 2026-11-09, 1.5 GB in a checkpoint on 2026-11-21), because
  `plan` read the run's budget from the memory the OS reported available. `plan` now takes the
  budget from its caller, and the test passes a fixed 2 GiB. The crate's other tests that reach
  the budget were searched: none asserts on an archive kept or on a wait.
- open (2026-11-20) — tests leave their scratch folders in `%TEMP%`: about 15,000 on the
  maintainer's machine, 680 MiB, mostly `export_upgrader_*` (`test_support.rs` `scratch`) and
  `studio_core_repro_*` / `studio_core_files_*`. Each folder's name ends in the test process's
  ID, and `scratch` only clears a folder of the same name before a test, so every test run
  (a mutation run's hundreds included) leaves its own. Fix: a scratch guard whose `Drop`
  removes the folder, in each crate's helper. The 17,280 folders found then (657 MiB) were
  deleted; about 700 more, whose process ID a running program now holds, were kept.
- resolved 2026-11-17 by 0.0.2-present (`0cbe7637`) (the maintainer; measured by the lead on the release build of `26ae8f7c`,
  DX12): while the window is resized, everything away from its middle jumps and comes back on
  each step (the sidebar's lower icons by two points). The cause is wgpu's DX12 swap chain,
  made on the window's handle with stretch scaling: Windows shows the last frame stretched
  over the resized window until the next arrives. A DirectComposition swap chain
  (`WGPU_DX12_PRESENTATION_SYSTEM=visual`) leaves it in place: 0 of 231 grabs of a scripted
  drag jumped against 230 of 231, and the maintainer's drag showed none; turning vsync off
  only thinned the jumps out. Fix: 0.0.2-present (`plans/core/gui.md` "The graphics
  backend"; harness `.tmp/lead/flicker/variants2.py`).
- open 2026-11-17 (the lead): the checkpoint's `wasm_check.py` warns that
  `ShellCondition`'s `SettingsSaveFailed`, `UpdateAvailable` and `UpdateReady` are never
  constructed (`studio_core/src/status.rs`): the code that builds them is native only. It
  also warns that `theme.rs`'s `CORNER_RADIUS` is never used (still both at `6eb5f71f`). The
  gate passes, and the warning was already in earlier runs (`pipeline_1117_rw5.log`), but it
  is noise every run prints, and a new warning could hide behind it.
- open 2026-11-17 (the lead): with `WGPU_BACKEND=gl` the release build of `26ae8f7c` never
  shows its window: the process spins one core and stops answering messages (window
  remembered on the 100% monitor).
- open 2026-11-14 (the lead, 0.0.2-dlgroom's sweep): two built dialogs differ from their
  drafts in ways the step did not touch.
  - The settings window (`.tmp/dlgroom/look/compare_settings.log`, `pair_settings.png`): the
    draft's window is wider and lists The Compiler's section, which the build leaves out,
    so its labels stand 12 points further right of the title and its Save editor section
    20 lower; its rows also stand 3 to 4 points lower against the title than the build's.
  - The range picture dialog's team selectors: the build writes each team's ID before its
    name (`701 /3/`), the draft only the name (`/a/`): resolved 2026-11-17, the maintainer
    keeps the build's (DECISIONS 2026-11-17), the draft follows.
- open 2026-11-17 (the lead, the answers recheck): with a 200% primary monitor and a 100%
  one beside it, two things go wrong on the 100% one (Windows 10, the release build of
  `a9ec4919`).
  - A window remembered there (`window_main` with scale 1.0) reopens at the primary's 200%,
    twice its size, straddling both monitors, and records scale 2.0 on close: the window is
    created at the primary's scale, and Windows moves it to 100% only when most of it lies
    on the 100% monitor. Seen with the place at (-1608, 453) on a 100% monitor at
    (-1920, 323); a place far enough left, (-1900, 330), came back at 100%.
  - Once the window is at 100% there, UI Automation (AccessKit) reports every widget's
    rectangle at twice its screen position and size (`uia_dump.ps1`: `v0.0.1 pre-alpha` at
    -3768,2240 160x26 for -1884,1120 80x13): a screen reader would point at the wrong
    places. Likely egui's AccessKit adapter keeping the creation scale: check upstream
    before a fix. `.tmp/lead/looks/compare_halved.py` halves the rectangles for
    `draft_compare.py` meanwhile.
- resolved 2026-11-14 (SWE-2, `513ef0af`): the copies of "write a temporary file beside it,
  then rename it onto the name" are one function, `studio_core::files::replace_file`, called
  by five sites (the repro bundle, the ruleset editor's `write_text`, the rules import, the
  teams list, the picture dialog). `Settings::save` and the Team compiler's
  `paths::replace_file` keep their own, since their temporary's name is unique per process.

- resolved 2026-11-14 (SWE-2 and the lead, `ddde3243`): the pipeline on `93d9b548` left 15
  mutants alive in `save_editor` `appearance/mod.rs`; 10 are caught by new layout tests, 3
  are equivalences in `.cargo/mutants.toml`, and 2 sat in a Physique stretch no version
  reaches (Physique is the taller side on every version), which is removed.
- resolved 2026-11-14 (SWE-2, `46d9d235`): SE-GUI-45's test failed once under load because the
  medal worker's reply, landing before the move frame, drew the held badge's outline (a
  stroked rect of the badge's size, transparent fill), which the finder took for the badge.
  The finder takes the filled rect; the assertion's value is unchanged. Reproduced by
  forcing the reply before the press.
- resolved 2026-11-14 (SWE-2, `46d9d235`): the patch panel's Item cells hovered the switch on a
  player no roster holds; the tooltip now follows the row's click target.
- resolved 2026-11-14 (the lead, briefing 0.0.2-noticeerr): the light theme's error red,
  egui's `#FF0000`, read at 3.8 on the light panel, under `plans/core/gui.md`'s contrast
  rule. The maintainer found 3.8 fine and the dark red too bright: both themes take
  GitHub's danger pair, `#F85149` and `#D1242F` (DECISIONS "the error red is GitHub's"),
  in 0.0.2-noticeerr.
- watch 2026-11-14: the Save editor's
  `view::tests::live_aatf::copying_before_a_result_copies_the_checking_line` failed once
  in a loaded full run (448 s) and passed alone five times, in its module and in a full
  rerun: its premise races the Live AATF check's worker thread. Fixed by 0.0.2-determ.
- watch 2026-11-13: the Save editor's
  `view::tests::savefile::cup::a_team_outside_the_cup_is_neither_listed_nor_checked_nor_repaired`
  failed twice under load ("the range's first team: left `702 /a/`, right `701 /3/`"), the
  second time in `just pipeline` on `9ed7606d`, and passes alone. Fixed from reading, never
  reproduced on demand: `range_items` (`view/tests/savefile.rs`) took the combo's bottom edge
  from the frame before its click, and a check landing between the two frames moved the
  dialog up, so the first item stood above the stale edge; it now reads the edge in the
  popup's frame. Remove this entry after two clean pipelines; a third failure means the
  cause is another.
- open 2026-11-12 (a member's 0.0.1 report, relayed by the maintainer): "importing a team
  on a PES 2021 save deleted every team, everyone is a placeholder". Not reproduced from
  the three files sent (the team file, a save named as the one before, the saved file;
  scratch `.tmp/report21/`): the saved file equals the earlier one but for teams 876 and
  886, both holding the file's 23 players, and 876 holds what the command line's `import-toml`
  writes (no difference in `diff` or in the team's export); the teams that were
  placeholders (156 of 220) were so before. The saved file is two bytes longer than the
  earlier save, which fits a serial (the writer's Windows account) one character longer,
  not read out: another account or the game wrote it last, or the earlier save sent is
  not the one that was open. Waits for the member to reproduce on 0.0.2 with its report button.
  One thing found on the way is in `docs/maintainer/questions.html` (the manager id a team file brings
  onto another team).
- resolved 2026-11-12 (the lead, writing the referee export page): the Export upgrader's
  spec and `referees.rs` said a conflicting referee folder is "left as it was", copied at
  its old paths for the author to repair, where EU-REF-03 and 11 write nothing. Both hold:
  the finding is an Error, so nothing is written, and "left as it was" is the run's plan,
  which keeps the folder's old paths so later findings name the files where they are. The
  spec's paragraph and the module's comment now say so; no behaviour changed.
- open 2026-11-11 (the worklog's collapse) — What the collapse left for the lead, from its
  report (`.tmp/sidelead/sk_wl.report.md`): (1) the closed issues' in-game results, Tests 2,
  4, 4b, 4c and 5 (the BootsList and GloveList rows, the referee collars on PES 17 and 21),
  are in git history only (`git show 85d6d966:docs/WORKLOG.md`): check the spec holds each,
  and move in what it lacks; (2) five rulings recorded only in the worklog get their
  DECISIONS entries (the PoC branch's `4cc_90_test`, 0.0.2-chk's checkpoints, 0.0.2b in the
  PoC bundle, the bats' `quick_` names, 0.0.2-acc's archive rule); (3) states to settle:
  open children under closed parents (ux1, poc12 slice 4, 4c's r2, f4a-c), items open though
  recorded done (k5b, k5b2, 8.36c4, f1e to f1g, poc13's asks), closed items still naming
  something open (8.36b, 20.6a1), the tags 8.9m and 0.0.1g each used twice.
- open 2026-11-12 — the VPS's disk during a whole-crate run is not measured. A diff run's
  `save_editor` half filled the disk from 19.5 GiB free (floor 12 then), and a sampled half
  of three mutants peaked at 23.6 GiB in `~/studio-mutants/tmp`, so `REMOTE_FREE_FLOOR` is
  26 GiB now (`DECISIONS.md`, "The VPS free-space floor for a mutation half is 26 GiB").
  That covers the two build copies. A whole run adds, over days, the scratch folder each
  caught mutant's test leaves in the same folder (28.6 GB of them on 2026-10-22), which the
  floor does not count, and nothing bounds the folder itself any more (DECISIONS 2026-11-12
  "the VPS folder's 30 GiB budget goes"). To do at the next `just mutants <crate>`: sample the host beside it
  (`.tmp/process/vps_peak.sh <log> <out file>`, read-only, 30 s; lengthen its 60-minute
  limit) and stop the half the RUNBOOK's way if the disk nears full.
- open 2026-11-12 (the readability findings): `spec/aatf_rules.md`'s sentence that a VGL25
  goalkeeper rated A at GK is still told of an unused A position comes from reading
  `crates/libs/aatf/logic/logic.rhai` (`a_positions` makes no goalkeeper exception): no
  fixture asserts that suggestion. A test with the ruleset's next change.
- open 2026-11-12 (the readability findings, item 9) — the comments of a kit's generated
  `config.toml` (`crates/libs/kit_config/src/toml_form.rs`, `to_toml`) give the old editor's
  ranges, which the game's fields can exceed (`x  # 0-14` in a field of 0-15). The two that
  stock configs do exceed are corrected (the collar's, per PES version, and the chest
  number's `y`); whether each of the others follows the old editor or its field is settled
  with the Kit config editor's plan (Phase 12), from what the game draws past the editor's
  limit.
- open 2026-11-12 (0.0.2-log, `.tmp/0_0_2/poc5/redirect_probe.bat`) — started from `cmd` or a
  bat, the exe's output can be neither redirected nor piped: `UTW.exe --version > file`,
  `| findstr` and `2>&1` each received 0 bytes, where Git Bash and a Rust `Command`, which
  hand the handles over themselves, receive every line. So `save-editor diff ... > report.txt`
  typed at a `cmd` prompt writes an empty file. `plans/core/architecture.md` "Windows
  subsystem" names only the interleaved prompt as the arrangement's cost. Unmeasured:
  PowerShell. `--log-file` covers a tool's findings; a `diff` or `--json` to a file from
  `cmd` has no route yet (a console-subsystem shim beside the exe is the usual one).
- open 2026-11-23 (the Export upgrader's view) — its draft's More menu stands flush on the
  arrow, where the Compiler's draft leaves 2 px; the build's shared `menu_above` gives 2.
  When the upgrader's toolbar slice is briefed, its draft takes the 2 px (a
  `ui_decisions.html` card with both pictures), or `menu_above` takes a gap per caller.
- open 2026-11-12 (0.0.2-upd, the look in the built exe) — a dialog whose button row is
  wider than its body draws the buttons out past the body's left edge
  (`widgets::modal_buttons` lays them out from the right in the body's width). Seen on the
  update offer with one-line notes and versions long enough to wrap, where the notes' box
  also hugs its text instead of spanning the dialog as the draft's does; a release's real
  notes fill the width and show neither. A fix would measure the row and widen the body.
  `Update ready`'s sentence now wraps with two words on its second line (the dialog is as
  wide as a fresh one, about 600 points): a look for the maintainer.
- open 2026-11-11 (the census index, `.tmp/sidelead/sk_census.report.md`) — (1) 603 upgraded
  or Studio FMDLs still carry an old Common path in their strings that no texture reference
  uses (a raw-bytes search finds 8,783 files, the references 8,180): which tool leaves the
  string, and whether the game reads it, is untraced. (2) `spec/export_upgrader.md`'s "6,053
  FMDLs in export folders" names no folders, so no query reproduces it (the index reads
  4,793 with repositories skipped): restate it from the index. (3) The team library holds no
  `Collars` FMDL at all: the 146 counted for S-poc12s4 were copies under this repository's
  `.tmp/`, so the issue below stays unmeasured on real exports.
- open 2026-11-11 (S-poc12s4 r1; `export_upgrader` `kit_magic.rs` `respell_outputs`) — The
  Export upgrader respells the old kit token only below `Players/`, `Boots/`, `Gloves/` and
  in `Common/`, and `kit_token_old` follows it there. Whether Red's kit magic also rewrote
  tokens in collar models (`Collars/`) is unchecked: if it did, an old export's collar
  naming `u0???pN` compiled right under Red and ships the token verbatim now, and the
  upgrader should respell collars too (and `kit_token_old` follow). Check Red's
  `path_id_change` callers on `main`.

- open (2026-11-11) — `scripts/derive_savefile_schema.py` no longer reproduces `schema/fields.rs`
  (from `Tools_4cc/4ccEditor-1` or `DBC7-Editor`): the file was edited by hand after it was
  generated (the serde derives, `ABILITY_FIELDS`, two doc comments). The version tables
  `pes15.rs`..`pes20.rs` still regenerate byte for byte (checked 2026-11-11, with `coaches`).
  Either the generator emits those additions, or `spec/pes_savefile/codec.md` stops naming
  `fields.rs` among its outputs.

- open (2026-11-11) — `studio_core` settings' `read_back == desired` guards (`save`'s in-place
  edit and `fresh_from_template`) survive mutation (`mutants-diff ee80b0ac`, line 224): the
  shape their tests use, a section written inline under a comment, makes an edit that does
  not parse at all, so only the `Err` arm is tested, and none of ten other shapes tried
  (dotted keys, an array of tables, inline, nested and dotted tool tables) parses to other
  values. The guards stay, as the net against writing a file `load` refuses; a test lands
  when a shape that reaches them is found.

- open (2026-11-09) — One Compiler for every export kind (DECISIONS 2026-11-09): before Phase
  9's acceptance is written, the plans naming the Balls and Stadium compilers (about 30 files)
  are respelled for the Compiler and the Balls arranger, and the crate layout is chosen.
  The layout waits on whether guardrail 3 (`spec/core/architecture.md` "Workspace
  guardrails") is amended for test cost, which the maintainer wants only if worth it:
  measured mutation cost per mutant (build + tests) is about 217 s for `save_editor`, 91 s
  for `pes_savefile`, 50 s for `team_widgets`, 23 s for `team_compiler`. First,
  `.tmp/lead/test_times.py <crate>` (nextest per-test times, the libtest wall time with and
  without each binary's slowest tests) over those crates, on an idle machine, to see whether
  a few tests dominate; a split pays only where slow suites are many tests, not a few. After
  0.0.2-poc ships (the maintainer: the PoC first).

- open (2026-11-09) — A member's `Collars/` FMDL is packed as it is on PES 18-21
  (`processing/mod.rs` `TaskKind::Collar`, `Engine::Fox if is_fmdl => Ok(bytes)`): its texture
  table is never read, so a `team_folder` directory keeps the placeholder instead of the team's
  ID, and since poc12 slice 2 so do `team_common_folder/` and `common_folder/` (the upgrader
  keeps a collar's paths as written, so it writes neither). Found by slice 2's sidekick; not
  measured how many real collars name the Fox common directory. The fix reads a collar like a
  package part (the short forms, then the ID replacement), or refuses the Common directories
  there.
- open (2026-11-07) — The Compiler's recovery manifest (`plans/team_compiler/pipeline.md`
  "Recovery manifest", the maintainer's request): early, since only DLCs compiled after it
  ships carry it. One slice in the writer (each team and referee export's manifest and the
  files it packs beside it, the round trip read back from a compiled CPK); no in-game check
  (every version ignores unread files). The balls, stadium and billboards manifests come with
  their kinds, and the Decompiler (`plans/decompiler.md`, a stub) waits for its own phase.
- open (2026-11-07) — Repro bundles in every tool (`plans/core/architecture.md` "Repro
  bundles", the maintainer's request after the 0.0.2-se19 report). To slice before briefing:
  (1) the button and dialog drafted in `docs/prototypes/` and looked at by the maintainer
  (done 2026-11-07: `index.html` `drawReport`, the sidebar tail's "Report a problem",
  looked at by the maintainer, its wording theirs; the icon is the warning glyph until
  one of its own is drawn);
  (2) `studio_core`'s journal type and bundle writer, with the Save editor's journal taken at
  `Session::changed` and its file operations, its inputs kept, and the test that every model
  change goes through the journal (done 2026-11-07, SWE-2; reviewed to 2026-11-08: SWE-2 r1-r11,
  Astra X1, Argon A1-A3, reworks to `eef1a9e1`; the loop's `mutants-diff 188cf158^..` done
  2026-11-08: 60 mutants, 3 missed, all Write's `NotFound` guard, `generate.rs:333`, tested in se_kills_2a); (3) `utw save-editor replay` (the plan's "Replay reads a typed
  change", DECISIONS 2026-11-08), in three slices: (3a) the player side end to end below
  the command: the step's `op` in `studio_core`, written and read back from a bundle, the
  kept files' indexes, the open's and the save write's ops and every player row's (serde on
  `PlayerDiff`, the face run's unused bits, `pes_savefile`'s apply and a file's salt), and
  replay run over a bundle a `Session` recorded (done 2026-11-08, SWE-2, one rework;
  Astra first, `.tmp/repro/rulings_3a.md`, round A-repro3a r1: 4 test gaps accepted, their rework
  landed; open: the SWE-2 loop over 3a and it, then Argon);
  (3b) the `replay` command, its messages and SE-REP-01 to 06; (3c) the team, roster and
  tactics rows, `Session::apply_patch`'s field lines, and a test per `Session` mutator that
  replaying its journal rebuilds the model. 3b and 3c wait until a member's bundle needs
  them (the maintainer, 2026-11-08, DECISIONS): until then a bundle is read with
  `save-editor diff` of its kept input against its written save beside the journal; (4) the
  "Report a problem" button and dialog in the shell (the draft's `drawReport`), each tool's
  current state through the tool interface, and replay comparing that state last: slice 2
  built the writer and nothing calls it outside tests yet, so no member can write a bundle
  until this lands, and it lands after (3) so the first bundles members post replay; (5)
  earlier sessions, after 0.0.2 (the maintainer, 2026-11-17): each window's crash monitor
  writes the session to the data folder as it happens, the last five kept, the saves
  decrypted, and Report a problem's unticked box adds them (`plans/core/architecture.md`
  "Repro bundles", "Earlier sessions are kept on disk"; ST-REP-06 to 12, SE-REP-07; the help
  page's line). It replaces the panic hook's bundle, which was never built; (6) the Compiler
  and the Export upgrader; the rest as each tool is built. Before (5), on its own after
  0.0.2 (the maintainer, 2026-11-17): the crash monitor's start tried once more after a
  failure at once (`plans/core/architecture.md` "Diagnostic logging", "A failed start is
  tried once more"; ST-RUN-13). Each slice gets SWE-2, Argon and Astra rounds (a new `pub` interface across
  crates).
- open (2026-11-06) — Whole-crate mutation runs on the VPS alone (`RUNBOOK.md` "Mutation
  runs and the VPS", the maintainer's directive). Slice 1 done 2026-11-09 (SWE-2 sidekick,
  `.tmp/mutation_vps/brief_vps_only.md`; `mutants_test.py` 79, 9 perturbations red): a whole
  run refuses without a host, runs every mutant there, its extra kinds first in the same
  service (`--remote-extra`) and fetched with the rest; the launch record says whether a
  local half exists, which the summary, `collect` and `local_refusal` read; the wait holds
  no run lock, and `mutants-diff` runs here while the VPS is busy. Its review (tier 3: one
  SWE-2 round, five parts, 2026-11-12, "S-vps r1" in `.tmp/duck_rulings.md`): 8 of 27
  accepted. A `mutants-collect` from a tree with no launch record counted that tree's stale
  results as the run's local half and is now the remote half alone; four tests tightened;
  three RUNBOOK sentences corrected (`9e38f13e`). The SWE-2 round over those fixes and the
  floor's change ("S-vps f1"): one concern, a message, accepted; the review is closed.
  Then, as a second slice: a stopped run
  resumes where it stopped (the VPS needing an update mid-run is the case), its tested
  mutants kept, never retested and never lost from the summary; first check whether
  cargo-mutants' `--iterate` (skips mutants an earlier run caught) does this for a half
  stopped mid-run, before writing our own. A vital script: SWE-2 rounds over the changed
  functions
- open (2026-11-06) — files past a thousand lines that recent diffs grew. `team_compiler`,
  in 8.36c's review fixes (review S-36c r3): `src/output/writer.rs`'s inline test module
  (~2,100 lines) and `tests/cli/models.rs` (~3,150). `export_upgrader`, in 0.0.2-poc6 and
  poc5/7 (reviews S-poc6c r1, S-poc57u r1): `src/players.rs` (2,993, its inline test module
  ~1,730), `tests/cli/players.rs` (1,248), `tests/cli/carried.rs` (1,178) and
  `src/materials.rs` (2,321). `save_editor`, in se_kills_2a: `src/view/tests.rs` (the
  helpers hub, 1,688), whose new helpers had no smaller home; `view/tests/live_aatf.rs`
  (2,682) and `view/tests/strip.rs` (1,807) grew only a `mod` line, their new tests in
  child files. Split each by
  concern as pure moves (the `fmdl` and `aesthetics_patch` splits' way: a parent keeping
  the shared helpers, one child file per concern, a helper used by one child moved with
  it), with no cargo-mutants run holding the crate.
- resolved (2026-11-06) — `zip` 9.0.5 fixed Dependabot's alerts. `cargo deny check advisories`
  runs in no gate (fails on `pulldown-cmark` 0.13.4, five unmaintained crates): Dependabot watches

- resolved (2026-11-01, 0.0.2-left01) — the Ruleset editor's tier handle was no defect: its
  drag source's rect is the glyph label's own, so the label never wins a press; a drag from
  beside the glyph is now tested
- resolved (2026-11-01, 0.0.2-left01) — `run_tests.py` keeps its timings in the target folder
  cargo names, so a worktree building into another tree's target passes `just test`
- resolved (2026-11-01, 0.0.2-left01) — `make_candidate.py` puts the candidate in
  `4cc-studio-temp` beside the main tree, found through git, from any worktree
- resolved (2026-11-01, cv40) — three `save_editor` view tests raced a worker's reply under load;
  `a_range_turned_by_the_wheel_is_checked_once_it_has_rested`'s cause stayed unconfirmed
- open 2026-11-01 — the vital scripts' first reviews (`RUNBOOK.md` "Review rounds", checkpoint
  (e); DECISIONS 2026-11-01): each reviewed whole by SWE-2 rounds once, in this order:
  - [x] the mutation runs' `mutants.py`, `mutants_diff.py`, `extra_mutants.py` (and
    `mutants_test.py`): SWE-2 rounds 1-3 (S-scripts), the loop ended (`bde2eafb`); then
    `scripts/`' root sorted by job (`6f37bf18`)
  - [ ] `run_tests.py`, `test_runner.py`, `remote_gates.py`
  - [x] `release.py`, `build_remapped.py`, `make_candidate.py`: SWE-2 rounds 1-2 (S-release),
    the loop ended (`82a5ca93`)
  - [ ] `scripts/census/`, `scripts/library_parity/`
- open 2026-11-01, 0.0.2 — two more resamples run on straight alpha (found by the cv18-cv26
  rework, `aeb8c917`): `export_upgrader` `carried.rs` `at_128` and `team_compiler`
  `kit_layout.rs` call `dds_convert::resize`, which fringes an edge against transparency the
  way the event logo did. Check first whether either input carries partial transparency on
  real exports, and what Red's output is there (library parity), before moving either to
  `resize_straight_alpha`
- open 2026-10-30, Phase 20 — a tier id's rename carries its key in the medal systems alone
  (`plans/ruleset_editor.md` RE-GUI-17), so renaming a tier that a special's `when.tiers`, a
  per-tier effect map or another tier's `counts_as` names leaves an "unknown tier id"
  problem the bottom bar lists until the tier is renamed back (found at cv8c: renaming
  Autumn 26's Gold makes Save refuse four specials). In 0.0.1 the specials cannot be edited,
  so renaming back is the only way out. Revisit with 20.6a2b/20.6b: carry the rename through
  every reference, or keep the problem as the member's cue.

- open 2026-10-30 — two member-facing errors still number a roster slot from 0, where the
  player list numbers it from 01 (found reviewing d1b2, whose rule covered the badge's failed
  check): `pes_savefile` `texport.rs`'s `roster slot {slot} holds player {roster}, the
  record is {player}` (a Texport import's refusal) and the Save editor's
  `session/player_ops.rs` `team {team_id}'s roster slot {slot} is empty`. Each takes the
  list's `place NN` the way `aatf`'s `MissingPlayer` now does; ride the next Save editor
  text slice

- open 2026-10-28 — `studio_core`'s first-run tests depend on the portable hint's width
  (8.29b): a hint of about 100 characters made three of them fail as if a click were lost
  (`the_first_run_dialog_writes_the_settings_file_to_the_user_folder`,
  `the_portable_choice_writes_next_to_the_program`,
  `a_failed_write_keeps_the_dialog_open_with_the_error`), with the old hint back they passed;
  78 characters pass. The dialog's layout moves with the text and the tests' two-frame settle
  no longer finds the button. Fix when next touched: settle until the layout stops moving
  (bounded), or click the button by its id, so a rewording cannot pass for a broken click
- open — `plans/core/gui.md` "Dark/light theme" says a disabled selector's selected entry
  "keeps the selection's colours, faded, as the drafts' locked formation picker does
  (`.fsel.off`)", but the draft dims it (`.fsel.off .sel.on { background: #2b2b2b; color:
  #5a5a5a; }`, and `.rate.off .sel` alike). The rate rows follow the draft since the frames
  rework of 2026-11-17; the formation picker and the plan sentence are to be checked against
  the draft and made to agree (found at that rework's review)
- open — Studio Web (phase 18): `theme::install_fonts` is native-only, so on wasm the named
  families the views draw in, `icons` (the sidebar) and `label` (the Tactics pitch's formation
  name, 0.0.1x4), are unbound, and epaint warns and draws no glyph for them (read in epaint
  0.37's `font_definitions.rs`, not run). The web frontend installs the same fonts when it
  starts (found by x4's sidekick, 2026-10-27)
- open — `plans/core/distribution.md` "Changelog and version display" was never moved to the
  spec although the version label it specifies is built (status bar, About, `--version`, the
  log's first line): `version_label`'s doc comment pointed at the spec heading that does not
  exist, across a line break the pointer check did not read; 0.0.1k's rewrap exposed it and
  the pointer went (2026-10-26). The built part moves to `spec/core/distribution.md` under
  the same heading, the changelog's unbuilt rest staying in the plan
- open — a save whose one player name is not UTF-8 is refused whole (0.0.1i's census,
  2026-10-25): `pes_savefile`'s player codec errs (`codec/player.rs`, "player name is not
  UTF-8"), so `check-aatf` and every other command refuse
  `Documents\KONAMI\Pro Evolution Soccer 2017\save_old\EDIT00000000 testteams` (a test save,
  the census's copy `.tmp/release_0_0_1/0_0_1i/census/saves/5e3d82ea3c9469ee` on side-lead B's worktree).
  Not diagnosed: which player and which bytes (a name cut inside a multi-byte character by an
  editor is the guess, unverified). Whether the game itself writes such names decides between
  a tolerant read and the refusal; one save of 424 holds one
- open (the Ruleset editor's half fixed 2026-10-23, 20.6a1's rework 2; the Save editor's
  waits) — a test can open a real file dialog on the developer's desktop (the maintainer saw
  four "Save As" windows, `.rhai` suggested, on 2026-10-23): `mutants-diff 8aeb0193`'s mutant
  `menu_item -> true` clicked every menu item each frame, and the views the tests build with
  `RulesetView::new()` carry the real `rfd` pickers, so Save as… opened a native dialog and
  the test hung until the run's timeout killed it (TIMEOUT, not caught). CONTRIBUTING: a test
  fails, it never hangs, and never reaches the developer's own environment. Fix (20.6a1's
  review rework, after 8.11): under `cfg(test)` the default pickers panic ("a test reached a
  real file dialog: stub `open_dialog`/`save_as_dialog`"), so such a mutant fails at once
  with no window. The Save editor calls `rfd::FileDialog` straight from its click handlers
  (`view/mod.rs`, `view/menus.rs`): the same exposure under a mutant, and the same fix
  there (pickers behind fields, panicking under `cfg(test)`), in its next rework
- open (SWE-2 S3 on 20.6a2a, 2026-10-24; for the Ruleset editor's resumption after 0.1.0) — a
  Ctrl+Z pressed while a tier's drag handle is held restores the tiers under the drag, whose
  payload is the row index: the release moves whichever tier now holds that index, and panics
  when the undo left a shorter list (`sections/tiers.rs`, the `dnd_drag_source` payload). A
  two-handed gesture, rejected for 20.6a2a as an edge case; the fix to weigh then is the
  tier's id as the payload, resolved at release
- open — tests built on the default settings may reach the real install (Astra A1 on 8.9j1,
  2026-10-24: a Save editor FPC test's `ToolContext` from `studio_core::Settings::default()`
  kept the default `pes_folder_path`, and `Tool::view` with a save open started the portrait
  worker against it; the Save editor's tests fixed in 8.9j1's rework 2). The other crates'
  tests that build a `Settings::default()` (`team_compiler`, `export_upgrader`,
  `ruleset_editor`, `studio_core`'s shell harness) are not audited yet: each one that reaches
  a path setting gets a scratch path, before the 0.0.1 cut
- open — repetition in the code (the maintainer, 2026-10-23, after the writer's review let
  `aatf`'s `writer/mod.rs` keep one six-line shape over 40 times): CONTRIBUTING's "The third
  copy is a function" had no check, so neither the sidekick nor the lead's sweeps applied it.
  A shape detector (`.tmp/topics/dup_audit/clones.py`: identifiers, strings and numbers blanked,
  declarations and table rows set aside; a run of 4 shaped lines found 3 times in a crate, or
  of 10 found twice, is a cluster; `--across` for runs in two crates) measured HEAD
  `8aeb0193`: about 260 non-test clusters (pes_savefile 70, aatf 30, team_compiler 31,
  save_editor 23), 81 across crates (`fmdl`'s and `pes_model`'s `ops/split/build.rs` share
  about 180 lines), and some 65,000 repeated shaped lines in tests. The work: (1) SWE-2 rules
  every non-test cluster, five groups (`.tmp/topics/dup_audit/swe_dup_<a-e>.md`, findings beside
  them), the lead rules the findings; (2) one rework brief per crate for the accepted ones,
  between the slices in flight, each gated and measured by `mutants-diff` like any rework,
  `writer/mod.rs` first after 20.6a1; (3) the test clusters, the same way; (4) the detector
  moves to `scripts/` with a `just clones` recipe, the thresholds the audit's rulings settle,
  and a committed keep list (each kept cluster's shape and its one-line reason, which the
  detector then skips), and runs at two points (the maintainer's question of 2026-10-23):
  in `just gates`, failing on any cluster the keep list does not hold, so a diff that adds a
  copy is fixed or ruled before it lands, with no reliance on a reviewer noticing; and at
  every phase's close, whole-crate, in the design-health pass (`RUNBOOK.md` "Closing a
  phase", beside `just mutants` and `just citation-audit`), where the keep list's entries for
  the phase's crates are read again, since a kept pair that grew a third copy is no longer
  a pair. `AGENTS.md` and CONTRIBUTING "Code style" say both. State: (1) done, the rulings in
  `.tmp/topics/dup_audit/rulings.md` (group IDs `A-F<n>` to `E-F<n>`, the lead's own `C-L<n>`; keep
  that file until (2) ends); (2) the first brief, `aatf` and the Ruleset editor's summary,
  `.tmp/topics/dup_audit/brief_rework_aatf.md`, done 2026-10-23 (SWE-2; `aatf`'s source clusters 30
  to 21, the summary's 7 to 5; one `push`, `check_empty`, `check_unique`, `id_known` and
  `check_id` in the checker, `push_set` and `push_some` in the writer, the new `pub`
  `PlayerValue::key` spelling a `ranges` key once for the writer, the reader and the
  summary; the writer's bytes and the summary goldens unchanged), reviewed 2026-10-24 (SWE-2
  S1 3 of 6, Astra A1 2 of 2, S2 none; the lead's fixes `ff88177d`: `id_counts`,
  `number_list`, `check_color_code`, the summary's `group_label`; `mutants-diff b8ed279f`:
  `aatf` 84 caught, none missed; over the fixes, `mutants-diff 86d79c92`: none missed), then
  the other crates' briefs; reworks touching `fmdl` or `pes_model` also run `just bindings`, and the first of them adds each mirrored file's header
  naming its twin (`spec/libs/format_crates.md`, "`ops/` are format-native transformations":
  the two `ops/split/build.rs`, `ops/split/combine.rs`, `ops/vertex_enc.rs`, `check.rs`, and
  `model/validate.rs`); (4)'s check also flags a keep-list entry that no longer matches, so a
  change to one copy of a kept pair stops the gates until both copies agree or the entry is
  re-ruled
- open 2026-10-23 — a command's `Report` (`events.rs`: its findings sent as events, the worst
  severity deciding the exit code) is copied in `save_editor` and `ruleset_editor` (20.5a),
  about 40 lines each, the Save editor's with one more method (`output`). Two copies are not
  yet a pattern; when a third tool needs it, it moves to `studio_core` and the copies go
- open 2026-10-21 — the 4ccEditor source audit's rows that need a measurement or a check of
  built code (the rest are in `plans/save_editor.md`'s parity table and "Editing UI", and in
  8.9b1's rework): (1) a custom skin of 7 on PES 18+ becomes 0 in 4ccEditor (`main.cpp`
  `show_player_info` and the squad load), where `spec/pes_savefile/model.md` `SkinColor::Preset`
  and the cross-version conversion reset it to 1: which one the game shows as the intended
  skin, checked in Edit mode; (2) 4ccEditor clamps a shirt number to 231 below PES 19 (its
  number field and the squad load), where the info strip takes the field's bit width: why 231,
  checked in game; (3) 4ccEditor's ALL hides players on no team and its team list hides a team
  whose first slot is empty (`pes20.cpp`), lists both in record order: compare with 8.9a's
  selector and list, and rule. (4), the motion randomizer's ranges, is in the plan
  (2026-10-22: from 1 to the measured tops, never the dribbling motion; DECISIONS)

- open 2026-10-19 — the maintainer saw AMD driver timeouts during the BC7 encoding tests
  (`dds_convert`'s `gpu.rs` tests, 16.x a). Logged so far: one Windows GPU reset (System log,
  event 4101 "Display driver amduw23g ... stopped responding and has successfully recovered")
  at 03:16:41, while 16.x's probe (`.tmp/phase_16/16x/gpu_probe/`, edited 02:50-03:09) dispatched whole
  mips in one submission, before `gpu.rs`'s bounded slabs (`SUBMISSION_PIXELS`, c68ef99 at
  04:36); none logged since. Still to rule out: the tests' own load, each GPU test building its
  own device and `cargo test` running them in parallel, several worktrees' gates at once, a
  `dds_convert` mutation run's parallel jobs. Parked (the maintainer, 2026-10-19): no
  investigation now; during every later BC7 test (16.x b, a `dds_convert` mutation run) check
  the System log for event 4101 (`Get-WinEvent -FilterHashtable @{LogName='System'; Id=4101}`)
  against the run's times, and if one recurs, reproduce (the GPU tests looped, alone and in
  parallel processes) and serialize the tests' device use or lower the budget, whichever the
  measurement points at.

- open (2026-10-15, 5.6c census) — PES 2014 files read as PES 15 and then fail with a
  misleading error: `SaveContainer::decrypt` labels a PES 2014 `EDIT.bin` or `TEXPORT0N.bin`
  `Pes15`, and the decode fails on a player name (not UTF-8) or a tactics byte. Loud, never a
  wrong write, but it should be refused as PES 2014 (unsupported): measure what separates the
  two versions' containers (PES 2014's EDIT is 4,799,651 bytes, its texport 5,389,806; no real
  PES 15 texport is on the machine) and refuse it by name. `.tmp/phase_05/5_6/audit_notes.md`.
- open (2026-10-16, 5.6b mutation runs) — three GUI views are placeholder lines no test reads:
  `save_editor`'s `view` and `settings_view` and `team_compiler`'s `settings_view`
  (`lib.rs`), each a survivor when replaced with `()`. Their tests come with the views they
  stand in for (the Save editor's window, the Compiler's settings editor, Phase 8), with
  `team_compiler`'s headless `view/mod.rs` harness.
- open (2026-10-15, Astra review B.A1 of 5.3b) — nothing refuses an `output_folder_path` that
  resolves to the PES folder's `download/` (or inside the PES install): a degraded or
  `--no-deploy` run then installs into the game past every deployment check, and a savefile
  failure's rollback promotes the new CPK over the old one it just restored. Fix when the
  settings get their validation pass (Phase 8's settings view at the latest): refuse such a
  path on the command line and in the settings view, comparing the canonical paths.
- open (2026-10-11, Phase 5 Acceptance) — a referee export's `settings.toml` reaches no table or
  patch: a referee's look is the template's `RefereeAppearance.bin`
  (`team_compiler/referees.md`), whose rows nothing writes, and `team_compiler/referees.md` has
  `fpc_off` apply its settings preset to a referee on PES 18-21. Settle with a Fox refkit (the issue on a referee's body under `fpc_off`): which
  route a referee's settings take (the aesthetics patch, as team 999, since 8.36 writes no
  player tables, or the template's table), and TC-REF-19's outcome then; with it, lift
  `settings_id_messages`' team-only gate (`validation.rs`).
- open (2026-10-11, 5.0c; deferred until the cup next plays a pre-Fox version) — what Red does
  on PES 15-17 that Studio does not (`.tmp/phase_05/5_0c/xml.md`, `checks.md`), most noticed first:
  - a member's `face.xml` that does not list `hair_high_win32.model` (1,248 of 1,263 such
    folders on the maintainer's machine): Red packs the file and the game loads it by its fixed
    name (inferred), while Studio reports `xml_model_unlisted` and leaves it out, so the head
    compiles bald
  - a generated `face.xml` lists `hair_high` as a `parts` entry, while Red leaves
    `hair_high_win32.model` out of the xml and packs it under its own name; the in-game effect
    is unknown
  - a `.mtl` Common path that only an installed CPK supplies keeps its `XXX`, while Red's
    `txt_id_change` writes the team ID in every `.mtl` (the answer to the question
    `messages.md` leaves for the pre-Fox parity step)
  - a Common model or `.mtl` that a member's xml names and only an installed CPK holds: Red
    accepts it, Studio reports `xml_model_not_found`
  - a WESYS-wrapped `face.xml` (48 on the machine): Red unwraps it, Studio reports `xml_broken`
  - a `.mtl` Common path whose team folder is not 3 characters: Red reports an Error, Studio
    nothing
  - a `<model>` with no `material`: Red reports an Error, Studio writes it (TC-XML-01)
  - `mtl_state_nonrecommended` misses Red's `alphablend 0` with `zwrite` other than 1 (Info)
  - two models packing to one name (`hat.model` beside `oral_hat_win32.model`)
  - the dummy team-ID respelling of file names (`Common/u0XXXg1.dds` named as
    `.../common/XXX/u0XXXg1.dds`, 9 exports): the file and the path are respelled together or
    not at all
  - DECISIONS 2026-10-09 sends the xml's `<dif>` on PES 15-17 beside a `face_diff.bin`, while Red
    lets the `.bin` replace it; the two differ in 36 of 53 such folders
  - `boots_edit.model` and `glove_edit.model`, the Edit-mode models (2 found), and a member's
    own `glove.xml`
  - a member's `face.xml` path in a game-path form other than `model/character/face/common/…`:
    Red reports an Error for a model or `.mtl` (a Warning for a texture), Studio keeps it with
    `xml_path_unchecked` (W); DECISIONS 2026-09-12's "Red waved unresolvable game paths through
    silently" holds for that one form only (`messages.md`, the `xml_path_unchecked` paragraph)
- open (2026-10-11, 4.y-mattable row 27; deferred until the cup next plays a pre-Fox version) —
  the cup wiki says a `.mtl` splits a path at spaces, and 212 of 60,027 texture names in real
  FMDLs hold one (`Kit 1_u0XXXp0.dds`), so a Fox model converted for PES 15-17 may write a
  `.mtl` path the game cuts short. Unverified in game.
- resolved 2026-11-20 (SWE-2, TC-REF-28): a PES 17 referee whose only model is a
  `.model.common` link got no face, while the converted Common model was written into the refs
  CPK with nothing naming it. Since a referee with no face model gets the blank face, his
  `face.xml` names the link's Common model, as a team player's does.
- open (2026-10-11, S19.1-B3) — which material files a glTF in a player or shared folder's
  subfolder reads (`model_format.md` "Name matching" is written for one flat folder; the
  `.mtl` search's two folders, the file's then the model folder's, are the likely answer):
  for Phase 7's plan pass, when glTF sources are planned.
- open (2026-10-09) — a Fox referee's body under `fpc_off` waits on a Fox refkit, an FMDL
  body the maintainer will make ("We'll make one later"; `team_compiler/referees.md` "The referee
  body"): then the Fox referee template ships it and a step merges it into an `fpc_off`
  referee's face package. Until then `fpc_off` applies its settings preset alone on Fox.
- open (2026-10-09) — per-kit face packages on Fox wait on FoxDen (`model_format.md`
  "Kit-dependent assets", DECISIONS 2026-10-09): each kit number gets a face package of its
  own, merged from that number's variants of any file kind, once FoxDen loads one per kit
  picked; the layout it reads is decided with FoxDen's work, then a step in the phase open
  at the time builds it and retires `kit_variant_model_fox`. FoxDen is a work in progress.
- open (4.y-fix17's review, 2026-10-10) — a PES 17 `.model` collar compiled for PES 15 or 16
  keeps its `uni_collar` material, packed as is when the pre-check does not flag it (PES 16)
  or moved onto the skeleton when it does (PES 15), while those versions' `uniform.mtl`
  defines no `uni_collar` (the FMDL conversion names `uni_shirts` alone for them, DECISIONS
  2026-10-09). What the game draws for a collar material its `uniform.mtl` lacks is unknown:
  an in-game check on PES 16 (the F: install), and if it draws nothing, the same-engine
  collar gets the version's names as the converted one does.
- open (converge census, 2026-10-09) — a pre-Fox face of several `.model` parts converted
  for PES 18-21 fails to merge: each part's conversion generates its own skeleton (the bones it
  uses) and the face merge refuses the differing skeletons (`skl_merge_conflict`,
  `skeleton=differs` or a `bone=`), nine referee folders in seven of the cup's current referee
  exports (FAA, germanbro, coolguy, geodude, donte, ...). Red never converted a `.model` for
  Fox, so these referees were never on PES 21; the fix is the converted parts of one folder
  sharing one generated skeleton (the union of their bones, or the target's full table), a
  `model_convert` and `fmdl` merge question for Phase 7, when the conversion work resumes.
- open (converge census, 2026-10-09) — on PES 18-21 a member's own `face.xml` is ignored
  (`xml_ignored_fox`), so a pre-Fox referee folder with Red-packed names
  (`oral_fixed_kit_win32.model`, five `.mtl` files) converted for Fox pairs each model with
  the `.mtl` search's choice (`apc.mtl`, the first by name) where the xml names `fixed.mtl`, and
  `model_material_undefined` drops the folder (ANF, four cup exports). The xml's `material`
  attributes could drive the conversion's pairing on Fox; a plan ruling first. Low priority:
  these referees are pre-Fox material.
- open (converge census, 2026-10-09; diagnosed as the plan's rules, no change) — the census's
  other refusals of the cup's referee exports: `common_file_disallowed` on `Common/refkit/`
  (a maintainer question, `docs/maintainer/questions.html`); `file_type_disallowed` on files two levels
  down (`Face/kit/`, `Face/face/`, a `Common_shared/` subfolder, a `.model` in a player's
  `common/`), which the plan's reserved-subfolder rule names and the Export upgrader flattens;
  `texture_not_pow2` on a 192x512 mipped texture (a maintainer question); ANF's
  `face_high_win32.model` naming `head_phong`, which the `face.mtl` his xml names lacks (a true
  defect of the export; `pass_through` compiles it as Red did). Three referee exports compile
  clean on both engines (Summer 26, `refs numbered`, Autumn 25 Final Boss on PES 17).
- open (found at 4.14b's review) — pre-Fox faces: two files of one face packing under one
  name (`hat.model` beside `face/hat.model`, two `.mtl` of one name in the folder and
  `face/`, or a member's own `dummy.mtl` when the dummy is added) fail the face task at
  `compile` (`folder_pack_failed`, folder dropped) while `check` reports nothing. No finding
  code exists for it; a planning check naming both files would make `check` agree. No real
  export is known to do it.
- open (found at 4.14b's review) — the pre-Fox gate names an `ingame_face` marker as
  `<folder>/ingame_face` (without a `.txt` it may carry), since `PlayerFolder` keeps no
  marker file's name; the message is still findable. Gone when 4.14e compiles it (links
  compile since 4.14c1).
- open (found at 4.14c1's review) — pre-Fox combined face: the player's own texture of a
  stem the linked `Faces/` folder also holds, with other bytes, is `merged_texture_conflict`
  and drops the folder, as on Fox, while his own model or `.mtl` of a shared file's packed
  name replaces it (the plan's copy with local files on top). On pre-Fox no merged model
  makes the texture ambiguous, so the player's could win too; the plan's step 6 rule is
  engine-wide today. Safe as it is (an Error, nothing wrong written).
- open (found at 4.14c2's review) — pre-Fox Common: a texture link whose target only an
  installed CPK holds is still `common_link_missing` on PES 15-17
  (`installed_common_textures` is empty there; the Fox walk's stems are `.ftex` paths). A
  later step can read the installed CPKs' `common/character1/.../common/{team}/` `.dds`
  stems the same way.
- open (found at 4.14c2's review) — pre-Fox Common: with `pass_through` on, a `Common/` `.mtl`
  kept despite an eligible Error is left out of the deep pass's `.mtl` search (which cannot
  see the setting), so a model whose only `.mtl` it is gets `model_material_undefined` where
  `compile` could have built it. Conservative (drops, never panics); rare.
- open (found at 4.14c2's review) — pre-Fox: two `.model.common` links to one Common model in
  one folder (`legs.model.common` and `boots/legs.model.common`) give one `face.xml` entry,
  the first in file order, though their positions could type them differently. Fox merges
  them as one part too. No real export is known to do it.
- open (found at 4.14c1's review) — pre-Fox: a file a task does not use is still read and
  charged (a shared face's model or `.mtl` the player's file replaces, a shared boots
  folder's `.mtl` its model does not use): `TaskKind::files` lists every role. Harmless.
- open (found at 4.14b's review) — pre-Fox faces: every `.mtl` of a player folder is packed
  into the face, one no model uses included (a blank face too), as Red packs the whole
  folder. Harmless to the game; revisit if a census shows stray `.mtl` files are common.
- open (2026-10-08) — the PC blue-screened three times during a local mutation run (4.14c2's
  half, 4.14e1's and 4.14e2's; MEMORY_MANAGEMENT at the first), never otherwise: the two
  cargo-mutants jobs each build a copy of the tree, the heaviest load the machine gets now that
  `team_compiler` runs locally whole. The maintainer memtested the RAM (clean), turned XMP off
  after the second, and after the third set the paging file to 32 GB, the size it had before
  the crashes began. A killed run leaves its two `%TEMP%\cargo-mutants-4cc-studio-*.tmp`
  copies (3.6 GB each; `.tmp/lead/rm_mutant_copies.py` removes them, clearing the read-only flag
  a test leaves on one file). If a run crashes again at 32 GB, the next lever is one job
  (`--jobs 1` in `scripts/mutation/mutants.py`'s local config).
- open (2026-10-08, 4.14e3's plan) — pre-Fox: a hand-weighted model behind a `.model.common`
  link is listed whole by its Common path, not split (`spec/model_conversion/hand_split.md`
  "Pipeline integration"): the game then loads a body with hands on the body skeleton. Splitting it
  means packing a copy into the face. Build it when an export has one. Likewise (4.14e3,
  decision entry) a hand-weighted model whose `.mtl` is a Common file fails its folder with
  `model_conversion_failed` instead of reading that file for the split.
- open (found at 4.14e2's review) — pre-Fox combined folders: when a player's `.mtl` replaces
  a linked folder's of the same name and a linked model he does not replace used it, that
  model is listed with his `.mtl`, which the deep pass never compared with its materials
  (it checked the linked `.mtl`): a material his lacks is undefined in game with no finding.
  The combined face behaves the same way. Rare (two `.mtl` files of one name, the linked
  model kept); a fix checks the material names against the `.mtl` that is packed.
- open (found at 4.18's review) — hand auto-split: a split glove beside an authored glove
  of the same hand may not merge. The split prunes the bones no vertex of a glove uses, so
  a split `sk_hand_l` can lose its parent `sk_forearm_l` where an authored glove keeps it,
  and `fmdl`'s merge reports `skl_merge_conflict`, leaving the gloves out (seen with the
  tracer's glove beside the lead's fixture, whose bones are all roots; unmeasured on a real
  full-body model). A player holding both is unlikely; revisit if one appears (the fix
  would be the split keeping a used bone's ancestors, a `model_convert` change).
- open (found at 4.18's review) — hand auto-split: a player folder with a plain `Gloves/`
  link and a hand-split face part keeps the link plain (links combine by file names), so
  the shared folder still compiles under its shared ID while the player's gloves ID goes to
  the split gloves, its own task. An authored glove beside a link combines
  instead. The plan is silent; no real export is known to do it.
- open (found at 4.18's review) — hand auto-split: a face model whose hand-weighted
  vertices sit in no face gives `model_hand_split` an empty `gloves` value and an empty
  gloves package. No real file is known to reach it.

- open (found at 4.26c's review) — when an all-or-none install's undo cannot move an old
  CPK back from `.cpk.old` (`deploy.rs` `undo`), the failure is only logged: `download/`
  then lacks a CPK its list names, so the game loads none of the folder, and the member
  sees only the `old_cpk_locked` or `deploy_target_unwritable` that started the undo. The
  move back renames a file the run itself just moved, so nothing real is known to fail it;
  the plan names no finding for it. Revisit if a member meets it.

- open (found at 4.26b's review) — a multi-CPK run promotes its CPKs to `output/` one
  rename at a time, so a rename failing part-way leaves `output/` with some new CPKs and
  some of the previous run's. The plan settles all-or-none for the installation only
  (`spec/team_compiler/pipeline.md` "Deploy CPKs"); the output folder is the maintainers' build folder, and the
  next run rewrites every CPK. Revisit if a maintainer meets it.

- open — the `livecpk/` replacement is not atomic (4.23a): the previous tree is removed before
  the staged one is renamed in, so a copy that fails partway (the output folder on another
  drive) or a removal that fails partway (a file the runtime holds open, unverified for FoxDen)
  leaves `livecpk/` partial or empty, reported as `output_commit_failed`. A swap through a
  sibling folder would narrow it; decide with the first in-game sideload test.
- open — the compiler reports only two of `kit_config::validate`'s findings (the two a
  version's encoding clamps, as `kit_config_version_clamped`, step 4.9a). The maintainer's
  rulings (2026-10-10): the three "requires model 144 or 160" warnings are reported as one
  Warning, `kit_config_option_ignored` (step 4.y-kitwarn); the two Infos
  (`kit_shirt_model_unknown`, `kit_unknown_sleeve_value`) stay unreported (step 4.y-ingame2
  (e): the game draws both values, no crash; DECISIONS 2026-10-10);
  `kit_collar_zero` (the lib's one Error; `messages.md` says `kit_config_invalid` covers a
  config that "fails to validate", which the deep pass does not do) waits for a collar
  census: before a kit is dropped for a zero collar, count how many real configs carry one
  (the AET exports' `Kit Configs/` and the installs' CPKs), eventually.

- open (4.y-fix11's review, 2026-10-10) — read in the Fox-to-pre-Fox direction,
  `KIT_LAYOUT_REMAP` puts two right-sock stripes (pre-Fox 2008-2040 and 1656-1688)
  exactly 6.00 units off their range's centre, TC-KIT-19's tolerance: the test passes
  at its edge, and one texel more would fail it. Nothing in game shows it (the
  maintainer's PES 21 view of design A sits as drawn, 4.y-ingame2). Check the right
  sock's column of the table against the models at the design-health pass before
  widening the tolerance.

- open — the headless egui test harness (a frame with AccessKit on, a node by label, a click as
  press and release frames) exists twice, in `studio_core/src/shell/mod.rs` and
  `team_compiler/src/view/mod.rs` (3.y design sweep). The next tool view in Phase 8 would be
  the third copy: extract it then into `studio_core` behind a test-support feature, rather
  than a third copy.
- open — for Phase 8's settings menu, the first code that saves `settings.toml`: the `[common]`
  table is read into `CommonSettings`, so a key it does not know (a newer version's) is lost on
  save, while a tool's table keeps unknown keys (`core/gui.md`). Settle whether `[common]` keeps
  them too before that save path lands (review S2.1).
- open, waits on FoxDen — Fox per-kit models: the maintainer answered (`DECISIONS.md` "on Fox
  each kit number gets a face package of its own, once FoxDen loads one"); the package layout is
  decided with FoxDen when it gains the support, and `kit_variant_model_fox` and
  `spec/model_format.md` "Kit-dependent assets" change then.
- open — an unwritable `livecpk\`: sideload mode writes it "with the same access path as
  deployment to `download/`, elevation included" (`spec/team_compiler/pipeline.md` "Output modes"), but no
  plan says what a sideload run does when it cannot (a tree has no `output/` to degrade
  to). Settle with the sideload step (review S6.1b item 21).
- open — the deep pass reads everything `compile` reads again (4.7): each checked file is
  read once by the pass and once by its task, and a solid `.7z` is decompressed twice (its
  check, then its tasks; three times before 4.7f for an export holding a metadata file).
  For a folder the second read comes from the system's file cache; for a `.7z` it is a whole
  extra decompression per export, about 2 s for a 0.6 to 1 GB export (4.7f's timing): about
  2 s of a 5.2 to 6.8 s compile. The buffer is not kept from the check to the tasks because
  planning needs every export validated first, and holding every `.7z` until then is the
  residency the budget forbids (`spec/libs/pipeline.md` "What a solid `.7z` is charged"). The
  maintainer's answer (2026-10-10): a small archive is kept, step 4.y-7z, done the same
  day: the `.7z` half of this issue is closed; the folder half (the file cache's second
  read) stands.
- open — a face diff is engine-specific (maintainer, 2026-10-03): a diff authored for one
  engine misplaces the face on the other; the compiler passes a diff through for whatever
  target it compiles, so a diff is the author's responsibility, as with Red. Investigated
  offline 2026-10-15 (`.tmp/topics/face_diff/findings.md`): one format on both engines, Konami's
  diffs carried unchanged from PES 17 to 18; the real split is version 4 (32 entries) against
  5 (33), with a mechanical conversion. The maintainer's example (2026-10-17, a member's
  anime face, `.tmp/topics/face_diff/lapwing/findings.md`): the Fox fix is two eye floats (head 0x2C
  and 0x30, 35 to 45), which PES 21's Edit mode does not show even at 0 and 90. Settled
  2026-10-19 by a second example and its author's explanation (`Sopdog`, the engines animate
  eyes differently, so model and diff are made per engine and do not convert). Answered
  2026-10-20 (DECISIONS "a face made for the other engine is transformed"): a `fox`/`pre-fox`
  face marker and the Sopdog pair's transformation applied when the engine changes, attempted
  after the DBC editors (plan `model_conversion/conversion.md` "A face made for the other
  engine"): derive the rule from the pair, check it in game on both engines, then build it.
- open — u16 face indices cap a reassembled split mesh (found at 2.20f review): `fmdl::Mesh`
  and the IR (`spec/model_conversion/ir.md` "IR struct") store faces as `[u16; 3]`, so `fmdl::ops::split::decode`
  refuses (loud `VertexMismatch`) an add-on file whose components together reference more
  than 65536 vertices, and vertex-limit splitting of an IR mesh can only move loose vertices.
  The 2.20h conversion census found 114 real files over it. Since 2.20h such a group stays
  split rather than erroring (`conversion.md` split rules), so nothing refuses them now. The
  face type is still Phase 7's decision (glTF brings u32 indices).
- open — texture names and case on Fox (found at 4.4b2): stems now compare case-folded when a
  player's sources share one, but `processing/model.rs` still points a model's texture path at the
  player's folder only when the referenced name matches a stem exactly (since 4.3), so a model
  naming `Shirt.dds` beside `shirt.dds` keeps its original directory; the written `.ftex` keeps
  one source's spelling while each model keeps its own. Whether the game's CPK lookup is
  case-insensitive is unmeasured; measure it (or fold both sides) before 4.20 opens every export.
  Also: per-stem conflict decisions are independent, so with two conflicts in one folder a
  package can be dropped over a stem whose winner is a package already dropped for another stem
  (rare; needs two differing stems across three packages).
- open — `model_convert` converge questions (2.20): (1) Fox decal shaders (`translucent`,
  `3ddc`, `eyeocclusion`) infer `Shaded` *exact* through the `3ddf` rule, so the highneck
  fixture converts to an opaque `Basic_C`, where the 19to16 converter wrote `Overlay` with alpha
  blending; the format plan deliberately gives decals no family — decide whether the rule should
  at least mark them approximate. (2) Hand split: a face inside both hands' selections goes to
  both gloves; unused materials/textures are not pruned from the split parts. (3) `retarget`
  returns `ConvertError::Validation` after mutating the IR (no rollback). (4) `.model` tangent `w`
  is 1.0 on import (plan bullet); the bitangent-derived handedness is untested in game.
  (5) A `Metal` material without an `Environment` texture exports `Basic_CNSR` with no
  `EnvironmentMap` sampler; the format plan assigns the template-cubemap fallback to the
  compiler's texture step (Phase 4), which must add the sampler, not just the file. (6) Konami
  FMDLs carry a 64-byte-per-bone `bone_matrices` block the IR does not; the add-on writes it
  empty and its files work in game, so the export drops it with a finding; what the block holds
  and whether it is derivable from `Bone.matrix` is unmeasured.
- open — `pes_savefile` the kit-slot rows: the ten-slot kit block is measured on 18 (+0x1C),
  19 (+0x20) and 21 (+0x30), with what Strip > Add writes (`spec/pes_savefile/model.md` "Kit
  slots and the strip table"); the tables carry `KitSlot*` rows for PES 17 alone. Add the rows
  for the other versions when the Save editor needs kit bindings. (The PES 21 team colours,
  this issue's other half, were measured in Edit mode on 2026-10-13 at the tables' bits.)
- open — FoxDen's PES 2021 gloves trampoline discards `GloveList.bin` (step 4.0, runs 4-7):
  `build_botch_21` in `Tools_4cc/FoxDen/scripts/01_patches.lua` (and the patched exe it
  reproduces) loads the gloves id from `[rsi+0x2C]` for outfield players and goalkeepers,
  overwriting the value the original code chose in `[rsp+0x70]`, which for a player whose save
  appearance id is -1 is his `GloveList.bin` row (the stock exe showed it; the patched exe showed
  bare hands). Request for FoxDen's maintainer: keep the original value when the player has no
  save appearance record, for outfield players too (p1 routes them through the goalkeeper path),
  then an in-game retest with a `GloveList.bin` row for an outfield and a goalkeeper player.
  The Compiler's gloves output on Fox depends on it. Probably moot since 8.36: no
  `GloveList.bin` row is written, and the patch writes the record's gloves id, which
  `[rsi+0x2C]` probably holds (not checked); 8.36d's in-game check settles it.
- open — `pes_savefile::ops::fpc::is_fpc_player` finds no FPC player in a real cup save (step
  4.0 survey of the 2023 Winter Cup save): FPC teams there give each player a blank boots and
  gloves id of his own (`/a/`: 126/126 ... 148/148) rather than 55/11, which its doc comment
  already calls indistinguishable. The Save editor cannot use it to tell FPC teams apart; a hide
  strip (long sleeves, tucked, short socks) plus boots no other player wears marked them in the
  survey (`.tmp/phase_04/4_0/apptest`, `fpc` subcommand). Settle with the FPC toggle, built with the
  Save editor's view in Phase 8 (DECISIONS 2026-10-15 "the session wiring only the view
  drives").
- **Texport write is unverified in-game** (2.17h): `Texport::new` synthesizes 18-21 files from
  measured templates and `to_bytes` rewrites read files; both round-trip byte-identical, but no
  generated file has been imported by the game yet (`plans/pes_savefile/verification.md` "Texport write": manual,
  per version; a `new` file and an edited round-tripped one). PES 15/16/20 texports have no
  fixture at all (offsets/key index are the reference's; PES 20's size is derived).
- open — FoxDen's LiveCPK gaps that sideload mode depends on (forwarded by the maintainer to
  FoxDen's maintainer, 2026-10-03; source `Tools_4cc/FoxDen/src/livecpk.c`, `patterns.c`):
  (1) the call sites are found on PES 2018 only, 2019 and 2021 still to locate; (2) PES 2020 is
  not recognised (proxy only); (3) a served path is limited to about 99 characters (redirect plus
  name in the read request's 0x80-byte field; a face texture's folder alone is 59), longer names
  silently fall back to the CPK; (4) only boot-time shader loads are verified in game, model,
  texture and kit-bin loads under `livecpk\` are not (Sider's buffered-copy and CPK-size hooks
  were not ported). Phase 4's sideload acceptance waits on them; recheck FoxDen when itemizing.
  FoxDen session 3 (2026-10-14, PES 2019 on the stock 1.6.0.0 exe, scratch
  `.tmp/release_0_0_2/foxden/plan.md`): (5) FoxDen's own build runs proxy-only on every PES 2019 exe here:
  `host_is_game()` wants a static `dinput8.dll` import, which neither exe has, on disk or in
  the running image (the game loads DirectInput at run time); (6) with that check relaxed
  (test copy), the game dies right after the livecpk Lua state is created, on the stock exe as
  on the patched one: `lua_tolstring` resolves to a misaligned address (RVA 0x4bbba5f, the
  pattern's second pass), most likely mid-function. FoxDen session 4 (2026-10-20, its new
  version on the same stock exe, scratch `.tmp/release_0_0_2/foxden/plan.md`): (5) and (6) are fixed (the
  host check reads the version resource; `lua_tolstring` resolves aligned), the game runs
  with every script loaded, and the kit-swapper script works on a Studio CPK: kit 1 in Edit
  mode and kit 2 in a match each load the per-kit face package (`<id>p<K>`) or the `kitK`
  texture, falling back to `kitN` when neither exists. Not yet checked: the team as away,
  boots and gloves packages, `dummy_kit`, a `kitN/` folder set.
- open — a run that does not end cleanly leaves its staging (3.z): closing the window while
  the GUI's compile runs ends the process mid-run, and the coordinator's `Cancelled` path does
  not discard the staging either, so `output/.staging/<run>/` can be left beside the previous
  CPK (which stays intact). Fixed with cancellation (`spec/core/gui.md` "Cancellation", Phase 8): the
  window stops the run and the cancelled path calls `deploy::discard`. The same step adds the
  check between a texture task's textures (4.6's slice (c)); a test cancels a real run partway.
- open — a texture link beside a combined shared folder's texture of the same stem (4.11d):
  a player holding `hair.dds.common` and a link to a shared face whose folder holds
  `hair.dds` compiles with no finding; the shared folder's copy wins, the link is ignored,
  and `hair` is packed twice (the player's home and the team's Common output).
  `texture_stem_conflict` covers the player folder's own files alone. The plan is silent;
  the likely rule is a conflict finding, since collisions are rejected and never silently
  resolved, and it belongs with the stem namespace in `aesthetics_export` or with the
  textures task's conflicts. Rare: it needs a shared folder combined into a player who
  also links one of its stems to `Common/`.
- open (2026-10-09, 4.y-fix4) — the deep pass answers "does this link feed the player's own
  package" with an empty hand-weighted set (`deep/mod.rs` `FaceUse::of_player`), because it
  is the pass that finds the hand-weighted models, while planning and validation answer
  with the real set. For a Fox player whose gloves link combines only through his split
  hands, the linked folder's textures do not count for his `.mtl` paths there, so a
  `mtl_texture_not_found` Warning may be spurious (read, not run); nothing is dropped.
  `FaceUse::combined`'s doc comment ("Empty for a team player ...") was already untrue on
  Fox. Fix: find the hand-weighted models before the face use is built, or build the face
  use after them.
- open (2026-10-11, 5.0e review S3) — a disallowed file a lenient run keeps in a `Common/`
  subfolder no link reaches (`Common/sub/x.doc`) is reported twice on Fox:
  `common_file_disallowed` and `file_not_used` (`validation.rs`, `read_by_no_task`'s
  `!is_read_common_file` arm), where `messages.md` `file_not_used` says a file the
  structure pass already named is not reported (read, not run). Fix: skip a file the
  structure pass reported.
- open (2026-10-15, step 20.0c) — a `stat_bonus` on a stat a tier already targets at 99 gives
  an exact target no player can reach (99 is the game's top), and nothing says so: the first
  Clouds template's "Medals at giga: +2 to every stat" made every gold 101, so `check-aatf`
  placed no player in gold (a TierCount violation on every VGL26 team) instead of naming the
  rule. `aatf_rules.md` is silent. Settle with the next interpreter step: clamp a raised target
  at 99 (the bonus then means nothing for that tier), or refuse it on load where a tier's
  value plus a special's bonus passes 99 (a special's condition may never meet that tier, so
  a load rule could refuse a playable file). The same logic version change fixes the Phase 5
  converge audit's finding (`.tmp/phase_05/5_6/audit_shapes.md` row 1.7): an explicit `()` on an
  optional `Condition` or `Effects` key (`captain: ()`, `fielded_only_at: ()`, `forbid: ()`)
  reads as `None` in Rust but as present in the logic, which tests the key's presence; the
  logic's `num`/`list`/`table`/`flag` helpers already treat `()` as absent. Only a hand-written
  file reaches it.

- open 2026-10-29 — the 120 s hang guards in the Save editor's tests (`frames_until`,
  `portraits`' `served`) cost a mutation run up to 120 s per mutant that silences a worker
  (they were 2 s and 30 s before, both failing correct code under load). At the next
  `save_editor` mutation run, read those mutants' durations from its outcomes; if they add
  notable time, lower the bound to the longest a loaded machine was measured to need
- open 2026-11-02 (f1g9a) — `studio_core/src/shell/mod.rs` reached 1313 lines with the update
  check's poll and its three tests (1089 before; past the thousand already). After 0.0.1's
  cut, move the update poll and its tests beside `updater.rs`, and the shell's tests into a
  `tests.rs` of their own, as `settings_menu` has

---

## Log

The dated log is `docs/LOG.md`.
- 2026-11-05 — 0.0.2-poc1/poc2 (the maintainer's answers): a custom boots or gloves ID no folder holds
  becomes the hide ID with a Warning (`custom_model_hidden`), and Fox's missing texture is a Warning keeping
  the package. All 53 VGL26 exports run on copies first: 31 upgrades refused, 279 players. SWE-2 sidekick.
