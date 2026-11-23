//! The `upgrade-dpfl` command (`spec/team_compiler/pipeline.md` "DpFileList upgrade"): the
//! installed `DpFileList.bin` replaced by the cup's official one, after an old DLC's CPKs are
//! renamed to the official names of their stem, so they keep loading. The placeholder CPK is
//! written for every official entry with no file, since the game loads none of `download/`
//! when a listed CPK is missing. Without `--yes` it
//! lists what it would do and writes nothing; it never deletes or overwrites a CPK. The
//! window's Upgrade DpFileList dialog alone deletes, one ticked box per dropped CPK:
//! `preview` works out what it shows, and `run`'s `delete` carries the ticks.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use anyhow::{Context, bail};
use studio_core::{Disposition, Message, Scope, Severity, ToolContext};

use crate::events::RunEvents;
use crate::messages::{Code, size_text, tool_message};
use crate::output::deploy::{DPFILELIST, UPGRADE_COMMAND};
use crate::paths::replace_file;
use crate::templates::{self, Templates};

/// The file the replaced list is kept as, beside it in `download/`.
const BACKUP: &str = "DpFileList.bin.bak";

/// The stems the official list has none of, and the official stem each one's CPKs take: the
/// `teams` runs replaced the faces/uniform split.
const ALIASES: [(&str, &str); 4] = [
    ("faces", "teams"),
    ("uniform", "teams"),
    ("other_faces", "teams2"),
    ("other_uniform", "teams2"),
];

/// What an upgrade does in `download/`, worked out before anything is written.
#[derive(Debug, PartialEq, Eq)]
struct Plan {
    /// The CPKs renamed, as `(from, to)` file names, in the order the renames run.
    renames: Vec<(String, String)>,
    /// The official entries with no file once the renames are made, in the official order:
    /// each gets the placeholder CPK.
    placeholders: Vec<String>,
    /// The installed entries the official list lacks that are not renamed, in the installed
    /// order: the game no longer loads them.
    dropped: Vec<String>,
}

/// The upgrade of the `installed` list's `download/` folder, whose file names are `present`,
/// to the `official` list. An old DLC's CPKs are renamed by stem, so they keep loading.
fn plan(installed: &[String], official: &[String], present: &BTreeSet<String>) -> Plan {
    // The names in `download/` as each rename's turn comes.
    let mut present = present.clone();
    let mut renames: Vec<(String, String)> = Vec::new();
    for (group_stem, entries) in stem_groups(installed, official, &present) {
        if entries.iter().all(|entry| official.contains(entry)) {
            continue;
        }
        // Every file of the stem takes the stem's official names in list order, those already
        // under one included, so the load order is kept.
        let names = official
            .iter()
            .filter(|name| stem(name) == Some(group_stem));
        let moves: Vec<(&String, &String)> = entries
            .into_iter()
            .zip(names)
            .filter(|(from, to)| from != to)
            .collect();
        // Last first, so a file moves off a name before the one before it takes the name; a
        // rename whose target is still held is not made, so none overwrites a file.
        for (from, to) in moves.into_iter().rev() {
            if present.contains(to) {
                continue;
            }
            present.remove(from);
            present.insert(to.clone());
            renames.push((from.clone(), to.clone()));
        }
    }
    let placeholders = official
        .iter()
        .filter(|name| !present.contains(*name))
        .cloned()
        .collect();
    let dropped = installed
        .iter()
        .filter(|entry| !official.contains(entry))
        .filter(|entry| renames.iter().all(|(from, _)| from != *entry))
        .cloned()
        .collect();
    Plan {
        renames,
        placeholders,
        dropped,
    }
}

/// The `installed` entries whose file is `present`, grouped by the official stem each one's
/// CPK takes (`official_stem`): the groups in the order of their first entry, each group's
/// entries in list order. An entry that takes no official stem is in no group.
fn stem_groups<'a>(
    installed: &'a [String],
    official: &'a [String],
    present: &BTreeSet<String>,
) -> Vec<(&'a str, Vec<&'a String>)> {
    let mut groups: Vec<(&str, Vec<&String>)> = Vec::new();
    for entry in installed.iter().filter(|entry| present.contains(*entry)) {
        let Some(stem) = official_stem(entry, official) else {
            continue;
        };
        match groups.iter_mut().find(|(known, _)| *known == stem) {
            Some((_, entries)) => entries.push(entry),
            None => groups.push((stem, vec![entry])),
        }
    }
    groups
}

