//! Model-folder mode (`spec/export_upgrader.md` "Model-folder mode"): a
//! folder of old-format model folders (the item folders that used to live
//! under `Faces/`, `Boots/` and `Gloves/`) upgraded one per output folder,
//! to drop into a Studio export. The copies are staged in the export layout
//! step 7's functions read (`Players/<name>/…`, `Boots/<name>/…`,
//! `Common/…`), the write alone taking the `Players`/`Boots`/`Gloves` level
//! off.

use std::collections::BTreeMap;

use aesthetics_export::{FileKind, IssueScope, Marker, ValidationIssue, classify};
use anyhow::anyhow;
use pes_version::{Engine, PesVersion};
use studio_core::{CliError, Disposition, Message, Scope, ToolContext};
use vtree::ScopePath;

use crate::carried::under;
use crate::cli::{CLEAN, ERRORS, UpgradeModelsArgs};
use crate::face_xml::{face_xml_rewrites, kept_common_files};
use crate::far::far_drops;
use crate::kit_magic::{drop_collisions, respell_outputs};
use crate::materials::material_references;
use crate::messages::{
    Code, INPUT, OUTPUT, UnmigratedReason, extract_failed, issue_message, read_failed,
    tool_message, unmigrated,
};
use crate::model_names::rename_models;
use crate::players::{
    ItemKind, clean_folder_name, file_name, is_stray, parse_item_name, path_scope, slot_of,
};
use crate::source::{self, OldExport};
use crate::staging::{claim_staging, finish_staging, write_output_file};
use crate::texture_pairs;
use crate::upgrade::{Identity, RunEvents, invalid, output_issues, resolve_input, verdict};

/// Where an output folder of model-folder mode goes once dropped into a
/// Studio export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Destination {
    /// A player folder, into `Players/`.
    Players,
    /// A shared boots folder, into `Boots/`.
    Boots,
    /// A shared gloves folder, into `Gloves/`.
    Gloves,
    /// The upgraded `Common/`, at the export's root.
    Root,
}

impl Destination {
    /// The report's `into` value: `Players`, `Boots`, `Gloves` or `root`.
    pub(crate) fn into_value(self) -> &'static str {
        match self {
            Destination::Players => "Players",
            Destination::Boots => "Boots",
            Destination::Gloves => "Gloves",
            Destination::Root => "root",
        }
    }

    /// The staged output path's first segment.
    fn staged_top(self) -> &'static str {
        match self {
            Destination::Players => "Players",
            Destination::Boots => "Boots",
            Destination::Gloves => "Gloves",
            Destination::Root => "Common",
        }
    }
}

/// One output folder: its name at the output's top level, and where it goes.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PlacedFolder {
    pub(crate) name: String,
    pub(crate) destination: Destination,
}

/// What model-folder mode makes of the input's top level.
#[derive(Debug, Default)]
pub(crate) struct ModelFolderPlan {
    /// Input path → staged output path, in the export layout step 7's
    /// functions read (`Players/15 - Snuffy/face.fmdl`, `Boots/Crocs/x.model`,
    /// `Common/hair.dds`).
    pub(crate) copies: Vec<(String, String)>,
    /// The `ingame_face.toml` marker files to write, staged paths.
    pub(crate) markers: Vec<String>,
    /// The output folders, in input-name order.
    pub(crate) folders: Vec<PlacedFolder>,
    /// The `unmigrated_content` findings on the input.
    pub(crate) findings: Vec<Message>,
}

/// The group's destination and the name its folder is published under,
/// `None` when no cleaned name exists (a real folder name always has one,
/// but `clean_folder_name`'s type allows it). `Common` in any letter case;
/// else `k`/`g` + four digits by the item grammar (or `g` and three); else
/// a face folder, `NN - Name` when its slot reads or the folder's own name.
fn placed_folder(name: &str) -> Option<(Destination, String)> {
    if name.eq_ignore_ascii_case("common") {
        return Some((Destination::Root, "Common".to_owned()));
    }
    for (kind, destination) in [
        (ItemKind::Boots, Destination::Boots),
        (ItemKind::Gloves, Destination::Gloves),
    ] {
        if let Some((id, name_part)) = parse_item_name(kind, name) {
            let name = clean_folder_name(&name_part)
                .unwrap_or_else(|| format!("{}{id:04}", kind.letter()));
            return Some((destination, name));
        }
    }
    if let Some(nn) = slot_of(name) {
        // The name part is what follows the five characters, its leading
        // spaces and hyphens trimmed, as `face_folders` reads it.
        let name_part = name.get(5..).unwrap_or("").trim_start_matches([' ', '-']);
        let name = match clean_folder_name(name_part) {
            Some(part) => format!("{nn:02} - {part}"),
            None => format!("{nn:02}"),
        };
        return Some((Destination::Players, name));
    }
    clean_folder_name(name).map(|name| (Destination::Players, name))
}

