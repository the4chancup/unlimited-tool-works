# Unlimited Tool Works — Model conversion plan: Model format conversion

Part of the [Model conversion plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Model Format Conversion

**Conversion is not offered as a standalone tool.** It is integrated into the Team compiler: exports
may contain models in the native formats (`.fmdl`, `.model`; `.glb`/`.gltf` are detected too and
refused until the glTF importer lands, `model_gltf_unsupported`), and the pipeline converts them to
the target PES version's format at compile time. The logic lives in the `model_convert` lib
crate.

### What the converters do

**FMDL → .model (19→16):**
1. Parse FMDL binary → FMDL object
2. Decode mesh splitting (reassemble sub-meshes)
3. Decode split vertex encoding (restore vertex order)
4. Decode anti-blur (strip duplicate meshes)
5. Remap bones through `missingBones` table (FMDL has more bones than .model) — in the Studio, the
   per-version retargeting pass (see "Skeleton retargeting and bone conformance")
6. Convert vertices/faces (reverse face winding, Vector4→Vector3 normals)
7. Convert materials (FMDL shader/technique → .model shader)
8. Encode split vertex encoding
9. Encode mesh splitting for .model format
10. Write .model binary

**.model → FMDL (16→21):**
1. Parse .model binary → Model object
2. Decode mesh splitting
3. Decode split vertex encoding
4. Build bone hierarchy (lookup in PesSkeletonData, set parent/children)
5. Convert vertices/faces (reverse winding, Vector3→Vector4 normals/tangents)
6. Create static bone for boneless meshes (fox PES requires bone assignment)
7. Convert materials (.model shader → FMDL shader/technique, assign textures, calculate alpha/shadow
   flags)
8. Calculate bone bounding boxes
9. Create mesh groups
10. Encode anti-blur (generate duplicate meshes)
11. Encode split vertex encoding
12. Encode mesh splitting for FMDL format
13. Write FMDL binary

### Performance-critical operation: mesh splitting

Mesh splitting is a graph partitioning algorithm that cuts meshes exceeding hardware limits (32
bones, 65535 vertices, 21845 faces) into sub-meshes. It involves:
- Building bone-to-vertex maps
- Greedy graph growing: iteratively grow sub-meshes by adding adjacent faces, checking bone count
  constraints
- Vertex deduplication across sub-mesh boundaries
- Face reordering

Rules both format crates' split ops follow, from the conversion census (see `ir.md`, "What real
files carry"):
- **Decode leaves a group that cannot be whole.** When a group's combined mesh would reference
  more than 65536 distinct vertices (114 community files), `split::decode` leaves that group's
  components and markers as they are and combines the other groups. It is not an error. The
  same holds for a group whose components are not one mesh: their material, vertex layout or
  `.model` weight width disagree, a `.model` component carries LOD levels, or the bones they
  use together pass the 256 a bone index can name. Of these, only differing materials were
  found, in 1 cup file, `70210 - Panzer vor/parts_body.model`. Such a group was never one mesh
  and renders as its components.
- **Encode never leaves an empty container.** A mesh over a limit that yields no components
  (no faces and no vertices: a face export's bone-marker mesh) stays as it is, unsplit.
- **FMDL `Split-Mesh-Groups` over-flagging.** Some writers also list root groups, or empty
  groups, in the header (4 community files). A listed group is a split container only when
  it has a parent and at least one mesh. Any other listed group reads as an ordinary group,
  and a rewrite does not list it.

This is O(V×F) in the worst case — tight loops over vertex arrays with per-iteration set operations.
This is where Rust's performance is essential:

| Language | Estimated time per complex model |
|----------|--------------------------------|
| Python | 1–5 seconds |
| Rust | 10–50 milliseconds |

### Skeleton data

`PesSkeletonData.py` (55KB) is hardcoded bone matrices, parent relationships, and positions for
*one* PES skeleton (its matrices equal PES17's `body.skl` transforms to 5e-16, measured). The Studio
replaces it with the games' own skeleton files, **one set per PES version**, embedded in the
binary (a few KB each) and exposed through one lookup:

```rust
pub struct PesBone {
    pub name: String,
    pub skl_parent: Option<String>,    // the SKL file's own parent_index, resolved to a name
    pub render_parent: Option<String>, // the mesh-splitting/Blender hierarchy (`render_parents.rs`)
    pub matrix: Affine,                // model-space 3×4 bind transform from the SKL
}

/// One skeleton file, bones sorted by name for lookup.
pub struct Skeleton { pub bones: Vec<PesBone> }
impl Skeleton { pub fn bone(&self, name: &str) -> Option<&PesBone>; }

pub struct VersionSkeletons {
    pub body: Skeleton,
    pub face: Skeleton,                // Fox versions ship face.skl; pre-Fox uses PES19's
    pub hand_l: Skeleton,              // likewise hand_l.skl / hand_r.skl
    pub hand_r: Skeleton,
    pub body_skl_bytes: &'static [u8], // the verbatim game file, for Fox template injection
}

pub fn skeletons(version: PesVersion) -> &'static VersionSkeletons;   // PES20 = PES21
```

The plan's first draft had `phf::Map`s of `&'static` data generated into a source file; a sorted
`Vec` with a binary search over at most 175 names is as fast as matters and needs neither a
generator nor a dependency. The Blender display positions (`start`/`end`, `PesSkeletonData`'s
`_displayPositions`) are not skeleton data the games ship; they join `PesBone` in Phase 7 with the
glTF exporter that needs them.

**Provenance.** Every number comes from a game file, never from a transcription. The files
themselves are checked in under `resources/skeletons/pes{15,16,17,18,19,21}/` (extracted once,
stored decompressed; see the README there), embedded with `include_bytes!` and parsed once at first
use with `fmdl::SklFile` — the Python literals are not copied. The one hand-maintained skeleton
table is `render_parents.rs`, names only: `PesSkeletonData`'s render hierarchy (139 entries, body,
hands and face), which no game file records. A bone outside that table is a render root, which is
what every `dsk_*` bone already is in the games' own parent columns. Where each version keeps its
player skeletons in the game data:

| Version | Archive | Path | Files |
|---|---|---|---|
| PES15 | `dt32.cpk` | `common/character1/model/character/body/body.skl` (WESYS-compressed) | body only |
| PES16 | `dt32_win.cpk` | same path | body only |
| PES17 | `dt32_win.cpk` | same path | body only |
| PES18 | `dt00_x64.cpk` | `Asset/model/character/#Win/common_package.fpk` → `/Assets/pes16/model/character/common/{body,face,hand_l,hand_r,boots}.skl` | body, face, hands, boots |
| PES19 | `dt00_x64.cpk` | same | same |
| PES21 | `dt00_x64.cpk` | same | same |

PES20 is not installed here and was used in one 4cc event only (core plan, "Project context");
**PES20 is assumed to equal PES21** until
its files are checked. Pre-Fox ships no hand or face SKL (its remaining `.skl` files are cutscene
mobs/props and 2D overlays), which is why `.model` files carry only names and matrices and leave the
hierarchy to the table; the hand and face hierarchies for pre-Fox targets come from PES19's files,
as `PesSkeletonData.py` already did. The `render_parent` column is `PesSkeletonData`'s own (the SKL
files' `parent_index` makes almost every `dsk_*` bone a root, so it cannot serve as a hierarchy for
mesh splitting or Blender armatures).

**The body skeleton is not one skeleton.** Parsing all six `body.skl` files side by side:

| Version | Bones | Compared with PES21 |
|---|---|---|
| PES15 | 76 | 12 exclusive bones (`dsk_belly`, `dsk_chest`, `dsk_upperarm_{b,m,t}_{l,r}`, `tip_belly`, `tip_chest`; `dsk_thigh_{l,r}`, which return in PES19); **lacks** `dsk_deltoid_*`, `dsk_trapezius_*`, `dsk_upperarm_long_*`; 20 shared bones have a different rest pose — `dsk_scapula_r` (1.89 max component delta), `dsk_scapula_l`, `dsk_pectoralis_*` (0.52), `dsk_belly_scale`, and the **whole forearm/hand chain** (`sk_forearm_*`, `sk_hand_*`, `dsk_wrist_*`, `dsk_forearm*_*`, `dsk_elbow_*`, 0.18–0.50) |
| PES16 | 70 | identical bone set and transforms to PES17 |
| PES17 | 70 | `dsk_deltoid_*` rest pose differs (0.25) |
| PES18 | 76 | PES17 + `dsk_belly_{f,o,ba}_{l,r}` (as roots; PES21 hangs them under `dsk_pos_belly_*`) |
| PES19 | 116 | adds `dsk_pos_*` helpers, `dsk_hip/leg/pants/thigh_*`, `dsk_upperarm_skin_*`; **8 bones PES21 lacks**: `dsk_pos_{1,2,3}_wrist_*`, `dsk_pos_clavicle_*`, `dsk_pos_trapezius_*` |
| PES21 | 175 | adds sleeves, sternum, underarm, kneeback, thighmain, hem fakes, `pos_arm_target_*`; not a superset of PES19 |

Consequences: PES21 is the right *Fox injection template* (Red's practice; older Fox games tolerate
extra bones in an injected `.skl`), but it is the wrong source of **bind matrices for a `.model`**
and the wrong **bone-set reference** for any version but itself.

### Skeleton retargeting and bone conformance

A model authored against one version's skeleton and compiled for another has two distinct problems,
which the legacy tools conflated:

- **A. Bones the target lacks.** Their weights must be transferred to a bone the target has.
- **B. Bones the target has at a different rest pose.** The mesh is bound to the *source* pose:
  its inverse bind matrices (inline in a `.model`, in the glTF skin, in an FMDL's companion `.skl`
  or the template) describe where each bone was when the vertices were authored. If those matrices
  do not match the target game's runtime skeleton, vertices rotate about the wrong pivots — fine at
  rest, wrong in any pose that flexes the joint.

The `4cc-model-simplifier-15` (`Engines/python/simplify.py`) and the 19to16 converter are the
prototypes. Both solve A with a hand-written table (`missingBones`: `dsk_deltoid_*` →
`dsk_upperarm_*`, `dsk_trapezius_*` → `sk_shoulder_*`, `dsk_upperarm_long_*` → `dsk_upperarm_*`).
Neither solves B: both write **PES17's** matrices (`PesSkeletonData`) into models destined for other
versions, and the simplifier's "full" mode papers over the worst B cases by transferring weights
away from the displaced bones (`movedBones`: `dsk_belly_scale` → `sk_belly`, `dsk_pectoralis_*` →
`sk_chest`, `dsk_scapula_r` → `sk_shoulder_r`) — losing the deformation those bones provide. The
forearm/hand chain, displaced just as much in PES15, was never handled, which is the probable cause
of the **broken wrist pose in some PES15 animations (the pre-match entrance) on 17→15 ports** — a
pose-dependent error is exactly what wrong bind pivots produce. *Hypothesis; verify on a known-broken
port once the pass exists.*

The Studio runs one IR pass, `retarget(ir: CanonicalModel, target: PesVersion) ->
Result<(CanonicalModel, Vec<loss::Finding>), ConvertError>` (`skeletons/retarget.rs`; the error
is a singular bind matrix, which no importer lets through, or the final `validate`). It takes
the model by value, so an error can never leave a half-retargeted model behind. It runs in every
export path after conversion and before mesh
splitting. A bone is *standard* when some version's tables (any body, face or hand skeleton) name
it, *custom* otherwise; only standard bones the target lacks are folded, and custom bones pass
through untouched, matrices included. The source bind pose is already in the IR's
`Bone.matrix` (the importers put it there); the pass rewrites the bones and vertices in place:

1. **Source bind pose.** `.model`: the inline matrices, inverted. glTF: `skin.inverseBindMatrices`.
   FMDL: the companion `.skl`; without one, the source is assumed to be **PES21**'s skeleton (the
   only Fox versions in 4cc use are 18/19/21 and their shared bones differ by ≤0.25 on `dsk_deltoid`
   and the toe tips; FMDL files do not record their game version).
2. **Fold bones the target lacks** (problem A). For each used bone absent from the target's body
   table, transfer its weight to the bone named by the **fold table** for that bone — one table for
   every version, name → name, in `skeletons/fold.rs`: the two legacy tables verbatim, the
   PES15-only bones (`dsk_chest` → `sk_chest`, `dsk_belly` → `sk_belly`, `dsk_upperarm_{b,m,t}_*` →
   `dsk_upperarm_*`, `tip_belly`/`tip_chest` → `sk_belly`/`sk_chest`), PES19's `dsk_pos_*` helpers
   (`dsk_pos_*_wrist_*` → `sk_hand_*`, `dsk_pos_<bone>_*` → `dsk_<bone>_*` or the `sk_` bone
   the helper sits on), and PES21's additions whose name spells the body part (`dsk_sleeve*`,
   `dsk_underarm*` → `sk_upperarm_*`; `dsk_hem_*_fake_*` → `dsk_hem_*_*`; `dsk_kneeback_*` →
   `sk_leg_*`; `dsk_thighmain_*` → `sk_thigh_*`; `dsk_sternum_*` → `sk_chest`). Entries beyond the
   legacy tables are name-based judgment, not game-verified. A table entry whose target the version
   also lacks is **followed as a chain** (`dsk_upperarm_long_l` → `dsk_upperarm_l` → PES15 has it;
   `dsk_pos_trapezius_l` → `dsk_trapezius_l` → `sk_shoulder_l` on PES15); a unit test walks every
   entry against every version and fails on a chain that ends nowhere or loops. A removed bone with
   no entry (PES21's `dsk_back`, the `pos_arm_target_*` IK helpers) falls back to the **nearest
   target body bone by rest position** (the model's own bind translation against the target
   table's). Every fold of a weighted bone is reported, one finding per bone with `detail`
   `"<bone> -> <target>"`: `bone_folded_for_version` from the table, `bone_folded_by_position`
   for the fallback (a guess the user should hear about); an unweighted removed bone just
   disappears. Bone groups and vertex mappings are remapped exactly as the simplifier's
   `simplifyModel` does: the folded bone leaves the bone list and every group, the target bone is
   appended where absent (to the model with the target's matrix, to the group), and a vertex whose
   slots now name one bone twice merges the weights into the first slot. Children of a removed bone
   take its parent. The SKL parent chain is *not* used: almost every `dsk_*` bone is a root in the
   games' own files.
3. **Re-bind to the target pose** (problem B). For every surviving bone, re-pose the vertices from
   the source rest pose into the target's — `v' = Σᵢ wᵢ · B_target,ᵢ · B_source,ᵢ⁻¹ · v` over the
   vertex's weights, the blend divided by their sum when they do not total 1 (community weights
   are kept as stored), normals/tangents by the rotational part — and write the **target
   version's**
   inverse bind matrices into the emitted `.model` bone table or generated `.skl`. Weights are
   untouched: `dsk_scapula_r` keeps driving the shoulder blade, sitting where PES15 expects it.
   Reported once per model as `skeleton_retargeted` (I, `detail` the moved-bone count) when any
   bone's delta `Dᵢ` differs from the identity by more than a tolerance (3e-3 on any component:
   Konami's own PES 17 glove `g101` sits 0.0025 off the hand table on three finger bones, and the
   smallest difference between two versions' body tables is 0.0042, one hem bone of PES 15
   against 17, so 3e-3 clears the games' own files and still catches every version difference;
   measured at 4.17g1);
   a bone within tolerance is left exactly as it was, and a vertex none of whose bones moved is
   not touched, so a same-version compile is a no-op by construction, float noise included.
   Normals, tangents (xyz; `w` kept) and bitangents take the blended rotation and are
   renormalized. Bones the target's tables do not know at all (a genuine custom skeleton) keep
   the model's own matrices, as before.

   **When re-binding changes anything.** Under linear-blend skinning the game draws a vertex at
   `Σᵢ wᵢ · Aᵢ · Dᵢ · v` (`Aᵢ` the animation about the rest pose, `Dᵢ` the bone's delta from the
   model's bind pose to that rest pose); a re-bound model is drawn at `Σⱼ wⱼ · Aⱼ · Σᵢ wᵢ · Dᵢ · v`.
   The two are equal for a vertex weighted to one bone, and for a vertex whose bones all share
   one delta: such a model is moved rigidly, never distorted, in every pose. They differ only
   for a vertex that blends bones whose deltas differ: that vertex flexes about the wrong pivots
   once the joint animates, which is problem B. So **a model is on the target's skeleton when no
   vertex blends bones with differing deltas**, whatever its deltas are, and that is the test the
   native pre-check runs (`needs_conversion`): per bone the delta to the target's table, then one
   pass over the vertex weights. A pre-Fox weight below half of Fox's 1/255 step counts as no
   weight: community exporters leave float noise (2^-24) on the unused slots of a vertex weighted
   whole to one bone (225 vertices of the parity tracer's boots carry it on the other leg's
   bone), and a weight the Fox format would round to zero moves the drawn vertex by nothing
   visible on either engine. The games' own files show why the per-bone comparison the plan
   first described is wrong (measured at 4.17g1 on the stock PES 2017 and PES 2021 files and the
   pre-Fox parity tracer): Konami's boots sit on a **boots pose** 7° off the body table on every
   version, both feet in one mesh with opposite per-leg deltas, and the community's PES 15-posed
   gloves sit 0.36 off PES 17's arm with every bone sharing one delta; all of them draw right,
   and a per-bone check would have re-bound every one.
4. **Hands** conform against the target's `hand_*` tables (Konami's gloves match them to 0.0025,
   measured; the tables are byte-identical across PES 18-21 and pre-Fox ships none, so PES19's
   serve every version). **A face's `skf_*` bones are not conformed and not re-bound**: their
   pose is per face (Konami's PES 2017 faces sit up to 0.28 off `face.skl` and 0.26 off each
   other; a PES 2021 face 0.34 off), the base table moved by the player's own face diff, so no
   shared table is a reference for them; they keep the model's own matrices, as custom bones do,
   and count as neither moved nor blended in the pre-check. A face's body bones (`sk_head`,
   `sk_neck`, `dsk_*`) conform against the body table, which every stock face matches to 0.0000.

The two legacy tables become regression fixtures: PES19→PES16 and PES17→PES15 must fold exactly the
`missingBones` those tools folded, and must *not* fold the `movedBones` (retargeting handles them).
Retargeting is also what makes the **PES19→PES21** compile correct — today Red leaves PES19's eight
`dsk_pos_*` bones dangling in a PES21 FMDL; whether the game ignores or misrenders them is untested,
and folding makes it moot. A manual per-folder override for fold targets is deliberately not
specified: the table plus nearest-bone fallback should cover real exports, and `settings.toml` is
kept to appearance settings; revisit if a case turns up.

**Cost.** Re-binding is linear-blend skinning evaluated once, offline. Per surviving bone one delta
matrix `Dᵢ = B_target,ᵢ · B_source,ᵢ⁻¹` is precomputed (≤ 175 small matrix products); per vertex the
position is `Σ wᵢ Dᵢ v` over at most four weights (≈ 50 multiply-adds) and the normal/tangent take
the rotational part (≈ 40 more) — on the order of 100 flops per vertex, done in place. A 20k-vertex
player model is ~2 M flops, well under a millisecond single-threaded; the largest 4cc models
(~100k vertices) stay in single-digit milliseconds. It is not measurable next to mesh splitting
(10–50 ms per model) or texture encoding (hundreds of ms), and it runs at all only when the
native-format pre-check (`needs_conversion`: bone names, the two versions' tables and one pass
over the vertex weights, no vertex position read) finds a used bone that the target lacks or a
vertex blending bones whose deltas to the target differ beyond tolerance (item 3, "When
re-binding changes anything") — same-version compiles, and cross-version compiles of models
whose blended bones moved together, never enter the pass. What it does
add for a **same-format** cross-version compile (a PES21-authored FMDL for PES18) is the IR
round-trip it needs to run at all, which is why it is listed with hand auto-split as an explicit
exception to the "same format skips the IR" rule; that round-trip
is the lossless one the plan already tests, and it costs a few milliseconds of parse/serialize.

### SKL binary format

Fox-engine `.skl` files carry the bone list and bind-pose transforms for a corresponding `.fmdl`.
Most models use the compiler's template skeletons (`body.skl` → `boots.skl` / `fcl_hair_sim.skl`);
custom `.skl` files are rare and only needed for models with a custom pose. The binary format is
simple (reference parser: `examples/skl.py`):

```
Header (12 bytes — 3 u32s):
  u32 magic       = 12        (version/identifier)
  u32 bone_count  = N
  u32 record_size = 56

Bone records (N × 56 bytes, starting at offset 12):
  u32 name_offset     — byte offset into the file for this bone's null-terminated name
  i32 parent_index    — -1 for root, else 0-based index into the bone array
  f32[12] transform   — 3×4 row-major [rotation_3x3 | translation_3x1]

Name table (at offset 12 + N×56):
  concatenated null-terminated ASCII strings, then trailing bytes
```

The trailing bytes are not fixed. The fmdl census of every SKL on the maintainer's machine
(2026-09-28) found Konami's own `dt00` skeletons ending right after the last name's NUL, most
files padded to 4, and one padded by 4 more. `SklFile` keeps whatever follows the name table as
read and writes it back verbatim, so byte identity holds on every SKL. A skeleton built from
scratch (`SklFile::new`) pads to 4, as the reference writer does.

Each transform row is `[rot_x, rot_y, rot_z, translation]`, giving a 3×3 bind-pose rotation plus a
model-space position per bone. The positions correspond to the `PesSkeletonData._positions`
reference, but template variants can differ numerically; do not require exact equality to that
hardcoded reference. Preserve the SKL parent hierarchy separately from the render hierarchy. The `fmdl` crate owns the SKL codec alongside
the FMDL codec (same format family — both are Fox-engine binary formats consumed together at pack
time).

### SKL and glTF: native integration, not a companion

For Fox-native `.fmdl` inputs staying native, the companion `.skl` can travel as opaque
pass-through bytes. Native→IR conversion and Blender import also decode it for skeleton
reconstruction; the SKL codec is not limited to glTF-authored inputs. The [Aesthetics export plan's "SKL pairing"
section](../aesthetics_export/README.md) describes how paired SKLs travel with their models through links and
merges regardless of source format.

### GPU offloading

**Not worth it.** The workload doesn't match GPU strengths:
- Mesh splitting: branch-heavy, irregular memory access, set operations — GPUs perform poorly
- Vertex conversion: 5K-20K vertices is too small to justify GPU dispatch overhead (~100μs)
- Matrix inversion: ~100 tiny 3×4 matrices — CPU does this in microseconds

Keep model geometry conversion on the CPU and benchmark the port rather than promising specific
latencies. Texture encoding is a separate workload: the [library crates plan](../libs/README.md) includes
first-release desktop GPU BC7 for PNG-heavy PES 19–21 builds, with CPU fallback. That texture-only
exception does not change this plan's CPU geometry processing or deterministic model output.

---
