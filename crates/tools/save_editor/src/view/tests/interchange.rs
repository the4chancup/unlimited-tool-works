//! The team file panel's tests (SE-GUI-16, SE-GUI-17, SE-GUI-22): a Team
//! TOML imported with gameplay alone as one undoable edit, another
//! version's notes in words, a refusal drawn under the
//! choices, a squad file's pairing and its refusal, a tactics file, a
//! Texport, the aesthetics lock, the import following the save, and the
//! Team menu's entry that opens the panel.

use pes_savefile::interchange::legacy::{read_squad, read_tactics};
use pes_savefile::interchange::team_toml::{TacticsSection, TeamToml};
use pes_savefile::model::player::PlayerMotion;

use crate::session::import_sections::{Sections, keep_sections};
use crate::test_support::cli_stdout;
use crate::view::appearance::OWNERSHIP_TIP;

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
fn a_team_toml_with_gameplay_alone_writes_the_abilities_as_one_undoable_edit() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_gameplay", THREE);
    assert_co_and_three_differ(&view);
    let source = rostered(&view, CO);
    let before_team = team_entry(&view, THREE);
    let before_players = rostered(&view, THREE);
    let co = export(&view, CO);
    let gameplay = Sections {
        gameplay: true,
        aesthetics: false,
        tactics: false,
    };
    let expected = lib_import(&view, &kept(&co, gameplay), THREE);
    picked(&mut window, &mut view, "co.toml", co.as_bytes());
    assert!(
        panel_lines(&mut window, &mut view)
            .contains(&"A Team TOML written from a PES 2019 save.".to_owned()),
        "the intro names the file's version, the save's"
    );
    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["Gameplay", "Aesthetics", "Tactics"]
    );
    toggle(&mut window, &mut view, "Aesthetics");
    toggle(&mut window, &mut view, "Tactics");
    assert!(
        ticked(&mut window, &mut view, "Gameplay"),
        "gameplay chosen"
    );
    assert!(
        !ticked(&mut window, &mut view, "Aesthetics"),
        "aesthetics off"
    );
    assert!(!ticked(&mut window, &mut view, "Tactics"), "tactics off");

    apply(&mut window, &mut view);

    assert_imported(&view, THREE, &expected);
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
    toggle(&mut window, &mut view, "Gameplay");
    toggle(&mut window, &mut view, "Tactics");

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

// SE-GUI-16
#[test]
fn a_file_of_another_version_lists_its_notes_in_words() {
    let (mut view, mut window) = open_on(PesVersion::Pes17, "interchange_notes", CO);
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
    let expected: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("- Warning import_"))
        .collect();
    assert!(
        expected
            .iter()
            .any(|line| line.contains("(path=team.manager_id)")),
        "premise: the command line notes the manager id: {expected:#?}"
    );

    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["Gameplay", "Aesthetics", "Tactics"]
    );
    let lines = panel_lines(&mut window, &mut view);
    for line in [
        "A Team TOML written from a PES 2021 save, converted to PES 2017 where it can be.",
        "Import notes (PES 2021 file onto a PES 2017 save)",
    ] {
        assert!(lines.contains(&line.to_owned()), "{line:?} in {lines:#?}");
    }

    // The catalog's text once per code the command line printed, none for
    // another code.
    let catalog = [
        (
            "import_key_not_in_version",
            "The team file holds a key the file's version stores but the save's does not. \
             The key is dropped from the import, but the rest is written.",
        ),
        (
            "import_value_not_encodable",
            "The team file holds a value the save's version cannot store: an instruction or a \
             playing style it lacks. The value is written as off (a playing style as None), and \
             the rest of the import is written.",
        ),
        (
            "import_value_capped",
            "The team file holds a value above what the save's version stores: a face type \
             above its cap, a motion above its last, or a custom skin it has no slot for. The \
             value is lowered (the face type to 0, the motion to its last, the skin to 1), and \
             the rest of the import is written.",
        ),
        (
            "import_slot_empty",
            "The team file holds a player for a roster place the team leaves empty. The player \
             is skipped, but the rest is written.",
        ),
    ];
    for (code, text) in catalog {
        let printed = expected
            .iter()
            .any(|line| line.starts_with(&format!("- Warning {code} ")));
        let drawn = lines.iter().filter(|line| *line == text).count();
        assert_eq!(drawn, usize::from(printed), "{code}'s text in {lines:#?}");
    }

    // A team key's group is its subject alone, in the warning colour.
    assert!(
        lines.contains(&"Manager ID".to_owned()),
        "Manager ID in {lines:#?}"
    );
    let warn = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .warn_fg_color;
    assert_eq!(
        painted(&mut window, &mut view, "Manager ID"),
        warn,
        "a team key in the warning colour"
    );

    // One collapsed header per players' group, `<subject> · <n> players`,
    // its count the command line's notes of that key.
    let mut groups: Vec<(String, usize)> = Vec::new();
    for line in expected.iter().filter(|line| !line.contains("(path=team.")) {
        let key = note_group(line);
        if key.contains(" tactics.") {
            continue;
        }
        match groups.iter_mut().find(|(group, _)| *group == key) {
            Some((_, count)) => *count += 1,
            None => groups.push((key, 1)),
        }
    }
    let (stat, players) = groups
        .iter()
        .find_map(|(key, count)| {
            let stat = key.strip_prefix("import_key_not_in_version players.*.stats.")?;
            Some((stat.to_owned(), *count))
        })
        .expect("premise: a stat the save's version does not store");
    assert!(players > 1, "premise: several players' {stat}");
    let headers = note_headers(&mut window, &mut view);
    let stat_header = format!(
        "{} \u{b7} {players} players",
        pes_savefile::interchange::team_toml::labels::stat_name(PesVersion::Pes17, &stat)
    );
    assert!(
        headers.contains(&stat_header),
        "{stat_header:?} in {headers:#?}"
    );
    let mut shown_counts: Vec<usize> = headers
        .iter()
        .map(|header| header_players(header))
        .collect();
    let mut counts: Vec<usize> = groups.iter().map(|(_, count)| *count).collect();
    shown_counts.sort_unstable();
    counts.sort_unstable();
    assert_eq!(shown_counts, counts, "one header per players' group");

    // Opened, no line speaks the command line's words.
    open_note_groups(&mut window, &mut view);
    let lines = panel_lines(&mut window, &mut view);
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("import_") || line.contains("path=")),
        "no code or key path drawn: {lines:#?}"
    );
}