/// The stem of the `official` list whose names the CPK `entry` takes: its own stem when an
/// official entry has it (always, for an official entry), else its stem without trailing
/// digits (`stadiums0` is `stadiums`), else that stem's alias (`ALIASES`); `None` when none is
/// official or `entry` has no stem.
fn official_stem<'a>(entry: &str, official: &'a [String]) -> Option<&'a str> {
    let own = stem(entry)?;
    let without_digits = own.trim_end_matches(|character: char| character.is_ascii_digit());
    let alias = ALIASES
        .iter()
        .find(|(old, _)| *old == without_digits)
        .map(|(_, new)| *new);
    [Some(own), Some(without_digits), alias]
        .into_iter()
        .flatten()
        .find_map(|candidate| {
            official
                .iter()
                .filter_map(|name| stem(name))
                .find(|official_stem| *official_stem == candidate)
        })
}

/// A CPK file name's stem: what follows `{prefix}_{NN}_` (`NN` one or more digits), without
/// `.cpk` (`stadiums0` for `4cc_30_stadiums0.cpk`); `None` for a name not of that shape.
fn stem(name: &str) -> Option<&str> {
    number_and_stem(name).map(|(_, stem)| stem)
}

/// A CPK file name's number `NN` and stem, as `stem` reads them (`("30", "stadiums0")` for
/// `4cc_30_stadiums0.cpk`); `None` for a name not of that shape.
pub(crate) fn number_and_stem(name: &str) -> Option<(&str, &str)> {
    let (prefix, rest) = name.strip_suffix(".cpk")?.split_once('_')?;
    let (number, stem) = rest.split_once('_')?;
    let shaped = !prefix.is_empty()
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
        && !stem.is_empty();
    shaped.then_some((number, stem))
}

/// What `run` and `preview` work out about `download` before anything is written.
struct WorkedOut {
    /// The resources the run builds with: each embedded one or its `templates/` override.
    templates: Templates,
    /// The installed `DpFileList.bin`'s bytes; `None` when there is no file.
    old_list: Option<Vec<u8>>,
    /// `old_list`'s bytes did not read as a list: the error's text.
    unreadable: Option<String>,
    /// The file names in `download/` as the working-out listed them.
    present: BTreeSet<String>,
    /// The renames, placeholders and dropped entries.
    plan: Plan,
    /// The installed list is the official one byte for byte.
    list_is_official: bool,
    /// `list_is_official` and nothing to rename or write: the upgrade has nothing to do.
    up_to_date: bool,
}

/// What the upgrade would do in `download`, worked out before anything is written (`run` and
/// `preview` share it): the templates read first, their findings reported through `events`;
/// `None` when a `templates/` file could not be read, its Fatal already reported.
fn work_out(
    download: &Path,
    ctx: &ToolContext,
    events: &mut RunEvents,
) -> anyhow::Result<Option<WorkedOut>> {
    let Some(templates) = templates::read_reported(ctx, events) else {
        return Ok(None);
    };
    let old_list = read_list_file(&download.join(DPFILELIST))?;
    // A list that does not read as a list is replaced like any other, with nothing renamed.
    let (installed, unreadable) = match old_list.as_deref().map(cpk::dp_file_list::read) {
        None => (Vec::new(), None),
        Some(Ok(entries)) => (entries, None),
        Some(Err(error)) => (Vec::new(), Some(format!("{error:#}"))),
    };
    let present = file_names(download)?;
    let plan = plan(&installed, &templates.official_list(), &present);
    let list_is_official = old_list.as_deref() == Some(templates.official_list_file());
    let up_to_date = list_is_official && plan.renames.is_empty() && plan.placeholders.is_empty();
    Ok(Some(WorkedOut {
        templates,
        old_list,
        unreadable,
        present,
        plan,
        list_is_official,
        up_to_date,
    }))
}

/// What the DpFileList upgrade would do in `download/`, worked out before anything is
/// written: the window's Upgrade DpFileList dialog shows it before the member confirms.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Preview {
    /// The CPKs renamed, as `(from, to)` file names, in the order the renames run.
    pub(crate) renames: Vec<(String, String)>,
    /// How many official entries have no file once the renames are made: each gets the
    /// placeholder CPK.
    pub(crate) placeholders: usize,
    /// The installed entries the official list lacks that are not renamed, in the installed
    /// order, each with its file's size in bytes when the file is in `download/` (`None`
    /// when it is not): the game no longer loads them.
    pub(crate) dropped: Vec<(String, Option<u64>)>,
    /// The installed list already is the official one and every CPK it names is in
    /// `download/`: nothing to do (the other fields are then empty).
    pub(crate) up_to_date: bool,
    /// The installed list is not the official one byte for byte: the upgrade replaces it.
    pub(crate) replaces_list: bool,
}