/// The input's top level resolved into the output folders (plan
/// "Model-folder mode"): folders and files grouped by their first segment,
/// `BTreeMap` giving the input-name order; a file directly in the input, a
/// folder holding no file directly in it and a folder whose published name
/// an earlier kept folder already took are unmigrated content.
pub(crate) fn model_folder_plan(files: &[String], folders: &[String]) -> ModelFolderPlan {
    let mut plan = ModelFolderPlan::default();
    let mut groups: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for file in files {
        match file.split_once('/') {
            Some((top, _)) => groups.entry(top.to_owned()).or_default().push(file),
            // A file at the input's top level is no model folder's.
            None => plan.findings.push(unmigrated(
                Scope::File {
                    export_id: INPUT,
                    path: path_scope(file),
                },
                Disposition::DropFile,
                UnmigratedReason::Unclaimed,
                None,
                vec![],
            )),
        }
    }
    // Every top-level folder seeds a group too: one holding no file at all
    // (an empty folder, or only an OS artifact the listing left out) is
    // still a folder the `ModelFolderPlan` must rule on.
    for folder in folders {
        if !folder.contains('/') {
            groups.entry(folder.clone()).or_default();
        }
    }
    // The published names taken so far, folded, each to the input folder's
    // name: they all sit at the output's top level whatever their
    // destination, so a face folder `Crocs` clashes with `k0430 - Crocs` and
    // `common` with `Common`.
    let mut taken: BTreeMap<String, String> = BTreeMap::new();
    for (name, group) in groups {
        let refused =
            |plan: &mut ModelFolderPlan, reason: UnmigratedReason, other: Option<String>| {
                plan.findings.push(unmigrated(
                    Scope::Folder {
                        export_id: INPUT,
                        path: path_scope(&name),
                    },
                    Disposition::DropFolder,
                    reason,
                    other,
                    vec![],
                ));
            };
        let Some((destination, published_name)) = placed_folder(&name) else {
            // `check_paths` keeps an un-scopable name from reaching here;
            // the export carries the finding, `name` naming the folder.
            plan.findings.push(unmigrated(
                Scope::Export { export_id: INPUT },
                Disposition::DropFolder,
                UnmigratedReason::FolderNameInvalid,
                None,
                vec![("name", name.clone())],
            ));
            continue;
        };
        // A folder holding no file directly in it is no model folder: one
        // level, one folder per model. Any file counts, a Fox face folder
        // may carry `face_diff.bin` and a portrait alone; `Common` too, an
        // empty one having nothing to carry.
        let has_file = group
            .iter()
            .any(|file| !file[name.len() + 1..].contains('/'));
        if !has_file {
            refused(&mut plan, UnmigratedReason::NoDirectFiles, None);
            continue;
        }
        if let Some(first) = taken.get(&vtree::fold_name(&published_name)) {
            let first = first.clone();
            refused(&mut plan, UnmigratedReason::FolderNameTaken, Some(first));
            continue;
        }
        taken.insert(vtree::fold_name(&published_name), name.clone());
        // Step 10's stray rule: a classify-unknown file no package read
        // is dropped as a leftover rather than staged into a folder the
        // strict output check would refuse. A Boots or Gloves destination
        // also rules a `.fmdl` as a Fox folder's (an `.fmdl` is Fox content
        // wherever it sits), which only an `.fmdl` sees (`is_stray`).
        let item = match destination {
            Destination::Boots => Some((ItemKind::Boots, Engine::Fox)),
            Destination::Gloves => Some((ItemKind::Gloves, Engine::Fox)),
            Destination::Players | Destination::Root => None,
        };
        let mut marker = false;
        for file in &group {
            let rest = &file[name.len() + 1..];
            if is_stray(file_name(file), item) {
                plan.findings.push(tool_message(
                    Code::LeftoverDropped,
                    Scope::File {
                        export_id: INPUT,
                        path: path_scope(file),
                    },
                    Disposition::DropFile,
                    vec![],
                ));
                continue;
            }
            // In a player folder the old `ingame_face` spellings normalize
            // to `ingame_face.toml`, one per folder; every other file keeps
            // its name (no portrait rename: no `Portraits/` to compare it
            // with).
            if destination == Destination::Players
                && !rest.contains('/')
                && classify(rest) == FileKind::Marker(Marker::OldIngameFace)
            {
                marker = true;
                continue;
            }
            let staged = match destination {
                Destination::Root => format!("Common/{rest}"),
                Destination::Players | Destination::Boots | Destination::Gloves => {
                    format!("{}/{published_name}/{rest}", destination.staged_top())
                }
            };
            plan.copies.push(((*file).clone(), staged));
        }
        if marker {
            plan.markers
                .push(format!("Players/{published_name}/ingame_face.toml"));
        }
        plan.folders.push(PlacedFolder {
            name: published_name,
            destination,
        });
    }
    plan
}

