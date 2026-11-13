//! Which players and team rows the master list shows: the team the selector
//! names, or every cup team at All teams and while the filter box is in use, so a
//! player is found without knowing his team first, narrowed to one
//! competition's teams by the selector's category. A team outside the cup
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

/// Which of the cup's team-id ranges the selector and the list show (the
/// teams list's own sections): every cup team, or one competition's alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TeamCategory {
    /// Every cup team.
    All,
    /// The 4cc teams (ids 701 to 780, the Backup rows included).
    FourCc,
    /// The VGL teams (781 to 850).
    Vgl,
    /// The invitational teams (851 to 920).
    Invitational,
}

impl TeamCategory {
    /// The categories in the radio buttons' order.
    pub(crate) const CHOICES: [TeamCategory; 4] = [
        TeamCategory::All,
        TeamCategory::FourCc,
        TeamCategory::Vgl,
        TeamCategory::Invitational,
    ];

    /// Whether `team_id` lies in the category's id range.
    pub(crate) fn contains(self, team_id: u32) -> bool {
        match self {
            TeamCategory::All => true,
            TeamCategory::FourCc => (701..=780).contains(&team_id),
            TeamCategory::Vgl => (781..=850).contains(&team_id),
            TeamCategory::Invitational => (851..=920).contains(&team_id),
        }
    }

