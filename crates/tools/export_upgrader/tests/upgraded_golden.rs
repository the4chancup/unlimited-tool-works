//! The goldens the parity cases compile, kept honest: `team_compiler`'s
//! `parity.rs` and `parity_prefox.rs` compile the export under each
//! `tests/fixtures/<tracer>/upgraded/` (`parity.rs`'s referee case compiles
//! `tracer_refs`' golden), which is what `upgrade <old>` wrote of the
//! fixture's `old/` (`--loose` for the team tracers, neither flag for a
//! referee export; the fixture's README). Upgrading `old/` must still give
//! exactly that tree, or the parity case compiles output the upgrader no
//! longer writes.

// The cli suite's shared sandbox, reused wholesale; its file-fixture helpers
// (`snapshot`, `assert_refused`, the `save`/`write`/`copy_fixture` methods) are
// the cli tests' and stay unused here, in this target only.
#[expect(dead_code, reason = "this target uses only the sandbox and the report")]
mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use common::Sandbox;

/// `upgrade <fixture>/<tracer>/old/<old> <flags> -o <sandbox>` gives the exit
/// code 0, the report `report` on stdout, and an output tree equal to
/// `<fixture>/<tracer>/upgraded/`, file set and bytes.
fn assert_upgrades_to_golden(
    sandbox_name: &str,
    tracer: &str,
    old: &str,
    flags: &[&str],
    report: &str,
) {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../team_compiler/tests/fixtures");
    let sandbox = Sandbox::new(sandbox_name);
    let old = fixtures.join(tracer).join("old").join(old);
    let output = sandbox.arg("upgraded");
    // The fixture is read in place: the upgrader never writes its input.
    let mut args = vec!["upgrade", old.to_str().unwrap()];
    args.extend_from_slice(flags);
    args.extend(["-o", output.as_str()]);
    let run = sandbox.run(&args);
    assert_eq!(run.exit_code(), 0);
    assert_eq!(run.stdout(), report);

    let ours = tree(&sandbox.path("upgraded"));
    let golden = tree(&fixtures.join(tracer).join("upgraded"));
    let mut differences = Vec::new();
    for path in ours.keys() {
        match golden.get(path) {
            None => differences.push(format!("{path}: the golden does not hold it")),
            Some(bytes) if bytes != &ours[path] => {
                differences.push(format!("{path}: the bytes differ"))
            }
            Some(_) => {}
        }
    }
    for path in golden.keys() {
        if !ours.contains_key(path) {
            differences.push(format!("{path}: the upgrade did not write it"));
        }
    }
    assert!(
        differences.is_empty(),
        "{}\nif the upgrader's output changed on purpose, regenerate `upgraded/` as the fixture's README says",
        differences.join("\n")
    );
}

/// Every file below `root`, `/`-joined relative path to bytes.
fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            if entry.path().is_dir() {
                pending.push(entry.path());
            } else {
                let path = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(path, fs::read(entry.path()).unwrap());
            }
        }
    }
    files
}

#[test]
fn the_fox_tracer_upgrades_to_its_golden() {
    assert_upgrades_to_golden(
        "golden_fox",
        "tracer",
        "egg Tracer",
        &["--loose"],
        "- egg Tracer: Info coverage_tag_chosen (tag=Full, source=default)\n\
         - egg Tracer: Info wearer_assumed at Boots/k2180 - The Chad Stormworks Player (id=2180, slot=05)\n\
         - egg Tracer: Info wearer_assumed at Gloves/g2180 - The Chad Stormworks Player (id=2180, slot=05)\n\
         - egg Tracer: Info antiblur_dropped at Boots/k2180 - The Chad Stormworks Player/boots.fmdl (meshes=1)\n\
         - egg Tracer: Info antiblur_dropped at Faces/XXX05 - The Chad Stormworks Player/fcl_hair.fmdl (meshes=1)\n\
         - Info migration_clean\n",
    );
}

#[test]
fn the_pre_fox_tracer_upgrades_to_its_golden() {
    assert_upgrades_to_golden(
        "golden_prefox",
        "tracer_prefox",
        "jp Tracer",
        &["--loose"],
        "- jp Tracer: Info coverage_tag_chosen (tag=Full, source=default)\n\
         - Info migration_clean\n",
    );
}

#[test]
fn the_referee_tracer_upgrades_to_its_golden() {
    assert_upgrades_to_golden(
        "golden_refs",
        "tracer_refs",
        "refs Tracer",
        &[],
        "- Info migration_clean\n",
    );
}
