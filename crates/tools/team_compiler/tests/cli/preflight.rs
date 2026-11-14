//! The preflight: the invocations and settings refused before any export is read.

use crate::common::Sandbox;
use crate::{bundled_bins_then, snapshot};

// TC-CLI-03
#[test]
fn no_deploy_with_test_mode_is_an_invalid_invocation() {
    let sandbox = Sandbox::new("no_deploy_test_mode");
    let run = sandbox.run("", &["compile", "--no-deploy", "--mode", "test"]);
    run.assert_refused(2, &["--no-deploy", "--mode test", "incompatible"]);
    assert!(!sandbox.root.join("output").exists());
}

// TC-CLI-05
#[test]
fn an_invalid_cpk_name_refuses_compile_before_any_export_is_read() {
    for (index, name) in ["", "con", "a/b"].into_iter().enumerate() {
        let sandbox = Sandbox::new(&format!("cpk_name_{index}"));
        sandbox.write("exports/aaa_export/notes.txt", b"kept as it is");
        let before = snapshot(&sandbox.root);

        // This exports root does not exist: reaching it would refuse naming it instead.
        let run = sandbox.run(
            &format!("[compiler]\ncpk_name = \"{name}\"\n"),
            &["compile", "--no-deploy", &sandbox.arg("no such exports")],
        );

        run.assert_refused(2, &[&format!("cpk_name = \"{name}\"")]);
        assert_eq!(snapshot(&sandbox.root), before);
    }
}

// TC-CLI-07
#[test]
fn an_export_path_that_is_missing_or_no_archive_is_an_invalid_invocation() {
    let sandbox = Sandbox::new("export_paths");
    sandbox.write("notes.txt", b"not an export");
    for command in ["check", "compile"] {
        let missing = sandbox.arg("no_such_export");
        let run = sandbox.run("", &[command, "--export", &missing]);
        run.assert_refused(2, &[&missing, "does not exist"]);
        let not_archive = sandbox.arg("notes.txt");
        let run = sandbox.run("", &[command, "--export", &not_archive]);
        run.assert_refused(2, &[&not_archive, "is not a folder, a .zip or a .7z file"]);
    }
}

// TC-CLI-08
#[test]
fn an_absolute_exports_folder_setting_naming_a_missing_folder_is_refused_in_plain_words() {
    let sandbox = Sandbox::new("absolute_exports_missing");
    let missing = sandbox.root.join("elsewhere").join("exports");
    let settings = format!("[common]\nexports_folder_path = '{}'\n", missing.display());
    let sentence = format!(
        "the exports folder {} does not exist. Create it and put your exports inside, or set \
         exports_folder_path in {} to the folder that holds them",
        missing.display(),
        sandbox.root.join("data").join("settings.toml").display()
    );
    for command in ["check", "compile"] {
        let run = sandbox.run(&settings, &[command]);
        run.assert_refused(2, &[&sentence]);
        let Err(error) = &run.result else {
            unreachable!("assert_refused checked the refusal")
        };
        assert!(!error.to_string().contains("os error"), "{error}");
        assert!(!missing.exists(), "{command} created the folder");
        assert!(!sandbox.root.join("output").exists(), "{command} wrote");
    }
}

// TC-CLI-09
#[test]
fn a_missing_default_exports_folder_is_created_and_reported_empty() {
    let sandbox = Sandbox::new("default_exports_missing");
    let exports = sandbox.root.join("exports");
    assert!(!exports.exists());

    let run = sandbox.run("", &["compile", "--no-deploy"]);

    assert!(exports.is_dir(), "the folder was created");
    // The default settings' PES folder does not exist, and their PES 19 has the three bins.
    assert_eq!(
        run.messages(),
        bundled_bins_then([format!(
            "Warning no_exports_found [Keep] (folder={})",
            exports.display()
        )])
    );
    assert!(!sandbox.root.join("output/4cc_99_test.cpk").exists());
    assert_eq!(run.exit_code(), 0);
}

// TC-CLI-05
#[test]
fn the_settings_table_is_compiler_and_a_team_compiler_table_is_unread() {
    let sandbox = Sandbox::new("settings_table");
    // `[team-compiler]` is a table no tool reads, as any unknown tool's is: the invalid
    // `cpk_name` under it is not seen, and the run proceeds to find no exports.
    let run = sandbox.run(
        "[team-compiler]\ncpk_name = \"con\"\n",
        &["compile", "--no-deploy"],
    );
    assert_eq!(run.exit_code(), 0, "{:?}", run.messages());
    let exports = sandbox.root.join("exports");
    assert_eq!(
        run.messages(),
        bundled_bins_then([format!(
            "Warning no_exports_found [Keep] (folder={})",
            exports.display()
        )])
    );

    // The same value under `[compiler]`, the Compiler's table, refuses the run.
    let run = sandbox.run(
        "[compiler]\ncpk_name = \"con\"\n",
        &["compile", "--no-deploy"],
    );
    run.assert_refused(2, &["cpk_name = \"con\""]);
}

// TC-CLI-05
#[test]
fn a_setting_of_the_wrong_type_is_a_configuration_error_naming_it() {
    let sandbox = Sandbox::new("wrong_type");
    let run = sandbox.run("[compiler]\nstrict_file_type_check = \"yes\"\n", &["check"]);
    run.assert_refused(2, &["the [compiler] settings", "strict_file_type_check"]);
    let Err(error) = &run.result else {
        unreachable!("assert_refused checked the refusal")
    };
    let text = error.to_string();
    assert_eq!(text.trim_end(), text, "no trailing newline");
}

// TC-CLI-05
#[test]
fn a_memory_cap_of_zero_is_a_configuration_error() {
    let sandbox = Sandbox::new("memory_cap");
    let run = sandbox.run("[common]\nmemory_cap_percent = 0.0\n", &["check"]);
    run.assert_refused(2, &["memory_cap_percent"]);
}

// TC-CLI-12
#[test]
fn an_unparsable_teams_list_aborts_naming_it() {
    let sandbox = Sandbox::new("teams_list");
    sandbox.write("data/teams_list.txt", b"not a teams list\n");
    let run = sandbox.run("", &["check"]);
    run.assert_refused(3, &["teams_list.txt"]);
}
