//! `compile`'s deployment into the PES folder's `download/`
//! (`spec/team_compiler/pipeline.md` "Deploy CPKs"): the checks made before any export is
//! read, the copy and rename over the old CPK, and the degraded run that leaves the CPK in the
//! output folder instead.

use std::fs;
use std::path::{Path, PathBuf};

use crate::bins::install_names;
use crate::common::{Run, Sandbox};
use crate::compile::{cpk_entries, pes21_settings, tracer_export};
use crate::{bundled_bins_then, patch_written, savefile_missing, snapshot};

/// The findings of a PES 21 `compile` of the tracer with bundled bins, after the bins' own.
const TRACER_FINDINGS: [&str; 4] = [
    "egg Midcup Tracer: Info fmdl_weights_not_normalized [Keep] at Players/05 - The Chad Stormworks Player (file=boots.fmdl, count=1662)",
    "egg Midcup Tracer: Info fmdl_weights_not_normalized [Keep] at Players/05 - The Chad Stormworks Player (file=fcl_hair.fmdl, count=1662)",
    "egg Midcup Tracer: Info fmdl_weights_not_normalized [Keep] at Players/05 - The Chad Stormworks Player (file=glove_l.fmdl, count=2)",
    "egg Midcup Tracer: Info export_identified [Keep] (team=/egg/, id=792)",
];

/// The line every finding about the installed list ends with, naming the subcommand that
/// installs the official one.
const UPGRADE: &str = concat!(
    "command=",
    studio_core::exe_name!(),
    " team-compiler upgrade-dpfl"
);

/// The repository's `resources/templates/` folder, which holds the official list.
pub(crate) fn templates_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resources/templates")
}

/// The official `DpFileList.bin`'s entries, in load order: the lines of
/// `resources/templates/DpFileList.txt`, which the list is written from, without its comments
/// and blank lines.
pub(crate) fn official_names() -> Vec<String> {
    fs::read_to_string(templates_folder().join("DpFileList.txt"))
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// Writes `PES/download/DpFileList.bin` listing `names`, in that order.
fn install_list(sandbox: &Sandbox, names: &[String]) {
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    install_names(sandbox, &names);
}

/// Writes the sandbox's `download/` folder as `upgrade-dpfl` leaves it: the official
/// `DpFileList.bin` and, for every CPK it lists but the run's `4cc_99_test.cpk`, the
/// placeholder CPK. A compile then reports nothing about the list, and its bins are still the
/// bundled ones, since the placeholders hold none.
fn install_official_list(sandbox: &Sandbox) {
    let templates = templates_folder();
    sandbox.write(
        "PES/download/DpFileList.bin",
        &fs::read(templates.join("DpFileList.bin")).unwrap(),
    );
    let placeholder = fs::read(templates.join("placeholder.cpk")).unwrap();
    for name in official_names() {
        if name != "4cc_99_test.cpk" {
            sandbox.write(&format!("PES/download/{name}"), &placeholder);
        }
    }
}

/// Writes the sandbox's PES 21 install: `PES/PES2021.exe` and an upgraded `download/` folder
/// (`install_official_list`).
pub(crate) fn install_pes(sandbox: &Sandbox) {
    sandbox.write("PES/PES2021.exe", b"the game");
    install_official_list(sandbox);
}

/// A deploying `compile` of the tracer in `sandbox`.
fn compile_tracer(sandbox: &Sandbox) -> Run {
    sandbox.run(
        &pes21_settings(sandbox),
        &["compile", "--export", &tracer_export()],
    )
}

/// The CPK a `--no-deploy` compile of the tracer leaves in the output folder of a sandbox of
/// its own, `name`.
fn twin_cpk(name: &str) -> PathBuf {
    let twin = Sandbox::new(name);
    let run = twin.run(
        &pes21_settings(&twin),
        &["compile", "--no-deploy", "--export", &tracer_export()],
    );
    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    twin.root.join("output/4cc_99_test.cpk")
}

/// The path, as the findings print it, the CPK of a degraded run is promoted to.
fn promoted(sandbox: &Sandbox) -> String {
    sandbox.display("output/4cc_99_test.cpk")
}

/// The names of the files in the folder `relative` that end with `.partial`: what a failed
/// deployment would leave.
fn partial_files(sandbox: &Sandbox, relative: &str) -> Vec<String> {
    fs::read_dir(sandbox.root.join(relative))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".partial"))
        .collect()
}

/// Asserts the CPK installed in `sandbox`'s `download/` is the `twin` CPK, byte for byte.
fn assert_installed(sandbox: &Sandbox, twin: &Path) {
    let installed = fs::read(sandbox.root.join("PES/download/4cc_99_test.cpk")).unwrap();
    assert!(
        installed == fs::read(twin).unwrap(),
        "download/ holds the run's CPK"
    );
}

