//! The team file panel's tests (SE-GUI-16, SE-GUI-17, SE-GUI-22): a Team
//! TOML imported with stats alone as one undoable edit, another
//! version's notes in words, a refusal drawn under the
//! choices, a squad file's pairing and its refusal, a tactics file, a
//! Texport, the aesthetics lock, the import following the save, and the
//! Team menu's entry that opens the panel.

use pes_savefile::interchange::legacy::{read_squad, read_tactics};
use pes_savefile::interchange::team_toml::{TacticsSection, TeamToml};
use pes_savefile::interchange::texport::Texport;
use pes_savefile::pes17_texport;

use crate::session::import_sections::{Sections, keep_sections};
use crate::test_support::{cli_stdout, journal_lines, steps_with, zip_entry};
use crate::view::appearance::OWNERSHIP_TIP;
use crate::view::interchange::LIST_HEIGHT;

use super::*;

/// The panel's title, its window's label.
const TITLE: &str = "Team file interchange";

/// The line under Apply when the chosen sections would change nothing.
const ALREADY_HOLDS: &str = "The team already holds every value of the chosen sections.";

/// Teams of the PES 19 fixture, a cup save whose teams hold real,
/// distinct players: `/3/`, `/cm/` and `/co/`.
const THREE: u32 = 701;
const CM: u32 = 713;
const CO: u32 = 714;

/// A squad file of PES 2019 onto PES 2017's `/co/`: the pairing list and
/// note groups whose rows carry a detail, both drawn.
const SQUAD_19: &str = "pes19_squad.4ccs";

/// `version`'s fixture open under `test` with team `target`'s row
/// selected, in a window tall enough for the panel's notes.
fn open_on(version: PesVersion, test: &str, target: u32) -> (SaveView, Headless) {
    let mut view = SaveView::new();
    view.open(&fixture(version, test));
    view.team = TeamChoice::Team(target);
    view.selected = Some(Selection::Team(target));
    (view, Headless::sized(DEFAULT_PANE.x, 1400.0))
}

/// The bytes of `name` in `pes_savefile`'s fixture folder.
fn lib_fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../libs/pes_savefile/tests/fixtures")
            .join(name),
    )
    .unwrap()
}

/// `bytes` written as `name` beside the open save, in the test's scratch
/// folder.
fn beside_save(view: &SaveView, name: &str, bytes: &[u8]) -> PathBuf {
    let path = view
        .session
        .as_ref()
        .expect("an open save")
        .path()
        .with_file_name(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

/// The Team TOML text of team `id` of the open save, as the command line
/// exports it.
fn export(view: &SaveView, id: u32) -> String {
    view.session
        .as_ref()
        .expect("an open save")
        .export_team(id)
        .unwrap()
}

/// The panel opened from the Team menu's entry.
fn open_panel(window: &mut Headless, view: &mut SaveView) {
    let nodes = open_menu(window, view, "Team", "Import / export team file");
    click(window, menu_item(&nodes, "Import / export team file"), view);
}

/// The panel's nodes on a fresh frame.
fn panel_nodes(window: &mut Headless, view: &mut SaveView) -> Vec<accesskit::Node> {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    nodes_in_window(&pairs, TITLE)
        .into_iter()
        .map(|(_, node)| node.clone())
        .collect()
}

/// The panel's node of `role` labeled `label`, `None` when it is not drawn.
fn panel_node(
    window: &mut Headless,
    view: &mut SaveView,
    role: Role,
    label: &str,
) -> Option<accesskit::Node> {
    panel_nodes(window, view)
        .into_iter()
        .find(|node| node.role() == role && node.label() == Some(label))
}

/// The panel's Label values and widget labels in top-to-bottom,
/// left-to-right order, the window's title, close button and resize
/// handles left out.
fn panel_lines(window: &mut Headless, view: &mut SaveView) -> Vec<String> {
    window_lines(window, view, TITLE)
        .into_iter()
        .filter(|text| {
            text != TITLE && text != "Close window" && !text.starts_with("Resize window")
        })
        .collect()
}

/// The texts of the panel's row holding the label `text`: every node whose
/// bounds share its vertical middle, left to right (a monospace label's
/// top sits lower than its row's, so `panel_lines` alone splits a row).
fn row_of(window: &mut Headless, view: &mut SaveView, text: &str) -> Vec<String> {
    let nodes = panel_nodes(window, view);
    let anchor = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some(text))
        .and_then(|node| node.bounds())
        .unwrap_or_else(|| panic!("the label {text:?}"));
    let middle = (anchor.y0 + anchor.y1) / 2.0;
    let mut row: Vec<(f64, String)> = nodes
        .iter()
        .filter(|node| node.role() == Role::Label)
        .filter_map(|node| Some((node.bounds()?, node.value()?.to_owned())))
        .filter(|(bounds, _)| bounds.y0 < middle && bounds.y1 > middle)
        .map(|(bounds, value)| (bounds.x0, value))
        .collect();
    row.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("drawn bounds"));
    row.into_iter().map(|(_, value)| value).collect()
}

