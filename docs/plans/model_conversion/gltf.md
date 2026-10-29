# Unlimited Tool Works — Model conversion plan: glTF source

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

Part of the [Model conversion plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## glTF as the universal model source

The suite's authoring format is **glTF with PES extensions plus `materials.toml`** — the [Unified
model format plan](../model_format.md) specifies it (rationale, folder layout, `PES_bone`/`PES_mesh`
extension schema, native-skin skeleton, material files and their `.common` links, the complete
material schema). This plan covers what `model_convert` does with it.

Unlike FMDL and .model, glTF gets **no workspace format crate of its own**: parsing is the external
`gltf` crate's job (a mature codec — there is no bespoke binary format to own), and everything the
suite adds on top — reading/writing the `PES_bone`/`PES_mesh` extensions and the `materials.toml`
files, `gltf_to_ir`, and `ir_to_gltf` — is
IR-coupled, so it lives inside `model_convert`. A separate wrapper crate would have `model_convert`
as its single permanent consumer, which the core plan's workspace guardrails flag as over-splitting;
there are also no format-native mesh algorithms or outside consumers to justify one (Blender-side
glTF handling is Blender's own pipeline plus the `pes-models` extension's PES glTF codec, no Rust
involved).

### glTF import (`gltf_to_ir`)

`gltf_to_ir` takes the glTF document plus the folder's **resolved material layers** for that glTF
(the caller — the Team compiler's folder task — performs name matching, `.common` link resolution,
and the least→most-specific ordering per the format plan; `model_convert` performs the field-level
deep merge and schema validation, so the merge rule has one implementation shared by every
consumer). Each layer is tagged with its origin folder (player folder or Common), because a texture
stem resolves in the folder of the material file that set it. It fills the IR's `Material` structs
from the merged entries, resolving canonical texture roles to `Texture` entries through the
caller-supplied asset resolver (see "glTF dependency and image contract"). Schema violations surface as typed errors the caller maps to the
`material_*` message catalog. The glTF files themselves are never rewritten (exports stay
read-only).

### glTF export (`ir_to_gltf`) and dual-set superset merge

What exists of this is in `spec/model_conversion/gltf.md` "glTF export (`ir_to_gltf`) and dual-set superset merge".

glTF is also an IR **target**, for the [Player aesthetics editor](../player_aesthetics_editor.md)'s
convert-to-glTF migration and the [Export upgrader](../export_upgrader.md)'s optional glTF conversion
pass (the Team compiler never emits glTF — its targets are the game formats). `ir_to_gltf` emits
a single `.glb` per model carrying the `PES_bone`/`PES_mesh` extension schema that `gltf_to_ir` reads — the
exporter and importer share the schema tables, so a Studio-emitted file is by construction
importable, and `gltf_to_ir(ir_to_gltf(x))` round-trips are part of the test suite. Existing
textures in the source folder are referenced externally from the material tomls by stem, never
re-encoded or embedded. Materials are emitted as a single catch-all `materials.toml` per folder with
app-injected comments, each entry carrying the inferred `family` plus the source engine's native
table verbatim (see "Emission" in the format plan).

The IR `Bone.matrix` carries the full PES bind transform. This is not a direct byte-layout copy
into glTF: the sample SKL positions are model-space, whereas glTF nodes use parent-local transforms
and serialize matrices column-major. `ir_to_gltf` converts coordinate conventions and hierarchy,
constructs matching `skin.joints` and `inverseBindMatrices`, and preserves PES-only metadata such as
`globalPosition` and `sklParent` in `PES_bone`. The reverse path reconstructs the PES transforms.
Verify a multi-level, rotated skeleton and skinned mesh in both directions; merely expanding 3×4
matrices to 4×4 would apply parent transforms twice.

For folders holding **both native sets**, `model_convert` provides the superset
merge the editor's conversion uses:

