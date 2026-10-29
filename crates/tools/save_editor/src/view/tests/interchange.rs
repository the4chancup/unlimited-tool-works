//! The team file panel's tests (SE-GUI-16, SE-GUI-17, SE-GUI-22): a Team
//! TOML imported with gameplay alone as one undoable edit, another
//! version's notes in the command line's words, a refusal drawn under the
//! choices, a squad file's pairing and its refusal, a tactics file, a
//! Texport, the aesthetics lock, the import following the save, and the
//! Team menu's entry that opens the panel.

use pes_savefile::interchange::legacy::{read_squad, read_tactics};
use pes_savefile::interchange::team_toml::{TacticsSection, TeamToml};
use pes_savefile::model::player::PlayerMotion;

use crate::test_support::cli_stdout;
use crate::view::appearance::ownership_tip;

use super::*;

/// The panel's title, its window's label.
const TITLE: &str = "Team file interchange";

/// Teams of the PES 19 fixture, a cup save whose teams hold real,
/// distinct players: `/3/`, `/cm/` and `/co/`.
const THREE: u32 = 701;
const CM: u32 = 713;
const CO: u32 = 714;

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
    view.interchange_dialog = Box::new(move || Some(path.clone()));
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
fn a_team_toml_with_gameplay_alone_writes_the_abilities_as_one_undoable_edit() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_gameplay", THREE);
    assert_co_and_three_differ(&view);
    let source = rostered(&view, CO);
    let before_team = team_entry(&view, THREE);
    let before_players = rostered(&view, THREE);
    let co = export(&view, CO);
    picked(&mut window, &mut view, "co.toml", co.as_bytes());
    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["gameplay", "aesthetics", "tactics"]
    );
    toggle(&mut window, &mut view, "aesthetics");
    toggle(&mut window, &mut view, "tactics");
    assert!(
        ticked(&mut window, &mut view, "gameplay"),
        "gameplay chosen"
    );
    assert!(
        !ticked(&mut window, &mut view, "aesthetics"),
        "aesthetics off"
    );
    assert!(!ticked(&mut window, &mut view, "tactics"), "tactics off");

    apply(&mut window, &mut view);

    let team = team_entry(&view, THREE);
    assert_eq!(team.tactics, before_team.tactics, "/3/'s tactics unchanged");
    // The file's `[team]` keys load with gameplay.
    assert_eq!(
        (before_team.name.as_str(), team.name.as_str()),
        ("/3/", "/co/"),
        "the team's name is the file's"
    );
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
            team.roster[slot].number, before_team.roster[slot].number,
            "slot {slot}: the shirt number"
        );
    }
    assert_eq!(
        panel_lines(&mut window, &mut view)
            .last()
            .map(String::as_str),
        Some("Applied co.toml. The changes stay unsaved until Save.")
    );

    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo entry");
    assert_eq!(team_entry(&view, THREE), before_team, "/3/ as before");
    assert_eq!(
        rostered(&view, THREE),
        before_players,
        "its players as before"
    );
}

// SE-GUI-16
#[test]
fn a_file_of_another_version_lists_the_command_lines_notes() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_notes", CO);
    let mut source = SaveView::new();
    source.open(&fixture(PesVersion::Pes21, "interchange_notes_source"));
    let path = picked(
        &mut window,
        &mut view,
        "co21.toml",
        export(&source, CO).as_bytes(),
    );

    // The command line's import of the same document onto the same save.
    let save = view.session.as_ref().unwrap().path().to_owned();
    let out = save.with_file_name("OUT");
    let stdout = cli_stdout(
        save.parent().unwrap(),
        &[
            "import-toml",
            path.to_str().unwrap(),
            "--base",
            save.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
    );
    let mut expected: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("- Warning import_"))
        .collect();
    assert!(
        !expected.is_empty(),
        "premise: the command line notes some keys"
    );

    let lines = panel_lines(&mut window, &mut view);
    for line in [
        "A Team TOML written from a PES 2021 save, converted to PES 2019 where it can be.",
        "Import notes (PES 2021 file onto a PES 2019 save)",
    ] {
        assert!(lines.contains(&line.to_owned()), "{line:?} in {lines:#?}");
    }
    open_note_groups(&mut window, &mut view);
    let lines = panel_lines(&mut window, &mut view);
    let mut shown: Vec<&str> = lines
        .iter()
        .map(String::as_str)
        .filter(|line| line.starts_with("- "))
        .collect();
    shown.sort_unstable();
    expected.sort_unstable();
    assert_eq!(shown, expected, "every note, as the command line prints it");
}

