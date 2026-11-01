# Unlimited Tool Works — Team compiler plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Crate layout

What exists of this is in `spec/team_compiler/README.md` "Crate layout".

These entries extend the spec's tree of `crates/tools/team_compiler/src/`; `check/` takes the place
of the spec's `check.rs`:

```
├── check/            # the `check` command, and the live grid's checks (the lib validates;
│   ├── mod.rs        #   this schedules): which export to check when, shallow or deep
│   └── live.rs       #   the watcher's and the cache's events mapped to rows and revisions
└── view/
    ├── grid.rs       #   team grid on studio_core's grid widget; ID cell editing
    └── toolbar.rs    #   split Compile button, writability badge, run strip
```

Placement rules:

- `check/` only schedules the export validations of `libs/aesthetics_export` for the live grid.
  The folder watcher (`notify` + debouncing) and the check cache (the manifest fingerprint and
  what it keys) are `libs/pipeline`'s (`spec/libs/pipeline.md`), not this crate's: the Balls
  compiler's list view watches its folder the same way (`plans/balls_compiler.md` "GUI"), and a
  watcher in this crate would be copied there, tool crates never depending on tool crates. The
  design of both is `gui.md` "Live validation", since the grid is what they exist for.
- The view is drafted first (`core/gui.md` "Prototypes before implementation"):
  `docs/prototypes/tools/team_compiler.js` shows the grid with sample rows in every cell state, the
  toolbar with its split button, badge and run strip, and the log panel, and is refined with the
  maintainer before `view/grid.rs` is written.

---

## First-release GPU BC7

Desktop GPU BC7 for PES 19–21 uses `block_compression`'s library backend in the first release.
Auto is the default, with CPU fallback and an explicit CPU mode for reproducible builds. The
[library crates plan](../libs/README.md) owns backend selection, startup, batching, memory, and fallback requirements; the
the reproducibility paragraph in `spec/team_compiler/testing.md` "Testing: parity against Red (adapted from Blue)" records the accepted texture-byte exception. Phase 4
builds the texture step CPU-only and Release 0.1.0 ships that way; the GPU backend is integrated
into it in Phase 16, rather than waiting for 3D preview.

## Future Features

Custom GPU kernels and browser GPU acceleration remain deferred until justified by measurements;
the shared library backend does not have to be replaced merely because it shipped first.

### 3D model preview viewport

A future enhancement to the GUI: when the user clicks a player on the progress grid, a 3D viewport
shows a preview of the model.

Scope note: this stays a quick *glance* ("is this the right model") — for actually inspecting a
player folder there is the [Player aesthetics editor](../player_aesthetics_editor.md), which launches
Blender instead of rendering in-app. When this feature is built, the viewport widget goes into a
shared **`libs/model_viewport`** crate (created then, not before — until this feature lands, no
viewport code exists anywhere): the Player aesthetics editor embeds the same widget as a quick
approximate preview beside its launcher, labeled as a preview with Blender as the accurate view,
which gives the crate its second consumer per the core plan's workspace guardrails.

**Feasibility:** High. egui renders via `wgpu`, which supports custom rendering alongside the
immediate-mode UI. The selected folder's structure descriptor can lazily load and parse its model
under the normal memory budget; the resulting vertex/face data is uploaded as GPU buffers and drawn
in the egui painter's render pass.

**Architecture:**

```
User clicks a player on the grid
        │
        ▼
GUI: submit the folder descriptor + source provider to a cancellable background load
  ─ Acquire a memory permit and materialize the selected FmdlFile/ModelFile off the UI thread
  ─ Read vertices (positions, normals, UVs) from the materialized model
  ─ Read faces (index buffer)
  ─ Read bone data (for optional skeleton overlay)
  ─ Convert texture (DDS/FTEX → raw RGBA via dds_convert)
        │
        ▼
GUI: render with wgpu alongside egui
  ─ Upload vertex/index buffers to GPU
  ─ Create texture from raw RGBA
  ─ Draw mesh in an egui custom painter callback
  ─ Camera: orbit/zoom/pan via egui input
  ─ Optional: skeleton wireframe overlay
```

