//! The real bytes of the files `aesthetics_export::test_exports` adds to make an export clean,
//! for tests that write an export on disk.

use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use aesthetics_export::test_exports::CleanFiles;

/// The bytes of the files a clean export lacks, real enough that `check` and `compile` find
/// nothing in them: built once, shared by every test.
pub fn clean_files() -> &'static CleanFiles {
    static FILES: OnceLock<CleanFiles> = OnceLock::new();
    FILES.get_or_init(|| {
        let materials = String::from_utf8(pre_fox_fixture("cardhead_materials.mtl")).unwrap();
        assert!(materials.contains("./texture.dds"), "{materials}");
        let materials = materials.replace("./texture.dds", "./skin.dds");
        CleanFiles {
            fox_player: vec![("face_high.fmdl".to_owned(), clean_model())],
            pre_fox_player: vec![
                (
                    "face_high.model".to_owned(),
                    pre_fox_fixture("cardhead_face_high.model"),
                ),
                ("face_high.mtl".to_owned(), materials.into_bytes()),
                (
                    "skin.dds".to_owned(),
                    fixture("tests/fixtures/textures/single_level.dds"),
                ),
            ],
            portrait: fixture(
                "tests/fixtures/tracer/studio/egg Midcup Tracer/Players/05 - The Chad Stormworks Player/portrait.dds",
            ),
            // At 512 x 512 the logo needs no upscale, so it adds no finding.
            logo: dds_convert::encode_png(&[0, 128, 0, 255].repeat(512 * 512), 512, 512).unwrap(),
        }
    })
}

/// A Fox model in which `fmdl`'s check finds nothing (the tracer's right glove), for a test
/// whose model only has to be one: both commands read every model, so an empty file would be
/// `model_broken`. Its hand bones are renamed from `skh_` to `skx_`, so a face model written
/// with it is not hand auto-split: the glove's vertices weigh on its finger bones, which would
/// give the player gloves and a `model_hand_split` the test is not about.
pub fn clean_model() -> Vec<u8> {
    let bytes = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tracer/studio/egg Midcup Tracer/Players/05 - The Chad Stormworks Player/glove_r.fmdl"),
    )
    .unwrap();
    let mut model = fmdl::Model::from_file(&fmdl::FmdlFile::read(&bytes).unwrap()).unwrap();
    for bone in &mut model.bones {
        if let Some(finger) = bone.name.strip_prefix("skh_") {
            bone.name = format!("skx_{finger}");
        }
    }
    model.to_file().unwrap().write()
}

/// The bytes of `relative`, below the crate's folder.
fn fixture(relative: &str) -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}

/// The bytes of `pes_model`'s fixture `name`.
fn pre_fox_fixture(name: &str) -> Vec<u8> {
    fixture(&format!("../../libs/pes_model/tests/fixtures/{name}"))
}
