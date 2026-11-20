//! `export-teams-list`: the save's team names and ids merged into the data directory's
//! `teams_list.txt`.

use std::fs;

use pes_savefile::EditFile;
use pes_version::PesVersion;
use teams_list::{TeamId, TeamName, TeamsList};

use crate::{Sandbox, TEST_SALT};

/// A working list of three rows of the bundled upstream list (`/a/` 702, `/b/` 707, `/jp/`
/// 731), plus `/co/` listed under 715 where the save has it at 714; every other team of the
/// save (`/3/` 701 among them) is missing from it.
const WORKING: &str = "ID\tName\r\n702\t/a/\r\n707\t/b/\r\n715\t/co/\r\n731\t/jp/\r\n";

impl Sandbox {
    /// `export-teams-list <save> <flags>`.
    fn export_teams_list(&self, save: &str, flags: &[&str]) -> crate::common::Run {
        let save = self.arg(save);
        let mut args = vec!["export-teams-list", &save];
        args.extend(flags);
        self.run(&args)
    }

    /// The parsed `data/teams_list.txt`.
    fn working_list(&self) -> TeamsList {
        TeamsList::parse(&fs::read_to_string(self.path("data/teams_list.txt")).unwrap()).unwrap()
    }

    /// The PES 21 fixture written at `relative` after `edit`, with every later team whose
    /// name folds to an earlier one's renamed out of the list's convention (`Backup <id>`),
    /// so its slash-wrapped names are unique. The save as written.
    fn unique_names_fixture(&self, relative: &str, edit: impl FnOnce(&mut EditFile)) -> EditFile {
        let mut file =
            EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        edit(&mut file);
        let mut seen = Vec::new();
        for team in file.teams_mut() {
            match TeamName::new(&team.name) {
                Ok(name) if team.name.starts_with('/') && !seen.contains(&name) => seen.push(name),
                Ok(_) if team.name.starts_with('/') => team.name = format!("Backup {}", team.id),
                Ok(_) | Err(_) => {}
            }
        }
        fs::write(self.path(relative), file.to_bytes(&TEST_SALT).unwrap()).unwrap();
        file
    }
}

fn id(number: u16) -> TeamId {
    TeamId::new(number).unwrap()
}

fn name(token: &str) -> TeamName {
    TeamName::new(token).unwrap()
}

/// The teams of `save` the list takes: slash-wrapped names.
fn listed_teams(save: &EditFile) -> usize {
    save.teams()
        .iter()
        .filter(|team| team.name.starts_with('/'))
        .count()
}

// SE-TL-01
#[test]
fn the_summary_is_printed_and_the_file_written_only_with_yes() {
    let sandbox = Sandbox::new("teams_list_merge");
    let save = sandbox.unique_names_fixture("EDIT", |_| {});
    fs::write(sandbox.path("data/teams_list.txt"), WORKING).unwrap();
    assert_eq!(save.team(714).unwrap().name, "/co/");
    assert_eq!(save.team(701).unwrap().name, "/3/");
    let listed = listed_teams(&save);
    assert!(
        listed < save.teams().len(),
        "the fixture holds backup slots"
    );
    let path = sandbox.path("data/teams_list.txt").display().to_string();

    let run = sandbox.export_teams_list("EDIT", &[]);
    assert_eq!(run.exit_code(), 0);
    let messages = run.messages();
    assert!(
        messages.contains(&"Info teams_list_added (team=701, name=/3/)".to_owned()),
        "{messages:?}"
    );
    assert!(
        messages.contains(&"Info teams_list_overridden (name=/co/, old=715, new=714)".to_owned()),
        "{messages:?}"
    );
    assert!(
        messages.contains(&"Info teams_list_kept (count=3)".to_owned()),
        "{messages:?}"
    );
    assert_eq!(
        messages.last().unwrap(),
        &format!("Info teams_list_not_written (path={path})")
    );
    assert!(
        !messages.iter().any(|line| line.starts_with("Error")),
        "{messages:?}"
    );
    assert_eq!(
        fs::read_to_string(sandbox.path("data/teams_list.txt")).unwrap(),
        WORKING
    );

    let run = sandbox.export_teams_list("EDIT", &["--yes"]);
    assert_eq!(run.exit_code(), 0);
    assert_eq!(
        run.messages().last().unwrap(),
        &format!("Info teams_list_written (path={path})")
    );
    let list = sandbox.working_list();
    assert_eq!(
        list.id_of(&name("/co/")),
        Some(id(714)),
        "the save's id wins"
    );
    assert_eq!(list.id_of(&name("/3/")), Some(id(701)));
    assert_eq!(list.id_of(&name("/a/")), Some(id(702)));
    assert_eq!(
        list.teams().count(),
        listed,
        "every slash-wrapped team, no other row"
    );
}

