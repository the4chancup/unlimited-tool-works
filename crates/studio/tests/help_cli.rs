//! `UTW help`, run through the real binary: the Markdown a chapter, the `--export`
//! files and a refusal's `error: ` line. The help reads no settings, so the binary runs as
//! built, in place.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use studio_core::EXE_NAME;

fn studio(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_utw"))
        .args(args)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The Team compiler's last catalog row (`notes_encoding_invalid`).
const NOTES_ENCODING_INVALID: &str = "| `notes_encoding_invalid` | Error | The root `notes.txt` is not valid UTF-8. | The note is left out; the export otherwise compiles. |";
/// The Export upgrader's last catalog row: the same code, the upgrader's own effect.
const UPGRADER_NOTES_ENCODING_INVALID: &str = "| `notes_encoding_invalid` | Error | The root `notes.txt` is not valid UTF-8. | The run is not clean, so nothing is written. |";

// ST-HELP-04
#[test]
fn a_chapter_prints_its_markdown_ending_with_the_messages_table() {
    let output = studio(&["help", "team-compiler"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.starts_with("# Team compiler\n"), "{stdout}");
    assert!(stdout.contains("## Running from a terminal"), "{stdout}");
    assert!(
        stdout.contains(
            "| `export_extract_failed` | Error | The archive (`.zip` or `.7z`) cannot be opened, or a path inside it is unusable: it leads outside the archive, or two names differ only in letter case or Unicode form. | The export is not compiled. Repack the archive, or unpack it into a folder. |"
        ),
        "{stdout}"
    );
    assert_eq!(
        stdout.lines().last(),
        Some(NOTES_ENCODING_INVALID),
        "{stdout}"
    );
}

// ST-HELP-04
#[test]
fn the_export_writes_one_markdown_file_per_chapter() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("help_cli_export");
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    let output = studio(&["help", "--export", dir.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert!(output.stdout.is_empty(), "the export prints nothing");

    let mut files: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    assert_eq!(
        files,
        [
            "export-upgrader.md",
            "first-run.md",
            "ruleset-editor.md",
            "save-editor.md",
            "setting-up.md",
            "team-compiler.md"
        ]
    );

    // Each file is what `help <id>` prints; each tool's ends with its last catalog row.
    let last_rows = [
        ("team-compiler", NOTES_ENCODING_INVALID),
        (
            "save-editor",
            "| `fpc_custom_skin_on_non_fpc` | Warning | A player wears a custom skin, but his body is not hidden by the FPC hide boots ID from Settings, so the skin hides his body but not his jersey. | Nothing is changed for you. Hide the body too, or pick a preset skin. |",
        ),
        ("export-upgrader", UPGRADER_NOTES_ENCODING_INVALID),
        (
            "ruleset-editor",
            "| `summary_written` | Info | `summary -o`: the file's readable rules text was written to `path`. | The file it replaced, if there was one, is kept beside it as a .bak copy. |",
        ),
    ];
    for (id, last_row) in last_rows {
        let printed = studio(&["help", id]);
        assert_eq!(printed.status.code(), Some(0), "{id}: {}", stderr(&printed));
        let file = fs::read_to_string(dir.join(format!("{id}.md"))).unwrap();
        assert_eq!(file, stdout(&printed), "{id}.md differs from `help {id}`");
        assert_eq!(file.lines().last(), Some(last_row), "{id}");
    }
    // The core chapters end with their last topic, having no catalog.
    for id in ["setting-up", "first-run"] {
        let printed = studio(&["help", id]);
        assert_eq!(printed.status.code(), Some(0), "{id}: {}", stderr(&printed));
        assert_eq!(
            fs::read_to_string(dir.join(format!("{id}.md"))).unwrap(),
            stdout(&printed),
            "{id}.md differs from `help {id}`"
        );
        assert!(!stdout(&printed).contains("## Messages"), "{id}");
    }
}

/// The indented first words of `help`'s `Commands:` section (clap lays the subcommands out
/// under the `Commands:` heading, one line each, until an unindented line ends it).
fn commands_of(help: &str) -> Vec<String> {
    let mut lines = help.lines();
    let mut commands = Vec::new();
    while let Some(line) = lines.next() {
        if line != "Commands:" {
            continue;
        }
        for line in lines.by_ref() {
            if !line.starts_with(char::is_whitespace) {
                break;
            }
            commands.push(line.split_whitespace().next().unwrap().to_owned());
        }
        break;
    }
    commands
}

// The project is Unlimited Tool Works, so no help text a member sees names 4cc Studio, and
// none spells the executable `UnlimitedToolWorks`: it is `UTW`.
#[test]
fn no_member_visible_text_names_4cc_studio() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("help_cli_names");
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    let output = studio(&["help", "--export", dir.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));

    // (source, text): every exported page, the root --help, every tool's --help and every
    // command's --help under it.
    let mut pages: Vec<(String, String)> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(&path).unwrap(),
            )
        })
        .collect();
    let root = studio(&["--help"]);
    assert_eq!(root.status.code(), Some(0), "stderr: {}", stderr(&root));
    let root_help = stdout(&root);
    let tools = commands_of(&root_help);
    assert!(
        tools.len() >= 4,
        "the root --help's Commands: section lists the tools: {root_help}"
    );
    for tool in &tools {
        let output = studio(&[tool.as_str(), "--help"]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{tool} --help: {}",
            stderr(&output)
        );
        let tool_help = stdout(&output);
        for command in commands_of(&tool_help) {
            if command == "help" {
                continue;
            }
            let output = studio(&[tool.as_str(), command.as_str(), "--help"]);
            assert_eq!(
                output.status.code(),
                Some(0),
                "{tool} {command} --help: {}",
                stderr(&output)
            );
            pages.push((format!("{tool} {command} --help"), stdout(&output)));
        }
        pages.push((format!("{tool} --help"), tool_help));
    }
    pages.push(("--help".to_owned(), root_help.clone()));

    for (source, text) in &pages {
        for (number, line) in text.lines().enumerate() {
            let lower = line.to_ascii_lowercase();
            assert!(
                !lower.contains("4cc studio")
                    && !lower.contains("4cc-studio")
                    && !line.contains("UnlimitedToolWorks"),
                "{source}:{}: {line}",
                number + 1
            );
            assert!(
                !line
                    .split(|c: char| !c.is_ascii_alphanumeric())
                    .any(|word| word == "Studio"),
                "{source}:{}: {line}",
                number + 1
            );
        }
    }

    let usage = root_help
        .lines()
        .find(|line| line.starts_with("Usage:"))
        .expect("the root --help has a usage line");
    let expected = if cfg!(windows) {
        "Usage: UTW "
    } else {
        "Usage: utw "
    };
    assert!(
        usage.starts_with(expected),
        "the usage line names the platform's executable: {usage}"
    );
    let compiler = &pages
        .iter()
        .find(|(source, _)| source == "team-compiler.md")
        .expect("the export wrote the Team compiler's page")
        .1;
    // A Markdown page cannot follow the platform, so the pages spell `UTW` on both.
    assert!(
        compiler.contains("UTW team-compiler compile"),
        "the Team compiler's page names the executable"
    );
}

#[test]
fn version_prints_the_name_and_the_status_bar_s_version() {
    let output = studio(&["--version"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    // The status bar's one-liner: the version, the release stage while there is
    // one, ` (debug)` for a debug build — which the test binary is.
    let build = if cfg!(debug_assertions) {
        " (debug)"
    } else {
        ""
    };
    assert_eq!(
        stdout(&output),
        format!(
            "{EXE_NAME} v{} pre-alpha{build}\n",
            env!("CARGO_PKG_VERSION")
        )
    );
}

#[test]
fn an_unknown_chapter_is_refused_with_exit_2() {
    let output = studio(&["help", "nope"]);
    assert_eq!(output.status.code(), Some(2), "stderr: {}", stderr(&output));
    assert!(
        stderr(&output).starts_with("error: there is no help chapter"),
        "{}",
        stderr(&output)
    );
}
