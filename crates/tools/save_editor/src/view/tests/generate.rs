//! The Generate dialogs' tests (SE-GUI-19, SE-GUI-33): the two Player-menu
//! items end to end — the explicit name with its colour
//! codes, the two files split as the format has them, an existing
//! settings.toml's comments kept, a player at -1 generated, Write's folder
//! gate and an existing file that does not parse.

use pes_savefile::model::names::display_name;
use pes_savefile::schema::ingame_face::IngameFaceField;
use pes_savefile::settings_toml::{IngameFaceSettings, NameSetting, PlayerSettings};

use super::*;

/// `version`'s fixture open with `(team_id, slot)` selected.
fn open_on(version: PesVersion, test: &str, team_id: u32, slot: u8) -> (SaveView, Headless) {
    let path = fixture(version, test);
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(team_id);
    view.selected = Some(Selection::Player(team_id, slot));
    (view, Headless::new())
}

/// The selected player's id.
fn selected_id(view: &SaveView, team_id: u32, slot: u8) -> u32 {
    view.session
        .as_ref()
        .expect("an open save")
        .slot_player(team_id, slot)
        .expect("the slot is filled")
}

/// `item`'s Player-menu item clicked.
fn player_menu_item(window: &mut Headless, view: &mut SaveView, item: &str) {
    let nodes = open_menu(window, view, "Player", item);
    click(window, menu_item(&nodes, item), view);
}

/// `dir` returned by the folder pick stub: Pick clicked, then Write.
fn pick_and_write(window: &mut Headless, view: &mut SaveView, dir: PathBuf) {
    view.pick_folder_dialog = Box::new(move || Some(dir.clone()));
    let nodes = draw(window, view);
    click(
        window,
        node_labeled(&nodes, Role::Button, "Pick\u{2026}"),
        view,
    );
    let nodes = draw(window, view);
    click(window, node_labeled(&nodes, Role::Button, "Write"), view);
}

// SE-GUI-19
#[test]
fn generated_settings_writes_the_explicit_colour_coded_name() {
    let path = fixture(PesVersion::Pes21, "generate_coded_name");
    let mut view = SaveView::new();
    view.open(&path);
    // The first rostered player whose savefile name holds a colour code:
    // the editor writes the save's strings, so `name` is always the
    // explicit string, never `name = true` (the folder-name rule).
    let (team_id, slot, raw) = {
        let file = view.session.as_ref().unwrap().file();
        file.teams()
            .iter()
            .flat_map(|team| {
                team.roster
                    .iter()
                    .enumerate()
                    .map(move |(slot, roster_slot)| (team.id, slot, roster_slot.player_id))
            })
            .find_map(|(team_id, slot, id)| {
                let name = &file.player(id)?.name;
                name.contains('\x11')
                    .then(|| (team_id, slot as u8, name.clone()))
            })
            .expect("the fixture holds a colour-coded rostered player")
    };
    view.team = TeamChoice::Team(team_id);
    view.selected = Some(Selection::Player(team_id, slot));
    let mut window = Headless::new();
    let dir = crate::test_support::scratch("generate_coded_name");
    player_menu_item(&mut window, &mut view, "Generate settings.toml");
    pick_and_write(&mut window, &mut view, dir.clone());
    let text = std::fs::read_to_string(dir.join("settings.toml")).unwrap();
    assert!(
        !text.contains("name = true"),
        "the name is never name = true"
    );
    let parsed = PlayerSettings::parse(&text).expect("the file parses");
    assert_eq!(
        parsed.name,
        Some(NameSetting::Explicit(raw.clone())),
        "the explicit string with its codes"
    );
    assert_eq!(
        view.notice.as_ref().map(|notice| notice.text.as_str()),
        Some(
            format!(
                "Wrote settings.toml for {} into {}",
                display_name(&raw),
                dir.display()
            )
            .as_str()
        ),
        "the status-bar notice"
    );
}

