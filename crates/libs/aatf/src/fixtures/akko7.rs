//! The Akko League VII ruleset's fixtures (`rulesets/akko7.rhai`).

use pes_savefile::{EditFile, fixture_save};

use super::*;

/// The ruleset loaded from its embedded data block.
fn rules() -> Rules {
    Rules::from_data_block(include_str!("../../rulesets/akko7.rhai")).expect("akko7 loads")
}

/// A PES 2014 player's Tenacity, a stat only that version stores, is a
/// rating stat under Akko VII: off his tier's rating, every other stat
/// on it, it is his one violation. The player is the vanilla save's
/// first rostered one rebuilt to the regular tier's targets.
#[test]
fn a_tenacity_off_its_rating_is_a_stat_violation() {
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes14)).expect("fixture save");
    let team_id = file
        .teams()
        .iter()
        .find(|team| team.roster.iter().any(|slot| slot.player_id != 0))
        .expect("a rostered team")
        .id;
    let (slot, player_id) = file
        .team(team_id)
        .expect("the team")
        .roster
        .iter()
        .enumerate()
        .find(|(_, slot)| slot.player_id != 0)
        .map(|(index, slot)| (index, slot.player_id))
        .expect("a rostered player");
    {
        let entry = file.player_mut(player_id).expect("the rostered player");
        // A regular in every read but Tenacity: every ability at the
        // regular rate but it, form 4, injury resistance 1, weak foot
        // 2/2 (each stored one below), a mid height, an A at his
        // registered position alone, no cards.
        for index in 0..26 {
            entry.stats.set_ability(index, 77);
        }
        entry.stats.tenacity = Some(40);
        entry.stats.form = 3;
        entry.stats.injury_resistance = 0;
        entry.stats.weak_foot_usage = 1;
        entry.stats.weak_foot_accuracy = 1;
        entry.basic.height = 180;
        let registered = entry.positions.registered;
        entry.positions.playable = [0; 13];
        let at = PLAYABLE_POSITIONS
            .iter()
            .position(|index| *index == usize::from(registered))
            .expect("a position");
        entry.positions.playable[at] = 2;
        entry.skills.skills = [false; 41];
        entry.skills.com_styles = [false; 7];
    }
    let mut team = TeamView::new(&file, team_id).expect("the team views");
    team.roster_breaks.clear();
    let slot = u8::try_from(slot).expect("a roster slot");
    assert_eq!(
        problems(&rules(), &team)
            .into_iter()
            .filter(|violation| violation.slot == Some(slot))
            .collect::<Vec<_>>(),
        vec![error(
            Some(slot),
            ViolationKind::Stat {
                stat: stat("tenacity"),
                value: 40,
                target: 77,
                at_most: false
            }
        )]
    );
}

/// `/4ccg/` (team 857) of the maintainer's `.akko7v2` reports no
/// error-severity violation under Akko VII.
#[test]
#[cfg(not(target_arch = "wasm32"))]
#[ignore = "reads the maintainer's PES 2014 saves"]
fn the_akko7v2_4ccg_team_is_legal_under_akko7() {
    let folder = std::path::Path::new("C:/Data/Documents/KONAMI/Pro Evolution Soccer 2014/save");
    let bytes = std::fs::read(folder.join("EDIT.bin.akko7v2")).expect("the save reads");
    let file = EditFile::from_bytes(&bytes).expect("the save opens");
    let team = TeamView::new(&file, 857).expect("team 857 views");
    let errors: Vec<Violation> = rules()
        .check_team(&team)
        .expect("the check runs")
        .violations
        .into_iter()
        .filter(|violation| violation.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}
