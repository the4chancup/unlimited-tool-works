use super::*;

// SE-GUI-13
#[test]
fn a_refused_pick_clears_the_held_result() {
    let (mut view, mut window, _) = compared_73401("comparator_refusal_clears");
    let dir = scratch_of(&view);
    let picked = picked_pes21(&dir);
    pick(&mut window, &mut view, picked);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line
            == "The picked save is a PES 2021 save and the open save a PES 2017 one, but both must be of the same version"),
        "the refusal's literal: {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line == "Team 734 /lit/"),
        "the landed result's rows are gone: {lines:?}"
    );
    assert!(
        view.comparator.comparison.result.is_none() && view.comparator.comparison.pending.is_none(),
        "the comparison reset"
    );
}

// SE-GUI-13
#[test]
fn a_repick_of_the_same_path_recompares() {
    let (mut view, mut window, _) = compared_73401("comparator_repick");
    let dir = scratch_of(&view);
    // The picked file rewritten on disk under the same path (the member
    // changed the reference in the game): a second edit lands.
    let picked = picked_save(&dir, "picked", |file| {
        file.player_mut(73404)
            .expect("73404 is a player")
            .basic
            .height = 180;
    });
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Height 180 \u{2192} 185");
    comparator_button(&pairs, "Height 180 \u{2192} 185");
    comparator_button(&pairs, "Height 185 \u{2192} 183");
}

// SE-GUI-13
#[test]
fn another_save_keeps_the_pick_and_recompares() {
    let (mut view, mut window, _) = compared_73401("comparator_reopen");
    let dir = scratch_of(&view);
    // A second save of the same version, carrying another edit: the pick
    // stays and the result is the new save's (a member compares several
    // saves against one reference).
    let second = picked_save(&dir, "second", |file| {
        file.player_mut(73402)
            .expect("73402 is a player")
            .basic
            .height = 180;
    });
    view.open(&second);
    compared(&mut window, &mut view);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    let picked_path = dir.join("picked").display().to_string();
    assert!(
        lines.iter().any(|line| line == &picked_path),
        "the picked path still reads: {lines:?}"
    );
    let pairs = scroll_to(&mut window, &mut view, "Height 185 \u{2192} 180");
    comparator_button(&pairs, "Height 185 \u{2192} 180");
    assert!(
        !comparator_nodes(&pairs)
            .iter()
            .any(|node| node.label() == Some("Height 185 \u{2192} 183")),
        "the first save's row is gone"
    );

    // A save of another version: the pick is refused naming both, the
    // pick dropped.
    let pes21 = picked_pes21(&dir);
    view.open(&pes21);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line == "(none)"),
        "the pick dropped: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line
            == "The picked save is a PES 2017 save and the open save a PES 2021 one, but both must be of the same version"),
        "the refusal's literal: {lines:?}"
    );
    assert!(
        !lines
            .iter()
            .any(|line| line.starts_with("Player ") || line.starts_with("Team ")),
        "no rows: {lines:?}"
    );
}

// SE-GUI-13
#[test]
fn the_team_filter_narrows_the_list_without_recomparing() {
    let (mut view, mut window) = open_734_with("comparator_filter", &[73401], 183);
    let dir = scratch_of(&view);
    // A second team's edit: 74301's height (189 in the fixture) to 183.
    let mut entry = player_entry(&view, 74301);
    assert_eq!(entry.basic.height, 189, "premise: 74301's height");
    entry.basic.height = 183;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    let picked = picked_save(&dir, "picked", |_| {});
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line == "Team 734 /lit/"),
        "premise: 734's heading under All: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line == "Team 743 /pol/"),
        "premise: 743's heading under All: {lines:?}"
    );
    let key = view
        .comparator
        .comparison
        .result
        .as_ref()
        .expect("the comparison landed")
        .0
        .clone();

    pick_comparator_team(&mut window, &mut view, "/lit/");
    assert_eq!(
        view.comparator.filter_team,
        TeamChoice::Team(734),
        "734 picked"
    );
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line == "Team 734 /lit/"),
        "734's heading kept: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line == "Height 185 \u{2192} 183"),
        "73401's row kept: {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line == "Team 743 /pol/"),
        "743's heading narrowed out: {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line == "Height 189 \u{2192} 183"),
        "74301's row narrowed out: {lines:?}"
    );
    assert!(
        view.comparator.comparison.pending.is_none(),
        "the filter started no comparison"
    );
    assert_eq!(
        view.comparator.comparison.result.as_ref().unwrap().0,
        key,
        "the held result's key is the one before the filter"
    );

    pick_comparator_team(&mut window, &mut view, "ALL");
    assert_eq!(view.comparator.filter_team, TeamChoice::All, "ALL again");
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line == "Team 734 /lit/")
            && lines.iter().any(|line| line == "Team 743 /pol/"),
        "both headings under All again: {lines:?}"
    );
    assert!(
        view.comparator.comparison.pending.is_none(),
        "still no comparison run"
    );
}

// SE-GUI-13
#[test]
fn the_team_filter_keeps_an_only_in_section_by_its_roster() {
    let (mut view, mut window) = open_734_with("comparator_only_in_filter", &[], 183);
    let dir = scratch_of(&view);
    // `the_players_only_in_one_save_list_under_their_headings`' pair:
    // 73410 renumbered out of the picked save — 9_999_999 (on no picked
    // roster) is picked-only, 73410 (on 734's open roster) open-only. And
    // 74302 renumbered with his place on 743's picked roster: 9_999_998
    // is picked-only on the picked save's 743, 74302 open-only on the open
    // save's.
    let picked = picked_save(&dir, "picked", |file| {
        file.player_mut(73410).expect("73410 is a player").id = 9_999_999;
        file.player_mut(74302).expect("74302 is a player").id = 9_999_998;
        file.team_mut(743)
            .expect("team 743")
            .roster
            .iter_mut()
            .find(|entry| entry.player_id == 74302)
            .expect("74302 on 743's roster")
            .player_id = 9_999_998;
    });
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line == "Only in the picked save")
            && lines.iter().any(|line| line == "Only in the open save"),
        "premise: both sections under All: {lines:?}"
    );

    pick_comparator_team(&mut window, &mut view, "/lit/");
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line == "Only in the open save"),
        "73410's section keeps its heading: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line == "Player 73410 - 46 - DFW"),
        "73410's head kept: {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line == "Only in the picked save"),
        "the picked-only section's heading is gone: {lines:?}"
    );

    // On 743, each section keeps its 743 player: the picked-only one by
    // the picked save's roster, the open-only one by the open save's.
    pick_comparator_team(&mut window, &mut view, "/pol/");
    let lines = window_lines(&mut window, &mut view, "Comparator");
    // The combo's popup can still be open, its team names among the lines:
    // the sections' headings and heads alone are compared.
    assert_eq!(
        lines
            .iter()
            .map(String::as_str)
            .filter(|line| line.starts_with("Only in ") || line.starts_with("Player "))
            .collect::<Vec<_>>(),
        [
            "Only in the picked save",
            "Player 9999998 - 50 - STOP WATCHING PORN",
            "Only in the open save",
            "Player 74302 - 50 - STOP WATCHING PORN",
        ],
        "743's only-in players under their headings: {lines:?}"
    );

    // A team no line holds.
    pick_comparator_team(&mut window, &mut view, "/pw/");
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        !lines.iter().any(|line| line == "Only in the open save")
            && !lines.iter().any(|line| line == "Only in the picked save"),
        "no section holds a 744 player: {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "No differences in this team."),
        "the empty-team read: {lines:?}"
    );
}
