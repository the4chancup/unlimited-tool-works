//! Argument parsing for the launch modes:
//!
//! | Invocation | Mode |
//! |---|---|
//! | `UTW` | GUI, last-used tool active |
//! | `UTW <tool-id> <command> [args]` | headless CLI |
//! | `UTW --gui <tool-id> <command> [args]` | GUI autorun: the GUI opens on the tool and runs the command |
//! | `UTW help [chapter] [topic] [--export <dir>]` | prints or exports the help's Markdown |
//!
//! Every tool contributes one clap subcommand named after its id; `--gui` and `-v` are the
//! shell's own flags and `help` its own subcommand, parsed here so no tool's command knows
//! about them. `--gui help` parses as `help`: the flag means nothing for it.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::LazyLock;

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::settings::COMMON_KEY;
use crate::tool::{CliError, StudioTool, ToolContext};

/// The `--version` line — `version_label()`, the same string the status bar shows —
/// as a `&'static str` clap can take without a new feature.
static VERSION_LABEL: LazyLock<String> = LazyLock::new(crate::shell::version_label);

/// The parsed command line: the mode plus the shell's own flags.
#[derive(Debug, Clone)]
pub struct Launch {
    /// How the binary was started.
    pub mode: LaunchMode,
    /// Diagnostic verbosity: `0` warnings only, `1` (`-v`) info, `2` (`-vv`) debug, more is trace.
    /// `RUST_LOG`, when set, overrides it.
    pub verbosity: u8,
}

/// How the binary was started.
#[derive(Debug, Clone)]
pub enum LaunchMode {
    /// No arguments: open the GUI.
    Gui,
    /// A tool subcommand: run it headless.
    Cli {
        /// The tool's id.
        tool: String,
        /// The tool subcommand's parsed arguments.
        matches: ArgMatches,
    },
    /// `--gui` plus a tool subcommand: open the GUI on the tool and run the command inside it.
    GuiAutorun {
        /// The tool's id.
        tool: String,
        /// The tool subcommand's parsed arguments.
        matches: ArgMatches,
    },
    /// `help`: print or export the help's Markdown.
    Help {
        /// The chapter's id (a tool's, or a core chapter's); `None` lists the chapters.
        chapter: Option<String>,
        /// A topic slug within the chapter; `None` prints the whole chapter.
        topic: Option<String>,
        /// `--export`: write every chapter under this folder, as a Markdown file named by its
        /// id, instead of printing.
        export: Option<PathBuf>,
    },
}

