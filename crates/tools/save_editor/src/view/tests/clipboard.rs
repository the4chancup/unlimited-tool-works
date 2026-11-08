//! SE-GUI-56: Ctrl+C and Ctrl+V on the player list's rows and the
//! Tactics view's preset and formation selectors, the events the window
//! sends (`Event::Copy`, `Event::Paste`).

use pes_savefile::interchange::team_toml::TeamToml;

use crate::session::import_sections::{Sections, keep_sections};
use crate::view::interchange::{held, stage};

use super::*;

/// The Gameplan section alone chosen: what a team row's copy keeps and
/// a team row's paste writes, as the import panel's Gameplan choice.
const GAMEPLAN: Sections = Sections {
    stats: false,
    aesthetics: false,
    gameplan: true,
};

/// The row label `player_row` draws for `team_id`'s roster `slot`.
fn player_row_label(view: &SaveView, team_id: u32, slot: u8) -> String {
    let file = view.session.as_ref().expect("an open save").file();
    let player_id = file.team(team_id).expect("the team").roster[usize::from(slot)].player_id;
    let player = file.player(player_id).expect("a rostered player");
    let position = POSITIONS
        .get(usize::from(player.positions.registered))
        .copied()
        .unwrap_or("?");
    format!("{:02} {position} {}", slot + 1, player.name)
}

/// A click on `team_id`'s roster `slot`'s row.
fn click_player_row(window: &mut Headless, view: &mut SaveView, team_id: u32, slot: u8) {
    let label = player_row_label(view, team_id, slot);
    let nodes = draw(window, view);
    click(window, row(&nodes, &label), view);
}

/// A click on `team_id`'s team row.
fn click_team_row(window: &mut Headless, view: &mut SaveView, team_id: u32) {
    let nodes = draw(window, view);
    click(window, team_row(&nodes, view, team_id), view);
}

/// One frame carrying `events` (a key or a clipboard event).
fn send(window: &mut Headless, view: &mut SaveView, events: Vec<egui::Event>) {
    window.frame(events, |ui| show(ui, view, &|| Vec::new()));
}

fn ctrl_c(window: &mut Headless, view: &mut SaveView) {
    send(window, view, vec![egui::Event::Copy]);
}

fn ctrl_v(window: &mut Headless, view: &mut SaveView) {
    send(window, view, vec![egui::Event::Paste("x".to_owned())]);
}

/// The PES 17 fixture with 70101's stats made distinct (the fixture's
/// players are placeholder clones), so a stats copy is a real change.
fn stats_fixture(test: &str) -> PathBuf {
    let path = fixture(PesVersion::Pes17, test);
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes17)).unwrap();
    let player = file.player_mut(70101).unwrap();
    player.stats.speed = 60;
    player.basic.height = 190;
    file.save(&path).unwrap();
    path
}

/// `id`'s team's Gameplan document, as Ctrl+C builds it.
fn gameplan_document(view: &SaveView, id: u32) -> TeamToml {
    let file = view.session.as_ref().expect("an open save").file();
    let team = file.team(id).expect("the team");
    let players: Vec<&PlayerEntry> = file.players().iter().collect();
    let mut document = TeamToml::from_team(file.version(), team, &players).unwrap();
    keep_sections(&mut document, GAMEPLAN);
    document
}

// SE-GUI-56
#[test]
fn ctrl_c_then_ctrl_v_copies_a_players_stats_onto_the_selected_row() {
    let path = stats_fixture("clip_player_stats");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let (a, b) = (70101, 70102);
    let b_before = player_entry(&view, b);
    // What the paste must write: Copy stats from A onto B, built
    // through the session on a second copy of the same save.
    let expected = {
        let mut second = Session::open(&path, Recorder::default()).unwrap();
        second.copy_stats(b, a);
        second.file().player(b).unwrap().clone()
    };
    assert_ne!(expected, b_before, "premise: the copy changes B");

    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    click_player_row(&mut window, &mut view, 701, 0);
    ctrl_c(&mut window, &mut view);
    click_player_row(&mut window, &mut view, 701, 1);
    ctrl_v(&mut window, &mut view);

    assert_eq!(
        player_entry(&view, b),
        expected,
        "B's record is what Copy stats from A writes"
    );
    // One Ctrl+Z restores B.
    send(
        &mut window,
        &mut view,
        vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)],
    );
    assert_eq!(player_entry(&view, b), b_before, "one undo restores B");
}

