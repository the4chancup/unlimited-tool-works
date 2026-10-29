# Unlimited Tool Works — Model conversion plan: Testing

Part of the [Model conversion plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Testing: IR roundtrips

The equality assertions below are schematic: compare normalized geometry, bindings, and retained
metadata, not arbitrary native bytes or glTF document structure. Decoding/re-encoding split meshes,
regenerating anti-blur, or consolidating material files can change representation without changing
the supported semantics. Native codec byte-preservation tests remain separate; unsupported fields
must not disappear under a blanket claim that IR routing is lossless.

```rust
#[test]
fn fmdl_roundtrip_lossless() {
    let original = parse_fmdl("test_data/face_high.fmdl");
    let ir = fmdl_to_ir(&original);
    let roundtripped = ir_to_fmdl(&ir);
    assert_eq!(original, roundtripped);
}

#[test]
fn model_roundtrip_lossless() {
    let original_model = parse_model("test_data/face.model");
    let original_mtl = parse_mtl("test_data/face.mtl");
    let ir = model_to_ir(&original_model, &original_mtl);
    let (roundtripped_model, roundtripped_mtl) = ir_to_model(&ir);
    assert_eq!((original_model, original_mtl), (roundtripped_model, roundtripped_mtl));
}
```

---

## Testing: conversion against reference outputs

Test files with known-good conversions (from the existing converters) serve as reference:
```rust
#[test]
fn fmdl_to_model_matches_reference() {
    let fmdl = parse_fmdl("test_data/face_high.fmdl");
    let ir = fmdl_to_ir(&fmdl);
    let (model, mtl) = ir_to_model(&ir);
    let reference_model = parse_model("test_data/face_high_converted.model");
    let reference_mtl = parse_mtl("test_data/face_high_converted.mtl");
    assert_eq!((model, mtl), (reference_model, reference_mtl));
}
```