/// `run` without `apply`'s plan for `download`, as data for the window's Upgrade DpFileList
/// dialog rather than findings: what the run would rename, write and drop, and whether there
/// is nothing to do. The templates' findings are reported as `run` reports them; a file that
/// cannot be read, a `templates/` file among them, is the error.
pub(crate) fn preview(download: &Path, ctx: &ToolContext) -> anyhow::Result<Preview> {
    let mut events = RunEvents::new(ctx, false);
    let Some(worked) = work_out(download, ctx, &mut events)? else {
        bail!("the DpFileList upgrade cannot be planned: its templates could not be read");
    };
    let mut dropped = Vec::with_capacity(worked.plan.dropped.len());
    for cpk in &worked.plan.dropped {
        let size = dropped_size(download, &worked.present, cpk)?;
        dropped.push((cpk.clone(), size));
    }
    Ok(Preview {
        renames: worked.plan.renames,
        placeholders: worked.plan.placeholders.len(),
        dropped,
        up_to_date: worked.up_to_date,
        replaces_list: !worked.list_is_official,
    })
}

/// The `upgrade-dpfl` command in `download`, the PES folder's `download/` (checked to be a
/// folder); `delete` is the window's dialog's ticked dropped CPKs, empty on the command
/// line, which never deletes. It reports the upgrade's findings (`plan`) in the order `apply`
/// (`--yes`) acts, each emitted once its action is done (renames, placeholders, the list,
/// then the dropped entries). Applying, a dropped CPK named in `delete` whose file is
/// present is removed after the list is replaced, and its line is `dpfilelist_cpk_deleted`
/// in place of `dpfilelist_cpk_dropped`; one whose removal fails keeps the dropped line,
/// with an `error` context saying why, and the run goes on to the next dropped entry. A
/// `delete` name that is not a dropped CPK is ignored. Without `apply` it reports
/// `dpfilelist_upgrade_planned` last and writes nothing; `dpfilelist_up_to_date` alone when
/// there is nothing to do. Returns the worst severity reported. A file that cannot be read
/// or written is the error, naming it — a failed delete aside — and what was done before
/// stays done: the list is replaced before the deletions, so a run stopped earlier leaves
/// the old one in place and a second run finishes the work.
pub(crate) fn run(
    download: &Path,
    apply: bool,
    delete: &BTreeSet<String>,
    ctx: &ToolContext,
) -> anyhow::Result<Option<Severity>> {
    let mut events = RunEvents::new(ctx, false);
    let Some(WorkedOut {
        templates,
        old_list,
        unreadable,
        present,
        plan,
        list_is_official,
        up_to_date,
    }) = work_out(download, ctx, &mut events)?
    else {
        return Ok(events.worst());
    };
    let list_path = download.join(DPFILELIST);
    if up_to_date {
        let path = ("path", list_path.display().to_string());
        events.message(finding(Code::DpfilelistUpToDate, vec![path]));
        return Ok(events.worst());
    }
    for (from, to) in &plan.renames {
        if apply {
            rename(download, from, to)?;
        }
        let context = vec![("from", from.clone()), ("to", to.clone())];
        events.message(finding(Code::DpfilelistCpkRenamed, context));
    }
    for cpk in &plan.placeholders {
        if apply {
            write_new(download, cpk, templates.placeholder_cpk())?;
        }
        let context = vec![("cpk", cpk.clone())];
        events.message(finding(Code::DpfilelistPlaceholderWritten, context));
    }
    if !list_is_official {
        let backup = download.join(BACKUP);
        if apply {
            if let Some(old_list) = &old_list {
                replace_file(&backup, old_list)?;
            }
            replace_file(&list_path, templates.official_list_file())?;
        }
        let mut context = vec![("path", list_path.display().to_string())];
        if old_list.is_some() {
            context.push(("backup", backup.display().to_string()));
        }
        if let Some(error) = unreadable {
            context.push(("unreadable", error));
        }
        events.message(finding(Code::DpfilelistReplaced, context));
    }
    for cpk in &plan.dropped {
        let mut context = vec![("cpk", cpk.clone())];
        let mut code = Code::DpfilelistCpkDropped;
        if let Some(size) = dropped_size(download, &present, cpk)? {
            context.push(("size", size_text(size)));
            if apply && delete.contains(cpk) {
                let path = download.join(cpk);
                match fs::remove_file(&path) {
                    Ok(()) => code = Code::DpfilelistCpkDeleted,
                    Err(error) => context.push(("error", error.to_string())),
                }
            }
        }
        events.message(finding(code, context));
    }
    if !apply {
        let command = ("command", format!("{UPGRADE_COMMAND} --yes"));
        events.message(finding(Code::DpfilelistUpgradePlanned, vec![command]));
    }
    Ok(events.worst())
}

/// One of the command's findings: about the run, nothing dropped by it.
fn finding(code: Code, context: Vec<(&'static str, String)>) -> Message {
    tool_message(code, Scope::Run, Disposition::Keep, context)
}

/// The bytes of the installed list at `path`; `None` when there is no file.
fn read_list_file(path: &Path) -> anyhow::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("cannot read the installed list {}", path.display()))
        }
    }
}

