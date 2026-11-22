//! The cup's teams and the players on their rosters: all the editor lists, checks and
//! edits. A save made from the stock game also holds the game's own clubs and national
//! teams, and players on no cup roster. Nobody edits those, so they stay in the save as
//! they were read and are written back untouched. On a PES 2014 save every team counts
//! as the cup's: the 2014 cup ran at real clubs' ids.

use std::collections::HashSet;

use pes_savefile::EditFile;
use pes_savefile::model::TeamEntry;
use pes_version::PesVersion;
use teams_list::TeamId;

/// Whether `team_id` is a cup team's: inside the teams list's id range (701 to 920).
/// On PES 2014 every id is one, the 2014 cup having run at real clubs' ids.
pub(crate) fn is_cup_team(version: PesVersion, team_id: u32) -> bool {
    version == PesVersion::Pes14 || u16::try_from(team_id).is_ok_and(|id| TeamId::new(id).is_ok())
}

/// `file`'s cup teams, in record order.
pub(crate) fn cup_teams(file: &EditFile) -> impl Iterator<Item = &TeamEntry> {
    file.teams()
        .iter()
        .filter(|team| is_cup_team(file.version(), team.id))
}

/// The ids of the players on a cup team's roster. Empty slots (id 0) are no player.
pub(crate) fn cup_player_ids(file: &EditFile) -> HashSet<u32> {
    cup_teams(file)
        .flat_map(|team| team.roster.iter())
        .map(|slot| slot.player_id)
        .filter(|&id| id != 0)
        .collect()
}

#[cfg(test)]
mod tests {
    use pes_savefile::fixture_save;
    use pes_version::PesVersion;

    use super::*;
    use crate::test_support::TEST_SALT;
    use crate::test_support::cup_save::{
        MOVED_TEAM, OUTSIDE_TEAM, assert_premises, outside_cup_file, outside_players,
    };

    /// `outside_cup_file`'s PES 17 save, written and read back.
    fn outside_cup_save() -> (EditFile, u32) {
        let (file, unrostered) = outside_cup_file(PesVersion::Pes17);
        let file = EditFile::from_bytes(&file.to_bytes(&TEST_SALT).unwrap()).unwrap();
        assert_premises(&file, unrostered);
        (file, unrostered)
    }

    #[test]
    fn a_cup_team_is_one_whose_id_lies_in_701_to_920() {
        for (id, cup) in [
            (0, false),
            (300, false),
            (700, false),
            (701, true),
            (814, true),
            (920, true),
            (921, false),
            (999, false),
            (70_100, false),
            (u32::MAX, false),
        ] {
            assert_eq!(is_cup_team(PesVersion::Pes21, id), cup, "team {id}");
        }
    }

    #[test]
    fn team_136_is_a_cup_team_on_pes14_alone() {
        // 136 is /gd/'s id in the 2014 cup, a real club's, outside 701 to 920.
        assert!(!is_cup_team(PesVersion::Pes21, 136), "team 136 on PES 21");
        assert!(is_cup_team(PesVersion::Pes14, 136), "team 136 on PES 2014");
    }

    #[test]
    fn on_a_pes14_save_every_team_is_a_cup_team() {
        let file = EditFile::from_bytes(&fixture_save(PesVersion::Pes14)).unwrap();
        assert_eq!(
            file.version(),
            PesVersion::Pes14,
            "premise: the fixture reads PES 2014"
        );
        assert_eq!(file.teams().len(), 478, "premise: the fixture's teams");
        // Team 136, VASCO DA GAMA: a real club's id outside the cup's
        // range, where /gd/ stood in the 2014 cup.
        let vasco = file.team(136).expect("premise: team 136 is in the save");
        assert_eq!(vasco.name, "VASCO DA GAMA", "premise: team 136's name");
        assert!(
            vasco.roster.iter().any(|slot| slot.player_id != 0),
            "premise: team 136 holds players"
        );
        let expected: Vec<u32> = file.teams().iter().map(|team| team.id).collect();
        let teams: Vec<u32> = cup_teams(&file).map(|team| team.id).collect();
        assert_eq!(teams, expected, "every team of the file, in record order");
    }

    #[test]
    fn the_cup_teams_are_every_team_but_the_outside_one_in_record_order() {
        let (file, _) = outside_cup_save();
        let expected: Vec<u32> = file
            .teams()
            .iter()
            .map(|team| team.id)
            .filter(|&id| id != OUTSIDE_TEAM)
            .collect();
        assert_eq!(expected.len(), 219, "premise: 220 teams, one moved out");
        let teams: Vec<u32> = cup_teams(&file).map(|team| team.id).collect();
        assert_eq!(teams, expected);
    }

    #[test]
    fn the_cup_players_leave_out_the_outside_teams_and_the_unrostered_one() {
        let (file, unrostered) = outside_cup_save();
        let cup = cup_player_ids(&file);
        for id in outside_players(&file) {
            assert!(!cup.contains(&id), "team {OUTSIDE_TEAM}'s player {id}");
        }
        assert!(!cup.contains(&unrostered), "the unrostered {unrostered}");
        assert!(!cup.contains(&0), "an empty slot is no player");
        // Every other player of the save is on a cup roster.
        let hidden = outside_players(&file).len() + 1;
        assert_eq!(cup.len(), file.players().len() - hidden);
        assert!(cup.contains(&70101), "team 701's first player");
        assert!(
            file.team(MOVED_TEAM).is_none(),
            "premise: no cup team keeps the moved id"
        );
    }
}
