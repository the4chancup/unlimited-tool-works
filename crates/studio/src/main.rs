//! The Unlimited Tool Works binary: registers the tools, then launches the GUI or dispatches the
//! CLI. With no arguments it opens the shell on the registered tools; `--gui` is refused until
//! a tool's view can run a command. Two hidden arguments are not commands a member types:
//! `--crash-monitor` runs the crash-dump monitor a window spawns, and `--crash-test` its crash
//! trigger. On Windows the exe is a GUI-subsystem application on every build, so a
//! double-click opens no console window beside it.
// Not on the unit tests' harness, built from this file too: a GUI-subsystem test binary's
// output never reaches the test runner, so a failing assertion would print nothing. The
// exe the integration tests run keeps the attribute.
#![cfg_attr(not(test), windows_subsystem = "windows")]

mod console;
mod crash_dump;
mod gui_log;
mod instance;
#[cfg(test)]
mod test_dir;

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use anyhow::Context;
use clap::ArgMatches;
use crossbeam_channel::{Receiver, unbounded};
use log::LevelFilter;
use rfd::MessageLevel;
use studio_core::icons;
use studio_core::updater::{self, CheckReply, Fetch};
use studio_core::{
    AppPaths, CommonSettings, EXE_NAME, LaunchMode, PlannedTool, SETTINGS_FILE_NAME, Settings,
    ShellCondition, StudioApp, StudioTool, ToolContext, parse_launch, resolve_data_dir, run_cli,
    run_gui, run_help, settings_file_writable, user_config_dir,
};

use crate::console::{RunLog, log_panics, spawn_notice_printer, spawn_printer};
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
        label: "Ruleset editor (WIP)",
        icon: icons::RULESET_EDITOR,
    },
    PlannedTool {
        id: "db-generator",
        label: "DB generator",
        icon: icons::DB_GENERATOR,
    },
];

/// The registered tools this build shows on the sidebar only as planned entries, grayed
/// and not clickable: the 0.0.1 preview's two tools whose views come later
/// (`team-compiler`, `export-upgrader`). Their CLI and help chapter stay; the settings
/// menu draws no section for them. The list goes when their views land.
const CLI_ONLY: [&str; 2] = ["team-compiler", "export-upgrader"];

/// The settings file's template: the bundle ships this same `data/settings.toml`, comments
/// included, and a file written fresh starts from it.
const SETTINGS_TEMPLATE: &str = include_str!("../../../resources/bundle/data/settings.toml");

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
    }
    .with_template(SETTINGS_TEMPLATE);
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
fn startup(tools: &[Box<dyn StudioTool>]) -> Result<(Settings, AppPaths), (u8, String)> {
    match exe_dir() {
        Ok(exe_dir) => startup_in(tools, exe_dir),
        Err(error) => Err((3, format!("error: {error:#}"))),
    }
}

/// `startup` once the executable's folder is found; the split lets a test point the
/// settings load at a folder of its own.
fn startup_in(
    tools: &[Box<dyn StudioTool>],
    exe_dir: PathBuf,
) -> Result<(Settings, AppPaths), (u8, String)> {
    load_settings(tools, exe_dir).map_err(|error| (2, format!("error: {error:#}")))
}

/// Runs one tool subcommand headless and returns the process exit code: the tool's own, the
/// startup failure's, 2 when the run's log file cannot be opened, or 3 when the console
/// printer or a write to the log failed. With `log_file` the run appends everything it
/// prints to that file, between a first line `$ UTW <the arguments>` and a last line
/// `exit <code>`.
#[expect(clippy::print_stderr, reason = "CLI result output is the binary's job")]
fn run_cli_mode(
    tools: &[Box<dyn StudioTool>],
    tool: &str,
    matches: &ArgMatches,
    log_file: Option<&Path>,
) -> ExitCode {
    let log = match log_file {
        None => None,
        Some(path) => match RunLog::open(path) {
            Ok(log) => Some(Arc::new(log)),
            Err(error) => {
                eprintln!(
                    "error: cannot open the log file {}: {error}",
                    path.display()
                );
                return ExitCode::from(2);
            }
        },
    };
    if let Some(log) = &log {
        log_panics(log.clone());
    }
    let result = log_line(log.as_deref(), &command_line(std::env::args_os().skip(1)))
        .and_then(|()| run_command(tools, tool, matches, log.clone()))
        .and_then(|code| log_line(log.as_deref(), &format!("exit {code}")).map(|()| code));
    match result {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            // A log that silently stops is worse than none, so the run reports a failure
            // even when the tool's own verdict was a success.
            eprintln!("error: the log file could not be written: {error}");
            ExitCode::from(3)
        }
    }
}