```rust
/// Fill `preferred`'s format-exclusive gaps (None fields) from `other`,
/// matching materials by name and meshes by part identity. Geometry, bones,
/// and all populated fields of `preferred` win; nothing is averaged.
pub fn merge_ir_superset(preferred: CanonicalModel, other: &CanonicalModel) -> CanonicalModel;
```

The result is one glTF per part holding the preferred geometry plus compatible format-exclusive
metadata from both sources. If the two native sets have different geometry, it cannot reproduce
both originals: choosing the preferred set is an intentional geometry replacement. Report
unmatched or incompatible metadata rather than claiming an unconditional lossless superset.
Which set supplies the geometry is the caller's choice, surfaced at conversion time.

### IR part merge (`merge_ir_parts`)

What exists of this is in `spec/model_conversion/gltf.md` "IR part merge (`merge_ir_parts`)".

`merge_ir_parts` stays specified here as the IR-level equivalent, **deferred**:
nothing in the plan calls it, and it is implemented only if a caller appears (a merge of parts
that must stay in the IR, inside the Player aesthetics editor for instance). The rules below are
the contract all three share.

```rust
/// Assemble several parts into one model. `parts` is in canonical (alphabetical
/// source-name) order and that order is preserved in the output.
pub fn merge_ir_parts(parts: Vec<CanonicalModel>) -> Result<CanonicalModel, MergeError>;

pub enum MergeError {
    MaterialConflict { name: String },          // → team-compiler `merge_material_conflict`
    SkeletonConflict,                           // → team-compiler `skl_merge_conflict`
    SourceFormatMismatch,                       // parts must share `source_format`
}
```

Rules (identical to the `fmdl` merge unless stated):

- **Bones** are unioned by name; a bone present in several parts must have the same `matrix`
  (and, when populated, the same positions) or the merge fails with `SkeletonConflict`. Each mesh's
  `bone_group` and every vertex `bone_mapping` are remapped to the unioned indices. In practice
  pre-Fox parts share the standard body skeleton, so the union is the identity; the rule exists so a
  custom-skeleton part cannot be silently merged with a default-skeleton one.
- **Meshes and mesh groups** are concatenated in part order; `split_group_id` and anti-blur
  metadata stay per mesh (they are re-encoded on export as usual).
- **Materials** are merged by name. Two definitions with the same name must be equal in every
  populated field — for `.model` sources that includes `state_settings`, i.e. the `.mtl` content —
  otherwise `MaterialConflict`. Mesh `material` indices are remapped. Because `model_to_ir` consumes
  the `.model` + `.mtl` bundle, merging materials *is* the `.mtl` merge: `ir_to_model` emits one
  `.mtl` for the merged model, so the Team compiler never has to splice MTL XML itself.
- **Textures** are unioned by path (same path → one entry, indices remapped).
- **`extension_headers`** are unioned per key with values deduplicated, in canonical order.
- **`source_format`** must be the same for all parts (the caller converts foreign-format parts
  first, as the Team compiler already does before Fox merging).

Deterministic by construction: the caller supplies canonical order and every union uses ordered
collections, satisfying the byte-reproducible output contract.

### glTF dependency and image contract

What exists of this is in `spec/model_conversion/gltf.md` "glTF dependency and image contract".

- **URI safety and locality:** `model_convert` accepts dependencies through a
  caller-supplied asset resolver rather than opening paths itself. The caller
  percent-decodes and canonicalizes external buffer/image URIs into the shared
  `vtree::ScopePath` type and requires them to remain
  inside the **source model folder**, not merely somewhere beneath the export root.
  Absolute paths, parent traversal, cross-folder references, network URLs, and
  non-`data:` URI schemes are rejected before materialization; this keeps folder
  tasks independently ownable and keeps `model_convert` reusable by the Balls
  compiler without depending on the aesthetics export format.