/// Asserts the run's CPK is in the output folder: the bytes of the `twin` CPK.
fn assert_promoted(sandbox: &Sandbox, twin: &Path) {
    let promoted = fs::read(sandbox.root.join("output/4cc_99_test.cpk")).unwrap();
    assert!(
        promoted == fs::read(twin).unwrap(),
        "the output folder holds the run's CPK"
    );
}

// TC-DEP-01
#[test]
fn compile_installs_the_cpk_over_the_old_one_and_leaves_nothing_in_the_output_folder() {
    let twin = twin_cpk("deploy_installs_twin");
    let sandbox = Sandbox::new("deploy_installs");
    install_pes(&sandbox);
    sandbox.write("PES/download/4cc_99_test.cpk", b"the old CPK");
    let download_before = snapshot(&sandbox.root.join("PES/download"));

    let run = compile_tracer(&sandbox);

    let mut expected = bundled_bins_then(TRACER_FINDINGS);
    expected.push(patch_written(&sandbox, "792"));
    expected.push(savefile_missing(&sandbox));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 0);
    // Only the CPK changed in download/: the installed list already names it.
    let mut expected_download = download_before;
    expected_download.insert(PathBuf::from("4cc_99_test.cpk"), fs::read(&twin).unwrap());
    let download_after = snapshot(&sandbox.root.join("PES/download"));
    // Compared whole, not with `assert_eq!`, which would print every file's bytes.
    assert!(
        download_after == expected_download,
        "download/ holds what it held, with the run's CPK over the old one: {:?}",
        download_after.keys().collect::<Vec<_>>()
    );
    let installed = sandbox.root.join("PES/download/4cc_99_test.cpk");
    assert_eq!(cpk_entries(&installed), cpk_entries(&twin));
    // Byte for byte too: the writer's output depends on the entries alone.
    assert!(fs::read(&installed).unwrap() == fs::read(&twin).unwrap());
    assert_eq!(
        partial_files(&sandbox, "PES/download"),
        Vec::<String>::new()
    );
    assert_eq!(
        snapshot(&sandbox.root.join("output"))
            .keys()
            .collect::<Vec<_>>(),
        [&PathBuf::from("aesthetics_patch.toml")],
        "the patch, and no CPK and no .staging, in the output folder"
    );
}

#[cfg(windows)]
// TC-DEP-02
#[test]
fn an_old_cpk_held_open_is_old_cpk_locked_and_the_cpk_is_promoted() {
    // Windows only: Linux renames over a file another process holds open.
    use std::os::windows::fs::OpenOptionsExt;
    /// `FILE_SHARE_READ` alone, as PES holds its CPKs: read, not deleted or replaced.
    const FILE_SHARE_READ: u32 = 1;

    let twin = twin_cpk("deploy_locked_twin");
    let sandbox = Sandbox::new("deploy_locked");
    install_pes(&sandbox);
    sandbox.write("PES/download/4cc_99_test.cpk", b"the old CPK");
    let held_open = fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(sandbox.root.join("PES/download/4cc_99_test.cpk"))
        .unwrap();

    let run = compile_tracer(&sandbox);

    drop(held_open);
    let lines = run.messages();
    // `patch_written` follows the degraded deployment's promote, the `old_cpk_locked`
    // finding before it.
    let (patch, first) = lines.split_last().unwrap();
    assert_eq!(*patch, patch_written(&sandbox, "792"));
    let (last, first) = first.split_last().unwrap();
    assert_eq!(first, bundled_bins_then(TRACER_FINDINGS));
    // The error ends with the platform's own text, so only its shape is fixed.
    let prefix = format!(
        "Error old_cpk_locked [Keep] (path={}, error=",
        sandbox.display("PES/download/4cc_99_test.cpk")
    );
    let suffix = format!(", output={})", promoted(&sandbox));
    assert!(
        last.starts_with(&prefix) && last.ends_with(&suffix),
        "{last}"
    );
    assert_eq!(run.exit_code(), 1);
    assert_promoted(&sandbox, &twin);
    assert_eq!(
        fs::read(sandbox.root.join("PES/download/4cc_99_test.cpk")).unwrap(),
        b"the old CPK"
    );
    assert_eq!(
        partial_files(&sandbox, "PES/download"),
        Vec::<String>::new(),
        "no .partial is left"
    );
}