    /// The radio button's text.
    pub(crate) fn label(self) -> &'static str {
        match self {
            TeamCategory::All => "All",
            TeamCategory::FourCc => "4cc",
            TeamCategory::Vgl => "VGL",
            TeamCategory::Invitational => "Invitational",
        }
    }

    /// The `team_category` setting's spelling of the category.
    pub(crate) fn setting(self) -> &'static str {
        match self {
            TeamCategory::All => "all",
            TeamCategory::FourCc => "4cc",
            TeamCategory::Vgl => "vgl",
            TeamCategory::Invitational => "invitational",
        }
    }

    /// The category `setting` spells; an unknown spelling reads as `All`.
    pub(crate) fn from_setting(setting: &str) -> TeamCategory {
        match setting {
            "4cc" => TeamCategory::FourCc,
            "vgl" => TeamCategory::Vgl,
            "invitational" => TeamCategory::Invitational,
            _ => TeamCategory::All,
        }
    }
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
/// in roster order; `category` leaves out the teams outside its range,
/// under `TeamChoice::All` and a non-empty `filter` alike. A non-empty
/// `filter` matches across the kept teams (whatever the choice) on the
/// display name, the shirt name or the id,
/// case-insensitive. A team whose own name (its display name, matched as
/// the players' names are) matches is then listed with all its players, so
/// a search for a team shows its squad; any other team is listed with its
/// matching players only, and skipped when none match. Empty slots (player
/// id 0) are never listed.
pub(crate) fn list(
    file: &EditFile,
    team: &TeamChoice,
    filter: &str,
    category: TeamCategory,
) -> Vec<ListRow> {
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
        if !category.contains(team_entry.id) {
            continue;
        }
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
pub(crate) fn rows(
    file: &EditFile,
    team: &TeamChoice,
    filter: &str,
    category: TeamCategory,
) -> Vec<Row> {
    list(file, team, filter, category)
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
            rows(&file, &TeamChoice::Team(718), "", TeamCategory::All),
            rostered(&file, 718)
        );
        // Team 718's first slot is player 71801; a team the save lacks lists nothing.
        assert_eq!(
            rows(&file, &TeamChoice::Team(718), "", TeamCategory::All)[0].player_id,
            71801
        );
        assert!(rows(&file, &TeamChoice::Team(99999), "", TeamCategory::All).is_empty());
    }

    #[test]
    fn all_lists_every_teams_roster_in_save_order() {
        let file = open(PesVersion::Pes17);
        let expected: Vec<Row> = file
            .teams()
            .iter()
            .flat_map(|team| rostered(&file, team.id))
            .collect();
        assert_eq!(
            rows(&file, &TeamChoice::All, "", TeamCategory::All),
            expected
        );
    }

    #[test]
    fn a_filter_matches_the_display_name_across_all_teams() {
        let file = open(PesVersion::Pes17);
        // 71801's raw name carries colour codes; the match is on the
        // code-free display name, case-insensitive.
        assert!(file.player(71801).unwrap().name.contains('\x11'));
        assert_eq!(
            rows(&file, &TeamChoice::Team(701), "roadtrip", TeamCategory::All),
            [Row {
                team_id: 718,
                slot: 0,
                player_id: 71801,
            }]
        );
        assert!(rows(&file, &TeamChoice::All, "ROADTRIP", TeamCategory::All).len() == 1);
        // A whitespace-only filter is no filter: the chosen team's roster.
        assert_eq!(
            rows(&file, &TeamChoice::Team(701), "   ", TeamCategory::All),
            rostered(&file, 701)
        );
    }

    #[test]
    fn a_filter_matches_the_id_and_the_shirt_name() {
        let file = open(PesVersion::Pes17);
        assert_eq!(
            rows(&file, &TeamChoice::Team(701), "71801", TeamCategory::All),
            [Row {
                team_id: 718,
                slot: 0,
                player_id: 71801,
            }]
        );
        assert_eq!(
            rows(
                &file,
                &TeamChoice::Team(701),
                "exact change",
                TeamCategory::All
            ),
            [Row {
                team_id: 718,
                slot: 0,
                player_id: 71801,
            }]
        );
        // No match: nothing, not the chosen team's roster.
        assert!(
            rows(
                &file,
                &TeamChoice::Team(701),
                "no such player",
                TeamCategory::All
            )
            .is_empty()
        );
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
        assert_eq!(
            list(&file, &TeamChoice::All, "", TeamCategory::All),
            expected
        );
    }

    // One team picked: its row then its players, no other team's.
    #[test]
    fn list_on_one_team_gives_its_row_then_its_players() {
        let file = open(PesVersion::Pes17);
        let expected: Vec<ListRow> = [ListRow::Team(718)]
            .into_iter()
            .chain(rostered(&file, 718).into_iter().map(ListRow::Player))
            .collect();
        assert_eq!(
            list(&file, &TeamChoice::Team(718), "", TeamCategory::All),
            expected
        );
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
        assert_eq!(
            list(&file, &TeamChoice::All, "/f/", TeamCategory::All),
            expected
        );
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
        assert!(
            list(&file, &TeamChoice::Team(701), "umajp", TeamCategory::All)
                .contains(&ListRow::Team(827))
        );
    }

    // A filter matching one player: his team's row heads him, no other
    // team's row is drawn.
    #[test]
    fn list_filter_matching_one_player_gives_his_row_then_him() {
        let file = open(PesVersion::Pes17);
        assert_eq!(
            list(&file, &TeamChoice::Team(701), "roadtrip", TeamCategory::All),
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

    // The categories' id ranges: 4cc 701 to 780, VGL 781 to 850,
    // Invitational 851 to 920, All teams holding them all.
    #[test]
    fn the_categories_hold_their_team_id_ranges() {
        for (id, four_cc, vgl, invitational) in [
            (700, false, false, false),
            (701, true, false, false),
            (780, true, false, false),
            (781, false, true, false),
            (850, false, true, false),
            (851, false, false, true),
            (920, false, false, true),
            (921, false, false, false),
        ] {
            assert_eq!(TeamCategory::FourCc.contains(id), four_cc, "4cc {id}");
            assert_eq!(TeamCategory::Vgl.contains(id), vgl, "VGL {id}");
            assert_eq!(
                TeamCategory::Invitational.contains(id),
                invitational,
                "Invitational {id}"
            );
            assert!(TeamCategory::All.contains(id), "All teams {id}");
        }
    }

    // The `team_category` setting's spellings; an unknown one reads as `all`.
    #[test]
    fn the_setting_spells_each_category() {
        for (setting, category) in [
            ("all", TeamCategory::All),
            ("4cc", TeamCategory::FourCc),
            ("vgl", TeamCategory::Vgl),
            ("invitational", TeamCategory::Invitational),
        ] {
            assert_eq!(category.setting(), setting, "{setting}");
            assert_eq!(TeamCategory::from_setting(setting), category, "{setting}");
        }
        assert_eq!(TeamCategory::from_setting("premier"), TeamCategory::All);
    }

    // A category lists its own teams alone, under All teams and under the
    // filter's search alike.
    #[test]
    fn list_lists_only_the_categorys_teams() {
        let file = open(PesVersion::Pes17);
        let expected: Vec<ListRow> = file
            .teams()
            .iter()
            .filter(|team| (781..=850).contains(&team.id))
            .flat_map(|team| {
                [ListRow::Team(team.id)]
                    .into_iter()
                    .chain(rostered(&file, team.id).into_iter().map(ListRow::Player))
            })
            .collect();
        assert!(expected.len() > 70, "premise: the VGL teams' rows");
        assert_eq!(
            list(&file, &TeamChoice::All, "", TeamCategory::Vgl),
            expected
        );
        // A team picked outside the category lists nothing.
        assert!(list(&file, &TeamChoice::Team(718), "", TeamCategory::Vgl).is_empty());

        // The filter box's search under a category finds no player of a
        // team outside it: 71801 is a 4cc player.
        assert!(list(&file, &TeamChoice::All, "roadtrip", TeamCategory::Vgl).is_empty());
        assert_eq!(
            list(&file, &TeamChoice::All, "roadtrip", TeamCategory::FourCc),
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