- **Dependency ownership:** `.gltf` external `.bin` entries are classified as
  `GltfBuffer`, while `.glb` buffers remain embedded. Callers classify candidate
  external image files as `PotentialGltfImage` from structure alone; deep URI
  resolution promotes referenced candidates to `GltfImage` and returns
  unreferenced candidates to the caller's ordinary allowlist policy. A
  materialized source folder owns one permit-charged `Arc<[u8]>` for each unique
  resolved external buffer/image; multiple `GltfModel`/`ModelPart` consumers clone
  that Arc without a second charge. Embedded GLB/data-URI bytes are owned once by
  their `GltfModel`. Dependency files are not also emitted as ancillary/pass-through
  game files: a source image cannot have both dependency and pass-through roles
  simultaneously.
- **Embedded data:** standard GLB buffer views and base64 `data:` URIs are accepted
  and charged to the same folder memory permit. Malformed or unsupported MIME types
  are validation errors; remote fetching is never performed.
- **Accepted image formats and stem-based resolution:** the accepted image formats (all
  interchangeable as sources for any model format), the stem→file resolution, and the
  `texture_stem_conflict` rule are specified in the format plan ("Dependencies and images"); the
  [library crates plan](../libs/README.md) has the full decoder matrix.
- **PNG-texture trajectory:** Blender cannot write DDS natively, and the ported
  addons convert DDS→PNG on import, so glTF-authored models (the plan's end-state
  authoring path) naturally carry PNG textures. As the community migrates from
  direct FMDL/.model editing to glTF authoring, PNG sources become the norm rather
  than the corner case. Measure decode, mip generation, and the selected CPU encoder on real
  textures; earlier hypothetical timings are not benchmarks of this implementation. The
  [in-memory conversion cache](../libs/README.md) reuses retained results on the edit→compile→test loop
  (engaging only for ≤2-team compiles); a fresh full-cup compile still pays for conversion.
  See the [library crates plan](../libs/README.md) for cache design and engagement rules.

### Blender integration

What exists of this is in `spec/model_conversion/gltf.md` "Blender integration".

The community works with PES models in Blender through two complementary paths. Both are packaged as
codec modules of **one Blender 5.2 extension** — the all-in-one `pes-models` plugin, which also
hosts the [Player aesthetics editor](../player_aesthetics_editor.md)'s loader module and **replaces the
standalone ported `pes-fmdl`/`pes-model` addons at its release** (plan:
`blender_project/pes_models_plan.md`, kept with the Blender project outside this repo):

**Path A — legacy FMDL/.model import-export (lossless, direct).** The existing `pes-fmdl` and
`pes-model` addons have working Blender 5.2 ports; `pes-models` vendors them as its `fmdl` and
`pes_model` codec modules, minimally modified (packaging consolidation, not a rewrite — the ported
code is the battle-tested asset). As a successor step, the hot paths — `parseVertices`,
`parseFaces`, `encodeVertices`, mesh splitting, split vertex encoding, anti-blur — are replaced by
calls into `fmdl` and `pes_model` via the `python_bindings` PyO3 wheel. The codecs retain a
pure-Python fallback so they work even if the wheel is not installed or has rotted against a newer
Blender Python. This path serves members editing existing `.fmdl`/`.model` files directly.

**SKL on FMDL import.** The FMDL bone data carries only local and global positions (Vector4), not
the 3×3 rotation matrices — those live in the companion `.skl`. The current `pes-fmdl` addon builds
the armature from `PesSkeletonData.py`'s hardcoded reference skeleton, ignoring any companion SKL.
This loses custom skeleton data when the user later exports to glTF via Path B (the glTF would
carry the reference skeleton, not the custom one). The `pes-models` FMDL codec fixes this: on
import, it reads the companion `.skl` (same-basename pairing, per the Aesthetics export plan's SKL pairing
rule) and builds the armature from the SKL's full 3×4 transforms. When no companion SKL exists, it
falls back to the current `PesSkeletonData` behavior (the reference skeleton's positions, which
were extracted from `body.skl` once). This ensures a Blender→glTF round-trip preserves the skeleton
regardless of source format. The SKL format is simple enough (3-u32 header, 56-byte fixed records,
null-terminated name table — ~150 lines) that a pure-Python reader ships with the initial
`pes-models` release, no wheel needed; when the PyO3 wheel arrives, the Python reader is replaced
by the `fmdl` crate's SKL codec alongside the other hot paths.

