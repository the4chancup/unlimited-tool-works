//! The PES 2014 save: every screen, menu and the roster drag works, with
//! each control whose field the version does not store greyed.

use pes_savefile::interchange::team_toml::TeamToml;
use pes_savefile::model::PlayerEntry;

use super::*;

/// The disabled state and the PES 2014 hover of `label`'s menu item.
fn assert_greyed_item(
    nodes: &[accesskit::Node],
    window: &mut Headless,
    view: &mut SaveView,
    label: &str,
) {
    let item = menu_item(nodes, label);
    assert!(item.is_disabled(), "{label} is disabled on a PES 2014 save");
    assert_eq!(
        tooltip_containing(
            window,
            view,
            centre(item),
            "Not available on PES 2014 saves"
        ),
        Some("Not available on PES 2014 saves".to_string()),
        "{label}'s hover"
    );
}

// Every screen, the four menus and a roster drag on the committed PES 2014
// fixture (a real save: team 1's 23-man squad, its players carrying no
// appearance records). Nothing panics; the items the version rules out —
// the in-game face run's and the not-in-scope list's — are disabled with
// the PES 2014 hover, and a drag undone leaves the loaded order.
#[test]
fn a_pes14_save_draws_screens_and_menus_and_drags_without_panics() {
    let path = fixture(PesVersion::Pes14, "pes14_screens");
    // The list shows the cup's teams alone (ids 701 to 920), of which the
    // vanilla save holds none: team 1 takes a cup id, its roster along.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes14)).unwrap();
    file.team_mut(1).expect("the vanilla first team").id = 701;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();

    // A rostered player selected: the Stats screen draws, then the Player
    // menu's items gate (the FPC entry and the two Generate items need the
    // in-game face run the version's records carry none of).
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let loaded = row_labels(&nodes);
    click(
        &mut window,
        nodes
            .iter()
            .find(|node| player_row(node))
            .expect("a player row"),
        &mut view,
    );
    assert!(
        matches!(view.selected, Some(Selection::Player(..))),
        "a player row selected"
    );

    let nodes = open_menu(&mut window, &mut view, "Player", "Set all stats");
    for label in [
        "Hide default body (Enable FPC)",
        "Generate settings.toml\u{2026}",
        "Generate in-game face\u{2026}",
    ] {
        assert_greyed_item(&nodes, &mut window, &mut view, label);
    }

    // The Team menu: the boots and gloves item stays live (the boots
    // are stored, the dialog's gloves side greys inside), the two
    // default-body items and the out-of-scope team file and formation
    // picture are greyed.
    let nodes = open_menu(&mut window, &mut view, "Team", "Set edited flag");
    assert!(
        !menu_item(&nodes, "Set boots and gloves IDs for everyone\u{2026}").is_disabled(),
        "the boots and gloves item is enabled on PES 2014"
    );
    for label in [
        "Hide default body (Enable FPC)",
        "Show default body (Disable FPC)",
        "Import / export team file\u{2026}",
        "Save a picture of the formations\u{2026}",
    ] {
        assert_greyed_item(&nodes, &mut window, &mut view, label);
    }

    // The Savefile menu's out-of-scope items; the read-only ones stay.
    let nodes = open_menu(&mut window, &mut view, "Savefile", "Output rosters");
    for label in [
        "Comparator\u{2026}",
        "Apply aesthetics patch\u{2026}",
        "Save formation pictures for a range of teams\u{2026}",
    ] {
        assert_greyed_item(&nodes, &mut window, &mut view, label);
    }
    assert!(
        !menu_item(&nodes, "Output rosters to TSV\u{2026}").is_disabled(),
        "the roster sheet stays enabled"
    );

    // The File menu only draws: nothing on it is gated (its discovery
    // scan and file dialogs are stubbed, as every screens test stubs them).
    view.discover = Box::new(|_| Vec::new());
    view.save_as_dialog = Box::new(|_| None);
    let _ = open_menu(&mut window, &mut view, "File", "Close save");
    // Escape closes the menu the draw check leaves open; a press meant
    // for a row below would only dismiss it.
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );

    // The Appearance screen draws: the FPC buttons and the colour cells
    // take the PES 2014 hover, the record-less player's look controls
    // their own stand-in, the unstored motions and edit flags the `PES
    // NN+` one, and the stored motions stay editable.
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for label in ["Hide default body", "Show default body"] {
        assert!(
            node_labeled(&nodes, Role::Button, label).is_disabled(),
            "the {label} button is disabled on PES 2014"
        );
    }
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Boots ID")
    });
    for label in ["Boots ID", "Gloves ID", "Sleeves", "Outfielder gloves"] {
        disabled_dash(&nodes, label);
    }
    for label in ["Corner kick", "Hunching (dribbling)"] {
        let nodes = scroll_into_view(&mut window, &mut view, |node| {
            node.role() == Role::Label && node.value() == Some(label)
        });
        disabled_dash(&nodes, label);
    }
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Free kick")
    });
    assert!(
        !control_right_of(&nodes, "Free kick").is_disabled(),
        "the free kick motion is stored on PES 2014"
    );
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Neck length")
    });
    disabled_dash(&nodes, "Neck length");
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::CheckBox && node.label() == Some("Face")
    });
    assert!(
        node_labeled(&nodes, Role::CheckBox, "Face").is_disabled(),
        "the face flag's box is disabled on PES 2014"
    );

    // A team row draws (the header and the Gameplan stand-in below it):
    // selected outright, since a list click after the scrolls above races
    // the reveal's own scroll.
    view.selected = Some(Selection::Team(701));
    let (_, _, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // A drag of the second rostered row of team 701 onto the first: the
    // list reorders, and Ctrl+Z puts the loaded order back.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let mut players: Vec<&accesskit::Node> = nodes.iter().filter(|node| player_row(node)).collect();
    players.sort_by(|a, b| {
        let y = |node: &&accesskit::Node| node.bounds().map(|r| r.y0).unwrap_or(0.0);
        y(a).partial_cmp(&y(b)).expect("ordered bounds")
    });
    assert!(players.len() >= 2, "two rostered rows to trade");
    let (from, onto) = (centre(players[1]), centre(players[0]));
    drag(&mut window, from, onto, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_ne!(row_labels(&nodes), loaded, "the drag reorders the roster");
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        row_labels(&nodes),
        loaded,
        "the undo restores the loaded roster order"
    );
}

