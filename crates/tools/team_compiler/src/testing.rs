//! What the crate's unit tests share.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use aesthetics_export::{
    CanonicalListing, ListedEntry, ListedKind, ResolvedAestheticsExport, SmallMetadata,
    ValidationContext, parse_listing,
};
use crossbeam_channel::{Receiver, unbounded};
use pes_version::PesVersion;
use studio_core::{AppPaths, ExportId, PipelineEventEnvelope, Settings, ToolContext};
use teams_list::TeamsList;

use crate::bins::Rgb;
use crate::events::RunEvents;
use crate::messages::TOOL_ID;
use crate::plan::ExportToPlan;
use crate::settings::default_table;
use crate::templates::Templates;

/// A test's temporary folder, removed when the guard drops (a test that panics included).
pub(crate) struct ScratchFolder {
    path: PathBuf,
}

impl ScratchFolder {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchFolder {
    fn drop(&mut self) {
        // Best effort: the cleanup must not mask the test's result (a panic here during a failed
        // test's unwind would abort the whole test process), so a folder that cannot be removed
        // (a handle still open on Windows) is only logged.
        if let Err(error) = fs::remove_dir_all(&self.path) {
            log::debug!("{}: not removed: {error}", self.path.display());
        }
    }
}

/// A fresh, empty `<temp>/team_compiler_<name>_<pid>` folder; `name` is unique per test.
pub(crate) fn scratch(name: &str) -> ScratchFolder {
    let path = std::env::temp_dir().join(format!("team_compiler_{name}_{}", std::process::id()));
    if path.exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    fs::create_dir_all(&path).unwrap();
    ScratchFolder { path }
}

/// Copies every file under `source` into `target`, folders created as needed.
fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let into = target.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &into);
        } else {
            fs::copy(entry.path(), into).unwrap();
        }
    }
}

/// A fresh `scratch` folder standing in for the executable's: a teams list holding `792 /egg/`
/// under `data/` and the tracer bullet's export under `exports/egg Midcup Tracer`, so a `compile` with
/// no arguments compiles it to `output/4cc_99_test.cpk`.
pub(crate) fn sandbox(name: &str) -> ScratchFolder {
    let temp = scratch(name);
    let root = temp.path();
    fs::create_dir_all(root.join("data")).unwrap();
    fs::write(root.join("data/teams_list.txt"), "ID\tName\n792\t/egg/\n").unwrap();
    let tracer = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tracer/studio/egg Midcup Tracer");
    copy_tree(&tracer, &root.join("exports/egg Midcup Tracer"));
    temp
}

/// A `DpFileList.bin` listing `names`, in the measured layout.
pub(crate) fn dpfilelist(names: &[&str]) -> Vec<u8> {
    let count = u32::try_from(names.len()).unwrap();
    let mut bytes = vec![0; 4];
    bytes.extend(count.to_le_bytes());
    bytes.extend([0; 8]);
    for name in names {
        let mut record = [0; 48];
        record[..name.len()].copy_from_slice(name.as_bytes());
        bytes.extend(record);
    }
    bytes
}

/// Makes `root/PES`, `tool_context`'s PES folder, a PES 2021 install a compile can deploy into,
/// as `upgrade-dpfl` leaves it: `PES2021.exe`, a `download/DpFileList.bin` listing the official
/// list's entries, and the placeholder CPK for each of them but the run's `4cc_99_test.cpk`. A
/// compile then reports nothing about the list, and its bins are still the bundled ones, since
/// the placeholders hold none.
pub(crate) fn install_pes(root: &Path) {
    let download = root.join("PES/download");
    fs::create_dir_all(&download).unwrap();
    fs::write(root.join("PES/PES2021.exe"), "the game").unwrap();
    let official = Templates::embedded().official_list();
    let names: Vec<&str> = official.iter().map(String::as_str).collect();
    fs::write(download.join("DpFileList.bin"), dpfilelist(&names)).unwrap();
    let placeholder = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resources/templates/placeholder.cpk"),
    )
    .unwrap();
    for name in names {
        if name != "4cc_99_test.cpk" {
            fs::write(download.join(name), &placeholder).unwrap();
        }
    }
}

/// A context whose executable folder is `root` (data directory `root/data`), over PES 21
/// settings with the PES folder at `root/PES` plus `extra` settings lines, the tool's defaults
/// merged in. Its shell channels have no consumer. `savefile_path` names a file under `root`
/// rather than the `auto` default, whose discovery would find the developer's own save.
pub(crate) fn tool_context(root: &Path, extra: &str) -> ToolContext {
    let text = format!(
        "[common]\npes_version = 21\npes_folder_path = '{}'\n{extra}",
        root.join("PES").display()
    );
    let mut settings = Settings::parse(&text).unwrap();
    // Before `merge_defaults` fills `savefile_path` with `auto`: an existing key is kept.
    let mut tool = settings.tool(TOOL_ID).cloned().unwrap_or_default();
    tool.entry("savefile_path").or_insert_with(|| {
        toml::Value::String(root.join("save").join("EDIT00000000").display().to_string())
    });
    settings.set_tool(TOOL_ID, tool);
    settings.merge_defaults(TOOL_ID, &default_table());
    let (events_tx, _events_rx) = unbounded();
    let (requests_tx, _requests_rx) = unbounded();
    ToolContext::new(
        Arc::new(Mutex::new(settings)),
        AppPaths {
            exe_dir: root.to_path_buf(),
            data_dir: Some(root.join("data")),
        },
        events_tx,
        requests_tx,
    )
}