**Implementation:**

```rust
pub struct ModelPreview {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub bones: Vec<BonePreview>,
    pub texture: Option<TexturePreview>,  // raw RGBA
}
// Loaded on a cancellable background worker under a normal memory permit,
// never on the UI thread; the result is tagged with the export revision so
// a stale preview is discarded.

// Conceptual egui_wgpu pseudocode: the exact constructor/API shape depends on
// the egui version pinned when this future feature is implemented.
let (rect, response) = ui.allocate_exact_size(
    egui::vec2(400.0, 300.0),
    egui::Sense::drag(),
);
let callback_state: Arc<dyn egui_wgpu::CallbackTrait> = Arc::new(ModelPreviewCallback {
    gpu_resources,
    camera,
});
// Wrap callback_state in the pinned version's PaintCallback helper and add it
// to the painter; CallbackTrait owns prepare/finish/render-pass integration.
ui.painter().add(make_paint_callback(rect, callback_state));
```

**Performance:** A 20K-vertex model is ~640KB of vertex data — uploaded to GPU once, rendered at
negligible cost. `wgpu` handles millions of vertices trivially. Texture conversion (one 2K DDS → raw
RGBA via `dds_convert`) takes ~10ms. Only one model is previewed at a time, so memory is not a
concern.

**Limitations:**
- **Bind pose only:** The model is shown in its T-pose. PES applies bone transforms at runtime via
  the skeleton and `face_diff.bin`. Showing a posed model would require implementing the PES
  skinning pipeline. Bind pose is sufficient for "is this the right model, does it look right."
- **Not a perfect PES render:** `wgpu` rendering uses standard shading. PES uses custom shaders
  (fox3ddf_blin, fox3dfw_constant, etc.) with specific alpha/blending behavior. The preview shows
  geometry and base texture accurately, but advanced material effects (anti-blur, UV scrolling,
  transparency) won't match the in-game appearance exactly.
- **One texture at a time:** A face model may have multiple materials (face, hair, oral). The
  preview shows the base texture of the selected material, or cycles through them.
- **WASM compatibility:** `wgpu` works in browsers via WebGPU. The 3D preview is available in both
  the desktop and web builds.

**When to implement:** After the GUI is functional (post-Phase 8). The core architecture doesn't
need to change; it's a `wgpu` rendering pass alongside egui. The object model already has all the
data needed. The first step is a short spike (one model, the orbit camera, one texture) that
shows whether a maintained Rust crate keeps the renderer small, before `libs/model_viewport` is
planned in detail: three.js would have given the loader and camera for free, so the renderer
is the part to keep small.

---

## Acceptance

What exists of this is in `spec/team_compiler/README.md` "Acceptance", whose rules (format, the
default export, the default version) these scenarios follow; IDs continue the spec's numbering.

**0.0.1 scope.** The preview's discovery skip (`pipeline.md` "1. Reader"):

```
TC-SRC-11  GIVEN an exports root holding "co Full Spring.upgrading", a folder an interrupted
                 upgrade left, beside a finished export of another team; and the same folder
                 spelled "co Full Spring.UPGRADING"
           WHEN the root is compiled
           THEN export_upgrade_unfinished (Warning) names the folder, nothing else is reported
                about it, and only the finished export is compiled; GIVEN --export naming such
                a folder THEN it is compiled as any export
```

**Phase 8 scope.** Phase 8 delivers the view: the live-validating grid (`gui.md`), the toolbar
with its split Compile button, the run strip, the log panel's cross-links, cancellation
(`spec/core/gui.md` "Cancellation") and the tool's settings section. What a scenario observes
is what a frame draws, through the headless harness (the frame's AccessKit nodes and the
painted cells' colors, read back from the tool's row state the view paints from; a click as
press and release frames), the files written, and the events the run emits; `manual` marks
only what needs a display or a running game. "The exports folder" is one under the test's
own folder, named by the test's settings; the shell's own behavior (sidebar, settings menu,
status bar, help) is `ST`'s (`plans/core/gui.md` "Acceptance").

