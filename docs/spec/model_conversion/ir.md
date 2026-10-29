# Unlimited Tool Works — Model conversion plan: Intermediate representation

Part of the [Model conversion plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Intermediate Representation (IR)

### Purpose

The IR is a canonical PES model representation that is a **superset** of both FMDL and .model
formats. It eliminates duplicated conversion logic by providing one decode path and one encode path
per format, instead of N×N direct converters.

### When the IR is used

- **Same format as target**: ordinary processing skips the IR; hand auto-split, pre-Fox
  `ingame_face` merging, and skeleton retargeting across PES versions (only when a native-format
  pre-check finds a used bone the target lacks or poses differently — see "Skeleton retargeting and
  bone conformance") are the explicit IR-operation exceptions described in `conversion.md`.
- **FMDL → .model**: FMDL → IR → .model
- **.model → FMDL**: .model → IR → FMDL

### IR struct

The IR uses `Option<T>` for format-specific fields. `Some` when imported from the format that has
the field, `None` when imported from a format that doesn't. Exporters fill in defaults for missing
fields.

```rust
pub struct CanonicalModel {
    pub bones: Vec<Bone>,
    pub meshes: Vec<Mesh>,
    pub mesh_groups: Vec<MeshGroup>,
    pub materials: Vec<Material>,
    pub textures: Vec<Texture>,
    pub extension_headers: BTreeSet<String>,   // model-level header lines neither format crate
                                               //   types (`Skeleton-Type: Simplified`, ...)
    pub source_format: SourceFormat,           // Fox | PreFox (Gltf joins in Phase 7)
}

pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,       // hierarchy: FMDL stores it; .model does not (resolved by
                                     //   name from the template skeleton's render parents)
    pub matrix: Affine,              // model-space 3×4 bind transform. Sources: FMDL's companion
                                     //   .skl (or the template); .model's inline per-bone inverse
                                     //   bind matrix, inverted; glTF skin bind data
    // FMDL-specific. In a self-consistent Konami FMDL + SKL pair (the audience fixture)
    // `global_position` is the SKL translation and `local_position` is `global` minus the
    // parent's `global` (roots: equal to `global`), so an exporter without them derives both
    // from `matrix` and `parent`; the player-part templates carry placeholder values instead.
    pub global_position: Option<[f32; 4]>,
    pub local_position: Option<[f32; 4]>,
    pub bounding_box: Option<BoundingBox>,
}

pub struct Mesh {
    pub vertices: Vertices,
    pub faces: Vec<[u16; 3]>,        // one winding for the whole IR (FMDL's); `.model` reverses
    pub bone_group: Vec<usize>,      // indices into `bones`; `vertices.bone_indices` index this
    pub material: usize,
    pub extension_headers: BTreeSet<String>,   // per-mesh header lines the format crate leaves raw
    pub custom_bounding_box: Option<BoundingBox>,  // FMDL `Custom-Bounding-Box-Meshes`
}

// Struct-of-arrays, like both format crates' `MeshVertices`: a conversion is a column copy,
// and no per-vertex allocation exists on a 100k-vertex model. Every `Vec` is `positions.len()`
// long when present.
pub struct Vertices {
    pub positions: Vec<[f32; 3]>,
    pub normals: Option<Vec<[f32; 4]>>,      // w is 1.0 in every Konami FMDL measured; a
                                             //   `.model` import sets 1.0, an export drops it
    pub tangents: Option<Vec<[f32; 4]>>,     // w is the handedness (+-1 in Konami files)
    pub bitangents: Option<Vec<[f32; 3]>>,   // `.model` only; an FMDL export drops them (loss)
    pub colors: Option<Vec<[u8; 4]>>,
    pub uvs: Vec<Vec<[f32; 2]>>,
    pub uv_high_precision: Vec<bool>,        // parallel to `uvs`; FMDL only, false elsewhere
    pub bone_indices: Option<Vec<[u8; 4]>>,  // into `Mesh::bone_group`; unused slots 0
    pub bone_weights: Option<Vec<[f32; 4]>>, // floats; FMDL's bytes are `/255` on import and
                                             //   re-quantized on export so the total is kept
    pub bone_weight_width: Option<u8>,       // `.model` stores 2, 3 or 4 weights; `None` = 4
}

pub struct MeshGroup {
    pub name: String,                // `.model` has no groups: import makes one per mesh named
    pub parent: Option<usize>,       //   after the mesh, export names the mesh after its group
    pub meshes: Vec<usize>,
    pub visible: bool,
}

pub struct Texture {
    pub directory: String,           // verbatim from the source (`/Assets/.../sourceimages/`,
    pub file_name: String,           //   `./`); extension kept as found. Final in-game paths are
}                                    //   the Team compiler's texture relocation step, not this crate's

pub struct BoundingBox { pub min: [f32; 4], pub max: [f32; 4] }
```

Shapes this settled after reading the format crates (each is a decision entry): vertices are a
struct of arrays, not a `Vec<Vertex>`; `Bone.children` is gone (derivable, and a second copy to
keep in step through folds and prunes); `Mesh` carries no `alpha_flags`/`shadow_flags`/anti-blur
fields (lifted to the material's `fox` table, see "Engine mapping") and no `split_group_id` (the
importers call the format crates' split *decode*, so an imported mesh is whole unless its group
was too large to be, see below; a glTF `PES_mesh`
identity is Phase 7's); `extension_headers` are sets of raw header lines, since both format
crates already type the known headers; the template display positions (`start`/`end`) wait for the
Blender export in Phase 7. `Affine` is the crate's own 3×4 row-major matrix type (`affine.rs`:
multiply, invert, transform point/direction), because the only matrix work in the suite is bone
transforms and a dependency on `nalgebra` would add a generic API for four functions.

**What real files carry (the conversion census).** Every `.model` and FMDL on the maintainer's
machine that its format crate reads was converted to the other engine and back (2026-09-28,
`.tmp/2_20/convert_census/`: 3695 `.model` bundles with their `.mtl`, 4088 FMDLs). About one in six
failed on rules the IR had assumed. They now read as follows:

- **Weights need not sum to 1.** Community exports routinely store unnormalized weights: `u8`
  bytes that do not total 255, and `f32` weights totalling 0.5 to 4 (879 files, most of them
  far past quantization noise). They render as stored, so the IR keeps them as stored. `validate`
  requires only that each weight is finite and not negative. A weight above 1 (a `.model`
  can store one) is kept. The FMDL export, whose `u8` cannot store it, clamps it to 1 with a
  `weight_clamped` finding, silently when the excess is float noise (at most `1e-6`, which is
  what `f32` accumulation leaves).
- **Fox bones are made parent-first on import**, as `.model` bones already are ("Bone order"
  below). 79 community FMDLs list a child before its parent, never in a cycle, as the game's
  own `body.skl` does. `fmdl_to_ir` moves a bone right after its parent (stable otherwise) and
  remaps the bone groups.
- **A split group that cannot be whole stays split.** In 114 files a `Split-Mesh` group's
  components together reference more than 65536 distinct vertices, which `u16` faces cannot
  hold. The format crates' split decode leaves such a group as it is, and the importer takes
  its components as separate meshes, the split marker dropped as for any decoded group (the
  target's export re-splits any component over its own limits). So an imported mesh is whole
  unless its source group was too large to be. The `u16` face type itself is still Phase 7's
  question (glTF brings `u32` indices).
- **Names the `.mtl` cannot carry.** Four corrupted FMDLs name a material with NUL bytes, which
  XML cannot represent even escaped. The pre-Fox export refuses such a name
  (`ConvertError`) rather than write a `.mtl` no reader accepts.
- **A mesh without vertices** (face exports carry one holding a 33-bone group as a marker)
  skins nothing, so the FMDL export writes it with an empty bone group
  (`empty_mesh_bone_group_dropped`, Info) instead of splitting it into nothing.

**What a `.model` import drops** (reported through `loss.rs`, never silently): Konami's lower LOD
face lists (no 4cc export carries any; the add-on writes none), Konami's `(kind, text)` tags, the
editor-data items and the geometry order word. Everything the community add-on writes survives.

The material side of the IR (`materials/mod.rs`, re-exported by `ir`):

```rust
// Mirrors the materials.toml schema (see the Unified model format plan): an
// engine-neutral core plus one optional table per engine. `Some` when imported
// from that engine's format or from a toml that fills it; exporters derive a
// missing engine table from `family` via the shader-family defaults.
pub struct Material {
    pub name: String,
    pub family: MaterialFamily,                  // Shaded | Shadeless | Metal | Glass
    pub two_sided: Option<bool>,
    pub transparent: Option<bool>,
    pub antiblur: Option<bool>,
    pub textures: Vec<(TextureRole, usize)>,     // canonical role → index into `textures`
    pub parameters: Vec<(String, [f32; 4])>,     // engine-neutral shader parameters
    pub fox: Option<FoxMaterial>,
    pub prefox: Option<PreFoxMaterial>,
}

pub struct FoxMaterial {
    pub shader: String,
    pub technique: String,
    pub alpha_flags: u8,                         // per-mesh in FMDL; see the format plan's flag rule
    pub shadow_flags: u8,
    pub cast_shadow: Option<bool>,               // Fox-only booleans over shadow_flags bits 1/2
    pub invisible: Option<bool>,
    pub base_linear: bool,
    pub textures: Vec<(String, usize)>,          // native sampler name → texture, non-canonical only
    pub parameters: Vec<(String, [f32; 4])>,
}

pub struct PreFoxMaterial {
    pub shader: String,
    pub states: Vec<(String, u32)>,              // ztest, zwrite, twosided, alphatest, alpharef, alphablend, blendmode
    pub samplers: Vec<(String, SamplerSettings)>, // native sampler name → srgb/filters/addressing/maxaniso
    pub textures: Vec<(String, usize)>,          // native sampler name → texture, non-canonical only
                                                 //   (`RoughnessMap` on 312 Konami materials, `Normal2`, ...)
    pub parameters: Vec<(String, Vec<f32>)>,     // .mtl <vector> elements, one to four components as stored
}

// The `.mtl` sampler attributes minus name and path (which live in `Texture`); the same
// closed sets `pes_model::format::mtl` reads, redeclared here because `materials/` imports no
// format crate.
pub struct SamplerSettings {
    pub srgb: Option<bool>,
    pub minfilter: Option<Filter>, pub maxfilter: Option<Filter>,
    pub magfilter: Option<Filter>, pub mipfilter: Option<Filter>,
    pub uaddr: Option<Address>, pub vaddr: Option<Address>, pub waddr: Option<Address>,
    pub maxaniso: Option<u32>,
}
pub enum Filter { Linear, Point, Anisotropic }
pub enum Address { Wrap, Clamp, Repeat }
```

(`states` are `u32` and `parameters` keep their stored component count, as the `.mtl` codec
reads them, so a `.model` round trip loses nothing; the plan's draft had `i32` and `[f32; 4]`.)

FMDL stores four-component normals and tangents; the IR must not silently discard their fourth
components.

**Deterministic collections.** Order-bearing IR metadata uses `BTreeMap`/`BTreeSet` (or equivalently
sorted vectors), never serialization by `HashMap`/`HashSet` iteration. Importers normalize extension
keys, and exporters/merge routines emit them in canonical order. This is required by the Team
compiler's byte-reproducible Rust-output contract.

### Material sources per format

The three formats keep material data (shader, texture paths, render states) in
different places; the IR's `Material` is the merge point:

- **FMDL** embeds materials in the model binary — `fmdl_to_ir` reads them from
  the file itself. FMDL texture path tables carry names the game resolves by stem,
  appending `.ftex` at load time whatever a stored name's extension, so FMDL has
  always been extension-agnostic.
- **.model** keeps them in the sibling **`.mtl` XML**, with meshes binding
  materials by name. A pre-Fox native source is therefore an atomic bundle
  (`.model` + `.mtl`): `model_to_ir` consumes both and `ir_to_model` emits both.
  Conversion never changes one without the other. The `.mtl` codec lives in
  `pes_model` alongside the `.model` codec (it's the same format family; the
  Team compiler's MTL validation and path rewriting use the same parser).

**Engine mapping.** The IR `Material` is engine-neutral in its core (`family`,
booleans, canonical texture roles, parameters) with an optional native table per
engine. Importers fill the native table of their own engine verbatim and infer
`family` from the shader; exporters use the target engine's native table when
present and otherwise derive it from `family` through the shader-family defaults
and role→sampler tables in the format plan. So a `.model` → IR → FMDL conversion
picks the Fox shader from the family (as `model2fmdl.py` does today with its
`Basic_C` → `fox3ddf_blin` table), while an FMDL → IR → FMDL round-trip never
touches the shader at all. Per-mesh FMDL flags are lifted to the material on
import (splitting instances whose meshes differ) and pushed back down on export.

What the native importers derive for the *other* engine: `fmdl_to_ir` sets `two_sided` from alpha
bit 32 and leaves `transparent` unset. It does not read bit 128 as transparency: a census of 1983
Konami FMDLs found bit 128 on 128/160 of `fox3ddf_blin`'s meshes in near-equal share with 32/0,
i.e. on ordinary opaque skin, and the legacy converter judged transparency from the texture's alpha
channel instead, which the format plan rules out. So a Fox part converted to pre-Fox gets the opaque
state set unless its family is `glass`; what bit 128 means on deferred shaders is an open question.
`model_to_ir` sets `two_sided` from the `twosided` state and `transparent` from `alphablend`.
`ir_to_fmdl` adds the game's `dummy_nrm`/`dummy_srm` textures (`/Assets/pes16/model/character/
common/sourceimages/`, the same dummies the anti-blur materials use) to a `shaded`/`metal` material
missing its normal or specular map, as the working 16→21 converter does; the compiler's texture
step replaces them when the folder has real maps. Only a material without a `fox` table gets them:
a Fox-native material that ships without the maps (Konami's audience parts) is left as the game
shipped it, so a same-format round trip adds no textures. Bone positions an FMDL export lacks are derived
from `Bone.matrix` (see the `Bone` comment); the root's `local_position` is `[0, 0, 0, 1]`, a guess
consistent with the audience fixture's `sk_belly` and unverified in game (the working converter
wrote zero for every bone, so the game appears not to read the field when the template `.skl` is
injected).

What the `.model` pair carries that the IR does not, and how the pre-Fox importer and exporter
treat it:

- **Bone order.** IR bones are parent-first (a one-pass invariant every consumer relies on, and
  the order the SKL and FMDL files use). A `.model` lists bones in Konami's order, which is not
  always parent-first under the render hierarchy: 44 of 2606 Konami files list `dsk_forearm_l`
  before its render parent `dsk_forearm_t_l`. `model_to_ir` therefore places a bone right after
  its parent when the file lists it earlier (everything else keeps file order) and remaps the bone
  groups; vertex indices are untouched. A bone's `parent` is its render parent when that bone is
  in the model, else `None`, as the legacy converters did (no climbing to a present ancestor: in
  914 files `skf_brow_*` hangs under the absent `skf_glabella`, and the legacy made them roots).
- **Stored matrices** are the row-major 3x4 inverse bind, the `Affine` layout: on the fixtures
  the inverse of the stored matrix equals PES17 `body.skl`'s bind pose within 1.3e-5 (7.2e-5 on
  the add-on-written card head). A singular stored matrix is an error, not a finding.
- **Normals and tangents** are three components; the import widens with `w = 1.0` on both, as
  the working 16→21 converter does. Whether the tangent `w` should instead be the handedness
  read off the `.model`'s bitangent is an open question for converge (the converter's output is
  accepted in game as is). Bitangents are carried in the IR and written back verbatim; a Fox
  source has none and the export writes none, as the 19→16 converter did.
- **Faces** reverse winding on import and again on export (the IR keeps FMDL's).
- **Lower LOD levels, Konami tags, editor data, a nonzero mesh `order`, nonzero model
  `flags`**: not in the IR (no 4cc export carries them, and a tag changes nothing the game
  draws: PES 17 draws the stock captain armband's `DCaptainmark`-tagged mesh on a
  non-captain, in Edit mode and in a match, exactly as it draws the same mesh with the tag
  renamed, measured at 4.y-ingame2). Each non-default one the import drops
  is a `native_field_dropped` finding whose `detail` names the field; the export writes no
  LODs (`LodRecord::for_levels(0)`), no tags, `order` 0, `flags` 0, and recomputes the mesh and
  model bounds from the positions.
- **A hidden Fox mesh** (`invisible`, shadow flag bit 2, the add-on's "invisible" box) is
  left out of a `.model` export: a `.mtl` cannot hide a mesh, and unhidden it draws what
  its author hid (the blank head's oral mesh in `addon_oral.fmdl`, whose `dummy_bsm`
  material resolves on no pre-Fox path). No finding: the game showed nothing of it on
  Fox, so nothing a member saw goes missing. Members hide meshes on purpose, as
  placeholders: of 35,251 meshes in the 7,315 FMDLs on the maintainer's machine, 61 are
  hidden, and in 38 files every mesh is (26 blank-head `oral.fmdl`, 8 one-sided
  `glove_*.fmdl`, 3 stadium shadow models); the rest are shadow-only glove meshes and a
  few face parts. An export left with no mesh is `ConvertError::EveryMeshHidden`, which
  a consumer treats as a model that draws nothing, not as an empty model. The compiler
  never reaches it: its deep pass drops such a file before any task converts or names it
  (`team_compiler/messages.md` `model_hidden_dropped`), since the task converting a model
  is not always the one naming it (a Common model's linking face, a shared folder's
  merge), and the player's other models stand. Findings keep naming meshes by their source index.
- **Mesh names.** `.model` has no groups; the import makes one group per mesh named after the
  add-on's mesh name, `mesh_<index>` when the mesh has none (every Konami mesh), and the export
  names each mesh after the group that lists it (an add-on kind-128 annotation), `None` when no
  group does. So a Konami `.model` round trip gains `mesh_<index>` names; the game ignores them.
- **Per-mesh extension headers** carry over minus the codec markers (`Split-Mesh` is consumed
  by the split decode, the vertex-loop marker by the export's re-encode).
- **The `.mtl`.** Each material name the model binds must have a definition, else an error; a
  definition no mesh binds (Konami's `accessory.mtl` defines many parts' materials in one file)
  is not carried, so the exported `.mtl` holds exactly the model's materials, in model order.
  Sampler paths split at the last `/` into `Texture`; entries are written samplers first, then
  states, then vectors: Konami's own order in 1217 of the 1265 materials that mix samplers and
  states (census over 941 PES 2017 `.mtl`, 3102 materials; 14 put states first, `glasses_02T`
  among them), so a round trip of those 14 regroups entries and changes nothing else.
- **Indices without weights.** A `.model` mesh may store bone indices and no weights (Konami's
  cards and glasses: slot 0 binds the vertex fully). The IR keeps one representation, indices
  and weights together, so the import synthesizes `[1, 0, 0, 0]` per vertex and the export writes
  the weights out (`QuadFloat32`): the same binding in the other of the format's two spellings,
  not a loss, so no finding.
- **`transparent` and the stored states.** `model_to_ir` reads `transparent` off `alphablend`
  alone, so on export the boolean owns `alphablend` alone when a `prefox` table is present (as
  `two_sided` owns `twosided`); the `zwrite`/`alphatest`/`alphablend` triple is set as a whole
  only when the states come from the family default set. Otherwise a stored `alphatest: 1,
  alphablend: 1` (the add-on's card heads) would come back as `alphatest: 0`, rewritten by the
  very boolean read from it.

**Stem-based texture references (input).** All three formats reference textures
by **stem** (filename without extension) in their **source/authoring** form; the
compiler resolves each stem to whatever accepted image file exists in the model
folder and converts to the target format at compile time (format list, stem
conflicts, and auto-naming rules: format plan, "Dependencies and images" and
"Textures"). Studio-format `.mtl` and `.materials.toml` files write stems; FMDL
path tables resolve by the same stem (a stored name's extension, when present, is
decorative, the game appending `.ftex` either way). The planned Export upgrader strips
extensions from old `.mtl` texture paths when migrating. **Output** `.mtl` files (written by
`ir_to_model` at compile time) keep the `.dds` extension on texture paths, as
the game expects; FMDL output path tables remain stem-based (the game appends
`.ftex`). Auto-naming and auto-detection of texture roles happen during deep
material parsing (not the structure pass), so a missing texture reports
`mtl_texture_not_found` / `fmdl_texture_not_found` at that stage, not during
shallow validation.

### Extension algorithms (one implementation each)

Each format extension (mesh splitting, anti-blur, split vertex encoding) lives in the format crate
that owns it, not in `model_convert`. The IR importers/exporters call the format crate's
decode/encode functions. This keeps same-format round-trips (e.g. FMDL→Blender→FMDL via the PyO3
bindings) lossless and IR-free, while cross-format conversions still benefit from a single
implementation:

| Algorithm | Owner crate | Called by (decode) | Called by (encode) |
|-----------|-------------|---------------------|---------------------|
| FMDL mesh splitting | `fmdl` | `fmdl_to_ir` | `ir_to_fmdl` |
| .model mesh splitting | `pes_model` | `model_to_ir` | `ir_to_model` |
| FMDL anti-blur | `fmdl` | `fmdl_to_ir` | `ir_to_fmdl` |
| Split vertex encoding (FMDL) | `fmdl` | `fmdl_to_ir` | `ir_to_fmdl` |
| Split vertex encoding (.model) | `pes_model` | `model_to_ir` | `ir_to_model` |

A bug fix in FMDL mesh splitting decode benefits every path starting from FMDL, regardless of target
format — and also benefits the Blender addon's direct FMDL round-trip, which calls the same `fmdl`
function without going through the IR at all.

The split vertex encoding is a property of the vertex *order* (loops of one vertex are consecutive
runs), and the IR keeps the vertex order, so its "decode" on import is a read the importers have
nothing to call: the owner map is recoverable from the IR at any time. The exporters recover it
with the format crate's per-mesh `vertex_enc::decode` on the IR order, *not* the flag-gated
`decode_model`: the IR carries no extension flag, and a run that satisfies the convention is a set
of loops of one vertex whether or not the file declared the extension (an add-on file's loops
survive the round trip; on a Konami file the same rule merges only vertices that share position,
weights and an increasing attribute order, which is what a loop is). The encode on export is not
a no-op: it groups every vertex sharing a topological key contiguously, and Konami files are not
written that way (highneck: 198 of 204 vertices move; the vertex multiset and the face corner-tuples
are unchanged). The legacy converters reorder the same way. A same-format round trip is therefore
compared against the input with the same encoders applied, never byte for byte (see "Testing: IR
roundtrips").

`fmdl` also owns **multi-FMDL mesh merging** — bone lists unioned by name (a shared bone's
bounding box is the union of the parts' boxes, since a bone's box covers every vertex weighted
to it; per-mesh bone groups remapped), meshes and mesh groups concatenated, materials merged by name (same-name materials with
differing definitions rejected), buffers and headers rebuilt. A part with repeated bone names
is refused (union by name is ambiguous), and so are parts whose anti-blur requests are encoded
in some and not in others: the merged model declares anti-blur only when every requesting part
carries its duplicates, so a later `antiblur::encode` never skips one. It is not an IR operation: the Team
compiler calls it directly to assemble multi-part models and to bake Common-linked models into
player FMDLs on Fox targets, where the engine cannot load models from the Common folder (see "Common
model links and model merging" in the [Team compiler plan](../team_compiler/README.md)). The pre-Fox
counterpart is `pes_model::ops::merge`, native over `Model` + `MaterialSet` (see
"`pes_model::ops::merge`" in the [Libraries plan](../libs/format_crates.md)), which follows the same rules.

### Conversion routing

```rust
/// A model in one of the native formats, at the semantic layer of its format crate.
pub enum NativeModelBundle {
    Fox { model: fmdl::Model, skl: Option<fmdl::SklFile> },
    PreFox { model: pes_model::model::Model, mtl: pes_model::format::mtl::MaterialSet },
    // Gltf(GltfModel) joins in Phase 7
}

/// The bundle in `target`'s format (`target.engine()`), on `target`'s skeleton: the input
/// itself when it already is both, otherwise source → IR → `retarget` → target.
/// `Converted::findings` lists what the target could not keep (`loss.rs`), so the caller
/// reports rather than the user discovers.
pub fn convert(bundle: NativeModelBundle, target: PesVersion) -> Result<Converted, ConvertError>;

/// Whether `convert` would go through the IR: another engine, a bone the bundle's groups
/// use that `target` lacks, or a vertex blending bones whose deltas to `target`'s tables
/// differ (bone names, poses and the weight rows; no vertex position read).
pub fn needs_conversion(bundle: &NativeModelBundle, target: PesVersion) -> bool;

pub struct Converted { pub bundle: NativeModelBundle, pub findings: Vec<loss::Finding> }
```

`convert` takes a PES *version*, not a format: the format follows from the engine, and the
skeleton retargeting pass (see `conversion.md` "Skeleton retargeting and bone conformance") runs inside the same
IR round trip, so every model the compiler emits is on the target's skeleton without a second
pass. The same-format short cut is decided by `needs_conversion`, the plan's native pre-check: the
bundle's bone names (those its bone groups reference) against the target's tables, poses compared
the way `retarget` does (source pose from the SKL or PES21's tables for Fox, the inverted inline
matrix for pre-Fox; 3e-3 on any component). `convert` takes the bundle by value and returns a new
one, so a pre-Fox result exists only once both its `.model` and its `.mtl` were produced: an error
can never leave a converted `.model` paired with an old or missing `.mtl`. (The plan's first draft
mutated a bundle in place; by value says the same thing with the type system instead of a
comment.) The Fox side carries the companion `.skl` when one exists, which is where `fmdl_to_ir`
reads the bind pose from (see "Skeleton reconstruction from FMDL"); `ir_to_fmdl` returns one when
an exported bone sits outside PES21's template tables or off their pose (a custom bone,
or a shared bone at the hand-table pose a retarget gave it).

**Unskinned meshes on Fox.** FMDL needs every vertex bound to a bone: an unbound mesh stays where
the model is placed instead of following the player. A pre-Fox mesh without bone indices (the
ancient templates the 16→21 converter met) therefore gets, on `ir_to_fmdl`, a bone named `static`
(identity matrix, `global_position` `[0.2, 0, 0, 1]`, `local_position` zero: the converter's
values, the name Konami's own skeleton lacks so the game leaves the mesh static) added once to the
model, with every such mesh's vertices fully weighted to it (`static_bone_added`, one finding per
mesh). The bone is not in any table, so the export also emits an SKL, which the compiler packs as
the slot's skeleton in place of the template, as it packs a member's `.skl` of that stem
(`team_compiler/pipeline.md` step 3 "Format conversion"): the template lacks the bone.

### Roundtrip testing

The IR enables lossless roundtrip tests for same-format paths (see `testing.md` "Testing: IR roundtrips").
Cross-format roundtrips (FMDL→.model→FMDL) are lossy in known, accepted ways (anti-blur regenerated,
mesh splits may differ). The IR preserves the metadata so the losses are explicit, not accidental.

### Data loss behavior

The existing direct converters are already lossy:
- FMDL→.model: anti-blur stripped (.model doesn't support it)
- .model→FMDL: anti-blur regenerated with defaults
- Both: bone names remapped through `missingBones` table
- Both: mesh splitting undone and redone
- FMDL→.model: custom bone transforms replaced by the template skeleton's (the converter never
  read the companion `.skl`)

Routing through the IR doesn't add new data loss, and removes the last one: bind transforms travel
in `Bone.matrix` from the `.skl` or the `.model`'s inline bone table to whichever format is written
(see "Skeleton in `.model`"). The other losses happen at different pipeline points. The IR
preserves metadata (e.g., `Material.antiblur`), so if exporting back to FMDL, the metadata is
available.

---
