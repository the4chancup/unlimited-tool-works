//! The Unlimited Tool Works binary: registers the tools, then launches the GUI or dispatches the
//! CLI. With no arguments it opens the shell on the registered tools; `--gui` autorun arrives with
//! the GUI phase. Two hidden arguments are not commands a member types: `--crash-monitor`
//! runs the crash-dump monitor a window spawns, and `--crash-test` its crash trigger. On
//! Windows the exe is a GUI-subsystem application on every build, so a double-click opens no
//! console window beside it.
// Not on the unit tests' harness, built from this file too: a GUI-subsystem test binary's
// output never reaches the test runner, so a failing assertion would print nothing. The
// exe the integration tests run keeps the attribute.
#![cfg_attr(not(test), windows_subsystem = "windows")]

mod console;
mod crash_dump;
mod gui_log;
#[cfg(test)]
mod test_dir;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use anyhow::Context;
use clap::ArgMatches;
use crossbeam_channel::unbounded;
use log::LevelFilter;
use rfd::MessageLevel;
use studio_core::icons;
use studio_core::{
    AppPaths, EXE_NAME, LaunchMode, PlannedTool, SETTINGS_FILE_NAME, Settings, ShellCondition,
    StudioApp, StudioTool, ToolContext, parse_launch, resolve_data_dir, run_cli, run_gui, run_help,
    settings_file_writable, user_config_dir,
};

use crate::console::{spawn_notice_printer, spawn_printer};
use crate::crash_dump::{
    HiddenLaunch, hidden_launch, overflow_the_stack, run_monitor, watch_crashes,
};
use crate::gui_log::{
    guard_gui, install_gui_logger, install_panic_hook, show_dialog, unclean_notice,
};

/// The suite's whole registry as data, in sidebar order. The shell lists after the
/// registered tools every entry whose id no
/// registered tool has, grayed as the roadmap. A registered tool's entry repeats what its
/// `icon()` returns.
const PLANNED: [PlannedTool; 14] = [
    PlannedTool {
        id: "team-compiler",
        label: "Team compiler",
        icon: icons::TEAM_COMPILER,
    },
    PlannedTool {
        id: "save-editor",
        label: "Save editor",
        icon: icons::SAVE_EDITOR,
    },
    PlannedTool {
        id: "stadium-compiler",
        label: "Stadium compiler",
        icon: icons::STADIUM_COMPILER,
    },
    PlannedTool {
        id: "export-upgrader",
        label: "Export upgrader",
        icon: icons::EXPORT_UPGRADER,
    },
    PlannedTool {
        id: "music-player",
        label: "Music player",
        icon: icons::MUSIC_PLAYER,
    },
    PlannedTool {
        id: "music-export-editor",
        label: "Music export editor",
        icon: icons::MUSIC_EXPORT_EDITOR,
    },
    PlannedTool {
        id: "match-tracker",
        label: "Match tracker",
        icon: icons::MATCH_TRACKER,
    },
    PlannedTool {
        id: "kit-config-editor",
        label: "Kit config editor",
        icon: icons::KIT_CONFIG_EDITOR,
    },
    PlannedTool {
        id: "refs-arranger",
        label: "Refs arranger",
        icon: icons::REFS_ARRANGER,
    },
    PlannedTool {
        id: "balls-compiler",
        label: "Balls compiler",
        icon: icons::BALLS_COMPILER,
    },
    PlannedTool {
        id: "player-aesthetics-editor",
        label: "Player aesthetics editor",
        icon: icons::PLAYER_AESTHETICS_EDITOR,
    },
    PlannedTool {
        id: "team-creator",
        label: "Team creator",
        icon: icons::TEAM_CREATOR,
    },
    PlannedTool {
        id: "ruleset-editor",
        label: "Ruleset editor",
        icon: icons::RULESET_EDITOR,
    },
    PlannedTool {
        id: "db-generator",
        label: "DB generator",
        icon: icons::DB_GENERATOR,
    },
];

/// The registered tools this build shows on the sidebar only as planned entries, grayed
/// and not clickable: the 0.0.1 preview's three tools whose views come later
/// (`team-compiler`, `export-upgrader`, `ruleset-editor`). Their CLI and help chapter
/// stay; the settings menu draws no section for them. The list goes when their views land.
const CLI_ONLY: [&str; 3] = ["team-compiler", "export-upgrader", "ruleset-editor"];

/// The registered tools, in sidebar order.
fn tools() -> Vec<Box<dyn StudioTool>> {
    vec![
        Box::new(team_compiler::Tool::new()),
        Box::new(save_editor::Tool::new()),
        Box::new(ruleset_editor::Tool::new()),
        Box::new(export_upgrader::Tool::new()),
    ]
}

