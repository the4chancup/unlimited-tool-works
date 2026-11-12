//! SE-GUI-56 and SE-GUI-72: Ctrl+C and Ctrl+V on the player list's rows
//! and the Tactics view's preset and formation selectors, the events the
//! window sends (`Event::Copy`, `Event::Paste` carrying the system
//! clipboard's text), within one window and from one window to another.

use pes_savefile::interchange::team_toml::TeamToml;

use crate::session::import_sections::{Sections, keep_sections};
use crate::view::interchange::{Import, held, stage};

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

/// Ctrl+C: one frame carrying `Event::Copy`; the text that frame put on
/// the system clipboard, `None` when it put none.
fn ctrl_c(window: &mut Headless, view: &mut SaveView) -> Option<String> {
    send(window, view, vec![egui::Event::Copy]);
    window.commands.iter().find_map(|command| match command {
        egui::OutputCommand::CopyText(text) => Some(text.clone()),
        _ => None,
    })
}

/// Ctrl+V as the window hears it: one frame carrying the system
/// clipboard's `text`.
fn ctrl_v(window: &mut Headless, view: &mut SaveView, text: &str) {
    send(window, view, vec![egui::Event::Paste(text.to_owned())]);
}

/// Ctrl+C where the last click names a copy: its clipboard text.
fn copy(window: &mut Headless, view: &mut SaveView) -> String {
    ctrl_c(window, view).expect("Ctrl+C put a text on the system clipboard")
}

/// The clipboard text's first two lines: the naming line and the kind.
fn head(text: &str) -> Vec<&str> {
    text.lines().take(2).collect()
}

fn ctrl_z(window: &mut Headless, view: &mut SaveView) {
    send(
        window,
        view,
        vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)],
    );
}

/// A window of its own with the save at `path` open, as a second Save
/// editor window is.
fn open_window(path: &Path) -> (Headless, SaveView) {
    let mut view = SaveView::new();
    view.open(path);
    (Headless::sized(DEFAULT_PANE.x, 900.0), view)
}

/// A click on the button labelled `label` (a Tactics selector's choice).
fn click_button(window: &mut Headless, view: &mut SaveView, label: &str) {
    let nodes = draw(window, view);
    click(window, node_labeled(&nodes, Role::Button, label), view);
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
    let copied = copy(&mut window, &mut view);
    click_player_row(&mut window, &mut view, 701, 1);
    ctrl_v(&mut window, &mut view, &copied);

    assert_eq!(
        player_entry(&view, b),
        expected,
        "B's record is what Copy stats from A writes"
    );
    // One Ctrl+Z restores B.
    ctrl_z(&mut window, &mut view);
    assert_eq!(player_entry(&view, b), b_before, "one undo restores B");
}

// SE-GUI-56: Ctrl+V arriving as `PasteImage` (the clipboard holding an
// image with no text form) pastes nothing: the copy is the clipboard's
// text, and an image replaced it.
#[test]
fn ctrl_v_arriving_as_paste_image_pastes_nothing() {
    let path = stats_fixture("clip_paste_image");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let b = 70102;
    let b_before = player_entry(&view, b);

    let mut window = Headless::sized(DEFAULT_PANE.x, 900.0);
    click_player_row(&mut window, &mut view, 701, 0);
    copy(&mut window, &mut view);
    click_player_row(&mut window, &mut view, 701, 1);
    send(
        &mut window,
        &mut view,
        vec![egui::Event::PasteImage(std::sync::Arc::new(
            egui::ColorImage::filled([1, 1], egui::Color32::BLACK),
        ))],
    );

    assert_eq!(player_entry(&view, b), b_before, "B is untouched");
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "no undo entry was pushed"
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
    let copied = copy(&mut window, &mut view);
    click_team_row(&mut window, &mut view, 702);
    let before = team_entry(&view, 702);
    ctrl_v(&mut window, &mut view, &copied);

    assert_gameplan_import(&mut window, &mut view, &expected, &before);
}

