//! The command line's own rules (the `spec/export_upgrader.md` "CLI" scenarios): the output
//! name's tag, `-o`, the flag and path refusals, and the exit codes.

use std::fs;

use pes_version::PesVersion;

use crate::common::{Sandbox, snapshot};
use crate::players::face;

/// `upgrade <input> <flags>`.
fn upgrade(sandbox: &Sandbox, input: &str, flags: &[&str]) -> crate::common::Run {
    let input = sandbox.arg(input);
    let mut args = vec!["upgrade", &input];
    args.extend(flags);
    sandbox.run(&args)
}

/// A minimal old export at `relative`: an `Other/` folder with a file, plus a
/// `co Note.txt` holding `Team: <team>` when `team` names one.
fn old_export(sandbox: &Sandbox, relative: &str, team: Option<&str>) {
    fs::create_dir_all(sandbox.path(&format!("{relative}/Other"))).unwrap();
    fs::write(sandbox.path(&format!("{relative}/Other/readme.txt")), b"x").unwrap();
    if let Some(team) = team {
        fs::write(
            sandbox.path(&format!("{relative}/co Note.txt")),
            format!("Team: {team}\r\n"),
        )
        .unwrap();
    }
}

// EU-CLI-01, EU-CLI-09
#[test]
fn the_output_name_carries_the_key_the_tag_and_the_rest() {
    let sandbox = Sandbox::new("cli_output_names");
    for name in [
        "co - Spring",
        "co Day 8 Additions",
        "co midcup Day 5",
        "co Full Spring",
    ] {
        old_export(&sandbox, name, Some("/co/"));
        // A face folder, so each run is clean and its output is written.
        face(&sandbox, &format!("{name}/Faces/XXX05 - A"));
    }
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");

    for (input, output, tag, source) in [
        ("co - Spring", "co Full - Spring", "Full", "default"),
        (
            "co Day 8 Additions",
            "co Midcup Day 8 Additions",
            "Midcup",
            "additions",
        ),
        (
            "co midcup Day 5",
            "co midcup Day 5 upgraded",
            "Midcup",
            "name",
        ),
        ("co Full Spring", "co Full Spring upgraded", "Full", "name"),
    ] {
        let run = upgrade(&sandbox, input, &["--savefile", &save]);
        assert_eq!(run.exit_code(), 0, "{input}: {}", run.stdout());
        assert!(sandbox.path(output).is_dir(), "{input} -> {output}");
        assert!(
            run.stdout().contains(&format!(
                "Info coverage_tag_chosen (tag={tag}, source={source})"
            )),
            "{input}: {}",
            run.stdout()
        );
    }
}

// EU-CLI-09
#[test]
fn coverage_flag_sets_the_tag_and_a_conflict_with_the_names_own_is_refused() {
    let sandbox = Sandbox::new("cli_coverage_flag");
    for name in ["co - Spring", "co Day 8 Additions", "co Full Spring"] {
        old_export(&sandbox, name, Some("/co/"));
        face(&sandbox, &format!("{name}/Faces/XXX05 - A"));
    }
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");

    // An unknown value is refused at the parse.
    upgrade(
        &sandbox,
        "co - Spring",
        &["--savefile", &save, "--coverage", "all"],
    )
    .assert_refused(2, &["--coverage", "invalid value"]);

    // The flag sets the tag in place of the name's guess.
    let run = upgrade(
        &sandbox,
        "co - Spring",
        &["--savefile", &save, "--coverage", "midcup"],
    );
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        sandbox.path("co Midcup - Spring").is_dir(),
        "{}",
        run.stdout()
    );
    assert!(
        run.stdout()
            .contains("Info coverage_tag_chosen (tag=Midcup, source=flag)"),
        "{}",
        run.stdout()
    );
    let run = upgrade(
        &sandbox,
        "co Day 8 Additions",
        &["--savefile", &save, "--coverage", "full"],
    );
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        sandbox.path("co Full Day 8 Additions").is_dir(),
        "{}",
        run.stdout()
    );
    assert!(
        run.stdout()
            .contains("Info coverage_tag_chosen (tag=Full, source=flag)"),
        "{}",
        run.stdout()
    );

    // A name whose second word is already a tag keeps it: a flag naming the
    // other tag is refused, and nothing is written.
    let before = sandbox.listing("");
    upgrade(
        &sandbox,
        "co Full Spring",
        &["--savefile", &save, "--coverage", "midcup"],
    )
    .assert_refused(2, &["--coverage midcup", "Full"]);
    assert_eq!(sandbox.listing(""), before, "nothing is written");

    // The flag naming the tag the name already carries is accepted, source
    // `name`.
    let run = upgrade(
        &sandbox,
        "co Full Spring",
        &["--savefile", &save, "--coverage", "full"],
    );
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        sandbox.path("co Full Spring upgraded").is_dir(),
        "{}",
        run.stdout()
    );
    assert!(
        run.stdout()
            .contains("Info coverage_tag_chosen (tag=Full, source=name)"),
        "{}",
        run.stdout()
    );
}

