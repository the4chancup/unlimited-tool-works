# Unlimited Tool Works — Model conversion plan: Hand auto-split

Part of the [Model conversion plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Hand auto-split

PES glove models use the **hand skeleton**, which has bones only up to the wrists and forearms —
separate from the body skeleton used by face and boots models, though they share wrist/forearm bones. On Fox, the hand skeleton is
inaccessible from the face folder, so users must provide gloves as separate model files; on pre-Fox,
gloves can coexist in the face folder (typed as `gloveL`/`gloveR` in `face.xml`), but the hand
geometry still has to be a separate model loaded with the hand skeleton. In both cases, the user
must manually cut the hands at the wrist from a full-body model and export them separately — the one
remaining authoring annoyance that the unified player folder format doesn't otherwise address.

The compiler eliminates this by **auto-splitting** any model whose vertices carry
hand-skeleton-exclusive bone weights into separate body and glove parts at compile time. The user
authors a single full-body model (e.g., `face_high.gltf`); the compiler produces the body (face)
model plus `glove_l` and `glove_r` models automatically.

### Detection

The check reads bone assignments **directly from the native format** — FMDL bone groups, `.model`
bone mappings, and in Phase 7 glTF `JOINTS_n`/`WEIGHTS_n` accessors resolved through `skin.joints` — without importing to the IR. The **hand-skeleton-exclusive
bones** are identified by name convention: they are prefixed with `skh_` (the finger bones:
`skh_thumb_mata_l`, `skh_index_dip_r`, ...) and suffixed with `_l` or `_r` for laterality. No
pre-defined list or `PES_SKELETON` lookup is needed — the prefix/suffix match is the complete
identification. The games' own `hand_l.skl`/`hand_r.skl` (PES 18, 19, 21) hold 19 `skh_` bones each
plus five bones shared with the body (`sk_forearm`, `sk_hand`, `dsk_forearm`, `dsk_wrist`,
`dsk_forearm_t`), and the `skf_` prefix belongs to the **face** skeleton (33 bones in `face.skl`);
an earlier version of this plan named `skf_` here, which would have cut faces off at the jaw.

No opt-in is needed: vertices with positive `skh_` weights seed the hand selection. Unused `skh_`
names in a bone list do not trigger splitting; the check follows actual vertex weights.

### Split

When hand weights are detected, import to the IR and perform the following mesh operations
**in-process in Rust**, independently for each hand. Blender's commands describe the intended
selection/separation semantics only: the compiler neither launches Blender nor uses `bpy`, and
compiling hundreds of models must not require a Blender installation or per-model external process.

1. **Select** vertices with any weight on that hand's `skh_` groups (`_l` or `_r`). A weight
   counts from half of Fox's 1/255 step, the threshold the conversion pre-check uses: community
   exporters leave float noise (2^-24) on unused slots, and one such slot on a hand bone must
   not make a body a hand model.
2. **Grow once** along the mesh topology — equivalent to one Blender **Select More** (`Ctrl +`)
   step in vertex mode, bringing in the neighboring row at the wrist. This is not a distance-based
   cut, and shared wrist/forearm weights do not exclude a vertex from the expanded selection.
   A *vertex* here is Blender's: the native formats store one entry per loop, so entries sharing
   position, bone indices and weights (the vertex-loop encoding's topological key) are one vertex
   for selection and growth, and a UV or normal seam at the wrist does not stop the growth.
3. **Separate the selection** into `glove_l` or `glove_r`, equivalent to Blender's `P` → Selection.
   Fully selected faces move with it; the remaining faces stay with the body, retaining copies of
   boundary vertices where needed. Preserve positions, weights, UVs, normals, and materials — no
   geometric cutting, invented wrist plane, or reweighting.

The hands are separated in turn, the left first, as a user in Blender would. The right hand's
selection is grown and separated from what the left hand's separation left, so a face fully
selected by both (a thin wrist bridge) goes to `glove_l` only and is never duplicated.

Prune unused bones, materials and textures from each result (a glove keeps only the materials
its faces use), and remap indices to the target skeleton as usual. The extra
row is part of the rule, not an optional refinement: vertices without finger weights can belong to
the separated glove. Validate against stored expected output prepared with the same select → grow
once → separate operation in Blender on a matching-topology fixture with a connected wrist, not
only already separated hand meshes. Blender is a development-time reference for preparing that
fixture, not part of compilation or the Rust regression-test runtime. Benchmark the in-process
split on a representative batch of hundreds of models without Blender installed.

Models without positively weighted hand vertices **pass through unchanged** in this step. The
native weight check needs no IR import or re-export; the original model is retained.

### Pipeline integration

The auto-split applies to the models categorization makes **face content** (see "Player folder
categorization" in the [Team compiler plan](../team_compiler/README.md)): the body stays face
content, and the gloves join the player's `glove_l`/`glove_r` parts as if they were authored as
separate files. The rest of the pipeline (merge, ID assignment, packing) is transparent to
whether the gloves were auto-split or authored. A model named as boots or gloves, and every model
of a shared `Boots/` or `Gloves/` folder, is never split, whatever its weights: an authored glove
is all hand vertices, so splitting it would carve its forearm off as face content, and a boots
model is on the body skeleton already (the one boots model with hand weights found on the
maintainer's machine, FNG's `Boots/k2214 - Park`, carries 9 stray vertices per hand, which a
split would cut into two tiny gloves). There is no weight or vertex-count threshold: no face
model on the maintainer's machine carries stray hand weights, and a full-body model's are
thousands (the `fcl_hair.fmdl` of Red's sample face folder `00007 - BLANK`: 1,684 per hand of
9,563 vertices).

On pre-Fox the split happens in the face task alone, since the face's `face.xml` lists gloves
beside the face (there is no gloves task without `ingame_face`, and under the marker no model is
face content, as on Fox, so none is split). A folder shipping its own `face.xml` is left as
written, the member's xml saying what the face loads and a split only adding glove entries they
did not write, so no model there is split either. The body keeps the model's name and entry; the
hands of `<stem>.model` become `<stem>_glove_l.model` and `<stem>_glove_r.model`, whose suffix
types them `gloveL` and `gloveR` and keeps them clear of an authored `glove_l.model`, each its
own `face.xml` entry. All three name the `.mtl` the source model uses: each keeps a subset of
its materials under their names, so that `.mtl` defines every one, and no new material file is
written. The detection reads the `.model`'s own bone weights, as the FMDL check does. A
hand-weighted model a `.model.common` link brings in is not split on pre-Fox: the face lists
the Common output's file by reference and packs nothing of it, so splitting it would mean
copying it into the face, which no export has needed (worklog "Issues"). A split needs the
model's `.mtl` bytes; a hand-weighted model whose `.mtl` is a Common file (a `.mtl.common`
link, or a `Common/` `.mtl` its search finds) is split with that file, which the face task's
files include since the conversion pre-check reads it for every model
(`team_compiler/pipeline.md` step 3 "Format conversion"), the hands' entries naming it as the
body's does. Until then the case was refused with `model_conversion_failed`, the face reading
no Common file; the refusal's reason went with the pre-check, and one rule covers both.

A split model goes through the IR and back (FMDL → IR → split → FMDL; on pre-Fox `.model` → IR →
split → `.model`). Measured at 4.18 on that
`fcl_hair.fmdl`, the round trip keeps the bones, the materials and every face, and changes two
things nothing in the game reads differently: the meshes of a split-mesh group are re-chunked
(the faces regrouped by bone-group size), and Konami's per-bone matrix block is written empty,
as every community FMDL has it.

### Where it lives

`model_convert`, as an IR-level operation (`split_by_skeleton_group`). It operates on the IR's
normalized bone mapping and identifies hand bones by the `skh_` name convention. It is a one-way
semantic transformation, not a same-format round-trip, so it does not belong in the
`fmdl`/`pes_model` format crates (which own format-native operations for lossless round-trips).

### Scope

Gloves only. Boots use the same body skeleton as the face model — there is no skeleton-exclusivity
problem to solve, so no auto-split is needed or offered for boots.

---