// SE-GUI-56: Ctrl+V arriving as `PasteImage` (the clipboard holding an
// image with no text form) pastes the clip as `Paste` does.
#[test]
fn ctrl_v_arriving_as_paste_image_pastes_the_clip() {
    let path = stats_fixture("clip_paste_image");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let (a, b) = (70101, 70102);
    let b_before = player_entry(&view, b);
    let expected = {
        let mut second = Session::open(&path, Recorder::default()).unwrap();
        second.copy_stats(b, a);
        second.file().player(b).unwrap().clone()
    };
    assert_ne!(expected, b_before, "premise: the copy changes B");

    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    click_player_row(&mut window, &mut view, 701, 0);
    ctrl_c(&mut window, &mut view);
    click_player_row(&mut window, &mut view, 701, 1);
    send(
        &mut window,
        &mut view,
        vec![egui::Event::PasteImage(std::sync::Arc::new(
            egui::ColorImage::filled([1, 1], egui::Color32::BLACK),
        ))],
    );

    assert_eq!(
        player_entry(&view, b),
        expected,
        "B's record is what Copy stats from A writes"
    );
}

// SE-GUI-56
#[test]
fn ctrl_v_of_a_team_row_writes_the_copied_teams_gameplan() {
    let path = fixture(PesVersion::Pes17, "clip_gameplan");
    let mut view = SaveView::new();
    view.open(&path);
    // The list under ALL, which `open` leaves, draws both teams' rows.
    // What the paste must write: 701's Gameplan-only document staged
    // onto 702, as the import panel's Apply would write it.
    let document = gameplan_document(&view, 701);
    assert!(held(&document).gameplan, "premise: a Gameplan is held");
    let expected = {
        let file = view.session.as_ref().unwrap().file();
        let import = stage(file, file.team(702).unwrap(), &document, GAMEPLAN)
            .expect("701's document stages onto 702");
        assert!(import.changes, "premise: the Gameplans differ");
        import
    };

    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    click_team_row(&mut window, &mut view, 701);
    ctrl_c(&mut window, &mut view);
    click_team_row(&mut window, &mut view, 702);
    let before = team_entry(&view, 702);
    ctrl_v(&mut window, &mut view);

    assert_eq!(
        team_entry(&view, 702),
        expected.team,
        "702 holds what the Gameplan import writes"
    );
    for player in &expected.players {
        assert_eq!(
            &player_entry(&view, player.id),
            player,
            "702's rostered players as the import leaves them"
        );
    }
    // One sealed entry, the roster's players with the team in it.
    send(
        &mut window,
        &mut view,
        vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)],
    );
    assert_eq!(team_entry(&view, 702), before, "one undo restores 702");
}

/// The fixture for the selector tests: preset sliders and formation
/// coordinates made distinct (the fixture's are placeholders), the
/// first preset's Fluid formation on so the formation selector clicks.
fn selector_fixture(test: &str) -> PathBuf {
    let path = fixture(PesVersion::Pes21, test);
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    for (preset, value) in [(0, 7), (1, 5), (2, 1)] {
        file.team_mut(701).unwrap().tactics.presets[preset]
            .sliders
            .support_range = value;
    }
    for (formation, x) in [(0, 10), (1, 20), (2, 30)] {
        file.team_mut(701).unwrap().tactics.presets[0].formations[formation].players[0].x = x;
    }
    file.team_mut(701).unwrap().tactics.presets[0].style.fluid = Some(true);
    file.save(&path).unwrap();
    path
}

// SE-GUI-56
#[test]
fn ctrl_v_on_the_preset_selector_writes_the_copied_preset_over_the_shown_one() {
    let path = selector_fixture("clip_preset");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    open_team_view(&mut window, &mut view, 701);
    let preset1 = team_entry(&view, 701).tactics.presets[0].clone();
    assert_ne!(
        team_entry(&view, 701).tactics.presets[1],
        preset1,
        "premise: presets 1 and 2 differ"
    );

    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "1"),
        &mut view,
    );
    ctrl_c(&mut window, &mut view);
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "2"),
        &mut view,
    );
    assert_eq!(view.tactics.preset, 1, "preset 2 is on screen");
    ctrl_v(&mut window, &mut view);

    assert_eq!(
        team_entry(&view, 701).tactics.presets[1],
        preset1,
        "the shown preset took the copied one"
    );
    send(
        &mut window,
        &mut view,
        vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)],
    );
    assert_ne!(
        team_entry(&view, 701).tactics.presets[1],
        preset1,
        "one undo restores preset 2"
    );
}

