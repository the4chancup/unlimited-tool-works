//! The command line's own rules (the `spec/export_upgrader.md` "CLI" scenarios): the output
//! name's tag, `-o`, the flag and path refusals, and the exit codes.

use std::fs;

use pes_version::PesVersion;
use studio_core::PipelineEvent;

use crate::common::{Run, Sandbox, snapshot};
use crate::players::face;

/// `upgrade <input> <flags>`.
fn upgrade(sandbox: &Sandbox, input: &str, flags: &[&str]) -> Run {
    upgrade_several(sandbox, &[input], flags)
}

/// `upgrade <inputs>... <flags>`.
fn upgrade_several(sandbox: &Sandbox, inputs: &[&str], flags: &[&str]) -> Run {
    let inputs: Vec<String> = inputs.iter().map(|input| sandbox.arg(input)).collect();
    let mut args = vec!["upgrade"];
    args.extend(inputs.iter().map(String::as_str));
    args.extend(flags);
    sandbox.run(&args)
}

/// The text of every `Output` event the run emitted, in order: the naming
/// lines, the empty lines between blocks, the `error:` lines and the tally.
fn output_lines(run: &Run) -> Vec<&str> {
    run.events
        .iter()
        .filter_map(|envelope| match &envelope.event {
            PipelineEvent::Output(text) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// A refusal of the whole run: exit 2 naming every one of `names`, before
/// any export is read, so no event at all.
fn assert_whole_run_refused(run: &Run, names: &[&str]) {
    run.assert_refused(2, names);
    assert!(
        run.events.is_empty(),
        "a refused run emits no event: {:?}",
        run.events
    );
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
    // A savefile that does not hold the resolved team: the PES 18 fixture save
    // has no /trg/ (822).
    old_export(&sandbox, "trg Spring", Some("/trg/"));
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
    upgrade(&sandbox, "trg Spring", &["--savefile", &save18]).assert_refused(2, &["/trg/"]);
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

/// Old exports at each of `names`, each with a face folder so that its run is
/// clean, the Note naming `team`.
fn clean_old_exports(sandbox: &Sandbox, names: &[&str], team: &str) {
    for name in names {
        old_export(sandbox, name, Some(team));
        face(sandbox, &format!("{name}/Faces/XXX05 - A"));
    }
}

// EU-CLI-11
#[test]
fn several_exports_are_each_upgraded_as_if_alone() {
    let sandbox = Sandbox::new("cli_several");
    clean_old_exports(&sandbox, &["co - Spring", "co - Autumn"], "/co/");
    // The PES 21 fixture save has no /trg/ (822).
    clean_old_exports(&sandbox, &["trg Spring"], "/trg/");
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");

    let run = upgrade_several(
        &sandbox,
        &["co - Spring", "co - Autumn", "trg Spring"],
        &["--savefile", &save],
    );
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 2, "{stdout}");
    assert!(sandbox.path("co Full - Spring").is_dir(), "{stdout}");
    assert!(sandbox.path("co Full - Autumn").is_dir(), "{stdout}");
    sandbox.assert_nothing_written("trg Full Spring", &stdout);
    let refused = format!("error: --savefile {save} does not hold the export's team /trg/");
    assert_eq!(
        output_lines(&run),
        [
            "Upgrading co - Spring",
            "",
            "Upgrading co - Autumn",
            "",
            "Upgrading trg Spring",
            &refused,
            "",
            "Upgraded: 2. Not upgraded: 1.",
        ]
    );
    // The empty lines are the Output ones alone (no finding prints an empty
    // line), so they cut the printed report into the three blocks and the
    // tally, each block's findings under its own naming line.
    let blocks: Vec<&str> = stdout.split("\n\n").collect();
    assert_eq!(blocks.len(), 4, "{stdout}");
    for (block, name) in blocks.iter().zip(["co - Spring", "co - Autumn"]) {
        assert!(
            block.starts_with(&format!("Upgrading {name}\n")),
            "{stdout}"
        );
        assert!(
            block.contains(&format!("- {name}: Info coverage_tag_chosen")),
            "{stdout}"
        );
        assert!(block.contains("- Info migration_clean"), "{stdout}");
    }
    assert!(
        blocks[2].starts_with("Upgrading trg Spring\n") && blocks[2].ends_with(&refused),
        "{stdout}"
    );
    assert_eq!(blocks[3], "Upgraded: 2. Not upgraded: 1.\n");

    // What the progress line counts from: the total announced before anything
    // else, each export a run of its own, and every export done once, the
    // refused one included.
    assert_eq!(
        run.events.first().map(|envelope| &envelope.event),
        Some(&PipelineEvent::ExportsExpected { total: 3 })
    );
    let done: Vec<u64> = run
        .events
        .iter()
        .filter(|envelope| matches!(envelope.event, PipelineEvent::ExportProcessed { .. }))
        .map(|envelope| envelope.run_id.0)
        .collect();
    assert_eq!(done, [1, 2, 3]);

    // One export named alone prints as it always did: no naming line, no
    // tally.
    let out = sandbox.arg("alone");
    let run = upgrade(&sandbox, "co - Spring", &["--savefile", &save, "-o", &out]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(output_lines(&run).is_empty(), "{}", run.stdout());

    // An unknown flag refuses the run before any export is read.
    let before = sandbox.listing("");
    let run = upgrade_several(
        &sandbox,
        &["co - Spring", "co - Autumn"],
        &["--savefile", &save, "--bogus"],
    );
    assert_whole_run_refused(&run, &["--bogus"]);
    assert_eq!(sandbox.listing(""), before, "nothing is written");
}

// EU-CLI-11
#[test]
fn a_missing_export_between_two_is_refused_alone() {
    let sandbox = Sandbox::new("cli_several_missing");
    clean_old_exports(&sandbox, &["co - Spring", "co - Autumn"], "/co/");
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");

    let run = upgrade_several(
        &sandbox,
        &["co - Spring", "co - Winter", "co - Autumn"],
        &["--savefile", &save],
    );
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 2, "{stdout}");
    assert!(sandbox.path("co Full - Spring").is_dir(), "{stdout}");
    assert!(sandbox.path("co Full - Autumn").is_dir(), "{stdout}");
    let refused = format!("error: {} does not exist", sandbox.arg("co - Winter"));
    assert_eq!(
        output_lines(&run),
        [
            "Upgrading co - Spring",
            "",
            "Upgrading co - Winter",
            &refused,
            "",
            "Upgrading co - Autumn",
            "",
            "Upgraded: 2. Not upgraded: 1.",
        ]
    );
}

// EU-CLI-11
#[test]
fn the_exit_code_is_the_worst_exports_and_only_a_clean_one_counts_as_upgraded() {
    let sandbox = Sandbox::new("cli_several_unclean");
    clean_old_exports(&sandbox, &["co - Spring"], "/co/");
    crate::carried::two_portraits_of_one_slot(&sandbox, "co Kits");
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");

    // A clean export (0), one with an Error (1) and a missing one (2).
    let run = upgrade_several(
        &sandbox,
        &["co - Spring", "co Kits", "gone"],
        &["--savefile", &save],
    );
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 2, "{stdout}");
    assert!(
        stdout.contains("- co Kits: Error unmigrated_content"),
        "{stdout}"
    );
    assert_eq!(
        output_lines(&run).last(),
        Some(&"Upgraded: 1. Not upgraded: 2."),
        "{stdout}"
    );
}

// EU-CLI-13
#[test]
fn all_upgrades_every_export_in_the_folder_in_name_order() {
    let sandbox = Sandbox::new("cli_all");
    // `CO Winter` sorts last without regard to case and first by its bytes, since an
    // uppercase letter's byte is below every lowercase one's.
    clean_old_exports(
        &sandbox,
        &["library/co - Spring", "library/CO Winter"],
        "/co/",
    );
    sandbox.copy_fixture("co Spring.zip", "library/co Spring.zip");
    sandbox.write("library/notes.txt", b"not an export");
    sandbox.copy_fixture("co Spring.zip", "one/co Spring.zip");
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");
    let (library, one, out) = (
        sandbox.arg("library"),
        sandbox.arg("one"),
        sandbox.arg("out"),
    );

    let run = sandbox.run(&[
        "upgrade",
        "--all",
        &library,
        "--savefile",
        &save,
        "-o",
        &out,
    ]);
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 0, "{stdout}");
    assert!(sandbox.path("out/co Full - Spring").is_dir(), "{stdout}");
    assert!(sandbox.path("out/co Full Spring").is_dir(), "{stdout}");
    assert_eq!(
        output_lines(&run),
        [
            "Upgrading co - Spring",
            "",
            "Upgrading co Spring.zip",
            "",
            "Upgrading CO Winter",
            "",
            "Upgraded: 3. Not upgraded: 0.",
        ]
    );
    // The one line naming a notes.txt is about the zip's own output, the
    // notes its Note carried; none is about the stray file.
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.contains("notes.txt"))
            .collect::<Vec<_>>(),
        ["- co Full Spring: Info notes_found at notes.txt"],
        "{stdout}"
    );

    // A folder holding one export still names it and tallies it.
    let out = sandbox.arg("out one");
    let run = sandbox.run(&["upgrade", "--all", &one, "--savefile", &save, "-o", &out]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        output_lines(&run),
        [
            "Upgrading co Spring.zip",
            "",
            "Upgraded: 1. Not upgraded: 0.",
        ]
    );

    // An export named and also in the `--all` folder: the named one runs first, and the
    // second turn is refused because the first wrote its output folder.
    let (zip, out) = (
        sandbox.arg("library/co Spring.zip"),
        sandbox.arg("out both"),
    );
    let run = sandbox.run(&[
        "upgrade",
        &zip,
        "--all",
        &library,
        "--savefile",
        &save,
        "-o",
        &out,
    ]);
    assert_eq!(run.exit_code(), 3, "{}", run.stdout());
    let lines = output_lines(&run);
    let refused = lines.get(5).copied().unwrap_or_default();
    assert!(
        refused.starts_with("error: ") && refused.contains("already exists"),
        "{lines:?}"
    );
    assert_eq!(
        lines,
        [
            "Upgrading co Spring.zip",
            "",
            "Upgrading co - Spring",
            "",
            "Upgrading co Spring.zip",
            refused,
            "",
            "Upgrading CO Winter",
            "",
            "Upgraded: 3. Not upgraded: 1.",
        ]
    );
}

