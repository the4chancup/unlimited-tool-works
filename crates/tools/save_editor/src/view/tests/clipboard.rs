//! SE-GUI-56 and SE-GUI-72: Ctrl+C and Ctrl+V on the player list's rows
//! and the Tactics view's preset and formation selectors, the events the
//! window sends (`Event::Copy`, `Event::Paste` carrying the system
//! clipboard's text), within one window and from one window to another.

use std::collections::BTreeMap;

use pes_savefile::interchange::team_toml::TeamToml;
use pes_savefile::model::instruction::Instruction;
use pes_savefile::model::tactics::{AdvancedInstruction, TeamAutoFlags, TeamTactics, bench_fill};
use pes_savefile::schema::instruction::encode;

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

/// The Stats section alone chosen: what a stats copy keeps.
const STATS: Sections = Sections {
    stats: true,
    aesthetics: false,
    gameplan: false,
};

/// The save `view` holds.
fn file(view: &SaveView) -> &EditFile {
    view.session.as_ref().expect("an open save").file()
}

/// The row label `player_row` draws for `team_id`'s roster `slot`.
fn player_row_label(view: &SaveView, team_id: u32, slot: u8) -> String {
    let file = file(view);
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

/// A click on the button labelled `label` (a Tactics selector's choice).
fn click_button(window: &mut Headless, view: &mut SaveView, label: &str) {
    let nodes = draw(window, view);
    click(window, node_labeled(&nodes, Role::Button, label), view);
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

/// Ctrl+Z: one frame carrying the key, the Edit menu's Undo.
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

/// Asserts that `view`'s save holds no edit: no undo entry, not dirty.
fn assert_no_edit(view: &SaveView, case: &str) {
    let session = view.session.as_ref().expect("an open save");
    assert!(!session.can_undo(), "{case}: no undo entry was pushed");
    assert!(!session.is_dirty(), "{case}: nothing was written");
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

/// What Copy stats from `a` writes onto `b` in the save at `path`, built
/// through a second session of it. Asserts that it differs from `b` as he
/// stands, so a paste that wrote nothing cannot pass.
fn copy_stats_result(path: &Path, a: u32, b: u32) -> PlayerEntry {
    let mut second = Session::open(path, Recorder::default()).unwrap();
    let before = second.file().player(b).unwrap().clone();
    second.copy_stats(b, a);
    let after = second.file().player(b).unwrap().clone();
    assert_ne!(after, before, "premise: Copy stats from {a} changes {b}");
    after
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

/// `id`'s team's Gameplan document, as Ctrl+C builds it.
fn gameplan_document(view: &SaveView, id: u32) -> TeamToml {
    let file = file(view);
    let team = file.team(id).expect("the team");
    let mut document = TeamToml::from_team(file.version(), team, &[]).unwrap();
    keep_sections(&mut document, GAMEPLAN);
    document
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

// SE-GUI-56
#[test]
fn ctrl_c_then_ctrl_v_copies_a_players_stats_onto_the_selected_row() {
    let path = stats_fixture("clip_player_stats");
    let (mut window, mut view) = open_window(&path);
    view.team = TeamChoice::Team(701);
    let (a, b) = (70101, 70102);
    let b_before = player_entry(&view, b);
    let expected = copy_stats_result(&path, a, b);

    click_player_row(&mut window, &mut view, 701, 0);
    let copied = copy(&mut window, &mut view);
    let (a_name, b_name) = (
        display_name(&player_entry(&view, a).name),
        display_name(&player_entry(&view, b).name),
    );
    assert_eq!(
        view.notice.as_ref().map(|notice| notice.text.as_str()),
        Some(format!("{a_name}'s stats copied.").as_str()),
        "the copy's notice"
    );
    click_player_row(&mut window, &mut view, 701, 1);
    ctrl_v(&mut window, &mut view, &copied);

    assert_eq!(
        player_entry(&view, b),
        expected,
        "B's record is what Copy stats from A writes"
    );
    assert_eq!(
        view.notice.as_ref().map(|notice| notice.text.as_str()),
        Some(format!("{a_name}'s stats pasted onto {b_name}.").as_str()),
        "the paste's notice"
    );
    // One Ctrl+Z restores B.
    ctrl_z(&mut window, &mut view);
    assert_eq!(player_entry(&view, b), b_before, "one undo restores B");
}

// Ctrl+V arriving as `PasteImage` (the clipboard holding an image with no
// text form) pastes nothing: the copy is the clipboard's text, and an
// image replaced it.
// SE-GUI-72
#[test]
fn ctrl_v_arriving_as_paste_image_pastes_nothing() {
    let path = stats_fixture("clip_paste_image");
    let (mut window, mut view) = open_window(&path);
    view.team = TeamChoice::Team(701);
    let b = 70102;
    let b_before = player_entry(&view, b);

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
    assert_no_edit(&view, "an image");
}

// SE-GUI-56
#[test]
fn ctrl_v_of_a_team_row_writes_the_copied_teams_gameplan() {
    let (mut window, mut view) = open_window(&fixture(PesVersion::Pes17, "clip_gameplan"));
    // The list under ALL, which `open` leaves, draws both teams' rows.
    // What the paste must write: 701's Gameplan-only document staged
    // onto 702, as the import panel's Apply would write it.
    let document = gameplan_document(&view, 701);
    assert!(held(&document).gameplan, "premise: a Gameplan is held");
    let expected = stage(
        file(&view),
        file(&view).team(702).unwrap(),
        &document,
        GAMEPLAN,
    )
    .expect("701's document stages onto 702");
    assert!(expected.changes, "premise: the Gameplans differ");

    click_team_row(&mut window, &mut view, 701);
    let copied = copy(&mut window, &mut view);
    let (from_name, onto_name) = (
        display_name(&team_entry(&view, 701).name),
        display_name(&team_entry(&view, 702).name),
    );
    assert_eq!(
        view.notice.as_ref().map(|notice| notice.text.as_str()),
        Some(format!("{from_name}'s Gameplan copied.").as_str()),
        "the copy's notice"
    );
    click_team_row(&mut window, &mut view, 702);
    let before = team_entry(&view, 702);
    ctrl_v(&mut window, &mut view, &copied);

    assert_gameplan_import(&mut window, &mut view, &expected, &before);
    assert_eq!(
        view.notice.as_ref().map(|notice| notice.text.as_str()),
        Some(format!("{from_name}'s Gameplan pasted onto {onto_name}.").as_str()),
        "the paste's notice"
    );
}

// SE-GUI-56
#[test]
fn ctrl_v_on_the_preset_selector_writes_the_copied_preset_over_the_shown_one() {
    let (mut window, mut view) = open_window(&selector_fixture("clip_preset"));
    open_team_view(&mut window, &mut view, 701);
    let preset1 = team_entry(&view, 701).tactics.presets[0].clone();
    assert_ne!(
        team_entry(&view, 701).tactics.presets[1],
        preset1,
        "premise: presets 1 and 2 differ"
    );

    click_button(&mut window, &mut view, "1");
    let copied = copy(&mut window, &mut view);
    let name = display_name(&team_entry(&view, 701).name);
    assert_eq!(
        view.notice.as_ref().map(|notice| notice.text.as_str()),
        Some(format!("{name}'s preset 1 copied.").as_str()),
        "the copy's notice"
    );
    click_button(&mut window, &mut view, "2");
    assert_eq!(view.tactics.preset, 1, "preset 2 is on screen");
    ctrl_v(&mut window, &mut view, &copied);

    assert_eq!(
        team_entry(&view, 701).tactics.presets[1],
        preset1,
        "the shown preset took the copied one"
    );
    assert_eq!(
        view.notice.as_ref().map(|notice| notice.text.as_str()),
        Some(format!("{name}'s preset 1 pasted onto {name}'s preset 2.").as_str()),
        "the paste's notice"
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
    let (mut window, mut view) = open_window(&selector_fixture("clip_formation"));
    open_team_view(&mut window, &mut view, 701);
    let kick_off = team_entry(&view, 701).tactics.presets[0].formations[0].clone();
    assert_ne!(
        team_entry(&view, 701).tactics.presets[0].formations[1],
        kick_off,
        "premise: the formations differ"
    );

    click_button(&mut window, &mut view, "Kick-off");
    let copied = copy(&mut window, &mut view);
    let name = display_name(&team_entry(&view, 701).name);
    assert_eq!(
        view.notice.as_ref().map(|notice| notice.text.as_str()),
        Some(format!("{name}'s preset 1 Kick-off formation copied.").as_str()),
        "the copy's notice"
    );
    click_button(&mut window, &mut view, "In-possession");
    assert_eq!(view.tactics.formation, 1, "in-possession is on screen");
    ctrl_v(&mut window, &mut view, &copied);

    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].formations[1],
        kick_off,
        "the shown formation took the copied one"
    );
    assert_eq!(
        view.notice.as_ref().map(|notice| notice.text.as_str()),
        Some(
            format!(
                "{name}'s preset 1 Kick-off formation pasted onto {name}'s preset 1 In-possession formation."
            )
            .as_str()
        ),
        "the paste's notice"
    );
}

// Each copy pasted onto a target of another kind, where a paste that
// ignored the kind would write something the assertions see (a preset
// onto a player row writes nothing either way: its document holds no
// player table).
// SE-GUI-56
#[test]
fn a_paste_of_another_kind_than_the_target_does_nothing() {
    let (mut window, mut view) = open_window(&selector_fixture("clip_kind_mismatch"));
    open_team_view(&mut window, &mut view, 701);
    // Preset 2 copied: written as a Gameplan, it would land on preset 1,
    // which differs.
    click_button(&mut window, &mut view, "2");
    let preset = copy(&mut window, &mut view);
    assert_eq!(head(&preset)[1], "# copied: preset");
    click_team_row(&mut window, &mut view, 701);
    let gameplan = copy(&mut window, &mut view);
    assert_eq!(head(&gameplan)[1], "# copied: gameplan");
    let team_before = team_entry(&view, 701);
    assert_ne!(
        team_before.tactics.presets[0], team_before.tactics.presets[1],
        "premise: presets 1 and 2 differ"
    );
    let player_before = player_entry(&view, 70101);
    view.notice = None;

    // Each target: a click selecting it, the copy of another kind. The
    // player row comes last: its click leaves the team view.
    type Select = fn(&mut Headless, &mut SaveView);
    let cases: [(&str, Select, &str); 3] = [
        (
            "a Gameplan onto the shown preset 2",
            |window, view| click_button(window, view, "2"),
            &gameplan,
        ),
        (
            "a preset onto the team row",
            |window, view| click_team_row(window, view, 701),
            &preset,
        ),
        (
            "a preset onto a player row",
            |window, view| click_player_row(window, view, 701, 0),
            &preset,
        ),
    ];
    for (case, select, text) in cases {
        select(&mut window, &mut view);
        ctrl_v(&mut window, &mut view, text);
        assert_eq!(
            team_entry(&view, 701),
            team_before,
            "{case}: the team is untouched"
        );
        assert_eq!(
            player_entry(&view, 70101),
            player_before,
            "{case}: the player is untouched"
        );
        assert_no_edit(&view, case);
        assert!(
            view.notice.is_none(),
            "{case}: the refused paste publishes no notice"
        );
    }
}

// SE-GUI-56
#[test]
fn ctrl_v_after_an_arrow_key_pastes_onto_the_row_the_key_reached() {
    let path = stats_fixture("clip_arrow_target");
    let (mut window, mut view) = open_window(&path);
    view.team = TeamChoice::Team(701);
    let (a, b) = (70101, 70102);
    let expected = copy_stats_result(&path, a, b);

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
    let (mut window, mut view) = open_window(&path);
    view.team = TeamChoice::Team(701);
    let (a, b) = (70101, 70102);
    let b_before = player_entry(&view, b);
    let expected = copy_stats_result(&path, a, b);

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

    // The field's own paste left the copy's text in the filter, its
    // line breaks turned to spaces as a single-line field holds it; the
    // click defocuses the field and selects B's row again.
    assert!(
        view.filter.contains("# copied: stats"),
        "the field's paste left the copy's text in the filter"
    );
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
    let (mut window, mut view) =
        open_window(&fixture(PesVersion::Pes17, "clip_gameplan_same_team"));
    click_team_row(&mut window, &mut view, 701);
    let copied = copy(&mut window, &mut view);
    click_team_row(&mut window, &mut view, 701);
    ctrl_v(&mut window, &mut view, &copied);

    assert_no_edit(&view, "the team's own Gameplan");
}

// SE-GUI-56
#[test]
fn a_press_on_the_locked_formation_selector_sets_the_clipboard_kind() {
    let (mut window, mut view) = open_window(&fixture(PesVersion::Pes21, "clip_locked_formation"));
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
        format!("# Unlimited Tool Works: {name}'s preset 1 Kick-off formation")
    );
}

// A press is what sets the kind: a pointer moved over a row without
// one leaves the last press's, here the preset selector's.
// SE-GUI-56
#[test]
fn a_pointer_move_without_a_press_leaves_the_clipboard_kind() {
    let (mut window, mut view) = open_window(&selector_fixture("clip_hover_no_press"));
    open_team_view(&mut window, &mut view, 701);
    click_button(&mut window, &mut view, "2");
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
        format!("# Unlimited Tool Works: {name}'s preset 2")
    );
}

// SE-GUI-56
#[test]
fn ctrl_c_names_what_it_copied_on_the_system_clipboard() {
    let (mut window, mut view) = open_window(&fixture(PesVersion::Pes17, "clip_system_line"));
    view.team = TeamChoice::Team(701);
    let name = display_name(&player_entry(&view, 70101).name);

    click_player_row(&mut window, &mut view, 701, 0);
    let copied = copy(&mut window, &mut view);
    assert_eq!(
        head(&copied)[0],
        format!("# Unlimited Tool Works: {name}'s stats")
    );
}

/// `document` through its text and back, as the clipboard carries it.
fn through_text(document: &TeamToml) -> TeamToml {
    TeamToml::parse(&document.to_toml().unwrap()).unwrap()
}

/// The cup's teams (701 to 799) of `file`, whose tactics are their
/// members'.
fn cup_teams(file: &EditFile) -> Vec<&TeamEntry> {
    file.teams()
        .iter()
        .filter(|team| (701..800).contains(&team.id))
        .collect()
}

/// The first of `teams` that `unlike` holds for: the team a document is
/// applied onto, so a key the document dropped leaves that team's own
/// value, which the assertion sees.
fn unlike_team<'a>(
    teams: &[&'a TeamEntry],
    unlike: impl Fn(&TeamEntry) -> bool,
    what: &str,
) -> &'a TeamEntry {
    teams
        .iter()
        .copied()
        .find(|team| unlike(team))
        .unwrap_or_else(|| panic!("premise: a cup team whose {what}"))
}

/// A team that is `team` (its id and roster) with `tactics`: the target
/// of a document's apply.
fn target_with(team: &TeamEntry, tactics: &TeamTactics) -> TeamEntry {
    TeamEntry {
        id: team.id,
        roster: team.roster.clone(),
        tactics: tactics.clone(),
        ..TeamEntry::default()
    }
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

// Each preset and each formation written as preset 1's, read back onto a
// team whose own differs, so a key the document drops shows.
#[test]
fn a_preset_and_a_formation_survive_the_document() {
    for version in [PesVersion::Pes17, PesVersion::Pes19, PesVersion::Pes21] {
        let file = EditFile::from_bytes(&fixture_save(version)).unwrap();
        let teams = cup_teams(&file);
        assert!(
            teams
                .iter()
                .flat_map(|team| &team.tactics.presets)
                .any(|preset| preset.man_marking != [0; 11]
                    && preset.formations[1] != Default::default()
                    && preset.attack_instructions.is_some()),
            "premise: {version:?} holds a preset unlike a blank team's"
        );
        for team in &teams {
            let exported = TeamToml::from_team(version, team, &[]).unwrap();
            for (index, preset) in team.tactics.presets.iter().enumerate() {
                let label = format!("{version:?} team {} preset {}", team.id, index + 1);
                let seed = unlike_team(
                    &teams,
                    |other| {
                        let theirs = &other.tactics.presets[index];
                        theirs != preset
                            && (0..3).all(|f| theirs.formations[f] != preset.formations[f])
                    },
                    &format!("preset and formations differ from {label}'s"),
                );
                let mut document = TeamToml {
                    pes_version: Some(version),
                    ..TeamToml::default()
                };
                document.tactics.presets[0] = exported.tactics.presets[index];
                let mut target = target_with(team, &seed.tactics);
                target.tactics.presets[0] = seed.tactics.presets[index].clone();
                let seeded = target.clone();
                through_text(&document)
                    .apply(version, &mut target, &mut [])
                    .unwrap();
                assert_eq!(target.tactics.presets[0], *preset, "{label}");
                assert_eq!(
                    target.tactics.presets[1..],
                    seeded.tactics.presets[1..],
                    "{label}: the other presets stay"
                );
                for (f, formation) in preset.formations.iter().enumerate() {
                    let mut document = TeamToml {
                        pes_version: Some(version),
                        ..TeamToml::default()
                    };
                    document.tactics.presets[0].formations[0] =
                        exported.tactics.presets[index].formations[f];
                    let mut target = target_with(team, &seed.tactics);
                    target.tactics.presets[0].formations[0] =
                        seed.tactics.presets[index].formations[f].clone();
                    through_text(&document)
                        .apply(version, &mut target, &mut [])
                        .unwrap();
                    assert_eq!(
                        target.tactics.presets[0].formations[0],
                        *formation,
                        "{label} formation {}",
                        f + 1
                    );
                }
            }
        }
    }
}

// A team's whole `[tactics]` table, the Gameplan a team row copies,
// written and read back onto a team whose own tactics differ: the record
// comes back entire but for the bench's last nine entries. The document
// carries the bench's first twelve, and the import writes the other nine
// as the game fills them, which is not always what the save stored (255
// on some teams).
#[test]
fn a_gameplan_survives_the_document() {
    for version in [PesVersion::Pes17, PesVersion::Pes19, PesVersion::Pes21] {
        let file = EditFile::from_bytes(&fixture_save(version)).unwrap();
        let teams = cup_teams(&file);
        for team in &teams {
            let label = format!("{version:?} team {}", team.id);
            let ours = &team.tactics;
            let seed = unlike_team(
                &teams,
                |other| {
                    let theirs = &other.tactics;
                    theirs.starting_eleven != ours.starting_eleven
                        && theirs.bench_order[..12] != ours.bench_order[..12]
                        && theirs.set_pieces != ours.set_pieces
                        && theirs.players_to_join_attack != ours.players_to_join_attack
                        && (0..3).all(|i| {
                            theirs.presets[i] != ours.presets[i]
                                && (0..3).all(|f| {
                                    theirs.presets[i].formations[f] != ours.presets[i].formations[f]
                                })
                        })
                },
                &format!(
                    "lineup, bench, set pieces, players to join attack \
                     and presets differ from {label}'s"
                ),
            );
            let document = TeamToml {
                pes_version: Some(version),
                tactics: TeamToml::from_team(version, team, &[]).unwrap().tactics,
                ..TeamToml::default()
            };
            let mut target = target_with(team, &seed.tactics);
            // The PES 17 and 21 fixtures hold the same auto flags on
            // every cup team, so no seed differs on them there: each of
            // the target's four is set by hand to another value than the
            // source's, since the import writes each as a key of its own.
            let flipped = |flag: Option<bool>| Some(!flag.unwrap_or(false));
            target.tactics.auto = TeamAutoFlags {
                substitution: Some(u8::from(ours.auto.substitution != Some(1))),
                offside_trap: flipped(ours.auto.offside_trap),
                preset_change: flipped(ours.auto.preset_change),
                attack_defence_levels: flipped(ours.auto.attack_defence_levels),
            };
            through_text(&document)
                .apply(version, &mut target, &mut [])
                .unwrap();
            let theirs = &target.tactics;
            assert_eq!(theirs.starting_eleven, ours.starting_eleven, "{label}");
            assert_eq!(theirs.bench_order[..12], ours.bench_order[..12], "{label}");
            assert_eq!(theirs.set_pieces, ours.set_pieces, "{label}");
            assert_eq!(
                theirs.players_to_join_attack, ours.players_to_join_attack,
                "{label}"
            );
            assert_eq!(theirs.auto, ours.auto, "{label}");
            assert_eq!(theirs.presets, ours.presets, "{label}");
            let first_twelve: [u8; 12] = ours.bench_order[..12].try_into().unwrap();
            assert_eq!(
                theirs.bench_order[12..],
                bench_fill(&ours.starting_eleven, &first_twelve),
                "{label}: the bench's other nine are the game's own fill"
            );
        }
    }
}

// A Gameplan, preset or formation copy keeps no player table, so the
// team's export without its players must hold the same `[tactics]` (and
// roster-order notes) as the export with them.
#[test]
fn a_teams_export_without_its_players_holds_the_same_tactics() {
    for version in [PesVersion::Pes17, PesVersion::Pes19, PesVersion::Pes21] {
        let file = EditFile::from_bytes(&fixture_save(version)).unwrap();
        let players: Vec<&PlayerEntry> = file.players().iter().collect();
        for team in cup_teams(&file) {
            let with = TeamToml::from_team(version, team, &players).unwrap();
            let without = TeamToml::from_team(version, team, &[]).unwrap();
            assert!(!with.players.is_empty(), "premise: a player table");
            assert_eq!(
                without.tactics, with.tactics,
                "{version:?} team {}",
                team.id
            );
            assert_eq!(without.roster_breaks, with.roster_breaks);
        }
    }
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
            format!("# Unlimited Tool Works: {name}'s stats").as_str(),
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
            format!("# Unlimited Tool Works: {name}'s Gameplan").as_str(),
            "# copied: gameplan"
        ]
    );
    click_team_row(&mut window_b, &mut view_b, 702);
    let before = team_entry(&view_b, 702);
    ctrl_v(&mut window_b, &mut view_b, &copied);

    assert_gameplan_import(&mut window_b, &mut view_b, &expected, &before);
}