```
TC-GUI-04  GIVEN an exports folder holding a plain-folder export with players 01, 03 and kits p1,
                 g1, and one archived (.zip) export
           WHEN the view opens
           THEN each export is a row with its team label and ID cell, the plain-folder row's
                cells at the fixed positions 01 and 03 (02 empty, no cell) and GK and K1, each
                cell checking then full-ok, warning or error as its scope's deep check ends;
                the archive's cells end partial-ok (blue) or partial-error with nothing
                decompressed beyond the root metadata a shallow check reads (players.txt, a kit's
                config.toml; a solid .7z, its entry table alone)
TC-GUI-05  GIVEN the view open on an exports folder
           WHEN an export folder is copied in and has been quiet for the debounce period
           THEN a row for it appears and is checked; WHEN an export is deleted or renamed THEN
                its row goes, or carries the new name, on the next frame; WHEN a file inside a
                checked export is edited (its players.txt given an unparseable line) and has
                been quiet for the debounce period THEN its row is checked again and shows the
                new finding, and a result of the check of its previous revision arriving after
                that leaves the row's new statuses as they are
TC-GUI-06  GIVEN an export checked at the last launch and unchanged since
           WHEN the view opens again
           THEN its row shows its last statuses without a check running; GIVEN one of its files
                edited since, or the teams list changed, or a validation-affecting setting
                changed, THEN it is checked again
TC-GUI-07  GIVEN an export whose team is not on the teams list
           WHEN the view draws its row
           THEN the ID cell is red and editable; typing an ID in 701-920 and Enter writes it to
                the data directory's teams_list.txt and every row revalidates; an entry out of
                range keeps the cell red with the reason in its tooltip; an ID another team
                holds asks "in use by {team}; reassign?" and writes only on yes; GIVEN the data
                directory not writable THEN the cell reverts, red, teams_list_read_only in its
                tooltip
TC-GUI-08  GIVEN a team export whose name carries neither Full nor Midcup
           WHEN the view draws its row
           THEN the row holds a Full and a Midcup button where its cells would be; clicking one
                renames the folder or archive with the tag after the team name and the row
                revalidates under the new name
TC-GUI-09  GIVEN a cell with findings
           WHEN the pointer hovers it
           THEN the tooltip shows the folder name and the findings' text, worst severity first;
                WHEN it is clicked THEN the detail panel opens on that scope
TC-GUI-10  GIVEN a kit folder whose colors.txt parses and whose icon_<N> marker names pattern N
           WHEN its cell is drawn
           THEN the cell shows pattern N's two-color swatch in the kit's two menu colors under
                its label; GIVEN the kit's check failed THEN the swatch shows the base
                UniColor.bin colors instead
TC-GUI-11  GIVEN the toolbar at rest
           WHEN the Compile button's arrow is clicked
           THEN the list offers Compile, Compile for sideloading (disabled on PES 15 and 16),
                Test output and Open output folder; picking a mode relabels the main part and
                the next main-part click runs that mode; the mode is normal again at the next
                launch
TC-GUI-12  GIVEN a run in flight
           WHEN frames are drawn
           THEN each cell being processed keeps its check color with a bold label, then turns
                done, done-with-warning or done-with-errors as its task ends; the run strip
                shows the stage, the elapsed time, the files written and the memory in use;
                the Compile button reads Cancel
TC-GUI-13  GIVEN a run in flight over two exports
           WHEN Cancel is clicked
           THEN the run ends after the task or texture in progress, no CPK is written or
                replaced and the previous outputs are as they were, the cells not yet processed
                return to their check colors, and the log's last line says the run was cancelled
TC-GUI-14  GIVEN an export holding a finding whose effective disposition is DropExport
           WHEN Compile is pressed
           THEN every player and kit cell of its row shows the blocked state, each tooltip
                naming the export-level cause, and no task of that export runs; GIVEN a folder
                with a DropFile finding THEN its cell ends done-with-errors (orange-red)
TC-GUI-15  GIVEN a warning line in the log panel
           WHEN it is clicked
           THEN the cell of its scope is highlighted in the grid; a line whose scope is the
                export highlights the row label
TC-GUI-16  GIVEN the destination not writable (the game running, or elevation needed)
           WHEN the toolbar is drawn
           THEN the Compile button carries the writability badge, its tooltip naming the reason
                and the remedy; the button stays enabled, the run degrading as TC-DEP's
                scenarios say
TC-GUI-17  manual  GIVEN the sideload mode selected and PES 21 installed in pes_folder_path
                   WHEN Launch PES is clicked
                   THEN PES2021.exe starts from that folder, not through Steam, and the button
                        is disabled while it runs
TC-GUI-18  GIVEN `UTW --gui team-compiler compile` with an export in the exports folder
           WHEN the initial check completes
           THEN the compile starts by itself, its grid and log indistinguishable from a click's;
                GIVEN quick_compile_close_on_success set THEN the window closes when the run
                ends with no Error-level finding and nothing skipped for errors, as any close
                does (a tool holding unsaved work is asked first, `core/gui.md` "Closing the
                window") (`settings.md`:
                warnings alone still close it), and stays open otherwise; a manual Compile click
                with the setting on never closes it
TC-GUI-19  GIVEN the settings menu open on the Team compiler's section
           WHEN cpk_name is edited
           THEN the settings file's [team-compiler] table holds the new value and the next
                compile writes a CPK of that name
TC-GUI-20  GIVEN a referee export whose players.txt lists slots 1, 2 and 35, slots 1 and 35
                 backed by the same model folder
           WHEN the view draws its row
           THEN the row's cells sit at the fixed positions 01 to 35 (only 01, 02 and 35 drawn),
                cells 01 and 35 share one status and their tooltips name the same folder, and a
                finding of that folder colors both
TC-GUI-21  GIVEN an export holding a shared model folder no player links (shared_folder_orphaned,
                 W) and a root logo_small.png with no main logo (logo_small_without_main, E)
           WHEN the view draws its row
           THEN the row label is red, the worse of the two findings' colors, its tooltip lists
                both with their scopes, and no player or kit cell changes for them
TC-GUI-22  GIVEN exports_folder_path an absolute path to a folder that does not exist (the
                preflight refuses before any event), and no input after the click
           WHEN Compile is clicked
           THEN a repaint is requested after the refusal is available, and the next frame's
                log ends with the refusal and its exit-code line; GIVEN a run over one export
                instead THEN the frame woken after its thread ends shows the exit-code line
TC-GUI-23  GIVEN a run in flight over two exports, another tool's view active
           WHEN the window's close is requested
           THEN the Team compiler's view is active with its prompt; WHEN it keeps the window
                THEN the run ends as it would have; GIVEN close chosen instead THEN the run is
                cancelled as by TC-GUI-13 (no CPK written or replaced, the previous outputs as
                they were) and the window closes after its last line, not before
TC-BIN-43  GIVEN fpc_hide_boots_id = 60 and fpc_hide_gloves_id = 12 in the common settings, and a
                /co/ export whose slot 05 carries fpc_on and no boots or gloves
           WHEN it is compiled for PES 21
           THEN the aesthetics patch gives player 71405 boots_id 60 and gloves_id 12; GIVEN
                fpc_hide_boots_id = 627 instead THEN the run is refused before any output
                (exit code 2), the message naming the setting and its range
```

