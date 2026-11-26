//! `pes_savefile`'s `test-support` feature gives a tool crate's tests a
//! whole fixture save: `fixture_save`'s bytes are a valid encrypted
//! `EDIT00000000` that `EditFile::load` opens.

use std::path::Path;

use pes_savefile::{EditFile, fixture_save};
use pes_version::PesVersion;

#[test]
fn fixture_save_is_a_whole_encrypted_save() {
    // Under CARGO_TARGET_TMPDIR like the `common::Sandbox` tests use.
    let dir = test_scratch::fresh(Path::new(env!("CARGO_TARGET_TMPDIR")), "fixture_save");
    let path = dir.join("EDIT00000000");
    std::fs::write(&path, fixture_save(PesVersion::Pes21)).expect("the bytes write");
    let save = EditFile::load(&path).expect("the fixture opens");
    assert_eq!(save.version(), PesVersion::Pes21);
    assert_eq!(save.team(714).expect("team 714").name, "/co/");
}