// SE-GUI-72
#[test]
fn a_teams_gameplan_pastes_into_a_save_of_another_version() {
    let (mut window17, mut view17) =
        open_window(&fixture(PesVersion::Pes17, "clip_cross_gameplan_17"));
    let (mut window21, mut view21) =
        open_window(&fixture(PesVersion::Pes21, "clip_cross_gameplan_21"));
    click_team_row(&mut window17, &mut view17, 701);
    let copied = copy(&mut window17, &mut view17);
    let name = display_name(&team_entry(&view17, 701).name);
    assert_eq!(
        head(&copied),
        [
            format!("# Unlimited Tool Works: {name}'s Gameplan").as_str(),
            "# copied: gameplan"
        ]
    );
    let document = TeamToml::parse(&copied).expect("the copy is a Team TOML document");
    assert_eq!(document.pes_version, Some(PesVersion::Pes17));
    // What importing 701's Gameplan document, Gameplan alone, writes onto
    // the PES 21 save's 702.
    let expected = stage(
        file(&view21),
        file(&view21).team(702).unwrap(),
        &gameplan_document(&view17, 701),
        GAMEPLAN,
    )
    .expect("the PES 17 document stages onto the PES 21 team");
    assert!(expected.changes, "premise: the import changes 702");

    click_team_row(&mut window21, &mut view21, 702);
    let before = team_entry(&view21, 702);
    ctrl_v(&mut window21, &mut view21, &copied);

    assert_gameplan_import(&mut window21, &mut view21, &expected, &before);
}

