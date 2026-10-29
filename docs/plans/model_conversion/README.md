# Unlimited Tool Works — Model conversion plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Crate layout

What exists of this is in `spec/model_conversion/README.md` "Crate layout".

The planned files extend the spec's tree:

```
crates/libs/model_convert/src/
├── formats/
│   └── gltf/           #   Phase 7
│       ├── mod.rs      #   gltf_to_ir / ir_to_gltf via the gltf crate
│       ├── extensions.rs   # PES_bone / PES_mesh read + write, fallbacks when absent
│       └── images.rs   #   the image contract (embedded images ignored, stems resolved from folder)
├── materials/
│   ├── toml.rs         #   Phase 7: materials.toml / *.materials.toml / .common link read + write
│   └── matching.rs     #   Phase 7: name matching (startsWith/endsWith), layering, Common-first cascade
└── ops/
    ├── merge_parts.rs  #   merge_ir_parts (deferred: no planned caller, see `gltf.md` "IR part merge")
    └── superset.rs     #   Phase 7: dual-set superset merge for ir_to_gltf (Player aesthetics editor)
```