/// `path` stubbed as the Pick dialog's choice; `Pick…` clicked.
fn pick(window: &mut Headless, view: &mut SaveView, path: PathBuf) {
    view.interchange_dialog = Box::new(move |_| Some(path.clone()));
    let button = panel_node(window, view, Role::Button, "Pick\u{2026}").expect("the Pick button");
    click(window, &button, view);
}

/// The panel open from the menu with `bytes` picked as `name`.
fn picked(window: &mut Headless, view: &mut SaveView, name: &str, bytes: &[u8]) -> PathBuf {
    let path = beside_save(view, name, bytes);
    open_panel(window, view);
    pick(window, view, path.clone());
    path
}

/// The section checkboxes drawn, left to right.
fn checkboxes(window: &mut Headless, view: &mut SaveView) -> Vec<String> {
    let mut boxes: Vec<(f64, String)> = panel_nodes(window, view)
        .into_iter()
        .filter(|node| node.role() == Role::CheckBox)
        .map(|node| {
            (
                node.bounds().expect("drawn bounds").x0,
                node.label().expect("a labeled checkbox").to_owned(),
            )
        })
        .collect();
    boxes.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("drawn bounds"));
    boxes.into_iter().map(|(_, label)| label).collect()
}

/// The section checkbox labeled `label`, which must be drawn.
fn checkbox(window: &mut Headless, view: &mut SaveView, label: &str) -> accesskit::Node {
    panel_node(window, view, Role::CheckBox, label)
        .unwrap_or_else(|| panic!("the {label} checkbox"))
}

/// Whether the checkbox labeled `label` is ticked.
fn ticked(window: &mut Headless, view: &mut SaveView, label: &str) -> bool {
    checkbox(window, view, label).toggled() == Some(accesskit::Toggled::True)
}

/// A click on the checkbox labeled `label`.
fn toggle(window: &mut Headless, view: &mut SaveView, label: &str) {
    let node = checkbox(window, view, label);
    click(window, &node, view);
}

/// The panel's Apply button, which must be drawn.
fn apply_button(window: &mut Headless, view: &mut SaveView) -> accesskit::Node {
    panel_node(window, view, Role::Button, "Apply").expect("the Apply button")
}

/// A click on Apply.
fn apply(window: &mut Headless, view: &mut SaveView) {
    let button = apply_button(window, view);
    assert!(!button.is_disabled(), "premise: Apply is enabled");
    click(window, &button, view);
}

/// Every note group opened, one AccessKit click a frame (bounded).
fn open_note_groups(window: &mut Headless, view: &mut SaveView) {
    for _ in 0..64 {
        let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
        let closed = nodes_in_window(&pairs, TITLE)
            .into_iter()
            .find(|(_, node)| {
                node.role() == Role::DisclosureTriangle && node.is_expanded() != Some(true)
            })
            .map(|(id, _)| id);
        match closed {
            Some(id) => action_click(window, view, id),
            None => return,
        }
    }
    panic!("the note groups did not all open in 64 clicks")
}

/// Team `team_id`'s rostered players in roster order, as the session holds
/// them.
fn rostered(view: &SaveView, team_id: u32) -> Vec<PlayerEntry> {
    team_entry(view, team_id)
        .roster
        .iter()
        .filter(|slot| slot.player_id != 0)
        .map(|slot| player_entry(view, slot.player_id))
        .collect()
}