// SE-GUI-16
#[test]
fn a_refusal_of_a_chosen_section_stands_under_the_choices_until_it_is_unticked() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_refusal", THREE);
    let text = export(&view, CO);
    let name_line = "name = \"/co/\"";
    assert_eq!(text.matches(name_line).count(), 1, "premise: [team] name");
    let edited = text.replacen(name_line, &format!("name = \"{}\"", "X".repeat(200)), 1);
    let document = TeamToml::parse(&edited).expect("the edited text parses");
    let refusal = document
        .apply(
            PesVersion::Pes19,
            &mut team_entry(&view, THREE),
            &mut rostered(&view, THREE),
        )
        .expect_err("premise: the lib refuses the long team name")
        .to_string();
    picked(&mut window, &mut view, "long.toml", edited.as_bytes());

    let lines = panel_lines(&mut window, &mut view);
    let at = |text: &str| lines.iter().position(|line| line == text);
    let refused = at(&refusal).unwrap_or_else(|| panic!("{refusal:?} in {lines:#?}"));
    assert!(
        at("tactics").is_some_and(|choice| choice < refused),
        "under the choices"
    );
    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["gameplay", "aesthetics", "tactics"]
    );
    assert!(
        apply_button(&mut window, &mut view).is_disabled(),
        "Apply is disabled"
    );

    toggle(&mut window, &mut view, "gameplay");
    assert!(
        !panel_lines(&mut window, &mut view).contains(&refusal),
        "the refusal goes with gameplay"
    );
    assert!(
        !apply_button(&mut window, &mut view).is_disabled(),
        "Apply is enabled"
    );
}

// SE-GUI-17
#[test]
fn a_squad_file_pairs_its_records_and_writes_gameplay_and_aesthetics() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_squad", CO);
    let bytes = lib_fixture("pes19_squad.4ccs");
    let file = view.session.as_ref().unwrap().file();
    let document = read_squad(&bytes, file.team(CO).unwrap(), file.players()).unwrap();
    assert_eq!(
        document.tactics,
        TacticsSection::default(),
        "premise: the fixture has no tactics block"
    );
    let (_, expected) = lib_import(&view, &document, CO);
    let before = rostered(&view, CO);
    assert_ne!(
        expected, before,
        "premise: the squad changes /co/'s players"
    );
    let numbers: Vec<String> = team_entry(&view, CO)
        .roster
        .iter()
        .map(|slot| slot.number.to_string())
        .collect();
    picked(&mut window, &mut view, "pes19_squad.4ccs", &bytes);

    let lines = panel_lines(&mut window, &mut view);
    for line in [
        "A legacy squad file (.4ccs) written from a PES 2019 save.",
        "Who gets which record",
    ] {
        assert!(lines.contains(&line.to_owned()), "{line:?} in {lines:#?}");
    }
    for row in [
        [
            "01",
            "SHOTABOT 9S",
            "\u{2192}",
            &numbers[0],
            "MR. /CO/ - SKELETOR",
        ],
        [
            "03",
            ">fujo's first husbando chart",
            "\u{2192}",
            &numbers[2],
            "BLUEY",
        ],
    ] {
        assert_eq!(row_of(&mut window, &mut view, row[1]), row);
    }
    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["gameplay", "aesthetics"]
    );

    apply(&mut window, &mut view);
    assert_eq!(rostered(&view, CO), expected, "the lib's import");
}

