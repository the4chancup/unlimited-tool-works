//! The Team compiler's command line (`spec/team_compiler/settings.md` "CLI"): a parsed command turned
//! into a run, refused before any export is read when its invocation or configuration is
//! invalid, and the run's outcome turned into the exit code scripts read.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use clap::{Args, Command, FromArgMatches, Subcommand, ValueEnum};
use pes_version::PesVersion;
use pipeline::CpkStem;
use studio_core::{
    AppPaths, CliError, CommonSettings, RunningPes, SETTINGS_FILE_NAME, Severity, ToolContext,
};
use teams_list::TeamsList;

use crate::compile::{CpkLayout, OutputMode, RunPaths, RunningGame};
use crate::messages::TOOL_ID;
use crate::output::deploy;
use crate::plan::legacy_blocks::LegacyBlocks;
use crate::reader::is_archive;
use crate::settings::{SavefilePath, TeamCompilerSettings, from_table};
use crate::{check, compile, upgrade};

// The exit codes are the command line's contract with scripts
// (`spec/team_compiler/settings.md` "CLI").
/// Exit code of a run that finished with no Error finding (warnings and notes allowed).
const CLEAN: u8 = 0;
/// Exit code of a run that finished, but some export had an Error finding.
const ERRORS: u8 = 1;
/// Exit code of an invalid invocation or configuration: nothing ran.
const INVALID: u8 = 2;
/// Exit code of a run aborted before or during its work.
const ABORTED: u8 = 3;

#[derive(Debug, Subcommand)]
enum TeamCompilerCommand {
    /// Compile the exports into a CPK.
    Compile(CompileArgs),
    /// Check the exports without writing anything.
    Check(SourceArgs),
    /// Replace the installed DpFileList with the bundled official one.
    UpgradeDpfl {
        /// Apply the change instead of only listing it.
        #[arg(long)]
        yes: bool,
    },
}

/// Which exports a run reads.
#[derive(Debug, Args)]
struct SourceArgs {
    /// The folder holding the exports, for this run only. Without it, the
    /// exports_folder_path setting's folder is used.
    exports_root: Option<PathBuf>,
    /// Only this export: a folder, a .zip or a .7z file, anywhere on disk. Repeat it to
    /// name several exports.
    #[arg(long = "export", value_name = "PATH")]
    exports: Vec<PathBuf>,
}

#[derive(Debug, Args)]
struct CompileArgs {
    #[command(flatten)]
    source: SourceArgs,
    /// Where the compiled content goes.
    #[arg(long, value_enum, default_value_t = Mode::Normal)]
    mode: Mode,
    /// Build the CPKs into the output folder without installing them.
    #[arg(long)]
    no_deploy: bool,
    /// The game's Edit mode is closed: write the savefile even while its PES runs.
    #[arg(long)]
    edit_mode_closed: bool,
}

/// The output mode of a compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Mode {
    /// CPKs, installed into the game.
    Normal,
    /// Loose files in the output folder's test_output folder.
    Test,
    /// Loose files in the game folder's livecpk folder.
    Sideload,
}

/// What the preflight resolved: everything `check` and `compile` start from.
#[derive(Debug)]
pub(crate) struct RunInputs {
    /// The tool's settings section, already checked.
    pub(crate) settings: TeamCompilerSettings,
    /// The suite-wide settings: the target PES version among them.
    pub(crate) common: CommonSettings,
    /// The FPC hide preset's boots and gloves IDs the common settings give
    /// (`fpc_hide_boots_id`/`fpc_hide_gloves_id`, refused by `read_settings`
    /// when outside their band): the `fpc_on` marker's default IDs.
    pub(crate) hide_ids: fpc::HideIds,
    /// The working teams list (the embedded one when none is installed), which maps each
    /// export's team name to its id.
    pub(crate) teams_list: TeamsList,
    /// The VGL27 old-ID blocks (the data directory's `legacy_boots_blocks.txt` when it
    /// holds one, else the embedded table), which a legacy export's recorded old IDs are
    /// checked against.
    pub(crate) legacy_blocks: LegacyBlocks,
    /// The folder scanned for exports.
    pub(crate) exports_root: PathBuf,
    /// The `--export` paths, as given; empty for every export under the root.
    pub(crate) exports: Vec<PathBuf>,
}

/// The `team-compiler` subcommand and its own subcommands.
pub(crate) fn command() -> Command {
    TeamCompilerCommand::augment_subcommands(Command::new(TOOL_ID).subcommand_required(true))
}

