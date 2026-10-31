//! Provenance of `crates/tools/team_compiler/tests/fixtures/deep/boots_far.fmdl`:
//! the tracer export's `boots.fmdl` with the first vertex of its first mesh moved to
//! (6000, 0, 0), which is 1000 units past the limit of `fmdl_vertex_far_from_origin`.
//!
//! Not part of the workspace. Run it as a bin of a scratch crate that depends on `fmdl` by
//! path (`.tmp/phase_04/4_7/fmdl_census/` was the one used), from the repository root:
//! `far_fixture <tracer boots.fmdl> <out>`. It writes the output once it has read it back
//! and checked that exactly one vertex is far and that nothing else about the model changed.

use fmdl::check::check;
use fmdl::{FmdlFile, Model};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [source, out] = args.as_slice() else {
        panic!("far_fixture <source fmdl> <out fmdl>")
    };
    let bytes = std::fs::read(source).expect("the source reads");
    let original = Model::from_file(&FmdlFile::read(&bytes).expect("an FMDL")).expect("a model");
    let far_before: Vec<_> = check(&original)
        .into_iter()
        .filter(|finding| finding.code == "fmdl_vertex_far_from_origin")
        .collect();
    assert!(far_before.is_empty(), "the source has no far vertex");

    let mut moved = original.clone();
    moved.meshes[0].vertices.positions[0] = [6000.0, 0.0, 0.0];
    let written = moved.to_file().expect("the model writes").write();

    let read_back = Model::from_file(&FmdlFile::read(&written).expect("an FMDL")).expect("a model");
    let far: Vec<_> = check(&read_back)
        .into_iter()
        .filter(|finding| finding.code == "fmdl_vertex_far_from_origin")
        .collect();
    assert_eq!(far.len(), 1, "one finding");
    assert_eq!(far[0].count, 1, "one far vertex");
    assert_eq!(read_back.meshes[0].vertices.positions[0], [6000.0, 0.0, 0.0]);
    // Nothing else changed: with the vertex moved back, the model is the source's.
    let mut restored = read_back.clone();
    restored.meshes[0].vertices.positions[0] = original.meshes[0].vertices.positions[0];
    assert!(restored == original, "only the one position differs");
    // The other findings are the source's own.
    let others = |model: &Model| -> Vec<&'static str> {
        check(model)
            .into_iter()
            .map(|finding| finding.code)
            .filter(|code| *code != "fmdl_vertex_far_from_origin")
            .collect()
    };
    assert_eq!(others(&read_back), others(&original));

    std::fs::write(out, &written).expect("the output writes");
    println!("{} bytes (source {}), other findings: {:?}", written.len(), bytes.len(), others(&original));
}