// The same version's import writes the record as it was
// (`a_preset_and_a_formation_survive_the_document`), so the shown preset
// is compared with the copied record itself. Preset 2 copied onto preset
// 3, so a copy or a paste that read or wrote another index shows.
// SE-GUI-72
#[test]
fn a_preset_copied_in_one_window_pastes_over_the_preset_shown_in_another() {
    let (mut window_a, mut view_a) = open_window(&selector_fixture("clip_cross_preset_a"));
    let (mut window_b, mut view_b) = open_window(&selector_fixture("clip_cross_preset_b"));
    open_team_view(&mut window_a, &mut view_a, 701);
    click_button(&mut window_a, &mut view_a, "2");
    let copied = copy(&mut window_a, &mut view_a);
    let name = display_name(&team_entry(&view_a, 701).name);
    assert_eq!(
        head(&copied),
        [
            format!("# Unlimited Tool Works: {name}'s preset 2").as_str(),
            "# copied: preset"
        ]
    );
    let presets = team_entry(&view_a, 701).tactics.presets.clone();
    assert_ne!(presets[0], presets[1], "premise: presets 1 and 2 differ");

    open_team_view(&mut window_b, &mut view_b, 701);
    click_button(&mut window_b, &mut view_b, "3");
    assert_eq!(view_b.tactics.preset, 2, "preset 3 is on screen");
    let before = team_entry(&view_b, 701);
    assert_ne!(
        before.tactics.presets[2], presets[1],
        "premise: the presets differ"
    );
    ctrl_v(&mut window_b, &mut view_b, &copied);

    let mut expected = before.clone();
    expected.tactics.presets[2] = presets[1].clone();
    assert_eq!(
        team_entry(&view_b, 701),
        expected,
        "preset 3 took the copied preset, and nothing else changed"
    );
    ctrl_z(&mut window_b, &mut view_b);
    assert_eq!(team_entry(&view_b, 701), before, "one undo restores it");
}