/// Runs a parsed `team-compiler` command: the preflight, then the command. `running` is the
/// running-PES probe `compile`'s savefile step asks: the real process list, or a test's.
pub(crate) fn run(
    matches: &clap::ArgMatches,
    ctx: &ToolContext,
    running: fn() -> Vec<RunningPes>,
) -> Result<u8, CliError> {
    let command = TeamCompilerCommand::from_arg_matches(matches)
        .map_err(|error| CliError::new(INVALID, error))?;
    match command {
        TeamCompilerCommand::Check(source) => {
            check_export_paths(&source.exports)?;
            let common = ctx.common();
            let settings = read_settings(&ctx.tool_settings(TOOL_ID), &common)?;
            let exports_root = prepare_exports_root(&source, &common, ctx.paths())?;
            let inputs =
                resolve_inputs(exports_root, source.exports, settings, common, ctx.paths())?;
            verdict(check::run(&inputs, ctx))
        }
        TeamCompilerCommand::Compile(args) => {
            let common = ctx.common();
            let mode = output_mode(args.mode, args.no_deploy, &common)?;
            check_export_paths(&args.source.exports)?;
            let settings = read_settings(&ctx.tool_settings(TOOL_ID), &common)?;
            let (layout, refs) = compile_settings(&settings, &mode)?;
            let exports_root = prepare_exports_root(&args.source, &common, ctx.paths())?;
            // A relative output folder sits beside the executable; an absolute one replaces
            // the base. A configured savefile path resolves the same way (`auto` runs no
            // discovery here: it resolves at the savefile stage).
            let paths = RunPaths {
                output_folder: ctx.paths().exe_dir.join(&settings.output_folder_path),
                savefile_path: match &settings.savefile_path {
                    SavefilePath::Auto => SavefilePath::Auto,
                    SavefilePath::Path(path) => SavefilePath::Path(ctx.paths().exe_dir.join(path)),
                },
                game: RunningGame {
                    probe: running,
                    edit_mode_closed: args.edit_mode_closed,
                },
            };
            // The output folder is probed before any export is read, so a run never does the
            // work and then fails to write it.
            deploy::prepare_output_folder(&paths.output_folder)
                .map_err(|error| CliError::new(ABORTED, error))?;
            create_teams_list(&settings.teams_list_path, ctx.paths().data_dir.as_deref());
            let inputs = resolve_inputs(
                exports_root,
                args.source.exports,
                settings,
                common,
                ctx.paths(),
            )?;
            verdict(compile::run(
                &inputs,
                &layout,
                refs.as_ref(),
                &paths,
                &mode,
                ctx,
            ))
        }
        TeamCompilerCommand::UpgradeDpfl { yes } => {
            let download = download_folder(&ctx.common())?;
            verdict(upgrade::run(&download, yes, ctx))
        }
    }
}

/// The PES folder's `download/`, which `upgrade-dpfl` works in. Refused, naming the path and
/// the setting, when it or the PES folder is not a folder.
fn download_folder(common: &CommonSettings) -> Result<PathBuf, CliError> {
    let pes_folder = common.pes_folder();
    let download = pes_folder.join(deploy::DOWNLOAD);
    let not_a_folder = if !pes_folder.is_dir() {
        pes_folder
    } else if !download.is_dir() {
        download
    } else {
        return Ok(download);
    };
    Err(invalid(anyhow!(
        "upgrade-dpfl works in the game's download folder, and {} is not a folder. Set \
         pes_folder_path to the folder PES is installed in. If it already is, create the \
         download folder in it",
        not_a_folder.display()
    )))
}

/// A run's outcome as the exit code: its worst finding's, or `ABORTED` with the error when the
/// run could not go on.
fn verdict(outcome: anyhow::Result<Option<Severity>>) -> Result<u8, CliError> {
    outcome
        .map(exit_code)
        .map_err(|error| CliError::new(ABORTED, error))
}

/// The run's exit code: 1 when some export had an Error, 3 when the run hit a Fatal finding,
/// else 0, warnings and notes included.
fn exit_code(worst: Option<Severity>) -> u8 {
    match worst {
        Some(Severity::Fatal) => ABORTED,
        Some(Severity::Error) => ERRORS,
        Some(Severity::Warning | Severity::Info) | None => CLEAN,
    }
}

fn invalid(error: anyhow::Error) -> CliError {
    CliError::new(INVALID, error)
}

/// The output mode `--mode` and `--no-deploy` ask for, or their refusal: `--no-deploy` has
/// nothing to skip in the loose-file modes, and sideload mode is refused as `sideload_mode`
/// says.
fn output_mode(
    mode: Mode,
    no_deploy: bool,
    common: &CommonSettings,
) -> Result<OutputMode, CliError> {
    if no_deploy && mode != Mode::Normal {
        return Err(invalid(anyhow!(
            "--no-deploy and --mode {} are incompatible, because only a normal compile deploys",
            mode_name(mode)
        )));
    }
    match mode {
        Mode::Normal => Ok(OutputMode::Normal { no_deploy }),
        Mode::Test => Ok(OutputMode::Test),
        Mode::Sideload => sideload_mode(common),
    }
}

/// Sideload mode for `common`'s PES version and folder: refused for PES 2015 and 2016, which
/// no sideloading runtime serves, and when the PES folder is not a folder, since `livecpk/`
/// goes in it.
fn sideload_mode(common: &CommonSettings) -> Result<OutputMode, CliError> {
    match common.pes_version {
        PesVersion::Pes15 | PesVersion::Pes16 => {
            return Err(invalid(anyhow!(
                "--mode sideload is not available for {}, because no sideloading runtime \
                 exists for it",
                common.pes_version
            )));
        }
        PesVersion::Pes17
        | PesVersion::Pes18
        | PesVersion::Pes19
        | PesVersion::Pes20
        | PesVersion::Pes21 => {}
    }
    let pes_folder = common.pes_folder();
    if !pes_folder.is_dir() {
        return Err(invalid(anyhow!(
            "--mode sideload writes into the PES folder, and {} is not a folder. Set \
             pes_folder_path to the folder PES is installed in",
            pes_folder.display()
        )));
    }
    Ok(OutputMode::Sideload { pes_folder })
}

/// The mode as `--mode` spells it.
fn mode_name(mode: Mode) -> String {
    mode.to_possible_value()
        .expect("no `Mode` variant is skipped, so each has a value on the command line")
        .get_name()
        .to_owned()
}

/// Each `--export` must be a folder or a `.zip`/`.7z` file; a relative path resolves against the
/// current directory.
fn check_export_paths(paths: &[PathBuf]) -> Result<(), CliError> {
    for path in paths {
        if path.is_dir() {
            continue;
        }
        if !path.exists() {
            return Err(invalid(anyhow!(
                "--export {} does not exist",
                path.display()
            )));
        }
        if !is_archive(path) {
            return Err(invalid(anyhow!(
                "--export {} is not a folder, a .zip or a .7z file",
                path.display()
            )));
        }
    }
    Ok(())
}