/// The output path a staged one is written at: `Players/`, `Boots/` or
/// `Gloves/` taken off, a `Common/` path as it is.
pub(crate) fn published(staged: &str) -> &str {
    match staged.split_once('/') {
        Some(("Players" | "Boots" | "Gloves", rest)) => rest,
        _ => staged,
    }
}

/// The validation's findings on where a folder will sit in an export rather
/// than what it holds, which model-folder mode leaves out: it has no
/// destination export (plan "Model-folder mode").
const PLACEMENT_CODES: [&str; 3] = [
    "player_folder_number_invalid",
    "player_number_duplicate",
    "shared_folder_orphaned",
];

/// The path the validated listing holds a published output path at:
/// `published`'s inverse. The first segment names a `PlacedFolder`; a
/// `Players`, `Boots` or `Gloves` one's path goes below that folder,
/// `Common/`'s stays as it is.
fn staged(published: &str, folders: &[PlacedFolder]) -> String {
    let first = published.split('/').next().unwrap_or(published);
    let Some(folder) = folders.iter().find(|folder| folder.name == first) else {
        // `run` writes only below a placed folder, so a first segment
        // naming none cannot appear in the output's listing.
        return published.to_owned();
    };
    match folder.destination {
        Destination::Root => published.to_owned(),
        Destination::Players | Destination::Boots | Destination::Gloves => {
            format!("{}/{published}", folder.destination.staged_top())
        }
    }
}

/// The issue's scope in the published layout the report names: each path's
/// staged `Players`/`Boots`/`Gloves` top taken back off, `Common/`'s kept.
fn published_scope(scope: &IssueScope) -> IssueScope {
    let mapped = |path: &ScopePath| {
        ScopePath::new(published(path.as_str()))
            .expect("a validated listing's path stays one with its first segment off")
    };
    match scope {
        IssueScope::Export => IssueScope::Export,
        IssueScope::Folder(path) => IssueScope::Folder(mapped(path)),
        IssueScope::File(path) => IssueScope::File(mapped(path)),
        IssueScope::RosterEntry { file, line, slot } => IssueScope::RosterEntry {
            file: mapped(file),
            line: *line,
            slot: *slot,
        },
    }
}