/// `document` applied through the lib onto copies of team `team_id` and its
/// rostered players: the import's expected outcome.
fn lib_import(view: &SaveView, document: &TeamToml, team_id: u32) -> (TeamEntry, Vec<PlayerEntry>) {
    let session = view.session.as_ref().expect("an open save");
    let mut team = team_entry(view, team_id);
    let mut players = rostered(view, team_id);
    document
        .apply(session.version(), &mut team, &mut players)
        .expect("the lib imports the document");
    (team, players)
}

/// The Team TOML `text` reduced to `sections`, as the panel reduces a file
/// before it applies it.
fn kept(text: &str, sections: Sections) -> TeamToml {
    let mut document = TeamToml::parse(text).expect("the text parses");
    keep_sections(&mut document, sections);
    document
}

/// Asserts team `team_id` and its rostered players are `expected`, the
/// lib's import.
fn assert_imported(view: &SaveView, team_id: u32, expected: &(TeamEntry, Vec<PlayerEntry>)) {
    assert_eq!(team_entry(view, team_id), expected.0, "the lib's team");
    for (slot, (after, expected)) in rostered(view, team_id).iter().zip(&expected.1).enumerate() {
        assert_eq!(after, expected, "slot {slot}: the lib's player");
    }
    assert_eq!(
        rostered(view, team_id).len(),
        expected.1.len(),
        "every rostered player"
    );
}

/// The number of players a group's header `<subject> · <n> player(s)`
/// counts.
fn header_players(header: &str) -> usize {
    let (_, count) = header
        .rsplit_once(" \u{b7} ")
        .unwrap_or_else(|| panic!("a group header: {header:?}"));
    let count = count
        .strip_suffix(" players")
        .or_else(|| count.strip_suffix(" player"))
        .unwrap_or_else(|| panic!("a player count: {header:?}"));
    count.parse().expect("a count")
}

/// The panel's Label value or widget label `text` painted on a fresh
/// frame: its colour.
fn painted(window: &mut Headless, view: &mut SaveView, text: &str) -> egui::Color32 {
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    painted_label_color(&nodes, &shapes, text)
}

/// `/co/`'s and `/3/`'s 23 slots differ in name, abilities and look, and
/// their tactics differ: an import of one onto the other shows in every
/// slot.
fn assert_co_and_three_differ(view: &SaveView) {
    let (co, three) = (rostered(view, CO), rostered(view, THREE));
    assert_eq!(
        (co.len(), three.len()),
        (23, 23),
        "premise: 23 players each"
    );
    for (slot, (co, three)) in co.iter().zip(&three).enumerate() {
        assert_ne!(co.name, three.name, "premise: slot {slot}'s names differ");
        assert_ne!(
            co.stats, three.stats,
            "premise: slot {slot}'s abilities differ"
        );
        assert_ne!(
            co.appearance, three.appearance,
            "premise: slot {slot}'s looks differ"
        );
    }
    assert_ne!(
        team_entry(view, CO).tactics,
        team_entry(view, THREE).tactics,
        "premise: the tactics differ"
    );
}

