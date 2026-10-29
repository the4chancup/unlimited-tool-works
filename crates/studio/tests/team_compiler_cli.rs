//! The Team compiler's command line, run through the real binary: what only the binary does,
//! which is loading the settings file, printing the console lines and a refusal's `error: ` line,
//! and turning the tool's verdict into the process exit code. The tool's own refusals and
//! findings are tested in-process, in the `team_compiler` crate's `tests/cli/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A fresh folder holding a copy of the binary and a `data/settings.toml`, so the portable data
/// location is found beside the executable and no settings file elsewhere on the machine is read.
struct Sandbox {
    root: PathBuf,
    exe: PathBuf,
}

impl Sandbox {
    fn new(name: &str, settings: &str) -> Sandbox {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("team_compiler_cli")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("data")).unwrap();
        let built = Path::new(env!("CARGO_BIN_EXE_utw"));
        let exe = root.join(built.file_name().unwrap());
        // A hard link, not a copy: a copy holds the file open for writing, and on Linux a
        // child another test thread forks in that instant inherits the handle, so running
        // the copy fails with "Text file busy" (ETXTBSY, seen on the remote gates).
        // `CARGO_TARGET_TMPDIR` is inside `target/`, the binary's own volume.
        fs::hard_link(built, &exe).unwrap();
        fs::write(root.join("data/settings.toml"), settings).unwrap();
        Sandbox { root, exe }
    }

    /// Runs the copied binary with the sandbox as the current directory.
    fn run(&self, args: &[&str]) -> Output {
        self.run_from(&self.root, args)
    }

    /// Runs the copied binary with `current_dir` as the current directory.
    fn run_from(&self, current_dir: &Path, args: &[&str]) -> Output {
        Command::new(&self.exe)
            .args(args)
            .current_dir(current_dir)
            .output()
            .unwrap()
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    /// The settings line naming the sandbox's `PES/` as the game folder: the default is the
    /// real install's path, which no test may reach.
    fn pes_folder_line(&self) -> String {
        format!("pes_folder_path = '{}'\n", self.root.join("PES").display())
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_refused(output: &Output, exit_code: i32, names: &[&str]) {
    let text = stderr(output);
    assert_eq!(output.status.code(), Some(exit_code), "stderr: {text}");
    for name in names {
        assert!(text.contains(name), "stderr does not name {name}: {text}");
    }
}

#[test]
fn a_refusal_prints_one_error_line_and_exits_with_its_code() {
    let sandbox = Sandbox::new("refusal", "");
    let output = sandbox.run(&["team-compiler", "compile", "--no-deploy", "--mode", "test"]);
    assert_refused(&output, 2, &["--no-deploy"]);
    assert!(
        stderr(&output).starts_with("error: "),
        "{}",
        stderr(&output)
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn check_prints_one_line_per_finding_and_exits_with_the_worst() {
    let sandbox = Sandbox::new("check", "");
    sandbox.write(
        "data/settings.toml",
        &format!("[common]\n{}", sandbox.pes_folder_line()),
    );
    sandbox.write("data/teams_list.txt", "ID\tName\n714\t/co/\n");
    // `check` reads every model, so the clean export holds a real one with no finding: the
    // tracer's right glove.
    let glove = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../tools/team_compiler/tests/fixtures/tracer/studio/egg Midcup Tracer/Players/05 - The Chad Stormworks Player/glove_r.fmdl",
    );
    let clean = sandbox.root.join("exports/co Midcup Clean/Players/03 - A");
    fs::create_dir_all(&clean).unwrap();
    fs::copy(&glove, clean.join("face_high.fmdl")).unwrap();
    sandbox.write("exports/co Midcup Error/Players/03 - A/readme.doc", "");

    let output = sandbox.run(&[
        "team-compiler",
        "check",
        "--export",
        "exports/co Midcup Clean",
    ]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "- co Midcup Clean: Info export_identified (team=/co/, id=714)\n"
    );

    let output = sandbox.run(&[
        "team-compiler",
        "check",
        "--export",
        "exports/co Midcup Error",
    ]);
    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "- co Midcup Error: Error file_type_disallowed at Players/03 - A (file=readme.doc)\n\
         - co Midcup Error: Info export_identified (team=/co/, id=714)\n"
    );
    assert!(stderr(&output).is_empty(), "{}", stderr(&output));
}

// TC-CLI-04
#[test]
fn compile_with_a_positional_root_compiles_it_and_leaves_the_settings_file_alone() {
    let sandbox = Sandbox::new("positional_root", "");
    let settings = format!("[common]\npes_version = 21\n{}", sandbox.pes_folder_line());
    sandbox.write("data/settings.toml", &settings);
    sandbox.write("data/teams_list.txt", "ID\tName\n714\t/co/\n790\t/dbg/\n");
    let kit = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../tools/team_compiler/tests/fixtures/tracer/studio/egg Midcup Tracer/Kits/g1/kit.dds",
    );
    for folder in [
        "elsewhere/co Midcup Kit/Kits/p1",
        "exports/dbg Midcup Other/Kits/p1",
    ] {
        let folder = sandbox.root.join(folder);
        fs::create_dir_all(&folder).unwrap();
        fs::copy(&kit, folder.join("kit.dds")).unwrap();
    }

    let output = sandbox.run(&["team-compiler", "compile", "elsewhere", "--no-deploy"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    // The sandbox's PES folder does not exist, so every working bin is the bundled one.
    let cpk = sandbox.root.join("output").join("4cc_99_test.cpk");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            "- Info bin_source (bin=TeamColor.bin, cpk=bundled)\n\
             - Info bin_source (bin=UniColor.bin, cpk=bundled)\n\
             - Info bin_source (bin=UniformParameter.bin, cpk=bundled)\n\
             - co Midcup Kit: Info export_identified (team=/co/, id=714)\n\
             - co Midcup Kit: Info team_colors_missing\n\
             - Info deploy_skipped_by_flag (path={})\n",
            cpk.display()
        )
    );
    assert!(cpk.is_file());
    assert_eq!(
        fs::read(sandbox.root.join("data/settings.toml")).unwrap(),
        settings.as_bytes()
    );
}

// ST-CLI-02
#[test]
fn a_compile_with_stderr_piped_draws_no_line_and_leaves_stderr_empty() {
    let sandbox = Sandbox::new("piped_stderr", "");
    sandbox.write(
        "data/settings.toml",
        &format!("[common]\npes_version = 21\n{}", sandbox.pes_folder_line()),
    );
    sandbox.write("data/teams_list.txt", "ID\tName\n714\t/co/\n");
    let kit = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../tools/team_compiler/tests/fixtures/tracer/studio/egg Midcup Tracer/Kits/g1/kit.dds",
    );
    let folder = sandbox.root.join("exports/co Midcup Kit/Kits/p1");
    fs::create_dir_all(&folder).unwrap();
    fs::copy(&kit, folder.join("kit.dds")).unwrap();

    let output = sandbox.run(&["team-compiler", "compile", "--no-deploy"]);

    // The process's stderr is a pipe, so the progress line and the summary draw nothing:
    // stderr is empty and stdout holds exactly the lines it holds in a terminal.
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            "- Info bin_source (bin=TeamColor.bin, cpk=bundled)\n\
             - Info bin_source (bin=UniColor.bin, cpk=bundled)\n\
             - Info bin_source (bin=UniformParameter.bin, cpk=bundled)\n\
             - co Midcup Kit: Info export_identified (team=/co/, id=714)\n\
             - co Midcup Kit: Info team_colors_missing\n\
             - Info deploy_skipped_by_flag (path={})\n",
            sandbox
                .root
                .join("output")
                .join("4cc_99_test.cpk")
                .display()
        )
    );
    assert!(stderr(&output).is_empty(), "{}", stderr(&output));
}