// The greying keys on the schema, not on "anything but PES 2021": a PES
// 2015 save still draws the Appearance screen's FPC controls enabled and
// the Team menu's default-body items enabled.
#[test]
fn a_pes15_save_keeps_the_face_run_controls_enabled() {
    let path = fixture(PesVersion::Pes15, "pes15_face_enabled");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        nodes
            .iter()
            .find(|node| player_row(node))
            .expect("a player row"),
        &mut view,
    );
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for label in ["Hide default body", "Show default body"] {
        assert!(
            !node_labeled(&nodes, Role::Button, label).is_disabled(),
            "{label} is enabled on PES 15"
        );
    }

    let nodes = open_menu(&mut window, &mut view, "Team", "Set edited flag");
    for label in [
        "Hide default body (Enable FPC)",
        "Show default body (Disable FPC)",
    ] {
        assert!(
            !menu_item(&nodes, label).is_disabled(),
            "{label} is enabled on PES 15"
        );
    }
}

// The Gameplan build on PES 2014: the toggle's segment is disabled, a team
// row keeps its header with a greyed line where the tactics editor would
// draw, the Edit menu's Copy and Paste refuse a team row's Gameplan
// target, and the team file panel's out-of-scope controls are disabled.
#[test]
fn a_pes14_save_greys_the_gameplan_and_the_team_file_panel() {
    let path = fixture(PesVersion::Pes14, "pes14_gameplan");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes14)).unwrap();
    file.team_mut(1).expect("the vanilla first team").id = 701;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();

    // A rostered player is selected on open, so the toggle draws: its
    // Gameplan segment is disabled with the build's tooltip.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let segment = node_labeled(&nodes, Role::Button, "Gameplan");
    assert!(
        segment.is_disabled(),
        "the Gameplan segment is disabled on PES 2014"
    );
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(segment), "PES 2014"),
        Some("PES 2014: in the next build".to_string()),
        "the Gameplan segment's hover"
    );

    // A team row draws the header's fields; the greyed line stands where
    // the tactics editor would, its own widgets absent.
    view.selected = Some(Selection::Team(701));
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let name = display_name(
        &view
            .session
            .as_ref()
            .expect("an open save")
            .file()
            .team(701)
            .expect("team 701")
            .name,
    );
    assert!(
        nodes
            .iter()
            .any(|node| node.role() == Role::TextInput && node.value() == Some(name.as_str())),
        "the team row draws its name box"
    );
    let line = "PES 2014: the Gameplan screen comes in the next build";
    let stand_in = nodes
        .iter()
        .find(|node| node.value() == Some(line) || node.label() == Some(line))
        .expect("the greyed line");
    assert!(stand_in.is_disabled(), "the line reads greyed");
    assert!(
        !nodes
            .iter()
            .any(|node| node.role() == Role::CheckBox && node.label() == Some("Scroll")),
        "the tactics editor's Scroll switch does not draw"
    );

    // The Edit menu's Copy and Paste are dead on the Gameplan target a
    // team row is, with the same build tooltip.
    let nodes = open_menu(&mut window, &mut view, "Edit", "Copy");
    for label in ["Copy", "Paste"] {
        let item = menu_item(&nodes, label);
        assert!(
            item.is_disabled(),
            "{label} is disabled on a PES 2014 Gameplan"
        );
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(item), "PES 2014"),
            Some("PES 2014: in the next build".to_string()),
            "{label}'s hover"
        );
    }

    // The team file panel, opened as the menu's disabled item cannot (the
    // menu check leaves the Edit menu open, closed here): the export and
    // pick buttons are disabled on PES 2014, and the Gameplan section is
    // too, drawn once a file is staged for it.
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    view.panel = Some(Panel::Interchange);
    let _ = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let panel: Vec<accesskit::Node> = nodes_in_window(&pairs, "Team file interchange")
        .into_iter()
        .map(|(_, node)| node.clone())
        .collect();
    for label in ["Team TOML\u{2026}", "Pick\u{2026}"] {
        assert!(
            node_labeled(&panel, Role::Button, label).is_disabled(),
            "the {label} button is disabled on PES 2014"
        );
    }
    let document = {
        let file19 = EditFile::from_bytes(&fixture_save(PesVersion::Pes19)).unwrap();
        let players: Vec<&PlayerEntry> = file19.players().iter().collect();
        TeamToml::from_team(
            PesVersion::Pes19,
            file19.team(714).expect("team 714"),
            &players,
        )
        .expect("the export")
    };
    let toml = path.with_file_name("picked_714.toml");
    std::fs::write(&toml, document.to_toml().expect("the document")).unwrap();
    let recorder = view.session.as_ref().expect("an open save").recorder();
    view.interchange.pick(toml, recorder);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let panel: Vec<accesskit::Node> = nodes_in_window(&pairs, "Team file interchange")
        .into_iter()
        .map(|(_, node)| node.clone())
        .collect();
    assert!(
        node_labeled(&panel, Role::CheckBox, "Gameplan").is_disabled(),
        "the Gameplan section is disabled on PES 2014"
    );

    // Apply cannot write the staged file into the save either.
    let apply = node_labeled(&panel, Role::Button, "Apply");
    assert!(apply.is_disabled(), "Apply is disabled on PES 2014");
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(apply), "PES 2014"),
        Some("Not available on PES 2014 saves".to_string()),
        "Apply's hover"
    );
    view.panel = None;
}