/// The CLI modes' diagnostic sink: `-v` sets the level,
/// `RUST_LOG` overrides it. The GUI mode gets its own sink, `utw.log`
/// (`gui_log::install_gui_logger`), installed once the data location resolves.
fn install_cli_logger(verbosity: u8) {
    env_logger::Builder::new()
        .filter_level(log_level(verbosity))
        .parse_default_env()
        .init();
}

/// The level `-v` repeated `verbosity` times asks for: warnings only, then info, debug, trace.
fn log_level(verbosity: u8) -> LevelFilter {
    match verbosity {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}

/// The folder holding the executable, which the data location and path settings resolve against.
fn exe_dir() -> anyhow::Result<PathBuf> {
    let exe = std::env::current_exe().context("cannot locate the executable")?;
    Ok(exe
        .parent()
        .context("the executable's path has no folder")?
        .to_path_buf())
}

/// Resolves the data location (presence-based, never asked and never written from the CLI) and
/// loads the settings from it, with every tool's defaults merged in memory. With no settings file
/// yet, the defaults alone and no data directory.
fn load_settings(
    tools: &[Box<dyn StudioTool>],
    exe_dir: PathBuf,
) -> anyhow::Result<(Settings, AppPaths)> {
    let data_dir = resolve_data_dir(&exe_dir, user_config_dir().as_deref());
    let mut settings = match &data_dir {
        Some(dir) => {
            let path = dir.join(SETTINGS_FILE_NAME);
            Settings::load(&path).with_context(|| path.display().to_string())?
        }
        None => Settings::default(),
    };
    for tool in tools {
        settings.merge_defaults(tool.id(), &tool.default_settings());
    }
    Ok((settings, AppPaths { exe_dir, data_dir }))
}

/// What both modes start from: the settings and the base folders. A failure returns the
/// exit code and its text, which each mode reports its own way (the CLI prints it, the
/// window shows it in a dialog): 3 when the executable's folder cannot be found (an
/// environment failure), 2 when the settings file cannot be loaded (a configuration
/// error: nothing ran). These are the binary's own codes, not a tool's.
fn startup(tools: &[Box<dyn StudioTool>]) -> Result<(Settings, AppPaths), (ExitCode, String)> {
    match exe_dir() {
        Ok(exe_dir) => startup_in(tools, exe_dir),
        Err(error) => Err((ExitCode::from(3), format!("error: {error:#}"))),
    }
}

/// `startup` once the executable's folder is found; the split lets a test point the
/// settings load at a folder of its own.
fn startup_in(
    tools: &[Box<dyn StudioTool>],
    exe_dir: PathBuf,
) -> Result<(Settings, AppPaths), (ExitCode, String)> {
    load_settings(tools, exe_dir).map_err(|error| (ExitCode::from(2), format!("error: {error:#}")))
}

/// Runs one tool subcommand headless and returns the process exit code: the tool's own, the
/// startup failure's, or 3 when the console printer failed.
#[expect(clippy::print_stderr, reason = "CLI result output is the binary's job")]
fn run_cli_mode(tools: &[Box<dyn StudioTool>], tool: &str, matches: &ArgMatches) -> ExitCode {
    let (settings, paths) = match startup(tools) {
        Ok(loaded) => loaded,
        Err((code, text)) => {
            eprintln!("{text}");
            return code;
        }
    };
    // The CLI never writes settings, so the read-only data directory is probed rather than
    // learned from a failed save. The condition prints once, as stderr's first line
    // (ST-CLI-01).
    if let Some(dir) = &paths.data_dir
        && !settings_file_writable(dir)
    {
        eprintln!("{}", ShellCondition::DataDirReadOnly.text());
    }
    let (events_tx, events_rx) = unbounded();
    let (requests_tx, requests_rx) = unbounded();
    let printer = spawn_printer(events_rx);
    let notices = spawn_notice_printer(requests_rx);
    let ctx = ToolContext::new(
        Arc::new(Mutex::new(settings)),
        paths,
        events_tx,
        requests_tx,
    );
    let result = run_cli(tools, tool, matches, &ctx);
    // The printers end when every sender is gone, and the context holds them.
    drop(ctx);
    // A printer's `Err` is a failed write: it loses lines like a panic does.
    if !matches!(printer.join(), Ok(Ok(()))) || !matches!(notices.join(), Ok(Ok(()))) {
        // Findings were lost, so the tool's verdict cannot be reported as it stands: 3 is the
        // code of a run that did not finish as reported.
        eprintln!("error: the output to the console stopped unexpectedly");
        return ExitCode::from(3);
    }
    match result {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(error.exit_code)
        }
    }
}

