//! `upgrade`'s input forms (the `spec/export_upgrader.md` "Input" scenarios): a folder, a `.zip`
//! or a `.7z`, the AET page's one-main-folder packing, and the no-old-export
//! refusal.

use std::fs;

use studio_core::PipelineEvent;

use crate::common::Sandbox;

/// `upgrade <input> --savefile <save> [-o <dir>]`.
fn upgrade(sandbox: &Sandbox, input: &str, save: &str, output: Option<&str>) -> crate::common::Run {
    let input = sandbox.arg(input);
    let save = sandbox.arg(save);
    let mut args = vec!["upgrade", &input, "--savefile", &save];
    let dir;
    if let Some(output) = output {
        dir = sandbox.arg(output);
        args.extend(["-o", &dir]);
    }
    sandbox.run(&args)
}

// EU-IN-01
#[test]
fn the_three_input_forms_upgrade_the_same() {
    let sandbox = Sandbox::new("in_forms");
    sandbox.copy_fixture_dir("co Spring", "folder/co Spring");
    sandbox.copy_fixture("co Spring.zip", "zip/co Spring.zip");
    sandbox.copy_fixture("co Spring.7z", "7z/co Spring.7z");
    let before: Vec<Vec<u8>> = [
        sandbox.path("folder/co Spring/co Note.txt"),
        sandbox.path("folder/co Spring/Other/readme.txt"),
        sandbox.path("folder/co Spring/Thumbs.db"),
        sandbox.path("zip/co Spring.zip"),
        sandbox.path("7z/co Spring.7z"),
    ]
    .iter()
    .map(|path| fs::read(path).unwrap())
    .collect();
    sandbox.save("EDIT");

    let mut runs = Vec::new();
    for (input, output) in [
        ("folder/co Spring", "folder-out"),
        ("zip/co Spring.zip", "zip-out"),
        ("7z/co Spring.7z", "7z-out"),
    ] {
        runs.push(upgrade(&sandbox, input, "EDIT", Some(output)));
    }

    for run in &runs {
        assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    }
    let stdout: Vec<String> = runs.iter().map(|run| run.stdout()).collect();
    assert_eq!(stdout[0], stdout[1], "{stdout:?}");
    assert_eq!(stdout[1], stdout[2], "{stdout:?}");
    assert!(
        stdout[0].contains("- co Spring: Warning leftover_dropped at Other/readme.txt"),
        "{}",
        stdout[0]
    );
    // The Note's kit entries give kits without configs.
    assert!(
        stdout[0].contains("- co Spring: Warning kit_without_config at co Note.txt (slot=p1)"),
        "{}",
        stdout[0]
    );
    assert!(
        stdout[0].trim_end().ends_with("- Info migration_clean"),
        "{}",
        stdout[0]
    );
    assert!(!stdout[0].contains("Thumbs.db"), "{}", stdout[0]);
    for run in &runs {
        let found = run
            .events
            .iter()
            .filter(|envelope| matches!(envelope.event, PipelineEvent::UnmigratedContentFound))
            .count();
        assert_eq!(found, 0, "a leftover is no unmigrated content");
    }

    // The three outputs hold the same files with the same contents.
    let mut contents: Vec<Vec<(String, Vec<u8>)>> = Vec::new();
    for output in [
        "folder-out/co Full Spring",
        "zip-out/co Full Spring",
        "7z-out/co Full Spring",
    ] {
        let mut files = Vec::new();
        for path in sandbox.listing(output) {
            let full = sandbox.path(output).join(&path);
            if full.is_file() {
                files.push((path, fs::read(&full).unwrap()));
            }
        }
        contents.push(files);
    }
    assert_eq!(contents[0], contents[1], "{contents:?}");
    assert_eq!(contents[1], contents[2], "{contents:?}");

    // Every input file is unchanged.
    let after: Vec<Vec<u8>> = [
        sandbox.path("folder/co Spring/co Note.txt"),
        sandbox.path("folder/co Spring/Other/readme.txt"),
        sandbox.path("folder/co Spring/Thumbs.db"),
        sandbox.path("zip/co Spring.zip"),
        sandbox.path("7z/co Spring.7z"),
    ]
    .iter()
    .map(|path| fs::read(path).unwrap())
    .collect();
    assert_eq!(before, after);
}

// EU-IN-02
#[test]
fn an_archives_name_is_its_files_stem() {
    let sandbox = Sandbox::new("in_stem");
    // day5.zip holds the folder "co Spring", whose Note's Team: line names /co/.
    sandbox.copy_fixture("co Spring.zip", "day5.zip");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "day5.zip", "EDIT", None);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(sandbox.path("co Full day5").is_dir(), "{}", run.stdout());
}

// EU-IN-04
#[test]
fn fpk_xml_files_are_left_out_of_the_output_and_the_report() {
    let sandbox = Sandbox::new("in_fpk_xml");
    sandbox.write("co Spring/Faces/XXX05 - Bob/face.fmdl", b"face");
    sandbox.write("co Spring/Faces/XXX05 - Bob/face.fpk.xml", b"<fpk/>");
    sandbox.write("co Spring/Boots/k2180 - Bob/boots.fmdl", b"boots");
    sandbox.write("co Spring/Boots/k2180 - Bob/boots.fpk.xml", b"<fpk/>");

    // `k2180` alone infers the 2176 block: slot 05, Bob's own (`--loose`,
    // `tests/cli/loose.rs`).
    let input = sandbox.arg("co Spring");
    let run = sandbox.run(&["upgrade", &input, "--loose"]);
    let stdout = run.stdout();
    assert!(
        sandbox
            .path("co Full Spring/Players/05 - Bob/face.fmdl")
            .exists(),
        "{stdout}"
    );
    assert!(
        sandbox
            .path("co Full Spring/Players/05 - Bob/boots.fmdl")
            .exists(),
        "{stdout}"
    );
    for entry in sandbox.listing("co Full Spring") {
        assert!(!entry.to_ascii_lowercase().ends_with(".fpk.xml"), "{entry}");
    }
    assert!(
        !stdout.to_ascii_lowercase().contains(".fpk.xml"),
        "{stdout}"
    );
}

// EU-IN-03
#[test]
fn an_input_holding_no_old_export_is_reported() {
    let sandbox = Sandbox::new("in_no_export");
    fs::create_dir_all(sandbox.path("co Spring")).unwrap();
    fs::write(sandbox.path("co Spring/readme.pdf"), b"not an export").unwrap();
    sandbox.save("EDIT");
    let before = sandbox.listing("");

    let run = upgrade(&sandbox, "co Spring", "EDIT", None);
    assert_eq!(run.exit_code(), 1);
    assert!(
        run.stdout().contains("- co Spring: Error no_old_export"),
        "{}",
        run.stdout()
    );
    assert_eq!(sandbox.listing(""), before, "nothing is written");
}