// PES 2014's stored offsets reach the body boxes' ranges: the Height box
// runs from the 132 the record stores less to the u8 field's own top, the
// Weight box's floor is the 30 less the record stores; a typed 200 and 90
// land, a typed 400 clamps to the top.
#[test]
fn a_pes14_save_shifts_the_height_and_weight_ranges() {
    let path = fixture(PesVersion::Pes14, "pes14_ranges");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes14)).unwrap();
    file.team_mut(1).expect("the vanilla first team").id = 701;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let player_id = {
        let session = view.session.as_ref().expect("an open save");
        session.file().team(701).expect("team 701").roster[0].player_id
    };

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let height = right_of_label(&nodes, "Height", Role::SpinButton)[0];
    assert_eq!(
        height.min_numeric_value(),
        Some(132.0),
        "the height box's floor is PES 2014's stored offset"
    );
    let weight = right_of_label(&nodes, "Weight", Role::SpinButton)[0];
    assert_eq!(
        weight.min_numeric_value(),
        Some(30.0),
        "the weight box's floor is PES 2014's stored offset"
    );

    // 200 lands; 400 clamps to the box's top: the shown bound tops at
    // 387, but the u8 field's own top is 255.
    click(&mut window, height, &mut view);
    type_text(&mut window, &mut view, "200");
    assert_eq!(player_entry(&view, player_id).basic.height, 200);
    click(&mut window, height, &mut view);
    type_text(&mut window, &mut view, "400");
    assert_eq!(
        player_entry(&view, player_id).basic.height,
        255,
        "a typed 400 clamps to the box's top"
    );

    let nodes = draw(&mut window, &mut view);
    let weight = right_of_label(&nodes, "Weight", Role::SpinButton)[0];
    click(&mut window, weight, &mut view);
    type_text(&mut window, &mut view, "90");
    assert_eq!(player_entry(&view, player_id).basic.weight, 90);
}

