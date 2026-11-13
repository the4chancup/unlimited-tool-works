//! The teams list as `export-teams-list` sees it (`spec/save_editor.md` "CLI"): the save's
//! team names and ids as an incoming list, the working `teams_list.txt` of the data
//! directory (the embedded upstream list when there is none yet), and the write of the
//! merged list. The merge itself is `teams_list::reconcile`.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use anyhow::anyhow;
use pes_savefile::EditFile;
use pes_savefile::model::names::display_name;
use teams_list::{TeamName, TeamsList};

use crate::session::cup::cup_teams;

// The merge, re-exported so the command reaches the lib's through this module: inside the
// crate the module's name shadows the lib's.
pub(crate) use teams_list::reconcile;

/// The file's name in the data directory.
pub(crate) const FILE_NAME: &str = "teams_list.txt";

/// A team of the save left out of its list because an earlier team (in record order) folds
/// to the same name: two rows of one name cannot form a list, and the first stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NameDuplicate {
    /// The team left out.
    pub(crate) team: u32,
    /// The team kept under the name.
    pub(crate) kept: u32,
    /// The folded name (`/4ccg/`).
    pub(crate) name: TeamName,
}

/// The save's teams as a teams list: every team whose name is slash-wrapped (`/co/`, the
/// list's own convention for a team row), folds to a team name and whose id is in the cup
/// range, by id and name with its colour codes stripped. The other teams (the game's own
/// clubs, a `Backup 1` slot) are no cup identity and are left out. A later team folding to
/// an earlier one's name (the fold is case-insensitive) is left out too, and returned as a
/// `NameDuplicate`: a real cup save holds `/4ccg/` twice.
pub(crate) fn from_save(file: &EditFile) -> (TeamsList, Vec<NameDuplicate>) {
    let mut text = String::from("ID\tName\r\n");
    let mut seen: BTreeMap<TeamName, u32> = BTreeMap::new();
    let mut duplicates = Vec::new();
    for team in cup_teams(file) {
        let display = display_name(&team.name);
        let trimmed = display.trim();
        let slash_wrapped =
            trimmed.len() >= 3 && trimmed.starts_with('/') && trimmed.ends_with('/');
        if !slash_wrapped {
            continue;
        }
        let Ok(name) = TeamName::new(trimmed) else {
            continue;
        };
        match seen.get(&name) {
            Some(&kept) => duplicates.push(NameDuplicate {
                team: team.id,
                kept,
                name,
            }),
            None => {
                text.push_str(&format!("{}\t{trimmed}\r\n", team.id));
                seen.insert(name, team.id);
            }
        }
    }
    let list = TeamsList::parse(&text).expect(
        "one row per folded name with a cup id in range is what the file's own rules admit",
    );
    (list, duplicates)
}

/// The working list at `path`: the file when there is one, else the embedded upstream list
/// (what a first run merges into, and all there is with no data directory chosen); a file
/// that cannot be read or parsed is an error naming it. A path under a file is "no list
/// there" too: Linux reports it as `NotADirectory` where Windows reports `NotFound`, and
/// the write then refuses it on both.
pub(crate) fn load_working(path: Option<&Path>) -> anyhow::Result<TeamsList> {
    let Some(path) = path else {
        return Ok(TeamsList::parse(TeamsList::UPSTREAM).expect("the embedded list parses"));
    };
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            TeamsList::UPSTREAM.to_owned()
        }
        Err(error) => {
            return Err(anyhow!(
                "{}: cannot read the teams list: {error}",
                path.display()
            ));
        }
    };
    TeamsList::parse(&text)
        .map_err(|error| anyhow!("{}: not a valid teams list: {error}", path.display()))
}

