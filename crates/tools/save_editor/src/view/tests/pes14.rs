//! The PES 2014 save: every screen, menu and the roster drag works, with
//! each control whose field the version does not store greyed.

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

    // The Team menu: the boots and gloves item (neither ID is stored),
    // the two default-body items, and the out-of-scope team file and
    // formation picture.
    let nodes = open_menu(&mut window, &mut view, "Team", "Set edited flag");
    for label in [
        "Set boots and gloves IDs for everyone\u{2026}",
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

    // The Appearance screen draws: the FPC buttons and the colour cells
    // take the PES 2014 hover, the strip and physique cells and the
    // unstored motions and edit flags the `PES NN+` stand-in, and the
    // stored motions stay editable.
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

    // A team row draws (the header and the tactics below it): selected
    // outright, since a list click after the scrolls above races the
    // reveal's own scroll.
    view.selected = Some(Selection::Team(701));
    let (_, _, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // A drag of the second rostered row of team 701 onto the first: the
    // list reorders, and Ctrl+Z puts the loaded order back.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let players: Vec<&accesskit::Node> = nodes.iter().filter(|node| player_row(node)).collect();
    assert!(players.len() >= 2, "two rostered rows to trade");
    let (from, onto) = (centre(players[1]), centre(players[0]));
    drag(&mut window, from, onto, &mut view);
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