// SE-GUI-17
#[test]
fn a_squad_file_with_more_records_than_the_roster_is_refused_alone() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_squad_short", CO);
    let mut team = team_entry(&view, CO);
    team.roster[22].player_id = 0;
    view.session
        .as_mut()
        .unwrap()
        .edit_entries(Vec::new(), vec![team]);
    assert_eq!(
        rostered(&view, CO).len(),
        22,
        "premise: 22 rostered players"
    );
    picked(
        &mut window,
        &mut view,
        "pes19_squad.4ccs",
        &lib_fixture("pes19_squad.4ccs"),
    );

    let lines = panel_lines(&mut window, &mut view);
    assert_eq!(
        lines.last().map(String::as_str),
        Some("pes19_squad.4ccs: 23 player records; the target team has 22 rostered players"),
        "{lines:#?}"
    );
    assert!(checkboxes(&mut window, &mut view).is_empty(), "no choice");
    assert!(
        panel_node(&mut window, &mut view, Role::Button, "Apply").is_none(),
        "no Apply"
    );
}

// SE-GUI-17
#[test]
fn a_tactics_file_offers_tactics_alone_and_writes_them() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_tactics", CO);
    let bytes = lib_fixture("pes19_tactics.4cct");
    let document = read_tactics(&bytes).unwrap();
    assert_eq!(document.team.id, Some(CM), "premise: /cm/'s tactics");
    let (expected, _) = lib_import(&view, &document, CO);
    let before = team_entry(&view, CO);
    assert_ne!(
        expected.tactics, before.tactics,
        "premise: the tactics differ"
    );
    let players = rostered(&view, CO);
    picked(&mut window, &mut view, "pes19_tactics.4cct", &bytes);

    assert!(
        panel_lines(&mut window, &mut view)
            .contains(&"A legacy tactics file (.4cct) written from a PES 2019 save.".to_owned())
    );
    assert_eq!(checkboxes(&mut window, &mut view), ["tactics"]);
    apply(&mut window, &mut view);
    assert_eq!(
        team_entry(&view, CO).tactics,
        expected.tactics,
        "the file's tactics"
    );
    assert_eq!(rostered(&view, CO), players, "the players untouched");
}

// SE-GUI-17
#[test]
fn a_texport_offers_every_section_and_names_its_version() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_texport", CO);
    picked(
        &mut window,
        &mut view,
        "pes19_texport.ted",
        &lib_fixture("pes19_texport.ted"),
    );

    assert!(
        panel_lines(&mut window, &mut view)
            .contains(&"A Texport written from a PES 2019 save.".to_owned())
    );
    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["gameplay", "aesthetics", "tactics"]
    );
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
    picked(&mut window, &mut view, "co.toml", co.as_bytes());

    let aesthetics = checkbox(&mut window, &mut view, "aesthetics");
    assert!(
        aesthetics.is_disabled(),
        "the aesthetics choice is disabled"
    );
    assert!(!ticked(&mut window, &mut view, "aesthetics"), "and off");
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(&aesthetics), "export owns"),
        Some(ownership_tip(PesVersion::Pes19).to_owned())
    );
    assert!(
        ticked(&mut window, &mut view, "gameplay"),
        "gameplay chosen"
    );
    assert!(ticked(&mut window, &mut view, "tactics"), "tactics chosen");

    apply(&mut window, &mut view);
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
            after.motion,
            PlayerMotion {
                free_kick: after.motion.free_kick,
                ..before.motion.clone()
            },
            "slot {slot}: the aesthetics motions"
        );
        assert_eq!(
            team.roster[slot].number, before_team.roster[slot].number,
            "slot {slot}: the shirt number"
        );
    }
}