#[test]
fn a_settings_file_that_is_not_toml_stops_the_binary_naming_it() {
    let sandbox = Sandbox::new("bad_settings", "this is = = not toml");
    let output = sandbox.run(&["team-compiler", "check"]);
    assert_refused(&output, 2, &["settings.toml"]);
}

#[test]
fn the_settings_beside_the_executable_are_read_whatever_the_working_folder() {
    let sandbox = Sandbox::new("working_folder", "");
    let settings = format!(
        "[common]\n{}[team-compiler]\ncpk_name = 'con'\n",
        sandbox.pes_folder_line()
    );
    sandbox.write("data/settings.toml", &settings);
    let elsewhere = sandbox.root.join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    let output = sandbox.run_from(&elsewhere, &["team-compiler", "compile", "--no-deploy"]);
    assert_refused(&output, 2, &["cpk_name"]);
}

// ST-CLI-01
#[test]
fn a_read_only_data_dir_prints_the_condition_as_stderrs_first_line() {
    let sandbox = Sandbox::new("read_only", "");
    // The check reads the PES folder, so the setting names the sandbox's own, written while
    // the file is still writable.
    sandbox.write(
        "data/settings.toml",
        &format!("[common]\n{}", sandbox.pes_folder_line()),
    );
    let file = sandbox.root.join("data/settings.toml");
    let mut readonly = fs::metadata(&file).unwrap().permissions();
    readonly.set_readonly(true);
    fs::set_permissions(&file, readonly).unwrap();
    // The file is made writable again before any assertion, or the sandbox could not be
    // cleared for the next run.
    let output = sandbox.run(&["team-compiler", "check"]);
    let mut writable = fs::metadata(&file).unwrap().permissions();
    #[expect(
        clippy::permissions_set_readonly_false,
        reason = "a test fixture must get its writable bit back or the sandbox cannot be cleared"
    )]
    writable.set_readonly(false);
    fs::set_permissions(&file, writable).unwrap();

    let text = stderr(&output);
    assert_eq!(
        text.lines().next(),
        Some("Data folder is read-only"),
        "stderr's first line is the condition: {text}"
    );
}