// EU-CLI-09
#[test]
fn a_referee_export_takes_no_coverage_flag_and_reports_no_tag() {
    let sandbox = Sandbox::new("cli_refs_coverage");
    sandbox.write("refs Spring/refs.txt", b"01 arbiter\n");
    sandbox.write("refs Spring/Players/arbiter/boots.fmdl", b"boots");
    let before = sandbox.listing("");

    upgrade(&sandbox, "refs Spring", &["--coverage", "full"])
        .assert_refused(2, &["--coverage", "referee export"]);
    assert_eq!(sandbox.listing(""), before, "nothing is written");

    let run = upgrade(&sandbox, "refs Spring", &[]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        !run.stdout().contains("coverage_tag_chosen"),
        "{}",
        run.stdout()
    );
}

// EU-CLI-08
#[test]
fn an_input_named_with_a_trailing_dot_or_dot_dot_upgrades_beside_it() {
    let sandbox = Sandbox::new("cli_dot_input");
    sandbox.copy_fixture_dir("co Spring", "a/co Spring");
    sandbox.copy_fixture_dir("co Spring", "b/co Spring");
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");
    let inside_a = sandbox.listing("a/co Spring");
    let inside_b = sandbox.listing("b/co Spring");

    for (input, output) in [
        (
            format!(
                "{}{}.",
                sandbox.arg("a/co Spring"),
                std::path::MAIN_SEPARATOR
            ),
            "a/co Full Spring",
        ),
        (
            format!(
                "{}{}..",
                sandbox.arg("b/co Spring/Other"),
                std::path::MAIN_SEPARATOR
            ),
            "b/co Full Spring",
        ),
    ] {
        let run = sandbox.run(&["upgrade", &input, "--savefile", &save]);
        assert_eq!(run.exit_code(), 0, "{input}: {}", run.stdout());
        assert!(sandbox.path(output).is_dir(), "{input}: {}", run.stdout());
    }
    assert_eq!(
        sandbox.listing("a/co Spring"),
        inside_a,
        "nothing is written inside the input"
    );
    assert_eq!(
        sandbox.listing("b/co Spring"),
        inside_b,
        "nothing is written inside the input"
    );
}