/// The size of `download`'s file `cpk` in bytes; `None` when the file is not `present` (a
/// dropped entry without a file has no size).
fn dropped_size(
    download: &Path,
    present: &BTreeSet<String>,
    cpk: &str,
) -> anyhow::Result<Option<u64>> {
    if !present.contains(cpk) {
        return Ok(None);
    }
    let path = download.join(cpk);
    let size = fs::metadata(&path)
        .with_context(|| format!("cannot read the size of {}", path.display()))?
        .len();
    Ok(Some(size))
}

/// The names of the entries of `folder`, as they are spelled.
fn file_names(folder: &Path) -> anyhow::Result<BTreeSet<String>> {
    let cannot_list = || format!("cannot list the files in {}", folder.display());
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(folder).with_context(cannot_list)? {
        // A name that is not Unicode matches no list entry, lossy or not.
        let name = entry.with_context(cannot_list)?.file_name();
        names.insert(name.to_string_lossy().into_owned());
    }
    Ok(names)
}

/// Renames the CPK `from` to `to` in `download`, unless a file took the name `to` since the
/// folder was listed: `fs::rename` would replace it.
fn rename(download: &Path, from: &str, to: &str) -> anyhow::Result<()> {
    let source = download.join(from);
    let target = download.join(to);
    let exists = target
        .try_exists()
        .with_context(|| format!("cannot tell whether {} exists", target.display()))?;
    if exists {
        bail!(
            "cannot rename {from} to {}, because a file of that name already exists",
            target.display()
        );
    }
    fs::rename(&source, &target)
        .with_context(|| format!("cannot rename {} to {to}", source.display()))
}