// SE-GUI-16
#[test]
fn a_team_toml_with_stats_alone_writes_the_abilities_as_one_undoable_edit() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_stats", THREE);
    assert_co_and_three_differ(&view);
    let source = rostered(&view, CO);
    let before_team = team_entry(&view, THREE);
    let before_players = rostered(&view, THREE);
    assert_ne!(
        before_team.colors,
        team_entry(&view, CO).colors,
        "premise: /3/'s and /co/'s colours differ"
    );
    let co = export(&view, CO);
    let stats = Sections {
        stats: true,
        aesthetics: false,
        gameplan: false,
    };
    let expected = lib_import(&view, &kept(&co, stats), THREE);
    picked(&mut window, &mut view, "co.toml", co.as_bytes());
    assert!(
        panel_lines(&mut window, &mut view)
            .contains(&"A Team TOML written from a PES 2019 save.".to_owned()),
        "the intro names the file's version, the save's"
    );
    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["Stats", "Aesthetics", "Gameplan"]
    );
    toggle(&mut window, &mut view, "Aesthetics");
    toggle(&mut window, &mut view, "Gameplan");
    assert!(ticked(&mut window, &mut view, "Stats"), "stats chosen");
    assert!(
        !ticked(&mut window, &mut view, "Aesthetics"),
        "aesthetics off"
    );
    assert!(!ticked(&mut window, &mut view, "Gameplan"), "gameplan off");

    apply(&mut window, &mut view);

    assert_imported(&view, THREE, &expected);
    let team = team_entry(&view, THREE);
    assert_eq!(team.tactics, before_team.tactics, "/3/'s tactics unchanged");
    // The file's `[team]` name keys load with stats, its look with
    // aesthetics: the name is the file's, the colours stay /3/'s.
    assert_eq!(
        (before_team.name.as_str(), team.name.as_str()),
        ("/3/", "/co/"),
        "the team's name is the file's"
    );
    assert_eq!(team.colors, before_team.colors, "the colours stay /3/'s");
    for (slot, (after, (before, co))) in rostered(&view, THREE)
        .iter()
        .zip(before_players.iter().zip(&source))
        .enumerate()
    {
        assert_eq!(after.stats, co.stats, "slot {slot}: /co/'s abilities");
        assert_eq!(after.name, before.name, "slot {slot}: the name is /3/'s");
        assert_eq!(
            after.shirt_name, before.shirt_name,
            "slot {slot}: shirt name"
        );
        assert_eq!(after.appearance, before.appearance, "slot {slot}: the look");
        assert_eq!(
            after.motion, before.motion,
            "slot {slot}: every motion, the free kick's with them"
        );
        assert_eq!(
            team.roster[slot].number, before_team.roster[slot].number,
            "slot {slot}: the shirt number"
        );
    }
    assert_eq!(
        panel_lines(&mut window, &mut view)
            .last()
            .map(String::as_str),
        Some("Applied co.toml.")
    );

    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo entry");
    assert_eq!(team_entry(&view, THREE), before_team, "/3/ as before");
    assert_eq!(
        rostered(&view, THREE),
        before_players,
        "its players as before"
    );
    let lines = panel_lines(&mut window, &mut view);
    assert!(
        !lines.iter().any(|line| line.starts_with("Applied")),
        "the Applied line goes with the undo: {lines:#?}"
    );
    assert!(
        !apply_button(&mut window, &mut view).is_disabled(),
        "Apply is enabled again"
    );
}

// SE-GUI-16: the extension names the format case-insensitively, so a
// `CO.TOML` reads as a Team TOML like `co.toml` does.
#[test]
fn an_upper_case_extension_reads_as_a_team_toml() {
    let (mut view, mut window) =
        open_on(PesVersion::Pes19, "interchange_upper_case_extension", THREE);
    let co = export(&view, CO);
    picked(&mut window, &mut view, "CO.TOML", co.as_bytes());
    assert!(
        panel_lines(&mut window, &mut view)
            .contains(&"A Team TOML written from a PES 2019 save.".to_owned()),
        "the intro names a Team TOML"
    );
}

// The section choices offer only what the file holds: an aesthetics-only
// file offers Aesthetics alone, a stats-only one Stats alone.
// SE-GUI-17
#[test]
fn the_section_choices_offer_only_the_sections_the_file_holds() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_held_sections", THREE);
    let only = |stats, aesthetics, gameplan| Sections {
        stats,
        aesthetics,
        gameplan,
    };
    let co = export(&view, CO);

    let aesthetics = kept(&co, only(false, true, false)).to_toml().unwrap();
    let path = beside_save(&view, "co_aesthetics.toml", aesthetics.as_bytes());
    open_panel(&mut window, &mut view);
    pick(&mut window, &mut view, path);
    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["Aesthetics"],
        "an aesthetics-only file offers Aesthetics alone"
    );

    let stats = kept(&co, only(true, false, false)).to_toml().unwrap();
    let path = beside_save(&view, "co_stats.toml", stats.as_bytes());
    pick(&mut window, &mut view, path);
    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["Stats"],
        "a stats-only file offers Stats alone"
    );
}