/// `UTW help`: prints or exports the help's Markdown, no settings load (the help reads
/// none). A refusal prints as `error: …` with its exit code, as a tool's does.
#[expect(clippy::print_stderr, reason = "CLI result output is the binary's job")]
fn run_help_mode(
    tools: &[Box<dyn StudioTool>],
    chapter: Option<String>,
    topic: Option<String>,
    export: Option<PathBuf>,
) -> ExitCode {
    match run_help(
        tools,
        chapter.as_deref(),
        topic.as_deref(),
        export.as_deref(),
        &mut std::io::stdout().lock(),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(error.exit_code)
        }
    }
}

/// Opens the shell on the registered tools and returns when the window closes: 0, the startup
/// failure's code, 3 when the window could not be opened or the crash test's trigger
/// returned, or 101 when a panic ended it. A failure shows its text in a dialog: a window
/// started by a double-click has no console to print it to. `crash_test` is the hidden
/// `--crash-test stack` launch (ST-RUN-10): the window's start runs as usual, then the
/// stack is overflowed before any window.
fn run_gui_mode(tools: Vec<Box<dyn StudioTool>>, crash_test: bool) -> ExitCode {
    let (settings, paths) = match startup(&tools) {
        Ok(loaded) => loaded,
        Err((code, text)) => {
            show_dialog(MessageLevel::Error, &text);
            return code;
        }
    };
    // The log and the hook go in before the window: the log needs the data location the
    // startup resolved, and the hook needs the log so a panic's line lands in it.
    let installed_log = install_gui_logger(paths.data_dir.as_deref());
    install_panic_hook();
    // The monitor starts with the log, once the data location exists: a first run has neither.
    let _crash_handler = match (&installed_log, &paths.data_dir) {
        (Some(_), Some(data_dir)) => {
            watch_crashes(std::env::current_exe(), data_dir, log::logger())
        }
        _ => None,
    };
    // ST-RUN-10's trigger, past the monitor's start and before any window, so the test
    // needs no display.
    if crash_test {
        let depth = overflow_the_stack(0);
        log::error!("the crash test's stack overflow returned {depth} instead of crashing");
        return ExitCode::from(3);
    }
    // A previous session's log without a closing line means a crash the hook could not
    // see, or a session Windows ended (ST-RUN-09): say so before the window opens,
    // naming the old log, and the dump beside it when the monitor wrote one (ST-RUN-10).
    if let Some(text) = installed_log.as_ref().and_then(unclean_notice) {
        show_dialog(MessageLevel::Info, &text);
    }
    // The receiver is dropped at once (the `_` pattern): a tool's run has its own sink
    // (`ToolContext::with_events`), and the notices the tools publish reach the status bar
    // through the requests channel — the shell consumes no run events of its own.
    let (events_tx, _) = unbounded();
    let (requests_tx, requests_rx) = unbounded();
    let ctx = ToolContext::new(
        Arc::new(Mutex::new(settings)),
        paths,
        events_tx,
        requests_tx,
    );
    guard_gui(
        move || match run_gui(StudioApp::new(
            tools,
            &PLANNED,
            &CLI_ONLY,
            ctx,
            requests_rx,
            user_config_dir(),
        )) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                show_dialog(MessageLevel::Error, &format!("error: {error:#}"));
                ExitCode::from(3)
            }
        },
        installed_log
            .as_ref()
            .map(|installed| installed.path.as_path()),
        |text| show_dialog(MessageLevel::Error, text),
        log::logger(),
    )
}

/// Binds this process to the console of the terminal that started it, so a command's
/// output prints where it was run: a GUI-subsystem exe owns no console of its own.
#[cfg(windows)]
fn attach_parent_console() {
    use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
    // SAFETY: `AttachConsole` takes a process id and no pointer of ours and binds no
    // memory; it only attaches this process to its parent's console.
    let attached = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
    // The `Result` is dropped: the call fails when the parent has no console (a launch
    // from Explorer with arguments), and output then goes where the inherited handles
    // point, which is all a GUI-subsystem exe can do.
    drop(attached);
}

/// No subsystem split off Windows: nothing to attach to.
#[cfg(not(windows))]
fn attach_parent_console() {}