**Path B — unified glTF authoring format (new, IR-routed).** The extension's PES glTF codec wraps
Blender's native glTF exporter and emits glTF with name-only material objects **plus the folder's
`materials.toml`** — it does not inject PES material data into the glTF JSON. On import, it reads
the tomls from the model's folder and maps each material's PES properties to Blender custom
properties, so glTF-authored models round-trip through Blender without losing their PES data
(essential once glTF folders are the norm). This replaces the ~260KB of Python across the legacy
addons for *new* authoring. The tomls hold the superset of both engines' settings (common,
FMDL-exclusive, and .model-exclusive), so a model authored once in glTF can be compiled to either
PES engine with the IR handling the format-specific losses explicitly. It reads the custom
properties that the legacy addons already set (`fmdl_material_shader`, `fmdl_texture_role`,
`fmdl_alpha_flags`, etc.) and writes them to the tomls. This path needs no Rust inside Blender —
conversion happens at compile time in `model_convert`.

**Material editing panel.** All PES material settings — both those that map to Blender material
properties (for viewport preview) and those that have no Blender equivalent (pure PES fields) —
are shown in a single panel, so the user edits everything in one place without jumping between
Blender's native material tabs and a separate PES extension panel.

**What users call it.** The community names model files by engine, not by format: `.fmdl` is a
"fox model file", `.model` a "pre-fox model file". The authoring format joins that family as the
**"global model file"**, extension `.glb` — one file that compiles for both engines, opens in
Blender and any glTF viewer, and echoes its letters the way `fmdl` echoes "fox model". This is a
nickname, never an expansion: user-facing text (GUI labels, messages, the user docs, the
extension's export menu item) says "global model file" or ".glb" and **never writes "glb stands
for"**; the one bridge to the real name sits beside the Blender export settings — *the format is
glTF 2.0, which is what Blender's own exporter is called; choose `.glb`* — so a member who meets
"glTF" in a menu is not contradicted. `.gltf` + `.bin` stays accepted and is mentioned once, in
the format plan's file rules, never in messages (a message names the file, not the format).
Plans, code, identifiers and the `PES_*` extension names keep **glTF**, the correct technical term
and the one the `gltf` crate, the spec and Blender's Python API use.

The two paths coexist as modules: Path A for legacy compatibility and direct editing, Path B for new
authoring targeting both engines. When the community fully migrates to glTF authoring, the
extension's `fmdl`/`pes_model` codec modules and the `python_bindings` crate are retired together.

**One schema, one canonical converter.** The extension's codecs write the format plan's schemas,
`gltf_to_ir` validates them, and any glTF + tomls the extension exports must import cleanly through
`gltf_to_ir`. Since the extension can technically cross formats itself (import `.model`, export
`.fmdl`), the documented rule is: Blender round-trips are for *authoring*; migrating a player folder
between formats is the Studio's IR conversion (see "One schema, one canonical converter" in the
format plan).

### Pipeline integration

What exists of this is in `spec/model_conversion/gltf.md` "Pipeline integration".

```
Path A (legacy):  Blender ──pes-models fmdl/pes_model codecs──→ .fmdl / .model
                         (PyO3 wheel from python_bindings crate)

Path B (new):     Blender ──pes-models glTF codec (native export + materials.toml)──→ .gltf + .bin + textures + materials.toml
                                                            │
                                                    ┌───────┴───────┐
                                                    │  model_       │
                                                    │  convert      │
                                                    │  glTF→IR      │
                                                    │  →FMDL or     │
                                                    │   .model      │
                                                    └───────────────┘
```

The compiler also detects `.gltf`/`.glb` by file extension; glTF sources always go through the IR.