// SE-GUI-33
#[test]
fn the_two_generate_actions_split_the_files_as_the_format_has_them() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "generate_face_split", 701, 1);
    // The selected player's record edited to hair length 5 and iris 7
    // through the session.
    let player_id = selected_id(&view, 701, 1);
    let mut entry = player_entry(&view, player_id);
    entry
        .appearance
        .ingame_face
        .set(IngameFaceField::HairLength, 5)
        .unwrap();
    entry
        .appearance
        .ingame_face
        .set(IngameFaceField::IrisColor, 7)
        .unwrap();
    let expected_face = PlayerSettings::from_player(&entry).unwrap().ingame_face();
    view.session.as_mut().unwrap().edit_players(vec![entry]);
    let dir = crate::test_support::scratch("generate_face_split");
    player_menu_item(&mut window, &mut view, "Generate settings.toml");
    pick_and_write(&mut window, &mut view, dir.clone());
    player_menu_item(&mut window, &mut view, "Generate in-game face");
    pick_and_write(&mut window, &mut view, dir.clone());

    let settings_text = std::fs::read_to_string(dir.join("settings.toml")).unwrap();
    PlayerSettings::parse(&settings_text).expect("settings.toml parses");
    assert!(!settings_text.contains("iris_color"), "no iris key");
    assert!(
        !settings_text.contains("[appearance.face]"),
        "no face table"
    );
    assert!(
        !settings_text.contains("[appearance.hair]"),
        "no hair table"
    );

    let face_text = std::fs::read_to_string(dir.join("ingame_face.toml")).unwrap();
    let face = IngameFaceSettings::parse(&face_text).expect("ingame_face.toml parses");
    assert_eq!(face, expected_face, "the record's face, hair and iris");
    assert!(face_text.contains("iris_color = 7"));
    assert!(
        face_text.lines().any(|line| line.starts_with("length = 5")),
        "[hair] length = 5 in {face_text}"
    );
}

// An existing settings.toml keeps its comments and every key it carries
// (`update_toml` rewrites the values in place, the record's, never the
// text around them).
#[test]
fn an_existing_settings_toml_keeps_its_comments() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "generate_keep_comments", 701, 1);
    let player_id = selected_id(&view, 701, 1);
    let dir = crate::test_support::scratch("generate_keep_comments");
    std::fs::write(dir.join("settings.toml"), "# mine\nname = \"STALE\"\n").unwrap();
    player_menu_item(&mut window, &mut view, "Generate settings.toml");
    pick_and_write(&mut window, &mut view, dir.clone());
    let text = std::fs::read_to_string(dir.join("settings.toml")).unwrap();
    assert!(text.contains("# mine"), "the file's own comment stays");
    let parsed = PlayerSettings::parse(&text).expect("the file parses");
    let expected = {
        let record = player_entry(&view, player_id);
        assert_ne!(record.name, "STALE", "premise: the value must move");
        let mut settings = PlayerSettings::from_player(&record).unwrap();
        settings.set_ingame_face(IngameFaceSettings::default());
        settings
    };
    assert_eq!(parsed, expected, "the values are the record's");
}

// A settings.toml already in the folder that does not parse is shown as
// the error, and Write stays disabled.
#[test]
fn an_unparsable_existing_settings_toml_is_the_error() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "generate_bad_toml", 701, 1);
    let dir = crate::test_support::scratch("generate_bad_toml");
    std::fs::write(dir.join("settings.toml"), "name = [\n").unwrap();
    view.pick_folder_dialog = {
        let dir = dir.clone();
        Box::new(move || Some(dir.clone()))
    };
    player_menu_item(&mut window, &mut view, "Generate settings.toml");
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Pick\u{2026}"),
        &mut view,
    );
    let nodes = draw(&mut window, &mut view);
    assert!(
        nodes.iter().any(|node| {
            node.value()
                .is_some_and(|value| value.contains("TOML parse error"))
        }),
        "the file's parse error is shown"
    );
    assert!(
        node_labeled(&nodes, Role::Button, "Write").is_disabled(),
        "Write stays disabled"
    );
}

// A player whose appearance id is the game's -1 is generated like any
// other: his record is his look.
#[test]
fn a_player_at_minus_one_is_generated() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "generate_minus_one", 701, 1);
    let player_id = selected_id(&view, 701, 1);
    let mut entry = player_entry(&view, player_id);
    entry.appearance_id = Some(u32::MAX);
    view.session.as_mut().unwrap().edit_players(vec![entry]);
    let dir = crate::test_support::scratch("generate_minus_one");
    player_menu_item(&mut window, &mut view, "Generate settings.toml");
    pick_and_write(&mut window, &mut view, dir.clone());
    let text = std::fs::read_to_string(dir.join("settings.toml")).unwrap();
    let parsed = PlayerSettings::parse(&text).expect("the file parses");
    let expected = {
        let record = player_entry(&view, player_id);
        assert_eq!(record.appearance_id, Some(u32::MAX), "premise: at -1");
        let mut settings = PlayerSettings::from_player(&record).unwrap();
        settings.set_ingame_face(IngameFaceSettings::default());
        settings
    };
    assert_eq!(parsed, expected, "the values are the record's");
}