/// The tool's settings from its table, with the checks both commands share on them and on the
/// common settings.
fn read_settings(
    table: &toml::Table,
    common: &CommonSettings,
) -> Result<TeamCompilerSettings, CliError> {
    // A toml error's text ends with a newline, which would leave a blank line under `error: `.
    let settings = from_table(table).map_err(|error| {
        invalid(anyhow!(
            "the [{TOOL_ID}] settings cannot be read: {}",
            error.to_string().trim_end()
        ))
    })?;
    check_memory_cap(common.memory_cap_percent)?;
    common
        .fpc_hide_ids()
        .map_err(|error| invalid(anyhow!("{error}")))?;
    Ok(settings)
}

fn check_memory_cap(percent: f32) -> Result<(), CliError> {
    if percent > 0.0 && percent <= 100.0 {
        return Ok(());
    }
    Err(invalid(anyhow!(
        "memory_cap_percent = {percent} must be above 0 and at most 100"
    )))
}

/// The CPKs a compile in `mode` writes, from the settings only `compile` reads: one CPK named
/// `cpk_name`, or in multi-CPK mode the bins CPK and the teams parts. Beside them, the name of
/// the refs CPK, `refs_cpk_name`, which a run holding a refs export writes too. Only a normal
/// compile writes either, so test and sideload mode ignore `multicpk_mode` and have no refs CPK
/// name.
/// Refused: a name that is not a valid CPK name, naming its setting; a teams stem that is the
/// bins CPK's own; and a refs CPK name that is a CPK of the team side (`shared_with_team_side`).
/// The teams stem is not checked otherwise: a stem no slot has gives no slot, and the first team
/// then reports `cpk_slots_exhausted`.
fn compile_settings(
    settings: &TeamCompilerSettings,
    mode: &OutputMode,
) -> Result<(CpkLayout, Option<CpkStem>), CliError> {
    let name = cpk_stem("cpk_name", &settings.cpk_name)?;
    let cap = settings.cpk_part_max_size;
    match mode {
        OutputMode::Normal { .. } => {}
        OutputMode::Test | OutputMode::Sideload { .. } => {
            return Ok((CpkLayout::Single { name, cap }, None));
        }
    }
    let layout = if settings.multicpk_mode {
        parts_layout(settings, cap)?
    } else {
        CpkLayout::Single { name, cap }
    };
    let refs = cpk_stem("refs_cpk_name", &settings.refs_cpk_name)?;
    if let Some(shared) = shared_with_team_side(&refs, &layout) {
        return Err(invalid(anyhow!(
            "refs_cpk_name = \"{}\" names {shared}, but the referees need a CPK of their own",
            refs.as_str()
        )));
    }
    Ok((layout, Some(refs)))
}

/// Multi-CPK mode's CPKs: the bins CPK `bins_cpk_name`, refused when it is not a valid CPK
/// name or when the teams stem is its own, and the teams slots, `cap` bytes each.
fn parts_layout(settings: &TeamCompilerSettings, cap: u64) -> Result<CpkLayout, CliError> {
    let bins = cpk_stem("bins_cpk_name", &settings.bins_cpk_name)?;
    // A slot is every official entry of the teams stem, so a teams stem that is the bins
    // CPK's own would make the bins CPK a part too: two writers on one file.
    let bins_file = deploy::cpk_file_name(&bins);
    let bins_stem = upgrade::number_and_stem(&bins_file).map(|(_, stem)| stem);
    if bins_stem == Some(settings.teams_cpk_name.as_str()) {
        return Err(invalid(anyhow!(
            "teams_cpk_name = \"{}\" is the stem of the bins CPK, {}, but the teams parts need \
             a stem of their own",
            settings.teams_cpk_name,
            bins.as_str()
        )));
    }
    Ok(CpkLayout::Parts {
        bins,
        teams_stem: settings.teams_cpk_name.clone(),
        cap,
    })
}

/// What of `layout`'s team side the refs CPK name `refs` also names, as its refusal says it:
/// the single CPK `cpk_name`, the bins CPK, or a teams slot (any name of the teams stem, which
/// the official list may reserve); `None` when it names none of them. Letter case aside, since
/// Windows folds it: either way two writers would share one file.
fn shared_with_team_side(refs: &CpkStem, layout: &CpkLayout) -> Option<String> {
    let same = |other: &CpkStem| refs.as_str().eq_ignore_ascii_case(other.as_str());
    match layout {
        CpkLayout::Single { name, .. } => {
            same(name).then(|| format!("the CPK of cpk_name = \"{}\"", name.as_str()))
        }
        CpkLayout::Parts {
            bins, teams_stem, ..
        } => {
            if same(bins) {
                return Some(format!("the CPK of bins_cpk_name = \"{}\"", bins.as_str()));
            }
            let refs_file = deploy::cpk_file_name(refs);
            let (_, stem) = upgrade::number_and_stem(&refs_file)?;
            stem.eq_ignore_ascii_case(teams_stem)
                .then(|| format!("a slot of teams_cpk_name = \"{teams_stem}\""))
        }
    }
}

/// The setting `key`'s `value` as a CPK name, or its refusal naming the setting.
fn cpk_stem(key: &str, value: &str) -> Result<CpkStem, CliError> {
    CpkStem::new(value).map_err(|error| {
        invalid(anyhow!(
            "{key} = \"{value}\" is not a valid CPK name ({error})"
        ))
    })
}

