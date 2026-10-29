# Unlimited Tool Works — Model conversion plan: Testing

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Testing: IR roundtrips

What exists of this is in `spec/model_conversion/testing.md` "Testing: IR roundtrips".

The glTF roundtrip extends the spec's two native ones:

```rust
#[test]
fn gltf_roundtrip_lossless() {
    let original_gltf = parse_gltf("test_data/face_high.gltf");
    let original_mats = parse_materials_toml("test_data/materials.toml");
    let ir = gltf_to_ir(&original_gltf, &original_mats);
    let (roundtripped_gltf, roundtripped_mats) = ir_to_gltf(&ir);
    assert_eq!(original_gltf, roundtripped_gltf);       // PES_bone/PES_mesh extensions preserved
    assert_eq!(original_mats, roundtripped_mats);       // material definitions preserved
}
```
