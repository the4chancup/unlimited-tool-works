//! Which players and team rows the master list shows: the team the selector
//! names, or every cup team at ALL and while the filter box is in use, so a
//! player is found without knowing his team first. A team outside the cup
//! (`session::cup`) is never listed, nor are its players.

use std::collections::HashMap;

use pes_savefile::EditFile;
use pes_savefile::model::names::display_name;
use pes_savefile::model::{PlayerEntry, TeamEntry};

use crate::session::cup::cup_teams;

/// Which team the list shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TeamChoice {
    /// Every cup team of the save, in the save's own order.
    All,
    /// One team, by id.
    Team(u32),
}

/// One listed row: the player's team, roster slot and id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Row {
    /// The team the player is rostered on.
    pub team_id: u32,
    /// His roster slot, the 0-based index into the team's roster.
    pub slot: u8,
    /// His player id.
    pub player_id: u32,
}

/// One drawn row of the master list: a team's row heading its players, or
/// one player row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListRow {
    /// A team's own row, by team id (selecting it shows the team view).
    Team(u32),
    /// A player row.
    Player(Row),
}

/// The rows the list draws: each listed team's row, then its players' rows
/// in roster order. A non-empty `filter` matches across all cup teams (whatever
/// the choice) on the display name, the shirt name or the id,
/// case-insensitive. A team whose own name (its display name, matched as
/// the players' names are) matches is then listed with all its players, so
/// a search for a team shows its squad; any other team is listed with its
/// matching players only, and skipped when none match. Empty slots (player
/// id 0) are never listed.
pub(crate) fn list(file: &EditFile, team: &TeamChoice, filter: &str) -> Vec<ListRow> {
    let query = filter.trim().to_lowercase();
    let filtering = !query.is_empty();
    // Player ids are stable inside an open file: one map serves the whole
    // scan instead of a `file.player()` linear scan per slot.
    let players: HashMap<u32, &PlayerEntry> = file
        .players()
        .iter()
        .map(|player| (player.id, player))
        .collect();
    let mut rows = Vec::new();
    for team_entry in cup_teams(file) {
        if !filtering
            && let TeamChoice::Team(id) = team
            && *id != team_entry.id
        {
            continue;
        }
        let named = filtering
            && display_name(&team_entry.name)
                .to_lowercase()
                .contains(&query);
        let player_query = if named { "" } else { query.as_str() };
        let player_rows = team_rows(team_entry, &players, player_query);
        if filtering && !named && player_rows.is_empty() {
            continue;
        }
        rows.push(ListRow::Team(team_entry.id));
        rows.extend(player_rows.into_iter().map(ListRow::Player));
    }
    rows
}

/// `list`'s player rows, alone, for the unsaved-changes table's player
/// order (`view::prompt`).
pub(crate) fn rows(file: &EditFile, team: &TeamChoice, filter: &str) -> Vec<Row> {
    list(file, team, filter)
        .into_iter()
        .filter_map(|row| match row {
            ListRow::Player(row) => Some(row),
            ListRow::Team(_) => None,
        })
        .collect()
}

/// The rostered slots of `team`, in slot order; with `query` non-empty only
/// the players matching it.
fn team_rows(team: &TeamEntry, players: &HashMap<u32, &PlayerEntry>, query: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    for (slot, roster_slot) in team.roster.iter().enumerate() {
        if roster_slot.player_id == 0 {
            continue;
        }
        if !query.is_empty()
            && !players
                .get(&roster_slot.player_id)
                .is_some_and(|player| matches(player, query))
        {
            continue;
        }
        rows.push(Row {
            team_id: team.id,
            slot: u8::try_from(slot).expect("a roster is at most 40 slots"),
            player_id: roster_slot.player_id,
        });
    }
    rows
}

/// Whether `player` answers the trimmed, lowercased `query`: a substring of
/// his display name, shirt name or id as text.
fn matches(player: &PlayerEntry, query: &str) -> bool {
    display_name(&player.name).to_lowercase().contains(query)
        || player.shirt_name.to_lowercase().contains(query)
        || player.id.to_string().contains(query)
}

#[cfg(test)]
mod tests {
    use pes_savefile::fixture_save;
    use pes_version::PesVersion;

    use super::*;

    fn open(version: PesVersion) -> EditFile {
        EditFile::from_bytes(&fixture_save(version)).unwrap()
    }