/// The log's first line: `$ UTW`, then `args` one space apart, an argument holding a space
/// inside double quotes.
fn command_line(args: impl IntoIterator<Item = OsString>) -> String {
    let args: Vec<String> = args
        .into_iter()
        .map(|arg| {
            let arg = arg.to_string_lossy().into_owned();
            if arg.contains(' ') {
                format!("\"{arg}\"")
            } else {
                arg
            }
        })
        .collect();
    format!("$ {EXE_NAME} {}", args.join(" "))
}

/// Writes `text` to the run's log, when there is one.
fn log_line(log: Option<&RunLog>, text: &str) -> io::Result<()> {
    match log {
        Some(log) => log.line(text),
        None => Ok(()),
    }
}

/// Prints `text` on stderr, and writes it to the run's log when there is one.
#[expect(clippy::print_stderr, reason = "CLI result output is the binary's job")]
fn report(log: Option<&RunLog>, text: &str) -> io::Result<()> {
    eprintln!("{text}");
    log_line(log, text)
}

/// The tool subcommand's run, past the log's opening: its exit code is the tool's own, the
/// startup failure's, or 3 when the console printer failed. Each line it prints on stderr
/// also goes to the log; an `Err` is a write to the log that failed. With
/// `check_for_updates` on, a check for a newer release runs beside the tool, and its
/// notice, when it has one in time, is the run's last line.
fn run_command(
    tools: &[Box<dyn StudioTool>],
    tool: &str,
    matches: &ArgMatches,
    log: Option<Arc<RunLog>>,
) -> io::Result<u8> {
    let (settings, paths) = match startup(tools) {
        Ok(loaded) => loaded,
        Err((code, text)) => {
            report(log.as_deref(), &text)?;
            return Ok(code);
        }
    };
    let update = start_update_check(&settings.common, Arc::new(updater::fetch_releases));
    run_loaded(tools, tool, matches, log, settings, paths, update)
}

/// `run_command` past the startup: the tool's run with `settings` and `paths` loaded, and
/// `update` the receiver of the check started beside it, `None` with checks off. The split
/// lets a test hand it a reply that has arrived, which a run of the binary cannot: it never
/// waits for one.
fn run_loaded(
    tools: &[Box<dyn StudioTool>],
    tool: &str,
    matches: &ArgMatches,
    log: Option<Arc<RunLog>>,
    settings: Settings,
    paths: AppPaths,
    update: Option<Receiver<CheckReply>>,
) -> io::Result<u8> {
    let skipped = settings.common.skipped_version.clone();
    // The CLI never writes settings, so the read-only data directory is probed rather than
    // learned from a failed save. The condition prints once, as stderr's first line
    // (ST-CLI-01).
    if let Some(dir) = &paths.data_dir
        && !settings_file_writable(dir)
    {
        report(log.as_deref(), &ShellCondition::DataDirReadOnly.text())?;
    }
    let (events_tx, events_rx) = unbounded();
    let (requests_tx, requests_rx) = unbounded();
    let printer = spawn_printer(events_rx, log.clone());
    let notices = spawn_notice_printer(requests_rx, log.clone());
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
        report(
            log.as_deref(),
            "error: the output to the console stopped unexpectedly",
        )?;
        return Ok(3);
    }
    let code = match result {
        Ok(code) => code,
        Err(error) => {
            report(log.as_deref(), &format!("error: {error}"))?;
            error.exit_code
        }
    };
    // Last, so the notice follows every other line of the run.
    report_update(update.as_ref(), skipped.as_deref(), log.as_deref())?;
    Ok(code)
}