// EU-CLI-13
#[test]
fn a_whole_run_refusal_reads_no_export() {
    let sandbox = Sandbox::new("cli_whole_run_refusals");
    clean_old_exports(&sandbox, &["co - Spring", "co - Autumn"], "/co/");
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");
    fs::write(sandbox.path("not-a-save.bin"), b"no save").unwrap();
    let not_save = sandbox.arg("not-a-save.bin");
    fs::create_dir_all(sandbox.path("empty")).unwrap();
    let (missing, empty) = (sandbox.arg("missing"), sandbox.arg("empty"));
    let before = sandbox.listing("");

    let run = upgrade_several(
        &sandbox,
        &["co - Spring", "co - Autumn"],
        &["--savefile", &not_save],
    );
    assert_whole_run_refused(&run, &["not-a-save.bin"]);
    let run = sandbox.run(&["upgrade", "--all", &missing, "--savefile", &save]);
    assert_whole_run_refused(&run, &[&missing, "does not exist"]);
    let run = sandbox.run(&["upgrade", "--all", &empty, "--savefile", &save]);
    assert_whole_run_refused(
        &run,
        &[&empty, "holds no export (a folder, a .zip or a .7z file)"],
    );
    let run = sandbox.run(&["upgrade", "--all", &not_save, "--savefile", &save]);
    assert_whole_run_refused(&run, &[&not_save, "is not a folder"]);
    // `--savefile` with `--loose` is one argument error, the same for a run
    // of one export (clap's own refusal) as for several.
    let run = upgrade_several(
        &sandbox,
        &["co - Spring", "co - Autumn"],
        &["--savefile", &save, "--loose"],
    );
    assert_whole_run_refused(&run, &["--savefile", "--loose"]);
    // Neither an export nor `--all`: clap's own refusal names the missing exports.
    let run = sandbox.run(&["upgrade", "--savefile", &save]);
    assert_whole_run_refused(&run, &["required arguments were not provided", "<EXPORTS>"]);
    assert_eq!(sandbox.listing(""), before, "nothing is written");
}