// SE-GUI-56
#[test]
fn ctrl_v_on_the_formation_selector_writes_the_copied_formation_over_the_shown_one() {
    let path = selector_fixture("clip_formation");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    open_team_view(&mut window, &mut view, 701);
    let kick_off = team_entry(&view, 701).tactics.presets[0].formations[0].clone();
    assert_ne!(
        team_entry(&view, 701).tactics.presets[0].formations[1],
        kick_off,
        "premise: the formations differ"
    );

    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Kick-off"),
        &mut view,
    );
    ctrl_c(&mut window, &mut view);
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "In-possession"),
        &mut view,
    );
    assert_eq!(view.tactics.formation, 1, "in-possession is on screen");
    ctrl_v(&mut window, &mut view);

    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].formations[1],
        kick_off,
        "the shown formation took the copied one"
    );
}

// SE-GUI-56
#[test]
fn a_paste_of_another_kind_than_the_target_does_nothing() {
    let path = selector_fixture("clip_kind_mismatch");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    open_team_view(&mut window, &mut view, 701);
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "1"),
        &mut view,
    );
    ctrl_c(&mut window, &mut view);
    // Preset 2 shown, so a paste that ignored the kinds would write the
    // copied preset 1 over it — a change the assertions can see.
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "2"),
        &mut view,
    );

    let team_before = team_entry(&view, 701);
    click_player_row(&mut window, &mut view, 701, 0);
    let player_before = player_entry(&view, 70101);
    ctrl_v(&mut window, &mut view);

    assert_eq!(team_entry(&view, 701), team_before, "the team is untouched");
    assert_eq!(
        player_entry(&view, 70101),
        player_before,
        "the selected player is untouched"
    );
    let session = view.session.as_ref().unwrap();
    assert!(!session.can_undo(), "no undo entry was pushed");
    assert!(!session.is_dirty(), "nothing was written");
}

// SE-GUI-56
#[test]
fn ctrl_v_after_an_arrow_key_pastes_onto_the_row_the_key_reached() {
    let path = stats_fixture("clip_arrow_target");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let (a, b) = (70101, 70102);
    let expected = {
        let mut second = Session::open(&path, Recorder::default()).unwrap();
        second.copy_stats(b, a);
        second.file().player(b).unwrap().clone()
    };

    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    click_player_row(&mut window, &mut view, 701, 0);
    ctrl_c(&mut window, &mut view);
    send(
        &mut window,
        &mut view,
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 1)),
        "Down reached the next row"
    );
    ctrl_v(&mut window, &mut view);

    assert_eq!(
        player_entry(&view, b),
        expected,
        "the row the arrows reached took the stats"
    );
}

// SE-GUI-56
#[test]
fn a_text_field_holding_the_keyboard_keeps_ctrl_c_and_ctrl_v() {
    let path = stats_fixture("clip_text_field");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let (a, b) = (70101, 70102);
    let b_before = player_entry(&view, b);
    let expected = {
        let mut second = Session::open(&path, Recorder::default()).unwrap();
        second.copy_stats(b, a);
        second.file().player(b).unwrap().clone()
    };

    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    click_player_row(&mut window, &mut view, 701, 0);
    ctrl_c(&mut window, &mut view);
    click_player_row(&mut window, &mut view, 701, 1);
    // The list's filter box takes the keyboard.
    let nodes = draw(&mut window, &mut view);
    let filter = nodes
        .iter()
        .find(|node| {
            node.role() == Role::TextInput && node.bounds().is_some_and(|bounds| bounds.x1 < 300.0)
        })
        .expect("the filter box's text field");
    click(&mut window, filter, &mut view);

    ctrl_v(&mut window, &mut view);
    assert_eq!(
        player_entry(&view, b),
        b_before,
        "a text field's own paste, no record written"
    );
    // Ctrl+C sets no clipboard either: the paste below still writes
    // A's stats, where a copy of B's would have made it a no-op.
    ctrl_c(&mut window, &mut view);

    // The field's own paste left "x" in the filter; the click defocuses
    // the field and selects B's row again.
    view.filter.clear();
    click_player_row(&mut window, &mut view, 701, 1);
    ctrl_v(&mut window, &mut view);
    assert_eq!(
        player_entry(&view, b),
        expected,
        "the clipboard still holds A's stats"
    );
}

// SE-GUI-56
#[test]
fn ctrl_v_of_a_teams_own_gameplan_pushes_no_undo_entry() {
    let path = fixture(PesVersion::Pes17, "clip_gameplan_same_team");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    click_team_row(&mut window, &mut view, 701);
    ctrl_c(&mut window, &mut view);
    click_team_row(&mut window, &mut view, 701);
    ctrl_v(&mut window, &mut view);

    let session = view.session.as_ref().unwrap();
    assert!(!session.can_undo(), "no undo entry was pushed");
    assert!(!session.is_dirty(), "the save is not dirty");
}