/// Starts the update check a tool command makes when `check_for_updates` is on, and returns
/// the receiver its reply arrives on; `None` with checks off. The command never waits for
/// it: `report_update` takes whatever has arrived once the run is over.
fn start_update_check(common: &CommonSettings, fetch: Arc<Fetch>) -> Option<Receiver<CheckReply>> {
    common
        .check_for_updates
        .then(|| updater::start_check(fetch, None))
}

/// Prints the update notice when the check's reply has arrived and offers a release, on
/// stderr and in the run's log. No reply yet, a failed check and no release to offer print
/// nothing; a failed check writes a debug line. An `Err` is a write to the log that failed.
fn report_update(
    reply: Option<&Receiver<CheckReply>>,
    skipped: Option<&str>,
    log: Option<&RunLog>,
) -> io::Result<()> {
    // `try_recv`, never `recv`: a reply still on its way is dropped with the process, so the
    // check costs the run no time.
    let Some(Ok(outcome)) = reply.map(Receiver::try_recv) else {
        return Ok(());
    };
    match outcome {
        Ok(newest) => match updater::command_line_notice(newest.as_ref(), skipped) {
            Some(line) => report(log, &line),
            None => Ok(()),
        },
        Err(error) => {
            log::debug!("the update check failed: {error}");
            Ok(())
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
            return ExitCode::from(code);
        }
    };
    // The copy's slot is claimed before the log opens, so a second window logs to its
    // own `utw.n.log` and never touches the first's (ST-RUN-12): the claim's lock lives
    // in `instance` until the process ends. A copy past the slots' cap gets none, and
    // runs with no log as when the location cannot be opened.
    let instance = paths.data_dir.as_deref().and_then(instance::claim);
    // The log and the hook go in before the window: the log needs the data location the
    // startup resolved, and the hook needs the log so a panic's line lands in it.
    let installed_log = match (&paths.data_dir, &instance) {
        (Some(data_dir), Some(instance)) => install_gui_logger(data_dir, instance.slot()),
        _ => None,
    };
    install_panic_hook();
    // The monitor starts with the log, once the data location exists: a first run has neither.
    let _crash_handler = match (&installed_log, &paths.data_dir) {
        (Some(installed), Some(data_dir)) => watch_crashes(
            std::env::current_exe(),
            data_dir,
            installed.slot,
            log::logger(),
        ),
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
    // The `Result` is discarded: the call fails when the parent has no console (a launch
    // from Explorer with arguments), and output then goes where the inherited handles
    // point, which is all a GUI-subsystem exe can do.
    #[expect(
        clippy::let_underscore_must_use,
        reason = "the parent may have no console; output then goes where the inherited handles point"
    )]
    // SAFETY: `AttachConsole` takes a process id and no pointer of ours and binds no
    // memory; it only attaches this process to its parent's console.
    let _ = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
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
        Some(HiddenLaunch::Monitor {
            data_dir,
            socket,
            slot,
        }) => {
            return run_monitor(&data_dir, &socket, slot);
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
        LaunchMode::Cli { tool, matches } => {
            run_cli_mode(&tools, &tool, &matches, launch.log_file.as_deref())
        }
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

    // The window tools are the registered ones not marked CLI-only: the rail draws each from
    // the registry as a clickable entry, the Ruleset editor right after the Save editor.
    #[test]
    fn the_ruleset_editor_is_a_window_tool() {
        let window_tools: Vec<&str> = tools()
            .iter()
            .map(|tool| tool.id())
            .filter(|id| !CLI_ONLY.contains(id))
            .collect();
        assert_eq!(window_tools, ["save-editor", "ruleset-editor"]);
    }

    // The settings menu draws a section for the tools with a settings form alone: the
    // Ruleset editor's table holds only what it writes for itself, the Export upgrader's
    // nothing, and the Team compiler's form is not built yet.
    #[test]
    fn the_save_editor_alone_has_a_settings_form() {
        let mut with_form: Vec<&str> = tools()
            .iter()
            .filter(|tool| tool.has_settings_view())
            .map(|tool| tool.id())
            .collect();
        with_form.sort_unstable();
        assert_eq!(with_form, ["save-editor"]);
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

    /// The bundled file's `pes_folder_path` line.
    const BUNDLED_PES_FOLDER_LINE: &str =
        "pes_folder_path = 'C:\\Program Files (x86)\\Pro Evolution Soccer 20**'";
    /// The `pes_folder_path` line a fresh write produces: the bundled one on Windows, the
    /// empty default elsewhere (no default install path exists off Windows).
    #[cfg(windows)]
    const FRESH_PES_FOLDER_LINE: &str = BUNDLED_PES_FOLDER_LINE;
    #[cfg(not(windows))]
    const FRESH_PES_FOLDER_LINE: &str = "pes_folder_path = \"\"";

    // A settings file written fresh is the bundled file, comments included, with the
    // bundle's presets (DBC's PES 17 and `dbc7` ruleset) replaced by the code's defaults.
    // `load_settings` is the real call; a `data/` with the settings file makes its location
    // the test folder's, so the real config folder is never read.
    #[test]
    fn a_fresh_settings_file_is_the_bundled_one_with_the_code_s_defaults() {
        let dir = TestDir::new("bundled_settings");
        fs::create_dir_all(dir.path().join("data")).unwrap();
        fs::write(dir.path().join("data").join(SETTINGS_FILE_NAME), "").unwrap();
        let (settings, _) = load_settings(&tools(), dir.path().to_path_buf()).unwrap();

        let fresh = TestDir::new("bundled_settings_fresh");
        let path = fresh.path().join(SETTINGS_FILE_NAME);
        settings.save(&path).unwrap();

        let expected = SETTINGS_TEMPLATE
            .replace("pes_version = 17", "pes_version = 19")
            .replace("aatf_ruleset = \"dbc7\"", "aatf_ruleset = \"autumn26\"")
            .replace(BUNDLED_PES_FOLDER_LINE, FRESH_PES_FOLDER_LINE);
        let written = fs::read_to_string(&path).unwrap();
        for (line, (written, wanted)) in written.lines().zip(expected.lines()).enumerate() {
            assert_eq!(written, wanted, "line {}", line + 1);
        }
        assert_eq!(
            written.lines().count(),
            expected.lines().count(),
            "the line counts differ"
        );
    }

    /// A releases list whose one release is newer than any version of the program.
    const NEWER_RELEASE: &str =
        r#"[{"tag_name": "v999.0.0", "body": null, "html_url": "u", "assets": []}]"#;

    /// The check's reply for `body`, as the check's thread parses it.
    fn reply_of(body: &str) -> CheckReply {
        updater::newest_release(body)
    }

    /// A receiver holding `reply` already, its sender gone.
    fn arrived(reply: CheckReply) -> Receiver<CheckReply> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        tx.send(reply).unwrap();
        rx
    }

    /// What `report_update` writes to a fresh run's log for `reply`, with `skipped` as the
    /// settings' `skipped_version`.
    fn reported(reply: Option<&Receiver<CheckReply>>, skipped: Option<&str>) -> String {
        let dir = TestDir::new("update_report");
        let path = dir.path().join("run_log.txt");
        let log = RunLog::open(&path).unwrap();
        report_update(reply, skipped, Some(&log)).unwrap();
        fs::read_to_string(&path).unwrap()
    }

    // ST-CLI-05
    #[test]
    fn a_tool_command_starts_the_check_only_with_checks_on() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let calls = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let fetch: Arc<Fetch> = Arc::new(move || {
            counted.fetch_add(1, Ordering::SeqCst);
            Ok(NEWER_RELEASE.to_owned())
        });
        let mut common = CommonSettings {
            check_for_updates: true,
            ..CommonSettings::default()
        };
        let reply = start_update_check(&common, Arc::clone(&fetch))
            .expect("checks on: a check starts")
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the check replies");
        let newest = reply.expect("the list parses").expect("a release");
        assert_eq!(newest.version.to_string(), "999.0.0");
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        common.check_for_updates = false;
        assert!(start_update_check(&common, fetch).is_none(), "checks off");
        assert_eq!(calls.load(Ordering::SeqCst), 1, "checks off: no fetch");
    }

    // ST-CLI-05
    #[test]
    fn the_notice_is_written_when_the_reply_has_arrived_and_offers_a_release() {
        let offer = reply_of(NEWER_RELEASE);
        let line = updater::command_line_notice(offer.as_ref().unwrap().as_ref(), None)
            .expect("the premise: the release is offered");
        assert!(line.contains("v999.0.0"), "{line}");
        assert_eq!(reported(Some(&arrived(offer)), None), format!("{line}\n"));

        let skipped = arrived(reply_of(NEWER_RELEASE));
        assert_eq!(reported(Some(&skipped), Some("999.0.0")), "", "skipped");
        let failed = arrived(reply_of("not json"));
        assert_eq!(reported(Some(&failed), None), "", "failed");
        assert_eq!(reported(None, None), "", "checks off");
    }

    // ST-CLI-05
    #[test]
    fn a_reply_that_has_not_arrived_writes_nothing_and_is_not_waited_for() {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let (go_tx, go_rx) = crossbeam_channel::bounded::<()>(1);
        // The reply comes only if the report has not returned within the wait: a report
        // that waited for it would then write the line and fail the test, not hang it.
        let check = std::thread::spawn(move || {
            if go_rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .is_err()
            {
                tx.send(reply_of(NEWER_RELEASE)).unwrap();
            }
        });
        let written = reported(Some(&rx), None);
        assert_eq!(written, "");
        assert!(
            matches!(rx.try_recv(), Err(crossbeam_channel::TryRecvError::Empty)),
            "the premise: the check's sender is still alive"
        );
        go_tx.send(()).unwrap();
        check.join().unwrap();
    }

    // ST-CLI-05
    #[test]
    fn the_notice_is_the_last_line_of_a_tool_commands_run() {
        let dir = TestDir::new("update_last_line");
        fs::create_dir_all(dir.path().join("data")).unwrap();
        let tools = tools();
        let (settings, paths) = startup_in(&tools, dir.path().to_path_buf())
            .unwrap_or_else(|(_, text)| panic!("{text}"));
        // A refused command: the run has a line of its own, and reads no game folder.
        let args = [
            "utw",
            "team-compiler",
            "compile",
            "--no-deploy",
            "--mode",
            "test",
        ];
        let launch = parse_launch(&tools, args.map(OsString::from)).unwrap();
        let LaunchMode::Cli { tool, matches } = launch.mode else {
            panic!("the premise: a tool's command")
        };
        let offer = reply_of(NEWER_RELEASE);
        let notice = updater::command_line_notice(offer.as_ref().unwrap().as_ref(), None)
            .expect("the premise: the release is offered");
        let path = dir.path().join("run_log.txt");
        let log = Arc::new(RunLog::open(&path).unwrap());
        let update = Some(arrived(offer));
        let code = run_loaded(&tools, &tool, &matches, Some(log), settings, paths, update);
        assert_eq!(code.unwrap(), 2);
        let written = fs::read_to_string(&path).unwrap();
        let [error, last] = written.lines().collect::<Vec<_>>()[..] else {
            panic!("the refusal's line, then the notice: {written}")
        };
        assert!(error.starts_with("error: "), "{written}");
        assert_eq!(last, notice);
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
        assert_eq!(code, 2);
        assert!(text.starts_with("error: "), "{text}");
    }
}
