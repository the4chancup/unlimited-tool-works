# Unlimited Tool Works — Model conversion plan

Covers the `model_convert` lib crate (the IR plus importers/exporters and skeleton
data) and the format-extension algorithms that live in the `fmdl` and `pes_model`
format crates (mesh splitting, split vertex encoding, anti-blur). Model format
conversion is not a standalone tool — it runs inside the
[Team compiler](../team_compiler/README.md) at compile time, converting models to the target
PES version's format. The glTF + `materials.toml` authoring format that `gltf_to_ir`
reads and `ir_to_gltf` writes (Phase 7) is specified in the [Unified model format
plan](../model_format.md). Platform context is in the [core plan](../core/README.md).

---

The plan is split into parts; section headings are unchanged from the
single-file plan.

| Part | Covers |
|---|---|
| [Intermediate representation](ir.md) | Intermediate Representation (IR) |
| [Hand auto-split](hand_split.md) | Hand auto-split |
| [glTF source](gltf.md) | glTF as the universal model source |
| [Model format conversion](conversion.md) | Model Format Conversion |
| [Testing](testing.md) | Testing: IR roundtrips; Testing: conversion against reference outputs |

## Crate layout

The crate is a hub: one IR in the middle, one importer/exporter pair per format around it, and
IR-level operations that never know which format the data came from. The layout enforces the two
boundaries the design depends on — **format code never touches the IR's internals beyond the public
struct, and IR operations never touch a format** — and keeps the format crates (`fmdl`,
`pes_model`) as dependencies, never dependents.

```
crates/libs/model_convert/src/
├── lib.rs              # re-exports
├── convert.rs          # NativeModelBundle, Converted, convert, needs_conversion
├── affine.rs           # Affine: 3×4 row-major bone transform (multiply, invert, apply)
├── ir/                 # the canonical model — a data structure, not a behavior
│   ├── mod.rs          #   CanonicalModel, Mesh, Vertices, Bone, MeshGroup, Texture
│   └── validate.rs     #   IR invariants (indices in range, weights finite and non-negative, one skin per mesh)
├── formats/            # one module per format: to_ir + from_ir, nothing else
│   ├── mod.rs          #   ConvertError, Imported
│   ├── fmdl/           #   fmdl_to_ir (import.rs) / ir_to_fmdl (export.rs; calls the fmdl crate's ops
│   │                   #   for splitting/encoding); mod.rs re-exports and the shared helpers
│   └── pes_model/      #   model_to_ir (import.rs) / ir_to_model (export.rs; + .mtl pairing; commits
│                       #   both or neither); same layout
├── materials/          # the engine-neutral material schema (spec: Unified model format plan)
│   ├── mod.rs          #   Material, MaterialFamily, TextureRole, FoxMaterial, PreFoxMaterial
│   ├── family.rs       #   family inference from a native shader name (the format plan's table)
│   ├── to_fox.rs       #   family → FMDL shader/technique/flags/params; role → sampler
│   └── to_prefox.rs    #   family → .mtl shader (Basic_* ladder), state sets, samplers
├── skeletons/          # per-version skeleton data and retargeting
│   ├── mod.rs          #   PesBone, Skeleton, VersionSkeletons, skeletons(version): the embedded
│   │                   #     .skl files, parsed once at first use through fmdl's SKL codec
│   ├── render_parents.rs   # the hand-transcribed render hierarchy (see `conversion.md` "Skeleton data")
│   ├── fold.rs         #   fold table (bones a version lacks → the bone that takes their weight)
│   └── retarget.rs     #   retargeting + bone conformance (the cross-version IR operation)
├── ops/                # IR-level operations, format-agnostic
│   ├── mod.rs
│   └── hand_split.rs   #   split_by_skeleton_group (gloves hand auto-split)
└── loss.rs             # data-loss reporting: what a target format cannot represent, as findings
```

Each `formats/<format>/` directory also holds its `tests.rs`.

Placement rules:

- **`formats/*` are the only modules that convert through `fmdl`, `pes_model`, or `gltf`**,
  with three named exceptions that read the format types without converting: `convert.rs`
  carries them in `NativeModelBundle` and reads their weight rows for the pre-check,
  `ops/hand_split.rs`'s detection does the same (`fox_has_hand_weights`,
  `prefox_has_hand_weights`), and `skeletons/` uses `fmdl::SklFile`, the SKL codec, which
  lives in `fmdl` only because SKL is a Fox-engine file. Everything else sees the IR. A
  format detail needed by an op is a field on the IR, not an import.
- **`ir/` has no format logic.** Validation plus the index bookkeeping the importers and
  ops share (parent-first ordering, bone-list rebuilds and weight remaps). Transformations
  are `ops/`; conversions are `formats/`.
- **`skeletons/` holds no transcribed numbers.** The `.skl` files under the repository's
  `resources/skeletons/` are embedded with `include_bytes!` and parsed on first use
  (`std::sync::LazyLock`); the only hand-maintained tables are the render parents and the fold
  table, both names only. Provenance
  stays with the `.skl` files, never with Python literals. (An earlier version of this plan generated
  a `data.rs` of Rust literals; that is a 250 KB source file plus a generator plus a drift test, for
  data the SKL codec already reads byte-exactly.)
- **`materials/` is shared by all three formats** and by the Blender project's glTF codec contract;
  it must remain independent of `formats/` so the schema can be tested without any model file.
- **Same-format paths do not enter this crate** unless an explicitly planned IR operation is
  requested (hand split, part merge, retargeting); ordinary FMDL→FMDL processing uses the `fmdl`
  crate directly from the Team compiler. This is a consumer-side rule, restated here because the
  temptation is to add a "convenient" pass-through.
- Stays `wasm32`-checkable and GUI-free (consumer today: the Team compiler; the Balls
  compiler, Export upgrader and Player aesthetics editor join it in the plans).

---