// SE-GUI-56
#[test]
fn a_press_on_the_locked_formation_selector_sets_the_clipboard_kind() {
    let path = fixture(PesVersion::Pes21, "clip_locked_formation");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    open_team_view(&mut window, &mut view, 701);
    // The fixture's preset 1 has Fluid formation off: the selector is
    // locked on Kick-off (disabled).
    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].fielded_formations(),
        [0].as_slice(),
        "premise: the shown preset fields Kick-off alone"
    );
    // A player row first, so the kind is List: only the press on the
    // locked selector can set Formation. The team row's click after it
    // brings the selector back without changing the kind.
    click_player_row(&mut window, &mut view, 701, 0);
    click_team_row(&mut window, &mut view, 701);
    let nodes = draw(&mut window, &mut view);
    let kick_off = node_labeled(&nodes, Role::Button, "Kick-off");
    assert!(kick_off.is_disabled(), "premise: the selector is locked");
    click(&mut window, kick_off, &mut view);

    let name = display_name(&team_entry(&view, 701).name);
    let (commands, _, _) = window.frame(vec![egui::Event::Copy], |ui| {
        show(ui, &mut view, &|| Vec::new());
        ui.ctx().output(|output| output.commands.clone())
    });
    let copied = commands.iter().find_map(|command| match command {
        egui::OutputCommand::CopyText(text) => Some(text),
        _ => None,
    });
    assert_eq!(
        copied,
        Some(&format!(
            "Unlimited Tool Works: the Kick-off formation of {name}"
        ))
    );
}

// A press is what sets the kind: a pointer moved over a row without
// one leaves the last press's, here the preset selector's.
// SE-GUI-56
#[test]
fn a_pointer_move_without_a_press_leaves_the_clipboard_kind() {
    let path = selector_fixture("clip_hover_no_press");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    open_team_view(&mut window, &mut view, 701);
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "2"),
        &mut view,
    );
    assert_eq!(view.tactics.preset, 1, "preset 2 is on screen");

    // One frame whose only event moves the pointer over 701's team
    // row — no button — then Ctrl+C.
    let nodes = draw(&mut window, &mut view);
    let over = centre(team_row(&nodes, &view, 701));
    send(
        &mut window,
        &mut view,
        vec![egui::Event::PointerMoved(over)],
    );
    let name = display_name(&team_entry(&view, 701).name);
    let (commands, _, _) = window.frame(vec![egui::Event::Copy], |ui| {
        show(ui, &mut view, &|| Vec::new());
        ui.ctx().output(|output| output.commands.clone())
    });
    let copied = commands.iter().find_map(|command| match command {
        egui::OutputCommand::CopyText(text) => Some(text),
        _ => None,
    });
    assert_eq!(
        copied,
        Some(&format!("Unlimited Tool Works: preset 2 of {name}"))
    );
}

// SE-GUI-56
#[test]
fn ctrl_c_names_what_it_copied_on_the_system_clipboard() {
    let path = fixture(PesVersion::Pes17, "clip_system_line");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let name = display_name(&player_entry(&view, 70101).name);

    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    click_player_row(&mut window, &mut view, 701, 0);
    let (commands, _, _) = window.frame(vec![egui::Event::Copy], |ui| {
        show(ui, &mut view, &|| Vec::new());
        ui.ctx().output(|output| output.commands.clone())
    });
    let copied = commands.iter().find_map(|command| match command {
        egui::OutputCommand::CopyText(text) => Some(text),
        _ => None,
    });
    assert_eq!(
        copied,
        Some(&format!("Unlimited Tool Works: the stats of {name}"))
    );
}

// SE-GUI-56
#[test]
fn opening_another_save_empties_the_clipboard() {
    let dir = crate::test_support::scratch("clip_open_empties");
    let path17 = dir.join("EDIT00000000");
    std::fs::write(&path17, fixture_save(PesVersion::Pes17)).unwrap();
    let path21 = dir.join("EDIT00000001");
    std::fs::write(&path21, fixture_save(PesVersion::Pes21)).unwrap();
    let mut view = SaveView::new();
    view.open(&path17);
    view.team = TeamChoice::Team(701);

    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    click_player_row(&mut window, &mut view, 701, 0);
    ctrl_c(&mut window, &mut view);

    view.open(&path21);
    let before = view.session.as_ref().unwrap().file().players().to_vec();
    ctrl_v(&mut window, &mut view);

    let session = view.session.as_ref().unwrap();
    assert_eq!(
        session.file().players().iter().collect::<Vec<_>>(),
        before.iter().collect::<Vec<_>>(),
        "no record changed"
    );
    assert!(!session.can_undo(), "no undo entry was pushed");
}