/// A base-copy id of `team × 100 + slot` in the file means "no base copy":
/// imported onto another team it becomes each target player's own id, never
/// the source player's. The rule reads the file's own `[team] id`, so the
/// import must leave that id as the file has it.
#[test]
fn an_aesthetics_import_writes_each_players_own_id_where_the_file_has_no_base_copy() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_base_copy", THREE);
    let source = rostered(&view, CO);
    let marked: Vec<usize> = (0..source.len())
        .filter(|&slot| {
            let own = CO * 100 + u32::try_from(slot + 1).expect("a roster slot");
            source[slot].appearance.base_copy_id == own
        })
        .collect();
    assert!(
        !marked.is_empty(),
        "premise: some /co/ player has no base copy (his own id)"
    );
    // The marked /3/ players set to a foreign base copy first, so a write
    // that never lands leaves a value the assertion below refuses.
    let foreign: Vec<PlayerEntry> = marked
        .iter()
        .map(|&slot| {
            let mut player = rostered(&view, THREE)[slot].clone();
            player.appearance.base_copy_id = source[slot].id;
            player
        })
        .collect();
    view.session.as_mut().unwrap().edit_players(foreign);
    for &slot in &marked {
        let player = &rostered(&view, THREE)[slot];
        assert_ne!(
            player.appearance.base_copy_id,
            player.id,
            "premise: slot {} does not hold his own id",
            slot + 1
        );
    }
    let co = export(&view, CO);
    picked(&mut window, &mut view, "co.toml", co.as_bytes());
    toggle(&mut window, &mut view, "Stats");
    toggle(&mut window, &mut view, "Gameplan");

    apply(&mut window, &mut view);

    let after = rostered(&view, THREE);
    for slot in marked {
        assert_eq!(
            after[slot].appearance.base_copy_id,
            after[slot].id,
            "slot {}: his own id, not /co/'s {}",
            slot + 1,
            source[slot].id
        );
    }
}

// SE-GUI-22
#[test]
fn under_the_lock_the_aesthetics_section_is_off_and_never_written() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_locked", THREE);
    assert_co_and_three_differ(&view);
    view.aesthetics_from_exports = true;
    assert!(view.aesthetics_locked(), "premise: the lock is on");
    let before_team = team_entry(&view, THREE);
    let before = rostered(&view, THREE);
    let co = export(&view, CO);
    let unlocked = Sections {
        stats: true,
        aesthetics: false,
        gameplan: true,
    };
    let expected = lib_import(&view, &kept(&co, unlocked), THREE);
    picked(&mut window, &mut view, "co.toml", co.as_bytes());

    let aesthetics = checkbox(&mut window, &mut view, "Aesthetics");
    assert!(
        aesthetics.is_disabled(),
        "the aesthetics choice is disabled"
    );
    assert!(!ticked(&mut window, &mut view, "Aesthetics"), "and off");
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(&aesthetics), "export owns"),
        Some(OWNERSHIP_TIP.to_owned())
    );
    assert!(ticked(&mut window, &mut view, "Stats"), "stats chosen");
    assert!(
        ticked(&mut window, &mut view, "Gameplan"),
        "gameplan chosen"
    );

    apply(&mut window, &mut view);
    assert_imported(&view, THREE, &expected);
    let team = team_entry(&view, THREE);
    for (slot, (after, before)) in rostered(&view, THREE).iter().zip(&before).enumerate() {
        assert_ne!(after.stats, before.stats, "premise: slot {slot} imported");
        assert_eq!(after.name, before.name, "slot {slot}: the name");
        assert_eq!(
            after.shirt_name, before.shirt_name,
            "slot {slot}: shirt name"
        );
        assert_eq!(after.appearance, before.appearance, "slot {slot}: the look");
        assert_eq!(
            after.motion, before.motion,
            "slot {slot}: the motions, the free kick's with them"
        );
        assert_eq!(
            team.roster[slot].number, before_team.roster[slot].number,
            "slot {slot}: the shirt number"
        );
    }
}

