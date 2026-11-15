//! SE-GUI-56 and SE-GUI-72: Ctrl+C and Ctrl+V on the player list's rows
//! and the Tactics view's preset and formation selectors, the events the
//! window sends (`Event::Copy`, `Event::Paste` carrying the system
//! clipboard's text), within one window and from one window to another.

use std::collections::BTreeMap;

use pes_savefile::interchange::team_toml::TeamToml;
use pes_savefile::model::instruction::Instruction;
use pes_savefile::model::playstyle::PlayStyle;
use pes_savefile::model::tactics::{AdvancedInstruction, TeamAutoFlags, TeamTactics, bench_fill};
use pes_savefile::schema::instruction::encode;
use pes_savefile::schema::playstyle;

use crate::session::import_sections::{Sections, keep_sections};
use crate::view::interchange::{Import, NoteGroup, held, stage};

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

/// `name` picked in the list's team selector: the combo clicked, its
/// popup's `name` item picked through the AccessKit action — a popup
/// item's bounds sit outside the window, where a pointer click lands on
/// nothing.
fn pick_team(window: &mut Headless, view: &mut SaveView, name: &str) {
    // The popup can still be open on the first frame (its close lands a
    // frame after the item's action): click the combo only until the
    // item is drawn — clicking while it is open closes it. Bounded, so
    // a missing item fails rather than hangs.
    for _ in 0..10 {
        let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
        if let Some((item, _)) = pairs
            .iter()
            .find(|(_, node)| node.role() == Role::Button && node.label() == Some(name))
        {
            action_click(window, view, *item);
            return;
        }
        let combo = pairs
            .iter()
            .map(|(_, node)| node)
            .find(|node| {
                node.role() == Role::ComboBox
                    && node.bounds().is_some_and(|bounds| bounds.x1 < 300.0)
            })
            .expect("the list's team selector");
        click(window, combo, view);
    }
    panic!("the team selector never showed {name:?}");
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

/// The status bar's notice, its text and kind (`None` with none): a
/// copy's and a paste's notice is a success one, which the text alone
/// does not say.
fn notice(view: &SaveView) -> Option<(&str, NoticeKind)> {
    view.notice
        .as_ref()
        .map(|notice| (notice.text.as_str(), notice.kind))
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

/// The fixture for the selector tests: preset sliders made distinct
/// (the fixture's are placeholders), every preset's Fluid formation on
/// so the formation selector clicks, and a distinct first player's x
/// per (preset, formation) pair, so a path that reads or writes
/// another preset or formation shows.
fn selector_fixture(test: &str) -> PathBuf {
    let path = fixture(PesVersion::Pes21, test);
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    for (preset, value) in [(0, 7), (1, 5), (2, 1)] {
        file.team_mut(701).unwrap().tactics.presets[preset]
            .sliders
            .support_range = value;
    }
    for (preset, xs) in [(0, [10, 20, 30]), (1, [40, 50, 60]), (2, [70, 80, 90])] {
        for (formation, x) in xs.into_iter().enumerate() {
            file.team_mut(701).unwrap().tactics.presets[preset].formations[formation].players[0]
                .x = x;
        }
        file.team_mut(701).unwrap().tactics.presets[preset]
            .style
            .fluid = Some(true);
    }
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
        notice(&view),
        Some((
            format!("{a_name}'s stats copied.").as_str(),
            NoticeKind::Success
        )),
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
        notice(&view),
        Some((
            format!("{a_name}'s stats pasted onto {b_name}.").as_str(),
            NoticeKind::Success
        )),
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
        notice(&view),
        Some((
            format!("{from_name}'s Gameplan copied.").as_str(),
            NoticeKind::Success
        )),
        "the copy's notice"
    );
    click_team_row(&mut window, &mut view, 702);
    let before = team_entry(&view, 702);
    ctrl_v(&mut window, &mut view, &copied);

    assert_gameplan_import(&mut window, &mut view, &expected, &before);
    assert_eq!(
        notice(&view),
        Some((
            format!("{from_name}'s Gameplan pasted onto {onto_name}.").as_str(),
            NoticeKind::Success
        )),
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
        notice(&view),
        Some((
            format!("{name}'s preset 1 copied.").as_str(),
            NoticeKind::Success
        )),
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
        notice(&view),
        Some((
            format!("{name}'s preset 1 pasted onto {name}'s preset 2.").as_str(),
            NoticeKind::Success
        )),
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
        notice(&view),
        Some((
            format!("{name}'s preset 1 Kick-off formation copied.").as_str(),
            NoticeKind::Success
        )),
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
        notice(&view),
        Some((
            format!(
                "{name}'s preset 1 Kick-off formation pasted onto {name}'s preset 1 In-possession formation."
            )
            .as_str(),
            NoticeKind::Success
        )),
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
            notice(&view).is_none(),
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

// A selector's press names its target only while a team row stays
// selected: Down moved the selection to a player row, which a preset
// or a formation copy does not paste onto.
// SE-GUI-56
#[test]
fn ctrl_v_of_a_selector_copy_after_an_arrow_key_to_a_player_row_pastes_nothing() {
    // The copied choice, then the one shown: a paste that wrote would
    // land on the shown one, which the fixture made differ.
    for (copied_button, shown_button, what) in [
        ("2", "3", "a preset"),
        ("In-possession", "Out-of-possession", "a formation"),
    ] {
        let (mut window, mut view) = open_window(&selector_fixture("clip_selector_arrow"));
        open_team_view(&mut window, &mut view, 701);
        click_button(&mut window, &mut view, copied_button);
        let copied = copy(&mut window, &mut view);
        click_button(&mut window, &mut view, shown_button);
        let presets = &team_entry(&view, 701).tactics.presets;
        let shown_differs = match copied_button {
            "2" => presets[1] != presets[2],
            _ => presets[0].formations[1] != presets[0].formations[2],
        };
        assert!(
            shown_differs,
            "premise: the copied {what} differs from the one shown"
        );
        let before = team_entry(&view, 701);
        view.notice = None;

        send(
            &mut window,
            &mut view,
            vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
        );
        assert_eq!(
            view.selected,
            Some(Selection::Player(701, 0)),
            "{what}: Down reached the player row under the team's"
        );
        ctrl_v(&mut window, &mut view, &copied);

        assert_eq!(
            team_entry(&view, 701),
            before,
            "{what}: the team is untouched"
        );
        assert_no_edit(&view, &format!("{what} onto a player row"));
        assert!(
            notice(&view).is_none(),
            "{what}: the refused paste publishes no notice"
        );
    }
}

// A selector's target ends when the selection leaves its team's row:
// the team selector's pick of 702 selects its row without a list press,
// and Ctrl+C then copies 702's Gameplan, not the shown preset.
// SE-GUI-56
#[test]
fn a_selector_target_ends_when_the_team_selector_picks_another_team() {
    let (mut window, mut view) = open_window(&selector_fixture("clip_target_combo"));
    open_team_view(&mut window, &mut view, 701);
    click_button(&mut window, &mut view, "2");
    assert_eq!(view.tactics.preset, 1, "preset 2 is on screen");
    let name = display_name(&team_entry(&view, 702).name);

    pick_team(&mut window, &mut view, &name);
    assert_eq!(
        view.selected,
        Some(Selection::Team(702)),
        "the pick selected 702's team row"
    );

    let copied = copy(&mut window, &mut view);
    assert_eq!(
        head(&copied),
        [
            format!("# Unlimited Tool Works: {name}'s Gameplan").as_str(),
            "# copied: gameplan"
        ],
        "702's Gameplan is the copy, not 701's shown preset"
    );
}

// And it does not come back when a key returns to its team's row: Down
// ended it at the player row, so the row Up reaches is the Gameplan's
// target again.
// SE-GUI-56
#[test]
fn a_selector_target_ended_by_a_player_row_stays_ended_back_on_the_team_row() {
    let (mut window, mut view) = open_window(&selector_fixture("clip_target_return"));
    open_team_view(&mut window, &mut view, 701);
    click_button(&mut window, &mut view, "2");
    assert_eq!(view.tactics.preset, 1, "preset 2 is on screen");

    send(
        &mut window,
        &mut view,
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(701, 0)),
        "Down reached the player row under the team's"
    );
    send(
        &mut window,
        &mut view,
        vec![key(egui::Key::ArrowUp, egui::Modifiers::NONE, true)],
    );
    assert_eq!(
        view.selected,
        Some(Selection::Team(701)),
        "Up returned to 701's team row"
    );

    let copied = copy(&mut window, &mut view);
    let name = display_name(&team_entry(&view, 701).name);
    assert_eq!(
        head(&copied),
        [
            format!("# Unlimited Tool Works: {name}'s Gameplan").as_str(),
            "# copied: gameplan"
        ],
        "701's Gameplan is the copy, not the preset pressed before"
    );
}

// No press recorded since the save opened: the selected row is the
// target still, so Down walks to the next player and the paste of
// another window's stats copy lands on him as a click would have it.
// SE-GUI-72
#[test]
fn ctrl_v_after_down_with_no_click_pastes_onto_the_row_it_reached() {
    let (mut window_a, mut view_a) = open_window(&stats_fixture("clip_no_click_a"));
    view_a.team = TeamChoice::Team(701);
    click_player_row(&mut window_a, &mut view_a, 701, 0);
    let copied = copy(&mut window_a, &mut view_a);

    let path = stats_fixture("clip_no_click_b");
    let (mut window, mut view) = open_window(&path);
    send(
        &mut window,
        &mut view,
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE, true)],
    );
    let Some(Selection::Player(team_id, slot)) = view.selected else {
        panic!("Down reached a player row: {:?}", view.selected);
    };
    let (a, b) = (
        file(&view).team(701).unwrap().roster[0].player_id,
        file(&view).team(team_id).unwrap().roster[usize::from(slot)].player_id,
    );
    assert_ne!(a, b, "premise: the reached player is not the copied one");
    let expected = copy_stats_result(&path, a, b);
    let (a_name, b_name) = (
        display_name(&player_entry(&view, a).name),
        display_name(&player_entry(&view, b).name),
    );
    ctrl_v(&mut window, &mut view, &copied);

    assert_eq!(
        player_entry(&view, b),
        expected,
        "the row Down reached took the stats"
    );
    assert_eq!(
        notice(&view),
        Some((
            format!("{a_name}'s stats pasted onto {b_name}.").as_str(),
            NoticeKind::Success
        )),
        "the notice names the reached player"
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

// Ctrl+C is a label's own while one of its texts is selected, as a text
// field's is: a drag across the Height label selects its text, and the
// copy that lands on the system clipboard is the selection's, not the
// save's target's.
// SE-GUI-56
#[test]
fn ctrl_c_over_a_labels_selected_text_copies_the_text() {
    let path = stats_fixture("clip_label_selection");
    let (mut window, mut view) = open_window(&path);
    view.team = TeamChoice::Team(701);
    click_player_row(&mut window, &mut view, 701, 0);
    let nodes = draw(&mut window, &mut view);
    let label = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some("Height"))
        .expect("the Height label");
    let bounds = label.bounds().expect("a drawn label's bounds");
    let mid = centre(label).y;
    drag(
        &mut window,
        egui::pos2(bounds.x0 as f32 + 2.0, mid),
        egui::pos2(bounds.x1 as f32 - 2.0, mid),
        &mut view,
    );
    assert!(
        window
            .ctx
            .plugin_or_default::<egui::text_selection::LabelSelectionState>()
            .lock()
            .has_selection(),
        "premise: the drag selected the label's text"
    );
    view.notice = None;

    send(&mut window, &mut view, vec![egui::Event::Copy]);
    let copied: Vec<&str> = window
        .commands
        .iter()
        .filter_map(|command| match command {
            egui::OutputCommand::CopyText(text) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        copied,
        ["Height"],
        "the label's selected text alone is copied, not the save's target"
    );
    assert!(
        notice(&view).is_none(),
        "no save copy ran, so no copied notice"
    );
}

// Team 705's record holds 255 in the bench's last nine, so a paste that
// refilled them as the game does would be a write an undo entry and the
// dirty flag show.
// SE-GUI-56
#[test]
fn ctrl_v_of_a_teams_own_gameplan_pushes_no_undo_entry() {
    let (mut window, mut view) =
        open_window(&fixture(PesVersion::Pes17, "clip_gameplan_same_team"));
    assert!(
        team_entry(&view, 705).tactics.bench_order[12..]
            .iter()
            .all(|slot| *slot == 255),
        "premise: team 705's last nine bench entries hold 255"
    );
    view.team = TeamChoice::Team(705);
    click_team_row(&mut window, &mut view, 705);
    let copied = copy(&mut window, &mut view);
    click_team_row(&mut window, &mut view, 705);
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
    // A player row first, so no selector is named: only the press on
    // the locked one can set Formation. The team row's click after it
    // brings the selector back without naming it.
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
// as the game fills them when its eleven or twelve changed the record's,
// as the seed's do — a fill that is not always what the save stored (255
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

/// The notes a stats copy of `from_team`'s roster `slot` of `from`
/// stages onto `onto_team` of `onto`: the player's table, Stats alone,
/// placed on the target's slot as `paste` builds it — built from the
/// source save, not the clipboard text, so a copy that drops a key
/// shows here.
fn staged_notes(
    from: &EditFile,
    from_team: u32,
    onto: &EditFile,
    onto_team: u32,
    slot: u8,
) -> Vec<NoteGroup> {
    let players: Vec<&PlayerEntry> = from.players().iter().collect();
    let mut exported = TeamToml::from_team(from.version(), from.team(from_team).unwrap(), &players)
        .expect("the source team exports");
    keep_sections(&mut exported, STATS);
    // `[players.NN]` counts the roster slots from 1.
    let key = slot + 1;
    let placed = TeamToml {
        pes_version: exported.pes_version,
        players: BTreeMap::from([(
            key,
            exported.players.remove(&key).expect("the slot's table"),
        )]),
        ..TeamToml::default()
    };
    stage(onto, onto.team(onto_team).unwrap(), &placed, STATS)
        .expect("the document stages onto the target")
        .notes
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

// Preset 2's In-possession copied onto preset 3's Out-of-possession, so
// a copy or a paste that read or wrote another preset or formation shows.
// SE-GUI-72
#[test]
fn a_formation_copied_in_one_window_pastes_over_the_formation_shown_in_another() {
    let (mut window_a, mut view_a) = open_window(&selector_fixture("clip_cross_formation_a"));
    let (mut window_b, mut view_b) = open_window(&selector_fixture("clip_cross_formation_b"));
    open_team_view(&mut window_a, &mut view_a, 701);
    click_button(&mut window_a, &mut view_a, "2");
    click_button(&mut window_a, &mut view_a, "In-possession");
    let copied = copy(&mut window_a, &mut view_a);
    let name = display_name(&team_entry(&view_a, 701).name);
    assert_eq!(
        head(&copied),
        [
            format!("# Unlimited Tool Works: {name}'s preset 2 In-possession formation").as_str(),
            "# copied: formation"
        ]
    );
    let formation = team_entry(&view_a, 701).tactics.presets[1].formations[1].clone();
    assert_ne!(
        team_entry(&view_a, 701).tactics.presets[2].formations[2],
        formation,
        "premise: preset 2's In-possession and preset 3's Out-of-possession differ"
    );

    open_team_view(&mut window_b, &mut view_b, 701);
    click_button(&mut window_b, &mut view_b, "3");
    click_button(&mut window_b, &mut view_b, "Out-of-possession");
    assert_eq!(
        view_b.tactics.formation, 2,
        "out-of-possession is on screen"
    );
    let before = team_entry(&view_b, 701);
    ctrl_v(&mut window_b, &mut view_b, &copied);

    let mut expected = before.clone();
    expected.tactics.presets[2].formations[2] = formation;
    assert_eq!(
        team_entry(&view_b, 701),
        expected,
        "preset 3's out-of-possession took the copied formation, and nothing else changed"
    );
    assert_eq!(
        notice(&view_b),
        Some((
            format!(
                "{name}'s preset 2 In-possession formation pasted onto {name}'s preset 3 Out-of-possession formation."
            )
            .as_str(),
            NoticeKind::Success
        )),
        "the paste's notice"
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

// A clipboard hands the text back with Windows' line endings: the CRLF
// copy pastes as the LF one did.
// SE-GUI-72
#[test]
fn a_copy_pasted_with_crlf_line_ends_writes_the_same() {
    let path = stats_fixture("clip_crlf");
    let (mut window, mut view) = open_window(&path);
    view.team = TeamChoice::Team(701);
    let (a, b) = (70101, 70102);
    let expected = copy_stats_result(&path, a, b);

    click_player_row(&mut window, &mut view, 701, 0);
    let copied = copy(&mut window, &mut view);
    click_player_row(&mut window, &mut view, 701, 1);
    ctrl_v(&mut window, &mut view, &copied);
    let lf_notice = notice(&view)
        .map(|(text, _)| text.to_owned())
        .expect("the LF paste's notice");
    ctrl_z(&mut window, &mut view);

    click_player_row(&mut window, &mut view, 701, 1);
    ctrl_v(&mut window, &mut view, &copied.replace('\n', "\r\n"));

    assert_eq!(
        player_entry(&view, b),
        expected,
        "the CRLF copy writes what the LF one did"
    );
    assert_eq!(
        notice(&view),
        Some((lf_notice.as_str(), NoticeKind::Success)),
        "the notice is the LF paste's, byte for byte"
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
    let notes = staged_notes(file(&view21), 714, file(&view17), 701, 0);
    let count: usize = notes.iter().map(|group| group.notes.len()).sum();
    assert_eq!(count, 3, "premise: the three keys PES 17 lacks");
    assert!(
        notes
            .iter()
            .all(|group| group.code == "import_key_not_in_version"),
        "premise: every note a key the version does not store"
    );

    click_player_row(&mut window17, &mut view17, 701, 0);
    ctrl_v(&mut window17, &mut view17, &copied);

    assert_eq!(
        notice(&view17),
        Some((
            format!(
                "{a_name}'s stats pasted onto {b_name}; {count} values PES 2017 does not store were left out."
            )
            .as_str(),
            NoticeKind::Success
        )),
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
            notice(&view17),
            Some((
                format!(
                    "{name}'s Gameplan pasted onto {name}; {count} value{} {was_were} left out or changed to fit PES 2017.",
                    if count == 1 { "" } else { "s" }
                )
                .as_str(),
                NoticeKind::Success
            )),
            "the notice ends with the changed-or-dropped count"
        );
    }
}

// The tail's other shape with one of each: the notes mix a key PES 17
// lacks with a value it cannot encode — a PES 21 stats copy carrying a
// playing style PES 17 has no slot for (Roaming Flank is a PES 19+
// style) beside the keys PES 20 and later alone store.
// SE-GUI-72
#[test]
fn a_paste_mixing_dropped_and_changed_notes_uses_the_changed_tail() {
    let path21 = {
        let path = pes21_stats_fixture("clip_tail_mixed_codes_21");
        let mut file = EditFile::from_bytes(&std::fs::read(&path).unwrap()).unwrap();
        let id = file.team(714).unwrap().roster[0].player_id;
        file.player_mut(id).unwrap().positions.playing_style =
            playstyle::encode(PesVersion::Pes21, PlayStyle::RoamingFlank)
                .expect("PES 21 stores Roaming Flank");
        file.save(&path).unwrap();
        path
    };
    let (mut window21, mut view21) = open_window(&path21);
    let (mut window17, mut view17) =
        open_window(&fixture(PesVersion::Pes17, "clip_tail_mixed_codes_17"));
    view21.team = TeamChoice::Team(714);
    click_player_row(&mut window21, &mut view21, 714, 0);
    let copied = copy(&mut window21, &mut view21);
    let (a_name, b_name) = (
        display_name(
            &player_entry(
                &view21,
                file(&view21).team(714).unwrap().roster[0].player_id,
            )
            .name,
        ),
        display_name(
            &player_entry(
                &view17,
                file(&view17).team(701).unwrap().roster[0].player_id,
            )
            .name,
        ),
    );

    // The notes the paste's staged import carries: the copy's one player
    // table placed on the target's first slot, as `paste` builds it.
    let notes = staged_notes(file(&view21), 714, file(&view17), 701, 0);
    let count: usize = notes.iter().map(|group| group.notes.len()).sum();
    assert_eq!(count, 4, "premise: the keys PES 17 lacks");
    let codes: Vec<&str> = notes.iter().map(|group| group.code.as_str()).collect();
    assert!(
        codes.contains(&"import_key_not_in_version")
            && codes.contains(&"import_value_not_encodable"),
        "premise: the notes mix a dropped key with another kind: {codes:?}"
    );

    click_player_row(&mut window17, &mut view17, 701, 0);
    ctrl_v(&mut window17, &mut view17, &copied);

    assert_eq!(
        notice(&view17),
        Some((
            format!(
                "{a_name}'s stats pasted onto {b_name}; {count} values were left out or changed to fit PES 2017."
            )
            .as_str(),
            NoticeKind::Success
        )),
        "the notice ends with the changed-or-dropped count"
    );
}

// The same tail, one note: the singular.
// SE-GUI-72
#[test]
fn a_paste_leaving_one_value_out_uses_the_singular_tail() {
    let (mut window17, mut view17) = open_window(&fixture(PesVersion::Pes17, "clip_tail_one_17"));
    click_player_row(&mut window17, &mut view17, 701, 0);
    // A PES 21 stats copy by hand: `tight_possession`, one key PES 17
    // lacks, so the staged import holds the one `NotInThisVersion` note.
    let text = "# Unlimited Tool Works: SOMEONE's stats\n# copied: stats\n\
                pes_version = 21\n\n[players.01.stats]\ntight_possession = 1\n";
    // The file stand-in for the same stage: the PES 20+ keys the
    // fixture's player always carries (`aggression` and `stronger_hand`)
    // are cleared, so `tight_possession` is left as the one note.
    let mut source = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let id = source.team(714).unwrap().roster[0].player_id;
    let player = source.player_mut(id).unwrap();
    player.stats.tight_possession = Some(1);
    player.stats.aggression = None;
    player.positions.stronger_hand = None;
    let notes = staged_notes(&source, 714, file(&view17), 701, 0);
    assert_eq!(
        notes.iter().map(|group| group.notes.len()).sum::<usize>(),
        1,
        "premise: the one dropped key is one note"
    );
    ctrl_v(&mut window17, &mut view17, text);
    let b_name = display_name(
        &player_entry(
            &view17,
            file(&view17).team(701).unwrap().roster[0].player_id,
        )
        .name,
    );
    assert_eq!(
        notice(&view17),
        Some((
            format!(
                "SOMEONE's stats pasted onto {b_name}; 1 value PES 2017 does not store was left out."
            )
            .as_str(),
            NoticeKind::Success
        )),
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
        assert!(
            notice(&view).is_none(),
            "{case}: no paste ran, so no notice"
        );
    }

    // A preset onto the shown formation.
    click_button(&mut window, &mut view, "1");
    click_button(&mut window, &mut view, "In-possession");
    assert_eq!(view.tactics.formation, 1, "in-possession is on screen");
    ctrl_v(&mut window, &mut view, &preset);
    assert_eq!(team_entry(&view, 701), before, "a preset onto a formation");
    assert_no_edit(&view, "a preset onto a formation");
    assert!(
        notice(&view).is_none(),
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

    // The refused player himself: a stats copy needs the players, so
    // his row gives Ctrl+C nothing and no notice.
    click_player_row(&mut window, &mut view, 701, 0);
    assert!(
        ctrl_c(&mut window, &mut view).is_none(),
        "a stats copy the export refuses puts nothing on the clipboard"
    );
    assert!(
        notice(&view).is_none(),
        "the refused copy publishes no notice"
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

// The Edit menu's Copy does what Ctrl+C does on the same target: the
// frame's CopyText and the notice are the key's.
// SE-GUI-56
#[test]
fn the_edit_menus_copy_copies_what_ctrl_c_copies() {
    let path = stats_fixture("clip_menu_copy");
    let (mut window, mut view) = open_window(&path);
    view.team = TeamChoice::Team(701);

    click_player_row(&mut window, &mut view, 701, 0);
    let key_copy = copy(&mut window, &mut view);
    let (key_text, key_kind) = notice(&view)
        .map(|(text, kind)| (text.to_owned(), kind))
        .expect("the key's notice");
    view.notice = None;

    choose(&mut window, &mut view, "Edit", "Copy");
    let menu_copy = window
        .commands
        .iter()
        .find_map(|command| match command {
            egui::OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .expect("the menu's Copy wrote the system clipboard");
    assert_eq!(menu_copy, key_copy, "the menu copies what Ctrl+C copies");
    assert_eq!(
        notice(&view),
        Some((key_text.as_str(), key_kind)),
        "the menu copy's notice is the key's"
    );
}

// The Edit menu's Paste asks egui for the system clipboard's text: no
// record changes on the click, and the text the window then hands over
// pastes as Ctrl+V's does.
// SE-GUI-56
#[test]
fn the_edit_menus_paste_asks_for_the_clipboard_text_then_pastes_it() {
    let path = stats_fixture("clip_menu_paste");
    let (mut window, mut view) = open_window(&path);
    view.team = TeamChoice::Team(701);
    let (a, b) = (70101, 70102);
    let b_before = player_entry(&view, b);
    let expected = copy_stats_result(&path, a, b);
    let (a_name, b_name) = (
        display_name(&player_entry(&view, a).name),
        display_name(&player_entry(&view, b).name),
    );

    click_player_row(&mut window, &mut view, 701, 0);
    let copied = copy(&mut window, &mut view);
    view.notice = None;

    click_player_row(&mut window, &mut view, 701, 1);
    let nodes = open_menu(&mut window, &mut view, "Edit", "Paste");
    let at = centre(menu_item(&nodes, "Paste"));
    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (commands, _, _) = window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new());
        ui.ctx()
            .viewport_for(egui::ViewportId::ROOT, |viewport| viewport.commands.clone())
    });
    assert!(
        commands.contains(&egui::ViewportCommand::RequestPaste),
        "Paste asks the window for the clipboard's text: {commands:?}"
    );
    assert_eq!(
        player_entry(&view, b),
        b_before,
        "asking changes no record yet"
    );

    // The window's answer is the clipboard's text, as Ctrl+V hears it.
    ctrl_v(&mut window, &mut view, &copied);
    assert_eq!(
        player_entry(&view, b),
        expected,
        "B holds what Copy stats from A writes"
    );
    assert_eq!(
        notice(&view),
        Some((
            format!("{a_name}'s stats pasted onto {b_name}.").as_str(),
            NoticeKind::Success
        )),
        "the paste's notice"
    );
}

// Copy is dead on a target whose copy is not built — a player the
// export refuses — and both entries are dead with no save open, the
// row still being a target Paste offers.
// SE-GUI-56
#[test]
fn the_edit_menus_copy_is_dead_when_ctrl_c_copies_nothing() {
    let (mut window, mut view) = open_window(&unknown_style_fixture("clip_menu_refused"));
    click_player_row(&mut window, &mut view, 701, 0);
    let nodes = open_menu(&mut window, &mut view, "Edit", "Copy");
    assert!(
        menu_item(&nodes, "Copy").is_disabled(),
        "a stats copy the export refuses is not offered"
    );
    assert!(
        !menu_item(&nodes, "Paste").is_disabled(),
        "the row is a target: Paste stays enabled"
    );

    let mut view = SaveView::new();
    // The welcome panel's discovery is stubbed to find nothing.
    view.discover = Box::new(|_| Vec::new());
    let mut window = Headless::new();
    let nodes = open_menu(&mut window, &mut view, "Edit", "Copy");
    assert!(
        menu_item(&nodes, "Copy").is_disabled(),
        "no save open: nothing copies"
    );
    assert!(
        menu_item(&nodes, "Paste").is_disabled(),
        "no save open: no target"
    );
}

// The entries, in order: Undo, Redo, a separator, Copy, Paste — each
// with its shortcut text.
// SE-GUI-56
#[test]
fn the_edit_menu_is_undo_redo_then_copy_paste_under_a_separator() {
    let (mut window, mut view) = open_window(&stats_fixture("clip_menu_entries"));
    let nodes = open_menu(&mut window, &mut view, "Edit", "Copy");
    let labels = ["Undo Ctrl+Z", "Redo Ctrl+Y", "Copy Ctrl+C", "Paste Ctrl+V"];
    let mut entries: Vec<&accesskit::Node> = labels
        .iter()
        .map(|label| node_labeled(&nodes, Role::Button, label))
        .collect();
    entries.sort_by(|a, b| {
        let top = |node: &&accesskit::Node| node.bounds().expect("a drawn entry's bounds").y0;
        top(a).partial_cmp(&top(b)).expect("ordered bounds")
    });
    assert_eq!(
        entries
            .iter()
            .filter_map(|node| node.label())
            .collect::<Vec<_>>(),
        labels,
        "the menu's entries in order"
    );
    let gap = |from: &accesskit::Node, onto: &accesskit::Node| {
        onto.bounds().expect("bounds").y0 - from.bounds().expect("bounds").y1
    };
    assert_eq!(gap(entries[0], entries[1]), 0.0, "Undo meets Redo");
    assert!(
        gap(entries[1], entries[2]) > 0.0,
        "the separator stands between Redo and Copy"
    );
    assert_eq!(gap(entries[2], entries[3]), 0.0, "Copy meets Paste");
}

/// The Edit menu's Copy and Paste hovers for the target now selected,
/// each read by resting the pointer on the entry in the open menu.
fn assert_edit_menu_tips(
    window: &mut Headless,
    view: &mut SaveView,
    copy_tip: &str,
    paste_tip: &str,
) {
    let nodes = open_menu(window, view, "Edit", "Copy");
    assert_eq!(
        tooltip_containing(window, view, centre(menu_item(&nodes, "Copy")), "Copies "),
        Some(copy_tip.to_owned()),
        "Copy's hover names the copy"
    );
    assert_eq!(
        tooltip_containing(window, view, centre(menu_item(&nodes, "Paste")), "Pastes "),
        Some(paste_tip.to_owned()),
        "Paste's hover names the kind and the target"
    );
}

// Each enabled entry's hover says what it does, in the notices' words:
// Copy names its copy (`Copies …`), Paste the copy's kind and the
// target (`Pastes the copied … onto …`), for each kind of target.
// SE-GUI-56
#[test]
fn the_edit_menus_hovers_name_the_copy_and_the_target() {
    let (mut window, mut view) = open_window(&stats_fixture("clip_menu_tips_row"));
    view.team = TeamChoice::Team(701);
    click_player_row(&mut window, &mut view, 701, 0);
    let name = display_name(&player_entry(&view, 70101).name);
    assert_edit_menu_tips(
        &mut window,
        &mut view,
        &format!("Copies {name}'s stats"),
        &format!("Pastes the copied stats onto {name}"),
    );

    let (mut window, mut view) = open_window(&stats_fixture("clip_menu_tips_team"));
    click_team_row(&mut window, &mut view, 701);
    let name = display_name(&team_entry(&view, 701).name);
    assert_edit_menu_tips(
        &mut window,
        &mut view,
        &format!("Copies {name}'s Gameplan"),
        &format!("Pastes the copied Gameplan onto {name}"),
    );

    let (mut window, mut view) = open_window(&selector_fixture("clip_menu_tips_preset"));
    open_team_view(&mut window, &mut view, 701);
    click_button(&mut window, &mut view, "1");
    let name = display_name(&team_entry(&view, 701).name);
    assert_edit_menu_tips(
        &mut window,
        &mut view,
        &format!("Copies {name}'s preset 1"),
        &format!("Pastes the copied preset onto {name}'s preset 1"),
    );

    let (mut window, mut view) = open_window(&selector_fixture("clip_menu_tips_formation"));
    open_team_view(&mut window, &mut view, 701);
    click_button(&mut window, &mut view, "Kick-off");
    let name = display_name(&team_entry(&view, 701).name);
    assert_edit_menu_tips(
        &mut window,
        &mut view,
        &format!("Copies {name}'s preset 1 Kick-off formation"),
        &format!("Pastes the copied formation onto {name}'s preset 1 Kick-off formation"),
    );
}
