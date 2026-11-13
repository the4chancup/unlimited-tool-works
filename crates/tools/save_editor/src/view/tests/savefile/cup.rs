use std::collections::HashMap;

use crate::test_support::TEST_SALT;
use crate::test_support::cup_save::{
    MOVED_TEAM, OUTSIDE_TEAM, assert_premises, outside_cup_file, outside_players,
};

use super::*;

/// The filter box's hint, which AccessKit gives as its placeholder.
const FILTER_HINT: &str = "Team or player: name, shirt name or ID";

/// `outside_cup_file`'s PES 17 save written under `test`'s scratch, and the unrostered
/// player's id. Team 300's first player, the unrostered player and the cup player 70101
/// carry the four visual flags, which Fix database clears on every player it repairs.
fn outside_cup_fixture(test: &str) -> (PathBuf, u32) {
    let path = crate::test_support::scratch(test).join("EDIT00000000");
    let (mut file, unrostered) = outside_cup_file(PesVersion::Pes17);
    let first = outside_players(&file)[0];
    for id in [first, unrostered, 70101] {
        let flags = &mut file.player_mut(id).unwrap().edit_flags;
        flags.face = true;
        flags.hair = true;
        flags.physique = true;
        flags.strip = true;
    }
    std::fs::write(&path, file.to_bytes(&TEST_SALT).unwrap()).unwrap();
    assert_premises(&EditFile::load(&path).unwrap(), unrostered);
    (path, unrostered)
}

/// The list's team rows and player rows the frame drew, top to bottom, as groups: each
/// team row's id and the number of player rows under it.
fn drawn_groups(nodes: &[accesskit::Node], file: &EditFile) -> Vec<(u32, usize)> {
    let team_rows: HashMap<String, u32> = file
        .teams()
        .iter()
        .map(|team| (format!("{} {}", team.id, display_name(&team.name)), team.id))
        .collect();
    let mut rows: Vec<(f64, Option<u32>)> = nodes
        .iter()
        .filter_map(|node| {
            let y = node.bounds()?.y0;
            if player_row(node) {
                return Some((y, None));
            }
            team_rows.get(node.label()?).map(|&id| (y, Some(id)))
        })
        .collect();
    rows.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("drawn bounds"));
    let mut groups: Vec<(u32, usize)> = Vec::new();
    for (_, team) in rows {
        match (team, groups.last_mut()) {
            (Some(id), _) => groups.push((id, 0)),
            (None, Some((_, players))) => *players += 1,
            (None, None) => panic!("a player row above every team row"),
        }
    }
    groups
}

/// Team `id`'s filled roster slots in `file`.
fn filled(file: &EditFile, id: u32) -> usize {
    file.team(id)
        .unwrap()
        .roster
        .iter()
        .filter(|slot| slot.player_id != 0)
        .count()
}