// SE-GUI-67
#[test]
fn the_staged_import_follows_the_lock_turned_on_and_off_after_the_pick() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_lock_flip", THREE);
    assert_co_and_three_differ(&view);
    let co = export(&view, CO);
    picked(&mut window, &mut view, "co.toml", co.as_bytes());
    for label in ["Stats", "Aesthetics", "Gameplan"] {
        assert!(
            ticked(&mut window, &mut view, label),
            "premise: {label} chosen"
        );
    }

    view.aesthetics_from_exports = true;
    assert!(view.aesthetics_locked(), "premise: the lock is on");
    let unlocked = Sections {
        aesthetics: false,
        ..ALL
    };
    let expected = lib_import(&view, &kept(&co, unlocked), THREE);
    apply(&mut window, &mut view);
    assert_imported(&view, THREE, &expected);

    view.edit_aesthetics = true;
    assert!(!view.aesthetics_locked(), "premise: the lock is off");
    let expected = lib_import(&view, &kept(&co, ALL), THREE);
    assert_ne!(
        expected.1,
        rostered(&view, THREE),
        "premise: the aesthetics section changes /3/'s players"
    );
    apply(&mut window, &mut view);
    assert_imported(&view, THREE, &expected);
}

// SE-GUI-67
#[test]
fn the_staged_import_follows_an_edit_made_after_the_pick() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_live", THREE);
    assert_co_and_three_differ(&view);
    let source = rostered(&view, CO);
    let co = export(&view, CO);
    picked(&mut window, &mut view, "co.toml", co.as_bytes());
    toggle(&mut window, &mut view, "Aesthetics");
    assert!(
        !ticked(&mut window, &mut view, "Aesthetics"),
        "premise: aesthetics off"
    );

    let mut edited = rostered(&view, THREE)[0].clone();
    assert_ne!(edited.name, "EDITED", "premise: a new name");
    edited.name = "EDITED".to_owned();
    view.session
        .as_mut()
        .unwrap()
        .edit_players(vec![edited.clone()]);
    apply(&mut window, &mut view);

    let after = player_entry(&view, edited.id);
    assert_eq!(after.stats, source[0].stats, "the import applied");
    assert_eq!(after.name, "EDITED", "the edit survives");
}

/// Every section chosen.
const ALL: Sections = Sections {
    stats: true,
    aesthetics: true,
    gameplan: true,
};

// SE-GUI-67
#[test]
fn another_team_selected_after_the_pick_is_the_one_imported_into() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_retarget", THREE);
    let co = export(&view, CO);
    let expected = lib_import(&view, &kept(&co, ALL), CM);
    assert_ne!(
        expected.1,
        rostered(&view, CM),
        "premise: the file changes /cm/'s players"
    );
    let (three_team, three) = (team_entry(&view, THREE), rostered(&view, THREE));
    picked(&mut window, &mut view, "co.toml", co.as_bytes());
    assert!(
        panel_lines(&mut window, &mut view).contains(&"Import into /3/ (701) ".to_owned()),
        "premise: /3/ is the target at the pick"
    );

    view.team = TeamChoice::Team(CM);
    view.selected = Some(Selection::Team(CM));
    let lines = panel_lines(&mut window, &mut view);
    assert!(
        lines.contains(&"Import into /cm/ (713) ".to_owned()),
        "the new target named: {lines:#?}"
    );
    apply(&mut window, &mut view);

    assert_imported(&view, CM, &expected);
    assert_eq!(team_entry(&view, THREE), three_team, "/3/ untouched");
    assert_eq!(rostered(&view, THREE), three, "its players untouched");
}

// SE-GUI-67
#[test]
fn another_save_opened_after_the_pick_keeps_the_file_and_stages_against_it() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_reopen", THREE);
    let co = export(&view, CO);
    // A name the open save holds and the next one does not; the aesthetics
    // section, which would overwrite it, unticked.
    let mut edited = rostered(&view, THREE)[0].clone();
    assert_ne!(edited.name, "EDITED", "premise: a new name");
    edited.name = "EDITED".to_owned();
    view.session.as_mut().unwrap().edit_players(vec![edited]);
    picked(&mut window, &mut view, "co.toml", co.as_bytes());
    toggle(&mut window, &mut view, "Aesthetics");
    // A frame drawn after the toggle stages the choice against this save.
    assert!(
        !ticked(&mut window, &mut view, "Aesthetics"),
        "premise: aesthetics off"
    );

    view.open(&fixture(PesVersion::Pes19, "interchange_reopen_fresh"));
    view.team = TeamChoice::Team(THREE);
    view.selected = Some(Selection::Team(THREE));
    assert_eq!(view.panel, Some(Panel::Interchange), "the panel stays open");
    assert_ne!(
        rostered(&view, THREE)[0].name,
        "EDITED",
        "premise: the new save lacks the edit"
    );
    let unlocked = Sections {
        aesthetics: false,
        ..ALL
    };
    let expected = lib_import(&view, &kept(&co, unlocked), THREE);
    assert!(
        panel_lines(&mut window, &mut view).contains(&"co.toml".to_owned()),
        "the file stays picked"
    );
    apply(&mut window, &mut view);

    assert_imported(&view, THREE, &expected);
}