// SE-GUI-16
#[test]
fn every_empty_place_a_file_names_is_one_group() {
    // PES 21's team 701 fills places 01 to 23 of its 32.
    let (mut view, mut window) = open_on(PesVersion::Pes21, "interchange_empty_places", 701);
    let team = team_entry(&view, 701);
    assert_eq!(
        team.roster.iter().position(|slot| slot.player_id == 0),
        Some(23),
        "premise: places 01 to 23 filled"
    );
    assert_eq!(team.roster[24].player_id, 0, "premise: place 25 empty");
    let text = "[players.24]\nname = \"GHOST\"\n\n[players.25]\nname = \"GHOST\"\n";
    picked(&mut window, &mut view, "ghosts.toml", text.as_bytes());

    assert_eq!(
        note_headers(&mut window, &mut view),
        ["Empty roster place \u{b7} 2 players"]
    );
}

/// The players' groups' headers, top to bottom.
fn note_headers(window: &mut Headless, view: &mut SaveView) -> Vec<String> {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let mut headers: Vec<(f64, String)> = nodes_in_window(&pairs, TITLE)
        .into_iter()
        .filter(|(_, node)| node.role() == Role::DisclosureTriangle)
        .map(|(_, node)| {
            (
                node.bounds().expect("drawn bounds").y0,
                node.label().expect("a labeled header").to_owned(),
            )
        })
        .collect();
    headers.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("drawn bounds"));
    headers.into_iter().map(|(_, header)| header).collect()
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