/// The root `UTW` command with one subcommand per registered tool plus the shell's own
/// `help`, which replaces clap's built-in one. clap's switch is global, so the tools' own
/// generated `help` subcommands go with it: `UTW <tool-id> help` is a usage error,
/// and `UTW <tool-id> --help` prints the tool's usage as before.
///
/// # Panics
/// If two tools share an id, or a tool uses the reserved settings key `common` or the help
/// subcommand's name `help` as its id: the registry is a compile-time list, so this is a
/// programming error caught at startup.
fn root_command(tools: &[Box<dyn StudioTool>]) -> Command {
    let mut command = Command::new(crate::EXE_NAME)
        .bin_name(crate::EXE_NAME)
        .about(format!(
            "{}: the 4cc community's PES tools",
            crate::APP_NAME
        ))
        // `--version` prints the status bar's version line beside the name, the line a
        // member pastes into a report (`spec/core/distribution.md` "Versioning: one
        // workspace version, one exception"). clap wants a `&'static str`, so the
        // label is built once into a static.
        .version(VERSION_LABEL.as_str())
        .disable_help_subcommand(true)
        .arg(
            Arg::new("gui")
                .long("gui")
                .action(ArgAction::SetTrue)
                .help("Open the tool's window and run the command inside it"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .action(ArgAction::Count)
                .global(true)
                .help("More diagnostic output with each v: -v for info, -vv for debug, -vvv for trace"),
        )
        .subcommand(
            Command::new("help")
                .about("Print a help chapter, or export them all as Markdown")
                .arg(
                    Arg::new("chapter")
                        .help("The chapter's id. Without it, the chapters are listed."),
                )
                .arg(
                    Arg::new("topic")
                        .help("A topic within the chapter, named as the chapter list shows it"),
                )
                .arg(
                    Arg::new("export")
                        .long("export")
                        .value_name("dir")
                        .value_parser(clap::value_parser!(PathBuf))
                        .conflicts_with_all(["chapter", "topic"])
                        .help("Write every chapter as <id>.md into this folder"),
                ),
        );
    for (index, tool) in tools.iter().enumerate() {
        let id = tool.id();
        assert_ne!(
            id, COMMON_KEY,
            "`{COMMON_KEY}` is the common settings key, not a tool id"
        );
        assert_ne!(id, "help", "`help` is the help subcommand, not a tool id");
        assert!(
            tools[..index].iter().all(|other| other.id() != id),
            "two tools are registered with the id `{id}`"
        );
        command = command.subcommand(tool.cli_command().name(id).about(tool.label()));
    }
    command
}

/// Parses the process arguments (`args[0]` included). A clap error carries its own help or usage
/// text; the binary prints it with `Error::exit`.
pub fn parse_launch(
    tools: &[Box<dyn StudioTool>],
    args: impl IntoIterator<Item = OsString>,
) -> Result<Launch, clap::Error> {
    let matches = root_command(tools).try_get_matches_from(args)?;
    let gui = matches.get_flag("gui");
    let verbosity = matches.get_count("verbose");
    let mode = match matches.subcommand() {
        None => LaunchMode::Gui,
        // The shell's own subcommand, `--gui` or not: the flag means nothing for it.
        Some(("help", subcommand)) => LaunchMode::Help {
            chapter: subcommand.get_one::<String>("chapter").cloned(),
            topic: subcommand.get_one::<String>("topic").cloned(),
            export: subcommand.get_one::<PathBuf>("export").cloned(),
        },
        Some((tool, subcommand)) if gui => LaunchMode::GuiAutorun {
            tool: tool.to_owned(),
            matches: subcommand.clone(),
        },
        Some((tool, subcommand)) => LaunchMode::Cli {
            tool: tool.to_owned(),
            matches: subcommand.clone(),
        },
    };
    Ok(Launch { mode, verbosity })
}

/// Runs a parsed CLI subcommand on the tool that owns it and returns what the tool returned
/// (its exit code, or its refusal) unchanged. An id no tool has is refused with exit code 2.
pub fn run_cli(
    tools: &[Box<dyn StudioTool>],
    tool_id: &str,
    matches: &ArgMatches,
    ctx: &ToolContext,
) -> Result<u8, CliError> {
    let tool = tools
        .iter()
        .find(|tool| tool.id() == tool_id)
        .ok_or_else(|| {
            CliError::new(
                2,
                anyhow::anyhow!("there is no tool with the id `{tool_id}`"),
            )
        })?;
    tool.cli_run(matches, ctx)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use crossbeam_channel::unbounded;

    use super::*;
    use crate::help::HelpSection;
    use crate::settings::Settings;
    use crate::status::{Notice, NoticeKind};
    use crate::tool::{AppPaths, ShellRequest};

    struct StubTool;

    impl StudioTool for StubTool {
        fn id(&self) -> &'static str {
            "stub"
        }
        fn label(&self) -> &'static str {
            "Stub"
        }
        fn icon(&self) -> &'static str {
            ""
        }
        fn view(&mut self, _ui: &mut egui::Ui, _ctx: &ToolContext) {}
        fn default_settings(&self) -> toml::Table {
            toml::Table::new()
        }
        fn help(&self) -> HelpSection {
            HelpSection {
                title: "Stub",
                topics: Vec::new(),
                messages: Vec::new(),
            }
        }
        fn cli_command(&self) -> Command {
            Command::new("ignored-name")
                .subcommand(Command::new("ping").arg(Arg::new("what").required(true)))
                .subcommand(Command::new("verdict"))
                .subcommand(Command::new("refuse"))
        }
        fn cli_run(&self, matches: &ArgMatches, ctx: &ToolContext) -> Result<u8, CliError> {
            match matches.subcommand() {
                Some(("ping", ping)) => {
                    let what: &String = ping.get_one("what").unwrap();
                    ctx.notify(Notice {
                        text: format!("pong {what}"),
                        kind: NoticeKind::Info,
                        action: None,
                    });
                    Ok(0)
                }
                Some(("verdict", _)) => Ok(1),
                Some(("refuse", _)) => Err(CliError::new(3, anyhow::anyhow!("refused"))),
                _ => Err(CliError::new(2, anyhow::anyhow!("unknown command"))),
            }
        }
    }

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    fn tools() -> Vec<Box<dyn StudioTool>> {
        vec![Box::new(StubTool)]
    }

    fn test_context() -> (ToolContext, crossbeam_channel::Receiver<ShellRequest>) {
        let (events_tx, _events_rx) = unbounded();
        let (requests_tx, requests_rx) = unbounded();
        let paths = AppPaths {
            exe_dir: PathBuf::from("exe"),
            data_dir: None,
        };
        let ctx = ToolContext::new(
            Arc::new(Mutex::new(Settings::default())),
            paths,
            events_tx,
            requests_tx,
        );
        (ctx, requests_rx)
    }

    fn cli_matches(tools: &[Box<dyn StudioTool>], list: &[&str]) -> (String, ArgMatches) {
        let LaunchMode::Cli { tool, matches } = parse_launch(tools, args(list)).unwrap().mode
        else {
            panic!("expected CLI mode");
        };
        (tool, matches)
    }

    fn mode(list: &[&str]) -> LaunchMode {
        parse_launch(&tools(), args(list)).unwrap().mode
    }

    #[test]
    fn no_arguments_means_gui() {
        assert!(matches!(mode(&["studio"]), LaunchMode::Gui));
    }

    #[test]
    fn the_version_flag_renders_the_status_bar_s_label() {
        assert_eq!(
            root_command(&tools()).render_version(),
            format!("{} {}\n", crate::EXE_NAME, crate::shell::version_label())
        );
    }

    #[test]
    fn tool_subcommand_is_named_after_the_tool_id() {
        assert!(
            matches!(mode(&["studio", "stub", "ping", "x"]), LaunchMode::Cli { tool, .. } if tool == "stub")
        );
        assert!(parse_launch(&tools(), args(&["studio", "ignored-name", "ping", "x"])).is_err());
    }

    #[test]
    fn gui_flag_makes_it_an_autorun() {
        let launch = mode(&["studio", "--gui", "stub", "ping", "x"]);
        assert!(matches!(launch, LaunchMode::GuiAutorun { tool, .. } if tool == "stub"));
    }

    #[test]
    fn verbosity_counts_and_is_accepted_after_the_subcommand() {
        assert_eq!(
            parse_launch(&tools(), args(&["studio"])).unwrap().verbosity,
            0
        );
        assert_eq!(
            parse_launch(&tools(), args(&["studio", "-vv"]))
                .unwrap()
                .verbosity,
            2
        );
        let launch = parse_launch(&tools(), args(&["studio", "stub", "-v", "ping", "x"])).unwrap();
        assert_eq!(launch.verbosity, 1);
        assert!(matches!(launch.mode, LaunchMode::Cli { .. }));
    }

    #[test]
    #[should_panic(expected = "two tools are registered")]
    fn duplicate_tool_ids_are_a_programming_error() {
        let twice: Vec<Box<dyn StudioTool>> = vec![Box::new(StubTool), Box::new(StubTool)];
        root_command(&twice);
    }

    #[test]
    fn the_trait_defaults_refuse_a_gui_run_and_block_no_version_change() {
        let mut tool = StubTool;
        let (ctx, _requests_rx) = test_context();
        let error = tool.gui_run(&ArgMatches::default(), &ctx).unwrap_err();
        assert_eq!(error.to_string(), "stub has no GUI-runnable commands");
        assert_eq!(tool.version_change_blocker(), None);
        assert!(
            !tool.hold_close(),
            "by default nothing holds the window's close"
        );
    }

    #[test]
    fn unknown_tool_and_missing_argument_are_clap_errors() {
        assert!(parse_launch(&tools(), args(&["studio", "nope"])).is_err());
        assert!(parse_launch(&tools(), args(&["studio", "stub", "ping"])).is_err());
    }

    #[test]
    fn cli_dispatch_reaches_the_tool_with_its_context() {
        let tools = tools();
        let (tool, matches) = cli_matches(&tools, &["studio", "stub", "ping", "there"]);
        let (ctx, requests_rx) = test_context();
        assert_eq!(run_cli(&tools, &tool, &matches, &ctx).unwrap(), 0);
        assert_eq!(
            requests_rx.try_recv().unwrap(),
            ShellRequest::Notify(Notice {
                text: "pong there".to_owned(),
                kind: NoticeKind::Info,
                action: None
            })
        );
    }

    #[test]
    fn unknown_tool_id_is_refused_with_exit_code_2() {
        let tools = tools();
        let (_, matches) = cli_matches(&tools, &["studio", "stub", "verdict"]);
        let (ctx, _requests_rx) = test_context();
        let error = run_cli(&tools, "missing", &matches, &ctx).unwrap_err();
        assert_eq!(error.exit_code, 2);
        assert_eq!(error.to_string(), "there is no tool with the id `missing`");
    }

    #[test]
    fn the_tools_exit_code_and_refusal_pass_through_unchanged() {
        let tools = tools();
        let (ctx, _requests_rx) = test_context();
        let (tool, verdict) = cli_matches(&tools, &["studio", "stub", "verdict"]);
        assert_eq!(run_cli(&tools, &tool, &verdict, &ctx).unwrap(), 1);
        let (tool, refuse) = cli_matches(&tools, &["studio", "stub", "refuse"]);
        let error = run_cli(&tools, &tool, &refuse, &ctx).unwrap_err();
        assert_eq!(error.exit_code, 3);
        assert_eq!(error.to_string(), "refused");
    }

    fn help_args(list: &[&str]) -> (Option<String>, Option<String>, Option<PathBuf>) {
        let LaunchMode::Help {
            chapter,
            topic,
            export,
        } = mode(list)
        else {
            panic!("expected help mode: {list:?}");
        };
        (chapter, topic, export)
    }

    #[test]
    fn the_help_subcommand_parses_its_chapter_topic_and_export() {
        assert_eq!(help_args(&["studio", "help"]), (None, None, None));
        assert_eq!(
            help_args(&["studio", "help", "team-compiler"]),
            (Some("team-compiler".to_owned()), None, None)
        );
        assert_eq!(
            help_args(&["studio", "help", "team-compiler", "command-line"]),
            (
                Some("team-compiler".to_owned()),
                Some("command-line".to_owned()),
                None
            )
        );
        assert_eq!(
            help_args(&["studio", "help", "--export", "out"]),
            (None, None, Some(PathBuf::from("out")))
        );
    }

    #[test]
    fn the_export_flag_conflicts_with_the_positionals() {
        assert!(parse_launch(&tools(), args(&["studio", "help", "x", "--export", "d"])).is_err());
        assert!(
            parse_launch(
                &tools(),
                args(&["studio", "help", "x", "y", "--export", "d"])
            )
            .is_err()
        );
    }

    #[test]
    fn the_gui_flag_means_nothing_for_the_help_subcommand() {
        assert!(matches!(
            mode(&["studio", "--gui", "help"]),
            LaunchMode::Help { .. }
        ));
    }

    #[test]
    #[should_panic(expected = "`help` is the help subcommand")]
    fn a_tool_id_of_help_is_a_programming_error() {
        struct HelpStub;

        impl StudioTool for HelpStub {
            fn id(&self) -> &'static str {
                "help"
            }
            fn label(&self) -> &'static str {
                "Help"
            }
            fn icon(&self) -> &'static str {
                ""
            }
            fn view(&mut self, _ui: &mut egui::Ui, _ctx: &ToolContext) {}
            fn default_settings(&self) -> toml::Table {
                toml::Table::new()
            }
            fn help(&self) -> HelpSection {
                HelpSection {
                    title: "Help",
                    topics: Vec::new(),
                    messages: Vec::new(),
                }
            }
            fn cli_command(&self) -> Command {
                Command::new("help")
            }
            fn cli_run(&self, _matches: &ArgMatches, _ctx: &ToolContext) -> Result<u8, CliError> {
                Ok(0)
            }
        }

        root_command(&[Box::new(HelpStub)]);
    }
}