/// Run events over a context of their own, timed as a `--timestamps` run's when `timestamps`,
/// and the receiver that reads every event they send.
pub(crate) fn run_events(timestamps: bool) -> (RunEvents, Receiver<PipelineEventEnvelope>) {
    let (events_tx, events_rx) = unbounded();
    let (requests_tx, _requests_rx) = unbounded();
    let ctx = ToolContext::new(
        Arc::new(Mutex::new(Settings::parse("").unwrap())),
        AppPaths {
            exe_dir: PathBuf::new(),
            data_dir: None,
        },
        events_tx,
        requests_tx,
    );
    (RunEvents::new(&ctx, timestamps), events_rx)
}

/// Calls `poll`, which polls a run and says whether it is still running, every 10 ms until it
/// says no; panics after 120 s, a bound only a hang reaches.
pub(crate) fn poll_until_done(mut poll: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(120);
    while poll() {
        assert!(
            Instant::now() < deadline,
            "the run did not end within 120 s"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The colors of a root `colors.txt` giving two, for a planning test about other findings:
/// an export planned with them reports no `team_colors_missing`.
pub(crate) fn two_team_colors() -> Option<Vec<Rgb>> {
    Some(vec![[0xc1, 0x12, 0x00], [0x41, 0x41, 0x41]])
}

/// `export`, numbered `export_id`, as planning takes it with the root `colors.txt` colors
/// `team_colors` and the note `notes`, the deep pass having found no model with hand weights,
/// none with a metal material, and no `settings.toml`.
pub(crate) fn to_plan(
    export_id: ExportId,
    export: ResolvedAestheticsExport,
    team_colors: Option<Vec<Rgb>>,
    notes: Option<String>,
) -> ExportToPlan {
    ExportToPlan {
        export_id,
        export,
        team_colors,
        notes,
        shared_ids: None,
        hand_weighted: BTreeSet::new(),
        metal_models: BTreeSet::new(),
        settings: BTreeMap::new(),
        settings_refused: BTreeSet::new(),
        legacy_ids: None,
        legacy_block: None,
    }
}

/// The export `name` with these files (path, size), folders and `players.txt`, validated
/// for PES 21 and resolved against a teams list holding `702 /a/`, `714 /co/` and `790 /dbg/`;
/// the export must validate with no issue.
pub(crate) fn resolved(
    name: &str,
    files: &[(&str, u64)],
    folders: &[&str],
    players_txt: Option<&[u8]>,
) -> ResolvedAestheticsExport {
    let (resolved, issues) = resolved_with_issues(name, files, folders, players_txt);
    assert_eq!(issues, Vec::<&str>::new(), "a clean export");
    resolved
}

/// `resolved`'s export, with the codes of the issues validation reported on it.
pub(crate) fn resolved_with_issues(
    name: &str,
    files: &[(&str, u64)],
    folders: &[&str],
    players_txt: Option<&[u8]>,
) -> (ResolvedAestheticsExport, Vec<&'static str>) {
    resolved_with_check(name, files, folders, players_txt, true)
}

/// `resolved_with_issues`, the file-type check strict or lenient as `strict_file_type_check`
/// says: lenient, a disallowed file keeps its item and stays in its folder's files.
pub(crate) fn resolved_with_check(
    name: &str,
    files: &[(&str, u64)],
    folders: &[&str],
    players_txt: Option<&[u8]>,
    strict_file_type_check: bool,
) -> (ResolvedAestheticsExport, Vec<&'static str>) {
    let roster = players_txt.map(|bytes| ("players.txt", bytes.len() as u64));
    let files = files
        .iter()
        .copied()
        .chain(roster)
        .map(|(path, size)| (path.to_owned(), ListedKind::File { size }));
    let folders = folders
        .iter()
        .map(|path| ((*path).to_owned(), ListedKind::Folder));
    let listing = CanonicalListing {
        display_name: name.to_owned(),
        entries: files
            .chain(folders)
            .map(|(path, kind)| ListedEntry { path, kind })
            .collect(),
    };
    let metadata = SmallMetadata {
        files: players_txt
            .map(|bytes| ("players.txt".to_owned(), Ok(bytes.to_vec())))
            .into_iter()
            .collect::<BTreeMap<_, _>>(),
    };
    let report = parse_listing(listing, metadata)
        .unwrap()
        .validate(&ValidationContext {
            version: PesVersion::Pes21,
            strict_file_type_check,
            pass_through: false,
            installed_common_textures: BTreeSet::new(),
        });
    let issues = report.issues.iter().map(|issue| issue.code).collect();
    let teams = TeamsList::parse("ID\tName\n702\t/a/\n714\t/co/\n790\t/dbg/\n").unwrap();
    let resolved = report.validated.unwrap().resolve_identity(&teams).unwrap();
    (resolved, issues)
}