/// The run's inputs over the ready `exports_root` and the `--export` paths: the teams list is
/// loaded here, last, so a refused root never reads it.
fn resolve_inputs(
    exports_root: PathBuf,
    exports: Vec<PathBuf>,
    settings: TeamCompilerSettings,
    common: CommonSettings,
    paths: &AppPaths,
) -> Result<RunInputs, CliError> {
    let teams_list = load_teams_list(&settings.teams_list_path, paths.data_dir.as_deref())?;
    let legacy_blocks = load_legacy_blocks(paths.data_dir.as_deref())?;
    Ok(RunInputs {
        settings,
        hide_ids: common
            .fpc_hide_ids()
            .expect("read_settings refuses an out-of-band hide ID"),
        common,
        teams_list,
        legacy_blocks,
        exports_root,
        exports,
    })
}

/// The teams list at `setting` (relative: in the data directory). With no data directory yet or
/// no file there, the embedded list, nothing written; a file that cannot be read or parsed aborts.
fn load_teams_list(setting: &Path, data_dir: Option<&Path>) -> Result<TeamsList, CliError> {
    let path = if setting.is_absolute() {
        setting.to_path_buf()
    } else if let Some(dir) = data_dir {
        dir.join(setting)
    } else {
        return Ok(embedded_teams_list());
    };
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(embedded_teams_list()),
        Err(error) => {
            return Err(CliError::new(
                ABORTED,
                anyhow!("{}: cannot read the teams list ({error})", path.display()),
            ));
        }
    };
    TeamsList::parse(&text).map_err(|error| {
        CliError::new(
            ABORTED,
            anyhow!("{} is not a valid teams list ({error})", path.display()),
        )
    })
}

/// `compile` gives the member a teams list to edit: with a data directory and no file at the
/// `teams_list_path` setting's path, it writes the embedded list there (`check` never writes
/// it). The write is best-effort, a failure detected by attempting it: a data directory that
/// cannot take it leaves the embedded list to be read with a warning naming the path, never a
/// failure.
fn create_teams_list(setting: &Path, data_dir: Option<&Path>) {
    let Some(data_dir) = data_dir else {
        return;
    };
    // An absolute setting replaces the data directory in the join, as `load_teams_list` reads it.
    let path = data_dir.join(setting);
    if path.exists() {
        return;
    }
    // The bytes are written unchanged: the file's CRLF bytes are the upstream list's.
    if let Err(error) = crate::paths::replace_file(&path, TeamsList::UPSTREAM.as_bytes()) {
        log::warn!(
            "{}: the teams list cannot be written ({error}); the embedded list is used",
            path.display()
        );
    }
}

fn embedded_teams_list() -> TeamsList {
    TeamsList::parse(TeamsList::UPSTREAM)
        .expect("the embedded teams list parses; the teams_list crate's tests check it")
}

/// The block table file's fixed name in the data directory: no setting, as the file is
/// the VGL27 stopgap the `legacy_ids.toml` one is.
const LEGACY_BLOCKS_FILE: &str = "legacy_boots_blocks.txt";

/// The VGL27 block table at `LEGACY_BLOCKS_FILE` in the data directory when present,
/// else the embedded copy; a file that cannot be read or parsed aborts, as the teams
/// list's does.
fn load_legacy_blocks(data_dir: Option<&Path>) -> Result<LegacyBlocks, CliError> {
    let Some(dir) = data_dir else {
        return Ok(LegacyBlocks::embedded());
    };
    let path = dir.join(LEGACY_BLOCKS_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(LegacyBlocks::embedded());
        }
        Err(error) => {
            return Err(CliError::new(
                ABORTED,
                anyhow!("{}: cannot read the block table ({error})", path.display()),
            ));
        }
    };
    LegacyBlocks::parse(&text).map_err(|error| {
        CliError::new(
            ABORTED,
            anyhow!("{} is not a valid block table ({error})", path.display()),
        )
    })
}

/// `compile` gives the member the VGL27 block table to edit: with a data directory and
/// no file there, it writes the embedded copy when the run compiles an export whose
/// `legacy_ids.toml` parsed (`check` never writes it). Best-effort, as
/// `create_teams_list` is: a failure leaves the embedded table to be read with a warning
/// naming the path, never a failure.
pub(crate) fn create_legacy_blocks(data_dir: Option<&Path>) {
    let Some(data_dir) = data_dir else {
        return;
    };
    let path = data_dir.join(LEGACY_BLOCKS_FILE);
    if path.exists() {
        return;
    }
    if let Err(error) = crate::paths::replace_file(&path, LegacyBlocks::EMBEDDED.as_bytes()) {
        log::warn!(
            "{}: the block table cannot be written ({error}); the embedded table is used",
            path.display()
        );
    }
}

/// The exports root: the command line's (relative to the current directory), else the
/// `exports_folder_path` setting (relative: beside the executable).
fn exports_root(argument: Option<&Path>, common: &CommonSettings, exe_dir: &Path) -> PathBuf {
    match argument {
        Some(root) => root.to_path_buf(),
        None => exe_dir.join(&common.exports_folder_path),
    }
}