// EU-CLI-13
#[test]
fn all_skips_a_staging_folder_an_interrupted_run_left() {
    let sandbox = Sandbox::new("cli_all_staging");
    clean_old_exports(&sandbox, &["library/co - Spring"], "/co/");
    // The incomplete output folder an interrupted run leaves is no export:
    // the scan passes it over without a line, and the export's own run
    // removes it when it takes the staging name.
    sandbox.write(
        "library/co Full - Spring.upgrading/left.txt",
        b"left behind",
    );
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");
    let library = sandbox.arg("library");

    let run = sandbox.run(&["upgrade", "--all", &library, "--savefile", &save]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        output_lines(&run),
        ["Upgrading co - Spring", "", "Upgraded: 1. Not upgraded: 0.",]
    );
}

// EU-CLI-14
#[test]
fn an_export_that_aborts_does_not_stop_the_run() {
    let sandbox = Sandbox::new("cli_several_abort");
    clean_old_exports(&sandbox, &["co - Spring", "co - Autumn"], "/co/");
    // The first export's output is already there, a member's edit inside.
    sandbox.write("co Full - Spring/keep.txt", b"my edit");
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");

    let run = upgrade_several(
        &sandbox,
        &["co - Spring", "co - Autumn"],
        &["--savefile", &save],
    );
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 3, "{stdout}");
    assert!(sandbox.path("co Full - Autumn").is_dir(), "{stdout}");
    assert_eq!(
        sandbox.listing("co Full - Spring"),
        ["keep.txt"],
        "the existing output is left as it was"
    );
    let lines = output_lines(&run);
    assert!(
        lines[1].starts_with("error: ") && lines[1].contains("already exists"),
        "{lines:?}"
    );
    assert_eq!(lines.last(), Some(&"Upgraded: 1. Not upgraded: 1."));
}