#[test]
fn a_file_that_does_not_read_is_refused_alone() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_unreadable", THREE);
    let text = "this is not a team = =";
    let error = TeamToml::parse(text)
        .expect_err("premise: the text does not parse")
        .to_string();
    picked(&mut window, &mut view, "bad.toml", text.as_bytes());

    let refusal = format!("bad.toml: {error}");
    let lines = panel_lines(&mut window, &mut view);
    assert_eq!(lines.last(), Some(&refusal), "{lines:#?}");
    assert!(
        !lines.iter().any(|line| line.starts_with("A Team TOML")),
        "no intro line: {lines:#?}"
    );
    assert!(checkboxes(&mut window, &mut view).is_empty(), "no choice");
    assert!(
        panel_node(&mut window, &mut view, Role::Button, "Apply").is_none(),
        "no Apply"
    );
    let error = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .error_fg_color;
    assert_eq!(
        painted(&mut window, &mut view, &refusal),
        error,
        "the refusal in the error colour"
    );
}

// SE-GUI-67
#[test]
fn a_second_pick_replaces_the_first_ones_import_and_applied_line() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_second_pick", THREE);
    let (co, cm) = (export(&view, CO), export(&view, CM));
    picked(&mut window, &mut view, "co.toml", co.as_bytes());
    apply(&mut window, &mut view);
    assert_eq!(
        panel_lines(&mut window, &mut view)
            .last()
            .map(String::as_str),
        Some("Applied co.toml."),
        "premise: the first file applied"
    );
    let expected = lib_import(&view, &kept(&cm, ALL), THREE);
    assert_ne!(
        expected.1,
        rostered(&view, THREE),
        "premise: the second file changes /3/'s players"
    );

    let path = beside_save(&view, "cm.toml", cm.as_bytes());
    pick(&mut window, &mut view, path);
    let lines = panel_lines(&mut window, &mut view);
    assert!(
        !lines.iter().any(|line| line.starts_with("Applied")),
        "the first file's Applied line goes: {lines:#?}"
    );
    apply(&mut window, &mut view);

    assert_imported(&view, THREE, &expected);
    assert_eq!(
        panel_lines(&mut window, &mut view)
            .last()
            .map(String::as_str),
        Some("Applied cm.toml.")
    );
}

#[test]
fn apply_is_disabled_when_no_section_the_file_holds_is_chosen() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_none_chosen", CO);
    picked(
        &mut window,
        &mut view,
        "pes19_tactics.4cct",
        &lib_fixture("pes19_tactics.4cct"),
    );
    assert_eq!(checkboxes(&mut window, &mut view), ["Gameplan"]);
    assert!(
        !apply_button(&mut window, &mut view).is_disabled(),
        "premise: Apply is enabled"
    );

    toggle(&mut window, &mut view, "Gameplan");
    assert!(
        apply_button(&mut window, &mut view).is_disabled(),
        "Apply is disabled"
    );
    let lines = panel_lines(&mut window, &mut view);
    assert!(
        !lines.contains(&ALREADY_HOLDS.to_owned()),
        "nothing chosen is not a file the team holds: {lines:#?}"
    );
}

