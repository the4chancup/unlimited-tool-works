//! The `upgrade-dpfl` command (`spec/team_compiler/pipeline.md` "DpFileList upgrade"): the
//! installed `DpFileList.bin` replaced by the cup's official one, after an old DLC's CPKs are
//! renamed to the official names of their stem, so they keep loading. The placeholder CPK is
//! written for every official entry with no file, since the game loads none of `download/`
//! when a listed CPK is missing. Without `--yes` it
//! lists what it would do and writes nothing. It never deletes or overwrites a CPK.

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
use crate::templates;

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

/// The `upgrade-dpfl` command in `download`, the PES folder's `download/` (checked to be a
/// folder). It reports the upgrade's findings (`plan`) in the order `apply` (`--yes`) acts,
/// each emitted once its action succeeded (renames, placeholders, the list, then the dropped
/// entries). Without `apply` it reports `dpfilelist_upgrade_planned` last and writes nothing;
/// `dpfilelist_up_to_date` alone when there is nothing to do. Returns the worst severity
/// reported. A file that cannot be read or written is the error, naming it, and what was done
/// before stays done: the list is replaced last, so the old one stays in place and a second
/// run finishes the work.
pub(crate) fn run(
    download: &Path,
    apply: bool,
    ctx: &ToolContext,
) -> anyhow::Result<Option<Severity>> {
    let mut events = RunEvents::new(ctx, false);
    let Some(templates) = templates::read_reported(ctx, &mut events) else {
        return Ok(events.worst());
    };
    let list_path = download.join(DPFILELIST);
    let old_list = read_list_file(&list_path)?;
    // A list that does not read as a list is replaced like any other, with nothing renamed.
    let (installed, unreadable) = match old_list.as_deref().map(cpk::dp_file_list::read) {
        None => (Vec::new(), None),
        Some(Ok(entries)) => (entries, None),
        Some(Err(error)) => (Vec::new(), Some(format!("{error:#}"))),
    };
    let present = file_names(download)?;
    let plan = plan(&installed, &templates.official_list(), &present);
    let official_file = templates.official_list_file();
    let list_is_official = old_list.as_deref() == Some(official_file);
    if list_is_official && plan.renames.is_empty() && plan.placeholders.is_empty() {
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
            replace_file(&list_path, official_file)?;
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
        if present.contains(cpk) {
            let path = download.join(cpk);
            let size = fs::metadata(&path)
                .with_context(|| format!("cannot read the size of {}", path.display()))?
                .len();
            context.push(("size", size_text(size)));
        }
        events.message(finding(Code::DpfilelistCpkDropped, context));
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
    use super::*;
    use crate::templates::Templates;
    use crate::testing::scratch;

    /// The entries of the PES 2017 install's `DpFileList.bin` (`examples/DpFileList.bin`).
    fn pes17_list() -> Vec<String> {
        let bytes = fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/DpFileList.bin"),
        )
        .unwrap();
        cpk::dp_file_list::read(&bytes).unwrap()
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
}