// SE-TL-02
#[test]
fn a_change_that_would_give_two_teams_one_id_is_unresolved_and_the_rest_written() {
    let sandbox = Sandbox::new("teams_list_collision");
    fs::write(sandbox.path("data/teams_list.txt"), WORKING).unwrap();
    // The save's 702 is named `/b/`, which the list has at 707; its 707 gets a name the
    // list ignores, so `/b/` appears once in the save.
    sandbox.unique_names_fixture("EDIT", |file| {
        file.team_mut(702).unwrap().name = "/b/".to_owned();
        file.team_mut(707).unwrap().name = "Backup 707".to_owned();
    });

    let run = sandbox.export_teams_list("EDIT", &["--yes"]);
    assert_eq!(run.exit_code(), 1);
    let messages = run.messages();
    assert!(
        messages.contains(&"Error teams_list_unresolved (team=702, name=/b/)".to_owned()),
        "{messages:?}"
    );
    assert!(
        messages.contains(&"Info teams_list_added (team=701, name=/3/)".to_owned()),
        "{messages:?}"
    );
    assert!(
        messages
            .last()
            .unwrap()
            .starts_with("Info teams_list_written"),
        "{messages:?}"
    );
    let list = sandbox.working_list();
    assert_eq!(
        list.id_of(&name("/b/")),
        Some(id(707)),
        "the dropped change"
    );
    assert_eq!(list.id_of(&name("/a/")), Some(id(702)));
    assert_eq!(list.id_of(&name("/3/")), Some(id(701)), "the other changes");
}

// SE-TL-03
#[test]
fn an_unwritable_data_directory_is_reported_and_nothing_written() {
    let sandbox = Sandbox::new("teams_list_read_only");
    sandbox.unique_names_fixture("EDIT", |_| {});
    // A data directory whose parent is a file: nothing can be created under it.
    fs::write(sandbox.path("blocked"), b"a file, not a folder").unwrap();
    let data_dir = sandbox.path("blocked/data");
    let save = sandbox.arg("EDIT");

    let run = sandbox.run_with_data_dir(
        Some(data_dir.clone()),
        &["export-teams-list", &save, "--yes"],
    );
    assert_eq!(run.exit_code(), 1);
    let last = run.messages().last().unwrap().clone();
    assert!(
        last.starts_with(&format!(
            "Error teams_list_read_only (path={}, error=",
            data_dir.join("teams_list.txt").display()
        )),
        "{last}"
    );
    assert!(!data_dir.exists());

    let run = sandbox.run_with_data_dir(None, &["export-teams-list", &save, "--yes"]);
    assert_eq!(run.exit_code(), 1);
    assert_eq!(
        run.messages().last().unwrap(),
        "Error teams_list_read_only (path=teams_list.txt, error=no data directory is chosen yet)"
    );
}

/// A `teams_list.txt` that cannot be read (here a folder of that name) is refused naming it,
/// not taken for a first run: the upstream list is never merged over a list that exists.
// SE-TL-05
#[test]
fn an_unreadable_working_list_is_refused() {
    let sandbox = Sandbox::new("teams_list_unreadable");
    // The plain fixture, whose two `/4ccg/` teams would emit teams_list_name_duplicate
    // lines were the working list read after the duplicate check.
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    let save_bytes = fs::read(sandbox.path("EDIT")).unwrap();
    fs::create_dir(sandbox.path("data/teams_list.txt")).unwrap();

    let run = sandbox.export_teams_list("EDIT", &["--yes"]);
    run.assert_refused(2, &["teams_list.txt", "cannot read the teams list"]);
    assert_eq!(fs::read(sandbox.path("EDIT")).unwrap(), save_bytes);
    let data: Vec<_> = fs::read_dir(sandbox.path("data")).unwrap().collect();
    assert_eq!(data.len(), 1, "the data directory is unchanged: {data:?}");
    assert!(sandbox.path("data/teams_list.txt").is_dir());
}

// SE-TL-04
#[test]
fn a_later_team_of_a_name_the_save_holds_twice_is_left_out_as_an_error() {
    let sandbox = Sandbox::new("teams_list_duplicate_name");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    fs::write(sandbox.path("data/teams_list.txt"), WORKING).unwrap();
    let save = EditFile::load(&sandbox.path("EDIT")).unwrap();
    let four_ccg: Vec<u32> = save
        .teams()
        .iter()
        .filter(|team| TeamName::new(&team.name).ok() == Some(name("/4ccg/")))
        .map(|team| team.id)
        .collect();
    let [kept, dropped] = four_ccg[..] else {
        panic!("the fixture holds /4ccg/ twice: {four_ccg:?}");
    };

    let run = sandbox.export_teams_list("EDIT", &["--yes"]);
    assert_eq!(run.exit_code(), 1);
    let messages = run.messages();
    assert_eq!(
        messages[0],
        format!("Error teams_list_name_duplicate (team={dropped}, name=/4ccg/, kept={kept})")
    );
    assert_eq!(
        messages
            .iter()
            .filter(|line| line.starts_with("Error"))
            .count(),
        1
    );
    assert!(
        messages
            .last()
            .unwrap()
            .starts_with("Info teams_list_written")
    );
    let list = sandbox.working_list();
    assert_eq!(
        list.id_of(&name("/4ccg/")),
        Some(id(u16::try_from(kept).unwrap()))
    );
    assert!(list.name_of(id(u16::try_from(dropped).unwrap())).is_none());
}

#[test]
fn a_pes_2014_save_is_refused() {
    let sandbox = Sandbox::new("teams_list_pes14");
    sandbox.fixture(PesVersion::Pes14, "EDIT14");

    let run = sandbox.export_teams_list("EDIT14", &["--yes"]);
    run.assert_refused(2, &["PES 2014"]);
    assert!(!sandbox.path("data/teams_list.txt").exists());
}
