# Unlimited Tool Works — Player aesthetics editor plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

Covers `tools/player_aesthetics_editor` — a convenience tool for viewing and
tweaking a single player folder: browse the available exports' player folders,
open one **in Blender** with everything pre-loaded, edit its `settings.toml`,
and convert its models to glTF. It also specifies the **launch manifest**, the
contract between the Studio tool and the Blender-side loader. The loader is a
module of the **all-in-one PES models extension** (working name `pes-models`) —
one Blender 5.2 extension owning import *and* export of `.fmdl`, `.model`, and
PES glTF, which replaces the standalone ported `pes-fmdl`/`pes-model` addons at
its release. The extension is planned with the Blender project, outside this
repo (`blender_project/pes_models_plan.md`); this plan owns only the manifest
contract and the Studio side.

The export format and `settings.toml` schema are owned by the
[Aesthetics export plan](aesthetics_export/README.md); the IR/glTF machinery by the
[Model conversion plan](model_conversion/README.md); FPC by the
[Save editor plan](save_editor.md); platform context in the [core plan](core/README.md).

The tool is **not essential** — nothing else depends on it — but it is planned
now because it settles library seams early: `model_convert` gains its only glTF
*exporter* (`ir_to_gltf`), and the decision to view models in Blender rather
than in an in-app viewport removes a whole prospective lib crate (see "Why
Blender, not a custom viewport").

---

## Purpose

Today, checking "what does this player actually look like with these settings"
means compiling and booting PES, and editing a player's savefile settings means
a trip through the save editor. This tool shortens both loops for the common
case of poking at one player:

1. **Browse** every player folder across the exports folder, pick one.
2. **View** its models in Blender — pre-assembled, categorized, with the
   generic PES body/kit shown or hidden according to the folder's FPC and strip
   settings, and per-mesh show/hide via Blender's native Outliner.
3. **Edit** the folder's `settings.toml` (and `fpc_on`/`fpc_off` markers) — quick edits in
   the Studio panel, or in a Blender sidebar panel with the FPC/strip changes
   reflected immediately in the viewport.
4. **Convert** the folder's models to glTF, in place — the migration path
   toward the unified authoring format.

## Why Blender, not a custom viewport

The tool was first conceived with an in-app wgpu viewport plus an egui outliner
panel. Launching Blender instead was chosen because it deletes the two largest
work items while giving strictly better results:

- **No viewport lib to build.** An in-app viewer needs a `model_viewport` crate: wgpu
  render callback, camera controls, mesh upload, DDS→GPU textures, bind-pose
  skeleton placement — all to reach a preview that is bind-pose-only with
  approximate shading. Blender's viewport, with the material setup the ported
  PES import codecs already produce, is simply better.
- **No outliner panel.** Blender's Outliner does per-mesh and per-collection
  show/hide natively; recreating it in egui would be a worse copy.
- **No new dependency in practice.** Model makers already work in Blender, the
  Blender 5.2 migration is already underway (see the Blender 5.2 plan kept with
  the Blender project), and the `pes-fmdl`/`pes-model` ports to 5.2 exist and
  work. The loader builds on their import codecs inside the all-in-one
  extension.

Costs accepted: the tool's viewing half is desktop-only (no WASM launch path),
and Blender plus the ported addons must be installed. Blender stays the
**accurate, primary** viewing path permanently; an in-app viewport re-enters
only as a byproduct: if/when the [Team compiler plan](team_compiler/README.md)'s
future 3D-preview feature is built, its widget lands in a shared
**`libs/model_viewport`** crate and this tool embeds it too — a quick
approximate preview beside the launcher, labeled "preview only — open in
Blender for the accurate version". Two consumers justify the crate then; until
that feature lands, no viewport code exists anywhere and nothing in this tool
waits for it.

**Division of knowledge:** all export-format logic stays in Rust. The Studio
tool resolves folders, links, categories, settings, and base models into a
**manifest**, and the Blender-side loader is deliberately dumb: it imports the files
the manifest lists, builds collections, and sets initial visibility. No
`aesthetics_export` logic is duplicated in Python.

---

## The Studio tool

One view, three areas. Initial launch, editing, and conversion support **plain-folder exports
only**. An archive source shows an extract-first notice; the user explicitly extracts it to a plain
folder before using these actions. The tool does not silently extract to a temporary editing copy
or write back into archives.

**Browser/launcher.** Lists the exports in the configured exports folder (via
`aesthetics_export` structure parsing — the same eager descriptor load the compiler
uses) and each export's player folders; referee exports appear like any other
(their folders are ordinary player folders). Shared `Faces/`/`Boots/`/`Gloves/`
folders are listed too and can be launched bare (models only — no settings, no
base-model context). Selecting a player folder shows its summary: models per
category and source format set, link targets and whether they resolve, portrait
presence, `fpc_on`/`fpc_off` marker, settings.toml presence. (Once `libs/model_viewport`
exists — created with the Team compiler's future 3D-preview feature, not for
this tool — the summary also gains the quick approximate preview described in
"Why Blender, not a custom viewport".) The **Launch** button writes
the manifest and starts Blender (path from the tool settings, auto-detected
when possible) with the loader module's bootstrap:
`blender --python <bootstrap> -- <manifest.json>` (exact invocation is an
implementation detail; the manifest is the contract).

**Folder tree.** The summary shows the selected folder's tree as the compiler reads it, so
the format's flexibility is seen rather than inferred: each subfolder, each `.common` link with
the `Common/` file it stands for (the file at the link's own path below `Common/`) and whether
it resolves, each shared link with its folder, kit-token folders (`kit1` to `kit9`) grouped as
one set with the kit numbers the export defines and those it lacks, and for each texture a
model names, where it resolves (beside the model, a parent folder, a subfolder path the model
names, a link into `Common/`, or nowhere). The resolution is `aesthetics_export`'s and the
compiler's planning, not a second implementation in the tool, so the view cannot disagree with
the compile. Rules: `spec/aesthetics_export/player_folders.md` "Subfolders", `model_format.md`
"Link files" and "Kit-dependent assets".

**Settings panel.** A form over the selected folder's `settings.toml` and `ingame_face.toml` —
the name rule (absent / `true` / explicit string) and **every `PlayerSettings` key**, grouped as
the schema groups them (appearance, physique, strip, motion, colours, ingame-face parameters),
each key written to the file that owns it (the face, hair and iris keys to `ingame_face.toml`,
"`ingame_face.toml`" in the [Player folders plan](aesthetics_export/player_folders.md)) — plus
the `fpc_on`/`fpc_off`/absent marker tri-state and an in-game face toggle, which creates an
empty `ingame_face.toml` or, once confirmed, removes it (its group shown only while the file
exists). The form is **generated from the `PlayerSettings` schema**, not hand-built per field:
the two files are the only route by which a team's aesthetics reach the game (the compiler
builds the Fox appearance tables and the pre-Fox aesthetics patch from them, and the save
editor's own fields are read-only — see "Player
settings in exports" in the [Aesthetics export plan](aesthetics_export/README.md)), so a field the
schema gains must appear here without anyone
remembering to add a widget. Each key renders from its schema type and range, with the same
app-injected comment as its tooltip. Writes preserve user comments and formatting (`toml_edit`);
validation is `aesthetics_export`'s, so the form can never write a file the compiler would reject.
Comments in auto-generated tomls are app-injected (predefined per-field documentation, never
user-authored from scratch); `toml_edit` preserves them and injects comments for newly added keys.
This is the quick-edit path that doesn't need Blender running — for example flipping `fpc_off` on
a player without launching anything.

The **ingame-face parameters** are the one group the form exposes but does not expect anyone to
type: the eleven decoded feature types, and from Phase 5 the version's undecoded bits, that
mean nothing without seeing the face. The expected
workflow for that group is the game's own face editor, then generating the folder's two files
from the save (the save editor's `settings.toml` generation and "Generate in-game face", or the
Export upgrader; both, since a skin or physique change made in the same visit is
`settings.toml`'s) and committing the
result. The form shows the numbers so a diff is readable and a
single value can be nudged. A face
editor with a live preview — a rendered head from the parameters, the game's editor outside the
game — would be a genuine tool of its own and is noted as a **future step**, not a requirement on
this panel: it needs the head-morph data the game applies those parameters to, which nothing in the
suite decodes yet.

**Unset values stay unset.** Missing FPC/strip settings are not inferred from a savefile and are
not filled in merely by opening the folder. The preview shows a key the file leaves out at its
default, which is what the compile writes (the template's value; with no FPC marker, the
base body is visible). Wait for explicit user edits: turning FPC on writes the canonical `fpc_on` marker, turning it off writes
`fpc_off`, and strip edits persist only the edited keys. Both panels follow this rule; there is no
second FPC boolean in `settings.toml`.

**Conversion.** The "Convert to glTF" action (see below).

Both this panel and the Blender panel edit the same file; last writer wins, and
the Studio form re-reads on focus (the suite's folder watcher makes this cheap).
That is acceptable for a single-user convenience tool; no locking is attempted.

---

## The launch manifest

A versioned JSON file (`serde_json` on the Rust side, `json` stdlib on the
Python side — no TOML parsing needed to *read* the contract), written to a temp
location per launch:

```json
{
  "manifest_version": 1,
  "player": { "folder": "C:/.../Players/15 - Snuffy", "name": "Snuffy",
              "export": "team_x", "export_path": "C:/.../exports/team_x" },
  "studio": { "exe": "C:/.../UTW.exe", "pes_version": 19 },
  "settings_toml": "C:/.../15 - Snuffy/settings.toml",
  "ingame_face_toml": "C:/.../15 - Snuffy/ingame_face.toml",
  "settings_schema": [
    { "group": "appearance", "file": "settings", "table": "appearance", "key": "skin_color",
      "type": "int", "min": 0, "max": 6,
      "doc": "Skin colour, 0 white to 6 black (the game's swatch order)" },
    { "group": "ingame-face", "file": "ingame_face", "table": "", "key": "iris_color",
      "type": "int", "min": 0, "max": 10, "doc": "Iris colour, 0 black to 10 violet" },
    "…one entry per PlayerSettings key of this PES version…"
  ],
  "sets": [
    { "id": "fox", "active": true,
      "parts": [
        { "file": ".../face_high.fmdl", "category": "face",  "origin": "local" },
        { "file": ".../Boots/Crocs/boots.fmdl", "category": "boots",
          "origin": "shared:Crocs" },
        { "file": ".../Common/torso.fmdl", "category": "face",
          "origin": "common" }
      ] },
    { "id": "prefox", "active": false, "parts": [ "…(.model + .mtl pairs)…" ] }
  ],
  "base_models": {
    "source": "extracted",
    "body": [ { "file": ".../cache/pes17/body.model",
                "variant": "sleeves_short" }, "…" ],
    "initial": { "visible": false, "sleeves": "short", "socks": "long",
                 "untucked": true }
  },
  "state": { "fpc": "on" }
}
```

- **Sets** mirror the folder's coexisting source representations (Fox `.fmdl`,
  pre-Fox `.model`+`.mtl`, glTF). The active one follows the compiler's
  selection rule for the current global PES version (target-native first, then
  glTF, then the opposite native format), so what opens visible is what would
  compile.
- **`settings_schema`** is the `PlayerSettings` field list for the manifest's PES version — group,
  the file that owns the key (`settings` or `ingame_face`, the manifest's `settings_toml` or
  `ingame_face_toml`) and its table there (empty for the file's top level), key, type, range,
  and the app-injected comment as `doc` — emitted by `pes_savefile` from the same
  tables that drive the Studio form. The Blender panel builds its widgets from it, so the two
  forms show the same fields by construction and the plugin carries no copy of the schema (the
  "no export-format reasoning on the Python side" rule, applied to settings).
- **Origins** record where each part came from (local file, resolved shared
  link, resolved `.common` link) for collection naming. Unresolvable links are
  listed in a `problems` array and shown in both UIs, not silently dropped.
- **Base models** point into the extraction cache (or the bundled stand-ins). Initial visibility
  follows explicit FPC/strip settings where present and the keys' defaults otherwise. Unset FPC
  is distinct from explicit Off in the file; the defaults shown are not written to it.
- `manifest_version` gates compatibility: the plugin refuses newer majors with
  a clear message instead of misloading.
- **`player.export_path` and `studio.exe`** exist for the plugin's **"Export model and compile for
  sideloading"** button: after saving the model into `player.folder`, the plugin runs
  `<UTW.exe> team-compiler compile --mode sideload --export <export_path>` and reports the exit
  code (see "CLI" in the Team compiler plan). The CLI is the whole integration — no IPC with the
  running Studio, no Python-side knowledge of the pipeline. `studio.pes_version` is informational,
  letting the plugin label the button with the target version — but **only in a Blender session
  that Studio launched**, where the manifest was written moments ago for this very launch. In a
  standalone Blender session there is no fresh manifest; the plugin must not reuse one remembered
  from an earlier launch, since the user may have changed the Studio's version selector since. There
  it falls back to its own preference for the `studio.exe` path (if set), labels the button without
  a version, and the compile simply uses whatever version the Studio's settings currently hold.

## The Blender side: the loader module of `pes-models`

The loader is one module of the all-in-one PES models extension, which lives
with the Blender 5.2 project (outside this repo) and consolidates the ported
`pes-fmdl`/`pes-model` codecs, a new PES glTF codec (Blender's native glTF I/O
plus `materials.toml` read/write and `PES_bone`/`PES_mesh` extension handling),
and this loader into a single extension. The
all-in-one shape exists partly *because of* the loader: a player folder's sets
may be `.fmdl`, `.model`+`.mtl`, or glTF, so the loader depends on all three
importers no matter how they're packaged — one extension makes that dependency
one install. Details, module layout, and the replacement of the standalone
addons are in the extension's own plan (`blender_project/pes_models_plan.md`).

The loader module does four things:

1. **Import** each set's parts into a collection tree:
   `Fox set / Face | Boots | Gloves`, with origin suffixes in object names
   (`boots [shared: Crocs]`). Inactive sets import hidden — switching sets is
   toggling two collections.
2. **Import the base models** into a `PES base body` collection, strip variants
   as sub-collections, with visibility from the manifest. `fpc_on` hides the base body;
   otherwise it is visible, using the manifest's strip variants. Missing settings remain
   visibly unset in the form, the preview showing their defaults, which the compile writes.
3. **Sidebar panel** (N-panel): the same form over `settings.toml` and `ingame_face.toml` as
   the Studio panel, each key to its own file — generated from the same schema, which the manifest carries so the two
   panels cannot disagree on the field set (`tomlkit` for comment-preserving
   writes, shipped with the addon) — plus the `fpc_on`/`fpc_off` marker tri-state. FPC and strip edits update the base-model
   collection visibility immediately — the "see the setting on the model" loop
   the Studio panel can't offer.
4. Everything else is stock Blender: per-mesh hide via the Outliner, and — as
   a bonus outside this tool's contract — the user can edit meshes and
   re-export through the extension's own codecs (any of the three formats).

The loader performs **no export-format reasoning**: it never scans folders,
resolves links, or assigns categories. If the manifest is wrong, the fix is in
`aesthetics_export`/the Studio tool, one place. (The extension's *codecs* obviously
reason about model file formats — the rule bans duplicating the **export
folder format** knowledge: rosters, links, categories, markers.)

## Base models (generic PES body/kit)

Showing a non-FPC player honestly requires the generic PES body wearing a
generic kit — and its absence is exactly what an FPC preview must show.

- **Primary source: the user's installed PES.** The generic body/kit models are
  extracted from the installed game's CPKs via the `cpk` lib into a per-PES-
  version cache in the app data dir (no game assets are ever bundled or
  redistributed). The exact per-version game paths of the generic models are an
  implementation detail recorded as constants when built, engine-appropriate to
  the active set where the version provides both. Extraction runs lazily at
  first launch per version, with a manual re-extract action in the settings.
- **Fallback: bundled stand-ins.** Home-made low-poly approximations of the
  body/kit with the sleeve/sock/untucked variants, authored once and shipped
  **as glTF** (Blender imports it natively; no PES formats needed for
  stand-ins). Used when no PES install is configured or extraction fails; the
  manifest's `source` field says which the user is looking at.
- Strip variants map to variant meshes/collections, not deformations.
- **Physique scaling is deliberately deferred**, but the seams expect it: the
  manifest's `base_models.initial` block is where a `physique` object would be
  added, and applying it would be plugin-side (Blender armature/lattice scaling)
  — no Studio-side redesign. Until then physique fields are edited blind, as
  they are today.

## glTF conversion

Converts the folder's models to the unified authoring format, **in place**:
after a successful conversion the folder contains the glTF set and the original
native files are gone — this is a migration, not an export, and leaving the
originals behind would change what the compiler selects (target-native beats
glTF in the selection rule, so a stale `.fmdl` would silently win).

- **Backup, outside the folder.** The removed originals are moved to the app
  data dir (`backups/`, keyed by folder path + timestamp) with a **Restore**
  button in the tool. An in-folder backup (`.bak` files or a backup subfolder)
  would trip the compiler's file-allowlist and folder-shape validation, so the
  export stays clean instead.
- **Transactional folder replacement.** Convert every part before changing the live folder,
  preserve a complete recoverable original, and report success only after publication finishes.
  Several writes/deletes or directory renames are not one atomic operation on Windows; publication
  failures must restore the original or leave a clearly recoverable backup, and compiler checks
  must not treat an intermediate folder state as a finished conversion.
- **Dual-set superset merge.** When both native sets exist, each part pairs its
  Fox and pre-Fox sources and produces **one** glTF holding the superset of
  both formats' data: geometry (and everything derivable from it) comes from
  the **preferred set** — asked at conversion time, defaulting to Fox/FMDL —
  while the other set contributes only its format-exclusive fields (e.g.
  `.model` state settings when FMDL geometry is preferred), merged by material
  name and part identity. Parts present in only one set convert plainly. The
  merge machinery is `model_convert`'s (see the [Model conversion
  plan](model_conversion/README.md)'s "glTF export and dual-set superset merge").
- **Textures stay put.** Existing accepted image files (including DDS and FTEX) are kept and referenced externally
  from the generated `materials.toml` (allowed by the glTF image contract);
  nothing is re-encoded. The conversion emits a single `materials.toml` per
  folder (the catch-all), with app-injected comments. Existing glTF/material files and
  same-name materials with different definitions must not be overwritten silently; an
  unresolved collision fails conversion with the originals retained.
- **Skeleton from SKL.** Fox FMDLs carry only bone positions, not rotation
  matrices; the full bind-pose transform comes from the companion `.skl` (or the
  embedded template skeleton when no custom SKL is present). See "Skeleton
  reconstruction from FMDL" in the [Model conversion plan](model_conversion/README.md).
- The CLI form takes the geometry preference as a flag instead of asking.

## Library impact

The point of planning this tool early — what moves where:

| Piece | Home | Status |
|-------|------|--------|
| `ir_to_gltf` exporter + dual-set superset merge | `libs/model_convert` | **New** — glTF was import-only until this tool; specced in the Model conversion plan |
| Export/folder/link/settings parsing + validation | `libs/aesthetics_export` | Existing; gains this tool as a consumer |
| Comment-preserving `settings.toml`/`materials.toml` writes | `toml_edit` dep, used from this tool | Existing workspace dependency (`pes_savefile`, `kit_config`), no new crate |
| Base-model extraction from installed CPKs | this tool crate | Single consumer, stays in-tool per the guardrails |
| In-app 3D viewport (`libs/model_viewport`) | future lib, created with the Team compiler's 3D-preview feature | **Not created by this tool**; when it exists, this tool embeds it as a labeled approximate preview — the second consumer that justifies the crate |
| Blender loader | module of the all-in-one `pes-models` extension, Blender 5.2 project (outside this repo) | New; consumes the manifest contract only; extension planned in `blender_project/pes_models_plan.md` |

## Settings (tool section)

| Setting | Default | Notes |
|---------|---------|-------|
| Blender path | auto-detected | required for launching |
| Base models | extract from installed PES | options: stand-ins only, off |
| Preferred geometry (conversion default) | Fox (FMDL) | the conversion dialog still asks; this preselects |

## CLI

```
UTW player-aesthetics-editor launch <player-folder>
UTW player-aesthetics-editor convert <player-folder> [--geometry fox|prefox]
```

## Effort valves

The tool is a convenience; each piece has a cheap fallback if it turns out
expensive, roughly in drop order:

| Feature | Fallback |
|---------|----------|
| Base-model extraction from CPKs | stand-ins only (manifest `source` field already covers it) |
| Blender sidebar settings panel | Studio-panel editing only; relaunch to see changes |
| Dual-set superset merge | convert the active set only |
| Strip-variant base meshes | single base body, FPC show/hide only |
| Studio settings panel | drop quick-edit; the save editor's TOML generation remains |

## Testing

- **Manifest**: golden-manifest test on a fixture export (links, `.common`,
  dual sets, FPC marker); version-gate behavior.
- **Superset merge**: fixture `.fmdl`+`.model` pair → glTF → `gltf_to_ir` →
  both native targets, asserting each field came from the correct source set;
  compared against direct single-source conversions.
- **Conversion publication**: a failing part leaves the folder untouched; backup + restore
  round-trip; interrupted publication and rollback failures retain recoverable originals,
  including Windows and backups on a different volume.
- **Settings writes**: comment/formatting preservation round-trips (`toml_edit`); marker tri-state
  operations; opening an unset folder writes nothing, while an explicit FPC toggle writes its
  canonical marker and strip edits persist only edited keys.
- **Source boundary**: archives require explicit extraction before launch/edit/conversion; no
  hidden temporary editing copy or archive write-back.
- **No-PES path**: manifest generation with stand-ins, no install configured.
- **Launch integration**: manual (requires Blender); the manifest contract
  keeps the automated surface on the Rust side.

## Key decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Viewing | Launch Blender 5.2 via a loader, not an in-app viewport | Deletes the viewport and outliner work; Blender's viewport/Outliner are better than any bespoke wgpu pass; the community already lives in Blender |
| Future embedded preview | When the Team compiler's 3D-preview feature creates `libs/model_viewport`, embed it beside the launcher, labeled "preview only — open in Blender for the accurate version" | Free at that point (two consumers justify the shared crate); Blender stays the accurate view, so the label keeps expectations straight |
| Blender-side packaging | Loader ships as a module of the all-in-one `pes-models` extension (fmdl + .model + PES glTF import/export) | The loader needs all three importers anyway; PES glTF round-trip (import with `materials.toml` + `PES_bone`/`PES_mesh` restored) exists nowhere else; one install, one schema, one retirement path — see `blender_project/pes_models_plan.md` |
| Studio↔plugin contract | Versioned JSON manifest, loader stays dumb | All export-folder knowledge stays in `aesthetics_export`/Rust; no logic duplicated in Python |
| Settings editing | Both a Studio panel and a Blender sidebar panel, both generated from the `PlayerSettings` schema (carried in the manifest as `settings_schema`) | Quick edits without Blender; in-Blender edits see FPC/strip reflected live — same file, last writer wins; generated forms cannot fall behind the schema, which is the only route to the save |
| Ingame-face parameters | Exposed as plain fields; the expected workflow is game face editor → generate `settings.toml` and `ingame_face.toml` from the save → commit → recompile (the recipe above). A face editor with a live preview is a future step, not a requirement here | Forty numbers mean nothing without a rendered head; the preview needs head-morph data nothing in the suite decodes yet |
| Base models | Extracted from installed PES per version, bundled glTF stand-ins as fallback | Honest FPC/strip preview without redistributing game assets; works without a PES install |
| glTF conversion | In place, originals removed, backup in app data with Restore | Leftover native files would out-rank the glTF in the compiler's selection rule; in-folder backups would trip export validation |
| Dual-set merge | Superset glTF; geometry from a preferred set chosen at conversion (default FMDL) | One authoritative file per part going forward; format-exclusive fields of both engines survive |
| Physique preview | Deferred, seams reserved (manifest `initial` block, plugin-side application) | Significant effort, approximation at best; blind editing is no worse than today |