/// `expected`'s Gameplan import written on its team and its rostered
/// players, as one undo entry that restores the team to `before`.
fn assert_gameplan_import(
    window: &mut Headless,
    view: &mut SaveView,
    expected: &Import,
    before: &TeamEntry,
) {
    let team_id = expected.team.id;
    assert_eq!(
        team_entry(view, team_id),
        expected.team,
        "{team_id} holds what the Gameplan import writes"
    );
    for player in &expected.players {
        assert_eq!(
            &player_entry(view, player.id),
            player,
            "{team_id}'s rostered players as the import leaves them"
        );
    }
    // One sealed entry, the roster's players with the team in it.
    ctrl_z(window, view);
    assert_eq!(
        team_entry(view, team_id),
        *before,
        "one undo restores {team_id}"
    );
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
    let copied = copy(&mut window, &mut view);
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "2"),
        &mut view,
    );
    assert_eq!(view.tactics.preset, 1, "preset 2 is on screen");
    ctrl_v(&mut window, &mut view, &copied);

    assert_eq!(
        team_entry(&view, 701).tactics.presets[1],
        preset1,
        "the shown preset took the copied one"
    );
    ctrl_z(&mut window, &mut view);
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
    let copied = copy(&mut window, &mut view);
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "In-possession"),
        &mut view,
    );
    assert_eq!(view.tactics.formation, 1, "in-possession is on screen");
    ctrl_v(&mut window, &mut view, &copied);

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
    let copied = copy(&mut window, &mut view);
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
    ctrl_v(&mut window, &mut view, &copied);

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
    let copied = copy(&mut window, &mut view);
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
    ctrl_v(&mut window, &mut view, &copied);

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
    let copied = copy(&mut window, &mut view);
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

    ctrl_v(&mut window, &mut view, &copied);
    assert_eq!(
        player_entry(&view, b),
        b_before,
        "a text field's own paste, no record written"
    );
    // Ctrl+C copies no record either: whatever the field put on the
    // clipboard, it is no copy of B's stats.
    let field_copy = ctrl_c(&mut window, &mut view);
    assert!(
        !field_copy.is_some_and(|text| text.contains("# copied:")),
        "the field's own copy"
    );

    // The field's own paste left the copy's text in the filter; the
    // click defocuses the field and selects B's row again.
    view.filter.clear();
    click_player_row(&mut window, &mut view, 701, 1);
    ctrl_v(&mut window, &mut view, &copied);
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
    let copied = copy(&mut window, &mut view);
    click_team_row(&mut window, &mut view, 701);
    ctrl_v(&mut window, &mut view, &copied);

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
    let copied = copy(&mut window, &mut view);
    assert_eq!(
        head(&copied)[0],
        format!("# Unlimited Tool Works: the Kick-off formation of {name}")
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
    let copied = copy(&mut window, &mut view);
    assert_eq!(
        head(&copied)[0],
        format!("# Unlimited Tool Works: preset 2 of {name}")
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
    let copied = copy(&mut window, &mut view);
    assert_eq!(
        head(&copied)[0],
        format!("# Unlimited Tool Works: the stats of {name}")
    );
}

/// The Stats section alone chosen: what a stats copy keeps.
const STATS: Sections = Sections {
    stats: true,
    aesthetics: false,
    gameplan: false,
};

/// `document` through its text and back, as the clipboard carries it.
fn through_text(document: &TeamToml) -> TeamToml {
    TeamToml::parse(&document.to_toml().unwrap()).unwrap()
}

/// The PES 21 fixture with /co/'s first player's stats made distinct (the
/// fixture's players are placeholder clones), the keys PES 20 and later
/// alone store among them.
fn pes21_stats_fixture(test: &str) -> PathBuf {
    let path = fixture(PesVersion::Pes21, test);
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let id = file.team(714).unwrap().roster[0].player_id;
    let player = file.player_mut(id).unwrap();
    player.basic.height = 191;
    player.stats.speed = 90;
    player.stats.tight_possession = Some(61);
    player.stats.aggression = Some(55);
    player.stats.playing_attitude = Some(1);
    player.positions.playable[5] = 2;
    player.positions.playing_style = 3;
    player.skills.skills[3] = true;
    player.skills.com_styles[2] = true;
    player.edit_flags.abilities = true;
    file.save(&path).unwrap();
    path
}

// Every /co/ player's stats onto /3/'s player of the same slot, where the
// players are real (PES 17 and 19), and one made distinct on PES 21.
#[test]
fn a_stats_document_writes_what_copy_stats_writes() {
    let cases = [
        (
            PesVersion::Pes17,
            fixture(PesVersion::Pes17, "clip_stats_17"),
            23,
        ),
        (
            PesVersion::Pes19,
            fixture(PesVersion::Pes19, "clip_stats_19"),
            23,
        ),
        (PesVersion::Pes21, pes21_stats_fixture("clip_stats_21"), 1),
    ];
    for (version, path, slots) in cases {
        let file = EditFile::from_bytes(&std::fs::read(&path).unwrap()).unwrap();
        let players: Vec<&PlayerEntry> = file.players().iter().collect();
        let (from, onto) = (714, 701);
        let mut exported =
            TeamToml::from_team(version, file.team(from).unwrap(), &players).unwrap();
        keep_sections(&mut exported, STATS);
        // Copy stats onto each slot's B, on a second session of the save.
        let mut twin = Session::open(&path, Recorder::default()).unwrap();
        for slot in 0..slots {
            let key = slot + 1;
            let a = file.team(from).unwrap().roster[usize::from(slot)].player_id;
            let b = file.team(onto).unwrap().roster[usize::from(slot)].player_id;
            let document = TeamToml {
                pes_version: Some(version),
                players: [(key, exported.players[&key].clone())].into(),
                ..TeamToml::default()
            };
            let mut team = file.team(onto).unwrap().clone();
            let mut written = file.players().to_vec();
            through_text(&document)
                .apply(version, &mut team, &mut written)
                .unwrap();
            twin.copy_stats(b, a);
            let expected = twin.file().player(b).unwrap().clone();
            assert_ne!(
                file.player(b).unwrap(),
                &expected,
                "premise: {version:?} slot {slot}: Copy stats changes B"
            );
            assert_eq!(
                written.iter().find(|player| player.id == b).unwrap(),
                &expected,
                "{version:?} slot {slot}: the document writes what Copy stats writes"
            );
            assert_eq!(team, *file.team(onto).unwrap(), "the team is untouched");
        }
    }
}

#[test]
fn a_preset_and_a_formation_survive_the_document() {
    for version in [PesVersion::Pes17, PesVersion::Pes19, PesVersion::Pes21] {
        let file = EditFile::from_bytes(&fixture_save(version)).unwrap();
        // The cup's teams (701 to 799), whose tactics are their members'.
        let teams: Vec<&TeamEntry> = file
            .teams()
            .iter()
            .filter(|team| (701..800).contains(&team.id))
            .collect();
        assert!(
            teams
                .iter()
                .flat_map(|team| &team.tactics.presets)
                .any(|preset| preset.man_marking != [0; 11]
                    && preset.formations[1] != Default::default()
                    && preset.attack_instructions.is_some()),
            "premise: {version:?} holds a preset unlike a blank team's"
        );
        for team in teams {
            let exported = TeamToml::from_team(version, team, &[]).unwrap();
            for (index, preset) in team.tactics.presets.iter().enumerate() {
                // Written as preset 1 and read back onto a blank team's
                // preset 1: every field must come back.
                let mut document = TeamToml {
                    pes_version: Some(version),
                    ..TeamToml::default()
                };
                document.tactics.presets[0] = exported.tactics.presets[index];
                let mut blank = TeamEntry {
                    id: team.id,
                    roster: team.roster.clone(),
                    ..TeamEntry::default()
                };
                through_text(&document)
                    .apply(version, &mut blank, &mut [])
                    .unwrap();
                assert_eq!(
                    blank.tactics.presets[0],
                    *preset,
                    "{version:?} team {} preset {}",
                    team.id,
                    index + 1
                );
                for (f, formation) in preset.formations.iter().enumerate() {
                    let mut document = TeamToml {
                        pes_version: Some(version),
                        ..TeamToml::default()
                    };
                    document.tactics.presets[0].formations[0] =
                        exported.tactics.presets[index].formations[f];
                    let mut blank = TeamEntry::default();
                    through_text(&document)
                        .apply(version, &mut blank, &mut [])
                        .unwrap();
                    assert_eq!(blank.tactics.presets[0].formations[0], *formation);
                }
            }
        }
    }
}

/// The save `view` holds.
fn file(view: &SaveView) -> &EditFile {
    view.session.as_ref().expect("an open save").file()
}

/// What importing team `from_team`'s Team TOML document from `from`, Stats
/// alone, writes onto `onto_team` of `onto`: each roster slot's player
/// takes the stats of the player in the same slot.
fn stats_import(from: &EditFile, from_team: u32, onto: &EditFile, onto_team: u32) -> Import {
    let players: Vec<&PlayerEntry> = from.players().iter().collect();
    let document =
        TeamToml::from_team(from.version(), from.team(from_team).unwrap(), &players).unwrap();
    stage(onto, onto.team(onto_team).unwrap(), &document, STATS).unwrap()
}

/// `id`'s entry as `import` leaves it.
fn imported_player(import: &Import, id: u32) -> PlayerEntry {
    import
        .players
        .iter()
        .find(|player| player.id == id)
        .expect("a rostered player")
        .clone()
}

// SE-GUI-72
#[test]
fn a_players_stats_copied_in_one_window_paste_into_a_save_of_another_version() {
    // B holds Long-range Shooting (skill 39), which PES 19 does not store:
    // an import from a PES 19 document leaves it as B holds it, where one
    // read as PES 21's own would clear it.
    let path21 = fixture(PesVersion::Pes21, "clip_cross_stats_21");
    let mut save21 = EditFile::from_bytes(&std::fs::read(&path21).unwrap()).unwrap();
    let id = save21.team(701).unwrap().roster[0].player_id;
    save21.player_mut(id).unwrap().skills.skills[39] = true;
    save21.save(&path21).unwrap();
    let (mut window19, mut view19) =
        open_window(&fixture(PesVersion::Pes19, "clip_cross_stats_19"));
    let (mut window21, mut view21) = open_window(&path21);
    view19.team = TeamChoice::Team(714);
    view21.team = TeamChoice::Team(701);
    let roster = |view: &SaveView, team: u32, slot: usize| {
        file(view).team(team).unwrap().roster[slot].player_id
    };
    let (a, b, b_next) = (
        roster(&view19, 714, 0),
        roster(&view21, 701, 0),
        roster(&view21, 701, 1),
    );
    let import = stats_import(file(&view19), 714, file(&view21), 701);
    let expected = imported_player(&import, b);
    let before = player_entry(&view21, b);
    let team_before = team_entry(&view21, 701);
    assert_ne!(expected, before, "premise: the import changes B");
    assert_ne!(
        imported_player(&import, b_next),
        player_entry(&view21, b_next),
        "premise: the whole team's import would change the next slot too"
    );

    click_player_row(&mut window19, &mut view19, 714, 0);
    let copied = copy(&mut window19, &mut view19);
    let name = display_name(&player_entry(&view19, a).name);
    assert_eq!(
        head(&copied),
        [
            format!("# Unlimited Tool Works: the stats of {name}").as_str(),
            "# copied: stats"
        ]
    );
    click_player_row(&mut window21, &mut view21, 701, 0);
    let next_before = player_entry(&view21, b_next);
    ctrl_v(&mut window21, &mut view21, &copied);

    assert_eq!(
        player_entry(&view21, b),
        expected,
        "B holds what the import writes on his slot"
    );
    assert!(
        player_entry(&view21, b).skills.skills[39],
        "B keeps the skill PES 19 does not store"
    );
    assert_eq!(
        player_entry(&view21, b_next),
        next_before,
        "the next slot's player is untouched"
    );
    assert_eq!(
        team_entry(&view21, 701),
        team_before,
        "the team is untouched"
    );
    ctrl_z(&mut window21, &mut view21);
    assert_eq!(player_entry(&view21, b), before, "one undo restores B");
}

// SE-GUI-72
#[test]
fn a_teams_gameplan_copied_in_one_window_pastes_into_another() {
    let (mut window_a, mut view_a) =
        open_window(&fixture(PesVersion::Pes17, "clip_cross_gameplan_a"));
    let (mut window_b, mut view_b) =
        open_window(&fixture(PesVersion::Pes17, "clip_cross_gameplan_b"));
    let target = file(&view_b).team(702).unwrap();
    let expected = stage(
        file(&view_b),
        target,
        &gameplan_document(&view_a, 701),
        GAMEPLAN,
    )
    .expect("701's document stages onto 702");
    assert!(expected.changes, "premise: the Gameplans differ");

    click_team_row(&mut window_a, &mut view_a, 701);
    let copied = copy(&mut window_a, &mut view_a);
    let name = display_name(&team_entry(&view_a, 701).name);
    assert_eq!(
        head(&copied),
        [
            format!("# Unlimited Tool Works: the Gameplan of {name}").as_str(),
            "# copied: gameplan"
        ]
    );
    click_team_row(&mut window_b, &mut view_b, 702);
    let before = team_entry(&view_b, 702);
    ctrl_v(&mut window_b, &mut view_b, &copied);

    assert_gameplan_import(&mut window_b, &mut view_b, &expected, &before);
}

// The same version's import writes the record as it was
// (`a_preset_and_a_formation_survive_the_document`), so the shown preset
// is compared with the copied record itself.
// SE-GUI-72
#[test]
fn a_preset_copied_in_one_window_pastes_over_the_preset_shown_in_another() {
    let (mut window_a, mut view_a) = open_window(&selector_fixture("clip_cross_preset_a"));
    let (mut window_b, mut view_b) =
        open_window(&fixture(PesVersion::Pes21, "clip_cross_preset_b"));
    open_team_view(&mut window_a, &mut view_a, 701);
    click_button(&mut window_a, &mut view_a, "1");
    let copied = copy(&mut window_a, &mut view_a);
    let name = display_name(&team_entry(&view_a, 701).name);
    assert_eq!(
        head(&copied),
        [
            format!("# Unlimited Tool Works: preset 1 of {name}").as_str(),
            "# copied: preset"
        ]
    );
    let preset1 = team_entry(&view_a, 701).tactics.presets[0].clone();

    open_team_view(&mut window_b, &mut view_b, 701);
    click_button(&mut window_b, &mut view_b, "2");
    assert_eq!(view_b.tactics.preset, 1, "preset 2 is on screen");
    let before = team_entry(&view_b, 701);
    assert_ne!(
        before.tactics.presets[1], preset1,
        "premise: the presets differ"
    );
    ctrl_v(&mut window_b, &mut view_b, &copied);

    let mut expected = before.clone();
    expected.tactics.presets[1] = preset1;
    assert_eq!(
        team_entry(&view_b, 701),
        expected,
        "preset 2 took the copied preset, and nothing else changed"
    );
    ctrl_z(&mut window_b, &mut view_b);
    assert_eq!(team_entry(&view_b, 701), before, "one undo restores it");
}

// SE-GUI-72
#[test]
fn a_formation_copied_in_one_window_pastes_over_the_formation_shown_in_another() {
    let (mut window_a, mut view_a) = open_window(&selector_fixture("clip_cross_formation_a"));
    let (mut window_b, mut view_b) = open_window(&selector_fixture("clip_cross_formation_b"));
    open_team_view(&mut window_a, &mut view_a, 701);
    click_button(&mut window_a, &mut view_a, "Kick-off");
    let copied = copy(&mut window_a, &mut view_a);
    let name = display_name(&team_entry(&view_a, 701).name);
    assert_eq!(
        head(&copied),
        [
            format!("# Unlimited Tool Works: the Kick-off formation of {name}").as_str(),
            "# copied: formation"
        ]
    );
    let kick_off = team_entry(&view_a, 701).tactics.presets[0].formations[0].clone();

    open_team_view(&mut window_b, &mut view_b, 701);
    click_button(&mut window_b, &mut view_b, "In-possession");
    assert_eq!(view_b.tactics.formation, 1, "in-possession is on screen");
    let before = team_entry(&view_b, 701);
    assert_ne!(
        before.tactics.presets[0].formations[1], kick_off,
        "premise: the formations differ"
    );
    ctrl_v(&mut window_b, &mut view_b, &copied);

    let mut expected = before.clone();
    expected.tactics.presets[0].formations[1] = kick_off;
    assert_eq!(
        team_entry(&view_b, 701),
        expected,
        "in-possession took the copied formation, and nothing else changed"
    );
    ctrl_z(&mut window_b, &mut view_b);
    assert_eq!(team_entry(&view_b, 701), before, "one undo restores it");
}

// SE-GUI-72
#[test]
fn a_copy_pastes_after_another_save_opens_in_the_same_window() {
    let path17 = stats_fixture("clip_open_keeps_17");
    let (mut window, mut view) = open_window(&path17);
    view.team = TeamChoice::Team(701);
    click_player_row(&mut window, &mut view, 701, 0);
    let copied = copy(&mut window, &mut view);

    view.open(&fixture(PesVersion::Pes21, "clip_open_keeps_21"));
    view.team = TeamChoice::Team(701);
    let source = EditFile::from_bytes(&std::fs::read(&path17).unwrap()).unwrap();
    let b = file(&view).team(701).unwrap().roster[0].player_id;
    let expected = imported_player(&stats_import(&source, 701, file(&view), 701), b);
    assert_ne!(
        expected,
        player_entry(&view, b),
        "premise: the import changes the player"
    );
    click_player_row(&mut window, &mut view, 701, 0);
    ctrl_v(&mut window, &mut view, &copied);

    assert_eq!(
        player_entry(&view, b),
        expected,
        "the other save's player holds what the import writes"
    );
}

// SE-GUI-72
#[test]
fn clipboard_text_that_is_no_copy_of_the_targets_kind_changes_nothing() {
    // A Gameplan and a preset copied in another window.
    let (mut window_a, mut view_a) = open_window(&selector_fixture("clip_no_copy_a"));
    click_team_row(&mut window_a, &mut view_a, 701);
    let gameplan = copy(&mut window_a, &mut view_a);
    click_button(&mut window_a, &mut view_a, "1");
    let preset = copy(&mut window_a, &mut view_a);
    let [naming, kind] = [0, 1].map(|line| preset.lines().nth(line).unwrap());
    let body: String = preset
        .lines()
        .skip(2)
        .map(|line| format!("{line}\n"))
        .collect();

    // Preset 1 copied in this window too, and preset 2 shown: a paste that
    // wrote anything over it would be a change the assertions see.
    let (mut window, mut view) = open_window(&selector_fixture("clip_no_copy_b"));
    open_team_view(&mut window, &mut view, 701);
    click_button(&mut window, &mut view, "1");
    copy(&mut window, &mut view);
    click_button(&mut window, &mut view, "2");
    let before = team_entry(&view, 701);
    let assert_unchanged = |view: &SaveView, case: &str| {
        assert_eq!(
            team_entry(view, 701),
            before,
            "{case}: the team is untouched"
        );
        let session = view.session.as_ref().unwrap();
        assert!(!session.can_undo(), "{case}: no undo entry was pushed");
        assert!(!session.is_dirty(), "{case}: nothing was written");
    };
    for (case, text) in [
        ("no Team TOML", "x".to_owned()),
        ("a Gameplan onto a preset", gameplan.clone()),
        ("no comment lines", body.clone()),
        (
            "a kind no copy names",
            format!("{naming}\n# copied: kit\n{body}"),
        ),
        (
            "a document that does not parse",
            format!("{naming}\n{kind}\n[tactics\n"),
        ),
    ] {
        ctrl_v(&mut window, &mut view, &text);
        assert_unchanged(&view, case);
    }

    // A preset onto the shown formation.
    click_button(&mut window, &mut view, "1");
    click_button(&mut window, &mut view, "In-possession");
    assert_eq!(view.tactics.formation, 1, "in-possession is on screen");
    ctrl_v(&mut window, &mut view, &preset);
    assert_unchanged(&view, "a preset onto a formation");
}
