# Unlimited Tool Works — Model conversion plan: Intermediate representation

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Intermediate Representation (IR)

### When the IR is used

What exists of this is in `spec/model_conversion/ir.md` "When the IR is used".

- **glTF → FMDL**: glTF → IR → FMDL
- **glTF → .model**: glTF → IR → .model
- **FMDL / .model → glTF**: native → IR → glTF (`ir_to_gltf` — the [Player aesthetics
  editor](../player_aesthetics_editor.md)'s conversion; the Team compiler never emits glTF)

### IR struct

What exists of this is in `spec/model_conversion/ir.md` "IR struct".

glTF uses three-component normals and four-component tangents (including handedness),
so conversion must distinguish semantic tangent handedness from native metadata and preserve
otherwise unrepresented native components through the PES metadata path. Roundtrip fixtures cover
non-default fourth components, not only ordinary XYZ data.

### Material sources per format

What exists of this is in `spec/model_conversion/ir.md` "Material sources per format".

- **glTF** keeps them in **TOML files** alongside the glTF, mirroring the
  `.model`/`.mtl` split: the glTF's `primitive.material` references name
  materials, and the PES-specific properties live in `materials.toml` /
  `*.materials.toml` files in the same folder (never embedded in the glTF
  JSON). The file pattern, name matching, layering, `.common` link files, and
  the complete schema are in the [Unified model format plan](../model_format.md).
  `gltf_to_ir` takes the folder's resolved material files as an argument and
  fills the IR's `Material` structs from them; the glTF files themselves are
  never rewritten (exports stay read-only). glTF-only — FMDL and .model keep
  their native mechanisms.