/// A warning logged while the notice printer runs reaches stderr and the command ends: the
/// printer once held stderr's lock for the whole command, and the logger's write of this
/// warning (the teams list cannot be written into a folder that does not exist) blocked
/// forever. The binary runs under a deadline so a regression fails instead of hanging.
#[test]
fn a_warning_logged_during_a_command_does_not_hang_it() {
    let sandbox = Sandbox::new("warning_no_hang", "");
    sandbox.write(
        "data/settings.toml",
        &format!(
            "[common]\n{}\n[team-compiler]\nteams_list_path = 'missing/teams_list.txt'\n",
            sandbox.pes_folder_line()
        ),
    );
    let mut child = Command::new(&sandbox.exe)
        .args(["team-compiler", "compile", "--no-deploy"])
        .current_dir(&sandbox.root)
        .stdout(fs::File::create(sandbox.root.join("stdout.txt")).unwrap())
        .stderr(fs::File::create(sandbox.root.join("stderr.txt")).unwrap())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if std::time::Instant::now() > deadline {
            child.kill().unwrap();
            panic!("the command was still running after 60 s: a stderr write is blocked");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    let text = fs::read_to_string(sandbox.root.join("stderr.txt")).unwrap();
    assert!(
        text.contains("the teams list cannot be written"),
        "the warning reached stderr ({status}): {text}"
    );
}

#[test]
fn the_help_lists_the_commands_and_exits_zero() {
    let sandbox = Sandbox::new("help", "");
    let output = sandbox.run(&["team-compiler", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8_lossy(&output.stdout);
    for command in ["compile", "check", "upgrade-dpfl"] {
        assert!(text.contains(command), "help lacks {command}: {text}");
    }
}