// In-possession copied onto Out-of-possession, so a copy or a paste that
// read or wrote another formation shows.
// SE-GUI-72
#[test]
fn a_formation_copied_in_one_window_pastes_over_the_formation_shown_in_another() {
    let (mut window_a, mut view_a) = open_window(&selector_fixture("clip_cross_formation_a"));
    let (mut window_b, mut view_b) = open_window(&selector_fixture("clip_cross_formation_b"));
    open_team_view(&mut window_a, &mut view_a, 701);
    click_button(&mut window_a, &mut view_a, "In-possession");
    let copied = copy(&mut window_a, &mut view_a);
    let name = display_name(&team_entry(&view_a, 701).name);
    assert_eq!(
        head(&copied),
        [
            format!("# Unlimited Tool Works: {name}'s preset 1 In-possession formation").as_str(),
            "# copied: formation"
        ]
    );
    let formations = team_entry(&view_a, 701).tactics.presets[0]
        .formations
        .clone();
    assert_ne!(
        formations[0], formations[1],
        "premise: Kick-off and In-possession differ"
    );

    open_team_view(&mut window_b, &mut view_b, 701);
    click_button(&mut window_b, &mut view_b, "Out-of-possession");
    assert_eq!(
        view_b.tactics.formation, 2,
        "out-of-possession is on screen"
    );
    let before = team_entry(&view_b, 701);
    assert_ne!(
        before.tactics.presets[0].formations[2], formations[1],
        "premise: the formations differ"
    );
    ctrl_v(&mut window_b, &mut view_b, &copied);

    let mut expected = before.clone();
    expected.tactics.presets[0].formations[2] = formations[1].clone();
    assert_eq!(
        team_entry(&view_b, 701),
        expected,
        "out-of-possession took the copied formation, and nothing else changed"
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

// A copy from a save of another PES version can leave values the target
// does not store out of the paste: the notice ends with the count, from
// the staged import's notes. Here every note is a key PES 17 lacks.
// SE-GUI-72
#[test]
fn a_paste_from_another_version_ends_its_notice_with_the_dropped_count() {
    let (mut window21, mut view21) = open_window(&pes21_stats_fixture("clip_tail_21"));
    let (mut window17, mut view17) = open_window(&fixture(PesVersion::Pes17, "clip_tail_17"));
    click_player_row(&mut window21, &mut view21, 714, 0);
    let copied = copy(&mut window21, &mut view21);
    let (a, b) = (
        file(&view21).team(714).unwrap().roster[0].player_id,
        file(&view17).team(701).unwrap().roster[0].player_id,
    );
    let (a_name, b_name) = (
        display_name(&player_entry(&view21, a).name),
        display_name(&player_entry(&view17, b).name),
    );

    // The notes the paste's staged import carries: the copy's one player
    // table placed on the target's first slot, as `paste` builds it.
    let document = TeamToml::parse(&copied).expect("the copy is a Team TOML document");
    let mut placed = TeamToml {
        pes_version: document.pes_version,
        ..TeamToml::default()
    };
    placed.players = BTreeMap::from([(
        1,
        document
            .players
            .into_values()
            .next()
            .expect("a stats copy holds one player table"),
    )]);
    let expected = stage(
        file(&view17),
        file(&view17).team(701).unwrap(),
        &placed,
        STATS,
    )
    .expect("the document stages onto 701");
    let count: usize = expected.notes.iter().map(|group| group.notes.len()).sum();
    assert!(count >= 3, "premise: several keys PES 17 lacks: {count}");
    assert!(
        expected
            .notes
            .iter()
            .all(|group| group.code == "import_key_not_in_version"),
        "premise: every note a key the version does not store"
    );

    click_player_row(&mut window17, &mut view17, 701, 0);
    ctrl_v(&mut window17, &mut view17, &copied);

    assert_eq!(
        view17.notice.as_ref().map(|notice| notice.text.as_str()),
        Some(
            format!(
                "{a_name}'s stats pasted onto {b_name}; {count} values PES 2017 does not store were left out"
            )
            .as_str()
        ),
        "the notice ends with the dropped count"
    );
}

// A note that is not a dropped key — here an instruction PES 17 lacks,
// written off — turns the tail into "left out or changed to fit".
// SE-GUI-72
#[test]
fn a_paste_leaving_a_value_changed_says_so_in_its_notice() {
    let (mut window21, mut view21) = open_window(&fixture(PesVersion::Pes21, "clip_tail_mixed_21"));
    for instruction in [Instruction::WingBack, Instruction::Anchoring] {
        assert_eq!(
            encode(PesVersion::Pes17, instruction),
            None,
            "premise: PES 17 has no {instruction:?}"
        );
    }
    let stored = |instruction| AdvancedInstruction {
        instruction: encode(PesVersion::Pes21, instruction).expect("a PES 21 instruction"),
        player: 0,
    };
    let set_instructions = |view: &mut SaveView, instructions| {
        let mut team = team_entry(view, 701);
        team.tactics.presets[0].attack_instructions = Some(instructions);
        view.session.as_mut().unwrap().edit_team(1, team);
    };
    let (mut window17, mut view17) = open_window(&fixture(PesVersion::Pes17, "clip_tail_mixed_17"));
    let name = display_name(&team_entry(&view17, 701).name);

    // The notes a copy pasted now stages against 701 of the PES 17 save.
    let notes_of = |view: &SaveView, copied: &str| {
        let document = TeamToml::parse(copied).expect("the copy is a Team TOML document");
        let expected = stage(
            file(view),
            file(view).team(701).unwrap(),
            &document,
            GAMEPLAN,
        )
        .expect("the document stages onto 701");
        assert!(
            expected
                .notes
                .iter()
                .all(|group| group.code == "import_value_not_encodable"),
            "premise: every note of another kind"
        );
        expected
            .notes
            .iter()
            .map(|group| group.notes.len())
            .sum::<usize>()
    };

    click_team_row(&mut window17, &mut view17, 701);
    for (instructions, count, was_were) in [
        (
            [stored(Instruction::WingBack), stored(Instruction::Off)],
            1,
            "was",
        ),
        (
            [
                stored(Instruction::WingBack),
                stored(Instruction::Anchoring),
            ],
            2,
            "were",
        ),
    ] {
        set_instructions(&mut view21, instructions);
        click_team_row(&mut window21, &mut view21, 701);
        let copied = copy(&mut window21, &mut view21);
        assert_eq!(
            notes_of(&view17, &copied),
            count,
            "premise: one note per instruction PES 17 lacks"
        );
        ctrl_v(&mut window17, &mut view17, &copied);
        assert_eq!(
            view17.notice.as_ref().map(|notice| notice.text.as_str()),
            Some(
                format!(
                    "{name}'s Gameplan pasted onto {name}; {count} value{} {was_were} left out or changed to fit PES 2017",
                    if count == 1 { "" } else { "s" }
                )
                .as_str()
            ),
            "the notice ends with the changed-or-dropped count"
        );
    }
}

// The same tail, one note: the singular.
// SE-GUI-72
#[test]
fn a_paste_leaving_one_value_out_uses_the_singular_tail() {
    let (mut window17, mut view17) = open_window(&fixture(PesVersion::Pes17, "clip_tail_one_17"));
    click_player_row(&mut window17, &mut view17, 701, 0);
    // A PES 21 stats copy by hand: `playing_attitude`, the one key PES 17
    // lacks, so the staged import holds the one `NotInThisVersion` note.
    let text = "# Unlimited Tool Works: SOMEONE's stats\n# copied: stats\n\
                pes_version = 21\n\n[players.01.stats]\nplaying_attitude = 1\n";
    let placed = TeamToml {
        pes_version: Some(PesVersion::Pes21),
        players: BTreeMap::from([(
            1,
            TeamToml::parse("pes_version = 21\n\n[players.01.stats]\nplaying_attitude = 1\n")
                .expect("the document parses")
                .players
                .into_values()
                .next()
                .expect("one player table"),
        )]),
        ..TeamToml::default()
    };
    let expected = stage(
        file(&view17),
        file(&view17).team(701).unwrap(),
        &placed,
        STATS,
    )
    .expect("the document stages onto 701");
    let count: usize = expected.notes.iter().map(|group| group.notes.len()).sum();
    assert_eq!(count, 1, "premise: the one dropped key is one note");
    ctrl_v(&mut window17, &mut view17, text);
    let b_name = display_name(
        &player_entry(
            &view17,
            file(&view17).team(701).unwrap().roster[0].player_id,
        )
        .name,
    );
    assert_eq!(
        view17.notice.as_ref().map(|notice| notice.text.as_str()),
        Some(
            format!(
                "SOMEONE's stats pasted onto {b_name}; 1 value PES 2017 does not store was left out"
            )
            .as_str()
        ),
        "the notice ends with the singular tail"
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
    assert_eq!(head(&gameplan)[1], "# copied: gameplan");
    assert_eq!(head(&preset)[1], "# copied: preset");
    let [naming, kind] = [0, 1].map(|line| preset.lines().nth(line).unwrap());
    let body: String = preset
        .lines()
        .skip(2)
        .map(|line| format!("{line}\n"))
        .collect();

    // Preset 2 shown: a paste that wrote anything over it would be a
    // change the assertions see.
    let (mut window, mut view) = open_window(&selector_fixture("clip_no_copy_b"));
    open_team_view(&mut window, &mut view, 701);
    click_button(&mut window, &mut view, "2");
    let before = team_entry(&view, 701);
    view.notice = None;
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
        assert_eq!(
            team_entry(&view, 701),
            before,
            "{case}: the team is untouched"
        );
        assert_no_edit(&view, case);
        assert!(view.notice.is_none(), "{case}: no paste ran, so no notice");
    }

    // A preset onto the shown formation.
    click_button(&mut window, &mut view, "1");
    click_button(&mut window, &mut view, "In-possession");
    assert_eq!(view.tactics.formation, 1, "in-possession is on screen");
    ctrl_v(&mut window, &mut view, &preset);
    assert_eq!(team_entry(&view, 701), before, "a preset onto a formation");
    assert_no_edit(&view, "a preset onto a formation");
    assert!(
        view.notice.is_none(),
        "a wrong-kind paste publishes no notice"
    );
}

/// The selector fixture with /3/'s first player's playing style set to
/// 31: a byte the five-bit field holds and PES 2021's list of styles
/// does not, so the Team TOML export refuses his record.
fn unknown_style_fixture(test: &str) -> PathBuf {
    let path = selector_fixture(test);
    let mut file = EditFile::from_bytes(&std::fs::read(&path).unwrap()).unwrap();
    let id = file.team(701).unwrap().roster[0].player_id;
    file.player_mut(id).unwrap().positions.playing_style = 31;
    file.save(&path).unwrap();
    path
}

// A Gameplan, preset or formation copy holds no player table, so a player
// whose record the export refuses does not stop his team's copies.
// SE-GUI-72
#[test]
fn a_player_the_export_refuses_does_not_stop_his_teams_tactics_copies() {
    let (mut window, mut view) = open_window(&unknown_style_fixture("clip_unknown_style"));
    let players: Vec<&PlayerEntry> = file(&view).players().iter().collect();
    assert!(
        TeamToml::from_team(PesVersion::Pes21, file(&view).team(701).unwrap(), &players).is_err(),
        "premise: the team's export with its players is refused"
    );

    open_team_view(&mut window, &mut view, 701);
    let gameplan = ctrl_c(&mut window, &mut view);
    click_button(&mut window, &mut view, "1");
    let preset = ctrl_c(&mut window, &mut view);
    click_button(&mut window, &mut view, "Kick-off");
    let formation = ctrl_c(&mut window, &mut view);
    let copies = [
        ("gameplan", gameplan),
        ("preset", preset),
        ("formation", formation),
    ];
    let missing: Vec<&str> = copies
        .iter()
        .filter(|(_, copied)| copied.is_none())
        .map(|(kind, _)| *kind)
        .collect();
    assert_eq!(
        missing,
        Vec::<&str>::new(),
        "the copies not on the clipboard"
    );
    for (kind, copied) in copies {
        let copied = copied.expect("checked above");
        assert_eq!(head(&copied)[1], format!("# copied: {kind}"));
    }
}