/// The PES 17 sample with /aco/ (703) renumbered 300 and one of its players taken off
/// every roster: the team selector offers the cup's teams alone, ALL lists none of team
/// 300's rows, a filter typed as its name or its first player's name lists nothing,
/// Multi-team AATF opens on 701 and offers and reports no team 300, and Fix database
/// leaves team 300 and both players' flags as they were.
// SE-GUI-77
#[test]
fn a_team_outside_the_cup_is_neither_listed_nor_checked_nor_repaired() {
    let (path, unrostered) = outside_cup_fixture("cup_outside_listed");
    let opened = EditFile::load(&path).unwrap();
    let outside = outside_players(&opened);
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    // Tall enough for ALL's first three teams: 701's and 702's rows, then the third's.
    let mut window = Headless::sized(DEFAULT_PANE.x, 2400.0);

    // The team selector: ALL, then every cup team in record order, and no team 300.
    let names: Vec<String> = opened
        .teams()
        .iter()
        .map(|team| display_name(&team.name))
        .collect();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let selector = nodes
        .iter()
        .filter(|node| node.role() == Role::ComboBox)
        .min_by(|a, b| {
            let x = |node: &&accesskit::Node| node.bounds().map(|r| r.x0).unwrap_or(f64::MAX);
            x(a).partial_cmp(&x(b)).expect("ordered bounds")
        })
        .expect("the team selector");
    assert_eq!(selector.value(), Some("ALL"));
    click(&mut window, selector, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let mut items: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && node
                    .label()
                    .is_some_and(|label| label == "ALL" || names.iter().any(|name| name == label))
        })
        .collect();
    items.sort_by(|a, b| {
        let y = |node: &&accesskit::Node| node.bounds().map(|r| r.y0).unwrap_or(0.0);
        y(a).partial_cmp(&y(b)).expect("ordered bounds")
    });
    let items: Vec<&str> = items.iter().filter_map(|node| node.label()).collect();
    let mut expected = vec!["ALL"];
    expected.extend(
        opened
            .teams()
            .iter()
            .zip(&names)
            .filter(|(team, _)| team.id != OUTSIDE_TEAM)
            .map(|(_, name)| name.as_str()),
    );
    assert!(
        !items.contains(&"/aco/"),
        "team 300 is not offered: {items:?}"
    );
    assert_eq!(items, expected, "the selector's entries");
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(view.team, TeamChoice::All, "premise: ALL stays chosen");

    // ALL: 701 and 702 with their squads, then 704; no row of team 300's.
    let nodes = draw(&mut window, &mut view);
    let groups = drawn_groups(&nodes, &opened);
    assert!(
        groups.iter().all(|(id, _)| *id != OUTSIDE_TEAM),
        "no row of team 300: {groups:?}"
    );
    assert_eq!(
        groups[..2],
        [(701, filled(&opened, 701)), (702, filled(&opened, 702))],
        "the first two teams' rows and squads, no other player's rows among them"
    );
    assert_eq!(
        groups[2].0, 704,
        "704's row follows 702's squad: {groups:?}"
    );

    // The filter, typed as team 300's name, then as its first player's: nothing.
    let first = display_name(&opened.player(outside[0]).unwrap().name);
    assert_eq!(first, "BUENOS DIAS, MANDY", "premise: /aco/'s first player");
    for typed in ["/aco/", first.as_str()] {
        // Enter ends the typing and lets the box go, so each text is typed after a click.
        let nodes = draw(&mut window, &mut view);
        let filter_box = nodes
            .iter()
            .find(|node| node.role() == Role::TextInput && node.placeholder() == Some(FILTER_HINT))
            .expect("the filter box");
        click(&mut window, filter_box, &mut view);
        type_text(&mut window, &mut view, typed);
        assert_eq!(view.filter, typed, "premise: the filter holds {typed:?}");
        let nodes = draw(&mut window, &mut view);
        assert_eq!(drawn_groups(&nodes, &opened), [], "{typed:?} lists nothing");
    }
    view.filter.clear();

    // Multi-team AATF: the range opens on 701, offers no team 300, and the report
    // lists none.
    open_multi_aatf(&mut window, &mut view);
    // Whichever range it opened on: a check that never lands fails here, not a wrong range.
    let pairs = frames_until(
        &mut window,
        &mut view,
        |view| {
            let check = &view.multi_aatf.check;
            check.pending.is_none() && check.result.is_some()
        },
        "the Multi-team AATF check never returned",
    );
    let lines = multi_lines(&pairs);
    let items = range_items(&mut window, &mut view, 0);
    assert!(
        !items.iter().any(|item| item.starts_with("300 ")),
        "the range offers no team 300: {items:?}"
    );
    assert_eq!(items[0], "701 /3/", "the range's first team");
    assert!(
        !lines.iter().any(|line| line.starts_with("300 ")),
        "the report has no team 300: {lines:?}"
    );
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, multi_button(&pairs, "Close"), &mut view);

    // Fix database: team 300, its players and the unrostered one as they were.
    let kit = opened
        .team(OUTSIDE_TEAM)
        .unwrap()
        .kit_slots
        .as_ref()
        .unwrap()[0];
    assert_eq!(
        kit.binding,
        MOVED_TEAM * 64,
        "premise: Fix database would bind team 300's kits to 300 * 64"
    );
    let nodes = open_savefile_menu(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Fix database\u{2026}"),
        &mut view,
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Fix database"),
        &mut view,
    );
    assert!(
        !player_entry(&view, 70101).edit_flags.face,
        "premise: Fix database cleared the cup player 70101's flags"
    );
    assert_eq!(
        team_entry(&view, OUTSIDE_TEAM),
        *opened.team(OUTSIDE_TEAM).unwrap(),
        "team 300 as it was"
    );
    for id in outside.iter().copied().chain([unrostered]) {
        assert_eq!(
            player_entry(&view, id),
            *opened.player(id).unwrap(),
            "player {id} as he was"
        );
    }
}

/// The same save, a cup player's Height typed in the strip and the save written with
/// Ctrl+S: team 300's records, its players' and the unrostered player's are the bytes
/// the opened save held.
// SE-GUI-77
#[test]
fn a_save_written_keeps_the_hidden_records_byte_for_byte() {
    let (path, unrostered) = outside_cup_fixture("cup_outside_written");
    let opened = EditFile::load(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::new();
    let height = opened.player(70101).unwrap().basic.height;
    assert_ne!(height, 190, "premise: 70101 is not 190 tall yet");

    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let field = node_id_right_of(&pairs, "Height", Role::SpinButton);
    action_focus(&mut window, &mut view, field);
    type_text(&mut window, &mut view, "190");
    assert_eq!(
        player_entry(&view, 70101).basic.height,
        190,
        "premise: the typed Height landed"
    );
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        !view.session.as_ref().unwrap().is_dirty(),
        "premise: Ctrl+S wrote the save"
    );

    let written = EditFile::load(&path).unwrap();
    assert_ne!(
        written.player_record(70101),
        opened.player_record(70101),
        "premise: the edited player's record changed"
    );
    let (team, kept) = (
        opened.team_records(OUTSIDE_TEAM).unwrap(),
        written.team_records(OUTSIDE_TEAM).unwrap(),
    );
    assert_eq!(kept.team, team.team, "team 300's team record");
    assert_eq!(kept.roster, team.roster, "team 300's roster record");
    assert_eq!(kept.tactics, team.tactics, "team 300's tactics record");
    assert_eq!(kept.coach, team.coach, "team 300's coach record");
    for id in outside_players(&opened).into_iter().chain([unrostered]) {
        assert_eq!(
            written.player_record(id),
            opened.player_record(id),
            "player {id}'s record"
        );
    }
}
