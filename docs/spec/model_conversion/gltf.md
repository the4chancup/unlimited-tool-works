# Unlimited Tool Works — Model conversion plan: glTF source

## glTF as the universal model source

### glTF export (`ir_to_gltf`) and dual-set superset merge

**Skeleton reconstruction from FMDL.** The FMDL bone data carries only local and global positions
(Vector4) — not the 3×3 rotation matrix. The full bind-pose transform (rotation + translation)
lives in the companion `.skl` file. `fmdl_to_ir` resolves the bone transforms as follows:

1. If the FMDL has a companion `.skl` (same basename, per the Aesthetics export plan's SKL pairing rule),
   read the full 3×4 transforms from the SKL via the `fmdl` crate's SKL codec.
2. If no companion SKL exists, use the embedded template skeleton (`body.skl`) for the standard
   bone transforms, supplemented by the FMDL's own position data.

**Skeleton in `.model`.** The pre-Fox format needs no sidecar: a `.model` stores its bone name
table and, per bone, a 3×4 **inverse bind matrix** inline (section 0's first entry, one 12-float
record per bone in name order — `ModelFile.parseBoneGroups` in the `pes-model` addon), which is
exactly the data an FMDL keeps in its `.skl`. What it lacks is the hierarchy: parents are resolved
by looking each bone name up in the template skeleton's render parents (the addon's `addBone`), so
a bone unknown to the template is a root. `model_to_ir` inverts the stored matrices into
`Bone.matrix`; `ir_to_model` writes the inverse of `Bone.matrix` back. What ends up in `Bone.matrix`
at export time is decided by the skeleton retargeting pass (see "Skeleton retargeting and bone
conformance"): standard bones carry the **target version's** bind pose — which is what a `.model`
must contain to skin correctly against that game's runtime skeleton — and only bones unknown to the
target's tables (a genuine custom skeleton) keep the model's own matrices. **This deviates from the
19to16 converter**, which always wrote PES17's matrices (`fmdl2model.py`, identity for unknown
bones) regardless of target and flattened custom skeletons. In the other direction, `.model` → Fox
generates a companion `.skl` from the IR when an exported bone sits outside PES21's template
tables or off their pose (a custom bone, or a glove's shared bones at the hand-table pose); a
model wholly on the template gets none, and the compiler's template injection stands.

### IR part merge (`merge_ir_parts`)

`merge_ir_superset` (Phase 7) merges *one part across two formats*; it never adds geometry. Assembling
*several parts into one model* is a different operation, and it is **format-native on both
engines**: `fmdl::ops::merge` for Fox, `pes_model::ops::merge` for pre-Fox (the `ingame_face`
boots merge, and a shared link combined with local parts under `ingame_face` — see the [Aesthetics
export plan](../aesthetics_export/README.md)'s "ingame_face marker"), with the same rules, so a same-format
merge never round-trips through the IR. An earlier version of this plan routed the pre-Fox cases
through the IR-level `merge_ir_parts`; that was dropped because it made them the one
exception to "same format skips the IR" for no gain (the `.mtl` merge is a material-name merge
either way).

### glTF dependency and image contract

- **PES texture boundary:** `dds_convert` selects codecs by PES version and material role,
  independently of the model's source format. BC7 is supported on PES 19–21, not PES18 despite
  its Fox model format. PES 15–18 convert BC7/raster inputs to BC3 or eligible opaque-color BC1;
  PES 19–21 retain supported BC7 and encode ordinary raster sources to BC7. The [library
  crates plan](../libs/README.md) owns alpha/mipmap/normal-map rules and the CPU encoder choice.
  Already-compatible blocks are retained where only DDS/FTEX container adaptation is needed.
  `model_convert` supplies pixels and material roles, not final player/team paths; the run-level
  `BuildManifest` and texture-relocation step own those destinations.

### Blender integration

Same-format round-trips (FMDL→Blender→FMDL, .model→Blender→.model) are lossless because they call
the format crate directly and never route through the IR.

The compiler materializes ZIP entries lazily or decompresses solid 7z data into an archive-owned
buffer without creating a mutable extraction tree, so archive-level compression handles submitted
size and the format needs none of its own (see "File size and compression" in the format plan).

### Pipeline integration

The compiler detects the model format by file extension (`.fmdl`, `.model`, `.glb`/`.gltf`; a
glTF selected for a stem drops its folder as `model_gltf_unsupported` until the importer lands)
and routes accordingly.
PES-native sources skip the IR for same-format targets — **except** when an IR-level operation applies: hand auto-split (models with
`skh_` bone weights are imported, partitioned, and exported back to the same format) and
cross-version skeleton retargeting (see `conversion.md` "Skeleton retargeting and bone conformance"). Both are lossless
same-format IR round-trips. All other same-format processing (ID replacement, texture path
rewriting, model merging on both engines, validation) is format-native and never touches the IR.

---