// SE-GUI-16
#[test]
fn the_staged_import_follows_an_edit_made_after_the_pick() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_live", THREE);
    assert_co_and_three_differ(&view);
    let source = rostered(&view, CO);
    let co = export(&view, CO);
    picked(&mut window, &mut view, "co.toml", co.as_bytes());
    toggle(&mut window, &mut view, "aesthetics");
    assert!(
        !ticked(&mut window, &mut view, "aesthetics"),
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

#[test]
fn the_team_menu_ends_with_the_entry_that_opens_the_panel() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_menu", CO);
    open_menu(&mut window, &mut view, "Team", "Import / export team file");
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // The popup's buttons: the descendants of the node holding its first
    // entry, top to bottom.
    let (first, _) = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("Set edited flag"))
        .expect("the Team menu's first entry");
    let (popup, _) = pairs
        .iter()
        .find(|(_, node)| node.children().contains(first))
        .expect("the entry's container");
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        pairs.iter().map(|(id, node)| (*id, node)).collect();
    let mut buttons: Vec<(f64, String)> = Vec::new();
    let mut stack = vec![*popup];
    while let Some(id) = stack.pop() {
        let node = by_id[&id];
        stack.extend(node.children());
        if node.role() == Role::Button
            && let (Some(bounds), Some(label)) = (node.bounds(), node.label())
        {
            buttons.push((bounds.y0, label.to_owned()));
        }
    }
    buttons.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("drawn bounds"));
    let labels: Vec<&str> = buttons.iter().map(|(_, label)| label.as_str()).collect();
    // Prefixes: an entry's label carries its shortcut after the name.
    let entries = [
        "Set edited flag",
        "Set boots and gloves IDs for everyone\u{2026}",
        "Set player names to positions",
        "Clear visual flags",
        "Set FPC invisibility",
        "Remove FPC invisibility",
        "Import / export team file\u{2026}",
    ];
    assert!(
        labels.len() == entries.len()
            && labels
                .iter()
                .zip(entries)
                .all(|(label, entry)| label.starts_with(entry)),
        "the entries in order, the new one last: {labels:?}"
    );

    // The separator: a horizontal line across the popup between the FPC
    // entry and the new one.
    let nodes: Vec<accesskit::Node> = pairs.into_iter().map(|(_, node)| node).collect();
    let fpc = menu_item(&nodes, "Remove FPC invisibility")
        .bounds()
        .expect("drawn");
    let last = menu_item(&nodes, "Import / export team file")
        .bounds()
        .expect("drawn");
    let separated = shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::LineSegment { points, .. } => {
            let (y, left, right) = (
                f64::from(points[0].y),
                f64::from(points[0].x.min(points[1].x)),
                f64::from(points[0].x.max(points[1].x)),
            );
            points[0].y == points[1].y
                && y > fpc.y1
                && y < last.y0
                && left < last.x1
                && right > last.x0
        }
        _ => false,
    });
    assert!(separated, "a separator before the new entry");

    click(
        &mut window,
        menu_item(&nodes, "Import / export team file"),
        &mut view,
    );
    assert_eq!(view.panel, Some(Panel::Interchange), "the panel opens");
    assert!(
        panel_lines(&mut window, &mut view).contains(
            &"Team TOML, .4ccs, .4cct and Texport files open here, and the file decides the format."
                .to_owned()
        ),
        "the sentence before a pick"
    );
}

#[test]
fn the_panel_without_a_team_row_asks_for_one() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_no_team", CO);
    view.selected = None;
    view.panel = Some(Panel::Interchange);
    // A window's first frame only measures it.
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        panel_lines(&mut window, &mut view),
        ["Pick a team row to import into."]
    );
}

#[test]
fn one_floating_panel_at_a_time() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_one_panel", CO);
    let savefile = |window: &mut Headless, view: &mut SaveView, entry: &str| {
        let nodes = open_menu(window, view, "Savefile", entry);
        click(window, menu_item(&nodes, entry), view);
    };
    savefile(&mut window, &mut view, "Comparator\u{2026}");
    assert_eq!(
        view.panel,
        Some(Panel::Comparator),
        "premise: the comparator"
    );
    open_panel(&mut window, &mut view);
    assert_eq!(
        view.panel,
        Some(Panel::Interchange),
        "the comparator closes"
    );
    savefile(&mut window, &mut view, "Apply aesthetics patch\u{2026}");
    assert_eq!(view.panel, Some(Panel::Patch), "the team file panel closes");
    open_panel(&mut window, &mut view);
    assert_eq!(
        view.panel,
        Some(Panel::Interchange),
        "the patch panel closes"
    );
    savefile(&mut window, &mut view, "Comparator\u{2026}");
    assert_eq!(
        view.panel,
        Some(Panel::Comparator),
        "the team file panel closes"
    );
}