#[test]
fn the_teams_own_file_changes_nothing_so_apply_is_disabled_and_says_so() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_own_file", THREE);
    let own = export(&view, THREE);
    picked(&mut window, &mut view, "701.toml", own.as_bytes());

    assert!(
        apply_button(&mut window, &mut view).is_disabled(),
        "Apply is disabled"
    );
    let lines = panel_lines(&mut window, &mut view);
    assert_eq!(
        lines.last().map(String::as_str),
        Some(ALREADY_HOLDS),
        "{lines:#?}"
    );
    assert!(!view.session.as_ref().unwrap().can_undo(), "no undo entry");
}

// A file the save's version stores whole carries no notes, so the panel
// draws no notes heading over an empty list.
#[test]
fn an_import_with_no_notes_draws_no_notes_heading() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_no_notes", THREE);
    let own = export(&view, THREE);
    picked(&mut window, &mut view, "701.toml", own.as_bytes());
    let lines = panel_lines(&mut window, &mut view);
    assert!(
        lines.iter().any(|line| line.starts_with("Sections")),
        "premise: the file is staged: {lines:#?}"
    );
    assert!(
        !lines.iter().any(|line| line.starts_with("Import notes")),
        "no notes heading: {lines:#?}"
    );
}

#[test]
fn the_applied_line_stands_after_an_apply_that_wrote_until_the_next_change() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_applied_once", THREE);
    assert_co_and_three_differ(&view);
    let co = export(&view, CO);
    picked(&mut window, &mut view, "co.toml", co.as_bytes());
    let lines = panel_lines(&mut window, &mut view);
    assert!(
        !lines.contains(&ALREADY_HOLDS.to_owned()),
        "premise: the file changes /3/: {lines:#?}"
    );
    apply(&mut window, &mut view);

    let lines = panel_lines(&mut window, &mut view);
    assert_eq!(
        lines.last().map(String::as_str),
        Some("Applied co.toml."),
        "{lines:#?}"
    );
    assert!(
        !lines.contains(&ALREADY_HOLDS.to_owned()),
        "the Applied line takes the other's place"
    );
    assert!(
        apply_button(&mut window, &mut view).is_disabled(),
        "a second Apply would change nothing"
    );

    // An edit of a player the file does not write.
    let mut other = rostered(&view, CM)[0].clone();
    assert_ne!(other.name, "EDITED", "premise: a new name");
    other.name = "EDITED".to_owned();
    view.session.as_mut().unwrap().edit_players(vec![other]);
    let lines = panel_lines(&mut window, &mut view);
    assert_eq!(
        lines.last().map(String::as_str),
        Some(ALREADY_HOLDS),
        "{lines:#?}"
    );
    assert!(
        apply_button(&mut window, &mut view).is_disabled(),
        "Apply stays disabled"
    );
}

// An elided picked file name shows the full path alone on hover: the
// label's own elided-text tooltip stays off, or the bare name would draw
// a second line over it.
#[test]
fn an_elided_picked_name_shows_the_path_alone_on_hover() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "interchange_long_name", CO);
    // Long enough to elide in the room the Pick button leaves, short
    // enough for a Windows path.
    let name = format!("co_{}.toml", "x".repeat(110));
    let text = export(&view, CO);
    let path = picked(&mut window, &mut view, &name, text.as_bytes());
    let nodes = panel_nodes(&mut window, &mut view);
    // egui names an elided label by its whole text.
    let label = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some(name.as_str()))
        .expect("the picked file's label");
    let bounds = label.bounds().expect("bounds");
    let font =
        window.ctx.style_of(window.ctx.theme()).text_styles[&egui::TextStyle::Monospace].clone();
    let unelided = window
        .ctx
        .fonts_mut(|fonts| fonts.layout_no_wrap(name.clone(), font, egui::Color32::WHITE));
    assert!(
        f64::from(unelided.size().x) > bounds.x1 - bounds.x0 + 2.0,
        "premise: the name is elided ({bounds:?} vs {})",
        unelided.size().x
    );

    let tip = path.display().to_string();
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(label), &tip),
        Some(tip),
        "the path on hover"
    );
    // The pointer still rests: the label's own elided-text tooltip would
    // add its `name` label — a second node of that value.
    let nodes = panel_nodes(&mut window, &mut view);
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node.value() == Some(name.as_str()))
            .count(),
        1,
        "no elided-text tooltip drawn"
    );
}

mod journal;
mod legacy;
mod notes;
mod panel;
