//! A save holding, beside the cup's teams, one team outside the cup's id range and a
//! player on no roster, as a save made from the stock game holds the game's own clubs:
//! the fixture saves hold the cup's teams alone. Shared by the crate's unit tests and its
//! command-line tests (`tests/common`).

use std::collections::HashSet;

use pes_savefile::{EditFile, fixture_save};
use pes_version::PesVersion;

/// The id the moved team takes, outside the cup's 701 to 920.
pub const OUTSIDE_TEAM: u32 = 300;

/// The cup team renumbered `OUTSIDE_TEAM`: /aco/, the third team in record order, whose
/// name and first player's name no other team or player of the fixtures holds, so a
/// filter typed as either finds only them.
pub const MOVED_TEAM: u32 = 703;

/// `version`'s fixture save with team `MOVED_TEAM` renumbered `OUTSIDE_TEAM` (its roster,
/// tactics and players' records kept) and its last filled roster slot emptied, so the
/// player it held is on no roster. Returns the file, still to be written, and that
/// player's id.
pub fn outside_cup_file(version: PesVersion) -> (EditFile, u32) {
    let mut file = EditFile::from_bytes(&fixture_save(version)).expect("the fixture save reads");
    let team = file
        .team_mut(MOVED_TEAM)
        .expect("the fixture holds the moved team");
    team.id = OUTSIDE_TEAM;
    let slot = team
        .roster
        .iter_mut()
        .rev()
        .find(|slot| slot.player_id != 0)
        .expect("the moved team has a rostered player");
    let unrostered = std::mem::replace(&mut slot.player_id, 0);
    (file, unrostered)
}

/// The cup player whose Height `second_copy` changes.
pub const CUP_PLAYER: u32 = 70101;

/// A second copy of `outside_cup_file`'s save `file`, for a compare: team `OUTSIDE_TEAM`'s
/// name, its first player's Height, the unrostered player's Height and `CUP_PLAYER`'s
/// Height each differ from `file`'s (the name gains a letter, each Height one more).
pub fn second_copy(file: &EditFile, unrostered: u32) -> EditFile {
    let mut copy = file.clone();
    copy.team_mut(OUTSIDE_TEAM)
        .expect("the save holds the outside team")
        .name
        .push('x');
    for id in [outside_players(file)[0], unrostered, CUP_PLAYER] {
        copy.player_mut(id)
            .expect("the player is in the save")
            .basic
            .height += 1;
    }
    copy
}

/// Asserts that `first` and `second`, `second_copy`'s pair as written and read back, differ
/// in each of the four records `second_copy` changes.
pub fn assert_copies_differ(first: &EditFile, second: &EditFile, unrostered: u32) {
    assert_ne!(
        first.team_records(OUTSIDE_TEAM).unwrap().team,
        second.team_records(OUTSIDE_TEAM).unwrap().team,
        "premise: team 300's records differ"
    );
    for id in [outside_players(first)[0], unrostered, CUP_PLAYER] {
        assert_ne!(
            first.player_record(id),
            second.player_record(id),
            "premise: player {id}'s records differ"
        );
    }
}

/// The ids on `OUTSIDE_TEAM`'s roster, empty slots left out.
pub fn outside_players(file: &EditFile) -> Vec<u32> {
    file.team(OUTSIDE_TEAM)
        .expect("the save holds the outside team")
        .roster
        .iter()
        .map(|slot| slot.player_id)
        .filter(|&id| id != 0)
        .collect()
}

/// Asserts what the tests on `outside_cup_file`'s save rely on: team `OUTSIDE_TEAM` is in
/// the save and its roster holds players that no other team's holds, `unrostered` is a
/// player of the save on no roster, and every other team is a cup team.
pub fn assert_premises(file: &EditFile, unrostered: u32) {
    let outside = outside_players(file);
    assert!(!outside.is_empty(), "premise: team 300 holds players");
    let others: HashSet<u32> = file
        .teams()
        .iter()
        .filter(|team| team.id != OUTSIDE_TEAM)
        .flat_map(|team| team.roster.iter().map(|slot| slot.player_id))
        .collect();
    for id in &outside {
        assert!(
            !others.contains(id),
            "premise: team 300's player {id} is on no other roster"
        );
        assert!(
            file.player(*id).is_some(),
            "premise: player {id} is in the save"
        );
    }
    assert!(
        file.player(unrostered).is_some(),
        "premise: the unrostered player {unrostered} is in the save"
    );
    assert!(
        file.teams()
            .iter()
            .all(|team| team.roster.iter().all(|slot| slot.player_id != unrostered)),
        "premise: player {unrostered} is on no roster"
    );
    assert!(
        file.teams()
            .iter()
            .all(|team| team.id == OUTSIDE_TEAM || (701..=920).contains(&team.id)),
        "premise: every other team is a cup team"
    );
}
