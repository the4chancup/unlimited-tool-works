//! `unpack-session`: the command's wiring — a bundle that does not read or a folder that
//! already holds a file is refused before a byte is written (the unpack itself is covered
//! by `replay.rs`'s tests).

use std::fs;

use crate::Sandbox;

impl Sandbox {
    /// `unpack-session <bundle> <folder>`.
    fn unpack_session(&self, bundle: &str, folder: &str) -> crate::common::Run {
        let (bundle, folder) = (self.arg(bundle), self.arg(folder));
        self.run(&["unpack-session", &bundle, &folder])
    }
}

// A file that is not a zip is `read_bundle`'s refusal: exit 2, the error naming the file,
// the folder never made.
// SE-REP-08
#[test]
fn a_bundle_that_is_not_a_zip_is_refused_naming_it() {
    let sandbox = Sandbox::new("unpack_session_not_zip");
    fs::write(sandbox.path("not-a-zip"), b"not a zip at all").unwrap();

    sandbox
        .unpack_session("not-a-zip", "out")
        .assert_refused(2, &["not-a-zip", "not a zip"]);
    assert!(!sandbox.path("out").exists(), "nothing was written");
}

// A folder already holding a file is refused before the bundle is even read — the zip
// named here does not exist, and is never reached — its file left as it was.
// SE-REP-08
#[test]
fn a_folder_already_holding_a_file_is_refused() {
    let sandbox = Sandbox::new("unpack_session_occupied");
    fs::create_dir_all(sandbox.path("out")).unwrap();
    fs::write(sandbox.path("out/already.txt"), b"stay").unwrap();

    sandbox
        .unpack_session("bundle.zip", "out")
        .assert_refused(2, &["already holds files"]);
    assert_eq!(fs::read(sandbox.path("out/already.txt")).unwrap(), b"stay");
}