// EU-CLI-11, EU-REF-05
#[test]
fn a_referee_export_among_team_exports_is_refused_its_flag() {
    let sandbox = Sandbox::new("cli_several_refs_save");
    clean_old_exports(&sandbox, &["co - Spring"], "/co/");
    sandbox.write("refs Spring/refs.txt", b"01 arbiter\n");
    sandbox.write("refs Spring/Players/arbiter/boots.fmdl", b"boots");
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");

    let run = upgrade_several(
        &sandbox,
        &["co - Spring", "refs Spring"],
        &["--savefile", &save],
    );
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 2, "{stdout}");
    assert!(sandbox.path("co Full - Spring").is_dir(), "{stdout}");
    sandbox.assert_nothing_written("refs Spring upgraded", &stdout);
    assert_eq!(
        output_lines(&run),
        [
            "Upgrading co - Spring",
            "",
            "Upgrading refs Spring",
            "error: --savefile does not apply to a referee export, which takes none of \
             --savefile, --loose or --coverage",
            "",
            "Upgraded: 1. Not upgraded: 1.",
        ]
    );
}

// EU-CLI-03, EU-CLI-11
#[test]
fn a_team_export_among_referee_exports_needs_its_flag() {
    let sandbox = Sandbox::new("cli_several_refs_none");
    clean_old_exports(&sandbox, &["co - Spring"], "/co/");
    sandbox.write("refs Spring/refs.txt", b"01 arbiter\n");
    sandbox.write("refs Spring/Players/arbiter/boots.fmdl", b"boots");

    let run = upgrade_several(&sandbox, &["co - Spring", "refs Spring"], &[]);
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 2, "{stdout}");
    assert!(sandbox.path("refs Spring upgraded").is_dir(), "{stdout}");
    sandbox.assert_nothing_written("co Full - Spring", &stdout);
    assert_eq!(
        output_lines(&run),
        [
            "Upgrading co - Spring",
            "error: a team export takes exactly one of --savefile or --loose",
            "",
            "Upgrading refs Spring",
            "",
            "Upgraded: 1. Not upgraded: 1.",
        ]
    );
}