/// Writes `list` at `path` through a sibling temporary file renamed over it, so the file
/// holds the previous list or the whole new one, never a half-written one the Team compiler
/// would then read.
pub(crate) fn write(path: &Path, list: &TeamsList) -> io::Result<()> {
    let temporary = path.with_extension("txt.tmp");
    fs::write(&temporary, list.write())?;
    if let Err(error) = fs::rename(&temporary, path) {
        // The rename's error is the one to report; the temporary file is cleaned up best
        // effort, a leftover being a diagnostic rather than a finding.
        if let Err(removal) = fs::remove_file(&temporary) {
            log::debug!("{}: kept: {removal}", temporary.display());
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use teams_list::TeamId;

    use super::*;

    /// A cup-range team named outside the slash convention is no cup identity, even one the
    /// fold accepts (`Backup`, one word): it does not take the name from a later team
    /// actually called `/backup/`.
    #[test]
    fn a_team_without_slashes_does_not_take_a_later_teams_name() {
        let mut file =
            EditFile::from_bytes(&pes_savefile::fixture_save(pes_version::PesVersion::Pes21))
                .unwrap();
        file.team_mut(707).unwrap().name = "Backup".to_owned();
        file.team_mut(708).unwrap().name = "/backup/".to_owned();
        let (list, duplicates) = from_save(&file);
        assert_eq!(
            list.id_of(&TeamName::new("/backup/").unwrap()),
            Some(TeamId::new(708).unwrap())
        );
        assert!(
            duplicates.iter().all(|duplicate| duplicate.team != 708),
            "{duplicates:?}"
        );
    }

    /// A cup-range team named with a leading slash only (`/backup`, no
    /// trailing one) is not slash-wrapped: it does not fold to `/backup/`,
    /// which a later team actually called `/backup/` takes.
    #[test]
    fn a_leading_slash_without_a_trailing_one_is_not_listed() {
        let mut file =
            EditFile::from_bytes(&pes_savefile::fixture_save(pes_version::PesVersion::Pes21))
                .unwrap();
        file.team_mut(707).unwrap().name = "/backup".to_owned();
        file.team_mut(708).unwrap().name = "/backup/".to_owned();
        let (list, duplicates) = from_save(&file);
        assert_eq!(
            list.id_of(&TeamName::new("/backup/").unwrap()),
            Some(TeamId::new(708).unwrap())
        );
        assert!(
            duplicates.iter().all(|duplicate| duplicate.team != 708),
            "{duplicates:?}"
        );
    }

    #[test]
    fn a_missing_file_loads_as_the_upstream_list() {
        let dir = crate::test_support::scratch("teams_list_missing");
        let list = load_working(Some(&dir.join("absent.txt"))).unwrap();
        assert_eq!(list, TeamsList::parse(TeamsList::UPSTREAM).unwrap());
    }

    // A `teams_list.txt` path under a regular file is "no list there" too:
    // Linux reports it as `NotADirectory` (the arm this kills, so the
    // kill is Linux-only), Windows as `NotFound`.
    #[test]
    fn a_teams_list_path_under_a_file_falls_back_to_upstream() {
        let dir = crate::test_support::scratch("teams_list_under_a_file");
        let file = dir.join("data");
        fs::write(&file, "a file, not a folder").unwrap();
        assert_eq!(
            load_working(Some(&file.join(FILE_NAME))).unwrap(),
            TeamsList::parse(TeamsList::UPSTREAM).unwrap()
        );
    }

    /// With no data directory, a `teams_list.txt` in the current directory is not read: the
    /// working list is the upstream one.
    #[test]
    fn no_data_directory_loads_the_upstream_list_whatever_the_current_directory_holds() {
        let here = Path::new(FILE_NAME);
        assert!(
            !here.exists(),
            "the test's current directory holds no teams list"
        );
        fs::write(here, "ID\tName\n714\t/co/\n").unwrap();
        let list = load_working(None);
        fs::remove_file(here).unwrap();
        assert_eq!(
            list.unwrap(),
            TeamsList::parse(TeamsList::UPSTREAM).unwrap()
        );
    }

    #[test]
    fn a_write_replaces_the_file_whole_and_leaves_no_temporary() {
        let dir = crate::test_support::scratch("teams_list_write");
        let path = dir.join("written.txt");
        fs::write(&path, "stale").unwrap();
        let list = TeamsList::parse("ID\tName\n714\t/co/\n").unwrap();
        write(&path, &list).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "ID\tName\r\n714\t/co/\r\n"
        );
        assert!(!dir.join("written.txt.tmp").exists());
    }
}