// TC-DEP-03
#[test]
fn without_a_pes_folder_the_run_is_pes_folder_not_found_and_the_cpk_is_promoted() {
    let twin = twin_cpk("deploy_no_pes_twin");
    let sandbox = Sandbox::new("deploy_no_pes");

    let run = compile_tracer(&sandbox);

    let mut expected = vec![format!(
        "Error pes_folder_not_found [Keep] (path={}, output={})",
        sandbox.display("PES"),
        promoted(&sandbox)
    )];
    // Every bin is the bundled one: there is no list to walk.
    expected.extend(bundled_bins_then(TRACER_FINDINGS));
    expected.push(patch_written(&sandbox, "792"));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 1);
    assert_promoted(&sandbox, &twin);
    assert!(!sandbox.root.join("PES").exists());
}

#[cfg(windows)]
// TC-DEP-04
#[test]
fn a_download_folder_that_denies_writes_is_reported_before_any_export_is_read() {
    // Windows only: the remote mutation runs build as root on Linux, and root ignores
    // permissions, so a read-only file would not deny the write there.
    let twin = twin_cpk("deploy_unwritable_twin");
    let sandbox = Sandbox::new("deploy_unwritable");
    install_pes(&sandbox);
    sandbox.write("PES/download/4cc_99_test.cpk", b"the old CPK");
    let old = sandbox.root.join("PES/download/4cc_99_test.cpk");
    let mut permissions = fs::metadata(&old).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&old, permissions).unwrap();

    let run = compile_tracer(&sandbox);

    let lines = run.messages();
    let prefix = format!(
        "Error deploy_target_unwritable [Keep] (path={}, error=",
        sandbox.display("PES/download/4cc_99_test.cpk")
    );
    let suffix = format!(", output={})", promoted(&sandbox));
    let unwritable = lines
        .iter()
        .position(|line| line.starts_with(&prefix) && line.ends_with(&suffix))
        .unwrap_or_else(|| panic!("no deploy_target_unwritable: {lines:#?}"));
    let first_export = lines
        .iter()
        .position(|line| line.starts_with("egg Midcup Tracer: "))
        .unwrap();
    assert!(unwritable < first_export, "{lines:#?}");
    assert_eq!(run.exit_code(), 1);
    assert_promoted(&sandbox, &twin);
    assert_eq!(fs::read(&old).unwrap(), b"the old CPK");
}

// TC-DEP-05
#[test]
fn a_list_not_naming_the_cpk_is_cpk_name_unlisted_and_the_cpk_is_promoted() {
    let twin = twin_cpk("deploy_unlisted_twin");
    let sandbox = Sandbox::new("deploy_unlisted");
    install_pes(&sandbox);
    let download = snapshot(&sandbox.root.join("PES/download"));

    // A name neither the installed list nor the official one holds.
    let run = sandbox.run(
        &format!(
            "{}[team-compiler]\ncpk_name = \"4cc_80_mine\"\n",
            pes21_settings(&sandbox)
        ),
        &["compile", "--export", &tracer_export()],
    );

    let mut expected = vec![format!(
        "Error cpk_name_unlisted [Keep] (cpk=4cc_80_mine.cpk, path={}, output={})",
        sandbox.display("PES/download/DpFileList.bin"),
        sandbox.display("output/4cc_80_mine.cpk")
    )];
    expected.extend(bundled_bins_then(TRACER_FINDINGS));
    expected.push(patch_written(&sandbox, "792"));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 1);
    // The CPK's bytes do not depend on its name.
    let promoted = fs::read(sandbox.root.join("output/4cc_80_mine.cpk")).unwrap();
    assert!(
        promoted == fs::read(&twin).unwrap(),
        "the output folder holds the run's CPK"
    );
    assert_eq!(snapshot(&sandbox.root.join("PES/download")), download);
}

// TC-DEP-06
#[test]
fn another_version_s_exe_is_a_warning_and_the_cpk_is_installed() {
    let sandbox = Sandbox::new("deploy_version_mismatch");
    install_official_list(&sandbox);
    sandbox.write("PES/PES2019.exe", b"another game");

    let run = compile_tracer(&sandbox);

    let mut expected = vec![format!(
        "Warning pes_version_mismatch [Keep] (path={}, exe=PES2021.exe)",
        sandbox.display("PES")
    )];
    expected.extend(bundled_bins_then(TRACER_FINDINGS));
    expected.push(patch_written(&sandbox, "792"));
    expected.push(savefile_missing(&sandbox));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 0);
    assert!(sandbox.root.join("PES/download/4cc_99_test.cpk").is_file());
    assert!(!sandbox.root.join("output/4cc_99_test.cpk").exists());
}