// The greying keys on the version, not on the field: a PES 2015 save's
// Gameplan segment stays enabled and its height box keeps the unshifted
// stored top of 255.
#[test]
fn a_pes15_save_keeps_the_gameplan_segment_and_the_height_top() {
    let path = fixture(PesVersion::Pes15, "pes15_gameplan");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        !node_labeled(&nodes, Role::Button, "Gameplan").is_disabled(),
        "the Gameplan segment is enabled on PES 2015"
    );
    assert_eq!(
        right_of_label(&nodes, "Height", Role::SpinButton)[0].max_numeric_value(),
        Some(255.0),
        "the height box's top is unchanged on PES 2015"
    );
}

// A PES 2014 player the appearance section holds no record for (the whole
// vanilla save): the look controls the record backs are greyed with the
// record's own tooltip — Boots ID's box and the Sleeves combo — while
// the player record's motions stay live.
#[test]
fn a_pes14_player_without_an_appearance_record_greys_the_look_controls() {
    let path = fixture(PesVersion::Pes14, "pes14_look_gated");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes14)).unwrap();
    file.team_mut(1).expect("the vanilla first team").id = 701;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        nodes
            .iter()
            .find(|node| player_row(node))
            .expect("a player row"),
        &mut view,
    );
    open_appearance(&mut window, &mut view);
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Boots ID")
    });
    for label in ["Boots ID", "Sleeves"] {
        let dash = disabled_dash(&nodes, label);
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(dash), "stores no look"),
            Some("PES 2014 stores no look for this player yet: edit it in game first".to_string()),
            "{label}'s hover"
        );
    }
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Free kick")
    });
    assert!(
        !control_right_of(&nodes, "Free kick").is_disabled(),
        "the free kick motion is stored on PES 2014"
    );
}