/// Runs `upgrade-models`: the input checked, the folder listed as it is, the
/// output folder made, the top level planned and migrated through step 7's
/// functions on the staged layout, the copies written where they publish,
/// each output folder's destination reported, and the verdict: `upgrade`'s
/// "CLI" order without the team, the roster or the kits.
pub(crate) fn run(args: &UpgradeModelsArgs, ctx: &ToolContext) -> Result<u8, CliError> {
    // "CLI": the input must exist and be a folder.
    let path = &args.folder;
    if !path.exists() {
        return Err(invalid(anyhow!("{} does not exist", path.display())));
    }
    if !path.is_dir() {
        return Err(invalid(anyhow!("{} is not a folder", path.display())));
    }
    let path = resolve_input(path)?;
    let name = source::source_name(&path);
    let mut events = RunEvents::new(ctx);
    events.started(INPUT, &name);

    // Step 1, as it is: the folder's top level is the model folders.
    let mut export = match OldExport::open_as_is(&path) {
        Ok(export) => export,
        Err(failure) => {
            events.message(extract_failed(INPUT, failure));
            events.processed(INPUT);
            return Ok(ERRORS);
        }
    };

    // "CLI": the output folder, claimed as `upgrade` claims its own
    // (`staging::claim_staging`). The path's name is read before anything is
    // created: a `-o` ending in `..`, or a root, has none and is refused,
    // never panicking after the parent was made.
    let output = args.output.clone().unwrap_or_else(|| {
        path.parent()
            .expect("the input is absolute and not a filesystem root (resolve_input)")
            .join(format!("{name} upgraded"))
    });
    let Some(output_name) = output.file_name() else {
        return Err(invalid(anyhow!(
            "the output path {} has no folder name",
            output.display()
        )));
    };
    let output_name = output_name.to_string_lossy().into_owned();
    let (staging, removed) = claim_staging(&output)?;
    events.started(OUTPUT, &output_name);
    if removed {
        events.message(tool_message(
            Code::StagingFolderRemoved,
            Scope::Export { export_id: OUTPUT },
            Disposition::Keep,
            vec![("path", staging.display().to_string())],
        ));
    }

    // Step 7's chain on the staged copies, exactly `upgrade`'s: the kit
    // magic's respells first, the `Common/` names a `face.xml` or a link
    // keeps, then the model renames and the output-path collisions, the
    // `.mtl` rewrites and the `face.xml` respells against the one list.
    let identity = Identity::ModelFolders;
    // `upgrade`'s drop too (`far.rs`): a model every vertex of which sits
    // past the far checks' 5000 units is geometry the compiler refuses.
    let mut files = export.files.clone();
    let far = far_drops(&files, true, |path| export.read(path));
    files.retain(|file| !far.dropped.contains(file));
    let mut plan = model_folder_plan(&files, &export.folders);
    let respelled = respell_outputs(std::mem::take(&mut plan.copies), &identity);
    let kept = kept_common_files(&respelled, &identity, |path| export.read(path));
    // Model-folder mode has no team ID: the old compile's rename cannot
    // be replayed, its collisions staying unmigrated content.
    let (copies, dropped) = drop_collisions(rename_models(respelled, &kept), None);
    plan.copies = copies;
    plan.findings.extend(dropped);
    // A `.dds` beside a `.ftex` of one stem in one output folder is a
    // `texture_stem_conflict` the validation would refuse, resolved as in
    // `upgrade`: the `.ftex` drops first so the links are spelled after
    // the kept `.dds` alone, and the `.dds` follows when no model wears
    // the stem.
    let pairs = texture_pairs::stem_pairs(plan.copies.iter());
    let ftex_dropped: std::collections::BTreeSet<&str> =
        pairs.iter().map(|pair| pair.ftex.0.as_str()).collect();
    plan.copies
        .retain(|(input, _)| !ftex_dropped.contains(input.as_str()));
    // `Common/` for the materials: the respelled output paths the links are
    // spelled after.
    let common: Vec<String> = plan
        .copies
        .iter()
        .filter(|(_, output)| under(output, "common").is_some())
        .map(|(_, output)| output.clone())
        .collect();
    let mut materials =
        material_references(&plan.copies, &common, &identity, |path| export.read(path));
    let (pair_findings, dds_dropped) = texture_pairs::resolve(&pairs, &materials.used);
    if !dds_dropped.is_empty() {
        plan.copies
            .retain(|(input, _)| !dds_dropped.contains(input));
    }
    materials.findings.extend(pair_findings);
    let face_xml = face_xml_rewrites(&plan.copies, &identity, |path| export.read(path));
    let mut any_unmigrated = false;
    for message in plan
        .findings
        .into_iter()
        .chain(far.findings)
        .chain(materials.findings)
    {
        if message.code.code.as_ref() == Code::UnmigratedContent.as_str() {
            any_unmigrated = true;
        }
        events.message(message);
    }
    if any_unmigrated {
        events.unmigrated();
    }

    // The writes publish the staged paths: the `Players/`/`Boots/`/`Gloves/`
    // level comes off, each output folder droppable as it stands.
    for (from, to) in &plan.copies {
        if materials.written.iter().any(|(output, _)| output == to)
            || face_xml.iter().any(|(output, _)| output == to)
        {
            // A rewritten `.mtl` or `face.xml` replaces its input's copy.
            continue;
        }
        match export.read(from) {
            Ok(bytes) => write_output_file(&staging, published(to), &bytes)?,
            Err(failure) => {
                events.message(read_failed(INPUT, failure, Disposition::DropFile));
            }
        }
    }
    for (path, bytes) in &materials.written {
        write_output_file(&staging, published(path), bytes)?;
    }
    for (path, bytes) in &face_xml {
        write_output_file(&staging, published(path), bytes)?;
    }
    for link in &materials.links {
        write_output_file(&staging, published(link), b"")?;
    }
    for marker in &plan.markers {
        write_output_file(&staging, published(marker), b"")?;
    }

    // The report names, per output folder, where it goes once dropped in.
    for folder in &plan.folders {
        events.message(tool_message(
            Code::ModelFolderDestination,
            Scope::Folder {
                export_id: OUTPUT,
                path: ScopePath::new(&folder.name)
                    .expect("a published name is a cleaned folder name, a valid segment"),
            },
            Disposition::Keep,
            vec![("into", folder.destination.into_value().to_owned())],
        ));
    }
    // The input's half ends here: read and migrated, as in `upgrade`.
    events.processed(INPUT);

    // "Verification": the shared validation on the output as it will sit
    // once dropped into an export (the published paths placed back below
    // their staged tops), as a Midcup export, which needs no kits. PES 16
    // when an output path holds a `.model`, PES 21 otherwise.
    let version = if plan
        .copies
        .iter()
        .any(|(_, output)| output.to_ascii_lowercase().ends_with(".model"))
    {
        PesVersion::Pes16
    } else {
        PesVersion::Pes21
    };
    let issues = output_issues(
        &staging,
        "models Midcup",
        version,
        |path| staged(path, &plan.folders),
        &mut events,
    );
    // The placement findings are the destination export's to make once the
    // folders are dropped in, not this run's, which has none; kept,
    // `shared_folder_orphaned` would drop every boots and gloves folder.
    for issue in issues {
        if PLACEMENT_CODES.contains(&issue.code) {
            continue;
        }
        let issue = ValidationIssue {
            scope: published_scope(&issue.scope),
            ..issue
        };
        events.message(issue_message(&issue, OUTPUT));
    }
    // The output's half ends with its validation.
    events.processed(OUTPUT);

    // "CLI": the verdict line and the exit code, as `upgrade`'s, the verdict
    // publishing the staging folder or removing it: only a clean run writes
    // its output.
    let (code, exit_code) = verdict(events.worst(), events.dropped());
    events.message(tool_message(code, Scope::Run, Disposition::Keep, vec![]));
    finish_staging(&staging, &output, exit_code == CLEAN)?;
    Ok(exit_code)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use studio_core::{PipelineEvent, PipelineEventEnvelope};

    use super::*;
    use crate::test_support::{scratch, test_ctx};

    fn files(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    fn placed(name: &str, destination: Destination) -> PlacedFolder {
        PlacedFolder {
            name: name.to_owned(),
            destination,
        }
    }

    #[test]
    fn the_input_folders_become_output_folders_with_their_destinations() {
        let plan = model_folder_plan(
            &files(&[
                "Common/hair.dds",
                "Snuffy/hair.model",
                "Snuffy/ingame_face.txt",
                "XXX15 - Snuffy/face.fmdl",
                "XXX15 - Snuffy/u0XXXp1.dds",
                "k0430 - Crocs/gloveL_crocs.model",
                "k0431/boots.model",
            ]),
            &[],
        );
        assert_eq!(
            plan.folders,
            [
                placed("Common", Destination::Root),
                placed("Snuffy", Destination::Players),
                placed("15 - Snuffy", Destination::Players),
                placed("Crocs", Destination::Boots),
                placed("k0431", Destination::Boots),
            ]
        );
        assert_eq!(
            plan.copies,
            [
                ("Common/hair.dds".to_owned(), "Common/hair.dds".to_owned()),
                (
                    "Snuffy/hair.model".to_owned(),
                    "Players/Snuffy/hair.model".to_owned()
                ),
                (
                    "XXX15 - Snuffy/face.fmdl".to_owned(),
                    "Players/15 - Snuffy/face.fmdl".to_owned()
                ),
                (
                    "XXX15 - Snuffy/u0XXXp1.dds".to_owned(),
                    "Players/15 - Snuffy/u0XXXp1.dds".to_owned()
                ),
                (
                    "k0430 - Crocs/gloveL_crocs.model".to_owned(),
                    "Boots/Crocs/gloveL_crocs.model".to_owned()
                ),
                (
                    "k0431/boots.model".to_owned(),
                    "Boots/k0431/boots.model".to_owned()
                ),
            ]
        );
        assert_eq!(plan.markers, ["Players/Snuffy/ingame_face.toml".to_owned()]);
        assert!(plan.findings.is_empty(), "{:?}", plan.findings);
    }

    #[test]
    fn an_ingame_face_spelling_in_any_letter_case_becomes_the_marker() {
        let plan = model_folder_plan(
            &files(&["Snuffy/INGAME_FACE.TXT", "Snuffy/hair.model"]),
            &[],
        );
        assert_eq!(plan.markers, ["Players/Snuffy/ingame_face.toml".to_owned()]);
        assert_eq!(
            plan.copies,
            [(
                "Snuffy/hair.model".to_owned(),
                "Players/Snuffy/hair.model".to_owned()
            )]
        );
    }

    #[test]
    fn a_gloves_folder_is_named_by_its_name_part_or_its_id() {
        let plan = model_folder_plan(
            &files(&["g0502 - Mitts/glove_l.model", "g567/glove_l.model"]),
            &[],
        );
        assert_eq!(
            plan.folders,
            [
                placed("Mitts", Destination::Gloves),
                placed("g0567", Destination::Gloves),
            ]
        );
        assert_eq!(
            plan.copies,
            [
                (
                    "g0502 - Mitts/glove_l.model".to_owned(),
                    "Gloves/Mitts/glove_l.model".to_owned()
                ),
                (
                    "g567/glove_l.model".to_owned(),
                    "Gloves/g0567/glove_l.model".to_owned()
                ),
            ]
        );
        assert!(plan.findings.is_empty(), "{:?}", plan.findings);
    }

    /// The refusal of a group: `unmigrated_content`, folder scope on `INPUT`,
    /// `DropFolder`, `reason` and `other` in the context.
    fn assert_refused(message: &Message, folder: &str, reason: &str, other: Option<&str>) {
        assert_eq!(message.code.code.as_ref(), "unmigrated_content");
        assert_eq!(
            message.scope,
            Scope::Folder {
                export_id: INPUT,
                path: path_scope(folder),
            }
        );
        assert_eq!(message.disposition, Disposition::DropFolder);
        let mut context = vec![("reason".to_owned(), reason.to_owned())];
        if let Some(other) = other {
            context.push(("other".to_owned(), other.to_owned()));
        }
        assert_eq!(message.context, context);
    }

    #[test]
    fn a_top_level_folder_holding_no_file_directly_in_it_is_refused() {
        // A two-level layout, an empty folder and an empty `Common` alike
        // hold no file directly in them; a face folder holding any file
        // forms (`Textures`'s `.dds` too: a Fox face folder may carry no
        // model of its own).
        let folders = files(&["Common", "Empty", "players", "players/Ann", "Textures"]);
        let plan = model_folder_plan(
            &files(&["players/Ann/face.fmdl", "Textures/u0XXXp1.dds"]),
            &folders,
        );
        assert_eq!(plan.findings.len(), 3, "{:?}", plan.findings);
        assert_refused(&plan.findings[0], "Common", "no_direct_files", None);
        assert_refused(&plan.findings[1], "Empty", "no_direct_files", None);
        assert_refused(&plan.findings[2], "players", "no_direct_files", None);
        assert_eq!(plan.folders, [placed("Textures", Destination::Players)]);
        assert_eq!(
            plan.copies,
            [(
                "Textures/u0XXXp1.dds".to_owned(),
                "Players/Textures/u0XXXp1.dds".to_owned()
            )]
        );
    }

    #[test]
    fn a_face_folder_without_a_model_keeps_its_files() {
        // `face_diff.bin` and a portrait are a real Fox face folder's
        // content: any file directly in it makes it a model folder.
        let plan = model_folder_plan(
            &files(&[
                "XXX01 - Dekinai/face_diff.bin",
                "XXX01 - Dekinai/portrait.dds",
            ]),
            &[],
        );
        assert_eq!(plan.folders, [placed("01 - Dekinai", Destination::Players)]);
        assert_eq!(
            plan.copies,
            [
                (
                    "XXX01 - Dekinai/face_diff.bin".to_owned(),
                    "Players/01 - Dekinai/face_diff.bin".to_owned()
                ),
                (
                    "XXX01 - Dekinai/portrait.dds".to_owned(),
                    "Players/01 - Dekinai/portrait.dds".to_owned()
                ),
            ]
        );
        assert!(plan.findings.is_empty(), "{:?}", plan.findings);
    }

    #[test]
    fn the_first_folder_by_input_name_wins_a_name_clash() {
        // `71415 - A` and `XXX15 - A` both publish `15 - A`.
        let plan = model_folder_plan(&files(&["71415 - A/face.fmdl", "XXX15 - A/face.fmdl"]), &[]);
        assert_eq!(plan.folders, [placed("15 - A", Destination::Players)]);
        assert_eq!(
            plan.copies,
            [(
                "71415 - A/face.fmdl".to_owned(),
                "Players/15 - A/face.fmdl".to_owned()
            )]
        );
        assert_eq!(plan.findings.len(), 1, "{:?}", plan.findings);
        assert_refused(
            &plan.findings[0],
            "XXX15 - A",
            "folder_name_taken",
            Some("71415 - A"),
        );

        // A face folder `Crocs` and `k0430 - Crocs` both publish `Crocs`
        // beside each other at the output's top level.
        let plan = model_folder_plan(
            &files(&["Crocs/face.fmdl", "k0430 - Crocs/boots.model"]),
            &[],
        );
        assert_eq!(plan.folders, [placed("Crocs", Destination::Players)]);
        assert_eq!(
            plan.copies,
            [(
                "Crocs/face.fmdl".to_owned(),
                "Players/Crocs/face.fmdl".to_owned()
            )]
        );
        assert_eq!(plan.findings.len(), 1, "{:?}", plan.findings);
        assert_refused(
            &plan.findings[0],
            "k0430 - Crocs",
            "folder_name_taken",
            Some("Crocs"),
        );

        // Letter case aside: `k0430 - crocs` still publishes `Crocs`'s name.
        let plan = model_folder_plan(
            &files(&["Crocs/face.fmdl", "k0430 - crocs/boots.model"]),
            &[],
        );
        assert_eq!(plan.folders, [placed("Crocs", Destination::Players)]);
        assert_eq!(plan.findings.len(), 1, "{:?}", plan.findings);
        assert_refused(
            &plan.findings[0],
            "k0430 - crocs",
            "folder_name_taken",
            Some("Crocs"),
        );
    }

    #[test]
    fn a_file_directly_in_the_input_is_unmigrated_content() {
        let plan = model_folder_plan(&files(&["readme.txt"]), &[]);
        assert_eq!(plan.findings.len(), 1, "{:?}", plan.findings);
        let message = &plan.findings[0];
        assert_eq!(message.code.code.as_ref(), "unmigrated_content");
        assert_eq!(
            message.scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("readme.txt"),
            }
        );
        assert_eq!(message.disposition, Disposition::DropFile);
        assert_eq!(
            message.context,
            [("reason".to_owned(), "unclaimed".to_owned())]
        );
        assert!(plan.folders.is_empty());
        assert!(plan.copies.is_empty());
    }

    #[test]
    fn a_folder_name_cleaning_to_nothing_is_refused() {
        // `?` is no folder-name character: nothing publishable is left.
        // The name cannot scope a folder, so the export carries the
        // finding and `name` names it.
        let plan = model_folder_plan(&files(&["???/face.fmdl"]), &[]);
        assert_eq!(plan.findings.len(), 1, "{:?}", plan.findings);
        let message = &plan.findings[0];
        assert_eq!(message.code.code.as_ref(), "unmigrated_content");
        assert_eq!(message.scope, Scope::Export { export_id: INPUT });
        assert_eq!(message.disposition, Disposition::DropFolder);
        assert_eq!(
            message.context,
            [
                ("reason".to_owned(), "folder_name_invalid".to_owned()),
                ("name".to_owned(), "???".to_owned()),
            ]
        );
        assert!(plan.folders.is_empty());
        assert!(plan.copies.is_empty());
    }

    #[test]
    fn a_face_folder_without_a_slot_keeps_its_own_name() {
        // `XXX30`'s "slot" is past 23: no slot, the name is the folder's own.
        let plan = model_folder_plan(&files(&["XXX30 - A/face.fmdl", "XXX15/face.fmdl"]), &[]);
        assert_eq!(
            plan.folders,
            [
                placed("15", Destination::Players),
                placed("XXX30 - A", Destination::Players),
            ]
        );
        assert_eq!(
            plan.copies,
            [
                (
                    "XXX15/face.fmdl".to_owned(),
                    "Players/15/face.fmdl".to_owned()
                ),
                (
                    "XXX30 - A/face.fmdl".to_owned(),
                    "Players/XXX30 - A/face.fmdl".to_owned()
                ),
            ]
        );
        assert!(plan.findings.is_empty(), "{:?}", plan.findings);
    }

    #[test]
    fn published_takes_the_staged_top_level_off() {
        assert_eq!(
            published("Players/15 - Snuffy/face.fmdl"),
            "15 - Snuffy/face.fmdl"
        );
        assert_eq!(published("Boots/Crocs/x"), "Crocs/x");
        assert_eq!(published("Common/hair.dds"), "Common/hair.dds");
    }

    #[test]
    fn staged_places_a_published_path_below_its_folders_top() {
        let folders = [
            placed("15 - Snuffy", Destination::Players),
            placed("Crocs", Destination::Boots),
            placed("Mitts", Destination::Gloves),
            placed("Common", Destination::Root),
        ];
        assert_eq!(
            staged("15 - Snuffy/face.fmdl", &folders),
            "Players/15 - Snuffy/face.fmdl"
        );
        assert_eq!(staged("Crocs/x", &folders), "Boots/Crocs/x");
        assert_eq!(staged("Mitts/x", &folders), "Gloves/Mitts/x");
        assert_eq!(staged("Common/hair.dds", &folders), "Common/hair.dds");
        // A first segment naming no placed folder is returned as it is.
        assert_eq!(staged("Other/x", &folders), "Other/x");
    }

    /// The codes of the run's `Message` events, in order.
    fn codes(events_rx: &crossbeam_channel::Receiver<PipelineEventEnvelope>) -> Vec<String> {
        events_rx
            .iter()
            .filter_map(|envelope| match envelope.event {
                PipelineEvent::Message(message) => Some(message.code.code.as_ref().to_owned()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_model_file_in_any_letter_case_makes_the_output_pes_16() {
        let root = scratch("models_run_model_case");
        fs::create_dir_all(root.join("new models/k0431")).unwrap();
        fs::write(root.join("new models/k0431/x.MODEL"), b"model").unwrap();
        let (ctx, events_rx) = test_ctx();
        let args = UpgradeModelsArgs {
            folder: root.join("new models"),
            output: None,
        };

        run(&args, &ctx).unwrap();
        drop(ctx);
        // A boots folder's model that is not boots-named is
        // `model_name_invalid` on the pre-Fox engine the `.MODEL` file
        // selects (`fmdl_name_invalid` under PES 21).
        let codes = codes(&events_rx);
        assert!(
            codes.contains(&"model_name_invalid".to_owned()),
            "{codes:?}"
        );
    }

    #[test]
    fn each_started_export_is_processed_once_after_its_start() {
        // The console's exports counter reads `0/2` for a whole run when neither the input
        // nor the output is ever reported processed.
        let root = scratch("models_run_processed");
        fs::create_dir_all(root.join("new models/k0431")).unwrap();
        fs::write(root.join("new models/k0431/boots.model"), b"model").unwrap();
        let (ctx, events_rx) = test_ctx();
        let args = UpgradeModelsArgs {
            folder: root.join("new models"),
            output: None,
        };

        run(&args, &ctx).unwrap();
        drop(ctx);

        let events: Vec<String> = events_rx
            .iter()
            .filter_map(|envelope| match envelope.event {
                PipelineEvent::ExportStarted { export_id, .. } => {
                    Some(format!("started {}", export_id.0))
                }
                PipelineEvent::ExportProcessed { export_id } => {
                    Some(format!("processed {}", export_id.0))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            events,
            [
                format!("started {}", INPUT.0),
                format!("started {}", OUTPUT.0),
                format!("processed {}", INPUT.0),
                format!("processed {}", OUTPUT.0),
            ]
        );
    }

    #[test]
    fn an_unused_pairs_dds_follows_its_ftex_but_the_rest_migrates() {
        // A `.dds`/`.ftex` pair no material names the stem of: the
        // `.ftex` drops with the pair's resolve and the `.dds` follows as
        // a leftover; the folder's other files migrate.
        let root = scratch("models_run_unused_pair");
        fs::create_dir_all(root.join("new models/Textures")).unwrap();
        for name in [
            "u0XXXp1.dds",
            "u0XXXp1.ftex",
            "portrait.dds",
            "ingame_face.txt",
        ] {
            fs::write(root.join("new models/Textures").join(name), name).unwrap();
        }
        let (ctx, _events_rx) = test_ctx();
        let args = UpgradeModelsArgs {
            folder: root.join("new models"),
            output: None,
        };

        run(&args, &ctx).unwrap();
        let out = root.join("new models upgraded/Textures");
        // The pair's respelled `kit1` files are both gone.
        let mut files: Vec<String> = fs::read_dir(&out)
            .unwrap_or_else(|_| panic!("the output folder {out:?} was published"))
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        files.sort();
        assert_eq!(files, ["ingame_face.toml", "portrait.dds"], "{out:?}");
    }

    #[test]
    fn a_stray_in_a_model_folder_is_dropped_as_a_leftover() {
        // Step 10's stray rule: the backup file is no `.txt` note and no
        // kind `classify` knows — a `leftover_dropped` finding and no copy,
        // the folder migrating without it.
        let plan = model_folder_plan(&files(&["k0431/boots.model", "k0431/boots.model.bak"]), &[]);
        assert_eq!(
            plan.copies,
            [(
                "k0431/boots.model".to_owned(),
                "Boots/k0431/boots.model".to_owned()
            )]
        );
        let stray = plan
            .findings
            .iter()
            .find(|m| m.code.code.as_ref() == "leftover_dropped")
            .expect("leftover_dropped expected");
        assert_eq!(
            stray.scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("k0431/boots.model.bak")
            }
        );
        assert_eq!(stray.disposition, Disposition::DropFile);
    }
}