/// The group of the command line's note `line` (`- Warning <code>
/// (<key>=<value>, …)`): its code and its path, a player's `players.NN.`
/// written `players.*.`; every empty slot's note is its code alone.
fn note_group(line: &str) -> String {
    let (code, context) = line
        .strip_prefix("- Warning ")
        .and_then(|rest| rest.split_once(" ("))
        .unwrap_or_else(|| panic!("a note line: {line:?}"));
    let (key, at) = context.split_once('=').expect("a context key");
    if key == "slot" {
        return code.to_owned();
    }
    let at = at.split([',', ')']).next().expect("a context value");
    let at = match at
        .strip_prefix("players.")
        .and_then(|rest| rest.split_once('.'))
    {
        Some((_, tail)) => format!("players.*.{tail}"),
        None => at.to_owned(),
    };
    format!("{code} {at}")
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
        at("Tactics").is_some_and(|choice| choice < refused),
        "under the choices"
    );
    assert!(
        at("Apply").is_some_and(|apply| refused < apply),
        "above Apply"
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
    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["Gameplay", "Aesthetics", "Tactics"]
    );
    assert!(
        apply_button(&mut window, &mut view).is_disabled(),
        "Apply is disabled"
    );

    toggle(&mut window, &mut view, "Gameplay");
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
    let (expected_team, expected) = lib_import(&view, &document, CO);
    let before = rostered(&view, CO);
    assert_ne!(
        expected, before,
        "premise: the squad changes /co/'s players"
    );
    assert!(
        expected_team
            .roster
            .iter()
            .zip(&team_entry(&view, CO).roster)
            .any(|(after, before)| after.number != before.number),
        "premise: the squad changes a shirt number"
    );
    let numbers: Vec<String> = team_entry(&view, CO)
        .roster
        .iter()
        .map(|slot| slot.number.to_string())
        .collect();
    // Every record against the player at its slot.
    let team = team_entry(&view, CO);
    let rows: Vec<[String; 5]> = document
        .players
        .iter()
        .map(|(&slot, section)| {
            let entry = team.roster[usize::from(slot - 1)];
            [
                format!("{slot:02}"),
                display_name(section.name.as_deref().expect("a record's name")),
                "\u{2192}".to_owned(),
                entry.number.to_string(),
                display_name(&player_entry(&view, entry.player_id).name),
            ]
        })
        .collect();
    assert_eq!(rows.len(), 23, "premise: 23 records");
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
    for row in &rows {
        assert_eq!(
            row_of(&mut window, &mut view, &row[1]),
            row,
            "slot {}",
            row[0]
        );
    }
    let arrows = panel_lines(&mut window, &mut view)
        .iter()
        .filter(|line| *line == "\u{2192}")
        .count();
    assert_eq!(arrows, 23, "one row per record");
    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["Gameplay", "Aesthetics"]
    );

    apply(&mut window, &mut view);
    assert_eq!(rostered(&view, CO), expected, "the lib's import");
    assert_eq!(
        team_entry(&view, CO),
        expected_team,
        "the lib's team, its shirt numbers included"
    );
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
    assert_eq!(checkboxes(&mut window, &mut view), ["Tactics"]);
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
        ["Gameplay", "Aesthetics", "Tactics"]
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
    let unlocked = Sections {
        gameplay: true,
        aesthetics: false,
        tactics: true,
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
    assert!(
        ticked(&mut window, &mut view, "Gameplay"),
        "gameplay chosen"
    );
    assert!(ticked(&mut window, &mut view, "Tactics"), "tactics chosen");

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

// SE-GUI-22
#[test]
fn the_staged_import_follows_the_lock_turned_on_and_off_after_the_pick() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_lock_flip", THREE);
    assert_co_and_three_differ(&view);
    let co = export(&view, CO);
    picked(&mut window, &mut view, "co.toml", co.as_bytes());
    for label in ["Gameplay", "Aesthetics", "Tactics"] {
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

// SE-GUI-16
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
    gameplay: true,
    aesthetics: true,
    tactics: true,
};

// SE-GUI-16
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

// SE-GUI-16
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

// SE-GUI-16
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
        Some("Applied co.toml. The changes stay unsaved until Save."),
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
        Some("Applied cm.toml. The changes stay unsaved until Save.")
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
    assert_eq!(checkboxes(&mut window, &mut view), ["Tactics"]);
    assert!(
        !apply_button(&mut window, &mut view).is_disabled(),
        "premise: Apply is enabled"
    );

    toggle(&mut window, &mut view, "Tactics");
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
        Some("Applied co.toml. The changes stay unsaved until Save."),
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

#[test]
fn apply_stays_inside_the_window_with_every_note_group_open() {
    let (mut view, _) = open_on(PesVersion::Pes19, "interchange_notes_scroll", CO);
    // The default pane, where the real window's height bounds the panel.
    let mut window = Headless::new();
    let mut source = SaveView::new();
    source.open(&fixture(
        PesVersion::Pes21,
        "interchange_notes_scroll_source",
    ));
    picked(
        &mut window,
        &mut view,
        "co21.toml",
        export(&source, CO).as_bytes(),
    );
    open_note_groups(&mut window, &mut view);

    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let nodes = nodes_in_window(&pairs, TITLE);
    let notes: usize = nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::DisclosureTriangle)
        .map(|(_, node)| header_players(node.label().expect("a labeled header")))
        .sum();
    assert!(notes > 100, "premise: a long list of notes, {notes}");
    let bounds = |role: Role, label: &str| {
        nodes
            .iter()
            .find(|(_, node)| node.role() == role && node.label() == Some(label))
            .and_then(|(_, node)| node.bounds())
            .unwrap_or_else(|| panic!("the {label:?} {role:?}"))
    };
    let (panel, apply) = (bounds(Role::Window, TITLE), bounds(Role::Button, "Apply"));
    assert!(
        apply.y0 >= panel.y0 && apply.y1 <= panel.y1,
        "Apply {apply:?} inside the panel {panel:?}"
    );
    assert!(
        apply.y1 <= f64::from(DEFAULT_PANE.y),
        "Apply {apply:?} on the screen"
    );
}

#[test]
fn the_close_button_closes_the_panel() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_close", CO);
    open_panel(&mut window, &mut view);
    assert_eq!(view.panel, Some(Panel::Interchange), "premise: open");
    let close =
        panel_node(&mut window, &mut view, Role::Button, "Close window").expect("the close button");
    click(&mut window, &close, &mut view);
    assert_eq!(view.panel, None, "the panel closes");
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
        ["Pick a team row to export or import."]
    );
}

#[test]
fn a_player_row_selected_makes_his_team_the_target() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_player_row", CO);
    view.selected = Some(Selection::Player(CO, 0));
    view.panel = Some(Panel::Interchange);
    // A window's first frame only measures it.
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let lines = panel_lines(&mut window, &mut view);
    assert!(
        lines.contains(&"Import into /co/ (714) ".to_owned()),
        "{lines:#?}"
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