#[expect(clippy::print_stderr, reason = "CLI result output is the binary's job")]
fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    // The monitor and the crash test are read before the command line is parsed:
    // neither is a command a member types, so neither is in `--help`.
    match hidden_launch(&args) {
        Some(HiddenLaunch::Monitor { data_dir, socket }) => {
            return run_monitor(&data_dir, &socket);
        }
        Some(HiddenLaunch::CrashTest) => return run_gui_mode(tools(), true),
        None => {}
    }
    // More than the program name in the arguments means a CLI run, a `--help` or a clap
    // error, all of which print; the window mode attaches to no console, which a
    // double-click does not have.
    if args.len() > 1 {
        attach_parent_console();
    }
    let tools = tools();
    let launch = match parse_launch(&tools, args) {
        Ok(launch) => launch,
        Err(error) => error.exit(),
    };
    // The GUI mode installs its own sink (`gui_log`) after the startup resolves the data
    // location; the console window's `env_logger` is the CLI, `--gui` and help modes'.
    if !matches!(launch.mode, LaunchMode::Gui) {
        install_cli_logger(launch.verbosity);
    }
    match launch.mode {
        LaunchMode::Gui => run_gui_mode(tools, false),
        LaunchMode::GuiAutorun { .. } => {
            eprintln!(
                "`--gui` comes in a later version. Run the command without it, or run `{EXE_NAME}` alone to open the window."
            );
            ExitCode::from(2)
        }
        LaunchMode::Cli { tool, matches } => run_cli_mode(&tools, &tool, &matches),
        LaunchMode::Help {
            chapter,
            topic,
            export,
        } => run_help_mode(&tools, chapter, topic, export),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::test_dir::TestDir;

    // The registry repeats the tools' own labels and icons: a registered tool's rail glyph is
    // its `icon()`, never a second spelling that could drift.
    #[test]
    fn every_registered_tool_has_a_matching_planned_entry() {
        for tool in tools() {
            let entry = PLANNED
                .iter()
                .find(|entry| entry.id == tool.id())
                .unwrap_or_else(|| panic!("{} is missing from the registry", tool.id()));
            assert_eq!(entry.label, tool.label());
            assert_eq!(entry.icon, tool.icon());
        }
    }

    // The sidebar draws a CLI-only tool as its planned entry, so each mark needs both:
    // a registered tool (its CLI, help and settings stay) and a registry entry to draw.
    #[test]
    fn every_cli_only_tool_is_registered_with_a_planned_entry() {
        let registered: Vec<&str> = tools().iter().map(|tool| tool.id()).collect();
        for id in CLI_ONLY {
            assert!(registered.contains(&id), "{id} is not a registered tool");
            assert!(
                PLANNED.iter().any(|entry| entry.id == id),
                "{id} is missing from the registry"
            );
        }
    }

    #[test]
    fn each_verbosity_maps_to_its_level() {
        assert_eq!(log_level(0), LevelFilter::Warn);
        assert_eq!(log_level(1), LevelFilter::Info);
        assert_eq!(log_level(2), LevelFilter::Debug);
        assert_eq!(log_level(3), LevelFilter::Trace);
    }

    // A process can install one logger only, so this is the one test that installs it.
    #[test]
    fn installing_the_logger_sets_the_verbosity_level() {
        install_cli_logger(1);
        // `RUST_LOG` in the environment overrides `-v` by design, so with it set the test can
        // only check that a logger is installed.
        if std::env::var_os("RUST_LOG").is_some() {
            assert_ne!(log::max_level(), LevelFilter::Off);
        } else {
            assert_eq!(log::max_level(), LevelFilter::Info);
        }
    }

    // Every `when`/`effect` cell of every registered tool's catalog reads for a member:
    // none of the code's own vocabulary reaches the Messages topic (`CONTRIBUTING.md`
    // "Code style").
    #[test]
    fn every_catalog_row_is_written_for_a_member() {
        const WORDS: &[&str] = &[
            "DropFile",
            "DropSlot",
            "DropFolder",
            "DropExport",
            "AbortRun",
            "Red",
            "Blue",
            "Fox",
            "context",
        ];
        const SUBSTRINGS: &[&str] = &[".md", "spec/", "plans/", "—"];
        let mut hits = Vec::new();
        for tool in tools() {
            for row in &tool.help().messages {
                for (cell_name, cell) in [("when", row.when), ("effect", row.effect)] {
                    for word in cell.split(|c: char| !c.is_ascii_alphanumeric()) {
                        if WORDS.contains(&word) {
                            hits.push(format!("{} {} {cell_name}: {word}", tool.id(), row.code));
                        }
                    }
                    for needle in SUBSTRINGS {
                        if cell.contains(needle) {
                            hits.push(format!("{} {} {cell_name}: {needle}", tool.id(), row.code));
                        }
                    }
                }
            }
        }
        assert!(hits.is_empty(), "{}", hits.join("\n"));
    }

    // A settings file that does not parse fails startup with code 2 (nothing ran) and
    // the `error: ` text the mode reports; the test's `data/` makes the location
    // portable, so the real config folder is never read.
    #[test]
    fn a_settings_file_that_does_not_parse_fails_startup_with_code_2() {
        let dir = TestDir::new("bad_settings");
        fs::create_dir_all(dir.path().join("data")).unwrap();
        fs::write(
            dir.path().join("data").join(SETTINGS_FILE_NAME),
            "not = [toml",
        )
        .unwrap();
        let Err((code, text)) = startup_in(&tools(), dir.path().to_path_buf()) else {
            panic!("a settings file that does not parse is a startup failure")
        };
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::from(2)));
        assert!(text.starts_with("error: "), "{text}");
    }
}
