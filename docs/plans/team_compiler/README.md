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
data needed.

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
           THEN BootsList.bin holds (71405, 60) and GloveList.bin (71405, 12); GIVEN
                fpc_hide_boots_id = 627 instead THEN the run is refused before any output
                (exit code 2), the message naming the setting and its range
```
