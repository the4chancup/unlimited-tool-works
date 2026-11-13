use crate::test_support::cup_save::{
    CUP_PLAYER, OUTSIDE_TEAM, assert_copies_differ, outside_cup_file, outside_players, second_copy,
};
use crate::view::comparator::roster_of;

use super::*;

/// A team outside the cup places no player: a reported player its roster holds is headed
/// `No team`, never by team 300, and his head is no click into a team the list hides.
#[test]
fn roster_of_places_no_one_on_a_team_outside_the_cup() {
    let (file, _) = outside_cup_file(PesVersion::Pes17);
    let roster = roster_of(&file);
    let outside = outside_players(&file);
    for id in &outside {
        assert!(!roster.contains_key(id), "team 300's player {id}");
    }
    assert!(
        roster
            .values()
            .all(|place| place.select().team_id != OUTSIDE_TEAM),
        "no place on team 300"
    );
    assert_eq!(
        roster[&CUP_PLAYER].select().team_id,
        701,
        "a cup player is placed"
    );
}

/// SE-GUI-77's save open and its second copy picked: team 300's name, its first player's
/// Height, the unrostered player's Height and 70101's Height differ, and the comparator's
/// rows hold 70101's Height alone, under no heading of team 300 or of no team.
// SE-GUI-77
#[test]
fn against_the_second_copy_the_rows_hold_the_cup_players_height_alone() {
    let dir = crate::test_support::scratch("comparator_outside_cup");
    let (file, unrostered) = outside_cup_file(PesVersion::Pes17);
    let (open, picked) = (dir.join("EDIT00000000"), dir.join("picked"));
    std::fs::write(&open, file.to_bytes(&TEST_SALT).unwrap()).unwrap();
    let copy = second_copy(&file, unrostered);
    std::fs::write(&picked, copy.to_bytes(&TEST_SALT).unwrap()).unwrap();
    let opened = EditFile::load(&open).unwrap();
    assert_copies_differ(&opened, &EditFile::load(&picked).unwrap(), unrostered);
    let outside = outside_players(&opened)[0];
    let height = opened.player(CUP_PLAYER).unwrap().basic.height;

    let mut view = SaveView::new();
    view.open(&open);
    let mut window = Headless::new();
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    // The picked save is the report's first, so a row reads picked, then open.
    let rows: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains('\u{2192}'))
        .collect();
    assert_eq!(
        rows,
        [&format!("Height {} \u{2192} {height}", height + 1)],
        "70101's Height alone: {lines:?}"
    );
    for text in [
        "/aco/".to_owned(),
        "No team".to_owned(),
        outside.to_string(),
        unrostered.to_string(),
    ] {
        assert!(
            !lines.iter().any(|line| line.contains(&text)),
            "nothing names {text:?}: {lines:?}"
        );
    }
}