**8.36c scope.** The full-look aesthetics patch (`plans/pes_savefile/operations.md`
"Aesthetics patch (new)"): every compiled player's look goes into the patch on every engine,
and the CPK carries no player table. The spec's scenarios that observe `PlayerAppearance.bin`,
`BootsList.bin` or `GloveList.bin` are restated below on the patch, each restatement naming
the scenarios it restates, which are withdrawn when this lands; TC-BIN-37 (the bins CPK keeping
installed rows in multi-CPK mode) and TC-SAV-07 (`savefile_player_not_stripped`) are
withdrawn with nothing to restate, the tables and the stripped route being gone. A pair `(P, N)`
of a restated THEN reads as the patch's `boots_id` (or `gloves_id`) for player P being N, and
"keeps the installed pair" as the entry holding no such key, the save keeping its ID; "the
patch" is
`<output_folder_path>/aesthetics_patch.toml`. Five hold unchanged, their tests no longer
installing the tables: TC-BIN-04, TC-MOD-31, TC-MOD-83, TC-BIN-39 and TC-BIN-40.

```
TC-SAV-18  GIVEN a /co/ export with no write_names marker whose slot 05 holds a face model and
                 a settings.toml setting skin_color = 4, one motion key and a name the save's
                 71405 does not carry, whose slot 06 holds an ingame_face.toml
                 setting iris_color = 7 and [hair] length = 5, and whose players.txt maps a
                 folder holding a face model and no settings file to 07 and 08; and a
                 savefile whose team 714 holds 71405 to 71408, 71405 in roster slot 06 and
                 71406 in slot 05, 71405's record holding appearance id -1 and base copy 7511
           WHEN it is compiled for PES 21
           THEN the CPK holds no PlayerAppearance.bin, BootsList.bin or GloveList.bin and no
                player_table_missing is reported; the patch names the CPK and patch_written
                names the patch; the patch's entries 05, 06, 07 and 08 each
                hold an [appearance] with every key of PES 21's PlayerSettings schema, 05's skin
                colour 4 and the motion key's value, 06's iris 7 and hair length 5, every other
                key the template's default, and boots_id 0 and gloves_id 0, and 05's holds no
                name; in the savefile each of the four records' appearance block and motions
                are his entry's, his appearance id and base-copy id his own id, 71405's name
                as it was, every other byte of the payload
                unchanged, and the previous file is the .bak (restates TC-SAV-01, TC-BIN-25,
                TC-BIN-28)
TC-SAV-19  GIVEN a /co/ export whose slot 05 holds a settings.toml setting skin_color = 4, a
                 motion key and boots_id = 12, and links Boots/Crocs/ (boots.model and
                 boots.mtl), and savefile_path naming a PES 17 fixture save holding team 714
           WHEN it is compiled for PES 17
           THEN settings_model_id_conflict names slot 05's boots; the patch is written with
                pes_version 17, its 71405 entry holding an [appearance] with every key of PES
                17's PlayerSettings schema (skin colour 4, the motion key's value) and boots_id
                625; in the savefile 71405's appearance block and motions are the entry's, his
                boots id 625, his appearance id and base-copy id his own id, and the base-copy
                flag clear; GIVEN the export carrying the write_names marker and slot 05's
                settings.toml also setting name = "Snuffy" THEN the entry and the savefile's
                71405 carry the name Snuffy (restates TC-SAV-12)
TC-SAV-20  GIVEN a /co/ export whose slot 05 holds 101334.copy and a settings.toml, and no
                 face, and a savefile whose 71405 record holds base copy 7511 with the
                 base-copy flag set (PES 18) and as it is (PES 21)
           WHEN it is compiled for PES 18, and for PES 21
           THEN each patch's 71405 entry holds base_copy_id = 101334 beside its
                [appearance], and in each savefile his base-copy id is 101334, PES 18's flag
                still set
TC-SAV-21  GIVEN a savefile whose 71405 to 71408 records each hold an appearance block no
                 template default matches, and boots 7; and a Full /co/ export whose slot 05
                 holds boots.fmdl and a settings.toml with a key of the wrong type, slot 06 an
                 ingame_face.toml that does not parse and a settings.toml setting
                 skin_color = 4, slot 07 a settings.toml with [appearance.hair] and an
                 ingame_face.toml setting iris_color = 7, and slot 08 an ingame_face.toml
                 holding an unknown key; none with a marker
           WHEN it is checked, then compiled for PES 21
           THEN both runs report settings_toml_invalid for 05 and 07 and
                ingame_face_toml_invalid for 06 and 08, each naming the file and the error
                (07's naming appearance.hair as an unknown key); the CPK holds no
                face/real/71406/ or 71408/ path; the patch's entries for 05 to 08 hold
                no [appearance], 05's holds boots_id 625 alone and the others no key; in the
                savefile the four records' looks and motions are as they were but for 71405's
                boots, now 625, the others' boots still 7 (restates TC-BIN-33, TC-STR-18,
                TC-STR-19)
TC-STR-21  GIVEN /co/ player folders holding, in turn: 101334.copy beside face_high.fmdl,
                 beside Round.face (Faces/Round/ holding a face model), and beside an
                 ingame_face.toml; 101334.copy and 7511.copy; 0.copy; 71405.copy in slot 05;
                 Bob.copy; 101334.copy.txt alone, in slot 09; and 71402.copy (a teammate)
                 alone, in slot 06
           WHEN the export is checked
           THEN the first three report copy_link_with_face, the fourth copy_link_duplicate,
                the fifth, sixth and seventh copy_link_id_invalid, each dropping its folder;
                slot 09's and slot 06's links resolve with no finding; GIVEN players.txt
                mapping slot 09's folder to 09 and 10 and its link renamed 71410.copy THEN
                copy_link_id_invalid drops it
TC-BIN-50  GIVEN a savefile whose 71405 record holds boots 7, and a /co/ export whose slot 05
                 holds a boots.fmdl whose task fails and a settings.toml setting an appearance
                 key and a motion key, and whose slot 06 holds a face model that compiles
           WHEN it is compiled for PES 21
           THEN the patch's 71405 entry holds no boots_id and an [appearance] carrying the two
                keys, and the savefile's 71405 keeps boots 7 (restates TC-BIN-11, TC-BIN-32)
TC-BIN-51  GIVEN slot 05 holding glove_l.fmdl and glove_r.fmdl, slots 07 and 08 linking
                Gloves/Keeper/, and slot 09 alone linking Gloves/Grip/
           WHEN the export is compiled for PES 21
           THEN the patch's gloves_id is 625 for 71405, 644 for 71407 and 71408, and 629 for
                71409 (restates TC-BIN-12)
TC-BIN-52  GIVEN a savefile whose 71405, 71406 and 71407 records hold boots 7, 9 and 55, and a
                /co/ export whose slot 05 holds boots.fmdl, slot 06 no boots, and slot 07 no
                boots and the fpc_off marker
           WHEN it is compiled for PES 21 as a Full export, then as a Midcup export
           THEN the Full run's patch gives boots_id 625, 0 and 0, and the Midcup run's 625,
                no boots_id for 71406 (the savefile keeping his 9) and 0 (restates TC-BIN-26)
TC-BIN-53  GIVEN no PES install configured, and a /co/ export whose slot 05 holds boots.fmdl
           WHEN it is compiled for PES 21
           THEN the CPK holds k0625's boots and no player table, the patch gives 71405
                boots_id 625, and nothing names a player table (restates TC-BIN-27)
TC-BIN-54  GIVEN a /co/ export whose slot 05 carries fpc_on and its settings.toml sets
                boots_id = 12, slot 06 carries fpc_on and no boots or gloves, and slot 07 holds
                boots.fmdl and sets boots_id = 12
           WHEN it is compiled for PES 21
           THEN the patch gives boots_id 12, 55 and 627 and gloves_id 11, 11 and 0 to 71405,
                71406 and 71407, and settings_model_id_conflict names slot 07's boots
                (restates TC-BIN-29)
TC-BIN-55  GIVEN a Full /co/ export whose slot 05 carries fpc_on, and whose settings.toml sets a
                 key the hide preset sets, to another value, the inners on, undershorts and
                 taping; and whose slot 06, with no marker, sets the gloves checkbox on and no
                 gloves; and two more /co/ exports whose slot 05, with no marker, sets the
                 gloves checkbox on and holds no gloves: a Full one linking Round.face, the
                 export's Faces/Round/ holding a face model, and a Midcup one holding a face
                 model
           WHEN each is checked and compiled for PES 21
           THEN both runs of the first report fpc_strip_conflict, fpc_inners_break_hiding,
                fpc_undershorts_break_hiding and fpc_taping_shows_pieces for slot 05 and
                fpc_gloves_checkbox_winter_gloves for slot 06; 71405's [appearance] in its
                patch carries the preset's value for the key and the inners, undershorts and
                taping as authored; the Full one reports fpc_gloves_checkbox_winter_gloves for
                slot 05 (a face link is no gloves ID), and the Midcup one reports no gloves
                finding (restates TC-BIN-30)
TC-BIN-56  GIVEN a Full /co/ export holding Boots/Crocs, Boots/adidas and Boots/Birk, slots 05
                 and 07 linking Crocs, slots 06 and 08 adidas, and slot 09 alone Birk (a shared
                 folder no player links takes no ID)
           WHEN it is compiled for PES 21
           THEN the CPK holds unlimited-tool-works/shared_ids/714.toml whose [boots] names 644
                adidas and 645 Crocs and not Birk, and the patch gives boots_id 645 to 71405 and
                71407, 644 to 71406 and 71408, and 629 to 71409, Birk under slot 09's own ID
                (restates TC-BIN-34)
TC-BIN-57  GIVEN an installed CPK below the output holding TC-BIN-56's record, and a Midcup
                 /co/ export holding Boots/Birk and Boots/crocs, slots 05 and 07 linking Birk
                 and slot 06 alone linking crocs
           WHEN it is compiled for PES 21
           THEN Birk takes 646 and crocs keeps 645, the record naming it though one player
                wears it: the patch gives boots_id 646 to 71405 and 71407 and 645 to 71406, the
                CPK's record names 644 adidas, 645 crocs and 646 Birk, and no
                shared_ids_unrecorded is reported (restates TC-BIN-35)
TC-BIN-58  GIVEN no installed CPK holding a /co/ record, and a Midcup /co/ export holding
                 Boots/Birk, slots 05 and 06 linking it; and, apart, the same Midcup with slot
                 05 alone linking Birk
           WHEN each is compiled for PES 21
           THEN the first's Birk takes 644, the patch giving boots_id 644 to 71405 and 71406,
                shared_ids_unrecorded names team 714, and the CPK's record names 644 Birk; the
                second's Birk takes 625, slot 05's own ID, the patch giving it to 71405, and
                the CPK holds no record and reports no shared_ids_unrecorded (restates
                TC-BIN-36)
TC-BIN-59  GIVEN four /co/ exports whose slot 05 settings.toml sets the stock ID 3 (gloves_id
                 in the first two, boots_id in the others): the first's slot 05 holding
                 Crocs.gloves, the export's Gloves/Crocs/ holding glove_l.fmdl; the second's a
                 body.fmdl whose vertices carry both hands' weights; the third's Round.face,
                 the export's Faces/Round/ holding face_high.fmdl and boots.fmdl; the fourth's
                 Crocs.boots, the export's Boots/Crocs/ holding a texture and no model
           WHEN each is checked and compiled for PES 21
           THEN the patch's 71405 entry holds gloves_id 625, his exclusive ID, in the first
                (Crocs, which he alone wears) and in the second (the hands split off his body),
                and boots_id 625 in the third (Round's boots.fmdl a part of his boots), each run
                reporting settings_model_id_conflict for the authored key; the fourth's holds
                the authored boots_id 3, shared_folder_no_model names Boots/Crocs, and no
                settings_model_id_conflict is reported (restates TC-BIN-41)
TC-BIN-60  GIVEN /egg/ slot 05 holding face_high.fmdl and a settings.toml, and slot 06
                 holding an ingame_face.toml setting iris_color = 7 and [hair] length = 5
           WHEN the export is compiled for PES 21
           THEN 79205's [appearance] in the patch holds the template's defaults for every face,
                hair and iris key, and 79206's holds iris 7 and hair length 5, its other face
                and hair keys the defaults (restates TC-BIN-47)
TC-BIN-61  GIVEN a Midcup /co/ export whose only player folder, slot 05, holds an
                 ingame_face.toml setting a facial hair type and a settings.toml setting
                 boots_id = 12, and no model
           WHEN it is compiled for PES 21
           THEN the CPK is written with no player table, and the patch's 71405 entry holds
                boots_id 12 and an [appearance] from his two files, the facial hair type the
                ingame_face.toml's (restates TC-BIN-48)
TC-BIN-62  GIVEN a /co/ export whose slot 05 ingame_face.toml holds facial_hair_type = 20
                 (PES 18 unmeasured: the field's 0 to 31; PES 21: 0 to 19) and in-game hair
                 (style = 4, color_red = 40), and whose settings.toml holds
                 hunching_dribbling = 5 (PES 18: 1 to 3) and dribbling = 2 (PES 20 and 21
                 only)
           WHEN it is compiled for PES 18, and for PES 21
           THEN the PES 18 run reports settings_value_out_of_range for hunching_dribbling
                alone, and its patch's 71405 [appearance] holds facial hair type 20,
                hunching_dribbling 1 and no dribbling key, no finding naming dribbling; the
                PES 21 run reports it for facial_hair_type alone and its [appearance] holds
                facial hair type 0; both carry that hair; both exit 0 (restates TC-BIN-49)
TC-MOD-95  GIVEN Faces/Round holding fcl_hair.fmdl and boots.fmdl, slot 05 linking it as his face
           WHEN the export is compiled for PES 21
           THEN the boots folder of slot 05's exclusive id holds boots.fmdl and the patch gives
                71405 that id, 625, as boots_id (restates TC-MOD-55)
TC-MOD-96  GIVEN Faces/Round holding fcl_hair.fmdl, boots.model and boots.mtl, slot 05 linking it
           as his face
           WHEN the export is compiled for PES 21
           THEN link_combined is reported, the boots folder of slot 05's exclusive id holds the
                boots converted with Round's .mtl, the patch gives 71405 boots_id 625, and the
                exit code is 0 (restates TC-MOD-58)
TC-MOD-97  GIVEN Faces/Round holding fcl_hair.fmdl and boots.fmdl, Boots/Crocs holding boots.fmdl,
           slot 05 holding no model and linking both (Round.face, Crocs.boots)
           WHEN the export is compiled for PES 21
           THEN link_combined is reported for both links, the boots folder of slot 05's exclusive
                id holds Round's and Crocs's boots merged, the patch gives 71405 boots_id 625,
                and no shared boots folder is written for Crocs (restates TC-MOD-59)
TC-MOD-98  GIVEN slot 05 holding a hand-weighted face model and Crocs.gloves, Gloves/Crocs holding
           glove_l.fmdl and glove_r.fmdl
           WHEN the export is compiled for PES 21
           THEN link_combined is reported, the gloves folder of slot 05's exclusive id holds the
                split hands and Crocs's gloves, the patch gives 71405 gloves_id 625, and no
                shared gloves folder is written for Crocs (restates TC-MOD-60)
TC-MOD-99  GIVEN Common/boots.fmdl, Boots/Crocs holding boots.fmdl.common alone, and slot 05
           holding face_high.fmdl and linking Crocs.boots
           WHEN the export is compiled for PES 21
           THEN Crocs takes slot 05's own boots ID, 625 (he is its one wearer), whose boots
                package holds Common's boots.fmdl's meshes, the patch gives 71405 boots_id 625,
                and neither shared_folder_no_model nor file_not_used is reported (restates
                TC-MOD-79)
TC-REF-20  GIVEN a refs export whose Ref A holds a face model and a settings.toml setting the
                 gloves checkbox on and no gloves; and a refs export whose Ref A, with the same
                 settings.toml, links Grip.gloves, the export's Gloves/Grip/ holding
                 glove_l.fmdl
           WHEN each is checked, and compiled for PES 21
           THEN both runs of the first report fpc_gloves_checkbox_winter_gloves for Ref A, and
                its compile writes no player table and no aesthetics patch; the second reports
                no gloves finding (the linked gloves are his own package) (restates TC-REF-19)
```