// TC-DEP-07
#[test]
fn a_dead_run_s_staging_folder_is_removed_when_compile_starts() {
    let sandbox = Sandbox::new("deploy_dead_staging");
    install_pes(&sandbox);
    // A killed run's folder: no lock file says it is alive.
    sandbox.write("output/.staging/1-1/x.cpk", b"half a CPK");

    let run = compile_tracer(&sandbox);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    assert!(!sandbox.root.join("output/.staging").exists());
    assert!(sandbox.root.join("PES/download/4cc_99_test.cpk").is_file());
}

// TC-DEP-12
#[test]
fn a_list_naming_a_cpk_the_official_one_lacks_is_dpfilelist_not_official_and_the_cpk_is_installed()
{
    let twin = twin_cpk("deploy_not_official_twin");
    let sandbox = Sandbox::new("deploy_not_official");
    install_pes(&sandbox);
    let mut names = official_names();
    names.push("4cc_80_mine.cpk".to_owned());
    install_list(&sandbox, &names);
    sandbox.write("PES/download/4cc_80_mine.cpk", b"mine");

    let run = compile_tracer(&sandbox);

    // No `dpfilelist_cpk_missing`: every listed CPK but the run's own has its file.
    let mut expected = vec![format!(
        "Warning dpfilelist_not_official [Keep] (path={}, unofficial=4cc_80_mine.cpk, {UPGRADE})",
        sandbox.display("PES/download/DpFileList.bin")
    )];
    expected.extend(bundled_bins_then(TRACER_FINDINGS));
    expected.push(patch_written(&sandbox, "792"));
    expected.push(savefile_missing(&sandbox));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 0);
    assert_installed(&sandbox, &twin);
}

// TC-DEP-12
#[test]
fn the_official_list_with_every_listed_cpk_is_no_list_finding_and_the_cpk_is_installed() {
    let twin = twin_cpk("deploy_official_twin");
    let sandbox = Sandbox::new("deploy_official");
    install_pes(&sandbox);

    let run = compile_tracer(&sandbox);

    let lines = run.messages();
    for code in [
        "dpfilelist_not_official",
        "dpfilelist_outdated",
        "dpfilelist_cpk_missing",
    ] {
        assert!(
            lines.iter().all(|line| !line.contains(code)),
            "{code}: {lines:#?}"
        );
    }
    assert_eq!(run.exit_code(), 0);
    assert_installed(&sandbox, &twin);
}

// TC-DEP-14
#[test]
fn a_listed_cpk_missing_from_download_is_dpfilelist_cpk_missing_and_the_cpk_is_installed() {
    let twin = twin_cpk("deploy_cpk_missing_twin");
    let sandbox = Sandbox::new("deploy_cpk_missing");
    install_pes(&sandbox);
    fs::remove_file(sandbox.root.join("PES/download/4cc_62_midcup.cpk")).unwrap();

    let run = compile_tracer(&sandbox);

    // Not `4cc_99_test.cpk`, absent too before the run: the run writes it.
    let mut expected = vec![format!(
        "Warning dpfilelist_cpk_missing [Keep] (path={}, files=4cc_62_midcup.cpk, {UPGRADE})",
        sandbox.display("PES/download")
    )];
    expected.extend(bundled_bins_then(TRACER_FINDINGS));
    expected.push(patch_written(&sandbox, "792"));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 0);
    assert_installed(&sandbox, &twin);
}

// TC-DEP-08
#[test]
fn a_list_lacking_the_cpk_the_official_one_names_is_dpfilelist_outdated_and_the_cpk_is_promoted() {
    let twin = twin_cpk("deploy_outdated_twin");
    let sandbox = Sandbox::new("deploy_outdated");
    install_pes(&sandbox);
    let names: Vec<String> = official_names()
        .into_iter()
        .filter(|name| name != "4cc_99_test.cpk")
        .collect();
    assert_eq!(names.len(), 52);
    install_list(&sandbox, &names);

    let run = compile_tracer(&sandbox);

    // No `dpfilelist_not_official`: the Error ends the checks.
    let mut expected = vec![format!(
        "Error dpfilelist_outdated [Keep] (cpk=4cc_99_test.cpk, path={}, output={}, {UPGRADE})",
        sandbox.display("PES/download/DpFileList.bin"),
        promoted(&sandbox)
    )];
    expected.extend(bundled_bins_then(TRACER_FINDINGS));
    expected.push(patch_written(&sandbox, "792"));
    assert_eq!(run.messages(), expected);
    assert_eq!(run.exit_code(), 1);
    assert_promoted(&sandbox, &twin);
    assert!(!sandbox.root.join("PES/download/4cc_99_test.cpk").exists());
}