/// The exports root, ready for the run's scan. The relative setting's folder is created when
/// missing, so a fresh install has one to put exports in. A folder named any other way (an
/// absolute setting, the command line's root) is never created, and a missing one is a
/// configuration error naming the path and what to do. With `--export` paths the root is not
/// scanned, so it is neither created nor checked.
fn prepare_exports_root(
    source: &SourceArgs,
    common: &CommonSettings,
    paths: &AppPaths,
) -> Result<PathBuf, CliError> {
    let root = exports_root(source.exports_root.as_deref(), common, &paths.exe_dir);
    if !source.exports.is_empty() || root.is_dir() {
        return Ok(root);
    }
    if source.exports_root.is_some() {
        return Err(invalid(anyhow!(
            "the exports folder {} given on the command line does not exist. Check the path, \
             or leave it out to use exports_folder_path",
            root.display()
        )));
    }
    if common.exports_folder_path.is_absolute() {
        // With no data directory there is no settings file, so the setting is the built-in
        // relative default and this arm is not reached in practice.
        let settings_file = match &paths.data_dir {
            Some(data_dir) => data_dir.join(SETTINGS_FILE_NAME).display().to_string(),
            None => "the settings file".to_owned(),
        };
        return Err(invalid(anyhow!(
            "the exports folder {} does not exist. Create it and put your exports inside, or \
             set exports_folder_path in {settings_file} to the folder that holds them",
            root.display()
        )));
    }
    fs::create_dir_all(&root).map_err(|error| {
        CliError::new(
            ABORTED,
            anyhow!(
                "{}: cannot create the exports folder ({error})",
                root.display()
            ),
        )
    })?;
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::savefile::EDIT_MODE_CLOSED_FLAG;
    use crate::testing::scratch;

    fn parse(args: &[&str]) -> TeamCompilerCommand {
        let matches = command()
            .try_get_matches_from(std::iter::once("team-compiler").chain(args.iter().copied()))
            .unwrap();
        TeamCompilerCommand::from_arg_matches(&matches).unwrap()
    }

    #[test]
    fn compile_defaults_to_normal_mode_and_collects_every_export() {
        let TeamCompilerCommand::Compile(args) =
            parse(&["compile", "root", "--export", "a", "--export", "b.zip"])
        else {
            panic!("not compile");
        };
        assert_eq!(args.mode, Mode::Normal);
        assert!(!args.no_deploy);
        assert!(!args.edit_mode_closed);
        assert_eq!(args.source.exports_root, Some(PathBuf::from("root")));
        assert_eq!(
            args.source.exports,
            [PathBuf::from("a"), PathBuf::from("b.zip")]
        );
    }

    // `--edit-mode-closed` is `compile`'s alone: `check` writes no save for it to open.
    #[test]
    fn only_compile_takes_the_edit_mode_closed_flag() {
        let TeamCompilerCommand::Compile(args) = parse(&["compile", EDIT_MODE_CLOSED_FLAG]) else {
            panic!("not compile");
        };
        assert!(args.edit_mode_closed);
        assert!(
            command()
                .try_get_matches_from(["team-compiler", "check", EDIT_MODE_CLOSED_FLAG])
                .is_err()
        );
    }

    #[test]
    fn a_command_is_required_and_check_takes_no_mode() {
        assert!(command().try_get_matches_from(["team-compiler"]).is_err());
        assert!(
            command()
                .try_get_matches_from(["team-compiler", "check", "--mode", "normal"])
                .is_err()
        );
    }

    #[test]
    fn the_worst_severity_decides_the_exit_code() {
        assert_eq!(exit_code(None), 0);
        assert_eq!(exit_code(Some(Severity::Info)), 0);
        assert_eq!(exit_code(Some(Severity::Warning)), 0);
        assert_eq!(exit_code(Some(Severity::Error)), 1);
        assert_eq!(exit_code(Some(Severity::Fatal)), 3);
    }

    #[test]
    fn a_run_that_cannot_go_on_is_aborted_with_its_error() {
        assert_eq!(verdict(Ok(Some(Severity::Error))).unwrap(), 1);
        let error = verdict(Err(anyhow!("the disk is full"))).unwrap_err();
        assert_eq!(error.exit_code, 3);
        assert_eq!(error.to_string(), "the disk is full");
    }

    #[test]
    fn modes_are_named_as_the_command_line_spells_them() {
        assert_eq!(mode_name(Mode::Normal), "normal");
        assert_eq!(mode_name(Mode::Test), "test");
        assert_eq!(mode_name(Mode::Sideload), "sideload");
    }

    #[test]
    fn no_deploy_is_refused_with_either_loose_file_mode_and_kept_with_normal() {
        let common = CommonSettings::default();
        for no_deploy in [true, false] {
            assert_eq!(
                output_mode(Mode::Normal, no_deploy, &common).unwrap(),
                OutputMode::Normal { no_deploy }
            );
        }
        for mode in [Mode::Test, Mode::Sideload] {
            let error = output_mode(mode, true, &common).unwrap_err();
            assert_eq!(error.exit_code, INVALID);
            assert!(error.to_string().contains("incompatible"), "{error}");
        }
        assert_eq!(
            output_mode(Mode::Test, false, &common).unwrap(),
            OutputMode::Test
        );
    }

    #[test]
    fn sideload_needs_a_runtime_for_the_version_and_a_pes_folder() {
        let temp = scratch("cli_sideload_mode");
        let pes_folder = temp.path().join("PES");
        fs::create_dir_all(&pes_folder).unwrap();
        let common = |pes_version, folder: &Path| CommonSettings {
            pes_version,
            pes_folder_path: folder.to_str().unwrap().to_owned(),
            ..CommonSettings::default()
        };
        for version in [PesVersion::Pes15, PesVersion::Pes16] {
            let error =
                output_mode(Mode::Sideload, false, &common(version, &pes_folder)).unwrap_err();
            assert_eq!(error.exit_code, INVALID);
            assert_eq!(
                error.to_string(),
                format!(
                    "--mode sideload is not available for {version}, because no sideloading \
                     runtime exists for it"
                )
            );
        }
        for version in [PesVersion::Pes17, PesVersion::Pes21] {
            assert_eq!(
                output_mode(Mode::Sideload, false, &common(version, &pes_folder)).unwrap(),
                OutputMode::Sideload {
                    pes_folder: pes_folder.clone()
                }
            );
        }
        // A file is no folder to put `livecpk/` in, and neither is nothing.
        fs::write(temp.path().join("file"), "").unwrap();
        for refused in [temp.path().join("file"), temp.path().join("missing")] {
            let error = output_mode(Mode::Sideload, false, &common(PesVersion::Pes21, &refused))
                .unwrap_err();
            assert_eq!(error.exit_code, INVALID);
            assert_eq!(
                error.to_string(),
                format!(
                    "--mode sideload writes into the PES folder, and {} is not a folder. Set \
                     pes_folder_path to the folder PES is installed in",
                    refused.display()
                )
            );
        }
    }

    #[test]
    fn export_paths_accept_folders_and_archives_only() {
        let temp = scratch("cli_export_paths");
        let root = temp.path();
        fs::write(root.join("pack.7z"), "").unwrap();
        fs::write(root.join("notes.txt"), "").unwrap();
        assert!(check_export_paths(&[root.to_path_buf(), root.join("pack.7z")]).is_ok());
        for refused in [root.join("notes.txt"), root.join("missing.zip")] {
            let error = check_export_paths(std::slice::from_ref(&refused)).unwrap_err();
            assert_eq!(error.exit_code, INVALID);
            assert!(error.to_string().contains(&refused.display().to_string()));
        }
    }

    #[test]
    fn the_memory_cap_must_be_above_zero_and_at_most_a_hundred() {
        assert!(check_memory_cap(100.0).is_ok());
        assert!(check_memory_cap(0.5).is_ok());
        for refused in [0.0, -1.0, 100.5, f32::NAN] {
            let error = check_memory_cap(refused).unwrap_err();
            assert_eq!(error.exit_code, INVALID);
            assert!(error.to_string().contains("memory_cap_percent"));
        }
    }

    #[test]
    fn settings_of_the_wrong_type_or_a_memory_cap_out_of_range_are_refused() {
        let table: toml::Table = toml::from_str("pass_through = true").unwrap();
        let settings = read_settings(&table, &CommonSettings::default()).unwrap();
        assert!(settings.pass_through);

        let table: toml::Table = toml::from_str("pass_through = 1").unwrap();
        let error = read_settings(&table, &CommonSettings::default()).unwrap_err();
        assert_eq!(error.exit_code, INVALID);
        assert!(error.to_string().contains("pass_through"), "{error}");

        let common = CommonSettings {
            memory_cap_percent: 0.0,
            ..CommonSettings::default()
        };
        let error = read_settings(&toml::Table::new(), &common).unwrap_err();
        assert!(error.to_string().contains("memory_cap_percent"), "{error}");
    }

    #[test]
    fn compile_writes_one_cpk_or_with_multicpk_mode_the_parts() {
        let no_deploy = OutputMode::Normal { no_deploy: true };
        let deploys = OutputMode::Normal { no_deploy: false };
        let single = CpkLayout::Single {
            name: CpkStem::new("4cc_99_test").unwrap(),
            cap: 3 << 30,
        };
        let refs = CpkStem::new("4cc_18_referees").unwrap();
        let mut settings = TeamCompilerSettings::default();
        for mode in [&deploys, &no_deploy] {
            let (layout, refs_name) = compile_settings(&settings, mode).unwrap();
            assert_eq!(layout, single, "{mode:?}");
            assert_eq!(refs_name.as_ref(), Some(&refs), "{mode:?}");
        }

        settings.multicpk_mode = true;
        let parts = CpkLayout::Parts {
            bins: CpkStem::new("4cc_08_bins").unwrap(),
            teams_stem: "teams".to_owned(),
            cap: 3 << 30,
        };
        // Deploying or not.
        for mode in [&deploys, &no_deploy] {
            let (layout, refs_name) = compile_settings(&settings, mode).unwrap();
            assert_eq!(layout, parts, "{mode:?}");
            assert_eq!(refs_name.as_ref(), Some(&refs), "{mode:?}");
        }
        // Only a normal compile splits or writes a refs CPK.
        let sideload = OutputMode::Sideload {
            pes_folder: PathBuf::from("PES"),
        };
        for multicpk_mode in [false, true] {
            settings.multicpk_mode = multicpk_mode;
            for mode in [&OutputMode::Test, &sideload] {
                let (layout, refs_name) = compile_settings(&settings, mode).unwrap();
                assert_eq!(layout, single, "{mode:?}");
                assert_eq!(refs_name, None, "{mode:?}");
            }
        }
    }

    #[test]
    fn compile_refuses_a_refs_cpk_name_that_is_a_cpk_of_the_team_side_naming_both_settings() {
        let no_deploy = OutputMode::Normal { no_deploy: true };
        let refused = |settings: &TeamCompilerSettings| {
            let error = compile_settings(settings, &no_deploy).unwrap_err();
            assert_eq!(error.exit_code, INVALID);
            error.to_string()
        };
        let mut settings = TeamCompilerSettings {
            refs_cpk_name: "con".to_owned(),
            ..TeamCompilerSettings::default()
        };
        assert!(
            refused(&settings).starts_with("refs_cpk_name = \"con\" is not a valid CPK name"),
            "{settings:?}"
        );

        settings.refs_cpk_name = "4cc_99_TEST".to_owned();
        assert_eq!(
            refused(&settings),
            "refs_cpk_name = \"4cc_99_TEST\" names the CPK of cpk_name = \"4cc_99_test\", but \
             the referees need a CPK of their own"
        );
        // Not written in test mode, and not a CPK of a multi-CPK run.
        assert!(compile_settings(&settings, &OutputMode::Test).is_ok());
        settings.multicpk_mode = true;
        assert!(compile_settings(&settings, &no_deploy).is_ok());

        settings.refs_cpk_name = "4cc_08_Bins".to_owned();
        assert_eq!(
            refused(&settings),
            "refs_cpk_name = \"4cc_08_Bins\" names the CPK of bins_cpk_name = \"4cc_08_bins\", \
             but the referees need a CPK of their own"
        );
        settings.refs_cpk_name = "4cc_46_TEAMS".to_owned();
        assert_eq!(
            refused(&settings),
            "refs_cpk_name = \"4cc_46_TEAMS\" names a slot of teams_cpk_name = \"teams\", but \
             the referees need a CPK of their own"
        );
        // A stem that only starts like the teams stem names no slot of it.
        settings.refs_cpk_name = "4cc_46_teams2".to_owned();
        assert!(compile_settings(&settings, &no_deploy).is_ok());
        // Nor does the bins CPK's name in single-CPK mode.
        settings.multicpk_mode = false;
        settings.refs_cpk_name = "4cc_08_bins".to_owned();
        assert!(compile_settings(&settings, &no_deploy).is_ok());
    }

    #[test]
    fn compile_refuses_an_invalid_cpk_name_or_bins_cpk_name_naming_it() {
        let no_deploy = OutputMode::Normal { no_deploy: true };
        let mut settings = TeamCompilerSettings {
            cpk_name: "con".to_owned(),
            ..TeamCompilerSettings::default()
        };
        let error = compile_settings(&settings, &no_deploy).unwrap_err();
        assert_eq!(error.exit_code, INVALID);
        assert!(error.to_string().contains("cpk_name = \"con\""), "{error}");

        // The bins CPK's name counts only in multi-CPK mode.
        settings.cpk_name = "4cc_99_test".to_owned();
        settings.bins_cpk_name = "bins.cpk".to_owned();
        assert!(compile_settings(&settings, &no_deploy).is_ok());
        settings.multicpk_mode = true;
        let error = compile_settings(&settings, &no_deploy).unwrap_err();
        assert_eq!(error.exit_code, INVALID);
        assert!(
            error
                .to_string()
                .starts_with("bins_cpk_name = \"bins.cpk\""),
            "{error}"
        );
        // The teams stem is not checked: a stem no slot has gives no slot.
        settings.bins_cpk_name = "4cc_08_bins".to_owned();
        settings.teams_cpk_name = "no such/stem".to_owned();
        assert!(compile_settings(&settings, &no_deploy).is_ok());
        // Except the bins CPK's own stem, which would make it a teams part too.
        settings.teams_cpk_name = "bins".to_owned();
        let error = compile_settings(&settings, &no_deploy).unwrap_err();
        assert_eq!(error.exit_code, INVALID);
        assert_eq!(
            error.to_string(),
            "teams_cpk_name = \"bins\" is the stem of the bins CPK, 4cc_08_bins, but the teams \
             parts need a stem of their own"
        );
    }

    #[test]
    fn the_teams_list_falls_back_to_the_embedded_one_without_a_file() {
        let embedded = embedded_teams_list();
        let temp = scratch("cli_teams_list");
        let root = temp.path();
        let relative = Path::new("teams_list.txt");
        assert_eq!(load_teams_list(relative, None).unwrap(), embedded);
        assert_eq!(load_teams_list(relative, Some(root)).unwrap(), embedded);
        assert!(!root.join("teams_list.txt").exists(), "nothing written");

        fs::write(root.join("teams_list.txt"), "ID\tName\n792\t/egg/\n").unwrap();
        let own = load_teams_list(relative, Some(root)).unwrap();
        assert_eq!(own.teams().count(), 1);
        // An absolute path is read even with no data directory.
        let absolute = root.join("teams_list.txt");
        assert_eq!(load_teams_list(&absolute, None).unwrap(), own);

        fs::write(root.join("teams_list.txt"), "not a teams list").unwrap();
        let error = load_teams_list(relative, Some(root)).unwrap_err();
        assert_eq!(error.exit_code, ABORTED);
        assert!(error.to_string().contains("teams_list.txt"), "{error}");

        // A path that exists but cannot be read as a file aborts too.
        fs::remove_file(root.join("teams_list.txt")).unwrap();
        fs::create_dir(root.join("teams_list.txt")).unwrap();
        let error = load_teams_list(relative, Some(root)).unwrap_err();
        assert_eq!(error.exit_code, ABORTED);
        assert!(error.to_string().contains("cannot read"), "{error}");
    }

    #[test]
    fn a_missing_teams_list_is_created_only_with_a_data_directory() {
        let temp = scratch("cli_create_teams_list");
        let root = temp.path();
        let file = root.join("teams_list.txt");
        create_teams_list(&file, None);
        assert!(!file.exists(), "no data directory, nothing written");

        create_teams_list(Path::new("teams_list.txt"), Some(root));
        assert_eq!(fs::read(&file).unwrap(), TeamsList::UPSTREAM.as_bytes());

        let own = "ID\tName\n792\t/egg/\n";
        fs::write(&file, own).unwrap();
        create_teams_list(Path::new("teams_list.txt"), Some(root));
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            own,
            "an existing file is kept"
        );
    }

    /// The warning lines `log` has recorded since the recorder was installed.
    fn warnings() -> Vec<String> {
        static LINES: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        struct Recorder;
        impl log::Log for Recorder {
            fn enabled(&self, _: &log::Metadata) -> bool {
                true
            }
            fn log(&self, record: &log::Record) {
                LINES
                    .lock()
                    .unwrap()
                    .push(format!("{} {}", record.level(), record.args()));
            }
            fn flush(&self) {}
        }
        static RECORDER: Recorder = Recorder;
        log::set_logger(&RECORDER).unwrap_or_default();
        log::set_max_level(log::LevelFilter::Warn);
        LINES.lock().unwrap().clone()
    }

    #[test]
    fn an_unwritable_teams_list_path_warns_and_the_embedded_list_stands() {
        let temp = scratch("cli_teams_list_unwritable");
        let root = temp.path();
        // The write is best-effort: it cannot be made, a warning names it, and the run reads
        // the embedded list exactly as it does with no data directory. The recorder must be
        // installed before the write, or the warning is gone before anyone hears it.
        warnings();
        create_teams_list(Path::new("no folder/teams_list.txt"), Some(root));
        assert_eq!(
            load_teams_list(Path::new("no folder/teams_list.txt"), Some(root)).unwrap(),
            embedded_teams_list(),
        );
        assert!(
            warnings().iter().any(|line| line.starts_with("WARN")
                && line.contains("no folder")
                && line.contains("embedded")),
            "{:?}",
            warnings()
        );
    }

    #[test]
    fn the_exports_root_is_the_argument_or_the_setting_beside_the_executable() {
        let common = CommonSettings::default();
        let exe_dir = Path::new("exe_dir");
        assert_eq!(
            exports_root(Some(Path::new("given")), &common, exe_dir),
            PathBuf::from("given")
        );
        assert_eq!(
            exports_root(None, &common, exe_dir),
            exe_dir.join("exports")
        );
    }

    fn app_paths(exe_dir: &Path, data_dir: Option<&Path>) -> AppPaths {
        AppPaths {
            exe_dir: exe_dir.to_path_buf(),
            data_dir: data_dir.map(Path::to_path_buf),
        }
    }

    #[test]
    fn a_missing_root_given_on_the_command_line_is_refused_and_not_created() {
        let temp = scratch("cli_root_command_line");
        let root = temp.path();
        let given = root.join("given");
        let source = SourceArgs {
            exports_root: Some(given.clone()),
            exports: vec![],
        };
        let error =
            prepare_exports_root(&source, &CommonSettings::default(), &app_paths(root, None))
                .unwrap_err();
        assert_eq!(error.exit_code, INVALID);
        assert_eq!(
            error.to_string(),
            format!(
                "the exports folder {} given on the command line does not exist. Check the \
                 path, or leave it out to use exports_folder_path",
                given.display()
            )
        );
        assert!(!given.exists(), "not created");
    }

    #[test]
    fn with_export_paths_a_missing_root_is_neither_created_nor_refused() {
        let temp = scratch("cli_root_named_exports");
        let root = temp.path();
        let source = SourceArgs {
            exports_root: None,
            exports: vec![root.join("co Midcup A")],
        };
        let ready =
            prepare_exports_root(&source, &CommonSettings::default(), &app_paths(root, None))
                .unwrap();
        assert_eq!(ready, root.join("exports"));
        assert!(!ready.exists(), "not created");
    }

    #[test]
    fn an_absolute_setting_naming_a_missing_folder_is_refused_naming_the_settings_file() {
        let temp = scratch("cli_root_absolute");
        let root = temp.path();
        let missing = root.join("elsewhere");
        let common = CommonSettings {
            exports_folder_path: missing.clone(),
            ..CommonSettings::default()
        };
        let source = SourceArgs {
            exports_root: None,
            exports: vec![],
        };
        let data_dir = root.join("data");
        let error =
            prepare_exports_root(&source, &common, &app_paths(root, Some(&data_dir))).unwrap_err();
        assert_eq!(error.exit_code, INVALID);
        assert_eq!(
            error.to_string(),
            format!(
                "the exports folder {} does not exist. Create it and put your exports inside, \
                 or set exports_folder_path in {} to the folder that holds them",
                missing.display(),
                data_dir.join("settings.toml").display()
            )
        );
        // Without a data directory the sentence still reads, naming no file.
        let error = prepare_exports_root(&source, &common, &app_paths(root, None)).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("exports_folder_path in the settings file to"),
            "{error}"
        );
        assert!(!missing.exists(), "not created");
    }

    #[test]
    fn the_relative_default_is_created_and_a_folder_that_cannot_be_aborts() {
        let temp = scratch("cli_root_created");
        let root = temp.path();
        let source = SourceArgs {
            exports_root: None,
            exports: vec![],
        };
        let common = CommonSettings::default();
        let ready = prepare_exports_root(&source, &common, &app_paths(root, None)).unwrap();
        assert_eq!(ready, root.join("exports"));
        assert!(ready.is_dir(), "created");
        // Already there: returned as it is.
        assert_eq!(
            prepare_exports_root(&source, &common, &app_paths(root, None)).unwrap(),
            ready
        );

        // A file where the executable's folder should be: the folder cannot be created.
        let blocker = root.join("blocker");
        fs::write(&blocker, "").unwrap();
        let error = prepare_exports_root(&source, &common, &app_paths(&blocker, None)).unwrap_err();
        assert_eq!(error.exit_code, ABORTED);
        let text = error.to_string();
        assert!(
            text.starts_with(&format!(
                "{}: cannot create the exports folder (",
                blocker.join("exports").display()
            )),
            "{text}"
        );
    }
}