    /// The rostered slots of `team_id` as the file lists them, for comparing
    /// `rows`' output with.
    fn rostered(file: &EditFile, team_id: u32) -> Vec<Row> {
        file.team(team_id)
            .unwrap()
            .roster
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.player_id != 0)
            .map(|(slot, roster_slot)| Row {
                team_id,
                slot: u8::try_from(slot).unwrap(),
                player_id: roster_slot.player_id,
            })
            .collect()
    }

    #[test]
    fn a_team_lists_its_rostered_slots_in_order() {
        let file = open(PesVersion::Pes17);
        assert_eq!(
            rows(&file, &TeamChoice::Team(718), ""),
            rostered(&file, 718)
        );
        // Team 718's first slot is player 71801; a team the save lacks lists nothing.
        assert_eq!(rows(&file, &TeamChoice::Team(718), "")[0].player_id, 71801);
        assert!(rows(&file, &TeamChoice::Team(99999), "").is_empty());
    }

    #[test]
    fn all_lists_every_teams_roster_in_save_order() {
        let file = open(PesVersion::Pes17);
        let expected: Vec<Row> = file
            .teams()
            .iter()
            .flat_map(|team| rostered(&file, team.id))
            .collect();
        assert_eq!(rows(&file, &TeamChoice::All, ""), expected);
    }

    #[test]
    fn a_filter_matches_the_display_name_across_all_teams() {
        let file = open(PesVersion::Pes17);
        // 71801's raw name carries colour codes; the match is on the
        // code-free display name, case-insensitive.
        assert!(file.player(71801).unwrap().name.contains('\x11'));
        assert_eq!(
            rows(&file, &TeamChoice::Team(701), "roadtrip"),
            [Row {
                team_id: 718,
                slot: 0,
                player_id: 71801,
            }]
        );
        assert!(rows(&file, &TeamChoice::All, "ROADTRIP").len() == 1);
        // A whitespace-only filter is no filter: the chosen team's roster.
        assert_eq!(
            rows(&file, &TeamChoice::Team(701), "   "),
            rostered(&file, 701)
        );
    }

    #[test]
    fn a_filter_matches_the_id_and_the_shirt_name() {
        let file = open(PesVersion::Pes17);
        assert_eq!(
            rows(&file, &TeamChoice::Team(701), "71801"),
            [Row {
                team_id: 718,
                slot: 0,
                player_id: 71801,
            }]
        );
        assert_eq!(
            rows(&file, &TeamChoice::Team(701), "exact change"),
            [Row {
                team_id: 718,
                slot: 0,
                player_id: 71801,
            }]
        );
        // No match: nothing, not the chosen team's roster.
        assert!(rows(&file, &TeamChoice::Team(701), "no such player").is_empty());
    }

    // A team row heads each listed team's players.
    #[test]
    fn list_heads_every_teams_players_with_its_row() {
        let file = open(PesVersion::Pes17);
        let expected: Vec<ListRow> = file
            .teams()
            .iter()
            .flat_map(|team| {
                [ListRow::Team(team.id)]
                    .into_iter()
                    .chain(rostered(&file, team.id).into_iter().map(ListRow::Player))
            })
            .collect();
        assert_eq!(list(&file, &TeamChoice::All, ""), expected);
    }

    // One team picked: its row then its players, no other team's.
    #[test]
    fn list_on_one_team_gives_its_row_then_its_players() {
        let file = open(PesVersion::Pes17);
        let expected: Vec<ListRow> = [ListRow::Team(718)]
            .into_iter()
            .chain(rostered(&file, 718).into_iter().map(ListRow::Player))
            .collect();
        assert_eq!(list(&file, &TeamChoice::Team(718), ""), expected);
    }

    // Under a filter a team whose own name matches lists its whole squad,
    // not only the players whose own names match: a search for a team
    // shows its squad.
    #[test]
    fn list_filter_matching_a_teams_name_lists_its_whole_squad() {
        let file = open(PesVersion::Pes17);
        // Team 718's name is "/f/", held by no other team's display name.
        // Of the whole pool only 77206 ("PRESS /f/", team 772's sixth
        // slot) holds it in his display name, shirt name or id, so none of
        // 718's players is listed for his own name.
        assert_eq!(file.team(718).unwrap().name, "/f/");
        for team in file.teams() {
            assert!(
                team.id == 718 || !display_name(&team.name).to_lowercase().contains("/f/"),
                "premise: team {} does not match /f/",
                team.id
            );
        }
        let holding: Vec<u32> = file
            .players()
            .iter()
            .filter(|player| {
                display_name(&player.name).to_lowercase().contains("/f/")
                    || player.shirt_name.to_lowercase().contains("/f/")
                    || player.id.to_string().contains("/f/")
            })
            .map(|player| player.id)
            .collect();
        assert_eq!(holding, [77206], "premise: the players holding /f/");
        assert_eq!(file.team(772).unwrap().roster[5].player_id, 77206);
        let squad = rostered(&file, 718);
        assert!(squad.len() > 1, "premise: 718 has a squad to list");
        let expected: Vec<ListRow> = [ListRow::Team(718)]
            .into_iter()
            .chain(squad.into_iter().map(ListRow::Player))
            .chain([
                ListRow::Team(772),
                ListRow::Player(Row {
                    team_id: 772,
                    slot: 5,
                    player_id: 77206,
                }),
            ])
            .collect();
        assert_eq!(list(&file, &TeamChoice::All, "/f/"), expected);
    }

    // A team whose name carries capitals matches the filter
    // case-insensitively, as a player's does: PES 21's 827 is `/umaJP/`,
    // found by `umajp`.
    #[test]
    fn list_filter_matches_a_capitalised_team_name_case_insensitively() {
        let file = open(PesVersion::Pes21);
        assert_eq!(file.team(827).unwrap().name, "/umaJP/", "premise");
        let players: HashMap<u32, &PlayerEntry> = file
            .players()
            .iter()
            .map(|player| (player.id, player))
            .collect();
        // Premise: none of 827's rostered players matches `umajp`, so the
        // row is listed only by the team's own name.
        for row in rostered(&file, 827) {
            let player = players[&row.player_id];
            assert!(
                !matches(player, "umajp"),
                "premise: {} does not match umajp",
                player.id
            );
        }
        assert!(list(&file, &TeamChoice::Team(701), "umajp").contains(&ListRow::Team(827)));
    }

    // A filter matching one player: his team's row heads him, no other
    // team's row is drawn.
    #[test]
    fn list_filter_matching_one_player_gives_his_row_then_him() {
        let file = open(PesVersion::Pes17);
        assert_eq!(
            list(&file, &TeamChoice::Team(701), "roadtrip"),
            [
                ListRow::Team(718),
                ListRow::Player(Row {
                    team_id: 718,
                    slot: 0,
                    player_id: 71801,
                }),
            ]
        );
    }
}