// Set boots and gloves IDs on the PES 2014 fixture: a gloves choice
// refuses outright on a version without the field, and a filled slot
// whose player has no appearance record — every slot of the vanilla
// save — refuses the whole operation naming the player as the roster
// shows him, nothing written either time.
#[test]
fn set_boots_and_gloves_on_pes14_refuses_gloves_and_record_less_players() {
    use crate::session::team_ops::{BootsMode, GlovesMode};

    let path = fixture(PesVersion::Pes14, "pes14_boglo");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes14)).unwrap();
    file.team_mut(1).expect("the vanilla first team").id = 701;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let first_name = {
        let session = view.session.as_ref().expect("an open save");
        let id = session
            .file()
            .team(701)
            .expect("team 701")
            .roster
            .iter()
            .find(|slot| slot.player_id != 0)
            .expect("a filled slot")
            .player_id;
        display_name(&session.file().player(id).expect("the player").name)
    };

    let error = view
        .session
        .as_mut()
        .unwrap()
        .set_boots_and_gloves(701, BootsMode::Same(83), GlovesMode::Unchanged)
        .unwrap_err();
    assert_eq!(
        format!("{error:#}"),
        format!("PES 2014 stores no look for {first_name} yet: edit it in game first"),
        "the refusal names the first slot's player"
    );
    let session = view.session.as_ref().unwrap();
    assert!(
        !session.is_dirty() && !session.can_undo(),
        "nothing was written"
    );

    let error = view
        .session
        .as_mut()
        .unwrap()
        .set_boots_and_gloves(701, BootsMode::Same(83), GlovesMode::Same(1))
        .unwrap_err();
    assert_eq!(
        format!("{error:#}"),
        "PES 2014 saves store no gloves IDs",
        "a gloves mode on a version without the field"
    );
    let session = view.session.as_ref().unwrap();
    assert!(
        !session.is_dirty() && !session.can_undo(),
        "nothing was written"
    );
}

// The strip combos' labels as the version's game words them
// (`strip_options`): PES 2014's Sleeves' Auto, Undershorts' Off/On and
// the Spectacles cell numbers; PES 2015's Sleeves the other versions'
// Seasonal list.
#[test]
fn strip_options_gives_pes14s_own_words() {
    use pes_savefile::settings_toml::SettingKey;

    use crate::view::appearance::fields::strip_options;

    assert_eq!(
        strip_options(PesVersion::Pes14, SettingKey::Sleeves),
        ["Auto", "Short", "Long"]
    );
    assert_eq!(
        strip_options(PesVersion::Pes14, SettingKey::Undershorts),
        ["Off", "On"]
    );
    assert_eq!(
        strip_options(PesVersion::Pes14, SettingKey::Spectacles),
        ["Off", "2", "3", "4", "5", "6", "7", "8"]
    );
    assert_eq!(
        strip_options(PesVersion::Pes15, SettingKey::Sleeves),
        ["Seasonal", "Short", "Long"]
    );
}

// `EDIT.bin.akko7v2`, the Akko League VII save whose players all carry an
// appearance record: /4ccg/'s 85720's look controls edit — his Sleeves
// combo reads Long and his Socks Short, in the version's own words.
#[test]
#[ignore = "reads the maintainer's PES 2014 saves"]
fn a_pes14_player_with_an_appearance_record_edits_the_look_controls() {
    let path = std::path::Path::new(
        "C:/Data/Documents/KONAMI/Pro Evolution Soccer 2014/save/EDIT.bin.akko7v2",
    );
    let mut view = SaveView::new();
    view.open(path);
    let mut window = Headless::new();
    // Team 857 is /4ccg/'s on this save; 85720's roster slot, selected
    // outright.
    let slot = view
        .session
        .as_ref()
        .expect("an open save")
        .file()
        .team(857)
        .expect("team 857")
        .roster
        .iter()
        .position(|slot| slot.player_id == 85720)
        .expect("85720 is rostered");
    view.selected = Some(Selection::Player(
        857,
        u8::try_from(slot).expect("a roster slot fits u8"),
    ));
    open_appearance(&mut window, &mut view);
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Sleeves")
    });
    let combo = right_of_label(&nodes, "Sleeves", Role::ComboBox)
        .first()
        .copied()
        .expect("the Sleeves combo");
    assert!(!combo.is_disabled(), "the Sleeves combo is enabled");
    assert_eq!(combo.value(), Some("Long"), "85720's sleeves read Long");
    let combo = right_of_label(&nodes, "Socks", Role::ComboBox)
        .first()
        .copied()
        .expect("the Socks combo");
    assert!(!combo.is_disabled(), "the Socks combo is enabled");
    assert_eq!(combo.value(), Some("Short"), "85720's socks read Short");
}