// EU-CLI-02
#[test]
fn the_output_goes_inside_the_directory_o_names() {
    let sandbox = Sandbox::new("cli_output_dir");
    sandbox.copy_fixture_dir("co Spring", "in/co Spring");
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");
    let out = sandbox.arg("out");

    let run = upgrade(&sandbox, "in/co Spring", &["--savefile", &save, "-o", &out]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(sandbox.path("out/co Full Spring").is_dir());
    assert!(
        !sandbox.path("in/co Full Spring").exists(),
        "nothing is written beside the input"
    );
}

// EU-CLI-03
#[test]
fn a_team_export_takes_exactly_one_of_savefile_and_loose() {
    let sandbox = Sandbox::new("cli_flag_pair");
    old_export(&sandbox, "co Spring", Some("/co/"));
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");
    let before = sandbox.listing("");

    let both = upgrade(&sandbox, "co Spring", &["--savefile", &save, "--loose"]);
    both.assert_refused(2, &["--savefile", "--loose"]);
    let neither = upgrade(&sandbox, "co Spring", &[]);
    neither.assert_refused(2, &["--savefile", "--loose"]);
    assert_eq!(sandbox.listing(""), before, "nothing is written");
}

// EU-REF-05
#[test]
fn a_referee_export_takes_neither_flag_and_its_note_is_a_leftover() {
    let sandbox = Sandbox::new("cli_refs");
    // A `refs` export's name decides before anything is read (step 9): it
    // takes neither flag, and its Note is a leftover: nothing compiled reads
    // a referee's Note.
    fs::create_dir_all(sandbox.path("refs Spring")).unwrap();
    fs::write(sandbox.path("refs Spring/refs.txt"), b"01 Keeper\n").unwrap();
    sandbox.write("refs Spring/Players/Keeper/boots.fmdl", b"boots");
    fs::write(sandbox.path("refs Spring/refs Note.txt"), b"Team: /co/\n").unwrap();
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");
    let before = sandbox.listing("");

    upgrade(&sandbox, "refs Spring", &["--savefile", &save]).assert_refused(2, &["--savefile"]);
    upgrade(&sandbox, "refs Spring", &["--loose"]).assert_refused(2, &["--loose"]);
    assert_eq!(sandbox.listing(""), before, "nothing is written");

    let run = upgrade(&sandbox, "refs Spring", &[]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        sandbox.path("refs Spring upgraded").is_dir(),
        "{}",
        run.stdout()
    );
    assert!(
        run.stdout()
            .contains("- refs Spring: Warning leftover_dropped at refs Note.txt"),
        "{}",
        run.stdout()
    );
}

// EU-CLI-04
#[test]
fn the_refusals_before_anything_is_written() {
    let sandbox = Sandbox::new("cli_refusals");
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");
    fs::write(sandbox.path("notes.txt"), b"text").unwrap();
    old_export(&sandbox, "co Spring", Some("/co/"));
    fs::write(sandbox.path("not-a-save.bin"), b"no save").unwrap();
    // A savefile that does not hold the resolved team: no fixture save has a
    // team named /newteam/.
    old_export(&sandbox, "newteam Spring", Some("/newteam/"));
    let save18 = sandbox.arg("EDIT18");
    fs::write(
        sandbox.path("EDIT18"),
        pes_savefile::fixture_save(PesVersion::Pes18),
    )
    .unwrap();
    // A filesystem root is not an export, savefile or not.
    let root = sandbox
        .path("co Spring")
        .ancestors()
        .last()
        .unwrap()
        .to_path_buf();
    let not_save = sandbox.arg("not-a-save.bin");
    let before = sandbox.listing("");

    upgrade(&sandbox, "gone", &["--savefile", &save])
        .assert_refused(2, &["gone", "does not exist"]);
    upgrade(&sandbox, "notes.txt", &["--savefile", &save])
        .assert_refused(2, &["notes.txt", "not a folder, a .zip or a .7z file"]);
    upgrade(&sandbox, "co Spring", &["--savefile", &not_save])
        .assert_refused(2, &["not-a-save.bin"]);
    upgrade(&sandbox, "newteam Spring", &["--savefile", &save18]).assert_refused(2, &["/newteam/"]);
    let root_arg = root.display().to_string();
    let run = sandbox.run(&["upgrade", &root_arg, "--savefile", &save]);
    run.assert_refused(2, &["a filesystem root, not an export"]);

    assert_eq!(sandbox.listing(""), before, "nothing is written");
}

// EU-CLI-05
#[test]
fn an_output_folder_that_exists_or_cannot_be_created_is_refused() {
    let sandbox = Sandbox::new("cli_output_exists");
    sandbox.copy_fixture_dir("co Spring", "co Spring");
    // An existing output folder, with a member's edit inside.
    fs::create_dir_all(sandbox.path("co Full Spring")).unwrap();
    fs::write(sandbox.path("co Full Spring/keep.txt"), b"my edit").unwrap();
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");

    let before = snapshot(&sandbox);
    let run = upgrade(&sandbox, "co Spring", &["--savefile", &save]);
    run.assert_refused(3, &["co Full Spring"]);
    assert_eq!(snapshot(&sandbox), before, "nothing is written");

    // An output that cannot be created: -o names a file, not a directory.
    fs::write(sandbox.path("blocked"), b"a file").unwrap();
    let blocked = sandbox.arg("blocked");
    let before = snapshot(&sandbox);
    let run = upgrade(
        &sandbox,
        "co Spring",
        &["--savefile", &save, "-o", &blocked],
    );
    run.assert_refused(3, &["blocked"]);
    assert_eq!(snapshot(&sandbox), before, "nothing is written");
}

// EU-CLI-10
#[test]
fn a_staging_folder_left_by_an_earlier_run_is_removed_and_reported() {
    let sandbox = Sandbox::new("cli_staging_exists");
    sandbox.copy_fixture_dir("co Spring", "co Spring");
    // An abandoned staging folder of an earlier run: the run removes it,
    // reports it, and writes its output.
    fs::create_dir_all(sandbox.path("co Full Spring.upgrading")).unwrap();
    fs::write(
        sandbox.path("co Full Spring.upgrading/left.txt"),
        b"left behind",
    )
    .unwrap();
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", &["--savefile", &save]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(sandbox.path("co Full Spring").is_dir(), "{}", run.stdout());
    assert!(
        !sandbox.path("co Full Spring.upgrading").exists(),
        "the staging folder is gone: {}",
        run.stdout()
    );
    assert!(
        run.stdout().contains("staging_folder_removed")
            && run.stdout().contains("co Full Spring.upgrading"),
        "{}",
        run.stdout()
    );

    // `upgrade-models` clears its staging folder the same way.
    fs::create_dir_all(sandbox.path("new models/XXX15 - A")).unwrap();
    fs::write(sandbox.path("new models/XXX15 - A/face.fmdl"), b"face").unwrap();
    fs::create_dir_all(sandbox.path("new models upgraded.upgrading")).unwrap();
    fs::write(
        sandbox.path("new models upgraded.upgrading/left.txt"),
        b"left behind",
    )
    .unwrap();
    let input = sandbox.arg("new models");
    let run = sandbox.run(&["upgrade-models", &input]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        sandbox.path("new models upgraded").is_dir(),
        "{}",
        run.stdout()
    );
    assert!(
        !sandbox.path("new models upgraded.upgrading").exists(),
        "the staging folder is gone: {}",
        run.stdout()
    );
    assert!(
        run.stdout().contains("staging_folder_removed")
            && run.stdout().contains("new models upgraded.upgrading"),
        "{}",
        run.stdout()
    );
}

#[cfg(windows)]
// EU-CLI-10
#[test]
fn a_staging_folder_whose_removal_fails_aborts_the_run() {
    use std::os::windows::fs::OpenOptionsExt;

    // A file of the leftover staging folder held open without share-delete,
    // as `staging.rs`'s `held_without_delete` opens it: Windows refuses the
    // removal while the handle lives.
    const FILE_SHARE_READ: u32 = 1;

    let sandbox = Sandbox::new("cli_staging_held");
    sandbox.copy_fixture_dir("co Spring", "co Spring");
    fs::create_dir_all(sandbox.path("co Full Spring.upgrading")).unwrap();
    let held = sandbox.path("co Full Spring.upgrading/held.txt");
    fs::write(&held, b"held").unwrap();
    let _lock = fs::File::options()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&held)
        .unwrap();
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", &["--savefile", &save]);
    run.assert_refused(3, &["co Full Spring.upgrading", "cannot remove"]);
    assert!(
        sandbox.path("co Full Spring.upgrading").exists(),
        "the staging folder stays"
    );
    assert!(
        !sandbox.path("co Full Spring").exists(),
        "nothing is written"
    );
}

// EU-CLI-07
#[test]
fn a_corrupt_archive_is_export_extract_failed() {
    let sandbox = Sandbox::new("cli_corrupt_zip");
    fs::write(sandbox.path("broken.zip"), b"not a zip at all").unwrap();
    sandbox.save("EDIT");
    let save = sandbox.arg("EDIT");
    let before = sandbox.listing("");

    let run = upgrade(&sandbox, "broken.zip", &["--savefile", &save]);
    assert_eq!(run.exit_code(), 1);
    assert!(
        run.stdout().contains("Error export_extract_failed"),
        "{}",
        run.stdout()
    );
    assert!(run.stdout().contains("broken.zip"), "{}", run.stdout());
    assert_eq!(sandbox.listing(""), before, "nothing is written");
}