// Write waits for a real folder: empty or a path that is not a directory
// it stays disabled; a picked directory enables it.
#[test]
fn write_is_disabled_until_the_folder_is_a_directory() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "generate_write_gate", 701, 1);
    let dir = crate::test_support::scratch("generate_write_gate");
    player_menu_item(&mut window, &mut view, "Generate settings.toml");
    let nodes = draw(&mut window, &mut view);
    assert!(
        node_labeled(&nodes, Role::Button, "Write").is_disabled(),
        "no folder yet"
    );
    // A typed path that is not a directory stays disabled.
    let field = right_of_label(&nodes, "Folder", Role::TextInput)
        .first()
        .copied()
        .expect("the folder field");
    click(&mut window, field, &mut view);
    type_text(
        &mut window,
        &mut view,
        &dir.join("missing").display().to_string(),
    );
    let nodes = draw(&mut window, &mut view);
    assert!(
        node_labeled(&nodes, Role::Button, "Write").is_disabled(),
        "a missing folder is still disabled"
    );
    // A picked directory enables it.
    view.pick_folder_dialog = Box::new(move || Some(dir.clone()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Pick\u{2026}"),
        &mut view,
    );
    let nodes = draw(&mut window, &mut view);
    assert!(
        !node_labeled(&nodes, Role::Button, "Write").is_disabled(),
        "an existing folder enables Write"
    );
}

// Each dialog is headed by its menu item's action, so the member sees which
// of the two files Write puts down.
#[test]
fn each_dialog_is_headed_by_its_action() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "generate_headings", 701, 1);
    for (item, heading) in [
        ("Generate settings.toml", "Generate settings.toml"),
        ("Generate in-game face", "Generate in-game face"),
    ] {
        player_menu_item(&mut window, &mut view, item);
        let nodes = draw(&mut window, &mut view);
        assert!(label_value(&nodes, heading), "the heading {heading:?}");
        click(
            &mut window,
            node_labeled(&nodes, Role::Button, "Cancel"),
            &mut view,
        );
        assert!(view.generate.is_none(), "Cancel closes the dialog");
    }
}

// A settings.toml there that cannot be read (here a folder of that name) is
// shown as the error, Write disabled: only a missing file stands for "no
// file yet", so the template never replaces a file the dialog could not read.
#[test]
fn an_unreadable_existing_settings_toml_is_the_error() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "generate_unreadable", 701, 1);
    let dir = crate::test_support::scratch("generate_unreadable");
    std::fs::create_dir(dir.join("settings.toml")).unwrap();
    view.pick_folder_dialog = {
        let dir = dir.clone();
        Box::new(move || Some(dir.clone()))
    };
    player_menu_item(&mut window, &mut view, "Generate settings.toml");
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Pick\u{2026}"),
        &mut view,
    );
    let nodes = draw(&mut window, &mut view);
    let shown = dir.join("settings.toml").display().to_string();
    assert!(
        nodes
            .iter()
            .any(|node| node.value().is_some_and(|value| value.starts_with(&shown))),
        "the read error names the file"
    );
    assert!(
        node_labeled(&nodes, Role::Button, "Write").is_disabled(),
        "Write stays disabled"
    );
}

// The preview is built when the folder text or the save changes, never on
// a frame where neither did: a settings.toml rewritten on disk behind the
// open dialog shows only once the save is edited, and Write puts down the
// text the dialog previewed.
#[test]
fn the_preview_is_built_when_the_folder_or_the_save_changes() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "generate_preview_kept", 701, 1);
    let dir = crate::test_support::scratch("generate_preview_kept");
    let existing = |comment: &str| format!("# {comment}\nname = \"STALE\"\n");
    std::fs::write(dir.join("settings.toml"), existing("first")).unwrap();
    view.pick_folder_dialog = {
        let dir = dir.clone();
        Box::new(move || Some(dir.clone()))
    };
    player_menu_item(&mut window, &mut view, "Generate settings.toml");
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Pick\u{2026}"),
        &mut view,
    );
    let shows = |nodes: &[accesskit::Node], comment: &str| {
        nodes.iter().any(|node| {
            node.value()
                .is_some_and(|value| value.contains(&format!("# {comment}\n")))
        })
    };
    assert!(shows(&draw(&mut window, &mut view), "first"), "premise");

    std::fs::write(dir.join("settings.toml"), existing("second")).unwrap();
    let nodes = draw(&mut window, &mut view);
    assert!(
        shows(&nodes, "first") && !shows(&nodes, "second"),
        "a frame where nothing changed reads no file"
    );

    // An edit to the save rebuilds it from the folder's file as it is now.
    let other = selected_id(&view, 701, 2);
    let mut entry = player_entry(&view, other);
    entry.basic.height = if entry.basic.height == 170 { 171 } else { 170 };
    view.session.as_mut().unwrap().edit_player(1, entry);
    assert!(shows(&draw(&mut window, &mut view), "second"), "rebuilt");

    std::fs::write(dir.join("settings.toml"), existing("third")).unwrap();
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Write"),
        &mut view,
    );
    assert!(view.generate.is_none(), "the write closed the dialog");
    let written = std::fs::read_to_string(dir.join("settings.toml")).unwrap();
    assert!(
        written.starts_with("# second\n"),
        "Write put down the previewed text: {written:?}"
    );
}