/// Writes `bytes` as the new file `name` in `download`, never over an existing one and never
/// half-written under its name: the bytes land in a temporary sibling first, renamed into
/// place by `rename`, which refuses a name taken since the listing. A run killed mid-write
/// leaves only the temporary, which the game does not read; a file cut short under the CPK's
/// own name would be read, and kept by every later run, which writes no name that exists.
/// A rename rather than a hard link, which is atomic and refuses a taken name by itself:
/// FAT32 and exFAT, where a game folder may sit, have no hard links.
fn write_new(download: &Path, name: &str, bytes: &[u8]) -> anyhow::Result<()> {
    let temporary = format!(".{name}-{}.tmp", std::process::id());
    let written = fs::write(download.join(&temporary), bytes)
        .with_context(|| format!("cannot write {}", download.join(name).display()))
        .and_then(|()| rename(download, &temporary, name));
    if written.is_err()
        && let Err(removal) = fs::remove_file(download.join(&temporary))
    {
        log::debug!("{temporary}: kept: {removal}");
    }
    written
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use studio_core::PipelineEvent;

    use super::*;
    use crate::templates::Templates;
    use crate::testing::{dpfilelist, scratch, tool_context};

    /// The entries of the PES 2017 install's `DpFileList.bin` (`examples/DpFileList.bin`).
    fn pes17_list() -> Vec<String> {
        let bytes = fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/DpFileList.bin"),
        )
        .unwrap();
        cpk::dp_file_list::read(&bytes).unwrap()
    }

    /// TC-DEP-09's install with a file for every listed CPK: the PES 2017 list in
    /// `download`, and a `kib_of` file per entry of it.
    fn install_pes17_full(download: &Path) {
        fs::create_dir_all(download).unwrap();
        let names = pes17_list();
        let borrowed: Vec<&str> = names.iter().map(String::as_str).collect();
        fs::write(download.join(DPFILELIST), dpfilelist(&borrowed)).unwrap();
        for name in &names {
            fs::write(download.join(name), kib_of(name)).unwrap();
        }
    }

    /// A 1 KiB file of its own bytes: `name`'s bytes, repeated.
    fn kib_of(name: &str) -> Vec<u8> {
        name.bytes().cycle().take(1024).collect()
    }

    /// The files of `folder`, by name, with their bytes (the tests' folders hold no
    /// subfolders).
    fn snapshot(folder: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut files = BTreeMap::new();
        for entry in fs::read_dir(folder).unwrap() {
            let entry = entry.unwrap();
            files.insert(entry.file_name().into(), fs::read(entry.path()).unwrap());
        }
        files
    }

    /// `message` as the line the console prints for it: severity, code, disposition and
    /// context, so a test asserts all of them at once.
    fn line(message: &Message) -> String {
        let context: Vec<String> = message
            .context
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        format!(
            "{:?} {} [{:?}] ({})",
            message.severity,
            message.code.code,
            message.disposition,
            context.join(", ")
        )
    }

    fn strings(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    /// `(from, to)` pairs of `4cc_NN_stem` names, `.cpk` added.
    fn renames(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(from, to)| (format!("{from}.cpk"), format!("{to}.cpk")))
            .collect()
    }

    /// `4cc_{number}_{stem}.cpk` for each number of `numbers`.
    fn run_of(stem: &str, numbers: std::ops::RangeInclusive<u8>) -> Vec<String> {
        numbers
            .map(|number| format!("4cc_{number:02}_{stem}.cpk"))
            .collect()
    }

    #[test]
    fn the_pes_17_list_s_dlc_takes_the_official_names_of_its_stems() {
        let installed = pes17_list();
        let official = Templates::embedded().official_list();
        let present: BTreeSet<String> = installed.iter().cloned().collect();

        let plan = plan(&installed, &official, &present);

        assert_eq!(
            plan.renames,
            renames(&[
                ("4cc_15_billboard", "4cc_11_billboard"),
                ("4cc_20_swipe", "4cc_12_swipe"),
                ("4cc_25_gametips", "4cc_14_gametips"),
                ("4cc_32_stadiums2", "4cc_22_stadiums"),
                ("4cc_31_stadiums1", "4cc_21_stadiums"),
                ("4cc_30_stadiums0", "4cc_20_stadiums"),
                ("4cc_35_referees", "4cc_18_referees"),
                ("4cc_38_balls", "4cc_16_balls"),
                ("4cc_45_uniform", "4cc_42_teams"),
                ("4cc_40_faces", "4cc_41_teams"),
                ("4cc_55_other_uniform", "4cc_52_teams2"),
                ("4cc_50_other_faces", "4cc_51_teams2"),
                ("4cc_90_test", "4cc_99_test"),
            ])
        );
        // Twenty midcups, 60 to 79, take the fifteen names 61 to 75: 74's rename to 75 is
        // blocked by 4cc_75_midcup, which has no name of its own to go to, and each rename
        // before it is blocked in turn, so none is made.
        let mut dropped = strings(&["4cc_60_midcup.cpk"]);
        dropped.extend(run_of("midcup", 76..=79));
        assert_eq!(plan.dropped, dropped);
        let mut placeholders = run_of("stadiums", 23..=35);
        placeholders.extend(run_of("teams", 43..=45));
        placeholders.extend(run_of("teams2", 53..=55));
        assert_eq!(plan.placeholders, placeholders);
    }

    // TC-DEP-09's install: only the faces CPK of its list is in `download/`.
    #[test]
    fn only_a_present_file_is_renamed_and_every_other_unofficial_entry_is_dropped() {
        let installed = pes17_list();
        let official = Templates::embedded().official_list();
        let present = BTreeSet::from(["4cc_40_faces.cpk".to_owned()]);

        let plan = plan(&installed, &official, &present);

        assert_eq!(plan.renames, renames(&[("4cc_40_faces", "4cc_41_teams")]));
        let unofficial: Vec<String> = installed
            .iter()
            .filter(|entry| !official.contains(entry) && *entry != "4cc_40_faces.cpk")
            .cloned()
            .collect();
        assert_eq!(unofficial.len(), 17);
        assert_eq!(plan.dropped, unofficial);
        let placeholders: Vec<String> = official
            .iter()
            .filter(|entry| *entry != "4cc_41_teams.cpk")
            .cloned()
            .collect();
        assert_eq!(plan.placeholders, placeholders);
    }

    // TC-DEP-13's install.
    #[test]
    fn a_stem_s_files_already_under_official_names_shift_with_the_others() {
        let installed = strings(&[
            "4cc_38_balls.cpk",
            "4cc_40_faces.cpk",
            "4cc_45_uniform.cpk",
            "4cc_60_midcup.cpk",
            "4cc_61_midcup.cpk",
            "4cc_86_mine.cpk",
        ]);
        let official = Templates::embedded().official_list();
        let present: BTreeSet<String> = installed.iter().cloned().collect();

        let plan = plan(&installed, &official, &present);

        assert_eq!(
            plan.renames,
            renames(&[
                ("4cc_38_balls", "4cc_16_balls"),
                ("4cc_45_uniform", "4cc_42_teams"),
                ("4cc_40_faces", "4cc_41_teams"),
                ("4cc_61_midcup", "4cc_62_midcup"),
                ("4cc_60_midcup", "4cc_61_midcup"),
            ])
        );
        assert_eq!(plan.dropped, strings(&["4cc_86_mine.cpk"]));
        let taken = [
            "4cc_16_balls.cpk",
            "4cc_41_teams.cpk",
            "4cc_42_teams.cpk",
            "4cc_61_midcup.cpk",
            "4cc_62_midcup.cpk",
        ];
        let placeholders: Vec<String> = official
            .iter()
            .filter(|entry| !taken.contains(&entry.as_str()))
            .cloned()
            .collect();
        assert_eq!(placeholders.len(), 48);
        assert_eq!(plan.placeholders, placeholders);
    }

    #[test]
    fn a_target_held_by_a_file_the_list_does_not_name_blocks_its_rename() {
        let official = strings(&["4cc_61_midcup.cpk", "4cc_62_midcup.cpk"]);
        let installed = strings(&["4cc_60_midcup.cpk"]);
        // `4cc_61_midcup.cpk` is in `download/` but not in the installed list.
        let present = BTreeSet::from([
            "4cc_60_midcup.cpk".to_owned(),
            "4cc_61_midcup.cpk".to_owned(),
        ]);

        let plan = plan(&installed, &official, &present);

        assert_eq!(plan.renames, []);
        assert_eq!(plan.dropped, strings(&["4cc_60_midcup.cpk"]));
        assert_eq!(plan.placeholders, strings(&["4cc_62_midcup.cpk"]));
    }

    #[test]
    fn a_name_with_no_stem_shape_is_dropped() {
        let official = strings(&["4cc_61_midcup.cpk"]);
        let installed = strings(&["midcup.cpk", "4cc_x_midcup.cpk", "4cc_61_midcup"]);
        let present: BTreeSet<String> = installed.iter().cloned().collect();

        let plan = plan(&installed, &official, &present);

        assert_eq!(plan.renames, []);
        assert_eq!(plan.dropped, installed);
        assert_eq!(plan.placeholders, official);
    }

    #[test]
    fn a_stem_is_what_follows_the_prefix_and_the_number() {
        assert_eq!(stem("4cc_30_stadiums0.cpk"), Some("stadiums0"));
        assert_eq!(stem("4cc_55_other_uniform.cpk"), Some("other_uniform"));
        for shapeless in [
            "4cc_30_stadiums0",
            "4cc_3a_stadiums.cpk",
            "4cc__stadiums.cpk",
            "_30_stadiums.cpk",
            "4cc_30_.cpk",
            "4cc_30.cpk",
        ] {
            assert_eq!(stem(shapeless), None, "{shapeless}");
        }
    }

    #[test]
    fn a_new_file_is_written_and_an_existing_one_never_overwritten() {
        let temp = scratch("upgrade_write_new");
        let download = temp.path();
        let path = download.join("4cc_61_midcup.cpk");
        write_new(download, "4cc_61_midcup.cpk", b"placeholder").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"placeholder");

        let error = write_new(download, "4cc_61_midcup.cpk", b"another").unwrap_err();
        assert!(
            format!("{error:#}").ends_with("a file of that name already exists"),
            "{error:#}"
        );
        assert_eq!(fs::read(&path).unwrap(), b"placeholder");
        // Neither write leaves its temporary behind.
        assert_eq!(
            file_names(download).unwrap(),
            BTreeSet::from(["4cc_61_midcup.cpk".to_owned()])
        );
    }

    #[test]
    fn a_placeholder_is_written_under_a_temporary_name_first() {
        // A kill mid-write leaves what `fs::write` had written; it must not be under the
        // CPK's name, which the game reads and no later run writes again. The temporary this
        // run would use is made a folder, so the write fails before any byte is written:
        // the CPK's name stays free.
        let temp = scratch("upgrade_write_new_temporary");
        let download = temp.path();
        let temporary = format!(".4cc_61_midcup.cpk-{}.tmp", std::process::id());
        fs::create_dir(download.join(&temporary)).unwrap();

        assert!(write_new(download, "4cc_61_midcup.cpk", b"placeholder").is_err());

        assert!(!download.join("4cc_61_midcup.cpk").exists());
    }

    #[test]
    fn a_rename_onto_a_file_that_appeared_since_the_listing_is_refused() {
        let temp = scratch("upgrade_rename_held");
        let download = temp.path();
        fs::write(download.join("4cc_40_faces.cpk"), b"faces").unwrap();
        fs::write(download.join("4cc_41_teams.cpk"), b"teams").unwrap();

        let error = rename(download, "4cc_40_faces.cpk", "4cc_41_teams.cpk").unwrap_err();

        assert_eq!(
            format!("{error:#}"),
            format!(
                "cannot rename 4cc_40_faces.cpk to {}, because a file of that name already \
                 exists",
                download.join("4cc_41_teams.cpk").display()
            )
        );
        assert_eq!(
            fs::read(download.join("4cc_40_faces.cpk")).unwrap(),
            b"faces"
        );
        assert_eq!(
            fs::read(download.join("4cc_41_teams.cpk")).unwrap(),
            b"teams"
        );

        fs::remove_file(download.join("4cc_41_teams.cpk")).unwrap();
        rename(download, "4cc_40_faces.cpk", "4cc_41_teams.cpk").unwrap();
        assert_eq!(
            fs::read(download.join("4cc_41_teams.cpk")).unwrap(),
            b"faces"
        );
        assert!(!download.join("4cc_40_faces.cpk").exists());
    }

    // TC-DEP-09's install with every listed CPK's file given: what the dialog shows.
    #[test]
    fn the_preview_lists_what_the_upgrade_would_do() {
        let temp = scratch("upgrade_preview");
        let download = temp.path().join("PES/download");
        install_pes17_full(&download);
        let ctx = tool_context(temp.path(), "");

        let preview = preview(&download, &ctx).unwrap();

        assert!(!preview.up_to_date);
        assert!(
            preview.replaces_list,
            "the PES 17 list is not the official one"
        );
        assert_eq!(
            preview.renames,
            renames(&[
                ("4cc_15_billboard", "4cc_11_billboard"),
                ("4cc_20_swipe", "4cc_12_swipe"),
                ("4cc_25_gametips", "4cc_14_gametips"),
                ("4cc_32_stadiums2", "4cc_22_stadiums"),
                ("4cc_31_stadiums1", "4cc_21_stadiums"),
                ("4cc_30_stadiums0", "4cc_20_stadiums"),
                ("4cc_35_referees", "4cc_18_referees"),
                ("4cc_38_balls", "4cc_16_balls"),
                ("4cc_45_uniform", "4cc_42_teams"),
                ("4cc_40_faces", "4cc_41_teams"),
                ("4cc_55_other_uniform", "4cc_52_teams2"),
                ("4cc_50_other_faces", "4cc_51_teams2"),
                ("4cc_90_test", "4cc_99_test"),
            ])
        );
        assert_eq!(preview.placeholders, 19);
        let mut dropped = vec![("4cc_60_midcup.cpk".to_owned(), Some(1024))];
        dropped.extend((76..=79).map(|number| (format!("4cc_{number}_midcup.cpk"), Some(1024))));
        assert_eq!(preview.dropped, dropped);
    }

    #[test]
    fn the_preview_writes_nothing() {
        let temp = scratch("upgrade_preview_readonly");
        let download = temp.path().join("PES/download");
        install_pes17_full(&download);
        let before = snapshot(&download);
        let ctx = tool_context(temp.path(), "");

        preview(&download, &ctx).unwrap();

        assert!(snapshot(&download) == before, "nothing written");
    }

    /// An install on the official list byte for byte, with a `kib_of` file for every
    /// official entry but `missing` (`""`: none missing).
    fn install_official(download: &Path, missing: &str) {
        fs::create_dir_all(download).unwrap();
        let templates = Templates::embedded();
        fs::write(download.join(DPFILELIST), templates.official_list_file()).unwrap();
        for name in templates.official_list() {
            if name != missing {
                fs::write(download.join(&name), kib_of(&name)).unwrap();
            }
        }
    }

    // An install holding the official list with one official CPK missing: the upgrade writes
    // its placeholder and replaces nothing.
    #[test]
    fn a_preview_of_the_official_list_with_a_missing_cpk_replaces_no_list() {
        let temp = scratch("upgrade_preview_same_list");
        let download = temp.path().join("PES/download");
        install_official(&download, "4cc_16_balls.cpk");
        let ctx = tool_context(temp.path(), "");

        let preview = preview(&download, &ctx).unwrap();

        assert!(!preview.up_to_date);
        assert!(
            !preview.replaces_list,
            "the installed list is the official one"
        );
        assert_eq!(preview.renames, []);
        assert_eq!(preview.placeholders, 1);
        assert_eq!(preview.dropped, []);
    }

    // An install already upgraded: the official list and a file for every entry of it.
    #[test]
    fn a_preview_of_an_upgraded_install_has_nothing_to_do() {
        let temp = scratch("upgrade_preview_up_to_date");
        let download = temp.path().join("PES/download");
        install_official(&download, "");
        let ctx = tool_context(temp.path(), "");

        let preview = preview(&download, &ctx).unwrap();

        assert!(preview.up_to_date);
        assert!(!preview.replaces_list);
        assert_eq!(preview.renames, []);
        assert_eq!(preview.placeholders, 0);
        assert_eq!(preview.dropped, []);
    }

    // `delete` is the dialog's ticked CPKs; without `apply` the run writes and deletes
    // nothing, whatever it holds.
    #[test]
    fn a_run_without_apply_deletes_nothing() {
        let temp = scratch("upgrade_no_apply_delete");
        let download = temp.path().join("PES/download");
        install_pes17_full(&download);
        let before = snapshot(&download);
        let (events_tx, events) = crossbeam_channel::unbounded();
        let ctx = tool_context(temp.path(), "").with_events(events_tx);
        let delete = BTreeSet::from(["4cc_60_midcup.cpk".to_owned()]);

        let worst = run(&download, false, &delete, &ctx).unwrap();

        assert!(snapshot(&download) == before, "nothing written or deleted");
        let lines: Vec<String> = events
            .try_iter()
            .filter_map(|envelope| {
                let PipelineEvent::Message(message) = envelope.event else {
                    return None;
                };
                Some(line(&message))
            })
            .collect();
        assert!(
            lines.iter().any(|line| line
                == "Warning dpfilelist_cpk_dropped [Keep] (cpk=4cc_60_midcup.cpk, size=1 KiB)"),
            "{lines:?}"
        );
        assert_eq!(worst, Some(Severity::Warning));
    }

    // The dialog's `delete` set: `4cc_60_midcup.cpk` ticked (a dropped CPK, deleted) and
    // `4cc_16_balls.cpk` (an official name, not dropped, ignored).
    #[test]
    fn a_ticked_dropped_cpk_is_deleted_and_the_others_are_left() {
        let temp = scratch("upgrade_delete");
        let download = temp.path().join("PES/download");
        install_pes17_full(&download);
        let (events_tx, events) = crossbeam_channel::unbounded();
        let ctx = tool_context(temp.path(), "").with_events(events_tx);
        let delete = BTreeSet::from([
            "4cc_60_midcup.cpk".to_owned(),
            "4cc_16_balls.cpk".to_owned(),
        ]);

        let worst = run(&download, true, &delete, &ctx).unwrap();

        // The four `dropped` findings are the run's worst.
        assert_eq!(worst, Some(Severity::Warning));
        let lines: Vec<String> = events
            .try_iter()
            .filter_map(|envelope| {
                let PipelineEvent::Message(message) = envelope.event else {
                    return None;
                };
                Some(line(&message))
            })
            .collect();
        // 13 renames, 19 placeholders, the list's replacement, then the five dropped
        // entries: the ticked one deleted, the others still dropped.
        assert_eq!(lines.len(), 38);
        assert_eq!(
            lines[32..],
            [
                format!(
                    "Info dpfilelist_replaced [Keep] (path={}, backup={})",
                    download.join("DpFileList.bin").display(),
                    download.join("DpFileList.bin.bak").display()
                ),
                "Info dpfilelist_cpk_deleted [Keep] (cpk=4cc_60_midcup.cpk, size=1 KiB)".to_owned(),
                "Warning dpfilelist_cpk_dropped [Keep] (cpk=4cc_76_midcup.cpk, size=1 KiB)"
                    .to_owned(),
                "Warning dpfilelist_cpk_dropped [Keep] (cpk=4cc_77_midcup.cpk, size=1 KiB)"
                    .to_owned(),
                "Warning dpfilelist_cpk_dropped [Keep] (cpk=4cc_78_midcup.cpk, size=1 KiB)"
                    .to_owned(),
                "Warning dpfilelist_cpk_dropped [Keep] (cpk=4cc_79_midcup.cpk, size=1 KiB)"
                    .to_owned(),
            ]
        );
        assert!(!download.join("4cc_60_midcup.cpk").exists());
        for number in 76..=79 {
            let name = format!("4cc_{number}_midcup.cpk");
            assert_eq!(fs::read(download.join(&name)).unwrap(), kib_of(&name));
        }
        assert!(download.join("4cc_16_balls.cpk").exists());
    }

    // A ticked dropped CPK whose file cannot be deleted (PES may hold it open) is reported
    // dropped with the error, and the run goes on to the other dropped CPKs. The file is
    // held open with no sharing, which a plain `File::open` does not block a delete under.
    #[cfg(windows)]
    #[test]
    fn a_ticked_cpk_whose_delete_fails_is_reported_dropped_with_the_error() {
        use std::os::windows::fs::OpenOptionsExt;

        let temp = scratch("upgrade_delete_failed");
        let download = temp.path().join("PES/download");
        install_pes17_full(&download);
        let _held = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(download.join("4cc_60_midcup.cpk"))
            .unwrap();
        let (events_tx, events) = crossbeam_channel::unbounded();
        let ctx = tool_context(temp.path(), "").with_events(events_tx);
        let delete = BTreeSet::from(["4cc_60_midcup.cpk".to_owned()]);

        let worst = run(&download, true, &delete, &ctx).unwrap();

        assert_eq!(worst, Some(Severity::Warning));
        let lines: Vec<String> = events
            .try_iter()
            .filter_map(|envelope| {
                let PipelineEvent::Message(message) = envelope.event else {
                    return None;
                };
                Some(line(&message))
            })
            .collect();
        let held = lines
            .iter()
            .find(|line| line.contains("cpk=4cc_60_midcup.cpk"))
            .expect("the held CPK's line");
        assert!(
            held.starts_with(
                "Warning dpfilelist_cpk_dropped [Keep] (cpk=4cc_60_midcup.cpk, size=1 KiB, error="
            ),
            "{held}"
        );
        for number in 76..=79 {
            let name = format!("cpk=4cc_{number}_midcup.cpk");
            assert!(
                lines.iter().any(|line| line.contains(&name)),
                "the next dropped CPKs still get their lines: {name}"
            );
        }
        assert!(download.join("4cc_60_midcup.cpk").exists(), "kept");
    }
}
